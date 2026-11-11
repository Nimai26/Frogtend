//! xemu (Xbox) : la langue de la console dès qu'elle est connue (règle de Seb, 08/11 : « configurer TOUS les
//! émulateurs […] à chaque nouvelle installation »). Relevé dans le code d'xemu (master, 11/11/2026) :
//! - la langue n'est pas un réglage d'xemu : elle est dans l'EEPROM de la Xbox, un fichier de 256 octets
//!   (`sys.files.eeprom_path`, vide = `eeprom.bin` à côté du programme en portable : ui/xemu-settings.cc
//!   `xemu_settings_get_default_eeprom_path`) ;
//! - xemu le CRÉE à son démarrage s'il manque (system/vl.c `get_eeprom_path` → hw/xbox/eeprom_generation.c
//!   `xbox_eeprom_generate`) avec des clés tirées au hasard et chiffrées : Frogtend ne le crée JAMAIS ; la langue est
//!   posée à la partie suivante ;
//! - `XboxEEPROM` (eeprom_generation.h, `#pragma pack(1)`) : la section « utilisateur » (non chiffrée) commence à
//!   0x64, sa somme de contrôle (`user_checksum`) est à 0x60 et couvre 0x5C octets (`xbox_eeprom_crc`, recopiée
//!   ci-dessous) ; la langue est à `user_section + 0x2C` = l'octet 0x90, 32 bits petit-boutiste ; xemu écrit 1
//!   (anglais, « FIXME: temporarily set […] English language ») ;
//! - les langues (`XC_LANGUAGE_*`, relues dans Cxbx-Reloaded src/common/EmuEEPROM.h) : anglais 1, japonais 2,
//!   allemand 3, français 4, espagnol 5, italien 6, coréen 7, chinois traditionnel 8, portugais 9 ;
//! - la manette : xemu branche seul la première manette SDL (Xbox ou PlayStation) au port 1 (`input.auto_bind`,
//!   vrai d'origine, ui/xemu-input.c) ; LB et RB y deviennent les boutons Blanc et Noir ;
//! - avec `-config_path` (un réglage par profil), l'EEPROM d'origine reste à côté du programme en portable
//!   (ui/xemu-settings.cc `xemu_settings_get_base_path` : `SDL_GetBasePath`, indépendant du chemin des réglages) ;
//! - le bouton Xbox (et Back + Start) ouvre le menu d'xemu, en dur (ui/xui/main.cc) : gardé.

use crate::erreurs::{Erreur, Resultat};
use std::path::{Path, PathBuf};

/// La taille de l'EEPROM de la Xbox.
const TAILLE_EEPROM: usize = 256;
/// La somme de contrôle de la section utilisateur, puis la section elle-même et sa longueur.
const SOMME: usize = 0x60;
const SECTION: usize = 0x64;
const LONGUEUR: usize = 0x5C;
/// L'octet de la langue.
const LANGUE: usize = 0x90;
/// L'anglais : la valeur qu'xemu écrit.
const ANGLAIS: u32 = 1;

fn marque(xemu: &Path) -> PathBuf {
    xemu.join(".frogtend-langue")
}

/// xemu installé en portable (un `xemu.toml` à côté du programme, comme Frogtend l'installe) ? Sinon : jamais touché.
pub fn portable(xemu: &Path) -> bool {
    xemu.join("xemu.toml").is_file()
}

/// La langue de la console Xbox pour une langue de Windows (LANGID) ; `None` : une langue que la console n'a pas.
pub fn langue_console(langid: u16) -> Option<u32> {
    Some(match langid & 0x3FF {
        0x09 => 1,
        0x11 => 2,
        0x07 => 3,
        0x0C => 4,
        0x0A => 5,
        0x10 => 6,
        0x12 => 7,
        // La console n'a que le chinois traditionnel : Taïwan, Hong Kong, Macao (et le traditionnel neutre).
        0x04 if matches!(langid, 0x0404 | 0x0C04 | 0x1404 | 0x7C04) => 8,
        0x16 => 9,
        _ => return None,
    })
}

