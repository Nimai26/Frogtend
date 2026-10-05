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
//!   AUCUNE combinaison de menu à la manette : le seul menu est celui de Frogtend (voir `retirer_menu_manette`).
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
    Ok(true)
}

/// Les raccourcis qui ouvrent le menu propre de DuckStation / PCSX2 (DuckStation core/hotkeys.cpp : OpenPauseMenu,
/// TogglePauseMenu ; PCSX2 : OpenPauseMenu).
const MENUS_EMULATEUR: &[&str] = &["OpenPauseMenu", "TogglePauseMenu"];

/// Un seul menu en jeu, celui de Frogtend, ouvert partout par la même combinaison (Seb, 05/10 : « le but est
/// d'uniformiser : il faut pouvoir avoir le menu Frogtend PARTOUT avec View + RB »). Avant chaque partie, le menu de
/// DuckStation / PCSX2 perd TOUTES ses liaisons à la manette (Frogtend y mettait Select + Start jusqu'en 0.46.2 ; une
/// liaison faite à la main part aussi) ; celles du clavier restent. Copié à l'abri avant d'être modifié ; un fichier
/// illisible n'est jamais touché.
pub fn retirer_menu_manette(emulateur: &Path, ini: &Path) -> Resultat<bool> {
    let texte = match std::fs::read_to_string(ini) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.into()),
    };
    let mut valeurs: Vec<(&str, Vec<String>)> = Vec::new();
    for cle in MENUS_EMULATEUR {
        let avant = lire_ini(&texte, "Hotkeys", cle);
        let garde: Vec<String> = avant.iter().filter(|l| !est_manette(l)).cloned().collect();
        if garde.len() != avant.len() {
            valeurs.push((cle, garde));
        }
    }
    if valeurs.is_empty() {
        return Ok(false);
    }
    modifier_ini(emulateur, ini, "Hotkeys", &valeurs)?;
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

/// Les lignes à ajouter au fichier de réglages de RetroArch (`--appendconfig`) pour la manette.
pub fn lignes_retroarch() -> String {
    String::from("input_autodetect_enable = \"true\"\n")
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
        "rpcs3" => regler_rpcs3_pour_partie(emulateur, forcer, manette_branchee()),
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

/// La manette du joueur 1 de RPCS3. RPCS3 n'a qu'UNE entrée par joueur, mais chacune a un nom générique et se
/// reconnecte seule quand la manette est branchée ou réveillée après le lancement (relevé dans son code, 04/10 :
/// xinput_pad_handler « XInput Pad # », ds4_pad_handler « DS4 Pad # », dualsense_pad_handler « DualSense Pad # »,
/// énumération toutes les 2 s dans hid_pad_handler.cpp).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadRpcs3 {
    /// Manette Xbox ou compatible (et la manette virtuelle de Sunshine) : emplacement XInput 0 à 3.
    XInput(u32),
    /// DualShock 4 (PS4).
    Ds4,
    /// DualSense (PS5).
    DualSense,
    /// Le clavier : seulement quand la personne l'a choisi pour le jeu.
    Clavier,
}

impl PadRpcs3 {
    fn famille(self) -> &'static str {
        match self {
            PadRpcs3::XInput(_) => "xinput",
            PadRpcs3::Ds4 => "ds4",
            PadRpcs3::DualSense => "dualsense",
            PadRpcs3::Clavier => "clavier",
        }
    }
    fn depuis_famille(f: &str) -> Option<Self> {
        match f.trim() {
            "xinput" => Some(PadRpcs3::XInput(0)),
            "ds4" => Some(PadRpcs3::Ds4),
            "dualsense" => Some(PadRpcs3::DualSense),
            _ => None,
        }
    }
}

