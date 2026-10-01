//! Les émulateurs : leur source OFFICIELLE, leur dernière version, leur installation (sur accord) dans le dossier
//! « Émulateurs » réglé par la personne, leurs mises à jour, les cœurs RetroArch, les BIOS.
//!
//! Chaque émulateur est installé en mode PORTABLE (sa configuration reste dans son dossier : elle se sauvegarde avec
//! lui et ne se mélange pas au reste du PC). Les lignes de commande viennent des sources officielles (relevées le
//! 30/09/2026) : RetroArch `-L cœur -f`, DOSBox Staging `--fullscreen --exit`, DuckStation et PCSX2
//! `-batch -fullscreen --`, Dolphin `-b -e`, PPSSPP (le jeu seul).

use crate::erreurs::{Erreur, Resultat};
use crate::installation::{decompresser, fichiers_de, Format};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// D'où vient un émulateur.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Origine {
    /// Le « buildbot » de libretro : dernière version stable de RetroArch.
    RetroArch,
    /// Les « releases » GitHub officielles : le paquet dont le nom contient tous ces morceaux.
    GitHub { depot: &'static str, morceaux: &'static [&'static str] },
    /// Le serveur de mise à jour officiel de Dolphin.
    Dolphin,
    /// Les « releases » d'un Forgejo/Gitea officiel (même forme de réponse que GitHub) : `api` = l'adresse du dépôt
    /// dans son API (`https://…/api/v1/repos/<proprio>/<depot>`).
    Forgejo { api: &'static str, morceaux: &'static [&'static str] },
}

/// Ce que Frogtend sait d'un émulateur.
#[derive(Debug, Clone, Copy)]
pub struct Fiche {
    pub id: &'static str,
    pub nom: &'static str,
    /// Les noms sous lesquels Firehouse le recommande (en minuscules).
    pub alias: &'static [&'static str],
    pub origine: Origine,
    /// Le programme à lancer (son nom commence ainsi, sans tenir compte des majuscules).
    pub programme: &'static str,
    /// La ligne de commande par défaut (le jeu est ajouté à la fin).
    pub ligne: &'static str,
    /// Le fichier qui met l'émulateur en mode portable, s'il en faut un.
    pub portable: Option<&'static str>,
    /// Les dossiers où il est souvent installé à la main (pour le retrouver sans chercher sur tout le disque).
    pub dossiers_habituels: &'static [&'static str],
}

pub const CATALOGUE: &[Fiche] = &[
    Fiche {
        id: "retroarch",
        nom: "RetroArch",
        alias: &["retroarch"],
        origine: Origine::RetroArch,
        programme: "retroarch.exe",
        ligne: "-f",
        portable: None, // la version .7z est portable d'elle-même
        dossiers_habituels: &["%PROGRAMFILES%\\RetroArch", "C:\\RetroArch-Win64", "%APPDATA%\\RetroArch",
            "%PROGRAMFILES(X86)%\\Steam\\steamapps\\common\\RetroArch"],
    },
    Fiche {
        id: "dosbox-staging",
        nom: "DOSBox Staging",
        alias: &["dosbox staging", "dosbox-staging", "dosbox"],
        origine: Origine::GitHub { depot: "dosbox-staging/dosbox-staging", morceaux: &["windows-x64", ".zip"] },
        programme: "dosbox.exe",
        ligne: "--fullscreen --exit",
        // Un « dosbox-staging.conf » à côté du programme = configuration portable (src/misc/cross.cpp).
        portable: Some("dosbox-staging.conf"),
        dossiers_habituels: &["%PROGRAMFILES%\\DOSBox Staging", "%LOCALAPPDATA%\\Programs\\DOSBox Staging"],
    },
    Fiche {
        id: "duckstation",
        nom: "DuckStation",
        alias: &["duckstation"],
        origine: Origine::GitHub { depot: "stenzek/duckstation", morceaux: &["windows-x64-release.zip"] },
        programme: "duckstation-qt",
        ligne: "-batch -fullscreen --",
        portable: Some("portable.txt"),
        dossiers_habituels: &["%PROGRAMFILES%\\DuckStation", "%LOCALAPPDATA%\\Programs\\DuckStation"],
    },
    Fiche {
        id: "pcsx2",
        nom: "PCSX2",
        alias: &["pcsx2"],
        origine: Origine::GitHub { depot: "PCSX2/pcsx2", morceaux: &["windows-x64-Qt.7z"] },
        programme: "pcsx2-qt",
        ligne: "-batch -fullscreen --",
        portable: Some("portable.ini"),
        dossiers_habituels: &["%PROGRAMFILES%\\PCSX2"],
    },
    Fiche {
        id: "dolphin",
        nom: "Dolphin",
        alias: &["dolphin"],
        origine: Origine::Dolphin,
        programme: "dolphin.exe",
        ligne: "-b -e",
        portable: Some("portable.txt"),
        dossiers_habituels: &["%PROGRAMFILES%\\Dolphin", "%PROGRAMFILES%\\Dolphin-x64"],
    },
    Fiche {
        id: "ppsspp",
        nom: "PPSSPP",
        alias: &["ppsspp"],
        origine: Origine::GitHub { depot: "hrydgard/ppsspp", morceaux: &["Windows-x64.zip"] },
        programme: "ppssppwindows64",
        ligne: "",
        portable: None, // portable tant qu'il n'y a pas de « installed.txt »
        dossiers_habituels: &["%PROGRAMFILES%\\PPSSPP"],
    },
    // Consoles récentes (lot 4d), relevé dans les sources le 01/10/2026.
    Fiche {
        id: "xenia",
        nom: "Xenia Canary",
        alias: &["xenia", "xenia canary"],
        // Étiquette = un numéro de commit : la date sert de version.
        origine: Origine::GitHub { depot: "xenia-canary/xenia-canary", morceaux: &["xenia_canary_windows.7z"] },
        programme: "xenia_canary.exe",
        ligne: "--fullscreen",
        portable: None, // portable d'office sous Windows (xenia_main.cc : `portable` vrai sous WIN32)
        dossiers_habituels: &[],
    },
    Fiche {
        id: "xemu",
        nom: "xemu",
        alias: &["xemu"],
        origine: Origine::GitHub { depot: "xemu-project/xemu", morceaux: &["windows-x86_64.zip", "!dbg", "!pdb"] },
        programme: "xemu.exe",
        ligne: "-dvd_path",
        // Un « xemu.toml » à côté du programme = mode portable (ui/xemu-settings.cc).
        portable: Some("xemu.toml"),
        dossiers_habituels: &[],
    },
    Fiche {
        id: "rpcs3",
        nom: "RPCS3",
        alias: &["rpcs3"],
        origine: Origine::GitHub { depot: "RPCS3/rpcs3-binaries-win", morceaux: &["win64_msvc.7z", "!sha256"] },
        programme: "rpcs3.exe",
        ligne: "--no-gui --fullscreen",
        portable: None, // sous Windows, tout est à côté du programme (Utilities/File.cpp, get_config_dir)
        dossiers_habituels: &[],
    },
    Fiche {
        id: "cemu",
        nom: "Cemu",
        alias: &["cemu"],
        origine: Origine::GitHub { depot: "cemu-project/Cemu", morceaux: &["windows-x64.zip"] },
        programme: "cemu.exe",
        ligne: "-f -g",
        portable: Some("portable/"),
        dossiers_habituels: &[],
    },
    Fiche {
        id: "azahar",
        nom: "Azahar",
        alias: &["azahar"],
        origine: Origine::GitHub { depot: "azahar-emu/azahar", morceaux: &["windows-msvc", ".zip", "!installer", "!libretro"] },
        programme: "azahar.exe",
        ligne: "-f",
        // Un dossier « user » à côté du programme = mode portable (common/file_util.cpp).
        portable: Some("user/"),
        dossiers_habituels: &[],
    },
    Fiche {
        id: "eden",
        nom: "Eden",
        alias: &["eden"],
        // Dépôt officiel git.eden-emu.dev (Forgejo), relevé le 01/10/2026 (v0.2.1).
        origine: Origine::Forgejo {
            api: "https://git.eden-emu.dev/api/v1/repos/eden-emu/eden",
            morceaux: &["Windows", "amd64-msvc-standard.zip"],
        },
        programme: "eden.exe",
        ligne: "-f -g",
        // Un dossier « user » à côté du programme = mode portable (common/fs/path_util.cpp).
        portable: Some("user/"),
        dossiers_habituels: &[],
    },
    Fiche {
        id: "vita3k",
        nom: "Vita3K",
        alias: &["vita3k"],
        origine: Origine::GitHub { depot: "Vita3K/Vita3K", morceaux: &["windows-latest.zip"] },
        programme: "vita3k.exe",
        ligne: "-F",
        portable: Some("portable/"), // vita3k/app/src/app_init.cpp
        dossiers_habituels: &[],
    },
];

/// La fiche d'un émulateur recommandé par Firehouse (d'après son nom).
pub fn fiche_pour(nom_firehouse: &str) -> Option<&'static Fiche> {
    let n = nom_firehouse.trim().to_lowercase();
    CATALOGUE.iter().find(|f| f.alias.iter().any(|a| n == *a || n.starts_with(a)))
}

