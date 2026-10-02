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
    /// La boutique d'origine (« gog », « epic », « steam »…) pour les jeux venus de GOG Galaxy ; vide pour Steam.
    #[serde(default)]
    pub plateforme: String,
    /// La jaquette (adresse publique), quand la source la donne.
    #[serde(default)]
    pub image: Option<String>,
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
                plateforme: String::new(),
                image: None,
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
    lire_source(dossier_profil, "steam")
}

pub fn ecrire(dossier_profil: &Path, c: Option<&CompteBoutique>) -> Resultat<()> {
    ecrire_source(dossier_profil, "steam", c)
}

/// Ce qui est gardé pour une source (« steam », « galaxy ») d'un profil.
pub fn lire_source(dossier_profil: &Path, source: &str) -> Option<CompteBoutique> {
    let v: Value = serde_json::from_slice(&std::fs::read(fichier(dossier_profil)).ok()?).ok()?;
    serde_json::from_value(v[source].clone()).ok()
}

pub fn ecrire_source(dossier_profil: &Path, source: &str, c: Option<&CompteBoutique>) -> Resultat<()> {
    let f = fichier(dossier_profil);
    let mut v: Value = std::fs::read(&f).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_else(|| serde_json::json!({}));
    match c {
        Some(c) => v[source] = serde_json::to_value(c).unwrap_or(Value::Null),
        None => {
            if let Some(o) = v.as_object_mut() {
                o.remove(source);
            }
        }
    }
    std::fs::create_dir_all(dossier_profil)?;
    std::fs::write(f, serde_json::to_vec_pretty(&v).unwrap_or_default())?;
    Ok(())
}

pub const API_STEAM: &str = API;

/// **GOG Galaxy 2.0** (accord de Seb, 02/10 : lecture seule des lanceurs) : Galaxy regroupe les jeux de GOG ET des
/// boutiques que la personne y a reliées (Steam, Epic, Xbox, Ubisoft, EA…). Relevé dans sa base (02/10) :
/// `LibraryReleases(releaseKey « gog_123 », « epic_xxx »…)`, `ReleaseProperties(isDlc, isVisibleInLibrary)`,
/// `GamePieces(gamePieceTypeId → GamePieceTypes.type : originalTitle, title, originalImages{verticalCover…})`,
/// `GameTimes(minutesInGame)`, `LastPlayedDates(lastPlayedDate)`, `InstalledBaseProducts(productId)` (GOG),
/// `InstalledExternalProducts(productId)` (les autres). La base est TOUJOURS lue sur une copie.
pub fn dossier_galaxy() -> Option<PathBuf> {
    let p = PathBuf::from(r"C:\ProgramData\GOG.com\Galaxy\storage");
    p.join("galaxy-2.0.db").is_file().then_some(p)
}

/// Le libellé d'une boutique d'après le préfixe d'une clé Galaxy.
pub fn nom_plateforme(prefixe: &str) -> &'static str {
    match prefixe {
        "gog" => "GOG",
        "steam" => "Steam",
        "epic" => "Epic Games",
        "xboxone" | "xbox" => "Xbox",
        "uplay" => "Ubisoft Connect",
        "origin" | "ea" => "EA",
        "amazon" => "Amazon",
        "battlenet" => "Battle.net",
        _ => "Autre",
    }
}

