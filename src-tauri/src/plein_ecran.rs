//! Le plein écran au lancement d'un jeu PC (Seb, 16/10 : « ce n'est qu'un jeu parmi d'autres, il faut que notre
//! création soit le plus généraliste possible »). Un réglage PAR JEU, désactivé d'office :
//! - `touche:<nom>` : Frogtend appuie sur la touche plein écran du jeu (F4, F11, Alt+Entrée…) quand sa fenêtre est
//!   prête — le jeu passe en plein écran à SA façon (mise à l'échelle comprise) ;
//! - `fenetre` : Frogtend agrandit la fenêtre du jeu à tout l'écran, sans bordure (marche avec presque tous les jeux
//!   fenêtrés ; le jeu choisit lui-même s'il étire son image).
//!
//! La touche peut être SUGGÉRÉE en lisant les textes du jeu (ReadMe, options) : une ligne qui parle de plein écran ET
//! nomme une touche (relevé sur les jeux de Seb le 16/10 : « F4-Toggle Fullscreen Mode »,
//! « key_key.fullscreen:key.keyboard.f11 »).

use std::path::Path;

/// Les touches plein écran proposées (nom montré, code de balayage, touche étendue, avec Alt).
pub const TOUCHES: &[(&str, u16, bool, bool)] = &[
    ("F1", 0x3B, false, false),
    ("F2", 0x3C, false, false),
    ("F3", 0x3D, false, false),
    ("F4", 0x3E, false, false),
    ("F5", 0x3F, false, false),
    ("F6", 0x40, false, false),
    ("F7", 0x41, false, false),
    ("F8", 0x42, false, false),
    ("F9", 0x43, false, false),
    ("F10", 0x44, false, false),
    ("F11", 0x57, false, false),
    ("F12", 0x58, false, false),
    ("Alt+Entrée", 0x1C, false, true),
];

/// Le mode de plein écran d'un jeu, lu depuis son réglage (`aucun`, `fenetre`, `touche:F4`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Aucun,
    Fenetre,
    Touche(&'static str),
}

pub fn mode(reglage: &str) -> Mode {
    match reglage.trim() {
        "fenetre" => Mode::Fenetre,
        r => match r.strip_prefix("touche:") {
            Some(n) => TOUCHES.iter().find(|t| t.0 == n).map_or(Mode::Aucun, |t| Mode::Touche(t.0)),
            None => Mode::Aucun,
        },
    }
}

/// Un texte lu tel quel : UTF-16 (avec sa marque), sinon UTF-8, sinon Windows-1252 approché (latin-1).
fn lire_texte(octets: &[u8]) -> String {
    if octets.len() >= 2 && octets[..2] == [0xFF, 0xFE] {
        let u: Vec<u16> = octets[2..].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        return String::from_utf16_lossy(&u);
    }
    if octets.len() >= 2 && octets[..2] == [0xFE, 0xFF] {
        let u: Vec<u16> = octets[2..].chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
        return String::from_utf16_lossy(&u);
    }
    match std::str::from_utf8(octets) {
        Ok(t) => t.to_string(),
        Err(_) => octets.iter().map(|&b| b as char).collect(),
    }
}

/// « alt+ », « ctrl+ », « shift+ » juste avant (« Alt+F4 quitte » : ce n'est pas la touche du plein écran).
fn precede_d_un_modificateur(avant: &str) -> bool {
    let a = avant.trim_end();
    ["alt+", "alt +", "ctrl+", "ctrl +", "shift+", "maj+"].iter().any(|m| a.ends_with(m.trim_end()))
}

/// La touche plein écran nommée dans UNE ligne qui parle de plein écran, s'il y en a une.
pub fn touche_dans_ligne(ligne: &str) -> Option<&'static str> {
    let l = ligne.to_lowercase();
    let parle = ["fullscreen", "full screen", "full-screen", "plein écran", "plein ecran", "plein-écran"].iter().any(|m| l.contains(m));
    if !parle {
        return None;
    }
    if ["alt+enter", "alt + enter", "alt-enter", "alt+entrée", "alt + entrée", "alt+return"].iter().any(|m| l.contains(m)) {
        return Some("Alt+Entrée");
    }
    // F1 à F12, comme mot entier (« F4- », « keyboard.f11 »), jamais « f » seul ni « f4x ».
    let octets = l.as_bytes();
    let mut i = 0;
    while i < octets.len() {
        if octets[i] == b'f' && (i == 0 || !octets[i - 1].is_ascii_alphanumeric()) && !precede_d_un_modificateur(&l[..i]) {
            let chiffres: String = l[i + 1..].chars().take_while(|c| c.is_ascii_digit()).collect();
            let apres = l[i + 1 + chiffres.len()..].chars().next();
            if !chiffres.is_empty() && !apres.is_some_and(|c| c.is_ascii_alphanumeric()) {
                let nom = format!("F{chiffres}");
                if let Some(t) = TOUCHES.iter().find(|t| t.0 == nom) {
                    return Some(t.0);
                }
            }
        }
        i += 1;
    }
    None
}

