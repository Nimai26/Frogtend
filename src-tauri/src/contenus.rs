//! Les contenus additionnels (DLC, avatars, thèmes, mises à jour) — règle de Seb (03/10) : ils sont DÉTECTÉS,
//! GÉRÉS, PROPOSÉS, et INSTALLÉS seulement si la personne qui utilise Frogtend dit oui (sur SON PC).
//!
//! PS3 d'abord (relevé dans les sources de RPCS3, 03/10) :
//! - un `.pkg` commence par 0x7F « PKG » et porte son « content ID » à l'octet 0x30 (36 caractères,
//!   ex. `EP0102-BLES01227_00-DLCS120000000000`, où `BLES01227` est l'identifiant du jeu) ;
//! - une licence `.rap` porte le même content ID dans son nom ;
//! - `rpcs3 --headless --installpkg <fichier>` installe un .pkg puis se ferme (contenu commun à tous les comptes) ;
//! - une licence va dans `dev_hdd0\home\<compte>\exdata\<content id>.rap` : PAR COMPTE RPCS3, donc dans le compte du
//!   profil qui installe.
//! Les .pkg et .rap peuvent être dans des zips (comme chez Seb) : seul ce qu'on installe est extrait, dans le dossier
//! de travail de Frogtend, puis retiré.

use crate::erreurs::{Erreur, Resultat};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};

/// Un contenu additionnel trouvé sur le disque.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Contenu {
    /// Le content ID (unique) : `EP0102-BLES01227_00-DLCS120000000000`.
    pub id: String,
    /// L'identifiant du jeu auquel il appartient (`BLES01227`).
    pub jeu: String,
    /// Un nom lisible (celui du zip ou du fichier, sans extension).
    pub nom: String,
    /// `dlc`, `avatar`, `theme`, `maj` ou `autre` (d'après les étiquettes du nom).
    pub genre: String,
    /// Le .pkg : fichier, et son nom DANS le zip s'il est zippé. Aucun : une licence seule (servie sans son .pkg).
    pub pkg: Option<(String, Option<String>)>,
    /// La licence .rap, s'il y en a une (même forme).
    pub rap: Option<(String, Option<String>)>,
    pub taille: u64,
}

/// Le content ID d'un en-tête .pkg (les 0x60 premiers octets suffisent).
pub fn content_id(entete: &[u8]) -> Option<String> {
    if entete.len() < 0x54 || &entete[..4] != b"\x7FPKG" {
        return None;
    }
    let brut = &entete[0x30..0x54];
    let id: String = brut.iter().take_while(|b| **b != 0).map(|b| *b as char).collect();
    (id.len() >= 19 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')).then_some(id)
}

/// L'identifiant du jeu dans un content ID : `EP0102-BLES01227_00-…` → `BLES01227`.
pub fn jeu_du_content_id(id: &str) -> Option<String> {
    let milieu = id.split('-').nth(1)?;
    let jeu = milieu.split('_').next()?;
    (jeu.len() == 9).then(|| jeu.to_uppercase())
}

/// Le genre d'après les étiquettes du nom (« (DLC) », « (Avatar) », « (Theme) », « (Update) »…).
pub fn genre_du_nom(nom: &str) -> &'static str {
    let n = nom.to_lowercase();
    if n.contains("(avatar") {
        "avatar"
    } else if n.contains("(theme") || n.contains("(thème") {
        "theme"
    } else if n.contains("(update") || n.contains("(patch") || n.contains("(v0") || n.contains("(mise à jour") {
        "maj"
    } else if n.contains("(dlc") || n.contains("(add-on") {
        "dlc"
    } else {
        "autre"
    }
}

/// La valeur d'une clé d'un PARAM.SFO (« TITLE_ID », « TITLE »…).
pub fn valeur_sfo(sfo: &[u8], cle: &str) -> Option<String> {
    if sfo.len() < 0x14 || &sfo[..4] != b"\0PSF" {
        return None;
    }
    let le32 = |i: usize| sfo.get(i..i + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize);
    let (cles, donnees, n) = (le32(0x08)?, le32(0x0C)?, le32(0x10)?);
    for i in 0..n.min(512) {
        let e = 0x14 + i * 16;
        let decalage_cle = u16::from_le_bytes([*sfo.get(e)?, *sfo.get(e + 1)?]) as usize;
        let longueur = le32(e + 4)?;
        let decalage_donnee = le32(e + 12)?;
        let nom: String = sfo.get(cles + decalage_cle..)?.iter().take_while(|b| **b != 0).map(|b| *b as char).collect();
        if nom == cle {
            let v = sfo.get(donnees + decalage_donnee..donnees + decalage_donnee + longueur)?;
            return Some(v.iter().take_while(|b| **b != 0).map(|b| *b as char).collect());
        }
    }
    None
}

/// Un PARAM.SFO fait quelques centaines d'octets : jamais plus lu (une image abîmée qui annonce 4 Go ne fait rien
/// réserver).
const SFO_MAX: u64 = 64 * 1024;

/// Un PARAM.SFO sur le disque, s'il a une taille raisonnable.
fn lire_sfo(f: &Path) -> Option<Vec<u8>> {
    (std::fs::metadata(f).ok()?.len() <= SFO_MAX).then(|| std::fs::read(f).ok()).flatten()
}

/// Le PARAM.SFO d'un jeu PS3 : un .iso/.chd, ou un dossier de jeu.
fn sfo_ps3(chemin: &Path) -> Option<Vec<u8>> {
    let ext = chemin.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    if ext == "iso" || ext == "chd" {
        let p = crate::disque::ouvrir(chemin).ok()??;
        let (s, t) = p.trouver("PS3_GAME\\PARAM.SFO").ok()??;
        p.lire(s, u64::from(t).min(SFO_MAX) as usize).ok()
    } else {
        let d = if chemin.is_dir() { chemin.to_path_buf() } else { chemin.parent()?.to_path_buf() };
        [d.join("PS3_GAME").join("PARAM.SFO"), d.join("PARAM.SFO"), d.parent()?.join("PARAM.SFO")].iter().find_map(|f| lire_sfo(f))
    }
}

/// L'identifiant d'un jeu PS3 (TITLE_ID de son PARAM.SFO) : un .iso/.chd, ou un dossier de jeu.
pub fn titre_id_ps3(chemin: &Path) -> Option<String> {
    valeur_sfo(&sfo_ps3(chemin)?, "TITLE_ID")
}

/// La version d'un PARAM.SFO comme RPCS3 la lit : APP_VER, sinon VERSION (`System.cpp`, relevé du 19/10).
fn version_sfo(sfo: &[u8]) -> Option<String> {
    valeur_sfo(sfo, "APP_VER").filter(|v| !v.is_empty()).or_else(|| valeur_sfo(sfo, "VERSION"))
}

/// La version d'un jeu PS3 telle que RPCS3 la LANCE (les patchs dépendent de l'exécutable qui tourne vraiment).
/// Règle relevée dans `System.cpp` (19/10) : RPCS3 démarre la mise à jour installée dans `dev_hdd0\game\<numéro>\`
/// seulement si elle a son `USRDIR\EBOOT.BIN` ET un PARAM.SFO du même jeu en catégorie GD — sinon (un simple cache,
/// comme chez Seb pour Asura's Wrath), c'est la version du disque.
pub fn version_ps3(chemin: &Path, rpcs3: &Path, numero: &str) -> Option<String> {
    let dossier_maj = rpcs3.join("dev_hdd0").join("game").join(numero);
    let maj = dossier_maj
        .join("USRDIR")
        .join("EBOOT.BIN")
        .is_file()
        .then(|| lire_sfo(&dossier_maj.join("PARAM.SFO")))
        .flatten()
        .filter(|s| valeur_sfo(s, "TITLE_ID").as_deref() == Some(numero) && valeur_sfo(s, "CATEGORY").as_deref() == Some("GD"));
    let v = match maj {
        Some(s) => version_sfo(&s)?,
        None => version_sfo(&sfo_ps3(chemin)?)?,
    };
    // Une version de PS3 : « 01.00 » (jamais autre chose dans une requête ou un fichier).
    let b = v.as_bytes();
    (b.len() == 5 && b[2] == b'.' && b.iter().enumerate().all(|(i, c)| i == 2 || c.is_ascii_digit())).then_some(v)
}

fn lire_debut_zip(zip: &Path, entree: &str, n: usize) -> Option<Vec<u8>> {
    let mut z = zip::ZipArchive::new(std::fs::File::open(zip).ok()?).ok()?;
    let mut f = z.by_name(entree).ok()?;
    let mut b = vec![0u8; n];
    let mut lu = 0;
    while lu < n {
        match f.read(&mut b[lu..]) {
            Ok(0) | Err(_) => break,
            Ok(k) => lu += k,
        }
    }
    b.truncate(lu);
    Some(b)
}

