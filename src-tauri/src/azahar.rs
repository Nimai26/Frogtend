//! Azahar (3DS) : la langue de la console dès qu'elle est connue (règle de Seb, 08/11 : « configurer TOUS les
//! émulateurs […] à chaque nouvelle installation »). Relevé dans le code d'Azahar (master, 11/11/2026) :
//! - la langue n'est pas dans `qt-config.ini` : elle est dans la sauvegarde système de la console (module CFG,
//!   core/hle/service/cfg/cfg.cpp), le fichier `config` de l'archive 0x00010017 :
//!   `<NAND>/data/00000000000000000000000000000000/sysdata/00010017/00000000/config` (fs/archive.h `SYSTEM_ID`,
//!   file_sys/archive_systemsavedata.cpp `GetSystemSaveDataPath`) ; la NAND est celle du profil Frogtend
//!   (`[Data Storage] nand_directory`, emulateurs_profils.rs) ;
//! - Azahar le crée au démarrage du module CFG (`LoadConfigNANDSaveFile` → `FormatConfig`) avec ses valeurs d'origine
//!   (cfg_defaults.cpp : `DEFAULT_LANGUAGE = LANGUAGE_EN`) et un identifiant de console tiré au hasard : Frogtend ne le
//!   crée JAMAIS ; la langue est posée à la partie suivante ;
//! - le fichier fait 0x8000 octets : entête `u16` nombre d'entrées, `u16` 0x455C, puis jusqu'à 1 479 entrées de 12
//!   octets {`u32` bloc, `u32` donnée (EN LIGNE si la taille ≤ 4) ou adresse, `u16` taille, `u16` droits}
//!   (cfg.cpp `SaveConfigBlockEntry`, `GetConfigBlockPointer`) ; aucune somme de contrôle ; la langue est le bloc
//!   0x000A0002 (cfg.h `LanguageBlockID`), un octet ;
//! - les langues (cfg.h `SystemLanguage`) : japonais 0, anglais 1, français 2, allemand 3, italien 4, espagnol 5,
//!   chinois simplifié 6, coréen 7, néerlandais 8, portugais 9, russe 10, chinois traditionnel 11 ; une langue que la
//!   région du jeu n'a pas est ajustée par Azahar lui-même, en mémoire (`UpdatePreferredRegionCode`).
//!
//! La manette (citra_qt/configuration/config.cpp `ReadControlValues`, input_common/sdl/sdl_impl.cpp) :
//! - les touches d'origine sont au CLAVIER : une manette ne fait rien tant qu'on ne l'a pas réglée ;
//! - `[Controls]` : tableau Qt `profiles` (`profiles\size`, `profiles\<n>\button_a`…, base 1), `profile` = le profil
//!   courant (base 0) ; une clé sans `\default=false` garde sa valeur d'origine (QSettings) ;
//! - `engine:sdl,api:controller,maptype:all` : TOUTE manette (le GUID est ignoré, sdl_impl.cpp `Create`), Xbox ou
//!   PlayStation, rangée par position (API GameController de SDL) ;
//! - la disposition que Frogtend écrit est celle de la configuration automatique d'Azahar pour une manette Xbox
//!   (`xinput_to_3ds_mapping`) : par POSITION comme sur la 3DS (son A, à droite, est le B de la manette Xbox), sauf le
//!   bouton Home, laissé au clavier : le bouton Xbox ouvre le menu de Frogtend.

use crate::erreurs::{Erreur, Resultat};
use std::path::{Path, PathBuf};

/// La taille du fichier `config` de la console.
const TAILLE_CONFIG: usize = 0x8000;
/// Le nombre maximal d'entrées (cfg.cpp `CONFIG_FILE_MAX_BLOCK_ENTRIES`).
const ENTREES_MAX: usize = 1479;
/// Le bloc de la langue.
const BLOC_LANGUE: u32 = 0x000A_0002;
/// L'anglais : la valeur d'origine d'Azahar.
const ANGLAIS: u8 = 1;

