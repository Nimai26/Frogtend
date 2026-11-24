//! Lot S6 (24/11) : reconnaître le MOTEUR d'un jeu PC sur le disque, et LIRE le nom variable de ses emplacements de
//! parties, pour les mettre à l'abri et les reprendre quand Firehouse ne les connaît pas.
//!
//! Principe (décisions de Seb, 24/11 ; recherche `docs/MOTEURS-PARTIES.md`) :
//! - les RÈGLES viennent de Firehouse (`/sauvegardes/references ▸ moteurs` : `reconnaitre`, `sauvegardes`, `nom_a_lire`,
//!   `certitude`) ; Frogtend n'en recopie AUCUNE. Il sait seulement reconnaître un motif de fichier et LIRE un nom ;
//! - reconnaître : les motifs `reconnaitre` appliqués au dossier du jeu, au PREMIER niveau ou sur le chemin relatif
//!   donné (`www/js/rpg_core.js`, `*_Data/…`, `*/Content/Paks/*.pak`) ; jamais un disque parcouru, un plafond d'entrées
//!   lues, aucun lien ni jonction suivi ;
//! - lire : jamais exécuter le jeu ni « dé-pickler » (Ren'Py) ; une lecture plafonnée, une recherche d'octets ;
//! - appliquer : seulement avec DEUX preuves (le marqueur ET le nom lu) ; un gabarit SANS nom à lire (RPG Maker…) ne
//!   vaut que DANS le dossier du jeu. Une valeur lue est refusée si elle est vide, porte `\`, `/`, `..`, un caractère
//!   interdit de Windows, ou un nom commun (`%APPDATA%\RenPy\tokens`). Le registre (`HKEY…`, PlayerPrefs) : ignoré ;
//! - jamais au journal : ni un chemin, ni un nom lu (le numéro du jeu seulement, côté appelant).

use crate::emplacements_parties::EmplacementPartie;
use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Au plus ce nombre d'entrées de dossier lues pour reconnaître un moteur (tous motifs confondus).
pub const ENTREES_MAX: usize = 20_000;
/// Au plus ce nombre d'entrées lues dans UN dossier.
const ENTREES_PAR_DOSSIER: usize = 4_096;
/// Un motif de reconnaissance a au plus ce nombre de niveaux.
const NIVEAUX_MAX: usize = 4;
/// `game/options.rpy` lu jusqu'à cette taille.
const RPY_MAX: u64 = 1 << 20;
/// Un `.rpyc` plus gros n'est pas lu ; décompressé, au plus cette taille.
const RPYC_MAX: u64 = 16 << 20;
const RPYC_DEGONFLE_MAX: usize = 64 << 20;
/// Au plus ce nombre d'octets LUS et DÉCOMPRESSÉS pour un jeu Ren'Py, tous fichiers confondus (au-delà : inconnu).
pub const RENPY_BUDGET: usize = 64 << 20;
/// Au plus ce nombre de `.rpyc` lus dans `game/`.
const RPYC_FICHIERS_MAX: usize = 64;
/// `app.info` lu jusqu'à cette taille.
const APP_INFO_MAX: u64 = 4_096;

/// Les jetons de nom que Frogtend sait LIRE (en minuscules, apostrophe droite). Un autre jeton (`<projet Godot>`,
/// `<nom du jeu GameMaker>`…) : le gabarit n'est pas appliqué.
const JETON_RENPY: &str = "<save_directory ren'py>";
const JETON_SOCIETE_UNITY: &str = "<société unity>";
const JETON_PRODUIT_UNITY: &str = "<produit unity>";
const JETON_PROJET_UNREAL: &str = "<projet unreal>";
/// Le seul jeton standard (contrat 1.25) admis dans un gabarit de moteur.
const JETON_JEU: &str = "<dossier du jeu>";

/// Une règle de moteur, telle que Firehouse la sert.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RegleMoteur {
    /// La clé de Firehouse (`renpy`, `unity`…).
    pub cle: String,
    /// Le nom à montrer (« Unity »).
    pub nom: String,
    pub reconnaitre: Vec<String>,
    /// (chemin avec jetons, nature).
    pub sauvegardes: Vec<(String, String)>,
    /// Ce que Firehouse dit du nom à lire (pour information : c'est le JETON qui décide — un jeton sans lecteur dans
    /// Frogtend, comme `<nom du jeu GameMaker>`, fait écarter son gabarit).
    pub nom_a_lire: Option<String>,
    pub certitude: Option<String>,
}

/// Les règles des moteurs d'une réponse de `/sauvegardes/references` (`moteurs`, un objet par clé) ; seulement les
/// lignes Windows qui ne sont pas du registre. Vide si Firehouse ne les sert pas (rien n'est deviné).
pub fn regles(refs: &Value) -> Vec<RegleMoteur> {
    let Some(m) = refs["moteurs"].as_object() else { return vec![] };
    let texte = |v: &Value| v.as_str().map(str::trim).filter(|t| !t.is_empty()).map(String::from);
    let mut l: Vec<RegleMoteur> = m
        .iter()
        .map(|(cle, v)| RegleMoteur {
            cle: cle.clone(),
            nom: texte(&v["nom"]).unwrap_or_else(|| cle.clone()),
            reconnaitre: v["reconnaitre"].as_array().into_iter().flatten().filter_map(texte).collect(),
            sauvegardes: v["sauvegardes"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|s| {
                    // Une ligne objet (forme des références) ; un simple texte est admis par prudence (nature déduite du
                    // chemin, système inconnu donc gardé comme Windows).
                    let (chemin, nature, os) = match s {
                        Value::String(c) => (c.trim().to_string(), None, None),
                        _ => (texte(&s["chemin"])?, texte(&s["nature"]), texte(&s["os"])),
                    };
                    if os.as_deref().is_some_and(|o| o != "Windows") || chemin.to_uppercase().starts_with("HKEY") || chemin.is_empty() {
                        return None;
                    }
                    let nature = crate::nuage_steam::nature_de(&chemin, nature.as_deref());
                    ["dossier", "fichier", "motif"].contains(&nature.as_str()).then_some((chemin, nature))
                })
                .collect(),
            nom_a_lire: texte(&v["nom_a_lire"]),
            certitude: texte(&v["certitude"]),
        })
        .filter(|r| !r.reconnaitre.is_empty())
        .collect();
    l.sort_by(|a, b| a.cle.cmp(&b.cle));
    l
}

/// Le budget des entrées lues.
struct Budget(usize);

impl Budget {
    fn prendre(&mut self) -> bool {
        if self.0 == 0 {
            return false;
        }
        self.0 -= 1;
        true
    }
}

/// Un motif `reconnaitre` (`renpy/`, `*.rpyb`, `www/js/rpg_core.js`, `*/Content/Paks/*.pak`) trouvé dans le dossier du
/// jeu : le NOM de l'entrée trouvée (dernier niveau ; jamais le chemin). Un `/` final : un dossier ; sinon un fichier.
fn trouver(racine: &Path, motif: &str, budget: &mut Budget) -> Option<String> {
    let m = motif.trim().replace('\\', "/");
    let dossier = m.ends_with('/');
    let segs: Vec<&str> = m.trim_matches('/').split('/').collect();
    if segs.is_empty() || segs.len() > NIVEAUX_MAX || m.starts_with('/') || segs.iter().any(|s| s.is_empty() || *s == "." || *s == ".." || s.contains(':')) {
        return None;
    }
    descendre(racine, &segs, dossier, budget)
}

