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

/// Réécrit les réglages de la personne (valeur JSON complète) : l'original mis à l'abri d'abord, un fichier provisoire
/// puis un renommage (jamais un fichier à moitié écrit), relu ensuite. BigPEmu relit un fichier aux clés réordonnées
/// et le réécrit à sa façon (relevé du 11/11).
fn ecrire_reglages(emulateur: &Path, f: &Path, v: &Value) -> Resultat<Value> {
    let texte = serde_json::to_string_pretty(v).map_err(|e| Erreur::Disque(format!("Réglages de BigPEmu impossibles à écrire ({e}).")))?;
    crate::emulateurs_profils::mettre_a_l_abri(emulateur, f)?;
    let provisoire = f.with_extension("bigpcfg.frogtend");
    std::fs::write(&provisoire, &texte)?;
    std::fs::rename(&provisoire, f)?;
    let relu = lire(f).unwrap_or_default();
    serde_json::from_str(&relu).map_err(|_| Erreur::Disque("Les réglages de BigPEmu relus ne sont plus lisibles.".into()))
}

fn lire_json(f: &Path) -> Option<Value> {
    serde_json::from_str(&lire(f)?).ok()
}

// ---------------------------------------------------------------------------------------------------------------
// Sauvegarde et chargement rapides, capture (✅ Seb 11/11 : « tout ce que l'émulateur propose, dans notre menu »).
// Relevé le 11/11 : `Input.Device0.Bindings[40]` = Save State, `[41]` = Load State, `[42]` = Screenshot
// (`STR_EMUBUTTON40..42`), vides d'origine ; un déclencheur clavier `{"B_KB": true, "B_ID": <code DIK>, "B_AH": 0.0}`
// est gardé par BigPEmu. Il sauve et charge dans son emplacement choisi (`System.StateSlot`).

/// Les touches que Frogtend lie aux actions de BigPEmu : (action, code DirectInput) — F13 sauver, F14 charger, F15
/// capture (DIK_F13 = 0x64…, les codes de balayage envoyés par le menu, `menu_jeu::code`).
pub const RACCOURCIS: [(usize, u64); 3] = [(40, 100), (41, 101), (42, 102)];


/// Ce déclencheur est-il cette touche du clavier, seule (sans touche tenue) ?
fn est_la_touche(t: &Value, code: u64) -> bool {
    t["B_KB"].as_bool() == Some(true) && t["B_ID"].as_u64() == Some(code) && t.get("M_ID").is_none()
}

fn a_la_touche(liaison: &Value, code: u64) -> bool {
    liaison["Triggers"].as_array().is_some_and(|l| l.iter().any(|t| est_la_touche(t, code)))
}

/// Les trois touches sont-elles liées à leurs actions dans ces réglages ?
fn raccourcis_dans(v: &Value) -> bool {
    let b = &v["BigPEmuConfig"]["Input"]["Device0"]["Bindings"];
    RACCOURCIS.iter().all(|(a, code)| a_la_touche(&b[*a], *code))
}

/// Les touches de Frogtend sont-elles en place chez cette personne ? (Sinon le menu ne propose ni sauver, ni charger,
/// ni capturer : la touche n'arriverait à rien.)
pub fn raccourcis_poses(dossier_profil: &Path) -> bool {
    lire_json(&reglages_du_profil(dossier_profil)).is_some_and(|v| raccourcis_dans(&v))
}

/// Lie F13, F14 et F15 à sauver, charger et capturer, AJOUTÉES à ce que la personne a déjà lié (jamais rien de
/// retiré) ; une touche déjà prise par une autre action de BigPEmu n'est pas posée (elle ferait les deux). Rien avant le
/// premier lancement (pas encore de réglages : BigPEmu les crée en quittant). Rend vrai si les trois sont en place.
pub fn poser_raccourcis(emulateur: &Path, dossier_profil: &Path) -> Resultat<bool> {
    let f = reglages_du_profil(dossier_profil);
    let Some(mut v) = lire_json(&f) else { return Ok(false) };
    let Some(b) = v["BigPEmuConfig"]["Input"]["Device0"]["Bindings"].as_array_mut() else { return Ok(false) };
    if b.len() <= RACCOURCIS.iter().map(|(a, _)| *a).max().unwrap_or(0) {
        return Ok(false);
    }
    let mut change = false;
    for (action, code) in RACCOURCIS {
        if a_la_touche(&b[action], code) {
            continue;
        }
        if b.iter().enumerate().any(|(i, l)| i != action && a_la_touche(l, code)) {
            continue;
        }
        let Some(l) = b[action]["Triggers"].as_array_mut() else { continue };
        l.push(serde_json::json!({"B_KB": true, "B_ID": code, "B_AH": 0.0}));
        change = true;
    }
    if !change {
        return Ok(raccourcis_dans(&v));
    }
    Ok(raccourcis_dans(&ecrire_reglages(emulateur, &f, &v)?))
}

// ---------------------------------------------------------------------------------------------------------------
// Les effets d'écran. Relevé le 11/11 : `BigPEmuConfig.ScreenEffect` AU PREMIER NIVEAU, le nom du fichier
// (`crt-hyllian.dstack`, sous `Data\ScreenEffects`) ; absent = aucun ; lu au démarrage seulement (pas de changement
// à chaud : le choix fait dans le menu vaut au prochain lancement).

/// Le préfixe des identifiants d'effets de BigPEmu dans les réglages du profil (`image`, partagé avec RetroArch) :
/// un choix pour la console « Atari Jaguar » ne se confond jamais avec un effet de RetroArch.
pub const PREFIXE_EFFET: &str = "bigpemu:";

/// Les effets livrés avec BigPEmu 1.221 que Frogtend propose (`shadertoy` : non relevé, pas proposé).
pub const EFFETS: &[&str] = &["cheesy_scanlines", "crt-hyllian", "gamma", "liz", "oh_my_gauss", "warping", "xbr-lv2", "xbr-lv3-multipass"];

/// Les effets présents dans ce BigPEmu, en identifiants du profil (`bigpemu:crt-hyllian`).
pub fn effets_presents(emulateur: &Path) -> Vec<String> {
    EFFETS
        .iter()
        .filter(|e| emulateur.join("Data").join("ScreenEffects").join(format!("{e}.dstack")).is_file())
        .map(|e| format!("{PREFIXE_EFFET}{e}"))
        .collect()
}

/// La marque de l'effet posé par Frogtend : `{"pose": <valeur écrite>, "avant": <celle d'avant>}`. Sans elle, un
/// `ScreenEffect` présent est celui de la personne.
const MARQUE_EFFET: &str = ".frogtend-bigpemu-effet";

/// Pose l'effet choisi (`aucun`, `bigpemu:<effet>` ; `None` ou un effet d'un autre émulateur : pas de choix) avant le
/// lancement. Sans choix, l'effet que Frogtend avait posé pour un autre jeu est retiré (celui d'avant revient) ; celui
/// que la personne a réglé dans BigPEmu reste. Rend l'effet en place ensuite (identifiant du profil, `None` : aucun ou
/// un effet hors de la liste).
pub fn poser_effet(emulateur: &Path, dossier_profil: &Path, choix: Option<&str>) -> Resultat<Option<String>> {
    let f = reglages_du_profil(dossier_profil);
    let Some(mut v) = lire_json(&f) else { return Ok(None) };
    let actuel = v["BigPEmuConfig"]["ScreenEffect"].as_str().filter(|s| !s.is_empty()).map(String::from);
    let marque_f = dossier_profil.join(MARQUE_EFFET);
    let marque = lire_json(&marque_f);
    // Ce qui était là avant Frogtend : la valeur notée si l'effet en place est encore le nôtre, sinon l'effet en place.
    let base = match &marque {
        Some(m) if m["pose"].as_str().map(String::from) == actuel => m["avant"].as_str().map(String::from),
        _ => actuel.clone(),
    };
    let voulu = match choix {
        Some("aucun") => None,
        Some(c) => match c.strip_prefix(PREFIXE_EFFET).filter(|e| EFFETS.contains(e)) {
            Some(e) if emulateur.join("Data").join("ScreenEffects").join(format!("{e}.dstack")).is_file() => Some(format!("{e}.dstack")),
            _ => base.clone(),
        },
        None => base.clone(),
    };
    // La marque AVANT les réglages : un effet posé par Frogtend n'est jamais pris ensuite pour celui de la personne.
    if voulu != base {
        std::fs::write(&marque_f, serde_json::json!({"pose": voulu, "avant": base}).to_string())?;
    }
    if voulu != actuel {
        let c = v["BigPEmuConfig"].as_object_mut().ok_or_else(|| Erreur::Disque("Réglages de BigPEmu inattendus.".into()))?;
        match &voulu {
            Some(e) => c.insert("ScreenEffect".into(), Value::String(e.clone())),
            None => c.remove("ScreenEffect"),
        };
        let relu = ecrire_reglages(emulateur, &f, &v)?;
        if relu["BigPEmuConfig"]["ScreenEffect"].as_str().filter(|s| !s.is_empty()).map(String::from) != voulu {
            return Err(Erreur::Disque("L'effet d'écran de BigPEmu n'a pas été écrit.".into()));
        }
    }
    if voulu == base {
        let _ = std::fs::remove_file(&marque_f);
    }
    Ok(voulu.and_then(|e| e.strip_suffix(".dstack").filter(|e| EFFETS.contains(e)).map(|e| format!("{PREFIXE_EFFET}{e}"))))
}

