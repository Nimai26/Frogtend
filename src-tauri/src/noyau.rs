//! Le noyau : les profils de ce PC, le profil ouvert et sa ludothèque.
//!
//! Tout ce qui a été reçu pour un profil vit dans SON dossier (`profils/<id>/`) et dans SA session. Fermer ou
//! changer de profil jette la session : rien n'en reste en mémoire pour le suivant.

use crate::coffre::Coffre;
use crate::erreurs::{Erreur, Resultat};
use crate::firehouse::{Client, Reponse};
use crate::ludotheque::{lire_page, lire_plateformes, Cache, Filtre, JeuResume, Liste, Plateforme};
use crate::profils::{Profils, ProfilVisible};
use crate::source::{Image, Source};
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Garde-fou : jamais plus de pages que ça pour une synchronisation (20 000 jeux à 100 par page = 200).
const PAGES_MAX: u32 = 5_000;

/// Les réglages du PC dont le noyau a besoin (lus dans `pc.json`).
#[derive(Debug, Clone)]
pub struct Connexion {
    pub adresse: String,
    pub simule: bool,
}

pub struct Session {
    pub profil: ProfilVisible,
    pub dossier: PathBuf,
    pub source: Source,
    pub cache: std::sync::Mutex<Cache>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct BilanSynchro {
    pub jeux: usize,
    pub plateformes: usize,
    pub pages: u32,
    pub simule: bool,
}

pub struct Noyau {
    dossier: PathBuf,
    pub profils: Profils,
    coffre: Box<dyn Coffre>,
    session: Mutex<Option<Arc<Session>>>,
}

fn maintenant() -> String {
    // Date ISO sans dépendance : secondes depuis 1970, lisibles par l'interface.
    let s = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
    s.to_string()
}

impl Noyau {
    pub fn nouveau(dossier: &Path, coffre: Box<dyn Coffre>) -> Resultat<Self> {
        Ok(Noyau {
            dossier: dossier.into(),
            profils: Profils::charger(&dossier.join("profils.json"))?,
            coffre,
            session: Mutex::new(None),
        })
    }

    fn dossier_profil(&self, id: &str) -> PathBuf {
        self.dossier.join("profils").join(id)
    }

    /// Crée un profil. Le jeton (facultatif en mode simulé) part dans le coffre.
    pub fn creer_profil(&self, nom: &str, pin: Option<&str>, jeton: Option<&str>) -> Resultat<ProfilVisible> {
        let jeton = jeton.map(str::trim).filter(|j| !j.is_empty());
        let p = self.profils.creer(nom, pin)?;
        if let Some(j) = jeton {
            if let Err(e) = self.coffre.ranger(&p.id, j) {
                let _ = self.profils.supprimer(&p.id);
                return Err(e);
            }
        }
        Ok(p)
    }

    pub fn a_un_jeton(&self, id: &str) -> Resultat<bool> {
        Ok(self.coffre.lire(id)?.is_some())
    }

    /// Ouvre un profil (après son PIN) : la session précédente est jetée.
    pub async fn ouvrir(&self, id: &str, pin: Option<&str>, connexion: &Connexion) -> Resultat<ProfilVisible> {
        self.profils.verifier_pin(id, pin)?;
        let profil = ProfilVisible::from(&self.profils.trouver(id)?);
        let source = self.source_pour(id, connexion)?;
        let dossier = self.dossier_profil(id);
        let cache = Cache::ouvrir(&dossier.join("ludotheque.db"))?;
        *self.session.lock().await =
            Some(Arc::new(Session { profil: profil.clone(), dossier, source, cache: std::sync::Mutex::new(cache) }));
        Ok(profil)
    }

    fn source_pour(&self, id: &str, connexion: &Connexion) -> Resultat<Source> {
        if connexion.simule {
            return Ok(Source::Simulee);
        }
        let jeton = self.coffre.lire(id)?.ok_or_else(|| {
            Erreur::JetonRefuse("Ce profil n'a pas encore de jeton Firehouse. Ajoute-le dans « Mon profil ».".into())
        })?;
        Ok(Source::Firehouse(Client::nouveau(&connexion.adresse, &jeton)?))
    }

