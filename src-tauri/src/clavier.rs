//! Manette → clavier pour les jeux PC qui ne connaissent que le clavier (Seb, 16/10 : « le plus généraliste
//! possible » ; n'importe quelle manette doit marcher partout). Une correspondance PAR JEU (bouton → touche), choisie
//! dans l'interface (préréglages courants ou bouton par bouton). Pendant la partie, le fil qui lit déjà la manette
//! (menu en jeu) traduit les boutons en touches :
//! - seulement vers le jeu au premier plan (un programme de SON dossier), vérifié à CHAQUE envoi ; jamais pendant le
//!   menu ni un jeu figé ;
//! - toutes les touches tenues sont relâchées AVANT l'ouverture du menu, quand le jeu perd le premier plan, à la fin de
//!   la partie, et si Frogtend plante (crochet de panique) ;
//! - la combinaison du menu n'envoie pas ses touches au jeu, même appuyée en deux temps ;
//! - les lettres partent selon la disposition du clavier (AZERTY : « Z » arrive « Z ») ; les touches « Déplacement »
//!   partent par leur POSITION (celles de ZQSD sur un clavier français, de WASD sur un clavier anglais).

use crate::osd_manette::EtatManette;
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Les boutons de la manette qu'on peut associer (nom stable, nom montré). Les directions : la croix OU le stick gauche.
pub const BOUTONS: &[(&str, &str)] = &[
    ("haut", "Haut"),
    ("bas", "Bas"),
    ("gauche", "Gauche"),
    ("droite", "Droite"),
    ("a", "A"),
    ("b", "B"),
    ("x", "X"),
    ("y", "Y"),
    ("lb", "LB"),
    ("rb", "RB"),
    ("lt", "LT"),
    ("rt", "RT"),
    ("start", "Start"),
    ("select", "View (Back)"),
    ("l3", "L3 (clic du stick gauche)"),
    ("r3", "R3 (clic du stick droit)"),
];

/// Les touches qu'on peut envoyer : (nom montré, touche virtuelle Windows, étendue, POSITION fixe). Une position fixe
/// (code de balayage) part telle quelle, quelle que soit la disposition du clavier ; sinon la touche virtuelle est
/// traduite selon la disposition du jeu au premier plan.
pub const TOUCHES: &[(&str, u16, bool, u16)] = &[
    ("Déplacement haut", 0, false, 0x11),
    ("Déplacement gauche", 0, false, 0x1E),
    ("Déplacement bas", 0, false, 0x1F),
    ("Déplacement droite", 0, false, 0x20),
    ("Haut", 0x26, true, 0), ("Bas", 0x28, true, 0), ("Gauche", 0x25, true, 0), ("Droite", 0x27, true, 0),
    ("Entrée", 0x0D, false, 0), ("Espace", 0x20, false, 0), ("Échap", 0x1B, false, 0), ("Maj", 0xA0, false, 0),
    ("Ctrl", 0xA2, false, 0), ("Alt", 0xA4, false, 0), ("Tab", 0x09, false, 0), ("Retour arrière", 0x08, false, 0),
    ("A", 0x41, false, 0), ("B", 0x42, false, 0), ("C", 0x43, false, 0), ("D", 0x44, false, 0), ("E", 0x45, false, 0),
    ("F", 0x46, false, 0), ("G", 0x47, false, 0), ("H", 0x48, false, 0), ("I", 0x49, false, 0), ("J", 0x4A, false, 0),
    ("K", 0x4B, false, 0), ("L", 0x4C, false, 0), ("M", 0x4D, false, 0), ("N", 0x4E, false, 0), ("O", 0x4F, false, 0),
    ("P", 0x50, false, 0), ("Q", 0x51, false, 0), ("R", 0x52, false, 0), ("S", 0x53, false, 0), ("T", 0x54, false, 0),
    ("U", 0x55, false, 0), ("V", 0x56, false, 0), ("W", 0x57, false, 0), ("X", 0x58, false, 0), ("Y", 0x59, false, 0),
    ("Z", 0x5A, false, 0),
    ("0", 0x30, false, 0), ("1", 0x31, false, 0), ("2", 0x32, false, 0), ("3", 0x33, false, 0), ("4", 0x34, false, 0),
    ("5", 0x35, false, 0), ("6", 0x36, false, 0), ("7", 0x37, false, 0), ("8", 0x38, false, 0), ("9", 0x39, false, 0),
    ("F1", 0x70, false, 0), ("F2", 0x71, false, 0), ("F3", 0x72, false, 0), ("F4", 0x73, false, 0), ("F5", 0x74, false, 0),
    ("F6", 0x75, false, 0), ("F7", 0x76, false, 0), ("F8", 0x77, false, 0), ("F9", 0x78, false, 0), ("F10", 0x79, false, 0),
    ("F11", 0x7A, false, 0), ("F12", 0x7B, false, 0),
    ("Inser", 0x2D, true, 0), ("Suppr", 0x2E, true, 0), ("Origine", 0x24, true, 0), ("Fin", 0x23, true, 0),
    ("Page préc.", 0x21, true, 0), ("Page suiv.", 0x22, true, 0),
];

