//! Le menu en jeu à la manette (lot OSD, étape 3). Pendant une partie, le CŒUR lit les manettes XInput (manettes Xbox
//! et compatibles, manette virtuelle de Sunshine) — la sonde de Seb (04/10) a prouvé qu'elles restent lisibles quand
//! le jeu a le premier plan. La page du menu ne lit pas la manette elle-même : la Gamepad API d'une fenêtre qui vient
//! d'apparaître par-dessus un jeu ne répond pas (essai de Seb, 0.45.0).
//!
//! - Menu fermé : la combinaison (Select + R1 par défaut, décision de Seb du 04/10) tenue 1 s l'ouvre ; le bouton Xbox
//!   (Guide) aussi, d'un appui court, seul (✅ Seb 28/10, « en plus de Vue + RB ») — seulement si la Game Bar ne le prend
//!   pas. Menu ouvert, le bouton Xbox rend la main au jeu.
//! - Menu ouvert : la croix ou le stick gauche, A et B deviennent des commandes (`haut`, `bas`, `gauche`, `droite`,
//!   `valider`, `retour`) envoyées à la page ; LB / RB : `precedent` / `suivant` (les boutons d'un lecteur). Seuls les appuis NOUVEAUX comptent : la combinaison encore tenue qui
//!   vient d'ouvrir le menu ne fait rien. A et B agissent au RELÂCHEMENT (expert lancement, 05/10) : le bouton qui
//!   ferme le menu (« Reprendre ») n'est plus enfoncé quand le jeu reprend la main, il ne lui arrive donc pas.
//!
//! Ce module ne fait que décider (testable sans manette) ; la lecture et l'envoi sont dans `commandes.rs`.

use std::time::{Duration, Instant};

// Les boutons de XINPUT_GAMEPAD (wButtons), documentation de Microsoft.
pub const HAUT: u16 = 0x0001;
pub const BAS: u16 = 0x0002;
pub const GAUCHE: u16 = 0x0004;
pub const DROITE: u16 = 0x0008;
pub const START: u16 = 0x0010;
pub const SELECT: u16 = 0x0020;
pub const L3: u16 = 0x0040;
pub const R3: u16 = 0x0080;
pub const L1: u16 = 0x0100;
pub const R1: u16 = 0x0200;
/// Le bouton Xbox (Guide) : absent de l'API officielle, donné par `XInputGetStateEx` (ordinal 100 de xinput1_4.dll,
/// non documenté, utilisé par SDL, RetroArch, Dolphin…) et par SDL pour les autres manettes (PS, Home).
pub const GUIDE: u16 = 0x0400;
pub const A: u16 = 0x1000;
pub const B: u16 = 0x2000;

/// Le temps de maintien de la combinaison avant d'ouvrir le menu (contre les ouvertures par erreur).
pub const MAINTIEN: Duration = Duration::from_millis(1000);
/// La répétition d'une direction tenue : un premier pas, puis la répétition après une pause (comme une touche).
const PREMIERE_REPETITION: Duration = Duration::from_millis(380);
const REPETITION: Duration = Duration::from_millis(130);
/// Le stick gauche compte comme une direction au-delà de la moitié de sa course.
const SEUIL_STICK: i16 = 16_000;

