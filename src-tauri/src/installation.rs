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

/// La signature d'une archive 7z.
const SIGNATURE_7Z: [u8; 6] = [0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C];

/// Une archive 7z AUTO-EXTRACTIBLE (un `.exe` qui porte une archive 7z, comme le paquet officiel de MAME,
/// « mame0289b_x64.exe ») : l'archive est recopiée dans `sortie`, SANS JAMAIS exécuter le programme ; elle se décompresse
/// ensuite avec les contrôles habituels. Rend `false` si ce n'en est pas une.
pub fn archive_7z_dans_exe(exe: &Path, sortie: &Path) -> Resultat<bool> {
    use std::io::{Seek, SeekFrom};
    let mut f = std::fs::File::open(exe)?;
    // Le programme d'extraction fait quelques centaines de Ko : l'archive commence dans les 8 premiers Mo.
    let mut debut = Vec::new();
    (&mut f).take(8 * 1024 * 1024).read_to_end(&mut debut)?;
    if !debut.starts_with(b"MZ") {
        return Ok(false);
    }
    let mut depuis = 2;
    while let Some(i) = debut[depuis..].windows(SIGNATURE_7Z.len()).position(|w| w == SIGNATURE_7Z) {
        let pos = depuis + i;
        depuis = pos + 1;
        // L'en-tête de départ d'une vraie archive : un CRC non nul, qui correspond aux 20 octets qui le suivent.
        let Some(entete) = debut.get(pos..pos + 32) else { continue };
        let crc = u32::from_le_bytes([entete[8], entete[9], entete[10], entete[11]]);
        if crc == 0 || crc != crc32fast::hash(&entete[12..32]) {
            continue;
        }
        f.seek(SeekFrom::Start(pos as u64))?;
        let mut copie = std::fs::File::create(sortie)?;
        let copiee = std::io::copy(&mut f, &mut copie);
        drop(copie);
        if let Err(e) = copiee {
            let _ = std::fs::remove_file(sortie);
            return Err(e.into());
        }
        // Une vraie archive (pas une simple suite d'octets qui ressemble à la signature) : elle doit s'ouvrir.
        if sevenz_rust::SevenZReader::open(sortie, sevenz_rust::Password::empty()).is_ok() {
            return Ok(true);
        }
        let _ = std::fs::remove_file(sortie);
    }
    Ok(false)
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
            // Une entrée au nom vide (le dossier racine que certains outils ajoutent) est ignorée ; tout autre chemin
            // dangereux fait refuser l'archive.
            // Une entrée sans nom n'est tolérée que si elle ne porte AUCUNE donnée (sinon, la sauter décalerait le bloc).
            let vide_sans_donnees = |e: &sevenz_rust::SevenZArchiveEntry| e.name().is_empty() && (e.is_directory() || !e.has_stream || e.size == 0);
            if let Some(e) = r.archive().files.iter().find(|e| !vide_sans_donnees(e) && crate::decompression::chemin_sur(e.name()).is_none()) {
                return Err(Erreur::Refus(format!("L'archive contient un chemin dangereux ({}) : refusée.", e.name())));
            }
            let mut n = 0;
            r.for_each_entries(|e, flux| {
                let Some(nom) = crate::decompression::chemin_sur(e.name()) else {
                    // Lue jusqu'au bout quand même : les entrées suivantes du bloc restent à leur place.
                    std::io::copy(flux, &mut std::io::sink()).map_err(sevenz_rust::Error::io)?;
                    return Ok(true);
                };
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

/// Le nom court MAME d'un jeu lancé par MAME (`mame*.exe`) avec son zip (« sfiii.zip » → « sfiii »), sinon `None`.
pub fn nom_court_mame(programme: &str, fichier_du_jeu: &str) -> Option<String> {
    let exe = Path::new(programme).file_name()?.to_string_lossy().to_lowercase();
    let f = Path::new(fichier_du_jeu);
    let est_zip = f.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip"));
    (exe.starts_with("mame") && est_zip && f.parent().is_none_or(|p| p.as_os_str().is_empty()))
        .then(|| f.file_stem().map(|s| s.to_string_lossy().to_string()))
        .flatten()
}

/// Les dossiers de ROM déjà réglés dans MAME (`rompath` de son `mame.ini`), sinon « roms », son réglage d'origine.
pub fn rompath_de_mame_ini(texte: Option<&str>) -> String {
    texte
        .and_then(|t| {
            t.lines().find_map(|l| {
                let l = l.trim();
                let (cle, valeur) = l.split_once(char::is_whitespace)?;
                cle.eq_ignore_ascii_case("rompath").then(|| valeur.trim().trim_matches('"').to_string())
            })
        })
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "roms".into())
}

/// Le lanceur d'un jeu MAME (contrat 1.13) : `mame <options> <nom court> -rompath "<dossier du jeu>;<ses dossiers>"`
/// — MAME trouve le zip du jeu et ses CHD (`<rompath>/<parent>/`) par le nom court, jamais par le chemin du zip. Le
/// dossier du jeu s'AJOUTE aux dossiers réglés dans MAME (un jeu importé peut avoir ses CHD ou BIOS ailleurs).
/// La personne a-t-elle déjà son propre script de démarrage MAME (`autoboot_script` / `autoboot_command`, dans la
/// ligne de commande ou dans `mame.ini`) ? MAME n'en charge qu'un : on ne remplace jamais le sien.
pub fn autoboot_libre(dossier_mame: &Path, ligne: &str) -> bool {
    if ligne.to_ascii_lowercase().contains("autoboot") {
        return false;
    }
    let ini = std::fs::read_to_string(dossier_mame.join("mame.ini")).unwrap_or_default();
    !ini.lines().any(|l| {
        let mut m = l.split_whitespace();
        matches!(m.next(), Some(c) if c.eq_ignore_ascii_case("autoboot_script") || c.eq_ignore_ascii_case("autoboot_command"))
            && m.next().is_some()
    })
}

/// `script` : le script du menu en jeu (`pilotage::SCRIPT_MAME`, relatif au dossier de MAME), s'il est préparé.
/// `-rompath` de MAME sépare ses dossiers par « ; », sans échappement : un dossier de jeu qui en contient serait coupé
/// en deux, et le jeu introuvable. La ROM n'est jamais renommée : on le dit à la personne.
pub fn verifier_rompath(dossier: &str) -> crate::erreurs::Resultat<()> {
    if dossier.contains(';') {
        return Err(crate::erreurs::Erreur::Refus(format!(
            "MAME ne sait pas lire un dossier dont le nom contient « ; » ({dossier}). Déplace le jeu dans un dossier sans « ; » (le nom de la ROM, lui, ne change pas)."
        )));
    }
    Ok(())
}

pub fn lanceur_mame(programme: &str, ligne: &str, nom_court: &str, rompath: &Path, script: Option<&str>) -> crate::erreurs::Resultat<Lanceur> {
    if let Some(v) = variable_inconnue(ligne) {
        return Err(refus_variable(&v));
    }
    // Frogtend construit lui-même le nom court et `-rompath` (avec celui de mame.ini) : les variables du conseil
    // (`%rom_nom% -rompath "%rom_dossier%"`, contrat 1.16) et le `-rompath` qui les précède sont retirés.
    let brut = decouper(ligne);
    let mut arguments: Vec<String> = Vec::new();
    let mut i = 0;
    while i < brut.len() {
        let a = &brut[i];
        let variable = |x: &str| x.to_ascii_lowercase().contains("%rom");
        if variable(a) {
            i += 1;
            continue;
        }
        if a.eq_ignore_ascii_case("-rompath") && brut.get(i + 1).is_some_and(|x| variable(x)) {
            i += 2;
            continue;
        }
        arguments.push(a.clone());
        i += 1;
    }
    if let Some(s) = script {
        arguments.push("-autoboot_script".into());
        arguments.push(s.into());
    }
    let ini = Path::new(programme).parent().and_then(|d| std::fs::read_to_string(d.join("mame.ini")).ok());
    arguments.push(nom_court.to_string());
    arguments.push("-rompath".into());
    arguments.push(format!("{};{}", rompath.to_string_lossy(), rompath_de_mame_ini(ini.as_deref())));
    let dossier = Path::new(programme).parent().map(PathBuf::from).unwrap_or_default().to_string_lossy().to_string();
    Ok(Lanceur { programme: programme.into(), arguments, dossier })
}

/// Les variables d'une ligne de commande d'émulateur (contrat 1.16), sans casse : le fichier du jeu, son nom sans
/// extension, son dossier (sans barre finale).
const VARIABLES: [&str; 3] = ["%rom_dossier%", "%rom_nom%", "%rom%"];

/// La première variable (`%nom%`) de la ligne que Frogtend ne connaît pas : le contrat demande de ne PAS lancer.
/// Cherchée sur la LIGNE, avant tout remplacement (un nom de fichier qui contiendrait `%x%` ne compte pas).
pub fn variable_inconnue(ligne: &str) -> Option<String> {
    let b = ligne.as_bytes();
    let mut i = 0;
    while let Some(d) = ligne[i..].find('%').map(|p| i + p) {
        let fin = ligne[d + 1..].find('%').map(|p| d + 1 + p);
        let Some(f) = fin else { break };
        let nom = &ligne[d + 1..f];
        if !nom.is_empty() && nom.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') {
            let v = format!("%{nom}%");
            if !VARIABLES.iter().any(|k| k.eq_ignore_ascii_case(&v)) {
                return Some(v);
            }
            i = f + 1;
        } else {
            i = d + 1;
        }
        if i >= b.len() {
            break;
        }
    }
    None
}

fn refus_variable(v: &str) -> crate::erreurs::Erreur {
    crate::erreurs::Erreur::Refus(format!(
        "La ligne de commande de l'émulateur contient « {v} », que Frogtend ne connaît pas : le jeu n'est pas lancé. Corrige-la dans Réglages ▸ Émulateurs."
    ))
}

/// Remplace, sans casse et en UNE seule passe sur le texte d'origine, chaque variable par sa valeur (un chemin qui
/// contiendrait lui-même « %rom% » n'est jamais remplacé une seconde fois). Rend aussi : une variable a-t-elle été vue ?
fn remplacer_variables(texte: &str, valeurs: &[&str; 3]) -> (String, bool) {
    let bas = texte.to_ascii_lowercase();
    let mut r = String::new();
    let mut i = 0;
    let mut vu = false;
    'texte: while i < texte.len() {
        for (motif, valeur) in VARIABLES.iter().zip(valeurs) {
            if bas[i..].starts_with(motif) {
                r.push_str(valeur);
                i += motif.len();
                vu = true;
                continue 'texte;
            }
        }
        let c = texte[i..].chars().next().unwrap_or_default();
        r.push(c);
        i += c.len_utf8().max(1);
    }
    (r, vu)
}

