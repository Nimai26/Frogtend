//! Le pilotage des émulateurs par le menu universel en jeu (lot 4 ter). Voir `docs/RECHERCHE-MENU-EN-JEU.md`.
//!
//! Avant chaque partie, Frogtend règle l'émulateur pour que :
//! - il se mette **en pause quand il perd le premier plan** : ouvrir le menu de Frogtend = la pause, le refermer = la
//!   reprise, et la manette ne pilote plus le jeu pendant ce temps ;
//! - il obéisse à des **touches que Frogtend lui envoie** : F13 à F16, qu'aucun clavier n'a (donc sans conflit) ;
//! - RetroArch écoute ses **commandes réseau**, sur la machine seulement (127.0.0.1) ;
//! - Dolphin ne soit pas en plein écran EXCLUSIF (sinon rien ne peut s'afficher par-dessus) ;
//! - quitter ne demande pas de confirmation (c'est le menu de Frogtend qui la demande).
//!
//! Relevé dans les sources officielles (30/09/2026), fichiers et lignes dans la recherche.

use crate::emulateurs_profils::modifier_ini;
use crate::erreurs::Resultat;
use std::path::Path;

/// Le port des commandes réseau de RetroArch (son défaut, `network_cmd_port`).
pub const PORT_RETROARCH: u16 = 55355;

/// Une action du menu en jeu qu'on transmet à l'émulateur.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Reset,
    DisqueSuivant,
    SauverEtat,
    ChargerEtat,
}

/// La touche que Frogtend envoie pour une action (DuckStation, PCSX2, Dolphin). `(touche, avec Maj)`.
pub fn touche(emulateur: &str, a: Action) -> Option<(&'static str, bool)> {
    match (emulateur, a) {
        ("duckstation" | "dolphin", Action::Reset) | ("pcsx2", Action::Reset) => Some(("F13", false)),
        ("duckstation" | "dolphin", Action::DisqueSuivant) => Some(("F14", false)),
        ("duckstation" | "pcsx2" | "dolphin", Action::SauverEtat) => Some(("F15", false)),
        ("duckstation" | "pcsx2", Action::ChargerEtat) => Some(("F16", false)),
        // Dolphin ne connaît que F13 à F15 (DInput) : Maj+F15.
        ("dolphin", Action::ChargerEtat) => Some(("F15", true)),
        _ => None,
    }
}

/// Ce que le menu peut faire dans cet émulateur (en plus de la pause et de quitter, toujours possibles).
pub fn actions(emulateur: &str) -> Vec<Action> {
    use Action::*;
    match emulateur {
        "retroarch" => vec![Reset, DisqueSuivant, SauverEtat, ChargerEtat],
        "duckstation" | "dolphin" => vec![Reset, DisqueSuivant, SauverEtat, ChargerEtat],
        "pcsx2" => vec![Reset, SauverEtat, ChargerEtat],
        _ => vec![],
    }
}

/// La commande réseau de RetroArch pour une action.
pub fn commande_retroarch(a: Action) -> &'static str {
    match a {
        Action::Reset => "RESET",
        Action::DisqueSuivant => "DISK_NEXT",
        Action::SauverEtat => "SAVE_STATE",
        Action::ChargerEtat => "LOAD_STATE",
    }
}

/// Les lignes à ajouter au fichier de réglages de RetroArch (`--appendconfig`).
pub fn lignes_retroarch() -> String {
    format!(
        "network_cmd_enable = \"true\"\nnetwork_cmd_port = \"{PORT_RETROARCH}\"\nnetwork_cmd_bind_address = \"127.0.0.1\"\npause_nonactive = \"true\"\n"
    )
}

fn un(v: &str) -> Vec<String> {
    vec![v.to_string()]
}

