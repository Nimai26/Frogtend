//! Chaque profil a SES parties dans chaque émulateur (décision de Seb, 30/09) : parties, états, codes de triche,
//! cartes mémoire dans `<émulateur>\Profils\<profil>\…`, à côté de l'émulateur (jamais dans le dossier utilisateur
//! de Windows). Les dossiers de jeux des émulateurs pointent vers les emplacements de Frogtend.
//!
//! Relevé dans les sources officielles (30/09/2026) :
//! - RetroArch : `--appendconfig=FICHIER` avec `savefile_directory`, `savestate_directory`, `cheat_database_path`,
//!   `rgui_browser_directory` ;
//! - DuckStation : `[MemoryCards] Directory`, `[Folders] SaveStates / Cheats`, `[GameList] RecursivePaths` (settings.ini) ;
//! - PCSX2 : `[Folders] MemoryCards / Savestates / Cheats`, `[GameList] RecursivePaths` (inis\PCSX2.ini) ;
//! - Dolphin : un dossier utilisateur par profil (`-u`), `[General] ISOPaths / ISOPath0…` (Config\Dolphin.ini) ;
//! - PPSSPP : son dossier `memstick` (à côté du programme) devient un lien vers celui du profil.
//!
//! La manette du joueur 1 est réglée au passage si elle ne l'est pas encore (module `manettes`).
//!
//! Toute configuration modifiée est d'abord copiée dans `.frogtend-sauvegardes\` (une fois par jour et par fichier).

use crate::erreurs::{Erreur, Resultat};
use std::path::{Path, PathBuf};

/// Un nom de dossier sûr pour un profil.
pub fn dossier_du_profil(emulateur: &Path, profil: &str) -> PathBuf {
    emulateur.join("Profils").join(crate::jeux_pc::nom_de_dossier(profil))
}

/// Écrit des valeurs dans une section d'un fichier INI en gardant tout le reste. Une clé répétée (liste) est
/// remplacée par toutes les valeurs données.
pub fn ecrire_ini(texte: &str, section: &str, valeurs: &[(&str, Vec<String>)]) -> String {
    let mut lignes: Vec<String> = texte.lines().map(String::from).collect();
    let entete = format!("[{section}]");
    let debut = lignes.iter().position(|l| l.trim().eq_ignore_ascii_case(&entete));
    let debut = match debut {
        Some(i) => i,
        None => {
            if lignes.last().is_some_and(|l| !l.trim().is_empty()) {
                lignes.push(String::new());
            }
            lignes.push(entete.clone());
            lignes.len() - 1
        }
    };
    let fin = lignes[debut + 1..].iter().position(|l| l.trim_start().starts_with('[')).map_or(lignes.len(), |p| debut + 1 + p);
    let cles: Vec<String> = valeurs.iter().map(|(c, _)| c.to_lowercase()).collect();
    let mut section_lignes: Vec<String> = lignes[debut + 1..fin]
        .iter()
        .filter(|l| {
            let cle = l.split('=').next().unwrap_or("").trim().to_lowercase();
            !cles.contains(&cle)
        })
        .cloned()
        .collect();
    while section_lignes.last().is_some_and(|l| l.trim().is_empty()) {
        section_lignes.pop();
    }
    for (c, vs) in valeurs {
        for v in vs {
            section_lignes.push(format!("{c} = {v}"));
        }
    }
    if fin < lignes.len() {
        section_lignes.push(String::new());
    }
    let mut resultat = lignes[..=debut].to_vec();
    resultat.extend(section_lignes);
    resultat.extend(lignes[fin..].iter().cloned());
    resultat.join("\r\n") + "\r\n"
}

/// Copie un fichier de configuration à l'abri avant de le modifier (une fois par jour), dans le dossier de
/// l'émulateur.
pub(crate) fn mettre_a_l_abri(emulateur: &Path, fichier: &Path) -> Resultat<()> {
    if !fichier.is_file() {
        return Ok(());
    }
    let jour = crate::noyau::maintenant().parse::<u64>().unwrap_or(0) / 86_400;
    // Hors du dossier de l'émulateur, la « copie » serait le fichier lui-même : on refuse plutôt que d'écrire sans abri.
    let relatif = fichier
        .strip_prefix(emulateur)
        .map_err(|_| Erreur::Refus(format!("{} n'est pas dans le dossier de l'émulateur : pas touché.", fichier.display())))?;
    let copie = emulateur.join(".frogtend-sauvegardes").join(format!("jour-{jour}")).join(relatif);
    if !copie.exists() {
        if let Some(p) = copie.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::copy(fichier, copie)?;
    }
    Ok(())
}

/// Remplace le texte d'un fichier de configuration (copié à l'abri d'abord).
pub(crate) fn remplacer_config(emulateur: &Path, fichier: &Path, apres: &str) -> Resultat<()> {
    let avant = std::fs::read_to_string(fichier).unwrap_or_default();
    if apres != avant {
        mettre_a_l_abri(emulateur, fichier)?;
        if let Some(p) = fichier.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::write(fichier, apres)?;
    }
    Ok(())
}

pub(crate) fn modifier_ini(emulateur: &Path, fichier: &Path, section: &str, valeurs: &[(&str, Vec<String>)]) -> Resultat<()> {
    let avant = std::fs::read_to_string(fichier).unwrap_or_default();
    let apres = ecrire_ini(&avant, section, valeurs);
    if apres != avant {
        mettre_a_l_abri(emulateur, fichier)?;
        if let Some(p) = fichier.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::write(fichier, apres)?;
    }
    Ok(())
}

fn creer(dossiers: &[&Path]) -> Resultat<()> {
    for d in dossiers {
        std::fs::create_dir_all(d)?;
    }
    Ok(())
}

fn texte(p: &Path) -> String {
    p.to_string_lossy().to_string()
}

/// La manette voulue pour une partie (réglage « Commandes » du jeu).
#[derive(Clone, Debug)]
pub enum Manette {
    /// Le réglage d'office : manette standard, et la référence du système (si la manette n'a pas été retouchée).
    Auto(Option<crate::references::Reference>),
    /// Clavier et souris : Frogtend ne règle pas de manette.
    Clavier,
    /// La référence choisie pour ce jeu.
    Imposee(crate::references::Reference),
}

/// Règle la manette du joueur 1 (voir [`Manette`]).
fn regler_manette(id: &str, emulateur: &Path, utilisateur: Option<&Path>, manette: &Manette) -> Resultat<()> {
    let u = utilisateur.unwrap_or(emulateur);
    match manette {
        // RPCS3 n'a qu'une entrée par joueur : « clavier » doit l'y remettre (si le fichier est celui de Frogtend).
        Manette::Clavier if id == "rpcs3" => {
            crate::manettes::regler_rpcs3(emulateur, false, crate::manettes::PadRpcs3::Clavier)?;
        }
        Manette::Clavier => {}
        Manette::Auto(r) => {
            crate::manettes::regler(id, emulateur, utilisateur, false)?;
            if let Some(r) = r.as_ref().filter(|r| r.emulateur == id) {
                crate::references::appliquer(r, emulateur, u, false)?;
            }
        }
        Manette::Imposee(r) => {
            crate::manettes::regler(id, emulateur, utilisateur, false)?;
            if r.emulateur == id {
                crate::references::appliquer(r, emulateur, u, true)?;
            }
        }
    }
    Ok(())
}

