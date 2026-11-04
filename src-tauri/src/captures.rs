//! Le dossier général des captures d'écran (✅ Seb 04/11) : pendant une partie lancée par Frogtend, chaque capture prise
//! par l'émulateur (menu de Frogtend ou touche de l'émulateur) est DÉPLACÉE vers
//! `<dossier choisi>\<Système>\<nom du jeu>\<date heure>.<extension>`.
//!
//! Règle « pas de pieuvre » : seul le dossier de captures de l'émulateur LANCÉ est regardé, et seules les images
//! apparues après le début de la partie (absentes de l'inventaire pris avant le lancement) sont déplacées. Une capture
//! prise hors d'une partie de Frogtend n'est jamais touchée. Un original n'est effacé qu'une fois sa copie relue.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Les extensions d'image déplacées (jamais une vidéo ni un son : DOSBox, RPCS3 rangent aussi des enregistrements).
const IMAGES: &[&str] = &["png", "jpg", "jpeg", "bmp", "webp"];

/// Les dossiers où l'émulateur range ses captures. Seulement ceux vérifiés (04/11) : RetroArch (son
/// `screenshot_directory`, « : » = son dossier, vide = le dossier du jeu), RPCS3 (`screenshots\<numéro>\`, vu dans le
/// journal de Seb), mGBA (le dossier que Frogtend lui donne, `emulateurs_profils.rs`). Les autres : rien (à relever).
pub fn dossiers_de(emulateur: &str, dossier_emulateur: &Path, profil: Option<&Path>, jeu: Option<&Path>) -> Vec<PathBuf> {
    match emulateur {
        "retroarch" => {
            // Dans le dossier du jeu : `screenshots_in_content_dir`, ou `screenshot_directory` vide / « default ».
            // Clé absente : `:\screenshots` ; un chemin relatif l'est au dossier de RetroArch (`dossier_reglage_retroarch`).
            let valeur = valeur_retroarch(dossier_emulateur, "screenshot_directory");
            let dans_le_jeu = crate::emulateurs::reglage_retroarch(dossier_emulateur, "screenshots_in_content_dir") == Some(true)
                || valeur.as_deref().is_some_and(|v| v.is_empty() || v.eq_ignore_ascii_case("default"));
            if dans_le_jeu {
                jeu.and_then(Path::parent).map(Path::to_path_buf).into_iter().collect()
            } else {
                vec![crate::emulateurs::dossier_reglage_retroarch(dossier_emulateur, "screenshot_directory", "screenshots")]
            }
        }
        "rpcs3" => vec![dossier_emulateur.join("screenshots")],
        "mgba" => profil.map(|p| p.join("screenshots")).into_iter().collect(),
        _ => Vec::new(),
    }
}

/// Une valeur texte de `retroarch.cfg` (lecture seule ; jamais par `--appendconfig`, que RetroArch recopie en quittant).
fn valeur_retroarch(dossier: &Path, cle: &str) -> Option<String> {
    let cfg = std::fs::read_to_string(dossier.join("retroarch.cfg")).ok()?;
    cfg.lines().find_map(|l| {
        let (c, v) = l.split_once('=')?;
        (c.trim() == cle).then(|| v.trim().trim_matches('"').to_string())
    })
}

/// Une image à déplacer : bonne extension, et jamais la vignette d'une sauvegarde rapide (`*.state*.png` de RetroArch).
fn est_capture(f: &Path) -> bool {
    let nom = f.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    let ext = f.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    // `jeu.state.png`, `jeu.state3.png`, `jeu.state.auto.png` : des vignettes (jamais « United.States.png »).
    let vignette = nom.split('.').skip(1).any(|s| s.strip_prefix("state").is_some_and(|r| r.is_empty() || r == "auto" || r.chars().all(|c| c.is_ascii_digit())));
    IMAGES.contains(&ext.as_str()) && !vignette
}

/// Les images présentes (le dossier et ses sous-dossiers directs : RPCS3 et RetroArch trient par jeu).
pub fn inventaire(dossiers: &[PathBuf]) -> HashSet<PathBuf> {
    let mut l = HashSet::new();
    for d in dossiers {
        let Ok(entrees) = std::fs::read_dir(d) else { continue };
        for e in entrees.flatten() {
            let p = e.path();
            if p.is_dir() {
                // Jamais le dossier des sauvegardes rapides (leurs vignettes).
                if p.file_name().is_some_and(|n| n.eq_ignore_ascii_case("states")) {
                    continue;
                }
                for s in std::fs::read_dir(&p).into_iter().flatten().flatten() {
                    if s.path().is_file() && est_capture(&s.path()) {
                        l.insert(s.path());
                    }
                }
            } else if est_capture(&p) {
                l.insert(p);
            }
        }
    }
    l
}

