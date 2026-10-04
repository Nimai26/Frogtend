//! Importer des jeux déjà présents sur le disque (menu « Importer », comme LaunchBox ; Seb, 02/10) : ROM et images
//! disque d'un dossier, jeux MS-DOS (un sous-dossier par jeu), jeu Windows (son programme), ajout manuel.
//!
//! Règles :
//! - **rien n'est déplacé, copié ni renommé** : Frogtend note où est le jeu, c'est tout (les émulateurs et les bases de
//!   jaquettes reconnaissent une ROM par son nom officiel) ;
//! - **rien ne s'efface** : retirer un jeu importé le retire de la ludothèque, jamais du disque (voir
//!   `VERSION_IMPORTEE` dans `partie.rs`) ;
//! - une opération de masse (un dossier de ROM) est **comptée et montrée** avant d'être faite.

use crate::erreurs::{Erreur, Resultat};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// La « version » d'un jeu importé dans le registre du PC : il n'a pas été téléchargé par Frogtend, ses fichiers
/// appartiennent à la personne et ne sont JAMAIS effacés par Frogtend.
pub const VERSION_IMPORTEE: i64 = -1;

/// Au-delà, on refuse de parcourir (un disque entier choisi par erreur) : la personne choisit un dossier plus précis.
pub const MAX_FICHIERS_PARCOURUS: usize = 200_000;

/// Le titre d'un jeu d'après son nom de fichier : sans extension, sans les étiquettes No-Intro / Redump / TOSEC
/// (« (Europe) », « [!] », « (Disc 1) »…), les « _ » en espaces. Le FICHIER, lui, garde son nom.
pub fn titre_depuis_nom(nom: &str) -> String {
    let base = Path::new(nom).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| nom.to_string());
    let mut t = String::new();
    let mut profondeur = 0i32;
    for c in base.chars() {
        match c {
            '(' | '[' => profondeur += 1,
            ')' | ']' => profondeur = (profondeur - 1).max(0),
            c if profondeur == 0 => t.push(if c == '_' { ' ' } else { c }),
            _ => {}
        }
    }
    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.is_empty() { base } else { t }
}

/// Ce que disent les étiquettes d'un nom de ROM (No-Intro, Redump, TOSEC, GoodTools) : (rang de région, qualité,
/// libellé lisible, numéro de disque). Rang : 0 fr, 1 eu, 2 us/en, 3 autres (préférences de Seb, 02/10).
pub fn etiquettes(nom: &str) -> (u8, i32, String, Option<u32>) {
    let base = Path::new(nom).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| nom.to_string());
    let mut rang = 3u8;
    let mut qualite = 0i32;
    let mut libelles: Vec<String> = Vec::new();
    let mut disque = None;
    let mut rang_traduction: Option<u8> = None;
    let mut reste = base.as_str();
    while let Some(i) = reste.find(['(', '[']) {
        let fermant = if reste.as_bytes()[i] == b'(' { ')' } else { ']' };
        let Some(j) = reste[i + 1..].find(fermant) else { break };
        let contenu = &reste[i + 1..i + 1 + j];
        let crochet = fermant == ']';
        reste = &reste[i + 2 + j..];
        let bas = contenu.trim().to_lowercase();
        if crochet {
            // Traduction de fan (règle de Seb, 03/10) : « [T-Fr by <groupe>] » compte comme une version FRANÇAISE,
            // « [T-En by <groupe>] » comme une version anglaise — plus comme un hack. Juste après la version
            // officielle de la même langue (qualité +1), et c'est dit en clair dans le libellé.
            if let Some((langue, groupe)) = traduction(contenu.trim()) {
                // La langue de la traduction l'emporte sur la région du jeu d'origine (« (France) [T-En by …] » se
                // joue en anglais).
                rang_traduction = Some(if langue == "fr" { 0 } else { 2 });
                qualite += 1;
                let quoi = if langue == "fr" { "Traduction française" } else { "Traduction anglaise" };
                libelles.push(match groupe {
                    Some(g) => format!("{quoi} par {g}"),
                    None => quoi.to_string(),
                });
                continue;
            }
            match bas.chars().next() {
                Some('!') => qualite -= 1,
                Some('b') => qualite += 5,
                Some('h') | Some('t') | Some('f') => qualite += 3,
                Some('o') => qualite += 2,
                Some('a') => qualite += 1,
                _ => {}
            }
            continue;
        }
        // Disque : « Disc 2 », « Disk 1 of 3 », « CD2 ».
        let mots: Vec<&str> = bas.split_whitespace().collect();
        if let Some(n) = match mots.as_slice() {
            ["disc" | "disk" | "cd", n, ..] => n.parse().ok(),
            [m] if m.starts_with("cd") => m[2..].parse().ok(),
            _ => None,
        } {
            disque = Some(n);
            continue;
        }
        if ["beta", "proto", "prototype", "demo", "sample", "hack", "unl", "pirate"].iter().any(|m| bas.starts_with(m)) {
            qualite += 4;
        }
        let mut vu = false;
        for jeton in contenu.split(',').map(str::trim) {
            let b = jeton.to_lowercase();
            let r = match b.as_str() {
                "france" | "fr" | "french" | "fre" | "f" => Some(0),
                "europe" | "eu" | "eur" | "e" | "uk" | "germany" | "spain" | "italy" | "netherlands" | "sweden"
                | "scandinavia" | "australia" | "de" | "es" | "it" | "nl" | "sv" | "g" | "s" | "i" => Some(1),
                "usa" | "us" | "u" | "world" | "w" | "en" | "english" | "canada" => Some(2),
                "japan" | "ja" | "j" | "korea" | "k" | "china" | "asia" | "brazil" | "taiwan" => Some(3),
                // GoodTools : plusieurs lettres collées (« JUE », « UE »).
                _ if jeton.len() <= 4 && jeton.chars().all(|c| "JUEFGSIAKW".contains(c)) => jeton
                    .chars()
                    .map(|c| match c {
                        'F' => 0,
                        'E' | 'G' | 'S' | 'I' | 'A' => 1,
                        'U' | 'W' => 2,
                        _ => 3,
                    })
                    .min(),
                _ => None,
            };
            if let Some(r) = r {
                rang = rang.min(r);
                vu = true;
            }
        }
        if vu || !bas.is_empty() {
            libelles.push(contenu.trim().to_string());
        }
    }
    (rang_traduction.unwrap_or(rang), qualite, libelles.join(", "), disque)
}

/// Une étiquette de traduction de fan : (« fr » ou « en », groupe). Forme retenue avec Firehouse : « T-Fr by
/// <groupe> » ; les variantes GoodTools / No-Intro sont reconnues aussi (« T+Fre », « T-Fr v1.1 by … », « T-Eng »).
fn traduction(etiquette: &str) -> Option<(&'static str, Option<String>)> {
    let e = etiquette.trim();
    let reste = e.strip_prefix(['T', 't'])?.strip_prefix(['-', '+'])?;
    let mot: String = reste.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    let langue = match mot.to_lowercase().as_str() {
        "fr" | "fre" | "french" => "fr",
        "en" | "eng" | "english" => "en",
        _ => return None,
    };
    // Découpe sur le texte d'origine (jamais sur sa version en minuscules, dont les positions peuvent différer).
    let groupe = [" by ", " By ", " BY "]
        .iter()
        .find_map(|s| e.split_once(s))
        .map(|(_, g)| g.trim().to_string())
        .filter(|g| !g.is_empty());
    Some((langue, groupe))
}

/// La clé qui réunit les versions d'un même jeu sur un même système : titre nettoyé, sans casse ni ponctuation.
pub fn cle_de_jeu(plateforme: &str, titre: &str) -> String {
    let t: String = titre.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect();
    format!("{}|{t}", plateforme.to_lowercase())
}

/// Range des fichiers d'un même jeu en versions : les disques d'une même version restent ensemble (le 1er est
/// lancé), puis les versions sont classées fr, eu, us/en, autres ; à région égale, la meilleure qualité d'abord.
pub fn en_versions(chemins: &[String]) -> Vec<crate::jeux_pc::VersionLocale> {
    use crate::jeux_pc::VersionLocale;
    let mut par_version: std::collections::BTreeMap<String, (u8, i32, String, Vec<(u32, String)>)> = Default::default();
    for c in chemins {
        let nom = Path::new(c).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let (rang, qualite, libelle, disque) = etiquettes(&nom);
        // La version : le nom sans son numéro de disque.
        let sans_disque: String = {
            let bas = nom.to_lowercase();
            let mut s = bas.clone();
            if disque.is_some() {
                for motif in ["(disc ", "(disk ", "(cd"] {
                    if let Some(i) = bas.find(motif) {
                        if let Some(j) = bas[i..].find(')') {
                            s = format!("{}{}", &bas[..i], &bas[i + j + 1..]);
                        }
                    }
                }
            }
            let d = Path::new(c).parent().map(|p| p.to_string_lossy().to_lowercase()).unwrap_or_default();
            format!("{d}|{}", s.split_whitespace().collect::<Vec<_>>().join(" "))
        };
        let e = par_version.entry(sans_disque).or_insert((rang, qualite, libelle, Vec::new()));
        e.3.push((disque.unwrap_or(1), c.clone()));
    }
    let mut l: Vec<VersionLocale> = par_version
        .into_values()
        .map(|(rang, qualite, libelle, mut disques)| {
            disques.sort();
            let chemin = disques[0].1.clone();
            let disques = if disques.len() > 1 { disques.into_iter().map(|(_, c)| c).collect() } else { vec![] };
            VersionLocale { chemin, libelle, rang, qualite, disques }
        })
        .collect();
    l.sort_by(|a, b| (a.rang, a.qualite, &a.libelle, &a.chemin).cmp(&(b.rang, b.qualite, &b.libelle, &b.chemin)));
    l
}

