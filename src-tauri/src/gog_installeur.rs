//! Lot C4c-2 (décisions de Seb des 26 et 27/11) : INSTALLER un jeu GOG possédé SANS GOG Galaxy, par l'installeur hors
//! ligne OFFICIEL de GOG (`setup_*.exe` d'Inno Setup et ses `.bin`), avec les DLC possédés que la personne a gardés
//! cochés.
//!
//! Le chemin, dans l'ordre (chaque étape VÉRIFIÉE avant la suivante) :
//! 1. la fiche publique `api.gog.com/products/<id>?expand=downloads,expanded_dlcs` (sans jeton : relevé réel du 26/11)
//!    donne l'installeur Windows français, sinon anglais, et ceux des DLC (même réponse, relevé du 27/11) ;
//! 2. pour chaque fichier, le `downlink` (jeton Bearer, `api.gog.com` SEULEMENT, sans redirection) rend deux liens
//!    SIGNÉS du CDN (un jeton dans le chemin : des secrets) : le fichier et son XML de contrôle ;
//! 3. le XML de contrôle (plafonné) donne le nom, la taille EXACTE (`total_size` ; celle de `products` est arrondie) et
//!    le MD5 de chaque morceau de 10 Mo ; le fichier est reçu dans `<nom>.part`, chaque morceau vérifié PENDANT la
//!    réception (un morceau faux est redemandé une fois), la reprise repart de la dernière frontière vérifiée, et le
//!    lien est redemandé à chaque départ et à chaque reprise (il vit ≈ 24 h, le jeton 3600 s) ;
//! 4. la signature Authenticode de chaque `.exe` (signataire « GOG Sp. z o.o. », statut valide) AVANT de le lancer (les
//!    `.bin` sont couverts par le MD5 de GOG) ;
//! 5. l'installeur, élevé par Windows (ShellExecuteExW : un refus de l'UAC se reconnaît, code 1223), en silence ;
//! 6. la vérification sans se fier au code de sortie : `goggame-<id>.info` et sa tâche de jeu, l'entrée du registre
//!    `GOG.com\Games\<id>` avec le bon dossier, et la lecture des jeux GOG installés qui la garde ;
//! 7. la marque « installé par Frogtend » et, seulement alors, l'effacement des fichiers de l'installeur (ceux de SA
//!    note, rien d'autre).
//!
//! Secrets : le jeton d'accès ne part que vers `api.gog.com` ; les liens signés ne sont jamais journalisés, écrits dans
//! un fichier, montrés ni mis dans une erreur (`without_url`) ; leur `Debug` est caché. Le journal ne reçoit que des
//! numéros et des chiffres.
//!
//! Les arguments d'Inno Setup viennent de sa documentation officielle (relue le 27/11 sur
//! <https://jrsoftware.org/ishelp/topic_setupcmdline.htm>) :
//! - `/SP-` : « Disables the This will install... Do you wish to continue? prompt at the beginning of Setup. »
//! - `/VERYSILENT` : « Instructs Setup to be silent or very silent. »
//! - `/SUPPRESSMSGBOXES` : « Instructs Setup to suppress message boxes. Only has an effect when combined with /SILENT or
//!   /VERYSILENT. »
//! - `/NORESTART` : « Prevents Setup from restarting the system following a successful installation »
//! - `/NOCANCEL` : « Prevents the user from cancelling during the installation process, by disabling the Cancel button »
//! - `/DIR="x:\dirname"` : « Overrides the default directory name […] A fully qualified pathname must be specified. »
//! - `/LOG="filename"` : « allows you to specify a fixed path/filename to use for the log file. If a file with the
//!   specified name already exists it will be overwritten. »
//! - PAS de `/CURRENTUSER` (« Instructs Setup to install in non administrative install mode ») : Frogtend lit les jeux
//!   GOG dans HKLM (`boutiques_pc::lire_gog`) ; PAS de `/LANG` (l'installeur français est choisi dans `products`).
//!
//! Les codes de sortie (<https://jrsoftware.org/ishelp/topic_setupexitcodes.htm>, relue le 27/11) : « 0 Setup was
//! successfully run to completion » ; « 1 Setup failed to initialize » ; 2 : annulé avant le début ; « 3 A fatal error
//! occurred while preparing to move to the next installation phase » ; « 4 A fatal error occurred during the actual
//! installation process » ; 5 : annulé pendant l'installation ; 6 : arrêté de force ; « 7 The Preparing to Install stage
//! determined that Setup cannot proceed with installation » ; 8 : comme 7, avec un redémarrage nécessaire. « Any
//! non-zero code means Setup did not run to completion. » Frogtend ne s'y fie pas pour dire « installé » : il vérifie.

use crate::boutiques::CompteGog;
use crate::boutiques_pc::Valeurs;
use crate::erreurs::{Erreur, Resultat};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// L'API de GOG (le seul hôte qui reçoit le jeton).
pub const API: &str = crate::gog_galaxy::PRODUITS;
/// Le plus gros XML de contrôle accepté (≈ 12 morceaux pour 120 Mo, des milliers pour 4 Go : bien en dessous).
pub const PLAFOND_XML: usize = 4 * 1024 * 1024;
/// La plus grosse fiche `products` acceptée.
pub const PLAFOND_FICHE: usize = 16 * 1024 * 1024;
/// Le signataire Authenticode des installeurs de GOG.
pub const SIGNATAIRE_GOG: &str = "GOG Sp. z o.o.";
/// Le sous-dossier de l'emplacement réglé « Windows » où attendent les installeurs (règle « pas de pieuvre ») ; un
/// sous-dossier par numéro (jeu ou DLC).
pub const DOSSIER_INSTALLEURS: &str = "Installeurs GOG (Frogtend)";
/// Le dossier propre à Frogtend dans le dossier du jeu, nommé comme ses autres entrées `.frogtend…` (ignorées par
/// `jeux_pc::dossier_des_parties_de` ; le relevé des parties lui-même ne les filtre pas : non vérifié plus loin) : la
/// marque et le journal de l'installeur.
pub const DOSSIER_MARQUE: &str = ".frogtend-gog";
pub const FICHIER_MARQUE: &str = "installe-par-frogtend.json";
/// Le journal d'Inno Setup (`/LOG=`), écrit à côté de l'installeur, puis copié avec la marque.
pub const JOURNAL_INNO: &str = "installation-gog.log";
/// La note d'un installeur reçu : les SEULS fichiers effacés après une installation vérifiée.
pub const NOTE_INSTALLEUR: &str = "frogtend-installeur.json";
/// Les coupures de suite (sans un morceau de vérifié entre elles) avant d'abandonner (le `.part` est gardé).
const COUPURES_MAX: u32 = 3;
/// Une marge sur les tailles de `products` (arrondies : 122 683 392 annoncés pour 123 571 824 réels le 26/11).
pub const MARGE_ARRONDI_POURCENT: u64 = 2;

/// Le refus de l'UAC (réponse Non à Windows) : une alerte, pas une erreur (décision de l'expert interface).
pub const MESSAGE_UAC: &str =
    "⚠ Installation annulée : tu as répondu Non à Windows. Rien n'a été installé. L'installeur est gardé : ⬇ Installer pour réessayer.";

// ───────────────────────────── La fiche (products) ─────────────────────────────

/// Un fichier d'un installeur dans `products` : son numéro (`en1installer0`), sa taille ANNONCÉE (arrondie), et la
/// route `downlink` de `api.gog.com` (elle exige le jeton). Son `Debug` cache la route.
#[derive(Clone, PartialEq)]
pub struct FichierInstalleur {
    pub id: String,
    pub taille_annoncee: u64,
    lien: String,
}

impl std::fmt::Debug for FichierInstalleur {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FichierInstalleur({}, {} octets annoncés)", self.id, self.taille_annoncee)
    }
}

/// Un installeur Windows de `products` (clés relevées le 26/11 : id, language, os, total_size, version, files).
#[derive(Clone, Debug, PartialEq)]
pub struct InstalleurGog {
    pub id: String,
    pub langue: String,
    pub version: String,
    /// La taille annoncée par `products` (arrondie) : pour l'accord et la place, jamais pour vérifier.
    pub total_annonce: u64,
    pub fichiers: Vec<FichierInstalleur>,
}

/// Un jeu ou un DLC à installer.
#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    pub id: String,
    pub titre: String,
    pub installeur: InstalleurGog,
}

/// Ce que `products` dit d'un jeu pour l'installer.
#[derive(Clone, Debug, PartialEq)]
pub struct ProduitInstallable {
    pub id: String,
    pub titre: String,
    /// Le jeu contient le français (`languages` de `products`, pas la langue de l'installeur : Cyberpunk n'a qu'un
    /// installeur « en » et contient le français, relevé du 27/11).
    pub en_francais: bool,
    /// L'installeur Windows du jeu (français, sinon anglais) ; `None` : pas d'installeur hors ligne Windows.
    pub jeu: Option<Element>,
    /// Les DLC POSSÉDÉS par le compte qui ont un installeur Windows (un DLC sans installeur n'est pas proposé).
    pub dlcs: Vec<Element>,
}

/// Un numéro GOG (nombre ou texte de chiffres).
fn numero(v: &Value) -> Option<String> {
    let n = v.as_u64().map(|n| n.to_string()).or_else(|| v.as_str().map(str::to_string))?;
    crate::gratuits::numero_gog(&n).then_some(n)
}

/// Un identifiant d'installeur ou de fichier sûr (lettres, chiffres, `_ - .`).
fn identifiant_sur(s: &str) -> bool {
    !s.is_empty() && s.len() <= 100 && s.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.'))
}

/// Une route de `api.gog.com` (sous `api`, sans hôte ni utilisateur détourné) : la seule où part le jeton.
pub fn lien_api_sur(lien: &str, api: &str) -> bool {
    let (Ok(u), Ok(base)) = (reqwest::Url::parse(lien), reqwest::Url::parse(api)) else { return false };
    u.scheme() == base.scheme() && u.host_str() == base.host_str() && u.port_or_known_default() == base.port_or_known_default() && u.username().is_empty() && u.password().is_none() && u.path().starts_with("/products/")
}

/// L'installeur Windows de la liste `installers` : le français, sinon l'anglais (décision de Seb, 26/11). Un
/// installeur dont un fichier est douteux (numéro, route) n'est pas pris.
pub fn installeur_windows(installers: &Value, api: &str) -> Option<InstalleurGog> {
    installeur_windows_dans(installers, api, &["fr", "en"])
}

/// L'installeur Windows dans la première langue de `ordre` qui en a un (un DLC : d'abord la langue de l'installeur
/// du jeu, relecture de l'expert du 27/11).
fn installeur_windows_dans(installers: &Value, api: &str, ordre: &[&str]) -> Option<InstalleurGog> {
    let liste = installers.as_array()?;
    ordre.iter().find_map(|langue| {
        liste.iter().filter(|i| i["os"].as_str() == Some("windows") && i["language"].as_str() == Some(langue)).find_map(|i| {
            let id = i["id"].as_str().map(str::to_string).or_else(|| i["id"].as_u64().map(|n| n.to_string())).filter(|s| identifiant_sur(s))?;
            let fichiers: Vec<FichierInstalleur> = i["files"]
                .as_array()?
                .iter()
                .map(|f| {
                    let id = f["id"].as_str().filter(|s| identifiant_sur(s))?.to_string();
                    let lien = f["downlink"].as_str().filter(|l| lien_api_sur(l, api))?.to_string();
                    Some(FichierInstalleur { id, taille_annoncee: f["size"].as_u64().unwrap_or(0), lien })
                })
                .collect::<Option<Vec<_>>>()?;
            if fichiers.is_empty() {
                return None;
            }
            let total = i["total_size"].as_u64().unwrap_or_else(|| fichiers.iter().map(|f| f.taille_annoncee).sum());
            Some(InstalleurGog { id, langue: (*langue).to_string(), version: i["version"].as_str().unwrap_or("").chars().take(100).collect(), total_annonce: total, fichiers })
        })
    })
}

/// Le jeu contient-il le français ? (`languages` : un objet `{"fr": "français", …}` ; une liste est acceptée aussi.)
pub fn contient_le_francais(v: &Value) -> bool {
    let fr = |k: &str| k.eq_ignore_ascii_case("fr") || k.to_ascii_lowercase().starts_with("fr-") || k.to_ascii_lowercase().starts_with("fr_");
    match &v["languages"] {
        Value::Object(o) => o.keys().any(|k| fr(k)),
        Value::Array(a) => a.iter().filter_map(Value::as_str).any(fr),
        _ => false,
    }
}

/// Lit la fiche `products?expand=downloads,expanded_dlcs` (formes relevées les 26 et 27/11). `possedes` : les numéros
/// de la bibliothèque du compte (`ids_lus`) : seuls ses DLC possédés sont gardés. `None` : fiche illisible.
pub fn lire_produit_installation(v: &Value, possedes: &[String], api: &str) -> Option<ProduitInstallable> {
    let id = numero(&v["id"])?;
    let titre = crate::boutiques_pc::sans_marques(v["title"].as_str()?);
    if titre.is_empty() {
        return None;
    }
    let titre: String = titre.chars().take(300).collect();
    let jeu = installeur_windows(&v["downloads"]["installers"], api).map(|installeur| Element { id: id.clone(), titre: titre.clone(), installeur });
    let langue_du_jeu = jeu.as_ref().map(|j| j.installeur.langue.clone()).unwrap_or_else(|| "fr".into());
    let ordre_dlc = [langue_du_jeu.as_str(), "fr", "en"];
    let dlcs = v["expanded_dlcs"]
        .as_array()
        .map(|l| {
            l.iter()
                .filter_map(|d| {
                    let id = numero(&d["id"])?;
                    if !possedes.contains(&id) {
                        return None;
                    }
                    let installeur = installeur_windows_dans(&d["downloads"]["installers"], api, &ordre_dlc)?;
                    let t = crate::boutiques_pc::sans_marques(d["title"].as_str().unwrap_or(""));
                    let titre = if t.is_empty() { format!("DLC {id}") } else { t.chars().take(300).collect() };
                    Some(Element { id, titre, installeur })
                })
                .collect()
        })
        .unwrap_or_default();
    Some(ProduitInstallable { id, titre, en_francais: contient_le_francais(v), jeu, dlcs })
}

/// Le compte GOG du profil qui possède ce jeu (sa bibliothèque lue, ou ses jeux) ; le compte `Profil` d'abord. Sans
/// compte qui le possède, le SEUL compte du profil s'il n'y en a qu'un. Jamais un compte sans numéro Galaxy (pas de
/// jeton pour lui). Pur, testé.
pub fn compte_possesseur<'a>(comptes: &'a [CompteGog], jeu: &str) -> Option<&'a CompteGog> {
    let cle = format!("gog_{jeu}");
    let avec_jeton: Vec<&CompteGog> = comptes.iter().filter(|c| !c.id_galaxy.is_empty()).collect();
    let possede = |c: &&&CompteGog| c.ids_lus.iter().any(|i| i == jeu) || c.jeux.iter().any(|j| j.id == cle);
    avec_jeton
        .iter()
        .filter(possede)
        .find(|c| c.navigateur == crate::boutiques::NavigateurGog::Profil)
        .or_else(|| avec_jeton.iter().find(possede))
        .copied()
        .or_else(|| (comptes.len() == 1).then(|| avec_jeton.first().copied()).flatten())
}

/// La place à demander pour ces installeurs (tailles ANNONCÉES) : l'installeur ET le jeu installé (2 ×), une marge
/// pour l'arrondi de `products`, et la marge de Windows (`jeux_pc::MARGE_LIBRE`).
pub fn place_necessaire(annonce: u64) -> u64 {
    let avec_arrondi = annonce.saturating_add(annonce / 100 * MARGE_ARRONDI_POURCENT);
    avec_arrondi.saturating_mul(2).saturating_add(crate::jeux_pc::MARGE_LIBRE)
}

// ───────────────────────────── Le XML de contrôle ─────────────────────────────

/// Un morceau du XML de contrôle (`<chunk id from to method="md5">md5</chunk>`), bornes comprises.
#[derive(Clone, Debug, PartialEq)]
pub struct Morceau {
    pub debut: u64,
    pub fin: u64,
    pub md5: String,
}

/// Le XML de contrôle d'un fichier (`<file name md5 chunks total_size …>`, relevé du 26/11).
#[derive(Clone, Debug, PartialEq)]
pub struct Controle {
    pub nom: String,
    pub md5: String,
    pub total: u64,
    pub morceaux: Vec<Morceau>,
}

fn illisible(pourquoi: &str) -> Erreur {
    Erreur::Serveur(format!("GOG a envoyé des informations de vérification illisibles pour cet installeur ({pourquoi}) : rien n'est téléchargé. Réessaie plus tard."))
}

