//! Le client de l'API de Firehouse (`/api/jeux/v1/`).
//!
//! Le jeton part dans l'en-tête `Authorization` et nulle part ailleurs. Les erreurs sont traduites en motifs
//! lisibles, sans chemin d'API ni jeton.

use crate::erreurs::{Erreur, Resultat};
use serde::de::DeserializeOwned;
use std::net::Ipv4Addr;
use std::time::Duration;

pub const PREFIXE: &str = "/api/jeux/v1";

/// Vrai pour une machine du réseau de la maison (10/8, 172.16/12, 192.168/16, 127/8, localhost, `.local`).
pub fn est_adresse_locale(hote: &str) -> bool {
    let h = hote.trim_matches(|c| c == '[' || c == ']').to_ascii_lowercase();
    if h == "localhost" || h == "::1" || h.ends_with(".local") {
        return true;
    }
    match h.parse::<Ipv4Addr>() {
        Ok(ip) => ip.is_private() || ip.is_loopback(),
        Err(_) => false,
    }
}

/// Vérifie et nettoie l'adresse de Firehouse. HTTPS obligatoire hors du réseau de la maison.
pub fn verifier_adresse(adresse: &str) -> Resultat<String> {
    let a = adresse.trim().trim_end_matches('/');
    let url = reqwest::Url::parse(a)
        .map_err(|_| Erreur::Reglage("L'adresse de Firehouse n'est pas une adresse web valable.".into()))?;
    let hote = url.host_str().unwrap_or_default();
    match url.scheme() {
        "https" => Ok(a.into()),
        "http" if est_adresse_locale(hote) => Ok(a.into()),
        "http" => Err(Erreur::Reglage(
            "Hors du réseau de la maison, l'adresse de Firehouse doit être en https://.".into(),
        )),
        _ => Err(Erreur::Reglage("L'adresse de Firehouse doit commencer par https://.".into())),
    }
}

/// Une réponse brute : utile quand on veut l'`ETag` ou les octets (images).
#[derive(Debug)]
pub enum Reponse {
    Corps { octets: Vec<u8>, etag: Option<String>, type_contenu: Option<String> },
    /// 304 : rien n'a changé depuis l'`ETag` donné.
    NonModifie,
}

pub struct Client {
    base: String,
    jeton: String,
    http: reqwest::Client,
}

/// Le motif envoyé par Firehouse (`{"detail": "..."}`), s'il y en a un lisible.
fn motif_du_serveur(corps: &[u8]) -> Option<String> {
    let v: serde_json::Value = serde_json::from_slice(corps).ok()?;
    let d = v.get("detail").or_else(|| v.get("message")).or_else(|| v.get("erreur"))?;
    let texte = match d {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(liste) => liste
            .iter()
            .filter_map(|x| x.get("msg").and_then(|m| m.as_str()).map(String::from))
            .collect::<Vec<_>>()
            .join(" ; "),
        _ => return None,
    };
    // Jamais de chemin d'API dans un message.
    let propre: String = texte.split_whitespace().filter(|m| !m.starts_with("/api/")).collect::<Vec<_>>().join(" ");
    (!propre.is_empty()).then_some(propre)
}

/// Traduit un code HTTP d'erreur en erreur lisible.
pub fn erreur_du_statut(code: u16, corps: &[u8]) -> Erreur {
    let motif = motif_du_serveur(corps);
    match code {
        401 => Erreur::JetonRefuse(
            "Firehouse refuse le jeton de ce profil : il a été révoqué ou il a expiré. Demande un nouveau jeton à un admin."
                .into(),
        ),
        404 => Erreur::Introuvable("Firehouse ne connaît pas ce jeu (ou il ne t'est pas visible).".into()),
        409 => Erreur::Conflit(motif.unwrap_or_else(|| "Firehouse signale un conflit : réessaie plus tard.".into())),
        400..=499 => Erreur::Refus(motif.unwrap_or_else(|| format!("Firehouse a refusé la demande (code {code})."))),
        _ => Erreur::Serveur(format!("Firehouse a rencontré un problème (code {code}). Réessaie plus tard.")),
    }
}

