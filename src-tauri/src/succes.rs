//! Les succès (lot « succès », Seb 02/10) : RetroAchievements pour l'émulation, Steam pour ses jeux.
//!
//! **RetroAchievements** (documentation officielle : api-docs.retroachievements.org, docs.retroachievements.org) :
//! - API Web `https://retroachievements.org/API/`, clé de la personne (`y`), rangée dans le coffre de Windows ;
//! - un jeu est reconnu par l'EMPREINTE (MD5) de sa ROM, calculée selon la console (ci-dessous, d'après
//!   « Game Identification » et `rcheevos`) ; `API_GetGameList` (`h=1`) donne les empreintes connues d'une
//!   console (gardées en cache une semaine : la documentation demande de le mettre en cache) ;
//! - `API_GetGameHashes` dit quelles versions sont compatibles (leur nom officiel) : si la version de la personne ne
//!   l'est pas, on dit laquelle l'est ;
//! - `API_GetGameInfoAndUserProgress` donne les succès obtenus.
//! Les numéros de console sont ceux de `rcheevos/include/rc_consoles.h`.
//!
//! **Steam** : `ISteamUserStats/GetPlayerAchievements` avec la clé d'API Steam déjà réglée pour le profil.

use crate::erreurs::{Erreur, Resultat};
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

pub const API_RA: &str = "https://retroachievements.org/API";

/// Plateforme LaunchBox → console RetroAchievements (rc_consoles.h).
pub const CONSOLES: &[(&str, u32)] = &[
    ("Sega Genesis", 1),
    ("Sega Mega Drive", 1),
    ("Nintendo 64", 2),
    ("Super Nintendo Entertainment System", 3),
    ("Nintendo Game Boy", 4),
    ("Nintendo Game Boy Advance", 5),
    ("Nintendo Game Boy Color", 6),
    ("Nintendo Entertainment System", 7),
    ("NEC TurboGrafx-16", 8),
    ("PC Engine", 8),
    ("Sega CD", 9),
    ("Sega 32X", 10),
    ("Sega Master System", 11),
    ("Sony Playstation", 12),
    ("Atari Lynx", 13),
    ("SNK Neo Geo Pocket", 14),
    ("SNK Neo Geo Pocket Color", 14),
    ("Sega Game Gear", 15),
    ("Nintendo GameCube", 16),
    ("Atari Jaguar", 17),
    ("Nintendo DS", 18),
    ("Nintendo Wii", 19),
    ("Sony Playstation 2", 21),
    ("Magnavox Odyssey 2", 23),
    ("Nintendo Pokemon Mini", 24),
    ("Atari 2600", 25),
    ("Arcade", 27),
    ("MAME", 27),
    ("Nintendo Virtual Boy", 28),
    ("Microsoft MSX", 29),
    ("Microsoft MSX2", 29),
    ("Sega SG-1000", 33),
    ("Amstrad CPC", 37),
    ("Apple II", 38),
    ("Sega Saturn", 39),
    ("Sega Dreamcast", 40),
    ("Sony PSP", 41),
    ("3DO Interactive Multiplayer", 43),
    ("ColecoVision", 44),
    ("Mattel Intellivision", 45),
    ("GCE Vectrex", 46),
    ("NEC PC-8801", 47),
    ("NEC PC-FX", 49),
    ("Atari 5200", 50),
    ("Atari 7800", 51),
    ("Bandai WonderSwan", 53),
    ("Bandai WonderSwan Color", 53),
    ("SNK Neo Geo CD", 56),
    ("Fairchild Channel F", 57),
    ("Watara Supervision", 63),
    ("NEC TurboGrafx-CD", 76),
    ("Nintendo Famicom Disk System", 81),
    ("Emerson Arcadia 2001", 73),
];

pub fn console_de(plateforme: &str) -> Option<u32> {
    CONSOLES.iter().find(|(p, _)| p.eq_ignore_ascii_case(plateforme)).map(|(_, c)| *c)
}

/// Les consoles dont Frogtend sait calculer l'empreinte (une ROM = un fichier). Les jeux sur CD (PlayStation, Saturn,
/// Dreamcast…) ont des méthodes à part : pas encore vérifiables.
pub fn empreinte_possible(console: u32) -> bool {
    matches!(console, 1..=8 | 10 | 11 | 13..=15 | 17 | 23..=25 | 27 | 28 | 33 | 44 | 45 | 46 | 50 | 51 | 53 | 57 | 63 | 73 | 81)
        || est_un_cd(console)
}