pub fn fiche(id: &str) -> Option<&'static Fiche> {
    CATALOGUE.iter().find(|f| f.id == id)
}

/// Un paquet à télécharger.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Paquet {
    pub version: String,
    pub url: String,
    /// Taille annoncée, si la source la donne.
    pub taille: Option<u64>,
}

/// Un émulateur présent sur ce PC.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmulateurInstalle {
    pub id: String,
    pub nom: String,
    /// `None` : installé à la main, version inconnue.
    pub version: Option<String>,
    pub dossier: String,
    pub programme: String,
    /// Vrai si Frogtend l'a installé (et peut donc le mettre à jour).
    pub par_frogtend: bool,
    pub installe_le: String,
}

fn client() -> Resultat<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(90))
        .user_agent(concat!("Frogtend/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|_| Erreur::Reseau("Impossible de préparer la connexion.".into()))
}

fn reseau(e: reqwest::Error) -> Erreur {
    Erreur::Reseau(format!("Impossible de joindre le site de l'émulateur ({e})."))
}

/// Compare deux versions « 1.22.2 » / « v0.83.0 » / « 2609 » (les parties numériques, dans l'ordre).
pub fn plus_recente(a: &str, b: &str) -> bool {
    let nombres = |v: &str| -> Vec<u64> {
        v.split(|c: char| !c.is_ascii_digit()).filter(|s| !s.is_empty()).filter_map(|s| s.parse().ok()).collect()
    };
    nombres(a) > nombres(b)
}

/// La dernière version stable de RetroArch, d'après l'index du buildbot.
pub fn version_retroarch_depuis_index(html: &str) -> Option<String> {
    let mut versions: Vec<String> = html
        .split("href=\"/stable/")
        .skip(1)
        .filter_map(|m| m.split('/').next())
        .filter(|v| v.split('.').count() == 3 && v.split('.').all(|p| p.parse::<u32>().is_ok()))
        .map(String::from)
        .collect();
    versions.sort_by(|a, b| if plus_recente(a, b) { std::cmp::Ordering::Greater } else { std::cmp::Ordering::Less });
    versions.pop()
}

/// Le paquet Windows x64 d'une « release » GitHub (réponse de l'API).
pub fn paquet_github(release: &Value, morceaux: &[&str]) -> Option<Paquet> {
    // Une « release » toujours étiquetée « latest » (DuckStation) : sa date de publication sert de version.
    let etiquette = release["tag_name"].as_str()?;
    let pas_un_numero = etiquette == "latest"
        || etiquette == "continuous"
        || etiquette.starts_with("build-")
        || (etiquette.len() >= 7 && etiquette.len() <= 40 && etiquette.chars().all(|c| c.is_ascii_hexdigit()));
    let version = if pas_un_numero {
        release["published_at"].as_str().map(|d| d.chars().take(10).collect()).unwrap_or_else(|| etiquette.into())
    } else {
        etiquette.to_string()
    };
    let a = release["assets"].as_array()?.iter().find(|a| {
        let n = a["name"].as_str().unwrap_or("");
        morceaux.iter().all(|m| match m.strip_prefix('!') {
            Some(exclu) => !n.contains(exclu),
            None => n.contains(m),
        }) && !n.contains("symbols")
    })?;
    Some(Paquet { version, url: a["browser_download_url"].as_str()?.into(), taille: a["size"].as_u64() })
}

/// Le paquet Windows x64 de Dolphin (réponse du serveur de mise à jour).
pub fn paquet_dolphin(v: &Value) -> Option<Paquet> {
    let version = v["shortrev"].as_str()?.to_string();
    let url = v["artifacts"].as_array()?.iter().find(|a| a["system"] == "Windows x64")?["url"].as_str()?.to_string();
    Some(Paquet { version, url, taille: None })
}

/// La taille d'un fichier distant, d'après l'en-tête `Content-Length` d'une requête HEAD (reqwest ne la rend pas
/// directement pour une réponse sans corps).
async fn taille_annoncee(c: &reqwest::Client, url: &str) -> Option<u64> {
    let r = c.head(url).send().await.ok()?;
    r.headers().get(reqwest::header::CONTENT_LENGTH)?.to_str().ok()?.parse().ok()
}

/// La dernière version officielle d'un émulateur.
pub async fn derniere_version(f: &Fiche) -> Resultat<Paquet> {
    let c = client()?;
    let introuvable = || Erreur::Serveur(format!("La dernière version de {} est introuvable sur son site.", f.nom));
    match f.origine {
        Origine::RetroArch => {
            let html = c.get("https://buildbot.libretro.com/stable/").send().await.map_err(reseau)?.text().await.map_err(reseau)?;
            let v = version_retroarch_depuis_index(&html).ok_or_else(introuvable)?;
            let url = format!("https://buildbot.libretro.com/stable/{v}/windows/x86_64/RetroArch.7z");
            let taille = taille_annoncee(&c, &url).await;
            Ok(Paquet { version: v, url, taille })
        }
        Origine::GitHub { depot, morceaux } => {
            let r: Value = c
                .get(format!("https://api.github.com/repos/{depot}/releases/latest"))
                .header("Accept", "application/vnd.github+json")
                .send()
                .await
                .map_err(reseau)?
                .json()
                .await
                .map_err(reseau)?;
            paquet_github(&r, morceaux).ok_or_else(introuvable)
        }
        Origine::Forgejo { api, morceaux } => {
            let r: Value = c.get(format!("{api}/releases/latest")).send().await.map_err(reseau)?.json().await.map_err(reseau)?;
            let mut p = paquet_github(&r, morceaux).ok_or_else(introuvable)?;
            // Le serveur annonce une taille 0 pour les paquets hébergés ailleurs : on la demande au fichier.
            if p.taille.unwrap_or(0) == 0 {
                p.taille = taille_annoncee(&c, &p.url).await;
            }
            Ok(p)
        }
        Origine::Dolphin => {
            let v: Value = c
                .get("https://dolphin-emu.org/update/latest/beta/")
                .send()
                .await
                .map_err(reseau)?
                .json()
                .await
                .map_err(reseau)?;
            let mut p = paquet_dolphin(&v).ok_or_else(introuvable)?;
            p.taille = taille_annoncee(&c, &p.url).await;
            Ok(p)
        }
    }
}

/// Télécharge une adresse vers un fichier (en passant par `.part`), et vérifie la taille reçue.
pub async fn telecharger(url: &str, vers: &Path, progres: &(dyn Fn(u64, Option<u64>) + Send + Sync)) -> Resultat<u64> {
    let c = client()?;
    let mut rep = c.get(url).send().await.map_err(reseau)?;
    if !rep.status().is_success() {
        return Err(Erreur::Serveur(format!("Le site de l'émulateur a répondu {}.", rep.status())));
    }
    let attendu = rep.content_length();
    if let Some(p) = vers.parent() {
        std::fs::create_dir_all(p)?;
    }
    let part = vers.with_extension("part");
    let mut sortie = std::fs::File::create(&part)?;
    let mut recus = 0u64;
    while let Some(m) = rep.chunk().await.map_err(reseau)? {
        sortie.write_all(&m)?;
        recus += m.len() as u64;
        progres(recus, attendu);
    }
    sortie.flush()?;
    drop(sortie);
    if attendu.is_some_and(|a| a != recus) {
        let _ = std::fs::remove_file(&part);
        return Err(Erreur::Reseau(format!("Téléchargement incomplet ({recus} octets sur {}).", attendu.unwrap_or(0))));
    }
    std::fs::rename(&part, vers)?;
    Ok(recus)
}

/// Le programme d'un émulateur dans son dossier (cherché : certains paquets ont un sous-dossier).
pub fn trouver_programme(dossier: &Path, f: &Fiche) -> Option<PathBuf> {
    let mut l: Vec<String> = fichiers_de(dossier).ok()?;
    l.sort_by_key(|r| r.matches('/').count()); // le moins profond d'abord
    l.into_iter()
        .find(|r| {
            let nom = r.rsplit('/').next().unwrap_or(r).to_lowercase();
            nom.starts_with(f.programme) && nom.ends_with(".exe")
        })
        .map(|r| dossier.join(r))
}

/// Installe (ou met à jour) un émulateur à partir de son paquet déjà téléchargé : décompression à côté, puis
/// copie par-dessus le dossier de l'émulateur. Ce qui n'est pas dans le paquet (sa configuration, les BIOS, les
/// parties) n'est jamais touché. Le mode portable est activé.
pub fn installer_paquet(f: &Fiche, paquet: &Path, dossier: &Path) -> Resultat<PathBuf> {
    let format = crate::installation::format_de(paquet)?;
    if !matches!(format, Format::Zip | Format::SeptZip) {
        return Err(Erreur::Refus(format!("Le paquet de {} n'est pas une archive attendue.", f.nom)));
    }
    let provisoire = dossier.with_extension("nouveau");
    if provisoire.exists() {
        std::fs::remove_dir_all(&provisoire)?;
    }
    decompresser(paquet, format, &provisoire)?;
    // Un paquet qui met tout dans un seul sous-dossier : on prend son contenu.
    let mut racine = provisoire.clone();
    let entrees: Vec<_> = std::fs::read_dir(&provisoire)?.flatten().collect();
    if entrees.len() == 1 && entrees[0].path().is_dir() {
        racine = entrees[0].path();
    }
    std::fs::create_dir_all(dossier)?;
    // Une configuration qui serait remplacée par celle du paquet est d'abord copiée à part.
    let abri = dossier.join(".frogtend-sauvegardes").join(crate::noyau::maintenant());
    for r in fichiers_de(&racine)? {
        let cible = dossier.join(&r);
        if est_configuration(&r) && cible.is_file() && std::fs::read(&cible).ok() != std::fs::read(racine.join(&r)).ok() {
            let copie = abri.join(&r);
            if let Some(p) = copie.parent() {
                std::fs::create_dir_all(p)?;
            }
            std::fs::copy(&cible, copie)?;
        }
        if let Some(p) = cible.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::copy(racine.join(&r), &cible)?;
    }
    std::fs::remove_dir_all(&provisoire)?;
    if let Some(p) = f.portable {
        let chemin = dossier.join(p.trim_end_matches('/'));
        if p.ends_with('/') {
            std::fs::create_dir_all(&chemin)?; // un DOSSIER à côté du programme (Azahar, Cemu, Vita3K)
        } else if !chemin.exists() {
            std::fs::write(chemin, b"")?;
        }
    }
    // PPSSPP : un « installed.txt » l'enverrait dans Documents (Windows/main.cpp) ; sans lui, tout reste à côté.
    if f.id == "ppsspp" {
        let installe = dossier.join("installed.txt");
        if installe.is_file() {
            std::fs::remove_file(installe)?;
        }
    }
    trouver_programme(dossier, f)
        .ok_or_else(|| Erreur::Disque(format!("Après l'installation, le programme de {} est introuvable.", f.nom)))
}

/// Un fichier de configuration (réglages, manettes…) : on le met à l'abri avant de l'écraser.
fn est_configuration(relatif: &str) -> bool {
    let n = relatif.to_lowercase();
    [".cfg", ".ini", ".conf", ".json", ".xml", ".opt"].iter().any(|e| n.ends_with(e))
}

/// Remplace `%VARIABLE%` par sa valeur.
fn developper(chemin: &str) -> String {
    let mut s = chemin.to_string();
    for v in ["PROGRAMFILES(X86)", "PROGRAMFILES", "LOCALAPPDATA", "APPDATA"] {
        if let Ok(val) = std::env::var(v) {
            s = s.replace(&format!("%{v}%"), &val);
        }
    }
    s
}

/// Cherche les émulateurs déjà installés à la main, dans leurs dossiers habituels seulement (jamais tout le disque).
pub fn detecter() -> Vec<EmulateurInstalle> {
    let mut l = Vec::new();
    for f in CATALOGUE {
        for d in f.dossiers_habituels {
            let dossier = PathBuf::from(developper(d));
            if !dossier.is_dir() {
                continue;
            }
            let Ok(entrees) = std::fs::read_dir(&dossier) else { continue };
            let programme = entrees.flatten().map(|e| e.path()).find(|p| {
                let n = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
                n.starts_with(f.programme) && n.ends_with(".exe")
            });
            if let Some(p) = programme {
                l.push(EmulateurInstalle {
                    id: f.id.into(),
                    nom: f.nom.into(),
                    version: None,
                    dossier: dossier.to_string_lossy().into(),
                    programme: p.to_string_lossy().into(),
                    par_frogtend: false,
                    installe_le: String::new(),
                });
                break;
            }
        }
    }
    l
}

/// Le nom du cœur RetroArch dans une ligne de commande (`-L "cores\snes9x_libretro.dll" -f` → `snes9x_libretro.dll`).
pub fn coeur_de(ligne: &str) -> Option<String> {
    let args = crate::installation::decouper(ligne);
    let i = args.iter().position(|a| a == "-L" || a == "--libretro")?;
    let chemin = args.get(i + 1)?;
    Some(chemin.rsplit(['\\', '/']).next()?.to_string())
}

/// L'adresse officielle d'un cœur RetroArch (dernière version compilée pour Windows x64).
pub fn url_coeur(coeur: &str) -> String {
    format!("https://buildbot.libretro.com/nightly/windows/x86_64/latest/{coeur}.zip")
}

/// Installe un cœur RetroArch (paquet zip officiel déjà téléchargé) dans `cores/`.
pub fn installer_coeur(paquet: &Path, retroarch: &Path, coeur: &str) -> Resultat<PathBuf> {
    let cores = retroarch.join("cores");
    decompresser(paquet, Format::Zip, &cores)?;
    let chemin = cores.join(coeur);
    if chemin.is_file() {
        Ok(chemin)
    } else {
        Err(Erreur::Disque(format!("Après l'installation, le cœur {coeur} est introuvable.")))
    }
}

/// Ajoute les profils de manette officiels de RetroArch (paquet zip déjà téléchargé) dans `autoconfig\`. Un profil
/// déjà présent n'est jamais remplacé. Rend le nombre de profils ajoutés.
pub fn installer_profils_manette(paquet: &Path, retroarch: &Path) -> Resultat<usize> {
    let provisoire = retroarch.join("autoconfig.nouveau");
    if provisoire.exists() {
        std::fs::remove_dir_all(&provisoire)?;
    }
    decompresser(paquet, Format::Zip, &provisoire)?;
    let cible = retroarch.join("autoconfig");
    let mut ajoutes = 0;
    for r in fichiers_de(&provisoire)? {
        let c = cible.join(&r);
        if c.exists() {
            continue;
        }
        if let Some(p) = c.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::copy(provisoire.join(&r), &c)?;
        ajoutes += 1;
    }
    std::fs::remove_dir_all(&provisoire)?;
    Ok(ajoutes)
}

/// Le dossier des BIOS de RetroArch : `system_directory` de retroarch.cfg s'il est réglé, sinon `system`.
pub fn dossier_bios_retroarch(dossier: &Path) -> PathBuf {
    let defaut = dossier.join("system");
    let Ok(cfg) = std::fs::read_to_string(dossier.join("retroarch.cfg")) else { return defaut };
    for ligne in cfg.lines() {
        if let Some(v) = ligne.trim().strip_prefix("system_directory") {
            let v = v.trim().trim_start_matches('=').trim().trim_matches('"');
            if v.is_empty() || v == "default" {
                return defaut;
            }
            // « :\system » = relatif au dossier de RetroArch (notation de RetroArch).
            if let Some(relatif) = v.strip_prefix(":\\").or_else(|| v.strip_prefix(":/")) {
                return dossier.join(relatif);
            }
            return PathBuf::from(v);
        }
    }
    defaut
}

/// Le registre des émulateurs de ce PC (`emulateurs.json`).
pub struct Registre {
    fichier: PathBuf,
}

impl Registre {
    pub fn nouveau(fichier: &Path) -> Self {
        Registre { fichier: fichier.into() }
    }
    pub fn tous(&self) -> Vec<EmulateurInstalle> {
        std::fs::read(&self.fichier).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default()
    }
    pub fn retenir(&self, e: EmulateurInstalle) -> Resultat<()> {
        let mut l = self.tous();
        l.retain(|x| x.id != e.id);
        l.push(e);
        if let Some(d) = self.fichier.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::write(&self.fichier, serde_json::to_vec_pretty(&l).unwrap())?;
        Ok(())
    }
    pub fn un(&self, id: &str) -> Option<EmulateurInstalle> {
        self.tous().into_iter().find(|e| e.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn les_noms_de_firehouse_trouvent_leur_fiche() {
        assert_eq!(fiche_pour("Retroarch").unwrap().id, "retroarch");
        assert_eq!(fiche_pour("DuckStation").unwrap().id, "duckstation");
        assert_eq!(fiche_pour("PCSX2").unwrap().id, "pcsx2");
        assert_eq!(fiche_pour("Dolphin").unwrap().id, "dolphin");
        assert_eq!(fiche_pour("DOSBox Staging").unwrap().id, "dosbox-staging");
        assert!(fiche_pour("Project64").is_none());
    }

    #[test]
    fn les_versions_se_comparent_par_leurs_nombres() {
        assert!(plus_recente("1.22.10", "1.22.2"));
        assert!(plus_recente("v0.83.0", "v0.82.9"));
        assert!(plus_recente("2609", "2512"));
        assert!(!plus_recente("1.22.2", "1.22.2"));
    }

    #[test]
    fn la_derniere_version_de_retroarch_se_lit_dans_l_index() {
        let html = r#"<a href="/stable/1.9.0/">1.9.0</a><a href="/stable/1.22.2/">1.22.2</a><a href="/stable/1.22.10/">x</a><a href="/stable/nightly/">n</a>"#;
        assert_eq!(version_retroarch_depuis_index(html).as_deref(), Some("1.22.10"));
    }

    #[test]
    fn le_paquet_github_est_le_bon_et_jamais_les_symboles() {
        let r = json!({"tag_name": "v2.8.2", "assets": [
            {"name": "pcsx2-v2.8.2-windows-x64-Qt-symbols.7z", "browser_download_url": "s", "size": 1},
            {"name": "pcsx2-v2.8.2-windows-x64-installer.exe", "browser_download_url": "i", "size": 2},
            {"name": "pcsx2-v2.8.2-windows-x64-Qt.7z", "browser_download_url": "https://github.com/x.7z", "size": 25670075}]});
        let p = paquet_github(&r, &["windows-x64-Qt.7z"]).unwrap();
        assert_eq!((p.version.as_str(), p.url.as_str(), p.taille), ("v2.8.2", "https://github.com/x.7z", Some(25670075)));
    }

    /// Les vrais noms relevés le 01/10/2026 (API GitHub) pour les consoles récentes.
    #[test]
    fn les_consoles_recentes_trouvent_leur_paquet_windows() {
        let paquet = |id: &str, tag: &str, noms: &[&str]| {
            let assets: Vec<Value> = noms.iter().map(|n| json!({"name": n, "browser_download_url": n, "size": 1})).collect();
            let r = json!({"tag_name": tag, "published_at": "2026-10-01T05:40:32Z", "assets": assets});
            match fiche(id).unwrap().origine {
                Origine::GitHub { morceaux, .. } => paquet_github(&r, morceaux).unwrap(),
                _ => unreachable!(),
            }
        };
        let x = paquet("xemu", "v0.8.136", &[
            "xemu-0.8.136-dbg-windows-x86_64.zip", "xemu-0.8.136-windows-x86_64-pdb.zip",
            "xemu-0.8.136-dbg-windows-x86_64-pdb.zip", "xemu-0.8.136-windows-arm64.zip", "xemu-0.8.136-windows-x86_64.zip"]);
        assert_eq!((x.url.as_str(), x.version.as_str()), ("xemu-0.8.136-windows-x86_64.zip", "v0.8.136"));
        let r = paquet("rpcs3", "build-4d88114c92aece3ebe6b613a516db29882b06dc9", &[
            "rpcs3-v0.0.43-20146-4d88114c_win64_msvc.7z.sha256", "rpcs3-v0.0.43-20146-4d88114c_win64_msvc.7z"]);
        assert_eq!((r.url.as_str(), r.version.as_str()), ("rpcs3-v0.0.43-20146-4d88114c_win64_msvc.7z", "2026-10-01"));
        let a = paquet("azahar", "2126.1.2", &[
            "azahar-libretro-windows-x86_64-2126.1.2.zip", "azahar-windows-msvc-2126.1.2-installer.exe",
            "azahar-windows-msvc-2126.1.2.zip", "azahar-windows-msys2-2126.1.2.zip"]);
        assert_eq!(a.url, "azahar-windows-msvc-2126.1.2.zip");
        let xe = paquet("xenia", "44f5b4a", &["xenia_canary_linux.AppImage", "xenia_canary_windows.7z"]);
        assert_eq!((xe.url.as_str(), xe.version.as_str()), ("xenia_canary_windows.7z", "2026-10-01"), "commit → date");
        let v = paquet("vita3k", "continuous", &["windows-arm64-latest.zip", "windows-latest.zip"]);
        assert_eq!((v.url.as_str(), v.version.as_str()), ("windows-latest.zip", "2026-10-01"));
        assert_eq!(paquet("cemu", "v2.6", &["cemu-2.6-ubuntu-22.04-x64.zip", "cemu-2.6-windows-x64.zip"]).url, "cemu-2.6-windows-x64.zip");

        // Eden : Forgejo officiel, même forme de réponse (noms relevés le 01/10/2026).
        let Origine::Forgejo { morceaux, .. } = fiche("eden").unwrap().origine else { unreachable!() };
        let r = json!({"tag_name": "v0.2.1", "assets": [
            {"name": "Eden-Windows-v0.2.1-amd64-clang-pgo.zip", "browser_download_url": "a", "size": 0},
            {"name": "Eden-Windows-v0.2.1-arm64-clang-standard.zip", "browser_download_url": "b", "size": 0},
            {"name": "Eden-Windows-v0.2.1-amd64-msvc-standard.zip", "browser_download_url": "c", "size": 0},
            {"name": "Eden-Windows-v0.2.1-rog-ally-gcc-standard.zip", "browser_download_url": "d", "size": 0}]});
        assert_eq!(paquet_github(&r, morceaux).unwrap().url, "c");
    }

    #[test]
    fn un_marqueur_portable_peut_etre_un_dossier() {
        let d = tempfile::tempdir().unwrap();
        let paquet = d.path().join("a.zip");
        {
            let mut z = zip::ZipWriter::new(std::fs::File::create(&paquet).unwrap());
            z.start_file("azahar.exe", zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(b"MZ").unwrap();
            z.start_file("azahar-room.exe", zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(b"MZ").unwrap();
            z.finish().unwrap();
        }
        let dossier = d.path().join("Azahar");
        let p = installer_paquet(fiche("azahar").unwrap(), &paquet, &dossier).unwrap();
        assert_eq!(p, dossier.join("azahar.exe"), "pas azahar-room.exe");
        assert!(dossier.join("user").is_dir(), "le dossier « user » rend Azahar portable");
    }

    #[test]
    fn une_release_toujours_nommee_latest_prend_sa_date_pour_version() {
        let r = json!({"tag_name": "latest", "published_at": "2026-09-12T12:20:15Z", "assets": [
            {"name": "duckstation-windows-x64-release.zip", "browser_download_url": "u", "size": 1}]});
        let p = paquet_github(&r, &["windows-x64-release.zip"]).unwrap();
        assert_eq!(p.version, "2026-09-12");
        assert!(plus_recente("2026-10-01", &p.version));
    }

    #[test]
    fn le_paquet_de_dolphin_vient_de_son_serveur_de_mise_a_jour() {
        let v = json!({"shortrev": "2609", "artifacts": [
            {"system": "Android", "url": "a.apk"},
            {"system": "Windows x64", "url": "https://dl.dolphin-emu.org/releases/2609/dolphin-2609-x64.7z"}]});
        let p = paquet_dolphin(&v).unwrap();
        assert_eq!(p.url, "https://dl.dolphin-emu.org/releases/2609/dolphin-2609-x64.7z");
    }

    #[test]
    fn un_paquet_s_installe_en_portable_sans_ecraser_la_configuration() {
        let d = tempfile::tempdir().unwrap();
        let f = fiche("duckstation").unwrap();
        let paquet = d.path().join("duck.zip");
        {
            let mut z = zip::ZipWriter::new(std::fs::File::create(&paquet).unwrap());
            let o = zip::write::SimpleFileOptions::default();
            z.start_file("duckstation-qt-x64-ReleaseLTCG.exe", o).unwrap();
            z.write_all(b"MZ v2").unwrap();
            z.start_file("resources/font.ttf", o).unwrap();
            z.write_all(b"f").unwrap();
            z.finish().unwrap();
        }
        let dossier = d.path().join("Emulateurs").join("DuckStation");
        std::fs::create_dir_all(&dossier).unwrap();
        std::fs::write(dossier.join("settings.ini"), b"ma configuration").unwrap();
        let programme = installer_paquet(f, &paquet, &dossier).unwrap();
        assert!(programme.ends_with("duckstation-qt-x64-ReleaseLTCG.exe"));
        assert!(dossier.join("portable.txt").is_file(), "mode portable activé");
        assert_eq!(std::fs::read(dossier.join("settings.ini")).unwrap(), b"ma configuration", "configuration gardée");
        assert!(!dossier.with_extension("nouveau").exists());
    }

    #[test]
    fn une_mise_a_jour_met_a_l_abri_la_configuration_qu_elle_remplace() {
        let d = tempfile::tempdir().unwrap();
        let f = fiche("dosbox-staging").unwrap();
        let paquet = d.path().join("db.zip");
        {
            let mut z = zip::ZipWriter::new(std::fs::File::create(&paquet).unwrap());
            let o = zip::write::SimpleFileOptions::default();
            z.start_file("dosbox.exe", o).unwrap();
            z.write_all(b"MZ v2").unwrap();
            z.start_file("dosbox-staging.conf", o).unwrap();
            z.write_all(b"conf d'origine").unwrap();
            z.finish().unwrap();
        }
        let dossier = d.path().join("DOSBox");
        std::fs::create_dir_all(&dossier).unwrap();
        std::fs::write(dossier.join("dosbox-staging.conf"), b"ma conf").unwrap();
        installer_paquet(f, &paquet, &dossier).unwrap();
        let abris: Vec<_> = std::fs::read_dir(dossier.join(".frogtend-sauvegardes")).unwrap().flatten().collect();
        assert_eq!(std::fs::read(abris[0].path().join("dosbox-staging.conf")).unwrap(), b"ma conf");
    }

    #[test]
    fn chaque_emulateur_reste_dans_son_dossier() {
        // Relevé dans les sources officielles le 30/09/2026.
        let attendu = [
            ("duckstation", Some("portable.txt")),
            ("pcsx2", Some("portable.ini")),
            ("dolphin", Some("portable.txt")),
            ("dosbox-staging", Some("dosbox-staging.conf")),
            ("retroarch", None), // le paquet .7z est portable de lui-même
            ("ppsspp", None),    // portable tant qu'il n'y a pas d'installed.txt
        ];
        for (id, portable) in attendu {
            assert_eq!(fiche(id).unwrap().portable, portable, "{id}");
        }
    }

    #[test]
    fn un_paquet_avec_un_sous_dossier_est_mis_a_plat() {
        let d = tempfile::tempdir().unwrap();
        let f = fiche("retroarch").unwrap();
        let paquet = d.path().join("ra.zip");
        {
            let mut z = zip::ZipWriter::new(std::fs::File::create(&paquet).unwrap());
            z.start_file("RetroArch-Win64/retroarch.exe", zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(b"MZ").unwrap();
            z.finish().unwrap();
        }
        let dossier = d.path().join("RetroArch");
        let p = installer_paquet(f, &paquet, &dossier).unwrap();
        assert_eq!(p, dossier.join("retroarch.exe"));
    }

    #[test]
    fn les_profils_de_manette_de_retroarch_s_ajoutent_sans_remplacer_ceux_deja_la() {
        let d = tempfile::tempdir().unwrap();
        let paquet = d.path().join("autoconfig.zip");
        {
            let mut z = zip::ZipWriter::new(std::fs::File::create(&paquet).unwrap());
            let o = zip::write::SimpleFileOptions::default();
            z.start_file("xinput/XInput Controller.cfg", o).unwrap();
            z.write_all(b"officiel").unwrap();
            z.start_file("dinput/Wireless Controller.cfg", o).unwrap();
            z.write_all(b"officiel").unwrap();
            z.finish().unwrap();
        }
        let ra = d.path().join("RetroArch");
        let perso = ra.join("autoconfig").join("dinput").join("Wireless Controller.cfg");
        std::fs::create_dir_all(perso.parent().unwrap()).unwrap();
        std::fs::write(&perso, b"perso").unwrap();
        assert_eq!(installer_profils_manette(&paquet, &ra).unwrap(), 1);
        assert_eq!(std::fs::read(&perso).unwrap(), b"perso");
        assert!(crate::manettes::retroarch_a_ses_profils(&ra));
        assert!(!ra.join("autoconfig.nouveau").exists());
    }

    #[test]
    fn le_coeur_se_lit_dans_la_ligne_de_firehouse() {
        assert_eq!(coeur_de(r#"-L "cores\snes9x_libretro.dll" -f"#).as_deref(), Some("snes9x_libretro.dll"));
        assert_eq!(coeur_de("-f"), None);
        assert_eq!(url_coeur("snes9x_libretro.dll"), "https://buildbot.libretro.com/nightly/windows/x86_64/latest/snes9x_libretro.dll.zip");
    }

    #[test]
    fn le_dossier_des_bios_de_retroarch_suit_sa_configuration() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(dossier_bios_retroarch(d.path()), d.path().join("system"));
        std::fs::write(d.path().join("retroarch.cfg"), "system_directory = \":\\bios\"\n").unwrap();
        assert_eq!(dossier_bios_retroarch(d.path()), d.path().join("bios"));
        std::fs::write(d.path().join("retroarch.cfg"), "system_directory = \"E:\\BIOS\"\n").unwrap();
        assert_eq!(dossier_bios_retroarch(d.path()), PathBuf::from("E:\\BIOS"));
    }

    #[test]
    fn le_registre_garde_un_emulateur_par_id() {
        let d = tempfile::tempdir().unwrap();
        let r = Registre::nouveau(&d.path().join("emulateurs.json"));
        let e = |v: &str| EmulateurInstalle {
            id: "pcsx2".into(),
            nom: "PCSX2".into(),
            version: Some(v.into()),
            dossier: "E:\\Emulateurs\\PCSX2".into(),
            programme: "E:\\Emulateurs\\PCSX2\\pcsx2-qt.exe".into(),
            par_frogtend: true,
            installe_le: "1".into(),
        };
        r.retenir(e("v2.8.1")).unwrap();
        r.retenir(e("v2.8.2")).unwrap();
        assert_eq!(r.tous().len(), 1);
        assert_eq!(r.un("pcsx2").unwrap().version.as_deref(), Some("v2.8.2"));
    }

    /// Essai RÉEL : télécharge les paquets 7z officiels (PCSX2, Dolphin, RetroArch) dans un dossier TEMPORAIRE et
    /// vérifie qu'ils s'installent (décompression 7z, programme trouvé). Tout est effacé à la fin.
    /// `cargo test essai_paquets_7z -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_paquets_7z() {
        let d = tempfile::tempdir().unwrap();
        for id in ["pcsx2", "dolphin", "retroarch"] {
            let f = fiche(id).unwrap();
            let p = derniere_version(f).await.unwrap();
            let paquet = d.path().join(format!("{id}.paquet"));
            let debut = std::time::Instant::now();
            let n = telecharger(&p.url, &paquet, &|_, _| {}).await.unwrap();
            let dossier = d.path().join(f.nom);
            let r = installer_paquet(f, &paquet, &dossier);
            println!(
                "{:<10} {} octets en {} s → {:?} ({} fichiers)",
                f.nom,
                n,
                debut.elapsed().as_secs(),
                r.as_ref().map(|p| p.file_name().unwrap().to_owned()),
                fichiers_de(&dossier).map(|l| l.len()).unwrap_or(0)
            );
            r.unwrap();
        }
    }

    /// Essai RÉEL : la dernière version officielle de chaque émulateur (aucun téléchargement de paquet).
    /// `cargo test essai_versions_emulateurs -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_versions_emulateurs() {
        for f in CATALOGUE {
            match derniere_version(f).await {
                Ok(p) => println!("{:<15} {:<10} {:>10} octets  {}", f.nom, p.version, p.taille.unwrap_or(0), p.url),
                Err(e) => println!("{:<15} ERREUR {e:?}", f.nom),
            }
        }
    }
}
