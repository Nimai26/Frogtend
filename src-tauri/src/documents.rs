//! Les documents d'un jeu (contrat 1.7) : livret (PDF), musique (MP3), vidéos (MP4). La liste est relue chez Firehouse
//! à chaque ouverture de la fiche (en ligne) et gardée pour le hors-ligne. Un document n'est téléchargé qu'au clic de
//! la personne (sa taille est sur le bouton : c'est son accord pour CE document), type et empreinte vérifiés avant de
//! le garder : avec le jeu s'il est sur le PC (règle « pas de pieuvre »), sinon dans le dossier DU PROFIL (rien de ce
//! qui a été reçu pour un profil ne se montre à un autre).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::erreurs::{Erreur, Resultat};
use crate::noyau::{Noyau, Session};
use crate::source::Image;

/// Un document d'un jeu, tel que Firehouse le décrit (jamais un chemin).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub id: i64,
    #[serde(rename = "type")]
    pub sorte: String,
    #[serde(default)]
    pub langue: String,
    pub format: String,
    #[serde(default)]
    pub taille: u64,
    #[serde(default)]
    pub pages: Option<u32>,
    #[serde(default)]
    pub duree_s: Option<f64>,
    #[serde(default)]
    pub empreinte: String,
    /// Déjà sur le PC (rien à télécharger au clic) : calculé par Frogtend, jamais reçu.
    #[serde(default, skip_deserializing)]
    pub garde: bool,
}

/// Les sortes connues, leur format et le type annoncé par Firehouse (contrat 1.7 : `Content-Type` vérifié).
const SORTES: &[(&str, &str, &str)] = &[
    ("livret", "pdf", "application/pdf"),
    ("musique", "mp3", "audio/mpeg"),
    ("video", "mp4", "video/mp4"),
    ("video_theme", "mp4", "video/mp4"),
];

fn type_attendu(d: &Document) -> Option<&'static str> {
    SORTES.iter().find(|(s, f, _)| *s == d.sorte && *f == d.format).map(|(_, _, t)| *t)
}

/// Les documents utilisables d'une réponse (`{documents: [...]}`), élément par élément : un élément illisible, d'une
/// sorte inconnue ou d'un format inattendu est laissé de côté sans perdre les autres.
pub fn lire_liste(v: &Value) -> Vec<Document> {
    let elements = v["documents"].as_array().or_else(|| v.as_array()).cloned().unwrap_or_default();
    elements
        .into_iter()
        .filter_map(|e| serde_json::from_value::<Document>(e).ok())
        .filter(|d| d.id > 0 && type_attendu(d).is_some())
        .collect()
}

/// Une empreinte sha256 bien formée (64 chiffres hexadécimaux), sinon `None` (rien à comparer).
fn empreinte_valable(e: &str) -> Option<&str> {
    (e.len() == 64 && e.chars().all(|c| c.is_ascii_hexdigit())).then_some(e)
}

/// Un seul téléchargement de document à la fois : deux demandes du même livret n'écrivent jamais le même fichier.
static TELECHARGEMENT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

impl Noyau {
    /// La liste des documents gardée pour CE profil (jamais partagée : Firehouse pourrait la filtrer selon le grade).
    fn liste_gardee(s: &Session, id: i64) -> PathBuf {
        s.dossier.join("documents").join(id.to_string()).join("documents.json")
    }

    /// Où va un document téléchargé : avec le jeu s'il est sur le PC (règle « pas de pieuvre »), sinon dans le
    /// dossier du profil.
    fn dossier_fichiers(&self, s: &Session, id: i64) -> PathBuf {
        match self.registre().jeu(id).ok().flatten() {
            Some(j) => crate::locale::medias_du_jeu(&j).join("documents"),
            None => s.dossier.join("documents").join(id.to_string()),
        }
    }

