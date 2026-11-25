//! Lot M1 (✅ Seb 25/11, docs/PLAN.md « Lot M — modding des jeux PC ») : les mods posés par VORTEX dans un jeu PC.
//! Frogtend les repère, dit pourquoi le jeu ne démarre plus (diagnostic, sans rien écrire), les RETIRE avec preuve et
//! de façon réversible, et en REMET un à la demande. Rien n'est propre à un jeu : tout jeu dont Vortex a posé les mods
//! par liens durs (`hardlink_activator`) ; les autres méthodes (liens symboliques, déplacement) sont refusées.
//!
//! Règles (décisions de Seb du 25/11) :
//! - le manifeste de Vortex (`vortex.deployment.json`) est une donnée NON SÛRE : chaque chemin est vérifié, le dépôt
//!   doit être un vrai dossier hors du jeu (sans lien ni jonction), la cible doit être le dossier du jeu ;
//! - un fichier n'est déplacé que s'il est PROUVÉ venir de Vortex : listé, même volume et même index de fichier que son
//!   fichier du dépôt, au moins 2 liens, aucun point de réanalyse, le fichier du dépôt présent ; sinon il est « laissé
//!   tel quel » (listé, jamais touché) ;
//! - le geste est un RENOMMAGE sur le même disque (jamais un effacement, jamais une copie, jamais par-dessus un autre
//!   fichier) vers `<jeu>\Frogtend\vortex\<horodatage>\fichiers\…` ; le manifeste du retrait (`retrait.json`) est écrit
//!   AVANT le premier déplacement, puis tenu à jour par lots : une coupure se reprend (ou tout se remet) ;
//! - les marqueurs de Vortex ne s'enlèvent qu'avec leur dossier, et seulement s'il ne contient plus qu'eux (texte exact
//!   de Vortex) ; `vortex.deployment.json` est rangé EN DERNIER ; une sauvegarde `.vortex_backup` reste en place ;
//! - le dépôt de Vortex n'est JAMAIS modifié ; les données des mods hors manifeste (réglages, journaux) non plus.

use crate::erreurs::{Erreur, Resultat};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Le manifeste de Vortex, à la racine du jeu.
pub const MANIFESTE_VORTEX: &str = "vortex.deployment.json";
/// Le marqueur que Vortex pose dans chaque dossier qu'il a créé.
pub const MARQUEUR: &str = "__folder_managed_by_vortex";
/// Son texte exact (relevé le 25/11 : 449 marqueurs identiques, sans fin de ligne).
pub const TEXTE_MARQUEUR: &[u8] = b"This directory was created by Vortex deployment and will be removed during purging if it's empty";
/// La seule méthode de Vortex que Frogtend sait défaire avec preuve.
pub const METHODE_LIENS: &str = "hardlink_activator";
/// Le suffixe des sauvegardes de Vortex (un original remplacé) : jamais remises par Frogtend.
pub const SUFFIXE_SAUVEGARDE: &str = ".vortex_backup";
/// Le manifeste d'un retrait, dans son dossier.
const NOM_RETRAIT: &str = "retrait.json";
/// Au-delà, un manifeste (de Vortex ou d'un retrait) n'est pas lu.
const TAILLE_MAX_MANIFESTE: u64 = 64 * 1024 * 1024;
/// Le manifeste du retrait est réécrit tous les `LOT` fichiers déplacés (et à la fin).
const LOT: usize = 100;

// ---------------------------------------------------------------------------------------------------------------
// L'identité d'un fichier sur le disque (volume, index, liens) : la preuve qu'il est le même que celui du dépôt.
// ---------------------------------------------------------------------------------------------------------------

/// Ce que Windows dit d'un fichier, sans le suivre s'il est un lien.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Identite {
    pub volume: u32,
    pub index: u64,
    pub liens: u32,
    pub point_de_reanalyse: bool,
    pub dossier: bool,
    pub taille: u64,
}

/// L'identité d'un fichier (`GetFileInformationByHandle`), ouvert SANS suivre un lien ni une jonction.
#[cfg(windows)]
pub fn identite(p: &Path) -> std::io::Result<Identite> {
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Storage::FileSystem::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION};
    const LIRE_ATTRIBUTS: u32 = 0x80;
    const SEMANTIQUE_SAUVEGARDE: u32 = 0x0200_0000;
    const OUVRIR_LE_POINT_DE_REANALYSE: u32 = 0x0020_0000;
    let f = std::fs::OpenOptions::new()
        .access_mode(LIRE_ATTRIBUTS)
        .share_mode(7)
        .custom_flags(SEMANTIQUE_SAUVEGARDE | OUVRIR_LE_POINT_DE_REANALYSE)
        .open(p)?;
    let mut i = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: la poignée vient d'un fichier ouvert (fermé à la fin de la fonction) ; la structure est à nous.
    unsafe { GetFileInformationByHandle(HANDLE(f.as_raw_handle()), &mut i) }.map_err(|e| std::io::Error::from_raw_os_error(e.code().0 & 0xFFFF))?;
    Ok(Identite {
        volume: i.dwVolumeSerialNumber,
        index: ((i.nFileIndexHigh as u64) << 32) | i.nFileIndexLow as u64,
        liens: i.nNumberOfLinks,
        point_de_reanalyse: i.dwFileAttributes & 0x400 != 0,
        dossier: i.dwFileAttributes & 0x10 != 0,
        taille: ((i.nFileSizeHigh as u64) << 32) | i.nFileSizeLow as u64,
    })
}

#[cfg(not(windows))]
pub fn identite(_p: &Path) -> std::io::Result<Identite> {
    Err(std::io::Error::from(std::io::ErrorKind::Unsupported))
}

/// Le nombre de liens d'un fichier (`None` : illisible ou absent).
pub fn liens(p: &Path) -> Option<u32> {
    identite(p).ok().map(|i| i.liens)
}

/// Un chemin relatif sûr (`decompression::chemin_sur`, séparateur `/`), sans nom réservé de Windows (CON, NUL…, un
/// point ou une espace finale) ni le dossier de Frogtend du jeu. `None` : refusé.
fn rel_sur(brut: &str) -> Option<String> {
    let r = crate::decompression::chemin_sur(brut)?;
    let mut parties = r.split('/');
    if parties.clone().any(|p| crate::mods::nom_reserve(p) || nom_court(p)) || parties.next().is_some_and(|p| p.eq_ignore_ascii_case("frogtend")) {
        return None;
    }
    Some(r)
}

/// Un nom court de Windows (8.3, « PROGRA~1 ») : un « ~ » suivi d'un chiffre — il pourrait désigner un autre fichier
/// que celui écrit. Un « ~ » en tête de nom (« ~Cyberarms_Patch.archive », ordre de chargement des archives, relevé
/// chez Seb le 25/11) n'en est pas un.
fn nom_court(p: &str) -> bool {
    p.as_bytes().windows(2).any(|w| w[0] == b'~' && w[1].is_ascii_digit())
}

/// Le nom d'un mod dans le dépôt de Vortex : UN seul composant sûr.
fn source_sure(brut: &str) -> Option<String> {
    rel_sur(brut).filter(|s| !s.contains('/'))
}

/// `base` + un chemin relatif sûr (`/`), composant par composant (jamais de `/` mêlé aux `\`).
fn joindre(base: &Path, rel: &str) -> PathBuf {
    let mut p = base.to_path_buf();
    for c in rel.split('/').filter(|c| !c.is_empty()) {
        p.push(c);
    }
    p
}

/// Un chemin long (plus de 240 caractères) passé à Windows sous sa forme `\\?\` (le renommage direct ne l'accepte pas
/// autrement).
fn long(p: &Path) -> PathBuf {
    let t = p.to_string_lossy();
    if t.len() < 240 || t.starts_with(r"\\") || t.as_bytes().get(1) != Some(&b':') {
        return p.to_path_buf();
    }
    PathBuf::from(format!(r"\\?\{}", t.replace('/', r"\")))
}

/// Déplace sur le même disque, JAMAIS par-dessus un fichier déjà là, jamais par une copie (un autre disque échoue).
fn deplacer(de: &Path, vers: &Path) -> std::io::Result<()> {
    crate::pack_mame::deplacer_sans_ecraser(&long(de), &long(vers))
}

// ---------------------------------------------------------------------------------------------------------------
// Le manifeste de Vortex (donnée non sûre).
// ---------------------------------------------------------------------------------------------------------------

#[derive(Deserialize)]
struct DeploiementBrut {
    #[serde(rename = "deploymentMethod", default)]
    methode: String,
    #[serde(rename = "stagingPath", default)]
    depot: String,
    #[serde(rename = "targetPath", default)]
    cible: String,
    #[serde(default)]
    files: Vec<FichierBrut>,
}

#[derive(Deserialize)]
struct FichierBrut {
    #[serde(rename = "relPath", default)]
    rel: String,
    #[serde(default)]
    source: String,
}

/// Une entrée du manifeste de Vortex : `sur` faux = un chemin refusé (gardé tel quel pour être listé, jamais suivi).
#[derive(Debug, Clone, PartialEq)]
pub struct EntreeVortex {
    pub rel: String,
    pub source: String,
    pub sur: bool,
}

/// Le manifeste de Vortex vérifié : son dépôt (vrai dossier hors du jeu) et ses fichiers.
#[derive(Debug, Clone)]
pub struct Deploiement {
    pub depot: PathBuf,
    pub fichiers: Vec<EntreeVortex>,
}

/// Le texte du manifeste (un vrai fichier, de taille raisonnable) ; `None` : absent.
fn lire_brut(jeu: &Path) -> Resultat<Option<Vec<u8>>> {
    let f = jeu.join(MANIFESTE_VORTEX);
    let m = match std::fs::symlink_metadata(&f) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    if !m.is_file() || crate::garde::est_un_lien(&f) {
        return Err(Erreur::Refus("Le fichier de Vortex de ce jeu n'est pas un vrai fichier : Frogtend n'y touche pas.".into()));
    }
    if m.len() > TAILLE_MAX_MANIFESTE {
        return Err(Erreur::Refus("Le fichier de Vortex de ce jeu est trop gros pour être lu : Frogtend n'y touche pas.".into()));
    }
    Ok(Some(std::fs::read(&f)?))
}

/// Un dépôt de Vortex qu'on peut croire : chemin absolu, vrai dossier, ni dans le jeu ni au-dessus, sans lien ni
/// jonction sur tout son chemin.
fn depot_sur(depot: &Path, jeu: &Path) -> bool {
    if crate::garde::cle(depot).is_none() || crate::garde::sous(depot, jeu) || crate::garde::sous(jeu, depot) {
        return false;
    }
    if !std::fs::symlink_metadata(depot).is_ok_and(|m| m.is_dir()) {
        return false;
    }
    !depot.ancestors().any(crate::garde::est_un_lien)
}

/// Le motif d'une méthode de Vortex que Frogtend ne sait pas défaire.
fn motif_methode(m: &str) -> String {
    let facon = match m {
        "symlink_activator" | "symlink_activator_elevate" => "par des raccourcis vers son dossier de mods",
        "move_activator" => "en déplaçant leurs fichiers",
        _ => "d'une façon que Frogtend ne connaît pas",
    };
    // Montré à l'écran : jamais « lien dur » ni nom technique.
    format!("Vortex a posé ces mods {facon} : Frogtend ne sait retirer avec certitude que ceux posés de la façon habituelle de Vortex. Rien n'est touché.")
}

/// Le manifeste de Vortex du jeu, vérifié ; `Ok(None)` : il n'y en a pas. Une méthode autre que les liens durs, une
/// cible qui n'est pas ce jeu ou un dépôt douteux : refus (rien ne sera touché). Un chemin de fichier dangereux ne
/// refuse pas tout : l'entrée est gardée « pas sûre » (laissée telle quelle au retrait).
pub fn lire_deploiement(jeu: &Path) -> Resultat<Option<Deploiement>> {
    let Some(octets) = lire_brut(jeu)? else { return Ok(None) };
    let v: DeploiementBrut = serde_json::from_slice(&octets).map_err(|_| Erreur::Refus("Le fichier de Vortex de ce jeu est illisible : Frogtend n'y touche pas.".into()))?;
    if v.methode != METHODE_LIENS {
        return Err(Erreur::Refus(motif_methode(&v.methode)));
    }
    if !crate::garde::meme(Path::new(v.cible.trim()), jeu) {
        return Err(Erreur::Refus("Le fichier de Vortex parle d'un autre dossier que celui de ce jeu : Frogtend n'y touche pas.".into()));
    }
    let depot = PathBuf::from(v.depot.trim());
    if !depot_sur(&depot, jeu) {
        return Err(Erreur::Refus(
            "Le dépôt des mods de Vortex est introuvable, mal placé (dans le jeu) ou passe par un lien : Frogtend ne peut rien prouver et ne touche à rien.".into(),
        ));
    }
    let fichiers = v
        .files
        .into_iter()
        .map(|f| match (rel_sur(&f.rel), source_sure(&f.source)) {
            (Some(rel), Some(source)) => EntreeVortex { rel, source, sur: true },
            _ => EntreeVortex { rel: f.rel, source: f.source, sur: false },
        })
        .collect();
    Ok(Some(Deploiement { depot, fichiers }))
}

/// Les fichiers listés par le manifeste de Vortex du jeu (chemins sûrs, en minuscules, séparés par `/`), lu sans rien
/// exiger d'autre : pour qu'une installation de mod par Frogtend ne prenne jamais un fichier de Vortex pour un
/// « original ». Vide s'il n'y a pas de manifeste lisible.
pub fn rels_listes(jeu: &Path) -> HashSet<String> {
    let Ok(Some(o)) = lire_brut(jeu) else { return HashSet::new() };
    serde_json::from_slice::<DeploiementBrut>(&o)
        .map(|v| v.files.iter().filter_map(|f| rel_sur(&f.rel)).map(|r| r.to_lowercase()).collect())
        .unwrap_or_default()
}

/// Le nom d'un mod pour l'écran : le nom de son dossier dans le dépôt de Vortex sans l'id Nexus, la version ni
/// l'horodatage (« ArchiveXL-4198-1-21-1-1737797101 » → « ArchiveXL »), ni « .zip ».
pub fn nom_du_mod(source: &str) -> String {
    let mut reste = source;
    // L'horodatage final (10 chiffres ou plus).
    if let Some(i) = reste.rfind('-') {
        let fin = &reste[i + 1..];
        if fin.len() >= 9 && fin.bytes().all(|b| b.is_ascii_digit()) {
            reste = &reste[..i];
        }
    }
    // Le premier « -<chiffres>- » (ou « -<chiffres> » en fin) : l'id Nexus, suivi de la version.
    let b = reste.as_bytes();
    let mut nom = reste;
    for i in 1..b.len() {
        if b[i] != b'-' {
            continue;
        }
        let chiffres = b[i + 1..].iter().take_while(|c| c.is_ascii_digit()).count();
        let apres = i + 1 + chiffres;
        if chiffres > 0 && (apres == b.len() || b[apres] == b'-') {
            nom = &reste[..i];
            break;
        }
    }
    let mut nom = nom.trim();
    for ext in [".zip", ".7z", ".rar"] {
        if nom.len() > ext.len() && nom.to_ascii_lowercase().ends_with(ext) {
            nom = nom[..nom.len() - ext.len()].trim();
        }
    }
    if nom.is_empty() { source.trim().to_string() } else { nom.to_string() }
}

// ---------------------------------------------------------------------------------------------------------------
// La preuve, fichier par fichier.
// ---------------------------------------------------------------------------------------------------------------

/// Pourquoi un fichier est laissé tel quel (jamais déplacé).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Motif {
    /// Le manifeste donne un chemin refusé (« ..\ », absolu, nom réservé…).
    CheminDangereux,
    /// Le même chemin listé deux fois.
    Double,
    /// Le fichier n'est plus dans le jeu.
    Absent,
    /// Un dossier, pas un fichier.
    PasUnFichier,
    /// Un lien symbolique ou une jonction sur son chemin.
    PointDeReanalyse,
    /// Le fichier n'est plus celui du dépôt (réécrit par le jeu ou sa boutique) : il ne mène plus nulle part.
    LienRompu,
    /// Son fichier du dépôt n'existe plus.
    DepotAbsent,
    /// Un seul lien : rien ne prouve qu'il vient de Vortex.
    UnSeulLien,
    /// Illisible (droits, disque).
    Illisible,
    /// Il a changé depuis la vérification.
    Change,
    /// Le retrait a été annulé avant son tour (jamais déplacé).
    Interrompu,
}

impl Motif {
    /// Un fichier qui « ne menait plus nulle part » (le lien vers le dépôt est rompu, ou le dépôt l'a perdu).
    pub fn rompu(self) -> bool {
        matches!(self, Motif::LienRompu | Motif::DepotAbsent)
    }
}

/// Prouve qu'un fichier du jeu est celui que Vortex a posé : listé (chemins sûrs), sans lien sur son chemin, un vrai
/// fichier, même volume et même index que son fichier du dépôt (présent), au moins 2 liens.
fn prouver(jeu: &Path, depot: &Path, e: &EntreeVortex) -> Result<Identite, Motif> {
    if !e.sur {
        return Err(Motif::CheminDangereux);
    }
    let cible = joindre(jeu, &e.rel);
    let ic = match identite(&cible) {
        Ok(i) => i,
        Err(x) if x.kind() == std::io::ErrorKind::NotFound => return Err(Motif::Absent),
        Err(_) => return Err(Motif::Illisible),
    };
    if ic.point_de_reanalyse || crate::garde::lien_entre(jeu, &cible) {
        return Err(Motif::PointDeReanalyse);
    }
    if ic.dossier {
        return Err(Motif::PasUnFichier);
    }
    let du_depot = joindre(&joindre(depot, &e.source), &e.rel);
    if crate::garde::lien_entre(depot, &du_depot) {
        return Err(Motif::PointDeReanalyse);
    }
    let id = identite(&du_depot).map_err(|_| Motif::DepotAbsent)?;
    if id.dossier || id.point_de_reanalyse {
        return Err(Motif::DepotAbsent);
    }
    if (ic.volume, ic.index) != (id.volume, id.index) {
        return Err(Motif::LienRompu);
    }
    if ic.liens < 2 {
        return Err(Motif::UnSeulLien);
    }
    Ok(ic)
}

// ---------------------------------------------------------------------------------------------------------------
// Le manifeste du retrait.
// ---------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EtatRetrait {
    EnCours,
    Fini,
    ToutRemis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EtatFichier {
    ADeplacer,
    Deplace,
    /// Rangé à part, mais pas reconnu à l'arrivée et son renvoi dans le jeu a échoué : proposé à la remise, jamais
    /// oublié.
    BloqueAPart,
    Remis,
    Laisse,
    /// Remis, puis effacé du jeu (par la personne) : il ne bloque plus son mod, qui sort de « Dans le jeu ».
    Parti,
}

impl EtatFichier {
    /// Rangé à part (donc remettable).
    pub fn a_part(self) -> bool {
        matches!(self, EtatFichier::Deplace | EtatFichier::BloqueAPart)
    }
}

/// Un fichier du retrait : où il était (`rel`, relatif au jeu), son mod (`source`), et son identité prouvée.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FichierRetire {
    pub rel: String,
    pub source: String,
    pub taille: u64,
    pub volume: u32,
    pub index: u64,
    pub etat: EtatFichier,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motif: Option<Motif>,
}

/// Le manifeste d'un retrait (`<jeu>\Frogtend\vortex\<horodatage>\retrait.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Retrait {
    pub version: u32,
    pub etat: EtatRetrait,
    pub commence_le: String,
    /// Le dépôt de Vortex au moment du retrait (jamais modifié ; relu pour reprendre).
    pub depot: String,
    pub fichiers: Vec<FichierRetire>,
    /// Les dossiers vidés de leur seul marqueur et retirés (relatifs au jeu).
    #[serde(default)]
    pub dossiers_retires: Vec<String>,
    /// `vortex.deployment.json` rangé dans le dossier du retrait.
    #[serde(default)]
    pub deploiement_range: bool,
    /// Les sauvegardes de Vortex trouvées à côté (laissées en place, jamais remises).
    #[serde(default)]
    pub sauvegardes_vortex: Vec<String>,
}