// ---------------------------------------------------------------------------------------------------------------
// Les triches. Relevé le 11/11 (BigPEmu 1.221, `Scripts\cheat_codes.c` livré, et Bubsy chez Seb) : le module
// `cheat_codes` (`BigPEmuConfig.ScriptsEnabled`) lit `Scripts\cheat_codes.json` (`DickGenie.<empreinte>.Codes[]`) et
// range chaque triche dans `BigPEmuConfig.ScriptData_CheatSetting_<empreinte>_<nom, espaces en _>`, un uint32 petit-
// boutiste en hexadécimal (`01000000` = cochée). L'empreinte est le FNV-1a 64 bits de l'image chargée : celui du
// `.j64` (dans son zip) calculé par Frogtend est le même que celui de BigPEmu (Bubsy : 5c0c7827948f0f27). Le module
// ouvre SA fenêtre de triches (déjà cochées) à chaque chargement d'un jeu qui en a. Lus au démarrage seulement : un
// choix fait dans le menu vaut au prochain lancement.

/// Le nom du module de triches dans `ScriptsEnabled` (relevé chez Seb ; `cheat_codes.bigpcvm` est aussi gardé par
/// BigPEmu, reconnu sans être écrit).
const MODULE_TRICHES: &str = "cheat_codes";
/// Les choix faits dans le menu, à poser au prochain lancement : `{"<empreinte>_<nom>": true|false}`.
const TRICHES_A_POSER: &str = ".frogtend-bigpemu-triches.json";
/// Une cartouche Jaguar fait au plus 6 Mo ; au-delà, ce n'est pas une cartouche.
const PLAFOND_CARTOUCHE: u64 = 32 * 1024 * 1024;
/// Les extensions d'une cartouche (ou d'un programme) Jaguar (relevé dans BigPEmu : `.j64;.cof;.rom;.jag;.abs`).
const EXT_CARTOUCHE: &[&str] = &["j64", "rom", "jag", "abs", "cof"];

/// FNV-1a 64 bits.
pub fn fnv1a64(octets: &[u8]) -> u64 {
    octets.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, o| (h ^ u64::from(*o)).wrapping_mul(0x0000_0100_0000_01b3))
}

/// L'empreinte d'un jeu en CARTOUCHE, comme BigPEmu la calcule (seize chiffres hexadécimaux minuscules) : le fichier
/// lui-même, ou l'unique cartouche de son zip. Un jeu CD (`.cue`, `.cdi`…) : `None` (empreinte pas encore relevée).
pub fn empreinte_jeu(rom: &Path) -> Option<String> {
    let ext = rom.extension()?.to_string_lossy().to_lowercase();
    let octets = if EXT_CARTOUCHE.contains(&ext.as_str()) {
        if std::fs::metadata(rom).ok()?.len() > PLAFOND_CARTOUCHE {
            return None;
        }
        std::fs::read(rom).ok()?
    } else if ext == "zip" {
        let mut z = zip::ZipArchive::new(std::fs::File::open(rom).ok()?).ok()?;
        let noms: Vec<String> = z
            .file_names()
            .filter(|n| Path::new(n).extension().is_some_and(|e| EXT_CARTOUCHE.contains(&e.to_string_lossy().to_lowercase().as_str())))
            .map(String::from)
            .collect();
        let [nom] = noms.as_slice() else { return None };
        let mut e = z.by_name(nom).ok()?;
        if e.size() > PLAFOND_CARTOUCHE {
            return None;
        }
        use std::io::Read;
        let mut o = Vec::with_capacity(e.size() as usize);
        (&mut e).take(PLAFOND_CARTOUCHE + 1).read_to_end(&mut o).ok()?;
        o
    } else {
        return None;
    };
    Some(format!("{:016x}", fnv1a64(&octets)))
}

/// Le nom d'une triche dans la clé de BigPEmu : espaces en `_` (relevé). Un nom avec d'autres signes (`-`, `/`,
/// parenthèses : 11 sur 159) : `None` — leur forme n'est pas relevée, la triche n'est pas proposée.
fn nom_de_cle(nom: &str) -> Option<String> {
    (!nom.is_empty() && nom.chars().all(|c| c.is_ascii_alphanumeric() || c == ' ')).then(|| nom.replace(' ', "_"))
}

/// Une triche de BigPEmu proposée dans le menu : son nom (anglais, celui du fichier), sa clé, et si elle est cochée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TricheBp {
    pub titre: String,
    pub cle: String,
    pub active: bool,
}

fn valeur_cochee(v: &Value) -> bool {
    v.as_str().is_some_and(|s| s.chars().any(|c| c != '0'))
}

