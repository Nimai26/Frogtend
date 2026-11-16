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
use serde_json::Value;
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

// ---------------------------------------------------------------------------------------------------------------
// Lot 2 — réglé dès la première partie (✅ Seb 15/11 : tout sans intervention de la personne). Le Config.json de la
// personne (dans SA racine, lot 3), Ryubing FERMÉ : posé COMPLET s'il manque (le modèle relevé le 15/11, version 70 :
// un champ absent vaudrait 0 ou null), puis réglé UNE fois par personne (marque : une valeur d'origine ne se distingue
// pas d'un choix). Mis à l'abri avant, relu après, remis s'il ne se relit pas : un Config.json illisible est remplacé
// en SILENCE par les réglages d'origine (Program.cs L211-219, `TryLoad` : version 0 ou JSON refusé). Le plein écran :
// `-f` sur la ligne (déjà là), jamais `start_fullscreen` — Ryubing ouvert hors de Frogtend reste en fenêtre.
// ---------------------------------------------------------------------------------------------------------------

/// Le Config.json d'origine de Ryubing 1.3.3 (relevé réel du 15/11, sans aucun secret).
const MODELE_CONFIG: &str = include_str!("ryubing/Config-origine.json");
/// Le bloc du joueur 1 avec une manette Xbox (relevé réel du 15/11) : disposition d'origine de Ryubing.
const MODELE_MANETTE: &str = include_str!("ryubing/manette-xbox.json");
/// Le fichier de réglages, à la racine des données.
pub const FICHIER_CONFIG: &str = "Config.json";
/// La version du Config.json que Frogtend sait lire (`ConfigurationFileFormat.CurrentVersion` de Ryubing 1.3.3) : un
/// fichier plus récent n'est jamais touché.
pub const VERSION_CONFIG: u64 = 70;
/// « Réglages de la première partie déjà posés pour cette personne » (dans sa racine).
const MARQUE_REGLAGES: &str = ".frogtend-ryubing-reglages";
/// L'identifiant de la manette que Frogtend a posée au joueur 1 (pour la suivre d'une manette à l'autre, jamais celle
/// choisie par la personne).
const MARQUE_MANETTE: &str = ".frogtend-ryubing-manette";

/// Le Config.json d'une racine de données.
pub fn config_de(racine: &Path) -> PathBuf {
    racine.join(FICHIER_CONFIG)
}

/// Un Config.json à côté de `Ryujinx.exe` passe devant tous les autres (Program.cs L183-196) : Frogtend n'écrit alors
/// rien (il réglerait un fichier que Ryubing ne lit pas).
pub fn config_a_cote_du_programme(ryubing: &Path) -> bool {
    ryubing.join(FICHIER_CONFIG).is_file()
}

/// Un Config.json que Ryubing chargera : du JSON, `version` entre 1 et 70, et chaque manette sur un pilote connu
/// (`WindowKeyboard` ou `GamepadSDL2` ; un autre ferait refuser tout le fichier).
pub fn config_valide(texte: &str) -> bool {
    let Ok(v) = serde_json::from_str::<Value>(texte) else { return false };
    v.is_object()
        && v["version"].as_u64().is_some_and(|n| (1..=VERSION_CONFIG).contains(&n))
        && v["input_config"]
            .as_array()
            .is_some_and(|l| l.iter().all(|e| matches!(e["backend"].as_str(), Some("WindowKeyboard" | "GamepadSDL2"))))
}

/// Les réglages de la première partie (valeurs relues dans le code de Ryubing 1.3.3 : `SystemLanguage.French`,
/// `FocusLostType.PauseEmulation` — le menu de Frogtend le met ainsi en pause —, `UpdaterType.Off` — Frogtend fait les
/// mises à jour —, `fr_FR` dans assets/locales.json).
fn reglages_de_frogtend() -> [(&'static str, Value); 7] {
    [
        ("system_language", Value::from("French")),
        ("language_code", Value::from("fr_FR")),
        ("show_confirm_exit", Value::from(false)),
        ("focus_lost_action_type", Value::from("PauseEmulation")),
        ("update_checker_type", Value::from("Off")),
        ("enable_discord_integration", Value::from(false)),
        // Region.cs L11 : `Europe` (la console d'une maison française).
        ("system_region", Value::from("Europe")),
    ]
}

/// Le GUID SDL tel que .NET l'écrit (`Guid.ToString()` : les 3 premiers groupes en petit-boutiste), puis comme Ryubing
/// le rend stable (SDL2GamepadDriver.cs L51-77 : les 4 premiers caractères, le CRC du nom, remplacés par « 0000 »).
pub fn guid_net_stable(o: &[u8; 16]) -> String {
    let d1 = u32::from_le_bytes([o[0], o[1], o[2], o[3]]);
    let d2 = u16::from_le_bytes([o[4], o[5]]);
    let d3 = u16::from_le_bytes([o[6], o[7]]);
    let fin: String = o[10..].iter().map(|b| format!("{b:02x}")).collect();
    let s = format!("{d1:08x}-{d2:04x}-{d3:04x}-{:02x}{:02x}-{fin}", o[8], o[9]);
    format!("0000{}", &s[4..])
}

/// L'identifiant que Ryubing donne à une manette XInput (« <n>-<GUID> », n = 0 pour la première de ce modèle) : SDL2,
/// pilote RAWINPUT (`SDL_rawinputjoystick.c` : bus USB, fournisseur, produit, version, signature « r », donnée 0).
/// Relevé réel (manette Xbox One 045e:02ff) : `0-00000003-045e-0000-ff02-000000007200`. La manette virtuelle de
/// Sunshine (045e:028e, celle d'une Xbox 360) suit la même formule — sa version reste à relever en vrai.
pub fn id_manette(fournisseur: u16, produit: u16, version: u16) -> String {
    let mut o = [0u8; 16];
    o[0] = 0x03; // bus USB (le CRC, octets 2-3, est retiré par Ryubing)
    o[4..6].copy_from_slice(&fournisseur.to_le_bytes());
    o[8..10].copy_from_slice(&produit.to_le_bytes());
    o[12..14].copy_from_slice(&version.to_le_bytes());
    o[14] = b'r';
    format!("0-{}", guid_net_stable(&o))
}

/// Le bloc du joueur 1 pour la manette `id` (le modèle relevé, son identifiant changé).
fn bloc_manette(id: &str) -> Value {
    let mut b: Value = serde_json::from_str(MODELE_MANETTE).unwrap_or(Value::Null);
    b["id"] = Value::from(id);
    b["name"] = Value::from("Manette XInput (0)");
    b
}

/// Ce que `regler_config` a fait.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ReglageConfig {
    /// Le modèle complet a été posé (il n'y avait pas de Config.json).
    pub pose: bool,
    /// Les réglages de la première partie ont été écrits (une seule fois par personne).
    pub regle: bool,
    /// La manette branchée a été posée au joueur 1.
    pub manette: bool,
    /// Le clavier d'origine a été remis à la place de la manette posée par Frogtend.
    pub clavier: bool,
    /// Une manette pas encore relevée était branchée : le clavier est gardé (à dire au journal).
    pub manette_inconnue: bool,
    /// L'avance rapide a été liée à F13 (lot 5, une seule fois).
    pub turbo: bool,
    /// Sa vitesse (`tick_scalar`) était à 0 : mise à 200 (`poser_vitesse_turbo`).
    pub vitesse: bool,
}

/// Ce que Frogtend fait du joueur 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Joueur1<'a> {
    /// Rien n'est touché (aucune manette XInput branchée).
    Rien,
    /// La manette XInput branchée (son identifiant Ryubing).
    Manette(&'a str),
    /// Le clavier choisi pour ce jeu : la manette posée par Frogtend est retirée.
    Clavier,
}

/// La seule manette dont l'identifiant a été RELEVÉ en vrai (✅ 15/11 : manette Xbox One 045e:02ff, en USB, par XInput).
/// Tant que la manette de Sunshine et le Bluetooth ne sont pas relevés, aucune autre n'est posée : un identifiant faux
/// laisserait le joueur 1 sans aucune commande (le clavier n'est plus lu, `enable_keyboard` vaut false).
pub const MANETTE_RELEVEE: &str = "0-00000003-045e-0000-ff02-000000007200";

/// Écrit `texte` à la place de `f` (provisoire puis renommage : jamais un fichier à moitié écrit).
fn ecrire_texte(f: &Path, texte: &str) -> Resultat<()> {
    if let Some(p) = f.parent() {
        std::fs::create_dir_all(p)?;
    }
    let provisoire = f.with_extension("json.frogtend");
    std::fs::write(&provisoire, texte)?;
    std::fs::rename(&provisoire, f)?;
    Ok(())
}

/// Le Config.json de la personne (racine `racine`, sous le dossier de Ryubing), Ryubing FERMÉ : posé complet s'il
/// manque, réglé une fois (marque). Le joueur 1 (`joueur1`) : la manette branchée, SI c'est celle déjà relevée
/// (`MANETTE_RELEVEE`), posée s'il est encore au clavier (`WindowKeyboard`) ou s'il a la manette que Frogtend y avait
/// mise ; le clavier choisi pour ce jeu (ou une manette pas encore relevée) remet le clavier d'origine à la place de la
/// manette de Frogtend ; jamais par-dessus une manette réglée par la personne. Un `games\<numéro>\Config.json` de la
/// personne n'est jamais touché (`configs_par_jeu`). Le fichier est mis à l'abri avant, relu après ; s'il ne se relit pas,
/// le texte d'avant est remis.
pub fn regler_config(ryubing: &Path, racine: &Path, joueur1: Joueur1) -> Resultat<ReglageConfig> {
    if config_a_cote_du_programme(ryubing) {
        return Err(Erreur::Refus("Un Config.json à côté de Ryujinx.exe passe devant celui de la personne : Ryubing n'est pas réglé.".into()));
    }
    if !crate::lancement::processus_du_dossier(ryubing).is_empty() {
        return Err(Erreur::Refus("Ryubing est ouvert : ses réglages ne sont pas touchés (il les réécrirait en quittant).".into()));
    }
    let f = config_de(racine);
    let mut r = ReglageConfig::default();
    let avant = match std::fs::read_to_string(&f) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Jamais un fichier minimal : le modèle COMPLET relevé (un champ absent vaudrait 0 ou null).
            ecrire_texte(&f, MODELE_CONFIG)?;
            if std::fs::read_to_string(&f).ok().as_deref() != Some(MODELE_CONFIG) {
                let _ = std::fs::remove_file(&f);
                return Err(Erreur::Disque("Le Config.json de Ryubing n'a pas pu être posé.".into()));
            }
            r.pose = true;
            MODELE_CONFIG.to_string()
        }
        Err(e) => return Err(e.into()),
    };
    if !config_valide(&avant) {
        return Err(Erreur::Refus("Le Config.json de cette personne ne se lit pas (ou vient d'un Ryubing plus récent) : il n'est pas touché.".into()));
    }
    let mut v: Value = serde_json::from_str(&avant).map_err(|_| Erreur::Disque("Config.json de Ryubing illisible.".into()))?;
    let marque_reglages = racine.join(MARQUE_REGLAGES);
    if !marque_reglages.exists() {
        for (cle, valeur) in reglages_de_frogtend() {
            v[cle] = valeur;
        }
        r.regle = true;
    }
    // L'avance rapide du menu (lot 5) : `turbo_mode` lié à F13, une seule fois par personne, seulement s'il est encore
    // « Unbound » et que F13 n'est à aucune autre action.
    if !racine.join(MARQUE_TURBO).exists() {
        r.turbo = poser_turbo(&mut v);
    }
    r.vitesse = poser_vitesse_turbo(&mut v);
    // Le joueur 1 : la manette branchée (seulement celle déjà relevée), sinon le clavier d'origine remis à la place de
    // la manette que FROGTEND avait posée — jamais une manette réglée par la personne.
    let voulue = match joueur1 {
        Joueur1::Manette(id) if id == MANETTE_RELEVEE => Some(id),
        Joueur1::Manette(_) => {
            r.manette_inconnue = true;
            None
        }
        Joueur1::Clavier => None,
        Joueur1::Rien => return ecrire_si_change(ryubing, racine, &f, &avant, v, r, None),
    };
    let posee_par_frogtend = std::fs::read_to_string(racine.join(MARQUE_MANETTE)).ok();
    if let Some(j1) = v["input_config"].as_array_mut().and_then(|l| l.iter_mut().find(|e| e["player_index"] == "Player1")) {
        let au_clavier = j1["backend"] == "WindowKeyboard";
        let la_notre = j1["backend"] == "GamepadSDL2" && posee_par_frogtend.as_deref().is_some_and(|p| j1["id"] == p.trim());
        match voulue {
            Some(id) if (au_clavier || la_notre) && j1["id"] != id => {
                *j1 = bloc_manette(id);
                r.manette = true;
            }
            None if la_notre => {
                let modele: Value = serde_json::from_str(MODELE_CONFIG).unwrap_or(Value::Null);
                *j1 = modele["input_config"][0].clone();
                r.clavier = true;
            }
            _ => {}
        }
    }
    ecrire_si_change(ryubing, racine, &f, &avant, v, r, voulue)
}

/// Écrit le Config.json réglé s'il a changé (à l'abri avant, relu après, celui d'avant remis s'il ne se relit pas),
/// puis les marques.
fn ecrire_si_change(ryubing: &Path, racine: &Path, f: &Path, avant: &str, v: Value, r: ReglageConfig, manette: Option<&str>) -> Resultat<ReglageConfig> {
    if r.regle || r.manette || r.clavier || r.turbo || r.vitesse {
        ecrire_verifie(ryubing, f, avant, &v)?;
        if r.regle {
            std::fs::write(racine.join(MARQUE_REGLAGES), b"")?;
        }
        if r.manette {
            std::fs::write(racine.join(MARQUE_MANETTE), manette.unwrap_or_default())?;
        }
        if r.clavier {
            let _ = std::fs::remove_file(racine.join(MARQUE_MANETTE));
        }
    }
    // Le turbo a été décidé (posé, ou laissé au choix de la personne) : plus jamais revu.
    if !racine.join(MARQUE_TURBO).exists() {
        std::fs::write(racine.join(MARQUE_TURBO), b"")?;
    }
    Ok(r)
}

/// Réécrit le Config.json (`v`), Ryubing fermé : à l'abri avant, relu après (valide, et ce qui a été écrit) ; sinon le
/// texte d'avant (`avant`) revient.
fn ecrire_verifie(ryubing: &Path, f: &Path, avant: &str, v: &Value) -> Resultat<()> {
    let texte = serde_json::to_string_pretty(v).map_err(|e| Erreur::Disque(format!("Config.json de Ryubing impossible à écrire ({e}).")))?;
    crate::emulateurs_profils::mettre_a_l_abri(ryubing, f)?;
    ecrire_texte(f, &texte)?;
    let relu = std::fs::read_to_string(f).unwrap_or_default();
    if !config_valide(&relu) || serde_json::from_str::<Value>(&relu).ok().as_ref() != Some(v) {
        ecrire_texte(f, avant)?;
        return Err(Erreur::Disque("Le Config.json de Ryubing ne se relisait pas après l'écriture : celui d'avant est remis.".into()));
    }
    Ok(())
}

/// La pause du menu est-elle réglée dans la racine de cette personne (`PauseEmulation`) ? Sinon (choix de la
/// personne, fichier absent), Ryubing est figé pendant le menu.
pub fn pause_reglee(racine: &Path) -> bool {
    let en_pause = |f: &Path| {
        std::fs::read_to_string(f).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).is_some_and(|v| v["focus_lost_action_type"] == "PauseEmulation")
    };
    // Frogtend ne sait pas quel jeu sera lancé (numéro du titre) : un seul réglage propre à un jeu sans la pause, et
    // Ryubing est figé pendant le menu (prudent : figé à tort ne coûte rien).
    en_pause(&config_de(racine)) && configs_par_jeu(racine).iter().all(|j| en_pause(&racine.join("games").join(j).join(FICHIER_CONFIG)))
}