/// L'état des manettes à un instant : les boutons (de toutes les manettes réunies) et le stick gauche.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EtatManette {
    pub boutons: u16,
    pub lx: i16,
    pub ly: i16,
    /// Les gâchettes gauche et droite (0 à 255, la plus enfoncée des manettes) : pour « manette → clavier ».
    pub gt: u8,
    pub dt: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Evenement {
    /// Ouvrir le menu (combinaison tenue).
    Ouvrir,
    /// Une commande pour le menu ouvert.
    Commande(&'static str),
    /// Un raccourci de sauvegarde rapide, menu fermé (✅ Seb 28/10) : `sauver` (Vue + LT tenus 0,5 s) ou `charger`
    /// (Vue + RT tenus 1 s), dans l'emplacement choisi.
    Raccourci(&'static str),
}

/// Le maintien des raccourcis de sauvegarde rapide (charger plus long : on y perd la partie en cours).
pub const MAINTIEN_SAUVER: Duration = Duration::from_millis(500);
pub const MAINTIEN_CHARGER: Duration = Duration::from_millis(1000);
/// Une gâchette « enfoncée » : au-delà de la moitié.
const GACHETTE: u8 = 128;
/// Le bouton Xbox n'ouvre le menu que d'un appui COURT : tenu plus longtemps, c'est qu'on éteint la manette.
pub const GUIDE_COURT: Duration = Duration::from_millis(1000);

/// Les émulateurs dont le réglage d'origine du bouton Xbox n'est pas connu (RPCS3 : le bouton PS ouvre son menu ;
/// xemu, Xenia, Eden, Cemu, PPSSPP, Vita3K : pas vérifié). Le bouton y reste au jeu : on ne promet pas un double menu.
const GUIDE_A_L_EMULATEUR: [&str; 7] = ["rpcs3", "xemu", "xenia", "eden", "cemu", "ppsspp", "vita3k"];

/// Ce qui fait tourner la partie, pour le bouton Xbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Moteur<'a> {
    /// Un jeu PC, sans émulateur.
    JeuPc,
    /// Un émulateur reconnu (son identifiant Frogtend).
    Emulateur(&'a str),
    /// Un émulateur que Frogtend ne reconnaît pas (hors catalogue) : on ne sait pas ce qu'il fait du bouton.
    Inconnu,
}

/// Le bouton Xbox ouvre-t-il le menu pendant cette partie ? Seulement si la Game Bar de Windows ne le prend pas
/// (`Some(false)` lu dans le registre ; `None` hors de Windows), si le réglage de Frogtend le permet, et si
/// l'émulateur est connu et ne s'en sert pas lui-même.
pub fn guide_permis(game_bar: Option<bool>, reglage: bool, moteur: Moteur) -> bool {
    game_bar == Some(false)
        && reglage
        && match moteur {
            Moteur::JeuPc => true,
            Moteur::Emulateur(e) => !GUIDE_A_L_EMULATEUR.contains(&e),
            Moteur::Inconnu => false,
        }
}

/// Le bouton Xbox d'une manette qui vient d'apparaître ne compte pas tant qu'on ne l'a pas vu relâché : on ALLUME
/// une manette sans fil par ce bouton, et sa première lecture le montre enfoncé (relecture lancement, 28/10).
pub struct GardeGuide<K> {
    vues: Vec<(K, bool)>,
}

impl<K: PartialEq + Copy> Default for GardeGuide<K> {
    fn default() -> Self {
        GardeGuide { vues: Vec::new() }
    }
}

impl<K: PartialEq + Copy> GardeGuide<K> {
    /// Les boutons de la manette `cle`, sans le Guide s'il n'a jamais été vu relâché depuis son apparition.
    pub fn filtrer(&mut self, cle: K, boutons: u16) -> u16 {
        let i = match self.vues.iter().position(|(k, _)| *k == cle) {
            Some(i) => i,
            None => {
                self.vues.push((cle, false));
                self.vues.len() - 1
            }
        };
        if boutons & GUIDE == 0 {
            self.vues[i].1 = true;
        }
        if self.vues[i].1 {
            boutons
        } else {
            boutons & !GUIDE
        }
    }

    /// Seules les manettes encore présentes restent connues.
    pub fn retenir(&mut self, presentes: &[K]) {
        self.vues.retain(|(k, _)| presentes.contains(k));
    }

    /// La manette `cle` a disparu : à sa réapparition, son Guide devra de nouveau être vu relâché.
    pub fn oublier(&mut self, cle: K) {
        self.vues.retain(|(k, _)| *k != cle);
    }
}

/// La combinaison d'ouverture d'après le réglage (`pc.json` ▸ `menuJeu.manette`) ; Select + R1 par défaut.
pub fn combinaison(nom: &str) -> u16 {
    match nom.trim() {
        "Select + L1" => SELECT | L1,
        "L3 + R3" => L3 | R3,
        "Select + Start" => SELECT | START,
        _ => SELECT | R1,
    }
}

/// La direction demandée : la croix, sinon le stick gauche.
fn direction(e: EtatManette) -> Option<&'static str> {
    if e.boutons & HAUT != 0 {
        Some("haut")
    } else if e.boutons & BAS != 0 {
        Some("bas")
    } else if e.boutons & GAUCHE != 0 {
        Some("gauche")
    } else if e.boutons & DROITE != 0 {
        Some("droite")
    } else if e.ly > SEUIL_STICK {
        Some("haut") // XInput : vers le haut = positif
    } else if e.ly < -SEUIL_STICK {
        Some("bas")
    } else if e.lx < -SEUIL_STICK {
        Some("gauche")
    } else if e.lx > SEUIL_STICK {
        Some("droite")
    } else {
        None
    }
}

/// Ce que décide la lecture de la manette, tour après tour.
pub struct Osd {
    combinaison: u16,
    tenue_depuis: Option<Instant>,
    /// La combinaison a déjà ouvert le menu : il faut la relâcher avant qu'elle puisse le rouvrir.
    declenchee: bool,
    avant: u16,
    dir: Option<&'static str>,
    prochaine: Option<Instant>,
    /// Le menu était ouvert au tour précédent (au premier tour d'ouverture, rien de ce qui est tenu ne compte).
    etait_ouvert: bool,
    /// A ou B enfoncés menu ouvert : ils agiront au relâchement.
    armes: u16,
    /// Les raccourcis de sauvegarde rapide (réglage, actifs d'office) ; tenus depuis ; déjà déclenchés (à relâcher).
    raccourcis: bool,
    sauver_depuis: Option<Instant>,
    charger_depuis: Option<Instant>,
    raccourci_fait: bool,
    /// Le bouton Xbox ouvre le menu ; enfoncé depuis (appui nouveau, menu fermé) ; un autre bouton est venu pendant.
    guide: bool,
    guide_depuis: Option<Instant>,
    guide_gache: bool,
    /// Au moins un tour a eu lieu : un bouton déjà tenu au premier tour (au lancement) n'est pas un appui nouveau.
    vu: bool,
}

impl Osd {
    pub fn nouveau(combinaison: u16) -> Self {
        Osd {
            combinaison,
            tenue_depuis: None,
            declenchee: false,
            avant: 0,
            dir: None,
            prochaine: None,
            etait_ouvert: false,
            armes: 0,
            raccourcis: false,
            sauver_depuis: None,
            charger_depuis: None,
            raccourci_fait: false,
            guide: false,
            guide_depuis: None,
            guide_gache: false,
            vu: false,
        }
    }

    /// Avec le bouton Xbox : un appui court et seul ouvre le menu ; menu ouvert, il rend la main au jeu.
    pub fn avec_guide(mut self, oui: bool) -> Self {
        self.guide = oui;
        self
    }

    /// Avec les raccourcis de sauvegarde rapide (Vue + LT / Vue + RT tenus), menu fermé.
    pub fn avec_raccourcis(mut self, oui: bool) -> Self {
        self.raccourcis = oui;
        self
    }

    /// Un tour : l'état des manettes, l'heure, et si le menu est ouvert. Rend ce qu'il faut faire.
    pub fn pas(&mut self, e: EtatManette, t: Instant, menu_ouvert: bool) -> Vec<Evenement> {
        let l = self.pas_interne(e, t, menu_ouvert);
        self.vu = true;
        l
    }

    fn pas_interne(&mut self, e: EtatManette, t: Instant, menu_ouvert: bool) -> Vec<Evenement> {
        let mut l = Vec::new();
        let b = e.boutons;
        let tenue = b & self.combinaison == self.combinaison;
        if !tenue {
            self.tenue_depuis = None;
            self.declenchee = false;
        }
        if !menu_ouvert {
            self.dir = None;
            self.etait_ouvert = false;
            self.armes = 0;
            if tenue && !self.declenchee {
                let depuis = *self.tenue_depuis.get_or_insert(t);
                if t.duration_since(depuis) >= MAINTIEN {
                    self.declenchee = true;
                    l.push(Evenement::Ouvrir);
                }
            }
            // Le bouton Xbox : ouvre au RELÂCHEMENT d'un appui nouveau, court, sans autre bouton ni gâchette entre-temps
            // (Guide + autre chose est une combinaison d'un autre programme ; tenu longtemps, on éteint la manette).
            if self.guide {
                let g = b & GUIDE != 0;
                if g && self.avant & GUIDE == 0 && self.vu {
                    self.guide_depuis = Some(t);
                    self.guide_gache = false;
                }
                if g && (b & !GUIDE != 0 || e.gt >= GACHETTE || e.dt >= GACHETTE) {
                    self.guide_gache = true;
                }
                if !g {
                    if let Some(d) = self.guide_depuis.take() {
                        if !self.guide_gache && t.duration_since(d) < GUIDE_COURT && !l.contains(&Evenement::Ouvrir) {
                            l.push(Evenement::Ouvrir);
                        }
                    }
                }
            }
            // Vue + une SEULE gâchette, tenues (jamais pendant la combinaison du menu) : sauver ou charger, une fois.
            let vue = b & SELECT != 0 && !tenue && self.raccourcis;
            let (lt, rt) = (e.gt >= GACHETTE, e.dt >= GACHETTE);
            let sauver = vue && lt && !rt;
            let charger = vue && rt && !lt;
            if !sauver {
                self.sauver_depuis = None;
            }
            if !charger {
                self.charger_depuis = None;
            }
            if !sauver && !charger {
                self.raccourci_fait = false;
            } else if !self.raccourci_fait {
                let (depuis, maintien, quoi) = if sauver {
                    (*self.sauver_depuis.get_or_insert(t), MAINTIEN_SAUVER, "sauver")
                } else {
                    (*self.charger_depuis.get_or_insert(t), MAINTIEN_CHARGER, "charger")
                };
                if t.duration_since(depuis) >= maintien {
                    self.raccourci_fait = true;
                    l.push(Evenement::Raccourci(quoi));
                }
            }
            self.avant = b;
            return l;
        }
        // Menu ouvert : seuls les appuis nouveaux (un bouton tenu à l'ouverture ne compte pas).
        self.guide_depuis = None;
        let premier_tour = !self.etait_ouvert;
        self.etait_ouvert = true;
        let nouveaux = if premier_tour { 0 } else { b & !self.avant };
        let au_relachement = if self.guide { A | B | GUIDE } else { A | B };
        self.armes |= nouveaux & au_relachement;
        let relaches = self.armes & !b;
        self.armes &= !relaches;
        if relaches & A != 0 {
            l.push(Evenement::Commande("valider"));
        }
        if relaches & B != 0 {
            l.push(Evenement::Commande("retour"));
        }
        // Le bouton Xbox, menu ouvert : retour au jeu (la page l'ignore si une question attend une réponse).
        if relaches & GUIDE != 0 {
            l.push(Evenement::Commande("reprendre"));
        }
        // LB / RB : d'un bouton à l'autre dans un lecteur (livret, musique, vidéo, images), dès l'appui (ils ne
        // ferment rien) ; tenus à l'ouverture (la combinaison Select + R1), ils ne comptent pas (essai de Seb, 28/10).
        if nouveaux & L1 != 0 {
            l.push(Evenement::Commande("precedent"));
        }
        if nouveaux & R1 != 0 {
            l.push(Evenement::Commande("suivant"));
        }
        let d = direction(e);
        match (d, self.dir) {
            (Some(d), Some(avant)) if d == avant => {
                if self.prochaine.is_some_and(|p| t >= p) {
                    l.push(Evenement::Commande(d));
                    self.prochaine = Some(t + REPETITION);
                }
            }
            (Some(_), _) if premier_tour => self.prochaine = None, // tenue à l'ouverture : ne compte pas
            (Some(d), _) => {
                l.push(Evenement::Commande(d));
                self.prochaine = Some(t + PREMIERE_REPETITION);
            }
            (None, _) => self.prochaine = None,
        }
        self.dir = d;
        self.avant = b;
        l
    }
}

/// Lit les manettes XInput (0 à 3). Un emplacement vide n'est réinterrogé qu'une fois par seconde : Microsoft
/// déconseille d'appeler XInputGetState à chaque image sur un emplacement sans manette (coûteux).
#[derive(Default)]
pub struct Lecteur {
    vide_jusqu_a: [Option<Instant>; 4],
    guide: GardeGuide<usize>,
}

/// L'état rendu par `XInputGetStateEx` : celui de `XINPUT_STATE` (16 octets) SUIVI d'un champ réservé (4 octets) — la
/// structure officielle est trop petite pour lui (il écrirait au-delà). Une marge en plus, par prudence ; on ne lit que
/// le début, identique à `XINPUT_STATE`.
#[cfg(windows)]
#[repr(C)]
#[derive(Default)]
struct EtatEx {
    paquet: u32,
    boutons: u16,
    gt: u8,
    dt: u8,
    lx: i16,
    ly: i16,
    rx: i16,
    ry: i16,
    reserve: [u32; 8],
}

#[cfg(windows)]
type LireEx = unsafe extern "system" fn(u32, *mut EtatEx) -> u32;

/// `XInputGetStateEx` (ordinal 100 de xinput1_4.dll), cherché une seule fois ; `None` s'il n'existe pas (on garde
/// alors `XInputGetState` : le bouton Xbox reste muet, tout le reste marche).
#[cfg(windows)]
fn lire_ex() -> Option<LireEx> {
    static F: std::sync::OnceLock<Option<usize>> = std::sync::OnceLock::new();
    let f = *F.get_or_init(|| {
        use windows::core::{w, PCSTR};
        use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW};
        let h = unsafe { GetModuleHandleW(w!("xinput1_4.dll")) }.or_else(|_| unsafe { LoadLibraryW(w!("xinput1_4.dll")) }).ok()?;
        unsafe { GetProcAddress(h, PCSTR(100usize as *const u8)) }.map(|p| p as usize)
    });
    // SAFETY : l'adresse vient de GetProcAddress pour cet ordinal, dont la signature est (index, *mut état) -> code.
    f.map(|p| unsafe { std::mem::transmute::<usize, LireEx>(p) })
}

