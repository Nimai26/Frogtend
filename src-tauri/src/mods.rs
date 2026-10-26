//! Les mods servis par Firehouse avec un fichier GitHub (contrat 1.19 ; ✅ Seb 19/10). Frogtend télécharge lui-même
//! (exception au « jamais sur Internet », limitée aux mods) et installe dans le dossier du jeu, après le oui de la
//! personne. Avis des experts (19/10) :
//! - le téléchargement n'utilise JAMAIS le client de Firehouse (son jeton partirait chez GitHub) : HTTPS seulement, au
//!   départ `github.com/<dépôt>/releases/download/…`, redirections seulement vers `*.githubusercontent.com` (relevé
//!   réel du 19/10 : `release-assets.githubusercontent.com`) ; la taille annoncée est un plafond EXACT ; l'empreinte
//!   est vérifiée quand elle est donnée (sinon : avertissement avant le oui, ✅ Seb 19/10) ; reprise d'un fichier coupé ;
//!   jamais une adresse (signée) dans un message ;
//! - l'archive est refusée en entier au premier chemin dangereux (« .. », absolu, « : », nom réservé de Windows, deux
//!   noms qui ne diffèrent que par la casse, lien) ; la décompression est plafonnée ;
//! - tout ce que le mod remplace est d'abord copié dans `<jeu>\Frogtend\mods\<clé>\remplaces\` ; chaque fichier est
//!   posé sous un autre nom puis renommé ; un MANIFESTE par mod (`<jeu>\Frogtend\mods\<clé>.json`) dit ce qui a été
//!   ajouté ou remplacé, pour désinstaller avec preuve : un fichier n'est retiré (ou l'original remis) que s'il n'a pas
//!   changé depuis la pose ; sinon il est gardé, et copié à l'abri.

use crate::erreurs::{Erreur, Resultat};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Décompressé au plus (une archive qui annoncerait plus est refusée sans rien écrire).
pub const DECOMPRESSE_MAX: u64 = 2 * 1024 * 1024 * 1024;

/// Un fichier de mod décrit par Firehouse (contrat 1.19), avec ce qu'il faut pour le télécharger et le poser.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FichierMod {
    pub nom: String,
    pub taille: u64,
    pub sha256: Option<String>,
    pub url: String,
    pub depot: String,
    pub tag: String,
    pub a_confirmer: bool,
    /// Sous-dossier du jeu où poser (`cible.dossier`, relatif et vérifié) ; vide : la racine.
    pub dossier: String,
}

impl FichierMod {
    /// Depuis la réponse de Firehouse : seulement `methode: "extraire"`, `cible.base: "jeu"`, une archive zip ou 7z, une
    /// taille, une adresse ; sinon `None` (Frogtend ne sait pas le poser sans risque : la page reste).
    pub fn depuis(v: &Value) -> Option<Self> {
        let texte = |k: &str| v[k].as_str().map(str::to_string);
        if v["methode"].as_str() != Some("extraire") || v["cible"]["base"].as_str() != Some("jeu") {
            return None;
        }
        let nom = texte("nom")?;
        let bas = nom.to_ascii_lowercase();
        if !bas.ends_with(".zip") && !bas.ends_with(".7z") {
            return None;
        }
        let dossier = match v["cible"]["dossier"].as_str().unwrap_or("") {
            "" => String::new(),
            d => crate::decompression::chemin_sur(d)?,
        };
        let sha256 = texte("sha256").filter(|s| s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())).map(|s| s.to_ascii_lowercase());
        Some(FichierMod {
            nom,
            taille: v["taille"].as_u64().filter(|t| *t > 0)?,
            sha256,
            url: texte("url")?,
            depot: texte("depot")?,
            tag: texte("tag").unwrap_or_default(),
            a_confirmer: v["a_confirmer"].as_bool().unwrap_or(true),
            dossier,
        })
    }

    /// La clé stable de ce fichier de mod (dépôt, version, nom) : son manifeste et ses dossiers dans le jeu.
    pub fn cle(&self) -> String {
        use sha2::Digest;
        let h = sha2::Sha256::digest(format!("{}|{}|{}", self.depot, self.tag, self.nom).as_bytes());
        h.iter().take(8).map(|b| format!("{b:02x}")).collect()
    }

    /// Le nom du fichier téléchargé : sa clé, sa taille et le début de son empreinte (un fichier remplacé par l'auteur
    /// sous la même version n'a jamais le même nom : un téléchargement coupé n'est jamais complété avec un autre).
    pub fn nom_telechargement(&self) -> String {
        let sha: String = self.sha256.as_deref().unwrap_or("sans-empreinte").chars().take(16).collect();
        let ext = if self.nom.to_ascii_lowercase().ends_with(".7z") { "7z" } else { "zip" };
        format!("{}-{}-{sha}.{ext}", self.cle(), self.taille)
    }
}

/// L'adresse de départ admise : HTTPS, `github.com`, une release du dépôt annoncé.
pub fn adresse_admise(f: &FichierMod) -> bool {
    let Ok(u) = reqwest::Url::parse(&f.url) else { return false };
    let depot_sur = !f.depot.is_empty() && f.depot.split('/').count() == 2 && f.depot.chars().all(|c| c.is_ascii_alphanumeric() || "-_./".contains(c));
    // Le chemin EXACT de la release annoncée : dépôt, version (tag) et nom du fichier (`Url::parse` normalise le chemin).
    let attendu = format!("/{}/releases/download/{}/{}", f.depot, f.tag, f.nom);
    let chemin = percent_decode(u.path());
    u.scheme() == "https" && u.host_str() == Some("github.com") && u.port().is_none() && depot_sur && !f.tag.is_empty() && chemin.eq_ignore_ascii_case(&attendu)
}

/// Un chemin d'adresse décodé (`%20` → espace), pour le comparer au nom annoncé.
fn percent_decode(p: &str) -> String {
    let b = p.as_bytes();
    let mut o = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&p[i + 1..i + 3], 16) {
                o.push(v);
                i += 3;
                continue;
            }
        }
        o.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&o).to_string()
}

/// Un hôte où GitHub peut rediriger un fichier de release.
pub fn hote_admis(h: &str) -> bool {
    h == "github.com" || h.ends_with(".githubusercontent.com")
}

/// Suivre une redirection ? Seulement en HTTPS, vers GitHub, et 5 sauts au plus.
pub fn suivre_redirection(deja: usize, url: &reqwest::Url) -> bool {
    deja < 5 && url.scheme() == "https" && url.host_str().is_some_and(hote_admis)
}

fn client() -> Resultat<reqwest::Client> {
    reqwest::Client::builder()
        // Un FICHIER téléchargé : jamais décompressé au passage (taille, reprise et empreinte restent exactes).
        .no_gzip()
        .connect_timeout(std::time::Duration::from_secs(15))
        .read_timeout(std::time::Duration::from_secs(90))
        .user_agent(concat!("Frogtend/", env!("CARGO_PKG_VERSION")))
        .https_only(true)
        .redirect(reqwest::redirect::Policy::custom(|a| if suivre_redirection(a.previous().len(), a.url()) { a.follow() } else { a.stop() }))
        .build()
        .map_err(|_| Erreur::Reseau("Impossible de préparer la connexion.".into()))
}

/// Une erreur réseau dite en français, SANS adresse (signée après la redirection : jamais dans un message ni le
/// journal).
fn reseau(e: reqwest::Error) -> Erreur {
    let quoi = if e.is_timeout() {
        "le site du mod ne répond pas assez vite"
    } else if e.is_connect() {
        "impossible de joindre le site du mod (connexion Internet ?)"
    } else {
        "le téléchargement du mod a été interrompu"
    };
    Erreur::Reseau(format!("{} : réessaie.", quoi[..1].to_uppercase() + &quoi[1..]))
}

/// Télécharge le fichier d'un mod dans `cible` (reprise d'un `.part` déjà là) : jamais plus que la taille annoncée,
/// empreinte vérifiée quand elle est donnée. Rend la taille reçue.
pub async fn telecharger(f: &FichierMod, cible: &Path, progres: &(dyn Fn(u64, u64) + Send + Sync)) -> Resultat<u64> {
    if !adresse_admise(f) {
        return Err(Erreur::Refus("L'adresse de ce mod n'est pas une page de GitHub : Frogtend ne le télécharge pas.".into()));
    }
    telecharger_depuis(&client()?, &f.url, f.taille, true, f.sha256.as_deref(), cible, progres).await
}

/// Télécharge un fichier de Nexus (adresse donnée par son API, client sans clé) : Nexus ne donne que la taille arrondie
/// en Ko, qui sert de PLAFOND ; jamais de reprise (pas d'empreinte). Rend la taille reçue.
pub async fn telecharger_plafonne(c: &reqwest::Client, url: &str, plafond: u64, cible: &Path, progres: &(dyn Fn(u64, u64) + Send + Sync)) -> Resultat<u64> {
    telecharger_depuis(c, url, plafond, false, None, cible, progres).await
}

