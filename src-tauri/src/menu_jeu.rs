//! Le menu universel en jeu (lot 4 ter) : pendant une partie, une touche (Pause/Attn par défaut, réglable) ouvre la
//! fenêtre « menu-jeu » de Frogtend par-dessus le jeu. L'émulateur, réglé par `pilotage`, se met en pause en perdant
//! le premier plan. Le menu rend ensuite la main au jeu et lui transmet l'ordre choisi.
//!
//! Voir `docs/RECHERCHE-MENU-EN-JEU.md`.

use crate::erreurs::{Erreur, Resultat};
use crate::pilotage::Action;
use serde::Serialize;
use std::sync::Mutex;
use std::time::Duration;

/// La partie en cours, pour le menu.
#[derive(Clone, Debug, Serialize)]
pub struct PartieEnCours {
    pub jeu: i64,
    pub titre: String,
    pub plateforme: String,
    /// Le processus lancé par Frogtend (émulateur ou lanceur du jeu).
    pub pid: u32,
    /// L'émulateur, s'il est connu de Frogtend (`retroarch`, `duckstation`…).
    pub emulateur: Option<String>,
    /// Le dossier du programme de l'émulateur (aucun pour un jeu PC) : seul ce qui s'y trouve peut être arrêté de force
    /// après « Quitter ».
    pub dossier_emulateur: Option<String>,
}

#[derive(Default)]
pub struct MenuJeu {
    pub partie: Mutex<Option<PartieEnCours>>,
    /// Le numéro de la partie en cours : la lecture de la manette d'une partie s'arrête quand il change.
    pub numero: std::sync::atomic::AtomicU64,
    /// La fenêtre du menu est-elle ouverte ? Tenu par `ouvrir_menu` et `cacher_menu` (la lecture de la manette le
    /// consulte 60 fois par seconde sans interroger la fenêtre).
    pub ouvert: std::sync::atomic::AtomicBool,
}

impl MenuJeu {
    pub fn partie(&self) -> Option<PartieEnCours> {
        self.partie.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
    /// Retient la partie et rend son numéro.
    pub fn commencer(&self, p: PartieEnCours) -> u64 {
        *self.partie.lock().unwrap_or_else(|e| e.into_inner()) = Some(p);
        self.numero.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1
    }
    /// Oublie la partie `numero` (une partie plus récente n'est jamais effacée par la fin d'une ancienne).
    pub fn finir(&self, numero: u64) {
        if self.numero.load(std::sync::atomic::Ordering::SeqCst) == numero {
            *self.partie.lock().unwrap_or_else(|e| e.into_inner()) = None;
        }
    }
    /// La partie `numero` est-elle toujours celle en cours ?
    pub fn en_cours(&self, numero: u64) -> bool {
        self.numero.load(std::sync::atomic::Ordering::SeqCst) == numero && self.partie().is_some()
    }
}

/// Ce que le menu montre : la partie et les actions possibles.
#[derive(Debug, Serialize, PartialEq)]
pub struct EtatMenu {
    pub jeu: i64,
    pub titre: String,
    pub plateforme: String,
    pub emulateur: Option<String>,
    /// Actions possibles en plus de reprendre et quitter : `reset`, `disque`, `sauver`, `charger`.
    pub actions: Vec<&'static str>,
    /// L'émulateur se met-il en pause quand le menu s'ouvre ? (Non pour un jeu PC.)
    pub en_pause: bool,
    /// Ouvert en Taodbox (fenêtre principale en plein écran) : le menu est à l'échelle ×2. Rempli par la commande.
    pub taodbox: bool,
}

pub fn nom_action(a: Action) -> &'static str {
    match a {
        Action::Reset => "reset",
        Action::DisqueSuivant => "disque",
        Action::SauverEtat => "sauver",
        Action::ChargerEtat => "charger",
    }
}

pub fn action_de(nom: &str) -> Option<Action> {
    match nom {
        "reset" => Some(Action::Reset),
        "disque" => Some(Action::DisqueSuivant),
        "sauver" => Some(Action::SauverEtat),
        "charger" => Some(Action::ChargerEtat),
        _ => None,
    }
}

