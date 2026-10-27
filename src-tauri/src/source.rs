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
const PATCHS_BLES01227: &str = include_str!("../fixtures/simule/patchs-BLES01227.json");
const TRICHE_SPIDERMAN: &str = include_str!("../fixtures/simule/triche-ultimate-spider-man.json");

/// Le 404 du fichier d'un code : la clé a changé (contrat 1.19, point 6), jamais « ce jeu est inconnu ».
fn code_disparu(e: Erreur) -> Erreur {
    match e {
        Erreur::Introuvable(_) => Erreur::Introuvable("Ce code n'est plus proposé par Firehouse : rouvre la liste des codes.".into()),
        e => e,
    }
}

/// La route du fichier d'un code, EXACTEMENT `/triche/<clé>/fichier`, suivie au plus de `?rom=…` (contrat 1.19) ; la
/// clé n'a que des minuscules, chiffres et tirets (rien de ce que le serveur lirait comme `..`, même encodé).
fn route_de_code_sure(r: &str) -> bool {
    let (chemin, requete) = r.split_once('?').unwrap_or((r, ""));
    let parties: Vec<&str> = chemin.split('/').collect();
    let cle_sure = |c: &str| !c.is_empty() && c.len() <= 64 && c.chars().all(|x| x.is_ascii_lowercase() || x.is_ascii_digit() || x == '-');
    parties.len() == 4
        && parties[0].is_empty()
        && parties[1] == "triche"
        && cle_sure(parties[2])
        && parties[3] == "fichier"
        && (requete.is_empty() || (requete.starts_with("rom=") && !requete.contains(['#', '&'])))
}

