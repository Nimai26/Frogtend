//! Les jeux offerts (lot 9). Décision de Seb (02/10) : on essaie d'abord de les **obtenir automatiquement** ; si ça
//! bloque (connexion, captcha, page changée), la fenêtre de la boutique s'affiche et la personne finit elle-même.
//!
//! **Epic** (relevé le 02/10) :
//! - liste PUBLIQUE, sans compte : `store-site-backend-static.ak.epicgames.com/freeGamesPromotions` →
//!   `data.Catalog.searchStore.elements[]` : `title`, `price.totalPrice.discountPrice` (0 = gratuit),
//!   `promotions.promotionalOffers[0].promotionalOffers[]{startDate, endDate}`, `catalogNs.mappings[].pageSlug`,
//!   `productSlug`, `keyImages[]{type, url}` ;
//! - obtention : la page officielle `store.epicgames.com/en-US/p/<slug>` dans une fenêtre de Frogtend (stockage du
//!   navigateur PROPRE AU PROFIL : chacun son compte Epic), et le script `ressources/gratuits/epic.js`, qui rend son
//!   résultat par le titre de la page. Méthode inspirée de vogler/free-games-claimer (AGPL : rien n'est copié).
//!
//! **PlayStation Plus** (demande de Seb, 02/10 : optionnel par profil, jamais dans la ludothèque) : la liste des jeux
//! du mois n'est lisible que sur le Store connecté. La page de la catégorie PS Plus s'ouvre CACHÉE (connexion Sony
//! propre au profil), le script `ressources/gratuits/psplus.js` visite chaque jeu et ne clique QUE sur « Ajouter à la
//! bibliothèque » (il ne peut rien acheter).

use crate::erreurs::{Erreur, Resultat};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub const SCRIPT_EPIC: &str = include_str!("../ressources/gratuits/epic.js");
pub const SCRIPT_PSPLUS: &str = include_str!("../ressources/gratuits/psplus.js");
/// La catégorie « nouveautés PS Plus » du Store (lien de la page officielle playstation.com/fr-fr/ps-plus/whats-new/,
/// relevé le 02/10).
pub const PAGE_PSPLUS: &str = "https://store.playstation.com/fr-fr/category/b3915b25-f581-43dd-95dd-a4ec50dbabe6/1";
pub const CONNEXION_PLAYSTATION: &str = "https://store.playstation.com/fr-fr/pages/latest";
pub const LISTE_EPIC: &str = "https://store-site-backend-static.ak.epicgames.com/freeGamesPromotions?locale=en-US&country=FR&allowCountries=FR";

/// Un jeu offert en ce moment.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct JeuOffert {
    pub boutique: String,
    pub titre: String,
    /// Le nom de la page (Epic : `/p/<slug>`).
    pub slug: String,
    /// Fin de l'offre (texte ISO).
    pub fin: String,
    pub image: Option<String>,
    /// L'espace de noms Epic du jeu (le même que dans le catalogue du lanceur : un jeu offert déjà possédé y est
    /// reconnu sans rien cliquer, Seb 19/11). Vide s'il manque.
    #[serde(default)]
    pub namespace: String,
}

/// Les jeux GRATUITS en ce moment dans la réponse publique d'Epic (`now` : date ISO, pour comparer).
pub fn lire_offerts_epic(v: &Value, now: &str) -> Vec<JeuOffert> {
    let mut l = Vec::new();
    for e in v["data"]["Catalog"]["searchStore"]["elements"].as_array().cloned().unwrap_or_default() {
        if e["price"]["totalPrice"]["discountPrice"].as_i64() != Some(0) {
            continue;
        }
        let offres = e["promotions"]["promotionalOffers"].as_array().cloned().unwrap_or_default();
        let Some(o) = offres.iter().flat_map(|x| x["promotionalOffers"].as_array().cloned().unwrap_or_default()).find(|o| {
            let (d, f) = (o["startDate"].as_str().unwrap_or(""), o["endDate"].as_str().unwrap_or(""));
            !d.is_empty() && d <= now && now < f
        }) else {
            continue;
        };
        let slug = e["catalogNs"]["mappings"]
            .as_array()
            .and_then(|m| m.iter().find_map(|x| x["pageSlug"].as_str().map(String::from)))
            .or_else(|| e["productSlug"].as_str().map(|s| s.trim_end_matches("/home").to_string()))
            .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'));
        let Some(slug) = slug else { continue };
        let image = e["keyImages"].as_array().and_then(|k| {
            k.iter()
                .find(|i| i["type"] == "OfferImageTall")
                .or_else(|| k.first())
                .and_then(|i| i["url"].as_str().map(String::from))
        });
        l.push(JeuOffert {
            boutique: "epic".into(),
            titre: e["title"].as_str().unwrap_or("?").into(),
            slug,
            fin: o["endDate"].as_str().unwrap_or("").into(),
            image,
            namespace: e["namespace"].as_str().filter(|n| !n.is_empty() && n.len() <= 64 && n.chars().all(|c| c.is_ascii_alphanumeric())).unwrap_or("").into(),
        });
    }
    l
}

pub async fn offerts_epic() -> Resultat<Vec<JeuOffert>> {
    let v: Value = reqwest::Client::builder()
        .user_agent("Frogtend")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|_| Erreur::Reseau("Connexion impossible.".into()))?
        .get(LISTE_EPIC)
        .send()
        .await
        .map_err(|e| Erreur::Reseau(format!("Epic ne répond pas ({e}).")))?
        .json()
        .await
        .map_err(|_| Erreur::Serveur("Réponse d'Epic illisible.".into()))?;
    Ok(lire_offerts_epic(&v, &maintenant_iso()))
}

/// L'heure UTC au format ISO (comparable aux dates d'Epic).
pub fn maintenant_iso() -> String {
    let s = crate::noyau::maintenant().parse::<i64>().unwrap_or(0);
    let (jours, reste) = (s.div_euclid(86_400), s.rem_euclid(86_400));
    // Jours depuis 1970 → date civile (H. Hinnant).
    let z = jours + 719_468;
    let ere = z.div_euclid(146_097);
    let doe = z - ere * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + ere * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.000Z", reste / 3600, reste % 3600 / 60, reste % 60)
}

/// Le résultat rendu par le script (titre « FROGTEND:… »).
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(tag = "etat", content = "motif", rename_all = "snake_case")]
pub enum Obtention {
    Obtenu,
    Deja,
    Connexion,
    Captcha,
    /// Fenêtre de connexion : la boutique montre le compte connecté.
    Connecte,
    /// `<étape> | <textes des boutons visibles>` (l'étape où le script s'est arrêté).
    Erreur(String),
}

pub fn lire_titre(titre: &str) -> Option<Obtention> {
    let r = titre.strip_prefix("FROGTEND:")?;
    Some(match r {
        "OBTENU" => Obtention::Obtenu,
        "DEJA" => Obtention::Deja,
        "CONNEXION" => Obtention::Connexion,
        "CAPTCHA" => Obtention::Captcha,
        "CONNECTE" => Obtention::Connecte,
        autre => Obtention::Erreur(autre.strip_prefix("ERREUR:").unwrap_or(autre).to_string()),
    })
}

/// Le script de la fenêtre « Se connecter à Epic » : le même, en mode connexion (il ne clique sur RIEN et dit
/// seulement « CONNECTE » quand la boutique montre le compte connecté).
pub fn script_epic_connexion() -> String {
    format!("window.__frogtendMode = 'connexion';\n{SCRIPT_EPIC}")
}

/// Le script de la vérification CACHÉE du compte Epic (à l'ouverture de 🛒 Boutiques, Seb 20/11) : aucun clic ; il dit
/// « CONNECTE » ou « CONNEXION », rien s'il n'est pas sûr.
pub fn script_epic_verification() -> String {
    format!("window.__frogtendMode = 'verification';\n{SCRIPT_EPIC}")
}

/// Le script du CONTRÔLE après une commande (Seb 20/11) : la page du jeu rechargée dit « DEJA » (In Library) ou une
/// erreur ; aucun clic.
pub fn script_epic_controle() -> String {
    format!("window.__frogtendMode = 'controle';\n{SCRIPT_EPIC}")
}

/// Le verdict après une commande : « obtenu » SEULEMENT si la page rechargée montre le jeu dans la bibliothèque ;
/// sinon l'erreur dite par la page, ou « pas de preuve » (jamais « pas vu = obtenu »).
pub fn apres_controle(preuve: Option<Obtention>) -> Obtention {
    match preuve {
        Some(Obtention::Deja) => Obtention::Obtenu,
        Some(Obtention::Erreur(m)) => Obtention::Erreur(m),
        _ => Obtention::Erreur("pas de preuve d'Epic que le jeu est dans ta bibliothèque".into()),
    }
}