/// Le dossier des retraits d'un jeu.
pub fn dossier_retraits(jeu: &Path) -> PathBuf {
    jeu.join("Frogtend").join("vortex")
}

/// Écrit `retrait.json` dans `dossier` (à côté puis renommé : jamais à moitié écrit).
fn ecrire_atomique(dossier: &Path, texte: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let provisoire = dossier.join(format!("{NOM_RETRAIT}.ecriture"));
    {
        let mut o = std::fs::File::create(&provisoire)?;
        o.write_all(texte)?;
        o.sync_all()?;
    }
    std::fs::rename(&provisoire, dossier.join(NOM_RETRAIT))
}

/// Écrit le manifeste du retrait puis le relit (vérifier le résultat). Une copie est tenue à jour en même temps dans
/// `copie` (les données de Frogtend pour ce jeu, `copie\<horodatage>\retrait.json`) : si le dossier du jeu se perd ou
/// s'abîme, le retrait reste connu. La copie est faite au mieux : son échec n'arrête jamais un retrait.
fn ecrire_retrait(dossier: &Path, r: &Retrait, copie: Option<&Path>) -> Resultat<()> {
    let texte = serde_json::to_vec_pretty(r).map_err(|e| Erreur::Disque(format!("Le manifeste du retrait n'a pas pu être écrit ({e}).")))?;
    ecrire_atomique(dossier, &texte)?;
    if lire_retrait(dossier).as_ref() != Some(r) {
        return Err(Erreur::Disque("Le manifeste du retrait relu ne correspond pas à celui écrit : rien de plus n'est déplacé.".into()));
    }
    if let (Some(c), Some(nom)) = (copie, dossier.file_name()) {
        let d = c.join(nom);
        if !crate::garde::sous(&d, dossier) && std::fs::create_dir_all(&d).is_ok() {
            let _ = ecrire_atomique(&d, &texte);
        }
    }
    Ok(())
}

/// Un manifeste de retrait relu : chaque entrée encore à déplacer ou rangée doit avoir un chemin sûr (sinon elle est
/// tenue pour laissée).
fn lire_retrait(dossier: &Path) -> Option<Retrait> {
    let f = dossier.join(NOM_RETRAIT);
    std::fs::symlink_metadata(&f).ok().filter(|m| m.is_file() && m.len() <= TAILLE_MAX_MANIFESTE)?;
    let mut r: Retrait = serde_json::from_slice(&std::fs::read(&f).ok()?).ok()?;
    for x in &mut r.fichiers {
        if (x.etat == EtatFichier::ADeplacer || x.etat.a_part() || x.etat == EtatFichier::Remis) && (rel_sur(&x.rel).as_deref() != Some(x.rel.as_str()) || source_sure(&x.source).is_none()) {
            x.etat = EtatFichier::Laisse;
            x.motif = Some(Motif::CheminDangereux);
        }
    }
    Some(r)
}

/// Les retraits du jeu, du plus ancien au plus récent (dossiers nommés par leur horodatage, sans lien).
pub fn retraits(jeu: &Path) -> Vec<(PathBuf, Retrait)> {
    let racine = dossier_retraits(jeu);
    if crate::garde::lien_entre(jeu, &racine) {
        return Vec::new();
    }
    let mut l: Vec<(u64, PathBuf)> = std::fs::read_dir(&racine)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let nom = e.file_name().to_string_lossy().to_string();
            let n = nom.parse::<u64>().ok().filter(|_| nom.bytes().all(|b| b.is_ascii_digit()))?;
            let p = e.path();
            (std::fs::symlink_metadata(&p).is_ok_and(|m| m.is_dir()) && !crate::garde::est_un_lien(&p)).then_some((n, p))
        })
        .collect();
    l.sort();
    l.into_iter().filter_map(|(_, p)| lire_retrait(&p).map(|r| (p, r))).collect()
}

/// Le retrait interrompu du jeu, s'il y en a un.
pub fn en_cours(jeu: &Path) -> Option<(PathBuf, Retrait)> {
    retraits(jeu).into_iter().rev().find(|(_, r)| r.etat == EtatRetrait::EnCours)
}

/// Ce qu'un retrait a fait (pour l'écran et le journal : des nombres seulement).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct BilanRetrait {
    pub deplaces: usize,
    pub laisses: usize,
    /// Parmi les laissés : ceux qui « ne menaient plus nulle part ».
    pub laisses_rompus: usize,
    /// Les mods dont au moins un fichier a été rangé.
    pub mods: usize,
    pub octets: u64,
    pub dossiers_retires: usize,
    /// Les sauvegardes de Vortex (l'original du jeu) à côté d'un fichier RANGÉ : à dire (à côté d'un laissé : rien).
    pub sauvegardes_vortex: usize,
    pub deploiement_range: bool,
}

fn bilan_de(r: &Retrait) -> BilanRetrait {
    let laisses: Vec<&FichierRetire> = r.fichiers.iter().filter(|f| f.etat == EtatFichier::Laisse).collect();
    let deplaces: Vec<&FichierRetire> = r.fichiers.iter().filter(|f| f.etat.a_part()).collect();
    let ranges: HashSet<String> = deplaces.iter().map(|f| f.rel.to_lowercase()).collect();
    BilanRetrait {
        deplaces: deplaces.len(),
        laisses: laisses.len(),
        laisses_rompus: laisses.iter().filter(|f| f.motif.is_some_and(Motif::rompu)).count(),
        mods: deplaces.iter().map(|f| f.source.to_lowercase()).collect::<HashSet<_>>().len(),
        octets: deplaces.iter().map(|f| f.taille).sum(),
        dossiers_retires: r.dossiers_retires.len(),
        sauvegardes_vortex: r
            .sauvegardes_vortex
            .iter()
            .filter_map(|s| s.strip_suffix(SUFFIXE_SAUVEGARDE))
            .filter(|base| ranges.contains(&base.to_lowercase()))
            .count(),
        deploiement_range: r.deploiement_range,
    }
}

/// Un nom de dossier de retrait libre à partir de `quand` (secondes) : jamais un dossier déjà là.
fn nom_libre(racine: &Path, quand: &str) -> String {
    let mut n: u64 = quand.parse().unwrap_or(0);
    while racine.join(n.to_string()).exists() {
        n += 1;
    }
    n.to_string()
}

/// Crée `dossier` sous `racine` sans jamais passer par un lien : le plus proche parent qui existe déjà est vérifié
/// AVANT la création, puis le dossier créé.
fn creer_dossiers(racine: &Path, dossier: &Path) -> bool {
    let Some(existant) = dossier.ancestors().find(|a| std::fs::symlink_metadata(a).is_ok()) else { return false };
    if !crate::garde::sous(existant, racine) || crate::garde::lien_entre(racine, existant) {
        return false;
    }
    std::fs::create_dir_all(dossier).is_ok() && !crate::garde::lien_entre(racine, dossier)
}

