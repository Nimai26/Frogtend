//! Les profils de manette standard (lot 4) : une manette branchée marche d'emblée dans chaque émulateur, en
//! joueur 1. Frogtend règle la manette seulement si le joueur 1 n'en a encore aucune (on ne défait jamais les
//! réglages de la personne), sauf quand elle demande « remettre la manette par défaut ». Les touches du clavier
//! déjà réglées sont gardées.
//!
//! Relevé dans les sources officielles (30/09/2026) :
//! - DuckStation et PCSX2 : source SDL active par défaut (Xbox, PS4/PS5, Switch Pro, 8BitDo, génériques).
//!   Liaisons `SDL-0/DPadUp`, `SDL-0/+LeftTrigger`, `SDL-0/-LeftX`… dans `[Pad1]` ; une touche peut avoir
//!   plusieurs liaisons (clé répétée) ; une combinaison s'écrit `A & B`.
//! - Dolphin : `Config\GCPadNew.ini [GCPad1]`, `Device = XInput/0/Gamepad`, entrées `` `Button A` ``, `` `Left Y+` ``…
//!   (manettes XInput seulement : une manette PlayStation y passe par Steam ou DS4Windows).
//! - RetroArch : profils officiels `autoconfig\` (xinput, dinput, sdl2), détection automatique active par défaut ;
//!   sans combinaison de menu à la manette sous Windows : Frogtend met L3 + R3 (`input_menu_toggle_gamepad_combo = 2`).
//! - PPSSPP (XInput et DInput par défaut sous Windows) et DOSBox Staging (`joysticktype = auto`) : rien à régler.
//! - RPCS3 (relevé dans son code, 03/10/2026 : Emu/Io/pad_config.h, Input/pad_thread.cpp, Input/xinput_pad_handler.cpp,
//!   Emu/system_utils.cpp) : `config\input_configs\global\Default.yml`, clé `Player 1 Input` avec `Handler` et
//!   `Device` ; les touches absentes reçoivent celles par défaut du handler (XInput : Cross = A, Circle = B…). UN seul
//!   handler par joueur, et une manette XInput absente laisse le joueur SANS commandes : Frogtend choisit donc à
//!   chaque partie la manette Xbox branchée (`XInput Pad #n`), sinon le clavier (Cross = X, Start = Entrée…). Il ne
//!   gère que le fichier qu'il a écrit (sa marque en 1re ligne) : un réglage fait dans RPCS3 n'est jamais défait.

use crate::emulateurs_profils::modifier_ini;
use crate::erreurs::Resultat;
use std::path::Path;

/// Les préfixes des liaisons à une manette (par opposition au clavier et à la souris).
const MANETTE: &[&str] = &["SDL-", "XInput-", "DInput-", "GameInput-"];

/// La manette PlayStation des deux émulateurs PlayStation, en SDL (disposition de la manette Xbox, par position).
const PAD_PLAYSTATION: &[(&str, &str)] = &[
    ("Up", "DPadUp"),
    ("Right", "DPadRight"),
    ("Down", "DPadDown"),
    ("Left", "DPadLeft"),
    ("Triangle", "Y"),
    ("Circle", "B"),
    ("Cross", "A"),
    ("Square", "X"),
    ("Select", "Back"),
    ("Start", "Start"),
    ("L1", "LeftShoulder"),
    ("R1", "RightShoulder"),
    ("L2", "+LeftTrigger"),
    ("R2", "+RightTrigger"),
    ("L3", "LeftStick"),
    ("R3", "RightStick"),
    ("LLeft", "-LeftX"),
    ("LRight", "+LeftX"),
    ("LDown", "+LeftY"),
    ("LUp", "-LeftY"),
    ("RLeft", "-RightX"),
    ("RRight", "+RightX"),
    ("RDown", "+RightY"),
    ("RUp", "-RightY"),
    ("LargeMotor", "LargeMotor"),
    ("SmallMotor", "SmallMotor"),
];

