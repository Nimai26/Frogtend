//! Lot 7 (✅ Seb 16-17/11) : ENVOYER à Firehouse les mises à jour et DLC Switch qu'il n'a pas, pour qu'il les serve
//! ensuite à toute la famille (besoin 51, contrat 1.42 ; précisions du besoin 52, réponses de Firehouse du 17/11, corps
//! réels dans docs/EXEMPLES-API-JEUX-V1.md § « Besoin 52 » ; contrat 1.43 : un fichier refusé n'est jamais renvoyé).
//!
//! L'ordre, pour chaque fichier : la route 1 (« ce que Firehouse a ») → les candidats (liste FERMÉE) → l'accord chiffré
//! de la personne (dans l'interface, sur la liste qu'elle a vue) → l'empreinte sha256 (par tranches, annulable, en pause
//! pendant une partie) → la taille et la date relues → `POST /switch/envois` → `PUT` par morceaux (≤ 8 Mo, débit
//! plafonné, pause pendant une partie, fichier revérifié avant chaque morceau) → le suivi (`GET /switch/envois/{id}`
//! toutes les ~10 s pendant « verification »).
//!
//! Jamais envoyé : le jeu de base, les clés, le firmware, les parties, un .xci, un ticket personnel. Jamais de chemin
//! local ni de nom de profil ou de PC dans une requête : `nom` est le nom du FICHIER seul. La file d'attente est propre
//! au profil (son dossier de données, `envois-switch.json`) ; le chemin y reste local.

use crate::erreurs::{Erreur, Resultat};
use crate::firehouse::{Cadence, Client, EnvoiParMorceaux, ReglesMorceaux};
use crate::switch_nca::{CleEntete, ContenuLu, ContenusDuJeu, Forme, Sorte, Ticket};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// La file d'attente, dans le dossier de données du PROFIL.
pub const FICHIER: &str = "envois-switch.json";
/// Un morceau ne dépasse jamais 8 Mo (contrat 1.42 ; au-delà, 413 avant même d'être lu). 8 000 000 octets : sous le
/// plafond quelle que soit la façon de compter le « Mo ».
pub const MORCEAU_MAX: usize = 8_000_000;
/// Le plus petit morceau (une connexion très lente).
pub const MORCEAU_MIN: usize = 256 * 1024;
/// Le premier morceau de CHAQUE série : petit, pour qu'il passe même sur une connexion très lente (256 Ko en moins de
/// 90 s dès 3 Ko/s) ; les suivants suivent le débit mesuré pendant la série (le débit enregistré ne sert qu'à estimer
/// la durée annoncée : la connexion a pu changer depuis).
pub const MORCEAU_INCONNU: usize = MORCEAU_MIN;
/// Chaque morceau vise ~20 s au débit mesuré : un PUT doit finir bien avant le délai de 90 s du client des flux
/// (reqwest compte ce délai depuis l'envoi de la requête jusqu'à la réponse, corps envoyé compris :
/// `PendingRequest::poll`, `read_timeout_fut`, reqwest 0.12.28 async_impl/client.rs).
const SECONDES_PAR_MORCEAU: f64 = 20.0;
/// L'attente entre deux regards pendant une partie.
pub const ATTENTE_PAUSE: Duration = Duration::from_secs(5);
/// L'intervalle du suivi pendant « verification » (contrat 1.42 : ~10 s).
pub const ATTENTE_SUIVI: Duration = Duration::from_secs(10);
/// Au plus ce nombre de regards par passage (10 min) : la suite au passage suivant.
const SUIVIS_MAX: u32 = 60;
/// L'empreinte se calcule par tranches de cette taille (entre deux, la pause et l'arrêt sont regardés).
const TRANCHE_EMPREINTE: u64 = 16 << 20;
/// Les envois finis gardés pour être montrés (les plus récents).
const MAX_FINIS: usize = 200;
/// Un fichier de file plus gros n'est pas lu (abîmé) : il est mis de côté.
const MAX_OCTETS_FILE: u64 = 4 << 20;

/// La `version` envoyée au POST. Besoin 52 (2), répondu par Firehouse le 17/11 : un ENTIER, la version brute du CNMT
/// comme dans le nom (`[v131072]` → 131072) ; un texte est refusé (400). Elle ne sert qu'au pré-contrôle « déjà là »
/// (Firehouse relit la vraie). UNE SEULE fonction à changer si la forme change.
pub fn version_pour_api(v: u32) -> Value {
    json!(v)
}

// --- Ce que Firehouse a (route 1) ---

/// Un contenu servi par Firehouse (route 1). Tout est lu avec indulgence : un champ manquant ne fait rien échouer.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct ContenuServi {
    #[serde(default)]
    pub tid: String,
    #[serde(default)]
    pub version: Value,
    #[serde(default)]
    pub taille: u64,
    #[serde(default)]
    pub sha256: String,
    #[serde(default)]
    pub format: String,
    #[serde(default)]
    pub nom: String,
}

/// La réponse de `GET /switch/{tid_base}/contenus`.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct CeQueFirehouseA {
    #[serde(default)]
    pub tid_base: String,
    /// La fiche du jeu (`media_id`), ou `null` (jeu inconnu de Firehouse, ou caché à ce jeton).
    #[serde(default)]
    pub jeu: Option<i64>,
    #[serde(default)]
    pub majs: Vec<ContenuServi>,
    #[serde(default)]
    pub dlc: Vec<ContenuServi>,
}

/// La route 1 pour un jeu de base.
pub fn route_contenus(tid_base: u64) -> String {
    format!("/switch/{tid_base:016X}/contenus")
}

/// Une version lue chez Firehouse : un entier, ou un texte de chiffres (`"131072"`, `"v131072"`).
fn version_lue(v: &Value) -> Option<u64> {
    match v {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => s.trim().trim_start_matches(['v', 'V']).parse().ok(),
        _ => None,
    }
}

fn tid_lu(s: &str) -> Option<u64> {
    let s = s.trim();
    (s.len() == 16).then(|| u64::from_str_radix(s, 16).ok()).flatten()
}

impl CeQueFirehouseA {
    /// Firehouse sert-il déjà une mise à jour au moins aussi récente que `version` ? Une version servie illisible
    /// compte comme « oui » (dans le doute, rien n'est envoyé).
    pub fn a_la_maj(&self, version: u32) -> bool {
        self.majs.iter().any(|m| version_lue(&m.version).is_none_or(|v| v >= version as u64))
    }

    /// Firehouse sert-il déjà ce DLC (quelle que soit sa version : besoin 52, réponse 2) ?
    pub fn a_le_dlc(&self, tid: u64) -> bool {
        self.dlc.iter().any(|d| tid_lu(&d.tid) == Some(tid))
    }
}

// --- Les candidats (liste FERMÉE) ---

/// Un contenu de ce PC que Firehouse n'a pas, envoyable.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidat {
    pub chemin: PathBuf,
    /// Le nom du fichier seul (ce qui part dans la requête), extension en minuscules.
    pub nom: String,
    /// « maj » ou « dlc ».
    pub role: &'static str,
    pub tid: u64,
    pub version: u32,
    pub taille: u64,
    pub secondes: u64,
    pub nanos: u32,
}

/// Un contenu trouvé qui ne part pas, et pourquoi (en mots simples, pour un public novice).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Ecarte {
    pub nom: String,
    pub raison: String,
}

/// Le tri des contenus d'un jeu face à ce que Firehouse a.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Tri {
    pub a_envoyer: Vec<Candidat>,
    pub ecartes: Vec<Ecarte>,
    /// Combien Firehouse en a déjà (rien à envoyer pour eux).
    pub deja_la: usize,
    /// Une raison qui vaut pour tout le jeu (un jeu importé que Firehouse ne connaît pas).
    pub motif: Option<String>,
}

/// Les raisons dites sur la fiche (« pas envoyé : … »), sans jargon.
pub const RAISON_FORME: &str = "ce n'est pas un fichier de mise à jour ou de DLC qui se partage";
pub const RAISON_PLUSIEURS_DLC: &str = "il contient plusieurs DLC";
pub const RAISON_PLUSIEURS: &str = "il contient plusieurs contenus à la fois";
pub const RAISON_SANS_META: &str = "il ne dit pas ce qu'il contient";
pub const RAISON_ILLISIBLE: &str = "il ne se lit pas correctement";
pub const RAISON_PERSONNEL: &str = "ce fichier est lié à un compte ou une console";
pub const RAISON_VERSION: &str = "sa version est inconnue";

fn nom_de(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
}

/// Le nom envoyé à Firehouse : le nom du fichier seul, son extension en minuscules (`.NSZ` → `.nsz` : le contrat
/// attend `.nsp` ou `.nsz`).
pub fn nom_pour_firehouse(p: &Path) -> String {
    match (p.file_stem(), p.extension()) {
        (Some(s), Some(x)) => format!("{}.{}", s.to_string_lossy(), x.to_string_lossy().to_lowercase()),
        _ => nom_de(p),
    }
}

/// La taille et la date d'un fichier (comme le cache de switch_nca).
pub fn taille_et_date(p: &Path) -> Option<(u64, u64, u32)> {
    let m = std::fs::metadata(p).ok().filter(|m| m.is_file())?;
    let (s, n) = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| (d.as_secs(), d.subsec_nanos())).unwrap_or((0, 0));
    Some((m.len(), s, n))
}

/// Ce qu'est devenu un fichier depuis l'accord.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FichierVu {
    Pareil,
    /// Changé ou disparu : il sort de la file.
    Change,
    /// Le disque ne laisse pas le lire maintenant (utilisé par un autre programme) : réessayé plus tard.
    Inaccessible,
}

fn regarder(e: &Envoi) -> FichierVu {
    let p = Path::new(&e.chemin);
    match std::fs::metadata(p) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => FichierVu::Change,
        Err(_) => FichierVu::Inaccessible,
        Ok(_) if taille_et_date(p) == Some((e.taille, e.secondes, e.nanos)) => FichierVu::Pareil,
        Ok(_) => FichierVu::Change,
    }
}

/// L'extension acceptée pour sa forme : `.nsp` exact (Ryubing compare la casse : un `.NSP` est déjà écarté par la
/// lecture), `.nsz` dans n'importe quelle casse (jamais donné à un émulateur).
fn extension_ok(c: &ContenuLu) -> bool {
    let ext = c.chemin.extension().and_then(|x| x.to_str()).unwrap_or_default();
    match c.forme {
        Forme::Nsp => ext == "nsp",
        Forme::Nsz => ext.eq_ignore_ascii_case("nsz"),
        Forme::Xci => false,
    }
}

/// Le contrôle local d'un fichier qui partirait (lu sur le disque, avec la clé des en-têtes) : seulement un PFS0 (.nsp
/// ou .nsz), un seul méta-contenu lu dans les en-têtes (besoin 52, réponse 3), aucun ticket personnel ni illisible
/// (réponse 9). `Err` : la raison de ne pas l'envoyer, en mots simples.
pub fn controle_local(c: &ContenuLu, cle: &CleEntete) -> std::result::Result<(), String> {
    if !extension_ok(c) || !matches!(c.sorte, Sorte::MiseAJour | Sorte::Dlc(_)) {
        return Err(RAISON_FORME.into());
    }
    match crate::switch_nca::nombre_de_metas(&c.chemin, cle) {
        Ok(1) => {}
        Ok(0) => return Err(RAISON_SANS_META.into()),
        Ok(_) if matches!(c.sorte, Sorte::Dlc(_)) => return Err(RAISON_PLUSIEURS_DLC.into()),
        Ok(_) => return Err(RAISON_PLUSIEURS.into()),
        Err(_) => return Err(RAISON_ILLISIBLE.into()),
    }
    match crate::switch_nca::tickets(&c.chemin) {
        Ok(t) if t.iter().all(|x| *x == Ticket::Commun) => Ok(()),
        Ok(t) if t.contains(&Ticket::Illisible) => Err(RAISON_ILLISIBLE.into()),
        Ok(_) => Err(RAISON_PERSONNEL.into()),
        Err(_) => Err(RAISON_ILLISIBLE.into()),
    }
}