/// Ce qu'une vérification a appris du compte Epic : vrai = connecté, faux = pas connecté, None = pas sûr.
pub fn compte_epic_vu(o: &Obtention) -> Option<bool> {
    match o {
        Obtention::Connecte => Some(true),
        Obtention::Connexion => Some(false),
        _ => None,
    }
}

/// Le bilan d'un passage sur les jeux PS Plus.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(tag = "etat", rename_all = "snake_case")]
pub enum RecoltePsPlus {
    Faite { ajoutes: u32, deja: u32, vus: u32 },
    Connexion,
    Erreur { motif: String },
}

pub fn lire_titre_psplus(titre: &str) -> Option<RecoltePsPlus> {
    let r = titre.strip_prefix("FROGTEND:")?;
    if r == "CONNEXION" {
        return Some(RecoltePsPlus::Connexion);
    }
    if let Some(n) = r.strip_prefix("PSPLUS:") {
        let v: Vec<u32> = n.split(':').filter_map(|x| x.parse().ok()).collect();
        if let [ajoutes, deja, vus] = v[..] {
            return Some(RecoltePsPlus::Faite { ajoutes, deja, vus });
        }
    }
    Some(RecoltePsPlus::Erreur { motif: r.strip_prefix("ERREUR:").unwrap_or(r).to_string() })
}

/// GOG et Prime Gaming (Seb, 02/10 : « Prime Gaming, GOG — mêmes principes » qu'Epic) : la page officielle, cachée,
/// avec la connexion propre au profil ; le script rend un bilan JSON par le titre.
pub const SCRIPT_GOG: &str = include_str!("../ressources/gratuits/gog.js");
pub const SCRIPT_PRIME: &str = include_str!("../ressources/gratuits/prime.js");

/// (page de connexion, page de récupération, script) d'une boutique.
pub fn boutique_offerte(b: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match b {
        "gog" => Some(("https://www.gog.com/en", "https://www.gog.com/en", SCRIPT_GOG)),
        "prime" => Some(("https://gaming.amazon.com/home", "https://gaming.amazon.com/home", SCRIPT_PRIME)),
        _ => None,
    }
}

/// Le bilan d'un passage sur GOG ou Prime Gaming.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Recolte {
    /// `faite`, `connexion`, `aucun` (pas de jeu offert en ce moment), `pas_abonne` (Prime), `erreur`.
    pub etat: String,
    #[serde(default)]
    pub obtenus: Vec<String>,
    #[serde(default)]
    pub deja: u32,
    /// Prime : les offres d'autres boutiques (code à activer, compte à relier) à finir sur la page.
    #[serde(default)]
    pub a_finir: u32,
    #[serde(default)]
    pub motif: Option<String>,
}

pub fn lire_titre_recolte(titre: &str) -> Option<Recolte> {
    let j = titre.strip_prefix("FROGTEND:")?;
    Some(serde_json::from_str::<Recolte>(j).unwrap_or(Recolte { etat: "erreur".into(), motif: Some(j.chars().take(120).collect()), ..Default::default() }))
}

/// Le script de la fenêtre de connexion à Steam (lot C1) : il dit seulement si la boutique montre un compte connecté,
/// et son `steamid` (jamais le jeton de la page).
pub const SCRIPT_STEAM: &str = include_str!("../ressources/gratuits/steam.js");

/// Le script de la fenêtre cachée qui lit le jeton d'accès Steam (lot C2, 0.215.2) : voir `lire_remise_steam`.
pub const SCRIPT_STEAM_JETON: &str = include_str!("../ressources/gratuits/steam-jeton.js");

/// Ce que la page de la boutique remet au cœur par une navigation (annulée) vers `https://frogtend.invalid/…`.
#[derive(PartialEq)]
pub enum RemiseSteam {
    Jeton(String),
    /// Pas de jeton : l'étape, en quelques mots sûrs (lettres, chiffres, tirets), pour le journal.
    Absent(String),
}

impl std::fmt::Debug for RemiseSteam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemiseSteam::Jeton(_) => f.write_str("Jeton(<caché>)"),
            RemiseSteam::Absent(e) => write!(f, "Absent({e})"),
        }
    }
}

/// Lot C3 : la page de Steam dit que le compte est connecté, mais Frogtend n'y trouve plus le jeton.
pub const MOTIF_PAGE_STEAM_CHANGEE: &str =
    "Steam a changé sa page : Frogtend n'arrive plus à lire tes jeux pour l'instant. Tes jeux restent dans ta ludothèque ; une mise à jour de Frogtend réglera ça.";
/// Lot C3 : la page de Steam n'a plus sa configuration (page changée, maintenance ou vérification) : pas « reconnecte-toi ».
pub const MOTIF_STEAM_INHABITUEL: &str =
    "Steam ne répond pas comme d'habitude (maintenance ou page changée) : réessaie plus tard. Tes jeux restent dans ta ludothèque.";

/// L'erreur d'un import Steam SANS jeton, d'après l'étape remise par la page (`steam-jeton.js`, pur, testé) :
/// « reconnecte-toi » seulement quand la page dit le compte déconnecté (ou que Steam refuse) ; la page dit le compte
/// connecté et répond normalement mais sans jeton : la page a changé ; plus de configuration du tout : réponse inhabituelle.
pub fn erreur_sans_jeton_steam(etape: &str) -> crate::erreurs::Erreur {
    use crate::erreurs::Erreur;
    if etape.ends_with("-erreur-reseau") {
        return Erreur::Reseau("Steam n'a pas répondu : vérifie ta connexion à Internet, puis réessaie.".into());
    }
    // Steam répond normalement (succès 1, ou pas du JSON), SANS redirection : la page a changé. Un succès 21 (« non
    // connecté » chez Steam) ou une redirection : « reconnecte-toi » reste le bon conseil.
    if etape == "connecte-statut-200-success-1" || etape == "connecte-statut-200-pas-json" {
        return Erreur::Serveur(MOTIF_PAGE_STEAM_CHANGEE.into());
    }
    if etape.starts_with("sans-configuration") {
        return Erreur::Serveur(MOTIF_STEAM_INHABITUEL.into());
    }
    Erreur::Refus(crate::boutiques::MOTIF_RECONNECTER.into())
}

/// Une navigation vers `https://frogtend.invalid/steam-jeton#<jwt>` ou `…/steam-absent#<étape>` : la remise de la page
/// (que l'appelant ANNULE). Toute autre adresse : None (navigation ordinaire). Un jeton sans la forme d'un JWT : Absent.
pub fn lire_remise_steam(url: &tauri::Url) -> Option<RemiseSteam> {
    if url.scheme() != "https" || url.host_str() != Some("frogtend.invalid") {
        return None;
    }
    let fragment = url.fragment().unwrap_or("");
    Some(match url.path() {
        "/steam-jeton" if forme_de_jwt(fragment) => RemiseSteam::Jeton(fragment.to_string()),
        "/steam-jeton" => RemiseSteam::Absent("jeton-de-forme-inattendue".into()),
        _ => {
            let etape: String = fragment.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').take(60).collect();
            RemiseSteam::Absent(if etape.is_empty() { "sans-detail".into() } else { etape })
        }
    })
}

/// La page officielle de connexion à Steam ; après la connexion, Steam ramène à la boutique (`/explore/`).
pub const CONNEXION_STEAM: &str = "https://store.steampowered.com/login/?redir=explore%2F";

/// Les seuls hôtes où la fenêtre de connexion à Steam peut aller (revue de l'expert, 19/11) : la boutique (page de
/// connexion et code Steam Guard y sont), le service de connexion, l'aide (mot de passe oublié, Steam Guard perdu) et
/// la communauté (connexion par l'application Steam). Tout autre lien est refusé.
const HOTES_STEAM: [&str; 4] = ["store.steampowered.com", "login.steampowered.com", "help.steampowered.com", "steamcommunity.com"];

/// La fenêtre de connexion à Steam peut-elle aller à cette adresse ? HTTPS, port normal, hôte EXACT de `HOTES_STEAM`.
pub fn navigation_steam(url: &tauri::Url) -> bool {
    url.scheme() == "https"
        && url.port().is_none()
        && url.username().is_empty()
        && url.host_str().is_some_and(|h| HOTES_STEAM.contains(&h.to_ascii_lowercase().as_str()))
}

/// Est-ce une page de la boutique Steam elle-même (seule origine dont le titre « FROGTEND:STEAM » est cru) ?
pub fn page_de_la_boutique_steam(url: &tauri::Url) -> bool {
    url.scheme() == "https" && url.port().is_none() && url.username().is_empty() && url.host_str().is_some_and(|h| h.eq_ignore_ascii_case("store.steampowered.com"))
}

