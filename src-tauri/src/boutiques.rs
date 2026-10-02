//! Les jeux que la personne possède dans ses boutiques (lot 9). Décision de Seb (02/10) : comme LaunchBox, chaque compte
//! se règle dans les Options (ou au premier import) ; les jeux importés restent dans le Frogtend de la personne et ne
//! remontent jamais dans Firehouse.
//!
//! **Steam** (API officielle, relevée le 02/10) :
//! - `ISteamUser/ResolveVanityURL/v1/?key=&vanityurl=` → `{response: {success: 1, steamid}}` (URL personnalisée) ;
//! - `IPlayerService/GetOwnedGames/v1/?key=&steamid=&include_appinfo=1&include_played_free_games=1` →
//!   `{response: {game_count, games: [{appid, name, playtime_forever, rtime_last_played, img_icon_url}]}}` ;
//! - jeux installés sur CE PC : `steamapps\libraryfolders.vdf` (bibliothèques) et `appmanifest_<appid>.acf` ;
//! - lancer / installer : `steam://rungameid/<appid>`, `steam://install/<appid>` (Steam fait le travail).
//!
//! La clé d'API est un SECRET : coffre de Windows (`<profil>#steam`), jamais écrite, jamais affichée, jamais
//! journalisée ; les erreurs réseau sont rendues SANS l'adresse (elle contient la clé).

use crate::erreurs::{Erreur, Resultat};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const API: &str = "https://api.steampowered.com";

/// Un jeu possédé dans une boutique.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct JeuBoutique {
    pub id: String,
    pub nom: String,
    /// Minutes jouées (selon la boutique).
    pub minutes: u64,
    /// Dernière partie (secondes depuis 1970), 0 = jamais.
    pub derniere: u64,
    /// Installé sur CE PC (vu au moment de l'import ou de la lecture).
    #[serde(default)]
    pub installe: bool,
}

/// Ce que Frogtend garde d'une boutique pour un profil (`profils\<id>\boutiques.json`), sans aucun secret.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct CompteBoutique {
    /// Ce que la personne a donné : URL personnalisée ou identifiant numérique.
    pub compte: String,
    /// L'identifiant Steam (64 bits) résolu.
    pub steamid: String,
    pub maj_le: String,
    pub jeux: Vec<JeuBoutique>,
}

/// Le nom du secret d'une boutique dans le coffre.
pub fn nom_secret(profil: &str, boutique: &str) -> String {
    format!("{profil}#{boutique}")
}

/// Accepte « NimaiTakahashi », « https://steamcommunity.com/id/NimaiTakahashi/ » ou un identifiant à 17 chiffres
/// (« …/profiles/7656119… »). Rend (personnalisé ?, valeur).
pub fn lire_compte_steam(saisie: &str) -> Option<(bool, String)> {
    let s = saisie.trim().trim_end_matches('/');
    let dernier = s.rsplit('/').next().unwrap_or(s).trim();
    if dernier.is_empty() || dernier.len() > 64 || !dernier.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return None;
    }
    let numerique = dernier.len() == 17 && dernier.chars().all(|c| c.is_ascii_digit());
    Some((!numerique, dernier.to_string()))
}

fn client() -> Resultat<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("Frogtend")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|_| Erreur::Reseau("Connexion impossible.".into()))
}

/// Une requête à l'API Steam. Les erreurs ne contiennent JAMAIS l'adresse (elle porte la clé).
async fn appeler(base: &str, chemin: &str, parametres: &[(&str, &str)]) -> Resultat<Value> {
    let r = client()?
        .get(format!("{base}{chemin}"))
        .query(parametres)
        .send()
        .await
        .map_err(|e| Erreur::Reseau(format!("Steam ne répond pas ({}).", e.without_url())))?;
    match r.status().as_u16() {
        200 => r.json().await.map_err(|_| Erreur::Serveur("Réponse de Steam illisible.".into())),
        401 | 403 => Err(Erreur::Refus("Steam refuse la clé d'API : vérifie-la (ou crée-en une nouvelle).".into())),
        429 => Err(Erreur::Refus("Steam demande d'attendre un peu avant de réessayer.".into())),
        c => Err(Erreur::Serveur(format!("Steam a répondu une erreur (code {c})."))),
    }
}

/// L'identifiant Steam 64 bits d'un compte (URL personnalisée résolue par Steam).
pub async fn steamid(base: &str, cle: &str, saisie: &str) -> Resultat<String> {
    let (perso, valeur) = lire_compte_steam(saisie).ok_or_else(|| Erreur::Refus("Compte Steam illisible : donne l'URL de ton profil ou son nom.".into()))?;
    if !perso {
        return Ok(valeur);
    }
    let v = appeler(base, "/ISteamUser/ResolveVanityURL/v1/", &[("key", cle), ("vanityurl", &valeur)]).await?;
    match (v["response"]["success"].as_i64(), v["response"]["steamid"].as_str()) {
        (Some(1), Some(id)) => Ok(id.to_string()),
        _ => Err(Erreur::Introuvable(format!("Steam ne connaît pas le profil « {valeur} ». Vérifie ton URL personnalisée."))),
    }
}

