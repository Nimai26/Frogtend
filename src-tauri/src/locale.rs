//! La ludothèque LOCALE : mettre un jeu sur le PC (ses fichiers ET ses médias), la file de téléchargements, et
//! tout ce qui rend un jeu de la ludothèque utilisable sans Internet (fiche, jaquette, documents).
//!
//! Médias d'un jeu : `medias/<id>/` dans le dossier de l'application — `fiche.json`, `jaquette.img` (en grand),
//! `jaquette-400.img` (miniature), `annexes/` (textes en JSON, fichiers tels quels).

use crate::erreurs::{Erreur, Resultat};
use crate::firehouse::{Client, Reponse};
use crate::jeux_pc::{
    nom_de_dossier, nom_de_fichier_sur, place_libre, proposer, Emplacements, EmplacementPropose, Etat, FichierJeu,
    JeuPc, MARGE_LIBRE,
};
use crate::noyau::{maintenant, Noyau, Session};
use crate::source::{encoder, Source};
use crate::telechargements::{telecharger_jeu, Issue, Progres};
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Largeur de la miniature gardée pour la grille (hors ligne).
pub const LARGEUR_MINIATURE: u32 = 400;

/// Ce que l'interface reçoit de la file de téléchargements.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "sorte", rename_all = "snake_case")]
pub enum Evenement {
    Progres(Progres),
    Etat { jeu: i64, etat: Etat, message: Option<String> },
}

/// Un jeu du PC tel que l'interface le voit : avec les octets déjà sur le disque.
#[derive(Debug, Clone, Serialize)]
pub struct JeuPcVu {
    #[serde(flatten)]
    pub jeu: JeuPc,
    pub recus: u64,
}

/// L'extension d'un document : d'après son CONTENU d'abord (Firehouse peut l'envoyer sans type précis), sinon
/// d'après son type.
fn extension(octets: &[u8], type_contenu: Option<&str>) -> &'static str {
    if octets.starts_with(b"%PDF") {
        return "pdf";
    }
    if octets.starts_with(b"PK") {
        return "zip";
    }
    if octets.starts_with(b"Rar!") {
        return "rar";
    }
    if octets.starts_with(&[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C]) {
        return "7z";
    }
    if let Some(t) = crate::noyau::type_image(octets) {
        return match t.as_str() {
            "image/png" => "png",
            "image/jpeg" => "jpg",
            "image/webp" => "webp",
            _ => "gif",
        };
    }
    let t = type_contenu.unwrap_or("").split(';').next().unwrap_or("").trim().to_ascii_lowercase();
    match t.as_str() {
        "application/pdf" => "pdf",
        "application/zip" | "application/x-zip-compressed" => "zip",
        "application/x-7z-compressed" => "7z",
        "application/vnd.rar" | "application/x-rar-compressed" => "rar",
        "text/plain" => "txt",
        "text/html" => "html",
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "bin",
    }
}

fn client(s: &Session) -> Resultat<&Client> {
    match &s.source {
        Source::Firehouse(c) => Ok(c),
        Source::Simulee => Err(Erreur::Refus(
            "En mode simulé, on ne peut pas mettre de jeu sur le PC : il faut le vrai Firehouse.".into(),
        )),
    }
}

impl Noyau {
    pub fn dossier_medias(&self, id: i64) -> PathBuf {
        self.dossier.join("medias").join(id.to_string())
    }

    /// Les jeux du PC que le profil ouvert a le droit de voir (ceux de son catalogue), avec leur progression.
    pub async fn jeux_du_pc(&self) -> Resultat<Vec<JeuPcVu>> {
        let s = self.session().await?;
        let tous = self.registre().tous()?;
        let c = s.verrou();
        let mut vus = Vec::new();
        for j in tous {
            if c.contient(j.id)? {
                let recus = j.recus();
                vus.push(JeuPcVu { jeu: j, recus });
            }
        }
        Ok(vus)
    }

    pub fn proposer_emplacements(&self, emplacements: &Emplacements, plateforme: &str, taille: u64) -> Vec<EmplacementPropose> {
        proposer(emplacements, plateforme, taille)
    }