/// La famille d'une manette PlayStation d'après l'identifiant Windows d'un périphérique HID (USB « VID_054C&PID_09CC »
/// ou Bluetooth « VID&0002054C_PID&09CC »), avec les identifiants que RPCS3 reconnaît lui-même.
pub fn famille_hid(instance: &str) -> Option<PadRpcs3> {
    let i = instance.to_lowercase();
    let pid = |p: &str| i.contains(&format!("pid_{p}")) || i.contains(&format!("pid&{p}"));
    let sony = i.contains("054c");
    if sony && (pid("05c4") || pid("09cc") || pid("0ba0")) || i.contains("0c12") && pid("0e20") {
        return Some(PadRpcs3::Ds4);
    }
    if sony && (pid("0ce6") || pid("0df2")) {
        return Some(PadRpcs3::DualSense);
    }
    None
}

/// Les identifiants des périphériques HID présents (lecture seule, SetupAPI de Windows).
fn peripheriques_hid() -> Vec<String> {
    #[allow(unused_mut)]
    let mut l = Vec::new();
    #[cfg(windows)]
    unsafe {
        use windows::core::w;
        use windows::Win32::Devices::DeviceAndDriverInstallation::{
            SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsW, SetupDiGetDeviceInstanceIdW,
            DIGCF_ALLCLASSES, DIGCF_PRESENT, SP_DEVINFO_DATA,
        };
        let Ok(ensemble) = SetupDiGetClassDevsW(None, w!("HID"), None, DIGCF_PRESENT | DIGCF_ALLCLASSES) else { return l };
        for i in 0..4096u32 {
            let mut d = SP_DEVINFO_DATA { cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32, ..Default::default() };
            if SetupDiEnumDeviceInfo(ensemble, i, &mut d).is_err() {
                break;
            }
            let mut tampon = [0u16; 512];
            if SetupDiGetDeviceInstanceIdW(ensemble, &d, Some(&mut tampon), None).is_ok() {
                let n = tampon.iter().position(|c| *c == 0).unwrap_or(tampon.len());
                l.push(String::from_utf16_lossy(&tampon[..n]));
            }
        }
        let _ = SetupDiDestroyDeviceInfoList(ensemble);
    }
    l
}

/// La manette branchée à l'instant, s'il y en a une : Xbox (XInput) d'abord, sinon PlayStation (HID).
pub fn manette_branchee() -> Option<PadRpcs3> {
    if let Some(i) = premiere_manette_xinput() {
        return Some(PadRpcs3::XInput(i));
    }
    peripheriques_hid().iter().find_map(|p| famille_hid(p))
}

/// Ce que reçoit le joueur 1 : la manette branchée ; sinon la dernière vue sur ce PC (une manette sans fil en veille au
/// lancement : RPCS3 la prendra à son réveil) ; sinon une manette Xbox (la plus courante, et celle du streaming).
pub fn choisir_pad(branchee: Option<PadRpcs3>, derniere: Option<PadRpcs3>) -> PadRpcs3 {
    branchee.or(derniere).unwrap_or(PadRpcs3::XInput(0))
}

/// La marque des réglages de manette écrits par Frogtend pour RPCS3 (1re ligne du fichier). Tout fichier qui commence
/// par `PREFIXE_MARQUE` a été écrit par Frogtend (y compris par les versions 0.44.0 à 0.44.2).
const PREFIXE_MARQUE: &str = "# Frogtend : manette du joueur 1";
const MARQUE_RPCS3: &str = "# Frogtend : manette du joueur 1 réglée à chaque partie (manette branchée, sinon la dernière vue ; clavier sur demande).";