// ───────────────────────────── GOG : les jeux du compte (lot C4a) ─────────────────────────────

/// Le script de la fenêtre CACHÉE qui lit les jeux du compte GOG connecté dans le navigateur du profil (lot C4a) :
/// voir `lire_remise_gog`.
pub const SCRIPT_GOG_JEUX: &str = include_str!("../ressources/gratuits/gog-jeux.js");

/// La page où la fenêtre cachée lit les jeux GOG (la même boutique que la connexion et le jeu offert).
pub const PAGE_GOG: &str = "https://www.gog.com/en";

/// Le plus grand nombre de morceaux accepté pour une liste (50 jeux par morceau : 20 000 jeux, bien au-delà d'un vrai
/// compte ; le script plafonne de son côté à 100 pages de 100 jeux, jeux cachés compris).
pub const MORCEAUX_GOG_MAX: usize = 400;

/// Un jeu du compte GOG tel que la page le remet : seulement son numéro (chiffres), son titre et l'adresse brute de son
/// image (normalisée ensuite par `boutiques::image_gog`). Rien d'autre n'est jamais remis.
#[derive(Clone, Debug, PartialEq)]
pub struct JeuGogLu {
    pub id: String,
    pub titre: String,
    pub image: String,
}

/// Ce que la page de GOG remet au cœur par une navigation (annulée) vers `https://frogtend.invalid/…`.
#[derive(PartialEq)]
pub enum RemiseGog {
    /// Le compte connecté (`userId`, chiffres seulement).
    Compte(String),
    /// Le morceau `k` sur `n` de la liste des jeux.
    Morceau { k: usize, n: usize, jeux: Vec<JeuGogLu> },
    /// Pas de liste : l'étape, en quelques mots sûrs (lettres, chiffres, tirets), pour le journal et le message.
    Absent(String),
    /// La page avance (le nombre de pages déjà demandées à GOG) : le délai sans nouvelle repart de zéro.
    Progres(usize),
    /// Le bilan de la lecture (0.217.1, pour le journal) : des nombres et des numéros de jeux écartés, jamais le compte.
    Bilan(String),
    /// 0.217.3 (sonde) : le jeton d'accès de la session GOG, lu par la page ; `None` : forme inattendue. Il ne vit qu'en
    /// mémoire (jamais écrit, journalisé ni rendu), le temps de lire la bibliothèque de Galaxy.
    Jeton(Option<JetonGog>),
}

/// Le jeton d'accès GOG : son `Debug` ne montre RIEN.
#[derive(Clone, PartialEq)]
pub struct JetonGog(pub String);

impl std::fmt::Debug for JetonGog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("JetonGog(<caché>)")
    }
}

impl std::fmt::Debug for RemiseGog {
    // Le numéro du compte ne s'écrit jamais (journal, message d'un test) ; d'un morceau, seulement sa place et sa taille.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemiseGog::Compte(_) => f.write_str("Compte(<caché>)"),
            RemiseGog::Morceau { k, n, jeux } => write!(f, "Morceau({k}/{n}, {} jeu(x))", jeux.len()),
            RemiseGog::Absent(e) => write!(f, "Absent({e})"),
            RemiseGog::Progres(n) => write!(f, "Progres({n})"),
            RemiseGog::Bilan(b) => write!(f, "Bilan({b})"),
            RemiseGog::Jeton(j) => write!(f, "Jeton({})", if j.is_some() { "<caché>" } else { "illisible" }),
        }
    }
}

/// Un numéro GOG (compte ou jeu) : des chiffres seulement, 1 à 20.
pub fn numero_gog(s: &str) -> bool {
    !s.is_empty() && s.len() <= 20 && s.bytes().all(|c| c.is_ascii_digit())
}

/// Lit un morceau de la liste (`<k>/<n>/<JSON en base64url>`, le JSON étant `[[id, titre, image], …]`). Tout morceau
/// d'une forme inattendue est refusé EN ENTIER (`None`) : un id qui n'est pas un numéro, un titre vide, un champ en trop
/// ou en moins veulent dire que la page a changé — jamais une liste devinée.
fn lire_morceau_gog(fragment: &str) -> Option<(usize, usize, Vec<JeuGogLu>)> {
    let mut parts = fragment.splitn(3, '/');
    let k: usize = parts.next()?.parse().ok()?;
    let n: usize = parts.next()?.parse().ok()?;
    if n == 0 || n > MORCEAUX_GOG_MAX || k >= n {
        return None;
    }
    let brut = crate::boutiques::base64url(parts.next()?)?;
    let v: Value = serde_json::from_slice(&brut).ok()?;
    let l = v.as_array()?;
    if l.len() > 100 {
        return None;
    }
    let mut jeux = Vec::with_capacity(l.len());
    for j in l {
        let a = j.as_array().filter(|a| a.len() == 3)?;
        let id = a[0].as_str().filter(|s| numero_gog(s))?;
        let titre = a[1].as_str().map(str::trim).filter(|t| !t.is_empty() && t.chars().count() <= 300)?;
        let image = a[2].as_str().filter(|i| i.len() <= 300)?;
        jeux.push(JeuGogLu { id: id.to_string(), titre: titre.to_string(), image: image.to_string() });
    }
    Some((k, n, jeux))
}

/// Une navigation vers `https://frogtend.invalid/gog-compte#<userId>`, `…/gog-jeux#<k>/<n>/<données>` ou
/// `…/gog-absent#<étape>` : la remise de la page (que l'appelant ANNULE). Toute autre adresse : `None` (navigation
/// ordinaire). Une remise de forme inattendue : `Absent` (jamais une valeur devinée).
pub fn lire_remise_gog(url: &tauri::Url) -> Option<RemiseGog> {
    if url.scheme() != "https" || url.host_str() != Some("frogtend.invalid") || url.port().is_some() {
        return None;
    }
    let fragment = url.fragment().unwrap_or("");
    Some(match url.path() {
        "/gog-compte" if numero_gog(fragment) => RemiseGog::Compte(fragment.to_string()),
        "/gog-compte" => RemiseGog::Absent("compte-de-forme-inattendue".into()),
        "/gog-progres" => RemiseGog::Progres(fragment.parse().unwrap_or(0)),
        "/gog-jeton" => RemiseGog::Jeton(
            (fragment.len() >= 20 && fragment.len() <= 4000 && fragment.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'-')))
                .then(|| JetonGog(fragment.to_string())),
        ),
        "/gog-bilan" => RemiseGog::Bilan(fragment.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').take(900).collect()),
        "/gog-jeux" => match lire_morceau_gog(fragment) {
            Some((k, n, jeux)) => RemiseGog::Morceau { k, n, jeux },
            None => RemiseGog::Absent("morceau-de-forme-inattendue".into()),
        },
        _ => {
            let etape: String = fragment.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').take(60).collect();
            RemiseGog::Absent(if etape.is_empty() { "sans-detail".into() } else { etape })
        }
    })
}

/// Où en est la réception d'une liste GOG (pur, testé) : le compte d'abord, puis les morceaux 0, 1, … n-1 DANS L'ORDRE.
#[derive(Default)]
pub struct ReceptionGog {
    compte: Option<String>,
    n: Option<usize>,
    recus: usize,
    jeux: Vec<JeuGogLu>,
    bilan: Option<String>,
    jeton: Option<JetonGog>,
    jeton_illisible: bool,
}

/// Ce que le cœur fait après une remise.
#[derive(Debug, PartialEq)]
pub enum SuiteGog {
    /// Demander à la page le morceau `k`.
    Demander(usize),
    /// Rien à faire : attendre la suite (la page avance).
    Attendre,
    /// Tout est reçu : (compte, jeux sans doublon).
    Fini(String, Vec<JeuGogLu>),
    /// La page dit pourquoi elle n'a rien (étape sûre).
    Absent(String),
    /// La remise ne suit pas le protocole (morceau manquant, désordre, compte absent) : échec, rien n'est écrit.
    Echec(String),
}

