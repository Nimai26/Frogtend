//! Lire la 1re piste d'une image de CD (`.cue`/`.bin`, `.ccd`/`.img` de CloneCD, `.iso`), pour l'empreinte
//! RetroAchievements des jeux sur CD. LECTURE SEULE. Reprend la logique officielle de `rcheevos`
//! (`src/rhash/cdreader.c` : taille de secteur devinée par la synchro et « CD001 » au secteur 16 ;
//! `src/rhash/hash_disc.c` : recherche d'un fichier ISO 9660, méthodes Sega CD / Saturn et PlayStation).
//! Les `.chd` (compressés, MAME) sont lus par la bibliothèque `chd` (chd-rs, Rust pur, BSD-3) : pistes d'après les
//! métadonnées `CHT2`/`CHGD` (blocs de 2448 octets par secteur : 2352 de données + 96 de sous-code, pistes alignées
//! sur 4 secteurs), adresse absolue d'un secteur lue dans son en-tête (comme `cdreader.c`) ; Dreamcast
//! (rc_hash_dreamcast) : `IP.BIN` de la piste 3 puis le programme de démarrage.

use crate::erreurs::{Erreur, Resultat};
use md5::{Digest, Md5};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// La 1re piste de données d'une image.
#[derive(Debug, Clone)]
pub struct Piste {
    pub fichier: PathBuf,
    /// Où commence la piste dans le fichier (octets).
    pub decalage: u64,
    /// 2352 (brut), 2336 ou 2048.
    pub taille_secteur: u64,
    /// Octets avant les données utiles d'un secteur (16 en MODE1 brut, 24 en MODE2).
    pub entete: u64,
    /// Le numéro (absolu) du secteur stocké en 1er pour cette piste (le début de ce qu'on lit).
    pub premier: i64,
    /// Le numéro (absolu) du début de la piste (INDEX 01) : là où sont le secteur 0 et le volume ISO 9660.
    pub index01: i64,
    /// Un CHD : (1er bloc de la piste dans le CHD, nombre de secteurs de la piste) ; `None` : image en clair.
    pub chd: Option<(u64, u64)>,
    /// Octets utiles par secteur.
    pub utiles: u64,
}

/// Un « mm:ss:ff » de feuille .cue → secteurs.
fn msf(t: &str) -> Option<u64> {
    let p: Vec<u64> = t.split(':').filter_map(|x| x.parse().ok()).collect();
    (p.len() == 3).then(|| (p[0] * 60 + p[1]) * 75 + p[2])
}

/// Le fichier et la position de la 1re piste d'après une feuille .cue.
fn piste_du_cue(cue: &Path) -> Option<(PathBuf, u64)> {
    let texte = std::fs::read_to_string(cue).ok()?;
    let dossier = cue.parent()?;
    let mut fichier: Option<PathBuf> = None;
    let mut taille = 2352u64;
    let mut dans_piste_1 = false;
    for l in texte.lines().map(str::trim) {
        let haut = l.to_uppercase();
        if haut.starts_with("FILE ") {
            let r = l[5..].trim();
            let nom = if let Some(r) = r.strip_prefix('"') { r.split('"').next()? } else { r.split_whitespace().next()? };
            fichier = Some(dossier.join(nom));
        } else if haut.starts_with("TRACK ") {
            let mots: Vec<&str> = haut.split_whitespace().collect();
            dans_piste_1 = mots.get(1).and_then(|n| n.parse::<u32>().ok()) == Some(1);
            taille = if mots.get(2).is_some_and(|m| m.ends_with("/2048")) { 2048 } else { 2352 };
        } else if dans_piste_1 && haut.starts_with("INDEX 01") {
            let debut = msf(haut.split_whitespace().nth(2)?)?;
            return Some((fichier?, debut * taille));
        }
    }
    None
}

/// Le secteur (absolu) écrit dans l'en-tête d'un secteur brut (MSF en BCD, moins les 150 secteurs de prégap).
fn secteur_de_l_entete(h: &[u8]) -> i64 {
    let bcd = |b: u8| ((b >> 4) * 10 + (b & 0x0f)) as i64;
    (bcd(h[12]) * 60 + bcd(h[13])) * 75 + bcd(h[14]) - 150
}