/// Les contenus additionnels PS3 d'un dossier (.pkg et .rap isolés, ou dans des .zip ; sous-dossiers compris).
pub fn chercher_ps3(dossier: &Path) -> Vec<Contenu> {
    let mut l: Vec<Contenu> = Vec::new();
    let mut raps: Vec<(String, (String, Option<String>))> = Vec::new();
    let mut a_voir = vec![dossier.to_path_buf()];
    let mut vus = 0;
    while let Some(d) = a_voir.pop() {
        let Ok(entrees) = std::fs::read_dir(&d) else { continue };
        for e in entrees.flatten() {
            vus += 1;
            if vus > 100_000 {
                return l; // un disque entier choisi par erreur : on s'arrête
            }
            let p = e.path();
            if p.is_dir() {
                a_voir.push(p);
                continue;
            }
            let ext = p.extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
            let nom_fichier = p.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let chemin = p.to_string_lossy().to_string();
            match ext.as_str() {
                "pkg" => {
                    let mut f = match std::fs::File::open(&p) {
                        Ok(f) => f,
                        Err(_) => continue,
                    };
                    let mut b = vec![0u8; 0x60];
                    let _ = f.read(&mut b);
                    if let Some(id) = content_id(&b) {
                        let taille = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
                        l.push(Contenu {
                            jeu: jeu_du_content_id(&id).unwrap_or_default(),
                            genre: genre_du_nom(&nom_fichier).into(),
                            nom: nom_fichier.clone(),
                            pkg: Some((chemin, None)),
                            rap: None,
                            taille,
                            id,
                        });
                    }
                }
                "rap" => raps.push((nom_fichier.to_uppercase(), (chemin, None))),
                "zip" => {
                    let Ok(f) = std::fs::File::open(&p) else { continue };
                    let Ok(mut z) = zip::ZipArchive::new(f) else { continue };
                    let noms: Vec<(String, u64)> = (0..z.len()).filter_map(|i| z.by_index(i).ok().map(|f| (f.name().to_string(), f.size()))).collect();
                    for (n, taille) in &noms {
                        let bas = n.to_lowercase();
                        if bas.ends_with(".rap") {
                            let stem = Path::new(n).file_stem().map(|s| s.to_string_lossy().to_uppercase()).unwrap_or_default();
                            raps.push((stem, (chemin.clone(), Some(n.clone()))));
                        } else if bas.ends_with(".pkg") {
                            if let Some(id) = lire_debut_zip(&p, n, 0x60).and_then(|b| content_id(&b)) {
                                l.push(Contenu {
                                    jeu: jeu_du_content_id(&id).unwrap_or_default(),
                                    genre: genre_du_nom(&nom_fichier).into(),
                                    nom: nom_fichier.clone(),
                                    pkg: Some((chemin.clone(), Some(n.clone()))),
                                    rap: None,
                                    taille: *taille,
                                    id,
                                });
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    for c in &mut l {
        c.rap = raps.iter().find(|(id, _)| *id == c.id.to_uppercase()).map(|(_, r)| r.clone());
    }
    l.sort_by(|a, b| a.nom.to_lowercase().cmp(&b.nom.to_lowercase()));
    l.dedup_by(|a, b| a.id == b.id);
    l
}

/// Les paquets d'une version qui ne sont PAS des .pkg lisibles (en-tête absent ou abîmé, fichier illisible) : leurs
/// noms, pour le DIRE à la personne (jamais écartés en silence).
pub fn illisibles(pkgs: &[(PathBuf, &str)]) -> Vec<String> {
    pkgs.iter()
        .filter(|(p, _)| {
            let mut b = vec![0u8; 0x60];
            let lu = std::fs::File::open(p).and_then(|mut f| f.read(&mut b)).is_ok();
            !lu || content_id(&b).is_none()
        })
        .map(|(p, _)| p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default())
        .collect()
}

/// Un nom lisible tiré d'un content ID, quand le fichier n'en a pas (les .pkg de Firehouse ont des noms aléatoires) :
/// `EP9000-BCES00799_00-000000000DOMINUS` → « DOMINUS ».
fn nom_du_content_id(id: &str) -> String {
    let fin = id.rsplit('-').next().unwrap_or(id).trim_start_matches('0');
    if fin.is_empty() { id.to_string() } else { fin.to_string() }
}

/// Les contenus d'une version de Firehouse (contrat 1.24, forme `contenus_emulateur`) : ses fichiers au rôle `dlc`
/// ou `maj` (.pkg, avec ce genre) et `licence` (.rap) — ils sont À CE JEU, sans tri par identifiant (un disque BCES00510 sert aussi des
/// licences BCES00799). Chaque .rap s'apparie à un .pkg par le content ID de l'EN-TÊTE du .pkg, jamais par son nom
/// (aléatoire) ; un .rap sans .pkg reste une licence seule, posée sous son nom.
pub fn ps3_de_la_version(pkgs: &[(PathBuf, &str)], raps: &[PathBuf]) -> Vec<Contenu> {
    let mut l: Vec<Contenu> = Vec::new();
    for (p, genre) in pkgs {
        let Ok(mut f) = std::fs::File::open(p) else { continue };
        let mut b = vec![0u8; 0x60];
        let _ = f.read(&mut b);
        let Some(id) = content_id(&b) else { continue };
        l.push(Contenu {
            jeu: jeu_du_content_id(&id).unwrap_or_default(),
            nom: nom_du_content_id(&id),
            genre: genre.to_string(),
            pkg: Some((p.to_string_lossy().to_string(), None)),
            rap: None,
            taille: std::fs::metadata(p).map(|m| m.len()).unwrap_or(0),
            id,
        });
    }
    for r in raps {
        let id = r.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let source = (r.to_string_lossy().to_string(), None);
        match l.iter_mut().find(|c| c.id.eq_ignore_ascii_case(&id)) {
            Some(c) => c.rap = Some(source),
            // Le nom d'une licence EST son content ID : rien d'autre n'est accepté (il devient un nom de fichier).
            None if jeu_du_content_id(&id).is_some() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') => l.push(Contenu {
                jeu: jeu_du_content_id(&id).unwrap_or_default(),
                nom: nom_du_content_id(&id),
                genre: "licence".into(),
                pkg: None,
                rap: Some(source),
                taille: std::fs::metadata(r).map(|m| m.len()).unwrap_or(0),
                id,
            }),
            None => {}
        }
    }
    l.sort_by(|a, b| a.nom.to_lowercase().cmp(&b.nom.to_lowercase()).then(a.id.cmp(&b.id)));
    l.dedup_by(|a, b| a.id == b.id);
    l
}

// --- Switch (Eden) : mises à jour et DLC en .nsp. `user\config\qt-config.ini` a une liste `external_content_dirs` de
// dossiers où Eden LIT les mises à jour et DLC, sans rien installer, rangée dans `[UI]` sous
// `Paths\external_content_dirs\size` et `Paths\external_content_dirs\N\path` (relu le 18/10 dans `qt_config.cpp` :
// le groupe `Paths` est ouvert DANS le groupe `UI` — la 0.68.1 et avant écrivaient à tort dans une section `[Paths]`).
// « Installer » = ajouter le dossier à cette liste (avec l'accord de la personne) ; rien n'est copié. ---

/// Un contenu Switch (mise à jour ou DLC) trouvé.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContenuSwitch {
    pub chemin: String,
    pub nom: String,
    /// `maj` ou `dlc`.
    pub genre: String,
    /// Son identifiant Nintendo (16 chiffres hexadécimaux).
    pub id: String,
    /// L'identifiant du jeu de base.
    pub jeu: String,
    pub taille: u64,
}

/// Un identifiant Switch écrit dans un nom : « [010055D009F79004] ».
pub fn id_switch_du_nom(nom: &str) -> Option<u64> {
    nom.split('[').skip(1).filter_map(|m| m.split(']').next()).find_map(|m| {
        (m.len() == 16 && m.chars().all(|c| c.is_ascii_hexdigit()) && m.starts_with("01")).then(|| u64::from_str_radix(m, 16).ok()).flatten()
    })
}

/// Les noms des fichiers d'un .nsp (en-tête PFS0, sans rien déchiffrer).
pub fn noms_pfs0(entete: &[u8]) -> Vec<String> {
    if entete.len() < 16 || &entete[..4] != b"PFS0" {
        return vec![];
    }
    let n = u32::from_le_bytes([entete[4], entete[5], entete[6], entete[7]]) as usize;
    let taille_noms = u32::from_le_bytes([entete[8], entete[9], entete[10], entete[11]]) as usize;
    let table = 16 + n * 24;
    let Some(noms) = entete.get(table..table + taille_noms) else { return vec![] };
    (0..n.min(256))
        .filter_map(|i| {
            let e = 16 + i * 24 + 16;
            let o = u32::from_le_bytes(entete.get(e..e + 4)?.try_into().ok()?) as usize;
            Some(noms.get(o..)?.iter().take_while(|b| **b != 0).map(|b| *b as char).collect())
        })
        .collect()
}

/// L'identifiant d'un .nsp : celui de son nom, sinon celui de son ticket (« <rights id>.tik » : 16 premiers chiffres).
pub fn id_switch(chemin: &Path) -> Option<u64> {
    let nom = chemin.file_name()?.to_string_lossy().to_string();
    if let Some(id) = id_switch_du_nom(&nom) {
        return Some(id);
    }
    let mut f = std::fs::File::open(chemin).ok()?;
    let mut b = vec![0u8; 0x4000];
    let n = f.read(&mut b).ok()?;
    noms_pfs0(&b[..n]).iter().find_map(|x| {
        let r = x.strip_suffix(".tik")?;
        (r.len() == 32 && r.is_ascii()).then(|| r.get(..16).and_then(|x| u64::from_str_radix(x, 16).ok())).flatten()
    })
}

/// Le genre et le jeu de base d'un identifiant : jeu (…000), mise à jour (…800), DLC (le jeu + 0x1000 + n).
pub fn classer_switch(id: u64) -> (&'static str, u64) {
    match id & 0xFFF {
        0 => ("jeu", id),
        0x800 => ("maj", id - 0x800),
        _ => ("dlc", (id & !0xFFF).wrapping_sub(0x1000)),
    }
}

/// Les mises à jour et DLC Switch (.nsp) d'un dossier (sous-dossiers compris).
pub fn chercher_switch(dossier: &Path) -> Vec<ContenuSwitch> {
    let mut l = Vec::new();
    let mut a_voir = vec![dossier.to_path_buf()];
    while let Some(d) = a_voir.pop() {
        let Ok(entrees) = std::fs::read_dir(&d) else { continue };
        for e in entrees.flatten() {
            let p = e.path();
            if p.is_dir() {
                a_voir.push(p);
                continue;
            }
            if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("nsp")) {
                continue;
            }
            let Some(id) = id_switch(&p) else { continue };
            let (genre, jeu) = classer_switch(id);
            if genre == "jeu" {
                continue; // un jeu de base, pas un contenu additionnel
            }
            l.push(ContenuSwitch {
                nom: p.file_stem().map(|n| n.to_string_lossy().replace('_', " ")).unwrap_or_default(),
                chemin: p.to_string_lossy().to_string(),
                genre: genre.into(),
                id: format!("{id:016X}"),
                jeu: format!("{jeu:016X}"),
                taille: e.metadata().map(|m| m.len()).unwrap_or(0),
            });
        }
    }
    l.sort_by(|a, b| a.nom.to_lowercase().cmp(&b.nom.to_lowercase()));
    l
}

/// Les contenus Switch d'un jeu : par l'identifiant du jeu ; sinon par son titre (le nom du contenu commence par lui).
pub fn switch_du_jeu<'a>(l: &'a [ContenuSwitch], id_jeu: Option<u64>, titre: &str) -> Vec<&'a ContenuSwitch> {
    let simple = |t: &str| -> String { t.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect() };
    let t = simple(&crate::import_local::titre_depuis_nom(&format!("{titre}.x")));
    l.iter()
        .filter(|c| match id_jeu {
            Some(id) => c.jeu == format!("{id:016X}"),
            None => {
                let n = simple(c.nom.split(['[', '(']).next().unwrap_or(&c.nom).split(" v").next().unwrap_or(&c.nom));
                !t.is_empty() && (n == t || n.starts_with(&t))
            }
        })
        .collect()
}

/// La configuration d'Eden (mode portable : `<eden>\user\config\qt-config.ini`).
pub fn ini_eden(eden: &Path) -> PathBuf {
    eden.join("user").join("config").join("qt-config.ini")
}

/// Les dossiers de contenus externes réglés dans Eden : la section `[UI]`, clés `Paths\external_content_dirs\…`
/// (sources d'Eden, `qt_config.cpp` relu le 18/10 : `ReadPathValues` ouvre le groupe `Paths` À L'INTÉRIEUR du groupe
/// `UI` ; même forme que `Paths\gamedirs\…` dans le `qt-config.ini` de suyu sur Venkman).
pub fn dossiers_externes_eden(texte: &str) -> Vec<String> {
    liste_eden(texte, "UI", "Paths\\external_content_dirs")
}

/// Ce que Frogtend avait écrit avant la 0.68.2 dans une section `[Paths]` qu'Eden ne lit pas : repris (avec l'accord
/// donné alors) au prochain ajout.
fn dossiers_eden_mal_ranges(texte: &str) -> Vec<String> {
    liste_eden(texte, "Paths", "external_content_dirs")
}

fn liste_eden(texte: &str, section: &str, prefixe: &str) -> Vec<String> {
    let lire = |cle: &str| crate::manettes::lire_ini(texte, section, cle).into_iter().next();
    let n: usize = lire(&format!("{prefixe}\\size")).and_then(|v| v.trim().parse().ok()).unwrap_or(0);
    // Une valeur entre guillemets (écrite ainsi par Eden ou par Frogtend) est relue sans eux.
    (1..=n.min(256))
        .filter_map(|i| lire(&format!("{prefixe}\\{i}\\path")))
        .map(|v| crate::emulateurs_profils::lire_valeur_qt(&v))
        .filter(|v| !v.is_empty())
        .collect()
}

/// Deux dossiers sont-ils le même ? (Windows : sans casse ; « / » et « \ » se valent.)
pub fn meme_dossier(a: &str, b: &str) -> bool {
    let n = |s: &str| s.replace('\\', "/").trim_end_matches('/').to_lowercase();
    n(a) == n(b)
}

/// Ajoute des dossiers à la liste d'Eden (sans doublon), en gardant tout le reste du fichier. Les chemins sont écrits
/// avec des « / » : dans un fichier de réglages Qt (QSettings), « \ » est un caractère d'échappement (un dossier
/// « D:\temp » serait relu avec une tabulation).
pub fn ajouter_dossiers_eden(texte: &str, dossiers: &[String]) -> String {
    // TOUS en « / », les anciens comme les nouveaux : une valeur ne porte alors jamais de « \ », que le lecteur d'Eden
    // traiterait comme un échappement (ou pas) selon sa version — le même chemin pour toutes.
    let mut l: Vec<String> = dossiers_externes_eden(texte).into_iter().map(|d| d.replace('\\', "/")).collect();
    for d in dossiers_eden_mal_ranges(texte).iter().chain(dossiers) {
        if !l.iter().any(|x| meme_dossier(x, d)) {
            l.push(d.replace('\\', "/"));
        }
    }
    let mut valeurs: Vec<(String, Vec<String>)> = vec![("Paths\\external_content_dirs\\size".into(), vec![l.len().to_string()])];
    for (i, d) in l.iter().enumerate() {
        // Entre guillemets : une virgule dans le chemin (« Zelda, TOTK ») ferait relire une LISTE à Eden.
        valeurs.push((format!("Paths\\external_content_dirs\\{}\\path", i + 1), vec![crate::emulateurs_profils::valeur_qt(d)]));
    }
    let refs: Vec<(&str, Vec<String>)> = valeurs.iter().map(|(c, v)| (c.as_str(), v.clone())).collect();
    let texte = crate::emulateurs_profils::ecrire_ini(texte, "UI", &refs);
    // Les anciennes lignes de `[Paths]` (écrites par Frogtend lui-même) partent une fois reprises : jamais rejouées, un
    // dossier que la personne retire ensuite dans Eden ne revient pas.
    let lire = |cle: &str| crate::manettes::lire_ini(&texte, "Paths", cle).into_iter().next();
    let Some(n) = lire("external_content_dirs\\size") else { return texte };
    let n: usize = n.trim().parse().unwrap_or(0).min(256);
    let mut cles = vec!["external_content_dirs\\size".to_string()];
    cles.extend((1..=n).map(|i| format!("external_content_dirs\\{i}\\path")));
    let vides: Vec<(&str, Vec<String>)> = cles.iter().map(|c| (c.as_str(), vec![])).collect();
    crate::emulateurs_profils::ecrire_ini(&texte, "Paths", &vides)
}

// --- Wii U : un .wua (ZArchive de l'équipe de Cemu, licence MIT-0) peut contenir le jeu, ses mises à jour et ses DLC,
// chacun dans un dossier racine « <identifiant 16 chiffres>_v<version> » ; Cemu les lit directement (rien à installer).
// Relevé dans ZArchive (include/zarchive/zarchivecommon.h, src/zarchivereader.cpp), 03/10 : fin de fichier de 144
// octets gros-boutistes (6 sections {position, taille}, empreinte, taille, version 0x61bf3a01, magie 0x169f52d6) ;
// arbre = entrées de 16 octets (bit 31 du 1er mot : fichier ; dossier : 1er enfant, nombre, réservé) ; table des noms non
// compressée (longueur sur 1 octet, ou 2 si le bit 7 est mis). ---

/// Un titre contenu dans un .wua.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TitreWiiU {
    pub id: String,
    /// `jeu` (00050000), `maj` (0005000E), `dlc` (0005000C), `autre`.
    pub genre: String,
    pub version: u32,
}

