//! Afficher ou cacher les images par seconde (le compteur DE L'ÉMULATEUR), depuis le menu en jeu de Frogtend.
//! Décision de Seb (08/11) : un choix PAR PERSONNE, pour tous ses jeux. Il est posé au lancement, avant que l'émulateur
//! ne démarre ; certains émulateurs savent aussi basculer pendant la partie (RetroArch, MAME, la famille d'Eden).
//!
//! Relevés (09/11) :
//! - RetroArch : `fps_show` (retroarch.cfg) ; posé dans le `frogtend.cfg` du profil (`--appendconfig`). RetroArch
//!   recopie la valeur de la partie dans retroarch.cfg en quittant (`config_save_on_exit`) : elle est rendue après.
//! - Eden, Eglantine, Lichii : `[UI] UIGameList\show_perf_overlay` (qt_common/config/uisettings.h, défaut false) ;
//!   lu au démarrage (main_window.cpp : `action_Show_Performance_Overlay` cochée, puis `OnLoadComplete` montre le
//!   compteur), bascule par le raccourci « Toggle Performance Overlay » (Ctrl+V d'origine), réenregistré en quittant.
//! - RPCS3 : `config\config.yml`, `Video:` ▸ `Performance Overlay:` ▸ `Enabled:` ; aucun raccourci : au prochain
//!   lancement seulement.

use std::path::Path;

use crate::erreurs::Resultat;

/// Frogtend sait-il poser le choix au lancement pour cet émulateur ?
pub fn au_lancement(id: &str) -> bool {
    matches!(id, "retroarch" | "rpcs3" | "eden" | "eglantine" | "lichii")
}

/// Ce que l'émulateur affiche aujourd'hui, d'après ses réglages (avant le lancement) ; `None` : inconnu.
pub fn lire(id: &str, emulateur: &Path) -> Option<bool> {
    match id {
        "retroarch" => crate::emulateurs::reglage_retroarch(emulateur, "fps_show").or(Some(false)),
        // Pas portable : ses vrais réglages sont dans le dossier de Windows, que Frogtend ne lit pas → inconnu.
        "eden" | "eglantine" | "lichii" if !crate::eden::portable(emulateur) => None,
        "eden" | "eglantine" | "lichii" => {
            let t = crate::eden::lire_reglages(emulateur).ok()?;
            Some(crate::manettes::lire_ini(&t, "UI", "UIGameList\\show_perf_overlay").first().is_some_and(|v| v.trim() == "true"))
        }
        "rpcs3" => lire_plafonne(&emulateur.join("config").join("config.yml")).and_then(|t| rpcs3_lire(&t)),
        _ => None,
    }
}