    /// Le fichier d'un document : là où il doit aller, ou déjà gardé pour le profil avant que le jeu ne vienne sur le
    /// PC (réutilisé tel quel, jamais retéléchargé).
    fn fichier_du_document(&self, s: &Session, id: i64, d: &Document) -> PathBuf {
        let nom = format!("{}.{}", d.id, d.format);
        let place = self.dossier_fichiers(s, id).join(&nom);
        let profil = s.dossier.join("documents").join(id.to_string()).join(&nom);
        if !place.is_file() && profil.is_file() {
            profil
        } else {
            place
        }
    }

    /// La session, si ce jeu est visible du profil ouvert ; sinon « introuvable » (comme Firehouse : 404).
    async fn session_du_jeu(&self, id: i64) -> Resultat<std::sync::Arc<Session>> {
        let s = self.session().await?;
        if id <= 0 || !s.verrou().contient(id)? {
            return Err(Erreur::Introuvable("Ce jeu ne t'est pas visible.".into()));
        }
        Ok(s)
    }

    /// Les documents d'un jeu : relus chez Firehouse, gardés (s'il y en a) pour le hors-ligne ; hors ligne, ceux gardés.
    pub async fn documents(&self, id: i64) -> Resultat<Vec<Document>> {
        let s = self.session_du_jeu(id).await?;
        let gardee = Self::liste_gardee(&s, id);
        let marquer = |mut l: Vec<Document>| {
            for d in &mut l {
                d.garde = self.fichier_du_document(&s, id, d).is_file();
            }
            l
        };
        match s.source.documents(id).await {
            Ok(v) => {
                let liste = lire_liste(&v);
                // Garder est un plus (le hors-ligne) ; une liste vide ne crée aucun dossier.
                if !liste.is_empty() || gardee.exists() {
                    if let Err(e) = std::fs::create_dir_all(gardee.parent().unwrap())
                        .and_then(|_| std::fs::write(&gardee, serde_json::to_vec_pretty(&v).unwrap()))
                    {
                        self.journaliser(&format!("documents du jeu {id} : liste non gardée ({e})"));
                    }
                }
                Ok(marquer(liste))
            }
            Err(e @ (Erreur::Reseau(_) | Erreur::Serveur(_))) => match std::fs::read(&gardee) {
                Ok(o) => Ok(marquer(serde_json::from_slice::<Value>(&o).map(|v| lire_liste(&v)).unwrap_or_default())),
                Err(_) => Err(e),
            },
            Err(e) => Err(e),
        }
    }

    /// Un document de la liste GARDÉE pour ce profil (`relire` : relue chez Firehouse si elle ne l'est pas encore),
    /// et son fichier.
    async fn document(&self, s: &Session, id: i64, doc: i64, relire: bool) -> Resultat<Option<(Document, PathBuf)>> {
        let liste = match std::fs::read(Self::liste_gardee(s, id)) {
            Ok(o) => serde_json::from_slice::<Value>(&o).map(|v| lire_liste(&v)).unwrap_or_default(),
            Err(_) if relire => self.documents(id).await?,
            Err(_) => return Ok(None),
        };
        Ok(liste.into_iter().find(|d| d.id == doc).map(|d| {
            let f = self.fichier_du_document(s, id, &d);
            (d, f)
        }))
    }

