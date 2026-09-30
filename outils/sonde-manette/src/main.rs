//! Sonde jetable du lot 4 ter : la manette est-elle lue quand la sonde N'A PAS le premier plan (un jeu l'a) ?
//!
//! Deux voies, lues côte à côte toutes les 50 ms :
//! - XInput (xinput1_4.dll) : manettes Xbox et manette virtuelle de Sunshine ;
//! - SDL3, avec `SDL_HINT_JOYSTICK_ALLOW_BACKGROUND_EVENTS = 1` : PS4, PS5, Switch Pro, 8BitDo, génériques.
//!
//! Chaque appui est noté avec la fenêtre au premier plan. À la fin (Ctrl+C ou 3 minutes), un bilan par manette :
//! vue au premier plan (sonde), vue en arrière-plan (jeu). Le journal va dans `sonde-manette.txt`, à côté.

use sdl3_sys::everything::*;
use std::collections::BTreeMap;
use std::ffi::CStr;
use std::io::Write;
use std::time::{Duration, Instant};
use windows::Win32::UI::Input::XboxController::{XInputGetState, XINPUT_STATE};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId};

/// La fenêtre au premier plan : (titre, est-ce la sonde elle-même ?).
fn premier_plan() -> (String, bool) {
    unsafe {
        let h = GetForegroundWindow();
        let mut titre = [0u16; 256];
        let n = GetWindowTextW(h, &mut titre);
        let mut pid = 0u32;
        GetWindowThreadProcessId(h, Some(&mut pid));
        let titre = String::from_utf16_lossy(&titre[..n.max(0) as usize]);
        // La console de la sonde appartient à conhost/Terminal, pas à notre processus : on la reconnaît à son titre.
        let sonde = pid == std::process::id() || titre.to_lowercase().contains("sonde-manette");
        (titre, sonde)
    }
}

const NOMS_XINPUT: [(u16, &str); 14] = [
    (0x0001, "Haut"),
    (0x0002, "Bas"),
    (0x0004, "Gauche"),
    (0x0008, "Droite"),
    (0x0010, "Start"),
    (0x0020, "Back/Select"),
    (0x0040, "L3"),
    (0x0080, "R3"),
    (0x0100, "LB"),
    (0x0200, "RB"),
    (0x1000, "A"),
    (0x2000, "B"),
    (0x4000, "X"),
    (0x8000, "Y"),
];

#[derive(Default)]
struct Bilan {
    au_premier_plan: u32,
    en_arriere_plan: u32,
    fenetres: BTreeMap<String, u32>,
}