/// Les jeux possédés (y compris gratuits joués). Un profil Steam privé rend une liste vide : on le dit.
pub async fn jeux_possedes(base: &str, cle: &str, steamid: &str) -> Resultat<Vec<JeuBoutique>> {
    let v = appeler(
        base,
        "/IPlayerService/GetOwnedGames/v1/",
        &[("key", cle), ("steamid", steamid), ("include_appinfo", "1"), ("include_played_free_games", "1"), ("format", "json")],
    )
    .await?;
    let Some(jeux) = v["response"]["games"].as_array() else {
        return Err(Erreur::Refus(
            "Steam ne montre aucun jeu : dans Steam ▸ Profil ▸ Modifier ▸ Confidentialité, mets « Détails du jeu » sur Public.".into(),
        ));
    };
    let mut l: Vec<JeuBoutique> = jeux
        .iter()
        .filter_map(|j| {
            Some(JeuBoutique {
                id: j["appid"].as_u64()?.to_string(),
                nom: j["name"].as_str().unwrap_or("?").to_string(),
                minutes: j["playtime_forever"].as_u64().unwrap_or(0),
                derniere: j["rtime_last_played"].as_u64().unwrap_or(0),
                installe: false,
            })
        })
        .collect();
    l.sort_by(|a, b| a.nom.to_lowercase().cmp(&b.nom.to_lowercase()));
    Ok(l)
}

/// Les bibliothèques Steam de ce PC (`libraryfolders.vdf`, lignes `"path" "D:\\SteamLibrary"`).
pub fn bibliotheques_steam(dossier_steam: &Path) -> Vec<PathBuf> {
    let mut l = vec![dossier_steam.to_path_buf()];
    let vdf = std::fs::read_to_string(dossier_steam.join("steamapps").join("libraryfolders.vdf")).unwrap_or_default();
    for ligne in vdf.lines() {
        let morceaux: Vec<&str> = ligne.split('"').collect();
        if morceaux.len() >= 4 && morceaux[1].eq_ignore_ascii_case("path") {
            let p = PathBuf::from(morceaux[3].replace("\\\\", "\\"));
            if !l.contains(&p) {
                l.push(p);
            }
        }
    }
    l
}

/// Les jeux Steam installés sur ce PC (un `appmanifest_<appid>.acf` dans une bibliothèque).
pub fn installes_steam(dossier_steam: &Path) -> BTreeSet<String> {
    let mut s = BTreeSet::new();
    for b in bibliotheques_steam(dossier_steam) {
        let Ok(entrees) = std::fs::read_dir(b.join("steamapps")) else { continue };
        for e in entrees.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if let Some(id) = n.strip_prefix("appmanifest_").and_then(|r| r.strip_suffix(".acf")) {
                s.insert(id.to_string());
            }
        }
    }
    s
}

/// Le dossier de Steam sur ce PC (registre `HKCU\Software\Valve\Steam`, valeur `SteamPath`, en LECTURE), sinon son
/// dossier habituel.
pub fn dossier_steam() -> Option<PathBuf> {
    let o = std::process::Command::new("reg").args(["query", r"HKCU\Software\Valve\Steam", "/v", "SteamPath"]).output().ok()?;
    let texte = String::from_utf8_lossy(&o.stdout);
    let lu = texte.lines().find(|l| l.contains("SteamPath")).and_then(|l| l.split("REG_SZ").nth(1)).map(|v| PathBuf::from(v.trim().replace('/', "\\")));
    lu.filter(|p| p.is_dir()).or_else(|| {
        let p = PathBuf::from(r"C:\Program Files (x86)\Steam");
        p.is_dir().then_some(p)
    })
}

fn fichier(dossier_profil: &Path) -> PathBuf {
    dossier_profil.join("boutiques.json")
}

/// Le compte Steam enregistré d'un profil (sans secret).
pub fn lire(dossier_profil: &Path) -> Option<CompteBoutique> {
    let v: Value = serde_json::from_slice(&std::fs::read(fichier(dossier_profil)).ok()?).ok()?;
    serde_json::from_value(v["steam"].clone()).ok()
}

pub fn ecrire(dossier_profil: &Path, c: Option<&CompteBoutique>) -> Resultat<()> {
    let f = fichier(dossier_profil);
    let mut v: Value = std::fs::read(&f).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_else(|| serde_json::json!({}));
    match c {
        Some(c) => v["steam"] = serde_json::to_value(c).unwrap_or(Value::Null),
        None => {
            if let Some(o) = v.as_object_mut() {
                o.remove("steam");
            }
        }
    }
    std::fs::create_dir_all(dossier_profil)?;
    std::fs::write(f, serde_json::to_vec_pretty(&v).unwrap_or_default())?;
    Ok(())
}

