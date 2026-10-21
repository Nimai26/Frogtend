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
    /// Le dossier d'installation sur CE PC, quand la boutique le dit (Steam : manifeste ; GOG : base de Galaxy). Lu
    /// seulement : Frogtend n'écrit, ne retire et ne désinstalle JAMAIS rien dans ce dossier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dossier: Option<String>,
    /// Prêt à lancer tout de suite (Steam : aucune mise à jour en attente ni en cours) ; `None` : la boutique ne le dit
    /// pas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pret: Option<bool>,
    /// Succès (obtenus, total) connus de GOG Galaxy (`UserAchievements`) : GOG, Steam, Xbox, EA — pas Epic, que
    /// Galaxy ne synchronise pas (relevé le 02/10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub succes: Option<(u32, u32)>,
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
                dossier: None,
                pret: None,
                succes: None,
            })
        })
        .collect();
    l.sort_by(|a, b| a.nom.to_lowercase().cmp(&b.nom.to_lowercase()));
    Ok(l)
}

/// Une forme comparable d'un chemin Windows (« / » ou « \ », majuscules, séparateur final) : le registre donne
/// `d:/launchbox/games/steam` et le vdf `D:\LaunchBox\Games\Steam` pour le MÊME dossier (relevé sur Venkman, 20/10).
fn cle_chemin(p: &Path) -> String {
    p.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_lowercase()
}

/// Les bibliothèques Steam de ce PC (`libraryfolders.vdf`, lignes `"path" "D:\\SteamLibrary"`), chacune une fois.
pub fn bibliotheques_steam(dossier_steam: &Path) -> Vec<PathBuf> {
    let mut l = vec![dossier_steam.to_path_buf()];
    // Lu en octets : un vdf qui ne serait pas tout à fait de l'UTF-8 ne fait pas perdre toutes les bibliothèques.
    let vdf = std::fs::read(dossier_steam.join("steamapps").join("libraryfolders.vdf")).map(|o| String::from_utf8_lossy(&o).into_owned()).unwrap_or_default();
    for ligne in vdf.lines() {
        let morceaux: Vec<&str> = ligne.split('"').collect();
        if morceaux.len() >= 4 && morceaux[1].eq_ignore_ascii_case("path") {
            let p = PathBuf::from(morceaux[3].replace("\\\\", "\\"));
            if !l.iter().any(|x| cle_chemin(x) == cle_chemin(&p)) {
                l.push(p);
            }
        }
    }
    l
}

/// Ce qu'un `appmanifest_<appid>.acf` dit d'un jeu : son numéro, son dossier (UN nom, sous `steamapps\common`) et son
/// état (`StateFlags`).
#[derive(Debug, Clone, PartialEq)]
pub struct ManifesteSteam {
    pub appid: String,
    pub installdir: String,
    pub etat: u64,
    /// La version installée (`buildid` ; jamais `TargetBuildID`, celle qu'une mise à jour visera) — facultative.
    pub buildid: Option<String>,
}

/// Lit les clés simples d'un manifeste Steam (format texte de Valve : `"clé"  "valeur"`, une par ligne).
pub fn lire_manifeste_steam(texte: &str) -> Option<ManifesteSteam> {
    let valeur = |cle: &str| {
        texte.lines().find_map(|l| {
            let m: Vec<&str> = l.split('"').collect();
            (m.len() >= 4 && m[1].eq_ignore_ascii_case(cle)).then(|| m[3].to_string())
        })
    };
    Some(ManifesteSteam {
        appid: valeur("appid").filter(|a| !a.is_empty() && a.chars().all(|c| c.is_ascii_digit()))?,
        installdir: valeur("installdir")?,
        etat: valeur("StateFlags")?.parse().ok()?,
        buildid: valeur("buildid").filter(|b| !b.is_empty() && b.chars().all(|c| c.is_ascii_digit())),
    })
}

/// Un jeu Steam prêt à jouer : entièrement installé (bit 4 de `StateFlags`) et rien en cours (mise à jour requise ou
/// en cours, fichiers manquants ou abîmés, désinstallation, vérification, téléchargement…). Valeurs de `EAppState`
/// (SteamKit) ; « Encrypted » (8, un pré-chargement pas encore jouable) empêche aussi ; « AppRunning » (64) et
/// « Locked » (16) n'empêchent pas.
pub fn steam_pret(etat: u64) -> bool {
    const EMPECHE: u64 = 2 | 8 | 32 | 128 | 256 | 512 | 1024 | 2048 | 4096 | 65536 | 131072 | 262144 | 524288 | 1048576 | 2097152 | 4194304 | 8388608;
    etat & 4 != 0 && etat & EMPECHE == 0
}