/// Trie les contenus d'un jeu (`c`, lus dans leurs en-têtes avec `cle`) : seulement LA mise à jour la plus récente
/// (.nsp ou .nsz, jamais une ancienne) et les DLC ; ce que Firehouse a déjà, ce qui vient de lui (`venus_de_firehouse` :
/// chemin et taille des fichiers de sa fiche), ce qui est déjà dans la file (`deja_en_file`) et ce qui ne passe pas le
/// contrôle local sont écartés. `jeu_importe` : un jeu importé du disque — si Firehouse ne connaît pas son jeu
/// (`jeu: null`), rien n'est proposé.
pub fn trier(c: &ContenusDuJeu, cle: &CleEntete, servi: &CeQueFirehouseA, venus_de_firehouse: &[(PathBuf, u64)], deja_en_file: &dyn Fn(&Path) -> bool, jeu_importe: bool) -> Tri {
    let mut t = Tri::default();
    if jeu_importe && servi.jeu.is_none() {
        t.motif = Some("Firehouse ne connaît pas ce jeu : ses mises à jour et DLC ne peuvent pas lui être envoyés.".into());
        return t;
    }
    let normal = |p: &Path| p.to_string_lossy().replace('/', "\\").to_lowercase();
    let vient_de_firehouse = |p: &Path, taille: u64| venus_de_firehouse.iter().any(|(v, n)| normal(v) == normal(p) && *n == taille);
    let mut a_voir: Vec<(&ContenuLu, &'static str)> = Vec::new();
    // La mise à jour : la plus récente de toutes, .nsp ou .nsz (un .nsz plus vieux que le .nsp retenu ne part jamais).
    let meilleure = c.maj.iter().chain(c.compresses.iter().filter(|x| x.sorte == Sorte::MiseAJour)).max_by_key(|m| m.version);
    if let Some(m) = meilleure {
        a_voir.push((m, "maj"));
    }
    a_voir.extend(c.dlc.iter().chain(c.compresses.iter().filter(|x| matches!(x.sorte, Sorte::Dlc(_)))).map(|d| (d, "dlc")));
    for (x, role) in a_voir {
        let nom = nom_de(&x.chemin);
        let ecarter = |t: &mut Tri, raison: &str| t.ecartes.push(Ecarte { nom: nom.clone(), raison: raison.into() });
        if x.base != c.base {
            continue;
        }
        let Some((taille, secondes, nanos)) = taille_et_date(&x.chemin) else { continue };
        if vient_de_firehouse(&x.chemin, taille) || deja_en_file(&x.chemin) {
            continue;
        }
        let (tid, version) = match (&x.sorte, role) {
            (Sorte::MiseAJour, "maj") => {
                // Besoin 52 (2) : une mise à jour sans version dans son nom n'est jamais envoyée (0 voudrait dire « déjà là »).
                let Some(v) = x.version else {
                    ecarter(&mut t, RAISON_VERSION);
                    continue;
                };
                if servi.a_la_maj(v) {
                    t.deja_la += 1;
                    continue;
                }
                (c.base + 0x800, v)
            }
            (Sorte::Dlc(ncas), "dlc") => {
                let mut ids: Vec<u64> = ncas.iter().map(|(_, i)| *i).collect();
                ids.sort_unstable();
                ids.dedup();
                let [tid] = ids[..] else {
                    ecarter(&mut t, RAISON_PLUSIEURS_DLC);
                    continue;
                };
                if servi.a_le_dlc(tid) {
                    t.deja_la += 1;
                    continue;
                }
                // Besoin 52 (2) : pour un DLC, la version du nom, ou 0 (elle n'est pas comparée).
                (tid, x.version.unwrap_or(0))
            }
            _ => continue,
        };
        if let Err(raison) = controle_local(x, cle) {
            ecarter(&mut t, &raison);
            continue;
        }
        t.a_envoyer.push(Candidat { chemin: x.chemin.clone(), nom: nom_pour_firehouse(&x.chemin), role, tid, version, taille, secondes, nanos });
    }
    t
}

/// L'accord porte sur la liste MONTRÉE : la liste du moment a-t-elle les mêmes fichiers et le même total ?
pub fn meme_liste(candidats: &[Candidat], noms: &[String], total: u64) -> bool {
    let mut a: Vec<&str> = candidats.iter().map(|c| c.nom.as_str()).collect();
    let mut b: Vec<&str> = noms.iter().map(String::as_str).collect();
    a.sort_unstable();
    b.sort_unstable();
    a == b && candidats.iter().map(|c| c.taille).sum::<u64>() == total
}

/// La durée estimée d'un envoi de `total` octets (`debit` : le débit montant mesuré, en octets par seconde ;
/// `plafonne` : la moitié seulement). `None` : débit pas encore mesuré.
pub fn duree_estimee(total: u64, debit: Option<u64>, plafonne: bool) -> Option<u64> {
    let d = debit.filter(|d| *d > 0)? as f64;
    let effectif = if plafonne { d / 2.0 } else { d };
    Some((total as f64 / effectif).ceil() as u64)
}

// --- La file d'attente persistante ---

pub const ATTENTE: &str = "attente";
pub const EN_COURS: &str = "envoi";
pub const VERIFICATION: &str = "verification";
pub const RANGE: &str = "range";
pub const DEJA_LA: &str = "deja_la";
pub const REFUSE: &str = "refuse";
/// Arrêté (400, 404, 409 sans `recu`, fichier trop gros…) : dit ; le fichier peut être reproposé avec un nouvel accord.
pub const ARRETE: &str = "arrete";

/// Un fichier à envoyer, dans la file du profil. `chemin` ne quitte jamais ce PC.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Envoi {
    pub chemin: String,
    pub taille: u64,
    pub secondes: u64,
    pub nanos: u32,
    pub nom: String,
    /// Le title id du contenu (16 hex).
    pub tid: String,
    /// Celui du jeu de base (la route 1).
    pub tid_base: String,
    pub role: String,
    pub version: u32,
    /// Le jeu dans la ludothèque (pour la fiche).
    pub jeu: i64,
    /// La fiche Firehouse à donner si Firehouse ne reconnaît pas le jeu de base (404 « media_id nécessaire ») : le
    /// numéro d'un jeu de Firehouse visible ; `None` pour un jeu importé.
    #[serde(default)]
    pub media_possible: Option<i64>,
    #[serde(default)]
    pub sha256: Option<String>,
    /// Le numéro de l'envoi chez Firehouse.
    #[serde(default)]
    pub envoi: Option<u64>,
    #[serde(default)]
    pub recu: u64,
    pub etat: String,
    #[serde(default)]
    pub motif: Option<String>,
    pub ajoute_le: String,
}

impl Envoi {
    pub fn fini(&self) -> bool {
        matches!(self.etat.as_str(), RANGE | DEJA_LA | REFUSE | ARRETE)
    }
    fn meme_fichier(&self, p: &Path, taille: u64, secondes: u64, nanos: u32) -> bool {
        self.chemin.eq_ignore_ascii_case(&p.to_string_lossy()) && (self.taille, self.secondes, self.nanos) == (taille, secondes, nanos)
    }
}

/// La file d'un profil.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FileEnvois {
    #[serde(default)]
    pub envois: Vec<Envoi>,
    /// Le débit montant mesuré (octets par seconde, arrondi), pour estimer la durée annoncée.
    #[serde(default)]
    pub debit: Option<u64>,
    /// Les empreintes refusées par Firehouse : jamais renvoyées (un nouveau POST recréerait un envoi refusé).
    #[serde(default)]
    pub refuses: Vec<String>,
    /// Le compte n'a pas le droit d'envoyer (403 : le grade mineur) : la proposition n'est plus montrée…
    #[serde(default)]
    pub interdit: Option<String>,
    /// … tant que le compte du profil (`username#grade` de `/moi`) est celui-là.
    #[serde(default)]
    pub interdit_compte: Option<String>,
}

impl FileEnvois {
    /// Ce fichier (même chemin, taille et date) est-il déjà dans la file — en cours, rangé, refusé ? Un envoi ARRÊTÉ ne
    /// compte pas : le fichier peut être reproposé (un nouvel accord).
    pub fn contient(&self, p: &Path) -> bool {
        taille_et_date(p).is_some_and(|(t, s, n)| self.envois.iter().any(|e| e.etat != ARRETE && e.meme_fichier(p, t, s, n)))
    }

    /// Le refus du droit d'envoyer, s'il vaut pour ce compte (un autre compte ou un autre grade : plus rien).
    pub fn interdit_pour(&self, compte: Option<&str>) -> Option<String> {
        self.interdit.clone().filter(|_| self.interdit_compte.as_deref() == compte)
    }
}

pub fn chemin_file(dossier: &Path) -> PathBuf {
    dossier.join(FICHIER)
}

/// Une erreur du disque sur la file des envois, dite simplement.
fn erreur_file(e: &std::io::Error) -> Erreur {
    Erreur::Disque(format!(
        "La liste des envois à Firehouse de ce profil ne peut pas être lue ou écrite sur ce PC ({}) : rien n'est changé, les envois reprendront plus tard.",
        crate::erreurs::message_disque(e)
    ))
}

/// Lit la file. Absente : vide. Un JSON illisible (ou une file anormalement grosse) est MIS DE CÔTÉ
/// (`envois-switch.abime-<horodatage>.json`), jamais écrasé. Une erreur du DISQUE (file utilisée par un autre
/// programme…) rend une erreur : rien n'est écrit par-dessus la vraie file.
pub fn lire(dossier: &Path) -> Resultat<FileEnvois> {
    let f = chemin_file(dossier);
    let m = match std::fs::metadata(&f) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(FileEnvois::default()),
        Err(e) => return Err(erreur_file(&e)),
    };
    if m.len() <= MAX_OCTETS_FILE {
        let o = std::fs::read(&f).map_err(|e| erreur_file(&e))?;
        if let Ok(l) = serde_json::from_slice::<FileEnvois>(&o) {
            return Ok(l);
        }
    }
    let de_cote = dossier.join(format!("envois-switch.abime-{}.json", crate::noyau::maintenant()));
    std::fs::rename(&f, &de_cote).map_err(|e| erreur_file(&e))?;
    Ok(FileEnvois::default())
}

/// Écrit la file (provisoire puis renommage, relue).
fn ecrire(dossier: &Path, l: &FileEnvois) -> Resultat<()> {
    std::fs::create_dir_all(dossier).map_err(|e| erreur_file(&e))?;
    let f = chemin_file(dossier);
    static NUMERO: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = NUMERO.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let provisoire = dossier.join(format!("{FICHIER}.{}-{n}.tmp", std::process::id()));
    let texte = serde_json::to_vec_pretty(l).map_err(|e| Erreur::Disque(format!("La liste des envois ne s'écrit pas ({e}).")))?;
    std::fs::write(&provisoire, texte).map_err(|e| erreur_file(&e))?;
    if let Err(e) = std::fs::rename(&provisoire, &f) {
        let _ = std::fs::remove_file(&provisoire);
        return Err(erreur_file(&e));
    }
    match std::fs::read(&f).ok().and_then(|o| serde_json::from_slice::<FileEnvois>(&o).ok()) {
        Some(relu) if relu == *l => Ok(()),
        _ => Err(Erreur::Disque("La liste des envois à Firehouse ne se relit pas après l'écriture : les envois reprendront plus tard.".into())),
    }
}

/// Toute lecture-modification-écriture de la file se fait sous ce verrou.
static VERROU: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Change la file sous le verrou, et rend ce que `f` rend. Une file qui ne se lit pas : rien n'est écrit.
pub fn modifier<R>(dossier: &Path, f: impl FnOnce(&mut FileEnvois) -> R) -> Resultat<R> {
    let _v = VERROU.lock().unwrap_or_else(|e| e.into_inner());
    let mut l = lire(dossier)?;
    let r = f(&mut l);
    // Les finis les plus anciens partent au-delà du plafond (jamais un envoi en cours).
    let finis = l.envois.iter().filter(|e| e.fini()).count();
    if finis > MAX_FINIS {
        let mut a_retirer = finis - MAX_FINIS;
        l.envois.retain(|e| {
            if a_retirer > 0 && e.fini() {
                a_retirer -= 1;
                return false;
            }
            true
        });
    }
    ecrire(dossier, &l)?;
    Ok(r)
}

/// Met les candidats acceptés dans la file (`media_possible` : la fiche du jeu, si c'en est une de Firehouse). Rend
/// combien ont été ajoutés. Un fichier déjà en file n'est pas ajouté deux fois ; un envoi ARRÊTÉ du même fichier repart
/// de zéro (nouvel accord).
pub fn ajouter(dossier: &Path, jeu: i64, tid_base: u64, media_possible: Option<i64>, candidats: &[Candidat]) -> Resultat<usize> {
    modifier(dossier, |l| {
        let mut n = 0;
        for c in candidats {
            if let Some(e) = l.envois.iter_mut().find(|e| e.meme_fichier(&c.chemin, c.taille, c.secondes, c.nanos)) {
                if e.etat == ARRETE {
                    e.etat = ATTENTE.into();
                    e.motif = None;
                    e.envoi = None;
                    e.recu = 0;
                    n += 1;
                }
                continue;
            }
            l.envois.push(Envoi {
                chemin: c.chemin.to_string_lossy().to_string(),
                taille: c.taille,
                secondes: c.secondes,
                nanos: c.nanos,
                nom: c.nom.clone(),
                tid: format!("{:016X}", c.tid),
                tid_base: format!("{tid_base:016X}"),
                role: c.role.into(),
                version: c.version,
                jeu,
                media_possible,
                sha256: None,
                envoi: None,
                recu: 0,
                etat: ATTENTE.into(),
                motif: None,
                ajoute_le: crate::noyau::maintenant(),
            });
            n += 1;
        }
        n
    })
}

/// « ✋ Arrêter l'envoi » : les envois NON FINIS de ce jeu sortent de la file (ils pourront être reproposés). Rend
/// combien.
pub fn annuler(dossier: &Path, jeu: i64) -> Resultat<usize> {
    modifier(dossier, |l| {
        let avant = l.envois.len();
        l.envois.retain(|e| e.jeu != jeu || e.fini());
        avant - l.envois.len()
    })
}

// --- Ce que voit la fiche ---

/// Un contenu que Firehouse n'a pas, à proposer.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AEnvoyerVu {
    pub nom: String,
    pub role: String,
    pub taille: u64,
}

/// Un envoi de la file, vu depuis la fiche (jamais de chemin).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EnvoiVu {
    pub nom: String,
    pub role: String,
    pub taille: u64,
    pub recu: u64,
    pub etat: String,
    pub motif: Option<String>,
}

/// Les envois d'un jeu : la file, l'envoi en cours (s'il est à ce jeu et à ce profil), et pourquoi le travail s'est
/// arrêté (seulement si ce jeu a encore un envoi à faire).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Suivi {
    pub envois: Vec<EnvoiVu>,
    pub en_cours: Option<Progres>,
    pub arret: Option<String>,
}

/// L'examen d'un jeu à l'ouverture de son panneau 📦.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Examen {
    /// `rien` (rien à dire), `simule` (mode simulé), `interdit` (compte sans droit d'envoi), `injoignable`, `ok`.
    pub etat: String,
    pub motif: Option<String>,
    pub a_envoyer: Vec<AEnvoyerVu>,
    pub total: u64,
    /// Secondes estimées au débit montant mesuré (`None` : pas encore mesuré).
    pub duree: Option<u64>,
    /// Vrai : le débit est plafonné à la moitié du débit mesuré.
    pub plafonne: bool,
    pub ecartes: Vec<Ecarte>,
    pub deja_la: usize,
    pub suivi: Suivi,
}

impl Examen {
    /// L'examen d'après un tri (`debit` : celui de la file).
    pub fn depuis(t: &Tri, debit: Option<u64>, plafonne: bool, suivi: Suivi) -> Self {
        let total = t.a_envoyer.iter().map(|c| c.taille).sum();
        Examen {
            etat: "ok".into(),
            motif: t.motif.clone(),
            a_envoyer: t.a_envoyer.iter().map(|c| AEnvoyerVu { nom: c.nom.clone(), role: c.role.into(), taille: c.taille }).collect(),
            total,
            duree: duree_estimee(total, debit, plafonne),
            plafonne,
            ecartes: t.ecartes.clone(),
            deja_la: t.deja_la,
            suivi,
        }
    }
}

/// L'état partagé du travail (un seul à la fois, pour tout Frogtend).
#[derive(Default)]
pub struct Travail {
    pub actif: std::sync::atomic::AtomicBool,
    /// Un autre profil (ou une nouvelle demande) attend que le travail en cours finisse : il sera relancé.
    pub relancer: std::sync::atomic::AtomicBool,
    /// « ✋ Arrêter l'envoi » : l'envoi en cours s'arrête (lu par `Rythme::arrete`).
    pub annuler: std::sync::atomic::AtomicBool,
    /// Le profil dont la file est en travail.
    pub profil: std::sync::Mutex<Option<String>>,
    pub progres: std::sync::Mutex<Option<Progres>>,
    /// Pourquoi le dernier travail s'est arrêté : (profil, motif).
    pub arret: std::sync::Mutex<Option<(String, String)>>,
}

