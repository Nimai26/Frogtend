//! Lot C5 (décision de Seb du 21/11) : les jeux GOG du compte, « comme dans GOG Galaxy ». La liste du site (88 jeux
//! chez Seb) n'a pas tout ce que Galaxy montre (197) : Frogtend se connecte à GOG comme GOG Galaxy (méthode de Heroic,
//! Playnite, Lutris), avec l'identifiant PUBLIC de Galaxy, lit la bibliothèque de Galaxy (`galaxy-library.gog.com`), puis
//! la base publique de GOG (`gamesdb.gog.com`) pour le titre, le type et la jaquette de chaque jeu.
//!
//! Secrets : le jeton de renouvellement (`refresh_token`) vit dans le coffre de Windows seulement ; le jeton d'accès
//! (`access_token`) et le code de connexion ne vivent qu'en mémoire. Aucun n'apparaît jamais dans un fichier, un journal,
//! un message, un `Debug` ni l'interface. Les erreurs réseau sont rendues SANS l'adresse (elle porte le code ou le jeton).
//!
//! Formes relevées le 21/11 sur gamesdb (lecture publique, sans compte) : voir `lire_fiche`. Formes NON vérifiées (pas de
//! compte utilisé pour les relever) : la redirection de la connexion, la réponse du jeton, la bibliothèque de Galaxy —
//! d'après la documentation de la communauté (Heroic, Playnite, Lutris, « gogapidocs »).

use crate::boutiques::JeuBoutique;
use crate::erreurs::{Erreur, Resultat};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// L'identifiant PUBLIC de l'application GOG Galaxy (le même dans Heroic, Playnite, Lutris).
pub const CLIENT_ID: &str = "46899977096215655";
/// Le « secret » PUBLIC de GOG Galaxy (publié dans ces mêmes logiciels : ce n'est pas un secret de la personne).
pub const CLIENT_SECRET: &str = "9d85c43b1482497dbbce61f6e4aa173a433796eeae2ca8c5f6129f2dc4de46d9";
/// Où GOG renvoie après la connexion (avec `?code=…`) : cette navigation est interceptée et ANNULÉE.
pub const REDIRECTION: &str = "https://embed.gog.com/on_login_success?origin=client";
pub const AUTH: &str = "https://auth.gog.com";
pub const BIBLIOTHEQUE: &str = "https://galaxy-library.gog.com";
pub const GAMESDB: &str = "https://gamesdb.gog.com";

/// Le nom du jeton de renouvellement au coffre de Windows (`boutiques::nom_secret(profil, SECRET)`) ; oublié avec le
/// profil (`noyau::supprimer_profil`).
pub const SECRET: &str = "gog-renouvellement";

/// Le message quand la connexion façon Galaxy ne vaut plus (jeton refusé) : la lecture repasse par la page.
pub const MOTIF_RECONNECTER_GOG: &str = "GOG ne te reconnaît plus : reconnecte-toi à GOG dans 🛒 Boutiques.";

/// Le plus grand nombre de pages de la bibliothèque lues (des centaines de jeux par page : bien au-delà d'un compte).
pub const PAGES_BIBLIOTHEQUE_MAX: usize = 100;
/// Le plus grand nombre de fiches demandées à gamesdb en une lecture (les suivantes le seront à la lecture d'après).
pub const FICHES_PAR_LECTURE_MAX: usize = 1500;
/// La pause entre deux fiches de gamesdb (lecture polie).
pub const PAUSE_FICHES_MS: u64 = 150;

/// L'adresse de la connexion façon Galaxy (page officielle de GOG ; silencieuse si la session GOG est ouverte).
pub fn adresse_connexion() -> String {
    tauri::Url::parse_with_params(
        &format!("{AUTH}/auth"),
        &[("client_id", CLIENT_ID), ("redirect_uri", REDIRECTION), ("response_type", "code"), ("layout", "client2")],
    )
    .map(|u| u.to_string())
    .unwrap_or_default()
}

/// Le code de connexion rendu par GOG : son `Debug` ne montre RIEN.
#[derive(Clone, PartialEq)]
pub struct CodeGog(String);

impl std::fmt::Debug for CodeGog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CodeGog(<caché>)")
    }
}

/// Ce que la redirection de fin de connexion dit.
#[derive(Debug, PartialEq)]
pub enum RetourConnexion {
    Code(CodeGog),
    /// Pas de code (refus, erreur de GOG) : l'étape, en caractères sûrs, pour le journal.
    SansCode(String),
}

/// Est-ce la redirection de fin de connexion (`https://embed.gog.com/on_login_success…`, ou `www.gog.com` : la
/// documentation hésite) ? Alors : son code (forme sûre) ou pourquoi il n'y en a pas. `None` : une autre adresse.
pub fn lire_retour_connexion(url: &tauri::Url) -> Option<RetourConnexion> {
    let hote = url.host_str()?.to_ascii_lowercase();
    if url.scheme() != "https" || url.port().is_some() || !url.username().is_empty() || url.password().is_some() || !["embed.gog.com", "www.gog.com"].contains(&hote.as_str()) || url.path() != "/on_login_success" {
        return None;
    }
    let code = url.query_pairs().find(|(k, _)| k == "code").map(|(_, v)| v.into_owned()).unwrap_or_default();
    if !code.is_empty() && code.len() <= 4096 && code.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.' | b'~')) {
        return Some(RetourConnexion::Code(CodeGog(code)));
    }
    let erreur: String = url.query_pairs().find(|(k, _)| k == "error").map(|(_, v)| v.into_owned()).unwrap_or_default().chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').take(40).collect();
    Some(RetourConnexion::SansCode(if !code.is_empty() {
        "code-de-forme-inattendue".into()
    } else if erreur.is_empty() {
        "sans-code".into()
    } else {
        format!("erreur-{erreur}")
    }))
}

/// Les fournisseurs d'identité que la page de connexion de GOG propose (relevé le 21/11 sans compte sur
/// login.gog.com : Google, Discord, Steam, Xbox, par `external-accounts.gog.com/login/providers/<nom>/init`), et leurs
/// pages de connexion (hôtes EXACTS ; Xbox = compte Microsoft).
const FOURNISSEURS_GOG: [&str; 7] = ["external-accounts.gog.com", "accounts.google.com", "discord.com", "steamcommunity.com", "login.live.com", "login.microsoftonline.com", "account.live.com"];

/// Où la fenêtre de connexion peut aller : en lecture DISCRÈTE, seulement les pages de GOG de la connexion ; MONTRÉE
/// (la personne se connecte), aussi les fournisseurs d'identité de la page de GOG (`FOURNISSEURS_GOG`), rien d'autre.
/// Toujours HTTPS, port normal, sans identifiant dans l'adresse.
pub fn navigation_connexion(url: &tauri::Url, montree: bool) -> bool {
    if url.scheme() != "https" || url.port().is_some() || !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    let h = url.host_str().unwrap_or("").to_ascii_lowercase();
    ["auth.gog.com", "login.gog.com", "embed.gog.com", "www.gog.com"].contains(&h.as_str()) || (montree && FOURNISSEURS_GOG.contains(&h.as_str()))
}

