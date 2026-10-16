//! Les fichiers système (BIOS) servis par Firehouse (contrat 1.9–1.11, besoin 19 ; relevés réels du 16/10 dans
//! `docs/EXEMPLES-API-JEUX-V1.md`). Le conseil de la maison (`GET /emulateurs?plateforme=`) dit, pour chaque fichier,
//! où il va (`racine` + `chemin`), s'il est obligatoire, s'il est servi (`disponible`, `url`, `sha1`). Frogtend pose
//! ceux qui sont obligatoires ET servis, après un seul accord (« Prêt à jouer ») :
//! - jamais un fichier déjà présent n'est remplacé (celui de la personne est gardé, même différent) ;
//! - l'empreinte SHA-1 est vérifiée AVANT la pose ; rien de faux n'arrive dans le dossier de l'émulateur ;
//! - seulement dans le dossier de l'émulateur, ou le dossier `system` de RetroArch (son propre réglage) ;
//! - les clés (`coffre`) ne sont jamais servies : Frogtend dit qu'elles manquent ; les firmwares à installer par
//!   l'émulateur (`installation`) sont traités à part.

use crate::erreurs::{Erreur, Resultat};
use crate::source::Source;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

/// Un fichier système du conseil (forme réelle relevée le 16/10).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BiosConseil {
    pub fichier: String,
    #[serde(rename = "type")]
    pub sorte: String,
    pub racine: String,
    #[serde(default, deserialize_with = "texte_ou_nul")]
    pub chemin: String,
    #[serde(default)]
    pub facultatif: bool,
    #[serde(default)]
    pub coffre: bool,
    #[serde(default)]
    pub disponible: bool,
    pub sha1: Option<String>,
    pub taille: Option<u64>,
    pub url: Option<String>,
    #[serde(default, deserialize_with = "texte_ou_nul")]
    pub note: String,
}

/// Un texte absent ou `null` = vide.
fn texte_ou_nul<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(d)?.unwrap_or_default())
}

/// Lit le `bios` d'un conseil élément par élément : absent ou `null` = rien à poser ; un élément illisible ne fait
/// pas échouer les autres (il est dit dans le bilan).
pub fn lire_bios(valeur: &serde_json::Value, bilan: &mut Bilan) -> Vec<BiosConseil> {
    let mut l = Vec::new();
    for v in valeur.as_array().into_iter().flatten() {
        match serde_json::from_value::<BiosConseil>(v.clone()) {
            Ok(b) => l.push(b),
            Err(_) => bilan.echecs.push(format!("{} : description illisible", v["fichier"].as_str().unwrap_or("un fichier système"))),
        }
    }
    l
}

/// Ce que Frogtend a fait des fichiers système d'un conseil.
#[derive(Debug, Default, Serialize, PartialEq)]
pub struct Bilan {
    /// Posés maintenant (vérifiés).
    pub poses: Vec<String>,
    /// Déjà là (gardés tels quels).
    pub deja: Vec<String>,
    /// Obligatoires mais pas servis par Firehouse : à poser soi-même (la note dit lesquels).
    pub a_poser: Vec<String>,
    /// Clés (prod.keys…) : jamais servies, personne ne peut les fournir.
    pub cles: Vec<String>,
    /// Firmwares à installer par l'émulateur (PS3UPDAT.PUP, firmware Switch) : traités à part.
    pub firmwares: Vec<String>,
    /// Ceux qui n'ont pas pu être posés (réseau, empreinte…), avec la raison.
    pub echecs: Vec<String>,
}

/// L'empreinte SHA-1 d'un fichier (celle des BIOS servis).
pub fn sha1_fichier(chemin: &Path) -> Resultat<String> {
    let mut f = std::fs::File::open(chemin)?;
    let mut h = Sha1::new();
    let mut tampon = vec![0u8; 256 * 1024];
    loop {
        let n = f.read(&mut tampon)?;
        if n == 0 {
            break;
        }
        h.update(&tampon[..n]);
    }
    Ok(h.finalize().iter().map(|o| format!("{o:02x}")).collect())
}

/// Un chemin relatif SÛR (venu du serveur) : seulement des noms, jamais `..`, une racine ni un lecteur.
pub fn relatif_sur(chemin: &str) -> Option<PathBuf> {
    let c = chemin.trim().replace('\\', "/");
    let mut p = PathBuf::new();
    for part in c.split('/').filter(|s| !s.is_empty()) {
        if part.contains(':') {
            return None;
        }
        let mut comps = Path::new(part).components();
        match (comps.next(), comps.next()) {
            (Some(Component::Normal(n)), None) => p.push(n),
            _ => return None,
        }
    }
    Some(p)
}

