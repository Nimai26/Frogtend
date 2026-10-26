//! Cemu (Wii U) : un COMPTE Wii U par personne (décision de Seb, 26/10 : « Séparer »), dans le mlc commun (les jeux
//! installés y restent, jamais copiés). Relevé dans le code source de Cemu (commit 5f10237, 26/10/2026) :
//! - un compte = `<mlc>\usr\save\system\act\<id 8 hexa minuscules>\account.dat`, texte `clé=valeur` (Account.cpp
//!   L194-222) ; valide si id ≥ 0x80000001, `MiiName` non vide (11 caractères UTF-16 = 44 hexa) et `MiiData` (96
//!   octets) non nulle (CheckValid L134) — un compte invalide est ignoré EN SILENCE ;
//! - le Mii par défaut et son CRC16 : Account.cpp L83-112 (refait ici à l'octet près) ;
//! - nouvel id : max(`PersistentIdHead` de `persisid.dat` + 1, plus grand compte + 1) (L383-412) ; 12 comptes au plus ;
//! - `-a <id hexa>` (LaunchSettings.cpp L175) n'est jamais écrit dans settings.xml ; un id INCONNU fait jouer le
//!   premier compte en silence (L367) : Frogtend crée le compte AVANT, Cemu fermé ;
//! - mlc : `mlc_path` de `portable\settings.xml` s'il est réglé, sinon `portable\mlc01` (ActiveSettings.cpp L276) ;
//! - parties : `<mlc>\usr\save\<haut>\<bas>\user\<id>\` ; `user\common\` est commun à tous (comme la vraie Wii U).

use crate::erreurs::{Erreur, Resultat};
use std::path::{Path, PathBuf};

/// Le plus petit identifiant de compte permis.
const ID_MIN: u32 = 0x8000_0001;
/// Le nombre de comptes au plus.
const MAX: usize = 12;
/// Dans une sauvegarde, le dossier (sous celui de l'émulateur) des parties du compte Cemu du profil.
pub const COMPTE_CEMU: &str = ".cemu-compte";

/// Le compte Cemu d'un profil, prêt pour une partie.
#[derive(Debug, Clone, PartialEq)]
pub struct CompteDuProfil {
    pub id: u32,
    pub cree: bool,
    /// Créé à l'instant : le compte que Cemu prenait JUSQUE-LÀ (ses parties d'avant Frogtend), s'il y en avait un.
    pub ancien_courant: Option<u32>,
}

/// Le programme est-il Cemu ?
pub fn est_cemu(programme: &str) -> bool {
    Path::new(programme).file_name().is_some_and(|n| n.to_string_lossy().to_ascii_lowercase().starts_with("cemu"))
}

/// Cemu installé en portable (son dossier `portable` à côté de lui, comme Frogtend l'installe) ? Sinon, ses données
/// sont dans `%APPDATA%` : Frogtend n'y écrit jamais.
pub fn portable(cemu: &Path) -> bool {
    cemu.join("portable").is_dir()
}

fn reglages(cemu: &Path) -> String {
    std::fs::read_to_string(cemu.join("portable").join("settings.xml")).unwrap_or_default()
}

/// La valeur d'une balise XML simple (`<nom>valeur</nom>`), à partir de `depuis`.
fn balise<'a>(xml: &'a str, nom: &str, depuis: usize) -> Option<&'a str> {
    let ouvre = format!("<{nom}>");
    let i = xml.get(depuis..)?.find(&ouvre)? + depuis + ouvre.len();
    let j = xml[i..].find(&format!("</{nom}>"))? + i;
    Some(xml[i..j].trim())
}