    /// Le livret d'un jeu, téléchargé s'il ne l'est pas encore (vérifié), puis ouvert : rend son nombre de pages.
    pub async fn livret(&self, id: i64, doc: i64) -> Resultat<u32> {
        let s = self.session_du_jeu(id).await?;
        let (d, fichier) = self
            .document(&s, id, doc, true)
            .await?
            .ok_or_else(|| Erreur::Introuvable("Ce document a changé chez Firehouse : ferme et rouvre la fiche du jeu.".into()))?;
        if d.sorte != "livret" {
            return Err(Erreur::Refus("Ce document n'est pas un livret.".into()));
        }
        if !fichier.is_file() {
            let _un_seul = TELECHARGEMENT.lock().await;
            if !fichier.is_file() {
                let types = [type_attendu(&d).unwrap_or("application/pdf")];
                match s.source.telecharger_document(id, doc, &fichier, empreinte_valable(&d.empreinte), &types).await {
                    Ok(n) if d.taille > 0 && n != d.taille => {
                        let _ = std::fs::remove_file(&fichier);
                        return Err(Erreur::Serveur("Le livret reçu n'a pas la taille annoncée : il n'est pas gardé.".into()));
                    }
                    Ok(_) => {}
                    // Un 404 : le document a été remplacé chez Firehouse (un contenu nouveau a un nouvel id).
                    Err(Erreur::Introuvable(_)) => {
                        let _ = self.documents(id).await;
                        return Err(Erreur::Introuvable("Ce document a changé chez Firehouse : ferme et rouvre la fiche du jeu.".into()));
                    }
                    Err(e) => return Err(e),
                }
            }
        }
        let f = fichier.clone();
        let pages = tauri::async_runtime::spawn_blocking(move || crate::livret::pages(&f))
            .await
            .map_err(|_| Erreur::Disque("La lecture du livret s'est arrêtée.".into()))?;
        match pages {
            Ok(0) => Err(Erreur::Refus("Ce livret n'a aucune page.".into())),
            Ok(n) => Ok(n),
            Err(e) => {
                // Retiré SEULEMENT s'il est prouvé abîmé (empreinte différente) : il sera repris au prochain clic. Un
                // livret intact que Windows ne sait pas lire est gardé (sinon on le retéléchargerait à chaque clic).
                let attendue = empreinte_valable(&d.empreinte);
                let f = fichier.clone();
                let abime = match attendue {
                    Some(a) => tauri::async_runtime::spawn_blocking(move || crate::sauvegarde::empreinte(&f))
                        .await
                        .ok()
                        .and_then(|r| r.ok())
                        .is_some_and(|e| !e.eq_ignore_ascii_case(a)),
                    None => false,
                };
                if abime {
                    let _ = std::fs::remove_file(&fichier);
                    return Err(Erreur::Disque("Le livret gardé sur le PC était abîmé : il est retiré, et sera repris au prochain essai.".into()));
                }
                self.journaliser(&format!("livret {id}/{doc} illisible : {e:?}"));
                Err(Erreur::Disque("Windows ne sait pas lire ce livret.".into()))
            }
        }
    }

    /// La page `n` d'un livret DÉJÀ sur le PC, en image (jamais de téléchargement ni d'appel à Firehouse ici : c'est
    /// `livret` qui le fait).
    pub async fn page_livret(&self, id: i64, doc: i64, n: u32, largeur: u32) -> Resultat<Option<Image>> {
        let s = self.session_du_jeu(id).await?;
        let Some((d, fichier)) = self.document(&s, id, doc, false).await? else { return Ok(None) };
        if d.sorte != "livret" || !fichier.is_file() {
            return Ok(None);
        }
        let png = tauri::async_runtime::spawn_blocking(move || crate::livret::page_png(&fichier, n, largeur))
            .await
            .map_err(|_| Erreur::Disque("La lecture du livret s'est arrêtée.".into()))??;
        Ok(Some(Image { octets: png, type_contenu: Some("image/png".into()) }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coffre::CoffreMemoire;
    use crate::noyau::Connexion;
    use httpmock::prelude::*;
    use serde_json::json;

    async fn noyau(adresse: String, simule: bool) -> (tempfile::TempDir, Noyau) {
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse, simule }).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();
        (d, n)
    }

    /// RÉEL, lecture seule chez Firehouse : la liste des documents d'un jeu, puis son livret téléchargé par le code de
    /// Frogtend (type et empreinte vérifiés) dans `FROGTEND_SORTIE`, et deux pages rendues en PNG à côté.
    /// `FROGTEND_PROFIL_ESSAI=<id> FROGTEND_JEU=111 FROGTEND_SORTIE=<dossier> cargo test essai_livret -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_livret() {
        use crate::coffre::Coffre as _;
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let jeu: i64 = std::env::var("FROGTEND_JEU").unwrap().parse().unwrap();
        let sortie = PathBuf::from(std::env::var("FROGTEND_SORTIE").unwrap());
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton");
        let source = crate::source::Source::Firehouse(crate::firehouse::Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap());
        let liste = lire_liste(&source.documents(jeu).await.unwrap());
        println!("DOCUMENTS {:?}", liste.iter().map(|d| (d.id, &d.sorte, d.taille, d.pages)).collect::<Vec<_>>());
        let d = liste.iter().find(|d| d.sorte == "livret").expect("pas de livret");
        let pdf = sortie.join(format!("{}.pdf", d.id));
        let debut = std::time::Instant::now();
        let n = source.telecharger_document(jeu, d.id, &pdf, empreinte_valable(&d.empreinte), &[type_attendu(d).unwrap()]).await.unwrap();
        println!("TELECHARGE {n} octets en {:?} (annoncé {})", debut.elapsed(), d.taille);
        let pages = crate::livret::pages(&pdf).unwrap();
        println!("PAGES {pages} (annoncé {:?})", d.pages);
        for p in [0, pages / 2] {
            let debut = std::time::Instant::now();
            let png = crate::livret::page_png(&pdf, p, 1000).unwrap();
            std::fs::write(sortie.join(format!("page-{p}.png")), &png).unwrap();
            println!("PAGE {p} : {} octets en {:?}", png.len(), debut.elapsed());
        }
    }