/// Les entités de base du XML.
fn entites(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

/// Les attributs `nom="valeur"` (ou entre apostrophes) d'une balise.
fn attributs(corps: &str) -> Vec<(String, String)> {
    let mut v = vec![];
    let mut reste = corps;
    loop {
        let Some(egal) = reste.find('=') else { break };
        let nom = reste[..egal].trim().to_string();
        let apres = reste[egal + 1..].trim_start();
        let Some(q) = apres.chars().next().filter(|c| *c == '"' || *c == '\'') else { break };
        let apres = &apres[1..];
        let Some(fin) = apres.find(q) else { break };
        v.push((nom, entites(&apres[..fin])));
        reste = &apres[fin + 1..];
    }
    v
}

fn attribut<'a>(a: &'a [(String, String)], nom: &str) -> Option<&'a str> {
    a.iter().find(|(k, _)| k == nom).map(|(_, v)| v.as_str())
}

/// Un MD5 en hexadécimal (32 caractères), en minuscules.
fn md5_sur(s: &str) -> Option<String> {
    let s = s.trim();
    (s.len() == 32 && s.bytes().all(|c| c.is_ascii_hexdigit())).then(|| s.to_ascii_lowercase())
}

/// Lit et VÉRIFIE le XML de contrôle : racine `<file>` disponible, nom de fichier sûr (gardé tel quel : jamais
/// renommé), MD5 de 32 caractères, taille > 0, morceaux contigus de 0 à `total_size - 1`, numérotés dans l'ordre, MD5
/// chacun, et leur nombre égal à `chunks`. Tout écart : refus (rien n'est téléchargé sur une liste douteuse).
pub fn lire_controle(xml: &str) -> Resultat<Controle> {
    if xml.len() > PLAFOND_XML {
        return Err(illisible("trop grande"));
    }
    // La première balise qui n'est ni une déclaration ni un commentaire : <file …>.
    let mut i = 0;
    let (racine, apres_racine) = loop {
        let debut = i + xml[i..].find('<').ok_or_else(|| illisible("pas de balise"))?;
        let fin = debut + xml[debut..].find('>').ok_or_else(|| illisible("balise coupée"))?;
        let balise = &xml[debut + 1..fin];
        i = fin + 1;
        if balise.starts_with('?') || balise.starts_with('!') {
            continue;
        }
        break (balise.trim_end_matches('/').to_string(), i);
    };
    let (nom_balise, corps) = racine.split_once(char::is_whitespace).unwrap_or((racine.as_str(), ""));
    if nom_balise != "file" {
        return Err(illisible("racine inattendue"));
    }
    let a = attributs(corps);
    if attribut(&a, "available").is_some_and(|v| v.trim() == "0") {
        return Err(Erreur::Serveur("GOG dit que ce fichier n'est pas disponible pour l'instant : réessaie plus tard.".into()));
    }
    let nom = attribut(&a, "name").ok_or_else(|| illisible("sans nom"))?.to_string();
    if crate::jeux_pc::nom_de_fichier_sur(&nom).is_err() || nom.len() > 255 || nom.ends_with(['.', ' ']) {
        return Err(illisible("nom de fichier inutilisable"));
    }
    let md5 = attribut(&a, "md5").and_then(md5_sur).ok_or_else(|| illisible("empreinte du fichier"))?;
    let total: u64 = attribut(&a, "total_size").and_then(|t| t.trim().parse().ok()).filter(|t| *t > 0).ok_or_else(|| illisible("taille"))?;
    let annonces: usize = attribut(&a, "chunks").and_then(|t| t.trim().parse().ok()).ok_or_else(|| illisible("nombre de morceaux"))?;
    let mut morceaux: Vec<Morceau> = Vec::new();
    let mut reste = &xml[apres_racine..];
    while let Some(p) = reste.find("<chunk") {
        let suite = &reste[p + "<chunk".len()..];
        let fin_balise = suite.find('>').ok_or_else(|| illisible("morceau coupé"))?;
        let a = attributs(&suite[..fin_balise]);
        let corps = &suite[fin_balise + 1..];
        let fin_texte = corps.find("</chunk>").ok_or_else(|| illisible("morceau sans fin"))?;
        let texte = &corps[..fin_texte];
        let n: usize = attribut(&a, "id").and_then(|t| t.trim().parse().ok()).ok_or_else(|| illisible("numéro de morceau"))?;
        let de: u64 = attribut(&a, "from").and_then(|t| t.trim().parse().ok()).ok_or_else(|| illisible("début de morceau"))?;
        let a_: u64 = attribut(&a, "to").and_then(|t| t.trim().parse().ok()).ok_or_else(|| illisible("fin de morceau"))?;
        if attribut(&a, "method").map(str::trim) != Some("md5") {
            return Err(illisible("méthode de contrôle inconnue"));
        }
        let m = md5_sur(texte).ok_or_else(|| illisible("empreinte de morceau"))?;
        let attendu = morceaux.last().map_or(0, |x| x.fin + 1);
        if n != morceaux.len() || de != attendu || a_ < de || a_ >= total {
            return Err(illisible("morceaux non contigus"));
        }
        morceaux.push(Morceau { debut: de, fin: a_, md5: m });
        if morceaux.len() > 1_000_000 {
            return Err(illisible("trop de morceaux"));
        }
        reste = &corps[fin_texte + "</chunk>".len()..];
    }
    if morceaux.is_empty() || morceaux.len() != annonces || morceaux.last().map(|m| m.fin + 1) != Some(total) {
        return Err(illisible("morceaux incomplets"));
    }
    Ok(Controle { nom, md5, total, morceaux })
}

// ───────────────────────────── Le réseau ─────────────────────────────

/// Le jeton d'accès de GOG, renouvelé à la demande (`renouveler` : celui d'avant a été refusé). Implémenté par le
/// noyau (coffre de Windows, verrou des comptes GOG) et par un faux dans les tests.
pub trait SourceJeton: Sync {
    fn jeton(&self, renouveler: bool) -> impl std::future::Future<Output = Resultat<String>> + Send;
}

/// Où parler à GOG. `https` : vrai hors des tests (le CDN, ses redirections et l'API en https seulement, le CDN sous
/// `*.gog.com`).
#[derive(Clone, Debug)]
pub struct ReseauGog {
    pub api: String,
    https: bool,
}

impl ReseauGog {
    pub fn officiel() -> Self {
        ReseauGog { api: API.into(), https: true }
    }

    /// Un faux GOG local (tests seulement).
    #[cfg(test)]
    pub fn essai(api: &str) -> Self {
        ReseauGog { api: api.trim_end_matches('/').into(), https: false }
    }

    /// Le client de l'API (petites réponses JSON) : sans cookies, sans redirection suivie (le jeton ne suit jamais une
    /// redirection), 30 s au plus.
    fn client_api(&self) -> Resultat<reqwest::Client> {
        reqwest::Client::builder()
            .user_agent("Frogtend")
            .timeout(std::time::Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .https_only(self.https)
            .build()
            .map_err(|_| Erreur::Reseau("Connexion impossible.".into()))
    }

    /// Le client du CDN : JAMAIS de jeton, https seulement, 15 s pour se connecter, 90 s sans rien recevoir, aucune
    /// limite de durée totale (un fichier de 4 Go prend des heures chez la sœur de Seb), 5 redirections au plus, et
    /// seulement vers `*.gog.com`.
    fn client_cdn(&self) -> Resultat<reqwest::Client> {
        let https = self.https;
        reqwest::Client::builder()
            .user_agent("Frogtend")
            .connect_timeout(std::time::Duration::from_secs(15))
            .read_timeout(std::time::Duration::from_secs(90))
            .https_only(https)
            .redirect(reqwest::redirect::Policy::custom(move |essai| {
                if essai.previous().len() >= 5 || !hote_cdn_permis(essai.url(), https) {
                    essai.stop()
                } else {
                    essai.follow()
                }
            }))
            .build()
            .map_err(|_| Erreur::Reseau("Connexion impossible.".into()))
    }
}

/// Un hôte du CDN permis : https, `gog.com` ou un de ses sous-domaines (hors tests).
pub fn hote_cdn_permis(u: &reqwest::Url, https: bool) -> bool {
    if !https {
        return true;
    }
    let hote = u.host_str().unwrap_or("").to_ascii_lowercase();
    u.scheme() == "https" && u.username().is_empty() && u.password().is_none() && (hote == "gog.com" || hote.ends_with(".gog.com"))
}

/// Une erreur réseau dite SANS l'adresse (elle porte un jeton).
fn coupure(e: reqwest::Error) -> Erreur {
    let e = e.without_url();
    Erreur::Reseau(format!("GOG ne répond pas ({}). Ce qui est déjà téléchargé est gardé : ⬇ Installer reprendra où il en était.", if e.is_timeout() { "délai dépassé" } else if e.is_connect() { "connexion impossible" } else { "connexion coupée" }))
}

/// Lit un corps de réponse en entier, plafonné.
async fn corps_plafonne(mut r: reqwest::Response, plafond: usize) -> Resultat<Vec<u8>> {
    let mut v = Vec::new();
    while let Some(m) = r.chunk().await.map_err(coupure)? {
        if v.len() + m.len() > plafond {
            return Err(Erreur::Serveur("GOG a envoyé une réponse bien plus grosse que prévu : elle est ignorée.".into()));
        }
        v.extend_from_slice(&m);
    }
    Ok(v)
}

/// La fiche publique du jeu avec ses téléchargements et ses DLC (sans jeton : relevé du 26/11).
pub async fn lire_fiche(reseau: &ReseauGog, id: &str) -> Resultat<Value> {
    if !crate::gratuits::numero_gog(id) {
        return Err(Erreur::Refus("Ce n'est pas un numéro de jeu GOG.".into()));
    }
    let r = reseau.client_api()?.get(format!("{}/products/{id}?expand=downloads,expanded_dlcs", reseau.api)).send().await.map_err(coupure)?;
    match r.status().as_u16() {
        200 => serde_json::from_slice(&corps_plafonne(r, PLAFOND_FICHE).await?).map_err(|_| Erreur::Serveur("GOG a répondu une fiche illisible.".into())),
        404 => Err(Erreur::Introuvable("GOG ne connaît pas ce jeu.".into())),
        429 => Err(Erreur::Reseau("GOG demande d'attendre un peu avant de réessayer.".into())),
        c => Err(Erreur::Serveur(format!("GOG a répondu une erreur (code {c})."))),
    }
}

/// Les deux liens SIGNÉS d'un fichier (des secrets : `Debug` caché, jamais journalisés).
struct LiensSignes {
    fichier: String,
    controle: String,
}

impl std::fmt::Debug for LiensSignes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LiensSignes(<caché>)")
    }
}

/// Demande les liens signés d'un fichier (jeton Bearer vers `api.gog.com` seulement ; un 401 fait renouveler le jeton
/// UNE fois). Redemandé à chaque départ et à chaque reprise.
async fn liens_signes<J: SourceJeton>(reseau: &ReseauGog, api: &reqwest::Client, jeton: &J, f: &FichierInstalleur) -> Resultat<LiensSignes> {
    if !lien_api_sur(&f.lien, &reseau.api) {
        return Err(Erreur::Refus("GOG a donné un lien de téléchargement inattendu : rien n'est téléchargé.".into()));
    }
    let mut renouveler = false;
    loop {
        let acces = jeton.jeton(renouveler).await?;
        let r = api.get(&f.lien).bearer_auth(&acces).send().await.map_err(coupure)?;
        drop(acces);
        match r.status().as_u16() {
            200 => {
                let v: Value = serde_json::from_slice(&corps_plafonne(r, 64 * 1024).await?).map_err(|_| Erreur::Serveur("GOG a répondu un lien illisible.".into()))?;
                let lien = |k: &str| {
                    v[k].as_str().filter(|l| reqwest::Url::parse(l).is_ok_and(|u| hote_cdn_permis(&u, reseau.https) && (reseau.https || u.scheme() == "http"))).map(str::to_string)
                };
                return match (lien("downlink"), lien("checksum")) {
                    (Some(fichier), Some(controle)) => Ok(LiensSignes { fichier, controle }),
                    // Sans XML de contrôle, rien ne prouverait le fichier : refus (jamais un téléchargement non vérifié).
                    (Some(_), None) => Err(Erreur::Serveur("GOG ne permet pas de vérifier ce fichier : Frogtend ne le télécharge pas sans pouvoir le vérifier.".into())),
                    _ => Err(Erreur::Serveur("GOG a répondu un lien illisible.".into())),
                };
            }
            401 if !renouveler => renouveler = true,
            401 => return Err(Erreur::Refus(crate::gog_galaxy::MOTIF_RECONNECTER_GOG.into())),
            403 | 404 => return Err(Erreur::Refus("GOG refuse de donner cet installeur à ton compte : le jeu est-il bien sur ce compte GOG ?".into())),
            429 => return Err(Erreur::Reseau("GOG demande d'attendre un peu avant de réessayer.".into())),
            c => return Err(Erreur::Serveur(format!("GOG a répondu une erreur (code {c})."))),
        }
    }
}

/// Le XML de contrôle (CDN, sans jeton), plafonné, lu et vérifié.
async fn controle_distant(cdn: &reqwest::Client, lien: &str) -> Resultat<Controle> {
    let r = cdn.get(lien).send().await.map_err(coupure)?;
    if r.status().as_u16() != 200 {
        return Err(Erreur::Serveur(format!("GOG ne permet pas de vérifier ce fichier pour l'instant (code {}) : réessaie plus tard.", r.status().as_u16())));
    }
    let octets = corps_plafonne(r, PLAFOND_XML).await?;
    lire_controle(&String::from_utf8_lossy(&octets))
}

// ───────────────────────────── Le téléchargement ─────────────────────────────

/// La note d'un `.part` (`<nom>.part.json`) : le `.part` n'est repris que pour CE fichier de CET installeur, avec le
/// même MD5 et la même taille dans le XML ; `verifies` : les morceaux déjà vérifiés (revérifiés à la reprise).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NotePart {
    pub installeur: String,
    pub version: String,
    pub fichier: String,
    pub md5: String,
    pub total: u64,
    pub verifies: usize,
}

impl NotePart {
    fn meme_fichier(&self, autre: &NotePart) -> bool {
        self.installeur == autre.installeur && self.version == autre.version && self.fichier == autre.fichier && self.md5 == autre.md5 && self.total == autre.total
    }
}

/// Un fichier reçu et vérifié.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Recu {
    pub nom: String,
    pub taille: u64,
    pub md5: String,
}

/// Comment s'est fini un téléchargement.
#[derive(Debug, PartialEq)]
pub enum Issue<T> {
    Fini(T),
    /// Arrêté à la demande : le `.part` et sa note sont gardés.
    Pause,
}

/// Ce qui suit le téléchargement : l'arrêt demandé, la progression (`étape, faits, total`), les octets déjà comptés
/// avant ce fichier et le total annoncé.
pub struct Avancement<'a> {
    pub arret: &'a AtomicBool,
    pub progres: &'a (dyn Fn(&'static str, u64, u64) + Sync),
    pub total: u64,
}

fn hex(o: &[u8]) -> String {
    o.iter().map(|b| format!("{b:02x}")).collect()
}

fn avec_suffixe(p: &Path, suffixe: &str) -> PathBuf {
    let mut s = p.as_os_str().to_owned();
    s.push(suffixe);
    PathBuf::from(s)
}

fn ecrire_note(chemin: &Path, n: &NotePart) -> Resultat<()> {
    std::fs::write(chemin, serde_json::to_vec(n).map_err(|_| Erreur::Disque("Note de téléchargement impossible à écrire.".into()))?)?;
    Ok(())
}

fn lire_note(chemin: &Path) -> Option<NotePart> {
    let m = std::fs::metadata(chemin).ok()?;
    if m.len() > 64 * 1024 {
        return None;
    }
    serde_json::from_slice(&std::fs::read(chemin).ok()?).ok()
}

/// Coupe (ou crée) un fichier à `n` octets.
fn tronquer(chemin: &Path, n: u64) -> Resultat<()> {
    let f = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(chemin)?;
    f.set_len(n)?;
    Ok(())
}

/// Relit les `jusqu_a` premiers morceaux d'un fichier et rend combien sont BONS de suite depuis le début (`None` :
/// arrêt demandé). Un fichier plus court s'arrête à son dernier morceau complet.
fn verifier_morceaux(chemin: &Path, morceaux: &[Morceau], jusqu_a: usize, av: &Avancement<'_>, deja: u64) -> Resultat<Option<usize>> {
    use md5::{Digest, Md5};
    use std::io::Read;
    let longueur = std::fs::metadata(chemin)?.len();
    let mut f = std::io::BufReader::with_capacity(4 * 1024 * 1024, std::fs::File::open(chemin)?);
    let mut tampon = vec![0u8; 1024 * 1024];
    for (k, m) in morceaux.iter().take(jusqu_a).enumerate() {
        if av.arret.load(Ordering::Relaxed) {
            return Ok(None);
        }
        if m.fin >= longueur {
            return Ok(Some(k));
        }
        let mut h = Md5::new();
        let mut reste = m.fin - m.debut + 1;
        while reste > 0 {
            let n = (reste as usize).min(tampon.len());
            f.read_exact(&mut tampon[..n])?;
            h.update(&tampon[..n]);
            reste -= n as u64;
        }
        if hex(&h.finalize()) != m.md5 {
            return Ok(Some(k));
        }
        (av.progres)("controle", deja + m.fin + 1, av.total);
    }
    Ok(Some(jusqu_a.min(morceaux.len())))
}