/// Ouvre la 1re piste d'une image (`None` : format pas encore lu, comme .chd).
pub fn ouvrir(chemin: &Path) -> Resultat<Option<Piste>> {
    let ext = chemin.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let (fichier, decalage) = match ext.as_str() {
        "cue" => match piste_du_cue(chemin) {
            Some(x) => x,
            None => return Err(Erreur::Disque(format!("Feuille .cue illisible : {}.", chemin.display()))),
        },
        "ccd" => (chemin.with_extension("img"), 0),
        "img" | "bin" | "iso" => (chemin.to_path_buf(), 0),
        "chd" => return ouvrir_chd(chemin, Voulue::Numero(1)),
        _ => return Ok(None),
    };
    let mut f = std::fs::File::open(&fichier).map_err(|_| Erreur::Disque(format!("Image introuvable : {}.", fichier.display())))?;
    const SYNCHRO: [u8; 12] = [0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0];
    let mut h = [0u8; 32];
    for taille in [2352u64, 2336] {
        f.seek(SeekFrom::Start(16 * taille + decalage))?;
        if f.read_exact(&mut h).is_ok() && h[..12] == SYNCHRO {
            let entete = if &h[25..30] == b"CD001" { 24 } else { 16 };
            return Ok(Some(Piste { fichier, decalage, taille_secteur: taille, entete, premier: secteur_de_l_entete(&h) - 16, index01: secteur_de_l_entete(&h) - 16, chd: None, utiles: 2048 }));
        }
    }
    f.seek(SeekFrom::Start(16 * 2048 + decalage))?;
    if f.read_exact(&mut h).is_ok() && &h[1..6] == b"CD001" {
        return Ok(Some(Piste { fichier, decalage, taille_secteur: 2048, entete: 0, premier: 0, index01: 0, chd: None, utiles: 2048 }));
    }
    // Pas de « CD001 » (Sega CD sans ISO 9660 en clair, par exemple) : brut 2352 MODE1 si la synchro est au début.
    f.seek(SeekFrom::Start(decalage))?;
    if f.read_exact(&mut h).is_ok() && h[..12] == SYNCHRO {
        return Ok(Some(Piste { fichier, decalage, taille_secteur: 2352, entete: 16, premier: secteur_de_l_entete(&h), index01: secteur_de_l_entete(&h), chd: None, utiles: 2048 }));
    }
    Ok(Some(Piste { fichier, decalage, taille_secteur: 2048, entete: 0, premier: 0, index01: 0, chd: None, utiles: 2048 }))
}

/// Une piste décrite par les métadonnées d'un CHD.
#[derive(Debug, Clone, PartialEq)]
pub struct PisteChd {
    pub numero: u32,
    pub genre: String,
    pub secteurs: u64,
    pub pregap: u64,
    pub pregap_stocke: bool,
    /// Le 1er bloc (secteur) de la piste dans le CHD.
    pub bloc: u64,
}

/// Lit une valeur « CLE:valeur » d'une ligne de métadonnées CHD.
fn champ<'a>(texte: &'a str, cle: &str) -> Option<&'a str> {
    texte.split_whitespace().find_map(|m| m.strip_prefix(cle).and_then(|r| r.strip_prefix(':')))
}

/// Les pistes d'après les métadonnées (`TRACK:1 TYPE:MODE1_RAW SUBTYPE:NONE FRAMES:1234 PREGAP:0 PGTYPE:MODE1 …`).
/// Chaque piste occupe un nombre de secteurs arrondi au multiple de 4 supérieur (rembourrage de chdman).
pub fn lire_pistes_chd(entrees: &[String]) -> Vec<PisteChd> {
    let mut l = Vec::new();
    let mut bloc = 0u64;
    for e in entrees {
        let Some(numero) = champ(e, "TRACK").and_then(|n| n.parse().ok()) else { continue };
        let secteurs: u64 = champ(e, "FRAMES").and_then(|n| n.parse().ok()).unwrap_or(0);
        let pregap = champ(e, "PREGAP").and_then(|n| n.parse().ok()).unwrap_or(0);
        let pregap_stocke = champ(e, "PGTYPE").is_some_and(|t| t.starts_with('V'));
        l.push(PisteChd { numero, genre: champ(e, "TYPE").unwrap_or("").to_string(), secteurs, pregap, pregap_stocke, bloc });
        bloc += secteurs.div_ceil(4) * 4;
    }
    l
}