/// Les petites fenêtres qu'ouvre la page de connexion MONTRÉE (« Se connecter avec Google »…) : seulement vers ces
/// mêmes adresses, avec le navigateur de la fenêtre.
pub fn fenetre_de_fournisseur<R: tauri::Runtime>(url: tauri::Url, _f: tauri::webview::NewWindowFeatures) -> tauri::webview::NewWindowResponse<R> {
    if navigation_connexion(&url, true) {
        tauri::webview::NewWindowResponse::Allow
    } else {
        tauri::webview::NewWindowResponse::Deny
    }
}

/// Une page qui a FINI de se charger sur le site de connexion : GOG demande de se connecter (pas de session ouverte).
/// Avec une session, l'adresse de connexion redirige tout de suite, sans rien afficher.
pub fn page_de_connexion_affichee(url: &tauri::Url) -> bool {
    url.scheme() == "https" && url.host_str().is_some_and(|h| h.eq_ignore_ascii_case("login.gog.com") || h.eq_ignore_ascii_case("auth.gog.com"))
}

/// Les jetons de la connexion façon Galaxy : son `Debug` ne montre RIEN (ni jeton, ni compte).
#[derive(Clone, PartialEq)]
pub struct JetonsGog {
    pub acces: String,
    pub renouvellement: String,
    pub user_id: String,
}

impl std::fmt::Debug for JetonsGog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("JetonsGog(<caché>)")
    }
}

/// Un jeton de forme sûre (pas d'espace ni de caractère de contrôle, taille raisonnable).
fn jeton_sur(t: &str) -> bool {
    !t.is_empty() && t.len() <= 8192 && t.bytes().all(|c| c.is_ascii_graphic())
}

/// Lit la réponse de `auth.gog.com/token` : `{access_token, refresh_token, user_id, expires_in, …}` (forme supposée).
pub fn lire_jetons(v: &Value) -> Resultat<JetonsGog> {
    let texte = |k: &str| v[k].as_str().map(str::to_string).or_else(|| v[k].as_u64().map(|n| n.to_string())).unwrap_or_default();
    let (acces, renouvellement, user_id) = (texte("access_token"), texte("refresh_token"), texte("user_id"));
    if !jeton_sur(&acces) || !jeton_sur(&renouvellement) || !crate::gratuits::numero_gog(&user_id) {
        return Err(Erreur::Serveur("GOG a répondu une connexion illisible.".into()));
    }
    Ok(JetonsGog { acces, renouvellement, user_id })
}

/// Le compte de la connexion façon Galaxy doit être celui que la page de GOG montre connecté (jamais les jeux d'un
/// autre compte).
pub fn verifier_compte(j: &JetonsGog, compte: &str) -> Resultat<()> {
    if j.user_id == compte {
        Ok(())
    } else {
        Err(Erreur::Refus("La connexion façon Galaxy est celle d'un autre compte GOG que celui connecté sur gog.com.".into()))
    }
}

/// Le client réseau de GOG (sans cookies, sans redirection suivie) ; un seul pour toutes les fiches d'une lecture.
pub fn client() -> Resultat<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("Frogtend")
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| Erreur::Reseau("Connexion impossible.".into()))
}

/// Un statut de GOG en erreur : 400/401/403 = la connexion ne vaut plus ; 429 = attendre ; sinon panne.
fn erreur_gog(code: u16) -> Erreur {
    match code {
        400 | 401 | 403 => Erreur::Refus(MOTIF_RECONNECTER_GOG.into()),
        429 => Erreur::Reseau("GOG demande d'attendre un peu avant de réessayer.".into()),
        c => Erreur::Serveur(format!("GOG a répondu une erreur (code {c}).")),
    }
}

async fn demander_jetons(base: &str, parametres: &[(&str, &str)]) -> Resultat<JetonsGog> {
    let r = client()?
        .get(format!("{base}/token"))
        .query(parametres)
        .send()
        .await
        .map_err(|e| Erreur::Reseau(format!("GOG ne répond pas ({}).", e.without_url())))?;
    match r.status().as_u16() {
        200 => lire_jetons(&r.json::<Value>().await.map_err(|_| Erreur::Serveur("GOG a répondu une connexion illisible.".into()))?),
        c => Err(erreur_gog(c)),
    }
}

/// Échange le code de connexion contre les jetons.
pub async fn jetons_par_code(base: &str, code: &CodeGog) -> Resultat<JetonsGog> {
    demander_jetons(base, &[("client_id", CLIENT_ID), ("client_secret", CLIENT_SECRET), ("grant_type", "authorization_code"), ("code", &code.0), ("redirect_uri", REDIRECTION)]).await
}

/// Renouvelle les jetons avec le jeton de renouvellement (rangé au coffre).
pub async fn jetons_renouveles(base: &str, renouvellement: &str) -> Resultat<JetonsGog> {
    demander_jetons(base, &[("client_id", CLIENT_ID), ("client_secret", CLIENT_SECRET), ("grant_type", "refresh_token"), ("refresh_token", renouvellement)]).await
}

/// Une page de la bibliothèque de Galaxy (`{items: [{platform_id, external_id, owned…}], next_page_token}`, forme
/// supposée) : les numéros des jeux GOG (plateforme « gog »), et la page suivante. Un élément `owned: false` n'est pas
/// gardé. `None` : page illisible.
pub fn lire_page_bibliotheque(v: &Value) -> Option<(Vec<String>, usize, Option<String>)> {
    let items = v["items"].as_array()?;
    let mut ids = Vec::new();
    for it in items {
        if it["platform_id"].as_str() != Some("gog") || it["owned"].as_bool() == Some(false) {
            continue;
        }
        let id = it["external_id"].as_str().map(str::to_string).or_else(|| it["external_id"].as_u64().map(|n| n.to_string())).unwrap_or_default();
        if crate::gratuits::numero_gog(&id) {
            ids.push(id);
        }
    }
    let suivante = v["next_page_token"].as_str().filter(|t| !t.is_empty() && t.len() <= 4096).map(str::to_string);
    Some((ids, items.len(), suivante))
}

/// Ce que la bibliothèque de Galaxy a donné : les numéros des jeux GOG (dans l'ordre, sans doublon) et le nombre total
/// d'éléments (toutes plateformes), pour le journal.
#[derive(Debug, Default, PartialEq)]
pub struct Bibliotheque {
    pub ids: Vec<String>,
    pub elements: usize,
    pub pages: usize,
}

