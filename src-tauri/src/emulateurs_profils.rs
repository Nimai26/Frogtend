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

// --- MAME par profil (lot 0.80.0 ; les sauvegardes sont sacrées). Relevé dans les sources de MAME 0.289
// (`src/emu/emuopts.cpp`/`.h`, 19/10) : `cfg_directory` (touches, réglages de la borne), `nvram_directory` (mémoire du
// jeu : meilleurs scores, EEPROM), `state_directory` (sauvegardes d'état), `diff_directory` (disques durs modifiés),
// `homepath` (données des greffons : le greffon hiscore écrit dans `<homepath>/hiscore`, `plugins/hiscore/init.lua`).
// Frogtend les passe sur la LIGNE DE COMMANDE, vers `Profils\<profil>\…` (couvert par la sauvegarde du profil) :
// aucun fichier de réglages de MAME n'est modifié. Une option déjà réglée par la personne dans `mame.ini` (autre que la
// valeur d'origine) est respectée. ---

/// (option de MAME, sa valeur d'origine, le sous-dossier du profil).
pub const DOSSIERS_MAME: [(&str, &str, &str); 5] =
    [("cfg_directory", "cfg", "cfg"), ("nvram_directory", "nvram", "nvram"), ("state_directory", "sta", "sta"), ("diff_directory", "diff", "diff"), ("homepath", ".", "home")];

/// Ce que la première partie d'un profil reprend de MAME (copié, jamais déplacé) : au-delà, rien n'est copié (un
/// disque dur modifié peut peser des Go) et c'est noté.
pub const REPRISE_MAME_MAX: u64 = 256 * 1024 * 1024;

/// La valeur d'une option dans le texte de `mame.ini` (`option   valeur`), sans guillemets.
pub fn option_mame_ini(texte: &str, option: &str) -> Option<String> {
    texte.lines().find_map(|l| {
        let (cle, valeur) = l.trim().split_once(char::is_whitespace)?;
        cle.eq_ignore_ascii_case(option).then(|| valeur.trim().trim_matches('"').to_string())
    })
}

/// La taille des fichiers d'un dossier (liens et jonctions ignorés), arrêtée dès qu'elle dépasse `plafond`.
fn taille_arbre(d: &Path, plafond: u64) -> u64 {
    let mut total = 0u64;
    let mut pile = vec![d.to_path_buf()];
    while let Some(x) = pile.pop() {
        let Ok(l) = std::fs::read_dir(&x) else { continue };
        for e in l.flatten() {
            let Ok(m) = std::fs::symlink_metadata(e.path()) else { continue };
            if m.file_type().is_symlink() || crate::garde::est_un_lien(&e.path()) {
                continue;
            }
            if m.is_dir() {
                pile.push(e.path());
            } else {
                total += m.len();
                if total > plafond {
                    return total;
                }
            }
        }
    }
    total
}

/// Copie un dossier dans un autre sans JAMAIS écraser (un fichier déjà là est gardé) ; liens et jonctions ignorés.
fn copier_sans_ecraser(source: &Path, cible: &Path) -> Resultat<u64> {
    let mut n = 0;
    std::fs::create_dir_all(cible)?;
    for e in std::fs::read_dir(source)?.flatten() {
        let m = std::fs::symlink_metadata(e.path())?;
        if m.file_type().is_symlink() || crate::garde::est_un_lien(&e.path()) {
            continue;
        }
        let vers = cible.join(e.file_name());
        if m.is_dir() {
            n += copier_sans_ecraser(&e.path(), &vers)?;
        } else if !vers.exists() {
            std::fs::copy(e.path(), &vers)?;
            n += 1;
        }
    }
    Ok(n)
}

/// Les dossiers de MAME de ce profil, à passer sur sa ligne de commande (`-nvram_directory "<…>"`…), et les notes à
/// journaliser (option laissée à la personne, reprise trop grosse). La PREMIÈRE fois pour ce profil, ce que MAME avait
/// déjà (scores, touches de la personne, états) y est copié — jamais déplacé ni écrasé.
pub fn preparer_mame_profil(mame: &Path, profil: &str) -> Resultat<(Vec<String>, Vec<String>)> {
    // `mame.ini` est cherché à la racine de MAME, puis dans `ini\` (son `inipath` d'origine : « .;ini;ini/presets ») :
    // le premier trouvé compte (ordre du chemin de recherche ; non vérifié dans les sources qu'un second ne s'ajoute pas).
    let inis: Vec<String> = [mame.join("mame.ini"), mame.join("ini").join("mame.ini")].iter().find_map(|f| std::fs::read_to_string(f).ok()).into_iter().collect();
    let racine = dossier_du_profil(mame, profil);
    let (mut args, mut notes) = (Vec::new(), Vec::new());
    // Un chemin que la ligne ne porterait pas sans risque (« % » d'une variable, « ; » séparateur de MAME,
    // « autoboot » lu comme un script de la personne) : MAME garde ses dossiers communs, c'est noté.
    let r = texte(&racine).to_lowercase();
    if r.contains(['%', ';', '"']) || r.contains("autoboot") {
        notes.push(format!("MAME : le nom du profil « {profil} » ne peut pas passer sur la ligne de MAME : parties dans les dossiers communs"));
        return Ok((args, notes));
    }
    for (option, origine, sous) in DOSSIERS_MAME {
        let reglee = inis.iter().find_map(|t| option_mame_ini(t, option)).filter(|v| !v.is_empty());
        if reglee.as_deref().is_some_and(|v| v != origine) {
            notes.push(format!("MAME : « {option} » réglé par la personne dans mame.ini, laissé tel quel (pas par profil)"));
            continue;
        }
        let cible = racine.join(sous);
        // La reprise de ce que MAME avait (copiée, jamais déplacée) : faite UNE fois par option, marquée une fois
        // RÉUSSIE (un dossier du profil créé plus tôt — l'état « Sauver » du 0.79.0 — n'empêche pas la reprise).
        let marque = racine.join(format!(".frogtend-reprise-{sous}"));
        if !marque.is_file() {
            // Pour `homepath` (« . » : tout MAME), seulement les données des greffons qui sont des parties ou des
            // réglages de la personne (sources 0.289 : hiscore, autofire, inputmacro, timer écrivent dans `<homepath>/…`).
            let paires: Vec<(PathBuf, PathBuf)> = if sous == "home" {
                ["hiscore", "autofire", "inputmacro", "timer"].iter().map(|g| (mame.join(g), cible.join(g))).collect()
            } else {
                vec![(mame.join(origine), cible.clone())]
            };
            let presentes: Vec<_> = paires.into_iter().filter(|(s, _)| s.is_dir()).collect();
            let taille: u64 = presentes.iter().map(|(s, _)| taille_arbre(s, REPRISE_MAME_MAX)).sum();
            if taille > REPRISE_MAME_MAX {
                // Trop gros (un disque dur modifié…) : MAME garde SON dossier pour cette option, jamais un dossier vide.
                notes.push(format!("MAME : « {option} » trop gros pour être repris dans le profil ({} Mo et plus) : MAME garde son dossier commun", REPRISE_MAME_MAX / 1_048_576));
                continue;
            }
            // Une copie interrompue (fichier verrouillé) : MAME garde son dossier commun cette fois ; la suivante
            // reprend sans rien écraser.
            if let Err(e) = presentes.iter().try_for_each(|(s, v)| copier_sans_ecraser(s, v).map(|_| ())) {
                notes.push(format!("MAME : reprise de « {option} » interrompue ({e}) : MAME garde son dossier commun, nouvel essai au prochain lancement"));
                continue;
            }
            std::fs::create_dir_all(&cible)?;
            std::fs::write(&marque, b"")?;
        }
        std::fs::create_dir_all(&cible)?;
        args.push(format!("-{option}"));
        args.push(texte(&cible));
    }
    Ok((args, notes))
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
    // Un fichier qui existe mais ne se lit pas (encodage, droits, verrou) n'est JAMAIS remplacé : l'appelant a pu
    // construire `apres` à partir d'un texte vide (expert, 05/10).
    let avant = match std::fs::read_to_string(fichier) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e.into()),
    };
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
    // Un fichier qui existe mais ne se lit pas n'est JAMAIS remplacé (il le serait par une seule section ; expert, 05/10).
    let avant = match std::fs::read_to_string(fichier) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e.into()),
    };
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
                cfg.push_str(&crate::manettes::lignes_retroarch());
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
            crate::manettes::retirer_menu_manette(emulateur, &ini)?;
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
            crate::manettes::retirer_menu_manette(emulateur, &ini)?;
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
            // Un jeu GameCube en plusieurs disques (.m3u) : Dolphin change de disque seul quand le jeu le demande
            // (« Change Discs Automatically »). Posé seulement si la personne n'a rien réglé : jamais son choix défait.
            if crate::manettes::lire_ini(&std::fs::read_to_string(&ini).unwrap_or_default(), "Core", "AutoDiscChange").is_empty() {
                modifier_ini(emulateur, &ini, "Core", &[("AutoDiscChange", vec!["True".into()])])?;
            }
            regler_manette(id, emulateur, Some(&utilisateur), manette)?;
            Ok(vec!["-u".into(), texte(&utilisateur)])
        }
        // Les parties PAR PERSONNE : mGBA les écrit à côté de la ROM d'office (jamais protégées) ; `-C` les envoie dans le
        // dossier du profil pour CE lancement, sans toucher à la configuration de mGBA (relevé le 26/10).
        "mgba" => {
            let (saves, etats, captures) = (p.join("saves"), p.join("states"), p.join("screenshots"));
            creer(&[&saves, &etats, &captures])?;
            Ok(vec![
                "-C".into(),
                format!("savegamePath={}", texte(&saves)),
                "-C".into(),
                format!("savestatePath={}", texte(&etats)),
                "-C".into(),
                format!("screenshotPath={}", texte(&captures)),
            ])
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
                v.push(("hdd_path", vec![chaine_toml(&texte(&disque))]));
            }
            if !v.is_empty() {
                modifier_ini(emulateur, &cfg, "sys.files", &v)?;
            }
            modifier_ini(emulateur, &cfg, "display.window", &[("fullscreen_on_startup", vec!["true".into()])])?;
            Ok(vec!["-config_path".into(), texte(&cfg)])
        }
        // Eden (Switch) : l'utilisateur Switch qui porte le nom du profil (`-u <nom>`, core/launch_params.cpp) ; à
        // créer une fois dans Eden. Absent : Eden garde son utilisateur courant.
        // Eden : l'utilisateur Switch de ce profil (créé s'il manque), passé par son INDEX — `-u <nom>` inconnu ferait
        // jouer en silence l'utilisateur précédent (eden.rs). Seulement Eden portable et FERMÉ (sinon il écraserait son
        // fichier des utilisateurs) ; autrement, le nom, comme avant.
        "eden" => {
            // Pas portable, ou ouvert : le nom, comme avant. Sinon, une erreur (fichier abîmé, 8 utilisateurs) est DITE :
            // lancer quand même ferait jouer l'utilisateur de quelqu'un d'autre, sur SES parties.
            if !crate::eden::portable(emulateur) || !crate::lancement::processus_du_dossier(emulateur).is_empty() {
                return Ok(vec!["-u".into(), profil_de(p).to_string()]);
            }
            let u = crate::eden::utilisateur_du_profil(emulateur, profil_de(p))?;
            // Les parties d'avant Frogtend : copiées chez la première personne qui joue. Un échec ne touche à rien
            // (elles restent où elles étaient) et n'empêche pas de jouer.
            let _ = crate::eden::reprendre_parties_d_avant(emulateur, &u);
            Ok(vec!["-u".into(), u.index.to_string()])
        }
        // Cemu : le compte Wii U de ce profil (créé s'il manque), passé par `-a` — un compte inconnu ferait jouer le
        // premier en silence (cemu.rs). Seulement Cemu portable et FERMÉ ; sinon (ou fichier de compte impossible à
        // écrire), Cemu part comme avant, jamais bloqué.
        "cemu" => {
            // Pas portable, ou ouvert : comme avant. Sinon, une erreur (12 comptes, dossier de Cemu absent) est DITE :
            // lancer sans `-a` ferait jouer le compte de quelqu'un d'autre, sur SES parties.
            if !crate::cemu::portable(emulateur) || !crate::lancement::processus_du_dossier(emulateur).is_empty() {
                return Ok(vec![]);
            }
            let c = crate::cemu::compte_du_profil(emulateur, profil_de(p))?;
            let _ = crate::cemu::reprendre_parties_d_avant(emulateur, &c);
            Ok(vec!["-a".into(), format!("{:08x}", c.id)])
        }
        // RPCS3 : un compte RPCS3 par profil (\`--user-id\`, rpcs3.cpp), donc ses propres parties.
        "rpcs3" => {
            // La manette du joueur 1, à CHAQUE partie (oubli de la 0.44.0, vu par Seb le 04/10 : rien n'était branché).
            // Un fichier qui ne peut pas être écrit (disque en lecture seule…) n'empêche jamais de jouer : RPCS3 garde
            // alors son réglage d'avant.
            let _ = regler_manette(id, emulateur, None, manette);
            // Les boîtes de RPCS3 (invisibles en plein écran) sont désactivées par `commandes::preparer_emulateur`,
            // qui note un échec au journal : `rpcs3_sans_boites`.
            Ok(vec!["--user-id".into(), compte_rpcs3(emulateur, profil_de(p))?])
        }
        // Azahar : NAND et carte SD du profil (\`[Data Storage]\` de qt-config.ini, configuration/config.cpp).
        "azahar" => {
            // Seulement en mode portable (dossier `user` déjà là) : l'écrire ferait basculer Azahar en portable, loin des
            // réglages de la personne dans %APPDATA% (relecture du 16/10). Sinon, ses données restent communes.
            if !emulateur.join("user").is_dir() {
                return Ok(vec![]);
            }
            let (nand, sdmc) = (p.join("nand"), p.join("sdmc"));
            creer(&[&nand, &sdmc])?;
            let ini = emulateur.join("user").join("config").join("qt-config.ini");
            // QSettings : « \\ » est un échappement, on écrit des « / » ; entre guillemets (une virgule ferait une liste).
            let barre = |c: &Path| valeur_qt(&format!("{}/", texte(c).replace('\\', "/")));
            modifier_ini(
                emulateur,
                &ini,
                "Data%20Storage",
                &[
                    ("use_custom_storage\\default", vec!["false".into()]),
                    ("use_custom_storage", vec!["true".into()]),
                    // QSettings ignore une valeur tant que son `\default` vaut true (comme pour la pause, pilotage.rs).
                    ("nand_directory\\default", vec!["false".into()]),
                    ("nand_directory", vec![barre(&nand)]),
                    ("sdmc_directory\\default", vec!["false".into()]),
                    ("sdmc_directory", vec![barre(&sdmc)]),
                ],
            )?;
            Ok(vec![])
        }
        _ => Ok(vec![]),
    }
}