/// La touche plein écran suggérée par les textes du jeu (`.txt`, `.ini`, `.cfg` de 1 Mo au plus, à deux niveaux).
pub fn suggestion(dossier: &Path) -> Option<&'static str> {
    let mut a_voir = vec![(dossier.to_path_buf(), 0usize)];
    let mut lus = 0usize;
    while let Some((d, profondeur)) = a_voir.pop() {
        let Ok(entrees) = std::fs::read_dir(&d) else { continue };
        for e in entrees.flatten() {
            let p = e.path();
            let Ok(t) = e.file_type() else { continue };
            if t.is_dir() {
                if profondeur < 2 {
                    a_voir.push((p, profondeur + 1));
                }
                continue;
            }
            let ext = p.extension().and_then(|x| x.to_str()).map(str::to_lowercase).unwrap_or_default();
            if !matches!(ext.as_str(), "txt" | "ini" | "cfg") || e.metadata().map(|m| m.len()).unwrap_or(u64::MAX) > 1_000_000 {
                continue;
            }
            lus += 1;
            if lus > 200 {
                return None;
            }
            let Ok(octets) = std::fs::read(&p) else { continue };
            if let Some(k) = lire_texte(&octets).lines().find_map(touche_dans_ligne) {
                return Some(k);
            }
        }
    }
    None
}

/// Où en est la partie, pour le plein écran : on n'agit que quand elle est LIBRE (ni menu ouvert, ni jeu figé).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Etat {
    Libre,
    /// Menu en jeu ouvert ou jeu figé : on attend (la stabilité repart de zéro).
    Attendre,
    /// La partie est finie : on abandonne.
    Finie,
}

/// Une fenêtre couvre-t-elle déjà tout son écran ? (Le jeu est alors en plein écran : on ne fait RIEN — une touche
/// qui bascule le remettrait en fenêtre.) Rectangles (gauche, haut, droite, bas).
pub fn couvre_l_ecran(fenetre: (i32, i32, i32, i32), ecran: (i32, i32, i32, i32)) -> bool {
    fenetre.0 <= ecran.0 && fenetre.1 <= ecran.1 && fenetre.2 >= ecran.2 && fenetre.3 >= ecran.3
}

/// Applique le mode aux fenêtres du jeu — SEULEMENT celles des programmes de son dossier (jamais l'explorateur, une
/// boutique, le Bloc-notes ouvert par un lanceur) — pendant 90 s : chaque NOUVELLE fenêtre principale qui devient la
/// plus grande et reste stable 2 s est traitée (un lanceur, puis la vraie fenêtre du jeu), 3 fois au plus ; une
/// fenêtre qui couvre déjà son écran n'est pas touchée. `etat` dit si la partie est libre. Rend ce qui a été fait.
pub fn appliquer(dossier_jeu: &Path, m: &Mode, etat: &dyn Fn() -> Etat) -> Result<String, String> {
    if *m == Mode::Aucun {
        return Ok("rien".into());
    }
    let d = dossier_jeu.to_path_buf();
    appliquer_pids(&move || crate::lancement::processus_du_dossier(&d), m, etat, std::time::Duration::from_secs(90))
}

/// [`appliquer`], les programmes du jeu donnés par `pids` (pour l'essai réel sur une fenêtre de test).
pub fn appliquer_pids(pids: &dyn Fn() -> Vec<u32>, m: &Mode, etat: &dyn Fn() -> Etat, duree: std::time::Duration) -> Result<String, String> {
    #[cfg(windows)]
    {
        windows_impl::appliquer(pids, m, etat, duree)
    }
    #[cfg(not(windows))]
    {
        let _ = (pids, m, etat, duree);
        Err("Windows seulement".into())
    }
}