/// Une ROM (ou image disque) trouvée.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RomTrouvee {
    /// Chemin complet du fichier (tel quel, jamais renommé).
    pub chemin: String,
    pub titre: String,
    pub taille: u64,
}

/// Le texte d'une feuille (.cue, .m3u, .gdi) : UTF-8 ou ANSI (Windows-1252, accents), sans marque d'ordre d'octets.
pub fn lire_feuille(fichier: &Path) -> Option<String> {
    // La taille d'abord : une feuille fait quelques Ko ; une image disque de 47 Go ne doit JAMAIS être lue en mémoire.
    if std::fs::metadata(fichier).ok()?.len() > 1 << 20 {
        return None;
    }
    let o = std::fs::read(fichier).ok()?;
    let o = o.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&o);
    Some(match std::str::from_utf8(o) {
        Ok(t) => t.to_string(),
        // ANSI : chaque octet est un caractère (assez pour retrouver les noms de fichiers).
        Err(_) => o.iter().map(|&b| b as char).collect(),
    })
}

/// Les fichiers qu'une liste de pistes (`.cue`, `.m3u`, `.gdi`) désigne : ils ne sont pas des jeux à part.
fn pistes_designees(fichier: &Path) -> Vec<PathBuf> {
    // Seules les listes de pistes sont lues (un .zip ou une image disque ne l'est jamais).
    let ext = fichier.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    if !matches!(ext.as_str(), "cue" | "m3u" | "gdi") {
        return vec![];
    }
    let Some(texte) = lire_feuille(fichier) else { return vec![] };
    let dossier = fichier.parent().unwrap_or(Path::new("."));
    texte
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            match ext.as_str() {
                // FILE "Jeu (Track 1).bin" BINARY
                "cue" => (l.len() > 5 && l[..5].eq_ignore_ascii_case("FILE ")).then(|| &l[5..]).and_then(|r| {
                    let r = r.trim_start();
                    if let Some(r) = r.strip_prefix('"') { r.split('"').next() } else { r.split_whitespace().next() }
                }),
                // Une ligne par disque ; « # » = commentaire.
                "m3u" => (!l.is_empty() && !l.starts_with('#')).then_some(l),
                // 1 0 4 2352 "piste01.bin" 0  ou  1 0 4 2352 piste01.bin 0
                "gdi" => {
                    if let Some(i) = l.find('"') {
                        l[i + 1..].split('"').next()
                    } else {
                        l.split_whitespace().nth(4)
                    }
                }
                _ => None,
            }
        })
        .map(|n| dossier.join(n))
        .collect()
}

/// Les ROM d'un dossier (et de ses sous-dossiers si `recursif`) dont l'extension est dans `extensions` (sans point,
/// en minuscules). Les pistes désignées par un `.cue`, `.m3u` ou `.gdi` du même dossier ne sont pas comptées à part :
/// le jeu, c'est la liste (comme LaunchBox).
pub fn chercher_roms(dossier: &Path, extensions: &[String], recursif: bool) -> Resultat<Vec<RomTrouvee>> {
    if !dossier.is_dir() {
        return Err(Erreur::Disque(format!("Dossier introuvable : {}.", dossier.display())));
    }
    let voulues: BTreeSet<String> = extensions.iter().map(|e| e.trim().trim_start_matches('.').to_lowercase()).filter(|e| !e.is_empty()).collect();
    if voulues.is_empty() {
        return Err(Erreur::Refus("Coche au moins un type de fichier.".into()));
    }
    let fichiers = fichiers_du_dossier(dossier, recursif)?;
    let ext_de = |p: &Path| p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let designees: BTreeSet<PathBuf> = fichiers
        .iter()
        .filter(|p| matches!(ext_de(p).as_str(), "cue" | "m3u" | "gdi") && voulues.contains(&ext_de(p)))
        .flat_map(|p| pistes_designees(p))
        .map(|p| normaliser_chemin(&p))
        .collect();
    let mut l: Vec<RomTrouvee> = fichiers
        .into_iter()
        .filter(|p| voulues.contains(&ext_de(p)) && !designees.contains(&normaliser_chemin(p)))
        .map(|p| RomTrouvee {
            titre: titre_depuis_nom(&p.file_name().unwrap_or_default().to_string_lossy()),
            taille: std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0),
            chemin: p.to_string_lossy().to_string(),
        })
        .collect();
    l.sort_by(|a, b| a.titre.to_lowercase().cmp(&b.titre.to_lowercase()).then(a.chemin.cmp(&b.chemin)));
    Ok(l)
}

/// Les fichiers d'un dossier (et de ses sous-dossiers si `recursif`). Seuls les noms sont lus, jamais le contenu.
fn fichiers_du_dossier(dossier: &Path, recursif: bool) -> Resultat<Vec<PathBuf>> {
    if !dossier.is_dir() {
        return Err(Erreur::Disque(format!("Dossier introuvable : {}.", dossier.display())));
    }
    let mut fichiers = Vec::new();
    let mut a_voir = vec![dossier.to_path_buf()];
    while let Some(d) = a_voir.pop() {
        for e in std::fs::read_dir(&d)?.flatten() {
            let p = e.path();
            let Ok(t) = e.file_type() else { continue };
            if t.is_dir() {
                if recursif {
                    a_voir.push(p);
                }
            } else if t.is_file() {
                fichiers.push(p);
                if fichiers.len() > MAX_FICHIERS_PARCOURUS {
                    return Err(Erreur::Refus(format!(
                        "Plus de {MAX_FICHIERS_PARCOURUS} fichiers dans ce dossier : choisis un dossier plus précis (celui d'un système)."
                    )));
                }
            }
        }
    }
    Ok(fichiers)
}

/// Les types de fichiers d'un dossier : (extension en minuscules, nombre), du plus fréquent au moins fréquent. Sert à
/// proposer quoi importer sans rien demander de taper.
pub fn types_de_fichiers(dossier: &Path, recursif: bool) -> Resultat<Vec<(String, usize)>> {
    let mut n: std::collections::BTreeMap<String, usize> = Default::default();
    for p in fichiers_du_dossier(dossier, recursif)? {
        let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        if !ext.is_empty() {
            *n.entry(ext).or_default() += 1;
        }
    }
    let mut l: Vec<(String, usize)> = n.into_iter().collect();
    l.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    Ok(l)
}

/// Un chemin comparable (Windows ne distingue pas les majuscules ; « / » et « \ » se valent).
fn normaliser_chemin(p: &Path) -> PathBuf {
    PathBuf::from(p.to_string_lossy().replace('/', "\\").to_lowercase())
}

/// Ce qu'il faut copier pour un jeu : le fichier (ou le dossier), les pistes qu'il désigne (`.cue`, `.m3u`, `.gdi`)
/// et, pour un zip MAME, le dossier de ses CHD (même nom, à côté). Chemins complets.
pub fn a_copier(element: &Path) -> Vec<PathBuf> {
    let mut l = vec![element.to_path_buf()];
    if element.is_file() {
        // Un .m3u désigne des .cue/.gdi, qui désignent à leur tour leurs pistes : on suit les deux niveaux.
        for p in pistes_designees(element).into_iter().filter(|p| p.is_file()) {
            let sous = pistes_designees(&p).into_iter().filter(|q| q.is_file() && !l.contains(q)).collect::<Vec<_>>();
            if !l.contains(&p) {
                l.push(p);
            }
            l.extend(sous);
        }
        if let (Some(dossier), Some(nom)) = (element.parent(), element.file_stem()) {
            let chd = dossier.join(nom);
            let zip = element.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip") || e.eq_ignore_ascii_case("7z"));
            if zip && chd.is_dir() {
                l.push(chd);
            }
        }
    }
    l
}

/// La taille totale de ce qu'il faut copier pour ces jeux.
pub fn taille_a_copier(elements: &[PathBuf]) -> u64 {
    elements
        .iter()
        .flat_map(|e| a_copier(e))
        .map(|p| {
            if p.is_dir() {
                crate::installation::fichiers_de(&p).unwrap_or_default().iter().map(|r| std::fs::metadata(p.join(r)).map(|m| m.len()).unwrap_or(0)).sum()
            } else {
                std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0)
            }
        })
        .sum()
}

