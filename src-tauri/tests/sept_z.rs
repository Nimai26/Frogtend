//! L'isolement réel des archives 7z (lot 0.82.1) : le VRAI programme Frogtend est lancé comme enfant, enfermé dans un
//! job Windows. Les tests unitaires de `sept_z` servent l'enfant sur place ; ici, c'est le processus à part.

use frogtend_lib::sept_z::{executer_avec, ARG};
use std::ffi::OsString;
use std::path::Path;

/// Le programme de développement, ou celui donné par `FROGTEND_EXE` (le programme LIVRÉ : sans console, `abort`).
fn exe() -> std::path::PathBuf {
    std::env::var_os("FROGTEND_EXE").map(Into::into).unwrap_or_else(|| env!("CARGO_BIN_EXE_frogtend").into())
}

fn archive(d: &Path, nom: &str) -> std::path::PathBuf {
    let src = d.join("src");
    std::fs::create_dir_all(src.join("jeu")).unwrap();
    std::fs::write(src.join("jeu/rom.bin"), vec![7u8; 300_000]).unwrap();
    std::fs::write(src.join("lisez-moi.txt"), b"bonjour").unwrap();
    let a = d.join(nom);
    sevenz_rust::compress_to_path(&src, &a).unwrap();
    a
}

#[test]
fn le_vrai_programme_lit_et_extrait_un_7z_au_chemin_accentue() {
    assert_eq!(ARG, "--frogtend-7z");
    let d = tempfile::tempdir().unwrap();
    let a = archive(d.path(), "Pokémon ✦ é.7z");
    let mut entrees = Vec::new();
    executer_avec(&exe(), &[OsString::from("lister"), a.clone().into()], &mut |v| entrees.push(v.clone())).unwrap();
    assert!(entrees.iter().any(|v| v["e"]["nom"] == "jeu/rom.bin" && v["e"]["taille"] == 300_000), "{entrees:?}");
    let dest = d.path().join("sortie é");
    let fin = executer_avec(&exe(), &[OsString::from("extraire"), a.into(), dest.clone().into()], &mut |_| {}).unwrap();
    assert_eq!(fin["fichiers"], 2);
    // Vérifier le résultat, pas le retour.
    assert_eq!(std::fs::read(dest.join("jeu/rom.bin")).unwrap(), vec![7u8; 300_000]);
}

/// Une archive qui réclame un dictionnaire LZMA2 de 4 Gio (propriété 40) : sans plafond, l'enfant tenterait de les
/// réserver ; dans le job (2 Gio), il meurt, et le parent répond « piégée ».
#[test]
fn un_dictionnaire_demesure_bute_sur_le_plafond_du_job() {
    let d = tempfile::tempdir().unwrap();
    let a = archive(d.path(), "dico.7z");
    let mut o = std::fs::read(&a).unwrap();
    let decalage = u64::from_le_bytes(o[12..20].try_into().unwrap()) as usize;
    let taille = u64::from_le_bytes(o[20..28].try_into().unwrap()) as usize;
    let mut entete = o[32 + decalage..32 + decalage + taille].to_vec();
    o.truncate(32 + decalage);
    // L'en-tête est lui-même compressé (LZMA : drapeau 0x23, identifiant 03 01 01, 5 octets de propriétés dont le
    // dictionnaire) : dictionnaire porté à 3,75 Gio.
    let i = entete.windows(5).position(|w| w == [0x23, 0x03, 0x01, 0x01, 0x05]).expect("codeur LZMA de l'en-tête") + 6;
    entete[i..i + 4].copy_from_slice(&0xF000_0000u32.to_le_bytes());
    // Et sa taille décompressée (0x0C, puis un nombre 7z) portée à 3 Gio : 0xFF + 8 octets.
    let j = entete.iter().position(|b| *b == 0x0C).expect("tailles décompressées");
    let longueur = match entete[j + 1] {
        x if x & 0x80 == 0 => 1,
        x if x & 0x40 == 0 => 2,
        _ => panic!("nombre plus long qu'attendu"),
    };
    let mut nombre = vec![0xFF];
    nombre.extend_from_slice(&(3u64 << 30).to_le_bytes());
    entete.splice(j + 1..j + 1 + longueur, nombre);
    o.extend_from_slice(&entete);
    o[20..28].copy_from_slice(&(entete.len() as u64).to_le_bytes());
    let crc = crc32fast::hash(&entete);
    o[28..32].copy_from_slice(&crc.to_le_bytes());
    let crc = crc32fast::hash(&o[12..32]);
    o[8..12].copy_from_slice(&crc.to_le_bytes());
    std::fs::write(&a, &o).unwrap();
    let debut = std::time::Instant::now();
    let r = executer_avec(&exe(), &[OsString::from("extraire"), a.into(), d.path().join("x").into()], &mut |_| {});
    // L'enfant est MORT (pas un refus poli de la bibliothèque) : c'est le plafond du job qui l'a arrêté.
    assert!(matches!(&r, Err(frogtend_lib::erreurs::Erreur::Disque(m)) if m.contains("piégée")), "{r:?}");
    assert!(debut.elapsed().as_secs() < 60);
}