impl ReceptionGog {
    pub fn recevoir(&mut self, r: RemiseGog) -> SuiteGog {
        match r {
            RemiseGog::Absent(e) => SuiteGog::Absent(e),
            // Le bilan arrive juste avant le compte : gardé pour le journal, il ne change rien à la suite.
            RemiseGog::Bilan(b) => {
                self.bilan = Some(b);
                SuiteGog::Attendre
            }
            RemiseGog::Jeton(j) => {
                self.jeton_illisible = j.is_none();
                self.jeton = j;
                SuiteGog::Attendre
            }
            // La page avance encore (avant le compte) ; une fois les morceaux commencés, c'est une page rechargée.
            RemiseGog::Progres(_) if self.recus == 0 => SuiteGog::Attendre,
            RemiseGog::Progres(_) => SuiteGog::Echec("page-rechargee-pendant-la-remise".into()),
            RemiseGog::Compte(c) if self.compte.is_none() => {
                self.compte = Some(c);
                SuiteGog::Demander(0)
            }
            // La page www.gog.com s'est rechargée avant le premier morceau : son script, relancé, redonne le MÊME compte ;
            // le morceau 0 lui est redemandé (la liste de la page d'avant est oubliée avec elle).
            RemiseGog::Compte(c) if self.compte.as_ref() == Some(&c) && self.recus == 0 => SuiteGog::Demander(0),
            RemiseGog::Compte(c) if self.compte.as_ref() == Some(&c) => SuiteGog::Echec("page-rechargee-pendant-la-remise".into()),
            RemiseGog::Compte(_) => SuiteGog::Echec("autre-compte-pendant-la-lecture".into()),
            RemiseGog::Morceau { .. } if self.compte.is_none() => SuiteGog::Echec("morceau-avant-le-compte".into()),
            RemiseGog::Morceau { k, n, jeux } => {
                if self.n.is_some_and(|m| m != n) {
                    return SuiteGog::Echec("nombre-de-morceaux-change".into());
                }
                if k != self.recus {
                    return SuiteGog::Echec("morceau-dans-le-desordre".into());
                }
                self.n = Some(n);
                self.recus += 1;
                for j in jeux {
                    if !self.jeux.iter().any(|x| x.id == j.id) {
                        self.jeux.push(j);
                    }
                }
                if self.recus == n {
                    SuiteGog::Fini(self.compte.clone().unwrap_or_default(), std::mem::take(&mut self.jeux))
                } else {
                    SuiteGog::Demander(self.recus)
                }
            }
        }
    }

    /// Le nombre de morceaux reçus (journal).
    pub fn recus(&self) -> usize {
        self.recus
    }

    /// Le jeton remis par la page (sonde 0.217.3), retiré de la réception ; et s'il était illisible.
    pub fn prendre_jeton(&mut self) -> (Option<JetonGog>, bool) {
        (self.jeton.take(), self.jeton_illisible)
    }

    /// Le bilan de lecture remis par la page (journal), s'il est arrivé.
    pub fn bilan(&self) -> Option<String> {
        self.bilan.clone()
    }
}

/// Le message d'une lecture GOG sans liste, d'après l'étape remise (pur, testé). `None` : le compte n'est pas connecté
/// (« reconnecte-toi », dit par l'appelant). Seule la page qui DIT « pas connecté » mène à « reconnecte-toi ».
pub fn erreur_sans_jeux_gog(etape: &str) -> Option<crate::erreurs::Erreur> {
    use crate::erreurs::Erreur;
    match etape {
        "deconnecte" => None,
        "trop-de-demandes" => Some(Erreur::Reseau("GOG demande d'attendre un peu avant de réessayer : réessaie dans un moment. Tes jeux restent dans ta ludothèque.".into())),
        "erreur-reseau" => Some(Erreur::Reseau("GOG n'a pas répondu : vérifie ta connexion à Internet, puis réessaie.".into())),
        _ => Some(Erreur::Serveur(MOTIF_GOG_INHABITUEL.into())),
    }
}

/// GOG ne répond pas comme d'habitude (page changée, maintenance) : pas « reconnecte-toi ».
pub const MOTIF_GOG_INHABITUEL: &str =
    "GOG ne répond pas comme d'habitude (maintenance ou page changée) : réessaie plus tard. Tes jeux restent dans ta ludothèque.";

/// La fenêtre cachée de lecture des jeux GOG peut-elle aller à cette adresse ? HTTPS, port normal, sans identifiant,
/// hôte EXACT www.gog.com (jamais un sous-domaine, ni « www.gog.com.autre »).
pub fn navigation_gog(url: &tauri::Url) -> bool {
    url.scheme() == "https" && url.port().is_none() && url.username().is_empty() && url.password().is_none() && url.host_str().is_some_and(|h| h.eq_ignore_ascii_case("www.gog.com"))
}

/// Un identifiant Steam 64 bits d'une personne : 17 chiffres, dans la plage des comptes individuels
/// (76561197960265728 et au-delà, univers « public »). Tout le reste est refusé.
pub fn steamid_valide(s: &str) -> bool {
    s.len() == 17 && s.bytes().all(|c| c.is_ascii_digit()) && s.starts_with("7656119") && s.parse::<u64>().is_ok_and(|n| n >= 76_561_197_960_265_728)
}

/// Ce que le script de Steam a vu sur la page.
#[derive(Clone, Debug, PartialEq)]
pub enum ConnexionSteam {
    Connecte(String),
    PasConnecte,
}

/// Lit le titre posé par `steam.js` : `None` si ce n'est pas le sien, ou si le `steamid` n'est pas valide (le script
/// n'est jamais cru sur parole).
pub fn lire_titre_steam(titre: &str) -> Option<ConnexionSteam> {
    #[derive(Deserialize)]
    struct Vu {
        connecte: bool,
        #[serde(default)]
        steamid: String,
    }
    let v: Vu = serde_json::from_str(titre.strip_prefix("FROGTEND:STEAM:")?).ok()?;
    match v.connecte {
        true if steamid_valide(&v.steamid) => Some(ConnexionSteam::Connecte(v.steamid)),
        true => None,
        false => Some(ConnexionSteam::PasConnecte),
    }
}

/// Un jeton a-t-il la forme d'un JWT (trois morceaux en base64url, taille raisonnable) ?
pub fn forme_de_jwt(j: &str) -> bool {
    let morceaux: Vec<&str> = j.split('.').collect();
    j.len() <= 8192 && morceaux.len() == 3 && morceaux.iter().all(|m| !m.is_empty() && m.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_'))
}

/// Les sites d'IDENTITÉ qu'une fenêtre de connexion peut ouvrir (connexion par Google, Apple, Microsoft, Steam, la
/// boutique elle-même…), en HTTPS seulement (revue des experts, 18/11 : jamais « tout site en https »).
pub fn site_d_identite(url: &tauri::Url) -> bool {
    if url.scheme() != "https" {
        return false;
    }
    let Some(h) = url.host_str().map(|h| h.trim_end_matches('.').to_ascii_lowercase()) else { return false };
    let sous = |d: &str| h == d || h.ends_with(&format!(".{d}"));
    // Steam (lot C1) : sa boutique, sa page de connexion et son aide (Steam Guard, compte perdu) ; rien d'autre.
    const EXACTS: [&str; 8] = [
        "accounts.google.com",
        "appleid.apple.com",
        "login.live.com",
        "login.microsoftonline.com",
        "steamcommunity.com",
        "store.steampowered.com",
        "login.steampowered.com",
        "help.steampowered.com",
    ];
    const DOMAINES: [&str; 10] = ["epicgames.com", "playstation.com", "sony.com", "gog.com", "facebook.com", "xbox.com", "nintendo.com", "disney.com", "lego.com", "amazon.com"];
    // Amazon : ses sites nationaux (`*.amazon.<pays>`), jamais un « amazon » au milieu d'un autre domaine.
    const AMAZON: [&str; 14] = ["fr", "de", "it", "es", "nl", "se", "pl", "ca", "co.uk", "co.jp", "com.au", "com.br", "com.be", "com.mx"];
    EXACTS.contains(&h.as_str()) || DOMAINES.iter().any(|d| sous(d)) || AMAZON.iter().any(|p| sous(&format!("amazon.{p}")))
}

/// Les fenêtres que la page d'une boutique ouvre elle-même dans une fenêtre de CONNEXION (connexion par Google, Apple,
/// Steam, Xbox… : une petite fenêtre à part) : permises seulement vers un site d'identité, avec le même navigateur
/// que la fenêtre de la boutique (sa connexion y reste). Sans elles, le bouton « Google » de la page d'Epic ne faisait
/// rien (Seb, 18/11).
pub fn fenetre_de_connexion<R: tauri::Runtime>(url: tauri::Url, _f: tauri::webview::NewWindowFeatures) -> tauri::webview::NewWindowResponse<R> {
    if site_d_identite(&url) {
        tauri::webview::NewWindowResponse::Allow
    } else {
        tauri::webview::NewWindowResponse::Deny
    }
}

/// Les fenêtres INVISIBLES d'automatisation (obtenir, récupérer, PS Plus) : aucune fenêtre nouvelle, jamais.
pub fn fenetre_d_automatisation<R: tauri::Runtime>(_url: tauri::Url, _f: tauri::webview::NewWindowFeatures) -> tauri::webview::NewWindowResponse<R> {
    tauri::webview::NewWindowResponse::Deny
}

/// Le dossier du navigateur d'un profil pour les boutiques (sa connexion Epic y reste).
pub fn dossier_navigateur(dossier_profil: &Path) -> PathBuf {
    dossier_profil.join("navigateur")
}

/// Les jeux offerts déjà obtenus (ou déjà possédés) par un profil : `gratuits.json`.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Obtenus {
    pub jeux: Vec<(String, String, String)>, // (boutique, slug, date)
    #[serde(default)]
    pub derniere_verification: String,
    /// 2 depuis la 0.214.5 : les « obtenus » Epic sont prouvés par la page du jeu rechargée.
    #[serde(default)]
    pub version: u32,
}