/// Les bits XInput des boutons (comme `osd_manette`).
fn bit(bouton: &str) -> Option<u16> {
    Some(match bouton {
        "haut" => 0x0001,
        "bas" => 0x0002,
        "gauche" => 0x0004,
        "droite" => 0x0008,
        "start" => 0x0010,
        "select" => 0x0020,
        "l3" => 0x0040,
        "r3" => 0x0080,
        "lb" => 0x0100,
        "rb" => 0x0200,
        "a" => 0x1000,
        "b" => 0x2000,
        "x" => 0x4000,
        "y" => 0x8000,
        _ => return None,
    })
}

/// Le stick gauche compte comme une direction au-delà de la moitié de sa course ; une gâchette, à mi-course.
const SEUIL_STICK: i32 = 16_000;
const SEUIL_GACHETTE: u8 = 128;
/// Un bouton de la combinaison du menu appuyé seul attend ce délai avant d'envoyer sa touche : la combinaison
/// s'appuie rarement des deux mains dans la même trame.
const ATTENTE_COMBO: Duration = Duration::from_millis(150);

fn appuye(bouton: &str, boutons: u16, e: &EtatManette) -> bool {
    bit(bouton).is_some_and(|b| boutons & b != 0)
        || match bouton {
            "haut" => (e.ly as i32) > SEUIL_STICK,
            "bas" => (e.ly as i32) < -SEUIL_STICK,
            "gauche" => (e.lx as i32) < -SEUIL_STICK,
            "droite" => (e.lx as i32) > SEUIL_STICK,
            "lt" => e.gt >= SEUIL_GACHETTE,
            "rt" => e.dt >= SEUIL_GACHETTE,
            _ => false,
        }
}

/// Une touche à envoyer : touche virtuelle OU position fixe, étendue, appuyée (sinon relâchée). Une `repetition` est
/// un nouvel appui d'une touche DÉJÀ tenue (sans relâchement), comme le fait un vrai clavier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frappe {
    pub vk: u16,
    pub position: u16,
    pub etendue: bool,
    pub bas: bool,
    pub repetition: bool,
}

type Touche = (u16, u16, bool);

/// La répétition d'une touche tenue (IDEES, 16/10 : les vieux menus ne défilent que par répétition). SendInput ne
/// répète pas : c'est le pilote d'un vrai clavier qui le fait. Frogtend imite donc Windows : son délai et sa cadence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Repetition {
    pub delai: Duration,
    pub periode: Duration,
}

impl Repetition {
    /// Depuis les réglages du clavier de Windows (`SPI_GETKEYBOARDDELAY` 0 à 3 : 250 à 1000 ms ; `SPI_GETKEYBOARDSPEED`
    /// 0 à 31 : environ 2,5 à 30 fois par seconde, doc de Microsoft — entre les deux bouts, une approximation linéaire).
    pub fn de_windows(delai: u32, vitesse: u32) -> Self {
        let delai = Duration::from_millis(250 * (u64::from(delai.min(3)) + 1));
        let par_seconde = 2.5 + (30.0 - 2.5) * f64::from(vitesse.min(31)) / 31.0;
        Repetition { delai, periode: Duration::from_secs_f64(1.0 / par_seconde) }
    }

    /// Ceux de ce PC (les valeurs par défaut de Windows si la lecture échoue).
    pub fn de_ce_pc() -> Self {
        #[cfg(windows)]
        {
            let (d, v) = windows_impl::reglages_clavier().unwrap_or((1, 31));
            Repetition::de_windows(d, v)
        }
        #[cfg(not(windows))]
        {
            Repetition::de_windows(1, 31)
        }
    }
}

/// Maj, Ctrl, Alt ne se répètent pas (inutile, et certains jeux s'en gênent).
fn se_repete(t: &Touche) -> bool {
    !matches!(t.0, 0xA0 | 0xA2 | 0xA4)
}

/// Le moteur d'un jeu : sa correspondance et les touches qu'IL tient (il ne relâche jamais que les siennes).
pub struct Moteur {
    liens: Vec<(&'static str, Touche)>,
    combo: u16,
    tenues: Vec<Touche>,
    /// La combinaison a été vue complète : ses boutons restent muets jusqu'à ce qu'ils soient TOUS relâchés.
    combo_vue: bool,
    /// Depuis quand un bouton de la combinaison est appuyé SEUL (pas encore la combinaison), et lesquels.
    partiel_depuis: Option<Instant>,
    partiel_bits: u16,
    /// La répétition (`None` : coupée pour ce jeu).
    repetition: Option<Repetition>,
    /// Comme un vrai clavier : seule la DERNIÈRE touche appuyée se répète ; l'instant de sa prochaine répétition. (Une
    /// différence voulue : Maj, Ctrl ou Alt appuyée pendant ce temps ne l'arrête pas — Maj + Z en continu dans un jeu.)
    derniere: Option<(Touche, Instant)>,
}

impl Moteur {
    /// Depuis la correspondance du jeu (noms de boutons et de touches) ; les noms inconnus sont ignorés.
    pub fn nouveau(correspondance: &BTreeMap<String, String>, combo: u16) -> Self {
        let liens = correspondance
            .iter()
            .filter_map(|(b, t)| {
                let b = BOUTONS.iter().find(|x| x.0 == b)?.0;
                let &(_, vk, ext, pos) = TOUCHES.iter().find(|x| x.0 == t)?;
                Some((b, (vk, pos, ext)))
            })
            .collect();
        Moteur { liens, combo, tenues: Vec::new(), combo_vue: false, partiel_depuis: None, partiel_bits: 0, repetition: None, derniere: None }
    }