/// Le lanceur d'une ROM ou d'une image disque par un émulateur : ses arguments, puis le fichier du jeu (tel quel).
/// Une ligne qui dit OÙ mettre le jeu (contrat 1.16 : `%rom%` le fichier, `%rom_nom%` son nom sans extension,
/// `%rom_dossier%` son dossier sans barre finale ; la forme du conseil de Firehouse, ex. `-L "cores\snes9x_libretro.dll"
/// "%rom%"`, `"%rom%" -localdata`) le reçoit à ces endroits ; sans variable, il va à la fin. Une variable inconnue :
/// refusé (le contrat demande de ne pas lancer). Jamais de shell : le programme reçoit ses arguments un par un.
pub fn lanceur_emulateur(programme: &str, ligne: &str, fichier_du_jeu: &Path) -> crate::erreurs::Resultat<Lanceur> {
    if let Some(v) = variable_inconnue(ligne) {
        return Err(refus_variable(&v));
    }
    let jeu = fichier_du_jeu.to_string_lossy().to_string();
    let nom = fichier_du_jeu.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    // `D:\` reste `D:\` (« D: » seul voudrait dire le dossier courant de D:) ; sinon sans barre finale.
    let dossier_du_jeu = fichier_du_jeu.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let valeurs = [dossier_du_jeu.as_str(), nom.as_str(), jeu.as_str()];
    let mut place = false;
    let mut arguments: Vec<String> = decouper(ligne)
        .into_iter()
        .map(|a| {
            let (r, vu) = remplacer_variables(&a, &valeurs);
            place |= vu;
            r
        })
        .collect();
    if !place {
        arguments.push(jeu);
    }
    let dossier = Path::new(programme).parent().map(PathBuf::from).unwrap_or_default().to_string_lossy().to_string();
    Ok(Lanceur { programme: programme.into(), arguments, dossier })
}