/// Lit les jeux de GOG Galaxy (copie de la base dans `travail`).
pub fn lire_galaxy(storage: &Path, travail: &Path) -> Resultat<Vec<JeuBoutique>> {
    std::fs::create_dir_all(travail)?;
    for f in ["galaxy-2.0.db", "galaxy-2.0.db-wal", "galaxy-2.0.db-shm"] {
        let src = storage.join(f);
        let dst = travail.join(f);
        let _ = std::fs::remove_file(&dst);
        if src.is_file() {
            std::fs::copy(&src, &dst)?;
        }
    }
    let lire = || -> rusqlite::Result<Vec<JeuBoutique>> {
        let c = rusqlite::Connection::open(travail.join("galaxy-2.0.db"))?;
        let piece = |t: &str| format!("(SELECT g.value FROM GamePieces g JOIN GamePieceTypes y ON y.id = g.gamePieceTypeId WHERE g.releaseKey = l.releaseKey AND y.type = '{t}' LIMIT 1)");
        let sql = format!(
            "SELECT DISTINCT l.releaseKey, {}, {}, {},
                    (SELECT MAX(t.minutesInGame) FROM GameTimes t WHERE t.releaseKey = l.releaseKey),
                    (SELECT MAX(d.lastPlayedDate) FROM LastPlayedDates d WHERE d.gameReleaseKey = l.releaseKey)
             FROM LibraryReleases l LEFT JOIN ReleaseProperties p ON p.releaseKey = l.releaseKey
             WHERE COALESCE(p.isDlc, 0) = 0 AND COALESCE(p.isVisibleInLibrary, 1) = 1",
            piece("originalTitle"),
            piece("title"),
            piece("originalImages")
        );
        let installes_gog: BTreeSet<String> =
            c.prepare("SELECT productId FROM InstalledBaseProducts")?.query_map([], |r| r.get::<_, i64>(0))?.flatten().map(|id| format!("gog_{id}")).collect();
        let installes_autres: BTreeSet<String> =
            c.prepare("SELECT productId FROM InstalledExternalProducts")?.query_map([], |r| r.get::<_, String>(0))?.flatten().collect();
        let mut s = c.prepare(&sql)?;
        let l = s
            .query_map([], |r| {
                let cle: String = r.get(0)?;
                let titre = |v: Option<String>| v.and_then(|t| serde_json::from_str::<Value>(&t).ok()).and_then(|v| v["title"].as_str().map(String::from));
                let nom = titre(r.get(1)?).or(titre(r.get(2)?)).unwrap_or_else(|| cle.clone());
                let image = r.get::<_, Option<String>>(3)?.and_then(|t| serde_json::from_str::<Value>(&t).ok()).and_then(|v| v["verticalCover"].as_str().map(String::from));
                let minutes = r.get::<_, Option<i64>>(4)?.unwrap_or(0).max(0) as u64;
                let derniere = r.get::<_, Option<String>>(5)?.and_then(|d| date_en_secondes(&d)).unwrap_or(0);
                let (prefixe, id) = cle.split_once('_').unwrap_or(("", &cle));
                let installe = installes_gog.contains(&cle) || (prefixe != "gog" && installes_autres.contains(id));
                Ok(JeuBoutique { id: cle.clone(), nom, minutes, derniere, installe, plateforme: prefixe.to_string(), image })
            })?
            .flatten()
            .collect::<Vec<_>>();
        Ok(l)
    };
    let mut l = lire().map_err(|e| Erreur::Disque(format!("La base de GOG Galaxy est illisible ({e}).")))?;
    l.sort_by(|a, b| a.nom.to_lowercase().cmp(&b.nom.to_lowercase()));
    Ok(l)
}

/// « 2022-04-29 20:23:22 » → secondes depuis 1970 (UTC, sans les secondes intercalaires).
fn date_en_secondes(d: &str) -> Option<u64> {
    let (date, heure) = d.split_once(' ').unwrap_or((d, "00:00:00"));
    let n: Vec<i64> = date.split('-').filter_map(|x| x.parse().ok()).collect();
    let h: Vec<i64> = heure.split(':').filter_map(|x| x.parse().ok()).collect();
    if n.len() != 3 {
        return None;
    }
    let (mut a, m, j) = (n[0], n[1], n[2]);
    // Jours depuis 1970 (algorithme civil de H. Hinnant).
    if m <= 2 {
        a -= 1;
    }
    let ere = a.div_euclid(400);
    let ya = a - ere * 400;
    let mp = (m + 9) % 12;
    let jour_an = (153 * mp + 2) / 5 + j - 1;
    let jour_ere = ya * 365 + ya / 4 - ya / 100 + jour_an;
    let jours = ere * 146_097 + jour_ere - 719_468;
    let s = jours * 86_400 + h.first().unwrap_or(&0) * 3600 + h.get(1).unwrap_or(&0) * 60 + h.get(2).unwrap_or(&0);
    (s >= 0).then_some(s as u64)
}