/// Le dossier choisi ne doit être ni dans un dossier regardé, ni le contenir (sinon les captures déplacées seraient
/// reprises en boucle).
pub fn cible_sure(cible: &Path, dossiers: &[PathBuf]) -> bool {
    let norme = |p: &Path| p.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_lowercase();
    let c = norme(cible);
    dossiers.iter().all(|d| {
        let d = norme(d);
        c != d && !c.starts_with(&format!("{d}\\")) && !d.starts_with(&format!("{c}\\"))
    })
}

/// « 2026-11-04 11-55-54 » : l'heure LOCALE de la capture (sans « : », interdit dans un nom de fichier).
fn horodatage(t: std::time::SystemTime) -> String {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::{FILETIME, SYSTEMTIME};
        use windows::Win32::System::Time::{FileTimeToSystemTime, SystemTimeToTzSpecificLocalTime};
        let cent_ns = t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() / 100).unwrap_or(0) as u64 + 116_444_736_000_000_000;
        let ft = FILETIME { dwLowDateTime: cent_ns as u32, dwHighDateTime: (cent_ns >> 32) as u32 };
        let (mut utc, mut local) = (SYSTEMTIME::default(), SYSTEMTIME::default());
        unsafe {
            if FileTimeToSystemTime(&ft, &mut utc).is_ok() && SystemTimeToTzSpecificLocalTime(None, &utc, &mut local).is_ok() {
                return format!("{:04}-{:02}-{:02} {:02}-{:02}-{:02}", local.wYear, local.wMonth, local.wDay, local.wHour, local.wMinute, local.wSecond);
            }
        }
    }
    let s = t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    format!("capture {s}")
}

/// Le nom libre dans le dossier du jeu : jamais par-dessus une capture (« -2 », « -3 »…).
fn nom_libre(dossier: &Path, base: &str, ext: &str) -> PathBuf {
    let mut f = dossier.join(format!("{base}.{ext}"));
    let mut n = 2;
    // Une erreur d'accès compte comme « pris » : jamais par-dessus.
    while f.try_exists().unwrap_or(true) {
        f = dossier.join(format!("{base}-{n}.{ext}"));
        n += 1;
    }
    f
}

/// Taille non nulle et identique à 250 ms d'écart : l'émulateur a fini de l'écrire.
fn stable(f: &Path) -> Option<u64> {
    let a = std::fs::metadata(f).ok()?.len();
    std::thread::sleep(std::time::Duration::from_millis(250));
    let b = std::fs::metadata(f).ok()?.len();
    (a > 0 && a == b).then_some(a)
}

/// Déplace une capture : renommage sur le même disque ; sinon copie à côté sous un nom provisoire, taille relue,
/// renommage, et SEULEMENT ENSUITE l'original effacé. Rend la destination.
fn deplacer(source: &Path, dossier: &Path) -> std::io::Result<PathBuf> {
    let taille = stable(source).ok_or_else(|| std::io::Error::other("capture en cours d'écriture"))?;
    let date = std::fs::metadata(source)?.modified().unwrap_or_else(|_| std::time::SystemTime::now());
    let ext = source.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_else(|| "png".into());
    std::fs::create_dir_all(dossier)?;
    let cible = nom_libre(dossier, &horodatage(date), &ext);
    if std::fs::rename(source, &cible).is_ok() {
        return Ok(cible);
    }
    let provisoire = cible.with_extension(format!("{ext}.en-cours"));
    let copie = std::fs::copy(source, &provisoire);
    if copie.as_ref().ok() != Some(&taille) || std::fs::metadata(&provisoire).map(|m| m.len()).ok() != Some(taille) {
        let _ = std::fs::remove_file(&provisoire);
        return Err(copie.err().unwrap_or_else(|| std::io::Error::other("copie incomplète")));
    }
    std::fs::rename(&provisoire, &cible)?;
    // L'original n'est effacé qu'une fois la copie en place (s'il est encore tenu, il reste : noté dans `vus`).
    let _ = std::fs::remove_file(source);
    Ok(cible)
}

