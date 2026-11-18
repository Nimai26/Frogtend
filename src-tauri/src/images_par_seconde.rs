//! Afficher ou cacher les images par seconde (le compteur DE L'ÉMULATEUR), depuis le menu en jeu de Frogtend.
//! Décision de Seb (08/11) : un choix PAR PERSONNE, pour tous ses jeux. Il est posé au lancement, avant que l'émulateur
//! ne démarre ; certains émulateurs savent aussi basculer pendant la partie (RetroArch, MAME, la famille d'Eden).
//!
//! Relevés (09/11) :
//! - RetroArch : `fps_show` (retroarch.cfg) ; posé dans le `frogtend.cfg` du profil (`--appendconfig`). RetroArch
//!   recopie la valeur de la partie dans retroarch.cfg en quittant (`config_save_on_exit`) : elle est rendue après.
//! - Eden, Eglantine, Lichii : `[UI] UIGameList\show_perf_overlay` (qt_common/config/uisettings.h, défaut false) ;
//!   lu au démarrage (main_window.cpp : `action_Show_Performance_Overlay` cochée, puis `OnLoadComplete` montre le
//!   compteur), bascule par le raccourci « Toggle Performance Overlay » (Ctrl+V d'origine), réenregistré en quittant.
//! - RPCS3 : `config\config.yml`, `Video:` ▸ `Performance Overlay:` ▸ `Enabled:` ; aucun raccourci : au prochain
//!   lancement seulement.
//!
//! Relevés (18/11, demande de Seb : « TOUS les émulateurs, quand c'est possible »), dans le code de leur branche
//! principale ; tous lus au démarrage, aucun avec une touche D'ORIGINE pour basculer (donc au prochain lancement) :
//! - DuckStation : `settings.ini` `[Display] ShowFPS` (src/core/settings.cpp L436, défaut false ; montre FPS et VPS,
//!   imgui_overlays.cpp L407-415). Réécrit en quittant seulement si un réglage a changé, depuis sa mémoire (qthost.cpp
//!   L2193-2198, core.cpp L302-309) : la valeur posée reste. `ToggleOSD` existe sans touche d'origine (hotkeys.cpp
//!   L38-52, L382-386).
//! - PCSX2 : `inis\PCSX2.ini` `[EmuCore/GS] OsdShowFPS` (Pcsx2Config.cpp L974-976) ; rien ne s'affiche si
//!   `OsdPerformancePos = 0` (« None », ImGuiOverlays.cpp L1811 ; défaut 3, en haut à droite, Pcsx2Config.cpp L728) :
//!   pour l'afficher, une position 0 devient 3. Réécrit en quittant seulement si un réglage a changé (QtHost.cpp
//!   L2584-2585). `ToggleOSD` sans touche d'origine (GS.cpp L1294-1298, Pad.cpp). Le fichier n'est jamais créé ici :
//!   sans `[UI] SettingsVersion`, PCSX2 remettrait tout par défaut (VMManager.cpp L552-557).
//! - Dolphin : `GFX.ini` `[Settings] ShowFPS` (Core/Config/GraphicsSettings.cpp L45) ; le choix part sur la ligne de
//!   commande, `-C Graphics.Settings.ShowFPS=True` (UICommon/CommandLineParse.cpp L97-101 ; le système s'appelle
//!   « Graphics », Common/Config/Config.cpp L162) : la couche « ligne de commande » n'est jamais enregistrée (L70-73),
//!   aucun fichier n'est écrit, et il vaut dès la première partie. Aucun raccourci (Core/HotkeyManager.cpp).
//! - PPSSPP : `memstick\PSP\SYSTEM\ppsspp.ini` (celui du profil, par le lien `memstick`) `[Graphics] iShowStatusFlags`,
//!   bit 2 = FPS (Core/ConfigValues.h L249-253, Config.cpp L924), les autres bits gardés ; réécrit en quittant depuis sa
//!   mémoire (NativeApp.cpp L2089). Aucun raccourci (Core/KeyMap.h L35-83).
//! - Cemu : `portable\settings.xml` `<Graphic><Overlay>` : `<Position>` 0 = tout l'affichage coupé (son défaut,
//!   CemuConfig.h L462 ; LatteOverlay.cpp L549-553), `<FPS>` (défaut true). Pour l'afficher, une position 0 devient 3
//!   (en haut à droite, loin des notifications en haut à gauche). Réécrit en quittant (MainWindow.cpp L453). Aucun
//!   raccourci (HotkeySettings.cpp L157-210).
//! - Vita3K : `portable\config.yml` `performance-overlay: true` (config/config.h L186) ; réécrit au démarrage depuis ce
//!   qu'il a lu (config.cpp L522-523), jamais en quittant. Aucun raccourci (config.h L64-131). Global seulement (le
//!   réglage par jeu ne l'a pas, settings_dialog.cpp L187).
//!
//! Impossibles (pas d'entrée dans le menu) :
//! - Ryubing : les FPS ne sont que dans sa barre d'état (AppHost.cs L1313-1331), cachée en plein écran
//!   (MainWindowViewModel.cs L2192-2196) ; F4 la remontre avec la barre de menus en rétrécissant l'image, sans rien
//!   enregistrer (AppHost.cs L1478-1479) ; aucune clé dans Config.json (miroir Leuconoe/Ryubing, code du 30/07/2026).
//! - Azahar : seulement sa barre d'état (citra_qt.cpp L631-644, L3937), cachée en plein écran (L2882-2890).
//! - xemu : seulement la fenêtre « Video Debug » du menu Debug (ui/xui/debug.cc L379-405), sans clé ni raccourci pour
//!   l'ouvrir (config_spec.yml, menubar.cc L223).
//! - Xenia Canary : aucun compteur ; son profileur (F3, `show_profiler`) est retiré des versions publiées
//!   (CMakeLists.txt L45-47 : `XE_OPTION_PROFILING=0`).
//! - mGBA : les FPS ne sont que dans le titre de la fenêtre (src/platform/qt/Window.cpp L1182-1195, L1225) ;
//!   `showFrameCounter` compte les images depuis le début, pas par seconde (MessagePainter.cpp L82).
//! - DOSBox Staging : aucun compteur (ni réglage, ni titre : src/gui/titlebar.cpp L33, L732-770).
//! - BigPEmu (code fermé) : rien dans son manuel ni sa FAQ officiels.

use std::path::{Path, PathBuf};

use crate::erreurs::Resultat;

/// Frogtend sait-il poser le choix au lancement pour cet émulateur ?
pub fn au_lancement(id: &str) -> bool {
    matches!(id, "retroarch" | "rpcs3" | "eden" | "eglantine" | "lichii") || PAR_FICHIER.contains(&id)
}

/// Les émulateurs réglés par un fichier relu ici (`fichier`), ou par la ligne de commande (Dolphin).
const PAR_FICHIER: [&str; 6] = ["duckstation", "pcsx2", "dolphin", "ppsspp", "cemu", "vita3k"];

/// Le fichier de réglages que l'émulateur lit VRAIMENT (`profil` : le dossier du profil dans l'émulateur) ; `None` :
/// ses réglages sont ailleurs (pas portable : le dossier de Windows, jamais lu ni touché).
fn fichier(id: &str, emulateur: &Path, profil: &Path) -> Option<PathBuf> {
    match id {
        // Portable avec `portable.txt`, ou dès qu'un settings.ini est à côté du programme (core.cpp L144-147).
        "duckstation" => (emulateur.join("portable.txt").is_file() || emulateur.join("settings.ini").is_file()).then(|| emulateur.join("settings.ini")),
        // Portable avec `portable.ini` ou `portable.txt` (Pcsx2Config.cpp L2207-2209).
        "pcsx2" => (emulateur.join("portable.ini").is_file() || emulateur.join("portable.txt").is_file()).then(|| emulateur.join("inis").join("PCSX2.ini")),
        // Lancé avec `-u <profil>\User` (emulateurs_profils.rs).
        "dolphin" => Some(profil.join("User").join("Config").join("GFX.ini")),
        // `installed.txt` : son memstick est ailleurs (Windows/main.cpp L1000-1025).
        "ppsspp" => (!emulateur.join("installed.txt").exists()).then(|| profil.join("memstick").join("PSP").join("SYSTEM").join("ppsspp.ini")),
        "cemu" => crate::cemu::portable(emulateur).then(|| emulateur.join("portable").join("settings.xml")),
        "vita3k" => crate::vita3k::portable(emulateur).then(|| emulateur.join("portable").join("config.yml")),
        _ => None,
    }
}

