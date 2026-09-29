//! D'où viennent les données : Firehouse (le vrai serveur), ou le mode simulé (fixtures, sans réseau).

use crate::erreurs::{Erreur, Resultat};
use crate::firehouse::{Client, Reponse};
use serde_json::{json, Value};

const TAILLE_PAGE_SIMULEE: usize = 25;
/// Jeux demandés par page au vrai Firehouse.
const PAR_PAGE: u32 = 100;

/// `/jeu/{id}` répond `{ok, jeu: {...}}` : on rend la fiche elle-même.
pub fn deballer_fiche(v: Value) -> Value {
    match v.get("jeu") {
        Some(j) if j.is_object() => j.clone(),
        _ => v,
    }
}
const CATALOGUE_SIMULE: &str = include_str!("../fixtures/simule/catalogue.json");
const FICHE_110: &str = include_str!("../fixtures/simule/jeu-110.json");

pub enum Source {
    Firehouse(Client),
    Simulee,
}

/// Une image reçue.
pub struct Image {
    pub octets: Vec<u8>,
    pub type_contenu: Option<String>,
}

fn catalogue_simule() -> Vec<Value> {
    let v: Value = serde_json::from_str(CATALOGUE_SIMULE).expect("fixture catalogue illisible");
    v["jeux"].as_array().cloned().unwrap_or_default()
}

/// Une fiche simulée : celle de Dune (le vrai exemple du contrat), sinon une fiche tirée du catalogue.
fn fiche_simulee(id: i64) -> Resultat<Value> {
    if id == 110 {
        return Ok(serde_json::from_str(FICHE_110).unwrap());
    }
    let j = catalogue_simule()
        .into_iter()
        .find(|j| j["id"].as_i64() == Some(id))
        .ok_or_else(|| Erreur::Introuvable("Firehouse ne connaît pas ce jeu (ou il ne t'est pas visible).".into()))?;
    let mut fiche = j.clone();
    fiche["resume"] = json!(format!(
        "Fiche d'exemple du mode simulé pour « {} ». Le vrai résumé viendra de Firehouse.",
        j["titre"].as_str().unwrap_or("?")
    ));
    fiche["versions"] = json!([]);
    fiche["annexes"] = json!([]);
    Ok(fiche)
}

impl Source {
    pub fn est_simulee(&self) -> bool {
        matches!(self, Source::Simulee)
    }

    pub async fn plateformes(&self) -> Resultat<Value> {
        match self {
            Source::Firehouse(c) => c.obtenir_json("/plateformes").await,
            Source::Simulee => {
                let mut compte: std::collections::BTreeMap<String, u64> = Default::default();
                for j in catalogue_simule() {
                    *compte.entry(j["plateforme"].as_str().unwrap_or("?").to_string()).or_default() += 1;
                }
                Ok(Value::Array(compte.into_iter().map(|(nom, jeux)| json!({"nom": nom, "jeux": jeux})).collect()))
            }
        }
    }

    pub async fn catalogue(&self, page: u32) -> Resultat<Value> {
        match self {
            Source::Firehouse(c) => c.obtenir_json(&format!("/catalogue?page={page}&par_page={PAR_PAGE}")).await,
            Source::Simulee => {
                let tous = catalogue_simule();
                // Même forme que le vrai Firehouse (docs/EXEMPLES-API-JEUX-V1.md).
                let total = tous.len();
                let debut = (page.saturating_sub(1) as usize) * TAILLE_PAGE_SIMULEE;
                let jeux: Vec<Value> = tous.into_iter().skip(debut).take(TAILLE_PAGE_SIMULEE).collect();
                let suivante = if debut + TAILLE_PAGE_SIMULEE < total { json!(page + 1) } else { Value::Null };
                Ok(json!({"ok": true, "total": total, "page": page, "par_page": TAILLE_PAGE_SIMULEE,
                          "suivante": suivante, "jeux": jeux}))
            }
        }
    }

    pub async fn fiche(&self, id: i64) -> Resultat<Value> {
        match self {
            Source::Firehouse(c) => Ok(deballer_fiche(c.obtenir_json(&format!("/jeu/{id}")).await?)),
            Source::Simulee => fiche_simulee(id),
        }
    }