pub fn genre_wiiu(id: u64) -> &'static str {
    match id >> 32 {
        0x0005_0000 => "jeu",
        0x0005_000E => "maj",
        0x0005_000C => "dlc",
        _ => "autre",
    }
}

/// Les titres d'un .wua (dossiers racine), sans rien décompresser.
pub fn titres_wua(chemin: &Path) -> Resultat<Vec<TitreWiiU>> {
    use std::io::{Seek, SeekFrom};
    let mut f = std::fs::File::open(chemin)?;
    let taille = f.metadata()?.len();
    if taille < 144 {
        return Ok(vec![]);
    }
    let mut pied = [0u8; 144];
    f.seek(SeekFrom::Start(taille - 144))?;
    f.read_exact(&mut pied)?;
    let be64 = |b: &[u8], i: usize| u64::from_be_bytes(b[i..i + 8].try_into().unwrap_or_default());
    let be32 = |b: &[u8], i: usize| u32::from_be_bytes(b[i..i + 4].try_into().unwrap_or_default());
    if be32(&pied, 140) != 0x169f_52d6 || be32(&pied, 136) != 0x61bf_3a01 {
        return Ok(vec![]); // pas une ZArchive
    }
    let section = |n: usize| (be64(&pied, n * 16), be64(&pied, n * 16 + 8));
    let (noms_o, noms_t) = section(2);
    let (arbre_o, arbre_t) = section(3);
    if noms_o + noms_t > taille || arbre_o + arbre_t > taille || noms_t > 64 << 20 || arbre_t > 256 << 20 {
        return Ok(vec![]);
    }
    let lire = |f: &mut std::fs::File, o: u64, t: u64| -> Resultat<Vec<u8>> {
        let mut b = vec![0u8; t as usize];
        f.seek(SeekFrom::Start(o))?;
        f.read_exact(&mut b)?;
        Ok(b)
    };
    let noms = lire(&mut f, noms_o, noms_t)?;
    let arbre = lire(&mut f, arbre_o, arbre_t)?;
    let nom = |o: usize| -> String {
        let Some(&b0) = noms.get(o) else { return String::new() };
        let (longueur, debut) = if b0 & 0x80 != 0 { (((b0 & 0x7F) as usize) | ((*noms.get(o + 1).unwrap_or(&0) as usize) << 7), o + 2) } else { (b0 as usize, o + 1) };
        noms.get(debut..debut + longueur).map(|n| n.iter().map(|c| *c as char).collect()).unwrap_or_default()
    };
    let entree = |i: usize| -> Option<(u32, u32, u32)> {
        let e = arbre.get(i * 16..i * 16 + 16)?;
        Some((be32(e, 0), be32(e, 4), be32(e, 8)))
    };
    let Some((racine, debut, nombre)) = entree(0) else { return Ok(vec![]) };
    if racine & 0x8000_0000 != 0 {
        return Ok(vec![]);
    }
    let mut l = Vec::new();
    for i in debut as usize..(debut as usize + nombre.min(64) as usize) {
        let Some((n, _, _)) = entree(i) else { break };
        if n & 0x8000_0000 != 0 {
            continue; // un fichier à la racine
        }
        let dossier = nom((n & 0x7FFF_FFFF) as usize);
        let Some((id, v)) = dossier.split_once("_v") else { continue };
        let (Ok(idn), Ok(version)) = (u64::from_str_radix(id, 16), v.parse::<u32>()) else { continue };
        l.push(TitreWiiU { id: id.to_uppercase(), genre: genre_wiiu(idn).into(), version });
    }
    Ok(l)
}