/// `Content-Range: bytes a-b/t` → (a, b, t).
pub fn lire_content_range(v: &str) -> Option<(u64, u64, u64)> {
    let r = v.trim().strip_prefix("bytes ")?;
    let (plage, total) = r.split_once('/')?;
    let (a, b) = plage.split_once('-')?;
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?, total.trim().parse().ok()?))
}

/// Ce qu'une requête au CDN a donné.
enum Requete {
    Flux(reqwest::Response, u64),
    /// Coupure ou lien à redemander.
    Rate(Erreur),
}

/// Demande le fichier à partir de `debut` et contrôle la réponse (206 et sa plage exacte ; 200 = depuis le début ;
/// ETag = le MD5 du XML s'il ressemble à un MD5).
async fn demander(cdn: &reqwest::Client, lien: &str, debut: u64, c: &Controle) -> Resultat<Requete> {
    let r = match cdn.get(lien).header(reqwest::header::RANGE, format!("bytes={debut}-")).send().await {
        Ok(r) => r,
        Err(e) => return Ok(Requete::Rate(coupure(e))),
    };
    let st = r.status().as_u16();
    let en_tete = |n: reqwest::header::HeaderName| r.headers().get(n).and_then(|v| v.to_str().ok()).map(str::to_string);
    if let Some(etag) = en_tete(reqwest::header::ETAG).and_then(|e| md5_sur(e.trim_start_matches("W/").trim_matches('"'))) {
        if (st == 200 || st == 206) && etag != c.md5 {
            return Err(Erreur::Serveur("GOG envoie un autre fichier que celui annoncé : rien n'est gardé de cet envoi. Réessaie plus tard.".into()));
        }
    }
    match st {
        206 => match en_tete(reqwest::header::CONTENT_RANGE).as_deref().and_then(lire_content_range) {
            Some((a, b, t)) if a == debut && t == c.total && b == c.total - 1 => Ok(Requete::Flux(r, debut)),
            _ => Err(Erreur::Serveur("GOG envoie une autre partie du fichier que celle demandée : réessaie plus tard.".into())),
        },
        200 => {
            if en_tete(reqwest::header::CONTENT_LENGTH).and_then(|l| l.parse::<u64>().ok()).is_some_and(|l| l != c.total) {
                return Err(Erreur::Serveur("GOG envoie un fichier d'une autre taille que celle annoncée : réessaie plus tard.".into()));
            }
            Ok(Requete::Flux(r, 0))
        }
        // Le lien a expiré ou a été refusé : un nouveau lien est demandé.
        401 | 403 | 404 | 410 => Ok(Requete::Rate(Erreur::Reseau(format!("Le lien de GOG ne vaut plus (code {st})")))),
        416 => Err(Erreur::Serveur("GOG ne sert plus la suite de ce fichier : réessaie plus tard.".into())),
        c => Ok(Requete::Rate(Erreur::Serveur(format!("GOG a répondu une erreur (code {c})")))),
    }
}

/// Télécharge UN fichier d'un installeur dans `dossier` (voir le module) ; `deja` : octets de l'installation déjà
/// comptés avant lui. Rend le fichier reçu et VÉRIFIÉ (tous ses morceaux), ou `Pause`.
#[allow(clippy::too_many_arguments)]
async fn telecharger_un<J: SourceJeton>(
    reseau: &ReseauGog,
    api: &reqwest::Client,
    cdn: &reqwest::Client,
    jeton: &J,
    inst: &InstalleurGog,
    f: &FichierInstalleur,
    dossier: &Path,
    av: &Avancement<'_>,
    deja: u64,
) -> Resultat<Issue<Recu>> {
    use md5::{Digest, Md5};
    use std::io::Write;
    if av.arret.load(Ordering::Relaxed) {
        return Ok(Issue::Pause);
    }
    let mut liens = liens_signes(reseau, api, jeton, f).await?;
    let c = controle_distant(cdn, &liens.controle).await?;
    std::fs::create_dir_all(dossier)?;
    let final_ = dossier.join(&c.nom);
    let part = avec_suffixe(&final_, ".part");
    let note = avec_suffixe(&final_, ".part.json");
    let mut voulue = NotePart { installeur: inst.id.clone(), version: inst.version.clone(), fichier: f.id.clone(), md5: c.md5.clone(), total: c.total, verifies: 0 };
    let n = c.morceaux.len();
    // Un fichier déjà complet (installeur gardé après un refus de Windows) : repris tel quel si sa note de
    // vérification est celle de CE fichier, à la même taille et à la même date (jamais 115 Go relus à chaque essai) ;
    // sinon revérifié en entier (la note d'abord, puis le renommage : un arrêt entre les deux ne perd rien).
    let verifie = avec_suffixe(&final_, SUFFIXE_VERIFIE);
    if final_.is_file() {
        let m = std::fs::metadata(&final_)?;
        let lu = std::fs::read(&verifie).ok().filter(|o| o.len() <= 64 * 1024).and_then(|o| serde_json::from_slice::<Verifie>(&o).ok());
        if lu.is_some_and(|v| v.note.meme_fichier(&voulue) && m.len() == c.total && v.modifie == date_de(&m)) {
            (av.progres)("telechargement", deja + c.total, av.total);
            return Ok(Issue::Fini(Recu { nom: c.nom, taille: c.total, md5: c.md5 }));
        }
        ecrire_note(&note, &NotePart { verifies: n, ..voulue.clone() })?;
        let _ = std::fs::remove_file(&verifie);
        std::fs::rename(&final_, &part)?;
    }
    // Un .part n'est repris que pour CE fichier (sa note) ; sinon il est écarté (preuve : sa note dit un autre fichier).
    let mut k = 0usize;
    if part.is_file() {
        match lire_note(&note) {
            Some(lue) if lue.meme_fichier(&voulue) => match verifier_morceaux(&part, &c.morceaux, lue.verifies.min(n), av, deja)? {
                None => return Ok(Issue::Pause),
                Some(bons) => k = bons,
            },
            _ => std::fs::remove_file(&part)?,
        }
    }
    let frontiere = |k: usize| c.morceaux.get(k).map_or(c.total, |m| m.debut);
    tronquer(&part, frontiere(k))?;
    voulue.verifies = k;
    ecrire_note(&note, &voulue)?;
    let mut faux: std::collections::HashMap<usize, u32> = std::collections::HashMap::new();
    let mut coupures = 0u32;
    let mut nouveau_lien = false;
    while k < n {
        if av.arret.load(Ordering::Relaxed) {
            return Ok(Issue::Pause);
        }
        if nouveau_lien {
            // Lien redemandé à chaque reprise (≈ 24 h de vie).
            liens = liens_signes(reseau, api, jeton, f).await?;
            nouveau_lien = false;
        }
        let (mut r, depart) = match demander(cdn, &liens.fichier, frontiere(k), &c).await? {
            Requete::Flux(r, d) => (r, d),
            Requete::Rate(e) => {
                coupures += 1;
                if coupures > COUPURES_MAX {
                    return Err(e);
                }
                nouveau_lien = true;
                continue;
            }
        };
        if depart == 0 && k > 0 {
            // Le CDN renvoie tout depuis le début : on réécrit depuis le début.
            k = 0;
            tronquer(&part, 0)?;
            voulue.verifies = 0;
            ecrire_note(&note, &voulue)?;
        }
        let mut sortie = std::fs::OpenOptions::new().append(true).open(&part)?;
        let mut pos = frontiere(k);
        let mut h = Md5::new();
        let mut dernier = std::time::Instant::now();
        let mut redemander = false;
        while !redemander {
            if av.arret.load(Ordering::Relaxed) {
                drop(sortie);
                tronquer(&part, frontiere(k))?;
                return Ok(Issue::Pause);
            }
            let morceau = match r.chunk().await {
                Ok(Some(m)) => m,
                Ok(None) | Err(_) => {
                    // Fin du flux avant la fin du fichier, ou coupure : on reprend à la dernière frontière vérifiée.
                    if k >= n {
                        break;
                    }
                    drop(sortie);
                    tronquer(&part, frontiere(k))?;
                    coupures += 1;
                    if coupures > COUPURES_MAX {
                        return Err(Erreur::Reseau(format!(
                            "La connexion avec GOG se coupe sans cesse. Ce qui est déjà téléchargé ({}) est gardé : ⬇ Installer reprendra où il en était.",
                            crate::erreurs::taille_lisible(deja + frontiere(k))
                        )));
                    }
                    nouveau_lien = true;
                    break;
                }
            };
            let mut reste = &morceau[..];
            while !reste.is_empty() {
                if k >= n {
                    drop(sortie);
                    tronquer(&part, c.total)?;
                    return Err(Erreur::Serveur("GOG envoie plus de données que prévu pour ce fichier : la suite est ignorée, réessaie.".into()));
                }
                let m = &c.morceaux[k];
                let pris = ((m.fin + 1 - pos) as usize).min(reste.len());
                sortie.write_all(&reste[..pris])?;
                h.update(&reste[..pris]);
                pos += pris as u64;
                reste = &reste[pris..];
                if pos == m.fin + 1 {
                    if hex(&h.finalize_reset()) == m.md5 {
                        k += 1;
                        coupures = 0;
                        voulue.verifies = k;
                        ecrire_note(&note, &voulue)?;
                    } else {
                        // Un morceau faux : coupé au début du morceau et redemandé UNE fois.
                        drop(std::mem::replace(&mut sortie, std::fs::OpenOptions::new().append(true).open(&part)?));
                        tronquer(&part, m.debut)?;
                        let essais = faux.entry(k).or_insert(0);
                        *essais += 1;
                        if *essais > 1 {
                            return Err(Erreur::Serveur(format!(
                                "« {} » arrive abîmé de GOG deux fois de suite : il a été écarté à partir de l'endroit abîmé. Réessaie plus tard ; si ça recommence, installe ce jeu avec GOG Galaxy.",
                                c.nom
                            )));
                        }
                        redemander = true;
                        break;
                    }
                }
            }
            if dernier.elapsed() >= std::time::Duration::from_millis(250) {
                (av.progres)("telechargement", deja + pos, av.total);
                dernier = std::time::Instant::now();
            }
        }
    }
    // Vérifier le résultat, pas le retour : la taille exacte sur le disque, tous les morceaux vérifiés.
    let sur_disque = std::fs::metadata(&part)?.len();
    if sur_disque != c.total {
        return Err(Erreur::Reseau(format!("« {} » est incomplet : ⬇ Installer reprendra où il en était.", c.nom)));
    }
    std::fs::rename(&part, &final_)?;
    let _ = std::fs::remove_file(&note);
    // La note de vérification du fichier fini (taille et date) : une reprise ne le relit pas.
    if let Ok(m) = std::fs::metadata(&final_) {
        let v = Verifie { note: NotePart { verifies: n, ..voulue.clone() }, modifie: date_de(&m) };
        let _ = std::fs::write(&verifie, serde_json::to_vec(&v).unwrap_or_default());
    }
    (av.progres)("telechargement", deja + c.total, av.total);
    Ok(Issue::Fini(Recu { nom: c.nom, taille: c.total, md5: c.md5 }))
}

/// Le suffixe de la note de vérification d'un fichier fini (`<nom>.verifie.json`).
const SUFFIXE_VERIFIE: &str = ".verifie.json";

/// La note de vérification d'un fichier fini : le fichier (comme la note d'un `.part`) et sa date de modification.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct Verifie {
    note: NotePart,
    modifie: u64,
}

/// La date de modification d'un fichier, en nanosecondes depuis 1970 (0 : inconnue).
fn date_de(m: &std::fs::Metadata) -> u64 {
    m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_nanos() as u64)
}

/// La note d'un installeur reçu (`frogtend-installeur.json`) : les seuls fichiers effacés après son installation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NoteInstalleur {
    pub element: String,
    pub installeur: String,
    pub version: String,
    pub fichiers: Vec<Recu>,
}

/// Télécharge tous les fichiers d'un installeur dans `dossier`, puis écrit sa note.
#[allow(clippy::too_many_arguments)]
async fn telecharger_installeur<J: SourceJeton>(
    reseau: &ReseauGog,
    api: &reqwest::Client,
    cdn: &reqwest::Client,
    jeton: &J,
    e: &Element,
    dossier: &Path,
    av: &Avancement<'_>,
    deja: &mut u64,
) -> Resultat<Issue<Vec<Recu>>> {
    let mut recus = Vec::new();
    for f in &e.installeur.fichiers {
        match telecharger_un(reseau, api, cdn, jeton, &e.installeur, f, dossier, av, *deja).await? {
            Issue::Pause => return Ok(Issue::Pause),
            Issue::Fini(r) => {
                *deja += r.taille;
                recus.push(r);
            }
        }
    }
    let note = NoteInstalleur { element: e.id.clone(), installeur: e.installeur.id.clone(), version: e.installeur.version.clone(), fichiers: recus.clone() };
    std::fs::write(dossier.join(NOTE_INSTALLEUR), serde_json::to_vec_pretty(&note).unwrap_or_default())?;
    Ok(Issue::Fini(recus))
}

/// Efface les fichiers d'un installeur LISTÉS dans sa note, à la taille notée (et rien d'autre), avec leur note de
/// vérification, puis la note, la note « en cours » et le dossier s'il est vide. Le journal d'Inno n'est pas touché
/// ici (`garder_journal_inno` le retire une fois copié). Rend le nombre de fichiers effacés.
pub fn effacer_installeur(dossier: &Path) -> usize {
    let Some(note) = std::fs::read(dossier.join(NOTE_INSTALLEUR)).ok().and_then(|o| serde_json::from_slice::<NoteInstalleur>(&o).ok()) else { return 0 };
    let mut n = 0;
    for f in &note.fichiers {
        if crate::jeux_pc::nom_de_fichier_sur(&f.nom).is_err() {
            continue;
        }
        let p = dossier.join(&f.nom);
        if std::fs::metadata(&p).is_ok_and(|m| m.is_file() && m.len() == f.taille) && std::fs::remove_file(&p).is_ok() {
            n += 1;
            let _ = std::fs::remove_file(avec_suffixe(&p, SUFFIXE_VERIFIE));
        }
    }
    let _ = std::fs::remove_file(dossier.join(NOTE_INSTALLEUR));
    let _ = std::fs::remove_file(dossier.join(NOTE_EN_COURS));
    let _ = std::fs::remove_dir(dossier); // seulement s'il est vide
    n
}

// ───────────────────────────── Le PC : signature, installeur, registre ─────────────────────────────

/// Comment l'installeur s'est fini.
#[derive(Clone, Debug, PartialEq)]
pub enum IssueInno {
    /// Son code de sortie (0 : fini, d'après Inno ; Frogtend vérifie quand même).
    Code(u32),
    /// La personne a répondu Non à Windows (ShellExecuteExW : ERROR_CANCELLED, 1223).
    Refuse,
    /// Windows n'a pas pu le lancer (le code de Windows, sans chemin).
    Impossible(String),
}

/// Ce que l'installation demande au PC : abstrait pour que les tests ne lancent JAMAIS un vrai installeur ni
/// PowerShell, et ne lisent jamais le vrai registre.
pub trait PcGog: Send + Sync + 'static {
    /// Le `.exe` est-il signé par GOG, signature valide ?
    fn signe_par_gog(&self, exe: &Path) -> Resultat<bool>;
    /// Lance l'installeur avec les droits que Windows demande, et attend sa fin.
    fn executer_eleve(&self, exe: &Path, parametres: &str, travail: &Path) -> IssueInno;
    /// L'entrée `GOG.com\Games\<id>` du registre (HKLM).
    fn entree(&self, id: &str) -> Option<Valeurs>;
}

/// Lit la sortie du petit script de signature : 1re ligne le statut (`Valid`), 2e ligne le sujet du certificat. Vrai
/// seulement pour un statut valide ET un signataire GOG (`O=` ou `CN=` exactement « GOG Sp. z o.o. »). Pur, testé.
pub fn signature_gog_valide(sortie: &str) -> bool {
    let mut lignes = sortie.lines().map(str::trim).filter(|l| !l.is_empty());
    let (Some(statut), Some(sujet)) = (lignes.next(), lignes.next()) else { return false };
    statut == "Valid" && sujet_gog(sujet)
}

/// Le sujet d'un certificat (`CN=GOG Sp. z o.o., O=GOG Sp. z o.o., L=Warsaw, C=PL`) nomme-t-il GOG ?
pub fn sujet_gog(sujet: &str) -> bool {
    // Découpé sur les virgules hors guillemets.
    let mut parties = vec![];
    let (mut courant, mut guillemets) = (String::new(), false);
    for ch in sujet.chars() {
        match ch {
            '"' => {
                guillemets = !guillemets;
                courant.push(ch);
            }
            ',' if !guillemets => parties.push(std::mem::take(&mut courant)),
            _ => courant.push(ch),
        }
    }
    parties.push(courant);
    parties.iter().any(|p| {
        let Some((k, v)) = p.trim().split_once('=') else { return false };
        matches!(k.trim().to_ascii_uppercase().as_str(), "O" | "CN") && nom_normalise(v.trim().trim_matches('"')) == nom_normalise(SIGNATAIRE_GOG)
    })
}