fn verrou<T>(m: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Travail {
    /// Commence le travail pour `profil` ; `false` s'il tourne déjà (il sera relancé à sa fin).
    pub fn commencer(&self, profil: &str) -> bool {
        use std::sync::atomic::Ordering::SeqCst;
        if self.actif.swap(true, SeqCst) {
            self.relancer.store(true, SeqCst);
            return false;
        }
        *verrou(&self.profil) = Some(profil.to_string());
        *verrou(&self.arret) = None;
        *verrou(&self.progres) = None;
        self.annuler.store(false, SeqCst);
        true
    }

    /// La fin du travail de `profil` (`fin` : ce qu'a rendu `travailler`). Rend vrai s'il faut relancer (un profil
    /// ouvert entre-temps, ou un arrêt demandé alors que d'autres envois attendent).
    pub fn finir(&self, profil: &str, fin: Option<Arret>) -> bool {
        use std::sync::atomic::Ordering::SeqCst;
        *verrou(&self.progres) = None;
        if let Some(Arret::Motif(m)) = fin {
            *verrou(&self.arret) = Some((profil.to_string(), m));
        }
        *verrou(&self.profil) = None;
        let annule = self.annuler.swap(false, SeqCst);
        self.actif.store(false, SeqCst);
        self.relancer.swap(false, SeqCst) || annule
    }

    /// Le profil `profil` a-t-il en ce moment l'envoi du jeu `jeu` ?
    pub fn en_cours_pour(&self, profil: &str, jeu: i64) -> bool {
        verrou(&self.progres).as_ref().is_some_and(|p| p.profil == profil && p.jeu == jeu)
    }
}

/// Les envois du jeu `jeu` dans la file du profil `profil` (`dossier`), l'envoi en cours s'il est à lui, et l'arrêt
/// du travail s'il concerne ce profil et que ce jeu a encore un envoi à faire.
pub fn suivi_du_jeu(dossier: &Path, profil: &str, jeu: i64, travail: &Travail) -> Resultat<Suivi> {
    let l = lire(dossier)?;
    let envois: Vec<EnvoiVu> = l
        .envois
        .iter()
        .filter(|e| e.jeu == jeu)
        .map(|e| EnvoiVu { nom: e.nom.clone(), role: e.role.clone(), taille: e.taille, recu: e.recu, etat: e.etat.clone(), motif: e.motif.clone() })
        .collect();
    let en_cours = verrou(&travail.progres).clone().filter(|p| p.jeu == jeu && p.profil == profil);
    let reste = l.envois.iter().any(|e| e.jeu == jeu && !e.fini());
    let arret = verrou(&travail.arret).clone().filter(|(p, _)| p == profil && reste && en_cours.is_none()).map(|(_, m)| m);
    Ok(Suivi { envois, en_cours, arret })
}

// --- Le travail en arrière-plan ---

/// Où en est l'envoi en cours (pour la fiche).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Progres {
    /// Le profil dont c'est l'envoi (jamais montré : il sert à ne montrer l'envoi qu'à ce profil).
    #[serde(skip)]
    pub profil: String,
    pub jeu: i64,
    pub nom: String,
    /// « empreinte » (lecture du fichier), « envoi », « pause » (une partie en cours), « verification ».
    pub etape: String,
    pub faits: u64,
    pub total: u64,
    /// Octets par seconde (débit brut mesuré), s'il est connu.
    pub debit: Option<f64>,
    /// Secondes restantes estimées, si le débit est connu.
    pub restant: Option<u64>,
}

/// Ce que l'envoi demande à l'application : savoir si le PC est occupé (partie, lancement, émulateur ouvert seul), s'il
/// faut s'arrêter (profil changé, « ✋ Arrêter l'envoi »), si le débit est plafonné, quel est le compte du profil ;
/// attendre ; dire où il en est. Une imitation dans les tests (jamais d'attente réelle).
#[allow(async_fn_in_trait)]
pub trait Rythme {
    fn occupe(&self) -> bool;
    fn arrete(&self) -> bool;
    fn plafonne(&self) -> bool;
    async fn dormir(&self, d: Duration);
    fn progres(&self, p: &Progres);
    fn journaliser(&self, _m: &str) {}
    /// Le compte Firehouse du profil (`username#grade`), s'il est connu.
    fn compte(&self) -> Option<String> {
        None
    }
    /// La durée d'un morceau telle qu'elle compte pour le débit (les tests la fixent ; en vrai, la durée mesurée).
    fn mesurer(&self, reelle: Duration) -> Duration {
        reelle
    }
}

/// Pourquoi tout le travail s'arrête (les envois restent dans la file).
#[derive(Debug, Clone, PartialEq)]
pub enum Arret {
    /// Le profil a changé, ou l'arrêt est demandé : rien à dire.
    Demande,
    /// À dire à la personne (jeton refusé, Firehouse injoignable, compte sans droit…).
    Motif(String),
}

/// L'issue du traitement d'un fichier.
enum Issue {
    Suivant,
    Arreter(Arret),
}

/// Firehouse injoignable : tout attend la prochaine occasion.
pub const PLUS_TARD: &str = "Firehouse n'est pas joignable pour l'instant : l'envoi reprendra plus tard, là où il en était.";
/// Le fichier de la personne est utilisé par un autre programme : il attend, les autres passent.
pub const SOURCE_OCCUPEE: &str = "Ce fichier est utilisé par un autre programme : il sera réessayé plus tard.";

fn erreur_vers_issue(e: Erreur) -> std::result::Result<String, Issue> {
    match e {
        // 401 : une seule tentative, tout s'arrête et c'est dit.
        Erreur::JetonRefuse(m) => Err(Issue::Arreter(Arret::Motif(m))),
        Erreur::Perimee(_) => Err(Issue::Arreter(Arret::Demande)),
        // Un Firehouse d'une autre version du contrat : tout attend, les envois restent en file.
        Erreur::Version(m) => Err(Issue::Arreter(Arret::Motif(m))),
        // La file des envois elle-même (son message est déjà dit simplement) : tout attend.
        Erreur::Disque(m) => Err(Issue::Arreter(Arret::Motif(m))),
        // Un envoi qui ne se cale pas, ou une fin non confirmée : arrêté pour CE fichier, avec ce message.
        Erreur::Serveur(m) if m.contains("ne se cale pas") || m.contains("n'a pas confirmé") => Ok(m),
        // Hors ligne, 503, réponse illisible : reste dans la file, repart à la prochaine occasion (jamais de boucle).
        Erreur::Reseau(_) | Erreur::Serveur(_) => Err(Issue::Arreter(Arret::Motif(PLUS_TARD.into()))),
        // Le reste (400, 404, 409 sans `recu`, 413…) : arrêté pour CE fichier, avec le motif de Firehouse.
        autre => Ok(autre.to_string()),
    }
}

/// Le motif d'un corps d'erreur (`{"detail": …}`), tel quel.
fn detail(corps: &[u8]) -> Option<String> {
    let v: Value = serde_json::from_slice(corps).ok()?;
    v["detail"].as_str().or(v["motif"].as_str()).map(String::from).filter(|s| !s.trim().is_empty())
}

/// Un refus « abandonné (sans nouvelles) » (7 jours sans morceau) : PAS définitif, l'envoi peut être reproposé
/// (besoin 52, réponse 5). Tout autre refus l'est.
pub fn refus_par_abandon(motif: &str) -> bool {
    let m = motif.to_lowercase();
    m.contains("abandonn") && m.contains("sans nouvelles")
}

/// Un 507 DÉFINITIF pour ce fichier : « fichier trop gros ». Tout autre 507 est passager : le plafond des envois EN
/// COURS de la personne (100 Go), ou le disque des jeux de Firehouse plein (4.0.1).
pub fn fichier_trop_gros(detail: &str) -> bool {
    detail.to_lowercase().contains("trop gros")
}

/// Un 507 pendant les morceaux : Firehouse n'a plus de place pour l'instant.
pub const PLUS_DE_PLACE: &str = "Firehouse n'a plus de place pour les envois pour l'instant : l'envoi reprendra plus tard, là où il en était.";

/// Le corps EXACT du POST : rien d'autre (jamais de chemin, de profil ni de PC).
pub fn corps_post(e: &Envoi, sha: &str, media_id: Option<i64>) -> Value {
    let mut v = json!({
        "tid": e.tid,
        "role": e.role,
        "version": version_pour_api(e.version),
        "nom": e.nom,
        "taille": e.taille,
        "sha256": sha,
    });
    if let Some(m) = media_id {
        v["media_id"] = json!(m);
    }
    v
}

fn progres(e: &Envoi, etape: &str, faits: u64) -> Progres {
    Progres { jeu: e.jeu, nom: e.nom.clone(), etape: etape.into(), faits, total: e.taille, ..Default::default() }
}

/// L'empreinte sha256 d'un fichier, par tranches de 16 Mo (chacune lue à part du fil principal) : entre deux, la pause
/// pendant une partie et l'arrêt sont regardés, la progression est dite. `None` : arrêtée. Une erreur du DISQUE vient
/// du fichier de la personne.
async fn empreinte(e: &Envoi, rythme: &impl Rythme) -> Resultat<Option<String>> {
    let mut h = Sha256::new();
    let mut position = 0u64;
    while position < e.taille {
        if !attendre_libre(e, rythme, position).await {
            return Ok(None);
        }
        rythme.progres(&progres(e, "empreinte", position));
        let chemin = PathBuf::from(&e.chemin);
        let (h2, lus) = tokio::task::spawn_blocking(move || -> Resultat<(Sha256, u64)> {
            use std::io::{Read, Seek, SeekFrom};
            let mut f = std::fs::File::open(&chemin)?;
            f.seek(SeekFrom::Start(position))?;
            let mut tampon = vec![0u8; 1 << 20];
            let mut lus = 0u64;
            while lus < TRANCHE_EMPREINTE {
                let n = f.read(&mut tampon)?;
                if n == 0 {
                    break;
                }
                h.update(&tampon[..n]);
                lus += n as u64;
            }
            Ok((h, lus))
        })
        .await
        .map_err(|_| Erreur::Disque("La lecture du fichier s'est arrêtée brutalement.".into()))??;
        h = h2;
        if lus == 0 {
            return Err(Erreur::Disque("Le fichier a raccourci pendant sa lecture.".into()));
        }
        position += lus;
    }
    Ok(Some(h.finalize().iter().map(|o| format!("{o:02x}")).collect()))
}

/// Attend que le PC soit libre (partie, lancement, émulateur ouvert seul) ; `false` : arrêt demandé.
async fn attendre_libre(e: &Envoi, rythme: &impl Rythme, faits: u64) -> bool {
    loop {
        if rythme.arrete() {
            return false;
        }
        if !rythme.occupe() {
            return true;
        }
        rythme.progres(&progres(e, "pause", faits));
        rythme.dormir(ATTENTE_PAUSE).await;
    }
}

/// La cadence des morceaux d'un envoi Switch : pause pendant une partie, fichier revérifié, débit plafonné à la moitié
/// du débit mesuré PENDANT CETTE SÉRIE (attente après chaque morceau), taille du morceau adaptée à ce débit (la série
/// commence à `MORCEAU_INCONNU`), progression et débit enregistrés.
struct CadenceSwitch<'a, R: Rythme> {
    rythme: &'a R,
    dossier: &'a Path,
    envoi: Envoi,
    /// Le débit mesuré pendant cette série (`None` au premier morceau).
    debit: Option<f64>,
    attente: Duration,
    morceau_min: usize,
    morceau_max: usize,
    /// Le fichier a changé (taille ou date) pendant l'envoi.
    change: bool,
    /// Le disque ne laisse plus lire le fichier (utilisé par un autre programme).
    inaccessible: bool,
}

impl<R: Rythme> CadenceSwitch<'_, R> {
    fn taille_morceau(&self) -> usize {
        match self.debit {
            Some(d) if d > 0.0 => ((d * SECONDES_PAR_MORCEAU) as usize).clamp(self.morceau_min, self.morceau_max),
            _ => MORCEAU_INCONNU.clamp(self.morceau_min, self.morceau_max),
        }
    }
}

impl<R: Rythme> Cadence for CadenceSwitch<'_, R> {
    async fn avant(&mut self, position: u64) -> Resultat<usize> {
        if !self.attente.is_zero() {
            let a = std::mem::take(&mut self.attente);
            self.rythme.dormir(a).await;
        }
        if !attendre_libre(&self.envoi, self.rythme, position).await {
            return Err(Erreur::Perimee("envoi arrêté".into()));
        }
        // Le fichier ne doit pas avoir changé depuis son empreinte.
        match regarder(&self.envoi) {
            FichierVu::Pareil => Ok(self.taille_morceau()),
            FichierVu::Change => {
                self.change = true;
                Err(Erreur::Perimee("fichier changé".into()))
            }
            FichierVu::Inaccessible => {
                self.inaccessible = true;
                Err(Erreur::Perimee("fichier inaccessible".into()))
            }
        }
    }

    fn apres(&mut self, recu: u64, octets: u64, duree: Duration) {
        let secondes = self.rythme.mesurer(duree).as_secs_f64().max(0.001);
        let mesure = octets as f64 / secondes;
        let avant = self.debit;
        self.debit = Some(avant.map_or(mesure, |d| 0.7 * d + 0.3 * mesure));
        // Plafond : la moitié du débit connu AVANT ce morceau (celui qui l'a dimensionné), ou de celui de ce morceau.
        if self.rythme.plafonne() {
            let reference = avant.unwrap_or(mesure);
            let voulu = octets as f64 / (reference / 2.0);
            self.attente = Duration::from_secs_f64((voulu - secondes).max(0.0));
        }
        self.envoi.recu = recu;
        let reste = self.envoi.taille.saturating_sub(recu);
        let debit = self.debit.map(|d| d.round() as u64);
        self.rythme.progres(&Progres {
            debit: self.debit,
            restant: duree_estimee(reste, debit, self.rythme.plafonne()),
            ..progres(&self.envoi, "envoi", recu)
        });
        let chemin = self.envoi.chemin.clone();
        let _ = modifier(self.dossier, |l| {
            l.debit = debit;
            if let Some(e) = l.envois.iter_mut().find(|e| e.chemin == chemin) {
                e.recu = recu;
            }
        });
    }
}

/// Met à jour l'envoi `chemin` de la file (rien s'il n'y est plus : annulé entre-temps).
fn noter(dossier: &Path, chemin: &str, f: impl FnOnce(&mut Envoi)) -> Resultat<()> {
    modifier(dossier, |l| {
        if let Some(e) = l.envois.iter_mut().find(|e| e.chemin == chemin) {
            f(e);
        }
    })
}

/// Les morceaux : jusqu'à 8 Mo, plus petits sur une connexion lente (réglables pour les tests).
#[derive(Debug, Clone, Copy)]
pub struct Bornes {
    pub min: usize,
    pub max: usize,
}

impl Default for Bornes {
    fn default() -> Self {
        Bornes { min: MORCEAU_MIN, max: MORCEAU_MAX }
    }
}

/// Fait avancer UN envoi de la file autant que possible.
async fn traiter(client: &Client, dossier: &Path, e: Envoi, bornes: Bornes, rythme: &impl Rythme) -> Issue {
    match traiter_ou_erreur(client, dossier, e.clone(), bornes, rythme).await {
        Ok(i) => i,
        Err(err) => match erreur_vers_issue(err) {
            Ok(motif) => {
                rythme.journaliser(&format!("envoi Switch « {} » arrêté : {motif}", e.nom));
                match noter(dossier, &e.chemin, |x| {
                    x.etat = ARRETE.into();
                    x.motif = Some(motif);
                }) {
                    Ok(()) => Issue::Suivant,
                    Err(err) => Issue::Arreter(Arret::Motif(err.to_string())),
                }
            }
            Err(i) => i,
        },
    }
}