/// Un .zip qui ne contient QUE des contenus additionnels (.pkg, .rap, .edat) : ce n'est pas un jeu (à l'import, il est
/// écarté et annoncé comme contenu). Seul le sommaire du zip est lu.
pub fn zip_de_contenus(chemin: &Path) -> bool {
    if !chemin.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip")) {
        return false;
    }
    let Ok(f) = std::fs::File::open(chemin) else { return false };
    let Ok(mut z) = zip::ZipArchive::new(f) else { return false };
    let noms: Vec<String> = (0..z.len()).filter_map(|i| z.by_index(i).ok().filter(|f| f.is_file()).map(|f| f.name().to_lowercase())).collect();
    !noms.is_empty() && noms.iter().any(|n| n.ends_with(".pkg")) && noms.iter().all(|n| n.ends_with(".pkg") || n.ends_with(".rap") || n.ends_with(".edat"))
}

/// Un fichier qui est un contenu additionnel et non un jeu : un zip de .pkg/.rap (PS3), ou un .nsp de mise à jour ou de
/// DLC (Switch : identifiant qui ne finit pas par 000).
pub fn est_un_contenu(chemin: &Path) -> bool {
    if zip_de_contenus(chemin) {
        return true;
    }
    chemin.extension().is_some_and(|e| e.eq_ignore_ascii_case("nsp")) && id_switch(chemin).is_some_and(|id| classer_switch(id).0 != "jeu")
}

/// Les contenus d'un jeu : par son identifiant (sûr) ; sinon par son titre (le nom du contenu commence par le titre du
/// jeu, comme « Asuras Wrath - Episode 11.5 »).
pub fn du_jeu<'a>(contenus: &'a [Contenu], titre_id: Option<&str>, titre: &str) -> Vec<&'a Contenu> {
    let simple = |t: &str| -> String { t.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect() };
    let t = simple(&crate::import_local::titre_depuis_nom(&format!("{titre}.x")));
    contenus
        .iter()
        .filter(|c| match titre_id {
            Some(id) if !c.jeu.is_empty() => c.jeu.eq_ignore_ascii_case(id),
            _ => !t.is_empty() && simple(c.nom.split(" - ").next().unwrap_or(&c.nom)) == t,
        })
        .collect()
}

/// Ce que Frogtend a installé dans un RPCS3 (les .pkg sont communs à tous les comptes) : `<rpcs3>\.frogtend-contenus.json`.
fn registre(rpcs3: &Path) -> PathBuf {
    rpcs3.join(".frogtend-contenus.json")
}

pub fn installes(rpcs3: &Path) -> Vec<String> {
    std::fs::read(registre(rpcs3)).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default()
}

/// La licence d'un contenu est-elle dans le compte RPCS3 de ce profil ?
pub fn licence_posee(rpcs3: &Path, compte: &str, id: &str) -> bool {
    rpcs3.join("dev_hdd0").join("home").join(compte).join("exdata").join(format!("{id}.rap")).is_file()
}

/// Sort un fichier (isolé ou dans un zip) vers \`dossier\` ; rend son chemin et s'il faut le retirer après.
fn sortir(source: &(String, Option<String>), dossier: &Path) -> Resultat<(PathBuf, bool)> {
    match &source.1 {
        None => Ok((PathBuf::from(&source.0), false)),
        Some(entree) => {
            std::fs::create_dir_all(dossier)?;
            let nom = Path::new(entree).file_name().ok_or_else(|| Erreur::Refus("Nom de fichier invalide dans le zip.".into()))?;
            let cible = dossier.join(nom);
            let mut z = zip::ZipArchive::new(std::fs::File::open(&source.0)?).map_err(|_| Erreur::Disque("Zip illisible.".into()))?;
            let mut f = z.by_name(entree).map_err(|_| Erreur::Disque("Fichier introuvable dans le zip.".into()))?;
            let attendu = f.size();
            let mut sortie = std::fs::File::create(&cible)?;
            std::io::copy(&mut f, &mut sortie)?;
            if std::fs::metadata(&cible).map(|m| m.len()).ok() != Some(attendu) {
                let _ = std::fs::remove_file(&cible);
                return Err(Erreur::Disque(format!("L'extraction de {entree} a échoué.")));
            }
            Ok((cible, true))
        }
    }
}

/// Installe un contenu PS3 dans un RPCS3, pour le compte de ce profil : le .pkg par \`installer_pkg\` (RPCS3 lui-même,
/// sauf s'il est déjà installé), puis la licence dans le compte du profil. Les fichiers extraits d'un zip sont
/// retirés ensuite (seulement eux).
pub fn installer_ps3(c: &Contenu, rpcs3: &Path, compte: &str, travail: &Path, installer_pkg: &dyn Fn(&Path) -> Resultat<()>) -> Resultat<()> {
    let deja = installes(rpcs3);
    let mut paquet: Resultat<()> = Ok(());
    if let Some(source) = c.pkg.as_ref().filter(|_| !deja.contains(&c.id)) {
        paquet = installer_paquet(c, source, rpcs3, travail, installer_pkg).and_then(|()| {
            let mut l = deja;
            l.push(c.id.clone());
            std::fs::create_dir_all(rpcs3)?;
            Ok(std::fs::write(registre(rpcs3), serde_json::to_vec_pretty(&l).unwrap_or_default())?)
        });
    }
    // La licence ne dépend pas du .pkg (il peut déjà être là, installé hors de Frogtend) : posée quand même.
    if let Some(rap) = &c.rap {
        let (fichier, temporaire) = sortir(rap, travail)?;
        let r = poser_licence(&fichier, rpcs3, compte, &c.id);
        if temporaire {
            let _ = std::fs::remove_file(&fichier);
        }
        r?;
    }
    paquet
}

/// Le relevé d'un dossier (chemin → taille, heure), borné : `None` au-delà (trop gros pour en juger).
fn releve(dossier: &Path) -> Option<std::collections::BTreeMap<PathBuf, (u64, Option<std::time::SystemTime>)>> {
    let mut r = std::collections::BTreeMap::new();
    let mut a_voir = vec![dossier.to_path_buf()];
    while let Some(d) = a_voir.pop() {
        let Ok(l) = std::fs::read_dir(&d) else { continue };
        for e in l.flatten() {
            if r.len() > 200_000 {
                return None;
            }
            let Ok(t) = e.file_type() else { continue };
            if t.is_dir() {
                a_voir.push(e.path());
            } else if let Ok(m) = e.metadata() {
                r.insert(e.path(), (m.len(), m.modified().ok()));
            }
        }
    }
    Some(r)
}

/// Installe le .pkg d'un contenu par RPCS3, puis VÉRIFIE le résultat (jamais le seul code de retour) : un relevé de
/// `dev_hdd0\game\<jeu>` avant/après (un fichier nouveau, changé de taille ou d'heure), sinon un fichier écrit
/// depuis le début ailleurs dans `game` (un paquet peut choisir son dossier).
fn installer_paquet(c: &Contenu, source: &(String, Option<String>), rpcs3: &Path, travail: &Path, installer_pkg: &dyn Fn(&Path) -> Resultat<()>) -> Resultat<()> {
    let game = rpcs3.join("dev_hdd0").join("game");
    let dossier_jeu = (!c.jeu.is_empty()).then(|| game.join(&c.jeu));
    let avant = dossier_jeu.as_deref().and_then(releve);
    let (pkg, temporaire) = sortir(source, travail)?;
    let debut = std::time::SystemTime::now() - std::time::Duration::from_secs(2);
    let r = installer_pkg(&pkg);
    if temporaire {
        let _ = std::fs::remove_file(&pkg);
    }
    r?;
    let apres = dossier_jeu.as_deref().and_then(releve);
    let change = match (&avant, &apres) {
        (Some(a), Some(b)) => a != b,
        _ => false,
    };
    if change || ecrit_depuis(&game, "", debut) {
        return Ok(());
    }
    if avant.is_some_and(|a| !a.is_empty()) {
        Err(Erreur::Refus("RPCS3 n'a rien changé : ce contenu était peut-être déjà installé hors de Frogtend. Il n'est pas noté installé.".into()))
    } else {
        Err(Erreur::Disque("RPCS3 s'est fermé sans rien installer (dev_hdd0\\game n'a pas changé) : ce contenu n'est pas noté installé.".into()))
    }
}

/// `dev_hdd0` est-il à sa place, dans le dossier de RPCS3 ? RPCS3 permet de le déplacer (`config\vfs.yml`,
/// `/dev_hdd0/`) : Frogtend ne poserait alors rien au bon endroit, il refuse.
pub fn dev_hdd0_a_sa_place(rpcs3: &Path) -> bool {
    let Ok(t) = std::fs::read_to_string(rpcs3.join("config").join("vfs.yml")) else { return true };
    match t.lines().find_map(|l| l.trim().strip_prefix("/dev_hdd0/:")) {
        None => true,
        Some(v) => {
            let v = v.trim().trim_matches(['"', '\'']).replace('\\', "/");
            v.is_empty() || v.eq_ignore_ascii_case("$(EmulatorDir)dev_hdd0/") || v.eq_ignore_ascii_case("$(EmulatorDir)dev_hdd0")
        }
    }
}