/// Lit TOUTE la bibliothèque de Galaxy du compte (une page en échec = échec : jamais une bibliothèque à moitié lue).
pub async fn lire_bibliotheque(base: &str, user_id: &str, acces: &str) -> Resultat<Bibliotheque> {
    if !crate::gratuits::numero_gog(user_id) {
        return Err(Erreur::Refus("Compte GOG illisible.".into()));
    }
    let c = client()?;
    let mut b = Bibliotheque::default();
    let mut vus = BTreeSet::new();
    let mut suivante: Option<String> = None;
    for _ in 0..PAGES_BIBLIOTHEQUE_MAX {
        let mut q = c.get(format!("{base}/users/{user_id}/releases")).bearer_auth(acces);
        if let Some(t) = &suivante {
            q = q.query(&[("page_token", t)]);
        }
        let r = q.send().await.map_err(|e| Erreur::Reseau(format!("La bibliothèque de GOG ne répond pas ({}).", e.without_url())))?;
        let statut = r.status().as_u16();
        if statut != 200 {
            return Err(erreur_gog(statut));
        }
        let v: Value = r.json().await.map_err(|_| Erreur::Serveur("La bibliothèque de GOG est illisible.".into()))?;
        let (ids, n, s) = lire_page_bibliotheque(&v).ok_or_else(|| Erreur::Serveur("La bibliothèque de GOG est illisible.".into()))?;
        b.pages += 1;
        b.elements += n;
        b.ids.extend(ids.into_iter().filter(|i| vus.insert(i.clone())));
        match s {
            Some(t) if suivante.as_ref() != Some(&t) => suivante = Some(t),
            Some(_) => return Err(Erreur::Serveur("La bibliothèque de GOG tourne en rond.".into())),
            None => return Ok(b),
        }
    }
    Err(Erreur::Serveur("La bibliothèque de GOG est trop longue pour être lue en entier.".into()))
}

/// Une fiche lue est redemandée après ce délai (titre ou jaquette changés chez GOG).
pub const FICHE_LUE_VALABLE_S: u64 = 60 * 86_400;
/// Une fiche INTROUVABLE (404) est redemandée après ce délai (la base de GOG se complète).
pub const FICHE_INTROUVABLE_VALABLE_S: u64 = 7 * 86_400;

/// La fiche d'un jeu GOG dans la base publique, gardée en cache dans `boutiques.json` (redemandée après
/// `FICHE_LUE_VALABLE_S`, ou `FICHE_INTROUVABLE_VALABLE_S` si la base ne la connaissait pas).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct FicheGog {
    #[serde(default)]
    pub titre: String,
    /// Le type de l'entrée (`type` : game, dlc, pack, spam…).
    #[serde(default)]
    pub genre: String,
    /// Le type du JEU auquel elle renvoie (`game.type`).
    #[serde(default)]
    pub genre_du_jeu: String,
    /// Le jeu auquel l'entrée renvoie (`game_id`) : une seule carte par jeu.
    #[serde(default)]
    pub jeu: String,
    /// `game.visible_in_library` (GOG Galaxy n'affiche pas un jeu marqué invisible).
    #[serde(default)]
    pub visible: Option<bool>,
    /// La jaquette verticale (`images.gog.com`, forme normalisée).
    #[serde(default)]
    pub image: Option<String>,
    /// La base ne connaissait pas ce numéro (404) : le jeu est GARDÉ comme une fiche en échec.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub introuvable: bool,
    /// Quand la fiche a été demandée (secondes depuis 1970) ; 0 : inconnu (redemandée).
    #[serde(default)]
    pub lu_le: u64,
}

/// La jaquette verticale d'après un `url_format` de gamesdb (relevé le 21/11 :
/// « https://images.gog.com/<empreinte>{formatter}.{ext}?namespace=gamesdb ») → « …<empreinte>_glx_vertical_cover.webp
/// ?namespace=gamesdb » (la forme que GOG Galaxy garde lui-même). Une autre forme : `None`.
pub fn image_verticale(format: &str) -> Option<String> {
    let reste = format.strip_prefix("https://images.gog.com/")?;
    let (empreinte, suite) = reste.split_once("{formatter}.{ext}")?;
    if !(16..=128).contains(&empreinte.len()) || !empreinte.bytes().all(|c| c.is_ascii_alphanumeric()) || !(suite.is_empty() || suite == "?namespace=gamesdb") {
        return None;
    }
    let u = format!("https://images.gog.com/{empreinte}_glx_vertical_cover.webp{suite}");
    crate::boutiques::image_autorisee(&u).then_some(u)
}

/// Lit une fiche de `gamesdb.gog.com/platforms/gog/external_releases/<id>` (formes relevées le 21/11 sur 1209025141,
/// 1423049311, 1272473144, 1548764757, 1207664663…) : `type` (« game », « dlc », « spam »), `game_id`, `parent_id`
/// (non nul pour un DLC), `title["*"]`, et `game{type, title["*"], visible_in_library, vertical_cover{url_format}, cover}`.
/// Le titre du JEU d'abord (celui de l'entrée est parfois moins propre), sans ™. `None` : forme inconnue.
pub fn lire_fiche(v: &Value) -> Option<FicheGog> {
    let g = &v["game"];
    let titre = [&g["title"]["*"], &v["title"]["*"], &v["title"]["en-US"]].iter().filter_map(|t| t.as_str()).map(crate::boutiques_pc::sans_marques).find(|t| !t.is_empty())?;
    let genre = v["type"].as_str().unwrap_or("").to_string();
    if genre.is_empty() && g["type"].as_str().is_none() {
        return None;
    }
    let image = [&g["vertical_cover"]["url_format"], &g["cover"]["url_format"]].iter().filter_map(|f| f.as_str()).find_map(image_verticale);
    Some(FicheGog {
        titre: titre.chars().take(300).collect(),
        genre,
        genre_du_jeu: g["type"].as_str().unwrap_or("").to_string(),
        jeu: v["game_id"].as_str().or(g["id"].as_str()).unwrap_or("").chars().filter(char::is_ascii_digit).take(30).collect(),
        visible: g["visible_in_library"].as_bool(),
        image,
        introuvable: false,
        lu_le: 0,
    })
}

/// Pourquoi une fiche n'a pas pu être lue.
#[derive(Debug, PartialEq)]
pub enum EchecFiche {
    /// GOG demande d'attendre : on arrête de demander jusqu'à la prochaine lecture, en gardant ce qui est résolu.
    TropDeDemandes,
    /// Panne, réseau, réponse illisible : le jeu est GARDÉ avec son titre connu (ou provisoire).
    Autre,
}

