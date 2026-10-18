//! La galerie d'un jeu (lot Médias M2, contrat 1.6) : tous ses visuels (`/jeu/{id}/medias`), chacun demandé par son
//! `id` (`/media/{id}/{type}?id=<media>&largeur=`, relevé réel du 18/10 : un id inconnu, ou sous un autre type → 404).
//! La liste est gardée pour le PROFIL ouvert (`<profil>\visuels\<id>\medias.json`), relue quand `medias_empreinte` du
//! catalogue change ; chaque image est gardée sous son empreinte (`m<media>-<largeur>-<empreinte>.img`, écrite puis
//! renommée), jamais l'original (100 à 1 000 px). Une image retirée de la liste quitte le cache ; un jeu devenu
//! invisible pour ce profil (404) : tout ce qui en était gardé part.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::erreurs::{Erreur, Resultat};
use crate::noyau::{Noyau, SORTES_VISUELS};
use crate::source::Image;

/// Un visuel de la galerie, tel que Firehouse le décrit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Media {
    pub id: i64,
    #[serde(rename = "type")]
    pub sorte: String,
    #[serde(default)]
    pub principal: bool,
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub largeur: Option<u32>,
    #[serde(default)]
    pub hauteur: Option<u32>,
    #[serde(default)]
    pub empreinte: String,
}

/// L'empreinte, telle qu'elle entre dans un nom de fichier (alphanumérique, 32 caractères au plus).
fn propre(e: &str) -> String {
    e.chars().filter(char::is_ascii_alphanumeric).take(32).collect()
}

/// Les visuels utilisables d'une réponse (`{medias: [...]}`, ou un tableau nu), élément par élément : un élément
/// illisible, d'un type inconnu (il entre dans l'adresse) ou sans empreinte est laissé de côté.
pub fn lire_medias(v: &Value) -> Vec<Media> {
    let elements = v["medias"].as_array().or_else(|| v.as_array()).cloned().unwrap_or_default();
    elements
        .into_iter()
        .filter_map(|e| serde_json::from_value::<Media>(e).ok())
        .filter(|m| m.id > 0 && (m.sorte == "jaquette" || SORTES_VISUELS.contains(&m.sorte.as_str())))
        .map(|m| Media { empreinte: propre(&m.empreinte), ..m })
        .filter(|m| !m.empreinte.is_empty())
        .collect()
}

/// Écrit à côté puis renomme : jamais un fichier tronqué gardé. Le fichier provisoire a un nom UNIQUE (deux demandes
/// simultanées de la même image n'écrivent jamais le même).
pub(crate) fn ecrire(fichier: &Path, octets: &[u8]) -> std::io::Result<()> {
    static COMPTEUR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    if let Some(d) = fichier.parent() {
        std::fs::create_dir_all(d)?;
    }
    let n = COMPTEUR.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut nom = fichier.as_os_str().to_os_string();
    nom.push(format!(".{}-{n}.part", std::process::id()));
    let part = PathBuf::from(nom);
    let r = std::fs::write(&part, octets).and_then(|_| std::fs::rename(&part, fichier));
    if r.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    r
}

