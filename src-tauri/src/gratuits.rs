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
    Erreur(String),
}

pub fn lire_titre(titre: &str) -> Option<Obtention> {
    let r = titre.strip_prefix("FROGTEND:")?;
    Some(match r {
        "OBTENU" => Obtention::Obtenu,
        "DEJA" => Obtention::Deja,
        "CONNEXION" => Obtention::Connexion,
        "CAPTCHA" => Obtention::Captcha,
        autre => Obtention::Erreur(autre.strip_prefix("ERREUR:").unwrap_or(autre).to_string()),
    })
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
    fn seuls_les_jeux_gratuits_en_ce_moment_sont_retenus() {
        let v = json!({"data": {"Catalog": {"searchStore": {"elements": [
            {"title": "System Shock 2", "price": {"totalPrice": {"discountPrice": 0}},
             "promotions": {"promotionalOffers": [{"promotionalOffers": [{"startDate": "2026-10-01T15:00:00.000Z", "endDate": "2026-10-08T15:00:00.000Z"}]}]},
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
        assert!(lire_offerts_epic(&v, "2026-10-09T00:00:00.000Z").is_empty(), "offre terminée");
    }

    #[test]
    fn le_titre_de_la_page_dit_le_resultat() {
        assert_eq!(lire_titre("FROGTEND:OBTENU"), Some(Obtention::Obtenu));
        assert_eq!(lire_titre("FROGTEND:CAPTCHA"), Some(Obtention::Captcha));
        assert_eq!(lire_titre("FROGTEND:ERREUR:délai dépassé"), Some(Obtention::Erreur("délai dépassé".into())));
        assert_eq!(lire_titre("Epic Games Store"), None);
        assert!(SCRIPT_EPIC.contains("purchase-cta-button") && SCRIPT_EPIC.contains("FROGTEND:"));
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