/// Le veilleur d'une partie : les dossiers regardés, ce qui y était avant, et où ranger.
pub struct Veilleur {
    pub dossiers: Vec<PathBuf>,
    vus: HashSet<PathBuf>,
    cible: PathBuf,
    echecs: std::collections::HashMap<PathBuf, u32>,
}

/// Au-delà de ces essais ratés (≈ 1 min), une capture est laissée à sa place et dite au journal.
const ESSAIS: u32 = 30;

impl Veilleur {
    /// `Ok(None)` : rien à faire (pas de dossier choisi, émulateur non relevé) ; `Err` : le dossier choisi est dans un
    /// dossier regardé ou le contient (à dire au journal).
    pub fn nouveau(choisi: Option<PathBuf>, dossiers: Vec<PathBuf>, systeme: &str, jeu: &str) -> Result<Option<Veilleur>, String> {
        let Some(choisi) = choisi else { return Ok(None) };
        if dossiers.is_empty() {
            return Ok(None);
        }
        if !cible_sure(&choisi, &dossiers) {
            return Err(format!("le dossier des captures {} touche un dossier de captures de l'émulateur ({dossiers:?}) : rien n'est rangé", choisi.display()));
        }
        let vus = inventaire(&dossiers);
        let cible = choisi.join(crate::jeux_pc::nom_de_dossier(systeme)).join(crate::jeux_pc::nom_de_dossier(jeu));
        Ok(Some(Veilleur { dossiers, vus, cible, echecs: Default::default() }))
    }