fn main() {
    let journal_chemin = std::env::current_exe().unwrap().with_file_name("sonde-manette.txt");
    let mut journal = std::fs::File::create(&journal_chemin).expect("journal");
    let mut ecrire = |ligne: String| {
        println!("{ligne}");
        let _ = writeln!(journal, "{ligne}");
    };
    ecrire("Sonde manette (Frogtend, lot 4 ter). Appuie sur des boutons, ici puis DANS UN JEU (cette fenêtre en arrière-plan).".into());
    ecrire("Durée : 3 minutes, ou Ctrl+C. Journal : sonde-manette.txt".into());

    unsafe {
        SDL_SetHint(SDL_HINT_JOYSTICK_ALLOW_BACKGROUND_EVENTS, c"1".as_ptr());
        if !SDL_Init(SDL_INIT_GAMEPAD) {
            ecrire(format!("SDL : échec d'initialisation ({})", CStr::from_ptr(SDL_GetError()).to_string_lossy()));
        }
    }

    let mut bilans: BTreeMap<String, Bilan> = BTreeMap::new();
    let mut avant: BTreeMap<String, Vec<&'static str>> = BTreeMap::new();
    let mut ouvertes: Vec<*mut SDL_Gamepad> = Vec::new();
    let debut = Instant::now();
    let mut derniere_liste = Instant::now() - Duration::from_secs(10);

    while debut.elapsed() < Duration::from_secs(180) {
        let (fenetre, sonde) = premier_plan();
        let mut presses: Vec<(String, Vec<&'static str>)> = Vec::new();

        // XInput : 4 emplacements.
        for i in 0..4u32 {
            let mut e = XINPUT_STATE::default();
            if unsafe { XInputGetState(i, &mut e) } == 0 {
                let b = e.Gamepad.wButtons.0;
                let mut l: Vec<&str> = NOMS_XINPUT.iter().filter(|(m, _)| b & m != 0).map(|(_, n)| *n).collect();
                if e.Gamepad.bLeftTrigger > 100 {
                    l.push("LT");
                }
                if e.Gamepad.bRightTrigger > 100 {
                    l.push("RT");
                }
                presses.push((format!("XInput n°{}", i + 1), l));
            }
        }

        // SDL : (ré)ouvrir les manettes branchées toutes les 2 s.
        unsafe {
            SDL_UpdateGamepads();
            if derniere_liste.elapsed() > Duration::from_secs(2) {
                derniere_liste = Instant::now();
                for g in ouvertes.drain(..) {
                    SDL_CloseGamepad(g);
                }
                let mut n = 0;
                let ids = SDL_GetGamepads(&mut n);
                for k in 0..n.max(0) as usize {
                    let g = SDL_OpenGamepad(*ids.add(k));
                    if !g.is_null() {
                        ouvertes.push(g);
                    }
                }
                SDL_free(ids as *mut _);
            }
            for (k, g) in ouvertes.iter().enumerate() {
                let nom = SDL_GetGamepadName(*g);
                let nom = if nom.is_null() { "?".into() } else { CStr::from_ptr(nom).to_string_lossy().to_string() };
                let boutons = [
                    (SDL_GAMEPAD_BUTTON_SOUTH, "Sud (A/Croix)"),
                    (SDL_GAMEPAD_BUTTON_EAST, "Est (B/Rond)"),
                    (SDL_GAMEPAD_BUTTON_WEST, "Ouest (X/Carré)"),
                    (SDL_GAMEPAD_BUTTON_NORTH, "Nord (Y/Triangle)"),
                    (SDL_GAMEPAD_BUTTON_BACK, "Back/Select"),
                    (SDL_GAMEPAD_BUTTON_START, "Start"),
                    (SDL_GAMEPAD_BUTTON_GUIDE, "Guide/PS/Home"),
                    (SDL_GAMEPAD_BUTTON_LEFT_SHOULDER, "L1"),
                    (SDL_GAMEPAD_BUTTON_RIGHT_SHOULDER, "R1"),
                    (SDL_GAMEPAD_BUTTON_DPAD_UP, "Haut"),
                    (SDL_GAMEPAD_BUTTON_DPAD_DOWN, "Bas"),
                ];
                let l: Vec<&str> = boutons.iter().filter(|(b, _)| SDL_GetGamepadButton(*g, *b)).map(|(_, n)| *n).collect();
                presses.push((format!("SDL n°{} « {nom} »", k + 1), l));
            }
        }

        for (manette, l) in presses {
            let precedent = avant.insert(manette.clone(), l.clone()).unwrap_or_default();
            let nouveaux: Vec<&str> = l.iter().filter(|b| !precedent.contains(b)).copied().collect();
            if nouveaux.is_empty() {
                continue;
            }
            let b = bilans.entry(manette.clone()).or_default();
            let ou = if sonde {
                b.au_premier_plan += 1;
                "sonde au premier plan".to_string()
            } else {
                b.en_arriere_plan += 1;
                *b.fenetres.entry(fenetre.clone()).or_default() += 1;
                format!("EN ARRIÈRE-PLAN, premier plan : « {fenetre} »")
            };
            ecrire(format!("[{:>5.1} s] {manette} : {} — {ou}", debut.elapsed().as_secs_f32(), nouveaux.join(" + ")));
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    ecrire(String::new());
    ecrire("=== BILAN ===".into());
    if bilans.is_empty() {
        ecrire("Aucun appui vu sur aucune manette.".into());
    }
    for (m, b) in &bilans {
        ecrire(format!(
            "{m} : {} appui(s) vus avec la sonde au premier plan, {} en arrière-plan{}",
            b.au_premier_plan,
            b.en_arriere_plan,
            if b.en_arriere_plan > 0 {
                format!(" (pendant : {})", b.fenetres.keys().cloned().collect::<Vec<_>>().join(", "))
            } else {
                " → ⚠ JAMAIS vue en arrière-plan".into()
            }
        ));
    }
    ecrire(format!("Journal : {}", journal_chemin.display()));
}
