//! Xenia Canary (Xbox 360) : la langue de la console dès qu'elle est connue (règle de Seb, 08/11 : « configurer TOUS
//! les émulateurs […] à chaque nouvelle installation »). Relevé dans le code de Xenia Canary (branche
//! canary_experimental, 10/11/2026) :
//! - la langue n'est plus une option : elle est dans la configuration de la console, un fichier BINAIRE
//!   `<storage_root>\xconfig.settings` (kernel/kernel_state.cc), `storage_root` = le dossier du programme en portable
//!   (app/xenia_main.cc, portable d'office sous Windows) ;
//! - Xenia le crée à son premier démarrage avec ses valeurs d'origine (kernel/xconfig.cc `SetDefaults` : anglais,
//!   États-Unis, fuseau, son…) et le relit tel quel (`fread` de `sizeof(XConfigData)`, aucune somme de contrôle lue) ;
//!   Frogtend ne le crée JAMAIS (il faudrait refaire toutes ces valeurs) : la langue est posée à la partie suivante ;
//! - `XConfigData` (kernel/xconfig.h, `#pragma pack(1)`) : sections Static 270 o, Statistic 1 496, Secured 512, User
//!   509, XNetMachineAccount 496, XNetParameters 13, MediaCenter 1 868, Console 324, DVD 664, IPTV 512, System 16 =
//!   6 680 octets ; `User.language` (grand-boutiste, 32 bits) à 0x2C dans User (static_assert) = octet 2 322 ;
//! - les langues (xbox.h `XLanguage`) : anglais 1, japonais 2, allemand 3, français 4, espagnol 5, italien 6, coréen 7,
//!   chinois traditionnel 8, portugais 9, polonais 11, russe 12, suédois 13, turc 14, norvégien 15, néerlandais 16,
//!   chinois simplifié 17 ; les jeux lisent cette valeur (kernel/xam/xam_locale.cc `xeGetLanguage`) ;
//! - le bouton Xbox : transmis au jeu tant que `guide_button` est vrai (hid/hid_flags.cc ; lu par XInputGetStateEx,
//!   hid/xinput/xinput_input_driver.cc ; le pilote SDL le respecte aussi, hid/sdl/sdl_input_driver.cc). Frogtend passe `--guide_button=false` à chaque partie : Xenia lit alors
//!   XInputGetState, qui ne le voit pas, et le bouton reste au menu de Frogtend (ses raccourcis avec le bouton Xbox,
//!   `controller_hotkeys`, sont coupés d'origine).

use crate::erreurs::{Erreur, Resultat};
use std::path::{Path, PathBuf};

/// La taille du fichier `xconfig.settings` que Xenia écrit (`sizeof(XConfigData)`, voir plus haut).
const TAILLE_XCONFIG: usize = 6680;
/// L'octet de `User.language` dans ce fichier.
const LANGUE: usize = 2322;
/// L'anglais : la valeur d'origine de Xenia.
const ANGLAIS: u32 = 1;

/// L'option qui laisse le bouton Xbox au menu de Frogtend.
pub const BOUTON_XBOX_LIBRE: &str = "--guide_button=false";

fn fichier_xconfig(xenia: &Path) -> PathBuf {
    xenia.join("xconfig.settings")
}

fn marque(xenia: &Path) -> PathBuf {
    xenia.join(".frogtend-langue")
}

/// La langue de la console Xbox 360 pour une langue de Windows (LANGID) ; `None` : une langue que la console n'a pas.
pub fn langue_console(langid: u16) -> Option<u32> {
    Some(match langid & 0x3FF {
        0x09 => 1,
        0x11 => 2,
        0x07 => 3,
        0x0C => 4,
        0x0A => 5,
        0x10 => 6,
        0x12 => 7,
        // Chinois : traditionnel à Taïwan, Hong Kong et Macao (et le traditionnel neutre), simplifié ailleurs.
        0x04 if matches!(langid, 0x0404 | 0x0C04 | 0x1404 | 0x7C04) => 8,
        0x04 => 17,
        0x16 => 9,
        0x15 => 11,
        0x19 => 12,
        0x1D => 13,
        0x1F => 14,
        0x14 => 15,
        0x13 => 16,
        _ => return None,
    })
}