/// Deux fichiers de même taille sont-ils les mêmes ? Comparés au début, au milieu et à la fin (64 Ko chacun) : assez pour
/// distinguer deux ROM différentes de même taille, sans relire une image disque de 50 Go.
fn memes_octets(a: &Path, b: &Path) -> bool {
    use std::io::{Read, Seek, SeekFrom};
    let (Ok(mut fa), Ok(mut fb)) = (std::fs::File::open(a), std::fs::File::open(b)) else { return false };
    let taille = fa.metadata().map(|m| m.len()).unwrap_or(0);
    const MORCEAU: u64 = 64 * 1024;
    for pos in [0, (taille / 2).saturating_sub(MORCEAU / 2), taille.saturating_sub(MORCEAU)] {
        let (mut x, mut y) = (Vec::new(), Vec::new());
        let ok = fa.seek(SeekFrom::Start(pos)).is_ok()
            && fb.seek(SeekFrom::Start(pos)).is_ok()
            && (&mut fa).take(MORCEAU).read_to_end(&mut x).is_ok()
            && (&mut fb).take(MORCEAU).read_to_end(&mut y).is_ok();
        if !ok || x != y {
            return false;
        }
    }
    true
}

/// Copie un fichier, vérifie sa taille ; un fichier déjà présent IDENTIQUE est gardé, un fichier différent n'est
/// JAMAIS écrasé. La copie passe par un fichier provisoire (« .frogtend-copie ») renommé à la fin : une copie
/// interrompue ne laisse jamais un faux « déjà là ».
fn copier_un(source: &Path, cible: &Path) -> Resultat<()> {
    let taille = std::fs::metadata(source)?.len();
    if let Ok(m) = std::fs::metadata(cible) {
        if m.len() == taille && memes_octets(source, cible) {
            return Ok(());
        }
        return Err(Erreur::Refus(format!("{} existe déjà (différent) : Frogtend n'écrit pas par-dessus.", cible.display())));
    }
    if let Some(p) = cible.parent() {
        std::fs::create_dir_all(p)?;
    }
    let mut nom = cible.file_name().unwrap_or_default().to_os_string();
    nom.push(".frogtend-copie");
    let provisoire = cible.with_file_name(nom);
    std::fs::copy(source, &provisoire)?;
    if std::fs::metadata(&provisoire).map(|m| m.len()).ok() != Some(taille) {
        let _ = std::fs::remove_file(&provisoire); // le nôtre, incomplet
        return Err(Erreur::Disque(format!("La copie de {} a échoué.", source.display())));
    }
    std::fs::rename(&provisoire, cible)?;
    Ok(())
}

/// Copie des jeux (fichiers ou dossiers, avec leurs pistes et CHD) dans `destination` — jamais déplacés : les
/// originaux restent. Rend, pour chaque élément, son nouveau chemin.
pub fn copier_jeux(elements: &[PathBuf], destination: &Path) -> Resultat<Vec<PathBuf>> {
    let mut nouveaux = Vec::new();
    for e in elements {
        for (i, p) in a_copier(e).into_iter().enumerate() {
            let nom = p.file_name().ok_or_else(|| Erreur::Refus("Chemin invalide.".into()))?;
            let cible = destination.join(nom);
            if p.is_dir() {
                for r in crate::installation::fichiers_de(&p)? {
                    copier_un(&p.join(&r), &cible.join(&r))?;
                }
            } else {
                copier_un(&p, &cible)?;
            }
            if i == 0 {
                nouveaux.push(cible);
            }
        }
    }
    Ok(nouveaux)
}

/// Un jeu MS-DOS trouvé : un sous-dossier, et le programme qui le lance le plus probablement.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct JeuDosTrouve {
    pub dossier: String,
    pub titre: String,
    /// Les programmes candidats (chemins RELATIFS au dossier du jeu), le plus probable d'abord.
    pub programmes: Vec<String>,
}

/// Les programmes DOS (`.exe`, `.com`, `.bat`) d'un dossier, chemins RELATIFS, le plus probable d'abord (d'après le
/// titre ; installeurs, réglages du son et extenseurs DOS en dernier).
pub fn programmes_dos(dossier: &Path, titre: &str) -> Resultat<Vec<String>> {
    const A_ECARTER: &[&str] = &["install", "setup", "setsound", "sound", "config", "unins", "dos4gw", "cwsdpmi", "readme", "patch"];
    let mots: Vec<String> = titre.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|m| m.len() >= 3).map(String::from).collect();
    let mut programmes: Vec<(i32, String)> = crate::installation::fichiers_de(dossier)?
        .into_iter()
        .filter_map(|r| {
            let bas = r.to_lowercase();
            let nom = bas.rsplit('/').next().unwrap_or(&bas).to_string();
            let ext = nom.rsplit('.').next().unwrap_or("");
            if !matches!(ext, "exe" | "com" | "bat") {
                return None;
            }
            let racine_nom = nom.trim_end_matches(&format!(".{ext}")).to_string();
            let mut note = 10 - 4 * r.matches('/').count() as i32;
            if A_ECARTER.iter().any(|m| racine_nom.contains(m)) {
                note -= 20;
            }
            note += 8 * mots.iter().filter(|m| racine_nom.contains(&m[..m.len().min(8)])).count() as i32;
            if ["play", "start", "go", "run", "jeu", "game"].contains(&racine_nom.as_str()) {
                note += 6;
            }
            Some((note, r))
        })
        .collect();
    programmes.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    Ok(programmes.into_iter().map(|(_, r)| r).collect())
}

/// Les jeux MS-DOS d'un dossier : chaque sous-dossier qui contient un `.exe`, `.com` ou `.bat`.
pub fn chercher_jeux_dos(dossier: &Path) -> Resultat<Vec<JeuDosTrouve>> {
    if !dossier.is_dir() {
        return Err(Erreur::Disque(format!("Dossier introuvable : {}.", dossier.display())));
    }
    let mut l = Vec::new();
    let mut sous: Vec<PathBuf> = std::fs::read_dir(dossier)?.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    sous.sort();
    for d in sous {
        let titre = titre_depuis_nom(&d.file_name().unwrap_or_default().to_string_lossy());
        let programmes = programmes_dos(&d, &titre)?;
        if programmes.is_empty() {
            continue;
        }
        l.push(JeuDosTrouve { dossier: d.to_string_lossy().to_string(), titre, programmes });
    }
    Ok(l)
}

/// Un chemin Windows pour une commande DOSBox entre guillemets (sans « \ » final, qui échapperait le guillemet).
fn pour_dosbox(p: &Path) -> String {
    let s = p.to_string_lossy().replace('/', "\\");
    let s = s.trim_end_matches('\\');
    if s.ends_with(':') { format!("{s}\\") } else { s.to_string() }
}

/// 📥 Installer un jeu DOS (comme LaunchBox) : les arguments de DOSBox qui montent la DESTINATION en C: et la SOURCE
/// en D: (un dossier ou une image ISO/CUE comme lecteur de CD, une image de disquette en A:), puis se placent sur le
/// lecteur source. La personne lance l'installeur et installe dans C:.
pub fn arguments_installation_dos(source: &Path, destination: &Path) -> Resultat<Vec<String>> {
    let c = format!("mount c \"{}\"", pour_dosbox(destination));
    let ext = source.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let (montage, lecteur) = if source.is_dir() {
        (format!("mount d \"{}\" -t cdrom", pour_dosbox(source)), "d:")
    } else if source.is_file() && matches!(ext.as_str(), "iso" | "cue") {
        (format!("imgmount d \"{}\" -t iso", pour_dosbox(source)), "d:")
    } else if source.is_file() && matches!(ext.as_str(), "img" | "ima" | "vfd") {
        (format!("imgmount a \"{}\" -t floppy", pour_dosbox(source)), "a:")
    } else {
        return Err(Erreur::Refus("La source doit être un dossier, une image de CD (.iso, .cue) ou de disquette (.img, .ima).".into()));
    };
    Ok(["-c", &c, "-c", &montage, "-c", lecteur].iter().map(|s| s.to_string()).collect())
}

/// Les arguments de DOSBox pour JOUER à un jeu installé : C: monté au même endroit que pendant l'installation (le jeu
/// retrouve ses chemins), puis le programme lancé depuis son dossier, et DOSBox se ferme à la sortie du jeu.
pub fn arguments_jeu_dos(destination: &Path, relatif: &str) -> Resultat<Vec<String>> {
    if relatif.contains("..") || relatif.trim().is_empty() {
        return Err(Erreur::Refus("Programme du jeu invalide.".into()));
    }
    let relatif = relatif.replace('/', "\\");
    let (dossier, programme) = match relatif.rsplit_once('\\') {
        Some((d, p)) => (format!("\\{d}"), p.to_string()),
        None => ("\\".to_string(), relatif.clone()),
    };
    let c = format!("mount c \"{}\"", pour_dosbox(destination));
    Ok(["-c", &c, "-c", "c:", "-c", &format!("cd {dossier}"), "-c", &programme, "-c", "exit"].iter().map(|s| s.to_string()).collect())
}

