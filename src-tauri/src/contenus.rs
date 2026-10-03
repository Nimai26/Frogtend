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
    /// Le .pkg : fichier, et son nom DANS le zip s'il est zippé.
    pub pkg: (String, Option<String>),
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

/// L'identifiant d'un jeu PS3 (TITLE_ID de son PARAM.SFO) : un .iso/.chd, ou un dossier de jeu.
pub fn titre_id_ps3(chemin: &Path) -> Option<String> {
    let ext = chemin.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let sfo = if ext == "iso" || ext == "chd" {
        let p = crate::disque::ouvrir(chemin).ok()??;
        let (s, t) = p.trouver("PS3_GAME\\PARAM.SFO").ok()??;
        p.lire(s, t as usize).ok()?
    } else {
        let d = if chemin.is_dir() { chemin.to_path_buf() } else { chemin.parent()?.to_path_buf() };
        [d.join("PS3_GAME").join("PARAM.SFO"), d.join("PARAM.SFO"), d.parent()?.join("PARAM.SFO")].iter().find_map(|f| std::fs::read(f).ok())?
    };
    valeur_sfo(&sfo, "TITLE_ID")
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
                            pkg: (chemin, None),
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
                                    pkg: (chemin.clone(), Some(n.clone())),
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

// --- Switch (Eden) : mises à jour et DLC en .nsp. Relevé dans les sources d'Eden (03/10) : `[Paths]` de
// `user\config\qt-config.ini` a une liste `external_content_dirs` (`external_content_dirs\size`,
// `external_content_dirs\N\path`) de dossiers où Eden LIT les mises à jour et DLC, sans rien installer. « Installer »
// = ajouter le dossier à cette liste (avec l'accord de la personne) ; rien n'est copié. ---

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
        (r.len() == 32).then(|| u64::from_str_radix(&r[..16], 16).ok()).flatten()
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

/// Les dossiers de contenus externes réglés dans Eden.
pub fn dossiers_externes_eden(texte: &str) -> Vec<String> {
    let lire = |cle: &str| crate::manettes::lire_ini(texte, "Paths", cle).into_iter().next();
    let n: usize = lire("external_content_dirs\\size").and_then(|v| v.trim().parse().ok()).unwrap_or(0);
    (1..=n.min(256)).filter_map(|i| lire(&format!("external_content_dirs\\{i}\\path"))).filter(|v| !v.is_empty()).collect()
}

/// Ajoute des dossiers à la liste d'Eden (sans doublon), en gardant tout le reste du fichier.
pub fn ajouter_dossiers_eden(texte: &str, dossiers: &[String]) -> String {
    let mut l = dossiers_externes_eden(texte);
    for d in dossiers {
        if !l.iter().any(|x| x.eq_ignore_ascii_case(d)) {
            l.push(d.clone());
        }
    }
    let mut valeurs: Vec<(String, Vec<String>)> = vec![("external_content_dirs\\size".into(), vec![l.len().to_string()])];
    for (i, d) in l.iter().enumerate() {
        valeurs.push((format!("external_content_dirs\\{}\\path", i + 1), vec![d.clone()]));
    }
    let refs: Vec<(&str, Vec<String>)> = valeurs.iter().map(|(c, v)| (c.as_str(), v.clone())).collect();
    crate::emulateurs_profils::ecrire_ini(texte, "Paths", &refs)
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
    if !deja.contains(&c.id) {
        let (pkg, temporaire) = sortir(&c.pkg, travail)?;
        let r = installer_pkg(&pkg);
        if temporaire {
            let _ = std::fs::remove_file(&pkg);
        }
        r?;
        let mut l = deja;
        l.push(c.id.clone());
        std::fs::create_dir_all(rpcs3)?;
        std::fs::write(registre(rpcs3), serde_json::to_vec_pretty(&l).unwrap_or_default())?;
    }
    if let Some(rap) = &c.rap {
        let (fichier, temporaire) = sortir(rap, travail)?;
        let dossier = rpcs3.join("dev_hdd0").join("home").join(compte).join("exdata");
        std::fs::create_dir_all(&dossier)?;
        let cible = dossier.join(format!("{}.rap", c.id));
        let r = std::fs::copy(&fichier, &cible);
        if temporaire {
            let _ = std::fs::remove_file(&fichier);
        }
        r?;
        if std::fs::metadata(&cible).map(|m| m.len()).unwrap_or(0) < 0x10 {
            return Err(Erreur::Disque("La licence .rap posée est trop courte : ce n'est pas une licence.".into()));
        }
    }
    Ok(())
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
        let avant = "[Paths]\r\ngamedirs\\size=1\r\nexternal_content_dirs\\size=1\r\nexternal_content_dirs\\1\\path=E:\\DLC\r\n[UI]\r\ntheme=dark\r\n";
        assert_eq!(dossiers_externes_eden(avant), ["E:\\DLC"]);
        let apres = ajouter_dossiers_eden(avant, &["D:\\Switch Maj & DLC".into(), "e:\\dlc".into()]);
        assert_eq!(dossiers_externes_eden(&apres), ["E:\\DLC", "D:\\Switch Maj & DLC"], "sans doublon");
        assert!(apres.contains("gamedirs\\size") && apres.contains("theme=dark"), "le reste est gardé");
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