/// La publication de la 0.214.4 (horodatage du commit) : ses « obtenus » Epic venaient d'une confirmation FAUSSE (Mark
/// of the Deep, 20/11 : « à toi » alors qu'Epic proposait encore « Obtenir »).
const PUBLICATION_0_214_4: u64 = 1_795_188_192;

/// Retire les « obtenus » Epic sans preuve (notés depuis la 0.214.4, avant la version 2 du fichier) : le jeu redevient
/// « à obtenir », et un nouvel essai le prouve. Rien d'autre n'est touché.
pub fn sans_obtenus_douteux(mut o: Obtenus) -> Obtenus {
    if o.version < 2 {
        o.jeux.retain(|(b, _, date)| b != "epic" || date.parse::<u64>().map_or(true, |d| d < PUBLICATION_0_214_4));
    }
    o
}

pub fn lire_obtenus(dossier_profil: &Path) -> Obtenus {
    sans_obtenus_douteux(std::fs::read(dossier_profil.join("gratuits.json")).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default())
}

pub fn ecrire_obtenus(dossier_profil: &Path, o: &Obtenus) -> Resultat<()> {
    std::fs::create_dir_all(dossier_profil)?;
    let o = Obtenus { version: 2, ..o.clone() };
    std::fs::write(dossier_profil.join("gratuits.json"), serde_json::to_vec_pretty(&o).unwrap_or_default())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn une_fenetre_de_connexion_n_ouvre_que_les_sites_d_identite_en_https() {
        let ok = |u: &str| site_d_identite(&tauri::Url::parse(u).unwrap());
        for u in [
            "https://accounts.google.com/o/oauth2/auth?x=1",
            "https://appleid.apple.com/auth/authorize",
            "https://login.live.com/oauth20_authorize.srf",
            "https://login.microsoftonline.com/common",
            "https://steamcommunity.com/openid/login",
            "https://www.epicgames.com/id/login",
            "https://my.account.sony.com/",
            "https://www.playstation.com/",
            "https://login.gog.com/auth",
            "https://www.facebook.com/dialog/oauth",
            "https://login.xbox.com/",
            "https://accounts.nintendo.com/",
            "https://login.disney.com/",
            "https://lego.com/",
            "https://identity.lego.com/",
            "https://www.amazon.com/ap/signin",
            "https://www.amazon.fr/ap/signin",
            "https://www.amazon.co.uk/ap/signin",
        ] {
            assert!(ok(u), "{u}");
        }
        for u in [
            "http://accounts.google.com/",
            "https://google.com/",
            "https://evil-epicgames.com/",
            "https://epicgames.com.evil.net/",
            "https://amazon.evil.com/",
            "https://www.amazon.fr.evil.net/",
            "https://exemple.com/",
            "https://store.steampowered.com.evil/",
            "http://store.steampowered.com/login/",
            "https://steampowered.com/",
            "https://api.steampowered.com/",
            "https://evil.store.steampowered.com/",
            "https://evilsteampowered.com/",
        ] {
            assert!(!ok(u), "{u}");
        }
    }

    #[test]
    fn la_connexion_steam_ouvre_la_boutique_et_ses_pages_d_identite_seulement() {
        let ok = |u: &str| site_d_identite(&tauri::Url::parse(u).unwrap());
        assert!(ok(CONNEXION_STEAM));
        for u in ["https://store.steampowered.com/explore/", "https://login.steampowered.com/jwt/finalizelogin", "https://help.steampowered.com/fr/wizard/HelpWithLogin", "https://steamcommunity.com/login/home/"] {
            assert!(ok(u), "{u}");
        }
    }

    #[test]
    fn la_fenetre_steam_ne_navigue_que_vers_les_pages_de_connexion_de_steam() {
        let ok = |u: &str| navigation_steam(&tauri::Url::parse(u).unwrap());
        assert!(ok(CONNEXION_STEAM));
        for u in [
            "https://store.steampowered.com/explore/",
            "https://STORE.steampowered.com/login/",
            "https://login.steampowered.com/jwt/finalizelogin",
            "https://help.steampowered.com/fr/wizard/HelpWithLoginInfo",
            "https://steamcommunity.com/login/home/",
        ] {
            assert!(ok(u), "{u}");
        }
        for u in [
            "http://store.steampowered.com/login/",
            "https://store.steampowered.com.evil.example/",
            "https://evil.store.steampowered.com/",
            "https://steampowered.com/",
            "https://api.steampowered.com/",
            "https://checkout.steampowered.com/",
            "https://www.valvesoftware.com/fr/privacy-policy",
            "https://store.steampowered.com:8443/",
            "https://seb@store.steampowered.com/",
            "https://accounts.google.com/",
            "file:///C:/Windows/",
        ] {
            assert!(!ok(u), "{u}");
        }
    }

    #[test]
    fn seule_la_boutique_steam_est_crue_quand_elle_dit_connecte() {
        let ok = |u: &str| page_de_la_boutique_steam(&tauri::Url::parse(u).unwrap());
        assert!(ok("https://store.steampowered.com/explore/"));
        assert!(ok(CONNEXION_STEAM));
        for u in ["https://steamcommunity.com/", "https://help.steampowered.com/", "http://store.steampowered.com/", "https://store.steampowered.com.evil.example/", "https://store.steampowered.com:444/"] {
            assert!(!ok(u), "{u}");
        }
    }

    #[test]
    fn un_steamid_valide_a_17_chiffres_d_un_compte_individuel() {
        assert!(steamid_valide("76561197960287930"));
        assert!(steamid_valide("76561198000000000"));
        for s in ["", "7656119796028793", "765611979602879301", "7656119796028793x", "76561197960265727", "12345678901234567", " 76561197960287930", "-6561197960287930"] {
            assert!(!steamid_valide(s), "{s:?}");
        }
    }

    #[test]
    fn le_titre_du_script_steam_se_lit_et_se_reverifie() {
        assert_eq!(lire_titre_steam(r#"FROGTEND:STEAM:{"connecte":true,"steamid":"76561197960287930"}"#), Some(ConnexionSteam::Connecte("76561197960287930".into())));
        assert_eq!(lire_titre_steam(r#"FROGTEND:STEAM:{"connecte":false}"#), Some(ConnexionSteam::PasConnecte));
        // Un steamid douteux, un titre d'une autre page ou d'un autre script : rien.
        assert_eq!(lire_titre_steam(r#"FROGTEND:STEAM:{"connecte":true,"steamid":"123"}"#), None);
        assert_eq!(lire_titre_steam(r#"FROGTEND:STEAM:{"connecte":true}"#), None);
        assert_eq!(lire_titre_steam("FROGTEND:STEAM:{pas du json"), None);
        assert_eq!(lire_titre_steam("FROGTEND:CONNECTE"), None);
        assert_eq!(lire_titre_steam("Bienvenue sur Steam"), None);
    }

    #[test]
    fn le_script_steam_ne_lit_ni_jeton_ni_cookie() {
        assert!(!SCRIPT_STEAM.contains("getAttribute('data-store_user_config"));
        assert!(!SCRIPT_STEAM.contains("document.cookie"));
        assert!(SCRIPT_STEAM.contains("data-userinfo"));
    }

    #[test]
    fn l_etape_sans_jeton_donne_le_bon_message() {
        use crate::erreurs::Erreur;
        let reco = Erreur::Refus(crate::boutiques::MOTIF_RECONNECTER.into());
        assert_eq!(erreur_sans_jeton_steam("deconnecte-statut-302-redirige-pas-json"), reco);
        assert_eq!(erreur_sans_jeton_steam("deconnecte-statut-200-success-21"), reco);
        assert_eq!(erreur_sans_jeton_steam("connecte-statut-401-pas-json"), reco);
        assert_eq!(erreur_sans_jeton_steam("jeton-de-forme-inattendue"), reco);
        assert_eq!(erreur_sans_jeton_steam("sans-detail"), reco);
        assert_eq!(erreur_sans_jeton_steam("connecte-statut-200-success-1"), Erreur::Serveur(MOTIF_PAGE_STEAM_CHANGEE.into()));
        assert_eq!(erreur_sans_jeton_steam("connecte-statut-200-pas-json"), Erreur::Serveur(MOTIF_PAGE_STEAM_CHANGEE.into()));
        assert_eq!(erreur_sans_jeton_steam("connecte-statut-200-success-21"), reco);
        assert_eq!(erreur_sans_jeton_steam("connecte-statut-200-redirige-pas-json"), reco);
        assert_eq!(erreur_sans_jeton_steam("sans-configuration-statut-200-pas-json"), Erreur::Serveur(MOTIF_STEAM_INHABITUEL.into()));
        assert!(matches!(erreur_sans_jeton_steam("connecte-erreur-reseau"), Erreur::Reseau(_)));
        assert!(matches!(erreur_sans_jeton_steam("sans-configuration-erreur-reseau"), Erreur::Reseau(_)));
    }

    #[test]
    fn la_remise_de_la_page_steam_se_lit_et_ne_montre_jamais_le_jeton() {
        let u = |s: &str| tauri::Url::parse(s).unwrap();
        assert_eq!(lire_remise_steam(&u("https://frogtend.invalid/steam-jeton#aaa.bbb.ccc")), Some(RemiseSteam::Jeton("aaa.bbb.ccc".into())));
        assert_eq!(lire_remise_steam(&u("https://frogtend.invalid/steam-jeton#pas-un-jwt")), Some(RemiseSteam::Absent("jeton-de-forme-inattendue".into())));
        assert_eq!(lire_remise_steam(&u("https://frogtend.invalid/steam-absent#statut-302-redirige-pas-json")), Some(RemiseSteam::Absent("statut-302-redirige-pas-json".into())));
        assert_eq!(lire_remise_steam(&u("https://frogtend.invalid/steam-absent#<script>")), Some(RemiseSteam::Absent("3Cscript3E".into())));
        assert_eq!(lire_remise_steam(&u("https://frogtend.invalid/steam-absent")), Some(RemiseSteam::Absent("sans-detail".into())));
        // Une autre adresse : navigation ordinaire (jamais prise pour une remise).
        assert_eq!(lire_remise_steam(&u("https://store.steampowered.com/steam-jeton#aaa.bbb.ccc")), None);
        assert_eq!(lire_remise_steam(&u("http://frogtend.invalid/steam-jeton#aaa.bbb.ccc")), None);
        assert_eq!(format!("{:?}", RemiseSteam::Jeton("aaa.bbb.ccc".into())), "Jeton(<caché>)");
        assert!(SCRIPT_STEAM_JETON.contains("location.replace") && !SCRIPT_STEAM_JETON.contains("document.cookie") && !SCRIPT_STEAM_JETON.contains("document.title"));
    }

    #[test]
    fn un_jeton_a_la_forme_d_un_jwt_ou_il_est_refuse() {
        assert!(forme_de_jwt("eyJhbGciOiJFZERTQSJ9.eyJzdWIiOiI3NjU2MTE5Nzk2MDI4NzkzMCJ9.c2lnbmF0dXJl"));
        for douteux in ["", "abc", "a.b", "a.b.c.d", "a..c", "a.b.c d", "a.b.<script>", "a".repeat(9000).as_str()] {
            assert!(!forme_de_jwt(douteux), "{douteux:.20}");
        }
    }

    #[test]
    fn seuls_les_jeux_gratuits_en_ce_moment_sont_retenus() {
        let v = json!({"data": {"Catalog": {"searchStore": {"elements": [
            {"title": "System Shock 2", "price": {"totalPrice": {"discountPrice": 0}},
             "promotions": {"promotionalOffers": [{"promotionalOffers": [{"startDate": "2026-10-01T15:00:00.000Z", "endDate": "2026-10-08T15:00:00.000Z"}]}]},
             "namespace": "02d659f31e1c43a78c43fd9fd9edc089",
             "catalogNs": {"mappings": [{"pageSlug": "system-shock-2-25th-anniversary-remaster-cb94d9"}]},
             "keyImages": [{"type": "Thumbnail", "url": "a"}, {"type": "OfferImageTall", "url": "b"}]},
            {"title": "Payant", "price": {"totalPrice": {"discountPrice": 1999}}},
            {"title": "À venir", "price": {"totalPrice": {"discountPrice": 0}},
             "promotions": {"promotionalOffers": [], "upcomingPromotionalOffers": [{}]}},
            {"title": "Slug douteux", "price": {"totalPrice": {"discountPrice": 0}},
             "promotions": {"promotionalOffers": [{"promotionalOffers": [{"startDate": "2026-10-01T15:00:00.000Z", "endDate": "2026-10-08T15:00:00.000Z"}]}]},
             "catalogNs": {"mappings": [{"pageSlug": "../../evil"}]}}
        ]}}}});
        let l = lire_offerts_epic(&v, "2026-10-02T03:00:00.000Z");
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].slug, "system-shock-2-25th-anniversary-remaster-cb94d9");
        assert_eq!(l[0].image.as_deref(), Some("b"));
        assert_eq!(l[0].namespace, "02d659f31e1c43a78c43fd9fd9edc089", "l'espace de noms, pour reconnaître un jeu déjà possédé");
        assert!(lire_offerts_epic(&v, "2026-10-09T00:00:00.000Z").is_empty(), "offre terminée");
    }

    #[test]
    fn obtenu_seulement_avec_la_preuve_d_epic() {
        assert_eq!(apres_controle(Some(Obtention::Deja)), Obtention::Obtenu);
        assert!(matches!(apres_controle(None), Obtention::Erreur(_)));
        assert!(matches!(apres_controle(Some(Obtention::Obtenu)), Obtention::Erreur(_)), "un nouvel OBTENU n'est pas une preuve");
        assert_eq!(apres_controle(Some(Obtention::Erreur("x".into()))), Obtention::Erreur("x".into()));
        assert!(script_epic_controle().starts_with("window.__frogtendMode = 'controle';"));
    }

    #[test]
    fn les_obtenus_epic_de_la_0_214_4_sont_retires() {
        let o = Obtenus {
            jeux: vec![
                ("epic".into(), "mark-of-the-deep-f7dc08".into(), "1795188632".into()),
                ("epic".into(), "ancien".into(), "1795000000".into()),
                ("gog".into(), "recent".into(), "1795188632".into()),
            ],
            ..Default::default()
        };
        let l = sans_obtenus_douteux(o.clone());
        assert_eq!(l.jeux.iter().map(|j| j.1.as_str()).collect::<Vec<_>>(), vec!["ancien", "recent"]);
        // Version 2 (écrite depuis la 0.214.5) : rien n'est retiré.
        assert_eq!(sans_obtenus_douteux(Obtenus { version: 2, ..o.clone() }).jeux.len(), 3);
        let d = std::env::temp_dir().join(format!("frogtend-obtenus-{}", std::process::id()));
        ecrire_obtenus(&d, &l).unwrap();
        assert_eq!(lire_obtenus(&d).version, 2);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn la_verification_epic_ne_retient_que_connecte_ou_connexion() {
        assert_eq!(compte_epic_vu(&Obtention::Connecte), Some(true));
        assert_eq!(compte_epic_vu(&Obtention::Connexion), Some(false));
        assert_eq!(compte_epic_vu(&Obtention::Captcha), None);
        assert_eq!(compte_epic_vu(&Obtention::Erreur("x".into())), None);
        assert!(script_epic_verification().starts_with("window.__frogtendMode = 'verification';"));
    }

    #[test]
    fn le_titre_de_la_page_dit_le_resultat() {
        assert_eq!(lire_titre("FROGTEND:OBTENU"), Some(Obtention::Obtenu));
        assert_eq!(lire_titre("FROGTEND:CAPTCHA"), Some(Obtention::Captcha));
        assert_eq!(lire_titre("FROGTEND:ERREUR:délai dépassé"), Some(Obtention::Erreur("délai dépassé".into())));
        assert_eq!(lire_titre("Epic Games Store"), None);
        assert!(SCRIPT_EPIC.contains("purchase-cta-button") && SCRIPT_EPIC.contains("FROGTEND:"));
        // Nouvelle boutique (18/11) : DEJA, CONNECTE, et l'étape de l'erreur avec les boutons vus.
        assert_eq!(lire_titre("FROGTEND:DEJA"), Some(Obtention::Deja));
        assert_eq!(lire_titre("FROGTEND:CONNEXION"), Some(Obtention::Connexion));
        assert_eq!(lire_titre("FROGTEND:CONNECTE"), Some(Obtention::Connecte));
        assert_eq!(
            lire_titre("FROGTEND:ERREUR:pas de bouton d'achat | wishlist ; share"),
            Some(Obtention::Erreur("pas de bouton d'achat | wishlist ; share".into()))
        );
        assert_eq!(serde_json::to_value(Obtention::Connecte).unwrap(), json!({"etat": "connecte"}));
        assert_eq!(serde_json::to_value(Obtention::Erreur("pas de commande | x".into())).unwrap(), json!({"etat": "erreur", "motif": "pas de commande | x"}));
        // Le mode connexion ne clique sur rien : il est posé AVANT le script.
        assert!(script_epic_connexion().starts_with("window.__frogtendMode = 'connexion';") && script_epic_connexion().ends_with(SCRIPT_EPIC));
    }

    #[test]
    fn le_script_epic_ne_clique_que_par_sa_garde() {
        // Tous les clics passent par `cliquer`, qui refuse un bouton payant ; seule exception, la case du contrat.
        let s = SCRIPT_EPIC.replace("\r\n", "\n");
        assert_eq!(s.matches(".click()").count(), 2, "e.click() dans `cliquer`, c.click() pour la case du contrat");
        assert!(s.contains("if (payant(texte(e))) throw") && s.contains("c.click();"));
    }

    #[test]
    fn le_bilan_ps_plus_se_lit_dans_le_titre() {
        assert_eq!(lire_titre_psplus("FROGTEND:PSPLUS:2:3:7"), Some(RecoltePsPlus::Faite { ajoutes: 2, deja: 3, vus: 7 }));
        assert_eq!(lire_titre_psplus("FROGTEND:CONNEXION"), Some(RecoltePsPlus::Connexion));
        assert_eq!(lire_titre_psplus("FROGTEND:ERREUR:délai dépassé"), Some(RecoltePsPlus::Erreur { motif: "délai dépassé".into() }));
        assert_eq!(lire_titre_psplus("FROGTEND:PSPLUS:x"), Some(RecoltePsPlus::Erreur { motif: "PSPLUS:x".into() }));
        assert_eq!(lire_titre_psplus("PlayStation Store"), None);
    }

    #[test]
    fn le_script_ps_plus_ne_peut_rien_acheter() {
        // Un seul clic dans tout le script, sur « Ajouter à la bibliothèque », et seulement sur le Store officiel.
        assert!(SCRIPT_PSPLUS.contains("location.hostname !== 'store.playstation.com'"));
        assert_eq!(SCRIPT_PSPLUS.matches(".click()").count(), 1);
        assert!(SCRIPT_PSPLUS.replace("\r\n", "\n").contains("if (bouton && estAjout(bouton)) {\n        bouton.click();"));
        assert!(PAGE_PSPLUS.starts_with("https://store.playstation.com/"));
    }

    #[test]
    fn le_bilan_gog_ou_prime_se_lit_dans_le_titre() {
        let r = lire_titre_recolte(r#"FROGTEND:{"etat":"faite","obtenus":["Beyond Good & Evil"],"deja":0}"#).unwrap();
        assert_eq!((r.etat.as_str(), r.obtenus.len(), r.a_finir), ("faite", 1, 0));
        assert_eq!(lire_titre_recolte(r#"FROGTEND:{"etat":"connexion"}"#).unwrap().etat, "connexion");
        assert_eq!(lire_titre_recolte("FROGTEND:pas du json").unwrap().etat, "erreur");
        assert_eq!(lire_titre_recolte("GOG.com"), None);
        // Les scripts n'agissent que sur la page officielle, et GOG passe par l'adresse officielle de récupération.
        assert!(SCRIPT_GOG.contains("location.hostname !== 'www.gog.com'") && SCRIPT_GOG.contains("'/giveaway/claim'"));
        assert!(SCRIPT_PRIME.contains("location.hostname !== 'gaming.amazon.com'"));
        assert!(boutique_offerte("prime").is_some() && boutique_offerte("epic").is_none());
    }

    /// Encode en base64url sans remplissage (comme `gog-jeux.js`).
    fn b64(o: &[u8]) -> String {
        const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let mut s = String::new();
        for bloc in o.chunks(3) {
            let n = (bloc[0] as u32) << 16 | (*bloc.get(1).unwrap_or(&0) as u32) << 8 | *bloc.get(2).unwrap_or(&0) as u32;
            for i in 0..=bloc.len() {
                s.push(A[(n >> (18 - 6 * i) & 63) as usize] as char);
            }
        }
        s
    }

    fn morceau(k: usize, n: usize, jeux: serde_json::Value) -> tauri::Url {
        tauri::Url::parse(&format!("https://frogtend.invalid/gog-jeux#{k}/{n}/{}", b64(jeux.to_string().as_bytes()))).unwrap()
    }

    #[test]
    fn la_remise_de_gog_est_lue_strictement() {
        let u = |s: &str| tauri::Url::parse(s).unwrap();
        assert_eq!(lire_remise_gog(&u("https://frogtend.invalid/gog-compte#48628349957132247")), Some(RemiseGog::Compte("48628349957132247".into())));
        assert_eq!(lire_remise_gog(&u("https://frogtend.invalid/gog-compte#abc")), Some(RemiseGog::Absent("compte-de-forme-inattendue".into())));
        assert_eq!(lire_remise_gog(&u("https://frogtend.invalid/gog-compte")), Some(RemiseGog::Absent("compte-de-forme-inattendue".into())));
        // Un morceau juste : titres accentués gardés (UTF-8), espaces retirés.
        let m = lire_remise_gog(&morceau(0, 2, json!([["1207658924", " Unreal Tournament™ ", "//images-1.gog-statics.com/abc"], ["2", "Été", ""]])));
        assert_eq!(
            m,
            Some(RemiseGog::Morceau {
                k: 0,
                n: 2,
                jeux: vec![
                    JeuGogLu { id: "1207658924".into(), titre: "Unreal Tournament™".into(), image: "//images-1.gog-statics.com/abc".into() },
                    JeuGogLu { id: "2".into(), titre: "Été".into(), image: String::new() }
                ]
            })
        );
        // Une forme inattendue : tout le morceau est refusé (jamais une liste devinée).
        let refuse = Some(RemiseGog::Absent("morceau-de-forme-inattendue".into()));
        assert_eq!(lire_remise_gog(&morceau(0, 1, json!([["12a", "Jeu", ""]]))), refuse, "id non numérique");
        assert_eq!(lire_remise_gog(&morceau(0, 1, json!([[12, "Jeu", ""]]))), refuse, "id qui n'est pas un texte");
        assert_eq!(lire_remise_gog(&morceau(0, 1, json!([["12", "  ", ""]]))), refuse, "titre vide");
        assert_eq!(lire_remise_gog(&morceau(0, 1, json!([["12", "Jeu"]]))), refuse, "champ manquant");
        assert_eq!(lire_remise_gog(&morceau(0, 1, json!({"jeux": []}))), refuse, "pas une liste");
        assert_eq!(lire_remise_gog(&morceau(1, 1, json!([]))), refuse, "k >= n");
        assert_eq!(lire_remise_gog(&morceau(0, 0, json!([]))), refuse, "n = 0");
        assert_eq!(lire_remise_gog(&morceau(0, MORCEAUX_GOG_MAX + 1, json!([]))), refuse, "trop de morceaux");
        assert_eq!(lire_remise_gog(&u("https://frogtend.invalid/gog-jeux#0/1/%%%")), refuse, "pas du base64");
        assert_eq!(lire_remise_gog(&u("https://frogtend.invalid/gog-jeux#x/1/W10")), refuse);
        // Les étapes d'échec : seulement des caractères sûrs.
        assert_eq!(lire_remise_gog(&u("https://frogtend.invalid/gog-absent#deconnecte")), Some(RemiseGog::Absent("deconnecte".into())));
        assert_eq!(lire_remise_gog(&u("https://frogtend.invalid/gog-absent#<script>")), Some(RemiseGog::Absent("3Cscript3E".into())));
        assert_eq!(lire_remise_gog(&u("https://frogtend.invalid/gog-absent")), Some(RemiseGog::Absent("sans-detail".into())));
        assert_eq!(lire_remise_gog(&u("https://frogtend.invalid/gog-progres#15")), Some(RemiseGog::Progres(15)));
        assert_eq!(lire_remise_gog(&u("https://frogtend.invalid/gog-progres#x")), Some(RemiseGog::Progres(0)));
        // Pas une remise : navigation ordinaire.
        assert_eq!(lire_remise_gog(&u("https://www.gog.com/gog-compte#123")), None);
        assert_eq!(lire_remise_gog(&u("http://frogtend.invalid/gog-compte#123")), None);
        assert_eq!(lire_remise_gog(&u("https://frogtend.invalid:8443/gog-compte#123")), None);
        // Le numéro du compte ne s'écrit jamais (journal, message d'un test).
        assert_eq!(format!("{:?}", RemiseGog::Compte("48628349957132247".into())), "Compte(<caché>)");
        assert!(!format!("{:?}", lire_remise_gog(&morceau(0, 1, json!([["5", "Secret de titre", ""]])))).contains("Secret"));
    }

    #[test]
    fn la_liste_gog_est_recue_en_entier_et_dans_l_ordre() {
        let jeu = |id: &str| JeuGogLu { id: id.into(), titre: format!("Jeu {id}"), image: String::new() };
        let m = |k, n, ids: &[&str]| RemiseGog::Morceau { k, n, jeux: ids.iter().map(|i| jeu(i)).collect() };
        // Le compte, puis les morceaux demandés un à un ; un jeu en double (visible ET caché) n'est gardé qu'une fois.
        let mut r = ReceptionGog::default();
        assert_eq!(r.recevoir(RemiseGog::Compte("42".into())), SuiteGog::Demander(0));
        assert_eq!(r.recevoir(m(0, 3, &["1", "2"])), SuiteGog::Demander(1));
        assert_eq!(r.recevoir(m(1, 3, &["3", "2"])), SuiteGog::Demander(2));
        assert_eq!(r.recevoir(m(2, 3, &["4"])), SuiteGog::Fini("42".into(), vec![jeu("1"), jeu("2"), jeu("3"), jeu("4")]));
        // Le bilan (0.217.1) : gardé pour le journal, sans rien changer à la suite ; filtré à des caractères sûrs.
        let mut r = ReceptionGog::default();
        let b = lire_remise_gog(&tauri::Url::parse("https://frogtend.invalid/gog-bilan#visibles-pages-1-gardes-88<x>").unwrap()).unwrap();
        assert_eq!(r.recevoir(b), SuiteGog::Attendre);
        assert_eq!(r.bilan().as_deref(), Some("visibles-pages-1-gardes-883Cx3E"));
        // Le jeton (sonde 0.217.3) : gardé en mémoire sans rien changer à la suite ; son Debug ne montre rien.
        let j = lire_remise_gog(&tauri::Url::parse("https://frogtend.invalid/gog-jeton#abcdefghij.KLMNOPQRST_uvwxyz-0123").unwrap()).unwrap();
        assert!(!format!("{j:?}").contains("abcdef"));
        assert_eq!(r.recevoir(j), SuiteGog::Attendre);
        assert_eq!(r.recevoir(RemiseGog::Compte("42".into())), SuiteGog::Demander(0));
        let (jeton, illisible) = r.prendre_jeton();
        assert_eq!(jeton.map(|j| j.0.len()), Some(33));
        assert!(!illisible);
        assert_eq!(lire_remise_gog(&tauri::Url::parse("https://frogtend.invalid/gog-jeton#court").unwrap()), Some(RemiseGog::Jeton(None)));
        // Un compte sans aucun jeu : un seul morceau vide.
        let mut r = ReceptionGog::default();
        r.recevoir(RemiseGog::Compte("42".into()));
        assert_eq!(r.recevoir(m(0, 1, &[])), SuiteGog::Fini("42".into(), vec![]));
        // Morceau manquant ou dans le désordre, nombre de morceaux qui change, morceau avant le compte : échec.
        let mut r = ReceptionGog::default();
        r.recevoir(RemiseGog::Compte("42".into()));
        r.recevoir(m(0, 3, &["1"]));
        assert_eq!(r.recevoir(m(2, 3, &["3"])), SuiteGog::Echec("morceau-dans-le-desordre".into()));
        let mut r = ReceptionGog::default();
        r.recevoir(RemiseGog::Compte("42".into()));
        assert_eq!(r.recevoir(m(1, 2, &["1"])), SuiteGog::Echec("morceau-dans-le-desordre".into()));
        let mut r = ReceptionGog::default();
        r.recevoir(RemiseGog::Compte("42".into()));
        r.recevoir(m(0, 3, &["1"]));
        assert_eq!(r.recevoir(m(1, 2, &["2"])), SuiteGog::Echec("nombre-de-morceaux-change".into()));
        let mut r = ReceptionGog::default();
        assert_eq!(r.recevoir(m(0, 1, &["1"])), SuiteGog::Echec("morceau-avant-le-compte".into()));
        let mut r = ReceptionGog::default();
        r.recevoir(RemiseGog::Compte("42".into()));
        assert_eq!(r.recevoir(RemiseGog::Compte("43".into())), SuiteGog::Echec("autre-compte-pendant-la-lecture".into()));
        // La page rechargée avant le premier morceau : le même compte redonné, le morceau 0 redemandé, puis tout suit.
        let mut r = ReceptionGog::default();
        assert_eq!(r.recevoir(RemiseGog::Progres(5)), SuiteGog::Attendre);
        r.recevoir(RemiseGog::Compte("42".into()));
        assert_eq!(r.recevoir(RemiseGog::Progres(5)), SuiteGog::Attendre, "le script relancé avance");
        assert_eq!(r.recevoir(RemiseGog::Compte("42".into())), SuiteGog::Demander(0));
        assert_eq!(r.recevoir(m(0, 1, &["1"])), SuiteGog::Fini("42".into(), vec![jeu("1")]));
        // Rechargée APRÈS un morceau : échec clair (jamais deux listes mêlées).
        let mut r = ReceptionGog::default();
        r.recevoir(RemiseGog::Compte("42".into()));
        r.recevoir(m(0, 2, &["1"]));
        assert_eq!(r.recevoir(RemiseGog::Compte("42".into())), SuiteGog::Echec("page-rechargee-pendant-la-remise".into()));
        let mut r = ReceptionGog::default();
        r.recevoir(RemiseGog::Compte("42".into()));
        r.recevoir(m(0, 2, &["1"]));
        assert_eq!(r.recevoir(RemiseGog::Progres(5)), SuiteGog::Echec("page-rechargee-pendant-la-remise".into()));
        // La page dit pourquoi elle n'a rien : remis tel quel.
        assert_eq!(ReceptionGog::default().recevoir(RemiseGog::Absent("deconnecte".into())), SuiteGog::Absent("deconnecte".into()));
    }

    #[test]
    fn une_lecture_gog_sans_liste_dit_le_bon_message() {
        use crate::erreurs::Erreur;
        assert!(erreur_sans_jeux_gog("deconnecte").is_none(), "seule la page qui dit « pas connecté » mène à « reconnecte-toi »");
        assert!(matches!(erreur_sans_jeux_gog("trop-de-demandes"), Some(Erreur::Reseau(m)) if m.contains("attendre")));
        assert!(matches!(erreur_sans_jeux_gog("erreur-reseau"), Some(Erreur::Reseau(m)) if m.contains("Internet")));
        for e in ["page-changee", "statut-500", "pas-json", "sans-detail", "compte-de-forme-inattendue"] {
            assert!(matches!(erreur_sans_jeux_gog(e), Some(Erreur::Serveur(m)) if m == MOTIF_GOG_INHABITUEL), "{e}");
        }
    }

    #[test]
    fn la_fenetre_cachee_de_gog_ne_va_que_sur_www_gog_com() {
        let ok = |s: &str| navigation_gog(&tauri::Url::parse(s).unwrap());
        assert!(ok("https://www.gog.com/en") && ok("https://www.gog.com/account/getFilteredProducts?page=1") && ok("https://WWW.GOG.COM/"));
        for piege in [
            "http://www.gog.com/en",
            "https://www.gog.com:8443/en",
            "https://www.gog.com.evil.example/",
            "https://evil.example/www.gog.com",
            "https://gog.com/",
            "https://login.gog.com/auth",
            "https://moi@www.gog.com/",
            "https://frogtend.invalid/gog-compte#1",
        ] {
            assert!(!ok(piege), "{piege}");
        }
        // Le script n'agit que sur la page officielle et ne parle qu'à la boutique elle-même.
        assert!(SCRIPT_GOG_JEUX.contains("location.hostname !== 'www.gog.com'") && SCRIPT_GOG_JEUX.contains("credentials: 'same-origin'"));
        assert!(navigation_gog(&tauri::Url::parse(PAGE_GOG).unwrap()));
    }

    #[test]
    fn l_heure_iso_est_bien_formee() {
        let m = maintenant_iso();
        assert_eq!(m.len(), 24);
        assert!(m.starts_with("20") && m.ends_with(".000Z") && m.as_bytes()[10] == b'T');
    }
}

#[cfg(test)]
mod essais {
    /// La vraie liste publique d'Epic (lecture seule, sans compte).
    #[tokio::test]
    #[ignore]
    async fn essai_offerts_epic() {
        for j in super::offerts_epic().await.unwrap() {
            println!("{} | {} | fin {} | image {}", j.titre, j.slug, j.fin, j.image.is_some());
        }
    }
}
