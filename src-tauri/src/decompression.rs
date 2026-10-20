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

/// Les images disque (et formats de consoles à disques) qu'un émulateur ne lit pas dans une archive. Pas « .bin » ni
/// « .img » seuls : ce sont aussi des ROM de cartouches (Atari 2600, Odyssey², Mega Drive…) et d'arcade, que les
/// émulateurs lisent zippées ; ils ne comptent qu'avec la feuille qui les décrit (.cue, .ccd, .gdi).
const EXTENSIONS_DISQUE: &[&str] = &[
    "iso", "cue", "ccd", "chd", "gdi", "cdi", "m3u", "mds", "nrg", "rvz", "wbfs", "gcm", "gcz", "ciso", "cso", "pbp",
    "wua", "wud", "wux", "xci", "nsp",
];

/// Ce qu'on lance, par ordre de préférence : la liste des disques, puis la description des pistes, puis l'image.
const PRIORITE: &[&str] = &["m3u", "cue", "gdi", "ccd", "mds", "chd", "iso", "rvz", "wbfs", "gcm", "gcz", "ciso", "cso", "pbp", "wua", "wud", "wux", "xci", "nsp", "cdi", "nrg"];

/// « Décompresser pour jouer » concerne-t-il cette plateforme ? Jamais l'arcade : MAME lit ses jeux zippés, et un
/// dossier du nom du jeu à côté du zip serait pris pour ses CHD.
pub fn concerne(plateforme: &str) -> bool {
    let p = plateforme.to_lowercase();
    crate::succes::console_de(plateforme) != Some(27) && !p.contains("arcade") && !p.contains("mame")
}

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
pub(crate) fn chemin_sur(nom: &str) -> Option<String> {
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
            // Lu par un Frogtend enfant (`sept_z` : un 7z piégé ne peut pas fermer Frogtend).
            // Un fichier vide n'a pas de flux : il compte quand même (taille 0).
            for e in crate::sept_z::lister(archive)? {
                if !e.dossier && !e.anti {
                    let nom = chemin_sur(&e.nom).ok_or_else(|| Erreur::Refus(format!("L'archive contient un chemin dangereux : {}.", e.nom)))?;
                    l.push(Entree { nom, taille: e.taille });
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
    if !entrees.iter().any(|e| EXTENSIONS_DISQUE.contains(&extension(&e.nom).as_str()) || e.nom.to_lowercase().ends_with("ps3_game/usrdir/eboot.bin")) {
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
                // Jamais plus que la taille annoncée (une archive piégée ne remplit pas le disque).
                let taille = e.size();
                ecrire(&mut (&mut e).take(taille), &dossier.join(&nom), progres)?;
            }
        }
        Some(_) => {
            // Par un Frogtend enfant ; les chemins ont déjà été vérifiés par `lister`, et le résultat l'est par
            // `complet` ci-dessous.
            crate::sept_z::extraire(archive, dossier, progres)?;
        }
        None => return Err(Erreur::Refus("Ce fichier n'est pas une archive.".into())),
    }
    if !complet(dossier, entrees) {
        return Err(Erreur::Disque("Un fichier décompressé n'a pas la taille annoncée par l'archive (archive abîmée ?).".into()));
    }
    Ok(())
}

/// Les archives en cours de décompression : une deuxième demande pour la même est refusée (elle effacerait le dossier
/// provisoire de la première).
static EN_COURS: std::sync::LazyLock<std::sync::Mutex<std::collections::BTreeSet<PathBuf>>> = std::sync::LazyLock::new(Default::default);

/// Libère l'archive à la fin, même en cas d'erreur.
struct Jeton(PathBuf);
impl Drop for Jeton {
    fn drop(&mut self) {
        EN_COURS.lock().unwrap_or_else(|e| e.into_inner()).remove(&self.0);
    }
}