pub fn etat(p: &PartieEnCours) -> EtatMenu {
    let e = p.emulateur.as_deref().unwrap_or("");
    EtatMenu {
        jeu: p.jeu,
        titre: p.titre.clone(),
        plateforme: p.plateforme.clone(),
        emulateur: p.emulateur.clone(),
        actions: crate::pilotage::actions(e).into_iter().map(nom_action).collect(),
        en_pause: crate::pilotage::se_met_en_pause(e),
        taodbox: false,
    }
}

/// Envoie une commande réseau à RetroArch (127.0.0.1, voir `pilotage::lignes_retroarch`).
pub fn commande_retroarch(commande: &str) -> Resultat<()> {
    let s = std::net::UdpSocket::bind("127.0.0.1:0").map_err(|e| Erreur::Disque(format!("Commande impossible ({e}).")))?;
    s.send_to(format!("{commande}\n").as_bytes(), ("127.0.0.1", crate::pilotage::PORT_RETROARCH))
        .map_err(|e| Erreur::Disque(format!("RetroArch ne répond pas ({e}).")))?;
    Ok(())
}

#[cfg(windows)]
mod fenetres {
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_EXTENDEDKEY,
        KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, MOUSEEVENTF_MOVE, MOUSEINPUT, VIRTUAL_KEY,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetForegroundWindow, GetWindow, GetWindowThreadProcessId, IsIconic, IsWindowVisible, PostMessageW,
        SetForegroundWindow, ShowWindow, GW_OWNER, SW_RESTORE, WM_CLOSE,
    };

    /// Une entrée neutre (la souris « bouge » de 0) : Windows laisse alors notre processus prendre le premier plan
    /// (SetForegroundWindow l'accepte quand « le processus a reçu le dernier événement d'entrée »). Sans elle, un menu
    /// ouvert depuis la manette resterait derrière le jeu (pas de pause, la manette pilote encore le jeu).
    pub fn entree_neutre() {
        let i = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 { mi: MOUSEINPUT { dx: 0, dy: 0, mouseData: 0, dwFlags: MOUSEEVENTF_MOVE, time: 0, dwExtraInfo: 0 } },
        };
        unsafe {
            SendInput(&[i], std::mem::size_of::<INPUT>() as i32);
        }
    }

    /// Ce processus a-t-il abandonné : plus AUCUNE fenêtre visible ? (Une fenêtre qui ne répond plus peut être un
    /// émulateur en pleine écriture d'une sauvegarde : on n'y touche pas — expert, 05/10.)
    pub fn abandonne(pid: u32) -> bool {
        du_processus(&[pid]).is_empty()
    }

    /// La fenêtre au premier plan appartient-elle à Frogtend ?
    pub fn premier_plan_a_frogtend() -> bool {
        let mut pid = 0u32;
        unsafe {
            GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut pid));
        }
        pid == std::process::id()
    }

    /// Les fenêtres principales (visibles, sans propriétaire) de ces processus.
    pub fn du_processus(pids: &[u32]) -> Vec<HWND> {
        struct Ctx<'a> {
            pids: &'a [u32],
            trouvees: Vec<HWND>,
        }
        unsafe extern "system" fn une(h: HWND, l: LPARAM) -> BOOL {
            let ctx = &mut *(l.0 as *mut Ctx);
            let mut pid = 0u32;
            GetWindowThreadProcessId(h, Some(&mut pid));
            let principale = GetWindow(h, GW_OWNER).map(|o| o.0.is_null()).unwrap_or(true);
            if ctx.pids.contains(&pid) && IsWindowVisible(h).as_bool() && principale {
                ctx.trouvees.push(h);
            }
            BOOL(1)
        }
        let mut ctx = Ctx { pids, trouvees: Vec::new() };
        unsafe {
            let _ = EnumWindows(Some(une), LPARAM(&mut ctx as *mut _ as isize));
        }
        ctx.trouvees
    }

    /// Le processus qui porte la fenêtre principale du jeu (celui sur lequel brancher Cheat Engine).
    pub fn pid_de_la_fenetre(pids: &[u32]) -> Option<u32> {
        let h = du_processus(pids).into_iter().next()?;
        let mut pid = 0u32;
        unsafe {
            GetWindowThreadProcessId(h, Some(&mut pid));
        }
        (pid != 0).then_some(pid)
    }

    /// Rend le premier plan au jeu. Rend `false` si aucune fenêtre n'a été trouvée.
    pub fn au_premier_plan(pids: &[u32]) -> bool {
        let Some(h) = du_processus(pids).into_iter().next() else { return false };
        unsafe {
            if IsIconic(h).as_bool() {
                let _ = ShowWindow(h, SW_RESTORE);
            }
            SetForegroundWindow(h).as_bool()
        }
    }

    /// Demande poliment aux fenêtres du jeu de se fermer (comme la croix) : l'émulateur enregistre ce qu'il doit.
    pub fn fermer(pids: &[u32]) -> usize {
        let l = du_processus(pids);
        for h in &l {
            unsafe {
                let _ = PostMessageW(Some(*h), WM_CLOSE, WPARAM(0), LPARAM(0));
            }
        }
        l.len()
    }

    fn cle(scan: u16, etendue: bool, relachee: bool) -> INPUT {
        let mut f: KEYBD_EVENT_FLAGS = KEYEVENTF_SCANCODE;
        if etendue {
            f |= KEYEVENTF_EXTENDEDKEY;
        }
        if relachee {
            f |= KEYEVENTF_KEYUP;
        }
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VIRTUAL_KEY(0), wScan: scan, dwFlags: f, time: 0, dwExtraInfo: 0 } },
        }
    }

    /// Appuie sur une touche (codes de balayage : DirectInput les lit, comme Qt) avec ses modificateurs, en la tenant
    /// un instant (certains émulateurs agissent au relâchement, à la trame suivante).
    pub fn touche(scan: u16, etendue: bool, modificateurs: &[u16]) {
        let mut bas: Vec<INPUT> = modificateurs.iter().map(|m| cle(*m, false, false)).collect();
        bas.push(cle(scan, etendue, false));
        let mut haut = vec![cle(scan, etendue, true)];
        haut.extend(modificateurs.iter().rev().map(|m| cle(*m, false, true)));
        unsafe {
            SendInput(&bas, std::mem::size_of::<INPUT>() as i32);
        }
        std::thread::sleep(std::time::Duration::from_millis(80));
        unsafe {
            SendInput(&haut, std::mem::size_of::<INPUT>() as i32);
        }
    }
}

