//! Les tests des mods Switch (contrat 1.44). Tout se passe dans des dossiers temporaires (jamais les vraies données de
//! Seb) ; Firehouse est simulé (httpmock), jamais appelé pour de vrai.

use super::*;
use crate::contenus_switch::{charger, choisir, choisir_mods, noter_changement_de_mod};
use crate::source::Source;
use crate::switch_nca::tests::BASE;
use httpmock::prelude::*;
use sha2::Digest;

const CLE: &str = "wollnashorn-0123456789ab";
const CLE2: &str = "masagrator-abcdef012345";

fn sha(o: &[u8]) -> String {
    format!("{:x}", sha2::Sha256::digest(o))
}

/// Un ZIP dont chaque entrée est STOCKÉE (la forme du contrat) — ou compressée (`compresse`).
fn zip_octets(fichiers: &[(&str, &[u8])], compresse: bool) -> Vec<u8> {
    let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let methode = if compresse { zip::CompressionMethod::Deflated } else { zip::CompressionMethod::Stored };
    let o = zip::write::SimpleFileOptions::default().compression_method(methode);
    for (nom, c) in fichiers {
        if nom.ends_with('/') {
            z.add_directory(*nom, o).unwrap();
        } else {
            z.start_file(*nom, o).unwrap();
            z.write_all(c).unwrap();
        }
    }
    z.finish().unwrap().into_inner()
}

/// Un mod de la forme du contrat 1.44 (pas un exemple réel : il n'y en a pas encore).
fn mod_json(cle: &str, revision: u64, archive: &[u8]) -> Value {
    json!({
        "cle": cle, "nom": "1080p", "revision": revision, "description": "Résolution 1080p", "auteur": "Wollnashorn",
        "source": "https://github.com/…", "licence": "aucune", "valide_le": "2026-11-18T10:00:00",
        "versions": [{"build_id": "8E9978D50BDD20B4C8395A106C27FFDE", "version_brute": 786432, "version_affichee": "1.6.0"}],
        "versions_annoncees": ["1.6.0"], "compatible": true, "raison": "", "emulateurs_essayes": [], "reglage_requis": "",
        "conflits": [], "touche_sauvegardes": false, "contenu": {"exefs": true, "romfs": false, "cheats": false}, "public": "tous",
        "fichier": {"url": format!("/api/jeux/v1/switch/mods/{cle}/archive"), "taille": archive.len(), "sha256": sha(archive)},
        "inconnu_demain": {"x": 1}
    })
}

fn servi(cle: &str, revision: u64, archive: &[u8]) -> ModServi {
    ModServi::depuis(&mod_json(cle, revision, archive)).unwrap()
}

fn source(m: &MockServer) -> Source {
    Source::Firehouse(crate::firehouse::Client::nouveau(&m.base_url(), "jeton").unwrap())
}

fn rien(_: u64, _: Option<u64>) {}

fn vivant() -> bool {
    false
}

/// L'écriture du cache d'une session vivante (comme `Session::garder_fichier`).
fn garder_libre(f: &Path, o: &[u8]) -> Resultat<()> {
    std::fs::create_dir_all(f.parent().unwrap())?;
    std::fs::write(f, o)?;
    Ok(())
}

#[test]
fn la_forme_du_contrat_se_lit_avec_tolerance() {
    let a = zip_octets(&[("exefs/a.ips", b"x")], false);
    let mut liste = mod_json(CLE, 3, &a);
    liste["revision"] = json!("3");
    let mut toutes = mod_json(CLE2, 1, &a);
    toutes["versions"] = json!("toutes");
    toutes["compatible"] = Value::Null;
    toutes["touche_sauvegardes"] = Value::Null;
    toutes["description"] = Value::Null;
    let mut bizarre = mod_json("autre-aaaaaaaaaaaa", 1, &a);
    bizarre["versions"] = json!({"x": 1});
    let v = json!({
        "ok": true, "tid_base": "01007EF00011E000",
        "version_servie": {"version": 786432, "version_affichee": "1.6.0", "build_id": "8E9978D50BDD20B4C8395A106C27FFDE"},
        "mods": [liste, toutes, bizarre, mod_json("MAJUSCULES-0123456789ab", 1, &a), mod_json(CLE, 9, &a), {"nom": "sans clé"}, 5],
        "champ_nouveau": true
    });
    let l = lire_reponse(&v);
    assert_eq!(l.version_servie, Some(VersionServie { version: Some(786432), version_affichee: Some("1.6.0".into()), build_id: Some("8E9978D50BDD20B4C8395A106C27FFDE".into()) }));
    assert_eq!(l.mods.len(), 3);
    assert_eq!(l.ecartes, 4, "clé hors contrat, doublon, sans clé, pas un objet");
    assert_eq!(l.mods[0].revision, 3, "une révision en texte se lit");
    assert_eq!(l.mods[0].versions, Versions::Liste(vec![VersionVisee { build_id: Some("8E9978D50BDD20B4C8395A106C27FFDE".into()), version_brute: Some(786432), version_affichee: Some("1.6.0".into()) }]));
    assert_eq!(l.mods[0].compatible, Some(true));
    assert_eq!(l.mods[1].versions, Versions::Toutes);
    assert_eq!(l.mods[1].compatible, None);
    assert_eq!(l.mods[1].touche_sauvegardes, None);
    assert_eq!(l.mods[1].description, "");
    assert_eq!(l.mods[2].versions, Versions::Inconnues);
    assert_eq!(l.mods[0].fichier.taille, a.len() as u64);
    // La forme se garde telle quelle dans un manifeste (« toutes » reste une chaîne).
    assert_eq!(serde_json::to_value(&Versions::Toutes).unwrap(), json!("toutes"));
    assert_eq!(serde_json::from_value::<Versions>(json!("toutes")).unwrap(), Versions::Toutes);
    // Pas de version servie : null.
    assert_eq!(lire_reponse(&json!({"version_servie": null, "mods": []})).version_servie, None);
    // La route : title id de base en majuscules.
    assert_eq!(route_mods(BASE), "/switch/01007EF00011E000/mods");
    // La réponse simulée suit la même forme.
    assert_eq!(lire_reponse(&reponse_simulee(BASE)).mods.len(), 1);
}

#[test]
fn seule_la_route_d_archive_de_firehouse_est_admise() {
    assert_eq!(route_archive(&format!("/api/jeux/v1/switch/mods/{CLE}/archive"), CLE).unwrap(), format!("/switch/mods/{CLE}/archive"));
    for url in [
        format!("https://ailleurs.example/api/jeux/v1/switch/mods/{CLE}/archive"),
        format!("http://127.0.0.1:8100/api/jeux/v1/switch/mods/{CLE}/archive"),
        format!("//ailleurs.example/api/jeux/v1/switch/mods/{CLE}/archive"),
        format!("/api/jeux/v1/switch/mods/{CLE2}/archive"),
        format!("/api/jeux/v1/switch/mods/{CLE}/archive?x=1"),
        format!("/api/jeux/v1/switch/mods/{CLE}/../../moi/archive"),
        "/api/jeux/v1/jeu/110/fichier/200/0".to_string(),
        format!("/api/jeux/v2/switch/mods/{CLE}/archive"),
        String::new(),
    ] {
        assert!(route_archive(&url, CLE).is_err(), "{url}");
    }
}

#[tokio::test]
async fn une_adresse_refusee_ne_fait_partir_aucun_appel() {
    let m = MockServer::start();
    let tout = m.mock(|w, r| {
        w.any_request();
        r.status(200).body("x");
    });
    let d = tempfile::tempdir().unwrap();
    let a = zip_octets(&[("exefs/a.ips", b"x")], false);
    let mut mm = servi(CLE, 1, &a);
    mm.fichier.url = format!("{}/api/jeux/v1/switch/mods/{CLE}/archive", m.base_url());
    assert!(matches!(telecharger(&source(&m), d.path(), &mm, &rien).await, Err(Erreur::Refus(_))));
    mm.fichier.url = format!("/api/jeux/v1/switch/mods/{CLE2}/archive");
    assert!(telecharger(&source(&m), d.path(), &mm, &rien).await.is_err());
    // Une empreinte absente : refusée aussi, sans appel.
    let mut sans = servi(CLE, 1, &a);
    sans.fichier.sha256 = String::new();
    assert!(telecharger(&source(&m), d.path(), &sans, &rien).await.is_err());
    assert_eq!(tout.hits(), 0, "aucun appel n'est parti");
}

#[tokio::test]
async fn le_telechargement_reprend_par_range_puis_le_mod_est_range() {
    let m = MockServer::start();
    let d = tempfile::tempdir().unwrap();
    let a = zip_octets(&[("Mod 1080p/exefs/a.ips", b"IPS32PATCH"), ("Mod 1080p/lisez-moi.txt", b"bonjour")], false);
    let mm = servi(CLE, 2, &a);
    let stock = d.path().join("mods-switch").join("01007EF00011E000");
    let tele = stock.join(".telechargements");
    std::fs::create_dir_all(&tele).unwrap();
    // Une coupure : la moitié est déjà là.
    let moitie = a.len() / 2;
    let part = tele.join(nom_archive(&mm).replace(".zip", ".part"));
    std::fs::write(&part, &a[..moitie]).unwrap();
    let suite = m.mock(|w, r| {
        w.method(GET).path(format!("/api/jeux/v1/switch/mods/{CLE}/archive")).header("range", format!("bytes={moitie}-")).header("authorization", "Bearer jeton");
        r.status(206).header("content-range", format!("bytes {moitie}-{}/{}", a.len() - 1, a.len())).body(&a[moitie..]);
    });
    let r = telecharger_et_ranger(&source(&m), &stock, &mm, None, &rien, Box::new(|_| Ok(()))).await.unwrap();
    suite.assert();
    assert_eq!(r.revision, 2);
    assert_eq!(r.fichiers, vec![FichierRange { chemin: "exefs/a.ips".into(), taille: 10, sha256: sha(b"IPS32PATCH") }]);
    assert_eq!(std::fs::read(stock.join(CLE).join("exefs").join("a.ips")).unwrap(), b"IPS32PATCH");
    assert!(!stock.join(CLE).join("lisez-moi.txt").exists(), "seuls exefs, romfs et cheats sont gardés");
    assert!(!part.exists() && !tele.join(nom_archive(&mm)).exists(), "l'archive reçue est retirée une fois rangée");
    assert_eq!(ranges(&stock), vec![r]);
}

#[tokio::test]
async fn une_empreinte_fausse_ou_une_taille_differente_ne_gardent_rien() {
    let m = MockServer::start();
    let d = tempfile::tempdir().unwrap();
    let a = zip_octets(&[("romfs/a.bin", b"0123456789")], false);
    // Empreinte fausse : le .part est retiré.
    let mut faux = servi(CLE, 1, &a);
    faux.fichier.sha256 = "ab".repeat(32);
    let f = m.mock(|w, r| {
        w.method(GET).path(format!("/api/jeux/v1/switch/mods/{CLE}/archive"));
        r.status(200).body(&a);
    });
    assert!(telecharger(&source(&m), d.path(), &faux, &rien).await.is_err());
    let tele = d.path().join(".telechargements");
    assert_eq!(std::fs::read_dir(&tele).unwrap().count(), 0, "rien ne reste");
    // Firehouse envoie MOINS que la taille annoncée (empreinte de ce qu'il envoie) : refusé, rien gardé.
    let mut court = servi(CLE, 1, &a);
    court.fichier.taille = a.len() as u64 + 5;
    assert!(telecharger(&source(&m), d.path(), &court, &rien).await.is_err());
    assert!(!tele.join(nom_archive(&court)).exists());
    // Firehouse envoie PLUS que la taille annoncée : refusé (plafond), rien gardé.
    let mut long = servi(CLE, 1, &a);
    long.fichier.taille = a.len() as u64 - 3;
    assert!(telecharger(&source(&m), d.path(), &long, &rien).await.is_err());
    assert_eq!(std::fs::read_dir(&tele).unwrap().count(), 0);
    f.assert_hits(3);
}

#[tokio::test]
async fn un_409_n_est_demande_qu_une_fois() {
    let m = MockServer::start();
    let d = tempfile::tempdir().unwrap();
    let a = zip_octets(&[("romfs/a.bin", b"0123456789")], false);
    let c = m.mock(|w, r| {
        w.method(GET).path(format!("/api/jeux/v1/switch/mods/{CLE}/archive"));
        r.status(409).json_body(json!({"ok": false, "detail": "le fichier a changé"}));
    });
    assert!(matches!(telecharger(&source(&m), d.path(), &servi(CLE, 1, &a), &rien).await, Err(Erreur::Conflit(_))));
    c.assert_hits(1);
}

#[tokio::test]
async fn une_nouvelle_revision_prend_un_nouveau_part() {
    let a = zip_octets(&[("romfs/a.bin", b"0123456789")], false);
    let b = zip_octets(&[("romfs/a.bin", b"9876543210")], false);
    let (r1, r2) = (servi(CLE, 1, &a), servi(CLE, 2, &b));
    assert_ne!(nom_archive(&r1), nom_archive(&r2));
    assert!(nom_archive(&r2).starts_with(&format!("{CLE}-r2-{}", &sha(&b)[..12])));
    // Le .part d'une autre révision est retiré avant de télécharger la nouvelle.
    let m = MockServer::start();
    let d = tempfile::tempdir().unwrap();
    let tele = d.path().join(".telechargements");
    std::fs::create_dir_all(&tele).unwrap();
    let vieux = tele.join(nom_archive(&r1).replace(".zip", ".part"));
    std::fs::write(&vieux, &a[..5]).unwrap();
    let f = m.mock(|w, r| {
        w.method(GET).path(format!("/api/jeux/v1/switch/mods/{CLE}/archive"));
        r.status(200).body(&b);
    });
    let recu = telecharger(&source(&m), d.path(), &r2, &rien).await.unwrap();
    f.assert();
    assert!(!vieux.exists());
    assert_eq!(std::fs::read(recu).unwrap(), b);
}

