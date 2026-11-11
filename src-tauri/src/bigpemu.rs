//! BigPEmu (Atari Jaguar et Jaguar CD) : gratuit, code fermé, servi par Firehouse depuis son site officiel (besoin 46).
//!
//! Relevé le 11/11 (manuel officiel richwhitehouse.com/jaguar, `index.php?content=userman`, et BigPEmu 1.221 ouvert
//! SANS jeu dans un dossier temporaire, avec l'accord de Seb) :
//! - l'image du jeu est TOUJOURS le premier argument ; `-localdata` range toutes ses données (réglages
//!   `BigPEmuConfig.bigpcfg`, mémoire des cartouches `game<…>_eeprom.bigpeep`, celle du Jaguar CD, états, captures)
//!   dans `UserData\` à côté du programme ; sans lui : `%APPDATA%\BigPEmu` ;
//! - ses réglages sont du JSON ; `Video.DisplayMode` 0 (celui d'origine) = fenêtre sans bord sur tout l'écran
//!   principal (relevé : 1920×1080, sans barre de titre) ;
//! - une manette connue de sa base, branchée au PREMIER démarrage, est liée seule au joueur 1 (`Device0`, relevé avec
//!   une manette Xbox : `DISPOSITION_XBOX`) ; branchée plus tard, elle ne l'est que si `Input.AutoAssign` vaut 1
//!   (relevé : 0 d'origine ; à 1, la manette branchée est liée au démarrage suivant) ;
//! - il lit les manettes par DirectInput : le bouton Xbox lui est invisible (il reste au menu de Frogtend).
//!
//! Chaque personne a SES parties : `UserData\` devient un lien (jonction) vers `Profils\<profil>\UserData`, comme le
//! `memstick` de PPSSPP. Les données d'avant Frogtend sont COPIÉES chez la première personne qui joue, puis le dossier
//! d'origine est renommé `UserData (avant Frogtend)` : rien n'est effacé.

use crate::erreurs::{Erreur, Resultat};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Le dossier de données de BigPEmu (`-localdata`), à côté du programme.
pub const DOSSIER_DONNEES: &str = "UserData";
/// Son fichier de réglages, dans ce dossier.
pub const FICHIER_REGLAGES: &str = "BigPEmuConfig.bigpcfg";
/// Au-delà, le fichier n'est pas lu (relevé : 48 Ko).
const PLAFOND: u64 = 4 * 1024 * 1024;

/// Une liaison d'une manette dans BigPEmu : (action BigPEmu, bouton/axe DirectInput, sens de l'axe, et la touche à
/// tenir en même temps — gâchette gauche pour le pavé numérique, droite pour le menu de BigPEmu).
pub type Liaison = (usize, u64, f64, Option<(u64, f64)>);

/// Ce que BigPEmu 1.221 lie seul pour une manette Xbox (DirectInput : A0 B1 X2 Y3 LB4 RB5 Vue6 Menu7 L3 8 R3 9,
/// croix = chapeau 134, sticks 128/129 et 131/132, gâchettes = axe 130 : + gauche, − droite). Relevé le 11/11 sur
/// une installation neuve (fixture `neuf-manette-xbox.bigpcfg`) ; les actions sont celles de `STR_EMUBUTTON<n>`.
pub const DISPOSITION_XBOX: [Liaison; 32] = [
    (0, 2, 0.0, None),            // C = X
    (1, 0, 0.0, None),            // B = A
    (2, 1, 0.0, None),            // A = B
    (3, 6, 0.0, None),            // Pause = Vue
    (4, 7, 0.0, None),            // Option = Menu
    (5, 134, 0.0, None),          // croix
    (6, 134, 0.5, None),
    (7, 134, 0.75, None),
    (8, 134, 0.25, None),
    (9, 9, 0.0, Some((130, 1.0))), // pavé 0 = LT + R3
    (10, 2, 0.0, Some((130, 1.0))), // 1 = LT + X
    (11, 3, 0.0, Some((130, 1.0))), // 2 = LT + Y
    (12, 1, 0.0, Some((130, 1.0))), // 3 = LT + B
    (13, 4, 0.0, None),           // 4 = LB
    (14, 3, 0.0, None),           // 5 = Y
    (15, 5, 0.0, None),           // 6 = RB
    (16, 4, 0.0, Some((130, 1.0))), // 7 = LT + LB
    (17, 0, 0.0, Some((130, 1.0))), // 8 = LT + A
    (18, 5, 0.0, Some((130, 1.0))), // 9 = LT + RB
    (19, 8, 0.0, None),           // * = L3
    (20, 9, 0.0, None),           // # = R3
    (21, 128, -1.0, None),        // stick gauche (analogique 0)
    (22, 128, 1.0, None),
    (23, 129, -1.0, None),
    (24, 129, 1.0, None),
    (25, 131, -1.0, None),        // stick droit (analogique 1)
    (26, 131, 1.0, None),
    (27, 132, -1.0, None),
    (28, 132, 1.0, None),
    (37, 7, 0.0, Some((130, -1.0))), // menu de BigPEmu = RT + Menu
    (38, 3, 0.0, Some((130, -1.0))), // avance rapide = RT + Y
    (43, 8, 0.0, Some((130, -1.0))), // surimpression = RT + L3
];

