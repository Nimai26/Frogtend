//! Les parties d'un jeu Steam gardées pour son nuage, mises à l'abri par Frogtend (lot B2 des jeux de boutique ;
//! Seb, 21/10 : « nuage Steam d'abord »). LECTURE SEULE chez Steam et dans les dossiers de Windows : Frogtend n'écrit
//! que dans le dossier des abris.
//!
//! Steam décrit les fichiers d'un jeu dans `userdata\<compte>\<appid>\remotecache.vdf` : chemin, racine, taille et
//! SHA1 (vérifié sur 50 vrais fichiers le 21/10). La racine 0 est `remote\` à côté ; les autres (Auto-Cloud : Mes
//! documents, AppData…) ont des NUMÉROS que Valve ne documente pas (seuls leurs noms le sont) : Frogtend ne les devine
//! pas, il retrouve chaque fichier par sa taille et son SHA1 parmi les dossiers de Windows lus dans le registre (Mes
//! documents peut être redirigé vers OneDrive : c'est le cas chez Seb). Un fichier OneDrive « en ligne seulement »
//! n'est jamais lu (il serait téléchargé).

use crate::erreurs::{Erreur, Resultat};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};

/// Le steamid64 d'un compte moins cette base = son numéro de dossier dans `userdata` (identifiant 32 bits).
const BASE_STEAMID: u64 = 76_561_197_960_265_728;

/// Au-delà, la personne confirme avant la copie (des parties de jeu font rarement plus).
pub const GROS: u64 = 2 << 30;

/// Le dossier `userdata\<compte>` d'un steamid64 (`None` : pas un steamid64).
pub fn compte(steamid: &str) -> Option<String> {
    let n: u64 = steamid.trim().parse().ok()?;
    n.checked_sub(BASE_STEAMID).filter(|c| *c > 0 && *c <= u32::MAX as u64).map(|c| c.to_string())
}

/// La marque d'un compte Steam dans les abris (`nuage-steam/<marque>/…`) : 8 hexadécimaux, jamais le numéro du
/// compte (les abris partent chez Firehouse). Deux comptes Steam du PC ne mêlent jamais leurs parties.
pub fn marque(compte: &str) -> String {
    use sha1::{Digest, Sha1};
    Sha1::digest(format!("frogtend-steam:{compte}").as_bytes()).iter().take(4).map(|b| format!("{b:02x}")).collect()
}

/// Au plus ces fichiers dans `remote\` (un dossier plus gros n'est pas celui des parties d'un jeu).
const REMOTE_MAX: usize = 20_000;

/// Les fichiers sous `racine` (chemins relatifs en `/`), SANS jamais suivre un lien ni une jonction (un lien dans
/// `remote\` ferait lire un disque entier) ; au-delà de `REMOTE_MAX`, refus.
fn fichiers_sans_lien(racine: &Path) -> Resultat<Vec<String>> {
    let mut l = Vec::new();
    let mut pile = vec![racine.to_path_buf()];
    while let Some(d) = pile.pop() {
        for e in std::fs::read_dir(&d)?.flatten() {
            let p = e.path();
            let Ok(m) = std::fs::symlink_metadata(&p) else { continue };
            if m.file_type().is_symlink() || point_de_reparse(&m) {
                continue;
            }
            if m.is_dir() {
                pile.push(p);
            } else if m.is_file() {
                if let Ok(r) = p.strip_prefix(racine) {
                    l.push(r.to_string_lossy().replace('\\', "/"));
                }
                if l.len() > REMOTE_MAX {
                    return Err(Erreur::Refus("Ce dossier contient trop de fichiers pour être celui des parties : rien n'en est copié.".into()));
                }
            }
        }
    }
    l.sort();
    Ok(l)
}

/// Une jonction, un lien ou un fichier géré par un service (OneDrive…) : FILE_ATTRIBUTE_REPARSE_POINT.
fn point_de_reparse(m: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // Un fichier OneDrive porte aussi cet attribut : seuls les DOSSIERS qui l'ont sont écartés.
        m.is_dir() && m.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        let _ = m;
        false
    }
}

/// Un fichier décrit par `remotecache.vdf`.
#[derive(Debug, Clone, PartialEq)]
pub struct FichierNuage {
    pub chemin: String,
    pub racine: String,
    pub taille: Option<u64>,
    /// SHA1 en 40 hexadécimaux minuscules (vide : inconnu).
    pub sha: String,
    /// `syncstate` différent de 1 : Steam n'a pas fini de synchroniser (conflit possible).
    pub en_attente: bool,
}

#[derive(Debug, PartialEq)]
enum Jeton {
    Texte(String),
    Ouvre,
    Ferme,
}

/// Les jetons du format texte de Valve (`"clé" "valeur"`, `{`, `}` ; `\\` et `\"` dans les textes). Tolérant : un
/// fichier tronqué rend ce qu'il a pu lire, jamais une panique.
fn jetons(t: &str) -> Vec<Jeton> {
    let mut l = Vec::new();
    let mut c = t.chars().peekable();
    while let Some(x) = c.next() {
        match x {
            '{' => l.push(Jeton::Ouvre),
            '}' => l.push(Jeton::Ferme),
            '"' => {
                let mut s = String::new();
                while let Some(y) = c.next() {
                    match y {
                        '"' => break,
                        '\\' => match c.next() {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some(z) => s.push(z),
                            None => break,
                        },
                        _ => s.push(y),
                    }
                }
                l.push(Jeton::Texte(s));
            }
            _ => {}
        }
    }
    l
}