#[tokio::test]
async fn la_lecture_suit_le_contrat_et_garde_la_liste_par_profil() {
    let m = MockServer::start();
    let d = tempfile::tempdir().unwrap();
    let (seb, soeur) = (d.path().join("profils").join("seb"), d.path().join("profils").join("soeur"));
    let a = zip_octets(&[("romfs/a.bin", b"x")], false);
    let route = "/api/jeux/v1/switch/01007EF00011E000/mods";
    let mut ok = m.mock(|w, r| {
        w.method(GET).path(route);
        r.status(200).json_body(json!({"ok": true, "version_servie": null, "mods": [mod_json(CLE, 1, &a)]}));
    });
    let l = lire(&source(&m), &seb, BASE, &vivant, &garder_libre).await.unwrap();
    assert_eq!((l.etat.as_str(), l.liste.mods.len()), ("ok", 1));
    ok.delete();
    // Firehouse injoignable : la dernière liste de CE profil, avec sa date ; jamais celle d'un autre.
    let mut panne = m.mock(|w, r| {
        w.method(GET).path(route);
        r.status(500);
    });
    let h = lire(&source(&m), &seb, BASE, &vivant, &garder_libre).await.unwrap();
    assert_eq!(h.etat, "hors_ligne");
    assert!(h.vue_le.is_some());
    assert_eq!(h.liste.mods.len(), 1);
    let autre = lire(&source(&m), &soeur, BASE, &vivant, &garder_libre).await.unwrap();
    assert_eq!((autre.etat.as_str(), autre.liste.mods.len()), ("injoignable", 0));
    panne.delete();
    // 404 (jeu caché pour ce jeton) : liste vide, et l'ancienne n'est plus montrée hors ligne.
    let mut cache = m.mock(|w, r| {
        w.method(GET).path(route);
        r.status(404);
    });
    let v = lire(&source(&m), &seb, BASE, &vivant, &garder_libre).await.unwrap();
    assert_eq!((v.etat.as_str(), v.liste.mods.len()), ("ok", 0));
    cache.delete();
    let _panne = m.mock(|w, r| {
        w.method(GET).path(route);
        r.status(503);
    });
    assert_eq!(lire(&source(&m), &seb, BASE, &vivant, &garder_libre).await.unwrap().liste.mods.len(), 0);
    // 401 remonte (jeton révoqué : on le dit, sans réessayer).
    let m2 = MockServer::start();
    let _j = m2.mock(|w, r| {
        w.method(GET).path(route);
        r.status(401);
    });
    assert!(matches!(lire(&source(&m2), &seb, BASE, &vivant, &garder_libre).await, Err(Erreur::JetonRefuse(_))));
    // Le mode simulé : la forme du contrat.
    assert_eq!(lire(&Source::Simulee, &d.path().join("simule"), BASE, &vivant, &garder_libre).await.unwrap().liste.mods.len(), 1);
}

#[test]
fn une_archive_compressee_ou_piegee_est_refusee_sans_rien_ranger() {
    let d = tempfile::tempdir().unwrap();
    let stock = d.path().join("stock");
    let essai = |octets: Vec<u8>| {
        let f = d.path().join("a.zip");
        std::fs::write(&f, &octets).unwrap();
        ranger(&stock, &f, &servi(CLE, 1, &octets), None)
    };
    assert!(matches!(essai(zip_octets(&[("romfs/a.bin", &[0u8; 4000])], true)), Err(Erreur::Refus(_))), "compressée");
    assert!(essai(zip_octets(&[("romfs/../../evil.bin", b"x")], false)).is_err(), "chemin qui remonte");
    assert!(essai(zip_octets(&[("C:/Windows/evil.bin", b"x")], false)).is_err(), "chemin absolu");
    assert!(essai(zip_octets(&[("romfs/CON.bin", b"x")], false)).is_err(), "nom réservé de Windows");
    assert!(essai(zip_octets(&[("romfs/a.bin", b"x"), ("ROMFS/A.bin", b"y")], false)).is_err(), "deux noms selon la casse");
    assert!(essai(zip_octets(&[("Mod/lisez-moi.txt", b"x")], false)).is_err(), "ni exefs, ni romfs, ni cheats");
    std::fs::write(d.path().join("b.7z"), [0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C, 0, 0]).unwrap();
    assert!(ranger(&stock, &d.path().join("b.7z"), &servi(CLE, 1, b"x"), None).is_err(), "pas un ZIP");
    // Plus gros une fois « décompressé » que la taille annoncée : refusé.
    let gros = zip_octets(&[("romfs/a.bin", &[1u8; 100])], false);
    let f = d.path().join("g.zip");
    std::fs::write(&f, &gros).unwrap();
    let mut menteur = servi(CLE, 1, &gros);
    menteur.fichier.taille = 50;
    assert!(ranger(&stock, &f, &menteur, None).is_err());
    assert!(ranges(&stock).is_empty());
    assert!(!stock.join(CLE).exists() && !stock.join(format!("{CLE}.nouveau")).exists());
}

#[test]
fn une_racine_de_mod_ou_les_dossiers_directs_sont_admis() {
    let d = tempfile::tempdir().unwrap();
    let stock = d.path().join("stock");
    let a = zip_octets(&[("RomFS/Actor/a.bin", b"a"), ("exefs/b.ips", b"b"), ("cheats/0123.txt", b"c"), ("image.png", b"i")], false);
    let f = d.path().join("a.zip");
    std::fs::write(&f, &a).unwrap();
    let r = ranger(&stock, &f, &servi(CLE, 1, &a), None).unwrap();
    let chemins: Vec<&str> = r.fichiers.iter().map(|f| f.chemin.as_str()).collect();
    assert_eq!(chemins, ["cheats/0123.txt", "exefs/b.ips", "romfs/Actor/a.bin"], "premier dossier en minuscules, le reste ignoré");
    assert!(range_intact(&stock, &r));
}

#[test]
fn une_nouvelle_revision_remplace_l_ancienne_seulement_une_fois_verifiee_et_le_retrait_exige_la_preuve() {
    let d = tempfile::tempdir().unwrap();
    let stock = d.path().join("stock");
    let a = zip_octets(&[("romfs/a.bin", b"un")], false);
    let fa = d.path().join("a.zip");
    std::fs::write(&fa, &a).unwrap();
    ranger(&stock, &fa, &servi(CLE, 1, &a), None).unwrap();
    // Une révision 2 fausse (compressée) : l'ancienne reste, intacte.
    let faux = zip_octets(&[("romfs/a.bin", &[2u8; 3000])], true);
    let ff = d.path().join("f.zip");
    std::fs::write(&ff, &faux).unwrap();
    assert!(ranger(&stock, &ff, &servi(CLE, 2, &faux), None).is_err());
    assert_eq!(ranges(&stock)[0].revision, 1);
    assert_eq!(std::fs::read(stock.join(CLE).join("romfs").join("a.bin")).unwrap(), b"un");
    // La vraie révision 2 : remplacée, l'ancienne partie (elle était intacte).
    let b = zip_octets(&[("romfs/b.bin", b"deux")], false);
    let fb = d.path().join("b.zip");
    std::fs::write(&fb, &b).unwrap();
    let r2 = ranger(&stock, &fb, &servi(CLE, 2, &b), None).unwrap();
    assert_eq!(ranges(&stock), vec![r2.clone()]);
    assert!(!stock.join(CLE).join("romfs").join("a.bin").exists());
    assert!(!stock.join(format!("{CLE}.ancien")).exists());
    // Un fichier rangé modifié : le retrait est refusé, rien n'est effacé.
    std::fs::write(stock.join(CLE).join("romfs").join("b.bin"), b"autre").unwrap();
    assert!(matches!(retirer(&stock, CLE), Err(Erreur::Refus(_))));
    assert!(stock.join(CLE).join("romfs").join("b.bin").exists());
    // … et une nouvelle révision met l'ancien dossier modifié DE CÔTÉ (jamais effacé).
    let c = zip_octets(&[("romfs/c.bin", b"trois")], false);
    let fc = d.path().join("c.zip");
    std::fs::write(&fc, &c).unwrap();
    ranger(&stock, &fc, &servi(CLE, 3, &c), None).unwrap();
    let garde = std::fs::read_dir(&stock).unwrap().flatten().find(|e| e.file_name().to_string_lossy().starts_with(&format!("{CLE}.garde-"))).unwrap();
    assert_eq!(std::fs::read(garde.path().join("romfs").join("b.bin")).unwrap(), b"autre");
    // Intact : retiré.
    retirer(&stock, CLE).unwrap();
    assert!(ranges(&stock).is_empty() && !stock.join(CLE).exists() && !stock.join(format!("{CLE}.json")).exists());
    assert!(retirer(&stock, CLE).is_err());
}

#[test]
fn le_choix_des_mods_note_seulement_l_eteint_et_met_les_parties_a_l_abri_a_tout_changement() {
    let d = tempfile::tempdir().unwrap();
    let donnees = d.path();
    assert!(charger(donnees).0.jeu(BASE).mod_actif(CLE), "un mod téléchargé est allumé sans rien écrire");
    // Rien ne change : rien n'est écrit, aucun abri.
    assert!(!choisir_mods(donnees, BASE, &[(CLE.into(), true)]).unwrap());
    assert!(!crate::contenus_switch::chemin(donnees).exists());
    // Éteint : noté, abri.
    assert!(choisir(donnees, BASE, &format!("mod:{CLE}"), false).unwrap());
    let c = charger(donnees).0.jeu(BASE);
    assert_eq!(c.mods_eteints, vec![format!("mod:{CLE}")]);
    assert!(c.abri_a_faire("ryubing:x") && !c.mod_actif(CLE));
    crate::contenus_switch::noter_abri_fait(donnees, BASE, "ryubing:x").unwrap();
    assert!(!charger(donnees).0.jeu(BASE).abri_a_faire("ryubing:x"));
    // Rallumé : l'abri revient aussi (TOUT changement de mod).
    assert!(choisir_mods(donnees, BASE, &[(CLE.into(), true)]).unwrap());
    let c = charger(donnees).0.jeu(BASE);
    assert!(c.mods_eteints.is_empty() && c.abri_a_faire("ryubing:x"));
    // Allumer A et éteindre B d'un coup.
    choisir_mods(donnees, BASE, &[(CLE.into(), true), (CLE2.into(), false)]).unwrap();
    let c = charger(donnees).0.jeu(BASE);
    assert!(c.mod_actif(CLE) && !c.mod_actif(CLE2));
    // Un premier téléchargement, une révision, un retrait : abri ; le retrait oublie l'éteint.
    crate::contenus_switch::noter_abri_fait(donnees, BASE, "eden:y").unwrap();
    noter_changement_de_mod(donnees, BASE, Some(CLE2)).unwrap();
    let c = charger(donnees).0.jeu(BASE);
    assert!(c.mod_actif(CLE2) && c.abri_a_faire("eden:y"));
    // Une clé hors contrat est refusée ; les DLC et la mise à jour gardent leurs règles.
    assert!(choisir(donnees, BASE, "mod:../x", false).is_err());
    assert!(choisir(donnees, BASE, "mod:", false).is_err());
    assert!(choisir(donnees, BASE, "autre:x", false).is_err());
}

#[test]
fn un_conflit_refuse_d_allumer_sauf_si_l_autre_s_eteint_en_meme_temps() {
    let man = |cle: &str, nom: &str, conflits: &[&str]| ManifesteMod {
        cle: cle.into(),
        nom: nom.into(),
        revision: 1,
        sha256: String::new(),
        taille: 1,
        versions: Versions::Toutes,
        version_servie: None,
        compatible: None,
        raison: None,
        conflits: conflits.iter().map(|s| s.to_string()).collect(),
        reglage_requis: String::new(),
        touche_sauvegardes: None,
        contenu: ContenuMod::default(),
        range_le: String::new(),
        fichiers: vec![],
        description: String::new(),
        type_mod: None,
        interet: None,
        public: None,
        adulte_vu: false,
    };
    let rangs = vec![man(CLE, "1080p", &[CLE2]), man(CLE2, "Haute résolution", &[])];
    let mut choix = crate::contenus_switch::ChoixJeu::default();
    let m = conflit(&rangs, &choix, CLE, None).unwrap();
    assert!(m.contains("« 1080p » ne va pas avec « Haute résolution »"), "{m}");
    // Dans l'autre sens aussi (le conflit n'est noté que d'un côté).
    assert!(conflit(&rangs, &choix, CLE2, None).is_some());
    assert_eq!(conflit(&rangs, &choix, CLE, Some(CLE2)), None, "l'autre s'éteint en même temps");
    choix.mods_eteints.push(format!("mod:{CLE2}"));
    assert_eq!(conflit(&rangs, &choix, CLE, None), None, "l'autre est déjà éteint");
}

#[test]
fn les_noms_reserves_des_emulateurs_ne_sont_jamais_pris() {
    for n in ["Update", "update", "UPDATE@65536", "Update@", "DLC", "dlc", "SDMC", "sdmc"] {
        assert!(nom_reserve(n), "{n}");
        assert!(!nom_reserve(&nom_de_dossier(n, CLE)), "{n} → {}", nom_de_dossier(n, CLE));
    }
    assert_eq!(nom_de_dossier("Update", CLE), "Mod Update");
    assert_eq!(nom_de_dossier("60 fps / 4K : <test>", CLE), "60 fps 4K test");
    assert_eq!(nom_de_dossier("CON", CLE), format!("Mod {CLE}"));
    assert_eq!(nom_de_dossier("...", CLE), format!("Mod {CLE}"));
    assert_eq!(nom_de_dossier("Résolution 1080p.", CLE), "Résolution 1080p");
    assert!(!nom_reserve("Updated textures"));
    // Un nom déjà pris par la personne : suffixe, jamais par-dessus.
    let pris = ["1080p".to_string(), "1080p (2)".to_string()];
    assert_eq!(nom_libre("1080p", &|n: &str| pris.iter().any(|p| p.eq_ignore_ascii_case(n))), "1080p (3)");
    assert_eq!(nom_libre("Libre", &|_| false), "Libre");
}

