//! Les manettes PlayStation, Switch, 8BitDo… pendant la partie (lot OSD, étape 4), par SDL3 compilé DANS le programme
//! (décision de Seb, 30/09 : « SDL3 embarqué, aucune DLL »). XInput lit déjà les manettes Xbox (`osd_manette::Lecteur`) ;
//! ce module lui ajoute les autres, dans le même état (`EtatManette`, bits XInput).
//!
//! Relu avec l'expert lancement (16/10), sources de SDL 3.4.18 :
//! - UN fil SDL pour tout le processus, lancé au premier besoin et jamais arrêté : il ouvre et ferme les manettes et
//!   publie leur état ; les lecteurs (menu en jeu, manette → clavier) ne font que le lire.
//! - Les évènements SDL (manette ET joystick) sont coupés, et la file vidée à chaque tour (personne ne les lit : elle
//!   monterait à 65 535) ; l'état se relit par `SDL_UpdateGamepads`, seulement pendant une partie. Hors partie, les
//!   manettes sont REFERMÉES (une manette Switch revient alors à son mode normal pour les autres programmes).
//! - Les manettes vues par XInput (chemin `XInput#…` : Xbox, manette virtuelle de Sunshine, celle de Steam) sont
//!   laissées à XInput, quel que soit le type annoncé : jamais lues deux fois.
//! - Frogtend touche le MOINS possible à la manette : pas de rapports « enrichis » demandés (ils cassent DirectInput
//!   pour les autres), voyants de joueur PS5 et Switch éteints plutôt qu'un numéro de joueur, ni vibration ni lumière
//!   demandées. Mais SDL lui-même, à l'ouverture, met une PS4/PS5 en USB en mode enrichi (couleur de barre lumineuse
//!   envoyée) et une manette Switch en rapport complet (voyants de joueur éteints) : ce n'est pas réglable (sources
//!   SDL_hidapi_ps5.c:935, SDL_hidapi_switch.c:780-851), c'est dit à Seb et à essayer avec les émulateurs.
//!   DirectInput est coupé : SDL y prend la manette en EXCLUSIF (SDL_dinputjoystick.c), ce qui pourrait la retirer à
//!   l'émulateur — une manette qui ne passe QUE par DirectInput (générique) n'ouvre donc pas le menu en jeu.
//! - Les boutons sont pris PAR POSITION : le bouton du bas valide (B sur une manette Nintendo), comme dans Taodbox.

use crate::osd_manette::{EtatManette, A, B, BAS, DROITE, GAUCHE, GUIDE, HAUT, L1, L3, R1, R3, SELECT, START};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// X et Y de XInput (non nommés dans `osd_manette`).
const X: u16 = 0x4000;
const Y: u16 = 0x8000;

/// Une manette vue par XInput : laissée à XInput (son chemin SDL, `SDL_xinputjoystick.c`).
pub fn ecartee(chemin: &str) -> bool {
    chemin.starts_with("XInput#")
}

/// L'axe vertical SDL (positif vers le BAS) en XInput (positif vers le HAUT), sans débordement.
pub fn stick_y(v: i16) -> i16 {
    if v == i16::MIN {
        i16::MAX
    } else {
        -v
    }
}

/// Une gâchette SDL (0 à 32 767) en XInput (0 à 255).
pub fn gachette(v: i16) -> u8 {
    ((v.max(0) as u32 * 255) / 32_767) as u8
}

/// Deux lectures réunies : les boutons de l'une OU de l'autre, le stick le plus poussé, la gâchette la plus enfoncée.
pub fn reunir(a: EtatManette, b: EtatManette) -> EtatManette {
    let force = |e: &EtatManette| (e.lx as i32).abs() + (e.ly as i32).abs();
    let (lx, ly) = if force(&b) > force(&a) { (b.lx, b.ly) } else { (a.lx, a.ly) };
    EtatManette { boutons: a.boutons | b.boutons, lx, ly, gt: a.gt.max(b.gt), dt: a.dt.max(b.dt) }
}

#[cfg(windows)]
mod sdl {
    use super::*;
    use sdl3_sys::everything::*;