/// Quelle piste ouvrir dans un CHD.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Voulue {
    Numero(u32),
    PremiereDeDonnees,
    Derniere,
}

/// Ouvre une piste d'un CHD (LECTURE SEULE).
pub fn ouvrir_chd(chemin: &Path, voulue: Voulue) -> Resultat<Option<Piste>> {
    let mut f = std::io::BufReader::new(std::fs::File::open(chemin).map_err(|_| Erreur::Disque(format!("Image introuvable : {}.", chemin.display())))?);
    let mut c = chd::Chd::open(&mut f, None).map_err(|e| Erreur::Disque(format!("CHD illisible ({e:?}).")))?;
    let refs: Vec<_> = c.metadata_refs().collect();
    let mut f2 = std::fs::File::open(chemin)?;
    let entrees: Vec<String> = refs
        .iter()
        .filter_map(|r| r.read(&mut f2).ok())
        .map(|m| String::from_utf8_lossy(&m.value).trim_end_matches('\0').to_string())
        .collect();
    let pistes = lire_pistes_chd(&entrees);
    let p = match voulue {
        Voulue::Numero(n) => pistes.iter().find(|p| p.numero == n),
        Voulue::PremiereDeDonnees => pistes.iter().find(|p| p.genre != "AUDIO"),
        Voulue::Derniere => pistes.last(),
    };
    let Some(p) = p.cloned() else { return Ok(None) };
    if p.genre == "AUDIO" {
        return Ok(None);
    }
    drop(c);
    let brut = p.genre.ends_with("_RAW");
    let mut piste = Piste {
        fichier: chemin.to_path_buf(),
        decalage: 0,
        taille_secteur: 2448,
        entete: 0,
        premier: 0,
        index01: 0,
        chd: Some((p.bloc, p.secteurs)),
        utiles: 2048,
    };
    if brut {
        // L'adresse absolue de la piste : celle écrite dans l'en-tête de son 1er secteur stocké.
        let mut tete = piste.clone();
        tete.utiles = 2352;
        let s0 = tete.lire_bloc(0, 32)?;
        if s0.len() >= 32 && s0[..12] == [0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0] {
            piste.premier = secteur_de_l_entete(&s0);
            // MODE2 (PlayStation…) : 24 octets d'en-tête ; MODE1 : 16.
            piste.entete = if s0[15] == 2 { 24 } else { 16 };
        } else {
            piste.entete = 16;
        }
    }
    piste.index01 = piste.premier + if p.pregap_stocke { p.pregap as i64 } else { 0 };
    Ok(Some(piste))
}

impl Piste {
    /// CHD : lit `n` octets à partir du `k`-ième secteur STOCKÉ de la piste (en-tête compris si `entete` vaut 0).
    fn lire_bloc(&self, k: u64, n: usize) -> Resultat<Vec<u8>> {
        let Some((bloc, secteurs)) = self.chd else { return Ok(vec![]) };
        let mut f = std::io::BufReader::new(std::fs::File::open(&self.fichier)?);
        let mut c = chd::Chd::open(&mut f, None).map_err(|e| Erreur::Disque(format!("CHD illisible ({e:?}).")))?;
        let taille_bloc = c.header().hunk_size() as u64;
        let unite = c.header().unit_bytes() as u64;
        let par_bloc = taille_bloc / unite.max(1);
        let mut tampon = c.get_hunksized_buffer();
        let mut compresse = Vec::new();
        let mut charge: Option<u64> = None;
        let mut sortie = Vec::with_capacity(n);
        let mut k = k;
        while sortie.len() < n && k < secteurs {
            let secteur_chd = bloc + k;
            let h = secteur_chd / par_bloc;
            if charge != Some(h) {
                c.hunk(h as u32)
                    .and_then(|mut x| x.read_hunk_in(&mut compresse, &mut tampon))
                    .map_err(|e| Erreur::Disque(format!("CHD illisible ({e:?}).")))?;
                charge = Some(h);
            }
            let debut = ((secteur_chd % par_bloc) * unite + self.entete) as usize;
            let voulu = (n - sortie.len()).min(self.utiles as usize);
            sortie.extend_from_slice(&tampon[debut..(debut + voulu).min(tampon.len())]);
            k += 1;
        }
        Ok(sortie)
    }