fn descendre(dir: &Path, segs: &[&str], dossier: bool, budget: &mut Budget) -> Option<String> {
    let (seg, reste) = segs.split_first()?;
    let convient = |t: &std::fs::FileType, dernier: bool| if dernier { if dossier { t.is_dir() } else { t.is_file() } } else { t.is_dir() };
    // Sans joker : un seul nom regardé (pas de liste du dossier).
    if !seg.contains(['*', '?']) {
        if !budget.prendre() {
            return None;
        }
        let p = dir.join(seg);
        let t = std::fs::symlink_metadata(&p).ok()?.file_type();
        if !convient(&t, reste.is_empty()) {
            return None;
        }
        return if reste.is_empty() { Some(seg.to_string()) } else { descendre(&p, reste, dossier, budget) };
    }
    let l = std::fs::read_dir(dir).ok()?;
    for (k, e) in l.flatten().enumerate() {
        if k >= ENTREES_PAR_DOSSIER || !budget.prendre() {
            return None;
        }
        let nom = e.file_name().to_string_lossy().to_string();
        if !crate::nuage_steam::repond(&nom, seg) {
            continue;
        }
        // `file_type` d'une entrée ne suit jamais un lien ni une jonction (ni dossier ni fichier pour eux).
        let Ok(t) = e.file_type() else { continue };
        if !convient(&t, reste.is_empty()) {
            continue;
        }
        if reste.is_empty() {
            return Some(nom);
        }
        if let Some(x) = descendre(&e.path(), reste, dossier, budget) {
            return Some(x);
        }
    }
    None
}

/// Une valeur lue peut-elle devenir UN nom de dossier de Windows ? Refusée : vide, trop longue, avec `\`, `/`, `..`, un
/// caractère interdit (`: * ? " < > |`, contrôle), `%` (lu comme une variable), une espace ou un point au bord, un nom
/// réservé (`CON`, `NUL`, `COM1`…).
pub fn valeur_sure(v: &str) -> bool {
    let reserve = {
        let base = v.split('.').next().unwrap_or("").to_ascii_uppercase();
        ["CON", "PRN", "AUX", "NUL"].contains(&base.as_str()) || ((base.starts_with("COM") || base.starts_with("LPT")) && base.len() == 4 && base.as_bytes()[3].is_ascii_digit())
    };
    !v.is_empty()
        && v.chars().count() <= 200
        && !v.contains("..")
        && !v.chars().any(|c| c.is_control() || ['\\', '/', ':', '*', '?', '"', '<', '>', '|', '%'].contains(&c))
        && !v.starts_with([' ', '.'])
        && !v.ends_with([' ', '.'])
        && !reserve
}

/// Un nom lu, ou pourquoi il ne l'est pas.
#[derive(Debug, Clone, PartialEq)]
pub enum NomLu {
    /// La valeur, et le NOM du fichier où elle a été lue.
    Valeur(String, String),
    /// Le jeu dit lui-même « aucun » (`config.save_directory = None`) : le gabarit ne s'applique pas.
    Aucun,
    /// Rien trouvé, ou des valeurs qui se contredisent : jamais deviné.
    Inconnu,
}

/// Lit au plus `max` octets d'un fichier ordinaire (jamais un lien) ; `None` s'il est plus gros.
fn lire_plafonne(p: &Path, max: u64) -> Option<Vec<u8>> {
    let m = std::fs::symlink_metadata(p).ok()?;
    if !m.is_file() || m.len() > max {
        return None;
    }
    let mut o = Vec::new();
    std::fs::File::open(p).ok()?.take(max + 1).read_to_end(&mut o).ok()?;
    (o.len() as u64 <= max).then_some(o)
}

/// `config.save_directory` dans un `.rpy` en clair : `define config.save_directory = "…"` ou `config.save_directory =
/// …` (`'…'`, `u"…"`, `r"…"`, `None`), lignes commentées ignorées. Plusieurs valeurs différentes : `Inconnu`.
pub fn save_directory_rpy(texte: &str, fichier: &str) -> NomLu {
    let mut vus: Vec<Option<String>> = Vec::new();
    for ligne in texte.lines() {
        let l = ligne.trim_start();
        if l.starts_with('#') {
            continue;
        }
        let Some(i) = l.find("config.save_directory") else { continue };
        // Le nom entier (pas `config.save_directory_x`), précédé de `define ` ou en début d'instruction.
        let avant = l[..i].trim();
        if !(avant.is_empty() || avant == "define" || avant.starts_with("define ")) {
            continue;
        }
        let apres = l[i + "config.save_directory".len()..].trim_start();
        let Some(valeur) = apres.strip_prefix('=').filter(|r| !r.starts_with('=')) else { continue };
        let valeur = valeur.trim_start();
        if valeur.starts_with("None") && !valeur[4..].starts_with(|c: char| c.is_alphanumeric() || c == '_') {
            vus.push(None);
            continue;
        }
        match litteral(valeur.as_bytes()) {
            Some(v) => vus.push(Some(v)),
            None => return NomLu::Inconnu,
        }
    }
    conclure(vus, fichier)
}

/// Une chaîne littérale Python au début de `o` (préfixe `u`, `r`, `b` admis ; sans échappement), en UTF-8.
fn litteral(o: &[u8]) -> Option<String> {
    let mut k = 0;
    while k < 2 && o.get(k).is_some_and(|c| matches!(c, b'u' | b'U' | b'r' | b'R')) {
        k += 1;
    }
    let q = *o.get(k)?;
    if q != b'"' && q != b'\'' {
        return None;
    }
    let fin = o[k + 1..].iter().take(256).position(|c| *c == q)?;
    let s = std::str::from_utf8(&o[k + 1..k + 1 + fin]).ok()?;
    (!s.contains('\\')).then(|| s.to_string())
}

fn conclure(vus: Vec<Option<String>>, fichier: &str) -> NomLu {
    let mut l = vus;
    l.dedup();
    l.sort();
    l.dedup();
    match l.as_slice() {
        [Some(v)] => NomLu::Valeur(v.clone(), fichier.to_string()),
        [None] => NomLu::Aucun,
        _ => NomLu::Inconnu,
    }
}

/// Les blocs zlib d'un `.rpyc` : format 2 (`RENPY RPC2`, puis des entrées de 12 octets `(slot, début, longueur)` en
/// petit-boutiste, terminées par `slot == 0`), sinon le fichier entier (format 1).
fn blocs_rpyc(o: &[u8]) -> Vec<&[u8]> {
    if !o.starts_with(b"RENPY RPC2") {
        return vec![o];
    }
    let mut l = Vec::new();
    let mut p = 10;
    for _ in 0..16 {
        let Some(t) = o.get(p..p + 12) else { break };
        let n = |i: usize| u32::from_le_bytes([t[i], t[i + 1], t[i + 2], t[i + 3]]) as usize;
        let (slot, debut, longueur) = (n(0), n(4), n(8));
        if slot == 0 {
            break;
        }
        if let Some(b) = debut.checked_add(longueur).and_then(|f| o.get(debut..f)) {
            l.push(b);
        }
        p += 12;
    }
    l
}

