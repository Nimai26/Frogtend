//! D'où viennent les données : Firehouse (le vrai serveur), ou le mode simulé (fixtures, sans réseau).

use crate::erreurs::{Erreur, Resultat};
use crate::firehouse::{Client, Reponse};
use serde_json::{json, Value};

const TAILLE_PAGE_SIMULEE: usize = 25;
/// Jeux demandés par page au vrai Firehouse (500 au plus, contrat 1.3).
const PAR_PAGE: u32 = 500;

/// Encode une valeur pour une adresse (`?cle=`, `?depuis=`).
pub fn encoder(valeur: &str) -> String {
    valeur
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

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

    /// Une page du catalogue ; avec `depuis`, seulement ce qui a changé (et `ids_visibles` en page 1).
    pub async fn catalogue(&self, page: u32, depuis: Option<&str>) -> Resultat<Value> {
        match self {
            Source::Firehouse(c) => {
                let depuis = depuis.map(|d| format!("&depuis={}", encoder(d))).unwrap_or_default();
                c.obtenir_json(&format!("/catalogue?page={page}&par_page={PAR_PAGE}{depuis}")).await
            }
            Source::Simulee => {
                let tous = catalogue_simule();
                // Même forme que le vrai Firehouse (docs/EXEMPLES-API-JEUX-V1.md).
                let total = tous.len();
                let debut = (page.saturating_sub(1) as usize) * TAILLE_PAGE_SIMULEE;
                let jeux: Vec<Value> = tous.into_iter().skip(debut).take(TAILLE_PAGE_SIMULEE).collect();
                let suivante = if debut + TAILLE_PAGE_SIMULEE < total { json!(page + 1) } else { Value::Null };
                let mut r = json!({"ok": true, "total": total, "page": page, "par_page": TAILLE_PAGE_SIMULEE,
                                   "suivante": suivante, "jeux": jeux});
                // Le mode simulé ne suit pas les changements : tout est « changé », et tout reste visible.
                if depuis.is_some() && page == 1 {
                    r["ids_visibles"] = catalogue_simule().iter().map(|j| j["id"].clone()).collect();
                }
                Ok(r)
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
            Source::Firehouse(c) => c.obtenir_json(&format!("/annexe/{id}/{i}?cle={}", encoder(cle))).await,
            Source::Simulee => Ok(json!({
                "ok": true,
                "titre": "Lancement sous DOSBox",
                "texte": "**Exemple du mode simulé.**\n\nLance `DUNE.BAT` depuis le dossier du jeu. Le vrai texte viendra de Firehouse."
            })),
        }
    }

    /// Une image (`jaquette`…), en miniature si `largeur` est donnée. `None` si Firehouse n'en a pas.
    pub async fn media(&self, id: i64, sorte: &str, largeur: Option<u32>) -> Resultat<Option<Image>> {
        match self {
            Source::Firehouse(c) => {
                let l = largeur.map(|l| format!("?largeur={l}")).unwrap_or_default();
                match c.obtenir(&format!("/media/{id}/{sorte}{l}"), None).await {
                    Ok(Reponse::Corps { octets, type_contenu, .. }) => Ok(Some(Image { octets, type_contenu })),
                    Ok(Reponse::NonModifie) => Ok(None),
                    Err(Erreur::Introuvable(_)) => Ok(None),
                    Err(e) => Err(e),
                }
            }
            // Pas d'images dans le mode simulé : l'interface montre une jaquette de remplacement.
            Source::Simulee => Ok(None),
        }
    }

    /// Annonce une session de jeu à Firehouse (`debut`, `vivant`, `fin`) : il met en pause ses travaux sur la carte
    /// graphique si ce PC calcule pour lui. En mode simulé : rien.
    pub async fn session(&self, etat: &str, media_id: i64) -> Resultat<Value> {
        match self {
            Source::Firehouse(c) => c.envoyer_json("/session", &json!({ "etat": etat, "media_id": media_id })).await,
            Source::Simulee => Ok(json!({ "ok": true, "session": null, "machine": "simulé" })),
        }
    }

    /// Les émulateurs recommandés par Firehouse pour un système (le recommandé d'abord).
    pub async fn emulateurs(&self, plateforme: &str) -> Resultat<Value> {
        match self {
            Source::Firehouse(c) => c.obtenir_json(&format!("/emulateurs?plateforme={}", encoder(plateforme))).await,
            Source::Simulee => Ok(json!({ "ok": true, "plateforme": plateforme, "emulateurs": [] })),
        }
    }

    /// Qui porte ce jeton (`/moi`) : `{ok, username, nom, grade, via, api}`.
    pub async fn moi(&self) -> Resultat<Value> {
        match self {
            Source::Firehouse(c) => c.obtenir_json("/moi").await,
            Source::Simulee => Ok(json!({"ok": true, "nom": "Mode simulé", "grade": "simulé", "via": "simulé"})),
        }
    }

    /// Enregistre le skin choisi dans le compte Firehouse de la personne (`PUT /theme`).
    pub async fn enregistrer_theme(&self, nom: &str) -> Resultat<()> {
        match self {
            Source::Firehouse(c) => {
                let _: Value = c.remplacer_json("/theme", &json!({ "theme": nom })).await?;
                Ok(())
            }
            Source::Simulee => Err(Erreur::Refus("En mode simulé, rien n'est enregistré dans Firehouse.".into())),
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
            let p = lire_page(&s.catalogue(page, None).await.unwrap(), page);
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