/// Les triches livrées pour ce jeu, avec leur état : le choix fait dans le menu s'il attend le prochain lancement,
/// sinon celui des réglages. Seules les triches à une valeur (cochée ou non) et au nom relevé sont proposées.
pub fn triches_du_jeu(emulateur: &Path, dossier_profil: &Path, empreinte: &str) -> Vec<TricheBp> {
    let Some(base) = lire_json(&emulateur.join("Scripts").join("cheat_codes.json")) else { return vec![] };
    let reglages = lire_json(&reglages_du_profil(dossier_profil)).unwrap_or(Value::Null);
    let a_poser = lire_json(&dossier_profil.join(TRICHES_A_POSER)).unwrap_or(Value::Null);
    base["DickGenie"][empreinte]["Codes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| c["Code"].is_string() && c.get("CodeLabels").is_none())
        .filter_map(|c| {
            let titre = c["Name"].as_str()?.to_string();
            let cle = format!("{empreinte}_{}", nom_de_cle(&titre)?);
            let active = a_poser[&cle].as_bool().unwrap_or_else(|| valeur_cochee(&reglages["BigPEmuConfig"][format!("ScriptData_CheatSetting_{cle}")]));
            Some(TricheBp { titre, cle, active })
        })
        .take(crate::menu_jeu::TRICHES_MAX)
        .collect()
}

/// Note le choix fait dans le menu (posé au prochain lancement : BigPEmu ouvert réécrirait ses réglages en quittant).
pub fn choisir_triche(dossier_profil: &Path, cle: &str, active: bool) -> Resultat<()> {
    let f = dossier_profil.join(TRICHES_A_POSER);
    let mut v = lire_json(&f).filter(Value::is_object).unwrap_or_else(|| serde_json::json!({}));
    v[cle] = Value::Bool(active);
    let provisoire = f.with_extension("json.frogtend");
    std::fs::write(&provisoire, v.to_string())?;
    std::fs::rename(&provisoire, &f)?;
    Ok(())
}

/// La marque « le module de triches a été activé par Frogtend » (gardée pour mémoire : depuis la 0.187.0, Frogtend tient
/// le module dans tous les cas, ✅ Seb 11/11).
const MARQUE_MODULE: &str = ".frogtend-bigpemu-module";

/// Ce que le lancement va écrire pour les triches (rien n'est encore écrit).
pub struct TrichesPrevues {
    reglages: Value,
    a_poser: Value,
    change: bool,
    /// Une triche est cochée pour ce jeu.
    pub actives: bool,
    /// `Some(vrai)` : Frogtend active le module (marque posée) ; `Some(faux)` : il le retire.
    module: Option<bool>,
}

/// Prévoit, SANS RIEN ÉCRIRE, les triches du prochain lancement : les choix du menu posés, et le module actif seulement
/// si ce jeu a une triche cochée (sinon sa fenêtre s'ouvrirait pour rien), coupé sinon (jeu CD compris). `None` : pas
/// encore de réglages (les choix attendent).
pub fn prevoir_triches(dossier_profil: &Path, empreinte: Option<&str>) -> Option<TrichesPrevues> {
    let mut v = lire_json(&reglages_du_profil(dossier_profil))?;
    let a_poser = lire_json(&dossier_profil.join(TRICHES_A_POSER)).unwrap_or(Value::Null);
    let c = v["BigPEmuConfig"].as_object_mut()?;
    let avant = c.clone();
    for (cle, active) in a_poser.as_object().into_iter().flatten() {
        if let Some(a) = active.as_bool() {
            c.insert(format!("ScriptData_CheatSetting_{cle}"), Value::String(if a { "01000000" } else { "00000000" }.into()));
        }
    }
    let actives = empreinte.is_some_and(|e| {
        let prefixe = format!("ScriptData_CheatSetting_{e}_");
        c.iter().any(|(k, val)| k.starts_with(&prefixe) && valeur_cochee(val))
    });
    // Le module de triches, tenu par Frogtend (✅ Seb 11/11 : « oui, tant que cela garde les choix ») : actif
    // seulement pour un jeu dont une triche est cochée, coupé pour les autres (jeux CD compris) — même activé par la
    // personne dans BigPEmu. Il montre sinon sa fenêtre à CHAQUE jeu qui a des triches (`cheat_codes.c` : les codes ne
    // s'appliquent qu'à sa fermeture) ; les triches se choisissent dans le menu de Frogtend. Les choix eux-mêmes
    // (`ScriptData_CheatSetting_*`) ne sont jamais touchés ici.
    let mut module = None;
    let modules = c.entry("ScriptsEnabled").or_insert_with(|| Value::Array(vec![]));
    if let Some(l) = modules.as_array_mut() {
        let est_le_module = |m: &Value| m.as_str().is_some_and(|s| s.eq_ignore_ascii_case(MODULE_TRICHES) || s.eq_ignore_ascii_case("cheat_codes.bigpcvm"));
        if actives && !l.iter().any(est_le_module) {
            l.push(Value::String(MODULE_TRICHES.into()));
            module = Some(true);
        } else if !actives && l.iter().any(est_le_module) {
            l.retain(|m| !est_le_module(m));
            module = Some(false);
        }
    }
    let change = *c != avant;
    Some(TrichesPrevues { reglages: v, a_poser, change, actives, module })
}

/// Écrit ce qui a été prévu (BigPEmu fermé), relu ; les choix posés sont alors oubliés.
pub fn ecrire_triches(emulateur: &Path, dossier_profil: &Path, prevu: TrichesPrevues) -> Resultat<()> {
    let marque = dossier_profil.join(MARQUE_MODULE);
    // La marque AVANT les réglages : jamais un module activé par Frogtend pris ensuite pour celui de la personne.
    if prevu.module == Some(true) {
        std::fs::write(&marque, b"")?;
    }
    if prevu.change {
        let relu = ecrire_reglages(emulateur, &reglages_du_profil(dossier_profil), &prevu.reglages)?;
        let rc = &relu["BigPEmuConfig"];
        let bien = prevu.a_poser.as_object().into_iter().flatten().all(|(cle, a)| a.as_bool().is_none_or(|a| valeur_cochee(&rc[format!("ScriptData_CheatSetting_{cle}")]) == a));
        if !bien {
            return Err(Erreur::Disque("Les triches de BigPEmu n'ont pas été écrites.".into()));
        }
    }
    if prevu.module == Some(false) {
        let _ = std::fs::remove_file(&marque);
    }
    if prevu.a_poser.is_object() {
        let _ = std::fs::remove_file(dossier_profil.join(TRICHES_A_POSER));
    }
    Ok(())
}

/// Prévoit puis écrit (voir `prevoir_triches`) ; rend vrai si une triche est cochée pour ce jeu.
pub fn appliquer_triches(emulateur: &Path, dossier_profil: &Path, empreinte: Option<&str>) -> Resultat<bool> {
    let Some(prevu) = prevoir_triches(dossier_profil, empreinte) else { return Ok(false) };
    let actives = prevu.actives;
    ecrire_triches(emulateur, dossier_profil, prevu)?;
    Ok(actives)
}

/// Les fichiers de ce jeu dans les données de la personne (mémoire de la cartouche, du Memory Track, états…) : ceux
/// dont le nom porte son empreinte — à mettre à l'abri avant de jouer avec une triche.
pub fn fichiers_du_jeu(dossier_profil: &Path, empreinte: &str) -> Vec<PathBuf> {
    let e = empreinte.to_lowercase();
    std::fs::read_dir(dossier_profil.join(DOSSIER_DONNEES))
        .into_iter()
        .flatten()
        .flatten()
        .map(|x| x.path())
        .filter(|p| p.is_file())
        .filter(|p| p.file_name().is_some_and(|n| {
            let n = n.to_string_lossy().to_lowercase();
            // Ni les copies de Frogtend (abris, sauvegardes rapides de côté ou précédentes).
            n.contains(&e) && !n.contains(".avant-triche-") && !n.contains(".frogtend-")
        }))
        .collect()
}

/// Ce que Frogtend sait d'une partie BigPEmu avant de la lancer (pour le menu en jeu).
#[derive(Debug, Default)]
pub struct Lancement {
    /// Les effets proposés, et celui en place (identifiants du profil).
    pub effets: Vec<String>,
    pub effet: Option<String>,
    /// L'effet, les triches n'ont pas pu être posés (dit au journal ; le jeu se lance quand même).
    pub erreur_effet: Option<String>,
    pub erreur_triches: Option<String>,
    /// Une sauvegarde rapide coupée la fois d'avant n'a pas pu être remise en place.
    pub erreur_etats: Option<String>,
    /// Le module de triches est actif pour ce jeu avec une triche cochée : BigPEmu ouvrira sa fenêtre de triches au
    /// démarrage (laissée à la personne, Frogtend prévient). Faux sans réglages lisibles (premier lancement d'un profil) : rien n'a été posé.
    pub fenetre_triches: bool,
    pub empreinte: Option<String>,
    pub triches: Vec<TricheBp>,
    /// Des triches sont cochées pour ce jeu : ses fichiers ont été mis à l'abri avant.
    pub abri_fait: bool,
    /// F13 / F14 / F15 en place (sinon le menu ne propose ni sauver, ni charger, ni capturer).
    pub raccourcis: bool,
    /// Le dossier du profil de la personne qui joue (où les choix du menu attendent le prochain lancement).
    pub dossier_profil: PathBuf,
    /// Les emplacements de sauvegarde rapide (`None` : BigPEmu sauve seul, dans son emplacement — « Increment on
    /// Save », ou une série par jeu pour un jeu sans empreinte).
    pub etats: Option<EtatsBp>,
}

/// Avant le lancement (BigPEmu fermé, `UserData\` déjà relié au profil) : l'effet choisi, les triches choisies dans le
/// menu (le jeu et ses parties mis à l'abri d'abord s'il en a de cochées — une copie ratée : `Err`, rien ne se
/// lance), et ce que le menu proposera.
pub fn preparer_partie(emulateur: &Path, dossier_profil: &Path, rom: Option<&Path>, choix_effet: Option<&str>, quand: u64) -> Resultat<Lancement> {
    let mut l = Lancement { effets: effets_presents(emulateur), dossier_profil: dossier_profil.to_path_buf(), ..Default::default() };
    match poser_effet(emulateur, dossier_profil, choix_effet) {
        Ok(e) => l.effet = e,
        Err(e) => l.erreur_effet = Some(format!("{e:?}")),
    }
    l.empreinte = rom.and_then(empreinte_jeu);
    // Une sauvegarde rapide coupée la fois d'avant (Frogtend arrêté au milieu) : tout est remis en place AVANT le reste
    // (la mise à l'abri des triches copierait sinon un fichier de passage) ; un échec : pas d'emplacements cette
    // partie (jamais une opération par-dessus une autre), dit au journal.
    let reprise = reprendre_operations_coupees(dossier_profil, quand);
    if let Err(e) = &reprise {
        l.erreur_etats = Some(format!("{e:?}"));
    }
    // La partie à l'abri AVANT d'écrire la moindre triche : une copie ratée, rien n'est écrit (le choix attend) et
    // rien ne se lance. Une écriture ratée ensuite : dit au journal, on joue sans le changement.
    let prevu = prevoir_triches(dossier_profil, l.empreinte.as_deref());
    if let (true, Some(e)) = (prevu.as_ref().is_some_and(|x| x.actives), l.empreinte.as_deref()) {
        crate::menu_jeu::mettre_a_l_abri_fichiers(&fichiers_du_jeu(dossier_profil, e), quand).map_err(Erreur::Refus)?;
        l.abri_fait = true;
    }
    if let Some(x) = prevu {
        let actives = x.actives;
        match ecrire_triches(emulateur, dossier_profil, x) {
            // Le module est posé (ou l'était déjà) pour une triche cochée : BigPEmu montrera sa fenêtre.
            Ok(()) => l.fenetre_triches = actives,
            Err(e) => l.erreur_triches = Some(format!("{e:?}")),
        }
    }
    l.triches = l.empreinte.as_deref().map(|e| triches_du_jeu(emulateur, dossier_profil, e)).unwrap_or_default();
    l.raccourcis = raccourcis_poses(dossier_profil);
    l.etats = etats_de_la_partie(dossier_profil, l.empreinte.as_deref()).filter(|_| reprise.is_ok());
    Ok(l)
}

// ---------------------------------------------------------------------------------------------------------------
// Les emplacements de sauvegarde rapide (✅ Seb 11/11 : « le même menu que la SNES », emplacements 1 à 10 PAR JEU).
// BigPEmu 1.221 n'a AUCUNE action pour changer d'emplacement en jeu (ses 51 actions, `STR_EMUBUTTON0..50` de
// `strings_en.txt`) : il sauve et charge dans `System.StateSlot`, lu au démarrage. Noms relevés dans le programme :
// `state%03i.bigpstate`, ou `game%016llX_state%03i.bigpstate` avec « Per-Title States » (`PerGameSlots`).
// Les 10 emplacements de Frogtend portent TOUJOURS le nom par jeu (`game<EMPREINTE>_stateNNN`) : d'origine, ceux de
// BigPEmu sont communs à tous les jeux (relecture de l'expert : une partie d'un jeu en remplaçait une d'un autre).
// Chaque opération passe par le fichier de BigPEmu (le « passage »), mis de côté le temps de l'opération puis
// remis. Rien n'est jamais effacé, sauf la copie de passage d'un chargement restée identique à son emplacement.

const EXT_ETAT: &str = "bigpstate";
const DE_COTE: &str = "frogtend-de-cote";
const PRECEDENT: &str = "frogtend-precedent";

/// Où sont les sauvegardes rapides de ce jeu, et le fichier où BigPEmu sauve et charge (le passage).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EtatsBp {
    pub dossier: PathBuf,
    /// L'empreinte du jeu, en majuscules (comme BigPEmu l'écrit).
    pub empreinte: String,
    pub passage: PathBuf,
}