    /// Lit `n` octets utiles à partir du secteur (absolu) donné, secteur après secteur.
    pub fn lire(&self, secteur: u32, n: usize) -> Resultat<Vec<u8>> {
        if self.chd.is_some() {
            let k = secteur as i64 - self.premier;
            return if k < 0 { Ok(vec![]) } else { self.lire_bloc(k as u64, n) };
        }
        let mut f = std::fs::File::open(&self.fichier)?;
        let mut sortie = Vec::with_capacity(n);
        let mut s = secteur as i64 - self.premier;
        if s < 0 {
            return Ok(sortie);
        }
        while sortie.len() < n {
            let pos = s as u64 * self.taille_secteur + self.entete + self.decalage;
            f.seek(SeekFrom::Start(pos))?;
            let voulu = (n - sortie.len()).min(self.utiles as usize);
            let mut b = vec![0u8; voulu];
            let lu = f.read(&mut b)?;
            sortie.extend_from_slice(&b[..lu]);
            if lu < voulu {
                break;
            }
            s += 1;
        }
        Ok(sortie)
    }

    /// Trouve un fichier ISO 9660 (`A\\B\\FICHIER`, sans casse, sans « ;1 ») : (secteur, taille).
    pub fn trouver(&self, chemin: &str) -> Resultat<Option<(u32, u32)>> {
        let chemin = chemin.trim_start_matches('\\');
        let (secteur_dossier, mut nb) = match chemin.rsplit_once('\\') {
            Some((parent, _)) => match self.trouver(parent)? {
                Some((s, _)) => (s, 1u32),
                None => return Ok(None),
            },
            None => {
                let pvd = self.lire(self.index01.max(0) as u32 + 16, 256)?;
                if pvd.len() < 170 {
                    return Ok(None);
                }
                let s = u32::from_le_bytes([pvd[158], pvd[159], pvd[160], 0]);
                let bloc = u16::from_le_bytes([pvd[128], pvd[129]]) as u32;
                let longueur = u32::from_le_bytes([pvd[166], pvd[167], pvd[168], pvd[169]]);
                (s, if bloc == 0 { 1 } else { (longueur / bloc).max(1) })
            }
        };
        let nom = chemin.rsplit('\\').next().unwrap_or(chemin).to_uppercase();
        let mut s = secteur_dossier;
        loop {
            let b = self.lire(s, 2048)?;
            let mut i = 0usize;
            while i < b.len() && b[i] != 0 {
                let l = b[i] as usize;
                if i + 33 > b.len() || l < 34 {
                    break;
                }
                let n = b[i + 32] as usize;
                let brut = String::from_utf8_lossy(&b[i + 33..(i + 33 + n).min(b.len())]).to_uppercase();
                let sans_version = brut.split(';').next().unwrap_or("");
                if sans_version == nom {
                    let secteur = u32::from_le_bytes([b[i + 2], b[i + 3], b[i + 4], 0]);
                    let taille = u32::from_le_bytes([b[i + 10], b[i + 11], b[i + 12], b[i + 13]]);
                    return Ok(Some((secteur, taille)));
                }
                i += l;
            }
            nb -= 1;
            if nb == 0 {
                return Ok(None);
            }
            s += 1;
        }
    }
}

fn md5_hex(o: &[u8]) -> String {
    Md5::digest(o).iter().map(|b| format!("{b:02x}")).collect()
}

/// Dreamcast (rc_hash_dreamcast) : les 256 octets d'`IP.BIN` (1er secteur de la piste 3, « SEGA SEGAKATANA »), puis
/// le contenu du programme de démarrage (son nom est à l'octet 96), lu dans la piste 3 ou, à défaut, la dernière piste.
/// Un MIL-CD a son `IP.BIN` sur la 1re piste de données.
pub fn empreinte_dreamcast(chemin: &Path) -> Resultat<Option<String>> {
    let est_ip = |p: &Piste| -> Resultat<Option<Vec<u8>>> {
        let b = p.lire(p.index01.max(0) as u32, 256)?;
        Ok((b.len() == 256 && b.starts_with(b"SEGA SEGAKATANA ")).then_some(b))
    };
    let mut piste = ouvrir_chd(chemin, Voulue::Numero(3))?;
    let mut ip = match &piste {
        Some(p) => est_ip(p)?,
        None => None,
    };
    if ip.is_none() {
        piste = ouvrir_chd(chemin, Voulue::PremiereDeDonnees)?;
        ip = match &piste {
            Some(p) => est_ip(p)?,
            None => None,
        };
    }
    let (Some(piste), Some(ip)) = (piste, ip) else { return Ok(None) };
    let nom: String = ip[96..112].iter().take_while(|b| !b.is_ascii_whitespace()).map(|b| *b as char).collect();
    if nom.is_empty() {
        return Ok(None);
    }
    let Some((secteur, taille)) = piste.trouver(&nom)? else { return Ok(None) };
    let lu = piste.lire(secteur, 1)?;
    let contenu = if !lu.is_empty() {
        piste.lire(secteur, taille.min(MAX_FICHIER) as usize)?
    } else {
        match ouvrir_chd(chemin, Voulue::Derniere)? {
            Some(derniere) => derniere.lire(secteur, taille.min(MAX_FICHIER) as usize)?,
            None => return Ok(None),
        }
    };
    let mut m = Md5::new();
    m.update(&ip);
    m.update(&contenu);
    Ok(Some(m.finalize().iter().map(|b| format!("{b:02x}")).collect()))
}

