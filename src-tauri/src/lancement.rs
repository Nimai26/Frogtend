//! Lancer un jeu et le suivre jusqu'à sa fermeture.
//!
//! Un jeu se lance souvent par un intermédiaire (un `.bat` qui ouvre DOSBox, un raccourci…) qui se ferme aussitôt.
//! Le jeu est donc « en cours » tant qu'il reste un processus DESCENDANT de celui qu'on a lancé, ou un processus
//! dont le programme est dans le dossier du jeu.

use crate::erreurs::{Erreur, Resultat};
use crate::installation::Lanceur;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use sysinfo::{Pid, ProcessesToUpdate, System};

/// Un petit outil en ligne de commande de Windows (reg, cmd, powershell) lancé SANS fenêtre de console : sans cela,
/// une fenêtre noire s'ouvre et se ferme à chaque appel (vu par Seb le 03/10 en ouvrant « Importer »).
pub fn outil_sans_fenetre(programme: &str) -> std::process::Command {
    #[allow(unused_mut)]
    let mut c = std::process::Command::new(programme);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    c
}

/// `cmd.exe` et `explorer.exe` pris dans Windows lui-même (`%SystemRoot%`), jamais cherchés d'abord dans le dossier
/// de Frogtend ou le dossier courant (durcissement) ; tout autre programme tel quel.
pub fn programme_systeme(programme: &str) -> std::path::PathBuf {
    let w = std::env::var_os("SystemRoot").map(std::path::PathBuf::from);
    match (programme.to_ascii_lowercase().as_str(), w) {
        ("cmd.exe", Some(w)) => w.join("System32").join("cmd.exe"),
        ("explorer.exe", Some(w)) => w.join("explorer.exe"),
        _ => std::path::PathBuf::from(programme),
    }
}

/// Démarre le jeu sans l'attendre. Rend le numéro du processus.
pub fn demarrer(l: &Lanceur) -> Resultat<u32> {
    let mut c = std::process::Command::new(programme_systeme(&l.programme));
    c.args(&l.arguments);
    if !l.dossier.is_empty() {
        c.current_dir(&l.dossier);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Pas de fenêtre de console pour un .bat lancé par cmd : seulement le jeu.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        if l.programme.eq_ignore_ascii_case("cmd.exe") {
            c.creation_flags(CREATE_NO_WINDOW);
        }
    }
    let enfant = c.spawn().map_err(|e| {
        if e.raw_os_error() == Some(740) {
            Erreur::Refus("Ce jeu demande les droits administrateur : Frogtend ne les prend pas tout seul.".into())
        } else {
            Erreur::Disque(format!("Impossible de lancer « {} » ({e}).", l.programme))
        }
    })?;
    noter_depart(enfant.id());
    Ok(enfant.id())
}

/// L'heure de démarrage (secondes) de chaque partie lancée par Frogtend : même fermé ou repris par un autre programme
/// (Windows réutilise les numéros), le numéro d'une partie reste daté.
static DEPARTS: std::sync::Mutex<Vec<(u32, u64)>> = std::sync::Mutex::new(Vec::new());

