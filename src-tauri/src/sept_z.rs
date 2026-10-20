//! Les archives 7z lues dans un PROCESSUS À PART (lot 0.82.1 ; avis de l'expert lancement, 20/10).
//!
//! La bibliothèque 7z (sevenz-rust 0.6.1) croit l'en-tête de l'archive : les nombres qu'il annonce servent tels quels à
//! réserver la mémoire (`reader.rs` l.347, l.360, l.644-954), le dictionnaire va jusqu'à 4 Gio (`decoders.rs` l.158) et
//! une propriété trop courte la fait paniquer (`decoders.rs` l.174). Frogtend livré s'arrête net sur une panique ou une
//! allocation ratée (`panic = "abort"`) : un 7z abîmé ou piégé le fermerait. Toute lecture de 7z passe donc par un
//! Frogtend ENFANT (le même programme, relancé avec `--frogtend-7z`), enfermé par Windows dans un « job » à la mémoire
//! plafonnée et tué avec son parent. S'il meurt, l'archive est écartée ; Frogtend reste debout.
//!
//! Protocole : l'enfant attend « go » sur son entrée (le temps que le parent l'enferme dans le job), puis écrit sur sa
//! sortie une ligne JSON par événement : `{"e": …}` une entrée, `{"p": n}` des octets écrits, `{"erreur": "…"}` un
//! refus dit en français, et enfin `{"fin": …}`. Seule une ligne `fin` ET un code de sortie 0 valent réussite. Les
//! règles de Frogtend (chemins sûrs, plafonds) restent chez le parent, qui revérifie ce qui est écrit.

use crate::erreurs::{Erreur, Resultat};
use serde_json::{json, Value};
use std::ffi::OsString;
use std::io::{BufRead, Read, Write};
use std::path::Path;

/// L'argument qui fait de Frogtend un lecteur de 7z (jamais une fenêtre).
pub const ARG: &str = "--frogtend-7z";

/// La mémoire au plus de l'enfant : un dictionnaire 7z de 1,5 Gio (le plus grand que 7-Zip propose) et sa marge.
const MEMOIRE_MAX: usize = 2 * 1024 * 1024 * 1024;

/// L'en-tête suivant au plus (quelques dizaines de Mo même avec 100 000 fichiers).
const ENTETE_MAX: u64 = 64 * 1024 * 1024;

/// Entrées d'une archive au plus (comme les mods : `mods::ENTREES_MAX` en vaut 100 000).
const ENTREES_MAX: usize = 1_000_000;

/// Un nom d'entrée au plus (la limite des chemins longs de Windows).
const NOM_MAX: usize = 32_767;

/// Une ligne de l'enfant au plus.
const LIGNE_MAX: usize = 64 * 1024;

const SIGNATURE: [u8; 6] = [0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C];

const PIEGEE: &str = "L'archive 7z est abîmée ou piégée : Frogtend l'a écartée.";

/// Contrôle les 32 premiers octets (signature, CRC de l'en-tête de départ, en-tête suivant DANS le fichier et de taille
/// raisonnable) avant même de lancer l'enfant.
pub fn verifier_entete(archive: &Path) -> Resultat<()> {
    let mut f = std::fs::File::open(archive)?;
    let longueur = f.metadata()?.len();
    let mut t = [0u8; 32];
    f.read_exact(&mut t).map_err(|_| Erreur::Disque(PIEGEE.into()))?;
    let crc = u32::from_le_bytes([t[8], t[9], t[10], t[11]]);
    let decalage = u64::from_le_bytes(t[12..20].try_into().unwrap_or_default());
    let taille = u64::from_le_bytes(t[20..28].try_into().unwrap_or_default());
    let dans_le_fichier = 32u64.checked_add(decalage).and_then(|x| x.checked_add(taille)).is_some_and(|fin| fin <= longueur);
    if t[..6] != SIGNATURE || crc != crc32fast::hash(&t[12..32]) || !dans_le_fichier || taille > ENTETE_MAX {
        return Err(Erreur::Disque(PIEGEE.into()));
    }
    Ok(())
}

