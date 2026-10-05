//! Figer un jeu PC pendant le menu en jeu (Seb, 05/10 : option par jeu, désactivée d'office ; refusée pour un jeu
//! protégé par un anti-triche). Un jeu PC n'a pas de réglage « pause à la perte du premier plan » comme les
//! émulateurs : Frogtend suspend ses fils d'exécution (API documentée de Windows : instantané Toolhelp,
//! `SuspendThread` / `ResumeThread`) et relance EXACTEMENT ceux qu'il a suspendus (jamais un fil que le jeu avait
//! suspendu lui-même). Seuls les programmes du dossier du jeu sont touchés, jamais Frogtend.
//!
//! Garde-fous (expert lancement, 05/10) : le handle de chaque fil figé reste OUVERT jusqu'au dégel (son numéro ne
//! peut pas être repris par un autre programme, et le dégel ne rouvre rien) ; plusieurs passes attrapent les fils nés
//! pendant le gel ; une NOTE sur le disque permet de relancer le jeu au démarrage suivant si Frogtend a été arrêté
//! brutalement, et un filet de plantage dégèle avant de quitter.
//!
//! Risques connus (dits à la personne avant d'activer) : son qui boucle ou grésille, jeu en ligne déconnecté,
//! anti-triche qui le prend mal (d'où le refus).

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Les anti-triches reconnus par ce qu'ils posent dans le dossier du jeu (début du nom d'un fichier ou d'un dossier,
/// en minuscules) — relevé dans leurs documentations d'intégration (dossier `EasyAntiCheat`, lanceur
/// `start_protected_game.exe`, dossier `BattlEye`, `BEService` / `BEClient`, `PnkBstrA/B` de PunkBuster…). VAC
/// (dans Steam), Riot Vanguard (dans Program Files) et les anti-triches du noyau ne se voient pas ainsi.
const ANTI_TRICHES: &[(&str, &str)] = &[
    ("easyanticheat", "Easy Anti-Cheat"),
    ("start_protected_game", "Easy Anti-Cheat"),
    ("eaanticheat", "EA Anti-Cheat"),
    ("battleye", "BattlEye"),
    ("beservice", "BattlEye"),
    ("beclient", "BattlEye"),
    ("pnkbstr", "PunkBuster"),
    ("punkbuster", "PunkBuster"),
    ("gameguard", "nProtect GameGuard"),
    ("npgamemon", "nProtect GameGuard"),
    ("xigncode", "XIGNCODE3"),
    ("anticheatexpert", "Anti-Cheat Expert"),
    ("equ8", "EQU8"),
    ("mhyprot", "mhyprot"),
    ("hoyokprotect", "HoYoKProtect"),
    ("blackcipher", "BlackCipher"),
    ("hshield", "AhnLab HackShield"),
    ("ehsvc", "AhnLab HackShield"),
];

/// Profondeur et nombre d'entrées lues au plus (on ne lit que des NOMS, jamais le contenu des fichiers).
const PROFONDEUR: usize = 4;
const ENTREES_MAX: usize = 50_000;

