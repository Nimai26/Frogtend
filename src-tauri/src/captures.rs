//! Le dossier général des captures d'écran (✅ Seb 04/11) : pendant une partie lancée par Frogtend, chaque capture prise
//! par l'émulateur (menu de Frogtend ou touche de l'émulateur) est DÉPLACÉE vers
//! `<dossier choisi>\<Système>\<nom du jeu>\<date heure>.<extension>`.
//!
//! Règle « pas de pieuvre » : seul le dossier de captures de l'émulateur LANCÉ est regardé, et seules les images
//! apparues après le début de la partie (absentes de l'inventaire pris avant le lancement, ou RÉÉCRITES depuis : BigPEmu
//! reprend `…_screen0000.tga` à chaque partie, par-dessus la précédente) sont déplacées. Une capture
//! prise hors d'une partie de Frogtend n'est jamais touchée. Un original n'est effacé qu'une fois sa copie relue.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Les extensions d'image déplacées (jamais une vidéo ni un son : DOSBox, RPCS3 rangent aussi des enregistrements).
/// `tga` : BigPEmu (converti en `bmp` au rangement, l'interface ne sait pas l'afficher).
const IMAGES: &[&str] = &["png", "jpg", "jpeg", "bmp", "webp", "tga"];

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
        // BigPEmu : `UserData\game<empreinte>_screenNNNN.tga` (son message à l'écran, relevé de Seb le 12/11 ; nom
        // `game%016llX_screen%04i.tga` dans le programme 1.221), chez la personne qui joue.
        "bigpemu" => profil.map(|p| p.join(crate::bigpemu::DOSSIER_DONNEES)).into_iter().collect(),
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

/// Ce qui distingue deux images au même nom : sa taille et sa date d'écriture.
type Trace = Option<(u64, std::time::SystemTime)>;

fn trace(f: &Path) -> Trace {
    std::fs::metadata(f).ok().and_then(|m| Some((m.len(), m.modified().ok()?)))
}

