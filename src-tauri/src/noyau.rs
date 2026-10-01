//! Le noyau : les profils de ce PC, le profil ouvert et sa ludothèque.
//!
//! Tout ce qui a été reçu pour un profil vit dans SON dossier (`profils/<id>/`) et dans SA session. Fermer ou
//! changer de profil jette la session : rien n'en reste en mémoire pour le suivant.

use crate::coffre::Coffre;
use crate::erreurs::{Erreur, Resultat};
use crate::firehouse::{origine, Client, Reponse};
use crate::ludotheque::{lire_page, lire_plateformes, Cache, Filtre, JeuResume, Liste, Plateforme};
use crate::profils::{Profils, ProfilVisible};
use crate::source::{Image, Source};
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Garde-fou : jamais plus de pages que ça pour une synchronisation (200 000 jeux à 500 par page = 400).
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

impl Session {
    /// Le cache du profil. Si une opération précédente a échoué en plein travail, le verrou reste utilisable :
    /// le cache SQLite, lui, est protégé par ses transactions.
    pub fn verrou(&self) -> std::sync::MutexGuard<'_, Cache> {
        self.cache.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Taille au-delà de laquelle le journal est mis de côté (`journal.1.txt`) et recommencé.
const TAILLE_MAX_JOURNAL: u64 = 512 * 1024;

#[derive(Debug, Serialize, PartialEq)]
pub struct BilanSynchro {
    /// Jeux dans la ludothèque après la synchronisation.
    pub jeux: usize,
    pub plateformes: usize,
    pub pages: u32,
    pub simule: bool,
    /// `complete` (tout relu) ou `increment` (seulement ce qui a changé).
    pub mode: &'static str,
    /// Jeux reçus (tous en complète, les changés en incrément).
    pub recus: usize,
    /// Jeux sortis de la ludothèque (plus visibles pour ce profil).
    pub retires: usize,
}

pub struct Noyau {
    pub(crate) dossier: PathBuf,
    pub profils: Profils,
    pub(crate) coffre: Box<dyn Coffre>,
    pub(crate) session: Mutex<Option<Arc<Session>>>,
    /// Les jeux présents sur CE PC (tous profils confondus).
    pub(crate) registre: std::sync::Mutex<crate::jeux_pc::Registre>,
    /// Le téléchargement en cours : (jeu, drapeau d'arrêt).
    pub(crate) en_cours: std::sync::Mutex<Option<(i64, Arc<std::sync::atomic::AtomicBool>)>>,
    /// Vrai tant que la file de téléchargements tourne.
    pub(crate) file_active: std::sync::atomic::AtomicBool,
}

pub(crate) fn maintenant() -> String {
    // Date ISO sans dépendance : secondes depuis 1970, lisibles par l'interface.
    let s = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
    s.to_string()
}

impl Noyau {
    pub fn nouveau(dossier: &Path, coffre: Box<dyn Coffre>) -> Resultat<Self> {
        let registre = crate::jeux_pc::Registre::ouvrir(&dossier.join("jeux.db"))?;
        // Un téléchargement interrompu par la fermeture de Frogtend ne reprend qu'à la demande.
        registre.interrompus_en_pause()?;
        Ok(Noyau {
            dossier: dossier.into(),
            profils: Profils::charger(&dossier.join("profils.json"))?,
            coffre,
            session: Mutex::new(None),
            registre: std::sync::Mutex::new(registre),
            en_cours: std::sync::Mutex::new(None),
            file_active: std::sync::atomic::AtomicBool::new(false),
        })
    }

    /// Écrit une ligne dans le journal de ce PC (`journal.txt`). Jamais de jeton ni de secret : seulement ce qui
    /// aide à comprendre un échec (quelle jaquette, quel motif).
    pub fn journaliser(&self, message: &str) {
        use std::io::Write;
        let fichier = self.dossier.join("journal.txt");
        if std::fs::metadata(&fichier).is_ok_and(|m| m.len() > TAILLE_MAX_JOURNAL) {
            let _ = std::fs::rename(&fichier, self.dossier.join("journal.1.txt"));
        }
        let _ = std::fs::create_dir_all(&self.dossier);
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&fichier) {
            let _ = writeln!(f, "{} {message}", maintenant());
        }
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