/// Ce que la recherche d'un anti-triche a conclu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "sorte", content = "nom", rename_all = "snake_case")]
pub enum Verification {
    Aucun,
    AntiTriche(&'static str),
    /// Trop de fichiers pour tout regarder : on ne sait pas, donc on refuse.
    Inconnu,
}

/// Cherche un anti-triche dans le dossier du jeu.
pub fn anti_triche(dossier: &Path) -> Verification {
    anti_triche_borne(dossier, ENTREES_MAX)
}

fn anti_triche_borne(dossier: &Path, entrees_max: usize) -> Verification {
    let mut a_voir = vec![(dossier.to_path_buf(), 0usize)];
    let mut vues = 0usize;
    while let Some((d, profondeur)) = a_voir.pop() {
        let Ok(entrees) = std::fs::read_dir(&d) else { continue };
        for e in entrees.flatten() {
            vues += 1;
            if vues > entrees_max {
                return Verification::Inconnu;
            }
            let nom = e.file_name().to_string_lossy().to_lowercase();
            if let Some((_, a)) = ANTI_TRICHES.iter().find(|(p, _)| nom.starts_with(p)) {
                return Verification::AntiTriche(a);
            }
            // Les lanceurs BattlEye d'un jeu : « Jeu_BE.exe ».
            if nom.ends_with("_be.exe") {
                return Verification::AntiTriche("BattlEye");
            }
            // `file_type` ne suit pas les liens : un lien vers ailleurs n'est jamais parcouru.
            if profondeur < PROFONDEUR && e.file_type().is_ok_and(|t| t.is_dir()) {
                a_voir.push((e.path(), profondeur + 1));
            }
        }
    }
    Verification::Aucun
}

/// Un fil figé : son processus, son numéro, et son handle gardé ouvert jusqu'au dégel.
#[derive(Debug)]
pub struct FilFige {
    pub pid: u32,
    pub tid: u32,
    handle: usize,
}

/// Les fils figés par le menu (un seul jeu à la fois), et la note qui permet de les relancer après un arrêt brutal.
static FIGES: Mutex<Vec<FilFige>> = Mutex::new(Vec::new());
static NOTE: OnceLock<PathBuf> = OnceLock::new();

/// Au démarrage de Frogtend : relance ce qu'un Frogtend arrêté brutalement aurait laissé figé (d'après la note), puis
/// installe le filet de plantage. Rend le nombre de fils relancés.
pub fn initialiser(note: PathBuf) -> usize {
    let relances = reprendre_apres_arret(&note);
    let _ = NOTE.set(note);
    let precedent = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // `try_lock` : une panique sous le verrou ne doit pas bloquer la sortie.
        if let Ok(mut f) = FIGES.try_lock() {
            relancer(std::mem::take(&mut *f));
            if let Some(n) = NOTE.get() {
                let _ = std::fs::remove_file(n);
            }
        }
        precedent(info);
    }));
    relances
}

/// Fige ces processus si `encore()` est toujours vrai (le menu est encore ouvert) et que rien ne l'est déjà. Le verrou
/// ordonne gel et dégel : appelé après la fermeture du menu, il ne fige rien. Rend le nombre de fils figés.
pub fn geler_si(pids: &[u32], encore: impl Fn() -> bool) -> usize {
    let mut f = FIGES.lock().unwrap_or_else(|e| e.into_inner());
    if !encore() || !f.is_empty() {
        return 0;
    }
    *f = suspendre(pids);
    if let Some(n) = NOTE.get() {
        let _ = ecrire_note(n, &f);
    }
    f.len()
}

/// Relance le jeu figé (rien s'il ne l'est pas).
pub fn degeler() {
    let mut f = FIGES.lock().unwrap_or_else(|e| e.into_inner());
    if f.is_empty() {
        return;
    }
    relancer(std::mem::take(&mut *f));
    if let Some(n) = NOTE.get() {
        let _ = std::fs::remove_file(n);
    }
}

/// Un jeu est-il figé en ce moment ?
pub fn est_fige() -> bool {
    !FIGES.lock().unwrap_or_else(|e| e.into_inner()).is_empty()
}

/// La note : une 1re ligne « frogtend pid création » (le Frogtend qui a figé : s'il vit encore, une autre instance
/// n'y touche pas), puis une ligne par processus, « pid création fil fil… » (création : l'heure de naissance du
/// processus, pour ne jamais relancer les fils d'un autre programme qui aurait repris le même numéro).
fn ecrire_note(note: &Path, fils: &[FilFige]) -> std::io::Result<()> {
    let mut pids: Vec<u32> = fils.iter().map(|f| f.pid).collect();
    pids.sort_unstable();
    pids.dedup();
    let moi = std::process::id();
    let mut texte = format!("frogtend {moi} {}\n", creation(moi).unwrap_or(0));
    for p in pids {
        let Some(c) = creation(p) else { continue };
        let tids: Vec<String> = fils.iter().filter(|f| f.pid == p).map(|f| f.tid.to_string()).collect();
        texte.push_str(&format!("{p} {c} {}\n", tids.join(" ")));
    }
    if let Some(d) = note.parent() {
        std::fs::create_dir_all(d)?;
    }
    std::fs::write(note, texte)
}