/// Un jeu à ajouter à la ludothèque, tel que la personne l'a validé.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JeuAImporter {
    pub titre: String,
    /// Nom LaunchBox de la plateforme (« Windows », « MS-DOS », « Super Nintendo Entertainment System »…).
    pub plateforme: String,
    /// Le dossier du jeu.
    pub dossier: String,
    /// Pour un émulateur : le fichier donné à l'émulateur, RELATIF à `dossier` (ROM, image, programme DOS).
    #[serde(default)]
    pub fichier: Option<String>,
    /// Pour un jeu Windows : le programme à lancer (chemin complet).
    #[serde(default)]
    pub programme: Option<String>,
    /// Les arguments du programme (un jeu DOS installé : DOSBox et ses commandes).
    #[serde(default)]
    pub arguments: Vec<String>,
    /// Ce qu'on sait déjà du jeu (la liste MAME le donne).
    #[serde(default)]
    pub annee: Option<i64>,
    #[serde(default)]
    pub editeur: Option<String>,
    #[serde(default)]
    pub genres: Vec<String>,
}

/// La clé stable d'un jeu importé (le même fichier importé deux fois = le même jeu).
pub fn cle_import(j: &JeuAImporter) -> String {
    let cible = match (&j.programme, &j.fichier) {
        (Some(p), _) => PathBuf::from(p),
        (None, Some(f)) => Path::new(&j.dossier).join(f),
        (None, None) => PathBuf::from(&j.dossier),
    };
    normaliser_chemin(&cible).to_string_lossy().to_string()
}

/// Vérifie un jeu à importer : un titre, une plateforme, et ce qui le lance existe vraiment sur le disque.
pub fn verifier(j: &JeuAImporter) -> Resultat<()> {
    if j.titre.trim().is_empty() || j.plateforme.trim().is_empty() {
        return Err(Erreur::Refus("Il faut un titre et une plateforme.".into()));
    }
    if !Path::new(&j.dossier).is_dir() {
        return Err(Erreur::Disque(format!("Dossier introuvable : {}.", j.dossier)));
    }
    match (&j.programme, &j.fichier) {
        (Some(p), _) if !Path::new(p).is_file() => Err(Erreur::Disque(format!("Programme introuvable : {p}."))),
        (None, Some(f)) if f.contains("..") || !Path::new(&j.dossier).join(f).is_file() => {
            Err(Erreur::Disque(format!("Fichier introuvable : {f}.")))
        }
        (None, None) => Err(Erreur::Refus("Choisis le programme ou le fichier du jeu.".into())),
        _ => Ok(()),
    }
}

/// Le bilan d'un import.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct BilanImport {
    pub ajoutes: usize,
    /// Versions ajoutées à des jeux déjà dans la ludothèque (une autre région du même jeu).
    #[serde(default)]
    pub versions: usize,
    /// Déjà dans la ludothèque (même fichier) : laissés tels quels.
    pub deja: usize,
    /// (titre, motif) de ceux qui n'ont pas pu être ajoutés.
    pub refuses: Vec<(String, String)>,
}

impl crate::noyau::Noyau {
    /// Ajoute des jeux du disque à la ludothèque du profil ouvert ET au registre du PC (pour « Jouer ») : rien n'est
    /// copié ni renommé ; le jeu est noté `VERSION_IMPORTEE` (Frogtend n'effacera jamais ses fichiers).
    pub async fn importer_locaux(&self, jeux: &[JeuAImporter]) -> Resultat<BilanImport> {
        use crate::installation::{Lanceur, Methode};
        use crate::jeux_pc::{Etat, Installation, JeuPc};
        let s = self.session().await?;
        let mut bilan = BilanImport::default();
        let mut lignes = Vec::new();

        // Les fichiers déjà connus (jeux importés avant, y compris un par fichier avant 0.31.0) : jamais en double.
        let mut connus: std::collections::HashSet<String> = std::collections::HashSet::new();
        for j in self.registre().tous()?.into_iter().filter(|j| j.version == VERSION_IMPORTEE) {
            if let Some(i) = &j.installation {
                if let Some(f) = &i.fichier_du_jeu {
                    connus.insert(normaliser_chemin(&Path::new(&i.dossier).join(f)).to_string_lossy().to_string());
                }
                for v in &i.versions {
                    for c in std::iter::once(&v.chemin).chain(v.disques.iter()) {
                        connus.insert(normaliser_chemin(Path::new(c)).to_string_lossy().to_string());
                    }
                }
            }
            connus.insert(cle_import_de_jeu(&j));
        }

        // Les jeux qu'on lance par un fichier (ROM, image, programme DOS) se regroupent par jeu ; les autres (un
        // programme Windows, DOSBox avec ses commandes) restent un par un.
        let mut groupes: std::collections::BTreeMap<String, Vec<&JeuAImporter>> = Default::default();
        let mut seuls = Vec::new();
        for j in jeux {
            if let Err(e) = verifier(j) {
                bilan.refuses.push((j.titre.clone(), format!("{e:?}")));
                continue;
            }
            if connus.contains(&cle_import(j)) {
                bilan.deja += 1;
                continue;
            }
            if j.programme.is_none() && j.fichier.is_some() {
                groupes.entry(cle_de_jeu(&j.plateforme, &j.titre)).or_default().push(j);
            } else {
                seuls.push(j);
            }
        }
        let quand = crate::noyau::maintenant();

        for (cle, membres) in groupes {
            let chemins: Vec<String> = membres.iter().map(|j| Path::new(&j.dossier).join(j.fichier.as_deref().unwrap_or("")).to_string_lossy().to_string()).collect();
            let nouvelles = en_versions(&chemins);
            let id = crate::ludotheque::id_boutique("local", &cle);
            // Lu à part : un `if let` garderait le verrou du registre pendant tout le bloc (qui le redemande).
            let existant = self.registre().jeu(id)?;
            if let Some(existant) = existant {
                // Une autre région d'un jeu déjà là : ses versions s'ajoutent, la version par défaut ne change pas.
                let mut i = existant.installation.clone().ok_or_else(|| Erreur::Disque("Jeu importé sans installation.".into()))?;
                // Un jeu importé seul n'a pas de liste de versions : sa version d'origine y entre d'abord, sinon elle
                // disparaîtrait de la liste.
                if i.versions.is_empty() {
                    if let Some(f) = &i.fichier_du_jeu {
                        i.versions = en_versions(&[Path::new(&i.dossier).join(f).to_string_lossy().to_string()]);
                    }
                }
                let avant = i.versions.len();
                for v in nouvelles {
                    if !i.versions.iter().any(|x| x.chemin.eq_ignore_ascii_case(&v.chemin)) {
                        i.versions.push(v);
                    }
                }
                i.versions.sort_by(|a, b| (a.rang, a.qualite, &a.libelle, &a.chemin).cmp(&(b.rang, b.qualite, &b.libelle, &b.chemin)));
                bilan.versions += i.versions.len() - avant;
                self.registre().changer_installation(id, Some(&i))?;
                continue;
            }
            let meilleure = &nouvelles[0];
            let modele = membres
                .iter()
                .find(|j| Path::new(&j.dossier).join(j.fichier.as_deref().unwrap_or("")).to_string_lossy() == meilleure.chemin)
                .copied()
                .unwrap_or(membres[0]);
            let dossier = Path::new(&meilleure.chemin).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
            let fichier = Path::new(&meilleure.chemin).file_name().map(|n| n.to_string_lossy().to_string());
            // Un programme DOS dans un sous-dossier garde son chemin relatif au dossier du jeu.
            let (dossier, fichier) = if modele.fichier.as_deref().is_some_and(|f| f.contains(['/', '\\'])) {
                (modele.dossier.clone(), modele.fichier.clone())
            } else {
                (dossier, fichier)
            };
            self.registre().ajouter(&JeuPc {
                id,
                version: VERSION_IMPORTEE,
                titre: modele.titre.trim().to_string(),
                plateforme: modele.plateforme.clone(),
                dossier: dossier.clone(),
                etat: Etat::Telecharge,
                total: 0,
                fichiers: vec![],
                message: None,
                ajoute_le: quand.clone(),
                ajoute_par: s.profil.id.clone(),
                installation: None,
                temps_jeu: 0,
                derniere_partie: None,
            })?;
            let versions = if nouvelles.len() > 1 || !nouvelles[0].disques.is_empty() { nouvelles.clone() } else { vec![] };
            self.registre().changer_installation(
                id,
                Some(&Installation { dossier, methode: Methode::Aucune, lanceur: None, fichier_du_jeu: fichier, installe_le: quand.clone(), versions }),
            )?;
            lignes.push(ligne_de_ludotheque(id, modele, cle, membres.iter().find_map(|j| j.annee), membres.iter().find_map(|j| j.editeur.clone())));
            bilan.ajoutes += 1;
        }

        for j in seuls {
            let cle = cle_import(j);
            let id = crate::ludotheque::id_boutique("local", &cle);
            if self.registre().jeu(id)?.is_some() {
                bilan.deja += 1;
                continue;
            }
            let lanceur = j.programme.as_ref().map(|p| Lanceur {
                programme: p.clone(),
                arguments: j.arguments.clone(),
                dossier: if j.arguments.is_empty() {
                    Path::new(p).parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_else(|| j.dossier.clone())
                } else {
                    j.dossier.clone()
                },
            });
            self.registre().ajouter(&JeuPc {
                id,
                version: VERSION_IMPORTEE,
                titre: j.titre.trim().to_string(),
                plateforme: j.plateforme.clone(),
                dossier: j.dossier.clone(),
                etat: Etat::Telecharge,
                total: 0,
                fichiers: vec![],
                message: None,
                ajoute_le: quand.clone(),
                ajoute_par: s.profil.id.clone(),
                installation: None,
                temps_jeu: 0,
                derniere_partie: None,
            })?;
            let installation = Installation {
                dossier: j.dossier.clone(),
                methode: Methode::Aucune,
                lanceur: lanceur.clone(),
                fichier_du_jeu: if lanceur.is_some() { None } else { j.fichier.clone() },
                installe_le: quand.clone(),
                versions: vec![],
            };
            self.registre().changer_installation(id, Some(&installation))?;
            lignes.push(ligne_de_ludotheque(id, j, cle, j.annee, j.editeur.clone()));
            bilan.ajoutes += 1;
        }
        s.verrou().ajouter_locaux(&lignes)?;
        self.journaliser(&format!(
            "import local : {} jeu(x) ajouté(s), {} version(s) ajoutée(s), {} déjà là, {} refusé(s)",
            bilan.ajoutes,
            bilan.versions,
            bilan.deja,
            bilan.refuses.len()
        ));
        Ok(bilan)
    }
}

