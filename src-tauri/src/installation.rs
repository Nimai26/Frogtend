//! Installer un jeu téléchargé, selon la NATURE de sa version, et trouver quoi lancer.
//!
//! - Installeur Inno Setup ou NSIS : installation automatique (silencieuse) dans le dossier choisi par Frogtend, ou
//!   guidée (l'installeur s'ouvre) si la personne le préfère.
//! - Archive zip / 7z : décompression.
//! - ROM, image disque : rien à installer, et le fichier n'est JAMAIS renommé.
//! - Autre installeur (repack, installeur d'origine) : il s'ouvre ; on suit les notes de la version.
//!
//! Après l'installation, Frogtend relève l'état des fichiers (le « manifeste ») : ce qui changera ensuite (les parties
//! sauvegardées) sera reconnu et mis à l'abri avant toute réinstallation ou tout retrait.

use crate::erreurs::{Erreur, Resultat};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

/// La nature d'une version (champ `qualite` de Firehouse) : elle décide de la façon d'installer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Nature {
    PretAJouer,
    Rom,
    ImageDisque,
    Repack,
    InstalleurOrigine,
    NonDite,
}

pub fn nature(qualite: &str) -> Nature {
    let q = qualite.to_lowercase();
    if q.contains("prêt à jouer") || q.contains("pret a jouer") {
        Nature::PretAJouer
    } else if q.contains("rom") {
        Nature::Rom
    } else if q.contains("image disque") {
        Nature::ImageDisque
    } else if q.contains("repack") {
        Nature::Repack
    } else if q.contains("installeur") {
        Nature::InstalleurOrigine
    } else {
        Nature::NonDite
    }
}

/// Ce qu'est un fichier, d'après son contenu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Inno,
    Nsis,
    Zip,
    SeptZip,
    Rar,
    /// Un exécutable Windows qui n'est pas un installeur connu (le jeu lui-même, ou un installeur inconnu).
    Exe,
    Autre,
}

/// Octets lus au début d'un fichier pour le reconnaître (les installeurs Inno portent leur marque vers 750 Ko).
const OCTETS_LUS: usize = 2 * 1024 * 1024;

pub fn format_de(chemin: &Path) -> Resultat<Format> {
    let mut f = std::fs::File::open(chemin)?;
    let mut o = Vec::with_capacity(OCTETS_LUS);
    f.by_ref().take(OCTETS_LUS as u64).read_to_end(&mut o)?;
    Ok(format_des_octets(&o))
}

pub fn format_des_octets(o: &[u8]) -> Format {
    let contient = |m: &[u8]| o.windows(m.len()).any(|w| w == m);
    if o.starts_with(b"PK\x03\x04") {
        Format::Zip
    } else if o.starts_with(&[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C]) {
        Format::SeptZip
    } else if o.starts_with(b"Rar!") {
        Format::Rar
    } else if o.starts_with(b"MZ") {
        if contient(b"Inno Setup") {
            Format::Inno
        } else if contient(b"Nullsoft") || contient(b"NullsoftInst") {
            Format::Nsis
        } else {
            Format::Exe
        }
    } else {
        Format::Autre
    }
}

/// Comment installer une version téléchargée.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "sorte", rename_all = "snake_case")]
pub enum Methode {
    /// Installeur connu (Inno, NSIS) : automatique possible.
    Installeur { fichier: String, format: Format },
    /// Installeur inconnu : il s'ouvre, la personne suit les notes.
    InstalleurGuide { fichier: String },
    /// Archive à décompresser.
    Archive { fichier: String, format: Format },
    /// Rien à installer : ROM, image disque, ou jeu déjà prêt (un exécutable).
    Aucune,
}