    pub async fn fermer(&self) {
        *self.session.lock().await = None;
    }

    pub async fn actif(&self) -> Option<ProfilVisible> {
        self.session.lock().await.as_ref().map(|s| s.profil.clone())
    }

    pub async fn session(&self) -> Resultat<Arc<Session>> {
        self.session
            .lock()
            .await
            .clone()
            .ok_or_else(|| Erreur::Profil("Aucun profil n'est ouvert.".into()))
    }

    /// Remplace le jeton du profil ouvert, puis rouvre la session avec lui.
    pub async fn changer_jeton(&self, jeton: &str, connexion: &Connexion) -> Resultat<()> {
        let s = self.session().await?;
        let jeton = jeton.trim();
        if jeton.is_empty() {
            return Err(Erreur::Coffre("Le jeton est vide.".into()));
        }
        self.coffre.ranger(&s.profil.id, jeton)?;
        let source = self.source_pour(&s.profil.id, connexion)?;
        let cache = Cache::ouvrir(&s.dossier.join("ludotheque.db"))?;
        *self.session.lock().await = Some(Arc::new(Session {
            profil: s.profil.clone(),
            dossier: s.dossier.clone(),
            source,
            cache: std::sync::Mutex::new(cache),
        }));
        Ok(())
    }

    /// Supprime un profil de ce PC : son jeton, son cache, ses jaquettes. Demande son PIN.
    pub async fn supprimer_profil(&self, id: &str, pin: Option<&str>) -> Resultat<()> {
        self.profils.verifier_pin(id, pin)?;
        if self.actif().await.is_some_and(|p| p.id == id) {
            self.fermer().await;
        }
        self.coffre.oublier(id)?;
        let dossier = self.dossier_profil(id);
        // Vérifier ce qu'on supprime : uniquement le dossier de CE profil, sous le dossier de l'application.
        if dossier.starts_with(self.dossier.join("profils")) && dossier.is_dir() {
            std::fs::remove_dir_all(&dossier)?;
        }
        self.profils.supprimer(id)
    }

    /// Synchronise TOUT le catalogue visible par ce profil, puis remplace le cache d'un coup.
    /// `progres(page)` est appelé après chaque page reçue.
    pub async fn synchroniser(&self, progres: impl Fn(u32, usize)) -> Resultat<BilanSynchro> {
        let s = self.session().await?;
        let plateformes = lire_plateformes(&s.source.plateformes().await?);
        let mut jeux: Vec<JeuResume> = Vec::new();
        let mut page = 1;
        loop {
            let p = lire_page(&s.source.catalogue(page).await?, page);
            let vide = p.jeux.is_empty();
            jeux.extend(p.jeux);
            progres(page, jeux.len());
            match p.encore {
                Some(true) => {}
                Some(false) => break,
                None if vide => break,
                None => {}
            }
            if page >= PAGES_MAX {
                break;
            }
            page += 1;
        }
        // Un même jeu reçu deux fois (pages qui bougent pendant la lecture) n'est gardé qu'une fois.
        jeux.sort_by_key(|j| j.id);
        jeux.dedup_by_key(|j| j.id);
        s.cache.lock().unwrap().remplacer(&plateformes, &jeux, &maintenant())?;
        Ok(BilanSynchro { jeux: jeux.len(), plateformes: plateformes.len(), pages: page, simule: s.source.est_simulee() })
    }

    pub async fn plateformes(&self) -> Resultat<Vec<Plateforme>> {
        let s = self.session().await?;
        let c = s.cache.lock().unwrap();
        c.plateformes()
    }

    pub async fn genres(&self, plateforme: Option<&str>) -> Resultat<Vec<String>> {
        let s = self.session().await?;
        let c = s.cache.lock().unwrap();
        c.genres(plateforme)
    }