/// Essai réel (lecture seule) : `FROGTEND_ESSAI_7Z` = des chemins d'archives séparés par « ; ». Chacune est listée puis
/// extraite par le vrai programme dans `D:\Frogtend\.essai-7z` (effacé ensuite) ; les fichiers écrits doivent avoir
/// exactement les tailles annoncées.
#[test]
#[ignore]
fn essai_vraies_archives_de_seb() {
    let liste = std::env::var("FROGTEND_ESSAI_7Z").expect("FROGTEND_ESSAI_7Z");
    let essai = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(".essai-7z");
    for (n, chemin) in liste.split(';').filter(|c| !c.is_empty()).enumerate() {
        let a = Path::new(chemin);
        let avant = std::fs::metadata(a).unwrap().modified().unwrap();
        let debut = std::time::Instant::now();
        let mut entrees: Vec<serde_json::Value> = Vec::new();
        executer_avec(&exe(), &[OsString::from("lister"), a.into()], &mut |v| entrees.push(v["e"].clone())).unwrap();
        let dest = essai.join(n.to_string());
        let fin = executer_avec(&exe(), &[OsString::from("extraire"), a.into(), dest.clone().into()], &mut |_| {}).unwrap();
        let fichiers: Vec<_> = entrees.iter().filter(|e| e["dossier"] == false && e["anti"] == false).collect();
        for e in &fichiers {
            let ecrit = std::fs::metadata(dest.join(e["nom"].as_str().unwrap())).unwrap().len();
            assert_eq!(ecrit, e["taille"].as_u64().unwrap(), "{}", e["nom"]);
        }
        assert_eq!(fin["fichiers"].as_u64().unwrap() as usize, fichiers.len());
        assert_eq!(std::fs::metadata(a).unwrap().modified().unwrap(), avant, "l'archive n'est jamais touchée");
        let octets: u64 = fichiers.iter().map(|e| e["taille"].as_u64().unwrap()).sum();
        println!("{chemin} : {} fichier(s), {} octets, {:?}", fichiers.len(), octets, debut.elapsed());
    }
    std::fs::remove_dir_all(&essai).unwrap();
}

#[test]
fn une_archive_abimee_est_ecartee_et_frogtend_reste_debout() {
    let d = tempfile::tempdir().unwrap();
    let a = archive(d.path(), "abimee.7z");
    // Les données compressées abîmées (l'en-tête de départ reste valide) : l'enfant échoue, le parent répond.
    let mut o = std::fs::read(&a).unwrap();
    let fin = 64.min(o.len());
    for b in &mut o[32..fin] {
        *b ^= 0xA5;
    }
    std::fs::write(&a, &o).unwrap();
    let r = executer_avec(&exe(), &[OsString::from("extraire"), a.into(), d.path().join("x").into()], &mut |_| {});
    assert!(r.is_err());
    // Un ordre inconnu : l'enfant refuse, sans ligne de fin.
    assert!(executer_avec(&exe(), &[OsString::from("inconnu"), d.path().join("abimee.7z").into()], &mut |_| {}).is_err());
}

/// La progression arrive PENDANT l'extraction (le tube de l'enfant est vidé à chaque ligne) : au premier signe, le
/// dernier fichier n'est pas encore écrit.
#[test]
fn la_progression_arrive_pendant_l_extraction() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    let mut x: u32 = 1;
    for n in 0..3 {
        // Des octets peu compressibles : l'extraction prend un peu de temps.
        let o: Vec<u8> = (0..6_000_000).map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x as u8
        }).collect();
        std::fs::write(src.join(format!("f{n}.bin")), o).unwrap();
    }
    let a = d.path().join("p.7z");
    sevenz_rust::compress_to_path(&src, &a).unwrap();
    let dest = d.path().join("x");
    let (mut signes, mut dernier_deja_la) = (0, None);
    executer_avec(&exe(), &[OsString::from("extraire"), a.into(), dest.clone().into()], &mut |v| {
        if v.get("p").is_some() {
            signes += 1;
            dernier_deja_la.get_or_insert(dest.join("f2.bin").exists());
        }
    })
    .unwrap();
    assert!(signes >= 3, "{signes}");
    assert_eq!(dernier_deja_la, Some(false), "la progression n'arrivait qu'à la fin");
}