/// Décompresse l'archive à côté d'elle (voir `examiner`) et rend le chemin complet de ce qu'on lancera. `libre` : la
/// place libre sur le disque de destination (`None` : inconnue). L'archive n'est jamais touchée.
pub fn decompresser(archive: &Path, libre: Option<u64>, progres: &mut dyn FnMut(u64)) -> Resultat<PathBuf> {
    let cle = PathBuf::from(archive.to_string_lossy().to_lowercase());
    if !EN_COURS.lock().unwrap_or_else(|e| e.into_inner()).insert(cle.clone()) {
        return Err(Erreur::Refus("Ce jeu est déjà en train d'être décompressé.".into()));
    }
    let _jeton = Jeton(cle);
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

/// Décompresse chaque archive À CÔTÉ d'elle, sans sous-dossier (les disques d'un multi-disques zippé, contrat 1.23
/// `archive_a_extraire` : le .m3u qui les liste est déjà là). Rend les fichiers posés (déjà là compris). Un fichier
/// déjà là à la bonne taille est gardé tel quel (une réinstallation ne touche pas aux disques) ; un fichier déjà là
/// autrement (une partie écrite dedans ?) fait tout refuser : jamais rien par-dessus. Les archives ne sont jamais
/// touchées. Rend (tous les fichiers, ceux posés CETTE fois) ; en cas d'échec, ceux posés cette fois sont effacés.
pub fn extraire_a_cote(archives: &[PathBuf], libre: Option<u64>) -> Resultat<(Vec<PathBuf>, Vec<PathBuf>)> {
    let mut jetons = Vec::new();
    for a in archives {
        let cle = PathBuf::from(a.to_string_lossy().to_lowercase());
        if !EN_COURS.lock().unwrap_or_else(|e| e.into_inner()).insert(cle.clone()) {
            return Err(Erreur::Refus("Ce jeu est déjà en train d'être décompressé.".into()));
        }
        jetons.push(Jeton(cle));
    }
    // Tout est examiné AVANT d'écrire quoi que ce soit : chemins, doublons entre archives, fichiers déjà là.
    let mut plan = Vec::new();
    let mut vus = std::collections::BTreeSet::new();
    let mut besoin = 0u64;
    for a in archives {
        if est_archive(a).is_none() || !a.is_file() {
            return Err(Erreur::Refus(format!("« {} » n'est pas une archive lisible.", a.display())));
        }
        let dossier = a.parent().map(Path::to_path_buf).unwrap_or_default();
        let entrees = lister(a)?;
        if entrees.is_empty() {
            return Err(Erreur::Refus(format!("L'archive « {} » est vide.", a.display())));
        }
        let mut a_poser = Vec::new();
        for e in &entrees {
            if e.nom.contains('/') {
                return Err(Erreur::Refus(format!("L'archive « {} » contient un dossier ({}) : Frogtend ne la décompresse pas à côté du jeu.", a.display(), e.nom)));
            }
            let cible = dossier.join(&e.nom);
            if !vus.insert(cible.to_string_lossy().to_lowercase()) {
                return Err(Erreur::Refus(format!("Deux archives de ce jeu contiennent le même fichier « {} » : rien n'est décompressé.", e.nom)));
            }
            match std::fs::symlink_metadata(&cible) {
                Ok(m) if m.is_file() && m.len() == e.taille => {}
                Ok(_) => {
                    return Err(Erreur::Refus(format!(
                        "« {} » est déjà à côté du jeu, mais pas tel que dans son archive (une partie y est peut-être écrite) : Frogtend n'écrit jamais par-dessus. Pour réinstaller, déplace d'abord ce fichier toi-même.",
                        e.nom
                    )))
                }
                Err(_) => {
                    besoin += e.taille;
                    a_poser.push(e.nom.clone());
                }
            }
        }
        plan.push((a.clone(), dossier, entrees, a_poser));
    }
    if let Some(libre) = libre {
        let avec_marge = besoin + (besoin / 100).max(100 << 20);
        if besoin > 0 && libre < avec_marge {
            return Err(Erreur::Refus(format!(
                "Pas assez de place sur le disque : il faut {} Go, il reste {} Go.",
                avec_marge.div_ceil(1 << 30),
                libre >> 30
            )));
        }
    }
    let mut nouveaux = Vec::new();
    let r = poser(&plan, &mut nouveaux);
    if r.is_err() {
        // Rien d'orphelin : ce qui vient d'être posé (absent avant, donc à Frogtend) est retiré.
        for p in &nouveaux {
            let _ = std::fs::remove_file(p);
        }
    }
    r?;
    drop(jetons);
    let tous = plan.iter().flat_map(|(_, dossier, entrees, _)| entrees.iter().map(move |e| dossier.join(&e.nom))).collect();
    Ok((tous, nouveaux))
}

type Plan = Vec<(PathBuf, PathBuf, Vec<Entree>, Vec<String>)>;

/// Pose les fichiers du plan (`nouveaux` : ce qui a été posé, même en cas d'échec), puis vérifie le résultat.
fn poser(plan: &Plan, nouveaux: &mut Vec<PathBuf>) -> Resultat<()> {
    for (a, dossier, entrees, a_poser) in plan {
        if a_poser.is_empty() {
            continue;
        }
        let nom = a.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "jeu".into());
        let tmp = provisoire(&dossier.join(nom));
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp)?;
        }
        let r = extraire(a, &tmp, entrees, &mut |_| {}).and_then(|_| {
            for n in a_poser {
                poser_sans_ecraser(&tmp.join(n), &dossier.join(n))?;
                nouveaux.push(dossier.join(n));
            }
            Ok(())
        });
        let _ = std::fs::remove_dir_all(&tmp);
        r?;
    }
    // Vérifier le résultat, pas le retour.
    for (_, dossier, entrees, _) in plan {
        if !complet(dossier, entrees) {
            return Err(Erreur::Disque("Les disques décompressés ne sont pas tous là, à la bonne taille.".into()));
        }
    }
    Ok(())
}

