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
const DOCUMENTS_110: &str = include_str!("../fixtures/simule/documents-110.json");
const LIVRET_9001: &[u8] = include_bytes!("../fixtures/simule/livret-9001.pdf");

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
                    Err(Erreur::Introuvable(_) | Erreur::Fusionne { .. }) => Ok(None),
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
            Source::Simulee => Ok(json!({ "ok": true, "plateforme": plateforme, "conseil": null, "emulateurs": [] })),
        }
    }

    /// Chercher un jeu dans toute la base LaunchBox de Firehouse (lot 6) : `{ok, resultats: [{launchbox_id, titre,
    /// titre_fr, plateforme, annee, genres, developpeur, jaquette, deja: {id, statut} | null}]}` (relevé le 30/09).
    pub async fn rechercher(&self, texte: &str) -> Resultat<Value> {
        match self {
            Source::Firehouse(c) => c.obtenir_json(&format!("/recherche?texte={}", encoder(texte))).await,
            Source::Simulee => {
                let t = texte.to_lowercase();
                let mut resultats: Vec<Value> = catalogue_simule()
                    .into_iter()
                    .filter(|j| j["titre"].as_str().unwrap_or("").to_lowercase().contains(&t))
                    .map(|j| {
                        json!({"launchbox_id": j["launchbox_id"], "titre": j["titre"], "titre_fr": "", "plateforme": j["plateforme"],
                            "annee": j["annee"], "genres": j["genres"], "developpeur": j["developpeur"], "jaquette": "",
                            "deja": {"id": j["id"], "statut": "possede"}})
                    })
                    .collect();
                if "jeu absent".contains(&t) || t.contains("absent") {
                    resultats.push(json!({"launchbox_id": 999001, "titre": "Jeu absent (exemple)", "titre_fr": "",
                        "plateforme": "Super Nintendo", "annee": 1994, "genres": ["Aventure"], "developpeur": "Exemple",
                        "jaquette": "", "deja": null}));
                }
                Ok(json!({"ok": true, "resultats": resultats}))
            }
        }
    }

    /// Demander un jeu absent (`POST /demandes {launchbox_id}`). Réservé aux grades admin et avancé ; un refus répond
    /// comme un succès (réponse neutre, contrat) : Frogtend dit seulement que la demande est partie.
    pub async fn demander(&self, launchbox_id: i64) -> Resultat<()> {
        match self {
            Source::Firehouse(c) => {
                c.envoyer_json::<Value>("/demandes", &json!({ "launchbox_id": launchbox_id })).await?;
                Ok(())
            }
            Source::Simulee => Ok(()),
        }
    }

    /// Poser une question à l'assistant jeux de Firehouse (lot 7, contrat § 4) : `{ok, texte, actions_proposees:
    /// [{type, titre, details, risque}], outils, confidentialite}`. `historique` : les échanges précédents, gardés
    /// par Frogtend (Firehouse n'en lit que les 20 derniers).
    pub async fn assistant(&self, question: &str, media_id: Option<i64>, historique: &[Value]) -> Resultat<Value> {
        let debut = historique.len().saturating_sub(40);
        match self {
            Source::Firehouse(c) => {
                let mut corps = json!({ "question": question, "historique": &historique[debut..] });
                if let Some(id) = media_id {
                    corps["media_id"] = json!(id);
                }
                c.envoyer_json("/assistant", &corps).await
            }
            Source::Simulee => Ok(json!({
                "ok": true,
                "texte": format!("(Mode simulé) Tu demandes : « {question} ». Le vrai assistant de Firehouse répond en 10 à 40 secondes, sources à l'appui."),
                "actions_proposees": media_id.map(|id| vec![json!({"type": "lancer", "titre": "Lancer le jeu", "details": {"media_id": id}, "risque": "aucun"})]).unwrap_or_default(),
                "outils": [], "confidentialite": []
            })),
        }
    }

    /// Le paquet d'un émulateur, résolu par Firehouse (contrat 14) : `{id, version, nom_fichier, url, taille, sha256,
    /// format, programme, portable, page_release}`. 409 = installation à la main (raison dite).
    pub async fn paquet_emulateur(&self, id: &str) -> Resultat<Value> {
        match self {
            Source::Firehouse(c) => c.obtenir_json(&format!("/emulateurs/{}/paquet", encoder(id))).await,
            Source::Simulee => Err(Erreur::Refus("Mode simulé : aucun émulateur n'est installé.".into())),
        }
    }

    /// Les cœurs RetroArch servis par Firehouse (contrat 1.12) : `{ok, coeurs: [...]}`.
    pub async fn coeurs_retroarch(&self) -> Resultat<Vec<crate::emulateurs::CoeurServi>> {
        match self {
            Source::Firehouse(c) => {
                let v: Value = c.obtenir_json("/emulateurs/retroarch/coeurs").await.map_err(|e| match e {
                    // Un Firehouse d'avant le contrat 1.12 n'a pas cette route.
                    Erreur::Introuvable(_) => Erreur::Introuvable("Firehouse ne fournit pas encore de cœurs RetroArch.".into()),
                    e => e,
                })?;
                serde_json::from_value(v["coeurs"].clone()).map_err(|_| Erreur::Serveur("Firehouse a envoyé une liste de cœurs illisible.".into()))
            }
            Source::Simulee => Err(Erreur::Refus("Mode simulé : aucun cœur n'est installé.".into())),
        }
    }

    /// Télécharge un fichier SERVI PAR FIREHOUSE (route de l'API, avec le jeton) : reprise par `Range` depuis le
    /// `.part`, progression. Rend le nombre d'octets du fichier.
    pub async fn telecharger_route(
        &self,
        route: &str,
        cible: &std::path::Path,
        progres: &(dyn Fn(u64, Option<u64>) + Send + Sync),
    ) -> Resultat<u64> {
        self.telecharger_controle(route, cible, None, &[], progres).await
    }

    /// Comme `telecharger_route`, avec des contrôles AVANT de garder le fichier : le type annoncé par Firehouse doit
    /// être l'un de `types` (s'il y en a), et l'empreinte sha256 du fichier entier doit valoir `sha256` (s'il est
    /// donné) — sinon le `.part` est retiré et l'erreur le dit (aucun nouvel essai : le même fichier reviendrait).
    pub async fn telecharger_controle(
        &self,
        route: &str,
        cible: &std::path::Path,
        sha256: Option<&str>,
        types: &[&str],
        progres: &(dyn Fn(u64, Option<u64>) + Send + Sync),
    ) -> Resultat<u64> {
        use std::io::Write;
        let Source::Firehouse(c) = self else {
            return Err(Erreur::Refus("Mode simulé : rien à télécharger.".into()));
        };
        if let Some(p) = cible.parent() {
            std::fs::create_dir_all(p)?;
        }
        let part = cible.with_extension("part");
        let mut debut = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
        let mut flux = match c.flux(route, debut).await {
            Ok(f) => f,
            // Une reprise refusée (416 : le `.part` vient d'une AUTRE version du fichier, ou il est déjà complet) : le
            // `.part` — un fichier de Frogtend, incomplet par nature — est retiré et on repart de zéro, une seule fois.
            // Sinon le même refus reviendrait à chaque essai. Une coupure réseau, elle, garde le `.part` (reprise).
            // 416 SEULEMENT : tout autre refus (404, 409, 410, 503…) garde le `.part` déjà reçu.
            Err(Erreur::Plage(_)) if debut > 0 => {
                std::fs::remove_file(&part)?;
                debut = 0;
                c.flux(route, 0).await?
            }
            Err(e) => return Err(e),
        };
        if !types.is_empty() {
            let annonce = flux.reponse.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("");
            let annonce = annonce.split(';').next().unwrap_or("").trim().to_ascii_lowercase();
            if !types.contains(&annonce.as_str()) {
                return Err(Erreur::Serveur(format!("Firehouse a envoyé un fichier d'une autre sorte que prévu ({annonce}) : il n'est pas gardé.")));
            }
        }
        let total = flux.reponse.content_length().map(|n| if flux.reprise { n + debut } else { n });
        let reprise = flux.reprise && debut > 0;
        let mut sortie = std::fs::OpenOptions::new().create(true).write(true).append(reprise).truncate(!reprise).open(&part)?;
        let mut recus = if reprise { debut } else { 0 };
        while let Some(m) = Client::morceau(&mut flux).await? {
            sortie.write_all(&m)?;
            recus += m.len() as u64;
            progres(recus, total);
        }
        sortie.flush()?;
        drop(sortie);
        if total.is_some_and(|t| t != recus) {
            return Err(Erreur::Reseau("Le téléchargement s'est arrêté avant la fin : il reprendra où il en était.".into()));
        }
        if let Some(attendue) = sha256 {
            // Hors des fils du réseau : une vidéo de 25 Mo se lit en entier pour son empreinte.
            let p = part.clone();
            let calculee = tokio::task::spawn_blocking(move || crate::sauvegarde::empreinte(&p))
                .await
                .map_err(|_| Erreur::Disque("Le contrôle du fichier reçu s'est arrêté.".into()))??;
            if !calculee.eq_ignore_ascii_case(attendue) {
                std::fs::remove_file(&part)?;
                return Err(Erreur::Serveur("Le fichier reçu est abîmé (son empreinte ne correspond pas) : il n'est pas gardé.".into()));
            }
        }
        std::fs::rename(&part, cible)?;
        Ok(recus)
    }

    /// Tous les visuels d'un jeu (contrat 1.6, galerie) : `{medias_empreinte, medias: [...]}`. En mode simulé : aucun
    /// (comme un jeu dont les images ne sont pas encore rapatriées).
    pub async fn medias(&self, id: i64) -> Resultat<Value> {
        match self {
            Source::Firehouse(c) => c.obtenir_json(&format!("/jeu/{id}/medias")).await,
            Source::Simulee => Ok(json!({"ok": true, "id": id, "medias_empreinte": null, "medias": []})),
        }
    }

    /// UN visuel précis d'un jeu (galerie), par son `id`, en miniature (100 à 1 000 px, jamais l'original) ; 404 →
    /// `None` (relevé réel du 18/10 : un id inconnu, ou sous un autre type, répond 404).
    pub async fn media_precis(&self, id: i64, sorte: &str, media: i64, largeur: u32) -> Resultat<Option<Image>> {
        match self {
            Source::Firehouse(c) => {
                let l = largeur.clamp(100, 1000);
                match c.obtenir(&format!("/media/{id}/{sorte}?id={media}&largeur={l}"), None).await {
                    Ok(Reponse::Corps { octets, type_contenu, .. }) => Ok(Some(Image { octets, type_contenu })),
                    Ok(Reponse::NonModifie) => Ok(None),
                    Err(Erreur::Introuvable(_) | Erreur::Fusionne { .. }) => Ok(None),
                    Err(e) => Err(e),
                }
            }
            Source::Simulee => Ok(None),
        }
    }

    /// Les documents d'un jeu (contrat 1.7) : `{documents_empreinte, documents: [...]}`.
    pub async fn documents(&self, id: i64) -> Resultat<Value> {
        match self {
            Source::Firehouse(c) => c.obtenir_json(&format!("/jeu/{id}/documents")).await,
            Source::Simulee if id == 110 => Ok(serde_json::from_str(DOCUMENTS_110).unwrap()),
            Source::Simulee => Ok(json!({"documents": [], "documents_empreinte": null})),
        }
    }

    /// Télécharge un document d'un jeu (contrat 1.7, `/jeu/{id}/document/{doc}`), type et empreinte vérifiés avant de
    /// le garder. En mode simulé : le petit livret des fixtures (Dune).
    pub async fn telecharger_document(&self, id: i64, doc: i64, cible: &std::path::Path, sha256: Option<&str>, types: &[&str]) -> Resultat<u64> {
        if let Source::Simulee = self {
            if (id, doc) != (110, 9001) {
                return Err(Erreur::Refus("Mode simulé : ce document n'existe pas.".into()));
            }
            if let Some(p) = cible.parent() {
                std::fs::create_dir_all(p)?;
            }
            std::fs::write(cible, LIVRET_9001)?;
            return Ok(LIVRET_9001.len() as u64);
        }
        self.telecharger_controle(&format!("/jeu/{id}/document/{doc}"), cible, sha256, types, &|_, _| {}).await
    }

    /// Les triches et mods connus d'un jeu (contrat 13). Route pas encore servie (404) : listes vides.
    pub async fn triches(&self, media_id: i64) -> Resultat<Value> {
        let vide = json!({"codes": [], "cheat_engine": [], "mods": [], "maj_le": null});
        match self {
            Source::Firehouse(c) => match c.obtenir_json::<Value>(&format!("/jeu/{media_id}/triche")).await {
                Ok(v) => Ok(v),
                Err(Erreur::Introuvable(_) | Erreur::Fusionne { .. }) => Ok(vide),
                Err(e) => Err(e),
            },
            Source::Simulee => Ok(vide),
        }
    }

    /// Le fichier brut d'un code de triche (contrat 13).
    pub async fn fichier_triche(&self, media_id: i64, cle: &str) -> Resultat<Vec<u8>> {
        match self {
            Source::Firehouse(c) => {
                let b = c.brute(reqwest::Method::GET, &format!("/jeu/{media_id}/triche/{}/fichier", encoder(cle)), None, &[]).await?;
                if (200..300).contains(&b.statut) {
                    Ok(b.octets.to_vec())
                } else {
                    Err(crate::firehouse::erreur_du_statut(b.statut, &b.octets))
                }
            }
            Source::Simulee => Err(Erreur::Refus("Mode simulé : pas de codes.".into())),
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

    #[test]
    fn un_chemin_est_entierement_encode_pour_passer_le_proxy_de_firehouse() {
        // Firehouse 2.21 : le proxy filtre la chaîne de requête BRUTE (« ( », « / », espace…).
        assert_eq!(encoder("Dune/concat (1).sav"), "Dune%2Fconcat%20%281%29.sav");
        assert_eq!(encoder("Zoé"), "Zo%C3%A9");
        assert_eq!(encoder("a-b_c.d~e"), "a-b_c.d~e");
    }

    #[tokio::test]
    async fn l_assistant_recoit_la_question_le_jeu_et_l_historique_recent() {
        use httpmock::prelude::*;
        let m = MockServer::start();
        let historique: Vec<Value> = (0..50).map(|i| json!({"role": if i % 2 == 0 { "user" } else { "assistant" }, "content": format!("m{i}")})).collect();
        let attendu: Vec<Value> = historique[10..].to_vec();
        let mock = m.mock(|w, t| {
            w.method(POST).path("/api/jeux/v1/assistant").json_body(json!({"question": "Comment je lance ce jeu ?", "media_id": 110, "historique": attendu}));
            t.status(200).json_body(json!({"ok": true, "texte": "Avec DOSBox.", "actions_proposees": [], "outils": [], "confidentialite": []}));
        });
        let s = Source::Firehouse(crate::firehouse::Client::nouveau(&m.base_url(), "jeton").unwrap());
        let r = s.assistant("Comment je lance ce jeu ?", Some(110), &historique).await.unwrap();
        assert_eq!(r["texte"], "Avec DOSBox.");
        mock.assert();
        // Simulé : une réponse d'exemple, et l'action « lancer » quand on parle d'un jeu.
        let r = Source::Simulee.assistant("Bonjour", Some(110), &[]).await.unwrap();
        assert_eq!(r["actions_proposees"][0]["type"], "lancer");
    }

    #[tokio::test]
    async fn un_paquet_servi_par_firehouse_se_telecharge_avec_le_jeton_et_reprend() {
        use httpmock::prelude::*;
        let m = MockServer::start();
        let d = tempfile::tempdir().unwrap();
        let cible = d.path().join("ce.paquet");
        // Une moitié déjà reçue (coupure) : la suite est demandée par Range.
        std::fs::write(cible.with_extension("part"), b"01234").unwrap();
        let suite = m.mock(|w, r| {
            w.method(GET).path("/api/jeux/v1/emulateurs/cheatengine/fichier").header("range", "bytes=5-").header("authorization", "Bearer jeton");
            r.status(206).body("56789");
        });
        let s = Source::Firehouse(crate::firehouse::Client::nouveau(&m.base_url(), "jeton").unwrap());
        let n = s.telecharger_route("/emulateurs/cheatengine/fichier", &cible, &|_, _| {}).await.unwrap();
        assert_eq!(n, 10);
        assert_eq!(std::fs::read(&cible).unwrap(), b"0123456789");
        assert!(!cible.with_extension("part").exists());
        suite.assert();
    }

    #[tokio::test]
    async fn triches_et_paquets_d_emulateurs_suivent_le_contrat() {
        use httpmock::prelude::*;
        let m = MockServer::start();
        let s = Source::Firehouse(crate::firehouse::Client::nouveau(&m.base_url(), "jeton").unwrap());
        // Route pas encore servie : listes vides, pas d'erreur.
        let v = s.triches(110).await.unwrap();
        assert_eq!(v["codes"].as_array().unwrap().len(), 0);
        let t = m.mock(|w, r| {
            w.method(GET).path("/api/jeux/v1/jeu/111/triche");
            r.status(200).json_body(json!({"codes": [{"cle": "c1", "emulateur": "retroarch", "nom_fichier": "Zelda.cht", "dossier": "cheats/Mupen64Plus-Next", "base": "donnees"}], "cheat_engine": [], "mods": [], "maj_le": "2026-10-02"}));
        });
        let f = m.mock(|w, r| {
            w.method(GET).path("/api/jeux/v1/jeu/111/triche/c1/fichier");
            r.status(200).body("cheats = 1\n");
        });
        assert_eq!(s.triches(111).await.unwrap()["codes"][0]["nom_fichier"], "Zelda.cht");
        assert_eq!(s.fichier_triche(111, "c1").await.unwrap(), b"cheats = 1\n");
        t.assert();
        f.assert();
        let p = m.mock(|w, r| {
            w.method(GET).path("/api/jeux/v1/emulateurs/ryubing/paquet");
            r.status(200).json_body(json!({"id": "ryubing", "version": "1.3.3", "url": "https://exemple/r.zip", "programme": "Ryujinx.exe", "portable": {"type": "dossier", "nom": "portable"}}));
        });
        assert_eq!(s.paquet_emulateur("ryubing").await.unwrap()["programme"], "Ryujinx.exe");
        p.assert();
    }

    #[tokio::test]
    async fn la_recherche_et_la_demande_suivent_le_contrat() {
        // Mode simulé : un jeu du catalogue est « déjà là », l'exemple absent ne l'est pas.
        let s = Source::Simulee;
        let r = s.rechercher("absent").await.unwrap();
        let l = r["resultats"].as_array().unwrap();
        assert!(l.iter().any(|j| j["deja"].is_null() && j["launchbox_id"] == 999001));
        let dune = s.rechercher("dune").await.unwrap();
        assert!(dune["resultats"].as_array().unwrap().iter().all(|j| j["deja"]["statut"] == "possede"));

        // Le vrai : la route, le texte encodé, le corps exact de la demande.
        use httpmock::prelude::*;
        let m = MockServer::start();
        let rech = m.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/recherche").query_param("texte", "zelda ocarina");
            t.status(200).json_body(json!({"ok": true, "resultats": []}));
        });
        let dem = m.mock(|w, t| {
            w.method(POST).path("/api/jeux/v1/demandes").json_body(json!({"launchbox_id": 161}));
            t.status(200).json_body(json!({"ok": true}));
        });
        let c = crate::firehouse::Client::nouveau(&m.base_url(), "jeton").unwrap();
        let s = Source::Firehouse(c);
        s.rechercher("zelda ocarina").await.unwrap();
        s.demander(161).await.unwrap();
        rech.assert();
        dem.assert();
    }

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