/// La manette GameCube de Dolphin sur une manette XInput (boutons à la même place que sur la GameCube).
const PAD_GAMECUBE: &[(&str, &str)] = &[
    ("Buttons/A", "`Button A`"),
    ("Buttons/B", "`Button X`"),
    ("Buttons/X", "`Button B`"),
    ("Buttons/Y", "`Button Y`"),
    ("Buttons/Z", "`Shoulder R`"),
    ("Buttons/Start", "Start"),
    ("Main Stick/Up", "`Left Y+`"),
    ("Main Stick/Down", "`Left Y-`"),
    ("Main Stick/Left", "`Left X-`"),
    ("Main Stick/Right", "`Left X+`"),
    ("C-Stick/Up", "`Right Y+`"),
    ("C-Stick/Down", "`Right Y-`"),
    ("C-Stick/Left", "`Right X-`"),
    ("C-Stick/Right", "`Right X+`"),
    ("Triggers/L", "`Trigger L`"),
    ("Triggers/R", "`Trigger R`"),
    ("Triggers/L-Analog", "`Trigger L`"),
    ("Triggers/R-Analog", "`Trigger R`"),
    ("D-Pad/Up", "`Pad N`"),
    ("D-Pad/Down", "`Pad S`"),
    ("D-Pad/Left", "`Pad W`"),
    ("D-Pad/Right", "`Pad E`"),
    ("Rumble/Motor", "`Motor L` | `Motor R`"),
];

/// La combinaison de RetroArch pour ouvrir son menu à la manette : L3 + R3.
pub const RETROARCH_COMBO_MENU: &str = "2";

/// Les valeurs d'une clé (répétée ou non) dans une section d'un INI.
pub fn lire_ini(texte: &str, section: &str, cle: &str) -> Vec<String> {
    let entete = format!("[{section}]").to_lowercase();
    let mut dedans = false;
    let mut v = Vec::new();
    for l in texte.lines() {
        let t = l.trim();
        if t.starts_with('[') {
            dedans = t.to_lowercase() == entete;
            continue;
        }
        if !dedans {
            continue;
        }
        if let Some((c, val)) = t.split_once('=') {
            if c.trim().eq_ignore_ascii_case(cle) {
                v.push(val.trim().to_string());
            }
        }
    }
    v
}

/// Les clés d'une section d'un INI.
fn cles_ini(texte: &str, section: &str) -> Vec<(String, String)> {
    let entete = format!("[{section}]").to_lowercase();
    let mut dedans = false;
    let mut v = Vec::new();
    for l in texte.lines() {
        let t = l.trim();
        if t.starts_with('[') {
            dedans = t.to_lowercase() == entete;
        } else if dedans {
            if let Some((c, val)) = t.split_once('=') {
                v.push((c.trim().to_string(), val.trim().to_string()));
            }
        }
    }
    v
}

fn est_manette(liaison: &str) -> bool {
    // Une combinaison (`A & B`) est à la manette si l'une de ses parties l'est.
    liaison.split('&').any(|p| MANETTE.iter().any(|m| p.trim().starts_with(m)))
}

/// Le joueur 1 a-t-il déjà une manette dans DuckStation / PCSX2 ?
fn pad1_a_une_manette(texte: &str) -> bool {
    cles_ini(texte, "Pad1").iter().any(|(c, v)| !c.eq_ignore_ascii_case("Type") && est_manette(v))
}

