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

/// Au plus ces dossiers, et cette profondeur, sous un emplacement de parties (au-delà : pas un dossier de parties).
const DOSSIERS_MAX: usize = 5_000;
const PROFONDEUR_MAX: usize = 12;

/// Pourquoi un parcours s'est arrêté.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Arret {
    /// Trop de fichiers, de dossiers, ou trop profond : ce n'est pas un dossier de parties.
    Trop,
    /// Un dossier ne se lit pas (droits, disque…).
    Illisible,
}

/// Un lien à NE JAMAIS suivre, d'après les attributs et l'étiquette de point de reparse de Windows : un lien symbolique
/// (IO_REPARSE_TAG_SYMLINK 0xA000000C), une jonction (IO_REPARSE_TAG_MOUNT_POINT 0xA0000003) et tout autre « substitut
/// de nom » (bit 0x20000000). Un fichier ou dossier de OneDrive / Cloud Files (étiquettes 0x9000xxxx) n'est PAS un lien :
/// il se parcourt (sans jamais lire un fichier « en ligne seulement »). C'est exactement la règle de la bibliothèque
/// standard (`FileType::new`, library/std/src/sys/fs/windows.rs : `is_symlink` = point de reparse ET substitut de nom),
/// qui sert donc au parcours ; cette fonction la fige dans les tests (la bibliothèque ne donne pas l'étiquette).
#[cfg(test)]
pub(crate) fn lien_selon_etiquette(attributs: u32, etiquette: u32) -> bool {
    const POINT_DE_REPARSE: u32 = 0x400;
    const SUBSTITUT_DE_NOM: u32 = 0x2000_0000;
    attributs & POINT_DE_REPARSE != 0 && etiquette & SUBSTITUT_DE_NOM != 0
}

/// Les fichiers sous `racine` (chemins relatifs en `/`), SANS jamais suivre un lien ni une jonction (un lien dans
/// `remote\` ferait lire un disque entier) ; les dossiers OneDrive se parcourent. Bornes : `REMOTE_MAX` fichiers,
/// `DOSSIERS_MAX` dossiers, `PROFONDEUR_MAX` niveaux.
pub(crate) fn parcourir(racine: &Path) -> std::result::Result<Vec<String>, Arret> {
    let mut l = Vec::new();
    let mut pile = vec![(racine.to_path_buf(), 0usize)];
    let mut dossiers = 0usize;
    while let Some((d, niveau)) = pile.pop() {
        let entrees = std::fs::read_dir(&d).map_err(|_| Arret::Illisible)?;
        for e in entrees.flatten() {
            let p = e.path();
            let Ok(m) = std::fs::symlink_metadata(&p) else { continue };
            // `is_dir` / `is_file` sont faux pour un lien ou une jonction (substitut de nom) : jamais suivis.
            if m.file_type().is_symlink() {
                continue;
            }
            if m.is_dir() {
                dossiers += 1;
                if dossiers > DOSSIERS_MAX || niveau + 1 > PROFONDEUR_MAX {
                    return Err(Arret::Trop);
                }
                pile.push((p, niveau + 1));
            } else if m.is_file() {
                if let Ok(r) = p.strip_prefix(racine) {
                    l.push(r.to_string_lossy().replace('\\', "/"));
                }
                if l.len() > REMOTE_MAX {
                    return Err(Arret::Trop);
                }
            }
        }
    }
    l.sort();
    Ok(l)
}