/// Le compte RetroAchievements DU PROFIL QUI JOUE dans l'émulateur (Seb, 02/10 : gagner les succès en jouant).
/// `compte` : (nom, jeton de connexion) ; `None` : ce profil n'a pas de compte → succès coupés (on ne joue jamais avec
/// le compte d'un autre). Relevé dans les sources officielles (02/10/2026) :
/// - RetroArch (configuration.c) : `cheevos_enable`, `cheevos_username`, `cheevos_token`, `cheevos_password` — dans
///   le fichier ajouté par `--appendconfig` (celui du profil) ;
/// - PCSX2 (Achievements.cpp, QtHost.cpp) : `[Achievements] Enabled/Username` dans `inis\PCSX2.ini`, le jeton dans
///   `inis\secrets.ini` ;
/// - DuckStation (achievements.cpp) : `[Cheevos] Enabled/Username/Token/LoginTimestamp`, jeton CHIFFRÉ par DuckStation
///   (refusé en clair) → la personne se connecte UNE fois dans DuckStation ; Frogtend garde ensuite sa connexion par
///   profil (`<émulateur>\Profils\<profil>\cheevos-duckstation.json`) et la remet à chaque partie.
pub fn regler_succes(id: &str, emulateur: &Path, profil: &str, compte: Option<(&str, &str)>) -> Resultat<()> {
    let p = dossier_du_profil(emulateur, profil);
    match id {
        "retroarch" => {
            let fichier = p.join("frogtend.cfg");
            let mut cfg = std::fs::read_to_string(&fichier).unwrap_or_default();
            let (actif, nom, jeton) = match compte {
                Some((n, j)) => ("true", n, j),
                None => ("false", "", ""),
            };
            cfg.push_str(&format!(
                "cheevos_enable = \"{actif}\"\ncheevos_username = \"{nom}\"\ncheevos_token = \"{jeton}\"\ncheevos_password = \"\"\n"
            ));
            creer(&[&p])?;
            std::fs::write(&fichier, cfg)?;
        }
        "pcsx2" => {
            let ini = emulateur.join("inis").join("PCSX2.ini");
            let (actif, nom, jeton) = match compte {
                Some((n, j)) => ("true", n, j),
                None => ("false", "", ""),
            };
            modifier_ini(emulateur, &ini, "Achievements", &[("Enabled", vec![actif.into()]), ("Username", vec![nom.into()])])?;
            // Le jeton va dans le fichier de secrets de PCSX2 (jamais copié dans les sauvegardes de configuration).
            let secrets = emulateur.join("inis").join("secrets.ini");
            let avant = std::fs::read_to_string(&secrets).unwrap_or_default();
            let apres = ecrire_ini(&avant, "Achievements", &[("Token", vec![jeton.into()])]);
            if apres != avant {
                // La connexion faite dans PCSX2 AVANT Frogtend (pas de marque « .cheevos-pcsx2 ») est copiée à l'abri,
                // dans le dossier de l'émulateur (jamais sauvegardé chez Firehouse), avant d'être remplacée. Celles
                // posées ensuite par Frogtend ne sont pas copiées : un profil qui oublie son compte ne doit rien laisser.
                if !emulateur.join("Profils").join(".cheevos-pcsx2").exists() {
                    mettre_a_l_abri(emulateur, &secrets)?;
                }
                std::fs::create_dir_all(emulateur.join("inis"))?;
                std::fs::write(&secrets, apres)?;
            }
            // Le profil dont la connexion est en place (pour l'oublier si ce profil oublie son compte).
            creer(&[&emulateur.join("Profils")])?;
            std::fs::write(emulateur.join("Profils").join(".cheevos-pcsx2"), profil)?;
        }
        "duckstation" => {
            let ini = emulateur.join("settings.ini");
            let texte = std::fs::read_to_string(&ini).unwrap_or_default();
            let lire = |cle: &str| crate::manettes::lire_ini(&texte, "Cheevos", cle).into_iter().next().unwrap_or_default();
            let (nom_actuel, jeton_actuel, quand) = (lire("Username"), lire("Token"), lire("LoginTimestamp"));
            // 1. La connexion en place appartient au dernier profil qui a joué : on la lui garde.
            let dernier = emulateur.join("Profils").join(".cheevos-dernier");
            if let Ok(d) = std::fs::read_to_string(&dernier) {
                let gardee = emulateur.join("Profils").join(d.trim()).join("cheevos-duckstation.json");
                let a_lui = std::fs::read(&gardee).ok().and_then(|o| serde_json::from_slice::<serde_json::Value>(&o).ok());
                let meme_nom = a_lui.as_ref().is_none_or(|v| v["Username"].as_str() == Some(nom_actuel.as_str()));
                if !d.trim().is_empty() && !nom_actuel.is_empty() && !jeton_actuel.is_empty() && meme_nom {
                    std::fs::create_dir_all(gardee.parent().unwrap_or(emulateur))?;
                    std::fs::write(&gardee, serde_json::json!({"Username": nom_actuel, "Token": jeton_actuel, "LoginTimestamp": quand}).to_string())?;
                }
            }
            // 2. Celle de ce profil.
            let a_moi = std::fs::read(p.join("cheevos-duckstation.json")).ok().and_then(|o| serde_json::from_slice::<serde_json::Value>(&o).ok());
            let valeurs: Vec<(&str, Vec<String>)> = match compte {
                None => vec![("Enabled", vec!["false".into()]), ("Username", vec![String::new()]), ("Token", vec![String::new()]), ("LoginTimestamp", vec![String::new()])],
                Some((n, _)) => match a_moi.filter(|v| v["Username"].as_str() == Some(n)) {
                    Some(v) => vec![
                        ("Enabled", vec!["true".into()]),
                        ("Username", vec![n.into()]),
                        ("Token", vec![v["Token"].as_str().unwrap_or("").into()]),
                        ("LoginTimestamp", vec![v["LoginTimestamp"].as_str().unwrap_or("").into()]),
                    ],
                    // Pas encore connecté dans DuckStation : il demandera la connexion à la première partie.
                    None => vec![("Enabled", vec!["true".into()]), ("Username", vec![n.into()]), ("Token", vec![String::new()]), ("LoginTimestamp", vec![String::new()])],
                },
            };
            modifier_ini(emulateur, &ini, "Cheevos", &valeurs)?;
            creer(&[&p])?;
            std::fs::write(&dernier, profil)?;
        }
        _ => {}
    }
    Ok(())
}