/// Écrit la manette SDL du joueur 1 (DuckStation, PCSX2) en gardant les touches du clavier. `type_defaut` n'est
/// écrit que si le joueur 1 n'a pas encore de type de manette.
fn regler_pad_sdl(emulateur: &Path, ini: &Path, type_defaut: &str, forcer: bool) -> Resultat<bool> {
    let texte = std::fs::read_to_string(ini).unwrap_or_default();
    if !forcer && pad1_a_une_manette(&texte) {
        return Ok(false);
    }
    let mut valeurs: Vec<(String, Vec<String>)> = Vec::new();
    if lire_ini(&texte, "Pad1", "Type").is_empty() {
        valeurs.push(("Type".into(), vec![type_defaut.into()]));
    }
    for (touche, sdl) in PAD_PLAYSTATION {
        let mut v: Vec<String> = lire_ini(&texte, "Pad1", touche).into_iter().filter(|l| !est_manette(l)).collect();
        v.push(format!("SDL-0/{sdl}"));
        valeurs.push(((*touche).into(), v));
    }
    let refs: Vec<(&str, Vec<String>)> = valeurs.iter().map(|(c, v)| (c.as_str(), v.clone())).collect();
    modifier_ini(emulateur, ini, "Pad1", &refs)?;
    // Le menu de pause à la manette : Select + Start (les touches du clavier déjà réglées restent).
    let mut menu: Vec<String> = lire_ini(&texte, "Hotkeys", "OpenPauseMenu").into_iter().filter(|l| !est_manette(l)).collect();
    menu.push("SDL-0/Back & SDL-0/Start".into());
    modifier_ini(emulateur, ini, "Hotkeys", &[("OpenPauseMenu", menu)])?;
    Ok(true)
}

/// La manette GameCube du joueur 1 dans le dossier utilisateur Dolphin d'un profil.
fn regler_dolphin(emulateur: &Path, utilisateur: &Path, forcer: bool) -> Resultat<bool> {
    let ini = utilisateur.join("Config").join("GCPadNew.ini");
    let texte = std::fs::read_to_string(&ini).unwrap_or_default();
    let appareil = lire_ini(&texte, "GCPad1", "Device");
    let deja = appareil.first().is_some_and(|d| !d.is_empty() && !d.to_lowercase().contains("keyboard"));
    if !forcer && deja {
        return Ok(false);
    }
    let mut valeurs: Vec<(&str, Vec<String>)> = vec![("Device", vec!["XInput/0/Gamepad".into()])];
    valeurs.extend(PAD_GAMECUBE.iter().map(|(c, v)| (*c, vec![(*v).to_string()])));
    modifier_ini(emulateur, &ini, "GCPad1", &valeurs)?;
    Ok(true)
}

/// Les lignes à ajouter au fichier de réglages de RetroArch (`--appendconfig`) : la combinaison du menu, sauf si
/// la personne en a déjà choisi une dans retroarch.cfg.
pub fn lignes_retroarch(emulateur: &Path) -> String {
    let cfg = std::fs::read_to_string(emulateur.join("retroarch.cfg")).unwrap_or_default();
    let choisie = cfg.lines().find_map(|l| {
        let (c, v) = l.split_once('=')?;
        (c.trim() == "input_menu_toggle_gamepad_combo").then(|| v.trim().trim_matches('"').to_string())
    });
    let mut s = String::from("input_autodetect_enable = \"true\"\n");
    if choisie.as_deref().unwrap_or("0") == "0" {
        s.push_str(&format!("input_menu_toggle_gamepad_combo = \"{RETROARCH_COMBO_MENU}\"\n"));
    }
    s
}

/// RetroArch a-t-il ses profils de manette officiels ? (`autoconfig\xinput`, ou `joypad_autoconfig_dir` réglé.)
pub fn retroarch_a_ses_profils(emulateur: &Path) -> bool {
    let cfg = std::fs::read_to_string(emulateur.join("retroarch.cfg")).unwrap_or_default();
    // `:\autoconfig` : relatif au dossier du programme (notation de RetroArch).
    let dossier = cfg
        .lines()
        .find_map(|l| {
            let (c, v) = l.split_once('=')?;
            let v = v.trim().trim_matches('"').to_string();
            (c.trim() == "joypad_autoconfig_dir" && !v.is_empty() && v != "default").then_some(v)
        })
        .map(|v| match v.strip_prefix(":\\").or_else(|| v.strip_prefix(":/")) {
            Some(relatif) => emulateur.join(relatif),
            None => Path::new(&v).to_path_buf(),
        })
        .unwrap_or_else(|| emulateur.join("autoconfig"));
    dossier.join("xinput").is_dir()
}