/// Un nom de signataire comparable : minuscules, espaces réduits à un seul, point final retiré. Le VRAI certificat de
/// GOG (relevé le 28/11 sur l'installeur d'Another World 20th) dit `CN=GOG  sp. z o.o` (deux espaces, « sp » en
/// minuscules, sans point final) ; le statut « Valid » reste exigé tel quel.
fn nom_normalise(nom: &str) -> String {
    nom.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase().trim_end_matches('.').to_string()
}

/// Les paramètres d'Inno Setup (voir la documentation citée en tête du module). `dest` : chemin complet, sans `\`
/// final ; jamais de guillemet dans un chemin (Windows n'en permet pas : refus sinon).
pub fn parametres_inno(dest: &Path, journal: &Path) -> Resultat<String> {
    let d = dest.to_string_lossy().trim_end_matches(['\\', '/']).to_string();
    let j = journal.to_string_lossy().to_string();
    if d.contains('"') || j.contains('"') || !dest.is_absolute() || d.is_empty() {
        return Err(Erreur::Refus("Ce dossier ne peut pas recevoir le jeu : choisis un autre emplacement des jeux.".into()));
    }
    Ok(format!("/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP- /NOCANCEL /DIR=\"{d}\" /LOG=\"{j}\""))
}

/// Le sens d'un code de sortie d'Inno Setup, en mots simples (documentation citée en tête du module).
pub fn sens_code_inno(code: u32) -> &'static str {
    match code {
        0 => "fini",
        1 => "l'installeur n'a pas pu démarrer",
        2 | 5 => "l'installation a été annulée (si Windows a demandé une autorisation, il faut répondre Oui)",
        3 | 4 => "une erreur grave pendant l'installation",
        6 => "l'installeur a été arrêté de force",
        7 => "l'installeur ne peut pas continuer sur ce PC",
        8 => "l'installeur ne peut pas continuer : Windows doit redémarrer",
        _ => "un code inconnu",
    }
}

/// Le vrai PC : PowerShell pour la signature, ShellExecuteExW pour l'installeur, le registre pour l'entrée.
/// `parent` : la fenêtre principale de Frogtend (son HWND en nombre ; 0 : aucune), donnée à ShellExecuteExW pour que
/// l'invite de l'UAC passe au premier plan, rattachée à Frogtend (au canapé surtout).
#[derive(Clone, Copy, Default)]
pub struct PcWindows {
    pub parent: isize,
}

/// Le script de signature : le chemin passe par une variable d'environnement (jamais dans la ligne de commande).
const SCRIPT_SIGNATURE: &str = "[Console]::OutputEncoding = [Text.Encoding]::UTF8; $s = Get-AuthenticodeSignature -LiteralPath $env:FROGTEND_SIGNE; Write-Output ([string]$s.Status); Write-Output ([string]$s.SignerCertificate.Subject)";

impl PcGog for PcWindows {
    fn signe_par_gog(&self, exe: &Path) -> Resultat<bool> {
        let s = crate::lancement::outil_sans_fenetre("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", SCRIPT_SIGNATURE])
            .env("FROGTEND_SIGNE", exe)
            .output()
            .map_err(|_| Erreur::Disque("Frogtend n'a pas pu vérifier la signature de l'installeur.".into()))?;
        Ok(s.status.success() && signature_gog_valide(&String::from_utf8_lossy(&s.stdout)))
    }

    #[cfg(windows)]
    fn executer_eleve(&self, exe: &Path, parametres: &str, travail: &Path) -> IssueInno {
        use std::os::windows::ffi::OsStrExt;
        use windows::core::{HRESULT, PCWSTR};
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject, INFINITE};
        use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE};
        use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
        // La documentation de ShellExecuteEx demande COM initialisé (STA) sur le fil appelant ; rendu à la fin.
        let com = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) }.is_ok();
        struct Com(bool);
        impl Drop for Com {
            fn drop(&mut self) {
                if self.0 {
                    unsafe { CoUninitialize() };
                }
            }
        }
        let _com = Com(com);
        let large = |s: &std::ffi::OsStr| s.encode_wide().chain([0]).collect::<Vec<u16>>();
        let (f, p, d) = (large(exe.as_os_str()), large(std::ffi::OsStr::new(parametres)), large(travail.as_os_str()));
        let runas: Vec<u16> = "runas".encode_utf16().chain([0]).collect();
        let lancer = |verbe: PCWSTR| -> windows::core::Result<windows::Win32::Foundation::HANDLE> {
            let mut info = SHELLEXECUTEINFOW {
                cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
                // La fenêtre de Frogtend (nulle si aucune) : l'invite de l'UAC passe devant, rattachée à elle.
                hwnd: windows::Win32::Foundation::HWND(self.parent as *mut core::ffi::c_void),
                // NO_UI : jamais une boîte d'erreur native de Windows (CLAUDE.md § 5 ; elle demanderait la souris au
                // canapé) ; l'échec revient en code (1223 : refus de l'UAC).
                fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_FLAG_NO_UI,
                lpVerb: verbe,
                lpFile: PCWSTR(f.as_ptr()),
                lpParameters: PCWSTR(p.as_ptr()),
                lpDirectory: PCWSTR(d.as_ptr()),
                nShow: SW_SHOWNORMAL.0,
                ..Default::default()
            };
            unsafe { ShellExecuteExW(&mut info) }?;
            Ok(info.hProcess)
        };
        // TOUJOURS « runas » : les installeurs de GOG sont `asInvoker` (relevé le 28/11 : Another World 20th, Inno
        // 5.6.2) et s'élèvent EUX-MÊMES ; leur invite n'était alors pas rattachée à Frogtend, et un Non sortait en
        // code 2 au lieu de 1223. Élevé par Frogtend, l'installeur n'en redemande pas. 1223 (ERROR_CANCELLED) : Non.
        let refus = HRESULT::from_win32(1223);
        let h = lancer(PCWSTR(runas.as_ptr()));
        let h = match h {
            Ok(h) if !h.is_invalid() && !h.0.is_null() => h,
            Ok(_) => return IssueInno::Impossible("pas de processus".into()),
            Err(e) if e.code() == refus => return IssueInno::Refuse,
            Err(e) => return IssueInno::Impossible(format!("code 0x{:08X}", e.code().0)),
        };
        let mut code = 0u32;
        let lu = unsafe {
            WaitForSingleObject(h, INFINITE);
            let lu = GetExitCodeProcess(h, &mut code);
            let _ = CloseHandle(h);
            lu
        };
        match lu {
            Ok(()) => IssueInno::Code(code),
            Err(_) => IssueInno::Impossible("code de sortie illisible".into()),
        }
    }

    #[cfg(not(windows))]
    fn executer_eleve(&self, _exe: &Path, _parametres: &str, _travail: &Path) -> IssueInno {
        IssueInno::Impossible("Windows seulement".into())
    }

    fn entree(&self, id: &str) -> Option<Valeurs> {
        crate::boutiques_pc::entree_gog(id)
    }
}

/// Le jeu est-il VRAIMENT installé dans `dest` ? (1) l'entrée du registre `GOG.com\Games\<id>` pointe sur `dest` ;
/// (2) la lecture des jeux GOG installés la garde (pas prise pour un DLC) ; (3) il se lance comme en C4c-1 : la tâche de
/// jeu de `goggame-<id>.info`, sinon le programme noté au registre (`programme_gog`, la même voie de secours que
/// `partie.rs`). Rend la voie de lancement (« game », « principale » ou « registre »), ou la raison d'un échec, en mots
/// (jamais de chemin).
pub fn verifier_jeu(pc: &dyn PcGog, dest: &Path, id: &str) -> Result<&'static str, &'static str> {
    let v = pc.entree(id).ok_or("pas d'entrée GOG au registre")?;
    let chemin = v.get("path").map(|p| PathBuf::from(p.trim())).ok_or("entrée du registre sans dossier")?;
    if crate::boutiques::cle_chemin(&chemin) != crate::boutiques::cle_chemin(dest) {
        return Err("le registre nomme un autre dossier");
    }
    match crate::boutiques_pc::lire_gog(Some(vec![(id.to_string(), v.clone())])) {
        Some(l) if !l.is_empty() => {}
        _ => return Err("la lecture des jeux GOG ne le garde pas"),
    }
    match crate::boutiques_pc::tache_jeu_gog(dest, id) {
        Ok(t) => Ok(t.categorie.mot()),
        Err(m) if crate::boutiques_pc::programme_gog(&v).is_some() => {
            let _ = m;
            Ok("registre")
        }
        Err(m) => Err(m.en_mots()),
    }
}

/// Un DLC est-il installé ? Son `goggame-<dlc>.info` dans le dossier du jeu, ou son entrée au registre (forme NON
/// relevée en vrai : à vérifier au premier essai).
pub fn verifier_dlc(pc: &dyn PcGog, dest: &Path, dlc: &str) -> bool {
    dest.join(format!("goggame-{dlc}.info")).is_file() || pc.entree(dlc).is_some()
}

/// La marque « installé par Frogtend » (dans `<dest>\.frogtend-gog\`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Marque {
    #[serde(rename = "gameId")]
    pub game_id: String,
    pub installeur: String,
    pub version: String,
    pub md5: Vec<String>,
    pub date: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dlcs: Vec<String>,
}

// ───────────────────────────── L'installation entière ─────────────────────────────

/// Ce qu'il faut installer, décidé AVANT (accord de la personne).
#[derive(Clone, Debug)]
pub struct Plan {
    pub dest: PathBuf,
    /// Le dossier des installeurs (`<emplacement>\Installeurs GOG (Frogtend)`), un sous-dossier par numéro.
    pub installeurs: PathBuf,
    pub jeu: Element,
    pub dlcs: Vec<Element>,
}

impl Plan {
    pub fn total_annonce(&self) -> u64 {
        std::iter::once(&self.jeu).chain(&self.dlcs).map(|e| e.installeur.total_annonce).sum()
    }
}

/// Ce que l'installation a donné.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct Bilan {
    /// Arrêtée pendant le téléchargement (rien n'est installé ; ce qui est reçu est gardé).
    pub pause: bool,
    pub titre: String,
    pub dlcs_installes: Vec<String>,
    /// (titre, raison) des DLC qui n'ont pas pu s'installer (le jeu, lui, est installé).
    pub dlcs_echoues: Vec<(String, String)>,
    /// Les octets des installeurs GARDÉS (DLC en échec) : rien ne s'efface sans preuve, mais la place est dite.
    pub installeurs_gardes: u64,
    /// Frogtend n'a pas pu écrire dans le dossier créé par l'installeur (droits) : ses parties et ses mods pourraient
    /// ne pas s'y poser (relecture de l'expert, 27/11 : dit, jamais tu).
    pub dossier_protege: bool,
    /// Le jeu avait été installé pendant que Frogtend était fermé : il a été vérifié et rendu jouable (sans DLC).
    pub rattrape: bool,
}

/// La note écrite JUSTE AVANT de lancer l'installeur du jeu (`<installeurs>\<id>\frogtend-en-cours.json`) : si
/// Frogtend est fermé pendant l'installeur, le prochain « ⬇ Installer » sait finir le travail (vérifier, marquer,
/// effacer l'installeur, reposer les parties).
pub const NOTE_EN_COURS: &str = "frogtend-en-cours.json";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct EnCours {
    dest: String,
}

/// L'exécution d'un installeur sur le PC, en tâche bloquante.
async fn executer<P: PcGog + Clone>(pc: &P, exe: PathBuf, parametres: String, travail: PathBuf) -> IssueInno {
    let pc = pc.clone();
    tauri::async_runtime::spawn_blocking(move || pc.executer_eleve(&exe, &parametres, &travail)).await.unwrap_or_else(|_| IssueInno::Impossible("arrêt brutal".into()))
}

/// La signature d'un installeur, en tâche bloquante.
async fn signe<P: PcGog + Clone>(pc: &P, exe: PathBuf) -> Resultat<bool> {
    let pc = pc.clone();
    tauri::async_runtime::spawn_blocking(move || pc.signe_par_gog(&exe)).await.map_err(|_| Erreur::Disque("La vérification de la signature s'est arrêtée brutalement.".into()))?
}

/// Le `.exe` d'un installeur reçu (un seul).
fn exe_de(recus: &[Recu]) -> Option<&Recu> {
    let exes: Vec<&Recu> = recus.iter().filter(|r| Path::new(&r.nom).extension().is_some_and(|x| x.eq_ignore_ascii_case("exe"))).collect();
    (exes.len() == 1).then(|| exes[0])
}

/// Le dossier de destination est-il vide (ou absent) ?
fn vide_ou_absent(d: &Path) -> bool {
    !d.exists() || std::fs::read_dir(d).is_ok_and(|mut l| l.next().is_none())
}

/// Le jeu a-t-il été installé par Frogtend pendant qu'il était FERMÉ ? (La note « en cours » de son installeur nomme
/// ce dossier, et le jeu s'y vérifie.)
pub fn rattrapable(pc: &dyn PcGog, plan: &Plan) -> bool {
    let note = plan.installeurs.join(&plan.jeu.id).join(NOTE_EN_COURS);
    let Some(n) = std::fs::read(&note).ok().filter(|o| o.len() <= 64 * 1024).and_then(|o| serde_json::from_slice::<EnCours>(&o).ok()) else { return false };
    crate::boutiques::cle_chemin(Path::new(&n.dest)) == crate::boutiques::cle_chemin(&plan.dest) && verifier_jeu(pc, &plan.dest, &plan.jeu.id).is_ok()
}

/// Le journal d'Inno Setup (`/LOG`) dit-il « Installation process succeeded » ? Lu jusqu'à 32 Mo ; absent ou illisible :
/// non. Ne suffit JAMAIS seul : la vérification du jeu (ou du DLC) suit toujours.
pub fn inno_a_reussi(journal: &Path) -> bool {
    match std::fs::metadata(journal) {
        Ok(m) if m.len() <= 32 * 1024 * 1024 => {}
        _ => return false,
    }
    std::fs::read(journal).map(|o| String::from_utf8_lossy(&o).contains("Installation process succeeded")).unwrap_or(false)
}

/// Le message d'un installeur arrêté avant la fin (ou dont le jeu ne se retrouve pas) : ce qu'il reste à faire, pour
/// ne jamais renvoyer vers un « Installer » qui serait refusé (dossier non vide, jeu au registre).
fn message_inacheve(debut: &str, dest: &Path, au_registre: bool) -> String {
    if au_registre {
        format!("{debut} Une installation incomplète est restée : désinstalle-la depuis Windows (Paramètres › Applications), puis ⬇ Installer pour réessayer (l'installeur est gardé) ; ou installe le jeu avec GOG Galaxy.")
    } else if !vide_ou_absent(dest) {
        format!("{debut} Des fichiers sont restés dans le dossier « {} » : retire ce dossier, puis ⬇ Installer pour réessayer (l'installeur est gardé) ; ou installe le jeu avec GOG Galaxy.", dest.display())
    } else {
        format!("{debut} Rien n'a été effacé ; l'installeur est gardé : ⬇ Installer pour réessayer, ou installe le jeu avec GOG Galaxy.")
    }
}

/// Après un jeu VÉRIFIÉ : la marque « installé par Frogtend » (qui prouve aussi que Frogtend peut écrire dans ce
/// dossier), le journal de l'installeur copié à côté, puis l'installeur effacé (seulement les fichiers de sa note).
/// Rend (la marque, `dossier_protege`).
fn finir_jeu(plan: &Plan, journal: &(dyn Fn(&str) + Sync)) -> (Marque, bool) {
    let id = &plan.jeu.id;
    let dossier_jeu = plan.installeurs.join(id);
    let note = std::fs::read(dossier_jeu.join(NOTE_INSTALLEUR)).ok().and_then(|o| serde_json::from_slice::<NoteInstalleur>(&o).ok());
    let marque = Marque {
        game_id: id.clone(),
        installeur: plan.jeu.installeur.id.clone(),
        version: plan.jeu.installeur.version.clone(),
        md5: note.map(|n| n.fichiers.iter().map(|f| f.md5.clone()).collect()).unwrap_or_default(),
        date: crate::noyau::maintenant(),
        dlcs: vec![],
    };
    let protege = ecrire_marque(&plan.dest, &marque).is_err();
    if protege {
        journal(&format!("installation GOG : jeu {id} : le dossier du jeu refuse l'écriture (marque impossible)"));
    }
    garder_journal_inno(&dossier_jeu.join(JOURNAL_INNO), &plan.dest.join(DOSSIER_MARQUE).join(JOURNAL_INNO));
    let effaces = effacer_installeur(&dossier_jeu);
    journal(&format!("installation GOG : jeu {id} : installé et vérifié ; installeur effacé ({effaces} fichier(s))"));
    (marque, protege)
}