/// Le fichier de réglages de BigPEmu pour une personne (`Profils\<profil>\UserData\…`).
pub fn reglages_du_profil(dossier_profil: &Path) -> PathBuf {
    dossier_profil.join(DOSSIER_DONNEES).join(FICHIER_REGLAGES)
}

fn lire(fichier: &Path) -> Option<String> {
    let m = std::fs::metadata(fichier).ok()?;
    if m.len() > PLAFOND {
        return None;
    }
    std::fs::read_to_string(fichier).ok()
}

/// Les liaisons MANETTE (pas clavier) du joueur 1 (`Device0`), dans l'ordre. `None` : texte illisible.
pub fn liaisons_joueur1(texte: &str) -> Option<Vec<Liaison>> {
    let v: Value = serde_json::from_str(texte).ok()?;
    let liaisons = v["BigPEmuConfig"]["Input"]["Device0"]["Bindings"].as_array()?;
    let mut l = Vec::new();
    for (i, b) in liaisons.iter().enumerate() {
        for t in b["Triggers"].as_array().into_iter().flatten() {
            if t["B_KB"].as_bool() != Some(false) {
                continue;
            }
            let modif = t["M_ID"].as_u64().map(|m| (m, t["M_AH"].as_f64().unwrap_or(0.0)));
            l.push((i, t["B_ID"].as_u64()?, t["B_AH"].as_f64().unwrap_or(0.0), modif));
        }
    }
    Some(l)
}

/// Ce que BigPEmu fait de la manette du joueur 1 chez cette personne.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Joueur1 {
    /// Pas encore de réglages (jamais lancé pour cette personne) : BigPEmu liera seul une manette branchée.
    PasEncore,
    /// Aucune manette liée : le joueur 1 joue au clavier.
    AuClavier,
    /// La disposition d'origine pour une manette Xbox.
    Origine,
    /// Une autre disposition (changée par la personne, une autre manette) : rien de dessiné.
    Autre,
}

pub fn joueur1(dossier_profil: &Path) -> Joueur1 {
    let f = reglages_du_profil(dossier_profil);
    if !f.exists() {
        return Joueur1::PasEncore;
    }
    match lire(&f).as_deref().and_then(liaisons_joueur1) {
        None => Joueur1::Autre,
        Some(l) if l.is_empty() => Joueur1::AuClavier,
        Some(l) => {
            let pareil = l.len() == DISPOSITION_XBOX.len()
                && l.iter().zip(DISPOSITION_XBOX.iter()).all(|(a, b)| {
                    a.0 == b.0 && a.1 == b.1 && (a.2 - b.2).abs() < 1e-6 && match (a.3, b.3) {
                        (None, None) => true,
                        (Some(x), Some(y)) => x.0 == y.0 && (x.1 - y.1).abs() < 1e-6,
                        _ => false,
                    }
                });
            if pareil {
                Joueur1::Origine
            } else {
                Joueur1::Autre
            }
        }
    }
}

/// La marque « AutoAssign déjà posé par Frogtend pour cette personne » : une seule fois, jamais contre son choix
/// (si elle le remet à 0 elle-même, il le reste).
const MARQUE_AUTO: &str = ".frogtend-bigpemu-autoassign";