/// L'adresse officielle des profils de manette de RetroArch (ceux de sa mise à jour en ligne).
pub const URL_PROFILS_RETROARCH: &str = "https://buildbot.libretro.com/assets/frontend/autoconfig.zip";

/// Règle la manette du joueur 1. `utilisateur` : le dossier utilisateur Dolphin du profil. Rend `true` si une
/// configuration a été écrite.
pub fn regler(id: &str, emulateur: &Path, utilisateur_dolphin: Option<&Path>, forcer: bool) -> Resultat<bool> {
    match id {
        "duckstation" => regler_pad_sdl(emulateur, &emulateur.join("settings.ini"), "AnalogController", forcer),
        "pcsx2" => regler_pad_sdl(emulateur, &emulateur.join("inis").join("PCSX2.ini"), "DualShock2", forcer),
        "dolphin" => match utilisateur_dolphin {
            Some(u) => regler_dolphin(emulateur, u, forcer),
            None => Ok(false),
        },
        "rpcs3" => regler_rpcs3(emulateur, forcer, premiere_manette_xinput()),
        _ => Ok(false),
    }
}

/// La première manette XInput branchée (0 à 3 ; la manette virtuelle de Sunshine en est une), par l'API de Windows.
pub fn premiere_manette_xinput() -> Option<u32> {
    #[cfg(windows)]
    {
        use windows::Win32::UI::Input::XboxController::{XInputGetState, XINPUT_STATE};
        for i in 0..4u32 {
            let mut s = XINPUT_STATE::default();
            // 0 (ERROR_SUCCESS) : une manette est branchée à cet emplacement.
            if unsafe { XInputGetState(i, &mut s) } == 0 {
                return Some(i);
            }
        }
    }
    None
}

/// La marque des réglages de manette écrits par Frogtend pour RPCS3 (1re ligne du fichier).
const MARQUE_RPCS3: &str = "# Frogtend : manette du joueur 1 réglée à chaque partie (manette Xbox branchée, sinon clavier).";

/// Le profil de manette RPCS3 voulu : la manette XInput n° `i` (0 à 3), sinon le clavier.
pub fn profil_rpcs3(manette: Option<u32>) -> String {
    let (handler, device) = match manette {
        Some(i) => ("XInput".to_string(), format!("XInput Pad #{}", i + 1)),
        None => ("Keyboard".to_string(), "Keyboard".to_string()),
    };
    format!("{MARQUE_RPCS3}\nPlayer 1 Input:\n  Handler: {handler}\n  Device: {device}\n")
}