#[test]
fn la_compatibilite_suit_la_mise_a_jour_utilisee() {
    let servie = VersionServie { version: Some(786432), version_affichee: Some("1.6.0".into()), build_id: None };
    let liste = Versions::Liste(vec![VersionVisee { build_id: Some("8E99".into()), version_brute: Some(786432), version_affichee: Some("1.6.0".into()) }]);
    let etat = |v: &Versions, c: Option<bool>, u: Option<u32>| compatibilite(v, c, None, Some(&servie), u);
    // « toutes » : toujours compatible, même version inconnue.
    assert_eq!(etat(&Versions::Toutes, Some(false), None).etat, "compatible");
    // Version inconnue : jamais grisé.
    assert_eq!(etat(&liste, Some(false), None).etat, "pas_verifie");
    // Une version visée de même version brute : compatible.
    assert_eq!(etat(&liste, Some(false), Some(786432)).etat, "compatible");
    // La mise à jour servie, sans entrée : le jugement de Firehouse (null = pas vérifié, false = grisé).
    let vide = Versions::Liste(vec![]);
    assert_eq!(etat(&vide, Some(true), Some(786432)).etat, "compatible");
    assert_eq!(etat(&vide, None, Some(786432)).etat, "pas_verifie");
    let g = compatibilite(&vide, Some(false), Some("le build ne correspond pas"), Some(&servie), Some(786432));
    assert_eq!((g.etat, g.raison.as_deref()), ("incompatible", Some("le build ne correspond pas")));
    assert_eq!(etat(&vide, Some(false), Some(786432)).raison.as_deref(), Some("ne va pas avec la version 1.6.0 de ce jeu"));
    // Une autre mise à jour, ou aucune : grisé, raison sans jargon.
    let autre = etat(&liste, Some(true), Some(720896));
    assert_eq!(autre.etat, "incompatible");
    assert_eq!(autre.raison.as_deref(), Some("fait pour la version 1.6.0, ce jeu utilise une autre mise à jour"));
    assert!(autre.grise());
    assert_eq!(etat(&liste, Some(true), Some(0)).raison.as_deref(), Some("fait pour la version 1.6.0, ce jeu utilise la version d'origine (sans mise à jour)"));
    // Des versions visées de forme inconnue : jamais grisé.
    assert_eq!(etat(&Versions::Inconnues, Some(true), Some(0)).etat, "pas_verifie");
}

#[test]
fn un_build_id_sans_version_brute_n_est_jamais_grise() {
    // La forme réelle de Firehouse (MK8D, 25/11) : un Build ID lu dans le .ips, version brute encore inconnue,
    // version servie pas lue.
    let mk8d = Versions::Liste(vec![VersionVisee { build_id: Some("4B7A0BB5AE57E627B63E2247F9470E57".into()), version_brute: None, version_affichee: None }]);
    for u in [Some(0), Some(196608), None] {
        let c = compatibilite(&mk8d, None, Some("vise 4B7A0BB5AE57E627 ; la signature de la version servie n'est pas encore lue"), None, u);
        assert_eq!(c.etat, "pas_verifie", "{u:?}");
        assert!(!c.grise());
    }
    // Mélangée à une entrée connue qui ne colle pas : toujours pas grisé (on ne sait pas tout).
    let mixte = Versions::Liste(vec![
        VersionVisee { build_id: Some("AAAA".into()), version_brute: Some(786432), version_affichee: Some("1.6.0".into()) },
        VersionVisee { build_id: Some("BBBB".into()), version_brute: None, version_affichee: None },
    ]);
    assert_eq!(compatibilite(&mixte, None, None, None, Some(0)).etat, "pas_verifie");
    // Une entrée connue qui colle reste compatible.
    assert_eq!(compatibilite(&mixte, None, None, None, Some(786432)).etat, "compatible");
    // Le jugement de Firehouse sur la version servie passe toujours avant : false = grisé.
    let servie = VersionServie { version: Some(0), version_affichee: None, build_id: None };
    assert!(compatibilite(&mk8d, Some(false), Some("le build ne correspond pas"), Some(&servie), Some(0)).grise());
}

#[test]
fn la_version_utilisee_par_ryubing_se_lit_dans_sa_liste() {
    use crate::ryubing::version_selectionnee;
    let notre = "D:\\Jeux\\Zelda\\Zelda [v786432].nsp";
    assert_eq!(version_selectionnee(None, Some(notre), Some(786432)), Some(0), "pas de liste : aucune mise à jour");
    assert_eq!(version_selectionnee(Some(&json!({"selected": "", "paths": [notre]})), Some(notre), Some(786432)), Some(0));
    assert_eq!(version_selectionnee(Some(&json!({"selected": notre})), Some(notre), Some(786432)), Some(786432));
    assert_eq!(version_selectionnee(Some(&json!({"selected": "E:/Autre/Zelda [v851968].nsp"})), Some(notre), Some(786432)), Some(851968));
    assert_eq!(version_selectionnee(Some(&json!({"selected": "E:/Autre/Zelda.nsp"})), Some(notre), Some(786432)), None, "nom sans version : inconnue");
    assert_eq!(version_selectionnee(Some(&json!({"selected": 5})), None, None), None);
    // Eden : la mise à jour trouvée et allumée, coupée (0), ou inconnue (aucune trouvée).
    let maj = crate::switch_nca::ContenuLu { chemin: notre.into(), sorte: crate::switch_nca::Sorte::MiseAJour, base: BASE, version: Some(786432), forme: crate::switch_nca::Forme::Nsp };
    let mut c = crate::switch_nca::ContenusDuJeu { base: BASE, ..Default::default() };
    let mut choix = crate::contenus_switch::ChoixJeu::default();
    assert_eq!(crate::eden::maj_utilisee(&c, &choix), None);
    c.maj = Some(maj);
    assert_eq!(crate::eden::maj_utilisee(&c, &choix), Some(786432));
    choix.maj = false;
    assert_eq!(crate::eden::maj_utilisee(&c, &choix), Some(0));
}

#[test]
fn le_dossier_des_mods_d_eden_reste_dans_eden() {
    let d = tempfile::tempdir().unwrap();
    let eden = d.path().join("Eden");
    let ini = eden.join("user").join("config");
    std::fs::create_dir_all(&ini).unwrap();
    assert_eq!(crate::eden::dossier_load(&eden), Some(eden.join("user").join("load")));
    std::fs::write(ini.join("qt-config.ini"), format!("[Data%20Storage]\r\nload_directory\\default=false\r\nload_directory={}/mods/\r\n", eden.to_string_lossy().replace('\\', "/"))).unwrap();
    assert_eq!(crate::eden::dossier_load(&eden), Some(PathBuf::from(format!("{}\\mods", eden.to_string_lossy()))));
    std::fs::write(ini.join("qt-config.ini"), "[Data%20Storage]\r\nload_directory=C:/Ailleurs/load/\r\n").unwrap();
    assert_eq!(crate::eden::dossier_load(&eden), None);
    std::fs::write(ini.join("qt-config.ini"), format!("[Data%20Storage]\r\nload_directory={}/../load\r\n", eden.to_string_lossy())).unwrap();
    assert_eq!(crate::eden::dossier_load(&eden), None);
}

/// Un rangement de test : deux mods (le second avec un réglage requis), rangés depuis de vraies archives.
fn stock_de_test(d: &Path) -> PathBuf {
    let stock = dossier_stock(&d.join("Jeu").join("Frogtend"), BASE);
    let a = zip_octets(&[("romfs/a.bin", b"un")], false);
    let fa = d.join("a.zip");
    std::fs::write(&fa, &a).unwrap();
    ranger(&stock, &fa, &servi(CLE, 1, &a), None).unwrap();
    let b = zip_octets(&[("exefs/b.ips", b"deux")], false);
    let fb = d.join("b.zip");
    std::fs::write(&fb, &b).unwrap();
    let mut m2 = servi(CLE2, 1, &b);
    m2.nom = "60 i/s".into();
    m2.reglage_requis = "VSync « Switch 60 Hz »".into();
    m2.versions = Versions::Toutes;
    ranger(&stock, &fb, &m2, None).unwrap();
    stock
}

#[test]
fn la_pose_ryubing_copie_les_mods_et_garde_les_entrees_de_la_personne_dans_mods_json() {
    let d = tempfile::tempdir().unwrap();
    let donnees = d.path().join("donnees");
    let stock = stock_de_test(d.path());
    let ryu = d.path().join("Ryubing");
    let racine = crate::ryubing::racine(&ryu, "Seb");
    std::fs::create_dir_all(ryu.join("portable")).unwrap();
    // Le dossier du jeu existe déjà en MAJUSCULES chez la personne, avec son mod à elle et un dossier « 1080p ».
    let contenus = racine.join("mods").join("contents");
    let tid = contenus.join("01007EF00011E000");
    std::fs::create_dir_all(tid.join("1080p").join("romfs")).unwrap();
    std::fs::write(tid.join("1080p").join("romfs").join("sien.bin"), b"sien").unwrap();
    let jeu = crate::ryubing::dossier_du_jeu(&racine, BASE);
    std::fs::create_dir_all(&jeu).unwrap();
    let sien = json!({"mods": [{"name": "1080p", "path": tid.join("1080p").to_string_lossy(), "enabled": false}], "autre": 1});
    std::fs::write(jeu.join("mods.json"), serde_json::to_vec(&sien).unwrap()).unwrap();
    // Version 1.6.0 utilisée : le mod 1080p est posé ; le 60 i/s (réglage requis) jamais.
    let b = poser_ryubing(&donnees, &ryu, &racine, BASE, &stock, Some(786432)).unwrap();
    assert_eq!(b.poses, vec!["1080p (2)".to_string()], "{b:?}");
    assert!(b.raisons.iter().any(|r| r.contains("réglage d'émulateur requis : VSync")), "{b:?}");
    let pose = tid.join("1080p (2)");
    assert_eq!(std::fs::read(pose.join("romfs").join("a.bin")).unwrap(), b"un");
    assert_eq!(std::fs::read(tid.join("1080p").join("romfs").join("sien.bin")).unwrap(), b"sien", "le dossier de la personne n'est pas touché");
    assert!(tid.join(NOTE_POSE).is_file() && !pose.join(NOTE_POSE).exists(), "la note est à côté, pas dans le mod");
    // mods.json : l'entrée de la personne gardée telle quelle, la nôtre ajoutée avec le chemin COMPLET exact.
    let v: Value = serde_json::from_slice(&std::fs::read(jeu.join("mods.json")).unwrap()).unwrap();
    assert_eq!(v["autre"], 1);
    assert_eq!(v["mods"][0], sien["mods"][0]);
    assert_eq!(v["mods"][1], json!({"name": "1080p (2)", "path": pose.to_string_lossy(), "enabled": true}));
    assert!(pose.to_string_lossy().contains("\\01007EF00011E000\\"), "la casse du dossier de la personne est reprise (Ryubing compare exactement)");
    // Une mise à jour d'une autre version : grisé → retiré (intact), son entrée part ; celle de la personne reste.
    let b = poser_ryubing(&donnees, &ryu, &racine, BASE, &stock, Some(0)).unwrap();
    assert_eq!(b.retires, vec!["1080p (2)".to_string()], "{b:?}");
    assert!(b.raisons.iter().any(|r| r.contains("ce jeu utilise la version d'origine")), "{b:?}");
    assert!(!pose.exists());
    let v: Value = serde_json::from_slice(&std::fs::read(jeu.join("mods.json")).unwrap()).unwrap();
    assert_eq!(v["mods"], json!([sien["mods"][0]]));
    assert!(!tid.join(NOTE_POSE).exists());
}

#[test]
fn un_dossier_pose_puis_modifie_n_est_jamais_efface_et_reste_coupe() {
    let d = tempfile::tempdir().unwrap();
    let donnees = d.path().join("donnees");
    let stock = stock_de_test(d.path());
    let ryu = d.path().join("Ryubing");
    let racine = crate::ryubing::racine(&ryu, "Seb");
    std::fs::create_dir_all(ryu.join("portable")).unwrap();
    poser_ryubing(&donnees, &ryu, &racine, BASE, &stock, None).unwrap();
    let pose = racine.join("mods").join("contents").join("01007ef00011e000").join("1080p");
    assert!(pose.join("romfs").join("a.bin").is_file(), "version inconnue : pas grisé, posé");
    // La personne change un fichier, puis le mod est éteint dans Frogtend.
    std::fs::write(pose.join("romfs").join("a.bin"), b"retouche").unwrap();
    choisir_mods(&donnees, BASE, &[(CLE.into(), false)]).unwrap();
    // Parties à l'abri : sans parties, rien à copier, l'abri est noté fait.
    let b = poser_ryubing(&donnees, &ryu, &racine, BASE, &stock, None).unwrap();
    assert!(b.retires.is_empty() && b.raisons.iter().any(|r| r.contains("a été modifié")), "{b:?}");
    assert_eq!(std::fs::read(pose.join("romfs").join("a.bin")).unwrap(), b"retouche");
    assert!(!charger(&donnees).0.jeu(BASE).abri_a_faire(&crate::contenus_switch::lieu("ryubing", &racine)));
    let v: Value = serde_json::from_slice(&std::fs::read(crate::ryubing::fichier_mods_json(&racine, BASE)).unwrap()).unwrap();
    assert_eq!(v["mods"][0]["enabled"], false, "gardé, mais coupé dans Ryubing");
    // Rallumé : jamais écrasé non plus.
    choisir_mods(&donnees, BASE, &[(CLE.into(), true)]).unwrap();
    let b = poser_ryubing(&donnees, &ryu, &racine, BASE, &stock, None).unwrap();
    assert!(b.poses.is_empty(), "{b:?}");
    assert_eq!(std::fs::read(pose.join("romfs").join("a.bin")).unwrap(), b"retouche");
}

#[test]
fn le_dernier_avis_de_firehouse_sur_une_revision_rangee_est_retenu() {
    let d = tempfile::tempdir().unwrap();
    let stock = d.path().join("stock");
    let a = zip_octets(&[("romfs/a.bin", b"un")], false);
    let f = d.path().join("a.zip");
    std::fs::write(&f, &a).unwrap();
    ranger(&stock, &f, &servi(CLE, 1, &a), None).unwrap();
    let mut avis = mod_json(CLE, 1, &a);
    avis["compatible"] = json!(false);
    avis["raison"] = json!("plante au chargement");
    avis["conflits"] = json!([CLE2]);
    let liste = lire_reponse(&json!({"version_servie": {"version": 786432}, "mods": [avis]}));
    // Pendant qu'un téléchargement occupe le rangement : rien n'est réécrit (ce sera pour la lecture suivante).
    let g = occuper_stock(&stock).unwrap();
    assert_eq!(rafraichir(&stock, &liste), 0);
    drop(g);
    assert_eq!(rafraichir(&stock, &liste), 1);
    assert!(!std::fs::read_dir(&stock).unwrap().flatten().any(|e| e.file_name().to_string_lossy().ends_with(".tmp")), "aucun provisoire laissé");
    let r = &ranges(&stock)[0];
    assert_eq!((r.compatible, r.raison.as_deref(), r.conflits.clone()), (Some(false), Some("plante au chargement"), vec![CLE2.to_string()]));
    assert_eq!(rafraichir(&stock, &liste), 0, "rien de neuf : rien d'écrit");
    // Une AUTRE révision servie : le manifeste de la révision rangée ne bouge pas.
    let autre = lire_reponse(&json!({"mods": [mod_json(CLE, 2, &a)]}));
    assert_eq!(rafraichir(&stock, &autre), 0);
    assert!(range_intact(&stock, &ranges(&stock)[0]));
}