    /// Met un jeu dans la ludothèque de ce PC : vérifie la version et la place, garde ses médias (fiche, jaquette,
    /// documents), puis le met en file de téléchargement. L'appelant lance ensuite la file.
    pub async fn ajouter(
        &self,
        id: i64,
        version: i64,
        emplacement: &str,
        emplacements: &Emplacements,
    ) -> Resultat<JeuPc> {
        let s = self.session().await?;
        let c = client(&s)?;
        if self.registre().jeu(id)?.is_some() {
            return Err(Erreur::Refus("Ce jeu est déjà dans la ludothèque de ce PC.".into()));
        }
        let fiche = s.source.fiche(id).await?;
        let titre = fiche["titre"].as_str().unwrap_or("Jeu").to_string();
        let plateforme = fiche["plateforme"].as_str().unwrap_or("Autres").to_string();

        // L'emplacement doit être l'un de ceux réglés pour ce système : l'interface ne peut pas écrire ailleurs.
        if !emplacements.pour(&plateforme).iter().any(|e| e == emplacement) {
            return Err(Erreur::Reglage(format!(
                "« {emplacement} » n'est pas un emplacement réglé pour {plateforme}. Règle-le dans « Réglages »."
            )));
        }
        let v = fiche["versions"]
            .as_array()
            .and_then(|l| l.iter().find(|v| v["telechargement_id"].as_i64() == Some(version)))
            .ok_or_else(|| Erreur::Introuvable("Cette version n'existe plus dans Firehouse : relis la fiche.".into()))?;
        let mut fichiers = Vec::new();
        for (n, f) in v["fichiers"].as_array().cloned().unwrap_or_default().iter().enumerate() {
            let nom = nom_de_fichier_sur(f["nom"].as_str().unwrap_or(""))?.to_string();
            fichiers.push(FichierJeu { n: n as u32, nom, taille: f["taille"].as_u64().unwrap_or(0) });
        }
        if fichiers.is_empty() {
            return Err(Erreur::Refus("Cette version n'a aucun fichier à télécharger.".into()));
        }
        let total: u64 = fichiers.iter().map(|f| f.taille).sum();

        let libre = place_libre(Path::new(emplacement)).ok_or_else(|| {
            Erreur::Disque(format!("Dossier introuvable : {emplacement} (disque débranché ou partage absent ?)."))
        })?;
        if libre < total + MARGE_LIBRE {
            return Err(Erreur::Disque(format!(
                "Pas assez de place dans {emplacement} : il faut {} Mo (plus 1 Go de marge), il en reste {} Mo.",
                total / 1_000_000,
                libre / 1_000_000
            )));
        }

        let nom = match fiche["annee"].as_i64() {
            Some(a) => format!("{titre} ({a})"),
            None => titre.clone(),
        };
        let base = Path::new(emplacement).join(nom_de_dossier(&plateforme));
        let mut dossier = base.join(nom_de_dossier(&nom));
        let occupe = |d: &Path| std::fs::read_dir(d).is_ok_and(|mut l| l.next().is_some());
        if occupe(&dossier) {
            dossier = base.join(nom_de_dossier(&format!("{nom} [{id}]")));
            if occupe(&dossier) {
                return Err(Erreur::Refus(format!(
                    "Le dossier {} existe déjà et n'est pas vide : Frogtend n'écrit pas par-dessus.",
                    dossier.display()
                )));
            }
        }

        self.garder_medias(&s, c, id, &fiche).await?;

        let jeu = JeuPc {
            id,
            version,
            titre,
            plateforme,
            dossier: dossier.to_string_lossy().into(),
            etat: Etat::Attente,
            total,
            fichiers,
            message: None,
            ajoute_le: maintenant(),
            ajoute_par: s.profil.id.clone(),
            installation: None,
            temps_jeu: 0,
            derniere_partie: None,
        };
        self.registre().ajouter(&jeu)?;
        Ok(jeu)
    }

    /// Garde sur le PC tout ce qui sert au jeu hors ligne : fiche, jaquette (en grand et en miniature), documents.
    async fn garder_medias(&self, s: &Session, c: &Client, id: i64, fiche: &Value) -> Resultat<()> {
        let dossier = self.dossier_medias(id);
        std::fs::create_dir_all(dossier.join("annexes"))?;
        std::fs::write(dossier.join("fiche.json"), serde_json::to_vec_pretty(fiche).unwrap())?;
        if let Some(img) = s.source.media(id, "jaquette", None).await? {
            std::fs::write(dossier.join("jaquette.img"), &img.octets)?;
        }
        if let Some(img) = s.source.media(id, "jaquette", Some(LARGEUR_MINIATURE)).await? {
            std::fs::write(dossier.join(format!("jaquette-{LARGEUR_MINIATURE}.img")), &img.octets)?;
        }
        for a in fiche["annexes"].as_array().cloned().unwrap_or_default() {
            let (Some(i), Some(cle)) = (a["i"].as_u64(), a["cle"].as_str()) else { continue };
            let route = format!("/annexe/{id}/{i}?cle={}", encoder(cle));
            if a["texte"].as_bool() == Some(true) {
                let v: Value = c.obtenir_json(&route).await?;
                std::fs::write(dossier.join("annexes").join(format!("{i}.json")), serde_json::to_vec_pretty(&v).unwrap())?;
            } else if let Reponse::Corps { octets, type_contenu, .. } = c.obtenir(&route, None).await? {
                let titre = nom_de_dossier(a["titre"].as_str().unwrap_or("Document"));
                let nom = format!("{i} - {titre}.{}", extension(&octets, type_contenu.as_deref()));
                std::fs::write(dossier.join("annexes").join(nom), &octets)?;
            }
        }
        Ok(())
    }