/// Les jeux qui ont LEUR réglage dans cette racine (`games\<numéro>\Config.json`, Program.cs L164-179 : il remplace
/// celui de la personne pour ce jeu). Frogtend n'y écrit jamais ; seuls ceux que Ryubing lirait comptent.
pub fn configs_par_jeu(racine: &Path) -> Vec<String> {
    let mut l: Vec<String> = std::fs::read_dir(racine.join("games"))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| std::fs::read_to_string(e.path().join(FICHIER_CONFIG)).is_ok_and(|t| config_valide(&t)))
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    l.sort();
    l
}

// ---------------------------------------------------------------------------------------------------------------
// Lot 3 — un dossier par personne (✅ Seb 15/11, voie B : `-r <dossier>`, CommandLineState.cs). La racine de la
// personne est `Profils\<nom>\` (convention de Frogtend) : ses réglages, ses utilisateurs (`system\Profiles.json`), ses
// parties ET l'index des parties (`bis\system\save\8000000000000000`, tenu par LibHac : un index par racine). Ses clés y
// sont copiées (`system\`, KeysDirPath) ; le firmware est PARTAGÉ (jonction `bis\system\Contents` → celui de
// `portable\`) ; `games\` est à chacun (✅ Seb 15/11) : caches de shaders et de code (PPTC) recompilés pour chaque
// personne, et un `games\<numéro>\Config.json` de la personne passe devant le sien pour ce jeu (Frogtend n'y écrit
// jamais). Une racine absente : Ryubing retomberait EN
// SILENCE sur `portable\` (AppDataManager.cs L84-95) — la mauvaise personne jouerait : refusé.
// ---------------------------------------------------------------------------------------------------------------

/// La racine des données de la personne.
pub fn racine(ryubing: &Path, profil: &str) -> PathBuf {
    crate::emulateurs_profils::dossier_du_profil(ryubing, profil)
}

/// Les parties d'une racine (`bis\user\save`) : y en a-t-il au moins une ?
pub fn a_des_parties(racine: &Path) -> bool {
    crate::installation::fichiers_de(&racine.join("bis").join("user").join("save")).is_ok_and(|l| !l.is_empty())
}

/// La note des clés copiées par Frogtend dans la racine (nom → empreinte) : une clé de la personne n'est jamais remplacée.
const NOTE_CLES: &str = ".frogtend-ryubing-cles.json";

/// Copie les clés (`*.keys` de `portable\system\`) dans `<racine>\system\` : copie vérifiée (empreinte), jamais
/// par-dessus une clé que Frogtend n'a pas posée ; une clé de Frogtend devenue ancienne (nouvelles clés servies) est
/// remplacée. Rend le nombre de clés copiées.
pub fn copier_les_cles(ryubing: &Path, racine: &Path) -> Resultat<usize> {
    let de = donnees(ryubing).join("system");
    let vers = racine.join("system");
    std::fs::create_dir_all(&vers)?;
    let f_note = racine.join(NOTE_CLES);
    let mut note: std::collections::BTreeMap<String, String> = std::fs::read(&f_note).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default();
    let mut copiees = 0;
    for e in std::fs::read_dir(&de).into_iter().flatten().flatten() {
        let p = e.path();
        let nom = e.file_name().to_string_lossy().to_string();
        if !p.is_file() || !nom.to_ascii_lowercase().ends_with(".keys") {
            continue;
        }
        let empreinte = crate::sauvegarde::empreinte(&p)?;
        let cible = vers.join(&nom);
        if cible.is_file() {
            let la = crate::sauvegarde::empreinte(&cible)?;
            if la == empreinte {
                note.insert(nom, empreinte);
                continue;
            }
            if note.get(&nom) != Some(&la) {
                continue; // posée par la personne : jamais touchée
            }
        }
        let provisoire = vers.join(format!("{nom}.frogtend"));
        std::fs::copy(&p, &provisoire)?;
        if crate::sauvegarde::empreinte(&provisoire)? != empreinte {
            let _ = std::fs::remove_file(&provisoire);
            return Err(Erreur::Disque(format!("La copie de {nom} pour Ryubing est abîmée : rien n'est posé.")));
        }
        std::fs::rename(&provisoire, &cible)?;
        note.insert(nom, empreinte);
        copiees += 1;
    }
    std::fs::write(&f_note, serde_json::to_vec_pretty(&note).unwrap_or_default())?;
    Ok(copiees)
}

/// `<parent>\<nom>` devient une jonction vers `cible` (sauf si elle y mène déjà).
fn relier(parent: &Path, nom: &str, cible: &Path) -> Resultat<()> {
    std::fs::create_dir_all(parent)?;
    std::fs::create_dir_all(cible)?;
    let lien = parent.join(nom);
    let deja = crate::garde::est_un_lien(&lien) && std::fs::canonicalize(&lien).ok().is_some_and(|l| std::fs::canonicalize(cible).ok() == Some(l));
    if deja {
        return Ok(());
    }
    crate::emulateurs_profils::relier_dossier(parent, nom, cible, "Ryubing")
}

/// Prépare la racine de la personne : `system\` et ses clés, le firmware partagé par jonction, son propre `games\`.
pub fn preparer_racine(ryubing: &Path, racine: &Path) -> Resultat<()> {
    std::fs::create_dir_all(racine.join("system"))?;
    copier_les_cles(ryubing, racine)?;
    let portable = donnees(ryubing);
    relier(&racine.join("bis").join("system"), "Contents", &portable.join("bis").join("system").join("Contents"))?;
    // `games\` : chacun le sien (✅ Seb 15/11) — caches recompilés pour chaque personne, réglages par jeu à elle.
    std::fs::create_dir_all(racine.join("games"))?;
    Ok(())
}

/// Ce qui est sauvegardé chez Firehouse d'une racine de Ryubing (`relatif` : sous `Profils\<nom>\`) : les parties
/// (`bis\user\save`, `bis\user\saveMeta`), toutes les sauvegardes système (`bis\system\save`, dont l'index des parties
/// `8000000000000000` et la base des Mii `8000000000000030`), les utilisateurs
/// (`system\Profiles.json`), les réglages (`Config.json`, sans secret) et la marque de leur réglage. Jamais les clés,
/// le firmware, les caches (`games\`), ni les notes de Frogtend.
pub fn a_sauvegarder(relatif: &str) -> bool {
    let r = relatif.replace('\\', "/").to_ascii_lowercase();
    if r.ends_with(".keys") {
        return false;
    }
    r.starts_with("bis/user/save/")
        || r.starts_with("bis/user/savemeta/")
        || r.starts_with("bis/system/save/")
        || r == "system/profiles.json"
        || r == "config.json"
        || r == MARQUE_REGLAGES
}

/// Le Config.json tel qu'il part chez Firehouse : une COPIE, `multiplayer_ldn_passphrase` vidé (le mot de passe des
/// parties en réseau local est un secret). `None` : illisible, il ne part pas. Le fichier de la personne n'est jamais
/// modifié.
pub fn config_a_envoyer(texte: &str) -> Option<String> {
    let mut v: Value = serde_json::from_str(texte).ok().filter(Value::is_object)?;
    v["multiplayer_ldn_passphrase"] = Value::from("");
    serde_json::to_string_pretty(&v).ok()
}

/// Les dossiers d'une racine qui vont ensemble : les parties, leurs métadonnées, et TOUTES les sauvegardes système
/// (l'index des parties `8000000000000000`, la base des Mii `8000000000000030`…).
const DOSSIERS_DU_BLOC: [&str; 3] = ["bis/user/save", "bis/user/saveMeta", "bis/system/save"];

/// Les fichiers d'une racine à sauvegarder (chemins relatifs, « / ») : seulement sous les dossiers retenus, sans
/// jamais parcourir les jonctions du firmware ni de `games\` (`a_sauvegarder`).
pub fn fichiers_a_sauvegarder(racine: &Path) -> Resultat<Vec<String>> {
    let mut l = Vec::new();
    for sous in DOSSIERS_DU_BLOC {
        let d = racine.join(sous);
        if d.is_dir() && !crate::garde::est_un_lien(&d) {
            l.extend(crate::installation::fichiers_de(&d)?.into_iter().map(|r| format!("{sous}/{r}")));
        }
    }
    for f in ["system/Profiles.json", FICHIER_CONFIG, MARQUE_REGLAGES] {
        if racine.join(f).is_file() {
            l.push(f.to_string());
        }
    }
    l.retain(|r| a_sauvegarder(r));
    Ok(l)
}

/// Pose des parties EN BLOC dans une racine SANS partie : chaque élément (source : fichier ou dossier, chemin relatif
/// dans la racine) est d'abord copié et vérifié à l'écart (`.frogtend-bloc`), puis mis en place ; ce qui occupait sa
/// place (un index vide, un `Profiles.json` d'origine) part à l'abri (`.frogtend-sauvegardes\ryubing-…`), jamais
/// effacé. Au moindre échec, rien ne reste à moitié. Rend le nombre de fichiers posés.
pub fn poser_en_bloc(ryubing: &Path, racine: &Path, elements: &[(PathBuf, &str)]) -> Resultat<usize> {
    if !crate::lancement::processus_du_dossier(ryubing).is_empty() {
        return Err(Erreur::Refus("Ryubing est ouvert : ses parties ne sont pas touchées.".into()));
    }
    // Des parties sans leur index : Ryubing ne les verrait pas (ou les écraserait). Jamais posées ainsi.
    let parties = elements.iter().any(|(p, rel)| *rel == "bis/user/save" && crate::installation::fichiers_de(p).is_ok_and(|l| !l.is_empty()));
    let index = elements.iter().any(|(p, rel)| {
        let i = match *rel {
            "bis/system/save" => p.join("8000000000000000"),
            "bis/system/save/8000000000000000" => p.clone(),
            _ => return false,
        };
        crate::installation::fichiers_de(&i).is_ok_and(|l| !l.is_empty())
    });
    if parties && !index {
        return Err(Erreur::Refus("Des parties de Ryubing arrivent sans leur index (bis\\system\\save\\8000000000000000) : rien n'est posé.".into()));
    }
    if a_des_parties(racine) {
        return Err(Erreur::Refus("Cette personne a déjà des parties dans Ryubing : deux index de parties ne se mêlent jamais, rien n'est posé.".into()));
    }
    let ecart = racine.join(".frogtend-bloc");
    if ecart.exists() {
        std::fs::remove_dir_all(&ecart)?; // le NÔTRE, d'une pose coupée
    }
    let r = (|| -> Resultat<usize> {
        let mut total = 0;
        for (source, rel) in elements {
            let vers = ecart.join(rel);
            if source.is_dir() {
                total += crate::eden::copier_dossier_entier(source, &vers)?;
            } else {
                std::fs::create_dir_all(vers.parent().unwrap_or(&ecart))?;
                std::fs::copy(source, &vers)?;
                if std::fs::metadata(&vers)?.len() != std::fs::metadata(source)?.len() {
                    return Err(Erreur::Disque(format!("La copie de {rel} est incomplète : rien n'est posé.")));
                }
                total += 1;
            }
        }
        let horodatage = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let nom = racine.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let abri = ryubing.join(".frogtend-sauvegardes").join(format!("ryubing-{horodatage}")).join(nom);
        // (cible posée, ce qui était là avant et où il est à l'abri)
        let mut faits: Vec<(PathBuf, Option<PathBuf>)> = Vec::new();
        let poser = |faits: &mut Vec<(PathBuf, Option<PathBuf>)>| -> Resultat<()> {
            for (_, rel) in elements {
                let cible = racine.join(rel);
                let mut ancien = None;
                if std::fs::symlink_metadata(&cible).is_ok() {
                    let a = abri.join(rel);
                    std::fs::create_dir_all(a.parent().unwrap_or(&abri))?;
                    std::fs::rename(&cible, &a)?;
                    ancien = Some(a);
                }
                std::fs::create_dir_all(cible.parent().unwrap_or(racine))?;
                if let Err(e) = std::fs::rename(ecart.join(rel), &cible) {
                    if let Some(a) = &ancien {
                        let _ = std::fs::rename(a, &cible);
                    }
                    return Err(e.into());
                }
                faits.push((cible, ancien));
            }
            Ok(())
        };
        if let Err(e) = poser(&mut faits) {
            // Rien à moitié : nos copies retirées (les sources restent), ce qui était là revient.
            for (cible, ancien) in faits.iter().rev() {
                let _ = if cible.is_dir() { std::fs::remove_dir_all(cible) } else { std::fs::remove_file(cible) };
                if let Some(a) = ancien {
                    let _ = std::fs::rename(a, cible);
                }
            }
            return Err(e);
        }
        Ok(total)
    })();
    let _ = std::fs::remove_dir_all(&ecart);
    r
}

/// Les éléments d'un bloc de parties pris dans des données de Ryubing (`de` : `portable\` ou une sauvegarde) : parties,
/// index et utilisateurs, ceux qui existent.
fn elements_du_bloc(de: &Path) -> Vec<(PathBuf, &'static str)> {
    DOSSIERS_DU_BLOC
        .into_iter()
        .chain(["system/Profiles.json"])
        .map(|rel| (de.join(rel), rel))
        .filter(|(p, _)| p.exists())
        .collect()
}

/// Décision de Seb (15/11, comme pour Eden) : les parties que Ryubing avait AVANT Frogtend (`portable\bis\user\save`,
/// avec leur index et leurs utilisateurs) sont COPIÉES chez la PREMIÈRE personne qui joue, une seule fois, en bloc,
/// jamais par-dessus des parties (`profil` : le nom de dossier de la personne). Une copie qui échoue est retentée
/// (`emulateurs_profils::reprise_a_faire`). Rend le nombre de fichiers copiés.
pub fn reprendre_parties_d_avant(ryubing: &Path, racine: &Path, profil: &str) -> Resultat<usize> {
    // Ryubing ouvert : rien cette fois (sans compter un essai) ; il pourrait écrire ces parties en ce moment.
    if !crate::lancement::processus_du_dossier(ryubing).is_empty() {
        return Ok(0);
    }
    let marque = ryubing.join("Profils").join(".reprise-ryubing");
    if !marque.exists() {
        std::fs::create_dir_all(ryubing.join("Profils"))?;
        std::fs::write(&marque, profil)?;
        crate::emulateurs_profils::noter_reprise(ryubing, profil, DOSSIER_PORTABLE)?;
    }
    if crate::emulateurs_profils::reprise_a_faire(ryubing, profil).as_deref() != Some(DOSSIER_PORTABLE) {
        return Ok(0);
    }
    let portable = donnees(ryubing);
    if a_des_parties(racine) || !a_des_parties(&portable) {
        crate::emulateurs_profils::reprise_faite(ryubing);
        return Ok(0);
    }
    let n = poser_en_bloc(ryubing, racine, &elements_du_bloc(&portable))?;
    crate::emulateurs_profils::reprise_faite(ryubing);
    Ok(n)
}

/// Restauration (sauvegarde de Firehouse, `de` : la zone d'attente de ce Ryubing) : parties, index et utilisateurs
/// reposés EN BLOC, seulement dans une racine qui n'a encore AUCUNE partie (sinon refus : deux index ne se fusionnent
/// jamais) ; les réglages (et leur marque) seulement si la personne n'en a pas encore ici. Rend le nombre de fichiers.
pub fn reposer_sauvegarde(ryubing: &Path, racine: &Path, de: &Path) -> Resultat<usize> {
    let mut elements = elements_du_bloc(de);
    // Des réglages que Ryubing ne lirait pas (il les remplacerait en silence) ne reviennent pas : le modèle sera posé.
    let config_lisible = std::fs::read_to_string(de.join(FICHIER_CONFIG)).is_ok_and(|t| config_valide(&t));
    if !config_de(racine).exists() && config_lisible {
        for (p, rel) in [(de.join(FICHIER_CONFIG), FICHIER_CONFIG), (de.join(MARQUE_REGLAGES), MARQUE_REGLAGES)] {
            if p.is_file() && racine.join(rel).symlink_metadata().is_err() {
                elements.push((p, rel));
            }
        }
    }
    if elements.is_empty() {
        return Ok(0);
    }
    poser_en_bloc(ryubing, racine, &elements)
}

