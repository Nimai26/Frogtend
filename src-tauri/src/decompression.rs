//! « Décompresser pour jouer » : un jeu importé dont le fichier est une archive (.zip, .7z) qui contient une image
//! disque (ou un jeu PS3 en dossier). Les émulateurs de consoles à disques (RPCS3, PCSX2, Dolphin, DuckStation, Cemu,
//! Eden…) ne lisent pas une image disque rangée dans une archive : Frogtend propose de la décompresser UNE fois, à côté
//! de l'archive (dans le dossier de la console, choisi par la personne), et seulement avec son accord.
//!
//! - L'archive est GARDÉE : rien ne s'efface sans preuve ; la personne la supprimera elle-même si elle veut.
//! - Les fichiers gardent leur nom (jamais de ROM ni d'image disque renommée).
//! - Rien n'est écrit par-dessus : la décompression se fait dans un dossier provisoire, chaque fichier est vérifié
//!   (sa taille), puis le dossier prend son vrai nom.

use crate::erreurs::{Erreur, Resultat};
use serde::Serialize;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Les images disque (et formats de consoles à disques) qu'un émulateur ne lit pas dans une archive.
const EXTENSIONS_DISQUE: &[&str] = &[
    "iso", "cue", "bin", "img", "ccd", "chd", "gdi", "cdi", "m3u", "mds", "mdf", "nrg", "rvz", "wbfs", "gcm", "gcz",
    "ciso", "cso", "pbp", "wua", "wud", "wux", "xci", "nsp",
];

/// Ce qu'on lance, par ordre de préférence : la liste des disques, puis la description des pistes, puis l'image.
const PRIORITE: &[&str] = &["m3u", "cue", "gdi", "ccd", "mds", "chd", "iso", "rvz", "wbfs", "gcm", "gcz", "ciso", "cso", "pbp", "wua", "wud", "wux", "xci", "nsp", "cdi", "nrg", "img", "bin"];

/// Une archive qui contient un jeu à décompresser.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ArchiveDeJeu {
    /// Chemin complet de l'archive.
    pub archive: String,
    /// Nombre de fichiers et taille une fois décompressés.
    pub fichiers: usize,
    pub taille: u64,
    /// Ce qu'on lancera, relatif au dossier de destination.
    pub principal: String,
    /// Le dossier où les fichiers iront (à côté de l'archive, du nom de l'archive).
    pub destination: String,
    /// Les fichiers sont-ils déjà là (décompressés avant, et complets) ?
    pub deja: bool,
}

/// Un fichier d'une archive : chemin relatif sûr (sans « .. »), taille.
#[derive(Debug, Clone, PartialEq)]
struct Entree {
    nom: String,
    taille: u64,
}

fn extension(nom: &str) -> String {
    Path::new(nom).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

/// Un chemin relatif sûr : pas absolu, pas de « .. », séparateur `/`.
fn chemin_sur(nom: &str) -> Option<String> {
    let n = nom.replace('\\', "/");
    let parties: Vec<&str> = n.split('/').filter(|p| !p.is_empty() && *p != ".").collect();
    if parties.is_empty() || n.starts_with('/') || parties.iter().any(|p| *p == ".." || p.contains(':')) {
        return None;
    }
    Some(parties.join("/"))
}

fn est_archive(chemin: &Path) -> Option<&'static str> {
    match chemin.extension()?.to_string_lossy().to_lowercase().as_str() {
        "zip" => Some("zip"),
        "7z" => Some("7z"),
        _ => None,
    }
}

/// Les fichiers d'une archive, sans rien décompresser.
fn lister(archive: &Path) -> Resultat<Vec<Entree>> {
    let mut l = Vec::new();
    match est_archive(archive) {
        Some("zip") => {
            let mut z = zip::ZipArchive::new(std::fs::File::open(archive)?)
                .map_err(|e| Erreur::Disque(format!("Archive zip illisible ({e}).")))?;
            for i in 0..z.len() {
                let e = z.by_index_raw(i).map_err(|e| Erreur::Disque(format!("Archive zip abîmée ({e}).")))?;
                if e.is_file() {
                    let nom = chemin_sur(e.name()).ok_or_else(|| Erreur::Refus(format!("L'archive contient un chemin dangereux : {}.", e.name())))?;
                    l.push(Entree { nom, taille: e.size() });
                }
            }
        }
        Some(_) => {
            let r = sevenz_rust::SevenZReader::open(archive, sevenz_rust::Password::empty())
                .map_err(|e| Erreur::Disque(format!("Archive 7z illisible ({e}).")))?;
            for e in &r.archive().files {
                if !e.is_directory() && e.has_stream() {
                    let nom = chemin_sur(e.name()).ok_or_else(|| Erreur::Refus(format!("L'archive contient un chemin dangereux : {}.", e.name())))?;
                    l.push(Entree { nom, taille: e.size() });
                }
            }
        }
        None => {}
    }
    Ok(l)
}

