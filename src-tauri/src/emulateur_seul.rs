//! « ▶ Ouvrir l'émulateur seul » (✅ Seb 16-17/11) : un émulateur ouvert SANS jeu, avec le profil de la personne,
//! pour gérer ses DLC, sa manette, ses réglages là où ils servent. Sa ligne est faite SEULEMENT des arguments de
//! profil (jamais celle du catalogue ni celle de pc.json : elles portent `-f`, `-L`, `%rom%`, `--no-gui`…) : pas de
//! plein écran, l'interface de l'émulateur s'ouvre. Relu dans les sources officielles (17/11) :
//! - Ryubing 1.3.3 (CommandLineState.cs) : `-r <racine>` sans fichier → la fenêtre principale et sa liste de jeux
//!   (Program.cs : rien n'est chargé sans `LaunchPathArg`) ; `--hide-updates` saute toute recherche de mise à jour au
//!   démarrage (MainWindow.axaml.cs) — Frogtend fait les mises à jour.
//! - Eden v0.2.1 (src/yuzu/main_window.cpp) : `-u <index>` change l'utilisateur courant même sans jeu (sans `-g`, rien
//!   n'est lancé).
//! - Dolphin (DolphinQt/Main.cpp, UICommon.cpp) : `-u <dossier utilisateur>` sans `-b`/`-e` ouvre la fenêtre principale
//!   avec ce dossier.
//! - DuckStation (duckstation-qt/qthost.cpp) et PCSX2 (pcsx2-qt/QtHost.cpp) : sans argument, la fenêtre principale et
//!   sa liste de jeux (`-batch` sans jeu serait refusé).

/// Les émulateurs que Frogtend sait ouvrir seuls (les autres : pas encore, le bouton ne les propose pas).
pub const PRIS_EN_CHARGE: [&str; 7] = ["ryubing", "eden", "eglantine", "lichii", "duckstation", "pcsx2", "dolphin"];

/// L'identifiant (catalogue) d'un programme d'émulateur, reconnu à son nom de fichier comme au lancement d'un jeu.
pub fn id_du_programme(programme: &str) -> Option<&'static str> {
    let nom = std::path::Path::new(programme).file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
    crate::emulateurs::CATALOGUE.iter().find(|f| !nom.is_empty() && nom.starts_with(f.programme)).map(|f| f.id)
}

/// Cet émulateur s'ouvre-t-il seul ?
pub fn pris_en_charge(id: &str) -> bool {
    PRIS_EN_CHARGE.contains(&id)
}

/// La valeur qui suit `option` dans les arguments de profil, avec l'option.
fn paire(profil: &[String], option: &str) -> Vec<String> {
    profil.iter().position(|a| a == option).and_then(|i| profil.get(i + 1)).map(|v| vec![option.to_string(), v.clone()]).unwrap_or_default()
}

