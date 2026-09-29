//! Télécharger les fichiers d'un jeu : reprise `Range`, fichiers `.part`, taille vérifiée, pause.
//!
//! Règles : on compte les octets réellement écrits (pas le « ok » d'une fonction) ; un fichier n'est renommé de
//! `.part` vers son nom final qu'une fois sa taille vérifiée ; les noms reçus ne sont JAMAIS modifiés ; un 409
//! (fichier abîmé sur le serveur) est dit, sans nouvel essai en boucle.

use crate::erreurs::{Erreur, Resultat};
use crate::firehouse::Client;
use crate::jeux_pc::{nom_de_fichier_sur, JeuPc};
use serde::Serialize;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Fréquence des nouvelles de progression envoyées à l'interface.
const INTERVALLE_PROGRES: Duration = Duration::from_millis(400);

/// Ce que l'interface reçoit pendant un téléchargement.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Progres {
    pub jeu: i64,
    /// Octets du jeu déjà sur le disque.
    pub recus: u64,
    pub total: u64,
    /// Débit mesuré, en octets par seconde.
    pub debit: u64,
    /// Le fichier en cours (son nom, tel quel).
    pub fichier: String,
}

/// Comment s'est terminé un téléchargement.
#[derive(Debug, PartialEq)]
pub enum Issue {
    Termine,
    /// Arrêté à la demande : reprendra où il en était.
    Pause,
}

/// Télécharge UN fichier vers `dossier/nom` en passant par `nom.part`.
/// `deja` : octets du jeu déjà comptés avant ce fichier (pour la progression d'ensemble).
#[allow(clippy::too_many_arguments)]
pub async fn telecharger_fichier(
    client: &Client,
    route: &str,
    dossier: &Path,
    nom: &str,
    taille: u64,
    arret: &AtomicBool,
    deja: u64,
    total: u64,
    jeu: i64,
    progres: &(impl Fn(Progres) + ?Sized),
) -> Resultat<Issue> {
    let nom = nom_de_fichier_sur(nom)?;
    std::fs::create_dir_all(dossier)?;
    let final_ = dossier.join(nom);
    let part = dossier.join(format!("{nom}.part"));

    // Déjà là, à la bonne taille : rien à faire.
    if std::fs::metadata(&final_).is_ok_and(|m| m.len() == taille) {
        return Ok(Issue::Termine);
    }

    let mut debut = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
    if debut > taille {
        // Un .part plus gros que prévu : il ne vaut rien.
        std::fs::remove_file(&part)?;
        debut = 0;
    }
    if debut == taille && taille > 0 {
        std::fs::rename(&part, &final_)?;
        return Ok(Issue::Termine);
    }

    let mut flux = match client.flux(route, debut).await {
        Ok(f) => f,
        // 416 : le serveur n'a rien après `debut` ; si notre .part n'a pas la bonne taille, on repart de zéro.
        Err(Erreur::Refus(_)) if debut > 0 => {
            std::fs::remove_file(&part)?;
            debut = 0;
            client.flux(route, 0).await?
        }
        Err(e) => return Err(e),
    };
    if !flux.reprise && debut > 0 {
        // Le serveur renvoie tout le fichier (il n'a pas repris) : on réécrit depuis le début.
        debut = 0;
    }
    let mut sortie = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(debut > 0)
        .truncate(debut == 0)
        .open(&part)?;

    let mut ecrits = debut;
    let mut dernier = Instant::now();
    let mut ecrits_depuis = 0u64;
    loop {
        if arret.load(Ordering::Relaxed) {
            sortie.flush()?;
            return Ok(Issue::Pause);
        }
        let Some(morceau) = Client::morceau(&mut flux).await? else { break };
        sortie.write_all(&morceau)?;
        ecrits += morceau.len() as u64;
        ecrits_depuis += morceau.len() as u64;
        if ecrits > taille {
            drop(sortie);
            std::fs::remove_file(&part)?;
            return Err(Erreur::Serveur(format!(
                "Firehouse envoie plus de données que prévu pour « {nom} » : le fichier est écarté."
            )));
        }
        let ecoule = dernier.elapsed();
        if ecoule >= INTERVALLE_PROGRES {
            progres(Progres {
                jeu,
                recus: deja + ecrits,
                total,
                debit: (ecrits_depuis as f64 / ecoule.as_secs_f64()) as u64,
                fichier: nom.into(),
            });
            dernier = Instant::now();
            ecrits_depuis = 0;
        }
    }
    sortie.flush()?;
    drop(sortie);

    // Vérifier le résultat, pas le retour : la taille sur le disque.
    let sur_disque = std::fs::metadata(&part)?.len();
    if sur_disque != taille {
        return Err(Erreur::Reseau(format!(
            "« {nom} » est incomplet ({sur_disque} octets sur {taille}). Il reprendra où il en était."
        )));
    }
    std::fs::rename(&part, &final_)?;
    progres(Progres { jeu, recus: deja + taille, total, debit: 0, fichier: nom.into() });
    Ok(Issue::Termine)
}