    /// Avec la répétition d'une touche tenue (`None` : coupée pour ce jeu).
    pub fn avec_repetition(mut self, r: Option<Repetition>) -> Self {
        self.repetition = r;
        self
    }

    pub fn est_vide(&self) -> bool {
        self.liens.is_empty()
    }

    /// Les boutons à prendre en compte : ceux de la combinaison du menu retirés tant qu'elle est complète (ou l'a été
    /// et n'est pas entièrement relâchée), ou qu'un de ses boutons est seul depuis moins de `ATTENTE_COMBO`.
    fn boutons_utiles(&mut self, boutons: u16, t: Instant) -> u16 {
        if self.combo == 0 {
            return boutons;
        }
        let dans_combo = boutons & self.combo;
        if dans_combo == self.combo {
            self.combo_vue = true;
        }
        if dans_combo == 0 {
            // Un appui BREF (lâché avant la fin de l'attente) sur un bouton de la combinaison, sans que la combinaison
            // ait été faite : c'était bien ce bouton — sa touche est rendue maintenant (appuyée ce tour-ci, relâchée au
            // suivant), jamais perdue.
            let rendre = if !self.combo_vue && self.partiel_depuis.is_some_and(|d| t.duration_since(d) < ATTENTE_COMBO) { self.partiel_bits } else { 0 };
            self.combo_vue = false;
            self.partiel_depuis = None;
            self.partiel_bits = 0;
            return boutons | rendre;
        }
        if self.combo_vue {
            return boutons & !self.combo;
        }
        self.partiel_bits |= dans_combo;
        let depuis = *self.partiel_depuis.get_or_insert(t);
        if t.duration_since(depuis) < ATTENTE_COMBO {
            boutons & !self.combo
        } else {
            boutons
        }
    }

    /// Les touches à appuyer ou relâcher pour cet état de la manette, à l'instant `t`.
    pub fn pas(&mut self, e: &EtatManette, t: Instant) -> Vec<Frappe> {
        let boutons = self.boutons_utiles(e.boutons, t);
        // Une touche est voulue si UN de ses boutons est appuyé (deux boutons peuvent donner la même touche).
        let mut voulues: Vec<Touche> = Vec::new();
        for (b, touche) in &self.liens {
            if appuye(b, boutons, e) && !voulues.contains(touche) {
                voulues.push(*touche);
            }
        }
        let frappe = |t: &Touche, bas: bool| Frappe { vk: t.0, position: t.1, etendue: t.2, bas, repetition: false };
        let mut f: Vec<Frappe> = self.tenues.iter().filter(|t| !voulues.contains(t)).map(|t| frappe(t, false)).collect();
        let nouvelles: Vec<Touche> = voulues.iter().filter(|v| !self.tenues.contains(v)).copied().collect();
        f.extend(nouvelles.iter().map(|v| frappe(v, true)));
        if let Some(r) = self.repetition {
            // La dernière touche appuyée (hors Maj, Ctrl, Alt) se répétera après le délai.
            if let Some(n) = nouvelles.iter().rev().find(|n| se_repete(n)) {
                self.derniere = Some((*n, t + r.delai));
            }
            // Relâchée : plus de répétition (les autres touches tenues ne reprennent pas, comme un vrai clavier).
            if self.derniere.is_some_and(|(d, _)| !voulues.contains(&d)) {
                self.derniere = None;
            }
            // Échue : UNE répétition par tour au plus (jamais de rafale après un trou, Frogtend occupé…).
            if let Some((d, echeance)) = self.derniere {
                if t >= echeance {
                    f.push(Frappe { repetition: true, ..frappe(&d, true) });
                    let suivante = echeance + r.periode;
                    self.derniere = Some((d, if suivante > t { suivante } else { t + r.periode }));
                }
            }
        }
        self.tenues = voulues;
        f
    }

    /// Relâche tout ce qu'il tient (menu qui s'ouvre, jeu qui perd le premier plan, fin de partie).
    pub fn relacher_tout(&mut self) -> Vec<Frappe> {
        self.derniere = None;
        std::mem::take(&mut self.tenues).into_iter().map(|t| Frappe { vk: t.0, position: t.1, etendue: t.2, bas: false, repetition: false }).collect()
    }
}

type IdTouche = (u16, u16, bool);

/// Ce que Frogtend tient VRAIMENT enfoncé dans Windows : le code envoyé à l'appui de chaque touche (pour relâcher ce
/// code-là, même si la disposition du clavier a changé entre-temps), et un compte par code (deux noms de touche peuvent
/// tomber sur la même touche physique — « Z » et « Déplacement haut » en AZERTY : le code n'est relâché qu'au dernier).
#[derive(Default)]
pub struct Enfoncees {
    par_touche: Vec<(IdTouche, u16)>,
    comptes: Vec<((u16, bool), u32)>,
}

impl Enfoncees {
    /// Les codes à envoyer pour ces frappes (`code_de` : le code d'une touche virtuelle dans la disposition actuelle ;
    /// 0 = aucun) : (code, étendue, appuyé).
    pub fn planifier(&mut self, frappes: &[Frappe], code_de: &dyn Fn(u16) -> u16) -> Vec<(u16, bool, bool)> {
        let mut l = Vec::new();
        for f in frappes {
            let id = (f.vk, f.position, f.etendue);
            if f.repetition {
                // Un nouvel appui du code DÉJÀ tenu, sans rien compter ; jamais pour une touche qui n'est plus tenue
                // (ce serait un appui sans relâchement).
                if let Some((_, code)) = self.par_touche.iter().find(|(i, _)| *i == id).filter(|_| f.bas) {
                    l.push((*code, f.etendue, true));
                }
                continue;
            }
            if f.bas {
                if self.par_touche.iter().any(|(i, _)| *i == id) {
                    continue;
                }
                let code = if f.position != 0 { f.position } else { code_de(f.vk) };
                if code == 0 {
                    continue; // une touche sans code dans cette disposition : jamais un code 0
                }
                self.par_touche.push((id, code));
                let cle = (code, f.etendue);
                match self.comptes.iter_mut().find(|(c, _)| *c == cle) {
                    Some((_, n)) => *n += 1,
                    None => {
                        self.comptes.push((cle, 1));
                        l.push((code, f.etendue, true));
                    }
                }
            } else {
                let Some(i) = self.par_touche.iter().position(|(x, _)| *x == id) else { continue };
                let (_, code) = self.par_touche.remove(i);
                let cle = (code, f.etendue);
                if let Some(j) = self.comptes.iter().position(|(c, _)| *c == cle) {
                    self.comptes[j].1 -= 1;
                    if self.comptes[j].1 == 0 {
                        self.comptes.remove(j);
                        l.push((code, f.etendue, false));
                    }
                }
            }
        }
        l
    }