/// Sega CD et Saturn (rc_hash_sega_cd) : les 512 premiers octets du secteur 0 (en-têtes du volume et de la ROM).
pub fn empreinte_sega_cd(p: &Piste) -> Resultat<Option<String>> {
    let b = p.lire(p.index01.max(0) as u32, 512)?;
    if b.len() < 512 || !(b.starts_with(b"SEGADISCSYSTEM  ") || b.starts_with(b"SEGA SEGASATURN ")) {
        return Ok(None);
    }
    Ok(Some(md5_hex(&b)))
}

/// La limite de `rcheevos` (MAX_BUFFER_SIZE) sur ce qui est lu d'un fichier.
const MAX_FICHIER: u32 = 64 * 1024 * 1024;

/// PlayStation (rc_hash_psx) : le nom du programme de démarrage (`BOOT = cdrom:\\SLUS_005.94;1` dans SYSTEM.CNF, sinon
/// PSX.EXE), puis son contenu (taille lue dans l'en-tête « PS-X EXE », + 2048).
pub fn empreinte_psx(p: &Piste) -> Resultat<Option<String>> {
    let mut exe = String::new();
    let mut trouve = None;
    if let Some((s, _)) = p.trouver("SYSTEM.CNF")? {
        let cnf = String::from_utf8_lossy(&p.lire(s, 2047)?).to_string();
        for ligne in cnf.lines() {
            let l = ligne.trim_start();
            if let Some(r) = l.strip_prefix("BOOT") {
                let r = r.trim_start();
                if let Some(r) = r.strip_prefix('=') {
                    let r = r.trim_start();
                    let r = r.strip_prefix("cdrom:").unwrap_or(r).trim_start_matches('\\');
                    exe = r.split(|c: char| c.is_whitespace() || c == ';').next().unwrap_or("").to_string();
                    trouve = p.trouver(&exe)?;
                    break;
                }
            }
        }
    }
    if trouve.is_none() {
        if let Some(t) = p.trouver("PSX.EXE")? {
            exe = "PSX.EXE".into();
            trouve = Some(t);
        }
    }
    let Some((secteur, mut taille)) = trouve else { return Ok(None) };
    let entete = p.lire(secteur, 32)?;
    if entete.len() < 32 {
        return Ok(None);
    }
    if entete.starts_with(b"PS-X EX") {
        taille = u32::from_le_bytes([entete[28], entete[29], entete[30], entete[31]]) + 2048;
    }
    let contenu = p.lire(secteur, taille.min(MAX_FICHIER) as usize)?;
    let mut m = Md5::new();
    m.update(exe.as_bytes());
    m.update(&contenu);
    Ok(Some(m.finalize().iter().map(|b| format!("{b:02x}")).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un petit disque ISO 9660 : SYSTEM.CNF et le programme de démarrage, en secteurs de 2048 ou bruts (MODE2/2352).
    fn disque(brut: bool) -> (Vec<u8>, Vec<u8>) {
        let mut secteurs = vec![vec![0u8; 2048]; 24];
        // PVD (secteur 16) : bloc de 2048, dossier racine au secteur 20 (une seule page).
        let pvd = &mut secteurs[16];
        pvd[0] = 1;
        pvd[1..6].copy_from_slice(b"CD001");
        pvd[128..130].copy_from_slice(&2048u16.to_le_bytes());
        pvd[158..161].copy_from_slice(&[20, 0, 0]);
        pvd[166..170].copy_from_slice(&2048u32.to_le_bytes());
        // Le dossier racine : deux fiches.
        let mut racine = Vec::new();
        for (nom, secteur, taille) in [("SYSTEM.CNF;1", 21u32, 40u32), ("SLUS_005.94;1", 22, 4096)] {
            let mut r = vec![0u8; 33 + nom.len() + (nom.len() + 1) % 2];
            r[0] = r.len() as u8;
            r[2..6].copy_from_slice(&secteur.to_le_bytes());
            r[10..14].copy_from_slice(&taille.to_le_bytes());
            r[32] = nom.len() as u8;
            r[33..33 + nom.len()].copy_from_slice(nom.as_bytes());
            racine.extend(r);
        }
        secteurs[20][..racine.len()].copy_from_slice(&racine);
        let cnf = b"BOOT = cdrom:\\SLUS_005.94;1\r\nTCB = 4\r\n";
        secteurs[21][..cnf.len()].copy_from_slice(cnf);
        // Le programme : en-tête « PS-X EXE », taille 2048 (+ 2048 d'en-tête = 4096 hachés).
        secteurs[22][..8].copy_from_slice(b"PS-X EXE");
        secteurs[22][28..32].copy_from_slice(&2048u32.to_le_bytes());
        secteurs[23].iter_mut().enumerate().for_each(|(i, b)| *b = (i % 251) as u8);
        let mut attendu = b"SLUS_005.94".to_vec();
        attendu.extend(&secteurs[22]);
        attendu.extend(&secteurs[23]);
        let image = if brut {
            let mut o = Vec::new();
            for (i, s) in secteurs.iter().enumerate() {
                let mut e = vec![0u8; 24];
                e[..12].copy_from_slice(&[0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0]);
                let a = i as u32 + 150;
                let bcd = |v: u32| (((v / 10) << 4) | (v % 10)) as u8;
                e[12] = bcd(a / 75 / 60);
                e[13] = bcd(a / 75 % 60);
                e[14] = bcd(a % 75);
                e[15] = 2;
                o.extend(e);
                o.extend(s);
                o.extend(vec![0u8; 280]);
            }
            o
        } else {
            secteurs.concat()
        };
        (image, attendu)
    }

    #[test]
    fn playstation_le_nom_du_programme_puis_son_contenu() {
        let d = tempfile::tempdir().unwrap();
        for brut in [false, true] {
            let (image, attendu) = disque(brut);
            let bin = d.path().join(if brut { "jeu.bin" } else { "jeu.iso" });
            std::fs::write(&bin, &image).unwrap();
            let cue = d.path().join("jeu.cue");
            std::fs::write(&cue, "FILE \"jeu.bin\" BINARY\n  TRACK 01 MODE2/2352\n    INDEX 01 00:00:00\n").unwrap();
            let p = ouvrir(if brut { &cue } else { &bin }).unwrap().unwrap();
            assert_eq!((p.taille_secteur, p.entete), if brut { (2352, 24) } else { (2048, 0) });
            assert_eq!(p.trouver("system.cnf").unwrap(), Some((21, 40)), "sans casse");
            assert_eq!(empreinte_psx(&p).unwrap(), Some(md5_hex(&attendu)));
        }
    }

    #[test]
    fn les_pistes_d_un_chd_se_lisent_dans_ses_metadonnees() {
        let l = lire_pistes_chd(&[
            "TRACK:1 TYPE:MODE1_RAW SUBTYPE:NONE FRAMES:1143 PAD:0 PREGAP:0 PGTYPE:MODE1 PGSUB:RW POSTGAP:0".into(),
            "TRACK:2 TYPE:AUDIO SUBTYPE:NONE FRAMES:1500 PAD:0 PREGAP:150 PGTYPE:VAUDIO PGSUB:RW POSTGAP:0".into(),
            "TRACK:3 TYPE:MODE1_RAW SUBTYPE:NONE FRAMES:504300 PAD:0 PREGAP:0 PGTYPE:MODE1 PGSUB:RW POSTGAP:0".into(),
        ]);
        assert_eq!(l.len(), 3);
        assert_eq!((l[0].bloc, l[1].bloc, l[2].bloc), (0, 1144, 2644), "chaque piste arrondie à 4 secteurs");
        assert!(l[1].pregap_stocke && !l[0].pregap_stocke);
        assert_eq!(l[2].genre, "MODE1_RAW");
    }

    #[test]
    fn sega_cd_les_512_premiers_octets() {
        let d = tempfile::tempdir().unwrap();
        let mut s0 = vec![0u8; 2048];
        s0[..16].copy_from_slice(b"SEGADISCSYSTEM  ");
        s0[100] = 7;
        let mut image = s0.clone();
        image.extend(vec![0u8; 2048 * 20]);
        let iso = d.path().join("sonic.iso");
        std::fs::write(&iso, &image).unwrap();
        let p = ouvrir(&iso).unwrap().unwrap();
        assert_eq!(empreinte_sega_cd(&p).unwrap(), Some(md5_hex(&s0[..512])));
        // Pas un disque Sega : pas d'empreinte.
        std::fs::write(&iso, vec![0u8; 2048 * 20]).unwrap();
        assert_eq!(empreinte_sega_cd(&ouvrir(&iso).unwrap().unwrap()).unwrap(), None);
        assert!(ouvrir(&d.path().join("jeu.cso")).unwrap().is_none(), "format inconnu : rien");
    }
}

#[cfg(test)]
mod essais {
    /// Sur les vrais disques de Seb, en LECTURE SEULE : l'empreinte Sega CD des premiers jeux d'un dossier.
    #[test]
    #[ignore]
    fn essai_sega_cd_reel() {
        let d = std::env::var("FROGTEND_ESSAI_DOSSIER").unwrap_or_else(|_| "E:/Games/Sega CD".into());
        let mut n = 0;
        let mut l: Vec<std::path::PathBuf> = std::fs::read_dir(&d)
            .unwrap()
            .flatten()
            .flat_map(|e| if e.path().is_dir() { std::fs::read_dir(e.path()).unwrap().flatten().map(|x| x.path()).collect() } else { vec![e.path()] })
            .collect();
        l.sort();
        for p in l.iter().filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("cue"))) {
            let piste = super::ouvrir(p).unwrap().unwrap();
            let e = super::empreinte_sega_cd(&piste).unwrap();
            if n < 4 {
                println!("{} : secteur {} entête {} → {:?}", p.file_name().unwrap().to_string_lossy(), piste.taille_secteur, piste.entete, e);
            }
            if e.is_some() {
                n += 1;
            }
        }
        println!("{n} empreinte(s) Sega CD calculée(s)");
    }
}