/// Les erreurs des routes des patchs RPCS3 (contrat 1.18) dites pour elles : 404 = pas de liste pour ce moteur (jamais
/// « ce jeu ») ; 503 = la liste est en cours de remplacement chez Firehouse.
pub fn erreur_patchs(e: Erreur) -> Erreur {
    match e {
        Erreur::Introuvable(_) => Erreur::Introuvable("Firehouse n'a pas de liste de patchs pour cette version de RPCS3.".into()),
        Erreur::Serveur(m) if m.contains("maintenance") => Erreur::Serveur("Firehouse met à jour la liste des patchs : réessaie dans un moment.".into()),
        e => e,
    }
}

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

    /// Une image d'un SYSTÈME (contrat 1.28) : `GET /plateforme/media/{type}?plateforme=<nom>&largeur=&id=` (WebP,
    /// transparence gardée) ; 404 → `None` (ce jeton ne voit aucun jeu du système, ou pas de visuel de ce type).
    pub async fn image_systeme(&self, nom: &str, sorte: &str, id: Option<i64>, largeur: u32) -> Resultat<Option<Image>> {
        match self {
            Source::Firehouse(c) => {
                let l = largeur.clamp(100, 1000);
                let id = id.map(|i| format!("&id={i}")).unwrap_or_default();
                match c.obtenir(&format!("/plateforme/media/{sorte}?plateforme={}&largeur={l}{id}", encoder(nom)), None).await {
                    Ok(Reponse::Corps { octets, type_contenu, .. }) => Ok(Some(Image { octets, type_contenu })),
                    Ok(Reponse::NonModifie) => Ok(None),
                    Err(Erreur::Introuvable(_)) => Ok(None),
                    Err(e) => Err(e),
                }
            }
            Source::Simulee => Ok(None),
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

    /// Identifie des jeux importés sans fiche (contrat 1.32, `POST /identifier`, 500 au plus par lot) : un statut
    /// `exact|probable|inconnu` par jeu. En mode simulé : tout est `inconnu`.
    pub async fn identifier(&self, corps: &Value) -> Resultat<Value> {
        match self {
            Source::Firehouse(c) => c.envoyer_json("/identifier", corps).await,
            Source::Simulee => Ok(json!({"ok": true, "resultats": corps["jeux"].as_array().cloned().unwrap_or_default().iter().map(|j| json!({
                "cle": j["cle"], "statut": "inconnu", "methode": null, "launchbox_id": null, "fiche_id": null, "candidats": [], "motif": null
            })).collect::<Vec<_>>()})),
        }
    }

    /// La fiche « de base » d'un jeu de la base LaunchBox (contrat 1.32, `GET /base/{launchbox_id}`) ; 404 (absent, ou
    /// caché à ce jeton) → `None`. En mode simulé : aucune.
    pub async fn base(&self, launchbox_id: i64) -> Resultat<Option<Value>> {
        match self {
            Source::Firehouse(c) => match c.obtenir_json::<Value>(&format!("/base/{launchbox_id}")).await {
                Ok(v) => Ok(Some(v)),
                Err(Erreur::Introuvable(_)) => Ok(None),
                Err(e) => Err(e),
            },
            Source::Simulee => Ok(None),
        }
    }

    /// Une image d'une fiche de base (contrat 1.32, `GET /base/{launchbox_id}/media/{type}?largeur=`, mêmes règles que
    /// `/media`) ; 404 → `None`.
    pub async fn media_base(&self, launchbox_id: i64, sorte: &str, largeur: u32) -> Resultat<Option<Image>> {
        match self {
            Source::Firehouse(c) => match c.obtenir(&format!("/base/{launchbox_id}/media/{sorte}?largeur={largeur}"), None).await {
                Ok(Reponse::Corps { octets, type_contenu, .. }) => Ok(Some(Image { octets, type_contenu })),
                Ok(Reponse::NonModifie) => Ok(None),
                Err(Erreur::Introuvable(_)) => Ok(None),
                Err(e) => Err(e),
            },
            Source::Simulee => Ok(None),
        }
    }

    /// Envoie un lot à rapprocher (contrat 1.21, `POST /reconcilier`) ; rend le code HTTP et le corps (le découpage d'un
    /// lot refusé pour sa taille se fait dans `reconciliation`). En mode simulé : aucune fiche n'est rapprochée.
    pub async fn reconcilier_brut(&self, corps: &Value) -> Resultat<(u16, Vec<u8>)> {
        match self {
            Source::Firehouse(c) => {
                let octets = serde_json::to_vec(corps).map_err(|_| Erreur::Disque("Envoi impossible à préparer.".into()))?;
                let b = c.brute(reqwest::Method::POST, "/reconcilier", Some(octets), &[("content-type", "application/json".into())]).await?;
                Ok((b.statut, b.octets))
            }
            Source::Simulee => Ok((200, serde_json::to_vec(&crate::reconciliation::reponse_simulee(corps)).unwrap_or_default())),
        }
    }

    /// Annonce une session de jeu à Firehouse (`debut`, `vivant`, `fin`) : il met en pause ses travaux sur la carte
    /// graphique si ce PC calcule pour lui. En mode simulé : rien.
    pub async fn session(&self, etat: &str, media_id: Option<i64>) -> Resultat<Value> {
        // `media_id` facultatif (contrat) : jamais un numéro négatif (un jeu importé) envoyé à Firehouse.
        let corps = match media_id.filter(|m| *m > 0) {
            Some(m) => json!({ "etat": etat, "media_id": m }),
            None => json!({ "etat": etat }),
        };
        match self {
            Source::Firehouse(c) => c.envoyer_json("/session", &corps).await,
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

    /// Contrat 1.18 : l'état de la liste des patchs RPCS3 gardée par Firehouse (`{version_moteur, sha256, taille,
    /// maj_le, vu_le, fichier}`, relevé réel du 19/10).
    pub async fn patchs_rpcs3_etat(&self) -> Resultat<Value> {
        match self {
            Source::Firehouse(c) => c.obtenir_json("/emulateurs/rpcs3/patchs").await.map_err(erreur_patchs),
            Source::Simulee => Ok(json!({ "ok": true, "version_moteur": "1.2", "sha256": "", "taille": 0, "fichier": "" })),
        }
    }

    /// Contrat 1.18 : les patchs d'un jeu PS3 (`numero` déjà vérifié : 4 lettres + 5 chiffres ; `version` « 01.00 »).
    pub async fn patchs_rpcs3_jeu(&self, numero: &str, version: Option<&str>) -> Resultat<Value> {
        match self {
            Source::Firehouse(c) => {
                let v = version.map(|v| format!("&version_jeu={}", encoder(v))).unwrap_or_default();
                c.obtenir_json(&format!("/emulateurs/rpcs3/patchs?serial={}{v}", encoder(numero))).await.map_err(erreur_patchs)
            }
            // La vraie réponse de Firehouse pour Asura's Wrath (relevé du 19/10), rien pour les autres jeux.
            Source::Simulee => {
                let mut v: Value = serde_json::from_str(PATCHS_BLES01227).unwrap_or(Value::Null);
                if numero != "BLES01227" {
                    v["serial"] = json!(numero);
                    v["patchs"] = json!([]);
                }
                Ok(v)
            }
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
        self.telecharger_plafonne(route, cible, sha256, types, None, progres).await
    }

    /// Comme `telecharger_controle`, en refusant tout fichier plus gros que `plafond` octets (annoncé ou reçu) : le
    /// `.part` est alors retiré aussitôt, jamais un disque rempli par un serveur qui se trompe.
    pub async fn telecharger_plafonne(
        &self,
        route: &str,
        cible: &std::path::Path,
        sha256: Option<&str>,
        types: &[&str],
        plafond: Option<u64>,
        progres: &(dyn Fn(u64, Option<u64>) + Send + Sync),
    ) -> Resultat<u64> {
        use std::io::Write;
        let trop_gros = || Erreur::Serveur("Firehouse a envoyé un fichier bien plus gros que prévu : il n'est pas gardé.".into());
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
        if plafond.is_some_and(|p| total.is_some_and(|t| t > p) || debut > p) {
            let _ = std::fs::remove_file(&part);
            return Err(trop_gros());
        }
        let reprise = flux.reprise && debut > 0;
        let mut sortie = std::fs::OpenOptions::new().create(true).write(true).append(reprise).truncate(!reprise).open(&part)?;
        let mut recus = if reprise { debut } else { 0 };
        while let Some(m) = Client::morceau(&mut flux).await? {
            sortie.write_all(&m)?;
            recus += m.len() as u64;
            if plafond.is_some_and(|p| recus > p) {
                drop(sortie);
                let _ = std::fs::remove_file(&part);
                return Err(trop_gros());
            }
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

    /// Contrat 1.19 (25a) : les codes et mods d'un jeu SANS fiche (jeu importé), d'après ce que Frogtend en sait
    /// (`titre`, `plateforme`, `rom` = nom du fichier, `serial`, `steam_appid` ; au moins un). Même forme que
    /// `/jeu/{id}/triche`, plus `patchs_rpcs3`. Même réponse quel que soit le grade.
    pub async fn triche_recherche(&self, criteres: &[(&str, String)]) -> Resultat<Value> {
        let vide = json!({"codes": [], "cheat_engine": [], "mods": [], "maj_le": null});
        let q: Vec<String> = criteres.iter().filter(|(_, v)| !v.trim().is_empty()).map(|(k, v)| format!("{k}={}", encoder(v))).collect();
        if q.is_empty() {
            return Ok(vide);
        }
        match self {
            Source::Firehouse(c) => c.obtenir_json(&format!("/triche/recherche?{}", q.join("&"))).await,
            // La vraie réponse d'Ultimate Spider-Man (relevé du 19/10) pour ce titre ; rien pour les autres.
            Source::Simulee => {
                let spider = criteres.iter().any(|(k, v)| *k == "titre" && v.to_lowercase().contains("spider-man"));
                Ok(if spider { serde_json::from_str(TRICHE_SPIDERMAN).unwrap_or(vide) } else { vide })
            }
        }
    }

    /// Le fichier d'un code d'un jeu sans fiche, par le `fichier` de la liste repris TEL QUEL (contrat 1.19) ; seule une
    /// route `/triche/<clé>/fichier` de l'API est admise.
    pub async fn fichier_triche_route(&self, fichier: &str) -> Resultat<Vec<u8>> {
        let route = fichier
            .strip_prefix(crate::firehouse::PREFIXE)
            .filter(|r| route_de_code_sure(r))
            .ok_or_else(|| Erreur::Serveur("Firehouse a décrit ce code de façon inattendue.".into()))?;
        match self {
            Source::Firehouse(c) => {
                let b = c.brute(reqwest::Method::GET, route, None, &[]).await?;
                if (200..300).contains(&b.statut) {
                    Ok(b.octets.to_vec())
                } else {
                    Err(code_disparu(crate::firehouse::erreur_du_statut(b.statut, &b.octets)))
                }
            }
            Source::Simulee => Err(Erreur::Refus("Mode simulé : pas de codes.".into())),
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
                    Err(code_disparu(crate::firehouse::erreur_du_statut(b.statut, &b.octets)))
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
    async fn les_patchs_rpcs3_d_un_jeu_et_leurs_erreurs_dites_simplement() {
        use httpmock::prelude::*;
        let m = MockServer::start();
        // La VRAIE réponse de Firehouse pour Asura's Wrath (relevé du 19/10), servie telle quelle.
        let vraie: Value = serde_json::from_str(PATCHS_BLES01227).unwrap();
        let jeu = m.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/emulateurs/rpcs3/patchs").query_param("serial", "BLES01227").query_param("version_jeu", "01.00").header("authorization", "Bearer jeton");
            t.status(200).json_body(vraie.clone());
        });
        let s = Source::Firehouse(crate::firehouse::Client::nouveau(&m.base_url(), "jeton").unwrap());
        let r = s.patchs_rpcs3_jeu("BLES01227", Some("01.00")).await.unwrap();
        assert_eq!(r["patchs"].as_array().unwrap().len(), 8);
        // Les deux formes de `versions_du_jeu` sont bien là (liste et « All »).
        assert!(r["patchs"].as_array().unwrap().iter().any(|p| p["versions_du_jeu"] == "All"));
        assert!(r["patchs"].as_array().unwrap().iter().any(|p| p["versions_du_jeu"].is_array()));
        jeu.assert();
        // 503 (liste en cours de remplacement) et 404 (moteur inconnu) : dits pour CETTE route, jamais « ce jeu ».
        let m2 = MockServer::start();
        m2.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/emulateurs/rpcs3/patchs");
            t.status(503).body("");
        });
        let s2 = Source::Firehouse(crate::firehouse::Client::nouveau(&m2.base_url(), "jeton").unwrap());
        let e = s2.patchs_rpcs3_etat().await.unwrap_err().to_string();
        assert!(e.contains("liste des patchs"), "{e}");
        let m3 = MockServer::start();
        m3.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/emulateurs/rpcs3/patchs");
            t.status(404).json_body(json!({"ok": false}));
        });
        let s3 = Source::Firehouse(crate::firehouse::Client::nouveau(&m3.base_url(), "jeton").unwrap());
        let e = s3.patchs_rpcs3_etat().await.unwrap_err().to_string();
        assert!(e.contains("liste de patchs") && !e.contains("ce jeu"), "{e}");
        // Simulé : la vraie réponse pour Asura's Wrath, rien pour un autre jeu.
        assert_eq!(Source::Simulee.patchs_rpcs3_jeu("BLES01227", None).await.unwrap()["patchs"].as_array().unwrap().len(), 8);
        assert!(Source::Simulee.patchs_rpcs3_jeu("BLUS30721", None).await.unwrap()["patchs"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn les_codes_d_un_jeu_sans_fiche_se_cherchent_et_leur_fichier_se_reprend_tel_quel() {
        use httpmock::prelude::*;
        let m = MockServer::start();
        // La VRAIE réponse de Firehouse pour Sonic (relevé du 19/10).
        let vraie: Value = serde_json::from_str(include_str!("../fixtures/simule/triche-recherche-sonic.json")).unwrap();
        let recherche = m.mock(|w, t| {
            w.method(GET)
                .path("/api/jeux/v1/triche/recherche")
                .query_param("titre", "Sonic the Hedgehog")
                .query_param("plateforme", "Sega Genesis")
                .query_param("rom", "Sonic (W) [!].md")
                // Un critère vide n'est jamais envoyé.
                .matches(|r| !r.query_params.as_ref().is_some_and(|q| q.iter().any(|(k, _)| k == "serial")));
            t.status(200).json_body(vraie.clone());
        });
        let fichier = m.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/triche/retroarch-61bb21b0369e0cc5/fichier");
            t.status(200).body("cheats = 1\n");
        });
        let s = Source::Firehouse(crate::firehouse::Client::nouveau(&m.base_url(), "jeton").unwrap());
        let r = s
            .triche_recherche(&[("titre", "Sonic the Hedgehog".into()), ("plateforme", "Sega Genesis".into()), ("rom", "Sonic (W) [!].md".into()), ("serial", String::new())])
            .await
            .unwrap();
        recherche.assert();
        assert_eq!(r["codes"].as_array().unwrap().len(), 8);
        let f = r["codes"][0]["fichier"].as_str().unwrap();
        assert_eq!(s.fichier_triche_route(f).await.unwrap(), b"cheats = 1\n");
        fichier.assert();
        // Jamais une autre route que celle d'un fichier de code (même « .. » encodé).
        for faux in [
            "/api/jeux/v1/jeu/1/fichier/2/0",
            "https://ailleurs.example/api/jeux/v1/triche/x/fichier",
            "/api/jeux/v1/triche/../moi/fichier",
            "/api/jeux/v1/triche/%2e%2e/fichier",
            "/api/jeux/v1/triche/x",
            "/api/jeux/v1/triche/x/fichier?autre=1",
        ] {
            assert!(s.fichier_triche_route(faux).await.is_err(), "{faux}");
        }
        // La VRAIE réponse de Super Mario World (le `fichier` porte `?rom=`, encodé par Firehouse) : reprise telle quelle.
        let smw: Value = serde_json::from_str(include_str!("../fixtures/simule/triche-recherche-smw.json")).unwrap();
        let avec_rom = m.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/triche/retroarch-7175b4fd6a5aaf30/fichier").query_param("rom", "Super Mario World (USA)");
            t.status(200).body("smw");
        });
        assert_eq!(s.fichier_triche_route(smw["codes"][0]["fichier"].as_str().unwrap()).await.unwrap(), b"smw");
        avec_rom.assert();
        // Un nom No-Intro qui commence par des points n'est pas un « .. » de chemin.
        assert!(super::route_de_code_sure("/triche/retroarch-1/fichier?rom=...Iru%21%20%28Japan%29"));
        // Une clé disparue (404) : « ce code n'est plus proposé », jamais « ce jeu est inconnu ».
        m.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/triche/ancienne/fichier");
            t.status(404).json_body(json!({"ok": false}));
        });
        let e = s.fichier_triche_route("/api/jeux/v1/triche/ancienne/fichier").await.unwrap_err().to_string();
        assert!(e.contains("plus proposé"), "{e}");
        // Rien à chercher : aucun appel. Simulé : la vraie réponse de Spider-Man pour ce titre.
        assert!(s.triche_recherche(&[("titre", "  ".into())]).await.unwrap()["codes"].as_array().unwrap().is_empty());
        let sim = Source::Simulee.triche_recherche(&[("titre", "Ultimate Spider-Man".into()), ("plateforme", "Windows".into())]).await.unwrap();
        assert!(sim["mods"].as_array().unwrap().iter().any(|m| m["nom"] == "Fusion Fix"));
    }

    #[tokio::test]
    async fn la_liste_des_patchs_ne_remplace_rien_si_son_empreinte_est_fausse() {
        use httpmock::prelude::*;
        let m = MockServer::start();
        let contenu = "Version: 1.2\n\nAnchors:\n";
        m.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/emulateurs/rpcs3/patchs/fichier").query_param("v", "1.2");
            t.status(200).body(contenu);
        });
        let d = tempfile::tempdir().unwrap();
        let rpcs3 = d.path();
        std::fs::create_dir_all(rpcs3.join("patches")).unwrap();
        std::fs::write(rpcs3.join("patches").join("patch.yml"), "Version: 1.2\nancienne\n").unwrap();
        let s = Source::Firehouse(crate::firehouse::Client::nouveau(&m.base_url(), "jeton").unwrap());
        let attente = rpcs3.join("patches").join("patch-0000.yml.frogtend");
        let faux = "0".repeat(64);
        assert!(s.telecharger_controle("/emulateurs/rpcs3/patchs/fichier?v=1.2", &attente, Some(&faux), &[], &|_, _| {}).await.is_err());
        assert!(!attente.exists());
        assert_eq!(std::fs::read_to_string(rpcs3.join("patches").join("patch.yml")).unwrap(), "Version: 1.2\nancienne\n");
        // La bonne empreinte : reçue entière, puis posée (version du moteur relue).
        use sha2::Digest;
        let vrai = format!("{:x}", sha2::Sha256::digest(contenu.as_bytes()));
        s.telecharger_controle("/emulateurs/rpcs3/patchs/fichier?v=1.2", &attente, Some(&vrai), &[], &|_, _| {}).await.unwrap();
        crate::patchs_ps3::poser_liste(rpcs3, &attente, "1.2").unwrap();
        assert_eq!(std::fs::read_to_string(rpcs3.join("patches").join("patch.yml")).unwrap(), contenu);
        assert_eq!(crate::patchs_ps3::empreinte_liste(rpcs3).as_deref(), Some(vrai.as_str()));
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

    /// RÉEL, lecture seule chez Firehouse (contrat 1.25, lot D) : la fiche de chaque jeu Steam INSTALLÉ sur ce PC par
    /// `/jeu/steam/{appid}` (réponse copiée dans `FROGTEND_SORTIE` ; le jeton ne s'affiche jamais).
    /// `FROGTEND_PROFIL_ESSAI=<id> FROGTEND_SORTIE=<dossier> cargo test essai_fiche_steam -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_fiche_steam() {
        use crate::coffre::Coffre as _;
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let sortie = std::path::PathBuf::from(std::env::var("FROGTEND_SORTIE").unwrap());
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton");
        let c = crate::firehouse::Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap();
        let installes = crate::boutiques::installes_steam_dossiers(&crate::boutiques::dossier_steam().expect("Steam"));
        let mut toutes = serde_json::Map::new();
        for appid in installes.keys() {
            match c.obtenir(&format!("/jeu/steam/{appid}"), None).await {
                Ok(crate::firehouse::Reponse::Corps { octets, .. }) => {
                    let v: serde_json::Value = serde_json::from_slice(&octets).unwrap_or_default();
                    println!("== {appid} : {v}");
                    toutes.insert(appid.clone(), v);
                }
                Ok(_) => println!("== {appid} : 304"),
                Err(e) => println!("== {appid} : {e:?}"),
            }
        }
        std::fs::write(sortie.join("fiche-steam.json"), serde_json::to_vec_pretty(&toutes).unwrap()).unwrap();
    }

    /// RÉEL, lecture seule (contrat 1.32) : `/identifier` sur trois exemples et `/base/{launchbox_id}`, copiés tels quels
    /// dans `FROGTEND_SORTIE`. Rien d'autre n'est écrit (aucun rattachement).
    /// `FROGTEND_PROFIL_ESSAI=<id> FROGTEND_SORTIE=<dossier> cargo test essai_identifier -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_identifier() {
        use crate::coffre::Coffre as _;
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let sortie = std::path::PathBuf::from(std::env::var("FROGTEND_SORTIE").unwrap());
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton");
        let c = crate::firehouse::Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap();
        let corps = serde_json::json!({"jeux": [
            {"cle": "a", "plateforme": "Nintendo Entertainment System", "fichier": "Abadox - The Deadly Inner War (USA).zip"},
            {"cle": "b", "plateforme": "Windows", "fichier": "Hades.exe", "dossier": "Hades"},
            {"cle": "c", "plateforme": "Windows", "fichier": "Hades.exe", "dossier": "Hades", "steam_appid": 1145360},
            {"cle": "d", "plateforme": "Sony Playstation 3", "fichier": "x.iso", "ps3_title_id": "BCES00569"},
            {"cle": "e", "plateforme": "Inconnue", "fichier": "rien.bin"}
        ]});
        let r: serde_json::Value = c.envoyer_json("/identifier", &corps).await.unwrap();
        std::fs::write(sortie.join("identifier.json"), serde_json::to_vec_pretty(&r).unwrap()).unwrap();
        for x in r["resultats"].as_array().cloned().unwrap_or_default() {
            println!("{} {} {} lb={} fiche={} cand={}", x["cle"], x["statut"], x["methode"], x["launchbox_id"], x["fiche_id"], x["candidats"].as_array().map(|l| l.len()).unwrap_or(0));
        }
        let b: serde_json::Value = c.obtenir_json("/base/1501").await.unwrap();
        std::fs::write(sortie.join("base-1501.json"), serde_json::to_vec_pretty(&b).unwrap()).unwrap();
        println!("base 1501 : {} / {} / images {}", b["titre"], b["plateforme_affichee"], b["images"].as_array().map(|l| l.len()).unwrap_or(0));
    }

    /// RÉEL, lecture seule (contrat 1.31) : les fiches PS3 qui servent leurs mises à jour (`fichiers[].version`, ordre
    /// d'installation), copiées telles quelles dans `FROGTEND_SORTIE`. Rien d'autre n'est écrit.
    /// `FROGTEND_PROFIL_ESSAI=<id> FROGTEND_SORTIE=<dossier> cargo test essai_maj_ps3 -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_maj_ps3() {
        use crate::coffre::Coffre as _;
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let sortie = std::path::PathBuf::from(std::env::var("FROGTEND_SORTIE").unwrap());
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton");
        let c = crate::firehouse::Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap();
        // GT5 (27 maj), God of War: Ascension (9), relevés par Firehouse le 26/10.
        for id in [31183, 31179] {
            let f: serde_json::Value = c.obtenir_json(&format!("/jeu/{id}")).await.unwrap();
            std::fs::write(sortie.join(format!("jeu-{id}.json")), serde_json::to_vec_pretty(&f).unwrap()).unwrap();
            for v in f["jeu"]["versions"].as_array().cloned().unwrap_or_default() {
                for x in v["fichiers"].as_array().cloned().unwrap_or_default() {
                    println!("{id} v{} {:<8} {:<10} {:>12} {}", v["telechargement_id"], x["role"], x["version"], x["taille"], x["nom"]);
                }
            }
        }
        // L'EN-TÊTE de deux maj de GT5 (seulement le premier morceau du flux, puis la connexion est coupée) : leur content
        // ID est-il le même ?
        let f: serde_json::Value = c.obtenir_json("/jeu/31183").await.unwrap();
        let v = &f["jeu"]["versions"][0];
        let fichiers = v["fichiers"].as_array().cloned().unwrap_or_default();
        for (n, x) in fichiers.iter().enumerate().filter(|(_, x)| x["role"] == "maj").take(3) {
            let mut flux = c.flux(&format!("/fichier/31183/{}/{n}", v["telechargement_id"]), 0).await.unwrap();
            let morceau = crate::firehouse::Client::morceau(&mut flux).await.unwrap().unwrap_or_default();
            drop(flux);
            println!("maj {} ({}) : content ID {:?}", x["version"], x["nom"], crate::contenus::content_id(&morceau));
        }
    }

    /// RÉEL, lecture seule : les fiches e-Reader (catalogue de la plateforme, puis Air Hockey-e — 1 carte — et
    /// Pokemon-e TCG — la plus grosse), copiées telles quelles dans `FROGTEND_SORTIE`. Rien d'autre n'est écrit.
    /// `FROGTEND_PROFIL_ESSAI=<id> FROGTEND_SORTIE=<dossier> cargo test essai_ereader -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_ereader() {
        use crate::coffre::Coffre as _;
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let sortie = std::path::PathBuf::from(std::env::var("FROGTEND_SORTIE").unwrap());
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton");
        let c = crate::firehouse::Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap();
        let cat: serde_json::Value = c.obtenir_json(&format!("/catalogue?plateforme={}&page=1", encoder("Nintendo e-Reader"))).await.unwrap();
        std::fs::write(sortie.join("catalogue-ereader.json"), serde_json::to_vec_pretty(&cat).unwrap()).unwrap();
        // Air Hockey-e (1 carte, 2 bandes) et Pokemon-e TCG (la plus grosse), relevées le 25/10.
        for id in [27855, 29704] {
            let f: serde_json::Value = c.obtenir_json(&format!("/jeu/{id}")).await.unwrap();
            println!("{id} : {} versions", f["jeu"]["versions"].as_array().map(|l| l.len()).unwrap_or(0));
            std::fs::write(sortie.join(format!("jeu-{id}.json")), serde_json::to_vec_pretty(&f).unwrap()).unwrap();
        }
        // Contrat 1.30 : le conseil « Nintendo e-Reader » et ses cartouches (3666 Japon, 3681 USA au 26/10).
        let e: serde_json::Value = c.obtenir_json(&format!("/emulateurs?plateforme={}", encoder("Nintendo e-Reader"))).await.unwrap();
        std::fs::write(sortie.join("emulateurs-ereader.json"), serde_json::to_vec_pretty(&e).unwrap()).unwrap();
        for id in [3666, 3681] {
            if let Ok(f) = c.obtenir_json::<serde_json::Value>(&format!("/jeu/{id}")).await {
                std::fs::write(sortie.join(format!("jeu-{id}.json")), serde_json::to_vec_pretty(&f).unwrap()).unwrap();
            }
        }
    }

    /// RÉEL, lecture seule : le temps de réponse BRUT de Firehouse (sans rien traiter) pour `/plateformes` et deux pages
    /// du catalogue, et la taille reçue — pour savoir où passe le temps d'une synchronisation.
    /// `FROGTEND_PROFIL_ESSAI=<id> cargo test essai_temps_firehouse -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_temps_firehouse() {
        use crate::coffre::Coffre as _;
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton");
        let c = crate::firehouse::Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap();
        for route in ["/plateformes", "/catalogue?page=1", "/catalogue?page=2", "/catalogue?page=30", "/catalogue?depuis=2026-10-25T00:00:00&page=1"] {
            let t = std::time::Instant::now();
            let r = c.obtenir(route, None).await;
            let taille = match &r {
                Ok(crate::firehouse::Reponse::Corps { octets, .. }) => octets.len(),
                _ => 0,
            };
            println!("{route} : {:.1} s, {} Ko{}", t.elapsed().as_secs_f64(), taille / 1024, if r.is_err() { " (erreur)" } else { "" });
        }
    }

    /// RÉEL, lecture seule (contrats 1.27/1.28) : `/plateformes` et la fiche d'un jeu, copiées telles quelles dans
    /// `FROGTEND_SORTIE` (de quoi écrire les réponses simulées d'après la vraie vie). Rien d'autre n'est écrit.
    /// `FROGTEND_PROFIL_ESSAI=<id> FROGTEND_SORTIE=<dossier> cargo test essai_plateformes_128 -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_plateformes_128() {
        use crate::coffre::Coffre as _;
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let sortie = std::path::PathBuf::from(std::env::var("FROGTEND_SORTIE").unwrap());
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton");
        let c = crate::firehouse::Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap();
        let p: serde_json::Value = c.obtenir_json("/plateformes").await.unwrap();
        std::fs::write(sortie.join("plateformes.json"), serde_json::to_vec_pretty(&p).unwrap()).unwrap();
        let j: serde_json::Value = c.obtenir_json("/jeu/110").await.unwrap();
        std::fs::write(sortie.join("jeu-110.json"), serde_json::to_vec_pretty(&j).unwrap()).unwrap();
        println!("plateformes : {} entrées", p["plateformes"].as_array().map(|l| l.len()).unwrap_or(0));
    }

    /// RÉEL, lecture seule (contrat 1.26) : les emplacements de parties que Firehouse donne pour des jeux Steam
    /// (`FROGTEND_APPIDS`, séparés par des virgules), puis ce qu'ils deviennent sur CE PC. Rien n'est écrit.
    /// `FROGTEND_PROFIL_ESSAI=<id> FROGTEND_APPIDS=271590 cargo test essai_sauvegardes_steam -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_sauvegardes_steam() {
        use crate::coffre::Coffre as _;
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton");
        let c = crate::firehouse::Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap();
        let steam = crate::boutiques::dossier_steam();
        for appid in std::env::var("FROGTEND_APPIDS").unwrap().split(',') {
            let v: serde_json::Value = c.obtenir_json(&format!("/triche/recherche?steam_appid={}&plateforme=Windows", appid.trim())).await.unwrap();
            println!("== {appid} : sauvegardes = {}", v["sauvegardes"]);
            let jeu = steam.as_ref().and_then(|s| crate::boutiques::installes_steam_dossiers(s).get(appid.trim()).map(|i| i.dossier.clone()));
            let ctx = crate::nuage_steam::Contexte { jeu, steam: steam.clone(), compte: None, windows: crate::nuage_steam::dossiers_windows() };
            let (l, illisibles) = crate::nuage_steam::emplacements(&v["sauvegardes"], &ctx);
            for e in &l {
                println!("   → {} {:?}", e.dossier.display(), e.filtre);
            }
            println!("   illisibles : {illisibles}");
        }
    }

    /// RÉEL, lecture seule chez Firehouse (contrat 1.19) : les codes et mods d'un jeu sans fiche, copiés tels quels dans
    /// `FROGTEND_SORTIE` (de quoi écrire les réponses simulées d'après la vraie vie). Rien d'autre n'est écrit.
    /// `FROGTEND_PROFIL_ESSAI=<id> FROGTEND_SORTIE=<dossier> cargo test essai_mods_1_19 -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_mods_1_19() {
        use crate::coffre::Coffre as _;
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let sortie = std::path::PathBuf::from(std::env::var("FROGTEND_SORTIE").unwrap());
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton");
        let c = crate::firehouse::Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap();
        for (nom, route) in [
            ("spiderman", "/triche/recherche?titre=Ultimate%20Spider-Man&plateforme=Windows"),
            ("skyrim", "/triche/recherche?titre=The%20Elder%20Scrolls%20V%3A%20Skyrim&plateforme=Windows"),
            ("steam-skyrim-se", "/triche/recherche?steam_appid=489830"),
            // Lot B des jeux de boutique (21/10) : deux jeux Steam installés chez Seb, avec titre ET appid.
            ("steam-gta5", "/triche/recherche?titre=Grand%20Theft%20Auto%20V&plateforme=Windows&steam_appid=271590"),
            ("steam-xcom2", "/triche/recherche?titre=XCOM%202&plateforme=Windows&steam_appid=268500"),
            ("asura", "/triche/recherche?serial=BLES01227"),
            ("smw", "/triche/recherche?titre=Super%20Mario%20World&plateforme=Super%20Nintendo%20Entertainment%20System&rom=Super%20Mario%20World%20(USA).sfc"),
            ("sonic", "/triche/recherche?titre=Sonic%20the%20Hedgehog&plateforme=Sega%20Genesis"),
            ("vide", "/triche/recherche"),
        ] {
            match c.obtenir(route, None).await {
                Ok(crate::firehouse::Reponse::Corps { octets, .. }) => {
                    std::fs::write(sortie.join(format!("triche-{nom}.json")), &octets).unwrap();
                    println!("== {nom} : {} octets", octets.len());
                }
                Ok(_) => println!("== {nom} : 304"),
                Err(e) => println!("== {nom} : ERREUR {e:?}"),
            }
        }
    }

    /// RÉEL, lecture seule chez Firehouse (contrat 1.18) : la forme exacte des réponses des patchs RPCS3, pour écrire
    /// les réponses simulées d'après la vraie vie. Rien n'est écrit sur le disque ; le jeton ne s'affiche jamais.
    /// `FROGTEND_PROFIL_ESSAI=<id> cargo test essai_patchs_rpcs3 -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_patchs_rpcs3() {
        use crate::coffre::Coffre as _;
        use sha2::Digest;
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton");
        let c = crate::firehouse::Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap();
        let montrer = |route: &str, r: &Resultat<crate::firehouse::Reponse>| match r {
            Ok(crate::firehouse::Reponse::Corps { octets, etag, type_contenu }) => {
                let texte = String::from_utf8_lossy(&octets[..octets.len().min(1500)]).to_string();
                println!("== {route}
   type {type_contenu:?} etag {etag:?} taille {}
   {texte}", octets.len());
            }
            Ok(crate::firehouse::Reponse::NonModifie) => println!("== {route} : 304"),
            Err(e) => println!("== {route} : ERREUR {e:?}"),
        };
        for route in [
            "/emulateurs/rpcs3/patchs",
            "/emulateurs/rpcs3/patchs?v=1.2",
            "/emulateurs/rpcs3/patchs?v=9.9",
            "/emulateurs/rpcs3/patchs?serial=BLES00932",
            "/emulateurs/rpcs3/patchs?serial=BLES00932&version_jeu=01.02",
            "/emulateurs/rpcs3/patchs?serial=BLES01227",
            "/emulateurs/rpcs3/patchs?serial=ABC",
        ] {
            let r = c.obtenir(route, None).await;
            montrer(route, &r);
        }
        // La réponse COMPLÈTE pour Asura's Wrath, copiée telle quelle (de quoi écrire la réponse simulée d'après la vraie).
        if let Ok(sortie) = std::env::var("FROGTEND_SORTIE") {
            if let Ok(crate::firehouse::Reponse::Corps { octets, .. }) = c.obtenir("/emulateurs/rpcs3/patchs?serial=BLES01227&version_jeu=01.00", None).await {
                std::fs::write(std::path::Path::new(&sortie).join("patchs-BLES01227.json"), &octets).unwrap();
            }
        }
        // Le fichier entier, EN MÉMOIRE seulement : sa taille et son sha256 comparés à l'état.
        let etat: Value = c.obtenir_json("/emulateurs/rpcs3/patchs").await.unwrap();
        if let Ok(crate::firehouse::Reponse::Corps { octets, etag, .. }) = c.obtenir("/emulateurs/rpcs3/patchs/fichier?v=1.2", None).await {
            let sha = format!("{:x}", sha2::Sha256::digest(&octets));
            println!("FICHIER {} octets, sha256 {sha}, etag {etag:?}, état {:?} / {:?}", octets.len(), etat["sha256"], etat["taille"]);
            println!("DÉBUT DU FICHIER :
{}", String::from_utf8_lossy(&octets[..octets.len().min(600)]));
        }
    }
}