/// Retire les mods de Vortex du jeu : chaque fichier prouvé est rangé sous `<jeu>\Frogtend\vortex\<quand>\fichiers\`,
/// les autres sont laissés tels quels. `copie` : où garder une copie du manifeste du retrait (données de Frogtend).
/// `progres("retrait", faits, total)` en fichiers.
pub fn retirer(jeu: &Path, copie: Option<&Path>, quand: &str, progres: &(dyn Fn(&'static str, u64, u64) + Sync)) -> Resultat<BilanRetrait> {
    retirer_avec(jeu, copie, quand, progres, None)
}

/// Le plan d'un retrait : chaque fichier listé, prouvé (à déplacer) ou laissé avec son motif.
/// `progres("verification", faits, total)` en fichiers aux chemins sûrs (ceux que l'écran compte par mod) : la preuve
/// de 2 015 fichiers prend quelques secondes, l'écran le montre dès le premier (0 sur N) jusqu'au dernier.
fn planifier(jeu: &Path, d: &Deploiement, progres: &(dyn Fn(&'static str, u64, u64) + Sync)) -> Vec<FichierRetire> {
    let mut vus = HashSet::new();
    let total = d.fichiers.iter().filter(|e| e.sur).count() as u64;
    let mut faits = 0u64;
    progres("verification", 0, total);
    d.fichiers
        .iter()
        .map(|e| {
            let preuve = if e.sur && !vus.insert(e.rel.to_lowercase()) { Err(Motif::Double) } else { prouver(jeu, &d.depot, e) };
            if e.sur {
                faits += 1;
                progres("verification", faits, total);
            }
            match preuve {
                Ok(i) => FichierRetire { rel: e.rel.clone(), source: e.source.clone(), taille: i.taille, volume: i.volume, index: i.index, etat: EtatFichier::ADeplacer, motif: None },
                Err(m) => FichierRetire { rel: e.rel.clone(), source: e.source.clone(), taille: 0, volume: 0, index: 0, etat: EtatFichier::Laisse, motif: Some(m) },
            }
        })
        .collect()
}

/// `retirer`, arrêté net après `arret` déplacements (les essais d'une coupure).
fn retirer_avec(jeu: &Path, copie: Option<&Path>, quand: &str, progres: &(dyn Fn(&'static str, u64, u64) + Sync), arret: Option<usize>) -> Resultat<BilanRetrait> {
    if en_cours(jeu).is_some() {
        return Err(Erreur::Refus("Un retrait des mods de Vortex a été interrompu : reprends-le, ou remets tout comme avant.".into()));
    }
    let d = lire_deploiement(jeu)?.ok_or_else(|| Erreur::Refus("Ce jeu n'a pas de mods posés par Vortex.".into()))?;
    // Le plan : chaque fichier prouvé AVANT le premier déplacement (un refus ne coûte rien).
    let fichiers = planifier(jeu, &d, progres);
    if !fichiers.iter().any(|f| f.etat == EtatFichier::ADeplacer) {
        return Err(Erreur::Refus("Aucun fichier de ce jeu n'est prouvé venir de Vortex : rien n'est déplacé.".into()));
    }
    // Les sauvegardes de Vortex (un original qu'il avait remplacé) : listées, laissées en place, jamais remises.
    let sauvegardes_vortex: Vec<String> = d
        .fichiers
        .iter()
        .filter(|e| e.sur)
        .map(|e| format!("{}{SUFFIXE_SAUVEGARDE}", e.rel))
        .filter(|r| std::fs::symlink_metadata(joindre(jeu, r)).is_ok_and(|m| m.is_file()))
        .collect();
    // Le dossier du retrait : dans le jeu, sans lien, sur le même volume que le jeu (un renommage, jamais une copie).
    let racine = dossier_retraits(jeu);
    if !creer_dossiers(jeu, &racine) {
        return Err(Erreur::Refus("Le dossier de Frogtend dans ce jeu passe par un lien : rien n'est déplacé.".into()));
    }
    let dossier = racine.join(nom_libre(&racine, quand));
    std::fs::create_dir(&dossier)?;
    std::fs::create_dir(dossier.join("fichiers"))?;
    let meme_volume = matches!((identite(jeu), identite(&dossier)), (Ok(a), Ok(b)) if a.volume == b.volume);
    if !meme_volume || crate::garde::lien_entre(jeu, &dossier) {
        let _ = std::fs::remove_dir(dossier.join("fichiers"));
        let _ = std::fs::remove_dir(&dossier);
        return Err(Erreur::Refus("Le dossier de Frogtend dans ce jeu n'est pas sur le même disque que le jeu : rien n'est déplacé.".into()));
    }
    let mut r = Retrait {
        version: 1,
        etat: EtatRetrait::EnCours,
        commence_le: quand.into(),
        depot: d.depot.to_string_lossy().into(),
        fichiers,
        dossiers_retires: Vec::new(),
        deploiement_range: false,
        sauvegardes_vortex,
    };
    // Le manifeste du retrait AVANT le premier déplacement.
    ecrire_retrait(&dossier, &r, copie)?;
    poursuivre(jeu, &dossier, &mut r, copie, progres, arret)
}

/// Reprend le retrait interrompu du jeu (après une coupure).
pub fn reprendre(jeu: &Path, copie: Option<&Path>, progres: &(dyn Fn(&'static str, u64, u64) + Sync)) -> Resultat<BilanRetrait> {
    let (dossier, mut r) = en_cours(jeu).ok_or_else(|| Erreur::Refus("Aucun retrait des mods de Vortex n'attend d'être repris.".into()))?;
    poursuivre(jeu, &dossier, &mut r, copie, progres, None)
}

/// Range UN fichier encore à déplacer, après l'avoir prouvé de nouveau (il doit être le même qu'au plan) ; une coupure
/// juste après son déplacement est reconnue (il est déjà rangé, à l'identique). Rend `Deplace`, ou `BloqueAPart` si le
/// fichier arrivé n'est pas reconnu et n'a pas pu revenir dans le jeu.
fn ranger_un(jeu: &Path, depot: &Path, fichiers: &Path, f: &FichierRetire) -> Result<EtatFichier, Motif> {
    let origine = joindre(jeu, &f.rel);
    let arrivee = joindre(fichiers, &f.rel);
    let meme = |i: &Identite| (i.volume, i.index) == (f.volume, f.index);
    let e = EntreeVortex { rel: f.rel.clone(), source: f.source.clone(), sur: true };
    match prouver(jeu, depot, &e) {
        Ok(i) if meme(&i) => {}
        Ok(_) => return Err(Motif::Change),
        Err(Motif::Absent) => {
            // Déjà rangé (coupure entre le déplacement et l'écriture du manifeste) ?
            return match identite(&arrivee) {
                Ok(a) if meme(&a) && !a.dossier => Ok(EtatFichier::Deplace),
                _ => Err(Motif::Absent),
            };
        }
        Err(m) => return Err(m),
    }
    let parent = arrivee.parent().ok_or(Motif::Illisible)?;
    if !creer_dossiers(jeu, parent) {
        return Err(Motif::PointDeReanalyse);
    }
    deplacer(&origine, &arrivee).map_err(|_| Motif::Illisible)?;
    // Vérifier le résultat : le fichier rangé est bien le même (sinon il revient à sa place).
    match identite(&arrivee) {
        Ok(a) if meme(&a) => Ok(EtatFichier::Deplace),
        _ if deplacer(&arrivee, &origine).is_ok() => Err(Motif::Change),
        // Il n'a pas pu revenir : il reste à part, et la remise le propose.
        _ => Ok(EtatFichier::BloqueAPart),
    }
}

/// Les dossiers (relatifs au jeu, au-dessus des fichiers rangés) qui ne contiennent plus que le marqueur de Vortex
/// (texte exact) : le marqueur puis le dossier sont retirés, du plus profond au moins profond. Jamais le jeu lui-même,
/// jamais un dossier qui contient autre chose (données des mods, réglages de la personne).
fn retirer_marqueurs(jeu: &Path, rels: &[&str]) -> Vec<String> {
    let mut dossiers: HashMap<String, String> = HashMap::new();
    for r in rels {
        let mut d = *r;
        while let Some(i) = d.rfind('/') {
            d = &d[..i];
            dossiers.entry(d.to_lowercase()).or_insert_with(|| d.to_string());
        }
    }
    let mut l: Vec<String> = dossiers.into_values().collect();
    l.sort_by(|a, b| b.matches('/').count().cmp(&a.matches('/').count()).then_with(|| a.cmp(b)));
    let mut retires = Vec::new();
    for d in l {
        let p = joindre(jeu, &d);
        if crate::garde::lien_entre(jeu, &p) || !marqueur_seul(&p) {
            continue;
        }
        let m = p.join(MARQUEUR);
        if std::fs::remove_file(&m).is_err() {
            continue;
        }
        if std::fs::remove_dir(&p).is_ok() {
            retires.push(d);
        } else {
            // Quelque chose est apparu entre-temps : le marqueur revient, le dossier reste.
            let _ = std::fs::write(&m, TEXTE_MARQUEUR);
        }
    }
    retires
}

/// Le dossier ne contient QUE le marqueur de Vortex, vrai fichier au texte exact.
fn marqueur_seul(p: &Path) -> bool {
    let Ok(l) = std::fs::read_dir(p) else { return false };
    let l: Vec<_> = l.flatten().take(2).collect();
    let [e] = l.as_slice() else { return false };
    let m = e.path();
    e.file_name().to_string_lossy() == MARQUEUR
        && std::fs::symlink_metadata(&m).is_ok_and(|x| x.is_file() && x.len() == TEXTE_MARQUEUR.len() as u64)
        && !crate::garde::est_un_lien(&m)
        && std::fs::read(&m).is_ok_and(|t| t == TEXTE_MARQUEUR)
}

/// La suite d'un retrait (un nouveau, ou un interrompu) : les fichiers encore à déplacer, puis les marqueurs, puis
/// `vortex.deployment.json` EN DERNIER.
fn poursuivre(jeu: &Path, dossier: &Path, r: &mut Retrait, copie: Option<&Path>, progres: &(dyn Fn(&'static str, u64, u64) + Sync), arret: Option<usize>) -> Resultat<BilanRetrait> {
    let depot = PathBuf::from(&r.depot);
    if !depot_sur(&depot, jeu) {
        return Err(Erreur::Refus("Le dépôt des mods de Vortex est introuvable ou mal placé : rien de plus n'est déplacé.".into()));
    }
    let fichiers = dossier.join("fichiers");
    if !creer_dossiers(jeu, &fichiers) {
        return Err(Erreur::Refus("Le dossier du retrait passe par un lien : rien de plus n'est déplacé.".into()));
    }
    // Compté en fichiers PROUVÉS (ceux que la confirmation annonce : « 0 fichier sur 2 014 », jamais les laissés).
    let total = r.fichiers.iter().filter(|f| f.etat != EtatFichier::Laisse).count() as u64;
    let mut faits = r.fichiers.iter().filter(|f| f.etat != EtatFichier::Laisse && f.etat != EtatFichier::ADeplacer).count() as u64;
    progres("retrait", faits, total);
    let (mut ici, mut depuis_ecriture) = (0usize, 0usize);
    for k in 0..r.fichiers.len() {
        if r.fichiers[k].etat != EtatFichier::ADeplacer {
            continue;
        }
        if arret.is_some_and(|n| ici >= n) {
            ecrire_retrait(dossier, r, copie)?;
            return Err(Erreur::Disque("Retrait arrêté net (essai d'une coupure).".into()));
        }
        match ranger_un(jeu, &depot, &fichiers, &r.fichiers[k]) {
            Ok(e) => {
                r.fichiers[k].etat = e;
                r.fichiers[k].motif = (e == EtatFichier::BloqueAPart).then_some(Motif::Change);
            }
            Err(m) => {
                r.fichiers[k].etat = EtatFichier::Laisse;
                r.fichiers[k].motif = Some(m);
            }
        }
        ici += 1;
        depuis_ecriture += 1;
        faits = (faits + 1).min(total);
        progres("retrait", faits, total);
        if depuis_ecriture >= LOT {
            ecrire_retrait(dossier, r, copie)?;
            depuis_ecriture = 0;
        }
    }
    ecrire_retrait(dossier, r, copie)?;
    // Les marqueurs des dossiers vidés.
    let rels: Vec<&str> = r.fichiers.iter().filter(|f| f.etat.a_part()).map(|f| f.rel.as_str()).collect();
    let retires = retirer_marqueurs(jeu, &rels);
    for d in retires {
        if !r.dossiers_retires.iter().any(|x| x.eq_ignore_ascii_case(&d)) {
            r.dossiers_retires.push(d);
        }
    }
    // `vortex.deployment.json` en dernier : tous les fichiers prouvés sont rangés.
    let (dans_le_jeu, range) = (jeu.join(MANIFESTE_VORTEX), dossier.join(MANIFESTE_VORTEX));
    if std::fs::symlink_metadata(&dans_le_jeu).is_ok_and(|m| m.is_file()) && !range.exists() {
        deplacer(&dans_le_jeu, &range)?;
    }
    r.deploiement_range = range.is_file() && !dans_le_jeu.exists();
    r.etat = EtatRetrait::Fini;
    ecrire_retrait(dossier, r, copie)?;
    Ok(bilan_de(r))
}

/// Ce qu'une remise a fait.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct BilanRemise {
    pub remis: usize,
    /// Laissés rangés : leur place dans le jeu est prise par un autre fichier.
    pub places_prises: usize,
    /// Laissés rangés : ils ont changé depuis le retrait.
    pub changes: usize,
    /// Laissés rangés : Windows a refusé le renommage pour une autre raison (droits, fichier ouvert…).
    #[serde(default)]
    pub erreurs: usize,
    pub deploiement_remis: bool,
}

/// Pourquoi un fichier n'a pas bougé (remise, ou nouveau rangement d'un fichier remis).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Echec {
    /// Sa place d'arrivée est prise (vue avant, ou refus « existe déjà » de Windows).
    PlacePrise,
    /// Ce n'est plus le fichier noté (changé, remplacé, lien ou jonction sur son chemin).
    Change,
    /// Il n'est plus là du tout (effacé par la personne).
    Parti,
    /// Windows a refusé le renommage pour une autre raison.
    Erreur,
}

/// Un refus de renommage de Windows : « place prise » seulement pour ERROR_ALREADY_EXISTS (183) et ERROR_FILE_EXISTS
/// (80) ; tout autre refus est une erreur, comptée à part.
fn echec_du_renommage(e: &std::io::Error) -> Echec {
    if matches!(e.raw_os_error(), Some(183) | Some(80)) || e.kind() == std::io::ErrorKind::AlreadyExists {
        Echec::PlacePrise
    } else {
        Echec::Erreur
    }
}

/// Après une coupure, le manifeste du retrait peut être en retard (il n'est réécrit que tous les `LOT` fichiers) : un
/// fichier noté « à déplacer » (ou laissé parce qu'il avait « changé », ou « remis ») qui n'est plus dans le jeu, et dont
/// l'identité à l'arrivée est celle notée, est bel et bien rangé. Un fichier « remis » qui n'est plus ni dans le jeu ni
/// rangé a été effacé : il est noté « parti » (il ne bloque plus son mod). Rend vrai si quelque chose a changé.
fn recaler(jeu: &Path, dossier: &Path, r: &mut Retrait) -> bool {
    let fichiers = dossier.join("fichiers");
    let mut change = false;
    for f in &mut r.fichiers {
        let candidat = f.etat == EtatFichier::ADeplacer || f.etat == EtatFichier::Remis || (f.etat == EtatFichier::Laisse && f.motif == Some(Motif::Change));
        if !candidat || f.index == 0 || rel_sur(&f.rel).as_deref() != Some(f.rel.as_str()) {
            continue;
        }
        match std::fs::symlink_metadata(joindre(jeu, &f.rel)) {
            Ok(_) => continue,
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => continue,
            Err(_) => {}
        }
        if matches!(identite(&joindre(&fichiers, &f.rel)), Ok(i) if (i.volume, i.index) == (f.volume, f.index) && !i.dossier) {
            f.etat = EtatFichier::Deplace;
            f.motif = None;
            change = true;
        } else if f.etat == EtatFichier::Remis && std::fs::symlink_metadata(joindre(&fichiers, &f.rel)).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound) {
            f.etat = EtatFichier::Parti;
            f.motif = Some(Motif::Absent);
            change = true;
        }
    }
    change
}

/// Remet les fichiers `idx` d'un retrait à leur place : seulement si la place est libre et si le fichier rangé est
/// toujours le même (même index ; un fichier « bloqué à part » revient tel quel) ; les dossiers manquants sont recréés
/// (jamais les marqueurs). `faits`/`total` : la progression GLOBALE (plusieurs retraits).
fn remettre_fichiers(jeu: &Path, dossier: &Path, r: &mut Retrait, idx: &[usize], progres: &(dyn Fn(&'static str, u64, u64) + Sync), faits: &mut u64, total: u64, b: &mut BilanRemise) {
    let fichiers = dossier.join("fichiers");
    for &k in idx {
        let f = r.fichiers[k].clone();
        let (origine, range) = (joindre(jeu, &f.rel), joindre(&fichiers, &f.rel));
        let issue = (|| {
            if std::fs::symlink_metadata(&origine).is_ok() {
                return Err(Echec::PlacePrise);
            }
            match identite(&range) {
                Ok(i) if !i.dossier && !i.point_de_reanalyse && !crate::garde::lien_entre(&fichiers, &range) && (f.etat == EtatFichier::BloqueAPart || (i.volume, i.index) == (f.volume, f.index)) => {}
                _ => return Err(Echec::Change),
            }
            let parent = origine.parent().ok_or(Echec::Change)?;
            if !creer_dossiers(jeu, parent) {
                return Err(Echec::Change);
            }
            // Jamais par-dessus : si la place vient d'être prise, le déplacement échoue.
            deplacer(&range, &origine).map_err(|e| echec_du_renommage(&e))
        })();
        match issue {
            Ok(()) => {
                // Le fichier est revenu : l'état le dit, quoi que donne une relecture.
                r.fichiers[k].etat = EtatFichier::Remis;
                r.fichiers[k].motif = None;
                b.remis += 1;
                // Les dossiers vides du rangement (à Frogtend) : retirés, jamais plus haut que `fichiers`.
                let mut p = range.parent().map(Path::to_path_buf);
                while let Some(d) = p.filter(|d| crate::garde::sous(d, &fichiers) && !crate::garde::meme(d, &fichiers)) {
                    if std::fs::remove_dir(&d).is_err() {
                        break;
                    }
                    p = d.parent().map(Path::to_path_buf);
                }
            }
            Err(Echec::PlacePrise) => b.places_prises += 1,
            Err(Echec::Erreur) => b.erreurs += 1,
            Err(_) => b.changes += 1,
        }
        *faits = (*faits + 1).min(total);
        progres("remise", *faits, total);
    }
}

/// Le dépôt de Vortex tient-il encore ce retrait ? Un vrai dossier sûr, ET au moins un fichier noté qui s'y prouve
/// encore (même volume et même index) : sinon le manifeste de Vortex ne mènerait nulle part.
fn depot_tient(jeu: &Path, r: &Retrait) -> bool {
    let depot = PathBuf::from(&r.depot);
    depot_sur(&depot, jeu)
        && r.fichiers.iter().filter(|f| f.index != 0 && rel_sur(&f.rel).is_some() && source_sure(&f.source).is_some()).any(|f| {
            let p = joindre(&joindre(&depot, &f.source), &f.rel);
            !crate::garde::lien_entre(&depot, &p) && matches!(identite(&p), Ok(i) if (i.volume, i.index) == (f.volume, f.index) && !i.dossier)
        })
}

/// Un retrait dont plus aucun fichier n'est à part : `vortex.deployment.json` revient (si sa place est libre) et le
/// retrait est « tout remis ». Sinon il reste « fini » (des mods restent remettables).
fn finir_si_entier(jeu: &Path, dossier: &Path, r: &mut Retrait, b: &mut BilanRemise) {
    if r.fichiers.iter().any(|f| f.etat.a_part()) {
        r.etat = EtatRetrait::Fini;
        return;
    }
    let (dans_le_jeu, range) = (jeu.join(MANIFESTE_VORTEX), dossier.join(MANIFESTE_VORTEX));
    // Le dépôt de Vortex effacé (Vortex désinstallé, 25/11) : son manifeste ne revient pas — il ne mènerait plus
    // nulle part, et l'écran ne verrait plus ces mods (« dépôt introuvable ») ; ils restent « dans le jeu », retirables.
    if depot_tient(jeu, r) && std::fs::symlink_metadata(&range).is_ok_and(|m| m.is_file()) && deplacer(&range, &dans_le_jeu).is_ok() {
        b.deploiement_remis = true;
    }
    r.deploiement_range = range.is_file();
    r.etat = if r.deploiement_range { EtatRetrait::Fini } else { EtatRetrait::ToutRemis };
}

/// Les retraits pas encore « tout remis » (du plus récent au plus ancien), recalés ; un retrait que le recalage a changé
/// est réécrit tout de suite (une coupure ne laisse jamais un mod coincé). Refus si un retrait est interrompu.
fn retraits_recales(jeu: &Path, copie: Option<&Path>) -> Resultat<Vec<(PathBuf, Retrait)>> {
    let liste = retraits(jeu);
    if liste.iter().any(|(_, r)| r.etat == EtatRetrait::EnCours) {
        return Err(Erreur::Refus("Le retrait des mods a été interrompu : reprends-le, ou remets tout comme avant.".into()));
    }
    let mut l = Vec::new();
    for (d, mut r) in liste.into_iter().rev().filter(|(_, r)| r.etat != EtatRetrait::ToutRemis) {
        if recaler(jeu, &d, &mut r) {
            ecrire_retrait(&d, &r, copie)?;
        }
        l.push((d, r));
    }
    Ok(l)
}

/// Remet UN mod (`source`) dans le jeu, depuis tous les retraits où il est rangé ; un retrait qui n'a plus rien à part
/// rend aussi `vortex.deployment.json`. La progression compte tous les retraits ensemble.
pub fn remettre_mod(jeu: &Path, copie: Option<&Path>, source: &str, progres: &(dyn Fn(&'static str, u64, u64) + Sync)) -> Resultat<BilanRemise> {
    let liste: Vec<(PathBuf, Retrait, Vec<usize>)> = retraits_recales(jeu, copie)?
        .into_iter()
        .map(|(d, r)| {
            let idx = (0..r.fichiers.len()).filter(|&k| r.fichiers[k].etat.a_part() && r.fichiers[k].source.eq_ignore_ascii_case(source)).collect();
            (d, r, idx)
        })
        .filter(|(_, _, idx): &(PathBuf, Retrait, Vec<usize>)| !idx.is_empty())
        .collect();
    if liste.is_empty() {
        return Err(Erreur::Introuvable("Ce mod n'est pas parmi les mods retirés.".into()));
    }
    let total = liste.iter().map(|(_, _, idx)| idx.len() as u64).sum::<u64>();
    let mut faits = 0;
    progres("remise", 0, total);
    let mut b = BilanRemise::default();
    for (dossier, mut r, idx) in liste {
        remettre_fichiers(jeu, &dossier, &mut r, &idx, progres, &mut faits, total, &mut b);
        finir_si_entier(jeu, &dossier, &mut r, &mut b);
        ecrire_retrait(&dossier, &r, copie)?;
    }
    progres("remise", total, total);
    Ok(b)
}

/// Ce que le nouveau rangement d'un ou de tous les mods remis a fait.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct BilanRetraitMod {
    /// Rangés de nouveau à part (et reconnus à l'arrivée).
    pub ranges: usize,
    /// Laissés dans le jeu : ils ne sont plus ceux que Frogtend avait remis (changés, remplacés, lien sur le chemin).
    pub changes: usize,
    /// Effacés depuis la remise : notés « partis », ils ne bloquent plus le mod.
    pub partis: usize,
    /// Laissés dans le jeu : leur place de rangement est prise.
    pub places_prises: usize,
    /// Laissés dans le jeu : Windows a refusé le renommage pour une autre raison.
    pub erreurs: usize,
    /// Partis à part mais pas reconnus à l'arrivée, et pas revenus : « coincés à part », proposés à la remise.
    pub coinces: usize,
    /// Les mods dont au moins un fichier a été rangé.
    pub mods: usize,
}

/// Range de nouveau UN fichier remis : il doit être EXACTEMENT celui que Frogtend a remis (même volume et même index
/// que notés au retrait), un vrai fichier sans lien sur son chemin, et sa place de rangement libre. Ni le dépôt de
/// Vortex ni un second lien ne sont exigés (✅ Seb 25/11 : Vortex désinstallé, son dépôt effacé — le dossier du retrait
/// est alors la SEULE copie des mods). `lire` et `renvoyer` : la relecture à l'arrivée et le retour en arrière (les
/// vrais en service ; simulés par les essais du cas « coincé à part »).
fn reranger_un_avec(
    jeu: &Path,
    fichiers: &Path,
    f: &FichierRetire,
    lire: &dyn Fn(&Path) -> std::io::Result<Identite>,
    renvoyer: &dyn Fn(&Path, &Path) -> std::io::Result<()>,
) -> Result<EtatFichier, Echec> {
    if rel_sur(&f.rel).as_deref() != Some(f.rel.as_str()) || f.index == 0 {
        return Err(Echec::Change);
    }
    let (origine, arrivee) = (joindre(jeu, &f.rel), joindre(fichiers, &f.rel));
    let meme = |i: &Identite| (i.volume, i.index) == (f.volume, f.index) && !i.dossier && !i.point_de_reanalyse;
    if crate::garde::lien_entre(jeu, &origine) {
        return Err(Echec::Change);
    }
    match identite(&origine) {
        Ok(i) if meme(&i) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(Echec::Parti),
        _ => return Err(Echec::Change),
    }
    if std::fs::symlink_metadata(&arrivee).is_ok() {
        return Err(Echec::PlacePrise);
    }
    let parent = arrivee.parent().ok_or(Echec::Change)?;
    if crate::garde::lien_entre(jeu, fichiers) || !creer_dossiers(jeu, parent) || crate::garde::lien_entre(fichiers, parent) {
        return Err(Echec::Change);
    }
    // Jamais par-dessus : si la place vient d'être prise, le déplacement échoue.
    deplacer(&origine, &arrivee).map_err(|e| echec_du_renommage(&e))?;
    match lire(&arrivee) {
        Ok(a) if meme(&a) => Ok(EtatFichier::Deplace),
        _ if renvoyer(&arrivee, &origine).is_ok() => Err(Echec::Change),
        // Il n'a pas pu revenir : il reste à part, et la remise le propose.
        _ => Ok(EtatFichier::BloqueAPart),
    }
}

fn reranger_un(jeu: &Path, fichiers: &Path, f: &FichierRetire) -> Result<EtatFichier, Echec> {
    reranger_un_avec(jeu, fichiers, f, &identite, &deplacer)
}

/// Compte l'issue d'un nouveau rangement dans le bilan, et met l'état du fichier à jour.
fn noter_rangement(f: &mut FichierRetire, issue: Result<EtatFichier, Echec>, b: &mut BilanRetraitMod) -> bool {
    match issue {
        Ok(EtatFichier::BloqueAPart) => {
            f.etat = EtatFichier::BloqueAPart;
            f.motif = Some(Motif::Change);
            b.coinces += 1;
            false
        }
        Ok(e) => {
            f.etat = e;
            f.motif = None;
            b.ranges += 1;
            true
        }
        Err(Echec::Parti) => {
            f.etat = EtatFichier::Parti;
            f.motif = Some(Motif::Absent);
            b.partis += 1;
            false
        }
        Err(Echec::PlacePrise) => {
            b.places_prises += 1;
            false
        }
        Err(Echec::Erreur) => {
            b.erreurs += 1;
            false
        }
        Err(Echec::Change) => {
            b.changes += 1;
            false
        }
    }
}

/// Range de nouveau les fichiers « remis » choisis par `garder` (un mod, ou tous), chacun dans le dossier de SON
/// retrait, par renommage, après la preuve de `reranger_un`. Rien n'est effacé.
fn reranger(jeu: &Path, copie: Option<&Path>, garder: &dyn Fn(&FichierRetire) -> bool, progres: &(dyn Fn(&'static str, u64, u64) + Sync)) -> Resultat<Option<BilanRetraitMod>> {
    let liste: Vec<(PathBuf, Retrait, Vec<usize>)> = retraits_recales(jeu, copie)?
        .into_iter()
        .map(|(d, r)| {
            let idx = (0..r.fichiers.len()).filter(|&k| r.fichiers[k].etat == EtatFichier::Remis && garder(&r.fichiers[k])).collect();
            (d, r, idx)
        })
        .filter(|(_, _, idx): &(PathBuf, Retrait, Vec<usize>)| !idx.is_empty())
        .collect();
    if liste.is_empty() {
        return Ok(None);
    }
    let total = liste.iter().map(|(_, _, idx)| idx.len() as u64).sum::<u64>();
    let mut faits = 0u64;
    progres("retrait", 0, total);
    let mut b = BilanRetraitMod::default();
    let mut mods: HashSet<String> = HashSet::new();
    for (dossier, mut r, idx) in liste {
        let fichiers = dossier.join("fichiers");
        if !creer_dossiers(jeu, &fichiers) {
            return Err(Erreur::Refus("Le dossier du retrait passe par un lien : rien de plus n'est déplacé.".into()));
        }
        for k in idx {
            let issue = reranger_un(jeu, &fichiers, &r.fichiers[k]);
            if noter_rangement(&mut r.fichiers[k], issue, &mut b) {
                mods.insert(r.fichiers[k].source.to_lowercase());
            }
            faits = (faits + 1).min(total);
            progres("retrait", faits, total);
        }
        ecrire_retrait(&dossier, &r, copie)?;
    }
    b.mods = mods.len();
    progres("retrait", total, total);
    Ok(Some(b))
}

/// 🧹 Range de nouveau UN mod remis (`source`) : chaque fichier « remis » de chaque retrait retourne dans le dossier
/// de CE retrait ; son état redevient « déplacé ». Un fichier qui n'est plus celui remis, ou dont la place est prise,
/// reste dans le jeu (compté) ; un fichier effacé est noté « parti ». Rien n'est effacé.
pub fn retirer_mod(jeu: &Path, copie: Option<&Path>, source: &str, progres: &(dyn Fn(&'static str, u64, u64) + Sync)) -> Resultat<BilanRetraitMod> {
    reranger(jeu, copie, &|f| f.source.eq_ignore_ascii_case(source), progres)?.ok_or_else(|| Erreur::Introuvable("Ce mod n'est pas parmi les mods remis dans le jeu.".into()))
}

/// 🧹 « Tout ranger de nouveau » : chaque mod remis, avec la même preuve (utile sans le dépôt de Vortex, quand son
/// manifeste n'est plus là pour un retrait global).
pub fn tout_reranger(jeu: &Path, copie: Option<&Path>, progres: &(dyn Fn(&'static str, u64, u64) + Sync)) -> Resultat<BilanRetraitMod> {
    reranger(jeu, copie, &|_| true, progres)?.ok_or_else(|| Erreur::Refus("Aucun mod remis à ranger de nouveau.".into()))
}

/// Les fichiers REMIS par Frogtend (chemins relatifs au jeu, en minuscules, `/`) : un mod Frogtend ne se pose jamais
/// par-dessus (sans le dépôt de Vortex, ils n'ont plus qu'un lien et ne sont plus listés par aucun manifeste).
pub fn rels_remis(jeu: &Path) -> HashSet<String> {
    retraits(jeu)
        .into_iter()
        .filter(|(_, r)| r.etat != EtatRetrait::ToutRemis)
        .flat_map(|(_, r)| r.fichiers.into_iter().filter(|f| f.etat == EtatFichier::Remis).map(|f| f.rel.to_lowercase()))
        .collect()
}

/// « Tout remettre comme avant » : chaque retrait pas encore tout remis, du plus récent au plus ancien ; chacun rend
/// ses fichiers, puis son `vortex.deployment.json` seulement s'il est entier et la place libre (sinon Vortex croirait
/// ses fichiers en place). Un retrait interrompu s'annule ainsi. La progression compte tous les retraits ensemble.
pub fn tout_remettre(jeu: &Path, copie: Option<&Path>, progres: &(dyn Fn(&'static str, u64, u64) + Sync)) -> Resultat<BilanRemise> {
    let mut liste: Vec<(PathBuf, Retrait)> = retraits(jeu).into_iter().rev().filter(|(_, r)| r.etat != EtatRetrait::ToutRemis).collect();
    if liste.is_empty() {
        return Err(Erreur::Refus("Aucun mod retiré à remettre.".into()));
    }
    for (dossier, r) in &mut liste {
        recaler(jeu, dossier, r);
        // Ceux qu'un retrait interrompu n'avait pas encore déplacés sont restés dans le jeu : rien à remettre.
        for f in &mut r.fichiers {
            if f.etat == EtatFichier::ADeplacer {
                f.etat = EtatFichier::Laisse;
                f.motif = Some(Motif::Interrompu);
            }
        }
    }
    let total = liste.iter().map(|(_, r)| r.fichiers.iter().filter(|f| f.etat.a_part()).count() as u64).sum::<u64>();
    let mut faits = 0;
    progres("remise", 0, total);
    let mut b = BilanRemise::default();
    for (dossier, mut r) in liste {
        let idx: Vec<usize> = (0..r.fichiers.len()).filter(|&k| r.fichiers[k].etat.a_part()).collect();
        remettre_fichiers(jeu, &dossier, &mut r, &idx, progres, &mut faits, total, &mut b);
        finir_si_entier(jeu, &dossier, &mut r, &mut b);
        ecrire_retrait(&dossier, &r, copie)?;
    }
    progres("remise", total, total);
    Ok(b)
}

// ---------------------------------------------------------------------------------------------------------------
// Ce que l'écran montre (lecture seule).
// ---------------------------------------------------------------------------------------------------------------

/// Un mod de Vortex : son dossier dans le dépôt (`source`, la clé), son nom pour l'écran, son nombre de fichiers.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ModVortex {
    pub source: String,
    pub nom: String,
    pub fichiers: usize,
}

/// Où en est le dernier retrait (des nombres seulement).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RetraitVu {
    pub en_cours: bool,
    pub faits: usize,
    pub total: usize,
    pub laisses: usize,
    pub laisses_rompus: usize,
    /// Les sauvegardes de Vortex à côté d'un fichier rangé.
    pub sauvegardes_vortex: usize,
}

/// L'état des mods de Vortex d'un jeu, pour son bloc (aucun chemin, aucun nom de fichier).
#[derive(Debug, Clone, Default, Serialize)]
pub struct EtatVortex {
    /// Vortex a des mods posés dans ce jeu (son manifeste est là).
    pub present: bool,
    /// Le manifeste est là mais Frogtend refuse d'y toucher (méthode, dépôt…) : le motif.
    pub refus: Option<String>,
    /// Les mods posés (manifeste présent), triés par nom.
    pub mods: Vec<ModVortex>,
    /// Les fichiers PROUVÉS (ceux qu'un retrait rangerait) et leur poids.
    pub fichiers: usize,
    pub octets: u64,
    /// Le dernier retrait, s'il y en a un qui n'est pas tout remis.
    pub retrait: Option<RetraitVu>,
    /// Les mods rangés (remettables), tous retraits confondus.
    pub retires: Vec<ModVortex>,
    /// Les mods REMIS dans le jeu par Frogtend (au moins un fichier « remis » dans un retrait pas tout remis), avec
    /// leur nombre de fichiers remis : la partie « 🧩 Dans le jeu », chacun retirable de nouveau.
    pub remis: Vec<ModVortex>,
    pub diagnostic: Diagnostic,
    /// Les mods (`source`) que le diagnostic désigne.
    pub en_cause: Vec<String>,
    /// Pourquoi rien ne peut se faire maintenant (Galaxy ou Vortex ouvert, partie en cours) : rempli par la commande.
    pub verrou: Option<String>,
}

fn ranger_mods(compte: BTreeMap<String, (String, usize)>) -> Vec<ModVortex> {
    let mut l: Vec<ModVortex> = compte.into_values().map(|(source, fichiers)| ModVortex { nom: nom_du_mod(&source), source, fichiers }).collect();
    l.sort_by(|a, b| a.nom.to_lowercase().cmp(&b.nom.to_lowercase()).then_with(|| a.source.cmp(&b.source)));
    l
}

/// L'état des mods de Vortex du jeu (lecture seule : rien n'est écrit).
pub fn etat(jeu: &Path) -> EtatVortex {
    let mut liste = retraits(jeu);
    // Le recalage d'une coupure, en mémoire seulement (rien n'est écrit) : l'écran voit l'état vrai du disque.
    for (d, r) in liste.iter_mut().filter(|(_, r)| r.etat != EtatRetrait::ToutRemis) {
        recaler(jeu, d, r);
    }
    // Les journaux d'avant le dernier retrait parlent de mods qui ne sont plus là : ignorés.
    let depuis = liste.iter().rev().find(|(_, r)| r.etat != EtatRetrait::ToutRemis).and_then(|(_, r)| r.commence_le.parse::<u64>().ok());
    let mut e = EtatVortex { diagnostic: diagnostic_depuis(jeu, depuis), ..Default::default() };
    // Qui possède quoi (minuscules, `/`) : pour désigner les mods en cause.
    let mut proprietaire: HashMap<String, String> = HashMap::new();
    match lire_deploiement(jeu) {
        Ok(Some(d)) => {
            e.present = true;
            let mut compte: BTreeMap<String, (String, usize)> = BTreeMap::new();
            for f in d.fichiers.iter().filter(|f| f.sur) {
                compte.entry(f.source.to_lowercase()).or_insert_with(|| (f.source.clone(), 0)).1 += 1;
                proprietaire.entry(f.rel.to_lowercase()).or_insert_with(|| f.source.clone());
            }
            // Le nombre annoncé : les fichiers PROUVÉS, comme le retrait les rangera.
            for f in planifier(jeu, &d, &|_, _, _| {}).iter().filter(|f| f.etat == EtatFichier::ADeplacer) {
                e.fichiers += 1;
                e.octets += f.taille;
            }
            e.mods = ranger_mods(compte);
        }
        Ok(None) => {}
        Err(x) => {
            e.present = true;
            e.refus = Some(x.to_string());
        }
    }
    let mut retires: BTreeMap<String, (String, usize)> = BTreeMap::new();
    let mut remis: BTreeMap<String, (String, usize)> = BTreeMap::new();
    for (_, r) in liste.iter().filter(|(_, r)| r.etat != EtatRetrait::ToutRemis) {
        for f in &r.fichiers {
            proprietaire.entry(f.rel.to_lowercase()).or_insert_with(|| f.source.clone());
            if f.etat.a_part() {
                retires.entry(f.source.to_lowercase()).or_insert_with(|| (f.source.clone(), 0)).1 += 1;
            } else if f.etat == EtatFichier::Remis {
                remis.entry(f.source.to_lowercase()).or_insert_with(|| (f.source.clone(), 0)).1 += 1;
            }
        }
    }
    e.retires = ranger_mods(retires);
    e.remis = ranger_mods(remis);
    if let Some((_, r)) = liste.iter().rev().find(|(_, r)| r.etat != EtatRetrait::ToutRemis) {
        let b = bilan_de(r);
        e.retrait = Some(RetraitVu {
            en_cours: r.etat == EtatRetrait::EnCours,
            faits: r.fichiers.iter().filter(|f| f.etat != EtatFichier::ADeplacer).count(),
            total: r.fichiers.len(),
            laisses: b.laisses,
            laisses_rompus: b.laisses_rompus,
            sauvegardes_vortex: b.sauvegardes_vortex,
        });
    }
    let mut en_cause: Vec<String> = Vec::new();
    for rel in &e.diagnostic.fichiers_en_cause {
        if let Some(s) = proprietaire.get(rel) {
            if !en_cause.iter().any(|x| x.eq_ignore_ascii_case(s)) {
                en_cause.push(s.clone());
            }
        }
    }
    e.en_cause = en_cause;
    e
}

// ---------------------------------------------------------------------------------------------------------------
// Le diagnostic (lecture seule) : pourquoi le jeu ne démarre plus.
// ---------------------------------------------------------------------------------------------------------------

/// Un outil de mods qui refuse cette version du jeu.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RefusOutil {
    pub outil: String,
    /// La version du jeu que l'outil attend (« 2.21 »), si elle est dite.
    pub version_attendue: Option<String>,
}

/// Un chargeur de greffons qui s'arrête en chargeant l'un d'eux.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Plante {
    pub chargeur: String,
    pub greffon: String,
}

/// Ce que les journaux du jeu et de ses outils de mods disent (aucun chemin n'est envoyé à l'écran).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Diagnostic {
    pub version: Option<String>,
    pub version_precedente: Option<String>,
    pub refus: Vec<RefusOutil>,
    pub plante: Option<Plante>,
    pub erreurs_scripts: usize,
    pub outil_scripts: Option<String>,
    /// Les fichiers désignés (relatifs au jeu, minuscules, `/`) : pour trouver leurs mods, jamais montrés.
    #[serde(skip)]
    pub fichiers_en_cause: Vec<String>,
}

/// Le diagnostic du jeu : les journaux connus de ses outils de mods, s'il en a (aujourd'hui ceux des jeux REDengine 4).
pub fn diagnostic(jeu: &Path) -> Diagnostic {
    diagnostic_depuis(jeu, None)
}

/// `diagnostic`, sans les journaux écrits avant `depuis` (secondes : le dernier retrait des mods — ils parlent de mods
/// qui ne sont plus là).
pub fn diagnostic_depuis(jeu: &Path, depuis: Option<u64>) -> Diagnostic {
    diagnostic_redengine(jeu, depuis).unwrap_or_default()
}

/// Les journaux d'un outil lus au plus (les plus récents).
const JOURNAUX_LUS: usize = 3;
/// La fin d'un journal lue au plus (jamais 8 Mo à chaque ouverture du panneau).
const FIN_LUE: u64 = 1024 * 1024;

/// Écrit après `depuis` (secondes) ? Sans `depuis` : oui ; une date illisible : non (prudence).
fn ecrit_apres(p: &Path, depuis: Option<u64>) -> bool {
    let Some(d) = depuis else { return true };
    std::fs::metadata(p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .is_some_and(|t| t.as_secs() > d)
}

/// Un journal lu en entier s'il est raisonnable (sinon sa fin seulement), en texte.
fn lire_journal(p: &Path) -> Option<String> {
    use std::io::{Read, Seek, SeekFrom};
    const MAX: u64 = FIN_LUE;
    let m = std::fs::symlink_metadata(p).ok().filter(|m| m.is_file())?;
    let mut f = std::fs::File::open(p).ok()?;
    if m.len() > MAX {
        f.seek(SeekFrom::Start(m.len() - MAX)).ok()?;
    }
    let mut o = Vec::new();
    f.take(MAX).read_to_end(&mut o).ok()?;
    Some(String::from_utf8_lossy(&o).into_owned())
}

/// Ce qui suit `cle` sur la ligne, sans espaces.
fn apres<'a>(ligne: &'a str, cle: &str) -> Option<&'a str> {
    ligne.find(cle).map(|i| ligne[i + cle.len()..].trim()).filter(|v| !v.is_empty())
}

/// Un chemin écrit dans un journal → relatif au jeu (minuscules, `/`), s'il est dans le jeu.
fn rel_du_jeu(jeu: &Path, chemin: &str) -> Option<String> {
    let (j, c) = (crate::garde::cle(jeu)?, crate::garde::cle(Path::new(chemin.trim()))?);
    c.strip_prefix(&format!("{j}\\")).map(|r| r.replace('\\', "/"))
}

/// Une version de programme « 2.5.0.0 » → « 2.5 » (les zéros de fin retirés, au moins deux nombres).
fn version_courte(v: [u16; 4]) -> String {
    let mut l: Vec<u16> = v.to_vec();
    while l.len() > 2 && l.last() == Some(&0) {
        l.pop();
    }
    l.iter().map(u16::to_string).collect::<Vec<_>>().join(".")
}

/// La version d'un programme (`VS_FIXEDFILEINFO`) : (version du fichier « 3.0.88.46235 », version du produit « 2.5 »).
#[cfg(windows)]
pub fn version_programme(p: &Path) -> Option<(String, String)> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW, VS_FIXEDFILEINFO};
    let large: Vec<u16> = p.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    // SAFETY: tampons à nous, de la taille annoncée par Windows ; le pointeur rendu par VerQueryValueW pointe dedans.
    unsafe {
        let n = GetFileVersionInfoSizeW(PCWSTR(large.as_ptr()), None);
        if n == 0 || n > 16 * 1024 * 1024 {
            return None;
        }
        let mut tampon = vec![0u8; n as usize];
        GetFileVersionInfoW(PCWSTR(large.as_ptr()), None, n, tampon.as_mut_ptr() as *mut _).ok()?;
        let mut info: *mut core::ffi::c_void = std::ptr::null_mut();
        let mut taille = 0u32;
        let racine: Vec<u16> = "\\".encode_utf16().chain(std::iter::once(0)).collect();
        if !VerQueryValueW(tampon.as_ptr() as *const _, PCWSTR(racine.as_ptr()), &mut info, &mut taille).as_bool() || info.is_null() || (taille as usize) < std::mem::size_of::<VS_FIXEDFILEINFO>() {
            return None;
        }
        let f = &*(info as *const VS_FIXEDFILEINFO);
        let quatre = |ms: u32, ls: u32| [(ms >> 16) as u16, (ms & 0xFFFF) as u16, (ls >> 16) as u16, (ls & 0xFFFF) as u16];
        let fichier = quatre(f.dwFileVersionMS, f.dwFileVersionLS).iter().map(u16::to_string).collect::<Vec<_>>().join(".");
        Some((fichier, version_courte(quatre(f.dwProductVersionMS, f.dwProductVersionLS))))
    }
}

#[cfg(not(windows))]
pub fn version_programme(_p: &Path) -> Option<(String, String)> {
    None
}

/// Ce qu'un journal de RED4ext dit : sa version du jeu (« Product version »), sa version de fichier, et le greffon en
/// cours de chargement quand il s'est arrêté (un « Loading plugin from '…' » que ne suit ni « has been loaded » ni
/// « N plugin(s) loaded »).
#[derive(Debug, Default, PartialEq)]
struct JournalRed4ext {
    version: Option<String>,
    version_fichier: Option<String>,
    plante: Option<String>,
}

fn lire_red4ext(texte: &str) -> JournalRed4ext {
    let mut j = JournalRed4ext::default();
    let mut en_chargement: Option<String> = None;
    for l in texte.lines() {
        if j.version.is_none() {
            j.version = apres(l, "Product version:").map(String::from);
        }
        if j.version_fichier.is_none() {
            j.version_fichier = apres(l, "File version:").map(String::from);
        }
        if let Some(r) = apres(l, "Loading plugin from '") {
            en_chargement = r.rfind('\'').map(|i| r[..i].to_string());
        } else if l.contains("has been loaded") || l.contains("plugin(s) loaded") {
            en_chargement = None;
        }
    }
    j.plante = en_chargement;
    j
}

/// Ce qu'un journal de Cyber Engine Tweaks dit de sa DERNIÈRE séance : la version du jeu vue, et la version qu'il
/// exige s'il refuse (« Unsupported game version! Only 2.21 is supported. »).
fn lire_cet(texte: &str) -> (Option<String>, Option<Option<String>>) {
    let lignes: Vec<&str> = texte.lines().collect();
    let debut = lignes.iter().rposition(|l| l.contains("CET version")).unwrap_or(0);
    let mut vu = None;
    let mut refus = None;
    for l in &lignes[debut..] {
        if vu.is_none() {
            vu = apres(l, "Game version").map(String::from);
        }
        if let Some(r) = apres(l, "Unsupported game version!") {
            let attendue = apres(r, "Only").and_then(|x| x.split_whitespace().next()).map(String::from);
            refus = Some(attendue);
        }
    }
    (vu, refus)
}

/// Les erreurs d'un journal de redscript (`[ERROR`) et les scripts qu'elles désignent (relatifs au jeu).
fn lire_redscript(texte: &str, jeu: &Path) -> (usize, Vec<String>) {
    let lignes: Vec<&str> = texte.lines().collect();
    // Une même erreur (même message, avec sa ligne « At … » qui suit) ne compte qu'une fois.
    let mut vues: HashSet<String> = HashSet::new();
    let mut fichiers = Vec::new();
    for (i, l) in lignes.iter().enumerate() {
        if !l.trim_start().starts_with("[ERROR") {
            continue;
        }
        let suivante = lignes.get(i + 1).copied().filter(|s| !s.trim_start().starts_with('['));
        let message = l.find(']').map(|k| l[k + 1..].trim()).unwrap_or(l);
        vues.insert(format!("{message}\n{}", suivante.unwrap_or("")));
        for x in [Some(*l), suivante].into_iter().flatten() {
            let bas = x.to_lowercase();
            if let (Some(d), Some(f)) = (bas.find(":\\"), bas.find(".reds")) {
                if d >= 1 && f > d {
                    // Les positions viennent du texte en minuscules : relues prudemment dans l'original.
                    if let Some(rel) = x.get(d - 1..f + 5).and_then(|c| rel_du_jeu(jeu, c)) {
                        if !fichiers.contains(&rel) {
                            fichiers.push(rel);
                        }
                    }
                }
            }
        }
    }
    (vues.len(), fichiers)
}

/// Le diagnostic des jeux REDengine 4 (Cyberpunk 2077) : journaux de RED4ext, de Cyber Engine Tweaks et de redscript,
/// et la version du programme du jeu. `None` : aucun de ces journaux n'existe (un autre jeu). Un journal écrit avant
/// `depuis` (le dernier retrait) est ignoré ; au plus les `JOURNAUX_LUS` derniers de RED4ext, et la fin de chacun.
///
/// La version dite est celle du journal de RED4ext (« 2.50 »). Le repli sur le programme (jeu changé depuis le dernier
/// journal, ou aucun journal) donne la version du produit telle que Windows la lit : « 2.5 » pour la 2.50 (les zéros de
/// fin retirés) — assumé : elle n'est montrée que dans ce cas, et jamais comparée à celle d'un journal.
fn diagnostic_redengine(jeu: &Path, depuis: Option<u64>) -> Option<Diagnostic> {
    let logs = jeu.join("red4ext").join("logs");
    let cet = jeu.join("bin").join("x64").join("plugins").join("cyber_engine_tweaks").join("cyber_engine_tweaks.log");
    let redscript = jeu.join("r6").join("logs").join("redscript_rCURRENT.log");
    let mut journaux: Vec<PathBuf> = std::fs::read_dir(&logs)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()).is_some_and(|n| n.starts_with("red4ext-") && n.ends_with(".log")))
        .collect();
    if journaux.is_empty() && !cet.is_file() && !redscript.is_file() {
        return None;
    }
    // Les noms portent la date (« red4ext-2026-11-25-15-57-22.log ») : l'ordre des noms est celui du temps.
    journaux.sort();
    journaux.retain(|p| ecrit_apres(p, depuis));
    let lus: Vec<JournalRed4ext> = journaux.iter().rev().take(JOURNAUX_LUS).rev().filter_map(|p| lire_journal(p)).map(|t| lire_red4ext(&t)).collect();
    let programme = jeu.join("bin").join("x64").join("Cyberpunk2077.exe");
    let exe = version_programme(&programme);
    let mut d = Diagnostic::default();
    let dernier = lus.last();
    // Le jeu changé depuis le dernier journal (pas encore relancé) : la version de son programme.
    let a_jour = |vf: &Option<String>| match (&exe, vf) {
        (Some((f, _)), Some(v)) => f == v,
        _ => true,
    };
    if let Some(j) = dernier.filter(|j| !a_jour(&j.version_fichier)) {
        d.version = exe.as_ref().map(|(_, p)| p.clone());
        d.version_precedente = j.version.clone();
    } else {
        d.version = dernier.and_then(|j| j.version.clone()).or_else(|| exe.as_ref().map(|(_, p)| p.clone()));
        // La version d'avant : celle du journal qui précède le DERNIER changement (entre deux journaux successifs).
        let versions: Vec<&String> = lus.iter().filter_map(|j| j.version.as_ref()).collect();
        d.version_precedente = versions.windows(2).rev().find(|w| w[0] != w[1]).filter(|w| Some(w[1]) == d.version.as_ref()).map(|w| w[0].clone());
        // RED4ext s'est arrêté en chargeant un greffon (seulement si ce journal parle de la version actuelle).
        if let Some(p) = dernier.and_then(|j| j.plante.as_ref()) {
            let greffon = Path::new(p).file_stem().map(|s| s.to_string_lossy().to_string()).filter(|s| !s.is_empty());
            if let Some(g) = greffon {
                d.plante = Some(Plante { chargeur: "RED4ext".into(), greffon: g });
                if let Some(r) = rel_du_jeu(jeu, p) {
                    d.fichiers_en_cause.push(r);
                }
            }
        }
    }
    // Cyber Engine Tweaks : sa dernière séance, si elle a vu la version actuelle du programme.
    if let Some(t) = Some(&cet).filter(|p| ecrit_apres(p, depuis)).and_then(|p| lire_journal(p)) {
        let (vu, refus) = lire_cet(&t);
        if let Some(attendue) = refus.filter(|_| a_jour(&vu)) {
            d.refus.push(RefusOutil { outil: "Cyber Engine Tweaks".into(), version_attendue: attendue });
            d.fichiers_en_cause.push("bin/x64/plugins/cyber_engine_tweaks.asi".into());
        }
    }
    // redscript : les erreurs de compilation des scripts des mods.
    if let Some(t) = Some(&redscript).filter(|p| ecrit_apres(p, depuis)).and_then(|p| lire_journal(p)) {
        let (n, fichiers) = lire_redscript(&t, jeu);
        if n > 0 {
            d.erreurs_scripts = n;
            d.outil_scripts = Some("redscript".into());
            d.fichiers_en_cause.extend(fichiers);
        }
    }
    Some(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RIEN: &(dyn Fn(&'static str, u64, u64) + Sync) = &|_, _, _| {};

    /// Un faux jeu et un faux dépôt de Vortex, dans le dossier temporaire (même disque : liens durs possibles).
    struct Faux {
        _d: tempfile::TempDir,
        jeu: PathBuf,
        depot: PathBuf,
        fichiers: Vec<(String, String)>,
    }

    impl Faux {
        fn nouveau() -> Faux {
            let d = tempfile::tempdir().unwrap();
            let jeu = d.path().join("Jeux").join("Mon jeu");
            let depot = d.path().join("Vortex Mods").join("Mon jeu");
            std::fs::create_dir_all(&jeu).unwrap();
            std::fs::create_dir_all(&depot).unwrap();
            Faux { _d: d, jeu, depot, fichiers: Vec::new() }
        }

        /// Un fichier d'un mod : dans le dépôt, puis posé dans le jeu par un lien dur (comme Vortex), avec son marqueur
        /// dans chaque dossier créé.
        fn mod_(&mut self, source: &str, rel: &str, contenu: &str) -> &mut Self {
            let du_depot = joindre(&joindre(&self.depot, source), rel);
            std::fs::create_dir_all(du_depot.parent().unwrap()).unwrap();
            std::fs::write(&du_depot, contenu).unwrap();
            let dans_le_jeu = joindre(&self.jeu, rel);
            let mut p = dans_le_jeu.parent().unwrap().to_path_buf();
            let mut a_creer = Vec::new();
            while !p.exists() {
                a_creer.push(p.clone());
                p = p.parent().unwrap().to_path_buf();
            }
            for d in a_creer.iter().rev() {
                std::fs::create_dir(d).unwrap();
                std::fs::write(d.join(MARQUEUR), TEXTE_MARQUEUR).unwrap();
            }
            std::fs::hard_link(&du_depot, &dans_le_jeu).unwrap();
            self.fichiers.push((rel.replace('/', "\\"), source.into()));
            self
        }

        fn manifeste(&self, methode: &str, depot: &Path, cible: &Path, en_plus: &[(&str, &str)]) {
            let mut files: Vec<serde_json::Value> = self.fichiers.iter().map(|(r, s)| serde_json::json!({"relPath": r, "source": s, "target": "", "time": 1699885510000u64})).collect();
            files.extend(en_plus.iter().map(|(r, s)| serde_json::json!({"relPath": r, "source": s, "target": "", "time": 1u64})));
            let v = serde_json::json!({
                "instance": "61357c16-7dfc-408b-85e7-0dd54637a9cc", "version": 1, "deploymentMethod": methode, "gameId": "monjeu",
                "deploymentTime": 1745510047826u64, "stagingPath": depot.to_string_lossy(), "targetPath": cible.to_string_lossy(), "files": files
            });
            std::fs::write(self.jeu.join(MANIFESTE_VORTEX), serde_json::to_vec(&v).unwrap()).unwrap();
        }

        fn manifeste_normal(&self) {
            self.manifeste(METHODE_LIENS, &self.depot, &self.jeu, &[]);
        }

        /// Le dépôt entier : chemin → (contenu, liens).
        fn photo_depot(&self) -> BTreeMap<String, (Vec<u8>, u32)> {
            fn tout(d: &Path, base: &Path, l: &mut BTreeMap<String, (Vec<u8>, u32)>) {
                for e in std::fs::read_dir(d).unwrap().flatten() {
                    let p = e.path();
                    if p.is_dir() {
                        tout(&p, base, l);
                    } else {
                        l.insert(p.strip_prefix(base).unwrap().to_string_lossy().into(), (std::fs::read(&p).unwrap(), liens(&p).unwrap()));
                    }
                }
            }
            let mut l = BTreeMap::new();
            tout(&self.depot, &self.depot, &mut l);
            l
        }

        fn dans_le_jeu(&self, rel: &str) -> PathBuf {
            joindre(&self.jeu, rel)
        }

        fn range(&self, rel: &str) -> PathBuf {
            let (d, _) = retraits(&self.jeu).pop().unwrap();
            joindre(&d.join("fichiers"), rel)
        }
    }

    /// Le cobaye ordinaire : deux mods, trois fichiers, un dossier créé par Vortex (marqueur), un dossier du jeu.
    fn cobaye() -> Faux {
        let mut f = Faux::nouveau();
        std::fs::create_dir_all(f.jeu.join("bin").join("x64")).unwrap();
        std::fs::write(f.jeu.join("bin").join("x64").join("jeu.exe"), b"le jeu").unwrap();
        f.mod_("ArchiveXL-4198-1-21-1-1737797101", "red4ext/plugins/ArchiveXL/ArchiveXL.dll", "archivexl")
            .mod_("ArchiveXL-4198-1-21-1-1737797101", "red4ext/plugins/ArchiveXL/Bundle/a.yaml", "yaml")
            .mod_("CET 1.35.0 - Patch 2.21-107-1-35-0-1737654030", "bin/x64/version.dll", "cet");
        f.manifeste_normal();
        f
    }

    #[test]
    fn l_identite_d_un_lien_dur_est_celle_de_son_original() {
        let f = cobaye();
        let a = identite(&f.dans_le_jeu("bin/x64/version.dll")).unwrap();
        let b = identite(&joindre(&f.depot, "CET 1.35.0 - Patch 2.21-107-1-35-0-1737654030/bin/x64/version.dll")).unwrap();
        assert_eq!((a.volume, a.index, a.liens, a.taille), (b.volume, b.index, 2, 3));
        assert!(!a.dossier && !a.point_de_reanalyse);
        assert_eq!(liens(&f.dans_le_jeu("bin/x64/jeu.exe")), Some(1));
        assert!(identite(&f.jeu).unwrap().dossier);
        assert_eq!(liens(&f.dans_le_jeu("absent")), None);
    }

    #[test]
    fn le_nom_d_un_mod_perd_son_id_sa_version_et_son_horodatage() {
        for (source, nom) in [
            ("ArchiveXL-4198-1-21-1-1737797101", "ArchiveXL"),
            ("CET 1.35.0 - Patch 2.21-107-1-35-0-1737654030", "CET 1.35.0 - Patch 2.21"),
            ("Apartment Cats - Corpo Plaza-6329-2-2-1-1740787060", "Apartment Cats - Corpo Plaza"),
            ("00_CBP_NPCEHB_Compatibility_Patch-10864-3-0--1-0-3-1708309436", "00_CBP_NPCEHB_Compatibility_Patch"),
            ("AdaptiveSliders-5075-2024-06-10-1718017645", "AdaptiveSliders"),
            ("2077 More Gore V3.0.-3040-3-0-1724171105", "2077 More Gore V3.0."),
            ("input_loader.zip-4575-0-2-2-1709344282", "input_loader"),
            ("All Vanilla Clothes Atelier Store-5544-v1-0-1738160755", "All Vanilla Clothes Atelier Store"),
            ("(4k) Immersive Bullet Holes-15309-4k-1718586866", "(4k) Immersive Bullet Holes"),
            ("redscript-1511-0-5-28-1739731226", "redscript"),
            ("Mon mod", "Mon mod"),
            ("-12-1700000000", "-12"),
        ] {
            assert_eq!(nom_du_mod(source), nom, "{source}");
        }
    }

    #[test]
    fn le_manifeste_de_vortex_est_lu_et_verifie() {
        let f = cobaye();
        let d = lire_deploiement(&f.jeu).unwrap().unwrap();
        assert!(crate::garde::meme(&d.depot, &f.depot));
        assert_eq!(d.fichiers.len(), 3);
        assert_eq!(d.fichiers[0], EntreeVortex { rel: "red4ext/plugins/ArchiveXL/ArchiveXL.dll".into(), source: "ArchiveXL-4198-1-21-1-1737797101".into(), sur: true });
        assert!(lire_deploiement(&f.jeu.join("bin")).unwrap().is_none(), "pas de manifeste : rien");
        assert_eq!(rels_listes(&f.jeu).len(), 3);
        assert!(rels_listes(&f.jeu).contains("bin/x64/version.dll"));
    }

    #[test]
    fn une_autre_methode_de_vortex_est_refusee_avec_un_motif_clair() {
        let f = cobaye();
        for (m, mot) in [("symlink_activator", "raccourcis"), ("move_activator", "déplaçant"), ("", "ne connaît pas")] {
            f.manifeste(m, &f.depot, &f.jeu, &[]);
            match retirer(&f.jeu, None, "100", RIEN) {
                Err(Erreur::Refus(x)) => assert!(x.contains(mot) && !x.contains("lien dur") && !x.contains("liens durs"), "{x}"),
                autre => panic!("{autre:?}"),
            }
        }
        assert!(retraits(&f.jeu).is_empty());
        assert!(!dossier_retraits(&f.jeu).exists(), "rien n'est créé avant la vérification");
    }

    #[test]
    fn un_depot_dans_le_jeu_ou_une_autre_cible_sont_refuses() {
        let f = cobaye();
        let avant = f.photo_depot();
        // Le dépôt dans le jeu.
        let dans = f.jeu.join("mods");
        std::fs::create_dir_all(&dans).unwrap();
        f.manifeste(METHODE_LIENS, &dans, &f.jeu, &[]);
        assert!(matches!(retirer(&f.jeu, None, "100", RIEN), Err(Erreur::Refus(_))));
        // Le jeu dans le dépôt.
        f.manifeste(METHODE_LIENS, f.jeu.parent().unwrap(), &f.jeu, &[]);
        assert!(matches!(retirer(&f.jeu, None, "100", RIEN), Err(Erreur::Refus(_))));
        // Un dépôt absent, un dépôt relatif.
        f.manifeste(METHODE_LIENS, &f.depot.join("nulle part"), &f.jeu, &[]);
        assert!(matches!(retirer(&f.jeu, None, "100", RIEN), Err(Erreur::Refus(_))));
        f.manifeste(METHODE_LIENS, Path::new("Vortex Mods"), &f.jeu, &[]);
        assert!(matches!(retirer(&f.jeu, None, "100", RIEN), Err(Erreur::Refus(_))));
        // Une autre cible que ce jeu.
        f.manifeste(METHODE_LIENS, &f.depot, &f.jeu.join("bin"), &[]);
        assert!(matches!(retirer(&f.jeu, None, "100", RIEN), Err(Erreur::Refus(_))));
        assert!(retraits(&f.jeu).is_empty());
        assert_eq!(liens(&f.dans_le_jeu("bin/x64/version.dll")), Some(2), "rien n'a bougé");
        assert_eq!(f.photo_depot(), avant);
    }

    #[test]
    fn un_retrait_range_chaque_fichier_prouve_sans_copie_et_le_depot_reste_intact() {
        let f = cobaye();
        let avant = f.photo_depot();
        let index: Vec<u64> = f.fichiers.iter().map(|(r, _)| identite(&f.dans_le_jeu(&r.replace('\\', "/"))).unwrap().index).collect();
        let vus = std::sync::Mutex::new(Vec::new());
        let b = retirer(&f.jeu, None, "1700000000", &|e, n, t| vus.lock().unwrap().push((e, n, t))).unwrap();
        assert_eq!((b.deplaces, b.laisses, b.mods, b.octets), (3, 0, 2, 9 + 4 + 3));
        assert!(b.deploiement_range);
        assert_eq!(vus.lock().unwrap().last(), Some(&("retrait", 3, 3)));
        for ((r, _), i) in f.fichiers.iter().zip(&index) {
            let r = r.replace('\\', "/");
            assert!(!f.dans_le_jeu(&r).exists(), "{r} a quitté le jeu");
            let a = identite(&f.range(&r)).unwrap();
            assert_eq!((a.index, a.liens), (*i, 2), "{r} : le même fichier, toujours lié au dépôt (renommé, pas copié)");
        }
        assert_eq!(std::fs::read(f.range("bin/x64/version.dll")).unwrap(), b"cet");
        assert_eq!(f.photo_depot(), avant, "le dépôt de Vortex n'est jamais modifié");
        // Le manifeste de Vortex rangé EN DERNIER, dans le dossier du retrait.
        assert!(!f.jeu.join(MANIFESTE_VORTEX).exists());
        let (d, r) = retraits(&f.jeu).pop().unwrap();
        assert_eq!(d.file_name().unwrap(), "1700000000");
        assert!(d.join(MANIFESTE_VORTEX).is_file());
        assert_eq!(r.etat, EtatRetrait::Fini);
        assert!(r.fichiers.iter().all(|x| x.etat == EtatFichier::Deplace && x.index != 0 && x.taille > 0));
        // Le fichier du jeu, jamais listé : en place.
        assert_eq!(std::fs::read(f.dans_le_jeu("bin/x64/jeu.exe")).unwrap(), b"le jeu");
        // Un second retrait : plus de manifeste de Vortex.
        assert!(matches!(retirer(&f.jeu, None, "1700000001", RIEN), Err(Erreur::Refus(_))));
    }

    #[test]
    fn un_lien_rompu_et_une_sauvegarde_de_vortex_sont_laisses_tels_quels() {
        let mut f = cobaye();
        // `options.json` réécrit par la boutique : un autre fichier (1 lien), le dépôt garde l'ancien.
        f.mod_("Options-1-1-0-1700000000", "r6/config/options.json", "vortex");
        let o = f.dans_le_jeu("r6/config/options.json");
        std::fs::remove_file(&o).unwrap();
        std::fs::write(&o, "réécrit par GOG").unwrap();
        std::fs::write(f.dans_le_jeu("r6/config/options.json.vortex_backup"), "2.21").unwrap();
        // Une sauvegarde aussi à côté d'un fichier qui sera RANGÉ : c'est elle seule que l'écran dira.
        std::fs::write(f.dans_le_jeu("bin/x64/version.dll.vortex_backup"), "original").unwrap();
        f.manifeste_normal();
        let b = retirer(&f.jeu, None, "100", RIEN).unwrap();
        assert_eq!((b.deplaces, b.laisses, b.laisses_rompus, b.sauvegardes_vortex), (3, 1, 1, 1));
        assert_eq!(std::fs::read_to_string(&o).unwrap(), "réécrit par GOG");
        assert_eq!(std::fs::read_to_string(f.dans_le_jeu("r6/config/options.json.vortex_backup")).unwrap(), "2.21");
        assert_eq!(std::fs::read_to_string(f.dans_le_jeu("bin/x64/version.dll.vortex_backup")).unwrap(), "original");
        let (_, r) = retraits(&f.jeu).pop().unwrap();
        let x = r.fichiers.iter().find(|x| x.rel.ends_with("options.json")).unwrap();
        assert_eq!((x.etat, x.motif), (EtatFichier::Laisse, Some(Motif::LienRompu)));
        assert_eq!(r.sauvegardes_vortex, ["bin/x64/version.dll.vortex_backup", "r6/config/options.json.vortex_backup"]);
        assert_eq!(etat(&f.jeu).retrait.map(|r| r.sauvegardes_vortex), Some(1));
        // Tout remettre : la sauvegarde de Vortex n'est jamais remise à la place du fichier.
        tout_remettre(&f.jeu, None, RIEN).unwrap();
        assert_eq!(std::fs::read_to_string(&o).unwrap(), "réécrit par GOG");
        assert!(f.dans_le_jeu("r6/config/options.json.vortex_backup").is_file());
    }

    #[test]
    fn un_chemin_dangereux_n_est_jamais_suivi() {
        let f = cobaye();
        // Un fichier HORS du jeu, lié au dépôt sous un chemin qui sortirait du jeu.
        let dehors = f.jeu.parent().unwrap().join("dehors.txt");
        std::fs::create_dir_all(f.depot.join("Malin-1-1-1700000000")).unwrap();
        std::fs::write(f.depot.join("Malin-1-1-1700000000").join("x.txt"), "x").unwrap();
        std::fs::hard_link(f.depot.join("Malin-1-1-1700000000").join("x.txt"), &dehors).unwrap();
        let absolu = dehors.to_string_lossy().to_string();
        f.manifeste(
            METHODE_LIENS,
            &f.depot,
            &f.jeu,
            &[(r"..\dehors.txt", "Malin-1-1-1700000000"), (&absolu, "Malin-1-1-1700000000"), ("bin\\x64\\jeu.exe", r"..\Malin"), ("CON\\x.txt", "Malin-1-1-1700000000"), ("Frogtend\\mods\\a", "Malin-1-1-1700000000")],
        );
        let d = lire_deploiement(&f.jeu).unwrap().unwrap();
        assert_eq!(d.fichiers.iter().filter(|e| !e.sur).count(), 5);
        let b = retirer(&f.jeu, None, "100", RIEN).unwrap();
        assert_eq!((b.deplaces, b.laisses), (3, 5));
        assert_eq!(std::fs::read_to_string(&dehors).unwrap(), "x", "le fichier hors du jeu n'a pas bougé");
        assert_eq!(liens(&dehors), Some(2));
        assert!(f.dans_le_jeu("bin/x64/jeu.exe").is_file());
        let (_, r) = retraits(&f.jeu).pop().unwrap();
        assert!(r.fichiers.iter().filter(|x| x.etat == EtatFichier::Laisse).all(|x| x.motif == Some(Motif::CheminDangereux)));
    }

    #[test]
    fn un_fichier_a_un_seul_lien_ou_sans_depot_est_laisse() {
        let mut f = cobaye();
        // Listé, mais posé sans lien (une copie au même contenu) : jamais pris pour un fichier de Vortex.
        f.mod_("Copie-2-1-1700000000", "archive/pc/mod/copie.archive", "copie");
        let c = f.dans_le_jeu("archive/pc/mod/copie.archive");
        std::fs::remove_file(&c).unwrap();
        std::fs::write(&c, "copie").unwrap();
        // Le dépôt a perdu le fichier d'un autre mod : il reste dans le jeu (1 lien).
        f.mod_("Perdu-3-1-1700000000", "archive/pc/mod/perdu.archive", "perdu");
        std::fs::remove_file(joindre(&f.depot, "Perdu-3-1-1700000000/archive/pc/mod/perdu.archive")).unwrap();
        f.manifeste_normal();
        let b = retirer(&f.jeu, None, "100", RIEN).unwrap();
        assert_eq!((b.deplaces, b.laisses, b.laisses_rompus), (3, 2, 2));
        assert!(c.is_file() && f.dans_le_jeu("archive/pc/mod/perdu.archive").is_file());
    }

    #[test]
    fn un_marqueur_seul_part_avec_son_dossier_jamais_a_cote_de_donnees() {
        let mut f = cobaye();
        // Un dossier où le mod a laissé des réglages de la personne (hors manifeste).
        f.mod_("Reglages-5-1-1700000000", "bin/x64/plugins/cet/mods/m/init.lua", "lua");
        std::fs::write(f.dans_le_jeu("bin/x64/plugins/cet/mods/m/db.sqlite3"), "données").unwrap();
        // Un dossier vide sauf un « marqueur » au texte différent : pas celui de Vortex.
        f.mod_("Autre-6-1-1700000000", "r6/tweaks/t/a.yaml", "a");
        std::fs::write(f.dans_le_jeu("r6/tweaks/t").join(MARQUEUR), b"autre chose").unwrap();
        f.manifeste_normal();
        let b = retirer(&f.jeu, None, "100", RIEN).unwrap();
        assert_eq!(b.deplaces, 5);
        // red4ext/plugins/ArchiveXL/Bundle, ArchiveXL, plugins, red4ext : créés par Vortex, vides → retirés.
        assert!(!f.jeu.join("red4ext").exists(), "les dossiers vidés de Vortex sont retirés");
        assert!(b.dossiers_retires >= 4);
        // Les données de la personne restent, avec le marqueur et leur dossier.
        let m = f.dans_le_jeu("bin/x64/plugins/cet/mods/m");
        assert_eq!(std::fs::read_to_string(m.join("db.sqlite3")).unwrap(), "données");
        assert!(m.join(MARQUEUR).is_file());
        // Le marqueur au texte différent reste, et son dossier aussi.
        assert_eq!(std::fs::read(f.dans_le_jeu("r6/tweaks/t").join(MARQUEUR)).unwrap(), b"autre chose");
        // Le dossier du jeu (bin\x64, sans marqueur) reste.
        assert!(f.jeu.join("bin").join("x64").join("jeu.exe").is_file());
    }

    #[test]
    fn une_coupure_a_mi_chemin_se_reprend() {
        let mut f = cobaye();
        for i in 0..5 {
            f.mod_("Lot-9-1-1700000000", &format!("archive/pc/mod/f{i}.archive"), "x");
        }
        f.manifeste_normal();
        assert!(retirer_avec(&f.jeu, None, "100", RIEN, Some(2)).is_err());
        let (_, r) = en_cours(&f.jeu).expect("un retrait interrompu");
        assert_eq!(r.fichiers.iter().filter(|x| x.etat == EtatFichier::Deplace).count(), 2);
        assert!(f.jeu.join(MANIFESTE_VORTEX).is_file(), "le manifeste de Vortex n'est rangé qu'à la fin");
        let e = etat(&f.jeu);
        assert_eq!(e.retrait.as_ref().map(|r| (r.en_cours, r.faits, r.total)), Some((true, 2, 8)));
        // Un nouveau retrait est refusé, une remise d'un seul mod aussi : reprendre ou tout remettre.
        assert!(matches!(retirer(&f.jeu, None, "101", RIEN), Err(Erreur::Refus(_))));
        assert!(matches!(remettre_mod(&f.jeu, None, "Lot-9-1-1700000000", RIEN), Err(Erreur::Refus(_))));
        // Une coupure juste APRÈS un déplacement, avant l'écriture du manifeste : le fichier est déjà rangé.
        let (d, r) = en_cours(&f.jeu).unwrap();
        let suivant = r.fichiers.iter().find(|x| x.etat == EtatFichier::ADeplacer).unwrap().rel.clone();
        let vers = joindre(&d.join("fichiers"), &suivant);
        std::fs::create_dir_all(vers.parent().unwrap()).unwrap();
        std::fs::rename(f.dans_le_jeu(&suivant), &vers).unwrap();
        let b = reprendre(&f.jeu, None, RIEN).unwrap();
        assert_eq!((b.deplaces, b.laisses), (8, 0));
        assert!(en_cours(&f.jeu).is_none());
        assert!(!f.jeu.join(MANIFESTE_VORTEX).exists());
        assert_eq!(liens(&vers), Some(2));
    }

    #[test]
    fn une_coupure_puis_tout_remettre_rend_le_jeu_comme_avant() {
        let f = cobaye();
        let avant = f.photo_depot();
        assert!(retirer_avec(&f.jeu, None, "100", RIEN, Some(1)).is_err());
        let b = tout_remettre(&f.jeu, None, RIEN).unwrap();
        assert_eq!(b.remis, 1);
        for (r, _) in &f.fichiers {
            assert_eq!(liens(&f.dans_le_jeu(&r.replace('\\', "/"))), Some(2), "{r}");
        }
        assert!(f.jeu.join(MANIFESTE_VORTEX).is_file());
        assert!(en_cours(&f.jeu).is_none());
        assert_eq!(retraits(&f.jeu).pop().unwrap().1.etat, EtatRetrait::ToutRemis);
        assert_eq!(f.photo_depot(), avant);
    }

    #[test]
    fn remettre_un_mod_le_rend_au_jeu_seulement_sur_une_place_libre() {
        let f = cobaye();
        retirer(&f.jeu, None, "100", RIEN).unwrap();
        let axl = "ArchiveXL-4198-1-21-1-1737797101";
        // Une place prise par un autre fichier : jamais écrasé ; l'autre fichier du mod revient.
        std::fs::create_dir_all(f.dans_le_jeu("red4ext/plugins/ArchiveXL")).unwrap();
        std::fs::write(f.dans_le_jeu("red4ext/plugins/ArchiveXL/ArchiveXL.dll"), "nouvelle version").unwrap();
        let b = remettre_mod(&f.jeu, None, axl, RIEN).unwrap();
        assert_eq!((b.remis, b.places_prises, b.changes), (1, 1, 0));
        assert_eq!(std::fs::read_to_string(f.dans_le_jeu("red4ext/plugins/ArchiveXL/ArchiveXL.dll")).unwrap(), "nouvelle version");
        assert_eq!(std::fs::read_to_string(f.dans_le_jeu("red4ext/plugins/ArchiveXL/Bundle/a.yaml")).unwrap(), "yaml");
        assert_eq!(liens(&f.dans_le_jeu("red4ext/plugins/ArchiveXL/Bundle/a.yaml")), Some(2));
        assert!(!f.dans_le_jeu("red4ext/plugins/ArchiveXL/Bundle").join(MARQUEUR).exists(), "les marqueurs ne sont jamais recréés");
        assert!(f.range("red4ext/plugins/ArchiveXL/ArchiveXL.dll").is_file(), "toujours rangé");
        // L'autre mod reste rangé ; l'état le dit.
        let e = etat(&f.jeu);
        assert_eq!(e.retires.iter().map(|m| (m.nom.as_str(), m.fichiers)).collect::<Vec<_>>(), [("ArchiveXL", 1), ("CET 1.35.0 - Patch 2.21", 1)]);
        assert!(!f.dans_le_jeu("bin/x64/version.dll").exists());
        assert!(matches!(remettre_mod(&f.jeu, None, "Inconnu", RIEN), Err(Erreur::Introuvable(_))));
    }

    #[test]
    fn un_fichier_range_qui_a_change_n_est_pas_remis() {
        let f = cobaye();
        retirer(&f.jeu, None, "100", RIEN).unwrap();
        let r = f.range("bin/x64/version.dll");
        std::fs::remove_file(&r).unwrap();
        std::fs::write(&r, "cet").unwrap();
        let b = remettre_mod(&f.jeu, None, "CET 1.35.0 - Patch 2.21-107-1-35-0-1737654030", RIEN).unwrap();
        assert_eq!((b.remis, b.changes), (0, 1));
        assert!(!f.dans_le_jeu("bin/x64/version.dll").exists());
        // Tout remettre : le reste revient, mais pas le manifeste de Vortex (un fichier n'est pas revenu).
        let b = tout_remettre(&f.jeu, None, RIEN).unwrap();
        assert_eq!((b.remis, b.changes, b.deploiement_remis), (2, 1, false));
        assert!(!f.jeu.join(MANIFESTE_VORTEX).exists());
    }

    #[test]
    fn tout_remettre_rend_chaque_fichier_et_le_manifeste_de_vortex() {
        let f = cobaye();
        let avant = f.photo_depot();
        let index: Vec<u64> = f.fichiers.iter().map(|(r, _)| identite(&f.dans_le_jeu(&r.replace('\\', "/"))).unwrap().index).collect();
        retirer(&f.jeu, None, "100", RIEN).unwrap();
        let vus = std::sync::Mutex::new(Vec::new());
        let b = tout_remettre(&f.jeu, None, &|e, n, t| vus.lock().unwrap().push((e, n, t))).unwrap();
        assert_eq!((b.remis, b.places_prises, b.changes, b.deploiement_remis), (3, 0, 0, true));
        assert_eq!(vus.lock().unwrap().last(), Some(&("remise", 3, 3)));
        for ((r, _), i) in f.fichiers.iter().zip(&index) {
            let a = identite(&f.dans_le_jeu(&r.replace('\\', "/"))).unwrap();
            assert_eq!((a.index, a.liens), (*i, 2), "{r}");
        }
        assert!(f.jeu.join(MANIFESTE_VORTEX).is_file());
        assert_eq!(f.photo_depot(), avant);
        let (d, r) = retraits(&f.jeu).pop().unwrap();
        assert_eq!(r.etat, EtatRetrait::ToutRemis);
        assert!(r.fichiers.iter().all(|x| x.etat == EtatFichier::Remis));
        assert!(!d.join("fichiers").join("bin").exists(), "les dossiers vides du rangement sont retirés");
        assert!(etat(&f.jeu).retrait.is_none());
        assert!(matches!(tout_remettre(&f.jeu, None, RIEN), Err(Erreur::Refus(_))));
        // Et un nouveau retrait redevient possible.
        assert_eq!(retirer(&f.jeu, None, "100", RIEN).unwrap().deplaces, 3);
        assert_eq!(retraits(&f.jeu).len(), 2);
    }

    #[test]
    fn l_etat_liste_les_mods_par_nom_avec_leurs_fichiers_et_leur_poids() {
        let f = cobaye();
        let e = etat(&f.jeu);
        assert!(e.present && e.refus.is_none());
        assert_eq!(e.mods.iter().map(|m| (m.nom.as_str(), m.fichiers)).collect::<Vec<_>>(), [("ArchiveXL", 2), ("CET 1.35.0 - Patch 2.21", 1)]);
        assert_eq!((e.fichiers, e.octets), (3, 16));
        assert!(e.retrait.is_none() && e.retires.is_empty());
        // Une méthode refusée : présent, avec le motif.
        f.manifeste("move_activator", &f.depot, &f.jeu, &[]);
        let e = etat(&f.jeu);
        assert!(e.present && e.refus.as_deref().is_some_and(|m| m.contains("déplaçant")));
        // Sans Vortex : rien.
        assert!(!etat(&f.jeu.join("bin")).present);
    }

    // --- Le diagnostic, sur des journaux simulés (lignes réelles relevées le 25/11). ---

    const RED4EXT_221: &str = "[2026-11-25 15:39:08.321] [RED4ext] [info] RED4ext (v1.27.0) is initializing...\r
[2026-11-25 15:39:08.321] [RED4ext] [info] Product version: 2.21\r
[2026-11-25 15:39:08.321] [RED4ext] [info] File version: 3.0.78.57301\r
[2026-11-24 14:38:16.451] [RED4ext] [info] Loading plugins...\r
[2026-11-24 14:38:16.454] [RED4ext] [info] Loading plugin from 'JEU\\red4ext\\plugins\\ArchiveXL\\ArchiveXL.dll'...\r
[2026-11-24 14:38:16.867] [RED4ext] [info] ArchiveXL (version: 1.21.1, author(s): psiberx) has been loaded\r
[2026-11-24 14:38:19.081] [RED4ext] [info] Loading plugin from 'JEU\\bin\\x64\\Cyberpunk2077.exe'...\r
[2026-11-24 14:38:19.081] [RED4ext] [info] 10 plugin(s) loaded\r
[2026-11-24 14:38:19.082] [RED4ext] [info] RED4ext has been started\r
";
    const RED4EXT_250: &str = "[2026-11-25 15:57:22.263] [RED4ext] [info] RED4ext (v1.27.0) is initializing...\r
[2026-11-25 15:57:22.263] [RED4ext] [info] Product version: 2.50\r
[2026-11-25 15:57:22.263] [RED4ext] [info] File version: 3.0.88.46235\r
[2026-11-25 15:57:23.646] [RED4ext] [info] 1032136 game addresses loaded\r
[2026-11-25 15:57:23.747] [RED4ext] [info] Loading plugins...\r
[2026-11-25 15:57:23.748] [RED4ext] [info] Loading plugin from 'JEU\\red4ext\\plugins\\ArchiveXL\\ArchiveXL.dll'...\r
";
    const CET_250: &str = "[2026-11-24 14:38:13 UTC+01:00] [info] [__cdecl Options::Options(struct Paths &)] [13508] CET version v1.35.0 [HEAD]
[2026-11-24 14:38:13 UTC+01:00] [info] [__cdecl Options::Options(struct Paths &)] [13508] Game version 3.0.78.57301
[2026-11-25 15:57:22 UTC+01:00] [info] [__cdecl Options::Options(struct Paths &)] [30636] CET version v1.35.0 [HEAD]
[2026-11-25 15:57:22 UTC+01:00] [info] [__cdecl Options::Options(struct Paths &)] [30636] Game version 3.0.88.46235
[2026-11-25 15:57:22 UTC+01:00] [error] [__cdecl Options::Options(struct Paths &)] [30636] Unsupported game version! Only 2.21 is supported.
";

    fn journaux(jeu: &Path, red4ext: &[(&str, &str)], cet: Option<&str>, redscript: Option<&str>) {
        let l = jeu.join("red4ext").join("logs");
        std::fs::create_dir_all(&l).unwrap();
        let j = jeu.to_string_lossy().to_string();
        for (nom, t) in red4ext {
            std::fs::write(l.join(nom), t.replace("JEU", &j)).unwrap();
        }
        if let Some(t) = cet {
            let d = jeu.join("bin").join("x64").join("plugins").join("cyber_engine_tweaks");
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("cyber_engine_tweaks.log"), t).unwrap();
        }
        if let Some(t) = redscript {
            let d = jeu.join("r6").join("logs");
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("redscript_rCURRENT.log"), t.replace("JEU", &j)).unwrap();
        }
    }

    #[test]
    fn le_diagnostic_dit_la_version_le_refus_de_cet_et_le_greffon_qui_plante() {
        let f = cobaye();
        journaux(
            &f.jeu,
            &[("red4ext-2026-11-24-14-38-14.log", RED4EXT_221), ("red4ext-2026-11-25-15-39-08.log", RED4EXT_221), ("red4ext-2026-11-25-15-57-22.log", RED4EXT_250)],
            Some(CET_250),
            None,
        );
        let d = diagnostic(&f.jeu);
        assert_eq!((d.version.as_deref(), d.version_precedente.as_deref()), (Some("2.50"), Some("2.21")));
        assert_eq!(d.refus, [RefusOutil { outil: "Cyber Engine Tweaks".into(), version_attendue: Some("2.21".into()) }]);
        assert_eq!(d.plante, Some(Plante { chargeur: "RED4ext".into(), greffon: "ArchiveXL".into() }));
        assert_eq!(d.erreurs_scripts, 0);
        // Les mods en cause, par leurs fichiers : ArchiveXL (le .asi de CET n'est pas posé par Vortex dans ce faux jeu).
        let e = etat(&f.jeu);
        assert_eq!(e.en_cause, ["ArchiveXL-4198-1-21-1-1737797101"]);
        // Aucun chemin dans ce que l'écran reçoit.
        let texte = serde_json::to_string(&e).unwrap();
        assert!(!texte.contains("red4ext/") && !texte.contains("Jeux") && !texte.contains("Vortex Mods"), "{texte}");
    }

    #[test]
    fn un_journal_sans_arret_ne_designe_aucun_greffon() {
        let f = cobaye();
        journaux(&f.jeu, &[("red4ext-2026-11-24-14-38-14.log", RED4EXT_221)], None, None);
        let d = diagnostic(&f.jeu);
        assert_eq!((d.version.as_deref(), d.version_precedente.as_deref(), d.plante.as_ref()), (Some("2.21"), None, None));
        assert!(d.refus.is_empty());
        // La dernière séance de CET a marché : l'ancien refus ne compte plus.
        let cet = format!("{CET_250}[2026-11-26 10:00:00 UTC+01:00] [info] [x] [1] CET version v1.36.0 [HEAD]\n[2026-11-26 10:00:00 UTC+01:00] [info] [x] [1] Game version 3.0.88.46235\n");
        journaux(&f.jeu, &[], Some(&cet), None);
        assert!(diagnostic(&f.jeu).refus.is_empty());
    }

    #[test]
    fn le_diagnostic_compte_les_erreurs_de_redscript_et_trouve_leurs_mods() {
        let mut f = cobaye();
        f.mod_("Mon script-77-1-0-1700000000", "r6/scripts/MonScript/a.reds", "func");
        f.manifeste_normal();
        let rs = "[INFO - Mon, 23 Nov 2026 14:52:24 +0100] Compiling files in JEU\\r6\\scripts:\n[ERROR - Wed, 25 Nov 2026 15:57:25 +0100] At JEU\\r6\\scripts\\MonScript\\a.reds:12:5:\nfunc x\n[ERROR - Wed, 25 Nov 2026 15:57:25 +0100] Can't find function\n";
        journaux(&f.jeu, &[], None, Some(rs));
        let d = diagnostic(&f.jeu);
        assert_eq!((d.erreurs_scripts, d.outil_scripts.as_deref()), (2, Some("redscript")));
        assert_eq!(etat(&f.jeu).en_cause, ["Mon script-77-1-0-1700000000"]);
    }

    #[test]
    fn un_jeu_sans_ces_journaux_n_a_pas_de_diagnostic() {
        let f = cobaye();
        assert_eq!(diagnostic(&f.jeu), Diagnostic::default());
        assert!(diagnostic_redengine(&f.jeu, None).is_none());
    }

    #[test]
    fn la_version_d_un_programme_se_lit() {
        // Un programme de Windows (toujours là) : une version de fichier à quatre nombres.
        let windir = std::env::var_os("SystemRoot").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
        let (fichier, produit) = version_programme(&windir.join("System32").join("notepad.exe")).expect("version de notepad");
        assert_eq!(fichier.split('.').count(), 4, "{fichier}");
        assert!(produit.split('.').count() >= 2, "{produit}");
        assert_eq!(version_courte([2, 5, 0, 0]), "2.5");
        assert_eq!(version_courte([2, 0, 0, 0]), "2.0");
        assert_eq!(version_courte([3, 0, 88, 46235]), "3.0.88.46235");
        assert!(version_programme(Path::new("absent.exe")).is_none());
    }

    /// B1 (relecture) : une vraie coupure — des fichiers rangés que le manifeste du retrait note encore « à déplacer »
    /// (il n'est réécrit que par lots) — puis « tout remettre » : chacun revient, rien ne reste à part en silence.
    #[test]
    fn tout_remettre_apres_une_vraie_coupure_rend_aussi_les_fichiers_pas_encore_notes() {
        let mut f = cobaye();
        for i in 0..4 {
            f.mod_("Lot-9-1-1700000000", &format!("archive/pc/mod/f{i}.archive"), "x");
        }
        f.manifeste_normal();
        assert!(retirer_avec(&f.jeu, None, "100", RIEN, Some(1)).is_err());
        // Deux fichiers déplacés à la main, sans réécrire retrait.json (la coupure arrive avant le lot suivant).
        let (d, r) = en_cours(&f.jeu).unwrap();
        let a_deplacer: Vec<String> = r.fichiers.iter().filter(|x| x.etat == EtatFichier::ADeplacer).take(2).map(|x| x.rel.clone()).collect();
        for rel in &a_deplacer {
            let vers = joindre(&d.join("fichiers"), rel);
            std::fs::create_dir_all(vers.parent().unwrap()).unwrap();
            std::fs::rename(f.dans_le_jeu(rel), &vers).unwrap();
        }
        let b = tout_remettre(&f.jeu, None, RIEN).unwrap();
        assert_eq!((b.remis, b.places_prises, b.changes), (3, 0, 0));
        for (rel, _) in &f.fichiers {
            assert_eq!(liens(&f.dans_le_jeu(&rel.replace('\\', "/"))), Some(2), "{rel} est revenu");
        }
        let (_, r) = retraits(&f.jeu).pop().unwrap();
        assert_eq!(r.etat, EtatRetrait::ToutRemis);
        assert!(!r.fichiers.iter().any(|x| x.etat.a_part()));
        assert!(f.jeu.join(MANIFESTE_VORTEX).is_file());
    }

    /// I2 : « tout remettre » parcourt TOUS les retraits pas encore remis, du plus récent au plus ancien ; chaque
    /// manifeste de Vortex ne revient que si sa place est libre.
    #[test]
    fn tout_remettre_parcourt_tous_les_retraits() {
        let mut f = cobaye();
        retirer(&f.jeu, None, "100", RIEN).unwrap();
        // Vortex rouvert a reposé un mod : un second manifeste, un second retrait.
        f.fichiers.clear();
        f.mod_("Nouveau-5-1-1700000000", "archive/pc/mod/nouveau.archive", "n");
        f.manifeste_normal();
        retirer(&f.jeu, None, "200", RIEN).unwrap();
        assert_eq!(etat(&f.jeu).retires.len(), 3);
        let b = tout_remettre(&f.jeu, None, RIEN).unwrap();
        assert_eq!(b.remis, 4);
        assert_eq!(liens(&f.dans_le_jeu("archive/pc/mod/nouveau.archive")), Some(2));
        assert_eq!(liens(&f.dans_le_jeu("bin/x64/version.dll")), Some(2));
        // Le manifeste du plus récent revient ; celui du premier ne peut pas (la place est prise) : il reste rangé.
        let l = retraits(&f.jeu);
        assert_eq!((l[0].1.etat, l[1].1.etat), (EtatRetrait::Fini, EtatRetrait::ToutRemis));
        assert!(l[0].0.join(MANIFESTE_VORTEX).is_file());
        assert!(etat(&f.jeu).retires.is_empty());
    }

    /// I4 : le dernier mod remis un par un rend aussi le manifeste de Vortex.
    #[test]
    fn le_dernier_mod_remis_rend_le_manifeste_de_vortex() {
        let f = cobaye();
        retirer(&f.jeu, None, "100", RIEN).unwrap();
        assert!(!remettre_mod(&f.jeu, None, "ArchiveXL-4198-1-21-1-1737797101", RIEN).unwrap().deploiement_remis);
        assert!(!f.jeu.join(MANIFESTE_VORTEX).exists());
        let b = remettre_mod(&f.jeu, None, "CET 1.35.0 - Patch 2.21-107-1-35-0-1737654030", RIEN).unwrap();
        assert!(b.deploiement_remis);
        assert!(f.jeu.join(MANIFESTE_VORTEX).is_file());
        assert_eq!(retraits(&f.jeu).pop().unwrap().1.etat, EtatRetrait::ToutRemis);
    }

    /// I3 : un fichier « bloqué à part » (son renvoi a échoué) reste proposé et revient à la remise.
    #[test]
    fn un_fichier_bloque_a_part_est_propose_et_remis() {
        let f = cobaye();
        retirer(&f.jeu, None, "100", RIEN).unwrap();
        let (d, mut r) = retraits(&f.jeu).pop().unwrap();
        let k = r.fichiers.iter().position(|x| x.rel == "bin/x64/version.dll").unwrap();
        r.fichiers[k].etat = EtatFichier::BloqueAPart;
        ecrire_retrait(&d, &r, None).unwrap();
        // Il n'est plus reconnu (un autre index), mais il est à part : la remise le rend tel quel.
        let rg = f.range("bin/x64/version.dll");
        std::fs::remove_file(&rg).unwrap();
        std::fs::write(&rg, "cet").unwrap();
        assert!(etat(&f.jeu).retires.iter().any(|m| m.source.starts_with("CET")));
        let b = remettre_mod(&f.jeu, None, "CET 1.35.0 - Patch 2.21-107-1-35-0-1737654030", RIEN).unwrap();
        assert_eq!((b.remis, b.changes), (1, 0));
        assert_eq!(std::fs::read_to_string(f.dans_le_jeu("bin/x64/version.dll")).unwrap(), "cet");
    }

    /// M9 : une copie du manifeste du retrait est tenue dans les données de Frogtend, à jour.
    #[test]
    fn une_copie_du_retrait_est_gardee_dans_les_donnees_de_frogtend() {
        let f = cobaye();
        let copie = f._d.path().join("donnees").join("mods").join("vortex");
        retirer(&f.jeu, Some(&copie), "100", RIEN).unwrap();
        let (d, r) = retraits(&f.jeu).pop().unwrap();
        assert_eq!(lire_retrait(&copie.join("100")), Some(r));
        remettre_mod(&f.jeu, Some(&copie), "ArchiveXL-4198-1-21-1-1737797101", RIEN).unwrap();
        assert_eq!(lire_retrait(&copie.join("100")), lire_retrait(&d), "mise à jour en même temps");
    }

    /// M2 : un nom court de Windows (8.3) est refusé ; un « ~ » en tête de nom (ordre des archives) est accepté.
    #[test]
    fn un_nom_court_de_windows_est_refuse() {
        assert!(rel_sur(r"PROGRA~1\a.dll").is_none());
        assert!(rel_sur(r"r6\MONMOD~2.RED").is_none());
        assert_eq!(rel_sur(r"archive\pc\mod\~Cyberarms_Patch.archive").as_deref(), Some("archive/pc/mod/~Cyberarms_Patch.archive"));
    }

    /// M1 : un dossier n'est jamais créé à travers une jonction.
    #[test]
    fn un_dossier_n_est_jamais_cree_a_travers_une_jonction() {
        let f = cobaye();
        let dehors = f._d.path().join("dehors");
        std::fs::create_dir_all(&dehors).unwrap();
        let j = f.jeu.join("jonction");
        let ok = std::process::Command::new("cmd").args(["/C", "mklink", "/J"]).arg(&j).arg(&dehors).output().is_ok_and(|o| o.status.success());
        if !ok {
            eprintln!("jonction impossible ici : test sauté");
            return;
        }
        assert!(!creer_dossiers(&f.jeu, &j.join("a").join("b")));
        assert!(std::fs::read_dir(&dehors).unwrap().next().is_none(), "rien n'est créé de l'autre côté");
        assert!(creer_dossiers(&f.jeu, &f.jeu.join("x").join("y")));
        let _ = std::fs::remove_dir(&j);
    }

    /// M7 : les journaux écrits avant le dernier retrait ne comptent plus ; seuls les 3 derniers de RED4ext sont lus ;
    /// la version d'avant est celle du dernier changement ; une même erreur de redscript compte une fois.
    #[test]
    fn le_diagnostic_ignore_les_journaux_d_avant_le_retrait() {
        let f = cobaye();
        journaux(&f.jeu, &[("red4ext-2026-11-25-15-39-08.log", RED4EXT_221), ("red4ext-2026-11-25-15-57-22.log", RED4EXT_250)], Some(CET_250), None);
        let futur = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() + 3600;
        let d = diagnostic_depuis(&f.jeu, Some(futur));
        assert!(d.plante.is_none() && d.refus.is_empty() && d.version_precedente.is_none(), "{d:?}");
        assert_eq!(diagnostic_depuis(&f.jeu, Some(0)).plante.map(|p| p.greffon).as_deref(), Some("ArchiveXL"));
        // Après un retrait fini (commencé « dans le futur » pour l'essai), l'état ne dit plus les anciens soucis.
        retirer(&f.jeu, None, &futur.to_string(), RIEN).unwrap();
        let e = etat(&f.jeu);
        assert!(e.diagnostic.plante.is_none() && e.diagnostic.refus.is_empty() && e.en_cause.is_empty());
    }

    #[test]
    fn la_version_d_avant_est_celle_du_dernier_changement_parmi_les_derniers_journaux() {
        let f = cobaye();
        let v = |x: &str| RED4EXT_221.replace("Product version: 2.21", &format!("Product version: {x}"));
        journaux(
            &f.jeu,
            &[("red4ext-1.log", &v("1.00")), ("red4ext-2.log", &v("2.21")), ("red4ext-3.log", &v("2.50")), ("red4ext-4.log", &v("2.50"))],
            None,
            None,
        );
        let d = diagnostic(&f.jeu);
        assert_eq!((d.version.as_deref(), d.version_precedente.as_deref()), (Some("2.50"), Some("2.21")));
        let rs = "[ERROR - a] Can't find function X\nAt JEU\\r6\\scripts\\a.reds:1:1:\n[ERROR - b] Can't find function X\nAt JEU\\r6\\scripts\\a.reds:1:1:\n[ERROR - c] Autre\n";
        journaux(&f.jeu, &[], None, Some(rs));
        assert_eq!(diagnostic(&f.jeu).erreurs_scripts, 2, "la même erreur compte une fois");
    }

    /// RÉEL, LECTURE SEULE (lot M1, 25/11) : le Cyberpunk 2077 de Seb — le manifeste lu, chaque fichier PROUVÉ sans rien
    /// déplacer, l'état et le diagnostic. Rien n'est écrit (aucune fonction qui écrit n'est appelée). À lancer seul :
    /// `cargo test --lib -- --ignored --exact mods_vortex::tests::repetition_reelle_cyberpunk`.
    #[test]
    #[ignore]
    fn repetition_reelle_cyberpunk() {
        let jeu = Path::new(r"D:\LaunchBox\Games\GOG Galaxy\Games\Cyberpunk 2077");
        // Après le vrai retrait de Seb (25/11, 0.227.0) : plus de manifeste dans le jeu, l'état se lit dans retrait.json.
        if lire_deploiement(jeu).unwrap().is_none() {
            let e = etat(jeu);
            eprintln!(
                "après retrait : retrait {:?}, remis {:?}, retirés {}, mods posés {}, diagnostic {:?}",
                e.retrait.as_ref().map(|r| (r.en_cours, r.faits, r.total)),
                e.remis.iter().map(|m| (&m.source, m.fichiers)).collect::<Vec<_>>(),
                e.retires.len(),
                e.mods.len(),
                e.diagnostic
            );
            assert!(e.mods.is_empty());
            assert_eq!(e.remis.len(), 1);
            assert_eq!(e.retires.len(), 283);
            return;
        }
        let d = lire_deploiement(jeu).unwrap().expect("manifeste de Vortex");
        let mut motifs: BTreeMap<String, usize> = BTreeMap::new();
        let mut prouves = 0;
        for e in &d.fichiers {
            match prouver(jeu, &d.depot, e) {
                Ok(_) => prouves += 1,
                Err(m) => {
                    eprintln!("laissé : {} ({m:?})", e.rel);
                    *motifs.entry(format!("{m:?}")).or_default() += 1;
                }
            }
        }
        let e = etat(jeu);
        eprintln!("prouvés {prouves}, laissés {motifs:?}, mods {}, fichiers {}, octets {}", e.mods.len(), e.fichiers, e.octets);
        eprintln!("diagnostic {:?}", e.diagnostic);
        eprintln!("en cause {:?}", e.en_cause);
        assert_eq!((prouves, e.fichiers, e.mods.len()), (2014, 2014, 284));
        assert_eq!(motifs.get("LienRompu"), Some(&1));
        assert!(e.retrait.is_none());
    }

    /// 0.227.1 (défaut 2 vu par Seb le 25/11) : la progression part dès la preuve des fichiers (« vérification », de 0 au
    /// dernier), puis le rangement compte les fichiers PROUVÉS de 0 à N (« 0 fichier sur 2 014 », jamais les laissés).
    #[test]
    fn la_progression_d_un_retrait_part_des_la_verification_et_compte_les_fichiers_prouves() {
        let mut f = cobaye();
        // Un lien rompu (laissé) : il est vérifié, mais pas compté dans le rangement.
        f.mod_("Options-1-1-0-1700000000", "r6/config/options.json", "vortex");
        let o = f.dans_le_jeu("r6/config/options.json");
        std::fs::remove_file(&o).unwrap();
        std::fs::write(&o, "réécrit").unwrap();
        f.manifeste_normal();
        let vus = std::sync::Mutex::new(Vec::new());
        retirer(&f.jeu, None, "100", &|e, n, t| vus.lock().unwrap().push((e, n, t))).unwrap();
        let vus = vus.into_inner().unwrap();
        assert_eq!(vus.first(), Some(&("verification", 0, 4)), "la vérification se montre dès le début");
        assert!(vus.contains(&("verification", 4, 4)));
        let premier_rangement = vus.iter().position(|v| v.0 == "retrait").unwrap();
        assert!(vus[..premier_rangement].iter().all(|v| v.0 == "verification"));
        assert_eq!(vus[premier_rangement], ("retrait", 0, 3), "le rangement part de 0 sur les 3 prouvés");
        assert_eq!(vus.last(), Some(&("retrait", 3, 3)));
    }

    /// 0.227.1 (défaut 1 vu par Seb le 25/11) : un mod REMIS est dans `remis` (partie « 🧩 Dans le jeu »), avec son
    /// nombre de fichiers ; « 🧹 Retirer » le range de nouveau dans le MÊME dossier du retrait, par renommage (même
    /// fichier, liens gardés), et il redevient remettable.
    #[test]
    fn un_mod_remis_est_dans_le_jeu_et_se_retire_de_nouveau() {
        let f = cobaye();
        let avant = f.photo_depot();
        let axl = "ArchiveXL-4198-1-21-1-1737797101";
        let rel = "red4ext/plugins/ArchiveXL/ArchiveXL.dll";
        let index = identite(&f.dans_le_jeu(rel)).unwrap().index;
        retirer(&f.jeu, None, "100", RIEN).unwrap();
        assert!(etat(&f.jeu).remis.is_empty());
        remettre_mod(&f.jeu, None, axl, RIEN).unwrap();
        let e = etat(&f.jeu);
        assert_eq!(e.remis.iter().map(|m| (m.nom.as_str(), m.fichiers)).collect::<Vec<_>>(), [("ArchiveXL", 2)]);
        assert!(!e.retires.iter().any(|m| m.source == axl), "un mod remis n'est plus parmi les retirés");
        let vus = std::sync::Mutex::new(Vec::new());
        let b = retirer_mod(&f.jeu, None, axl, &|e, n, t| vus.lock().unwrap().push((e, n, t))).unwrap();
        assert_eq!(b, BilanRetraitMod { ranges: 2, mods: 1, ..Default::default() });
        assert_eq!(vus.lock().unwrap().last(), Some(&("retrait", 2, 2)));
        assert!(!f.dans_le_jeu(rel).exists());
        let a = identite(&f.range(rel)).unwrap();
        assert_eq!((a.index, a.liens), (index, 2), "le même fichier, renommé, jamais copié");
        let (_, r) = retraits(&f.jeu).pop().unwrap();
        assert_eq!(retraits(&f.jeu).len(), 1, "le même dossier du retrait");
        assert!(r.fichiers.iter().filter(|x| x.source == axl).all(|x| x.etat == EtatFichier::Deplace));
        let e = etat(&f.jeu);
        assert!(e.remis.is_empty());
        assert_eq!(e.retires.iter().find(|m| m.source == axl).map(|m| m.fichiers), Some(2));
        assert_eq!(f.photo_depot(), avant, "le dépôt de Vortex n'est jamais modifié");
        // Plus rien de ce mod à ranger : introuvable ; et il se remet de nouveau.
        assert!(matches!(retirer_mod(&f.jeu, None, axl, RIEN), Err(Erreur::Introuvable(_))));
        assert_eq!(remettre_mod(&f.jeu, None, axl, RIEN).unwrap().remis, 2);
        assert_eq!(liens(&f.dans_le_jeu(rel)), Some(2));
    }

    /// Un fichier remis qui n'est plus celui que Frogtend a remis (réécrit), ou dont la place de rangement est prise,
    /// reste dans le jeu, compté ; les autres fichiers du mod sont rangés.
    #[test]
    fn retirer_un_mod_remis_laisse_un_fichier_change_ou_une_place_prise() {
        let f = cobaye();
        let axl = "ArchiveXL-4198-1-21-1-1737797101";
        retirer(&f.jeu, None, "100", RIEN).unwrap();
        remettre_mod(&f.jeu, None, axl, RIEN).unwrap();
        // Réécrit par la personne (un autre fichier, même nom).
        let dll = f.dans_le_jeu("red4ext/plugins/ArchiveXL/ArchiveXL.dll");
        std::fs::remove_file(&dll).unwrap();
        std::fs::write(&dll, "autre").unwrap();
        // La place de rangement de l'autre fichier, prise.
        let place = f.range("red4ext/plugins/ArchiveXL/Bundle/a.yaml");
        std::fs::create_dir_all(place.parent().unwrap()).unwrap();
        std::fs::write(&place, "déjà là").unwrap();
        let b = retirer_mod(&f.jeu, None, axl, RIEN).unwrap();
        assert_eq!(b, BilanRetraitMod { changes: 1, places_prises: 1, ..Default::default() });
        assert_eq!(std::fs::read_to_string(&dll).unwrap(), "autre");
        assert_eq!(std::fs::read_to_string(&place).unwrap(), "déjà là");
        assert!(f.dans_le_jeu("red4ext/plugins/ArchiveXL/Bundle/a.yaml").is_file());
        assert_eq!(etat(&f.jeu).remis.len(), 1, "toujours dans le jeu");
    }

    /// ✅ Seb 25/11 : Vortex désinstallé et son DÉPÔT EFFACÉ — les fichiers rangés n'ont plus qu'un lien, le dossier du
    /// retrait est leur seule copie. Remettre puis retirer de nouveau marchent sans le dépôt ; le manifeste de Vortex ne
    /// revient pas (il ne mènerait nulle part) ; le dossier du retrait n'est jamais effacé.
    #[test]
    fn sans_le_depot_de_vortex_remettre_puis_retirer_de_nouveau_marchent() {
        let f = cobaye();
        retirer(&f.jeu, None, "100", RIEN).unwrap();
        std::fs::remove_dir_all(&f.depot).unwrap();
        let rel = "bin/x64/version.dll";
        assert_eq!(liens(&f.range(rel)), Some(1));
        let cet = "CET 1.35.0 - Patch 2.21-107-1-35-0-1737654030";
        let axl = "ArchiveXL-4198-1-21-1-1737797101";
        assert_eq!(remettre_mod(&f.jeu, None, axl, RIEN).unwrap().remis, 2);
        let b = remettre_mod(&f.jeu, None, cet, RIEN).unwrap();
        assert_eq!((b.remis, b.deploiement_remis), (1, false), "sans dépôt, le manifeste de Vortex reste rangé");
        assert!(!f.jeu.join(MANIFESTE_VORTEX).exists());
        assert_eq!((std::fs::read_to_string(f.dans_le_jeu(rel)).unwrap().as_str(), liens(&f.dans_le_jeu(rel))), ("cet", Some(1)));
        let e = etat(&f.jeu);
        assert!(!e.present && e.refus.is_none());
        assert_eq!(e.remis.len(), 2, "tous les mods remis restent visibles, dans le jeu");
        assert_eq!(retirer_mod(&f.jeu, None, cet, RIEN).unwrap().ranges, 1);
        assert_eq!(std::fs::read_to_string(f.range(rel)).unwrap(), "cet");
        assert!(!f.dans_le_jeu(rel).exists());
        // Tout remettre, sans dépôt : tout revient, le manifeste et le dossier du retrait restent.
        let b = tout_remettre(&f.jeu, None, RIEN).unwrap();
        assert_eq!((b.remis, b.deploiement_remis), (1, false));
        let (d, r) = retraits(&f.jeu).pop().unwrap();
        assert_eq!(r.etat, EtatRetrait::Fini);
        assert!(d.join(NOM_RETRAIT).is_file() && d.join(MANIFESTE_VORTEX).is_file());
        assert_eq!(etat(&f.jeu).remis.len(), 2);
    }

    /// I4 : une coupure pendant « 🧹 Retirer » un mod remis, juste APRÈS le renommage et AVANT l'écriture du retrait :
    /// l'état le voit déjà rangé (recalage en mémoire), la remise suivante le recale, l'écrit, et le rend.
    #[test]
    fn une_coupure_pendant_le_retrait_d_un_mod_remis_ne_le_laisse_pas_coince() {
        let f = cobaye();
        let cet = "CET 1.35.0 - Patch 2.21-107-1-35-0-1737654030";
        let rel = "bin/x64/version.dll";
        retirer(&f.jeu, None, "100", RIEN).unwrap();
        remettre_mod(&f.jeu, None, cet, RIEN).unwrap();
        // La coupure : le fichier rangé à la main, retrait.json toujours « remis ».
        std::fs::create_dir_all(f.range(rel).parent().unwrap()).unwrap();
        std::fs::rename(f.dans_le_jeu(rel), f.range(rel)).unwrap();
        let (d, r) = retraits(&f.jeu).pop().unwrap();
        assert_eq!(r.fichiers.iter().find(|x| x.rel == rel).unwrap().etat, EtatFichier::Remis);
        let e = etat(&f.jeu);
        assert!(e.remis.is_empty(), "l'écran le voit déjà rangé");
        assert!(e.retires.iter().any(|m| m.source == cet));
        assert!(matches!(retirer_mod(&f.jeu, None, cet, RIEN), Err(Erreur::Introuvable(_))));
        // « 🧹 » a déjà réécrit le retrait recalé : plus jamais coincé.
        assert_eq!(lire_retrait(&d).unwrap().fichiers.iter().find(|x| x.rel == rel).unwrap().etat, EtatFichier::Deplace);
        assert_eq!(remettre_mod(&f.jeu, None, cet, RIEN).unwrap().remis, 1);
        assert_eq!(std::fs::read_to_string(f.dans_le_jeu(rel)).unwrap(), "cet");
    }

    /// m4 : un fichier remis puis effacé par la personne est noté « parti » ; son mod sort de « Dans le jeu » quand plus
    /// aucun de ses fichiers n'y est. L'état le voit dès l'effacement.
    #[test]
    fn un_fichier_remis_puis_efface_est_parti_et_ne_bloque_plus_le_mod() {
        let f = cobaye();
        let axl = "ArchiveXL-4198-1-21-1-1737797101";
        retirer(&f.jeu, None, "100", RIEN).unwrap();
        remettre_mod(&f.jeu, None, axl, RIEN).unwrap();
        std::fs::remove_file(f.dans_le_jeu("red4ext/plugins/ArchiveXL/Bundle/a.yaml")).unwrap();
        assert_eq!(etat(&f.jeu).remis.iter().map(|m| m.fichiers).collect::<Vec<_>>(), [1], "un seul fichier encore dans le jeu");
        let b = retirer_mod(&f.jeu, None, axl, RIEN).unwrap();
        assert_eq!((b.ranges, b.partis), (1, 0), "le parti a été recalé avant : il n'est plus à ranger");
        let (_, r) = retraits(&f.jeu).pop().unwrap();
        assert_eq!(r.fichiers.iter().find(|x| x.rel.ends_with("a.yaml")).unwrap().etat, EtatFichier::Parti);
        // Tous les fichiers effacés : le mod sort de « Dans le jeu ».
        remettre_mod(&f.jeu, None, axl, RIEN).unwrap();
        std::fs::remove_file(f.dans_le_jeu("red4ext/plugins/ArchiveXL/ArchiveXL.dll")).unwrap();
        assert!(etat(&f.jeu).remis.is_empty());
        assert!(matches!(retirer_mod(&f.jeu, None, axl, RIEN), Err(Erreur::Introuvable(_))));
        // reranger_un, s'il croise un fichier parti, le dit « parti ».
        let x = FichierRetire { rel: "absent.dll".into(), source: axl.into(), taille: 1, volume: 1, index: 2, etat: EtatFichier::Remis, motif: None };
        assert_eq!(reranger_un(&f.jeu, &f.jeu.join("Frogtend").join("x"), &x), Err(Echec::Parti));
    }

    /// m7 : une jonction sur le chemin du fichier remis, ou dans le dossier du rangement : rien ne bouge.
    #[test]
    fn retirer_un_mod_remis_refuse_une_jonction_sur_le_chemin_ou_dans_le_rangement() {
        let f = cobaye();
        let axl = "ArchiveXL-4198-1-21-1-1737797101";
        retirer(&f.jeu, None, "100", RIEN).unwrap();
        remettre_mod(&f.jeu, None, axl, RIEN).unwrap();
        let (d, r) = retraits(&f.jeu).pop().unwrap();
        let fichiers = d.join("fichiers");
        let x = r.fichiers.iter().find(|x| x.rel == "red4ext/plugins/ArchiveXL/ArchiveXL.dll").unwrap().clone();
        // Dans le rangement : `fichiers\red4ext` devient une jonction vers ailleurs.
        let dehors = f._d.path().join("dehors");
        std::fs::create_dir_all(&dehors).unwrap();
        let _ = std::fs::remove_dir_all(fichiers.join("red4ext"));
        let j = fichiers.join("red4ext");
        let ok = std::process::Command::new("cmd").args(["/C", "mklink", "/J"]).arg(&j).arg(&dehors).output().is_ok_and(|o| o.status.success());
        if !ok {
            eprintln!("jonction impossible ici : test sauté");
            return;
        }
        assert_eq!(reranger_un(&f.jeu, &fichiers, &x), Err(Echec::Change));
        assert!(f.dans_le_jeu(&x.rel).is_file() && std::fs::read_dir(&dehors).unwrap().next().is_none(), "rien de l'autre côté");
        std::fs::remove_dir(&j).unwrap();
        // Sur le chemin du fichier dans le jeu : `red4ext\plugins` remplacé par une jonction vers le vrai dossier.
        let vrai = f._d.path().join("vrai plugins");
        std::fs::rename(f.jeu.join("red4ext").join("plugins"), &vrai).unwrap();
        let j = f.jeu.join("red4ext").join("plugins");
        assert!(std::process::Command::new("cmd").args(["/C", "mklink", "/J"]).arg(&j).arg(&vrai).output().unwrap().status.success());
        assert_eq!(reranger_un(&f.jeu, &fichiers, &x), Err(Echec::Change));
        let b = retirer_mod(&f.jeu, None, axl, RIEN).unwrap();
        assert_eq!((b.ranges, b.changes), (0, 2));
        assert!(vrai.join("ArchiveXL").join("ArchiveXL.dll").is_file(), "rien n'a bougé");
        std::fs::remove_dir(&j).unwrap();
    }

    /// m7 / m3 : un fichier parti à part mais pas reconnu à l'arrivée, et qui ne peut pas revenir, est « coincé à
    /// part » : compté à part, jamais parmi les rangés, et proposé à la remise.
    #[test]
    fn un_fichier_coince_a_part_est_compte_a_part() {
        let f = cobaye();
        let cet = "CET 1.35.0 - Patch 2.21-107-1-35-0-1737654030";
        retirer(&f.jeu, None, "100", RIEN).unwrap();
        remettre_mod(&f.jeu, None, cet, RIEN).unwrap();
        let (d, mut r) = retraits(&f.jeu).pop().unwrap();
        let fichiers = d.join("fichiers");
        let k = r.fichiers.iter().position(|x| x.rel == "bin/x64/version.dll").unwrap();
        let autre = |_: &Path| -> std::io::Result<Identite> { Ok(Identite { volume: 0, index: 1, liens: 1, point_de_reanalyse: false, dossier: false, taille: 0 }) };
        let refus = |_: &Path, _: &Path| -> std::io::Result<()> { Err(std::io::Error::from_raw_os_error(5)) };
        let issue = reranger_un_avec(&f.jeu, &fichiers, &r.fichiers[k], &autre, &refus);
        assert_eq!(issue, Ok(EtatFichier::BloqueAPart));
        let mut b = BilanRetraitMod::default();
        assert!(!noter_rangement(&mut r.fichiers[k], issue, &mut b));
        assert_eq!((b.ranges, b.coinces), (0, 1));
        assert_eq!(r.fichiers[k].etat, EtatFichier::BloqueAPart);
        ecrire_retrait(&d, &r, None).unwrap();
        assert!(etat(&f.jeu).retires.iter().any(|m| m.source == cet), "proposé à la remise");
        // Un retour possible : « changé », le fichier revient dans le jeu.
        let (_, r2) = retraits(&f.jeu).pop().unwrap();
        assert_eq!(remettre_mod(&f.jeu, None, cet, RIEN).unwrap().remis, 1);
        let _ = r2;
        let (_, r) = retraits(&f.jeu).pop().unwrap();
        let ok = |a: &Path, b: &Path| deplacer(a, b);
        assert_eq!(reranger_un_avec(&f.jeu, &fichiers, &r.fichiers[k], &autre, &ok), Err(Echec::Change));
        assert!(f.dans_le_jeu("bin/x64/version.dll").is_file());
    }

    /// I2 : sans le dépôt de Vortex, après « tout remettre », « 🧹 Tout ranger de nouveau » range chaque mod remis.
    #[test]
    fn tout_ranger_de_nouveau_range_chaque_mod_remis_sans_le_depot() {
        let f = cobaye();
        retirer(&f.jeu, None, "100", RIEN).unwrap();
        std::fs::remove_dir_all(&f.depot).unwrap();
        tout_remettre(&f.jeu, None, RIEN).unwrap();
        assert_eq!(etat(&f.jeu).remis.len(), 2);
        let vus = std::sync::Mutex::new(Vec::new());
        let b = tout_reranger(&f.jeu, None, &|e, n, t| vus.lock().unwrap().push((e, n, t))).unwrap();
        assert_eq!(b, BilanRetraitMod { ranges: 3, mods: 2, ..Default::default() });
        assert_eq!(vus.lock().unwrap().first(), Some(&("retrait", 0, 3)));
        assert_eq!(vus.lock().unwrap().last(), Some(&("retrait", 3, 3)));
        let e = etat(&f.jeu);
        assert!(e.remis.is_empty() && e.retires.len() == 2);
        assert!(matches!(tout_reranger(&f.jeu, None, RIEN), Err(Erreur::Refus(_))));
    }

    /// m1 : le manifeste de Vortex ne revient que si au moins un fichier se prouve encore dans son dépôt.
    #[test]
    fn le_manifeste_ne_revient_que_si_le_depot_tient_encore_le_retrait() {
        let f = cobaye();
        retirer(&f.jeu, None, "100", RIEN).unwrap();
        // Le dossier du dépôt existe toujours, mais vidé de ces mods.
        for e in std::fs::read_dir(&f.depot).unwrap().flatten() {
            std::fs::remove_dir_all(e.path()).unwrap();
        }
        let b = tout_remettre(&f.jeu, None, RIEN).unwrap();
        assert_eq!((b.remis, b.deploiement_remis), (3, false));
        assert!(!f.jeu.join(MANIFESTE_VORTEX).exists());
        assert_eq!(retraits(&f.jeu).pop().unwrap().1.etat, EtatRetrait::Fini);
    }

    /// m6 : la progression d'une remise sur plusieurs retraits compte un total GLOBAL, de 0 à la fin.
    #[test]
    fn la_progression_d_une_remise_sur_plusieurs_retraits_est_globale() {
        let mut f = cobaye();
        retirer(&f.jeu, None, "100", RIEN).unwrap();
        f.fichiers.clear();
        f.mod_("Nouveau-5-1-1700000000", "archive/pc/mod/nouveau.archive", "n");
        f.manifeste_normal();
        retirer(&f.jeu, None, "200", RIEN).unwrap();
        let vus = std::sync::Mutex::new(Vec::new());
        tout_remettre(&f.jeu, None, &|e, n, t| vus.lock().unwrap().push((e, n, t))).unwrap();
        let vus = vus.into_inner().unwrap();
        assert_eq!(vus.first(), Some(&("remise", 0, 4)));
        assert!(vus.iter().all(|v| v.2 == 4), "{vus:?}");
        assert_eq!(vus.iter().filter(|v| v.1 == 4).count() >= 1, true);
        assert_eq!(vus.last(), Some(&("remise", 4, 4)));
    }

    /// m5 : relu, un fichier « remis » à la source douteuse est tenu pour laissé (jamais suivi).
    #[test]
    fn la_relecture_verifie_la_source_d_un_fichier_remis() {
        let f = cobaye();
        retirer(&f.jeu, None, "100", RIEN).unwrap();
        let (d, mut r) = retraits(&f.jeu).pop().unwrap();
        r.fichiers[0].etat = EtatFichier::Remis;
        r.fichiers[0].source = r"..\ailleurs".into();
        std::fs::write(d.join(NOM_RETRAIT), serde_json::to_vec(&r).unwrap()).unwrap();
        let x = &lire_retrait(&d).unwrap().fichiers[0];
        assert_eq!((x.etat, x.motif), (EtatFichier::Laisse, Some(Motif::CheminDangereux)));
    }

    /// m2 : « place prise » seulement pour les refus « existe déjà » de Windows ; tout autre refus est une erreur.
    #[test]
    fn seul_un_refus_existe_deja_est_une_place_prise() {
        for (code, attendu) in [(183, Echec::PlacePrise), (80, Echec::PlacePrise), (5, Echec::Erreur), (32, Echec::Erreur)] {
            assert_eq!(echec_du_renommage(&std::io::Error::from_raw_os_error(code)), attendu, "{code}");
        }
        // Un vrai renommage par-dessus un fichier : refusé « existe déjà ».
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("a"), "a").unwrap();
        std::fs::write(d.path().join("b"), "b").unwrap();
        let e = deplacer(&d.path().join("a"), &d.path().join("b")).unwrap_err();
        assert_eq!(echec_du_renommage(&e), Echec::PlacePrise, "{e:?}");
    }

    #[test]
    fn un_chemin_relatif_du_manifeste_est_verifie() {
        assert_eq!(rel_sur(r"r6\tweaks\a.yaml").as_deref(), Some("r6/tweaks/a.yaml"));
        for mauvais in [r"..\a", r"C:\a", r"\a", "", r"a\..\..\b", r"NUL\a", "a.", r"Frogtend\x", "a:b"] {
            assert!(rel_sur(mauvais).is_none(), "{mauvais}");
        }
        assert!(source_sure("A-1-1").is_some());
        assert!(source_sure(r"A\B").is_none());
        assert_eq!(rel_du_jeu(Path::new(r"D:\J"), r"d:\j\R6\a.reds").as_deref(), Some("r6/a.reds"));
        assert_eq!(rel_du_jeu(Path::new(r"D:\J"), r"D:\K\a.reds"), None);
    }
}
