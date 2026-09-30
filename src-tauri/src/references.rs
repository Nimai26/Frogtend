//! Les réglages de manette de référence (lot 4c, Seb le 30/09) : Seb règle une manette dans un émulateur, l'enregistre
//! comme profil avec l'outil de l'émulateur lui-même, et Frogtend en fait une **référence**. Les références sont
//! déposées dans chaque profil (on les retrouve dans l'émulateur) et appliquées au jeu : celle par défaut du système,
//! ou celle choisie pour ce jeu (« Commandes » du jeu).
//!
//! Relevé dans les sources officielles (30/09/2026) :
//! - Dolphin : profils `Config\Profiles\<Wiimote|GCPad>\<nom>.ini`, section `[Profile]` ; la manette active est
//!   `Config\WiimoteNew.ini [Wiimote1]` / `Config\GCPadNew.ini [GCPad1]` (clé `Source` à part, gardée).
//! - DuckStation, PCSX2 : profils `inputprofiles\<nom>.ini` (sections `[Pad1]`…), la manette active est `[Pad1]`.
//!
//! Les références livrées avec Frogtend sont dans `src-tauri/references/` ; celles reprises sur ce PC, dans
//! `<données>\references\` (un même nom remplace celle livrée). Frogtend ne défait jamais une manette que la
//! personne a réglée à la main, sauf si elle choisit une référence pour un jeu.

use crate::emulateurs_profils::modifier_ini;
use crate::erreurs::{Erreur, Resultat};
#[cfg(test)]
use crate::manettes::lire_ini;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Reference {
    pub emulateur: String,
    /// `Wiimote`, `GCPad` (Dolphin) ou `Pad` (DuckStation, PCSX2).
    pub genre: String,
    pub nom: String,
    /// Le fichier de profil de l'émulateur, tel quel.
    #[serde(skip_serializing)]
    pub contenu: String,
    /// Livrée avec Frogtend (sinon : reprise sur ce PC).
    pub livree: bool,
}

/// Les références livrées avec Frogtend (à régler de nouveau par Seb : celles-ci sont un départ).
const LIVREES: &[(&str, &str, &str, &str)] = &[
    ("dolphin", "Wiimote", "Wiimote + Nunchuk", include_str!("../references/dolphin/Wiimote/Wiimote + Nunchuk.ini")),
    ("dolphin", "Wiimote", "Wiimote horizontale", include_str!("../references/dolphin/Wiimote/Wiimote horizontale.ini")),
];

/// La référence appliquée d'office à un système, si sa manette n'a pas été réglée à la main.
pub fn par_defaut(plateforme: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match plateforme {
        "Nintendo Wii" => Some(("dolphin", "Wiimote", "Wiimote + Nunchuk")),
        _ => None,
    }
}

/// Les genres de manette que connaît un émulateur.
pub fn genres(emulateur: &str) -> &'static [&'static str] {
    match emulateur {
        "dolphin" => &["Wiimote", "GCPad"],
        "duckstation" | "pcsx2" => &["Pad"],
        _ => &[],
    }
}

/// Le dossier des profils de l'émulateur pour un genre. `utilisateur` : le dossier utilisateur Dolphin du profil.
pub fn dossier_natif(emulateur: &str, genre: &str, dossier: &Path, utilisateur: &Path) -> Option<PathBuf> {
    match (emulateur, genre) {
        ("dolphin", "Wiimote" | "GCPad") => Some(utilisateur.join("Config").join("Profiles").join(genre)),
        ("duckstation" | "pcsx2", "Pad") => Some(dossier.join("inputprofiles")),
        _ => None,
    }
}

/// Où la manette active est rangée : (fichier, section, section du fichier de profil).
fn cible(emulateur: &str, genre: &str, dossier: &Path, utilisateur: &Path) -> Option<(PathBuf, &'static str, &'static str)> {
    match (emulateur, genre) {
        ("dolphin", "Wiimote") => Some((utilisateur.join("Config").join("WiimoteNew.ini"), "Wiimote1", "Profile")),
        ("dolphin", "GCPad") => Some((utilisateur.join("Config").join("GCPadNew.ini"), "GCPad1", "Profile")),
        ("duckstation", "Pad") => Some((dossier.join("settings.ini"), "Pad1", "Pad1")),
        ("pcsx2", "Pad") => Some((dossier.join("inis").join("PCSX2.ini"), "Pad1", "Pad1")),
        _ => None,
    }
}