/// Une image déjà vue : avec sa trace (réécrite, elle redevient une capture), ou présente avant la partie sans trace
/// lisible (jamais touchée : on ne saurait pas dire si elle a été réécrite).
#[derive(Debug, PartialEq)]
enum Vu {
    Trace(Trace),
    Jamais,
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

/// Une image TGA sans compression (type 2, 24 ou 32 bits, sans palette) en BMP, sans perte : mêmes pixels (BGR ou
/// BGRA), lignes remises de bas en haut. `None` : une autre sorte de TGA (laissée telle quelle).
pub fn tga_en_bmp(tga: &[u8]) -> Option<Vec<u8>> {
    if tga.len() < 18 {
        return None;
    }
    let (id, palette, sorte) = (tga[0] as usize, tga[1], tga[2]);
    let largeur = u16::from_le_bytes([tga[12], tga[13]]) as usize;
    let hauteur = u16::from_le_bytes([tga[14], tga[15]]) as usize;
    let (bits, descripteur) = (tga[16], tga[17]);
    if palette != 0 || sorte != 2 || !(bits == 24 || bits == 32) || largeur == 0 || hauteur == 0 || descripteur & 0x10 != 0 {
        return None;
    }
    let octets = bits as usize / 8;
    let ligne = largeur * octets;
    let debut = 18 + id;
    let pixels = tga.get(debut..debut + ligne * hauteur)?;
    let pas = ligne.div_ceil(4) * 4;
    let taille_image = pas * hauteur;
    let mut b = Vec::with_capacity(54 + taille_image);
    b.extend_from_slice(b"BM");
    b.extend_from_slice(&((54 + taille_image) as u32).to_le_bytes());
    b.extend_from_slice(&[0; 4]);
    b.extend_from_slice(&54u32.to_le_bytes());
    b.extend_from_slice(&40u32.to_le_bytes());
    b.extend_from_slice(&(largeur as i32).to_le_bytes());
    b.extend_from_slice(&(hauteur as i32).to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&(bits as u16).to_le_bytes());
    b.extend_from_slice(&[0; 4]);
    b.extend_from_slice(&(taille_image as u32).to_le_bytes());
    b.extend_from_slice(&[0; 16]);
    // BMP : de bas en haut. Le TGA l'est aussi, sauf si son bit 5 dit « en haut à gauche ».
    let de_haut = descripteur & 0x20 != 0;
    for k in 0..hauteur {
        let r = if de_haut { hauteur - 1 - k } else { k };
        b.extend_from_slice(&pixels[r * ligne..(r + 1) * ligne]);
        b.extend(std::iter::repeat_n(0u8, pas - ligne));
    }
    Some(b)
}

/// Déplace une capture : renommage sur le même disque ; sinon copie à côté sous un nom provisoire, taille relue,
/// renommage, et SEULEMENT ENSUITE l'original effacé — s'il n'a pas été réécrit entre-temps (sinon il reste, pour le
/// passage suivant). Rend la destination, et la trace de l'original resté en place (`None` : il n'y est plus).
fn deplacer(source: &Path, dossier: &Path) -> std::io::Result<(PathBuf, Trace)> {
    let taille = stable(source).ok_or_else(|| std::io::Error::other("capture en cours d'écriture"))?;
    let avant = trace(source);
    let effacer = || {
        let t = trace(source);
        if t == avant && avant.is_some() && std::fs::remove_file(source).is_ok() {
            return None;
        }
        trace(source)
    };
    let date = std::fs::metadata(source)?.modified().unwrap_or_else(|_| std::time::SystemTime::now());
    let ext = source.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_else(|| "png".into());
    std::fs::create_dir_all(dossier)?;
    // Une capture TGA (BigPEmu) devient un BMP sans perte : écrit à côté sous un nom provisoire, relu, puis l'original
    // effacé. Un TGA d'une autre sorte est rangé tel quel.
    if ext == "tga" {
        if let Some(bmp) = std::fs::read(source).ok().and_then(|t| tga_en_bmp(&t)) {
            let cible = nom_libre(dossier, &horodatage(date), "bmp");
            let provisoire = cible.with_extension("bmp.en-cours");
            std::fs::write(&provisoire, &bmp)?;
            if std::fs::read(&provisoire).ok().as_deref() != Some(bmp.as_slice()) {
                let _ = std::fs::remove_file(&provisoire);
                return Err(std::io::Error::other("conversion incomplète"));
            }
            std::fs::rename(&provisoire, &cible)?;
            return Ok((cible, effacer()));
        }
    }
    let cible = nom_libre(dossier, &horodatage(date), &ext);
    if std::fs::rename(source, &cible).is_ok() {
        return Ok((cible, None));
    }
    let provisoire = cible.with_extension(format!("{ext}.en-cours"));
    let copie = std::fs::copy(source, &provisoire);
    if copie.as_ref().ok() != Some(&taille) || std::fs::metadata(&provisoire).map(|m| m.len()).ok() != Some(taille) {
        let _ = std::fs::remove_file(&provisoire);
        return Err(copie.err().unwrap_or_else(|| std::io::Error::other("copie incomplète")));
    }
    std::fs::rename(&provisoire, &cible)?;
    // L'original n'est effacé qu'une fois la copie en place (s'il est encore tenu, il reste : noté dans `vus`).
    Ok((cible, effacer()))
}

/// Le veilleur d'une partie : les dossiers regardés, ce qui y était avant, et où ranger.
pub struct Veilleur {
    pub dossiers: Vec<PathBuf>,
    /// Chaque image vue (avant la partie, ou déjà rangée), avec sa trace : une image réécrite sous le même nom est
    /// une nouvelle capture.
    vus: HashMap<PathBuf, Vu>,
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
        let vus = inventaire(&dossiers).into_iter().map(|f| {
            let t = match trace(&f) {
                Some(t) => Vu::Trace(Some(t)),
                None => Vu::Jamais,
            };
            (f, t)
        }).collect();
        let cible = choisi.join(crate::jeux_pc::nom_de_dossier(systeme)).join(crate::jeux_pc::nom_de_dossier(jeu));
        Ok(Some(Veilleur { dossiers, vus, cible, echecs: Default::default() }))
    }