    /// Les boutons SDL (par position) et leur bit XInput.
    const BOUTONS: [(SDL_GamepadButton, u16); 15] = [
        (SDL_GAMEPAD_BUTTON_SOUTH, A),
        (SDL_GAMEPAD_BUTTON_EAST, B),
        (SDL_GAMEPAD_BUTTON_WEST, X),
        (SDL_GAMEPAD_BUTTON_NORTH, Y),
        (SDL_GAMEPAD_BUTTON_BACK, SELECT),
        (SDL_GAMEPAD_BUTTON_START, START),
        (SDL_GAMEPAD_BUTTON_LEFT_STICK, L3),
        (SDL_GAMEPAD_BUTTON_RIGHT_STICK, R3),
        (SDL_GAMEPAD_BUTTON_LEFT_SHOULDER, L1),
        (SDL_GAMEPAD_BUTTON_RIGHT_SHOULDER, R1),
        (SDL_GAMEPAD_BUTTON_DPAD_UP, HAUT),
        (SDL_GAMEPAD_BUTTON_DPAD_DOWN, BAS),
        (SDL_GAMEPAD_BUTTON_DPAD_LEFT, GAUCHE),
        (SDL_GAMEPAD_BUTTON_DPAD_RIGHT, DROITE),
        // Le bouton PS / Home / 8BitDo : comme le bouton Xbox (il n'ouvre le menu que si le réglage le permet).
        (SDL_GAMEPAD_BUTTON_GUIDE, GUIDE),
    ];

    /// Les bits XInput des boutons enfoncés, d'après `appuye(bouton SDL)`.
    pub fn bits(appuye: impl Fn(SDL_GamepadButton) -> bool) -> u16 {
        BOUTONS.iter().filter(|(b, _)| appuye(*b)).fold(0, |acc, (_, bit)| acc | bit)
    }

    /// Une manette ouverte : touchée SEULEMENT par le fil SDL.
    struct Ouverte(*mut SDL_Gamepad);

