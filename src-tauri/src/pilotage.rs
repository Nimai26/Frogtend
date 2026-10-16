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
    // Les options du jeu (lot OSD, i) : des bascules, sans état connu de Frogtend (l'émulateur le tient).
    /// Afficher ou masquer les images par seconde.
    ImagesParSeconde,
    /// Avance rapide (activer ou couper).
    AvanceRapide,
    /// Une capture d'écran (rangée par l'émulateur, dans SON dossier de captures).
    Capture,
    /// Couper ou remettre le son.
    Muet,
}

/// Une touche envoyée par Frogtend, avec ses modificateurs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Touche {
    pub touche: &'static str,
    pub maj: bool,
    pub ctrl: bool,
    pub alt: bool,
}

const fn t(touche: &'static str) -> Touche {
    Touche { touche, maj: false, ctrl: false, alt: false }
}

/// La touche que Frogtend envoie pour une action (DuckStation, PCSX2, Dolphin : réglées par Frogtend ; DOSBox
/// Staging : ses raccourcis d'origine).
pub fn touche(emulateur: &str, a: Action) -> Option<Touche> {
    match (emulateur, a) {
        ("duckstation" | "pcsx2" | "dolphin", Action::Reset) => Some(t("F13")),
        ("duckstation" | "dolphin", Action::DisqueSuivant) => Some(t("F14")),
        ("duckstation" | "pcsx2" | "dolphin", Action::SauverEtat) => Some(t("F15")),
        ("duckstation" | "pcsx2", Action::ChargerEtat) => Some(t("F16")),
        // Dolphin ne connaît que F13 à F15 (DInput) : Maj+F15.
        ("dolphin", Action::ChargerEtat) => Some(Touche { maj: true, ..t("F15") }),
        // DOSBox Staging (sdl_gui.cpp, bios_disk.cpp) : Ctrl+Alt+Début redémarre, Ctrl+F4 change de disque.
        ("dosbox-staging", Action::Reset) => Some(Touche { ctrl: true, alt: true, ..t("Home") }),
        ("dosbox-staging", Action::DisqueSuivant) => Some(Touche { ctrl: true, ..t("F4") }),
        _ => None,
    }
}

/// Ce que le menu peut faire dans cet émulateur (en plus de reprendre et de quitter, toujours possibles).
pub fn actions(emulateur: &str) -> Vec<Action> {
    use Action::*;
    match emulateur {
        // RetroArch : ses commandes réseau (command.h de la 1.22.2 : FPS_TOGGLE, FAST_FORWARD, SCREENSHOT, MUTE).
        "retroarch" => vec![Reset, DisqueSuivant, SauverEtat, ChargerEtat, ImagesParSeconde, AvanceRapide, Capture, Muet],
        "duckstation" | "dolphin" => vec![Reset, DisqueSuivant, SauverEtat, ChargerEtat],
        "pcsx2" => vec![Reset, SauverEtat, ChargerEtat],
        "dosbox-staging" => vec![Reset, DisqueSuivant],
        _ => vec![],
    }
}

/// L'émulateur se met-il en pause quand il perd le premier plan (réglé par `preparer` ; RPCS3 par
/// `commandes::preparer_emulateur`) ?
pub fn se_met_en_pause(emulateur: &str) -> bool {
    matches!(emulateur, "retroarch" | "duckstation" | "pcsx2" | "dolphin" | "ppsspp" | "dosbox-staging" | "rpcs3" | "mame" | "eden" | "azahar")
}

/// Les émulateurs SANS pause réglable (code source relu le 16/10 : Cemu, Xenia Canary, xemu, Vita3K — au mieux une
/// touche qui BASCULE la pause, et la manette lue en arrière-plan pour Cemu, Xenia et Vita3K) : FIGÉS d'office pendant
/// le menu (décision de Seb, 16/10 : « Oui, toujours »), comme un jeu PC dont l'option est cochée. Le menu ne dit « en
/// pause » qu'une fois le gel réussi (`menu-jeu-pause`).
pub fn fige_au_menu(emulateur: &str) -> bool {
    matches!(emulateur, "cemu" | "xenia" | "xemu" | "vita3k")
}