/// Azahar installé en portable (un dossier `user` à côté du programme) ? Sinon : jamais touché.
pub fn portable(azahar: &Path) -> bool {
    azahar.join("user").is_dir()
}

/// Le fichier `config` de la console dans une NAND.
pub fn fichier_config(nand: &Path) -> PathBuf {
    nand.join("data").join("00000000000000000000000000000000").join("sysdata").join("00010017").join("00000000").join("config")
}

/// La langue de la console 3DS pour une langue de Windows (LANGID) ; `None` : une langue que la console n'a pas.
pub fn langue_console(langid: u16) -> Option<u8> {
    Some(match langid & 0x3FF {
        0x11 => 0,
        0x09 => 1,
        0x0C => 2,
        0x07 => 3,
        0x10 => 4,
        0x0A => 5,
        // Chinois : traditionnel à Taïwan, Hong Kong et Macao (et le traditionnel neutre), simplifié ailleurs.
        0x04 if matches!(langid, 0x0404 | 0x0C04 | 0x1404 | 0x7C04) => 11,
        0x04 => 6,
        0x12 => 7,
        0x13 => 8,
        0x16 => 9,
        0x19 => 10,
        _ => return None,
    })
}

/// L'octet de la langue dans le fichier `config` (la donnée en ligne de son entrée), s'il a la forme attendue.
fn place_langue(octets: &[u8]) -> Option<usize> {
    if octets.len() != TAILLE_CONFIG {
        return None;
    }
    let total = u16::from_le_bytes([octets[0], octets[1]]) as usize;
    if total > ENTREES_MAX || u16::from_le_bytes([octets[2], octets[3]]) != 0x455C {
        return None;
    }
    (0..total).map(|i| 4 + 12 * i).find_map(|e| {
        let bloc = u32::from_le_bytes([octets[e], octets[e + 1], octets[e + 2], octets[e + 3]]);
        let taille = u16::from_le_bytes([octets[e + 8], octets[e + 9]]);
        (bloc == BLOC_LANGUE && taille == 1).then_some(e + 4)
    })
}

/// La langue de la console prise de Windows, UNE seule fois par profil (marque dans le dossier du profil) :
/// seulement Azahar portable et FERMÉ, seulement un fichier `config` de la forme connue, encore en anglais. Absent :
/// rien (Azahar le crée à sa première partie ; la langue sera posée à la suivante). L'ancien est copié à l'abri ;
/// écrit par un provisoire renommé, puis relu. `profil` : le dossier du profil (sa NAND est `profil\nand`). Rend vrai
/// si la langue a été posée.
pub fn regler_langue(azahar: &Path, profil: &Path, langid: u16) -> Resultat<bool> {
    let marque = profil.join(".frogtend-langue");
    if !portable(azahar) || marque.exists() || !crate::lancement::processus_du_dossier(azahar).is_empty() {
        return Ok(false);
    }
    let f = fichier_config(&profil.join("nand"));
    // La taille d'abord (jamais lire un fichier inattendu en entier).
    match std::fs::metadata(&f) {
        Ok(m) if m.len() == TAILLE_CONFIG as u64 => {}
        Ok(_) => return Ok(false),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.into()),
    }
    let octets = std::fs::read(&f)?;
    let Some(i) = place_langue(&octets) else { return Ok(false) };
    let mut pose = false;
    if let Some(n) = langue_console(langid).filter(|n| *n != ANGLAIS && octets[i] == ANGLAIS) {
        let mut nouveau = octets.clone();
        nouveau[i] = n;
        crate::emulateurs_profils::mettre_a_l_abri(azahar, &f)?;
        let provisoire = f.with_extension("frogtend-tmp");
        std::fs::write(&provisoire, &nouveau)?;
        if let Err(e) = std::fs::rename(&provisoire, &f) {
            let _ = std::fs::remove_file(&provisoire);
            return Err(e.into());
        }
        if std::fs::read(&f).ok().as_deref() != Some(nouveau.as_slice()) {
            return Err(Erreur::Disque("La langue d'Azahar n'a pas pu être posée.".into()));
        }
        pose = true;
    }
    std::fs::write(marque, "")?;
    Ok(pose)
}