    /// Tout relâcher (le crochet de panique).
    pub fn tout(&mut self) -> Vec<(u16, bool, bool)> {
        self.par_touche.clear();
        self.comptes.drain(..).map(|((c, x), _)| (c, x, false)).collect()
    }
}

static ENFONCEES: Mutex<Enfoncees> = Mutex::new(Enfoncees { par_touche: Vec::new(), comptes: Vec::new() });

/// Installe le filet : si Frogtend plante, les touches qu'il tient sont relâchées avant l'arrêt.
pub fn initialiser() {
    let precedent = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Ok(mut e) = ENFONCEES.try_lock() {
            let haut = e.tout();
            #[cfg(windows)]
            windows_impl::envoyer_scans(&haut);
            #[cfg(not(windows))]
            let _ = haut;
        }
        precedent(info);
    }));
}

/// Envoie les frappes au premier plan actuel. Rend (codes à envoyer, codes acceptés par Windows) : un écart = un refus
/// (bureau sécurisé, jeu en administrateur).
pub fn envoyer(frappes: &[Frappe]) -> (usize, usize) {
    #[cfg(windows)]
    {
        windows_impl::envoyer(frappes)
    }
    #[cfg(not(windows))]
    {
        let _ = frappes;
        (0, 0)
    }
}

/// Le premier plan, vu par le moteur : est-il au jeu ? Vérifié à CHAQUE envoi (la fenêtre et son processus) ; le
/// chemin du programme n'est relu que quand le processus au premier plan change.
#[derive(Default)]
pub struct PremierPlan {
    dernier_pid: u32,
    au_jeu: bool,
    eleve: bool,
}