/// Relance les fils notés (seulement s'ils appartiennent toujours au même processus, né à la même heure), puis efface
/// la note. Rend le nombre de fils relancés.
pub fn reprendre_apres_arret(note: &Path) -> usize {
    let Ok(texte) = std::fs::read_to_string(note) else { return 0 };
    // Le Frogtend qui a figé le jeu vit encore (une deuxième instance démarre) : c'est à lui de dégeler.
    if let Some(l) = texte.lines().find(|l| l.starts_with("frogtend ")) {
        let mut c = l.split_whitespace().skip(1);
        if let (Some(Ok(pid)), Some(Ok(cr))) = (c.next().map(str::parse::<u32>), c.next().map(str::parse::<u64>)) {
            if cr != 0 && creation(pid) == Some(cr) {
                return 0;
            }
        }
    }
    let mut n = 0;
    for ligne in texte.lines().filter(|l| !l.starts_with("frogtend ")) {
        let mut champs = ligne.split_whitespace();
        let (Some(Ok(pid)), Some(Ok(c))) = (champs.next().map(str::parse::<u32>), champs.next().map(str::parse::<u64>)) else { continue };
        if creation(pid) != Some(c) {
            continue;
        }
        for tid in champs.filter_map(|t| t.parse::<u32>().ok()) {
            if relancer_fil_du_processus(tid, pid) {
                n += 1;
            }
        }
    }
    let _ = std::fs::remove_file(note);
    n
}

#[cfg(windows)]
use windows_impl::{creation, relancer, relancer_fil_du_processus, suspendre};

#[cfg(not(windows))]
fn creation(_: u32) -> Option<u64> {
    None
}
#[cfg(not(windows))]
fn relancer(_: Vec<FilFige>) {}
#[cfg(not(windows))]
fn relancer_fil_du_processus(_: u32, _: u32) -> bool {
    false
}
#[cfg(not(windows))]
pub fn suspendre(_: &[u32]) -> Vec<FilFige> {
    Vec::new()
}