/// Les réglages Qt d'Azahar (portable : à côté du programme, comme ceux d'Eden).
fn fichier_reglages(azahar: &Path) -> PathBuf {
    crate::eden::fichier_reglages(azahar)
}

/// Une touche sur toute manette (bouton SDL GameController).
fn bouton(n: u8) -> String {
    format!("api:controller,button:{n},engine:sdl,guid:0,maptype:all,port:0")
}

/// La disposition de Frogtend (clé dans le profil → valeur, sans guillemets) : boutons SDL A 0, B 1, X 2, Y 3, Back 4,
/// Start 6, LB 9, RB 10, croix 11-14 ; gâchettes = axes 4 et 5 ; sticks = axes 0/1 et 2/3.
fn disposition() -> Vec<(&'static str, String)> {
    let gachette = |a: u8| format!("api:controller,axis:{a},direction:+,engine:sdl,guid:0,maptype:all,port:0,threshold:0.500000");
    let stick = |x: u8, y: u8| format!("api:controller,axis_x:{x},axis_y:{y},engine:sdl,guid:0,maptype:all,port:0");
    vec![
        ("button_a", bouton(1)),
        ("button_b", bouton(0)),
        ("button_x", bouton(3)),
        ("button_y", bouton(2)),
        ("button_up", bouton(11)),
        ("button_down", bouton(12)),
        ("button_left", bouton(13)),
        ("button_right", bouton(14)),
        ("button_l", bouton(9)),
        ("button_r", bouton(10)),
        ("button_start", bouton(6)),
        ("button_select", bouton(4)),
        ("button_zl", gachette(4)),
        ("button_zr", gachette(5)),
        ("circle_pad", stick(0, 1)),
        ("c_stick", stick(2, 3)),
    ]
}

/// La valeur d'une clé de `[Controls]` (sans guillemets), telle qu'Azahar la prend : `None` si sa ligne `\default`
/// vaut true (QSettings garde alors la valeur d'origine : le clavier pour une touche).
fn valeur(texte: &str, cle: &str) -> Option<String> {
    let lue = |c: &str| crate::manettes::lire_ini(texte, "Controls", c).first().map(|v| crate::emulateurs_profils::lire_valeur_qt(v));
    if lue(&format!("{cle}\\default")).as_deref() == Some("true") {
        return None;
    }
    lue(cle)
}

/// Le préfixe des clés du profil courant (`profiles\<n>\`, base 1), si ce profil existe (ou si aucun n'existe encore :
/// le premier, qu'Azahar créera d'après ces clés).
fn profil_courant(texte: &str) -> Option<(String, bool)> {
    let courant: usize = valeur(texte, "profile").and_then(|v| v.parse().ok()).unwrap_or(0);
    match valeur(texte, "profiles\\size").and_then(|v| v.parse::<usize>().ok()) {
        Some(n) if courant < n => Some((format!("profiles\\{}\\", courant + 1), false)),
        None | Some(0) if courant == 0 => Some(("profiles\\1\\".into(), true)),
        _ => None,
    }
}

/// Le joueur 1 est-il encore au clavier (le réglage d'origine d'Azahar, ou rien d'écrit) ?
fn au_clavier(texte: &str, prefixe: &str) -> bool {
    valeur(texte, &format!("{prefixe}button_a")).is_none_or(|v| v.is_empty() || v.contains("engine:keyboard"))
}

/// Le profil courant a-t-il exactement la disposition de Frogtend ?
fn de_frogtend(texte: &str, prefixe: &str) -> bool {
    disposition().iter().all(|(c, v)| valeur(texte, &format!("{prefixe}{c}")).as_deref() == Some(v.as_str()))
}