    /// Range les nouvelles captures ; rend celles rangées à ce passage (leur nouvelle place), et les captures
    /// abandonnées (à dire au journal).
    pub fn ranger(&mut self) -> (Vec<PathBuf>, Vec<String>) {
        let (mut n, mut abandons) = (Vec::new(), Vec::new());
        for f in inventaire(&self.dossiers) {
            // Présente avant la partie (à la personne), ou déjà rangée (original encore tenu par l'émulateur, qui reste
            // alors en double plutôt que d'être recopié) : jamais touchée, tant qu'elle n'est pas réécrite.
            match self.vus.get(&f) {
                Some(Vu::Jamais) => continue,
                Some(Vu::Trace(t)) if *t == trace(&f) => continue,
                Some(Vu::Trace(_)) => {
                    // Réécrite : ses essais repartent de zéro.
                    self.echecs.remove(&f);
                }
                None => {}
            }
            match deplacer(&f, &self.cible) {
                Ok((place, reste)) => {
                    n.push(place);
                    self.echecs.remove(&f);
                    self.vus.insert(f, Vu::Trace(reste));
                }
                // En cours d'écriture, ou tenue : reprise au passage suivant, puis laissée à sa place.
                Err(e) => {
                    let k = self.echecs.entry(f.clone()).or_insert(0);
                    *k += 1;
                    if *k >= ESSAIS {
                        abandons.push(format!("{} ({e})", f.display()));
                        let t = trace(&f);
                        self.vus.insert(f, Vu::Trace(t));
                    }
                }
            }
        }
        (n, abandons)
    }
}

/// Une capture notée par Frogtend au rangement (✅ Seb 06/11) : QUI l'a prise (le profil) et pour QUEL jeu. Chaque
/// profil ne voit que les siennes ; le jeu reste retrouvé même si son titre ou sa console changent de nom.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Notee {
    /// Identifiant stable (empreinte du chemin), pour l'adresse `capture.localhost/<id>`.
    pub id: String,
    pub chemin: String,
    pub profil: String,
    pub jeu: i64,
    /// Secondes depuis 1970.
    pub quand: u64,
    /// Le titre et la console au moment de la capture : la galerie les montre même si le jeu a quitté le PC.
    #[serde(default)]
    pub titre: String,
    #[serde(default)]
    pub plateforme: String,
}

/// Un seul lecteur-écrivain de l'index à la fois (rangement pendant la partie, suppression depuis la galerie).
static VERROU_INDEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn fichier_index(donnees: &Path) -> PathBuf {
    donnees.join("captures.json")
}

fn lire_sans_verrou(donnees: &Path) -> Vec<Notee> {
    std::fs::read(fichier_index(donnees)).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default()
}

fn ecrire_sans_verrou(donnees: &Path, l: &[Notee]) -> std::io::Result<()> {
    let f = fichier_index(donnees);
    let provisoire = f.with_extension("json.tmp");
    std::fs::write(&provisoire, serde_json::to_vec_pretty(l).unwrap_or_default())?;
    std::fs::rename(&provisoire, &f)
}

/// L'index des captures (`captures.json` des données de Frogtend).
pub fn index(donnees: &Path) -> Vec<Notee> {
    let _v = VERROU_INDEX.lock().unwrap_or_else(|e| e.into_inner());
    lire_sans_verrou(donnees)
}

/// L'identifiant d'une capture : les 16 premiers caractères de l'empreinte SHA-256 de son chemin.
pub fn identifiant(chemin: &Path) -> String {
    use sha2::Digest;
    let h = sha2::Sha256::digest(chemin.to_string_lossy().to_lowercase().as_bytes());
    h.iter().take(8).map(|o| format!("{o:02x}")).collect()
}

/// Note les captures rangées pour ce profil et ce jeu (sans profil ouvert : rien, elles ne seraient à personne).
pub fn noter(donnees: &Path, places: &[PathBuf], profil: &str, jeu: i64, titre: &str, plateforme: &str) -> std::io::Result<()> {
    if places.is_empty() || profil.is_empty() {
        return Ok(());
    }
    let _v = VERROU_INDEX.lock().unwrap_or_else(|e| e.into_inner());
    let mut l = lire_sans_verrou(donnees);
    for p in places {
        let quand = std::fs::metadata(p).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
        let id = identifiant(p);
        l.retain(|n| n.id != id);
        l.push(Notee { id, chemin: p.to_string_lossy().into(), profil: profil.into(), jeu, quand, titre: titre.into(), plateforme: plateforme.into() });
    }
    ecrire_sans_verrou(donnees, &l)
}

/// Retire une capture de l'index (supprimée, ou disparue du disque).
pub fn oublier(donnees: &Path, id: &str) -> std::io::Result<()> {
    let _v = VERROU_INDEX.lock().unwrap_or_else(|e| e.into_inner());
    let mut l = lire_sans_verrou(donnees);
    let avant = l.len();
    l.retain(|n| n.id != id);
    if l.len() != avant { ecrire_sans_verrou(donnees, &l) } else { Ok(()) }
}

/// Une capture de CE profil, encore sur le disque.
pub fn de_ce_profil(donnees: &Path, id: &str, profil: &str) -> Option<Notee> {
    notee_de_ce_profil(donnees, id, profil).filter(|n| Path::new(&n.chemin).is_file())
}