/// Choisit la méthode d'installation d'après la nature et les fichiers reçus (dans `dossier`).
pub fn methode(nature: Nature, dossier: &Path, fichiers: &[String]) -> Resultat<Methode> {
    if matches!(nature, Nature::Rom | Nature::ImageDisque) {
        return Ok(Methode::Aucune);
    }
    let mut formats = Vec::new();
    for f in fichiers {
        formats.push((f.clone(), format_de(&dossier.join(f))?));
    }
    // 1. Un installeur connu.
    if let Some((f, fmt)) = formats.iter().find(|(_, fmt)| matches!(fmt, Format::Inno | Format::Nsis)) {
        return Ok(Methode::Installeur { fichier: f.clone(), format: *fmt });
    }
    // 2. Un « setup » (repack, installeur d'origine).
    let est_setup = |n: &str| {
        let n = n.to_lowercase();
        n.ends_with(".exe") && (n.starts_with("setup") || n.starts_with("install"))
    };
    if let Some((f, _)) = formats.iter().find(|(f, fmt)| *fmt == Format::Exe && est_setup(f)) {
        return Ok(Methode::InstalleurGuide { fichier: f.clone() });
    }
    // 3. Une archive.
    if let Some((f, fmt)) = formats.iter().find(|(_, fmt)| matches!(fmt, Format::Zip | Format::SeptZip | Format::Rar)) {
        return Ok(Methode::Archive { fichier: f.clone(), format: *fmt });
    }
    // 4. Un exécutable d'une version « à installer » : c'est son installeur.
    if matches!(nature, Nature::Repack | Nature::InstalleurOrigine) {
        if let Some((f, _)) = formats.iter().find(|(_, fmt)| *fmt == Format::Exe) {
            return Ok(Methode::InstalleurGuide { fichier: f.clone() });
        }
    }
    Ok(Methode::Aucune)
}

/// Les arguments d'une installation automatique (Inno : `/VERYSILENT… /DIR=` ; NSIS : `/S /D=` en dernier, sans
/// guillemets, comme l'exige NSIS).
pub fn arguments_silencieux(format: Format, destination: &Path, journal: &Path) -> Vec<String> {
    let dest = destination.to_string_lossy().to_string();
    match format {
        Format::Inno => vec![
            "/VERYSILENT".into(),
            "/SUPPRESSMSGBOXES".into(),
            "/NORESTART".into(),
            "/SP-".into(),
            "/NOICONS".into(),
            "/CURRENTUSER".into(),
            format!("/DIR={dest}"),
            format!("/LOG={}", journal.to_string_lossy()),
        ],
        Format::Nsis => vec!["/S".into(), format!("/D={dest}")],
        _ => vec![],
    }
}

/// Lance un programme et attend sa fin. Si Windows exige les droits administrateur (code 740), le relance en
/// demandant l'accord de Windows (fenêtre de contrôle de compte d'utilisateur). Rend le code de sortie.
pub fn executer_et_attendre(programme: &Path, arguments: &[String], dossier_travail: &Path) -> Resultat<i32> {
    match std::process::Command::new(programme).args(arguments).current_dir(dossier_travail).status() {
        Ok(s) => Ok(s.code().unwrap_or(-1)),
        Err(e) if e.raw_os_error() == Some(740) => executer_eleve(programme, arguments, dossier_travail),
        Err(e) => Err(Erreur::Disque(format!("Impossible de lancer {} ({e}).", programme.display()))),
    }
}