fn suffixe(f: &Path, s: &str) -> PathBuf {
    PathBuf::from(format!("{}.{s}", f.to_string_lossy()))
}

fn maintenant_ms() -> u128 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)
}

impl EtatsBp {
    pub fn fichier(&self, n: u8) -> PathBuf {
        self.dossier.join(format!("game{}_state{n:03}.{EXT_ETAT}", self.empreinte))
    }
    fn de_cote(&self) -> PathBuf {
        suffixe(&self.passage, DE_COTE)
    }
    /// Les emplacements du menu (0 à 9) et la date de leur sauvegarde (`None` : vide).
    pub fn emplacements(&self) -> Vec<(u8, Option<u64>)> {
        (0..=crate::menu_jeu::EMPLACEMENT_MAX).map(|n| (n, crate::menu_jeu::date_de(&self.fichier(n)))).collect()
    }
}

/// Les sauvegardes rapides de ce jeu, d'après les réglages de la personne (absents : ceux d'origine, emplacement 0,
/// une série commune). `None` : pas d'empreinte (jeu CD), ou « Increment on Save » (BigPEmu change d'emplacement
/// seul à chaque sauvegarde).
pub fn etats_de_la_partie(dossier_profil: &Path, empreinte: Option<&str>) -> Option<EtatsBp> {
    let empreinte = empreinte?.to_uppercase();
    let v = lire_json(&reglages_du_profil(dossier_profil)).unwrap_or(Value::Null);
    let s = &v["BigPEmuConfig"]["System"];
    let n = |k: &str| s[k].as_u64().unwrap_or(0);
    if n("SaveAutoIncr") != 0 {
        return None;
    }
    let slot = n("StateSlot");
    if slot > 999 {
        return None;
    }
    let dossier = dossier_profil.join(DOSSIER_DONNEES);
    let nom = if n("PerGameSlots") != 0 { format!("game{empreinte}_state{slot:03}.{EXT_ETAT}") } else { format!("state{slot:03}.{EXT_ETAT}") };
    Some(EtatsBp { passage: dossier.join(nom), dossier, empreinte })
}

/// Une opération coupée (Frogtend arrêté au milieu) : chaque fichier mis de côté (`*.frogtend-de-cote`, quel que soit
/// le jeu) revient à sa place ; ce qui occupe sa place est gardé à côté (`.frogtend-recupere-<quand>`). Rend le
/// nombre de fichiers remis.
pub fn reprendre_operations_coupees(dossier_profil: &Path, quand: u64) -> Resultat<usize> {
    let mut n = 0;
    let fin = format!(".{DE_COTE}");
    for cote in std::fs::read_dir(dossier_profil.join(DOSSIER_DONNEES)).into_iter().flatten().flatten().map(|x| x.path()) {
        let nom = cote.to_string_lossy().to_string();
        let Some(original) = nom.strip_suffix(&fin).map(PathBuf::from) else { continue };
        if !cote.is_file() {
            continue;
        }
        if original.exists() {
            std::fs::rename(&original, suffixe(&original, &format!("frogtend-recupere-{quand}-{}", maintenant_ms())))?;
        }
        std::fs::rename(&cote, &original)?;
        n += 1;
    }
    Ok(n)
}

/// Le fichier peut-il être ouvert seul (BigPEmu l'a fermé) ?
fn libre(f: &Path) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        std::fs::OpenOptions::new().read(true).share_mode(0).open(f).is_ok()
    }
    #[cfg(not(windows))]
    {
        f.is_file()
    }
}

/// Le fichier existe, non vide, de taille stable entre deux regards, et fermé par BigPEmu.
fn attendre_fichier(f: &Path, delai: std::time::Duration) -> bool {
    let debut = std::time::Instant::now();
    let mut avant: Option<u64> = None;
    while debut.elapsed() < delai {
        let t = std::fs::metadata(f).ok().map(|m| m.len()).filter(|t| *t > 0);
        if t.is_some() && t == avant && libre(f) {
            return true;
        }
        avant = t;
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    false
}

/// Un renommage réessayé (BigPEmu peut tenir le fichier un instant).
fn renommer(de: &Path, vers: &Path) -> std::io::Result<()> {
    let mut r = std::fs::rename(de, vers);
    for _ in 0..10 {
        if r.is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        r = std::fs::rename(de, vers);
    }
    r
}

/// Les sauvegardes rapides du dossier, avec leur taille et leur date (pour voir laquelle BigPEmu a écrite).
fn releve(dossier: &Path) -> Vec<(PathBuf, Option<(u64, std::time::SystemTime)>)> {
    std::fs::read_dir(dossier)
        .into_iter()
        .flatten()
        .flatten()
        .map(|x| x.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case(EXT_ETAT)))
        .map(|p| {
            let m = std::fs::metadata(&p).ok().and_then(|m| Some((m.len(), m.modified().ok()?)));
            (p, m)
        })
        .collect()
}