#[test]
fn sans_mod_a_poser_rien_n_est_cree_dans_l_emulateur() {
    let d = tempfile::tempdir().unwrap();
    let ryu = d.path().join("Ryubing");
    let racine = crate::ryubing::racine(&ryu, "Seb");
    std::fs::create_dir_all(ryu.join("portable")).unwrap();
    // Pas de rangement : rien.
    let b = poser_ryubing(&d.path().join("donnees"), &ryu, &racine, BASE, &d.path().join("aucun"), None).unwrap();
    assert_eq!(b, BilanMods::default());
    assert!(!racine.join("mods").exists());
    // Un rangement dont tout est éteint : aucun dossier créé.
    let stock = stock_de_test(d.path());
    choisir_mods(&d.path().join("donnees"), BASE, &[(CLE.into(), false)]).unwrap();
    poser_ryubing(&d.path().join("donnees"), &ryu, &racine, BASE, &stock, None).unwrap();
    assert!(!racine.join("mods").join("contents").join("01007ef00011e000").exists());
}

#[test]
fn ryubing_non_portable_ne_recoit_rien() {
    let d = tempfile::tempdir().unwrap();
    let stock = stock_de_test(d.path());
    let ryu = d.path().join("Ryubing");
    let racine = crate::ryubing::racine(&ryu, "Seb");
    let b = poser_ryubing(&d.path().join("donnees"), &ryu, &racine, BASE, &stock, None).unwrap();
    assert!(b.raisons[0].contains("pas été installé par Frogtend"));
    assert!(!racine.join("mods").exists());
}

#[test]
fn mods_json_garde_ce_qui_n_est_pas_a_frogtend() {
    use crate::ryubing::mods_json_voulu;
    let nos = vec![("1080p".to_string(), "D:\\R\\mods\\contents\\x\\1080p".to_string(), true)];
    // Pas de fichier : créé.
    assert_eq!(mods_json_voulu(None, &nos, &[]).unwrap().unwrap(), json!({"mods": [{"name": "1080p", "path": "D:\\R\\mods\\contents\\x\\1080p", "enabled": true}]}));
    // Rien à poser ni à retirer, pas de fichier : rien n'est écrit.
    assert_eq!(mods_json_voulu(None, &[], &[]).unwrap(), None);
    // Déjà juste : rien à écrire.
    let juste = json!({"mods": [{"name": "1080p", "path": "D:\\R\\mods\\contents\\x\\1080p", "enabled": true}]});
    assert_eq!(mods_json_voulu(Some(&juste), &nos, &[]).unwrap(), None);
    // Une forme inattendue : on n'y touche pas.
    assert!(mods_json_voulu(Some(&json!([1])), &nos, &[]).is_err());
    assert!(mods_json_voulu(Some(&json!({"mods": 3})), &nos, &[]).is_err());
    // Un mod parti : son entrée part, celle de la personne reste.
    let avant = json!({"mods": [{"name": "Sien", "path": "E:\\sien", "enabled": true}, {"name": "Vieux", "path": "D:\\R\\vieux", "enabled": true}]});
    assert_eq!(mods_json_voulu(Some(&avant), &[], &["D:\\R\\vieux".into()]).unwrap().unwrap(), json!({"mods": [{"name": "Sien", "path": "E:\\sien", "enabled": true}]}));
}

#[test]
fn la_pose_eden_copie_les_mods_et_garde_les_coupures_de_la_personne() {
    let d = tempfile::tempdir().unwrap();
    let donnees = d.path().join("donnees");
    let stock = stock_de_test(d.path());
    let eden = d.path().join("Eden");
    let config = eden.join("user").join("config");
    std::fs::create_dir_all(&config).unwrap();
    // La personne a coupé son mod « 60 fps, 4K » et une vieille mise à jour pour ce jeu.
    let titre = BASE;
    let ini = format!(
        "[UI]\r\ntheme=dark\r\n\r\n[DisabledAddOns]\r\n1\\title_id\\default=false\r\n1\\title_id={titre}\r\n1\\disabled\\1\\d\\default=false\r\n1\\disabled\\1\\d=\"60 fps, 4K\"\r\n1\\disabled\\2\\d\\default=false\r\n1\\disabled\\2\\d=Update@65536\r\n1\\disabled\\size=2\r\nsize=1\r\n"
    );
    std::fs::write(config.join("qt-config.ini"), &ini).unwrap();
    // Allumé : posé (copié), rien de coupé pour lui ; le 60 i/s (réglage requis) jamais posé seul.
    let b = poser_eden(&donnees, &eden, BASE, &stock, None, false).unwrap();
    let load = eden.join("user").join("load").join("01007EF00011E000");
    assert_eq!(b.poses, vec!["1080p".to_string()], "{b:?}");
    assert_eq!(std::fs::read(load.join("1080p").join("romfs").join("a.bin")).unwrap(), b"un");
    assert!(!load.join("60 i s").exists(), "réglage requis : jamais posé seul");
    let texte = std::fs::read_to_string(config.join("qt-config.ini")).unwrap();
    assert_eq!(crate::eden::lire_coupures(&texte), vec![(BASE, vec!["60 fps, 4K".to_string(), "Update@65536".to_string()])], "rien de coupé pour un mod allumé, la personne gardée");
    // La personne retouche la copie, puis le mod est éteint dans Frogtend : gardé, et coupé dans Eden.
    std::fs::write(load.join("1080p").join("romfs").join("a.bin"), b"retouche").unwrap();
    choisir_mods(&donnees, BASE, &[(CLE.into(), false)]).unwrap();
    // Eden ouvert : rien ne change.
    let b = poser_eden(&donnees, &eden, BASE, &stock, None, true).unwrap();
    assert!(b.raisons[0].contains("Eden est ouvert"));
    let b = poser_eden(&donnees, &eden, BASE, &stock, None, false).unwrap();
    assert!(b.retires.is_empty(), "{b:?}");
    let texte = std::fs::read_to_string(config.join("qt-config.ini")).unwrap();
    assert_eq!(crate::eden::lire_coupures(&texte), vec![(BASE, vec!["60 fps, 4K".to_string(), "Update@65536".to_string(), "1080p".to_string()])]);
    assert!(texte.contains("theme=dark"));
    // Le fichier remis au même contenu par la personne (sa date a changé : toujours « touché »), puis rallumé : gardé
    // tel quel, jamais écrasé ; le nom sort des coupures, les autres restent.
    std::fs::write(load.join("1080p").join("romfs").join("a.bin"), b"un").unwrap();
    choisir_mods(&donnees, BASE, &[(CLE.into(), true)]).unwrap();
    let b = poser_eden(&donnees, &eden, BASE, &stock, None, false).unwrap();
    assert!(b.poses.is_empty() && b.raisons.iter().any(|r| r.contains("a été modifié")), "{b:?}");
    let texte = std::fs::read_to_string(config.join("qt-config.ini")).unwrap();
    assert_eq!(crate::eden::lire_coupures(&texte), vec![(BASE, vec!["60 fps, 4K".to_string(), "Update@65536".to_string()])]);
    // La personne efface son dossier retouché : Frogtend repose sa copie.
    std::fs::remove_dir_all(load.join("1080p")).unwrap();
    let b = poser_eden(&donnees, &eden, BASE, &stock, None, false).unwrap();
    assert_eq!(b.poses, vec!["1080p".to_string()]);
    // Retiré du rangement : la copie intacte part de chez Eden.
    retirer(&stock, CLE).unwrap();
    let b = poser_eden(&donnees, &eden, BASE, &stock, None, false).unwrap();
    // Plus rangé : on ne sait plus s'il était réservé, il n'est plus nommé au journal.
    assert_eq!(b.retires, vec![MOD_RETIRE.to_string()]);
    assert!(!load.join("1080p").exists());
}

#[test]
fn les_coupures_d_eden_s_etendent_aux_mods_de_frogtend() {
    let actuels = vec!["Sien".to_string(), "Update".to_string()];
    assert_eq!(crate::eden::coupures_voulues_avec_mods(&actuels, None, (0, 0), &[("1080p".into(), false)]), ["Sien", "Update", "1080p"]);
    assert_eq!(crate::eden::coupures_voulues_avec_mods(&["Sien".into(), "1080p".into()], None, (0, 0), &[("1080p".into(), true)]), ["Sien"]);
    assert_eq!(crate::eden::coupures_voulues(&actuels, None, (0, 0)), actuels, "sans mods : comme avant");
}

#[test]
fn la_liste_vue_dit_l_etat_de_chaque_mod() {
    let d = tempfile::tempdir().unwrap();
    let stock = stock_de_test(d.path());
    let rangs = ranges(&stock);
    let a = zip_octets(&[("romfs/a.bin", b"un")], false);
    let mut v2 = mod_json(CLE, 2, &a);
    v2["nom"] = json!("1080p");
    let lecture = Lecture { etat: "ok".into(), vue_le: None, liste: lire_reponse(&json!({"version_servie": {"version": 786432, "version_affichee": "1.6.0"}, "mods": [v2, mod_json("nouveau-0123456789ab", 1, &a)]})) };
    let choix = crate::contenus_switch::ChoixJeu::default();
    let vue = liste_vue(&lecture, &rangs, &choix, Some(786432), BASE);
    let etat = |c: &str| vue.mods.iter().find(|m| m.cle == c).map(|m| (m.etat.clone(), m.allume, m.servi)).unwrap();
    assert_eq!(etat(CLE), ("revision_dispo".into(), true, true));
    assert_eq!(etat("nouveau-0123456789ab"), ("pas_telecharge".into(), false, true));
    assert_eq!(etat(CLE2), ("telecharge".into(), true, false), "rangé, plus servi");
    assert_eq!(vue.tid_base.as_deref(), Some("01007EF00011E000"));
    assert!(vue.version_connue);
    let grise = liste_vue(&lecture, &rangs, &choix, Some(0), BASE);
    assert_eq!(grise.mods.iter().find(|m| m.cle == CLE).unwrap().etat, "incompatible");
}

// --- Corrections après la relecture des experts (18/11) ---

/// Un rangement avec UN mod (1080p, pour la 1.6.0) et une racine Ryubing portable avec une partie.
fn ryubing_avec_partie(d: &Path) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    let donnees = d.join("donnees");
    let stock = dossier_stock(&d.join("Jeu").join("Frogtend"), BASE);
    let a = zip_octets(&[("romfs/a.bin", b"un")], false);
    let fa = d.join("a.zip");
    std::fs::write(&fa, &a).unwrap();
    ranger(&stock, &fa, &servi(CLE, 1, &a), None).unwrap();
    let ryu = d.join("Ryubing");
    std::fs::create_dir_all(ryu.join("portable")).unwrap();
    let racine = crate::ryubing::racine(&ryu, "Seb");
    let save = racine.join("bis").join("user").join("save").join("0000000000000001").join("0");
    std::fs::create_dir_all(&save).unwrap();
    std::fs::write(save.join("partie.sav"), b"sacree").unwrap();
    (donnees, stock, ryu, racine)
}

fn abris(ryu: &Path) -> usize {
    std::fs::read_dir(ryu.join(".frogtend-sauvegardes")).map(|l| l.count()).unwrap_or(0)
}

#[test]
fn b2_un_mod_qui_devient_posable_sans_changement_de_choix_met_d_abord_les_parties_a_l_abri() {
    let d = tempfile::tempdir().unwrap();
    let (donnees, stock, ryu, racine) = ryubing_avec_partie(d.path());
    // Version d'origine : grisé, rien posé, aucun abri (rien ne change chez Ryubing).
    let b = poser_ryubing(&donnees, &ryu, &racine, BASE, &stock, Some(0)).unwrap();
    assert!(b.poses.is_empty());
    assert_eq!(abris(&ryu), 0);
    // La personne rallume la mise à jour (aucun interrupteur de mod n'a bougé) : le plan change → abri AVANT la pose.
    assert!(!charger(&donnees).0.jeu(BASE).abri_a_faire(&crate::contenus_switch::lieu("ryubing", &racine)));
    let b = poser_ryubing(&donnees, &ryu, &racine, BASE, &stock, Some(786432)).unwrap();
    assert_eq!(b.poses, vec!["1080p".to_string()]);
    assert_eq!(abris(&ryu), 1, "parties à l'abri avant la pose");
    // L'abri est dit au journal (essai de Seb 18/11 : il manquait).
    assert!(b.abrite && b.resume().starts_with("parties mises à l'abri avant"), "{}", b.resume());
    // Rien ne change ensuite : pas de nouvel abri.
    let b = poser_ryubing(&donnees, &ryu, &racine, BASE, &stock, Some(786432)).unwrap();
    assert!(!b.abrite && !b.resume().contains("abri"), "{}", b.resume());
    assert_eq!(abris(&ryu), 1);
}

#[test]
fn b2_abri_impossible_rien_ne_change_et_la_raison_va_sur_la_fiche() {
    let d = tempfile::tempdir().unwrap();
    let (donnees, stock, ryu, racine) = ryubing_avec_partie(d.path());
    // L'abri ne peut pas être créé (un FICHIER à la place de son dossier).
    std::fs::write(ryu.join(".frogtend-sauvegardes"), b"x").unwrap();
    let b = poser_ryubing(&donnees, &ryu, &racine, BASE, &stock, Some(786432)).unwrap();
    assert!(b.inchange && b.poses.is_empty(), "{b:?}");
    assert!(b.raisons.iter().any(|r| r.contains("pas pu être mises à l'abri")), "{b:?}");
    assert!(!racine.join("mods").join("contents").join("01007ef00011e000").join("1080p").exists());
    assert!(!crate::ryubing::fichier_mods_json(&racine, BASE).exists(), "Ryubing pas touché non plus");
    // La raison est gardée pour la fiche.
    let p = derniere_pose(&stock).unwrap();
    assert_eq!(p.emulateur, "Ryubing");
    assert!(p.raisons.iter().any(|r| r.contains("pas pu être mises à l'abri")));
}