    /// Range les nouvelles captures ; rend le nombre rangé à ce passage, et les captures abandonnées (à dire au journal).
    pub fn ranger(&mut self) -> (usize, Vec<String>) {
        let (mut n, mut abandons) = (0, Vec::new());
        for f in inventaire(&self.dossiers) {
            // Présente avant la partie (à la personne), ou déjà rangée (original encore tenu par l'émulateur, qui reste
            // alors en double plutôt que d'être recopié) : jamais touchée.
            if self.vus.contains(&f) {
                continue;
            }
            match deplacer(&f, &self.cible) {
                Ok(_) => {
                    n += 1;
                    self.vus.insert(f);
                }
                // En cours d'écriture, ou tenue : reprise au passage suivant, puis laissée à sa place.
                Err(e) => {
                    let k = self.echecs.entry(f.clone()).or_insert(0);
                    *k += 1;
                    if *k >= ESSAIS {
                        abandons.push(format!("{} ({e})", f.display()));
                        self.vus.insert(f);
                    }
                }
            }
        }
        (n, abandons)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_dossiers_de_chaque_emulateur() {
        let d = tempfile::tempdir().unwrap();
        let ra = d.path().join("RetroArch");
        std::fs::create_dir_all(&ra).unwrap();
        std::fs::write(ra.join("retroarch.cfg"), "screenshot_directory = \":\\screenshots\"\n").unwrap();
        assert_eq!(dossiers_de("retroarch", &ra, None, None), vec![ra.join("screenshots")]);
        std::fs::write(ra.join("retroarch.cfg"), "screenshot_directory = \"\"\n").unwrap();
        let rom = d.path().join("Roms").join("jeu.sfc");
        assert_eq!(dossiers_de("retroarch", &ra, None, Some(&rom)), vec![d.path().join("Roms")], "vide : le dossier du jeu");
        std::fs::write(ra.join("retroarch.cfg"), "screenshot_directory = \"F:\\Captures\"\n").unwrap();
        assert_eq!(dossiers_de("retroarch", &ra, None, None), vec![PathBuf::from("F:\\Captures")]);
        std::fs::write(ra.join("retroarch.cfg"), "video_smooth = \"false\"\n").unwrap();
        assert_eq!(dossiers_de("retroarch", &ra, None, Some(&rom)), vec![ra.join("screenshots")], "clé absente : son dossier");
        std::fs::write(ra.join("retroarch.cfg"), "screenshot_directory = \"captures\"\n").unwrap();
        assert_eq!(dossiers_de("retroarch", &ra, None, None), vec![ra.join("captures")], "relatif : au dossier de RetroArch");
        std::fs::write(ra.join("retroarch.cfg"), "screenshot_directory = \":\\screenshots\"\nscreenshots_in_content_dir = \"true\"\n").unwrap();
        assert_eq!(dossiers_de("retroarch", &ra, None, Some(&rom)), vec![d.path().join("Roms")], "dans le dossier du jeu");
        assert_eq!(dossiers_de("rpcs3", Path::new("E:\\RPCS3"), None, None), vec![PathBuf::from("E:\\RPCS3\\screenshots")]);
        assert!(dossiers_de("dosbox-staging", Path::new("E:\\D"), None, None).is_empty(), "non relevé : rien");
    }

    #[test]
    fn le_dossier_choisi_jamais_dans_un_dossier_regarde() {
        let regardes = vec![PathBuf::from("E:\\RetroArch\\screenshots")];
        assert!(cible_sure(Path::new("E:\\Captures"), &regardes));
        assert!(!cible_sure(Path::new("E:\\RetroArch\\screenshots"), &regardes));
        assert!(!cible_sure(Path::new("e:/retroarch/screenshots/Frogtend"), &regardes));
        assert!(!cible_sure(Path::new("E:\\RetroArch"), &regardes), "le contient");
        assert!(cible_sure(Path::new("E:\\RetroArch\\screenshots2"), &regardes));
    }

    #[test]
    fn seules_les_nouvelles_captures_sont_deplacees() {
        let d = tempfile::tempdir().unwrap();
        let ecrans = d.path().join("screenshots");
        std::fs::create_dir_all(ecrans.join("BLES01227")).unwrap();
        std::fs::create_dir_all(ecrans.join("states")).unwrap();
        std::fs::write(ecrans.join("ancienne.png"), b"avant").unwrap();
        let choisi = d.path().join("Mes captures");
        let mut v = Veilleur::nouveau(Some(choisi.clone()), vec![ecrans.clone()], "Sony Playstation 3", "Asura's Wrath").unwrap().unwrap();
        // Pendant la partie : une capture, une vignette de sauvegarde, une vidéo.
        std::fs::write(ecrans.join("BLES01227").join("BLES01227_screenshot_1.png"), b"image").unwrap();
        std::fs::write(ecrans.join("states").join("x.png"), b"vignette").unwrap();
        std::fs::write(ecrans.join("jeu.state1.png"), b"vignette").unwrap();
        std::fs::write(ecrans.join("film.avi"), b"video").unwrap();
        assert_eq!(v.ranger().0, 1);
        let rangees: Vec<_> = std::fs::read_dir(choisi.join("Sony Playstation 3").join("Asura's Wrath")).unwrap().flatten().map(|e| e.path()).collect();
        assert_eq!(rangees.len(), 1);
        assert_eq!(std::fs::read(&rangees[0]).unwrap(), b"image");
        assert!(rangees[0].extension().unwrap() == "png");
        assert!(!ecrans.join("BLES01227").join("BLES01227_screenshot_1.png").exists(), "déplacée, pas copiée");
        assert!(ecrans.join("ancienne.png").exists(), "celle d'avant la partie : jamais touchée");
        assert!(ecrans.join("jeu.state1.png").exists() && ecrans.join("states").join("x.png").exists() && ecrans.join("film.avi").exists());
        // Deux captures dans la même seconde : jamais l'une par-dessus l'autre.
        let t = std::fs::metadata(&rangees[0]).unwrap().modified().unwrap();
        for i in 0..2 {
            let f = ecrans.join(format!("c{i}.png"));
            std::fs::write(&f, format!("i{i}")).unwrap();
            std::fs::File::options().write(true).open(&f).unwrap().set_modified(t).unwrap();
        }
        assert_eq!(v.ranger().0, 2);
        assert_eq!(std::fs::read_dir(choisi.join("Sony Playstation 3").join("Asura's Wrath")).unwrap().count(), 3);
        assert_eq!(v.ranger().0, 0, "rien de neuf");
    }

    #[test]
    fn sans_dossier_choisi_rien() {
        assert!(Veilleur::nouveau(None, vec![PathBuf::from("E:\\x")], "S", "J").unwrap().is_none());
        assert!(Veilleur::nouveau(Some(PathBuf::from("E:\\C")), vec![], "S", "J").unwrap().is_none());
        assert!(Veilleur::nouveau(Some(PathBuf::from("E:\\x\\C")), vec![PathBuf::from("E:\\x")], "S", "J").is_err());
        assert!(est_capture(Path::new("United.States.png")) && !est_capture(Path::new("jeu.state.auto.png")) && !est_capture(Path::new("jeu.state12.png")));
    }
}