/// Les codes de balayage (jeu 1) des touches envoyées.
fn code(nom: &str) -> Option<(u16, bool)> {
    Some(match nom {
        "F13" => (0x64, false),
        "F14" => (0x65, false),
        "F15" => (0x66, false),
        "F16" => (0x67, false),
        "F4" => (0x3E, false),
        "Home" => (0x47, true),
        _ => return None,
    })
}

const MAJ: u16 = 0x2A;
const CTRL: u16 = 0x1D;
const ALT: u16 = 0x38;

/// Le processus du jeu en cours qui a la fenêtre (sinon le premier lancé).
pub fn processus_du_jeu(racine: u32) -> u32 {
    let pids = crate::lancement::processus_de_la_partie(racine);
    #[cfg(windows)]
    if let Some(p) = fenetres::pid_de_la_fenetre(&pids) {
        return p;
    }
    let _ = pids;
    racine
}

/// Prépare la prise du premier plan par Frogtend (menu ouvert depuis la manette) : une entrée neutre.
pub fn autoriser_premier_plan() {
    #[cfg(windows)]
    fenetres::entree_neutre();
}

/// Ce processus de l'émulateur a-t-il abandonné (plus aucune fenêtre visible) ? (`false` hors Windows.)
pub fn abandonne(pid: u32) -> bool {
    #[cfg(windows)]
    {
        fenetres::abandonne(pid)
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
        false
    }
}

/// La fenêtre au premier plan est-elle à Frogtend ? (`true` hors Windows.)
pub fn frogtend_au_premier_plan() -> bool {
    #[cfg(windows)]
    {
        fenetres::premier_plan_a_frogtend()
    }
    #[cfg(not(windows))]
    {
        true
    }
}

/// Rend le premier plan au jeu (reprise : l'émulateur sort seul de la pause).
pub fn reprendre(pids: &[u32]) -> bool {
    #[cfg(windows)]
    {
        fenetres::au_premier_plan(pids)
    }
    #[cfg(not(windows))]
    {
        let _ = pids;
        false
    }
}