/// La disposition dessinée dans le menu est-elle vraie ? Celle de Frogtend, ou un joueur 1 encore au clavier dans un
/// Azahar portable (Frogtend l'écrira au lancement). Hors portable (Frogtend n'écrit rien) ou une manette réglée par
/// la personne : non.
pub fn disposition_de_frogtend(azahar: &Path) -> bool {
    if !portable(azahar) {
        return false;
    }
    let texte = crate::eden::lire_reglages(azahar).unwrap_or_default();
    profil_courant(&texte).is_some_and(|(p, _)| au_clavier(&texte, &p) || de_frogtend(&texte, &p))
}

/// Le bouton Xbox est-il libre pour le menu de Frogtend ? Oui, sauf si le bouton Home du profil courant est sur une
/// manette, ou si la personne a posé des raccourcis à la manette (`controller_keyseq`) : alors il reste à Azahar.
/// Lu dans les réglages d'Azahar où qu'ils soient (portable, ou `%APPDATA%\Azahar`) ; absents : libre.
pub fn guide_libre(azahar: &Path) -> bool {
    let f = if portable(azahar) {
        fichier_reglages(azahar)
    } else {
        match std::env::var_os("APPDATA") {
            Some(a) => PathBuf::from(a).join("Azahar").join("config").join("qt-config.ini"),
            None => return true,
        }
    };
    // Un fichier anormalement gros n'est pas celui d'Azahar : on ne sait pas, le bouton reste à Azahar.
    match std::fs::metadata(&f) {
        Ok(m) if m.len() > 8 * 1024 * 1024 => return false,
        Ok(_) => {}
        Err(_) => return true,
    }
    let Ok(texte) = std::fs::read_to_string(f) else { return true };
    let home_sur_manette = profil_courant(&texte).is_none_or(|(p, _)| valeur(&texte, &format!("{p}button_home")).is_some_and(|v| v.contains("engine:sdl")));
    // Les raccourcis d'origine d'Azahar n'ont aucune touche de manette (config.cpp `default_hotkeys`) ; un
    // `\default=true` garde cette valeur vide.
    let lignes: Vec<(&str, &str)> = texte.lines().filter_map(|l| l.split_once('=')).collect();
    let raccourcis = lignes.iter().any(|(c, v)| {
        c.ends_with("controller_keyseq") && !crate::emulateurs_profils::lire_valeur_qt(v).is_empty() && !lignes.iter().any(|(c2, v2)| *c2 == format!("{c}\\default") && v2.trim() == "true")
    });
    !home_sur_manette && !raccourcis
}

/// La manette du joueur 1 : la disposition de Frogtend (toute manette), seulement Azahar portable et FERMÉ, et
/// seulement si le joueur 1 est encore au clavier, déjà à Frogtend, ou à la demande de la personne (`forcer` :
/// « Remettre la manette par défaut ») — jamais par-dessus une manette qu'elle a réglée elle-même. Rend vrai si le
/// joueur 1 a la disposition de Frogtend.
pub fn regler_manette(azahar: &Path, forcer: bool) -> Resultat<bool> {
    if !crate::eden::reglages_modifiables(azahar) {
        return Ok(false);
    }
    let texte = crate::eden::lire_reglages(azahar)?;
    let Some((p, nouveau)) = profil_courant(&texte) else { return Ok(false) };
    if de_frogtend(&texte, &p) {
        return Ok(true);
    }
    if !forcer && !au_clavier(&texte, &p) {
        return Ok(false);
    }
    let mut v: Vec<(String, String)> = disposition().into_iter().map(|(c, x)| (format!("{p}{c}"), crate::emulateurs_profils::valeur_qt(&x))).collect();
    // Toute manette : le choix « Toutes les manettes » de la fenêtre d'Azahar, pour qu'elle le montre et le garde.
    v.push((format!("{p}input_maptype"), "0".into()));
    if nouveau {
        v.push(("profiles\\size".into(), "1".into()));
    }
    let v: Vec<(&str, String)> = v.iter().map(|(c, x)| (c.as_str(), x.clone())).collect();
    crate::eden::ecrire_qt(azahar, &texte, "Controls", &v)?;
    Ok(true)
}