/// Le profil de manette RPCS3 voulu.
pub fn profil_rpcs3(pad: PadRpcs3) -> String {
    let (handler, device) = match pad {
        PadRpcs3::XInput(i) => ("XInput".to_string(), format!("XInput Pad #{}", i + 1)),
        PadRpcs3::Ds4 => ("DualShock 4".to_string(), "DS4 Pad #1".to_string()),
        PadRpcs3::DualSense => ("DualSense".to_string(), "DualSense Pad #1".to_string()),
        PadRpcs3::Clavier => ("Keyboard".to_string(), "Keyboard".to_string()),
    };
    // Valeurs ENTRE GUILLEMETS : en YAML, « #1 » après une espace est un commentaire (bug de la 0.44.3 : RPCS3 lisait
    // « XInput Pad », ne trouvait rien et mettait le joueur 1 sur « aucune entrée », clavier compris).
    format!("{MARQUE_RPCS3}\nPlayer 1 Input:\n  Handler: \"{handler}\"\n  Device: \"{device}\"\n")
}

/// Le joueur 1 de RPCS3 sur `pad`. Un fichier sans la marque de Frogtend est celui de la personne : il n'est remplacé
/// qu'à sa demande (`forcer`), après une copie à l'abri. Une manette (pas le clavier) est retenue comme « dernière vue ».
pub(crate) fn regler_rpcs3(emulateur: &Path, forcer: bool, pad: PadRpcs3) -> Resultat<bool> {
    let fichier = emulateur.join("config").join("input_configs").join("global").join("Default.yml");
    let actuel = std::fs::read_to_string(&fichier).ok();
    let a_nous = actuel.as_deref().is_none_or(|t| t.starts_with(PREFIXE_MARQUE));
    if !a_nous && !forcer {
        return Ok(false);
    }
    let voulu = profil_rpcs3(pad);
    if actuel.as_deref() != Some(voulu.as_str()) {
        if !a_nous {
            crate::emulateurs_profils::mettre_a_l_abri(emulateur, &fichier)?;
        }
        if let Some(p) = fichier.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::write(&fichier, voulu)?;
    }
    Ok(true)
}

/// Le fichier où Frogtend retient la dernière famille de manette vue pour cet émulateur.
fn fichier_derniere(emulateur: &Path) -> std::path::PathBuf {
    emulateur.join("Profils").join(".frogtend-manette")
}

/// La dernière manette vue sur ce PC pour cet émulateur.
pub fn derniere_manette(emulateur: &Path) -> Option<PadRpcs3> {
    std::fs::read_to_string(fichier_derniere(emulateur)).ok().and_then(|f| PadRpcs3::depuis_famille(&f))
}