fn maintenant_s() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Note le départ d'une partie (appelé dès qu'elle est lancée) : sa VRAIE heure de démarrage, lue chez Windows (la même
/// que celle des relevés, comparée à l'égalité comme `Suivi`). Déjà fermée : l'instant présent moins 5 s.
pub fn noter_depart(pid: u32) {
    let mut s = System::new();
    let p = Pid::from_u32(pid);
    s.refresh_processes(ProcessesToUpdate::Some(&[p]), true);
    let heure = s.process(p).map(|x| x.start_time()).filter(|t| *t > 0).unwrap_or_else(|| maintenant_s().saturating_sub(5));
    let mut d = DEPARTS.lock().unwrap_or_else(|e| e.into_inner());
    d.retain(|(p, _)| *p != pid);
    d.push((pid, heure));
    // Quelques centaines de parties dans une journée, au plus : on garde les dernières.
    let n = d.len();
    if n > 512 {
        d.drain(..n - 512);
    }
}

fn depart_note(pid: u32) -> Option<u64> {
    DEPARTS.lock().unwrap_or_else(|e| e.into_inner()).iter().rev().find(|(p, _)| *p == pid).map(|(_, t)| *t)
}

/// Le processus lancé et tous ses descendants encore vivants (un lanceur .bat, puis l'émulateur qu'il ouvre…).
pub fn processus_de_la_partie(racine: u32) -> Vec<u32> {
    let mut s = System::new();
    s.refresh_processes(ProcessesToUpdate::All, true);
    let releve: Vec<(u32, Option<u32>, u64)> = s.processes().iter().map(|(pid, p)| (pid.as_u32(), p.parent().map(|x| x.as_u32()), p.start_time())).collect();
    descendants(&releve, racine, depart_note(racine))
}

/// Les descendants VIVANTS de `racine` dans un relevé (numéro, parent noté, heure de démarrage), elle comprise.
/// Windows RÉUTILISE les numéros, et un programme garde le numéro de son parent même quand ce parent est fermé depuis
/// longtemps : un nouveau programme qui reçoit ce numéro « adopterait » alors des programmes sans rapport (vu le 21/10 :
/// la suite de tests s'est fermée elle-même). Un vrai enfant démarre APRÈS son parent : sinon, il n'en est pas un.
/// `depart` : l'heure notée au lancement de la partie (`noter_depart`) — elle garde les enfants d'une racine déjà
/// fermée (le .bat qui a ouvert l'émulateur) et écarte un programme qui aurait repris son numéro depuis.
pub fn descendants(releve: &[(u32, Option<u32>, u64)], racine: u32, depart: Option<u64>) -> Vec<u32> {
    let vivante = releve.iter().find(|(p, _, _)| *p == racine).map(|(_, _, t)| *t);
    // L'heure notée est celle de Windows (`noter_depart`) : une autre heure, c'est un autre programme.
    let reprise = matches!((vivante, depart), (Some(v), Some(d)) if v != d);
    let mut vus: Vec<(u32, u64)> = match (vivante, depart) {
        (Some(t), _) if !reprise => vec![(racine, t)],
        // Fermée, ou son numéro repris par un autre : seulement ses enfants nés après son départ.
        (_, Some(d)) => vec![(racine, d)],
        _ => vec![],
    };
    loop {
        let avant = vus.len();
        for (pid, parent, t) in releve {
            if vus.iter().any(|(v, _)| v == pid) {
                continue;
            }
            if let Some(&(pa, t_parent)) = parent.and_then(|pa| vus.iter().find(|(v, _)| *v == pa)) {
                // Numéro de la racine repris : un enfant né APRÈS l'arrivée du nouveau venu est le sien, pas le nôtre.
                let du_nouveau_venu = reprise && pa == racine && vivante.is_some_and(|v| *t >= v);
                if *t >= t_parent && !du_nouveau_venu {
                    vus.push((*pid, *t));
                }
            }
        }
        if vus.len() == avant {
            break;
        }
    }
    // Seulement ce qui vit encore : une racine fermée (ou reprise par un autre) n'en fait pas partie.
    vus.into_iter().map(|(p, _)| p).filter(|p| releve.iter().any(|(x, _, _)| x == p) && !(*p == racine && (vivante.is_none() || reprise))).collect()
}

/// Frogtend lui-même et les programmes qui l'ont lancé (jusqu'en haut) : jamais terminés, quoi qu'il arrive.
fn intouchables(s: &System) -> HashSet<u32> {
    let mut l = HashSet::new();
    let mut p = Some(Pid::from_u32(std::process::id()));
    let mut avant = u64::MAX;
    while let Some(pid) = p {
        let Some(x) = s.process(pid) else { break };
        // Un « parent » né après son enfant est un numéro repris : la remontée s'arrête là.
        if x.start_time() > avant || !l.insert(pid.as_u32()) {
            break;
        }
        avant = x.start_time();
        p = x.parent();
    }
    l
}

/// `pids` sans Frogtend ni les programmes qui l'ont lancé (Steam, l'explorateur, Sunshine…) : la garde commune à tout
/// ce qui AGIT sur les programmes d'une partie (terminer, fermer ses fenêtres, la figer). Un seul relevé.
pub fn sauf_frogtend(pids: &[u32]) -> Vec<u32> {
    let mut s = System::new();
    s.refresh_processes(ProcessesToUpdate::All, true);
    a_terminer(pids, &intouchables(&s))
}

/// Ce qui peut être terminé dans `pids` : jamais ce qui est dans `jamais`.
fn a_terminer(pids: &[u32], jamais: &HashSet<u32>) -> Vec<u32> {
    pids.iter().copied().filter(|p| !jamais.contains(p)).collect()
}

/// Les programmes d'un jeu PC à figer : TOUS les processus dont le programme est dans `dossier`, qu'ils descendent ou
/// non de ce que Frogtend a lancé (un raccourci ouvert par l'explorateur, un jeu relancé par sa boutique) — jamais
/// l'explorateur ni la boutique, hors du dossier. Un seul relevé des processus.
/// Ce programme précis est-il ouvert (son exécutable, pas un autre rangé dans le même dossier) ?
pub fn programme_ouvert(programme: &Path) -> bool {
    let cle = programme.to_string_lossy().to_lowercase().replace('/', "\\");
    if cle.is_empty() {
        return false;
    }
    let mut s = System::new();
    s.refresh_processes(ProcessesToUpdate::All, true);
    s.processes().values().any(|p| p.exe().is_some_and(|e| e.to_string_lossy().to_lowercase().replace('/', "\\") == cle))
}

pub fn processus_du_dossier(dossier: &Path) -> Vec<u32> {
    if dossier.parent().is_none() || dossier.as_os_str().is_empty() {
        return vec![];
    }
    let mut s = System::new();
    s.refresh_processes(ProcessesToUpdate::All, true);
    let d = format!("{}\\", dossier.to_string_lossy().trim_end_matches(['\\', '/']).to_lowercase());
    let mut v: Vec<u32> = s
        .processes()
        .iter()
        .filter(|(_, p)| p.exe().is_some_and(|e| e.to_string_lossy().to_lowercase().replace('/', "\\").starts_with(&d)))
        .map(|(pid, _)| pid.as_u32())
        .collect();
    v.sort_unstable();
    v
}

/// Termine ces processus (ceux d'UNE partie, que la personne a demandé de quitter et qui ne se sont pas fermés
/// poliment). Rend le nombre de processus terminés.
pub fn terminer(pids: &[u32]) -> usize {
    let mut s = System::new();
    s.refresh_processes(ProcessesToUpdate::All, true);
    // La même garde que `sauf_frogtend`, sur le relevé gardé pour terminer.
    let jamais = intouchables(&s);
    a_terminer(pids, &jamais).into_iter().filter_map(|p| s.process(Pid::from_u32(p))).filter(|p| p.kill()).count()
}

/// Les processus de cette liste dont le programme est dans `dossier` (sans tenir compte des majuscules).
pub fn processus_dans(pids: &[u32], dossier: &Path) -> Vec<u32> {
    let mut s = System::new();
    s.refresh_processes(ProcessesToUpdate::All, true);
    // Un dossier sans parent (la racine d'un disque) engloberait tout le disque : refusé. Le séparateur final évite que
    // « D:\Emu\RPCS3 » englobe « D:\Emu\RPCS3-ancien ».
    if dossier.parent().is_none() || dossier.as_os_str().is_empty() {
        return vec![];
    }
    let d = format!("{}\\", dossier.to_string_lossy().trim_end_matches(['\\', '/']).to_lowercase());
    pids.iter()
        .copied()
        .filter(|p| {
            s.process(Pid::from_u32(*p))
                .and_then(|x| x.exe())
                .is_some_and(|e| e.to_string_lossy().to_lowercase().replace('/', "\\").starts_with(&d))
        })
        .collect()
}

/// Après « Quitter » (demande polie déjà envoyée) : attend jusqu'à `delai` que l'ÉMULATEUR se ferme de lui-même ;
/// sinon termine ses processus s'ils n'ont TOUS plus aucune fenêtre visible (`abandonne`). Rend le nombre de
/// processus terminés (0 : fermé seul, ou encore une fenêtre, même figée — il enregistre peut-être : on n'y touche pas).
/// Vu le 05/10 chez Seb : RPCS3 ferme la fenêtre du jeu mais son programme reste en vie, caché, « ne répond pas ».
/// Garde-fous (expert lancement, 05/10) : seuls les programmes situés dans le dossier de l'émulateur de la partie
/// (jamais un jeu PC, une boutique relancée par le jeu, ni un programme qui aurait repris un numéro de processus).
pub fn finir_la_partie(racine: u32, dossier_emulateur: &Path, delai: std::time::Duration, abandonne: impl Fn(u32) -> bool) -> usize {
    let fin = std::time::Instant::now() + delai;
    loop {
        let restants = processus_dans(&processus_de_la_partie(racine), dossier_emulateur);
        if restants.is_empty() {
            return 0;
        }
        if std::time::Instant::now() >= fin {
            // Seulement si TOUS ont abandonné : un programme d'aide sans fenêtre n'est pas arrêté pendant que
            // l'émulateur principal, qui a encore sa fenêtre, enregistre peut-être (expert, 05/10).
            return if restants.iter().all(|p| abandonne(*p)) { terminer(&restants) } else { 0 };
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

/// La famille d'Eden après « Quitter » (✅ Seb 15/11 : « souvent le logiciel plante après la fermeture du jeu ») : la
/// fenêtre reste figée (« ne répond pas »). Expert lancement (15/11, code d'Eden) : pendant une fermeture NORMALE, la
/// fenêtre est aussi figée (`emu_thread->wait()`), et la dernière sauvegarde du jeu peut encore partir sur le disque
/// (tampons fermés dans `ShutdownMainProcess`). Donc terminé seulement si, en même temps : au moins `minimum` écoulé,
/// TOUTES ses fenêtres (questions comprises) figées sans interruption depuis 20 s, et plus aucun octet écrit depuis
/// 10 s. Une fenêtre qui répond (une question d'Eden) : jamais touché (✅ Seb 15/11).
pub const FIGE_EDEN: std::time::Duration = std::time::Duration::from_secs(20);
pub const MUET_EDEN: std::time::Duration = std::time::Duration::from_secs(10);

/// La décision, à chaque relevé. `fige` : depuis combien de temps toutes les fenêtres sont figées (`None` : une
/// répond) ; `aucune_fenetre` : il n'en a plus (abandonné, comme les autres émulateurs) ; `muet` : depuis combien de
/// temps il n'a rien écrit.
pub fn eden_a_terminer(ecoule: std::time::Duration, minimum: std::time::Duration, fige: Option<std::time::Duration>, aucune_fenetre: bool, muet: std::time::Duration) -> bool {
    if ecoule < minimum || muet < MUET_EDEN {
        return false;
    }
    aucune_fenetre || fige.is_some_and(|f| f >= FIGE_EDEN)
}

/// Les octets écrits sur le disque par ces processus depuis leur démarrage (somme).
pub fn octets_ecrits(pids: &[u32]) -> u64 {
    let mut s = System::new();
    s.refresh_processes(ProcessesToUpdate::All, true);
    pids.iter().filter_map(|p| s.process(Pid::from_u32(*p))).map(|p| p.disk_usage().total_written_bytes).sum()
}

/// L'attente après « Quitter » de la famille d'Eden, relevé par relevé (pur : testé sans émulateur).
#[derive(Debug, Default)]
pub struct AttenteEden {
    fige_depuis: Option<std::time::Duration>,
    ecrits: Option<u64>,
    dernier_ecrit: std::time::Duration,
    pub premier: u64,
}

impl AttenteEden {
    /// Un relevé à l'instant `t` (depuis le début de l'attente) : les octets écrits (cumul), l'état des fenêtres de
    /// chaque processus (`None` : aucune ; `Some(toutes figées)`), et si Frogtend l'a mis en pause lui-même (menu
    /// rouvert : gelé, il paraîtrait figé et muet). Rend `Some(secondes figées)` s'il faut le terminer.
    pub fn pas(&mut self, t: std::time::Duration, ecrits: u64, etats: &[Option<bool>], en_pause: bool, minimum: std::time::Duration) -> Option<u64> {
        if self.ecrits.is_none() {
            self.premier = ecrits;
        }
        // Une écriture, ou une pause de Frogtend : tout recommence (jamais un émulateur que Frogtend a gelé).
        if self.ecrits != Some(ecrits) || en_pause {
            self.ecrits = Some(ecrits);
            self.dernier_ecrit = t;
        }
        let aucune_fenetre = etats.iter().all(|e| e.is_none());
        // Figé : toutes les fenêtres qui existent ne répondent plus (et il en reste au moins une).
        let tout_fige = !aucune_fenetre && !en_pause && etats.iter().all(|e| e.is_none_or(|f| f));
        if tout_fige {
            self.fige_depuis.get_or_insert(t);
        } else {
            self.fige_depuis = None;
        }
        let fige = self.fige_depuis.map(|f| t.saturating_sub(f));
        let muet = t.saturating_sub(self.dernier_ecrit);
        eden_a_terminer(t, minimum, fige, aucune_fenetre && !en_pause, muet).then(|| fige.map(|f| f.as_secs()).unwrap_or(0))
    }
}

/// Ce qu'a donné l'attente, pour le journal.
#[derive(Debug, PartialEq)]
pub enum FinEden {
    /// Fermé de lui-même.
    Ferme,
    /// Terminé : (processus, secondes figées, octets écrits pendant l'attente).
    Termine(usize, u64, u64),
    /// Toujours là au plafond (une fenêtre répondait, ou il écrivait encore) : laissé tel quel.
    Laisse,
}

/// Après « Quitter » de la famille d'Eden : relevé chaque seconde jusqu'à `plafond` (`AttenteEden::pas`).
pub fn finir_la_partie_eden(
    racine: u32,
    dossier_emulateur: &Path,
    minimum: std::time::Duration,
    plafond: std::time::Duration,
    fenetres: impl Fn(u32) -> Option<bool>,
    en_pause: impl Fn() -> bool,
    avant_d_arreter: impl Fn(),
) -> FinEden {
    let debut = std::time::Instant::now();
    let mut a = AttenteEden::default();
    loop {
        let restants = processus_dans(&processus_de_la_partie(racine), dossier_emulateur);
        if restants.is_empty() {
            return FinEden::Ferme;
        }
        if debut.elapsed() >= plafond {
            return FinEden::Laisse;
        }
        let o = octets_ecrits(&restants);
        let etats: Vec<Option<bool>> = restants.iter().map(|p| fenetres(*p)).collect();
        if let Some(fige) = a.pas(debut.elapsed(), o, &etats, en_pause(), minimum) {
            avant_d_arreter();
            return FinEden::Termine(terminer(&restants), fige, o.saturating_sub(a.premier));
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

#[cfg(test)]
mod tests_eden {
    use super::*;
    use std::time::Duration as D;

    #[test]
    fn eden_n_est_termine_que_fige_longtemps_et_muet() {
        let s = D::from_secs;
        // Une fenêtre répond (une question d'Eden) : jamais.
        assert!(!eden_a_terminer(s(120), s(5), None, false, s(60)));
        // Figé mais encore en train d'écrire (sa dernière sauvegarde) : non.
        assert!(!eden_a_terminer(s(60), s(5), Some(s(40)), false, s(3)));
        // Figé depuis peu (une fermeture normale lente) : non.
        assert!(!eden_a_terminer(s(30), s(5), Some(s(15)), false, s(30)));
        // Figé depuis 20 s et muet depuis 10 s : oui.
        assert!(eden_a_terminer(s(25), s(5), Some(s(20)), false, s(10)));
        // Plus aucune fenêtre : seulement muet depuis 10 s (donc jamais avant 10 s d'attente).
        assert!(eden_a_terminer(s(12), s(5), None, true, s(10)));
        assert!(!eden_a_terminer(s(12), s(5), None, true, s(2)), "encore en train d'écrire");
    }

    /// Rejoue une attente : à chaque seconde, (octets écrits, fenêtres, en pause) ; rend la seconde de l'arrêt.
    fn rejouer(releves: impl Fn(u64) -> (u64, Vec<Option<bool>>, bool), jusqu_a: u64) -> Option<u64> {
        let mut a = AttenteEden::default();
        (0..=jusqu_a).find(|&t| {
            let (o, f, p) = releves(t);
            a.pas(D::from_secs(t), o, &f, p, D::from_secs(5)).is_some()
        })
    }

    #[test]
    fn l_attente_recommence_a_chaque_ecriture_reponse_ou_pause() {
        let fige = vec![Some(true)];
        // Figé dès le début, muet : arrêté à 20 s (figé depuis 20 s, muet depuis 20 s).
        assert_eq!(rejouer(|_| (100, fige.clone(), false), 60), Some(20));
        // Il écrit jusqu'à 30 s (sa dernière sauvegarde) : arrêté 10 s après, à 40 s.
        assert_eq!(rejouer(|t| (100 + t.min(30), fige.clone(), false), 80), Some(40));
        // Il répond un instant à 15 s : les 20 s figées recommencent à 16 s, arrêté à 36 s.
        assert_eq!(rejouer(|t| (100, if t == 15 { vec![Some(false)] } else { fige.clone() }, false), 80), Some(36));
        // Une question d'Eden ouverte (une fenêtre qui répond) : jamais.
        assert_eq!(rejouer(|_| (100, vec![Some(true), Some(false)], false), 300), None);
        // Gelé par le menu de Frogtend de 0 à 50 s : rien pendant la pause, arrêté 20 s après sa fin.
        assert_eq!(rejouer(|t| (100, fige.clone(), t <= 50), 120), Some(71));
        // Plus de fenêtre, muet : comme les autres émulateurs, à 10 s.
        assert_eq!(rejouer(|_| (100, vec![None], false), 60), Some(10));
    }
}

/// Les heures du suivi sont en unités de 100 ns depuis 1970 (celles de Windows, `GetProcessTimes`) : à la SECONDE près
/// (`start_time` de sysinfo), un numéro repris par un autre programme dans la même seconde passait pour le même
/// processus, et un « enfant » né juste après la fermeture d'un processus de la partie passait pour le sien.
const PAR_SECONDE: u64 = 10_000_000;

fn maintenant_precis() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| (d.as_nanos() / 100) as u64).unwrap_or(0)
}

/// Deux heures de création désignent-elles le même processus ? Égales ; ou, quand l'une n'est connue qu'à la seconde
/// (repli : Windows a refusé d'ouvrir le processus), dans la même seconde — jamais moins sûr que l'ancien suivi.
fn meme_heure(a: u64, b: u64) -> bool {
    a == b || flou(a, b)
}

/// Une heure en repli (connue à la seconde) est une seconde ronde : `secondes × 10⁷`. Une heure précise tombant pile sur
/// une seconde ronde (une chance sur dix millions) est traitée comme un repli : jamais moins sûr que l'ancien suivi.
fn repli(a: u64) -> bool {
    a % PAR_SECONDE == 0
}

/// L'ordre de deux heures est-il indécidable ? L'une n'est connue qu'à la seconde, et les deux tombent dans la même
/// seconde : on applique alors l'ancienne règle, à la seconde.
fn flou(a: u64, b: u64) -> bool {
    (repli(a) || repli(b)) && a / PAR_SECONDE == b / PAR_SECONDE
}

/// `a` n'est pas après `b` (à la seconde près si l'une est en repli, comme l'ancien `a <= b` en secondes).
fn pas_apres(a: u64, b: u64) -> bool {
    a <= b || flou(a, b)
}

/// `a` est strictement avant `b` (à la seconde près si l'une est en repli : même seconde = pas avant, comme l'ancien
/// `a < b` en secondes).
fn avant(a: u64, b: u64) -> bool {
    a < b && !flou(a, b)
}

/// Une poignée Windows sur un processus de la partie, gardée tant qu'il vit : Windows ne redonne JAMAIS le numéro d'un
/// processus dont une poignée est ouverte, et elle donne l'heure EXACTE de sa fermeture.
#[cfg(windows)]
struct Poignee(isize);

// La poignée n'est qu'un nombre rendu par Windows, utilisable depuis n'importe quel fil (le suivi passe d'un fil à
// l'autre dans `partie::suivre_partie`).
#[cfg(windows)]
unsafe impl Send for Poignee {}

#[cfg(windows)]
impl Poignee {
    fn ouvrir(pid: u32) -> Option<Poignee> {
        use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE};
        if pid == 0 {
            return None;
        }
        let h = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE, false, pid) }.ok()?;
        Some(Poignee(h.0 as isize))
    }

    /// (création, fin) en 100 ns depuis 1970 ; la fin seulement si le processus est fermé.
    fn heures(&self) -> Option<(u64, Option<u64>)> {
        use windows::Win32::Foundation::{FILETIME, HANDLE, WAIT_OBJECT_0};
        use windows::Win32::System::Threading::{GetProcessTimes, WaitForSingleObject};
        // 1601 → 1970, en 100 ns.
        const ECART: u64 = 116_444_736_000_000_000;
        let h = HANDLE(self.0 as *mut core::ffi::c_void);
        let (mut c, mut x, mut k, mut u) = (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
        unsafe { GetProcessTimes(h, &mut c, &mut x, &mut k, &mut u) }.ok()?;
        let en_100ns = |f: FILETIME| (((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64).saturating_sub(ECART);
        let ferme = unsafe { WaitForSingleObject(h, 0) } == WAIT_OBJECT_0;
        Some((en_100ns(c), ferme.then(|| en_100ns(x))))
    }
}

#[cfg(windows)]
impl Drop for Poignee {
    fn drop(&mut self) {
        let _ = unsafe { windows::Win32::Foundation::CloseHandle(windows::Win32::Foundation::HANDLE(self.0 as *mut core::ffi::c_void)) };
    }
}

/// Hors de Windows : pas de poignée, les heures restent à la seconde (repli).
#[cfg(not(windows))]
struct Poignee;

#[cfg(not(windows))]
impl Poignee {
    fn ouvrir(_pid: u32) -> Option<Poignee> {
        None
    }
    fn heures(&self) -> Option<(u64, Option<u64>)> {
        None
    }
}

/// L'heure de création d'un processus du relevé (`secondes` : celle de sysinfo) : précise si Windows la donne ET
/// qu'elle tombe dans la même seconde (sinon le numéro a changé de mains entre le relevé et la lecture) ; à la seconde
/// sinon. `0` : inconnue.
fn heure_creation(pid: Pid, secondes: u64) -> u64 {
    if secondes == 0 {
        return 0;
    }
    Poignee::ouvrir(pid.as_u32())
        .and_then(|h| h.heures())
        .map(|(c, _)| c)
        .filter(|c| c / PAR_SECONDE == secondes)
        .unwrap_or(secondes.saturating_mul(PAR_SECONDE))
}

/// Suit les processus d'une partie.
pub struct Suivi {
    systeme: System,
    racine: Pid,
    /// Les dossiers « du jeu » : un processus dont le programme y est fait partie de la partie.
    dossiers: Vec<PathBuf>,
    /// Tous les processus vus dans la partie (même si leur parent s'est fermé depuis), avec leur heure de création
    /// (100 ns) : Windows RÉUTILISE les numéros de processus, et un numéro repris par un autre programme (sous forte
    /// charge) ne doit jamais faire croire la partie en cours. `0` : heure inconnue.
    vus: std::collections::HashMap<Pid, u64>,
    /// Les processus de la partie FERMÉS, avec l'heure de leur fermeture (exacte, lue sur leur poignée ; sinon l'instant
    /// où on l'a vu) : un processus fermé ne lance plus rien, donc un « enfant » né après vient d'un autre programme qui
    /// a repris le numéro (rouge le 26/10 et le 08/11 pendant une livraison : la partie jamais finie).
    morts: std::collections::HashMap<Pid, u64>,
    /// Une poignée sur chaque processus de la partie encore vivant (voir [`Poignee`]).
    poignees: std::collections::HashMap<Pid, Poignee>,
    /// Ce qui gardait la partie en cours au dernier relevé (voir [`Suivi::vivants`]).
    derniers_vivants: Vec<String>,
}

/// Ce processus (`numero`, créé à `debut`) est-il un de ceux retenus ? Même numéro ET même heure de création.
fn est_le_meme(vus: &std::collections::HashMap<Pid, u64>, numero: &Pid, debut: u64) -> bool {
    vus.get(numero).is_some_and(|t| *t != 0 && meme_heure(*t, debut))
}

/// Un processus créé à `debut`, dont le parent porte le numéro `parent`, descend-il de la partie ? Le parent retenu
/// toujours vivant (même heure) : oui. Mort : oui si l'enfant est né après lui. Numéro repris par un autre programme
/// (vivant, autre heure) : seulement si l'enfant est né AVANT ce nouveau venu (il vient donc de l'ancien).
#[cfg(test)]
fn descend(vus: &std::collections::HashMap<Pid, u64>, parent: &Pid, debut: u64, parent_vivant: Option<u64>) -> bool {
    descend_avant(vus, &Default::default(), parent, debut, parent_vivant)
}

/// Comme [`descend`], en sachant quand chaque processus de la partie s'est fermé (`morts`) : un parent fermé n'a pu
/// donner que des enfants nés AVANT sa fermeture.
fn descend_avant(
    vus: &std::collections::HashMap<Pid, u64>,
    morts: &std::collections::HashMap<Pid, u64>,
    parent: &Pid,
    debut: u64,
    parent_vivant: Option<u64>,
) -> bool {
    let Some(t) = vus.get(parent).copied() else { return false };
    // Toutes les comparaisons d'ordre tolèrent une heure en repli (à la seconde), comme `meme_heure`.
    let avant_sa_fin = morts.get(parent).is_none_or(|m| pas_apres(debut, *m));
    match parent_vivant {
        Some(v) if t != 0 && meme_heure(v, t) => avant_sa_fin,
        Some(v) => avant(debut, v) && pas_apres(t, debut) && avant_sa_fin,
        None => pas_apres(t, debut) && avant_sa_fin,
    }
}

impl Suivi {
    pub fn nouveau(racine: u32, dossiers: Vec<PathBuf>) -> Self {
        let mut systeme = System::new();
        let r = Pid::from_u32(racine);
        systeme.refresh_processes(ProcessesToUpdate::Some(&[r]), true);
        let mut poignees = std::collections::HashMap::new();
        // Le lanceur déjà fermé au premier relevé (un .bat qui lance le jeu puis rend la main, machine chargée) : son
        // heure est inconnue. Prendre alors l'instant du lancement (moins une marge) : un enfant de la partie est
        // forcément né après — jamais « 0 », qui ferait prendre pour un enfant tout vieux processus orphelin dont le
        // parent mort portait le même numéro (vu le 19/10 : partie jamais finie, test rouge pendant la livraison).
        let debut = match systeme.process(r).map(|p| p.start_time()) {
            Some(s) => {
                // Sa poignée, prise tout de suite : son numéro ne sera plus redonné tant qu'on la garde.
                let h = Poignee::ouvrir(racine);
                let precise = h.as_ref().and_then(|h| h.heures()).map(|(c, _)| c).filter(|c| s != 0 && c / PAR_SECONDE == s);
                if let (Some(_), Some(h)) = (precise, h) {
                    poignees.insert(r, h);
                }
                precise.unwrap_or(s.saturating_mul(PAR_SECONDE))
            }
            None => maintenant_precis().saturating_sub(5 * PAR_SECONDE),
        };
        let vus = std::collections::HashMap::from([(r, debut)]);
        Suivi { systeme, racine: r, dossiers, vus, morts: Default::default(), poignees, derniers_vivants: Vec::new() }
    }

    fn dans_un_dossier(&self, exe: Option<&Path>) -> bool {
        let Some(exe) = exe else { return false };
        let exe = exe.to_string_lossy().to_lowercase().replace('/', "\\");
        // Avec le séparateur final (« …\common\Portal » n'englobe jamais « …\common\Portal 2 ») et jamais la racine
        // d'un disque (elle engloberait tout).
        self.dossiers.iter().any(|d| {
            let racine = d.parent().is_none() || d.as_os_str().is_empty();
            let d = format!("{}\\", d.to_string_lossy().trim_end_matches(['\\', '/']).to_lowercase().replace('/', "\\"));
            !racine && exe.starts_with(&d)
        })
    }

    /// Vrai tant qu'un processus de la partie tourne.
    pub fn en_cours(&mut self) -> bool {
        self.systeme.refresh_processes(ProcessesToUpdate::All, true);
        let processus = self.systeme.processes();
        // L'heure précise n'est lue (une ouverture du processus chez Windows) que pour ceux qui touchent à la partie,
        // une fois par relevé.
        let mut heures: std::collections::HashMap<Pid, u64> = Default::default();
        // Ceux dont on tient la poignée : lus sur elle, sans rouvrir le processus. Encore ouverte = toujours lui (son
        // numéro ne peut pas avoir été redonné) ; fermée = l'heure exacte de sa fin.
        let mut fins: std::collections::HashMap<Pid, Option<u64>> = Default::default();
        for (pid, h) in &self.poignees {
            match h.heures() {
                Some((_, None)) => {
                    if let Some(t) = self.vus.get(pid) {
                        heures.insert(*pid, *t);
                    }
                }
                Some((_, Some(fin))) => {
                    fins.insert(*pid, Some(fin));
                }
                None => {}
            }
        }
        let mut heure = |pid: &Pid, p: &sysinfo::Process| *heures.entry(*pid).or_insert_with(|| heure_creation(*pid, p.start_time()));
        // Ceux de la partie qui viennent de se fermer (ou dont le numéro est repris par un autre) : notés, avec l'heure
        // exacte de leur fermeture quand on tenait leur poignée.
        let maintenant = maintenant_precis();
        for (pid, t) in &self.vus {
            if self.morts.contains_key(pid) {
                continue;
            }
            let ferme = fins.get(pid).copied().flatten();
            if ferme.is_some() || processus.get(pid).is_none_or(|p| !meme_heure(heure(pid, p), *t)) {
                self.morts.insert(*pid, ferme.unwrap_or(maintenant));
                self.poignees.remove(pid);
            }
        }
        // Les descendants : on répète tant qu'on en découvre (petits-enfants…), en reconnaissant chacun à son numéro ET
        // à son heure de création.
        loop {
            let avant = self.vus.len();
            for (pid, p) in processus {
                if self.vus.contains_key(pid) {
                    continue;
                }
                let Some(parent) = p.parent() else { continue };
                if !self.vus.contains_key(&parent) {
                    continue;
                }
                let parent_vivant = processus.get(&parent).map(|x| heure(&parent, x));
                let debut = heure(pid, p);
                if descend_avant(&self.vus, &self.morts, &parent, debut, parent_vivant) {
                    self.vus.insert(*pid, debut);
                    // Sa poignée : seulement si c'est bien lui (même heure de création), sinon on s'en passe.
                    if let Some(h) = Poignee::ouvrir(pid.as_u32()).filter(|h| h.heures().is_some_and(|(c, _)| c == debut)) {
                        self.poignees.insert(*pid, h);
                    }
                }
            }
            if self.vus.len() == avant {
                break;
            }
        }
        let _ = self.racine;
        self.derniers_vivants.clear();
        for (pid, p) in processus {
            let descendant = self.vus.contains_key(pid) && !self.morts.contains_key(pid) && est_le_meme(&self.vus, pid, heure(pid, p));
            if descendant || self.dans_un_dossier(p.exe()) {
                let pourquoi = if descendant { "lancé par la partie" } else { "dans le dossier du jeu" };
                self.derniers_vivants.push(format!("{} ({}, {pourquoi})", p.name().to_string_lossy(), pid.as_u32()));
            }
        }
        !self.derniers_vivants.is_empty()
    }

    /// Les programmes qui faisaient la partie « en cours » au dernier relevé (nom, numéro, pourquoi) : pour le journal,
    /// quand une partie dure (Seb 18/11 : une partie restée « en cours » après la fermeture d'Eden).
    pub fn vivants(&self) -> &[String] {
        &self.derniers_vivants
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn un_numero_reutilise_par_windows_n_adopte_jamais_des_programmes_sans_rapport() {
        // Le cas du 21/10 : la suite de tests (500), lancée il y a longtemps par un programme fermé depuis dont le
        // numéro (77) vient d'être donné au `cmd` d'un essai. Elle « descendait » de 77 : elle s'est fermée elle-même.
        let releve = vec![(77, Some(1), 1000), (78, Some(77), 1001), (500, Some(77), 10), (501, Some(500), 11)];
        assert_eq!(descendants(&releve, 77, None), vec![77, 78], "jamais 500 ni ses enfants");
        // Une partie dont le lanceur (.bat, 77) s'est fermé : l'émulateur qu'il a ouvert (78) reste à elle…
        let releve = vec![(78, Some(77), 1001), (79, Some(78), 1002), (500, Some(77), 10)];
        assert_eq!(descendants(&releve, 77, Some(1000)), vec![78, 79], "les orphelins nés après le départ, pas 500");
        // … mais sans heure de départ connue, une racine fermée n'a plus d'enfants sûrs : rien.
        assert!(descendants(&releve, 77, None).is_empty());
        // Le numéro de la partie (77, partie à 1000) repris par un autre programme (né à 1500) : ni lui, ni ses
        // enfants ; l'émulateur de la partie (78, né à 1001) si.
        let releve = vec![(77, Some(3), 1500), (90, Some(77), 1501), (78, Some(77), 1001)];
        assert_eq!(descendants(&releve, 77, Some(1000)), vec![78]);
    }

    #[test]
    fn frogtend_ne_se_termine_jamais_lui_meme_ni_ce_qui_l_a_lance() {
        let moi = std::process::id();
        let mut s = System::new();
        s.refresh_processes(ProcessesToUpdate::All, true);
        let parent = s.process(Pid::from_u32(moi)).and_then(|p| p.parent()).map(|p| p.as_u32()).unwrap();
        // Jamais le VRAI `terminer` sur la suite de tests ici : si la garde régressait, elle se fermerait sans test rouge.
        let jamais = intouchables(&s);
        assert!(jamais.contains(&moi) && jamais.contains(&parent));
        assert_eq!(a_terminer(&[moi, parent, 4242], &jamais), vec![4242]);
        assert_eq!(sauf_frogtend(&[moi, parent, 4242]), vec![4242], "la garde commune (fermer, figer)");
    }

    #[test]
    fn un_dossier_n_englobe_jamais_son_voisin_ni_une_racine_de_disque() {
        let s = Suivi::nouveau(0, vec![PathBuf::from(r"D:\Steam\steamapps\common\Portal"), PathBuf::from(r"E:\")]);
        assert!(s.dans_un_dossier(Some(Path::new(r"D:\Steam\steamapps\common\Portal\portal.exe"))));
        assert!(s.dans_un_dossier(Some(Path::new(r"d:/steam/steamapps/common/portal/bin/x.exe"))), "majuscules et « / »");
        assert!(!s.dans_un_dossier(Some(Path::new(r"D:\Steam\steamapps\common\Portal 2\portal2.exe"))), "« Portal 2 » n'est pas « Portal »");
        assert!(!s.dans_un_dossier(Some(Path::new(r"E:\autre\jeu.exe"))), "la racine d'un disque n'englobe rien");
    }

    /// Une heure précise (100 ns) : `s` secondes et une fraction quelconque (jamais une seconde ronde, qui serait un repli).
    fn h(s: u64, fraction: u64) -> u64 {
        s * PAR_SECONDE + fraction
    }

    #[test]
    fn un_numero_de_processus_repris_par_un_autre_programme_ne_compte_pas() {
        let p = |n: u32| Pid::from_u32(n);
        let vus = std::collections::HashMap::from([(p(100), h(1000, 1)), (p(200), 0)]);
        // Le même processus (même numéro, même heure) : oui ; le numéro repris par un autre (autre heure) : non.
        assert!(est_le_meme(&vus, &p(100), h(1000, 1)));
        assert!(!est_le_meme(&vus, &p(100), h(1500, 1)), "numéro repris par un autre programme");
        assert!(!est_le_meme(&vus, &p(200), 0), "heure inconnue : jamais vivant");
        // Enfant d'un parent retenu encore vivant : oui.
        assert!(descend(&vus, &p(100), h(1001, 1), Some(h(1000, 1))));
        // Parent mort (le .bat qui s'est fermé) : un enfant né après lui en descend.
        assert!(descend(&vus, &p(100), h(1001, 1), None));
        assert!(!descend(&vus, &p(100), h(999, 1), None), "né avant le parent : impossible");
        // Numéro du parent repris par un autre programme (né à 1500) : ses propres enfants ne sont pas de la partie.
        assert!(!descend(&vus, &p(100), h(1600, 1), Some(h(1500, 1))));
        assert!(descend(&vus, &p(100), h(1200, 1), Some(h(1500, 1))), "né avant le nouveau venu : de l'ancien parent");
        assert!(!descend(&vus, &p(300), h(1600, 1), None), "parent inconnu");
        // Le parent fermé à 1100 : un « enfant » né après (numéro repris par un programme vite refermé) n'est pas
        // de la partie ; né avant, si (lancé juste avant que le parent se ferme).
        let morts = std::collections::HashMap::from([(p(100), h(1100, 1))]);
        assert!(!descend_avant(&vus, &morts, &p(100), h(1200, 1), None));
        assert!(descend_avant(&vus, &morts, &p(100), h(1100, 1), None));
        assert!(!descend_avant(&vus, &morts, &p(100), h(1200, 1), Some(h(1500, 1))), "repris ET né après sa fin");
    }

    #[test]
    fn un_autre_processus_dans_la_meme_seconde_n_est_jamais_pris_pour_celui_de_la_partie() {
        // Ce qu'un suivi à la seconde (`start_time`) ne pouvait pas voir : tout se passe dans la seconde 1000.
        let p = |n: u32| Pid::from_u32(n);
        let vus = std::collections::HashMap::from([(p(100), h(1000, 1_000_000))]);
        // Le numéro 100 repris par un autre programme 0,8 s plus tard, même seconde : ce n'est plus la partie.
        assert!(!est_le_meme(&vus, &p(100), h(1000, 9_000_000)));
        // Ses enfants à lui (nés après lui) ne sont pas de la partie ; un enfant de l'ancien (né avant lui), si.
        assert!(!descend(&vus, &p(100), h(1000, 9_500_000), Some(h(1000, 9_000_000))));
        assert!(descend(&vus, &p(100), h(1000, 5_000_000), Some(h(1000, 9_000_000))));
        // Le processus de la partie fermé à 1000,3 s : un « enfant » né à 1000,6 s (d'un programme qui a repris le
        // numéro et s'est vite refermé) n'est pas de la partie ; né à 1000,2 s, si.
        let morts = std::collections::HashMap::from([(p(100), h(1000, 3_000_000))]);
        assert!(!descend_avant(&vus, &morts, &p(100), h(1000, 6_000_000), None));
        assert!(descend_avant(&vus, &morts, &p(100), h(1000, 2_000_000), None));
        // Une heure connue seulement à la seconde (Windows a refusé d'ouvrir le processus) : même seconde = même
        // processus, comme avant (jamais moins sûr) ; deux heures précises différentes : jamais.
        assert!(meme_heure(h(1000, 0), h(1000, 7_654_321)));
        assert!(meme_heure(h(1000, 7_654_321), h(1000, 0)));
        assert!(!meme_heure(h(1000, 0), h(1001, 1)));
        assert!(!meme_heure(h(1000, 1), h(1000, 2)));
    }

    #[test]
    fn une_heure_en_repli_garde_l_ancienne_regle_a_la_seconde_pour_l_ordre() {
        let p = |n: u32| Pid::from_u32(n);
        // Un enfant en repli (Windows a refusé de l'ouvrir : 1000,0) né dans la même seconde que son parent précis
        // (1000,4) : il descend de lui, comme avec l'ancien suivi (le repli arrondit vers le bas).
        let vus = std::collections::HashMap::from([(p(100), h(1000, 4_000_000))]);
        assert!(descend(&vus, &p(100), h(1000, 0), None));
        // L'inverse : un parent en repli (1000,0), un enfant précis de la même seconde.
        let vus_repli = std::collections::HashMap::from([(p(100), h(1000, 0))]);
        assert!(descend(&vus_repli, &p(100), h(1000, 2_000_000), None));
        // Un parent fermé (fin précise à 1000,6) et un enfant en repli de la même seconde : de la partie.
        let morts = std::collections::HashMap::from([(p(100), h(1000, 6_000_000))]);
        assert!(descend_avant(&vus, &morts, &p(100), h(1000, 0), None));
        // Numéro du parent (né à 999,4) repris par un programme précis (1000,5), enfant en repli de la même seconde que
        // ce nouveau venu : ordre indécidable, l'ancienne règle s'applique (pas « né avant » lui) : jamais adopté.
        let vus_999 = std::collections::HashMap::from([(p(100), h(999, 4_000_000))]);
        assert!(!descend(&vus_999, &p(100), h(1000, 0), Some(h(1000, 5_000_000))));
        assert!(descend(&vus_999, &p(100), h(999, 0), Some(h(1000, 5_000_000))), "repli, mais la seconde d'avant : de l'ancien");
        // Un nouveau venu en repli dans la même seconde que le parent précis : indiscernable de lui (`meme_heure`),
        // comme avec l'ancien suivi.
        assert!(meme_heure(h(1000, 0), h(1000, 4_000_000)));
        // Hors de la même seconde, l'ordre reste strict, repli ou pas.
        assert!(!descend(&vus, &p(100), h(999, 0), None), "né la seconde d'avant le parent");
        assert!(!descend_avant(&vus, &morts, &p(100), h(1001, 0), None), "né la seconde d'après sa fin");
        // Deux heures précises de la même seconde : l'ordre exact décide.
        assert!(!descend(&vus, &p(100), h(1000, 3_999_999), None));
        assert!(pas_apres(h(1000, 0), h(1000, 1)) && pas_apres(h(1000, 1), h(1000, 0)) && !avant(h(1000, 0), h(1000, 1)));
        assert!(avant(h(1000, 1), h(1000, 2)) && !pas_apres(h(1000, 2), h(1000, 1)));
    }

    #[test]
    fn le_suivi_lit_l_heure_precise_et_la_fermeture_exacte_chez_windows() {
        // Un vrai programme (~1 s) : son heure de création est la précise (dans la seconde donnée par sysinfo), et sa
        // fermeture est l'heure exacte lue sur sa poignée — avant l'instant où le suivi l'a vue.
        let mut enfant = std::process::Command::new("ping").args(["-n", "2", "127.0.0.1"]).stdout(std::process::Stdio::null()).spawn().unwrap();
        let pid = enfant.id();
        let mut s = Suivi::nouveau(pid, vec![]);
        let mut sys = System::new();
        sys.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(pid)]), true);
        let secondes = sys.process(Pid::from_u32(pid)).map(|p| p.start_time()).unwrap();
        let t = s.vus[&s.racine];
        assert_eq!(t / PAR_SECONDE, secondes, "la même seconde que sysinfo");
        #[cfg(windows)]
        assert!(s.poignees.contains_key(&s.racine), "sa poignée est tenue (son numéro ne peut pas être redonné)");
        let _ = enfant.wait();
        assert!(!s.en_cours());
        let vu = maintenant_precis();
        let fin = s.morts[&s.racine];
        assert!(fin > t && fin <= vu, "fermeture {fin} entre la création {t} et l'instant vu {vu}");
        assert!(s.poignees.is_empty(), "la poignée est rendue à la fermeture");
    }

    #[test]
    fn un_lanceur_deja_ferme_n_adopte_jamais_un_vieil_orphelin() {
        // Le lanceur a fini avant le premier relevé (sous charge) : son heure n'est pas « 0 » mais l'instant du lancement.
        let s = Suivi::nouveau(u32::MAX - 7, vec![]);
        let maintenant = maintenant_precis();
        let t = s.vus[&s.racine];
        assert!(t > 0 && t + 10 * PAR_SECONDE >= maintenant && t <= maintenant, "{t} / {maintenant}");
        // Un vieux processus orphelin (né il y a une heure) dont le parent mort portait ce numéro : pas de la partie.
        assert!(!descend(&s.vus, &s.racine, maintenant - 3600 * PAR_SECONDE, None));
        // Un enfant né après le lancement : de la partie.
        assert!(descend(&s.vus, &s.racine, maintenant, None));
    }

    #[test]
    fn cmd_et_explorer_sont_pris_dans_windows() {
        if let Some(w) = std::env::var_os("SystemRoot").map(std::path::PathBuf::from) {
            assert_eq!(programme_systeme("CMD.exe"), w.join("System32").join("cmd.exe"));
            assert_eq!(programme_systeme("explorer.exe"), w.join("explorer.exe"));
            assert!(programme_systeme("cmd.exe").is_file());
        }
        assert_eq!(programme_systeme(r"E:\Jeux\Dune\DUNE.EXE"), std::path::PathBuf::from(r"E:\Jeux\Dune\DUNE.EXE"));
    }

    fn attendre_fin(s: &mut Suivi, max: Duration) -> Duration {
        let debut = Instant::now();
        while s.en_cours() && debut.elapsed() < max {
            std::thread::sleep(Duration::from_millis(200));
        }
        debut.elapsed()
    }

    /// Un programme qui resterait 30 s (comme RPCS3 après la fermeture de sa fenêtre de jeu) : ping, dans System32.
    fn programme_qui_reste() -> (std::process::Child, u32, PathBuf) {
        let enfant = std::process::Command::new("ping").args(["-n", "30", "127.0.0.1"]).stdout(std::process::Stdio::null()).spawn().unwrap();
        let pid = enfant.id();
        // Daté comme une vraie partie (`demarrer`) : les essais suivent la même règle que l'application.
        noter_depart(pid);
        let mut s = System::new();
        s.refresh_processes(ProcessesToUpdate::All, true);
        let dossier = s.process(Pid::from_u32(pid)).and_then(|p| p.exe()).and_then(|e| e.parent()).map(PathBuf::from).unwrap();
        (enfant, pid, dossier)
    }

    #[test]
    fn quitter_termine_un_emulateur_reste_en_vie_sans_fenetre() {
        let (mut enfant, pid, dossier) = programme_qui_reste();
        let debut = Instant::now();
        assert_eq!(finir_la_partie(pid, &dossier, Duration::from_millis(1500), |_| true), 1, "terminé après le délai");
        assert!(debut.elapsed() >= Duration::from_millis(1500), "on lui laisse d'abord le temps de se fermer seul");
        let _ = enfant.wait();
        assert!(processus_de_la_partie(pid).is_empty(), "plus rien de la partie");
        // Déjà fermé : rien à terminer, aucune attente.
        let debut = Instant::now();
        assert_eq!(finir_la_partie(pid, &dossier, Duration::from_secs(10), |_| true), 0);
        assert!(debut.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn quitter_ne_termine_jamais_ce_qui_est_hors_du_dossier_ni_un_emulateur_qui_a_encore_sa_fenetre() {
        // Hors du dossier de l'émulateur (un jeu PC, une boutique, un numéro de processus repris) : jamais.
        let (mut enfant, pid, _) = programme_qui_reste();
        let ailleurs = tempfile::tempdir().unwrap();
        assert_eq!(finir_la_partie(pid, ailleurs.path(), Duration::from_millis(300), |_| true), 0);
        assert!(processus_de_la_partie(pid).contains(&pid), "toujours en vie");
        // Dans le dossier, mais il a encore sa fenêtre et répond (il enregistre peut-être) : on n'y touche pas.
        let (mut enfant2, pid2, dossier) = programme_qui_reste();
        assert_eq!(finir_la_partie(pid2, &dossier, Duration::from_millis(300), |_| false), 0);
        assert!(processus_de_la_partie(pid2).contains(&pid2));
        // Deux programmes de l'émulateur, un seul abandonné (un programme d'aide sans fenêtre, l'émulateur qui
        // enregistre encore) : rien n'est arrêté.
        let mut groupe = std::process::Command::new("cmd")
            .args(["/c", "start", "/b", "ping", "-n", "30", "127.0.0.1", "&", "ping", "-n", "30", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let racine = groupe.id();
        std::thread::sleep(Duration::from_millis(500));
        assert!(processus_dans(&processus_de_la_partie(racine), &dossier).len() >= 2);
        assert_eq!(finir_la_partie(racine, &dossier, Duration::from_millis(300), |p| p != racine), 0, "pas tous abandonnés");
        assert!(!processus_de_la_partie(racine).is_empty());
        terminer(&processus_de_la_partie(racine));
        let _ = groupe.wait();
        // Un dossier voisin au nom qui commence pareil (« System32-ancien ») ou la racine d'un disque : jamais.
        let voisin = PathBuf::from(format!("{}-ancien", dossier.display()));
        assert!(processus_dans(&[pid2], &voisin).is_empty());
        assert!(processus_dans(&[pid2], std::path::Path::new("C:\\")).is_empty());
        assert_eq!(processus_dans(&[pid2], &dossier), [pid2]);
        // Les programmes d'un jeu à figer : trouvés par leur dossier, même sans lien avec ce que Frogtend a lancé
        // (un raccourci ouvert par l'explorateur) ; jamais pour la racine d'un disque ni un dossier voisin.
        assert!(processus_du_dossier(&dossier).contains(&pid2));
        assert!(!processus_du_dossier(&dossier).contains(&std::process::id()), "jamais Frogtend (hors du dossier)");
        assert!(processus_du_dossier(std::path::Path::new("C:\\")).is_empty());
        assert!(processus_du_dossier(&voisin).is_empty());
        let _ = enfant.kill();
        let _ = enfant2.kill();
        let _ = (enfant.wait(), enfant2.wait());
    }

    #[test]
    fn un_jeu_lance_par_un_intermediaire_est_suivi_jusqu_a_sa_fin() {
        // Un « .bat » qui lance un programme durant ~2 s puis se ferme aussitôt : c'est le programme qui compte.
        let d = tempfile::tempdir().unwrap();
        let bat = d.path().join("jeu.bat");
        // Le .bat vit encore ~1 s après avoir lancé le programme : le premier relevé connaît alors son heure de démarrage
        // exacte. (Rouge deux fois pendant une livraison, PC chargé, 21/10 : le .bat déjà fermé au premier relevé, son
        // heure prise « à 5 s près », et un processus d'un autre test pris pour un enfant de la partie.)
        std::fs::write(&bat, "@echo off\r\nstart \"\" /b ping -n 3 127.0.0.1 >nul\r\nping -n 2 127.0.0.1 >nul\r\nexit /b 0\r\n").unwrap();
        let l = Lanceur {
            programme: "cmd.exe".into(),
            arguments: vec!["/C".into(), bat.to_string_lossy().into()],
            dossier: d.path().to_string_lossy().into(),
        };
        let pid = demarrer(&l).unwrap();
        let mut s = Suivi::nouveau(pid, vec![]);
        assert!(s.en_cours(), "juste après le lancement, la partie est en cours");
        // 60 s de marge : rouge le 25/10 pendant une suite qui a pris 21,7 s au lieu de 9 (Venkman très chargé) — le
        // programme ne vit que ~2 s, la limite ne sert qu'à voir que la fin est bien vue.
        let duree = attendre_fin(&mut s, Duration::from_secs(60));
        assert!(duree >= Duration::from_millis(1500), "suivi jusqu'à la fin du programme lancé ({duree:?})");
        // Rouge : on dit QUI tenait la partie (numéro, nom, heure de création, parent), pour analyser sans relancer.
        let tiennent: Vec<String> = s
            .systeme
            .processes()
            .iter()
            .filter(|(pid, _)| s.vus.contains_key(*pid) && !s.morts.contains_key(*pid))
            .map(|(pid, p)| format!("{pid} {:?} créé {} (suivi : {}) parent {:?}", p.name(), heure_creation(*pid, p.start_time()), s.vus[pid], p.parent()))
            .collect();
        assert!(duree < Duration::from_secs(60), "la fin est bien vue ; tenaient la partie : {tiennent:?}");
    }

    #[test]
    fn les_processus_de_la_partie_comprennent_ceux_lances_par_le_lanceur() {
        // cmd lance ping : le menu en jeu doit trouver ping (c'est lui qui a la fenêtre, pour un vrai jeu).
        let l = Lanceur {
            programme: "cmd.exe".into(),
            arguments: vec!["/C".into(), "ping -n 4 127.0.0.1 >nul".into()],
            dossier: String::new(),
        };
        let pid = demarrer(&l).unwrap();
        std::thread::sleep(Duration::from_millis(500));
        let p = processus_de_la_partie(pid);
        assert!(p.contains(&pid));
        assert!(p.len() >= 2, "le lanceur et le programme qu'il a ouvert ({p:?})");
        assert!(processus_de_la_partie(u32::MAX - 7).is_empty(), "un processus disparu ne donne rien");
    }

    #[test]
    fn un_programme_introuvable_est_dit() {
        let l = Lanceur { programme: "Z:\\n-existe-pas\\jeu.exe".into(), arguments: vec![], dossier: String::new() };
        assert!(matches!(demarrer(&l), Err(Erreur::Disque(_))));
    }
}