/// Eden et Azahar : leur pause en arrière-plan est-elle VRAIMENT réglée (après `preparer`) ? Sinon (pas portable, ou
/// « pas de pause » choisi par la personne), ils sont figés pendant le menu comme ceux de `fige_au_menu`.
pub fn pause_reglee(id: &str, emulateur: &Path) -> bool {
    match id {
        "eden" | "azahar" => std::fs::read_to_string(emulateur.join("user").join("config").join("qt-config.ini"))
            .is_ok_and(|t| t.lines().any(|l| l.replace(' ', "") == "pauseWhenInBackground=true")),
        _ => true,
    }
}

/// MAME n'a pas de pause à la perte du premier plan (demande ouverte depuis 2019, mametesters 7339). Frogtend le
/// lance avec un petit script Lua (`-autoboot_script`, officiel) qui se met en pause tant qu'un fichier témoin
/// existe : Frogtend le crée à l'ouverture du menu, le retire à sa fermeture. Le script tourne aussi pendant la pause
/// (MAME 0.289 : `machine.cpp` appelle `frame_update` en pause, qui appelle les rappels périodiques, `video.cpp:253`).
/// Il ne relance QUE ce qu'il a mis en pause : une pause faite par la personne (touche P) reste.
/// Chemins RELATIFS au dossier de MAME (son dossier de travail au lancement) : aucun souci d'accents dans un chemin.
pub const DOSSIER_MAME: &str = "frogtend";
pub const SCRIPT_MAME: &str = "frogtend\\menu-en-jeu.lua";
pub const TEMOIN_MAME: &str = "frogtend\\menu-ouvert";

const LUA_MAME: &str = r#"-- Frogtend : la pause du menu en jeu. Écrit par Frogtend à chaque partie (ne pas modifier : il serait refait).
-- Tant que le fichier témoin existe (menu de Frogtend ouvert), MAME est en pause ; ensuite il repart, seulement si
-- c'est ce script qui l'avait mis en pause.
-- Une seule fois, même si MAME relance le script à chaque remise à zéro (F3).
if frogtend_menu_en_jeu then return end
frogtend_menu_en_jeu = true
local temoin = "frogtend\\menu-ouvert"
local par_frogtend = false
local tours = 0
emu.register_periodic(function()
  tours = tours + 1
  if tours < 10 then return end
  tours = 0
  local f = io.open(temoin, "r")
  local ouvert = f ~= nil
  if f then f:close() end
  local m = manager.machine
  if ouvert and not par_frogtend and not m.paused then
    emu.pause()
    par_frogtend = true
  elseif not ouvert and par_frogtend then
    if m.paused then emu.unpause() end
    par_frogtend = false
  end
end)
"#;

/// Prépare MAME pour le menu en jeu, dans SON dossier : le script (réécrit seulement s'il a changé) et aucun témoin
/// resté d'une partie précédente (sinon le jeu démarrerait en pause).
pub fn preparer_mame(dossier_mame: &Path) -> Resultat<()> {
    std::fs::create_dir_all(dossier_mame.join(DOSSIER_MAME))?;
    let script = dossier_mame.join(SCRIPT_MAME);
    if std::fs::read_to_string(&script).ok().as_deref() != Some(LUA_MAME) {
        std::fs::write(&script, LUA_MAME)?;
    }
    let _ = std::fs::remove_file(dossier_mame.join(TEMOIN_MAME));
    Ok(())
}

/// Le menu s'ouvre (`true`) ou se ferme : le témoin que le script de MAME surveille.
pub fn menu_mame(dossier_mame: &Path, ouvert: bool) {
    let t = dossier_mame.join(TEMOIN_MAME);
    if ouvert {
        if t.parent().is_some_and(|d| d.is_dir()) {
            let _ = std::fs::write(&t, b"");
        }
    } else {
        let _ = std::fs::remove_file(&t);
    }
}

/// La commande réseau de RetroArch pour une action.
pub fn commande_retroarch(a: Action) -> &'static str {
    match a {
        Action::Reset => "RESET",
        Action::DisqueSuivant => "DISK_NEXT",
        Action::SauverEtat => "SAVE_STATE",
        Action::ChargerEtat => "LOAD_STATE",
        Action::ImagesParSeconde => "FPS_TOGGLE",
        Action::AvanceRapide => "FAST_FORWARD",
        Action::Capture => "SCREENSHOT",
        Action::Muet => "MUTE",
    }
}