/// Un jeu Steam INSTALLÉ (bit 4) : il est sur le disque, même si une mise à jour l'attend (« mettre à jour au
/// lancement » : 6) ; s'il n'est pas `steam_pret`, Steam le mettra à jour au lancement.
pub fn steam_installe(etat: u64) -> bool {
    etat & 4 != 0
}

/// Un jeu Steam installé sur ce PC : son dossier et s'il est prêt à lancer tout de suite.
#[derive(Debug, Clone, PartialEq)]
pub struct InstalleSteam {
    pub dossier: PathBuf,
    pub pret: bool,
    /// Sa version installée (`buildid`), si l'appmanifest la donne.
    pub buildid: Option<String>,
    /// Steam le dit en cours (« AppRunning », 64) : rien n'y est posé ni retiré.
    pub lance: bool,
}

/// Les jeux Steam installés sur ce PC, avec leur dossier : `<bibliothèque>\steamapps\common\<installdir>`, où
/// `installdir` doit être UN nom sûr (jamais vide, jamais « .. ») et le dossier exister.
pub fn installes_steam_dossiers(dossier_steam: &Path) -> std::collections::BTreeMap<String, InstalleSteam> {
    let mut s = std::collections::BTreeMap::new();
    for b in bibliotheques_steam(dossier_steam) {
        let Ok(entrees) = std::fs::read_dir(b.join("steamapps")) else { continue };
        for e in entrees.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if !(n.starts_with("appmanifest_") && n.ends_with(".acf")) || !e.metadata().is_ok_and(|m| m.len() <= 1024 * 1024) {
                continue;
            }
            let Some(m) = std::fs::read(e.path()).ok().and_then(|o| lire_manifeste_steam(&String::from_utf8_lossy(&o))) else { continue };
            if !steam_installe(m.etat) || crate::jeux_pc::nom_de_fichier_sur(&m.installdir).is_err() {
                continue;
            }
            let dossier = b.join("steamapps").join("common").join(&m.installdir);
            if dossier.is_dir() {
                s.insert(m.appid, InstalleSteam { dossier, pret: steam_pret(m.etat), lance: m.etat & 64 != 0, buildid: m.buildid });
            }
        }
    }
    s
}


/// Le dossier de Steam sur ce PC (registre `HKCU\Software\Valve\Steam`, valeur `SteamPath`, LU par l'API de Windows :
/// un chemin accentué reste juste), sinon son dossier habituel.
pub fn dossier_steam() -> Option<PathBuf> {
    let lu = crate::registre::lire_texte_hkcu(r"Software\Valve\Steam", "SteamPath").map(|v| PathBuf::from(v.trim().replace('/', "\\")));
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
    // Toujours sous les données communes de Windows (pas un dossier écrit en dur : `%ProgramData%`).
    let p = PathBuf::from(std::env::var_os("ProgramData")?).join("GOG.com").join("Galaxy").join("storage");
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
        // Les jeux GOG installés et leur dossier (`installationPath`, relevé le 20/10 : « D:\LaunchBox\Games\GOG
        // Galaxy\Games\Horizon Zero Dawn ») ; les jeux des autres boutiques n'ont AUCUN chemin dans Galaxy.
        let dossiers_gog: std::collections::HashMap<String, Option<String>> = c
            .prepare("SELECT productId, installationPath FROM InstalledBaseProducts")?
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?)))?
            .flatten()
            .map(|(id, p)| (format!("gog_{id}"), p.filter(|p| Path::new(p).is_dir())))
            .collect();
        let installes_autres: BTreeSet<String> =
            c.prepare("SELECT productId FROM InstalledExternalProducts")?.query_map([], |r| r.get::<_, String>(0))?.flatten().collect();
        // Les succès (table présente seulement si Galaxy en a synchronisé).
        let a_des_succes: bool = c
            .query_row("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'UserAchievements'", [], |r| r.get::<_, i64>(0))
            .map(|n| n > 0)?;
        let mut succes: std::collections::HashMap<String, (u32, u32)> = std::collections::HashMap::new();
        if a_des_succes {
            let mut q = c.prepare("SELECT gameReleaseKey, SUM(COALESCE(isUnlocked, 0)), COUNT(*) FROM UserAchievements GROUP BY gameReleaseKey")?;
            for r in q.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?)))?.flatten() {
                succes.insert(r.0, (r.1.max(0) as u32, r.2.max(0) as u32));
            }
        }
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
                let installe = dossiers_gog.contains_key(&cle) || (prefixe != "gog" && installes_autres.contains(id));
                let dossier = dossiers_gog.get(&cle).cloned().flatten();
                let succes = succes.get(&cle).copied().filter(|(_, total)| *total > 0);
                Ok(JeuBoutique { id: cle.clone(), nom, minutes, derniere, installe, plateforme: prefixe.to_string(), image, dossier, pret: None, succes })
            })?
            .flatten()
            .collect::<Vec<_>>();
        Ok(l)
    };
    let mut l = lire().map_err(|e| Erreur::Disque(format!("La base de GOG Galaxy est illisible ({e}).")))?;
    l.sort_by(|a, b| a.nom.to_lowercase().cmp(&b.nom.to_lowercase()));
    Ok(l)
}