/// La somme de contrôle de l'EEPROM (hw/xbox/eeprom_generation.c `xbox_eeprom_crc`, à l'identique).
fn somme(octets: &[u8]) -> u32 {
    let (mut haut, mut bas) = (0u32, 0u32);
    for m in octets.chunks_exact(4) {
        let v = u32::from_le_bytes([m[0], m[1], m[2], m[3]]);
        let s = ((haut as u64) << 32) | bas as u64;
        haut = ((s + v as u64) >> 32) as u32;
        bas = bas.wrapping_add(v);
    }
    !(haut.wrapping_add(bas))
}

/// L'EEPROM que ce profil d'xemu utilise : celle que donnent ses réglages, ou celle d'origine à côté du programme.
/// Seulement celle d'origine (dans le dossier d'xemu) : une EEPROM choisie ailleurs par la personne n'est jamais touchée.
fn eeprom(xemu: &Path, reglages: &str) -> Option<PathBuf> {
    let origine = xemu.join("eeprom.bin");
    match crate::emulateurs_profils::valeur_toml(reglages, "sys.files", "eeprom_path") {
        None => Some(origine),
        Some(p) => {
            let p = PathBuf::from(p);
            let meme = |a: &Path, b: &Path| a.to_string_lossy().replace('/', "\\").to_lowercase() == b.to_string_lossy().replace('/', "\\").to_lowercase();
            (meme(&p, &origine) || std::fs::canonicalize(&p).ok().zip(std::fs::canonicalize(&origine).ok()).is_some_and(|(a, b)| a == b)).then_some(origine)
        }
    }
}

/// La langue de la console prise de Windows, UNE seule fois par xemu (marque) : seulement xemu portable et FERMÉ,
/// seulement l'EEPROM d'origine, de 256 octets, dont la somme de la section utilisateur est juste, encore en
/// anglais. Absente : rien (xemu la crée à sa première partie ; la langue sera posée à la suivante). L'ancienne est
/// copiée à l'abri ; écrite par un provisoire renommé, puis relue. `reglages` : le `xemu.toml` du profil. Rend vrai si
/// la langue a été posée.
pub fn regler_langue(xemu: &Path, reglages: &Path, langid: u16) -> Resultat<bool> {
    if !portable(xemu) || marque(xemu).exists() || !crate::lancement::processus_du_dossier(xemu).is_empty() {
        return Ok(false);
    }
    let Some(f) = eeprom(xemu, &std::fs::read_to_string(reglages).unwrap_or_default()) else { return Ok(false) };
    // La taille d'abord (jamais lire un fichier inattendu en entier).
    match std::fs::metadata(&f) {
        Ok(m) if m.len() == TAILLE_EEPROM as u64 => {}
        Ok(_) => return Ok(false),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.into()),
    }
    let octets = std::fs::read(&f)?;
    if octets.len() != TAILLE_EEPROM {
        return Ok(false);
    }
    let lu = |o: &[u8], i: usize| u32::from_le_bytes([o[i], o[i + 1], o[i + 2], o[i + 3]]);
    // Une somme fausse : une EEPROM inconnue (ou abîmée) — jamais touchée.
    if lu(&octets, SOMME) != somme(&octets[SECTION..SECTION + LONGUEUR]) {
        return Ok(false);
    }
    let mut pose = false;
    if let Some(n) = langue_console(langid).filter(|n| *n != ANGLAIS && lu(&octets, LANGUE) == ANGLAIS) {
        let mut nouveau = octets.clone();
        nouveau[LANGUE..LANGUE + 4].copy_from_slice(&n.to_le_bytes());
        let s = somme(&nouveau[SECTION..SECTION + LONGUEUR]);
        nouveau[SOMME..SOMME + 4].copy_from_slice(&s.to_le_bytes());
        crate::emulateurs_profils::mettre_a_l_abri(xemu, &f)?;
        let provisoire = f.with_extension("bin.frogtend-tmp");
        std::fs::write(&provisoire, &nouveau)?;
        if let Err(e) = std::fs::rename(&provisoire, &f) {
            let _ = std::fs::remove_file(&provisoire);
            return Err(e.into());
        }
        if std::fs::read(&f).ok().as_deref() != Some(nouveau.as_slice()) {
            return Err(Erreur::Disque("La langue d'xemu n'a pas pu être posée.".into()));
        }
        pose = true;
    }
    std::fs::write(marque(xemu), "")?;
    Ok(pose)
}