/// Lit `remotecache.vdf` : un bloc par fichier, dans le bloc du jeu.
pub fn lire_remotecache(texte: &str) -> Vec<FichierNuage> {
    let j = jetons(texte);
    let mut l = Vec::new();
    let mut profondeur = 0usize;
    let mut i = 0;
    while i < j.len() {
        match &j[i] {
            Jeton::Ouvre => profondeur += 1,
            Jeton::Ferme => profondeur = profondeur.saturating_sub(1),
            Jeton::Texte(nom) if profondeur == 1 && j.get(i + 1) == Some(&Jeton::Ouvre) => {
                // Le bloc d'un fichier : ses paires clé / valeur, jusqu'à son « } » (un bloc imbriqué est sauté).
                let mut f = FichierNuage { chemin: nom.clone(), racine: String::new(), taille: None, sha: String::new(), en_attente: false };
                let mut k = i + 2;
                let mut p = 1usize;
                while k < j.len() && p > 0 {
                    match (&j[k], j.get(k + 1)) {
                        (Jeton::Ouvre, _) => p += 1,
                        (Jeton::Ferme, _) => p -= 1,
                        (Jeton::Texte(cle), Some(Jeton::Texte(v))) if p == 1 => {
                            match cle.to_lowercase().as_str() {
                                "root" => f.racine = v.clone(),
                                "size" => f.taille = v.parse().ok(),
                                "sha" => f.sha = v.to_lowercase(),
                                "syncstate" => f.en_attente = v != "1",
                                _ => {}
                            }
                            k += 1;
                        }
                        _ => {}
                    }
                    k += 1;
                }
                l.push(f);
                i = k;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    l
}

/// Les dossiers de Windows où l'Auto-Cloud de Steam peut ranger des parties, lus dans le registre (« User Shell
/// Folders », développés) : (nom dans l'abri, dossier). Mes documents d'abord.
pub fn dossiers_windows() -> Vec<(&'static str, PathBuf)> {
    const CLE: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders";
    let mut l = Vec::new();
    for (nom, valeur) in [
        ("Documents", "Personal"),
        ("AppData-Local", "Local AppData"),
        ("AppData-Roaming", "AppData"),
        ("AppData-LocalLow", "{A520A1A4-1780-4FF6-BD18-167343C5AF16}"),
        ("Parties-enregistrees", "{4C5C32FF-BB9D-43B0-B5B4-2D72E54EAAA4}"),
    ] {
        if let Some(d) = crate::registre::lire_texte_hkcu(CLE, valeur).map(PathBuf::from).filter(|d| d.is_dir()) {
            l.push((nom, d));
        }
    }
    // LocalLow et « Parties enregistrées » ne sont souvent pas notés dans le registre (relevé du 21/10) : leur place
    // habituelle, s'ils existent.
    if let Some(u) = std::env::var_os("USERPROFILE").map(PathBuf::from) {
        for (nom, d) in [("AppData-LocalLow", u.join("AppData").join("LocalLow")), ("Parties-enregistrees", u.join("Saved Games"))] {
            if !l.iter().any(|(n, _)| *n == nom) && d.is_dir() {
                l.push((nom, d));
            }
        }
    }
    l
}

/// Un fichier « en ligne seulement » (OneDrive…) : le lire le téléchargerait.
fn en_ligne_seulement(m: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // FILE_ATTRIBUTE_OFFLINE, RECALL_ON_OPEN, RECALL_ON_DATA_ACCESS.
        m.file_attributes() & (0x1000 | 0x4_0000 | 0x40_0000) != 0
    }
    #[cfg(not(windows))]
    {
        let _ = m;
        false
    }
}

/// Le SHA1 d'un fichier, lu par morceaux (jamais en entier en mémoire).
fn sha1(f: &Path) -> std::io::Result<String> {
    use sha1::{Digest, Sha1};
    let mut r = std::fs::File::open(f)?;
    let mut h = Sha1::new();
    let mut t = vec![0u8; 256 * 1024];
    loop {
        let n = r.read(&mut t)?;
        if n == 0 {
            break;
        }
        h.update(&t[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Un fichier à copier : sa source, sa place dans l'abri (`nuage-steam/<dossier>/<chemin>`), sa taille, son SHA1.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AMettre {
    #[serde(skip)]
    pub source: PathBuf,
    pub relatif: String,
    pub taille: u64,
    pub sha: String,
}

/// Ce que Frogtend mettrait à l'abri, et ce qu'il laisse (dit à la personne).
#[derive(Debug, Clone, Default, Serialize)]
pub struct Plan {
    pub fichiers: Vec<AMettre>,
    pub octets: u64,
    /// Décrits par Steam mais introuvables tels quels sur ce PC (ou à une racine qui n'est pas une partie).
    pub introuvables: usize,
    /// Dans OneDrive « en ligne seulement » : pas lus.
    pub en_ligne_seulement: usize,
    /// Steam n'a pas fini de les synchroniser.
    pub en_attente: usize,
    /// Plus récents que ce que Steam a noté (une partie jouée depuis sa dernière synchronisation) : copiés tels quels.
    pub modifies: usize,
    /// Fichiers pris HORS du nuage, aux emplacements que Firehouse donne (contrat 1.25).
    pub hors_nuage: usize,
    /// Emplacements donnés par Firehouse que Frogtend ne sait pas lire (racine perdue, jeton inconnu…).
    pub emplacements_illisibles: usize,
}

/// Ce qu'on trouve au chemin d'un fichier décrit par Steam, dans un dossier.
enum Trouve {
    /// Le fichier tel que Steam le décrit (taille, SHA1).
    Exact(PathBuf, u64, String),
    /// Un fichier à cet endroit, mais différent (plus récent que la dernière synchronisation ?).
    Autre(PathBuf, u64),
    /// Seulement dans un nuage (OneDrive…) : jamais lu.
    EnLigne,
    Absent,
}

fn retrouver(base: &Path, rel: &str, f: &FichierNuage) -> Trouve {
    let p = base.join(rel);
    let Ok(m) = std::fs::symlink_metadata(&p) else { return Trouve::Absent };
    if !m.is_file() {
        return Trouve::Absent;
    }
    if en_ligne_seulement(&m) {
        return Trouve::EnLigne;
    }
    if f.taille.is_some_and(|t| t != m.len()) {
        return Trouve::Autre(p, m.len());
    }
    match sha1(&p) {
        Ok(sha) if f.sha.is_empty() || f.sha == sha => Trouve::Exact(p, m.len(), sha),
        Ok(_) => Trouve::Autre(p, m.len()),
        Err(_) => Trouve::Absent,
    }
}

/// Le plan d'un jeu : `app` = `userdata\<compte>\<appid>`, `marque` = celle du compte (`marque`) ; `None` si Steam
/// ne garde rien de ce jeu.
pub fn planifier(app: &Path, marque: &str, windows: &[(&'static str, PathBuf)]) -> Resultat<Option<Plan>> {
    let remote = app.join("remote");
    let vdf = std::fs::read(app.join("remotecache.vdf")).ok().map(|o| String::from_utf8_lossy(&o).into_owned());
    if vdf.is_none() && !remote.is_dir() {
        return Ok(None);
    }
    let mut plan = Plan::default();
    let mut vus = std::collections::BTreeSet::new();
    for f in vdf.as_deref().map(lire_remotecache).unwrap_or_default() {
        // Racine 1 : les configurations de manette de Steam (relevé du 21/10), pas des parties.
        if f.racine == "1" {
            continue;
        }
        if f.en_attente {
            plan.en_attente += 1;
        }
        // Un chemin qui sortirait de son dossier (« .. », absolu) n'est jamais suivi.
        let Some(rel) = crate::decompression::chemin_sur(&f.chemin) else {
            plan.introuvables += 1;
            continue;
        };
        // Racine 0 : `remote\` ; sinon les dossiers de Windows, chacun vérifié (taille + SHA1), jamais deviné.
        let bases: Vec<(&str, &Path)> = if f.racine == "0" { vec![("remote", remote.as_path())] } else { windows.iter().map(|(n, d)| (*n, d.as_path())).collect() };
        let mut exact = None;
        let mut autres = Vec::new();
        let mut nuage = false;
        for (nom, base) in bases {
            match retrouver(base, &rel, &f) {
                Trouve::Exact(p, t, sha) => {
                    exact = Some((nom, p, t, sha));
                    break;
                }
                Trouve::Autre(p, t) => autres.push((nom, p, t)),
                Trouve::EnLigne => nuage = true,
                Trouve::Absent => {}
            }
        }
        // Tel que Steam le décrit ; sinon, s'il n'est qu'à UN endroit, la partie plus récente que Steam n'a pas encore
        // synchronisée (jamais perdue) ; à plusieurs endroits différents, on ne devine pas.
        let choisi = match (exact, autres.as_slice()) {
            (Some(x), _) => Some(x),
            (None, [(nom, p, t)]) => match sha1(p) {
                Ok(sha) => {
                    plan.modifies += 1;
                    Some((*nom, p.clone(), *t, sha))
                }
                Err(_) => None,
            },
            _ => None,
        };
        match choisi {
            Some((nom, source, taille, sha)) => {
                let relatif = format!("nuage-steam/{marque}/{nom}/{rel}");
                if vus.insert(relatif.to_lowercase()) {
                    plan.octets += taille;
                    plan.fichiers.push(AMettre { source, relatif, taille, sha });
                }
            }
            None if nuage => plan.en_ligne_seulement += 1,
            None => plan.introuvables += 1,
        }
    }
    // Ce qui est dans `remote\` sans être décrit (Steam peut ne pas avoir tout noté) : copié aussi.
    if remote.is_dir() {
        for r in fichiers_sans_lien(&remote)? {
            let relatif = format!("nuage-steam/{marque}/remote/{r}");
            if vus.contains(&relatif.to_lowercase()) {
                continue;
            }
            let p = remote.join(&r);
            let Ok(m) = std::fs::symlink_metadata(&p) else { continue };
            if !m.is_file() || en_ligne_seulement(&m) {
                continue;
            }
            let sha = sha1(&p)?;
            vus.insert(relatif.to_lowercase());
            plan.octets += m.len();
            plan.fichiers.push(AMettre { source: p, relatif, taille: m.len(), sha });
        }
    }
    plan.fichiers.sort_by(|a, b| a.relatif.cmp(&b.relatif));
    Ok(Some(plan))
}

/// De quoi résoudre un emplacement de parties donné par Firehouse (contrat 1.25 `sauvegardes`, chemins NON résolus).
pub struct Contexte {
    /// Le dossier du jeu (`<dossier du jeu>`).
    pub jeu: Option<PathBuf>,
    /// Le dossier de Steam (`<Steam>`).
    pub steam: Option<PathBuf>,
    /// Le compte Steam (`<identifiant Steam>`, le dossier de `userdata`).
    pub compte: Option<String>,
    /// Les dossiers de Windows lus dans le registre (`dossiers_windows`) : « Mes documents » peut être chez OneDrive.
    pub windows: Vec<(&'static str, PathBuf)>,
}

/// Un emplacement de Firehouse rendu en vrai dossier de CE PC, ou `None` (jeton inconnu, racine perdue, registre,
/// joker, chemin d'un autre système). Jamais deviné : seuls les jetons connus du contrat 1.25.
pub fn resoudre(chemin: &str, ctx: &Contexte) -> Option<PathBuf> {
    let c = chemin.trim().replace('/', "\\");
    if c.is_empty() || c.starts_with('\\') || c.starts_with('~') || c.to_uppercase().starts_with("HKEY") || c.contains('*') || c.contains('?') {
        return None;
    }
    let win = |n: &str| ctx.windows.iter().find(|(x, _)| *x == n).map(|(_, d)| d.to_string_lossy().to_string());
    let env = |n: &str| std::env::var(n).ok().filter(|v| !v.is_empty());
    // Les plus longs d'abord : « %USERPROFILE%\Documents » avant « %USERPROFILE% ».
    let jetons: Vec<(&str, Option<String>)> = vec![
        ("%USERPROFILE%\\Documents", win("Documents")),
        ("%USERPROFILE%\\My Documents", win("Documents")),
        ("%USERPROFILE%\\Saved Games", win("Parties-enregistrees")),
        ("%USERPROFILE%\\AppData\\LocalLow", win("AppData-LocalLow")),
        ("%LOCALAPPDATA%", win("AppData-Local").or_else(|| env("LOCALAPPDATA"))),
        ("%APPDATA%", win("AppData-Roaming").or_else(|| env("APPDATA"))),
        ("%USERPROFILE%", env("USERPROFILE")),
        ("%PROGRAMDATA%", env("ProgramData")),
        ("%ALLUSERSPROFILE%", env("ALLUSERSPROFILE")),
        ("%PUBLIC%", env("PUBLIC")),
        ("%USERNAME%", env("USERNAME")),
        ("<dossier du jeu>", ctx.jeu.as_ref().map(|d| d.to_string_lossy().to_string())),
        ("<Steam>", ctx.steam.as_ref().map(|d| d.to_string_lossy().to_string())),
        ("<identifiant Steam>", ctx.compte.clone()),
    ];
    let mut r = c;
    for (jeton, valeur) in jetons {
        // En minuscules ASCII seulement : les positions restent celles du texte d'origine (accents compris).
        let bas = r.to_ascii_lowercase();
        if let Some(i) = bas.find(&jeton.to_ascii_lowercase()) {
            // Le compte Steam seulement SOUS `userdata` : ailleurs (le profil Rockstar de GTA V…), c'est l'identifiant
            // propre au jeu, inconnu (Firehouse, 21/10) — `emplacements` énumère alors les sous-dossiers.
            if jeton == "<identifiant Steam>" && !bas[..i].ends_with("\\userdata\\") {
                return None;
            }
            let v = valeur?;
            r = format!("{}{}{}", &r[..i], v.trim_end_matches('\\'), &r[i + jeton.len()..]);
        }
    }
    // Un jeton resté inconnu (« <jeton> », « %X% ») : jamais deviné.
    if r.contains('%') || r.contains('<') || r.contains('>') {
        return None;
    }
    let p = PathBuf::from(r.trim_end_matches('\\'));
    // Un chemin absolu de ce PC, sans « .. ».
    (p.is_absolute() && !p.components().any(|c| matches!(c, std::path::Component::ParentDir))).then_some(p)
}

/// Au plus ce nombre de sous-dossiers « un par utilisateur » énumérés (au-delà, ce n'est pas un dossier de profils).
const PROFILS_MAX: usize = 32;

/// Les dossiers d'un emplacement : un seul, ou — quand l'identifiant de l'utilisateur est HORS de `userdata` (le
/// profil propre au jeu, inconnu) — chaque vrai sous-dossier (sans lien) du dossier qui le précède, suivi de la fin
/// du chemin. `None` : illisible.
fn bases(base: &str, ctx: &Contexte) -> Option<Vec<PathBuf>> {
    let bas = base.to_ascii_lowercase();
    let jeton = ["<identifiant utilisateur>", "<identifiant steam>"]
        .into_iter()
        .filter_map(|j| bas.find(j).map(|i| (i, j.len())))
        .find(|(i, _)| !bas[..*i].ends_with("\\userdata\\"));
    let Some((i, n)) = jeton else { return resoudre(base, ctx).map(|p| vec![p]) };
    let parent = resoudre(base[..i].trim_end_matches('\\'), ctx)?;
    let reste = base[i + n..].trim_matches('\\');
    // Le reste ne doit plus porter d'identifiant ni de jeton : un seul niveau énuméré.
    if reste.contains(['<', '>', '%']) {
        return None;
    }
    let Ok(l) = std::fs::read_dir(&parent) else { return Some(vec![]) };
    let mut v: Vec<PathBuf> = l
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter(|e| !crate::garde::lien_entre(&parent, &e.path()))
        .map(|e| if reste.is_empty() { e.path() } else { e.path().join(reste) })
        .filter(|p| !p.components().any(|c| matches!(c, std::path::Component::ParentDir)))
        .collect();
    if v.len() > PROFILS_MAX {
        return None;
    }
    v.sort();
    Some(v)
}

/// Un emplacement de parties résolu : un dossier (sous-dossiers compris), ou seulement ses fichiers qui répondent à
/// `filtre` (contrat 1.26 `nature` : `fichier` = un nom exact, `motif` = un nom avec * ou ?).
#[derive(Debug, Clone, PartialEq)]
pub struct Emplacement {
    pub dossier: PathBuf,
    pub filtre: Option<String>,
}

/// Un nom de fichier qui répond à un motif (`*`, `?`), sans tenir compte des majuscules (comme Windows).
fn repond(nom: &str, motif: &str) -> bool {
    fn m(n: &[char], p: &[char]) -> bool {
        match (p.first(), n.first()) {
            (None, None) => true,
            (Some('*'), _) => m(n, &p[1..]) || (!n.is_empty() && m(&n[1..], p)),
            (Some('?'), Some(_)) => m(&n[1..], &p[1..]),
            (Some(a), Some(b)) if a == b => m(&n[1..], &p[1..]),
            _ => false,
        }
    }
    let (n, p): (Vec<char>, Vec<char>) = (nom.to_lowercase().chars().collect(), motif.to_lowercase().chars().collect());
    m(&n, &p)
}

/// Une RACINE (dossier du jeu, de Steam, de Windows, de l'utilisateur, d'un disque) : jamais un emplacement de
/// parties (elle ferait copier un disque entier) — comptée comme illisible.
fn est_une_racine(p: &Path, ctx: &Contexte) -> bool {
    let cle = |x: &Path| x.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_lowercase();
    let c = cle(p);
    let env = ["USERPROFILE", "ProgramData", "ALLUSERSPROFILE", "PUBLIC", "LOCALAPPDATA", "APPDATA", "SystemRoot", "ProgramFiles", "ProgramFiles(x86)"];
    p.components().count() <= 2
        || ctx.jeu.iter().chain(ctx.steam.iter()).chain(ctx.windows.iter().map(|(_, d)| d)).any(|r| cle(r) == c)
        || env.iter().filter_map(|v| std::env::var_os(v)).any(|r| cle(Path::new(&r)) == c)
}

/// Les emplacements de parties d'un jeu Steam d'après Firehouse (`sauvegardes` : seulement « Windows » et « Steam »,
/// liste ouverte : le reste est ignoré), résolus : (emplacements existants, nombre d'illisibles).
pub fn emplacements(sauvegardes: &serde_json::Value, ctx: &Contexte) -> (Vec<Emplacement>, usize) {
    let mut l: Vec<Emplacement> = Vec::new();
    let mut illisibles = 0;
    for s in sauvegardes.as_array().into_iter().flatten() {
        if !matches!(s["os"].as_str(), Some("Windows") | Some("Steam")) {
            continue;
        }
        let Some(chemin) = s["chemin"].as_str() else { continue };
        let c = chemin.trim().replace('/', "\\");
        let dernier = c.trim_end_matches('\\').rsplit('\\').next().unwrap_or("").to_string();
        // La nature (contrat 1.26), sinon déduite comme Firehouse la décrit.
        let nature = s["nature"].as_str().map(String::from).unwrap_or_else(|| {
            if dernier.contains('*') || dernier.contains('?') {
                "motif".into()
            } else if !c.ends_with('\\') && dernier.contains('.') {
                "fichier".into()
            } else {
                "dossier".into()
            }
        });
        let (base, filtre) = match nature.as_str() {
            "fichier" | "motif" => (c.trim_end_matches('\\').rsplit_once('\\').map(|(b, _)| b.to_string()).unwrap_or_default(), Some(dernier)),
            "dossier" => (c.clone(), None),
            _ => {
                illisibles += 1;
                continue;
            }
        };
        if filtre.as_deref().is_some_and(|f| f.is_empty() || f.contains(['/', '\\', ':'])) {
            illisibles += 1;
            continue;
        }
        let Some(candidats) = bases(&base, ctx) else {
            illisibles += 1;
            continue;
        };
        for p in candidats {
            if est_une_racine(&p, ctx) && filtre.is_none() {
                illisibles += 1;
            } else if p.is_dir() {
                let e = Emplacement { dossier: p, filtre: filtre.clone() };
                if !l.contains(&e) {
                    l.push(e);
                }
            } // sinon : pas (encore) de parties à cet endroit
        }
    }
    // Un dossier entier DANS un autre déjà pris : une seule fois.
    let entiers: Vec<String> = l.iter().filter(|e| e.filtre.is_none()).map(|e| format!("{}\\", e.dossier.to_string_lossy().to_lowercase())).collect();
    l.retain(|e| {
        let moi = format!("{}\\", e.dossier.to_string_lossy().to_lowercase());
        !entiers.iter().any(|x| moi.starts_with(x.as_str()) && (moi != *x || e.filtre.is_some()))
    });
    (l, illisibles)
}

/// Au plus cette taille par emplacement de parties (au-delà, ce n'est pas un dossier de parties : il est compté
/// illisible, rien n'en est lu).
pub const EMPLACEMENT_MAX: u64 = 2 << 30;

/// Ajoute au plan les fichiers de ces emplacements (hors nuage), sans lien ni jonction ; un fichier déjà dans le plan
/// (le nuage le couvre) n'est pas pris deux fois. Un emplacement trop gros ou illisible ne gêne jamais les autres.
pub fn ajouter_emplacements(plan: &mut Plan, marque: &str, emplacements: &[Emplacement]) {
    let normal = |p: &Path| p.to_string_lossy().replace('/', "\\").to_lowercase();
    let deja: std::collections::BTreeSet<String> = plan.fichiers.iter().map(|f| normal(&f.source)).collect();
    for e in emplacements {
        // Une clé stable (le chemin), pour que « déjà à l'abri » ne change pas quand un autre emplacement apparaît.
        let cle: String = {
            use sha1::{Digest, Sha1};
            Sha1::digest(normal(&e.dossier).as_bytes()).iter().take(4).map(|b| format!("{b:02x}")).collect()
        };
        let nom: String = e.dossier.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_else(|| "parties".into()).chars().map(|c| if c.is_alphanumeric() || " -_.".contains(c) { c } else { '_' }).collect();
        let relatifs: Vec<String> = match &e.filtre {
            None => match fichiers_sans_lien(&e.dossier) {
                Ok(l) => l,
                Err(_) => {
                    plan.emplacements_illisibles += 1;
                    continue;
                }
            },
            Some(f) => std::fs::read_dir(&e.dossier)
                .into_iter()
                .flatten()
                .flatten()
                .filter(|x| x.file_type().is_ok_and(|t| t.is_file()))
                .map(|x| x.file_name().to_string_lossy().to_string())
                .filter(|n| repond(n, f))
                .collect(),
        };
        // La taille d'abord (rien n'est lu au-delà du plafond).
        let a_prendre: Vec<(PathBuf, String, std::fs::Metadata)> = relatifs
            .into_iter()
            .filter_map(|r| {
                let p = e.dossier.join(&r);
                let m = std::fs::symlink_metadata(&p).ok()?;
                (m.is_file() && !deja.contains(&normal(&p))).then_some((p, r, m))
            })
            .collect();
        if a_prendre.iter().map(|(_, _, m)| m.len()).sum::<u64>() > EMPLACEMENT_MAX {
            plan.emplacements_illisibles += 1;
            continue;
        }
        for (p, r, m) in a_prendre {
            if en_ligne_seulement(&m) {
                plan.en_ligne_seulement += 1;
                continue;
            }
            let Ok(sha) = sha1(&p) else { continue };
            plan.octets += m.len();
            plan.hors_nuage += 1;
            plan.fichiers.push(AMettre { source: p, relatif: format!("nuage-steam/{marque}/hors-nuage/{cle}-{nom}/{}", r.replace('\\', "/")), taille: m.len(), sha });
        }
    }
    plan.fichiers.sort_by(|a, b| a.relatif.cmp(&b.relatif));
}

/// La liste rangée dans chaque abri du nuage Steam, par compte (pour ne pas refaire un abri identique).
fn liste(marque: &str) -> String {
    format!("nuage-steam/{marque}/abri.json")
}

/// Le plus récent abri de ce jeu POUR CE COMPTE contient-il déjà exactement ces fichiers (chemins et SHA1) ?
pub fn deja_a_l_abri(abris_du_jeu: &Path, marque: &str, plan: &Plan) -> bool {
    let mut dates: Vec<(u64, PathBuf)> = std::fs::read_dir(abris_du_jeu)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().to_str().and_then(|n| n.parse::<u64>().ok()).map(|n| (n, e.path())))
        .collect();
    dates.sort_by(|a, b| b.0.cmp(&a.0));
    let Some(o) = dates.iter().find_map(|(_, d)| std::fs::read(d.join(liste(marque))).ok()) else { return false };
    let Ok(avant) = serde_json::from_slice::<Vec<AMettre>>(&o) else { return false };
    let cle = |l: &[AMettre]| l.iter().map(|f| (f.relatif.to_lowercase(), f.sha.clone())).collect::<std::collections::BTreeSet<_>>();
    cle(&avant) == cle(&plan.fichiers)
}

/// Copie le plan dans un nouvel abri `<abris>/<id>/<quand>/`, en passant par un dossier provisoire hors des abris
/// lus (un abri à moitié fait n'est jamais pris pour une sauvegarde). Chaque copie est vérifiée par son SHA1 ; un
/// fichier qui change pendant la copie (Steam synchronise) est recopié une fois, sinon tout est refusé.
pub fn abriter(plan: &Plan, marque: &str, abris: &Path, id: i64, quand: &str) -> Resultat<PathBuf> {
    let fin = abris.join(id.to_string()).join(quand);
    if fin.exists() {
        return Err(Erreur::Refus("Un abri de ce jeu vient d'être fait : réessaie dans une seconde.".into()));
    }
    let tmp = abris.join(".frogtend-en-cours").join(format!("{id}-{quand}"));
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp)?;
    }
    let r = (|| -> Resultat<()> {
        for f in &plan.fichiers {
            let cible = tmp.join(&f.relatif);
            if let Some(p) = cible.parent() {
                std::fs::create_dir_all(p)?;
            }
            let mut bon = false;
            for _ in 0..2 {
                std::fs::copy(&f.source, &cible)?;
                if sha1(&cible)? == f.sha {
                    bon = true;
                    break;
                }
            }
            if !bon {
                return Err(Erreur::Disque(format!(
                    "« {} » a changé pendant la copie (Steam synchronise peut-être) : rien n'est mis à l'abri, réessaie dans un moment.",
                    f.relatif
                )));
            }
        }
        let l = tmp.join(liste(marque));
        if let Some(p) = l.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::write(l, serde_json::to_vec_pretty(&plan.fichiers).unwrap_or_default())?;
        if let Some(p) = fin.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::rename(&tmp, &fin)?;
        Ok(())
    })();
    if r.is_err() {
        // Le dossier provisoire est le nôtre (son nom le dit) : rien d'autre n'est touché.
        let _ = std::fs::remove_dir_all(&tmp);
    }
    let _ = std::fs::remove_dir(abris.join(".frogtend-en-cours")); // seulement s'il est vide
    r?;
    Ok(fin)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La forme réelle de `remotecache.vdf` (relevé du 21/10, valeurs changées).
    const VDF: &str = "\"1466860\"\n{\n\t\"ChangeNumber\"\t\t\"7\"\n\t\"OSType\"\t\t\"0\"\n\t\"campaign\"\n\t{\n\t\t\"root\"\t\t\"0\"\n\t\t\"size\"\t\t\"5\"\n\t\t\"localtime\"\t\t\"1\"\n\t\t\"sha\"\t\t\"SHA\"\n\t\t\"syncstate\"\t\t\"1\"\n\t}\n\t\"my games/Jeu/SaveData/save_1\"\n\t{\n\t\t\"root\"\t\t\"2\"\n\t\t\"size\"\t\t\"3\"\n\t\t\"sha\"\t\t\"SHA2\"\n\t\t\"syncstate\"\t\t\"3\"\n\t}\n}\n";

    fn sha_de(o: &[u8]) -> String {
        use sha1::{Digest, Sha1};
        Sha1::digest(o).iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn un_steamid64_donne_son_dossier_userdata() {
        assert_eq!(compte("76561198000000000").as_deref(), Some("39734272"));
        assert_eq!(compte("abc"), None);
        assert_eq!(compte("76561197960265728"), None, "la base seule n'est pas un compte");
        assert_eq!(compte("1"), None);
    }

    #[test]
    fn remotecache_se_lit_meme_tronque() {
        let l = lire_remotecache(VDF);
        assert_eq!(l.len(), 2);
        assert_eq!((l[0].chemin.as_str(), l[0].racine.as_str(), l[0].taille, l[0].sha.as_str(), l[0].en_attente), ("campaign", "0", Some(5), "sha", false));
        assert!(l[1].en_attente, "syncstate 3 : pas fini");
        assert!(lire_remotecache("").is_empty());
        assert_eq!(lire_remotecache(&VDF[..VDF.len() / 2]).len(), 1, "tronqué : ce qui a pu être lu");
        assert_eq!(lire_remotecache("\"a\" { \"b\\\\c\" { \"root\" \"0\" } }")[0].chemin, "b\\c");
    }

    #[test]
    fn le_plan_retrouve_chaque_fichier_par_son_sha1_et_ne_devine_jamais() {
        let d = tempfile::tempdir().unwrap();
        let app = d.path().join("userdata").join("1").join("42");
        let (docs, local) = (d.path().join("OneDrive-Documents"), d.path().join("Local"));
        std::fs::create_dir_all(app.join("remote")).unwrap();
        std::fs::create_dir_all(docs.join("my games/Jeu/SaveData")).unwrap();
        std::fs::create_dir_all(local.join("my games/Jeu/SaveData")).unwrap();
        std::fs::write(app.join("remote/campaign"), b"12345").unwrap();
        std::fs::write(app.join("remote/en-plus"), b"x").unwrap();
        // Le même chemin dans deux dossiers : seul celui qui a le bon SHA1 est pris.
        std::fs::write(local.join("my games/Jeu/SaveData/save_1"), b"zzz").unwrap();
        std::fs::write(docs.join("my games/Jeu/SaveData/save_1"), b"abc").unwrap();
        let vdf = VDF.replace("\"SHA\"", &format!("\"{}\"", sha_de(b"12345"))).replace("\"SHA2\"", &format!("\"{}\"", sha_de(b"abc")));
        let vdf = vdf.replace("\t}\n}\n", "\t}\n\t\"../../Windows/win.ini\"\n\t{\n\t\t\"root\"\t\t\"2\"\n\t}\n\t\"absent\"\n\t{\n\t\t\"root\"\t\t\"3\"\n\t}\n}\n");
        // Une partie jouée depuis la dernière synchronisation (SHA1 différent), à UN seul endroit : gardée, jamais perdue.
        let vdf = vdf.replace("}\n}\n", "}\n\t\"my games/Jeu/SaveData/save_2\"\n\t{\n\t\t\"root\"\t\t\"2\"\n\t\t\"sha\"\t\t\"0000\"\n\t}\n\t\"manette.vdf\"\n\t{\n\t\t\"root\"\t\t\"1\"\n\t}\n}\n");
        std::fs::write(docs.join("my games/Jeu/SaveData/save_2"), b"nouveau").unwrap();
        std::fs::write(app.join("remotecache.vdf"), vdf).unwrap();
        // Une jonction (ici un lien de dossier) dans `remote\` n'est jamais suivie.
        #[cfg(windows)]
        let _ = std::os::windows::fs::symlink_dir(&local, app.join("remote").join("lien"));
        let windows = vec![("AppData-Local", local.clone()), ("Documents", docs.clone())];
        let p = planifier(&app, "m1", &windows).unwrap().unwrap();
        let rel: Vec<&str> = p.fichiers.iter().map(|f| f.relatif.as_str()).collect();
        assert_eq!(
            rel,
            vec![
                "nuage-steam/m1/Documents/my games/Jeu/SaveData/save_1",
                "nuage-steam/m1/Documents/my games/Jeu/SaveData/save_2",
                "nuage-steam/m1/remote/campaign",
                "nuage-steam/m1/remote/en-plus"
            ]
        );
        assert_eq!((p.octets, p.introuvables, p.en_attente, p.modifies), (16, 2, 1, 1), "racine 1 (manette) sautée");

        // L'abri : tout y est, vérifié ; un second abri identique n'est pas utile.
        let abris = d.path().join("Abris");
        let fin = abriter(&p, "m1", &abris, -7, "100").unwrap();
        assert_eq!(std::fs::read(fin.join("nuage-steam/m1/Documents/my games/Jeu/SaveData/save_1")).unwrap(), b"abc");
        assert!(!abris.join(".frogtend-en-cours").exists());
        assert!(deja_a_l_abri(&abris.join("-7"), "m1", &p));
        std::fs::write(app.join("remote/campaign"), b"54321").unwrap();
        let p2 = planifier(&app, "m1", &windows).unwrap().unwrap();
        assert!(!deja_a_l_abri(&abris.join("-7"), "m1", &p2), "une partie a changé");
        // La source n'a jamais été touchée.
        assert_eq!(std::fs::read(docs.join("my games/Jeu/SaveData/save_1")).unwrap(), b"abc");
    }

    #[test]
    fn un_fichier_qui_change_pendant_la_copie_ne_fait_pas_un_faux_abri() {
        let d = tempfile::tempdir().unwrap();
        let s = d.path().join("s");
        std::fs::write(&s, b"neuf").unwrap();
        let plan = Plan { fichiers: vec![AMettre { source: s, relatif: "nuage-steam/remote/s".into(), taille: 4, sha: sha_de(b"ancien") }], octets: 4, ..Default::default() };
        let abris = d.path().join("Abris");
        assert!(abriter(&plan, "m1", &abris, -7, "100").is_err());
        assert!(!abris.join("-7").exists() && !abris.join(".frogtend-en-cours").exists());
    }

    #[test]
    fn hors_de_userdata_l_identifiant_est_un_sous_dossier_par_utilisateur() {
        // Relevé réel GTA V (Firehouse 1.26, 21/10) : le profil Rockstar n'est PAS le compte Steam.
        let d = tempfile::tempdir().unwrap();
        let docs = d.path().join("Docs");
        let profils = docs.join("Rockstar Games").join("GTA V").join("Profiles");
        for p in ["A1B2C3D4", "E5F6"] {
            std::fs::create_dir_all(profils.join(p)).unwrap();
            std::fs::write(profils.join(p).join("SGTA50000"), b"partie").unwrap();
        }
        std::fs::write(profils.join("pas-un-dossier.txt"), b"x").unwrap();
        let ctx = Contexte { jeu: None, steam: Some(d.path().join("Steam")), compte: Some("42".into()), windows: vec![("Documents", docs.clone())] };
        assert_eq!(resoudre(r"%USERPROFILE%\Documents\Rockstar Games\GTA V\Profiles\<identifiant Steam>", &ctx), None, "jamais le compte Steam");
        let s = serde_json::json!([
            {"os": "Windows", "chemin": r"%USERPROFILE%\Documents\Rockstar Games\GTA V\Profiles\<identifiant Steam>", "nature": "dossier"},
            {"os": "Windows", "chemin": r"%USERPROFILE%\Documents\Rockstar Games\GTA V\Profiles\<identifiant utilisateur>\*.bak", "nature": "motif"},
        ]);
        let (l, illisibles) = emplacements(&s, &ctx);
        assert_eq!(illisibles, 0);
        let dossiers: Vec<_> = l.iter().filter(|e| e.filtre.is_none()).map(|e| e.dossier.clone()).collect();
        assert_eq!(dossiers, vec![profils.join("A1B2C3D4"), profils.join("E5F6")], "chaque profil, rien d'autre");
        // Sous userdata : toujours le compte Steam.
        assert_eq!(bases(r"<Steam>\userdata\<identifiant Steam>\271590\remote", &ctx), Some(vec![d.path().join(r"Steam\userdata\42\271590\remote")]));
        // Deux identifiants : jamais deviné.
        assert_eq!(bases(r"%USERPROFILE%\Documents\<identifiant utilisateur>\<identifiant Steam>", &ctx), None);
    }

    #[test]
    fn les_emplacements_de_firehouse_se_resolvent_sans_rien_deviner() {
        let d = tempfile::tempdir().unwrap();
        let docs = d.path().join("OneDrive").join("Documents");
        let saves = docs.join("My Games").join("XCOM2").join("XComGame").join("SaveData");
        std::fs::create_dir_all(&saves).unwrap();
        std::fs::write(saves.join("save_1"), b"partie").unwrap();
        let ctx = Contexte { jeu: Some(d.path().join("Jeu")), steam: Some(d.path().join("Steam")), compte: Some("42".into()), windows: vec![("Documents", docs.clone())] };
        // Mes documents par le registre (OneDrive chez Seb), jamais « %USERPROFILE%\Documents » écrit en dur.
        assert_eq!(resoudre(r"%USERPROFILE%\Documents\My Games\XCOM2\XComGame\SaveData\", &ctx), Some(saves.clone()));
        assert_eq!(resoudre(r"<Steam>\userdata\<identifiant Steam>\489830\remote\", &ctx), Some(d.path().join(r"Steam\userdata\42\489830\remote")));
        // Relevés réels du 21/10 : la racine perdue (GTA V), un autre système, un joker, le registre, un jeton inconnu.
        for douteux in [r"\Rockstar Games\GTA V\Profiles\", "~/Library/Application Support/x/", r"%APPDATA%\x\*.sav", r"HKEY_CURRENT_USER\Software\x", r"<inconnu>\x", r"%X%\y"] {
            assert_eq!(resoudre(douteux, &ctx), None, "{douteux}");
        }
        // La réponse réelle de XCOM 2 : seulement « Windows » (et « Steam »), dossiers existants.
        let v: serde_json::Value = serde_json::json!([
            {"os": "Windows", "chemin": r"%USERPROFILE%\Documents\My Games\XCOM2\XComGame\SaveData\", "steam_userdata": false, "nuage": null},
            {"os": "OS X", "chemin": "~/Library/Application Support/Feral Interactive/XCOM 2/", "steam_userdata": false, "nuage": null},
            {"os": "Windows", "chemin": r"\Rockstar Games\GTA V\Profiles\", "steam_userdata": false, "nuage": null},
            {"os": "Steam", "chemin": r"%USERPROFILE%\Documents\My Games\Absent\", "steam_userdata": true, "nuage": true}]);
        let (l, illisibles) = emplacements(&v, &ctx);
        assert_eq!((l.iter().map(|e| e.dossier.clone()).collect::<Vec<_>>(), illisibles), (vec![saves.clone()], 1));
        // Le nuage couvre déjà un fichier (chemin écrit avec « / » par `planifier`) : jamais deux fois.
        std::fs::write(saves.join("save_2"), b"nuage").unwrap();
        let mut plan = Plan::default();
        plan.fichiers.push(AMettre { source: PathBuf::from(format!("{}/save_2", saves.to_string_lossy())), relatif: "nuage-steam/m1/Documents/x".into(), taille: 5, sha: String::new() });
        ajouter_emplacements(&mut plan, "m1", &l);
        assert_eq!((plan.hors_nuage, plan.octets), (1, 6));
        assert!(plan.fichiers.iter().any(|f| f.relatif.starts_with("nuage-steam/m1/hors-nuage/") && f.relatif.ends_with("-SaveData/save_1")));
        // Une RACINE (le dossier du jeu entier, Mes documents…) n'est jamais un emplacement : illisible.
        std::fs::create_dir_all(d.path().join("Jeu")).unwrap();
        let racines = serde_json::json!([
            {"os": "Windows", "chemin": r"<dossier du jeu>\"},
            {"os": "Windows", "chemin": r"%USERPROFILE%\Documents\"},
            {"os": "Epic Games Store", "chemin": r"%USERPROFILE%\Documents\My Games\XCOM2\"}]);
        assert_eq!(emplacements(&racines, &ctx), (vec![], 2), "les racines refusées ; une autre boutique ignorée sans erreur");
        // Contrat 1.26 : un FICHIER, un MOTIF ; un dossier dans un autre déjà pris n'est compté qu'une fois.
        std::fs::write(saves.join("profil.ini"), b"ini").unwrap();
        let v126 = serde_json::json!([
            {"os": "Windows", "chemin": r"%USERPROFILE%\Documents\My Games\XCOM2\XComGame\SaveData\save_*", "nature": "motif"},
            {"os": "Windows", "chemin": r"%USERPROFILE%\Documents\My Games\XCOM2\XComGame\SaveData\profil.ini", "nature": "fichier"},
            {"os": "Windows", "chemin": r"%USERPROFILE%\Documents\My Games\XCOM2\"}]);
        let (l, i) = emplacements(&v126, &ctx);
        assert_eq!(i, 0);
        assert_eq!(l.len(), 1, "le motif et le fichier sont DANS le dossier XCOM2 déjà pris en entier");
        let seul = serde_json::json!([{"os": "Windows", "chemin": r"%USERPROFILE%\Documents\My Games\XCOM2\XComGame\SaveData\save_*"}]);
        let (l, _) = emplacements(&seul, &ctx);
        let mut p = Plan::default();
        ajouter_emplacements(&mut p, "m1", &l);
        assert_eq!(p.hors_nuage, 2, "save_1 et save_2, jamais profil.ini");
        assert!(repond("SAVE_12.sav", "save_*.sav") && !repond("profil.ini", "save_*") && repond("a1", "a?"));
    }

    /// RÉEL, lecture seule : les emplacements d'une réponse de Firehouse (`FROGTEND_TRICHE_JSON`, fichier gardé par
    /// `essai_mods_1_19`) résolus sur CE PC (rien n'est copié).
    #[test]
    #[ignore]
    fn essai_emplacements() {
        let v: serde_json::Value = serde_json::from_slice(&std::fs::read(std::env::var("FROGTEND_TRICHE_JSON").unwrap()).unwrap()).unwrap();
        let ctx = Contexte { jeu: None, steam: crate::boutiques::dossier_steam(), compte: None, windows: dossiers_windows() };
        let (d, illisibles) = emplacements(&v["sauvegardes"], &ctx);
        for x in &d {
            println!("dossier : {} {:?} ({} fichiers)", x.dossier.display(), x.filtre, fichiers_sans_lien(&x.dossier).map(|l| l.len()).unwrap_or(0));
        }
        println!("illisibles : {illisibles}");
    }

    #[test]
    fn un_jeu_sans_nuage_n_a_pas_de_plan() {
        let d = tempfile::tempdir().unwrap();
        assert!(planifier(d.path(), "m1", &[]).unwrap().is_none());
    }

    /// RÉEL, lecture seule : le plan d'un jeu Steam de ce PC (rien n'est copié ; aucun chemin ni compte affiché).
    /// `FROGTEND_STEAM_APP=268500 cargo test essai_nuage_steam -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn essai_nuage_steam() {
        let app = std::env::var("FROGTEND_STEAM_APP").unwrap();
        let steam = crate::boutiques::dossier_steam().expect("Steam");
        let comptes: Vec<PathBuf> = std::fs::read_dir(steam.join("userdata")).unwrap().flatten().map(|e| e.path()).collect();
        assert_eq!(comptes.len(), 1, "un seul compte attendu sur ce PC");
        let w = dossiers_windows();
        println!("dossiers de Windows : {:?}", w.iter().map(|(n, _)| *n).collect::<Vec<_>>());
        let p = planifier(&comptes[0].join(&app), "essai", &w).unwrap();
        // L'abri complet, dans un dossier temporaire (effacé à la fin) : chaque copie vérifiée par son SHA1.
        if let Some(p) = &p {
            let t = tempfile::tempdir().unwrap();
            let fin = abriter(p, "essai", t.path(), -1, "1").unwrap();
            let copies = crate::installation::fichiers_de(&fin).unwrap().len();
            println!("abri d'essai : {copies} fichiers (liste comprise), déjà à l'abri ensuite : {}", deja_a_l_abri(&t.path().join("-1"), "essai", p));
        }
        match p {
            None => println!("{app} : rien dans le nuage"),
            Some(p) => println!(
                "{app} : {} fichiers, {} octets ; introuvables {}, en ligne seulement {}, en attente {} ; dossiers : {:?}",
                p.fichiers.len(),
                p.octets,
                p.introuvables,
                p.en_ligne_seulement,
                p.en_attente,
                p.fichiers.iter().map(|f| f.relatif.split('/').nth(1).unwrap_or("")).collect::<std::collections::BTreeSet<_>>()
            ),
        }
    }
}