/// Ce que l'émulateur affiche aujourd'hui, d'après ses réglages (avant le lancement) ; `None` : inconnu. `profil` :
/// le dossier du profil dans l'émulateur.
pub fn lire(id: &str, emulateur: &Path, profil: &Path) -> Option<bool> {
    if PAR_FICHIER.contains(&id) {
        let f = fichier(id, emulateur, profil)?;
        // Absent (jamais lancé, ou jamais enregistré) : la valeur d'origine de l'émulateur, qui le cache.
        if !f.exists() {
            return Some(false);
        }
        let t = lire_plafonne(&f)?;
        return Some(lire_texte(id, &t));
    }
    match id {
        "retroarch" => crate::emulateurs::reglage_retroarch(emulateur, "fps_show").or(Some(false)),
        // Pas portable : ses vrais réglages sont dans le dossier de Windows, que Frogtend ne lit pas → inconnu.
        "eden" | "eglantine" | "lichii" if !crate::eden::portable(emulateur) => None,
        "eden" | "eglantine" | "lichii" => {
            let t = crate::eden::lire_reglages(emulateur).ok()?;
            Some(crate::manettes::lire_ini(&t, "UI", "UIGameList\\show_perf_overlay").first().is_some_and(|v| v.trim() == "true"))
        }
        "rpcs3" => lire_plafonne(&emulateur.join("config").join("config.yml")).and_then(|t| rpcs3_lire(&t)),
        _ => None,
    }
}