    #[tokio::test]
    async fn en_mode_simule_le_livret_de_dune_se_lit_page_par_page() {
        let (d, n) = noyau(String::new(), true).await;
        let l = n.documents(110).await.unwrap();
        assert_eq!(l.iter().map(|d| d.sorte.as_str()).collect::<Vec<_>>(), ["livret", "musique", "video_theme"]);
        // Avant le clic : aucune page (rien n'est téléchargé sans la personne).
        assert!(n.page_livret(110, 9001, 0, 600).await.unwrap().is_none());
        assert!(!l[0].garde);
        assert_eq!(n.livret(110, 9001).await.unwrap(), 2);
        assert!(n.documents(110).await.unwrap()[0].garde, "désormais sur le PC");
        let img = n.page_livret(110, 9001, 1, 600).await.unwrap().unwrap();
        assert!(img.octets.starts_with(b"\x89PNG"));
        // Gardé dans le dossier DU PROFIL (le jeu n'est pas sur le PC), nulle part ailleurs.
        let gardes: Vec<_> = walkdir(d.path()).into_iter().filter(|p| p.extension().is_some_and(|e| e == "pdf")).collect();
        assert_eq!(gardes.len(), 1);
        assert!(gardes[0].ends_with(std::path::Path::new("documents").join("110").join("9001.pdf")), "{gardes:?}");
        // Un document qui n'est pas un livret, un document inconnu, un jeu invisible : refusés.
        assert!(n.livret(110, 9002).await.is_err());
        assert!(matches!(n.livret(110, 4242).await, Err(Erreur::Introuvable(_))));
        assert!(matches!(n.documents(999_999).await, Err(Erreur::Introuvable(_))));
    }