/// Un bloc zlib décompressé, ou pourquoi non.
#[derive(Debug, PartialEq)]
enum Degonfle {
    Octets(Vec<u8>),
    /// Pas du zlib (`.rpyc` « obscurci ») : bloc ignoré.
    PasZlib,
    /// Plus que le budget : on s'arrête (inconnu, jamais deviné).
    Trop,
}

/// Décompresse un bloc zlib, au plus `max` octets.
fn degonfler(b: &[u8], max: usize) -> Degonfle {
    let mut o = Vec::new();
    if flate2::read::ZlibDecoder::new(b).take(max as u64 + 1).read_to_end(&mut o).is_err() {
        return Degonfle::PasZlib;
    }
    if o.len() > max {
        Degonfle::Trop
    } else {
        Degonfle::Octets(o)
    }
}

/// `save_directory` dans un `.rpyc` : les octets décompressés, cherchés (JAMAIS dé-picklés) ; la chaîne littérale qui
/// suit (la source de l'expression du `define`, rangée en texte), ou `None` écrit en clair.
pub fn save_directory_rpyc(o: &[u8], fichier: &str) -> NomLu {
    let mut budget = RPYC_DEGONFLE_MAX;
    save_directory_rpyc_dans(o, fichier, &mut budget).unwrap_or(NomLu::Inconnu)
}

/// Comme `save_directory_rpyc`, en décomptant les octets décompressés de `budget` ; `None` : budget dépassé.
fn save_directory_rpyc_dans(o: &[u8], fichier: &str, budget: &mut usize) -> Option<NomLu> {
    let mut vus: Vec<Option<String>> = Vec::new();
    for b in blocs_rpyc(o) {
        let d = match degonfler(b, (*budget).min(RPYC_DEGONFLE_MAX)) {
            Degonfle::Octets(d) => d,
            Degonfle::PasZlib => continue,
            Degonfle::Trop => return None,
        };
        *budget -= d.len();
        let cle = b"save_directory";
        let mut i = 0;
        while let Some(k) = d[i..].windows(cle.len()).position(|w| w == cle) {
            let debut = i + k + cle.len();
            i = debut;
            // Pas `save_directory_x` ni `xsave_directory`.
            if d.get(debut).is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_') || (debut > cle.len() && d.get(debut - cle.len() - 1).is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')) {
                continue;
            }
            let fenetre = &d[debut..d.len().min(debut + 512)];
            let quote = fenetre.iter().position(|c| *c == b'"' || *c == b'\'');
            let none = fenetre.windows(4).position(|w| w == b"None");
            match (quote, none) {
                (Some(q), n) if n.is_none_or(|n| q < n) => {
                    // Le littéral commence au préfixe éventuel (`u"…"`).
                    let s = if q > 0 && matches!(fenetre[q - 1], b'u' | b'r' | b'U' | b'R') { q - 1 } else { q };
                    if let Some(v) = litteral(&fenetre[s..]) {
                        vus.push(Some(v));
                    }
                }
                (_, Some(_)) => vus.push(None),
                _ => {}
            }
        }
    }
    if vus.is_empty() {
        return Some(NomLu::Inconnu);
    }
    Some(conclure(vus, fichier))
}

/// `<save_directory Ren'Py>` : d'abord `game/options.rpy` en clair, puis `game/options.rpyc` — le premier qui répond
/// décide ; sinon les autres `.rpyc` de `game/` (premier niveau, plafonnés). Les fichiers en vrac seulement (les
/// archives `.rpa` ne sont pas ouvertes). Au plus `RENPY_BUDGET` octets lus et décompressés en tout : au-delà, inconnu.
pub fn lire_renpy(jeu: &Path) -> NomLu {
    lire_renpy_avec(jeu, RENPY_BUDGET)
}

fn lire_renpy_avec(jeu: &Path, budget: usize) -> NomLu {
    let game = jeu.join("game");
    let mut reste = budget;
    // Lit un fichier en le décomptant ; `Err(())` : le budget n'y suffit pas.
    let lire = |f: &Path, max: u64, reste: &mut usize| -> Result<Option<Vec<u8>>, ()> {
        let Ok(m) = std::fs::symlink_metadata(f) else { return Ok(None) };
        if !m.is_file() || m.len() > max {
            return Ok(None);
        }
        if m.len() > *reste as u64 {
            return Err(());
        }
        let o = lire_plafonne(f, max);
        if let Some(o) = &o {
            *reste = reste.saturating_sub(o.len());
        }
        Ok(o)
    };
    match lire(&game.join("options.rpy"), RPY_MAX, &mut reste) {
        Err(()) => return NomLu::Inconnu,
        Ok(Some(o)) => {
            let r = save_directory_rpy(&String::from_utf8_lossy(&o), "options.rpy");
            if r != NomLu::Inconnu {
                return r;
            }
        }
        Ok(None) => {}
    }
    let un = |f: &Path, reste: &mut usize| -> Option<NomLu> {
        let nom = f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        match lire(f, RPYC_MAX, reste) {
            Err(()) => None,
            Ok(None) => Some(NomLu::Inconnu),
            Ok(Some(o)) => save_directory_rpyc_dans(&o, &nom, reste),
        }
    };
    match un(&game.join("options.rpyc"), &mut reste) {
        None => return NomLu::Inconnu,
        Some(NomLu::Inconnu) => {}
        Some(r) => return r,
    }
    let mut fichiers: Vec<PathBuf> = Vec::new();
    if let Ok(l) = std::fs::read_dir(&game) {
        fichiers = l
            .flatten()
            .take(ENTREES_PAR_DOSSIER)
            .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("rpyc")) && !p.file_name().is_some_and(|n| n.eq_ignore_ascii_case("options.rpyc")))
            .collect();
        fichiers.sort();
        fichiers.truncate(RPYC_FICHIERS_MAX);
    }
    let mut vus: Vec<NomLu> = Vec::new();
    for f in fichiers {
        match un(&f, &mut reste) {
            None => return NomLu::Inconnu,
            Some(NomLu::Inconnu) => {}
            Some(r) => vus.push(r),
        }
    }
    // Plusieurs fichiers : une seule valeur admise (le premier nom de fichier qui la donne).
    let valeurs: Vec<Option<&str>> = {
        let mut v: Vec<Option<&str>> = vus.iter().map(|r| if let NomLu::Valeur(x, _) = r { Some(x.as_str()) } else { None }).collect();
        v.sort();
        v.dedup();
        v
    };
    match valeurs.as_slice() {
        [_] => vus.into_iter().next().unwrap_or(NomLu::Inconnu),
        _ => NomLu::Inconnu,
    }
}

