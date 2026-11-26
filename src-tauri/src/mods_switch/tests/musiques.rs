//! Les tests des musiques de Mario Kart 8 Deluxe (contrat 1.55). Firehouse est simulé (httpmock), jamais appelé pour de
//! vrai ; tout se passe dans des dossiers temporaires (jamais les vraies données de Seb). Les formes viennent de
//! l'exemple réel du contrat (26/11 : DK Summit, version servie 3.0.1 = 1179648), les octets sont imaginaires.

use super::*;

/// DK Summit (exemple réel du contrat : clé, amorces).
const DK: &str = "gamebanana-ee3c0683c3f6";
/// Un autre mod de musique, pour tous.
const AUTRE: &str = "gamebanana-0123456789ab";
/// Un mod de musique RÉSERVÉ (« adulte »), reçu par un seul profil.
const SECRET: &str = "gamebanana-5ec2e7000000";
/// La version servie de MK8D (3.0.1).
const V301: u32 = 1179648;

fn version_servie() -> VersionServie {
    VersionServie { version: Some(1179648), version_affichee: Some("3.0.1".into()), build_id: Some("9EF5CAA2D5B933C772358C5AA6FABA15".into()) }
}

fn mod_musique(cle: &str, nom: &str, revision: u64, archive: &[u8], amorces: &[&str], public: &str) -> Value {
    let mut v = mod_json(cle, revision, archive);
    v["nom"] = json!(nom);
    v["versions"] = json!("toutes");
    v["amorces"] = json!(amorces);
    v["public"] = json!(public);
    v
}

/// Range un mod (archive écrite à côté).
fn ranger_json(d: &Path, stock: &Path, v: &Value, archive: &[u8]) {
    let f = d.join(format!("{}.zip", v["cle"].as_str().unwrap()));
    std::fs::write(&f, archive).unwrap();
    ranger(stock, &f, &ModServi::depuis(v).unwrap(), Some(&version_servie())).unwrap();
}

/// Le décor : un rangement MK8D (DK Summit, avec son PROPRE BGM.bars dans l'archive ; un mod de musique réservé ; un mod
/// sans musique), une racine Ryubing portable avec une partie, et les listes de deux profils (admin et autre).
struct Decor {
    _d: tempfile::TempDir,
    donnees: PathBuf,
    stock: PathBuf,
    ryu: PathBuf,
    racine: PathBuf,
    admin: ListeMods,
    autre: ListeMods,
}

fn decor() -> Decor {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().to_path_buf();
    let stock = dossier_stock(&p.join("Jeu").join("Frogtend"), BASE);
    let a_dk = zip_octets(&[("DK Summit/romfs/Course/a.bin", b"circuit"), ("DK Summit/romfs/Audio/Static/BGM.bars", b"VIEUX-BGM-DU-MOD")], false);
    let a_secret = zip_octets(&[("romfs/UI/s.bin", b"secret")], false);
    // Un mod SANS musique qui porte quand même un BGM.bars (fait pour une autre version) : sauté tant que le fichier
    // commun est en place, posé sinon.
    let a_sans = zip_octets(&[("exefs/x.ips", b"IPS"), ("romfs/Audio/Static/BGM.bars", b"BGM-DU-1080P")], false);
    let dk = mod_musique(DK, "DK Summit", 2, &a_dk, &["pBGM_GC_SHERBET_LAND_F", "pBGM_GC_SHERBET_LAND_N"], "tous");
    let secret = mod_musique(SECRET, "Nom Secret", 1, &a_secret, &["pBGM_SECRET"], "adulte");
    let mut sans = mod_musique(CLE, "1080p", 1, &a_sans, &[], "tous");
    sans["versions"] = json!("toutes");
    ranger_json(&p, &stock, &dk, &a_dk);
    ranger_json(&p, &stock, &secret, &a_secret);
    ranger_json(&p, &stock, &sans, &a_sans);
    let ryu = p.join("Ryubing");
    std::fs::create_dir_all(ryu.join("portable")).unwrap();
    let racine = crate::ryubing::racine(&ryu, "Seb");
    let save = racine.join("bis").join("user").join("save").join("0000000000000001").join("0");
    std::fs::create_dir_all(&save).unwrap();
    std::fs::write(save.join("partie.sav"), b"sacree").unwrap();
    let sv = json!({"version": 1179648, "version_affichee": "3.0.1", "build_id": "9EF5CAA2D5B933C772358C5AA6FABA15"});
    let admin = lire_reponse(&json!({"version_servie": sv, "mods": [dk.clone(), secret, sans.clone()]}));
    let autre = lire_reponse(&json!({"version_servie": sv, "mods": [dk, sans]}));
    // La personne a dit OUI aux musiques en allumant ses mods de musique (la taille lui a été annoncée).
    noter_accord_musiques(&stock, &vec![(SECRET.into(), 1), (DK.into(), 2)], Some(1179648), Some(41018496)).unwrap();
    Decor { donnees: p.join("donnees"), stock, ryu, racine, admin, autre, _d: d }
}

impl Decor {
    fn dossier_jeu(&self) -> PathBuf {
        dossier_mods_ryubing(&self.racine, BASE)
    }

    fn bgm_pose(&self) -> PathBuf {
        self.dossier_jeu().join(DOSSIER_MUSIQUES).join("romfs").join("Audio").join("Static").join("BGM.bars")
    }

    /// Un lancement complet chez Ryubing : la décision (HEAD…), puis la pose.
    async fn lancer(&self, source: Option<&Source>, liste: Option<&ListeMods>, utilisee: Option<u32>, auto: bool) -> (DecisionMusiques, BilanMods) {
        let (ensemble, a_taire, servie) = ensemble_du_profil(&self.donnees, &self.stock, BASE, utilisee, liste);
        let dj = self.dossier_jeu();
        let dec = resoudre_musiques(DemandeMusiques { source, stock: &self.stock, dossier_jeu: &dj, base: BASE, ensemble, a_taire, utilisee, servie, auto, budget: BUDGET_MUSIQUES_LANCEMENT }).await;
        let b = poser_ryubing_avec(&self.donnees, &self.ryu, &self.racine, BASE, &self.stock, utilisee, liste, &dec).unwrap();
        (dec, b)
    }

    fn mods_json(&self) -> Value {
        serde_json::from_slice(&std::fs::read(crate::ryubing::fichier_mods_json(&self.racine, BASE)).unwrap()).unwrap()
    }
}

fn route_tete() -> String {
    format!("/api/jeux/v1/switch/{BASE:016X}/bgm-bars")
}

/// Firehouse sert ce BGM.bars pour `mods` (HEAD et GET) ; rend les deux simulations.
fn servir<'a>(m: &'a MockServer, mods: &str, bgm: &[u8], version: u64) -> (httpmock::Mock<'a>, httpmock::Mock<'a>) {
    let s = sha(bgm);
    let tete = m.mock(|w, r| {
        w.method(httpmock::Method::HEAD).path(route_tete()).query_param("mods", mods).header("authorization", "Bearer jeton");
        r.status(200).header("X-Sha256", &s).header("X-Version-Jeu", version.to_string()).header("ETag", format!("\"{s}\"")).header("content-length", bgm.len().to_string());
    });
    let corps = m.mock(|w, r| {
        w.method(GET).path(route_tete()).query_param("mods", mods).header("authorization", "Bearer jeton");
        r.status(200).header("X-Sha256", &s).header("X-Version-Jeu", version.to_string()).body(bgm);
    });
    (tete, corps)
}