/// Une licence est exactement 16 octets.
const TAILLE_RAP: u64 = 0x10;

/// Pose une licence dans le compte RPCS3 de ce profil, JAMAIS par-dessus une autre : la même déjà là → rien à faire ;
/// une différente → refus (elle reste telle quelle).
fn poser_licence(fichier: &Path, rpcs3: &Path, compte: &str, id: &str) -> Resultat<()> {
    if std::fs::metadata(fichier).map(|m| m.len()).unwrap_or(0) != TAILLE_RAP {
        return Err(Erreur::Refus("Ce fichier .rap n'a pas la taille d'une licence (16 octets) : il n'est pas posé.".into()));
    }
    let neuve = std::fs::read(fichier)?;
    let dossier = rpcs3.join("dev_hdd0").join("home").join(compte).join("exdata");
    let cible = dossier.join(format!("{id}.rap"));
    if cible.exists() {
        // La taille d'abord : un fichier énorme à ce nom n'est jamais lu en entier.
        let pareille = std::fs::metadata(&cible).is_ok_and(|m| m.len() == TAILLE_RAP) && std::fs::read(&cible).ok().as_deref() == Some(&neuve[..]);
        return if pareille {
            Ok(())
        } else {
            Err(Erreur::Refus("Une autre licence pour ce contenu est déjà dans ton compte RPCS3 : Frogtend ne la remplace pas.".into()))
        };
    }
    std::fs::create_dir_all(&dossier)?;
    // `create_new` : rien n'est écrasé, même si une licence apparaît entre-temps.
    let mut f = std::fs::OpenOptions::new().write(true).create_new(true).open(&cible)?;
    std::io::Write::write_all(&mut f, &neuve)?;
    drop(f);
    if std::fs::read(&cible).ok().as_deref() != Some(&neuve[..]) {
        let _ = std::fs::remove_file(&cible);
        return Err(Erreur::Disque("La licence relue ne correspond pas à celle de Firehouse : retirée.".into()));
    }
    Ok(())
}

