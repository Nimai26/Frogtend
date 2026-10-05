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

/// Démarre le jeu sans l'attendre. Rend le numéro du processus.
pub fn demarrer(l: &Lanceur) -> Resultat<u32> {
    let mut c = std::process::Command::new(&l.programme);
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
    Ok(enfant.id())
}

/// Le processus lancé et tous ses descendants encore vivants (un lanceur .bat, puis l'émulateur qu'il ouvre…).
pub fn processus_de_la_partie(racine: u32) -> Vec<u32> {
    let mut s = System::new();
    s.refresh_processes(ProcessesToUpdate::All, true);
    let mut vus: HashSet<Pid> = HashSet::from([Pid::from_u32(racine)]);
    loop {
        let avant = vus.len();
        for (pid, p) in s.processes() {
            if p.parent().is_some_and(|parent| vus.contains(&parent)) {
                vus.insert(*pid);
            }
        }
        if vus.len() == avant {
            break;
        }
    }
    vus.into_iter().filter(|p| s.process(*p).is_some()).map(|p| p.as_u32()).collect()
}

/// Les programmes d'un jeu PC à figer : TOUS les processus dont le programme est dans `dossier`, qu'ils descendent ou
/// non de ce que Frogtend a lancé (un raccourci ouvert par l'explorateur, un jeu relancé par sa boutique) — jamais
/// l'explorateur ni la boutique, hors du dossier. Un seul relevé des processus.
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
    pids.iter().filter_map(|p| s.process(Pid::from_u32(*p))).filter(|p| p.kill()).count()
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
/// sinon termine ceux de ses processus qui n'ont plus de fenêtre ou ne répondent plus (`abandonne`). Rend le nombre de
/// processus terminés (0 : fermé seul, ou encore occupé avec sa fenêtre — il enregistre peut-être : on n'y touche pas).
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

/// Suit les processus d'une partie.
pub struct Suivi {
    systeme: System,
    racine: Pid,
    /// Les dossiers « du jeu » : un processus dont le programme y est fait partie de la partie.
    dossiers: Vec<PathBuf>,
    /// Tous les processus vus dans la partie (même si leur parent s'est fermé depuis).
    vus: HashSet<Pid>,
}

impl Suivi {
    pub fn nouveau(racine: u32, dossiers: Vec<PathBuf>) -> Self {
        let mut vus = HashSet::new();
        vus.insert(Pid::from_u32(racine));
        Suivi { systeme: System::new(), racine: Pid::from_u32(racine), dossiers, vus }
    }

    fn dans_un_dossier(&self, exe: Option<&Path>) -> bool {
        let Some(exe) = exe else { return false };
        let exe = exe.to_string_lossy().to_lowercase();
        self.dossiers
            .iter()
            .any(|d| !d.as_os_str().is_empty() && exe.starts_with(&d.to_string_lossy().to_lowercase()))
    }

    /// Vrai tant qu'un processus de la partie tourne.
    pub fn en_cours(&mut self) -> bool {
        self.systeme.refresh_processes(ProcessesToUpdate::All, true);
        let processus = self.systeme.processes();
        // Les descendants : on répète tant qu'on en découvre (petits-enfants…).
        loop {
            let avant = self.vus.len();
            for (pid, p) in processus {
                if p.parent().is_some_and(|parent| self.vus.contains(&parent)) {
                    self.vus.insert(*pid);
                }
            }
            if self.vus.len() == avant {
                break;
            }
        }
        let _ = self.racine;
        processus
            .iter()
            .any(|(pid, p)| self.vus.contains(pid) || self.dans_un_dossier(p.exe()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

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
        std::fs::write(&bat, "@echo off\r\nstart \"\" /b ping -n 3 127.0.0.1 >nul\r\nexit /b 0\r\n").unwrap();
        let l = Lanceur {
            programme: "cmd.exe".into(),
            arguments: vec!["/C".into(), bat.to_string_lossy().into()],
            dossier: d.path().to_string_lossy().into(),
        };
        let pid = demarrer(&l).unwrap();
        let mut s = Suivi::nouveau(pid, vec![]);
        assert!(s.en_cours(), "juste après le lancement, la partie est en cours");
        let duree = attendre_fin(&mut s, Duration::from_secs(20));
        assert!(duree >= Duration::from_millis(1500), "suivi jusqu'à la fin du programme lancé ({duree:?})");
        assert!(duree < Duration::from_secs(20), "la fin est bien vue");
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