#[cfg(windows)]
mod windows_impl {
    use super::FilFige;
    use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE};
    use windows::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32};
    use windows::Win32::System::Threading::{
        GetProcessIdOfThread, GetProcessTimes, OpenProcess, OpenThread, ResumeThread, SuspendThread, PROCESS_QUERY_LIMITED_INFORMATION,
        THREAD_QUERY_LIMITED_INFORMATION, THREAD_SUSPEND_RESUME,
    };

    /// Les passes de gel au plus (un fil encore actif peut en créer un autre pendant le gel).
    const PASSES: usize = 5;

    fn h(v: usize) -> HANDLE {
        HANDLE(v as *mut core::ffi::c_void)
    }

    /// Les fils (processus, fil) de ces processus.
    fn fils_de(pids: &[u32]) -> Vec<(u32, u32)> {
        let mut v = Vec::new();
        unsafe {
            let Ok(instantane) = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) else { return v };
            let mut e = THREADENTRY32 { dwSize: std::mem::size_of::<THREADENTRY32>() as u32, ..Default::default() };
            let mut ok = Thread32First(instantane, &mut e).is_ok();
            while ok {
                if pids.contains(&e.th32OwnerProcessID) {
                    v.push((e.th32OwnerProcessID, e.th32ThreadID));
                }
                ok = Thread32Next(instantane, &mut e).is_ok();
            }
            let _ = CloseHandle(instantane);
        }
        v
    }

    /// Suspend tous les fils de ces processus (sauf Frogtend), en plusieurs passes.
    pub fn suspendre(pids: &[u32]) -> Vec<FilFige> {
        let moi = std::process::id();
        let pids: Vec<u32> = pids.iter().copied().filter(|p| *p != moi && *p != 0).collect();
        let mut figes: Vec<FilFige> = Vec::new();
        let mut essayes: Vec<u32> = Vec::new();
        for _ in 0..PASSES {
            let nouveaux: Vec<(u32, u32)> = fils_de(&pids).into_iter().filter(|(_, t)| !essayes.contains(t)).collect();
            if nouveaux.is_empty() {
                break;
            }
            for (pid, tid) in nouveaux {
                essayes.push(tid);
                unsafe {
                    let Ok(handle) = OpenThread(THREAD_SUSPEND_RESUME, false, tid) else { continue };
                    // u32::MAX : échec (le fil a pu finir entre-temps).
                    if SuspendThread(handle) != u32::MAX {
                        figes.push(FilFige { pid, tid, handle: handle.0 as usize });
                    } else {
                        let _ = CloseHandle(handle);
                    }
                }
            }
        }
        figes
    }

    /// Relance chaque fil une fois, par son handle gardé, puis le referme.
    pub fn relancer(fils: Vec<FilFige>) {
        for f in fils {
            unsafe {
                let _ = ResumeThread(h(f.handle));
                let _ = CloseHandle(h(f.handle));
            }
        }
    }

    /// Relance un fil noté, seulement s'il appartient toujours à ce processus.
    pub fn relancer_fil_du_processus(tid: u32, pid: u32) -> bool {
        unsafe {
            let Ok(handle) = OpenThread(THREAD_SUSPEND_RESUME | THREAD_QUERY_LIMITED_INFORMATION, false, tid) else { return false };
            let ok = GetProcessIdOfThread(handle) == pid && ResumeThread(handle) != u32::MAX;
            let _ = CloseHandle(handle);
            ok
        }
    }

    /// L'heure de naissance d'un processus (unités de 100 ns), pour le reconnaître.
    pub fn creation(pid: u32) -> Option<u64> {
        unsafe {
            let p = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let (mut c, mut x, mut k, mut u) = (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
            let r = GetProcessTimes(p, &mut c, &mut x, &mut k, &mut u);
            let _ = CloseHandle(p);
            r.ok()?;
            Some(((c.dwHighDateTime as u64) << 32) | c.dwLowDateTime as u64)
        }
    }

    #[cfg(test)]
    pub fn oublier_sans_relancer(fils: Vec<FilFige>) {
        for f in fils {
            unsafe {
                let _ = CloseHandle(h(f.handle));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant};

    fn ping(programme: &str, n: u32) -> Child {
        Command::new(programme).args(["-n", &n.to_string(), "127.0.0.1"]).stdout(Stdio::null()).spawn().unwrap()
    }

    #[test]
    fn un_anti_triche_se_reconnait_a_ses_fichiers() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("Binaries/Win64")).unwrap();
        std::fs::write(d.path().join("Binaries/Win64/Jeu.exe"), b"x").unwrap();
        assert_eq!(anti_triche(d.path()), Verification::Aucun, "un jeu solo ordinaire");
        std::fs::create_dir_all(d.path().join("EasyAntiCheat")).unwrap();
        assert_eq!(anti_triche(d.path()), Verification::AntiTriche("Easy Anti-Cheat"));
        let b = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(b.path().join("Jeu/Binaries")).unwrap();
        std::fs::write(b.path().join("Jeu/Binaries/BEService_x64.exe"), b"x").unwrap();
        assert_eq!(anti_triche(b.path()), Verification::AntiTriche("BattlEye"), "trouvé dans un sous-dossier");
        let l = tempfile::tempdir().unwrap();
        std::fs::write(l.path().join("Jeu_BE.exe"), b"x").unwrap();
        assert_eq!(anti_triche(l.path()), Verification::AntiTriche("BattlEye"), "lanceur BattlEye");
        // Trop profond : pas parcouru (on ne lit jamais tout un disque).
        let p = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(p.path().join("a/b/c/d/e/BattlEye")).unwrap();
        assert_eq!(anti_triche(p.path()), Verification::Aucun);
        assert_eq!(anti_triche(&p.path().join("absent")), Verification::Aucun);
        // Trop de fichiers pour tout voir : « inconnu », jamais « aucun ».
        let gros = tempfile::tempdir().unwrap();
        for i in 0..5 {
            std::fs::write(gros.path().join(format!("f{i}.txt")), b"x").unwrap();
        }
        assert_eq!(anti_triche_borne(gros.path(), 3), Verification::Inconnu);
        assert_eq!(anti_triche_borne(gros.path(), 10), Verification::Aucun);
        assert_eq!(serde_json::to_value(Verification::AntiTriche("BattlEye")).unwrap(), serde_json::json!({ "sorte": "anti_triche", "nom": "BattlEye" }));
        assert_eq!(serde_json::to_value(Verification::Inconnu).unwrap(), serde_json::json!({ "sorte": "inconnu" }));
    }

    #[test]
    fn un_vrai_programme_se_fige_puis_repart() {
        // Un vrai processus (ping, ~2 s) figé 2 s : il finit en plus de 3,5 s (il était bien arrêté), et normalement.
        let debut = Instant::now();
        let mut enfant = ping("ping", 3);
        std::thread::sleep(Duration::from_millis(300));
        let fils = suspendre(&[enfant.id()]);
        assert!(!fils.is_empty(), "au moins un fil suspendu");
        assert!(fils.iter().all(|f| f.pid == enfant.id()));
        assert!(suspendre(&[std::process::id()]).is_empty(), "Frogtend ne se fige jamais lui-même");
        std::thread::sleep(Duration::from_secs(2));
        assert!(enfant.try_wait().unwrap().is_none(), "figé : il ne finit pas");
        relancer(fils);
        assert!(enfant.wait().unwrap().success());
        assert!(debut.elapsed() >= Duration::from_millis(3500), "{:?}", debut.elapsed());
    }

    #[test]
    fn un_programme_32_bits_se_fige_aussi() {
        let wow = r"C:\Windows\SysWOW64\PING.EXE";
        if !Path::new(wow).is_file() {
            return;
        }
        let mut enfant = ping(wow, 3);
        std::thread::sleep(Duration::from_millis(300));
        let fils = suspendre(&[enfant.id()]);
        assert!(!fils.is_empty());
        std::thread::sleep(Duration::from_millis(2500));
        assert!(enfant.try_wait().unwrap().is_none(), "figé");
        relancer(fils);
        assert!(enfant.wait().unwrap().success());
    }

    #[test]
    fn un_programme_deja_suspendu_par_lui_meme_le_reste() {
        use std::os::windows::process::CommandExt;
        // CREATE_SUSPENDED : son fil principal est suspendu avant nous ; le dégel ne doit PAS le relancer.
        let mut enfant = Command::new("ping").args(["-n", "1", "127.0.0.1"]).stdout(Stdio::null()).creation_flags(0x4).spawn().unwrap();
        let fils = suspendre(&[enfant.id()]);
        relancer(fils);
        std::thread::sleep(Duration::from_millis(1500));
        assert!(enfant.try_wait().unwrap().is_none(), "toujours suspendu par son créateur");
        let _ = enfant.kill();
        let _ = enfant.wait();
    }

    #[test]
    fn apres_un_arret_brutal_la_note_relance_le_jeu() {
        let d = tempfile::tempdir().unwrap();
        let note = d.path().join("menu-fige.txt");
        let mut enfant = ping("ping", 3);
        std::thread::sleep(Duration::from_millis(300));
        let fils = suspendre(&[enfant.id()]);
        ecrire_note(&note, &fils).unwrap();
        // Une deuxième instance de Frogtend (le premier, ici ce test, vit encore) n'y touche pas.
        assert_eq!(reprendre_apres_arret(&note), 0);
        assert!(note.exists());
        // Frogtend « meurt » : les handles se ferment, rien n'est relancé. (Sa ligne est retirée de la note, comme
        // si son processus n'existait plus.)
        windows_impl::oublier_sans_relancer(fils);
        let sans_frogtend: String = std::fs::read_to_string(&note).unwrap().lines().filter(|l| !l.starts_with("frogtend ")).map(|l| format!("{l}\n")).collect();
        std::fs::write(&note, sans_frogtend).unwrap();
        std::thread::sleep(Duration::from_millis(2500));
        assert!(enfant.try_wait().unwrap().is_none(), "resté figé");
        // Au démarrage suivant : la note relance le jeu, puis disparaît.
        assert!(reprendre_apres_arret(&note) > 0);
        assert!(!note.exists());
        assert!(enfant.wait().unwrap().success());
        // Une note dont le processus n'existe plus (ou est un autre) ne relance rien.
        std::fs::write(&note, format!("{} 1 1 2 3\n", enfant.id())).unwrap();
        assert_eq!(reprendre_apres_arret(&note), 0);
    }
}

#[cfg(test)]
mod essai_reel_figer {
    /// Sur les vrais dossiers de jeux (lecture des NOMS seulement) :
    /// `FROGTEND_JEUX=<dossier contenant des jeux> cargo test --lib essai_reel_figer -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn anti_triche_sur_les_vrais_jeux() {
        let racine = std::path::PathBuf::from(std::env::var("FROGTEND_JEUX").unwrap());
        for e in std::fs::read_dir(&racine).unwrap().flatten() {
            let debut = std::time::Instant::now();
            let a = super::anti_triche(&e.path());
            println!("JEU {:?} : {:?} ({:?})", e.file_name(), a, debut.elapsed());
        }
    }
}