/// Retire les images de galerie gardées de ce jeu : celles dont l'id n'est pas dans `gardes`, ou, pour `seul` = Some
/// (id, empreinte), les autres versions de cette image (toutes largeurs de l'empreinte en cours gardées).
fn purger(dossier: &Path, gardes: Option<&[i64]>, seul: Option<(i64, &str)>) {
    let Ok(l) = std::fs::read_dir(dossier) else { return };
    for e in l.flatten() {
        let n = e.file_name().to_string_lossy().to_string();
        let Some(reste) = n.strip_prefix('m') else { continue };
        let Some((mid, suite)) = reste.split_once('-') else { continue };
        let Ok(mid) = mid.parse::<i64>() else { continue };
        let partir = match (gardes, seul) {
            (Some(g), _) => !g.contains(&mid),
            (None, Some((m, emp))) => mid == m && !suite.ends_with(&format!("-{emp}.img")),
            _ => false,
        };
        if partir {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

impl Noyau {
    fn dossier_galerie(s: &crate::noyau::Session, id: i64) -> PathBuf {
        s.dossier.join("visuels").join(id.to_string())
    }

    /// La liste gardée pour ce profil (sans réseau), et son empreinte.
    fn medias_gardes(dossier: &Path) -> Option<(Vec<Media>, Option<String>)> {
        let v: Value = serde_json::from_slice(&std::fs::read(dossier.join("medias.json")).ok()?).ok()?;
        Some((lire_medias(&v), v["medias_empreinte"].as_str().map(String::from)))
    }

    /// Les visuels d'un jeu (galerie de la fiche) : la liste gardée si son empreinte est celle du catalogue, sinon
    /// relue chez Firehouse et gardée ; hors ligne, la liste gardée.
    pub async fn medias(&self, id: i64) -> Resultat<Vec<Media>> {
        let s = self.session_du_jeu(id).await?;
        let dossier = Self::dossier_galerie(&s, id);
        let attendue = s.verrou().jeu(id)?.and_then(|j| j.medias_empreinte);
        let gardee = Self::medias_gardes(&dossier);
        if let (Some((l, Some(e))), Some(a)) = (&gardee, &attendue) {
            if e == a {
                return Ok(l.clone());
            }
        }
        match s.source.medias(id).await {
            // Une réponse sans liste lisible (ni `medias: [...]` ni tableau nu) ne touche à rien de ce qui est gardé.
            Ok(v) if !(v["medias"].is_array() || v.is_array()) => Ok(gardee.map(|(l, _)| l).unwrap_or_default()),
            Ok(v) => {
                let l = lire_medias(&v);
                // Garder est un plus. Une liste vide n'est gardée que si le catalogue donne l'empreinte (sinon elle
                // serait relue de toute façon) ; les images retirées de la liste partent.
                if !l.is_empty() || attendue.is_some() || dossier.join("medias.json").exists() {
                    if let Err(e) = ecrire(&dossier.join("medias.json"), &serde_json::to_vec_pretty(&v).unwrap()) {
                        self.journaliser(&format!("galerie du jeu {id} : liste non gardée ({e})"));
                    }
                }
                purger(&dossier, Some(&l.iter().map(|m| m.id).collect::<Vec<_>>()), None);
                Ok(l)
            }
            Err(e @ (Erreur::Reseau(_) | Erreur::Serveur(_))) => gardee.map(|(l, _)| l).ok_or(e),
            // Caché pour ce profil (404, comme un jeu inexistant) : TOUT ce qui en était gardé pour ce profil part
            // (galerie et visuels), seulement dans le dossier de CE profil.
            Err(e @ Erreur::Introuvable(_)) => {
                if dossier.starts_with(s.dossier.join("visuels")) && dossier != s.dossier.join("visuels") {
                    let _ = std::fs::remove_dir_all(&dossier);
                }
                Err(e)
            }
            Err(e) => Err(e),
        }
    }

    /// Une image précise de la galerie, en miniature (100 à 1 000 px) : seulement si elle est dans la liste GARDÉE de
    /// ce jeu pour ce profil (jamais d'appel pour un id inconnu), gardée sous son empreinte.
    pub async fn media_precis(&self, id: i64, media: i64, largeur: u32) -> Resultat<Option<Image>> {
        let s = self.session_du_jeu(id).await?;
        let dossier = Self::dossier_galerie(&s, id);
        let Some((liste, _)) = Self::medias_gardes(&dossier) else { return Ok(None) };
        let Some(m) = liste.into_iter().find(|m| m.id == media) else { return Ok(None) };
        let largeur = largeur.clamp(100, 1000).div_ceil(100) * 100;
        let fichier = dossier.join(format!("m{media}-{largeur}-{}.img", m.empreinte));
        if let Ok(octets) = std::fs::read(&fichier) {
            return Ok(Some(Image { type_contenu: crate::noyau::type_image(&octets), octets }));
        }
        match s.source.media_precis(id, &m.sorte, media, largeur).await? {
            Some(img) => {
                if ecrire(&fichier, &img.octets).is_ok() {
                    purger(&dossier, None, Some((media, &m.empreinte)));
                }
                Ok(Some(Image { type_contenu: img.type_contenu.or_else(|| crate::noyau::type_image(&img.octets)), octets: img.octets }))
            }
            None => Ok(None),
        }
    }
}

/// Hors ligne (lot Médias M4) : la même image (même empreinte) gardée à une AUTRE largeur, la plus proche au-dessus,
/// sinon la plus grande au-dessous. Les fichiers lus : `<debut><largeur>-<empreinte>.img` dans `dossier`.
pub(crate) fn autre_largeur(dossier: &Path, debut: &str, empreinte: &str, largeur: u32) -> Option<Vec<u8>> {
    let fin = format!("-{empreinte}.img");
    let mut dessus: Option<(u32, PathBuf)> = None;
    let mut dessous: Option<(u32, PathBuf)> = None;
    for e in std::fs::read_dir(dossier).ok()?.flatten() {
        let n = e.file_name().to_string_lossy().to_string();
        let Some(l) = n.strip_prefix(debut).and_then(|r| r.strip_suffix(&fin)).and_then(|l| l.parse::<u32>().ok()) else { continue };
        if l >= largeur {
            if dessus.as_ref().is_none_or(|(d, _)| l < *d) {
                dessus = Some((l, e.path()));
            }
        } else if dessous.as_ref().is_none_or(|(d, _)| l > *d) {
            dessous = Some((l, e.path()));
        }
    }
    std::fs::read(dessus.or(dessous)?.1).ok()
}

/// Le numéro de jeu d'un nom rangé dans un cache d'images : le texte avant le premier `-` (ou le nom entier), lu comme
/// un entier > 0. Tout autre nom n'est jamais touché.
fn id_du_nom(nom: &str) -> Option<i64> {
    let id = nom.split('-').next()?.parse::<i64>().ok()?;
    (id > 0).then_some(id)
}

/// Ce qu'occupe un fichier ou un dossier (jamais en suivant un lien).
fn taille(p: &Path) -> u64 {
    match std::fs::symlink_metadata(p) {
        Ok(m) if m.is_dir() => std::fs::read_dir(p).into_iter().flatten().flatten().map(|e| taille(&e.path())).sum(),
        Ok(m) => m.len(),
        Err(_) => 0,
    }
}

/// Ce que le ménage a retiré.
#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize)]
pub struct Menage {
    pub elements: usize,
    pub octets: u64,
}

impl Noyau {
    /// Le ménage des caches d'images d'UN profil (lot Médias M4), après une synchronisation complète et réussie : tout
    /// ce qui est rangé sous un jeu que Firehouse ne montre plus à ce profil (caché, retiré, fiche réunie) part —
    /// jaquettes, visuels et galerie, et la LISTE des documents. Jamais les fichiers des documents (un livret
    /// téléchargé avec l'accord de la personne peut n'être que là), jamais un jeu du PC (son ancien numéro reste
    /// jouable), jamais hors des trois dossiers du profil, jamais un nom non reconnu ni un lien.
    pub(crate) fn menage_caches(&self, s: &crate::noyau::Session) -> Resultat<Menage> {
        let mut gardes: std::collections::HashSet<i64> = s.verrou().ids_des_jeux()?.into_iter().collect();
        if gardes.is_empty() {
            return Ok(Menage::default()); // un catalogue vide ne prouve rien
        }
        gardes.extend(self.registre().ids()?);
        let mut m = Menage::default();
        let mut retirer = |p: &Path| {
            let Ok(meta) = std::fs::symlink_metadata(p) else { return };
            if meta.file_type().is_symlink() {
                return;
            }
            let t = taille(p);
            let r = if meta.is_dir() { std::fs::remove_dir_all(p) } else { std::fs::remove_file(p) };
            if r.is_ok() {
                m.elements += 1;
                m.octets += t;
            }
        };
        for sous in ["jaquettes", "visuels"] {
            let Ok(l) = std::fs::read_dir(s.dossier.join(sous)) else { continue };
            for e in l.flatten() {
                let n = e.file_name().to_string_lossy().to_string();
                if id_du_nom(&n).is_some_and(|id| !gardes.contains(&id)) {
                    retirer(&e.path());
                }
            }
        }
        if let Ok(l) = std::fs::read_dir(s.dossier.join("documents")) {
            for e in l.flatten() {
                let n = e.file_name().to_string_lossy().to_string();
                if n.parse::<i64>().ok().filter(|id| *id > 0).is_some_and(|id| !gardes.contains(&id)) {
                    retirer(&e.path().join("documents.json"));
                }
            }
        }
        Ok(m)
    }

    /// Taodbox hors ligne (lot Médias M4) : le fond et le logo des jeux du PC prêts à jouer, visibles de ce profil,
    /// gardés à l'avance, aux largeurs de Taodbox et rangés comme `visuel` les range. Un visuel déjà gardé ne coûte
    /// rien et ne compte pas ; au plus `plafond` jeux TÉLÉCHARGÉS par passage (les suivants au passage d'après) ; un
    /// visuel que Firehouse n'a pas (404) fait passer au jeu suivant ; hors ligne ou serveur en panne : arrêt. Un seul
    /// passage à la fois. Rend le nombre de jeux dont un visuel a été reçu.
    pub async fn precharger_taodbox(&self, plafond: usize) -> Resultat<usize> {
        static EN_COURS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if EN_COURS.swap(true, std::sync::atomic::Ordering::SeqCst) {
            return Ok(0);
        }
        let r = self.precharger(plafond).await;
        EN_COURS.store(false, std::sync::atomic::Ordering::SeqCst);
        r
    }

    async fn precharger(&self, plafond: usize) -> Resultat<usize> {
        let s = self.session().await?;
        if s.source.est_simulee() {
            return Ok(0);
        }
        let jeux: Vec<i64> = self.registre().tous()?.into_iter().filter(|j| j.etat == crate::jeux_pc::Etat::Telecharge).map(|j| j.id).collect();
        let mut recus = 0;
        'jeux: for id in jeux {
            if recus >= plafond {
                break;
            }
            let Some(images) = s.verrou().jeu(id)?.and_then(|j| j.images) else { continue };
            let du_jeu = s.dossier.join("visuels").join(id.to_string());
            let mut recu = false;
            for (chaine, largeur) in [(&["fond", "capture", "ecran_titre"][..], 1000u32), (&["logo"][..], 600)] {
                // Le premier type de la chaîne que le jeu a : celui que Taodbox montrera.
                let Some((sorte, image)) = chaine.iter().find_map(|t| images.get(*t).map(|i| (*t, i))) else { continue };
                let Some(empreinte) = image.empreinte.as_deref().map(propre).filter(|e| !e.is_empty()) else { continue };
                let fichier = du_jeu.join(format!("{sorte}-{largeur}-{empreinte}.img"));
                if fichier.is_file() {
                    continue;
                }
                match s.source.media(id, sorte, Some(largeur)).await {
                    Ok(Some(img)) => {
                        if ecrire(&fichier, &img.octets).is_ok() {
                            // Les anciennes versions de ce visuel partent (comme dans `visuel`).
                            if let Ok(l) = std::fs::read_dir(&du_jeu) {
                                for e in l.flatten() {
                                    let n = e.file_name().to_string_lossy().to_string();
                                    if n.starts_with(&format!("{sorte}-")) && n.ends_with(".img") && !n.ends_with(&format!("-{empreinte}.img")) {
                                        let _ = std::fs::remove_file(e.path());
                                    }
                                }
                            }
                        }
                        recu = true;
                    }
                    Ok(None) => {} // Firehouse ne l'a pas (ou plus) : le jeu suivant
                    Err(Erreur::Reseau(_) | Erreur::Serveur(_)) => {
                        self.journaliser(&format!("Taodbox : fonds et logos gardés à l'avance arrêtés au jeu {id} (hors ligne ou Firehouse indisponible)"));
                        if recu {
                            recus += 1;
                        }
                        break 'jeux;
                    }
                    Err(e) => return Err(e),
                }
            }
            if recu {
                recus += 1;
            }
        }
        if recus > 0 {
            self.journaliser(&format!("Taodbox : fonds et logos de {recus} jeu(x) gardés à l'avance"));
        }
        Ok(recus)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coffre::CoffreMemoire;
    use crate::noyau::Connexion;
    use httpmock::prelude::*;
    use serde_json::json;

    #[test]
    fn la_liste_garde_les_visuels_utilisables_un_par_un() {
        // L'exemple réel (3x3 Eyes, jeu 116), plus des éléments à écarter.
        let v = json!({"ok": true, "id": 116, "medias_empreinte": "9301a836222a4b31", "medias": [
            {"id": 509030, "type": "jaquette", "principal": true, "region": "Japan", "largeur": 1281, "hauteur": 2311, "empreinte": "aec7e68e65fb"},
            {"id": 509043, "type": "capture", "principal": true, "region": "Japan", "largeur": 256, "hauteur": 224, "empreinte": "449e182b…"},
            {"id": 7, "type": "../../moi", "empreinte": "x"},
            {"id": 8, "type": "capture"},
            {"id": "neuf"}
        ]});
        let l = lire_medias(&v);
        assert_eq!(l.iter().map(|m| m.id).collect::<Vec<_>>(), [509030, 509043]);
        assert_eq!(l[1].empreinte, "449e182b", "assainie pour un nom de fichier");
        // Un jeu non rapatrié : un tableau nu, vide.
        assert!(lire_medias(&json!([])).is_empty());
    }

    #[tokio::test]
    async fn la_galerie_se_garde_par_profil_suit_son_empreinte_et_oublie_ce_qui_est_retire() {
        let serveur = MockServer::start();
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/plateformes");
            t.status(200).json_body(json!({"ok": true, "plateformes": [{"nom": "SNES", "jeux": 1}]}));
        });
        let mut catalogue = serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue");
            t.status(200).json_body(json!({"ok": true, "suivante": null, "jeux": [{"id": 116, "titre": "3x3 Eyes", "plateforme": "SNES", "medias_empreinte": "e1"}]}));
        });
        let mut liste = serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/jeu/116/medias");
            t.status(200).json_body(json!({"ok": true, "medias_empreinte": "e1", "medias": [
                {"id": 1, "type": "capture", "principal": true, "empreinte": "c1", "largeur": 256, "hauteur": 224},
                {"id": 2, "type": "capture", "principal": false, "empreinte": "c2", "largeur": 256, "hauteur": 224}]}));
        });
        let deux = serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/media/116/capture").query_param("id", "2").query_param("largeur", "300");
            t.status(200).header("Content-Type", "image/webp").body(b"RIFF\0\0\0\0WEBPVP8 deux".to_vec());
        });
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(CoffreMemoire::default())).unwrap();
        let p = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        n.ouvrir(&p, None, &Connexion { adresse: serveur.base_url(), simule: false }).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();

        // Avant la liste : aucune image n'est demandée.
        assert!(n.media_precis(116, 2, 300).await.unwrap().is_none());
        deux.assert_hits(0);
        assert_eq!(n.medias(116).await.unwrap().len(), 2);
        // Même empreinte que le catalogue : la liste gardée, sans appel.
        n.medias(116).await.unwrap();
        liste.assert_hits(1);
        // L'image PRÉCISE (la 2e capture), gardée sous son empreinte ; un id inconnu : jamais demandé.
        assert!(n.media_precis(116, 2, 300).await.unwrap().unwrap().octets.ends_with(b"deux"));
        assert!(n.media_precis(116, 2, 300).await.unwrap().is_some());
        deux.assert_hits(1);
        assert!(n.media_precis(116, 99, 300).await.unwrap().is_none());
        let galerie = n.session().await.unwrap().dossier.join("visuels").join("116");
        assert!(galerie.join("m2-300-c2.img").is_file());

        // Un autre profil (qui n'a pas ce jeu) n'obtient rien.
        let lea = n.creer_profil("Léa", None, Some("j2")).unwrap().id;
        n.ouvrir(&lea, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        assert!(n.medias(116).await.is_err());
        assert!(n.media_precis(116, 2, 300).await.unwrap_or(None).is_none());
        n.ouvrir(&p, None, &Connexion { adresse: serveur.base_url(), simule: false }).await.unwrap();

        // L'admin retire la 2e capture (le catalogue change d'empreinte) : la liste est relue, son image part.
        liste.delete();
        liste = serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/jeu/116/medias");
            t.status(200).json_body(json!({"ok": true, "medias_empreinte": "e2", "medias": [
                {"id": 1, "type": "capture", "principal": true, "empreinte": "c1"}]}));
        });
        n.session().await.unwrap().verrou().ecrire_meta("schema_catalogue", "1").unwrap();
        catalogue.delete();
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue");
            t.status(200).json_body(json!({"ok": true, "suivante": null, "jeux": [{"id": 116, "titre": "3x3 Eyes", "plateforme": "SNES", "medias_empreinte": "e2"}]}));
        });
        n.synchroniser(|_, _| {}).await.unwrap();
        assert_eq!(n.medias(116).await.unwrap().len(), 1);
        liste.assert_hits(1);
        assert!(!galerie.join("m2-300-c2.img").exists(), "une image retirée quitte le cache");

        // Hors ligne : la liste gardée.
        n.reconnecter(&Connexion { adresse: "http://127.0.0.1:9".into(), simule: false }).await.unwrap();
        assert_eq!(n.medias(116).await.unwrap().len(), 1);

        // Le jeu devient caché pour ce profil (404) : tout ce qui en était gardé part.
        n.reconnecter(&Connexion { adresse: serveur.base_url(), simule: false }).await.unwrap();
        std::fs::write(galerie.join("boite_3d-400-b1.img"), b"x").unwrap();
        liste.delete();
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/jeu/116/medias");
            t.status(404).json_body(json!({"ok": false}));
        });
        // (La liste gardée retirée : sinon, même empreinte que le catalogue, elle serait servie sans appel.)
        std::fs::remove_file(galerie.join("medias.json")).unwrap();
        assert!(n.medias(116).await.is_err());
        assert!(!galerie.exists(), "plus rien de ce jeu n'est gardé pour ce profil");
    }
}