/// Où va un fichier du conseil (un fichier : son chemin complet ; une archive : le dossier où l'extraire). `None` :
/// une racine que Frogtend ne pose pas lui-même, ou un chemin refusé.
pub fn destination(b: &BiosConseil, dossier_emulateur: &Path, system_retroarch: Option<&Path>) -> Option<PathBuf> {
    let base = match b.racine.as_str() {
        "system_retroarch" => system_retroarch?.to_path_buf(),
        "dossier_emulateur" => dossier_emulateur.to_path_buf(),
        _ => return None,
    };
    match b.sorte.as_str() {
        // Une archive s'extrait DANS racine/chemin (chemin vide = la racine même).
        "archive" => Some(base.join(relatif_sur(&b.chemin)?)),
        "fichier" => {
            let c = b.chemin.trim();
            let rel = if c.is_empty() {
                relatif_sur(&b.fichier)?
            } else if c.ends_with('/') || c.ends_with('\\') {
                // « bios/ » : un dossier ; le fichier va dedans, sous son nom.
                relatif_sur(&format!("{c}{}", b.fichier))?
            } else {
                relatif_sur(c)?
            };
            (!rel.as_os_str().is_empty()).then(|| base.join(rel))
        }
        _ => None,
    }
}

/// Un fichier que Frogtend sait poser : servi, de type `fichier` ou `archive`, avec empreinte et route (même critère
/// que l'interface, `biosATelecharger`).
pub fn posable(b: &BiosConseil) -> bool {
    b.disponible && matches!(b.sorte.as_str(), "fichier" | "archive") && b.sha1.is_some() && b.url.is_some()
}

/// Les fichiers OBLIGATOIRES du conseil, rangés : à poser (servis), puis le reste dans le bilan.
pub fn trier(bios: &[BiosConseil], bilan: &mut Bilan) -> Vec<BiosConseil> {
    let mut a_poser = Vec::new();
    for b in bios.iter().filter(|b| !b.facultatif) {
        if b.coffre {
            bilan.cles.push(b.fichier.clone());
        } else if b.sorte == "installation" {
            bilan.firmwares.push(b.fichier.clone());
        } else if posable(b) {
            a_poser.push(b.clone());
        } else {
            bilan.a_poser.push(if b.note.trim().is_empty() { b.fichier.clone() } else { format!("{} ({})", b.fichier, b.note.trim()) });
        }
    }
    a_poser
}

/// Déplace un fichier vérifié à sa place. Sur un autre disque : copie dans un fichier provisoire À CÔTÉ de la place
/// finale, puis renommage — une copie coupée (disque plein, débranché) ne laisse jamais un fichier tronqué que Frogtend
/// prendrait ensuite pour « déjà là ».
fn deplacer(de: &Path, vers: &Path) -> Resultat<()> {
    if let Some(p) = vers.parent() {
        std::fs::create_dir_all(p)?;
    }
    if std::fs::rename(de, vers).is_ok() {
        return Ok(());
    }
    let provisoire = vers.with_extension(format!("{}.frogtend-part", vers.extension().and_then(|e| e.to_str()).unwrap_or("")));
    let r = std::fs::copy(de, &provisoire).and_then(|_| std::fs::rename(&provisoire, vers));
    if let Err(e) = r {
        let _ = std::fs::remove_file(&provisoire);
        return Err(e.into());
    }
    let _ = std::fs::remove_file(de);
    Ok(())
}