impl PremierPlan {
    /// Le jeu (un programme de `dossier`) a-t-il le premier plan ? Et son processus est-il en administrateur (alors
    /// Windows bloque les frappes de Frogtend, sans rien dire) ?
    pub fn au_jeu(&mut self, dossier: &std::path::Path) -> (bool, bool) {
        #[cfg(windows)]
        {
            let pid = windows_impl::pid_au_premier_plan();
            if pid != self.dernier_pid {
                self.dernier_pid = pid;
                self.au_jeu = pid != 0 && pid != std::process::id() && windows_impl::programme_dans(pid, dossier);
                self.eleve = self.au_jeu && windows_impl::eleve(pid) && !windows_impl::eleve(std::process::id());
            }
            (self.au_jeu, self.eleve)
        }
        #[cfg(not(windows))]
        {
            let _ = dossier;
            (false, false)
        }
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::{Frappe, ENFONCEES};
    use windows::core::PWSTR;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    use windows::Win32::System::Threading::{OpenProcess, OpenProcessToken, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetKeyboardLayout, MapVirtualKeyExW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_EXTENDEDKEY,
        KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, MAPVK_VK_TO_VSC, VIRTUAL_KEY,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    fn entree(scan: u16, etendue: bool, bas: bool) -> INPUT {
        let mut d: KEYBD_EVENT_FLAGS = KEYEVENTF_SCANCODE;
        if etendue {
            d |= KEYEVENTF_EXTENDEDKEY;
        }
        if !bas {
            d |= KEYEVENTF_KEYUP;
        }
        INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VIRTUAL_KEY(0), wScan: scan, dwFlags: d, time: 0, dwExtraInfo: 0 } } }
    }

    /// Des codes de balayage bruts (le crochet de panique).
    pub fn envoyer_scans(l: &[(u16, bool, bool)]) {
        let e: Vec<INPUT> = l.iter().map(|(s, x, b)| entree(*s, *x, *b)).collect();
        if !e.is_empty() {
            unsafe {
                SendInput(&e, std::mem::size_of::<INPUT>() as i32);
            }
        }
    }

    pub fn envoyer(frappes: &[Frappe]) -> (usize, usize) {
        // La disposition du clavier DU JEU au premier plan (AZERTY : la touche marquée « Z ») : le jeu reçoit la même
        // chose que si la personne appuyait elle-même. (Un relâchement réutilise le code de son appui.)
        let disposition = unsafe {
            let fil = GetWindowThreadProcessId(GetForegroundWindow(), None);
            GetKeyboardLayout(fil)
        };
        let code_de = |vk: u16| (unsafe { MapVirtualKeyExW(vk as u32, MAPVK_VK_TO_VSC, Some(disposition)) }) as u16;
        let l = match ENFONCEES.lock() {
            Ok(mut e) => e.planifier(frappes, &code_de),
            Err(_) => return (0, 0),
        };
        if l.is_empty() {
            return (0, 0);
        }
        let e: Vec<INPUT> = l.iter().map(|(s, x, b)| entree(*s, *x, *b)).collect();
        let n = unsafe { SendInput(&e, std::mem::size_of::<INPUT>() as i32) } as usize;
        (l.len(), n)
    }

    /// Le délai (0 à 3) et la vitesse (0 à 31) de répétition du clavier réglés dans Windows.
    pub fn reglages_clavier() -> Option<(u32, u32)> {
        use windows::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETKEYBOARDDELAY, SPI_GETKEYBOARDSPEED, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS};
        let mut delai = 0u32;
        let mut vitesse = 0u32;
        unsafe {
            SystemParametersInfoW(SPI_GETKEYBOARDDELAY, 0, Some(&mut delai as *mut u32 as *mut _), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0)).ok()?;
            SystemParametersInfoW(SPI_GETKEYBOARDSPEED, 0, Some(&mut vitesse as *mut u32 as *mut _), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0)).ok()?;
        }
        Some((delai, vitesse))
    }

    pub fn pid_au_premier_plan() -> u32 {
        let mut pid = 0u32;
        unsafe {
            GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut pid));
        }
        pid
    }

    pub fn programme_dans(pid: u32, dossier: &std::path::Path) -> bool {
        let d = dossier.to_string_lossy().trim_end_matches(['\\', '/']).to_lowercase();
        if d.is_empty() || dossier.parent().is_none() {
            return false;
        }
        unsafe {
            let Ok(p) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return false };
            let mut b = [0u16; 1024];
            let mut n = b.len() as u32;
            let ok = QueryFullProcessImageNameW(p, PROCESS_NAME_WIN32, PWSTR(b.as_mut_ptr()), &mut n).is_ok();
            let _ = CloseHandle(p);
            ok && String::from_utf16_lossy(&b[..n as usize]).to_lowercase().replace('/', "\\").starts_with(&format!("{d}\\"))
        }
    }

    /// Le processus tourne-t-il en administrateur ? (Un refus de lecture compte comme « oui ».)
    pub fn eleve(pid: u32) -> bool {
        unsafe {
            let Ok(p) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return true };
            let mut jeton = HANDLE::default();
            let r = if OpenProcessToken(p, TOKEN_QUERY, &mut jeton).is_ok() {
                let mut el = TOKEN_ELEVATION::default();
                let mut n = 0u32;
                let ok = GetTokenInformation(jeton, TokenElevation, Some(&mut el as *mut _ as *mut core::ffi::c_void), std::mem::size_of::<TOKEN_ELEVATION>() as u32, &mut n).is_ok();
                let _ = CloseHandle(jeton);
                !ok || el.TokenIsElevated != 0
            } else {
                true
            };
            let _ = CloseHandle(p);
            r
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corr(l: &[(&str, &str)]) -> BTreeMap<String, String> {
        l.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
    }
    fn e(boutons: u16) -> EtatManette {
        EtatManette { boutons, ..Default::default() }
    }
    fn f(vk: u16, bas: bool) -> Frappe {
        Frappe { vk, position: 0, etendue: false, bas, repetition: false }
    }

    #[test]
    fn un_bouton_appuye_puis_relache_donne_une_touche_appuyee_puis_relachee() {
        let t = Instant::now();
        let mut m = Moteur::nouveau(&corr(&[("a", "Z"), ("x", "X"), ("haut", "Haut")]), 0);
        assert_eq!(m.pas(&e(0x1000), t), [f(0x5A, true)]);
        assert!(m.pas(&e(0x1000), t).is_empty(), "tenu : rien de plus");
        assert_eq!(m.pas(&e(0), t), [f(0x5A, false)]);
        // Le stick gauche vers le haut = la flèche (touche étendue).
        let haut = EtatManette { ly: 30_000, ..Default::default() };
        assert_eq!(m.pas(&haut, t), [Frappe { vk: 0x26, position: 0, etendue: true, bas: true, repetition: false }]);
        assert_eq!(m.relacher_tout(), [Frappe { vk: 0x26, position: 0, etendue: true, bas: false, repetition: false }]);
        assert!(m.relacher_tout().is_empty());
    }

    #[test]
    fn une_touche_tenue_se_repete_comme_un_vrai_clavier() {
        let t0 = Instant::now();
        let ms = |n: u64| t0 + Duration::from_millis(n);
        let r = Repetition { delai: Duration::from_millis(500), periode: Duration::from_millis(50) };
        let rep = |vk: u16| Frappe { repetition: true, ..f(vk, true) };
        let mut m = Moteur::nouveau(&corr(&[("a", "Z"), ("x", "X"), ("rt", "Maj"), ("select", "Échap"), ("rb", "S")]), 0x0220).avec_repetition(Some(r));
        // Rien avant le délai, puis une répétition à chaque période.
        assert_eq!(m.pas(&e(0x1000), ms(0)), [f(0x5A, true)]);
        assert!(m.pas(&e(0x1000), ms(499)).is_empty());
        assert_eq!(m.pas(&e(0x1000), ms(500)), [rep(0x5A)]);
        assert!(m.pas(&e(0x1000), ms(530)).is_empty());
        assert_eq!(m.pas(&e(0x1000), ms(550)), [rep(0x5A)]);
        // Un trou de 2 s (Frogtend occupé) : UNE seule répétition, jamais une rafale.
        assert_eq!(m.pas(&e(0x1000), ms(2550)), [rep(0x5A)]);
        assert!(m.pas(&e(0x1000), ms(2560)).is_empty());
        // Une deuxième touche : c'est elle qui se répète ; la première ne reprend pas quand on lâche la seconde.
        assert_eq!(m.pas(&e(0x1000 | 0x4000), ms(3000)), [f(0x58, true)]);
        assert_eq!(m.pas(&e(0x1000 | 0x4000), ms(3500)), [rep(0x58)]);
        assert_eq!(m.pas(&e(0x1000), ms(3600)), [f(0x58, false)]);
        assert!(m.pas(&e(0x1000), ms(5000)).is_empty());
        // Lâchée : plus rien ; rappuyée : le délai repart de zéro.
        assert_eq!(m.pas(&e(0), ms(5100)), [f(0x5A, false)]);
        assert_eq!(m.pas(&e(0x1000), ms(5200)), [f(0x5A, true)]);
        assert!(m.pas(&e(0x1000), ms(5600)).is_empty());
        // Le menu qui s'ouvre relâche tout : aucune répétition ensuite, et le délai repart.
        m.relacher_tout();
        assert_eq!(m.pas(&e(0x1000), ms(6000)), [f(0x5A, true)]);
        assert!(m.pas(&e(0x1000), ms(6400)).is_empty());
        m.relacher_tout();
        // Maj tenue ne se répète pas.
        assert_eq!(m.pas(&EtatManette { dt: 255, ..Default::default() }, ms(7000)), [f(0xA0, true)]);
        assert!(m.pas(&EtatManette { dt: 255, ..Default::default() }, ms(9000)).is_empty());
        m.relacher_tout();
        // Les boutons de la combinaison du menu (View + RB) ne partent jamais, donc ne se répètent jamais.
        assert!(m.pas(&e(0x0220), ms(10_000)).is_empty());
        assert!(m.pas(&e(0x0220), ms(12_000)).is_empty());
        // Coupée pour ce jeu : jamais de répétition.
        let mut sans = Moteur::nouveau(&corr(&[("a", "Z")]), 0);
        assert_eq!(sans.pas(&e(0x1000), ms(0)), [f(0x5A, true)]);
        assert!(sans.pas(&e(0x1000), ms(5000)).is_empty());
    }

    #[test]
    fn une_repetition_ne_change_pas_ce_qui_est_tenu() {
        let mut en = Enfoncees::default();
        let code_de = |vk: u16| vk + 0x100;
        let z = f(0x5A, true);
        let rep = Frappe { repetition: true, ..z };
        assert_eq!(en.planifier(&[z], &code_de), [(0x15A, false, true)]);
        assert_eq!(en.planifier(&[rep, rep], &code_de), [(0x15A, false, true), (0x15A, false, true)]);
        // Un seul relâchement suffit (les répétitions n'ont rien compté).
        assert_eq!(en.planifier(&[f(0x5A, false)], &code_de), [(0x15A, false, false)]);
        // Une répétition d'une touche qui n'est plus tenue : rien (jamais un appui sans relâchement).
        assert!(en.planifier(&[rep], &code_de).is_empty());
    }

    #[test]
    fn le_delai_et_la_cadence_viennent_de_windows() {
        assert_eq!(Repetition::de_windows(0, 31).delai, Duration::from_millis(250));
        assert_eq!(Repetition::de_windows(3, 31).delai, Duration::from_millis(1000));
        assert_eq!(Repetition::de_windows(9, 31).delai, Duration::from_millis(1000), "hors bornes : le maximum");
        let vite = Repetition::de_windows(1, 31).periode.as_millis();
        assert!((33..=34).contains(&vite), "{vite}");
        assert_eq!(Repetition::de_windows(1, 0).periode.as_millis(), 400);
    }

    #[test]
    fn deux_boutons_pour_une_touche_les_gachettes_et_les_positions() {
        let t = Instant::now();
        let mut m = Moteur::nouveau(&corr(&[("haut", "Haut"), ("rt", "Maj"), ("lb", "A"), ("rb", "A"), ("gauche", "Déplacement gauche")]), 0);
        let les_deux = EtatManette { boutons: 0x0001, ly: 30_000, ..Default::default() };
        assert_eq!(m.pas(&les_deux, t).len(), 1);
        assert!(m.pas(&EtatManette { ly: 30_000, ..Default::default() }, t).is_empty(), "le stick tient encore la flèche");
        assert_eq!(m.pas(&e(0), t).len(), 1);
        assert_eq!(m.pas(&EtatManette { dt: 200, ..Default::default() }, t), [f(0xA0, true)]);
        assert_eq!(m.pas(&EtatManette { dt: 50, ..Default::default() }, t).len(), 1, "gâchette lâchée sous la mi-course");
        assert_eq!(m.pas(&e(0x0300), t).len(), 1, "LB et RB sur la même touche : une seule frappe");
        m.relacher_tout();
        // « Déplacement gauche » part par sa POSITION (celle de Q en AZERTY, A en QWERTY).
        assert_eq!(m.pas(&e(0x0004), t), [Frappe { vk: 0, position: 0x1E, etendue: false, bas: true, repetition: false }]);
    }

    #[test]
    fn la_combinaison_du_menu_n_envoie_rien_au_jeu_meme_appuyee_en_deux_temps() {
        // Back + RB (0x0020 | 0x0200).
        let t0 = Instant::now();
        let ms = |n: u64| t0 + Duration::from_millis(n);
        let mut m = Moteur::nouveau(&corr(&[("select", "Échap"), ("rb", "S"), ("a", "Z")]), 0x0220);
        // RB d'abord, Back 50 ms après : rien ne part ; A, lui, part.
        assert!(m.pas(&e(0x0200), ms(0)).is_empty());
        assert!(m.pas(&e(0x0220), ms(50)).is_empty());
        assert_eq!(m.pas(&e(0x0220 | 0x1000), ms(60)), [f(0x5A, true)]);
        // Relâchés dans le désordre : toujours rien, jusqu'à ce que TOUS soient lâchés.
        assert_eq!(m.pas(&e(0x0200), ms(500)), [f(0x5A, false)]);
        assert!(m.pas(&e(0x0200), ms(900)).is_empty());
        assert!(m.pas(&e(0), ms(950)).is_empty());
        // RB seul, tenu plus de 150 ms : c'est bien RB, sa touche part.
        assert!(m.pas(&e(0x0200), ms(1000)).is_empty());
        assert_eq!(m.pas(&e(0x0200), ms(1200)), [f(0x53, true)]);
        assert_eq!(m.pas(&e(0), ms(1300)), [f(0x53, false)]);
        // RB appuyé BREVEMENT (60 ms) : sa touche part au relâchement, puis se relâche au tour suivant.
        assert!(m.pas(&e(0x0200), ms(2000)).is_empty());
        assert_eq!(m.pas(&e(0), ms(2060)), [f(0x53, true)]);
        assert_eq!(m.pas(&e(0), ms(2076)), [f(0x53, false)]);
    }

    #[test]
    fn deux_noms_sur_la_meme_touche_physique_ne_relachent_qu_au_dernier() {
        // AZERTY : « Z » (touche virtuelle 0x5A → code 0x11) et « Déplacement haut » (position 0x11).
        let code_de = |vk: u16| if vk == 0x5A { 0x11 } else { 0 };
        let z = Frappe { vk: 0x5A, position: 0, etendue: false, bas: true, repetition: false };
        let dep = Frappe { vk: 0, position: 0x11, etendue: false, bas: true, repetition: false };
        let mut e = Enfoncees::default();
        assert_eq!(e.planifier(&[z], &code_de), [(0x11, false, true)]);
        assert!(e.planifier(&[dep], &code_de).is_empty(), "déjà enfoncée : pas un 2e appui");
        assert!(e.planifier(&[Frappe { bas: false, ..z }], &code_de).is_empty(), "l'autre la tient encore");
        assert_eq!(e.planifier(&[Frappe { bas: false, ..dep }], &code_de), [(0x11, false, false)]);
        // Le relâchement réutilise le code de l'appui, même si la disposition a changé entre-temps.
        assert_eq!(e.planifier(&[z], &code_de), [(0x11, false, true)]);
        let autre_disposition = |_: u16| 0x2C;
        assert_eq!(e.planifier(&[Frappe { bas: false, ..z }], &autre_disposition), [(0x11, false, false)]);
        // Une touche sans code n'est jamais envoyée ; le crochet de panique relâche tout ce qui reste.
        assert!(e.planifier(&[Frappe { vk: 0x99, position: 0, etendue: false, bas: true, repetition: false }], &code_de).is_empty());
        e.planifier(&[dep], &code_de);
        assert_eq!(e.tout(), [(0x11, false, false)]);
    }

    #[test]
    fn un_nom_inconnu_est_ignore_et_l_interface_propose_les_memes_noms() {
        let m = Moteur::nouveau(&corr(&[("a", "Inconnue"), ("bouton_x", "Z")]), 0);
        assert!(m.est_vide());
        let ts = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../src/lib/ludotheque/clavier.ts")).unwrap();
        for (t, ..) in TOUCHES {
            assert!(ts.contains(&format!("'{t}'")), "touche {t} absente de l'interface");
        }
        for (b, _) in BOUTONS {
            assert!(ts.contains(&format!("id: '{b}'")), "bouton {b} absent de l'interface");
        }
    }

    #[cfg(windows)]
    #[test]
    fn chaque_touche_a_un_code_dans_la_disposition_du_pc() {
        use windows::Win32::UI::Input::KeyboardAndMouse::{MapVirtualKeyW, MAPVK_VK_TO_VSC};
        for (nom, vk, _, pos) in TOUCHES {
            let code = if *pos != 0 { *pos as u32 } else { unsafe { MapVirtualKeyW(*vk as u32, MAPVK_VK_TO_VSC) } };
            assert_ne!(code, 0, "{nom} n'a pas de code");
        }
    }
}