/// Les hôtes d'images de boutiques autorisés (images publiques).
pub fn image_autorisee(url: &str) -> bool {
    url.starts_with("https://images.gog.com/") || url.starts_with("https://cdn.akamai.steamstatic.com/") || url.starts_with("https://cdn1.epicgames.com/")
}

/// Une image publique de boutique (hôte autorisé), gardée en cache sous son empreinte.
pub async fn image_par_adresse(cache: &Path, url: &str) -> Option<Vec<u8>> {
    if !image_autorisee(url) {
        return None;
    }
    use sha2::{Digest, Sha256};
    let f = cache.join("images").join(format!("{:x}", Sha256::digest(url.as_bytes())));
    if let Ok(o) = std::fs::read(&f) {
        return Some(o);
    }
    let r = client().ok()?.get(url).send().await.ok()?;
    if !r.status().is_success() {
        return None;
    }
    let o = r.bytes().await.ok()?.to_vec();
    let _ = std::fs::create_dir_all(f.parent()?);
    let _ = std::fs::write(&f, &o);
    Some(o)
}

/// Les images officielles d'un jeu Steam (CDN public de Steam, sans clé) : la jaquette en hauteur, sinon la bannière.
pub fn adresses_image_steam(appid: &str) -> [String; 2] {
    [
        format!("https://cdn.akamai.steamstatic.com/steam/apps/{appid}/library_600x900.jpg"),
        format!("https://cdn.akamai.steamstatic.com/steam/apps/{appid}/header.jpg"),
    ]
}