#[cfg(test)]
mod essais_chd {
    /// Sur les vrais disques Dreamcast de Seb (.chd), en LECTURE SEULE : leurs empreintes.
    #[test]
    #[ignore]
    fn essai_dreamcast_reel() {
        let d = std::env::var("FROGTEND_ESSAI_DOSSIER").unwrap_or_else(|_| "E:/Games/Sega Dreamcast".into());
        let mut l: Vec<std::path::PathBuf> = std::fs::read_dir(&d)
            .unwrap()
            .flatten()
            .flat_map(|e| if e.path().is_dir() { std::fs::read_dir(e.path()).unwrap().flatten().map(|x| x.path()).collect() } else { vec![e.path()] })
            .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("chd")))
            .collect();
        l.sort();
        let max: usize = std::env::var("FROGTEND_ESSAI_MAX").ok().and_then(|n| n.parse().ok()).unwrap_or(5);
        let (mut ok, mut rien, mut erreurs) = (0, 0, 0);
        let debut = std::time::Instant::now();
        for p in l.iter().take(max) {
            match super::empreinte_dreamcast(p) {
                Ok(Some(e)) => {
                    ok += 1;
                    if ok <= 5 {
                        println!("{} → {e}", p.file_name().unwrap().to_string_lossy());
                    }
                }
                Ok(None) => {
                    rien += 1;
                    println!("RIEN : {}", p.file_name().unwrap().to_string_lossy());
                }
                Err(e) => {
                    erreurs += 1;
                    println!("ERREUR : {} : {e:?}", p.file_name().unwrap().to_string_lossy());
                }
            }
        }
        println!("{ok} empreinte(s), {rien} sans, {erreurs} erreur(s) sur {} ({} fichiers) en {:?}", max.min(l.len()), l.len(), debut.elapsed());
    }
}
