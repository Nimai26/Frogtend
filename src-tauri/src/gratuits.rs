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

/// Les sites d'IDENTITÉ qu'une fenêtre de connexion peut ouvrir (connexion par Google, Apple, Microsoft, Steam, la
/// boutique elle-même…), en HTTPS seulement (revue des experts, 18/11 : jamais « tout site en https »).
pub fn site_d_identite(url: &tauri::Url) -> bool {
    if url.scheme() != "https" {
        return false;
    }
    let Some(h) = url.host_str().map(|h| h.trim_end_matches('.').to_ascii_lowercase()) else { return false };
    let sous = |d: &str| h == d || h.ends_with(&format!(".{d}"));
    const EXACTS: [&str; 5] = ["accounts.google.com", "appleid.apple.com", "login.live.com", "login.microsoftonline.com", "steamcommunity.com"];
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
}

pub fn lire_obtenus(dossier_profil: &Path) -> Obtenus {
    std::fs::read(dossier_profil.join("gratuits.json")).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default()
}

pub fn ecrire_obtenus(dossier_profil: &Path, o: &Obtenus) -> Resultat<()> {
    std::fs::create_dir_all(dossier_profil)?;
    std::fs::write(dossier_profil.join("gratuits.json"), serde_json::to_vec_pretty(o).unwrap_or_default())?;
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
        ] {
            assert!(!ok(u), "{u}");
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