/// Un texte XML décodé (`&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;`).
fn decoder_xml(t: &str) -> String {
    t.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

/// Le mlc de ce Cemu : celui réglé par la personne dans Cemu, sinon `portable\mlc01`.
pub fn mlc(cemu: &Path) -> PathBuf {
    match balise(&reglages(cemu), "mlc_path", 0).filter(|m| !m.is_empty()) {
        Some(m) => PathBuf::from(decoder_xml(m)),
        None => cemu.join("portable").join("mlc01"),
    }
}

/// Le mlc, seulement s'il est sûr d'y écrire : celui de Cemu par défaut, ou un dossier réglé par la personne qui
/// EXISTE (chemin complet). Jamais un dossier inventé ailleurs (règle « pas de pieuvre »).
fn mlc_sur(cemu: &Path) -> Resultat<PathBuf> {
    let m = mlc(cemu);
    if m.starts_with(cemu.join("portable")) || (m.is_absolute() && m.is_dir()) {
        Ok(m)
    } else {
        Err(Erreur::Prevenir(format!(
            "Cemu est réglé pour ranger ses données dans « {} », qui n'existe pas (disque débranché ?) : Frogtend n'y crée rien. Branche ce disque, ou corrige le dossier dans Cemu, puis réessaie.",
            m.display()
        )))
    }
}

/// La table profil → compte que Frogtend tient pour ce Cemu (`Profils\.comptes-frogtend`, « id<TAB>profil » par
/// ligne) : deux profils dont les noms se ressemblent après la coupe à 10 caractères n'ont jamais le même compte.
fn table(cemu: &Path) -> Vec<(u32, String)> {
    std::fs::read_to_string(cemu.join("Profils").join(".comptes-frogtend"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let (i, p) = l.split_once('\t')?;
            Some((u32::from_str_radix(i, 16).ok()?, p.to_string()))
        })
        .collect()
}

fn noter(cemu: &Path, id: u32, profil: &str) -> Resultat<()> {
    let mut t = table(cemu);
    t.retain(|(_, p)| p != profil);
    t.push((id, profil.to_string()));
    std::fs::create_dir_all(cemu.join("Profils"))?;
    let texte: String = t.iter().map(|(i, p)| format!("{i:08x}\t{p}\n")).collect();
    std::fs::write(cemu.join("Profils").join(".comptes-frogtend"), texte)?;
    Ok(())
}

/// Le compte de ce profil parmi `l` : celui que la table lui attribue (s'il existe encore), sinon un compte à son nom
/// qui n'est attribué à AUCUN autre profil (créé à la main, ou d'avant la table).
fn trouver(cemu: &Path, l: &[(u32, String)], profil: &str) -> Option<u32> {
    let t = table(cemu);
    if let Some((id, _)) = t.iter().find(|(i, p)| p == profil && l.iter().any(|(x, _)| x == i)) {
        return Some(*id);
    }
    let nom = String::from_utf16_lossy(&nom_cemu(profil));
    l.iter().find(|(i, n)| *n == nom && !t.iter().any(|(x, p)| x == i && p != profil)).map(|(i, _)| *i)
}

/// Le compte choisi dans les réglages de Cemu (`<Account><PersistentId>`), s'il est lisible.
fn compte_regle(cemu: &Path) -> Option<u32> {
    let x = reglages(cemu);
    let i = x.find("<Account>")?;
    let v = balise(&x, "PersistentId", i)?;
    v.parse::<u32>().ok().or_else(|| u32::from_str_radix(v.trim_start_matches("0x"), 16).ok())
}

fn act(mlc: &Path) -> PathBuf {
    mlc.join("usr").join("save").join("system").join("act")
}

/// Le nom tel que Cemu le garde : 10 unités UTF-16 au plus (jamais une moitié de caractère).
pub fn nom_cemu(nom: &str) -> Vec<u16> {
    let mut u: Vec<u16> = Vec::new();
    for c in nom.chars() {
        let mut b = [0u16; 2];
        let e = c.encode_utf16(&mut b);
        if u.len() + e.len() > 10 {
            break;
        }
        u.extend_from_slice(e);
    }
    u
}

/// Le CRC16 du Mii, tel que Cemu le calcule (Account.cpp `FFLCalculateCRC16`).
fn crc16(o: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &b in o {
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x1021 } else { crc << 1 };
        }
        crc ^= b as u16;
    }
    crc
}