/// Une entrée d'une archive 7z, telle que l'archive la décrit (nom BRUT : le parent le juge).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EntreeSeptZ {
    pub nom: String,
    pub taille: u64,
    pub dossier: bool,
    /// Une entrée « à supprimer » (7z de mise à jour) : jamais écrite.
    pub anti: bool,
    /// Porte des données.
    pub flux: bool,
    /// Les attributs Windows de l'entrée, s'ils sont donnés (un lien se reconnaît à eux).
    #[serde(default)]
    pub attributs: Option<u32>,
}

impl EntreeSeptZ {
    /// Un lien (point de reparse Windows, ou lien symbolique Unix rangé dans les 16 bits hauts) : jamais posé.
    pub fn est_un_lien(&self) -> bool {
        self.attributs.is_some_and(|a| a & 0x400 != 0 || (a & 0x8000 != 0 && (a >> 16) & 0xF000 == 0xA000))
    }
}

/// Les entrées de l'archive (rien n'est décompressé).
pub fn lister(archive: &Path) -> Resultat<Vec<EntreeSeptZ>> {
    let mut l = Vec::new();
    executer(&[OsString::from("lister"), archive.into()], &mut |v| {
        if l.len() > ENTREES_MAX {
            return;
        }
        if let Some(e) = v.get("e").and_then(|e| serde_json::from_value::<EntreeSeptZ>(e.clone()).ok()) {
            l.push(e);
        }
    })?;
    if l.len() > ENTREES_MAX {
        return Err(Erreur::Disque(PIEGEE.into()));
    }
    Ok(l)
}

/// Décompresse dans `dossier` chaque entrée dont le chemin est sûr (les autres sont lues dans le vide : un bloc compact
/// reste aligné) ; le parent a d'abord refusé, d'après `lister`, une archive aux chemins dangereux. Rend le nombre de
/// fichiers écrits ; `progres` reçoit les octets écrits.
pub fn extraire(archive: &Path, dossier: &Path, progres: &mut dyn FnMut(u64)) -> Resultat<usize> {
    let fin = executer(&[OsString::from("extraire"), archive.into(), dossier.into()], &mut |v| {
        if let Some(n) = v.get("p").and_then(Value::as_u64) {
            progres(n)
        }
    })?;
    Ok(fin["fichiers"].as_u64().unwrap_or(0) as usize)
}

/// L'empreinte RetroAchievements (`succes::empreinte_octets`) du fichier de l'archive si elle en porte UN seul d'au
/// plus `max` octets (une ROM seule), sinon `None`. Calculée DANS l'enfant : rien n'est écrit, rien de gros ne revient.
pub fn empreinte_unique(archive: &Path, console: u32, max: u64) -> Resultat<Option<String>> {
    let fin = executer(&[OsString::from("empreinte"), archive.into(), OsString::from(console.to_string()), OsString::from(max.to_string())], &mut |_| {})?;
    Ok(fin["empreinte"].as_str().filter(|e| e.len() <= 64 && e.chars().all(|c| c.is_ascii_hexdigit())).map(str::to_string))
}

// --- Le parent ---

/// Lance l'enfant et lit ses lignes ; rend le contenu de sa ligne `fin`.
fn executer(args: &[OsString], ligne: &mut dyn FnMut(&Value)) -> Resultat<Value> {
    #[cfg(test)]
    {
        verifier_entete(Path::new(&args[1]))?;
        // Les tests unitaires n'ont pas Frogtend.exe sous la main : l'enfant est servi sur place (l'isolement réel a
        // son propre test, `tests/sept_z.rs`, avec le vrai programme).
        let mut sortie = Vec::new();
        let code = servir(args, &mut sortie);
        return lire_sortie(std::io::Cursor::new(sortie), code == 0, ligne);
    }
    #[cfg(not(test))]
    {
        let exe = match PROGRAMME.get() {
            Some(p) => p.clone(),
            None => std::env::current_exe().map_err(|_| Erreur::Disque("Frogtend ne se trouve pas lui-même.".into()))?,
        };
        executer_avec(&exe, args, ligne)
    }
}