/// Quelque chose a-t-il été écrit dans `game` depuis `debut` ? D'abord le dossier du jeu du contenu (là où RPCS3
/// installe d'ordinaire), sinon tout `game` (un paquet peut choisir son dossier). Relevé borné : au-delà, on ne sait
/// pas, et on ne bloque pas.
fn ecrit_depuis(game: &Path, jeu: &str, debut: std::time::SystemTime) -> bool {
    let recent = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).is_ok_and(|t| t >= debut);
    let cherche = |racine: &Path| -> Option<bool> {
        let mut a_voir = vec![racine.to_path_buf()];
        let mut vus = 0u32;
        // Les FICHIERS seulement : l'heure d'un dossier change aussi quand on n'y fait que retirer.
        while let Some(d) = a_voir.pop() {
            let Ok(l) = std::fs::read_dir(&d) else { continue };
            for e in l.flatten() {
                vus += 1;
                if vus > 200_000 {
                    return None;
                }
                let p = e.path();
                if e.file_type().is_ok_and(|t| t.is_dir()) {
                    a_voir.push(p);
                } else if recent(&p) {
                    return Some(true);
                }
            }
        }
        Some(false)
    };
    if !jeu.is_empty() && cherche(&game.join(jeu)) == Some(true) {
        return true;
    }
    cherche(game).unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entete_pkg(id: &str) -> Vec<u8> {
        let mut b = vec![0u8; 0x100];
        b[..4].copy_from_slice(b"\x7FPKG");
        b[0x30..0x30 + id.len()].copy_from_slice(id.as_bytes());
        b
    }

    #[test]
    fn un_nsp_au_ticket_bizarre_ne_fait_pas_fermer_frogtend() {
        // Un nom de ticket de 32 octets dont un n'est pas ASCII, à cheval sur la coupe à 16 caractères.
        let mut nom = vec![b'a'; 15];
        nom.push(0xE9);
        nom.extend([b'b'; 16]);
        nom.extend(b".tik\0");
        let mut o = b"PFS0".to_vec();
        o.extend(1u32.to_le_bytes());
        o.extend((nom.len() as u32).to_le_bytes());
        o.extend([0u8; 4]);
        o.extend([0u8; 16]); // position et taille du fichier
        o.extend(0u32.to_le_bytes()); // position du nom
        o.extend([0u8; 4]);
        o.extend(&nom);
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("contenu.nsp");
        std::fs::write(&f, &o).unwrap();
        assert_eq!(id_switch(&f), None);
    }

    #[test]
    fn switch_identifiants_genres_et_jeu_de_base() {
        assert_eq!(id_switch_du_nom("Fire Emblem Three Houses [Additional Quests][010055D009F79004][US][v196608].nsp"), Some(0x010055D009F79004));
        assert_eq!(id_switch_du_nom("Dragon_Quest_Builders_2 v65536.nsp"), None);
        assert_eq!(classer_switch(0x010055D009F79004), ("dlc", 0x010055D009F78000));
        assert_eq!(classer_switch(0x010055D009F78800), ("maj", 0x010055D009F78000));
        assert_eq!(classer_switch(0x010055D009F78000).0, "jeu");
        // Un .nsp sans identifiant dans son nom : celui de son ticket.
        let mut e = b"PFS0".to_vec();
        let noms = b"0123456789abcdef0123456789abcdef.nca\0010055D009F788000000000000000000.tik\0";
        e.extend(2u32.to_le_bytes());
        e.extend((noms.len() as u32).to_le_bytes());
        e.extend(0u32.to_le_bytes());
        for o in [0u32, 37] {
            e.extend(0u64.to_le_bytes());
            e.extend(0u64.to_le_bytes());
            e.extend(o.to_le_bytes());
            e.extend(0u32.to_le_bytes());
        }
        e.extend(noms);
        assert_eq!(noms_pfs0(&e)[1], "010055D009F788000000000000000000.tik");
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("Dragon_Quest_Builders_2 v65536.nsp"), &e).unwrap();
        assert_eq!(id_switch(&d.path().join("Dragon_Quest_Builders_2 v65536.nsp")), Some(0x010055D009F78800));
        let l = chercher_switch(d.path());
        assert_eq!((l.len(), l[0].genre.as_str()), (1, "maj"));
        assert_eq!(switch_du_jeu(&l, None, "Dragon Quest Builders 2").len(), 1, "par le titre");
        assert_eq!(switch_du_jeu(&l, Some(0x010055D009F78000), "autre").len(), 1, "par l'identifiant");
        assert!(est_un_contenu(&d.path().join("Dragon_Quest_Builders_2 v65536.nsp")), "une mise à jour n'est pas un jeu");
        std::fs::write(d.path().join("Jeu [0100AAAA00001000].nsp"), b"x").unwrap();
        assert!(!est_un_contenu(&d.path().join("Jeu [0100AAAA00001000].nsp")), "un jeu de base reste un jeu");
    }

    #[test]
    fn eden_recoit_un_dossier_de_contenus_sans_perdre_le_reste() {
        // La forme réelle (qt_config.cpp d'Eden ; `Paths\gamedirs\…` du qt-config.ini de suyu sur Venkman) : `[UI]`.
        let avant = "[UI]\r\nPaths\\gamedirs\\size=1\r\nPaths\\external_content_dirs\\size=1\r\nPaths\\external_content_dirs\\1\\path=E:\\DLC\r\ntheme=dark\r\n[Controls]\r\nx=1\r\n";
        assert_eq!(dossiers_externes_eden(avant), ["E:\\DLC"]);
        let apres = ajouter_dossiers_eden(avant, &["D:\\Switch Maj & DLC".into(), "e:\\dlc".into()]);
        assert_eq!(dossiers_externes_eden(&apres), ["E:/DLC", "D:/Switch Maj & DLC"], "sans doublon ; tous en « / »");
        assert!(!apres.contains("E:\\\\DLC"), "jamais de « \\ » réécrit dans une valeur d'Eden");
        assert!(!apres.contains("[Paths]"), "jamais une section qu'Eden ne lit pas");
        assert!(meme_dossier("D:/Switch Maj & DLC/", "d:\\switch maj & dlc"));
        assert!(!meme_dossier("D:/Switch", "D:/Switch2"));
        assert!(apres.contains("Paths\\gamedirs\\size") && apres.contains("theme=dark") && apres.contains("x=1"), "le reste est gardé");
    }

    #[test]
    fn les_dossiers_ecrits_avant_dans_une_section_qu_eden_ne_lit_pas_sont_repris() {
        // 0.68.1 et avant : `[Paths]` (jamais lu par Eden). Repris dans `[UI]` au prochain ajout.
        let avant = "[Paths]\r\nexternal_content_dirs\\size=1\r\nexternal_content_dirs\\1\\path=E:/Vieux DLC\r\n[UI]\r\ntheme=dark\r\n";
        assert!(dossiers_externes_eden(avant).is_empty(), "Eden ne les voyait pas");
        let apres = ajouter_dossiers_eden(avant, &["D:\\Nouveau".into()]);
        assert_eq!(dossiers_externes_eden(&apres), ["E:/Vieux DLC", "D:/Nouveau"]);
        assert!(!apres.contains("external_content_dirs\\1\\path=E:/Vieux"), "les anciennes lignes partent une fois reprises");
        assert!(apres.contains("theme=dark"));
        // La personne retire ensuite « E:/Vieux DLC » dans Eden : un nouvel ajout ne le remet jamais.
        let retire = apres
            .replace("Paths\\external_content_dirs\\size = 2", "Paths\\external_content_dirs\\size = 1")
            .replace("Paths\\external_content_dirs\\1\\path = \"E:/Vieux DLC\"", "Paths\\external_content_dirs\\1\\path = \"D:/Nouveau\"")
            .replace("Paths\\external_content_dirs\\2\\path = \"D:/Nouveau\"\r\n", "");
        assert_eq!(dossiers_externes_eden(&retire), ["D:/Nouveau"], "{retire}");
        let encore = ajouter_dossiers_eden(&retire, &["F:\\Autre".into()]);
        assert_eq!(dossiers_externes_eden(&encore), ["D:/Nouveau", "F:/Autre"]);
        // Sans anciennes lignes : aucune section `[Paths]` créée.
        assert!(!ajouter_dossiers_eden("[UI]\r\ntheme=dark\r\n", &["F:\\X".into()]).contains("[Paths]"));
    }

    #[test]
    fn un_dossier_de_contenus_avec_une_virgule_reste_un_seul_dossier_pour_eden() {
        // Eden (QSettings) met entre guillemets une valeur qui porte une virgule ; Frogtend aussi, et relit sans eux.
        let avant = "[UI]\r\nPaths\\external_content_dirs\\size=1\r\nPaths\\external_content_dirs\\1\\path=\"E:/Zelda, TOTK\"\r\n";
        assert_eq!(dossiers_externes_eden(avant), ["E:/Zelda, TOTK"]);
        let apres = ajouter_dossiers_eden(avant, &["D:\\Jeux d'émulation, Switch".into()]);
        assert_eq!(dossiers_externes_eden(&apres), ["E:/Zelda, TOTK", "D:/Jeux d'émulation, Switch"]);
        assert!(apres.contains("\"D:/Jeux d'émulation, Switch\""), "entre guillemets : jamais relu comme une liste");
        assert_eq!(apres.matches("Zelda").count(), 1, "pas de guillemets doublés au passage");
    }

    #[test]
    fn un_wua_dit_ses_titres() {
        // Une petite ZArchive : noms, arbre (racine + 3 dossiers), puis la fin de fichier.
        let mut noms = vec![0u8]; // nom vide de la racine
        let mut decalages = Vec::new();
        for n in ["0005000010102000_v0", "0005000E10102000_v48", "0005000C10102000_v16"] {
            decalages.push(noms.len() as u32);
            noms.push(n.len() as u8);
            noms.extend(n.as_bytes());
        }
        let mut arbre = Vec::new();
        arbre.extend(0u32.to_be_bytes());
        arbre.extend(1u32.to_be_bytes());
        arbre.extend(3u32.to_be_bytes());
        arbre.extend(0u32.to_be_bytes());
        for d in &decalages {
            arbre.extend(d.to_be_bytes());
            arbre.extend([0u8; 12]);
        }
        let mut o = vec![0u8; 16];
        let (pos_noms, pos_arbre) = (o.len() as u64, (o.len() + noms.len()) as u64);
        o.extend(&noms);
        o.extend(&arbre);
        let mut pied = Vec::new();
        for (p, t) in [(0u64, 0u64), (0, 0), (pos_noms, noms.len() as u64), (pos_arbre, arbre.len() as u64), (0, 0), (0, 0)] {
            pied.extend(p.to_be_bytes());
            pied.extend(t.to_be_bytes());
        }
        pied.extend([0u8; 32]);
        pied.extend(0u64.to_be_bytes());
        pied.extend(0x61bf_3a01u32.to_be_bytes());
        pied.extend(0x169f_52d6u32.to_be_bytes());
        o.extend(&pied);
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("jeu.wua");
        std::fs::write(&f, &o).unwrap();
        let l = titres_wua(&f).unwrap();
        assert_eq!(l.iter().map(|t| (t.genre.as_str(), t.version)).collect::<Vec<_>>(), [("jeu", 0), ("maj", 48), ("dlc", 16)]);
        std::fs::write(&f, b"pas une archive").unwrap();
        assert!(titres_wua(&f).unwrap().is_empty());
    }

    #[test]
    fn un_zip_de_dlc_n_est_pas_un_jeu() {
        use std::io::Write;
        let d = tempfile::tempdir().unwrap();
        let ecrire = |nom: &str, fichiers: &[&str]| {
            let mut z = zip::ZipWriter::new(std::fs::File::create(d.path().join(nom)).unwrap());
            for f in fichiers {
                z.start_file(*f, zip::write::SimpleFileOptions::default()).unwrap();
                z.write_all(b"x").unwrap();
            }
            z.finish().unwrap();
        };
        ecrire("dlc.zip", &["a.pkg", "a.rap"]);
        ecrire("jeu.zip", &["jeu.iso", "README.TXT"]);
        ecrire("licence-seule.zip", &["a.rap"]);
        assert!(zip_de_contenus(&d.path().join("dlc.zip")));
        assert!(!zip_de_contenus(&d.path().join("jeu.zip")));
        assert!(!zip_de_contenus(&d.path().join("licence-seule.zip")), "pas de .pkg : on ne décide pas");
    }

    #[test]
    fn le_content_id_donne_le_jeu() {
        let id = content_id(&entete_pkg("EP0102-BLES01227_00-DLCS120000000000")).unwrap();
        assert_eq!(id, "EP0102-BLES01227_00-DLCS120000000000");
        assert_eq!(jeu_du_content_id(&id).as_deref(), Some("BLES01227"));
        assert_eq!(content_id(b"pas un pkg"), None);
        assert_eq!(genre_du_nom("God of War - Achilles (Europe) (Avatar)"), "avatar");
        assert_eq!(genre_du_nom("Asuras Wrath - Episode 11.5 (Europe) (DLC)"), "dlc");
    }

    /// Un PARAM.SFO avec ces clés (chaînes UTF-8).
    fn faire_sfo(paires: &[(&str, &str)]) -> Vec<u8> {
        let mut cles: Vec<u8> = Vec::new();
        let mut donnees: Vec<u8> = Vec::new();
        let mut entrees: Vec<u8> = Vec::new();
        for (k, v) in paires {
            let (dk, dd) = (cles.len() as u16, donnees.len() as u32);
            cles.extend(k.as_bytes());
            cles.push(0);
            let mut val = v.as_bytes().to_vec();
            val.push(0);
            let n = val.len() as u32;
            donnees.extend(&val);
            entrees.extend(dk.to_le_bytes());
            entrees.extend(0x0204u16.to_le_bytes());
            entrees.extend(n.to_le_bytes());
            entrees.extend(n.to_le_bytes());
            entrees.extend(dd.to_le_bytes());
        }
        let debut_cles = 0x14 + entrees.len();
        let mut s = b"\0PSF".to_vec();
        s.extend(0x0101u32.to_le_bytes());
        s.extend((debut_cles as u32).to_le_bytes());
        s.extend(((debut_cles + cles.len()) as u32).to_le_bytes());
        s.extend((paires.len() as u32).to_le_bytes());
        s.extend(entrees);
        s.extend(cles);
        s.extend(donnees);
        s
    }

    #[test]
    fn la_version_d_un_jeu_ps3_est_celle_que_rpcs3_lance() {
        let d = tempfile::tempdir().unwrap();
        let rpcs3 = d.path().join("RPCS3");
        let jeu = d.path().join("Jeu");
        std::fs::create_dir_all(jeu.join("PS3_GAME")).unwrap();
        // Le disque d'Asura's Wrath (relevé réel du 19/10) : APP_VER 01.00, mais VERSION 01.01 — APP_VER compte.
        std::fs::write(jeu.join("PS3_GAME").join("PARAM.SFO"), faire_sfo(&[("APP_VER", "01.00"), ("CATEGORY", "DG"), ("TITLE_ID", "BLES01227"), ("VERSION", "01.01")])).unwrap();
        assert_eq!(version_ps3(&jeu, &rpcs3, "BLES01227").as_deref(), Some("01.00"));
        // Un cache FIOS de RPCS3 (GD, sans EBOOT.BIN, comme chez Seb) : ce n'est pas une mise à jour.
        let maj = rpcs3.join("dev_hdd0").join("game").join("BLES01227");
        std::fs::create_dir_all(maj.join("USRDIR")).unwrap();
        std::fs::write(maj.join("PARAM.SFO"), faire_sfo(&[("CATEGORY", "GD"), ("TITLE_ID", "BLES01227"), ("VERSION", "01.00"), ("APP_VER", "01.05")])).unwrap();
        assert_eq!(version_ps3(&jeu, &rpcs3, "BLES01227").as_deref(), Some("01.00"));
        // Une vraie mise à jour (EBOOT.BIN, même jeu, GD) : sa version.
        std::fs::write(maj.join("USRDIR").join("EBOOT.BIN"), b"x").unwrap();
        assert_eq!(version_ps3(&jeu, &rpcs3, "BLES01227").as_deref(), Some("01.05"));
        // Le PARAM.SFO d'un AUTRE jeu dans ce dossier : ignoré.
        std::fs::write(maj.join("PARAM.SFO"), faire_sfo(&[("CATEGORY", "GD"), ("TITLE_ID", "BLUS30721"), ("APP_VER", "01.05")])).unwrap();
        assert_eq!(version_ps3(&jeu, &rpcs3, "BLES01227").as_deref(), Some("01.00"));
        // Sans APP_VER : VERSION. Une valeur qui n'a pas la forme « 01.00 » : rien.
        std::fs::remove_dir_all(&maj).unwrap();
        std::fs::write(jeu.join("PS3_GAME").join("PARAM.SFO"), faire_sfo(&[("TITLE_ID", "BLES01227"), ("VERSION", "01.02")])).unwrap();
        assert_eq!(version_ps3(&jeu, &rpcs3, "BLES01227").as_deref(), Some("01.02"));
        std::fs::write(jeu.join("PS3_GAME").join("PARAM.SFO"), faire_sfo(&[("APP_VER", "../x")])).unwrap();
        assert_eq!(version_ps3(&jeu, &rpcs3, "BLES01227"), None);
        // Un PARAM.SFO démesuré n'est jamais lu en entier.
        std::fs::File::create(jeu.join("PS3_GAME").join("PARAM.SFO")).unwrap().set_len(1 << 30).unwrap();
        assert_eq!(titre_id_ps3(&jeu), None);
    }

    #[test]
    fn un_param_sfo_donne_l_identifiant_du_jeu() {
        // En-tête, une entrée « TITLE_ID », table des clés, table des données.
        let mut sfo = b"\0PSF".to_vec();
        sfo.extend(0x0101u32.to_le_bytes());
        let (cles, donnees) = (0x14 + 16, 0x14 + 16 + 12);
        sfo.extend((cles as u32).to_le_bytes());
        sfo.extend((donnees as u32).to_le_bytes());
        sfo.extend(1u32.to_le_bytes());
        sfo.extend(0u16.to_le_bytes());
        sfo.extend(0x0204u16.to_le_bytes());
        sfo.extend(10u32.to_le_bytes());
        sfo.extend(16u32.to_le_bytes());
        sfo.extend(0u32.to_le_bytes());
        sfo.extend(b"TITLE_ID\0\0\0\0");
        sfo.extend(b"BLES01227\0\0\0\0\0\0\0");
        assert_eq!(valeur_sfo(&sfo, "TITLE_ID").as_deref(), Some("BLES01227"));
        assert_eq!(valeur_sfo(&sfo, "TITLE"), None);
    }

    #[test]
    fn detecter_rattacher_puis_installer_avec_la_licence_du_profil() {
        use std::io::Write;
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("PS3");
        std::fs::create_dir_all(&src).unwrap();
        // Un DLC zippé avec sa licence, et un DLC isolé d'un autre jeu.
        let id = "EP9000-BCES01741_00-AVAGOWAACHILLES1";
        let mut z = zip::ZipWriter::new(std::fs::File::create(src.join("God of War - Ascension - Achilles (Europe) (Avatar).zip")).unwrap());
        z.start_file(format!("{id}.rap"), zip::write::SimpleFileOptions::default()).unwrap();
        z.write_all(&[7u8; 16]).unwrap();
        z.start_file("HLlXGojg.pkg", zip::write::SimpleFileOptions::default()).unwrap();
        z.write_all(&entete_pkg(id)).unwrap();
        z.finish().unwrap();
        std::fs::write(src.join("Asuras Wrath - Episode 11.5 (Europe) (DLC).pkg"), entete_pkg("EP0102-BLES01227_00-DLCS120000000000")).unwrap();

        let l = chercher_ps3(&src);
        assert_eq!(l.len(), 2);
        let gow = du_jeu(&l, Some("BCES01741"), "God of War - Ascension");
        assert_eq!(gow.len(), 1);
        assert!(gow[0].rap.is_some(), "la licence est rattachée par son nom");
        assert_eq!(du_jeu(&l, None, "Asura's Wrath (Europe)").len(), 1, "sans identifiant : par le titre");

        // Installation : le .pkg passe par RPCS3 (simulé), la licence va dans le compte DU PROFIL, le temporaire part.
        let rpcs3 = d.path().join("RPCS3");
        let travail = d.path().join("travail");
        let vus = std::cell::RefCell::new(Vec::new());
        installer_ps3(gow[0], &rpcs3, "00000002", &travail, &|p| {
            assert!(p.is_file(), "le .pkg extrait existe pendant l'installation");
            vus.borrow_mut().push(p.to_path_buf());
            let g = rpcs3.join("dev_hdd0").join("game").join("BCES01741");
            std::fs::create_dir_all(&g).unwrap();
            std::fs::write(g.join("PARAM.SFO"), b"x").unwrap();
            Ok(())
        })
        .unwrap();
        assert_eq!(vus.borrow().len(), 1);
        assert!(!vus.borrow()[0].exists(), "le .pkg extrait est retiré après");
        assert!(licence_posee(&rpcs3, "00000002", id) && !licence_posee(&rpcs3, "00000003", id), "licence du profil seulement");
        assert_eq!(installes(&rpcs3), vec![id.to_string()]);
        // Un 2e profil : le .pkg n'est pas réinstallé, seule sa licence est posée.
        installer_ps3(gow[0], &rpcs3, "00000003", &travail, &|_| panic!("pas de 2e installation du .pkg")).unwrap();
        assert!(licence_posee(&rpcs3, "00000003", id));
    }

    /// Le relevé RÉEL de Firehouse (God of War III, contrat 1.24) : 12 .pkg aux noms aléatoires, 6 licences.
    fn version_gow3(d: &Path) -> (Vec<(PathBuf, &'static str)>, Vec<PathBuf>) {
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/simule/version-ps3-contenus-124.json")).unwrap()).unwrap();
        let f = v["version"]["fichiers"].as_array().unwrap();
        let raps: Vec<String> = f.iter().filter(|x| x["role"] == "licence").map(|x| x["nom"].as_str().unwrap().trim_end_matches(".rap").to_string()).collect();
        let pkgs: Vec<String> = f.iter().filter(|x| x["role"] == "dlc").map(|x| x["nom"].as_str().unwrap().to_string()).collect();
        assert_eq!((pkgs.len(), raps.len()), (12, 6));
        // Les 6 premiers .pkg portent les content ID des licences (dans le désordre des noms), les 6 autres des DLC
        // gratuits du même jeu.
        let mut lp = Vec::new();
        for (i, n) in pkgs.iter().enumerate() {
            let id = if i < 6 { raps[5 - i].clone() } else { format!("EP9000-BCES00510_00-GRATUIT{i:09}") };
            std::fs::write(d.join(n), entete_pkg(&id)).unwrap();
            lp.push((d.join(n), "dlc"));
        }
        let mut lr = Vec::new();
        for r in &raps {
            std::fs::write(d.join(format!("{r}.rap")), [9u8; 16]).unwrap();
            lr.push(d.join(format!("{r}.rap")));
        }
        (lp, lr)
    }

    #[test]
    fn un_dev_hdd0_deplace_est_refuse_et_une_maj_reste_une_maj() {
        let d = tempfile::tempdir().unwrap();
        assert!(dev_hdd0_a_sa_place(d.path()), "sans vfs.yml : la place par défaut");
        std::fs::create_dir_all(d.path().join("config")).unwrap();
        std::fs::write(d.path().join("config").join("vfs.yml"), "$(EmulatorDir): \"\"\n/dev_hdd0/: $(EmulatorDir)dev_hdd0/\n").unwrap();
        assert!(dev_hdd0_a_sa_place(d.path()));
        std::fs::write(d.path().join("config").join("vfs.yml"), "/dev_hdd0/: E:/PS3/dev_hdd0/\n").unwrap();
        assert!(!dev_hdd0_a_sa_place(d.path()));
        let p = d.path().join("x.pkg");
        std::fs::write(&p, entete_pkg("EP9000-BCES00510_00-0000000000PATCH1")).unwrap();
        assert_eq!(ps3_de_la_version(&[(p, "maj")], &[])[0].genre, "maj");
    }

    #[test]
    fn l_identifiant_d_un_jeu_ps3_se_lit_dans_son_iso() {
        // Un petit ISO 9660 : PS3_GAME\PARAM.SFO (comme l'.iso déchiffré servi par Firehouse).
        let sfo = faire_sfo(&[("TITLE_ID", "BCES00510"), ("APP_VER", "01.00")]);
        let mut s = vec![vec![0u8; 2048]; 23];
        s[16][0] = 1;
        s[16][1..6].copy_from_slice(b"CD001");
        s[16][128..130].copy_from_slice(&2048u16.to_le_bytes());
        s[16][158..161].copy_from_slice(&[20, 0, 0]);
        s[16][166..170].copy_from_slice(&2048u32.to_le_bytes());
        let fiche = |nom: &str, secteur: u32, taille: u32, dossier: bool| {
            let mut r = vec![0u8; 33 + nom.len() + (nom.len() + 1) % 2];
            r[0] = r.len() as u8;
            r[2..6].copy_from_slice(&secteur.to_le_bytes());
            r[10..14].copy_from_slice(&taille.to_le_bytes());
            r[25] = if dossier { 2 } else { 0 };
            r[32] = nom.len() as u8;
            r[33..33 + nom.len()].copy_from_slice(nom.as_bytes());
            r
        };
        let r = fiche("PS3_GAME", 21, 2048, true);
        s[20][..r.len()].copy_from_slice(&r);
        let r = fiche("PARAM.SFO;1", 22, sfo.len() as u32, false);
        s[21][..r.len()].copy_from_slice(&r);
        s[22][..sfo.len()].copy_from_slice(&sfo);
        let d = tempfile::tempdir().unwrap();
        let iso = d.path().join("God of War III (Europe).iso");
        std::fs::write(&iso, s.concat()).unwrap();
        assert_eq!(titre_id_ps3(&iso).as_deref(), Some("BCES00510"));
    }

    #[test]
    fn un_paquet_illisible_est_dit_jamais_ecarte_en_silence() {
        let d = tempfile::tempdir().unwrap();
        let bon = d.path().join("bon.pkg");
        std::fs::write(&bon, entete_pkg("EP9000-BCES00510_00-0000000000DEIMOS")).unwrap();
        let tronque = d.path().join("tronque.pkg");
        std::fs::write(&tronque, b"PKG court").unwrap();
        let faux = d.path().join("faux.pkg");
        std::fs::write(&faux, vec![0u8; 0x100]).unwrap();
        let l = [(bon.clone(), "dlc"), (tronque, "dlc"), (faux, "maj"), (d.path().join("absent.pkg"), "dlc")];
        assert_eq!(illisibles(&l), vec!["tronque.pkg", "faux.pkg", "absent.pkg"]);
        assert_eq!(ps3_de_la_version(&l, &[]).len(), 1, "seul le bon devient un contenu");
    }

    #[test]
    fn une_version_firehouse_apparie_ses_licences_par_l_entete_sans_trier_par_jeu() {
        let d = tempfile::tempdir().unwrap();
        let (pkgs, raps) = version_gow3(d.path());
        let l = ps3_de_la_version(&pkgs, &raps);
        assert_eq!(l.len(), 12, "un contenu par .pkg");
        assert_eq!(l.iter().filter(|c| c.rap.is_some()).count(), 6);
        // Les licences BCES00799 (disque BCES00510) ne sont pas écartées.
        assert!(l.iter().any(|c| c.id == "EP9000-BCES00799_00-000000000DOMINUS" && c.rap.is_some() && c.nom == "DOMINUS"));
        for c in l.iter().filter(|c| c.rap.is_some()) {
            let r = c.rap.as_ref().unwrap();
            assert!(r.0.ends_with(&format!("{}.rap", c.id)), "chaque licence à son .pkg par l'en-tête");
        }
        // Une licence sans son .pkg : proposée seule, sous son nom.
        let l = ps3_de_la_version(&pkgs[6..], &raps);
        assert_eq!(l.iter().filter(|c| c.pkg.is_none() && c.genre == "licence").count(), 6);
        // Un .rap au nom qui n'est pas un content ID : ignoré (jamais un nom de fichier inventé).
        std::fs::write(d.path().join("pas une licence.rap"), [1u8; 16]).unwrap();
        assert!(ps3_de_la_version(&[], &[d.path().join("pas une licence.rap")]).is_empty());
    }

    #[test]
    fn une_licence_seule_se_pose_sans_rpcs3_et_jamais_par_dessus_une_autre() {
        let d = tempfile::tempdir().unwrap();
        let (_, raps) = version_gow3(d.path());
        let rpcs3 = d.path().join("RPCS3");
        let travail = d.path().join("travail");
        let l = ps3_de_la_version(&[], &raps);
        let c = &l[0];
        installer_ps3(c, &rpcs3, "00000001", &travail, &|_| panic!("aucun .pkg à installer")).unwrap();
        assert!(licence_posee(&rpcs3, "00000001", &c.id));
        assert!(installes(&rpcs3).is_empty(), "rien de noté pour un .pkg absent");
        // Les mêmes octets : rien à faire. D'autres : refus, l'existante est gardée.
        installer_ps3(c, &rpcs3, "00000001", &travail, &|_| panic!()).unwrap();
        let cible = rpcs3.join("dev_hdd0").join("home").join("00000002").join("exdata").join(format!("{}.rap", c.id));
        std::fs::create_dir_all(cible.parent().unwrap()).unwrap();
        std::fs::write(&cible, [1u8; 16]).unwrap();
        assert!(installer_ps3(c, &rpcs3, "00000002", &travail, &|_| panic!()).is_err());
        assert_eq!(std::fs::read(&cible).unwrap(), [1u8; 16], "la licence d'avant est intacte");
        // Un .rap qui n'a pas 16 octets n'est jamais posé.
        std::fs::write(&raps[1], [9u8; 40]).unwrap();
        let l = ps3_de_la_version(&[], &raps[1..2]);
        assert!(installer_ps3(&l[0], &rpcs3, "00000003", &travail, &|_| panic!()).is_err());
        assert!(!licence_posee(&rpcs3, "00000003", &l[0].id));
    }

    #[test]
    fn un_paquet_que_rpcs3_dit_installe_sans_rien_ecrire_n_est_pas_note() {
        let d = tempfile::tempdir().unwrap();
        let (pkgs, raps) = version_gow3(d.path());
        let rpcs3 = d.path().join("RPCS3");
        let travail = d.path().join("travail");
        let l = ps3_de_la_version(&pkgs, &raps);
        let c = l.iter().find(|c| c.rap.is_some()).unwrap();
        // Un ancien jeu déjà là, pas touché pendant l'installation.
        let ancien = rpcs3.join("dev_hdd0").join("game").join("BLES01227");
        std::fs::create_dir_all(&ancien).unwrap();
        std::fs::write(ancien.join("EBOOT.BIN"), b"x").unwrap();
        let vieux = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
        std::fs::File::options().write(true).open(ancien.join("EBOOT.BIN")).unwrap().set_modified(vieux).unwrap();
        let e = installer_ps3(c, &rpcs3, "00000001", &travail, &|_| Ok(())).unwrap_err();
        assert!(format!("{e:?}").contains("sans rien installer"), "{e:?}");
        assert!(installes(&rpcs3).is_empty(), "pas noté");
        assert!(licence_posee(&rpcs3, "00000001", &c.id), "la licence ne dépend pas du .pkg");
        // Déjà installé hors de Frogtend (le dossier du jeu existe, RPCS3 n'y change rien) : dit tel quel.
        let autre = l.iter().find(|x| x.rap.is_none()).unwrap();
        let g = rpcs3.join("dev_hdd0").join("game").join(&autre.jeu);
        std::fs::create_dir_all(&g).unwrap();
        std::fs::write(g.join("PARAM.SFO"), b"x").unwrap();
        std::fs::File::options().write(true).open(g.join("PARAM.SFO")).unwrap().set_modified(vieux).unwrap();
        let e = installer_ps3(autre, &rpcs3, "00000001", &travail, &|_| Ok(())).unwrap_err();
        assert!(format!("{e:?}").contains("hors de Frogtend"), "{e:?}");
        // Un paquet installé dans le dossier de SON jeu : noté, puis sa licence.
        installer_ps3(c, &rpcs3, "00000001", &travail, &|_| {
            let g = rpcs3.join("dev_hdd0").join("game").join(&c.jeu);
            std::fs::create_dir_all(&g).unwrap();
            std::fs::write(g.join("PARAM.SFO"), b"x").unwrap();
            Ok(())
        })
        .unwrap();
        assert_eq!(installes(&rpcs3), vec![c.id.clone()]);
        assert!(licence_posee(&rpcs3, "00000001", &c.id));
    }
}