fn injoignable() -> Source {
    Source::Firehouse(crate::firehouse::Client::nouveau("http://127.0.0.1:9", "jeton").unwrap())
}

#[test]
fn m155_les_amorces_se_lisent_avec_tolerance_et_suivent_le_manifeste_et_la_vue() {
    let a = zip_octets(&[("romfs/a.bin", b"un")], false);
    let mut v = mod_json(DK, 2, &a);
    v["amorces"] = json!(["pBGM_B", " pBGM_A ", "pBGM_B", "", 3]);
    assert_eq!(ModServi::depuis(&v).unwrap().amorces, vec!["pBGM_A".to_string(), "pBGM_B".to_string()]);
    assert!(ModServi::depuis(&v).unwrap().musique());
    v["amorces"] = json!("pBGM_A");
    assert!(ModServi::depuis(&v).unwrap().amorces.is_empty(), "une autre forme : pas un mod de musique");
    v.as_object_mut().unwrap().remove("amorces");
    assert!(!ModServi::depuis(&v).unwrap().musique(), "absent : []");
    // Rangé AVANT la 1.55 (sans amorces), puis la liste les donne : le manifeste les retient (même révision).
    let d = tempfile::tempdir().unwrap();
    let stock = d.path().join("stock");
    ranger_json(d.path(), &stock, &mod_json(DK, 2, &a), &a);
    assert!(ranges(&stock)[0].amorces.is_empty());
    let mut avec = mod_json(DK, 2, &a);
    avec["amorces"] = json!(["pBGM_GC_SHERBET_LAND_F", "pBGM_GC_SHERBET_LAND_N"]);
    let liste = lire_reponse(&json!({"mods": [avec]}));
    assert_eq!(rafraichir(&stock, &liste), 1);
    assert_eq!(ranges(&stock)[0].amorces, vec!["pBGM_GC_SHERBET_LAND_F".to_string(), "pBGM_GC_SHERBET_LAND_N".to_string()]);
    // La vue les porte (pour voir un chevauchement à l'allumage).
    let lecture = Lecture { etat: "ok".into(), vue_le: None, liste: liste.clone() };
    let vue = liste_vue(&lecture, &ranges(&stock), &crate::contenus_switch::ChoixJeu::default(), Some(V301), BASE);
    assert_eq!(vue.mods[0].amorces.len(), 2);
    // Le cache des musiques vit dans le rangement, mais n'est jamais pris pour un mod.
    std::fs::create_dir_all(dossier_cache_musiques(&stock)).unwrap();
    std::fs::write(dossier_cache_musiques(&stock).join("BGM-0123456789ab.bars"), b"x").unwrap();
    std::fs::write(dossier_cache_musiques(&stock).join("faux.json"), b"{}").unwrap();
    assert_eq!(ranges(&stock).iter().map(|m| m.cle.as_str()).collect::<Vec<_>>(), vec![DK]);
}

#[test]
fn m155_une_archive_sans_rien_d_utile_est_acceptee_seulement_pour_un_mod_de_musique() {
    let d = tempfile::tempdir().unwrap();
    let stock = d.path().join("stock");
    let a = zip_octets(&[("lisez-moi.txt", b"seulement les amorces")], false);
    let f = d.path().join("a.zip");
    std::fs::write(&f, &a).unwrap();
    assert!(ranger(&stock, &f, &servi(CLE, 1, &a), None).is_err(), "un mod ordinaire vide est refusé");
    let m = ModServi::depuis(&mod_musique(DK, "DK", 1, &a, &["pBGM_X"], "tous")).unwrap();
    let r = ranger(&stock, &f, &m, None).unwrap();
    assert!(r.fichiers.is_empty() && stock.join(DK).is_dir());
    assert_eq!(ranges(&stock).len(), 1);
}

#[test]
fn m155_la_liste_active_est_celle_du_profil_qui_lance() {
    let x = decor();
    // Le profil admin voit le mod réservé : il compte, et il est à taire.
    let (e, a_taire, servie) = ensemble_du_profil(&x.donnees, &x.stock, BASE, Some(V301), Some(&x.admin));
    assert_eq!(e, vec![(SECRET.to_string(), 1), (DK.to_string(), 2)], "triées par clé");
    assert_eq!(a_taire, vec![SECRET.to_string()]);
    assert_eq!(servie.and_then(|s| s.version), Some(1179648));
    // L'autre profil : jamais le mod réservé d'un autre (ni le mod sans musique).
    let (e, a_taire, _) = ensemble_du_profil(&x.donnees, &x.stock, BASE, Some(V301), Some(&x.autre));
    assert_eq!(e, vec![(DK.to_string(), 2)]);
    assert!(a_taire.is_empty());
    // Un mod de musique éteint n'en fait plus partie.
    choisir_mods(&x.donnees, BASE, &[(DK.into(), false)]).unwrap();
    let (e, _, _) = ensemble_du_profil(&x.donnees, &x.stock, BASE, Some(V301), Some(&x.autre));
    assert!(e.is_empty());
    assert_eq!(route_bgm_bars(BASE, &vec![(SECRET.into(), 1), (DK.into(), 2)]), format!("/switch/{BASE:016X}/bgm-bars?mods={SECRET},{DK}"));
}

#[test]
fn m155_la_regle_de_version_est_stricte() {
    let sv = version_servie();
    assert_eq!(raison_version_musiques(Some(&sv), None, Some(V301)), None, "égale : posé");
    assert_eq!(raison_version_musiques(Some(&sv), Some(1179648), Some(V301)), None);
    let r = raison_version_musiques(Some(&sv), Some(1179648), Some(1114112)).unwrap();
    assert!(r.contains("faites pour la version 3.0.1") && r.contains("musiques d'origine"), "{r}");
    let r = raison_version_musiques(Some(&sv), None, Some(0)).unwrap();
    assert!(r.contains("la version d'origine (sans mise à jour)"), "{r}");
    let r = raison_version_musiques(Some(&sv), None, None).unwrap();
    assert!(r.contains("pas connue"), "inconnue : pas de musique de mod ({r})");
    assert!(raison_version_musiques(Some(&sv), Some(1114112), Some(1114112)).is_some(), "X-Version-Jeu doit aussi égaler la version servie");
    assert_eq!(raison_version_musiques(None, None, Some(V301)), None, "rien de connu : le HEAD jugera");
}

