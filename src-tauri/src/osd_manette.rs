//! Le menu en jeu à la manette (lot OSD, étape 3). Pendant une partie, le CŒUR lit les manettes XInput (manettes Xbox
//! et compatibles, manette virtuelle de Sunshine) — la sonde de Seb (04/10) a prouvé qu'elles restent lisibles quand
//! le jeu a le premier plan. La page du menu ne lit pas la manette elle-même : la Gamepad API d'une fenêtre qui vient
//! d'apparaître par-dessus un jeu ne répond pas (essai de Seb, 0.45.0).
//!
//! - Menu fermé : la combinaison (Select + R1 par défaut, décision de Seb du 04/10) tenue 1 s l'ouvre.
//! - Menu ouvert : la croix ou le stick gauche, A et B deviennent des commandes (`haut`, `bas`, `gauche`, `droite`,
//!   `valider`, `retour`) envoyées à la page. Seuls les appuis NOUVEAUX comptent : la combinaison encore tenue qui
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Evenement {
    /// Ouvrir le menu (combinaison tenue).
    Ouvrir,
    /// Une commande pour le menu ouvert.
    Commande(&'static str),
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
}

impl Osd {
    pub fn nouveau(combinaison: u16) -> Self {
        Osd { combinaison, tenue_depuis: None, declenchee: false, avant: 0, dir: None, prochaine: None, etait_ouvert: false, armes: 0 }
    }

    /// Un tour : l'état des manettes, l'heure, et si le menu est ouvert. Rend ce qu'il faut faire.
    pub fn pas(&mut self, e: EtatManette, t: Instant, menu_ouvert: bool) -> Vec<Evenement> {
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
            self.avant = b;
            return l;
        }
        // Menu ouvert : seuls les appuis nouveaux (un bouton tenu à l'ouverture ne compte pas).
        let premier_tour = !self.etait_ouvert;
        self.etait_ouvert = true;
        let nouveaux = if premier_tour { 0 } else { b & !self.avant };
        self.armes |= nouveaux & (A | B);
        let relaches = self.armes & !b;
        self.armes &= !relaches;
        if relaches & A != 0 {
            l.push(Evenement::Commande("valider"));
        }
        if relaches & B != 0 {
            l.push(Evenement::Commande("retour"));
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
}

impl Lecteur {
    /// L'état de toutes les manettes branchées : leurs boutons réunis ; le stick de celle qui le pousse le plus.
    pub fn lire(&mut self, t: Instant) -> EtatManette {
        #[allow(unused_mut)]
        let mut e = EtatManette::default();
        #[cfg(windows)]
        {
            use windows::Win32::UI::Input::XboxController::{XInputGetState, XINPUT_STATE};
            for i in 0..4usize {
                if self.vide_jusqu_a[i].is_some_and(|f| t < f) {
                    continue;
                }
                let mut s = XINPUT_STATE::default();
                if unsafe { XInputGetState(i as u32, &mut s) } != 0 {
                    self.vide_jusqu_a[i] = Some(t + Duration::from_secs(1));
                    continue;
                }
                self.vide_jusqu_a[i] = None;
                let g = s.Gamepad;
                e.boutons |= g.wButtons.0;
                let force = |x: i16, y: i16| (x as i32).abs() + (y as i32).abs();
                if force(g.sThumbLX, g.sThumbLY) > force(e.lx, e.ly) {
                    e.lx = g.sThumbLX;
                    e.ly = g.sThumbLY;
                }
            }
        }
        let _ = t;
        e
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
        let haut = EtatManette { boutons: 0, lx: 0, ly: 30_000 };
        assert_eq!(o.pas(haut, t0 + Duration::from_millis(900), true), [Evenement::Commande("haut")]);
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
        let mut avant = super::EtatManette { boutons: 0xFFFF, lx: 0, ly: 0 };
        let mut l = super::Lecteur::default();
        while std::time::Instant::now() < fin {
            let e = l.lire(std::time::Instant::now());
            if e.boutons != avant.boutons {
                println!("MANETTE boutons={:#06x} lx={} ly={}", e.boutons, e.lx, e.ly);
                avant = e;
            }
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    }
}