#[cfg(test)]
mod essais {
    /// Sur le vrai dossier PS3 de Seb, en LECTURE SEULE : les contenus trouvés et leur rattachement.
    #[test]
    #[ignore]
    fn essai_contenus_ps3_reels() {
        let debut = std::time::Instant::now();
        let l = super::chercher_ps3(std::path::Path::new("E:/Games/Playstation 3"));
        let avec_licence = l.iter().filter(|c| c.rap.is_some()).count();
        println!("{} contenus ({} avec licence) en {:?}", l.len(), avec_licence, debut.elapsed());
        let mut par_jeu: std::collections::BTreeMap<&str, usize> = Default::default();
        for c in &l {
            *par_jeu.entry(c.jeu.as_str()).or_default() += 1;
        }
        println!("par jeu : {par_jeu:?}");
        for (jeu, titre) in [("BLES01227", "Asura's Wrath (Europe)"), ("BCES01741", "God of War - Ascension")] {
            println!("{titre} : {} contenu(s)", super::du_jeu(&l, Some(jeu), titre).len());
        }
    }
}

#[cfg(test)]
mod essais_import {
    /// Sur le vrai dossier PS3 de Seb, en LECTURE SEULE : à l'import, combien de jeux et combien de zips de contenus.
    #[test]
    #[ignore]
    fn essai_import_ps3_reel() {
        let l = crate::import_local::chercher_roms(std::path::Path::new("E:/Games/Playstation 3"), &["zip".into()], false).unwrap();
        let (contenus, jeux): (Vec<_>, Vec<_>) = l.into_iter().partition(|r| super::zip_de_contenus(std::path::Path::new(&r.chemin)));
        println!("{} jeu(x), {} contenu(s) écarté(s)", jeux.len(), contenus.len());
        for j in &jeux {
            println!("  jeu : {}", j.titre);
        }
    }
}

