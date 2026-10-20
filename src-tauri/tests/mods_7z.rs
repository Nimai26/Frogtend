//! Les mods en 7z (lot 0.82.2) posés À TRAVERS le vrai lecteur isolé (le programme Frogtend lancé comme enfant).

use frogtend_lib::mods::{desinstaller, dossier_mods, installer, installes, FichierMod};
use std::path::{Path, PathBuf};

fn regler() {
    let exe = std::env::var_os("FROGTEND_EXE").map(PathBuf::from).unwrap_or_else(|| env!("CARGO_BIN_EXE_frogtend").into());
    frogtend_lib::sept_z::programme_pour_les_tests(&exe);
}

fn fichier(nom: &str) -> FichierMod {
    FichierMod { nom: nom.into(), taille: 1, sha256: None, url: String::new(), depot: "nexus/essai/1".into(), tag: "2".into(), a_confirmer: true, dossier: String::new() }
}

/// Tout le contenu d'un dossier (hors `Frogtend\`), octet par octet, pour comparer avant / après.
fn etat(d: &Path) -> Vec<(String, Vec<u8>)> {
    let mut l = Vec::new();
    let mut pile = vec![d.to_path_buf()];
    while let Some(p) = pile.pop() {
        for e in std::fs::read_dir(&p).unwrap().flatten() {
            let c = e.path();
            if c.is_dir() {
                if c.file_name().is_some_and(|n| n != "Frogtend") {
                    pile.push(c);
                }
            } else {
                l.push((c.strip_prefix(d).unwrap().to_string_lossy().to_string(), std::fs::read(&c).unwrap()));
            }
        }
    }
    l.sort();
    l
}

#[test]
fn un_mod_7z_passe_par_le_vrai_lecteur_isole() {
    regler();
    let d = tempfile::tempdir().unwrap();
    let jeu = d.path().join("Jeu é");
    std::fs::create_dir_all(&jeu).unwrap();
    std::fs::write(jeu.join("jeu.exe"), b"jeu").unwrap();
    std::fs::write(jeu.join("dsound.dll"), b"original").unwrap();
    let src = d.path().join("src");
    std::fs::create_dir_all(src.join("scripts")).unwrap();
    std::fs::write(src.join("dsound.dll"), b"du mod").unwrap();
    std::fs::write(src.join("scripts/Fix.asi"), vec![3u8; 200_000]).unwrap();
    let a = d.path().join("mod.7z");
    sevenz_rust::compress_to_path(&src, &a).unwrap();
    let avant = etat(&jeu);
    let f = fichier("mod.7z");
    let b = installer(&jeu, &a, &f, "Mod d'essai", "abc", "", false, "2026-10-20").unwrap();
    assert_eq!((b.ajoutes, b.remplaces), (1, 1));
    assert_eq!(std::fs::read(jeu.join("scripts/Fix.asi")).unwrap(), vec![3u8; 200_000]);
    assert!(!dossier_mods(&jeu).join(f.cle()).join("brut").exists());
    desinstaller(&jeu, &f.cle()).unwrap();
    assert!(installes(&jeu).is_empty());
    assert_eq!(etat(&jeu), avant, "le jeu redevient comme avant");
}

/// Essai réel (lecture seule sur l'archive) : `FROGTEND_ESSAI_MOD7Z` = une vraie archive 7z de Seb, posée comme un mod
/// dans un FAUX jeu (`D:\Frogtend\.essai-mod7z`, effacé ensuite), puis retirée : le faux jeu doit redevenir identique.
#[test]
#[ignore]
fn essai_vraie_archive_7z_comme_mod() {
    regler();
    let archive = PathBuf::from(std::env::var("FROGTEND_ESSAI_MOD7Z").expect("FROGTEND_ESSAI_MOD7Z"));
    let date = std::fs::metadata(&archive).unwrap().modified().unwrap();
    // Sans « .. » dans le chemin (la garde anti-lien le refuserait, à juste titre).
    let essai = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join(".essai-mod7z");
    let jeu = essai.join("Faux jeu");
    std::fs::create_dir_all(&jeu).unwrap();
    std::fs::write(jeu.join("jeu.exe"), b"jeu").unwrap();
    let avant = etat(&jeu);
    let f = fichier(&archive.file_name().unwrap().to_string_lossy());
    let debut = std::time::Instant::now();
    let b = installer(&jeu, &archive, &f, "Essai", "abc", "", false, "2026-10-20").unwrap();
    println!("{} : {} ajouté(s), {} remplacé(s), {:?}", archive.display(), b.ajoutes, b.remplaces, debut.elapsed());
    assert!(b.ajoutes > 0);
    desinstaller(&jeu, &f.cle()).unwrap();
    assert_eq!(etat(&jeu), avant);
    assert_eq!(std::fs::metadata(&archive).unwrap().modified().unwrap(), date, "l'archive n'est jamais touchée");
    std::fs::remove_dir_all(&essai).unwrap();
}