/// Range un fichier décompressé à sa place, SANS jamais écraser (sous Windows, `rename` d'un fichier remplace la
/// cible) : un lien dur échoue si la cible existe ; sur un disque qui n'en veut pas (partage réseau), on revérifie
/// juste avant de déplacer.
fn poser_sans_ecraser(source: &Path, cible: &Path) -> Resultat<()> {
    match std::fs::hard_link(source, cible) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            Err(Erreur::Refus(format!("« {} » est apparu pendant la décompression : Frogtend n'écrit jamais par-dessus.", cible.display())))
        }
        Err(_) => {
            if std::fs::symlink_metadata(cible).is_ok() {
                return Err(Erreur::Refus(format!("« {} » est apparu pendant la décompression : Frogtend n'écrit jamais par-dessus.", cible.display())));
            }
            std::fs::rename(source, cible)?;
            Ok(())
        }
    }
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
        // Des .bin seuls : une ROM d'arcade (MAME) ou de cartouche, pas un disque.
        let a = d.path().join("dkong.zip");
        zip(&a, &[("c_5et_g.bin", b"1"), ("v_5h_b.bin", b"2")]);
        assert_eq!(examiner(&a).unwrap(), None);
        let a = d.path().join("Pitfall (USA).7z");
        let src = d.path().join("src");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(src.join("Pitfall (USA).bin"), b"2600").unwrap();
        sevenz_rust::compress_to_path(&src, &a).unwrap();
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
    fn un_7z_se_decompresse_aussi_avec_ses_fichiers_vides() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("src");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(src.join("Jeu.chd"), [3u8; 300]).unwrap();
        std::fs::write(src.join("vide.dat"), b"").unwrap();
        let a = d.path().join("Jeu.7z");
        sevenz_rust::compress_to_path(&src, &a).unwrap();
        assert_eq!(examiner(&a).unwrap().unwrap().fichiers, 2, "le fichier vide compte");
        let p = decompresser(&a, None, &mut |_| {}).unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), vec![3u8; 300]);
        assert!(p.with_file_name("vide.dat").is_file(), "le fichier vide est recréé");
    }

    #[test]
    fn deux_decompressions_du_meme_jeu_ne_se_genent_pas() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("Jeu.zip");
        zip(&a, &[("Jeu.iso", &[1u8; 10])]);
        let cle = PathBuf::from(a.to_string_lossy().to_lowercase());
        EN_COURS.lock().unwrap().insert(cle.clone());
        assert!(matches!(decompresser(&a, None, &mut |_| {}), Err(Erreur::Refus(_))));
        assert!(EN_COURS.lock().unwrap().contains(&cle), "la première garde sa place");
        EN_COURS.lock().unwrap().remove(&cle);
        decompresser(&a, None, &mut |_| {}).unwrap();
        assert!(!EN_COURS.lock().unwrap().contains(&cle), "libérée à la fin");
    }

    #[test]
    fn jamais_pour_l_arcade() {
        assert!(!concerne("Arcade") && !concerne("MAME") && !concerne("Sega Naomi Arcade"));
        assert!(concerne("Sony Playstation 3") && concerne("Nintendo GameCube"));
    }

    #[test]
    fn refuse_les_chemins_qui_sortent_du_dossier() {
        assert_eq!(chemin_sur("../../Windows/x.iso"), None);
        assert_eq!(chemin_sur("C:/x.iso"), None);
        assert_eq!(chemin_sur("/x.iso"), None);
        assert_eq!(chemin_sur("a\\b.iso").as_deref(), Some("a/b.iso"));
    }

    #[test]
    fn des_disques_zippes_se_posent_a_cote_sans_rien_ecraser() {
        // Contrat 1.23 (relevé réel Xak, PC-8801) : un zip TOSEC par disque, un .d88 dedans.
        let d = tempfile::tempdir().unwrap();
        let (a1, a2) = (d.path().join("Xak (Disk 1 of 2).zip"), d.path().join("Xak (Disk 2 of 2).zip"));
        zip(&a1, &[("Xak (Disk 1 of 2).d88", &[1u8; 10])]);
        zip(&a2, &[("Xak (Disk 2 of 2).d88", &[2u8; 20])]);
        let (poses, nouveaux) = extraire_a_cote(&[a1.clone(), a2.clone()], None).unwrap();
        assert_eq!(poses, vec![d.path().join("Xak (Disk 1 of 2).d88"), d.path().join("Xak (Disk 2 of 2).d88")]);
        assert_eq!(nouveaux, poses);
        assert_eq!(std::fs::read(&poses[1]).unwrap(), vec![2u8; 20]);
        assert!(a1.is_file() && a2.is_file(), "les archives restent");
        assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 4, "aucun dossier provisoire laissé");

        // Une réinstallation garde les disques tels quels…
        let avant = std::fs::metadata(&poses[0]).unwrap().modified().unwrap();
        assert!(extraire_a_cote(&[a1.clone(), a2.clone()], None).unwrap().1.is_empty(), "rien de posé cette fois");
        assert_eq!(std::fs::metadata(&poses[0]).unwrap().modified().unwrap(), avant);
        // … et un disque changé (une partie écrite dedans ?) fait tout refuser, sans rien toucher.
        std::fs::write(&poses[1], b"partie").unwrap();
        std::fs::remove_file(&poses[0]).unwrap();
        assert!(matches!(extraire_a_cote(&[a1.clone(), a2.clone()], None), Err(Erreur::Refus(_))));
        assert_eq!(std::fs::read(&poses[1]).unwrap(), b"partie");
        assert!(!poses[0].exists(), "rien n'est décompressé quand une seule archive pose problème");
    }

    #[test]
    fn des_disques_zippes_refuses_doublon_dossier_ou_place() {
        let d = tempfile::tempdir().unwrap();
        let (a1, a2, a3) = (d.path().join("1.zip"), d.path().join("2.zip"), d.path().join("3.zip"));
        zip(&a1, &[("disque.d88", &[1u8; 10])]);
        zip(&a2, &[("DISQUE.d88", &[2u8; 10])]);
        zip(&a3, &[("sous/disque.d88", &[3u8; 10])]);
        assert!(matches!(extraire_a_cote(&[a1.clone(), a2], None), Err(Erreur::Refus(_))), "deux archives, un même nom");
        assert!(matches!(extraire_a_cote(&[a3], None), Err(Erreur::Refus(_))), "un dossier dans l'archive");
        assert!(matches!(extraire_a_cote(std::slice::from_ref(&a1), Some(1 << 20)), Err(Erreur::Refus(_))), "pas la place");
        assert!(!d.path().join("disque.d88").exists());
        assert!(!EN_COURS.lock().unwrap().contains(&PathBuf::from(a1.to_string_lossy().to_lowercase())), "archives libérées");
    }

    #[test]
    fn un_fichier_pose_n_ecrase_jamais() {
        let d = tempfile::tempdir().unwrap();
        let (s, c) = (d.path().join("s"), d.path().join("c"));
        std::fs::write(&s, b"neuf").unwrap();
        std::fs::write(&c, b"partie").unwrap();
        assert!(poser_sans_ecraser(&s, &c).is_err());
        assert_eq!(std::fs::read(&c).unwrap(), b"partie");
    }
}