/// « Clavier et souris » choisi pour ce jeu : la disposition de Frogtend (seulement elle) rendue aux réglages
/// d'origine d'Azahar (`\default=true` : Qt reprend alors le clavier). Rend vrai si le fichier a changé.
pub fn rendre_le_clavier(azahar: &Path) -> Resultat<bool> {
    if !crate::eden::reglages_modifiables(azahar) {
        return Ok(false);
    }
    let texte = crate::eden::lire_reglages(azahar)?;
    let Some((p, _)) = profil_courant(&texte).filter(|(p, _)| de_frogtend(&texte, p)) else { return Ok(false) };
    let defauts: Vec<String> = disposition().iter().map(|(c, _)| format!("{p}{c}\\default")).collect();
    let v: Vec<(&str, &str)> = defauts.iter().map(|d| (d.as_str(), "true")).collect();
    let Some(apres) = crate::emulateurs_profils::regler_cles_qt(&texte, "Controls", &v) else { return Ok(false) };
    crate::emulateurs_profils::remplacer_config(azahar, &fichier_reglages(azahar), &apres)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_manette_d_azahar_seulement_au_clavier_ou_a_frogtend() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path();
        // Pas portable : rien.
        assert!(!regler_manette(a, false).unwrap());
        assert!(!disposition_de_frogtend(a));
        // Installation neuve (pas encore de réglages) : le premier profil, créé avec sa taille.
        std::fs::create_dir_all(a.join("user")).unwrap();
        assert!(disposition_de_frogtend(a), "Frogtend l'écrira au lancement");
        assert!(regler_manette(a, false).unwrap());
        let t = std::fs::read_to_string(fichier_reglages(a)).unwrap();
        assert!(t.contains("profiles\\size=1"), "{t}");
        assert!(t.contains("profiles\\1\\button_a\\default=false"), "{t}");
        assert!(t.contains("profiles\\1\\button_a=\"api:controller,button:1,engine:sdl,guid:0,maptype:all,port:0\""), "entre guillemets : {t}");
        assert!(t.contains("profiles\\1\\input_maptype=0"));
        assert!(!t.contains("button_home"), "le bouton Home reste au clavier");
        assert!(de_frogtend(&t, "profiles\\1\\") && disposition_de_frogtend(a) && guide_libre(a));
        // Déjà à Frogtend : rien de réécrit.
        assert!(regler_manette(a, false).unwrap());
        assert_eq!(std::fs::read_to_string(fichier_reglages(a)).unwrap(), t);
        // « Clavier et souris » : rendu au clavier d'origine, et plus dessiné comme la disposition de Frogtend.
        assert!(rendre_le_clavier(a).unwrap());
        let t2 = std::fs::read_to_string(fichier_reglages(a)).unwrap();
        assert!(t2.contains("profiles\\1\\button_a\\default=true"), "{t2}");
        assert!(!de_frogtend(&t2, "profiles\\1\\"), "le clavier d'origine reprend");
        // Revenir à la manette ensuite : réécrite.
        assert!(regler_manette(a, false).unwrap());
        assert!(de_frogtend(&std::fs::read_to_string(fichier_reglages(a)).unwrap(), "profiles\\1\\"));
    }

    #[test]
    fn la_manette_reglee_par_la_personne_ou_un_autre_profil_ne_sont_jamais_touches() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path();
        std::fs::create_dir_all(a.join("user").join("config")).unwrap();
        // Sa manette à elle (un GUID précis), Home sur le bouton Xbox : gardée, rien de dessiné, bouton Xbox à Azahar.
        let sienne = "[Controls]\nprofile=0\nprofiles\\size=1\nprofiles\\1\\button_a=\"button:1,engine:sdl,guid:030000005e040000,port:0\"\nprofiles\\1\\button_home=\"button:5,engine:sdl,guid:030000005e040000,port:0\"\n";
        std::fs::write(fichier_reglages(a), sienne).unwrap();
        assert!(!regler_manette(a, false).unwrap());
        assert_eq!(std::fs::read_to_string(fichier_reglages(a)).unwrap(), sienne);
        assert!(!disposition_de_frogtend(a));
        assert!(!guide_libre(a));
        // À sa demande (« Remettre la manette par défaut ») : remplacée.
        assert!(regler_manette(a, true).unwrap());
        assert!(disposition_de_frogtend(a));
        // Un autre profil courant (le 2e) : c'est lui qui est réglé, s'il est au clavier.
        let deux = "[Controls]\nprofile=1\nprofiles\\size=2\nprofiles\\1\\button_a=\"button:1,engine:sdl,guid:x,port:0\"\nprofiles\\2\\button_a=\"code:65,engine:keyboard\"\n";
        std::fs::write(fichier_reglages(a), deux).unwrap();
        assert!(regler_manette(a, false).unwrap());
        let t = std::fs::read_to_string(fichier_reglages(a)).unwrap();
        assert!(t.contains("profiles\\2\\button_b=") && t.contains("profiles\\1\\button_a=\"button:1,engine:sdl,guid:x,port:0\""), "{t}");
        assert!(!t.contains("profiles\\size=1"));
        // Un index courant hors du tableau : on ne sait pas, rien.
        let hors = "[Controls]\nprofile=3\nprofiles\\size=1\n";
        std::fs::write(fichier_reglages(a), hors).unwrap();
        assert!(!regler_manette(a, false).unwrap());
        assert!(!guide_libre(a));
        // Des raccourcis à la manette posés par la personne : le bouton Xbox reste à Azahar.
        std::fs::write(fichier_reglages(a), "[Shortcuts]\nshortcuts\\main_window\\fullscreen\\controller_keyseq=Home+A\n").unwrap();
        assert!(!guide_libre(a));
        std::fs::write(fichier_reglages(a), "[Shortcuts]\nshortcuts\\main_window\\fullscreen\\controller_keyseq=\n").unwrap();
        assert!(guide_libre(a));
        std::fs::write(fichier_reglages(a), "[Shortcuts]\nshortcuts\\main_window\\fullscreen\\controller_keyseq\\default=true\nshortcuts\\main_window\\fullscreen\\controller_keyseq=Home+A\n").unwrap();
        assert!(guide_libre(a), "valeur d'origine gardée par Qt");
    }

    /// Un fichier `config` comme Azahar le crée : quelques blocs avant la langue (dont un de plus de 4 octets, rangé
    /// plus loin), la langue en ligne.
    fn config_azahar(langue: u8) -> Vec<u8> {
        let mut o = vec![0u8; TAILLE_CONFIG];
        let entrees: [(u32, u32, u16); 3] = [(0x0005_0005, 0x455C, 8), (BLOC_LANGUE, langue as u32, 1), (0x000B_0000, 0x0000_4E46, 4)];
        o[0..2].copy_from_slice(&(entrees.len() as u16).to_le_bytes());
        o[2..4].copy_from_slice(&0x455Cu16.to_le_bytes());
        for (n, (bloc, donnee, taille)) in entrees.iter().enumerate() {
            let e = 4 + 12 * n;
            o[e..e + 4].copy_from_slice(&bloc.to_le_bytes());
            o[e + 4..e + 8].copy_from_slice(&donnee.to_le_bytes());
            o[e + 8..e + 10].copy_from_slice(&taille.to_le_bytes());
            o[e + 10..e + 12].copy_from_slice(&0x000Eu16.to_le_bytes());
        }
        o[0x455C..0x4564].copy_from_slice(&[9u8; 8]); // la donnée du premier bloc : jamais touchée
        o
    }

    fn azahar_portable() -> (tempfile::TempDir, PathBuf) {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("user")).unwrap();
        let p = d.path().join("Profils").join("Seb");
        std::fs::create_dir_all(&p).unwrap();
        (d, p)
    }

    fn poser_config(p: &Path, octets: &[u8]) -> PathBuf {
        let f = fichier_config(&p.join("nand"));
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(&f, octets).unwrap();
        f
    }

    #[test]
    fn la_langue_d_azahar_une_seule_fois_par_profil_et_jamais_par_dessus_un_choix() {
        let (d, p) = azahar_portable();
        // Pas encore de fichier (aucune partie) : rien, et réessayé la fois suivante.
        assert!(!regler_langue(d.path(), &p, 0x040C).unwrap());
        assert!(!p.join(".frogtend-langue").exists());
        // Créé par Azahar, en anglais : le français posé, le reste intact, l'ancien à l'abri.
        let f = poser_config(&p, &config_azahar(1));
        assert!(regler_langue(d.path(), &p, 0x040C).unwrap());
        assert_eq!(std::fs::read(&f).unwrap(), config_azahar(2));
        assert!(d.path().join(".frogtend-sauvegardes").is_dir());
        assert!(!f.with_extension("frogtend-tmp").exists(), "aucun provisoire laissé");
        // Une seule fois : l'anglais remis par la personne est gardé.
        std::fs::write(&f, config_azahar(1)).unwrap();
        assert!(!regler_langue(d.path(), &p, 0x040C).unwrap());
        assert_eq!(std::fs::read(&f).unwrap(), config_azahar(1));
        // Un autre profil a sa propre console : réglé à son tour.
        let p2 = d.path().join("Profils").join("Lea");
        let f2 = poser_config(&p2, &config_azahar(1));
        assert!(regler_langue(d.path(), &p2, 0x040C).unwrap());
        assert_eq!(std::fs::read(&f2).unwrap(), config_azahar(2));
    }

    #[test]
    fn windows_en_anglais_ou_une_langue_absente_de_la_console_ne_changent_rien() {
        for langid in [0x0409u16, 0x0415] {
            let (d, p) = azahar_portable();
            let f = poser_config(&p, &config_azahar(1));
            assert!(!regler_langue(d.path(), &p, langid).unwrap());
            assert_eq!(std::fs::read(&f).unwrap(), config_azahar(1), "{langid:#06x}");
            assert!(p.join(".frogtend-langue").exists(), "fait une fois : plus réessayé");
        }
    }

    #[test]
    fn un_autre_choix_une_autre_forme_ou_hors_portable_ne_sont_jamais_touches() {
        // Allemand choisi.
        let (d, p) = azahar_portable();
        let f = poser_config(&p, &config_azahar(3));
        assert!(!regler_langue(d.path(), &p, 0x040C).unwrap());
        assert_eq!(std::fs::read(&f).unwrap(), config_azahar(3));
        // Une autre taille, un entête inconnu, une langue de taille inattendue.
        let (e, q) = azahar_portable();
        poser_config(&q, &[0u8; 100]);
        assert!(!regler_langue(e.path(), &q, 0x040C).unwrap());
        let mut autre = config_azahar(1);
        autre[2] = 0;
        let g = poser_config(&q, &autre);
        assert!(!regler_langue(e.path(), &q, 0x040C).unwrap());
        assert_eq!(std::fs::read(&g).unwrap(), autre);
        let mut large = config_azahar(1);
        large[4 + 12 + 8] = 2;
        assert_eq!(place_langue(&large), None);
        assert_eq!(place_langue(&config_azahar(1)), Some(4 + 12 + 4));
        // Hors portable (données dans %APPDATA%) : jamais touché.
        let n = tempfile::tempdir().unwrap();
        let r = n.path().join("Profils").join("Seb");
        let h = poser_config(&r, &config_azahar(1));
        assert!(!regler_langue(n.path(), &r, 0x040C).unwrap());
        assert_eq!(std::fs::read(&h).unwrap(), config_azahar(1));
        assert_eq!((langue_console(0x040C), langue_console(0x0411), langue_console(0x0804), langue_console(0x0404), langue_console(0x0418)), (Some(2), Some(0), Some(6), Some(11), None));
    }
}