    /// Vérifie un jeton auprès de Firehouse (`/moi`) AVANT de le ranger : rend qui le porte.
    pub async fn verifier_jeton(connexion: &Connexion, jeton: &str) -> Resultat<Value> {
        Client::nouveau(&connexion.adresse, jeton.trim())?.obtenir_json("/moi").await
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

    /// Refait la connexion du profil ouvert avec les réglages actuels (adresse, mode simulé), sans redemander son PIN.
    pub async fn reconnecter(&self, connexion: &Connexion) -> Resultat<()> {
        let s = self.session().await?;
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

    /// Synchronise la ludothèque du profil ouvert avec Firehouse.
    ///
    /// Première fois (cache vide) : TOUT le catalogue, qui remplace le cache d'un coup. Ensuite : seulement ce qui
    /// a changé depuis la dernière modification connue (`depuis`), et la page 1 donne `ids_visibles` : tout ce qui
    /// n'y est plus sort du cache. Si un jeu visible est inconnu du cache (il vient de redevenir visible), on
    /// refait une synchronisation complète. `progres(page, jeux)` est appelé après chaque page reçue.
    pub async fn synchroniser(&self, progres: impl Fn(u32, usize)) -> Resultat<BilanSynchro> {
        let s = self.session().await?;
        let mut depuis = {
            let c = s.verrou();
            if c.nombre_de_jeux()? > 0 { c.derniere_modification()? } else { None }
        };
        let plateformes = lire_plateformes(&s.source.plateformes().await?);
        loop {
            let mut jeux: Vec<JeuResume> = Vec::new();
            let mut ids_visibles: Option<Vec<i64>> = None;
            let mut page = 1;
            loop {
                let p = lire_page(&s.source.catalogue(page, depuis.as_deref()).await?, page);
                if page == 1 {
                    ids_visibles = p.ids_visibles;
                }
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
            let simule = s.source.est_simulee();

            if let (Some(_), Some(ids)) = (&depuis, ids_visibles) {
                let mut c = s.verrou();
                let recus: std::collections::HashSet<i64> = jeux.iter().map(|j| j.id).collect();
                let mut inconnu = false;
                for id in &ids {
                    if !recus.contains(id) && !c.contient(*id)? {
                        inconnu = true;
                        break;
                    }
                }
                if !inconnu {
                    let (recus, retires) = c.appliquer_increment(&plateformes, &jeux, &ids, &maintenant())?;
                    return Ok(BilanSynchro {
                        jeux: c.nombre_de_jeux()? as usize,
                        plateformes: plateformes.len(),
                        pages: page,
                        simule,
                        mode: "increment",
                        recus,
                        retires,
                    });
                }
                // Un jeu visible nous est inconnu : on relit tout.
                drop(c);
                depuis = None;
                continue;
            }
            // Synchronisation complète (première fois, ou serveur sans `ids_visibles`).
            if depuis.is_some() {
                depuis = None;
                continue;
            }
            let mut c = s.verrou();
            let avant = c.nombre_de_jeux()? as usize;
            c.remplacer(&plateformes, &jeux, &maintenant())?;
            let apres = jeux.len();
            return Ok(BilanSynchro {
                jeux: apres,
                plateformes: plateformes.len(),
                pages: page,
                simule,
                mode: "complete",
                recus: apres,
                retires: avant.saturating_sub(apres),
            });
        }
    }

    /// Qui porte le jeton du profil ouvert (`/moi`).
    pub async fn compte(&self) -> Resultat<Value> {
        self.session().await?.source.moi().await
    }

    /// Enregistre le skin dans le compte Firehouse de la personne du profil ouvert.
    pub async fn enregistrer_skin(&self, nom: &str) -> Resultat<()> {
        self.session().await?.source.enregistrer_theme(nom).await
    }

    /// Le registre des jeux du PC (utilisable même après une erreur en plein travail).
    pub(crate) fn registre(&self) -> std::sync::MutexGuard<'_, crate::jeux_pc::Registre> {
        self.registre.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// `locale` : seulement les jeux du PC (« Ma ludothèque ») ; sinon tout le catalogue du profil.
    fn restriction(&self, locale: bool) -> Resultat<Option<Vec<i64>>> {
        Ok(if locale { Some(self.registre().ids()?) } else { None })
    }

    pub async fn plateformes(&self, locale: bool) -> Resultat<Vec<Plateforme>> {
        let s = self.session().await?;
        let seulement = self.restriction(locale)?;
        let c = s.verrou();
        c.plateformes(seulement.as_deref())
    }

    pub async fn genres(&self, plateforme: Option<&str>, locale: bool) -> Resultat<Vec<String>> {
        let s = self.session().await?;
        let seulement = self.restriction(locale)?;
        let c = s.verrou();
        c.genres(plateforme, seulement.as_deref())
    }

    pub async fn lister(&self, filtre: &Filtre) -> Resultat<Liste> {
        let s = self.session().await?;
        let seulement = self.restriction(filtre.ludotheque)?;
        let c = s.verrou();
        c.lister(filtre, seulement.as_deref())
    }

    pub async fn au_hasard(&self, plateforme: Option<&str>, locale: bool) -> Resultat<Option<JeuResume>> {
        let s = self.session().await?;
        let seulement = self.restriction(locale)?;
        let c = s.verrou();
        c.au_hasard(plateforme, seulement.as_deref())
    }

    pub async fn synchronise_le(&self) -> Resultat<Option<String>> {
        let s = self.session().await?;
        let c = s.verrou();
        c.synchronise_le()
    }

    /// La fiche complète d'un jeu : de Firehouse si possible (et gardée), sinon celle du cache (hors ligne).
    pub async fn fiche(&self, id: i64) -> Resultat<FicheLue> {
        let s = self.session().await?;
        // Un jeu de la ludothèque du PC : sa fiche est sur le disque (hors ligne), si ce profil a le droit de le voir.
        if s.verrou().contient(id)? && self.registre().jeu(id)?.is_some() {
            if let Some(f) = self.fiche_locale(id) {
                return Ok(FicheLue { fiche: f, hors_ligne: false, locale: true });
            }
        }
        match s.source.fiche(id).await {
            Ok(f) => {
                s.verrou().garder_fiche(id, &f, &maintenant())?;
                Ok(FicheLue { fiche: f, hors_ligne: false, locale: false })
            }
            Err(Erreur::Reseau(motif)) => match s.verrou().fiche(id)? {
                Some(f) => Ok(FicheLue { fiche: f, hors_ligne: true, locale: false }),
                None => Err(Erreur::Reseau(motif)),
            },
            Err(e) => Err(e),
        }
    }

    pub async fn annexe_texte(&self, id: i64, i: u32, cle: &str) -> Resultat<Value> {
        let s = self.session().await?;
        if s.verrou().contient(id)? {
            if let Some(v) = self.annexe_texte_locale(id, i) {
                return Ok(v);
            }
        }
        s.source.annexe_texte(id, i, cle).await
    }

    /// Une jaquette du profil ouvert, en miniature si `largeur` est donnée : depuis son dossier si elle y est (et que
    /// son empreinte n'a pas changé), sinon depuis Firehouse (et gardée). Une absence est retenue une semaine.
    pub async fn jaquette(&self, id: i64, largeur: Option<u32>) -> Resultat<Option<Image>> {
        let s = self.session().await?;
        let Some(jeu) = s.verrou().jeu(id)? else {
            return Ok(None); // un jeu qui n'est pas dans SA ludothèque : rien.
        };
        // Un jeu de la ludothèque du PC : sa jaquette est sur le disque (hors ligne).
        if let Some(octets) = self.jaquette_locale(id, largeur) {
            return Ok(Some(Image { type_contenu: type_image(&octets), octets }));
        }
        if jeu.jaquette == Some(false) {
            return Ok(None); // Firehouse dit ne pas en avoir : inutile de demander.
        }
        let largeur = largeur.map(|l| l.clamp(100, 1000).div_ceil(100) * 100);
        let taille = largeur.map_or("orig".to_string(), |l| l.to_string());
        let empreinte: String = jeu
            .jaquette_empreinte
            .as_deref()
            .unwrap_or("x")
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .take(32)
            .collect();
        let prefixe = format!("{id}-{taille}-");
        let dossier = s.dossier.join("jaquettes");
        let fichier = dossier.join(format!("{prefixe}{empreinte}.img"));
        let absent = dossier.join(format!("{prefixe}{empreinte}.absent"));
        if let Ok(octets) = std::fs::read(&fichier) {
            return Ok(Some(Image { type_contenu: type_image(&octets), octets }));
        }
        if let Ok(m) = std::fs::metadata(&absent) {
            if m.modified().ok().and_then(|t| t.elapsed().ok()).is_some_and(|d| d.as_secs() < 7 * 24 * 3600) {
                return Ok(None);
            }
        }
        std::fs::create_dir_all(&dossier)?;
        let recue = s.source.media(id, "jaquette", largeur).await?;
        // L'ancienne jaquette de ce jeu (autre empreinte) n'a plus lieu d'être.
        if let Ok(liste) = std::fs::read_dir(&dossier) {
            for e in liste.flatten() {
                let n = e.file_name().to_string_lossy().to_string();
                if n.starts_with(&prefixe) && e.path() != fichier {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
        match recue {
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

    /// Les skins servis par Firehouse (`/themes`, SANS jeton : utilisable dès l'écran « Qui joue ? »), gardés en
    /// cache pour ce PC avec leur `ETag`. `None` en mode simulé. Hors ligne : ceux gardés.
    pub async fn skins(&self, connexion: &Connexion) -> Resultat<Option<Value>> {
        if connexion.simule {
            return Ok(None);
        }
        let fichier = self.dossier.join("skins.json");
        let garde = self.skins_gardes();
        let etag = garde.as_ref().and_then(|v| v["etag"].as_str().map(String::from));
        let catalogue_garde = garde.map(|v| v["catalogue"].clone());
        match Client::public(&connexion.adresse)?.obtenir("/themes", etag.as_deref()).await {
            Ok(Reponse::NonModifie) => Ok(catalogue_garde),
            Ok(Reponse::Corps { octets, etag, .. }) => {
                let catalogue: Value = serde_json::from_slice(&octets)
                    .map_err(|_| Erreur::Serveur("Firehouse a envoyé des skins illisibles.".into()))?;
                std::fs::create_dir_all(&self.dossier)?;
                std::fs::write(&fichier, serde_json::json!({"etag": etag, "catalogue": catalogue}).to_string())?;
                Ok(Some(catalogue))
            }
            Err(Erreur::Reseau(_)) if catalogue_garde.is_some() => Ok(catalogue_garde),
            Err(e) => Err(e),
        }
    }

    fn skins_gardes(&self) -> Option<Value> {
        let t = std::fs::read_to_string(self.dossier.join("skins.json")).ok()?;
        serde_json::from_str(&t).ok()
    }

    /// La vidéo de fond d'un skin (webm), gardée pour ce PC. Son adresse est `video_api`, un chemin depuis la
    /// RACINE du serveur (contrat 1.3) : on la joint à l'origine de l'adresse, jamais à la base de l'API.
    pub async fn video_skin(&self, connexion: &Connexion, nom: &str) -> Resultat<Option<Vec<u8>>> {
        if connexion.simule {
            return Ok(None);
        }
        let Some(catalogue) = self.skins_gardes().map(|v| v["catalogue"].clone()) else { return Ok(None) };
        let Some(chemin) = catalogue["themes"][nom]["video_api"].as_str().map(String::from) else { return Ok(None) };
        if !chemin.starts_with('/') {
            return Ok(None);
        }
        let version: String = catalogue["version"].as_str().unwrap_or("x").chars().filter(char::is_ascii_alphanumeric).take(32).collect();
        let nom_sur: String = nom.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
        let dossier = self.dossier.join("videos");
        let fichier = dossier.join(format!("{nom_sur}-{version}.webm"));
        if let Ok(o) = std::fs::read(&fichier) {
            return Ok(Some(o));
        }
        let octets = Client::public(&connexion.adresse)?.obtenir_absolu(&format!("{}{chemin}", origine(&connexion.adresse)?)).await?;
        std::fs::create_dir_all(&dossier)?;
        // Les vidéos d'une ancienne version des skins partent.
        if let Ok(liste) = std::fs::read_dir(&dossier) {
            for e in liste.flatten() {
                if e.file_name().to_string_lossy().starts_with(&format!("{nom_sur}-")) {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
        std::fs::write(&fichier, &octets)?;
        Ok(Some(octets))
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
    /// Vrai si la fiche vient des médias gardés sur le PC (jeu de la ludothèque).
    pub locale: bool,
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
        assert!(n.plateformes(false).await.unwrap().iter().any(|p| p.nom == "MS-DOS"));
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
        assert!(n.jaquette(110, None).await.unwrap().is_none());
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
            t.status(200).json_body(json!({"ok": true, "suivante": 2, "jeux": [{"id": 1, "titre": "A", "plateforme": "MS-DOS", "jaquette": true}]}));
        });
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue").query_param("page", "2");
            t.status(200).json_body(json!({"ok": true, "suivante": null, "jeux": [{"id": 2, "titre": "B", "plateforme": "MS-DOS", "jaquette": false}]}));
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

        let img = n.jaquette(1, None).await.unwrap().unwrap();
        assert_eq!(img.type_contenu.as_deref(), Some("image/png"));
        n.jaquette(1, None).await.unwrap().unwrap();
        media.assert_hits(1); // la seconde fois vient du disque
        // Le jeu 2 n'a pas de jaquette selon Firehouse : aucune requête.
        assert!(n.jaquette(2, None).await.unwrap().is_none());
        assert!(d.path().join("profils").join(&id).join("jaquettes").join("1-orig-x.img").is_file());
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

    #[tokio::test]
    async fn reconnecter_change_de_source_sans_redemander_le_pin() {
        let (_d, n) = noyau();
        let id = n.creer_profil("Seb", Some("1234"), None).unwrap().id;
        n.ouvrir(&id, Some("1234"), &SIMULE).await.unwrap();
        let vrai = Connexion { adresse: "https://jeux.hikari-no-sekai.fr".into(), simule: false };
        // Sans jeton, le passage au vrai serveur est refusé, et le dit.
        assert!(matches!(n.reconnecter(&vrai).await, Err(Erreur::JetonRefuse(_))));
        n.reconnecter(&SIMULE).await.unwrap();
        assert!(n.session().await.unwrap().source.est_simulee());
    }

    #[tokio::test]
    async fn la_seconde_synchronisation_est_incrementale_et_retire_les_invisibles() {
        let serveur = MockServer::start();
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/plateformes");
            t.status(200).json_body(json!({"ok": true, "plateformes": [{"nom": "MS-DOS", "jeux": 2}]}));
        });
        // Synchronisation incrémentale : seulement le jeu 1 a changé ; le jeu 2 n'est plus visible.
        let increment = serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue").query_param("depuis", "2026-09-29T10:00:00");
            t.status(200).json_body(json!({"ok": true, "suivante": null, "ids_visibles": [1],
                "jeux": [{"id": 1, "titre": "A (nouvelle version)", "plateforme": "MS-DOS", "maj_le": "2026-09-30T08:00:00"}]}));
        });
        let complete = serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue");
            t.status(200).json_body(json!({"ok": true, "suivante": null, "jeux": [
                {"id": 1, "titre": "A", "plateforme": "MS-DOS", "maj_le": "2026-09-29T10:00:00"},
                {"id": 2, "titre": "B", "plateforme": "MS-DOS", "maj_le": "2026-09-28T10:00:00"}]}));
        });
        let (_d, n) = noyau();
        let id = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: serveur.base_url(), simule: false }).await.unwrap();

        let b1 = n.synchroniser(|_, _| {}).await.unwrap();
        assert_eq!((b1.mode, b1.jeux), ("complete", 2));
        let b2 = n.synchroniser(|_, _| {}).await.unwrap();
        assert_eq!((b2.mode, b2.recus, b2.retires, b2.jeux), ("increment", 1, 1, 1));
        assert_eq!(n.lister(&Filtre::default()).await.unwrap().jeux[0].titre, "A (nouvelle version)");
        increment.assert_hits(1);
        complete.assert_hits(1);
    }

    #[tokio::test]
    async fn un_jeu_redevenu_visible_declenche_une_synchronisation_complete() {
        let serveur = MockServer::start();
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/plateformes");
            t.status(200).json_body(json!({"ok": true, "plateformes": []}));
        });
        // L'incrément annonce l'id 3, visible mais ni changé ni connu : il faut tout relire.
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue").query_param_exists("depuis");
            t.status(200).json_body(json!({"ok": true, "suivante": null, "ids_visibles": [1, 3], "jeux": []}));
        });
        // Première synchronisation : seul le jeu 1 est visible.
        let mut premiere = serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue");
            t.status(200).json_body(json!({"ok": true, "suivante": null, "jeux": [
                {"id": 1, "titre": "A", "plateforme": "PC", "maj_le": "2026-09-29T10:00:00"}]}));
        });
        let (_d, n) = noyau();
        let id = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: serveur.base_url(), simule: false }).await.unwrap();
        assert_eq!(n.synchroniser(|_, _| {}).await.unwrap().jeux, 1);
        premiere.delete();
        // Puis le jeu 3 redevient visible (changement de grade), avec un maj_le ancien.
        let complete = serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue").matches(|r| {
                !r.query_params.as_ref().is_some_and(|q| q.iter().any(|(k, _)| k == "depuis"))
            });
            t.status(200).json_body(json!({"ok": true, "suivante": null, "jeux": [
                {"id": 1, "titre": "A", "plateforme": "PC", "maj_le": "2026-09-29T10:00:00"},
                {"id": 3, "titre": "C", "plateforme": "PC", "maj_le": "2020-01-01T00:00:00"}]}));
        });
        let b = n.synchroniser(|_, _| {}).await.unwrap();
        assert_eq!((b.mode, b.jeux), ("complete", 2));
        complete.assert_hits(1);
    }

    #[tokio::test]
    async fn une_jaquette_changee_est_retelechargee_et_l_ancienne_effacee() {
        let serveur = MockServer::start();
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/plateformes");
            t.status(200).json_body(json!({"ok": true, "plateformes": []}));
        });
        let mut cat = serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue");
            t.status(200).json_body(json!({"ok": true, "suivante": null, "jeux": [
                {"id": 1, "titre": "A", "plateforme": "PC", "jaquette": true, "jaquette_empreinte": "v1", "maj_le": "t1"}]}));
        });
        let media = serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/media/1/jaquette").query_param("largeur", "400");
            t.status(200).body([0xFFu8, 0xD8, 0xFF, 1]);
        });
        let (d, n) = noyau();
        let id = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: serveur.base_url(), simule: false }).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();
        // 350 est arrondi à la centaine supérieure : 400.
        n.jaquette(1, Some(350)).await.unwrap().unwrap();
        n.jaquette(1, Some(400)).await.unwrap().unwrap();
        media.assert_hits(1);

        // La jaquette change dans Firehouse : nouvelle empreinte (reçue par une synchronisation complète ici).
        cat.delete();
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue");
            t.status(200).json_body(json!({"ok": true, "suivante": null, "ids_visibles": [1], "jeux": [
                {"id": 1, "titre": "A", "plateforme": "PC", "jaquette": true, "jaquette_empreinte": "v2", "maj_le": "t2"}]}));
        });
        n.synchroniser(|_, _| {}).await.unwrap();
        n.jaquette(1, Some(400)).await.unwrap().unwrap();
        media.assert_hits(2);
        let fichiers: Vec<String> = std::fs::read_dir(d.path().join("profils").join(&id).join("jaquettes"))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(fichiers, vec!["1-400-v2.img".to_string()]);
    }

    #[tokio::test]
    async fn les_skins_se_lisent_sans_jeton_et_la_video_se_joint_a_l_origine() {
        let serveur = MockServer::start();
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/themes");
            t.status(200).header("etag", "W/\"s1\"").json_body(json!({"ok": true, "version": "s1", "themes": {
                "firehouse": {"base": {}, "resolus": {}, "video_api": "/api/jeux/v1/skin/firehouse/video"}}}));
        });
        let video = serveur.mock(|w, t| {
            // Le chemin est joint à l'ORIGINE : jamais /api/jeux/v1/api/jeux/v1/...
            w.method(GET).path("/api/jeux/v1/skin/firehouse/video");
            t.status(200).body(b"webm");
        });
        let (_d, n) = noyau();
        let c = Connexion { adresse: serveur.base_url(), simule: false };
        // Aucun profil ouvert : l'écran « Qui joue ? » a déjà ses skins.
        let s = n.skins(&c).await.unwrap().unwrap();
        assert_eq!(s["version"], "s1");
        assert_eq!(n.video_skin(&c, "firehouse").await.unwrap().unwrap(), b"webm");
        n.video_skin(&c, "firehouse").await.unwrap().unwrap();
        video.assert_hits(1); // gardée sur le disque
        assert_eq!(n.skins(&SIMULE).await.unwrap(), None);
    }

    #[tokio::test]
    async fn un_jeton_se_verifie_par_moi_avant_d_etre_range() {
        let serveur = MockServer::start();
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/moi").header("authorization", "Bearer bon");
            t.status(200).json_body(json!({"ok": true, "nom": "Seb", "grade": "admin"}));
        });
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/moi");
            t.status(401).json_body(json!({"detail": "connexion requise"}));
        });
        let c = Connexion { adresse: serveur.base_url(), simule: false };
        assert_eq!(Noyau::verifier_jeton(&c, " bon ").await.unwrap()["grade"], "admin");
        assert!(matches!(Noyau::verifier_jeton(&c, "mauvais").await, Err(Erreur::JetonRefuse(_))));
    }

    /// Essai des MINIATURES sur le vrai Firehouse (lecture seule, jeton du profil lu dans le coffre).
    /// `FROGTEND_PROFIL_ESSAI=<id> cargo test essai_miniatures -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_recherche_sur_firehouse() {
        // LECTURE SEULE : la forme de GET /recherche (lot 6). Aucune demande n'est envoyée.
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton pour ce profil");
        let c = Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap();
        for texte in ["dune", "zelda ocarina", "xyzzyqq"] {
            let b = c.brute(reqwest::Method::GET, &format!("/recherche?texte={}", crate::source::encoder(texte)), None, &[]).await.unwrap();
            let t = String::from_utf8_lossy(&b.octets);
            println!("« {texte} » → {} : {}", b.statut, &t[..t.len().min(1500)]);
        }
    }

    #[tokio::test]
    #[ignore]
    async fn essai_plateformes_et_emulateurs_sur_firehouse() {
        // LECTURE SEULE : les plateformes du compte, et l'émulateur recommandé par Firehouse pour chacune.
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton pour ce profil");
        let c = Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap();
        let s = crate::source::Source::Firehouse(c);
        for nom in std::env::var("FROGTEND_PLATEFORMES_ESSAI").unwrap_or_default().split(';').filter(|n| !n.is_empty()) {
            match s.emulateurs(nom).await {
                Ok(v) => {
                    let l: Vec<String> = v["emulateurs"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default()
                        .iter()
                        .map(|e| format!("{}{}", e["nom"].as_str().unwrap_or("?"), if e["recommande"] == true { " (recommandé)" } else { "" }))
                        .collect();
                    println!("{nom:<30} → {}", l.join(", "));
                }
                Err(e) => println!("{nom:<30} → {e:?}"),
            }
        }
        let p = s.plateformes().await.unwrap();
        for pl in p["plateformes"].as_array().or(p.as_array()).cloned().unwrap_or_default() {
            let nom = pl["nom"].as_str().unwrap_or("?").to_string();
            let e = s.emulateurs(&nom).await.map(|v| {
                v["emulateurs"].as_array().cloned().unwrap_or_default().iter().take(3).map(|e| e["nom"].as_str().unwrap_or("?").to_string()).collect::<Vec<_>>().join(", ")
            });
            println!("{nom:<35} {:>5} jeu(x) — {}", pl["jeux"], e.unwrap_or_else(|e| format!("{e:?}")));
        }
    }

    #[tokio::test]
    #[ignore]
    async fn essai_miniatures_sur_firehouse() {
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton pour ce profil");
        let c = Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap();
        for id in [110, 111] {
            for largeur in ["", "?largeur=100", "?largeur=200", "?largeur=300", "?largeur=400", "?largeur=500", "?largeur=1000"] {
                let debut = std::time::Instant::now();
                match c.obtenir(&format!("/media/{id}/jaquette{largeur}"), None).await {
                    Ok(Reponse::Corps { octets, type_contenu, .. }) => println!(
                        "jeu {id} {largeur:<14} → {} octets, {:?}, {} ms",
                        octets.len(),
                        type_contenu,
                        debut.elapsed().as_millis()
                    ),
                    Ok(Reponse::NonModifie) => println!("jeu {id} {largeur:<14} → 304"),
                    Err(e) => println!("jeu {id} {largeur:<14} → ERREUR {e:?} ({} ms)", debut.elapsed().as_millis()),
                }
            }
        }
    }

    /// Essai des routes PUBLIQUES du vrai Firehouse (sans jeton), jamais lancé par la suite (`#[ignore]`).
    /// `cargo test essai_public -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_public_sur_firehouse() {
        let adresse =
            std::env::var("FROGTEND_ADRESSE_ESSAI").unwrap_or_else(|_| "https://jeux.hikari-no-sekai.fr".into());
        let (_d, n) = noyau();
        let c = Connexion { adresse, simule: false };
        let s = n.skins(&c).await.unwrap().expect("skins absents");
        let themes = s["themes"].as_object().unwrap();
        println!("SKINS sans jeton : {} (version {})", themes.len(), s["version"]);
        let avec_video: Vec<_> = themes.iter().filter(|(_, t)| t["video_api"].is_string()).map(|(k, t)| (k, &t["video_api"])).collect();
        println!("SKINS avec vidéo : {avec_video:?}");
        let v = n.video_skin(&c, "firehouse").await.unwrap().expect("vidéo absente");
        println!("VIDÉO firehouse : {} octets, début {:02X?}", v.len(), &v[..4]);
        assert_eq!(&v[..4], &[0x1A, 0x45, 0xDF, 0xA3], "un webm commence par l'en-tête EBML");
        let s2 = n.skins(&c).await.unwrap().unwrap();
        assert_eq!(s["version"], s2["version"]);
    }

    /// Essai sur le VRAI Firehouse, jamais lancé par la suite de tests (`#[ignore]`).
    /// Lecture seule. Jeton lu dans FROGTEND_JETON_ESSAI (jamais affiché), dossier temporaire effacé à la fin.
    /// `cargo test essai_reel -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_reel_sur_firehouse() {
        // Le jeton : FROGTEND_JETON_ESSAI, sinon celui du profil FROGTEND_PROFIL_ESSAI de l'application installée,
        // lu dans le coffre de Windows (ainsi, aucun jeton ne passe par une commande ni par un journal).
        let jeton = std::env::var("FROGTEND_JETON_ESSAI").unwrap_or_else(|_| {
            let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_JETON_ESSAI ou FROGTEND_PROFIL_ESSAI");
            crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton pour ce profil")
        });
        let adresse =
            std::env::var("FROGTEND_ADRESSE_ESSAI").unwrap_or_else(|_| "https://jeux.hikari-no-sekai.fr".into());
        let (_d, n) = noyau();
        let id = n.creer_profil("Essai", None, Some(&jeton)).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse, simule: false }).await.unwrap();

        let b = n.synchroniser(|p, j| println!("  page {p} : {j} jeux")).await.unwrap();
        println!("SYNCHRO : {} jeux, {} plateformes, {} page(s)", b.jeux, b.plateformes, b.pages);
        for p in n.plateformes(false).await.unwrap() {
            println!("  plateforme « {} » : {} jeu(x)", p.nom, p.jeux);
        }
        let liste = n.lister(&Filtre::default()).await.unwrap();
        for j in &liste.jeux {
            println!("  jeu {} « {} » ({:?}) jaquette={:?} versions={:?}", j.id, j.titre, j.annee, j.jaquette, j.versions);
        }
        if let Some(j) = liste.jeux.first() {
            let f = n.fiche(j.id).await.unwrap();
            println!(
                "FICHE {} : « {} », {} version(s), {} annexe(s), résumé de {} caractères",
                j.id,
                f.fiche["titre"].as_str().unwrap_or("?"),
                f.fiche["versions"].as_array().map_or(0, Vec::len),
                f.fiche["annexes"].as_array().map_or(0, Vec::len),
                f.fiche["resume"].as_str().map_or(0, |r| r.chars().count())
            );
            if let Some(a) = f.fiche["annexes"].as_array().and_then(|l| l.iter().find(|a| a["texte"] == true)) {
                let t = n.annexe_texte(j.id, a["i"].as_u64().unwrap() as u32, a["cle"].as_str().unwrap()).await.unwrap();
                println!("ANNEXE « {} » : {} caractères", a["titre"], t["texte"].as_str().map_or(0, |x| x.len()));
            }
            match n.jaquette(j.id, None).await.unwrap() {
                Some(img) => println!("JAQUETTE {} : {:?}, {} octets", j.id, img.type_contenu, img.octets.len()),
                None => println!("JAQUETTE {} : aucune", j.id),
            }
        }
        let c = Connexion { adresse: std::env::var("FROGTEND_ADRESSE_ESSAI").unwrap_or_else(|_| "https://jeux.hikari-no-sekai.fr".into()), simule: false };
        let skins = n.skins(&c).await.unwrap().expect("skins absents");
        println!("SKINS : {} (version {})", skins["themes"].as_object().map_or(0, |t| t.len()), skins["version"]);
        let deuxieme = n.skins(&c).await.unwrap().expect("skins absents au second appel");
        println!("COMPTE : {}", n.compte().await.unwrap());
        match n.video_skin(&c, "firehouse").await.unwrap() {
            Some(v) => println!("VIDÉO firehouse : {} octets", v.len()),
            None => println!("VIDÉO firehouse : aucune"),
        }
        if let Some(j) = liste.jeux.first() {
            match n.jaquette(j.id, Some(400)).await.unwrap() {
                Some(img) => println!("MINIATURE 400 : {:?}, {} octets", img.type_contenu, img.octets.len()),
                None => println!("MINIATURE 400 : aucune"),
            }
        }
        let b2 = n.synchroniser(|_, _| {}).await.unwrap();
        println!("2e SYNCHRO : mode {}, {} reçu(s), {} retiré(s), {} jeux", b2.mode, b2.recus, b2.retires, b2.jeux);
        assert_eq!(skins, deuxieme, "le cache ETag doit rendre les mêmes skins");
        println!("SKIN PERSONNEL : {:?}", n.skin_personnel().await.unwrap());
    }

    #[test]
    fn le_journal_ecrit_une_ligne_et_se_met_de_cote_quand_il_est_gros() {
        let (d, n) = noyau();
        n.journaliser("jaquette 110 : essai");
        let texte = std::fs::read_to_string(d.path().join("journal.txt")).unwrap();
        assert!(texte.trim_end().ends_with("jaquette 110 : essai"));
        std::fs::write(d.path().join("journal.txt"), vec![b'x'; (TAILLE_MAX_JOURNAL + 1) as usize]).unwrap();
        n.journaliser("suite");
        assert!(d.path().join("journal.1.txt").is_file());
        assert!(std::fs::read_to_string(d.path().join("journal.txt")).unwrap().contains("suite"));
    }

    #[test]
    fn le_cache_reste_utilisable_apres_une_erreur_en_plein_travail() {
        let s = Session {
            profil: ProfilVisible { id: "x".into(), nom: "X".into(), protege: false },
            dossier: std::env::temp_dir(),
            source: Source::Simulee,
            cache: std::sync::Mutex::new(Cache::en_memoire().unwrap()),
        };
        let s = std::sync::Arc::new(s);
        let s2 = s.clone();
        // Un travail qui échoue brutalement en tenant le verrou…
        let _ = std::thread::spawn(move || {
            let _garde = s2.verrou();
            panic!("échec simulé");
        })
        .join();
        // … n'empêche pas la suite de lire le cache.
        assert_eq!(s.verrou().nombre_de_jeux().unwrap(), 0);
    }

    #[test]
    fn reconnait_les_types_d_image() {
        assert_eq!(type_image(&[0xFF, 0xD8, 0xFF, 0]).as_deref(), Some("image/jpeg"));
        assert_eq!(type_image(b"RIFF\0\0\0\0WEBPVP8").as_deref(), Some("image/webp"));
        assert_eq!(type_image(b"xx"), None);
    }
}