#[tokio::test]
async fn i1_une_session_perimee_ne_garde_rien() {
    let m = MockServer::start();
    let d = tempfile::tempdir().unwrap();
    let a = zip_octets(&[("romfs/a.bin", b"x")], false);
    let _ok = m.mock(|w, r| {
        w.method(GET).path("/api/jeux/v1/switch/01007EF00011E000/mods");
        r.status(200).json_body(json!({"mods": [mod_json(CLE, 1, &a)]}));
    });
    let perimee = || true;
    assert!(matches!(lire(&source(&m), d.path(), BASE, &perimee, &garder_libre).await, Err(Erreur::Perimee(_))));
    assert!(!fichier_cache(d.path(), BASE).exists(), "rien de ce qu'un ancien jeton a reçu n'est gardé");
    // Périmée ENTRE la vérification et l'écriture : c'est l'écriture (sous le verrou de la session) qui refuse, et la
    // liste n'est pas rendue.
    let refuse = |_: &Path, _: &[u8]| -> Resultat<()> { Err(Erreur::Perimee("remplacée".into())) };
    assert!(matches!(lire(&source(&m), d.path(), BASE, &vivant, &refuse).await, Err(Erreur::Perimee(_))));
    assert!(!fichier_cache(d.path(), BASE).exists());
}

#[test]
fn i2_un_seul_travail_par_jeu_et_un_seul_ecrivain_par_rangement() {
    let t = Travail::default();
    assert!(t.commencer(7, 2));
    assert!(!t.commencer(7, 1), "un second profil ne relance pas le même jeu");
    assert!(t.actif(7) && !t.actif(8));
    t.modifier(7, |s| s.actif = false);
    assert!(t.commencer(7, 1));
    let d = tempfile::tempdir().unwrap();
    let stock = d.path().join("stock");
    let g = occuper_stock(&stock).unwrap();
    assert!(occuper_stock(&stock).is_err());
    assert!(occuper_stock(&d.path().join("STOCK")).is_err(), "même rangement, autre casse");
    assert!(matches!(retirer(&stock, CLE), Err(Erreur::Refus(_))), "pas de retrait pendant un téléchargement");
    drop(g);
    assert!(occuper_stock(&stock).is_ok());
}

#[test]
fn i3_le_changement_de_mod_est_note_avant_l_echange_et_un_echec_ne_change_rien() {
    let d = tempfile::tempdir().unwrap();
    let donnees = d.path().join("donnees");
    let stock = d.path().join("stock");
    let a = zip_octets(&[("romfs/a.bin", b"un")], false);
    let fa = d.path().join("a.zip");
    std::fs::write(&fa, &a).unwrap();
    // Noté AVANT l'échange : au moment de l'appel, l'ancien dossier n'existe pas encore (premier téléchargement).
    let vu = std::sync::Mutex::new(false);
    ranger_avec(&stock, &fa, &servi(CLE, 1, &a), None, &|x| {
        *vu.lock().unwrap() = !stock.join(CLE).exists();
        noter_arrivee(&donnees, &stock, BASE, x).map(|_| ())
    })
    .unwrap();
    assert!(*vu.lock().unwrap());
    assert!(charger(&donnees).0.jeu(BASE).abri_a_faire("ryubing:x"));
    // Une révision dont la note échoue : l'ancienne reste telle quelle.
    let b = zip_octets(&[("romfs/b.bin", b"deux")], false);
    let fb = d.path().join("b.zip");
    std::fs::write(&fb, &b).unwrap();
    assert!(ranger_avec(&stock, &fb, &servi(CLE, 2, &b), None, &|_| Err(Erreur::Disque("note impossible".into()))).is_err());
    assert_eq!(ranges(&stock)[0].revision, 1);
    assert!(range_intact(&stock, &ranges(&stock)[0]));
    assert!(!stock.join(format!("{CLE}.nouveau")).exists());
    // La garde de « Jouer » : le message est lisible.
    assert!(PARTIE_PENDANT_TELECHARGEMENT.contains("en train d'arriver"));
}

#[tokio::test]
async fn i4_seules_les_pannes_donnent_hors_ligne() {
    let d = tempfile::tempdir().unwrap();
    let route = "/api/jeux/v1/switch/01007EF00011E000/mods";
    for (code, corps) in [(400, "{}"), (403, "{}"), (409, "{}"), (200, "pas du json"), (200, "[1]")] {
        let m = MockServer::start();
        let _r = m.mock(|w, r| {
            w.method(GET).path(route);
            r.status(code).body(corps);
        });
        let r = lire(&source(&m), d.path(), BASE, &vivant, &garder_libre).await;
        assert!(matches!(r, Err(Erreur::Refus(_) | Erreur::Conflit(_))), "{code} {corps} : {r:?}");
    }
    for code in [500, 502, 503] {
        let m = MockServer::start();
        let _r = m.mock(|w, r| {
            w.method(GET).path(route);
            r.status(code);
        });
        assert_eq!(lire(&source(&m), d.path(), BASE, &vivant, &garder_libre).await.unwrap().etat, "injoignable", "{code}");
    }
    // Personne au bout du fil (réseau) : injoignable.
    let s = Source::Firehouse(crate::firehouse::Client::nouveau("http://127.0.0.1:9", "jeton").unwrap());
    assert_eq!(lire(&s, d.path(), BASE, &vivant, &garder_libre).await.unwrap().etat, "injoignable");
}

/// Deux mods à poser, dont le second déjà posé en révision 1 (à remplacer).
#[cfg(windows)]
#[test]
fn i5_la_note_suit_chaque_pose_meme_si_une_etape_suivante_echoue() {
    use std::os::windows::fs::OpenOptionsExt;
    let d = tempfile::tempdir().unwrap();
    let stock = d.path().join("stock");
    let dj = d.path().join("contents").join("01007ef00011e000");
    let rangement = |cle: &str, nom: &str, rev: u64, contenu: &[u8]| {
        let a = zip_octets(&[("romfs/a.bin", contenu)], false);
        let f = d.path().join(format!("{cle}.zip"));
        std::fs::write(&f, &a).unwrap();
        let mut m = servi(cle, rev, &a);
        m.nom = nom.into();
        m.versions = Versions::Toutes;
        ranger(&stock, &f, &m, None).unwrap();
    };
    rangement(CLE2, "Zelda HD", 1, b"un");
    let choix = crate::contenus_switch::ChoixJeu::default();
    let mut r = Vec::new();
    synchroniser(&dj, &voulus(&stock, &choix, None, &mut r), false, &|| Ok(())).unwrap();
    assert!(dj.join("Zelda HD").exists());
    rangement(CLE2, "Zelda HD", 2, b"deux");
    rangement(CLE, "1080p", 1, b"trois");
    // Un fichier du dossier à remplacer est verrouillé (ouvert sans partage) : le remplacement échoue.
    let _verrou = std::fs::OpenOptions::new().read(true).share_mode(0).open(dj.join("Zelda HD").join("romfs").join("a.bin")).unwrap();
    assert!(synchroniser(&dj, &voulus(&stock, &choix, None, &mut r), false, &|| Ok(())).is_err());
    let note = lire_note(&dj).unwrap();
    assert!(note.contains_key(CLE), "le mod posé AVANT l'échec est noté : jamais pris pour un dossier de la personne");
    assert!(dj.join("1080p").exists());
    assert!(!std::fs::read_dir(dj.parent().unwrap()).unwrap().flatten().any(|e| e.file_name().to_string_lossy().starts_with(PREFIXE_COPIE)), "la copie provisoire est partie");
}

#[test]
fn i6_meme_revision_autre_empreinte_est_reposee() {
    let d = tempfile::tempdir().unwrap();
    let stock = d.path().join("stock");
    let a = zip_octets(&[("romfs/a.bin", b"un")], false);
    let f = d.path().join("a.zip");
    std::fs::write(&f, &a).unwrap();
    let mut m = servi(CLE, 1, &a);
    m.versions = Versions::Toutes;
    ranger(&stock, &f, &m, None).unwrap();
    let dj = d.path().join("contents").join("01007ef00011e000");
    let choix = crate::contenus_switch::ChoixJeu::default();
    let mut r = Vec::new();
    let v = voulus(&stock, &choix, None, &mut r);
    synchroniser(&dj, &v, false, &|| Ok(())).unwrap();
    let note = lire_note(&dj).unwrap();
    assert_eq!(note[CLE].sha256, sha(&a));
    assert_eq!(planifier(&dj, &v, &note).etapes, vec![], "même révision, même empreinte : rien");
    let mut autre = note.clone();
    autre.get_mut(CLE).unwrap().sha256 = "ab".repeat(32);
    assert_eq!(planifier(&dj, &v, &autre).etapes, vec![Etape::Remplacer { cle: CLE.into(), nom: "1080p".into() }]);
}

#[test]
fn i7_un_mod_en_conflit_n_est_pas_pose_et_arrive_eteint() {
    let d = tempfile::tempdir().unwrap();
    let donnees = d.path().join("donnees");
    let stock = d.path().join("stock");
    let range = |cle: &str, nom: &str, conflits: Vec<String>| {
        let a = zip_octets(&[("romfs/a.bin", nom.as_bytes())], false);
        let f = d.path().join(format!("{cle}.zip"));
        std::fs::write(&f, &a).unwrap();
        let mut m = servi(cle, 1, &a);
        m.nom = nom.into();
        m.versions = Versions::Toutes;
        m.conflits = conflits;
        ranger_avec(&stock, &f, &m, None, &|x| noter_arrivee(&donnees, &stock, BASE, x).map(|_| ())).unwrap();
    };
    range(CLE, "1080p", vec![]);
    range(CLE2, "Haute résolution", vec![CLE.to_string()]);
    let choix = charger(&donnees).0.jeu(BASE);
    assert!(choix.mod_actif(CLE) && !choix.mod_actif(CLE2), "le nouveau, en conflit avec un mod allumé, arrive éteint");
    // Les deux allumés quand même (choix forcé ailleurs) : aucun des deux n'est posé, la raison est dite.
    let mut tous = choix.clone();
    tous.mods_eteints.clear();
    let mut raisons = Vec::new();
    let v = voulus(&stock, &tous, None, &mut raisons);
    assert!(v.iter().all(|x| !x.poser));
    assert!(raisons.iter().any(|r| r.contains("ne va pas avec")), "{raisons:?}");
}

#[test]
fn i8_sans_place_sur_le_disque_de_l_emulateur_rien_n_est_pose() {
    let d = tempfile::tempdir().unwrap();
    let stock = d.path().join("stock");
    let a = zip_octets(&[("romfs/a.bin", b"un")], false);
    let f = d.path().join("a.zip");
    std::fs::write(&f, &a).unwrap();
    let mut m = servi(CLE, 1, &a);
    m.versions = Versions::Toutes;
    ranger(&stock, &f, &m, None).unwrap();
    let dj = d.path().join("contents").join("01007ef00011e000");
    let mut r = Vec::new();
    let mut v = voulus(&stock, &crate::contenus_switch::ChoixJeu::default(), None, &mut r);
    // Un mod annoncé démesuré (bien plus que n'importe quel disque).
    v[0].m.fichiers[0].taille = u64::MAX / 4;
    let b = synchroniser(&dj, &v, false, &|| Ok(())).unwrap();
    assert!(b.inchange && b.poses.is_empty());
    assert!(b.raisons.iter().any(|x| x.contains("pas assez de place")));
    assert!(!dj.exists());
}

#[test]
fn d1_un_ancien_dossier_de_cote_pas_verifie_n_est_jamais_efface() {
    let d = tempfile::tempdir().unwrap();
    let stock = d.path().join("stock");
    let a = zip_octets(&[("romfs/a.bin", b"un")], false);
    let fa = d.path().join("a.zip");
    std::fs::write(&fa, &a).unwrap();
    ranger(&stock, &fa, &servi(CLE, 1, &a), None).unwrap();
    // Un `<clé>.ancien` laissé là (on ne sait pas ce qu'il contient) : jamais effacé.
    std::fs::create_dir_all(stock.join(format!("{CLE}.ancien"))).unwrap();
    std::fs::write(stock.join(format!("{CLE}.ancien")).join("x"), b"inconnu").unwrap();
    let b = zip_octets(&[("romfs/b.bin", b"deux")], false);
    let fb = d.path().join("b.zip");
    std::fs::write(&fb, &b).unwrap();
    ranger(&stock, &fb, &servi(CLE, 2, &b), None).unwrap();
    assert_eq!(std::fs::read(stock.join(format!("{CLE}.ancien")).join("x")).unwrap(), b"inconnu");
    assert_eq!(ranges(&stock)[0].revision, 2);
    // L'ancienne révision vérifiée, elle, est partie (aucun autre `.ancien-…`).
    assert!(!std::fs::read_dir(&stock).unwrap().flatten().any(|e| e.file_name().to_string_lossy().starts_with(&format!("{CLE}.ancien-"))));
}

#[test]
fn d2_le_retrait_ne_laisse_rien() {
    let d = tempfile::tempdir().unwrap();
    let stock = d.path().join("stock");
    let a = zip_octets(&[("romfs/a.bin", b"un")], false);
    let fa = d.path().join("a.zip");
    std::fs::write(&fa, &a).unwrap();
    ranger(&stock, &fa, &servi(CLE, 1, &a), None).unwrap();
    retirer(&stock, CLE).unwrap();
    let restes: Vec<String> = std::fs::read_dir(&stock).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
    assert!(restes.is_empty(), "{restes:?}");
}