    /// Fait tourner la file : télécharge les jeux en attente l'un après l'autre, avec le jeton de la session donnée.
    /// Ne fait rien si la file tourne déjà.
    pub async fn executer_file(&self, s: Arc<Session>, emettre: &(dyn Fn(Evenement) + Send + Sync)) -> Resultat<()> {
        if self.file_active.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        let resultat = self.boucle_file(&s, emettre).await;
        self.file_active.store(false, Ordering::SeqCst);
        resultat
    }

    async fn boucle_file(&self, s: &Session, emettre: &(dyn Fn(Evenement) + Send + Sync)) -> Resultat<()> {
        let c = client(s)?;
        loop {
            let Some(jeu) = self.registre().prochain()? else { return Ok(()) };
            let arret = Arc::new(AtomicBool::new(false));
            *self.en_cours.lock().unwrap_or_else(|e| e.into_inner()) = Some((jeu.id, arret.clone()));
            self.registre().changer_etat(jeu.id, Etat::EnCours, None)?;
            emettre(Evenement::Etat { jeu: jeu.id, etat: Etat::EnCours, message: None });

            let issue = telecharger_jeu(c, &jeu, &arret, &|p| emettre(Evenement::Progres(p))).await;
            *self.en_cours.lock().unwrap_or_else(|e| e.into_inner()) = None;
            let (etat, message) = match issue {
                Ok(Issue::Termine) => (Etat::Telecharge, None),
                Ok(Issue::Pause) => (Etat::Pause, None),
                Err(e) => {
                    self.journaliser(&format!("téléchargement du jeu {} : {e:?}", jeu.id));
                    (Etat::Erreur, Some(e.to_string()))
                }
            };
            // Un jeu annulé pendant le téléchargement n'est plus dans le registre : on n'y réécrit rien.
            if self.registre().jeu(jeu.id)?.is_some() {
                self.registre().changer_etat(jeu.id, etat, message.as_deref())?;
            }
            emettre(Evenement::Etat { jeu: jeu.id, etat, message });
        }
    }