/// Télécharge tous les fichiers d'un jeu du PC, l'un après l'autre.
pub async fn telecharger_jeu(
    client: &Client,
    jeu: &JeuPc,
    arret: &AtomicBool,
    progres: &(impl Fn(Progres) + ?Sized),
) -> Resultat<Issue> {
    let dossier = Path::new(&jeu.dossier);
    let mut deja = 0u64;
    for f in &jeu.fichiers {
        let route = format!("/fichier/{}/{}/{}", jeu.id, jeu.version, f.n);
        match telecharger_fichier(client, &route, dossier, &f.nom, f.taille, arret, deja, jeu.total, jeu.id, progres)
            .await?
        {
            Issue::Pause => return Ok(Issue::Pause),
            Issue::Termine => deja += f.taille,
        }
    }
    // Contrôle final : tout est là, à la bonne taille.
    if jeu.recus() != jeu.total {
        return Err(Erreur::Disque(format!(
            "Après le téléchargement, {} octets sur {} sont sur le disque.",
            jeu.recus(),
            jeu.total
        )));
    }
    Ok(Issue::Termine)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jeux_pc::{Etat, FichierJeu};
    use httpmock::prelude::*;
    use std::sync::Mutex;

    const CONTENU: &[u8] = b"0123456789ABCDEFGHIJ"; // 20 octets

    fn jeu(dossier: &Path, fichiers: Vec<FichierJeu>) -> JeuPc {
        let total = fichiers.iter().map(|f| f.taille).sum();
        JeuPc {
            id: 110,
            version: 200,
            titre: "Dune".into(),
            plateforme: "MS-DOS".into(),
            dossier: dossier.to_string_lossy().into(),
            etat: Etat::Attente,
            total,
            fichiers,
            message: None,
            ajoute_le: "1".into(),
            ajoute_par: "seb".into(),
            installation: None,
            temps_jeu: 0,
            derniere_partie: None,
        }
    }

    #[tokio::test]
    async fn un_jeu_se_telecharge_sous_ses_noms_d_origine() {
        let s = MockServer::start();
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/fichier/110/200/0");
            t.status(200).body(CONTENU);
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/fichier/110/200/1");
            t.status(200).body(b"xyz");
        });
        let d = tempfile::tempdir().unwrap();
        let j = jeu(
            d.path(),
            vec![
                FichierJeu { n: 0, nom: "Super Mario World (USA).sfc".into(), taille: 20 },
                FichierJeu { n: 1, nom: "LISEZ-MOI.txt".into(), taille: 3 },
            ],
        );
        let c = Client::nouveau(&s.base_url(), "j").unwrap();
        let vus = Mutex::new(Vec::new());
        let issue = telecharger_jeu(&c, &j, &AtomicBool::new(false), &|p: Progres| vus.lock().unwrap().push(p.recus))
            .await
            .unwrap();
        assert_eq!(issue, Issue::Termine);
        assert_eq!(std::fs::read(d.path().join("Super Mario World (USA).sfc")).unwrap(), CONTENU);
        assert_eq!(std::fs::read(d.path().join("LISEZ-MOI.txt")).unwrap(), b"xyz");
        assert!(!d.path().join("Super Mario World (USA).sfc.part").exists());
        assert_eq!(*vus.lock().unwrap().last().unwrap(), 23);
    }

    #[tokio::test]
    async fn un_fichier_commence_reprend_avec_range() {
        let s = MockServer::start();
        let reprise = s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/fichier/110/200/0").header("range", "bytes=8-");
            t.status(206).body(&CONTENU[8..]);
        });
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("jeu.iso.part"), &CONTENU[..8]).unwrap();
        let c = Client::nouveau(&s.base_url(), "j").unwrap();
        let j = jeu(d.path(), vec![FichierJeu { n: 0, nom: "jeu.iso".into(), taille: 20 }]);
        telecharger_jeu(&c, &j, &AtomicBool::new(false), &|_| {}).await.unwrap();
        reprise.assert();
        assert_eq!(std::fs::read(d.path().join("jeu.iso")).unwrap(), CONTENU);
    }

    #[tokio::test]
    async fn un_serveur_qui_ne_reprend_pas_fait_repartir_de_zero_sans_doublon() {
        let s = MockServer::start();
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/fichier/110/200/0");
            t.status(200).body(CONTENU); // ignore Range : tout le fichier
        });
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("jeu.iso.part"), &CONTENU[..8]).unwrap();
        let c = Client::nouveau(&s.base_url(), "j").unwrap();
        let j = jeu(d.path(), vec![FichierJeu { n: 0, nom: "jeu.iso".into(), taille: 20 }]);
        telecharger_jeu(&c, &j, &AtomicBool::new(false), &|_| {}).await.unwrap();
        assert_eq!(std::fs::read(d.path().join("jeu.iso")).unwrap(), CONTENU);
    }

    #[tokio::test]
    async fn un_fichier_trop_court_reste_en_part_et_le_dit() {
        let s = MockServer::start();
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/fichier/110/200/0");
            t.status(200).body(&CONTENU[..12]);
        });
        let d = tempfile::tempdir().unwrap();
        let c = Client::nouveau(&s.base_url(), "j").unwrap();
        let j = jeu(d.path(), vec![FichierJeu { n: 0, nom: "jeu.iso".into(), taille: 20 }]);
        let e = telecharger_jeu(&c, &j, &AtomicBool::new(false), &|_| {}).await.unwrap_err();
        assert!(matches!(&e, Erreur::Reseau(m) if m.contains("12 octets sur 20")), "{e:?}");
        assert!(!d.path().join("jeu.iso").exists(), "jamais de nom final pour un fichier incomplet");
        assert_eq!(std::fs::metadata(d.path().join("jeu.iso.part")).unwrap().len(), 12);
    }

    #[tokio::test]
    async fn un_409_est_dit_sans_nouvel_essai() {
        let s = MockServer::start();
        let m = s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/fichier/110/200/0");
            t.status(409).json_body(serde_json::json!({"detail": "le fichier n'a plus la taille notée"}));
        });
        let d = tempfile::tempdir().unwrap();
        let c = Client::nouveau(&s.base_url(), "j").unwrap();
        let j = jeu(d.path(), vec![FichierJeu { n: 0, nom: "jeu.iso".into(), taille: 20 }]);
        let e = telecharger_jeu(&c, &j, &AtomicBool::new(false), &|_| {}).await.unwrap_err();
        assert!(matches!(e, Erreur::Conflit(_)));
        m.assert_hits(1);
    }

    #[tokio::test]
    async fn la_pause_arrete_et_garde_le_part() {
        let s = MockServer::start();
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/fichier/110/200/0");
            t.status(200).body(CONTENU);
        });
        let d = tempfile::tempdir().unwrap();
        let c = Client::nouveau(&s.base_url(), "j").unwrap();
        let j = jeu(d.path(), vec![FichierJeu { n: 0, nom: "jeu.iso".into(), taille: 20 }]);
        let issue = telecharger_jeu(&c, &j, &AtomicBool::new(true), &|_| {}).await.unwrap();
        assert_eq!(issue, Issue::Pause);
        assert!(!d.path().join("jeu.iso").exists());
    }

    #[tokio::test]
    async fn un_fichier_deja_complet_n_est_pas_redemande() {
        let s = MockServer::start();
        let m = s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/fichier/110/200/0");
            t.status(200).body(CONTENU);
        });
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("jeu.iso"), CONTENU).unwrap();
        let c = Client::nouveau(&s.base_url(), "j").unwrap();
        let j = jeu(d.path(), vec![FichierJeu { n: 0, nom: "jeu.iso".into(), taille: 20 }]);
        telecharger_jeu(&c, &j, &AtomicBool::new(false), &|_| {}).await.unwrap();
        m.assert_hits(0);
    }

    #[tokio::test]
    async fn la_version_zero_du_recensement_est_un_vrai_numero() {
        let s = MockServer::start();
        let m = s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/fichier/110/0/0");
            t.status(200).body(b"abc");
        });
        let d = tempfile::tempdir().unwrap();
        let c = Client::nouveau(&s.base_url(), "j").unwrap();
        let mut j = jeu(d.path(), vec![FichierJeu { n: 0, nom: "rom.bin".into(), taille: 3 }]);
        j.version = 0;
        telecharger_jeu(&c, &j, &AtomicBool::new(false), &|_| {}).await.unwrap();
        m.assert();
    }
}