#[test]
fn d3_le_chemin_de_mods_json_est_celui_que_ryubing_compose() {
    let d = tempfile::tempdir().unwrap();
    let (donnees, stock, ryu, racine) = ryubing_avec_partie(d.path());
    // La racine passée à `-r` écrite avec des « / » : Ryubing la normalise (GetFullPath) ; le dossier `Contents`
    // existe chez la personne avec une majuscule : Ryubing l'énumère tel quel.
    std::fs::create_dir_all(racine.join("mods").join("Contents")).unwrap();
    let racine_slash = PathBuf::from(racine.to_string_lossy().replace('\\', "/"));
    poser_ryubing(&donnees, &ryu, &racine_slash, BASE, &stock, Some(786432)).unwrap();
    let v: Value = serde_json::from_slice(&std::fs::read(crate::ryubing::fichier_mods_json(&racine, BASE)).unwrap()).unwrap();
    let attendu = format!("{}{s}mods{s}Contents{s}01007ef00011e000{s}1080p", racine.to_string_lossy(), s = std::path::MAIN_SEPARATOR);
    assert_eq!(v["mods"][0]["path"], attendu);
}

#[test]
fn d4_une_note_qui_vise_hors_du_dossier_est_ignoree() {
    let d = tempfile::tempdir().unwrap();
    let dj = d.path().join("contents").join("01007ef00011e000");
    std::fs::create_dir_all(&dj).unwrap();
    std::fs::create_dir_all(d.path().join("victime")).unwrap();
    let note = json!({
        CLE: {"dossier": "../../victime", "revision": 1, "fichiers": {}},
        CLE2: {"dossier": "CON", "revision": 1, "fichiers": {}},
        "autre-aaaaaaaaaaaa": {"dossier": ".frogtend-copie-x", "revision": 1, "fichiers": {}},
        "bonne-bbbbbbbbbbbb": {"dossier": "Mon mod", "revision": 1, "fichiers": {}},
        "pas une cle": {"dossier": "x", "revision": 1, "fichiers": {}}
    });
    std::fs::write(dj.join(NOTE_POSE), serde_json::to_vec(&note).unwrap()).unwrap();
    let l = lire_note(&dj).unwrap();
    assert_eq!(l.keys().cloned().collect::<Vec<_>>(), vec!["bonne-bbbbbbbbbbbb".to_string()]);
    synchroniser(&dj, &[], false, &|| Ok(())).unwrap();
    assert!(d.path().join("victime").exists());
    for n in ["Mod", "1080p (2)", "Résolution"] {
        assert!(nom_simple(n), "{n}");
    }
    for n in ["a/b", "a\\b", "..", "C:x", "nul.txt", "x.", ".frogtend-copie-1"] {
        assert!(!nom_simple(n), "{n}");
    }
}

#[test]
fn d5_seule_une_copie_marquee_par_frogtend_est_effacee() {
    let d = tempfile::tempdir().unwrap();
    let stock = d.path().join("stock");
    let a = zip_octets(&[("romfs/a.bin", b"un")], false);
    let fa = d.path().join("a.zip");
    std::fs::write(&fa, &a).unwrap();
    ranger(&stock, &fa, &servi(CLE, 1, &a), None).unwrap();
    let contents = d.path().join("contents");
    let dj = contents.join("01007ef00011e000");
    // Un reste NON marqué (pas à Frogtend) et un reste marqué (copie interrompue de Frogtend).
    let nom = format!("{PREFIXE_COPIE}01007ef00011e000-{CLE}");
    std::fs::create_dir_all(contents.join(&nom)).unwrap();
    std::fs::write(contents.join(&nom).join("a.personne"), b"x").unwrap();
    let marque = contents.join(format!("{nom}-2"));
    std::fs::create_dir_all(&marque).unwrap();
    std::fs::write(marque.join(MARQUE_COPIE), b"").unwrap();
    let mut r = Vec::new();
    let v = voulus(&stock, &crate::contenus_switch::ChoixJeu::default(), Some(786432), &mut r);
    synchroniser(&dj, &v, false, &|| Ok(())).unwrap();
    assert!(contents.join(&nom).join("a.personne").exists(), "jamais effacé : pas marqué");
    assert!(!marque.exists(), "copie interrompue de Frogtend : réutilisée puis mise à sa place");
    assert!(dj.join("1080p").join("romfs").join("a.bin").exists());
}

#[test]
fn d6_une_liste_de_versions_vide_n_est_jamais_grisee() {
    let servie = VersionServie { version: Some(786432), version_affichee: Some("1.6.0".into()), build_id: None };
    assert_eq!(compatibilite(&Versions::Liste(vec![]), Some(true), None, Some(&servie), Some(0)).etat, "pas_verifie");
    assert_eq!(compatibilite(&Versions::Liste(vec![]), None, None, None, Some(720896)).etat, "pas_verifie");
}

#[test]
fn d7_la_liste_vue_compare_aussi_l_empreinte_et_ne_deborde_pas() {
    use crate::commandes::{mods_accordes, ModAccorde};
    let a = zip_octets(&[("romfs/a.bin", b"un")], false);
    let servis = vec![servi(CLE, 1, &a)];
    let vu = |sha: String, taille: u64| ModAccorde { cle: CLE.into(), revision: 1, taille, sha256: sha };
    let t = a.len() as u64;
    assert_eq!(mods_accordes(&servis, &[vu(sha(&a), t)], t).unwrap().len(), 1);
    assert!(matches!(mods_accordes(&servis, &[vu("ab".repeat(32), t)], t), Err(Erreur::Conflit(_))), "autre empreinte");
    assert!(matches!(mods_accordes(&servis, &[vu(sha(&a), t)], t + 1), Err(Erreur::Conflit(_))), "autre total");
    // Deux mods dont la somme déborde : refusé, jamais un total qui « tombe juste » par débordement.
    let mut gros = servis.clone();
    let mut g2 = servi(CLE2, 1, &a);
    g2.fichier.taille = u64::MAX;
    gros[0].fichier.taille = 2;
    gros.push(g2);
    let vus = [ModAccorde { cle: CLE.into(), revision: 1, taille: 2, sha256: sha(&a) }, ModAccorde { cle: CLE2.into(), revision: 1, taille: u64::MAX, sha256: sha(&a) }];
    assert!(matches!(mods_accordes(&gros, &vus, 1), Err(Erreur::Conflit(_))));
}

#[test]
fn d8_eden_sans_fichier_de_reglages_n_en_recoit_jamais() {
    let d = tempfile::tempdir().unwrap();
    let donnees = d.path().join("donnees");
    let stock = stock_de_test(d.path());
    let eden = d.path().join("Eden");
    std::fs::create_dir_all(eden.join("user")).unwrap();
    let ini = crate::contenus::ini_eden(&eden);
    let b = poser_eden(&donnees, &eden, BASE, &stock, None, false).unwrap();
    assert_eq!(b.poses, vec!["1080p".to_string()]);
    assert!(!ini.exists(), "jamais créé");
    // Un mod gardé modifié puis éteint : il ne peut pas être coupé sans fichier — dit, rien créé.
    let load = eden.join("user").join("load").join("01007EF00011E000");
    std::fs::write(load.join("1080p").join("romfs").join("a.bin"), b"retouche").unwrap();
    choisir_mods(&donnees, BASE, &[(CLE.into(), false)]).unwrap();
    let b = poser_eden(&donnees, &eden, BASE, &stock, None, false).unwrap();
    assert!(b.raisons.iter().any(|r| r.contains("pas encore de fichier de réglages")), "{b:?}");
    assert!(!ini.exists());
    // Un fichier VIDE : pas touché non plus.
    std::fs::create_dir_all(ini.parent().unwrap()).unwrap();
    std::fs::write(&ini, b"").unwrap();
    poser_eden(&donnees, &eden, BASE, &stock, None, false).unwrap();
    assert_eq!(std::fs::read(&ini).unwrap(), b"");
}

/// L'accord de l'interface (`api.modsSwitchTelecharger`, `clesVues`) reprend la clé, la révision, la taille ET
/// l'empreinte montrées par la liste vue : la forme réellement envoyée se lit telle quelle dans `ModAccorde`.
#[test]
fn l_accord_de_l_interface_porte_l_empreinte_de_la_liste_vue() {
    let a = zip_octets(&[("romfs/a.bin", b"un")], false);
    let lecture = Lecture { etat: "ok".into(), vue_le: None, liste: lire_reponse(&json!({"mods": [mod_json("nouveau-0123456789ab", 3, &a)]})) };
    let vue = liste_vue(&lecture, &[], &crate::contenus_switch::ChoixJeu::default(), Some(786432), BASE);
    let m = &vue.mods[0];
    assert_eq!(m.sha256, sha(&a));
    let vu = serde_json::to_value(m).unwrap();
    assert_eq!(vu["sha256"], json!(sha(&a)), "la liste vue expose l'empreinte à l'interface");
    // Ce que l'interface envoie (mods.ts, `clesVues`) : {cle, revision, taille, sha256}.
    let envoye = json!([{"cle": vu["cle"], "revision": vu["revision"], "taille": vu["taille"], "sha256": vu["sha256"]}]);
    let lu: Vec<crate::commandes::ModAccorde> = serde_json::from_value(envoye).unwrap();
    assert_eq!((lu[0].cle.as_str(), lu[0].revision, lu[0].taille, lu[0].sha256.as_str()), ("nouveau-0123456789ab", 3, a.len() as u64, sha(&a).as_str()));
}

// --- Relecture finale (18/11) ---

#[test]
fn f2_sans_mises_a_jour_lisibles_les_mods_sont_quand_meme_remis_d_accord() {
    let d = tempfile::tempdir().unwrap();
    // Le title id de base vient du nom du fichier (ou de son ticket), sans clés.
    let f = d.path().join("Zelda [01007EF00011E800][v786432].nsp");
    std::fs::write(&f, b"pas un vrai nsp").unwrap();
    assert_eq!(base_du_fichier(&f), Some(BASE));
    assert_eq!(base_du_fichier(&d.path().join("Zelda.nsp")), None);
    // Version inconnue (None) : un mod ÉTEINT et déjà posé est bien retiré.
    let donnees = d.path().join("donnees");
    let stock = stock_de_test(d.path());
    let ryu = d.path().join("Ryubing");
    let racine = crate::ryubing::racine(&ryu, "Seb");
    std::fs::create_dir_all(ryu.join("portable")).unwrap();
    poser_ryubing(&donnees, &ryu, &racine, BASE, &stock, None).unwrap();
    let pose = racine.join("mods").join("contents").join("01007ef00011e000").join("1080p");
    assert!(pose.exists());
    choisir_mods(&donnees, BASE, &[(CLE.into(), false)]).unwrap();
    let b = poser_ryubing(&donnees, &ryu, &racine, BASE, &stock, None).unwrap();
    assert_eq!(b.retires, vec!["1080p".to_string()]);
    assert!(!pose.exists());
    // Jeu pas reconnu : la raison va sur la fiche de chaque rangement du jeu.
    let frogtend = d.path().join("Jeu").join("Frogtend");
    assert_eq!(noter_raison_partout(&frogtend, "Eden", RAISON_SANS_TITRE), 1);
    let p = derniere_pose(&stock).unwrap();
    assert_eq!((p.emulateur.as_str(), p.raisons.clone()), ("Eden", vec![RAISON_SANS_TITRE.to_string()]));
    assert_eq!(noter_raison_partout(&d.path().join("aucun"), "Eden", RAISON_SANS_TITRE), 0);
}

#[test]
fn f5_les_refus_montres_a_la_personne_sont_sans_jargon() {
    let d = tempfile::tempdir().unwrap();
    let stock = d.path().join("stock");
    let essai = |octets: Vec<u8>| {
        let f = d.path().join("a.zip");
        std::fs::write(&f, &octets).unwrap();
        match ranger(&stock, &f, &servi(CLE, 1, &octets), None) {
            Err(Erreur::Refus(m)) => m,
            r => panic!("{r:?}"),
        }
    };
    let m1 = essai(zip_octets(&[("romfs/a.bin", &[0u8; 4000])], true));
    let m2 = essai(zip_octets(&[("Mod/lisez-moi.txt", b"x")], false));
    for m in [m1, m2] {
        for mot in ["contrat", "ZIP", "zip", "exefs", "romfs", "cheats", "compress"] {
            assert!(!m.contains(mot), "« {mot} » dans : {m}");
        }
    }
}

// --- Contrat 1.46 : la fiche de chaque mod, et le public « adulte » (brief § 2) ---

const ADULTE: &str = "adulte-00000000000a";

#[test]
fn c146_la_fiche_se_lit_avec_ses_null() {
    let a = zip_octets(&[("romfs/a.bin", b"x")], false);
    let mut avec = mod_json(CLE, 1, &a);
    avec["description"] = json!("Le jeu s'affiche en 1080p.");
    avec["type"] = json!("Technique");
    avec["interet"] = json!(5);
    avec["public"] = json!("tous");
    let mut sans = mod_json(CLE2, 1, &a);
    for k in ["description", "type", "interet", "public"] {
        sans[k] = Value::Null;
    }
    let mut faux = mod_json("autre-aaaaaaaaaaaa", 1, &a);
    faux["interet"] = json!(9);
    faux["type"] = json!(3);
    let l = lire_reponse(&json!({"mods": [avec, sans, faux]}));
    assert_eq!((l.mods[0].type_mod.as_deref(), l.mods[0].interet, l.mods[0].public.as_deref()), (Some("technique"), Some(5), Some("tous")));
    assert_eq!(l.mods[0].description, "Le jeu s'affiche en 1080p.");
    assert_eq!((l.mods[1].type_mod.clone(), l.mods[1].interet, l.mods[1].public.clone(), l.mods[1].description.as_str()), (None, None, None, ""));
    assert_eq!((l.mods[2].type_mod.clone(), l.mods[2].interet), (None, None), "hors des valeurs du contrat : ignoré");
}

#[test]
fn c146_la_liste_vue_trie_par_interet_puis_par_nom() {
    let a = zip_octets(&[("romfs/a.bin", b"x")], false);
    let m = |cle: &str, nom: &str, interet: Value| {
        let mut v = mod_json(cle, 1, &a);
        v["nom"] = json!(nom);
        v["interet"] = interet;
        v
    };
    let lecture = Lecture {
        etat: "ok".into(),
        vue_le: None,
        liste: lire_reponse(&json!({"mods": [
            m("a-000000000001", "zèbre", Value::Null), m("b-000000000001", "Beta", json!(2)), m("c-000000000001", "alpha", json!(5)),
            m("d-000000000001", "Alpha bis", json!(2)), m("e-000000000001", "Aa", Value::Null)
        ]})),
    };
    let vue = liste_vue(&lecture, &[], &crate::contenus_switch::ChoixJeu::default(), None, BASE);
    let noms: Vec<&str> = vue.mods.iter().map(|m| m.nom.as_str()).collect();
    assert_eq!(noms, ["alpha", "Alpha bis", "Beta", "Aa", "zèbre"]);
    assert_eq!((vue.mods[0].interet, vue.mods[0].type_mod.as_deref()), (Some(5), None));
    // La forme JSON vue par l'interface : `type` et `interet`.
    let j = serde_json::to_value(&vue.mods[0]).unwrap();
    assert_eq!((j["interet"].clone(), j["type"].clone()), (json!(5), Value::Null));
}