/// Pose UN fichier servi (déjà trié) : téléchargé dans `<dossier de l'émulateur>\.telechargements`, SHA-1 vérifié,
/// puis posé (un fichier) ou extrait sans rien remplacer (une archive). Rend `true` s'il a posé quelque chose.
pub async fn poser_un(source: &Source, b: &BiosConseil, cible: &Path, dossier_emulateur: &Path) -> Resultat<bool> {
    let sha1 = b.sha1.clone().unwrap_or_default().to_lowercase();
    if !sha1.chars().all(|c| c.is_ascii_hexdigit()) || sha1.len() != 40 {
        return Err(Erreur::Serveur("Empreinte de BIOS invalide.".into()));
    }
    if b.sorte == "fichier" && cible.exists() {
        return Ok(false); // déjà là : jamais remplacé
    }
    let route = b
        .url
        .as_deref()
        .and_then(|u| u.strip_prefix(crate::firehouse::PREFIXE))
        .ok_or_else(|| Erreur::Serveur("Firehouse n'a pas donné de route pour ce fichier système.".into()))?
        .to_string();
    let attente = dossier_emulateur.join(".telechargements");
    let paquet = attente.join(format!("bios-{sha1}.bin"));
    if let Err(e) = source.telecharger_route(&route, &paquet, &|_, _| {}).await {
        // Un dossier vide seulement (un `.part` à reprendre reste) : jamais de dossier oublié.
        let _ = std::fs::remove_dir(&attente);
        return Err(e);
    }
    let a_part = attente.join(format!("bios-{sha1}"));
    let r = (|| -> Resultat<bool> {
        if sha1_fichier(&paquet)? != sha1 {
            return Err(Erreur::Conflit(format!("{} est arrivé abîmé (empreinte différente) : rien n'est posé.", b.fichier)));
        }
        if b.sorte == "fichier" {
            deplacer(&paquet, cible)?;
            return Ok(true);
        }
        // Une archive : extraite à part, puis chaque fichier posé s'il n'existe pas déjà.
        crate::installation::decompresser(&paquet, crate::installation::Format::Zip, &a_part)?;
        let mut pose = false;
        for rel in crate::installation::fichiers_de(&a_part)? {
            let f = a_part.join(&rel);
            let vers = cible.join(&rel);
            if !vers.exists() {
                deplacer(&f, &vers)?;
                pose = true;
            }
        }
        Ok(pose)
    })();
    // Le dossier d'extraction (nommé par l'empreinte vérifiée : 40 hexadécimaux) et le paquet, sur TOUS les chemins.
    let _ = std::fs::remove_dir_all(&a_part);
    let _ = std::fs::remove_file(&paquet);
    let _ = std::fs::remove_dir(&attente);
    r
}

/// Pose les fichiers système obligatoires et servis d'un conseil pour l'émulateur installé dans `dossier_emulateur`
/// (`retroarch` : vers son dossier `system`). Un échec n'arrête pas les autres ; tout est dit dans le bilan.
pub async fn poser(source: &Source, bios: &[BiosConseil], dossier_emulateur: &Path, retroarch: bool) -> Bilan {
    let mut bilan = Bilan::default();
    let system = retroarch.then(|| crate::emulateurs::dossier_bios_retroarch(dossier_emulateur));
    for b in trier(bios, &mut bilan) {
        let Some(cible) = destination(&b, dossier_emulateur, system.as_deref()) else {
            bilan.a_poser.push(b.fichier.clone());
            continue;
        };
        match poser_un(source, &b, &cible, dossier_emulateur).await {
            Ok(true) => bilan.poses.push(b.fichier.clone()),
            Ok(false) => bilan.deja.push(b.fichier.clone()),
            // 404 = inconnu OU pas (encore) servi (contrat 1.9) : à poser soi-même, ce n'est pas une panne.
            Err(Erreur::Introuvable(_)) => bilan.a_poser.push(format!("{} (pas encore servi par Firehouse)", b.fichier)),
            Err(e) => bilan.echecs.push(format!("{} : {e}", b.fichier)),
        }
    }
    bilan
}

/// Le firmware à installer par l'émulateur (`type: installation`) que Firehouse sert pour ce conseil, s'il y en a un
/// (PS3UPDAT.PUP pour RPCS3 ; firmware-switch.zip pour Eden).
pub fn firmware_servi(bios: &[BiosConseil]) -> Option<&BiosConseil> {
    bios.iter().find(|b| b.sorte == "installation" && b.disponible && b.sha1.as_deref().is_some_and(|s| s.len() == 40) && b.url.is_some())
}

/// La version annoncée par la note d'un firmware servi (« firmware 4.93 récupéré chez Sony… » → « 4.93 »).
pub fn version_dans_note(note: &str) -> Option<String> {
    let bas = note.to_lowercase();
    let i = ["firmware ", "micrologiciel "].iter().find_map(|m| bas.find(m).map(|i| i + m.len()))?;
    let v: String = bas[i..].chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    let v = v.trim_end_matches('.');
    (v.contains('.') && v.split('.').all(|p| !p.is_empty())).then(|| v.to_string())
}