/// Les consoles sur CD que Frogtend sait lire (module `disque` : .cue/.bin, .ccd/.img, .iso ; pas encore .chd).
pub fn est_un_cd(console: u32) -> bool {
    matches!(console, 9 | 12 | 39)
}

fn md5_hex(o: &[u8]) -> String {
    Md5::digest(o).iter().map(|b| format!("{b:02x}")).collect()
}

/// L'empreinte RetroAchievements des octets d'une ROM, selon sa console (règles de « Game Identification »).
pub fn empreinte_octets(console: u32, o: &[u8]) -> String {
    let sans = |n: usize| md5_hex(&o[n.min(o.len())..]);
    match console {
        7 if o.starts_with(b"NES\x1a") => sans(16),
        81 if o.starts_with(b"FDS\x1a") => sans(16),
        13 if o.starts_with(b"LYNX\0") => sans(64),
        51 if o.starts_with(b"\x01ATARI7800") => sans(128),
        3 if o.len() % 8192 == 512 => sans(512),
        8 if o.len() % 131_072 == 512 => sans(512),
        // Nintendo 64 : l'empreinte est celle de l'ordre « big endian » (.z64).
        2 => match o.first() {
            Some(0x37) => md5_hex(&o.chunks(2).flat_map(|c| c.iter().rev().copied()).collect::<Vec<_>>()),
            Some(0x40) => md5_hex(&o.chunks(4).flat_map(|c| c.iter().rev().copied()).collect::<Vec<_>>()),
            _ => md5_hex(o),
        },
        _ => md5_hex(o),
    }
}

/// L'empreinte d'un fichier de jeu. Arcade : le NOM du fichier sans extension (sensible à la casse). Un .zip d'une
/// console à cartouche : la ROM qu'il contient (s'il n'en contient qu'une).
pub fn empreinte_fichier(console: u32, chemin: &Path) -> Resultat<Option<String>> {
    if !empreinte_possible(console) {
        return Ok(None);
    }
    if console == 27 {
        let nom = chemin.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        return Ok(Some(md5_hex(nom.as_bytes())));
    }
    if est_un_cd(console) {
        // Une liste de disques (.m3u) : l'empreinte est celle du 1er disque.
        if chemin.extension().is_some_and(|e| e.eq_ignore_ascii_case("m3u")) {
            let premier = std::fs::read_to_string(chemin)?
                .lines()
                .map(str::trim)
                .find(|l| !l.is_empty() && !l.starts_with('#'))
                .map(|l| chemin.parent().unwrap_or(Path::new(".")).join(l));
            return match premier {
                Some(p) => empreinte_fichier(console, &p),
                None => Ok(None),
            };
        }
        let Some(piste) = crate::disque::ouvrir(chemin)? else { return Ok(None) };
        return match console {
            12 => crate::disque::empreinte_psx(&piste),
            _ => crate::disque::empreinte_sega_cd(&piste),
        };
    }
    let taille = std::fs::metadata(chemin)?.len();
    if taille > 256 * 1024 * 1024 {
        return Ok(None); // pas une ROM de cartouche
    }
    let o = std::fs::read(chemin)?;
    let est_zip = chemin.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip"));
    if est_zip {
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(&o)).map_err(|_| Erreur::Disque("Archive zip illisible.".into()))?;
        let fichiers: Vec<usize> = (0..z.len()).filter(|i| z.by_index(*i).is_ok_and(|f| f.is_file())).collect();
        if fichiers.len() != 1 {
            return Ok(None);
        }
        let mut f = z.by_index(fichiers[0]).map_err(|_| Erreur::Disque("Archive zip illisible.".into()))?;
        let mut contenu = Vec::new();
        std::io::Read::read_to_end(&mut f, &mut contenu)?;
        return Ok(Some(empreinte_octets(console, &contenu)));
    }
    Ok(Some(empreinte_octets(console, &o)))
}

/// Un jeu de la liste d'une console (avec ses empreintes connues).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JeuRa {
    pub id: u64,
    pub titre: String,
    pub succes: u32,
    pub points: u32,
    #[serde(default)]
    pub empreintes: Vec<String>,
}

