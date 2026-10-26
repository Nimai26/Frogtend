//! Nexus Mods (lot 0.82.0 ; ✅ Seb 19-20/10). La voie OFFICIELLE seulement : la personne clique « Mod Manager
//! Download » sur le site, Windows ouvre un lien `nxm://` (enregistré par l'installeur, `windows/crochets.nsh`), et
//! Frogtend demande le lien de téléchargement à l'API de Nexus avec la clé PERSONNELLE du profil (gratuite ; l'annonce
//! officielle de l'API dit que cette méthode « ne demande aucune approbation »). Jamais d'automate sur les pages.
//!
//! Relevé dans le client OFFICIEL (github.com/Nexus-Mods/node-nexus-api, 20/10) : base `https://api.nexusmods.com/v1`,
//! en-tête `APIKEY`, `GET /users/validate`, `GET /games/{jeu}/mods/{mod}/files/{fichier}` (taille en Ko seulement, pas
//! d'empreinte), `GET …/download_link?key=&expires=` (`key`/`expires` du lien nxm pour un compte gratuit),
//! `GET /games/{jeu}/mods/md5_search/{md5}` ; réponses en JSON seulement.
//!
//! Avis des experts (20/10) : la clé ne part QUE vers l'API (client sans aucune redirection, clé posée par requête) ;
//! le téléchargement passe par un autre client, sans clé ; la taille en Ko sert de plafond ; le fichier reçu est
//! vérifié après coup par `md5_search` (il doit être exactement le mod et le fichier demandés).

use crate::erreurs::{Erreur, Resultat};
use serde_json::Value;

pub const API: &str = "https://api.nexusmods.com/v1";

/// Un lien `nxm://<jeu>/mods/<mod>/files/<fichier>?key=…&expires=…` lu strictement. `key` est une clé à usage unique :
/// jamais affichée ni journalisée (Debug ne la montre pas) ; `user_id` est jeté.
#[derive(Clone, PartialEq)]
pub struct LienNxm {
    pub jeu: String,
    pub mod_id: u64,
    pub fichier_id: u64,
    pub key: Option<String>,
    pub expires: Option<u64>,
}

impl std::fmt::Debug for LienNxm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "nxm://{}/mods/{}/files/{} (clé masquée)", self.jeu, self.mod_id, self.fichier_id)
    }
}

/// Un domaine de jeu de Nexus (`skyrimspecialedition`) : minuscules et chiffres.
fn domaine_sur(d: &str) -> bool {
    !d.is_empty() && d.len() <= 64 && d.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

/// Lit un lien `nxm://` ; `None` pour toute autre forme (une collection, un paramètre inconnu ou en double…).
pub fn lire_nxm(s: &str) -> Option<LienNxm> {
    if s.len() > 2048 {
        return None;
    }
    let reste = s.strip_prefix("nxm://")?;
    let (chemin, requete) = reste.split_once('?').unwrap_or((reste, ""));
    let p: Vec<&str> = chemin.split('/').collect();
    let nombre = |x: &str| x.parse::<u64>().ok().filter(|n| *n > 0 && x.chars().all(|c| c.is_ascii_digit()));
    if p.len() != 5 || p[1] != "mods" || p[3] != "files" || !domaine_sur(p[0]) {
        return None;
    }
    let (mod_id, fichier_id) = (nombre(p[2])?, nombre(p[4])?);
    let (mut key, mut expires, mut user_id) = (None, None, false);
    for paire in requete.split('&').filter(|x| !x.is_empty()) {
        let (k, v) = paire.split_once('=')?;
        match k {
            "key" if key.is_none() && !v.is_empty() && v.len() <= 128 && v.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') => {
                key = Some(v.to_string())
            }
            "expires" if expires.is_none() => expires = Some(nombre(v)?),
            "user_id" if !user_id => user_id = true,
            _ => return None,
        }
    }
    // Les deux ensemble, ou aucun (un compte Premium n'en a pas besoin).
    if key.is_some() != expires.is_some() {
        return None;
    }
    Some(LienNxm { jeu: p[0].to_string(), mod_id, fichier_id, key, expires })
}