/// Règle l'émulateur pour le menu en jeu. `utilisateur` : le dossier utilisateur Dolphin du profil. PPSSPP : ses
/// réglages sont dans `memstick`, qui mène déjà au profil (à appeler après `relier_memstick`).
pub fn preparer(id: &str, emulateur: &Path, utilisateur: Option<&Path>) -> Resultat<()> {
    let clavier = |t: &str| format!("Keyboard/{t}");
    match id {
        "duckstation" => {
            let ini = emulateur.join("settings.ini");
            modifier_ini(
                emulateur,
                &ini,
                "Main",
                &[("PauseOnFocusLoss", un("true")), ("DisableBackgroundInput", un("true")), ("ConfirmPowerOff", un("false"))],
            )?;
            modifier_ini(
                emulateur,
                &ini,
                "Hotkeys",
                &[
                    ("Reset", vec![clavier("F13")]),
                    ("SwitchToNextDisc", vec![clavier("F14")]),
                    ("SaveSelectedSaveState", vec![clavier("F15")]),
                    ("LoadSelectedSaveState", vec![clavier("F16")]),
                ],
            )?;
        }
        "pcsx2" => {
            let ini = emulateur.join("inis").join("PCSX2.ini");
            modifier_ini(emulateur, &ini, "UI", &[("PauseOnFocusLoss", un("true")), ("ConfirmShutdown", un("false"))])?;
            modifier_ini(
                emulateur,
                &ini,
                "Hotkeys",
                &[
                    ("ResetVM", vec![clavier("F13")]),
                    ("SaveStateToSlot1", vec![clavier("F15")]),
                    ("LoadStateFromSlot1", vec![clavier("F16")]),
                ],
            )?;
        }
        "dolphin" => {
            let Some(u) = utilisateur else { return Ok(()) };
            let config = u.join("Config");
            modifier_ini(emulateur, &config.join("Dolphin.ini"), "Interface", &[("PauseOnFocusLost", un("True")), ("ConfirmStop", un("False"))])?;
            modifier_ini(emulateur, &config.join("GFX.ini"), "Settings", &[("BorderlessFullscreen", un("True"))])?;
            let hotkeys = config.join("Hotkeys.ini");
            // Un Hotkeys.ini absent : Dolphin prendrait ses raccourcis d'origine ; en le créant, on les remet
            // (Échap = arrêter, F10 = pause, Alt+Entrée = plein écran) pour ne rien perdre.
            let mut v: Vec<(&str, Vec<String>)> = vec![
                ("Device", un("DInput/0/Keyboard Mouse")),
                ("General/Reset", un("F13")),
                ("General/Change Disc", un("F14")),
                ("Save State/Save to Selected Slot", un("F15")),
                ("Load State/Load from Selected Slot", un("@(Shift+F15)")),
            ];
            if !hotkeys.is_file() {
                v.extend([
                    ("General/Stop", un("ESCAPE")),
                    ("General/Toggle Pause", un("F10")),
                    ("General/Toggle Fullscreen", un("@(Alt+RETURN)")),
                ]);
            }
            modifier_ini(emulateur, &hotkeys, "Hotkeys", &v)?;
        }
        "ppsspp" => {
            let ini = emulateur.join("memstick").join("PSP").join("SYSTEM").join("ppsspp.ini");
            modifier_ini(emulateur, &ini, "General", &[("PauseOnLostFocus", un("True"))])?;
        }
        "dosbox-staging" => {
            let conf = emulateur.join("dosbox-staging.conf");
            modifier_ini(emulateur, &conf, "sdl", &[("pause_when_inactive", un("true"))])?;
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manettes::lire_ini;

    #[test]
    fn duckstation_et_pcsx2_se_mettent_en_pause_et_ecoutent_f13_a_f16() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("settings.ini"), "[Main]\r\nSettingsVersion = 3\r\n\r\n[Hotkeys]\r\nTogglePause = Keyboard/Space\r\n").unwrap();
        preparer("duckstation", d.path(), None).unwrap();
        let t = std::fs::read_to_string(d.path().join("settings.ini")).unwrap();
        assert_eq!(lire_ini(&t, "Main", "PauseOnFocusLoss"), vec!["true"]);
        assert_eq!(lire_ini(&t, "Main", "DisableBackgroundInput"), vec!["true"]);
        assert_eq!(lire_ini(&t, "Hotkeys", "Reset"), vec!["Keyboard/F13"]);
        assert_eq!(lire_ini(&t, "Hotkeys", "TogglePause"), vec!["Keyboard/Space"], "les raccourcis de la personne restent");
        assert_eq!(lire_ini(&t, "Main", "SettingsVersion"), vec!["3"]);

        preparer("pcsx2", d.path(), None).unwrap();
        let t = std::fs::read_to_string(d.path().join("inis").join("PCSX2.ini")).unwrap();
        assert_eq!(lire_ini(&t, "UI", "PauseOnFocusLoss"), vec!["true"]);
        assert_eq!(lire_ini(&t, "Hotkeys", "LoadStateFromSlot1"), vec!["Keyboard/F16"]);
    }

    #[test]
    fn dolphin_passe_sans_bordure_et_garde_ses_raccourcis_d_origine() {
        let d = tempfile::tempdir().unwrap();
        let u = d.path().join("User");
        preparer("dolphin", d.path(), Some(&u)).unwrap();
        let c = u.join("Config");
        assert_eq!(lire_ini(&std::fs::read_to_string(c.join("GFX.ini")).unwrap(), "Settings", "BorderlessFullscreen"), vec!["True"]);
        assert_eq!(lire_ini(&std::fs::read_to_string(c.join("Dolphin.ini")).unwrap(), "Interface", "PauseOnFocusLost"), vec!["True"]);
        let h = std::fs::read_to_string(c.join("Hotkeys.ini")).unwrap();
        assert_eq!(lire_ini(&h, "Hotkeys", "General/Reset"), vec!["F13"]);
        assert_eq!(lire_ini(&h, "Hotkeys", "General/Stop"), vec!["ESCAPE"], "créé : les raccourcis d'origine sont remis");

        // Un Hotkeys.ini déjà là : on n'y remet pas nos défauts (la personne a peut-être changé Échap).
        std::fs::write(c.join("Hotkeys.ini"), "[Hotkeys]\r\nGeneral/Stop = Q\r\n").unwrap();
        preparer("dolphin", d.path(), Some(&u)).unwrap();
        let h = std::fs::read_to_string(c.join("Hotkeys.ini")).unwrap();
        assert_eq!(lire_ini(&h, "Hotkeys", "General/Stop"), vec!["Q"]);
    }

    #[test]
    fn ppsspp_et_dosbox_se_mettent_en_pause() {
        let d = tempfile::tempdir().unwrap();
        preparer("ppsspp", d.path(), None).unwrap();
        let t = std::fs::read_to_string(d.path().join("memstick/PSP/SYSTEM/ppsspp.ini")).unwrap();
        assert_eq!(lire_ini(&t, "General", "PauseOnLostFocus"), vec!["True"]);
        std::fs::write(d.path().join("dosbox-staging.conf"), "[sdl]\nfullscreen = true\n\n[cpu]\ncycles = auto\n").unwrap();
        preparer("dosbox-staging", d.path(), None).unwrap();
        let t = std::fs::read_to_string(d.path().join("dosbox-staging.conf")).unwrap();
        assert_eq!(lire_ini(&t, "sdl", "pause_when_inactive"), vec!["true"]);
        assert_eq!(lire_ini(&t, "cpu", "cycles"), vec!["auto"]);
    }

    #[test]
    fn chaque_action_a_son_moyen() {
        for e in ["retroarch", "duckstation", "pcsx2", "dolphin"] {
            for a in actions(e) {
                if e == "retroarch" {
                    assert!(!commande_retroarch(a).is_empty());
                } else {
                    assert!(touche(e, a).is_some(), "{e} {a:?}");
                }
            }
        }
        assert!(!actions("pcsx2").contains(&Action::DisqueSuivant), "PCSX2 ne sait pas changer de disque de l'extérieur");
        assert!(lignes_retroarch().contains("network_cmd_bind_address = \"127.0.0.1\""));
    }
}