/// Le fichier de la personne est utilisé par un autre programme : il attend (dit), on passe au suivant.
fn source_occupee(dossier: &Path, e: &Envoi, rythme: &impl Rythme) -> Resultat<Issue> {
    rythme.journaliser(&format!("envoi Switch « {} » : fichier utilisé par un autre programme, réessayé plus tard", e.nom));
    noter(dossier, &e.chemin, |x| x.motif = Some(SOURCE_OCCUPEE.into()))?;
    Ok(Issue::Suivant)
}

async fn traiter_ou_erreur(client: &Client, dossier: &Path, mut e: Envoi, bornes: Bornes, rythme: &impl Rythme) -> Resultat<Issue> {
    let chemin = PathBuf::from(&e.chemin);
    let (cle, nom) = (e.chemin.clone(), e.nom.clone());
    let journal = |m: &str| rythme.journaliser(&format!("envoi Switch « {nom} » : {m}"));
    // Un fichier changé ou disparu depuis l'accord : il sort de la file (il sera reproposé, avec un nouvel accord).
    let oublier = || -> Resultat<Issue> {
        journal("le fichier a changé depuis l'accord : il sort de la file");
        modifier(dossier, |l| l.envois.retain(|x| x.chemin != cle))?;
        Ok(Issue::Suivant)
    };
    if e.etat != VERIFICATION || e.envoi.is_none() {
        match regarder(&e) {
            FichierVu::Pareil => {}
            FichierVu::Change => return oublier(),
            FichierVu::Inaccessible => return source_occupee(dossier, &e, rythme),
        }
        // Route 1 AVANT chaque envoi (besoin 52, réponse 7 : sans coût) : déjà là entre-temps ?
        let tid_base = u64::from_str_radix(&e.tid_base, 16).map_err(|_| Erreur::Refus("identifiant de jeu illisible".into()))?;
        let servi: CeQueFirehouseA = client.obtenir_json(&route_contenus(tid_base)).await?;
        let tid = u64::from_str_radix(&e.tid, 16).unwrap_or(0);
        let deja = if e.role == "maj" { servi.a_la_maj(e.version) } else { servi.a_le_dlc(tid) };
        if deja {
            noter(dossier, &e.chemin, |x| {
                x.etat = DEJA_LA.into();
                x.motif = Some("Firehouse l'a déjà (envoyé depuis un autre PC, peut-être).".into());
            })?;
            return Ok(Issue::Suivant);
        }
        // L'empreinte (gardée tant que le fichier ne change pas).
        let sha = match e.sha256.clone() {
            Some(s) => s,
            None => {
                let s = match empreinte(&e, rythme).await {
                    Ok(Some(s)) => s,
                    Ok(None) => return Ok(Issue::Arreter(Arret::Demande)),
                    Err(Erreur::Disque(_)) => return source_occupee(dossier, &e, rythme),
                    Err(err) => return Err(err),
                };
                match regarder(&e) {
                    FichierVu::Pareil => {}
                    FichierVu::Change => return oublier(),
                    FichierVu::Inaccessible => return source_occupee(dossier, &e, rythme),
                }
                noter(dossier, &e.chemin, |x| x.sha256 = Some(s.clone()))?;
                s
            }
        };
        e.sha256 = Some(sha.clone());
        if lire(dossier)?.refuses.contains(&sha) {
            noter(dossier, &e.chemin, |x| {
                x.etat = REFUSE.into();
                x.motif.get_or_insert_with(|| "Firehouse a déjà refusé ce fichier.".into());
            })?;
            return Ok(Issue::Suivant);
        }
        // Le POST (même sha256 = reprise) ; la fiche n'est donnée que si Firehouse ne reconnaît pas le jeu.
        let mut media = if servi.jeu.is_none() { e.media_possible } else { None };
        let reponse = loop {
            let b = client.brute(reqwest::Method::POST, "/switch/envois", Some(serde_json::to_vec(&corps_post(&e, &sha, media)).unwrap_or_default()), &[("Content-Type", "application/json".into())]).await?;
            match b.statut {
                200..=299 => break serde_json::from_slice::<Value>(&b.octets).map_err(|_| Erreur::Serveur("Firehouse a envoyé une réponse illisible.".into()))?,
                // 403 : le compte de ce profil n'a pas le droit d'envoyer (grade mineur) — tout s'arrête, les envois
                // de la file sont arrêtés avec ce motif, et la proposition n'est plus montrée à ce compte.
                403 => {
                    let m = "Ton compte Firehouse ne permet pas d'envoyer des mises à jour ou des DLC.".to_string();
                    let compte = rythme.compte();
                    modifier(dossier, |l| {
                        for x in l.envois.iter_mut().filter(|x| !x.fini()) {
                            x.etat = ARRETE.into();
                            x.motif = Some(m.clone());
                        }
                        l.interdit = Some(m.clone());
                        l.interdit_compte = compte;
                    })?;
                    return Ok(Issue::Arreter(Arret::Motif(m)));
                }
                404 if detail(&b.octets).is_some_and(|d| d.contains("media_id")) && media.is_none() && e.media_possible.is_some() => {
                    media = e.media_possible;
                }
                404 => {
                    let m = detail(&b.octets).unwrap_or_else(|| "Firehouse ne connaît pas ce jeu (ou il ne t'est pas visible).".into());
                    return Err(Erreur::Refus(m));
                }
                // 507 : le motif de Firehouse tel quel. Le plafond des envois EN COURS de la personne est passager :
                // l'envoi attend le prochain passage ; « fichier trop gros » l'arrête.
                507 => {
                    let d = detail(&b.octets).unwrap_or_else(|| "Firehouse n'a plus de place pour les envois.".into());
                    // 4.0.1 : le disque des jeux de Firehouse plein (marge de 2 Go) est passager lui aussi ; seul
                    // « fichier trop gros » arrête CE fichier pour de bon.
                    if !fichier_trop_gros(&d) {
                        noter(dossier, &e.chemin, |x| x.motif = Some(d.clone()))?;
                        return Ok(Issue::Arreter(Arret::Motif(d)));
                    }
                    return Err(Erreur::Refus(d));
                }
                code => return Err(crate::firehouse::erreur_du_statut(code, &b.octets)),
            }
        };
        // L'ordre du contrat 1.43 : `deja_la` d'abord, puis `refuse` (CE fichier, même sha256, déjà refusé pour une
        // raison qui tient à lui : rien n'est créé, rien à envoyer, jamais reproposé), puis l'envoi.
        if reponse["deja_la"] == true {
            let motif = reponse["motif"].as_str().map(String::from);
            noter(dossier, &e.chemin, |x| {
                x.etat = DEJA_LA.into();
                x.motif = motif;
            })?;
            return Ok(Issue::Suivant);
        }
        if reponse["refuse"] == true {
            let motif = reponse["motif"].as_str().map(String::from).or_else(|| Some("Firehouse a déjà refusé ce fichier.".into()));
            return finir_refus_definitif(dossier, &e, motif, rythme);
        }
        let Some(id) = reponse["envoi"].as_u64() else { return Err(Erreur::Serveur("Firehouse n'a pas donné de numéro d'envoi.".into())) };
        let recu = reponse["recu"].as_u64().unwrap_or(0).min(e.taille);
        let etat_post = reponse["etat"].as_str().unwrap_or("en_cours").to_string();
        noter(dossier, &e.chemin, |x| {
            x.envoi = Some(id);
            x.recu = recu;
            x.etat = EN_COURS.into();
            x.motif = None;
        })?;
        e.envoi = Some(id);
        e.recu = recu;
        // Les morceaux (sauf si Firehouse a déjà tout, ou a déjà fini).
        let v = if etat_post == VERIFICATION || (e.taille > 0 && recu >= e.taille) {
            json!({"etat": VERIFICATION})
        } else if matches!(etat_post.as_str(), RANGE | DEJA_LA | REFUSE) {
            return finir(dossier, &e, &etat_post, reponse["motif"].as_str().map(String::from), rythme);
        } else {
            let route = format!("/switch/envois/{id}");
            let envoi = EnvoiParMorceaux { route: &route, source: &chemin, total: e.taille, depart: recu, entetes: vec![], nom: &e.nom };
            let regles = ReglesMorceaux { fini: |v| v["etat"].as_str().is_some_and(|s| s != "en_cours"), zero_sans_recu: false };
            let mut cadence = CadenceSwitch { rythme, dossier, envoi: e.clone(), debit: None, attente: Duration::ZERO, morceau_min: bornes.min, morceau_max: bornes.max, change: false, inaccessible: false };
            match client.envoyer_par_morceaux(&envoi, &regles, &mut cadence).await {
                Ok((_, v)) => v,
                Err(Erreur::Perimee(_)) if cadence.change => {
                    journal("le fichier a changé pendant l'envoi : son empreinte est oubliée");
                    return oublier();
                }
                Err(Erreur::Perimee(_)) if cadence.inaccessible => return source_occupee(dossier, &e, rythme),
                Err(Erreur::Perimee(_)) => return Ok(Issue::Arreter(Arret::Demande)),
                // Lire le fichier de la personne a échoué (verrouillé, raccourci) : il attend, les autres passent.
                Err(Erreur::Disque(_)) => return source_occupee(dossier, &e, rythme),
                // 4.0.1 : un 507 en plein envoi (disque des jeux de Firehouse plein, ou plafond par personne) : tout
                // attend, l'envoi reste en file et reprendra là où Firehouse en est.
                Err(Erreur::Plein { .. }) => {
                    noter(dossier, &e.chemin, |x| x.motif = Some(PLUS_DE_PLACE.into()))?;
                    return Ok(Issue::Arreter(Arret::Motif(PLUS_DE_PLACE.into())));
                }
                Err(err @ Erreur::Reseau(_)) => {
                    // La connexion a lâché pendant un morceau : le débit enregistré ne vaut plus rien.
                    let _ = modifier(dossier, |l| l.debit = None);
                    return Err(err);
                }
                Err(err) => return Err(err),
            }
        };
        let etat = v["etat"].as_str().unwrap_or(VERIFICATION).to_string();
        if etat != VERIFICATION {
            return finir(dossier, &e, &etat, v["motif"].as_str().map(String::from), rythme);
        }
        noter(dossier, &e.chemin, |x| {
            x.etat = VERIFICATION.into();
            x.recu = x.taille;
        })?;
        e.etat = VERIFICATION.into();
    }
    // Le suivi pendant la vérification.
    let id = e.envoi.unwrap_or_default();
    for _ in 0..SUIVIS_MAX {
        rythme.progres(&progres(&e, "verification", e.taille));
        rythme.dormir(ATTENTE_SUIVI).await;
        if rythme.arrete() {
            return Ok(Issue::Arreter(Arret::Demande));
        }
        let b = client.brute(reqwest::Method::GET, &format!("/switch/envois/{id}"), None, &[]).await?;
        if b.statut == 404 {
            // « envoi inconnu » : on repart d'un nouveau POST (même empreinte), au prochain passage.
            noter(dossier, &e.chemin, |x| {
                x.envoi = None;
                x.recu = 0;
                x.etat = ATTENTE.into();
            })?;
            return Ok(Issue::Suivant);
        }
        if !(200..300).contains(&b.statut) {
            return Err(crate::firehouse::erreur_du_statut(b.statut, &b.octets));
        }
        let v: Value = serde_json::from_slice(&b.octets).unwrap_or(Value::Null);
        match v["etat"].as_str().unwrap_or(VERIFICATION) {
            VERIFICATION => continue,
            "en_cours" => {
                // Firehouse attend encore des morceaux : repris au prochain passage, là où il en est.
                let recu = v["recu"].as_u64().unwrap_or(0);
                noter(dossier, &e.chemin, |x| {
                    x.etat = EN_COURS.into();
                    x.recu = recu;
                })?;
                return Ok(Issue::Suivant);
            }
            autre => return finir(dossier, &e, autre, v["motif"].as_str().map(String::from), rythme),
        }
    }
    Ok(Issue::Suivant)
}

/// Note la fin d'un envoi (`range`, `deja_la`, `refuse`). Un refus garde l'empreinte (jamais renvoyée), sauf
/// « abandonné (sans nouvelles) » : l'envoi sort de la file, reproposable.
fn finir(dossier: &Path, e: &Envoi, etat: &str, motif: Option<String>, rythme: &impl Rythme) -> Resultat<Issue> {
    match etat {
        REFUSE if motif.as_deref().is_some_and(refus_par_abandon) => {
            rythme.journaliser(&format!("envoi Switch « {} » abandonné par Firehouse (sans nouvelles) : il sort de la file", e.nom));
            modifier(dossier, |l| l.envois.retain(|x| x.chemin != e.chemin))?;
        }
        REFUSE => return finir_refus_definitif(dossier, e, motif, rythme),
        RANGE | DEJA_LA => {
            rythme.journaliser(&format!("envoi Switch « {} » : {}", e.nom, if etat == RANGE { "rangé par Firehouse" } else { "déjà là" }));
            noter(dossier, &e.chemin, |x| {
                x.etat = etat.into();
                x.motif = motif;
                x.recu = x.taille;
            })?;
        }
        _ => {
            noter(dossier, &e.chemin, |x| {
                x.etat = ARRETE.into();
                x.motif = Some("Firehouse a répondu un état inconnu.".into());
            })?;
        }
    }
    Ok(Issue::Suivant)
}

/// Un refus pour de bon : le motif est gardé (montré), l'empreinte notée comme refusée (jamais renvoyée ni reproposée).
fn finir_refus_definitif(dossier: &Path, e: &Envoi, motif: Option<String>, rythme: &impl Rythme) -> Resultat<Issue> {
    rythme.journaliser(&format!("envoi Switch « {} » refusé par Firehouse", e.nom));
    let sha = e.sha256.clone();
    modifier(dossier, |l| {
        if let Some(s) = sha {
            if !l.refuses.contains(&s) {
                l.refuses.push(s);
            }
        }
        if let Some(x) = l.envois.iter_mut().find(|x| x.chemin == e.chemin) {
            x.etat = REFUSE.into();
            x.motif = motif;
        }
    })?;
    Ok(Issue::Suivant)
}