    /// Met un téléchargement en pause (il reprendra où il en était).
    pub fn mettre_en_pause(&self, id: i64) -> Resultat<()> {
        if let Some((en_cours, arret)) = self.en_cours.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            if *en_cours == id {
                arret.store(true, Ordering::SeqCst);
            }
        }
        let j = self.registre().jeu(id)?.ok_or_else(|| Erreur::Introuvable("Ce jeu n'est pas sur ce PC.".into()))?;
        if matches!(j.etat, Etat::Attente | Etat::EnCours) {
            self.registre().changer_etat(id, Etat::Pause, None)?;
        }
        Ok(())
    }

    /// Remet un téléchargement en pause ou en erreur dans la file (l'appelant relance la file).
    pub fn reprendre(&self, id: i64) -> Resultat<()> {
        let j = self.registre().jeu(id)?.ok_or_else(|| Erreur::Introuvable("Ce jeu n'est pas sur ce PC.".into()))?;
        if matches!(j.etat, Etat::Pause | Etat::Erreur) {
            self.registre().changer_etat(id, Etat::Attente, None)?;
        }
        Ok(())
    }

    /// Annule le téléchargement d'un jeu PAS ENCORE téléchargé : ses fichiers (et eux seuls), ses médias, sa place
    /// dans le registre. Un jeu complet ne s'enlève pas ici : ses sauvegardes de parties passent avant (lot 3).
    pub async fn annuler(&self, id: i64) -> Resultat<()> {
        let j = self.registre().jeu(id)?.ok_or_else(|| Erreur::Introuvable("Ce jeu n'est pas sur ce PC.".into()))?;
        if j.etat == Etat::Telecharge {
            return Err(Erreur::Refus(
                "Ce jeu est complet : on ne l'enlève pas d'ici (ses sauvegardes de parties d'abord, bientôt).".into(),
            ));
        }
        self.mettre_en_pause(id)?;
        // Attendre que le téléchargement ait lâché ses fichiers (au plus 10 s).
        for _ in 0..100 {
            if self.en_cours.lock().unwrap_or_else(|e| e.into_inner()).as_ref().map(|(i, _)| *i) != Some(id) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        let dossier = Path::new(&j.dossier);
        for f in &j.fichiers {
            let nom = nom_de_fichier_sur(&f.nom)?;
            for chemin in [dossier.join(nom), dossier.join(format!("{nom}.part"))] {
                if chemin.is_file() {
                    std::fs::remove_file(&chemin)?;
                }
            }
        }
        // Le dossier du jeu n'est retiré que s'il est vide (rien d'autre n'y est effacé).
        let _ = std::fs::remove_dir(dossier);
        let medias = self.dossier_medias(id);
        if medias.starts_with(self.dossier.join("medias")) && medias.is_dir() {
            std::fs::remove_dir_all(&medias)?;
        }
        self.registre().retirer(id)
    }

    /// La fiche gardée sur le PC d'un jeu de la ludothèque (hors ligne), si elle y est.
    pub fn fiche_locale(&self, id: i64) -> Option<Value> {
        let t = std::fs::read(self.dossier_medias(id).join("fiche.json")).ok()?;
        serde_json::from_slice(&t).ok()
    }

    /// La jaquette gardée sur le PC : la miniature pour une petite largeur, sinon la grande.
    pub fn jaquette_locale(&self, id: i64, largeur: Option<u32>) -> Option<Vec<u8>> {
        let d = self.dossier_medias(id);
        let mini = d.join(format!("jaquette-{LARGEUR_MINIATURE}.img"));
        let grande = d.join("jaquette.img");
        let ordre: [&Path; 2] = if largeur.is_some_and(|l| l <= LARGEUR_MINIATURE) { [&mini, &grande] } else { [&grande, &mini] };
        ordre.iter().find_map(|p| std::fs::read(p).ok())
    }

    /// Une annexe texte gardée sur le PC.
    pub fn annexe_texte_locale(&self, id: i64, i: u32) -> Option<Value> {
        let t = std::fs::read(self.dossier_medias(id).join("annexes").join(format!("{i}.json"))).ok()?;
        serde_json::from_slice(&t).ok()
    }

    /// Le chemin d'une annexe FICHIER gardée sur le PC (pour l'ouvrir avec le programme de Windows).
    pub fn annexe_fichier_locale(&self, id: i64, i: u32) -> Option<PathBuf> {
        let d = self.dossier_medias(id).join("annexes");
        std::fs::read_dir(&d).ok()?.flatten().map(|e| e.path()).find(|p| {
            p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(&format!("{i} - ")))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coffre::CoffreMemoire;
    use crate::noyau::Connexion;
    use httpmock::prelude::*;
    use serde_json::json;
    use std::sync::Mutex;

    /// Un Firehouse simulé avec Dune : catalogue, fiche (1 version, 2 fichiers, 1 annexe texte, 1 annexe fichier),
    /// jaquettes et fichiers.
    fn serveur_dune() -> MockServer {
        let s = MockServer::start();
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/plateformes");
            t.status(200).json_body(json!({"ok": true, "plateformes": [{"nom": "MS-DOS", "jeux": 2}]}));
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue");
            t.status(200).json_body(json!({"ok": true, "suivante": null, "jeux": [
                {"id": 110, "titre": "Dune", "plateforme": "MS-DOS", "jaquette": true},
                {"id": 111, "titre": "Dune II", "plateforme": "MS-DOS"}]}));
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/jeu/110");
            t.status(200).json_body(json!({"ok": true, "jeu": {
                "id": 110, "titre": "Dune", "annee": 1992, "plateforme": "MS-DOS",
                "versions": [{"telechargement_id": 200, "qualite": "Jeu — prêt à jouer",
                              "fichiers": [{"nom": "Dune (1992) [MS-DOS].exe", "taille": 6}, {"nom": "LISEZ-MOI.txt", "taille": 3}]}],
                "annexes": [{"i": 0, "cle": "af:manuel:Manuel", "type": "manuel", "titre": "Manuel de Dune", "taille": 4, "texte": false},
                            {"i": 1, "cle": "af:lancement:DOSBox", "type": "lancement", "titre": "Lancement", "taille": 9, "texte": true}]}}));
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/media/110/jaquette").query_param("largeur", "400");
            t.status(200).body([0xFFu8, 0xD8, 0xFF, 4]);
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/media/110/jaquette");
            t.status(200).body([0xFFu8, 0xD8, 0xFF, 1, 2, 3]);
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/annexe/110/0").query_param("cle", "af:manuel:Manuel");
            t.status(200).header("content-type", "application/pdf").body(b"%PDF");
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/annexe/110/1");
            t.status(200).json_body(json!({"ok": true, "titre": "Lancement", "texte": "Lance DUNE.BAT"}));
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/fichier/110/200/0");
            t.status(200).body(b"DUNE!!");
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/fichier/110/200/1");
            t.status(200).body(b"lis");
        });
        s
    }

    async fn noyau_ouvert(s: &MockServer) -> (tempfile::TempDir, Noyau, Emplacements) {
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: s.base_url(), simule: false }).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();
        let jeux = d.path().join("Jeux");
        std::fs::create_dir_all(&jeux).unwrap();
        let e = Emplacements { defaut: vec![jeux.to_string_lossy().into()], ..Default::default() };
        (d, n, e)
    }

    #[tokio::test]
    async fn ajouter_un_jeu_garde_ses_medias_puis_le_telecharge() {
        let s = serveur_dune();
        let (d, n, e) = noyau_ouvert(&s).await;
        let empl = e.defaut[0].clone();

        // Avant : la ludothèque locale est vide, le catalogue a 2 jeux.
        let filtre = |ludotheque| crate::ludotheque::Filtre { ludotheque, ..Default::default() };
        assert_eq!(n.lister(&filtre(true)).await.unwrap().total, 0);
        assert_eq!(n.lister(&filtre(false)).await.unwrap().total, 2);

        let j = n.ajouter(110, 200, &empl, &e).await.unwrap();
        assert_eq!(j.total, 9);
        assert!(j.dossier.ends_with("MS-DOS\\Dune (1992)") || j.dossier.ends_with("MS-DOS/Dune (1992)"));
        // Le jeu apparaît tout de suite dans la ludothèque (en attente de téléchargement).
        assert_eq!(n.lister(&filtre(true)).await.unwrap().jeux[0].id, 110);

        // Médias gardés : fiche, jaquettes, documents.
        let m = n.dossier_medias(110);
        assert_eq!(n.fiche_locale(110).unwrap()["titre"], "Dune");
        assert_eq!(n.jaquette_locale(110, Some(200)).unwrap(), vec![0xFF, 0xD8, 0xFF, 4]);
        assert_eq!(n.jaquette_locale(110, Some(800)).unwrap().len(), 6);
        assert_eq!(n.annexe_texte_locale(110, 1).unwrap()["texte"], "Lance DUNE.BAT");
        assert!(m.join("annexes").join("0 - Manuel de Dune.pdf").is_file());
        assert!(n.annexe_fichier_locale(110, 0).is_some());

        // La file télécharge.
        let evenements = Mutex::new(Vec::new());
        let session = n.session().await.unwrap();
        n.executer_file(session, &|ev| evenements.lock().unwrap().push(ev)).await.unwrap();
        let dossier = Path::new(&j.dossier);
        assert_eq!(std::fs::read(dossier.join("Dune (1992) [MS-DOS].exe")).unwrap(), b"DUNE!!");
        assert_eq!(std::fs::read(dossier.join("LISEZ-MOI.txt")).unwrap(), b"lis");
        let vu = &n.jeux_du_pc().await.unwrap()[0];
        assert_eq!((vu.jeu.etat, vu.recus), (Etat::Telecharge, 9));
        assert!(evenements
            .lock()
            .unwrap()
            .iter()
            .any(|ev| matches!(ev, Evenement::Etat { etat: Etat::Telecharge, .. })));
        drop(d);
    }

    #[tokio::test]
    async fn sans_internet_la_ludotheque_reste_utilisable() {
        let s = serveur_dune();
        let (_d, n, e) = noyau_ouvert(&s).await;
        n.ajouter(110, 200, &e.defaut[0], &e).await.unwrap();
        n.executer_file(n.session().await.unwrap(), &|_| {}).await.unwrap();

        // Firehouse devient injoignable (même profil, sans redemander de PIN).
        n.reconnecter(&Connexion { adresse: "http://127.0.0.1:9".into(), simule: false }).await.unwrap();
        let f = crate::ludotheque::Filtre { ludotheque: true, ..Default::default() };
        assert_eq!(n.lister(&f).await.unwrap().jeux[0].titre, "Dune");
        assert_eq!(n.plateformes(true).await.unwrap()[0].nom, "MS-DOS");
        let fiche = n.fiche(110).await.unwrap();
        assert!(fiche.locale);
        assert_eq!(fiche.fiche["versions"][0]["telechargement_id"], 200);
        assert!(n.jaquette(110, Some(200)).await.unwrap().is_some());
        assert_eq!(n.annexe_texte(110, 1, "af:lancement:DOSBox").await.unwrap()["texte"], "Lance DUNE.BAT");
        // Ce qui demande Firehouse (un jeu hors de la ludothèque) le dit clairement.
        assert!(matches!(n.fiche(111).await, Err(Erreur::Reseau(_))));
    }

    #[tokio::test]
    async fn un_emplacement_non_regle_est_refuse() {
        let s = serveur_dune();
        let (d, n, e) = noyau_ouvert(&s).await;
        let ailleurs = d.path().join("Ailleurs");
        std::fs::create_dir_all(&ailleurs).unwrap();
        let r = n.ajouter(110, 200, &ailleurs.to_string_lossy(), &e).await;
        assert!(matches!(r, Err(Erreur::Reglage(_))));
        assert!(n.registre().ids().unwrap().is_empty());
    }

    #[tokio::test]
    async fn une_version_inconnue_ou_un_doublon_sont_refuses() {
        let s = serveur_dune();
        let (_d, n, e) = noyau_ouvert(&s).await;
        assert!(matches!(n.ajouter(110, 999, &e.defaut[0], &e).await, Err(Erreur::Introuvable(_))));
        n.ajouter(110, 200, &e.defaut[0], &e).await.unwrap();
        assert!(matches!(n.ajouter(110, 200, &e.defaut[0], &e).await, Err(Erreur::Refus(_))));
    }

    #[tokio::test]
    async fn annuler_efface_seulement_les_fichiers_du_jeu() {
        let s = serveur_dune();
        let (_d, n, e) = noyau_ouvert(&s).await;
        let j = n.ajouter(110, 200, &e.defaut[0], &e).await.unwrap();
        let dossier = PathBuf::from(&j.dossier);
        std::fs::create_dir_all(&dossier).unwrap();
        std::fs::write(dossier.join("Dune (1992) [MS-DOS].exe.part"), b"DU").unwrap();
        // Un fichier qui n'est pas à Frogtend, dans le même dossier : il ne doit pas être touché.
        std::fs::write(dossier.join("SAUVEGARDE.SAV"), b"partie").unwrap();

        n.annuler(110).await.unwrap();
        assert!(!dossier.join("Dune (1992) [MS-DOS].exe.part").exists());
        assert!(dossier.join("SAUVEGARDE.SAV").is_file(), "rien d'autre n'est effacé");
        assert!(!n.dossier_medias(110).exists());
        assert!(n.registre().jeu(110).unwrap().is_none());
    }

    #[tokio::test]
    async fn un_jeu_complet_ne_s_annule_pas() {
        let s = serveur_dune();
        let (_d, n, e) = noyau_ouvert(&s).await;
        n.ajouter(110, 200, &e.defaut[0], &e).await.unwrap();
        n.executer_file(n.session().await.unwrap(), &|_| {}).await.unwrap();
        assert!(matches!(n.annuler(110).await, Err(Erreur::Refus(_))));
    }

    #[tokio::test]
    async fn pause_puis_reprise() {
        let s = serveur_dune();
        let (_d, n, e) = noyau_ouvert(&s).await;
        n.ajouter(110, 200, &e.defaut[0], &e).await.unwrap();
        n.mettre_en_pause(110).unwrap();
        // En pause, la file ne le prend pas.
        n.executer_file(n.session().await.unwrap(), &|_| {}).await.unwrap();
        assert_eq!(n.registre().jeu(110).unwrap().unwrap().etat, Etat::Pause);
        n.reprendre(110).unwrap();
        n.executer_file(n.session().await.unwrap(), &|_| {}).await.unwrap();
        assert_eq!(n.registre().jeu(110).unwrap().unwrap().etat, Etat::Telecharge);
    }

    #[tokio::test]
    async fn un_autre_profil_ne_voit_pas_un_jeu_du_pc_qui_ne_lui_est_pas_visible() {
        let s = serveur_dune();
        let (_d, n, e) = noyau_ouvert(&s).await;
        n.ajouter(110, 200, &e.defaut[0], &e).await.unwrap();
        // Léa n'a jamais reçu le jeu 110 dans son catalogue (son cache est vide) : il ne lui est pas visible.
        let lea = n.creer_profil("Léa", None, None).unwrap().id;
        n.ouvrir(&lea, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        assert!(n.jeux_du_pc().await.unwrap().is_empty());
        let f = crate::ludotheque::Filtre { ludotheque: true, ..Default::default() };
        assert_eq!(n.lister(&f).await.unwrap().total, 0);
    }

    #[tokio::test]
    async fn en_mode_simule_on_ne_met_pas_de_jeu_sur_le_pc() {
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(d.path(), Box::new(CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, None).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        let e = Emplacements { defaut: vec![d.path().to_string_lossy().into()], ..Default::default() };
        assert!(matches!(n.ajouter(110, 200, &e.defaut[0], &e).await, Err(Erreur::Refus(_))));
    }

    /// Essai RÉEL (jamais lancé par la suite) : met un jeu dans une ludothèque de TEST, dans un dossier temporaire
    /// effacé à la fin, avec le jeton d'un profil lu dans le coffre. Télécharge vraiment : seulement avec l'accord
    /// de Seb. `FROGTEND_PROFIL_ESSAI=<id> FROGTEND_JEU_ESSAI=110 cargo test essai_ajout_reel -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_ajout_reel() {
        use crate::coffre::Coffre as _;
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let jeu: i64 = std::env::var("FROGTEND_JEU_ESSAI").expect("FROGTEND_JEU_ESSAI").parse().unwrap();
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton pour ce profil");

        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Essai", None, Some(&jeton)).unwrap().id;
        let c = Connexion { adresse: "https://jeux.hikari-no-sekai.fr".into(), simule: false };
        n.ouvrir(&id, None, &c).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();
        let jeux = d.path().join("Jeux");
        std::fs::create_dir_all(&jeux).unwrap();
        let e = Emplacements { defaut: vec![jeux.to_string_lossy().into()], ..Default::default() };

        let fiche = n.fiche(jeu).await.unwrap().fiche;
        let version = fiche["versions"][0]["telechargement_id"].as_i64().expect("aucune version");
        let debut = std::time::Instant::now();
        let j = n.ajouter(jeu, version, &e.defaut[0], &e).await.unwrap();
        println!("AJOUT « {} » : {} octets, {} fichier(s), dossier {}", j.titre, j.total, j.fichiers.len(), j.dossier);
        println!("MÉDIAS : {:?}", std::fs::read_dir(n.dossier_medias(jeu)).unwrap().map(|e| e.unwrap().file_name()).collect::<Vec<_>>());
        println!("DOCUMENTS : {:?}", std::fs::read_dir(n.dossier_medias(jeu).join("annexes")).unwrap().map(|e| e.unwrap().file_name()).collect::<Vec<_>>());

        let dernier = Mutex::new(std::time::Instant::now());
        n.executer_file(n.session().await.unwrap(), &|ev| {
            if let Evenement::Progres(p) = &ev {
                let mut t = dernier.lock().unwrap();
                if t.elapsed().as_secs() >= 5 {
                    println!("  {} / {} octets, {} Ko/s", p.recus, p.total, p.debit / 1024);
                    *t = std::time::Instant::now();
                }
            } else {
                println!("  {ev:?}");
            }
        })
        .await
        .unwrap();
        let vu = &n.jeux_du_pc().await.unwrap()[0];
        println!("FIN : état {:?}, {} octets sur le disque sur {}, en {} s", vu.jeu.etat, vu.recus, vu.jeu.total, debut.elapsed().as_secs());
        for f in &vu.jeu.fichiers {
            let sur_disque = std::fs::metadata(Path::new(&vu.jeu.dossier).join(&f.nom)).unwrap().len();
            println!("  « {} » : {} octets (attendu {})", f.nom, sur_disque, f.taille);
            assert_eq!(sur_disque, f.taille);
        }
        assert_eq!(vu.jeu.etat, Etat::Telecharge);
        // Hors ligne ensuite :
        n.reconnecter(&Connexion { adresse: "http://127.0.0.1:9".into(), simule: false }).await.unwrap();
        assert!(n.fiche(jeu).await.unwrap().locale);
        assert!(n.jaquette(jeu, Some(200)).await.unwrap().is_some());
        println!("HORS LIGNE : fiche et jaquette lues sur le disque.");
        drop(d); // tout est effacé
    }

    /// Essai RÉEL (lecture seule) : la nature d'un document de Firehouse, d'après ses premiers octets.
    /// `FROGTEND_PROFIL_ESSAI=<id> cargo test essai_document_reel -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_document_reel() {
        use crate::coffre::Coffre as _;
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton");
        let c = Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap();
        let route = format!("/annexe/110/0?cle={}", encoder("abandonware_france:manuel:Manuel de Dune"));
        if let Reponse::Corps { octets, type_contenu, .. } = c.obtenir(&route, None).await.unwrap() {
            println!("MANUEL : {} octets, type {:?}, début {:02X?} → .{}", octets.len(), type_contenu, &octets[..8], extension(&octets, type_contenu.as_deref()));
        }
    }

    /// Reconnaissance RÉELLE en lecture seule, pour le lot 3 : notice de lancement, début du fichier d'une version
    /// (type d'installeur), émulateurs recommandés. Ne télécharge que 1 Mo du fichier.
    /// `FROGTEND_PROFIL_ESSAI=<id> cargo test essai_reconnaissance -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_reconnaissance_lot3() {
        use crate::coffre::Coffre as _;
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton");
        let base = "https://jeux.hikari-no-sekai.fr";
        let c = Client::nouveau(base, &jeton).unwrap();
        let route = format!("/annexe/110/1?cle={}", encoder("abandonware_france:lancement:Lancement sous DOSBox"));
        let v: Value = c.obtenir_json(&route).await.unwrap();
        println!("NOTICE DE LANCEMENT :\n{}\n", v["texte"].as_str().unwrap_or("?"));

        let http = reqwest::Client::new();
        let rep = http
            .get(format!("{base}/api/jeux/v1/fichier/110/200/0"))
            .bearer_auth(&jeton)
            .header("Range", "bytes=0-1048575")
            .send()
            .await
            .unwrap();
        println!("FICHIER : statut {}, {:?}", rep.status(), rep.headers().get("content-range"));
        let o = rep.bytes().await.unwrap();
        println!("  {} octets lus, début {:02X?}", o.len(), &o[..4]);
        for marque in ["Inno Setup", "Nullsoft", "7z\u{BC}\u{AF}", "WinRAR", "Setup Factory", "InstallShield", "7-Zip", "DOSBox", "rDlPtS", "zlb"] {
            if let Some(p) = o.windows(marque.len()).position(|w| w == marque.as_bytes()) {
                println!("  « {marque} » trouvé à l'octet {p}");
            }
        }
        if let Some(p) = o.windows(6).position(|w| w == [0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C]) {
            println!("  signature 7z trouvée à l'octet {p}");
        }
        // Les chaînes lisibles les plus longues (version, nom du produit…).
        let mut lisibles: Vec<String> = o
            .split(|b| !(0x20..0x7F).contains(b))
            .filter(|s| s.len() >= 18)
            .map(|s| String::from_utf8_lossy(s).to_string())
            .collect();
        lisibles.dedup();
        println!("  chaînes : {:?}", lisibles.iter().take(40).collect::<Vec<_>>());

        for p in ["MS-DOS", "Super Nintendo", "Nintendo 64", "Sony Playstation", "Windows"] {
            match c.obtenir_json::<Value>(&format!("/emulateurs?plateforme={}", encoder(p))).await {
                Ok(v) => println!("\nÉMULATEURS {p} : {}", serde_json::to_string(&v["emulateurs"]).unwrap_or_default().chars().take(900).collect::<String>()),
                Err(e) => println!("\nÉMULATEURS {p} : {e:?}"),
            }
        }
    }

    #[test]
    fn l_extension_d_un_document_vient_de_son_type() {
        assert_eq!(extension(b"", Some("application/pdf")), "pdf");
        assert_eq!(extension(b"", Some("text/plain; charset=utf-8")), "txt");
        assert_eq!(extension(b"", None), "bin");
        assert_eq!(extension(b"", Some("application/x-7z-compressed")), "7z");
        assert_eq!(extension(&[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C], None), "7z");
        // Le contenu l'emporte sur un type trop vague.
        assert_eq!(extension(b"%PDF-1.4 ...", Some("application/octet-stream")), "pdf");
        assert_eq!(extension(b"PK...", None), "zip");
        assert_eq!(extension(&[0xFF, 0xD8, 0xFF, 0], None), "jpg");
    }
}