/// Les fichiers d'un profil d'émulateur qui portent une connexion RetroAchievements (jeton) : ils restent sur ce PC et
/// ne partent JAMAIS dans la sauvegarde chez Firehouse. `relatif` : chemin sous `<émulateur>\Profils\<profil>`.
pub fn porte_un_secret(relatif: &str) -> bool {
    let r = relatif.replace('\\', "/").to_lowercase();
    r == "frogtend.cfg" || r == "cheevos-duckstation.json" || r.ends_with("config/retroachievements.ini")
}

/// Un profil oublie son compte RetroAchievements (ou est supprimé) : sa connexion est retirée de cet émulateur.
/// Ne touche que ce que Frogtend a écrit pour CE profil (fichiers de son dossier, ou connexion posée pour lui).
pub fn oublier_succes(emulateur: &Path, profil: &str) -> Resultat<()> {
    let p = dossier_du_profil(emulateur, profil);
    // RetroArch : le fichier du profil, régénéré à chaque partie ; on y retire les lignes « cheevos_ ».
    let cfg = p.join("frogtend.cfg");
    if let Ok(texte) = std::fs::read_to_string(&cfg) {
        let garde: String = texte.lines().filter(|l| !l.trim_start().starts_with("cheevos_")).map(|l| format!("{l}\n")).collect();
        std::fs::write(&cfg, garde)?;
    }
    // DuckStation : la connexion gardée pour ce profil (fichier écrit par Frogtend).
    let gardee = p.join("cheevos-duckstation.json");
    if gardee.is_file() {
        std::fs::remove_file(&gardee)?;
    }
    let dernier = |nom: &str| std::fs::read_to_string(emulateur.join("Profils").join(nom)).is_ok_and(|d| d.trim() == profil);
    let vide = |cles: &[&'static str]| cles.iter().map(|c| (*c, vec![String::new()])).collect::<Vec<_>>();
    // DuckStation : la connexion en place, si c'est celle de ce profil (dernier à avoir joué).
    // Écrit directement (sans copie à l'abri, qui garderait le jeton) : seule la section [Cheevos] change.
    let ini = emulateur.join("settings.ini");
    if dernier(".cheevos-dernier") && ini.is_file() {
        let mut v = vide(&["Username", "Token", "LoginTimestamp"]);
        v.push(("Enabled", vec!["false".into()]));
        let avant = std::fs::read_to_string(&ini)?;
        std::fs::write(&ini, ecrire_ini(&avant, "Cheevos", &v))?;
    }
    // PCSX2 : la connexion en place, si Frogtend l'a posée pour ce profil.
    let secrets = emulateur.join("inis").join("secrets.ini");
    if dernier(".cheevos-pcsx2") && secrets.is_file() {
        let avant = std::fs::read_to_string(&secrets)?;
        std::fs::write(&secrets, ecrire_ini(&avant, "Achievements", &vide(&["Token"])))?;
        let ini = emulateur.join("inis").join("PCSX2.ini");
        if ini.is_file() {
            modifier_ini(emulateur, &ini, "Achievements", &[("Enabled", vec!["false".into()]), ("Username", vec![String::new()])])?;
        }
    }
    Ok(())
}

/// Prépare l'émulateur pour ce profil avant une partie. Rend les arguments à mettre AVANT sa ligne de commande.
/// `jeux` : les emplacements de Frogtend pour ce système.
pub fn preparer(id: &str, emulateur: &Path, profil: &str, jeux: &[String], manette: &Manette) -> Resultat<Vec<String>> {
    let p = dossier_du_profil(emulateur, profil);
    let args = preparer_dossiers(id, emulateur, &p, jeux, manette)?;
    // Le menu universel en jeu : pause à la perte du premier plan, touches F13–F16, commandes de RetroArch.
    crate::pilotage::preparer(id, emulateur, Some(&p.join("User")))?;
    Ok(args)
}

fn preparer_dossiers(id: &str, emulateur: &Path, p: &Path, jeux: &[String], manette: &Manette) -> Resultat<Vec<String>> {
    match id {
        "retroarch" => {
            let (saves, states, triches) = (p.join("saves"), p.join("states"), p.join("cheats"));
            creer(&[&saves, &states, &triches])?;
            let mut cfg = format!(
                "savefile_directory = \"{}\"\nsavestate_directory = \"{}\"\ncheat_database_path = \"{}\"\n",
                texte(&saves),
                texte(&states),
                texte(&triches)
            );
            if let Some(j) = jeux.first() {
                cfg.push_str(&format!("rgui_browser_directory = \"{j}\"\n"));
            }
            cfg.push_str(&crate::pilotage::lignes_retroarch());
            if !matches!(manette, Manette::Clavier) {
                cfg.push_str(&crate::manettes::lignes_retroarch(emulateur));
            }
            let fichier = p.join("frogtend.cfg");
            std::fs::write(&fichier, cfg)?;
            Ok(vec![format!("--appendconfig={}", texte(&fichier))])
        }
        "duckstation" => {
            let (cartes, etats, triches) = (p.join("memcards"), p.join("savestates"), p.join("cheats"));
            creer(&[&cartes, &etats, &triches])?;
            let ini = emulateur.join("settings.ini");
            modifier_ini(emulateur, &ini, "MemoryCards", &[("Directory", vec![texte(&cartes)])])?;
            modifier_ini(emulateur, &ini, "Folders", &[("SaveStates", vec![texte(&etats)]), ("Cheats", vec![texte(&triches)])])?;
            modifier_ini(emulateur, &ini, "GameList", &[("RecursivePaths", jeux.to_vec())])?;
            regler_manette(id, emulateur, None, manette)?;
            Ok(vec![])
        }
        "pcsx2" => {
            let (cartes, etats, triches) = (p.join("memcards"), p.join("sstates"), p.join("cheats"));
            creer(&[&cartes, &etats, &triches])?;
            let ini = emulateur.join("inis").join("PCSX2.ini");
            modifier_ini(
                emulateur,
                &ini,
                "Folders",
                &[
                    ("MemoryCards", vec![texte(&cartes)]),
                    ("Savestates", vec![texte(&etats)]),
                    ("Cheats", vec![texte(&triches)]),
                ],
            )?;
            modifier_ini(emulateur, &ini, "GameList", &[("RecursivePaths", jeux.to_vec())])?;
            regler_manette(id, emulateur, None, manette)?;
            Ok(vec![])
        }
        "dolphin" => {
            let utilisateur = p.join("User");
            let ini = utilisateur.join("Config").join("Dolphin.ini");
            let mut v: Vec<(String, Vec<String>)> = vec![("ISOPaths".into(), vec![jeux.len().to_string()])];
            for (i, j) in jeux.iter().enumerate() {
                v.push((format!("ISOPath{i}"), vec![j.clone()]));
            }
            let refs: Vec<(&str, Vec<String>)> = v.iter().map(|(c, vs)| (c.as_str(), vs.clone())).collect();
            modifier_ini(emulateur, &ini, "General", &refs)?;
            regler_manette(id, emulateur, Some(&utilisateur), manette)?;
            Ok(vec!["-u".into(), texte(&utilisateur)])
        }
        "ppsspp" => {
            relier_memstick(emulateur, &p.join("memstick"))?;
            Ok(vec![])
        }
        // Xenia : les parties dans le dossier du profil (\`--content_root\`, xenia_main.cc).
        "xenia" => {
            let contenu = p.join("content");
            creer(&[&contenu])?;
            Ok(vec![format!("--content_root={}", texte(&contenu))])
        }
        // xemu : les parties sont DANS le disque dur virtuel de la Xbox → un disque et un fichier de réglages par
        // profil (\`-config_path\`), copiés au premier lancement depuis ceux réglés dans xemu.
        "xemu" => {
            creer(&[p])?;
            let cfg = p.join("xemu.toml");
            let principal = std::fs::read_to_string(emulateur.join("xemu.toml")).unwrap_or_default();
            if !cfg.is_file() {
                std::fs::write(&cfg, &principal)?;
            }
            let disque = p.join("xbox_hdd.qcow2");
            if !disque.is_file() {
                if let Some(source) = valeur_toml(&principal, "sys.files", "hdd_path").map(std::path::PathBuf::from) {
                    if source.is_file() {
                        std::fs::copy(&source, &disque)?;
                    }
                }
            }
            let mut v = vec![];
            if disque.is_file() {
                v.push(("hdd_path", vec![format!("'{}'", texte(&disque))]));
            }
            if !v.is_empty() {
                modifier_ini(emulateur, &cfg, "sys.files", &v)?;
            }
            modifier_ini(emulateur, &cfg, "display.window", &[("fullscreen_on_startup", vec!["true".into()])])?;
            Ok(vec!["-config_path".into(), texte(&cfg)])
        }
        // Eden (Switch) : l'utilisateur Switch qui porte le nom du profil (`-u <nom>`, core/launch_params.cpp) ; à
        // créer une fois dans Eden. Absent : Eden garde son utilisateur courant.
        "eden" => Ok(vec!["-u".into(), profil_de(p).to_string()]),
        // RPCS3 : un compte RPCS3 par profil (\`--user-id\`, rpcs3.cpp), donc ses propres parties.
        "rpcs3" => {
            // La manette du joueur 1, à CHAQUE partie (oubli de la 0.44.0, vu par Seb le 04/10 : rien n'était branché).
            // Un fichier qui ne peut pas être écrit (disque en lecture seule…) n'empêche jamais de jouer : RPCS3 garde
            // alors son réglage d'avant.
            let _ = regler_manette(id, emulateur, None, manette);
            Ok(vec!["--user-id".into(), compte_rpcs3(emulateur, profil_de(p))?])
        }
        // Azahar : NAND et carte SD du profil (\`[Data Storage]\` de qt-config.ini, configuration/config.cpp).
        "azahar" => {
            let (nand, sdmc) = (p.join("nand"), p.join("sdmc"));
            creer(&[&nand, &sdmc])?;
            let ini = emulateur.join("user").join("config").join("qt-config.ini");
            // QSettings : « \\ » est un échappement, on écrit des « / ».
            let barre = |c: &Path| format!("{}/", texte(c).replace('\\', "/"));
            modifier_ini(
                emulateur,
                &ini,
                "Data%20Storage",
                &[
                    ("use_custom_storage\\default", vec!["false".into()]),
                    ("use_custom_storage", vec!["true".into()]),
                    ("nand_directory", vec![barre(&nand)]),
                    ("sdmc_directory", vec![barre(&sdmc)]),
                ],
            )?;
            Ok(vec![])
        }
        _ => Ok(vec![]),
    }
}

/// Le dossier « données » d'un émulateur pour un profil (contrat 13 : `base = donnees`), là où Frogtend range ses
/// parties et ses triches pour ce profil.
pub fn dossier_donnees(id: &str, emulateur: &Path, profil: &str) -> PathBuf {
    let p = dossier_du_profil(emulateur, profil);
    match id {
        "retroarch" | "duckstation" | "pcsx2" => p,
        "dolphin" => p.join("User"),
        "ppsspp" => emulateur.join("memstick"),
        _ => emulateur.to_path_buf(),
    }
}

/// Où poser un fichier de triche : `base` (`donnees` | `programme`), `dossier` relatif (« / »), `nom_fichier`.
/// Refuse tout chemin qui sortirait du dossier de l'émulateur.
pub fn place_triche(id: &str, emulateur: &Path, profil: &str, base: &str, dossier: &str, nom_fichier: &str) -> Resultat<PathBuf> {
    // Un chemin enraciné (« \Windows ») remplacerait tout le chemin au `join` : refusé comme « .. » et « C: ».
    let sur = |s: &str| !s.starts_with(['/', '\\']) && !s.split(['/', '\\']).any(|x| x == ".." || x.contains(':'));
    if nom_fichier.is_empty() || nom_fichier.contains(['/', '\\', ':']) || nom_fichier.starts_with('.') || !sur(dossier) {
        return Err(Erreur::Refus("Emplacement de triche douteux : refusé.".into()));
    }
    let racine = if base == "programme" { emulateur.to_path_buf() } else { dossier_donnees(id, emulateur, profil) };
    let mut c = racine.clone();
    for morceau in dossier.split(['/', '\\']).filter(|m| !m.is_empty()) {
        c = c.join(morceau);
    }
    let c = c.join(nom_fichier);
    if !c.starts_with(&racine) {
        return Err(Erreur::Refus("Emplacement de triche douteux : refusé.".into()));
    }
    Ok(c)
}

/// Le nom du profil d'après son dossier (`…\Profils\<nom>`).
fn profil_de(p: &Path) -> &str {
    p.file_name().and_then(|n| n.to_str()).unwrap_or("Frogtend")
}

/// Une valeur d'un fichier TOML simple (`[section]` puis `cle = 'valeur'` ou `"valeur"`).
pub fn valeur_toml(texte: &str, section: &str, cle: &str) -> Option<String> {
    let v = crate::manettes::lire_ini(texte, section, cle).into_iter().next()?;
    let v = v.trim();
    let v = v.strip_prefix('\'').and_then(|x| x.strip_suffix('\'')).or_else(|| v.strip_prefix('"').and_then(|x| x.strip_suffix('"'))).unwrap_or(v);
    Some(v.replace("\\\\", "\\")).filter(|s| !s.is_empty())
}

/// La version du micrologiciel PS3 installé dans un RPCS3 (« 4.93 »), ou `None` s'il n'y en a pas. Relevé dans le code
/// de RPCS3 (03/10) : il est « manquant » sans `dev_flash\sys\external\liblv2.sprx` ; la version est dans
/// `dev_flash\vsh\etc\version.txt`, entre le 1er et le 2e « : » (« release:04.9300: » → 4.93).
pub fn micrologiciel_rpcs3(emulateur: &Path) -> Option<String> {
    let flash = emulateur.join("dev_flash");
    if !flash.join("sys").join("external").join("liblv2.sprx").is_file() {
        return None;
    }
    let texte = std::fs::read_to_string(flash.join("vsh").join("etc").join("version.txt")).unwrap_or_default();
    let brut = texte.split(':').nth(1).unwrap_or("").trim();
    let Some((majeur, mineur)) = brut.split_once('.') else { return Some("inconnue".into()) };
    let majeur = majeur.trim_start_matches('0');
    let mineur: String = mineur.chars().take(2).collect();
    Some(format!("{}.{mineur}", if majeur.is_empty() { "0" } else { majeur }))
}

/// Le compte RPCS3 d'un profil s'il existe déjà, sans rien créer.
pub fn compte_rpcs3_existant(emulateur: &Path, profil: &str) -> Option<String> {
    std::fs::read_dir(emulateur.join("dev_hdd0").join("home")).ok()?.flatten().find_map(|e| {
        let id = e.file_name().to_string_lossy().to_string();
        let ok = id.len() == 8 && id.chars().all(|c| c.is_ascii_digit());
        (ok && std::fs::read_to_string(e.path().join("localusername")).is_ok_and(|n| n.trim() == profil)).then_some(id)
    })
}

/// Le compte RPCS3 d'un profil (`dev_hdd0\home\<8 chiffres>\localusername` = nom du profil) : retrouvé, ou créé
/// au premier numéro libre à partir de 00000002 (le 00000001 reste celui de RPCS3).
pub fn compte_rpcs3(emulateur: &Path, profil: &str) -> Resultat<String> {
    let maison = emulateur.join("dev_hdd0").join("home");
    std::fs::create_dir_all(&maison)?;
    let mut pris = std::collections::BTreeSet::new();
    for e in std::fs::read_dir(&maison)?.flatten() {
        let id = e.file_name().to_string_lossy().to_string();
        if id.len() != 8 || !id.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if std::fs::read_to_string(e.path().join("localusername")).is_ok_and(|n| n.trim() == profil) {
            return Ok(id);
        }
        pris.insert(id.parse::<u32>().unwrap_or(0));
    }
    let n = (2..99_999_999u32).find(|n| !pris.contains(n)).unwrap_or(2);
    let id = format!("{n:08}");
    let d = maison.join(&id);
    std::fs::create_dir_all(&d)?;
    std::fs::write(d.join("localusername"), profil)?;
    Ok(id)
}

/// PPSSPP garde tout dans `memstick\` à côté du programme : ce dossier devient un lien (jonction Windows, sans droits
/// administrateur) vers celui du profil. Un vrai dossier `memstick` déjà rempli (avant Frogtend) est renommé en
/// `memstick (avant Frogtend)` : rien n'est effacé.
pub fn relier_memstick(emulateur: &Path, cible: &Path) -> Resultat<()> {
    std::fs::create_dir_all(cible)?;
    let lien = emulateur.join("memstick");
    if let Ok(m) = std::fs::symlink_metadata(&lien) {
        let est_lien = m.file_type().is_symlink() || std::fs::read_link(&lien).is_ok();
        if est_lien {
            std::fs::remove_dir(&lien)?; // retire le LIEN seulement, pas ce vers quoi il pointe
        } else {
            let de_cote = emulateur.join("memstick (avant Frogtend)");
            if de_cote.exists() {
                return Err(Erreur::Refus(format!(
                    "{} et {} existent tous les deux : range-les avant de jouer.",
                    lien.display(),
                    de_cote.display()
                )));
            }
            std::fs::rename(&lien, &de_cote)?;
        }
    }
    let s = crate::lancement::outil_sans_fenetre("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&lien)
        .arg(cible)
        .output()
        .map_err(|e| Erreur::Disque(format!("Impossible de créer le lien de PPSSPP ({e}).")))?;
    if !s.status.success() || !lien.exists() {
        return Err(Erreur::Disque("Impossible de créer le lien du dossier memstick de PPSSPP.".into()));
    }
    Ok(())
}

/// Les dossiers que ces émulateurs créent dans le dossier utilisateur de Windows quand ils NE sont PAS portables :
/// s'ils existent, l'« effet pieuvre » est là (installation ancienne, à la main…). On le signale, on n'efface rien.
pub fn traces_hors_du_dossier(id: &str) -> Vec<PathBuf> {
    let env = |v: &str| std::env::var(v).map(PathBuf::from).ok();
    let documents = env("USERPROFILE").map(|u| u.join("Documents"));
    let candidats: Vec<Option<PathBuf>> = match id {
        "retroarch" => vec![env("APPDATA").map(|a| a.join("RetroArch"))],
        "duckstation" => vec![documents.clone().map(|d| d.join("DuckStation")), env("LOCALAPPDATA").map(|a| a.join("DuckStation"))],
        "pcsx2" => vec![documents.clone().map(|d| d.join("PCSX2"))],
        "dolphin" => vec![documents.clone().map(|d| d.join("Dolphin Emulator")), env("APPDATA").map(|a| a.join("Dolphin Emulator"))],
        "ppsspp" => vec![documents.clone().map(|d| d.join("PPSSPP"))],
        "xenia" => vec![documents.clone().map(|d| d.join("Xenia"))],
        "xemu" => vec![env("APPDATA").map(|a| a.join("xemu"))],
        "cemu" => vec![env("APPDATA").map(|a| a.join("Cemu"))],
        "azahar" => vec![env("APPDATA").map(|a| a.join("Azahar"))],
        "vita3k" => vec![env("APPDATA").map(|a| a.join("Vita3K"))],
        "eden" => vec![env("APPDATA").map(|a| a.join("eden"))],
        "dosbox-staging" => vec![env("LOCALAPPDATA").map(|a| a.join("DOSBox"))],
        _ => vec![],
    };
    candidats.into_iter().flatten().filter(|p| p.is_dir()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_ini_est_modifie_sans_perdre_le_reste() {
        let avant = "[Main]\r\nLanguage = fr\r\n\r\n[Folders]\r\nSaveStates = savestates\r\nCovers = covers\r\n\r\n[GameList]\r\nRecursivePaths = C:\\vieux\r\nRecursivePaths = D:\\vieux\r\n";
        let a = ecrire_ini(avant, "Folders", &[("SaveStates", vec!["E:\\P\\etats".into()])]);
        assert!(a.contains("SaveStates = E:\\P\\etats"));
        assert!(a.contains("Covers = covers") && a.contains("Language = fr"));
        let b = ecrire_ini(&a, "GameList", &[("RecursivePaths", vec!["E:\\Jeux".into()])]);
        assert_eq!(b.matches("RecursivePaths").count(), 1, "la liste est remplacée");
        assert!(b.contains("RecursivePaths = E:\\Jeux"));
        let c = ecrire_ini("", "MemoryCards", &[("Directory", vec!["E:\\m".into()])]);
        assert_eq!(c, "[MemoryCards]\r\nDirectory = E:\\m\r\n");
    }

    #[test]
    fn le_micrologiciel_de_rpcs3_se_lit() {
        let d = tempfile::tempdir().unwrap();
        let e = d.path();
        assert_eq!(micrologiciel_rpcs3(e), None, "rien d'installé");
        std::fs::create_dir_all(e.join("dev_flash/vsh/etc")).unwrap();
        std::fs::write(e.join("dev_flash/vsh/etc/version.txt"), "release:04.9100:\nbuild:68392,20231207:tetsu@tetsu-linux17\n").unwrap();
        assert_eq!(micrologiciel_rpcs3(e), None, "sans liblv2.sprx, RPCS3 le dit manquant");
        std::fs::create_dir_all(e.join("dev_flash/sys/external")).unwrap();
        std::fs::write(e.join("dev_flash/sys/external/liblv2.sprx"), b"x").unwrap();
        assert_eq!(micrologiciel_rpcs3(e).as_deref(), Some("4.91"));
        std::fs::write(e.join("dev_flash/vsh/etc/version.txt"), "release:04.9300:\n").unwrap();
        assert_eq!(micrologiciel_rpcs3(e).as_deref(), Some("4.93"));
    }

    #[test]
    fn chaque_profil_joue_avec_son_compte_retroachievements() {
        let d = tempfile::tempdir().unwrap();
        let e = d.path();
        // RetroArch : le compte dans le fichier du profil ; sans compte, succès coupés et rien de l'autre profil.
        preparer("retroarch", e, "Seb", &[], &Manette::Clavier).unwrap();
        regler_succes("retroarch", e, "Seb", Some(("SebRA", "JETON1"))).unwrap();
        let cfg = std::fs::read_to_string(dossier_du_profil(e, "Seb").join("frogtend.cfg")).unwrap();
        assert!(cfg.contains("cheevos_enable = \"true\"") && cfg.contains("cheevos_token = \"JETON1\""));
        preparer("retroarch", e, "Lea", &[], &Manette::Clavier).unwrap();
        regler_succes("retroarch", e, "Lea", None).unwrap();
        let cfg = std::fs::read_to_string(dossier_du_profil(e, "Lea").join("frogtend.cfg")).unwrap();
        assert!(cfg.contains("cheevos_enable = \"false\"") && cfg.contains("cheevos_token = \"\"") && !cfg.contains("JETON1"));

        // PCSX2 : nom dans PCSX2.ini, jeton dans secrets.ini.
        regler_succes("pcsx2", e, "Seb", Some(("SebRA", "JETON1"))).unwrap();
        let ini = std::fs::read_to_string(e.join("inis").join("PCSX2.ini")).unwrap();
        assert!(ini.contains("Username = SebRA") || ini.contains("Username=SebRA"));
        assert!(!ini.contains("JETON1"), "le jeton n'est pas dans PCSX2.ini");
        assert!(std::fs::read_to_string(e.join("inis").join("secrets.ini")).unwrap().contains("JETON1"));
        regler_succes("pcsx2", e, "Lea", None).unwrap();
        assert!(!std::fs::read_to_string(e.join("inis").join("secrets.ini")).unwrap().contains("JETON1"));

        // DuckStation : Seb s'est connecté dans DuckStation (jeton chiffré par lui) ; Léa joue ; Seb retrouve SA connexion.
        regler_succes("duckstation", e, "Seb", Some(("SebRA", ""))).unwrap();
        let ini = e.join("settings.ini");
        let t = std::fs::read_to_string(&ini).unwrap();
        std::fs::write(&ini, ecrire_ini(&t, "Cheevos", &[("Token", vec!["CHIFFRE-PAR-DUCKSTATION".into()]), ("LoginTimestamp", vec!["1790000000".into()])])).unwrap();
        regler_succes("duckstation", e, "Lea", None).unwrap();
        let t = std::fs::read_to_string(&ini).unwrap();
        assert!(!t.contains("CHIFFRE-PAR-DUCKSTATION"), "Léa ne joue pas avec le compte de Seb");
        regler_succes("duckstation", e, "Seb", Some(("SebRA", ""))).unwrap();
        let t = std::fs::read_to_string(&ini).unwrap();
        assert!(t.contains("CHIFFRE-PAR-DUCKSTATION") && t.contains("1790000000"), "Seb retrouve sa connexion");
    }

    #[test]
    fn la_connexion_faite_avant_frogtend_est_gardee_et_un_compte_oublie_ne_laisse_rien() {
        let d = tempfile::tempdir().unwrap();
        let e = d.path();
        // Seb s'était connecté dans PCSX2 avant Frogtend : sa connexion est copiée à l'abri avant d'être remplacée.
        std::fs::create_dir_all(e.join("inis")).unwrap();
        std::fs::write(e.join("inis").join("secrets.ini"), "[Achievements]\r\nToken = AVANT-FROGTEND\r\n").unwrap();
        regler_succes("pcsx2", e, "Lea", Some(("LeaRA", "JETON-LEA"))).unwrap();
        let abri = std::fs::read_dir(e.join(".frogtend-sauvegardes")).unwrap().next().unwrap().unwrap().path();
        assert!(std::fs::read_to_string(abri.join("inis").join("secrets.ini")).unwrap().contains("AVANT-FROGTEND"));
        // Le jeton posé par Frogtend n'est jamais copié à l'abri.
        regler_succes("pcsx2", e, "Seb", Some(("SebRA", "JETON-SEB"))).unwrap();
        assert!(!std::fs::read_to_string(abri.join("inis").join("secrets.ini")).unwrap().contains("JETON-LEA"));

        // Seb oublie son compte : sa connexion part de PCSX2, de RetroArch et de DuckStation.
        preparer("retroarch", e, "Seb", &[], &Manette::Clavier).unwrap();
        regler_succes("retroarch", e, "Seb", Some(("SebRA", "JETON-SEB"))).unwrap();
        regler_succes("duckstation", e, "Seb", Some(("SebRA", ""))).unwrap();
        let ini = e.join("settings.ini");
        let t = std::fs::read_to_string(&ini).unwrap();
        std::fs::write(&ini, ecrire_ini(&t, "Cheevos", &[("Token", vec!["CHIFFRE-SEB".into()])])).unwrap();
        regler_succes("duckstation", e, "Lea", None).unwrap(); // garde la connexion de Seb dans son dossier
        let p = dossier_du_profil(e, "Seb");
        assert!(p.join("cheevos-duckstation.json").is_file());
        regler_succes("duckstation", e, "Seb", Some(("SebRA", ""))).unwrap();
        oublier_succes(e, "Seb").unwrap();
        assert!(!std::fs::read_to_string(e.join("inis").join("secrets.ini")).unwrap().contains("JETON-SEB"));
        assert!(!std::fs::read_to_string(p.join("frogtend.cfg")).unwrap().contains("JETON-SEB"));
        assert!(!p.join("cheevos-duckstation.json").exists());
        assert!(!std::fs::read_to_string(&ini).unwrap().contains("CHIFFRE-SEB"));

        // La connexion d'un autre profil n'est pas touchée quand Léa oublie le sien.
        regler_succes("pcsx2", e, "Seb", Some(("SebRA", "JETON-SEB"))).unwrap();
        oublier_succes(e, "Lea").unwrap();
        assert!(std::fs::read_to_string(e.join("inis").join("secrets.ini")).unwrap().contains("JETON-SEB"));
    }

    #[test]
    fn la_sauvegarde_n_emporte_pas_les_connexions() {
        assert!(porte_un_secret("frogtend.cfg"));
        assert!(porte_un_secret("cheevos-duckstation.json"));
        assert!(porte_un_secret("User\\Config\\RetroAchievements.ini"));
        assert!(!porte_un_secret("saves/Mario.srm"));
        assert!(!porte_un_secret("memcards/Mcd001.ps2"));
    }

    #[test]
    fn retroarch_recoit_un_fichier_de_reglages_par_profil() {
        let d = tempfile::tempdir().unwrap();
        let args = preparer("retroarch", d.path(), "Sebastien", &["E:\\Jeux\\SNES".into()], &Manette::Auto(None)).unwrap();
        let fichier = d.path().join("Profils").join("Sebastien").join("frogtend.cfg");
        assert_eq!(args, vec![format!("--appendconfig={}", fichier.display())]);
        let cfg = std::fs::read_to_string(&fichier).unwrap();
        assert!(cfg.contains("savefile_directory") && cfg.contains("Sebastien"));
        assert!(cfg.contains("rgui_browser_directory = \"E:\\Jeux\\SNES\""));
        assert!(d.path().join("Profils").join("Sebastien").join("saves").is_dir());
    }

    #[test]
    fn deux_profils_ont_des_parties_separees_dans_duckstation_et_la_configuration_est_mise_a_l_abri() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("settings.ini"), "[Main]\r\nSettingsVersion = 3\r\n").unwrap();
        preparer("duckstation", d.path(), "Seb", &["E:\\Jeux\\PS1".into()], &Manette::Auto(None)).unwrap();
        let ini = std::fs::read_to_string(d.path().join("settings.ini")).unwrap();
        assert!(ini.contains("SettingsVersion = 3"), "le reste est gardé");
        assert!(ini.contains(&format!("Directory = {}", d.path().join("Profils").join("Seb").join("memcards").display())));
        assert!(ini.contains("RecursivePaths = E:\\Jeux\\PS1"));
        preparer("duckstation", d.path(), "Léa", &["E:\\Jeux\\PS1".into()], &Manette::Auto(None)).unwrap();
        let ini = std::fs::read_to_string(d.path().join("settings.ini")).unwrap();
        assert!(ini.contains("Léa") && !ini.contains("\\Seb\\"));
        // La configuration d'origine a été copiée à l'abri.
        let abri = std::fs::read_dir(d.path().join(".frogtend-sauvegardes")).unwrap().next().unwrap().unwrap().path();
        assert!(std::fs::read_to_string(abri.join("settings.ini")).unwrap().contains("SettingsVersion = 3"));
    }

    #[test]
    fn pcsx2_et_dolphin() {
        let d = tempfile::tempdir().unwrap();
        preparer("pcsx2", d.path(), "Seb", &["E:\\Jeux\\PS2".into()], &Manette::Auto(None)).unwrap();
        let ini = std::fs::read_to_string(d.path().join("inis").join("PCSX2.ini")).unwrap();
        assert!(ini.contains("[Folders]") && ini.contains("MemoryCards = ") && ini.contains("RecursivePaths = E:\\Jeux\\PS2"));

        let args = preparer("dolphin", d.path(), "Seb", &["E:\\Jeux\\GC".into(), "F:\\GC".into()], &Manette::Auto(None)).unwrap();
        assert_eq!(args[0], "-u");
        let ini = std::fs::read_to_string(PathBuf::from(&args[1]).join("Config").join("Dolphin.ini")).unwrap();
        assert!(ini.contains("ISOPaths = 2") && ini.contains("ISOPath1 = F:\\GC"));
    }

    #[test]
    fn les_consoles_recentes_ont_des_parties_par_profil() {
        let d = tempfile::tempdir().unwrap();
        let racine = d.path();
        // Xenia : dossier des parties du profil.
        let a = preparer("xenia", racine, "Seb", &[], &Manette::Auto(None)).unwrap();
        assert_eq!(a, vec![format!("--content_root={}", racine.join("Profils").join("Seb").join("content").display())]);

        // RPCS3 : la manette est réglée à CHAQUE partie (manette branchée, sinon la dernière vue, sinon Xbox), « clavier » y remet le
        // clavier (bug de la 0.44.0 : ce réglage n'était appelé qu'à l'installation de RPCS3).
        let pad = racine.join("config/input_configs/global/Default.yml");
        preparer("rpcs3", racine, "Seb", &[], &Manette::Auto(None)).unwrap();
        assert!(std::fs::read_to_string(&pad).unwrap().contains("Player 1 Input"), "réglée dès la 1re partie");
        preparer("rpcs3", racine, "Seb", &[], &Manette::Clavier).unwrap();
        assert!(std::fs::read_to_string(&pad).unwrap().contains("Handler: \"Keyboard\""));
        std::fs::remove_file(&pad).unwrap();
        std::fs::remove_dir_all(racine.join("dev_hdd0")).ok();
        // RPCS3 : un compte par profil, retrouvé ensuite ; le 00000001 n'est jamais pris.
        std::fs::create_dir_all(racine.join("dev_hdd0/home/00000001")).unwrap();
        let s = preparer("rpcs3", racine, "Seb", &[], &Manette::Auto(None)).unwrap();
        let l = preparer("rpcs3", racine, "Léa", &[], &Manette::Auto(None)).unwrap();
        assert_eq!(s, vec!["--user-id", "00000002"]);
        assert_eq!(l, vec!["--user-id", "00000003"]);
        assert_eq!(preparer("rpcs3", racine, "Seb", &[], &Manette::Auto(None)).unwrap(), s);
        assert_eq!(std::fs::read_to_string(racine.join("dev_hdd0/home/00000003/localusername")).unwrap(), "Léa");
        // Regarder n'écrit rien : un compte qui n'existe pas n'est pas créé.
        assert_eq!(compte_rpcs3_existant(racine, "Léa").as_deref(), Some("00000003"));
        assert_eq!(compte_rpcs3_existant(racine, "Papa"), None);
        assert!(!racine.join("dev_hdd0/home/00000004").exists());
        // Le même compte que celui du lancement, même pour un nom qui n'est pas un nom de dossier valable.
        let nom = "Papa ?";
        let lance = preparer("rpcs3", racine, nom, &[], &Manette::Auto(None)).unwrap();
        assert_eq!(compte_rpcs3_existant(racine, &crate::jeux_pc::nom_de_dossier(nom)).as_deref(), Some(lance[1].as_str()));

        // xemu : réglages et disque dur copiés pour le profil.
        let disque = racine.join("system").join("xbox_hdd.qcow2");
        std::fs::create_dir_all(disque.parent().unwrap()).unwrap();
        std::fs::write(&disque, b"disque").unwrap();
        std::fs::write(racine.join("xemu.toml"), format!("[sys.files]\nbootrom_path = '{}'\nhdd_path = '{}'\n", racine.join("system/mcpx.bin").display(), disque.display())).unwrap();
        let x = preparer("xemu", racine, "Seb", &[], &Manette::Auto(None)).unwrap();
        let cfg = racine.join("Profils").join("Seb").join("xemu.toml");
        assert_eq!(x, vec!["-config_path".to_string(), cfg.display().to_string()]);
        let t = std::fs::read_to_string(&cfg).unwrap();
        assert_eq!(valeur_toml(&t, "sys.files", "hdd_path").unwrap(), racine.join("Profils").join("Seb").join("xbox_hdd.qcow2").display().to_string());
        assert!(t.contains("bootrom_path"), "les fichiers de la console restent ceux réglés");
        assert_eq!(std::fs::read(racine.join("Profils").join("Seb").join("xbox_hdd.qcow2")).unwrap(), b"disque");
        assert_eq!(std::fs::read(&disque).unwrap(), b"disque", "le disque d'origine n'est pas touché");

        // Eden : l'utilisateur Switch du nom du profil.
        assert_eq!(preparer("eden", racine, "Léa", &[], &Manette::Auto(None)).unwrap(), vec!["-u", "Léa"]);

        // Azahar : NAND et carte SD du profil.
        preparer("azahar", racine, "Seb", &[], &Manette::Auto(None)).unwrap();
        let ini = std::fs::read_to_string(racine.join("user/config/qt-config.ini")).unwrap();
        assert!(ini.contains("[Data%20Storage]") && ini.contains("use_custom_storage = true"));
        assert!(ini.contains("Profils/Seb/sdmc/"));
    }

    #[test]
    fn un_fichier_de_triche_va_dans_le_dossier_du_profil_et_jamais_ailleurs() {
        let e = Path::new("E:\\Emu\\RetroArch");
        let p = place_triche("retroarch", e, "Seb", "donnees", "cheats/Beetle PSX HW", "Crash Bandicoot (USA).cht").unwrap();
        assert_eq!(p, e.join("Profils").join("Seb").join("cheats").join("Beetle PSX HW").join("Crash Bandicoot (USA).cht"));
        let d = Path::new("E:\\Emu\\Dolphin");
        assert_eq!(place_triche("dolphin", d, "Seb", "donnees", "GameSettings", "GALE01.ini").unwrap(), d.join("Profils/Seb/User/GameSettings/GALE01.ini"));
        assert_eq!(place_triche("xemu", d, "Seb", "programme", "", "a.txt").unwrap(), d.join("a.txt"));
        assert!(place_triche("pcsx2", e, "Seb", "donnees", "../../Windows", "x.pnach").is_err());
        assert!(place_triche("pcsx2", e, "Seb", "donnees", "cheats", "..\\x.pnach").is_err());
        assert!(place_triche("pcsx2", e, "Seb", "donnees", "C:/Windows", "x.pnach").is_err());
        assert!(place_triche("pcsx2", e, "Seb", "donnees", "\\Windows\\System32", "x.pnach").is_err(), "chemin enraciné");
        assert!(place_triche("pcsx2", e, "Seb", "donnees", "/Windows", "x.pnach").is_err());
        assert!(place_triche("pcsx2", e, "Seb", "donnees", "cheats\\sous", "x.pnach").unwrap().ends_with("cheats/sous/x.pnach"));
    }

    #[test]
    fn le_choix_des_commandes_du_jeu_est_suivi() {
        // Clavier et souris : aucune manette réglée, ni combinaison de menu RetroArch.
        let d = tempfile::tempdir().unwrap();
        preparer("duckstation", d.path(), "Seb", &[], &Manette::Clavier).unwrap();
        let ini = std::fs::read_to_string(d.path().join("settings.ini")).unwrap();
        assert!(!ini.contains("SDL-0"));
        preparer("retroarch", d.path(), "Seb", &[], &Manette::Clavier).unwrap();
        let cfg = std::fs::read_to_string(d.path().join("Profils").join("Seb").join("frogtend.cfg")).unwrap();
        assert!(!cfg.contains("gamepad_combo"));

        // Une Wii : la référence par défaut, puis celle choisie pour le jeu.
        let magasin = d.path().join("magasin");
        let (e, g, n) = crate::references::par_defaut("Nintendo Wii").unwrap();
        let defaut = crate::references::trouver(&magasin, e, g, n).unwrap();
        let args = preparer("dolphin", d.path(), "Seb", &[], &Manette::Auto(Some(defaut))).unwrap();
        let wiimote = PathBuf::from(&args[1]).join("Config").join("WiimoteNew.ini");
        assert!(std::fs::read_to_string(&wiimote).unwrap().contains("Extension = Nunchuk"));
        let h = crate::references::trouver(&magasin, "dolphin", "Wiimote", "Wiimote horizontale").unwrap();
        preparer("dolphin", d.path(), "Seb", &[], &Manette::Imposee(h)).unwrap();
        let t = std::fs::read_to_string(&wiimote).unwrap();
        assert!(t.contains("Options/Sideways Wiimote = True") && !t.contains("Nunchuk/"));
    }

    #[test]
    fn le_memstick_de_ppsspp_devient_un_lien_vers_le_profil_sans_rien_effacer() {
        let d = tempfile::tempdir().unwrap();
        let ancien = d.path().join("memstick").join("PSP").join("SAVEDATA");
        std::fs::create_dir_all(&ancien).unwrap();
        std::fs::write(ancien.join("partie.bin"), b"avant").unwrap();

        preparer("ppsspp", d.path(), "Seb", &[], &Manette::Auto(None)).unwrap();
        assert!(d.path().join("memstick (avant Frogtend)").join("PSP").join("SAVEDATA").join("partie.bin").is_file());
        std::fs::write(d.path().join("memstick").join("x.txt"), b"seb").unwrap();
        assert!(d.path().join("Profils").join("Seb").join("memstick").join("x.txt").is_file(), "le lien mène au profil");

        preparer("ppsspp", d.path(), "Léa", &[], &Manette::Auto(None)).unwrap();
        assert!(!d.path().join("memstick").join("x.txt").exists(), "Léa ne voit pas les fichiers de Seb");
        assert!(d.path().join("Profils").join("Seb").join("memstick").join("x.txt").is_file(), "ceux de Seb restent");
    }
}

#[cfg(test)]
mod essai_reel_micrologiciel {
    #[test]
    #[ignore]
    fn lire_le_vrai_micrologiciel() {
        let d = std::path::PathBuf::from(std::env::var("FROGTEND_RPCS3").unwrap());
        println!("MICROLOGICIEL {:?}", super::micrologiciel_rpcs3(&d));
    }
}