/// Les succès en détail d'UN jeu de GOG Galaxy (`cle` : sa clé Galaxy, « gog_… », « steam_620 »…), en LECTURE SEULE
/// sur une copie de sa base (effacée ensuite : elle contient les données du compte). `None` : Galaxy n'en a pas.
/// Relevé réel (21/10) : textes en français notés « non traduits » (`isLocalized` 0, ils sont en anglais) → le
/// français seulement s'il est vraiment traduit, sinon l'anglais (dit) ; aucun succès caché noté (`isVisible`
/// toujours 1) ; rareté en % ; icônes sur images.gog.com ; date « AAAA-MM-JJ hh:mm:ss ».
pub fn succes_galaxy(storage: &Path, travail: &Path, cle: &str) -> Resultat<Option<Vec<crate::succes::SuccesDetail>>> {
    if cle.is_empty() || cle.len() > 128 || !cle.chars().all(|c| c.is_ascii_alphanumeric() || "_-.".contains(c)) {
        return Ok(None);
    }
    // La copie part dans TOUS les cas (même une copie à moitié faite : elle contient les données du compte).
    struct Efface<'a>(&'a Path);
    impl Drop for Efface<'_> {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(self.0);
        }
    }
    let _efface = Efface(travail);
    let fichiers = ["galaxy-2.0.db", "galaxy-2.0.db-wal", "galaxy-2.0.db-shm"];
    // La place : la base et son journal, plus une marge (jamais un disque rempli par une copie de lecture).
    let besoin: u64 = fichiers.iter().filter_map(|f| std::fs::metadata(storage.join(f)).ok()).map(|m| m.len()).sum::<u64>() + (64 << 20);
    std::fs::create_dir_all(travail)?;
    if crate::jeux_pc::place_libre(travail).is_some_and(|l| l < besoin) {
        return Err(Erreur::Disque("Pas assez de place sur le disque des données de Frogtend pour lire les succès de GOG Galaxy.".into()));
    }
    for f in fichiers {
        let (src, dst) = (storage.join(f), travail.join(f));
        if src.is_file() {
            std::fs::copy(&src, &dst)?;
        }
    }
    let lire = || -> rusqlite::Result<Option<Vec<crate::succes::SuccesDetail>>> {
        let c = rusqlite::Connection::open(travail.join("galaxy-2.0.db"))?;
        let tables: i64 = c.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('Achievements', 'UserAchievements', 'LocalizedAchievements', 'Languages')",
            [],
            |r| r.get(0),
        )?;
        if tables < 4 {
            return Ok(None);
        }
        // Le texte : français vraiment traduit, sinon anglais (en-US), sinon n'importe lequel.
        let sql = "SELECT a.apikey, a.imageUnlockedUrl, a.imageLockedUrl, a.isVisible, a.rarity,
                    COALESCE(u.isUnlocked, 0), u.unlockTime,
                    (SELECT l.name || char(31) || COALESCE(l.description, '') || char(31) ||
                            (CASE WHEN g.code LIKE 'fr%' AND l.isLocalized = 1 THEN 'fr' ELSE 'autre' END)
                       FROM LocalizedAchievements l LEFT JOIN Languages g ON g.id = l.languageId
                      WHERE l.gameReleaseKey = a.gameReleaseKey AND l.apikey = a.apikey
                      ORDER BY (CASE WHEN g.code LIKE 'fr%' AND l.isLocalized = 1 THEN 0 WHEN g.code = 'en-US' THEN 1 ELSE 2 END)
                      LIMIT 1)
             FROM Achievements a
             LEFT JOIN UserAchievements u ON u.gameReleaseKey = a.gameReleaseKey AND u.apikey = a.apikey
            WHERE a.gameReleaseKey = ?1";
        let mut q = c.prepare(sql)?;
        let l: Vec<crate::succes::SuccesDetail> = q
            .query_map([cle], |r| {
                let texte: Option<String> = r.get(7)?;
                let mut t = texte.as_deref().unwrap_or("").split('\u{1f}');
                let (nom, description, langue) = (t.next().unwrap_or("").to_string(), t.next().unwrap_or("").to_string(), t.next().unwrap_or("autre"));
                let image = |v: Option<String>| v.filter(|u| image_autorisee(u));
                Ok(crate::succes::SuccesDetail {
                    id: r.get(0)?,
                    nom,
                    description,
                    en_anglais: langue != "fr",
                    obtenu: r.get::<_, Option<i64>>(5)?.unwrap_or(0) == 1,
                    quand: r.get::<_, Option<String>>(6)?.and_then(|d| date_en_secondes(&d)),
                    rarete: r.get::<_, Option<f64>>(4)?.filter(|x| x.is_finite() && *x >= 0.0 && *x <= 100.0),
                    icone: image(r.get(1)?),
                    icone_grise: image(r.get(2)?),
                    secret: r.get::<_, Option<i64>>(3)?.unwrap_or(1) == 0,
                })
            })?
            .flatten()
            .filter(|s| !s.nom.is_empty())
            .collect();
        Ok((!l.is_empty()).then_some(l))
    };
    // La connexion est fermée à la fin de `lire` ; `_efface` retire ensuite la copie.
    lire().map_err(|e| Erreur::Disque(format!("La base de GOG Galaxy est illisible ({e}).")))
}