/// Le cœur du téléchargement (l'adresse déjà admise ; séparé pour les tests). `exacte` : `taille` est la taille exacte
/// du fichier ; sinon c'est un plafond.
async fn telecharger_depuis(c: &reqwest::Client, url: &str, taille: u64, exacte: bool, sha256: Option<&str>, cible: &Path, progres: &(dyn Fn(u64, u64) + Send + Sync)) -> Resultat<u64> {
    if let Some(p) = cible.parent() {
        std::fs::create_dir_all(p)?;
    }
    let part = PathBuf::from(format!("{}.part", cible.to_string_lossy()));
    let mut debut = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
    // Sans empreinte, rien ne prouverait qu'un début gardé vient du même fichier (l'auteur peut le remplacer sous la
    // même version) : jamais de reprise, on repart de zéro.
    if debut >= taille || sha256.is_none() || !exacte {
        let _ = std::fs::remove_file(&part);
        debut = 0;
    }
    let mut req = c.get(url);
    if debut > 0 {
        req = req.header(reqwest::header::RANGE, format!("bytes={debut}-"));
    }
    let mut rep = req.send().await.map_err(reseau)?;
    let statut = rep.status().as_u16();
    let reprise = match statut {
        206 if debut > 0 => true,
        200 => false,
        416 => {
            let _ = std::fs::remove_file(&part);
            return Err(Erreur::Reseau("Le téléchargement du mod doit repartir de zéro : réessaie.".into()));
        }
        300..=399 => return Err(Erreur::Refus("Le site du mod renvoie vers une adresse non admise : Frogtend ne suit pas.".into())),
        _ => return Err(Erreur::Serveur(format!("Le site du mod a répondu {statut}."))),
    };
    // Une reprise doit continuer EXACTEMENT où on en était, sur un fichier de la taille annoncée.
    if reprise {
        let plage = rep.headers().get(reqwest::header::CONTENT_RANGE).and_then(|v| v.to_str().ok()).unwrap_or("");
        if !plage.starts_with(&format!("bytes {debut}-")) || !plage.ends_with(&format!("/{taille}")) {
            let _ = std::fs::remove_file(&part);
            return Err(Erreur::Reseau("Le téléchargement du mod doit repartir de zéro : réessaie.".into()));
        }
    }
    let deja = if reprise { debut } else { 0 };
    if rep.content_length().is_some_and(|n| if exacte { n + deja != taille } else { n > taille }) {
        return Err(Erreur::Serveur("Le fichier du mod n'a pas la taille annoncée : il n'est pas téléchargé.".into()));
    }
    let mut sortie = std::fs::OpenOptions::new().create(true).write(true).append(reprise).truncate(!reprise).open(&part)?;
    let mut recus = deja;
    while let Some(m) = rep.chunk().await.map_err(reseau)? {
        recus += m.len() as u64;
        if recus > taille {
            drop(sortie);
            let _ = std::fs::remove_file(&part);
            return Err(Erreur::Serveur("Le fichier du mod est plus gros qu'annoncé : il n'est pas gardé.".into()));
        }
        sortie.write_all(&m)?;
        progres(recus, taille);
    }
    sortie.flush()?;
    drop(sortie);
    if !exacte && recus == 0 {
        let _ = std::fs::remove_file(&part);
        return Err(Erreur::Serveur("Le fichier du mod est vide : il n'est pas gardé.".into()));
    }
    if exacte && recus != taille {
        return Err(Erreur::Reseau("Le téléchargement du mod s'est arrêté avant la fin : il reprendra où il en était.".into()));
    }
    if let Some(attendue) = sha256 {
        let calculee = crate::sauvegarde::empreinte(&part)?;
        if !calculee.eq_ignore_ascii_case(attendue) {
            let _ = std::fs::remove_file(&part);
            return Err(Erreur::Conflit("Le fichier du mod ne correspond pas à son empreinte : rien n'est installé. Firehouse relit la liste chaque jour : réessaie demain.".into()));
        }
    }
    std::fs::rename(&part, cible)?;
    Ok(recus)
}

/// Un fichier de l'archive : son chemin relatif sûr (séparateur `/`) et sa taille annoncée.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Entree {
    pub chemin: String,
    pub taille: u64,
}

/// Un nom réservé de Windows (« CON », « nul.txt », « COM1 »…) ou terminé par un point ou une espace.
fn nom_reserve(partie: &str) -> bool {
    let base = partie.split('.').next().unwrap_or("").trim().to_ascii_uppercase();
    let reserves = ["CON", "PRN", "AUX", "NUL"];
    reserves.contains(&base.as_str())
        || (base.len() == 4 && (base.starts_with("COM") || base.starts_with("LPT")) && base.as_bytes()[3].is_ascii_digit())
        || partie.ends_with('.')
        || partie.ends_with(' ')
}

/// Entrées d'une archive au plus.
const ENTREES_MAX: usize = 100_000;

/// Un chemin (relatif au jeu) qui tombe dans le dossier de Frogtend du jeu.
fn dans_frogtend(chemin: &str) -> bool {
    chemin.split('/').next().is_some_and(|p| p.eq_ignore_ascii_case("frogtend"))
}

/// Le format d'une archive de mod, d'après ses premiers octets (jamais d'après son nom).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FormatMod {
    Zip,
    SeptZ,
}

pub fn format_mod(archive: &Path) -> Option<FormatMod> {
    let mut t = [0u8; 6];
    std::fs::File::open(archive).and_then(|mut f| f.read_exact(&mut t)).ok()?;
    if t[..4] == *b"PK\x03\x04" {
        Some(FormatMod::Zip)
    } else if t == [0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C] {
        Some(FormatMod::SeptZ)
    } else {
        None
    }
}

/// Une entrée brute d'une archive (zip ou 7z), avant les règles de Frogtend.
struct Brute {
    nom: String,
    taille: u64,
    dossier: bool,
    lien: bool,
}

fn brutes(archive: &Path) -> Resultat<Vec<Brute>> {
    let trop = || Erreur::Refus("L'archive du mod contient bien trop de fichiers : rien n'est installé.".into());
    match format_mod(archive) {
        Some(FormatMod::Zip) => {
            let mut z = zip::ZipArchive::new(std::fs::File::open(archive)?).map_err(|e| Erreur::Disque(format!("Archive du mod illisible ({e}).")))?;
            if z.len() > ENTREES_MAX {
                return Err(trop());
            }
            (0..z.len())
                .map(|i| {
                    let e = z.by_index_raw(i).map_err(|e| Erreur::Disque(format!("Archive du mod abîmée ({e}).")))?;
                    Ok(Brute { nom: e.name().to_string(), taille: e.size(), dossier: e.is_dir(), lien: e.is_symlink() })
                })
                .collect()
        }
        Some(FormatMod::SeptZ) => {
            // Lu par un Frogtend enfant (`sept_z` : un 7z piégé ne peut pas fermer Frogtend).
            let l = crate::sept_z::lister(archive)?;
            if l.len() > ENTREES_MAX {
                return Err(trop());
            }
            let mut r = Vec::new();
            for e in l {
                // Une entrée « à supprimer » (7z de mise à jour) ne porte aucune donnée : ignorée, Frogtend ne supprime
                // jamais rien sur l'ordre d'une archive. (sevenz-rust 0.6.1 marque même ainsi, à tort, les dossiers
                // ET les fichiers vides qu'il écrit : `write_file_anti_items` inverse le drapeau. Un fichier vide
                // d'une telle archive est donc omis ; les archives de 7-Zip ne sont pas touchées.)
                if e.anti {
                    continue;
                }
                // Une entrée sans nom (le dossier racine de certains outils) est ignorée si elle ne porte rien.
                if e.nom.is_empty() && (e.dossier || !e.flux || e.taille == 0) {
                    continue;
                }
                r.push(Brute { lien: e.est_un_lien(), nom: e.nom, taille: e.taille, dossier: e.dossier });
            }
            Ok(r)
        }
        None => Err(Erreur::Refus("Ce fichier n'est ni un .zip ni un .7z (un .rar, peut-être) : Frogtend ne sait pas encore le poser. Choisis un autre fichier sur la page du mod s'il y en a un, ou installe-le à la main.".into())),
    }
}

/// Les fichiers d'une archive de mod (zip ou 7z), refusée EN ENTIER au premier chemin dangereux ; plafond de taille.
pub fn lister(archive: &Path) -> Resultat<Vec<Entree>> {
    let mut l: Vec<Entree> = Vec::new();
    let mut vus = std::collections::HashSet::new();
    let mut total = 0u64;
    for e in brutes(archive)? {
        let dangereux = || Erreur::Refus(format!("L'archive du mod contient un chemin dangereux ({}) : rien n'est installé.", e.nom));
        if e.lien {
            return Err(dangereux());
        }
        let chemin = crate::decompression::chemin_sur(&e.nom).ok_or_else(dangereux)?;
        // Jamais dans le dossier de Frogtend du jeu (ses manifestes et ses copies de sauvegarde).
        if chemin.split('/').any(nom_reserve) || dans_frogtend(&chemin) {
            return Err(dangereux());
        }
        if e.dossier {
            continue;
        }
        // Deux noms qui ne diffèrent que par la casse : un seul fichier sous Windows.
        if !vus.insert(chemin.to_lowercase()) {
            return Err(dangereux());
        }
        total = total.saturating_add(e.taille);
        if total > DECOMPRESSE_MAX {
            return Err(Erreur::Refus("L'archive du mod est démesurée une fois décompressée : rien n'est installé.".into()));
        }
        l.push(Entree { chemin, taille: e.taille });
    }
    if l.is_empty() {
        return Err(Erreur::Refus("L'archive du mod est vide.".into()));
    }
    Ok(l)
}

/// Le seul dossier à la racine de l'archive, s'il contient tout (« MonMod/… »).
pub fn racine_unique(entrees: &[Entree]) -> Option<String> {
    let premiere = entrees.first()?.chemin.split_once('/')?.0.to_string();
    entrees.iter().all(|e| e.chemin.starts_with(&format!("{premiere}/"))).then_some(premiere)
}

/// Une ligne du manifeste : un fichier posé par le mod.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pose {
    /// Relatif au dossier du jeu, séparateur `/`.
    pub chemin: String,
    /// `ajoute` (n'existait pas) ou `remplace` (l'original est dans `remplaces\`).
    pub sorte: String,
    pub sha256_pose: String,
    /// L'empreinte de l'original remplacé (une désinstallation reprise après une coupure reconnaît un original déjà
    /// remis, au lieu de le croire modifié par la personne).
    #[serde(default)]
    pub sha256_origine: Option<String>,
}

/// Le manifeste d'un mod installé (`<jeu>\Frogtend\mods\<clé>.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifeste {
    pub cle: String,
    pub mod_nom: String,
    pub fichier: String,
    pub depot: String,
    pub tag: String,
    pub sha256_archive: String,
    pub pose_le: String,
    /// `en_cours` pendant la pose (une pose interrompue se défait), puis `installe`.
    pub etat: String,
    pub fichiers: Vec<Pose>,
    /// Les dossiers créés par le mod (retirés à la désinstallation s'ils sont vides).
    pub dossiers: Vec<String>,
    /// Un jeu Steam (lot B3) : la version du jeu (`buildid` de son appmanifest) quand le mod a été posé. Une autre
    /// version ensuite = Steam a mis le jeu à jour, et le mod a pu être défait.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steam_buildid: Option<String>,
}