/// La racine passée par `-r "<racine>"` dans une ligne de lancement (arguments entre guillemets ou non).
pub fn racine_de_la_ligne(ligne: &str) -> Option<PathBuf> {
    let mut mots: Vec<String> = Vec::new();
    let (mut mot, mut entre, mut commence) = (String::new(), false, false);
    for c in ligne.chars() {
        match c {
            '"' => {
                entre = !entre;
                commence = true;
            }
            c if c.is_whitespace() && !entre => {
                if commence {
                    mots.push(std::mem::take(&mut mot));
                    commence = false;
                }
            }
            c => {
                mot.push(c);
                commence = true;
            }
        }
    }
    if commence {
        mots.push(mot);
    }
    let i = mots.iter().position(|m| m == "-r" || m == "--root-data-dir")?;
    mots.get(i + 1).map(PathBuf::from)
}

/// Juste avant de lancer : la racine de la ligne existe encore. Sinon Ryubing jouerait EN SILENCE sur les données
/// communes (`portable\`) : refusé.
pub fn verifier_la_racine(ligne: &str) -> Resultat<()> {
    match racine_de_la_ligne(ligne) {
        Some(r) if !r.is_absolute() || !r.is_dir() => Err(Erreur::Refus(format!(
            "Le dossier de cette personne dans Ryubing a disparu ({}) : la partie n'est pas lancée (elle se jouerait sur les données d'un autre). Relance le jeu.",
            r.display()
        ))),
        _ => Ok(()),
    }
}

/// Avant chaque partie (`emulateurs_profils::preparer`) : la racine de la personne prête (clés, firmware partagé, son
/// propre `games\`), ses parties d'avant reprises, son Config.json réglé (`joueur1` : la manette XInput branchée, le
/// clavier choisi pour ce jeu, ou rien). Rend `-r <racine>`. Ryubing pas portable :
/// rien (ses données sont dans %APPDATA%, où Frogtend n'écrit jamais), comme avant. Un échec du réglage n'empêche
/// jamais de jouer ; un échec de la racine, si.
pub fn preparer_partie(ryubing: &Path, racine: &Path, joueur1: Joueur1) -> Resultat<Vec<String>> {
    if !portable(ryubing) {
        return Ok(vec![]);
    }
    preparer_racine(ryubing, racine)?;
    let profil = racine.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    if let Err(e) = reprendre_parties_d_avant(ryubing, racine, &profil) {
        eprintln!("Ryubing : parties d'avant Frogtend pas encore reprises ({e:?})");
    }
    if let Err(e) = regler_config(ryubing, racine, joueur1) {
        eprintln!("Ryubing : réglages de la première partie non posés ({e:?})");
    }
    if !racine.is_dir() {
        return Err(Erreur::Disque("Le dossier de cette personne dans Ryubing n'a pas pu être créé.".into()));
    }
    Ok(vec!["-r".into(), racine.to_string_lossy().to_string()])
}

// ---------------------------------------------------------------------------------------------------------------
// Lot 5 — le menu en jeu (✅ Seb : « tout ce que l'émulateur propose, dans notre menu »). Relu dans Ryubing 1.3.3 :
// - ses raccourcis (`hotkeys` du Config.json, ConfigurationState.cs L261-276) : F1 vsync, F8 capture, F4 interface,
//   F5 pause, F2 son, le reste « Unbound » ; lus SEULEMENT au premier plan (AppHost.cs L1247) — le menu de Frogtend
//   rend la main au jeu avant d'envoyer la touche ; une chaîne de `if/else` (`GetHotkeyState` L1397-1445) : une touche
//   prise par deux actions ne fait que la première ;
// - jamais surchargés par un réglage propre à un jeu (Migration.cs L149 : « Get from global config only ») ;
// - `turbo_mode` : une BASCULE tant que `turbo_mode_while_held` vaut false (L1306-1311) ; Frogtend le lie à F13
//   (Key.cs : F13-F24 existent, AvaloniaKeyboardMappingHelper.cs) ;
// - le son coupé (F2) n'est PAS enregistré (L1330-1341 : `Device.SetVolume` seulement) : rien à remettre ensuite ;
// - F4 (`ShowUI`) montre la barre de menus ET la barre d'état (où sont les images par seconde) : ce n'est pas un
//   compteur, la barre de menus prendrait la souris et l'image rétrécirait — pas proposé ;
// - pas de sauvegardes rapides, pas de Recommencer (le menu relance la partie) ;
// - les triches : REPORTÉES, pas encore de source de triches Switch (Ryubing lirait `<racine>\mods\contents\<jeu>\
//   cheats\<BuildId>.txt` et `enabled.txt`, ModLoader.cs L740-768 ; il écrit le BuildId dans son journal, L730, et
//   ignore un fichier qui ne correspond pas) — docs/IDEES.md ;
// - les effets d'image : `anti_aliasing` et `scaling_filter` (Graphics, lus au démarrage) : au prochain lancement.
// ---------------------------------------------------------------------------------------------------------------

/// La touche que Frogtend lie à l'avance rapide (`turbo_mode`).
pub const TOUCHE_TURBO: &str = "F13";
/// « L'avance rapide a été décidée pour cette personne » (posée, ou laissée à son choix).
const MARQUE_TURBO: &str = ".frogtend-ryubing-turbo";

/// Les actions du menu par les raccourcis de Ryubing : (action, clé de `hotkeys`, touche attendue).
pub const RACCOURCIS_MENU: &[(crate::pilotage::Action, &str, &str)] = &[
    (crate::pilotage::Action::Capture, "screenshot", "F8"),
    (crate::pilotage::Action::Muet, "toggle_mute", "F2"),
    (crate::pilotage::Action::AvanceRapide, "turbo_mode", TOUCHE_TURBO),
];

/// La touche est-elle à cette action SEULEMENT (aucune autre action de `hotkeys` ne l'a) ?
fn touche_seule(hotkeys: &Value, cle: &str, touche: &str) -> bool {
    hotkeys[cle] == touche && hotkeys.as_object().is_some_and(|o| o.iter().filter(|(_, v)| v.as_str() == Some(touche)).count() == 1)
}

/// Lie l'avance rapide à F13 si elle est encore « Unbound », en bascule, et que F13 n'est à personne. Rend vrai si
/// `v` a changé.
fn poser_turbo(v: &mut Value) -> bool {
    let h = &v["hotkeys"];
    let libre = h["turbo_mode"] == "Unbound"
        && h["turbo_mode_while_held"] == false
        && h.as_object().is_some_and(|o| o.values().all(|x| x.as_str() != Some(TOUCHE_TURBO)));
    if libre {
        v["hotkeys"]["turbo_mode"] = Value::from(TOUCHE_TURBO);
    }
    libre
}

/// La vitesse de l'avance rapide (`tick_scalar`, en %) : un Ryubing NEUF l'écrit à 0 (relevé du 16/11 ; seule sa
/// migration 68 met 200 aux anciens Config.json, ConfigurationState.Migration.cs L467) ; à 0, `Switch::ToggleTurbo`
/// ne peut rien accélérer. Mise à 200 (la valeur de Ryubing) quand l'avance rapide est sur F13 (celle du menu) ; une
/// vitesse choisie par la personne (> 0) n'est jamais touchée. Rend vrai si `v` a changé.
fn poser_vitesse_turbo(v: &mut Value) -> bool {
    let a_regler = v["hotkeys"]["turbo_mode"] == TOUCHE_TURBO && v["tick_scalar"].as_f64().is_none_or(|x| x <= 0.0);
    if a_regler {
        v["tick_scalar"] = Value::from(200);
    }
    a_regler
}

/// Les actions du menu possibles pour cette personne : celles dont la touche est la bonne, à elle seule (un choix de
/// la personne n'est jamais deviné ; l'avance rapide seulement en bascule). Rien si Ryubing n'est pas portable ou si
/// un Config.json à côté de Ryujinx.exe passe devant (la touche pourrait y faire autre chose).
pub fn actions_d_origine(ryubing: &Path, racine: &Path) -> Vec<crate::pilotage::Action> {
    if !portable(ryubing) || config_a_cote_du_programme(ryubing) {
        return vec![];
    }
    let Some(v) = std::fs::read_to_string(config_de(racine)).ok().filter(|t| config_valide(t)).and_then(|t| serde_json::from_str::<Value>(&t).ok()) else {
        return vec![];
    };
    let h = &v["hotkeys"];
    RACCOURCIS_MENU
        .iter()
        .filter(|(a, cle, touche)| {
            touche_seule(h, cle, touche)
                && (*a != crate::pilotage::Action::AvanceRapide
                    || (h["turbo_mode_while_held"] == false && v["tick_scalar"].as_f64().is_some_and(|x| x > 0.0)))
        })
        .map(|(a, _, _)| *a)
        .collect()
}

/// Le préfixe des effets d'image de Ryubing dans les réglages du profil (`image`) : jamais confondus avec ceux de
/// RetroArch ou de BigPEmu.
pub const PREFIXE_EFFET: &str = "ryubing:";

/// Les effets proposés : (identifiant, `anti_aliasing`, `scaling_filter`). Valeurs relues dans Ryubing 1.3.3
/// (AntiAliasing : None, Fxaa, SmaaLow…SmaaUltra ; ScalingFilter : Bilinear, Nearest, Fsr, Area). Peu nombreux,
/// compréhensibles : les noms français sont dans src/lib/ludotheque/effetsImage.ts.
pub const EFFETS: &[(&str, &str, &str)] = &[
    ("contours", "SmaaHigh", "Bilinear"),
    ("adouci", "Fxaa", "Bilinear"),
    ("net", "None", "Fsr"),
    ("pixels", "None", "Nearest"),
];

/// La marque de l'effet posé par Frogtend : `{"pose": [aa, filtre]}`. Sans elle (ou si le Config.json dit autre
/// chose), un réglage d'image autre que celui d'origine est le choix de la personne.
const MARQUE_EFFET: &str = ".frogtend-ryubing-effet";

/// Le réglage d'image d'origine de Ryubing (modèle relevé : `anti_aliasing` None, `scaling_filter` Bilinear) : « aucun
/// effet ».
const IMAGE_D_ORIGINE: (&str, &str) = ("None", "Bilinear");

/// Les effets d'image sont-ils possibles pour ce jeu ? `Err(phrase)` sinon, dite telle quelle à la personne.
/// `titre` : le numéro du jeu (lu dans un .nsp, `contenus::id_switch`) ; inconnu (.xci), le moindre réglage propre à
/// un jeu suffit à refuser (Frogtend ne sait pas s'il est le sien).
pub fn effets_possibles(ryubing: &Path, racine: &Path, titre: Option<u64>) -> Result<(), String> {
    if !portable(ryubing) || config_a_cote_du_programme(ryubing) {
        return Err("Ryubing n'a pas été installé par Frogtend : ses réglages d'image restent les siens.".into());
    }
    let par_jeu = configs_par_jeu(racine);
    let le_sien = match titre {
        // Le dossier du jeu est son numéro de base (ApplicationData.cs L104-106 : `Id & ~0x1FFF`, « x16 »).
        Some(t) => par_jeu.iter().any(|j| j.eq_ignore_ascii_case(&format!("{:016x}", t & !0x1FFF))),
        None => !par_jeu.is_empty(),
    };
    if le_sien {
        return Err(match titre {
            Some(_) => "Ce jeu a ses propres réglages dans Ryubing : l'effet d'image ne s'y applique pas.".into(),
            None => "Des jeux ont leurs propres réglages dans Ryubing, et Frogtend ne sait pas si celui-ci en fait partie : l'effet d'image n'est pas proposé.".into(),
        });
    }
    Ok(())
}

/// L'effet en place après `poser_effet`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffetPose {
    /// L'effet (identifiant du profil) que donne le réglage en place ; `None` : l'image d'origine, ou un réglage de la
    /// personne qui n'est pas dans la liste.
    pub effet: Option<String>,
    /// Le réglage d'image fait par la personne dans Ryubing : il passe devant (jamais remplacé).
    pub de_la_personne: bool,
}

fn id_d_effet(p: &(String, String)) -> Option<String> {
    EFFETS.iter().find(|(_, aa, filtre)| (aa.to_string(), filtre.to_string()) == *p).map(|(e, _, _)| format!("{PREFIXE_EFFET}{e}"))
}

/// Les effets présents (tous : ce sont des réglages de Ryubing), en identifiants du profil.
pub fn effets_presents() -> Vec<String> {
    EFFETS.iter().map(|(e, _, _)| format!("{PREFIXE_EFFET}{e}")).collect()
}

/// Pose l'effet choisi avant le lancement, Ryubing FERMÉ (`aucun`, `ryubing:<effet>` ; `None` ou un effet d'un autre
/// émulateur : pas de choix = l'image d'origine). Refusé (`Err`) si les effets ne sont pas possibles pour ce jeu
/// (`effets_possibles`), si Ryubing est ouvert ou si la personne n'a pas encore de Config.json. Un réglage d'image
/// qu'elle a fait elle-même dans Ryubing GAGNE (comme celui de RetroArch) : jamais remplacé. À l'abri avant, relu
/// après.
pub fn poser_effet(ryubing: &Path, racine: &Path, titre: Option<u64>, choix: Option<&str>) -> Resultat<EffetPose> {
    effets_possibles(ryubing, racine, titre).map_err(Erreur::Refus)?;
    if !crate::lancement::processus_du_dossier(ryubing).is_empty() {
        return Err(Erreur::Refus("Ryubing est encore ouvert : l'effet d'image n'a pas pu être posé pour cette partie.".into()));
    }
    let f = config_de(racine);
    let avant = std::fs::read_to_string(&f)
        .ok()
        .filter(|t| config_valide(t))
        .ok_or_else(|| Erreur::Refus("Les réglages de Ryubing de cette personne ne se lisent pas : l'effet d'image n'est pas posé.".into()))?;
    let mut v: Value = serde_json::from_str(&avant).map_err(|_| Erreur::Disque("Config.json de Ryubing illisible.".into()))?;
    let paire = |v: &Value| (v["anti_aliasing"].as_str().unwrap_or("").to_string(), v["scaling_filter"].as_str().unwrap_or("").to_string());
    let actuel = paire(&v);
    let origine = (IMAGE_D_ORIGINE.0.to_string(), IMAGE_D_ORIGINE.1.to_string());
    let marque_f = racine.join(MARQUE_EFFET);
    let marque: Option<Value> = std::fs::read(&marque_f).ok().and_then(|o| serde_json::from_slice(&o).ok());
    let pose = marque.as_ref().map(|m| (m["pose"][0].as_str().unwrap_or("").to_string(), m["pose"][1].as_str().unwrap_or("").to_string()));
    let le_notre = pose.as_ref() == Some(&actuel);
    // Changé dans Ryubing depuis (ou réglé avant Frogtend) : c'est son choix, il passe devant.
    if !le_notre && actuel != origine {
        let _ = std::fs::remove_file(&marque_f);
        return Ok(EffetPose { effet: id_d_effet(&actuel), de_la_personne: true });
    }
    let voulu = match choix.and_then(|c| c.strip_prefix(PREFIXE_EFFET)).and_then(|c| EFFETS.iter().find(|(e, _, _)| *e == c)) {
        Some((_, aa, filtre)) => (aa.to_string(), filtre.to_string()),
        None => origine.clone(),
    };
    // La marque AVANT les réglages : un effet posé par Frogtend n'est jamais pris ensuite pour celui de la personne.
    if voulu != origine {
        std::fs::write(&marque_f, serde_json::json!({"pose": [voulu.0, voulu.1]}).to_string())?;
    }
    if voulu != actuel {
        v["anti_aliasing"] = Value::from(voulu.0.clone());
        v["scaling_filter"] = Value::from(voulu.1.clone());
        ecrire_verifie(ryubing, &f, &avant, &v)?;
    }
    if voulu == origine {
        let _ = std::fs::remove_file(&marque_f);
    }
    Ok(EffetPose { effet: id_d_effet(&voulu), de_la_personne: false })
}

/// Ce que montre le schéma « 🎮 Les touches » : le joueur 1 porte-t-il la manette posée par Frogtend, le clavier, ou
/// autre chose (une manette réglée par la personne) ?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Joueur1Lu {
    DeFrogtend,
    AuClavier,
    Autre,
}