/// Ce qu'on lancera parmi ces fichiers : `None` s'il n'y a pas d'image disque (une ROM de cartouche zippée se lit
/// telle quelle par les émulateurs : rien à faire).
fn principal(entrees: &[Entree]) -> Option<String> {
    // Un jeu PS3 en dossier : RPCS3 lance son EBOOT.BIN.
    if let Some(e) = entrees.iter().find(|e| e.nom.to_lowercase().ends_with("ps3_game/usrdir/eboot.bin")) {
        return Some(e.nom.clone());
    }
    for ext in PRIORITE {
        let mut l: Vec<&Entree> = entrees.iter().filter(|e| extension(&e.nom) == *ext).collect();
        // Le moins profond d'abord, puis par nom (le disque 1 avant le disque 2).
        l.sort_by_key(|e| (e.nom.matches('/').count(), e.nom.to_lowercase()));
        if let Some(e) = l.first() {
            return Some(e.nom.clone());
        }
    }
    None
}

/// Le dossier de destination : à côté de l'archive, de son nom sans extension.
fn destination(archive: &Path) -> PathBuf {
    let nom = archive.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "jeu".into());
    archive.with_file_name(nom)
}

/// Le dossier provisoire, le temps de la décompression (reconnaissable : seul lui peut être effacé par Frogtend).
fn provisoire(destination: &Path) -> PathBuf {
    let nom = destination.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    destination.with_file_name(format!("{nom} (décompression Frogtend en cours)"))
}

/// Tous les fichiers sont-ils là, à la bonne taille ?
fn complet(dossier: &Path, entrees: &[Entree]) -> bool {
    entrees.iter().all(|e| std::fs::metadata(dossier.join(&e.nom)).is_ok_and(|m| m.is_file() && m.len() == e.taille))
}

/// Examine une archive : `None` si ce n'est pas une archive ou si elle ne contient pas d'image disque.
pub fn examiner(archive: &Path) -> Resultat<Option<ArchiveDeJeu>> {
    if est_archive(archive).is_none() || !archive.is_file() {
        return Ok(None);
    }
    let entrees = lister(archive)?;
    if !entrees.iter().any(|e| EXTENSIONS_DISQUE.contains(&extension(&e.nom).as_str()) || e.nom.to_lowercase().ends_with("eboot.bin")) {
        return Ok(None);
    }
    let Some(principal) = principal(&entrees) else { return Ok(None) };
    let dest = destination(archive);
    Ok(Some(ArchiveDeJeu {
        archive: archive.to_string_lossy().to_string(),
        fichiers: entrees.len(),
        taille: entrees.iter().map(|e| e.taille).sum(),
        principal,
        deja: dest.is_dir() && complet(&dest, &entrees),
        destination: dest.to_string_lossy().to_string(),
    }))
}

/// Copie un flux dans un fichier en comptant les octets.
fn ecrire(source: &mut dyn Read, cible: &Path, progres: &mut dyn FnMut(u64)) -> std::io::Result<u64> {
    if let Some(p) = cible.parent() {
        std::fs::create_dir_all(p)?;
    }
    let mut sortie = std::io::BufWriter::with_capacity(1 << 20, std::fs::File::create(cible)?);
    let mut tampon = vec![0u8; 1 << 20];
    let mut total = 0;
    loop {
        let n = source.read(&mut tampon)?;
        if n == 0 {
            break;
        }
        sortie.write_all(&tampon[..n])?;
        total += n as u64;
        progres(n as u64);
    }
    sortie.flush()?;
    Ok(total)
}