/// Pose le choix de la personne avant le lancement. `profil` : le dossier du profil dans l'émulateur (RetroArch : son
/// `frogtend.cfg`, déjà écrit pour cette partie). Rend vrai si quelque chose a été posé.
pub fn poser(id: &str, emulateur: &Path, profil: &Path, afficher: bool) -> Resultat<bool> {
    let v = if afficher { "true" } else { "false" };
    match id {
        "retroarch" => {
            // Ajouté à la fin (jamais le fichier réécrit : une écriture coupée ne le tronque pas).
            use std::io::Write;
            let cfg = profil.join("frogtend.cfg");
            if !cfg.is_file() {
                return Ok(false);
            }
            std::fs::OpenOptions::new().append(true).open(&cfg)?.write_all(format!("fps_show = \"{v}\"\n").as_bytes())?;
            Ok(true)
        }
        "eden" | "eglantine" | "lichii" => crate::eden::regler_images_par_seconde(emulateur, afficher),
        "rpcs3" => {
            // Ouvert : il réécrirait ses réglages en quittant ; rien n'est touché.
            if !crate::lancement::processus_du_dossier(emulateur).is_empty() {
                return Ok(false);
            }
            let f = emulateur.join("config").join("config.yml");
            // Absent (jamais lancé) : rien de créé ; RPCS3 l'écrit à sa première partie, le choix vaut à la suivante.
            let Some(t) = lire_plafonne(&f) else { return Ok(false) };
            let Some(apres) = rpcs3_regler(&t, afficher) else { return Ok(false) };
            // Copie à l'abri, et jamais un fichier illisible remplacé.
            crate::emulateurs_profils::remplacer_config(emulateur, &f, &apres)?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// La famille d'Eden bascule-t-elle le compteur pendant la partie ? Seulement si le raccourci « Toggle Performance
/// Overlay » est encore celui d'origine (Ctrl+V) : un raccourci changé par la personne n'est jamais deviné.
pub fn eden_en_direct(emulateur: &Path) -> bool {
    // Pas portable : son raccourci est dans le dossier de Windows, jamais deviné.
    if !crate::eden::portable(emulateur) {
        return false;
    }
    let Ok(t) = crate::eden::lire_reglages(emulateur) else { return false };
    crate::manettes::lire_ini(&t, "UI", "Shortcuts\\Main%20Window\\Toggle%20Performance%20Overlay\\KeySeq")
        .first()
        .is_none_or(|v| v.trim().eq_ignore_ascii_case("Ctrl+V"))
}

/// Ce que le lancement a fait du compteur, gardé dans la partie pour le menu.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct EtatLancement {
    /// Ce que l'émulateur affichera (`None` : inconnu).
    pub affiche: Option<bool>,
    pub au_lancement: bool,
    pub en_direct: bool,
    /// RetroArch : `fps_show` d'avant la partie, et si Frogtend en a posé un (rendu après la partie).
    pub avant: Option<bool>,
    pub pose: bool,
    pub erreur: Option<String>,
}

/// Avant le lancement : lit l'état de l'émulateur, pose le choix de la personne (`None` : jamais choisi dans Frogtend,
/// le réglage de l'émulateur est gardé).
pub fn au_lancement_de_la_partie(id: &str, emulateur: &Path, profil: &str, choix: Option<bool>, jeu_ps3: Option<&str>) -> EtatLancement {
    // RPCS3 : un réglage PROPRE à ce jeu (custom_configs, appliqué par-dessus le général) décide ; jamais écrit
    // par-dessus. Le menu montre sa valeur, sans entrée « à la prochaine partie ».
    let par_jeu = jeu_ps3
        .filter(|_| id == "rpcs3")
        .and_then(|n| crate::emulateurs_profils::rpcs3_config_du_jeu(emulateur, n).ok())
        .and_then(|f| lire_plafonne(&f))
        .and_then(|t| rpcs3_lire(&t));
    if par_jeu.is_some() {
        return EtatLancement { affiche: par_jeu, ..Default::default() };
    }
    let avant = lire(id, emulateur);
    let eden_ailleurs = matches!(id, "eden" | "eglantine" | "lichii") && !crate::eden::portable(emulateur);
    let mut e = EtatLancement {
        affiche: avant,
        au_lancement: au_lancement(id) && !eden_ailleurs,
        en_direct: !matches!(id, "eden" | "eglantine" | "lichii") || eden_en_direct(emulateur),
        avant: if id == "retroarch" { crate::emulateurs::reglage_retroarch(emulateur, "fps_show") } else { None },
        ..Default::default()
    };
    if let Some(v) = choix.filter(|_| e.au_lancement) {
        let p = crate::emulateurs_profils::dossier_du_profil(emulateur, profil);
        match poser(id, emulateur, &p, v) {
            Ok(fait) => {
                // Posé, ou déjà ainsi : la valeur est connue. RPCS3 sans fichier : il ne l'affiche pas encore.
                if fait || avant == Some(v) {
                    e.affiche = Some(v);
                }
                e.pose = fait && id == "retroarch";
            }
            Err(err) => e.erreur = Some(format!("{err:?}")),
        }
    }
    e
}

/// Un fichier de réglages lu avec un plafond (plusieurs Mo : ce n'est pas lui, jamais lu ni touché).
fn lire_plafonne(f: &Path) -> Option<String> {
    if std::fs::metadata(f).ok()?.len() > 8 * 1024 * 1024 {
        return None;
    }
    std::fs::read_to_string(f).ok()
}

/// RPCS3 : la valeur de `Video:` ▸ `Performance Overlay:` ▸ `Enabled:`.
pub fn rpcs3_lire(texte: &str) -> Option<bool> {
    rpcs3_ligne(texte).map(|(_, v)| v)
}

/// RPCS3 : le texte avec `Enabled:` réglé (fins de ligne et tout le reste gardés) ; `None` si rien ne change ou si
/// le bloc est introuvable (jamais créé : un autre `Enabled:` ne doit pas être touché).
pub fn rpcs3_regler(texte: &str, afficher: bool) -> Option<String> {
    let (i, v) = rpcs3_ligne(texte)?;
    if v == afficher {
        return None;
    }
    let mut lignes: Vec<&str> = texte.split_inclusive('\n').collect();
    let fin = if lignes[i].ends_with("\r\n") { "\r\n" } else if lignes[i].ends_with('\n') { "\n" } else { "" };
    let neuve = format!("    Enabled: {}{fin}", if afficher { "true" } else { "false" });
    lignes[i] = &neuve;
    Some(lignes.concat())
}

/// L'index de la ligne `    Enabled:` du bloc `  Performance Overlay:` de la section `Video:`, et sa valeur.
fn rpcs3_ligne(texte: &str) -> Option<(usize, bool)> {
    let mut video = false;
    let mut bloc = false;
    for (i, brute) in texte.split_inclusive('\n').enumerate() {
        let l = brute.trim_end_matches(['\r', '\n']);
        if l.trim().is_empty() {
            continue;
        }
        let retrait = l.len() - l.trim_start_matches(' ').len();
        if retrait == 0 {
            video = l == "Video:";
            bloc = false;
            continue;
        }
        if !video {
            continue;
        }
        if retrait == 2 {
            bloc = l.trim() == "Performance Overlay:";
            continue;
        }
        if bloc && retrait == 4 {
            if let Some(v) = l.trim().strip_prefix("Enabled:") {
                return match v.trim() {
                    "true" => Some((i, true)),
                    "false" => Some((i, false)),
                    _ => None,
                };
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const RPCS3: &str = "Core:\r\n  Enabled: true\r\nVideo:\r\n  Renderer: Vulkan\r\n  Performance Overlay:\r\n    Body Color (hex): \"#FFE138FF\"\r\n    Enabled: false\r\n    Font: n023055ms.ttf\r\n  Shader Mode: Async\r\nAudio:\r\n  Enabled: true\r\n";

    #[test]
    fn rpcs3_seule_la_ligne_du_compteur_change() {
        assert_eq!(rpcs3_lire(RPCS3), Some(false));
        let apres = rpcs3_regler(RPCS3, true).unwrap();
        assert_eq!(apres, RPCS3.replace("    Enabled: false\r\n", "    Enabled: true\r\n"), "CRLF gardé, les autres « Enabled » jamais touchés");
        assert_eq!(rpcs3_lire(&apres), Some(true));
        assert!(rpcs3_regler(&apres, true).is_none(), "déjà réglé : rien à écrire");
        assert!(rpcs3_regler("Video:\n  Renderer: Vulkan\nCore:\n  Enabled: false\n", true).is_none(), "bloc absent : jamais un autre Enabled");
        assert!(rpcs3_regler("", true).is_none());
    }

    #[test]
    fn rpcs3_absent_rien_de_cree() {
        let d = tempfile::tempdir().unwrap();
        assert!(!poser("rpcs3", d.path(), d.path(), true).unwrap());
        assert!(!d.path().join("config").exists());
    }

    #[test]
    fn retroarch_le_choix_va_dans_le_fichier_du_profil() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("Profils").join("Seb");
        assert!(!poser("retroarch", d.path(), &p, true).unwrap(), "sans frogtend.cfg : rien");
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(p.join("frogtend.cfg"), "savefile_directory = \"x\"\n").unwrap();
        assert!(poser("retroarch", d.path(), &p, true).unwrap());
        assert_eq!(std::fs::read_to_string(p.join("frogtend.cfg")).unwrap(), "savefile_directory = \"x\"\nfps_show = \"true\"\n");
        assert!(!d.path().join("retroarch.cfg").exists(), "le fichier commun n'est jamais touché au lancement");
        assert_eq!(lire("retroarch", d.path()), Some(false), "absent : RetroArch ne l'affiche pas");
        std::fs::write(d.path().join("retroarch.cfg"), "fps_show = \"true\"\n").unwrap();
        assert_eq!(lire("retroarch", d.path()), Some(true));
    }

    #[test]
    fn le_lancement_pose_le_choix_et_garde_l_etat() {
        let d = tempfile::tempdir().unwrap();
        let p = crate::emulateurs_profils::dossier_du_profil(d.path(), "Seb");
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(p.join("frogtend.cfg"), "").unwrap();
        std::fs::write(d.path().join("retroarch.cfg"), "fps_show = \"false\"\n").unwrap();
        let e = au_lancement_de_la_partie("retroarch", d.path(), "Seb", Some(true), None);
        assert_eq!(e, EtatLancement { affiche: Some(true), au_lancement: true, en_direct: true, avant: Some(false), pose: true, erreur: None });
        // Jamais choisi : rien d'écrit, l'état de l'émulateur est montré.
        std::fs::write(p.join("frogtend.cfg"), "").unwrap();
        let e = au_lancement_de_la_partie("retroarch", d.path(), "Seb", None, None);
        assert_eq!((e.affiche, e.pose), (Some(false), false));
        assert_eq!(std::fs::read_to_string(p.join("frogtend.cfg")).unwrap(), "");
        // Un émulateur que Frogtend ne règle pas : rien.
        let e = au_lancement_de_la_partie("cemu", d.path(), "Seb", Some(true), None);
        assert_eq!((e.affiche, e.au_lancement, e.pose), (None, false, false));
    }

    #[test]
    fn eden_hors_mode_portable_rien_n_est_devine() {
        let d = tempfile::tempdir().unwrap();
        let e = au_lancement_de_la_partie("eglantine", d.path(), "Seb", Some(false), None);
        assert_eq!((e.affiche, e.au_lancement, e.en_direct), (None, false, false), "réglages dans le dossier de Windows : inconnus");
        assert!(!d.path().join("user").exists(), "rien de créé");
    }

    #[test]
    fn rpcs3_un_reglage_du_jeu_passe_devant() {
        let d = tempfile::tempdir().unwrap();
        let c = d.path().join("config");
        std::fs::create_dir_all(c.join("custom_configs")).unwrap();
        std::fs::write(c.join("config.yml"), RPCS3).unwrap();
        std::fs::write(c.join("custom_configs").join("config_BLES01227.yml"), "Video:\n  Performance Overlay:\n    Enabled: true\n").unwrap();
        let e = au_lancement_de_la_partie("rpcs3", d.path(), "Seb", Some(false), Some("BLES01227"));
        assert_eq!((e.affiche, e.au_lancement), (Some(true), false), "le réglage du jeu, jamais écrasé");
        assert_eq!(std::fs::read_to_string(c.join("config.yml")).unwrap(), RPCS3, "le général non touché");
        let e = au_lancement_de_la_partie("rpcs3", d.path(), "Seb", Some(true), Some("BLUS00001"));
        assert_eq!((e.affiche, e.au_lancement), (Some(true), true));
        assert_eq!(rpcs3_lire(&std::fs::read_to_string(c.join("config.yml")).unwrap()), Some(true));
    }

    #[test]
    fn retroarch_fps_show_rendu_apres_la_partie() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("retroarch.cfg"), "a = \"1\"\r\nfps_show = \"true\"\r\nvideo_shader_enable = \"true\"\r\n").unwrap();
        crate::emulateurs::rendre_reglage_retroarch(d.path(), "fps_show", false).unwrap();
        assert_eq!(std::fs::read_to_string(d.path().join("retroarch.cfg")).unwrap(), "a = \"1\"\r\nfps_show = \"false\"\r\nvideo_shader_enable = \"true\"\r\n");
    }

    #[test]
    fn eden_installation_neuve_et_raccourci() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("user")).unwrap();
        assert_eq!(lire("lichii", d.path()), Some(false));
        assert!(eden_en_direct(d.path()), "pas de fichier : Ctrl+V d'origine");
        assert!(poser("lichii", d.path(), d.path(), true).unwrap(), "installation neuve : fichier créé");
        let t = std::fs::read_to_string(crate::eden::fichier_reglages(d.path())).unwrap();
        assert!(t.contains("UIGameList\\show_perf_overlay=true") && t.contains("UIGameList\\show_perf_overlay\\default=false"), "{t}");
        assert_eq!(lire("lichii", d.path()), Some(true));
        assert!(!poser("lichii", d.path(), d.path(), true).unwrap(), "déjà réglé : rien");
        let autre = "[UI]\nShortcuts\\Main%20Window\\Toggle%20Performance%20Overlay\\KeySeq=Ctrl+Shift+P\n";
        std::fs::write(crate::eden::fichier_reglages(d.path()), autre).unwrap();
        assert!(!eden_en_direct(d.path()), "raccourci changé par la personne : pas de bascule en direct");
        assert!(!au_lancement("cemu") && au_lancement("lichii") && au_lancement("rpcs3"));
    }
}