/// Le lien a-t-il expiré (`maintenant` en secondes depuis 1970) ?
pub fn expire(l: &LienNxm, maintenant: u64) -> bool {
    l.expires.is_some_and(|e| e <= maintenant)
}

/// Le domaine et le numéro du mod d'une page Nexus (`https://www.nexusmods.com/<jeu>/mods/<n>…`).
pub fn page_nexus(url: &str) -> Option<(String, u64)> {
    let u = reqwest::Url::parse(url).ok()?;
    if u.scheme() != "https" || !matches!(u.host_str(), Some("www.nexusmods.com" | "nexusmods.com")) {
        return None;
    }
    let p: Vec<&str> = u.path().trim_matches('/').split('/').collect();
    (p.len() >= 3 && p[1] == "mods" && domaine_sur(p[0])).then(|| p[2].parse::<u64>().ok().map(|n| (p[0].to_string(), n))).flatten()
}

/// Le client de l'API : AUCUNE redirection (la clé, en-tête maison, partirait ailleurs), https seulement.
fn client_api() -> Resultat<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .https_only(!cfg!(test))
        .build()
        .map_err(|_| Erreur::Reseau("Impossible de préparer la connexion.".into()))
}

/// Une requête à l'API de Nexus avec la clé du profil ; une réponse qui n'est pas du JSON est refusée. Les erreurs ne
/// contiennent jamais la clé ni l'adresse.
async fn obtenir(base: &str, route: &str, cle: &str) -> Resultat<Value> {
    let rep = client_api()?
        .get(format!("{base}{route}"))
        .header("APIKEY", cle)
        .header("Application-Name", "Frogtend")
        .header("Application-Version", env!("CARGO_PKG_VERSION"))
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|e| Erreur::Reseau(if e.is_timeout() { "Nexus ne répond pas assez vite : réessaie.".into() } else { "Impossible de joindre Nexus (connexion Internet ?).".into() }))?;
    let statut = rep.status().as_u16();
    let json = rep.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).is_some_and(|t| t.starts_with("application/json"));
    match statut {
        200 if json => rep.json().await.map_err(|_| Erreur::Serveur("Nexus a envoyé une réponse illisible.".into())),
        200 => Err(Erreur::Serveur("Nexus a envoyé une réponse illisible.".into())),
        // Jamais `JetonRefuse` : c'est le jeton de FIREHOUSE. Un refus ailleurs que sur la clé peut venir d'un lien
        // créé par un autre compte Nexus (navigateur partagé) ou réservé aux comptes Premium.
        401 | 403 if route == "/users/validate" => Err(Erreur::Refus("Nexus refuse cette clé : recopie-la en entier depuis ton compte Nexus.".into())),
        401 => Err(Erreur::Refus("Nexus refuse ta clé : recolle-la dans ⚙ Options ▸ Comptes ▸ Nexus Mods.".into())),
        403 => Err(Erreur::Refus("Nexus refuse ce lien avec ton compte (le lien vient-il du même compte Nexus que ta clé ?).".into())),
        404 => Err(Erreur::Introuvable("Nexus ne connaît pas ce fichier (retiré par son auteur ?).".into())),
        410 => Err(Erreur::Refus("Ce lien de Nexus a expiré : reclique « Mod Manager Download » sur la page du mod.".into())),
        429 => Err(Erreur::Refus("Quota de Nexus atteint pour cette heure : réessaie plus tard.".into())),
        300..=399 => Err(Erreur::Serveur("Nexus a répondu par une redirection inattendue : rien n'est envoyé.".into())),
        _ => Err(Erreur::Serveur(format!("Nexus a répondu {statut}."))),
    }
}

/// Vérifie une clé : rend le nom du compte (rien d'autre de la réponse n'est gardé : ni la clé renvoyée, ni l'adresse
/// électronique, ni le numéro de compte).
pub async fn valider(base: &str, cle: &str) -> Resultat<String> {
    if cle.trim().is_empty() || cle.len() > 512 || cle.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(Erreur::Refus("Ce n'est pas une clé d'API de Nexus (copie-la en entier, sans espace).".into()));
    }
    let v = obtenir(base, "/users/validate", cle).await?;
    v["name"].as_str().map(str::to_string).filter(|n| !n.is_empty()).ok_or_else(|| Erreur::Serveur("Nexus n'a pas confirmé ta clé.".into()))
}