/// Un mod « adulte » rangé par un profil ADMIN, à côté d'un mod pour tous (1080p).
fn rangement_avec_adulte(d: &Path) -> (PathBuf, ListeMods, ListeMods) {
    let stock = dossier_stock(&d.join("Jeu").join("Frogtend"), BASE);
    let a = zip_octets(&[("romfs/a.bin", b"un")], false);
    let fa = d.join("a.zip");
    std::fs::write(&fa, &a).unwrap();
    let mut tous = mod_json(CLE, 1, &a);
    tous["versions"] = json!("toutes");
    tous["public"] = json!("tous");
    ranger(&stock, &fa, &ModServi::depuis(&tous).unwrap(), None).unwrap();
    let b = zip_octets(&[("romfs/b.bin", b"secret")], false);
    let fb = d.join("b.zip");
    std::fs::write(&fb, &b).unwrap();
    let mut adulte = mod_json(ADULTE, 1, &b);
    adulte["nom"] = json!("Nom Secret");
    adulte["versions"] = json!("toutes");
    adulte["public"] = json!("adulte");
    ranger(&stock, &fb, &ModServi::depuis(&adulte).unwrap(), None).unwrap();
    let liste_admin = lire_reponse(&json!({"mods": [tous.clone(), adulte]}));
    let liste_autre = lire_reponse(&json!({"mods": [tous]}));
    (stock, liste_admin, liste_autre)
}

#[test]
fn c146_un_mod_adulte_n_existe_que_pour_le_profil_qui_le_recoit() {
    let d = tempfile::tempdir().unwrap();
    let (stock, admin, autre) = rangement_avec_adulte(d.path());
    let rangs = ranges(&stock);
    let r = rangs.iter().find(|m| m.cle == ADULTE).unwrap();
    assert!(r.reserve() && r.adulte_vu && r.public.as_deref() == Some("adulte"));
    let choix = crate::contenus_switch::ChoixJeu::default();
    let vue = |l: &ListeMods, etat: &str| liste_vue(&Lecture { etat: etat.into(), vue_le: None, liste: l.clone() }, &rangs, &choix, None, BASE);
    // Profil A (admin) : montré.
    assert!(vue(&admin, "ok").mods.iter().any(|m| m.cle == ADULTE));
    // Profil B : ni montré, ni en « plus servi » ; hors ligne avec son cache, ou injoignable sans rien, pareil.
    for (l, e) in [(&autre, "ok"), (&autre, "hors_ligne"), (&ListeMods::default(), "injoignable")] {
        let v = vue(l, e);
        assert!(!v.mods.iter().any(|m| m.cle == ADULTE || m.nom.contains("Secret")), "{e}");
        assert!(v.mods.iter().any(|m| m.cle == CLE), "le mod pour tous reste montré ({e})");
    }
    // Le suivi des téléchargements et le journal (communs à la maison) ne le nomment jamais.
    let servi_adulte = admin.mods.iter().find(|m| m.cle == ADULTE).unwrap();
    assert_eq!(nom_a_dire(servi_adulte, &rangs), MOD_RESERVE);
    assert_eq!(nom_a_dire(&admin.mods[0], &rangs), "1080p");
    // Une nouvelle révision servie SANS fiche (`public: null`) d'un mod déjà « adulte » : jamais nommée non plus.
    let mut sans_fiche = servi_adulte.clone();
    sans_fiche.public = None;
    assert_eq!(nom_a_dire(&sans_fiche, &rangs), MOD_RESERVE);
    // Et même « tous » : le mod rangé a été « adulte » une fois.
    sans_fiche.public = Some("tous".into());
    assert_eq!(nom_a_dire(&sans_fiche, &rangs), MOD_RESERVE);
    // Le nom d'un mod rangé, pour le journal (choix d'un interrupteur).
    assert_eq!(nom_range_a_dire(rangs.iter().find(|m| m.cle == ADULTE).unwrap()), MOD_RESERVE);
    assert_eq!(nom_range_a_dire(rangs.iter().find(|m| m.cle == CLE).unwrap()), "1080p");
    // Écarté, jamais effacé.
    assert!(stock.join(ADULTE).is_dir());
    assert_eq!(visibles(rangs.clone(), None).len(), 1);
    assert_eq!(visibles(rangs.clone(), Some(&admin)).len(), 2);
}

#[test]
fn c146_fiche_redevenue_null_apres_adulte_reste_reservee() {
    let d = tempfile::tempdir().unwrap();
    let (stock, _admin, _autre) = rangement_avec_adulte(d.path());
    // La fiche est réécrite à null (Firehouse la refait) : rafraîchie par la liste de l'admin, le mod reste réservé.
    let b = zip_octets(&[("romfs/b.bin", b"secret")], false);
    let mut nul = mod_json(ADULTE, 1, &b);
    nul["public"] = Value::Null;
    nul["versions"] = json!("toutes");
    assert_eq!(rafraichir(&stock, &lire_reponse(&json!({"mods": [nul]}))), 1);
    let r = ranges(&stock).into_iter().find(|m| m.cle == ADULTE).unwrap();
    assert_eq!(r.public, None);
    assert!(r.reserve(), "« adulte » une fois : réservé tant que la fiche est vide");
    // Sans fiche (`null`), même jamais « adulte » : réservé tant que Firehouse ne dit pas « tous » ou « ado »
    // (relecture des experts : un mod rangé avant sa fiche, puis classé « adulte », ne fuit jamais).
    let jamais = ranges(&stock).into_iter().find(|m| m.cle == CLE).unwrap();
    assert!(ManifesteMod { public: None, ..jamais.clone() }.reserve());
    assert!(ManifesteMod { public: Some("inconnu".into()), ..jamais.clone() }.reserve());
    assert!(!ManifesteMod { public: Some("ado".into()), ..jamais.clone() }.reserve());
    assert!(!jamais.reserve(), "« tous »");
}

#[test]
fn c146_a_la_pose_le_profil_b_ne_recoit_ni_ne_lit_le_mod_adulte_puis_a_le_retrouve() {
    let d = tempfile::tempdir().unwrap();
    let donnees = d.path().join("donnees");
    let (stock, admin, autre) = rangement_avec_adulte(d.path());
    let eden = d.path().join("Eden");
    let config = eden.join("user").join("config");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("qt-config.ini"), "[UI]\r\ntheme=dark\r\n").unwrap();
    let load = eden.join("user").join("load").join("01007EF00011E000");
    let neutre = format!("Mod {ADULTE}");
    let coupes = || crate::eden::lire_coupures(&std::fs::read_to_string(config.join("qt-config.ini")).unwrap()).into_iter().find(|(b, _)| *b == BASE).map(|(_, l)| l).unwrap_or_default();
    // A (admin) lance : les deux mods sont posés ; dans Eden (commun), le mod réservé a un nom NEUTRE.
    let b = poser_eden_pour(&donnees, &eden, BASE, &stock, None, false, Some(&admin)).unwrap();
    assert_eq!(b.poses.len(), 2, "{b:?}");
    assert!(load.join(&neutre).join("romfs").join("b.bin").is_file());
    assert!(!load.join("Nom Secret").exists(), "jamais son vrai nom dans Eden");
    assert!(b.poses.contains(&MOD_RESERVE.to_string()) && !b.resume().contains("Secret"), "même pour l'admin, jamais nommé au journal");
    assert_eq!(verifier_masques_eden(&eden, BASE, &stock, Some(&admin)), None);
    // B lance : le mod réservé est COUPÉ (pas recopié, pas retiré), sous son nom neutre ; jamais nommé.
    let b = poser_eden_pour(&donnees, &eden, BASE, &stock, None, false, Some(&autre)).unwrap();
    assert!(load.join(&neutre).join("romfs").join("b.bin").is_file(), "gardé en place pour l'admin");
    assert!(b.poses.is_empty() && b.retires.is_empty(), "{b:?}");
    assert!(coupes().contains(&neutre));
    let journal = b.resume();
    assert!(!journal.contains("Secret") && !journal.contains(ADULTE), "{journal}");
    assert_eq!(verifier_masques_eden(&eden, BASE, &stock, Some(&autre)), None, "coupé : B peut jouer");
    let p = derniere_pose(&stock).unwrap().pour_le_profil(Some(&autre));
    assert!(!serde_json::to_string(&p).unwrap().contains("Secret"));
    // Ryubing : la racine de B ne le reçoit jamais.
    let ryu = d.path().join("Ryubing");
    std::fs::create_dir_all(ryu.join("portable")).unwrap();
    let racine_b = crate::ryubing::racine(&ryu, "Soeur");
    poser_ryubing_pour(&donnees, &ryu, &racine_b, BASE, &stock, None, Some(&autre)).unwrap();
    let tid_b = racine_b.join("mods").join("contents").join("01007ef00011e000");
    assert!(tid_b.join("1080p").exists() && std::fs::read_dir(&tid_b).unwrap().flatten().filter(|e| e.path().is_dir()).count() == 1);
    let mj = std::fs::read_to_string(crate::ryubing::fichier_mods_json(&racine_b, BASE)).unwrap();
    assert!(!mj.contains("Secret") && !mj.contains(ADULTE));
    assert_eq!(verifier_masques_ryubing(&racine_b, BASE, &stock, Some(&autre)), None);
    // Retour de A : rallumé (pas recopié), tel quel.
    let b = poser_eden_pour(&donnees, &eden, BASE, &stock, None, false, Some(&admin)).unwrap();
    assert!(b.poses.is_empty(), "{b:?}");
    assert!(!coupes().contains(&neutre));
    assert_eq!(std::fs::read(load.join(&neutre).join("romfs").join("b.bin")).unwrap(), b"secret");
}

#[test]
fn x2_un_mod_range_sans_fiche_puis_absent_de_la_liste_de_b_est_ecarte() {
    let d = tempfile::tempdir().unwrap();
    let stock = dossier_stock(&d.path().join("Jeu").join("Frogtend"), BASE);
    let a = zip_octets(&[("romfs/a.bin", b"x")], false);
    let f = d.path().join("a.zip");
    std::fs::write(&f, &a).unwrap();
    // Rangé AVANT sa fiche (ou avant la 1.46) : `public` null.
    let mut v = mod_json(CLE, 1, &a);
    v["public"] = Value::Null;
    v["versions"] = json!("toutes");
    ranger(&stock, &f, &ModServi::depuis(&v).unwrap(), None).unwrap();
    let rangs = ranges(&stock);
    let liste_b = lire_reponse(&json!({"mods": []}));
    assert!(visibles(rangs.clone(), Some(&liste_b)).is_empty(), "absent de la liste de B : écarté");
    assert!(visibles(rangs.clone(), None).is_empty());
    assert_eq!(visibles(rangs.clone(), Some(&lire_reponse(&json!({"mods": [v]})))).len(), 1, "présent dans sa liste : gardé");
    let vue = liste_vue(&Lecture { etat: "ok".into(), vue_le: None, liste: liste_b.clone() }, &rangs, &crate::contenus_switch::ChoixJeu::default(), None, BASE);
    assert!(vue.mods.is_empty());
    // Jamais posé pour B.
    let ryu = d.path().join("Ryubing");
    std::fs::create_dir_all(ryu.join("portable")).unwrap();
    let racine = crate::ryubing::racine(&ryu, "B");
    let b = poser_ryubing_pour(&d.path().join("donnees"), &ryu, &racine, BASE, &stock, None, Some(&liste_b)).unwrap();
    assert!(b.poses.is_empty());
    assert!(stock.join(CLE).is_dir(), "jamais effacé");
}

#[test]
fn x3_un_mod_d_un_autre_profil_encore_charge_refuse_le_lancement() {
    let d = tempfile::tempdir().unwrap();
    let donnees = d.path().join("donnees");
    let (stock, admin, autre) = rangement_avec_adulte(d.path());
    let eden = d.path().join("Eden");
    let config = eden.join("user").join("config");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("qt-config.ini"), "[UI]\r\ntheme=dark\r\n").unwrap();
    poser_eden_pour(&donnees, &eden, BASE, &stock, None, false, Some(&admin)).unwrap();
    // Eden ouvert quand B lance : rien n'est changé → le mod réservé reste chargé → refus, message neutre.
    let b = poser_eden_pour(&donnees, &eden, BASE, &stock, None, true, Some(&autre)).unwrap();
    assert!(b.raisons[0].contains("Eden est ouvert"));
    let m = verifier_masques_eden(&eden, BASE, &stock, Some(&autre)).unwrap();
    assert!(m.contains("Un mod d'un autre profil est encore en place dans Eden") && !m.contains("Secret") && !m.contains(ADULTE));
    // Sans qt-config.ini (impossible de couper) : refus aussi, et jamais de fichier créé.
    std::fs::remove_file(config.join("qt-config.ini")).unwrap();
    poser_eden_pour(&donnees, &eden, BASE, &stock, None, false, Some(&autre)).unwrap();
    assert!(!config.join("qt-config.ini").exists());
    assert!(verifier_masques_eden(&eden, BASE, &stock, Some(&autre)).is_some());
    // L'admin, lui, peut jouer.
    assert_eq!(verifier_masques_eden(&eden, BASE, &stock, Some(&admin)), None);
    // Abri impossible chez Ryubing : le mod réservé posé pour l'admin dans SA racine, puis B... (racine de B : rien à
    // couper). Une racine où un mod masqué est posé et pas coupé : refus.
    let ryu = d.path().join("Ryubing");
    std::fs::create_dir_all(ryu.join("portable")).unwrap();
    let racine = crate::ryubing::racine(&ryu, "Partagee");
    poser_ryubing_pour(&donnees, &ryu, &racine, BASE, &stock, None, Some(&admin)).unwrap();
    std::fs::write(ryu.join(".frogtend-sauvegardes"), b"x").unwrap();
    let save = racine.join("bis").join("user").join("save").join("0000000000000001");
    std::fs::create_dir_all(&save).unwrap();
    std::fs::write(save.join("p"), b"p").unwrap();
    let b = poser_ryubing_pour(&donnees, &ryu, &racine, BASE, &stock, None, Some(&autre)).unwrap();
    assert!(b.inchange, "abri impossible : rien ne bouge");
    assert!(verifier_masques_ryubing(&racine, BASE, &stock, Some(&autre)).unwrap().contains("Ryubing"));
}