/// La clé d'import d'un jeu déjà dans le registre (programme, ou fichier) : pour ne pas l'ajouter deux fois.
fn cle_import_de_jeu(j: &crate::jeux_pc::JeuPc) -> String {
    let i = j.installation.as_ref();
    let cible = match (i.and_then(|i| i.lanceur.as_ref()), i.and_then(|i| i.fichier_du_jeu.as_ref())) {
        (Some(l), _) if l.arguments.is_empty() => PathBuf::from(&l.programme),
        (_, Some(f)) => Path::new(&j.dossier).join(f),
        _ => PathBuf::from(&j.dossier),
    };
    normaliser_chemin(&cible).to_string_lossy().to_string()
}

/// La ligne d'un jeu importé dans la ludothèque du profil.
fn ligne_de_ludotheque(id: i64, j: &JeuAImporter, cle: String, annee: Option<i64>, editeur: Option<String>) -> crate::ludotheque::JeuResume {
    crate::ludotheque::JeuResume {
        id,
        titre: j.titre.trim().to_string(),
        plateforme: j.plateforme.clone(),
        annee,
        editeur,
        genres: j.genres.clone(),
        statut: Some("importe".into()),
        jaquette: Some(false),
        source: Some("local".into()),
        boutique: Some("local".into()),
        cle_boutique: Some(cle),
        installe: Some(true),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn un_jeu_importe_se_joue_et_ne_s_efface_jamais_du_disque() {
        use crate::coffre::CoffreMemoire;
        use crate::noyau::{Connexion, Noyau};
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(CoffreMemoire::default())).unwrap();
        let profil = n.creer_profil("Seb", None, None).unwrap().id;
        n.ouvrir(&profil, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        let jeux = d.path().join("Mes jeux");
        ecrire(&jeux, "Mario (USA).sfc", "rom");
        ecrire(&jeux, "Doom/DOOM.EXE", "exe");
        // Un désinstalleur présent : il ne doit JAMAIS être lancé pour un jeu importé.
        ecrire(&jeux, "Doom/unins000.exe", "x");
        let racine = jeux.to_string_lossy().to_string();
        let a = vec![
            JeuAImporter { titre: "Mario".into(), plateforme: "Super Nintendo Entertainment System".into(), dossier: racine.clone(), fichier: Some("Mario (USA).sfc".into()), programme: None, arguments: vec![], annee: None, editeur: None, genres: vec![] },
            JeuAImporter { titre: "Doom".into(), plateforme: "Windows".into(), dossier: jeux.join("Doom").to_string_lossy().into(), fichier: None, programme: Some(jeux.join("Doom/DOOM.EXE").to_string_lossy().into()), arguments: vec![], annee: None, editeur: None, genres: vec![] },
            JeuAImporter { titre: "Absent".into(), plateforme: "Windows".into(), dossier: racine.clone(), fichier: None, programme: Some(jeux.join("rien.exe").to_string_lossy().into()), arguments: vec![], annee: None, editeur: None, genres: vec![] },
        ];
        let b = n.importer_locaux(&a).await.unwrap();
        assert_eq!((b.ajoutes, b.deja, b.refuses.len()), (2, 0, 1));
        assert_eq!(n.importer_locaux(&a[..1]).await.unwrap().deja, 1, "le même fichier n'est pas ajouté deux fois");

        // Dans la ludothèque du profil, rangé « local » et installé.
        let l = n.lister(&crate::ludotheque::Filtre { boutique: Some("local".into()), ludotheque: true, ..Default::default() }).await.unwrap();
        assert_eq!(l.total, 2);
        let mario = l.jeux.iter().find(|j| j.titre == "Mario").unwrap().clone();
        let i = n.registre().jeu(mario.id).unwrap().unwrap().installation.unwrap();
        assert_eq!(i.fichier_du_jeu.as_deref(), Some("Mario (USA).sfc"), "la ROM garde son nom");

        // Retirer : la ludothèque oublie le jeu, le disque garde TOUT.
        let doom = l.jeux.iter().find(|j| j.titre == "Doom").unwrap().id;
        n.retirer_du_pc(doom).await.unwrap();
        n.retirer_du_pc(mario.id).await.unwrap();
        assert!(jeux.join("Doom/DOOM.EXE").is_file() && jeux.join("Doom/unins000.exe").is_file());
        assert!(jeux.join("Mario (USA).sfc").is_file());
        assert!(n.registre().jeu(doom).unwrap().is_none());
        let l = n.lister(&crate::ludotheque::Filtre { boutique: Some("local".into()), ludotheque: true, ..Default::default() }).await.unwrap();
        assert_eq!(l.total, 0);
    }

    #[test]
    fn les_etiquettes_donnent_la_region_et_la_qualite() {
        assert_eq!(etiquettes("Mario (France).nes").0, 0);
        assert_eq!(etiquettes("Mario (En,Fr,De).nes").0, 0, "le français suffit");
        assert_eq!(etiquettes("Mario (Europe) (Rev 1).nes").0, 1);
        assert_eq!(etiquettes("Mario (E) [!].nes").0, 1);
        assert_eq!(etiquettes("Mario (USA).nes").0, 2);
        assert_eq!(etiquettes("Mario (JU).nes").0, 2, "GoodTools : Japon + USA");
        assert_eq!(etiquettes("Mario (UE).nes").0, 1);
        assert_eq!(etiquettes("Mario (World).nes").0, 2);
        assert_eq!(etiquettes("Mario (Japan).nes").0, 3);
        assert_eq!(etiquettes("Mario.nes").0, 3);
        assert!(etiquettes("Mario (U) [!].nes").1 < etiquettes("Mario (U).nes").1);
        assert!(etiquettes("Mario (U) [b1].nes").1 > etiquettes("Mario (U) [a1].nes").1);
        assert!(etiquettes("Mario (USA) (Beta).nes").1 > 0);
        let (_, _, libelle, disque) = etiquettes("FF7 (France) (Disc 2).cue");
        assert_eq!((libelle.as_str(), disque), ("France", Some(2)));
        assert_eq!(etiquettes("Jeu (Disk 1 of 3).adf").3, Some(1));
        assert_eq!(etiquettes("Mario (Europe) (Rev 1).nes").2, "Europe, Rev 1");
    }

    #[test]
    fn les_versions_suivent_l_ordre_fr_eu_us_puis_les_autres() {
        let c = |n: &str| format!("E:\\ROM\\{n}");
        let l = en_versions(&[
            c("Mario (Japan).nes"),
            c("Mario (USA).nes"),
            c("Mario (Europe).nes"),
            c("Mario (France).nes"),
            c("Mario (USA) [!].nes"),
        ]);
        let ordre: Vec<&str> = l.iter().map(|v| v.chemin.rsplit('\\').next().unwrap()).collect();
        assert_eq!(ordre, ["Mario (France).nes", "Mario (Europe).nes", "Mario (USA) [!].nes", "Mario (USA).nes", "Mario (Japan).nes"]);

        // Traductions de fan (Seb, 03/10) : « [T-Fr by …] » compte comme français (juste après le français officiel,
        // avant l'Europe) ; « [T-En by …] » comme anglais ; plus jamais rangées avec les hacks.
        let l = en_versions(&[
            c("Chrono Trigger (Japan).sfc"),
            c("Chrono Trigger (USA).sfc"),
            c("Chrono Trigger (Europe).sfc"),
            c("Chrono Trigger (USA) [T-Fr by Génération IX].sfc"),
            c("Chrono Trigger (France).sfc"),
            c("Chrono Trigger (Japan) [T-En by Aeon Genesis].sfc"),
            c("Chrono Trigger (USA) [h1C].sfc"),
        ]);
        let ordre: Vec<&str> = l.iter().map(|v| v.chemin.rsplit('\\').next().unwrap()).collect();
        assert_eq!(
            ordre,
            [
                "Chrono Trigger (France).sfc",
                "Chrono Trigger (USA) [T-Fr by Génération IX].sfc",
                "Chrono Trigger (Europe).sfc",
                "Chrono Trigger (USA).sfc",
                "Chrono Trigger (Japan) [T-En by Aeon Genesis].sfc",
                "Chrono Trigger (USA) [h1C].sfc",
                "Chrono Trigger (Japan).sfc",
            ]
        );
        let trad = l.iter().find(|v| v.chemin.contains("T-Fr")).unwrap();
        assert_eq!(trad.rang, 0);
        assert!(trad.libelle.contains("Traduction française par Génération IX"), "{}", trad.libelle);
        assert_eq!(traduction("T+Fre"), Some(("fr", None)));
        assert_eq!(traduction("T-Fr v1.1 by Terminus"), Some(("fr", Some("Terminus".into()))));
        assert_eq!(traduction("T-Eng"), Some(("en", None)));
        assert_eq!(traduction("T-Ger by X"), None, "une autre langue reste une étiquette ordinaire");
        // Une lettre qui change de longueur en minuscule (« İ ») avant « by » ne doit rien casser.
        assert_eq!(traduction("T-Fr İİ by Groupe"), Some(("fr", Some("Groupe".into()))));
        // La langue de la traduction l'emporte sur la région d'origine.
        assert_eq!(etiquettes("Jeu (France) [T-En by X].sfc").0, 2);
        assert_eq!(traduction("!"), None);
        // Même fiche que le jeu d'origine : l'étiquette ne fait pas partie du titre.
        assert_eq!(titre_depuis_nom("Chrono Trigger (USA) [T-Fr by Génération IX].sfc"), "Chrono Trigger");

        // Les disques d'une même version restent ensemble ; le 1er est lancé.
        let l = en_versions(&[c("FF7 (France) (Disc 2).cue"), c("FF7 (France) (Disc 1).cue"), c("FF7 (USA) (Disc 1).cue")]);
        assert_eq!(l.len(), 2);
        assert!(l[0].chemin.ends_with("FF7 (France) (Disc 1).cue"));
        assert_eq!(l[0].disques.len(), 2);
        assert!(l[1].disques.is_empty(), "un seul disque : pas de liste");
    }

    #[tokio::test]
    async fn une_fiche_par_jeu_et_ses_versions() {
        use crate::coffre::CoffreMemoire;
        use crate::noyau::{Connexion, Noyau};
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(CoffreMemoire::default())).unwrap();
        let profil = n.creer_profil("Seb", None, None).unwrap().id;
        n.ouvrir(&profil, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        let roms = d.path().join("NES");
        for f in ["USA/Mario (U).nes", "Europe/Mario (E).nes", "France/Mario (F).nes", "Zelda (U).nes"] {
            ecrire(&roms, f, "rom");
        }
        let a_importer = |chemins: &[&str]| -> Vec<JeuAImporter> {
            chemins
                .iter()
                .map(|c| {
                    let p = roms.join(c);
                    JeuAImporter {
                        titre: titre_depuis_nom(&p.file_name().unwrap().to_string_lossy()),
                        plateforme: "Nintendo Entertainment System".into(),
                        dossier: p.parent().unwrap().to_string_lossy().into(),
                        fichier: Some(p.file_name().unwrap().to_string_lossy().into()),
                        programme: None,
                        arguments: vec![],
                        annee: None,
                        editeur: None,
                        genres: vec![],
                    }
                })
                .collect()
        };
        let b = n.importer_locaux(&a_importer(&["USA/Mario (U).nes", "Europe/Mario (E).nes", "Zelda (U).nes"])).await.unwrap();
        assert_eq!((b.ajoutes, b.versions), (2, 0), "Mario (2 versions) et Zelda : 2 fiches");
        let filtre = crate::ludotheque::Filtre { boutique: Some("local".into()), ludotheque: true, ..Default::default() };
        let l = n.lister(&filtre).await.unwrap();
        assert_eq!(l.total, 2);
        let mario = l.jeux.iter().find(|j| j.titre == "Mario").unwrap().id;
        let i = n.registre().jeu(mario).unwrap().unwrap().installation.unwrap();
        assert_eq!(i.versions.len(), 2);
        assert_eq!(i.fichier_du_jeu.as_deref(), Some("Mario (E).nes"), "l'Europe passe avant les USA");

        // La version française arrive ensuite : elle rejoint la fiche (en tête), la version lancée ne change pas.
        let b = n.importer_locaux(&a_importer(&["France/Mario (F).nes", "USA/Mario (U).nes"])).await.unwrap();
        assert_eq!((b.ajoutes, b.versions, b.deja), (0, 1, 1));
        let i = n.registre().jeu(mario).unwrap().unwrap().installation.unwrap();
        assert_eq!(i.versions.len(), 3);
        assert!(i.versions[0].chemin.ends_with("Mario (F).nes"));
        assert_eq!(i.fichier_du_jeu.as_deref(), Some("Mario (E).nes"));
        assert_eq!(n.lister(&filtre).await.unwrap().total, 2, "toujours une seule fiche Mario");

        // La version par défaut : la française, choisie par la personne. Une version qui n'est pas du jeu : refusée.
        let fr = i.versions[0].chemin.clone();
        n.choisir_version(mario, &fr).await.unwrap();
        assert_eq!(n.registre().jeu(mario).unwrap().unwrap().installation.unwrap().fichier_du_jeu.as_deref(), Some("Mario (F).nes"));
        assert!(n.choisir_version(mario, "C:\\ailleurs.nes").await.is_err());
        assert!(n.jouer(mario, None, Some("C:\\ailleurs.nes")).await.is_err());

        // Zelda a été importé seul ; sa version européenne arrive : l'américaine reste dans la liste.
        ecrire(&roms, "Zelda (E).nes", "rom");
        let b = n.importer_locaux(&a_importer(&["Zelda (E).nes"])).await.unwrap();
        assert_eq!(b.versions, 1);
        let zelda = n.lister(&filtre).await.unwrap().jeux.iter().find(|j| j.titre == "Zelda").unwrap().id;
        let i = n.registre().jeu(zelda).unwrap().unwrap().installation.unwrap();
        assert_eq!(i.versions.len(), 2, "la version d'origine n'est pas perdue");
        assert!(i.versions.iter().any(|v| v.chemin.ends_with("Zelda (U).nes")));
    }

    #[tokio::test]
    async fn un_jeu_zippe_se_decompresse_pour_jouer() {
        use crate::coffre::CoffreMemoire;
        use crate::noyau::{Connexion, Noyau};
        use std::io::Write;
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(CoffreMemoire::default())).unwrap();
        let profil = n.creer_profil("Seb", None, None).unwrap().id;
        n.ouvrir(&profil, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        let ps3 = d.path().join("PS3");
        std::fs::create_dir_all(&ps3).unwrap();
        let archive = ps3.join("Jeu (Europe).zip");
        let mut z = zip::ZipWriter::new(std::fs::File::create(&archive).unwrap());
        z.start_file("Jeu (Europe).iso", zip::write::SimpleFileOptions::default()).unwrap();
        z.write_all(&[9u8; 2048]).unwrap();
        z.finish().unwrap();
        let jeu = JeuAImporter {
            titre: "Jeu".into(),
            plateforme: "Sony Playstation 3".into(),
            dossier: ps3.to_string_lossy().into(),
            fichier: Some("Jeu (Europe).zip".into()),
            programme: None,
            arguments: vec![],
            annee: None,
            editeur: None,
            genres: vec![],
        };
        n.importer_locaux(&[jeu]).await.unwrap();
        let filtre = crate::ludotheque::Filtre { boutique: Some("local".into()), ludotheque: true, ..Default::default() };
        let id = n.lister(&filtre).await.unwrap().jeux[0].id;
        assert_eq!(n.fichier_lance(id).await.unwrap(), Some(archive.clone()));
        let iso = crate::decompression::decompresser(&archive, None, &mut |_| {}).unwrap();
        n.remplacer_fichier(id, &archive, &iso).await.unwrap();
        let i = n.registre().jeu(id).unwrap().unwrap().installation.unwrap();
        assert_eq!(i.fichier_du_jeu.as_deref(), Some("Jeu (Europe).iso"), "le nom de l'image est gardé");
        assert_eq!(std::path::PathBuf::from(&i.dossier), ps3.join("Jeu (Europe)"));
        assert!(i.versions.iter().all(|v| !v.chemin.ends_with(".zip")), "plus de version qui pointe l'archive");
        assert!(archive.is_file(), "l'archive est gardée");
        assert!(n.remplacer_fichier(id, &archive, &iso).await.is_err(), "l'archive n'est plus celle du jeu");
    }

    #[test]
    fn une_feuille_ansi_avec_marque_se_lit() {
        let d = tempfile::tempdir().unwrap();
        // ANSI (é = 0xE9), marque UTF-8 en tête, « File » en casse mêlée.
        let mut o = vec![0xEF, 0xBB, 0xBF];
        o.extend(b"File \"Jeu \xE9t\xE9 (Track 1).bin\" BINARY\r\n  TRACK 01 MODE1/2352\r\n");
        std::fs::write(d.path().join("Jeu.cue"), &o).unwrap();
        assert_eq!(pistes_designees(&d.path().join("Jeu.cue")), vec![d.path().join("Jeu été (Track 1).bin")]);
    }

    #[test]
    fn un_gros_fichier_n_est_jamais_lu_pour_chercher_des_pistes() {
        // Le bug du 03/10 : mesurer 7 zips PS3 (218 Go) les lisait en entier. Un fichier de 4 Go (creux) doit être
        // écarté sans être lu, qu'il ait l'extension d'une archive ou d'une liste de pistes.
        let d = tempfile::tempdir().unwrap();
        for nom in ["Jeu.zip", "Enorme.cue"] {
            let f = d.path().join(nom);
            std::fs::File::create(&f).unwrap().set_len(4 << 30).unwrap();
            let t = std::time::Instant::now();
            assert!(pistes_designees(&f).is_empty());
            assert_eq!(a_copier(&f), vec![f.clone()]);
            assert!(t.elapsed().as_millis() < 1000, "{nom} lu en entier ?");
        }
    }

    #[test]
    fn la_copie_d_un_jeu_m3u_emporte_les_pistes_de_ses_disques() {
        let d = tempfile::tempdir().unwrap();
        ecrire(d.path(), "FF7.m3u", "FF7 (Disc 1).cue\nFF7 (Disc 2).cue\n");
        ecrire(d.path(), "FF7 (Disc 1).cue", "FILE \"FF7 (Disc 1).bin\" BINARY\n");
        ecrire(d.path(), "FF7 (Disc 2).cue", "FILE \"FF7 (Disc 2).bin\" BINARY\n");
        ecrire(d.path(), "FF7 (Disc 1).bin", "1");
        ecrire(d.path(), "FF7 (Disc 2).bin", "2");
        let l = a_copier(&d.path().join("FF7.m3u"));
        for n in ["FF7.m3u", "FF7 (Disc 1).cue", "FF7 (Disc 1).bin", "FF7 (Disc 2).cue", "FF7 (Disc 2).bin"] {
            assert!(l.contains(&d.path().join(n)), "{n}");
        }
        assert_eq!(l.len(), 5);
    }

    #[test]
    fn une_rom_de_meme_taille_mais_differente_n_est_pas_prise_pour_la_meme() {
        let d = tempfile::tempdir().unwrap();
        ecrire(d.path(), "USA/Tetris.gb", "AAAA");
        ecrire(d.path(), "Europe/Tetris.gb", "BBBB");
        let dest = d.path().join("dest");
        copier_un(&d.path().join("USA/Tetris.gb"), &dest.join("Tetris.gb")).unwrap();
        assert!(matches!(copier_un(&d.path().join("Europe/Tetris.gb"), &dest.join("Tetris.gb")), Err(Erreur::Refus(_))));
        assert_eq!(std::fs::read(dest.join("Tetris.gb")).unwrap(), b"AAAA", "rien d'écrasé");
        copier_un(&d.path().join("USA/Tetris.gb"), &dest.join("Tetris.gb")).unwrap(); // la même : gardée
        assert!(!dest.join("Tetris.gb.frogtend-copie").exists(), "pas de fichier provisoire qui traîne");
    }

    #[test]
    fn le_titre_perd_ses_etiquettes_mais_pas_le_fichier() {
        assert_eq!(titre_depuis_nom("Super Mario World (USA) [!].sfc"), "Super Mario World");
        assert_eq!(titre_depuis_nom("Final Fantasy VII (France) (Disc 1).cue"), "Final Fantasy VII");
        assert_eq!(titre_depuis_nom("Sonic_the_Hedgehog_(Europe).md"), "Sonic the Hedgehog");
        assert_eq!(titre_depuis_nom("(Proto).bin"), "(Proto)", "rien d'autre : on garde le nom");
        assert_eq!(titre_depuis_nom("Dune"), "Dune");
    }

    fn ecrire(d: &Path, nom: &str, contenu: &str) {
        if let Some(p) = d.join(nom).parent() {
            std::fs::create_dir_all(p).unwrap();
        }
        std::fs::write(d.join(nom), contenu).unwrap();
    }

    #[test]
    fn les_types_de_fichiers_d_un_dossier_sont_comptes() {
        let d = tempfile::tempdir().unwrap();
        for f in ["a.zip", "b.ZIP", "c.zip", "film.mkv", "lisez-moi", "sous/d.zip", "sous/e.7z"] {
            ecrire(d.path(), f, "x");
        }
        assert_eq!(types_de_fichiers(d.path(), false).unwrap(), [("zip".to_string(), 3), ("mkv".to_string(), 1)]);
        assert_eq!(types_de_fichiers(d.path(), true).unwrap(), [("zip".to_string(), 4), ("7z".to_string(), 1), ("mkv".to_string(), 1)]);
        assert!(types_de_fichiers(&d.path().join("absent"), true).is_err());
    }

    #[test]
    fn les_pistes_d_un_cue_ou_d_un_m3u_ne_sont_pas_des_jeux() {
        let d = tempfile::tempdir().unwrap();
        ecrire(d.path(), "FF7 (France) (Disc 1).cue", "FILE \"FF7 (France) (Disc 1).bin\" BINARY\n  TRACK 01 MODE2/2352\n");
        ecrire(d.path(), "FF7 (France) (Disc 1).bin", "x");
        ecrire(d.path(), "Crash (Europe).cue", "FILE \"Crash (Europe) (Track 1).bin\" BINARY\nFILE \"crash (europe) (track 2).bin\" BINARY\n");
        ecrire(d.path(), "Crash (Europe) (Track 1).bin", "x");
        ecrire(d.path(), "Crash (Europe) (Track 2).bin", "x");
        ecrire(d.path(), "Seul (Japan).bin", "x");
        ecrire(d.path(), "notes.txt", "x");
        ecrire(d.path(), "sous/Ape Escape (Europe).cue", "FILE \"Ape Escape (Europe).bin\" BINARY\n");
        ecrire(d.path(), "sous/Ape Escape (Europe).bin", "x");
        let ext = vec!["cue".to_string(), ".BIN".to_string()];
        let l = chercher_roms(d.path(), &ext, false).unwrap();
        let titres: Vec<&str> = l.iter().map(|r| r.titre.as_str()).collect();
        assert_eq!(titres, ["Crash", "FF7", "Seul"]);
        assert!(l[1].chemin.ends_with("FF7 (France) (Disc 1).cue"), "le fichier garde son nom");
        assert_eq!(chercher_roms(d.path(), &ext, true).unwrap().len(), 4, "récursif : Ape Escape en plus");

        ecrire(d.path(), "m3u/FF8.m3u", "# disques\nFF8 (Disc 1).chd\nFF8 (Disc 2).chd\n");
        ecrire(d.path(), "m3u/FF8 (Disc 1).chd", "x");
        ecrire(d.path(), "m3u/FF8 (Disc 2).chd", "x");
        let l = chercher_roms(&d.path().join("m3u"), &["m3u".into(), "chd".into()], false).unwrap();
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].titre, "FF8");
        assert!(chercher_roms(d.path(), &[], false).is_err());
    }

    #[test]
    fn un_jeu_dos_par_sous_dossier_avec_son_programme_probable() {
        let d = tempfile::tempdir().unwrap();
        ecrire(d.path(), "Dune (1992)/INSTALL.EXE", "x");
        ecrire(d.path(), "Dune (1992)/DUNE.EXE", "x");
        ecrire(d.path(), "Dune (1992)/SETSOUND.EXE", "x");
        ecrire(d.path(), "Prince of Persia/PRINCE.EXE", "x");
        ecrire(d.path(), "Vide/LISEZMOI.TXT", "x");
        let l = chercher_jeux_dos(d.path()).unwrap();
        assert_eq!(l.len(), 2, "un dossier sans programme n'est pas un jeu");
        assert_eq!(l[0].titre, "Dune");
        assert_eq!(l[0].programmes[0], "DUNE.EXE");
        assert_eq!(l[1].programmes[0], "PRINCE.EXE");
    }

    #[test]
    fn copier_emporte_les_pistes_et_n_ecrase_jamais() {
        let d = tempfile::tempdir().unwrap();
        ecrire(d.path(), "src/Crash (Europe).cue", "FILE \"Crash (Europe) (Track 1).bin\" BINARY\n");
        ecrire(d.path(), "src/Crash (Europe) (Track 1).bin", "piste");
        ecrire(d.path(), "src/kinst.zip", "zip");
        ecrire(d.path(), "src/kinst/kinst.chd", "disque");
        ecrire(d.path(), "src/Dune/DUNE.EXE", "exe");
        let src = d.path().join("src");
        let elements = vec![src.join("Crash (Europe).cue"), src.join("kinst.zip"), src.join("Dune")];
        // Le .cue, sa piste (5), le zip (3), son CHD (6), le jeu DOS (3).
        let cue = std::fs::metadata(src.join("Crash (Europe).cue")).unwrap().len();
        assert_eq!(taille_a_copier(&elements), cue + 5 + 3 + 6 + 3);
        let dest = d.path().join("dest");
        let n = copier_jeux(&elements, &dest).unwrap();
        assert_eq!(n, [dest.join("Crash (Europe).cue"), dest.join("kinst.zip"), dest.join("Dune")]);
        assert!(dest.join("Crash (Europe) (Track 1).bin").is_file(), "la piste suit le .cue");
        assert!(dest.join("kinst").join("kinst.chd").is_file(), "le CHD suit le zip MAME");
        assert!(dest.join("Dune").join("DUNE.EXE").is_file());
        assert!(src.join("Crash (Europe).cue").is_file(), "les originaux restent");
        // Recommencer : rien ne change (mêmes tailles). Un fichier différent : refus, rien d'écrasé.
        assert!(copier_jeux(&elements, &dest).is_ok());
        std::fs::write(dest.join("kinst.zip"), "autre contenu").unwrap();
        assert!(copier_jeux(&elements[1..2], &dest).is_err());
        assert_eq!(std::fs::read(dest.join("kinst.zip")).unwrap(), b"autre contenu");
    }

    #[test]
    fn dosbox_monte_la_source_et_la_destination() {
        let d = tempfile::tempdir().unwrap();
        ecrire(d.path(), "cd/INSTALL.EXE", "x");
        ecrire(d.path(), "Kings Quest.iso", "x");
        ecrire(d.path(), "disk1.img", "x");
        ecrire(d.path(), "notes.txt", "x");
        let dest = Path::new("D:/Jeux DOS/Kings Quest/");
        let a = arguments_installation_dos(&d.path().join("cd"), dest).unwrap();
        assert_eq!(a[0], "-c");
        assert_eq!(a[1], r#"mount c "D:\Jeux DOS\Kings Quest""#, "pas de \\ final avant le guillemet");
        assert!(a[3].starts_with("mount d \"") && a[3].ends_with("\" -t cdrom"));
        assert_eq!(a[5], "d:");
        let iso = arguments_installation_dos(&d.path().join("Kings Quest.iso"), dest).unwrap();
        assert!(iso[3].starts_with("imgmount d ") && iso[3].ends_with(" -t iso"));
        let disquette = arguments_installation_dos(&d.path().join("disk1.img"), dest).unwrap();
        assert!(disquette[3].ends_with(" -t floppy"));
        assert_eq!(disquette[5], "a:");
        assert!(arguments_installation_dos(&d.path().join("notes.txt"), dest).is_err());
        assert_eq!(arguments_installation_dos(d.path(), Path::new("E:/")).unwrap()[1], r#"mount c "E:\""#);
    }

    #[test]
    fn un_jeu_dos_installe_se_relance_depuis_son_dossier() {
        let a = arguments_jeu_dos(Path::new("D:/Jeux DOS/KQ5"), "SIERRA/KQ5/SIERRA.EXE").unwrap();
        assert_eq!(a, ["-c", r#"mount c "D:\Jeux DOS\KQ5""#, "-c", "c:", "-c", r"cd \SIERRA\KQ5", "-c", "SIERRA.EXE", "-c", "exit"]);
        let b = arguments_jeu_dos(Path::new("D:/Jeux DOS/Dune"), "DUNE.EXE").unwrap();
        assert_eq!(b[5], r"cd \");
        assert!(arguments_jeu_dos(Path::new("D:/x"), "../evil.exe").is_err());
    }

    #[test]
    fn un_jeu_a_importer_est_verifie_sur_le_disque() {
        let d = tempfile::tempdir().unwrap();
        ecrire(d.path(), "jeu.sfc", "x");
        let racine = d.path().to_string_lossy().to_string();
        let mut j = JeuAImporter { titre: "Jeu".into(), plateforme: "SNES".into(), dossier: racine.clone(), fichier: Some("jeu.sfc".into()), programme: None, arguments: vec![], annee: None, editeur: None, genres: vec![] };
        assert!(verifier(&j).is_ok());
        j.fichier = Some("../ailleurs.sfc".into());
        assert!(verifier(&j).is_err());
        j.fichier = Some("absent.sfc".into());
        assert!(verifier(&j).is_err());
        j.fichier = None;
        assert!(verifier(&j).is_err());
        j.programme = Some(d.path().join("jeu.sfc").to_string_lossy().to_string());
        assert!(verifier(&j).is_ok());
        let a = JeuAImporter { titre: "A".into(), plateforme: "X".into(), dossier: "D:/Jeux".into(), fichier: Some("Jeu.SFC".into()), programme: None, arguments: vec![], annee: None, editeur: None, genres: vec![] };
        let b = JeuAImporter { titre: "B".into(), plateforme: "X".into(), dossier: "d:\\jeux".into(), fichier: Some("jeu.sfc".into()), programme: None, arguments: vec![], annee: None, editeur: None, genres: vec![] };
        assert_eq!(cle_import(&a), cle_import(&b), "même fichier, même jeu");
    }
}

#[cfg(test)]
mod essais {
    /// Sur le vrai disque de Seb, en LECTURE SEULE : ce que la recherche trouverait (rien n'est ajouté).
    #[test]
    #[ignore]
    fn essai_recherche_reelle() {
        let d = std::env::var("FROGTEND_ESSAI_DOSSIER").unwrap_or_else(|_| "E:/Games/Nintendo Entertainement System".into());
        let ext: Vec<String> = std::env::var("FROGTEND_ESSAI_EXT").unwrap_or_else(|_| "nes".into()).split(',').map(String::from).collect();
        let debut = std::time::Instant::now();
        let l = super::chercher_roms(std::path::Path::new(&d), &ext, true).unwrap();
        println!("{} jeu(x) en {:?}", l.len(), debut.elapsed());
        for r in l.iter().take(6) {
            println!("{} <- {}", r.titre, r.chemin);
        }
        // Le regroupement en fiches (une par jeu), avec leurs versions.
        let mut groupes: std::collections::BTreeMap<String, Vec<String>> = Default::default();
        for r in &l {
            groupes.entry(super::cle_de_jeu("x", &r.titre)).or_default().push(r.chemin.clone());
        }
        println!("{} fiche(s) pour {} fichier(s)", groupes.len(), l.len());
        for (cle, chemins) in groupes.iter().filter(|(_, c)| c.len() > 2).take(4) {
            println!("{cle} :");
            for v in super::en_versions(chemins) {
                println!("   [{}] {} — {}", v.rang, v.libelle, v.chemin.rsplit(['\\', '/']).next().unwrap_or(""));
            }
        }
    }
}


#[cfg(test)]
mod repetition_reelle {
    use super::*;
    #[tokio::test]
    #[ignore]
    async fn parcours_complet_sur_un_vrai_dossier() {
        use crate::coffre::CoffreMemoire;
        use crate::noyau::{Connexion, Noyau};
        let dossier = std::path::PathBuf::from(std::env::var("FROGTEND_DOSSIER").unwrap());
        let plateforme = std::env::var("FROGTEND_PLATEFORME").unwrap();
        let t = std::time::Instant::now();
        let types = types_de_fichiers(&dossier, true).unwrap();
        println!("ETAPE types {:?} en {} ms", types, t.elapsed().as_millis());
        let t = std::time::Instant::now();
        let l = chercher_roms(&dossier, &["zip".to_string()], true).unwrap();
        let (contenus, roms): (Vec<_>, Vec<_>) = l.into_iter().partition(|r| crate::contenus::est_un_contenu(Path::new(&r.chemin)));
        println!("ETAPE recherche {} jeux, {} contenus écartés, en {} ms", roms.len(), contenus.len(), t.elapsed().as_millis());
        for r in &roms {
            println!("  JEU {}", r.titre);
        }
        let elements: Vec<PathBuf> = roms.iter().map(|r| PathBuf::from(&r.chemin)).collect();
        let t = std::time::Instant::now();
        println!("ETAPE mesure {} Go en {} ms", taille_a_copier(&elements) >> 30, t.elapsed().as_millis());
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(CoffreMemoire::default())).unwrap();
        let profil = n.creer_profil("Essai", None, None).unwrap().id;
        n.ouvrir(&profil, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        let jeux: Vec<JeuAImporter> = roms
            .iter()
            .map(|r| {
                let p = Path::new(&r.chemin);
                JeuAImporter {
                    titre: r.titre.clone(),
                    plateforme: plateforme.clone(),
                    dossier: p.parent().unwrap().to_string_lossy().into(),
                    fichier: Some(p.file_name().unwrap().to_string_lossy().into()),
                    programme: None,
                    arguments: vec![],
                    annee: None,
                    editeur: None,
                    genres: vec![],
                }
            })
            .collect();
        let t = std::time::Instant::now();
        let b = n.importer_locaux(&jeux).await.unwrap();
        println!("ETAPE ajout {} fiches, {} refus, en {} ms", b.ajoutes, b.refuses.len(), t.elapsed().as_millis());
        let filtre = crate::ludotheque::Filtre { boutique: Some("local".into()), ludotheque: true, ..Default::default() };
        for j in n.lister(&filtre).await.unwrap().jeux {
            let t = std::time::Instant::now();
            let a = match n.fichier_lance(j.id).await.unwrap() {
                Some(f) => crate::decompression::examiner(&f).unwrap().map(|a| format!("décompresser {} Go -> {}", a.taille >> 30, a.principal)),
                None => None,
            };
            println!("  FICHE {} : {:?} ({} ms)", j.titre, a, t.elapsed().as_millis());
        }
    }
}