/// Ce que Nexus dit d'un fichier : son nom et sa taille arrondie (en Ko).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct FichierNexus {
    pub nom: String,
    pub taille_ko: u64,
    pub nom_mod: String,
}

pub async fn fichier(base: &str, l: &LienNxm, cle: &str) -> Resultat<FichierNexus> {
    let v = obtenir(base, &format!("/games/{}/mods/{}/files/{}", l.jeu, l.mod_id, l.fichier_id), cle).await?;
    let nom = v["file_name"].as_str().filter(|n| !n.is_empty() && n.len() <= 255 && !n.contains(['/', '\\', ':'])).ok_or_else(|| Erreur::Serveur("Nexus n'a pas donné le nom du fichier.".into()))?;
    let taille_ko = v["size_kb"].as_u64().or_else(|| v["size"].as_u64()).filter(|t| *t > 0).ok_or_else(|| Erreur::Serveur("Nexus n'a pas donné la taille du fichier.".into()))?;
    Ok(FichierNexus { nom: nom.to_string(), taille_ko, nom_mod: v["name"].as_str().unwrap_or(nom).to_string() })
}

/// Le plafond d'un fichier dont Nexus ne donne que la taille arrondie en Ko.
pub fn plafond(taille_ko: u64) -> u64 {
    taille_ko.saturating_add(1).saturating_mul(1024)
}

/// Les adresses de téléchargement (https seulement) d'un fichier, pour ce lien (clé à usage unique d'un compte gratuit).
pub async fn adresses(base: &str, l: &LienNxm, cle: &str) -> Resultat<Vec<String>> {
    let mut route = format!("/games/{}/mods/{}/files/{}/download_link", l.jeu, l.mod_id, l.fichier_id);
    if let (Some(k), Some(e)) = (&l.key, l.expires) {
        route.push_str(&format!("?key={k}&expires={e}"));
    }
    let v = obtenir(base, &route, cle).await?;
    let l: Vec<String> = v.as_array().map(|a| a.iter().filter_map(|x| x["URI"].as_str()).filter(|u| u.starts_with("https://")).map(str::to_string).collect()).unwrap_or_default();
    if l.is_empty() {
        return Err(Erreur::Serveur("Nexus n'a pas donné d'adresse de téléchargement.".into()));
    }
    Ok(l)
}

/// Le fichier reçu est-il EXACTEMENT ce mod et ce fichier ? (`md5_search` : Nexus retrouve un fichier par son md5.)
pub async fn verifier_md5(base: &str, l: &LienNxm, md5: &str, cle: &str) -> Resultat<bool> {
    if md5.len() != 32 || !md5.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(false);
    }
    let v = match obtenir(base, &format!("/games/{}/mods/md5_search/{md5}", l.jeu), cle).await {
        Ok(v) => v,
        Err(Erreur::Introuvable(_)) => return Ok(false),
        Err(e) => return Err(e),
    };
    Ok(v.as_array().is_some_and(|a| a.iter().any(|r| r["mod"]["mod_id"].as_u64() == Some(l.mod_id) && r["file_details"]["file_id"].as_u64() == Some(l.fichier_id))))
}

/// Le client de TÉLÉCHARGEMENT : sans clé, https seulement (redirections comprises), quelques redirections au plus.
pub fn client_telechargement() -> Resultat<reqwest::Client> {
    reqwest::Client::builder()
        // Un FICHIER téléchargé : jamais décompressé au passage (taille, reprise et empreinte restent exactes).
        .no_gzip()
        .connect_timeout(std::time::Duration::from_secs(15))
        .read_timeout(std::time::Duration::from_secs(90))
        .user_agent(concat!("Frogtend/", env!("CARGO_PKG_VERSION")))
        .https_only(true)
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|_| Erreur::Reseau("Impossible de préparer la connexion.".into()))
}

/// Efface l'archive téléchargée (et son `.part`, et son dossier s'il est vide) quoi qu'il arrive : jamais un fichier non
/// vérifié laissé dans le jeu (« pas de pieuvre »).
pub struct Ramasse(pub std::path::PathBuf);