/// Le joueur 1 dans le Config.json de la personne (absent : il sera au clavier, puis réglé).
pub fn joueur1_lu(racine: &Path) -> Joueur1Lu {
    let Some(v) = std::fs::read_to_string(config_de(racine)).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()) else { return Joueur1Lu::AuClavier };
    let marque = std::fs::read_to_string(racine.join(MARQUE_MANETTE)).ok();
    match v["input_config"].as_array().and_then(|l| l.iter().find(|e| e["player_index"] == "Player1")) {
        Some(j) if j["backend"] == "WindowKeyboard" => Joueur1Lu::AuClavier,
        // La manette de Frogtend, sa disposition intacte (le nom affiché peut changer : il ne compte pas).
        Some(j)
            if j["backend"] == "GamepadSDL2"
                && marque.as_deref().is_some_and(|m| j["id"] == m.trim())
                && ["left_joycon", "right_joycon", "left_joycon_stick", "right_joycon_stick"].iter().all(|k| j[*k] == bloc_manette("")[*k]) =>
        {
            Joueur1Lu::DeFrogtend
        }
        _ => Joueur1Lu::Autre,
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Lot 6 — mises à jour et DLC posés AUTOMATIQUEMENT (✅ Seb 16/11 : sans accord). Lancé par `-f`, Ryubing ne charge
// pas sa bibliothèque (`autoload_dirs` ne sert à rien) : il lit seulement, dans `<racine>\games\<base en hexa
// minuscule, 16 caractères>\` (Ryubing 1.3.3) :
// - `updates.json` `{"selected": …, "paths": […]}` : la mise à jour `selected` si le fichier existe et porte bien ce
//   jeu (NcaExtensions.cs L146-163) ; l'interface de Ryubing la reconnaît par égalité EXACTE avec un des `paths`
//   (TitleUpdatesHelper.cs) ;
// - `dlc.json` `[{"path": <conteneur>, "dlc_nca_list": [{"path": "/<nca>", "title_id": <décimal>, "is_enabled"}]}]`
//   (PartitionFileSystemExtensions.cs L118-138) : seul le conteneur est vérifié ; un mauvais chemin de NCA ferait
//   PLANTER le jeu, un mauvais identifiant cacherait le DLC — les deux sont LUS dans les en-têtes (switch_nca.rs).
// Jamais par-dessus un choix de la personne : `.frogtend-ryubing-contenus.json` (dans sa racine) garde ce que Frogtend
// a posé ; seul cela peut être remplacé ou retiré. Ryubing fermé, chaque fichier à l'abri avant, relu après.
// ---------------------------------------------------------------------------------------------------------------

/// La note de ce que Frogtend a posé dans `games\`, par jeu (base en hexa minuscule).
const NOTE_CONTENUS: &str = ".frogtend-ryubing-contenus.json";

#[derive(Debug, Default, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct PoseParFrogtend {
    /// La mise à jour que Frogtend a posée (choisie, ou seulement ajoutée à la liste quand la personne en a une autre).
    #[serde(default)]
    maj: Option<String>,
    /// Les DLC posés par Frogtend et encore dans la liste de Ryubing.
    #[serde(default)]
    dlc: Vec<String>,
    /// Les DLC posés par Frogtend puis RETIRÉS par la personne dans Ryubing : plus jamais reposés.
    #[serde(default)]
    dlc_refuses: Vec<String>,
}

/// Le `prod.keys` que Ryubing lira pour cette personne : celui de sa racine (copié par `copier_les_cles`), sinon celui
/// de `portable\`.
pub fn fichier_cles(ryubing: &Path, racine: &Path) -> Option<PathBuf> {
    [racine.join("system").join("prod.keys"), donnees(ryubing).join("system").join("prod.keys")].into_iter().find(|f| f.is_file())
}

/// La clé des en-têtes de NCA de cette personne (jamais affichée).
pub fn cle_entete(ryubing: &Path, racine: &Path) -> Option<crate::switch_nca::CleEntete> {
    crate::switch_nca::cle_entete(&fichier_cles(ryubing, racine)?)
}

/// Le dossier de Ryubing pour un jeu (`<racine>\games\<base>`, hexa minuscule comme `ToString("x16")`).
pub fn dossier_du_jeu(racine: &Path, base: u64) -> PathBuf {
    racine.join("games").join(format!("{base:016x}"))
}

/// À qui est le choix de la mise à jour.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum ChoixMaj {
    /// Celle de Frogtend (ou aucune trouvée).
    #[default]
    DeFrogtend,
    /// La personne en a choisi une autre (ou « aucune ») dans Ryubing : gardé.
    DeLaPersonne,
    /// La personne a RETIRÉ de Ryubing celle que Frogtend avait posée : jamais remise.
    RetireeParLaPersonne,
}

/// Ce qui est (ou sera) posé pour un jeu.
#[derive(Debug, Default, Clone, PartialEq, serde::Serialize)]
pub struct ContenusPoses {
    /// La mise à jour posée et choisie par Frogtend (nom du fichier).
    pub maj: Option<String>,
    pub choix_maj: ChoixMaj,
    /// Les DLC que Ryubing chargera (noms des fichiers).
    pub dlc: Vec<String>,
    /// Les DLC dans la liste de Ryubing mais coupés par la personne (`is_enabled` à false).
    pub dlc_coupes: Vec<String>,
    /// Les DLC posés par Frogtend puis retirés par la personne dans Ryubing : plus reposés.
    pub dlc_retires: Vec<String>,
    /// Un fichier a été écrit à ce lancement.
    pub ecrit: bool,
    /// Ce qui n'a pas pu être fait, en mots simples (dit au journal et sur la fiche ; le jeu se lance quand même).
    pub erreurs: Vec<String>,
}

impl ContenusPoses {
    /// Une phrase pour le journal (jamais de clé, jamais de numéro de version brut).
    pub fn resume(&self) -> String {
        let mut l = Vec::new();
        match (&self.maj, self.choix_maj) {
            (_, ChoixMaj::DeLaPersonne) => l.push("mise à jour choisie dans Ryubing gardée".to_string()),
            (_, ChoixMaj::RetireeParLaPersonne) => l.push("mise à jour retirée dans Ryubing : pas remise".to_string()),
            (Some(m), _) => l.push(format!("mise à jour « {m} »")),
            (None, _) => l.push("aucune mise à jour".to_string()),
        }
        l.push(format!("{} DLC", self.dlc.len()));
        if !self.dlc_coupes.is_empty() {
            l.push(format!("{} DLC coupé(s) dans Ryubing", self.dlc_coupes.len()));
        }
        if !self.dlc_retires.is_empty() {
            l.push(format!("{} DLC retiré(s) dans Ryubing", self.dlc_retires.len()));
        }
        l.extend(self.erreurs.iter().cloned());
        l.join(" ; ")
    }
}

fn nom_de(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
}

/// Écrit `o` à la place de `f` (provisoire puis renommage : jamais un fichier à moitié écrit).
fn ecrire_octets(f: &Path, o: &[u8]) -> Resultat<()> {
    if let Some(p) = f.parent() {
        std::fs::create_dir_all(p)?;
    }
    let mut provisoire = f.as_os_str().to_owned();
    provisoire.push(".frogtend");
    let provisoire = PathBuf::from(provisoire);
    std::fs::write(&provisoire, o)?;
    std::fs::rename(&provisoire, f)?;
    Ok(())
}