#[tokio::test]
async fn m155_une_autre_version_ne_demande_rien_a_firehouse_et_retire_le_fichier() {
    let x = decor();
    let m = MockServer::start();
    let bgm = b"BGM-FABRIQUE-3.0.1".to_vec();
    let (tete, _) = servir(&m, DK, &bgm, 1179648);
    let src = source(&m);
    let (dec, b) = x.lancer(Some(&src), Some(&x.autre), Some(V301), false).await;
    assert!(matches!(dec.action, Musiques::Poser { .. }), "{dec:?}");
    assert!(x.bgm_pose().is_file() && b.musiques.as_ref().is_some_and(|l| l.en_place));
    // La personne coupe la mise à jour (version d'origine) : aucune question à Firehouse, le fichier commun part.
    let (dec, b) = x.lancer(Some(&src), Some(&x.autre), Some(0), false).await;
    tete.assert_hits(1);
    assert!(matches!(&dec.action, Musiques::Retirer { raison: Some(r) } if r.contains("3.0.1")), "{dec:?}");
    assert!(!x.dossier_jeu().join(DOSSIER_MUSIQUES).exists());
    let l = b.musiques.unwrap();
    assert!(!l.en_place && l.raison.unwrap().contains("musiques d'origine"));
    // Le fichier dit fait pour une autre version que celle utilisée (X-Version-Jeu) : rien de posé.
    let m2 = MockServer::start();
    servir(&m2, DK, &bgm, 1114112);
    let (dec, _) = x.lancer(Some(&source(&m2)), Some(&x.autre), Some(V301), false).await;
    assert!(matches!(dec.action, Musiques::Retirer { raison: Some(_) }), "{dec:?}");
    assert!(!x.bgm_pose().exists());
}

#[tokio::test]
async fn m155_meme_sha_rien_a_faire_autre_sha_retelecharge_et_met_les_parties_a_l_abri() {
    let x = decor();
    let m = MockServer::start();
    let bgm = b"BGM-FABRIQUE-DK-r2".to_vec();
    let (tete, corps) = servir(&m, DK, &bgm, 1179648);
    let src = source(&m);
    let (_, b) = x.lancer(Some(&src), Some(&x.autre), Some(V301), false).await;
    corps.assert_hits(1);
    assert_eq!(std::fs::read(x.bgm_pose()).unwrap(), bgm);
    assert!(b.abrite, "un changement du BGM.bars met les parties à l'abri");
    let abris_avant = abris(&x.ryu);
    assert!(dossier_cache_musiques(&x.stock).join(nom_cache_bgm(&sha(&bgm))).is_file(), "gardé dans le cache");
    let n = note_musiques(&x.dossier_jeu()).unwrap();
    assert_eq!((n.cles.clone(), n.version, n.sha256.clone(), n.taille), (vec![(DK.to_string(), 2)], 1179648, sha(&bgm), bgm.len() as u64));
    // mods.json : le dossier commun, allumé, avec le chemin complet exact.
    let mj = x.mods_json();
    let chemin = x.dossier_jeu().join(DOSSIER_MUSIQUES).to_string_lossy().to_string();
    assert!(mj["mods"].as_array().unwrap().iter().any(|e| e["name"] == DOSSIER_MUSIQUES && e["path"] == chemin.as_str() && e["enabled"] == true), "{mj}");
    // Même sha au lancement suivant : rien n'est retéléchargé ni changé, pas de nouvel abri.
    let (_, b) = x.lancer(Some(&src), Some(&x.autre), Some(V301), false).await;
    corps.assert_hits(1);
    tete.assert_hits(2);
    assert!(!b.abrite && b.musiques.as_ref().is_some_and(|l| l.en_place && l.fait == "gardé"), "{b:?}");
    assert_eq!(abris(&x.ryu), abris_avant);
    // Firehouse refait le fichier (une révision a monté chez lui) : autre sha → téléchargé, remplacé, abri d'abord.
    let m2 = MockServer::start();
    let bgm2 = b"BGM-FABRIQUE-DK-r3-plus-long".to_vec();
    let (_, corps2) = servir(&m2, DK, &bgm2, 1179648);
    // (La partie a avancé entre-temps : un abri identique au précédent n'est pas recopié.)
    std::fs::write(x.racine.join("bis").join("user").join("save").join("0000000000000001").join("0").join("partie.sav"), b"sacree, plus loin").unwrap();
    let (_, b) = x.lancer(Some(&source(&m2)), Some(&x.autre), Some(V301), false).await;
    corps2.assert_hits(1);
    assert_eq!(std::fs::read(x.bgm_pose()).unwrap(), bgm2);
    assert!(b.abrite && b.musiques.as_ref().is_some_and(|l| l.fait == "remplacé"), "{b:?}");
    assert_eq!(abris(&x.ryu), abris_avant + 1);
    assert_eq!(note_musiques(&x.dossier_jeu()).unwrap().sha256, sha(&bgm2));
    assert!(derniere_pose(&x.stock).unwrap().musiques.is_some_and(|l| l.en_place && l.actives == 1));
}

#[tokio::test]
async fn m155_un_fichier_deja_dans_le_cache_n_est_pas_retelecharge_et_une_nouvelle_version_suit_le_reglage_auto() {
    let x = decor();
    let m = MockServer::start();
    let bgm = b"BGM-3.0.1".to_vec();
    let (_, corps) = servir(&m, DK, &bgm, 1179648);
    // Préchargé à l'allumage (hors partie) : dans le cache, sans rien toucher chez l'émulateur.
    let e = vec![(DK.to_string(), 2)];
    assert_eq!(precharger_musiques(&source(&m), &x.stock, BASE, &e, Some(V301), Some(&version_servie())).await.unwrap(), Some(bgm.len() as u64));
    corps.assert_hits(1);
    assert!(!x.dossier_jeu().exists());
    assert_eq!(taille_musiques(Some(&source(&m)), &x.stock, BASE, &e).await, Some(bgm.len() as u64));
    assert_eq!(taille_musiques(Some(&injoignable()), &x.stock, BASE, &e).await, Some(bgm.len() as u64), "à défaut : le dernier connu");
    let (dec, _) = x.lancer(Some(&source(&m)), Some(&x.autre), Some(V301), false).await;
    corps.assert_hits(1);
    assert!(dec.journal.iter().any(|j| j == "pris dans le cache"), "{dec:?}");
    assert!(x.bgm_pose().is_file());
    // Firehouse sert une NOUVELLE version (3.0.2) et la personne joue avec : la refabrication suit « tout seul ».
    let mut sv = version_servie();
    sv.version = Some(1245184);
    sv.version_affichee = Some("3.0.2".into());
    let m2 = MockServer::start();
    let bgm2 = b"BGM-3.0.2".to_vec();
    let (_, corps2) = servir(&m2, DK, &bgm2, 1245184);
    let dj = x.dossier_jeu();
    let demande = |auto: bool| DemandeMusiques { source: None, stock: &x.stock, dossier_jeu: &dj, base: BASE, ensemble: e.clone(), a_taire: vec![], utilisee: Some(1245184), servie: Some(sv.clone()), auto, budget: BUDGET_MUSIQUES_LANCEMENT };
    let src2 = source(&m2);
    let dec = resoudre_musiques(DemandeMusiques { source: Some(&src2), ..demande(false) }).await;
    corps2.assert_hits(0);
    // L'accord noté était pour la 3.0.1 : rien n'est téléchargé sans un nouveau oui (bouton de la fiche).
    assert!(matches!(&dec.action, Musiques::Retirer { raison: Some(r) } if r.contains("musiques à préparer (≈ 9 octets)") && r.contains("Préparer les musiques")), "{dec:?}");
    let dec = resoudre_musiques(DemandeMusiques { source: Some(&src2), ..demande(true) }).await;
    corps2.assert_hits(1);
    assert!(matches!(&dec.action, Musiques::Poser { version: 1245184, .. }), "{dec:?}");
}