#[cfg(all(test, windows))]
mod essai_reel {
    /// RÉEL : une fenêtre d'essai (PowerShell) avec une zone de texte reçoit les frappes du moteur : des LETTRES Z Q S D,
    /// puis les quatre POSITIONS « Déplacement » ; elle écrit ce qu'elle a reçu dans un fichier.
    /// `cargo test essai_frappes_reelles -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn essai_frappes_reelles() {
        let d = tempfile::tempdir().unwrap();
        let sortie = d.path().join("recu.txt");
        let script = format!(
            "Add-Type -AssemblyName System.Windows.Forms; $f = New-Object Windows.Forms.Form; $f.Text = 'Frogtend essai clavier'; \
             $t = New-Object Windows.Forms.TextBox; $t.Dock = 'Fill'; $f.Controls.Add($t); \
             $m = New-Object Windows.Forms.Timer; $m.Interval = 6000; $m.Add_Tick({{ Set-Content -Path '{}' -Value $t.Text -Encoding UTF8; $f.Close() }}); $m.Start(); \
             $f.Add_Shown({{ $f.Activate(); $t.Focus() }}); $f.ShowDialog()",
            sortie.display()
        );
        let mut enfant = std::process::Command::new("powershell").args(["-NoProfile", "-Command", &script]).spawn().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2500));
        let corr: std::collections::BTreeMap<String, String> = [
            ("a", "Z"),
            ("x", "Q"),
            ("y", "S"),
            ("b", "D"),
            ("haut", "Déplacement haut"),
            ("gauche", "Déplacement gauche"),
            ("bas", "Déplacement bas"),
            ("droite", "Déplacement droite"),
        ]
        .iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();
        let mut m = super::Moteur::nouveau(&corr, 0);
        let t = std::time::Instant::now();
        for b in [0x1000u16, 0x4000, 0x8000, 0x2000, 0x0001, 0x0004, 0x0002, 0x0008] {
            super::envoyer(&m.pas(&crate::osd_manette::EtatManette { boutons: b, ..Default::default() }, t));
            std::thread::sleep(std::time::Duration::from_millis(60));
            super::envoyer(&m.pas(&crate::osd_manette::EtatManette::default(), t));
            std::thread::sleep(std::time::Duration::from_millis(60));
        }
        let _ = enfant.wait();
        let recu = std::fs::read_to_string(&sortie).unwrap_or_default();
        let recu = recu.trim_start_matches('\u{feff}').trim().to_lowercase();
        println!("RECU {recu:?}");
        // AZERTY : les lettres Z Q S D, puis les positions de ZQSD (= WASD en QWERTY), qui donnent aussi « zqsd ».
        assert_eq!(recu, "zqsdzqsd");
    }

    /// Une touche TENUE 1,5 s se répète dans une vraie fenêtre, au délai et à la cadence du clavier de Windows. Rien
    /// n'est envoyé si la fenêtre d'essai n'a plus le premier plan (jamais une touche ailleurs).
    #[test]
    #[ignore]
    fn essai_repetition_reelle() {
        let d = tempfile::tempdir().unwrap();
        let sortie = d.path().join("recu.txt");
        let script = format!(
            "Add-Type -AssemblyName System.Windows.Forms; $f = New-Object Windows.Forms.Form; $f.Text = 'Frogtend essai répétition'; \
             $t = New-Object Windows.Forms.TextBox; $t.Dock = 'Fill'; $f.Controls.Add($t); \
             $m = New-Object Windows.Forms.Timer; $m.Interval = 6000; $m.Add_Tick({{ Set-Content -Path '{}' -Value $t.Text -Encoding UTF8; $f.Close() }}); $m.Start(); \
             $f.Add_Shown({{ $f.Activate(); $t.Focus() }}); $f.ShowDialog()",
            sortie.display()
        );
        let mut enfant = std::process::Command::new("powershell").args(["-NoProfile", "-Command", &script]).spawn().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2500));
        let r = super::Repetition::de_ce_pc();
        println!("RÉPÉTITION {r:?}");
        let corr: std::collections::BTreeMap<String, String> = [("a".to_string(), "Z".to_string())].into_iter().collect();
        let mut m = super::Moteur::nouveau(&corr, 0).avec_repetition(Some(r));
        let a = crate::osd_manette::EtatManette { boutons: 0x1000, ..Default::default() };
        let debut = std::time::Instant::now();
        let mut envoyes = 0;
        while debut.elapsed() < std::time::Duration::from_millis(1500) {
            if super::windows_impl::pid_au_premier_plan() != enfant.id() {
                super::envoyer(&m.relacher_tout());
                panic!("la fenêtre d'essai n'a plus le premier plan : arrêté, rien envoyé ailleurs");
            }
            let f = m.pas(&a, std::time::Instant::now());
            envoyes += f.len();
            super::envoyer(&f);
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
        super::envoyer(&m.relacher_tout());
        let _ = enfant.wait();
        let recu = std::fs::read_to_string(&sortie).unwrap_or_default();
        let recu = recu.trim_start_matches('\u{feff}').trim().to_lowercase();
        // Attendu : 1 appui + (1500 ms - délai) / période répétitions, à un ou deux près (le fil tourne par pas de 16 ms).
        let attendu = 1 + (1500u128.saturating_sub(r.delai.as_millis())) / r.periode.as_millis().max(1);
        println!("REÇU {} « z » (frappes envoyées {envoyes}, attendu ≈ {attendu})", recu.len());
        assert!(recu.chars().all(|c| c == 'z'), "{recu:?}");
        assert!(recu.len() as u128 + 3 >= attendu && recu.len() as u128 <= attendu + 1, "{} pour ≈ {attendu}", recu.len());
    }
}
