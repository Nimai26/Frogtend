//! Ryubing (Switch), l'alternative à Eden (✅ Seb 15/11) : la suite communautaire de Ryujinx, servie par Firehouse
//! (contrat 1.41 : `conseil.alternatives[]` de la Switch porte SES clés et SON firmware). Relevé dans le code source
//! de Ryubing 1.3.3 et sur une installation réelle sans jeu (15/11, `docs/RECHERCHE-MENU-EN-JEU.md`) :
//! - un dossier `portable\` à côté de `Ryujinx.exe` = mode portable : TOUTES ses données y vont
//!   (Ryujinx.Common/Configuration/AppDataManager.cs, `DefaultPortableDir`) ; sans lui, `%APPDATA%\Ryujinx` ;
//! - les clés : `<données>\system\prod.keys` et `title.keys` (`KeysDir = "system"`), sinon `%USERPROFILE%\.switch` ;
//!   sans `prod.keys`, le TOUT PREMIER démarrage ouvre une fenêtre « Keys not found » qui bloque : elles sont posées
//!   avant (`bios_poser`, chemin `portable/system/…` servi par Firehouse) ;
//! - le firmware : `<données>\bis\system\Contents\registered\<id>.nca\00`, UN DOSSIER par NCA qui contient le fichier
//!   `00` (Ryujinx.HLE/FileSystem/ContentManager.cs : `InstallFromZip` L554-582, `.cnmt` retiré du nom ; `LoadEntries`
//!   L131-150 lit chaque dossier de `registered` qui contient un fichier). Son menu « Install Firmware » VIDE d'abord
//!   `registered` (`FinishInstallation` L513-520) : Frogtend ne l'utilise pas, il pose lui-même, Ryubing FERMÉ, dans un
//!   `registered` vide, sans jamais rien écraser.

use crate::eden::NoteFirmware;
use crate::erreurs::{Erreur, Resultat};
use std::path::{Path, PathBuf};

/// Le dossier qui met Ryubing en mode portable (à côté de `Ryujinx.exe`) : ses données.
pub const DOSSIER_PORTABLE: &str = "portable";

/// Le programme est-il Ryubing (`Ryujinx.exe`, le nom que garde Ryubing) ?
pub fn est_ryubing(programme: &str) -> bool {
    Path::new(programme).file_name().is_some_and(|n| n.to_string_lossy().to_ascii_lowercase().starts_with("ryujinx"))
}

/// Les données de Ryubing en mode portable : `portable\` à côté du programme.
pub fn donnees(ryubing: &Path) -> PathBuf {
    ryubing.join(DOSSIER_PORTABLE)
}

/// Ryubing est-il installé en portable ? Sinon, ses données sont dans `%APPDATA%\Ryujinx` : Frogtend n'y écrit jamais
/// (et ne crée jamais `portable\` lui-même hors installation : Ryubing perdrait de vue ses parties).
pub fn portable(ryubing: &Path) -> bool {
    donnees(ryubing).is_dir()
}

/// Les clés de la personne sont-elles là (`portable\system\prod.keys`) ? Sans elles, Ryubing bloque au démarrage.
pub fn cles_presentes(ryubing: &Path) -> bool {
    donnees(ryubing).join("system").join("prod.keys").is_file()
}

/// Le dossier du firmware : `portable\bis\system\Contents\registered`.
pub fn dossier_firmware(ryubing: &Path) -> PathBuf {
    donnees(ryubing).join("bis").join("system").join("Contents").join("registered")
}

/// Un nom de dossier de `registered` tel que Ryubing l'écrit : `<32 hexa>.nca`.
fn nom_de_dossier_nca(nom: &str) -> bool {
    let n = nom.to_ascii_lowercase();
    n.strip_suffix(".nca").is_some_and(|b| b.len() == 32 && b.chars().all(|c| c.is_ascii_hexdigit()))
}

/// Les NCA posés dans `registered` : chaque dossier `<id>.nca` qui contient son fichier `00`.
pub fn nca_poses(ryubing: &Path) -> Vec<String> {
    let mut l: Vec<String> = std::fs::read_dir(dossier_firmware(ryubing))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().join("00").is_file())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| nom_de_dossier_nca(n))
        .collect();
    l.sort();
    l
}