fn extraire(archive: &Path, dossier: &Path, entrees: &[Entree], progres: &mut dyn FnMut(u64)) -> Resultat<()> {
    std::fs::create_dir_all(dossier)?;
    match est_archive(archive) {
        Some("zip") => {
            let mut z = zip::ZipArchive::new(std::fs::File::open(archive)?)
                .map_err(|e| Erreur::Disque(format!("Archive zip illisible ({e}).")))?;
            for i in 0..z.len() {
                let mut e = z.by_index(i).map_err(|e| Erreur::Disque(format!("Archive zip abîmée ({e}).")))?;
                if !e.is_file() {
                    continue;
                }
                let Some(nom) = chemin_sur(e.name()) else { continue };
                ecrire(&mut e, &dossier.join(&nom), progres)?;
            }
        }
        Some(_) => {
            let mut r = sevenz_rust::SevenZReader::open(archive, sevenz_rust::Password::empty())
                .map_err(|e| Erreur::Disque(format!("Archive 7z illisible ({e}).")))?;
            r.for_each_entries(|e, flux| {
                if e.is_directory() || !e.has_stream() {
                    return Ok(true);
                }
                if let Some(nom) = chemin_sur(e.name()) {
                    ecrire(flux, &dossier.join(&nom), progres).map_err(sevenz_rust::Error::io)?;
                }
                Ok(true)
            })
            .map_err(|e| Erreur::Disque(format!("Décompression du 7z interrompue ({e}).")))?;
        }
        None => return Err(Erreur::Refus("Ce fichier n'est pas une archive.".into())),
    }
    if !complet(dossier, entrees) {
        return Err(Erreur::Disque("Un fichier décompressé n'a pas la taille annoncée par l'archive (archive abîmée ?).".into()));
    }
    Ok(())
}