/// Télécharge un firmware servi dans `<dossier>\.telechargements\firmware-<sha1>.<ext>` (reprise, progression) et
/// vérifie son SHA-1. Rend le chemin du fichier vérifié (à retirer après l'installation).
pub async fn telecharger_firmware(
    source: &Source,
    f: &BiosConseil,
    dossier: &Path,
    progres: &(dyn Fn(u64, Option<u64>) + Send + Sync),
) -> Resultat<PathBuf> {
    let sha1 = f.sha1.clone().unwrap_or_default().to_lowercase();
    if sha1.len() != 40 || !sha1.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(Erreur::Serveur("Empreinte du firmware invalide.".into()));
    }
    let ext = Path::new(&f.fichier).extension().and_then(|e| e.to_str()).filter(|e| e.chars().all(|c| c.is_ascii_alphanumeric())).unwrap_or("bin");
    let route = f
        .url
        .as_deref()
        .and_then(|u| u.strip_prefix(crate::firehouse::PREFIXE))
        .ok_or_else(|| Erreur::Serveur("Firehouse n'a pas donné de route pour ce firmware.".into()))?
        .to_string();
    let fichier = dossier.join(".telechargements").join(format!("firmware-{sha1}.{ext}"));
    source.telecharger_route(&route, &fichier, progres).await?;
    if sha1_fichier(&fichier)? != sha1 {
        let _ = std::fs::remove_file(&fichier);
        return Err(Erreur::Conflit(format!("{} est arrivé abîmé (empreinte différente) : rien n'est installé.", f.fichier)));
    }
    Ok(fichier)
}