/// Une capture de CE profil notée dans l'index (même si son fichier a disparu).
pub fn notee_de_ce_profil(donnees: &Path, id: &str, profil: &str) -> Option<Notee> {
    index(donnees).into_iter().find(|n| n.id == id && n.profil == profil)
}

/// Les dimensions d'une image PNG (lues dans son en-tête) ; `None` pour un autre format.
pub fn dimensions_png(octets: &[u8]) -> Option<(u32, u32)> {
    if octets.len() < 24 || &octets[..8] != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    Some((u32::from_be_bytes(octets[16..20].try_into().ok()?), u32::from_be_bytes(octets[20..24].try_into().ok()?)))
}

/// Assez grande pour un fond net (la même règle que l'interface, `assezGrandPourUnFond` : 640 × 360).
pub fn assez_grande_pour_un_fond(octets: &[u8]) -> bool {
    dimensions_png(octets).is_none_or(|(l, h)| l >= 640 && h >= 360)
}

/// Le type d'image d'après l'extension.
pub fn type_image(chemin: &Path) -> &'static str {
    match chemin.extension().map(|e| e.to_string_lossy().to_lowercase()).as_deref() {
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("bmp") => "image/bmp",
        Some("webp") => "image/webp",
        _ => "image/png",
    }
}

/// Le fond choisi par un profil pour un jeu (`<profil>\fonds-choisis.json` : jeu → identifiant de capture).
pub fn fonds_choisis(dossier_profil: &Path) -> std::collections::BTreeMap<String, String> {
    std::fs::read(dossier_profil.join("fonds-choisis.json")).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default()
}

