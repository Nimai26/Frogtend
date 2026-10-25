//! Télécharger les fichiers d'un jeu : reprise `Range`, fichiers `.part`, taille vérifiée, pause.
//!
//! Règles : on compte les octets réellement écrits (pas le « ok » d'une fonction) ; un fichier n'est renommé de
//! `.part` vers son nom final qu'une fois sa taille vérifiée ; les noms reçus ne sont JAMAIS modifiés ; un 409
//! (fichier abîmé sur le serveur) est dit, sans nouvel essai en boucle.

use crate::erreurs::{Erreur, Resultat};
use crate::firehouse::Client;
use crate::jeux_pc::JeuPc;
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

/// Le refus d'un fichier arrivé abîmé (empreinte différente de celle de Firehouse) : il repartira de zéro.
fn abime(nom: &str) -> Erreur {
    Erreur::Serveur(format!(
        "« {nom} » est arrivé abîmé (son empreinte n'est pas celle annoncée par Firehouse) : il a été écarté. Une reprise le retélécharge depuis le début ; s'il arrive encore abîmé, c'est le fichier chez Firehouse : inutile de relancer, préviens l'administrateur de Firehouse."
    ))
}

/// Relit les `n` premiers octets d'un fichier dans une empreinte, par blocs (un `.part` de 50 Go prend du temps) :
/// la pause est possible à chaque bloc (`None`), et la progression est dite.
fn relire_dans(
    h: &mut sha1::Sha1,
    p: &Path,
    n: u64,
    arret: &AtomicBool,
    signaler: &mut dyn FnMut(u64),
) -> Resultat<Option<()>> {
    use sha1::Digest;
    use std::io::Read;
    let mut f = std::fs::File::open(p)?.take(n);
    let mut tampon = vec![0u8; 8 * 1024 * 1024];
    let mut lus = 0u64;
    loop {
        if arret.load(Ordering::Relaxed) {
            return Ok(None);
        }
        let k = f.read(&mut tampon)?;
        if k == 0 {
            return Ok(Some(()));
        }
        h.update(&tampon[..k]);
        lus += k as u64;
        signaler(lus);
    }
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
    sha1: Option<&str>,
    arret: &AtomicBool,
    deja: u64,
    total: u64,
    jeu: i64,
    progres: &(impl Fn(Progres) + ?Sized),
) -> Resultat<Issue> {
    use sha1::{Digest, Sha1};
    // Un nom, ou « sous-dossier/nom » (CHD MAME) : chaque partie contrôlée, jamais hors du dossier du jeu.
    let rel = crate::jeux_pc::chemin_de_fichier_sur(nom)?;
    let final_ = dossier.join(&rel);
    let mut p = final_.clone().into_os_string();
    p.push(".part");
    let part = std::path::PathBuf::from(p);
    std::fs::create_dir_all(final_.parent().unwrap_or(dossier))?;

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
    // La progression pendant la relecture d'un début déjà reçu (vérification de l'empreinte).
    let mut verifie = |lus: u64| progres(Progres { jeu, recus: deja + lus, total, debit: 0, fichier: format!("vérification de « {nom} »") });
    if debut == taille && taille > 0 {
        if let Some(attendue) = sha1 {
            let mut h = Sha1::new();
            if relire_dans(&mut h, &part, taille, arret, &mut verifie)?.is_none() {
                return Ok(Issue::Pause);
            }
            let recue: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
            if !recue.eq_ignore_ascii_case(attendue) {
                std::fs::remove_file(&part)?;
                return Err(abime(nom));
            }
        }
        std::fs::rename(&part, &final_)?;
        return Ok(Issue::Termine);
    }

    let mut flux = match client.flux(route, debut).await {
        Ok(f) => f,
        // 416 SEULEMENT : le serveur n'a rien après `debut` ; si notre .part n'a pas la bonne taille, on repart de zéro.
        // Toute autre erreur (404, 409, 410, 503…) GARDE le .part déjà reçu (un jeu de 100 Go à moitié reçu).
        Err(Erreur::Plage(_)) if debut > 0 => {
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
    // L'empreinte (contrat 1.24) se calcule PENDANT la réception : le début déjà reçu d'abord (reprise), puis chaque
    // morceau — jamais une deuxième lecture d'un fichier de 50 Go.
    let mut empreinte = match sha1 {
        Some(_) => {
            let mut h = Sha1::new();
            if debut > 0 && relire_dans(&mut h, &part, debut, arret, &mut verifie)?.is_none() {
                return Ok(Issue::Pause);
            }
            Some(h)
        }
        None => None,
    };
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
        if let Some(h) = empreinte.as_mut() {
            h.update(&morceau);
        }
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
    if let (Some(h), Some(attendue)) = (empreinte.take(), sha1) {
        let recue: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
        if !recue.eq_ignore_ascii_case(attendue) {
            std::fs::remove_file(&part)?;
            return Err(abime(nom));
        }
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
        // Chaque fichier sous SA version (les cartes d'un jeu e-Reader), sinon celle de la ligne.
        let route = format!("/fichier/{}/{}/{}", jeu.id, f.version.unwrap_or(jeu.version), f.n);
        match telecharger_fichier(client, &route, dossier, &f.nom, f.taille, f.sha1.as_deref(), arret, deja, jeu.total, jeu.id, progres)
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
                FichierJeu { n: 0, nom: "Super Mario World (USA).sfc".into(), taille: 20, ..Default::default() },
                FichierJeu { n: 1, nom: "LISEZ-MOI.txt".into(), taille: 3, ..Default::default() },
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

    /// Le sha1 d'un contenu, en hexadécimal.
    fn sha1_de(o: &[u8]) -> String {
        use sha1::{Digest, Sha1};
        Sha1::digest(o).iter().map(|b| format!("{b:02x}")).collect()
    }

    #[tokio::test]
    async fn l_empreinte_de_firehouse_est_verifiee_meme_apres_une_reprise() {
        // Repris en 206 depuis l'octet 8, empreinte annoncée = celle du fichier entier : accepté.
        let s = MockServer::start();
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/fichier/110/200/0").header("range", "bytes=8-");
            t.status(206).body(&CONTENU[8..]);
        });
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("jeu.iso.part"), &CONTENU[..8]).unwrap();
        let c = Client::nouveau(&s.base_url(), "j").unwrap();
        let bon = FichierJeu { n: 0, nom: "jeu.iso".into(), taille: 20, sha1: Some(sha1_de(CONTENU)), version: None };
        telecharger_jeu(&c, &jeu(d.path(), vec![bon.clone()]), &AtomicBool::new(false), &|_| {}).await.unwrap();
        assert_eq!(std::fs::read(d.path().join("jeu.iso")).unwrap(), CONTENU);
        // Une empreinte différente : le fichier est écarté (jamais gardé abîmé), et c'est dit.
        let s2 = MockServer::start();
        s2.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/fichier/110/200/0");
            t.status(200).body(CONTENU);
        });
        let d2 = tempfile::tempdir().unwrap();
        let faux = FichierJeu { sha1: Some("0".repeat(40)), ..bon };
        let e = telecharger_jeu(&Client::nouveau(&s2.base_url(), "j").unwrap(), &jeu(d2.path(), vec![faux]), &AtomicBool::new(false), &|_| {})
            .await
            .unwrap_err();
        assert!(format!("{e:?}").contains("abîmé"), "{e:?}");
        assert!(!d2.path().join("jeu.iso").exists() && !d2.path().join("jeu.iso.part").exists());
    }

    #[tokio::test]
    async fn un_part_deja_complet_mais_abime_est_ecarte_sans_rien_demander() {
        let s = MockServer::start();
        let rien = s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/fichier/110/200/0");
            t.status(200).body(CONTENU);
        });
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("jeu.iso.part"), vec![b'x'; 20]).unwrap();
        let f = FichierJeu { n: 0, nom: "jeu.iso".into(), taille: 20, sha1: Some(sha1_de(CONTENU)), version: None };
        let e = telecharger_jeu(&Client::nouveau(&s.base_url(), "j").unwrap(), &jeu(d.path(), vec![f]), &AtomicBool::new(false), &|_| {})
            .await
            .unwrap_err();
        assert!(format!("{e:?}").contains("abîmé"));
        assert!(!d.path().join("jeu.iso").exists() && !d.path().join("jeu.iso.part").exists());
        assert_eq!(rien.hits(), 0, "un .part complet n'est pas redemandé : il est vérifié");
    }

    #[tokio::test]
    async fn un_fichier_de_sa_propre_version_est_demande_sous_elle() {
        // Les cartes d'un jeu e-Reader : chaque fichier sous SA version (-41144), pas celle de la ligne (200).
        let s = MockServer::start();
        let carte = s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/fichier/110/-41144/0");
            t.status(200).body(b"carte");
        });
        let d = tempfile::tempdir().unwrap();
        let f = FichierJeu { n: 0, nom: "carte.7z".into(), taille: 5, sha1: None, version: Some(-41144) };
        let mut j = jeu(d.path(), vec![f]);
        j.total = 5;
        telecharger_jeu(&Client::nouveau(&s.base_url(), "j").unwrap(), &j, &AtomicBool::new(false), &|_| {}).await.unwrap();
        carte.assert();
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
        let j = jeu(d.path(), vec![FichierJeu { n: 0, nom: "jeu.iso".into(), taille: 20, ..Default::default() }]);
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
        let j = jeu(d.path(), vec![FichierJeu { n: 0, nom: "jeu.iso".into(), taille: 20, ..Default::default() }]);
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
        let j = jeu(d.path(), vec![FichierJeu { n: 0, nom: "jeu.iso".into(), taille: 20, ..Default::default() }]);
        let e = telecharger_jeu(&c, &j, &AtomicBool::new(false), &|_| {}).await.unwrap_err();
        assert!(matches!(&e, Erreur::Reseau(m) if m.contains("12 octets sur 20")), "{e:?}");
        assert!(!d.path().join("jeu.iso").exists(), "jamais de nom final pour un fichier incomplet");
        assert_eq!(std::fs::metadata(d.path().join("jeu.iso.part")).unwrap().len(), 12);
    }

    #[tokio::test]
    async fn un_refus_en_pleine_reprise_garde_le_part_deja_recu() {
        // Relecture de l'expert contrat (18/10) : un 410 (fiche réunie, contrat 1.14), un 404 ou un 503 reçu en
        // reprise effaçait le `.part` (un jeu de 100 Go à moitié reçu). Seul un 416 l'autorise.
        for (code, corps) in [
            (410, serde_json::json!({"ok": false, "fusionne_dans": 324, "le": "2026-10-05T14:02:37", "detail": "ce jeu a été réuni à la fiche 324"})),
            (404, serde_json::json!({"ok": false})),
            (503, serde_json::json!({"ok": false})),
        ] {
            let s = MockServer::start();
            let m = s.mock(|w, t| {
                w.method(GET).path("/api/jeux/v1/fichier/110/200/0");
                t.status(code).json_body(corps.clone());
            });
            let d = tempfile::tempdir().unwrap();
            std::fs::write(d.path().join("jeu.iso.part"), &CONTENU[..8]).unwrap();
            let c = Client::nouveau(&s.base_url(), "j").unwrap();
            let j = jeu(d.path(), vec![FichierJeu { n: 0, nom: "jeu.iso".into(), taille: 20, ..Default::default() }]);
            let e = telecharger_jeu(&c, &j, &AtomicBool::new(false), &|_| {}).await.unwrap_err();
            assert_eq!(std::fs::read(d.path().join("jeu.iso.part")).unwrap(), &CONTENU[..8], "{code} : le .part est gardé");
            m.assert_hits(1);
            if code == 410 {
                assert!(e.to_string().contains("324"), "le texte de Firehouse est dit : {e}");
            }
        }
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
        let j = jeu(d.path(), vec![FichierJeu { n: 0, nom: "jeu.iso".into(), taille: 20, ..Default::default() }]);
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
        let j = jeu(d.path(), vec![FichierJeu { n: 0, nom: "jeu.iso".into(), taille: 20, ..Default::default() }]);
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
        let j = jeu(d.path(), vec![FichierJeu { n: 0, nom: "jeu.iso".into(), taille: 20, ..Default::default() }]);
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
        let mut j = jeu(d.path(), vec![FichierJeu { n: 0, nom: "rom.bin".into(), taille: 3, ..Default::default() }]);
        j.version = 0;
        telecharger_jeu(&c, &j, &AtomicBool::new(false), &|_| {}).await.unwrap();
        m.assert();
    }
}