    /// Le fil SDL : prépare SDL, puis relit les manettes tant que quelqu'un s'en sert (sinon il dort).
    pub fn fil(
        etat: &'static Mutex<EtatManette>,
        demande: &'static AtomicU64,
        debut: Instant,
        erreur: &'static Mutex<Option<String>>,
        vues: &'static Mutex<Vec<String>>,
    ) {
        unsafe {
            // Avant SDL_Init (SDL_hints.h) : lue en arrière-plan (le jeu a le premier plan), rien changé à la manette.
            SDL_SetHint(c"SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS".as_ptr(), c"1".as_ptr());
            SDL_SetHint(c"SDL_JOYSTICK_ENHANCED_REPORTS".as_ptr(), c"auto".as_ptr());
            SDL_SetHint(c"SDL_JOYSTICK_HIDAPI_PS5_PLAYER_LED".as_ptr(), c"0".as_ptr());
            SDL_SetHint(c"SDL_JOYSTICK_HIDAPI_SWITCH_PLAYER_LED".as_ptr(), c"0".as_ptr());
            SDL_SetHint(c"SDL_JOYSTICK_DIRECTINPUT".as_ptr(), c"0".as_ptr());
            if !SDL_Init(SDL_INIT_GAMEPAD) {
                let e = std::ffi::CStr::from_ptr(SDL_GetError()).to_string_lossy().to_string();
                *erreur.lock().unwrap_or_else(|p| p.into_inner()) = Some(e);
                return;
            }
            SDL_SetGamepadEventsEnabled(false);
            SDL_SetJoystickEventsEnabled(false);
            SDL_SetEventEnabled(SDL_EVENT_JOYSTICK_UPDATE_COMPLETE.into(), false);
        }
        let mut ouvertes: Vec<Ouverte> = Vec::new();
        let mut garde_guide = crate::osd_manette::GardeGuide::<SDL_JoystickID>::default();
        // Les manettes déjà vues (laissées à XInput, ou refusées à l'ouverture) : notées une fois, pas retentées.
        let mut ecartees: Vec<SDL_JoystickID> = Vec::new();
        let mut releve = Instant::now() - Duration::from_secs(2);
        loop {
            // Personne ne s'en sert depuis 2 s (pas de partie) : les manettes se referment, et on dort.
            let derniere = Duration::from_millis(demande.load(Ordering::Relaxed));
            if debut.elapsed().saturating_sub(derniere) > Duration::from_secs(2) {
                if !ouvertes.is_empty() {
                    for g in ouvertes.drain(..) {
                        unsafe { SDL_CloseGamepad(g.0) };
                    }
                    *etat.lock().unwrap_or_else(|p| p.into_inner()) = EtatManette::default();
                }
                ecartees.clear();
                releve = Instant::now() - Duration::from_secs(2);
                unsafe { SDL_FlushEvents(SDL_EVENT_FIRST.into(), SDL_EVENT_LAST.into()) };
                std::thread::sleep(Duration::from_millis(250));
                continue;
            }
            unsafe {
                SDL_UpdateGamepads();
                SDL_FlushEvents(SDL_EVENT_FIRST.into(), SDL_EVENT_LAST.into());
                if releve.elapsed() >= Duration::from_secs(1) {
                    releve = Instant::now();
                    // Les débranchées se ferment ; les nouvelles s'ouvrent (sauf celles d'XInput).
                    ouvertes.retain(|g| {
                        let ok = SDL_GamepadConnected(g.0);
                        if !ok {
                            SDL_CloseGamepad(g.0);
                        }
                        ok
                    });
                    let mut n = 0;
                    let ids = SDL_GetGamepads(&mut n);
                    if !ids.is_null() {
                        for k in 0..n.max(0) as usize {
                            let id = *ids.add(k);
                            if ecartees.contains(&id) || ouvertes.iter().any(|g| SDL_GetGamepadID(g.0) == id) {
                                continue;
                            }
                            let chemin = SDL_GetGamepadPathForID(id);
                            let chemin = if chemin.is_null() { String::new() } else { std::ffi::CStr::from_ptr(chemin).to_string_lossy().to_string() };
                            let nom = SDL_GetGamepadNameForID(id);
                            let nom = if nom.is_null() { "?".to_string() } else { std::ffi::CStr::from_ptr(nom).to_string_lossy().to_string() };
                            let noter = |quoi: String| vues.lock().unwrap_or_else(|p| p.into_inner()).push(format!("{nom} ({chemin}) : {quoi}"));
                            if ecartee(&chemin) {
                                noter("laissée à XInput".into());
                                ecartees.push(id);
                                continue;
                            }
                            let g = SDL_OpenGamepad(id);
                            if g.is_null() {
                                noter(format!("refusée ({})", std::ffi::CStr::from_ptr(SDL_GetError()).to_string_lossy()));
                                ecartees.push(id);
                            } else {
                                noter("lue par SDL".into());
                                ouvertes.push(Ouverte(g));
                            }
                        }
                        SDL_free(ids as *mut _);
                    }
                }
                let mut e = EtatManette::default();
                // Le bouton PS / Home d'une manette tout juste ouverte (qu'il vient d'allumer) ne compte pas.
                let presentes: Vec<SDL_JoystickID> = ouvertes.iter().map(|g| SDL_GetGamepadID(g.0)).collect();
                garde_guide.retenir(&presentes);
                for g in &ouvertes {
                    let g = g.0;
                    let lue = EtatManette {
                        boutons: garde_guide.filtrer(SDL_GetGamepadID(g), bits(|b| SDL_GetGamepadButton(g, b))),
                        lx: SDL_GetGamepadAxis(g, SDL_GAMEPAD_AXIS_LEFTX),
                        ly: stick_y(SDL_GetGamepadAxis(g, SDL_GAMEPAD_AXIS_LEFTY)),
                        gt: gachette(SDL_GetGamepadAxis(g, SDL_GAMEPAD_AXIS_LEFT_TRIGGER)),
                        dt: gachette(SDL_GetGamepadAxis(g, SDL_GAMEPAD_AXIS_RIGHT_TRIGGER)),
                    };
                    e = reunir(e, lue);
                }
                *etat.lock().unwrap_or_else(|p| p.into_inner()) = e;
            }
            // Le menu relit toutes les 16 ms : autant ici.
            std::thread::sleep(Duration::from_millis(16));
        }
    }
}

struct Partage {
    etat: Mutex<EtatManette>,
    demande: AtomicU64,
    debut: Instant,
    erreur: Mutex<Option<String>>,
    /// Les manettes vues par SDL (nom, chemin, lue ou laissée à XInput), pour le journal.
    vues: Mutex<Vec<String>>,
}