/// Un seul écrivain des fonds choisis à la fois (un changement de fond pendant une suppression ne perd rien).
static VERROU_FONDS: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn regler_fond(dossier_profil: &Path, jeu: i64, capture: Option<&str>) -> std::io::Result<()> {
    let _v = VERROU_FONDS.lock().unwrap_or_else(|e| e.into_inner());
    let mut m = fonds_choisis(dossier_profil);
    match capture {
        Some(c) => m.insert(jeu.to_string(), c.into()),
        None => m.remove(&jeu.to_string()),
    };
    std::fs::create_dir_all(dossier_profil)?;
    let f = dossier_profil.join("fonds-choisis.json");
    let provisoire = f.with_extension("json.tmp");
    std::fs::write(&provisoire, serde_json::to_vec_pretty(&m).unwrap_or_default())?;
    std::fs::rename(&provisoire, &f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_capture_tga_de_bigpemu_devient_un_bmp_sans_perte() {
        // 2 × 2 pixels, 24 bits, origine en bas à gauche (comme BigPEmu : `0 0 2 … 24 0`).
        let mut tga = vec![0u8, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 2, 0, 24, 0];
        let pixels: Vec<u8> = (1..=12).collect();
        tga.extend_from_slice(&pixels);
        let bmp = tga_en_bmp(&tga).unwrap();
        assert_eq!(&bmp[..2], b"BM");
        assert_eq!(bmp.len(), 54 + 2 * 8, "lignes de 6 octets complétées à 8");
        assert_eq!(&bmp[54..60], &pixels[..6], "même ordre de bas en haut");
        assert_eq!(&bmp[62..68], &pixels[6..]);
        let mut en_haut = tga.clone();
        en_haut[17] = 0x20;
        assert_eq!(&tga_en_bmp(&en_haut).unwrap()[54..60], &pixels[6..], "origine en haut : lignes retournées");
        let mut rle = tga.clone();
        rle[2] = 10;
        assert!(tga_en_bmp(&rle).is_none(), "compressé : laissé tel quel");
        // Rangée : un .bmp, l'original retiré.
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("game5C0C7827948F0F27_screen0000.tga");
        std::fs::write(&src, &tga).unwrap();
        let (place, reste) = deplacer(&src, &d.path().join("rangees")).unwrap();
        assert_eq!(reste, None);
        assert_eq!(place.extension().unwrap(), "bmp");
        assert_eq!(std::fs::read(&place).unwrap(), bmp);
        assert!(!src.exists());
        assert_eq!(dossiers_de("bigpemu", d.path(), Some(d.path()), None), vec![d.path().join("UserData")]);
    }

    #[test]
    fn l_index_des_captures_par_profil() {
        let d = tempfile::tempdir().unwrap();
        let img = d.path().join("SNES").join("SMW").join("a.png");
        std::fs::create_dir_all(img.parent().unwrap()).unwrap();
        std::fs::write(&img, b"png").unwrap();
        noter(d.path(), std::slice::from_ref(&img), "seb", 114, "Super Mario World", "Super Nintendo Entertainment System").unwrap();
        noter(d.path(), std::slice::from_ref(&img), "", 114, "x", "y").unwrap();
        let id = identifiant(&img);
        assert_eq!(index(d.path()).len(), 1);
        assert!(de_ce_profil(d.path(), &id, "seb").is_some());
        assert!(de_ce_profil(d.path(), &id, "lea").is_none(), "jamais celle d'un autre profil");
        noter(d.path(), std::slice::from_ref(&img), "seb", 114, "Super Mario World", "Super Nintendo Entertainment System").unwrap();
        assert_eq!(index(d.path()).len(), 1, "jamais en double (ni sans profil)");
        assert_eq!(index(d.path())[0].titre, "Super Mario World");
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
        png.extend_from_slice(&240u32.to_be_bytes());
        png.extend_from_slice(&160u32.to_be_bytes());
        assert_eq!(dimensions_png(&png), Some((240, 160)));
        assert!(!assez_grande_pour_un_fond(&png), "une capture de Game Boy Advance : trop petite");
        assert!(assez_grande_pour_un_fond(b"jpeg"), "autre format : on ne sait pas, accepté");
        oublier(d.path(), &id).unwrap();
        assert!(index(d.path()).is_empty());
        let p = d.path().join("profil");
        regler_fond(&p, 114, Some(&id)).unwrap();
        assert_eq!(fonds_choisis(&p).get("114"), Some(&id));
        regler_fond(&p, 114, None).unwrap();
        assert!(fonds_choisis(&p).is_empty());
        assert_eq!(type_image(Path::new("x.JPG")), "image/jpeg");
    }

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
        assert_eq!(v.ranger().0.len(), 1);
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
        assert_eq!(v.ranger().0.len(), 2);
        assert_eq!(std::fs::read_dir(choisi.join("Sony Playstation 3").join("Asura's Wrath")).unwrap().count(), 3);
        assert_eq!(v.ranger().0.len(), 0, "rien de neuf");
    }

    #[test]
    fn une_capture_reecrite_sous_le_meme_nom_est_rangee() {
        // BigPEmu reprend `…_screen0000.tga` à chaque partie, par-dessus celle de la fois d'avant.
        let d = tempfile::tempdir().unwrap();
        let ecrans = d.path().join("UserData");
        std::fs::create_dir_all(&ecrans).unwrap();
        let f = ecrans.join("game5C0C7827948F0F27_screen0000.tga");
        std::fs::write(&f, b"hier").unwrap();
        let hier = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
        std::fs::File::options().write(true).open(&f).unwrap().set_modified(hier).unwrap();
        let choisi = d.path().join("Mes captures");
        let mut v = Veilleur::nouveau(Some(choisi.clone()), vec![ecrans.clone()], "Atari Jaguar", "Bubsy").unwrap().unwrap();
        assert_eq!(v.ranger().0.len(), 0, "celle d'avant la partie : jamais touchée");
        assert!(f.exists());
        std::fs::write(&f, b"aujourd'hui").unwrap();
        assert_eq!(v.ranger().0.len(), 1, "réécrite pendant la partie : une nouvelle capture");
        assert!(!f.exists());
        let rangees: Vec<_> = std::fs::read_dir(choisi.join("Atari Jaguar").join("Bubsy")).unwrap().flatten().map(|e| e.path()).collect();
        assert_eq!(std::fs::read(&rangees[0]).unwrap(), b"aujourd'hui");
        assert_eq!(v.ranger().0.len(), 0);
    }

    #[test]
    fn sans_dossier_choisi_rien() {
        assert!(Veilleur::nouveau(None, vec![PathBuf::from("E:\\x")], "S", "J").unwrap().is_none());
        assert!(Veilleur::nouveau(Some(PathBuf::from("E:\\C")), vec![], "S", "J").unwrap().is_none());
        assert!(Veilleur::nouveau(Some(PathBuf::from("E:\\x\\C")), vec![PathBuf::from("E:\\x")], "S", "J").is_err());
        assert!(est_capture(Path::new("United.States.png")) && !est_capture(Path::new("jeu.state.auto.png")) && !est_capture(Path::new("jeu.state12.png")));
    }
}