#[cfg(test)]
mod essais_switch {
    /// Sur le vrai dossier Switch de Seb, en LECTURE SEULE : les mises à jour et DLC trouvés.
    #[test]
    #[ignore]
    fn essai_contenus_switch_reels() {
        let debut = std::time::Instant::now();
        let l = super::chercher_switch(std::path::Path::new("D:/LaunchBox/Games/Nintendo Switch Maj & DLC"));
        println!("{} contenu(s) en {:?}", l.len(), debut.elapsed());
        for c in &l {
            println!("  {} | {} | jeu {} | {}", c.genre, c.id, c.jeu, c.nom);
        }
        println!("Dragon Quest Builders 2 : {}", super::switch_du_jeu(&l, None, "Dragon Quest Builders 2").len());
        println!("Fire Emblem Three Houses : {}", super::switch_du_jeu(&l, None, "Fire Emblem Three Houses").len());
    }
}

#[cfg(test)]
mod essais_wua {
    /// Sur les vrais .wua de Seb, en LECTURE SEULE : leurs titres (jeu, mises à jour, DLC).
    #[test]
    #[ignore]
    fn essai_wua_reels() {
        let mut l: Vec<_> = std::fs::read_dir("E:/Games/Nintendo Wii U").unwrap().flatten().map(|e| e.path()).collect();
        l.sort();
        let (mut avec_maj, mut avec_dlc, mut vides) = (0, 0, 0);
        let debut = std::time::Instant::now();
        for (i, p) in l.iter().filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("wua"))).enumerate() {
            let t = super::titres_wua(p).unwrap();
            if t.is_empty() {
                vides += 1;
            }
            if t.iter().any(|x| x.genre == "maj") {
                avec_maj += 1;
            }
            if t.iter().any(|x| x.genre == "dlc") {
                avec_dlc += 1;
            }
            if i < 5 {
                println!("{} : {:?}", p.file_name().unwrap().to_string_lossy(), t.iter().map(|x| format!("{} v{}", x.genre, x.version)).collect::<Vec<_>>());
            }
        }
        println!("{avec_maj} avec mise à jour, {avec_dlc} avec DLC, {vides} illisible(s), en {:?}", debut.elapsed());
    }
}