/// Le joueur 1 encore au clavier : `Input.AutoAssign` passé à 1, pour que BigPEmu lie la manette branchée au
/// lancement (relevé le 11/11). Une seule fois par personne, et seulement si aucune manette n'est liée au joueur 1
/// (jamais par-dessus un choix) ; l'original est copié à l'abri d'abord. Rend vrai si le fichier a été changé.
pub fn lier_la_manette_branchee(emulateur: &Path, dossier_profil: &Path) -> Resultat<bool> {
    let f = reglages_du_profil(dossier_profil);
    if dossier_profil.join(MARQUE_AUTO).exists() {
        return Ok(false);
    }
    let Some(texte) = lire(&f) else { return Ok(false) };
    if liaisons_joueur1(&texte).is_none_or(|l| !l.is_empty()) {
        return Ok(false);
    }
    const AVANT: &str = "\"AutoAssign\": 0,";
    if texte.matches(AVANT).count() != 1 {
        return Ok(false);
    }
    let apres = texte.replacen(AVANT, "\"AutoAssign\": 1,", 1);
    // Le fichier est dans le dossier du profil, sous le dossier de l'émulateur : la copie à l'abri y est faite.
    crate::emulateurs_profils::mettre_a_l_abri(emulateur, &f)?;
    let provisoire = f.with_extension("bigpcfg.frogtend");
    std::fs::write(&provisoire, &apres)?;
    std::fs::rename(&provisoire, &f)?;
    // Vérifier le résultat : relu, du JSON qui dit 1.
    let relu = lire(&f).unwrap_or_default();
    let v: Value = serde_json::from_str(&relu).map_err(|_| Erreur::Disque("Les réglages de BigPEmu relus ne sont plus lisibles.".into()))?;
    let fait = v["BigPEmuConfig"]["Input"]["AutoAssign"].as_i64() == Some(1);
    if fait {
        std::fs::write(dossier_profil.join(MARQUE_AUTO), b"")?;
    }
    Ok(fait)
}

/// La ligne de lancement de BigPEmu, toujours avec `-localdata` (sans lui, ses données iraient dans %APPDATA%,
/// communes à toute la famille) et l'image EN PREMIER (manuel officiel) : ajoutés s'ils manquent (une ligne prise de
/// LaunchBox, ou changée à la main).
pub fn ligne_complete(ligne: &str) -> String {
    let mut l = ligne.trim().to_string();
    if !l.contains("%rom%") {
        l = format!("\"%rom%\" {l}").trim().to_string();
    }
    if !l.split_whitespace().any(|m| m.eq_ignore_ascii_case("-localdata")) {
        l.push_str(" -localdata");
    }
    l
}

/// Les données de BigPEmu de cette personne : `UserData\` relié à `Profils\<profil>\UserData`. Un vrai dossier
/// `UserData` d'avant Frogtend est d'abord COPIÉ chez cette personne si elle n'a encore rien (copie vérifiée), puis
/// renommé `UserData (avant Frogtend)`.
pub fn relier_donnees(emulateur: &Path, dossier_profil: &Path) -> Resultat<()> {
    // Déjà ouvert : changer le lien maintenant lui ferait écrire, en quittant, chez la nouvelle personne.
    if !crate::lancement::processus_du_dossier(emulateur).is_empty() {
        return Err(Erreur::Refus("BigPEmu est déjà ouvert : ferme-le avant de lancer un autre jeu.".into()));
    }
    let cible = dossier_profil.join(DOSSIER_DONNEES);
    let lien = emulateur.join(DOSSIER_DONNEES);
    let est_lien = std::fs::symlink_metadata(&lien).is_ok_and(|m| m.file_type().is_symlink()) || crate::garde::est_un_lien(&lien);
    if lien.is_dir() && !est_lien && !cible.exists() {
        crate::eden::copier_dossier_entier(&lien, &cible)?;
    }
    crate::emulateurs_profils::relier_dossier(emulateur, DOSSIER_DONNEES, &cible, "BigPEmu")
}

#[cfg(test)]
mod tests {
    use super::*;

    const XBOX: &str = include_str!("../fixtures/bigpemu/neuf-manette-xbox.bigpcfg");
    const CLAVIER: &str = include_str!("../fixtures/bigpemu/neuf-clavier.bigpcfg");