/// Pose le choix de la personne avant le lancement. `profil` : le dossier du profil dans l'émulateur (RetroArch : son
/// `frogtend.cfg`, déjà écrit pour cette partie). Rend vrai si quelque chose a été posé.
pub fn poser(id: &str, emulateur: &Path, profil: &Path, afficher: bool) -> Resultat<bool> {
    let v = if afficher { "true" } else { "false" };
    match id {
        "retroarch" => {
            // Ajouté à la fin (jamais le fichier réécrit : une écriture coupée ne le tronque pas).
            use std::io::Write;
            let cfg = profil.join("frogtend.cfg");
            if !cfg.is_file() {
                return Ok(false);
            }
            std::fs::OpenOptions::new().append(true).open(&cfg)?.write_all(format!("fps_show = \"{v}\"\n").as_bytes())?;
            Ok(true)
        }
        "eden" | "eglantine" | "lichii" => crate::eden::regler_images_par_seconde(emulateur, afficher),
        "rpcs3" => {
            // Ouvert : il réécrirait ses réglages en quittant ; rien n'est touché.
            if !crate::lancement::processus_du_dossier(emulateur).is_empty() {
                return Ok(false);
            }
            let f = emulateur.join("config").join("config.yml");
            // Absent (jamais lancé) : rien de créé ; RPCS3 l'écrit à sa première partie, le choix vaut à la suivante.
            let Some(t) = lire_plafonne(&f) else { return Ok(false) };
            let Some(apres) = rpcs3_regler(&t, afficher) else { return Ok(false) };
            // Copie à l'abri, et jamais un fichier illisible remplacé.
            crate::emulateurs_profils::remplacer_config(emulateur, &f, &apres)?;
            Ok(true)
        }
        // Dolphin : rien d'écrit, le choix part sur la ligne de commande (`arguments`).
        "dolphin" => Ok(false),
        _ if PAR_FICHIER.contains(&id) => {
            // Ouvert : il réécrirait ses réglages en quittant ; rien n'est touché.
            if !crate::lancement::processus_du_dossier(emulateur).is_empty() {
                return Ok(false);
            }
            let Some(f) = fichier(id, emulateur, profil) else { return Ok(false) };
            // Absent : rien de créé (l'émulateur l'écrit lui-même, le choix vaudra à la partie suivante) ; illisible ou
            // trop gros : jamais remplacé.
            if !f.is_file() {
                return Ok(false);
            }
            let Some(t) = lire_plafonne(&f) else { return Ok(false) };
            // PCSX2, Cemu : une position « aucune » que FROGTEND a changée (sa marque) est remise en cachant.
            let marque = marque_position(emulateur);
            let remettre = !afficher && matches!(id, "pcsx2" | "cemu") && marque.is_file();
            let Some(apres) = regler_texte(id, &t, afficher, remettre) else {
                // Rien à faire : la position n'est plus celle que Frogtend avait posée (la personne l'a reprise).
                if remettre {
                    std::fs::remove_file(&marque)?;
                }
                return Ok(false);
            };
            // Copie à l'abri d'abord ; seule la ligne du compteur change (et la position, voir `regler_texte`).
            crate::emulateurs_profils::remplacer_config(emulateur, &f, &apres)?;
            if matches!(id, "pcsx2" | "cemu") {
                if afficher && position_coupee(id, &t) && !position_coupee(id, &apres) {
                    std::fs::create_dir_all(emulateur.join("Profils"))?;
                    std::fs::write(&marque, "")?;
                }
                if remettre {
                    std::fs::remove_file(&marque)?;
                }
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// Ce qu'ajoute la ligne de commande (Dolphin seulement) : jamais enregistré par l'émulateur.
pub fn arguments(id: &str, afficher: bool) -> Vec<String> {
    match id {
        "dolphin" => vec!["-C".into(), format!("Graphics.Settings.ShowFPS={}", if afficher { "True" } else { "False" })],
        _ => vec![],
    }
}

/// Le compteur s'affiche-t-il, d'après le texte du fichier de réglages de l'émulateur ?
pub fn lire_texte(id: &str, t: &str) -> bool {
    let premiere = |section: &str, cle: &str| crate::manettes::lire_ini(t, section, cle).into_iter().next();
    let vrai = |v: Option<String>| v.is_some_and(|v| v.trim().eq_ignore_ascii_case("true"));
    match id {
        "duckstation" => vrai(premiere("Display", "ShowFPS")),
        "pcsx2" => vrai(premiere("EmuCore/GS", "OsdShowFPS")) && premiere("EmuCore/GS", "OsdPerformancePos").is_none_or(|p| p.trim() != "0"),
        "dolphin" => vrai(premiere("Settings", "ShowFPS")),
        "ppsspp" => drapeaux_ppsspp(t) & FPS_PPSSPP != 0,
        "cemu" => cemu_lire(t),
        "vita3k" => t.lines().find_map(|l| l.strip_prefix("performance-overlay:")).is_some_and(|v| v.split(" #").next().unwrap_or("").trim() == "true"),
        _ => false,
    }
}

/// PCSX2 et Cemu : Frogtend a passé la position de leur affichage des performances de « aucune » à 3 (une marque dans
/// le dossier `Profils\` que Frogtend tient dans l'émulateur) : remise à « aucune » quand le compteur est caché.
fn marque_position(emulateur: &Path) -> PathBuf {
    emulateur.join("Profils").join(".compteur-position-frogtend")
}

/// Les AUTRES lignes de l'affichage des performances, que la position « aucune » cache aussi. PCSX2 : celles de
/// `DrawPerformanceOverlay` (ImGuiOverlays.cpp L320-576 ; toutes fausses d'origine, Pcsx2Config.cpp L729-742).
const AUTRES_PCSX2: [&str; 11] = [
    "OsdShowSpeed",
    "OsdShowVPS",
    "OsdShowVersion",
    "OsdShowGSStats",
    "OsdShowResolution",
    "OsdShowHardwareInfo",
    "OsdShowCPU",
    "OsdShowGPU",
    "OsdShowGPUDebug",
    "OsdShowGPUStats",
    "OsdShowFrameTimes",
];
/// Cemu : celles de `<Overlay>` (CemuConfig.cpp L151-163, LatteOverlay.cpp L81-110 ; fausses d'origine).
const AUTRES_CEMU: [&str; 6] = ["DrawCalls", "CPUUsage", "CPUPerCoreUsage", "RAMUsage", "VRAMUsage", "Debug"];

/// La position de l'affichage des performances, telle qu'écrite (PCSX2 : `OsdPerformancePos` ; Cemu : `<Position>`
/// de `<Overlay>`, ou l'ancienne `<OverlayPosition>`).
fn position(id: &str, t: &str) -> Option<String> {
    match id {
        "pcsx2" => crate::manettes::lire_ini(t, "EmuCore/GS", "OsdPerformancePos").into_iter().next().map(|p| p.trim().to_string()),
        "cemu" => {
            let (bloc, nom) = match balise(t, "Overlay") {
                Some((a, b)) => (&t[a..b], "Position"),
                None => (t, "OverlayPosition"),
            };
            balise(bloc, nom).map(|(a, b)| bloc[a..b].trim().to_string())
        }
        _ => None,
    }
}

/// La position coupe-t-elle tout l'affichage ? (PCSX2 : 0 écrit, son défaut est 3 ; Cemu : 0, ou absente, son défaut.)
fn position_coupee(id: &str, t: &str) -> bool {
    match id {
        "pcsx2" => position(id, t).as_deref() == Some("0"),
        "cemu" => position(id, t).and_then(|p| p.parse::<u32>().ok()).unwrap_or(0) == 0,
        _ => false,
    }
}

/// Une autre ligne de l'affichage des performances est-elle cochée ?
fn autres_lignes(id: &str, t: &str) -> bool {
    let vrai = |v: &str| v.trim().eq_ignore_ascii_case("true") || v.trim() == "1";
    match id {
        "pcsx2" => AUTRES_PCSX2.iter().any(|c| crate::manettes::lire_ini(t, "EmuCore/GS", c).first().is_some_and(|v| vrai(v))),
        "cemu" => {
            let (bloc, prefixe) = match balise(t, "Overlay") {
                Some((a, b)) => (&t[a..b], ""),
                None => (t, "Overlay"),
            };
            AUTRES_CEMU.iter().any(|c| balise(bloc, &format!("{prefixe}{c}")).is_some_and(|(a, b)| vrai(&bloc[a..b])))
        }
        _ => false,
    }
}

/// PCSX2, Cemu : la position « aucune » ET d'autres lignes cochées. Passer à 3 les ferait réapparaître : pas d'entrée
/// dans le menu pour cette personne.
pub fn bloque(id: &str, t: &str) -> bool {
    matches!(id, "pcsx2" | "cemu") && position_coupee(id, t) && autres_lignes(id, t)
}

/// Le texte du fichier avec le choix posé (seulement la ligne du compteur, et la position quand elle coupe tout) ;
/// `None` : déjà ainsi, ou une forme inattendue (jamais touchée). `remettre` (PCSX2, Cemu) : en cachant, la position que
/// Frogtend avait passée de 0 à 3 revient à 0 (si elle vaut encore 3 et qu'aucune autre ligne n'est cochée).
pub fn regler_texte(id: &str, t: &str, afficher: bool, remettre: bool) -> Option<String> {
    let v = if afficher { "true" } else { "false" };
    if id == "cemu" {
        return cemu_regler(t, afficher, remettre);
    }
    if id == "pcsx2" {
        let s = "EmuCore/GS";
        if afficher && bloque(id, t) {
            return None;
        }
        let mut x = t.to_string();
        let fps = crate::manettes::lire_ini(t, s, "OsdShowFPS").first().is_some_and(|f| f.trim().eq_ignore_ascii_case("true"));
        if fps != afficher {
            x = regler_ini(&x, s, "OsdShowFPS", v);
        }
        if afficher && position_coupee(id, t) {
            x = regler_ini(&x, s, "OsdPerformancePos", "3");
        }
        if !afficher && remettre && position(id, t).as_deref() == Some("3") && !autres_lignes(id, t) {
            x = regler_ini(&x, s, "OsdPerformancePos", "0");
        }
        return (x != t && lire_texte(id, &x) == afficher).then_some(x);
    }
    if lire_texte(id, t) == afficher {
        return None;
    }
    let apres = match id {
        "duckstation" => regler_ini(t, "Display", "ShowFPS", v),
        "dolphin" => regler_ini(t, "Settings", "ShowFPS", if afficher { "True" } else { "False" }),
        "ppsspp" => {
            let d = drapeaux_ppsspp(t);
            regler_ini(t, "Graphics", "iShowStatusFlags", &(if afficher { d | FPS_PPSSPP } else { d & !FPS_PPSSPP }).to_string())
        }
        "vita3k" => crate::vita3k::remplacer_ligne(t, "performance-overlay", &format!("performance-overlay: {v}")),
        _ => return None,
    };
    (apres != t && lire_texte(id, &apres) == afficher).then_some(apres)
}

/// PPSSPP : le bit du compteur d'images dans `iShowStatusFlags` (`FPS_COUNTER = 1 << 1`).
const FPS_PPSSPP: u32 = 2;

fn drapeaux_ppsspp(t: &str) -> u32 {
    crate::manettes::lire_ini(t, "Graphics", "iShowStatusFlags").first().and_then(|v| v.trim().parse().ok()).unwrap_or(0)
}

/// Une clé d'une section d'un INI réglée en gardant TOUT le reste à l'octet près (fins de ligne, espaces autour du
/// « = », ordre) : remplacée sur place, sinon ajoutée à la fin de sa section, sinon la section ajoutée à la fin.
pub fn regler_ini(texte: &str, section: &str, cle: &str, valeur: &str) -> String {
    let fin = if texte.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lignes: Vec<String> = texte.split_inclusive('\n').map(String::from).collect();
    let entete = format!("[{section}]").to_lowercase();
    let mut dedans = false;
    // La dernière ligne non vide de la section (son en-tête au moins).
    let mut derniere: Option<usize> = None;
    for i in 0..lignes.len() {
        let l = lignes[i].trim();
        if l.starts_with('[') {
            if dedans {
                break;
            }
            dedans = l.to_lowercase() == entete;
            if dedans {
                derniere = Some(i);
            }
            continue;
        }
        if !dedans || l.is_empty() {
            continue;
        }
        derniere = Some(i);
        if l.split_once('=').is_some_and(|(c, _)| c.trim().eq_ignore_ascii_case(cle)) {
            let brute = &lignes[i];
            let eol = if brute.ends_with("\r\n") { "\r\n" } else if brute.ends_with('\n') { "\n" } else { "" };
            let egal = brute.find('=').unwrap_or(0) + 1;
            let espaces = brute[egal..].len() - brute[egal..].trim_start_matches(' ').len();
            lignes[i] = format!("{}{valeur}{eol}", &brute[..egal + espaces]);
            return lignes.concat();
        }
    }
    let ligne = format!("{cle} = {valeur}{fin}");
    match derniere {
        Some(i) => {
            if !lignes[i].ends_with('\n') {
                lignes[i].push_str(fin);
            }
            lignes.insert(i + 1, ligne);
            lignes.concat()
        }
        None => {
            let mut t = texte.to_string();
            if !t.is_empty() && !t.ends_with('\n') {
                t.push_str(fin);
            }
            if !t.is_empty() {
                t.push_str(fin);
            }
            format!("{t}[{section}]{fin}{ligne}")
        }
    }
}

/// Le texte entre `<nom>` et `</nom>` (la première fois), avec sa place dans `xml`.
fn balise(xml: &str, nom: &str) -> Option<(usize, usize)> {
    let ouvre = format!("<{nom}>");
    let debut = xml.find(&ouvre)? + ouvre.len();
    let fin = debut + xml[debut..].find(&format!("</{nom}>"))?;
    Some((debut, fin))
}

/// Cemu : `<Overlay>` affiche-t-il le compteur ? (Position 1 à 6 ET FPS vrai ; l'ancienne forme `<OverlayPosition>` /
/// `<OverlayFPS>` sous `<Graphic>`, lue par Cemu quand `<Overlay>` manque, CemuConfig.cpp L172-174.)
pub fn cemu_lire(xml: &str) -> bool {
    let (bloc, position, fps) = match balise(xml, "Overlay") {
        Some((a, b)) => (&xml[a..b], "Position", "FPS"),
        None => (xml, "OverlayPosition", "OverlayFPS"),
    };
    let valeur = |n: &str| balise(bloc, n).map(|(a, b)| bloc[a..b].trim().to_string());
    let position = valeur(position).and_then(|p| p.parse::<u32>().ok()).unwrap_or(0);
    let fps = valeur(fps).is_none_or(|f| f.eq_ignore_ascii_case("true") || f == "1");
    position != 0 && fps
}

/// Cemu : `settings.xml` avec le compteur affiché ou caché (`<FPS>`, et `<Position>` 0 → 3 pour l'afficher, seulement si
/// aucune autre ligne n'est cochée) ; `<Overlay>` ajouté s'il manque (Cemu prend la valeur d'origine de chaque clé
/// absente), avec les fins de ligne du fichier. `remettre` : en cachant, la position que Frogtend avait passée à 3
/// revient à 0 (seulement si elle vaut encore 3 et qu'aucune autre ligne n'est cochée ; `<FPS>` alors gardé, comme
/// avant). `None` : rien à faire, ou une forme inattendue (l'ancienne forme, un `<Graphic/>` vide…) jamais touchée.
pub fn cemu_regler(xml: &str, afficher: bool, remettre: bool) -> Option<String> {
    if afficher && bloque("cemu", xml) {
        return None;
    }
    let Some((a, b)) = balise(xml, "Overlay") else {
        if cemu_lire(xml) == afficher || !afficher || xml.contains("<OverlayPosition>") || xml.contains("<OverlayFPS>") {
            return None;
        }
        let n = if xml.contains("\r\n") { "\r\n" } else { "\n" };
        let overlay = format!("<Overlay>{n}<Position>3</Position>{n}<FPS>true</FPS>{n}</Overlay>{n}");
        if let Some((_, g)) = balise(xml, "Graphic") {
            return Some(format!("{}{overlay}{}", &xml[..g], &xml[g..]));
        }
        if xml.contains("<Graphic") {
            return None;
        }
        let c = xml.rfind("</content>")?;
        return Some(format!("{}<Graphic>{n}{overlay}</Graphic>{n}{}", &xml[..c], &xml[c..]));
    };
    let mut bloc = xml[a..b].to_string();
    let poser = |bloc: &mut String, nom: &str, valeur: &str| match balise(bloc, nom) {
        Some((x, y)) => bloc.replace_range(x..y, valeur),
        None => bloc.insert_str(0, &format!("<{nom}>{valeur}</{nom}>")),
    };
    if afficher {
        if position_coupee("cemu", xml) {
            poser(&mut bloc, "Position", "3");
        }
        poser(&mut bloc, "FPS", "true");
    } else if remettre && position("cemu", xml).as_deref() == Some("3") && !autres_lignes("cemu", xml) {
        poser(&mut bloc, "Position", "0");
    } else if cemu_lire(xml) {
        poser(&mut bloc, "FPS", "false");
    }
    let r = format!("{}{bloc}{}", &xml[..a], &xml[b..]);
    (r != xml && cemu_lire(&r) == afficher).then_some(r)
}

/// La famille d'Eden bascule-t-elle le compteur pendant la partie ? Seulement si le raccourci « Toggle Performance
/// Overlay » est encore celui d'origine (Ctrl+V) : un raccourci changé par la personne n'est jamais deviné.
pub fn eden_en_direct(emulateur: &Path) -> bool {
    // Pas portable : son raccourci est dans le dossier de Windows, jamais deviné.
    if !crate::eden::portable(emulateur) {
        return false;
    }
    let Ok(t) = crate::eden::lire_reglages(emulateur) else { return false };
    crate::manettes::lire_ini(&t, "UI", "Shortcuts\\Main%20Window\\Toggle%20Performance%20Overlay\\KeySeq")
        .first()
        .is_none_or(|v| v.trim().eq_ignore_ascii_case("Ctrl+V"))
}

/// Ce que le lancement a fait du compteur, gardé dans la partie pour le menu.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct EtatLancement {
    /// Ce que l'émulateur affichera (`None` : inconnu).
    pub affiche: Option<bool>,
    pub au_lancement: bool,
    pub en_direct: bool,
    /// RetroArch : `fps_show` d'avant la partie, et si Frogtend en a posé un (rendu après la partie).
    pub avant: Option<bool>,
    pub pose: bool,
    pub erreur: Option<String>,
    /// Ajoutés DEVANT la ligne de commande de l'émulateur (Dolphin : `-C Graphics.Settings.ShowFPS=…`).
    pub arguments: Vec<String>,
    /// Pourquoi l'entrée n'est pas proposée (noté au journal).
    pub raison: Option<String>,
}

/// Dolphin : la valeur du DERNIER `-C Graphics.Settings.ShowFPS=…` (ou `--config`, `--config=`, `-C…` collé) de la
/// ligne de lancement ; `None` : aucun.
pub fn dolphin_dans_la_ligne(ligne: &str) -> Option<bool> {
    let mots = mots(ligne);
    let mut dernier = None;
    let mut i = 0;
    while i < mots.len() {
        let m = mots[i].as_str();
        let valeur = if m == "-C" || m == "--config" {
            i += 1;
            mots.get(i).cloned()
        } else if let Some(v) = m.strip_prefix("--config=") {
            Some(v.to_string())
        } else {
            m.strip_prefix("-C").filter(|v| !v.is_empty()).map(String::from)
        };
        if let Some((cle, v)) = valeur.as_deref().and_then(|v| v.split_once('=')) {
            if cle.trim().eq_ignore_ascii_case("Graphics.Settings.ShowFPS") {
                dernier = Some(v.trim().eq_ignore_ascii_case("true") || v.trim() == "1");
            }
        }
        i += 1;
    }
    dernier
}

/// Les mots d'une ligne de commande (espaces hors guillemets ; guillemets retirés).
fn mots(ligne: &str) -> Vec<String> {
    let mut l = Vec::new();
    let mut m = String::new();
    let mut guillemets = false;
    let mut present = false;
    for c in ligne.chars() {
        match c {
            '"' => {
                guillemets = !guillemets;
                present = true;
            }
            c if c.is_whitespace() && !guillemets => {
                if present {
                    l.push(std::mem::take(&mut m));
                    present = false;
                }
            }
            c => {
                m.push(c);
                present = true;
            }
        }
    }
    if present {
        l.push(m);
    }
    l
}

/// Avant le lancement : lit l'état de l'émulateur, pose le choix de la personne (`None` : jamais choisi dans Frogtend,
/// le réglage de l'émulateur est gardé).
pub fn au_lancement_de_la_partie(id: &str, emulateur: &Path, profil: &str, choix: Option<bool>, jeu_ps3: Option<&str>, ligne: &str) -> EtatLancement {
    // Dolphin : un `-C …ShowFPS` déjà dans SA ligne de lancement passe après le nôtre, donc l'emporte (les `-C` sont
    // posés dans l'ordre, CommandLineParse.cpp L46-67, et `Layer::Set` remplace, Layer.h L145-151) : il décide, rien
    // n'est ajouté, pas d'entrée « à la prochaine partie ».
    if let Some(v) = dolphin_dans_la_ligne(ligne).filter(|_| id == "dolphin") {
        return EtatLancement {
            affiche: Some(v),
            raison: Some("Dolphin : sa ligne de lancement règle déjà le compteur (-C …ShowFPS) ; c'est elle qui décide, le choix de Frogtend n'est pas ajouté.".into()),
            ..Default::default()
        };
    }
    // RPCS3 : un réglage PROPRE à ce jeu (custom_configs, appliqué par-dessus le général) décide ; jamais écrit
    // par-dessus. Le menu montre sa valeur, sans entrée « à la prochaine partie ».
    let par_jeu = jeu_ps3
        .filter(|_| id == "rpcs3")
        .and_then(|n| crate::emulateurs_profils::rpcs3_config_du_jeu(emulateur, n).ok())
        .and_then(|f| lire_plafonne(&f))
        .and_then(|t| rpcs3_lire(&t));
    if par_jeu.is_some() {
        return EtatLancement { affiche: par_jeu, ..Default::default() };
    }
    let p = crate::emulateurs_profils::dossier_du_profil(emulateur, profil);
    let avant = lire(id, emulateur, &p);
    // Des réglages hors de son dossier (pas portable) : jamais devinés, jamais touchés.
    let ailleurs = (matches!(id, "eden" | "eglantine" | "lichii") && !crate::eden::portable(emulateur))
        || (PAR_FICHIER.contains(&id) && fichier(id, emulateur, &p).is_none());
    // PCSX2, Cemu : l'affichage des performances coupé avec d'autres lignes cochées : jamais réveillé (pas d'entrée).
    let bloque_ici = matches!(id, "pcsx2" | "cemu")
        && fichier(id, emulateur, &p).filter(|f| f.is_file()).and_then(|f| lire_plafonne(&f)).is_some_and(|t| bloque(id, &t));
    let mut e = EtatLancement {
        affiche: avant,
        au_lancement: au_lancement(id) && !ailleurs && !bloque_ici,
        raison: bloque_ici.then(|| {
            format!(
                "{} : son affichage des performances est réglé sur « aucun » et d'autres lignes y sont cochées ; le compteur d'images n'est pas proposé, pour ne pas les faire réapparaître.",
                if id == "pcsx2" { "PCSX2" } else { "Cemu" }
            )
        }),
        // Seuls RetroArch, MAME et la famille d'Eden (raccourci d'origine) basculent pendant la partie.
        en_direct: match id {
            "eden" | "eglantine" | "lichii" => eden_en_direct(emulateur),
            _ => !PAR_FICHIER.contains(&id),
        },
        avant: if id == "retroarch" { crate::emulateurs::reglage_retroarch(emulateur, "fps_show") } else { None },
        ..Default::default()
    };
    // Dolphin : par sa ligne de commande, jamais enregistré ; il l'affichera dès cette partie.
    if let Some(v) = choix.filter(|_| e.au_lancement && id == "dolphin") {
        e.arguments = arguments(id, v);
        e.affiche = Some(v);
        return e;
    }
    if let Some(v) = choix.filter(|_| e.au_lancement) {
        match poser(id, emulateur, &p, v) {
            Ok(fait) => {
                // Posé, ou déjà ainsi : la valeur est connue. RPCS3 sans fichier : il ne l'affiche pas encore.
                if fait || avant == Some(v) {
                    e.affiche = Some(v);
                }
                e.pose = fait && id == "retroarch";
            }
            Err(err) => e.erreur = Some(format!("{err:?}")),
        }
    }
    e
}

/// Un fichier de réglages lu avec un plafond (plusieurs Mo : ce n'est pas lui, jamais lu ni touché).
fn lire_plafonne(f: &Path) -> Option<String> {
    if std::fs::metadata(f).ok()?.len() > 8 * 1024 * 1024 {
        return None;
    }
    std::fs::read_to_string(f).ok()
}

/// RPCS3 : la valeur de `Video:` ▸ `Performance Overlay:` ▸ `Enabled:`.
pub fn rpcs3_lire(texte: &str) -> Option<bool> {
    rpcs3_ligne(texte).map(|(_, v)| v)
}

/// RPCS3 : le texte avec `Enabled:` réglé (fins de ligne et tout le reste gardés) ; `None` si rien ne change ou si
/// le bloc est introuvable (jamais créé : un autre `Enabled:` ne doit pas être touché).
pub fn rpcs3_regler(texte: &str, afficher: bool) -> Option<String> {
    let (i, v) = rpcs3_ligne(texte)?;
    if v == afficher {
        return None;
    }
    let mut lignes: Vec<&str> = texte.split_inclusive('\n').collect();
    let fin = if lignes[i].ends_with("\r\n") { "\r\n" } else if lignes[i].ends_with('\n') { "\n" } else { "" };
    let neuve = format!("    Enabled: {}{fin}", if afficher { "true" } else { "false" });
    lignes[i] = &neuve;
    Some(lignes.concat())
}

/// L'index de la ligne `    Enabled:` du bloc `  Performance Overlay:` de la section `Video:`, et sa valeur.
fn rpcs3_ligne(texte: &str) -> Option<(usize, bool)> {
    let mut video = false;
    let mut bloc = false;
    for (i, brute) in texte.split_inclusive('\n').enumerate() {
        let l = brute.trim_end_matches(['\r', '\n']);
        if l.trim().is_empty() {
            continue;
        }
        let retrait = l.len() - l.trim_start_matches(' ').len();
        if retrait == 0 {
            video = l == "Video:";
            bloc = false;
            continue;
        }
        if !video {
            continue;
        }
        if retrait == 2 {
            bloc = l.trim() == "Performance Overlay:";
            continue;
        }
        if bloc && retrait == 4 {
            if let Some(v) = l.trim().strip_prefix("Enabled:") {
                return match v.trim() {
                    "true" => Some((i, true)),
                    "false" => Some((i, false)),
                    _ => None,
                };
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const RPCS3: &str = "Core:\r\n  Enabled: true\r\nVideo:\r\n  Renderer: Vulkan\r\n  Performance Overlay:\r\n    Body Color (hex): \"#FFE138FF\"\r\n    Enabled: false\r\n    Font: n023055ms.ttf\r\n  Shader Mode: Async\r\nAudio:\r\n  Enabled: true\r\n";

    #[test]
    fn rpcs3_seule_la_ligne_du_compteur_change() {
        assert_eq!(rpcs3_lire(RPCS3), Some(false));
        let apres = rpcs3_regler(RPCS3, true).unwrap();
        assert_eq!(apres, RPCS3.replace("    Enabled: false\r\n", "    Enabled: true\r\n"), "CRLF gardé, les autres « Enabled » jamais touchés");
        assert_eq!(rpcs3_lire(&apres), Some(true));
        assert!(rpcs3_regler(&apres, true).is_none(), "déjà réglé : rien à écrire");
        assert!(rpcs3_regler("Video:\n  Renderer: Vulkan\nCore:\n  Enabled: false\n", true).is_none(), "bloc absent : jamais un autre Enabled");
        assert!(rpcs3_regler("", true).is_none());
    }

    #[test]
    fn rpcs3_absent_rien_de_cree() {
        let d = tempfile::tempdir().unwrap();
        assert!(!poser("rpcs3", d.path(), d.path(), true).unwrap());
        assert!(!d.path().join("config").exists());
    }

    #[test]
    fn retroarch_le_choix_va_dans_le_fichier_du_profil() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("Profils").join("Seb");
        assert!(!poser("retroarch", d.path(), &p, true).unwrap(), "sans frogtend.cfg : rien");
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(p.join("frogtend.cfg"), "savefile_directory = \"x\"\n").unwrap();
        assert!(poser("retroarch", d.path(), &p, true).unwrap());
        assert_eq!(std::fs::read_to_string(p.join("frogtend.cfg")).unwrap(), "savefile_directory = \"x\"\nfps_show = \"true\"\n");
        assert!(!d.path().join("retroarch.cfg").exists(), "le fichier commun n'est jamais touché au lancement");
        assert_eq!(lire("retroarch", d.path(), &p), Some(false), "absent : RetroArch ne l'affiche pas");
        std::fs::write(d.path().join("retroarch.cfg"), "fps_show = \"true\"\n").unwrap();
        assert_eq!(lire("retroarch", d.path(), &p), Some(true));
    }

    #[test]
    fn le_lancement_pose_le_choix_et_garde_l_etat() {
        let d = tempfile::tempdir().unwrap();
        let p = crate::emulateurs_profils::dossier_du_profil(d.path(), "Seb");
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(p.join("frogtend.cfg"), "").unwrap();
        std::fs::write(d.path().join("retroarch.cfg"), "fps_show = \"false\"\n").unwrap();
        let e = au_lancement_de_la_partie("retroarch", d.path(), "Seb", Some(true), None, "");
        assert_eq!(e, EtatLancement { affiche: Some(true), au_lancement: true, en_direct: true, avant: Some(false), pose: true, erreur: None, arguments: vec![], raison: None });
        // Jamais choisi : rien d'écrit, l'état de l'émulateur est montré.
        std::fs::write(p.join("frogtend.cfg"), "").unwrap();
        let e = au_lancement_de_la_partie("retroarch", d.path(), "Seb", None, None, "");
        assert_eq!((e.affiche, e.pose), (Some(false), false));
        assert_eq!(std::fs::read_to_string(p.join("frogtend.cfg")).unwrap(), "");
        // Un émulateur que Frogtend ne règle pas : rien.
        let e = au_lancement_de_la_partie("cemu", d.path(), "Seb", Some(true), None, "");
        assert_eq!((e.affiche, e.au_lancement, e.pose), (None, false, false));
    }

    #[test]
    fn eden_hors_mode_portable_rien_n_est_devine() {
        let d = tempfile::tempdir().unwrap();
        let e = au_lancement_de_la_partie("eglantine", d.path(), "Seb", Some(false), None, "");
        assert_eq!((e.affiche, e.au_lancement, e.en_direct), (None, false, false), "réglages dans le dossier de Windows : inconnus");
        assert!(!d.path().join("user").exists(), "rien de créé");
    }

    #[test]
    fn rpcs3_un_reglage_du_jeu_passe_devant() {
        let d = tempfile::tempdir().unwrap();
        let c = d.path().join("config");
        std::fs::create_dir_all(c.join("custom_configs")).unwrap();
        std::fs::write(c.join("config.yml"), RPCS3).unwrap();
        std::fs::write(c.join("custom_configs").join("config_BLES01227.yml"), "Video:\n  Performance Overlay:\n    Enabled: true\n").unwrap();
        let e = au_lancement_de_la_partie("rpcs3", d.path(), "Seb", Some(false), Some("BLES01227"), "");
        assert_eq!((e.affiche, e.au_lancement), (Some(true), false), "le réglage du jeu, jamais écrasé");
        assert_eq!(std::fs::read_to_string(c.join("config.yml")).unwrap(), RPCS3, "le général non touché");
        let e = au_lancement_de_la_partie("rpcs3", d.path(), "Seb", Some(true), Some("BLUS00001"), "");
        assert_eq!((e.affiche, e.au_lancement), (Some(true), true));
        assert_eq!(rpcs3_lire(&std::fs::read_to_string(c.join("config.yml")).unwrap()), Some(true));
    }

    #[test]
    fn retroarch_fps_show_rendu_apres_la_partie() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("retroarch.cfg"), "a = \"1\"\r\nfps_show = \"true\"\r\nvideo_shader_enable = \"true\"\r\n").unwrap();
        crate::emulateurs::rendre_reglage_retroarch(d.path(), "fps_show", false).unwrap();
        assert_eq!(std::fs::read_to_string(d.path().join("retroarch.cfg")).unwrap(), "a = \"1\"\r\nfps_show = \"false\"\r\nvideo_shader_enable = \"true\"\r\n");
    }

    #[test]
    fn eden_installation_neuve_et_raccourci() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("user")).unwrap();
        assert_eq!(lire("lichii", d.path(), d.path()), Some(false));
        assert!(eden_en_direct(d.path()), "pas de fichier : Ctrl+V d'origine");
        assert!(poser("lichii", d.path(), d.path(), true).unwrap(), "installation neuve : fichier créé");
        let t = std::fs::read_to_string(crate::eden::fichier_reglages(d.path())).unwrap();
        assert!(t.contains("UIGameList\\show_perf_overlay=true") && t.contains("UIGameList\\show_perf_overlay\\default=false"), "{t}");
        assert_eq!(lire("lichii", d.path(), d.path()), Some(true));
        assert!(!poser("lichii", d.path(), d.path(), true).unwrap(), "déjà réglé : rien");
        let autre = "[UI]\nShortcuts\\Main%20Window\\Toggle%20Performance%20Overlay\\KeySeq=Ctrl+Shift+P\n";
        std::fs::write(crate::eden::fichier_reglages(d.path()), autre).unwrap();
        assert!(!eden_en_direct(d.path()), "raccourci changé par la personne : pas de bascule en direct");
        assert!(!au_lancement("xemu") && au_lancement("cemu") && au_lancement("lichii") && au_lancement("rpcs3"));
    }

    // Des extraits à la forme exacte des fichiers écrits par chaque émulateur (relevée dans leur code d'écriture).
    const DUCKSTATION: &str = "[Main]\r\nSettingsVersion = 3\r\n\r\n[Display]\r\nShowOSDMessages = true\r\nShowFPS = false\r\nShowSpeed = false\r\n\r\n[Hotkeys]\r\nToggleOSD = \r\n";
    const PCSX2: &str = "[UI]\r\nSettingsVersion = 1\r\n\r\n[EmuCore/GS]\r\nOsdShowSpeed = false\r\nOsdShowFPS = false\r\nOsdPerformancePos = 3\r\n\r\n[Folders]\r\nBios = bios\r\n";
    const GFX: &str = "[Settings]\nShowFPS = False\nShowFTimes = False\n[Enhancements]\nMaxAnisotropy = 0\n";
    const PPSSPP: &str = "[General]\r\nLanguage = fr_FR\r\n[Graphics]\r\nFrameSkip = 0\r\niShowStatusFlags = 8\r\nRenderingMode = 1\r\n[Sound]\r\nEnable = True\r\n";
    const CEMU: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<content>\n    <Graphic>\n        <api>1</api>\n        <GraphicPack>\n            <Entry filename=\"x\"/>\n        </GraphicPack>\n        <Overlay>\n            <Position>0</Position>\n            <TextColor>4294967295</TextColor>\n            <FPS>true</FPS>\n            <DrawCalls>false</DrawCalls>\n        </Overlay>\n        <Notification>\n            <Position>1</Position>\n        </Notification>\n    </Graphic>\n</content>\n";
    const VITA3K: &str = "---\nshow-welcome: false\nperformance-overlay: false\nperformance-overlay-detail: 0\nuser-id: \"00\"\n...\n";

    #[test]
    fn regler_ini_garde_tout_le_reste() {
        // Sur place : fins de ligne, espaces et ordre gardés.
        assert_eq!(regler_ini(DUCKSTATION, "Display", "ShowFPS", "true"), DUCKSTATION.replace("ShowFPS = false", "ShowFPS = true"));
        assert_eq!(regler_ini("[A]\nb=1\n", "a", "B", "2"), "[A]\nb=2\n", "sans espaces, section et clé sans casse");
        // Une clé absente : ajoutée à la fin de SA section, pas dans la suivante.
        assert_eq!(regler_ini("[A]\r\nx = 1\r\n\r\n[B]\r\ny = 2\r\n", "A", "z", "3"), "[A]\r\nx = 1\r\nz = 3\r\n\r\n[B]\r\ny = 2\r\n");
        assert_eq!(regler_ini("[A]\nx = 1", "A", "z", "3"), "[A]\nx = 1\nz = 3\n", "dernière ligne sans fin");
        // Une section absente : ajoutée à la fin.
        assert_eq!(regler_ini("[A]\r\nx = 1\r\n", "Display", "ShowFPS", "true"), "[A]\r\nx = 1\r\n\r\n[Display]\r\nShowFPS = true\r\n");
        assert_eq!(regler_ini("", "Display", "ShowFPS", "true"), "[Display]\nShowFPS = true\n");
        // Le même nom dans une autre section n'est jamais touché.
        assert_eq!(regler_ini("[Autre]\nShowFPS = false\n[Display]\n", "Display", "ShowFPS", "true"), "[Autre]\nShowFPS = false\n[Display]\nShowFPS = true\n");
    }

    #[test]
    fn la_lecture_suit_chaque_emulateur() {
        assert!(!lire_texte("duckstation", DUCKSTATION));
        assert!(lire_texte("duckstation", &DUCKSTATION.replace("ShowFPS = false", "ShowFPS = true")));
        assert!(!lire_texte("pcsx2", PCSX2));
        let pcsx2 = PCSX2.replace("OsdShowFPS = false", "OsdShowFPS = true");
        assert!(lire_texte("pcsx2", &pcsx2));
        assert!(!lire_texte("pcsx2", &pcsx2.replace("OsdPerformancePos = 3", "OsdPerformancePos = 0")), "position « None » : rien ne s'affiche");
        assert!(lire_texte("pcsx2", "[EmuCore/GS]\nOsdShowFPS = true\n"), "position absente : son défaut (en haut à droite)");
        assert!(!lire_texte("dolphin", GFX));
        assert!(lire_texte("dolphin", &GFX.replace("ShowFPS = False", "ShowFPS = True")), "Dolphin écrit True avec une majuscule");
        assert!(!lire_texte("ppsspp", PPSSPP), "8 = la batterie seule");
        assert!(lire_texte("ppsspp", &PPSSPP.replace("= 8", "= 10")));
        assert!(!lire_texte("ppsspp", "[Graphics]\niShowStatusFlags = 1\n"), "1 : bit qui ne sert plus");
        assert!(!lire_texte("cemu", CEMU), "position 0 : l'affichage entier est coupé");
        assert!(lire_texte("cemu", &CEMU.replace("<Position>0</Position>", "<Position>4</Position>")));
        assert!(!lire_texte("vita3k", VITA3K));
        assert!(lire_texte("vita3k", &VITA3K.replace("performance-overlay: false", "performance-overlay: true")));
        assert!(!lire_texte("vita3k", "performance-overlay-detail: 1\n"), "une autre clé qui commence pareil");
    }

    #[test]
    fn seule_la_ligne_du_compteur_change() {
        assert_eq!(regler_texte("duckstation", DUCKSTATION, true, false).unwrap(), DUCKSTATION.replace("ShowFPS = false", "ShowFPS = true"));
        assert!(regler_texte("duckstation", DUCKSTATION, false, false).is_none(), "déjà ainsi : rien");
        // Le fichier créé par Frogtend à la première partie (sans [Display]) : la section ajoutée.
        let neuf = crate::emulateurs_profils::base_neuve(Path::new("C:/DuckStation"), Path::new("C:/DuckStation/settings.ini"));
        let apres = regler_texte("duckstation", &neuf, true, false).unwrap();
        assert!(apres.starts_with(&neuf) && apres.ends_with("\r\n[Display]\r\nShowFPS = true\r\n"), "{apres}");
        assert_eq!(regler_texte("pcsx2", PCSX2, true, false).unwrap(), PCSX2.replace("OsdShowFPS = false", "OsdShowFPS = true"));
        let cache = PCSX2.replace("OsdShowFPS = false", "OsdShowFPS = true").replace("OsdPerformancePos = 3", "OsdPerformancePos = 0");
        assert_eq!(regler_texte("pcsx2", &cache, true, false).unwrap(), PCSX2.replace("OsdShowFPS = false", "OsdShowFPS = true"), "position 0 → 3");
        assert_eq!(regler_texte("pcsx2", &PCSX2.replace("OsdShowFPS = false", "OsdShowFPS = true"), false, false).unwrap(), PCSX2, "cacher : la position gardée");
        assert_eq!(regler_texte("dolphin", GFX, true, false).unwrap(), GFX.replace("ShowFPS = False", "ShowFPS = True"));
        assert_eq!(regler_texte("ppsspp", PPSSPP, true, false).unwrap(), PPSSPP.replace("= 8", "= 10"), "la batterie gardée");
        assert_eq!(regler_texte("ppsspp", &PPSSPP.replace("= 8", "= 14"), false, false).unwrap(), PPSSPP.replace("= 8", "= 12"), "la vitesse gardée");
        assert_eq!(regler_texte("vita3k", VITA3K, true, false).unwrap(), VITA3K.replace("performance-overlay: false", "performance-overlay: true"));
        assert!(regler_texte("xemu", "", true, false).is_none());
    }

    #[test]
    fn cemu_position_et_fps_dans_overlay_seulement() {
        let apres = cemu_regler(CEMU, true, false).unwrap();
        assert_eq!(apres, CEMU.replace("<Position>0</Position>\n            <TextColor>", "<Position>3</Position>\n            <TextColor>"), "seule la position de l'affichage change");
        assert!(apres.contains("<Notification>\n            <Position>1</Position>"), "les notifications jamais touchées");
        assert!(cemu_lire(&apres));
        let cache = cemu_regler(&apres, false, false).unwrap();
        assert_eq!(cache, apres.replace("<FPS>true</FPS>", "<FPS>false</FPS>"), "cacher : seulement FPS");
        assert!(!cemu_lire(&cache));
        assert_eq!(cemu_regler(&cache, true, false).unwrap(), apres, "réaffiché : la position gardée");
        assert!(cemu_regler(CEMU, false, false).is_none(), "déjà caché");
        // Le settings.xml que Frogtend crée pour la langue (sans <Graphic>) : le bloc ajouté, lu comme Cemu le lirait.
        let langue = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<content>\n    <console_language>2</console_language>\n</content>\n";
        let x = cemu_regler(langue, true, false).unwrap();
        assert!(x.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<content>\n    <console_language>2</console_language>\n<Graphic>") && x.ends_with("</Graphic>\n</content>\n"), "{x}");
        assert!(cemu_lire(&x));
        // <Graphic> sans <Overlay> : ajouté dedans (jamais dans <GraphicPack>).
        let g = "<content><Graphic><GraphicPack></GraphicPack><api>1</api></Graphic></content>";
        let y = cemu_regler(g, true, false).unwrap();
        assert!(y.starts_with("<content><Graphic><GraphicPack></GraphicPack><api>1</api><Overlay>") && y.ends_with("</Overlay>\n</Graphic></content>"), "{y}");
        // Formes inattendues : jamais touchées.
        assert!(cemu_regler("<content><Graphic><OverlayPosition>0</OverlayPosition></Graphic></content>", true, false).is_none(), "ancienne forme");
        assert!(cemu_regler("<content><Graphic/></content>", true, false).is_none());
        assert!(cemu_regler("<autre/>", true, false).is_none());
        // L'ancienne forme se lit quand même.
        assert!(cemu_lire("<content><Graphic><OverlayPosition>2</OverlayPosition><OverlayFPS>true</OverlayFPS></Graphic></content>"));
    }

    #[test]
    fn poser_par_fichier_copie_a_l_abri_et_ne_cree_rien() {
        let d = tempfile::tempdir().unwrap();
        let e = d.path();
        let profil = e.join("Profils").join("Seb");
        // DuckStation portable, settings.ini encore absent : rien de créé.
        std::fs::write(e.join("portable.txt"), "").unwrap();
        assert_eq!(lire("duckstation", e, &profil), Some(false));
        assert!(!poser("duckstation", e, &profil, true).unwrap());
        assert!(!e.join("settings.ini").exists(), "jamais créé");
        // Présent : seule la ligne change, l'ancien fichier à l'abri.
        std::fs::write(e.join("settings.ini"), DUCKSTATION).unwrap();
        assert!(poser("duckstation", e, &profil, true).unwrap());
        assert_eq!(std::fs::read_to_string(e.join("settings.ini")).unwrap(), DUCKSTATION.replace("ShowFPS = false", "ShowFPS = true"));
        assert_eq!(lire("duckstation", e, &profil), Some(true));
        let abri = std::fs::read_dir(e.join(".frogtend-sauvegardes")).unwrap().next().unwrap().unwrap().path();
        assert_eq!(std::fs::read_to_string(abri.join("settings.ini")).unwrap(), DUCKSTATION);
        assert!(!poser("duckstation", e, &profil, true).unwrap(), "déjà ainsi : rien");
        // Un « fichier » illisible (ici un dossier) : jamais remplacé.
        let f = tempfile::tempdir().unwrap();
        std::fs::write(f.path().join("portable.ini"), "").unwrap();
        std::fs::create_dir_all(f.path().join("inis").join("PCSX2.ini")).unwrap();
        assert!(!poser("pcsx2", f.path(), &profil, true).unwrap());
        assert!(f.path().join("inis").join("PCSX2.ini").is_dir());
        // PPSSPP : le fichier du PROFIL (son memstick).
        let pp = profil.join("memstick").join("PSP").join("SYSTEM");
        std::fs::create_dir_all(&pp).unwrap();
        std::fs::write(pp.join("ppsspp.ini"), PPSSPP).unwrap();
        assert!(poser("ppsspp", e, &profil, true).unwrap());
        assert_eq!(std::fs::read_to_string(pp.join("ppsspp.ini")).unwrap(), PPSSPP.replace("= 8", "= 10"));
        // Vita3K et Cemu portables.
        std::fs::create_dir_all(e.join("portable")).unwrap();
        std::fs::write(e.join("portable").join("config.yml"), VITA3K).unwrap();
        assert!(poser("vita3k", e, &profil, true).unwrap());
        assert_eq!(lire("vita3k", e, &profil), Some(true));
        std::fs::write(e.join("portable").join("settings.xml"), CEMU).unwrap();
        assert!(poser("cemu", e, &profil, true).unwrap());
        assert_eq!(lire("cemu", e, &profil), Some(true));
        // Dolphin : jamais un fichier écrit (la ligne de commande).
        assert!(!poser("dolphin", e, &profil, true).unwrap());
        assert!(!profil.join("User").exists());
    }

    #[test]
    fn hors_mode_portable_rien_n_est_devine() {
        let d = tempfile::tempdir().unwrap();
        for id in ["duckstation", "pcsx2", "cemu", "vita3k"] {
            let e = au_lancement_de_la_partie(id, d.path(), "Seb", Some(true), None, "");
            assert_eq!((e.affiche, e.au_lancement, e.en_direct), (None, false, false), "{id}");
        }
        std::fs::write(d.path().join("installed.txt"), "").unwrap();
        assert!(!au_lancement_de_la_partie("ppsspp", d.path(), "Seb", Some(true), None, "").au_lancement);
        assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 1, "rien de créé");
        assert!(arguments("pcsx2", true).is_empty());
    }

    #[test]
    fn premiere_partie_et_dolphin_par_la_ligne_de_commande() {
        let d = tempfile::tempdir().unwrap();
        // Dolphin : GFX.ini du profil absent, l'argument suffit (dès la première partie), rien d'écrit.
        let e = au_lancement_de_la_partie("dolphin", d.path(), "Seb", Some(true), None, "");
        assert_eq!(e.arguments, ["-C", "Graphics.Settings.ShowFPS=True"]);
        assert_eq!((e.affiche, e.au_lancement, e.en_direct, e.pose), (Some(true), true, false, false));
        assert_eq!(arguments("dolphin", false), ["-C", "Graphics.Settings.ShowFPS=False"], "le système s'appelle « Graphics », jamais « GFX »");
        assert!(au_lancement_de_la_partie("dolphin", d.path(), "Seb", None, None, "").arguments.is_empty(), "jamais choisi : rien");
        assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 0, "rien de créé");
        // GFX.ini du profil réglé dans Dolphin : relu quand rien n'est choisi.
        let c = crate::emulateurs_profils::dossier_du_profil(d.path(), "Seb").join("User").join("Config");
        std::fs::create_dir_all(&c).unwrap();
        std::fs::write(c.join("GFX.ini"), GFX.replace("False\nShowFTimes", "True\nShowFTimes")).unwrap();
        assert_eq!(au_lancement_de_la_partie("dolphin", d.path(), "Seb", None, None, "").affiche, Some(true));
        // PPSSPP neuf (ppsspp.ini pas encore écrit) : l'entrée existe, rien de créé, le choix vaudra ensuite.
        let p = tempfile::tempdir().unwrap();
        let e = au_lancement_de_la_partie("ppsspp", p.path(), "Seb", Some(true), None, "");
        assert_eq!((e.affiche, e.au_lancement, e.en_direct, e.erreur), (Some(false), true, false, None));
        assert_eq!(std::fs::read_dir(p.path()).unwrap().count(), 0, "rien de créé");
        // PCSX2 avec le fichier que Frogtend écrit à la première partie : réglé, SettingsVersion gardé.
        let x = tempfile::tempdir().unwrap();
        std::fs::write(x.path().join("portable.ini"), "").unwrap();
        let ini = x.path().join("inis").join("PCSX2.ini");
        std::fs::create_dir_all(ini.parent().unwrap()).unwrap();
        std::fs::write(&ini, crate::emulateurs_profils::base_neuve(x.path(), &ini)).unwrap();
        let e = au_lancement_de_la_partie("pcsx2", x.path(), "Seb", Some(true), None, "");
        assert_eq!((e.affiche, e.au_lancement, e.en_direct), (Some(true), true, false));
        let t = std::fs::read_to_string(&ini).unwrap();
        assert!(t.starts_with("[UI]\r\nSettingsVersion = 1\r\n") && t.ends_with("[EmuCore/GS]\r\nOsdShowFPS = true\r\n"), "{t}");
    }

    #[test]
    fn pcsx2_position_aucune_avec_d_autres_lignes_pas_d_entree() {
        let coupe = PCSX2.replace("OsdPerformancePos = 3", "OsdPerformancePos = 0").replace("OsdShowSpeed = false", "OsdShowSpeed = true");
        assert!(bloque("pcsx2", &coupe));
        assert!(regler_texte("pcsx2", &coupe, true, false).is_none(), "jamais réveillées");
        assert!(!bloque("pcsx2", &PCSX2.replace("OsdPerformancePos = 3", "OsdPerformancePos = 0")), "aucune autre ligne : permis");
        assert!(!bloque("pcsx2", &PCSX2.replace("OsdShowSpeed = false", "OsdShowSpeed = true")), "position 3 : rien à réveiller");
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("portable.ini"), "").unwrap();
        let ini = d.path().join("inis").join("PCSX2.ini");
        std::fs::create_dir_all(ini.parent().unwrap()).unwrap();
        std::fs::write(&ini, &coupe).unwrap();
        let e = au_lancement_de_la_partie("pcsx2", d.path(), "Seb", Some(true), None, "");
        assert_eq!((e.affiche, e.au_lancement), (Some(false), false), "pas d'entrée");
        assert!(e.raison.is_some_and(|r| r.starts_with("PCSX2 : son affichage des performances")));
        assert_eq!(std::fs::read_to_string(&ini).unwrap(), coupe, "rien touché");
    }

    #[test]
    fn pcsx2_la_position_posee_par_frogtend_est_remise() {
        let d = tempfile::tempdir().unwrap();
        let e = d.path();
        std::fs::write(e.join("portable.ini"), "").unwrap();
        let ini = e.join("inis").join("PCSX2.ini");
        std::fs::create_dir_all(ini.parent().unwrap()).unwrap();
        let coupe = PCSX2.replace("OsdPerformancePos = 3", "OsdPerformancePos = 0");
        std::fs::write(&ini, &coupe).unwrap();
        assert!(poser("pcsx2", e, e, true).unwrap());
        assert_eq!(std::fs::read_to_string(&ini).unwrap(), PCSX2.replace("OsdShowFPS = false", "OsdShowFPS = true"));
        assert!(marque_position(e).is_file(), "Frogtend retient qu'il a changé la position");
        assert!(poser("pcsx2", e, e, false).unwrap());
        assert_eq!(std::fs::read_to_string(&ini).unwrap(), coupe, "remis comme avant");
        assert!(!marque_position(e).exists());
        // Une position déjà réglée par la personne : jamais de marque, jamais remise.
        std::fs::write(&ini, PCSX2).unwrap();
        assert!(poser("pcsx2", e, e, true).unwrap());
        assert!(!marque_position(e).exists());
        assert!(poser("pcsx2", e, e, false).unwrap());
        assert_eq!(std::fs::read_to_string(&ini).unwrap(), PCSX2);
        // Marque, mais la personne a changé la position depuis, ou coché une autre ligne : gardées, marque oubliée.
        std::fs::write(&ini, &coupe).unwrap();
        assert!(poser("pcsx2", e, e, true).unwrap());
        let change = std::fs::read_to_string(&ini).unwrap().replace("OsdPerformancePos = 3", "OsdPerformancePos = 5");
        std::fs::write(&ini, &change).unwrap();
        assert!(poser("pcsx2", e, e, false).unwrap());
        assert!(std::fs::read_to_string(&ini).unwrap().contains("OsdPerformancePos = 5\r\n") && !marque_position(e).exists());
        std::fs::write(&ini, &coupe).unwrap();
        assert!(poser("pcsx2", e, e, true).unwrap());
        let autre = std::fs::read_to_string(&ini).unwrap().replace("OsdShowSpeed = false", "OsdShowSpeed = true");
        std::fs::write(&ini, &autre).unwrap();
        assert!(poser("pcsx2", e, e, false).unwrap());
        let t = std::fs::read_to_string(&ini).unwrap();
        assert!(t.contains("OsdPerformancePos = 3\r\n") && t.contains("OsdShowFPS = false\r\n") && !marque_position(e).exists(), "{t}");
        // Déjà caché, marque d'une position qui n'est plus la nôtre : rien d'écrit, marque oubliée.
        std::fs::write(marque_position(e), "").unwrap();
        std::fs::write(&ini, &change.replace("OsdShowFPS = true", "OsdShowFPS = false")).unwrap();
        assert!(!poser("pcsx2", e, e, false).unwrap());
        assert!(!marque_position(e).exists());
    }

    #[test]
    fn cemu_position_aucune_et_remise() {
        let autres = CEMU.replace("<DrawCalls>false</DrawCalls>", "<DrawCalls>true</DrawCalls>");
        assert!(bloque("cemu", &autres));
        assert!(cemu_regler(&autres, true, false).is_none());
        assert!(!bloque("cemu", CEMU));
        let d = tempfile::tempdir().unwrap();
        let e = d.path();
        std::fs::create_dir_all(e.join("portable")).unwrap();
        let f = e.join("portable").join("settings.xml");
        std::fs::write(&f, &autres).unwrap();
        let etat = au_lancement_de_la_partie("cemu", e, "Seb", Some(true), None, "");
        assert!(!etat.au_lancement && etat.raison.is_some_and(|r| r.starts_with("Cemu :")));
        assert_eq!(std::fs::read_to_string(&f).unwrap(), autres, "rien touché");
        // Position 0 posée à 3 par Frogtend, puis remise à 0 en cachant (FPS gardé, comme avant).
        std::fs::write(&f, CEMU).unwrap();
        assert!(poser("cemu", e, e, true).unwrap());
        assert!(marque_position(e).is_file());
        assert!(poser("cemu", e, e, false).unwrap());
        assert_eq!(std::fs::read_to_string(&f).unwrap(), CEMU, "remis exactement");
        assert!(!marque_position(e).exists());
        // Sans marque : seulement FPS.
        let place = CEMU.replace("<Position>0</Position>\n            <TextColor>", "<Position>3</Position>\n            <TextColor>");
        assert_eq!(cemu_regler(&place, false, false).unwrap(), place.replace("<FPS>true</FPS>", "<FPS>false</FPS>"));
        assert_eq!(cemu_regler(&place, false, true).unwrap(), CEMU, "avec la marque : la position remise");
    }

    #[test]
    fn cemu_le_bloc_ajoute_garde_les_fins_de_ligne() {
        let crlf = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\r\n<content>\r\n    <console_language>2</console_language>\r\n</content>\r\n";
        let x = cemu_regler(crlf, true, false).unwrap();
        assert!(x.ends_with("<Graphic>\r\n<Overlay>\r\n<Position>3</Position>\r\n<FPS>true</FPS>\r\n</Overlay>\r\n</Graphic>\r\n</content>\r\n"), "{x}");
        assert_eq!(x.matches('\n').count(), x.matches("\r\n").count(), "aucune fin de ligne seule");
        let g = cemu_regler("<content>\r\n<Graphic>\r\n<api>1</api>\r\n</Graphic>\r\n</content>\r\n", true, false).unwrap();
        assert_eq!(g, "<content>\r\n<Graphic>\r\n<api>1</api>\r\n<Overlay>\r\n<Position>3</Position>\r\n<FPS>true</FPS>\r\n</Overlay>\r\n</Graphic>\r\n</content>\r\n");
    }

    #[test]
    fn vita3k_la_cle_ajoutee_avant_la_fin_du_document() {
        // Vita3K finit son fichier par `...` : une clé après serait ignorée (seul le premier document est lu).
        let t = "---\nuser-id: \"00\"\nshow-welcome: false\n...\n";
        assert_eq!(regler_texte("vita3k", t, true, false).unwrap(), "---\nuser-id: \"00\"\nshow-welcome: false\nperformance-overlay: true\n...\n");
        let crlf = "---\r\nuser-id: \"00\"\r\n...\r\n";
        assert_eq!(regler_texte("vita3k", crlf, true, false).unwrap(), "---\r\nuser-id: \"00\"\r\nperformance-overlay: true\r\n...\r\n");
        // Sans marque de fin : à la fin, comme avant ; présente : remplacée sur place.
        assert_eq!(crate::vita3k::remplacer_ligne("---\nuser-id: 00\n", "sys-lang", "sys-lang: 2"), "---\nuser-id: 00\nsys-lang: 2\n");
        assert_eq!(regler_texte("vita3k", VITA3K, true, false).unwrap(), VITA3K.replace("performance-overlay: false", "performance-overlay: true"));
    }

    #[test]
    fn dolphin_un_c_dans_la_ligne_decide() {
        assert_eq!(dolphin_dans_la_ligne("-b -e \"{rom}\""), None);
        assert_eq!(dolphin_dans_la_ligne("-C Graphics.Settings.ShowFPS=True -b"), Some(true));
        assert_eq!(dolphin_dans_la_ligne("\"--config=graphics.settings.showfps=false\" -e x"), Some(false));
        assert_eq!(dolphin_dans_la_ligne("-CGraphics.Settings.ShowFPS=1"), Some(true));
        assert_eq!(dolphin_dans_la_ligne("--config Graphics.Settings.ShowFPS=True -C Graphics.Settings.ShowFPS=False"), Some(false), "le dernier l'emporte");
        assert_eq!(dolphin_dans_la_ligne("-C Graphics.Settings.ShowFTimes=True -C Dolphin.Core.CPUThread=False"), None, "une autre clé");
        let d = tempfile::tempdir().unwrap();
        let e = au_lancement_de_la_partie("dolphin", d.path(), "Seb", Some(false), None, "\"-u\" \"x\" -C Graphics.Settings.ShowFPS=True -b -e \"{rom}\"");
        assert!(e.arguments.is_empty(), "le nôtre, mis devant, perdrait : pas ajouté");
        assert_eq!((e.affiche, e.au_lancement), (Some(true), false));
        assert!(e.raison.is_some_and(|r| r.starts_with("Dolphin :")));
        // Sa ligne n'en parle pas : le nôtre est ajouté.
        assert!(!au_lancement_de_la_partie("dolphin", d.path(), "Seb", Some(false), None, "-b -e \"{rom}\"").arguments.is_empty());
    }
}