#[test]
fn x6_les_conflits_se_verifient_sur_tous_les_mods_sans_nommer_un_mod_cache() {
    let d = tempfile::tempdir().unwrap();
    let (stock, _admin, autre) = rangement_avec_adulte(d.path());
    // Le mod pour tous (1080p) ne va pas avec le mod réservé, allumé pour la maison.
    let f = stock.join(format!("{CLE}.json"));
    let mut m: Value = serde_json::from_slice(&std::fs::read(&f).unwrap()).unwrap();
    m["conflits"] = json!([ADULTE]);
    std::fs::write(&f, serde_json::to_vec(&m).unwrap()).unwrap();
    let tous = ranges(&stock);
    let vus: Vec<String> = visibles(tous.clone(), Some(&autre)).iter().map(|m| m.cle.clone()).collect();
    let choix = crate::contenus_switch::ChoixJeu::default();
    let motif = conflit_pour(&tous, &vus, &choix, CLE, None).unwrap();
    assert!(motif.contains("un autre mod allumé sur ce PC") && !motif.contains("Secret"), "{motif}");
    // Pour l'admin, qui le voit : nommé.
    let tous_vus: Vec<String> = tous.iter().map(|m| m.cle.clone()).collect();
    assert!(conflit_pour(&tous, &tous_vus, &choix, CLE, None).unwrap().contains("Nom Secret"));
}

#[test]
fn x7_un_mod_reserve_modifie_et_garde_est_coupe_pour_un_autre_profil_sans_rien_nommer() {
    let d = tempfile::tempdir().unwrap();
    let donnees = d.path().join("donnees");
    let (stock, admin, autre) = rangement_avec_adulte(d.path());
    let eden = d.path().join("Eden");
    let config = eden.join("user").join("config");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("qt-config.ini"), "[UI]\r\ntheme=dark\r\n").unwrap();
    poser_eden_pour(&donnees, &eden, BASE, &stock, None, false, Some(&admin)).unwrap();
    let neutre = format!("Mod {ADULTE}");
    let load = eden.join("user").join("load").join("01007EF00011E000");
    std::fs::write(load.join(&neutre).join("romfs").join("b.bin"), b"retouche").unwrap();
    let b = poser_eden_pour(&donnees, &eden, BASE, &stock, None, false, Some(&autre)).unwrap();
    let texte = std::fs::read_to_string(config.join("qt-config.ini")).unwrap();
    assert!(crate::eden::lire_coupures(&texte).iter().any(|(x, l)| *x == BASE && l.contains(&neutre)));
    assert!(load.join(&neutre).join("romfs").join("b.bin").is_file(), "jamais effacé");
    assert!(!b.resume().contains("Secret") && b.raisons.iter().all(|r| !r.contains("Secret")));
    assert_eq!(verifier_masques_eden(&eden, BASE, &stock, Some(&autre)), None);
}

#[test]
fn c146_les_raisons_qui_nomment_un_mod_reserve_ne_vont_qu_au_profil_qui_le_voit() {
    let d = tempfile::tempdir().unwrap();
    let donnees = d.path().join("donnees");
    let (stock, admin, autre) = rangement_avec_adulte(d.path());
    // Le mod adulte demande un réglage : la raison le nomme → gardée à part.
    let f = stock.join(format!("{ADULTE}.json"));
    let mut m: Value = serde_json::from_slice(&std::fs::read(&f).unwrap()).unwrap();
    m["reglage_requis"] = json!("60 Hz");
    std::fs::write(&f, serde_json::to_vec(&m).unwrap()).unwrap();
    let eden = d.path().join("Eden");
    std::fs::create_dir_all(eden.join("user")).unwrap();
    let b = poser_eden_pour(&donnees, &eden, BASE, &stock, None, false, Some(&admin)).unwrap();
    assert!(b.raisons.iter().all(|r| !r.contains("Secret")));
    assert!(b.reservees.iter().any(|(c, r)| c == ADULTE && r.contains("Nom Secret")));
    let brut = derniere_pose(&stock).unwrap();
    assert!(brut.raisons.iter().all(|r| !r.contains("Secret")));
    assert!(brut.clone().pour_le_profil(Some(&admin)).raisons.iter().any(|r| r.contains("Nom Secret")), "l'admin la lit");
    let pour_b = brut.pour_le_profil(Some(&autre));
    assert!(pour_b.raisons.iter().all(|r| !r.contains("Secret")) && pour_b.reservees.is_empty());
}

// --- Dernière relecture (contrat 1.46) ---

#[test]
fn y1_un_cache_perime_ne_laisse_jamais_passer_un_mod_devenu_adulte() {
    let d = tempfile::tempdir().unwrap();
    let (stock, admin, _autre) = rangement_avec_adulte(d.path());
    let rangs = ranges(&stock);
    // Le cache de l'enfant date d'AVANT la classification de la nuit : le mod y est, avec `null` ou « tous ».
    let b = admin.mods.iter().find(|m| m.cle == ADULTE).unwrap().clone();
    for public in [None, Some("tous".to_string()), Some("ado".to_string())] {
        let mut ancien = b.clone();
        ancien.public = public.clone();
        let cache = ListeMods { mods: vec![admin.mods[0].clone(), ancien], ..Default::default() };
        assert!(!visibles(rangs.clone(), Some(&cache)).iter().any(|m| m.cle == ADULTE), "{public:?}");
    }
    // La liste de l'admin le dit « adulte » : visible.
    assert!(visibles(rangs.clone(), Some(&admin)).iter().any(|m| m.cle == ADULTE));
    // Reclassé « tous » par Firehouse, mais déjà vu « adulte » : seule une entrée « adulte » le montre encore.
    let f = stock.join(format!("{ADULTE}.json"));
    let mut m: Value = serde_json::from_slice(&std::fs::read(&f).unwrap()).unwrap();
    m["public"] = json!("tous");
    std::fs::write(&f, serde_json::to_vec(&m).unwrap()).unwrap();
    let rangs = ranges(&stock);
    assert!(!visibles(rangs.clone(), None).iter().any(|m| m.cle == ADULTE));
    // Un mod JAMAIS « adulte », sans fiche, présent dans le cache avec `null` : visible (il a été servi à ce profil).
    let a = zip_octets(&[("romfs/c.bin", b"c")], false);
    let fc = d.path().join("c.zip");
    std::fs::write(&fc, &a).unwrap();
    let mut sans = mod_json("sansfiche-0000000000cc", 1, &a);
    sans["public"] = Value::Null;
    ranger(&stock, &fc, &ModServi::depuis(&sans).unwrap(), None).unwrap();
    let cache = lire_reponse(&json!({"mods": [sans]}));
    assert!(visibles(ranges(&stock), Some(&cache)).iter().any(|m| m.cle == "sansfiche-0000000000cc"));
}

#[test]
fn y2_un_ancien_dossier_au_vrai_nom_d_un_mod_reserve_est_renomme_dans_eden() {
    let d = tempfile::tempdir().unwrap();
    let donnees = d.path().join("donnees");
    let stock = dossier_stock(&d.path().join("Jeu").join("Frogtend"), BASE);
    let a = zip_octets(&[("romfs/a.bin", b"un")], false);
    let fa = d.path().join("a.zip");
    std::fs::write(&fa, &a).unwrap();
    let mut v = mod_json(CLE, 1, &a);
    v["versions"] = json!("toutes");
    ranger(&stock, &fa, &ModServi::depuis(&v).unwrap(), None).unwrap();
    let eden = d.path().join("Eden");
    let config = eden.join("user").join("config");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("qt-config.ini"), "[UI]\r\ntheme=dark\r\n").unwrap();
    let load = eden.join("user").join("load").join("01007EF00011E000");
    // Posé « pour tous » sous son vrai nom (comme le 1080p de Seb avant cette version).
    poser_eden(&donnees, &eden, BASE, &stock, None, false).unwrap();
    assert!(load.join("1080p").is_dir());
    // Sa fiche est réécrite : `null` → réservé. Le profil qui le voit (sa liste le contient) lance.
    let f = stock.join(format!("{CLE}.json"));
    let mut m: Value = serde_json::from_slice(&std::fs::read(&f).unwrap()).unwrap();
    m["public"] = Value::Null;
    std::fs::write(&f, serde_json::to_vec(&m).unwrap()).unwrap();
    let mut entree = v.clone();
    entree["public"] = Value::Null;
    let sa_liste = lire_reponse(&json!({"mods": [entree]}));
    let abris = || std::fs::read_dir(eden.join(".frogtend-sauvegardes")).map(|l| l.count()).unwrap_or(0);
    let save = eden.join("user").join("nand").join("user").join("save").join("0000000000000000");
    std::fs::create_dir_all(&save).unwrap();
    let b = poser_eden_pour(&donnees, &eden, BASE, &stock, None, false, Some(&sa_liste)).unwrap();
    let neutre = nom_neutre(CLE);
    assert!(load.join(&neutre).join("romfs").join("a.bin").is_file() && !load.join("1080p").exists(), "{b:?}");
    assert!(b.abrite, "l'abri des parties se fait avant le renommage");
    let note = lire_note(&load).unwrap();
    assert_eq!(note[CLE].dossier, neutre);
    let _ = abris;
    // Une coupure ancienne au vrai nom (posée pour un autre profil) part aussi.
    let texte = std::fs::read_to_string(config.join("qt-config.ini")).unwrap();
    assert!(!crate::eden::lire_coupures(&texte).iter().any(|(_, l)| l.iter().any(|n| n == "1080p")));
    // Pour un profil qui ne le voit pas : coupé sous le nom neutre, jamais sous son vrai nom.
    poser_eden_pour(&donnees, &eden, BASE, &stock, None, false, None).unwrap();
    let texte = std::fs::read_to_string(config.join("qt-config.ini")).unwrap();
    let coupes = crate::eden::lire_coupures(&texte).into_iter().find(|(x, _)| *x == BASE).map(|(_, l)| l).unwrap_or_default();
    assert!(coupes.contains(&neutre) && !coupes.iter().any(|n| n == "1080p"), "{coupes:?}");
}

#[test]
fn y2_masque_et_au_vrai_nom_il_est_renomme_puis_coupe() {
    let d = tempfile::tempdir().unwrap();
    let donnees = d.path().join("donnees");
    let stock = dossier_stock(&d.path().join("Jeu").join("Frogtend"), BASE);
    let a = zip_octets(&[("romfs/a.bin", b"un")], false);
    let fa = d.path().join("a.zip");
    std::fs::write(&fa, &a).unwrap();
    let mut v = mod_json(CLE, 1, &a);
    v["versions"] = json!("toutes");
    ranger(&stock, &fa, &ModServi::depuis(&v).unwrap(), None).unwrap();
    let eden = d.path().join("Eden");
    let config = eden.join("user").join("config");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("qt-config.ini"), "[UI]\r\ntheme=dark\r\n").unwrap();
    poser_eden(&donnees, &eden, BASE, &stock, None, false).unwrap();
    // Classé « adulte » : pour un profil sans liste, renommé (intact) puis coupé, et B peut jouer.
    let f = stock.join(format!("{CLE}.json"));
    let mut m: Value = serde_json::from_slice(&std::fs::read(&f).unwrap()).unwrap();
    m["public"] = json!("adulte");
    m["adulte_vu"] = json!(true);
    std::fs::write(&f, serde_json::to_vec(&m).unwrap()).unwrap();
    let b = poser_eden_pour(&donnees, &eden, BASE, &stock, None, false, None).unwrap();
    let load = eden.join("user").join("load").join("01007EF00011E000");
    assert!(load.join(nom_neutre(CLE)).is_dir() && !load.join("1080p").exists(), "{b:?}");
    assert!(!b.resume().contains("1080p"));
    assert_eq!(verifier_masques_eden(&eden, BASE, &stock, None), None);
    let texte = std::fs::read_to_string(config.join("qt-config.ini")).unwrap();
    assert!(crate::eden::lire_coupures(&texte).iter().any(|(x, l)| *x == BASE && l.contains(&nom_neutre(CLE))));
}

#[test]
fn y5_les_raisons_d_un_mod_deja_vu_adulte_sont_tues_et_une_note_illisible_se_dit_simplement() {
    let d = tempfile::tempdir().unwrap();
    let (stock, admin, _autre) = rangement_avec_adulte(d.path());
    // Reclassé « tous » mais déjà vu « adulte », avec un réglage requis : sa raison va à part (jamais au journal).
    let f = stock.join(format!("{ADULTE}.json"));
    let mut m: Value = serde_json::from_slice(&std::fs::read(&f).unwrap()).unwrap();
    m["public"] = json!("tous");
    m["reglage_requis"] = json!("60 Hz");
    std::fs::write(&f, serde_json::to_vec(&m).unwrap()).unwrap();
    let (mut raisons, mut reservees) = (Vec::new(), Vec::new());
    let v = voulus_pour(&stock, &crate::contenus_switch::ChoixJeu::default(), None, Some(&admin), &mut raisons, &mut reservees);
    assert!(raisons.iter().all(|r| !r.contains("Secret")));
    assert!(reservees.iter().any(|(c, r)| c == ADULTE && r.contains("Nom Secret")));
    assert!(v.iter().find(|x| x.m.cle == ADULTE).unwrap().reserve);
    // Une note illisible alors qu'un mod est masqué : la partie ne part pas, avec un message simple.
    let eden = d.path().join("Eden");
    let load = eden.join("user").join("load").join("01007EF00011E000");
    std::fs::create_dir_all(&load).unwrap();
    std::fs::write(load.join(NOTE_POSE), b"{abime").unwrap();
    let msg = verifier_masques_eden(&eden, BASE, &stock, None).unwrap();
    assert!(msg.starts_with("Frogtend ne sait plus quels mods sont en place pour ce jeu"), "{msg}");
}