/// `parcourir`, avec les messages de l'abri.
pub(crate) fn fichiers_sans_lien(racine: &Path) -> Resultat<Vec<String>> {
    parcourir(racine).map_err(|a| match a {
        Arret::Trop => Erreur::Refus("Ce dossier contient trop de fichiers pour être celui des parties : rien n'en est copié.".into()),
        Arret::Illisible => Erreur::Disque("Un dossier de parties ne se lit pas sur ce PC.".into()),
    })
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
pub(crate) fn en_ligne_seulement(m: &std::fs::Metadata) -> bool {
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
pub(crate) fn sha1(f: &Path) -> std::io::Result<String> {
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
    /// Lot S2 : les emplacements hors du nuage pris dans le plan (chemin NON résolu), pour la fiche de l'abri.
    #[serde(skip)]
    pub abrites: Vec<crate::abri_parties::EmplacementAbrite>,
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
    /// La boutique du jeu (`steam`, `gog`, `epic`…) ; `None` : un jeu de Firehouse ou importé du disque.
    /// `<racine de la boutique>` n'est résolue que pour un jeu de boutique AUTRE que Steam (Firehouse, 22/11 : pour Steam
    /// il écrit `<Steam>`).
    pub boutique: Option<String>,
}

impl Contexte {
    /// Le dossier qui CONTIENT le dossier du jeu (le `<root>` de Ludusavi), pour un jeu de boutique autre que Steam ;
    /// jamais la racine d'un disque.
    fn racine_de_la_boutique(&self) -> Option<PathBuf> {
        self.boutique.as_deref().filter(|b| !b.is_empty() && *b != "steam")?;
        let p = self.jeu.as_ref()?.parent()?;
        p.components().any(|c| matches!(c, std::path::Component::Normal(_))).then(|| p.to_path_buf())
    }
}

/// La bibliothèque Steam d'un jeu rangé dans `<bibliothèque>\steamapps\common\<jeu>` ; sinon `None` (jamais deviné).
fn bibliotheque_steam(jeu: &Path) -> Option<PathBuf> {
    let common = jeu.parent()?;
    let steamapps = common.parent()?;
    let nom = |p: &Path| p.file_name().map(|n| n.to_string_lossy().to_lowercase());
    (nom(common).as_deref() == Some("common") && nom(steamapps).as_deref() == Some("steamapps")).then(|| steamapps.parent().map(Path::to_path_buf)).flatten()
}

/// Un morceau de chemin qui n'est qu'un repère entre parenthèses, crochets ou accolades (« (UserID) », « [id] »,
/// « {{p|uid}} ») : la source ne l'a pas remplacé par un jeton connu. Pur, testé.
pub(crate) fn repere_non_remplace(morceau: &str) -> bool {
    let m = morceau.trim();
    m.len() >= 3 && ((m.starts_with('(') && m.ends_with(')')) || (m.starts_with('[') && m.ends_with(']')) || (m.starts_with('{') && m.ends_with('}')))
}

/// Un emplacement de Firehouse rendu en vrai dossier de CE PC, ou `None` (jeton inconnu, racine perdue, registre,
/// joker, chemin d'un autre système, repère non remplacé). Jamais deviné : seuls les jetons connus du contrat 1.25.
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
        // Contrat 1.48 : les autres variables de Windows servies.
        ("%WINDIR%", env("SystemRoot").or_else(|| env("windir"))),
        ("%PROGRAMFILES%", env("ProgramFiles")),
        ("%TEMP%", env("TEMP")),
        // Contrat 1.48 : le NOM du dossier du jeu, et le dossier qui le contient (là où sa boutique installe ses jeux).
        ("<nom du dossier du jeu>", ctx.jeu.as_ref().and_then(|d| d.file_name()).map(|n| n.to_string_lossy().to_string())),
        ("<racine de la boutique>", ctx.racine_de_la_boutique().map(|d| d.to_string_lossy().to_string())),
        // Une bibliothèque Steam : seulement pour un jeu rangé dans `<bibliothèque>\steamapps\common\<jeu>`.
        ("<SteamLibrary>", ctx.jeu.as_ref().and_then(|d| bibliotheque_steam(d)).map(|d| d.to_string_lossy().to_string())),
        ("<dossier du jeu>", ctx.jeu.as_ref().map(|d| d.to_string_lossy().to_string())),
        ("<Steam>", ctx.steam.as_ref().map(|d| d.to_string_lossy().to_string())),
        ("<identifiant Steam>", ctx.compte.clone()),
    ];
    // Chaque occurrence d'un jeton devient d'abord une MARQUE (jamais dans un chemin) ; un jeton resté dans le texte
    // d'origine est inconnu ; les valeurs (qui peuvent porter un « % », un nom de dossier de jeu…) ne sont posées qu'à
    // la fin, donc jamais relues comme un jeton.
    let marque = |k: usize| format!("\u{1}{k}\u{1}");
    let mut r = c;
    let mut valeurs: Vec<String> = Vec::new();
    for (jeton, valeur) in jetons {
        let j = jeton.to_ascii_lowercase();
        loop {
            // En minuscules ASCII seulement : les positions restent celles du texte d'origine (accents compris).
            let bas = r.to_ascii_lowercase();
            let Some(i) = bas.find(&j) else { break };
            // Le compte Steam seulement SOUS `userdata` : ailleurs (le profil Rockstar de GTA V…), c'est l'identifiant
            // propre au jeu, inconnu (Firehouse, 21/10) — `emplacements` énumère alors les sous-dossiers.
            if jeton == "<identifiant Steam>" && !bas[..i].ends_with("\\userdata\\") {
                return None;
            }
            let v = valeur.clone()?;
            valeurs.push(v.trim_end_matches('\\').to_string());
            r = format!("{}{}{}", &r[..i], marque(valeurs.len() - 1), &r[i + jeton.len()..]);
        }
    }
    // Un jeton resté inconnu (« <jeton> », « %X% ») : jamais deviné.
    if r.contains('%') || r.contains('<') || r.contains('>') {
        return None;
    }
    // 0.220.1 (Seb 22/11, Age of Empires IV : « …\Users\(UserID)\Savegames » servi tel quel) : un morceau du chemin
    // ENTIÈREMENT entre parenthèses, crochets ou accolades est un repère que la source n'a pas remplacé (« (UserID) »,
    // « [user id] », « {{p|uid}} ») — jamais pris pour un nom de dossier. Vu AVANT de poser les valeurs des jetons, pour
    // ne jamais refuser un vrai nom de dossier du jeu.
    if r.split(['\\', '/']).any(repere_non_remplace) {
        return None;
    }
    for (k, v) in valeurs.iter().enumerate() {
        r = r.replace(&marque(k), v);
    }
    if r.contains('\u{1}') {
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
#[cfg(test)]
fn bases(base: &str, ctx: &Contexte) -> Option<Vec<PathBuf>> {
    bases_et_identifiants(base, ctx).map(|l| l.into_iter().map(|(p, _)| p).collect())
}

/// `bases`, avec pour chaque dossier le nom pris à la place du jeton d'énumération (`None` : pas d'énumération).
fn bases_et_identifiants(base: &str, ctx: &Contexte) -> Option<Vec<(PathBuf, Option<String>)>> {
    let bas = base.to_ascii_lowercase();
    // Un jeton d'énumération : `<identifiant utilisateur>`, ou `<identifiant Steam>` HORS de `<Steam>\userdata\` (là,
    // c'est le compte connu, posé par `resoudre`). Ailleurs (le profil Rockstar de GTA V, le `Users\<SteamID64>` d'Age
    // of Empires IV), on n'écrit jamais un numéro : on prend les sous-dossiers qui existent.
    let jeton = ["<identifiant utilisateur>", "<identifiant steam>"]
        .into_iter()
        .filter_map(|j| bas.find(j).map(|i| (i, j.len(), j == "<identifiant steam>")))
        .find(|(i, _, _)| !bas[..*i].ends_with("\\userdata\\"));
    let Some((i, n, steam)) = jeton else { return resoudre(base, ctx).map(|p| vec![(p, None)]) };
    // Le jeton peut être À L'INTÉRIEUR d'un nom (« Steam_<identifiant Steam> ») : le morceau devient un motif
    // préfixe + n'importe quoi + suffixe, comparé aux noms des sous-dossiers.
    let debut = base[..i].rfind('\\').map_or(0, |k| k + 1);
    let fin = base[i + n..].find('\\').map_or(base.len(), |k| i + n + k);
    let (prefixe, suffixe) = (&base[debut..i], &base[i + n..fin]);
    let parent = resoudre(base[..debut].trim_end_matches('\\'), ctx)?;
    let reste = base[fin..].trim_matches('\\');
    // Plus aucun jeton ni joker autour, ni dans la suite : un seul niveau énuméré, jamais deviné.
    if reste.contains(['<', '>', '%']) || prefixe.contains(['<', '>', '%', '*', '?']) || suffixe.contains(['<', '>', '%', '*', '?']) {
        return None;
    }
    let Ok(l) = std::fs::read_dir(&parent) else { return Some(vec![]) };
    let motif = format!("{prefixe}*{suffixe}");
    let mut trouves: Vec<(String, PathBuf)> = l
        .flatten()
        // `is_dir` est faux pour un lien ou une jonction (jamais suivis) ; un dossier OneDrive, lui, est gardé.
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| {
            let nom = e.file_name().to_string_lossy().to_string();
            (repond(&nom, &motif) && nom.chars().count() > prefixe.chars().count() + suffixe.chars().count()).then(|| {
                // La partie qui remplace le jeton (pour reconnaître le compte Steam).
                let id = nom.chars().skip(prefixe.chars().count()).take(nom.chars().count() - prefixe.chars().count() - suffixe.chars().count()).collect::<String>();
                (id, if reste.is_empty() { e.path() } else { e.path().join(reste) })
            })
        })
        .filter(|(_, p)| !p.components().any(|c| matches!(c, std::path::Component::ParentDir)))
        .collect();
    if steam {
        trouves = comptes_steam_du_profil(trouves, ctx.compte.as_deref());
    }
    let mut v: Vec<(PathBuf, Option<String>)> = trouves.into_iter().map(|(id, p)| (p, Some(id))).collect();
    if v.len() > PROFILS_MAX {
        return None;
    }
    v.sort();
    Some(v)
}

/// Les sous-dossiers d'un `<identifiant Steam>` hors de `userdata` (identifiant, dossier) gardés pour le compte Steam
/// du profil (`compte` : l'identifiant COURT de `userdata`) : jamais le dossier d'un AUTRE compte Steam — un SteamID64
/// (17 chiffres, `7656119…`) qui n'est pas le sien est écarté ; s'il en reste un qui porte son identifiant (court ou
/// SteamID64), seulement celui-là ; sinon (compte inconnu, noms qui ne sont pas des comptes), toute la liste, que la
/// fiche montre ligne par ligne.
fn comptes_steam_du_profil(trouves: Vec<(String, PathBuf)>, compte: Option<&str>) -> Vec<(String, PathBuf)> {
    let Some(court) = compte.and_then(|c| c.trim().parse::<u64>().ok()) else { return trouves };
    let long = (court + BASE_STEAMID).to_string();
    let court = court.to_string();
    let steamid64 = |s: &str| s.len() == 17 && s.starts_with("7656119") && s.bytes().all(|b| b.is_ascii_digit());
    let siens: Vec<_> = trouves.iter().filter(|(id, _)| *id == court || *id == long).cloned().collect();
    if !siens.is_empty() {
        return siens;
    }
    trouves.into_iter().filter(|(id, _)| !steamid64(id)).collect()
}

/// Un emplacement de parties résolu : un dossier (sous-dossiers compris), ou seulement ses fichiers qui répondent à
/// `filtre` (contrat 1.26 `nature` : `fichier` = un nom exact, `motif` = un nom avec * ou ?).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Emplacement {
    pub dossier: PathBuf,
    pub filtre: Option<String>,
    /// Le chemin NON résolu d'où il vient (jetons du contrat 1.25) : c'est lui qui va dans la fiche d'un abri (lot S2),
    /// jamais `dossier` (qui contient le nom de l'utilisateur).
    pub chemin: String,
    /// `dossier`, `fichier` ou `motif`.
    pub nature: String,
    /// Le nom du sous-dossier pris à la place de `<identifiant utilisateur>` / `<identifiant Steam>` (hors de
    /// `userdata`), quand il a fallu énumérer : seule son EMPREINTE va dans un abri.
    pub identifiant: Option<String>,
}