/// `<société Unity>` / `<produit Unity>` : `<Jeu>_Data/app.info` (ligne 1, ligne 2) ; plusieurs `*_Data` qui
/// disent autre chose : rien.
pub fn lire_unity(jeu: &Path) -> Option<(String, String)> {
    let mut vus: Vec<(String, String)> = Vec::new();
    for e in std::fs::read_dir(jeu).ok()?.flatten().take(ENTREES_PAR_DOSSIER) {
        let nom = e.file_name().to_string_lossy().to_string();
        if !e.file_type().is_ok_and(|t| t.is_dir()) || !nom.to_lowercase().ends_with("_data") {
            continue;
        }
        let Some(o) = lire_plafonne(&e.path().join("app.info"), APP_INFO_MAX) else { continue };
        let t = String::from_utf8(o).ok()?;
        let mut l = t.trim_start_matches('\u{feff}').split('\n').map(|x| x.trim_end_matches('\r'));
        let (Some(societe), Some(produit)) = (l.next(), l.next()) else { continue };
        if !vus.iter().any(|(a, b)| a == societe && b == produit) {
            vus.push((societe.to_string(), produit.to_string()));
        }
    }
    (vus.len() == 1).then(|| vus.remove(0))
}

/// `<projet Unreal>` : le dossier voisin de `Engine\` qui contient `Content\Paks` (un seul).
pub fn lire_unreal(jeu: &Path) -> Option<String> {
    if !std::fs::symlink_metadata(jeu.join("Engine")).is_ok_and(|m| m.is_dir()) {
        return None;
    }
    let mut l: Vec<String> = Vec::new();
    for e in std::fs::read_dir(jeu).ok()?.flatten().take(ENTREES_PAR_DOSSIER) {
        let nom = e.file_name().to_string_lossy().to_string();
        if nom.eq_ignore_ascii_case("Engine") || !e.file_type().is_ok_and(|t| t.is_dir()) {
            continue;
        }
        if std::fs::symlink_metadata(e.path().join("Content").join("Paks")).is_ok_and(|m| m.is_dir()) {
            l.push(nom);
        }
    }
    (l.len() == 1).then(|| l.remove(0))
}

/// Ce qui a été reconnu d'un jeu.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Constat {
    pub cle: String,
    pub nom: String,
    /// Le NOM du fichier (ou dossier) marqueur trouvé.
    pub marqueur: String,
    /// Les NOMS des fichiers où un nom a été lu.
    pub lus_dans: Vec<String>,
    /// Les noms lus (état local seulement : jamais au journal ; pour la remontée du besoin 60, plus tard).
    pub noms_lus: Vec<String>,
    /// Les gabarits appliqués (chemins aux noms posés), d'origine `moteur`.
    pub emplacements: Vec<EmplacementPartie>,
    /// Firehouse nomme pour ce jeu un AUTRE moteur connu des références : désaccord noté (état local, besoin 60).
    pub firehouse_contredit: bool,
}

impl Constat {
    /// La preuve : le fichier marqueur, et le fichier lu s'il y en a un (des noms de fichiers seulement).
    pub fn preuve(&self) -> String {
        if self.lus_dans.is_empty() {
            self.marqueur.clone()
        } else {
            format!("{} + {}", self.marqueur, self.lus_dans.join(", "))
        }
    }
}

/// Ce que dit le disque du moteur d'un jeu.
#[derive(Debug, Clone, PartialEq)]
pub enum Reconnaissance {
    /// Aucun moteur connu de Firehouse.
    Aucun,
    /// Les marqueurs de plusieurs moteurs : rien n'est appliqué (jamais deviné).
    Douteux,
    Prouve(Constat),
}

/// Le nom à poser à la place d'un jeton lisible, lu une seule fois par jeu.
#[derive(Default)]
struct Lecteurs {
    renpy: Option<NomLu>,
    unity: Option<Option<(String, String)>>,
    unreal: Option<Option<String>>,
}

impl Lecteurs {
    /// (valeur, fichier lu) pour un jeton, `None` : pas de nom (non lu, refusé, ou « aucun »).
    fn valeur(&mut self, jeton: &str, jeu: &Path) -> Option<(String, String)> {
        let (v, fichier) = match jeton {
            JETON_RENPY => match self.renpy.get_or_insert_with(|| lire_renpy(jeu)) {
                NomLu::Valeur(v, f) => (v.clone(), f.clone()),
                _ => return None,
            },
            JETON_SOCIETE_UNITY | JETON_PRODUIT_UNITY => {
                let (s, p) = self.unity.get_or_insert_with(|| lire_unity(jeu)).clone()?;
                // Jamais un dossier commun seul (`LocalLow\<société>` sans produit) : les deux, sûrs, ou rien. Le couple
                // par défaut de l'éditeur (« DefaultCompany » / « My project ») est partagé par d'innombrables jeux.
                let tasse = |x: &str| x.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_lowercase();
                if !valeur_sure(&s) || !valeur_sure(&p) || (tasse(&s) == "defaultcompany" && tasse(&p) == "myproject") {
                    return None;
                }
                (if jeton == JETON_SOCIETE_UNITY { s } else { p }, "app.info".to_string())
            }
            JETON_PROJET_UNREAL => (self.unreal.get_or_insert_with(|| lire_unreal(jeu)).clone()?, "Content\\Paks".to_string()),
            _ => return None,
        };
        // `%APPDATA%\RenPy\tokens` (clés de signature communes à tous les jeux Ren'Py) et `persistent` (données
        // partagées entre jeux d'un auteur) : jamais le dossier d'UN jeu.
        if !valeur_sure(&v) || (jeton == JETON_RENPY && ["tokens", "persistent"].contains(&v.to_lowercase().as_str())) {
            return None;
        }
        Some((v, fichier))
    }
}

/// Les jetons `<…>` d'un chemin, en minuscules (apostrophe typographique ramenée à la droite).
fn jetons_de(chemin: &str) -> Vec<String> {
    let mut l = Vec::new();
    let mut reste = chemin;
    while let Some(i) = reste.find('<') {
        let Some(j) = reste[i..].find('>') else { break };
        l.push(reste[i..i + j + 1].to_lowercase().replace('’', "'"));
        reste = &reste[i + j + 1..];
    }
    l
}

/// Remplace un jeton (comparé en minuscules) par sa valeur.
fn poser(chemin: &str, jeton: &str, valeur: &str) -> String {
    let mut r = String::new();
    let mut reste = chemin.to_string();
    loop {
        let bas = reste.to_lowercase().replace('’', "'");
        // `to_lowercase` peut changer la longueur d'un caractère : on ne remplace que si les positions concordent.
        let Some(i) = bas.find(jeton).filter(|_| bas.len() == reste.len()) else { break };
        r.push_str(&reste[..i]);
        r.push_str(valeur);
        reste = reste[i + jeton.len()..].to_string();
    }
    r.push_str(&reste);
    r
}