/// Le programme lecteur, quand ce n'est pas Frogtend lui-même : réglé UNIQUEMENT par les tests d'intégration (leur
/// programme de test ne sait pas servir de lecteur ; ils donnent le vrai Frogtend). Jamais par l'application.
static PROGRAMME: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

#[doc(hidden)]
pub fn programme_pour_les_tests(exe: &Path) {
    let _ = PROGRAMME.set(exe.to_path_buf());
}

/// Lance `exe` en lecteur de 7z enfermé dans un job (mémoire plafonnée, tué avec Frogtend). Séparé pour le test qui
/// passe le vrai programme.
pub fn executer_avec(exe: &Path, args: &[OsString], ligne: &mut dyn FnMut(&Value)) -> Resultat<Value> {
    verifier_entete(Path::new(args.get(1).ok_or_else(|| Erreur::Disque(PIEGEE.into()))?))?;
    let mut c = std::process::Command::new(exe);
    c.arg(ARG).args(args).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    let mut enfant = c.spawn().map_err(|_| Erreur::Disque("Impossible de lancer le lecteur d'archives 7z.".into()))?;
    // Enfermé AVANT de lire quoi que ce soit (il attend « go »).
    let _job = match enfermer(&enfant) {
        Ok(j) => j,
        Err(e) => {
            let _ = enfant.kill();
            let _ = enfant.wait();
            return Err(e);
        }
    };
    let go = enfant.stdin.take().map(|mut s| s.write_all(b"go\n").and_then(|_| s.flush()));
    if !matches!(go, Some(Ok(()))) {
        let _ = enfant.kill();
        let _ = enfant.wait();
        return Err(Erreur::Disque(PIEGEE.into()));
    }
    let sortie = enfant.stdout.take().ok_or_else(|| Erreur::Disque(PIEGEE.into()))?;
    let r = lire_sortie(std::io::BufReader::new(sortie), true, ligne);
    if r.is_err() {
        // Lecture arrêtée avant la fin : l'enfant ne doit pas rester bloqué sur un tube plein.
        let _ = enfant.kill();
    }
    let code = enfant.wait().map(|s| s.success()).unwrap_or(false);
    match r {
        Ok(v) if code => Ok(v),
        Ok(_) => Err(Erreur::Disque(PIEGEE.into())),
        Err(e) => Err(e),
    }
}

/// Lit les lignes de l'enfant : un refus (`erreur`) est rendu tel quel ; sans ligne `fin` (enfant mort, sortie
/// tronquée), l'archive est écartée.
fn lire_sortie(r: impl BufRead, reussi: bool, ligne: &mut dyn FnMut(&Value)) -> Resultat<Value> {
    let mut fin = None;
    let mut r = r;
    loop {
        // Une ligne au plus LIGNE_MAX octets : l'enfant est plafonné, ce qu'il renvoie aussi.
        let mut octets = Vec::new();
        match (&mut r).take(LIGNE_MAX as u64 + 1).read_until(b'\n', &mut octets) {
            Ok(0) | Err(_) => break,
            Ok(_) if octets.len() > LIGNE_MAX => return Err(Erreur::Disque(PIEGEE.into())),
            Ok(_) => {}
        }
        let Ok(v) = serde_json::from_slice::<Value>(&octets) else { continue };
        if let Some(e) = v.get("erreur").and_then(Value::as_str) {
            return Err(Erreur::Refus(e.to_string()));
        }
        if let Some(f) = v.get("fin") {
            fin = Some(f.clone());
            continue;
        }
        ligne(&v);
    }
    match fin {
        Some(f) if reussi => Ok(f),
        _ => Err(Erreur::Disque(PIEGEE.into())),
    }
}

