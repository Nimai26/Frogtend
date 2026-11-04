//! Changer un dossier de Frogtend sans rien casser (✅ Seb 04/11) : le contenu de l'ancien dossier est déplacé dans le
//! nouveau, après l'accord de la personne (combien de fichiers, combien d'octets).
//!
//! Seul ce que FROGTEND y a écrit part (règle « pas de pieuvre ») : pour les abris, les dossiers `<numéro du jeu>` ;
//! pour les captures, `<Système>\<jeu>\<image>`. Fichier par fichier : renommé sur le même disque ; sinon copié dans
//! `<nouveau>\.frogtend-en-cours\` (hors de tout abri), CONTENU comparé, renommé à sa place, et SEULEMENT ENSUITE
//! l'original effacé. Jamais par-dessus un fichier déjà dans le nouveau dossier : il est laissé à sa place (et
//! compté). Les liens et jonctions ne sont jamais suivis. Les dossiers vidés sont retirés, jamais un dossier qui garde
//! un fichier.

use std::path::{Component, Path, PathBuf};

use serde::Serialize;

use crate::erreurs::{Erreur, Resultat};

/// Le dossier provisoire des copies, dans le nouveau dossier (jamais lu comme un abri ni une capture).
const PROVISOIRE: &str = ".frogtend-en-cours";

/// Ce que contient un dossier (pour l'accord chiffré).
#[derive(Debug, Default, Clone, Copy, Serialize, PartialEq)]
pub struct Contenu {
    pub fichiers: usize,
    pub octets: u64,
}

/// Ce qui a été fait.
#[derive(Debug, Default, Serialize, PartialEq)]
pub struct Bilan {
    pub deplaces: usize,
    pub octets: u64,
    /// Déjà présents dans le nouveau dossier, ou impossibles à déplacer : gardés à l'ancienne place.
    pub laisses: Vec<String>,
}

/// Ce que Frogtend a écrit dans un dossier d'une `sorte` (chemin relatif) : seul cela se déplace.
pub fn a_frogtend(sorte: &str, rel: &Path) -> bool {
    let morceaux: Vec<String> = rel
        .components()
        .map(|c| match c {
            Component::Normal(s) => s.to_string_lossy().to_string(),
            _ => String::new(),
        })
        .collect();
    if morceaux.is_empty() || morceaux.iter().any(|m| m.is_empty()) {
        return false;
    }
    match sorte {
        // `<numéro du jeu>\…` (les numéros des jeux importés sont négatifs).
        "abris" => {
            let m = &morceaux[0];
            let chiffres = m.strip_prefix('-').unwrap_or(m);
            morceaux.len() > 1 && !chiffres.is_empty() && chiffres.chars().all(|c| c.is_ascii_digit())
        }
        // `<Système>\<jeu>\<image>`.
        "captures" => {
            morceaux.len() == 3
                && rel.extension().is_some_and(|e| ["png", "jpg", "jpeg", "bmp", "webp"].contains(&e.to_string_lossy().to_lowercase().as_str()))
        }
        _ => false,
    }
}

/// Les fichiers d'un dossier, chemins relatifs (les liens et jonctions ne sont jamais suivis ; le dossier provisoire
/// est ignoré). Le booléen est faux si un sous-dossier n'a pas pu être lu.
fn fichiers(racine: &Path) -> (Vec<(PathBuf, u64)>, bool) {
    let (mut l, mut lisible) = (Vec::new(), true);
    let mut a_voir = vec![PathBuf::new()];
    while let Some(rel) = a_voir.pop() {
        let Ok(entrees) = std::fs::read_dir(racine.join(&rel)) else {
            lisible = false;
            continue;
        };
        for e in entrees.flatten() {
            let Ok(t) = e.file_type() else { continue };
            let r = rel.join(e.file_name());
            // Un lien ou une JONCTION (PPSSPP : memstick → dossier du profil) n'est jamais suivi ni déplacé.
            if t.is_symlink() || crate::garde::est_un_lien(&racine.join(&r)) || e.file_name() == PROVISOIRE {
                continue;
            }
            if t.is_dir() {
                a_voir.push(r);
            } else if t.is_file() {
                l.push((r, e.metadata().map(|m| m.len()).unwrap_or(0)));
            }
        }
    }
    l.sort();
    (l, lisible)
}

/// Ce que Frogtend a écrit dans ce dossier (voir `a_frogtend`).
pub fn mesurer(dossier: &Path, sorte: &str) -> Contenu {
    let l: Vec<_> = fichiers(dossier).0.into_iter().filter(|(r, _)| a_frogtend(sorte, r)).collect();
    Contenu { fichiers: l.len(), octets: l.iter().map(|(_, t)| t).sum() }
}