    pub async fn lister(&self, filtre: &Filtre) -> Resultat<Liste> {
        let s = self.session().await?;
        let c = s.cache.lock().unwrap();
        c.lister(filtre)
    }

    pub async fn au_hasard(&self, plateforme: Option<&str>) -> Resultat<Option<JeuResume>> {
        let s = self.session().await?;
        let c = s.cache.lock().unwrap();
        c.au_hasard(plateforme)
    }

    pub async fn synchronise_le(&self) -> Resultat<Option<String>> {
        let s = self.session().await?;
        let c = s.cache.lock().unwrap();
        c.synchronise_le()
    }

    /// La fiche complète d'un jeu : de Firehouse si possible (et gardée), sinon celle du cache (hors ligne).
    pub async fn fiche(&self, id: i64) -> Resultat<FicheLue> {
        let s = self.session().await?;
        match s.source.fiche(id).await {
            Ok(f) => {
                s.cache.lock().unwrap().garder_fiche(id, &f, &maintenant())?;
                Ok(FicheLue { fiche: f, hors_ligne: false })
            }
            Err(Erreur::Reseau(motif)) => match s.cache.lock().unwrap().fiche(id)? {
                Some(f) => Ok(FicheLue { fiche: f, hors_ligne: true }),
                None => Err(Erreur::Reseau(motif)),
            },
            Err(e) => Err(e),
        }
    }

    pub async fn annexe_texte(&self, id: i64, i: u32, cle: &str) -> Resultat<Value> {
        let s = self.session().await?;
        s.source.annexe_texte(id, i, cle).await
    }

    /// Une jaquette du profil ouvert : depuis son dossier si elle y est, sinon depuis Firehouse (et gardée).
    /// Une absence est retenue une semaine pour ne pas redemander sans cesse.
    pub async fn jaquette(&self, id: i64) -> Resultat<Option<Image>> {
        let s = self.session().await?;
        if !s.cache.lock().unwrap().contient(id)? {
            return Ok(None); // un jeu qui n'est pas dans SA ludothèque : rien.
        }
        let dossier = s.dossier.join("jaquettes");
        let fichier = dossier.join(format!("{id}.img"));
        let absent = dossier.join(format!("{id}.absent"));
        if let Ok(octets) = std::fs::read(&fichier) {
            return Ok(Some(Image { type_contenu: type_image(&octets), octets }));
        }
        if let Ok(m) = std::fs::metadata(&absent) {
            if m.modified().ok().and_then(|t| t.elapsed().ok()).is_some_and(|d| d.as_secs() < 7 * 24 * 3600) {
                return Ok(None);
            }
        }
        std::fs::create_dir_all(&dossier)?;
        match s.source.media(id, "jaquette").await? {
            Some(img) => {
                std::fs::write(&fichier, &img.octets)?;
                Ok(Some(Image { type_contenu: img.type_contenu.or_else(|| type_image(&img.octets)), octets: img.octets }))
            }
            None => {
                if !s.source.est_simulee() {
                    std::fs::write(&absent, b"")?;
                }
                Ok(None)
            }
        }
    }

    /// Les skins servis par Firehouse, gardés en cache avec leur `ETag`. `None` en mode simulé.
    pub async fn skins(&self) -> Resultat<Option<Value>> {
        let s = self.session().await?;
        let fichier = s.dossier.join("skins.json");
        let (etag, garde): (Option<String>, Option<Value>) = match std::fs::read_to_string(&fichier) {
            Ok(t) => match serde_json::from_str::<Value>(&t) {
                Ok(v) => (v["etag"].as_str().map(String::from), Some(v["catalogue"].clone())),
                Err(_) => (None, None),
            },
            Err(_) => (None, None),
        };
        match s.source.themes(etag.as_deref()).await {
            Ok(None) => Ok(None),
            Ok(Some(Reponse::NonModifie)) => Ok(garde),
            Ok(Some(Reponse::Corps { octets, etag, .. })) => {
                let catalogue: Value = serde_json::from_slice(&octets)
                    .map_err(|_| Erreur::Serveur("Firehouse a envoyé des skins illisibles.".into()))?;
                let a_garder = serde_json::json!({"etag": etag, "catalogue": catalogue});
                std::fs::write(&fichier, a_garder.to_string())?;
                Ok(Some(catalogue))
            }
            // Hors ligne : les skins gardés.
            Err(Erreur::Reseau(_)) if garde.is_some() => Ok(garde),
            Err(e) => Err(e),
        }
    }