/// Un firmware est-il déjà là ? Tout ce que Ryubing lirait compte (`LoadEntries`) : un dossier de `registered` qui
/// contient un fichier, ou un `.nca` à plat dans `Contents`. Prudent : jamais une pose par-dessus.
pub fn firmware_present(ryubing: &Path) -> bool {
    let registered = dossier_firmware(ryubing);
    let dossier_plein = std::fs::read_dir(&registered).into_iter().flatten().flatten().any(|e| {
        let p = e.path();
        p.is_dir() && std::fs::read_dir(&p).into_iter().flatten().flatten().any(|x| x.path().is_file())
    });
    let nca_a_plat = registered
        .parent()
        .and_then(|c| std::fs::read_dir(c).ok())
        .into_iter()
        .flatten()
        .flatten()
        .any(|e| e.path().is_file() && e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("nca")));
    dossier_plein || nca_a_plat
}

fn fichier_note_firmware(ryubing: &Path) -> PathBuf {
    ryubing.join(".frogtend-firmware.json")
}

/// Le firmware posé par Frogtend : sa version et SES dossiers (les seuls que Frogtend mettra à l'abri pour une mise à
/// jour).
pub fn note_firmware(ryubing: &Path) -> Option<NoteFirmware> {
    std::fs::read(fichier_note_firmware(ryubing)).ok().and_then(|o| serde_json::from_slice(&o).ok())
}

/// Note le firmware posé : tous les dossiers `<id>.nca` de `registered` (il était vide avant la pose).
pub fn noter_firmware(ryubing: &Path, version: Option<&str>) -> Resultat<()> {
    let note = NoteFirmware { version: version.map(String::from), fichiers: nca_poses(ryubing) };
    let f = fichier_note_firmware(ryubing);
    let provisoire = f.with_extension("json.tmp");
    std::fs::write(&provisoire, serde_json::to_vec_pretty(&note).unwrap_or_default())?;
    std::fs::rename(&provisoire, &f)?;
    Ok(())
}

/// Pour une mise à jour : les dossiers que FROGTEND a posés partent à l'abri (`.frogtend-sauvegardes\firmware-…`),
/// jamais effacés. S'il reste autre chose (posé par la personne ou par Ryubing), tout revient et la mise à jour est
/// refusée.
pub fn mettre_firmware_a_l_abri(ryubing: &Path) -> Resultat<(PathBuf, NoteFirmware)> {
    let note = note_firmware(ryubing).ok_or_else(|| Erreur::Refus("Ce firmware n'a pas été posé par Frogtend : il est laissé tel quel.".into()))?;
    let registered = dossier_firmware(ryubing);
    let horodatage = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let abri = ryubing
        .join(".frogtend-sauvegardes")
        .join(format!("firmware-{}-{horodatage}", crate::jeux_pc::nom_de_dossier(note.version.as_deref().unwrap_or("inconnu"))));
    std::fs::create_dir_all(&abri)?;
    let mut partis: Vec<String> = Vec::new();
    let remettre = |partis: &[String]| {
        for n in partis {
            let _ = std::fs::rename(abri.join(n), registered.join(n));
        }
        let _ = std::fs::remove_dir(&abri); // seulement s'il est vide
    };
    for n in note.fichiers.iter().filter(|n| nom_de_dossier_nca(n)) {
        let de = registered.join(n);
        if !de.is_dir() {
            continue;
        }
        if let Err(e) = std::fs::rename(&de, abri.join(n)) {
            remettre(&partis);
            return Err(Erreur::Disque(format!("{n} n'a pas pu être mis à l'abri ({e}) : rien n'a changé.")));
        }
        partis.push(n.clone());
    }
    if firmware_present(ryubing) {
        remettre(&partis);
        return Err(Erreur::Refus("Le dossier système contient aussi des fichiers que Frogtend n'a pas posés : la mise à jour est refusée, rien n'a changé.".into()));
    }
    let _ = std::fs::remove_file(fichier_note_firmware(ryubing));
    Ok((abri, note))
}

/// Remet le firmware mis à l'abri (une mise à jour qui a échoué), avec sa note. Ce qui ne revient pas reste à l'abri.
pub fn remettre_firmware(ryubing: &Path, abri: &(PathBuf, NoteFirmware)) {
    let registered = dossier_firmware(ryubing);
    let _ = std::fs::create_dir_all(&registered);
    for n in &abri.1.fichiers {
        let de = abri.0.join(n);
        if de.is_dir() && !registered.join(n).exists() {
            let _ = std::fs::rename(&de, registered.join(n));
        }
    }
    let _ = std::fs::write(fichier_note_firmware(ryubing), serde_json::to_vec_pretty(&abri.1).unwrap_or_default());
    let _ = std::fs::remove_dir(&abri.0);
}