#[tokio::test]
async fn m155_les_codes_d_erreur_de_firehouse() {
    let x = decor();
    let m = MockServer::start();
    let bgm = b"BGM-DK".to_vec();
    servir(&m, DK, &bgm, 1179648);
    x.lancer(Some(&source(&m)), Some(&x.autre), Some(V301), false).await;
    assert!(x.bgm_pose().is_file());
    let dj = x.dossier_jeu();
    let code = |statut: u16, code: &'static str| {
        let m = MockServer::start();
        m.mock(|w, r| {
            w.method(httpmock::Method::HEAD).path(route_tete());
            r.status(statut).header("X-Code", code);
        });
        m
    };
    let resoudre = |s: MockServer| {
        let dj = dj.clone();
        let stock = x.stock.clone();
        let (ensemble, a_taire, servie) = ensemble_du_profil(&x.donnees, &x.stock, BASE, Some(V301), Some(&x.autre));
        async move {
            let src = source(&s);
            resoudre_musiques(DemandeMusiques { source: Some(&src), stock: &stock, dossier_jeu: &dj, base: BASE, ensemble, a_taire, utilisee: Some(V301), servie, auto: false, budget: BUDGET_MUSIQUES_LANCEMENT }).await
        }
    };
    // original_absent : l'existant est GARDÉ (et dit).
    let dec = resoudre(code(409, "original_absent")).await;
    assert_eq!(dec.action, Musiques::Garder { raison: Some(RAISON_MUSIQUES_ORIGINAL_ABSENT.into()), maj: None });
    // introuvable : rien de posé (retiré).
    let dec = resoudre(code(404, "introuvable")).await;
    assert_eq!(dec.action, Musiques::Retirer { raison: Some(RAISON_MUSIQUES_INTROUVABLE.into()) });
    // conflit : rien de posé ; le lancement continue sans musique de mod (le choix se fait à l'allumage).
    let dec = resoudre(code(409, "conflit")).await;
    assert_eq!(dec.action, Musiques::Retirer { raison: Some(RAISON_MUSIQUES_CONFLIT.into()) });
    let b = poser_ryubing_avec(&x.donnees, &x.ryu, &x.racine, BASE, &x.stock, Some(V301), Some(&x.autre), &dec).unwrap();
    assert!(!x.dossier_jeu().join(DOSSIER_MUSIQUES).exists());
    assert!(b.musiques.unwrap().raison.unwrap().contains("même musique"));
    let mj = x.mods_json();
    assert!(!mj["mods"].as_array().unwrap().iter().any(|e| e["name"] == DOSSIER_MUSIQUES), "son entrée part de mods.json : {mj}");
    // original_absent sans rien de posé : rien.
    let dec = resoudre(code(409, "original_absent")).await;
    assert_eq!(dec.action, Musiques::Retirer { raison: Some(RAISON_MUSIQUES_ORIGINAL_ABSENT_RIEN.into()) });
}

#[tokio::test]
async fn m155_hors_ligne_le_meme_ensemble_est_garde_un_autre_est_retire() {
    let x = decor();
    let m = MockServer::start();
    servir(&m, DK, b"BGM-DK", 1179648);
    x.lancer(Some(&source(&m)), Some(&x.autre), Some(V301), false).await;
    assert!(x.bgm_pose().is_file());
    // Firehouse injoignable, même ensemble (clés, révisions, version) : gardé.
    let (dec, b) = x.lancer(Some(&injoignable()), Some(&x.autre), Some(V301), false).await;
    assert_eq!(dec.action, Musiques::Garder { raison: None, maj: None });
    assert!(x.bgm_pose().is_file() && b.musiques.is_some_and(|l| l.en_place));
    // Sans session non plus : comme hors ligne.
    let (dec, _) = x.lancer(None, Some(&x.autre), Some(V301), false).await;
    assert_eq!(dec.action, Musiques::Garder { raison: None, maj: None });
    // L'ensemble change (le profil admin a aussi le mod réservé) : retiré, et dit.
    let (dec, b) = x.lancer(Some(&injoignable()), Some(&x.admin), Some(V301), false).await;
    assert_eq!(dec.action, Musiques::Retirer { raison: Some(RAISON_MUSIQUES_HORS_LIGNE.into()) });
    assert!(!x.dossier_jeu().join(DOSSIER_MUSIQUES).exists());
    assert!(b.musiques.unwrap().raison.unwrap().contains("Firehouse ne répond pas"));
}

#[tokio::test]
async fn m155_plus_aucun_mod_de_musique_le_fichier_commun_part_sans_rien_demander() {
    let x = decor();
    let m = MockServer::start();
    let (tete, _) = servir(&m, DK, b"BGM-DK", 1179648);
    x.lancer(Some(&source(&m)), Some(&x.autre), Some(V301), false).await;
    assert!(x.bgm_pose().is_file());
    choisir_mods(&x.donnees, BASE, &[(DK.into(), false)]).unwrap();
    let (dec, b) = x.lancer(Some(&source(&m)), Some(&x.autre), Some(V301), false).await;
    tete.assert_hits(1);
    assert_eq!(dec.action, Musiques::Retirer { raison: None });
    assert!(!x.dossier_jeu().join(DOSSIER_MUSIQUES).exists() && note_musiques(&x.dossier_jeu()).is_none());
    assert!(b.abrite, "le retrait change ce que charge l'émulateur : abri d'abord");
    let mj = x.mods_json();
    assert!(!mj["mods"].as_array().unwrap().iter().any(|e| e["name"] == DOSSIER_MUSIQUES), "{mj}");
    assert!(b.musiques.is_some_and(|l| l.actives == 0 && !l.en_place && l.fait == "retiré"));
}