pub fn lire_liste(v: &Value) -> Vec<JeuRa> {
    v.as_array()
        .map(|l| {
            l.iter()
                .filter_map(|j| {
                    Some(JeuRa {
                        id: j["ID"].as_u64()?,
                        titre: j["Title"].as_str()?.to_string(),
                        succes: j["NumAchievements"].as_u64().unwrap_or(0) as u32,
                        points: j["Points"].as_u64().unwrap_or(0) as u32,
                        empreintes: j["Hashes"].as_array().map(|h| h.iter().filter_map(|x| x.as_str().map(str::to_lowercase)).collect()).unwrap_or_default(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Une version compatible d'un jeu (son nom officiel, ses étiquettes).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VersionCompatible {
    pub md5: String,
    pub nom: String,
    pub etiquettes: Vec<String>,
}

pub fn lire_empreintes_du_jeu(v: &Value) -> Vec<VersionCompatible> {
    v["Results"]
        .as_array()
        .map(|l| {
            l.iter()
                .filter_map(|h| {
                    Some(VersionCompatible {
                        md5: h["MD5"].as_str()?.to_lowercase(),
                        nom: h["Name"].as_str().unwrap_or("").to_string(),
                        etiquettes: h["Labels"].as_array().map(|x| x.iter().filter_map(|s| s.as_str().map(String::from)).collect()).unwrap_or_default(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// La progression de la personne sur un jeu.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct Progression {
    pub id: u64,
    pub titre: String,
    pub succes: u32,
    pub obtenus: u32,
    pub obtenus_hardcore: u32,
    pub points: u32,
    pub points_obtenus: u32,
    pub image: Option<String>,
}

pub fn lire_progression(v: &Value) -> Progression {
    let mut points = 0;
    let mut points_obtenus = 0;
    if let Some(a) = v["Achievements"].as_object() {
        for s in a.values() {
            let p = s["Points"].as_u64().unwrap_or(0) as u32;
            points += p;
            if s["DateEarned"].is_string() || s["DateEarnedHardcore"].is_string() {
                points_obtenus += p;
            }
        }
    }
    Progression {
        id: v["ID"].as_u64().unwrap_or(0),
        titre: v["Title"].as_str().unwrap_or("").to_string(),
        succes: v["NumAchievements"].as_u64().unwrap_or(0) as u32,
        obtenus: v["NumAwardedToUser"].as_u64().unwrap_or(0) as u32,
        obtenus_hardcore: v["NumAwardedToUserHardcore"].as_u64().unwrap_or(0) as u32,
        points,
        points_obtenus,
        image: v["ImageIcon"].as_str().map(|i| format!("https://media.retroachievements.org{i}")),
    }
}

/// Un titre comparable (sans casse, ni ponctuation, ni article final « , The »).
pub fn titre_comparable(t: &str) -> String {
    let t = crate::import_local::titre_depuis_nom(&format!("{t}.x"));
    let t = t.trim_end_matches(", The").trim_end_matches(", the");
    t.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect()
}

async fn appeler(base: &str, route: &str, cle: &str, params: &[(&str, String)]) -> Resultat<Value> {
    let mut q: Vec<(&str, String)> = vec![("y", cle.to_string())];
    q.extend(params.iter().cloned());
    let r = reqwest::Client::builder()
        .user_agent("Frogtend")
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|_| Erreur::Reseau("Connexion impossible.".into()))?
        .get(format!("{base}/{route}"))
        .query(&q)
        .send()
        .await
        // L'adresse contient la clé : on ne la recopie jamais dans un message.
        .map_err(|_| Erreur::Reseau("RetroAchievements ne répond pas.".into()))?;
    match r.status().as_u16() {
        200 => r.json().await.map_err(|_| Erreur::Serveur("Réponse de RetroAchievements illisible.".into())),
        401 | 403 => Err(Erreur::Refus("RetroAchievements refuse ta clé : vérifie ton nom et ta clé d'API Web.".into())),
        429 => Err(Erreur::Serveur("RetroAchievements demande de ralentir : réessaie dans un moment.".into())),
        s => Err(Erreur::Serveur(format!("RetroAchievements a répondu {s}."))),
    }
}

/// Le jeton de connexion des émulateurs, obtenu comme ils le font (rcheevos, `rc_api_user.c` : `dorequest.php`,
/// `r=login2`, `u`, `p`, en POST). Le mot de passe sert à cette seule demande : il n'est jamais gardé.
pub async fn jeton_connexion(hote: &str, utilisateur: &str, mot_de_passe: &str) -> Resultat<(String, String)> {
    let r = reqwest::Client::builder()
        .user_agent("Frogtend")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|_| Erreur::Reseau("Connexion impossible.".into()))?
        .post(format!("{hote}/dorequest.php"))
        .form(&[("r", "login2"), ("u", utilisateur.trim()), ("p", mot_de_passe)])
        .send()
        .await
        .map_err(|_| Erreur::Reseau("RetroAchievements ne répond pas.".into()))?;
    let v: Value = r.json().await.map_err(|_| Erreur::Serveur("Réponse de RetroAchievements illisible.".into()))?;
    lire_connexion(&v)
}

pub fn lire_connexion(v: &Value) -> Resultat<(String, String)> {
    match (v["Success"].as_bool(), v["User"].as_str(), v["Token"].as_str()) {
        (Some(true), Some(u), Some(t)) if !t.is_empty() => Ok((u.to_string(), t.to_string())),
        _ => Err(Erreur::Refus(format!(
            "RetroAchievements refuse la connexion{}.",
            v["Error"].as_str().map(|e| format!(" : {e}")).unwrap_or_default()
        ))),
    }
}

pub const HOTE_RA: &str = "https://retroachievements.org";

/// Vérifie le compte (nom + clé) : rend le nom tel que RetroAchievements l'écrit.
pub async fn verifier_compte(base: &str, cle: &str, utilisateur: &str) -> Resultat<String> {
    let v = appeler(base, "API_GetUserProfile.php", cle, &[("u", utilisateur.trim().to_string())]).await?;
    v["User"].as_str().map(String::from).ok_or_else(|| Erreur::Refus("Compte RetroAchievements introuvable.".into()))
}

pub async fn liste_console(base: &str, cle: &str, console: u32) -> Resultat<Vec<JeuRa>> {
    let v = appeler(base, "API_GetGameList.php", cle, &[("i", console.to_string()), ("f", "1".into()), ("h", "1".into())]).await?;
    Ok(lire_liste(&v))
}

pub async fn empreintes_du_jeu(base: &str, cle: &str, jeu: u64) -> Resultat<Vec<VersionCompatible>> {
    Ok(lire_empreintes_du_jeu(&appeler(base, "API_GetGameHashes.php", cle, &[("i", jeu.to_string())]).await?))
}

pub async fn progression(base: &str, cle: &str, utilisateur: &str, jeu: u64) -> Resultat<Progression> {
    let v = appeler(base, "API_GetGameInfoAndUserProgress.php", cle, &[("u", utilisateur.to_string()), ("g", jeu.to_string())]).await?;
    Ok(lire_progression(&v))
}

/// La liste d'une console, gardée une semaine dans le cache de Frogtend (`cache/retroachievements/<console>.json`).
pub async fn liste_en_cache(cache: &Path, base: &str, cle: &str, console: u32) -> Resultat<Vec<JeuRa>> {
    const SEMAINE: u64 = 7 * 24 * 3600;
    let f = cache.join(format!("{console}.json"));
    let age = std::fs::metadata(&f).ok().and_then(|m| m.modified().ok()).and_then(|t| t.elapsed().ok()).map(|d| d.as_secs());
    if age.is_some_and(|a| a < SEMAINE) {
        if let Some(l) = std::fs::read(&f).ok().and_then(|o| serde_json::from_slice::<Vec<JeuRa>>(&o).ok()) {
            return Ok(l);
        }
    }
    match liste_console(base, cle, console).await {
        Ok(l) => {
            std::fs::create_dir_all(cache)?;
            std::fs::write(&f, serde_json::to_vec(&l).unwrap_or_default())?;
            Ok(l)
        }
        // Hors ligne : l'ancienne liste vaut mieux que rien.
        Err(e) => std::fs::read(&f).ok().and_then(|o| serde_json::from_slice(&o).ok()).ok_or(e),
    }
}

/// Steam : (obtenus, total) d'un jeu, ou `None` s'il n'a pas de succès.
pub async fn succes_steam(base: &str, cle: &str, steamid: &str, appid: &str) -> Resultat<Option<(u32, u32)>> {
    let r = reqwest::Client::builder()
        .user_agent("Frogtend")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|_| Erreur::Reseau("Connexion impossible.".into()))?
        .get(format!("{base}/ISteamUserStats/GetPlayerAchievements/v1/"))
        .query(&[("key", cle), ("steamid", steamid), ("appid", appid), ("l", "french")])
        .send()
        .await
        .map_err(|_| Erreur::Reseau("Steam ne répond pas.".into()))?;
    let v: Value = r.json().await.unwrap_or(Value::Null);
    Ok(lire_succes_steam(&v))
}

pub fn lire_succes_steam(v: &Value) -> Option<(u32, u32)> {
    let l = v["playerstats"]["achievements"].as_array()?;
    if l.is_empty() {
        return None;
    }
    Some((l.iter().filter(|a| a["achieved"].as_u64() == Some(1)).count() as u32, l.len() as u32))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn les_consoles_suivent_rcheevos() {
        assert_eq!(console_de("Nintendo Entertainment System"), Some(7));
        assert_eq!(console_de("super nintendo entertainment system"), Some(3));
        assert_eq!(console_de("Arcade"), Some(27));
        assert_eq!(console_de("Windows"), None);
        assert!(empreinte_possible(7) && empreinte_possible(12) && empreinte_possible(9), "PlayStation et Sega CD : lus par le module disque");
        assert!(!empreinte_possible(40), "Dreamcast (GD-ROM, .chd) : pas encore");
    }

    #[test]
    fn l_empreinte_suit_les_regles_de_chaque_console() {
        let rom = b"ROM!";
        // NES : l'en-tête iNES (16 octets) est ignoré.
        let mut nes = b"NES\x1a".to_vec();
        nes.extend([0u8; 12]);
        nes.extend(rom);
        assert_eq!(empreinte_octets(7, &nes), md5_hex(rom));
        // Sans en-tête : tout le fichier.
        assert_eq!(empreinte_octets(7, rom), md5_hex(rom));
        // SNES : 512 octets d'en-tête quand la taille dépasse un multiple de 8 Kio de 512.
        let mut snes = vec![0xAAu8; 512];
        snes.extend(vec![1u8; 8192]);
        assert_eq!(empreinte_octets(3, &snes), md5_hex(&vec![1u8; 8192]));
        assert_eq!(empreinte_octets(3, &vec![1u8; 8192]), md5_hex(&vec![1u8; 8192]));
        // Nintendo 64 : .v64 (octets échangés deux à deux) et .n64 (inversés quatre à quatre) → empreinte du .z64.
        let z64 = [0x80u8, 0x37, 0x12, 0x40, 1, 2, 3, 4];
        let v64 = [0x37u8, 0x80, 0x40, 0x12, 2, 1, 4, 3];
        let n64 = [0x40u8, 0x12, 0x37, 0x80, 4, 3, 2, 1];
        assert_eq!(empreinte_octets(2, &v64), md5_hex(&z64));
        assert_eq!(empreinte_octets(2, &n64), md5_hex(&z64));
        // Lynx et 7800 : leurs en-têtes.
        let mut lynx = b"LYNX\0".to_vec();
        lynx.extend([0u8; 59]);
        lynx.extend(rom);
        assert_eq!(empreinte_octets(13, &lynx), md5_hex(rom));
        // Le MD5 lui-même : la valeur connue de « abc ».
        assert_eq!(md5_hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
    }

    #[test]
    fn arcade_et_zip() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("pacman.zip");
        let mut z = zip::ZipWriter::new(std::fs::File::create(&p).unwrap());
        z.start_file("Mario.nes", zip::write::SimpleFileOptions::default()).unwrap();
        std::io::Write::write_all(&mut z, b"ROM!").unwrap();
        z.finish().unwrap();
        // Arcade : le nom du fichier (sans extension), pas son contenu.
        assert_eq!(empreinte_fichier(27, &p).unwrap(), Some(md5_hex(b"pacman")));
        // Une console à cartouche : la ROM DANS le zip.
        assert_eq!(empreinte_fichier(7, &p).unwrap(), Some(md5_hex(b"ROM!")));
        assert_eq!(empreinte_fichier(12, &p).unwrap(), None);
    }

    #[tokio::test]
    async fn le_compte_et_la_liste_d_une_console() {
        use httpmock::prelude::*;
        let s = MockServer::start();
        let profil = s.mock(|w, t| {
            w.method(GET).path("/API_GetUserProfile.php").query_param("u", "Seb").query_param("y", "CLESECRETE0123456");
            t.status(200).json_body(json!({"User": "Seb"}));
        });
        s.mock(|w, t| {
            w.method(GET).path("/API_GetUserProfile.php").query_param("y", "MAUVAISE012345678");
            t.status(401);
        });
        let liste = s.mock(|w, t| {
            w.method(GET).path("/API_GetGameList.php").query_param("i", "7").query_param("h", "1").query_param("f", "1");
            t.status(200).json_body(json!([{"Title": "Super Mario Bros.", "ID": 1446, "NumAchievements": 30, "Points": 300, "Hashes": ["ABC"]}]));
        });
        assert_eq!(verifier_compte(&s.base_url(), "CLESECRETE0123456", " Seb ").await.unwrap(), "Seb");
        profil.assert();
        let e = verifier_compte(&s.base_url(), "MAUVAISE012345678", "Seb").await.unwrap_err();
        assert!(!format!("{e:?}").contains("MAUVAISE"), "la clé n'apparaît jamais dans un message");

        // La liste d'une console : demandée une fois, puis lue dans le cache (une semaine).
        let d = tempfile::tempdir().unwrap();
        let l = liste_en_cache(d.path(), &s.base_url(), "CLESECRETE0123456", 7).await.unwrap();
        assert_eq!(l[0].empreintes, ["abc"]);
        let l2 = liste_en_cache(d.path(), &s.base_url(), "CLESECRETE0123456", 7).await.unwrap();
        assert_eq!(l, l2);
        liste.assert_hits(1);
        assert!(!std::fs::read_to_string(d.path().join("7.json")).unwrap().contains("CLESECRETE"), "pas de clé dans le cache");
    }

    #[tokio::test]
    async fn le_jeton_des_emulateurs_s_obtient_sans_garder_le_mot_de_passe() {
        use httpmock::prelude::*;
        let s = MockServer::start();
        s.mock(|w, t| {
            w.method(POST).path("/dorequest.php").body_contains("r=login2").body_contains("u=Seb").body_contains("p=secret");
            t.status(200).json_body(json!({"Success": true, "User": "Seb", "Token": "JETON", "Score": 10}));
        });
        s.mock(|w, t| {
            w.method(POST).path("/dorequest.php").body_contains("p=faux");
            t.status(200).json_body(json!({"Success": false, "Error": "Invalid User/Password combination. Please try again"}));
        });
        assert_eq!(jeton_connexion(&s.base_url(), "Seb", "secret").await.unwrap(), ("Seb".into(), "JETON".into()));
        let e = jeton_connexion(&s.base_url(), "Seb", "faux").await.unwrap_err();
        assert!(format!("{e:?}").contains("Invalid User/Password") && !format!("{e:?}").contains("faux"));
    }

    #[test]
    fn les_reponses_de_retroachievements_se_lisent() {
        let l = lire_liste(&json!([{"Title": "Super Mario Bros.", "ID": 1446, "NumAchievements": 30, "Points": 300, "Hashes": ["ABC", "def"]}]));
        assert_eq!(l[0].empreintes, ["abc", "def"]);
        let h = lire_empreintes_du_jeu(&json!({"Results": [{"MD5": "ABC", "Name": "Super Mario Bros. (World)", "Labels": ["nointro"], "PatchUrl": null}]}));
        assert_eq!(h[0].nom, "Super Mario Bros. (World)");
        let p = lire_progression(&json!({"ID": 1446, "Title": "SMB", "NumAchievements": 2, "NumAwardedToUser": 1, "NumAwardedToUserHardcore": 0,
            "ImageIcon": "/Images/1.png",
            "Achievements": {"1": {"Points": 5, "DateEarned": "2026-10-01 10:00:00"}, "2": {"Points": 10}}}));
        assert_eq!((p.obtenus, p.succes, p.points, p.points_obtenus), (1, 2, 15, 5));
        assert_eq!(p.image.as_deref(), Some("https://media.retroachievements.org/Images/1.png"));
        assert_eq!(lire_succes_steam(&json!({"playerstats": {"achievements": [{"achieved": 1}, {"achieved": 0}]}})), Some((1, 2)));
        assert_eq!(lire_succes_steam(&json!({"playerstats": {"error": "Requested app has no stats"}})), None);
        assert_eq!(titre_comparable("Legend of Zelda, The (USA)"), titre_comparable("legend of zelda"));
    }
}

#[cfg(test)]
mod essais {
    /// Sur une vraie ROM de Seb, en LECTURE SEULE : son empreinte RetroAchievements.
    #[test]
    #[ignore]
    fn essai_empreinte_reelle() {
        let f = std::env::var("FROGTEND_ESSAI_ROM").unwrap_or_else(|_| "E:/Games/Nintendo Entertainement System/Europe/Super Mario Bros (E).nes".into());
        let o = std::fs::read(&f).unwrap();
        println!("taille {} ; en-tête iNES : {}", o.len(), o.starts_with(b"NES\x1a"));
        println!("empreinte RA : {:?}", super::empreinte_fichier(7, std::path::Path::new(&f)).unwrap());
    }
}