/// Une fenêtre du jeu est-elle à l'écran ? (La plus grande fenêtre principale visible de ses programmes, au moins
/// 200 × 150, jamais une console — la règle du plein écran.) Pour l'écran de lancement (lot 4c) : lecture seule.
pub fn fenetre_visible(pids: &[u32]) -> bool {
    #[cfg(windows)]
    {
        windows_impl::fenetre_du_jeu(pids).is_some()
    }
    #[cfg(not(windows))]
    {
        let _ = pids;
        false
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::{couvre_l_ecran, Etat, Mode, TOUCHES};
    use std::time::{Duration, Instant};
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST};
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, GetForegroundWindow, GetWindowLongPtrW, GetWindowRect, IsHungAppWindow, IsIconic, IsWindow, IsZoomed, SetForegroundWindow,
        SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE, GWL_STYLE, HWND_TOP, SWP_ASYNCWINDOWPOS, SWP_FRAMECHANGED, SWP_NOACTIVATE,
        SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE, SWP_NOZORDER, SW_RESTORE, WS_CAPTION, WS_EX_CLIENTEDGE, WS_EX_DLGMODALFRAME, WS_EX_WINDOWEDGE, WS_MAXIMIZEBOX, WS_MINIMIZEBOX,
        WS_SYSMENU, WS_THICKFRAME,
    };

    fn classe(h: HWND) -> String {
        let mut b = [0u16; 128];
        let n = unsafe { GetClassNameW(h, &mut b) };
        String::from_utf16_lossy(&b[..n.max(0) as usize])
    }

    fn rect(h: HWND) -> Option<(i32, i32, i32, i32)> {
        let mut r = RECT::default();
        unsafe { GetWindowRect(h, &mut r).ok()? };
        Some((r.left, r.top, r.right, r.bottom))
    }

    fn ecran(h: HWND) -> Option<(i32, i32, i32, i32)> {
        unsafe {
            let e = MonitorFromWindow(h, MONITOR_DEFAULTTONEAREST);
            let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
            GetMonitorInfoW(e, &mut info).as_bool().then_some((info.rcMonitor.left, info.rcMonitor.top, info.rcMonitor.right, info.rcMonitor.bottom))
        }
    }

    /// La plus grande fenêtre principale visible des programmes du jeu (au moins 200 × 150 ; jamais une console).
    pub(super) fn fenetre_du_jeu(pids: &[u32]) -> Option<(HWND, (i32, i32, i32, i32))> {
        crate::menu_jeu::fenetres_du_processus(pids)
            .into_iter()
            .filter(|h| classe(*h) != "ConsoleWindowClass")
            .filter_map(|h| {
                let r = rect(h)?;
                (r.2 - r.0 >= 200 && r.3 - r.1 >= 150).then_some((h, r))
            })
            .max_by_key(|(_, r)| (r.2 - r.0) * (r.3 - r.1))
    }

    /// Alt, Ctrl, Maj, Windows tenus par la personne ? (Une touche envoyée alors changerait de sens : Alt+F4 ferme.)
    fn modificateur_tenu() -> bool {
        [VK_MENU, VK_CONTROL, VK_SHIFT, VK_LWIN, VK_RWIN].iter().any(|v| unsafe { GetAsyncKeyState(v.0 as i32) } as u16 & 0x8000 != 0)
    }

    pub fn appliquer(pids: &dyn Fn() -> Vec<u32>, m: &Mode, etat: &dyn Fn() -> Etat, duree: Duration) -> Result<String, String> {
        let debut = Instant::now();
        let mut traitees: Vec<isize> = Vec::new();
        let mut fait: Vec<String> = Vec::new();
        let mut vue: Option<(isize, (i32, i32, i32, i32))> = None;
        let mut stable_depuis = Instant::now();
        let mut jamais_vue = true;
        while debut.elapsed() < duree && traitees.len() < 3 {
            std::thread::sleep(Duration::from_millis(700));
            match etat() {
                Etat::Finie => {
                    if jamais_vue {
                        return Ok("partie finie avant l'ouverture du jeu".into());
                    }
                    break;
                }
                Etat::Attendre => {
                    vue = None;
                    continue;
                }
                Etat::Libre => {}
            }
            let Some((h, r)) = fenetre_du_jeu(&pids()) else { continue };
            jamais_vue = false;
            // Une fenêtre déjà traitée existe encore : on ne touche à AUCUNE autre (la personne a pu remettre le jeu en
            // fenêtre exprès, et le jeu recréer sa fenêtre : son choix n'est jamais défait). Seul un lanceur qui s'est
            // fermé laisse la place à la vraie fenêtre du jeu.
            if traitees.iter().any(|t| unsafe { IsWindow(Some(HWND(*t as *mut core::ffi::c_void))).as_bool() }) {
                continue;
            }
            if vue != Some((h.0 as isize, r)) {
                vue = Some((h.0 as isize, r));
                stable_depuis = Instant::now();
                continue;
            }
            // Même fenêtre, même taille depuis 2 s : le jeu a fini de l'ouvrir.
            if stable_depuis.elapsed() < Duration::from_secs(2) || unsafe { IsHungAppWindow(h).as_bool() } {
                continue;
            }
            let c = classe(h);
            match unsafe { agir(h, m, etat) } {
                Ok(Some(s)) => {
                    traitees.push(h.0 as isize);
                    fait.push(format!("{s} (fenêtre « {c} »)"));
                }
                // Le menu s'est ouvert (ou le jeu a été figé) juste avant d'agir : on attendra.
                Ok(None) => vue = None,
                Err(e) => return Err(format!("{e} (fenêtre « {c} »)")),
            }
        }
        if jamais_vue {
            return Err("aucune fenêtre du jeu (dans son dossier) en 90 s".into());
        }
        Ok(if fait.is_empty() { "rien à faire".into() } else { fait.join(" ; ") })
    }

    /// Agit sur la fenêtre. `Ok(None)` : la partie n'est plus libre (menu, jeu figé) — rien n'a été fait, à refaire.
    unsafe fn agir(h: HWND, m: &Mode, etat: &dyn Fn() -> Etat) -> Result<Option<String>, String> {
        // D'ABORD : couvre-t-elle déjà l'écran ? (Une fenêtre agrandie au maximum aussi : la « restaurer » la sortirait
        // du plein écran — vu à l'essai réel du 16/10.)
        let Some(e) = ecran(h) else { return Err("écran introuvable".into()) };
        if !IsIconic(h).as_bool() && rect(h).is_some_and(|r| couvre_l_ecran(r, e)) {
            return Ok(Some("déjà en plein écran : rien".into()));
        }
        if etat() != Etat::Libre {
            return Ok(None);
        }
        if IsIconic(h).as_bool() || IsZoomed(h).as_bool() {
            let _ = ShowWindow(h, SW_RESTORE);
            std::thread::sleep(Duration::from_millis(300));
        }
        match m {
            Mode::Aucun => Ok(Some("rien".into())),
            Mode::Fenetre => {
                let style = GetWindowLongPtrW(h, GWL_STYLE);
                let ex = GetWindowLongPtrW(h, GWL_EXSTYLE);
                let sans = style & !((WS_CAPTION | WS_THICKFRAME | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX).0 as isize);
                SetWindowLongPtrW(h, GWL_STYLE, sans);
                SetWindowLongPtrW(h, GWL_EXSTYLE, ex & !((WS_EX_CLIENTEDGE | WS_EX_WINDOWEDGE | WS_EX_DLGMODALFRAME).0 as isize));
                // Sans activer le jeu (le menu en jeu, s'il s'ouvre, reste devant) ; sans attendre un jeu occupé.
                let _ = SetWindowPos(
                    h,
                    Some(HWND_TOP),
                    e.0,
                    e.1,
                    e.2 - e.0,
                    e.3 - e.1,
                    SWP_FRAMECHANGED | SWP_NOOWNERZORDER | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS,
                );
                std::thread::sleep(Duration::from_millis(700));
                // Le RÉSULTAT : la fenêtre couvre-t-elle l'écran ? (Un jeu lancé en administrateur refuse sans le dire.)
                match rect(h) {
                    Some(r) if couvre_l_ecran(r, e) => Ok(Some(format!("fenêtre agrandie à {} × {}", e.2 - e.0, e.3 - e.1))),
                    _ => {
                        // Échec : son cadre lui est rendu (jamais une petite fenêtre sans barre de titre).
                        SetWindowLongPtrW(h, GWL_STYLE, style);
                        SetWindowLongPtrW(h, GWL_EXSTYLE, ex);
                        let _ = SetWindowPos(h, None, 0, 0, 0, 0, SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS);
                        Err("la fenêtre du jeu n'a pas pu être agrandie (jeu lancé en administrateur ?)".into())
                    }
                }
            }
            Mode::Touche(nom) => {
                let Some(&(_, scan, etendue, alt)) = TOUCHES.iter().find(|t| t.0 == *nom) else { return Err("touche inconnue".into()) };
                // Jamais pendant qu'une touche de modification est tenue (F4 deviendrait Alt+F4).
                let attente = Instant::now();
                while modificateur_tenu() {
                    if attente.elapsed() > Duration::from_secs(3) {
                        return Err("une touche (Alt, Ctrl, Maj) était tenue : touche non envoyée".into());
                    }
                    std::thread::sleep(Duration::from_millis(200));
                }
                // L'état relu JUSTE AVANT de passer le jeu devant (le menu a pu s'ouvrir pendant l'attente).
                if etat() != Etat::Libre {
                    return Ok(None);
                }
                let _ = SetForegroundWindow(h);
                std::thread::sleep(Duration::from_millis(300));
                // La touche ne part QUE si le jeu a le premier plan (jamais dans une autre fenêtre) et la partie libre.
                if etat() != Etat::Libre {
                    return Ok(None);
                }
                if GetForegroundWindow() != h {
                    return Err("le jeu n'a pas le premier plan : touche non envoyée".into());
                }
                crate::menu_jeu::envoyer_touche(scan, etendue, if alt { &[0x38] } else { &[] });
                std::thread::sleep(Duration::from_millis(1500));
                Ok(Some(match rect(h) {
                    Some(r) if couvre_l_ecran(r, e) => format!("touche {nom} envoyée : le jeu est en plein écran"),
                    _ => format!("touche {nom} envoyée (effet non vérifiable : la fenêtre a pu être recréée)"),
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests_lancement {
    #[test]
    fn aucune_fenetre_pour_un_programme_absent() {
        assert!(!super::fenetre_visible(&[]));
        assert!(!super::fenetre_visible(&[u32::MAX - 7]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_reglage_se_lit_et_une_valeur_inconnue_ne_fait_rien() {
        assert_eq!(mode("fenetre"), Mode::Fenetre);
        assert_eq!(mode("touche:F4"), Mode::Touche("F4"));
        assert_eq!(mode("touche:Alt+Entrée"), Mode::Touche("Alt+Entrée"));
        assert_eq!(mode("touche:Suppr"), Mode::Aucun);
        assert_eq!(mode(""), Mode::Aucun);
        assert_eq!(mode("aucun"), Mode::Aucun);
    }

    #[test]
    fn la_touche_se_trouve_dans_les_lignes_reelles_des_jeux_de_seb() {
        // Relevés du 16/10 (ReadMe de Mega Man Rock Force, options.txt de Minecraft).
        assert_eq!(touche_dans_ligne("F4-Toggle Fullscreen Mode"), Some("F4"));
        assert_eq!(touche_dans_ligne("key_key.fullscreen:key.keyboard.f11"), Some("F11"));
        // Une ligne « fullscreen » sans touche (MegaMan Unlimited, JourneyMap) : rien.
        assert_eq!(touche_dans_ligne("(You can set it up to 1x, 2x, 3x, 4x or FullScreen)"), None);
        assert_eq!(touche_dans_ligne("key_key.journeymap.fullscreen_follow_player:key.keyboard.f"), None);
        assert_eq!(touche_dans_ligne("fullscreen:false"), None);
        // Autres formes courantes.
        assert_eq!(touche_dans_ligne("Press Alt+Enter to toggle full screen"), Some("Alt+Entrée"));
        assert_eq!(touche_dans_ligne("Plein écran : F11"), Some("F11"));
        assert_eq!(touche_dans_ligne("F9 - Take Screenshot"), None, "ne parle pas de plein écran");
        assert_eq!(touche_dans_ligne("fullscreen f4x"), None);
        // « Alt+F4 » n'est jamais la touche du plein écran ; la suivante l'est.
        assert_eq!(touche_dans_ligne("Alt+F4 quits full screen, F11 toggles it"), Some("F11"));
    }

    #[test]
    fn une_fenetre_qui_couvre_deja_l_ecran_n_est_pas_touchee() {
        let ecran = (0, 0, 1920, 1080);
        assert!(couvre_l_ecran((0, 0, 1920, 1080), ecran));
        assert!(couvre_l_ecran((-8, -8, 1928, 1088), ecran), "fenêtre agrandie avec ses bords hors de l'écran");
        assert!(!couvre_l_ecran((0, 0, 640, 480), ecran));
        assert!(!couvre_l_ecran((0, 0, 1920, 1040), ecran), "barre des tâches visible : pas plein écran");
        // Deuxième écran à droite.
        assert!(couvre_l_ecran((1920, 0, 4480, 1440), (1920, 0, 4480, 1440)));
    }

    #[test]
    fn l_interface_propose_les_memes_touches_que_le_coeur() {
        let ts = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../src/lib/ludotheque/plein-ecran.ts")).unwrap();
        for (nom, ..) in TOUCHES {
            assert!(ts.contains(&format!("'{nom}'")), "{nom} absente de l'interface");
        }
    }

    #[test]
    fn la_suggestion_lit_un_readme_en_utf16_comme_celui_de_rock_force() {
        let d = tempfile::tempdir().unwrap();
        let texte = "DEFAULT KEYBOARD CONFIGURATION\r\nEsc-Quit Game\r\nF4-Toggle Fullscreen Mode\r\nF9-Take Screenshot\r\n";
        let mut octets = vec![0xFF, 0xFE];
        for u in texte.encode_utf16() {
            octets.extend_from_slice(&u.to_le_bytes());
        }
        std::fs::write(d.path().join("ReadMe.txt"), octets).unwrap();
        assert_eq!(suggestion(d.path()), Some("F4"));
        let vide = tempfile::tempdir().unwrap();
        std::fs::write(vide.path().join("notes.txt"), "Rien ici").unwrap();
        assert_eq!(suggestion(vide.path()), None);
    }
}

#[cfg(test)]
mod essai_reel {
    /// Sur les vrais dossiers de jeux (lecture seule) : `FROGTEND_JEUX=<dossier> cargo test essai_suggestions -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn essai_suggestions() {
        let racine = std::path::PathBuf::from(std::env::var("FROGTEND_JEUX").unwrap());
        for e in std::fs::read_dir(&racine).unwrap().flatten() {
            println!("JEU {:?} : {:?}", e.file_name(), super::suggestion(&e.path()));
        }
    }
}

#[cfg(all(test, windows))]
mod essai_reel_fenetre {
    /// Une VRAIE fenêtre d'essai (formulaire PowerShell, ouvert par ce test) agrandie sans bordure, puis fermée.
    /// `cargo test essai_fenetre_agrandie -- --ignored --nocapture` (la fenêtre apparaît quelques secondes à l'écran).
    #[test]
    #[ignore]
    fn essai_fenetre_agrandie() {
        let mut enfant = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", "Add-Type -AssemblyName System.Windows.Forms; $f = New-Object Windows.Forms.Form; $f.Text = 'Frogtend essai plein ecran'; $f.Width = 640; $f.Height = 480; $f.ShowDialog()"])
            .spawn()
            .unwrap();
        let pid = enfant.id();
        let pids = move || crate::lancement::processus_de_la_partie(pid);
        let r = super::appliquer_pids(&pids, &super::Mode::Fenetre, &|| super::Etat::Libre, std::time::Duration::from_secs(15));
        println!("RESULTAT {r:?}");
        std::thread::sleep(std::time::Duration::from_secs(2));
        let _ = enfant.kill();
        let _ = enfant.wait();
        assert!(r.unwrap().starts_with("fenêtre agrandie"));
        // Une deuxième fois sur une fenêtre déjà plein écran : rien (jamais de bascule inverse).
        let mut enfant2 = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", "Add-Type -AssemblyName System.Windows.Forms; $f = New-Object Windows.Forms.Form; $f.WindowState = 'Maximized'; $f.FormBorderStyle = 'None'; $f.ShowDialog()"])
            .spawn()
            .unwrap();
        let pid2 = enfant2.id();
        let pids2 = move || crate::lancement::processus_de_la_partie(pid2);
        let r2 = super::appliquer_pids(&pids2, &super::Mode::Touche("F11"), &|| super::Etat::Libre, std::time::Duration::from_secs(15));
        println!("RESULTAT 2 {r2:?}");
        let _ = enfant2.kill();
        let _ = enfant2.wait();
        assert!(r2.unwrap().contains("déjà en plein écran"));
    }
}