/// La fiche publique d'un jeu (sans jeton ni cookie), datée `maintenant`. Un 404 donne une fiche `introuvable`.
pub async fn fiche(client: &reqwest::Client, base: &str, id: &str, maintenant: u64) -> Result<FicheGog, EchecFiche> {
    if !crate::gratuits::numero_gog(id) {
        return Err(EchecFiche::Autre);
    }
    let r = client.get(format!("{base}/platforms/gog/external_releases/{id}")).send().await.map_err(|_| EchecFiche::Autre)?;
    let f = match r.status().as_u16() {
        200 => r.json::<Value>().await.ok().as_ref().and_then(lire_fiche).ok_or(EchecFiche::Autre)?,
        404 => FicheGog { introuvable: true, ..Default::default() },
        429 => return Err(EchecFiche::TropDeDemandes),
        _ => return Err(EchecFiche::Autre),
    };
    Ok(FicheGog { lu_le: maintenant, ..f })
}

/// La règle de Frogtend (lot C5) pour un jeu qui ne vient QUE de la bibliothèque de Galaxy (un jeu que la liste du site
/// montre est toujours gardé, `choisir`) : d'après le type du JEU auquel l'entrée renvoie (`game.type`, sinon `type`) —
/// « game » gardé (sauf marqué invisible par GOG) ; « pack » gardé seulement si GOG le montre dans la bibliothèque ;
/// « dlc » écarté (rangé sous son jeu) ; « spam » écarté (licences « … - Amazon Prime / Luna », anciennes éditions
/// remplacées : relevé le 21/11, leur `game` est lui-même « spam » et invisible) ; tout autre type écarté. Une fiche
/// introuvable n'est PAS un écart (le jeu est gardé, comme une fiche en échec). Rend la raison d'un écart.
pub fn raison_ecart(f: &FicheGog) -> Option<&'static str> {
    if f.introuvable {
        return None;
    }
    let t = if f.genre_du_jeu.is_empty() { f.genre.as_str() } else { f.genre_du_jeu.as_str() };
    match t {
        "game" if f.visible == Some(false) => Some("invisible"),
        "game" => None,
        "pack" if f.visible == Some(true) => None,
        "pack" => Some("pack"),
        "dlc" => Some("dlc"),
        "spam" => Some("licence"),
        _ => Some("autre-type"),
    }
}

/// Le bilan d'un choix (journal : des nombres et des titres de jeux, jamais de données de compte).
#[derive(Debug, Default, PartialEq)]
pub struct BilanChoix {
    /// Par type (`game`, `dlc`, `spam`…, « introuvable », « en-echec »).
    pub par_type: BTreeMap<String, usize>,
    pub gardes: usize,
    /// « raison : titre » des entrées écartées.
    pub ecartes: Vec<String>,
    /// « raison : titre » des entrées que la règle écarterait, GARDÉES parce que la liste du site les montre.
    pub gardes_par_le_site: Vec<String>,
    /// Les numéros dont la fiche n'a pas pu être lue, ou que la base ne connaît pas (gardés avec un titre connu).
    pub en_echec: Vec<String>,
    /// Les jeux de la liste du site absents de la bibliothèque (gardés aussi).
    pub hors_bibliotheque: usize,
}

/// Un jeu déjà connu (liste du site, liste rangée) sous son numéro.
fn connu<'a>(id: &str, listes: &[&'a [JeuBoutique]]) -> Option<&'a JeuBoutique> {
    let cle = format!("gog_{id}");
    listes.iter().flat_map(|l| l.iter()).find(|j| j.id == cle)
}

/// Le titre d'un jeu dont la fiche manque : celui déjà connu, sinon un titre provisoire.
fn titre_connu(id: &str, listes: &[&[JeuBoutique]]) -> String {
    let cle = format!("gog_{id}");
    listes
        .iter()
        .flat_map(|l| l.iter())
        .find(|j| j.id == cle && !j.nom.trim().is_empty() && j.nom != j.id && !j.nom.starts_with("Jeu GOG "))
        .map(|j| j.nom.clone())
        .unwrap_or_else(|| format!("Jeu GOG {id}"))
}

/// Les jeux du compte d'après la bibliothèque de Galaxy (pur, testé) :
/// - un numéro que la liste du SITE (`page`) montre est toujours gardé (la règle des types ne l'écarte jamais ; l'écart
///   évité va au journal) ; les autres suivent `raison_ecart` ;
/// - un numéro sans fiche (lecture ratée) ou introuvable dans la base est GARDÉ avec son titre connu (site, liste rangée
///   `ranges`) ou provisoire ;
/// - une seule carte par jeu (`jeu` de la fiche) : d'abord le numéro déjà rangé, puis celui du site, puis l'entrée
///   « game », sinon la première ;
/// - puis les jeux du site que la bibliothèque n'a pas du tout (garde-fou). Ids de Galaxy : `gog_<numéro>`.
pub fn choisir(ids: &[String], fiches: &BTreeMap<String, FicheGog>, page: &[JeuBoutique], ranges: &[JeuBoutique]) -> (Vec<JeuBoutique>, BilanChoix) {
    let mut b = BilanChoix::default();
    let sur_le_site = |id: &str| connu(id, &[page]).is_some();
    let deja_range = |id: &str| connu(id, &[ranges]).is_some();
    let titre_de = |f: &FicheGog, id: &str| if f.titre.is_empty() { format!("n° {id}") } else { f.titre.clone() };
    // (numéro, fiche lisible ou non) gardés, dans l'ordre.
    let mut gardes: Vec<(String, Option<FicheGog>)> = Vec::new();
    for id in ids {
        match fiches.get(id) {
            None => {
                *b.par_type.entry("en-echec".into()).or_default() += 1;
                b.en_echec.push(id.clone());
                gardes.push((id.clone(), None));
            }
            Some(f) if f.introuvable => {
                *b.par_type.entry("introuvable".into()).or_default() += 1;
                b.en_echec.push(id.clone());
                gardes.push((id.clone(), None));
            }
            Some(f) => {
                let t = if f.genre.is_empty() { f.genre_du_jeu.clone() } else { f.genre.clone() };
                *b.par_type.entry(t).or_default() += 1;
                match raison_ecart(f) {
                    Some(r) if sur_le_site(id) => {
                        b.gardes_par_le_site.push(format!("{r} : {}", titre_de(f, id)));
                        gardes.push((id.clone(), Some(f.clone())));
                    }
                    Some(r) => b.ecartes.push(format!("{r} : {}", titre_de(f, id))),
                    None => gardes.push((id.clone(), Some(f.clone()))),
                }
            }
        }
    }
    // Une seule carte par jeu : le numéro déjà rangé, puis celui du site, puis l'entrée « game ».
    let rang = |id: &str, f: &Option<FicheGog>| (deja_range(id), sur_le_site(id), f.as_ref().is_some_and(|f| f.genre == "game"));
    let mut par_jeu: BTreeMap<String, usize> = BTreeMap::new();
    let mut retenus: Vec<Option<(String, Option<FicheGog>)>> = Vec::new();
    for (id, f) in gardes {
        let cle = f.as_ref().map(|f| f.jeu.clone()).filter(|j| !j.is_empty());
        let titre = |f: &Option<FicheGog>, id: &str| f.as_ref().map(|f| titre_de(f, id)).unwrap_or_else(|| format!("n° {id}"));
        match cle.as_ref().and_then(|c| par_jeu.get(c).copied()) {
            Some(i) => {
                let actuel = retenus[i].as_ref().map(|(a, fa)| rang(a, fa)).unwrap_or_default();
                if rang(&id, &f) > actuel {
                    if let Some((ancien, fa)) = retenus[i].take() {
                        b.ecartes.push(format!("doublon : {}", titre(&fa, &ancien)));
                    }
                    retenus[i] = Some((id, f));
                } else {
                    b.ecartes.push(format!("doublon : {}", titre(&f, &id)));
                }
            }
            None => {
                if let Some(c) = cle {
                    par_jeu.insert(c, retenus.len());
                }
                retenus.push(Some((id, f)));
            }
        }
    }
    let mut l: Vec<JeuBoutique> = retenus
        .into_iter()
        .flatten()
        .map(|(id, f)| {
            let ancien = connu(&id, &[page, ranges]);
            let (nom, image) = match &f {
                Some(f) => (f.titre.clone(), f.image.clone().or_else(|| ancien.and_then(|j| j.image.clone()))),
                None => (titre_connu(&id, &[page, ranges]), ancien.and_then(|j| j.image.clone())),
            };
            JeuBoutique {
                id: format!("gog_{id}"),
                nom,
                minutes: 0,
                derniere: 0,
                installe: false,
                plateforme: "gog".into(),
                image,
                dossier: None,
                pret: None,
                succes: None,
                releases: vec![],
                titre_douteux: false,
            }
        })
        .collect();
    b.gardes = l.len();
    // Le garde-fou : un jeu de la liste du site que la bibliothèque n'a pas du tout reste.
    let dans_bibliotheque: BTreeSet<String> = ids.iter().map(|i| format!("gog_{i}")).collect();
    for j in page.iter().filter(|j| !dans_bibliotheque.contains(&j.id)) {
        if !l.iter().any(|x| x.id == j.id) {
            b.hors_bibliotheque += 1;
            l.push(j.clone());
        }
    }
    (l, b)
}