/// La jaquette d'un jeu Steam, gardée en cache sur ce PC (`<cache>\steam\<appid>.jpg`) ; `None` si Steam n'en a pas.
pub async fn image_steam(cache: &Path, appid: &str) -> Option<Vec<u8>> {
    if appid.is_empty() || !appid.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let f = cache.join("steam").join(format!("{appid}.jpg"));
    if let Ok(o) = std::fs::read(&f) {
        return Some(o);
    }
    let c = client().ok()?;
    for u in adresses_image_steam(appid) {
        let Ok(r) = c.get(&u).send().await else { continue };
        if r.status().is_success() {
            let o = r.bytes().await.ok()?.to_vec();
            if o.len() > 1000 {
                let _ = std::fs::create_dir_all(f.parent()?);
                let _ = std::fs::write(&f, &o);
                return Some(o);
            }
        }
    }
    None
}

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
    #[ignore]
    fn essai_lire_galaxy() {
        let d = tempfile::tempdir().unwrap();
        let l = lire_galaxy(&dossier_galaxy().expect("pas de Galaxy"), d.path()).unwrap();
        println!("{} jeux", l.len());
        let mut par: std::collections::BTreeMap<String, usize> = Default::default();
        for j in &l {
            *par.entry(nom_plateforme(&j.plateforme).to_string()).or_default() += 1;
        }
        println!("par boutique : {par:?}");
        for j in l.iter().filter(|j| j.installe).take(6) {
            println!("installé : {} [{}] {} min, image={}", j.nom, j.plateforme, j.minutes, j.image.is_some());
        }
        println!("avec jaquette : {}", l.iter().filter(|j| j.image.is_some()).count());
    }

    /// LECTURE SEULE, sur une COPIE (accord de Seb, 02/10) : la structure de la base de GOG Galaxy.
    #[test]
    #[ignore]
    fn essai_structure_gog_galaxy() {
        let d = tempfile::tempdir().unwrap();
        let source = Path::new(r"C:\ProgramData\GOG.com\Galaxy\storage");
        for f in ["galaxy-2.0.db", "galaxy-2.0.db-wal", "galaxy-2.0.db-shm"] {
            let _ = std::fs::copy(source.join(f), d.path().join(f));
        }
        let c = rusqlite::Connection::open(d.path().join("galaxy-2.0.db")).unwrap();
        let tables: Vec<String> = c.prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name").unwrap()
            .query_map([], |r| r.get(0)).unwrap().flatten().collect();
        println!("tables : {}", tables.join(", "));
        for t in ["LibraryReleases", "GamePieces", "GamePieceTypes", "InstalledBaseProducts", "InstalledExternalProducts", "ReleaseProperties"] {
            if let Ok(s) = c.prepare(&format!("SELECT * FROM {t} LIMIT 3")) {
                let cols: Vec<String> = s.column_names().iter().map(|x| x.to_string()).collect();
                let n: i64 = c.query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| r.get(0)).unwrap_or(-1);
                println!("{t} ({n} lignes) : {}", cols.join(", "));
            }
        }
        if let Ok(mut s) = c.prepare("SELECT id, type FROM GamePieceTypes") {
            let l: Vec<String> = s.query_map([], |r| Ok(format!("{}={}", r.get::<_, i64>(0)?, r.get::<_, String>(1)?))).unwrap().flatten().collect();
            println!("types : {}", l.join(" "));
        };
        let mut s = c.prepare("SELECT substr(releaseKey, 1, instr(releaseKey, '_') - 1) AS p, COUNT(*) FROM LibraryReleases GROUP BY p ORDER BY 2 DESC").unwrap();
        let l: Vec<String> = s.query_map([], |r| Ok(format!("{}={}", r.get::<_, String>(0)?, r.get::<_, i64>(1)?))).unwrap().flatten().collect();
        println!("par boutique : {}", l.join(" "));
        drop(s);
        for (t, nom) in [(460, "originalTitle"), (457, "originalImages"), (6, "storeProductState")] {
            let v: Option<(String, String)> = c
                .query_row(&format!("SELECT releaseKey, value FROM GamePieces WHERE gamePieceTypeId = {t} AND releaseKey LIKE 'gog_%' LIMIT 1"), [], |r| Ok((r.get(0)?, r.get(1)?)))
                .ok();
            println!("{nom} : {}", v.map(|(k, v)| format!("{k} → {}", v.chars().take(300).collect::<String>())).unwrap_or_default());
        }
        let n: i64 = c.query_row("SELECT COUNT(*) FROM LibraryReleases l JOIN ReleaseProperties p ON p.releaseKey = l.releaseKey WHERE p.isDlc = 0 AND p.isVisibleInLibrary = 1", [], |r| r.get(0)).unwrap_or(-1);
        println!("jeux visibles (hors DLC) : {n}");
        let ins: Vec<String> = c.prepare("SELECT productId, installationPath FROM InstalledBaseProducts").unwrap().query_map([], |r| Ok(format!("{} → {}", r.get::<_, i64>(0)?, r.get::<_, String>(1)?))).unwrap().flatten().collect();
        println!("installés GOG : {}", ins.join(" | "));
        for t in ["GameTimes", "LastPlayedDates", "InstalledExternalProducts", "Platforms", "PlayTasks", "PlayTaskLaunchParameters"] {
            let mut s = c.prepare(&format!("SELECT * FROM {t} LIMIT 2")).unwrap();
            let cols: Vec<String> = s.column_names().iter().map(|x| x.to_string()).collect();
            let n = cols.len();
            let lignes: Vec<String> = s.query_map([], |r| Ok((0..n).map(|i| r.get::<_, rusqlite::types::Value>(i).map(|v| format!("{v:?}").chars().take(80).collect::<String>()).unwrap_or_default()).collect::<Vec<_>>().join(" ; "))).unwrap().flatten().collect();
            println!("{t} [{}] : {}", cols.join(", "), lignes.join(" || "));
        }
        let v: String = c.query_row("SELECT value FROM GamePieces WHERE gamePieceTypeId = 457 AND releaseKey LIKE 'epic_%' LIMIT 1", [], |r| r.get(0)).unwrap_or_default();
        println!("images epic : {}", v.chars().take(500).collect::<String>());
    }

    #[test]
    fn galaxy_dates_plateformes_et_images_autorisees() {
        assert_eq!(date_en_secondes("1970-01-01 00:00:00"), Some(0));
        assert_eq!(date_en_secondes("2022-04-29 20:23:22"), Some(1_651_263_802));
        assert_eq!(date_en_secondes("n'importe quoi"), None);
        assert_eq!(nom_plateforme("epic"), "Epic Games");
        assert_eq!(nom_plateforme("origin"), "EA");
        assert!(image_autorisee("https://images.gog.com/x_glx_vertical_cover.webp?namespace=gamesdb"));
        assert!(!image_autorisee("https://exemple.com/x.png"));
        assert!(!image_autorisee("http://images.gog.com/x.png"), "https seulement");
    }

    /// Une petite base « façon Galaxy » : les DLC et jeux cachés sont écartés, les installés repérés.
    #[test]
    fn galaxy_se_lit_sur_une_copie_et_ecarte_les_dlc() {
        let d = tempfile::tempdir().unwrap();
        let stock = d.path().join("storage");
        std::fs::create_dir_all(&stock).unwrap();
        let c = rusqlite::Connection::open(stock.join("galaxy-2.0.db")).unwrap();
        c.execute_batch(
            "CREATE TABLE LibraryReleases(id INTEGER, userId INTEGER, releaseKey TEXT);
             CREATE TABLE ReleaseProperties(releaseKey TEXT, isDlc INTEGER, isVisibleInLibrary INTEGER, gameId TEXT);
             CREATE TABLE GamePieceTypes(id INTEGER, type TEXT);
             CREATE TABLE GamePieces(releaseKey TEXT, gamePieceTypeId INTEGER, userId INTEGER, value TEXT, languageId INTEGER);
             CREATE TABLE GameTimes(userId INTEGER, releaseKey TEXT, minutesInGame INTEGER);
             CREATE TABLE LastPlayedDates(userId INTEGER, gameReleaseKey TEXT, lastPlayedDate TEXT);
             CREATE TABLE InstalledBaseProducts(productId INTEGER, installationPath TEXT);
             CREATE TABLE InstalledExternalProducts(id INTEGER, platformId INTEGER, productId TEXT);
             INSERT INTO GamePieceTypes VALUES (460, 'originalTitle'), (457, 'originalImages');
             INSERT INTO LibraryReleases VALUES (1, 1, 'gog_10'), (2, 1, 'epic_Fortnite'), (3, 1, 'gog_11'), (4, 1, 'steam_620');
             INSERT INTO ReleaseProperties VALUES ('gog_10', 0, 1, ''), ('epic_Fortnite', 0, 1, ''), ('gog_11', 1, 1, ''), ('steam_620', 0, 0, '');
             INSERT INTO GamePieces VALUES ('gog_10', 460, 1, '{\"title\":\"Cyberpunk 2077\"}', 0), ('gog_10', 457, 1, '{\"verticalCover\":\"https://images.gog.com/a.webp\"}', 0),
                                           ('epic_Fortnite', 460, 1, '{\"title\":\"Fortnite\"}', 0);
             INSERT INTO GameTimes VALUES (1, 'gog_10', 92);
             INSERT INTO LastPlayedDates VALUES (1, 'gog_10', '2022-04-29 20:23:22');
             INSERT INTO InstalledBaseProducts VALUES (10, 'D:\\Jeux\\Cyberpunk');
             INSERT INTO InstalledExternalProducts VALUES (5, 3, 'Fortnite');",
        )
        .unwrap();
        drop(c);
        let l = lire_galaxy(&stock, &d.path().join("copie")).unwrap();
        assert_eq!(l.iter().map(|j| j.nom.as_str()).collect::<Vec<_>>(), ["Cyberpunk 2077", "Fortnite"], "DLC et caché écartés");
        assert!(l[0].installe && l[0].minutes == 92 && l[0].derniere == 1_651_263_802);
        assert_eq!(l[0].image.as_deref(), Some("https://images.gog.com/a.webp"));
        assert!(l[1].installe && l[1].plateforme == "epic");
        assert!(stock.join("galaxy-2.0.db").is_file(), "l'original n'est pas touché");
    }

    #[tokio::test]
    async fn la_jaquette_steam_vient_du_cache_et_un_identifiant_douteux_est_refuse() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("steam")).unwrap();
        std::fs::write(d.path().join("steam").join("620.jpg"), b"image-en-cache").unwrap();
        assert_eq!(image_steam(d.path(), "620").await.unwrap(), b"image-en-cache");
        assert!(image_steam(d.path(), "../../secret").await.is_none());
        assert!(adresses_image_steam("620")[0].ends_with("/620/library_600x900.jpg"));
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
