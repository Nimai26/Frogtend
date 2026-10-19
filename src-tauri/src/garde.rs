//! Le durcissement (lot de l'audit du 03/10, ✅ Seb 19/10 : les trois premières étapes) : le cœur ne croit plus
//! l'interface sur parole quand elle lui désigne un programme à LANCER ou un dossier à EFFACER. Il les recompare à ce
//! qui est réglé sur ce PC (`pc.json`, relu par le cœur), à son registre d'émulateurs et à ses jeux.
//!
//! Limite (dite à Seb) : l'interface écrit elle-même `pc.json`. Ces gardes protègent d'une erreur ou d'un bogue de
//! l'interface, pas d'une fenêtre compromise. Seules les NOUVELLES entrées sont vérifiées : ce qui est déjà enregistré
//! n'est jamais refusé après coup.
//!
//! Les chemins sont comparés après normalisation : absolus seulement (lettre ou partage, puis racine), jamais de
//! `..` ni de `.`, sans le préfixe `\\?\`, sans casse (Windows), composant par composant (`D:\Jeux2` n'est pas sous
//! `D:\Jeux`).

use serde_json::Value;
use std::path::{Component, Path, PathBuf, Prefix};

/// Le chemin sous forme comparable : absolu, sans `.` ni `..`, sans `\\?\`, en minuscules, séparé par `\`, sans
/// barre finale (sauf la racine d'un disque, `c:\`). `None` : relatif, ou avec `.`/`..` (refusé, jamais deviné).
pub fn cle(p: &Path) -> Option<String> {
    let mut prefixe = None;
    let mut racine = false;
    let mut morceaux: Vec<String> = Vec::new();
    for c in p.components() {
        match c {
            Component::Prefix(x) => {
                prefixe = Some(match x.kind() {
                    Prefix::Disk(l) | Prefix::VerbatimDisk(l) => format!("{}:", (l as char).to_ascii_lowercase()),
                    Prefix::UNC(s, d) | Prefix::VerbatimUNC(s, d) => {
                        format!(r"\\{}\{}", s.to_string_lossy(), d.to_string_lossy()).to_lowercase()
                    }
                    _ => return None,
                })
            }
            Component::RootDir => racine = true,
            Component::Normal(n) => morceaux.push(n.to_string_lossy().to_lowercase()),
            Component::CurDir | Component::ParentDir => return None,
        }
    }
    let prefixe = prefixe?;
    if !racine {
        return None; // « C:jeux » : relatif au dossier courant de C:
    }
    if morceaux.is_empty() {
        return Some(format!(r"{prefixe}\"));
    }
    Some(format!(r"{prefixe}\{}", morceaux.join(r"\")))
}

/// `enfant` est-il `parent` lui-même ou en dessous ? (Composant par composant, sans casse.)
pub fn sous(enfant: &Path, parent: &Path) -> bool {
    match (cle(enfant), cle(parent)) {
        (Some(e), Some(p)) => {
            let p = p.trim_end_matches('\\');
            e == p || e.starts_with(&format!(r"{p}\")) || (p.ends_with(':') && e.starts_with(&format!(r"{p}\")))
        }
        _ => false,
    }
}

/// Le même chemin ? (Normalisé, sans casse.)
pub fn meme(a: &Path, b: &Path) -> bool {
    matches!((cle(a), cle(b)), (Some(x), Some(y)) if x == y)
}

/// Un point de réparation (lien symbolique, JONCTION) sur ce chemin ? Sous Windows, une jonction n'est pas toujours
/// vue comme un lien : on lit l'attribut de point de réparation.
pub fn est_un_lien(p: &Path) -> bool {
    let Ok(m) = std::fs::symlink_metadata(p) else { return false };
    if m.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const POINT_DE_REPARATION: u32 = 0x400;
        if m.file_attributes() & POINT_DE_REPARATION != 0 {
            return true;
        }
    }
    false
}

/// Un lien ou une jonction entre `racine` (exclue) et `cible` (comprise) ? Un effacement qui le traverserait sortirait
/// du dossier visé : refusé.
pub fn lien_entre(racine: &Path, cible: &Path) -> bool {
    let (Some(r), Some(_)) = (cle(racine), cle(cible)) else { return true };
    let mut courant = PathBuf::new();
    for c in cible.components() {
        courant.push(c);
        let Some(k) = cle(&courant) else { continue };
        if k.len() > r.len() && sous(&courant, racine) && est_un_lien(&courant) {
            return true;
        }
    }
    false
}

/// Un dossier qu'on ne prend JAMAIS pour le dossier d'un jeu : la racine d'un disque, Windows, `Program Files`, le
/// dossier de l'utilisateur ou un de ses dossiers de base.
pub fn dossier_systeme(p: &Path) -> bool {
    if cle(p).is_none() {
        return true;
    }
    // La racine d'un disque (`D:\`) ou d'un partage (`\\serveur\partage`) : aucun dossier après le préfixe.
    if !p.components().any(|c| matches!(c, Component::Normal(_))) {
        return true;
    }
    let env = |n: &str| std::env::var_os(n).map(PathBuf::from);
    let mut systeme: Vec<PathBuf> = ["SystemRoot", "windir", "ProgramFiles", "ProgramFiles(x86)", "ProgramW6432", "ProgramData", "USERPROFILE", "PUBLIC"]
        .iter()
        .filter_map(|n| env(n))
        .collect();
    if let Some(u) = env("USERPROFILE") {
        for d in ["Desktop", "Documents", "Downloads", "AppData", "Music", "Pictures", "Videos"] {
            systeme.push(u.join(d));
        }
    }
    // Windows lui-même : TOUT ce qui est dessous ; les autres : le dossier exactement (un jeu peut être rangé dans
    // `Program Files\Mon jeu` ou `Documents\Jeux`).
    if env("SystemRoot").is_some_and(|w| sous(p, &w)) {
        return true;
    }
    systeme.iter().any(|s| meme(p, s))
}

/// Ce qui est réglé sur ce PC, relu par le cœur dans `pc.json` (clé `reglages`).
#[derive(Debug, Default, Clone)]
pub struct Reglages {
    pub emplacements: Vec<PathBuf>,
    pub programmes: Vec<PathBuf>,
    pub dossiers: Vec<PathBuf>,
}

impl Reglages {
    pub fn lire(reglages: &Value) -> Self {
        let chemin = |v: &Value| v.as_str().filter(|s| !s.trim().is_empty()).map(PathBuf::from);
        let mut emplacements: Vec<PathBuf> = reglages["emplacements"]["defaut"].as_array().into_iter().flatten().filter_map(chemin).collect();
        for v in reglages["emplacements"]["systemes"].as_object().into_iter().flat_map(|o| o.values()) {
            match v {
                Value::Array(l) => emplacements.extend(l.iter().filter_map(chemin)),
                v => emplacements.extend(chemin(v)),
            }
        }
        let programmes = crate::choix_emulateur::tous_les_programmes(reglages).into_iter().filter(|p| !p.trim().is_empty()).map(PathBuf::from).collect();
        let dossiers = ["dossierEmulateurs", "dossierOutils", "dossierAbris"].iter().filter_map(|k| chemin(&reglages[*k])).collect();
        Reglages { emplacements, programmes, dossiers }
    }

    /// Un dossier réglé lui-même (un emplacement, le dossier des émulateurs, des outils, des abris), ou un dossier qui
    /// en CONTIENT un (`E:\` qui contient `E:\Jeux`).
    pub fn est_un_dossier_regle(&self, p: &Path) -> bool {
        self.emplacements.iter().chain(&self.dossiers).any(|d| meme(p, d) || sous(d, p))
    }
}

/// Le dossier des émulateurs ou des outils RÉGLÉ dans `pc.json` (`dossierEmulateurs`, `dossierOutils`) : le seul où
/// un émulateur s'installe (étape 5).
pub fn dossier_de_base_admis(reglages: &Value, dossier: &Path) -> bool {
    ["dossierEmulateurs", "dossierOutils"]
        .iter()
        .filter_map(|k| reglages[*k].as_str().filter(|s| !s.trim().is_empty()))
        .any(|b| meme(dossier, Path::new(b)))
}

/// Un programme de confiance : un émulateur réglé dans `pc.json`, ou inscrit au registre des émulateurs de Frogtend
/// (installé ou adopté), ou trouvé dans son dossier habituel. Le fichier doit exister.
pub fn programme_de_confiance(p: &Path, reglages: &Reglages, connus: &[PathBuf]) -> bool {
    p.is_file() && cle(p).is_some() && reglages.programmes.iter().chain(connus).any(|x| meme(p, x))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_chemins_se_comparent_sans_casse_et_sans_detour() {
        assert_eq!(cle(Path::new(r"D:\Jeux\Dune")).as_deref(), Some(r"d:\jeux\dune"));
        assert_eq!(cle(Path::new(r"\\?\D:\Jeux\Dune\")).as_deref(), Some(r"d:\jeux\dune"));
        assert_eq!(cle(Path::new(r"D:\")).as_deref(), Some(r"d:\"));
        assert_eq!(cle(Path::new(r"\\Serveur\Partage\Jeux")).as_deref(), Some(r"\\serveur\partage\jeux"));
        assert_eq!(cle(Path::new(r"\\?\UNC\Serveur\Partage\Jeux")).as_deref(), Some(r"\\serveur\partage\jeux"));
        // Relatif, ou avec un détour (`..`) : jamais deviné. (Un `.` au milieu est retiré par Rust lui-même : sans
        // danger.)
        for p in [r"Jeux\Dune", r"\Jeux", r"C:Jeux", r"D:\Jeux\..\Windows", r"\\?\D:\Jeux\..\Windows"] {
            assert_eq!(cle(Path::new(p)), None, "{p}");
        }
        assert!(sous(Path::new(r"D:\JEUX\Dune\x.exe"), Path::new(r"d:\jeux")));
        assert!(sous(Path::new(r"D:\Jeux"), Path::new(r"D:\Jeux\")));
        assert!(!sous(Path::new(r"D:\Jeux2\x"), Path::new(r"D:\Jeux")), "composant par composant");
        assert!(!sous(Path::new(r"D:\Jeux\Dune\..\..\Windows"), Path::new(r"D:\Jeux")), "jamais par un détour");
        assert!(sous(Path::new(r"D:\Jeux\x"), Path::new(r"D:\")));
        assert!(meme(Path::new(r"\\?\E:\Emu\RPCS3\rpcs3.exe"), Path::new(r"e:\emu\rpcs3\RPCS3.EXE")));
    }

    #[test]
    fn un_dossier_systeme_n_est_jamais_le_dossier_d_un_jeu() {
        assert!(dossier_systeme(Path::new(r"D:\")));
        assert!(dossier_systeme(Path::new(r"\\Serveur\Partage")), "la racine d'un partage");
        assert!(!dossier_systeme(Path::new(r"\\Serveur\Partage\Jeux\Dune")));
        assert!(dossier_systeme(Path::new(r"Jeux")), "relatif");
        if let Some(w) = std::env::var_os("SystemRoot") {
            assert!(dossier_systeme(Path::new(&w)));
            assert!(dossier_systeme(&Path::new(&w).join("System32")));
        }
        if let Some(u) = std::env::var_os("USERPROFILE") {
            assert!(dossier_systeme(Path::new(&u)));
            assert!(dossier_systeme(&Path::new(&u).join("Documents")));
            assert!(!dossier_systeme(&Path::new(&u).join("Documents").join("Mes jeux").join("Dune")), "un jeu rangé dessous");
        }
        assert!(!dossier_systeme(Path::new(r"E:\Jeux\PC\Dune")));
    }

    #[test]
    fn les_reglages_du_pc_sont_relus_par_le_coeur() {
        let r = Reglages::lire(&serde_json::json!({
            "emplacements": {"defaut": [r"E:\Jeux", ""], "systemes": {"SNES": r"F:\SNES", "PS3": [r"G:\PS3"]}},
            "emulateurs": {"Sony Playstation 3": {"liste": [{"programme": r"E:\Emu\RPCS3\rpcs3.exe", "ligne": ""}]}},
            "dossierEmulateurs": r"E:\Emu", "dossierOutils": "", "dossierAbris": r"E:\Abris"
        }));
        assert!(r.est_un_dossier_regle(Path::new(r"e:\jeux\")));
        assert!(r.est_un_dossier_regle(Path::new(r"F:\SNES")) && r.est_un_dossier_regle(Path::new(r"G:\PS3")));
        assert!(r.est_un_dossier_regle(Path::new(r"E:\Emu")) && r.est_un_dossier_regle(Path::new(r"E:\Abris")));
        assert!(!r.est_un_dossier_regle(Path::new(r"E:\Jeux\Dune")));
        assert!(r.est_un_dossier_regle(Path::new(r"E:\")), "un dossier qui contient un emplacement");
        assert!(r.programmes.iter().any(|p| meme(p, Path::new(r"E:\Emu\RPCS3\rpcs3.exe"))), "{:?}", r.programmes);
    }

    #[test]
    fn un_emulateur_ne_s_installe_que_dans_le_dossier_regle() {
        let r = serde_json::json!({"dossierEmulateurs": r"E:\Emulateurs", "dossierOutils": ""});
        assert!(dossier_de_base_admis(&r, Path::new(r"e:\emulateurs\")));
        assert!(!dossier_de_base_admis(&r, Path::new(r"E:\Emulateurs\RPCS3")), "le dossier réglé lui-même, pas dessous");
        assert!(!dossier_de_base_admis(&r, Path::new(r"C:\Windows")));
        assert!(!dossier_de_base_admis(&r, Path::new("")), "un dossier des outils vide ne vaut rien");
        assert!(dossier_de_base_admis(&serde_json::json!({"dossierOutils": r"F:\Outils"}), Path::new(r"F:\Outils")));
    }

    #[test]
    fn un_programme_de_confiance_est_regle_ou_connu_et_existe() {
        let d = tempfile::tempdir().unwrap();
        let emu = d.path().join("RPCS3").join("rpcs3.exe");
        std::fs::create_dir_all(emu.parent().unwrap()).unwrap();
        std::fs::write(&emu, b"x").unwrap();
        let autre = d.path().join("autre.exe");
        std::fs::write(&autre, b"x").unwrap();
        let r = Reglages { programmes: vec![emu.clone()], ..Default::default() };
        assert!(programme_de_confiance(&emu, &r, &[]));
        assert!(!programme_de_confiance(&autre, &r, &[]), "un programme quelconque");
        assert!(programme_de_confiance(&autre, &Reglages::default(), std::slice::from_ref(&autre)), "connu du registre");
        assert!(!programme_de_confiance(&d.path().join("absent.exe"), &Reglages::default(), &[d.path().join("absent.exe")]));
    }

    #[cfg(windows)]
    #[test]
    fn une_jonction_est_vue_et_bloque_un_effacement() {
        let d = tempfile::tempdir().unwrap();
        let jeu = d.path().join("Jeu");
        let ailleurs = d.path().join("Ailleurs");
        std::fs::create_dir_all(&jeu).unwrap();
        std::fs::create_dir_all(&ailleurs).unwrap();
        let lien = jeu.join("Sauvegardes");
        let ok = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&lien)
            .arg(&ailleurs)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !ok {
            eprintln!("JONCTION impossible ici : test sans objet");
            return;
        }
        assert!(est_un_lien(&lien), "une jonction est un point de réparation");
        eprintln!("JONCTION créée et reconnue");
        assert!(lien_entre(&jeu, &lien.join("fichier.sav")));
        assert!(!lien_entre(d.path(), &jeu));
        let _ = std::fs::remove_dir(&lien);
    }
}