/// La ligne SANS JEU d'un émulateur, faite des seuls arguments de profil (`profil` : ce que
/// `emulateurs_profils::preparer` a rendu). `None` : émulateur pas pris en charge. `eden_portable` : la famille d'Eden
/// est portable — sinon la préparation rend `-u <nom>` (pas un index), jamais vérifié sans `-g` : aucun argument.
pub fn arguments_sans_jeu(id: &str, profil: &[String], eden_portable: bool) -> Option<Vec<String>> {
    match id {
        "ryubing" => {
            let mut a = paire(profil, "-r");
            a.push("--hide-updates".into());
            Some(a)
        }
        "eden" | "eglantine" | "lichii" if !eden_portable => Some(vec![]),
        "eden" | "eglantine" | "lichii" | "dolphin" => Some(paire(profil, "-u")),
        "duckstation" | "pcsx2" => Some(vec![]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(l: &[&str]) -> Vec<String> {
        l.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn les_emulateurs_pris_en_charge_sont_reconnus_a_leur_programme() {
        // La même table que `ouvrableSeul` (src/lib/emulateurs/seul.test.ts).
        for (programme, id) in [
            ("E:\\Emulateurs\\Ryubing\\Ryujinx.exe", Some("ryubing")),
            ("E:\\Emulateurs\\Eden\\eden.exe", Some("eden")),
            ("E:\\Emulateurs\\Eglantine\\eglantine.exe", Some("eglantine")),
            ("E:\\Emulateurs\\Lichii\\Lichii.exe", Some("lichii")),
            ("E:\\Emulateurs\\DuckStation\\duckstation-qt-x64-ReleaseLTCG.exe", Some("duckstation")),
            ("E:\\Emulateurs\\PCSX2\\pcsx2-qt.exe", Some("pcsx2")),
            ("E:\\Emulateurs\\Dolphin\\Dolphin.exe", Some("dolphin")),
            ("E:\\Emulateurs\\RetroArch\\retroarch.exe", Some("retroarch")),
            ("E:\\Emulateurs\\RPCS3\\rpcs3.exe", Some("rpcs3")),
            ("", None),
        ] {
            assert_eq!(id_du_programme(programme), id, "{programme}");
            assert_eq!(id.is_some_and(pris_en_charge), !["retroarch", "rpcs3", ""].contains(&id.unwrap_or("")), "{programme}");
        }
    }

    #[test]
    fn la_ligne_sans_jeu_ne_porte_que_le_profil() {
        // Ce que rendrait la préparation, plus ce qui ne doit JAMAIS passer (plein écran, jeu, sans interface).
        let bruit = s(&["-f", "-g", "%rom%", "-b", "-e", "--no-gui", "-batch", "-fullscreen"]);
        let avec = |p: &[&str]| [s(p), bruit.clone()].concat();
        assert_eq!(arguments_sans_jeu("ryubing", &avec(&["-r", "D:\\Ryu\\Profils\\Seb"]), false), Some(s(&["-r", "D:\\Ryu\\Profils\\Seb", "--hide-updates"])));
        // Ryubing pas portable (aucune racine) : seulement sans les mises à jour.
        assert_eq!(arguments_sans_jeu("ryubing", &[], false), Some(s(&["--hide-updates"])));
        for e in ["eden", "eglantine", "lichii"] {
            assert_eq!(arguments_sans_jeu(e, &avec(&["-u", "2"]), true), Some(s(&["-u", "2"])), "{e}");
            // Pas portable : la préparation rend `-u <nom>`, jamais vérifié sans jeu → aucun argument.
            assert_eq!(arguments_sans_jeu(e, &avec(&["-u", "Seb"]), false), Some(vec![]), "{e}");
        }
        assert_eq!(arguments_sans_jeu("dolphin", &avec(&["-u", "E:\\Dolphin\\Profils\\Seb\\User"]), false), Some(s(&["-u", "E:\\Dolphin\\Profils\\Seb\\User"])));
        assert_eq!(arguments_sans_jeu("duckstation", &bruit, false), Some(vec![]));
        assert_eq!(arguments_sans_jeu("pcsx2", &bruit, false), Some(vec![]));
        for e in ["retroarch", "rpcs3", "mame", "xemu", "cemu", "dosbox", "bigpemu"] {
            assert_eq!(arguments_sans_jeu(e, &[], true), None, "{e} pas encore");
        }
    }

    #[test]
    fn la_ligne_sans_jeu_vient_de_la_vraie_preparation() {
        use crate::emulateurs_profils::{dossier_du_profil, preparer, Manette};
        let d = tempfile::tempdir().unwrap();
        // Ryubing portable : sa racine, celle de la personne.
        let ryu = d.path().join("Ryubing");
        std::fs::create_dir_all(ryu.join("portable").join("system")).unwrap();
        let args = preparer("ryubing", &ryu, "Seb", &[], &Manette::Clavier).unwrap();
        let racine = dossier_du_profil(&ryu, "Seb").to_string_lossy().to_string();
        assert_eq!(arguments_sans_jeu("ryubing", &args, false), Some(vec!["-r".into(), racine, "--hide-updates".into()]));
        // Eden : portable, l'index de l'utilisateur de la personne ; pas portable, rien.
        let eden = d.path().join("Eden");
        std::fs::create_dir_all(&eden).unwrap();
        let args = preparer("eden", &eden, "Seb", &[], &Manette::Clavier).unwrap();
        assert_eq!(arguments_sans_jeu("eden", &args, crate::eden::portable(&eden)), Some(vec![]));
        std::fs::create_dir_all(eden.join("user")).unwrap();
        let args = preparer("eden", &eden, "Seb", &[], &Manette::Clavier).unwrap();
        assert_eq!(arguments_sans_jeu("eden", &args, crate::eden::portable(&eden)), Some(s(&["-u", "1"])));
        // Dolphin : le dossier utilisateur de la personne.
        let dol = d.path().join("Dolphin");
        let args = preparer("dolphin", &dol, "Seb", &[], &Manette::Clavier).unwrap();
        let user = dossier_du_profil(&dol, "Seb").join("User").to_string_lossy().to_string();
        assert_eq!(arguments_sans_jeu("dolphin", &args, false), Some(vec!["-u".into(), user]));
        // DuckStation : rien sur la ligne (ses réglages communs portent les dossiers de la personne).
        let ds = d.path().join("DuckStation");
        std::fs::create_dir_all(&ds).unwrap();
        let args = preparer("duckstation", &ds, "Seb", &[], &Manette::Clavier).unwrap();
        assert_eq!(arguments_sans_jeu("duckstation", &args, false), Some(vec![]));
    }
}