/// Le nom d'une référence, sûr pour un nom de fichier.
fn nom_sur(nom: &str) -> Resultat<String> {
    let n = nom.trim();
    if n.is_empty() || n.len() > 80 || n.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) || n.starts_with('.') {
        return Err(Erreur::Refus(format!("Nom de réglage invalide : « {nom} ».")));
    }
    Ok(n.to_string())
}

/// Toutes les références d'un émulateur : celles livrées, puis celles reprises sur ce PC (qui remplacent une livrée
/// de même nom).
pub fn toutes(magasin: &Path, emulateur: &str) -> Vec<Reference> {
    let mut l: Vec<Reference> = LIVREES
        .iter()
        .filter(|(e, ..)| *e == emulateur)
        .map(|(e, g, n, c)| Reference { emulateur: e.to_string(), genre: g.to_string(), nom: n.to_string(), contenu: c.to_string(), livree: true })
        .collect();
    for genre in genres(emulateur) {
        let Ok(entrees) = std::fs::read_dir(magasin.join(emulateur).join(genre)) else { continue };
        let mut fichiers: Vec<PathBuf> = entrees.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "ini")).collect();
        fichiers.sort();
        for f in fichiers {
            let (Some(nom), Ok(contenu)) = (f.file_stem().and_then(|s| s.to_str()), std::fs::read_to_string(&f)) else { continue };
            let r = Reference { emulateur: emulateur.into(), genre: genre.to_string(), nom: nom.into(), contenu, livree: false };
            match l.iter_mut().find(|x| x.genre == r.genre && x.nom == r.nom) {
                Some(x) => *x = r,
                None => l.push(r),
            }
        }
    }
    l
}

pub fn trouver(magasin: &Path, emulateur: &str, genre: &str, nom: &str) -> Option<Reference> {
    toutes(magasin, emulateur).into_iter().find(|r| r.genre == genre && r.nom == nom)
}

#[derive(Debug, Serialize, PartialEq)]
pub struct ProfilNatif {
    pub genre: String,
    pub nom: String,
    pub chemin: String,
}

/// Les profils enregistrés dans l'émulateur (ce que Seb peut reprendre comme référence). Pour Dolphin : ceux du
/// dossier utilisateur du profil (Dolphin lancé par Frogtend) ET ceux de son dossier `User` (Dolphin ouvert à la
/// main, en mode portable).
pub fn profils_natifs(emulateur: &str, dossier: &Path, utilisateur: &Path) -> Vec<ProfilNatif> {
    let mut utilisateurs = vec![utilisateur.to_path_buf()];
    if emulateur == "dolphin" && dossier.join("User") != utilisateur {
        utilisateurs.push(dossier.join("User"));
    }
    let mut l = Vec::new();
    for u in &utilisateurs {
        for genre in genres(emulateur) {
            let Some(d) = dossier_natif(emulateur, genre, dossier, u) else { continue };
            let Ok(entrees) = std::fs::read_dir(&d) else { continue };
            let mut fichiers: Vec<PathBuf> = entrees.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "ini")).collect();
            fichiers.sort();
            for f in fichiers {
                let chemin = f.to_string_lossy().to_string();
                if let (Some(nom), false) = (f.file_stem().and_then(|s| s.to_str()), l.iter().any(|p: &ProfilNatif| p.chemin == chemin)) {
                    l.push(ProfilNatif { genre: genre.to_string(), nom: nom.into(), chemin });
                }
            }
        }
    }
    l
}

/// Fait d'un profil de l'émulateur une référence de ce PC (sous le nom donné). Une référence de même nom déjà
/// reprise est d'abord copiée à côté (`.avant-<date>`).
pub fn reprendre(magasin: &Path, emulateur: &str, genre: &str, source: &Path, nom: &str) -> Resultat<Reference> {
    if !genres(emulateur).contains(&genre) {
        return Err(Erreur::Refus(format!("{emulateur} n'a pas de manette « {genre} ».")));
    }
    let nom = nom_sur(nom)?;
    let contenu = std::fs::read_to_string(source).map_err(|e| Erreur::Disque(format!("Profil illisible ({e}).")))?;
    let section = if emulateur == "dolphin" { "Profile" } else { "Pad1" };
    if !contenu.lines().any(|l| l.trim().eq_ignore_ascii_case(&format!("[{section}]"))) {
        return Err(Erreur::Refus(format!("Ce fichier n'est pas un profil de manette de {emulateur} (pas de section [{section}]).")));
    }
    let dossier = magasin.join(emulateur).join(genre);
    std::fs::create_dir_all(&dossier)?;
    let fichier = dossier.join(format!("{nom}.ini"));
    if fichier.is_file() && std::fs::read_to_string(&fichier).ok().as_deref() != Some(contenu.as_str()) {
        std::fs::copy(&fichier, dossier.join(format!("{nom}.ini.avant-{}", crate::noyau::maintenant())))?;
    }
    std::fs::write(&fichier, &contenu)?;
    Ok(Reference { emulateur: emulateur.into(), genre: genre.into(), nom, contenu, livree: false })
}