/// L'un des deux dossiers est-il dans l'autre (ou le même) ? Alors on ne déplace rien.
pub fn imbriques(a: &Path, b: &Path) -> bool {
    crate::garde::meme(a, b) || crate::garde::sous(a, b) || crate::garde::sous(b, a)
}

/// La copie d'un disque à l'autre : dans `provisoire`, CONTENU comparé (des parties parfois uniques), renommée en
/// `cible`, puis l'original effacé. Un échec retire le provisoire et garde l'original.
fn copier_verifie(source: &Path, cible: &Path, provisoire: &Path) -> std::io::Result<()> {
    if let Some(p) = provisoire.parent() {
        std::fs::create_dir_all(p)?;
    }
    let essai = (|| {
        std::fs::copy(source, provisoire)?;
        if !crate::installation::memes_contenus(source, provisoire) {
            return Err(std::io::Error::other("copie différente de l'original"));
        }
        std::fs::rename(provisoire, cible)
    })();
    if let Err(e) = essai {
        let _ = std::fs::remove_file(provisoire);
        return Err(e);
    }
    std::fs::remove_file(source)
}

/// Déplace un fichier (voir le module). `Ok(false)` : la cible existe déjà, rien n'est fait.
fn deplacer_fichier(source: &Path, cible: &Path, provisoire: &Path) -> std::io::Result<bool> {
    if cible.try_exists().unwrap_or(true) {
        return Ok(false);
    }
    if let Some(p) = cible.parent() {
        std::fs::create_dir_all(p)?;
    }
    // (Une petite fenêtre reste entre `try_exists` et `rename`, qui remplace sous Windows : seulement si un autre
    // programme écrit ce même fichier dans le nouveau dossier pendant le déplacement.)
    if std::fs::rename(source, cible).is_err() {
        copier_verifie(source, cible, provisoire)?;
    }
    Ok(true)
}

/// Retire les dossiers VIDES sous `racine` (du plus profond au moins profond), puis `racine` si elle est vide.
fn retirer_dossiers_vides(racine: &Path) {
    let mut dossiers = Vec::new();
    let mut a_voir = vec![racine.to_path_buf()];
    while let Some(d) = a_voir.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            if e.file_type().is_ok_and(|t| t.is_dir() && !t.is_symlink()) && !crate::garde::est_un_lien(&e.path()) {
                a_voir.push(e.path());
            }
        }
        dossiers.push(d);
    }
    // `remove_dir` refuse un dossier qui n'est pas vide : rien d'autre ne peut partir.
    for d in dossiers.iter().rev() {
        let _ = std::fs::remove_dir(d);
    }
}