/// Le Mii par défaut d'un compte, octet pour octet comme Cemu (Account.cpp L83-112) : mots à 1, deux champs fixés,
/// identifiant bas 0x33334, le nom à 0x1A, puis un compteur (octets 4-7) jusqu'à un CRC16 nul.
pub fn mii(nom: &[u16]) -> [u8; 96] {
    let mut m = [0u8; 96];
    for i in 0..48 {
        m[2 * i..2 * i + 2].copy_from_slice(&[0x01, 0x00]);
    }
    m[0x02..0x04].copy_from_slice(&[0x01, 0x10]);
    m[0x3A..0x3C].copy_from_slice(&[0x01, 0x06]);
    m[0x08..0x0C].copy_from_slice(&[0x03, 0x00, 0x34, 0x33]);
    m[0x1A..0x2E].fill(0);
    for (i, c) in nom.iter().take(10).enumerate() {
        m[0x1A + 2 * i..0x1A + 2 * i + 2].copy_from_slice(&c.to_le_bytes());
    }
    let mut compteur: u32 = 0;
    while crc16(&m) != 0 {
        m[4..8].copy_from_slice(&compteur.to_be_bytes());
        compteur = compteur.wrapping_add(1);
    }
    m
}

fn hexa(o: &[u8]) -> String {
    o.iter().map(|b| format!("{b:02x}")).collect()
}

/// Le contenu d'un `account.dat` (les clés et l'ordre de Cemu ; fins de ligne CRLF, comme Cemu sous Windows).
pub fn account_dat(id: u32, nom: &str, uuid: &[u8; 16]) -> String {
    let n = nom_cemu(nom);
    let mut nom11 = [0u16; 11];
    nom11[..n.len()].copy_from_slice(&n);
    let base = (0x2000004u64 << 32) | u32::from_be_bytes([uuid[12], uuid[13], uuid[14], uuid[15]]) as u64;
    let lignes = [
        "AccountInstance_20120705".to_string(),
        format!("PersistentId={id:08x}"),
        format!("TransferableIdBase={base:x}"),
        format!("Uuid={}", hexa(uuid)),
        format!("MiiData={}", hexa(&mii(&n))),
        format!("MiiName={}", nom11.iter().map(|c| format!("{c:04x}")).collect::<String>()),
        "AccountId=".into(),
        "BirthYear=0".into(),
        "BirthMonth=0".into(),
        "BirthDay=0".into(),
        "Gender=0".into(),
        "EmailAddress=".into(),
        "Country=0".into(),
        "SimpleAddressId=0".into(),
        "PrincipalId=0".into(),
        "IsPasswordCacheEnabled=0".into(),
        format!("AccountPasswordCache={}", "0".repeat(64)),
    ];
    lignes.iter().map(|l| format!("{l}\r\n")).collect()
}

/// Le nom d'un compte d'après son `account.dat` (clé `MiiName`), s'il est lisible.
fn nom_du_compte(dat: &str) -> Option<String> {
    let h = dat.lines().find_map(|l| l.trim_end_matches('\r').strip_prefix("MiiName="))?;
    if h.len() != 44 {
        return None;
    }
    let u: Vec<u16> = (0..11).map_while(|i| u16::from_str_radix(&h[4 * i..4 * i + 4], 16).ok()).take_while(|c| *c != 0).collect();
    Some(String::from_utf16_lossy(&u))
}

/// Les comptes d'un mlc : (id, nom), par id.
pub fn comptes(mlc: &Path) -> Vec<(u32, String)> {
    let mut l: Vec<(u32, String)> = std::fs::read_dir(act(mlc))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            let id = (n.len() == 8).then(|| u32::from_str_radix(&n, 16).ok()).flatten().filter(|i| *i >= ID_MIN)?;
            let dat = std::fs::read_to_string(e.path().join("account.dat")).ok()?;
            let mii = dat.lines().find_map(|l| l.trim_end_matches('\r').strip_prefix("MiiData="))?;
            if mii.len() != 192 || mii.chars().all(|c| c == '0') {
                return None; // Cemu l'ignore : jamais choisi
            }
            let nom = nom_du_compte(&dat)?;
            (!nom.is_empty()).then_some((id, nom))
        })
        .collect();
    l.sort();
    l
}