/// Un job Windows qui plafonne la mémoire de l'enfant et le tue quand Frogtend ferme le job (ou meurt).
#[cfg(windows)]
struct Job(windows::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl Drop for Job {
    fn drop(&mut self) {
        // SAFETY: handle obtenu de CreateJobObjectW, fermé une seule fois (KILL_ON_JOB_CLOSE tue l'enfant s'il vit).
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

#[cfg(windows)]
fn enfermer(enfant: &std::process::Child) -> Resultat<Job> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::JobObjects::*;
    let refus = || Erreur::Disque("Impossible de protéger la lecture de l'archive 7z : elle n'est pas ouverte.".into());
    // SAFETY: appels Win32 sur des handles valides ; la structure est initialisée à zéro puis remplie.
    unsafe {
        let job = Job(CreateJobObjectW(None, None).map_err(|_| refus())?);
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_PROCESS_MEMORY | JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION;
        info.ProcessMemoryLimit = MEMOIRE_MAX;
        SetInformationJobObject(job.0, JobObjectExtendedLimitInformation, &info as *const _ as *const _, std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32).map_err(|_| refus())?;
        AssignProcessToJobObject(job.0, HANDLE(enfant.as_raw_handle())).map_err(|_| refus())?;
        Ok(job)
    }
}

#[cfg(not(windows))]
fn enfermer(_: &std::process::Child) -> Resultat<()> {
    Ok(())
}

// --- L'enfant ---

/// Le point d'entrée de l'enfant (`run()`, avant toute autre chose) : attend « go », sert, rend le code de sortie.
pub fn enfant(args: &[OsString]) -> i32 {
    // Un enfant qui meurt (plafond du job) ne montre aucune fenêtre d'erreur de Windows.
    #[cfg(windows)]
    {
        use windows::Win32::System::Diagnostics::Debug::{SetErrorMode, SEM_FAILCRITICALERRORS, SEM_NOGPFAULTERRORBOX};
        // SAFETY: réglage du processus courant, sans pointeur.
        unsafe {
            SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX);
        }
    }
    let mut go = String::new();
    if std::io::stdin().lock().read_line(&mut go).is_err() || go.trim() != "go" {
        return 2;
    }
    let sortie = std::io::stdout();
    let mut s = std::io::BufWriter::new(sortie.lock());
    let code = servir(args, &mut s);
    let _ = s.flush();
    code
}

/// Le travail de l'enfant (appelé tel quel par les tests unitaires). Rend 0 si une ligne `fin` a été écrite.
pub fn servir(args: &[OsString], sortie: &mut dyn Write) -> i32 {
    let op = args.first().and_then(|a| a.to_str()).unwrap_or("");
    let Some(archive) = args.get(1).map(Path::new) else { return 2 };
    let r = match op {
        "lister" => servir_lister(archive, sortie),
        "extraire" => match args.get(2) {
            Some(d) => servir_extraire(archive, Path::new(d), sortie),
            None => return 2,
        },
        "empreinte" => {
            let nombre = |i: usize| args.get(i).and_then(|m| m.to_str()).and_then(|m| m.parse::<u64>().ok());
            match (nombre(2).and_then(|c| u32::try_from(c).ok()), nombre(3)) {
                (Some(console), Some(max)) => servir_empreinte(archive, console, max),
                _ => return 2,
            }
        }
        _ => return 2,
    };
    let ligne = match r {
        Ok(fin) => json!({ "fin": fin }),
        Err(e) => json!({ "erreur": e }),
    };
    if writeln!(sortie, "{ligne}").is_err() {
        return 3;
    }
    if ligne.get("fin").is_some() {
        0
    } else {
        1
    }
}

fn ouvrir(archive: &Path) -> Result<sevenz_rust::SevenZReader<std::fs::File>, String> {
    sevenz_rust::SevenZReader::open(archive, sevenz_rust::Password::empty()).map_err(|e| format!("Archive 7z illisible ({e})."))
}

fn servir_lister(archive: &Path, sortie: &mut dyn Write) -> Result<Value, String> {
    let r = ouvrir(archive)?;
    if r.archive().files.len() > ENTREES_MAX {
        return Err("L'archive 7z annonce bien trop de fichiers : Frogtend l'a écartée.".into());
    }
    for e in &r.archive().files {
        if e.name().len() > NOM_MAX {
            return Err("L'archive 7z contient un nom démesuré : Frogtend l'a écartée.".into());
        }
        let d = EntreeSeptZ { nom: e.name().to_string(), taille: e.size(), dossier: e.is_directory(), anti: e.is_anti_item(), flux: e.has_stream, attributs: e.has_windows_attributes.then_some(e.windows_attributes) };
        writeln!(sortie, "{}", json!({ "e": d })).map_err(|e| e.to_string())?;
    }
    Ok(json!({}))
}

fn servir_extraire(archive: &Path, dossier: &Path, sortie: &mut dyn Write) -> Result<Value, String> {
    let mut r = ouvrir(archive)?;
    std::fs::create_dir_all(dossier).map_err(|e| e.to_string())?;
    let mut n = 0usize;
    let mut a_dire = 0u64;
    let mut tampon = vec![0u8; 1 << 20];
    r.for_each_entries(|e, flux| {
        let nom = (!e.is_anti_item()).then(|| crate::decompression::chemin_sur(e.name())).flatten();
        let Some(nom) = nom else {
            // Lue jusqu'au bout quand même : les entrées suivantes du bloc restent à leur place.
            std::io::copy(flux, &mut std::io::sink()).map_err(sevenz_rust::Error::io)?;
            return Ok(true);
        };
        let cible = dossier.join(nom);
        if e.is_directory() {
            std::fs::create_dir_all(&cible).map_err(sevenz_rust::Error::io)?;
            return Ok(true);
        }
        if let Some(p) = cible.parent() {
            std::fs::create_dir_all(p).map_err(sevenz_rust::Error::io)?;
        }
        let mut f = std::io::BufWriter::with_capacity(1 << 20, std::fs::File::create(&cible).map_err(sevenz_rust::Error::io)?);
        loop {
            let k = flux.read(&mut tampon).map_err(sevenz_rust::Error::io)?;
            if k == 0 {
                break;
            }
            f.write_all(&tampon[..k]).map_err(sevenz_rust::Error::io)?;
            a_dire += k as u64;
            // Une ligne par 4 Mo au plus : la progression sans noyer le tube.
            if a_dire >= 4 << 20 {
                let _ = writeln!(sortie, "{}", json!({ "p": a_dire }));
                let _ = sortie.flush();
                a_dire = 0;
            }
        }
        f.flush().map_err(sevenz_rust::Error::io)?;
        n += 1;
        Ok(true)
    })
    .map_err(|e| format!("Décompression du 7z interrompue ({e})."))?;
    if a_dire > 0 {
        let _ = writeln!(sortie, "{}", json!({ "p": a_dire }));
        let _ = sortie.flush();
    }
    Ok(json!({ "fichiers": n }))
}

fn servir_empreinte(archive: &Path, console: u32, max: u64) -> Result<Value, String> {
    let mut r = ouvrir(archive)?;
    let fichiers: Vec<u64> = r.archive().files.iter().filter(|e| !e.is_directory() && !e.is_anti_item()).map(|e| e.size()).collect();
    if fichiers.len() != 1 || fichiers[0] > max {
        return Ok(json!({ "empreinte": null }));
    }
    let mut rom = None;
    r.for_each_entries(|e, flux| {
        if e.is_directory() || e.is_anti_item() {
            return Ok(true);
        }
        let mut o = Vec::new();
        flux.take(max + 1).read_to_end(&mut o).map_err(sevenz_rust::Error::io)?;
        rom = Some(o);
        Ok(false)
    })
    .map_err(|e| format!("Archive 7z illisible ({e})."))?;
    Ok(match rom {
        Some(o) if o.len() as u64 <= max => json!({ "empreinte": crate::succes::empreinte_octets(console, &o) }),
        _ => json!({ "empreinte": null }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn archive(d: &Path, fichiers: &[(&str, &[u8])]) -> std::path::PathBuf {
        let src = d.join("src");
        for (n, c) in fichiers {
            let p = src.join(n);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, c).unwrap();
        }
        let a = d.join("a.7z");
        sevenz_rust::compress_to_path(&src, &a).unwrap();
        a
    }

    #[test]
    fn lister_extraire_et_empreinte_passent_par_le_protocole() {
        let d = tempfile::tempdir().unwrap();
        let a = archive(d.path(), &[("jeu/rom.bin", b"0123456789"), ("lisez.txt", b"ok")]);
        let mut l = lister(&a).unwrap();
        l.sort_by(|x, y| x.nom.cmp(&y.nom));
        assert!(l.iter().any(|e| e.nom == "jeu/rom.bin" && e.taille == 10 && !e.dossier));
        let mut total = 0;
        assert_eq!(extraire(&a, &d.path().join("x"), &mut |n| total += n).unwrap(), 2);
        assert_eq!(total, 12);
        assert_eq!(std::fs::read(d.path().join("x/jeu/rom.bin")).unwrap(), b"0123456789");
        assert_eq!(empreinte_unique(&a, 7, 100).unwrap(), None, "deux fichiers : pas une ROM seule");
        let d2 = tempfile::tempdir().unwrap();
        let un = archive(d2.path(), &[("rom.pce", b"ROM!")]);
        assert_eq!(empreinte_unique(&un, 8, 100).unwrap(), Some(crate::succes::empreinte_octets(8, b"ROM!")));
        assert_eq!(empreinte_unique(&un, 8, 3).unwrap(), None, "plus gros que le plafond");
    }

    #[test]
    fn un_en_tete_menteur_est_refuse_sans_rien_ouvrir() {
        let d = tempfile::tempdir().unwrap();
        let a = archive(d.path(), &[("f", b"x")]);
        let mut o = std::fs::read(&a).unwrap();
        // En-tête suivant annoncé à 3 Gio dans un petit fichier, CRC de départ recalculé (sinon refusé pour le CRC).
        o[20..28].copy_from_slice(&(3u64 << 30).to_le_bytes());
        let crc = crc32fast::hash(&o[12..32]);
        o[8..12].copy_from_slice(&crc.to_le_bytes());
        let piegee = d.path().join("piegee.7z");
        std::fs::write(&piegee, &o).unwrap();
        assert!(matches!(lister(&piegee), Err(Erreur::Disque(_))));
        std::fs::write(d.path().join("court.7z"), b"7z").unwrap();
        assert!(lister(&d.path().join("court.7z")).is_err());
        std::fs::write(d.path().join("faux.7z"), [0u8; 64]).unwrap();
        assert!(lister(&d.path().join("faux.7z")).is_err());
    }

    #[test]
    fn un_enfant_mort_ou_une_sortie_tronquee_ecartent_l_archive() {
        let mut rien = |_: &Value| {};
        assert!(lire_sortie(std::io::Cursor::new(b"{\"e\":{}}\n".to_vec()), true, &mut rien).is_err(), "sans fin");
        assert!(lire_sortie(std::io::Cursor::new(b"{\"fin\":{}}\n".to_vec()), false, &mut rien).is_err(), "code non nul");
        assert!(matches!(lire_sortie(std::io::Cursor::new(b"{\"erreur\":\"non\"}\n".to_vec()), true, &mut rien), Err(Erreur::Refus(m)) if m == "non"));
        assert!(lire_sortie(std::io::Cursor::new(b"{\"p\":1}\n{\"fin\":{\"a\":1}}\n".to_vec()), true, &mut rien).is_ok());
        // Une ligne démesurée : écartée, sans tout lire en mémoire.
        let mut longue = vec![b'a'; LIGNE_MAX + 10];
        longue.push(b'\n');
        assert!(lire_sortie(std::io::Cursor::new(longue), true, &mut rien).is_err());
    }
}