/// Les lignes à ajouter au fichier de réglages de RetroArch (`--appendconfig`).
pub fn lignes_retroarch() -> String {
    format!(
        "network_cmd_enable = \"true\"\nnetwork_cmd_port = \"{PORT_RETROARCH}\"\nnetwork_cmd_bind_address = \"127.0.0.1\"\npause_nonactive = \"true\"\ninput_menu_toggle_gamepad_combo = \"0\"\n"
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
        // Eden et Azahar (Qt) : `[UI] pauseWhenInBackground` (Eden uisettings.h:113-119, main_window.cpp:1404-1420 ;
        // Azahar uisettings.h:92, citra_qt.cpp:1035-1051) — ils ne reprennent que s'ils avaient eux-mêmes mis en pause.
        // QSettings ignore la valeur tant que `\default` vaut true : les deux lignes. Seulement en mode portable
        // (dossier `user` déjà là) : le créer ferait basculer l'émulateur en portable, loin des réglages de la personne.
        "eden" | "azahar" => {
            let user = emulateur.join("user");
            let ini = user.join("config").join("qt-config.ini");
            // « Pas de pause » réglé par la personne elle-même (les deux lignes à false) : jamais défait (l'émulateur
            // sera figé à la place, voir `pause_reglee`).
            let choix_de_la_personne = std::fs::read_to_string(&ini).is_ok_and(|t| {
                let l: Vec<String> = t.lines().map(|l| l.replace(' ', "")).collect();
                l.iter().any(|x| x == "pauseWhenInBackground\\default=false") && l.iter().any(|x| x == "pauseWhenInBackground=false")
            });
            if user.is_dir() && !choix_de_la_personne {
                modifier_ini(
                    emulateur,
                    &ini,
                    "UI",
                    &[("pauseWhenInBackground\\default", un("false")), ("pauseWhenInBackground", un("true"))],
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    /// RÉEL : le vrai MAME (`FROGTEND_MAME` = son dossier, pas dans Temp) lance Pong (sans ROM) avec le script de
    /// Frogtend, plus un journal des pauses ; Frogtend crée puis retire le témoin. Attendu : « pause » puis « reprise ».
    /// `FROGTEND_MAME=<dossier> cargo test essai_pause_mame -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn essai_pause_mame() {
        let d = std::path::PathBuf::from(std::env::var("FROGTEND_MAME").unwrap());
        super::preparer_mame(&d).unwrap();
        let journal = d.join("frogtend").join("journal.txt");
        let _ = std::fs::remove_file(&journal);
        let essai = format!(
            "{}\nlocal function noter(t) local f = io.open(\"frogtend\\\\journal.txt\", \"a\") f:write(t .. \"\\n\") f:close() end\nfrogtend_essai = {{ emu.add_machine_pause_notifier(function() noter(\"pause\") end), emu.add_machine_resume_notifier(function() noter(\"reprise\") manager.machine:exit() end) }}\n",
            super::LUA_MAME
        );
        std::fs::write(d.join("frogtend").join("essai.lua"), essai).unwrap();
        let mut p = std::process::Command::new(d.join("mame.exe"))
            .args(["pong", "-window", "-nomax", "-skip_gameinfo", "-sound", "none", "-autoboot_script", "frogtend\\essai.lua"])
            .current_dir(&d)
            .spawn()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_secs(6));
        super::menu_mame(&d, true);
        std::thread::sleep(std::time::Duration::from_secs(3));
        println!("JOURNAL APRÈS OUVERTURE : {:?}", std::fs::read_to_string(&journal).unwrap_or_default());
        super::menu_mame(&d, false);
        let debut = std::time::Instant::now();
        let fin = loop {
            if let Some(s) = p.try_wait().unwrap() {
                break Some(s);
            }
            if debut.elapsed().as_secs() > 20 {
                let _ = p.kill();
                break None;
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        };
        println!("SORTIE {fin:?} ; JOURNAL FINAL : {:?}", std::fs::read_to_string(&journal).unwrap_or_default());
    }

    /// RÉEL : la préparation du menu en jeu sur un vrai Eden ou Azahar portable (`FROGTEND_EMU` = id,
    /// `FROGTEND_DOSSIER` = son dossier). Vérifier ensuite que l'émulateur relit et réécrit la valeur. ⚠️ Il ÉCRIT dans
    /// ce dossier : jamais sur l'émulateur de la personne, seulement sur une COPIE d'essai.
    /// `FROGTEND_EMU=azahar FROGTEND_DOSSIER=<dossier> cargo test essai_pause_qt -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn essai_pause_qt() {
        let id = std::env::var("FROGTEND_EMU").unwrap();
        let d = std::path::PathBuf::from(std::env::var("FROGTEND_DOSSIER").unwrap());
        super::preparer(&id, &d, None).unwrap();
        let t = std::fs::read_to_string(d.join("user").join("config").join("qt-config.ini")).unwrap();
        println!("APRÈS FROGTEND : {:?}", t.lines().filter(|l| l.contains("pauseWhenInBackground")).collect::<Vec<_>>());
    }

    #[test]
    fn eden_et_azahar_se_mettent_en_pause_en_arriere_plan_seulement_en_portable() {
        for id in ["eden", "azahar"] {
            let d = tempfile::tempdir().unwrap();
            // Pas de dossier « user » : rien n'est créé (l'émulateur resterait près des réglages de la personne).
            super::preparer(id, d.path(), None).unwrap();
            assert!(!d.path().join("user").exists(), "{id}");
            // Portable : la pause est réglée, le reste du fichier gardé.
            let ini = d.path().join("user").join("config").join("qt-config.ini");
            std::fs::create_dir_all(ini.parent().unwrap()).unwrap();
            std::fs::write(&ini, "[UI]\nfullscreen\\default=false\nfullscreen=true\n").unwrap();
            super::preparer(id, d.path(), None).unwrap();
            let t = std::fs::read_to_string(&ini).unwrap();
            assert!(t.contains("pauseWhenInBackground\\default=false") || t.contains("pauseWhenInBackground\\default = false"), "{id} : {t}");
            assert!(t.contains("fullscreen=true") || t.contains("fullscreen = true"), "{id} : {t}");
            assert!(super::se_met_en_pause(id));
        }
        for id in ["cemu", "xenia", "xemu", "vita3k"] {
            assert!(super::fige_au_menu(id) && !super::se_met_en_pause(id), "{id}");
        }
        // Réglée seulement si le fichier le dit ; « pas de pause » choisi par la personne : jamais défait.
        let d = tempfile::tempdir().unwrap();
        assert!(!super::pause_reglee("eden", d.path()), "pas portable");
        let ini = d.path().join("user").join("config").join("qt-config.ini");
        std::fs::create_dir_all(ini.parent().unwrap()).unwrap();
        std::fs::write(&ini, "[UI]\npauseWhenInBackground\\default=false\npauseWhenInBackground=false\n").unwrap();
        super::preparer("eden", d.path(), None).unwrap();
        assert!(!super::pause_reglee("eden", d.path()), "le choix de la personne est gardé");
        std::fs::write(&ini, "[UI]\npauseWhenInBackground\\default=true\npauseWhenInBackground=false\n").unwrap();
        super::preparer("eden", d.path(), None).unwrap();
        assert!(super::pause_reglee("eden", d.path()), "la valeur d'origine de l'émulateur est remplacée");
        assert!(super::pause_reglee("retroarch", d.path()));
        assert!(!super::fige_au_menu("retroarch"));
    }

    #[test]
    fn mame_se_met_en_pause_par_son_script_et_son_temoin() {
        let d = tempfile::tempdir().unwrap();
        // Un témoin resté d'une partie précédente (Frogtend fermé de force) : retiré à la préparation.
        std::fs::create_dir_all(d.path().join(super::DOSSIER_MAME)).unwrap();
        std::fs::write(d.path().join(super::TEMOIN_MAME), b"").unwrap();
        super::preparer_mame(d.path()).unwrap();
        let script = std::fs::read_to_string(d.path().join(super::SCRIPT_MAME)).unwrap();
        assert!(script.contains("emu.pause()") && script.contains("emu.unpause()") && script.contains("par_frogtend"));
        assert!(script.contains(r#""frogtend\\menu-ouvert""#), "le même témoin que Frogtend, en chemin relatif");
        assert!(!d.path().join(super::TEMOIN_MAME).exists());
        super::menu_mame(d.path(), true);
        assert!(d.path().join(super::TEMOIN_MAME).exists());
        super::menu_mame(d.path(), false);
        assert!(!d.path().join(super::TEMOIN_MAME).exists());
        assert!(super::se_met_en_pause("mame"));
        // Sans dossier préparé (un MAME lancé autrement) : rien n'est créé.
        let vide = tempfile::tempdir().unwrap();
        super::menu_mame(vide.path(), true);
        assert!(!vide.path().join(super::TEMOIN_MAME).exists());
    }

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
        for e in ["retroarch", "duckstation", "pcsx2", "dolphin", "dosbox-staging"] {
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