/// Déplace ce que Frogtend a écrit dans `ancien` (`sorte`) vers `nouveau` ; `progres(octets déplacés)`.
pub fn deplacer_contenu(sorte: &str, ancien: &Path, nouveau: &Path, progres: &mut dyn FnMut(u64)) -> Resultat<Bilan> {
    if imbriques(ancien, nouveau) {
        return Err(Erreur::Refus("L'ancien et le nouveau dossier sont l'un dans l'autre : rien n'est déplacé.".into()));
    }
    std::fs::create_dir_all(nouveau)?;
    let (liste, lisible) = fichiers(ancien);
    let mut b = Bilan::default();
    if !lisible {
        b.laisses.push(format!("une partie de {} n'a pas pu être lue", ancien.display()));
    }
    let provisoire = nouveau.join(PROVISOIRE);
    for (rel, taille) in liste.into_iter().filter(|(r, _)| a_frogtend(sorte, r)) {
        match deplacer_fichier(&ancien.join(&rel), &nouveau.join(&rel), &provisoire.join(&rel)) {
            Ok(true) => {
                b.deplaces += 1;
                b.octets += taille;
                progres(b.octets);
            }
            Ok(false) => b.laisses.push(format!("{} (déjà dans le nouveau dossier)", rel.display())),
            Err(e) => b.laisses.push(format!("{} ({e})", rel.display())),
        }
    }
    retirer_dossiers_vides(&provisoire);
    retirer_dossiers_vides(ancien);
    Ok(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seul_ce_que_frogtend_a_ecrit() {
        assert!(a_frogtend("abris", Path::new("110/1790724553/C/DUNE.SAV")));
        assert!(a_frogtend("abris", Path::new("-7/1790724553.json")));
        assert!(!a_frogtend("abris", Path::new("Mes documents/lettre.docx")));
        assert!(!a_frogtend("abris", Path::new("110a/x")));
        assert!(!a_frogtend("abris", Path::new("-/x")));
        assert!(!a_frogtend("abris", Path::new("2024")), "un fichier seul à la racine : pas un abri");
        assert!(a_frogtend("captures", Path::new("Super Nintendo/Super Mario World/2026-11-04 12-00-00.png")));
        assert!(!a_frogtend("captures", Path::new("vacances.jpg")), "une photo à la personne");
        assert!(!a_frogtend("captures", Path::new("S/J/film.avi")));
        assert!(!a_frogtend("jeux", Path::new("110/x")));
    }

    #[test]
    fn tout_part_sauf_ce_qui_existe_deja_ou_n_est_pas_a_frogtend() {
        let d = tempfile::tempdir().unwrap();
        let (a, n) = (d.path().join("Ancien"), d.path().join("Nouveau"));
        std::fs::create_dir_all(a.join("110").join("1790724553").join("C")).unwrap();
        std::fs::write(a.join("110").join("1790724553").join("C").join("DUNE.SAV"), b"partie").unwrap();
        std::fs::write(a.join("110").join("1790724553.json"), b"{}").unwrap();
        std::fs::write(a.join("lettre.docx"), b"a la personne").unwrap();
        std::fs::create_dir_all(n.join("110")).unwrap();
        std::fs::write(a.join("110").join("deja.txt"), b"ancien").unwrap();
        std::fs::write(n.join("110").join("deja.txt"), b"nouveau").unwrap();
        assert_eq!(mesurer(&a, "abris"), Contenu { fichiers: 3, octets: 6 + 2 + 6 });
        let mut vu = 0;
        let b = deplacer_contenu("abris", &a, &n, &mut |o| vu = o).unwrap();
        assert_eq!((b.deplaces, b.octets, b.laisses.len()), (2, 8, 1));
        assert_eq!(vu, 8);
        assert_eq!(std::fs::read(n.join("110").join("1790724553").join("C").join("DUNE.SAV")).unwrap(), b"partie");
        assert!(n.join("110").join("1790724553.json").is_file());
        assert_eq!(std::fs::read(n.join("110").join("deja.txt")).unwrap(), b"nouveau", "jamais par-dessus");
        assert_eq!(std::fs::read(a.join("110").join("deja.txt")).unwrap(), b"ancien", "laissé à sa place");
        assert!(a.join("lettre.docx").is_file() && !n.join("lettre.docx").exists(), "pas à Frogtend : jamais touché");
        assert!(!a.join("110").join("1790724553").exists(), "dossier vidé retiré");
        assert!(!n.join(PROVISOIRE).exists(), "rien de provisoire ne reste");
    }

    #[test]
    fn la_copie_d_un_disque_a_l_autre() {
        let d = tempfile::tempdir().unwrap();
        let source = d.path().join("a.sav");
        std::fs::write(&source, b"partie unique").unwrap();
        let cible = d.path().join("N").join("110").join("a.sav");
        std::fs::create_dir_all(cible.parent().unwrap()).unwrap();
        let provisoire = d.path().join("N").join(PROVISOIRE).join("110").join("a.sav");
        copier_verifie(&source, &cible, &provisoire).unwrap();
        assert_eq!(std::fs::read(&cible).unwrap(), b"partie unique");
        assert!(!source.exists() && !provisoire.exists());
        // Un échec (le dossier de la cible n'existe pas) : l'original reste, rien de provisoire.
        std::fs::write(&source, b"partie").unwrap();
        assert!(copier_verifie(&source, &d.path().join("absent").join("x.sav"), &provisoire).is_err());
        assert!(source.is_file() && !provisoire.exists());
    }

    #[test]
    fn jamais_l_un_dans_l_autre() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("A");
        std::fs::create_dir_all(&a).unwrap();
        assert!(deplacer_contenu("abris", &a, &a.join("B"), &mut |_| {}).is_err());
        assert!(deplacer_contenu("abris", &a.join("B"), &a, &mut |_| {}).is_err());
        assert!(deplacer_contenu("abris", &a, &a, &mut |_| {}).is_err());
    }

    #[test]
    fn un_dossier_absent() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(mesurer(&d.path().join("absent"), "abris"), Contenu::default());
        let b = deplacer_contenu("abris", &d.path().join("absent"), &d.path().join("N"), &mut |_| {}).unwrap();
        assert_eq!((b.deplaces, b.laisses.len()), (0, 1), "illisible : dit, rien déplacé");
    }
}

#[cfg(test)]
mod repetition {
    /// Répétition sur une COPIE des vrais abris de Seb : `FROGTEND_REPETITION=<dossier> cargo test sur_une_copie -- --ignored`
    #[test]
    #[ignore]
    fn sur_une_copie_des_vrais_abris() {
        let d = std::path::PathBuf::from(std::env::var("FROGTEND_REPETITION").unwrap());
        let avant = super::mesurer(&d.join("Ancien"), "abris");
        let b = super::deplacer_contenu("abris", &d.join("Ancien"), &d.join("Nouveau"), &mut |_| {}).unwrap();
        println!("avant {avant:?} ; bilan {b:?}");
        assert_eq!(b.deplaces, avant.fichiers);
        assert!(b.laisses.is_empty());
    }
}