/// Le fichier du passage revient. Un fichier arrivé entre-temps à sa place est gardé à côté (nom unique) ; s'il ne
/// peut pas l'être, rien n'est remis (la reprise du prochain lancement s'en chargera).
fn remettre(e: &EtatsBp, avait: bool) {
    if !avait {
        return;
    }
    if e.passage.exists() && std::fs::rename(&e.passage, suffixe(&e.passage, &format!("frogtend-tardif-{}", maintenant_ms()))).is_err() {
        return;
    }
    if !e.passage.exists() {
        let _ = std::fs::rename(e.de_cote(), &e.passage);
    }
}

fn mettre_de_cote(e: &EtatsBp) -> Resultat<bool> {
    if e.de_cote().exists() {
        return Err(Erreur::Refus("Une sauvegarde rapide précédente n'a pas fini : quitte le jeu et relance-le, Frogtend remettra tout en place.".into()));
    }
    if !e.passage.exists() {
        return Ok(false);
    }
    std::fs::rename(&e.passage, e.de_cote()).map_err(|er| Erreur::Refus(format!("La sauvegarde de BigPEmu n'a pas pu être mise de côté ({er}) : rien n'a été fait.")))?;
    Ok(true)
}

/// Sauver dans l'emplacement `n` : le passage mis de côté, `envoyer` (la touche de BigPEmu), la nouvelle sauvegarde
/// attendue (au plus `delai`) puis rangée dans `n` (l'ancienne de `n` copiée en `.frogtend-precedent` d'abord), le
/// passage remis.
pub fn sauver(e: &EtatsBp, n: u8, envoyer: impl FnOnce() -> Resultat<()>, delai: std::time::Duration) -> Resultat<()> {
    let avait = mettre_de_cote(e)?;
    let avant = releve(&e.dossier);
    // Chaque emplacement du jeu copié à l'abri d'abord (si BigPEmu a changé d'emplacement dans son propre menu, F13
    // écrirait dans un autre : son ancien contenu deviendra sa « précédente ») ; une copie ratée : rien n'est envoyé.
    let abri = |f: &Path| suffixe(f, "frogtend-abri");
    let mut abris: Vec<PathBuf> = Vec::new();
    for f in (0..=crate::menu_jeu::EMPLACEMENT_MAX).map(|k| e.fichier(k)).filter(|f| f.is_file() && *f != e.passage) {
        if let Err(er) = std::fs::copy(&f, abri(&f)) {
            for g in &abris {
                let _ = std::fs::remove_file(abri(g));
            }
            remettre(e, avait);
            return Err(Erreur::Refus(format!("Tes sauvegardes n'ont pas pu être mises à l'abri ({er}) : rien n'a été sauvé.")));
        }
        abris.push(f);
    }
    // Après l'envoi : la copie d'un emplacement réécrit par BigPEmu devient sa « précédente », les autres partent.
    let ranger_abris = || {
        for g in &abris {
            let change = releve(&e.dossier).into_iter().find(|(f, _)| f == g).map(|(_, m)| m) != avant.iter().find(|(f, _)| f == g).map(|(_, m)| *m);
            if change {
                let _ = renommer(&abri(g), &suffixe(g, PRECEDENT));
            } else {
                let _ = std::fs::remove_file(abri(g));
            }
        }
    };
    if let Err(er) = envoyer() {
        ranger_abris();
        remettre(e, avait);
        return Err(er);
    }
    let vue = attendre_fichier(&e.passage, delai);
    ranger_abris();
    if !vue {
        // Pas de sauvegarde vue : une COPIE revient au passage, l'original reste à côté sous un nom unique (une
        // sauvegarde tardive de BigPEmu n'écrasera que la copie).
        if avait {
            let garde = suffixe(&e.passage, &format!("frogtend-avant-{}", maintenant_ms()));
            if renommer(&e.de_cote(), &garde).is_ok() && !e.passage.exists() {
                let _ = std::fs::copy(&garde, &e.passage);
            }
        }
        let autre = releve(&e.dossier).into_iter().find(|(f, m)| *f != e.passage && avant.iter().all(|(g, m2)| g != f || m2 != m)).map(|(f, _)| f);
        return Err(Erreur::Refus(match autre {
            Some(f) => format!(
                "BigPEmu a sauvé dans un autre fichier ({}) : son emplacement a dû être changé dans son propre menu ; l'ancien contenu est gardé à côté. Quitte et relance le jeu.",
                f.file_name().unwrap_or_default().to_string_lossy()
            ),
            None => format!(
                "BigPEmu n'a pas confirmé la sauvegarde dans l'emplacement {} : tes sauvegardes sont gardées (celle de BigPEmu aussi, à côté de « {} »).",
                n + 1,
                e.passage.file_name().unwrap_or_default().to_string_lossy()
            ),
        }));
    }
    let coupe = |er: std::io::Error| {
        Erreur::Refus(format!("La sauvegarde est faite mais n'a pas pu être rangée ({er}) : rien n'est perdu, relance le jeu pour que Frogtend remette tout en place."))
    };
    let cible = e.fichier(n);
    if cible == e.passage {
        if avait {
            renommer(&e.de_cote(), &suffixe(&cible, PRECEDENT)).map_err(coupe)?;
        }
        return Ok(());
    }
    if cible.exists() {
        std::fs::copy(&cible, suffixe(&cible, PRECEDENT)).map_err(coupe)?;
    }
    renommer(&e.passage, &cible).map_err(coupe)?;
    if avait && !e.passage.exists() {
        renommer(&e.de_cote(), &e.passage).map_err(coupe)?;
    }
    Ok(())
}

fn identiques(a: &Path, b: &Path) -> bool {
    matches!((std::fs::read(a), std::fs::read(b)), (Ok(x), Ok(y)) if x == y)
}