/// « 2022-04-29 20:23:22 » → secondes depuis 1970 (UTC, sans les secondes intercalaires).
pub(crate) fn date_en_secondes(d: &str) -> Option<u64> {
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
    url.starts_with("https://images.gog.com/")
        || url.starts_with("https://cdn.akamai.steamstatic.com/")
        || url.starts_with("https://cdn1.epicgames.com/")
        // Les badges des succès de RetroAchievements (lot C3).
        || url.starts_with("https://media.retroachievements.org/Badge/")
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
        let manifeste = |appid: &str, dir: &str, etat: u64| format!("\"AppState\"\n{{\n\t\"appid\"\t\t\"{appid}\"\n\t\"StateFlags\"\t\t\"{etat}\"\n\t\"installdir\"\t\t\"{dir}\"\n}}\n");
        std::fs::write(steam.join("steamapps/appmanifest_620.acf"), manifeste("620", "Portal 2", 4)).unwrap();
        std::fs::write(autre.join("steamapps/appmanifest_400.acf"), manifeste("400", "Portal", 4 | 64)).unwrap();
        // En cours de téléchargement (1048576) : pas prêt. Un dossier « .. » : refusé. Dossier absent : rien.
        std::fs::write(autre.join("steamapps/appmanifest_500.acf"), manifeste("500", "Left 4 Dead", 4 | 1_048_576)).unwrap();
        std::fs::write(autre.join("steamapps/appmanifest_501.acf"), manifeste("501", "..", 4)).unwrap();
        std::fs::write(autre.join("steamapps/appmanifest_502.acf"), manifeste("502", "Absent", 4)).unwrap();
        for (b, dir) in [(&steam, "Portal 2"), (&autre, "Portal"), (&autre, "Left 4 Dead")] {
            std::fs::create_dir_all(b.join("steamapps/common").join(dir)).unwrap();
        }
        let s = installes_steam_dossiers(&steam);
        assert_eq!(s.keys().collect::<Vec<_>>(), ["400", "500", "620"], "en mise à jour : installé quand même");
        assert_eq!(s["400"].dossier, autre.join("steamapps/common/Portal"), "jamais « Portal 2 » pour « Portal »");
        assert!(s["400"].pret && s["620"].pret && !s["500"].pret);
        assert!(lire_manifeste_steam("\"appid\" \"12a\"\n\"installdir\" \"x\"\n\"StateFlags\" \"4\"").is_none());
        assert!(s["400"].lance && !s["620"].lance, "AppRunning (64)");
        // `buildid` (relevé réel du 21/10 : présent dans les 11 appmanifest de Venkman), jamais `TargetBuildID`.
        let m = lire_manifeste_steam("\"appid\" \"268500\"\n\"StateFlags\" \"4\"\n\"installdir\" \"XCOM 2\"\n\"TargetBuildID\" \"9\"\n\"buildid\" \"8270065\"").unwrap();
        assert_eq!(m.buildid.as_deref(), Some("8270065"));
        assert_eq!(lire_manifeste_steam("\"appid\" \"1\"\n\"StateFlags\" \"4\"\n\"installdir\" \"x\"\n\"TargetBuildID\" \"9\"").unwrap().buildid, None);
        assert!(steam_pret(4) && steam_pret(4 | 64) && !steam_pret(6) && !steam_pret(1) && !steam_pret(4 | 1024) && !steam_pret(4 | 8));
        assert!(steam_installe(6) && !steam_installe(1) && !steam_installe(1_048_576), "mise à jour au lancement : installé ; 1er téléchargement : non");
    }

    #[test]
    fn une_bibliotheque_steam_ecrite_deux_fois_n_est_lue_qu_une_fois() {
        let d = tempfile::tempdir().unwrap();
        let steam = d.path().join("Steam");
        std::fs::create_dir_all(steam.join("steamapps")).unwrap();
        // Le registre donne « d:/…/steam », le vdf « D:\…\Steam » : le même dossier.
        let autre_forme = steam.to_string_lossy().to_uppercase().replace('\\', "\\\\");
        std::fs::write(steam.join("steamapps/libraryfolders.vdf"), format!("\"0\"\n{{\n\t\"path\"\t\t\"{autre_forme}\\\\\"\n}}\n")).unwrap();
        assert_eq!(bibliotheques_steam(&steam).len(), 1);
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
        for j in l.iter().filter(|j| j.installe) {
            println!("installé : {} [{}] {} min, image={}, dossier={:?}", j.nom, j.plateforme, j.minutes, j.image.is_some(), j.dossier);
        }
        // Steam, en lecture seule aussi : le dossier par le registre (API), les jeux prêts et leur dossier.
        if let Some(s) = dossier_steam() {
            println!("Steam : {} ; bibliothèques {:?}", s.display(), bibliotheques_steam(&s));
            for (id, i) in installes_steam_dossiers(&s) {
                println!("Steam installé : {id} → {} (prêt : {})", i.dossier.display(), i.pret);
            }
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
             INSERT INTO InstalledExternalProducts VALUES (5, 3, 'Fortnite');
             CREATE TABLE UserAchievements(gameReleaseKey TEXT, userId INTEGER, apikey TEXT, unlockTime INTEGER, isUnlocked INTEGER);
             INSERT INTO UserAchievements VALUES ('gog_10', 1, 'a', 1, 1), ('gog_10', 1, 'b', 0, 0), ('gog_10', 1, 'c', 0, 0);",
        )
        .unwrap();
        drop(c);
        let l = lire_galaxy(&stock, &d.path().join("copie")).unwrap();
        assert_eq!(l.iter().map(|j| j.nom.as_str()).collect::<Vec<_>>(), ["Cyberpunk 2077", "Fortnite"], "DLC et caché écartés");
        assert!(l[0].installe && l[0].minutes == 92 && l[0].derniere == 1_651_263_802);
        assert_eq!(l[0].dossier, None, "un dossier noté par Galaxy mais absent du disque n'est pas gardé");
        assert_eq!(l[1].dossier, None, "Galaxy ne note aucun chemin pour les autres boutiques");
        assert_eq!(l[0].image.as_deref(), Some("https://images.gog.com/a.webp"));
        assert!(l[1].installe && l[1].plateforme == "epic");
        assert_eq!(l[0].succes, Some((1, 3)), "succès lus dans Galaxy");
        assert_eq!(l[1].succes, None, "Epic : Galaxy ne les a pas");
        assert!(stock.join("galaxy-2.0.db").is_file(), "l'original n'est pas touché");
    }

    #[test]
    fn les_succes_galaxy_en_detail_francais_s_il_est_vraiment_traduit_sinon_anglais() {
        // La forme réelle des tables (relevé du 21/10 sur la base de Venkman, lue sur une copie).
        let d = tempfile::tempdir().unwrap();
        let stock = d.path().join("storage");
        std::fs::create_dir_all(&stock).unwrap();
        let c = rusqlite::Connection::open(stock.join("galaxy-2.0.db")).unwrap();
        c.execute_batch(
            "CREATE TABLE Achievements(gameReleaseKey TEXT, apikey TEXT, backendId TEXT, imageUnlockedUrl TEXT, imageLockedUrl TEXT, isVisible INTEGER, rarity REAL, raritySlug TEXT);
             CREATE TABLE UserAchievements(gameReleaseKey TEXT, userId INTEGER, apikey TEXT, unlockTime TEXT, isUnlocked INTEGER);
             CREATE TABLE LocalizedAchievements(gameReleaseKey TEXT, apikey TEXT, name TEXT, description TEXT, languageId INTEGER, isLocalized INTEGER);
             CREATE TABLE Languages(id INTEGER, name TEXT, code TEXT);
             INSERT INTO Languages VALUES (16, 'English', 'en-US'), (24, 'French', 'fr-FR'), (12, 'German', 'de-DE');
             INSERT INTO Achievements VALUES ('gog_1', 'a', '1', 'https://images.gog.com/a.png', 'https://images.gog.com/a-gris.png', 1, 3.81, 'rare'),
                                             ('gog_1', 'b', '2', 'https://ailleurs.example/b.png', NULL, 0, 80.5, 'common'),
                                             ('gog_2', 'z', '3', NULL, NULL, 1, NULL, NULL);
             INSERT INTO UserAchievements VALUES ('gog_1', 7, 'a', '2022-04-29 20:23:22', 1), ('gog_1', 7, 'b', NULL, 0);
             INSERT INTO LocalizedAchievements VALUES ('gog_1', 'a', 'Legendary Fossil', 'Caught a Starlight Nautilus', 24, 0),
                                                      ('gog_1', 'a', 'Legendary Fossil', 'Caught a Starlight Nautilus', 16, 0),
                                                      ('gog_1', 'a', 'Legendärer', 'Gefangen', 12, 1),
                                                      ('gog_1', 'b', 'Fossile', 'Un vrai texte français', 24, 1),
                                                      ('gog_1', 'b', 'Fossil', 'English', 16, 0);",
        )
        .unwrap();
        drop(c);
        let travail = d.path().join("copie");
        let l = succes_galaxy(&stock, &travail, "gog_1").unwrap().unwrap();
        let a = l.iter().find(|s| s.id == "a").unwrap();
        assert_eq!((a.nom.as_str(), a.en_anglais, a.obtenu, a.quand, a.rarete), ("Legendary Fossil", true, true, Some(1_651_263_802), Some(3.81)), "« français » non traduit : l'anglais, dit");
        assert_eq!(a.icone_grise.as_deref(), Some("https://images.gog.com/a-gris.png"));
        let b = l.iter().find(|s| s.id == "b").unwrap();
        assert_eq!((b.description.as_str(), b.en_anglais, b.obtenu, b.secret), ("Un vrai texte français", false, false, true));
        assert_eq!(b.icone, None, "un hôte d'image inconnu n'est jamais gardé");
        assert!(!travail.exists(), "la copie de la base ne reste pas");
        assert_eq!(succes_galaxy(&stock, &travail, "gog_2").unwrap(), None, "sans texte : rien");
        assert_eq!(succes_galaxy(&stock, &travail, "x' OR 1=1 --").unwrap(), None, "clé douteuse : rien lu");
        assert!(stock.join("galaxy-2.0.db").is_file());
    }

    /// RÉEL, lecture seule : les succès d'un jeu de la vraie base de Galaxy (sur une copie effacée ensuite).
    /// `FROGTEND_GALAXY_JEU=gog_1084933262 cargo test essai_succes_galaxy -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn essai_succes_galaxy() {
        let cle = std::env::var("FROGTEND_GALAXY_JEU").unwrap();
        let d = tempfile::tempdir().unwrap();
        let l = succes_galaxy(&dossier_galaxy().expect("Galaxy"), &d.path().join("copie"), &cle).unwrap().unwrap_or_default();
        let obtenus = l.iter().filter(|s| s.obtenu).count();
        println!("{cle} : {} succès, {obtenus} obtenus, {} en anglais, {} secrets, {} avec rareté, {} avec icône", l.len(), l.iter().filter(|s| s.en_anglais).count(), l.iter().filter(|s| s.secret).count(), l.iter().filter(|s| s.rarete.is_some()).count(), l.iter().filter(|s| s.icone.is_some()).count());
        if let Some(s) = l.iter().find(|s| s.obtenu) {
            println!("exemple obtenu : « {} » — {} ({:?} %, quand {:?})", s.nom, s.description, s.rarete, s.quand);
        }
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