/// Fait avancer toute la file du profil (`dossier`), un fichier à la fois, jusqu'à ce qu'il n'y ait plus rien à faire
/// maintenant. Rend pourquoi il s'est arrêté (`None` : tout est fait, ou il reste des envois pour plus tard).
pub async fn travailler(client: &Client, dossier: &Path, bornes: Bornes, rythme: &impl Rythme) -> Option<Arret> {
    let mut vus: Vec<String> = Vec::new();
    loop {
        if rythme.arrete() {
            return Some(Arret::Demande);
        }
        let l = match lire(dossier) {
            Ok(l) => l,
            Err(e) => return Some(Arret::Motif(e.to_string())),
        };
        let compte = rythme.compte();
        if let Some(m) = l.interdit_pour(compte.as_deref()) {
            return Some(Arret::Motif(m));
        }
        if l.interdit.is_some() {
            // Un autre compte, ou un autre grade : le refus d'avant ne vaut plus.
            if let Err(e) = modifier(dossier, |l| {
                l.interdit = None;
                l.interdit_compte = None;
            }) {
                return Some(Arret::Motif(e.to_string()));
            }
        }
        let Some(e) = l.envois.into_iter().find(|e| !e.fini() && !vus.contains(&e.chemin)) else { return None };
        vus.push(e.chemin.clone());
        if let Issue::Arreter(a) = traiter(client, dossier, e, bornes, rythme).await {
            return Some(a);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::switch_nca::tests::{cle_de_test, nca, partition, xci};
    use httpmock::prelude::*;
    use std::sync::Mutex;

    const BASE: u64 = 0x0100_7EF0_0011_E000;

    /// Un ticket de test : signature RSA-2048 SHA-256 (0x10004), type de clé `type_cle` en données + 0x141.
    fn ticket(type_cle: u8) -> Vec<u8> {
        let debut = 4 + 0x100 + 0x3C;
        let mut t = vec![0u8; debut + 0x180];
        t[..4].copy_from_slice(&0x10004u32.to_le_bytes());
        t[debut + 0x141] = type_cle;
        t
    }

    /// Un dossier Switch : le jeu (.xci), une mise à jour .nsp, deux DLC .nsp ; et ce qui ne doit JAMAIS partir.
    fn dossier(d: &Path) -> PathBuf {
        let cle = cle_de_test();
        let jeu = d.join("Zelda (v0).xci");
        std::fs::write(&jeu, xci(&[("p.nca", nca(&cle, 0, BASE, 0)), ("m.cnmt.nca", nca(&cle, 1, BASE, 0))], 0)).unwrap();
        std::fs::write(d.join("Zelda maj [v786432].nsp"), partition(&[("m.cnmt.nca", nca(&cle, 1, BASE + 0x800, 0)), ("p.nca", nca(&cle, 0, BASE, 0)), ("t.tik", ticket(0))], false)).unwrap();
        std::fs::write(d.join("Zelda vieille [v65536].nsp"), partition(&[("m.cnmt.nca", nca(&cle, 1, BASE + 0x800, 0)), ("p.nca", nca(&cle, 0, BASE, 0))], false)).unwrap();
        std::fs::write(d.join("DLC 1 [v0].nsp"), partition(&[("a.nca", nca(&cle, 5, BASE + 0x1001, 0)), ("a.cnmt.nca", nca(&cle, 1, BASE + 0x1001, 0))], false)).unwrap();
        std::fs::write(d.join("DLC 2.nsp"), partition(&[("b.nca", nca(&cle, 5, BASE + 0x1002, 0)), ("b.cnmt.nca", nca(&cle, 1, BASE + 0x1002, 0))], false)).unwrap();
        // Jamais : les clés, le firmware, des parties, un autre jeu de base, une mise à jour en .xci.
        std::fs::write(d.join("prod.keys"), "header_key = 00").unwrap();
        std::fs::write(d.join("title.keys"), "01=02").unwrap();
        std::fs::write(d.join("Firmware 19.0.1.zip"), b"PK").unwrap();
        std::fs::create_dir_all(d.join("parties").join("0100")).unwrap();
        std::fs::write(d.join("parties").join("0100").join("save.dat"), b"partie").unwrap();
        std::fs::write(d.join("Autre jeu.nsp"), partition(&[("m.cnmt.nca", nca(&cle, 1, BASE, 0)), ("p.nca", nca(&cle, 0, BASE, 0))], false)).unwrap();
        std::fs::write(d.join("Zelda maj [v65537].xci"), xci(&[("m.cnmt.nca", nca(&cle, 1, BASE + 0x800, 0)), ("p.nca", nca(&cle, 0, BASE, 0))], 0)).unwrap();
        jeu
    }

    fn rien_en_file(_: &Path) -> bool {
        false
    }

    fn lus(jeu: &Path) -> ContenusDuJeu {
        crate::switch_nca::contenus_pour(jeu, &cle_de_test()).unwrap().unwrap()
    }

    /// La route 1, d'après le CONTRAT 1.42 écrit (pas encore de relevé réel : à remplacer par l'exemple réel quand il
    /// sera recopié dans EXEMPLES-API-JEUX-V1.md).
    fn route1_d_apres_le_contrat(jeu: Option<i64>, majs: Value, dlc: Value) -> Value {
        json!({"tid_base": format!("{BASE:016X}"), "jeu": jeu, "majs": majs, "dlc": dlc})
    }

    #[test]
    fn seuls_la_maj_la_plus_recente_et_les_dlc_partent_jamais_cles_firmware_parties_jeu_ni_xci() {
        let d = tempfile::tempdir().unwrap();
        let jeu = dossier(d.path());
        let c = lus(&jeu);
        let servi: CeQueFirehouseA = serde_json::from_value(route1_d_apres_le_contrat(Some(110), json!([]), json!([]))).unwrap();
        let t = trier(&c, &cle_de_test(), &servi, &[], &rien_en_file, false);
        let noms: Vec<(&str, &str, u64, u32)> = t.a_envoyer.iter().map(|x| (x.nom.as_str(), x.role, x.tid, x.version)).collect();
        assert_eq!(
            noms,
            [("Zelda maj [v786432].nsp", "maj", BASE + 0x800, 786432), ("DLC 1 [v0].nsp", "dlc", BASE + 0x1001, 0), ("DLC 2.nsp", "dlc", BASE + 0x1002, 0)],
            "ni la vieille mise à jour, ni le jeu, ni le .xci, ni les clés, le firmware ou les parties"
        );
        // Ce que Firehouse a déjà : rien (même mise à jour ou plus récente ; même DLC, quelle que soit sa version).
        let servi: CeQueFirehouseA = serde_json::from_value(route1_d_apres_le_contrat(
            Some(110),
            json!([{"tid": format!("{:016X}", BASE + 0x800), "version": 786432, "taille": 1, "sha256": "x", "format": "nsp", "nom": "a.nsp", "fichier": "f"}]),
            json!([{"tid": format!("{:016X}", BASE + 0x1001), "version": 65536, "taille": 1, "sha256": "x", "format": "nsz", "nom": "b.nsz", "fichier": "f"}]),
        ))
        .unwrap();
        let t = trier(&c, &cle_de_test(), &servi, &[], &rien_en_file, false);
        assert_eq!(t.a_envoyer.iter().map(|x| x.nom.as_str()).collect::<Vec<_>>(), ["DLC 2.nsp"]);
        assert_eq!(t.deja_la, 2);
        // Une mise à jour PLUS ANCIENNE chez Firehouse : la nôtre part. Une version servie en texte se lit aussi.
        let servi: CeQueFirehouseA = serde_json::from_value(route1_d_apres_le_contrat(Some(110), json!([{"tid": "x", "version": "65536"}]), json!([]))).unwrap();
        assert!(trier(&c, &cle_de_test(), &servi, &[], &rien_en_file, false).a_envoyer.iter().any(|x| x.role == "maj"));
        // Ce qui vient de Firehouse (même chemin, même taille) et ce qui est déjà en file : jamais proposé.
        let venu = d.path().join("DLC 2.nsp");
        let taille = std::fs::metadata(&venu).unwrap().len();
        let t = trier(&c, &cle_de_test(), &servi, &[(venu.clone(), taille)], &|p: &Path| p.ends_with("DLC 1 [v0].nsp"), false);
        assert_eq!(t.a_envoyer.iter().map(|x| x.nom.as_str()).collect::<Vec<_>>(), ["Zelda maj [v786432].nsp"]);
        // Un jeu importé que Firehouse ne connaît pas : rien.
        let servi: CeQueFirehouseA = serde_json::from_value(route1_d_apres_le_contrat(None, json!([]), json!([]))).unwrap();
        let t = trier(&c, &cle_de_test(), &servi, &[], &rien_en_file, true);
        assert!(t.a_envoyer.is_empty() && t.motif.is_some());
        // Le même jeu venu de Firehouse (fiche connue) : proposé.
        assert_eq!(trier(&c, &cle_de_test(), &servi, &[], &rien_en_file, false).a_envoyer.len(), 3);
        // Une mise à jour en .xci plus récente : c'est elle que les émulateurs prennent, mais elle ne part jamais (et
        // la .nsp, devenue ancienne, non plus).
        let cle = cle_de_test();
        std::fs::write(d.path().join("Zelda maj [v1048576].xci"), xci(&[("m.cnmt.nca", nca(&cle, 1, BASE + 0x800, 0)), ("p.nca", nca(&cle, 0, BASE, 0))], 0)).unwrap();
        let t = trier(&lus(&jeu), &cle_de_test(), &servi, &[], &rien_en_file, false);
        assert!(t.a_envoyer.iter().all(|x| x.role == "dlc"), "{t:?}");
        assert!(t.ecartes.iter().any(|e| e.nom == "Zelda maj [v1048576].xci"));
    }

    #[test]
    fn un_ticket_personnel_illisible_plusieurs_metas_ou_sans_version_ne_partent_pas() {
        let cle = cle_de_test();
        let d = tempfile::tempdir().unwrap();
        let jeu = d.path().join("j.xci");
        std::fs::write(&jeu, xci(&[("m.cnmt.nca", nca(&cle, 1, BASE, 0))], 0)).unwrap();
        let dlc = |n: u64, extra: Vec<(&str, Vec<u8>)>| {
            let mut f = vec![("a.nca", nca(&cle, 5, BASE + 0x1000 + n, 0)), ("a.cnmt.nca", nca(&cle, 1, BASE + 0x1000 + n, 0))];
            f.extend(extra);
            partition(&f, false)
        };
        std::fs::write(d.path().join("commun.nsp"), dlc(1, vec![("t.tik", ticket(0))])).unwrap();
        std::fs::write(d.path().join("perso.nsp"), dlc(2, vec![("t.tik", ticket(1))])).unwrap();
        std::fs::write(d.path().join("illisible.nsp"), dlc(3, vec![("t.tik", vec![9, 9])])).unwrap();
        std::fs::write(d.path().join("deux.nsp"), dlc(4, vec![("b.cnmt.nca", nca(&cle, 1, BASE + 0x1005, 0))])).unwrap();
        // Une mise à jour sans version dans le nom.
        std::fs::write(d.path().join("maj.nsp"), partition(&[("m.cnmt.nca", nca(&cle, 1, BASE + 0x800, 0))], false)).unwrap();
        let c = lus(&jeu);
        let t = trier(&c, &cle_de_test(), &CeQueFirehouseA { jeu: Some(1), ..Default::default() }, &[], &rien_en_file, false);
        assert_eq!(t.a_envoyer.iter().map(|x| x.nom.as_str()).collect::<Vec<_>>(), ["commun.nsp"]);
        let raison = |n: &str| t.ecartes.iter().find(|e| e.nom == n).map(|e| e.raison.clone()).unwrap_or_default();
        // Des raisons simples, sans jargon (« pas envoyé : … »).
        assert_eq!(raison("perso.nsp"), RAISON_PERSONNEL, "{t:?}");
        assert_eq!(raison("illisible.nsp"), RAISON_ILLISIBLE);
        assert_eq!(raison("deux.nsp"), RAISON_PLUSIEURS_DLC);
        assert_eq!(raison("maj.nsp"), RAISON_VERSION);
    }

    #[test]
    fn le_type_du_ticket_se_lit_comme_libhac() {
        assert_eq!(type_ticket_de(&ticket(0)), Ticket::Commun);
        assert_eq!(type_ticket_de(&ticket(1)), Ticket::Personnel);
        // RSA-4096 : les données commencent plus loin.
        let mut t = vec![0u8; 0x240 + 0x200];
        t[..4].copy_from_slice(&0x10000u32.to_le_bytes());
        t[0x240 + 0x141] = 1;
        assert_eq!(type_ticket_de(&t), Ticket::Personnel);
        t[0x240 + 0x141] = 0;
        assert_eq!(type_ticket_de(&t), Ticket::Commun);
        // ECDSA (SHA-1 et SHA-256) et RSA-4096 SHA-256 : chacun son décalage (LibHac Ticket.cs).
        for (sig, debut) in [(0x10002u32, 4 + 0x3C + 0x40), (0x10005, 4 + 0x3C + 0x40), (0x10003, 4 + 0x200 + 0x3C)] {
            let mut t = vec![0u8; debut + 0x200];
            t[..4].copy_from_slice(&sig.to_le_bytes());
            assert_eq!(type_ticket_de(&t), Ticket::Commun, "{sig:#x}");
            t[debut + 0x141] = 1;
            assert_eq!(type_ticket_de(&t), Ticket::Personnel, "{sig:#x}");
            assert_eq!(type_ticket_de(&t[..debut + 0x141]), Ticket::Illisible, "{sig:#x} tronqué");
        }
        // Signature inconnue, ou trop court : illisible.
        assert_eq!(type_ticket_de(&[1, 2, 3, 4, 5]), Ticket::Illisible);
        assert_eq!(type_ticket_de(&ticket(0)[..0x100]), Ticket::Illisible);
    }

    fn type_ticket_de(o: &[u8]) -> Ticket {
        crate::switch_nca::type_ticket(o)
    }

    #[test]
    fn la_duree_est_estimee_au_debit_mesure_ou_inconnue() {
        assert_eq!(duree_estimee(1000, Some(100), false), Some(10));
        assert_eq!(duree_estimee(1000, Some(100), true), Some(20));
        assert_eq!(duree_estimee(1000, None, true), None);
        assert_eq!(duree_estimee(1000, Some(0), true), None);
    }

    #[test]
    fn le_debit_est_range_en_entier_et_se_relit_a_l_identique() {
        let d = tempfile::tempdir().unwrap();
        modifier(d.path(), |l| l.debit = Some(1_234_567)).unwrap();
        assert_eq!(lire(d.path()).unwrap().debit, Some(1_234_567));
        // Écrit deux fois de suite : toujours relu pareil (aucune erreur « ne se relit pas »).
        modifier(d.path(), |l| l.debit = Some(3)).unwrap();
        assert_eq!(lire(d.path()).unwrap().debit, Some(3));
    }

    #[test]
    fn l_accord_porte_sur_la_liste_montree() {
        let c = |nom: &str, taille: u64| Candidat { chemin: PathBuf::from(nom), nom: nom.into(), role: "dlc", tid: 1, version: 0, taille, secondes: 0, nanos: 0 };
        let l = [c("a.nsp", 10), c("b.nsz", 5)];
        assert!(meme_liste(&l, &["b.nsz".into(), "a.nsp".into()], 15));
        assert!(!meme_liste(&l, &["a.nsp".into()], 10), "un contenu en plus depuis");
        assert!(!meme_liste(&l, &["a.nsp".into(), "b.nsz".into()], 16), "un poids différent");
    }

    #[test]
    fn l_etat_du_travail_est_propre_a_un_profil() {
        let d = tempfile::tempdir().unwrap();
        let (profil, e) = preparer(d.path());
        let t = Travail::default();
        assert!(t.commencer("A"));
        *t.progres.lock().unwrap() = Some(Progres { profil: "A".into(), jeu: 110, nom: e.nom.clone(), etape: "envoi".into(), ..Default::default() });
        // Un autre profil ouvert pendant l'envoi : il ne voit ni l'envoi en cours ni l'arrêt du premier…
        assert!(suivi_du_jeu(&profil, "B", 110, &t).unwrap().en_cours.is_none());
        assert!(suivi_du_jeu(&profil, "A", 110, &t).unwrap().en_cours.is_some());
        assert!(t.en_cours_pour("A", 110) && !t.en_cours_pour("B", 110) && !t.en_cours_pour("A", 7));
        // … et son travail attend la fin de celui-ci, puis est relancé.
        assert!(!t.commencer("B"));
        assert!(t.finir("A", Some(Arret::Motif("Firehouse injoignable".into()))));
        assert!(t.commencer("B"));
        assert!(!t.finir("B", None));
        // L'arrêt n'est dit qu'au profil concerné, et seulement si ce jeu a encore un envoi à faire.
        *t.arret.lock().unwrap() = Some(("A".into(), "Firehouse injoignable".into()));
        assert_eq!(suivi_du_jeu(&profil, "A", 110, &t).unwrap().arret.as_deref(), Some("Firehouse injoignable"));
        assert!(suivi_du_jeu(&profil, "B", 110, &t).unwrap().arret.is_none());
        assert!(suivi_du_jeu(&profil, "A", 7, &t).unwrap().arret.is_none(), "pas sur un autre jeu");
        modifier(&profil, |l| l.envois[0].etat = RANGE.into()).unwrap();
        assert!(suivi_du_jeu(&profil, "A", 110, &t).unwrap().arret.is_none(), "tout est fini");
    }

    #[tokio::test]
    async fn arreter_l_envoi_retire_le_jeu_de_la_file_et_arrete_l_envoi_en_cours() {
        let s = MockServer::start();
        let d = tempfile::tempdir().unwrap();
        let (profil, e) = preparer(d.path());
        route1_vide(&s, Some(110));
        post_nouveau(&s, 0);
        let p1 = put(&s, "bytes 0-7/20", 200, json!({"ok": true, "envoi": 1, "recu": 8, "taille": 20, "etat": "en_cours"}));
        let p2 = put(&s, "bytes 8-15/20", 200, json!({}));
        // La personne appuie sur « ✋ Arrêter l'envoi » après le premier morceau.
        struct Annule<'a>(&'a Path, &'a httpmock::Mock<'a>, std::sync::atomic::AtomicBool);
        impl Rythme for Annule<'_> {
            fn occupe(&self) -> bool {
                if self.1.hits() == 1 && !self.2.swap(true, std::sync::atomic::Ordering::SeqCst) {
                    assert_eq!(annuler(self.0, 110).unwrap(), 1);
                    return true;
                }
                false
            }
            fn arrete(&self) -> bool {
                self.2.load(std::sync::atomic::Ordering::SeqCst)
            }
            fn plafonne(&self) -> bool {
                false
            }
            async fn dormir(&self, _d: Duration) {}
            fn progres(&self, _p: &Progres) {}
        }
        let r = Annule(&profil, &p1, Default::default());
        assert_eq!(travailler(&client(&s), &profil, PETITS, &r).await, Some(Arret::Demande));
        p2.assert_hits(0);
        let l = lire(&profil).unwrap();
        assert!(l.envois.is_empty());
        // Reproposable : il n'est plus « dans la file ».
        assert!(!l.contient(Path::new(&e.chemin)));
        // Un envoi FINI d'un jeu n'est jamais retiré par « Arrêter ».
        std::fs::create_dir_all(d.path().join("autre")).unwrap();
        let (p2, _) = preparer(&d.path().join("autre"));
        modifier(&p2, |l| l.envois[0].etat = RANGE.into()).unwrap();
        assert_eq!(annuler(&p2, 110).unwrap(), 0);
    }

    #[test]
    fn la_version_part_en_entier() {
        assert_eq!(version_pour_api(131072), json!(131072));
    }

    // --- Le travail, contre un Firehouse simulé (httpmock, corps RÉELS du besoin 52) ---

    /// Un rythme d'essai : jamais d'attente réelle ; les attentes demandées sont notées.
    #[derive(Default)]
    struct Essai {
        dormi: Mutex<Vec<Duration>>,
        /// Combien de fois encore « occupé » (une partie en cours).
        occupe: Mutex<u32>,
        sans_limite: bool,
        etapes: Mutex<Vec<String>>,
        arret: std::sync::atomic::AtomicBool,
        /// Le compte du profil (`username#grade`).
        compte: Option<String>,
        /// La durée « mesurée » de chaque morceau (sinon la vraie).
        mesure: Option<Duration>,
    }

    impl Rythme for Essai {
        fn compte(&self) -> Option<String> {
            self.compte.clone()
        }
        fn mesurer(&self, reelle: Duration) -> Duration {
            self.mesure.unwrap_or(reelle)
        }
        fn occupe(&self) -> bool {
            let mut o = self.occupe.lock().unwrap();
            if *o > 0 {
                *o -= 1;
                return true;
            }
            false
        }
        fn arrete(&self) -> bool {
            self.arret.load(std::sync::atomic::Ordering::SeqCst)
        }
        fn plafonne(&self) -> bool {
            !self.sans_limite
        }
        async fn dormir(&self, d: Duration) {
            self.dormi.lock().unwrap().push(d);
        }
        fn progres(&self, p: &Progres) {
            let mut e = self.etapes.lock().unwrap();
            if e.last() != Some(&p.etape) {
                e.push(p.etape.clone());
            }
        }
    }

    const TID_MAJ: &str = "01007EF00011E800";
    const PETITS: Bornes = Bornes { min: 8, max: 8 };

    /// Un profil d'essai avec UN envoi accepté (fichier de 20 octets, comme le relevé réel de Firehouse).
    fn preparer(d: &Path) -> (PathBuf, Envoi) {
        let f = d.join("Zelda maj [v131072].nsp");
        std::fs::write(&f, b"01234567890123456789").unwrap();
        let (taille, secondes, nanos) = taille_et_date(&f).unwrap();
        let c = Candidat { chemin: f.clone(), nom: "Zelda maj [v131072].nsp".into(), role: "maj", tid: BASE + 0x800, version: 131072, taille, secondes, nanos };
        let profil = d.join("profil");
        assert_eq!(ajouter(&profil, 110, BASE, Some(110), std::slice::from_ref(&c)).unwrap(), 1);
        let e = lire(&profil).unwrap().envois[0].clone();
        (profil, e)
    }

    fn sha_de(f: &Path) -> String {
        crate::sauvegarde::empreinte(f).unwrap()
    }

    fn client(s: &MockServer) -> Client {
        Client::nouveau(&s.base_url(), "j").unwrap()
    }

    /// La route 1 : Firehouse n'a rien pour ce jeu (forme D'APRÈS LE CONTRAT, à remplacer par le relevé réel).
    fn route1_vide(s: &MockServer, jeu: Option<i64>) -> httpmock::Mock<'_> {
        s.mock(|w, t| {
            w.method(GET).path(format!("/api/jeux/v1/switch/{BASE:016X}/contenus"));
            t.status(200).json_body(route1_d_apres_le_contrat(jeu, json!([]), json!([])));
        })
    }

    /// Le POST d'un nouvel envoi (corps RÉEL du besoin 52).
    fn post_nouveau(s: &MockServer, recu: u64) -> httpmock::Mock<'_> {
        s.mock(|w, t| {
            w.method(POST).path("/api/jeux/v1/switch/envois");
            t.status(200).json_body(json!({"ok": true, "deja_la": false, "envoi": 1, "recu": recu, "taille": 20, "etat": "en_cours"}));
        })
    }

    fn put<'a>(s: &'a MockServer, plage: &str, statut: u16, corps: Value) -> httpmock::Mock<'a> {
        let plage = plage.to_string();
        s.mock(move |w, t| {
            w.method(PUT).path("/api/jeux/v1/switch/envois/1").header("content-range", plage.as_str());
            t.status(statut).json_body(corps);
        })
    }

    fn etat_get<'a>(s: &'a MockServer, etat: &str, motif: Option<&str>) -> httpmock::Mock<'a> {
        let (etat, motif) = (etat.to_string(), motif.map(String::from));
        s.mock(move |w, t| {
            w.method(GET).path("/api/jeux/v1/switch/envois/1");
            t.status(200).json_body(json!({"ok": true, "envoi": 1, "etat": etat, "motif": motif, "taille": 20, "recu": 20}));
        })
    }

    /// Les trois morceaux d'un envoi complet de 20 octets en morceaux de 8 (corps RÉELS).
    fn trois_morceaux(s: &MockServer) -> [httpmock::Mock<'_>; 3] {
        [
            put(s, "bytes 0-7/20", 200, json!({"ok": true, "envoi": 1, "recu": 8, "taille": 20, "etat": "en_cours"})),
            put(s, "bytes 8-15/20", 200, json!({"ok": true, "envoi": 1, "recu": 16, "taille": 20, "etat": "en_cours"})),
            put(s, "bytes 16-19/20", 200, json!({"ok": true, "envoi": 1, "recu": 20, "taille": 20, "etat": "verification"})),
        ]
    }

    #[tokio::test]
    async fn un_envoi_complet_puis_range_avec_le_corps_exact_du_post() {
        let s = MockServer::start();
        let d = tempfile::tempdir().unwrap();
        let (profil, e) = preparer(d.path());
        let sha = sha_de(Path::new(&e.chemin));
        let r1 = route1_vide(&s, Some(110));
        // Le corps EXACT : aucun chemin, aucun profil ni PC ; pas de media_id (Firehouse connaît le jeu).
        let post = s.mock(|w, t| {
            w.method(POST).path("/api/jeux/v1/switch/envois").json_body(json!({
                "tid": TID_MAJ, "role": "maj", "version": 131072, "nom": "Zelda maj [v131072].nsp", "taille": 20, "sha256": sha
            }));
            t.status(200).json_body(json!({"ok": true, "deja_la": false, "envoi": 1, "recu": 0, "taille": 20, "etat": "en_cours"}));
        });
        // Aucun en-tête d'empreinte sur les PUT (besoin 52, réponse 4).
        let sans_empreinte = s.mock(|w, t| {
            w.method(PUT).path("/api/jeux/v1/switch/envois/1").header_exists("x-contenu-sha256");
            t.status(500);
        });
        let m = trois_morceaux(&s);
        let verif = etat_get(&s, "range", None);
        let essai = Essai::default();
        let fin = travailler(&client(&s), &profil, PETITS, &essai).await;
        assert_eq!(fin, None);
        r1.assert();
        post.assert();
        sans_empreinte.assert_hits(0);
        for x in &m {
            x.assert();
        }
        verif.assert();
        let l = lire(&profil).unwrap();
        assert_eq!((l.envois[0].etat.as_str(), l.envois[0].recu), (RANGE, 20));
        assert!(l.debit.is_some(), "le débit montant est mesuré");
        let texte = std::fs::read_to_string(chemin_file(&profil)).unwrap();
        assert!(texte.contains(&sha), "l'empreinte est gardée dans la file");
        assert!(essai.etapes.lock().unwrap().contains(&"verification".to_string()));
        // Un fichier fini n'est pas repris.
        assert_eq!(travailler(&client(&s), &profil, PETITS, &essai).await, None);
        post.assert_hits(1);
    }

    #[tokio::test]
    async fn deja_la_au_post_aucun_put_et_le_motif_est_garde() {
        let s = MockServer::start();
        let d = tempfile::tempdir().unwrap();
        let (profil, _) = preparer(d.path());
        route1_vide(&s, Some(110));
        s.mock(|w, t| {
            w.method(POST).path("/api/jeux/v1/switch/envois");
            t.status(200).json_body(json!({"ok": true, "deja_la": true, "motif": "Firehouse sert déjà une mise à jour au moins aussi récente (v131072)"}));
        });
        let tout_put = s.mock(|w, t| {
            w.method(PUT);
            t.status(200);
        });
        assert_eq!(travailler(&client(&s), &profil, PETITS, &Essai::default()).await, None);
        tout_put.assert_hits(0);
        let e = &lire(&profil).unwrap().envois[0];
        assert_eq!(e.etat, DEJA_LA);
        assert!(e.motif.as_deref().unwrap().contains("au moins aussi récente"));
    }

    #[tokio::test]
    async fn la_route_1_avant_l_envoi_suffit_si_firehouse_l_a_deja() {
        let s = MockServer::start();
        let d = tempfile::tempdir().unwrap();
        let (profil, _) = preparer(d.path());
        s.mock(|w, t| {
            w.method(GET).path(format!("/api/jeux/v1/switch/{BASE:016X}/contenus"));
            t.status(200).json_body(route1_d_apres_le_contrat(Some(110), json!([{"tid": TID_MAJ, "version": 131072}]), json!([])));
        });
        let post = post_nouveau(&s, 0);
        assert_eq!(travailler(&client(&s), &profil, PETITS, &Essai::default()).await, None);
        post.assert_hits(0);
        assert_eq!(lire(&profil).unwrap().envois[0].etat, DEJA_LA);
    }

    #[tokio::test]
    async fn la_reprise_part_de_recu_donne_par_le_post_et_suit_un_409() {
        let s = MockServer::start();
        let d = tempfile::tempdir().unwrap();
        let (profil, _) = preparer(d.path());
        route1_vide(&s, Some(110));
        // Firehouse a déjà 8 octets.
        post_nouveau(&s, 8);
        let debut = put(&s, "bytes 0-7/20", 200, json!({}));
        // 8-15 : Firehouse en est en fait à 16 (409 réel : `{detail, recu}`).
        let decale = put(&s, "bytes 8-15/20", 409, json!({"detail": "reprendre à l'octet 16", "recu": 16}));
        let fin = put(&s, "bytes 16-19/20", 200, json!({"ok": true, "envoi": 1, "recu": 20, "taille": 20, "etat": "verification"}));
        etat_get(&s, "range", None);
        assert_eq!(travailler(&client(&s), &profil, PETITS, &Essai::default()).await, None);
        debut.assert_hits(0);
        decale.assert();
        fin.assert();
        assert_eq!(lire(&profil).unwrap().envois[0].etat, RANGE);
    }

    #[tokio::test]
    async fn un_409_sans_recu_est_dit_sans_boucle() {
        let s = MockServer::start();
        let d = tempfile::tempdir().unwrap();
        let (profil, _) = preparer(d.path());
        route1_vide(&s, Some(110));
        post_nouveau(&s, 0);
        let p = put(&s, "bytes 0-7/20", 409, json!({"detail": "conflit"}));
        assert_eq!(travailler(&client(&s), &profil, PETITS, &Essai::default()).await, None);
        p.assert_hits(1);
        let e = &lire(&profil).unwrap().envois[0];
        assert_eq!((e.etat.as_str(), e.motif.as_deref()), (ARRETE, Some("conflit")));
    }

    #[tokio::test]
    async fn un_507_montre_le_plafond_tel_quel_le_plafond_par_personne_attend_le_trop_gros_s_arrete() {
        // Le plafond des envois EN COURS de la personne : passager, l'envoi attend le prochain passage (corps réel).
        let s = MockServer::start();
        let d = tempfile::tempdir().unwrap();
        let (profil, e) = preparer(d.path());
        route1_vide(&s, Some(110));
        let post = s.mock(|w, t| {
            w.method(POST).path("/api/jeux/v1/switch/envois");
            t.status(507).json_body(json!({"detail": "plafond de 0 Go d'envois en cours atteint pour cette personne"}));
        });
        let fin = travailler(&client(&s), &profil, PETITS, &Essai::default()).await;
        assert_eq!(fin, Some(Arret::Motif("plafond de 0 Go d'envois en cours atteint pour cette personne".into())));
        let x = &lire(&profil).unwrap().envois[0];
        assert_eq!(x.etat, ATTENTE);
        assert_eq!(x.motif.as_deref(), Some("plafond de 0 Go d'envois en cours atteint pour cette personne"));
        assert!(lire(&profil).unwrap().contient(Path::new(&e.chemin)));
        travailler(&client(&s), &profil, PETITS, &Essai::default()).await;
        post.assert_hits(2);
        // « Fichier trop gros » : arrêté, le motif tel quel (jamais celui de la sauvegarde) ; reproposable plus tard.
        let s = MockServer::start();
        route1_vide(&s, Some(110));
        let post = s.mock(|w, t| {
            w.method(POST).path("/api/jeux/v1/switch/envois");
            t.status(507).json_body(json!({"detail": "fichier trop gros : 0 Go au plus"}));
        });
        assert_eq!(travailler(&client(&s), &profil, PETITS, &Essai::default()).await, None);
        let l = lire(&profil).unwrap();
        assert_eq!((l.envois[0].etat.as_str(), l.envois[0].motif.as_deref()), (ARRETE, Some("fichier trop gros : 0 Go au plus")));
        assert!(!l.contient(Path::new(&e.chemin)), "un envoi arrêté peut être reproposé");
        travailler(&client(&s), &profil, PETITS, &Essai::default()).await;
        post.assert_hits(1);
        // Un nouvel accord le remet en file.
        let (taille, secondes, nanos) = taille_et_date(Path::new(&e.chemin)).unwrap();
        let c = Candidat { chemin: PathBuf::from(&e.chemin), nom: e.nom.clone(), role: "maj", tid: BASE + 0x800, version: 131072, taille, secondes, nanos };
        assert_eq!(ajouter(&profil, 110, BASE, Some(110), &[c]).unwrap(), 1);
        let l = lire(&profil).unwrap();
        assert_eq!((l.envois.len(), l.envois[0].etat.as_str(), l.envois[0].motif.clone()), (1, ATTENTE, None));
    }

    #[tokio::test]
    async fn le_disque_plein_ou_absent_de_firehouse_met_l_envoi_en_pause_jamais_en_echec() {
        // 4.0.1 : 507 « disque des jeux plein » à l'annonce : passager, l'envoi reste en file avec le motif.
        let s = MockServer::start();
        let d = tempfile::tempdir().unwrap();
        let (profil, e) = preparer(d.path());
        route1_vide(&s, Some(110));
        s.mock(|w, t| {
            w.method(POST).path("/api/jeux/v1/switch/envois");
            t.status(507).json_body(json!({"detail": "le disque des jeux de Firehouse est plein"}));
        });
        let fin = travailler(&client(&s), &profil, PETITS, &Essai::default()).await;
        assert_eq!(fin, Some(Arret::Motif("le disque des jeux de Firehouse est plein".into())));
        let l = lire(&profil).unwrap();
        assert_eq!(l.envois[0].etat, ATTENTE);
        assert!(l.contient(Path::new(&e.chemin)));
        // 507 puis 503 sur un morceau : jamais arrêté pour de bon, l'envoi reste en file.
        for (code, corps) in [(507, json!({"detail": "le disque des jeux de Firehouse est plein"})), (503, json!({"detail": "stockage non monté"}))] {
            let s = MockServer::start();
            route1_vide(&s, Some(110));
            post_nouveau(&s, 0);
            put(&s, "bytes 0-7/20", code, corps);
            let fin = travailler(&client(&s), &profil, PETITS, &Essai::default()).await;
            assert!(matches!(&fin, Some(Arret::Motif(m)) if m.contains("plus tard")), "{code} : {fin:?}");
            let l = lire(&profil).unwrap();
            assert_ne!(l.envois[0].etat, ARRETE, "{code}");
            assert!(l.contient(Path::new(&e.chemin)), "{code}");
        }
    }

    #[tokio::test]
    async fn un_401_arrete_tout_en_un_seul_appel() {
        let s = MockServer::start();
        let d = tempfile::tempdir().unwrap();
        let (profil, _) = preparer(d.path());
        let r1 = s.mock(|w, t| {
            w.method(GET);
            t.status(401).json_body(json!({"detail": "jeton"}));
        });
        let fin = travailler(&client(&s), &profil, PETITS, &Essai::default()).await;
        assert!(matches!(fin, Some(Arret::Motif(m)) if m.contains("jeton")));
        r1.assert_hits(1);
        assert_eq!(lire(&profil).unwrap().envois[0].etat, ATTENTE, "resté dans la file");
    }

    #[tokio::test]
    async fn un_403_dit_que_le_compte_ne_permet_pas_et_cache_la_proposition() {
        let s = MockServer::start();
        let d = tempfile::tempdir().unwrap();
        let (profil, _) = preparer(d.path());
        route1_vide(&s, Some(110));
        let post = s.mock(|w, t| {
            w.method(POST).path("/api/jeux/v1/switch/envois");
            t.status(403).json_body(json!({"detail": "droit manquant"}));
        });
        let mineur = || Essai { compte: Some("lea#mineur".into()), ..Default::default() };
        let fin = travailler(&client(&s), &profil, PETITS, &mineur()).await;
        assert!(matches!(fin, Some(Arret::Motif(m)) if m.contains("ne permet pas")));
        let l = lire(&profil).unwrap();
        assert_eq!((l.interdit.is_some(), l.interdit_compte.as_deref()), (true, Some("lea#mineur")));
        // Les envois non finis sont arrêtés, avec le motif.
        assert_eq!((l.envois[0].etat.as_str(), l.envois[0].motif.as_deref().is_some_and(|m| m.contains("ne permet pas"))), (ARRETE, true));
        assert_eq!(l.interdit_pour(Some("lea#mineur")).is_some(), true);
        travailler(&client(&s), &profil, PETITS, &mineur()).await;
        post.assert_hits(1);
        // Le compte a changé de grade : le refus ne vaut plus, il est effacé.
        assert!(l.interdit_pour(Some("lea#avance")).is_none());
        travailler(&client(&s), &profil, PETITS, &Essai { compte: Some("lea#avance".into()), ..Default::default() }).await;
        let l = lire(&profil).unwrap();
        assert!(l.interdit.is_none() && l.interdit_compte.is_none());
    }

    #[tokio::test]
    async fn un_503_ou_hors_ligne_laisse_l_envoi_dans_la_file() {
        let s = MockServer::start();
        let d = tempfile::tempdir().unwrap();
        let (profil, _) = preparer(d.path());
        let r1 = s.mock(|w, t| {
            w.method(GET);
            t.status(503);
        });
        let fin = travailler(&client(&s), &profil, PETITS, &Essai::default()).await;
        assert!(matches!(fin, Some(Arret::Motif(m)) if m.contains("plus tard")));
        r1.assert_hits(1);
        assert_eq!(lire(&profil).unwrap().envois[0].etat, ATTENTE);
        // Hors ligne : une adresse où rien n'écoute.
        let hors = Client::nouveau("http://127.0.0.1:9", "j").unwrap();
        assert!(matches!(travailler(&hors, &profil, PETITS, &Essai::default()).await, Some(Arret::Motif(_))));
        assert_eq!(lire(&profil).unwrap().envois[0].etat, ATTENTE);
    }

    #[tokio::test]
    async fn un_refus_garde_l_empreinte_et_n_est_jamais_renvoye() {
        let s = MockServer::start();
        let d = tempfile::tempdir().unwrap();
        let (profil, e) = preparer(d.path());
        route1_vide(&s, Some(110));
        let post = post_nouveau(&s, 0);
        trois_morceaux(&s);
        etat_get(&s, "refuse", Some("ticket personnel (lié à une console ou un compte) : jamais servi à la famille"));
        assert_eq!(travailler(&client(&s), &profil, PETITS, &Essai::default()).await, None);
        let l = lire(&profil).unwrap();
        assert_eq!(l.envois[0].etat, REFUSE);
        assert!(l.envois[0].motif.as_deref().unwrap().contains("ticket personnel"));
        assert_eq!(l.refuses, [sha_de(Path::new(&e.chemin))]);
        // Même remis dans la file (nouvel accord), le même fichier ne repart jamais.
        modifier(&profil, |l| l.envois[0].etat = ATTENTE.into()).unwrap();
        travailler(&client(&s), &profil, PETITS, &Essai::default()).await;
        post.assert_hits(1);
        assert_eq!(lire(&profil).unwrap().envois[0].etat, REFUSE);
    }

    #[tokio::test]
    async fn un_post_refuse_contrat_1_43_ne_cree_rien_montre_le_motif_et_n_est_jamais_repropose() {
        let s = MockServer::start();
        let d = tempfile::tempdir().unwrap();
        let (profil, e) = preparer(d.path());
        route1_vide(&s, Some(110));
        // Réponse RÉELLE du contrat 1.43 (base d'essai de Firehouse).
        let post = s.mock(|w, t| {
            w.method(POST).path("/api/jeux/v1/switch/envois");
            t.status(200).json_body(json!({"ok": true, "deja_la": false, "refuse": true, "motif": "ticket personnel (lié à une console ou un compte) : jamais servi à la famille"}));
        });
        let tout_put = s.mock(|w, t| {
            w.method(PUT);
            t.status(200);
        });
        assert_eq!(travailler(&client(&s), &profil, PETITS, &Essai::default()).await, None);
        tout_put.assert_hits(0);
        let l = lire(&profil).unwrap();
        assert_eq!(l.envois[0].etat, REFUSE);
        assert_eq!(l.envois[0].motif.as_deref(), Some("ticket personnel (lié à une console ou un compte) : jamais servi à la famille"));
        assert_eq!(l.refuses, [sha_de(Path::new(&e.chemin))]);
        // Jamais reproposé : le fichier est dans la file (fini), et son empreinte refusée arrête tout nouvel essai.
        assert!(l.contient(Path::new(&e.chemin)));
        let c = lus_un(d.path(), &e);
        assert!(trier(&c, &cle_de_test(), &CeQueFirehouseA { jeu: Some(110), ..Default::default() }, &[], &|p: &Path| l.contient(p), false).a_envoyer.is_empty());
        modifier(&profil, |l| l.envois[0].etat = ATTENTE.into()).unwrap();
        travailler(&client(&s), &profil, PETITS, &Essai::default()).await;
        post.assert_hits(1);
    }

    /// Le même fichier vu comme une mise à jour de Zelda (pour le tri), sans relire de vrai conteneur.
    fn lus_un(_d: &Path, e: &Envoi) -> ContenusDuJeu {
        ContenusDuJeu {
            base: BASE,
            maj: Some(ContenuLu { chemin: PathBuf::from(&e.chemin), sorte: Sorte::MiseAJour, base: BASE, version: Some(e.version), forme: Forme::Nsp }),
            ..Default::default()
        }
    }

    #[cfg(windows)]
    #[test]
    fn une_file_verrouillee_n_est_ni_mise_de_cote_ni_ecrasee() {
        use std::os::windows::fs::OpenOptionsExt;
        let d = tempfile::tempdir().unwrap();
        let (profil, _) = preparer(d.path());
        let avant = std::fs::read(chemin_file(&profil)).unwrap();
        let verrou = std::fs::OpenOptions::new().read(true).share_mode(0).open(chemin_file(&profil)).unwrap();
        assert!(matches!(lire(&profil), Err(Erreur::Disque(m)) if m.contains("liste des envois")));
        assert!(modifier(&profil, |l| l.envois.clear()).is_err());
        drop(verrou);
        assert_eq!(std::fs::read(chemin_file(&profil)).unwrap(), avant, "jamais écrasée");
        assert!(!std::fs::read_dir(&profil).unwrap().flatten().any(|x| x.file_name().to_string_lossy().contains("abime")));
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn un_fichier_utilise_par_un_autre_programme_attend_sans_tout_arreter() {
        use std::os::windows::fs::OpenOptionsExt;
        let s = MockServer::start();
        let d = tempfile::tempdir().unwrap();
        let (profil, e) = preparer(d.path());
        route1_vide(&s, Some(110));
        let post = post_nouveau(&s, 0);
        let verrou = std::fs::OpenOptions::new().read(true).share_mode(0).open(&e.chemin).unwrap();
        assert_eq!(travailler(&client(&s), &profil, PETITS, &Essai::default()).await, None, "on passe au suivant");
        post.assert_hits(0);
        let x = &lire(&profil).unwrap().envois[0];
        assert_eq!((x.etat.as_str(), x.motif.as_deref()), (ATTENTE, Some(SOURCE_OCCUPEE)));
        drop(verrou);
        trois_morceaux(&s);
        etat_get(&s, "range", None);
        assert_eq!(travailler(&client(&s), &profil, PETITS, &Essai::default()).await, None);
        assert_eq!(lire(&profil).unwrap().envois[0].etat, RANGE);
    }

    #[tokio::test]
    async fn un_post_qui_dit_deja_verification_passe_au_suivi_sans_put() {
        let s = MockServer::start();
        let d = tempfile::tempdir().unwrap();
        let (profil, _) = preparer(d.path());
        route1_vide(&s, Some(110));
        s.mock(|w, t| {
            w.method(POST).path("/api/jeux/v1/switch/envois");
            t.status(200).json_body(json!({"ok": true, "deja_la": false, "envoi": 1, "recu": 20, "taille": 20, "etat": "verification"}));
        });
        let tout_put = s.mock(|w, t| {
            w.method(PUT);
            t.status(200);
        });
        etat_get(&s, "range", None);
        assert_eq!(travailler(&client(&s), &profil, PETITS, &Essai::default()).await, None);
        tout_put.assert_hits(0);
        assert_eq!(lire(&profil).unwrap().envois[0].etat, RANGE);
    }

    #[tokio::test]
    async fn un_envoi_qui_ne_se_cale_pas_est_dit_sans_parler_de_connexion() {
        let s = MockServer::start();
        let d = tempfile::tempdir().unwrap();
        let (profil, _) = preparer(d.path());
        route1_vide(&s, Some(110));
        post_nouveau(&s, 0);
        // Firehouse répond toujours « reprendre à 8 » : au 4e conflit, l'envoi s'arrête pour ce fichier seulement.
        s.mock(|w, t| {
            w.method(PUT).path("/api/jeux/v1/switch/envois/1");
            t.status(409).json_body(json!({"detail": "reprendre à l'octet 8", "recu": 8}));
        });
        assert_eq!(travailler(&client(&s), &profil, PETITS, &Essai::default()).await, None);
        let x = &lire(&profil).unwrap().envois[0];
        assert_eq!(x.etat, ARRETE);
        assert!(x.motif.as_deref().unwrap().contains("ne se cale pas"), "{x:?}");
    }

    #[tokio::test]
    async fn un_abandon_sans_nouvelles_n_est_pas_un_refus_definitif() {
        let s = MockServer::start();
        let d = tempfile::tempdir().unwrap();
        let (profil, _) = preparer(d.path());
        route1_vide(&s, Some(110));
        post_nouveau(&s, 0);
        trois_morceaux(&s);
        etat_get(&s, "refuse", Some("abandonné (sans nouvelles)"));
        travailler(&client(&s), &profil, PETITS, &Essai::default()).await;
        let l = lire(&profil).unwrap();
        assert!(l.envois.is_empty() && l.refuses.is_empty(), "sorti de la file, reproposable : {l:?}");
    }

    #[tokio::test]
    async fn la_verification_est_suivie_toutes_les_dix_secondes() {
        let s = MockServer::start();
        let d = tempfile::tempdir().unwrap();
        let (profil, _) = preparer(d.path());
        route1_vide(&s, Some(110));
        post_nouveau(&s, 0);
        trois_morceaux(&s);
        // Le premier regard voit « verification » ; avant le deuxième, Firehouse a rangé le fichier.
        struct Bascule<'a> {
            s: &'a MockServer,
            verif: Mutex<Option<httpmock::Mock<'a>>>,
            range: Mutex<Option<httpmock::Mock<'a>>>,
            suivis: Mutex<u32>,
        }
        impl Rythme for Bascule<'_> {
            fn occupe(&self) -> bool {
                false
            }
            fn arrete(&self) -> bool {
                false
            }
            fn plafonne(&self) -> bool {
                false
            }
            async fn dormir(&self, d: Duration) {
                if d == ATTENTE_SUIVI {
                    let mut n = self.suivis.lock().unwrap();
                    *n += 1;
                    if *n == 2 {
                        if let Some(mut m) = self.verif.lock().unwrap().take() {
                            m.delete();
                        }
                        *self.range.lock().unwrap() = Some(etat_get(self.s, "range", None));
                    }
                }
            }
            fn progres(&self, _p: &Progres) {}
        }
        let b = Bascule { s: &s, verif: Mutex::new(Some(etat_get(&s, "verification", None))), range: Mutex::new(None), suivis: Mutex::new(0) };
        assert_eq!(travailler(&client(&s), &profil, PETITS, &b).await, None);
        assert_eq!(lire(&profil).unwrap().envois[0].etat, RANGE);
        assert_eq!(*b.suivis.lock().unwrap(), 2, "un regard toutes les ~10 s, jusqu'au rangement");
        b.range.lock().unwrap().as_ref().unwrap().assert();
    }

    #[tokio::test]
    async fn un_404_media_id_necessaire_refait_le_post_avec_la_fiche() {
        let s = MockServer::start();
        let d = tempfile::tempdir().unwrap();
        let (profil, e) = preparer(d.path());
        let sha = sha_de(Path::new(&e.chemin));
        // Firehouse donne `jeu` dans la route 1 : pas de media_id au premier POST… mais il le demande quand même.
        route1_vide(&s, Some(110));
        let sans = s.mock(|w, t| {
            w.method(POST).path("/api/jeux/v1/switch/envois").json_body(json!({
                "tid": TID_MAJ, "role": "maj", "version": 131072, "nom": "Zelda maj [v131072].nsp", "taille": 20, "sha256": sha
            }));
            t.status(404).json_body(json!({"detail": "jeu de base inconnu de Firehouse : `media_id` (la fiche du jeu) est nécessaire"}));
        });
        let avec = s.mock(|w, t| {
            w.method(POST).path("/api/jeux/v1/switch/envois").json_body_partial(r#"{"media_id": 110}"#);
            t.status(200).json_body(json!({"ok": true, "deja_la": true, "motif": "ce fichier est déjà servi par Firehouse"}));
        });
        travailler(&client(&s), &profil, PETITS, &Essai::default()).await;
        sans.assert();
        avec.assert();
        assert_eq!(lire(&profil).unwrap().envois[0].etat, DEJA_LA);
        // Un jeu importé (aucune fiche possible) : arrêté, dit, pas de boucle.
        let s = MockServer::start();
        route1_vide(&s, Some(110));
        let p = s.mock(|w, t| {
            w.method(POST).path("/api/jeux/v1/switch/envois");
            t.status(404).json_body(json!({"detail": "jeu de base inconnu de Firehouse : `media_id` (la fiche du jeu) est nécessaire"}));
        });
        modifier(&profil, |l| {
            l.envois[0].etat = ATTENTE.into();
            l.envois[0].media_possible = None;
        })
        .unwrap();
        travailler(&client(&s), &profil, PETITS, &Essai::default()).await;
        p.assert_hits(1);
        assert_eq!(lire(&profil).unwrap().envois[0].etat, ARRETE);
    }

    #[tokio::test]
    async fn le_debit_est_plafonne_par_une_pause_entre_les_morceaux() {
        let s = MockServer::start();
        let d = tempfile::tempdir().unwrap();
        let (profil, _) = preparer(d.path());
        route1_vide(&s, Some(110));
        post_nouveau(&s, 0);
        trois_morceaux(&s);
        etat_get(&s, "range", None);
        // Chaque morceau de 8 octets « prend » 4 s : 2 octets/s ; plafond = 1 octet/s → 8 s par morceau, donc 4 s
        // d'attente après chacun (le dernier n'en a pas besoin).
        let essai = Essai { mesure: Some(Duration::from_secs(4)), ..Default::default() };
        travailler(&client(&s), &profil, PETITS, &essai).await;
        let dormi: Vec<Duration> = essai.dormi.lock().unwrap().iter().copied().filter(|d| *d != ATTENTE_SUIVI).collect();
        assert_eq!(dormi, [Duration::from_secs(4), Duration::from_secs(4)]);
        assert_eq!(lire(&profil).unwrap().debit, Some(2), "le débit mesuré est enregistré, en octets/s entiers");
        // « Sans limite » : aucune pause entre les morceaux.
        let s = MockServer::start();
        let d = tempfile::tempdir().unwrap();
        let (profil, _) = preparer(d.path());
        route1_vide(&s, Some(110));
        post_nouveau(&s, 0);
        trois_morceaux(&s);
        etat_get(&s, "range", None);
        let essai = Essai { sans_limite: true, mesure: Some(Duration::from_secs(4)), ..Default::default() };
        travailler(&client(&s), &profil, PETITS, &essai).await;
        assert!(essai.dormi.lock().unwrap().iter().all(|d| *d == ATTENTE_SUIVI));
    }

    #[test]
    fn un_morceau_ne_depasse_jamais_8_mo_et_reste_court_sur_une_connexion_lente() {
        let essai = Essai::default();
        let d = tempfile::tempdir().unwrap();
        let mut c = CadenceSwitch { rythme: &essai, dossier: d.path(), envoi: preparer(d.path()).1, debit: Some(1e12), attente: Duration::ZERO, morceau_min: MORCEAU_MIN, morceau_max: MORCEAU_MAX, change: false, inaccessible: false };
        assert_eq!(c.taille_morceau(), MORCEAU_MAX);
        assert!(MORCEAU_MAX <= 8_000_000);
        // 50 Ko/s : ~1 Mo par morceau (20 s), bien sous le délai de 90 s.
        c.debit = Some(50_000.0);
        assert_eq!(c.taille_morceau(), 1_000_000);
        c.debit = Some(1.0);
        assert_eq!(c.taille_morceau(), MORCEAU_MIN);
        c.debit = None;
        assert_eq!(c.taille_morceau(), MORCEAU_INCONNU);
    }

    #[tokio::test]
    async fn une_serie_commence_petite_meme_avec_un_gros_debit_enregistre_et_suit_un_serveur_lent() {
        let d = tempfile::tempdir().unwrap();
        let (profil, e) = preparer(d.path());
        // Un débit énorme enregistré (une autre connexion, un autre jour) : il ne sert qu'à l'estimation.
        modifier(&profil, |l| l.debit = Some(1_000_000_000_000)).unwrap();
        let lent = Essai { sans_limite: true, mesure: Some(Duration::from_secs(100)), ..Default::default() };
        let mut c = CadenceSwitch { rythme: &lent, dossier: &profil, envoi: e, debit: None, attente: Duration::ZERO, morceau_min: 1024, morceau_max: MORCEAU_MAX, change: false, inaccessible: false };
        assert_eq!(c.avant(0).await.unwrap(), MORCEAU_INCONNU, "jamais 8 Mo d'entrée");
        // 256 Ko en 100 s (~2,6 Ko/s) : le morceau suivant est plus petit (~52 Ko, 20 s à ce débit).
        c.apres(8, MORCEAU_INCONNU as u64, Duration::from_millis(1));
        let suivant = c.avant(8).await.unwrap();
        assert!(suivant < MORCEAU_INCONNU && suivant >= 1024, "{suivant}");
        assert_eq!(lire(&profil).unwrap().debit, Some(2621));
    }

    #[tokio::test]
    async fn en_pause_pendant_une_partie_puis_reprise() {
        let s = MockServer::start();
        let d = tempfile::tempdir().unwrap();
        let (profil, _) = preparer(d.path());
        route1_vide(&s, Some(110));
        post_nouveau(&s, 0);
        trois_morceaux(&s);
        etat_get(&s, "range", None);
        // Une partie occupe le PC pendant les 3 premiers regards.
        let essai = Essai { occupe: Mutex::new(3), sans_limite: true, ..Default::default() };
        assert_eq!(travailler(&client(&s), &profil, PETITS, &essai).await, None);
        assert_eq!(essai.dormi.lock().unwrap().iter().filter(|d| **d == ATTENTE_PAUSE).count(), 3);
        assert!(essai.etapes.lock().unwrap().contains(&"pause".to_string()));
        assert_eq!(lire(&profil).unwrap().envois[0].etat, RANGE);
    }

    #[tokio::test]
    async fn la_file_survit_a_un_redemarrage_et_reprend_ou_elle_en_etait() {
        let s = MockServer::start();
        let d = tempfile::tempdir().unwrap();
        let (profil, e) = preparer(d.path());
        route1_vide(&s, Some(110));
        // Premier passage : arrêt demandé après le premier morceau (Frogtend fermé).
        post_nouveau(&s, 0);
        let p1 = put(&s, "bytes 0-7/20", 200, json!({"ok": true, "envoi": 1, "recu": 8, "taille": 20, "etat": "en_cours"}));
        let essai = Essai { sans_limite: true, ..Default::default() };
        struct ArretApres<'a>(&'a Essai, &'a httpmock::Mock<'a>);
        impl Rythme for ArretApres<'_> {
            fn occupe(&self) -> bool {
                false
            }
            fn arrete(&self) -> bool {
                self.1.hits() >= 1
            }
            fn plafonne(&self) -> bool {
                false
            }
            async fn dormir(&self, d: Duration) {
                self.0.dormir(d).await
            }
            fn progres(&self, p: &Progres) {
                self.0.progres(p)
            }
        }
        assert_eq!(travailler(&client(&s), &profil, PETITS, &ArretApres(&essai, &p1)).await, Some(Arret::Demande));
        let l = lire(&profil).unwrap();
        assert_eq!((l.envois[0].recu, l.envois[0].envoi, l.envois[0].sha256.clone()), (8, Some(1), Some(sha_de(Path::new(&e.chemin)))));
        // « Redémarrage » : un nouveau Firehouse simulé, la file relue du disque ; le même POST rend `recu` = 8.
        let s = MockServer::start();
        route1_vide(&s, Some(110));
        post_nouveau(&s, 8);
        let jamais = put(&s, "bytes 0-7/20", 200, json!({}));
        let suite = [
            put(&s, "bytes 8-15/20", 200, json!({"ok": true, "envoi": 1, "recu": 16, "taille": 20, "etat": "en_cours"})),
            put(&s, "bytes 16-19/20", 200, json!({"ok": true, "envoi": 1, "recu": 20, "taille": 20, "etat": "verification"})),
        ];
        etat_get(&s, "range", None);
        assert_eq!(travailler(&client(&s), &profil, PETITS, &Essai::default()).await, None);
        jamais.assert_hits(0);
        for x in &suite {
            x.assert();
        }
        assert_eq!(lire(&profil).unwrap().envois[0].etat, RANGE);
    }

    #[tokio::test]
    async fn un_fichier_change_pendant_l_envoi_sort_de_la_file_et_son_empreinte_est_oubliee() {
        let s = MockServer::start();
        let d = tempfile::tempdir().unwrap();
        let (profil, e) = preparer(d.path());
        route1_vide(&s, Some(110));
        post_nouveau(&s, 0);
        let p1 = put(&s, "bytes 0-7/20", 200, json!({"ok": true, "envoi": 1, "recu": 8, "taille": 20, "etat": "en_cours"}));
        let p2 = put(&s, "bytes 8-15/20", 200, json!({}));
        struct Changeur<'a>(&'a Path, &'a httpmock::Mock<'a>);
        impl Rythme for Changeur<'_> {
            fn occupe(&self) -> bool {
                if self.1.hits() == 1 {
                    std::fs::write(self.0, b"un autre contenu, plus long").unwrap();
                }
                false
            }
            fn arrete(&self) -> bool {
                false
            }
            fn plafonne(&self) -> bool {
                false
            }
            async fn dormir(&self, _d: Duration) {}
            fn progres(&self, _p: &Progres) {}
        }
        travailler(&client(&s), &profil, PETITS, &Changeur(Path::new(&e.chemin), &p1)).await;
        p2.assert_hits(0);
        assert!(lire(&profil).unwrap().envois.is_empty());
    }

    #[test]
    fn la_file_s_ecrit_et_se_relit_et_une_file_abimee_est_mise_de_cote() {
        let d = tempfile::tempdir().unwrap();
        let (profil, e) = preparer(d.path());
        assert_eq!(lire(&profil).unwrap().envois, [e.clone()]);
        assert!(!std::fs::read_dir(&profil).unwrap().flatten().any(|x| x.file_name().to_string_lossy().ends_with(".tmp")));
        // Ajouté deux fois : une seule ligne.
        let c = Candidat { chemin: PathBuf::from(&e.chemin), nom: e.nom.clone(), role: "maj", tid: 1, version: 1, taille: e.taille, secondes: e.secondes, nanos: e.nanos };
        assert_eq!(ajouter(&profil, 110, BASE, None, &[c]).unwrap(), 0);
        assert!(lire(&profil).unwrap().contient(Path::new(&e.chemin)));
        std::fs::write(chemin_file(&profil), b"{pas du json").unwrap();
        assert_eq!(lire(&profil).unwrap(), FileEnvois::default());
        assert!(std::fs::read_dir(&profil).unwrap().flatten().any(|x| x.file_name().to_string_lossy().starts_with("envois-switch.abime-")));
    }

    #[test]
    fn un_refus_par_abandon_se_reconnait() {
        assert!(refus_par_abandon("abandonné (sans nouvelles)"));
        assert!(!refus_par_abandon("ticket personnel (lié à une console ou un compte) : jamais servi à la famille"));
    }
}