pub const API_STEAM: &str = API;

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use serde_json::json;

    #[test]
    fn le_compte_steam_se_lit_de_toutes_les_facons() {
        assert_eq!(lire_compte_steam("NimaiTakahashi"), Some((true, "NimaiTakahashi".into())));
        assert_eq!(lire_compte_steam("https://steamcommunity.com/id/NimaiTakahashi/"), Some((true, "NimaiTakahashi".into())));
        assert_eq!(lire_compte_steam("https://steamcommunity.com/profiles/76561197960287930"), Some((false, "76561197960287930".into())));
        assert_eq!(lire_compte_steam(""), None);
        assert_eq!(lire_compte_steam("nom avec espace"), None);
    }

    #[tokio::test]
    async fn steam_resout_le_compte_puis_liste_les_jeux_et_ne_montre_jamais_la_cle() {
        let m = MockServer::start();
        let r = m.mock(|w, t| {
            w.method(GET).path("/ISteamUser/ResolveVanityURL/v1/").query_param("key", "CLE").query_param("vanityurl", "Seb");
            t.status(200).json_body(json!({"response": {"success": 1, "steamid": "76561197960287930"}}));
        });
        let j = m.mock(|w, t| {
            w.method(GET).path("/IPlayerService/GetOwnedGames/v1/").query_param("steamid", "76561197960287930").query_param("include_appinfo", "1");
            t.status(200).json_body(json!({"response": {"game_count": 2, "games": [
                {"appid": 620, "name": "Portal 2", "playtime_forever": 300, "rtime_last_played": 1700000000},
                {"appid": 400, "name": "Portal", "playtime_forever": 0}]}}));
        });
        let id = steamid(&m.base_url(), "CLE", "https://steamcommunity.com/id/Seb").await.unwrap();
        let l = jeux_possedes(&m.base_url(), "CLE", &id).await.unwrap();
        assert_eq!(l.iter().map(|x| x.nom.as_str()).collect::<Vec<_>>(), ["Portal", "Portal 2"]);
        assert_eq!(l[1].minutes, 300);
        r.assert();
        j.assert();

        // Clé refusée : message clair, sans l'adresse (qui contient la clé).
        let m2 = MockServer::start();
        m2.mock(|w, t| {
            w.method(GET).path("/IPlayerService/GetOwnedGames/v1/");
            t.status(403);
        });
        let e = format!("{:?}", jeux_possedes(&m2.base_url(), "SECRETE", "1").await.unwrap_err());
        assert!(e.contains("clé") && !e.contains("SECRETE"));
        // Profil privé : pas de « games ».
        let m3 = MockServer::start();
        m3.mock(|w, t| {
            w.method(GET).path("/IPlayerService/GetOwnedGames/v1/");
            t.status(200).json_body(json!({"response": {}}));
        });
        assert!(format!("{:?}", jeux_possedes(&m3.base_url(), "C", "1").await.unwrap_err()).contains("Confidentialité"));
    }

    #[test]
    fn les_jeux_installes_se_lisent_dans_les_bibliotheques() {
        let d = tempfile::tempdir().unwrap();
        let steam = d.path().join("Steam");
        let autre = d.path().join("SteamLibrary");
        std::fs::create_dir_all(steam.join("steamapps")).unwrap();
        std::fs::create_dir_all(autre.join("steamapps")).unwrap();
        std::fs::write(
            steam.join("steamapps/libraryfolders.vdf"),
            format!("\"libraryfolders\"\n{{\n\t\"0\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n\t\"1\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n}}\n",
                steam.display().to_string().replace('\\', "\\\\"), autre.display().to_string().replace('\\', "\\\\")),
        ).unwrap();
        std::fs::write(steam.join("steamapps/appmanifest_620.acf"), "x").unwrap();
        std::fs::write(autre.join("steamapps/appmanifest_400.acf"), "x").unwrap();
        let s = installes_steam(&steam);
        assert!(s.contains("620") && s.contains("400") && s.len() == 2);
    }

    #[test]
    fn le_compte_s_enregistre_sans_secret_et_s_oublie() {
        let d = tempfile::tempdir().unwrap();
        let c = CompteBoutique { compte: "Seb".into(), steamid: "1".into(), maj_le: "0".into(), jeux: vec![] };
        ecrire(d.path(), Some(&c)).unwrap();
        assert_eq!(lire(d.path()), Some(c));
        assert!(!std::fs::read_to_string(d.path().join("boutiques.json")).unwrap().to_lowercase().contains("key"));
        ecrire(d.path(), None).unwrap();
        assert_eq!(lire(d.path()), None);
    }
}