/// La disposition d'origine d'xemu pour une manette (config_spec.yml `controller_mapping`, relu le 11/11) : bouton ou
/// axe Xbox → numéro SDL.
const DISPOSITION: [(&str, i64); 21] = [
    ("a", 0), ("b", 1), ("x", 2), ("y", 3), ("back", 4), ("guide", 5), ("start", 6), ("lstick_btn", 7), ("rstick_btn", 8),
    ("lshoulder", 9), ("rshoulder", 10), ("dpad_up", 11), ("dpad_down", 12), ("dpad_left", 13), ("dpad_right", 14),
    ("axis_left_x", 0), ("axis_left_y", 1), ("axis_right_x", 2), ("axis_right_y", 3), ("axis_trigger_left", 4), ("axis_trigger_right", 5),
];

/// Les réglages d'un profil d'xemu (`xemu.toml`) gardent-ils sa disposition d'origine ? Faux si la personne a changé
/// une touche ou inversé un axe pour une manette (`input.gamepad_mappings`), si le joueur 1 est au clavier, ou si le
/// fichier est illisible (jamais deviné). Absent : vrai (xemu partira de son origine).
pub fn disposition_d_origine(texte: Option<&str>) -> bool {
    let Some(texte) = texte else { return true };
    // Lu ligne par ligne (le TOML qu'xemu écrit : une section par table, `[[…]]` pour chaque manette). Une forme
    // inattendue (table en ligne, valeur illisible) : on ne sait pas → faux.
    let mut section = String::new();
    for ligne in texte.lines() {
        let l = ligne.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        if l.starts_with('[') {
            if !l.ends_with(']') {
                return false;
            }
            section = l.trim_matches(|c| c == '[' || c == ']').trim().to_string();
            continue;
        }
        let Some((cle, val)) = l.split_once('=') else { return false };
        let (cle, val) = (cle.trim(), val.split('#').next().unwrap_or("").trim());
        let ok = match section.as_str() {
            // Le joueur 1 au clavier (« keyboard », ui/xemu-input.c), ou plus aucune manette branchée d'office.
            "input.bindings" if cle == "port1" => val.trim_matches(|c| c == '\'' || c == '"') != "keyboard",
            "input" if cle == "auto_bind" => val != "false",
            "input" | "input.gamepad_mappings" if cle == "controller_mapping" || cle == "gamepad_mappings" => false,
            "input.gamepad_mappings.controller_mapping" => {
                if cle.starts_with("invert_axis_") {
                    val == "false"
                } else {
                    DISPOSITION.iter().find(|(k, _)| *k == cle).map_or(true, |(_, d)| val.parse::<i64>().ok() == Some(*d))
                }
            }
            _ => true,
        };
        if !ok {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_disposition_changee_dans_xemu_est_reconnue() {
        assert!(disposition_d_origine(None));
        assert!(disposition_d_origine(Some("[general]\nshow_welcome = false\n")));
        assert!(disposition_d_origine(Some("[input.bindings]\nport1 = '030000005e040000'\n\n[[input.gamepad_mappings]]\ngamepad_id = 'x'\nenable_rumble = true\n\n[input.gamepad_mappings.controller_mapping]\na = 0\nb = 1\ninvert_axis_left_y = false\n")));
        assert!(!disposition_d_origine(Some("[[input.gamepad_mappings]]\ngamepad_id = 'x'\n\n[input.gamepad_mappings.controller_mapping]\na = 1\nb = 0\n")), "A et B échangés");
        assert!(!disposition_d_origine(Some("[[input.gamepad_mappings]]\n[input.gamepad_mappings.controller_mapping]\ninvert_axis_right_y = true\n")), "axe inversé");
        assert!(!disposition_d_origine(Some("[input.bindings]\nport1 = 'keyboard'\n")), "joueur 1 au clavier");
        assert!(!disposition_d_origine(Some("[input]\nauto_bind = false\n")), "manette plus branchée d'office");
        assert!(!disposition_d_origine(Some("[input\n")), "illisible");
        assert!(!disposition_d_origine(Some("[[input.gamepad_mappings]]\ncontroller_mapping = { a = 1 }\n")), "table en ligne : on ne sait pas");
        assert!(!disposition_d_origine(Some("[input.bindings]\nport1 = \"keyboard\"\n")));
    }

    /// Une EEPROM comme xemu la crée (section utilisateur d'eeprom_generation.c), avec sa somme juste.
    fn eeprom_xemu(langue: u32) -> Vec<u8> {
        let mut o = vec![0u8; TAILLE_EEPROM];
        o[0..20].copy_from_slice(&[7u8; 20]); // le reste (chiffré) : jamais touché
        let debut = b"\x00\x00\x00\x00\x47\x4D\x54\x00\x42\x53\x54\x00\x00\x00\x00\x00\x00\x00\x00\x00\x0A\x05\x00\x02\x03\x05\x00\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\xC4\xFF\xFF\xFF";
        o[SECTION..SECTION + debut.len()].copy_from_slice(debut);
        o[LANGUE..LANGUE + 4].copy_from_slice(&langue.to_le_bytes());
        let s = somme(&o[SECTION..SECTION + LONGUEUR]);
        o[SOMME..SOMME + 4].copy_from_slice(&s.to_le_bytes());
        o
    }

    fn xemu_portable() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("xemu.toml"), "").unwrap();
        d
    }

    #[test]
    fn la_somme_est_celle_d_xemu() {
        // Vérifiée à la main : des zéros → !0 ; un report de 32 bits compté (comme `high` dans xemu).
        assert_eq!(somme(&[0u8; 8]), 0xFFFF_FFFF);
        assert_eq!(somme(&[1, 0, 0, 0, 2, 0, 0, 0]), !3u32);
        assert_eq!(somme(&[0xFF, 0xFF, 0xFF, 0xFF, 1, 0, 0, 0]), !(1u32 + 0)); // bas = 0, haut = 1
    }

    #[test]
    fn la_langue_d_xemu_une_seule_fois_et_jamais_par_dessus_un_choix() {
        let d = xemu_portable();
        let x = d.path();
        let reglages = x.join("Profils").join("Seb").join("xemu.toml");
        std::fs::create_dir_all(reglages.parent().unwrap()).unwrap();
        // Pas encore d'EEPROM (xemu jamais lancé) : rien, et réessayé la fois suivante.
        assert!(!regler_langue(x, &reglages, 0x040C).unwrap());
        assert!(!marque(x).exists());
        // Créée par xemu, en anglais, chemin écrit par xemu dans les réglages du profil : le français posé, la somme
        // refaite, le reste intact, l'ancienne à l'abri.
        let f = x.join("eeprom.bin");
        std::fs::write(&f, eeprom_xemu(1)).unwrap();
        std::fs::write(&reglages, format!("[sys.files]\neeprom_path = '{}'\n", f.display())).unwrap();
        assert!(regler_langue(x, &reglages, 0x040C).unwrap());
        assert_eq!(std::fs::read(&f).unwrap(), eeprom_xemu(4));
        assert!(x.join(".frogtend-sauvegardes").is_dir());
        assert!(!x.join("eeprom.bin.frogtend-tmp").exists(), "aucun provisoire laissé");
        // Le chemin tel qu'xemu l'écrit (guillemets doubles, barres échappées) est reconnu aussi.
        let e = xemu_portable();
        let fe = e.path().join("eeprom.bin");
        std::fs::write(&fe, eeprom_xemu(1)).unwrap();
        let re = e.path().join("profil.toml");
        std::fs::write(&re, format!("[sys.files]\neeprom_path = \"{}\"\n", fe.display().to_string().replace('\\', "\\\\"))).unwrap();
        assert!(regler_langue(e.path(), &re, 0x040C).unwrap());
        assert_eq!(std::fs::read(&fe).unwrap(), eeprom_xemu(4));
        // Une seule fois : l'anglais remis par la personne est gardé.
        std::fs::write(&f, eeprom_xemu(1)).unwrap();
        assert!(!regler_langue(x, &reglages, 0x040C).unwrap());
        assert_eq!(std::fs::read(&f).unwrap(), eeprom_xemu(1));
    }

    #[test]
    fn windows_en_anglais_ou_une_langue_absente_de_la_console_ne_changent_rien() {
        for langid in [0x0409u16, 0x0415] {
            let d = xemu_portable();
            let f = d.path().join("eeprom.bin");
            std::fs::write(&f, eeprom_xemu(1)).unwrap();
            assert!(!regler_langue(d.path(), &d.path().join("xemu.toml"), langid).unwrap());
            assert_eq!(std::fs::read(&f).unwrap(), eeprom_xemu(1), "{langid:#06x}");
            assert!(marque(d.path()).exists(), "fait une fois : plus réessayé");
            assert!(!d.path().join(".frogtend-sauvegardes").exists());
        }
    }

    #[test]
    fn un_autre_choix_une_autre_forme_ou_une_autre_eeprom_ne_sont_jamais_touches() {
        // Allemand choisi.
        let d = xemu_portable();
        let f = d.path().join("eeprom.bin");
        std::fs::write(&f, eeprom_xemu(3)).unwrap();
        assert!(!regler_langue(d.path(), &d.path().join("xemu.toml"), 0x040C).unwrap());
        assert_eq!(std::fs::read(&f).unwrap(), eeprom_xemu(3));
        // Une somme fausse, une autre taille.
        let e = xemu_portable();
        let mut fausse = eeprom_xemu(1);
        fausse[SECTION] ^= 1;
        std::fs::write(e.path().join("eeprom.bin"), &fausse).unwrap();
        assert!(!regler_langue(e.path(), &e.path().join("xemu.toml"), 0x040C).unwrap());
        assert_eq!(std::fs::read(e.path().join("eeprom.bin")).unwrap(), fausse);
        let g = xemu_portable();
        std::fs::write(g.path().join("eeprom.bin"), vec![0u8; 512]).unwrap();
        assert!(!regler_langue(g.path(), &g.path().join("xemu.toml"), 0x040C).unwrap());
        // Une EEPROM choisie ailleurs par la personne : ni elle, ni celle d'origine.
        let h = xemu_portable();
        let ailleurs = tempfile::tempdir().unwrap();
        let sienne = ailleurs.path().join("ma-xbox.bin");
        std::fs::write(&sienne, eeprom_xemu(1)).unwrap();
        std::fs::write(h.path().join("eeprom.bin"), eeprom_xemu(1)).unwrap();
        let reglages = h.path().join("profil.toml");
        std::fs::write(&reglages, format!("[sys.files]\neeprom_path = '{}'\n", sienne.display())).unwrap();
        assert!(!regler_langue(h.path(), &reglages, 0x040C).unwrap());
        assert_eq!((std::fs::read(&sienne).unwrap(), std::fs::read(h.path().join("eeprom.bin")).unwrap()), (eeprom_xemu(1), eeprom_xemu(1)));
        // Hors portable (données dans %APPDATA%) : jamais touché.
        let n = tempfile::tempdir().unwrap();
        std::fs::write(n.path().join("eeprom.bin"), eeprom_xemu(1)).unwrap();
        assert!(!regler_langue(n.path(), &n.path().join("xemu.toml"), 0x040C).unwrap());
        assert_eq!((langue_console(0x040C), langue_console(0x080C), langue_console(0x0804), langue_console(0x0404), langue_console(0x0416), langue_console(0x0419)), (Some(4), Some(4), None, Some(8), Some(9), None));
    }
}