/// Une ligne de MAME à la forme « machine de salon » du conseil (contrat 1.16 : `studio2 -cart "%rom%"`) : le fichier
/// de la cartouche à sa place, même zippé — jamais le lanceur de l'arcade (nom court + `-rompath`).
pub fn ligne_mame_de_salon(ligne: &str) -> bool {
    let l = ligne.to_ascii_lowercase();
    l.contains("%rom%") && !l.contains("%rom_nom%")
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
        let l = lanceur_emulateur(r"E:\RetroArch\retroarch.exe", r#"-L "cores\snes9x_libretro.dll" -f"#, Path::new(r"E:\Jeux\SNES\Super Mario World (USA).sfc")).unwrap();
        assert_eq!(l.arguments.last().unwrap(), r"E:\Jeux\SNES\Super Mario World (USA).sfc");
        assert_eq!(l.dossier, r"E:\RetroArch", "les cœurs se trouvent depuis le dossier de RetroArch");
    }

    #[test]
    fn une_archive_7z_auto_extractible_se_lit_sans_lancer_le_programme() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("mame");
        std::fs::create_dir_all(src.join("hash")).unwrap();
        std::fs::write(src.join("mame.exe"), b"MZ le vrai mame").unwrap();
        std::fs::write(src.join("hash").join("x.xml"), b"<x/>").unwrap();
        let a7 = d.path().join("a.7z");
        sevenz_rust::compress_to_path(&src, &a7).unwrap();
        // Un faux programme d'extraction : « MZ », du remplissage qui contient un faux début de signature, puis l'archive.
        let mut exe = b"MZ".to_vec();
        exe.extend(vec![0u8; 5000]);
        exe.extend(&SIGNATURE_7Z[..4]);
        exe.extend(vec![1u8; 300]);
        // Une signature complète suivie de déchets (CRC nul, puis CRC faux) : écartée sans être analysée.
        exe.extend(&SIGNATURE_7Z);
        exe.extend(vec![0u8; 40]);
        exe.extend(&SIGNATURE_7Z);
        exe.extend([0, 4, 1, 2, 3, 4]);
        exe.extend(vec![7u8; 40]);
        exe.extend(std::fs::read(&a7).unwrap());
        let sfx = d.path().join("mame0289b_x64.exe");
        std::fs::write(&sfx, &exe).unwrap();
        assert_eq!(format_de(&sfx).unwrap(), Format::Exe);
        let sortie = d.path().join("sorti.7z");
        assert!(archive_7z_dans_exe(&sfx, &sortie).unwrap());
        let dest = d.path().join("dest");
        decompresser(&sortie, Format::SeptZip, &dest).unwrap();
        assert_eq!(std::fs::read(dest.join("mame.exe")).unwrap(), b"MZ le vrai mame");
        // Un programme ordinaire (pas d'archive dedans) : non.
        let simple = d.path().join("simple.exe");
        std::fs::write(&simple, b"MZ rien du tout").unwrap();
        assert!(!archive_7z_dans_exe(&simple, &d.path().join("rien.7z")).unwrap());
    }

    #[test]
    fn un_dossier_avec_un_point_virgule_est_refuse_pour_mame_avec_une_explication() {
        assert!(verifier_rompath(r"E:\Jeux d'arcade, MAME\Street Fighter").is_ok(), "apostrophe et virgule passent");
        match verifier_rompath(r"E:\Arcade; Capcom\sfiii") {
            Err(crate::erreurs::Erreur::Refus(m)) => assert!(m.contains("« ; »") && m.contains("ROM")),
            r => panic!("{r:?}"),
        }
    }

    #[test]
    fn mame_se_lance_par_le_nom_court_et_le_dossier_des_zips() {
        // Relevé réel du 16/10 : sfiiia (clone de sfiii), sa CHD dans <rompath>/sfiii/.
        assert_eq!(nom_court_mame(r"E:\Emulateurs\MAME\mame.exe", "sfiiia.zip").as_deref(), Some("sfiiia"));
        assert_eq!(nom_court_mame(r"E:\Emulateurs\MAME\mame.exe", "sfiii/cap-sf3-3.chd"), None, "jamais la CHD");
        assert_eq!(nom_court_mame(r"E:\Emulateurs\RetroArch\retroarch.exe", "sfiiia.zip"), None, "FBNeo prend le chemin");
        let avec = lanceur_mame(r"E:\Emulateurs\MAME\mame.exe", "-skip_gameinfo", "sfiiia", Path::new(r"E:\Jeux"), Some(r"frogtend\menu-en-jeu.lua")).unwrap();
        assert_eq!(avec.arguments[..4], ["-skip_gameinfo", "-autoboot_script", r"frogtend\menu-en-jeu.lua", "sfiiia"]);
        // Le script de démarrage de la personne : jamais remplacé.
        let d = tempfile::tempdir().unwrap();
        assert!(autoboot_libre(d.path(), "-skip_gameinfo"));
        assert!(!autoboot_libre(d.path(), "-autoboot_script mon.lua"));
        std::fs::write(d.path().join("mame.ini"), "autoboot_command          \n").unwrap();
        assert!(autoboot_libre(d.path(), ""), "réglage vide : libre");
        std::fs::write(d.path().join("mame.ini"), "autoboot_script           C:\\mes\\scripts\\boot.lua\n").unwrap();
        assert!(!autoboot_libre(d.path(), ""));
        let l = lanceur_mame(r"E:\Emulateurs\MAME\mame.exe", "", "sfiiia", Path::new(r"E:\Jeux\Arcade\Street Fighter III"), None).unwrap();
        assert_eq!(l.arguments, ["sfiiia", "-rompath", r"E:\Jeux\Arcade\Street Fighter III;roms"]);
        // Les dossiers déjà réglés dans MAME sont gardés (un jeu importé peut y avoir ses CHD).
        let ini = "#\n# CORE SEARCH PATH OPTIONS\n#\nhomepath                  .\nrompath                   \"roms;E:\\CHD\"\n";
        assert_eq!(rompath_de_mame_ini(Some(ini)), r"roms;E:\CHD");
        assert_eq!(rompath_de_mame_ini(Some("hashpath hash\n")), "roms");
        assert_eq!(rompath_de_mame_ini(None), "roms");
        assert_eq!(l.dossier, r"E:\Emulateurs\MAME");
    }

    #[test]
    fn la_ligne_du_conseil_de_firehouse_recoit_le_jeu_a_la_place_de_rom() {
        // Relevé réel du 16/10 (SNES) : le jeu remplace « %rom% », il n'est pas ajouté une deuxième fois.
        let jeu = Path::new(r"E:\Jeux\SNES\Super Mario World (USA).sfc");
        let l = lanceur_emulateur(r"E:\RetroArch\retroarch.exe", r#"-L "cores\snes9x_libretro.dll" "%rom%""#, jeu).unwrap();
        assert_eq!(l.arguments, vec!["-L", r"cores\snes9x_libretro.dll", r"E:\Jeux\SNES\Super Mario World (USA).sfc"]);
        // Au milieu de la ligne, collé à une option, en majuscules : à sa place aussi.
        let l = lanceur_emulateur("emu.exe", r#"--jeu=%ROM% -f"#, jeu).unwrap();
        assert_eq!(l.arguments, vec![r"--jeu=E:\Jeux\SNES\Super Mario World (USA).sfc", "-f"]);
        // Un conseil d'émulateur autonome (ligne vide) : le jeu seul.
        assert_eq!(lanceur_emulateur("pcsx2-qt.exe", "", jeu).unwrap().arguments, vec![r"E:\Jeux\SNES\Super Mario World (USA).sfc"]);
    }

    #[test]
    fn contrat_1_16_les_trois_variables_sont_remplacees_et_une_inconnue_refuse_le_lancement() {
        let jeu = Path::new(r"E:\Jeux\Arcade\sfiii.zip");
        let l = lanceur_emulateur(r"E:\MAME\mame.exe", r#"%rom_nom% -rompath "%ROM_DOSSIER%""#, jeu).unwrap();
        assert_eq!(l.arguments, vec!["sfiii", "-rompath", r"E:\Jeux\Arcade"], "nom sans extension, dossier sans barre finale");
        assert_eq!(l.dossier, r"E:\MAME", "dossier de travail : celui de l'émulateur");
        // BigPEmu : l'image EN PREMIER.
        let l = lanceur_emulateur(r"E:\BigPEmu\BigPEmu.exe", r#""%rom%" -localdata"#, Path::new(r"E:\Jag\Doom.j64")).unwrap();
        assert_eq!(l.arguments, vec![r"E:\Jag\Doom.j64", "-localdata"]);
        // Un jeu à la racine d'un disque : `D:\`, jamais « D: » (le dossier courant de D:).
        let l = lanceur_emulateur("emu.exe", r#"-d "%rom_dossier%""#, Path::new(r"D:\jeu.bin")).unwrap();
        assert_eq!(l.arguments, vec!["-d", r"D:\"]);
        // Une variable inconnue : pas de lancement, et la raison est dite.
        match lanceur_emulateur("emu.exe", r#"-x "%rom_inconnue%" "%rom%""#, jeu) {
            Err(crate::erreurs::Erreur::Refus(m)) => assert!(m.contains("%rom_inconnue%"), "{m}"),
            r => panic!("{:?}", r.map(|l| l.arguments)),
        }
        assert!(lanceur_mame(r"E:\MAME\mame.exe", "%x%", "sfiii", Path::new(r"E:\Jeux"), None).is_err());
        // Un nom de FICHIER qui contient des « % » ne bloque rien (seule la ligne est examinée), et un dossier qui
        // contient « %rom% » n'est jamais remplacé une seconde fois (une seule passe).
        assert!(lanceur_emulateur("emu.exe", r#""%rom%""#, Path::new(r"E:\100%fun%.bin")).is_ok());
        let l = lanceur_emulateur("emu.exe", r#""%rom_dossier%" "%rom%""#, Path::new(r"E:\a%rom%b\jeu.bin")).unwrap();
        assert_eq!(l.arguments, vec![r"E:\a%rom%b", r"E:\a%rom%b\jeu.bin"]);
        assert_eq!(variable_inconnue("-f 50% de plus"), None);
    }

    /// Essai RÉEL (lecture seule, rien n'est lancé) : les lignes du conseil 1.16 de Firehouse pour quelques consoles, et
    /// la commande que Frogtend en tirerait. `FROGTEND_PROFIL_ESSAI=<id> cargo test essai_lignes_1_16 -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_lignes_1_16() {
        use crate::coffre::Coffre as _;
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").unwrap();
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton");
        let c = crate::firehouse::Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap();
        for p in ["Nintendo GameCube", "Sony Playstation 2", "Sony Playstation 3", "Arcade", "RCA Studio II", "Atari Jaguar", "Microsoft Xbox"] {
            let v: serde_json::Value = c.obtenir_json(&format!("/emulateurs?plateforme={}", p.replace(' ', "%20"))).await.unwrap();
            let k = &v["conseil"];
            let ligne = k["ligne_de_commande"].as_str().unwrap_or("");
            let jeu = Path::new(r"E:\Jeux\Exemple\Jeu (Europe).zip");
            let r = if k["emulateur"] == "mame" && !ligne_mame_de_salon(ligne) {
                lanceur_mame(r"E:\MAME\mame.exe", ligne, "jeu", Path::new(r"E:\Jeux\Exemple"), None).map(|l| l.arguments)
            } else {
                lanceur_emulateur(r"E:\Emu\emu.exe", ligne, jeu).map(|l| l.arguments)
            };
            println!("CONSEIL {p} : {} | ligne {ligne:?} | commande {:?}", k["emulateur"], r.map_err(|e| e.to_string()));
        }
    }

    #[test]
    fn contrat_1_16_mame_arcade_et_machines_de_salon() {
        // L'arcade d'après le conseil (`%rom_nom% -rompath "%rom_dossier%"`) : Frogtend garde SA commande (avec le
        // rompath de mame.ini), les variables du conseil et leur `-rompath` sont retirés.
        let l = lanceur_mame(r"E:\MAME\mame.exe", r#"%rom_nom% -rompath "%rom_dossier%""#, "sfiii", Path::new(r"E:\Jeux\Arcade"), None).unwrap();
        assert_eq!(l.arguments, vec!["sfiii", "-rompath", r"E:\Jeux\Arcade;roms"]);
        // Une machine de salon (`studio2 -cart "%rom%"`) : sa cartouche à sa place, même zippée.
        assert!(ligne_mame_de_salon(r#"studio2 -cart "%rom%""#));
        assert!(!ligne_mame_de_salon(r#"%rom_nom% -rompath "%rom_dossier%""#));
        assert!(!ligne_mame_de_salon(""), "un ancien réglage sans variable reste l'arcade");
        let l = lanceur_emulateur(r"E:\MAME\mame.exe", r#"studio2 -cart "%rom%""#, Path::new(r"E:\Jeux\Studio II\Space War.zip")).unwrap();
        assert_eq!(l.arguments, vec!["studio2", "-cart", r"E:\Jeux\Studio II\Space War.zip"]);
        assert_eq!(l.dossier, r"E:\MAME", "ses ROM système dans roms/ du dossier de MAME");
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

#[cfg(test)]
mod essai_reel_sfx {
    /// RÉEL : une archive 7z officielle (RetroArch…) décompressée par le code de Frogtend dans `FROGTEND_DEST`.
    /// `FROGTEND_7Z=<archive> FROGTEND_DEST=<dossier> cargo test essai_decompresser_7z -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn essai_decompresser_7z() {
        let a = std::path::PathBuf::from(std::env::var("FROGTEND_7Z").unwrap());
        let d = std::path::PathBuf::from(std::env::var("FROGTEND_DEST").unwrap());
        let n = super::decompresser(&a, super::Format::SeptZip, &d).unwrap();
        println!("FICHIERS {n}");
    }

    /// RÉEL : le paquet officiel de MAME (archive 7z auto-extractible) se décompresse SANS être exécuté.
    /// `FROGTEND_SFX=<chemin du .exe> cargo test essai_paquet_mame -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn essai_paquet_mame() {
        let exe = std::path::PathBuf::from(std::env::var("FROGTEND_SFX").unwrap());
        let d = exe.parent().unwrap().to_path_buf();
        let archive = d.join("extrait.7z");
        let debut = std::time::Instant::now();
        println!("ARCHIVE TROUVEE {}", super::archive_7z_dans_exe(&exe, &archive).unwrap());
        let n = super::decompresser(&archive, super::Format::SeptZip, &d.join("MAME")).unwrap();
        println!("FICHIERS {n} en {:?} ; mame.exe présent : {}", debut.elapsed(), d.join("MAME").join("mame.exe").is_file());
    }
}
