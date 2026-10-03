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