impl Client {
    pub fn nouveau(adresse: &str, jeton: &str) -> Resultat<Self> {
        let base = verifier_adresse(adresse)?;
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(60))
            .user_agent(concat!("Frogtend/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| Erreur::Reseau("Impossible de préparer la connexion.".into()))?;
        Ok(Client { base, jeton: jeton.into(), http })
    }

    fn url(&self, route: &str) -> String {
        format!("{}{PREFIXE}{route}", self.base)
    }

    fn erreur_reseau(e: reqwest::Error) -> Erreur {
        if e.is_timeout() {
            Erreur::Reseau("Firehouse ne répond pas (délai dépassé). Vérifie ta connexion.".into())
        } else {
            Erreur::Reseau("Impossible de joindre Firehouse. Vérifie ta connexion et l'adresse dans les réglages.".into())
        }
    }

    /// `GET` d'une route (`/plateformes`…), avec un `ETag` éventuel.
    pub async fn obtenir(&self, route: &str, etag: Option<&str>) -> Resultat<Reponse> {
        let mut req = self.http.get(self.url(route)).bearer_auth(&self.jeton);
        if let Some(e) = etag {
            req = req.header(reqwest::header::IF_NONE_MATCH, e);
        }
        let rep = req.send().await.map_err(Self::erreur_reseau)?;
        let statut = rep.status().as_u16();
        if statut == 304 {
            return Ok(Reponse::NonModifie);
        }
        let entete = |nom: reqwest::header::HeaderName| {
            rep.headers().get(nom).and_then(|v| v.to_str().ok()).map(String::from)
        };
        let etag = entete(reqwest::header::ETAG);
        let type_contenu = entete(reqwest::header::CONTENT_TYPE);
        let octets = rep.bytes().await.map_err(Self::erreur_reseau)?.to_vec();
        if !(200..300).contains(&statut) {
            return Err(erreur_du_statut(statut, &octets));
        }
        Ok(Reponse::Corps { octets, etag, type_contenu })
    }

    /// `GET` d'une route JSON.
    pub async fn obtenir_json<T: DeserializeOwned>(&self, route: &str) -> Resultat<T> {
        match self.obtenir(route, None).await? {
            Reponse::Corps { octets, .. } => serde_json::from_slice(&octets)
                .map_err(|_| Erreur::Serveur("Firehouse a envoyé une réponse illisible.".into())),
            Reponse::NonModifie => Err(Erreur::Serveur("Réponse inattendue de Firehouse.".into())),
        }
    }

    /// `POST` d'un objet JSON.
    pub async fn envoyer_json<T: DeserializeOwned>(&self, route: &str, corps: &serde_json::Value) -> Resultat<T> {
        let rep = self
            .http
            .post(self.url(route))
            .bearer_auth(&self.jeton)
            .json(corps)
            .send()
            .await
            .map_err(Self::erreur_reseau)?;
        let statut = rep.status().as_u16();
        let octets = rep.bytes().await.map_err(Self::erreur_reseau)?;
        if !(200..300).contains(&statut) {
            return Err(erreur_du_statut(statut, &octets));
        }
        serde_json::from_slice(&octets).map_err(|_| Erreur::Serveur("Firehouse a envoyé une réponse illisible.".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    #[test]
    fn https_partout_http_seulement_a_la_maison() {
        assert_eq!(verifier_adresse("https://jeux.hikari-no-sekai.fr/").unwrap(), "https://jeux.hikari-no-sekai.fr");
        assert!(verifier_adresse("http://10.10.0.2:8100").is_ok());
        assert!(verifier_adresse("http://127.0.0.1:9").is_ok());
        assert!(matches!(verifier_adresse("http://jeux.hikari-no-sekai.fr"), Err(Erreur::Reglage(_))));
        assert!(matches!(verifier_adresse("http://8.8.8.8"), Err(Erreur::Reglage(_))));
        assert!(matches!(verifier_adresse("pas une adresse"), Err(Erreur::Reglage(_))));
    }

    #[test]
    fn les_codes_d_erreur_deviennent_des_motifs_lisibles() {
        assert!(matches!(erreur_du_statut(401, b""), Erreur::JetonRefuse(_)));
        assert!(matches!(erreur_du_statut(404, b""), Erreur::Introuvable(_)));
        assert_eq!(
            erreur_du_statut(409, br#"{"detail":"fichier abime"}"#),
            Erreur::Conflit("fichier abime".into())
        );
        assert_eq!(
            erreur_du_statut(403, br#"{"detail":"grade insuffisant"}"#),
            Erreur::Refus("grade insuffisant".into())
        );
        assert!(matches!(erreur_du_statut(502, b"<html>"), Erreur::Serveur(_)));
    }

    #[test]
    fn un_motif_ne_montre_jamais_de_chemin_d_api() {
        let e = erreur_du_statut(400, br#"{"detail":"route /api/jeux/v1/jeu/12 invalide"}"#);
        assert_eq!(e, Erreur::Refus("route invalide".into()));
    }

    #[tokio::test]
    async fn le_jeton_part_dans_l_en_tete_et_le_json_est_lu() {
        let serveur = MockServer::start();
        let m = serveur.mock(|when, then| {
            when.method(GET).path("/api/jeux/v1/plateformes").header("authorization", "Bearer jeton-test");
            then.status(200).json_body(serde_json::json!([{"nom": "MS-DOS", "jeux": 3}]));
        });
        let c = Client::nouveau(&serveur.base_url(), "jeton-test").unwrap();
        let v: serde_json::Value = c.obtenir_json("/plateformes").await.unwrap();
        assert_eq!(v[0]["nom"], "MS-DOS");
        m.assert();
    }

    #[tokio::test]
    async fn un_401_devient_jeton_refuse_sans_nouvel_essai() {
        let serveur = MockServer::start();
        let m = serveur.mock(|when, then| {
            when.method(GET).path("/api/jeux/v1/catalogue");
            then.status(401).json_body(serde_json::json!({"detail": "connexion requise"}));
        });
        let c = Client::nouveau(&serveur.base_url(), "vieux").unwrap();
        let e = c.obtenir_json::<serde_json::Value>("/catalogue").await.unwrap_err();
        assert!(matches!(e, Erreur::JetonRefuse(_)));
        m.assert_hits(1);
    }

    #[tokio::test]
    async fn l_etag_donne_un_304_non_modifie() {
        let serveur = MockServer::start();
        serveur.mock(|when, then| {
            when.method(GET).path("/api/jeux/v1/themes").header("if-none-match", "\"v1\"");
            then.status(304);
        });
        serveur.mock(|when, then| {
            when.method(GET).path("/api/jeux/v1/themes");
            then.status(200).header("etag", "\"v1\"").body("{}");
        });
        let c = Client::nouveau(&serveur.base_url(), "j").unwrap();
        match c.obtenir("/themes", None).await.unwrap() {
            Reponse::Corps { etag, .. } => assert_eq!(etag.as_deref(), Some("\"v1\"")),
            r => panic!("{r:?}"),
        }
        assert!(matches!(c.obtenir("/themes", Some("\"v1\"")).await.unwrap(), Reponse::NonModifie));
    }

    #[tokio::test]
    async fn un_serveur_injoignable_est_une_erreur_reseau() {
        // Port 9 (discard) de la machine elle-même : personne n'écoute.
        let c = Client::nouveau("http://127.0.0.1:9", "j").unwrap();
        let e = c.obtenir_json::<serde_json::Value>("/plateformes").await.unwrap_err();
        assert!(matches!(e, Erreur::Reseau(_)), "{e:?}");
    }
}