/// Un fichier JSON de Ryubing (`quoi` : ce qu'il est, en mots simples) : son contenu et ses octets d'origine ; `None`
/// s'il n'existe pas ; illisible : refusé (jamais touché).
fn lire_json(f: &Path, quoi: &str) -> Resultat<Option<(Value, Vec<u8>)>> {
    match std::fs::read(f) {
        Ok(o) => {
            let v = std::str::from_utf8(&o).ok().and_then(|t| serde_json::from_str(t.trim_start_matches('\u{feff}')).ok());
            v.map(|v| Some((v, o))).ok_or_else(|| Erreur::Refus(format!("{quoi} de Ryubing ne se lit pas : Frogtend n'y touche pas")))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Écrit `v` à la place de `f` (à l'abri avant, relu après ; s'il ne se relit pas, les OCTETS d'origine reviennent, ou
/// le fichier est retiré s'il n'existait pas).
fn ecrire_json_verifie(ryubing: &Path, f: &Path, avant: Option<&[u8]>, v: &Value, quoi: &str) -> Resultat<()> {
    let texte = serde_json::to_string_pretty(v).map_err(|e| Erreur::Disque(format!("{quoi} de Ryubing impossible à écrire ({e})")))?;
    crate::emulateurs_profils::mettre_a_l_abri(ryubing, f)?;
    ecrire_octets(f, texte.as_bytes())?;
    if lire_json(f, quoi).ok().flatten().map(|(x, _)| x).as_ref() != Some(v) {
        match avant {
            Some(o) => ecrire_octets(f, o)?,
            None => std::fs::remove_file(f)?,
        }
        return Err(Erreur::Disque(format!("{quoi} de Ryubing ne se relisait pas après l'écriture : remise comme avant")));
    }
    Ok(())
}

/// Le `updates.json` voulu. `notre` : la mise à jour trouvée par Frogtend ; `posee` : celle qu'il avait posée la fois
/// d'avant ; `existe` : un fichier est-il là. Rend (le nouveau contenu s'il change, à qui est le choix).
/// - la nôtre, déjà posée, n'est plus dans la liste de Ryubing : la personne l'a RETIRÉE, elle n'est jamais remise (une
///   mise à jour nouvelle, différente, peut l'être) ;
/// - un choix de la personne : un `selected` dont le fichier existe et que Frogtend n'a pas posé, ou `""` (Ryubing
///   l'écrit quand on décoche tout) alors qu'une mise à jour existante est listée — gardé, la nôtre seulement ajoutée
///   à la liste.
pub fn updates_voulu(avant: Option<&Value>, notre: Option<&str>, posee: Option<&str>, existe: &dyn Fn(&str) -> bool) -> (Option<Value>, ChoixMaj) {
    let meme = crate::contenus::meme_dossier;
    let selected = avant.and_then(|v| v["selected"].as_str()).unwrap_or("").to_string();
    let mut paths: Vec<String> = avant.and_then(|v| v["paths"].as_array()).map(|l| l.iter().filter_map(|p| p.as_str().map(String::from)).collect()).unwrap_or_default();
    if avant.is_some() && posee.is_some_and(|p| notre.is_some_and(|n| meme(p, n)) && !paths.iter().any(|x| meme(x, p))) {
        return (None, ChoixMaj::RetireeParLaPersonne);
    }
    let de_la_personne = match avant {
        None => false,
        Some(_) if selected.is_empty() => avant.is_some_and(|v| v["selected"].is_string()) && paths.iter().any(|p| existe(p)),
        Some(_) => existe(&selected) && posee.is_none_or(|p| !meme(&selected, p)) && notre.is_none_or(|n| !meme(&selected, n)),
    };
    let choix = if de_la_personne { ChoixMaj::DeLaPersonne } else { ChoixMaj::DeFrogtend };
    let Some(notre) = notre else { return (None, choix) };
    // L'ancienne mise à jour de Frogtend, partie du disque : retirée de la liste (jamais celle de la personne).
    paths.retain(|p| !(posee.is_some_and(|q| meme(p, q)) && !meme(p, notre) && !existe(p)));
    if !paths.iter().any(|p| meme(p, notre)) {
        paths.push(notre.to_string());
    }
    // Ryubing compare `selected` à `paths` à l'identique : la forme déjà dans la liste.
    let notre_ecrit = paths.iter().find(|p| meme(p, notre)).cloned().unwrap_or_else(|| notre.to_string());
    let mut v = avant.cloned().filter(Value::is_object).unwrap_or_else(|| serde_json::json!({}));
    v["selected"] = Value::from(if de_la_personne { selected } else { notre_ecrit });
    v["paths"] = Value::from(paths);
    ((Some(&v) != avant).then_some(v), choix)
}

/// Le `dlc.json` voulu. `nos` : les DLC de ce jeu lus CE JOUR (conteneur, NCA PublicData) ; `poses` : les conteneurs
/// que Frogtend avait posés ; `refuses` : ceux que la personne avait retirés. Rend (le nouveau contenu, les conteneurs de
/// Frogtend dans la liste, les refusés) ; `Err` : le fichier n'est pas une liste.
/// - une entrée de Frogtend dont le fichier a disparu, ou qui n'est plus lue ce jour comme un DLC de ce jeu (illisible,
///   autre jeu, autre sorte), est retirée : Ryubing ouvrirait conteneur et NCA sans pardon (ContentManager.cs L205-234) ;
/// - ses NCA sont relus à chaque fois, le choix `is_enabled` de la personne gardé par identifiant ;
/// - un DLC de Frogtend ABSENT de la liste existante (le fichier, lui, est là) a été retiré par la personne : noté
///   refusé, plus jamais reposé ;
/// - les entrées de la personne ne sont jamais touchées.
pub fn dlc_voulu(
    avant: Option<&Value>,
    nos: &[(String, Vec<(String, u64)>)],
    poses: &[String],
    refuses: &[String],
    existe: &dyn Fn(&str) -> bool,
) -> Result<(Value, Vec<String>, Vec<String>), ()> {
    let meme = crate::contenus::meme_dossier;
    let mut l: Vec<Value> = match avant {
        None => vec![],
        Some(Value::Array(a)) => a.clone(),
        Some(_) => return Err(()),
    };
    let chemin = |c: &Value| c["path"].as_str().unwrap_or("").to_string();
    let de_frogtend = |p: &str| poses.iter().any(|q| meme(p, q));
    let lu_ce_jour = |p: &str| nos.iter().any(|(c, _)| meme(c, p));
    l.retain(|c| {
        let p = chemin(c);
        !(de_frogtend(&p) && (!existe(&p) || !lu_ce_jour(&p)))
    });
    let liste = |ncas: &[(String, u64)], ancien: Option<&Value>| -> Value {
        Value::from(
            ncas.iter()
                .map(|(p, t)| {
                    let actif = ancien.and_then(|a| a["dlc_nca_list"].as_array()).and_then(|a| a.iter().find(|x| x["title_id"].as_u64() == Some(*t))).and_then(|x| x["is_enabled"].as_bool()).unwrap_or(true);
                    serde_json::json!({ "path": p, "title_id": t, "is_enabled": actif })
                })
                .collect::<Vec<_>>(),
        )
    };
    let mut note: Vec<String> = Vec::new();
    let mut refuses: Vec<String> = refuses.to_vec();
    for (c, ncas) in nos {
        match l.iter().position(|x| meme(&chemin(x), c)) {
            Some(i) if de_frogtend(&chemin(&l[i])) => {
                let ancien = l[i].clone();
                l[i]["dlc_nca_list"] = liste(ncas, Some(&ancien));
                note.push(chemin(&l[i]));
            }
            Some(_) => {} // posé par la personne : son choix
            None if refuses.iter().any(|r| meme(r, c)) => {}
            // Posé par Frogtend, plus dans une liste qui existe : retiré par la personne.
            None if avant.is_some() && de_frogtend(c) => refuses.push(c.clone()),
            None => {
                l.push(serde_json::json!({ "path": c, "dlc_nca_list": liste(ncas, None) }));
                note.push(c.clone());
            }
        }
    }
    Ok((Value::from(l), note, refuses))
}

/// Pose (`ecrire`) ou montre seulement (`!ecrire`) les mises à jour et DLC d'un jeu (`c`, lus par switch_nca) dans la
/// racine de la personne. Écrire : Ryubing fermé (il réécrirait ces fichiers). Un fichier illisible n'est jamais touché ;
/// chaque problème est rendu dans `erreurs`, le reste est fait.
pub fn poser_contenus(ryubing: &Path, racine: &Path, c: &crate::switch_nca::ContenusDuJeu, ecrire: bool) -> Resultat<ContenusPoses> {
    if ecrire && !crate::lancement::processus_du_dossier(ryubing).is_empty() {
        return Err(Erreur::Refus("Ryubing est ouvert : ses mises à jour et DLC ne sont pas touchés.".into()));
    }
    let meme = crate::contenus::meme_dossier;
    let dossier = dossier_du_jeu(racine, c.base);
    let f_note = racine.join(NOTE_CONTENUS);
    let mut note: std::collections::BTreeMap<String, PoseParFrogtend> = std::fs::read(&f_note).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default();
    let cle = format!("{:016x}", c.base);
    let posee = note.get(&cle).cloned().unwrap_or_default();
    let mut nouvelle = posee.clone();
    let existe = |p: &str| Path::new(p).is_file();
    let mut r = ContenusPoses::default();

    // La mise à jour.
    const MAJS: &str = "La liste des mises à jour";
    let notre = c.maj.as_ref().map(|m| m.chemin.to_string_lossy().to_string());
    let f_maj = dossier.join("updates.json");
    match lire_json(&f_maj, MAJS) {
        Err(e) => r.erreurs.push(format!("mise à jour non utilisée ({e})")),
        Ok(avant) => {
            let (av, octets) = avant.map_or((None, None), |(v, o)| (Some(v), Some(o)));
            let (voulu, choix) = updates_voulu(av.as_ref(), notre.as_deref(), posee.maj.as_deref(), &existe);
            r.choix_maj = choix;
            let ok = match voulu.filter(|_| ecrire) {
                Some(v) => {
                    std::fs::create_dir_all(&dossier)?;
                    match ecrire_json_verifie(ryubing, &f_maj, octets.as_deref(), &v, MAJS) {
                        Ok(()) => {
                            r.ecrit = true;
                            true
                        }
                        Err(e) => {
                            r.erreurs.push(format!("mise à jour non utilisée ({e})"));
                            false
                        }
                    }
                }
                None => true,
            };
            if ok && choix == ChoixMaj::DeFrogtend {
                r.maj = c.maj.as_ref().map(|m| nom_de(&m.chemin));
            }
            // Ajoutée à la liste (choisie ou non) : retenue comme la nôtre, pour voir si la personne la retire.
            if ok && choix != ChoixMaj::RetireeParLaPersonne && notre.is_some() {
                nouvelle.maj = notre.clone();
            }
        }
    }

    // Les DLC.
    const DLCS: &str = "La liste des DLC";
    let nos: Vec<(String, Vec<(String, u64)>)> = c
        .dlc
        .iter()
        .filter_map(|d| match &d.sorte {
            crate::switch_nca::Sorte::Dlc(n) => Some((d.chemin.to_string_lossy().to_string(), n.clone())),
            _ => None,
        })
        .collect();
    let f_dlc = dossier.join("dlc.json");
    match lire_json(&f_dlc, DLCS) {
        Err(e) => r.erreurs.push(format!("DLC non utilisés ({e})")),
        Ok(avant) => {
            let (av, octets) = avant.map_or((None, None), |(v, o)| (Some(v), Some(o)));
            match dlc_voulu(av.as_ref(), &nos, &posee.dlc, &posee.dlc_refuses, &existe) {
                Err(()) => r.erreurs.push("DLC non utilisés (la liste des DLC de Ryubing n'a pas la forme attendue : Frogtend n'y touche pas)".into()),
                Ok((v, n, refuses)) => {
                    let change = av.as_ref() != Some(&v) && !(av.is_none() && v.as_array().is_some_and(|a| a.is_empty()));
                    let ok = if ecrire && change {
                        std::fs::create_dir_all(&dossier)?;
                        match ecrire_json_verifie(ryubing, &f_dlc, octets.as_deref(), &v, DLCS) {
                            Ok(()) => {
                                r.ecrit = true;
                                true
                            }
                            Err(e) => {
                                r.erreurs.push(format!("DLC non utilisés ({e})"));
                                false
                            }
                        }
                    } else {
                        true
                    };
                    if ok {
                        nouvelle.dlc = n;
                        nouvelle.dlc_refuses = refuses.clone();
                    }
                    // L'état de chaque DLC trouvé, tel que Ryubing le verra.
                    let liste = if ok { v } else { av.unwrap_or(Value::Null) };
                    for (p, _) in &nos {
                        let entree = liste.as_array().into_iter().flatten().find(|x| meme(x["path"].as_str().unwrap_or(""), p));
                        let nom = nom_de(Path::new(p));
                        match entree {
                            Some(x) if x["dlc_nca_list"].as_array().is_some_and(|a| !a.is_empty() && a.iter().all(|n| n["is_enabled"] == false)) => r.dlc_coupes.push(nom),
                            Some(_) => r.dlc.push(nom),
                            None if refuses.iter().any(|q| meme(q, p)) => r.dlc_retires.push(nom),
                            None => {}
                        }
                    }
                }
            }
        }
    }

    if ecrire && nouvelle != posee {
        if nouvelle == PoseParFrogtend::default() {
            note.remove(&cle);
        } else {
            note.insert(cle, nouvelle);
        }
        ecrire_octets(&f_note, &serde_json::to_vec_pretty(&note).unwrap_or_default())?;
    }
    Ok(r)
}

/// Avant chaque partie Ryubing (`jeu` : le fichier lancé par `-f`) : lit le jeu et son dossier avec la clé de la
/// personne, puis pose ses mises à jour et DLC. `None` : Ryubing pas portable, ou pas un jeu lisible (rien à poser).
/// Un échec n'empêche JAMAIS de jouer : l'appelant le note au journal.
pub fn preparer_contenus(ryubing: &Path, racine: &Path, jeu: &Path) -> Resultat<Option<ContenusPoses>> {
    if !portable(ryubing) {
        return Ok(None);
    }
    let cle = cle_entete(ryubing, racine).ok_or_else(|| Erreur::Refus("les clés de la console manquent : mises à jour et DLC non utilisés".into()))?;
    let Some(c) = crate::switch_nca::contenus_pour(jeu, &cle)? else { return Ok(None) };
    let mut r = poser_contenus(ryubing, racine, &c, true)?;
    r.erreurs.extend(remarques_du_dossier(&c));
    Ok(Some(r))
}

/// Ce que le dossier du jeu a de particulier, en mots simples : fichiers illisibles, extensions que Ryubing ne lirait
/// pas, dossier trop grand pour être lu en entier.
pub fn remarques_du_dossier(c: &crate::switch_nca::ContenusDuJeu) -> Vec<String> {
    let mut l = Vec::new();
    if !c.illisibles.is_empty() {
        l.push(format!("{} fichier(s) du dossier illisible(s) : {}", c.illisibles.len(), c.illisibles.join(", ")));
    }
    for n in &c.mauvaise_extension {
        l.push(format!("« {n} » n'est pas utilisé : Ryubing ne le lit que si son extension est en minuscules et correspond à son contenu (.nsp ou .xci)"));
    }
    if c.tronque {
        l.push(format!("le dossier du jeu contient trop de fichiers : seuls les {} premiers ont été regardés", crate::switch_nca::MAX_VOISINS));
    }
    l
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

    // --- Lots 2 et 3 ---

    /// Un Ryubing portable avec ses clés et un firmware posé, et la racine de Seb.
    fn ryubing_installe(d: &Path) -> (PathBuf, PathBuf) {
        let ryu = d.join("Ryubing");
        let sys = ryu.join("portable").join("system");
        std::fs::create_dir_all(&sys).unwrap();
        std::fs::write(sys.join("prod.keys"), b"cles prod").unwrap();
        std::fs::write(sys.join("title.keys"), b"cles titres").unwrap();
        std::fs::write(sys.join("Profiles.json"), b"{}").unwrap();
        let nca = dossier_firmware(&ryu).join(format!("{A}.nca"));
        std::fs::create_dir_all(&nca).unwrap();
        std::fs::write(nca.join("00"), b"firmware").unwrap();
        std::fs::write(ryu.join("Ryujinx.exe"), b"").unwrap();
        let racine = racine(&ryu, "Seb");
        (ryu, racine)
    }

    fn lire_config(racine: &Path) -> Value {
        serde_json::from_str(&std::fs::read_to_string(config_de(racine)).unwrap()).unwrap()
    }

    const MANETTE_XBOX_ONE: &str = "0-00000003-045e-0000-ff02-000000007200";

    #[test]
    fn l_identifiant_de_la_manette_est_celui_que_ryubing_note() {
        // Relevé réel du 15/11 : manette Xbox One (045e:02ff) branchée, choisie dans Ryubing.
        assert_eq!(id_manette(0x045e, 0x02ff, 0), MANETTE_XBOX_ONE);
        assert_eq!(serde_json::from_str::<Value>(MODELE_MANETTE).unwrap()["id"], MANETTE_XBOX_ONE, "le modèle relevé");
        // Le CRC du nom (octets 2-3) est retiré par Ryubing : il ne change rien.
        let mut o = [0u8; 16];
        o[0] = 3;
        o[2] = 0xab;
        o[3] = 0xcd;
        o[4] = 0x5e;
        o[5] = 0x04;
        o[8] = 0xff;
        o[9] = 0x02;
        o[14] = b'r';
        assert_eq!(format!("0-{}", guid_net_stable(&o)), MANETTE_XBOX_ONE);
        // La manette virtuelle de Sunshine (celle d'une Xbox 360, 045e:028e, version lue par XInput) : même formule.
        assert_eq!(id_manette(0x045e, 0x028e, 0x0114), "0-00000003-045e-0000-8e02-000014017200");
    }

    #[test]
    fn le_modele_complet_est_pose_puis_regle_une_seule_fois() {
        let d = tempfile::tempdir().unwrap();
        let (ryu, racine) = ryubing_installe(d.path());
        assert!(config_valide(MODELE_CONFIG), "le modèle relevé est lisible par Ryubing");
        let r = regler_config(&ryu, &racine, Joueur1::Rien).unwrap();
        assert_eq!(r, ReglageConfig { pose: true, regle: true, turbo: true, vitesse: true, ..Default::default() });
        let v = lire_config(&racine);
        // Complet : chaque champ du modèle est là (jamais un fichier minimal).
        let modele: Value = serde_json::from_str(MODELE_CONFIG).unwrap();
        for cle in modele.as_object().unwrap().keys() {
            assert!(v.get(cle).is_some(), "{cle} manque");
        }
        assert_eq!(v["version"], 70);
        for (cle, attendu) in reglages_de_frogtend() {
            assert_eq!(v[cle], attendu, "{cle}");
        }
        assert_eq!(v["start_fullscreen"], false, "le plein écran vient de `-f`, jamais du fichier");
        assert_eq!(v["input_config"][0]["backend"], "WindowKeyboard", "sans manette branchée : le clavier reste");
        assert!(racine.join(MARQUE_REGLAGES).exists());
        // La personne choisit l'allemand et la question de sortie : jamais défait ensuite.
        let mut perso = v.clone();
        perso["system_language"] = "German".into();
        perso["show_confirm_exit"] = true.into();
        std::fs::write(config_de(&racine), serde_json::to_string_pretty(&perso).unwrap()).unwrap();
        assert_eq!(regler_config(&ryu, &racine, Joueur1::Rien).unwrap(), ReglageConfig::default());
        assert_eq!(lire_config(&racine), perso);
        // Une autre personne a SA racine, réglée pour elle.
        let lea = super::racine(&ryu, "Léa");
        assert!(regler_config(&ryu, &lea, Joueur1::Rien).unwrap().regle);
        assert_eq!(lire_config(&lea)["system_language"], "French");
        assert_eq!(lire_config(&racine)["system_language"], "German");
    }

    #[test]
    fn la_manette_branchee_va_au_joueur_1_jamais_par_dessus_celle_de_la_personne() {
        let d = tempfile::tempdir().unwrap();
        let (ryu, racine) = ryubing_installe(d.path());
        let r = regler_config(&ryu, &racine, Joueur1::Manette(MANETTE_XBOX_ONE)).unwrap();
        assert!(r.manette && r.pose && r.regle);
        let j1 = &lire_config(&racine)["input_config"][0];
        assert_eq!((j1["backend"].as_str(), j1["id"].as_str(), j1["player_index"].as_str()), (Some("GamepadSDL2"), Some(MANETTE_XBOX_ONE), Some("Player1")));
        assert_eq!(j1["right_joycon"]["button_a"], "B", "la disposition d'origine de Ryubing (relevé)");
        assert!(config_valide(&std::fs::read_to_string(config_de(&racine)).unwrap()));
        // Mis à l'abri avant d'être réécrit.
        let abris = crate::installation::fichiers_de(&ryu.join(".frogtend-sauvegardes")).unwrap();
        assert!(abris.iter().any(|f| f.ends_with("Profils/Seb/Config.json")), "{abris:?}");
        // Streaming : la manette de Sunshine n'est pas encore relevée → jamais posée ; la manette de FROGTEND (qui ne
        // répondrait plus) laisse la place au clavier d'origine, et c'est signalé.
        let sunshine = id_manette(0x045e, 0x028e, 0x0114);
        let r = regler_config(&ryu, &racine, Joueur1::Manette(&sunshine)).unwrap();
        assert!(r.manette_inconnue && r.clavier && !r.manette, "{r:?}");
        let modele: Value = serde_json::from_str(MODELE_CONFIG).unwrap();
        assert_eq!(lire_config(&racine)["input_config"][0], modele["input_config"][0]);
        assert!(!racine.join(MARQUE_MANETTE).exists());
        // Rebranchée, la manette relevée revient ; une manette inconnue au clavier ne change rien.
        assert!(regler_config(&ryu, &racine, Joueur1::Manette(MANETTE_XBOX_ONE)).unwrap().manette);
        // La personne choisit SA manette dans Ryubing : jamais remplacée.
        let mut v = lire_config(&racine);
        v["input_config"][0]["id"] = "0-00000003-054c-0000-ce0d-000000006800".into();
        std::fs::write(config_de(&racine), serde_json::to_string_pretty(&v).unwrap()).unwrap();
        assert!(!regler_config(&ryu, &racine, Joueur1::Manette(MANETTE_XBOX_ONE)).unwrap().manette);
        assert_eq!(lire_config(&racine), v);
        // … ni retirée par le clavier choisi pour un jeu, ni par une manette inconnue.
        assert_eq!(regler_config(&ryu, &racine, Joueur1::Clavier).unwrap(), ReglageConfig::default());
        assert!(!regler_config(&ryu, &racine, Joueur1::Manette(&sunshine)).unwrap().clavier);
        assert_eq!(lire_config(&racine), v);
    }

    #[test]
    fn le_clavier_choisi_pour_un_jeu_retire_la_manette_de_frogtend() {
        let d = tempfile::tempdir().unwrap();
        let (ryu, racine) = ryubing_installe(d.path());
        regler_config(&ryu, &racine, Joueur1::Manette(MANETTE_XBOX_ONE)).unwrap();
        let r = regler_config(&ryu, &racine, Joueur1::Clavier).unwrap();
        assert!(r.clavier && !r.manette && !r.manette_inconnue, "{r:?}");
        let modele: Value = serde_json::from_str(MODELE_CONFIG).unwrap();
        let j1 = &lire_config(&racine)["input_config"][0];
        assert_eq!(j1, &modele["input_config"][0], "le bloc clavier d'origine du modèle");
        assert!(!racine.join(MARQUE_MANETTE).exists());
        // Déjà au clavier : rien de plus.
        assert_eq!(regler_config(&ryu, &racine, Joueur1::Clavier).unwrap(), ReglageConfig::default());
    }

    #[test]
    fn un_reglage_propre_a_un_jeu_n_est_jamais_touche_et_decide_de_la_pause() {
        let d = tempfile::tempdir().unwrap();
        let (ryu, racine) = ryubing_installe(d.path());
        preparer_partie(&ryu, &racine, Joueur1::Rien).unwrap();
        assert!(pause_reglee(&racine) && configs_par_jeu(&racine).is_empty());
        assert!(!crate::garde::est_un_lien(&racine.join("games")) && racine.join("games").is_dir(), "son propre games\\");
        // La personne a réglé un jeu à part, sans la pause : jamais écrit, et Ryubing sera figé au menu.
        let jeu = racine.join("games").join("0100000000010000");
        std::fs::create_dir_all(&jeu).unwrap();
        std::fs::write(jeu.join(FICHIER_CONFIG), MODELE_CONFIG).unwrap();
        preparer_partie(&ryu, &racine, Joueur1::Manette(MANETTE_XBOX_ONE)).unwrap();
        assert_eq!(std::fs::read_to_string(jeu.join(FICHIER_CONFIG)).unwrap(), MODELE_CONFIG);
        assert_eq!(configs_par_jeu(&racine), vec!["0100000000010000".to_string()]);
        assert!(!pause_reglee(&racine));
        // Un réglage de jeu illisible (Ryubing ne le lirait pas) ne compte pas.
        std::fs::write(jeu.join(FICHIER_CONFIG), "abîmé").unwrap();
        assert!(pause_reglee(&racine));
    }

    #[test]
    fn un_fichier_illisible_ou_plus_recent_ou_prioritaire_n_est_jamais_touche() {
        let d = tempfile::tempdir().unwrap();
        let (ryu, racine) = ryubing_installe(d.path());
        std::fs::create_dir_all(&racine).unwrap();
        // Illisible : laissé tel quel (Frogtend ne l'écrase pas par des réglages d'origine).
        std::fs::write(config_de(&racine), "{ abîmé").unwrap();
        assert!(matches!(regler_config(&ryu, &racine, Joueur1::Manette(MANETTE_XBOX_ONE)), Err(Erreur::Refus(_))));
        assert_eq!(std::fs::read_to_string(config_de(&racine)).unwrap(), "{ abîmé");
        // D'un Ryubing plus récent (version 71) : jamais touché.
        let mut v: Value = serde_json::from_str(MODELE_CONFIG).unwrap();
        v["version"] = 71.into();
        std::fs::write(config_de(&racine), v.to_string()).unwrap();
        assert!(regler_config(&ryu, &racine, Joueur1::Rien).is_err());
        assert_eq!(lire_config(&racine)["system_language"], "AmericanEnglish");
        // Un pilote de manette inconnu ferait refuser TOUT le fichier par Ryubing : jamais écrit ainsi.
        v["version"] = 70.into();
        v["input_config"][0]["backend"] = "GamepadSDL3".into();
        assert!(!config_valide(&v.to_string()));
        // Un Config.json à côté de Ryujinx.exe passe devant : rien n'est écrit chez la personne.
        let lea = super::racine(&ryu, "Léa");
        std::fs::write(ryu.join(FICHIER_CONFIG), MODELE_CONFIG).unwrap();
        assert!(matches!(regler_config(&ryu, &lea, Joueur1::Rien), Err(Erreur::Refus(_))));
        assert!(!config_de(&lea).exists());
        assert!(!racine.join(MARQUE_REGLAGES).exists());
    }

    #[test]
    fn la_racine_de_la_personne_a_ses_cles_et_partage_le_firmware() {
        let d = tempfile::tempdir().unwrap();
        let (ryu, racine) = ryubing_installe(d.path());
        let args = preparer_partie(&ryu, &racine, Joueur1::Rien).unwrap();
        assert_eq!(args, vec!["-r".to_string(), racine.to_string_lossy().to_string()]);
        // Ses clés, copiées (jamais déplacées).
        assert_eq!(std::fs::read(racine.join("system").join("prod.keys")).unwrap(), b"cles prod");
        assert_eq!(std::fs::read(racine.join("system").join("title.keys")).unwrap(), b"cles titres");
        assert!(ryu.join("portable").join("system").join("prod.keys").is_file());
        // Le firmware, partagé : Ryubing le lit dans SA racine, par la jonction.
        let par_la_racine = racine.join("bis").join("system").join("Contents").join("registered").join(format!("{A}.nca")).join("00");
        assert_eq!(std::fs::read(par_la_racine).unwrap(), b"firmware");
        assert!(crate::garde::est_un_lien(&racine.join("bis").join("system").join("Contents")));
        assert!(!crate::garde::est_un_lien(&racine.join("games")) && racine.join("games").is_dir(), "games : chacun le sien");
        assert!(config_de(&racine).is_file(), "réglée dès la première partie");
        // Relancée : rien ne change (jonctions gardées).
        assert_eq!(preparer_partie(&ryu, &racine, Joueur1::Rien).unwrap(), args);
        // Une clé posée par la personne n'est jamais remplacée ; celle de Frogtend suit les nouvelles clés servies.
        std::fs::write(racine.join("system").join("title.keys"), b"les miennes").unwrap();
        std::fs::write(ryu.join("portable").join("system").join("prod.keys"), b"cles prod 2").unwrap();
        std::fs::write(ryu.join("portable").join("system").join("title.keys"), b"titres 2").unwrap();
        preparer_partie(&ryu, &racine, Joueur1::Rien).unwrap();
        assert_eq!(std::fs::read(racine.join("system").join("prod.keys")).unwrap(), b"cles prod 2");
        assert_eq!(std::fs::read(racine.join("system").join("title.keys")).unwrap(), b"les miennes");
        // Pas portable : rien n'est créé, pas de `-r` (ses données sont dans %APPDATA%).
        let main = d.path().join("ALaMain");
        std::fs::create_dir_all(&main).unwrap();
        assert!(preparer_partie(&main, &super::racine(&main, "Seb"), Joueur1::Rien).unwrap().is_empty());
        assert!(!main.join("Profils").exists() && !main.join("portable").exists());
    }

    #[test]
    fn la_ligne_porte_la_racine_et_le_lancement_est_refuse_si_elle_manque() {
        let d = tempfile::tempdir().unwrap();
        let (ryu, racine) = ryubing_installe(d.path());
        let args = preparer_partie(&ryu, &racine, Joueur1::Rien).unwrap();
        // La ligne telle que commandes::preparer_emulateur la compose : arguments entre guillemets, puis celle de la fiche.
        let ligne = format!("{} -f --hide-updates \"%rom%\"", args.iter().map(|a| format!("\"{a}\"")).collect::<Vec<_>>().join(" "));
        assert_eq!(racine_de_la_ligne(&ligne), Some(racine.clone()));
        assert_eq!(racine_de_la_ligne(r#"-f --root-data-dir D:\Ryu\Profils\Seb "x.nsp""#), Some(PathBuf::from(r"D:\Ryu\Profils\Seb")));
        assert_eq!(racine_de_la_ligne(r#"-f "%rom%""#), None);
        assert!(verifier_la_racine(&ligne).is_ok());
        assert!(verifier_la_racine(r#"-f "%rom%""#).is_ok(), "Ryubing pas portable : comme avant");
        assert!(matches!(verifier_la_racine(r#""-r" "Profils\Seb" -f"#), Err(Erreur::Refus(_))), "jamais un chemin relatif");
        // La racine a disparu entre la préparation et le lancement : refusé (il jouerait sur `portable\`).
        std::fs::rename(&racine, d.path().join("ailleurs")).unwrap();
        assert!(matches!(verifier_la_racine(&ligne), Err(Erreur::Refus(_))));
    }

    /// Des parties (une partie et l'index) dans des données de Ryubing.
    fn parties_dans(donnees: &Path, contenu: &[u8]) {
        let s = donnees.join("bis").join("user").join("save").join("0000000000000001").join("0");
        std::fs::create_dir_all(&s).unwrap();
        std::fs::write(s.join("partie.bin"), contenu).unwrap();
        let i = donnees.join("bis").join("system").join("save").join("8000000000000000").join("0");
        std::fs::create_dir_all(&i).unwrap();
        std::fs::write(i.join("imkvdb.arc"), [b"index de ", contenu].concat()).unwrap();
    }

    #[test]
    fn les_parties_d_avant_frogtend_vont_une_fois_chez_la_premiere_personne() {
        let d = tempfile::tempdir().unwrap();
        let (ryu, seb) = ryubing_installe(d.path());
        parties_dans(&ryu.join("portable"), b"zelda d'avant");
        preparer_partie(&ryu, &seb, Joueur1::Rien).unwrap();
        let partie = seb.join("bis").join("user").join("save").join("0000000000000001").join("0").join("partie.bin");
        assert_eq!(std::fs::read(&partie).unwrap(), b"zelda d'avant");
        let index = seb.join("bis").join("system").join("save").join("8000000000000000").join("0").join("imkvdb.arc");
        assert_eq!(std::fs::read(&index).unwrap(), b"index de zelda d'avant", "avec leur index");
        assert_eq!(std::fs::read(seb.join("system").join("Profiles.json")).unwrap(), b"{}", "et leurs utilisateurs");
        assert!(ryu.join("portable").join("bis").join("user").join("save").join("0000000000000001").is_dir(), "copiées, jamais déplacées");
        assert!(!crate::emulateurs_profils::reprise_en_attente(&ryu));
        // Une seule fois : Léa n'en reçoit rien ; Seb, qui a rejoué, n'est jamais écrasé.
        std::fs::write(&partie, b"zelda plus loin").unwrap();
        let lea = super::racine(&ryu, "Léa");
        preparer_partie(&ryu, &lea, Joueur1::Rien).unwrap();
        assert!(!a_des_parties(&lea));
        preparer_partie(&ryu, &seb, Joueur1::Rien).unwrap();
        assert_eq!(std::fs::read(&partie).unwrap(), b"zelda plus loin");
    }

    #[test]
    fn un_bloc_ne_se_pose_que_chez_qui_n_a_aucune_partie() {
        let d = tempfile::tempdir().unwrap();
        let (ryu, seb) = ryubing_installe(d.path());
        let sauvegarde = d.path().join("attente");
        parties_dans(&sauvegarde, b"restauree");
        std::fs::create_dir_all(sauvegarde.join("system")).unwrap();
        std::fs::write(sauvegarde.join("system").join("Profiles.json"), b"utilisateurs de Seb").unwrap();
        // Un index vide (Ryubing lancé une fois) et ses utilisateurs d'origine : mis à l'abri, jamais effacés.
        let index = seb.join("bis").join("system").join("save").join("8000000000000000").join("0").join("imkvdb.arc");
        std::fs::create_dir_all(index.parent().unwrap()).unwrap();
        std::fs::write(&index, b"index vide").unwrap();
        std::fs::create_dir_all(seb.join("system")).unwrap();
        std::fs::write(seb.join("system").join("Profiles.json"), b"RyuPlayer").unwrap();
        std::fs::write(sauvegarde.join(FICHIER_CONFIG), MODELE_CONFIG.replace("AmericanEnglish", "German")).unwrap();
        std::fs::write(sauvegarde.join(MARQUE_REGLAGES), b"").unwrap();
        assert_eq!(reposer_sauvegarde(&ryu, &seb, &sauvegarde).unwrap(), 5);
        assert_eq!(lire_config(&seb)["system_language"], "German", "ses réglages reviennent, marque comprise");
        assert!(!regler_config(&ryu, &seb, Joueur1::Rien).unwrap().regle, "jamais réglés de nouveau");
        assert_eq!(std::fs::read(&index).unwrap(), b"index de restauree");
        assert_eq!(std::fs::read(seb.join("system").join("Profiles.json")).unwrap(), b"utilisateurs de Seb");
        let abri = crate::installation::fichiers_de(&ryu.join(".frogtend-sauvegardes")).unwrap();
        assert!(abri.iter().any(|f| f.ends_with("Seb/bis/system/save/8000000000000000/0/imkvdb.arc")), "{abri:?}");
        assert!(abri.iter().any(|f| f.ends_with("Seb/system/Profiles.json")), "{abri:?}");
        assert!(!seb.join(".frogtend-bloc").exists());
        // Des parties déjà là : refus, rien ne bouge (jamais deux index mêlés).
        parties_dans(&sauvegarde, b"autre");
        assert!(matches!(reposer_sauvegarde(&ryu, &seb, &sauvegarde), Err(Erreur::Refus(_))));
        assert_eq!(std::fs::read(&index).unwrap(), b"index de restauree");
    }

    #[test]
    fn des_parties_sans_leur_index_ne_se_posent_jamais() {
        let d = tempfile::tempdir().unwrap();
        let (ryu, seb) = ryubing_installe(d.path());
        let sauvegarde = d.path().join("attente");
        parties_dans(&sauvegarde, b"orpheline");
        std::fs::remove_dir_all(sauvegarde.join("bis").join("system").join("save").join("8000000000000000")).unwrap();
        // La base des Mii seule ne fait pas un index.
        std::fs::create_dir_all(sauvegarde.join("bis/system/save/8000000000000030/0")).unwrap();
        std::fs::write(sauvegarde.join("bis/system/save/8000000000000030/0/miidb"), b"mii").unwrap();
        assert!(matches!(reposer_sauvegarde(&ryu, &seb, &sauvegarde), Err(Erreur::Refus(_))));
        assert!(!a_des_parties(&seb) && !seb.join("bis").join("system").join("save").exists());
        // Les parties d'avant Frogtend non plus : laissées où elles sont, jamais à moitié chez la personne.
        parties_dans(&ryu.join("portable"), b"zelda");
        std::fs::remove_dir_all(ryu.join("portable/bis/system/save/8000000000000000")).unwrap();
        preparer_partie(&ryu, &seb, Joueur1::Rien).unwrap();
        assert!(!a_des_parties(&seb));
    }

    #[test]
    fn seules_les_parties_l_index_les_utilisateurs_et_les_reglages_sont_sauvegardes() {
        let d = tempfile::tempdir().unwrap();
        let (ryu, seb) = ryubing_installe(d.path());
        parties_dans(&ryu.join("portable"), b"zelda");
        preparer_partie(&ryu, &seb, Joueur1::Manette(MANETTE_XBOX_ONE)).unwrap();
        // Ce que Ryubing écrit aussi dans la racine : caches (`games\`), et la base des Mii (sauvegarde système).
        std::fs::create_dir_all(seb.join("games").join("0100000000010000").join("cache")).unwrap();
        std::fs::write(seb.join("games").join("0100000000010000").join("cache").join("shader.cache"), b"x").unwrap();
        std::fs::create_dir_all(seb.join("bis").join("system").join("save").join("8000000000000010")).unwrap();
        std::fs::write(seb.join("bis").join("system").join("save").join("8000000000000010").join("autre"), b"x").unwrap();
        let l = fichiers_a_sauvegarder(&seb).unwrap();
        assert_eq!(
            l,
            vec![
                "bis/user/save/0000000000000001/0/partie.bin",
                "bis/system/save/8000000000000000/0/imkvdb.arc",
                "bis/system/save/8000000000000010/autre",
                "system/Profiles.json",
                "Config.json",
                MARQUE_REGLAGES,
            ]
        );
        assert!(!a_sauvegarder("system/prod.keys") && !a_sauvegarder("bis/user/save/x/prod.keys"));
        assert!(!a_sauvegarder("bis/system/Contents/registered/a.nca/00") && !a_sauvegarder("games/0100/cache/x"));
        assert!(!a_sauvegarder(NOTE_CLES) && !a_sauvegarder(MARQUE_MANETTE));
        assert!(a_sauvegarder("bis/user/saveMeta/0000000000000001/x") && a_sauvegarder("bis/system/save/8000000000000030/0/miidb"));
        // Le mot de passe du réseau local ne part jamais : une copie sans lui.
        let mut v: Value = serde_json::from_str(MODELE_CONFIG).unwrap();
        v["multiplayer_ldn_passphrase"] = "secret-ldn".into();
        let copie = config_a_envoyer(&v.to_string()).unwrap();
        assert!(!copie.contains("secret-ldn") && config_valide(&copie));
        assert!(config_a_envoyer("abîmé").is_none());
    }

    // --- Lot 5 ---

    /// Les raccourcis du Config.json de la personne, modifiés par `f`.
    fn changer_raccourcis(racine: &Path, f: impl Fn(&mut Value)) {
        let mut v = lire_config(racine);
        f(&mut v["hotkeys"]);
        std::fs::write(config_de(racine), serde_json::to_string_pretty(&v).unwrap()).unwrap();
    }

    #[test]
    fn l_avance_rapide_est_liee_a_f13_une_fois_jamais_par_dessus_un_choix() {
        use crate::pilotage::Action::*;
        let d = tempfile::tempdir().unwrap();
        let (ryu, seb) = ryubing_installe(d.path());
        assert!(regler_config(&ryu, &seb, Joueur1::Rien).unwrap().turbo);
        let h = &lire_config(&seb)["hotkeys"];
        assert_eq!((h["turbo_mode"].as_str(), h["turbo_mode_while_held"].as_bool()), (Some("F13"), Some(false)), "une bascule");
        assert_eq!(actions_d_origine(&ryu, &seb), vec![Capture, Muet, AvanceRapide]);
        // Sa vitesse : 0 dans un Ryubing neuf (rien n'accélérerait), 200 après Frogtend ; un choix (150) gardé.
        assert_eq!(serde_json::from_str::<Value>(MODELE_CONFIG).unwrap()["tick_scalar"], 0);
        assert_eq!(lire_config(&seb)["tick_scalar"], 200);
        let mut v = lire_config(&seb);
        v["tick_scalar"] = 150.into();
        std::fs::write(config_de(&seb), serde_json::to_string_pretty(&v).unwrap()).unwrap();
        assert!(!regler_config(&ryu, &seb, Joueur1::Rien).unwrap().vitesse);
        assert_eq!(lire_config(&seb)["tick_scalar"], 150);
        // Remise à 0 (un Ryubing qui la réécrit) : reposée à 200, même la marque du turbo posée ; à 0, jamais proposée.
        let mut v = lire_config(&seb);
        v["tick_scalar"] = 0.into();
        std::fs::write(config_de(&seb), serde_json::to_string_pretty(&v).unwrap()).unwrap();
        assert_eq!(actions_d_origine(&ryu, &seb), vec![Capture, Muet]);
        assert!(regler_config(&ryu, &seb, Joueur1::Rien).unwrap().vitesse);
        assert_eq!(lire_config(&seb)["tick_scalar"], 200);
        // La personne la remet à « Unbound » (ou ailleurs) : jamais reposée.
        changer_raccourcis(&seb, |h| h["turbo_mode"] = "Unbound".into());
        assert!(!regler_config(&ryu, &seb, Joueur1::Rien).unwrap().turbo);
        assert_eq!(lire_config(&seb)["hotkeys"]["turbo_mode"], "Unbound");
        assert_eq!(actions_d_origine(&ryu, &seb), vec![Capture, Muet]);
        // F13 déjà à une autre action, ou l'avance rapide « tant qu'on appuie » : rien posé, rien proposé.
        let lea = super::racine(&ryu, "Léa");
        std::fs::create_dir_all(&lea).unwrap();
        let mut v: Value = serde_json::from_str(MODELE_CONFIG).unwrap();
        v["hotkeys"]["show_ui"] = "F13".into();
        std::fs::write(config_de(&lea), v.to_string()).unwrap();
        assert!(!regler_config(&ryu, &lea, Joueur1::Rien).unwrap().turbo);
        assert_eq!(lire_config(&lea)["hotkeys"]["turbo_mode"], "Unbound");
        let zoe = super::racine(&ryu, "Zoé");
        std::fs::create_dir_all(&zoe).unwrap();
        v["hotkeys"]["show_ui"] = "F4".into();
        v["hotkeys"]["turbo_mode_while_held"] = true.into();
        std::fs::write(config_de(&zoe), v.to_string()).unwrap();
        assert!(!regler_config(&ryu, &zoe, Joueur1::Rien).unwrap().turbo);
        changer_raccourcis(&zoe, |h| h["turbo_mode"] = "F13".into());
        assert!(!actions_d_origine(&ryu, &zoe).contains(&AvanceRapide), "tant qu'on appuie : la touche envoyée ne ferait qu'un instant");
    }

    #[test]
    fn une_action_n_est_proposee_que_si_sa_touche_est_la_bonne_et_a_elle_seule() {
        use crate::pilotage::Action::*;
        let d = tempfile::tempdir().unwrap();
        let (ryu, seb) = ryubing_installe(d.path());
        assert!(actions_d_origine(&ryu, &seb).is_empty(), "pas encore de réglages : rien");
        regler_config(&ryu, &seb, Joueur1::Rien).unwrap();
        changer_raccourcis(&seb, |h| h["screenshot"] = "F9".into());
        assert_eq!(actions_d_origine(&ryu, &seb), vec![Muet, AvanceRapide], "capture déplacée par la personne : jamais devinée");
        // Une autre action sur F2 (la synchro verticale, testée AVANT le son par Ryubing) : le son n'est plus proposé.
        changer_raccourcis(&seb, |h| h["toggle_vsync_mode"] = "F2".into());
        assert_eq!(actions_d_origine(&ryu, &seb), vec![AvanceRapide]);
    }

    #[test]
    fn un_effet_d_image_se_pose_au_lancement_et_aucun_rend_celui_de_la_personne() {
        let d = tempfile::tempdir().unwrap();
        let (ryu, seb) = ryubing_installe(d.path());
        assert!(poser_effet(&ryu, &seb, None, Some("ryubing:contours")).is_err(), "pas encore de réglages : refusé");
        regler_config(&ryu, &seb, Joueur1::Rien).unwrap();
        let paire = || {
            let v = lire_config(&seb);
            (v["anti_aliasing"].as_str().unwrap().to_string(), v["scaling_filter"].as_str().unwrap().to_string())
        };
        let p = |a: &str, b: &str| (a.to_string(), b.to_string());
        let de_frogtend = |e: &str| EffetPose { effet: Some(e.to_string()), de_la_personne: false };
        assert_eq!(paire(), p("None", "Bilinear"), "d'origine");
        assert_eq!(poser_effet(&ryu, &seb, None, Some("ryubing:contours")).unwrap(), de_frogtend("ryubing:contours"));
        assert_eq!(paire(), p("SmaaHigh", "Bilinear"));
        assert!(config_valide(&std::fs::read_to_string(config_de(&seb)).unwrap()));
        assert_eq!(poser_effet(&ryu, &seb, None, Some("ryubing:pixels")).unwrap(), de_frogtend("ryubing:pixels"));
        assert_eq!(paire(), p("None", "Nearest"));
        // « Aucun » (ou pas de choix, ou un effet d'un autre émulateur) : l'image d'origine revient.
        assert_eq!(poser_effet(&ryu, &seb, None, Some("aucun")).unwrap(), EffetPose { effet: None, de_la_personne: false });
        assert_eq!(paire(), p("None", "Bilinear"));
        assert!(!seb.join(MARQUE_EFFET).exists());
        poser_effet(&ryu, &seb, None, Some("ryubing:contours")).unwrap();
        assert_eq!(poser_effet(&ryu, &seb, None, Some("bigpemu:crt-hyllian")).unwrap().effet, None);
        assert_eq!(paire(), p("None", "Bilinear"));
        // Son réglage à elle (fait dans Ryubing) GAGNE : jamais remplacé, dit « de la personne » ; égal à un des effets,
        // il est reconnu (le menu ne dit pas « aucun » à tort).
        let mut v = lire_config(&seb);
        v["anti_aliasing"] = "Fxaa".into();
        std::fs::write(config_de(&seb), serde_json::to_string_pretty(&v).unwrap()).unwrap();
        assert_eq!(poser_effet(&ryu, &seb, None, Some("ryubing:pixels")).unwrap(), EffetPose { effet: Some("ryubing:adouci".into()), de_la_personne: true });
        assert_eq!(paire(), p("Fxaa", "Bilinear"));
        // Changé dans Ryubing pendant que l'effet de Frogtend était posé : c'est son choix, gardé.
        v["anti_aliasing"] = "None".into();
        std::fs::write(config_de(&seb), serde_json::to_string_pretty(&v).unwrap()).unwrap();
        poser_effet(&ryu, &seb, None, Some("ryubing:contours")).unwrap();
        let mut v = lire_config(&seb);
        v["anti_aliasing"] = "SmaaUltra".into();
        std::fs::write(config_de(&seb), serde_json::to_string_pretty(&v).unwrap()).unwrap();
        assert_eq!(poser_effet(&ryu, &seb, None, None).unwrap(), EffetPose { effet: None, de_la_personne: true });
        assert_eq!(paire(), p("SmaaUltra", "Bilinear"));
        assert!(!seb.join(MARQUE_EFFET).exists());
        // Un Config.json à côté du programme passe devant : refusé, dit simplement.
        std::fs::write(ryu.join(FICHIER_CONFIG), MODELE_CONFIG).unwrap();
        assert!(matches!(poser_effet(&ryu, &seb, None, Some("ryubing:net")), Err(Erreur::Refus(m)) if m.contains("pas été installé par Frogtend")));
        assert!(actions_d_origine(&ryu, &seb).is_empty(), "sa touche pourrait y faire autre chose");
        assert_eq!(paire(), p("SmaaUltra", "Bilinear"));
    }

    #[test]
    fn un_jeu_qui_a_ses_propres_reglages_n_a_pas_d_effet_d_image() {
        let d = tempfile::tempdir().unwrap();
        let (ryu, seb) = ryubing_installe(d.path());
        regler_config(&ryu, &seb, Joueur1::Rien).unwrap();
        assert!(effets_possibles(&ryu, &seb, Some(0x0100000000010000)).is_ok());
        let jeu = seb.join("games").join("0100000000010000");
        std::fs::create_dir_all(&jeu).unwrap();
        std::fs::write(jeu.join(FICHIER_CONFIG), MODELE_CONFIG).unwrap();
        // CE jeu (son numéro lu dans le .nsp ; une mise à jour …800 mène au même jeu) : refusé, et dit.
        let e = effets_possibles(&ryu, &seb, Some(0x0100000000010800)).unwrap_err();
        assert!(e.contains("Ce jeu a ses propres réglages"), "{e}");
        assert!(poser_effet(&ryu, &seb, Some(0x0100000000010000), Some("ryubing:contours")).is_err());
        assert_eq!(lire_config(&seb)["anti_aliasing"], "None", "rien n'est posé");
        // Un autre jeu : possible ; un jeu au numéro inconnu (.xci) : prudence, refusé.
        assert!(effets_possibles(&ryu, &seb, Some(0x0100000000020000)).is_ok());
        assert!(effets_possibles(&ryu, &seb, None).is_err());
        // Ryubing pas installé par Frogtend (pas portable) : la phrase dite sur la fiche.
        let main = d.path().join("ALaMain");
        std::fs::create_dir_all(&main).unwrap();
        assert_eq!(effets_possibles(&main, &seb, None).unwrap_err(), "Ryubing n'a pas été installé par Frogtend : ses réglages d'image restent les siens.");
        assert!(actions_d_origine(&main, &seb).is_empty());
    }

    #[test]
    fn le_schema_des_touches_suit_la_manette_de_frogtend_seulement() {
        let d = tempfile::tempdir().unwrap();
        let (ryu, seb) = ryubing_installe(d.path());
        assert_eq!(joueur1_lu(&seb), Joueur1Lu::AuClavier, "pas encore réglé : il le sera au clavier");
        regler_config(&ryu, &seb, Joueur1::Manette(MANETTE_RELEVEE)).unwrap();
        assert_eq!(joueur1_lu(&seb), Joueur1Lu::DeFrogtend);
        // Le nom affiché changé par Ryubing ne compte pas ; un bouton changé par la personne, si.
        let mut v = lire_config(&seb);
        v["input_config"][0]["name"] = "Xbox One Controller (0)".into();
        std::fs::write(config_de(&seb), serde_json::to_string_pretty(&v).unwrap()).unwrap();
        assert_eq!(joueur1_lu(&seb), Joueur1Lu::DeFrogtend);
        v["input_config"][0]["right_joycon"]["button_a"] = "A".into();
        std::fs::write(config_de(&seb), serde_json::to_string_pretty(&v).unwrap()).unwrap();
        assert_eq!(joueur1_lu(&seb), Joueur1Lu::Autre);
        assert!(!crate::manettes::disposition_de_frogtend("ryubing", &ryu, Some(&seb)));
        let lea = super::racine(&ryu, "Léa");
        regler_config(&ryu, &lea, Joueur1::Clavier).unwrap();
        assert_eq!(joueur1_lu(&lea), Joueur1Lu::AuClavier);
        assert!(crate::manettes::disposition_de_frogtend("ryubing", &ryu, Some(&lea)));
    }

    // --- Lot 6 : mises à jour et DLC ---

    /// Ryubing installé, la clé de TEST dans la racine de Seb, et le dossier de Zelda fabriqué (switch_nca.rs).
    fn zelda_pret(d: &Path) -> (PathBuf, PathBuf, PathBuf) {
        let (ryu, seb) = ryubing_installe(d);
        preparer_racine(&ryu, &seb).unwrap();
        let o: String = (0..32u8).map(|i| format!("{:02x}", i.wrapping_mul(7).wrapping_add(3))).collect();
        std::fs::write(seb.join("system").join("prod.keys"), format!("header_key = {o}\n")).unwrap();
        let jeux = d.join("Jeux");
        std::fs::create_dir_all(&jeux).unwrap();
        let jeu = crate::switch_nca::tests::dossier_de_zelda(&jeux);
        (ryu, seb, jeu)
    }

    fn json(f: &Path) -> Value {
        serde_json::from_str(&std::fs::read_to_string(f).unwrap()).unwrap()
    }

    #[test]
    fn la_maj_et_les_dlc_sont_poses_dans_la_forme_que_ryubing_lit() {
        let d = tempfile::tempdir().unwrap();
        let (ryu, seb, jeu) = zelda_pret(d.path());
        let r = preparer_contenus(&ryu, &seb, &jeu).unwrap().unwrap();
        // Le dossier : l'identifiant de base en hexa MINUSCULE (`ToString("x16")`), comme celui que Ryubing a créé chez Seb.
        let dossier = seb.join("games").join("01007ef00011e000");
        assert_eq!(dossier_du_jeu(&seb, crate::switch_nca::tests::BASE), dossier);
        let maj = jeu.parent().unwrap().join("Zelda maj [v786432].nsp").to_string_lossy().to_string();
        let u = json(&dossier.join("updates.json"));
        assert_eq!(u, serde_json::json!({ "selected": maj, "paths": [maj] }), "la plus récente, jamais celle d'un autre jeu");
        let l = json(&dossier.join("dlc.json"));
        let p1 = jeu.parent().unwrap().join("DLC_Pack_1.nsp").to_string_lossy().to_string();
        assert_eq!(l.as_array().unwrap().len(), 2);
        assert_eq!(l[0], serde_json::json!({ "path": p1, "dlc_nca_list": [{ "path": "/a1.nca", "title_id": 0x0100_7EF0_0011_F001u64, "is_enabled": true }] }));
        // L'identifiant écrit en DÉCIMAL (un nombre JSON, `ulong` de Ryubing), jamais en texte hexadécimal.
        assert!(std::fs::read_to_string(dossier.join("dlc.json")).unwrap().contains(&format!("\"title_id\": {}", 0x0100_7EF0_0011_F001u64)));
        assert_eq!((r.maj.as_deref(), r.dlc.len(), r.ecrit, r.choix_maj), (Some("Zelda maj [v786432].nsp"), 2, true, ChoixMaj::DeFrogtend));
        assert!(r.resume().contains("Zelda maj [v786432].nsp") && r.resume().contains("2 DLC"), "{}", r.resume());
        // La note de Frogtend : écrite par un fichier provisoire renommé, aucun provisoire laissé.
        assert!(seb.join(NOTE_CONTENUS).is_file());
        assert!(!std::fs::read_dir(&seb).unwrap().flatten().any(|e| e.file_name().to_string_lossy().ends_with(".frogtend")));
        assert!(r.erreurs.iter().any(|e| e.contains("abime.nsp")), "un fichier illisible est dit");
        // Relancé : rien ne change, rien n'est réécrit.
        let r2 = preparer_contenus(&ryu, &seb, &jeu).unwrap().unwrap();
        assert!(!r2.ecrit);
        assert_eq!(json(&dossier.join("updates.json")), u);
        // Une mise à jour plus récente arrive : elle remplace celle de Frogtend, l'ancienne reste dans la liste.
        std::fs::copy(jeu.parent().unwrap().join("Zelda maj [v786432].nsp"), jeu.parent().unwrap().join("Zelda maj [v851968].nsp")).unwrap();
        preparer_contenus(&ryu, &seb, &jeu).unwrap().unwrap();
        let u = json(&dossier.join("updates.json"));
        assert!(u["selected"].as_str().unwrap().ends_with("[v851968].nsp"));
        assert_eq!(u["paths"].as_array().unwrap().len(), 2);
        // Et l'ancien fichier a été mis à l'abri avant d'être remplacé.
        assert!(crate::installation::fichiers_de(&ryu.join(".frogtend-sauvegardes")).unwrap().iter().any(|f| f.ends_with("updates.json")));
    }

    #[test]
    fn le_choix_de_la_personne_dans_ryubing_n_est_jamais_ecrase() {
        let d = tempfile::tempdir().unwrap();
        let (ryu, seb, jeu) = zelda_pret(d.path());
        let dossier = seb.join("games").join("01007ef00011e000");
        std::fs::create_dir_all(&dossier).unwrap();
        let vieille = jeu.parent().unwrap().join("Zelda vieille maj [v65536].nsp").to_string_lossy().to_string();
        let p1 = jeu.parent().unwrap().join("DLC_Pack_1.nsp").to_string_lossy().to_string();
        let disparu = d.path().join("Ailleurs").join("DLC perdu.nsp").to_string_lossy().to_string();
        std::fs::write(dossier.join("updates.json"), serde_json::to_string(&serde_json::json!({ "selected": vieille, "paths": [vieille] })).unwrap()).unwrap();
        // Le DLC 1 désactivé par la personne, et un DLC à elle dont le fichier a disparu.
        let dlc_personne = serde_json::json!([
            { "path": p1, "dlc_nca_list": [{ "path": "/a1.nca", "title_id": 1u64, "is_enabled": false }] },
            { "path": disparu, "dlc_nca_list": [] }
        ]);
        std::fs::write(dossier.join("dlc.json"), serde_json::to_string(&dlc_personne).unwrap()).unwrap();
        let r = preparer_contenus(&ryu, &seb, &jeu).unwrap().unwrap();
        assert_eq!(r.choix_maj, ChoixMaj::DeLaPersonne);
        assert_eq!(r.dlc_coupes, ["DLC_Pack_1.nsp"], "coupé par la personne : jamais montré comme utilisé");
        assert_eq!(r.dlc, ["DLC_Pack_2.nsp"]);
        let u = json(&dossier.join("updates.json"));
        assert_eq!(u["selected"], vieille.as_str(), "sa mise à jour reste choisie");
        assert_eq!(u["paths"].as_array().unwrap().len(), 2, "la nôtre est seulement proposée dans la liste");
        let l = json(&dossier.join("dlc.json"));
        assert_eq!(l[0], dlc_personne[0], "son DLC désactivé reste tel quel");
        assert_eq!(l[1], dlc_personne[1], "son DLC disparu n'est jamais retiré par Frogtend");
        assert!(l[2]["path"].as_str().unwrap().ends_with("DLC_Pack_2.nsp"));
        // Le DLC 2 (posé par Frogtend) disparaît : seul lui est retiré.
        std::fs::remove_file(jeu.parent().unwrap().join("DLC_Pack_2.nsp")).unwrap();
        preparer_contenus(&ryu, &seb, &jeu).unwrap().unwrap();
        let l = json(&dossier.join("dlc.json"));
        assert_eq!(l.as_array().unwrap().len(), 2);
        assert_eq!(l[1], dlc_personne[1]);
        // « Aucune mise à jour » choisie dans Ryubing (`""`, la nôtre toujours listée) : gardé.
        let notre = jeu.parent().unwrap().join("Zelda maj [v786432].nsp").to_string_lossy().to_string();
        std::fs::write(dossier.join("updates.json"), serde_json::to_string(&serde_json::json!({ "selected": "", "paths": [vieille, notre] })).unwrap()).unwrap();
        let r = preparer_contenus(&ryu, &seb, &jeu).unwrap().unwrap();
        assert_eq!(r.choix_maj, ChoixMaj::DeLaPersonne);
        assert_eq!(json(&dossier.join("updates.json"))["selected"], "");
        // La nôtre retirée de la liste par la personne : retirée, jamais remise.
        std::fs::write(dossier.join("updates.json"), serde_json::to_string(&serde_json::json!({ "selected": "", "paths": [vieille] })).unwrap()).unwrap();
        assert_eq!(preparer_contenus(&ryu, &seb, &jeu).unwrap().unwrap().choix_maj, ChoixMaj::RetireeParLaPersonne);
        assert_eq!(json(&dossier.join("updates.json"))["paths"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn un_dlc_retire_dans_ryubing_n_est_jamais_repose() {
        let d = tempfile::tempdir().unwrap();
        let (ryu, seb, jeu) = zelda_pret(d.path());
        let dossier = seb.join("games").join("01007ef00011e000");
        preparer_contenus(&ryu, &seb, &jeu).unwrap().unwrap();
        // La personne retire le DLC 2 dans Ryubing (il réécrit la liste sans lui ; le fichier, lui, reste).
        let mut l = json(&dossier.join("dlc.json"));
        l.as_array_mut().unwrap().retain(|c| !c["path"].as_str().unwrap().ends_with("DLC_Pack_2.nsp"));
        std::fs::write(dossier.join("dlc.json"), serde_json::to_string_pretty(&l).unwrap()).unwrap();
        for _ in 0..2 {
            let r = preparer_contenus(&ryu, &seb, &jeu).unwrap().unwrap();
            assert_eq!((r.dlc.clone(), r.dlc_retires.clone()), (vec!["DLC_Pack_1.nsp".to_string()], vec!["DLC_Pack_2.nsp".to_string()]));
            assert_eq!(json(&dossier.join("dlc.json")), l, "jamais remis, ni à ce lancement ni au suivant");
        }
        // Tout retiré (`[]`) : rien n'est remis.
        std::fs::write(dossier.join("dlc.json"), b"[]").unwrap();
        let r = preparer_contenus(&ryu, &seb, &jeu).unwrap().unwrap();
        assert_eq!(r.dlc_retires.len(), 2);
        assert_eq!(json(&dossier.join("dlc.json")), serde_json::json!([]));
    }

    #[test]
    fn une_maj_retiree_dans_ryubing_n_est_jamais_remise_seule_une_nouvelle_l_est() {
        let d = tempfile::tempdir().unwrap();
        let (ryu, seb, jeu) = zelda_pret(d.path());
        let dossier = seb.join("games").join("01007ef00011e000");
        preparer_contenus(&ryu, &seb, &jeu).unwrap().unwrap();
        // Tout décoché ET retiré dans Ryubing : `""` et une liste vide.
        std::fs::write(dossier.join("updates.json"), br#"{"selected":"","paths":[]}"#).unwrap();
        for _ in 0..2 {
            let r = preparer_contenus(&ryu, &seb, &jeu).unwrap().unwrap();
            assert_eq!(r.choix_maj, ChoixMaj::RetireeParLaPersonne);
            assert_eq!(std::fs::read(dossier.join("updates.json")).unwrap(), br#"{"selected":"","paths":[]}"#);
        }
        // Retirée de la liste mais une autre choisie : pas remise non plus.
        let vieille = jeu.parent().unwrap().join("Zelda vieille maj [v65536].nsp").to_string_lossy().to_string();
        let a_elle = serde_json::json!({ "selected": vieille, "paths": [vieille] });
        std::fs::write(dossier.join("updates.json"), serde_json::to_string(&a_elle).unwrap()).unwrap();
        assert_eq!(preparer_contenus(&ryu, &seb, &jeu).unwrap().unwrap().choix_maj, ChoixMaj::RetireeParLaPersonne);
        assert_eq!(json(&dossier.join("updates.json")), a_elle);
        // Une mise à jour NOUVELLE arrive : elle, oui (ajoutée à la liste ; le choix de la personne reste).
        std::fs::copy(jeu.parent().unwrap().join("Zelda maj [v786432].nsp"), jeu.parent().unwrap().join("Zelda maj [v851968].nsp")).unwrap();
        let r = preparer_contenus(&ryu, &seb, &jeu).unwrap().unwrap();
        assert_eq!(r.choix_maj, ChoixMaj::DeLaPersonne);
        let u = json(&dossier.join("updates.json"));
        assert_eq!(u["selected"], vieille.as_str());
        assert!(u["paths"].as_array().unwrap().iter().any(|p| p.as_str().unwrap().ends_with("[v851968].nsp")));
        // Sans choix de la personne (liste vidée par elle avant), une nouvelle est choisie.
        std::fs::write(dossier.join("updates.json"), br#"{"selected":"","paths":[]}"#).unwrap();
        std::fs::copy(jeu.parent().unwrap().join("Zelda maj [v786432].nsp"), jeu.parent().unwrap().join("Zelda maj [v917504].nsp")).unwrap();
        let r = preparer_contenus(&ryu, &seb, &jeu).unwrap().unwrap();
        assert_eq!(r.choix_maj, ChoixMaj::DeFrogtend);
        assert!(json(&dossier.join("updates.json"))["selected"].as_str().unwrap().ends_with("[v917504].nsp"));
    }

    #[test]
    fn un_dlc_de_frogtend_devenu_illisible_ou_d_un_autre_jeu_quitte_la_liste() {
        use crate::switch_nca::tests::{cle_de_test, nca, partition};
        let d = tempfile::tempdir().unwrap();
        let (ryu, seb, jeu) = zelda_pret(d.path());
        let dossier = seb.join("games").join("01007ef00011e000");
        let p1 = jeu.parent().unwrap().join("DLC_Pack_1.nsp");
        let p2 = jeu.parent().unwrap().join("DLC_Pack_2.nsp");
        preparer_contenus(&ryu, &seb, &jeu).unwrap().unwrap();
        // Le DLC 1 abîmé (illisible), le DLC 2 remplacé par le DLC d'un autre jeu : les deux entrées de Frogtend partent
        // (Ryubing ouvrirait conteneur et NCA sans pardon).
        std::fs::write(&p1, b"PFS0 abime").unwrap();
        std::fs::write(&p2, partition(&[("z.nca", nca(&cle_de_test(), 5, 0x0100_AAAA_0000_1001, 0))], false)).unwrap();
        let r = preparer_contenus(&ryu, &seb, &jeu).unwrap().unwrap();
        assert!(r.dlc.is_empty());
        assert_eq!(json(&dossier.join("dlc.json")), serde_json::json!([]));
        assert!(r.erreurs.iter().any(|e| e.contains("DLC_Pack_1.nsp")), "l'illisible est dit : {:?}", r.erreurs);
    }

    #[test]
    fn une_extension_que_ryubing_ne_lirait_pas_n_est_pas_posee_et_le_dit() {
        let d = tempfile::tempdir().unwrap();
        let (ryu, seb, jeu) = zelda_pret(d.path());
        std::fs::rename(jeu.parent().unwrap().join("DLC_Pack_2.nsp"), jeu.parent().unwrap().join("DLC_Pack_2.NSP")).unwrap();
        let r = preparer_contenus(&ryu, &seb, &jeu).unwrap().unwrap();
        assert_eq!(r.dlc, ["DLC_Pack_1.nsp"]);
        assert!(r.erreurs.iter().any(|e| e.contains("DLC_Pack_2.NSP") && e.contains("minuscules")), "{:?}", r.erreurs);
        let l = json(&seb.join("games").join("01007ef00011e000").join("dlc.json"));
        assert_eq!(l.as_array().unwrap().len(), 1);
    }

    #[test]
    fn un_fichier_illisible_n_est_jamais_touche_et_le_reste_est_fait() {
        let d = tempfile::tempdir().unwrap();
        let (ryu, seb, jeu) = zelda_pret(d.path());
        let dossier = seb.join("games").join("01007ef00011e000");
        std::fs::create_dir_all(&dossier).unwrap();
        std::fs::write(dossier.join("updates.json"), b"{abime").unwrap();
        let r = preparer_contenus(&ryu, &seb, &jeu).unwrap().unwrap();
        assert_eq!(std::fs::read(dossier.join("updates.json")).unwrap(), b"{abime");
        assert!(r.erreurs.iter().any(|e| e.contains("mise à jour non utilisée")), "{:?}", r.erreurs);
        assert_eq!(json(&dossier.join("dlc.json")).as_array().unwrap().len(), 2, "les DLC sont posés quand même");
        // Un dlc.json qui n'est pas une liste : pas touché non plus.
        std::fs::write(dossier.join("dlc.json"), b"{}").unwrap();
        let r = preparer_contenus(&ryu, &seb, &jeu).unwrap().unwrap();
        assert_eq!(std::fs::read(dossier.join("dlc.json")).unwrap(), b"{}");
        assert!(r.erreurs.iter().any(|e| e.contains("forme attendue")));
    }

    #[test]
    fn sans_cle_ou_sans_mode_portable_rien_n_est_pose() {
        let d = tempfile::tempdir().unwrap();
        let (ryu, seb, jeu) = zelda_pret(d.path());
        // Le prod.keys de test sans `header_key` (ici, celui de portable\ aussi) : refusé, rien deviné.
        std::fs::write(seb.join("system").join("prod.keys"), b"cles prod").unwrap();
        assert!(matches!(preparer_contenus(&ryu, &seb, &jeu), Err(Erreur::Refus(_))));
        assert!(!seb.join("games").join("01007ef00011e000").join("dlc.json").exists());
        // Une mise à jour lancée comme jeu : pas un jeu, rien à poser.
        let o: String = (0..32u8).map(|i| format!("{:02x}", i.wrapping_mul(7).wrapping_add(3))).collect();
        std::fs::write(seb.join("system").join("prod.keys"), format!("header_key = {o}\n")).unwrap();
        assert_eq!(preparer_contenus(&ryu, &seb, &jeu.parent().unwrap().join("Zelda maj [v786432].nsp")).unwrap(), None);
        // Pas portable : ses données sont dans %APPDATA%, Frogtend n'y écrit jamais.
        std::fs::remove_dir_all(ryu.join(DOSSIER_PORTABLE)).unwrap();
        assert_eq!(preparer_contenus(&ryu, &seb, &jeu).unwrap(), None);
    }

    #[test]
    fn les_nca_d_un_dlc_de_frogtend_sont_relus_et_le_choix_actif_garde() {
        let existe = |_: &str| true;
        let poses = vec!["D:\\J\\dlc.nsp".to_string()];
        let avant = serde_json::json!([{ "path": "D:\\J\\dlc.nsp", "dlc_nca_list": [{ "path": "/vieux.nca", "title_id": 7u64, "is_enabled": false }] }]);
        let nos = vec![("d:/j/DLC.nsp".to_string(), vec![("/neuf.nca".to_string(), 7u64), ("/autre.nca".to_string(), 8u64)])];
        let (v, note, refuses) = dlc_voulu(Some(&avant), &nos, &poses, &[], &existe).unwrap();
        assert!(refuses.is_empty());
        assert_eq!(v.as_array().unwrap().len(), 1, "le même fichier (casse et barres différentes) n'est pas ajouté deux fois");
        assert_eq!(v[0]["dlc_nca_list"][0], serde_json::json!({ "path": "/neuf.nca", "title_id": 7u64, "is_enabled": false }));
        assert_eq!(v[0]["dlc_nca_list"][1]["is_enabled"], true);
        assert_eq!(note, poses);
        assert!(dlc_voulu(Some(&serde_json::json!({})), &nos, &poses, &[], &existe).is_err());
        // Le dlc.json effacé en entier (pas un retrait fait dans Ryubing, qui écrit `[]`) : reposé.
        let (v, _, refuses) = dlc_voulu(None, &nos, &poses, &[], &existe).unwrap();
        assert_eq!((v.as_array().unwrap().len(), refuses.len()), (1, 0));
        // Un refus noté reste un refus, même fichier effacé.
        let (v, _, _) = dlc_voulu(None, &nos, &poses, &["D:\\J\\dlc.nsp".to_string()], &existe).unwrap();
        assert_eq!(v, serde_json::json!([]));
        // updates.json : `selected` écrit sous la forme DÉJÀ présente dans `paths` (Ryubing compare à l'identique).
        let avant = serde_json::json!({ "selected": "", "paths": ["D:\\J\\Maj.nsp"] });
        let (v, choix) = updates_voulu(Some(&avant), Some("d:/j/maj.nsp"), None, &|_: &str| false);
        assert_eq!(choix, ChoixMaj::DeFrogtend, "« aucune » sans fichier existant n'est pas un choix");
        assert_eq!(v.unwrap(), serde_json::json!({ "selected": "D:\\J\\Maj.nsp", "paths": ["D:\\J\\Maj.nsp"] }));
        // Pas de mise à jour trouvée : rien n'est écrit.
        assert_eq!(updates_voulu(None, None, None, &existe), (None, ChoixMaj::DeFrogtend));
    }
}