    /// Une annexe TEXTE (`{ok, titre, texte}`). `cle` protège d'une fiche périmée (409 : relire la fiche).
    pub async fn annexe_texte(&self, id: i64, i: u32, cle: &str) -> Resultat<Value> {
        match self {
            Source::Firehouse(c) => {
                let cle_url: String = cle
                    .bytes()
                    .map(|b| match b {
                        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
                        _ => format!("%{b:02X}"),
                    })
                    .collect();
                c.obtenir_json(&format!("/annexe/{id}/{i}?cle={cle_url}")).await
            }
            Source::Simulee => Ok(json!({
                "ok": true,
                "titre": "Lancement sous DOSBox",
                "texte": "**Exemple du mode simulé.**\n\nLance `DUNE.BAT` depuis le dossier du jeu. Le vrai texte viendra de Firehouse."
            })),
        }
    }

    /// Une image (`jaquette`…). `None` si Firehouse n'en a pas.
    pub async fn media(&self, id: i64, sorte: &str) -> Resultat<Option<Image>> {
        match self {
            Source::Firehouse(c) => match c.obtenir(&format!("/media/{id}/{sorte}"), None).await {
                Ok(Reponse::Corps { octets, type_contenu, .. }) => Ok(Some(Image { octets, type_contenu })),
                Ok(Reponse::NonModifie) => Ok(None),
                Err(Erreur::Introuvable(_)) => Ok(None),
                Err(e) => Err(e),
            },
            // Pas d'images dans le mode simulé : l'interface montre une jaquette de remplacement.
            Source::Simulee => Ok(None),
        }
    }

    /// Le catalogue des skins (`/themes`), avec l'`ETag` du cache. `None` en mode simulé (l'interface prend
    /// alors l'instantané fourni avec Frogtend).
    pub async fn themes(&self, etag: Option<&str>) -> Resultat<Option<Reponse>> {
        match self {
            Source::Firehouse(c) => Ok(Some(c.obtenir("/themes", etag).await?)),
            Source::Simulee => Ok(None),
        }
    }

    /// Le skin choisi par la personne dans Firehouse (`/theme`).
    pub async fn theme(&self) -> Resultat<Option<Value>> {
        match self {
            Source::Firehouse(c) => Ok(Some(c.obtenir_json("/theme").await?)),
            Source::Simulee => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ludotheque::{lire_page, lire_plateformes};

    #[tokio::test]
    async fn le_catalogue_simule_se_lit_comme_le_vrai_page_par_page() {
        let s = Source::Simulee;
        let mut total = 0;
        let mut page = 1;
        loop {
            let p = lire_page(&s.catalogue(page).await.unwrap(), page);
            total += p.jeux.len();
            if p.encore != Some(true) {
                break;
            }
            page += 1;
        }
        assert_eq!(total, catalogue_simule().len());
        assert!(page >= 2, "la fixture doit tenir sur plusieurs pages pour tester la pagination");
    }

    #[tokio::test]
    async fn les_plateformes_simulees_comptent_leurs_jeux() {
        let p = lire_plateformes(&Source::Simulee.plateformes().await.unwrap());
        assert_eq!(p.iter().map(|p| p.jeux).sum::<u64>() as usize, catalogue_simule().len());
    }

    #[test]
    fn la_vraie_fiche_enveloppee_est_deballee() {
        let f = deballer_fiche(crate::ludotheque::tests::exemple_reel("## `GET /jeu/110`"));
        assert_eq!(f["titre"], "Dune");
        assert_eq!(f["versions"][0]["fichiers"][0]["taille"], 237887038);
        assert_eq!(f["annexes"].as_array().unwrap().len(), 5);
        // Une fiche déjà nue reste telle quelle.
        assert_eq!(deballer_fiche(json!({"id": 1, "titre": "X"}))["titre"], "X");
    }

    #[tokio::test]
    async fn la_fiche_de_dune_est_celle_du_contrat_et_un_jeu_absent_est_introuvable() {
        let f = Source::Simulee.fiche(110).await.unwrap();
        assert_eq!(f["versions"][0]["qualite"], "Jeu — prêt à jouer");
        assert_eq!(f["annexes"].as_array().unwrap().len(), 2);
        assert!(matches!(Source::Simulee.fiche(999_999).await, Err(Erreur::Introuvable(_))));
    }
}