    fn walkdir(d: &std::path::Path) -> Vec<PathBuf> {
        let mut v = vec![];
        for e in std::fs::read_dir(d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                v.extend(walkdir(&p));
            } else {
                v.push(p);
            }
        }
        v
    }

    fn serveur(livret: &'static [u8], type_contenu: &'static str, empreinte: &str) -> MockServer {
        let s = MockServer::start();
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/plateformes");
            t.status(200).json_body(json!({"ok": true, "plateformes": [{"nom": "Nintendo 64", "jeux": 1}]}));
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue");
            t.status(200).json_body(json!({"ok": true, "suivante": null, "jeux": [{"id": 111, "titre": "Zelda", "plateforme": "Nintendo 64"}]}));
        });
        let e = empreinte.to_string();
        s.mock(move |w, t| {
            w.method(GET).path("/api/jeux/v1/jeu/111/documents");
            t.status(200).json_body(json!({"ok": true, "id": 111, "documents_empreinte": "a", "documents": [
                {"id": 6142, "type": "livret", "langue": "", "format": "pdf", "taille": livret.len(), "pages": 2, "empreinte": e, "source": "launchbox_local"}]}));
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/jeu/111/document/6142");
            t.status(200).header("content-type", type_contenu).body(livret);
        });
        s
    }

    fn sha(o: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(o))
    }

    #[tokio::test]
    async fn un_livret_n_est_garde_que_si_son_type_et_son_empreinte_sont_les_bons() {
        let pdf: &'static [u8] = Box::leak(crate::livret::tests::pdf_de_test(2).into_boxed_slice());
        // Le bon : gardé, 2 pages ; la liste est gardée pour le hors-ligne.
        let s = serveur(pdf, "application/pdf", &sha(pdf));
        let (_d, n) = noyau(s.base_url(), false).await;
        assert_eq!(n.documents(111).await.unwrap().len(), 1);
        assert_eq!(n.livret(111, 6142).await.unwrap(), 2);
        // Hors ligne : la liste gardée, le livret gardé.
        n.reconnecter(&Connexion { adresse: "http://127.0.0.1:9".into(), simule: false }).await.unwrap();
        assert_eq!(n.documents(111).await.unwrap().len(), 1);
        assert!(n.page_livret(111, 6142, 0, 400).await.unwrap().is_some());

        // Une empreinte qui ne correspond pas : rien de gardé, l'erreur le dit.
        let s = serveur(pdf, "application/pdf", &"0".repeat(64));
        let (d, n) = noyau(s.base_url(), false).await;
        n.documents(111).await.unwrap();
        let e = n.livret(111, 6142).await.unwrap_err();
        assert!(e.to_string().contains("abîmé"), "{e}");
        assert!(!walkdir(d.path()).iter().any(|p| p.extension().is_some_and(|x| x == "pdf" || x == "part")));

        // Un autre type que celui annoncé (une page HTML à la place du PDF) : refusé avant d'écrire.
        let s = serveur(b"<html>", "text/html", &sha(b"<html>"));
        let (d, n) = noyau(s.base_url(), false).await;
        n.documents(111).await.unwrap();
        assert!(n.livret(111, 6142).await.is_err());
        assert!(!walkdir(d.path()).iter().any(|p| p.extension().is_some_and(|x| x == "pdf" || x == "part")));
    }

    #[test]
    fn la_liste_garde_les_documents_utilisables_un_par_un() {
        // L'exemple réel du 16/10 (Zelda, /jeu/111/documents), plus des éléments à écarter.
        let v = json!({"ok": true, "id": 111, "documents_empreinte": "x", "documents": [
            {"id": 6142, "type": "livret", "langue": "", "format": "pdf", "taille": 8583117, "pages": 23, "duree_s": null, "empreinte": "4adbac82181b87dc29e054452203e186a55cc557088e6b844ce90944d56fdd82", "source": "launchbox_local"},
            {"id": 6482, "type": "musique", "langue": "", "format": "mp3", "taille": 6110240, "pages": null, "duree_s": 152.6, "empreinte": "5861563a33320b58d944cd74c846900ccc298149b3873c317ffe8c0a7ca1e365", "source": "launchbox_local"},
            {"id": 1, "type": "livret", "format": "exe"},
            {"id": 2, "type": "poster", "format": "png"},
            {"id": "trois"},
            {"id": 0, "type": "livret", "format": "pdf"}
        ]});
        let l = lire_liste(&v);
        assert_eq!(l.iter().map(|d| d.id).collect::<Vec<_>>(), [6142, 6482]);
        assert_eq!(l[0].pages, Some(23));
        assert_eq!(l[1].duree_s, Some(152.6));
        assert_eq!(empreinte_valable(&l[0].empreinte), Some(l[0].empreinte.as_str()));
        assert_eq!(empreinte_valable("abc"), None);
        // La forme écrite dans le contrat (un tableau) se lit aussi.
        assert_eq!(lire_liste(&json!([{"id": 5, "type": "video", "format": "mp4"}])).len(), 1);
    }
}