/// Charger l'emplacement `n` : une COPIE de sa sauvegarde à la place du passage (mis de côté), `envoyer`, le temps
/// que BigPEmu la lise (`lecture`), puis la copie retirée (seulement si elle est restée identique, sinon gardée à
/// côté) et le passage remis. L'emplacement `n` n'est jamais déplacé.
pub fn charger(e: &EtatsBp, n: u8, envoyer: impl FnOnce() -> Resultat<()>, lecture: std::time::Duration) -> Resultat<()> {
    let cible = e.fichier(n);
    if !cible.is_file() {
        return Err(Erreur::Refus(format!("Aucune sauvegarde dans l'emplacement {} : sauve d'abord.", n + 1)));
    }
    if cible == e.passage {
        return envoyer();
    }
    let avait = mettre_de_cote(e)?;
    if let Err(er) = std::fs::copy(&cible, &e.passage) {
        // Le passage était libre (mis de côté ou absent) : ce qui s'y trouve est la copie ratée de Frogtend.
        let _ = std::fs::remove_file(&e.passage);
        remettre(e, avait);
        return Err(Erreur::Refus(format!("La sauvegarde de l'emplacement {} n'a pas pu être préparée ({er}) : rien n'a été fait.", n + 1)));
    }
    let r = envoyer();
    if r.is_ok() {
        std::thread::sleep(lecture);
    }
    if identiques(&e.passage, &cible) {
        let _ = std::fs::remove_file(&e.passage);
    }
    remettre(e, avait);
    r
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

    const E: &str = "5C0C7827948F0F27";

    /// Un dossier de données avec le passage commun d'origine (`state000`).
    fn etats_dans(d: &Path) -> EtatsBp {
        std::fs::create_dir_all(d).unwrap();
        EtatsBp { dossier: d.to_path_buf(), empreinte: E.into(), passage: d.join("state000.bigpstate") }
    }

    fn lire_t(f: &Path) -> String {
        std::fs::read_to_string(f).unwrap()
    }

    #[test]
    fn chaque_jeu_a_ses_emplacements_et_le_passage_est_celui_de_bigpemu() {
        let d = profil_avec(XBOX);
        let p = d.path().join("Profils").join("Seb");
        let e = etats_de_la_partie(&p, Some("5c0c7827948f0f27")).unwrap();
        assert_eq!(e.passage.file_name().unwrap(), "state000.bigpstate");
        assert_eq!(e.fichier(4).file_name().unwrap(), "game5C0C7827948F0F27_state004.bigpstate", "par jeu, même d'origine");
        assert_eq!(etats_de_la_partie(&p, None), None, "jeu sans empreinte (CD) : pas d'emplacements");
        let mut v: Value = serde_json::from_str(XBOX).unwrap();
        v["BigPEmuConfig"]["System"]["PerGameSlots"] = 1.into();
        v["BigPEmuConfig"]["System"]["StateSlot"] = 3.into();
        std::fs::write(reglages_du_profil(&p), v.to_string()).unwrap();
        let e = etats_de_la_partie(&p, Some("5c0c7827948f0f27")).unwrap();
        assert_eq!(e.passage, e.fichier(3), "Per-Title States : le passage est l'emplacement 4 du jeu");
        v["BigPEmuConfig"]["System"]["SaveAutoIncr"] = 1.into();
        std::fs::write(reglages_du_profil(&p), v.to_string()).unwrap();
        assert_eq!(etats_de_la_partie(&p, Some("5c0c7827948f0f27")), None, "Increment on Save : BigPEmu choisit seul");
    }

    #[test]
    fn sauver_range_dans_l_emplacement_du_jeu_et_remet_le_passage() {
        let d = tempfile::tempdir().unwrap();
        let e = etats_dans(d.path());
        std::fs::write(&e.passage, "passage").unwrap();
        std::fs::write(e.fichier(4), "ancienne").unwrap();
        let pass = e.passage.clone();
        let r = sauver(&e, 4, || {
            assert!(!pass.exists(), "le passage est de côté pendant la sauvegarde (aucune question de BigPEmu)");
            std::fs::write(&pass, "nouvelle").map_err(Erreur::from)
        }, std::time::Duration::from_secs(2));
        assert!(r.is_ok(), "{r:?}");
        assert_eq!(lire_t(&e.fichier(4)), "nouvelle");
        assert_eq!(lire_t(&e.passage), "passage", "le passage est remis");
        assert_eq!(lire_t(&suffixe(&e.fichier(4), PRECEDENT)), "ancienne");
        assert!(!e.de_cote().exists());
    }

    #[test]
    fn sauver_dans_le_passage_par_jeu_garde_l_ancienne_a_cote() {
        let d = tempfile::tempdir().unwrap();
        let mut e = etats_dans(d.path());
        e.passage = e.fichier(2);
        std::fs::write(&e.passage, "ancienne").unwrap();
        let f = e.passage.clone();
        sauver(&e, 2, || std::fs::write(&f, "nouvelle").map_err(Erreur::from), std::time::Duration::from_secs(2)).unwrap();
        assert_eq!(lire_t(&e.fichier(2)), "nouvelle");
        assert_eq!(lire_t(&suffixe(&e.fichier(2), PRECEDENT)), "ancienne");
    }

    #[test]
    fn une_sauvegarde_non_vue_garde_l_original_et_ne_touche_pas_l_emplacement() {
        let d = tempfile::tempdir().unwrap();
        let e = etats_dans(d.path());
        std::fs::write(&e.passage, "passage").unwrap();
        std::fs::write(e.fichier(4), "ancienne").unwrap();
        assert!(sauver(&e, 4, || Err(Erreur::Refus("touche".into())), std::time::Duration::from_millis(300)).is_err());
        assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 2, "touche non envoyée : tout remis, rien en plus");
        assert!(sauver(&e, 4, || Ok(()), std::time::Duration::from_millis(300)).is_err());
        assert_eq!(lire_t(&e.passage), "passage", "une copie revient au passage");
        assert_eq!(lire_t(&e.fichier(4)), "ancienne");
        let gardes: Vec<_> = std::fs::read_dir(d.path()).unwrap().flatten().filter(|x| x.file_name().to_string_lossy().contains(".frogtend-avant-")).collect();
        assert_eq!(gardes.len(), 1, "l'original reste à côté : une sauvegarde tardive n'écrase que la copie");
        assert_eq!(lire_t(&gardes[0].path()), "passage");
    }

    #[test]
    fn bigpemu_qui_sauve_ailleurs_ne_fait_rien_perdre() {
        let d = tempfile::tempdir().unwrap();
        let mut e = etats_dans(d.path());
        e.passage = e.fichier(0);
        std::fs::write(e.fichier(0), "zero").unwrap();
        std::fs::write(e.fichier(5), "cinq").unwrap();
        let k = e.fichier(5);
        // Son emplacement changé dans son propre menu : F13 écrit dans l'emplacement 6 du jeu.
        let r = sauver(&e, 0, || std::fs::write(&k, "ecrasee").map_err(Erreur::from), std::time::Duration::from_millis(300));
        assert!(r.unwrap_err().to_string().contains("autre fichier"));
        assert_eq!(lire_t(&suffixe(&e.fichier(5), PRECEDENT)), "cinq", "l'ancien contenu est gardé");
        assert_eq!(lire_t(&e.fichier(0)), "zero");
        assert!(std::fs::read_dir(d.path()).unwrap().flatten().all(|x| !x.file_name().to_string_lossy().ends_with(".frogtend-abri")));
    }

    #[test]
    fn un_fichier_arrive_pendant_l_operation_n_est_jamais_ecrase() {
        let d = tempfile::tempdir().unwrap();
        let e = etats_dans(d.path());
        std::fs::write(&e.passage, "passage").unwrap();
        std::fs::write(e.fichier(7), "sept").unwrap();
        let pass = e.passage.clone();
        // BigPEmu sauve (par sa propre touche) pendant le chargement : sa sauvegarde est gardée à côté.
        charger(&e, 7, || std::fs::write(&pass, "ecrite par bigpemu").map_err(Erreur::from), std::time::Duration::ZERO).unwrap();
        assert_eq!(lire_t(&e.passage), "passage");
        let tardifs: Vec<_> = std::fs::read_dir(d.path()).unwrap().flatten().filter(|x| x.file_name().to_string_lossy().contains(".frogtend-tardif-")).collect();
        assert_eq!(tardifs.len(), 1);
        assert_eq!(lire_t(&tardifs[0].path()), "ecrite par bigpemu");
    }

    #[test]
    fn charger_un_emplacement_passe_par_une_copie() {
        let d = tempfile::tempdir().unwrap();
        let e = etats_dans(d.path());
        std::fs::write(&e.passage, "passage").unwrap();
        std::fs::write(e.fichier(7), "sept").unwrap();
        let pass = e.passage.clone();
        let lu = std::cell::RefCell::new(String::new());
        charger(&e, 7, || {
            *lu.borrow_mut() = lire_t(&pass);
            Ok(())
        }, std::time::Duration::from_millis(10))
        .unwrap();
        assert_eq!(*lu.borrow(), "sept", "BigPEmu lit l'emplacement 7 à la place du sien");
        assert_eq!(lire_t(&e.passage), "passage");
        assert_eq!(lire_t(&e.fichier(7)), "sept");
        assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 2);
        assert!(charger(&e, 5, || Ok(()), std::time::Duration::ZERO).is_err(), "emplacement vide");
    }

    #[test]
    fn une_operation_coupee_est_remise_en_place_sans_rien_perdre() {
        let d = profil_avec(XBOX);
        let p = d.path().join("Profils").join("Seb");
        let e = etats_de_la_partie(&p, Some(E)).unwrap();
        std::fs::write(e.de_cote(), "passage").unwrap();
        std::fs::write(&e.passage, "copie").unwrap();
        assert!(sauver(&e, 3, || Ok(()), std::time::Duration::ZERO).is_err(), "jamais par-dessus une opération coupée");
        assert_eq!(reprendre_operations_coupees(&p, 9).unwrap(), 1);
        assert_eq!(lire_t(&e.passage), "passage");
        let recup: Vec<_> = std::fs::read_dir(e.passage.parent().unwrap()).unwrap().flatten().filter(|x| x.file_name().to_string_lossy().contains(".frogtend-recupere-9-")).collect();
        assert_eq!(lire_t(&recup[0].path()), "copie");
        assert_eq!(reprendre_operations_coupees(&p, 9).unwrap(), 0);
        assert!(fichiers_du_jeu(&p, E).iter().all(|f| !f.to_string_lossy().contains(".frogtend-")), "jamais copiés avant une triche");
    }
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

    #[test]
    fn les_touches_du_menu_sont_ajoutees_sans_rien_retirer() {
        let d = profil_avec(XBOX);
        let p = d.path().join("Profils").join("Seb");
        assert!(!raccourcis_poses(&p));
        assert!(poser_raccourcis(d.path(), &p).unwrap());
        assert!(raccourcis_poses(&p));
        let v: Value = serde_json::from_str(&std::fs::read_to_string(reglages_du_profil(&p)).unwrap()).unwrap();
        let b = &v["BigPEmuConfig"]["Input"]["Device0"]["Bindings"];
        assert_eq!(b[40]["Triggers"], serde_json::json!([{"B_KB": true, "B_ID": 100, "B_AH": 0.0}]));
        assert_eq!(b[42]["Triggers"][0]["B_ID"], 102);
        // La manette d'origine est intacte ; une seconde fois : rien à écrire.
        assert_eq!(liaisons_joueur1(&std::fs::read_to_string(reglages_du_profil(&p)).unwrap()).unwrap(), DISPOSITION_XBOX.to_vec());
        let avant = std::fs::read_to_string(reglages_du_profil(&p)).unwrap();
        assert!(poser_raccourcis(d.path(), &p).unwrap());
        assert_eq!(std::fs::read_to_string(reglages_du_profil(&p)).unwrap(), avant);
        // F13 déjà pris par une autre action de la personne : pas posé (il ferait les deux), le reste oui.
        let mut v: Value = serde_json::from_str(XBOX).unwrap();
        v["BigPEmuConfig"]["Input"]["Device0"]["Bindings"][38]["Triggers"] = serde_json::json!([{"B_KB": true, "B_ID": 100, "B_AH": 0.0}]);
        let d2 = profil_avec(&v.to_string());
        let p2 = d2.path().join("Profils").join("Seb");
        assert!(!poser_raccourcis(d2.path(), &p2).unwrap());
        let v2: Value = serde_json::from_str(&std::fs::read_to_string(reglages_du_profil(&p2)).unwrap()).unwrap();
        assert_eq!(v2["BigPEmuConfig"]["Input"]["Device0"]["Bindings"][40]["Triggers"], serde_json::json!([]));
        assert_eq!(v2["BigPEmuConfig"]["Input"]["Device0"]["Bindings"][41]["Triggers"][0]["B_ID"], 101);
        // Jamais lancé : rien.
        assert!(!poser_raccourcis(d.path(), &d.path().join("Profils").join("Léa")).unwrap());
    }

    fn avec_effets(d: &Path) {
        let fx = d.join("Data").join("ScreenEffects");
        std::fs::create_dir_all(&fx).unwrap();
        for e in ["crt-hyllian", "gamma", "shadertoy"] {
            std::fs::write(fx.join(format!("{e}.dstack")), b"x").unwrap();
        }
    }

    fn effet_ecrit(p: &Path) -> Option<String> {
        let v: Value = serde_json::from_str(&std::fs::read_to_string(reglages_du_profil(p)).unwrap()).unwrap();
        v["BigPEmuConfig"]["ScreenEffect"].as_str().map(String::from)
    }

    #[test]
    fn l_effet_choisi_est_pose_puis_rendu_et_celui_de_la_personne_reste() {
        let d = profil_avec(XBOX);
        avec_effets(d.path());
        let p = d.path().join("Profils").join("Seb");
        assert_eq!(effets_presents(d.path()), vec!["bigpemu:crt-hyllian", "bigpemu:gamma"], "shadertoy : pas proposé");
        assert_eq!(poser_effet(d.path(), &p, Some("bigpemu:crt-hyllian")).unwrap().as_deref(), Some("bigpemu:crt-hyllian"));
        assert_eq!(effet_ecrit(&p).as_deref(), Some("crt-hyllian.dstack"));
        // Un jeu sans choix : l'effet posé par Frogtend est retiré.
        assert_eq!(poser_effet(d.path(), &p, None).unwrap(), None);
        assert_eq!(effet_ecrit(&p), None);
        // Un effet de RetroArch, un effet absent : pas de choix pour BigPEmu.
        assert_eq!(poser_effet(d.path(), &p, Some("crt")).unwrap(), None);
        assert_eq!(poser_effet(d.path(), &p, Some("bigpemu:liz")).unwrap(), None);
        // L'effet réglé par la personne dans BigPEmu : il revient après un jeu à effet choisi, et reste sans choix.
        let mut v: Value = serde_json::from_str(&std::fs::read_to_string(reglages_du_profil(&p)).unwrap()).unwrap();
        v["BigPEmuConfig"]["ScreenEffect"] = "gamma.dstack".into();
        std::fs::write(reglages_du_profil(&p), v.to_string()).unwrap();
        assert_eq!(poser_effet(d.path(), &p, None).unwrap().as_deref(), Some("bigpemu:gamma"));
        poser_effet(d.path(), &p, Some("bigpemu:crt-hyllian")).unwrap();
        assert_eq!(effet_ecrit(&p).as_deref(), Some("crt-hyllian.dstack"));
        poser_effet(d.path(), &p, None).unwrap();
        assert_eq!(effet_ecrit(&p).as_deref(), Some("gamma.dstack"));
        // « aucun » : retiré pour ce jeu, celui de la personne revient ensuite.
        poser_effet(d.path(), &p, Some("aucun")).unwrap();
        assert_eq!(effet_ecrit(&p), None);
        poser_effet(d.path(), &p, None).unwrap();
        assert_eq!(effet_ecrit(&p).as_deref(), Some("gamma.dstack"));
        // Jamais lancé : rien.
        assert_eq!(poser_effet(d.path(), &d.path().join("Profils").join("Léa"), Some("bigpemu:gamma")).unwrap(), None);
    }

    #[test]
    fn l_empreinte_d_une_cartouche_est_celle_de_bigpemu() {
        // FNV-1a 64 de référence : chaîne vide et « a ».
        assert_eq!(fnv1a64(b""), 0xcbf29ce484222325);
        assert_eq!(fnv1a64(b"a"), 0xaf63dc4c8601ec8c);
        let d = tempfile::tempdir().unwrap();
        let j64 = d.path().join("Jeu.j64");
        std::fs::write(&j64, b"cartouche").unwrap();
        let attendue = format!("{:016x}", fnv1a64(b"cartouche"));
        assert_eq!(empreinte_jeu(&j64).as_deref(), Some(attendue.as_str()));
        let zip = d.path().join("Jeu (World).zip");
        {
            let mut z = zip::ZipWriter::new(std::fs::File::create(&zip).unwrap());
            z.start_file("Jeu (World).j64", zip::write::SimpleFileOptions::default()).unwrap();
            std::io::Write::write_all(&mut z, b"cartouche").unwrap();
            z.finish().unwrap();
        }
        assert_eq!(empreinte_jeu(&zip).as_deref(), Some(attendue.as_str()), "dans son zip : la même");
        assert_eq!(empreinte_jeu(&d.path().join("Jeu.cue")), None, "un jeu CD : pas encore relevé");
    }

    const TRICHES: &str = r#"{"DickGenie": {"5c0c7827948f0f27": {"Name": "Bubsy", "Codes": [
        {"Name": "Infinite lives", "Code": "5?1:!V@{"},
        {"Name": "Invincible", "Code": "=?EP!V@V($"},
        {"Name": "1-shot kills", "Code": "x"},
        {"Name": "Force lap count", "Code": ["a", "b"], "CodeLabels": ["!STR_DISABLED", "1"]}]}}}"#;

    #[test]
    fn la_fenetre_de_triches_n_est_prevue_que_si_le_module_est_pose() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("Profils").join("Seb");
        std::fs::create_dir_all(p.join(DOSSIER_DONNEES)).unwrap();
        let rom = d.path().join("jeu.j64");
        std::fs::write(&rom, b"cartouche").unwrap();
        let e = empreinte_jeu(&rom).unwrap();
        choisir_triche(&p, &format!("{e}_Infinite_lives"), true).unwrap();
        // Premier lancement du profil : pas encore de réglages, rien n'est posé, donc pas d'Échap.
        let l = preparer_partie(d.path(), &p, Some(&rom), None, 1).unwrap();
        assert!(!l.fenetre_triches, "sans réglages : pas de module, pas de fenêtre");
        std::fs::write(reglages_du_profil(&p), XBOX).unwrap();
        let l = preparer_partie(d.path(), &p, Some(&rom), None, 2).unwrap();
        assert!(l.fenetre_triches, "module posé pour la triche cochée");
        let v: Value = serde_json::from_str(&std::fs::read_to_string(reglages_du_profil(&p)).unwrap()).unwrap();
        assert_eq!(v["BigPEmuConfig"]["ScriptsEnabled"], serde_json::json!(["cheat_codes"]));
    }

    #[test]
    fn les_triches_choisies_sont_posees_au_lancement_avec_la_partie_a_l_abri() {
        let d = profil_avec(XBOX);
        let p = d.path().join("Profils").join("Seb");
        std::fs::create_dir_all(d.path().join("Scripts")).unwrap();
        std::fs::write(d.path().join("Scripts").join("cheat_codes.json"), TRICHES).unwrap();
        let h = "5c0c7827948f0f27";
        let l = triches_du_jeu(d.path(), &p, h);
        assert_eq!(l.iter().map(|t| t.titre.as_str()).collect::<Vec<_>>(), vec!["Infinite lives", "Invincible"], "nom non relevé, plusieurs valeurs : pas proposés");
        assert!(l.iter().all(|t| !t.active));
        assert!(triches_du_jeu(d.path(), &p, "0000000000000000").is_empty());
        // Cochée dans le menu : notée, montrée cochée, rien d'écrit dans les réglages avant le lancement.
        choisir_triche(&p, &l[0].cle, true).unwrap();
        assert!(triches_du_jeu(d.path(), &p, h)[0].active);
        assert!(!std::fs::read_to_string(reglages_du_profil(&p)).unwrap().contains("ScriptData"));
        // Le lancement : la mémoire du jeu à l'abri, la triche et le module posés, le choix oublié.
        let partie = p.join("UserData").join("game5C0C7827948F0F27_eeprom.bigpeep");
        std::fs::write(&partie, b"partie").unwrap();
        let rom = d.path().join("Bubsy.j64");
        std::fs::write(&rom, b"rom").unwrap();
        let l = preparer_partie(d.path(), &p, None, None, 1).unwrap();
        assert!(l.empreinte.is_none() && !l.abri_fait && l.triches.is_empty() && l.erreur_triches.is_none(), "jeu sans empreinte : pas de triches");
        let v: Value = serde_json::from_str(&std::fs::read_to_string(reglages_du_profil(&p)).unwrap()).unwrap();
        assert_eq!(v["BigPEmuConfig"]["ScriptData_CheatSetting_5c0c7827948f0f27_Infinite_lives"], "01000000");
        assert_eq!(v["BigPEmuConfig"]["ScriptsEnabled"], serde_json::json!([]), "pas pour un autre jeu : pas de fenêtre");
        assert!(!p.join(TRICHES_A_POSER).exists());
        assert!(appliquer_triches(d.path(), &p, Some(h)).unwrap());
        let v: Value = serde_json::from_str(&std::fs::read_to_string(reglages_du_profil(&p)).unwrap()).unwrap();
        assert_eq!(v["BigPEmuConfig"]["ScriptsEnabled"], serde_json::json!(["cheat_codes"]));
        assert_eq!(fichiers_du_jeu(&p, h), vec![partie.clone()]);
        crate::menu_jeu::mettre_a_l_abri_fichiers(&fichiers_du_jeu(&p, h), 7).unwrap();
        assert_eq!(std::fs::read(p.join("UserData").join("game5C0C7827948F0F27_eeprom.bigpeep.avant-triche-7")).unwrap(), b"partie");
        assert_eq!(fichiers_du_jeu(&p, h), vec![partie], "la copie n'est pas recopiée");
        // Décochée : le module (activé par Frogtend) coupé, un autre module de la personne gardé.
        let mut v: Value = serde_json::from_str(&std::fs::read_to_string(reglages_du_profil(&p)).unwrap()).unwrap();
        v["BigPEmuConfig"]["ScriptsEnabled"] = serde_json::json!(["cheat_codes", "turbo_joy"]);
        std::fs::write(reglages_du_profil(&p), v.to_string()).unwrap();
        choisir_triche(&p, &format!("{h}_Infinite_lives"), false).unwrap();
        assert!(!appliquer_triches(d.path(), &p, Some(h)).unwrap());
        let v: Value = serde_json::from_str(&std::fs::read_to_string(reglages_du_profil(&p)).unwrap()).unwrap();
        assert_eq!(v["BigPEmuConfig"]["ScriptsEnabled"], serde_json::json!(["turbo_joy"]));
        assert_eq!(v["BigPEmuConfig"]["ScriptData_CheatSetting_5c0c7827948f0f27_Infinite_lives"], "00000000");
        // Le module activé par la personne elle-même (✅ Seb 11/11) : coupé aussi sans triche cochée (sinon sa fenêtre à
        // chaque lancement), pour un jeu CD aussi ; les choix restent.
        for empreinte in [Some(h), None] {
            let mut v: Value = serde_json::from_str(&std::fs::read_to_string(reglages_du_profil(&p)).unwrap()).unwrap();
            v["BigPEmuConfig"]["ScriptsEnabled"] = serde_json::json!(["cheat_codes", "turbo_joy"]);
            v["BigPEmuConfig"]["ScriptData_CheatSetting_aaaaaaaaaaaaaaaa_Autre"] = "01000000".into();
            std::fs::write(reglages_du_profil(&p), v.to_string()).unwrap();
            assert!(!appliquer_triches(d.path(), &p, empreinte).unwrap());
            let v: Value = serde_json::from_str(&std::fs::read_to_string(reglages_du_profil(&p)).unwrap()).unwrap();
            assert_eq!(v["BigPEmuConfig"]["ScriptsEnabled"], serde_json::json!(["turbo_joy"]));
            assert_eq!(v["BigPEmuConfig"]["ScriptData_CheatSetting_aaaaaaaaaaaaaaaa_Autre"], "01000000", "le choix d'un autre jeu reste");
        }
    }

    /// RÉEL, lecture seule : l'empreinte d'un jeu et ses triches dans un BigPEmu installé (rien n'est écrit).
    /// `FROGTEND_BIGPEMU=<dossier> FROGTEND_PROFIL_BP=<dossier du profil> FROGTEND_ROM=<jeu> cargo test essai_triches_reelles -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn essai_triches_reelles() {
        let e = PathBuf::from(std::env::var("FROGTEND_BIGPEMU").unwrap());
        let p = PathBuf::from(std::env::var("FROGTEND_PROFIL_BP").unwrap());
        let rom = PathBuf::from(std::env::var("FROGTEND_ROM").unwrap());
        let h = empreinte_jeu(&rom).expect("empreinte");
        println!("empreinte {h}");
        for t in triches_du_jeu(&e, &p, &h) {
            println!("{t:?}");
        }
        println!("effets {:?} ; touches posées : {}", effets_presents(&e), raccourcis_poses(&p));
    }

    /// RÉEL sur une COPIE : la préparation complète (touches, effet, triches) appliquée au profil d'un BigPEmu de
    /// test. `FROGTEND_BIGPEMU=<copie> FROGTEND_PROFIL_BP=<copie\Profils\X> FROGTEND_ROM=<jeu> FROGTEND_EFFET=bigpemu:…`
    #[test]
    #[ignore]
    fn essai_preparer_une_copie() {
        let e = PathBuf::from(std::env::var("FROGTEND_BIGPEMU").unwrap());
        let p = PathBuf::from(std::env::var("FROGTEND_PROFIL_BP").unwrap());
        assert!(p.starts_with(&e) && !e.to_string_lossy().contains("Frogtend\\Emulateurs"), "seulement une copie de test");
        let rom = PathBuf::from(std::env::var("FROGTEND_ROM").unwrap());
        println!("touches : {}", poser_raccourcis(&e, &p).unwrap());
        let l = preparer_partie(&e, &p, Some(&rom), std::env::var("FROGTEND_EFFET").ok().as_deref(), 1).unwrap();
        println!("{l:?}");
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