/// PCSX2 (contrat 1.11) : son BIOS par défaut réglé dans `inis\PCSX2.ini` (clé par clé, le reste gardé), seulement
/// si la personne n'en a pas déjà choisi un. Jamais les `.nvm`.
pub fn regler_pcsx2(emulateur: &Path, bios: &[BiosConseil]) -> Resultat<bool> {
    let Some(defaut) = bios.iter().find(|b| !b.facultatif && b.racine == "dossier_emulateur" && b.sorte == "fichier") else {
        return Ok(false);
    };
    let Some(pose) = destination(defaut, emulateur, None).filter(|p| p.is_file()) else { return Ok(false) };
    let ini = emulateur.join("inis").join("PCSX2.ini");
    let mut texte = match std::fs::read_to_string(&ini) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e.into()),
    };
    let lire = |t: &str, s: &str, c: &str| crate::manettes::lire_ini(t, s, c).into_iter().find(|v| !v.trim().is_empty());
    if lire(&texte, "Filenames", "BIOS").is_some() {
        return Ok(false); // un BIOS déjà choisi : on n'y touche pas
    }
    // Le dossier des BIOS de PCSX2 : s'il est réglé AILLEURS que là où Frogtend a posé le fichier, on ne règle rien
    // (PCSX2 ne le trouverait pas) ; sinon « bios ».
    let dossier_pose = pose.parent().map(Path::to_path_buf).unwrap_or_default();
    match lire(&texte, "Folders", "Bios") {
        Some(d) => {
            let d = Path::new(d.trim());
            let reel = if d.is_absolute() { d.to_path_buf() } else { emulateur.join(d) };
            if reel != dossier_pose {
                return Ok(false);
            }
        }
        None => texte = crate::emulateurs_profils::ecrire_ini(&texte, "Folders", &[("Bios", vec!["bios".into()])]),
    }
    use crate::emulateurs_profils::ecrire_ini;
    // PCSX2 lit la version de son fichier : ajoutée si elle manque (contrat 1.11), jamais changée.
    if lire(&texte, "UI", "SettingsVersion").is_none() {
        texte = ecrire_ini(&texte, "UI", &[("SettingsVersion", vec!["1".into()])]);
    }
    texte = ecrire_ini(&texte, "Filenames", &[("BIOS", vec![defaut.fichier.clone()])]);
    // Le démarrage rapide : seulement si la personne n'a rien choisi.
    if lire(&texte, "EmuCore", "EnableFastBoot").is_none() {
        texte = ecrire_ini(&texte, "EmuCore", &[("EnableFastBoot", vec!["true".into()])]);
    }
    // Une seule écriture (copie à l'abri de l'état d'AVANT, s'il existait).
    crate::emulateurs_profils::remplacer_config(emulateur, &ini, &texte)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn b(v: serde_json::Value) -> BiosConseil {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn un_chemin_venu_du_serveur_reste_dans_son_dossier() {
        assert_eq!(relatif_sur("dc/naomi.zip"), Some(PathBuf::from("dc").join("naomi.zip")));
        assert_eq!(relatif_sur("bios\\ps2.bin"), Some(PathBuf::from("bios").join("ps2.bin")));
        assert_eq!(relatif_sur(""), Some(PathBuf::new()));
        for piege in ["../x", "a/../../x", "C:x", "C:\\Windows\\x", "\\\\serveur\\x", "a/./b"] {
            let r = relatif_sur(piege);
            assert!(r.is_none() || !r.unwrap().components().any(|c| !matches!(c, Component::Normal(_))), "{piege}");
        }
        assert!(relatif_sur("../x").is_none() && relatif_sur("C:x").is_none());
    }

    #[test]
    fn chaque_fichier_va_a_sa_place_d_apres_le_conseil_reel() {
        let emu = Path::new("E:\\Emu\\PCSX2");
        let sys = Path::new("E:\\Emu\\RetroArch\\system");
        // Relevés réels du 16/10 : Naomi (system_retroarch), PS2 (dossier_emulateur), MSX (archive).
        let naomi = b(json!({"fichier": "naomi.zip", "type": "fichier", "racine": "system_retroarch", "chemin": "dc/naomi.zip", "disponible": true, "sha1": "c96711c01c0158f161791d6fbe75d88329e8ac0a", "url": "/api/jeux/v1/bios/c96711c01c0158f161791d6fbe75d88329e8ac0a?nom=naomi.zip", "coffre": false}));
        assert_eq!(destination(&naomi, emu, Some(sys)), Some(sys.join("dc").join("naomi.zip")));
        assert_eq!(destination(&naomi, emu, None), None, "pas de RetroArch : pas de dossier system");
        let ps2 = b(json!({"fichier": "ps2-0250e-20100415.bin", "type": "fichier", "racine": "dossier_emulateur", "chemin": "bios/ps2-0250e-20100415.bin", "facultatif": false, "disponible": true, "sha1": "b9cb5775af29cd4d1ec5521e8231f8b6636e2e44", "url": "/api/jeux/v1/bios/x"}));
        assert_eq!(destination(&ps2, emu, None), Some(emu.join("bios").join("ps2-0250e-20100415.bin")));
        let msx = b(json!({"fichier": "blueMSX.zip", "type": "archive", "racine": "system_retroarch", "disponible": true, "sha1": "bda0dec1a075f06d36427c1c24397d5a1793fba5", "url": "/api/jeux/v1/bios/y"}));
        assert_eq!(destination(&msx, emu, Some(sys)), Some(sys.to_path_buf()), "chemin vide = la racine même");
        let piege = b(json!({"fichier": "x", "type": "fichier", "racine": "dossier_emulateur", "chemin": "../../Windows/x"}));
        assert_eq!(destination(&piege, emu, None), None);
    }

    #[test]
    fn le_conseil_se_trie_obligatoires_servis_cles_firmwares() {
        // Switch réel : clé au coffre + firmware à installer ; Xbox réel : motif non servi (fichiers: null) ; PS2 :
        // le facultatif n'est pas posé.
        let bios = vec![
            b(json!({"fichier": "prod.keys", "type": "fichier", "racine": "dossier_emulateur", "chemin": "prod.keys", "disponible": false, "coffre": true})),
            b(json!({"fichier": "firmware-switch.zip", "type": "installation", "racine": "installation_par_l_emulateur", "disponible": true, "sha1": "f85244d668ae8cd20a9b140fe4a53b16d8f93900", "url": "/api/jeux/v1/bios/f8"})),
            b(json!({"fichier": "*.bin", "type": "motif", "racine": "dossier_emulateur", "chemin": "*.bin", "disponible": false, "fichiers": null, "note": "BIOS flash (Complex 4627)"})),
            b(json!({"fichier": "ps2-0230a-20080220.bin", "type": "fichier", "racine": "dossier_emulateur", "chemin": "bios/ps2-0230a-20080220.bin", "facultatif": true, "disponible": true, "sha1": "f9229fe159d0353b9f0632f3fdc66819c9030458", "url": "/api/jeux/v1/bios/f9"})),
            b(json!({"fichier": "naomi.zip", "type": "fichier", "racine": "system_retroarch", "chemin": "dc/naomi.zip", "disponible": true, "sha1": "c96711c01c0158f161791d6fbe75d88329e8ac0a", "url": "/api/jeux/v1/bios/c9"})),
        ];
        let mut bilan = Bilan::default();
        let a = trier(&bios, &mut bilan);
        assert_eq!(a.iter().map(|x| x.fichier.as_str()).collect::<Vec<_>>(), ["naomi.zip"]);
        assert_eq!(bilan.cles, ["prod.keys"]);
        assert_eq!(bilan.firmwares, ["firmware-switch.zip"]);
        assert_eq!(bilan.a_poser, ["*.bin (BIOS flash (Complex 4627))"]);
    }

    #[tokio::test]
    async fn un_bios_servi_est_verifie_puis_pose_et_jamais_ne_remplace() {
        use httpmock::prelude::*;
        use std::io::Write;
        let contenu = b"BIOS PS2 de test".to_vec();
        let sha = {
            let mut h = Sha1::new();
            h.update(&contenu);
            h.finalize().iter().map(|o| format!("{o:02x}")).collect::<String>()
        };
        let mut zip_octets = Vec::new();
        {
            let mut z = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_octets));
            z.start_file("Machines/MSX2/x.rom", zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(b"rom").unwrap();
            z.start_file("Databases/deja.xml", zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(b"nouveau").unwrap();
            z.finish().unwrap();
        }
        let sha_zip = {
            let mut h = Sha1::new();
            h.update(&zip_octets);
            h.finalize().iter().map(|o| format!("{o:02x}")).collect::<String>()
        };
        let serveur = MockServer::start();
        let route = serveur.mock(|w, r| {
            w.method(GET).path(format!("/api/jeux/v1/bios/{sha}"));
            r.status(200).body(contenu.clone());
        });
        serveur.mock(|w, r| {
            w.method(GET).path(format!("/api/jeux/v1/bios/{sha_zip}"));
            r.status(200).body(zip_octets.clone());
        });
        let faux = serveur.mock(|w, r| {
            w.method(GET).path("/api/jeux/v1/bios/0000000000000000000000000000000000000000");
            r.status(200).body(b"pas le bon".to_vec());
        });
        let source = Source::Firehouse(crate::firehouse::Client::nouveau(&serveur.base_url(), "j").unwrap());
        let d = tempfile::tempdir().unwrap();
        let emu = d.path().join("PCSX2");
        let ra = d.path().join("RetroArch");
        std::fs::create_dir_all(ra.join("system").join("Databases")).unwrap();
        std::fs::write(ra.join("system").join("Databases").join("deja.xml"), b"le mien").unwrap();
        let ps2 = |sha1: &str| b(json!({"fichier": "ps2-0250e-20100415.bin", "type": "fichier", "racine": "dossier_emulateur", "chemin": "bios/ps2-0250e-20100415.bin", "disponible": true, "sha1": sha1, "url": format!("/api/jeux/v1/bios/{sha1}?nom=ps2-0250e-20100415.bin")}));
        // Empreinte fausse : rien n'est posé, aucun reste.
        let bilan = poser(&source, &[ps2("0000000000000000000000000000000000000000")], &emu, false).await;
        assert_eq!(bilan.echecs.len(), 1);
        assert!(!emu.join("bios").exists());
        assert!(!emu.join(".telechargements").exists());
        faux.assert_hits(1);
        // Juste : posé, octet pour octet ; une deuxième fois : déjà là, rien téléchargé.
        let bilan = poser(&source, &[ps2(&sha)], &emu, false).await;
        assert_eq!(bilan.poses, ["ps2-0250e-20100415.bin"]);
        assert_eq!(std::fs::read(emu.join("bios").join("ps2-0250e-20100415.bin")).unwrap(), contenu);
        assert_eq!(poser(&source, &[ps2(&sha)], &emu, false).await.deja, ["ps2-0250e-20100415.bin"]);
        route.assert_hits(1);
        // Une archive (blueMSX) s'extrait dans system\ sans rien remplacer.
        let msx = b(json!({"fichier": "blueMSX.zip", "type": "archive", "racine": "system_retroarch", "disponible": true, "sha1": sha_zip, "url": format!("/api/jeux/v1/bios/{sha_zip}?nom=blueMSX.zip")}));
        let bilan = poser(&source, &[msx], &ra, true).await;
        assert_eq!(bilan.poses, ["blueMSX.zip"]);
        assert_eq!(std::fs::read(ra.join("system").join("Machines").join("MSX2").join("x.rom")).unwrap(), b"rom");
        assert_eq!(std::fs::read(ra.join("system").join("Databases").join("deja.xml")).unwrap(), b"le mien");
        assert!(!ra.join(".telechargements").exists());
    }

    #[test]
    fn la_reponse_reelle_ps2_se_lit_et_un_element_mal_forme_ne_bloque_pas_les_autres() {
        // Réponse COMPLÈTE réelle du 16/10 (EXEMPLES-API-JEUX-V1.md), champ `bios` du conseil PS2.
        let reel = json!([
            {"fichier": "ps2-0250e-20100415.bin", "type": "fichier", "racine": "dossier_emulateur", "chemin": "bios/ps2-0250e-20100415.bin", "facultatif": false, "coffre": false, "note": "PAR DÉFAUT (Europe, v2.50) : PCSX2.ini [Filenames] BIOS = ps2-0250e-20100415.bin ; [EmuCore] EnableFastBoot = true", "disponible": true, "sha1": "b9cb5775af29cd4d1ec5521e8231f8b6636e2e44", "taille": 4194304, "url": "/api/jeux/v1/bios/b9cb5775af29cd4d1ec5521e8231f8b6636e2e44?nom=ps2-0250e-20100415.bin"},
            {"fichier": "ps2-0230a-20080220.bin", "type": "fichier", "racine": "dossier_emulateur", "chemin": "bios/ps2-0230a-20080220.bin", "facultatif": true, "coffre": false, "note": "USA (v2.30), facultatif", "disponible": true, "sha1": "f9229fe159d0353b9f0632f3fdc66819c9030458", "taille": 4194304, "url": "/api/jeux/v1/bios/f9229fe159d0353b9f0632f3fdc66819c9030458?nom=ps2-0230a-20080220.bin"}
        ]);
        let mut bilan = Bilan::default();
        let l = lire_bios(&reel, &mut bilan);
        assert_eq!(l.len(), 2);
        assert!(bilan.echecs.is_empty());
        assert_eq!(trier(&l, &mut bilan).iter().map(|b| b.fichier.as_str()).collect::<Vec<_>>(), ["ps2-0250e-20100415.bin"]);
        // `null` pour un texte : lu comme vide ; un élément sans `type` : dit, les autres gardés ; pas de `bios` : rien.
        let mal = json!([{"fichier": "a.bin", "type": "fichier", "racine": "dossier_emulateur", "chemin": null, "note": null}, {"fichier": "b.bin"}]);
        let mut bilan = Bilan::default();
        assert_eq!(lire_bios(&mal, &mut bilan).len(), 1);
        assert_eq!(bilan.echecs, ["b.bin : description illisible"]);
        assert!(lire_bios(&serde_json::Value::Null, &mut Bilan::default()).is_empty());
    }

    #[tokio::test]
    async fn un_bios_pas_encore_servi_est_a_poser_soi_meme_pas_une_panne() {
        use httpmock::prelude::*;
        let serveur = MockServer::start();
        serveur.mock(|w, r| {
            w.method(GET).path_contains("/api/jeux/v1/bios/");
            r.status(404).json_body(json!({"ok": false}));
        });
        let source = Source::Firehouse(crate::firehouse::Client::nouveau(&serveur.base_url(), "j").unwrap());
        let d = tempfile::tempdir().unwrap();
        let naomi = b(json!({"fichier": "naomi.zip", "type": "fichier", "racine": "dossier_emulateur", "chemin": "dc/naomi.zip", "disponible": true, "sha1": "c96711c01c0158f161791d6fbe75d88329e8ac0a", "url": "/api/jeux/v1/bios/c96711c01c0158f161791d6fbe75d88329e8ac0a?nom=naomi.zip"}));
        let bilan = poser(&source, &[naomi], d.path(), false).await;
        assert!(bilan.echecs.is_empty());
        assert_eq!(bilan.a_poser, ["naomi.zip (pas encore servi par Firehouse)"]);
        assert!(!d.path().join("dc").exists());
    }

    #[tokio::test]
    async fn le_firmware_servi_est_verifie_avant_d_etre_rendu() {
        use httpmock::prelude::*;
        let contenu = b"PUP de test".to_vec();
        let sha = {
            let mut h = Sha1::new();
            h.update(&contenu);
            h.finalize().iter().map(|o| format!("{o:02x}")).collect::<String>()
        };
        let serveur = MockServer::start();
        serveur.mock(|w, r| {
            w.method(GET).path(format!("/api/jeux/v1/bios/{sha}"));
            r.status(200).body(contenu.clone());
        });
        let source = Source::Firehouse(crate::firehouse::Client::nouveau(&serveur.base_url(), "j").unwrap());
        // Relevé réel du 16/10 (PS3) : type installation, chemin vide.
        let bios = vec![
            b(json!({"fichier": "prod.keys", "type": "fichier", "racine": "dossier_emulateur", "coffre": true, "disponible": false})),
            b(json!({"fichier": "PS3UPDAT.PUP", "type": "installation", "racine": "installation_par_l_emulateur", "chemin": "", "disponible": true, "sha1": sha, "taille": 11, "url": format!("/api/jeux/v1/bios/{sha}?nom=PS3UPDAT.PUP"), "note": "firmware 4.93"})),
        ];
        let f = firmware_servi(&bios).unwrap();
        assert_eq!(f.fichier, "PS3UPDAT.PUP");
        let d = tempfile::tempdir().unwrap();
        let p = telecharger_firmware(&source, f, d.path(), &|_, _| {}).await.unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), contenu);
        assert!(p.to_string_lossy().ends_with(".PUP"));
        // Empreinte annoncée fausse : refusé, rien ne reste.
        let faux = BiosConseil { sha1: Some("0".repeat(40)), url: Some(format!("/api/jeux/v1/bios/{sha}?nom=x")), ..f.clone() };
        assert!(matches!(telecharger_firmware(&source, &faux, d.path(), &|_, _| {}).await, Err(Erreur::Conflit(_))));
        assert!(!d.path().join(".telechargements").join(format!("firmware-{}.PUP", "0".repeat(40))).exists());
        assert!(firmware_servi(&bios[..1]).is_none());
    }

    #[test]
    fn la_version_d_un_firmware_se_lit_dans_sa_note() {
        // Note réelle du 16/10.
        assert_eq!(version_dans_note("firmware 4.93 récupéré chez Sony (04/10) — installé par le menu de RPCS3").as_deref(), Some("4.93"));
        assert_eq!(version_dans_note("Micrologiciel 18.1.0 (zip des NCA)").as_deref(), Some("18.1.0"));
        assert_eq!(version_dans_note("clés : dans le coffre de Seb").as_deref(), None);
        assert_eq!(version_dans_note("firmware récent").as_deref(), None);
    }

    #[test]
    fn pcsx2_recoit_son_bios_par_defaut_sauf_choix_de_la_personne() {
        let d = tempfile::tempdir().unwrap();
        let emu = d.path();
        let bios = vec![b(json!({"fichier": "ps2-0250e-20100415.bin", "type": "fichier", "racine": "dossier_emulateur", "chemin": "bios/ps2-0250e-20100415.bin", "disponible": true}))];
        assert!(!regler_pcsx2(emu, &bios).unwrap(), "le BIOS n'est pas encore là : rien");
        std::fs::create_dir_all(emu.join("bios")).unwrap();
        std::fs::write(emu.join("bios").join("ps2-0250e-20100415.bin"), b"x").unwrap();
        assert!(regler_pcsx2(emu, &bios).unwrap());
        let t = std::fs::read_to_string(emu.join("inis").join("PCSX2.ini")).unwrap();
        assert_eq!(crate::manettes::lire_ini(&t, "Filenames", "BIOS"), ["ps2-0250e-20100415.bin"]);
        assert_eq!(crate::manettes::lire_ini(&t, "Folders", "Bios"), ["bios"]);
        assert_eq!(crate::manettes::lire_ini(&t, "EmuCore", "EnableFastBoot"), ["true"]);
        assert_eq!(crate::manettes::lire_ini(&t, "UI", "SettingsVersion"), ["1"]);
        assert!(!emu.join(".frogtend-sauvegardes").exists(), "rien à mettre à l'abri : le fichier n'existait pas");
        // Un BIOS choisi par la personne : jamais remplacé.
        std::fs::write(emu.join("inis").join("PCSX2.ini"), "[Filenames]\r\nBIOS = mon-bios.bin\r\n").unwrap();
        assert!(!regler_pcsx2(emu, &bios).unwrap());
        assert!(std::fs::read_to_string(emu.join("inis").join("PCSX2.ini")).unwrap().contains("mon-bios.bin"));
        // Démarrage rapide refusé par la personne : gardé ; dossier des BIOS réglé ailleurs : rien n'est réglé.
        std::fs::write(emu.join("inis").join("PCSX2.ini"), "[EmuCore]\r\nEnableFastBoot = false\r\n").unwrap();
        assert!(regler_pcsx2(emu, &bios).unwrap());
        let t = std::fs::read_to_string(emu.join("inis").join("PCSX2.ini")).unwrap();
        assert_eq!(crate::manettes::lire_ini(&t, "EmuCore", "EnableFastBoot"), ["false"]);
        std::fs::write(emu.join("inis").join("PCSX2.ini"), "[Folders]\r\nBios = D:\\BIOS\\PS2\r\n").unwrap();
        assert!(!regler_pcsx2(emu, &bios).unwrap());
    }
}