/// Applique les gabarits d'une règle au jeu : (emplacements, fichiers lus, noms lus). Un gabarit est appliqué si chacun
/// de ses jetons est lu (deuxième preuve), ou s'il n'en a pas et reste DANS le dossier du jeu.
fn gabarits(r: &RegleMoteur, jeu: &Path, lecteurs: &mut Lecteurs) -> (Vec<EmplacementPartie>, Vec<String>, Vec<String>) {
    let (mut l, mut lus, mut noms) = (Vec::new(), Vec::new(), Vec::new());
    for (chemin, nature) in &r.sauvegardes {
        let c = chemin.trim().replace('/', "\\");
        let mut sortie = c.clone();
        let mut ok = true;
        let mut a_lire = false;
        for j in jetons_de(&c) {
            if j == JETON_JEU {
                continue;
            }
            a_lire = true;
            match lecteurs.valeur(&j, jeu) {
                Some((v, f)) => {
                    sortie = poser(&sortie, &j, &v);
                    if !lus.contains(&f) {
                        lus.push(f);
                    }
                    if !noms.contains(&v) {
                        noms.push(v);
                    }
                }
                None => {
                    ok = false;
                    break;
                }
            }
        }
        // Sans nom à lire : seulement DANS le dossier du jeu, plus bas que lui (jamais `%APPDATA%\RenPy` seul).
        let dans_le_jeu = sortie.to_lowercase().starts_with(&format!("{JETON_JEU}\\")) && sortie.len() > JETON_JEU.len() + 1;
        // Un motif qui prendrait TOUT un dossier (`*`, `*.*`) : jamais un emplacement de parties.
        let dernier = sortie.rsplit('\\').next().unwrap_or("");
        if nature != "dossier" && dernier.chars().all(|c| matches!(c, '*' | '?' | '.')) {
            continue;
        }
        if !ok || (!a_lire && !dans_le_jeu) || sortie.contains(['<', '>']) && !dans_le_jeu_seulement(&sortie) {
            continue;
        }
        if l.iter().any(|e: &EmplacementPartie| e.chemin.eq_ignore_ascii_case(&sortie)) {
            continue;
        }
        l.push(EmplacementPartie {
            chemin: sortie,
            nature: nature.clone(),
            os: "Windows".into(),
            origine: "moteur".into(),
            moteur: Some(r.nom.clone()),
            ..Default::default()
        });
    }
    (l, lus, noms)
}

/// Le seul jeton restant est `<dossier du jeu>`.
fn dans_le_jeu_seulement(chemin: &str) -> bool {
    jetons_de(chemin).iter().all(|j| j == JETON_JEU)
}

/// La règle que désigne le moteur nommé par Firehouse pour ce jeu (`moteur` : sa clé ou son nom, sans tenir compte de
/// la casse) ; `None` s'il n'en désigne aucune (« RPGM », `null`…).
pub fn regle_nommee<'a>(regles: &'a [RegleMoteur], indice: Option<&str>) -> Option<&'a RegleMoteur> {
    let i = indice.map(str::trim).filter(|i| !i.is_empty())?;
    regles.iter().find(|r| r.cle.eq_ignore_ascii_case(i) || r.nom.eq_ignore_ascii_case(i))
}

/// Reconnaît le moteur du jeu installé dans `jeu` d'après les règles de Firehouse, et lit ses noms. Jamais pour une
/// racine (un disque, un dossier trop haut).
pub fn reconnaitre(jeu: &Path, regles: &[RegleMoteur]) -> Reconnaissance {
    reconnaitre_avec(jeu, regles, None)
}

/// Comme `reconnaitre`, avec le moteur que Firehouse nomme pour ce jeu (`indice`) : il départage les marqueurs de
/// plusieurs moteurs s'il est l'un d'eux ; s'il désigne un autre moteur connu que celui du disque, c'est noté
/// (`firehouse_contredit`). Chaque règle a son propre plafond d'entrées lues.
pub fn reconnaitre_avec(jeu: &Path, regles: &[RegleMoteur], indice: Option<&str>) -> Reconnaissance {
    if regles.is_empty() || jeu.components().count() <= 2 || !std::fs::symlink_metadata(jeu).is_ok_and(|m| m.is_dir()) {
        return Reconnaissance::Aucun;
    }
    let mut trouves: Vec<(&RegleMoteur, String)> = Vec::new();
    for r in regles {
        let mut budget = Budget(ENTREES_MAX);
        if let Some(m) = r.reconnaitre.iter().find_map(|m| trouver(jeu, m, &mut budget)) {
            trouves.push((r, m));
        }
    }
    let nommee = regle_nommee(regles, indice);
    let (r, marqueur) = match trouves.len() {
        0 => return Reconnaissance::Aucun,
        1 => trouves.remove(0),
        _ => match nommee.and_then(|n| trouves.iter().position(|(r, _)| r.cle == n.cle)) {
            Some(k) => trouves.remove(k),
            None => return Reconnaissance::Douteux,
        },
    };
    let mut c = Constat { cle: r.cle.clone(), nom: r.nom.clone(), marqueur, firehouse_contredit: nommee.is_some_and(|n| n.cle != r.cle), ..Default::default() };
    let (l, lus, noms) = gabarits(r, jeu, &mut Lecteurs::default());
    c.lus_dans = lus;
    c.noms_lus = noms;
    let preuve = c.preuve();
    c.emplacements = l
        .into_iter()
        .map(|mut e| {
            e.preuve = Some(preuve.clone());
            e
        })
        .collect();
    Reconnaissance::Prouve(c)
}