/// Le dossier `<id>.nca` d'un fichier du zip, comme Ryubing le nomme (`InstallFromZip`) : `<hexa>.nca` ou
/// `<hexa>.cnmt.nca` (`.cnmt` retiré), ou le fichier `00` d'un NCA déjà fragmenté (`<hexa>.nca/00`). `None` : pas un
/// fichier du système.
fn dossier_pour(fichier: &Path) -> Option<String> {
    let nom = fichier.file_name()?.to_string_lossy().to_string();
    let nom = if nom == "00" { fichier.parent()?.file_name()?.to_string_lossy().to_string() } else { nom };
    if !crate::eden::nom_de_nca(&nom) {
        return None;
    }
    let bas = nom.to_ascii_lowercase();
    let base = bas.strip_suffix(".cnmt.nca").or_else(|| bas.strip_suffix(".nca"))?;
    Some(format!("{base}.nca"))
}

/// Pose le firmware servi (le .zip de Firehouse, déjà vérifié) dans un `registered` VIDE, Ryubing FERMÉ et portable :
/// extrait dans `portable\.frogtend-firmware`, chaque NCA posé dans SON dossier `<id>.nca\00` sans jamais écraser,
/// relu (taille). Tout ou rien : au moindre échec, ce que cette pose a créé est retiré. Après la pose, le nombre de
/// NCA relus dans `registered` doit être celui du zip. Rend ce nombre.
pub fn poser_firmware(ryubing: &Path, zip: &Path) -> Resultat<usize> {
    if !portable(ryubing) {
        return Err(Erreur::Refus("Ryubing n'est pas installé en mode portable : Frogtend n'écrit pas dans ses données de Windows.".into()));
    }
    if firmware_present(ryubing) {
        return Err(Erreur::Refus("Un firmware est déjà installé dans Ryubing : il est laissé tel quel.".into()));
    }
    let registered = dossier_firmware(ryubing);
    let tmp = donnees(ryubing).join(".frogtend-firmware");
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp)?; // le NÔTRE, d'une pose coupée
    }
    let r = (|| -> Resultat<usize> {
        crate::installation::decompresser(zip, crate::installation::Format::Zip, &tmp)?;
        let mut nca: Vec<(PathBuf, String)> = Vec::new();
        let mut pile = vec![tmp.clone()];
        while let Some(d) = pile.pop() {
            for e in std::fs::read_dir(&d)?.flatten() {
                let p = e.path();
                if p.is_dir() {
                    pile.push(p);
                } else if let Some(dossier) = dossier_pour(&p) {
                    nca.push((p, dossier));
                }
            }
        }
        if nca.is_empty() {
            return Err(Erreur::Serveur("Le firmware reçu ne contient aucun fichier du système (.nca).".into()));
        }
        // Deux fichiers pour le même dossier (un même NCA en double, ou `x.nca` et `x.cnmt.nca`) : Ryubing n'en
        // garderait qu'un. Refusé AVANT de poser quoi que ce soit.
        let mut noms = std::collections::HashSet::new();
        if !nca.iter().all(|(_, d)| noms.insert(d.clone())) {
            return Err(Erreur::Serveur("Le firmware reçu contient deux fois le même fichier : rien n'est posé.".into()));
        }
        std::fs::create_dir_all(&registered)?;
        let mut crees: Vec<PathBuf> = Vec::new();
        let r = nca.iter().try_for_each(|(f, dossier)| -> Resultat<()> {
            let taille = std::fs::metadata(f)?.len();
            let d = registered.join(dossier);
            // `create_dir` (pas `_all`) : échoue si le dossier existe déjà — jamais par-dessus.
            std::fs::create_dir(&d).map_err(|e| Erreur::Disque(format!("{dossier} n'a pas pu être créé ({e}).")))?;
            crees.push(d.clone());
            let cible = d.join("00");
            crate::pack_mame::deplacer_sans_ecraser(f, &cible).map_err(|e| Erreur::Disque(format!("{dossier} n'a pas pu être posé ({e}).")))?;
            if std::fs::metadata(&cible).map(|m| m.len()).ok() != Some(taille) {
                return Err(Erreur::Disque(format!("{dossier} est abîmé après la pose.")));
            }
            Ok(())
        });
        // Relu : chaque NCA du zip est dans son dossier, rien d'autre.
        let r = r.and_then(|_| {
            let relus = nca_poses(ryubing).len();
            if relus != nca.len() {
                return Err(Erreur::Disque(format!("Après la pose, {relus} fichier(s) du système relus au lieu de {} : rien n'est gardé.", nca.len())));
            }
            Ok(nca.len())
        });
        if r.is_err() {
            for d in &crees {
                let _ = std::fs::remove_file(d.join("00"));
                let _ = std::fs::remove_dir(d); // vide seulement : jamais un fichier d'un autre
            }
        }
        r
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn zip(chemin: &Path, entrees: &[(&str, &[u8])]) {
        let mut z = zip::ZipWriter::new(std::fs::File::create(chemin).unwrap());
        let o = zip::write::SimpleFileOptions::default();
        for (n, c) in entrees {
            z.start_file(*n, o).unwrap();
            z.write_all(c).unwrap();
        }
        z.finish().unwrap();
    }

    const A: &str = "0123456789abcdef0123456789abcdef";
    const B: &str = "fedcba9876543210fedcba9876543210";

    #[test]
    fn le_programme_et_ses_dossiers_sont_ceux_de_ryubing() {
        assert!(est_ryubing(r"E:\Emu\Ryubing\Ryujinx.exe") && !est_ryubing(r"E:\Emu\Eden\eden.exe"));
        let d = Path::new(r"E:\Emu\Ryubing");
        assert_eq!(dossier_firmware(d), d.join("portable").join("bis").join("system").join("Contents").join("registered"));
        assert!(!crate::eden::est_eden(r"E:\Emu\Ryubing\Ryujinx.exe"), "jamais de la famille d'Eden");
    }

    #[test]
    fn le_firmware_est_pose_un_dossier_par_nca_comme_ryubing_l_installe() {
        let d = tempfile::tempdir().unwrap();
        let ryu = d.path().join("Ryubing");
        std::fs::create_dir_all(ryu.join("portable")).unwrap();
        let z = d.path().join("firmware-switch.zip");
        // Le zip servi : un .nca, un .cnmt.nca, un NCA déjà fragmenté, et un fichier étranger (jamais posé).
        let c = "00112233445566778899aabbccddeeff";
        zip(&z, &[
            (&format!("18.1.0/{A}.nca"), b"AAAA"),
            (&format!("18.1.0/{B}.cnmt.nca"), b"BB"),
            (&format!("18.1.0/{c}.nca/00"), b"CCC"),
            ("18.1.0/lisez-moi.txt", b"x"),
        ]);
        assert!(!firmware_present(&ryu));
        assert_eq!(poser_firmware(&ryu, &z).unwrap(), 3);
        let reg = dossier_firmware(&ryu);
        assert_eq!(std::fs::read(reg.join(format!("{A}.nca")).join("00")).unwrap(), b"AAAA");
        assert_eq!(std::fs::read(reg.join(format!("{B}.nca")).join("00")).unwrap(), b"BB", "« .cnmt » retiré du nom, comme Ryubing");
        assert_eq!(std::fs::read(reg.join(format!("{c}.nca")).join("00")).unwrap(), b"CCC");
        assert!(!reg.join("lisez-moi.txt").exists());
        assert_eq!(nca_poses(&ryu).len(), 3);
        assert!(!ryu.join("portable").join(".frogtend-firmware").exists(), "rien ne reste à l'écart");
        assert!(firmware_present(&ryu));
        // Déjà un firmware : jamais touché.
        assert!(matches!(poser_firmware(&ryu, &z), Err(Erreur::Refus(_))));
        assert_eq!(std::fs::read(reg.join(format!("{A}.nca")).join("00")).unwrap(), b"AAAA");
    }

    #[test]
    fn jamais_hors_du_mode_portable_ni_par_dessus_ni_a_moitie() {
        let d = tempfile::tempdir().unwrap();
        let z = d.path().join("f.zip");
        zip(&z, &[(&format!("{A}.nca"), b"A")]);
        // Sans `portable\` : rien n'est créé (il ferait basculer un Ryubing installé à la main).
        let main = d.path().join("ALaMain");
        std::fs::create_dir_all(&main).unwrap();
        assert!(matches!(poser_firmware(&main, &z), Err(Erreur::Refus(_))));
        assert!(!main.join("portable").exists());
        // Un firmware posé par la personne (à plat, ou par Ryubing) : jamais remplacé.
        let ryu = d.path().join("Ryubing");
        std::fs::create_dir_all(dossier_firmware(&ryu).join(format!("{B}.nca"))).unwrap();
        std::fs::write(dossier_firmware(&ryu).join(format!("{B}.nca")).join("00"), b"perso").unwrap();
        assert!(matches!(poser_firmware(&ryu, &z), Err(Erreur::Refus(_))));
        assert_eq!(std::fs::read(dossier_firmware(&ryu).join(format!("{B}.nca")).join("00")).unwrap(), b"perso");
        // Un `registered` vide (Ryubing déjà lancé une fois le crée) : la pose est permise.
        let neuf = d.path().join("Neuf");
        std::fs::create_dir_all(dossier_firmware(&neuf)).unwrap();
        assert!(!firmware_present(&neuf));
        // Deux fois le même NCA (`x.nca` et `x.cnmt.nca`) : rien n'est posé.
        let double = d.path().join("double.zip");
        zip(&double, &[(&format!("a/{A}.nca"), b"x"), (&format!("b/{A}.cnmt.nca"), b"y"), (&format!("b/{B}.nca"), b"z")]);
        assert!(poser_firmware(&neuf, &double).is_err());
        assert!(!firmware_present(&neuf) && nca_poses(&neuf).is_empty(), "jamais un firmware à moitié");
        // Un zip sans aucun NCA : refusé.
        let vide = d.path().join("vide.zip");
        zip(&vide, &[("lisez-moi.txt", b"x")]);
        assert!(poser_firmware(&neuf, &vide).is_err());
        assert_eq!(poser_firmware(&neuf, &z).unwrap(), 1);
    }

    #[test]
    fn une_mise_a_jour_met_l_ancien_a_l_abri_et_le_remet_si_elle_echoue() {
        let d = tempfile::tempdir().unwrap();
        let ryu = d.path().join("Ryubing");
        std::fs::create_dir_all(ryu.join("portable")).unwrap();
        let z = d.path().join("f.zip");
        zip(&z, &[(&format!("{A}.nca"), b"A")]);
        poser_firmware(&ryu, &z).unwrap();
        noter_firmware(&ryu, Some("18.1.0")).unwrap();
        assert_eq!(note_firmware(&ryu).unwrap(), NoteFirmware { version: Some("18.1.0".into()), fichiers: vec![format!("{A}.nca")] });
        // Un dossier système qui n'est pas de Frogtend : refusé, rien ne bouge.
        let autre = dossier_firmware(&ryu).join(format!("{B}.nca"));
        std::fs::create_dir_all(&autre).unwrap();
        std::fs::write(autre.join("00"), b"x").unwrap();
        assert!(mettre_firmware_a_l_abri(&ryu).is_err());
        assert!(dossier_firmware(&ryu).join(format!("{A}.nca")).join("00").is_file() && note_firmware(&ryu).is_some());
        std::fs::remove_dir_all(&autre).unwrap();
        let abri = mettre_firmware_a_l_abri(&ryu).unwrap();
        assert!(!firmware_present(&ryu) && note_firmware(&ryu).is_none());
        remettre_firmware(&ryu, &abri);
        assert!(dossier_firmware(&ryu).join(format!("{A}.nca")).join("00").is_file());
        assert_eq!(note_firmware(&ryu).unwrap().version.as_deref(), Some("18.1.0"));
        mettre_firmware_a_l_abri(&ryu).unwrap();
        let abris: Vec<_> = std::fs::read_dir(ryu.join(".frogtend-sauvegardes")).unwrap().flatten().collect();
        assert_eq!(abris.len(), 1, "un refus ne laisse pas d'abri vide");
        assert!(abris[0].path().join(format!("{A}.nca")).join("00").is_file(), "à l'abri, jamais effacé");
    }

    #[test]
    fn les_cles_sont_cherchees_dans_portable_system() {
        let d = tempfile::tempdir().unwrap();
        assert!(!cles_presentes(d.path()));
        std::fs::create_dir_all(d.path().join("portable").join("system")).unwrap();
        std::fs::write(d.path().join("portable").join("system").join("prod.keys"), b"k").unwrap();
        assert!(cles_presentes(d.path()) && portable(d.path()));
    }
}
