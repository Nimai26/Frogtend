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
fn mettre_a_l_abri(emulateur: &Path, fichier: &Path) -> Resultat<()> {
    if !fichier.is_file() {
        return Ok(());
    }
    let jour = crate::noyau::maintenant().parse::<u64>().unwrap_or(0) / 86_400;
    let relatif = fichier.strip_prefix(emulateur).unwrap_or(fichier);
    let copie = emulateur.join(".frogtend-sauvegardes").join(format!("jour-{jour}")).join(relatif);
    if !copie.exists() {
        if let Some(p) = copie.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::copy(fichier, copie)?;
    }
    Ok(())
}

fn modifier_ini(emulateur: &Path, fichier: &Path, section: &str, valeurs: &[(&str, Vec<String>)]) -> Resultat<()> {
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

/// Prépare l'émulateur pour ce profil avant une partie. Rend les arguments à mettre AVANT sa ligne de commande.
/// `jeux` : les emplacements de Frogtend pour ce système.
pub fn preparer(id: &str, emulateur: &Path, profil: &str, jeux: &[String]) -> Resultat<Vec<String>> {
    let p = dossier_du_profil(emulateur, profil);
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
            Ok(vec!["-u".into(), texte(&utilisateur)])
        }
        "ppsspp" => {
            relier_memstick(emulateur, &p.join("memstick"))?;
            Ok(vec![])
        }
        _ => Ok(vec![]),
    }
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
    let s = std::process::Command::new("cmd")
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
        "ppsspp" => vec![documents.map(|d| d.join("PPSSPP"))],
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
    fn retroarch_recoit_un_fichier_de_reglages_par_profil() {
        let d = tempfile::tempdir().unwrap();
        let args = preparer("retroarch", d.path(), "Sebastien", &["E:\\Jeux\\SNES".into()]).unwrap();
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
        preparer("duckstation", d.path(), "Seb", &["E:\\Jeux\\PS1".into()]).unwrap();
        let ini = std::fs::read_to_string(d.path().join("settings.ini")).unwrap();
        assert!(ini.contains("SettingsVersion = 3"), "le reste est gardé");
        assert!(ini.contains(&format!("Directory = {}", d.path().join("Profils").join("Seb").join("memcards").display())));
        assert!(ini.contains("RecursivePaths = E:\\Jeux\\PS1"));
        preparer("duckstation", d.path(), "Léa", &["E:\\Jeux\\PS1".into()]).unwrap();
        let ini = std::fs::read_to_string(d.path().join("settings.ini")).unwrap();
        assert!(ini.contains("Léa") && !ini.contains("\\Seb\\"));
        // La configuration d'origine a été copiée à l'abri.
        let abri = std::fs::read_dir(d.path().join(".frogtend-sauvegardes")).unwrap().next().unwrap().unwrap().path();
        assert!(std::fs::read_to_string(abri.join("settings.ini")).unwrap().contains("SettingsVersion = 3"));
    }

    #[test]
    fn pcsx2_et_dolphin() {
        let d = tempfile::tempdir().unwrap();
        preparer("pcsx2", d.path(), "Seb", &["E:\\Jeux\\PS2".into()]).unwrap();
        let ini = std::fs::read_to_string(d.path().join("inis").join("PCSX2.ini")).unwrap();
        assert!(ini.contains("[Folders]") && ini.contains("MemoryCards = ") && ini.contains("RecursivePaths = E:\\Jeux\\PS2"));

        let args = preparer("dolphin", d.path(), "Seb", &["E:\\Jeux\\GC".into(), "F:\\GC".into()]).unwrap();
        assert_eq!(args[0], "-u");
        let ini = std::fs::read_to_string(PathBuf::from(&args[1]).join("Config").join("Dolphin.ini")).unwrap();
        assert!(ini.contains("ISOPaths = 2") && ini.contains("ISOPath1 = F:\\GC"));
    }

    #[test]
    fn le_memstick_de_ppsspp_devient_un_lien_vers_le_profil_sans_rien_effacer() {
        let d = tempfile::tempdir().unwrap();
        let ancien = d.path().join("memstick").join("PSP").join("SAVEDATA");
        std::fs::create_dir_all(&ancien).unwrap();
        std::fs::write(ancien.join("partie.bin"), b"avant").unwrap();

        preparer("ppsspp", d.path(), "Seb", &[]).unwrap();
        assert!(d.path().join("memstick (avant Frogtend)").join("PSP").join("SAVEDATA").join("partie.bin").is_file());
        std::fs::write(d.path().join("memstick").join("x.txt"), b"seb").unwrap();
        assert!(d.path().join("Profils").join("Seb").join("memstick").join("x.txt").is_file(), "le lien mène au profil");

        preparer("ppsspp", d.path(), "Léa", &[]).unwrap();
        assert!(!d.path().join("memstick").join("x.txt").exists(), "Léa ne voit pas les fichiers de Seb");
        assert!(d.path().join("Profils").join("Seb").join("memstick").join("x.txt").is_file(), "ceux de Seb restent");
    }
}