/// Transmet une action à l'émulateur : le jeu reprend le premier plan, puis reçoit l'ordre.
pub fn agir(emulateur: &str, pids: &[u32], a: Action) -> Resultat<()> {
    if emulateur == "retroarch" {
        reprendre(pids);
        std::thread::sleep(Duration::from_millis(150));
        return commande_retroarch(crate::pilotage::commande_retroarch(a));
    }
    let t = crate::pilotage::touche(emulateur, a)
        .ok_or_else(|| Erreur::Refus("Cet émulateur ne sait pas le faire depuis le menu.".into()))?;
    let (scan, etendue) = code(t.touche).ok_or_else(|| Erreur::Refus(format!("Touche inconnue : {}.", t.touche)))?;
    if !reprendre(pids) {
        return Err(Erreur::Introuvable("La fenêtre du jeu est introuvable.".into()));
    }
    // Le temps que l'émulateur reprenne le premier plan et sorte de sa pause.
    std::thread::sleep(Duration::from_millis(250));
    let mut m = Vec::new();
    if t.ctrl {
        m.push(CTRL);
    }
    if t.alt {
        m.push(ALT);
    }
    if t.maj {
        m.push(MAJ);
    }
    #[cfg(windows)]
    fenetres::touche(scan, etendue, &m);
    #[cfg(not(windows))]
    let _ = (scan, etendue, m);
    Ok(())
}

/// Quitte proprement : la commande de RetroArch, sinon la fermeture de la fenêtre (comme la croix). Rend le nombre
/// de fenêtres fermées (0 : rien trouvé).
pub fn quitter(emulateur: Option<&str>, pids: &[u32]) -> Resultat<usize> {
    if emulateur == Some("retroarch") && commande_retroarch("QUIT").is_ok() {
        return Ok(1);
    }
    #[cfg(windows)]
    {
        Ok(fenetres::fermer(pids))
    }
    #[cfg(not(windows))]
    {
        let _ = pids;
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn partie(emulateur: Option<&str>) -> PartieEnCours {
        PartieEnCours { jeu: 1, titre: "Jeu".into(), plateforme: "X".into(), pid: 1, emulateur: emulateur.map(String::from), dossier_emulateur: None }
    }

    #[test]
    fn le_menu_propose_ce_que_l_emulateur_sait_faire() {
        let e = etat(&partie(Some("retroarch")));
        assert_eq!(e.actions, vec!["reset", "disque", "sauver", "charger"]);
        assert!(e.en_pause);
        let e = etat(&partie(Some("pcsx2")));
        assert!(!e.actions.contains(&"disque"));
        let e = etat(&partie(None));
        assert!(e.actions.is_empty() && !e.en_pause, "jeu PC : reprendre et quitter seulement");
        for n in ["reset", "disque", "sauver", "charger"] {
            assert_eq!(nom_action(action_de(n).unwrap()), n);
        }
        assert!(action_de("formater").is_none());
    }

    #[test]
    fn chaque_touche_envoyee_a_son_code() {
        for emu in ["duckstation", "pcsx2", "dolphin", "dosbox-staging"] {
            for a in crate::pilotage::actions(emu) {
                let t = crate::pilotage::touche(emu, a).unwrap();
                assert!(code(t.touche).is_some(), "{emu} {a:?} : {}", t.touche);
            }
        }
    }

    #[test]
    fn les_touches_proposees_dans_les_options_sont_comprises() {
        // Les choix de ⚙ Options ▸ Émulateurs (src/lib/reglages/Emulateurs.svelte).
        for t in ["Pause", "ScrollLock", "Ctrl+Shift+M"] {
            assert!(t.parse::<tauri_plugin_global_shortcut::Shortcut>().is_ok(), "{t}");
        }
    }

    #[test]
    fn la_partie_en_cours_se_retient_et_s_oublie() {
        let m = MenuJeu::default();
        assert!(m.partie().is_none());
        let n = m.commencer(partie(Some("dolphin")));
        assert_eq!(m.partie().unwrap().emulateur.as_deref(), Some("dolphin"));
        assert!(m.en_cours(n));
        // Une deuxième partie commence avant la fin de la première : la fin de la première ne l'efface pas.
        let n2 = m.commencer(partie(Some("rpcs3")));
        assert!(!m.en_cours(n), "la lecture de la manette de la 1re partie s'arrête");
        m.finir(n);
        assert_eq!(m.partie().unwrap().emulateur.as_deref(), Some("rpcs3"));
        m.finir(n2);
        assert!(m.partie().is_none());
    }
}