/// Dépose les références dans les profils de l'émulateur, pour qu'on les retrouve dans ses menus. Un profil de même
/// nom déjà là n'est jamais remplacé (Seb est peut-être en train de le retoucher).
pub fn deposer(refs: &[Reference], dossier: &Path, utilisateur: &Path) -> Resultat<usize> {
    let mut n = 0;
    for r in refs {
        let Some(d) = dossier_natif(&r.emulateur, &r.genre, dossier, utilisateur) else { continue };
        let f = d.join(format!("{}.ini", r.nom));
        if !f.exists() {
            std::fs::create_dir_all(&d)?;
            std::fs::write(&f, &r.contenu)?;
            n += 1;
        }
    }
    Ok(n)
}

/// Les lignes `clé = valeur` d'une section, dans l'ordre.
fn lignes_section(texte: &str, section: &str) -> Vec<(String, String)> {
    let entete = format!("[{section}]").to_lowercase();
    let mut dedans = false;
    let mut v = Vec::new();
    for l in texte.lines() {
        let t = l.trim();
        if t.starts_with('[') {
            dedans = t.to_lowercase() == entete;
        } else if dedans {
            if let Some((c, val)) = t.split_once('=') {
                v.push((c.trim().to_string(), val.trim().to_string()));
            }
        }
    }
    v
}

fn empreinte_section(texte: &str, section: &str) -> String {
    let mut h = Sha256::new();
    for (c, v) in lignes_section(texte, section) {
        h.update(format!("{c}={v}\n"));
    }
    format!("{:x}", h.finalize())
}

/// Le fichier où Frogtend retient ce qu'il a écrit (pour savoir si la personne a retouché la manette depuis).
fn marqueur(fichier: &Path) -> PathBuf {
    fichier.with_file_name(".frogtend-manettes.json")
}