/// Le dossier de Frogtend pour les mods d'un jeu (dans le jeu : « pas de pieuvre »). Un jeu de BOUTIQUE garde le sien
/// dans les données de Frogtend (`installes_dans`, `installer_dans`, `desinstaller_dans` : le paramètre `garde`).
pub fn dossier_mods(jeu: &Path) -> PathBuf {
    jeu.join("Frogtend").join("mods")
}

/// Un manifeste qu'on peut croire : sa clé (16 hexadécimaux) est celle de son nom de fichier, chaque chemin et chaque
/// dossier est un chemin relatif sûr (hors du dossier de Frogtend), chaque sorte est connue. Sinon il n'existe pas pour
/// Frogtend (un faux manifeste ne fera jamais effacer quoi que ce soit).
fn manifeste_sain(m: &Manifeste, nom_fichier: &str) -> bool {
    let sur = |c: &str| crate::decompression::chemin_sur(c).as_deref() == Some(c) && !dans_frogtend(c) && !c.split('/').any(nom_reserve);
    m.cle.len() == 16
        && m.cle.chars().all(|c| c.is_ascii_hexdigit())
        && nom_fichier == format!("{}.json", m.cle)
        && m.fichiers.iter().all(|p| sur(&p.chemin) && (p.sorte == "ajoute" || p.sorte == "remplace"))
        && m.dossiers.iter().all(|d| sur(d))
}

/// Les mods installés dans ce jeu (manifestes lus, plafonnés, vérifiés ; un manifeste illisible ou douteux est ignoré).
pub fn installes(jeu: &Path) -> Vec<Manifeste> {
    installes_dans(&dossier_mods(jeu))
}

/// `installes`, d'après le dossier de garde des mods (`garde`).
pub fn installes_dans(garde: &Path) -> Vec<Manifeste> {
    let Ok(l) = std::fs::read_dir(garde) else { return Vec::new() };
    let mut r: Vec<Manifeste> = l
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter(|e| e.metadata().is_ok_and(|m| m.is_file() && m.len() <= 4 * 1024 * 1024))
        .filter_map(|e| {
            let m = serde_json::from_slice::<Manifeste>(&std::fs::read(e.path()).ok()?).ok()?;
            manifeste_sain(&m, &e.file_name().to_string_lossy()).then_some(m)
        })
        .collect();
    r.sort_by(|a, b| a.mod_nom.cmp(&b.mod_nom));
    r
}

fn ecrire_manifeste(garde: &Path, m: &Manifeste) -> Resultat<()> {
    let f = garde.join(format!("{}.json", m.cle));
    let tmp = garde.join(format!("{}.json.tmp", m.cle));
    std::fs::create_dir_all(garde)?;
    std::fs::write(&tmp, serde_json::to_vec_pretty(m).map_err(|e| Erreur::Disque(e.to_string()))?)?;
    std::fs::rename(&tmp, &f)?;
    Ok(())
}

/// Copie un fichier sous un nom provisoire puis le renomme (jamais un fichier à moitié écrit à sa place).
fn poser_fichier(source: &Path, cible: &Path) -> Resultat<()> {
    let tmp = PathBuf::from(format!("{}.frogtend-tmp", cible.to_string_lossy()));
    std::fs::copy(source, &tmp)?;
    std::fs::rename(&tmp, cible).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })?;
    Ok(())
}

/// Où le mod va : le dossier du jeu (ou un sous-dossier vérifié), sans lien ni jonction sur le chemin.
fn dossier_cible(jeu: &Path, sous: &str) -> Resultat<(PathBuf, String)> {
    let sous = if sous.is_empty() { String::new() } else { crate::decompression::chemin_sur(sous).ok_or_else(|| Erreur::Refus("Dossier du mod inattendu.".into()))? };
    if dans_frogtend(&sous) {
        return Err(Erreur::Refus("Dossier du mod inattendu.".into()));
    }
    let cible = if sous.is_empty() { jeu.to_path_buf() } else { jeu.join(&sous) };
    if crate::garde::lien_entre(jeu, &cible) {
        return Err(Erreur::Refus("Le dossier du jeu passe par un lien : Frogtend n'y pose pas de mod.".into()));
    }
    Ok((cible, sous))
}

/// Retire le dossier de travail d'une installation tant qu'elle n'est pas engagée (aucun fichier du jeu touché).
struct Nettoyage {
    dossier: PathBuf,
    actif: bool,
}

impl Drop for Nettoyage {
    fn drop(&mut self) {
        if self.actif {
            let _ = std::fs::remove_dir_all(&self.dossier);
        }
    }
}

/// Le résumé d'une installation.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Bilan {
    pub ajoutes: usize,
    pub remplaces: usize,
    pub identiques: usize,
}

/// Les restes d'une pose arrêtée net (Frogtend fermé pendant l'extraction : le nettoyage n'a pas pu passer) : dans
/// chaque dossier de mod SANS manifeste, seulement `brut` et `contenu` (des copies de l'archive, refaites à la pose).
/// Jamais `remplaces` ni `gardes`.
fn ranger_les_restes(garde: &Path) {
    let Ok(l) = std::fs::read_dir(garde) else { return };
    for e in l.flatten() {
        let nom = e.file_name().to_string_lossy().to_string();
        let cle_seule = nom.len() == 16 && nom.chars().all(|c| c.is_ascii_hexdigit());
        if !cle_seule || !e.file_type().is_ok_and(|t| t.is_dir()) || garde.join(format!("{nom}.json")).exists() {
            continue;
        }
        for reste in ["brut", "contenu"] {
            let _ = std::fs::remove_dir_all(e.path().join(reste));
        }
    }
}

/// D'où viennent les fichiers d'un mod pendant la pose.
enum SourceMod {
    Zip(Box<zip::ZipArchive<std::fs::File>>, std::collections::HashMap<String, usize>),
    /// Le dossier où le 7z a été décompressé par l'enfant.
    SeptZ(PathBuf),
}

impl SourceMod {
    /// Range le fichier `e` en `vers` ; rend sa taille (au plus `e.taille + 1` : au-delà, c'est déjà faux).
    fn poser(&mut self, e: &Entree, vers: &Path) -> Resultat<u64> {
        match self {
            SourceMod::Zip(z, index) => {
                let i = *index.get(&e.chemin).ok_or_else(|| Erreur::Disque(format!("Fichier introuvable dans l'archive : {}.", e.chemin)))?;
                let mut entree = z.by_index(i).map_err(|_| Erreur::Disque(format!("Fichier illisible dans l'archive : {}.", e.chemin)))?;
                let mut sortie = std::fs::File::create(vers)?;
                Ok(std::io::copy(&mut (&mut entree).take(e.taille + 1), &mut sortie)?)
            }
            SourceMod::SeptZ(brut) => {
                // Vérifier le résultat, pas le retour : un vrai fichier (jamais un lien), de la taille annoncée.
                let de = brut.join(&e.chemin);
                let m = std::fs::symlink_metadata(&de).map_err(|_| Erreur::Disque(format!("Fichier absent après la décompression : {}.", e.chemin)))?;
                if !m.is_file() || crate::garde::lien_entre(brut, &de) {
                    return Err(Erreur::Refus(format!("L'archive du mod contient un chemin dangereux ({}) : rien n'est installé.", e.chemin)));
                }
                std::fs::rename(&de, vers)?;
                Ok(m.len())
            }
        }
    }
}

/// Installe l'archive (déjà téléchargée et vérifiée) dans le jeu, dans `sous` (relatif au jeu), sans son dossier
/// racine si `sans_racine`. Tout ce qui est remplacé est d'abord copié à l'abri ; une pose interrompue se défait.
#[allow(clippy::too_many_arguments)]
pub fn installer(jeu: &Path, archive: &Path, f: &FichierMod, mod_nom: &str, sha256_archive: &str, sous: &str, sans_racine: bool, quand: &str) -> Resultat<Bilan> {
    installer_avec(jeu, archive, f, mod_nom, sha256_archive, sous, sans_racine, quand, &|_, _, _| {})
}