impl Drop for Ramasse {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
        let _ = std::fs::remove_file(format!("{}.part", self.0.to_string_lossy()));
        if let Some(p) = self.0.parent() {
            let _ = std::fs::remove_dir(p);
        }
    }
}

/// Le md5 d'un fichier (pour `md5_search`).
pub fn md5_de(f: &std::path::Path) -> Resultat<String> {
    use md5::{Digest, Md5};
    use std::io::Read;
    let mut h = Md5::new();
    let mut r = std::fs::File::open(f)?;
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

// --- La boîte aux lettres : un lien reçu par un 2e Frogtend (Windows en lance un à chaque clic) est déposé ici, et le
// Frogtend déjà ouvert le relève. Pas de greffon « une seule instance » : il ferait s'arrêter aussitôt le Frogtend que
// Sunshine lance avec `--taodbox` (avis de l'expert lancement, 20/10). ---

/// Le dossier de la boîte aux lettres (dans les données de Frogtend).
pub fn boite(donnees: &std::path::Path) -> std::path::PathBuf {
    donnees.join("nxm-recus")
}

/// Dépose un lien (déjà lu strictement) dans la boîte : écrit à côté puis renommé.
pub fn deposer(donnees: &std::path::Path, l: &LienNxm) -> Resultat<()> {
    let d = boite(donnees);
    std::fs::create_dir_all(&d)?;
    let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let tmp = d.join(format!("{n}.tmp"));
    let mut texte = format!("nxm://{}/mods/{}/files/{}", l.jeu, l.mod_id, l.fichier_id);
    if let (Some(k), Some(e)) = (&l.key, l.expires) {
        texte.push_str(&format!("?key={k}&expires={e}"));
    }
    std::fs::write(&tmp, texte)?;
    std::fs::rename(&tmp, d.join(format!("{n}.lien")))?;
    Ok(())
}

/// Relève la boîte : les liens valides et pas expirés (relus strictement), chacun retiré une fois lu ; le reste est
/// jeté, y compris un `.tmp` laissé par un arrêt brutal depuis plus de 5 minutes.
pub fn relever(donnees: &std::path::Path) -> Vec<LienNxm> {
    let Ok(l) = std::fs::read_dir(boite(donnees)) else { return Vec::new() };
    let maintenant = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let mut r = Vec::new();
    for e in l.flatten() {
        let p = e.path();
        if p.extension().is_some_and(|x| x == "lien") {
            if e.metadata().is_ok_and(|m| m.len() <= 4096) {
                if let Some(lien) = std::fs::read_to_string(&p).ok().and_then(|t| lire_nxm(t.trim())).filter(|l| !expire(l, maintenant)) {
                    r.push(lien);
                }
            }
            let _ = std::fs::remove_file(&p);
        } else if p.extension().is_some_and(|x| x == "tmp") && e.metadata().ok().and_then(|m| m.modified().ok()).and_then(|t| t.elapsed().ok()).is_some_and(|d| d.as_secs() > 300) {
            let _ = std::fs::remove_file(&p);
        }
    }
    r
}

/// Le lien `nxm://` passé à Frogtend par Windows (le premier argument qui en a la forme), lu strictement.
pub fn lien_des_arguments(args: impl Iterator<Item = String>) -> Option<LienNxm> {
    argument_nxm(args).and_then(|a| lire_nxm(&a))
}

/// Le premier argument qui a la forme d'un lien `nxm://`, lisible ou non (schéma ramené en minuscules : Windows peut le
/// passer en « NXM:// »).
fn argument_nxm(args: impl Iterator<Item = String>) -> Option<String> {
    args.skip(1).find(|a| a.len() > 6 && a.is_char_boundary(6) && a[..6].eq_ignore_ascii_case("nxm://")).map(|a| format!("nxm://{}", &a[6..]))
}

/// Au démarrage : un lien `nxm://` reçu est déposé dans la boîte aux lettres. Rend `true` si un autre Frogtend tourne
/// déjà et que Windows nous a passé un lien `nxm://` (même illisible, comme une collection) : ce Frogtend-ci n'a plus
/// qu'à s'arrêter, avant toute fenêtre (l'autre relève le lien) ; jamais deux Frogtend sur les mêmes données.
/// Sans lien (un lancement normal, ou `--taodbox` par Sunshine), rien ne change : jamais d'arrêt.
pub fn au_demarrage(donnees: &std::path::Path, args: impl Iterator<Item = String>) -> bool {
    let autre = autre_frogtend(donnees);
    let Some(a) = argument_nxm(args) else { return false };
    // Même si le dépôt échoue (disque plein…) ou si le lien est illisible : le lien est perdu, pas les données.
    if let Some(lien) = lire_nxm(&a) {
        let _ = deposer(donnees, &lien);
    }
    autre
}

/// Un autre Frogtend tourne-t-il ? (un mutex nommé de Windows, gardé jusqu'à la fin du programme)
#[cfg(windows)]
fn autre_frogtend(donnees: &std::path::Path) -> bool {
    use windows::core::HSTRING;
    use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
    use windows::Win32::System::Threading::CreateMutexW;
    let nom = format!("Local\\Frogtend-{}", donnees.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
    // SAFETY: appel Win32 sans pointeur à nous ; le handle est volontairement gardé ouvert (jusqu'à la sortie).
    match unsafe { CreateMutexW(None, false, &HSTRING::from(nom)) } {
        // Le handle n'est jamais fermé (HANDLE n'a pas de Drop) : le mutex vit jusqu'à la sortie.
        Ok(_h) => (unsafe { GetLastError() }) == ERROR_ALREADY_EXISTS,
        Err(_) => false,
    }
}

#[cfg(not(windows))]
fn autre_frogtend(_: &std::path::Path) -> bool {
    false
}

/// Un clic « Ouvrir sur Nexus » depuis un jeu : c'est lui qui dit à quel jeu va un lien reçu (30 minutes au plus).
#[derive(Debug, Clone)]
struct Clic {
    profil: String,
    jeu_id: i64,
    mod_nom: String,
    domaine: String,
    mod_id: u64,
    quand: std::time::Instant,
}

/// Un lien reçu, tel que l'interface le voit (sans sa clé).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct LienRecu {
    pub n: u32,
    pub jeu_id: Option<i64>,
    pub mod_nom: Option<String>,
    pub domaine: String,
    pub mod_id: u64,
    pub fichier_id: u64,
}

/// Un lien reçu associé (ou non) à un jeu du profil.
#[derive(Debug, Clone)]
pub struct Recu {
    pub n: u32,
    pub profil: String,
    pub lien: LienNxm,
    pub jeu_id: Option<i64>,
    pub mod_nom: Option<String>,
}

/// Les clics et les liens reçus (en mémoire seulement : la clé à usage unique n'est jamais écrite ailleurs que dans la
/// boîte aux lettres, le temps du relevé).
#[derive(Default)]
pub struct Nexus {
    clics: std::sync::Mutex<Vec<Clic>>,
    recus: std::sync::Mutex<Vec<Recu>>,
    prochain: std::sync::atomic::AtomicU32,
}

const CLIC_VALABLE: std::time::Duration = std::time::Duration::from_secs(30 * 60);

impl Nexus {
    /// Note un clic « Ouvrir sur Nexus » (un clic plus ancien sur le même mod est remplacé).
    pub fn noter_clic(&self, profil: &str, jeu_id: i64, mod_nom: &str, domaine: &str, mod_id: u64) {
        let mut c = self.clics.lock().unwrap_or_else(|e| e.into_inner());
        c.retain(|x| x.quand.elapsed() < CLIC_VALABLE && !(x.profil == profil && x.domaine == domaine && x.mod_id == mod_id));
        c.push(Clic { profil: profil.into(), jeu_id, mod_nom: mod_nom.into(), domaine: domaine.into(), mod_id, quand: std::time::Instant::now() });
    }

    /// Range des liens relevés pour ce profil, chacun associé au dernier clic récent sur le même mod.
    pub fn recevoir(&self, profil: &str, liens: Vec<LienNxm>) {
        let clics = self.clics.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let mut r = self.recus.lock().unwrap_or_else(|e| e.into_inner());
        for lien in liens {
            let clic = clics.iter().rev().find(|c| c.profil == profil && c.quand.elapsed() < CLIC_VALABLE && c.domaine == lien.jeu && c.mod_id == lien.mod_id);
            // Le même fichier reçu deux fois (double clic) : un seul.
            if r.iter().any(|x| x.profil == profil && x.lien.mod_id == lien.mod_id && x.lien.fichier_id == lien.fichier_id && x.lien.jeu == lien.jeu) {
                continue;
            }
            let n = self.prochain.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            r.push(Recu { n, profil: profil.into(), jeu_id: clic.map(|c| c.jeu_id), mod_nom: clic.map(|c| c.mod_nom.clone()), lien });
        }
    }

    /// Les liens reçus pour ce profil (jamais ceux d'un autre).
    pub fn liens(&self, profil: &str) -> Vec<LienRecu> {
        self.recus
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter(|x| x.profil == profil)
            .map(|x| LienRecu { n: x.n, jeu_id: x.jeu_id, mod_nom: x.mod_nom.clone(), domaine: x.lien.jeu.clone(), mod_id: x.lien.mod_id, fichier_id: x.lien.fichier_id })
            .collect()
    }

    pub fn recu(&self, profil: &str, n: u32) -> Option<Recu> {
        self.recus.lock().unwrap_or_else(|e| e.into_inner()).iter().find(|x| x.profil == profil && x.n == n).cloned()
    }

    pub fn oublier(&self, profil: &str, n: u32) {
        self.recus.lock().unwrap_or_else(|e| e.into_inner()).retain(|x| !(x.profil == profil && x.n == n));
    }

    /// Un profil fermé ou supprimé : ses clics et ses liens partent.
    pub fn oublier_profil(&self, profil: &str) {
        self.clics.lock().unwrap_or_else(|e| e.into_inner()).retain(|x| x.profil != profil);
        self.recus.lock().unwrap_or_else(|e| e.into_inner()).retain(|x| x.profil != profil);
    }
}

/// Des liens attendent-ils dans la boîte ? (sans les lire)
pub fn courrier(donnees: &std::path::Path) -> usize {
    std::fs::read_dir(boite(donnees)).map(|l| l.flatten().filter(|e| e.path().extension().is_some_and(|x| x == "lien")).count()).unwrap_or(0)
}

/// Un nom de fichier de Nexus que Frogtend sait poser (zip ou 7z ; le format réel est revérifié sur les octets reçus).
pub fn nom_pris_en_charge(nom: &str) -> bool {
    let n = nom.to_ascii_lowercase();
    n.ends_with(".zip") || n.ends_with(".7z")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_lien_nxm_se_lit_strictement_et_ne_montre_jamais_sa_cle() {
        let l = lire_nxm("nxm://skyrimspecialedition/mods/266/files/1000172397?key=AbC-12_x&expires=1760000000&user_id=42").unwrap();
        assert_eq!((l.jeu.as_str(), l.mod_id, l.fichier_id, l.expires), ("skyrimspecialedition", 266, 1000172397, Some(1760000000)));
        assert!(!format!("{l:?}").contains("AbC"), "la clé à usage unique ne s'affiche jamais");
        // Premium : sans clé ni échéance.
        assert!(lire_nxm("nxm://skyrim/mods/1/files/2").unwrap().key.is_none());
        for faux in [
            "http://skyrim/mods/1/files/2",
            "nxm://skyrim/collections/abc/revisions/1",
            "nxm://Skyrim/mods/1/files/2",
            "nxm://sky..rim/mods/1/files/2",
            "nxm://skyrim/mods/-1/files/2",
            "nxm://skyrim/mods/0/files/2",
            "nxm://skyrim/mods/1/files/2/extra",
            "nxm://skyrim/mods/1/files/2?key=a",
            "nxm://skyrim/mods/1/files/2?key=a&expires=1&key=b",
            "nxm://skyrim/mods/1/files/2?key=a&expires=1&autre=2",
            "nxm://skyrim/mods/1/files/2?key=a b&expires=1",
            "nxm://skyrim/mods/1/files/2?key=a&expires=-5",
        ] {
            assert!(lire_nxm(faux).is_none(), "{faux}");
        }
        assert!(expire(&l, 1760000000) && !expire(&l, 1759999999));
        assert_eq!(page_nexus("https://www.nexusmods.com/skyrimspecialedition/mods/266?tab=files"), Some(("skyrimspecialedition".into(), 266)));
        assert_eq!(page_nexus("https://evil.example/skyrim/mods/1"), None);
        assert_eq!(plafond(1000), 1001 * 1024);
    }

    #[test]
    fn la_boite_aux_lettres_ne_rend_que_des_liens_sains_une_seule_fois() {
        let d = tempfile::tempdir().unwrap();
        let l = lire_nxm("nxm://skyrim/mods/5/files/9?key=k&expires=4102444800").unwrap();
        deposer(d.path(), &l).unwrap();
        std::fs::write(boite(d.path()).join("faux.lien"), "nxm://evil/../x").unwrap();
        let r = relever(d.path());
        assert_eq!(r, [l]);
        assert!(relever(d.path()).is_empty(), "relevé une seule fois ; le faux est jeté");
        // Un lien expiré est jeté sans être rendu.
        deposer(d.path(), &lire_nxm("nxm://skyrim/mods/5/files/9?key=k&expires=99").unwrap()).unwrap();
        assert!(relever(d.path()).is_empty() && courrier(d.path()) == 0);
        // Le ramasse-archive : l'archive, son .part et le dossier vide partent, quoi qu'il arrive.
        let t = d.path().join("telechargements");
        std::fs::create_dir_all(&t).unwrap();
        std::fs::write(t.join("m.zip"), b"x").unwrap();
        std::fs::write(t.join("m.zip.part"), b"x").unwrap();
        drop(Ramasse(t.join("m.zip")));
        assert!(!t.exists());
    }

    #[test]
    fn un_second_frogtend_ne_s_arrete_que_s_il_apporte_un_lien() {
        let d = tempfile::tempdir().unwrap();
        let args = |l: &[&str]| l.iter().map(|s| s.to_string()).collect::<Vec<_>>().into_iter();
        // Le premier : il garde le lien pour lui et démarre.
        assert!(!au_demarrage(d.path(), args(&["frogtend.exe", "nxm://skyrim/mods/1/files/2?key=k&expires=4102444800"])));
        // Sunshine lance « --taodbox » pendant que Frogtend tourne : jamais arrêté.
        assert!(!au_demarrage(d.path(), args(&["frogtend.exe", "--taodbox"])));
        // Un faux lien ou une collection, Frogtend déjà ouvert : rien de déposé, mais ce Frogtend-ci s'arrête (jamais
        // deux Frogtend sur les mêmes données).
        assert!(au_demarrage(d.path(), args(&["frogtend.exe", "nxm://evil/../x"])));
        assert!(au_demarrage(d.path(), args(&["frogtend.exe", "nxm://skyrim/collections/abc/revisions/1"])));
        // Un second clic sur Nexus : déposé, et ce Frogtend-ci s'arrête.
        assert!(au_demarrage(d.path(), args(&["frogtend.exe", "nxm://skyrim/mods/1/files/3"])));
        assert_eq!(relever(d.path()).len(), 2);
        assert_eq!(lien_des_arguments(args(&["nxm://skyrim/mods/1/files/2"])), None, "le nom du programme n'est pas un argument");
        assert!(lien_des_arguments(args(&["frogtend.exe", "NXM://skyrim/mods/1/files/2"])).is_some(), "schéma en majuscules");
    }

    #[test]
    fn un_lien_va_au_jeu_du_dernier_clic_du_meme_profil_seulement() {
        let n = Nexus::default();
        n.noter_clic("seb", 12, "Fusion Fix", "gtaiv", 5);
        n.noter_clic("lea", 99, "Autre", "gtaiv", 5);
        let l = lire_nxm("nxm://gtaiv/mods/5/files/9?key=k&expires=99").unwrap();
        n.recevoir("seb", vec![l.clone(), l.clone(), lire_nxm("nxm://skyrim/mods/1/files/2").unwrap()]);
        let r = n.liens("seb");
        assert_eq!(r.len(), 2, "le même fichier reçu deux fois : un seul");
        assert_eq!((r[0].jeu_id, r[0].mod_nom.as_deref()), (Some(12), Some("Fusion Fix")));
        assert_eq!(r[1].jeu_id, None, "sans clic : à aucun jeu");
        assert!(n.liens("lea").is_empty(), "jamais les liens d'un autre profil");
        assert!(n.recu("lea", r[0].n).is_none());
        n.oublier("seb", r[0].n);
        assert_eq!(n.liens("seb").len(), 1);
        n.oublier_profil("seb");
        assert!(n.liens("seb").is_empty());
        assert!(nom_pris_en_charge("Mod-1.ZIP") && nom_pris_en_charge("mod.7z") && !nom_pris_en_charge("mod.rar") && !nom_pris_en_charge("mod.zip.exe"));
    }

    #[tokio::test]
    async fn l_api_de_nexus_recoit_la_cle_et_rien_d_autre_ne_la_recoit() {
        use httpmock::prelude::*;
        let m = MockServer::start();
        m.mock(|w, t| {
            w.method(GET).path("/users/validate").header("APIKEY", "ma-cle").header("Application-Name", "Frogtend");
            t.status(200).header("content-type", "application/json").json_body(serde_json::json!({"user_id": 1, "key": "ma-cle", "name": "Seb", "is_premium": false, "email": "x@y"}));
        });
        m.mock(|w, t| {
            w.method(GET).path("/games/skyrim/mods/5/files/9/download_link").query_param("key", "k").query_param("expires", "99");
            t.status(200).header("content-type", "application/json").json_body(serde_json::json!([{"URI": "https://cdn.example/f.zip?sig=1", "name": "CDN", "short_name": "c"}, {"URI": "http://pas-https/x"}]));
        });
        m.mock(|w, t| {
            w.method(GET).path("/games/skyrim/mods/5/files/9");
            t.status(200).header("content-type", "application/json").json_body(serde_json::json!({"file_name": "Mod-5-1-0.zip", "size_kb": 120, "name": "Mon mod"}));
        });
        m.mock(|w, t| {
            w.method(GET).path_contains("/games/skyrim/mods/md5_search/");
            t.status(200).header("content-type", "application/json").json_body(serde_json::json!([{"mod": {"mod_id": 5}, "file_details": {"file_id": 9}}]));
        });
        m.mock(|w, t| {
            w.method(GET).path("/redirige");
            t.status(302).header("location", "https://ailleurs.example/vole");
        });
        let base = m.base_url();
        assert_eq!(valider(&base, "ma-cle").await.unwrap(), "Seb");
        assert!(valider(&base, "ma cle").await.is_err(), "une clé avec une espace n'est même pas envoyée");
        let l = lire_nxm("nxm://skyrim/mods/5/files/9?key=k&expires=99").unwrap();
        assert_eq!(adresses(&base, &l, "ma-cle").await.unwrap(), ["https://cdn.example/f.zip?sig=1"], "seulement https");
        assert_eq!(fichier(&base, &l, "ma-cle").await.unwrap(), FichierNexus { nom: "Mod-5-1-0.zip".into(), taille_ko: 120, nom_mod: "Mon mod".into() });
        assert!(verifier_md5(&base, &l, &"a".repeat(32), "ma-cle").await.unwrap());
        let autre = lire_nxm("nxm://skyrim/mods/5/files/10").unwrap();
        assert!(!verifier_md5(&base, &autre, &"a".repeat(32), "ma-cle").await.unwrap(), "un autre fichier : refusé");
        // Une redirection de l'API n'est jamais suivie (la clé partirait ailleurs).
        assert!(obtenir(&base, "/redirige", "ma-cle").await.is_err());
        // Ni la clé ni l'adresse dans un message d'erreur.
        let e = obtenir("http://127.0.0.1:1", "/users/validate", "ma-cle").await.unwrap_err().to_string();
        assert!(!e.contains("ma-cle") && !e.contains("127.0.0.1"), "{e}");
    }
}