fn partage() -> &'static Partage {
    static P: OnceLock<&'static Partage> = OnceLock::new();
    P.get_or_init(|| {
        let p: &'static Partage = Box::leak(Box::new(Partage {
            etat: Mutex::new(EtatManette::default()),
            demande: AtomicU64::new(0),
            debut: Instant::now(),
            erreur: Mutex::new(None),
            vues: Mutex::new(Vec::new()),
        }));
        #[cfg(windows)]
        {
            if let Err(e) = std::thread::Builder::new()
                .name("manettes-sdl".into())
                .spawn(move || sdl::fil(&p.etat, &p.demande, p.debut, &p.erreur, &p.vues))
            {
                *p.erreur.lock().unwrap_or_else(|x| x.into_inner()) = Some(format!("fil non créé : {e}"));
            }
        }
        p
    })
}

/// L'état des manettes lues par SDL (aucune Xbox) ; lance le fil SDL au premier appel et le tient éveillé.
pub fn etat() -> EtatManette {
    let p = partage();
    p.demande.store(p.debut.elapsed().as_millis() as u64, Ordering::Relaxed);
    *p.etat.lock().unwrap_or_else(|e| e.into_inner())
}

/// Les manettes vues par SDL depuis la dernière fois (nom, chemin, lue ou laissée à XInput) : pour le journal.
pub fn nouvelles_vues() -> Vec<String> {
    std::mem::take(&mut *partage().vues.lock().unwrap_or_else(|e| e.into_inner()))
}

/// L'échec de démarrage de SDL, s'il y en a eu un (pour le journal) : XInput continue seul.
pub fn erreur() -> Option<String> {
    partage().erreur.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_manette_sdl_se_lit_en_bits_xinput_par_position() {
        #[cfg(windows)]
        {
            use sdl3_sys::everything::*;
            assert_eq!(sdl::bits(|b| b == SDL_GAMEPAD_BUTTON_SOUTH), A, "le bouton du bas valide");
            assert_eq!(sdl::bits(|b| b == SDL_GAMEPAD_BUTTON_EAST), B);
            assert_eq!(sdl::bits(|b| b == SDL_GAMEPAD_BUTTON_BACK || b == SDL_GAMEPAD_BUTTON_RIGHT_SHOULDER), SELECT | R1, "la combinaison du menu");
            assert_eq!(sdl::bits(|b| b == SDL_GAMEPAD_BUTTON_GUIDE), GUIDE, "le bouton PS/Home : comme le bouton Xbox");
            assert_eq!(sdl::bits(|_| true).count_ones(), 15);
        }
        assert_eq!(stick_y(100), -100);
        assert_eq!(stick_y(i16::MIN), i16::MAX, "sans débordement");
        assert_eq!(gachette(32_767), 255);
        assert_eq!(gachette(0), 0);
        assert_eq!(gachette(-5), 0);
        assert!(ecartee("XInput#0") && !ecartee(r"\\?\HID#VID_054C&PID_0CE6"), "PS5 lue, Xbox laissée à XInput");
    }

    #[test]
    fn deux_lectures_se_reunissent() {
        let xinput = EtatManette { boutons: SELECT, lx: 100, ly: 0, gt: 10, dt: 0 };
        let sdl = EtatManette { boutons: R1, lx: 0, ly: -20_000, gt: 0, dt: 200 };
        let r = reunir(xinput, sdl);
        assert_eq!(r.boutons, SELECT | R1);
        assert_eq!((r.lx, r.ly), (0, -20_000), "le stick le plus poussé");
        assert_eq!((r.gt, r.dt), (10, 200));
    }

    /// RÉEL : SDL démarre dans Frogtend, et lit les manettes branchées pendant 10 s (affiche les boutons vus).
    /// `cargo test essai_manettes_sdl -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn essai_manettes_sdl() {
        let debut = Instant::now();
        let mut vus = 0u16;
        while debut.elapsed() < Duration::from_secs(10) {
            let e = etat();
            if e.boutons & !vus != 0 {
                println!("BOUTONS {:#06x}", e.boutons);
            }
            vus |= e.boutons;
            std::thread::sleep(Duration::from_millis(20));
        }
        println!("MANETTES VUES : {:?}", nouvelles_vues());
        println!("ERREUR SDL : {:?} ; boutons vus en 10 s : {vus:#06x}", erreur());
    }
}