    pub async fn skin_personnel(&self) -> Resultat<Option<Value>> {
        let s = self.session().await?;
        match s.source.theme().await {
            Err(Erreur::Reseau(_)) => Ok(None),
            r => r,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FicheLue {
    pub fiche: Value,
    /// Vrai si Firehouse était injoignable et que la fiche vient du cache.
    pub hors_ligne: bool,
}

/// Le type d'une image d'après ses premiers octets.
pub fn type_image(o: &[u8]) -> Option<String> {
    let t = if o.starts_with(&[0x89, b'P', b'N', b'G']) {
        "image/png"
    } else if o.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "image/jpeg"
    } else if o.len() > 12 && &o[0..4] == b"RIFF" && &o[8..12] == b"WEBP" {
        "image/webp"
    } else if o.starts_with(b"GIF8") {
        "image/gif"
    } else {
        return None;
    };
    Some(t.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coffre::CoffreMemoire;
    use httpmock::prelude::*;
    use serde_json::json;

    const SIMULE: Connexion = Connexion { adresse: String::new(), simule: true };

    fn noyau() -> (tempfile::TempDir, Noyau) {
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(d.path(), Box::new(CoffreMemoire::default())).unwrap();
        (d, n)
    }

    #[tokio::test]
    async fn rien_n_est_lisible_sans_profil_ouvert() {
        let (_d, n) = noyau();
        assert!(matches!(n.lister(&Filtre::default()).await, Err(Erreur::Profil(_))));
    }

    #[tokio::test]
    async fn synchroniser_en_simule_remplit_la_ludotheque() {
        let (_d, n) = noyau();
        let id = n.creer_profil("Seb", Some("1234"), None).unwrap().id;
        n.ouvrir(&id, Some("1234"), &SIMULE).await.unwrap();
        let pages = std::sync::Mutex::new(0);
        let b = n.synchroniser(|p, _| *pages.lock().unwrap() = p).await.unwrap();
        assert!(b.jeux > 30 && b.simule);
        assert_eq!(*pages.lock().unwrap(), b.pages);
        assert_eq!(n.lister(&Filtre::default()).await.unwrap().total as usize, b.jeux);
        assert!(n.plateformes().await.unwrap().iter().any(|p| p.nom == "MS-DOS"));
    }

    #[tokio::test]
    async fn un_mauvais_pin_n_ouvre_rien() {
        let (_d, n) = noyau();
        let id = n.creer_profil("Seb", Some("1234"), None).unwrap().id;
        assert!(matches!(n.ouvrir(&id, Some("0000"), &SIMULE).await, Err(Erreur::Pin(_))));
        assert_eq!(n.actif().await, None);
    }

    #[tokio::test]
    async fn changer_de_profil_ne_montre_jamais_la_ludotheque_du_precedent() {
        let (_d, n) = noyau();
        let seb = n.creer_profil("Seb", None, None).unwrap().id;
        let lea = n.creer_profil("Léa", None, None).unwrap().id;
        n.ouvrir(&seb, None, &SIMULE).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();
        assert!(n.lister(&Filtre::default()).await.unwrap().total > 0);

        n.ouvrir(&lea, None, &SIMULE).await.unwrap();
        assert_eq!(n.lister(&Filtre::default()).await.unwrap().total, 0);
        assert!(n.jaquette(110).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn en_vrai_sans_jeton_le_profil_ne_s_ouvre_pas() {
        let (_d, n) = noyau();
        let id = n.creer_profil("Seb", None, None).unwrap().id;
        let vrai = Connexion { adresse: "https://jeux.hikari-no-sekai.fr".into(), simule: false };
        assert!(matches!(n.ouvrir(&id, None, &vrai).await, Err(Erreur::JetonRefuse(_))));
    }

    #[tokio::test]
    async fn synchronisation_reelle_paginee_puis_jaquette_gardee() {
        let serveur = MockServer::start();
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/plateformes");
            t.status(200).json_body(json!([{"nom": "MS-DOS", "jeux": 2}]));
        });
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue").query_param("page", "1");
            t.status(200).json_body(json!({"pages": 2, "jeux": [{"id": 1, "titre": "A", "plateforme": "MS-DOS"}]}));
        });
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue").query_param("page", "2");
            t.status(200).json_body(json!({"pages": 2, "jeux": [{"id": 2, "titre": "B", "plateforme": "MS-DOS"}]}));
        });
        let png = [0x89u8, b'P', b'N', b'G', 1, 2, 3];
        let media = serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/media/1/jaquette");
            t.status(200).body(png);
        });

        let (d, n) = noyau();
        let id = n.creer_profil("Seb", None, Some("jeton-test")).unwrap().id;
        let c = Connexion { adresse: serveur.base_url(), simule: false };
        n.ouvrir(&id, None, &c).await.unwrap();
        let b = n.synchroniser(|_, _| {}).await.unwrap();
        assert_eq!((b.jeux, b.pages), (2, 2));

        let img = n.jaquette(1).await.unwrap().unwrap();
        assert_eq!(img.type_contenu.as_deref(), Some("image/png"));
        n.jaquette(1).await.unwrap().unwrap();
        media.assert_hits(1); // la seconde fois vient du disque
        assert!(d.path().join("profils").join(&id).join("jaquettes").join("1.img").is_file());
    }

    #[tokio::test]
    async fn hors_ligne_la_fiche_deja_lue_reste_consultable() {
        let (_d, n) = noyau();
        let id = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        // Premier passage en simulé pour garder la fiche de Dune…
        n.ouvrir(&id, None, &SIMULE).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();
        assert!(!n.fiche(110).await.unwrap().hors_ligne);
        // … puis Firehouse injoignable.
        let panne = Connexion { adresse: "http://127.0.0.1:9".into(), simule: false };
        n.ouvrir(&id, None, &panne).await.unwrap();
        let f = n.fiche(110).await.unwrap();
        assert!(f.hors_ligne);
        assert_eq!(f.fiche["titre"], "Dune");
        assert!(n.lister(&Filtre::default()).await.unwrap().total > 0);
    }

    #[tokio::test]
    async fn supprimer_un_profil_efface_son_jeton_et_son_dossier_seulement() {
        let (d, n) = noyau();
        let seb = n.creer_profil("Seb", Some("1234"), Some("j1")).unwrap().id;
        let lea = n.creer_profil("Léa", None, Some("j2")).unwrap().id;
        for id in [&seb, &lea] {
            n.ouvrir(id, if id == &seb { Some("1234") } else { None }, &SIMULE).await.unwrap();
            n.synchroniser(|_, _| {}).await.unwrap();
        }
        assert!(n.supprimer_profil(&seb, Some("0000")).await.is_err());
        n.supprimer_profil(&seb, Some("1234")).await.unwrap();
        assert!(!d.path().join("profils").join(&seb).exists());
        assert!(d.path().join("profils").join(&lea).join("ludotheque.db").exists());
        assert!(!n.a_un_jeton(&seb).unwrap());
        assert!(n.a_un_jeton(&lea).unwrap());
    }

    #[test]
    fn reconnait_les_types_d_image() {
        assert_eq!(type_image(&[0xFF, 0xD8, 0xFF, 0]).as_deref(), Some("image/jpeg"));
        assert_eq!(type_image(b"RIFF\0\0\0\0WEBPVP8").as_deref(), Some("image/webp"));
        assert_eq!(type_image(b"xx"), None);
    }
}