/// Le bouton Xbox est-il lisible sur ce PC (pour le journal) ?
pub fn guide_lisible() -> bool {
    #[cfg(windows)]
    {
        lire_ex().is_some()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

impl Lecteur {
    /// L'état de toutes les manettes branchées : leurs boutons réunis ; le stick de celle qui le pousse le plus.
    pub fn lire(&mut self, t: Instant) -> EtatManette {
        #[allow(unused_mut)]
        let mut e = EtatManette::default();
        #[cfg(windows)]
        {
            use windows::Win32::UI::Input::XboxController::{XInputGetState, XINPUT_STATE};
            let ex = lire_ex();
            for i in 0..4usize {
                if self.vide_jusqu_a[i].is_some_and(|f| t < f) {
                    continue;
                }
                let (code, s) = match ex {
                    Some(f) => {
                        let mut s = EtatEx::default();
                        (unsafe { f(i as u32, &mut s) }, (s.boutons, s.gt, s.dt, s.lx, s.ly))
                    }
                    None => {
                        let mut s = XINPUT_STATE::default();
                        let code = unsafe { XInputGetState(i as u32, &mut s) };
                        let g = s.Gamepad;
                        (code, (g.wButtons.0, g.bLeftTrigger, g.bRightTrigger, g.sThumbLX, g.sThumbLY))
                    }
                };
                if code != 0 {
                    self.vide_jusqu_a[i] = Some(t + Duration::from_secs(1));
                    self.guide.oublier(i);
                    continue;
                }
                self.vide_jusqu_a[i] = None;
                let (boutons, gt, dt, lx, ly) = s;
                e.boutons |= self.guide.filtrer(i, boutons);
                e.gt = e.gt.max(gt);
                e.dt = e.dt.max(dt);
                let force = |x: i16, y: i16| (x as i32).abs() + (y as i32).abs();
                if force(lx, ly) > force(e.lx, e.ly) {
                    e.lx = lx;
                    e.ly = ly;
                }
            }
        }
        let _ = t;
        // Les autres manettes (PlayStation, Switch, 8BitDo…), lues par SDL3 dans son propre fil.
        crate::manettes_sdl::reunir(e, crate::manettes_sdl::etat())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(boutons: u16) -> EtatManette {
        EtatManette { boutons, ..Default::default() }
    }

    #[test]
    fn select_r1_tenus_une_seconde_ouvrent_le_menu_une_seule_fois() {
        let mut o = Osd::nouveau(combinaison("Select + R1"));
        let t0 = Instant::now();
        assert!(o.pas(e(SELECT | R1), t0, false).is_empty());
        assert!(o.pas(e(SELECT | R1), t0 + Duration::from_millis(600), false).is_empty(), "pas avant 1 s");
        assert_eq!(o.pas(e(SELECT | R1), t0 + Duration::from_millis(1000), false), [Evenement::Ouvrir]);
        assert!(o.pas(e(SELECT | R1), t0 + Duration::from_millis(3000), false).is_empty(), "toujours tenue : une seule fois");
        // Relâchée puis reprise : à nouveau 1 s.
        o.pas(e(0), t0 + Duration::from_millis(3100), false);
        assert!(o.pas(e(SELECT | R1), t0 + Duration::from_millis(3200), false).is_empty());
        assert_eq!(o.pas(e(SELECT | R1), t0 + Duration::from_millis(4200), false), [Evenement::Ouvrir]);
    }

    #[test]
    fn un_appui_bref_ou_un_seul_bouton_n_ouvre_rien() {
        let mut o = Osd::nouveau(combinaison("Select + R1"));
        let t0 = Instant::now();
        o.pas(e(SELECT | R1), t0, false);
        o.pas(e(0), t0 + Duration::from_millis(500), false);
        assert!(o.pas(e(SELECT | R1), t0 + Duration::from_millis(1200), false).is_empty(), "relâchée : on recompte");
        let mut o = Osd::nouveau(combinaison("Select + R1"));
        o.pas(e(SELECT), t0, false);
        assert!(o.pas(e(SELECT), t0 + Duration::from_secs(5), false).is_empty(), "Select seul (jeu) : rien");
        let mut o = Osd::nouveau(combinaison("Select + R1"));
        o.pas(e(SELECT | START), t0, false);
        assert!(o.pas(e(SELECT | START), t0 + Duration::from_secs(5), false).is_empty(), "Select + Start (jeu) : rien");
    }

    #[test]
    fn menu_ouvert_la_croix_a_et_b_deviennent_des_commandes() {
        let mut o = Osd::nouveau(combinaison("Select + R1"));
        let t0 = Instant::now();
        // La combinaison est encore tenue quand le menu s'ouvre : rien.
        assert!(o.pas(e(SELECT | R1), t0, true).is_empty());
        o.pas(e(0), t0 + Duration::from_millis(50), true);
        assert_eq!(o.pas(e(BAS), t0 + Duration::from_millis(100), true), [Evenement::Commande("bas")]);
        assert!(o.pas(e(BAS), t0 + Duration::from_millis(200), true).is_empty(), "pas encore la répétition");
        assert_eq!(o.pas(e(BAS), t0 + Duration::from_millis(500), true), [Evenement::Commande("bas")], "répétition");
        o.pas(e(0), t0 + Duration::from_millis(550), true);
        // A et B agissent au RELÂCHEMENT (le bouton qui ferme le menu n'arrive pas au jeu).
        assert!(o.pas(e(A), t0 + Duration::from_millis(600), true).is_empty(), "A enfoncé : pas encore");
        assert!(o.pas(e(A), t0 + Duration::from_millis(700), true).is_empty());
        assert_eq!(o.pas(e(0), t0 + Duration::from_millis(750), true), [Evenement::Commande("valider")]);
        o.pas(e(B), t0 + Duration::from_millis(800), true);
        assert_eq!(o.pas(e(0), t0 + Duration::from_millis(850), true), [Evenement::Commande("retour")]);
        // Le stick gauche.
        let haut = EtatManette { ly: 30_000, ..Default::default() };
        assert_eq!(o.pas(haut, t0 + Duration::from_millis(900), true), [Evenement::Commande("haut")]);
    }

    #[test]
    fn lb_et_rb_passent_d_un_bouton_a_l_autre_jamais_ceux_de_la_combinaison() {
        let mut o = Osd::nouveau(combinaison("Select + R1"));
        let t0 = Instant::now();
        // R1 tenu à l'ouverture (la combinaison) : rien, ni tout de suite ni tenu.
        assert!(o.pas(e(SELECT | R1), t0, true).is_empty());
        assert!(o.pas(e(R1), t0 + Duration::from_millis(50), true).is_empty());
        o.pas(e(0), t0 + Duration::from_millis(100), true);
        // Puis R1 et L1 : une commande à l'appui, une seule même tenus.
        assert_eq!(o.pas(e(R1), t0 + Duration::from_millis(150), true), [Evenement::Commande("suivant")]);
        assert!(o.pas(e(R1), t0 + Duration::from_millis(900), true).is_empty(), "tenu : une seule fois");
        o.pas(e(0), t0 + Duration::from_millis(950), true);
        assert_eq!(o.pas(e(L1), t0 + Duration::from_millis(1000), true), [Evenement::Commande("precedent")]);
    }

    #[test]
    fn vue_et_une_gachette_tenues_sauvent_ou_chargent_une_seule_fois() {
        let g = |boutons: u16, gt: u8, dt: u8| EtatManette { boutons, gt, dt, ..Default::default() };
        let mut o = Osd::nouveau(combinaison("Select + R1")).avec_raccourcis(true);
        let t0 = Instant::now();
        assert!(o.pas(g(SELECT, 255, 0), t0, false).is_empty(), "pas encore 0,5 s");
        assert_eq!(o.pas(g(SELECT, 255, 0), t0 + Duration::from_millis(520), false), [Evenement::Raccourci("sauver")]);
        assert!(o.pas(g(SELECT, 255, 0), t0 + Duration::from_millis(1500), false).is_empty(), "tenus : une seule fois");
        o.pas(g(0, 0, 0), t0 + Duration::from_millis(1600), false);
        // Charger : 1 s.
        assert!(o.pas(g(SELECT, 0, 200), t0 + Duration::from_millis(1700), false).is_empty());
        assert!(o.pas(g(SELECT, 0, 200), t0 + Duration::from_millis(2300), false).is_empty(), "0,6 s : pas encore");
        assert_eq!(o.pas(g(SELECT, 0, 200), t0 + Duration::from_millis(2710), false), [Evenement::Raccourci("charger")]);
        o.pas(g(0, 0, 0), t0 + Duration::from_millis(2800), false);
        // Les deux gâchettes, une gâchette à peine enfoncée, ou sans le réglage : rien.
        o.pas(g(SELECT, 255, 255), t0 + Duration::from_millis(2900), false);
        assert!(o.pas(g(SELECT, 255, 255), t0 + Duration::from_millis(4000), false).is_empty());
        o.pas(g(0, 0, 0), t0 + Duration::from_millis(4100), false);
        o.pas(g(SELECT, 60, 0), t0 + Duration::from_millis(4200), false);
        assert!(o.pas(g(SELECT, 60, 0), t0 + Duration::from_millis(5000), false).is_empty());
        let mut sans = Osd::nouveau(combinaison("Select + R1"));
        sans.pas(g(SELECT, 255, 0), t0, false);
        assert!(sans.pas(g(SELECT, 255, 0), t0 + Duration::from_secs(2), false).is_empty(), "réglage coupé");
        // Menu ouvert : jamais.
        let mut m = Osd::nouveau(combinaison("Select + R1")).avec_raccourcis(true);
        m.pas(g(SELECT, 255, 0), t0, true);
        assert!(!m.pas(g(SELECT, 255, 0), t0 + Duration::from_secs(2), true).iter().any(|e| matches!(e, Evenement::Raccourci(_))));
    }

    #[test]
    fn le_bouton_xbox_court_et_seul_ouvre_au_relachement() {
        let mut o = Osd::nouveau(combinaison("Select + R1")).avec_guide(true);
        let t0 = Instant::now();
        o.pas(e(0), t0, false);
        assert!(o.pas(e(GUIDE), t0 + Duration::from_millis(20), false).is_empty(), "enfoncé : pas encore");
        assert_eq!(o.pas(e(0), t0 + Duration::from_millis(200), false), [Evenement::Ouvrir]);
        assert!(o.pas(e(0), t0 + Duration::from_millis(300), false).is_empty(), "une seule fois");
        // Tenu plus d'une seconde (éteindre la manette) : rien.
        o.pas(e(GUIDE), t0 + Duration::from_millis(400), false);
        assert!(o.pas(e(0), t0 + Duration::from_millis(1500), false).is_empty(), "appui long");
        // Avec un autre bouton ou une gâchette (combinaison d'un autre programme) : rien.
        o.pas(e(GUIDE), t0 + Duration::from_millis(1600), false);
        o.pas(e(GUIDE | A), t0 + Duration::from_millis(1650), false);
        o.pas(e(GUIDE), t0 + Duration::from_millis(1700), false);
        assert!(o.pas(e(0), t0 + Duration::from_millis(1750), false).is_empty(), "Guide + A");
        o.pas(e(GUIDE), t0 + Duration::from_millis(1800), false);
        o.pas(EtatManette { boutons: GUIDE, dt: 255, ..Default::default() }, t0 + Duration::from_millis(1850), false);
        assert!(o.pas(e(0), t0 + Duration::from_millis(1900), false).is_empty(), "Guide + RT");
    }

    #[test]
    fn le_bouton_xbox_sans_le_reglage_ou_tenu_au_lancement_n_ouvre_rien() {
        let t0 = Instant::now();
        let mut sans = Osd::nouveau(combinaison("Select + R1"));
        sans.pas(e(0), t0, false);
        sans.pas(e(GUIDE), t0 + Duration::from_millis(20), false);
        assert!(sans.pas(e(0), t0 + Duration::from_millis(100), false).is_empty(), "réglage coupé");
        // Tenu dès le premier tour (le lancement par Moonlight, par exemple) : son relâchement n'ouvre rien.
        let mut o = Osd::nouveau(combinaison("Select + R1")).avec_guide(true);
        o.pas(e(GUIDE), t0, false);
        assert!(o.pas(e(0), t0 + Duration::from_millis(100), false).is_empty());
    }

    #[test]
    fn menu_ouvert_le_bouton_xbox_rend_la_main_au_jeu() {
        let mut o = Osd::nouveau(combinaison("Select + R1")).avec_guide(true);
        let t0 = Instant::now();
        o.pas(e(0), t0, false);
        o.pas(e(GUIDE), t0 + Duration::from_millis(20), false);
        assert_eq!(o.pas(e(0), t0 + Duration::from_millis(100), false), [Evenement::Ouvrir]);
        assert!(o.pas(e(0), t0 + Duration::from_millis(150), true).is_empty());
        assert!(o.pas(e(GUIDE), t0 + Duration::from_millis(200), true).is_empty(), "enfoncé : pas encore");
        assert_eq!(o.pas(e(0), t0 + Duration::from_millis(250), true), [Evenement::Commande("reprendre")]);
        // Tenu au moment de l'ouverture (ouvert par la combinaison) : son relâchement ne ferme rien.
        let mut m = Osd::nouveau(combinaison("Select + R1")).avec_guide(true);
        m.pas(e(0), t0, false);
        m.pas(e(GUIDE), t0 + Duration::from_millis(20), true);
        assert!(m.pas(e(0), t0 + Duration::from_millis(80), true).is_empty());
        // Sans le réglage : rien.
        let mut s = Osd::nouveau(combinaison("Select + R1"));
        s.pas(e(0), t0, true);
        s.pas(e(GUIDE), t0 + Duration::from_millis(20), true);
        assert!(s.pas(e(0), t0 + Duration::from_millis(80), true).is_empty());
    }

    #[test]
    fn le_bouton_xbox_ne_gene_ni_la_combinaison_ni_les_raccourcis() {
        let mut o = Osd::nouveau(combinaison("Select + R1")).avec_guide(true).avec_raccourcis(true);
        let t0 = Instant::now();
        o.pas(e(0), t0, false);
        o.pas(e(SELECT | R1), t0 + Duration::from_millis(10), false);
        assert_eq!(o.pas(e(SELECT | R1), t0 + Duration::from_millis(1010), false), [Evenement::Ouvrir]);
        o.pas(e(0), t0 + Duration::from_millis(1100), false);
        let g = EtatManette { boutons: SELECT, gt: 255, ..Default::default() };
        o.pas(g, t0 + Duration::from_millis(1200), false);
        assert_eq!(o.pas(g, t0 + Duration::from_millis(1720), false), [Evenement::Raccourci("sauver")]);
    }

    #[test]
    fn le_bouton_xbox_seulement_sans_game_bar_et_hors_des_emulateurs_qui_s_en_servent() {
        use Moteur::*;
        assert!(guide_permis(Some(false), true, Emulateur("retroarch")));
        assert!(guide_permis(Some(false), true, JeuPc));
        assert!(!guide_permis(Some(true), true, Emulateur("retroarch")), "la Game Bar le prend");
        assert!(!guide_permis(None, true, Emulateur("retroarch")), "hors de Windows");
        assert!(!guide_permis(Some(false), false, Emulateur("retroarch")), "réglage coupé");
        assert!(!guide_permis(Some(false), true, Emulateur("rpcs3")), "le bouton PS de RPCS3");
        assert!(!guide_permis(Some(false), true, Emulateur("xemu")));
        assert!(!guide_permis(Some(false), true, Emulateur("xenia")));
        assert!(!guide_permis(Some(false), true, Inconnu), "émulateur hors catalogue");
    }

    #[test]
    fn une_manette_allumee_par_le_bouton_xbox_n_ouvre_pas_le_menu() {
        let mut g = GardeGuide::<usize>::default();
        assert_eq!(g.filtrer(0, GUIDE | A), A, "première lecture, Guide tenu (allumage) : masqué");
        assert_eq!(g.filtrer(0, GUIDE), 0, "toujours tenu : masqué");
        assert_eq!(g.filtrer(0, 0), 0);
        assert_eq!(g.filtrer(0, GUIDE), GUIDE, "vu relâché : le prochain appui compte");
        assert_eq!(g.filtrer(1, GUIDE), 0, "une autre manette : sa propre garde");
        g.oublier(0);
        assert_eq!(g.filtrer(0, GUIDE), 0, "réapparue : de nouveau masqué");
        // Bout à bout avec le menu : rien ne s'ouvre au relâchement de l'allumage.
        let mut o = Osd::nouveau(combinaison("Select + R1")).avec_guide(true);
        let mut g = GardeGuide::<usize>::default();
        let t0 = Instant::now();
        o.pas(e(0), t0, false);
        assert!(o.pas(e(g.filtrer(2, GUIDE)), t0 + Duration::from_millis(20), false).is_empty());
        assert!(o.pas(e(g.filtrer(2, 0)), t0 + Duration::from_millis(200), false).is_empty(), "allumage : rien");
        o.pas(e(g.filtrer(2, GUIDE)), t0 + Duration::from_millis(400), false);
        assert_eq!(o.pas(e(g.filtrer(2, 0)), t0 + Duration::from_millis(500), false), [Evenement::Ouvrir]);
    }

    #[test]
    fn a_tenu_a_l_ouverture_ne_valide_rien() {
        let mut o = Osd::nouveau(combinaison("Select + R1"));
        let t0 = Instant::now();
        o.pas(e(A), t0, false); // A tenu dans le jeu
        assert!(o.pas(e(A), t0 + Duration::from_millis(50), true).is_empty(), "le menu s'ouvre au clavier, A était tenu");
        assert!(o.pas(e(0), t0 + Duration::from_millis(100), true).is_empty(), "son relâchement ne valide rien non plus");
    }

    #[test]
    fn une_direction_tenue_a_l_ouverture_ne_deplace_rien() {
        let mut o = Osd::nouveau(combinaison("Select + R1"));
        let t0 = Instant::now();
        o.pas(e(BAS), t0, false);
        assert!(o.pas(e(BAS), t0 + Duration::from_millis(20), true).is_empty());
        assert!(o.pas(e(BAS), t0 + Duration::from_millis(900), true).is_empty(), "ni répétée");
        o.pas(e(0), t0 + Duration::from_millis(950), true);
        assert_eq!(o.pas(e(BAS), t0 + Duration::from_millis(1000), true), [Evenement::Commande("bas")]);
    }

    #[test]
    fn les_combinaisons_reglables() {
        assert_eq!(combinaison(""), SELECT | R1);
        assert_eq!(combinaison("Select + R1"), SELECT | R1);
        assert_eq!(combinaison("Select + L1"), SELECT | L1);
        assert_eq!(combinaison("L3 + R3"), L3 | R3);
        assert_eq!(combinaison("Select + Start"), SELECT | START);
    }
}

#[cfg(test)]
mod essai_reel {
    /// `cargo test --lib lire_la_vraie_manette -- --ignored --nocapture` : 3 secondes de lecture (appuie sur des
    /// boutons) ; affiche chaque changement.
    #[test]
    #[ignore]
    fn lire_la_vraie_manette() {
        let fin = std::time::Instant::now() + std::time::Duration::from_secs(3);
        let mut avant = super::EtatManette { boutons: 0xFFFF, ..Default::default() };
        let mut l = super::Lecteur::default();
        while std::time::Instant::now() < fin {
            let e = l.lire(std::time::Instant::now());
            if e.boutons != avant.boutons {
                println!("MANETTE boutons={:#06x} lx={} ly={} (bouton Xbox lisible : {})", e.boutons, e.lx, e.ly, super::guide_lisible());
                avant = e;
            }
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    }
}