/// Un nom de fichier qui répond à un motif (`*`, `?`), sans tenir compte des majuscules (comme Windows).
pub(crate) fn repond(nom: &str, motif: &str) -> bool {
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

/// Une RACINE (dossier du jeu, le dossier qui le contient, de Steam et de ses bibliothèques, de Windows, de
/// l'utilisateur, d'AppData, le dossier temporaire, un disque) : jamais un emplacement de parties ENTIER (elle ferait
/// copier d'autres jeux, voire un disque) — comptée comme illisible ; seul un chemin qui descend plus bas est accepté.
fn est_une_racine(p: &Path, ctx: &Contexte) -> bool {
    let cle = |x: &Path| x.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_lowercase();
    let c = cle(p);
    if !p.components().any(|x| matches!(x, std::path::Component::Normal(_))) || p.components().count() <= 2 {
        return true;
    }
    let mut racines: Vec<PathBuf> = ctx.windows.iter().map(|(_, d)| d.clone()).collect();
    if let Some(j) = &ctx.jeu {
        racines.push(j.clone());
        // Le dossier qui contient le jeu (là où sa boutique range ses jeux) et, pour un jeu Steam, sa bibliothèque.
        racines.extend(j.parent().map(Path::to_path_buf));
        if let Some(lib) = bibliotheque_steam(j) {
            racines.extend([lib.join("steamapps"), lib.join("steamapps").join("common"), lib]);
        }
    }
    if let Some(s) = &ctx.steam {
        racines.extend([s.join("steamapps"), s.join("steamapps").join("common"), s.join("userdata"), s.clone()]);
    }
    let env = ["USERPROFILE", "ProgramData", "ALLUSERSPROFILE", "PUBLIC", "LOCALAPPDATA", "APPDATA", "SystemRoot", "windir", "ProgramFiles", "ProgramFiles(x86)", "TEMP", "TMP"];
    racines.extend(env.iter().filter_map(std::env::var_os).map(PathBuf::from));
    if let Some(u) = std::env::var_os("USERPROFILE").map(PathBuf::from) {
        let a = u.join("AppData");
        racines.extend([a.join("Local"), a.join("LocalLow"), a.join("Roaming"), a]);
    }
    racines.iter().any(|r| cle(r) == c)
}

/// La nature d'un emplacement (contrat 1.26 : `dossier`, `fichier`, `motif`), sinon déduite du chemin comme Firehouse
/// la décrit (un nom avec * ou ? : motif ; un nom avec un point, sans « \ » final : fichier ; sinon dossier).
pub fn nature_de(chemin: &str, nature: Option<&str>) -> String {
    let c = chemin.trim().replace('/', "\\");
    let dernier = c.trim_end_matches('\\').rsplit('\\').next().unwrap_or("");
    nature.map(String::from).unwrap_or_else(|| {
        if dernier.contains('*') || dernier.contains('?') {
            "motif".into()
        } else if !c.ends_with('\\') && dernier.contains('.') {
            "fichier".into()
        } else {
            "dossier".into()
        }
    })
}

/// Les dossiers de CE PC que vise UN emplacement de Firehouse (chemin NON résolu), qu'ils existent ou non, avec leur
/// filtre : (candidats, nombre d'illisibles — jeton inconnu, nature inconnue, nom de fichier douteux, racine).
pub fn candidats(chemin: &str, nature: Option<&str>, ctx: &Contexte) -> (Vec<Emplacement>, usize) {
    let c = chemin.trim().replace('/', "\\");
    let dernier = c.trim_end_matches('\\').rsplit('\\').next().unwrap_or("").to_string();
    let la_nature = nature_de(&c, nature);
    let (base, filtre) = match la_nature.as_str() {
        "fichier" | "motif" => (c.trim_end_matches('\\').rsplit_once('\\').map(|(b, _)| b.to_string()).unwrap_or_default(), Some(dernier)),
        "dossier" => (c.clone(), None),
        _ => return (vec![], 1),
    };
    // Un jeton d'énumération dans le NOM du fichier (« <identifiant Steam>.sav ») : n'importe quoi à sa place (un motif) ;
    // tout autre jeton dans un nom de fichier : refusé, jamais deviné.
    // Un nom fait du SEUL jeton : n'importe quel fichier du dossier, peut-être d'un autre compte — refusé.
    let seul_jeton = filtre.as_deref().is_some_and(|f| ["<identifiant utilisateur>", "<identifiant steam>"].contains(&f.trim().to_ascii_lowercase().as_str()));
    let filtre = filtre.map(|f| remplacer_sans_casse(&remplacer_sans_casse(&f, "<identifiant utilisateur>", "*"), "<identifiant steam>", "*"));
    if seul_jeton || filtre.as_deref().is_some_and(|f| f.is_empty() || f.contains(['/', '\\', ':', '<', '>', '%'])) {
        return (vec![], 1);
    }
    let Some(bases) = bases_et_identifiants(&base, ctx) else { return (vec![], 1) };
    let mut l = Vec::new();
    let mut illisibles = 0;
    for (p, identifiant) in bases {
        if est_une_racine(&p, ctx) && filtre.is_none() {
            illisibles += 1;
        } else {
            l.push(Emplacement { dossier: p, filtre: filtre.clone(), chemin: chemin.trim().to_string(), nature: la_nature.clone(), identifiant });
        }
    }
    (l, illisibles)
}

/// Remplace chaque occurrence de `jeton` (en minuscules ASCII) sans tenir compte des majuscules.
fn remplacer_sans_casse(texte: &str, jeton: &str, par: &str) -> String {
    let mut r = String::new();
    let mut reste = texte;
    while let Some(i) = reste.to_ascii_lowercase().find(jeton) {
        r.push_str(&reste[..i]);
        r.push_str(par);
        reste = &reste[i + jeton.len()..];
    }
    r.push_str(reste);
    r
}

/// Les emplacements de parties d'un jeu Steam d'après Firehouse (`sauvegardes` : seulement « Windows » et « Steam »,
/// liste ouverte : le reste est ignoré), résolus : (emplacements existants, nombre d'illisibles).
pub fn emplacements(sauvegardes: &serde_json::Value, ctx: &Contexte) -> (Vec<Emplacement>, usize) {
    let mut l: Vec<Emplacement> = Vec::new();
    let mut illisibles = 0;
    // Le même filtre que « 📁 Où sont mes parties » (lot S1) : Windows et Steam, jamais une ligne d'une autre boutique,
    // un `os` inconnu, le registre, ni un chemin de Mac ou de Linux servi en « Steam » (contrat 1.48).
    let refs = crate::emplacements_parties::references_de_secours();
    let gardes = crate::emplacements_parties::depuis_firehouse(sauvegardes, Some("steam"), &refs, None, "").unwrap_or_default();
    for s in gardes {
        let (vus, n) = candidats(&s.chemin, Some(&s.nature), ctx);
        illisibles += n;
        for e in vus {
            // Un dossier absent : pas (encore) de parties à cet endroit.
            if e.dossier.is_dir() && !l.iter().any(|x| x.dossier == e.dossier && x.filtre == e.filtre) {
                l.push(e);
            }
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
/// (le nuage le couvre) n'est pas pris deux fois. Un emplacement trop gros, illisible ou dont un fichier est verrouillé
/// ne gêne jamais les autres (compté illisible).
///
/// Lot S2 (22/11) : rangés au NOUVEAU format, `emplacements/<clé du chemin NON résolu>/…`, avec la fiche de l'abri
/// (`abri_parties`) — avant, `nuage-steam/<marque>/hors-nuage/<4 octets du SHA1 du chemin LOCAL>-<nom>/…`, sans le
/// chemin d'origine : impossibles à remettre en place (ces anciens abris restent lisibles, jamais réécrits).
/// `sel` : celui du profil (`abri_parties::sel_du_profil`), mêlé à l'empreinte d'un sous-dossier énuméré.
pub fn ajouter_emplacements(plan: &mut Plan, emplacements: &[Emplacement], sel: &str) {
    let deja: std::collections::BTreeSet<String> = plan.fichiers.iter().map(|f| crate::abri_parties::normal(&f.source)).collect();
    let p = crate::abri_parties::planifier(emplacements, &deja, sel);
    plan.en_ligne_seulement += p.en_ligne_seulement;
    for e in p.emplacements {
        if e.echec.is_some() {
            plan.emplacements_illisibles += 1;
            continue;
        }
        plan.octets += e.octets;
        plan.hors_nuage += e.fichiers.len();
        plan.fichiers.extend(e.fichiers);
        plan.abrites.push(e.abrite);
    }
    plan.fichiers.sort_by(|a, b| a.relatif.cmp(&b.relatif));
}

/// La liste rangée dans chaque abri du nuage Steam, par compte (pour ne pas refaire un abri identique).
fn liste(marque: &str) -> String {
    format!("nuage-steam/{marque}/abri.json")
}

/// Le plus récent abri de ce jeu POUR CE COMPTE contient-il déjà exactement ces fichiers (chemins et SHA1) ? Quand le
/// plan a des emplacements hors du nuage (lot S2), seulement un abri de CE profil (`profil`, sa fiche le dit) : les
/// parties AppData sont communes aux profils d'un compte Windows, mais l'abri de Léa n'est jamais celui de Seb.
pub fn deja_a_l_abri(abris_du_jeu: &Path, marque: &str, plan: &Plan, profil: &str) -> bool {
    let mut dates: Vec<(u64, PathBuf)> = std::fs::read_dir(abris_du_jeu)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().to_str().and_then(|n| n.parse::<u64>().ok()).map(|n| (n, e.path())))
        .collect();
    dates.sort_by(|a, b| b.0.cmp(&a.0));
    let hors_nuage = plan.fichiers.iter().any(|f| f.relatif.starts_with(crate::abri_parties::PREFIXE));
    let a_lui = |d: &Path| !hors_nuage || crate::abri_parties::lire_fiche(d).is_some_and(|f| f.profil == profil);
    let Some(o) = dates.iter().filter(|(_, d)| a_lui(d)).find_map(|(_, d)| std::fs::read(d.join(liste(marque))).ok()) else { return false };
    let Ok(avant) = serde_json::from_slice::<Vec<AMettre>>(&o) else { return false };
    let cle = |l: &[AMettre]| l.iter().map(|f| (f.relatif.to_lowercase(), f.sha.clone())).collect::<std::collections::BTreeSet<_>>();
    cle(&avant) == cle(&plan.fichiers)
}

/// Copie le plan dans un nouvel abri `<abris>/<id>/<quand>/`, en passant par un dossier provisoire hors des abris
/// lus (un abri à moitié fait n'est jamais pris pour une sauvegarde). Chaque copie est vérifiée par son SHA1 ; un
/// fichier qui change pendant la copie (Steam synchronise) est recopié une fois, sinon tout est refusé.
/// Lot S2 : la fiche de l'abri (`<quand>.json`, à côté) dit le profil propriétaire (`profil`, celui de la session) et
/// les emplacements hors du nuage (chemins NON résolus) ; elle est écrite avant que l'abri prenne son nom.
pub fn abriter(plan: &Plan, marque: &str, abris: &Path, id: i64, quand: &str, profil: &str) -> Resultat<PathBuf> {
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
        let fiche = crate::abri_parties::FicheAbri::nouvelle(crate::abri_parties::SORTE_NUAGE_STEAM, profil, plan.abrites.clone(), &plan.fichiers, plan.en_ligne_seulement, plan.emplacements_illisibles);
        crate::abri_parties::poser_avec_fiche(&tmp, &fin, &fiche)
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
        let fin = abriter(&p, "m1", &abris, -7, "100", "p1").unwrap();
        assert_eq!(std::fs::read(fin.join("nuage-steam/m1/Documents/my games/Jeu/SaveData/save_1")).unwrap(), b"abc");
        assert!(!abris.join(".frogtend-en-cours").exists());
        assert!(deja_a_l_abri(&abris.join("-7"), "m1", &p, "p1"));
        std::fs::write(app.join("remote/campaign"), b"54321").unwrap();
        let p2 = planifier(&app, "m1", &windows).unwrap().unwrap();
        assert!(!deja_a_l_abri(&abris.join("-7"), "m1", &p2, "p1"), "une partie a changé");
        // La source n'a jamais été touchée.
        assert_eq!(std::fs::read(docs.join("my games/Jeu/SaveData/save_1")).unwrap(), b"abc");
    }

    /// Lot S2 : un NOUVEL abri Steam range les parties hors du nuage sous `emplacements/<clé du chemin NON résolu>/` avec
    /// sa fiche (profil, chemin d'origine) ; un ANCIEN abri (`hors-nuage/`, sans chemin d'origine) reste lisible, n'est
    /// jamais réécrit, et « déjà à l'abri » le lit encore.
    #[test]
    fn le_nouvel_abri_steam_hors_nuage_garde_son_chemin_et_l_ancien_reste_tel_quel() {
        let d = tempfile::tempdir().unwrap();
        let docs = d.path().join("OneDrive").join("Documents");
        let saves = docs.join("My Games").join("XCOM2").join("XComGame").join("SaveData");
        std::fs::create_dir_all(&saves).unwrap();
        std::fs::write(saves.join("save_1"), b"partie").unwrap();
        let ctx = Contexte { jeu: None, steam: None, compte: None, windows: vec![("Documents", docs.clone())], boutique: Some("steam".into()) };
        let chemin = r"%USERPROFILE%\Documents\My Games\XCOM2\XComGame\SaveData\";
        let (l, _) = emplacements(&serde_json::json!([{"os": "Windows", "chemin": chemin, "nature": "dossier"}]), &ctx);
        let mut plan = Plan::default();
        ajouter_emplacements(&mut plan, &l, "sel");
        let abris = d.path().join("Abris");
        // L'ancien abri, d'avant le lot S2 : sa liste dit les anciens chemins.
        let ancien = abris.join("-7").join("50");
        let vieux = ancien.join("nuage-steam").join("m1").join("hors-nuage").join("abcd1234-SaveData").join("save_1");
        std::fs::create_dir_all(vieux.parent().unwrap()).unwrap();
        std::fs::write(&vieux, b"partie").unwrap();
        let liste_ancienne = serde_json::to_vec(&[AMettre { source: PathBuf::new(), relatif: "nuage-steam/m1/hors-nuage/abcd1234-SaveData/save_1".into(), taille: 6, sha: sha_de(b"partie") }]).unwrap();
        std::fs::write(ancien.join("nuage-steam").join("m1").join("abri.json"), &liste_ancienne).unwrap();
        assert!(!deja_a_l_abri(&abris.join("-7"), "m1", &plan, "profil-seb"), "les chemins ont changé de forme : un nouvel abri");
        let fin = abriter(&plan, "m1", &abris, -7, "100", "profil-seb").unwrap();
        let cle = crate::abri_parties::cle_chemin(chemin, "dossier");
        assert_eq!(std::fs::read(fin.join("emplacements").join(&cle).join("save_1")).unwrap(), b"partie");
        assert!(!fin.join("nuage-steam").join("m1").join("hors-nuage").exists());
        let f = crate::abri_parties::lire_fiche(&fin).unwrap();
        assert_eq!((f.sorte.as_str(), f.profil.as_str(), f.fichiers.len()), (crate::abri_parties::SORTE_NUAGE_STEAM, "profil-seb", 1));
        assert_eq!(f.emplacements.iter().map(|e| (e.cle.as_str(), e.chemin.as_str())).collect::<Vec<_>>(), [(cle.as_str(), chemin)]);
        assert!(!std::fs::read_to_string(abris.join("-7").join("100.json")).unwrap().contains("OneDrive"), "jamais le chemin de ce PC");
        assert!(deja_a_l_abri(&abris.join("-7"), "m1", &plan, "profil-seb"));
        // Correctif 1 (revue du 22/11) : pour Léa, l'abri de Seb ne vaut pas (elle a ses propres parties hors du nuage).
        assert!(!deja_a_l_abri(&abris.join("-7"), "m1", &plan, "profil-lea"));
        // Un plan sans emplacement hors du nuage : le compte Steam suffit, comme avant.
        let mut nuage = plan.clone();
        nuage.fichiers.retain(|f| !f.relatif.starts_with("emplacements/"));
        nuage.abrites.clear();
        let _ = abriter(&nuage, "m1", &abris, -7, "101", "profil-seb").unwrap();
        assert!(deja_a_l_abri(&abris.join("-7"), "m1", &nuage, "profil-lea"));
        // L'ancien : intact, sans fiche ajoutée, toujours listé (son chemin d'origine est inconnu).
        assert_eq!(std::fs::read(&vieux).unwrap(), b"partie");
        assert_eq!(std::fs::read(ancien.join("nuage-steam").join("m1").join("abri.json")).unwrap(), liste_ancienne);
        assert!(!abris.join("-7").join("50.json").exists());
        let vus = crate::abri_parties::lister(&[abris.join("-7")], "profil-seb", Some("m1"));
        assert_eq!(vus.iter().map(|a| (a.date.as_str(), a.a_moi, a.sans_origine)).collect::<Vec<_>>(), [("101", true, 0), ("100", true, 0), ("50", false, 1)]);
        // Correctif 7 : la fiche dit le vrai nombre d'emplacements en échec ; « complet » seulement sans aucun.
        assert!(f.complet && f.echecs == 0);
        let mut manque = plan.clone();
        manque.emplacements_illisibles = 1;
        let fin = abriter(&manque, "m1", &abris, -7, "102", "profil-seb").unwrap();
        let f = crate::abri_parties::lire_fiche(&fin).unwrap();
        assert_eq!((f.echecs, f.complet), (1, false));
    }

    #[test]
    fn un_fichier_qui_change_pendant_la_copie_ne_fait_pas_un_faux_abri() {
        let d = tempfile::tempdir().unwrap();
        let s = d.path().join("s");
        std::fs::write(&s, b"neuf").unwrap();
        let plan = Plan { fichiers: vec![AMettre { source: s, relatif: "nuage-steam/remote/s".into(), taille: 4, sha: sha_de(b"ancien") }], octets: 4, ..Default::default() };
        let abris = d.path().join("Abris");
        assert!(abriter(&plan, "m1", &abris, -7, "100", "p1").is_err());
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
        let ctx = Contexte { jeu: None, steam: Some(d.path().join("Steam")), compte: Some("42".into()), windows: vec![("Documents", docs.clone())], boutique: None };
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
        // 0.220.1 : `Profiles\<identifiant Steam>` énumère les dossiers existants (jamais un numéro écrit).
        assert_eq!(bases(r"%USERPROFILE%\Documents\Rockstar Games\GTA V\Profiles\<identifiant Steam>", &ctx), Some(vec![profils.join("A1B2C3D4"), profils.join("E5F6")]));
    }

    /// 0.220.1 (Seb 22/11, contrat 1.50) : Age of Empires IV range ses parties sous `Users\<SteamID64>\Savegames`,
    /// alors que `userdata` porte l'identifiant COURT du compte. Hors de `userdata`, on énumère ; le dossier du compte
    /// du profil (court ou SteamID64) est préféré ; un autre SteamID64 n'est JAMAIS pris (l'abri Steam y compris).
    #[test]
    fn age_of_empires_iv_trouve_le_dossier_steamid64_du_compte() {
        let d = tempfile::tempdir().unwrap();
        let docs = d.path().join("Docs");
        let users = docs.join("My Games").join("Age of Empires IV").join("Users");
        let sien = (39_734_272u64 + BASE_STEAMID).to_string(); // 76561198000000000
        assert_eq!(sien, "76561198000000000");
        for u in [sien.as_str(), "76561198111111111"] {
            std::fs::create_dir_all(users.join(u).join("Savegames")).unwrap();
            std::fs::write(users.join(u).join("Savegames").join("partie.aoe4save"), b"p").unwrap();
        }
        let chemin = r"%USERPROFILE%\Documents\My Games\Age of Empires IV\Users\<identifiant Steam>\Savegames";
        let avec = |compte: Option<&str>| Contexte { jeu: None, steam: None, compte: compte.map(String::from), windows: vec![("Documents", docs.clone())], boutique: Some("steam".into()) };
        // Le compte du profil connu (identifiant court de `userdata`) : seulement SON dossier.
        assert_eq!(bases(chemin, &avec(Some("39734272"))), Some(vec![users.join(&sien).join("Savegames")]));
        // Un compte connu qui n'a pas de dossier ici : les autres SteamID64 sont écartés (jamais le dossier d'un autre).
        assert_eq!(bases(chemin, &avec(Some("12345"))), Some(vec![]));
        // Compte inconnu : la liste, que la fiche montre ligne par ligne.
        assert_eq!(bases(chemin, &avec(None)).unwrap().len(), 2);
        // Par l'abri Steam (`emplacements`) : le même choix.
        let s = serde_json::json!([{"os": "Windows", "chemin": chemin, "nature": "dossier", "boutique": null}]);
        let (l, illisibles) = emplacements(&s, &avec(Some("39734272")));
        assert_eq!((l.iter().map(|e| e.dossier.clone()).collect::<Vec<_>>(), illisibles), (vec![users.join(&sien).join("Savegames")], 0));
        // Des noms qui ne sont pas des comptes Steam (un profil propre au jeu) restent pris quand rien ne correspond.
        assert_eq!(comptes_steam_du_profil(vec![("A1B2".into(), PathBuf::from("a")), ("76561198111111111".into(), PathBuf::from("b"))], Some("42")), vec![("A1B2".to_string(), PathBuf::from("a"))]);
    }

    /// 0.220.1 : un jeton À L'INTÉRIEUR d'un nom — « Steam_<identifiant Steam> » (dossier) et « <identifiant Steam>.sav »
    /// (fichier) — devient un motif ; un nom fait du seul jeton, ou deux jetons, sont refusés.
    #[test]
    fn un_jeton_dans_un_nom_devient_un_motif() {
        let d = tempfile::tempdir().unwrap();
        let docs = d.path().join("Docs");
        let jeu = docs.join("Jeu");
        for n in ["Steam_76561198000000000", "Steam_42", "Autre_1", "steam_"] {
            std::fs::create_dir_all(jeu.join(n)).unwrap();
        }
        std::fs::write(jeu.join("76561198000000000.sav"), b"p").unwrap();
        std::fs::write(jeu.join("notes.txt"), b"x").unwrap();
        let ctx = Contexte { jeu: None, steam: None, compte: None, windows: vec![("Documents", docs.clone())], boutique: None };
        // Dossier : préfixe + n'importe quoi (au moins un caractère) + suffixe.
        assert_eq!(bases(r"%USERPROFILE%\Documents\Jeu\Steam_<identifiant Steam>\saves", &ctx), Some(vec![jeu.join("Steam_42").join("saves"), jeu.join("Steam_76561198000000000").join("saves")]));
        assert_eq!(bases(r"%USERPROFILE%\Documents\Jeu\Steam_<identifiant Steam>_<identifiant utilisateur>", &ctx), None, "deux jetons : jamais deviné");
        // Fichier : le nom devient un motif.
        let (l, n) = candidats(r"%USERPROFILE%\Documents\Jeu\<identifiant Steam>.sav", Some("fichier"), &ctx);
        assert_eq!((l.iter().map(|e| (e.dossier.clone(), e.filtre.clone(), e.nature.as_str(), e.identifiant.clone())).collect::<Vec<_>>(), n), (vec![(jeu.clone(), Some("*.sav".to_string()), "fichier", None)], 0));
        assert_eq!(l[0].chemin, r"%USERPROFILE%\Documents\Jeu\<identifiant Steam>.sav", "le chemin NON résolu suit l'emplacement");
        assert!(repond("76561198000000000.sav", "*.sav") && !repond("notes.txt", "*.sav"));
        // Le seul jeton comme nom de fichier, ou un autre jeton dans le nom : refusé.
        assert_eq!(candidats(r"%USERPROFILE%\Documents\Jeu\<identifiant Steam>", Some("fichier"), &ctx).1, 1);
        assert_eq!(candidats(r"%USERPROFILE%\Documents\Jeu\<nom du jeu GameMaker>.sav", Some("fichier"), &ctx).1, 1);
        // Avec le compte connu, le dossier qui porte son SteamID64 est préféré.
        let c = Contexte { compte: Some("39734272".into()), ..ctx };
        assert_eq!(bases(r"%USERPROFILE%\Documents\Jeu\Steam_<identifiant Steam>", &c), Some(vec![jeu.join("Steam_76561198000000000")]));
    }

    #[test]
    fn les_emplacements_de_firehouse_se_resolvent_sans_rien_deviner() {
        let d = tempfile::tempdir().unwrap();
        let docs = d.path().join("OneDrive").join("Documents");
        let saves = docs.join("My Games").join("XCOM2").join("XComGame").join("SaveData");
        std::fs::create_dir_all(&saves).unwrap();
        std::fs::write(saves.join("save_1"), b"partie").unwrap();
        let ctx = Contexte { jeu: Some(d.path().join("Jeu")), steam: Some(d.path().join("Steam")), compte: Some("42".into()), windows: vec![("Documents", docs.clone())], boutique: None };
        // Mes documents par le registre (OneDrive chez Seb), jamais « %USERPROFILE%\Documents » écrit en dur.
        assert_eq!(resoudre(r"%USERPROFILE%\Documents\My Games\XCOM2\XComGame\SaveData\", &ctx), Some(saves.clone()));
        assert_eq!(resoudre(r"<Steam>\userdata\<identifiant Steam>\489830\remote\", &ctx), Some(d.path().join(r"Steam\userdata\42\489830\remote")));
        // Relevés réels du 21/10 : la racine perdue (GTA V), un autre système, un joker, le registre, un jeton inconnu.
        for douteux in [r"\Rockstar Games\GTA V\Profiles\", "~/Library/Application Support/x/", r"%APPDATA%\x\*.sav", r"HKEY_CURRENT_USER\Software\x", r"<inconnu>\x", r"%X%\y"] {
            assert_eq!(resoudre(douteux, &ctx), None, "{douteux}");
        }
        // 0.220.1 (Seb 22/11, Age of Empires IV) : un repère que la source n'a pas remplacé n'est jamais un dossier.
        for repere in [r"%USERPROFILE%\Documents\My Games\Age of Empires IV\Users\(UserID)\Savegames", r"%APPDATA%\x\[user id]\saves", r"%APPDATA%\x\{{p|uid}}\saves"] {
            assert_eq!(resoudre(repere, &ctx), None, "{repere}");
        }
        assert!(!repere_non_remplace("Program Files (x86)") && !repere_non_remplace("()") && !repere_non_remplace("Jeu (2019)"));
        assert!(repere_non_remplace("(UserID)") && repere_non_remplace(" [id] ") && repere_non_remplace("{{p|uid}}"));
        // Un vrai nom du dossier du jeu entre parenthèses n'est pas refusé (vu avant de poser les jetons).
        let ctx_jeu = Contexte { jeu: Some(d.path().join("(Jeu)")), steam: None, compte: None, windows: vec![], boutique: None };
        assert_eq!(resoudre(r"<dossier du jeu>\saves", &ctx_jeu), Some(d.path().join(r"(Jeu)\saves")));
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
        ajouter_emplacements(&mut plan, &l, "sel");
        assert_eq!((plan.hors_nuage, plan.octets), (1, 6));
        // Lot S2 : au nouveau format, sous la clé du chemin NON résolu (jamais celui de ce PC), avec son emplacement.
        let cle = crate::abri_parties::cle_chemin(r"%USERPROFILE%\Documents\My Games\XCOM2\XComGame\SaveData\", "dossier");
        assert!(plan.fichiers.iter().any(|f| f.relatif == format!("emplacements/{cle}/save_1")), "{:?}", plan.fichiers);
        assert!(!plan.fichiers.iter().any(|f| f.relatif.contains("hors-nuage")));
        assert_eq!(plan.abrites.iter().map(|a| (a.cle.as_str(), a.chemin.as_str())).collect::<Vec<_>>(), [(cle.as_str(), r"%USERPROFILE%\Documents\My Games\XCOM2\XComGame\SaveData\")]);
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
        ajouter_emplacements(&mut p, &l, "sel");
        assert_eq!(p.hors_nuage, 2, "save_1 et save_2, jamais profil.ini");
        assert!(repond("SAVE_12.sav", "save_*.sav") && !repond("profil.ini", "save_*") && repond("a1", "a?"));
    }

    /// RÉEL, lecture seule : les emplacements d'une réponse de Firehouse (`FROGTEND_TRICHE_JSON`, fichier gardé par
    /// `essai_mods_1_19`) résolus sur CE PC (rien n'est copié).
    #[test]
    #[ignore]
    fn essai_emplacements() {
        let v: serde_json::Value = serde_json::from_slice(&std::fs::read(std::env::var("FROGTEND_TRICHE_JSON").unwrap()).unwrap()).unwrap();
        let ctx = Contexte { jeu: None, steam: crate::boutiques::dossier_steam(), compte: None, windows: dossiers_windows(), boutique: Some("steam".into()) };
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

    #[test]
    #[ignore]
    fn essai_onedrive_en_lecture_seule() {
        // Revue des experts (22/11) : un dossier OneDrive (sous-dossiers « Directory, ReparsePoint ») se parcourt ; rien
        // n'est lu ni copié, aucun fichier « en ligne seulement » n'est ouvert (seuls des noms et des tailles).
        // `FROGTEND_ONEDRIVE` : le dossier à regarder (par exemple C:\Users\<nom>\OneDrive\Documents).
        let d = PathBuf::from(std::env::var("FROGTEND_ONEDRIVE").unwrap());
        let r = parcourir(&d);
        match &r {
            Ok(l) => println!("{} fichier(s) vus sous le dossier OneDrive (aucun ouvert)", l.len()),
            Err(a) => println!("arrêt : {a:?}"),
        }
        for e in std::fs::read_dir(&d).unwrap().flatten().take(10) {
            let m = std::fs::symlink_metadata(e.path()).unwrap();
            println!("dossier={} lien={}", m.is_dir(), m.file_type().is_symlink());
        }
    }

    /// Revue des experts (22/11) : seuls les liens et les jonctions sont écartés (substitut de nom), jamais un dossier
    /// ou un fichier OneDrive / Cloud Files.
    #[test]
    fn seuls_les_liens_et_les_jonctions_sont_ecartes() {
        const REPARSE: u32 = 0x400;
        const DOSSIER: u32 = 0x10;
        assert!(lien_selon_etiquette(REPARSE | DOSSIER, 0xA000_000C), "lien symbolique");
        assert!(lien_selon_etiquette(REPARSE | DOSSIER, 0xA000_0003), "jonction");
        assert!(lien_selon_etiquette(REPARSE, 0xA000_000C), "lien vers un fichier");
        for onedrive in [0x9000_001A, 0x9000_101A, 0x9000_601A, 0x9000_A01A] {
            assert!(!lien_selon_etiquette(REPARSE | DOSSIER, onedrive), "OneDrive / Cloud Files {onedrive:#x}");
        }
        assert!(!lien_selon_etiquette(DOSSIER, 0), "un dossier ordinaire");
        assert!(!lien_selon_etiquette(DOSSIER, 0xA000_000C), "sans l'attribut, l'étiquette ne compte pas");
    }

    /// Une jonction sous un emplacement n'est jamais suivie (l'abri Steam reste sûr), un vrai sous-dossier, si.
    #[test]
    fn une_jonction_n_est_jamais_suivie() {
        let d = tempfile::tempdir().unwrap();
        let racine = d.path().join("Parties");
        let ailleurs = d.path().join("Ailleurs");
        std::fs::create_dir_all(racine.join("vrai")).unwrap();
        std::fs::create_dir_all(&ailleurs).unwrap();
        std::fs::write(racine.join("vrai").join("a.sav"), b"a").unwrap();
        std::fs::write(ailleurs.join("secret.txt"), b"x").unwrap();
        let ok = std::process::Command::new("cmd").args(["/C", "mklink", "/J"]).arg(racine.join("lien")).arg(&ailleurs).output().is_ok_and(|o| o.status.success());
        if !ok {
            eprintln!("jonction impossible à créer ici : test sauté");
            return;
        }
        assert_eq!(parcourir(&racine).unwrap(), ["vrai/a.sav"]);
        // Et `<identifiant utilisateur>` n'énumère pas la jonction.
        let ctx = Contexte { jeu: None, steam: None, compte: None, windows: vec![("Documents", d.path().to_path_buf())], boutique: None };
        let b = bases(r"%USERPROFILE%\Documents\Parties\<identifiant utilisateur>", &ctx).unwrap();
        assert_eq!(b, [racine.join("vrai")]);
    }

    /// Revue des experts (22/11), correctif 11 : l'abri Steam prend le même filtre que le S1 — jamais une ligne d'une
    /// autre boutique, un `os` inconnu, le registre, ni un chemin de Mac ou de Linux servi en « Steam ».
    #[test]
    fn l_abri_steam_ecarte_les_lignes_d_une_autre_boutique() {
        let d = tempfile::tempdir().unwrap();
        let docs = d.path().join("Docs");
        for s in ["Xbox", "Ancien", "Bon", ".config"] {
            std::fs::create_dir_all(docs.join("Jeu").join(s)).unwrap();
        }
        let ctx = Contexte { jeu: None, steam: None, compte: None, windows: vec![("Documents", docs.clone())], boutique: Some("steam".into()) };
        let s = serde_json::json!([
            {"os": "Microsoft Store", "chemin": r"%USERPROFILE%\Documents\Jeu\Xbox", "nature": "dossier", "boutique": "microsoft"},
            {"os": "Windows", "chemin": r"%USERPROFILE%\Documents\Jeu\Xbox", "nature": "dossier", "boutique": "microsoft"},
            {"os": "Windows Store", "os_inconnu": true, "chemin": r"%USERPROFILE%\Documents\Jeu\Ancien", "nature": "dossier"},
            {"os": "Windows", "chemin": r"HKEY_CURRENT_USER\Software\Jeu", "nature": "registre"},
            {"os": "Steam", "chemin": r"%USERPROFILE%\.config\Jeu", "nature": "dossier", "boutique": "steam"},
            {"os": "Steam", "chemin": r"%USERPROFILE%\Documents\Jeu\Bon", "nature": "dossier", "boutique": "steam"}
        ]);
        let (l, illisibles) = emplacements(&s, &ctx);
        assert_eq!(l.iter().map(|e| e.dossier.clone()).collect::<Vec<_>>(), [docs.join("Jeu").join("Bon")]);
        assert_eq!(illisibles, 0, "écartées, pas « illisibles »");
    }

    /// Les bornes du parcours : trop de dossiers, ou trop profond.
    #[test]
    fn le_parcours_est_borne() {
        let d = tempfile::tempdir().unwrap();
        let large = d.path().join("Large");
        for i in 0..=DOSSIERS_MAX {
            std::fs::create_dir_all(large.join(format!("d{i}"))).unwrap();
        }
        assert_eq!(parcourir(&large), Err(Arret::Trop));
        let mut p = d.path().join("Profond");
        for i in 0..PROFONDEUR_MAX {
            p = p.join(format!("n{i}"));
        }
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(p.join("x.sav"), b"x").unwrap();
        assert_eq!(parcourir(&d.path().join("Profond")).unwrap().len(), 1, "12 niveaux : permis");
        std::fs::create_dir_all(p.join("trop")).unwrap();
        assert_eq!(parcourir(&d.path().join("Profond")), Err(Arret::Trop));
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
            let fin = abriter(p, "essai", t.path(), -1, "1", "essai").unwrap();
            let copies = crate::installation::fichiers_de(&fin).unwrap().len();
            println!("abri d'essai : {copies} fichiers (liste comprise), déjà à l'abri ensuite : {}", deja_a_l_abri(&t.path().join("-1"), "essai", p, "essai"));
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