/// La langue de la console prise de Windows, UNE seule fois par Xenia (marque) : seulement Xenia FERMÉ (il réécrit le
/// fichier quand un jeu change un réglage), seulement un `xconfig.settings` de la taille connue (une autre version de
/// Xenia : jamais touché) et encore en anglais (la valeur d'origine). Absent : rien (Xenia le crée à sa première
/// partie ; la langue sera posée à la suivante). L'ancien fichier est copié à l'abri ; écrit par un provisoire
/// renommé, puis relu. Rend vrai si la langue a été posée.
pub fn regler_langue(xenia: &Path, langid: u16) -> Resultat<bool> {
    if marque(xenia).exists() || !crate::lancement::processus_du_dossier(xenia).is_empty() {
        return Ok(false);
    }
    let f = fichier_xconfig(xenia);
    // La taille d'abord (jamais lire un fichier inattendu en entier).
    match std::fs::metadata(&f) {
        Ok(m) if m.len() == TAILLE_XCONFIG as u64 => {}
        Ok(_) => return Ok(false),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.into()),
    }
    let octets = std::fs::read(&f)?;
    if octets.len() != TAILLE_XCONFIG {
        return Ok(false);
    }
    let lue = |o: &[u8]| u32::from_be_bytes([o[LANGUE], o[LANGUE + 1], o[LANGUE + 2], o[LANGUE + 3]]);
    let mut pose = false;
    if let Some(n) = langue_console(langid).filter(|n| *n != ANGLAIS && lue(&octets) == ANGLAIS) {
        let mut nouveau = octets.clone();
        nouveau[LANGUE..LANGUE + 4].copy_from_slice(&n.to_be_bytes());
        crate::emulateurs_profils::mettre_a_l_abri(xenia, &f)?;
        let provisoire = f.with_extension("settings.frogtend-tmp");
        std::fs::write(&provisoire, &nouveau)?;
        if let Err(e) = std::fs::rename(&provisoire, &f) {
            let _ = std::fs::remove_file(&provisoire);
            return Err(e.into());
        }
        if std::fs::read(&f).ok().as_deref() != Some(nouveau.as_slice()) {
            return Err(Erreur::Disque("La langue de Xenia n'a pas pu être posée.".into()));
        }
        pose = true;
    }
    std::fs::write(marque(xenia), "")?;
    Ok(pose)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xconfig(langue: u32) -> Vec<u8> {
        let mut o = vec![0u8; TAILLE_XCONFIG];
        o[LANGUE..LANGUE + 4].copy_from_slice(&langue.to_be_bytes());
        o[LANGUE + 20] = 103; // le pays (États-Unis), juste après : jamais touché
        o
    }

    #[test]
    fn la_langue_de_xenia_une_seule_fois_et_jamais_par_dessus_un_choix() {
        let d = tempfile::tempdir().unwrap();
        let x = d.path();
        // Pas encore de fichier (Xenia jamais lancé) : rien, et réessayé la fois suivante.
        assert!(!regler_langue(x, 0x040C).unwrap());
        assert!(!marque(x).exists());
        // Créé par Xenia, en anglais : le français posé, le reste intact, l'ancien à l'abri.
        std::fs::write(fichier_xconfig(x), xconfig(1)).unwrap();
        assert!(regler_langue(x, 0x040C).unwrap());
        let o = std::fs::read(fichier_xconfig(x)).unwrap();
        assert_eq!(o, { let mut a = xconfig(1); a[LANGUE + 3] = 4; a });
        assert!(x.join(".frogtend-sauvegardes").is_dir());
        assert!(!x.join("xconfig.settings.frogtend-tmp").exists(), "aucun provisoire laissé");
        // Une seule fois : l'anglais remis par la personne (réglages de la console) est gardé.
        std::fs::write(fichier_xconfig(x), xconfig(1)).unwrap();
        assert!(!regler_langue(x, 0x040C).unwrap());
        assert_eq!(std::fs::read(fichier_xconfig(x)).unwrap(), xconfig(1));
    }

    #[test]
    fn windows_en_anglais_ou_une_langue_absente_de_la_console_ne_changent_rien() {
        for langid in [0x0409u16, 0x0418] {
            let d = tempfile::tempdir().unwrap();
            std::fs::write(fichier_xconfig(d.path()), xconfig(1)).unwrap();
            assert!(!regler_langue(d.path(), langid).unwrap());
            assert_eq!(std::fs::read(fichier_xconfig(d.path())).unwrap(), xconfig(1), "{langid:#06x}");
            assert!(marque(d.path()).exists(), "fait une fois : plus réessayé");
            assert!(!d.path().join(".frogtend-sauvegardes").exists());
        }
    }

    #[test]
    fn un_autre_choix_ou_une_autre_forme_ne_sont_jamais_touches() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(fichier_xconfig(d.path()), xconfig(3)).unwrap();
        assert!(!regler_langue(d.path(), 0x040C).unwrap(), "allemand choisi");
        assert_eq!(std::fs::read(fichier_xconfig(d.path())).unwrap(), xconfig(3));
        let e = tempfile::tempdir().unwrap();
        std::fs::write(fichier_xconfig(e.path()), vec![0u8; 6000]).unwrap();
        assert!(!regler_langue(e.path(), 0x040C).unwrap(), "une autre version de Xenia");
        assert_eq!(std::fs::read(fichier_xconfig(e.path())).unwrap().len(), 6000);
        assert_eq!((langue_console(0x040C), langue_console(0x0C0C), langue_console(0x0804), langue_console(0x0404), langue_console(0x0418)), (Some(4), Some(4), Some(17), Some(8), None));
    }
}