/// Le compte Cemu de ce profil, créé s'il manque. Cemu doit être FERMÉ : l'appelant le vérifie. Seul un dossier de
/// compte NEUF est écrit (`account.dat` provisoire puis renommé, relu) ; aucun autre fichier de Cemu n'est touché.
pub fn compte_du_profil(cemu: &Path, profil: &str) -> Resultat<CompteDuProfil> {
    let mlc = mlc_sur(cemu)?;
    let l = comptes(&mlc);
    let nom = String::from_utf16_lossy(&nom_cemu(profil));
    if let Some(id) = trouver(cemu, &l, profil) {
        if !table(cemu).iter().any(|(i, p)| *i == id && p == profil) {
            noter(cemu, id, profil)?;
        }
        return Ok(CompteDuProfil { id, cree: false, ancien_courant: None });
    }
    if l.len() >= MAX {
        return Err(Erreur::Prevenir(
            "Cemu a déjà 12 comptes (le maximum de la Wii U) : Frogtend ne peut pas en créer un pour ce profil. Retire un compte dans Cemu (Options ▸ Réglages généraux ▸ Compte), puis réessaie.".into(),
        ));
    }
    let tete = std::fs::read_to_string(act(&mlc).join("persisid.dat"))
        .ok()
        .and_then(|t| t.lines().find_map(|x| x.trim().strip_prefix("PersistentIdHead=").and_then(|v| u32::from_str_radix(v.trim(), 16).ok())));
    // Les dossiers de compte existants, même illisibles : jamais réutilisés.
    let plus_grand = std::fs::read_dir(act(&mlc))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| u32::from_str_radix(&e.file_name().to_string_lossy(), 16).ok())
        .max();
    let id = [Some(ID_MIN), tete.map(|t| t.saturating_add(1)), plus_grand.map(|m| m.saturating_add(1))].into_iter().flatten().max().unwrap_or(ID_MIN);
    let dossier = act(&mlc).join(format!("{id:08x}"));
    if dossier.exists() {
        return Err(Erreur::Disque("Le compte Cemu de ce profil n'a pas pu être créé (numéro déjà pris).".into()));
    }
    let uuid: [u8; 16] = rand::random();
    std::fs::create_dir_all(&dossier)?;
    let provisoire = dossier.join("account.dat.frogtend");
    std::fs::write(&provisoire, account_dat(id, &nom, &uuid))?;
    std::fs::rename(&provisoire, dossier.join("account.dat"))?;
    // Vérifier le résultat : relu, le compte est là, valide, à ce nom.
    if !comptes(&mlc).iter().any(|(i, n)| *i == id && *n == nom) {
        return Err(Erreur::Disque("Le compte Cemu de ce profil n'a pas pu être enregistré.".into()));
    }
    noter(cemu, id, profil)?;
    // La reprise des parties d'avant Frogtend : seulement à la TOUTE PREMIÈRE création de compte par Frogtend sur ce
    // Cemu (au lancement OU à la restauration : la marque est posée ici), depuis le compte que Cemu prenait jusque-là —
    // jamais un compte créé par Frogtend pour une autre personne.
    let marque = cemu.join("Profils").join(".reprise-cemu");
    let ancien_courant = if marque.exists() {
        None
    } else {
        std::fs::write(&marque, profil)?;
        compte_regle(cemu).filter(|c| l.iter().any(|(i, _)| i == c)).or_else(|| l.first().map(|(i, _)| *i))
    };
    Ok(CompteDuProfil { id, cree: true, ancien_courant })
}

/// Le compte Cemu de ce profil s'il existe, SANS rien créer (pour la sauvegarde).
pub fn compte_existant(cemu: &Path, profil: &str) -> Option<u32> {
    trouver(cemu, &comptes(&mlc(cemu)), profil)
}