/// Règle RPCS3 pour une partie : la manette branchée, sinon la dernière vue, sinon Xbox ; retient celle qui est branchée.
pub fn regler_rpcs3_pour_partie(emulateur: &Path, forcer: bool, branchee: Option<PadRpcs3>) -> Resultat<bool> {
    let regle = regler_rpcs3(emulateur, forcer, choisir_pad(branchee, derniere_manette(emulateur)))?;
    // Retenir la manette vue : au mieux (jamais bloquant), et seulement si le réglage est celui de Frogtend.
    if let (true, Some(b)) = (regle, branchee) {
        let f = fichier_derniere(emulateur);
        if let Some(p) = f.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        let _ = std::fs::write(&f, b.famille());
    }
    Ok(regle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rpcs3_prend_la_manette_branchee_sinon_la_derniere_vue_sans_defaire_un_reglage() {
        let d = tempfile::tempdir().unwrap();
        let e = d.path();
        let f = e.join("config").join("input_configs").join("global").join("Default.yml");
        let lire = || std::fs::read_to_string(&f).unwrap();
        // Ce que RPCS3 LIT (règles YAML : guillemets, et « # » après une espace = commentaire).
        let vu = |cle: &str| valeur_yaml(&lire(), cle);
        // Rien de branché, rien de connu : une manette Xbox (RPCS3 la prendra quand elle se réveille) — PAS le clavier.
        assert!(regler_rpcs3_pour_partie(e, false, None).unwrap());
        assert!(lire().starts_with(MARQUE_RPCS3));
        assert_eq!((vu("Handler"), vu("Device")), ("XInput".into(), "XInput Pad #1".into()));
        // Une DualSense branchée : elle, et elle est retenue.
        regler_rpcs3_pour_partie(e, false, Some(PadRpcs3::DualSense)).unwrap();
        assert_eq!((vu("Handler"), vu("Device")), ("DualSense".into(), "DualSense Pad #1".into()));
        // En veille à la partie suivante : la dernière vue (la DualSense), pas la Xbox.
        regler_rpcs3_pour_partie(e, false, None).unwrap();
        assert_eq!(vu("Device"), "DualSense Pad #1");
        // Une manette PS4, puis une Xbox sur l'emplacement 2.
        regler_rpcs3_pour_partie(e, false, Some(PadRpcs3::Ds4)).unwrap();
        assert_eq!((vu("Handler"), vu("Device")), ("DualShock 4".into(), "DS4 Pad #1".into()));
        regler_rpcs3_pour_partie(e, false, Some(PadRpcs3::XInput(1))).unwrap();
        assert_eq!(vu("Device"), "XInput Pad #2");
        // Le clavier, choisi pour le jeu.
        regler_rpcs3(e, false, PadRpcs3::Clavier).unwrap();
        assert_eq!((vu("Handler"), vu("Device")), ("Keyboard".into(), "Keyboard".into()));
        // Un fichier écrit par la 0.44.0–0.44.2 (ancienne marque) est bien reconnu comme celui de Frogtend.
        std::fs::write(&f, "# Frogtend : manette du joueur 1 réglée à chaque partie (manette Xbox branchée, sinon clavier).\nPlayer 1 Input:\n  Handler: Keyboard\n").unwrap();
        regler_rpcs3_pour_partie(e, false, Some(PadRpcs3::XInput(0))).unwrap();
        assert_eq!(vu("Handler"), "XInput");
        // Réglé par la personne dans RPCS3 (sans notre marque) : jamais touché…
        std::fs::write(&f, "Player 1 Input:\n  Handler: DualSense\n").unwrap();
        assert!(!regler_rpcs3_pour_partie(e, false, Some(PadRpcs3::XInput(1))).unwrap());
        assert_eq!(lire(), "Player 1 Input:\n  Handler: DualSense\n");
        // … sauf à sa demande (« manette par défaut ») : copie à l'abri d'abord.
        assert!(regler_rpcs3(e, true, PadRpcs3::XInput(1)).unwrap());
        assert_eq!(vu("Device"), "XInput Pad #2");
        let abri = std::fs::read_dir(e.join(".frogtend-sauvegardes")).unwrap().next().unwrap().unwrap().path();
        assert!(std::fs::read_to_string(abri.join("config/input_configs/global/Default.yml")).unwrap().contains("DualSense"));
    }

    /// Une valeur d'un fichier YAML simple, lue comme le fait RPCS3 (yaml-cpp) : entre guillemets, ou jusqu'à un
    /// « # » précédé d'une espace (commentaire).
    fn valeur_yaml(texte: &str, cle: &str) -> String {
        let l = texte.lines().find(|l| l.trim_start().starts_with(&format!("{cle}:"))).unwrap_or("");
        let v = l.trim_start()[cle.len() + 1..].trim();
        if let Some(r) = v.strip_prefix('"') {
            return r.split('"').next().unwrap_or("").to_string();
        }
        v.split(" #").next().unwrap_or("").trim().to_string()
    }

    #[test]
    fn la_lecture_yaml_du_test_suit_la_regle_du_diese() {
        // Le bug de la 0.44.3 : sans guillemets, « #1 » disparaît.
        assert_eq!(valeur_yaml("  Device: XInput Pad #1\n", "Device"), "XInput Pad");
        assert_eq!(valeur_yaml("  Device: \"XInput Pad #1\"\n", "Device"), "XInput Pad #1");
    }

    #[test]
    fn les_manettes_playstation_sont_reconnues_par_leur_identifiant() {
        // USB et Bluetooth (forme des identifiants de périphérique de Windows).
        assert_eq!(famille_hid(r"HID\VID_054C&PID_09CC&MI_03\8&1A2B3C&0&0000"), Some(PadRpcs3::Ds4));
        assert_eq!(famille_hid(r"HID\{00001124-0000-1000-8000-00805F9B34FB}_VID&0002054C_PID&05C4\9&ABC&0&0000"), Some(PadRpcs3::Ds4));
        assert_eq!(famille_hid(r"HID\VID_054C&PID_0CE6&MI_03\8&2B&0&0000"), Some(PadRpcs3::DualSense));
        assert_eq!(famille_hid(r"HID\VID_054C&PID_0DF2\8&2B&0&0000"), Some(PadRpcs3::DualSense), "DualSense Edge");
        // Une autre manette Sony (pas lue par RPCS3 en natif) ou un clavier : rien.
        assert_eq!(famille_hid(r"HID\VID_054C&PID_0268\1"), None);
        assert_eq!(famille_hid(r"HID\VID_046D&PID_C52B&MI_00\7&1"), None);
        assert_eq!(choisir_pad(None, None), PadRpcs3::XInput(0));
    }

    #[test]
    fn le_menu_des_emulateurs_n_a_plus_aucune_liaison_a_la_manette() {
        let d = tempfile::tempdir().unwrap();
        let ini = d.path().join("settings.ini");
        // Ce que Frogtend écrivait jusqu'en 0.46.2, plus une liaison faite à la main : les deux partent, le clavier reste.
        std::fs::write(&ini, "[Hotkeys]\r\nOpenPauseMenu = Keyboard/Escape\r\nOpenPauseMenu = SDL-0/Back & SDL-0/Start\r\nOpenPauseMenu = SDL-1/Guide\r\nTogglePauseMenu = SDL-0/Guide\r\nFastForward = Keyboard/Tab\r\n").unwrap();
        assert!(retirer_menu_manette(d.path(), &ini).unwrap());
        let t = std::fs::read_to_string(&ini).unwrap();
        assert_eq!(lire_ini(&t, "Hotkeys", "OpenPauseMenu"), vec!["Keyboard/Escape"]);
        assert!(lire_ini(&t, "Hotkeys", "TogglePauseMenu").is_empty());
        assert_eq!(lire_ini(&t, "Hotkeys", "FastForward"), vec!["Keyboard/Tab"]);
        assert!(!retirer_menu_manette(d.path(), &ini).unwrap(), "la 2e fois : rien à faire");
        assert!(d.path().join(".frogtend-sauvegardes").is_dir(), "l'original est à l'abri");
        // Absent : rien n'est créé ; illisible : jamais touché.
        assert!(!retirer_menu_manette(d.path(), &d.path().join("absent.ini")).unwrap());
        assert!(!d.path().join("absent.ini").exists());
        let illisible = [b'[', b'H', b']', b'\n', 0xFF, 0xFE, b'\n'];
        std::fs::write(&ini, illisible).unwrap();
        assert!(retirer_menu_manette(d.path(), &ini).is_err());
        assert_eq!(std::fs::read(&ini).unwrap(), illisible);
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
        assert_eq!(lire_ini(&t, "Hotkeys", "OpenPauseMenu"), vec!["Keyboard/Escape"], "aucun menu de l'émulateur à la manette");
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
    fn retroarch_reconnait_ses_profils_de_manette() {
        let d = tempfile::tempdir().unwrap();
        assert!(!lignes_retroarch().contains("input_menu_toggle_gamepad_combo"));
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
        println!("MANETTE BRANCHEE : {:?}", super::manette_branchee());
        let hid = super::peripheriques_hid();
        println!("HID : {} périphériques ; Sony : {:?}", hid.len(), hid.iter().filter(|h| h.to_lowercase().contains("054c")).collect::<Vec<_>>());
    }
}