/// La bibliothèque de Galaxy n'a pas été lue EN ENTIER : la liste du site, PLUS les jeux GOG déjà rangés qu'elle n'a
/// pas (jamais une liste qui rétrécit sur une lecture ratée ; pur, testé).
pub fn union_sans_bibliotheque(page: Vec<JeuBoutique>, ranges: &[JeuBoutique]) -> Vec<JeuBoutique> {
    let mut l = page;
    for j in ranges.iter().filter(|j| j.id.strip_prefix("gog_").is_some_and(crate::gratuits::numero_gog)) {
        if !l.iter().any(|x| x.id == j.id) {
            l.push(j.clone());
        }
    }
    l
}

/// Les numéros dont la fiche est à demander : ceux de la bibliothèque sans fiche en cache, ou dont la fiche est
/// périmée (lue depuis plus de `FICHE_LUE_VALABLE_S`, introuvable depuis plus de `FICHE_INTROUVABLE_VALABLE_S`).
pub fn fiches_a_demander(ids: &[String], cache: &BTreeMap<String, FicheGog>, maintenant: u64) -> Vec<String> {
    ids.iter()
        .filter(|i| match cache.get(*i) {
            None => true,
            Some(f) => {
                let valable = if f.introuvable { FICHE_INTROUVABLE_VALABLE_S } else { FICHE_LUE_VALABLE_S };
                f.lu_le == 0 || maintenant.saturating_sub(f.lu_le) > valable
            }
        })
        .take(FICHES_PAR_LECTURE_MAX)
        .cloned()
        .collect()
}