#[tokio::test]
async fn m155_le_bgm_bars_venu_du_dossier_d_un_mod_est_saute_tant_que_le_fichier_commun_est_actif() {
    let x = decor();
    let m = MockServer::start();
    servir(&m, DK, b"BGM-COMMUN", 1179648);
    let (dec, b) = x.lancer(Some(&source(&m)), Some(&x.autre), Some(V301), false).await;
    let pose_dk = x.dossier_jeu().join("DK Summit");
    assert_eq!(std::fs::read(pose_dk.join("romfs").join("Course").join("a.bin")).unwrap(), b"circuit");
    assert!(!pose_dk.join("romfs").join("Audio").join("Static").join("BGM.bars").exists(), "le BGM.bars du mod n'est jamais posé");
    let bgm_1080 = x.dossier_jeu().join("1080p").join("romfs").join("Audio").join("Static").join("BGM.bars");
    assert!(!bgm_1080.exists(), "le fichier commun en place : le BGM.bars d'un mod sans musique est sauté aussi");
    assert_eq!(b.bgm_sautes, 2);
    let j = ligne_journal_musiques(&dec, &b).unwrap();
    assert!(j.contains("2 fichier(s) de musique d'un mod sauté(s)"), "{j}");
    // Plus de fichier commun (version inconnue) : le mod SANS musique est reposé avec son fichier, mais jamais celui
    // d'un mod de MUSIQUE (sa musique ne vient que du fichier commun).
    let (_, b) = x.lancer(Some(&source(&m)), Some(&x.autre), None, false).await;
    assert!(!x.dossier_jeu().join(DOSSIER_MUSIQUES).exists());
    assert!(!pose_dk.join("romfs").join("Audio").join("Static").join("BGM.bars").exists(), "jamais pour un mod de musique");
    assert_eq!(std::fs::read(&bgm_1080).unwrap(), b"BGM-DU-1080P");
    assert_eq!(b.bgm_sautes, 0, "seul le 1080p est reposé, avec son fichier ; le mod de musique n'est pas recopié");
    assert!(raison_musiques_a_preparer(Some(41018496)).starts_with("musiques à préparer (≈ 39,1 Mo) : bouton « 🎵 Préparer les musiques »"));
    assert!(raison_musiques_a_preparer(None).starts_with("musiques à préparer : bouton"));
    assert!(est_chemin_bgm("ROMFS\\audio\\static\\bgm.BARS") && !est_chemin_bgm("romfs/Audio/Static/BGM.bars.bak"));
}

#[test]
fn m155_la_pose_est_atomique_et_un_dossier_modifie_n_est_jamais_touche() {
    let d = tempfile::tempdir().unwrap();
    let dj = d.path().join("contents").join("01007ef00011e000");
    std::fs::create_dir_all(&dj).unwrap();
    let cache = d.path().join("BGM-bon.bars");
    std::fs::write(&cache, b"BON").unwrap();
    let poser_sha = |f: &Path, o: &[u8], s: String| Musiques::Poser { fichier: f.to_path_buf(), sha256: s, taille: o.len() as u64, version: 1179648, cles: vec![(DK.into(), 2)] };
    let poser = |f: &Path, o: &[u8]| poser_sha(f, o, sha(o));
    // Une pose neuve qui échoue (le fichier du cache ne se relit pas comme annoncé) : aucun dossier commun, aucun reste.
    let faux = poser_sha(&cache, b"BON", sha(b"AUTRE"));
    let p = planifier_musiques(&dj, &faux);
    assert_eq!(p.etape, EtapeMusiques::Poser);
    assert!(executer_musiques(&dj, &p, &faux).is_err());
    assert!(!dj.join(DOSSIER_MUSIQUES).exists());
    assert!(std::fs::read_dir(d.path().join("contents")).unwrap().count() == 1, "aucune copie provisoire laissée");
    // La bonne : posée.
    let bon = poser(&cache, b"BON");
    let p = planifier_musiques(&dj, &bon);
    executer_musiques(&dj, &p, &bon).unwrap();
    let bgm = dj.join(DOSSIER_MUSIQUES).join("romfs").join("Audio").join("Static").join("BGM.bars");
    assert_eq!(std::fs::read(&bgm).unwrap(), b"BON");
    // Un remplacement qui échoue : l'ancien BGM.bars reste ENTIER, aucun provisoire.
    let cache2 = d.path().join("BGM-neuf.bars");
    std::fs::write(&cache2, b"NEUF").unwrap();
    let faux2 = poser_sha(&cache2, b"NEUF", sha(b"AUTRE"));
    let p = planifier_musiques(&dj, &faux2);
    assert_eq!(p.etape, EtapeMusiques::Remplacer);
    assert!(executer_musiques(&dj, &p, &faux2).is_err());
    assert_eq!(std::fs::read(&bgm).unwrap(), b"BON");
    assert!(!bgm.with_file_name(PROVISOIRE_BGM).exists());
    // Un provisoire laissé par une coupure (fichier de Frogtend, à son nom) n'empêche pas de reconnaître le dossier.
    std::fs::write(bgm.with_file_name(PROVISOIRE_BGM), b"moitie").unwrap();
    let neuf = poser(&cache2, b"NEUF");
    let p = planifier_musiques(&dj, &neuf);
    assert_eq!(p.etape, EtapeMusiques::Remplacer);
    executer_musiques(&dj, &p, &neuf).unwrap();
    assert_eq!(std::fs::read(&bgm).unwrap(), b"NEUF");
    assert!(!bgm.with_file_name(PROVISOIRE_BGM).exists());
    // La personne touche au dossier commun : plus jamais remplacé ni retiré.
    std::fs::write(bgm.with_file_name("sien.txt"), b"a moi").unwrap();
    for m in [Musiques::Retirer { raison: None }, poser(&cache, b"BON")] {
        let p = planifier_musiques(&dj, &m);
        assert!(matches!(p.etape, EtapeMusiques::Garde(_)) && !p.change() && !p.en_place, "{p:?}");
    }
}

#[tokio::test]
async fn m155_le_dossier_commun_n_est_jamais_liste_comme_un_mod() {
    let x = decor();
    let m = MockServer::start();
    servir(&m, DK, b"BGM-DK", 1179648);
    let (_, b) = x.lancer(Some(&source(&m)), Some(&x.autre), Some(V301), false).await;
    assert!(x.bgm_pose().is_file());
    // Ni dans la note des mods, ni parmi les mods présents, ni dans le rangement, ni dans la liste vue.
    let note: Value = serde_json::from_slice(&std::fs::read(x.dossier_jeu().join(NOTE_POSE)).unwrap()).unwrap();
    assert!(note.as_object().unwrap().values().all(|n| n["dossier"] != DOSSIER_MUSIQUES), "{note}");
    assert!(b.presents.iter().all(|p| p.dossier != DOSSIER_MUSIQUES));
    assert_eq!(ranges(&x.stock).len(), 3);
    let lecture = Lecture { etat: "ok".into(), vue_le: None, liste: x.autre.clone() };
    let vue = liste_vue(&lecture, &ranges(&x.stock), &charger(&x.donnees).0.jeu(BASE), Some(V301), BASE);
    assert!(vue.mods.iter().all(|m| m.nom != DOSSIER_MUSIQUES));
    // Le plan des mods ne le retire jamais, même quand plus aucun mod n'est allumé.
    choisir_mods(&x.donnees, BASE, &[(DK.into(), false), (CLE.into(), false)]).unwrap();
    let (raisons, reservees) = (&mut vec![], &mut vec![]);
    let v = voulus_pour(&x.stock, &charger(&x.donnees).0.jeu(BASE), Some(V301), Some(&x.autre), raisons, reservees);
    let plan = planifier(&x.dossier_jeu(), &v, &lire_note(&x.dossier_jeu()).unwrap());
    assert!(plan.etapes.iter().all(|e| !format!("{e:?}").contains(DOSSIER_MUSIQUES)), "{plan:?}");
    // Le nom est EXACTEMENT celui convenu avec Firehouse (jamais passé par `nom_de_dossier`).
    assert_eq!(DOSSIER_MUSIQUES, "!Firehouse - musiques");
    assert!(x.dossier_jeu().join("!Firehouse - musiques").is_dir());
}