#[cfg(test)]
mod essai_reel {
    #[test]
    #[ignore]
    fn essai_moteur_s6() {
        let jeu = std::path::PathBuf::from(std::env::var("FROGTEND_DOSSIER_JEU").expect("FROGTEND_DOSSIER_JEU"));
        let refs: serde_json::Value = serde_json::from_slice(include_bytes!("../fixtures/sauvegardes-references-148-reel.json")).unwrap();
        let regles = super::regles(&refs);
        println!("== règles : {}", regles.len());
        match super::reconnaitre(&jeu, &regles) {
            super::Reconnaissance::Prouve(c) => {
                println!("== moteur {} ({}), marqueur {:?}, lu dans {:?}, noms {:?}", c.nom, c.cle, c.marqueur, c.lus_dans, c.noms_lus);
                for e in &c.emplacements {
                    println!("== emplacement {:?} ({:?})", e.chemin, e.nature);
                }
            }
            autre => println!("== {autre:?}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn refs() -> Value {
        serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/sauvegardes-references-148-reel.json")).unwrap()).unwrap()
    }

    fn ecrire(d: &Path, r: &str, o: &[u8]) {
        let f = r.split('/').fold(d.to_path_buf(), |p, s| p.join(s));
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(f, o).unwrap();
    }

    fn zlib(o: &[u8]) -> Vec<u8> {
        let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        e.write_all(o).unwrap();
        e.finish().unwrap()
    }

    /// Un `.rpyc` au format 2 : l'en-tête, une entrée (slot 1), la fin de table, puis le bloc zlib.
    fn rpyc2(contenu: &[u8]) -> Vec<u8> {
        let bloc = zlib(contenu);
        let debut = 10 + 12 * 2;
        let mut o = b"RENPY RPC2".to_vec();
        for n in [1u32, debut as u32, bloc.len() as u32, 0, 0, 0] {
            o.extend(n.to_le_bytes());
        }
        o.extend(bloc);
        o
    }

    fn prouve(r: Reconnaissance) -> Constat {
        match r {
            Reconnaissance::Prouve(c) => c,
            autre => panic!("un moteur prouvé attendu : {autre:?}"),
        }
    }

    fn chemins(c: &Constat) -> Vec<&str> {
        c.emplacements.iter().map(|e| e.chemin.as_str()).collect()
    }

    #[test]
    fn les_regles_reelles_sont_lues_sans_le_registre() {
        let l = regles(&refs());
        assert_eq!(l.len(), 13);
        let unity = l.iter().find(|r| r.cle == "unity").unwrap();
        assert_eq!(unity.nom, "Unity");
        assert_eq!(unity.sauvegardes, vec![(r"%USERPROFILE%\AppData\LocalLow\<société Unity>\<produit Unity>".to_string(), "dossier".to_string())], "jamais HKEY");
        assert_eq!(l.iter().find(|r| r.cle == "rpgmaker_vxace").unwrap().sauvegardes[0].1, "motif");
        assert!(regles(&serde_json::json!({"os": {}})).is_empty(), "sans `moteurs` : rien de deviné");
    }

    #[test]
    fn les_motifs_restent_au_premier_niveau_ou_sur_leur_chemin() {
        let d = tempfile::tempdir().unwrap();
        let j = d.path().join("Jeux").join("Jeu");
        ecrire(&j, "www/js/rpg_core.js", b"x");
        ecrire(&j, "Projet/Content/Paks/Projet-WindowsNoEditor.pak", b"x");
        ecrire(&j, "profond/a/b/c/UnityPlayer.dll", b"x");
        std::fs::create_dir_all(j.join("renpy")).unwrap();
        let mut b = Budget(ENTREES_MAX);
        assert_eq!(trouver(&j, "www/js/rpg_core.js", &mut b).as_deref(), Some("rpg_core.js"));
        assert_eq!(trouver(&j, "*/Content/Paks/*.pak", &mut b).as_deref(), Some("Projet-WindowsNoEditor.pak"));
        assert_eq!(trouver(&j, "renpy/", &mut b).as_deref(), Some("renpy"));
        assert_eq!(trouver(&j, "renpy", &mut b), None, "un fichier attendu, c'est un dossier");
        assert_eq!(trouver(&j, "UnityPlayer.dll", &mut b), None, "jamais plus bas que le premier niveau");
        assert_eq!(trouver(&j, "../Jeu/renpy/", &mut b), None);
        assert_eq!(trouver(&j, "C:/Windows/", &mut b), None);
        // Le plafond : rien n'est plus lu.
        let mut vide = Budget(0);
        assert_eq!(trouver(&j, "renpy/", &mut vide), None);
    }

    #[test]
    fn une_valeur_lue_n_est_jamais_un_chemin() {
        for ok in ["MonJeu-1234567890", "Studio Été", "Jeu (2024)"] {
            assert!(valeur_sure(ok), "{ok}");
        }
        for ko in ["", "..", "a\\b", "a/b", "C:", "a*b", "a?", "<x>", "a|b", "a\"b", "%APPDATA%", " jeu", "jeu.", "CON", "nul.txt", "COM1", "a\u{1}b", "x..y"] {
            assert!(!valeur_sure(ko), "{ko:?}");
        }
    }

    #[test]
    fn options_rpy_se_lit_en_clair() {
        let t = "# define config.save_directory = \"Commentee\"\ninit -1:\n    define config.save_directory = \"MonJeu-1600000000\"\n";
        assert_eq!(save_directory_rpy(t, "options.rpy"), NomLu::Valeur("MonJeu-1600000000".into(), "options.rpy".into()));
        assert_eq!(save_directory_rpy("init -1 python hide:\n    config.save_directory = u'Ancien-1'\n", "options.rpy"), NomLu::Valeur("Ancien-1".into(), "options.rpy".into()));
        assert_eq!(save_directory_rpy("define config.save_directory = None\n", "options.rpy"), NomLu::Aucun);
        assert_eq!(save_directory_rpy("define config.save_directory = \"A\"\ndefine config.save_directory = \"B\"\n", "options.rpy"), NomLu::Inconnu, "deux valeurs : douteux");
        assert_eq!(save_directory_rpy("define config.save_directory = nom_calcule\n", "options.rpy"), NomLu::Inconnu, "une expression : jamais évaluée");
        assert_eq!(save_directory_rpy("if config.save_directory == \"x\":\n", "options.rpy"), NomLu::Inconnu);
    }

    #[test]
    fn un_rpyc_compresse_se_lit_par_ses_octets() {
        let pickle = b"\x80\x02crenpy.ast\nDefine\n\x8c\x0esave_directory\x8c\x0cstore.config\x8c\x14\"MonJeu-1600000000\"\x8c\x10game/options.rpy";
        assert_eq!(save_directory_rpyc(&rpyc2(pickle), "options.rpyc"), NomLu::Valeur("MonJeu-1600000000".into(), "options.rpyc".into()));
        // Format 1 : le fichier entier en zlib.
        assert_eq!(save_directory_rpyc(&zlib(pickle), "script.rpyc"), NomLu::Valeur("MonJeu-1600000000".into(), "script.rpyc".into()));
        assert_eq!(save_directory_rpyc(&zlib(b"\x8c\x0esave_directory\x8c\x04None"), "a.rpyc"), NomLu::Aucun);
        // Obscurci (pas du zlib), ou sans le mot : rien.
        assert_eq!(save_directory_rpyc(b"RENPY RPC2 DDGAME n'importe quoi", "a.rpyc"), NomLu::Inconnu);
        assert_eq!(save_directory_rpyc(&zlib(b"rien ici"), "a.rpyc"), NomLu::Inconnu);
        // Une bombe zlib : jamais décompressée au-delà du plafond.
        assert_eq!(degonfler(&zlib(&vec![0u8; 1000]), 100), Degonfle::Trop);
        assert_eq!(degonfler(b"pas du zlib", 100), Degonfle::PasZlib);
    }

    #[test]
    fn renpy_lit_options_rpy_puis_les_rpyc_et_garde_les_deux_emplacements() {
        let d = tempfile::tempdir().unwrap();
        let j = d.path().join("Jeux").join("Mon Jeu");
        std::fs::create_dir_all(j.join("renpy")).unwrap();
        ecrire(&j, "game/script.rpyc", &rpyc2(b"\x8c\x0esave_directory\x8c\x14\"MonJeu-1600000000\""));
        let c = prouve(reconnaitre(&j, &regles(&refs())));
        assert_eq!((c.cle.as_str(), c.nom.as_str(), c.marqueur.as_str()), ("renpy", "Ren'Py", "renpy"));
        assert_eq!(chemins(&c), [r"%APPDATA%\RenPy\MonJeu-1600000000", r"<dossier du jeu>\game\saves"], "les deux endroits");
        assert_eq!(c.preuve(), "renpy + script.rpyc", "des noms de fichiers seulement");
        assert!(c.emplacements.iter().all(|e| e.origine == "moteur" && e.moteur.as_deref() == Some("Ren'Py") && e.preuve.as_deref() == Some("renpy + script.rpyc")));
        // `options.rpy` en clair l'emporte.
        ecrire(&j, "game/options.rpy", b"define config.save_directory = \"Clair-1\"\n");
        assert_eq!(chemins(&prouve(reconnaitre(&j, &regles(&refs()))))[0], r"%APPDATA%\RenPy\Clair-1");
        // `None` : seulement `game\saves`.
        ecrire(&j, "game/options.rpy", b"define config.save_directory = None\n");
        assert_eq!(chemins(&prouve(reconnaitre(&j, &regles(&refs())))), [r"<dossier du jeu>\game\saves"]);
        // `tokens` (clés communes à tous les jeux Ren'Py) et un chemin : refusés, `game\saves` reste.
        for v in ["tokens", "persistent", "..", "a/b"] {
            ecrire(&j, "game/options.rpy", format!("define config.save_directory = \"{v}\"\n").as_bytes());
            assert_eq!(chemins(&prouve(reconnaitre(&j, &regles(&refs())))), [r"<dossier du jeu>\game\saves"], "{v}");
        }
    }

    #[test]
    fn unity_lit_app_info_et_jamais_le_registre() {
        let d = tempfile::tempdir().unwrap();
        let j = d.path().join("Jeux").join("Jeu");
        ecrire(&j, "UnityPlayer.dll", b"x");
        ecrire(&j, "Jeu_Data/app.info", "Studio Été\r\nMon Produit".as_bytes());
        let c = prouve(reconnaitre(&j, &regles(&refs())));
        assert_eq!(chemins(&c), [r"%USERPROFILE%\AppData\LocalLow\Studio Été\Mon Produit"]);
        assert_eq!(c.preuve(), "UnityPlayer.dll + app.info");
        assert_eq!(c.noms_lus, ["Studio Été", "Mon Produit"]);
        // Sans produit (une seule ligne) : jamais `LocalLow\<société>` seul ; le moteur reste reconnu.
        ecrire(&j, "Jeu_Data/app.info", b"Studio");
        let c = prouve(reconnaitre(&j, &regles(&refs())));
        assert!(c.emplacements.is_empty());
        // Un produit vide, une société avec « \ » : rien.
        ecrire(&j, "Jeu_Data/app.info", b"Studio\n");
        assert!(prouve(reconnaitre(&j, &regles(&refs()))).emplacements.is_empty());
        ecrire(&j, "Jeu_Data/app.info", b"..\\x\nJeu");
        assert!(prouve(reconnaitre(&j, &regles(&refs()))).emplacements.is_empty());
        // Deux `*_Data` qui se contredisent : rien.
        ecrire(&j, "Jeu_Data/app.info", b"A\nB");
        ecrire(&j, "Autre_Data/app.info", b"C\nD");
        assert!(prouve(reconnaitre(&j, &regles(&refs()))).emplacements.is_empty());
    }

    #[test]
    fn unreal_prend_le_dossier_voisin_d_engine() {
        let d = tempfile::tempdir().unwrap();
        let j = d.path().join("Jeux").join("Jeu");
        ecrire(&j, "Engine/Binaries/ThirdParty/x.dll", b"x");
        ecrire(&j, "MonProjet/Content/Paks/pakchunk0-Windows.pak", b"x");
        ecrire(&j, "MonProjet/Binaries/Win64/MonProjet-Win64-Shipping.exe", b"x");
        let c = prouve(reconnaitre(&j, &regles(&refs())));
        assert_eq!((c.cle.as_str(), c.marqueur.as_str()), ("unreal", "ThirdParty"));
        assert_eq!(chemins(&c), [r"%LOCALAPPDATA%\MonProjet\Saved\SaveGames"]);
        assert_eq!(c.preuve(), r"ThirdParty + Content\Paks");
        // Deux projets possibles : rien.
        ecrire(&j, "Autre/Content/Paks/a.pak", b"x");
        assert!(prouve(reconnaitre(&j, &regles(&refs()))).emplacements.is_empty());
    }

    #[test]
    fn rpg_maker_wolf_et_tyrano_n_ont_rien_a_lire_et_restent_dans_le_jeu() {
        let d = tempfile::tempdir().unwrap();
        let cas: [(&str, &str, &[&str]); 4] = [
            ("rpgmaker_mv", "www/js/rpg_core.js", &[r"<dossier du jeu>\www\save", r"<dossier du jeu>\save"]),
            ("rpgmaker_vxace", "Game.rgss3a", &[r"<dossier du jeu>\Save*.rvdata2"]),
            ("wolfrpg", "data.wolf", &[r"<dossier du jeu>\Save"]),
            ("tyrano", "tyrano.js", &[r"<dossier du jeu>\*_tyrano_data.sav"]),
        ];
        for (k, (cle, marqueur, attendus)) in cas.iter().enumerate() {
            let j = d.path().join("Jeux").join(format!("Jeu{k}"));
            ecrire(&j, marqueur, b"x");
            let c = prouve(reconnaitre(&j, &regles(&refs())));
            assert_eq!(c.cle, *cle);
            assert_eq!(chemins(&c), *attendus);
            assert!(c.lus_dans.is_empty() && c.preuve() == marqueur.rsplit('/').next().unwrap());
        }
    }

    #[test]
    fn godot_et_gamemaker_sont_reconnus_sans_rien_appliquer() {
        let d = tempfile::tempdir().unwrap();
        let g = d.path().join("Jeux").join("G");
        ecrire(&g, "Jeu.pck", b"GDPC");
        let c = prouve(reconnaitre(&g, &regles(&refs())));
        assert_eq!(c.cle, "godot");
        assert!(c.emplacements.is_empty(), "<projet Godot> : pas de lecture sûre et simple (pck binaire), pas appliqué");
        let m = d.path().join("Jeux").join("M");
        ecrire(&m, "data.win", b"x");
        let c = prouve(reconnaitre(&m, &regles(&refs())));
        assert_eq!(c.cle, "gamemaker");
        assert!(c.emplacements.is_empty(), "nom_a_lire « inconnu » : pas appliqué");
    }

    #[test]
    fn deux_moteurs_ou_une_racine_ne_donnent_rien() {
        let d = tempfile::tempdir().unwrap();
        let j = d.path().join("Jeux").join("Jeu");
        ecrire(&j, "UnityPlayer.dll", b"x");
        ecrire(&j, "data.wolf", b"x");
        assert_eq!(reconnaitre(&j, &regles(&refs())), Reconnaissance::Douteux);
        assert_eq!(reconnaitre(Path::new("C:\\"), &regles(&refs())), Reconnaissance::Aucun, "jamais un disque");
        assert_eq!(reconnaitre(&d.path().join("Absent").join("X"), &regles(&refs())), Reconnaissance::Aucun);
        assert_eq!(reconnaitre(&j, &[]), Reconnaissance::Aucun, "sans les règles de Firehouse : rien");
        let vide = d.path().join("Jeux").join("Vide");
        std::fs::create_dir_all(&vide).unwrap();
        assert_eq!(reconnaitre(&vide, &regles(&refs())), Reconnaissance::Aucun);
    }

    #[test]
    fn un_gabarit_sans_nom_hors_du_jeu_ou_avec_un_jeton_inconnu_n_est_jamais_applique() {
        let d = tempfile::tempdir().unwrap();
        let j = d.path().join("Jeux").join("Jeu");
        ecrire(&j, "marqueur.bin", b"x");
        let r = RegleMoteur {
            cle: "x".into(),
            nom: "X".into(),
            reconnaitre: vec!["marqueur.bin".into()],
            sauvegardes: vec![
                (r"%APPDATA%\RenPy".into(), "dossier".into()),
                (r"<dossier du jeu>".into(), "dossier".into()),
                (r"%APPDATA%\<inconnu>\x".into(), "dossier".into()),
                (r"<dossier du jeu>\<nom du dossier du jeu>\x".into(), "dossier".into()),
                (r"<dossier du jeu>\Saves".into(), "dossier".into()),
            ],
            ..Default::default()
        };
        assert_eq!(chemins(&prouve(reconnaitre(&j, &[r]))), [r"<dossier du jeu>\Saves"]);
    }

    /// Relecture S6 : le couple par défaut de l'éditeur Unity est partagé par d'innombrables jeux : refusé.
    #[test]
    fn unity_refuse_le_couple_par_defaut() {
        let d = tempfile::tempdir().unwrap();
        let j = d.path().join("Jeux").join("Jeu");
        ecrire(&j, "UnityPlayer.dll", b"x");
        for t in ["DefaultCompany\nMy project", "defaultcompany\r\nMy Project", "Default Company\nMyProject"] {
            ecrire(&j, "Jeu_Data/app.info", t.as_bytes());
            assert!(prouve(reconnaitre(&j, &regles(&refs()))).emplacements.is_empty(), "{t:?}");
        }
        // La société par défaut avec un vrai produit : gardé (le produit les sépare).
        ecrire(&j, "Jeu_Data/app.info", b"DefaultCompany\nMon Jeu");
        assert_eq!(chemins(&prouve(reconnaitre(&j, &regles(&refs())))), [r"%USERPROFILE%\AppData\LocalLow\DefaultCompany\Mon Jeu"]);
    }

    /// Relecture S6 : un motif qui prendrait tout un dossier (`*`, `*.*`) n'est jamais appliqué.
    #[test]
    fn un_motif_qui_prend_tout_n_est_jamais_applique() {
        let d = tempfile::tempdir().unwrap();
        let j = d.path().join("Jeux").join("Jeu");
        ecrire(&j, "marqueur.bin", b"x");
        let r = RegleMoteur {
            cle: "x".into(),
            nom: "X".into(),
            reconnaitre: vec!["marqueur.bin".into()],
            sauvegardes: vec![(r"<dossier du jeu>\*".into(), "motif".into()), (r"<dossier du jeu>\*.*".into(), "motif".into()), (r"<dossier du jeu>\Save\*.sav".into(), "motif".into())],
            ..Default::default()
        };
        assert_eq!(chemins(&prouve(reconnaitre(&j, &[r]))), [r"<dossier du jeu>\Save\*.sav"]);
    }

    /// Relecture S6 : deux marqueurs sont départagés par le moteur que Firehouse nomme, s'il est l'un d'eux ; un moteur
    /// connu différent de celui du disque est noté.
    #[test]
    fn le_moteur_nomme_par_firehouse_departage_ou_contredit() {
        let d = tempfile::tempdir().unwrap();
        let j = d.path().join("Jeux").join("Jeu");
        ecrire(&j, "UnityPlayer.dll", b"x");
        ecrire(&j, "data.wolf", b"x");
        let l = regles(&refs());
        assert_eq!(reconnaitre_avec(&j, &l, None), Reconnaissance::Douteux);
        assert_eq!(reconnaitre_avec(&j, &l, Some("RPGM")), Reconnaissance::Douteux, "un nom qui ne désigne aucune règle");
        assert_eq!(reconnaitre_avec(&j, &l, Some("godot")), Reconnaissance::Douteux, "pas un des candidats");
        let c = prouve(reconnaitre_avec(&j, &l, Some("wolfrpg")));
        assert_eq!((c.cle.as_str(), c.firehouse_contredit), ("wolfrpg", false));
        // Un seul marqueur, Firehouse nomme un autre moteur connu : désaccord noté ; un nom inconnu : rien.
        let u = d.path().join("Jeux").join("U");
        ecrire(&u, "UnityPlayer.dll", b"x");
        assert!(prouve(reconnaitre_avec(&u, &l, Some("Ren'Py"))).firehouse_contredit);
        assert!(!prouve(reconnaitre_avec(&u, &l, Some("unity"))).firehouse_contredit);
        assert!(!prouve(reconnaitre_avec(&u, &l, Some("RPGM"))).firehouse_contredit);
    }

    /// Relecture S6 : Ren'Py s'arrête dès qu'`options.rpy` ou `options.rpyc` répond ; un budget global de lecture.
    #[test]
    fn renpy_s_arrete_a_options_et_respecte_son_budget() {
        let d = tempfile::tempdir().unwrap();
        let j = d.path().join("Jeux").join("Jeu");
        std::fs::create_dir_all(j.join("renpy")).unwrap();
        ecrire(&j, "game/options.rpyc", &rpyc2(b"\x8c\x0esave_directory\x8c\x03\"A\""));
        ecrire(&j, "game/script.rpyc", &rpyc2(b"\x8c\x0esave_directory\x8c\x03\"B\""));
        assert_eq!(lire_renpy(&j), NomLu::Valeur("A".into(), "options.rpyc".into()), "options.rpyc décide, script.rpyc n'est pas lu");
        // Sans options.rpyc : les autres, une valeur unique.
        std::fs::remove_file(j.join("game").join("options.rpyc")).unwrap();
        assert_eq!(lire_renpy(&j), NomLu::Valeur("B".into(), "script.rpyc".into()));
        // Budget épuisé (fichiers ou décompression) : inconnu, jamais deviné.
        assert_eq!(lire_renpy_avec(&j, 10), NomLu::Inconnu);
        let gros = [b"\x8c\x0esave_directory\x8c\x03\"B\"".to_vec(), vec![0u8; 4000]].concat();
        ecrire(&j, "game/script.rpyc", &rpyc2(&gros));
        let taille = std::fs::metadata(j.join("game").join("script.rpyc")).unwrap().len() as usize;
        assert_eq!(lire_renpy_avec(&j, taille + 100), NomLu::Inconnu, "décompressé, il dépasse le budget");
        assert_eq!(lire_renpy_avec(&j, taille + 5000), NomLu::Valeur("B".into(), "script.rpyc".into()));
    }
}