fn lire_marqueur(fichier: &Path) -> serde_json::Map<String, serde_json::Value> {
    std::fs::read_to_string(marqueur(fichier))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

/// Applique une référence à la manette du joueur 1.
/// - `imposee` (choisie pour ce jeu) : toujours appliquée (la configuration d'avant est mise à l'abri) ;
/// - sinon (par défaut du système) : seulement si la manette n'a jamais été réglée, ou si c'est Frogtend qui l'a
///   réglée en dernier (une retouche à la main n'est jamais défaite).
///
/// Rend `true` si la configuration a été écrite.
pub fn appliquer(r: &Reference, dossier: &Path, utilisateur: &Path, imposee: bool) -> Resultat<bool> {
    let Some((fichier, section, source)) = cible(&r.emulateur, &r.genre, dossier, utilisateur) else {
        return Ok(false);
    };
    let texte = std::fs::read_to_string(&fichier).unwrap_or_default();
    let cle = section.to_string();
    let mut marques = lire_marqueur(&fichier);
    let actuelle = empreinte_section(&texte, section);
    let libre = lignes_section(&texte, section).iter().all(|(c, v)| c.eq_ignore_ascii_case("Source") || v.is_empty())
        || marques.get(&cle).and_then(|v| v.as_str()) == Some(actuelle.as_str());
    if !imposee && !libre {
        return Ok(false);
    }
    let lignes = lignes_section(&r.contenu, source);
    if lignes.is_empty() {
        return Err(Erreur::Refus(format!("Le réglage « {} » est vide.", r.nom)));
    }
    // Toute la section devient celle de la référence ; seule `Source` (d'où vient la Wiimote) reste.
    let mut par_cle: Vec<(String, Vec<String>)> = Vec::new();
    for (c, v) in lignes {
        if c.eq_ignore_ascii_case("Source") {
            continue;
        }
        match par_cle.iter_mut().find(|(k, _)| *k == c) {
            Some((_, l)) => l.push(v),
            None => par_cle.push((c, vec![v])),
        }
    }
    for (c, _) in lignes_section(&texte, section) {
        if !c.eq_ignore_ascii_case("Source") && !par_cle.iter().any(|(k, _)| k.eq_ignore_ascii_case(&c)) {
            par_cle.push((c, vec![])); // une clé qui n'est pas dans la référence disparaît
        }
    }
    let refs: Vec<(&str, Vec<String>)> = par_cle.iter().map(|(c, v)| (c.as_str(), v.clone())).collect();
    // modifier_ini met l'ancienne configuration à l'abri avant d'écrire, et n'écrit rien si rien ne change.
    modifier_ini(dossier, &fichier, section, &refs)?;
    let ecrite = std::fs::read_to_string(&fichier).unwrap_or_default();
    marques.insert(cle, serde_json::Value::String(empreinte_section(&ecrite, section)));
    std::fs::write(marqueur(&fichier), serde_json::to_string_pretty(&marques).unwrap_or_default())?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(nom: &str, contenu: &str) -> Reference {
        Reference { emulateur: "dolphin".into(), genre: "Wiimote".into(), nom: nom.into(), contenu: contenu.into(), livree: false }
    }

    #[test]
    fn les_references_livrees_sont_des_profils_dolphin_valides() {
        for (e, g, n, c) in LIVREES {
            assert!(genres(e).contains(g), "{n}");
            let l = lignes_section(c, "Profile");
            assert!(l.iter().any(|(k, v)| k == "Device" && v == "XInput/0/Gamepad"), "{n}");
        }
        let d = tempfile::tempdir().unwrap();
        let (e, g, n) = par_defaut("Nintendo Wii").unwrap();
        assert!(trouver(d.path(), e, g, n).is_some(), "la référence par défaut de la Wii existe");
    }

    #[test]
    fn une_reference_reprise_sur_ce_pc_remplace_celle_livree_de_meme_nom() {
        let d = tempfile::tempdir().unwrap();
        let source = d.path().join("seb.ini");
        std::fs::write(&source, "[Profile]\r\nDevice = XInput/0/Gamepad\r\nButtons/A = `Button B`\r\n").unwrap();
        let magasin = d.path().join("references");
        reprendre(&magasin, "dolphin", "Wiimote", &source, "Wiimote + Nunchuk").unwrap();
        let r = trouver(&magasin, "dolphin", "Wiimote", "Wiimote + Nunchuk").unwrap();
        assert!(!r.livree && r.contenu.contains("`Button B`"));
        assert_eq!(toutes(&magasin, "dolphin").iter().filter(|r| r.nom == "Wiimote + Nunchuk").count(), 1);

        // La reprendre de nouveau, changée : l'ancienne est gardée à côté.
        std::fs::write(&source, "[Profile]\r\nDevice = XInput/1/Gamepad\r\n").unwrap();
        reprendre(&magasin, "dolphin", "Wiimote", &source, "Wiimote + Nunchuk").unwrap();
        let cote = std::fs::read_dir(magasin.join("dolphin").join("Wiimote")).unwrap().count();
        assert_eq!(cote, 2);
    }

    #[test]
    fn reprendre_refuse_un_nom_ou_un_fichier_douteux() {
        let d = tempfile::tempdir().unwrap();
        let source = d.path().join("x.ini");
        std::fs::write(&source, "[Autre]\r\nA = 1\r\n").unwrap();
        assert!(reprendre(d.path(), "dolphin", "Wiimote", &source, "..\\evasion").is_err());
        assert!(reprendre(d.path(), "dolphin", "Wiimote", &source, "Bon nom").is_err(), "pas de section [Profile]");
        assert!(reprendre(d.path(), "ppsspp", "Pad", &source, "Bon nom").is_err());
    }

    #[test]
    fn les_profils_de_l_emulateur_sont_trouves_et_les_references_deposees_sans_rien_remplacer() {
        let d = tempfile::tempdir().unwrap();
        let u = d.path().join("User");
        let natif = u.join("Config").join("Profiles").join("Wiimote");
        std::fs::create_dir_all(&natif).unwrap();
        std::fs::write(natif.join("Wiimote horizontale.ini"), "[Profile]\r\nretouche = seb\r\n").unwrap();
        let n = deposer(&toutes(&d.path().join("magasin"), "dolphin"), d.path(), &u).unwrap();
        assert_eq!(n, 1, "seule « Wiimote + Nunchuk » manquait");
        assert!(std::fs::read_to_string(natif.join("Wiimote horizontale.ini")).unwrap().contains("retouche = seb"));
        let l = profils_natifs("dolphin", d.path(), &u);
        assert_eq!(l.iter().map(|p| p.nom.as_str()).collect::<Vec<_>>(), ["Wiimote + Nunchuk", "Wiimote horizontale"]);

        // Dolphin ouvert à la main (dossier `User` à côté du programme) : ses profils sont trouvés aussi.
        let autre = d.path().join("Profils").join("Seb").join("User");
        let main = d.path().join("User").join("Config").join("Profiles").join("GCPad");
        std::fs::create_dir_all(&main).unwrap();
        std::fs::write(main.join("Seb GC.ini"), "[Profile]\r\n").unwrap();
        let l = profils_natifs("dolphin", d.path(), &autre);
        assert!(l.iter().any(|p| p.nom == "Seb GC" && p.genre == "GCPad"));
    }

    #[test]
    fn la_reference_par_defaut_ne_defait_pas_une_retouche_a_la_main_mais_un_choix_pour_le_jeu_si() {
        let d = tempfile::tempdir().unwrap();
        let u = d.path().join("User");
        let ini = u.join("Config").join("WiimoteNew.ini");
        std::fs::create_dir_all(ini.parent().unwrap()).unwrap();
        std::fs::write(&ini, "[Wiimote1]\r\nSource = 1\r\n\r\n[Wiimote2]\r\nSource = 0\r\n").unwrap();
        let nunchuk = reference("N", "[Profile]\r\nDevice = XInput/0/Gamepad\r\nExtension = Nunchuk\r\nButtons/A = `Button A`\r\n");
        let horizontale = reference("H", "[Profile]\r\nDevice = XInput/0/Gamepad\r\nOptions/Sideways Wiimote = True\r\n");

        // Jamais réglée : la référence par défaut s'applique, la source et la 2e Wiimote sont gardées.
        assert!(appliquer(&nunchuk, d.path(), &u, false).unwrap());
        let t = std::fs::read_to_string(&ini).unwrap();
        assert_eq!(lire_ini(&t, "Wiimote1", "Source"), vec!["1"]);
        assert_eq!(lire_ini(&t, "Wiimote1", "Extension"), vec!["Nunchuk"]);
        assert!(t.contains("[Wiimote2]"));

        // Choisie pour un jeu : elle remplace tout (plus de Nunchuk).
        assert!(appliquer(&horizontale, d.path(), &u, true).unwrap());
        let t = std::fs::read_to_string(&ini).unwrap();
        assert!(lire_ini(&t, "Wiimote1", "Extension").is_empty());
        assert_eq!(lire_ini(&t, "Wiimote1", "Options/Sideways Wiimote"), vec!["True"]);

        // Jeu suivant, sans choix : c'est Frogtend qui avait réglé, la référence par défaut revient.
        assert!(appliquer(&nunchuk, d.path(), &u, false).unwrap());

        // Seb retouche à la main dans Dolphin : la référence par défaut ne la défait plus.
        let t = std::fs::read_to_string(&ini).unwrap().replace("`Button A`", "`Button X`");
        std::fs::write(&ini, &t).unwrap();
        assert!(!appliquer(&nunchuk, d.path(), &u, false).unwrap());
        assert_eq!(std::fs::read_to_string(&ini).unwrap(), t);
        assert!(d.path().join(".frogtend-sauvegardes").is_dir(), "l'ancienne configuration est à l'abri");
    }

    #[test]
    fn duckstation_recoit_la_section_pad1_d_un_profil() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("settings.ini"), "[Main]\r\nX = 1\r\n\r\n[Pad1]\r\nType = AnalogController\r\nUp = SDL-0/DPadUp\r\n").unwrap();
        let r = Reference {
            emulateur: "duckstation".into(),
            genre: "Pad".into(),
            nom: "Seb".into(),
            contenu: "[Pad1]\r\nType = DigitalController\r\nUp = Keyboard/W\r\nUp = SDL-0/DPadUp\r\n".into(),
            livree: false,
        };
        assert!(appliquer(&r, d.path(), d.path(), true).unwrap());
        let t = std::fs::read_to_string(d.path().join("settings.ini")).unwrap();
        assert_eq!(lire_ini(&t, "Pad1", "Type"), vec!["DigitalController"]);
        assert_eq!(lire_ini(&t, "Pad1", "Up"), vec!["Keyboard/W", "SDL-0/DPadUp"]);
        assert_eq!(lire_ini(&t, "Main", "X"), vec!["1"]);
    }
}