/// Le joueur 1 de RPCS3 sur la manette branchée (sinon le clavier). Un fichier sans la marque de Frogtend est celui de
/// la personne : il n'est remplacé qu'à sa demande (`forcer`), après une copie à l'abri.
fn regler_rpcs3(emulateur: &Path, forcer: bool, manette: Option<u32>) -> Resultat<bool> {
    let fichier = emulateur.join("config").join("input_configs").join("global").join("Default.yml");
    let actuel = std::fs::read_to_string(&fichier).ok();
    let a_nous = actuel.as_deref().is_none_or(|t| t.starts_with(MARQUE_RPCS3));
    if !a_nous && !forcer {
        return Ok(false);
    }
    let voulu = profil_rpcs3(manette);
    if actuel.as_deref() == Some(voulu.as_str()) {
        return Ok(true);
    }
    if !a_nous {
        crate::emulateurs_profils::mettre_a_l_abri(emulateur, &fichier)?;
    }
    if let Some(p) = fichier.parent() {
        std::fs::create_dir_all(p)?;
    }
    std::fs::write(&fichier, voulu)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rpcs3_prend_la_manette_branchee_sinon_le_clavier_sans_defaire_un_reglage() {
        let d = tempfile::tempdir().unwrap();
        let e = d.path();
        let f = e.join("config").join("input_configs").join("global").join("Default.yml");
        // Une manette Xbox branchée (emplacement 0) : « XInput Pad #1 ».
        assert!(regler_rpcs3(e, false, Some(0)).unwrap());
        let t = std::fs::read_to_string(&f).unwrap();
        assert!(t.starts_with(MARQUE_RPCS3) && t.contains("Handler: XInput") && t.contains("Device: XInput Pad #1"));
        // Débranchée à la partie suivante : le clavier (sinon le joueur n'aurait aucune commande).
        assert!(regler_rpcs3(e, false, None).unwrap());
        assert!(std::fs::read_to_string(&f).unwrap().contains("Handler: Keyboard"));
        // Réglé par la personne dans RPCS3 (sans notre marque) : jamais touché…
        std::fs::write(&f, "Player 1 Input:\n  Handler: DualSense\n").unwrap();
        assert!(!regler_rpcs3(e, false, Some(1)).unwrap());
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "Player 1 Input:\n  Handler: DualSense\n");
        // … sauf à sa demande (« manette par défaut ») : copie à l'abri d'abord.
        assert!(regler_rpcs3(e, true, Some(1)).unwrap());
        assert!(std::fs::read_to_string(&f).unwrap().contains("Device: XInput Pad #2"));
        let abri = std::fs::read_dir(e.join(".frogtend-sauvegardes")).unwrap().next().unwrap().unwrap().path();
        assert!(std::fs::read_to_string(abri.join("config/input_configs/global/Default.yml")).unwrap().contains("DualSense"));
    }

    #[test]
    fn duckstation_recoit_la_manette_sdl_en_gardant_le_clavier() {
        let d = tempfile::tempdir().unwrap();
        let ini = d.path().join("settings.ini");
        std::fs::write(
            &ini,
            "[Main]\r\nSettingsVersion = 3\r\n\r\n[Pad1]\r\nType = DigitalController\r\nUp = Keyboard/Up\r\nCross = Keyboard/X\r\n\r\n[Hotkeys]\r\nOpenPauseMenu = Keyboard/Escape\r\n",
        )
        .unwrap();
        assert!(regler("duckstation", d.path(), None, false).unwrap());
        let t = std::fs::read_to_string(&ini).unwrap();
        assert_eq!(lire_ini(&t, "Pad1", "Type"), vec!["DigitalController"], "le type choisi est gardé");
        assert_eq!(lire_ini(&t, "Pad1", "Up"), vec!["Keyboard/Up", "SDL-0/DPadUp"]);
        assert_eq!(lire_ini(&t, "Pad1", "Cross"), vec!["Keyboard/X", "SDL-0/A"]);
        assert_eq!(lire_ini(&t, "Pad1", "L2"), vec!["SDL-0/+LeftTrigger"]);
        assert_eq!(lire_ini(&t, "Pad1", "LUp"), vec!["SDL-0/-LeftY"]);
        assert_eq!(lire_ini(&t, "Hotkeys", "OpenPauseMenu"), vec!["Keyboard/Escape", "SDL-0/Back & SDL-0/Start"]);
        assert!(t.contains("SettingsVersion = 3"));
        assert!(d.path().join(".frogtend-sauvegardes").is_dir(), "la configuration d'origine est à l'abri");
    }

    #[test]
    fn une_manette_deja_reglee_par_la_personne_n_est_pas_touchee_sauf_si_elle_le_demande() {
        let d = tempfile::tempdir().unwrap();
        let ini = d.path().join("inis").join("PCSX2.ini");
        std::fs::create_dir_all(ini.parent().unwrap()).unwrap();
        let perso = "[Pad1]\r\nType = DualShock2\r\nCross = SDL-0/B\r\n";
        std::fs::write(&ini, perso).unwrap();
        assert!(!regler("pcsx2", d.path(), None, false).unwrap());
        assert_eq!(std::fs::read_to_string(&ini).unwrap(), perso);

        assert!(regler("pcsx2", d.path(), None, true).unwrap());
        let t = std::fs::read_to_string(&ini).unwrap();
        assert_eq!(lire_ini(&t, "Pad1", "Cross"), vec!["SDL-0/A"], "remis par défaut, sans doublon");
        // Une deuxième fois ne change rien.
        regler("pcsx2", d.path(), None, true).unwrap();
        assert_eq!(std::fs::read_to_string(&ini).unwrap(), t);
    }

    #[test]
    fn pcsx2_neuf_recoit_le_type_et_toutes_les_touches() {
        let d = tempfile::tempdir().unwrap();
        assert!(regler("pcsx2", d.path(), None, false).unwrap());
        let t = std::fs::read_to_string(d.path().join("inis").join("PCSX2.ini")).unwrap();
        assert_eq!(lire_ini(&t, "Pad1", "Type"), vec!["DualShock2"]);
        for (touche, _) in PAD_PLAYSTATION {
            assert_eq!(lire_ini(&t, "Pad1", touche).len(), 1, "{touche}");
        }
    }

    #[test]
    fn dolphin_recoit_la_manette_xinput_dans_le_dossier_du_profil() {
        let d = tempfile::tempdir().unwrap();
        let u = d.path().join("Profils").join("Seb").join("User");
        let ini = u.join("Config").join("GCPadNew.ini");
        std::fs::create_dir_all(ini.parent().unwrap()).unwrap();
        std::fs::write(&ini, "[GCPad1]\r\nDevice = DInput/0/Keyboard Mouse\r\nButtons/A = X\r\n").unwrap();
        assert!(regler("dolphin", d.path(), Some(&u), false).unwrap());
        let t = std::fs::read_to_string(&ini).unwrap();
        assert_eq!(lire_ini(&t, "GCPad1", "Device"), vec!["XInput/0/Gamepad"]);
        assert_eq!(lire_ini(&t, "GCPad1", "Buttons/A"), vec!["`Button A`"]);
        assert_eq!(lire_ini(&t, "GCPad1", "Main Stick/Up"), vec!["`Left Y+`"]);
        // Réglée : on n'y touche plus.
        std::fs::write(&ini, "[GCPad1]\r\nDevice = XInput/1/Gamepad\r\n").unwrap();
        assert!(!regler("dolphin", d.path(), Some(&u), false).unwrap());
    }

    #[test]
    fn retroarch_combinaison_du_menu_sauf_choix_de_la_personne() {
        let d = tempfile::tempdir().unwrap();
        assert!(lignes_retroarch(d.path()).contains("input_menu_toggle_gamepad_combo = \"2\""));
        std::fs::write(d.path().join("retroarch.cfg"), "input_menu_toggle_gamepad_combo = \"4\"\n").unwrap();
        assert!(!lignes_retroarch(d.path()).contains("input_menu_toggle_gamepad_combo"));
        assert!(!retroarch_a_ses_profils(d.path()));
        std::fs::create_dir_all(d.path().join("autoconfig").join("xinput")).unwrap();
        assert!(retroarch_a_ses_profils(d.path()));
    }

    #[test]
    fn les_autres_emulateurs_n_ont_rien_a_regler() {
        let d = tempfile::tempdir().unwrap();
        for id in ["ppsspp", "dosbox-staging", "retroarch"] {
            assert!(!regler(id, d.path(), None, true).unwrap());
        }
        assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 0);
    }
}

#[cfg(test)]
mod essai_reel_manette {
    #[test]
    #[ignore]
    fn quelle_manette_xinput_est_branchee() {
        println!("MANETTE XINPUT : {:?}", super::premiere_manette_xinput());
    }
}