/// Relance avec l'accord de Windows (droits administrateur), et attend.
fn executer_eleve(programme: &Path, arguments: &[String], dossier_travail: &Path) -> Resultat<i32> {
    let echapper = |s: &str| s.replace('\'', "''");
    let args = arguments.iter().map(|a| format!("'{}'", echapper(a))).collect::<Vec<_>>().join(",");
    let liste = if args.is_empty() { String::new() } else { format!("-ArgumentList {args} ") };
    let script = format!(
        "$p = Start-Process -FilePath '{}' {liste}-WorkingDirectory '{}' -Verb RunAs -Wait -PassThru; exit $p.ExitCode",
        echapper(&programme.to_string_lossy()),
        echapper(&dossier_travail.to_string_lossy())
    );
    let s = crate::lancement::outil_sans_fenetre("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .status()
        .map_err(|e| Erreur::Disque(format!("Impossible de demander les droits administrateur ({e}).")))?;
    Ok(s.code().unwrap_or(-1))
}

/// Décompresse une archive zip ou 7z dans `destination`. Rend le nombre de fichiers écrits.
pub fn decompresser(archive: &Path, format: Format, destination: &Path) -> Resultat<usize> {
    std::fs::create_dir_all(destination)?;
    match format {
        Format::Zip => {
            let f = std::fs::File::open(archive)?;
            let mut z = zip::ZipArchive::new(f)
                .map_err(|e| Erreur::Disque(format!("Archive zip illisible ({e}).")))?;
            let mut n = 0;
            for i in 0..z.len() {
                let mut e = z.by_index(i).map_err(|e| Erreur::Disque(format!("Archive zip abîmée ({e}).")))?;
                // `enclosed_name` refuse les chemins qui sortiraient du dossier (« ../ »).
                let Some(relatif) = e.enclosed_name() else { continue };
                let cible = destination.join(relatif);
                if e.is_dir() {
                    std::fs::create_dir_all(&cible)?;
                } else {
                    if let Some(p) = cible.parent() {
                        std::fs::create_dir_all(p)?;
                    }
                    let mut sortie = std::fs::File::create(&cible)?;
                    std::io::copy(&mut e, &mut sortie)?;
                    n += 1;
                }
            }
            Ok(n)
        }
        Format::SeptZip => {
            // Pas `sevenz_rust::decompress_file` : il écrit chaque entrée là où son nom le dit, même « .. » ou « C:… ».
            // Chaque chemin est contrôlé ; un chemin dangereux fait refuser TOUTE l'archive (sauter une entrée d'un
            // bloc 7z compact abîmerait les suivantes).
            let mut r = sevenz_rust::SevenZReader::open(archive, sevenz_rust::Password::empty())
                .map_err(|e| Erreur::Disque(format!("Archive 7z illisible ({e}).")))?;
            if let Some(e) = r.archive().files.iter().find(|e| crate::decompression::chemin_sur(e.name()).is_none()) {
                return Err(Erreur::Refus(format!("L'archive contient un chemin dangereux ({}) : refusée.", e.name())));
            }
            let mut n = 0;
            r.for_each_entries(|e, flux| {
                let Some(nom) = crate::decompression::chemin_sur(e.name()) else { return Ok(true) };
                let cible = destination.join(nom);
                if e.is_directory() {
                    std::fs::create_dir_all(&cible).map_err(sevenz_rust::Error::io)?;
                } else {
                    if let Some(p) = cible.parent() {
                        std::fs::create_dir_all(p).map_err(sevenz_rust::Error::io)?;
                    }
                    let mut sortie = std::fs::File::create(&cible).map_err(sevenz_rust::Error::io)?;
                    std::io::copy(flux, &mut sortie).map_err(sevenz_rust::Error::io)?;
                    n += 1;
                }
                Ok(true)
            })
            .map_err(|e| Erreur::Disque(format!("Archive 7z illisible ({e}).")))?;
            Ok(n)
        }
        Format::Rar => Err(Erreur::Refus(
            "Frogtend ne sait pas encore ouvrir les archives RAR : ouvre-la avec 7-Zip dans le dossier du jeu.".into(),
        )),
        _ => Err(Erreur::Refus("Ce fichier n'est pas une archive.".into())),
    }
}

/// Tous les fichiers sous un dossier (chemins relatifs, séparateur `/`).
pub fn fichiers_de(racine: &Path) -> Resultat<Vec<String>> {
    let mut l = Vec::new();
    let mut pile = vec![racine.to_path_buf()];
    while let Some(d) = pile.pop() {
        for e in std::fs::read_dir(&d)?.flatten() {
            let p = e.path();
            if p.is_dir() {
                pile.push(p);
            } else if let Ok(r) = p.strip_prefix(racine) {
                l.push(r.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    l.sort();
    Ok(l)
}

/// L'état des fichiers d'une installation : taille et date de modification de chacun.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Manifeste {
    pub fichiers: BTreeMap<String, (u64, u64)>,
}

fn etat_fichier(p: &Path) -> Option<(u64, u64)> {
    let m = std::fs::metadata(p).ok()?;
    let date = m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
    Some((m.len(), date))
}

pub fn relever(racine: &Path) -> Resultat<Manifeste> {
    let mut fichiers = BTreeMap::new();
    for r in fichiers_de(racine)? {
        if let Some(e) = etat_fichier(&racine.join(&r)) {
            fichiers.insert(r, e);
        }
    }
    Ok(Manifeste { fichiers })
}

/// Les fichiers NOUVEAUX ou MODIFIÉS depuis l'installation : les parties, les réglages du jeu…
pub fn changes_depuis(racine: &Path, m: &Manifeste) -> Resultat<Vec<String>> {
    Ok(fichiers_de(racine)?
        .into_iter()
        .filter(|r| m.fichiers.get(r) != etat_fichier(&racine.join(r)).as_ref())
        .collect())
}

/// Copie ces fichiers (chemins relatifs à `racine`) dans `destination`, en gardant leur arborescence.
pub fn copier(racine: &Path, relatifs: &[String], destination: &Path) -> Resultat<u64> {
    let mut octets = 0;
    for r in relatifs {
        let cible = destination.join(r);
        if let Some(p) = cible.parent() {
            std::fs::create_dir_all(p)?;
        }
        octets += std::fs::copy(racine.join(r), &cible)?;
    }
    Ok(octets)
}

/// Ce qu'on lance pour jouer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Lanceur {
    pub programme: String,
    #[serde(default)]
    pub arguments: Vec<String>,
    /// Le dossier de travail (celui du programme par défaut).
    pub dossier: String,
}

/// Un programme qui pourrait lancer le jeu, avec une note (plus elle est haute, plus il est probable).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Candidat {
    pub lanceur: Lanceur,
    /// Chemin relatif au dossier d'installation, pour l'affichage.
    pub relatif: String,
    pub note: i32,
}

/// Les programmes qui pourraient lancer le jeu, les plus probables d'abord.
pub fn candidats(racine: &Path, titre: &str) -> Resultat<Vec<Candidat>> {
    let mots: Vec<String> = titre
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|m| m.len() >= 3)
        .map(String::from)
        .collect();
    let mut l = Vec::new();
    for r in fichiers_de(racine)? {
        let bas = r.to_lowercase();
        let nom = bas.rsplit('/').next().unwrap_or(&bas).to_string();
        let ext = nom.rsplit('.').next().unwrap_or("");
        if !matches!(ext, "exe" | "bat" | "cmd" | "lnk") {
            continue;
        }
        const A_ECARTER: &[&str] =
            &["unins", "setup", "install", "vcredist", "dxsetup", "directx", "crash", "report", "update", "patch", "redist"];
        if A_ECARTER.iter().any(|m| nom.contains(m)) {
            continue;
        }
        let profondeur = r.matches('/').count() as i32;
        let mut note = 10 - 3 * profondeur;
        note += 8 * mots.iter().filter(|m| nom.contains(m.as_str())).count() as i32;
        if ["jouer", "play", "lancer", "launch", "start", "game", "jeu"].iter().any(|m| nom.contains(m)) {
            note += 5;
        }
        if ext == "lnk" {
            note += 3;
        }
        if nom == "dosbox.exe" {
            note -= 4; // DOSBox seul, sans sa configuration, ne lance pas le jeu
        }
        let chemin = racine.join(&r);
        let dossier = chemin.parent().unwrap_or(racine).to_string_lossy().to_string();
        let (programme, arguments) = if ext == "bat" || ext == "cmd" {
            ("cmd.exe".to_string(), vec!["/C".to_string(), chemin.to_string_lossy().to_string()])
        } else if ext == "lnk" {
            // Un raccourci s'ouvre par Windows (explorer), qui suit sa cible.
            ("explorer.exe".to_string(), vec![chemin.to_string_lossy().to_string()])
        } else {
            (chemin.to_string_lossy().to_string(), vec![])
        };
        l.push(Candidat { lanceur: Lanceur { programme, arguments, dossier }, relatif: r, note });
    }
    l.sort_by(|a, b| b.note.cmp(&a.note).then(a.relatif.cmp(&b.relatif)));
    Ok(l)
}

/// Découpe une ligne de commande d'émulateur (`-L "cores\snes9x_libretro.dll" -f`) en arguments.
pub fn decouper(ligne: &str) -> Vec<String> {
    let mut l = Vec::new();
    let mut courant = String::new();
    let mut entre_guillemets = false;
    for c in ligne.chars() {
        match c {
            '"' => entre_guillemets = !entre_guillemets,
            c if c.is_whitespace() && !entre_guillemets => {
                if !courant.is_empty() {
                    l.push(std::mem::take(&mut courant));
                }
            }
            c => courant.push(c),
        }
    }
    if !courant.is_empty() {
        l.push(courant);
    }
    l
}

/// Le lanceur d'une ROM ou d'une image disque par un émulateur : ses arguments, puis le fichier du jeu (tel quel).
pub fn lanceur_emulateur(programme: &str, ligne: &str, fichier_du_jeu: &Path) -> Lanceur {
    let mut arguments = decouper(ligne);
    arguments.push(fichier_du_jeu.to_string_lossy().to_string());
    let dossier = Path::new(programme).parent().map(PathBuf::from).unwrap_or_default().to_string_lossy().to_string();
    Lanceur { programme: programme.into(), arguments, dossier }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_nature_vient_de_la_qualite_de_firehouse() {
        assert_eq!(nature("Jeu — prêt à jouer"), Nature::PretAJouer);
        assert_eq!(nature("ROM"), Nature::Rom);
        assert_eq!(nature("Image disque"), Nature::ImageDisque);
        assert_eq!(nature("Repack"), Nature::Repack);
        assert_eq!(nature("Installeur d'origine"), Nature::InstalleurOrigine);
        assert_eq!(nature(""), Nature::NonDite);
    }

    #[test]
    fn le_format_se_reconnait_au_contenu() {
        let mut inno = b"MZP\0".to_vec();
        inno.extend(vec![0u8; 751_000]);
        inno.extend(b"Inno Setup Setup Data");
        assert_eq!(format_des_octets(&inno), Format::Inno);
        assert_eq!(format_des_octets(b"MZ...Nullsoft Install System"), Format::Nsis);
        assert_eq!(format_des_octets(b"MZ... un jeu"), Format::Exe);
        assert_eq!(format_des_octets(b"PK\x03\x04..."), Format::Zip);
        assert_eq!(format_des_octets(&[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C, 0]), Format::SeptZip);
        assert_eq!(format_des_octets(b"Rar!\x1a"), Format::Rar);
        assert_eq!(format_des_octets(b"NES\x1a"), Format::Autre);
    }

    #[test]
    fn la_methode_suit_la_nature_puis_le_contenu() {
        let d = tempfile::tempdir().unwrap();
        let ecrire = |n: &str, o: &[u8]| std::fs::write(d.path().join(n), o).unwrap();
        ecrire("Dune (1992) [MS-DOS].exe", b"MZ.......Inno Setup Setup Data");
        ecrire("Super Mario World (USA).sfc", b"\0\0\0");
        ecrire("jeu.zip", b"PK\x03\x04");
        ecrire("setup.exe", b"MZ inconnu");
        ecrire("jeu.exe", b"MZ le jeu");

        let m = |n: Nature, f: &[&str]| methode(n, d.path(), &f.iter().map(|s| s.to_string()).collect::<Vec<_>>()).unwrap();
        assert_eq!(
            m(Nature::PretAJouer, &["Dune (1992) [MS-DOS].exe"]),
            Methode::Installeur { fichier: "Dune (1992) [MS-DOS].exe".into(), format: Format::Inno }
        );
        assert_eq!(m(Nature::Rom, &["Super Mario World (USA).sfc"]), Methode::Aucune);
        assert_eq!(m(Nature::Rom, &["jeu.zip"]), Methode::Aucune, "une ROM zippée se donne telle quelle à l'émulateur");
        assert_eq!(m(Nature::PretAJouer, &["jeu.zip"]), Methode::Archive { fichier: "jeu.zip".into(), format: Format::Zip });
        assert_eq!(m(Nature::Repack, &["setup.exe", "jeu.zip"]), Methode::InstalleurGuide { fichier: "setup.exe".into() });
        assert_eq!(m(Nature::PretAJouer, &["jeu.exe"]), Methode::Aucune, "un jeu prêt à jouer qui est un exécutable se lance");
        assert_eq!(m(Nature::InstalleurOrigine, &["jeu.exe"]), Methode::InstalleurGuide { fichier: "jeu.exe".into() });
    }

    #[test]
    fn les_arguments_silencieux_d_inno_et_de_nsis() {
        let a = arguments_silencieux(Format::Inno, Path::new("E:\\Jeux\\Dune\\Jeu"), Path::new("C:\\j.log"));
        assert!(a.contains(&"/VERYSILENT".to_string()));
        assert!(a.contains(&"/DIR=E:\\Jeux\\Dune\\Jeu".to_string()));
        let n = arguments_silencieux(Format::Nsis, Path::new("E:\\X"), Path::new("j"));
        assert_eq!(n.last().unwrap(), "/D=E:\\X", "NSIS exige /D= en dernier, sans guillemets");
    }

    #[test]
    fn un_programme_s_execute_et_rend_son_code() {
        let d = tempfile::tempdir().unwrap();
        let script = d.path().join("installe.cmd");
        std::fs::write(&script, "@echo off\r\necho installé> \"%~dp0resultat.txt\"\r\nexit /b 3\r\n").unwrap();
        let code = executer_et_attendre(Path::new("cmd.exe"), &["/C".into(), script.to_string_lossy().into()], d.path()).unwrap();
        assert_eq!(code, 3);
        assert!(d.path().join("resultat.txt").is_file());
    }

    #[test]
    fn un_zip_se_decompresse_sans_sortir_de_son_dossier() {
        let d = tempfile::tempdir().unwrap();
        let archive = d.path().join("jeu.zip");
        {
            let f = std::fs::File::create(&archive).unwrap();
            let mut z = zip::ZipWriter::new(f);
            let o = zip::write::SimpleFileOptions::default();
            z.start_file("JEU/DUNE.BAT", o).unwrap();
            std::io::Write::write_all(&mut z, b"dune.exe").unwrap();
            z.start_file("../dehors.txt", o).unwrap();
            std::io::Write::write_all(&mut z, b"non").unwrap();
            z.finish().unwrap();
        }
        let dest = d.path().join("Jeu");
        assert_eq!(decompresser(&archive, Format::Zip, &dest).unwrap(), 1);
        assert_eq!(std::fs::read(dest.join("JEU").join("DUNE.BAT")).unwrap(), b"dune.exe");
        assert!(!d.path().join("dehors.txt").exists(), "un chemin « ../ » est ignoré");
    }

    #[test]
    fn un_7z_se_decompresse_et_refuse_les_chemins_dangereux() {
        let d = tempfile::tempdir().unwrap();
        let archive = d.path().join("jeu.7z");
        let ecrire_7z = |noms: &[&str]| {
            let mut w = sevenz_rust::SevenZWriter::create(&archive).unwrap();
            for nom in noms {
                let mut e = sevenz_rust::SevenZArchiveEntry::new();
                e.name = nom.to_string();
                e.has_stream = true;
                w.push_archive_entry(e, Some(&b"contenu"[..])).unwrap();
            }
            w.finish().unwrap();
        };
        ecrire_7z(&["JEU/DUNE.BAT", "JEU/DUNE.EXE"]);
        let dest = d.path().join("Jeu");
        assert_eq!(decompresser(&archive, Format::SeptZip, &dest).unwrap(), 2);
        assert_eq!(std::fs::read(dest.join("JEU").join("DUNE.BAT")).unwrap(), b"contenu");
        for piege in ["../dehors.txt", "C:/dehors.txt", "/dehors.txt"] {
            ecrire_7z(&["JEU/DUNE.BAT", piege]);
            let dest = d.path().join("Piege");
            assert!(matches!(decompresser(&archive, Format::SeptZip, &dest), Err(Erreur::Refus(_))), "{piege}");
            assert!(!d.path().join("dehors.txt").exists());
            assert!(!dest.join("JEU").exists(), "rien n'est écrit d'une archive refusée");
        }
    }

    #[test]
    fn les_fichiers_changes_depuis_l_installation_sont_les_parties() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("SAVES")).unwrap();
        std::fs::write(d.path().join("DUNE.EXE"), b"jeu").unwrap();
        std::fs::write(d.path().join("DUNE.CFG"), b"v1").unwrap();
        let m = relever(d.path()).unwrap();
        assert!(changes_depuis(d.path(), &m).unwrap().is_empty());

        std::fs::write(d.path().join("SAVES").join("PARTIE1.SAV"), b"partie").unwrap();
        std::fs::write(d.path().join("DUNE.CFG"), b"v2 plus long").unwrap();
        let changes = changes_depuis(d.path(), &m).unwrap();
        assert_eq!(changes, vec!["DUNE.CFG".to_string(), "SAVES/PARTIE1.SAV".to_string()]);

        let abri = d.path().join("abri");
        assert_eq!(copier(d.path(), &changes, &abri).unwrap(), 18);
        assert_eq!(std::fs::read(abri.join("SAVES").join("PARTIE1.SAV")).unwrap(), b"partie");
    }

    #[test]
    fn les_candidats_au_lancement_mettent_le_plus_probable_en_premier() {
        let d = tempfile::tempdir().unwrap();
        for f in ["unins000.exe", "DOSBox/DOSBox.exe", "Dune.bat", "Outils/config.exe", "vcredist_x86.exe"] {
            let p = d.path().join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, b"x").unwrap();
        }
        let c = candidats(d.path(), "Dune").unwrap();
        let noms: Vec<_> = c.iter().map(|c| c.relatif.as_str()).collect();
        assert_eq!(noms[0], "Dune.bat");
        assert!(!noms.contains(&"unins000.exe") && !noms.contains(&"vcredist_x86.exe"));
        assert_eq!(c[0].lanceur.programme, "cmd.exe", "un .bat se lance par cmd");
    }

    #[test]
    fn la_ligne_de_commande_d_un_emulateur_se_decoupe_et_recoit_le_jeu_tel_quel() {
        assert_eq!(decouper(r#"-L "cores\snes9x_libretro.dll" -f"#), vec!["-L", r"cores\snes9x_libretro.dll", "-f"]);
        let l = lanceur_emulateur(r"E:\RetroArch\retroarch.exe", r#"-L "cores\snes9x_libretro.dll" -f"#, Path::new(r"E:\Jeux\SNES\Super Mario World (USA).sfc"));
        assert_eq!(l.arguments.last().unwrap(), r"E:\Jeux\SNES\Super Mario World (USA).sfc");
        assert_eq!(l.dossier, r"E:\RetroArch", "les cœurs se trouvent depuis le dossier de RetroArch");
    }
}

#[cfg(test)]
mod essai_reel_paquet {
    #[test]
    #[ignore]
    fn decompresser_un_vrai_paquet() {
        let a = std::path::PathBuf::from(std::env::var("FROGTEND_PAQUET").unwrap());
        let d = tempfile::tempdir().unwrap();
        let f = super::format_de(&a).unwrap();
        let t = std::time::Instant::now();
        let n = super::decompresser(&a, f, d.path()).unwrap();
        let exe = d.path().join("rpcs3.exe");
        println!("PAQUET format {:?}, {} fichiers en {} ms, rpcs3.exe {:?} octets", f, n, t.elapsed().as_millis(), std::fs::metadata(&exe).map(|m| m.len()).ok());
        println!("PAQUET total fichiers relus : {}", super::fichiers_de(d.path()).unwrap().len());
    }
}