fn ecrire_marque(dest: &Path, m: &Marque) -> Resultat<()> {
    let d = dest.join(DOSSIER_MARQUE);
    std::fs::create_dir_all(&d)?;
    std::fs::write(d.join(FICHIER_MARQUE), serde_json::to_vec_pretty(m).unwrap_or_default())?;
    Ok(())
}

/// Copie le journal d'Inno à côté de la marque ; l'original n'est retiré que si la copie a réussi.
fn garder_journal_inno(de: &Path, vers: &Path) {
    if de.is_file() && std::fs::copy(de, vers).is_ok() {
        let _ = std::fs::remove_file(de);
    }
}

/// Les octets des fichiers listés dans la note d'un installeur encore là.
fn octets_installeur(dossier: &Path) -> u64 {
    std::fs::read(dossier.join(NOTE_INSTALLEUR))
        .ok()
        .and_then(|o| serde_json::from_slice::<NoteInstalleur>(&o).ok())
        .map(|n| n.fichiers.iter().filter(|f| dossier.join(&f.nom).is_file()).map(|f| f.taille).sum())
        .unwrap_or(0)
}

/// Installe le jeu, puis ses DLC cochés (voir le module). `annulable` passe à faux AVANT de lancer l'installeur (plus
/// d'annulation possible ensuite). `journal` : numéros et chiffres seulement.
#[allow(clippy::too_many_arguments)]
pub async fn installer<J: SourceJeton, P: PcGog + Clone>(
    reseau: &ReseauGog,
    jeton: &J,
    pc: &P,
    plan: &Plan,
    arret: &AtomicBool,
    annulable: &AtomicBool,
    progres: &(dyn Fn(&'static str, u64, u64) + Sync),
    journal: &(dyn Fn(&str) + Sync),
) -> Resultat<Bilan> {
    let id = plan.jeu.id.clone();
    if pc.entree(&id).is_some() {
        // Installé par Frogtend pendant qu'il était fermé : le travail est fini (sans les DLC, à refaire plus tard).
        if rattrapable(pc, plan) {
            annulable.store(false, Ordering::SeqCst);
            let (_, protege) = finir_jeu(plan, journal);
            journal(&format!("installation GOG : jeu {id} : installé pendant que Frogtend était fermé, vérifié et rattaché"));
            let gardes = plan.dlcs.iter().map(|e| octets_installeur(&plan.installeurs.join(&e.id))).sum();
            let _ = std::fs::remove_dir(&plan.installeurs);
            return Ok(Bilan { titre: plan.jeu.titre.clone(), rattrape: true, dossier_protege: protege, installeurs_gardes: gardes, ..Default::default() });
        }
        // Jeu déjà présent : refus dans ce lot (la réinstallation viendra plus tard).
        return Err(Erreur::Refus("Ce jeu GOG est déjà installé sur ce PC : Frogtend ne le réinstalle pas.".into()));
    }
    if !vide_ou_absent(&plan.dest) {
        return Err(Erreur::Refus(format!("Le dossier « {} » existe déjà et n'est pas vide : Frogtend n'y installe rien. Range-le ou choisis un autre emplacement des jeux.", plan.dest.display())));
    }
    let (api, cdn) = (reseau.client_api()?, reseau.client_cdn()?);
    let av = Avancement { arret, progres, total: plan.total_annonce() };
    let elements: Vec<&Element> = std::iter::once(&plan.jeu).chain(&plan.dlcs).collect();
    // 1. Tout est téléchargé et vérifié AVANT le premier installeur (l'annulation reste possible jusque-là).
    let mut recus: Vec<Vec<Recu>> = Vec::new();
    let mut deja = 0u64;
    progres("telechargement", 0, av.total);
    for e in &elements {
        match telecharger_installeur(reseau, &api, &cdn, jeton, e, &plan.installeurs.join(&e.id), &av, &mut deja).await? {
            Issue::Pause => {
                journal(&format!("installation GOG : jeu {id} : téléchargement arrêté à la demande ({deja} octets gardés)"));
                return Ok(Bilan { pause: true, titre: plan.jeu.titre.clone(), ..Default::default() });
            }
            Issue::Fini(r) => recus.push(r),
        }
    }
    annulable.store(false, Ordering::SeqCst);
    if arret.load(Ordering::SeqCst) {
        journal(&format!("installation GOG : jeu {id} : arrêté à la demande juste avant l'installation"));
        return Ok(Bilan { pause: true, titre: plan.jeu.titre.clone(), ..Default::default() });
    }
    // Dit tout de suite : l'interface retire « Arrêter » (plus d'annulation à partir d'ici).
    progres("installation", 0, 0);
    journal(&format!("installation GOG : jeu {id} : {} installeur(s) reçu(s) et vérifié(s), {deja} octets", elements.len()));
    // 2. Les signatures du jeu (refus de tout s'il n'est pas signé par GOG).
    let exe_jeu = exe_de(&recus[0]).ok_or_else(|| Erreur::Serveur("L'installeur de GOG n'a pas la forme attendue (un seul programme) : Frogtend ne le lance pas.".into()))?;
    let dossier_jeu = plan.installeurs.join(&plan.jeu.id);
    if !signe(pc, dossier_jeu.join(&exe_jeu.nom)).await? {
        journal(&format!("installation GOG : jeu {id} : installeur NON signé par GOG, pas lancé"));
        return Err(Erreur::Refus("L'installeur téléchargé n'est pas signé par GOG : Frogtend ne le lance pas. Rien n'a été installé.".into()));
    }
    // 3. L'installeur du jeu, après la note « en cours » (Frogtend fermé pendant l'installeur : rattrapé ensuite).
    let journal_inno = dossier_jeu.join(JOURNAL_INNO);
    let parametres = parametres_inno(&plan.dest, &journal_inno)?;
    std::fs::write(dossier_jeu.join(NOTE_EN_COURS), serde_json::to_vec(&EnCours { dest: plan.dest.display().to_string() }).unwrap_or_default())?;
    let issue = executer(pc, dossier_jeu.join(&exe_jeu.nom), parametres.clone(), dossier_jeu.clone()).await;
    let au_registre = || pc.entree(&id).is_some();
    match issue {
        IssueInno::Refuse => {
            let _ = std::fs::remove_file(dossier_jeu.join(NOTE_EN_COURS));
            journal(&format!("installation GOG : jeu {id} : refus de Windows (UAC)"));
            return Err(Erreur::Refus(MESSAGE_UAC.into()));
        }
        IssueInno::Impossible(m) => {
            let _ = std::fs::remove_file(dossier_jeu.join(NOTE_EN_COURS));
            journal(&format!("installation GOG : jeu {id} : installeur pas lancé ({m})"));
            return Err(Erreur::Disque("Windows n'a pas pu lancer l'installeur de GOG. Rien n'a été effacé : l'installeur est gardé.".into()));
        }
        IssueInno::Code(c) if c != 0 && inno_a_reussi(&journal_inno) && verifier_jeu(pc, &plan.dest, &id).is_ok() => {
            // Relevé le 28/11 (Another World 20th, Inno 5.6.2) : « Installation process succeeded », puis l'installeur
            // prévient GOG Galaxy et PLANTE en sortant (0xC000041D). Le jeu est là et vérifié : on ne s'y fie pas au code.
            journal(&format!("installation GOG : jeu {id} : code de sortie {c} après « Installation process succeeded », jeu vérifié"));
        }
        IssueInno::Code(c) if c != 0 => {
            let r = au_registre();
            journal(&format!("installation GOG : jeu {id} : code de sortie {c}, dossier {}, registre {}", if vide_ou_absent(&plan.dest) { "vide" } else { "non vide" }, if r { "oui" } else { "non" }));
            return Err(Erreur::Serveur(message_inacheve(&format!("L'installeur de GOG s'est arrêté avant la fin ({}, code {c}).", sens_code_inno(c)), &plan.dest, r)));
        }
        IssueInno::Code(_) => {}
    }
    // 4. Vérifier, sans se fier au code 0.
    match verifier_jeu(pc, &plan.dest, &id) {
        Ok(voie) => journal(&format!("installation GOG : jeu {id} : vérifié (lancement : {voie})")),
        Err(raison) => {
            journal(&format!("installation GOG : jeu {id} : code 0 mais vérification ratée ({raison})"));
            return Err(Erreur::Serveur(message_inacheve(&format!("L'installeur de GOG a fini, mais Frogtend ne retrouve pas le jeu installé ({raison})."), &plan.dest, au_registre())));
        }
    }
    // 5. La marque, le journal de l'installeur, puis l'installeur effacé (seulement ses fichiers).
    let (mut marque, protege) = finir_jeu(plan, journal);
    let mut bilan = Bilan { titre: plan.jeu.titre.clone(), dossier_protege: protege, ..Default::default() };
    // 6. Chaque DLC coché, dans le même dossier ; un échec n'enlève jamais le jeu.
    for (e, r) in plan.dlcs.iter().zip(recus.iter().skip(1)) {
        let dossier = plan.installeurs.join(&e.id);
        let echec = |bilan: &mut Bilan, raison: &str| {
            bilan.dlcs_echoues.push((e.titre.clone(), raison.to_string()));
            bilan.installeurs_gardes += r.iter().map(|f| f.taille).sum::<u64>();
        };
        let Some(exe) = exe_de(r) else {
            echec(&mut bilan, "son installeur n'a pas la forme attendue");
            continue;
        };
        match signe(pc, dossier.join(&exe.nom)).await {
            Ok(true) => {}
            _ => {
                echec(&mut bilan, "son installeur n'est pas signé par GOG");
                continue;
            }
        }
        let journal_dlc = dossier.join(JOURNAL_INNO);
        let Ok(parametres) = parametres_inno(&plan.dest, &journal_dlc) else {
            echec(&mut bilan, "dossier refusé");
            continue;
        };
        match executer(pc, dossier.join(&exe.nom), parametres, dossier.clone()).await {
            IssueInno::Refuse => echec(&mut bilan, "tu as répondu Non à Windows"),
            IssueInno::Impossible(_) => echec(&mut bilan, "Windows n'a pas pu lancer son installeur"),
            IssueInno::Code(c) if c != 0 && !inno_a_reussi(&journal_dlc) => echec(&mut bilan, sens_code_inno(c)),
            IssueInno::Code(_) if !verifier_dlc(pc, &plan.dest, &e.id) => echec(&mut bilan, "Frogtend ne le retrouve pas dans le jeu"),
            IssueInno::Code(_) => {
                garder_journal_inno(&journal_dlc, &plan.dest.join(DOSSIER_MARQUE).join(format!("installation-gog-{}.log", e.id)));
                effacer_installeur(&dossier);
                marque.dlcs.push(e.id.clone());
                bilan.dlcs_installes.push(e.titre.clone());
            }
        }
    }
    if !marque.dlcs.is_empty() && !protege {
        let _ = ecrire_marque(&plan.dest, &marque);
    }
    let _ = std::fs::remove_dir(&plan.installeurs); // seulement s'il est vide
    journal(&format!("installation GOG : jeu {id} : {} DLC installé(s), {} en échec", bilan.dlcs_installes.len(), bilan.dlcs_echoues.len()));
    Ok(bilan)
}

/// Les octets déjà reçus pour ce plan (installeurs et `.part`), déduits de la place à demander à une reprise.
pub fn octets_deja_recus(plan: &Plan) -> u64 {
    std::iter::once(&plan.jeu)
        .chain(&plan.dlcs)
        .filter_map(|e| std::fs::read_dir(plan.installeurs.join(&e.id)).ok())
        .flat_map(|l| l.flatten())
        .filter_map(|x| x.metadata().ok().filter(|m| m.is_file()).map(|m| m.len()))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::sync::atomic::AtomicUsize;
    use std::sync::{Arc, Mutex};

    fn md5_de(o: &[u8]) -> String {
        use md5::{Digest, Md5};
        hex(&Md5::digest(o))
    }

    /// Un XML de contrôle pour `donnees`, morceaux de `taille` octets.
    fn xml_de(nom: &str, donnees: &[u8], taille: usize) -> String {
        let mut s = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<file name=\"{nom}\" available=\"1\" notavailablemsg=\"\" md5=\"{}\" chunks=\"{}\" timestamp=\"2026-11-26 10:00:00\" total_size=\"{}\">\n", md5_de(donnees), donnees.len().div_ceil(taille), donnees.len());
        for (i, m) in donnees.chunks(taille).enumerate() {
            let de = i * taille;
            s.push_str(&format!("\t<chunk id=\"{i}\" from=\"{de}\" to=\"{}\" method=\"md5\">{}</chunk>\n", de + m.len() - 1, md5_de(m)));
        }
        s.push_str("</file>");
        s
    }

    #[test]
    fn le_xml_de_controle_reel_se_lit_et_les_ecarts_sont_refuses() {
        let donnees: Vec<u8> = (0..25u8).collect();
        let c = lire_controle(&xml_de("setup_jeu_1.0.exe", &donnees, 10)).unwrap();
        assert_eq!((c.nom.as_str(), c.total, c.morceaux.len()), ("setup_jeu_1.0.exe", 25, 3));
        assert_eq!(c.morceaux[2], Morceau { debut: 20, fin: 24, md5: md5_de(&donnees[20..]) });
        assert_eq!(c.md5, md5_de(&donnees));
        // Un nom avec une entité XML est gardé tel quel (décodé), jamais renommé.
        let c = lire_controle(&xml_de("setup_tom_&amp;_jerry.exe", &donnees, 10)).unwrap();
        assert_eq!(c.nom, "setup_tom_&_jerry.exe");
        let bon = xml_de("setup.exe", &donnees, 10);
        let pieges = [
            bon.replace("setup.exe", "..\\setup.exe"),
            bon.replace("setup.exe", "a/b.exe"),
            bon.replace("from=\"10\"", "from=\"11\""),
            bon.replace("chunks=\"3\"", "chunks=\"4\""),
            bon.replace("total_size=\"25\"", "total_size=\"26\""),
            bon.replace("method=\"md5\"", "method=\"sha1\""),
            bon.replace("<file ", "<fichier "),
            bon.replace(&md5_de(&donnees), "zz"),
            bon.replace("id=\"1\"", "id=\"2\""),
            bon.replace("available=\"1\"", "available=\"0\""),
            "pas du xml".to_string(),
            "x".repeat(PLAFOND_XML + 1),
        ];
        for (i, p) in pieges.iter().enumerate() {
            assert!(lire_controle(p).is_err(), "piège {i} accepté");
        }
    }

    fn produit(api: &str) -> Value {
        json!({
            "id": 1207658924, "title": "Another World™ – 20th Anniversary Edition", "languages": {"en": "English", "fr": "français"},
            "downloads": {"installers": [
                {"id": "installer_mac_en", "os": "mac", "language": "en", "version": "1.0", "total_size": 10, "files": [{"id": "en2installer0", "size": 10, "downlink": format!("{api}/products/1207658924/downlink/installer/en2installer0")}]},
                {"id": "installer_windows_en", "os": "windows", "language": "en", "version": "1.0", "total_size": 25, "files": [{"id": "en1installer0", "size": 25, "downlink": format!("{api}/products/1207658924/downlink/installer/en1installer0")}]},
            ]},
            "expanded_dlcs": [
                {"id": 1597316373, "title": "REDmod", "downloads": {"installers": [{"id": "installer_windows_en", "os": "windows", "language": "fr", "version": "2", "total_size": 7, "files": [{"id": "fr1installer0", "size": 7, "downlink": format!("{api}/products/1597316373/downlink/installer/fr1installer0")}]}]}},
                {"id": 1256837418, "title": "Phantom Liberty", "downloads": {"installers": [{"id": "x", "os": "windows", "language": "en", "total_size": 7, "files": [{"id": "en1installer0", "size": 7, "downlink": format!("{api}/products/1256837418/downlink/installer/en1installer0")}]}]}},
                {"id": 2000000001, "title": "Carte interactive", "downloads": {"installers": []}},
            ]
        })
    }

    #[test]
    fn la_fiche_donne_l_installeur_windows_fr_sinon_en_et_les_dlc_possedes_seulement() {
        let api = "https://api.gog.com";
        let p = lire_produit_installation(&produit(api), &["1597316373".into(), "2000000001".into()], api).unwrap();
        assert_eq!(p.titre, "Another World – 20th Anniversary Edition");
        assert!(p.en_francais, "languages contient fr");
        let j = p.jeu.unwrap();
        assert_eq!((j.installeur.langue.as_str(), j.installeur.total_annonce), ("en", 25), "pas de fr : repli en");
        assert_eq!(j.installeur.fichiers[0].id, "en1installer0");
        // Phantom Liberty n'est pas possédé ; la carte n'a pas d'installeur Windows : aucun des deux.
        assert_eq!(p.dlcs.iter().map(|d| d.id.as_str()).collect::<Vec<_>>(), ["1597316373"]);
        assert_eq!(p.dlcs[0].installeur.langue, "fr");
        // Un lien qui n'est pas sous api.gog.com : l'installeur n'est pas pris.
        let mut v = produit(api);
        v["downloads"]["installers"][1]["files"][0]["downlink"] = json!("https://evil.example/products/1/downlink");
        assert!(lire_produit_installation(&v, &[], api).unwrap().jeu.is_none());
        // Le français dit par languages (objet ou liste), pas par la langue de l'installeur.
        assert!(!contient_le_francais(&json!({"languages": {"en": "English"}})));
        assert!(contient_le_francais(&json!({"languages": ["en", "fr-FR"]})));
        // Le Debug d'un fichier ne montre jamais sa route.
        assert!(!format!("{:?}", j.installeur.fichiers[0]).contains("downlink"));
    }

    #[test]
    fn le_jeton_ne_part_que_vers_l_api_et_le_cdn_reste_chez_gog() {
        let api = "https://api.gog.com";
        assert!(lien_api_sur("https://api.gog.com/products/1/downlink/installer/en1installer0", api));
        for piege in ["http://api.gog.com/products/1", "https://api.gog.com.evil.example/products/1", "https://x@api.gog.com/products/1", "https://api.gog.com/users/1", "https://api.gog.com:8443/products/1"] {
            assert!(!lien_api_sur(piege, api), "{piege}");
        }
        let u = |s: &str| reqwest::Url::parse(s).unwrap();
        assert!(hote_cdn_permis(&u("https://gog-cdn-fastly.gog.com/token=x/f.exe"), true));
        assert!(!hote_cdn_permis(&u("http://gog-cdn-fastly.gog.com/f.exe"), true));
        assert!(!hote_cdn_permis(&u("https://gog.com.evil.example/f.exe"), true));
        assert!(!hote_cdn_permis(&u("https://evilgog.com/f.exe"), true));
    }

    #[test]
    fn le_compte_qui_possede_le_jeu_est_choisi() {
        use crate::boutiques::NavigateurGog;
        let c = |g: &str, nav: NavigateurGog, ids: &[&str]| CompteGog { id_galaxy: g.into(), navigateur: nav, ids_lus: ids.iter().map(|s| s.to_string()).collect(), ..Default::default() };
        let comptes = vec![c("1", NavigateurGog::Ajoute, &["42"]), c("2", NavigateurGog::Profil, &["42"]), c("3", NavigateurGog::Ajoute, &["7"])];
        assert_eq!(compte_possesseur(&comptes, "42").unwrap().id_galaxy, "2", "le compte Profil d'abord");
        assert_eq!(compte_possesseur(&comptes, "7").unwrap().id_galaxy, "3");
        assert!(compte_possesseur(&comptes, "99").is_none(), "personne ne l'a et plusieurs comptes : on ne devine pas");
        let seul = vec![c("5", NavigateurGog::Profil, &[])];
        assert_eq!(compte_possesseur(&seul, "99").unwrap().id_galaxy, "5", "un seul compte : lui");
        assert!(compte_possesseur(&[c("", NavigateurGog::Profil, &["42"])], "42").is_none(), "sans numéro Galaxy, pas de jeton");
    }

    #[test]
    fn la_place_demandee_compte_l_installeur_le_jeu_et_les_marges() {
        assert_eq!(place_necessaire(0), crate::jeux_pc::MARGE_LIBRE);
        assert_eq!(place_necessaire(100_000), 2 * 102_000 + crate::jeux_pc::MARGE_LIBRE);
        // Le relevé du 26/11 : 122 683 392 annoncés pour 123 571 824 réels : couvert.
        assert!(place_necessaire(122_683_392) - crate::jeux_pc::MARGE_LIBRE >= 2 * 123_571_824);
    }

    #[test]
    fn le_journal_d_inno_dit_si_l_installation_a_reussi() {
        let d = tempfile::tempdir().unwrap();
        let j = d.path().join("j.log");
        assert!(!inno_a_reussi(&j), "absent");
        std::fs::write(&j, "2026-11-28 15:24:02.626   Writing uninstall key values.
").unwrap();
        assert!(!inno_a_reussi(&j));
        std::fs::write(&j, "2026-11-28 15:24:02.703   Installation process succeeded.
2026-11-28 15:24:04.364   Notify launcher
").unwrap();
        assert!(inno_a_reussi(&j));
    }

    #[test]
    fn la_signature_doit_etre_valide_et_celle_de_gog() {
        assert!(signature_gog_valide("Valid\r\nCN=GOG Sp. z o.o., O=GOG Sp. z o.o., L=Warsaw, S=Mazowieckie, C=PL\r\n"));
        assert!(signature_gog_valide("Valid\nO=\"GOG Sp. z o.o.\", C=PL"));
        assert!(!signature_gog_valide("HashMismatch\nCN=GOG Sp. z o.o."));
        assert!(!signature_gog_valide("NotSigned\n"));
        assert!(!signature_gog_valide("Valid\nCN=GOG Sp. z o.o. Fake, O=Autre"));
        assert!(!signature_gog_valide("Valid\nOU=GOG Sp. z o.o., O=Autre"));
        assert!(!signature_gog_valide(""));
        // Le sujet RÉEL relevé le 28/11 (installeur d'Another World 20th) : deux espaces, « sp », sans point final.
        assert!(signature_gog_valide("Valid\nCN=GOG  sp. z o.o, O=GOG  sp. z o.o, L=WARSZAWA, S=MAZOWIECKIE, C=PL\n"));
        assert!(!signature_gog_valide("Valid\nCN=GOG sp. z o.o. extra, O=GOG"));
        assert!(!signature_gog_valide("NotSigned\nCN=GOG  sp. z o.o"));
    }

    #[test]
    fn les_parametres_d_inno_suivent_sa_documentation() {
        let p = parametres_inno(Path::new(r"E:\Frogtend\Jeux\Another World\"), Path::new(r"E:\Frogtend\Jeux\Installeurs GOG (Frogtend)\1\installation-gog.log")).unwrap();
        assert_eq!(p, r#"/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP- /NOCANCEL /DIR="E:\Frogtend\Jeux\Another World" /LOG="E:\Frogtend\Jeux\Installeurs GOG (Frogtend)\1\installation-gog.log""#);
        assert!(!p.contains("/CURRENTUSER") && !p.contains("/LANG"), "HKLM lu par Frogtend ; langue choisie dans products");
        assert!(parametres_inno(Path::new("relatif"), Path::new(r"C:\j.log")).is_err());
        assert_eq!(sens_code_inno(0), "fini");
        assert!(sens_code_inno(5).starts_with("l'installation a été annulée") && sens_code_inno(2).contains("répondre Oui"));
        assert_eq!(sens_code_inno(99), "un code inconnu");
        assert_eq!(lire_content_range("bytes 10-24/25"), Some((10, 24, 25)));
        assert_eq!(lire_content_range("bytes */25"), None);
    }

    // ---- Le téléchargement contre un faux GOG (httpmock) ----

    /// Un faux jeton : `NEUF` après un renouvellement, `VIEUX` avant.
    struct FauxJeton {
        renouvellements: AtomicUsize,
        appels: AtomicUsize,
    }

    impl FauxJeton {
        fn nouveau() -> Self {
            FauxJeton { renouvellements: AtomicUsize::new(0), appels: AtomicUsize::new(0) }
        }
    }

    impl SourceJeton for FauxJeton {
        async fn jeton(&self, renouveler: bool) -> Resultat<String> {
            self.appels.fetch_add(1, Ordering::SeqCst);
            if renouveler {
                self.renouvellements.fetch_add(1, Ordering::SeqCst);
            }
            Ok(if self.renouvellements.load(Ordering::SeqCst) > 0 { "NEUF".into() } else { "VIEUX".into() })
        }
    }

    /// Les données d'un faux fichier (2,5 morceaux de 10 octets).
    fn donnees(graine: u8, n: usize) -> Vec<u8> {
        (0..n).map(|i| (i as u8).wrapping_mul(7).wrapping_add(graine)).collect()
    }

    /// Sert un fichier du faux CDN : downlink (jeton `jeton`), XML, et le fichier par plages (une par frontière de
    /// morceau).
    fn servir(s: &MockServer, produit: &str, fid: &str, nom: &str, d: &[u8], jeton: &str) {
        let base = s.base_url();
        s.mock(|w, t| {
            w.method(GET).path(format!("/products/{produit}/downlink/installer/{fid}")).header("authorization", format!("Bearer {jeton}"));
            t.status(200).json_body(json!({"downlink": format!("{base}/cdn/{produit}/{nom}?signe=SECRET"), "checksum": format!("{base}/cdn/{produit}/{nom}.xml?signe=SECRET")}));
        });
        let xml = xml_de(nom, d, 10);
        s.mock(|w, t| {
            w.method(GET).path(format!("/cdn/{produit}/{nom}.xml"));
            t.status(200).body(xml.clone());
        });
        for debut in (0..d.len()).step_by(10) {
            let corps = d[debut..].to_vec();
            s.mock(|w, t| {
                w.method(GET).path(format!("/cdn/{produit}/{nom}")).header("range", format!("bytes={debut}-"));
                t.status(206).header("content-range", format!("bytes {debut}-{}/{}", d.len() - 1, d.len())).header("etag", format!("\"{}\"", md5_de(d))).body(corps);
            });
        }
    }

    fn element(s: &MockServer, id: &str, fid: &str, total: u64) -> Element {
        Element {
            id: id.into(),
            titre: format!("Jeu {id}"),
            installeur: InstalleurGog { id: "installer_windows_en".into(), langue: "en".into(), version: "1.0".into(), total_annonce: total, fichiers: vec![FichierInstalleur { id: fid.into(), taille_annoncee: total, lien: format!("{}/products/{id}/downlink/installer/{fid}", s.base_url()) }] },
        }
    }

    fn rien(_: &'static str, _: u64, _: u64) {}

    #[tokio::test]
    async fn un_fichier_est_recu_morceau_par_morceau_avec_un_jeton_renouvele_apres_un_401() {
        let s = MockServer::start_async().await;
        let d = donnees(3, 25);
        servir(&s, "11", "en1installer0", "setup_jeu.exe", &d, "NEUF");
        let refuse = s.mock(|w, t| {
            w.method(GET).path("/products/11/downlink/installer/en1installer0").header("authorization", "Bearer VIEUX");
            t.status(401);
        });
        let dir = tempfile::tempdir().unwrap();
        let reseau = ReseauGog::essai(&s.base_url());
        let jeton = FauxJeton::nouveau();
        let arret = AtomicBool::new(false);
        let vus = Mutex::new(vec![]);
        let progres = |e: &'static str, f: u64, t: u64| vus.lock().unwrap().push((e, f, t));
        let av = Avancement { arret: &arret, progres: &progres, total: 25 };
        let e = element(&s, "11", "en1installer0", 25);
        let mut deja = 0;
        let r = telecharger_installeur(&reseau, &reseau.client_api().unwrap(), &reseau.client_cdn().unwrap(), &jeton, &e, dir.path(), &av, &mut deja).await.unwrap();
        let Issue::Fini(r) = r else { panic!("pause") };
        assert_eq!(r, vec![Recu { nom: "setup_jeu.exe".into(), taille: 25, md5: md5_de(&d) }]);
        assert_eq!(std::fs::read(dir.path().join("setup_jeu.exe")).unwrap(), d, "le fichier, octet pour octet, sous SON nom");
        assert!(!dir.path().join("setup_jeu.exe.part").exists() && !dir.path().join("setup_jeu.exe.part.json").exists());
        assert_eq!(deja, 25);
        refuse.assert_async().await;
        assert_eq!(jeton.renouvellements.load(Ordering::SeqCst), 1, "un seul renouvellement");
        let note: NoteInstalleur = serde_json::from_slice(&std::fs::read(dir.path().join(NOTE_INSTALLEUR)).unwrap()).unwrap();
        assert_eq!(note.fichiers.len(), 1);
        assert!(vus.lock().unwrap().iter().any(|(e, f, _)| *e == "telechargement" && *f == 25));
        // Rien de secret écrit sur le disque.
        for x in std::fs::read_dir(dir.path()).unwrap().flatten() {
            let o = std::fs::read(x.path()).unwrap();
            assert!(!String::from_utf8_lossy(&o).contains("SECRET") && !String::from_utf8_lossy(&o).contains("NEUF"));
        }
    }

    #[tokio::test]
    async fn un_morceau_abime_est_redemande_une_fois_puis_refuse() {
        let s = MockServer::start_async().await;
        let d = donnees(5, 25);
        let base = s.base_url();
        s.mock(|w, t| {
            w.method(GET).path("/products/12/downlink/installer/f0");
            t.status(200).json_body(json!({"downlink": format!("{base}/cdn/f"), "checksum": format!("{base}/cdn/f.xml")}));
        });
        let xml = xml_de("setup.exe", &d, 10);
        s.mock(|w, t| {
            w.method(GET).path("/cdn/f.xml");
            t.status(200).body(xml.clone());
        });
        // Depuis 0 : le 2e morceau est faux ; depuis 10 (la reprise après le faux) : juste.
        let mut faux = d.clone();
        faux[12] ^= 0xFF;
        let depuis0 = s.mock(|w, t| {
            w.method(GET).path("/cdn/f").header("range", "bytes=0-");
            t.status(206).header("content-range", "bytes 0-24/25").body(faux.clone());
        });
        let depuis10 = s.mock(|w, t| {
            w.method(GET).path("/cdn/f").header("range", "bytes=10-");
            t.status(206).header("content-range", "bytes 10-24/25").body(d[10..].to_vec());
        });
        let dir = tempfile::tempdir().unwrap();
        let reseau = ReseauGog::essai(&base);
        let arret = AtomicBool::new(false);
        let av = Avancement { arret: &arret, progres: &rien, total: 25 };
        let e = element(&s, "12", "f0", 25);
        let mut deja = 0;
        let r = telecharger_installeur(&reseau, &reseau.client_api().unwrap(), &reseau.client_cdn().unwrap(), &FauxJeton::nouveau(), &e, dir.path(), &av, &mut deja).await.unwrap();
        assert!(matches!(r, Issue::Fini(_)));
        assert_eq!(std::fs::read(dir.path().join("setup.exe")).unwrap(), d, "le morceau faux a été remplacé");
        depuis0.assert_async().await;
        depuis10.assert_async().await;

        // Toujours faux depuis 10 : refusé à la 2e fois, le début vérifié gardé.
        let s2 = MockServer::start_async().await;
        let base2 = s2.base_url();
        s2.mock(|w, t| {
            w.method(GET).path("/products/12/downlink/installer/f0");
            t.status(200).json_body(json!({"downlink": format!("{base2}/cdn/f"), "checksum": format!("{base2}/cdn/f.xml")}));
        });
        s2.mock(|w, t| {
            w.method(GET).path("/cdn/f.xml");
            t.status(200).body(xml.clone());
        });
        s2.mock(|w, t| {
            w.method(GET).path("/cdn/f").header("range", "bytes=0-");
            t.status(206).header("content-range", "bytes 0-24/25").body(faux.clone());
        });
        s2.mock(|w, t| {
            w.method(GET).path("/cdn/f").header("range", "bytes=10-");
            t.status(206).header("content-range", "bytes 10-24/25").body(faux[10..].to_vec());
        });
        let dir2 = tempfile::tempdir().unwrap();
        let reseau2 = ReseauGog::essai(&base2);
        let e2 = element(&s2, "12", "f0", 25);
        let mut deja = 0;
        let err = telecharger_installeur(&reseau2, &reseau2.client_api().unwrap(), &reseau2.client_cdn().unwrap(), &FauxJeton::nouveau(), &e2, dir2.path(), &av, &mut deja).await.unwrap_err();
        assert!(err.to_string().contains("abîmé de GOG deux fois"), "{err}");
        assert_eq!(std::fs::metadata(dir2.path().join("setup.exe.part")).unwrap().len(), 10, "coupé au début du morceau abîmé");
        assert!(!err.to_string().contains("cdn"), "jamais d'adresse dans une erreur");
    }

    #[tokio::test]
    async fn une_reprise_reverifie_le_debut_et_repart_de_la_derniere_frontiere() {
        let s = MockServer::start_async().await;
        let d = donnees(9, 25);
        servir(&s, "13", "f0", "setup.exe", &d, "VIEUX");
        let dir = tempfile::tempdir().unwrap();
        let reseau = ReseauGog::essai(&s.base_url());
        // Un .part de 15 octets (1 morceau et demi), sa note dit 1 morceau vérifié.
        std::fs::write(dir.path().join("setup.exe.part"), &d[..15]).unwrap();
        let note = NotePart { installeur: "installer_windows_en".into(), version: "1.0".into(), fichier: "f0".into(), md5: md5_de(&d), total: 25, verifies: 1 };
        ecrire_note(&dir.path().join("setup.exe.part.json"), &note).unwrap();
        let arret = AtomicBool::new(false);
        let av = Avancement { arret: &arret, progres: &rien, total: 25 };
        let e = element(&s, "13", "f0", 25);
        let mut deja = 0;
        let r = telecharger_installeur(&reseau, &reseau.client_api().unwrap(), &reseau.client_cdn().unwrap(), &FauxJeton::nouveau(), &e, dir.path(), &av, &mut deja).await.unwrap();
        assert!(matches!(r, Issue::Fini(_)));
        assert_eq!(std::fs::read(dir.path().join("setup.exe")).unwrap(), d);

        // Une note d'un AUTRE fichier (md5 changé chez GOG) : le .part est écarté, tout repart de zéro.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("setup.exe.part"), b"vieux contenu").unwrap();
        ecrire_note(&dir.path().join("setup.exe.part.json"), &NotePart { md5: "0".repeat(32), ..note.clone() }).unwrap();
        let mut deja = 0;
        telecharger_installeur(&reseau, &reseau.client_api().unwrap(), &reseau.client_cdn().unwrap(), &FauxJeton::nouveau(), &e, dir.path(), &av, &mut deja).await.unwrap();
        assert_eq!(std::fs::read(dir.path().join("setup.exe")).unwrap(), d);

        // Une note qui ment (morceau 0 abîmé sur le disque) : revérifié, coupé, retéléchargé.
        let dir = tempfile::tempdir().unwrap();
        let mut abime = d[..20].to_vec();
        abime[3] ^= 1;
        std::fs::write(dir.path().join("setup.exe.part"), &abime).unwrap();
        ecrire_note(&dir.path().join("setup.exe.part.json"), &NotePart { verifies: 2, ..note.clone() }).unwrap();
        let mut deja = 0;
        telecharger_installeur(&reseau, &reseau.client_api().unwrap(), &reseau.client_cdn().unwrap(), &FauxJeton::nouveau(), &e, dir.path(), &av, &mut deja).await.unwrap();
        assert_eq!(std::fs::read(dir.path().join("setup.exe")).unwrap(), d);
    }

    #[tokio::test]
    async fn l_arret_garde_le_part_et_un_etag_d_un_autre_fichier_est_refuse() {
        let s = MockServer::start_async().await;
        let d = donnees(1, 25);
        servir(&s, "14", "f0", "setup.exe", &d, "VIEUX");
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("setup.exe.part"), &d[..10]).unwrap();
        ecrire_note(&dir.path().join("setup.exe.part.json"), &NotePart { installeur: "installer_windows_en".into(), version: "1.0".into(), fichier: "f0".into(), md5: md5_de(&d), total: 25, verifies: 1 }).unwrap();
        let reseau = ReseauGog::essai(&s.base_url());
        let arret = AtomicBool::new(true);
        let av = Avancement { arret: &arret, progres: &rien, total: 25 };
        let e = element(&s, "14", "f0", 25);
        let mut deja = 0;
        let r = telecharger_installeur(&reseau, &reseau.client_api().unwrap(), &reseau.client_cdn().unwrap(), &FauxJeton::nouveau(), &e, dir.path(), &av, &mut deja).await.unwrap();
        assert_eq!(r, Issue::Pause);
        assert_eq!(std::fs::read(dir.path().join("setup.exe.part")).unwrap(), &d[..10], "rien n'est perdu");

        let s = MockServer::start_async().await;
        let base = s.base_url();
        s.mock(|w, t| {
            w.method(GET).path("/products/14/downlink/installer/f0");
            t.status(200).json_body(json!({"downlink": format!("{base}/cdn/f"), "checksum": format!("{base}/cdn/f.xml")}));
        });
        let xml = xml_de("setup.exe", &d, 10);
        s.mock(|w, t| {
            w.method(GET).path("/cdn/f.xml");
            t.status(200).body(xml.clone());
        });
        s.mock(|w, t| {
            w.method(GET).path("/cdn/f");
            t.status(206).header("content-range", "bytes 0-24/25").header("etag", format!("\"{}\"", "a".repeat(32))).body(d.clone());
        });
        let reseau = ReseauGog::essai(&base);
        let arret = AtomicBool::new(false);
        let av = Avancement { arret: &arret, progres: &rien, total: 25 };
        let dir = tempfile::tempdir().unwrap();
        let e = element(&s, "14", "f0", 25);
        let mut deja = 0;
        let err = telecharger_installeur(&reseau, &reseau.client_api().unwrap(), &reseau.client_cdn().unwrap(), &FauxJeton::nouveau(), &e, dir.path(), &av, &mut deja).await.unwrap_err();
        assert!(err.to_string().contains("autre fichier"), "{err}");
        assert!(!dir.path().join("setup.exe").exists());
    }

    #[tokio::test]
    async fn une_coupure_en_pleine_reception_reprend_a_la_frontiere_avec_un_lien_redemande() {
        let s = MockServer::start_async().await;
        let d = donnees(6, 25);
        let base = s.base_url();
        let lien = s.mock(|w, t| {
            w.method(GET).path("/products/17/downlink/installer/f0");
            t.status(200).json_body(json!({"downlink": format!("{base}/cdn/f"), "checksum": format!("{base}/cdn/f.xml")}));
        });
        let xml = xml_de("setup.exe", &d, 10);
        s.mock(|w, t| {
            w.method(GET).path("/cdn/f.xml");
            t.status(200).body(xml.clone());
        });
        // Depuis 0 : le flux s'arrête à 15 octets (1 morceau et demi) ; la reprise demande 10-.
        s.mock(|w, t| {
            w.method(GET).path("/cdn/f").header("range", "bytes=0-");
            t.status(206).header("content-range", "bytes 0-24/25").body(d[..15].to_vec());
        });
        let reprise = s.mock(|w, t| {
            w.method(GET).path("/cdn/f").header("range", "bytes=10-");
            t.status(206).header("content-range", "bytes 10-24/25").body(d[10..].to_vec());
        });
        let reseau = ReseauGog::essai(&base);
        let arret = AtomicBool::new(false);
        let av = Avancement { arret: &arret, progres: &rien, total: 25 };
        let dir = tempfile::tempdir().unwrap();
        let mut deja = 0;
        let r = telecharger_installeur(&reseau, &reseau.client_api().unwrap(), &reseau.client_cdn().unwrap(), &FauxJeton::nouveau(), &element(&s, "17", "f0", 25), dir.path(), &av, &mut deja).await.unwrap();
        assert!(matches!(r, Issue::Fini(_)));
        assert_eq!(std::fs::read(dir.path().join("setup.exe")).unwrap(), d);
        reprise.assert_async().await;
        lien.assert_hits_async(2).await;
    }

    #[tokio::test]
    async fn une_plage_inattendue_et_un_xml_trop_gros_sont_refuses() {
        let s = MockServer::start_async().await;
        let d = donnees(2, 25);
        let base = s.base_url();
        s.mock(|w, t| {
            w.method(GET).path("/products/15/downlink/installer/f0");
            t.status(200).json_body(json!({"downlink": format!("{base}/cdn/f"), "checksum": format!("{base}/cdn/f.xml")}));
        });
        let xml = xml_de("setup.exe", &d, 10);
        s.mock(|w, t| {
            w.method(GET).path("/cdn/f.xml");
            t.status(200).body(xml.clone());
        });
        s.mock(|w, t| {
            w.method(GET).path("/cdn/f");
            t.status(206).header("content-range", "bytes 0-24/30").body(d.clone());
        });
        let reseau = ReseauGog::essai(&base);
        let arret = AtomicBool::new(false);
        let av = Avancement { arret: &arret, progres: &rien, total: 25 };
        let dir = tempfile::tempdir().unwrap();
        let mut deja = 0;
        let err = telecharger_installeur(&reseau, &reseau.client_api().unwrap(), &reseau.client_cdn().unwrap(), &FauxJeton::nouveau(), &element(&s, "15", "f0", 25), dir.path(), &av, &mut deja).await.unwrap_err();
        assert!(err.to_string().contains("autre partie"), "{err}");

        let s = MockServer::start_async().await;
        let base = s.base_url();
        s.mock(|w, t| {
            w.method(GET).path("/products/16/downlink/installer/f0");
            t.status(200).json_body(json!({"downlink": format!("{base}/cdn/f"), "checksum": format!("{base}/cdn/f.xml")}));
        });
        s.mock(|w, t| {
            w.method(GET).path("/cdn/f.xml");
            t.status(200).body("x".repeat(PLAFOND_XML + 10));
        });
        let reseau = ReseauGog::essai(&base);
        let mut deja = 0;
        let err = telecharger_installeur(&reseau, &reseau.client_api().unwrap(), &reseau.client_cdn().unwrap(), &FauxJeton::nouveau(), &element(&s, "16", "f0", 25), dir.path(), &av, &mut deja).await.unwrap_err();
        assert!(err.to_string().contains("plus grosse"), "{err}");
    }

    // ---- L'installation entière, avec un faux PC (aucun installeur lancé, aucun registre lu) ----

    #[derive(Clone)]
    struct FauxPc(Arc<FauxPcIn>);

    struct FauxPcIn {
        signe: bool,
        /// Les issues des installeurs, dans l'ordre (le jeu, puis chaque DLC).
        issues: Mutex<Vec<IssueInno>>,
        registre: Mutex<BTreeMap<String, Valeurs>>,
        lances: Mutex<Vec<(PathBuf, String)>>,
        /// Ce que l'installeur « pose » dans le dossier (dest, numéro) quand il rend 0.
        pose: bool,
        /// L'installeur laisse un dossier où Frogtend ne peut pas écrire sa marque (un FICHIER `.frogtend-gog`).
        bloque: AtomicBool,
    }

    const PLANTAGE_APRES_SUCCES: u32 = 0xC000_041D;

    impl FauxPc {
        fn nouveau(signe: bool, issues: Vec<IssueInno>, pose: bool) -> Self {
            FauxPc(Arc::new(FauxPcIn { signe, issues: Mutex::new(issues), registre: Mutex::new(BTreeMap::new()), lances: Mutex::new(vec![]), pose, bloque: AtomicBool::new(false) }))
        }
    }

    impl PcGog for FauxPc {
        fn signe_par_gog(&self, _exe: &Path) -> Resultat<bool> {
            Ok(self.0.signe)
        }
        fn executer_eleve(&self, exe: &Path, parametres: &str, _travail: &Path) -> IssueInno {
            self.0.lances.lock().unwrap().push((exe.to_path_buf(), parametres.to_string()));
            let issue = { let mut l = self.0.issues.lock().unwrap(); if l.is_empty() { IssueInno::Code(0) } else { l.remove(0) } };
            // 0xC000041D : comme Another World 20th le 28/11 — tout posé, « Installation process succeeded » au journal,
            // puis un plantage en sortant.
            let plante = issue == IssueInno::Code(PLANTAGE_APRES_SUCCES);
            if plante && self.0.pose {
                let log = parametres.split("/LOG=\"").nth(1).and_then(|r| r.split('"').next()).unwrap();
                std::fs::write(log, "Installation process succeeded.
Notify launcher: GalaxyClient.exe
").unwrap();
            }
            if (issue == IssueInno::Code(0) || plante) && self.0.pose {
                // Comme un installeur de GOG : le dossier, le programme, goggame-<id>.info, l'entrée du registre.
                let dir = parametres.split("/DIR=\"").nth(1).and_then(|r| r.split('"').next()).unwrap();
                let id = exe.parent().unwrap().file_name().unwrap().to_string_lossy().to_string();
                let dest = Path::new(dir);
                std::fs::create_dir_all(dest.join("bin")).unwrap();
                std::fs::write(dest.join("bin").join("jeu.exe"), b"MZ").unwrap();
                if self.0.bloque.load(Ordering::SeqCst) {
                    std::fs::write(dest.join(DOSSIER_MARQUE), b"").unwrap();
                }
                std::fs::write(dest.join(format!("goggame-{id}.info")), serde_json::to_vec(&json!({"gameId": id, "playTasks": [{"type": "FileTask", "category": "game", "isPrimary": true, "path": "bin\\jeu.exe"}]})).unwrap()).unwrap();
                if id == "21" {
                    let mut v = Valeurs::new();
                    v.insert("path".into(), dir.into());
                    v.insert("gameName".into(), "Jeu".into());
                    self.0.registre.lock().unwrap().insert(id, v);
                }
            }
            issue
        }
        fn entree(&self, id: &str) -> Option<Valeurs> {
            self.0.registre.lock().unwrap().get(id).cloned()
        }
    }

    fn plan_essai(s: &MockServer, racine: &Path) -> Plan {
        Plan { dest: racine.join("Jeu 21"), installeurs: racine.join(DOSSIER_INSTALLEURS), jeu: element(s, "21", "f0", 25), dlcs: vec![element(s, "22", "f0", 15)] }
    }

    fn servir_jeu_et_dlc(s: &MockServer) -> (Vec<u8>, Vec<u8>) {
        let (j, dl) = (donnees(4, 25), donnees(8, 15));
        servir(s, "21", "f0", "setup_jeu.exe", &j, "VIEUX");
        servir(s, "22", "f0", "setup_dlc.exe", &dl, "VIEUX");
        (j, dl)
    }

    #[tokio::test]
    async fn le_jeu_puis_son_dlc_sont_installes_verifies_marques_et_leurs_installeurs_effaces() {
        let s = MockServer::start_async().await;
        servir_jeu_et_dlc(&s);
        let racine = tempfile::tempdir().unwrap();
        let plan = plan_essai(&s, racine.path());
        let pc = FauxPc::nouveau(true, vec![], true);
        let (arret, annulable) = (AtomicBool::new(false), AtomicBool::new(true));
        let lignes = Mutex::new(vec![]);
        let journal = |m: &str| lignes.lock().unwrap().push(m.to_string());
        let etapes = Mutex::new(vec![]);
        let progres = |e: &'static str, _: u64, _: u64| etapes.lock().unwrap().push(e);
        let b = installer(&ReseauGog::essai(&s.base_url()), &FauxJeton::nouveau(), &pc, &plan, &arret, &annulable, &progres, &journal).await.unwrap();
        assert_eq!(b, Bilan { pause: false, titre: "Jeu 21".into(), dlcs_installes: vec!["Jeu 22".into()], ..Default::default() });
        assert!(plan.dest.join(DOSSIER_MARQUE).join(JOURNAL_INNO).exists() || !plan.installeurs.exists(), "journal gardé s'il existait");
        assert!(!annulable.load(Ordering::SeqCst), "plus d'annulation une fois l'installeur lancé");
        let lances = pc.0.lances.lock().unwrap().clone();
        assert_eq!(lances.len(), 2, "le jeu, puis le DLC");
        assert!(lances[0].0.ends_with(Path::new("21").join("setup_jeu.exe")) && lances[1].0.ends_with(Path::new("22").join("setup_dlc.exe")));
        assert!(lances.iter().all(|(_, p)| p.contains(&format!("/DIR=\"{}\"", plan.dest.display()))), "même dossier pour le DLC");
        let marque: Marque = serde_json::from_slice(&std::fs::read(plan.dest.join(DOSSIER_MARQUE).join(FICHIER_MARQUE)).unwrap()).unwrap();
        assert_eq!((marque.game_id.as_str(), marque.dlcs.as_slice()), ("21", &["22".to_string()][..]));
        assert!(!plan.installeurs.exists(), "installeurs effacés après vérification, dossier vide retiré");
        assert!(etapes.lock().unwrap().contains(&"installation"));
        assert!(lignes.lock().unwrap().iter().all(|l| !l.contains("Jeu 21") && !l.contains(&racine.path().display().to_string())), "jamais de titre ni de chemin au journal");
    }

    #[tokio::test]
    async fn un_installeur_qui_plante_apres_avoir_reussi_donne_un_jeu_verifie_et_installe() {
        let s = MockServer::start_async().await;
        servir_jeu_et_dlc(&s);
        let racine = tempfile::tempdir().unwrap();
        let plan = plan_essai(&s, racine.path());
        let pc = FauxPc::nouveau(true, vec![IssueInno::Code(PLANTAGE_APRES_SUCCES), IssueInno::Code(PLANTAGE_APRES_SUCCES)], true);
        let (arret, annulable) = (AtomicBool::new(false), AtomicBool::new(true));
        let bilan = installer(&ReseauGog::essai(&s.base_url()), &FauxJeton::nouveau(), &pc, &plan, &arret, &annulable, &rien, &|_| {}).await.unwrap();
        assert_eq!(bilan.dlcs_installes.len(), 1, "le DLC aussi : {:?}", bilan.dlcs_echoues);
        let marque: Marque = serde_json::from_slice(&std::fs::read(plan.dest.join(DOSSIER_MARQUE).join(FICHIER_MARQUE)).unwrap()).unwrap();
        assert_eq!(marque.game_id, "21");
        assert!(!plan.installeurs.exists(), "installeurs effacés après vérification");

        // Le même plantage SANS rien de posé (ni journal ni jeu) : échec, rien d'effacé.
        let racine = tempfile::tempdir().unwrap();
        let plan = plan_essai(&s, racine.path());
        let pc = FauxPc::nouveau(true, vec![IssueInno::Code(PLANTAGE_APRES_SUCCES)], false);
        let err = installer(&ReseauGog::essai(&s.base_url()), &FauxJeton::nouveau(), &pc, &plan, &arret, &AtomicBool::new(true), &rien, &|_| {}).await.unwrap_err();
        assert!(err.to_string().contains("s'est arrêté avant la fin"), "{err}");
        assert!(plan.installeurs.join("21").join("setup_jeu.exe").is_file(), "installeur gardé");
    }

    #[tokio::test]
    async fn un_refus_de_windows_un_code_d_erreur_ou_une_verification_ratee_n_effacent_rien() {
        for (issue, pose, attendu) in [
            (IssueInno::Refuse, true, MESSAGE_UAC.to_string()),
            (IssueInno::Code(4), true, "une erreur grave pendant l'installation, code 4".to_string()),
            (IssueInno::Code(0), false, "ne retrouve pas le jeu".to_string()),
        ] {
            let s = MockServer::start_async().await;
            servir_jeu_et_dlc(&s);
            let racine = tempfile::tempdir().unwrap();
            let plan = plan_essai(&s, racine.path());
            let pc = FauxPc::nouveau(true, vec![issue.clone()], pose);
            let (arret, annulable) = (AtomicBool::new(false), AtomicBool::new(true));
            let err = installer(&ReseauGog::essai(&s.base_url()), &FauxJeton::nouveau(), &pc, &plan, &arret, &annulable, &rien, &|_| {}).await.unwrap_err();
            assert!(err.to_string().contains(&attendu), "{issue:?} : {err}");
            assert!(plan.installeurs.join("21").join("setup_jeu.exe").is_file(), "{issue:?} : installeur gardé");
            assert!(!plan.dest.join(DOSSIER_MARQUE).exists(), "{issue:?} : jamais marqué");
            assert_eq!(pc.0.lances.lock().unwrap().len(), 1, "{issue:?} : le DLC n'est pas tenté");
        }
    }

    #[tokio::test]
    async fn un_installeur_non_signe_n_est_jamais_lance_et_un_jeu_deja_la_est_refuse() {
        let s = MockServer::start_async().await;
        servir_jeu_et_dlc(&s);
        let racine = tempfile::tempdir().unwrap();
        let plan = plan_essai(&s, racine.path());
        let pc = FauxPc::nouveau(false, vec![], true);
        let (arret, annulable) = (AtomicBool::new(false), AtomicBool::new(true));
        let err = installer(&ReseauGog::essai(&s.base_url()), &FauxJeton::nouveau(), &pc, &plan, &arret, &annulable, &rien, &|_| {}).await.unwrap_err();
        assert!(err.to_string().contains("pas signé par GOG"), "{err}");
        assert!(pc.0.lances.lock().unwrap().is_empty());

        // Déjà au registre, ou dossier non vide : refus avant tout téléchargement.
        let pc = FauxPc::nouveau(true, vec![], true);
        pc.0.registre.lock().unwrap().insert("21".into(), Valeurs::new());
        let err = installer(&ReseauGog::essai("http://127.0.0.1:9"), &FauxJeton::nouveau(), &pc, &plan, &arret, &annulable, &rien, &|_| {}).await.unwrap_err();
        assert!(err.to_string().contains("déjà installé"), "{err}");
        let pc = FauxPc::nouveau(true, vec![], true);
        std::fs::create_dir_all(&plan.dest).unwrap();
        std::fs::write(plan.dest.join("x"), b"x").unwrap();
        let err = installer(&ReseauGog::essai("http://127.0.0.1:9"), &FauxJeton::nouveau(), &pc, &plan, &arret, &annulable, &rien, &|_| {}).await.unwrap_err();
        assert!(err.to_string().contains("pas vide"), "{err}");
    }

    #[tokio::test]
    async fn un_dlc_qui_echoue_n_enleve_pas_le_jeu_et_une_annulation_n_installe_rien() {
        let s = MockServer::start_async().await;
        servir_jeu_et_dlc(&s);
        let racine = tempfile::tempdir().unwrap();
        let plan = plan_essai(&s, racine.path());
        let pc = FauxPc::nouveau(true, vec![IssueInno::Code(0), IssueInno::Refuse], true);
        let (arret, annulable) = (AtomicBool::new(false), AtomicBool::new(true));
        let b = installer(&ReseauGog::essai(&s.base_url()), &FauxJeton::nouveau(), &pc, &plan, &arret, &annulable, &rien, &|_| {}).await.unwrap();
        assert_eq!(b.dlcs_echoues, vec![("Jeu 22".to_string(), "tu as répondu Non à Windows".to_string())]);
        assert!(plan.dest.join(DOSSIER_MARQUE).join(FICHIER_MARQUE).is_file(), "le jeu reste installé et marqué");
        assert!(plan.installeurs.join("22").join("setup_dlc.exe").is_file(), "l'installeur du DLC est gardé");
        assert!(!plan.installeurs.join("21").exists());

        let racine = tempfile::tempdir().unwrap();
        let plan = plan_essai(&s, racine.path());
        let pc = FauxPc::nouveau(true, vec![], true);
        let (arret, annulable) = (AtomicBool::new(true), AtomicBool::new(true));
        let b = installer(&ReseauGog::essai(&s.base_url()), &FauxJeton::nouveau(), &pc, &plan, &arret, &annulable, &rien, &|_| {}).await.unwrap();
        assert!(b.pause);
        assert!(pc.0.lances.lock().unwrap().is_empty(), "arrêté pendant le téléchargement : aucun installeur lancé");
    }

    #[test]
    fn seuls_les_fichiers_de_la_note_sont_effaces() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("setup.exe"), b"1").unwrap();
        std::fs::write(d.path().join("setup-1.bin"), b"2").unwrap();
        std::fs::write(d.path().join("a-moi.txt"), b"3").unwrap();
        // Un fichier de la note qui n'a plus la taille notée : gardé (rien ne s'efface sans preuve).
        std::fs::write(d.path().join("setup-2.bin"), b"autre taille").unwrap();
        let note = NoteInstalleur { element: "1".into(), installeur: "i".into(), version: "v".into(), fichiers: vec![Recu { nom: "setup.exe".into(), taille: 1, md5: String::new() }, Recu { nom: "setup-1.bin".into(), taille: 1, md5: String::new() }, Recu { nom: "setup-2.bin".into(), taille: 1, md5: String::new() }, Recu { nom: "../a-moi.txt".into(), taille: 1, md5: String::new() }] };
        std::fs::write(d.path().join(NOTE_INSTALLEUR), serde_json::to_vec(&note).unwrap()).unwrap();
        assert_eq!(effacer_installeur(d.path()), 2);
        assert!(d.path().join("a-moi.txt").is_file(), "un fichier qui n'est pas dans la note reste");
        assert!(d.path().join("setup-2.bin").is_file(), "une autre taille que celle notée : gardé");
        assert!(d.path().is_dir(), "dossier non vide : gardé");
        let vide = tempfile::tempdir().unwrap();
        assert_eq!(effacer_installeur(vide.path()), 0, "sans note, rien n'est effacé");
    }

    #[test]
    fn la_verification_d_un_jeu_installe_suit_le_registre_et_goggame_info() {
        let d = tempfile::tempdir().unwrap();
        let dest = d.path().join("Jeu");
        let pc = FauxPc::nouveau(true, vec![], true);
        assert_eq!(verifier_jeu(&pc, &dest, "21"), Err("pas d'entrée GOG au registre"));
        let mut v = Valeurs::new();
        v.insert("path".into(), d.path().join("Autre").display().to_string());
        pc.0.registre.lock().unwrap().insert("21".into(), v.clone());
        assert_eq!(verifier_jeu(&pc, &dest, "21"), Err("le registre nomme un autre dossier"));
        v.insert("path".into(), format!("{}\\", dest.display()));
        pc.0.registre.lock().unwrap().insert("21".into(), v.clone());
        assert_eq!(verifier_jeu(&pc, &dest, "21"), Err("la lecture des jeux GOG ne le garde pas"), "dossier absent");
        std::fs::create_dir_all(dest.join("bin")).unwrap();
        assert_eq!(verifier_jeu(&pc, &dest, "21"), Err("pas de fichier goggame.info"));
        // Sans tâche de jeu, le programme noté au registre suffit (la voie de secours du lancement de C4c-1).
        std::fs::write(dest.join("bin").join("jeu.exe"), b"MZ").unwrap();
        v.insert("exe".into(), dest.join("bin").join("jeu.exe").display().to_string());
        pc.0.registre.lock().unwrap().insert("21".into(), v);
        assert_eq!(verifier_jeu(&pc, &dest, "21"), Ok("registre"));
        std::fs::write(dest.join("goggame-21.info"), br#"{"gameId":"21","playTasks":[{"type":"FileTask","category":"game","path":"bin\\jeu.exe"}]}"#).unwrap();
        assert_eq!(verifier_jeu(&pc, &dest, "21"), Ok("game"));
        assert!(!verifier_dlc(&pc, &dest, "22"));
        std::fs::write(dest.join("goggame-22.info"), b"{}").unwrap();
        assert!(verifier_dlc(&pc, &dest, "22"));
    }

    #[tokio::test]
    async fn installe_pendant_que_frogtend_etait_ferme_le_jeu_est_verifie_marque_et_son_installeur_efface() {
        let s = MockServer::start_async().await;
        let racine = tempfile::tempdir().unwrap();
        let plan = plan_essai(&s, racine.path());
        // Ce que laisse une installation finie sans Frogtend : le jeu, son entrée au registre, l'installeur et ses notes.
        let pc = FauxPc::nouveau(true, vec![], true);
        let dossier = plan.installeurs.join("21");
        std::fs::create_dir_all(&dossier).unwrap();
        std::fs::write(dossier.join("setup_jeu.exe"), b"12345").unwrap();
        let note = NoteInstalleur { element: "21".into(), installeur: "i".into(), version: "1.0".into(), fichiers: vec![Recu { nom: "setup_jeu.exe".into(), taille: 5, md5: "a".repeat(32) }] };
        std::fs::write(dossier.join(NOTE_INSTALLEUR), serde_json::to_vec(&note).unwrap()).unwrap();
        pc.executer_eleve(&dossier.join("setup_jeu.exe"), &parametres_inno(&plan.dest, &dossier.join(JOURNAL_INNO)).unwrap(), &dossier);
        let (arret, annulable) = (AtomicBool::new(false), AtomicBool::new(true));
        // Sans la note « en cours », c'est un jeu installé par quelqu'un d'autre : refus.
        let err = installer(&ReseauGog::essai("http://127.0.0.1:9"), &FauxJeton::nouveau(), &pc, &plan, &arret, &annulable, &rien, &|_| {}).await.unwrap_err();
        assert!(err.to_string().contains("déjà installé"), "{err}");
        assert!(dossier.join("setup_jeu.exe").is_file());
        // Avec elle : vérifié, marqué, installeur effacé, sans rien télécharger ni lancer de plus.
        std::fs::write(dossier.join(NOTE_EN_COURS), serde_json::to_vec(&EnCours { dest: plan.dest.display().to_string() }).unwrap()).unwrap();
        assert!(rattrapable(&pc, &plan));
        let b = installer(&ReseauGog::essai("http://127.0.0.1:9"), &FauxJeton::nouveau(), &pc, &plan, &arret, &annulable, &rien, &|_| {}).await.unwrap();
        assert!(b.rattrape && !b.pause && !b.dossier_protege);
        let marque: Marque = serde_json::from_slice(&std::fs::read(plan.dest.join(DOSSIER_MARQUE).join(FICHIER_MARQUE)).unwrap()).unwrap();
        assert_eq!(marque.md5, vec!["a".repeat(32)]);
        assert!(!dossier.exists(), "installeur effacé (seulement ses fichiers, puis le dossier vide)");
        assert_eq!(pc.0.lances.lock().unwrap().len(), 1, "aucun installeur relancé");
    }

    #[tokio::test]
    async fn un_dossier_ou_frogtend_ne_peut_pas_ecrire_est_dit_dans_le_bilan() {
        let s = MockServer::start_async().await;
        servir_jeu_et_dlc(&s);
        let racine = tempfile::tempdir().unwrap();
        let mut plan = plan_essai(&s, racine.path());
        plan.dlcs.clear();
        let pc = FauxPc::nouveau(true, vec![], true);
        pc.0.bloque.store(true, Ordering::SeqCst);
        let (arret, annulable) = (AtomicBool::new(false), AtomicBool::new(true));
        let b = installer(&ReseauGog::essai(&s.base_url()), &FauxJeton::nouveau(), &pc, &plan, &arret, &annulable, &rien, &|_| {}).await.unwrap();
        assert!(b.dossier_protege, "jamais tu");
        assert!(!plan.installeurs.join("21").join("setup_jeu.exe").exists(), "le jeu est vérifié : son installeur est effacé quand même");
    }

    #[test]
    fn un_installeur_arrete_dit_ce_qu_il_reste_a_faire() {
        let d = tempfile::tempdir().unwrap();
        let dest = d.path().join("Jeu");
        assert!(message_inacheve("Arrêté.", &dest, true).contains("désinstalle-la depuis Windows (Paramètres › Applications)"));
        std::fs::create_dir_all(&dest).unwrap();
        assert!(message_inacheve("Arrêté.", &dest, false).contains("Rien n'a été effacé"), "dossier vide : rien à retirer");
        std::fs::write(dest.join("x"), b"x").unwrap();
        assert!(message_inacheve("Arrêté.", &dest, false).contains("Des fichiers sont restés dans le dossier"));
        assert!(message_inacheve("Arrêté.", &dest, false).contains("retire ce dossier"));
        assert!(message_inacheve("Arrêté.", &d.path().join("absent"), false).contains("⬇ Installer pour réessayer"));
    }

    #[tokio::test]
    async fn un_fichier_deja_verifie_n_est_pas_relu_sauf_s_il_a_change() {
        let s = MockServer::start_async().await;
        let d = donnees(7, 25);
        servir(&s, "31", "f0", "setup.exe", &d, "VIEUX");
        let dir = tempfile::tempdir().unwrap();
        let arret = AtomicBool::new(false);
        let av = Avancement { arret: &arret, progres: &rien, total: 25 };
        let reseau = ReseauGog::essai(&s.base_url());
        let mut deja = 0;
        telecharger_installeur(&reseau, &reseau.client_api().unwrap(), &reseau.client_cdn().unwrap(), &FauxJeton::nouveau(), &element(&s, "31", "f0", 25), dir.path(), &av, &mut deja).await.unwrap();
        assert!(dir.path().join(format!("setup.exe{SUFFIXE_VERIFIE}")).is_file());
        // Un 2e GOG qui ne sert PAS le fichier : la reprise ne le redemande pas (ni ne le relit).
        let s2 = MockServer::start_async().await;
        let base2 = s2.base_url();
        s2.mock(|w, t| {
            w.method(GET).path("/products/31/downlink/installer/f0");
            t.status(200).json_body(json!({"downlink": format!("{base2}/cdn/f"), "checksum": format!("{base2}/cdn/f.xml")}));
        });
        let xml = xml_de("setup.exe", &d, 10);
        s2.mock(|w, t| {
            w.method(GET).path("/cdn/f.xml");
            t.status(200).body(xml.clone());
        });
        let rien_du_cdn = s2.mock(|w, t| {
            w.method(GET).path("/cdn/f");
            t.status(404);
        });
        let reseau2 = ReseauGog::essai(&base2);
        let mut deja = 0;
        let r = telecharger_installeur(&reseau2, &reseau2.client_api().unwrap(), &reseau2.client_cdn().unwrap(), &FauxJeton::nouveau(), &element(&s2, "31", "f0", 25), dir.path(), &av, &mut deja).await.unwrap();
        assert!(matches!(r, Issue::Fini(_)));
        rien_du_cdn.assert_hits_async(0).await;
        // Changé sur le disque (une autre date) : revérifié ; un morceau faux est redemandé (ici refusé par ce GOG).
        let mut abime = d.clone();
        abime[21] ^= 1;
        std::fs::write(dir.path().join("setup.exe"), &abime).unwrap();
        let mut deja = 0;
        assert!(telecharger_installeur(&reseau2, &reseau2.client_api().unwrap(), &reseau2.client_cdn().unwrap(), &FauxJeton::nouveau(), &element(&s2, "31", "f0", 25), dir.path(), &av, &mut deja).await.is_err());
        assert_eq!(std::fs::metadata(dir.path().join("setup.exe.part")).unwrap().len(), 20, "coupé au début du morceau abîmé, le début vérifié gardé");
    }

    #[test]
    fn un_dlc_prend_d_abord_la_langue_de_l_installeur_du_jeu() {
        let api = "https://api.gog.com";
        let mut v = produit(api);
        let f = |l: &str| json!({"id": format!("installer_windows_{l}"), "os": "windows", "language": l, "total_size": 7, "files": [{"id": format!("{l}1installer0"), "size": 7, "downlink": format!("{api}/products/1597316373/downlink/installer/{l}1installer0")}]});
        v["expanded_dlcs"][0]["downloads"]["installers"] = json!([f("fr"), f("en")]);
        let p = lire_produit_installation(&v, &["1597316373".into()], api).unwrap();
        assert_eq!(p.dlcs[0].installeur.langue, "en", "le jeu n'a qu'un installeur anglais : son DLC aussi");
    }
}