/// mGBA : la partie qu'il avait écrite À CÔTÉ de la ROM (`<rom>.sav`, son réglage d'origine) est COPIÉE dans le
/// dossier de parties du profil (`-C savegamePath`) si celui-ci n'en a pas encore pour ce jeu. Jamais déplacée, jamais
/// par-dessus. Rend la copie faite.
pub fn reprendre_partie_mgba(rom: &Path, emulateur: &Path, profil: &str) -> Resultat<Option<PathBuf>> {
    let (Some(nom), Some(dossier)) = (rom.file_stem(), rom.parent()) else { return Ok(None) };
    let ancienne = dossier.join(format!("{}.sav", nom.to_string_lossy()));
    let nouvelle = dossier_du_profil(emulateur, profil).join("saves").join(format!("{}.sav", nom.to_string_lossy()));
    if !ancienne.is_file() || nouvelle.exists() {
        return Ok(None);
    }
    creer(&[nouvelle.parent().unwrap_or(emulateur)])?;
    let octets = std::fs::read(&ancienne)?;
    let mut f = std::fs::OpenOptions::new().write(true).create_new(true).open(&nouvelle)?;
    std::io::Write::write_all(&mut f, &octets)?;
    drop(f);
    // Vérifier le résultat : la copie relue est identique.
    if std::fs::read(&nouvelle)? != octets {
        let _ = std::fs::remove_file(&nouvelle);
        return Err(Erreur::Disque("La copie de la partie de mGBA n'est pas identique : elle n'est pas gardée.".into()));
    }
    Ok(Some(nouvelle))
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

/// Une valeur d'un fichier TOML simple (`[section]` puis `cle = 'valeur'` ou `"valeur"`) : une chaîne littérale
/// `'…'` telle quelle, une chaîne basique `"…"` sans ses échappements (`\\`, `\"`, `\t`, `\n`, `\r`, `\uXXXX`).
pub fn valeur_toml(texte: &str, section: &str, cle: &str) -> Option<String> {
    let v = crate::manettes::lire_ini(texte, section, cle).into_iter().next()?;
    let v = v.trim();
    let r = if let Some(x) = v.strip_prefix('\'') {
        x.split('\'').next().unwrap_or("").to_string()
    } else if let Some(x) = v.strip_prefix('"') {
        sans_echappements(x, '"')
    } else {
        v.to_string()
    };
    Some(r).filter(|s| !s.is_empty())
}

/// Le texte d'une chaîne entre guillemets (après le guillemet ouvrant), jusqu'au guillemet fermant, sans ses
/// échappements (communs à TOML et aux réglages Qt).
fn sans_echappements(x: &str, fin: char) -> String {
    let mut r = String::new();
    let mut c = x.chars();
    while let Some(ch) = c.next() {
        match ch {
            '\\' => match c.next() {
                Some('n') => r.push('\n'),
                Some('t') => r.push('\t'),
                Some('r') => r.push('\r'),
                Some('u') => {
                    let h: String = c.by_ref().take(4).collect();
                    if let Some(u) = u32::from_str_radix(&h, 16).ok().and_then(char::from_u32) {
                        r.push(u);
                    }
                }
                Some(o) => r.push(o),
                None => break,
            },
            ch if ch == fin => break,
            ch => r.push(ch),
        }
    }
    r
}

/// Une chaîne TOML « basique » : entre guillemets, `\` et `"` échappés. Jamais une chaîne littérale `'…'` : une
/// apostrophe dans le chemin (« Jeux d'émulation », un profil « D'Aquin ») la fermerait et rendrait le fichier
/// illisible pour l'émulateur.
pub fn chaine_toml(s: &str) -> String {
    // DEL (permis dans un nom Windows, interdit tel quel en TOML) par son échappement.
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\u{7f}', "\\u007F"))
}

/// Une valeur texte d'un fichier de réglages Qt (QSettings), toujours entre guillemets, `\` et `"` échappés : sans
/// guillemets, une virgule (« Zelda, TOTK ») la ferait relire comme une LISTE, et `\` est un échappement.
pub fn valeur_qt(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Relit une valeur texte de QSettings : entre guillemets (sans ses échappements), ou telle quelle.
pub fn lire_valeur_qt(v: &str) -> String {
    match v.trim().strip_prefix('"') {
        Some(x) => sans_echappements(x, '"'),
        None => v.trim().to_string(),
    }
}

/// Règle des clés d'un fichier de réglages Qt (QSettings : « cle=valeur », sans espace) dans une section, en ne
/// touchant QUE ces lignes : le reste (dont les lignes binaires « @ByteArray(\x1\xd9…) ») est recopié tel quel, fins de
/// ligne comprises. Une clé absente est ajoutée en tête de sa section ; une section absente, à la fin. `None` : rien à
/// changer.
pub fn regler_cles_qt(texte: &str, section: &str, valeurs: &[(&str, &str)]) -> Option<String> {
    let fin_ligne = if texte.contains("\r\n") { "\r\n" } else { "\n" };
    let entete = format!("[{section}]");
    let mut sortie = String::with_capacity(texte.len() + 64);
    let mut dans = false;
    let mut change = false;
    let mut section_vue = false;
    for ligne in texte.split_inclusive('\n') {
        let nue = ligne.trim_end_matches(['\r', '\n']);
        if nue.starts_with('[') {
            dans = nue == entete;
            if dans {
                section_vue = true;
                sortie.push_str(ligne);
                if !ligne.ends_with('\n') {
                    sortie.push_str(fin_ligne);
                }
                // Les clés absentes de la section, ajoutées juste après son en-tête.
                for (c, v) in valeurs {
                    if !texte_section_contient(texte, &entete, c) {
                        sortie.push_str(&format!("{c}={v}{fin_ligne}"));
                        change = true;
                    }
                }
                continue;
            }
        } else if dans {
            if let Some((c, v)) = valeurs.iter().find(|(c, _)| nue.split_once('=').is_some_and(|(k, _)| k == *c)) {
                let voulue = format!("{c}={v}");
                if nue != voulue {
                    change = true;
                    sortie.push_str(&voulue);
                    sortie.push_str(&ligne[nue.len()..]);
                    continue;
                }
            }
        }
        sortie.push_str(ligne);
    }
    if !section_vue {
        if !sortie.is_empty() && !sortie.ends_with('\n') {
            sortie.push_str(fin_ligne);
        }
        sortie.push_str(&entete);
        sortie.push_str(fin_ligne);
        for (c, v) in valeurs {
            sortie.push_str(&format!("{c}={v}{fin_ligne}"));
        }
        change = true;
    }
    change.then_some(sortie)
}

/// Règle des clés de premier niveau d'une section du `config.yml` de RPCS3 (« Section: » puis « ␣␣Cle: valeur »), en ne
/// touchant QUE ces lignes : le reste est recopié tel quel, fins de ligne comprises. Une clé absente est ajoutée en
/// tête de sa section ; une section absente, à la fin. `None` : rien à changer.
pub fn regler_cles_yaml(texte: &str, section: &str, valeurs: &[(&str, &str)]) -> Option<String> {
    let fin_ligne = if texte.contains("\r\n") { "\r\n" } else { "\n" };
    let entete = format!("{section}:");
    // La clé d'une ligne de la section (exactement deux espaces d'indentation : jamais une sous-clé).
    let cle_de = |nue: &str| -> Option<String> {
        let reste = nue.strip_prefix("  ")?;
        if reste.starts_with(' ') {
            return None;
        }
        reste.split_once(':').map(|(k, _)| k.to_string())
    };
    // Les clés déjà présentes dans la section.
    let mut presentes: Vec<String> = Vec::new();
    let mut dans = false;
    for l in texte.lines() {
        if !l.is_empty() && !l.starts_with(' ') {
            dans = l.trim_end() == entete;
        } else if dans {
            presentes.extend(cle_de(l));
        }
    }
    let mut sortie = String::with_capacity(texte.len() + 64);
    let mut change = false;
    let mut section_vue = false;
    dans = false;
    for ligne in texte.split_inclusive('\n') {
        let nue = ligne.trim_end_matches(['\r', '\n']);
        if !nue.is_empty() && !nue.starts_with(' ') {
            dans = nue.trim_end() == entete;
            if dans {
                section_vue = true;
                sortie.push_str(ligne);
                if !ligne.ends_with('\n') {
                    sortie.push_str(fin_ligne);
                }
                for (c, v) in valeurs {
                    if !presentes.iter().any(|p| p == c) {
                        sortie.push_str(&format!("  {c}: {v}{fin_ligne}"));
                        change = true;
                    }
                }
                continue;
            }
        } else if dans {
            if let Some((c, v)) = cle_de(nue).and_then(|k| valeurs.iter().find(|(c, _)| *c == k)) {
                let voulue = format!("  {c}: {v}");
                if nue != voulue {
                    change = true;
                    sortie.push_str(&voulue);
                    sortie.push_str(&ligne[nue.len()..]);
                    continue;
                }
            }
        }
        sortie.push_str(ligne);
    }
    if !section_vue {
        if !sortie.is_empty() && !sortie.ends_with('\n') {
            sortie.push_str(fin_ligne);
        }
        sortie.push_str(&entete);
        sortie.push_str(fin_ligne);
        for (c, v) in valeurs {
            sortie.push_str(&format!("  {c}: {v}{fin_ligne}"));
        }
        change = true;
    }
    change.then_some(sortie)
}

/// RPCS3 et le menu de Frogtend (05/10, Seb : « le menu ne met pas en pause le jeu et les boutons sont encore saisis
/// dans le jeu »). Relevé dans son code (05/10) : `Miscellaneous ▸ Pause emulation on RPCS3 focus loss` (faux par
/// défaut ; gui_application.cpp OnAppStateChanged : pause quand RPCS3 perd le premier plan, reprise quand il le
/// retrouve) et `Input/Output ▸ Background input enabled` (vrai par défaut ; GSFrameBase.cpp : la manette n'est lue
/// sans le premier plan que si vrai). Le menu prenant le premier plan, ces deux réglages font la pause et coupent la
/// manette du jeu. Fichier `config\config.yml`, copié à l'abri ; seules ces deux lignes changent. Les réglages PAR JEU
/// (`config\custom_configs\*.yml`, appliqués par-dessus à chaque démarrage : System.cpp Emulator::Load) reçoivent les
/// mêmes deux lignes, sinon ils les écraseraient.
pub fn rpcs3_pause_au_menu(emulateur: &Path) -> Resultat<bool> {
    let config = emulateur.join("config");
    let fichier = config.join("config.yml");
    // Absent : créé (RPCS3 complète le reste), sauf s'il reste un ancien `config.yml` à la racine que RPCS3 déplacera
    // lui-même (il ne le fait plus si le nouveau existe : System.cpp). Illisible : JAMAIS touché.
    let creer = !emulateur.join("config.yml").exists();
    let mut change = rpcs3_pause_dans(emulateur, &fichier, creer)?;
    if let Ok(par_jeu) = std::fs::read_dir(config.join("custom_configs")) {
        for f in par_jeu.flatten().map(|e| e.path()) {
            if f.is_file() && f.extension().is_some_and(|x| x.eq_ignore_ascii_case("yml")) {
                change |= rpcs3_pause_dans(emulateur, &f, false)?;
            }
        }
    }
    Ok(change)
}

/// Les deux lignes de [`rpcs3_pause_au_menu`] dans un fichier de réglages de RPCS3.
fn rpcs3_pause_dans(emulateur: &Path, fichier: &Path, creer: bool) -> Resultat<bool> {
    let texte = match std::fs::read_to_string(fichier) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && creer => String::new(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.into()),
    };
    let apres_misc = regler_cles_yaml(&texte, "Miscellaneous", &[("Pause emulation on RPCS3 focus loss", "true")]);
    let base = apres_misc.as_deref().unwrap_or(&texte);
    let apres_io = regler_cles_yaml(base, "Input/Output", &[("Background input enabled", "false")]);
    let Some(nouveau) = apres_io.or(apres_misc) else {
        return Ok(false);
    };
    mettre_a_l_abri(emulateur, fichier)?;
    if let Some(p) = fichier.parent() {
        std::fs::create_dir_all(p)?;
    }
    std::fs::write(fichier, nouveau)?;
    Ok(true)
}

// --- RPCS3 : image et fluidité d'un jeu (✅ Seb 19/10 : « shaders = les deux »). Relevé dans les sources de RPCS3
// (master, 19/10) : `Emu/system_config.h` section `Video`, chaînes de `system_config_types.cpp`, vérifiées dans le vrai
// `config.yml` de Seb ; le fichier d'un jeu est `config\custom_configs\config_<numéro>.yml` (`Emu/system_utils.cpp`),
// appliqué par-dessus le global à chaque démarrage. Frogtend n'y écrit QUE les clés changées par la personne. ---

/// Les réglages d'image d'un jeu : (clé dans `Video:`, valeurs admises, valeur par défaut de RPCS3).
pub const RPCS3_IMAGE: [(&str, &[&str], &str); 6] = [
    (
        "Shader Mode",
        &["Async Recompiler with Shader Interpreter", "Async Recompiler (multi-threaded)", "Shader Interpreter only"],
        "Async Recompiler with Shader Interpreter",
    ),
    ("Resolution Scale", &["100", "150", "200", "300"], "100"),
    ("Output Scaling Mode", &["Bilinear", "FidelityFX Super Resolution"], "Bilinear"),
    ("FidelityFX CAS Sharpening Intensity", &["0", "25", "50", "75", "100"], "50"),
    ("Anisotropic Filter Override", &["0", "2", "4", "8", "16"], "0"),
    ("MSAA", &["Auto", "Disabled"], "Auto"),
];

/// Un numéro de jeu PS3 utilisable dans un nom de fichier : 4 lettres majuscules puis 5 chiffres (`BLES01227`). Il est
/// lu dans le PARAM.SFO d'un disque : jamais pris tel quel (un `..\x` sortirait du dossier).
pub fn numero_ps3_sur(n: &str) -> bool {
    // Sur les octets : un caractère non ASCII lu dans un PARAM.SFO abîmé ne coupe jamais une chaîne en deux.
    let b = n.as_bytes();
    b.len() == 9 && b[..4].iter().all(u8::is_ascii_uppercase) && b[4..].iter().all(u8::is_ascii_digit)
}

/// Le fichier des réglages propres à un jeu PS3 dans RPCS3.
pub fn rpcs3_config_du_jeu(rpcs3: &Path, numero: &str) -> Resultat<PathBuf> {
    if !numero_ps3_sur(numero) {
        return Err(Erreur::Refus(format!("Numéro de jeu PS3 inattendu ({numero}) : rien n'est écrit.")));
    }
    Ok(rpcs3.join("config").join("custom_configs").join(format!("config_{numero}.yml")))
}

/// Les valeurs de clés de premier niveau d'une section YAML de RPCS3 (exactement deux espaces d'indentation, comme
/// `regler_cles_yaml`) ; guillemets retirés.
pub fn lire_cles_yaml(texte: &str, section: &str, cles: &[&str]) -> std::collections::HashMap<String, String> {
    let entete = format!("{section}:");
    let mut r = std::collections::HashMap::new();
    let mut dans = false;
    for l in texte.lines() {
        let l = l.trim_end_matches('\r');
        if !l.is_empty() && !l.starts_with(' ') {
            dans = l.trim_end() == entete;
            continue;
        }
        let Some(reste) = l.strip_prefix("  ").filter(|r| dans && !r.starts_with(' ')) else { continue };
        if let Some((k, v)) = reste.split_once(':') {
            if cles.contains(&k) {
                let v = v.trim();
                let v = v.strip_prefix('"').and_then(|x| x.strip_suffix('"')).or_else(|| v.strip_prefix('\'').and_then(|x| x.strip_suffix('\''))).unwrap_or(v);
                r.insert(k.to_string(), v.to_string());
            }
        }
    }
    r
}

/// Retire des clés de premier niveau d'une section YAML (le reste recopié tel quel) ; une section qui n'a plus rien
/// perd aussi son en-tête (un `Video:` vide serait un nœud nul pour RPCS3). `None` : rien à retirer.
pub fn retirer_cles_yaml(texte: &str, section: &str, cles: &[&str]) -> Option<String> {
    let entete = format!("{section}:");
    let mut lignes: Vec<&str> = Vec::new();
    // Où commence la section dans `lignes`, et combien de lignes lui restent.
    let mut debut: Option<usize> = None;
    let mut restent = 0usize;
    let mut dans = false;
    let mut change = false;
    for ligne in texte.split_inclusive('\n') {
        let nue = ligne.trim_end_matches(['\r', '\n']);
        if !nue.is_empty() && !nue.starts_with(' ') {
            dans = nue.trim_end() == entete;
            if dans {
                debut = Some(lignes.len());
                restent = 0;
            }
        } else if dans {
            if let Some(reste) = nue.strip_prefix("  ").filter(|r| !r.starts_with(' ')) {
                if reste.split_once(':').is_some_and(|(k, _)| cles.contains(&k)) {
                    change = true;
                    continue;
                }
            }
            if !nue.trim().is_empty() {
                restent += 1;
            }
        }
        lignes.push(ligne);
    }
    if !change {
        return None;
    }
    if let (Some(i), 0) = (debut, restent) {
        lignes.remove(i);
    }
    Some(lignes.concat())
}

/// Les réglages d'image d'un jeu, chacun : celui du fichier du jeu, sinon du global, sinon la valeur par défaut de
/// RPCS3 ; et les clés réglées POUR CE JEU.
pub fn rpcs3_image(rpcs3: &Path, numero: &str) -> Resultat<(Vec<(String, String)>, Vec<String>)> {
    let cles: Vec<&str> = RPCS3_IMAGE.iter().map(|(k, _, _)| *k).collect();
    let du_jeu = match std::fs::read_to_string(rpcs3_config_du_jeu(rpcs3, numero)?) {
        Ok(t) => lire_cles_yaml(&t, "Video", &cles),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Default::default(),
        Err(e) => return Err(e.into()),
    };
    // Absent : les défauts de RPCS3 ; illisible : une erreur (jamais des valeurs fausses à l'écran).
    let global = match std::fs::read_to_string(rpcs3.join("config").join("config.yml")) {
        Ok(t) => lire_cles_yaml(&t, "Video", &cles),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Default::default(),
        Err(e) => return Err(e.into()),
    };
    let valeurs = RPCS3_IMAGE
        .iter()
        .map(|(k, _, defaut)| (k.to_string(), du_jeu.get(*k).or_else(|| global.get(*k)).cloned().unwrap_or_else(|| defaut.to_string())))
        .collect();
    let par_jeu = RPCS3_IMAGE.iter().filter(|(k, _, _)| du_jeu.contains_key(*k)).map(|(k, _, _)| k.to_string()).collect();
    Ok((valeurs, par_jeu))
}

/// Le cache de shaders sur le disque est-il coupé pour ce jeu (`Video ▸ Disable On-Disk Shader Cache: true`, dans le
/// fichier du jeu, sinon le global) ? Alors « Sans saccades » recompile tout à chaque partie. Lu seulement, jamais
/// écrit : la personne l'a peut-être coupé exprès dans RPCS3 (✅ Seb 19/10 : prévenir ; avis de l'expert lancement : sans y toucher).
pub fn rpcs3_cache_shaders_coupe(rpcs3: &Path, numero: &str) -> Resultat<bool> {
    const CLE: &str = "Disable On-Disk Shader Cache";
    for f in [rpcs3_config_du_jeu(rpcs3, numero)?, rpcs3.join("config").join("config.yml")] {
        match std::fs::read_to_string(&f) {
            Ok(t) => {
                if let Some(v) = lire_cles_yaml(&t, "Video", &[CLE]).get(CLE) {
                    return Ok(v == "true");
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(false)
}

/// Règle pour CE jeu les clés données (seulement celles-là, valeurs de la liste fermée), dans son fichier de RPCS3
/// (créé s'il manque, copié à l'abri sinon) ; `vide` = revenir aux valeurs de RPCS3 (les clés d'image retirées du
/// fichier du jeu, jamais le fichier lui-même).
pub fn rpcs3_regler_image(rpcs3: &Path, numero: &str, valeurs: &[(String, String)], vide: bool) -> Resultat<()> {
    let fichier = rpcs3_config_du_jeu(rpcs3, numero)?;
    // Un ancien `config.yml` à la racine que RPCS3 déplacera lui-même : on n'écrit rien tant qu'il est là.
    if rpcs3.join("config.yml").exists() && !rpcs3.join("config").join("config.yml").exists() {
        return Err(Erreur::Refus("RPCS3 n'a pas encore rangé ses réglages : lance une fois un jeu PS3, puis reviens ici.".into()));
    }
    if vide && !valeurs.is_empty() {
        return Err(Erreur::Refus("Réglage inattendu : revenir aux valeurs de RPCS3 ne prend aucune valeur.".into()));
    }
    for (i, (k, v)) in valeurs.iter().enumerate() {
        // Une clé deux fois serait écrite deux fois (clé YAML en double).
        if valeurs[..i].iter().any(|(c, _)| c == k) {
            return Err(Erreur::Refus(format!("Réglage en double : {k}.")));
        }
        let ok = RPCS3_IMAGE.iter().any(|(c, admises, _)| c == k && admises.contains(&v.as_str()));
        if !ok {
            return Err(Erreur::Refus(format!("Réglage inattendu : {k} = {v}.")));
        }
    }
    let avant = match std::fs::read_to_string(&fichier) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e.into()),
    };
    let apres = if vide {
        let cles: Vec<&str> = RPCS3_IMAGE.iter().map(|(k, _, _)| *k).collect();
        retirer_cles_yaml(&avant, "Video", &cles)
    } else {
        let v: Vec<(&str, &str)> = valeurs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        regler_cles_yaml(&avant, "Video", &v)
    };
    let Some(mut apres) = apres else { return Ok(()) };
    // Jamais un fichier vide (sa lecture par RPCS3 n'est pas vérifiée) : à la place, les deux lignes que Frogtend y
    // met déjà à chaque lancement (pause au menu), qui valent aussi dans le global : rien ne change pour le jeu.
    if apres.trim().is_empty() {
        apres = "Miscellaneous:\n  Pause emulation on RPCS3 focus loss: true\nInput/Output:\n  Background input enabled: false\n".into();
    }
    remplacer_config(rpcs3, &fichier, &apres)
}

/// La section `entete` de ce texte contient-elle la clé `cle` ?
fn texte_section_contient(texte: &str, entete: &str, cle: &str) -> bool {
    let mut dans = false;
    for l in texte.lines() {
        let l = l.trim_end_matches('\r');
        if l.starts_with('[') {
            dans = l == entete;
        } else if dans && l.split_once('=').is_some_and(|(k, _)| k == cle) {
            return true;
        }
    }
    false
}

/// RPCS3 en plein écran sans interface (`--no-gui`) ne doit ouvrir AUCUNE boîte : invisible derrière le jeu, elle le
/// bloque. Relevé dans son code (05/10) : l'écran d'accueil « Welcome to RPCS3 » (`infoBoxEnabledWelcome`, vrai par
/// défaut, ouvert à chaque lancement) et la question « Exit Game? » à la fermeture (`confirmationBoxExitGame`) — vu
/// chez Seb : processus caché « ne répond pas » après « Quitter », puis plantage « fenêtres encore ouvertes » à
/// l'arrêt. Fichier `GuiConfigs\CurrentSettings.ini`, section `[main_window]` ; copié à l'abri avant d'être modifié.
pub fn rpcs3_sans_boites(emulateur: &Path) -> Resultat<bool> {
    let fichier = emulateur.join("GuiConfigs").join("CurrentSettings.ini");
    // Absent : il est créé. Illisible (encodage inattendu…) : JAMAIS touché (il serait remplacé par un fichier vide).
    let texte = match std::fs::read_to_string(&fichier) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e.into()),
    };
    let Some(nouveau) = regler_cles_qt(&texte, "main_window", &[("infoBoxEnabledWelcome", "false"), ("confirmationBoxExitGame", "false")]) else {
        return Ok(false);
    };
    mettre_a_l_abri(emulateur, &fichier)?;
    if let Some(p) = fichier.parent() {
        std::fs::create_dir_all(p)?;
    }
    std::fs::write(&fichier, nouveau)?;
    Ok(true)
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

/// Dans une sauvegarde, le dossier (sous celui de l'émulateur) des parties du compte RPCS3 du profil — sans son numéro.
pub const COMPTE_RPCS3: &str = ".rpcs3-compte";
/// Ce qui part dans la sauvegarde, du compte RPCS3 d'un profil : ses parties et ses trophées. JAMAIS `exdata` (les
/// licences .rif et act.dat).
pub const PARTIES_RPCS3: [&str; 2] = ["savedata", "trophy"];

/// Le programme est-il RPCS3 ?
pub fn est_rpcs3(programme: &str) -> bool {
    Path::new(programme).file_name().is_some_and(|n| n.to_string_lossy().to_ascii_lowercase().starts_with("rpcs3"))
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
        // mGBA non portable (sans `portable.ini`) : `mCoreConfigDirectory`, dossier « mGBA » de l’AppData itinérant.
        "mgba" => vec![env("APPDATA").map(|a| a.join("mGBA"))],
        _ => vec![],
    };
    candidats.into_iter().flatten().filter(|p| p.is_dir()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Essai RÉEL en LECTURE SEULE sur les jeux PS3 de Seb : numéro lu sur le disque, réglages d'image affichés.
    /// `FROGTEND_REGISTRE=<copie de jeux.db> FROGTEND_RPCS3=E:\Emulateurs\RPCS3 cargo test essai_image_ps3 -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn essai_image_ps3() {
        let r = crate::jeux_pc::Registre::ouvrir(Path::new(&std::env::var("FROGTEND_REGISTRE").unwrap())).unwrap();
        let rpcs3 = PathBuf::from(std::env::var("FROGTEND_RPCS3").unwrap());
        for j in r.tous().unwrap().into_iter().filter(|j| j.plateforme.contains("3")) {
            let f = j.installation.as_ref().map(|i| match &i.fichier_du_jeu {
                Some(f) => Path::new(&i.dossier).join(f),
                None => PathBuf::from(&i.dossier),
            });
            let n = f.as_deref().and_then(crate::contenus::titre_id_ps3);
            let image = n.as_deref().filter(|n| numero_ps3_sur(n)).map(|n| rpcs3_image(&rpcs3, n).map(|(v, p)| (v.len(), p.len())));
            let version = f.as_deref().zip(n.as_deref()).and_then(|(f, n)| crate::contenus::version_ps3(f, &rpcs3, n));
            println!("PS3 « {} » : numéro {n:?} ; version {version:?} ; image {image:?}", j.titre);
        }
    }

    /// Répétition sur une COPIE du vrai RPCS3 de Seb (`FROGTEND_RPCS3_COPIE` : un dossier d'essai avec
    /// `config\config.yml` copié) : régler, relire, revenir aux valeurs de RPCS3.
    #[test]
    #[ignore]
    fn essai_regler_image_copie() {
        let rpcs3 = PathBuf::from(std::env::var("FROGTEND_RPCS3_COPIE").unwrap());
        let global_avant = std::fs::read(rpcs3.join("config").join("config.yml")).unwrap();
        rpcs3_regler_image(&rpcs3, "BLES01227", &[("Resolution Scale".into(), "200".into()), ("Output Scaling Mode".into(), "FidelityFX Super Resolution".into())], false).unwrap();
        let f = rpcs3_config_du_jeu(&rpcs3, "BLES01227").unwrap();
        println!("FICHIER DU JEU :\n{}", std::fs::read_to_string(&f).unwrap());
        println!("RELU : {:?}", rpcs3_image(&rpcs3, "BLES01227").unwrap());
        assert_eq!(std::fs::read(rpcs3.join("config").join("config.yml")).unwrap(), global_avant, "le global n'est jamais touché");
        rpcs3_regler_image(&rpcs3, "BLES01227", &[], true).unwrap();
        println!("APRÈS « Valeurs de RPCS3 » :\n{:?}", std::fs::read_to_string(&f).unwrap());
    }

    #[test]
    fn rpcs3_image_d_un_jeu_seulement_les_cles_changees_par_dessus_le_global() {
        let d = tempfile::tempdir().unwrap();
        let rpcs3 = d.path();
        std::fs::create_dir_all(rpcs3.join("config")).unwrap();
        // La forme du vrai config.yml de Seb (relevé du 19/10) : 2 espaces, sous-clés à 4 espaces laissées.
        std::fs::write(
            rpcs3.join("config").join("config.yml"),
            "Video:\n  Renderer: Vulkan\n  MSAA: Auto\n  Shader Mode: Async Recompiler with Shader Interpreter\n  Resolution Scale: 100\n  Output Scaling Mode: Bilinear\n  Vulkan:\n    Adapter: \"\"\nAudio:\n  Renderer: Cubeb\n",
        )
        .unwrap();
        // Un numéro douteux n'est jamais pris dans un chemin.
        assert!(numero_ps3_sur("BLES01227") && !numero_ps3_sur(r"..\..\x") && !numero_ps3_sur("bles01227") && !numero_ps3_sur("BLES0122"));
        assert!(!numero_ps3_sur("AAAé1234") && !numero_ps3_sur("ÉÉÉ12345"), "non ASCII : refusé, sans plantage");
        assert!(rpcs3_regler_image(rpcs3, r"..\x", &[], false).is_err());
        // Une clé en double, ou des valeurs avec « Valeurs de RPCS3 » : refusées, rien d'écrit.
        let r200 = ("Resolution Scale".to_string(), "200".to_string());
        assert!(rpcs3_regler_image(rpcs3, "BLES01227", &[r200.clone(), r200.clone()], false).is_err());
        assert!(rpcs3_regler_image(rpcs3, "BLES01227", &[r200], true).is_err());
        assert!(!rpcs3.join("config").join("custom_configs").exists());
        // Rien pour ce jeu : tout vient du global, ou des défauts de RPCS3.
        let (v, par_jeu) = rpcs3_image(rpcs3, "BLES01227").unwrap();
        let val = |v: &[(String, String)], k: &str| v.iter().find(|(c, _)| c == k).unwrap().1.clone();
        assert_eq!(val(&v, "Resolution Scale"), "100");
        assert_eq!(val(&v, "FidelityFX CAS Sharpening Intensity"), "50", "absent du global : défaut de RPCS3");
        assert!(par_jeu.is_empty());
        // Le cache de shaders : absent = pas coupé ; coupé dans le global ; le fichier du jeu l'emporte.
        assert!(!rpcs3_cache_shaders_coupe(rpcs3, "BLES01227").unwrap());
        let g = rpcs3.join("config").join("config.yml");
        let global = std::fs::read_to_string(&g).unwrap();
        std::fs::write(&g, global.replace("Video:\n", "Video:\n  Disable On-Disk Shader Cache: true\n")).unwrap();
        assert!(rpcs3_cache_shaders_coupe(rpcs3, "BLES01227").unwrap());
        let cc = rpcs3.join("config").join("custom_configs");
        std::fs::create_dir_all(&cc).unwrap();
        std::fs::write(cc.join("config_BLES01227.yml"), "Video:\n  Disable On-Disk Shader Cache: false\n").unwrap();
        assert!(!rpcs3_cache_shaders_coupe(rpcs3, "BLES01227").unwrap());
        std::fs::remove_dir_all(&cc).unwrap();
        std::fs::write(&g, global).unwrap();
        // Deux réglages changés : le fichier du jeu n'en contient QUE deux (le reste suit le global).
        rpcs3_regler_image(rpcs3, "BLES01227", &[("Resolution Scale".into(), "200".into()), ("Output Scaling Mode".into(), "FidelityFX Super Resolution".into())], false).unwrap();
        let f = rpcs3.join("config").join("custom_configs").join("config_BLES01227.yml");
        let t = std::fs::read_to_string(&f).unwrap();
        assert!(t.contains("Video:") && t.contains("  Resolution Scale: 200") && t.contains("  Output Scaling Mode: FidelityFX Super Resolution"));
        assert!(!t.contains("Renderer") && !t.contains("MSAA"), "{t}");
        let (v, par_jeu) = rpcs3_image(rpcs3, "BLES01227").unwrap();
        assert_eq!((val(&v, "Resolution Scale"), val(&v, "MSAA")), ("200".into(), "Auto".into()));
        assert_eq!(par_jeu.len(), 2);
        // Une valeur hors de la liste fermée : refusée, rien d'écrit.
        assert!(rpcs3_regler_image(rpcs3, "BLES01227", &[("Resolution Scale".into(), "800".into())], false).is_err());
        assert!(rpcs3_regler_image(rpcs3, "BLES01227", &[("Renderer".into(), "OpenGL".into())], false).is_err());
        // Un réglage fait à la main dans RPCS3 pour ce jeu reste ; « Valeurs de RPCS3 » ne retire que les nôtres.
        std::fs::write(&f, format!("{t}Miscellaneous:\n  Pause emulation on RPCS3 focus loss: true\n")).unwrap();
        rpcs3_regler_image(rpcs3, "BLES01227", &[], true).unwrap();
        let t = std::fs::read_to_string(&f).unwrap();
        assert!(!t.contains("Resolution Scale") && t.contains("Pause emulation on RPCS3 focus loss: true"), "{t}");
        assert!(f.is_file(), "le fichier du jeu n'est jamais effacé");
        // Un fichier qui n'avait que nos réglages n'est jamais laissé vide.
        std::fs::write(&f, "Video:\n  MSAA: Disabled\n").unwrap();
        rpcs3_regler_image(rpcs3, "BLES01227", &[], true).unwrap();
        let t = std::fs::read_to_string(&f).unwrap();
        assert!(!t.contains("MSAA") && t.contains("  Background input enabled: false"), "{t}");
        // Une section vidée perd son en-tête ; une section qui garde d'autres clés le garde.
        assert_eq!(retirer_cles_yaml("Video:\n  MSAA: Auto\n", "Video", &["MSAA"]).as_deref(), Some(""));
        assert_eq!(retirer_cles_yaml("Video:\n  MSAA: Auto\n  Renderer: Vulkan\nAudio:\n  X: 1\n", "Video", &["MSAA"]).as_deref(), Some("Video:\n  Renderer: Vulkan\nAudio:\n  X: 1\n"));
        // Relu comme RPCS3 le lirait (YAML valide).
        assert!(lire_cles_yaml("Video:\n  MSAA: \"Disabled\"\n", "Video", &["MSAA"])["MSAA"] == "Disabled");
    }

    #[test]
    fn mame_range_les_parties_de_chaque_profil_a_part_sans_rien_perdre() {
        let d = tempfile::tempdir().unwrap();
        let mame = d.path().join("MAME l'été");
        // Ce que MAME avait déjà (avant Frogtend) : un score, les touches de la personne, un état F7, un score du
        // greffon hiscore.
        for (f, c) in [("nvram/sf2/eeprom", "score"), ("cfg/default.cfg", "touches"), ("cfg/sf2.cfg", "sf2"), ("sta/sf2/1.sta", "etat"), ("hiscore/sf2.hi", "hi"), ("autofire/sf2.cfg", "tir")] {
            let p = mame.join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, c).unwrap();
        }
        // Le profil a déjà sauvé depuis le menu en jeu (0.79.0) : son dossier `sta` existe — la reprise se fait quand même.
        let deja = dossier_du_profil(&mame, "Zoé").join("sta").join("sf2");
        std::fs::create_dir_all(&deja).unwrap();
        std::fs::write(deja.join("frogtend.sta"), "menu").unwrap();
        let (args, notes) = preparer_mame_profil(&mame, "Zoé").unwrap();
        assert!(notes.is_empty(), "{notes:?}");
        let profil = dossier_du_profil(&mame, "Zoé");
        let valeur = |o: &str| args.iter().position(|a| a == o).map(|i| args[i + 1].clone());
        assert_eq!(valeur("-nvram_directory"), Some(texte(&profil.join("nvram"))));
        assert_eq!(valeur("-cfg_directory"), Some(texte(&profil.join("cfg"))));
        assert_eq!(valeur("-state_directory"), Some(texte(&profil.join("sta"))));
        assert_eq!(valeur("-diff_directory"), Some(texte(&profil.join("diff"))));
        assert_eq!(valeur("-homepath"), Some(texte(&profil.join("home"))));
        // La première fois : repris (copié), l'original intact.
        assert_eq!(std::fs::read_to_string(profil.join("nvram/sf2/eeprom")).unwrap(), "score");
        assert_eq!(std::fs::read_to_string(profil.join("cfg/default.cfg")).unwrap(), "touches");
        assert_eq!(std::fs::read_to_string(profil.join("sta/sf2/1.sta")).unwrap(), "etat");
        assert_eq!(std::fs::read_to_string(profil.join("home/hiscore/sf2.hi")).unwrap(), "hi");
        assert_eq!(std::fs::read_to_string(profil.join("home/autofire/sf2.cfg")).unwrap(), "tir");
        assert_eq!(std::fs::read_to_string(profil.join("sta/sf2/frogtend.sta")).unwrap(), "menu", "rien d'écrasé");
        assert_eq!(std::fs::read_to_string(mame.join("nvram/sf2/eeprom")).unwrap(), "score");
        // Ensuite, chaque profil a les siens : rien n'est recopié par-dessus.
        std::fs::write(profil.join("nvram/sf2/eeprom"), "score de Zoé").unwrap();
        std::fs::write(mame.join("nvram/sf2/eeprom"), "autre").unwrap();
        preparer_mame_profil(&mame, "Zoé").unwrap();
        assert_eq!(std::fs::read_to_string(profil.join("nvram/sf2/eeprom")).unwrap(), "score de Zoé");
        // Le dossier du profil est bien celui que la sauvegarde du profil emporte.
        assert!(profil.starts_with(mame.join("Profils")));
    }

    /// RÉEL (lot 0.80.0) : un vrai MAME lancé avec les dossiers d'un profil accentué (« Zoé l'été ») sur sa ligne de
    /// commande, quitté PROPREMENT après 5 s (il écrit alors cfg et nvram) : tout doit être dans le profil, rien dans
    /// les dossiers communs de MAME. `FROGTEND_MAME=<MAME d'essai>` ; `FROGTEND_JEU_MAME` (pong) ; `FROGTEND_ROMPATH`
    /// (lu seulement, pour un jeu à ROM).
    #[test]
    #[ignore]
    fn essai_mame_par_profil() {
        let mame = PathBuf::from(std::env::var("FROGTEND_MAME").unwrap());
        let jeu = std::env::var("FROGTEND_JEU_MAME").unwrap_or_else(|_| "pong".into());
        let rompath = std::env::var("FROGTEND_ROMPATH").unwrap_or_else(|_| "roms".into());
        let (args, notes) = preparer_mame_profil(&mame, "Zoé l'été").unwrap();
        println!("ARGUMENTS {args:?} ; NOTES {notes:?}");
        std::fs::create_dir_all(mame.join("frogtend")).unwrap();
        std::fs::write(mame.join("frogtend").join("sortie.lua"), "local n = 0\nemu.register_periodic(function() n = n + 1 if n == 300 then manager.machine:exit() end end)\n").unwrap();
        let mut a = args.clone();
        a.extend([jeu.clone(), "-rompath".into(), rompath, "-window".into(), "-nomax".into(), "-skip_gameinfo".into(), "-sound".into(), "none".into(), "-autoboot_script".into(), "frogtend\\sortie.lua".into()]);
        let mut p = std::process::Command::new(mame.join("mame.exe")).args(&a).current_dir(&mame).spawn().unwrap();
        let debut = std::time::Instant::now();
        let fin = loop {
            if let Some(s) = p.try_wait().unwrap() {
                break Some(s);
            }
            if debut.elapsed().as_secs() > 40 {
                let _ = p.kill();
                break None;
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        };
        println!("SORTIE {fin:?}");
        let profil = dossier_du_profil(&mame, "Zoé l'été");
        let mut pile = vec![profil.clone()];
        while let Some(x) = pile.pop() {
            for e in std::fs::read_dir(&x).unwrap().flatten() {
                if e.path().is_dir() {
                    pile.push(e.path());
                } else {
                    println!("DANS LE PROFIL : {}", e.path().strip_prefix(&profil).unwrap().display());
                }
            }
        }
        for commun in ["cfg", "nvram", "sta", "diff", "hiscore"] {
            println!("COMMUN {commun} : {:?}", std::fs::read_dir(mame.join(commun)).map(|l| l.flatten().map(|e| e.file_name()).collect::<Vec<_>>()).ok());
        }
        assert!(profil.join("cfg").join(format!("{jeu}.cfg")).is_file(), "le cfg du jeu va dans le profil");
        assert!(!mame.join("cfg").join(format!("{jeu}.cfg")).exists(), "rien dans le cfg commun");
    }

    #[test]
    fn mame_respecte_un_dossier_regle_par_la_personne_et_ne_reprend_pas_l_enorme() {
        let d = tempfile::tempdir().unwrap();
        let mame = d.path();
        std::fs::write(mame.join("mame.ini"), "#\nrompath                   roms\nnvram_directory           \"D:\\Mes scores\"\ncfg_directory             cfg\n").unwrap();
        assert_eq!(option_mame_ini("nvram_directory   \"D:\\x y\"\n", "nvram_directory").as_deref(), Some("D:\\x y"));
        // Un disque dur modifié de 300 Mo : pas repris dans le profil (noté), jamais bloquant.
        std::fs::create_dir_all(mame.join("diff")).unwrap();
        std::fs::File::create(mame.join("diff").join("jeu.dif")).unwrap().set_len(300 * 1024 * 1024).unwrap();
        let (args, notes) = preparer_mame_profil(mame, "Seb").unwrap();
        assert!(!args.contains(&"-nvram_directory".to_string()), "réglé par la personne : laissé");
        assert!(args.contains(&"-cfg_directory".to_string()), "valeur d'origine : par profil");
        assert!(notes.iter().any(|n| n.contains("nvram_directory")) && notes.iter().any(|n| n.contains("trop gros")), "{notes:?}");
        assert!(!dossier_du_profil(mame, "Seb").join("diff").join("jeu.dif").exists());
        assert!(mame.join("diff").join("jeu.dif").exists());
        // Trop gros : MAME garde SON dossier pour cette option (jamais un dossier vide à la place).
        assert!(!args.contains(&"-diff_directory".to_string()));
        // Sans `mame.ini` à la racine, celui de `ini\` compte : respecté.
        std::fs::remove_file(mame.join("mame.ini")).unwrap();
        std::fs::create_dir_all(mame.join("ini")).unwrap();
        std::fs::write(mame.join("ini").join("mame.ini"), "cfg_directory   D:\\Mes touches\n").unwrap();
        let (args, _) = preparer_mame_profil(mame, "Seb").unwrap();
        assert!(!args.contains(&"-cfg_directory".to_string()));
        // Un nom de profil que la ligne de MAME ne porterait pas : dossiers communs, noté.
        let (args, notes) = preparer_mame_profil(mame, "50%;x").unwrap();
        assert!(args.is_empty() && !notes.is_empty());
    }

    #[test]
    fn une_apostrophe_ou_une_virgule_dans_le_chemin_ne_casse_aucun_fichier_de_reglages() {
        let d = tempfile::tempdir().unwrap();
        let racine = d.path().join("Jeux d'émulation, essai");
        std::fs::create_dir_all(&racine).unwrap();
        let profil = "D'Aquin";

        // xemu : son disque dur d'origine dans un dossier à apostrophe ; le fichier du profil reste du TOML valide, et
        // le chemin y est exact.
        let disque = racine.join("system").join("xbox_hdd.qcow2");
        std::fs::create_dir_all(disque.parent().unwrap()).unwrap();
        std::fs::write(&disque, b"disque").unwrap();
        std::fs::write(racine.join("xemu.toml"), format!("[sys.files]\nhdd_path = {}\n", chaine_toml(&disque.display().to_string()))).unwrap();
        preparer("xemu", &racine, profil, &[], &Manette::Auto(None)).unwrap();
        let cfg = dossier_du_profil(&racine, profil).join("xemu.toml");
        let t = std::fs::read_to_string(&cfg).unwrap();
        let v: toml::Value = toml::from_str(&t).expect("le fichier de réglages de xemu reste lisible");
        let attendu = dossier_du_profil(&racine, profil).join("xbox_hdd.qcow2").display().to_string();
        assert_eq!(v["sys"]["files"]["hdd_path"].as_str(), Some(attendu.as_str()));
        assert_eq!(valeur_toml(&t, "sys.files", "hdd_path").as_deref(), Some(attendu.as_str()));
        // Une chaîne littérale écrite par xemu lui-même reste relue telle quelle.
        assert_eq!(valeur_toml("[a]\nb = 'E:\\x\\y'\n", "a", "b").as_deref(), Some("E:\\x\\y"));

        // Azahar portable : ses dossiers NAND et carte SD, entre guillemets (une virgule ferait une liste pour Qt).
        std::fs::create_dir_all(racine.join("user").join("config")).unwrap();
        preparer("azahar", &racine, profil, &[], &Manette::Auto(None)).unwrap();
        let ini = std::fs::read_to_string(racine.join("user/config/qt-config.ini")).unwrap();
        let nand = crate::manettes::lire_ini(&ini, "Data%20Storage", "nand_directory").pop().unwrap();
        assert!(nand.starts_with('"'), "{nand}");
        let attendu = format!("{}/", dossier_du_profil(&racine, profil).join("nand").display().to_string().replace('\\', "/"));
        assert_eq!(lire_valeur_qt(&nand), attendu);
    }

    #[test]
    fn les_valeurs_qt_et_toml_sont_echappees_et_relues() {
        assert_eq!(valeur_qt("E:/Zelda, TOTK/"), "\"E:/Zelda, TOTK/\"");
        assert_eq!(lire_valeur_qt(&valeur_qt("E:/a \"b\" \\c")), "E:/a \"b\" \\c");
        assert_eq!(lire_valeur_qt("E:/DLC"), "E:/DLC", "une valeur sans guillemets reste telle quelle");
        assert_eq!(chaine_toml("E:\\Jeux d'émulation"), "\"E:\\\\Jeux d'émulation\"");
    }

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
    fn rpcs3_n_ouvre_aucune_boite_et_le_reste_du_fichier_est_intact() {
        let avant = "[Logger]\r\nlevel=4\r\n\r\n[main_window]\r\ngeometry=@ByteArray(\\x1\\xd9\\xd0\\xcb\\0\\x3)\r\nconfirmationBoxExitGame=false\r\ninfoBoxEnabledWelcome=true\r\n\r\n[GSFrame]\r\nvisibility=FullScreen\r\n";
        let apres = regler_cles_qt(avant, "main_window", &[("infoBoxEnabledWelcome", "false"), ("confirmationBoxExitGame", "false")]).unwrap();
        // Une seule ligne a changé, tout le reste à l'identique (ligne binaire, fins de ligne Windows, autres sections).
        assert_eq!(apres, avant.replace("infoBoxEnabledWelcome=true", "infoBoxEnabledWelcome=false"));
        assert!(regler_cles_qt(&apres, "main_window", &[("infoBoxEnabledWelcome", "false")]).is_none(), "déjà réglé : rien");
        // Clé absente : ajoutée en tête de sa section ; section absente : ajoutée à la fin.
        let sans = "[main_window]\nvisibility=Windowed\n";
        assert_eq!(regler_cles_qt(sans, "main_window", &[("infoBoxEnabledWelcome", "false")]).unwrap(), "[main_window]\ninfoBoxEnabledWelcome=false\nvisibility=Windowed\n");
        assert_eq!(regler_cles_qt("[Logger]\nlevel=4\n", "main_window", &[("a", "1")]).unwrap(), "[Logger]\nlevel=4\n[main_window]\na=1\n");
        assert_eq!(regler_cles_qt("", "main_window", &[("a", "1")]).unwrap(), "[main_window]\na=1\n");
        // Une clé d'une AUTRE section du même nom n'est pas touchée.
        let autre = "[GSFrame]\ninfoBoxEnabledWelcome=true\n[main_window]\ninfoBoxEnabledWelcome=true\n";
        assert_eq!(regler_cles_qt(autre, "main_window", &[("infoBoxEnabledWelcome", "false")]).unwrap(), "[GSFrame]\ninfoBoxEnabledWelcome=true\n[main_window]\ninfoBoxEnabledWelcome=false\n");
        // Sur le disque : copie à l'abri, puis le fichier relu.
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("GuiConfigs")).unwrap();
        std::fs::write(d.path().join("GuiConfigs/CurrentSettings.ini"), avant).unwrap();
        assert!(rpcs3_sans_boites(d.path()).unwrap());
        assert!(std::fs::read_to_string(d.path().join("GuiConfigs/CurrentSettings.ini")).unwrap().contains("infoBoxEnabledWelcome=false\r\n"));
        assert!(!rpcs3_sans_boites(d.path()).unwrap(), "la 2e fois : rien à faire");
        assert!(d.path().join(".frogtend-sauvegardes").is_dir(), "l'original est à l'abri");
        // Un fichier illisible (octets non UTF-8) n'est jamais remplacé.
        let illisible = [b'[', b'm', b']', b'\n', 0xFF, 0xFE, b'\n'];
        std::fs::write(d.path().join("GuiConfigs/CurrentSettings.ini"), illisible).unwrap();
        assert!(rpcs3_sans_boites(d.path()).is_err());
        assert_eq!(std::fs::read(d.path().join("GuiConfigs/CurrentSettings.ini")).unwrap(), illisible);
    }

    #[test]
    fn rpcs3_se_met_en_pause_au_menu_et_ignore_la_manette_derriere_lui() {
        let avant = "Core:\n  PPU Threads: 2\nInput/Output:\n  Keyboard: \"Null\"\n  Background input enabled: true\n  Show move cursor: false\nSystem:\n  Background input enabled: true\nMiscellaneous:\n  Automatically start games after boot: true\n  Pause emulation on RPCS3 focus loss: false\n  Start games in fullscreen mode: true\n";
        let io = regler_cles_yaml(avant, "Input/Output", &[("Background input enabled", "false")]).unwrap();
        // Seule la ligne de la bonne section change (une clé du même nom ailleurs reste).
        assert_eq!(io, avant.replacen("Background input enabled: true", "Background input enabled: false", 1));
        assert!(regler_cles_yaml(&io, "Input/Output", &[("Background input enabled", "false")]).is_none());
        // Une sous-clé (4 espaces) n'est jamais prise pour une clé de la section.
        let sous = "Video:\n  Vulkan:\n    Adapter: x\n  Adapter: y\n";
        assert_eq!(regler_cles_yaml(sous, "Video", &[("Adapter", "z")]).unwrap(), "Video:\n  Vulkan:\n    Adapter: x\n  Adapter: z\n");
        // Clé absente : en tête de sa section ; section absente : à la fin ; fins de ligne Windows gardées.
        assert_eq!(regler_cles_yaml("A:\r\n  b: 1\r\n", "A", &[("c", "2")]).unwrap(), "A:\r\n  c: 2\r\n  b: 1\r\n");
        assert_eq!(regler_cles_yaml("A:\n  b: 1\n", "M", &[("c", "2")]).unwrap(), "A:\n  b: 1\nM:\n  c: 2\n");
        // Sur le disque : les deux lignes, l'original à l'abri, un fichier illisible jamais remplacé.
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("config/config.yml");
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(&f, avant).unwrap();
        assert!(rpcs3_pause_au_menu(d.path()).unwrap());
        let apres = std::fs::read_to_string(&f).unwrap();
        assert_eq!(
            apres,
            avant
                .replacen("Background input enabled: true", "Background input enabled: false", 1)
                .replace("Pause emulation on RPCS3 focus loss: false", "Pause emulation on RPCS3 focus loss: true")
        );
        assert!(!rpcs3_pause_au_menu(d.path()).unwrap(), "la 2e fois : rien à faire");
        assert!(d.path().join(".frogtend-sauvegardes").is_dir());
        let illisible = [b'A', b':', b'\n', 0xFF, 0xFE, b'\n'];
        std::fs::write(&f, illisible).unwrap();
        assert!(rpcs3_pause_au_menu(d.path()).is_err());
        assert_eq!(std::fs::read(&f).unwrap(), illisible);

        // Absent : créé avec les deux lignes ; mais pas s'il reste l'ancien config.yml à la racine.
        let neuf = tempfile::tempdir().unwrap();
        assert!(rpcs3_pause_au_menu(neuf.path()).unwrap());
        let t = std::fs::read_to_string(neuf.path().join("config/config.yml")).unwrap();
        assert!(t.contains("  Pause emulation on RPCS3 focus loss: true\n") && t.contains("  Background input enabled: false\n"));
        let ancien = tempfile::tempdir().unwrap();
        std::fs::write(ancien.path().join("config.yml"), "Core:\n").unwrap();
        assert!(!rpcs3_pause_au_menu(ancien.path()).unwrap());
        assert!(!ancien.path().join("config/config.yml").exists());

        // Un réglage par jeu reçoit les mêmes lignes (sinon il écraserait la pause) ; un autre fichier n'est pas touché.
        let par_jeu = neuf.path().join("config/custom_configs");
        std::fs::create_dir_all(&par_jeu).unwrap();
        std::fs::write(par_jeu.join("config_BLUS30508.yml"), avant).unwrap();
        std::fs::write(par_jeu.join("notes.txt"), "Background input enabled: true\n").unwrap();
        assert!(rpcs3_pause_au_menu(neuf.path()).unwrap());
        let j = std::fs::read_to_string(par_jeu.join("config_BLUS30508.yml")).unwrap();
        assert!(j.contains("  Pause emulation on RPCS3 focus loss: true\n") && j.contains("Input/Output:\n  Keyboard: \"Null\"\n  Background input enabled: false\n"));
        assert_eq!(std::fs::read_to_string(par_jeu.join("notes.txt")).unwrap(), "Background input enabled: true\n");
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
        // Les jeux en plusieurs disques : Dolphin change de disque seul…
        assert_eq!(crate::manettes::lire_ini(&ini, "Core", "AutoDiscChange"), vec!["True"]);
        // … sauf si la personne l'a désactivé : son choix est gardé.
        let f = PathBuf::from(&args[1]).join("Config").join("Dolphin.ini");
        std::fs::write(&f, ini.replace("AutoDiscChange = True", "AutoDiscChange = False")).unwrap();
        preparer("dolphin", d.path(), "Seb", &["E:\\Jeux\\GC".into()], &Manette::Auto(None)).unwrap();
        assert_eq!(crate::manettes::lire_ini(&std::fs::read_to_string(&f).unwrap(), "Core", "AutoDiscChange"), vec!["False"]);
    }

    #[test]
    fn mgba_range_les_parties_de_chaque_personne_dans_son_profil() {
        let d = tempfile::tempdir().unwrap();
        let args = preparer("mgba", d.path(), "Zoé", &[], &Manette::Auto(None)).unwrap();
        let p = dossier_du_profil(d.path(), "Zoé");
        assert_eq!(args.iter().filter(|a| *a == "-C").count(), 3);
        assert!(args.contains(&format!("savegamePath={}", texte(&p.join("saves")))), "{args:?}");
        assert!(args.contains(&format!("savestatePath={}", texte(&p.join("states")))));
        assert!(p.join("saves").is_dir() && p.join("states").is_dir());
        // Deux personnes : deux dossiers distincts (jamais les parties de l'une chez l'autre).
        let args2 = preparer("mgba", d.path(), "Léa", &[], &Manette::Auto(None)).unwrap();
        assert_ne!(args, args2);
    }

    #[test]
    fn rpcs3_est_reconnu_a_son_programme() {
        assert!(est_rpcs3(r"E:\Emulateurs\RPCS3\RPCS3.EXE"));
        assert!(est_rpcs3("C:/RPCS3/rpcs3.exe"));
        assert!(!est_rpcs3(r"E:\Emulateurs\RetroArch\retroarch.exe"));
    }

    #[test]
    fn mgba_reprend_la_partie_d_a_cote_de_la_rom_sans_jamais_l_ecraser() {
        let d = tempfile::tempdir().unwrap();
        let (emu, jeux) = (d.path().join("mGBA"), d.path().join("Jeux"));
        std::fs::create_dir_all(&jeux).unwrap();
        let rom = jeux.join("Golden Sun (Europe).gba");
        std::fs::write(&rom, b"ROM").unwrap();
        assert_eq!(reprendre_partie_mgba(&rom, &emu, "Zoé").unwrap(), None, "pas de partie : rien");
        std::fs::write(jeux.join("Golden Sun (Europe).sav"), b"partie d'avant").unwrap();
        let p = reprendre_partie_mgba(&rom, &emu, "Zoé").unwrap().unwrap();
        assert_eq!(p, dossier_du_profil(&emu, "Zoé").join("saves").join("Golden Sun (Europe).sav"));
        assert_eq!(std::fs::read(&p).unwrap(), b"partie d'avant");
        assert!(jeux.join("Golden Sun (Europe).sav").is_file(), "copiée, jamais déplacée");
        // La partie du profil a avancé : jamais écrasée par l'ancienne.
        std::fs::write(&p, b"partie de Zoe").unwrap();
        assert_eq!(reprendre_partie_mgba(&rom, &emu, "Zoé").unwrap(), None);
        assert_eq!(std::fs::read(&p).unwrap(), b"partie de Zoe");
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

        // xemu : réglages et disque dur copiés pour le profil. (Une apostrophe ou une virgule dans le chemin : voir
        // `une_apostrophe_ou_une_virgule_dans_le_chemin_ne_casse_aucun_fichier_de_reglages`.)
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

        // Eden : l'utilisateur Switch du nom du profil (pas portable : son nom, rien n'est écrit chez lui).
        assert_eq!(preparer("eden", racine, "Léa", &[], &Manette::Auto(None)).unwrap(), vec!["-u", "Léa"]);
        assert!(!crate::eden::fichier_profils(racine).exists());
        // Portable : l'utilisateur de Léa est créé (après « Eden »), puis passé par son index ; retrouvé ensuite.
        let eden = d.path().join("Eden");
        std::fs::create_dir_all(eden.join("user")).unwrap();
        assert_eq!(preparer("eden", &eden, "Léa", &[], &Manette::Auto(None)).unwrap(), vec!["-u", "1"]);
        assert_eq!(preparer("eden", &eden, "Seb", &[], &Manette::Auto(None)).unwrap(), vec!["-u", "2"]);
        assert_eq!(preparer("eden", &eden, "Léa", &[], &Manette::Auto(None)).unwrap(), vec!["-u", "1"]);

        // Cemu : pas portable, rien (comme avant) ; portable, le compte Wii U du profil (créé), par `-a`.
        let cemu = d.path().join("Cemu");
        std::fs::create_dir_all(&cemu).unwrap();
        assert!(preparer("cemu", &cemu, "Léa", &[], &Manette::Auto(None)).unwrap().is_empty());
        std::fs::create_dir_all(cemu.join("portable")).unwrap();
        assert_eq!(preparer("cemu", &cemu, "Léa", &[], &Manette::Auto(None)).unwrap(), vec!["-a", "80000001"]);
        assert_eq!(preparer("cemu", &cemu, "Seb", &[], &Manette::Auto(None)).unwrap(), vec!["-a", "80000002"]);
        assert_eq!(crate::cemu::compte_existant(&cemu, "Léa"), Some(0x8000_0001));

        // Azahar : rien tant qu'il n'est pas portable (il ne bascule jamais dans le dos de la personne) ; portable :
        // NAND et carte SD du profil.
        preparer("azahar", racine, "Seb", &[], &Manette::Auto(None)).unwrap();
        assert!(!racine.join("user").exists());
        std::fs::create_dir_all(racine.join("user")).unwrap();
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
    fn une_partie_pcsx2_retire_l_ancien_menu_a_la_manette() {
        let d = tempfile::tempdir().unwrap();
        let ini = d.path().join("inis").join("PCSX2.ini");
        std::fs::create_dir_all(ini.parent().unwrap()).unwrap();
        std::fs::write(&ini, "[Pad1]\r\nType = DualShock2\r\nCross = SDL-0/A\r\n\r\n[Hotkeys]\r\nOpenPauseMenu = Keyboard/Escape\r\nOpenPauseMenu = SDL-0/Back & SDL-0/Start\r\n").unwrap();
        preparer("pcsx2", d.path(), "Seb", &[], &Manette::Auto(None)).unwrap();
        let t = std::fs::read_to_string(&ini).unwrap();
        assert_eq!(crate::manettes::lire_ini(&t, "Hotkeys", "OpenPauseMenu"), vec!["Keyboard/Escape"]);
        assert_eq!(crate::manettes::lire_ini(&t, "Pad1", "Cross"), vec!["SDL-0/A"], "la manette de la personne reste");
    }

    #[test]
    fn un_ini_illisible_n_est_jamais_remplace() {
        let d = tempfile::tempdir().unwrap();
        let ini = d.path().join("settings.ini");
        let illisible = [b'[', b'M', b']', b'\n', 0xFF, 0xFE, b'\n'];
        std::fs::write(&ini, illisible).unwrap();
        assert!(modifier_ini(d.path(), &ini, "Main", &[("A", vec!["1".into()])]).is_err());
        assert_eq!(std::fs::read(&ini).unwrap(), illisible);
    }

    #[test]
    fn le_choix_des_commandes_du_jeu_est_suivi() {
        // Clavier et souris : aucune manette réglée ; RetroArch n'a jamais de menu à la manette (un seul menu : Frogtend).
        let d = tempfile::tempdir().unwrap();
        preparer("duckstation", d.path(), "Seb", &[], &Manette::Clavier).unwrap();
        let ini = std::fs::read_to_string(d.path().join("settings.ini")).unwrap();
        assert!(!ini.contains("SDL-0"));
        preparer("retroarch", d.path(), "Seb", &[], &Manette::Clavier).unwrap();
        let cfg = std::fs::read_to_string(d.path().join("Profils").join("Seb").join("frogtend.cfg")).unwrap();
        assert!(cfg.contains("input_menu_toggle_gamepad_combo = \"0\""));
        preparer("retroarch", d.path(), "Seb", &[], &Manette::Auto(None)).unwrap();
        let cfg = std::fs::read_to_string(d.path().join("Profils").join("Seb").join("frogtend.cfg")).unwrap();
        assert_eq!(cfg.matches("input_menu_toggle_gamepad_combo").count(), 1);
        assert!(cfg.contains("input_menu_toggle_gamepad_combo = \"0\""));

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

#[cfg(test)]
mod essai_reel_rpcs3 {
    /// Sur une COPIE du vrai fichier de RPCS3 : `FROGTEND_RPCS3=<dossier> cargo test --lib copie_du_vrai -- --ignored`.
    #[test]
    #[ignore]
    fn copie_du_vrai_fichier_de_rpcs3() {
        let vrai = std::path::PathBuf::from(std::env::var("FROGTEND_RPCS3").unwrap()).join("GuiConfigs/CurrentSettings.ini");
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("GuiConfigs")).unwrap();
        std::fs::copy(&vrai, d.path().join("GuiConfigs/CurrentSettings.ini")).unwrap();
        let avant = std::fs::read(&vrai).unwrap();
        println!("RPCS3 réglé : {}", super::rpcs3_sans_boites(d.path()).unwrap());
        let apres = std::fs::read(d.path().join("GuiConfigs/CurrentSettings.ini")).unwrap();
        let (a, b) = (String::from_utf8_lossy(&avant), String::from_utf8_lossy(&apres));
        let differences: Vec<(&str, &str)> = a.lines().zip(b.lines()).filter(|(x, y)| x != y).collect();
        println!("RPCS3 lignes : {} avant, {} après ; différences : {:?}", a.lines().count(), b.lines().count(), differences);
        assert_eq!(std::fs::read(&vrai).unwrap(), avant, "le vrai fichier n'est pas touché");
    }

    /// Même essai pour `config\config.yml` (pause au menu).
    #[test]
    #[ignore]
    fn copie_du_vrai_config_yml_de_rpcs3() {
        let vrai = std::path::PathBuf::from(std::env::var("FROGTEND_RPCS3").unwrap()).join("config/config.yml");
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("config")).unwrap();
        std::fs::copy(&vrai, d.path().join("config/config.yml")).unwrap();
        let avant = std::fs::read(&vrai).unwrap();
        println!("RPCS3 réglé : {}", super::rpcs3_pause_au_menu(d.path()).unwrap());
        let apres = std::fs::read(d.path().join("config/config.yml")).unwrap();
        let (a, b) = (String::from_utf8_lossy(&avant), String::from_utf8_lossy(&apres));
        let differences: Vec<(&str, &str)> = a.lines().zip(b.lines()).filter(|(x, y)| x != y).collect();
        println!("RPCS3 lignes : {} avant, {} après ; octets {} / {} ; différences : {:?}", a.lines().count(), b.lines().count(), avant.len(), apres.len(), differences);
        assert_eq!(std::fs::read(&vrai).unwrap(), avant, "le vrai fichier n'est pas touché");
    }
}