/// Les parties d'un compte, jeu par jeu : (`<haut>/<bas>`, dossier `user\<id>`).
pub fn parties_du_compte(cemu: &Path, id: u32) -> Vec<(String, PathBuf)> {
    let save = mlc(cemu).join("usr").join("save");
    let mut l = Vec::new();
    for haut in std::fs::read_dir(&save).into_iter().flatten().flatten() {
        let h = haut.file_name().to_string_lossy().to_string();
        if h.len() != 8 || h == "system" {
            continue;
        }
        for bas in std::fs::read_dir(haut.path()).into_iter().flatten().flatten() {
            let d = bas.path().join("user").join(format!("{id:08x}"));
            if d.is_dir() {
                l.push((format!("{h}/{}", bas.file_name().to_string_lossy()), d));
            }
        }
    }
    l.sort();
    l
}

/// Le dossier `user\<id>` d'un jeu (`<haut>/<bas>`) pour un compte.
pub fn dossier_du_jeu(cemu: &Path, jeu: &str, id: u32) -> PathBuf {
    let mut p = mlc(cemu).join("usr").join("save");
    for morceau in jeu.split('/') {
        p = p.join(morceau);
    }
    p.join("user").join(format!("{id:08x}"))
}

/// Décision de Seb (26/10) : les parties qu'avait le compte que Cemu prenait AVANT Frogtend sont COPIÉES dans le compte
/// de la PREMIÈRE personne qui joue — `c.ancien_courant`, rempli seulement à la toute première création de compte par
/// Frogtend sur ce Cemu ; jamais déplacées, jamais par-dessus. Rend le nombre de jeux copiés.
pub fn reprendre_parties_d_avant(cemu: &Path, c: &CompteDuProfil) -> Resultat<usize> {
    let mut copies = 0;
    if let Some(source) = c.ancien_courant.filter(|s| *s != c.id) {
        for (jeu, de) in parties_du_compte(cemu, source) {
            let vers = dossier_du_jeu(cemu, &jeu, c.id);
            if !vers.exists() {
                crate::eden::copier_dossier_entier(&de, &vers)?;
                copies += 1;
            }
        }
    }
    Ok(copies)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_mii_par_defaut_est_celui_de_cemu() {
        let n = nom_cemu("default");
        let m = mii(&n);
        assert_eq!(crc16(&m), 0, "le CRC16 du Mii est nul, comme Cemu l'exige");
        assert_eq!(&m[0x1A..0x1A + 14], &[b'd', 0, b'e', 0, b'f', 0, b'a', 0, b'u', 0, b'l', 0, b't', 0]);
        assert_eq!(&m[0..4], &[0x01, 0x00, 0x01, 0x10]);
        // Relevé RÉEL (26/10, lecture seule) : le Mii du compte « default » créé par Cemu sur Venkman — octet pour octet.
        let reel = "010001100000d73e030034330100010001000100010001000100640065006600610075006c0074000000000000000100010001000100010001000106010001000100010001000100010001000100010001000100010001000100010001000100";
        assert_eq!(hexa(&m), reel);
        assert_eq!(&m[0x08..0x0C], &[0x03, 0x00, 0x34, 0x33]);
        assert_eq!(&m[0x3A..0x3C], &[0x01, 0x06]);
        assert!(m.iter().any(|b| *b != 0));
        // 10 unités UTF-16 au plus, jamais une moitié de caractère.
        assert_eq!(nom_cemu("Sébastien Lagarde").len(), 10);
        assert_eq!(nom_cemu(&"😀".repeat(6)).len(), 10);
    }

    #[test]
    fn le_fichier_du_compte_a_la_forme_de_cemu() {
        let uuid: [u8; 16] = core::array::from_fn(|i| i as u8);
        let t = account_dat(0x8000_0002, "Léa", &uuid);
        assert!(t.starts_with("AccountInstance_20120705\r\nPersistentId=80000002\r\n"));
        assert!(t.contains("TransferableIdBase=20000040c0d0e0f\r\n"));
        assert!(t.contains("Uuid=000102030405060708090a0b0c0d0e0f\r\n"));
        let mii = t.lines().find_map(|l| l.strip_prefix("MiiData=")).unwrap();
        assert_eq!(mii.len(), 192);
        let nom = t.lines().find_map(|l| l.strip_prefix("MiiName=")).unwrap();
        assert_eq!(nom, format!("004c00e90061{}", "0000".repeat(8)));
        assert_eq!(nom_du_compte(&t).as_deref(), Some("Léa"));
    }

    fn cemu_portable() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("portable")).unwrap();
        d
    }

    #[test]
    fn chaque_profil_a_son_compte_cree_une_fois() {
        let d = cemu_portable();
        let cemu = d.path();
        // Le compte « default » que Cemu crée lui-même.
        let def = act(&mlc(cemu)).join("80000001");
        std::fs::create_dir_all(&def).unwrap();
        std::fs::write(def.join("account.dat"), account_dat(0x8000_0001, "default", &[1; 16])).unwrap();
        std::fs::write(act(&mlc(cemu)).join("persisid.dat"), "PersistentIdManager_20120607\nPersistentIdHead=80000004\n").unwrap();
        let seb = compte_du_profil(cemu, "Seb").unwrap();
        assert_eq!((seb.id, seb.cree, seb.ancien_courant), (0x8000_0005, true, Some(0x8000_0001)), "après la tête de persisid.dat");
        assert_eq!(compte_du_profil(cemu, "Seb").unwrap(), CompteDuProfil { id: 0x8000_0005, cree: false, ancien_courant: None });
        let lea = compte_du_profil(cemu, "Léa").unwrap();
        assert_eq!(lea.id, 0x8000_0006);
        assert_eq!(compte_existant(cemu, "Léa"), Some(0x8000_0006));
        assert_eq!(compte_existant(cemu, "Zoé"), None);
        assert_eq!(std::fs::read(def.join("account.dat")).unwrap(), account_dat(0x8000_0001, "default", &[1; 16]).into_bytes(), "jamais touché");
        assert!(!act(&mlc(cemu)).join("80000005").join("account.dat.frogtend").exists());
    }

    #[test]
    fn apres_une_restauration_la_personne_suivante_ne_recoit_jamais_les_parties_d_une_autre() {
        // PC neuf : Cemu n'a jamais tourné ; la restauration crée le compte de Seb (avec ses parties).
        let d = cemu_portable();
        let cemu = d.path();
        let seb = compte_du_profil(cemu, "Seb").unwrap();
        assert_eq!(seb.ancien_courant, None, "rien d'avant Frogtend");
        let p = dossier_du_jeu(cemu, "00050000/101c9500", seb.id);
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(p.join("save.dat"), b"zelda de Seb").unwrap();
        // Léa joue ensuite : jamais les parties de Seb.
        let lea = compte_du_profil(cemu, "Léa").unwrap();
        assert_eq!(lea.ancien_courant, None);
        assert_eq!(reprendre_parties_d_avant(cemu, &lea).unwrap(), 0);
        assert!(!dossier_du_jeu(cemu, "00050000/101c9500", lea.id).exists());
    }

    #[test]
    fn deux_profils_aux_noms_coupes_pareil_ont_chacun_leur_compte() {
        let d = cemu_portable();
        let a = compte_du_profil(d.path(), "Christophe1").unwrap();
        let b = compte_du_profil(d.path(), "Christophe2").unwrap();
        assert_ne!(a.id, b.id);
        assert_eq!(compte_existant(d.path(), "Christophe1"), Some(a.id));
        assert_eq!(compte_existant(d.path(), "Christophe2"), Some(b.id));
        assert_eq!(compte_du_profil(d.path(), "Christophe2").unwrap().id, b.id);
    }

    #[test]
    fn un_compte_que_cemu_ignorerait_n_est_jamais_choisi() {
        let d = cemu_portable();
        let c = act(&mlc(d.path())).join("80000001");
        std::fs::create_dir_all(&c).unwrap();
        let t = account_dat(0x8000_0001, "Seb", &[1; 16]);
        let mii = t.lines().find_map(|l| l.strip_prefix("MiiData=")).unwrap().to_string();
        std::fs::write(c.join("account.dat"), t.replace(&mii, &"0".repeat(192))).unwrap();
        assert!(comptes(&mlc(d.path())).is_empty());
        // Un compte valide est créé à côté (jamais par-dessus le dossier existant).
        assert_eq!(compte_du_profil(d.path(), "Seb").unwrap().id, 0x8000_0002);
    }

    #[test]
    fn un_mlc_regle_absent_est_refuse_et_son_chemin_est_decode() {
        let d = cemu_portable();
        let absent = d.path().join("Disque débranché").join("mlc01");
        std::fs::write(d.path().join("portable").join("settings.xml"), format!("<content><mlc_path>{}</mlc_path></content>", absent.display())).unwrap();
        assert!(matches!(compte_du_profil(d.path(), "Seb"), Err(Erreur::Prevenir(_))));
        assert!(!absent.exists(), "rien n'est créé ailleurs");
        let reel = d.path().join("Jeux & co");
        std::fs::create_dir_all(&reel).unwrap();
        std::fs::write(d.path().join("portable").join("settings.xml"), format!("<content><mlc_path>{}</mlc_path></content>", reel.display().to_string().replace('&', "&amp;"))).unwrap();
        assert_eq!(mlc(d.path()), reel);
        assert!(compte_du_profil(d.path(), "Seb").is_ok());
    }

    #[test]
    fn le_mlc_regle_dans_cemu_est_suivi_et_douze_comptes_au_plus() {
        let d = cemu_portable();
        let autre = d.path().join("Wii U");
        std::fs::write(d.path().join("portable").join("settings.xml"), format!("<content>\n<mlc_path>{}</mlc_path>\n<Account>\n<PersistentId>2147483650</PersistentId>\n</Account>\n</content>", autre.display())).unwrap();
        assert_eq!(mlc(d.path()), autre);
        assert_eq!(compte_regle(d.path()), Some(0x8000_0002));
        for i in 0..12u32 {
            let c = act(&autre).join(format!("{:08x}", ID_MIN + i));
            std::fs::create_dir_all(&c).unwrap();
            std::fs::write(c.join("account.dat"), account_dat(ID_MIN + i, &format!("P{i}"), &[2; 16])).unwrap();
        }
        assert!(matches!(compte_du_profil(d.path(), "Seb"), Err(Erreur::Prevenir(_))));
        assert_eq!(compte_du_profil(d.path(), "P3").unwrap().id, ID_MIN + 3);
    }

    #[test]
    fn les_parties_d_avant_vont_a_la_premiere_personne_qui_joue_une_seule_fois() {
        let d = cemu_portable();
        let cemu = d.path();
        let def = act(&mlc(cemu)).join("80000001");
        std::fs::create_dir_all(&def).unwrap();
        std::fs::write(def.join("account.dat"), account_dat(0x8000_0001, "default", &[1; 16])).unwrap();
        for (jeu, o) in [("00050000/101c9500", &b"zelda"[..]), ("00050000/10145000", b"mario")] {
            let p = dossier_du_jeu(cemu, jeu, 0x8000_0001);
            std::fs::create_dir_all(&p).unwrap();
            std::fs::write(p.join("save.dat"), o).unwrap();
        }
        let seb = compte_du_profil(cemu, "Seb").unwrap();
        // Seb avait déjà Mario dans son compte : jamais écrasé.
        let mario = dossier_du_jeu(cemu, "00050000/10145000", seb.id);
        std::fs::create_dir_all(&mario).unwrap();
        std::fs::write(mario.join("save.dat"), b"mario de Seb").unwrap();
        assert_eq!(reprendre_parties_d_avant(cemu, &seb).unwrap(), 1);
        assert_eq!(std::fs::read(dossier_du_jeu(cemu, "00050000/101c9500", seb.id).join("save.dat")).unwrap(), b"zelda");
        assert_eq!(std::fs::read(mario.join("save.dat")).unwrap(), b"mario de Seb");
        assert!(dossier_du_jeu(cemu, "00050000/101c9500", 0x8000_0001).join("save.dat").is_file(), "copiées, jamais déplacées");
        assert_eq!(parties_du_compte(cemu, seb.id).len(), 2);
        let lea = compte_du_profil(cemu, "Léa").unwrap();
        assert_eq!(reprendre_parties_d_avant(cemu, &lea).unwrap(), 0, "une seule fois par Cemu");
    }
}