/// Décompresse l'archive à côté d'elle (voir `examiner`) et rend le chemin complet de ce qu'on lancera. `libre` : la
/// place libre sur le disque de destination (`None` : inconnue). L'archive n'est jamais touchée.
pub fn decompresser(archive: &Path, libre: Option<u64>, progres: &mut dyn FnMut(u64)) -> Resultat<PathBuf> {
    let a = examiner(archive)?.ok_or_else(|| Erreur::Refus("Cette archive ne contient pas de jeu à décompresser.".into()))?;
    let dest = PathBuf::from(&a.destination);
    if a.deja {
        return Ok(dest.join(&a.principal));
    }
    if dest.exists() {
        return Err(Erreur::Refus(format!(
            "Le dossier « {} » existe déjà et ne contient pas ce jeu complet : Frogtend n'écrit jamais par-dessus. Renomme-le ou vide-le toi-même.",
            dest.display()
        )));
    }
    // Une marge de 1 % (et au moins 100 Mo) : le disque ne doit pas finir plein.
    if let Some(libre) = libre {
        let besoin = a.taille + (a.taille / 100).max(100 << 20);
        if libre < besoin {
            return Err(Erreur::Refus(format!(
                "Pas assez de place sur le disque : il faut {} Go, il reste {} Go.",
                besoin.div_ceil(1 << 30),
                libre >> 30
            )));
        }
    }
    let entrees = lister(archive)?;
    let tmp = provisoire(&dest);
    // Un dossier provisoire laissé par une décompression interrompue : c'est le nôtre (son nom le dit), on le reprend
    // de zéro.
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp)?;
    }
    if let Err(e) = extraire(archive, &tmp, &entrees, progres) {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(e);
    }
    std::fs::rename(&tmp, &dest)?;
    // Vérifier le résultat, pas le retour.
    if !complet(&dest, &entrees) {
        return Err(Erreur::Disque("Les fichiers décompressés ne sont pas tous là après le rangement.".into()));
    }
    Ok(dest.join(&a.principal))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zip(chemin: &Path, fichiers: &[(&str, &[u8])]) {
        let mut z = zip::ZipWriter::new(std::fs::File::create(chemin).unwrap());
        for (nom, contenu) in fichiers {
            z.start_file(*nom, zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(contenu).unwrap();
        }
        z.finish().unwrap();
    }

    #[test]
    fn une_rom_de_cartouche_zippee_ne_se_decompresse_pas() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("Mario (Europe).zip");
        zip(&a, &[("Mario (Europe).nes", b"NES")]);
        assert_eq!(examiner(&a).unwrap(), None);
        assert_eq!(examiner(&d.path().join("rien.iso")).unwrap(), None, "pas une archive");
    }

    #[test]
    fn le_cue_passe_avant_ses_pistes() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("Crash (Europe).zip");
        zip(&a, &[("Crash (Europe) (Track 1).bin", b"1234"), ("Crash (Europe) (Track 2).bin", b"56"), ("Crash (Europe).cue", b"FILE")]);
        let x = examiner(&a).unwrap().unwrap();
        assert_eq!((x.principal.as_str(), x.fichiers, x.taille, x.deja), ("Crash (Europe).cue", 3, 10, false));
        assert_eq!(PathBuf::from(&x.destination), d.path().join("Crash (Europe)"));
    }

    #[test]
    fn un_jeu_ps3_en_dossier_se_lance_par_son_eboot() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("Demons Souls.zip");
        zip(&a, &[("BLES00932/PS3_GAME/USRDIR/EBOOT.BIN", b"SCE"), ("BLES00932/PS3_GAME/PARAM.SFO", b"PSF")]);
        assert_eq!(examiner(&a).unwrap().unwrap().principal, "BLES00932/PS3_GAME/USRDIR/EBOOT.BIN");
    }

    #[test]
    fn decompresse_a_cote_et_garde_l_archive() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("Jeu.zip");
        zip(&a, &[("Jeu.iso", &[7u8; 5000]), ("lisez-moi.txt", b"bonjour")]);
        let avant = std::fs::read(&a).unwrap();
        let mut ecrits = 0;
        let p = decompresser(&a, None, &mut |n| ecrits += n).unwrap();
        assert_eq!(p, d.path().join("Jeu").join("Jeu.iso"));
        assert_eq!(std::fs::read(&p).unwrap(), vec![7u8; 5000], "contenu relu");
        assert_eq!(ecrits, 5007);
        assert_eq!(std::fs::read(&a).unwrap(), avant, "l'archive n'est pas touchée");
        assert!(!provisoire(&d.path().join("Jeu")).exists(), "plus de dossier provisoire");
        // Une deuxième fois : déjà fait, rien n'est réécrit.
        assert!(examiner(&a).unwrap().unwrap().deja);
        assert_eq!(decompresser(&a, None, &mut |_| panic!("rien à réécrire")).unwrap(), p);
    }

    #[test]
    fn n_ecrit_jamais_par_dessus_ni_sur_un_disque_plein() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("Jeu.zip");
        zip(&a, &[("Jeu.iso", &[1u8; 100])]);
        std::fs::create_dir(d.path().join("Jeu")).unwrap();
        std::fs::write(d.path().join("Jeu").join("Jeu.iso"), b"autre").unwrap();
        assert!(matches!(decompresser(&a, None, &mut |_| {}), Err(Erreur::Refus(_))));
        assert_eq!(std::fs::read(d.path().join("Jeu").join("Jeu.iso")).unwrap(), b"autre", "rien d'écrasé");
        std::fs::remove_dir_all(d.path().join("Jeu")).unwrap();
        assert!(matches!(decompresser(&a, Some(1000), &mut |_| {}), Err(Erreur::Refus(_))), "il faut au moins 100 Mo de marge");
        assert!(!d.path().join("Jeu").exists());
    }

    #[test]
    fn un_7z_se_decompresse_aussi() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("src");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(src.join("Jeu.chd"), [3u8; 300]).unwrap();
        let a = d.path().join("Jeu.7z");
        sevenz_rust::compress_to_path(&src, &a).unwrap();
        let p = decompresser(&a, None, &mut |_| {}).unwrap();
        assert_eq!(std::fs::read(p).unwrap(), vec![3u8; 300]);
    }

    #[test]
    fn refuse_les_chemins_qui_sortent_du_dossier() {
        assert_eq!(chemin_sur("../../Windows/x.iso"), None);
        assert_eq!(chemin_sur("C:/x.iso"), None);
        assert_eq!(chemin_sur("/x.iso"), None);
        assert_eq!(chemin_sur("a\\b.iso").as_deref(), Some("a/b.iso"));
    }
}