/// Le texte du bilan pour le journal (des nombres et des titres de jeux ; 300 titres écartés au plus).
pub fn texte_bilan(bib: &Bibliotheque, b: &BilanChoix, demandees: usize, total: usize) -> String {
    let types: Vec<String> = b.par_type.iter().map(|(t, n)| format!("{t} {n}")).collect();
    let mut ecartes: Vec<&str> = b.ecartes.iter().map(String::as_str).take(300).collect();
    if b.ecartes.len() > 300 {
        ecartes.push("…");
    }
    format!(
        "bibliothèque de Galaxy lue ({} page(s), {} élément(s), {} jeu(x) GOG) ; {demandees} fiche(s) demandée(s) ; types : {} ; gardés {} ; écartés {} [{}] ; gardés car sur le site {} [{}] ; fiches en échec ou introuvables {} (gardés) ; jeux du site hors bibliothèque {} ; total {total}",
        bib.pages,
        bib.elements,
        bib.ids.len(),
        types.join(", "),
        b.gardes,
        b.ecartes.len(),
        ecartes.join(" | "),
        b.gardes_par_le_site.len(),
        b.gardes_par_le_site.iter().take(100).map(String::as_str).collect::<Vec<_>>().join(" | "),
        b.en_echec.len(),
        b.hors_bibliotheque
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use serde_json::json;

    fn u(s: &str) -> tauri::Url {
        tauri::Url::parse(s).unwrap()
    }

    #[test]
    fn l_adresse_de_connexion_est_celle_de_galaxy() {
        let a = u(&adresse_connexion());
        assert_eq!((a.scheme(), a.host_str(), a.path()), ("https", Some("auth.gog.com"), "/auth"));
        let q: BTreeMap<String, String> = a.query_pairs().into_owned().collect();
        assert_eq!(q["client_id"], CLIENT_ID);
        assert_eq!(q["redirect_uri"], REDIRECTION);
        assert_eq!((q["response_type"].as_str(), q["layout"].as_str()), ("code", "client2"));
    }

    #[test]
    fn la_redirection_de_fin_de_connexion_est_lue_strictement() {
        let code = |s: &str| match lire_retour_connexion(&u(s)) {
            Some(RetourConnexion::Code(c)) => Some(c.0),
            _ => None,
        };
        assert_eq!(code("https://embed.gog.com/on_login_success?origin=client&code=AbC-12_x.y~z").as_deref(), Some("AbC-12_x.y~z"));
        assert_eq!(code("https://www.gog.com/on_login_success?origin=client&code=abc").as_deref(), Some("abc"));
        // Pas la redirection : navigation ordinaire.
        for autre in [
            "http://embed.gog.com/on_login_success?code=abc",
            "https://embed.gog.com:8443/on_login_success?code=abc",
            "https://embed.gog.com.evil.example/on_login_success?code=abc",
            "https://evil.example/on_login_success?code=abc",
            "https://login.gog.com/on_login_success?code=abc",
            "https://embed.gog.com/autre?code=abc",
            "https://moi@embed.gog.com/on_login_success?code=abc",
        ] {
            assert_eq!(lire_retour_connexion(&u(autre)), None, "{autre}");
        }
        // La redirection sans code : l'étape dit pourquoi, en caractères sûrs.
        assert_eq!(lire_retour_connexion(&u("https://embed.gog.com/on_login_success?origin=client")), Some(RetourConnexion::SansCode("sans-code".into())));
        assert_eq!(lire_retour_connexion(&u("https://embed.gog.com/on_login_success?error=access_denied<x>")), Some(RetourConnexion::SansCode("erreur-access_deniedx".into())));
        assert_eq!(lire_retour_connexion(&u("https://embed.gog.com/on_login_success?code=a%20b")), Some(RetourConnexion::SansCode("code-de-forme-inattendue".into())));
        // Le code n'apparaît jamais dans un Debug.
        assert!(!format!("{:?}", lire_retour_connexion(&u("https://embed.gog.com/on_login_success?code=SECRETCODE"))).contains("SECRET"));
    }

    #[test]
    fn la_fenetre_de_connexion_ne_va_que_chez_gog_sauf_montree() {
        let ok = |s: &str, m: bool| navigation_connexion(&u(s), m);
        for a in ["https://auth.gog.com/auth?x=1", "https://login.gog.com/login", "https://embed.gog.com/x", "https://www.gog.com/"] {
            assert!(ok(a, false) && ok(a, true), "{a}");
        }
        assert!(!ok("https://accounts.google.com/o/oauth2", false), "discrète : jamais un autre site");
        assert!(ok("https://accounts.google.com/o/oauth2", true), "montrée : la connexion par Google de la page de GOG");
        for piege in ["http://auth.gog.com/auth", "https://auth.gog.com:8443/auth", "https://auth.gog.com.evil.example/", "https://x@auth.gog.com/", "https://evil.example/"] {
            assert!(!ok(piege, true), "{piege}");
        }
        assert!(page_de_connexion_affichee(&u("https://login.gog.com/auth?x")) && !page_de_connexion_affichee(&u("https://embed.gog.com/on_login_success")));
    }

    #[test]
    fn la_reponse_du_jeton_est_lue_et_le_compte_verifie() {
        let j = lire_jetons(&json!({"expires_in": 3600, "access_token": "aaa.bbb", "refresh_token": "rrr", "user_id": "48628349957132247", "session_id": "s"})).unwrap();
        assert_eq!((j.acces.as_str(), j.renouvellement.as_str(), j.user_id.as_str()), ("aaa.bbb", "rrr", "48628349957132247"));
        assert_eq!(format!("{j:?}"), "JetonsGog(<caché>)");
        assert!(lire_jetons(&json!({"access_token": "a", "refresh_token": "r", "user_id": 12})).is_ok(), "user_id en nombre");
        for mauvais in [json!({}), json!({"access_token": "a", "user_id": "1"}), json!({"access_token": "a b", "refresh_token": "r", "user_id": "1"}), json!({"access_token": "a", "refresh_token": "r", "user_id": "x1"})] {
            assert!(lire_jetons(&mauvais).is_err(), "{mauvais}");
        }
        assert!(verifier_compte(&j, "48628349957132247").is_ok());
        assert!(matches!(verifier_compte(&j, "1"), Err(Erreur::Refus(_))), "un autre compte que celui de la page : refus");
    }

    #[tokio::test]
    async fn l_echange_du_code_et_le_renouvellement_parlent_a_auth_gog() {
        let s = MockServer::start_async().await;
        let m = s
            .mock_async(|w, t| {
                w.method(GET).path("/token").query_param("client_id", CLIENT_ID).query_param("grant_type", "authorization_code").query_param("code", "C0DE").query_param("redirect_uri", REDIRECTION);
                t.status(200).json_body(json!({"access_token": "A", "refresh_token": "R", "user_id": "42"}));
            })
            .await;
        let j = jetons_par_code(&s.base_url(), &CodeGog("C0DE".into())).await.unwrap();
        assert_eq!(j.user_id, "42");
        m.assert_async().await;
        let refus = s
            .mock_async(|w, t| {
                w.method(GET).path("/token").query_param("grant_type", "refresh_token");
                t.status(400).json_body(json!({"error": "invalid_grant"}));
            })
            .await;
        let e = jetons_renouveles(&s.base_url(), "VIEUX").await.unwrap_err();
        assert!(matches!(&e, Erreur::Refus(m) if m == MOTIF_RECONNECTER_GOG));
        assert!(!format!("{e:?}").contains("VIEUX"), "le jeton n'apparaît jamais dans l'erreur");
        refus.assert_async().await;
    }

    #[test]
    fn une_page_de_la_bibliotheque_ne_garde_que_les_jeux_gog_possedes() {
        let v = json!({"total_count": 4, "items": [
            {"platform_id": "gog", "external_id": "1209025141", "owned": true},
            {"platform_id": "steam", "external_id": "620"},
            {"platform_id": "gog", "external_id": 1423049311},
            {"platform_id": "gog", "external_id": "999", "owned": false},
            {"platform_id": "gog", "external_id": "pas-un-numero"}
        ], "next_page_token": "abc=="});
        assert_eq!(lire_page_bibliotheque(&v), Some((vec!["1209025141".to_string(), "1423049311".to_string()], 5, Some("abc==".into()))));
        assert_eq!(lire_page_bibliotheque(&json!({"items": [], "next_page_token": ""})), Some((vec![], 0, None)));
        assert_eq!(lire_page_bibliotheque(&json!({"releases": []})), None);
    }

    #[tokio::test]
    async fn la_bibliotheque_se_lit_page_apres_page_avec_le_jeton() {
        let s = MockServer::start_async().await;
        let p2 = s
            .mock_async(|w, t| {
                w.method(GET).path("/users/42/releases").query_param("page_token", "p2+/=").header("authorization", "Bearer ACCES");
                t.status(200).json_body(json!({"items": [{"platform_id": "gog", "external_id": "2"}, {"platform_id": "gog", "external_id": "1"}]}));
            })
            .await;
        let p1 = s
            .mock_async(|w, t| {
                w.method(GET).path("/users/42/releases").header("authorization", "Bearer ACCES").matches(|r| r.query_params.as_ref().is_none_or(|q| q.is_empty()));
                t.status(200).json_body(json!({"items": [{"platform_id": "gog", "external_id": "1"}, {"platform_id": "epic", "external_id": "x"}], "next_page_token": "p2+/="}));
            })
            .await;
        let b = lire_bibliotheque(&s.base_url(), "42", "ACCES").await.unwrap();
        assert_eq!((b.ids, b.elements, b.pages), (vec!["1".to_string(), "2".to_string()], 4, 2));
        p1.assert_async().await;
        p2.assert_async().await;
        // Jeton refusé : « reconnecte-toi » (le cœur oublie alors le jeton et refait la connexion).
        let s = MockServer::start_async().await;
        s.mock_async(|w, t| {
            w.method(GET).path("/users/42/releases");
            t.status(401);
        })
        .await;
        assert!(matches!(lire_bibliotheque(&s.base_url(), "42", "X").await, Err(Erreur::Refus(_))));
        // Une page qui renvoie à elle-même : échec (jamais une boucle sans fin).
        let s = MockServer::start_async().await;
        s.mock_async(|w, t| {
            w.method(GET).path("/users/42/releases");
            t.status(200).json_body(json!({"items": [], "next_page_token": "meme"}));
        })
        .await;
        assert!(lire_bibliotheque(&s.base_url(), "42", "X").await.is_err());
    }

    /// Les formes relevées le 21/11 sur gamesdb (abrégées).
    fn horizon() -> Value {
        json!({"id": "53698417414694084", "game_id": "51152978713549456", "platform_id": "gog", "external_id": "1209025141", "parent_id": null, "type": "game",
            "title": {"*": "Horizon Zero Dawn™ Complete Edition", "en-US": "Horizon Zero Dawn: Complete Edition"},
            "game": {"id": "51152978713549456", "type": "game", "visible_in_library": true, "title": {"*": "Horizon Zero Dawn: Complete Edition"},
                "vertical_cover": {"url_format": "https://images.gog.com/ad5223c9fb72d2aa86a2e08c2664008135ef5e4cfc4df08a4b0b6833060f10fa{formatter}.{ext}?namespace=gamesdb"}}})
    }

    #[test]
    fn une_fiche_de_gamesdb_se_lit_titre_type_et_jaquette_verticale() {
        let f = lire_fiche(&horizon()).unwrap();
        assert_eq!(f.titre, "Horizon Zero Dawn: Complete Edition", "le titre du jeu d'abord");
        assert_eq!((f.genre.as_str(), f.genre_du_jeu.as_str(), f.jeu.as_str(), f.visible), ("game", "game", "51152978713549456", Some(true)));
        let img = f.image.unwrap();
        assert_eq!(img, "https://images.gog.com/ad5223c9fb72d2aa86a2e08c2664008135ef5e4cfc4df08a4b0b6833060f10fa_glx_vertical_cover.webp?namespace=gamesdb");
        assert!(crate::boutiques::image_autorisee(&img));
        // Une licence Amazon (relevé : 1272473144).
        let spam = lire_fiche(&json!({"game_id": "59033084739339888", "type": "spam", "title": {"*": "Hellslave - Amazon Prime"}, "game": {"id": "59033084739339888", "type": "spam", "visible_in_library": false, "title": {"*": "Hellslave - Amazon Prime"}}})).unwrap();
        assert_eq!((spam.titre.as_str(), spam.image), ("Hellslave - Amazon Prime", None));
        // Formes inconnues : pas de fiche ; image d'une autre forme : pas d'image.
        assert_eq!(lire_fiche(&json!({"error": "not_found"})), None);
        for f in ["https://images.gog.com/abc{formatter}.{ext}", "https://evil.example/ad5223c9fb72d2aa86a2e08c26640081{formatter}.{ext}", "https://images.gog.com/ad5223c9fb72d2aa86a2e08c26640081.jpg", "https://images.gog.com/ad5223c9fb72d2aa86a2e08c26640081{formatter}.{ext}?x=../"] {
            assert_eq!(image_verticale(f), None, "{f}");
        }
    }


    #[tokio::test]
    async fn la_fiche_publique_se_demande_sans_jeton_et_se_date() {
        let s = MockServer::start_async().await;
        let ok = s
            .mock_async(|w, t| {
                w.method(GET).path("/platforms/gog/external_releases/1209025141").matches(|r| r.headers.as_ref().is_none_or(|h| !h.iter().any(|(k, _)| k.eq_ignore_ascii_case("authorization") || k.eq_ignore_ascii_case("cookie"))));
                t.status(200).json_body(horizon());
            })
            .await;
        s.mock_async(|w, t| {
            w.method(GET).path("/platforms/gog/external_releases/1");
            t.status(404).json_body(json!({"error": "not_found"}));
        })
        .await;
        s.mock_async(|w, t| {
            w.method(GET).path("/platforms/gog/external_releases/2");
            t.status(429);
        })
        .await;
        s.mock_async(|w, t| {
            w.method(GET).path("/platforms/gog/external_releases/3");
            t.status(500);
        })
        .await;
        // Un seul client pour toutes les fiches.
        let c = client().unwrap();
        let h = fiche(&c, &s.base_url(), "1209025141", 1000).await.unwrap();
        assert_eq!((h.titre.as_str(), h.lu_le), ("Horizon Zero Dawn: Complete Edition", 1000));
        ok.assert_async().await;
        let i = fiche(&c, &s.base_url(), "1", 1000).await.unwrap();
        assert!(i.introuvable && i.lu_le == 1000);
        assert_eq!(fiche(&c, &s.base_url(), "2", 1000).await, Err(EchecFiche::TropDeDemandes));
        assert_eq!(fiche(&c, &s.base_url(), "3", 1000).await, Err(EchecFiche::Autre));
        assert_eq!(fiche(&c, &s.base_url(), "../x", 1000).await, Err(EchecFiche::Autre));
    }

    fn f(titre: &str, genre: &str, genre_du_jeu: &str, jeu: &str, visible: Option<bool>) -> FicheGog {
        FicheGog { titre: titre.into(), genre: genre.into(), genre_du_jeu: genre_du_jeu.into(), jeu: jeu.into(), visible, image: Some(format!("https://images.gog.com/{jeu}")), introuvable: false, lu_le: 1 }
    }

    fn jb(id: &str, nom: &str) -> JeuBoutique {
        JeuBoutique { id: id.into(), nom: nom.into(), minutes: 0, derniere: 0, installe: false, plateforme: "gog".into(), image: Some("page".into()), dossier: None, pret: None, succes: None, releases: vec![], titre_douteux: false }
    }

    fn ids(l: &[&str]) -> Vec<String> {
        l.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn la_regle_des_types_garde_les_jeux_et_une_carte_par_jeu() {
        assert_eq!(raison_ecart(&f("A", "game", "game", "1", Some(true))), None);
        assert_eq!(raison_ecart(&f("A", "game", "game", "1", None)), None);
        assert_eq!(raison_ecart(&f("A", "game", "game", "1", Some(false))), Some("invisible"));
        assert_eq!(raison_ecart(&f("D", "dlc", "dlc", "1", Some(true))), Some("dlc"));
        assert_eq!(raison_ecart(&f("P", "pack", "pack", "1", Some(true))), None);
        assert_eq!(raison_ecart(&f("P", "pack", "pack", "1", None)), Some("pack"));
        assert_eq!(raison_ecart(&f("L", "spam", "spam", "1", Some(false))), Some("licence"));
        assert_eq!(raison_ecart(&f("X", "truc", "truc", "1", Some(true))), Some("autre-type"));
        // Une licence qui RENVOIE à un jeu (son `game` est un jeu) : c'est ce jeu.
        assert_eq!(raison_ecart(&f("L", "spam", "game", "1", Some(true))), None);
        // Introuvable dans la base : pas un écart (gardé comme une fiche en échec).
        assert_eq!(raison_ecart(&FicheGog { introuvable: true, ..Default::default() }), None);
        let biblio = ids(&["10", "11", "12", "13", "14", "15", "16"]);
        let mut fiches = BTreeMap::new();
        // 10 : une licence qui renvoie au jeu 900 ; 11 : l'entrée « game » du même jeu 900 (préférée) ; 12 : DLC ;
        // 13 : licence Amazon ; 14 : pas de fiche (lecture ratée) ; 15 : introuvable ; 16 : licence que le SITE montre.
        fiches.insert("10".to_string(), f("Jeu 900 (licence)", "spam", "game", "900", Some(true)));
        fiches.insert("11".to_string(), f("Jeu 900", "game", "game", "900", Some(true)));
        fiches.insert("12".to_string(), f("Goodies", "dlc", "dlc", "901", Some(true)));
        fiches.insert("13".to_string(), f("Hellslave - Amazon Prime", "spam", "spam", "902", Some(false)));
        fiches.insert("15".to_string(), FicheGog { introuvable: true, lu_le: 1, ..Default::default() });
        fiches.insert("16".to_string(), f("Vieux jeu", "spam", "spam", "903", Some(false)));
        let page = vec![jb("gog_14", "Titre du site"), jb("gog_16", "Vieux jeu (site)"), jb("gog_77", "Seulement sur le site")];
        let ranges = vec![jb("gog_15", "Titre rangé 15")];
        let (l, b) = choisir(&biblio, &fiches, &page, &ranges);
        assert_eq!(
            l.iter().map(|j| (j.id.as_str(), j.nom.as_str())).collect::<Vec<_>>(),
            [("gog_11", "Jeu 900"), ("gog_14", "Titre du site"), ("gog_15", "Titre rangé 15"), ("gog_16", "Vieux jeu"), ("gog_77", "Seulement sur le site")]
        );
        assert_eq!(l[0].image.as_deref(), Some("https://images.gog.com/900"), "la jaquette verticale de gamesdb");
        assert_eq!(l[1].image.as_deref(), Some("page"), "fiche en échec : l'image du site");
        assert_eq!(b.gardes, 4);
        assert_eq!(b.hors_bibliotheque, 1);
        assert_eq!(b.en_echec, ids(&["14", "15"]));
        assert_eq!(b.gardes_par_le_site, vec!["licence : Vieux jeu".to_string()], "le site montre ce jeu : jamais écarté");
        assert!(b.ecartes.contains(&"doublon : Jeu 900 (licence)".to_string()));
        assert!(b.ecartes.contains(&"dlc : Goodies".to_string()));
        assert!(b.ecartes.contains(&"licence : Hellslave - Amazon Prime".to_string()));
        // Une fiche en échec sans titre connu : un titre provisoire (le jeu ne disparaît pas).
        let (l, _) = choisir(&ids(&["5"]), &BTreeMap::new(), &[], &[]);
        assert_eq!(l[0].nom, "Jeu GOG 5");
        let t = texte_bilan(&Bibliotheque { ids: biblio.clone(), elements: 9, pages: 1 }, &b, 5, 5);
        assert!(t.contains("types : dlc 1, en-echec 1, game 1, introuvable 1, spam 3 ; gardés 4"), "{t}");
        assert!(t.contains("licence : Hellslave - Amazon Prime") && t.contains("gardés car sur le site 1"));
    }

    #[test]
    fn un_doublon_garde_le_numero_deja_range_puis_celui_du_site_puis_l_entree_game() {
        let mut fiches = BTreeMap::new();
        fiches.insert("1".to_string(), f("Jeu (game)", "game", "game", "900", Some(true)));
        fiches.insert("2".to_string(), f("Jeu (licence)", "spam", "game", "900", Some(true)));
        let biblio = ids(&["1", "2"]);
        let garde = |page: &[JeuBoutique], ranges: &[JeuBoutique]| choisir(&biblio, &fiches, page, ranges).0.iter().map(|j| j.id.clone()).collect::<Vec<_>>();
        assert_eq!(garde(&[], &[]), ["gog_1"], "l'entrée « game »");
        assert_eq!(garde(&[jb("gog_2", "x")], &[]), ["gog_2"], "celle du site");
        assert_eq!(garde(&[jb("gog_1", "x")], &[jb("gog_2", "x")]), ["gog_2"], "celle déjà rangée d'abord (jamais deux cartes)");
    }

    #[test]
    fn sans_bibliotheque_entiere_la_liste_ne_retrecit_jamais() {
        let l = union_sans_bibliotheque(vec![jb("gog_1", "Site")], &[jb("gog_1", "Rangé"), jb("gog_2", "Rangé 2"), jb("620", "pas GOG")]);
        assert_eq!(l.iter().map(|j| (j.id.as_str(), j.nom.as_str())).collect::<Vec<_>>(), [("gog_1", "Site"), ("gog_2", "Rangé 2")]);
    }

    #[test]
    fn les_fiches_en_cache_ne_sont_redemandees_qu_une_fois_perimees() {
        let jour = 86_400;
        let maintenant = 1000 * jour;
        let biblio = ids(&["1", "2", "3", "4", "5", "6"]);
        let mut cache = BTreeMap::new();
        cache.insert("1".to_string(), FicheGog { lu_le: maintenant - 10 * jour, ..f("A", "game", "game", "9", Some(true)) });
        cache.insert("3".to_string(), FicheGog { introuvable: true, lu_le: maintenant - 3 * jour, ..Default::default() });
        cache.insert("4".to_string(), FicheGog { introuvable: true, lu_le: maintenant - 8 * jour, ..Default::default() });
        cache.insert("5".to_string(), FicheGog { lu_le: maintenant - 61 * jour, ..f("B", "game", "game", "8", Some(true)) });
        cache.insert("6".to_string(), FicheGog { lu_le: 0, ..f("C", "game", "game", "7", Some(true)) });
        assert_eq!(fiches_a_demander(&biblio, &cache, maintenant), ids(&["2", "4", "5", "6"]));
        // Le cache se range dans boutiques.json et se relit (un ancien cache sans date se lit aussi).
        let v = serde_json::to_value(&cache).unwrap();
        let relu: BTreeMap<String, FicheGog> = serde_json::from_value(v).unwrap();
        assert_eq!(relu, cache);
        let ancien: FicheGog = serde_json::from_value(json!({"titre": "X", "genre": "game"})).unwrap();
        assert_eq!(ancien.lu_le, 0);
    }
}