#[tokio::test]
async fn m155_eden_recoit_le_dossier_commun_sans_jamais_le_couper() {
    let x = decor();
    let eden = x.ryu.parent().unwrap().join("Eden");
    let config = eden.join("user").join("config");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("qt-config.ini"), format!("[UI]\r\ntheme=dark\r\n\r\n[DisabledAddOns]\r\n1\\title_id\\default=false\r\n1\\title_id={BASE}\r\n1\\disabled\\1\\d\\default=false\r\n1\\disabled\\1\\d=Sien\r\n1\\disabled\\size=1\r\nsize=1\r\n")).unwrap();
    let m = MockServer::start();
    let bgm = b"BGM-EDEN".to_vec();
    servir(&m, DK, &bgm, 1179648);
    let dj = dossier_mods_eden(&eden, BASE).unwrap();
    let lancer = |src: Source, utilisee: Option<u32>| {
        let (donnees, stock, eden, dj, autre) = (x.donnees.clone(), x.stock.clone(), eden.clone(), dj.clone(), x.autre.clone());
        async move {
            let (ensemble, a_taire, servie) = ensemble_du_profil(&donnees, &stock, BASE, utilisee, Some(&autre));
            let dec = resoudre_musiques(DemandeMusiques { source: Some(&src), stock: &stock, dossier_jeu: &dj, base: BASE, ensemble, a_taire, utilisee, servie, auto: false, budget: BUDGET_MUSIQUES_LANCEMENT }).await;
            poser_eden_avec(&donnees, &eden, BASE, &stock, utilisee, false, Some(&autre), &dec).unwrap()
        }
    };
    let b = lancer(source(&m), Some(V301)).await;
    assert!(b.musiques.as_ref().is_some_and(|l| l.en_place), "{b:?}");
    let bgm_pose = dj.join(DOSSIER_MUSIQUES).join("romfs").join("Audio").join("Static").join("BGM.bars");
    assert_eq!(std::fs::read(&bgm_pose).unwrap(), bgm);
    assert!(dj.ends_with("01007EF00011E000"));
    let texte = std::fs::read_to_string(config.join("qt-config.ini")).unwrap();
    assert_eq!(crate::eden::lire_coupures(&texte), vec![(BASE, vec!["Sien".to_string()])], "jamais coupé, la personne gardée");
    // La version d'origine : le dossier commun part ; les coupures de la personne restent.
    let b = lancer(source(&m), Some(0)).await;
    assert!(!dj.join(DOSSIER_MUSIQUES).exists() && b.musiques.is_some_and(|l| !l.en_place));
    let texte = std::fs::read_to_string(config.join("qt-config.ini")).unwrap();
    assert_eq!(crate::eden::lire_coupures(&texte), vec![(BASE, vec!["Sien".to_string()])]);
}

#[tokio::test]
async fn m155_le_journal_et_la_fiche_ne_disent_rien_d_un_mod_reserve() {
    let x = decor();
    let m = MockServer::start();
    let bgm = b"BGM-AVEC-SECRET".to_vec();
    servir(&m, &format!("{SECRET},{DK}"), &bgm, 1179648);
    let (dec, b) = x.lancer(Some(&source(&m)), Some(&x.admin), Some(V301), false).await;
    assert_eq!(dec.actives, 2);
    assert!(x.bgm_pose().is_file());
    let j = ligne_journal_musiques(&dec, &b).unwrap();
    for interdit in [SECRET, DK, "pBGM", "Nom Secret", "DK Summit", "SHERBET"] {
        assert!(!j.contains(interdit), "le journal ne dit ni clé, ni amorce, ni titre : {j}");
    }
    // Un mod réservé parmi les musiques : même leur nombre se tait.
    assert!(j.starts_with("des mods de musique actifs") && !j.contains("2 mod") && j.contains(&sha(&bgm)[..12]) && !j.contains(&sha(&bgm)), "{j}");
    // La fiche : la ligne n'est montrée qu'au profil qui voit le mod réservé, et jamais avec ses clés.
    let p = derniere_pose(&x.stock).unwrap();
    assert!(p.clone().pour_le_profil(Some(&x.autre)).musiques.is_none());
    let vue = serde_json::to_string(&p.pour_le_profil(Some(&x.admin))).unwrap();
    assert!(vue.contains("\"en_place\":true") && !vue.contains(SECRET), "{vue}");
    // Pour l'autre profil, le fichier fait avec le mod réservé, encore en place, empêche la partie de partir.
    assert!(verifier_masques_ryubing(&x.racine, BASE, &x.stock, Some(&x.autre)).is_some());
    // … jusqu'à son lancement à lui, qui refait le fichier sans le mod réservé.
    let m2 = MockServer::start();
    servir(&m2, DK, b"BGM-DK-SEUL", 1179648);
    x.lancer(Some(&source(&m2)), Some(&x.autre), Some(V301), false).await;
    assert_eq!(note_musiques(&x.dossier_jeu()).unwrap().cles, vec![(DK.to_string(), 2)]);
    assert!(verifier_masques_ryubing(&x.racine, BASE, &x.stock, Some(&x.autre)).is_none());
}

#[test]
fn m155_deux_mods_de_musique_qui_veulent_la_meme_amorce_ne_s_allument_pas_ensemble() {
    let d = tempfile::tempdir().unwrap();
    let stock = d.path().join("stock");
    let a = zip_octets(&[("romfs/a.bin", b"un")], false);
    let b = zip_octets(&[("romfs/b.bin", b"deux")], false);
    let c = zip_octets(&[("romfs/c.bin", b"trois")], false);
    ranger_json(d.path(), &stock, &mod_musique(DK, "DK Summit", 2, &a, &["pBGM_GC_SHERBET_LAND_F"], "tous"), &a);
    ranger_json(d.path(), &stock, &mod_musique(AUTRE, "Sherbet remix", 1, &b, &["pBGM_GC_SHERBET_LAND_F", "pBGM_X"], "tous"), &b);
    ranger_json(d.path(), &stock, &mod_musique(CLE, "Autre circuit", 1, &c, &["pBGM_Y"], "tous"), &c);
    let rangs = ranges(&stock);
    let mut choix = crate::contenus_switch::ChoixJeu::default();
    choix.mods_eteints.push(format!("mod:{AUTRE}"));
    let m = conflit(&rangs, &choix, AUTRE, None).unwrap();
    assert!(m.contains("« Sherbet remix » et « DK Summit » changent la même musique du jeu"), "{m}");
    assert_eq!(conflit(&rangs, &choix, AUTRE, Some(DK)), None, "l'autre s'éteint en même temps");
    assert_eq!(conflit(&rangs, &choix, CLE, None), None, "des amorces différentes vont ensemble");
    // Un mod de musique NOUVEAU qui chevauche un mod allumé arrive éteint.
    let donnees = d.path().join("donnees");
    let neuf = ModServi::depuis(&mod_musique("neuf-0123456789ab", "Neuf", 1, &a, &["pBGM_Y"], "tous")).unwrap();
    let man = ManifesteMod { cle: neuf.cle.clone(), nom: neuf.nom.clone(), amorces: neuf.amorces.clone(), ..rangs[0].clone() };
    assert!(noter_arrivee(&donnees, &stock, BASE, &man).unwrap());
}