/// `installer`, en disant où il en est : `progres(étape, faits, total)` — « decompression » (octets) puis « pose »
/// (fichiers).
#[allow(clippy::too_many_arguments)]
pub fn installer_avec(jeu: &Path, archive: &Path, f: &FichierMod, mod_nom: &str, sha256_archive: &str, sous: &str, sans_racine: bool, quand: &str, progres: &(dyn Fn(&'static str, u64, u64) + Sync)) -> Resultat<Bilan> {
    installer_dans(jeu, &dossier_mods(jeu), None, archive, f, mod_nom, sha256_archive, sous, sans_racine, quand, progres)
}

/// `installer_avec`, en gardant manifeste, travail et originaux dans `garde` (un jeu de boutique : hors de son dossier
/// chez la boutique) ; `steam_buildid` : la version du jeu Steam à cet instant.
#[allow(clippy::too_many_arguments)]
pub fn installer_dans(
    jeu: &Path,
    garde: &Path,
    steam_buildid: Option<String>,
    archive: &Path,
    f: &FichierMod,
    mod_nom: &str,
    sha256_archive: &str,
    sous: &str,
    sans_racine: bool,
    quand: &str,
    progres: &(dyn Fn(&'static str, u64, u64) + Sync),
) -> Resultat<Bilan> {
    let cle = f.cle();
    if installes_dans(garde).iter().any(|m| m.cle == cle) {
        return Err(Erreur::Refus("Ce mod est déjà installé.".into()));
    }
    let entrees = lister(archive)?;
    let racine = if sans_racine { racine_unique(&entrees) } else { None };
    // Le dossier visé est vérifié (pas de lien) ; il est créé avec les autres, et noté s'il est nouveau.
    let (_cible, sous) = dossier_cible(jeu, sous)?;
    let sous = sous.as_str();
    // Où va chaque fichier dans le jeu (avant toute décompression : un refus ne coûte rien).
    let rel_de = |e: &Entree| match &racine {
        Some(r) => e.chemin.strip_prefix(&format!("{r}/")).unwrap_or(&e.chemin).to_string(),
        None => e.chemin.clone(),
    };
    let dans_le_jeu = |rel: &str| if sous.is_empty() { rel.to_string() } else { format!("{sous}/{rel}") };
    // Un fichier qui appartient déjà à un autre mod : refusé (désinstaller l'autre d'abord).
    let autres: Vec<(String, String)> = installes_dans(garde).into_iter().flat_map(|m| m.fichiers.into_iter().map(move |p| (p.chemin.to_lowercase(), m.mod_nom.clone()))).collect();
    for e in &entrees {
        let c = dans_le_jeu(&rel_de(e)).to_lowercase();
        if let Some((_, autre)) = autres.iter().find(|(x, _)| *x == c) {
            return Err(Erreur::Refus(format!("Ce mod remplacerait un fichier du mod « {autre} » : désinstalle-le d'abord.")));
        }
    }
    // La place : le mod décompressé dans `garde`, puis posé dans le jeu, plus les originaux copiés dans `garde` (deux
    // disques quand le jeu est chez sa boutique : chacun vérifié).
    let mod_ = entrees.iter().map(|e| e.taille).sum::<u64>();
    let originaux: u64 = entrees.iter().filter_map(|e| std::fs::metadata(jeu.join(dans_le_jeu(&rel_de(e)))).ok()).filter(|m| m.is_file()).map(|m| m.len()).sum();
    std::fs::create_dir_all(garde)?;
    let (libre_jeu, libre_garde) = (crate::jeux_pc::place_libre(jeu), crate::jeux_pc::place_libre(garde));
    let meme_disque = jeu.components().next() == garde.components().next();
    let manque = if meme_disque {
        libre_jeu.is_some_and(|l| mod_.saturating_mul(2).saturating_add(originaux) > l)
    } else {
        libre_jeu.is_some_and(|l| mod_ > l) || libre_garde.is_some_and(|l| mod_.saturating_add(originaux) > l)
    };
    if manque {
        return Err(Erreur::Disque("Pas assez de place sur le disque pour ce mod.".into()));
    }
    ranger_les_restes(garde);
    let travail = garde.join(&cle);
    let contenu = travail.join("contenu");
    if travail.exists() {
        // Un dossier de ce mod sans manifeste : une pose interrompue, ou un mod RETIRÉ dont la désinstallation a gardé
        // des originaux (`remplaces`) ou des fichiers modifiés par la personne (`gardes`). Ceux-là ne s'effacent
        // jamais : le dossier est mis de côté sous un autre nom ; sinon, ce ne sont que des restes.
        if travail.join("remplaces").exists() || travail.join("gardes").exists() {
            let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            std::fs::rename(&travail, garde.join(format!("{cle}-ancien-{n}")))?;
        } else {
            std::fs::remove_dir_all(&travail)?;
        }
    }
    // Tant que le manifeste n'est pas écrit, rien du jeu n'est touché : une erreur efface tout le travail.
    let mut nettoyage = Nettoyage { dossier: travail.clone(), actif: true };
    // 1. Décompresser à côté, plafonné à la taille annoncée de chaque fichier.
    let mut source = match format_mod(archive) {
        Some(FormatMod::SeptZ) => {
            // Un 7z : décompressé en UNE passe (un bloc compact se relirait depuis le début pour chaque fichier) par un
            // Frogtend enfant, dans `brut` ; chaque fichier y est ensuite vérifié avant d'être rangé.
            let brut = travail.join("brut");
            let total = entrees.iter().map(|e| e.taille).sum::<u64>();
            let mut faits = 0u64;
            crate::sept_z::extraire(archive, &brut, &mut |n| {
                faits += n;
                progres("decompression", faits.min(total), total);
            })?;
            SourceMod::SeptZ(brut)
        }
        _ => {
            let mut z = zip::ZipArchive::new(std::fs::File::open(archive)?).map_err(|e| Erreur::Disque(format!("Archive du mod illisible ({e}).")))?;
            // Chaque chemin sûr → son numéro dans l'archive (les noms bruts peuvent avoir « \ » ou « ./ »).
            let mut index = std::collections::HashMap::new();
            for i in 0..z.len() {
                if let Ok(e) = z.by_index_raw(i) {
                    if let Some(c) = crate::decompression::chemin_sur(e.name()) {
                        index.insert(c, i);
                    }
                }
            }
            SourceMod::Zip(Box::new(z), index)
        }
    };
    let mut plan: Vec<(String, PathBuf)> = Vec::new();
    let (mut faits_zip, total_zip) = (0u64, entrees.iter().map(|e| e.taille).sum::<u64>());
    for e in &entrees {
        let rel = rel_de(e);
        let vers = contenu.join(&rel);
        if let Some(p) = vers.parent() {
            std::fs::create_dir_all(p)?;
        }
        if source.poser(e, &vers)? != e.taille {
            let _ = std::fs::remove_dir_all(&travail);
            return Err(Erreur::Refus("L'archive du mod ne correspond pas à ce qu'elle annonce : rien n'est installé.".into()));
        }
        plan.push((dans_le_jeu(&rel), vers));
        // Un zip se décompresse fichier par fichier ici (un 7z l'a été d'un coup par l'enfant, plus haut).
        if let SourceMod::Zip(..) = source {
            faits_zip += e.taille;
            progres("decompression", faits_zip, total_zip);
        }
    }
    // Ce que le 7z a laissé d'autre (dossiers vides) : retiré, rien n'en sort.
    if let SourceMod::SeptZ(brut) = &source {
        std::fs::remove_dir_all(brut)?;
    }
    drop(source);
    // 2. Comparer au jeu : ajouté, remplacé (l'original copié à l'abri, vérifié), ou identique.
    let remplaces = travail.join("remplaces");
    let mut m = Manifeste {
        cle: cle.clone(),
        mod_nom: mod_nom.into(),
        fichier: f.nom.clone(),
        depot: f.depot.clone(),
        tag: f.tag.clone(),
        sha256_archive: sha256_archive.into(),
        pose_le: quand.into(),
        etat: "en_cours".into(),
        fichiers: Vec::new(),
        dossiers: Vec::new(),
        steam_buildid,
    };
    let mut bilan = Bilan::default();
    let mut a_poser: Vec<(PathBuf, PathBuf, Pose)> = Vec::new();
    for (chemin_jeu, source) in &plan {
        let dest = jeu.join(chemin_jeu);
        // Une jonction ou un lien SOUS le dossier visé mènerait ailleurs : jamais.
        if crate::garde::lien_entre(jeu, &dest) {
            return Err(Erreur::Refus("Un dossier du jeu passe par un lien : Frogtend n'y pose pas de mod.".into()));
        }
        let sha_nouveau = crate::sauvegarde::empreinte(source)?;
        let (sorte, origine) = if dest.is_file() {
            let sha_origine = crate::sauvegarde::empreinte(&dest)?;
            if sha_origine == sha_nouveau {
                bilan.identiques += 1;
                continue;
            }
            let copie = remplaces.join(chemin_jeu);
            if let Some(p) = copie.parent() {
                std::fs::create_dir_all(p)?;
            }
            std::fs::copy(&dest, &copie)?;
            if crate::sauvegarde::empreinte(&copie)? != sha_origine {
                return Err(Erreur::Disque("La copie de sauvegarde d'un fichier du jeu a échoué : rien n'est installé.".into()));
            }
            bilan.remplaces += 1;
            ("remplace", Some(sha_origine))
        } else if dest.exists() {
            return Err(Erreur::Refus(format!("Un dossier du jeu porte le nom d'un fichier du mod ({chemin_jeu}) : rien n'est installé.")));
        } else {
            bilan.ajoutes += 1;
            ("ajoute", None)
        };
        a_poser.push((source.clone(), dest, Pose { chemin: chemin_jeu.clone(), sorte: sorte.into(), sha256_pose: sha_nouveau, sha256_origine: origine }));
    }
    // 3. Les dossiers à créer (notés pour être retirés s'ils redeviennent vides), y compris le dossier visé lui-même.
    let mut a_creer: Vec<PathBuf> = Vec::new();
    for (_, dest, _) in &a_poser {
        let mut d = dest.parent().map(Path::to_path_buf);
        let mut nouveaux = Vec::new();
        while let Some(x) = d {
            if x == jeu || x.exists() || !x.starts_with(jeu) {
                break;
            }
            nouveaux.push(x.clone());
            d = x.parent().map(Path::to_path_buf);
        }
        for x in nouveaux.into_iter().rev() {
            if let Ok(r) = x.strip_prefix(jeu) {
                let r = r.to_string_lossy().replace('\\', "/");
                if !m.dossiers.contains(&r) {
                    m.dossiers.push(r);
                    a_creer.push(x);
                }
            }
        }
    }
    // 4. Le manifeste « en cours » AVANT de toucher au jeu (dossiers, fichiers) ; à partir de là, un échec DÉFAIT ce
    // qui a été fait (les originaux sont remis depuis leur copie, qui n'est effacée que si tout est remis).
    m.fichiers = a_poser.iter().map(|(_, _, p)| p.clone()).collect();
    ecrire_manifeste(garde, &m)?;
    nettoyage.actif = false;
    let echec = |e: Erreur, n: usize, m: &Manifeste| -> Erreur {
        if defaire(jeu, garde, m, n).is_ok() {
            let _ = std::fs::remove_dir_all(&travail);
        }
        e
    };
    for x in &a_creer {
        if let Err(e) = std::fs::create_dir_all(x) {
            return Err(echec(e.into(), 0, &m));
        }
    }
    for (i, (source, dest, _)) in a_poser.iter().enumerate() {
        if let Err(e) = poser_fichier(source, dest) {
            return Err(echec(e, i, &m));
        }
        progres("pose", i as u64 + 1, a_poser.len() as u64);
    }
    m.etat = "installe".into();
    if let Err(e) = ecrire_manifeste(garde, &m) {
        return Err(echec(e, a_poser.len(), &m));
    }
    let _ = std::fs::remove_dir_all(&contenu);
    Ok(bilan)
}

/// Défait les `n` premières poses d'un manifeste (installation interrompue) et retire les dossiers qu'il avait créés
/// (s'ils sont vides). Le manifeste n'est retiré que si tout a été remis.
fn defaire(jeu: &Path, garde: &Path, m: &Manifeste, n: usize) -> Resultat<()> {
    let remplaces = garde.join(&m.cle).join("remplaces");
    for p in m.fichiers.iter().take(n) {
        let dest = jeu.join(&p.chemin);
        if p.sorte == "remplace" {
            poser_fichier(&remplaces.join(&p.chemin), &dest)?;
        } else {
            let _ = std::fs::remove_file(&dest);
        }
    }
    retirer_dossiers_vides(jeu, &m.dossiers);
    std::fs::remove_file(garde.join(format!("{}.json", m.cle)))?;
    Ok(())
}

/// Les dossiers créés par un mod, s'ils sont vides (les plus profonds d'abord).
fn retirer_dossiers_vides(jeu: &Path, dossiers: &[String]) {
    let mut d = dossiers.to_vec();
    d.sort_by_key(|x| std::cmp::Reverse(x.matches('/').count()));
    for x in d {
        let _ = std::fs::remove_dir(jeu.join(&x));
    }
}

/// Désinstalle un mod : chaque fichier posé est retiré (ou son original remis) SEULEMENT s'il n'a pas changé ; un
/// fichier modifié depuis (un réglage de la personne) est gardé et copié à l'abri. L'original d'un fichier remplacé
/// n'est JAMAIS effacé tant qu'il n'a pas été remis (fichier modifié ou supprimé par la personne : sa copie reste dans
/// `<jeu>\Frogtend\mods\<clé>\remplaces\`). Rend les fichiers gardés.
pub fn desinstaller(jeu: &Path, cle: &str) -> Resultat<Vec<String>> {
    desinstaller_dans(jeu, &dossier_mods(jeu), cle, false).map(|r| r.gardes)
}

/// Ce que la désinstallation a laissé.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Retrait {
    /// Modifiés par la personne depuis la pose : gardés, et copiés à l'abri.
    pub gardes: Vec<String>,
    /// Remplacés par la boutique depuis la pose (une mise à jour) : laissés tels quels, jamais pris pour ceux de la
    /// personne.
    pub laisses_a_la_boutique: Vec<String>,
}

/// `desinstaller`, d'après le dossier de garde ; `apres_maj` : la boutique a mis le jeu à jour depuis la pose, donc un
/// fichier qui n'est plus celui du mod est celui de la boutique (laissé, sans copie).
pub fn desinstaller_dans(jeu: &Path, garde: &Path, cle: &str, apres_maj: bool) -> Resultat<Retrait> {
    let m = installes_dans(garde).into_iter().find(|m| m.cle == cle).ok_or_else(|| Erreur::Introuvable("Ce mod n'est pas installé dans ce jeu.".into()))?;
    let travail = garde.join(&m.cle);
    let mut laisses = Vec::new();
    let remplaces = travail.join("remplaces");
    let gardes_dir = travail.join("gardes");
    let mut gardes = Vec::new();
    // Les originaux remis (leur copie peut alors être effacée) ; tout autre original reste à l'abri.
    let mut remis: Vec<PathBuf> = Vec::new();
    let mut originaux_restants = false;
    for p in &m.fichiers {
        let dest = jeu.join(&p.chemin);
        let origine = remplaces.join(&p.chemin);
        if crate::garde::lien_entre(jeu, &dest) {
            return Err(Erreur::Refus("Un dossier du jeu passe par un lien : Frogtend n'y touche pas.".into()));
        }
        if !dest.is_file() {
            // Supprimé par la personne : rien à retirer ; l'original (s'il y en avait un) reste à l'abri.
            originaux_restants |= p.sorte == "remplace" && origine.is_file();
            continue;
        }
        let actuel = crate::sauvegarde::empreinte(&dest)?;
        // Déjà remis (une désinstallation reprise après une coupure).
        if p.sorte == "remplace" && p.sha256_origine.as_deref() == Some(actuel.as_str()) {
            remis.push(origine);
            continue;
        }
        if actuel != p.sha256_pose && apres_maj && p.sorte == "remplace" {
            // La version de la boutique (elle ne connaît que SES fichiers : un fichier ajouté par le mod et changé est
            // celui de la personne, traité plus bas) : laissée telle quelle (l'original gardé reste à l'abri).
            laisses.push(p.chemin.clone());
            originaux_restants |= p.sorte == "remplace" && origine.is_file();
            continue;
        }
        if actuel != p.sha256_pose {
            let copie = gardes_dir.join(&p.chemin);
            if let Some(d) = copie.parent() {
                std::fs::create_dir_all(d)?;
            }
            std::fs::copy(&dest, &copie)?;
            gardes.push(p.chemin.clone());
            originaux_restants |= p.sorte == "remplace" && origine.is_file();
            continue;
        }
        if p.sorte == "remplace" {
            if !origine.is_file() {
                return Err(Erreur::Disque(format!("L'original de {} n'est plus à l'abri : rien n'est retiré.", p.chemin)));
            }
            poser_fichier(&origine, &dest)?;
            remis.push(origine);
        } else {
            std::fs::remove_file(&dest)?;
        }
    }
    retirer_dossiers_vides(jeu, &m.dossiers);
    std::fs::remove_file(garde.join(format!("{}.json", m.cle)))?;
    // Seules les copies des originaux REMIS s'effacent ; le reste (originaux non remis, fichiers gardés) demeure.
    for o in remis {
        let _ = std::fs::remove_file(o);
    }
    if !originaux_restants && gardes.is_empty() {
        let _ = std::fs::remove_dir_all(&travail);
    }
    Ok(Retrait { gardes, laisses_a_la_boutique: laisses })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Le vrai fichier « Fusion Fix » de Firehouse (relevé du 19/10).
    fn fusion() -> Value {
        let v: Value = serde_json::from_str(include_str!("../fixtures/simule/triche-ultimate-spider-man.json")).unwrap();
        v["mods"].as_array().unwrap().iter().find(|m| m["nom"] == "Fusion Fix").unwrap()["fichiers"][0].clone()
    }

    fn zip_de(f: &Path, fichiers: &[(&str, &[u8])]) {
        let mut z = zip::ZipWriter::new(std::fs::File::create(f).unwrap());
        for (nom, c) in fichiers {
            if nom.ends_with('/') {
                z.add_directory(*nom, zip::write::SimpleFileOptions::default()).unwrap();
            } else {
                z.start_file(*nom, zip::write::SimpleFileOptions::default()).unwrap();
                z.write_all(c).unwrap();
            }
        }
        z.finish().unwrap();
    }

    /// Un 7z sur mesure (noms bruts : les pièges s'y écrivent tels quels) ; `attributs` pour une entrée lien.
    fn sept_de(f: &Path, fichiers: &[(&str, &[u8])], lien: Option<&str>) {
        let mut w = sevenz_rust::SevenZWriter::create(f).unwrap();
        for (nom, c) in fichiers {
            let mut e = sevenz_rust::SevenZArchiveEntry::new();
            e.name = nom.to_string();
            if nom.ends_with('/') {
                e.name = nom.trim_end_matches('/').to_string();
                e.is_directory = true;
                w.push_archive_entry::<&[u8]>(e, None).unwrap();
                continue;
            }
            e.has_stream = true;
            if lien == Some(*nom) {
                e.has_windows_attributes = true;
                e.windows_attributes = 0x400;
            }
            w.push_archive_entry(e, Some(*c)).unwrap();
        }
        w.finish().unwrap();
    }

    #[test]
    fn un_mod_en_7z_s_installe_et_se_retire_comme_un_zip() {
        let d = tempfile::tempdir().unwrap();
        let jeu = d.path().join("Jeu");
        std::fs::create_dir_all(&jeu).unwrap();
        std::fs::write(jeu.join("jeu.exe"), b"jeu").unwrap();
        std::fs::write(jeu.join("dsound.dll"), b"original").unwrap();
        let a = d.path().join("mod.7z");
        sept_de(&a, &[("MonMod/", b""), ("MonMod/dsound.dll", b"du mod"), ("MonMod/scripts/Fix.asi", b"asi")], None);
        assert_eq!(format_mod(&a), Some(FormatMod::SeptZ));
        let f = FichierMod { nom: "mod.7z".into(), taille: 1, sha256: None, url: String::new(), depot: "nexus/jeu/1".into(), tag: "2".into(), a_confirmer: true, dossier: String::new() };
        let b = installer(&jeu, &a, &f, "Mon mod", "abc", "", true, "2026-10-20").unwrap();
        assert_eq!(b, Bilan { ajoutes: 1, remplaces: 1, identiques: 0 });
        assert_eq!(std::fs::read(jeu.join("dsound.dll")).unwrap(), b"du mod");
        assert_eq!(std::fs::read(jeu.join("scripts/Fix.asi")).unwrap(), b"asi");
        assert!(!dossier_mods(&jeu).join(f.cle()).join("brut").exists(), "le dossier de travail du 7z est retiré");
        desinstaller(&jeu, &f.cle()).unwrap();
        assert_eq!(std::fs::read(jeu.join("dsound.dll")).unwrap(), b"original");
        assert!(!jeu.join("scripts").exists());
    }

    #[test]
    fn l_installation_dit_ou_elle_en_est_pour_un_zip_comme_pour_un_7z() {
        for ext in ["zip", "7z"] {
            let d = tempfile::tempdir().unwrap();
            let jeu = d.path().join("Jeu");
            std::fs::create_dir_all(&jeu).unwrap();
            let a = d.path().join(format!("m.{ext}"));
            let fichiers: &[(&str, &[u8])] = &[("a.dll", b"12345"), ("b/c.ini", b"678")];
            if ext == "zip" { zip_de(&a, fichiers) } else { sept_de(&a, fichiers, None) }
            let vus = std::sync::Mutex::new(Vec::new());
            let f = FichierMod { nom: format!("m.{ext}"), taille: 1, sha256: None, url: String::new(), depot: "x/y".into(), tag: "1".into(), a_confirmer: true, dossier: String::new() };
            installer_avec(&jeu, &a, &f, "M", "abc", "", false, "1", &|e, faits, total| vus.lock().unwrap().push((e, faits, total))).unwrap();
            let vus = vus.into_inner().unwrap();
            assert_eq!(vus.iter().filter(|v| v.0 == "decompression").last(), Some(&("decompression", 8, 8)), "{ext}");
            assert_eq!(vus.last(), Some(&("pose", 2, 2)), "{ext}");
        }
    }

    #[test]
    fn reinstaller_un_mod_retire_ne_perd_ni_l_original_ni_les_fichiers_de_la_personne() {
        let d = tempfile::tempdir().unwrap();
        let jeu = d.path().join("Jeu");
        std::fs::create_dir_all(&jeu).unwrap();
        std::fs::write(jeu.join("dsound.dll"), b"original").unwrap();
        let a = d.path().join("mod.zip");
        zip_de(&a, &[("dsound.dll", b"du mod"), ("Fix.ini", b"ini")]);
        let f = FichierMod::depuis(&fusion()).unwrap();
        installer(&jeu, &a, &f, "Fusion Fix", "abc", "", false, "1").unwrap();
        // La personne change le fichier du mod qui avait remplacé l'original, et son .ini.
        std::fs::write(jeu.join("dsound.dll"), b"a moi").unwrap();
        std::fs::write(jeu.join("Fix.ini"), b"mes reglages").unwrap();
        desinstaller(&jeu, &f.cle()).unwrap();
        let travail = dossier_mods(&jeu).join(f.cle());
        assert!(travail.join("remplaces").join("dsound.dll").exists(), "l'original reste à l'abri");
        // Réinstaller le même mod : l'ancien dossier est mis de côté, jamais effacé.
        installer(&jeu, &a, &f, "Fusion Fix", "abc", "", false, "2").ok();
        let anciens: Vec<_> = std::fs::read_dir(dossier_mods(&jeu)).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().contains("-ancien-")).collect();
        assert_eq!(anciens.len(), 1);
        assert_eq!(std::fs::read(anciens[0].path().join("remplaces").join("dsound.dll")).unwrap(), b"original");
        // Des restes d'une pose arrêtée net (brut, contenu) partent ; jamais remplaces ni gardes.
        let reste = dossier_mods(&jeu).join("0123456789abcdef");
        for s in ["brut", "contenu", "remplaces", "gardes"] {
            std::fs::create_dir_all(reste.join(s)).unwrap();
            std::fs::write(reste.join(s).join("x"), b"x").unwrap();
        }
        ranger_les_restes(&dossier_mods(&jeu));
        assert!(!reste.join("brut").exists() && !reste.join("contenu").exists());
        assert!(reste.join("remplaces").join("x").exists() && reste.join("gardes").join("x").exists());
    }

    #[test]
    fn un_jeu_steam_garde_tout_hors_de_son_dossier_et_se_retire_proprement_apres_une_maj() {
        // Lot B3 : le jeu chez « Steam », la garde dans les données de Frogtend (un autre dossier, ici à côté).
        let d = tempfile::tempdir().unwrap();
        let (jeu, garde) = (d.path().join("steamapps/common/Jeu"), d.path().join("donnees/boutiques/42/mods"));
        std::fs::create_dir_all(&jeu).unwrap();
        std::fs::write(jeu.join("dsound.dll"), b"original").unwrap();
        std::fs::write(jeu.join("data.pak"), b"original pak").unwrap();
        let a = d.path().join("mod.zip");
        zip_de(&a, &[("dsound.dll", b"du mod"), ("data.pak", b"pak du mod"), ("Fix.asi", b"asi")]);
        let f = FichierMod::depuis(&fusion()).unwrap();
        let b = installer_dans(&jeu, &garde, Some("100".into()), &a, &f, "Fusion Fix", "abc", "", false, "1", &|_, _, _| {}).unwrap();
        assert_eq!((b.ajoutes, b.remplaces), (1, 2));
        assert!(!jeu.join("Frogtend").exists(), "rien d'autre que le mod chez Steam");
        assert_eq!(std::fs::read(garde.join(f.cle()).join("remplaces").join("dsound.dll")).unwrap(), b"original");
        let m = &installes_dans(&garde)[0];
        assert_eq!(m.steam_buildid.as_deref(), Some("100"));
        assert!(installes(&jeu).is_empty(), "le dossier du jeu n'a aucun manifeste");

        // Steam met le jeu à jour : il remplace data.pak par SA nouvelle version, laisse dsound.dll (fichier du mod) et
        // Fix.asi (inconnu de lui) ; la personne, elle, a changé Fix.asi (un fichier AJOUTÉ par le mod).
        std::fs::write(jeu.join("data.pak"), b"pak de la v2").unwrap();
        std::fs::write(jeu.join("Fix.asi"), b"mes reglages").unwrap();
        let r = desinstaller_dans(&jeu, &garde, &f.cle(), true).unwrap();
        assert_eq!(r.laisses_a_la_boutique, vec!["data.pak"], "le fichier de Steam est laissé, jamais pris pour celui de la personne");
        assert_eq!(r.gardes, vec!["Fix.asi"], "un ajout du mod changé est à la personne (Steam ne le connaît pas)");
        assert_eq!(std::fs::read(garde.join(f.cle()).join("gardes").join("Fix.asi")).unwrap(), b"mes reglages");
        assert_eq!(std::fs::read(jeu.join("data.pak")).unwrap(), b"pak de la v2");
        assert_eq!(std::fs::read(jeu.join("dsound.dll")).unwrap(), b"original", "l'original remis (Steam le revérifiera)");
        assert!(!garde.join(f.cle()).join("gardes").join("data.pak").exists(), "le fichier de Steam n'est pas copié comme « modifié par la personne »");
        assert!(garde.join(f.cle()).join("remplaces").join("data.pak").exists(), "l'ancien original reste à l'abri");
        assert!(!jeu.join("Frogtend").exists());
    }

    #[test]
    fn un_7z_piege_est_refuse_en_entier_sans_rien_poser() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("m.7z");
        for piege in ["../evil.dll", "C:/x.dll", "scripts/CON.ini", "Frogtend/mods/x.json"] {
            sept_de(&a, &[("bon.txt", b"x"), (piege, b"y")], None);
            assert!(lister(&a).is_err(), "{piege}");
        }
        sept_de(&a, &[("Dsound.dll", b"x"), ("dsound.DLL", b"y")], None);
        assert!(lister(&a).is_err(), "deux noms qui ne diffèrent que par la casse");
        sept_de(&a, &[("bon.txt", b"x"), ("lien.dll", b"C:/Windows")], Some("lien.dll"));
        assert!(lister(&a).is_err(), "une entrée marquée lien (Windows)");
        let lien_unix = crate::sept_z::EntreeSeptZ { nom: "l".into(), taille: 1, dossier: false, anti: false, flux: true, attributs: Some(0x8000 | (0o120777 << 16)), crc: None };
        assert!(lien_unix.est_un_lien(), "un lien Unix (S_IFLNK dans les 16 bits hauts)");
        assert!(!crate::sept_z::EntreeSeptZ { attributs: Some(0x8000 | (0o100644 << 16)), ..lien_unix.clone() }.est_un_lien(), "un fichier Unix ordinaire");
        // Ni zip ni 7z (un rar, par exemple) : refusé, sans rien ouvrir.
        std::fs::write(&a, b"Rar!\x1a\x07\x00reste").unwrap();
        assert_eq!(format_mod(&a), None);
        assert!(lister(&a).is_err());
        // Rien n'a été posé nulle part par ces refus.
        assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 1);
    }

    #[test]
    fn le_vrai_fichier_de_firehouse_se_lit_et_son_adresse_est_admise() {
        let f = FichierMod::depuis(&fusion()).unwrap();
        assert_eq!((f.taille, f.a_confirmer, f.dossier.as_str()), (7115342, false, ""));
        assert_eq!(f.sha256.as_deref(), Some("d1dcb9c987229502e0586753924e012d01421a1221b2aa32233f83e9eb712892"));
        assert!(adresse_admise(&f));
        assert_eq!(f.cle().len(), 16);
        // Autre site, http, autre dépôt : refusés.
        for url in ["http://github.com/ThirteenAG/WidescreenFixesPack/releases/download/usm/x.zip", "https://evil.example/ThirteenAG/WidescreenFixesPack/releases/download/usm/x.zip", "https://github.com/autre/depot/releases/download/usm/x.zip"] {
            assert!(!adresse_admise(&FichierMod { url: url.into(), ..f.clone() }), "{url}");
        }
        assert!(hote_admis("release-assets.githubusercontent.com") && hote_admis("objects.githubusercontent.com") && !hote_admis("githubusercontent.com.evil.example"));
        // Une méthode ou une cible inconnue, ou un dossier dangereux : Frogtend ne sait pas le poser.
        let mut v = fusion();
        v["methode"] = json!("installeur");
        assert!(FichierMod::depuis(&v).is_none());
        let mut v = fusion();
        v["cible"]["dossier"] = json!("../Windows");
        assert!(FichierMod::depuis(&v).is_none());
        // Sans empreinte : permis (✅ Seb 19/10), la taille reste obligatoire.
        let mut v = fusion();
        v["sha256"] = Value::Null;
        assert_eq!(FichierMod::depuis(&v).unwrap().sha256, None);
        v["taille"] = Value::Null;
        assert!(FichierMod::depuis(&v).is_none());
    }

    #[test]
    fn une_archive_piegee_est_refusee_en_entier() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("m.zip");
        for piege in ["../evil.dll", "C:/x.dll", "a:b.txt", "scripts/CON.ini", "nul", "x./y"] {
            zip_de(&a, &[("bon.txt", b"x"), (piege, b"y")]);
            assert!(lister(&a).is_err(), "{piege}");
        }
        zip_de(&a, &[("Dsound.dll", b"x"), ("dsound.DLL", b"y")]);
        assert!(lister(&a).is_err(), "deux noms qui ne diffèrent que par la casse");
        zip_de(&a, &[("dsound.dll", b"x"), ("scripts/", b""), ("scripts/a.asi", b"yy")]);
        let l = lister(&a).unwrap();
        assert_eq!(l, [Entree { chemin: "dsound.dll".into(), taille: 1 }, Entree { chemin: "scripts/a.asi".into(), taille: 2 }]);
        assert_eq!(racine_unique(&l), None);
        zip_de(&a, &[("MonMod/dsound.dll", b"x"), ("MonMod/scripts/a.asi", b"y")]);
        assert_eq!(racine_unique(&lister(&a).unwrap()).as_deref(), Some("MonMod"));
    }

    #[test]
    fn installer_puis_desinstaller_rend_le_jeu_tel_qu_il_etait() {
        let d = tempfile::tempdir().unwrap();
        let jeu = d.path().join("Ultimate Spider-Man");
        std::fs::create_dir_all(jeu.join("scripts")).unwrap();
        std::fs::write(jeu.join("USM.exe"), b"jeu").unwrap();
        std::fs::write(jeu.join("dsound.dll"), b"original").unwrap();
        std::fs::write(jeu.join("scripts").join("autre.asi"), b"a moi").unwrap();
        let a = d.path().join("mod.zip");
        zip_de(&a, &[("dsound.dll", b"du mod"), ("scripts/", b""), ("scripts/Fix.asi", b"asi"), ("scripts/Fix.ini", b"ini"), ("docs/lisezmoi.txt", b"txt")]);
        let f = FichierMod::depuis(&fusion()).unwrap();
        let avant: Vec<(PathBuf, String)> = tous(&jeu);
        let b = installer(&jeu, &a, &f, "Fusion Fix", "abc", "", false, "2026-10-19").unwrap();
        assert_eq!(b, Bilan { ajoutes: 3, remplaces: 1, identiques: 0 });
        assert_eq!(std::fs::read(jeu.join("dsound.dll")).unwrap(), b"du mod");
        assert_eq!(std::fs::read(dossier_mods(&jeu).join(f.cle()).join("remplaces").join("dsound.dll")).unwrap(), b"original");
        let m = &installes(&jeu)[0];
        assert_eq!((m.etat.as_str(), m.fichiers.len(), m.dossiers.clone()), ("installe", 4, vec!["docs".to_string()]));
        // Déjà installé : refusé.
        assert!(installer(&jeu, &a, &f, "Fusion Fix", "abc", "", false, "x").is_err());
        // La personne règle le .ini du mod : à la désinstallation il est GARDÉ (et copié à l'abri).
        std::fs::write(jeu.join("scripts").join("Fix.ini"), b"mes reglages").unwrap();
        let gardes = desinstaller(&jeu, &f.cle()).unwrap();
        assert_eq!(gardes, ["scripts/Fix.ini"]);
        assert_eq!(std::fs::read(jeu.join("dsound.dll")).unwrap(), b"original");
        assert!(!jeu.join("scripts").join("Fix.asi").exists() && !jeu.join("docs").exists());
        assert_eq!(std::fs::read(jeu.join("scripts").join("autre.asi")).unwrap(), b"a moi");
        assert!(installes(&jeu).is_empty());
        std::fs::remove_file(jeu.join("scripts").join("Fix.ini")).unwrap();
        let apres: Vec<(PathBuf, String)> = tous(&jeu).into_iter().filter(|(p, _)| !p.starts_with("Frogtend")).collect();
        assert_eq!(apres, avant, "le jeu redevient identique (empreintes)");
    }

    #[test]
    fn deux_mods_ne_se_marchent_pas_dessus_et_le_dossier_racine_se_retire_sur_demande() {
        let d = tempfile::tempdir().unwrap();
        let jeu = d.path().join("Jeu");
        std::fs::create_dir_all(&jeu).unwrap();
        let a = d.path().join("a.zip");
        zip_de(&a, &[("dsound.dll", b"A")]);
        let f = FichierMod::depuis(&fusion()).unwrap();
        installer(&jeu, &a, &f, "Mod A", "x", "", false, "t").unwrap();
        let g = FichierMod { tag: "autre".into(), ..f.clone() };
        zip_de(&a, &[("dsound.dll", b"B")]);
        assert!(installer(&jeu, &a, &g, "Mod B", "y", "", false, "t").unwrap_err().to_string().contains("Mod A"));
        assert_eq!(std::fs::read(jeu.join("dsound.dll")).unwrap(), b"A");
        // Sans son dossier racine (« MonMod/ ») quand la personne le choisit.
        let h = FichierMod { tag: "h".into(), ..f.clone() };
        zip_de(&a, &[("MonMod/scripts/h.asi", b"h")]);
        installer(&jeu, &a, &h, "Mod H", "z", "", true, "t").unwrap();
        assert!(jeu.join("scripts").join("h.asi").is_file());
    }

    #[test]
    fn l_original_d_un_fichier_remplace_n_est_jamais_perdu() {
        let d = tempfile::tempdir().unwrap();
        let jeu = d.path().join("Jeu");
        std::fs::create_dir_all(&jeu).unwrap();
        std::fs::write(jeu.join("dsound.dll"), b"original").unwrap();
        std::fs::write(jeu.join("autre.dll"), b"autre original").unwrap();
        let a = d.path().join("m.zip");
        zip_de(&a, &[("dsound.dll", b"mod"), ("autre.dll", b"mod 2")]);
        let f = FichierMod::depuis(&fusion()).unwrap();
        installer(&jeu, &a, &f, "M", "x", "", false, "t").unwrap();
        // La personne modifie l'un et supprime l'autre : à la désinstallation, les DEUX originaux restent à l'abri.
        std::fs::write(jeu.join("dsound.dll"), b"modifie").unwrap();
        std::fs::remove_file(jeu.join("autre.dll")).unwrap();
        assert_eq!(desinstaller(&jeu, &f.cle()).unwrap(), ["dsound.dll"]);
        let remplaces = dossier_mods(&jeu).join(f.cle()).join("remplaces");
        assert_eq!(std::fs::read(remplaces.join("dsound.dll")).unwrap(), b"original");
        assert_eq!(std::fs::read(remplaces.join("autre.dll")).unwrap(), b"autre original");
    }

    #[test]
    fn une_desinstallation_reprise_reconnait_un_original_deja_remis() {
        let d = tempfile::tempdir().unwrap();
        let jeu = d.path().join("Jeu");
        std::fs::create_dir_all(&jeu).unwrap();
        std::fs::write(jeu.join("a.dll"), b"original").unwrap();
        let a = d.path().join("m.zip");
        zip_de(&a, &[("a.dll", b"mod"), ("b.txt", b"ajout")]);
        let f = FichierMod::depuis(&fusion()).unwrap();
        installer(&jeu, &a, &f, "M", "x", "", false, "t").unwrap();
        // Coupure au milieu d'une 1re désinstallation : l'original est déjà revenu, le manifeste est resté.
        std::fs::write(jeu.join("a.dll"), b"original").unwrap();
        assert!(desinstaller(&jeu, &f.cle()).unwrap().is_empty(), "pas pris pour une modification de la personne");
        assert_eq!(std::fs::read(jeu.join("a.dll")).unwrap(), b"original");
        assert!(!jeu.join("b.txt").exists() && !dossier_mods(&jeu).join(f.cle()).exists());
    }

    #[test]
    fn un_faux_manifeste_ou_une_archive_qui_vise_frogtend_ne_font_rien() {
        let d = tempfile::tempdir().unwrap();
        let jeu = d.path().join("Jeu");
        std::fs::create_dir_all(&jeu).unwrap();
        let a = d.path().join("m.zip");
        zip_de(&a, &[("dsound.dll", b"x"), ("Frogtend/mods/0123456789abcdef.json", b"{}")]);
        assert!(lister(&a).is_err(), "une archive ne dépose jamais rien dans le dossier de Frogtend");
        assert!(dossier_cible(&jeu, "frogtend/mods").is_err());
        // Un manifeste fabriqué (clé qui sort du jeu, chemin « .. ») : ignoré, rien n'est effacé.
        std::fs::create_dir_all(dossier_mods(&jeu)).unwrap();
        std::fs::write(d.path().join("precieux.txt"), b"a moi").unwrap();
        let faux = Manifeste {
            cle: "../../../../x".into(),
            mod_nom: "Faux".into(),
            fichier: "f.zip".into(),
            depot: String::new(),
            tag: String::new(),
            sha256_archive: String::new(),
            pose_le: String::new(),
            etat: "installe".into(),
            fichiers: vec![Pose { chemin: "../precieux.txt".into(), sorte: "ajoute".into(), sha256_pose: String::new(), sha256_origine: None }],
            dossiers: vec![],
            steam_buildid: None,
        };
        std::fs::write(dossier_mods(&jeu).join("faux.json"), serde_json::to_vec(&faux).unwrap()).unwrap();
        let vrai_nom = Manifeste { cle: "0123456789abcdef".into(), ..faux.clone() };
        std::fs::write(dossier_mods(&jeu).join("0123456789abcdef.json"), serde_json::to_vec(&vrai_nom).unwrap()).unwrap();
        assert!(installes(&jeu).is_empty());
        assert!(desinstaller(&jeu, "0123456789abcdef").is_err());
        assert_eq!(std::fs::read(d.path().join("precieux.txt")).unwrap(), b"a moi");
    }

    #[tokio::test]
    async fn le_telechargement_d_un_mod_reprend_verifie_et_ne_depasse_jamais() {
        use httpmock::prelude::*;
        use sha2::Digest;
        let m = MockServer::start();
        let contenu = b"0123456789".to_vec();
        let sha = format!("{:x}", sha2::Sha256::digest(&contenu));
        let mock = m.mock(|w, t| {
            w.method(GET).path("/mod.zip");
            t.status(200).body(contenu.clone());
        });
        let c = reqwest::Client::new();
        let d = tempfile::tempdir().unwrap();
        let cible = d.path().join("mod.zip");
        // Taille annoncée exacte, empreinte juste : reçu, sans jeton.
        assert_eq!(telecharger_depuis(&c, &m.url("/mod.zip"), 10, true, Some(&sha), &cible, &|_, _| {}).await.unwrap(), 10);
        mock.assert();
        // Plus gros qu'annoncé : refusé, rien ne reste.
        let autre = d.path().join("b.zip");
        assert!(telecharger_depuis(&c, &m.url("/mod.zip"), 5, true, None, &autre, &|_, _| {}).await.is_err());
        assert!(!autre.exists());
        // Empreinte fausse : rien gardé.
        assert!(matches!(telecharger_depuis(&c, &m.url("/mod.zip"), 10, true, Some(&"0".repeat(64)), &autre, &|_, _| {}).await, Err(Erreur::Conflit(_))));
        assert!(!autre.exists() && !PathBuf::from(format!("{}.part", autre.to_string_lossy())).exists());
        // Reprise : un .part de 4 octets, le serveur sert la suite (206).
        let m2 = MockServer::start();
        m2.mock(|w, t| {
            w.method(GET).path("/r.zip").header("range", "bytes=4-");
            t.status(206).header("content-range", "bytes 4-9/10").body(&contenu[4..]);
        });
        let r = d.path().join("r.zip");
        std::fs::write(format!("{}.part", r.to_string_lossy()), &contenu[..4]).unwrap();
        assert_eq!(telecharger_depuis(&c, &m2.url("/r.zip"), 10, true, Some(&sha), &r, &|_, _| {}).await.unwrap(), 10);
        assert_eq!(std::fs::read(&r).unwrap(), contenu);
        // Une reprise qui ne reprend pas où on en était (Content-Range faux) : on repart de zéro, rien de mélangé.
        let m3 = MockServer::start();
        m3.mock(|w, t| {
            w.method(GET).path("/x.zip");
            t.status(206).header("content-range", "bytes 0-5/10").body(&contenu[4..]);
        });
        let x = d.path().join("x.zip");
        let part_x = format!("{}.part", x.to_string_lossy());
        std::fs::write(&part_x, &contenu[..4]).unwrap();
        assert!(telecharger_depuis(&c, &m3.url("/x.zip"), 10, true, Some(&sha), &x, &|_, _| {}).await.is_err());
        assert!(!std::path::Path::new(&part_x).exists());
        // Sans empreinte, jamais de reprise : un début gardé n'est pas réutilisé.
        std::fs::write(&part_x, b"XXXX").unwrap();
        let m4 = MockServer::start();
        m4.mock(|w, t| {
            w.method(GET).path("/y.zip");
            t.status(200).body(contenu.clone());
        });
        assert_eq!(telecharger_depuis(&c, &m4.url("/y.zip"), 10, true, None, &x, &|_, _| {}).await.unwrap(), 10);
        assert_eq!(std::fs::read(&x).unwrap(), contenu);
        // 416 : le reste est retiré ; une redirection rendue telle quelle (hors GitHub) : refusée.
        let m5 = MockServer::start();
        m5.mock(|w, t| {
            w.method(GET).path("/z.zip");
            t.status(416);
        });
        m5.mock(|w, t| {
            w.method(GET).path("/r3.zip");
            t.status(302).header("location", "https://evil.example/x.zip");
        });
        let z = d.path().join("z.zip");
        std::fs::write(format!("{}.part", z.to_string_lossy()), b"ab").unwrap();
        assert!(telecharger_depuis(&c, &m5.url("/z.zip"), 10, true, Some(&sha), &z, &|_, _| {}).await.is_err());
        assert!(!std::path::Path::new(&format!("{}.part", z.to_string_lossy())).exists());
        let sans_suivre = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).build().unwrap();
        assert!(matches!(telecharger_depuis(&sans_suivre, &m5.url("/r3.zip"), 10, true, Some(&sha), &z, &|_, _| {}).await, Err(Erreur::Refus(_))));
        // Une taille annoncée différente de celle de Firehouse : refusée avant d'écrire.
        assert!(telecharger_depuis(&c, &m4.url("/y.zip"), 12, true, None, &d.path().join("w.zip"), &|_, _| {}).await.is_err());
        // Nexus : la taille est un PLAFOND (arrondie en Ko) ; au-delà, refusé et rien n'est gardé.
        assert_eq!(telecharger_plafonne(&c, &m4.url("/y.zip"), 12, &d.path().join("n.zip"), &|_, _| {}).await.unwrap(), 10);
        assert!(telecharger_plafonne(&c, &m4.url("/y.zip"), 9, &d.path().join("n2.zip"), &|_, _| {}).await.is_err());
        assert!(!d.path().join("n2.zip").exists() && !d.path().join("n2.zip.part").exists());
    }

    #[test]
    fn la_regle_de_redirection_ne_suit_que_github_en_https_et_5_fois() {
        let u = |s: &str| reqwest::Url::parse(s).unwrap();
        assert!(suivre_redirection(0, &u("https://release-assets.githubusercontent.com/x")));
        assert!(suivre_redirection(4, &u("https://github.com/a/b")));
        assert!(!suivre_redirection(5, &u("https://github.com/a/b")), "6e saut");
        assert!(!suivre_redirection(0, &u("http://release-assets.githubusercontent.com/x")), "http");
        assert!(!suivre_redirection(0, &u("https://evil.example/x")));
        assert!(!suivre_redirection(0, &u("https://githubusercontent.com.evil.example/x")));
        // L'adresse de départ : le chemin EXACT de la release (dépôt, version, nom).
        let f = FichierMod::depuis(&fusion()).unwrap();
        assert!(!adresse_admise(&FichierMod { tag: "autre".into(), ..f.clone() }));
        assert!(!adresse_admise(&FichierMod { nom: "autre.zip".into(), ..f.clone() }));
        assert!(f.nom_telechargement().starts_with(&f.cle()) && f.nom_telechargement().contains("7115342"));
    }

    /// RÉEL (✅ Seb 19/10) : le vrai « Fusion Fix » téléchargé depuis GitHub par le code de Frogtend (adresse admise,
    /// redirection suivie, taille et empreinte vérifiées), installé dans un FAUX dossier de jeu, puis désinstallé.
    /// `FROGTEND_ESSAI_MODS=<dossier d'essai> cargo test essai_fusion_fix -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_fusion_fix() {
        let d = PathBuf::from(std::env::var("FROGTEND_ESSAI_MODS").unwrap());
        let jeu = d.join("Faux Ultimate Spider-Man");
        std::fs::create_dir_all(&jeu).unwrap();
        std::fs::write(jeu.join("USM.exe"), b"faux jeu").unwrap();
        std::fs::write(jeu.join("dsound.dll"), b"original").unwrap();
        let f = FichierMod::depuis(&fusion()).unwrap();
        let archive = dossier_mods(&jeu).join("telechargements").join(format!("{}.zip", f.cle()));
        let debut = std::time::Instant::now();
        let morceaux = std::sync::atomic::AtomicUsize::new(0);
        let n = telecharger(&f, &archive, &|faits, total| {
            assert!(faits <= total);
            morceaux.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        })
        .await
        .unwrap();
        println!("téléchargement : {} signes de progression", morceaux.load(std::sync::atomic::Ordering::Relaxed));
        assert!(morceaux.load(std::sync::atomic::Ordering::Relaxed) > 1, "la progression arrive pendant le téléchargement");
        println!("TÉLÉCHARGÉ {n} octets en {:?} ; empreinte {}", debut.elapsed(), crate::sauvegarde::empreinte(&archive).unwrap());
        println!("CONTENU {:?}", lister(&archive).unwrap());
        let etapes = std::sync::Mutex::new(Vec::new());
        let b = installer_avec(&jeu, &archive, &f, "Fusion Fix", f.sha256.as_deref().unwrap_or(""), "", false, "2026-10-19", &|e, faits, total| etapes.lock().unwrap().push((e, faits, total))).unwrap();
        let etapes = etapes.into_inner().unwrap();
        println!("pose : {:?} … {:?}", etapes.first(), etapes.last());
        assert!(matches!(etapes.last(), Some(("pose", a, b)) if a == b));
        println!("INSTALLÉ {b:?}");
        for p in ["dsound.dll", "scripts/UltimateSpiderMan.WidescreenFix.asi", "scripts/UltimateSpiderMan.WidescreenFix.ini"] {
            println!("  {p} : {:?} octets", std::fs::metadata(jeu.join(p)).map(|m| m.len()).ok());
        }
        let gardes = desinstaller(&jeu, &f.cle()).unwrap();
        println!("DÉSINSTALLÉ ; gardés {gardes:?} ; dsound.dll = {:?}", String::from_utf8_lossy(&std::fs::read(jeu.join("dsound.dll")).unwrap()));
        assert_eq!(std::fs::read(jeu.join("dsound.dll")).unwrap(), b"original");
        assert!(!jeu.join("scripts").exists());
    }

    fn tous(d: &Path) -> Vec<(PathBuf, String)> {
        let mut r = Vec::new();
        let mut pile = vec![d.to_path_buf()];
        while let Some(x) = pile.pop() {
            for e in std::fs::read_dir(&x).unwrap().flatten() {
                if e.path().is_dir() {
                    pile.push(e.path());
                } else {
                    r.push((e.path().strip_prefix(d).unwrap().to_path_buf(), crate::sauvegarde::empreinte(&e.path()).unwrap()));
                }
            }
        }
        r.sort();
        r
    }
}