    fn profil_avec(texte: &str) -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("Profils").join("Seb");
        std::fs::create_dir_all(p.join(DOSSIER_DONNEES)).unwrap();
        std::fs::write(reglages_du_profil(&p), texte).unwrap();
        d
    }

    #[test]
    fn la_disposition_relevee_est_reconnue() {
        assert_eq!(liaisons_joueur1(XBOX).unwrap(), DISPOSITION_XBOX.to_vec());
        assert_eq!(liaisons_joueur1(CLAVIER).unwrap(), vec![]);
        let d = profil_avec(XBOX);
        let p = d.path().join("Profils").join("Seb");
        assert_eq!(joueur1(&p), Joueur1::Origine);
        // Une touche changée par la personne : autre chose.
        std::fs::write(reglages_du_profil(&p), XBOX.replacen("\"B_ID\": 2,", "\"B_ID\": 3,", 1)).unwrap();
        assert_eq!(joueur1(&p), Joueur1::Autre);
        std::fs::write(reglages_du_profil(&p), CLAVIER).unwrap();
        assert_eq!(joueur1(&p), Joueur1::AuClavier);
        std::fs::write(reglages_du_profil(&p), "pas du json").unwrap();
        assert_eq!(joueur1(&p), Joueur1::Autre);
        assert_eq!(joueur1(&d.path().join("Profils").join("Léa")), Joueur1::PasEncore);
    }

    #[test]
    fn le_joueur_au_clavier_prendra_la_manette_branchee() {
        let d = profil_avec(CLAVIER);
        let p = d.path().join("Profils").join("Seb");
        assert!(lier_la_manette_branchee(d.path(), &p).unwrap());
        let t = std::fs::read_to_string(reglages_du_profil(&p)).unwrap();
        assert!(t.contains("\"AutoAssign\": 1,") && t.len() == CLAVIER.len(), "seule la valeur change");
        assert!(d.path().join(".frogtend-sauvegardes").is_dir(), "l'original est à l'abri");
        // Une fois fait, ou une manette déjà liée : rien. Remis à 0 par la personne : il le reste.
        assert!(!lier_la_manette_branchee(d.path(), &p).unwrap());
        std::fs::write(reglages_du_profil(&p), CLAVIER).unwrap();
        assert!(!lier_la_manette_branchee(d.path(), &p).unwrap());
        assert_eq!(std::fs::read_to_string(reglages_du_profil(&p)).unwrap(), CLAVIER);
        let x = profil_avec(XBOX);
        let px = x.path().join("Profils").join("Seb");
        assert!(!lier_la_manette_branchee(x.path(), &px).unwrap());
        assert_eq!(std::fs::read_to_string(reglages_du_profil(&px)).unwrap(), XBOX);
        // Jamais lancé : rien à écrire (BigPEmu liera seul la manette branchée).
        assert!(!lier_la_manette_branchee(x.path(), &x.path().join("Profils").join("Léa")).unwrap());
    }

    #[test]
    fn la_ligne_garde_l_image_en_premier_et_ses_donnees_a_cote() {
        assert_eq!(ligne_complete("\"%rom%\" -localdata"), "\"%rom%\" -localdata");
        assert_eq!(ligne_complete(""), "\"%rom%\" -localdata");
        assert_eq!(ligne_complete("-noborder"), "\"%rom%\" -noborder -localdata");
        assert_eq!(ligne_complete("\"%rom%\" -nomenubg"), "\"%rom%\" -nomenubg -localdata");
    }

    #[cfg(windows)]
    #[test]
    fn chaque_personne_a_ses_donnees_et_celles_d_avant_sont_gardees() {
        let d = tempfile::tempdir().unwrap();
        let e = d.path();
        // Des données d'avant Frogtend (BigPEmu lancé par la voie générique).
        std::fs::create_dir_all(e.join("UserData")).unwrap();
        std::fs::write(e.join("UserData").join("game0123456789ABCDEF_eeprom.bigpeep"), b"partie").unwrap();
        let seb = e.join("Profils").join("Seb");
        relier_donnees(e, &seb).unwrap();
        assert_eq!(std::fs::read(e.join("UserData").join("game0123456789ABCDEF_eeprom.bigpeep")).unwrap(), b"partie", "par le lien");
        assert_eq!(std::fs::read(seb.join("UserData").join("game0123456789ABCDEF_eeprom.bigpeep")).unwrap(), b"partie");
        assert_eq!(std::fs::read(e.join("UserData (avant Frogtend)").join("game0123456789ABCDEF_eeprom.bigpeep")).unwrap(), b"partie", "rien d'effacé");
        // Une autre personne : son dossier à elle, vide ; celui de Seb intact.
        let lea = e.join("Profils").join("Léa");
        relier_donnees(e, &lea).unwrap();
        assert!(lea.join("UserData").is_dir() && std::fs::read_dir(e.join("UserData")).unwrap().next().is_none());
        assert!(seb.join("UserData").join("game0123456789ABCDEF_eeprom.bigpeep").is_file());
        relier_donnees(e, &seb).unwrap();
        assert!(e.join("UserData").join("game0123456789ABCDEF_eeprom.bigpeep").is_file());
    }
}