#[test]
fn m155_le_reglage_mettre_a_jour_tout_seul_se_lit_dans_le_profil() {
    let d = tempfile::tempdir().unwrap();
    assert!(!auto_du_profil(d.path(), "p1"), "absent : non");
    std::fs::create_dir_all(d.path().join("profils")).unwrap();
    std::fs::write(d.path().join("profils").join("p1.json"), br#"{"reglages": {"modsSwitch": {"auto": true}}}"#).unwrap();
    std::fs::write(d.path().join("profils").join("p2.json"), br#"{"reglages": {"modsSwitch": {"auto": false}}}"#).unwrap();
    std::fs::write(d.path().join("profils").join("p3.json"), b"{abime").unwrap();
    assert!(auto_du_profil(d.path(), "p1"));
    assert!(!auto_du_profil(d.path(), "p2"));
    assert!(!auto_du_profil(d.path(), "p3"), "illisible : non");
}

// --- Corrections après la relecture des experts (26/11) ---

/// Retire l'accord du décor (la personne n'a jamais dit oui aux musiques).
fn sans_accord(x: &Decor) {
    std::fs::remove_file(dossier_cache_musiques(&x.stock).join("accord.json")).unwrap();
}

#[tokio::test]
async fn m155_sans_oui_les_musiques_ne_se_telechargent_jamais() {
    let x = decor();
    sans_accord(&x);
    let m = MockServer::start();
    let bgm = b"BGM-DK".to_vec();
    let (_, corps) = servir(&m, DK, &bgm, 1179648);
    // Un mod de musique allumé avant la 1.55 (aucun oui) : jamais de téléchargement silencieux, le bouton de la fiche.
    for _ in 0..3 {
        let (dec, b) = x.lancer(Some(&source(&m)), Some(&x.autre), Some(V301), false).await;
        assert!(matches!(&dec.action, Musiques::Retirer { raison: Some(r) } if r.contains("Préparer les musiques")), "{dec:?}");
        assert!(b.musiques.is_some_and(|l| !l.en_place));
    }
    corps.assert_hits(0);
    assert!(!x.bgm_pose().exists());
    // Le oui (bouton « Préparer ») : noté pour l'ensemble et la version, puis le lancement télécharge.
    noter_accord_musiques(&x.stock, &vec![(DK.into(), 2)], Some(1179648), Some(bgm.len() as u64)).unwrap();
    assert_eq!(accord_musiques(&x.stock).unwrap().cles, vec![DK.to_string()]);
    x.lancer(Some(&source(&m)), Some(&x.autre), Some(V301), false).await;
    corps.assert_hits(1);
    assert!(x.bgm_pose().is_file());
    // L'accord couvre les clés (un mod éteint ensuite reste couvert), pour la MÊME version seulement.
    assert!(accord_couvre(&x.stock, &vec![(DK.into(), 2)], 1179648));
    assert!(!accord_couvre(&x.stock, &vec![(DK.into(), 2), (AUTRE.into(), 1)], 1179648));
    assert!(!accord_couvre(&x.stock, &vec![(DK.into(), 2)], 1245184));
}

#[tokio::test]
async fn m155_une_nouvelle_version_servie_sans_tout_seul_ne_telecharge_rien_lancement_apres_lancement() {
    let x = decor();
    let m = MockServer::start();
    servir(&m, DK, b"BGM-3.0.1", 1179648);
    x.lancer(Some(&source(&m)), Some(&x.autre), Some(V301), false).await;
    assert!(x.bgm_pose().is_file());
    // Firehouse sert la 3.0.2, la personne joue avec : sans « tout seul », aucun téléchargement, lancement après lancement.
    let mut sv = version_servie();
    sv.version = Some(1245184);
    sv.version_affichee = Some("3.0.2".into());
    let m2 = MockServer::start();
    let (_, corps2) = servir(&m2, DK, b"BGM-3.0.2", 1245184);
    let dj = x.dossier_jeu();
    let src2 = source(&m2);
    let demande = |auto: bool| DemandeMusiques {
        source: Some(&src2),
        stock: &x.stock,
        dossier_jeu: &dj,
        base: BASE,
        ensemble: vec![(DK.into(), 2)],
        a_taire: vec![],
        utilisee: Some(1245184),
        servie: Some(sv.clone()),
        auto,
        budget: BUDGET_MUSIQUES_LANCEMENT,
    };
    for _ in 0..3 {
        let dec = resoudre_musiques(demande(false)).await;
        assert!(matches!(&dec.action, Musiques::Retirer { raison: Some(r) } if r.contains("Préparer les musiques")), "{dec:?}");
        let b = poser_ryubing_avec(&x.donnees, &x.ryu, &x.racine, BASE, &x.stock, Some(1245184), Some(&x.autre), &dec).unwrap();
        assert!(b.musiques.is_some_and(|l| !l.en_place));
    }
    corps2.assert_hits(0);
    assert!(!x.bgm_pose().exists(), "le fichier de la 3.0.1 ne reste pas pour la 3.0.2");
    // « Mettre à jour les mods tout seul » : téléchargé.
    let dec = resoudre_musiques(demande(true)).await;
    corps2.assert_hits(1);
    assert!(matches!(dec.action, Musiques::Poser { version: 1245184, .. }));
}

#[tokio::test]
async fn m155_au_lancement_le_telechargement_a_un_temps_limite_et_reprend_la_fois_suivante() {
    let x = decor();
    let m = MockServer::start();
    let bgm = b"BGM-LENT".to_vec();
    let s = sha(&bgm);
    m.mock(|w, r| {
        w.method(httpmock::Method::HEAD).path(route_tete());
        r.status(200).header("X-Sha256", &s).header("X-Version-Jeu", "1179648").header("content-length", bgm.len().to_string());
    });
    let mut lent = m.mock(|w, r| {
        w.method(GET).path(route_tete());
        r.status(200).body(&bgm).delay(std::time::Duration::from_secs(3));
    });
    let (ensemble, a_taire, servie) = ensemble_du_profil(&x.donnees, &x.stock, BASE, Some(V301), Some(&x.autre));
    let dj = x.dossier_jeu();
    let src = source(&m);
    let dec = resoudre_musiques(DemandeMusiques {
        source: Some(&src),
        stock: &x.stock,
        dossier_jeu: &dj,
        base: BASE,
        ensemble,
        a_taire,
        utilisee: Some(V301),
        servie,
        auto: false,
        budget: std::time::Duration::from_millis(300),
    })
    .await;
    assert_eq!(dec.action, Musiques::Retirer { raison: Some(RAISON_MUSIQUES_PROCHAINE.into()) });
    lent.delete();
    // La fois suivante (Firehouse répond vite) : le fichier arrive.
    m.mock(|w, r| {
        w.method(GET).path(route_tete());
        r.status(200).body(&bgm);
    });
    let (dec, _) = x.lancer(Some(&src), Some(&x.autre), Some(V301), false).await;
    assert!(matches!(dec.action, Musiques::Poser { .. }), "{dec:?}");
    assert_eq!(std::fs::read(x.bgm_pose()).unwrap(), bgm);
}

#[tokio::test]
async fn m155_un_head_sans_taille_prend_un_plafond_fixe_et_le_sha_est_toujours_verifie() {
    let x = decor();
    let m = MockServer::start();
    let bgm = b"BGM-SANS-TAILLE".to_vec();
    let s = sha(&bgm);
    m.mock(|w, r| {
        w.method(httpmock::Method::HEAD).path(route_tete());
        r.status(200).header("X-Sha256", &s).header("X-Version-Jeu", "1179648");
    });
    let corps = m.mock(|w, r| {
        w.method(GET).path(route_tete());
        r.status(200).body(&bgm);
    });
    let (dec, _) = x.lancer(Some(&source(&m)), Some(&x.autre), Some(V301), false).await;
    corps.assert_hits(1);
    assert!(matches!(&dec.action, Musiques::Poser { taille, .. } if *taille == bgm.len() as u64), "{dec:?}");
    assert_eq!(std::fs::read(x.bgm_pose()).unwrap(), bgm);
    // Un fichier reçu qui n'a pas le sha annoncé n'est jamais gardé ni posé.
    let x2 = decor();
    let m2 = MockServer::start();
    m2.mock(|w, r| {
        w.method(httpmock::Method::HEAD).path(route_tete());
        r.status(200).header("X-Sha256", sha(b"AUTRE")).header("X-Version-Jeu", "1179648");
    });
    m2.mock(|w, r| {
        w.method(GET).path(route_tete());
        r.status(200).body(&bgm);
    });
    let (dec, _) = x2.lancer(Some(&source(&m2)), Some(&x2.autre), Some(V301), false).await;
    assert!(matches!(dec.action, Musiques::Retirer { .. } | Musiques::Garder { .. }), "{dec:?}");
    assert!(!x2.bgm_pose().exists());
    assert!(!dossier_cache_musiques(&x2.stock).join(nom_cache_bgm(&sha(b"AUTRE"))).exists());
}

#[test]
fn m155_une_pose_ou_un_retrait_coupes_en_route_laissent_un_dossier_que_frogtend_reconnait() {
    let d = tempfile::tempdir().unwrap();
    let dj = d.path().join("contents").join("01007ef00011e000");
    let commun = dj.join(DOSSIER_MUSIQUES);
    let statique = commun.join("romfs").join("Audio").join("Static");
    std::fs::create_dir_all(&statique).unwrap();
    let note = |en_cours: Option<String>| NoteMusiques { cles: vec![(DK.into(), 2)], version: 1179648, sha256: sha(b"ANCIEN"), taille: 6, fichiers: BTreeMap::new(), en_cours };
    let ecrire = |n: &NoteMusiques| std::fs::write(dj.join(NOTE_MUSIQUES), serde_json::to_vec(n).unwrap()).unwrap();
    let cache = d.path().join("BGM-neuf.bars");
    std::fs::write(&cache, b"NEUF").unwrap();
    let poser = Musiques::Poser { fichier: cache.clone(), sha256: sha(b"NEUF"), taille: 4, version: 1179648, cles: vec![(DK.into(), 2)] };
    // Coupure juste après le renommage du NOUVEAU fichier (le sha attendu était noté, pas encore la note finale).
    std::fs::write(statique.join("BGM.bars"), b"NEUF").unwrap();
    ecrire(&note(Some(sha(b"NEUF"))));
    assert_eq!(planifier_musiques(&dj, &Musiques::Retirer { raison: None }).etape, EtapeMusiques::Retirer);
    let p = planifier_musiques(&dj, &poser);
    assert_eq!(p.etape, EtapeMusiques::Remplacer);
    executer_musiques(&dj, &p, &poser).unwrap();
    let n = note_musiques(&dj).unwrap();
    assert!(n.en_cours.is_none() && n.sha256 == sha(b"NEUF"));
    assert_eq!(planifier_musiques(&dj, &poser).etape, EtapeMusiques::Rien, "reconnu tel quel ensuite");
    // Un BGM.bars à un AUTRE sha (ni le noté, ni l'attendu) : jamais touché.
    std::fs::write(statique.join("BGM.bars"), b"ETRANGER").unwrap();
    ecrire(&note(Some(sha(b"NEUF"))));
    assert!(matches!(planifier_musiques(&dj, &Musiques::Retirer { raison: None }).etape, EtapeMusiques::Garde(_)));
    // Un retrait coupé en route : plus que des dossiers vides (ou le provisoire) → reconnu, retiré.
    std::fs::remove_file(statique.join("BGM.bars")).unwrap();
    std::fs::write(statique.join(PROVISOIRE_BGM), b"moit").unwrap();
    ecrire(&note(None));
    let p = planifier_musiques(&dj, &Musiques::Retirer { raison: None });
    assert_eq!(p.etape, EtapeMusiques::Retirer);
    executer_musiques(&dj, &p, &Musiques::Retirer { raison: None }).unwrap();
    assert!(!commun.exists() && note_musiques(&dj).is_none());
    // Un fichier de la personne à côté : jamais reconnu comme à Frogtend.
    std::fs::create_dir_all(&statique).unwrap();
    std::fs::write(commun.join("sien.txt"), b"a moi").unwrap();
    ecrire(&note(None));
    assert!(matches!(planifier_musiques(&dj, &Musiques::Retirer { raison: None }).etape, EtapeMusiques::Garde(_)));
}

#[tokio::test]
async fn m155_un_get_refuse_apres_un_head_reussi_ne_pose_rien() {
    let x = decor();
    let m = MockServer::start();
    let s = sha(b"BGM");
    m.mock(|w, r| {
        w.method(httpmock::Method::HEAD).path(route_tete());
        r.status(200).header("X-Sha256", &s).header("X-Version-Jeu", "1179648").header("content-length", "3");
    });
    m.mock(|w, r| {
        w.method(GET).path(route_tete());
        r.status(409).header("X-Code", "conflit").json_body(json!({"ok": false, "code": "conflit", "detail": "deux mods"}));
    });
    let (dec, _) = x.lancer(Some(&source(&m)), Some(&x.autre), Some(V301), false).await;
    assert!(matches!(dec.action, Musiques::Retirer { raison: Some(_) }), "{dec:?}");
    assert!(!x.bgm_pose().exists());
    assert!(dec.journal.iter().all(|j| !j.contains("deux mods")), "le texte de Firehouse ne va jamais au journal");
    // Les mêmes règles X-Code pour le HEAD et pour le GET.
    let hl = Musiques::Garder { raison: None, maj: None };
    assert_eq!(musiques_selon_code(409, Some("conflit"), true, hl.clone()), Musiques::Retirer { raison: Some(RAISON_MUSIQUES_CONFLIT.into()) });
    assert!(matches!(musiques_selon_code(409, Some("original_absent"), true, hl.clone()), Musiques::Garder { .. }));
    assert_eq!(musiques_selon_code(404, None, true, hl.clone()), Musiques::Retirer { raison: Some(RAISON_MUSIQUES_INTROUVABLE.into()) });
    assert_eq!(musiques_selon_code(503, None, true, hl.clone()), hl);
}
