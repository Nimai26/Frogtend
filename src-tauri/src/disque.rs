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
    /// Un .cso (PSP compressé) : son index de blocs ; `None` : image en clair.
    pub cso: Option<std::sync::Arc<Cso>>,
    /// Octets utiles par secteur.
    pub utiles: u64,
}

/// Un .cso (CISO) : un en-tête de 24 octets, un index de blocs (u32 ; bit 31 = bloc stocké tel quel ; position
/// décalée de `align`), puis des blocs compressés en « deflate » brut. Seul ce qui est lu est décompressé.
#[derive(Debug)]
pub struct Cso {
    pub taille_bloc: u64,
    pub align: u32,
    pub total: u64,
    pub index: Vec<u32>,
}

impl Cso {
    pub fn ouvrir(chemin: &Path) -> Resultat<Option<Cso>> {
        let mut f = std::fs::File::open(chemin)?;
        let e = lire_a(&mut f, 0, 24);
        if &e[..4] != b"CISO" {
            return Ok(None);
        }
        let total = u64::from_le_bytes(e[8..16].try_into().unwrap_or_default());
        let taille_bloc = u32::from_le_bytes(e[16..20].try_into().unwrap_or_default()) as u64;
        if taille_bloc == 0 || total == 0 {
            return Ok(None);
        }
        let blocs = total.div_ceil(taille_bloc) + 1;
        let brut = lire_a(&mut f, 24, (blocs * 4) as usize);
        let index = brut.chunks(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
        Ok(Some(Cso { taille_bloc, align: e[21] as u32, total, index }))
    }

    /// `n` octets à la position logique `pos` (dans l'image décompressée).
    pub fn lire(&self, chemin: &Path, pos: u64, n: usize) -> Resultat<Vec<u8>> {
        let mut f = std::fs::File::open(chemin)?;
        let mut sortie = Vec::with_capacity(n);
        let mut pos = pos;
        while sortie.len() < n && pos < self.total {
            let b = (pos / self.taille_bloc) as usize;
            let (Some(e), Some(e2)) = (self.index.get(b), self.index.get(b + 1)) else { break };
            let debut = ((e & 0x7FFF_FFFF) as u64) << self.align;
            let fin = ((e2 & 0x7FFF_FFFF) as u64) << self.align;
            let brut = lire_a(&mut f, debut, fin.saturating_sub(debut) as usize);
            let bloc = if e & 0x8000_0000 != 0 {
                brut
            } else {
                let mut d = Vec::new();
                let _ = flate2::read::DeflateDecoder::new(&brut[..]).take(self.taille_bloc).read_to_end(&mut d);
                d
            };
            let dans = (pos % self.taille_bloc) as usize;
            if dans >= bloc.len() {
                break;
            }
            let k = (n - sortie.len()).min(bloc.len() - dans);
            sortie.extend_from_slice(&bloc[dans..dans + k]);
            pos += k as u64;
        }
        Ok(sortie)
    }
}

/// Un « mm:ss:ff » de feuille .cue → secteurs.
fn msf(t: &str) -> Option<u64> {
    let p: Vec<u64> = t.split(':').filter_map(|x| x.parse().ok()).collect();
    (p.len() == 3).then(|| (p[0] * 60 + p[1]) * 75 + p[2])
}

/// Une piste d'une feuille .cue.
#[derive(Debug, Clone, PartialEq)]
pub struct PisteCue {
    pub numero: u32,
    pub fichier: PathBuf,
    /// Où commence la piste (INDEX 01) dans son fichier, en octets.
    pub decalage: u64,
    pub audio: bool,
    /// La session (« REM SESSION 02 ») ; 1 par défaut.
    pub session: u32,
    /// Sa longueur en octets (jusqu'à la piste suivante du même fichier, ou la fin du fichier).
    pub longueur: u64,
}

/// Toutes les pistes d'une feuille .cue.
pub fn pistes_du_cue(cue: &Path) -> Vec<PisteCue> {
    let Ok(texte) = std::fs::read_to_string(cue) else { return vec![] };
    let dossier = cue.parent().unwrap_or(Path::new("."));
    let mut l: Vec<PisteCue> = Vec::new();
    let mut fichier: Option<PathBuf> = None;
    let (mut numero, mut audio, mut taille, mut session) = (0u32, false, 2352u64, 1u32);
    for ligne in texte.lines().map(str::trim) {
        let haut = ligne.to_uppercase();
        if let Some(s) = haut.strip_prefix("REM SESSION ") {
            session = s.trim().parse().unwrap_or(session);
        } else if haut.starts_with("FILE ") {
            let r = ligne[5..].trim();
            let nom = if let Some(r) = r.strip_prefix('"') { r.split('"').next() } else { r.split_whitespace().next() };
            fichier = nom.map(|n| dossier.join(n));
        } else if haut.starts_with("TRACK ") {
            let mots: Vec<&str> = haut.split_whitespace().collect();
            numero = mots.get(1).and_then(|n| n.parse().ok()).unwrap_or(0);
            audio = mots.get(2).is_some_and(|m| *m == "AUDIO");
            taille = if mots.get(2).is_some_and(|m| m.ends_with("/2048")) { 2048 } else { 2352 };
        } else if haut.starts_with("INDEX 01") {
            if let (Some(f), Some(debut)) = (fichier.clone(), haut.split_whitespace().nth(2).and_then(msf)) {
                l.push(PisteCue { numero, fichier: f, decalage: debut * taille, audio, session, longueur: 0 });
            }
        }
    }
    for i in 0..l.len() {
        let fin = match l.get(i + 1) {
            Some(s) if s.fichier == l[i].fichier => s.decalage,
            _ => std::fs::metadata(&l[i].fichier).map(|m| m.len()).unwrap_or(l[i].decalage),
        };
        l[i].longueur = fin.saturating_sub(l[i].decalage);
    }
    l
}

/// Le secteur (absolu) écrit dans l'en-tête d'un secteur brut (MSF en BCD, moins les 150 secteurs de prégap).
fn secteur_de_l_entete(h: &[u8]) -> i64 {
    let bcd = |b: u8| ((b >> 4) * 10 + (b & 0x0f)) as i64;
    (bcd(h[12]) * 60 + bcd(h[13])) * 75 + bcd(h[14]) - 150
}

/// Ouvre la 1re piste d'une image (`None` : format pas lu).
pub fn ouvrir(chemin: &Path) -> Resultat<Option<Piste>> {
    ouvrir_piste(chemin, Voulue::Numero(1))
}

/// Ouvre la piste voulue d'une image (.cue : toutes ses pistes ; .chd ; une image d'une seule piste sinon).
pub fn ouvrir_piste(chemin: &Path, voulue: Voulue) -> Resultat<Option<Piste>> {
    let ext = chemin.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let (fichier, decalage) = match ext.as_str() {
        "cue" => {
            let pistes = pistes_du_cue(chemin);
            if pistes.is_empty() {
                return Err(Erreur::Disque(format!("Feuille .cue illisible : {}.", chemin.display())));
            }
            let p = match voulue {
                Voulue::Numero(n) => pistes.iter().find(|p| p.numero == n),
                Voulue::PremiereDeDonnees => pistes.iter().find(|p| !p.audio),
                Voulue::PlusGrandeDeDonnees => pistes.iter().filter(|p| !p.audio).max_by_key(|p| p.longueur),
                Voulue::Derniere => pistes.last(),
            };
            match p {
                Some(p) if !p.audio => (p.fichier.clone(), p.decalage),
                _ => return Ok(None),
            }
        }
        "ccd" => (chemin.with_extension("img"), 0),
        "img" | "bin" | "iso" => (chemin.to_path_buf(), 0),
        "chd" => return ouvrir_chd(chemin, voulue),
        "cso" => {
            return Ok(Cso::ouvrir(chemin)?.map(|c| Piste {
                fichier: chemin.to_path_buf(),
                decalage: 0,
                taille_secteur: 2048,
                entete: 0,
                premier: 0,
                index01: 0,
                chd: None,
                cso: Some(std::sync::Arc::new(c)),
                utiles: 2048,
            }))
        }
        _ => return Ok(None),
    };
    let mut f = std::fs::File::open(&fichier).map_err(|_| Erreur::Disque(format!("Image introuvable : {}.", fichier.display())))?;
    const SYNCHRO: [u8; 12] = [0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0];
    let mut h = [0u8; 32];
    for taille in [2352u64, 2336] {
        f.seek(SeekFrom::Start(16 * taille + decalage))?;
        if f.read_exact(&mut h).is_ok() && h[..12] == SYNCHRO {
            let entete = if &h[25..30] == b"CD001" { 24 } else { 16 };
            return Ok(Some(Piste { fichier, decalage, taille_secteur: taille, entete, premier: secteur_de_l_entete(&h) - 16, index01: secteur_de_l_entete(&h) - 16, chd: None, cso: None, utiles: 2048 }));
        }
    }
    f.seek(SeekFrom::Start(16 * 2048 + decalage))?;
    if f.read_exact(&mut h).is_ok() && &h[1..6] == b"CD001" {
        return Ok(Some(Piste { fichier, decalage, taille_secteur: 2048, entete: 0, premier: 0, index01: 0, chd: None, cso: None, utiles: 2048 }));
    }
    // Pas de « CD001 » (Sega CD sans ISO 9660 en clair, par exemple) : brut 2352 MODE1 si la synchro est au début.
    f.seek(SeekFrom::Start(decalage))?;
    if f.read_exact(&mut h).is_ok() && h[..12] == SYNCHRO {
        return Ok(Some(Piste { fichier, decalage, taille_secteur: 2352, entete: 16, premier: secteur_de_l_entete(&h), index01: secteur_de_l_entete(&h), chd: None, cso: None, utiles: 2048 }));
    }
    Ok(Some(Piste { fichier, decalage, taille_secteur: 2048, entete: 0, premier: 0, index01: 0, chd: None, cso: None, utiles: 2048 }))
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
    PlusGrandeDeDonnees,
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
        Voulue::PlusGrandeDeDonnees => pistes.iter().filter(|p| p.genre != "AUDIO").max_by_key(|p| p.secteurs),
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
        cso: None,
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
        if let Some(c) = &self.cso {
            let k = secteur as i64 - self.premier;
            return if k < 0 { Ok(vec![]) } else { c.lire(&self.fichier, k as u64 * 2048, n) };
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

// --- GameCube et Wii (rc_hash_gamecube, rc_hash_wii) : lus comme un .iso BRUT (les .rvz/.wia/.gcz/.wbfs/.ciso par la
// bibliothèque `nod` 1.4, qui rechiffre les données Wii comme sur le disque d'origine, ce que lit Dolphin). ---

/// Lit `n` octets à une position (des zéros si le disque est plus court, comme rcheevos).
fn lire_a<R: Read + Seek>(r: &mut R, pos: u64, n: usize) -> Vec<u8> {
    let mut b = vec![0u8; n];
    if r.seek(SeekFrom::Start(pos)).is_ok() {
        let mut lu = 0;
        while lu < n {
            match r.read(&mut b[lu..]) {
                Ok(0) | Err(_) => break,
                Ok(k) => lu += k,
            }
        }
    }
    b
}

fn be32(b: &[u8], i: usize) -> u64 {
    u32::from_be_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]) as u64
}

/// Une partition Nintendo (rc_hash_nintendo_disc_partition) : en-têtes jusqu'à la fin de l'apploader (au plus 1 Mio),
/// puis les 18 segments de main.dol. `decalage` : décalage de la partition ; `wii` : adresses ×4.
fn partition_nintendo<R: Read + Seek>(r: &mut R, m: &mut Md5, partition: u64, wii: bool) {
    const BASE: u64 = 0x2440;
    let decale = |v: u64| if wii { v << 2 } else { v };
    let tailles = lire_a(r, partition + BASE + 0x14, 8);
    let entete = (BASE + 0x20 + be32(&tailles, 0) + be32(&tailles, 4)).min(1024 * 1024);
    let tete = lire_a(r, partition, entete as usize);
    m.update(&tete);
    let dol = decale(be32(&tete, 0x420));
    let adresses = lire_a(r, partition + dol, 0xD8);
    for i in 0..18 {
        let (pos, taille) = (decale(be32(&adresses, i * 4)), decale(be32(&adresses, 0x90 + i * 4)));
        if taille == 0 {
            continue;
        }
        let mut reste = taille;
        let mut p = partition + pos;
        while reste > 0 {
            let n = reste.min(1024 * 1024);
            m.update(lire_a(r, p, n as usize));
            p += n;
            reste -= n;
        }
    }
}

/// GameCube : la partition unique, si le disque porte la signature GameCube (0xC2339F3D à 0x1C).
pub fn empreinte_gamecube_flux<R: Read + Seek>(r: &mut R) -> Option<String> {
    if lire_a(r, 0x1C, 4) != [0xC2, 0x33, 0x9F, 0x3D] {
        return None;
    }
    let mut m = Md5::new();
    partition_nintendo(r, &mut m, 0, false);
    Some(m.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Wii (rc_hash_wii_disc) : en-tête principal (0x80), code de région, puis pour chaque partition (sauf « mise à
/// jour ») son TMD et ses 1 024 premiers groupes chiffrés (0x7C00 octets après 0x400 de chaque bloc de 0x8000) ; un
/// disque non chiffré : la partition comme un GameCube (adresses ×4).
pub fn empreinte_wii_flux<R: Read + Seek>(r: &mut R) -> Option<String> {
    if lire_a(r, 0x18, 4) != [0x5D, 0x1C, 0x9E, 0xA3] {
        return None;
    }
    let chiffre = lire_a(r, 0x61, 1)[0] == 0;
    let mut m = Md5::new();
    m.update(lire_a(r, 0, 0x80));
    m.update(lire_a(r, 0x4E000, 4));
    let table = lire_a(r, 0x40000, 32);
    let mut partitions = Vec::new();
    for g in 0..4 {
        let (nombre, ou) = (be32(&table, g * 8), be32(&table, g * 8 + 4) << 2);
        let l = lire_a(r, ou, (nombre.min(64) * 8) as usize);
        for i in 0..nombre.min(64) as usize {
            partitions.push((be32(&l, i * 8) << 2, be32(&l, i * 8 + 4)));
        }
    }
    if partitions.is_empty() {
        return None;
    }
    for (debut, genre) in partitions {
        if genre == 1 {
            continue; // partition de mise à jour
        }
        let t = lire_a(r, debut + 0x2A4, 8);
        let (taille_tmd, ou_tmd) = (be32(&t, 0).min(0x7C00), be32(&t, 4) << 2);
        m.update(lire_a(r, debut + ou_tmd, taille_tmd as usize));
        let p = lire_a(r, debut + 0x2B8, 8);
        let (donnees, taille) = (be32(&p, 0) << 2, be32(&p, 4) << 2);
        if chiffre {
            for i in 0..(taille / 0x8000).min(1024) {
                m.update(lire_a(r, debut + donnees + i * 0x8000 + 0x400, 0x7C00));
            }
        } else {
            partition_nintendo(r, &mut m, debut + donnees, true);
        }
    }
    Some(m.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Ouvre une image GameCube/Wii (iso, rvz, wia, gcz, wbfs, ciso…) par `nod`, comme un .iso brut.
fn ouvrir_nintendo(chemin: &Path) -> Resultat<nod::Disc> {
    nod::Disc::new_with_options(chemin, &nod::OpenOptions { rebuild_encryption: true, validate_hashes: false })
        .map_err(|e| Erreur::Disque(format!("Image GameCube/Wii illisible ({e}).")))
}

pub fn empreinte_gamecube(chemin: &Path) -> Resultat<Option<String>> {
    Ok(empreinte_gamecube_flux(&mut ouvrir_nintendo(chemin)?))
}

pub fn empreinte_wii(chemin: &Path) -> Resultat<Option<String>> {
    Ok(empreinte_wii_flux(&mut ouvrir_nintendo(chemin)?))
}

/// 3DO (rc_hash_3do) : les 132 octets du volume « Opera » (secteur 0), puis le contenu du programme `LaunchMe` trouvé
/// dans le dossier racine (fiches de 0x48 octets + copies, suite éventuelle dans d'autres blocs).
pub fn empreinte_3do(p: &Piste) -> Resultat<Option<String>> {
    let base = p.index01.max(0) as u32;
    let lire = |s: u32, n: usize| p.lire(base + s, n);
    let v = lire(0, 132)?;
    if v.len() < 132 || v[..7] != [0x01, 0x5A, 0x5A, 0x5A, 0x5A, 0x5A, 0x01] {
        return Ok(None);
    }
    let octets3 = |b: &[u8], i: usize| (b[i] as u32) << 16 | (b[i + 1] as u32) << 8 | b[i + 2] as u32;
    let taille_bloc_volume = octets3(&v, 0x4D);
    let racine = octets3(&v, 0x65) * taille_bloc_volume;
    let mut secteur = racine / 2048;
    let mut trouve: Option<(u32, u32)> = None;
    for _ in 0..256 {
        let b = lire(secteur, 2048)?;
        if b.len() < 2048 {
            break;
        }
        let mut o = ((b[0x12] as usize) << 8) | b[0x13] as usize;
        let fin = octets3(&b, 0x0D) as usize;
        while o < fin && o + 0x48 <= b.len() {
            if b[o + 0x03] == 0x02 {
                let nom: Vec<u8> = b[o + 0x20..(o + 0x40).min(b.len())].iter().take_while(|c| **c != 0).copied().collect();
                if String::from_utf8_lossy(&nom).eq_ignore_ascii_case("LaunchMe") {
                    let taille_bloc = octets3(&b, o + 0x0D);
                    let emplacement = octets3(&b, o + 0x45) * taille_bloc;
                    let taille = octets3(&b, o + 0x11);
                    trouve = Some((emplacement, taille));
                    break;
                }
            }
            o += 0x48 + b[o + 0x43] as usize * 4;
        }
        if trouve.is_some() {
            break;
        }
        let suite = ((b[0x02] as u32) << 8) | b[0x03] as u32;
        if suite == 0xFFFF {
            break;
        }
        secteur = (racine + suite * taille_bloc_volume) / 2048;
    }
    let Some((emplacement, taille)) = trouve else { return Ok(None) };
    let contenu = lire(emplacement / 2048, taille.min(MAX_FICHIER) as usize)?;
    let mut m = Md5::new();
    m.update(&v);
    m.update(&contenu);
    Ok(Some(m.finalize().iter().map(|b| format!("{b:02x}")).collect()))
}

/// PSP (rc_hash_psp) : un .pbp entier ; un disque : PSP_GAME\PARAM.SFO puis PSP_GAME\SYSDIR\EBOOT.BIN.
pub fn empreinte_psp(chemin: &Path) -> Resultat<Option<String>> {
    if chemin.extension().is_some_and(|e| e.eq_ignore_ascii_case("pbp")) {
        let mut m = Md5::new();
        let mut f = std::fs::File::open(chemin)?;
        std::io::copy(&mut f, &mut m)?;
        return Ok(Some(m.finalize().iter().map(|b| format!("{b:02x}")).collect()));
    }
    let Some(p) = ouvrir(chemin)? else { return Ok(None) };
    let mut m = Md5::new();
    for f in ["PSP_GAME\\PARAM.SFO", "PSP_GAME\\SYSDIR\\EBOOT.BIN"] {
        let Some((s, taille)) = p.trouver(f)? else { return Ok(None) };
        m.update(p.lire(s, taille.min(MAX_FICHIER) as usize)?);
    }
    Ok(Some(m.finalize().iter().map(|b| format!("{b:02x}")).collect()))
}

/// PS3 (rc_hash_ps3) : un disque (.iso, .chd) → PS3_GAME\PARAM.SFO puis PS3_GAME\USRDIR\EBOOT.BIN ; un jeu en
/// dossier (on désigne son EBOOT.BIN) → le PARAM.SFO de son dossier (PS3_GAME\, TITRE\ ou à côté) puis l'EBOOT.BIN.
/// Un .iso ZIPPÉ n'est pas lisible sans le décompresser en entier : `None` (« à décompresser d'abord »).
pub fn empreinte_ps3(chemin: &Path) -> Resultat<Option<String>> {
    let ext = chemin.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let mut m = Md5::new();
    if ext == "iso" || ext == "chd" {
        let Some(p) = ouvrir(chemin)? else { return Ok(None) };
        for f in ["PS3_GAME\\PARAM.SFO", "PS3_GAME\\USRDIR\\EBOOT.BIN"] {
            let Some((s, taille)) = p.trouver(f)? else { return Ok(None) };
            m.update(p.lire(s, taille.min(MAX_FICHIER) as usize)?);
        }
    } else if chemin.file_name().is_some_and(|n| n.eq_ignore_ascii_case("EBOOT.BIN")) {
        let texte = chemin.to_string_lossy().replace('/', "\\");
        let dossier: PathBuf = if let Some(i) = texte.to_uppercase().find("PS3_GAME\\USRDIR\\") {
            PathBuf::from(&texte[..i + 9])
        } else if let Some(i) = texte.to_uppercase().find("USRDIR\\") {
            PathBuf::from(&texte[..i])
        } else {
            chemin.parent().unwrap_or(Path::new(".")).to_path_buf()
        };
        let Ok(sfo) = std::fs::read(dossier.join("PARAM.SFO")) else { return Ok(None) };
        m.update(&sfo);
        m.update(std::fs::read(chemin)?);
    } else {
        return Ok(None);
    }
    Ok(Some(m.finalize().iter().map(|b| format!("{b:02x}")).collect()))
}

/// Jaguar CD (rc_hash_jaguar_cd) : la 1re piste de la DEUXIÈME session (feuille .cue avec « REM SESSION 02 ») porte,
/// dans son 1er secteur brut, « ATARI APPROVED DATA HEADER ATRI » (ou sa version aux octets échangés) suivi de
/// l'adresse et de la taille du programme de démarrage ; on hache le programme. Les jeux « homebrew » (empreinte
/// connue 254487b5…) ont leur vrai code dans la piste 2 (« KART »).
pub fn empreinte_jaguar_cd(chemin: &Path) -> Resultat<Option<String>> {
    if !chemin.extension().is_some_and(|e| e.eq_ignore_ascii_case("cue")) {
        return Ok(None);
    }
    let pistes = pistes_du_cue(chemin);
    let brute = |p: &PisteCue| Piste {
        fichier: p.fichier.clone(),
        decalage: p.decalage,
        taille_secteur: 2352,
        entete: 0,
        premier: 0,
        index01: 0,
        chd: None,
        cso: None,
        utiles: 2352,
    };
    let Some(seconde) = pistes.iter().find(|p| p.session >= 2) else { return Ok(None) };
    let hacher = |p: &Piste, secteur: u32, decalage: usize, taille: usize, echange: bool| -> Resultat<Option<String>> {
        let mut m = Md5::new();
        let (mut s, mut o, mut reste) = (secteur, decalage, taille.min(MAX_FICHIER as usize));
        while reste > 0 {
            let mut b = p.lire(s, 2352)?;
            if b.len() < 2352 {
                return Ok(None); // pas assez de données
            }
            if echange {
                b.chunks_mut(2).for_each(|c| c.swap(0, 1));
            }
            let k = (2352 - o).min(reste);
            m.update(&b[o..o + k]);
            reste -= k;
            o = 0;
            s += 1;
        }
        Ok(Some(m.finalize().iter().map(|b| format!("{b:02x}")).collect()))
    };
    let p = brute(seconde);
    let b = p.lire(0, 2352)?;
    if b.len() < 2352 {
        return Ok(None);
    }
    let mut trouve = None;
    for i in 64..(2352 - 32 - 12) {
        if &b[i..i + 32] == b"TARA IPARPVODED TA AEHDAREA RT I" {
            let o = i + 36;
            let taille = ((b[o] as usize) << 16) | ((b[o + 1] as usize) << 24) | b[o + 2] as usize | ((b[o + 3] as usize) << 8);
            trouve = Some((o + 4, taille, true));
            break;
        }
        if &b[i..i + 32] == b"ATARI APPROVED DATA HEADER ATRI " {
            let o = i + 36;
            let taille = ((b[o] as usize) << 24) | ((b[o + 1] as usize) << 16) | ((b[o + 2] as usize) << 8) | b[o + 3] as usize;
            trouve = Some((o + 4, taille, false));
            break;
        }
    }
    let Some((o, taille, echange)) = trouve.filter(|(_, t, _)| *t > 0) else { return Ok(None) };
    let e = hacher(&p, 0, o, taille, echange)?;
    if e.as_deref() != Some("254487b59ab21bc005338e85cbf9fd2f") || !echange {
        return Ok(e);
    }
    // Homebrew : le code est dans la piste 2 (« KART »).
    let Some(p2) = pistes.iter().find(|p| p.numero == 2).map(brute) else { return Ok(e) };
    let b = p2.lire(0, 2352)?;
    if b.len() < 0xAA || &b[0x5E..0x66] != b"RT!IRTKA" {
        return Ok(None);
    }
    let taille = ((b[0xA6] as usize) << 16) | ((b[0xA7] as usize) << 24) | b[0xA8] as usize | ((b[0xA9] as usize) << 8);
    hacher(&p2, 0, 0xAA + 4, taille, true)
}

/// Neo Geo CD (rc_hash_neogeo_cd) : chaque programme « .PRG » cité par IPL.TXT, dans l'ordre.
pub fn empreinte_neogeo_cd(p: &Piste) -> Resultat<Option<String>> {
    let Some((s, _)) = p.trouver("IPL.TXT")? else { return Ok(None) };
    let ipl = p.lire(s, 1023)?;
    let texte = String::from_utf8_lossy(&ipl);
    let mut m = Md5::new();
    for ligne in texte.split('\n') {
        if ligne.starts_with('\x1a') || ligne.starts_with('\0') {
            break;
        }
        let Some(point) = ligne.find('.') else { continue };
        if !ligne[point..].to_uppercase().starts_with(".PRG") {
            continue;
        }
        let nom = &ligne[..point + 4];
        let Some((s, taille)) = p.trouver(nom)? else { return Ok(None) };
        m.update(p.lire(s, taille.min(MAX_FICHIER) as usize)?);
    }
    Ok(Some(m.finalize().iter().map(|b| format!("{b:02x}")).collect()))
}

/// PC Engine CD (rc_hash_pce_track) sur une piste de données : l'en-tête du secteur 1 (« PC Engine CD-ROM SYSTEM ») →
/// le titre (22 octets) puis le programme (secteur et nombre de secteurs) ; sinon un BOOT.BIN (GameExpress).
fn piste_pce(p: &Piste) -> Resultat<Option<String>> {
    let base = p.index01.max(0) as u32;
    let b = p.lire(base + 1, 128)?;
    if b.len() < 128 {
        return Ok(None);
    }
    let mut m = Md5::new();
    if &b[32..55] == b"PC Engine CD-ROM SYSTEM" {
        m.update(&b[106..128]);
        let secteur = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        for i in 0..b[3] as u32 {
            let mut s = p.lire(base + secteur + i, 2048)?;
            s.resize(2048, 0);
            m.update(&s);
        }
    } else if let Some((s, taille)) = p.trouver("BOOT.BIN")?.filter(|(_, t)| *t < MAX_FICHIER) {
        m.update(p.lire(s, taille as usize)?);
    } else {
        return Ok(None);
    }
    Ok(Some(m.finalize().iter().map(|b| format!("{b:02x}")).collect()))
}

/// PC Engine CD (rc_hash_pce_cd) : la 1re piste de données.
pub fn empreinte_pce_cd(chemin: &Path) -> Resultat<Option<String>> {
    match ouvrir_piste(chemin, Voulue::PremiereDeDonnees)? {
        Some(p) => piste_pce(&p),
        None => Ok(None),
    }
}

/// PC-FX (rc_hash_pcfx_cd) : la plus grande piste de données (sinon la piste 2) porte « PC-FX:Hu_CD-ROM » au secteur 0 ;
/// les 128 premiers octets du secteur 1, puis le programme (secteur et nombre en petit-boutiste). Certains disques
/// PC-FX se présentent comme des PC Engine CD.
pub fn empreinte_pcfx(chemin: &Path) -> Resultat<Option<String>> {
    let marque = |p: &Piste| -> Resultat<bool> { Ok(p.lire(p.index01.max(0) as u32, 32)?.starts_with(b"PC-FX:Hu_CD-ROM")) };
    let mut piste = ouvrir_piste(chemin, Voulue::PlusGrandeDeDonnees)?;
    if !matches!(&piste, Some(p) if marque(p)?) {
        piste = ouvrir_piste(chemin, Voulue::Numero(2))?;
    }
    let Some(p) = piste else { return Ok(None) };
    let base = p.index01.max(0) as u32;
    if !marque(&p)? {
        let b = p.lire(base + 1, 128)?;
        return if b.len() >= 55 && &b[32..55] == b"PC Engine CD-ROM SYSTEM" { piste_pce(&p) } else { Ok(None) };
    }
    let b = p.lire(base + 1, 128)?;
    if b.len() < 128 {
        return Ok(None);
    }
    let mut m = Md5::new();
    m.update(&b);
    let secteur = ((b[34] as u32) << 16) | ((b[33] as u32) << 8) | b[32] as u32;
    let nombre = ((b[38] as u32) << 16) | ((b[37] as u32) << 8) | b[36] as u32;
    for i in 0..nombre.min(32 * 1024) {
        let mut s = p.lire(base + secteur + i, 2048)?;
        s.resize(2048, 0);
        m.update(&s);
    }
    Ok(Some(m.finalize().iter().map(|b| format!("{b:02x}")).collect()))
}

/// Nintendo DS (rc_hash_nintendo_ds) : l'en-tête (0x160), le code arm9, le code arm7, puis l'icône et les titres
/// (0xA00, complétés de zéros) ; un en-tête SuperCard de 512 octets est ignoré.
pub fn empreinte_nds(chemin: &Path) -> Resultat<Option<String>> {
    let mut f = std::fs::File::open(chemin)?;
    let mut entete = lire_a(&mut f, 0, 512);
    let mut decalage = 0u64;
    if entete[..4] == [0x2E, 0, 0, 0xEA] && entete[0xB0..0xB4] == [0x44, 0x46, 0x96, 0] {
        decalage = 512;
        entete = lire_a(&mut f, 512, 512);
    }
    let le32 = |i: usize| u32::from_le_bytes([entete[i], entete[i + 1], entete[i + 2], entete[i + 3]]) as u64;
    let (arm9, t9, arm7, t7, icone) = (le32(0x20), le32(0x2C), le32(0x30), le32(0x3C), le32(0x68));
    if t9 + t7 > 16 * 1024 * 1024 {
        return Ok(None); // pas une ROM DS
    }
    let mut m = Md5::new();
    m.update(&entete[..0x160]);
    m.update(lire_a(&mut f, arm9 + decalage, t9 as usize));
    m.update(lire_a(&mut f, arm7 + decalage, t7 as usize));
    m.update(lire_a(&mut f, icone + decalage, 0xA00));
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

/// PlayStation (rc_hash_psx) : le nom du programme de démarrage (`BOOT = cdrom:\SLUS_005.94;1` dans SYSTEM.CNF, sinon
/// PSX.EXE), puis son contenu (taille lue dans l'en-tête « PS-X EXE », + 2048).
pub fn empreinte_psx(p: &Piste) -> Resultat<Option<String>> {
    empreinte_playstation(p, "BOOT", "cdrom:", true)
}

/// PlayStation 2 (rc_hash_ps2) : pareil avec `BOOT2 = cdrom0:\…` ; la taille est celle du fichier.
pub fn empreinte_ps2(p: &Piste) -> Resultat<Option<String>> {
    empreinte_playstation(p, "BOOT2", "cdrom0:", false)
}

fn empreinte_playstation(p: &Piste, cle: &str, prefixe: &str, ps1: bool) -> Resultat<Option<String>> {
    let mut exe = String::new();
    let mut trouve = None;
    if let Some((s, _)) = p.trouver("SYSTEM.CNF")? {
        let cnf = String::from_utf8_lossy(&p.lire(s, 2047)?).to_string();
        for ligne in cnf.lines() {
            let l = ligne.trim_start();
            if let Some(r) = l.strip_prefix(cle) {
                let r = r.trim_start();
                if let Some(r) = r.strip_prefix('=') {
                    let r = r.trim_start();
                    let r = r.strip_prefix(prefixe).unwrap_or(r).trim_start_matches('\\');
                    exe = r.split(|c: char| c.is_whitespace() || c == ';').next().unwrap_or("").to_string();
                    trouve = p.trouver(&exe)?;
                    break;
                }
            }
        }
    }
    if trouve.is_none() && ps1 {
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
    if ps1 && entete.starts_with(b"PS-X EX") {
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
            assert_eq!(empreinte_ps2(&p).unwrap(), None, "pas de BOOT2 : pas un disque PS2");
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
    fn gamecube_les_en_tetes_puis_les_segments_de_main_dol() {
        let mut d = vec![0u8; 0x8000];
        d[0x1C..0x20].copy_from_slice(&[0xC2, 0x33, 0x9F, 0x3D]);
        // Apploader : corps 0x10, fin 0 → en-têtes jusqu'à 0x2440 + 0x20 + 0x10.
        d[0x2440 + 0x14..0x2440 + 0x18].copy_from_slice(&0x10u32.to_be_bytes());
        // main.dol à 0x3000 : un seul segment de code, à 0x4000, 0x100 octets.
        d[0x420..0x424].copy_from_slice(&0x3000u32.to_be_bytes());
        d[0x3000..0x3004].copy_from_slice(&0x4000u32.to_be_bytes());
        d[0x3090..0x3094].copy_from_slice(&0x100u32.to_be_bytes());
        d[0x4000..0x4100].iter_mut().enumerate().for_each(|(i, b)| *b = i as u8);
        let mut attendu = d[..0x2440 + 0x20 + 0x10].to_vec();
        attendu.extend(&d[0x4000..0x4100]);
        assert_eq!(empreinte_gamecube_flux(&mut std::io::Cursor::new(d.clone())), Some(md5_hex(&attendu)));
        assert_eq!(empreinte_wii_flux(&mut std::io::Cursor::new(d)), None, "pas un disque Wii");
    }

    #[test]
    fn trois_do_le_volume_opera_puis_launchme() {
        let d = tempfile::tempdir().unwrap();
        let mut s = vec![vec![0u8; 2048]; 6];
        // Volume : identifiant Opera, bloc de 2048, dossier racine au bloc 2.
        s[0][..7].copy_from_slice(&[0x01, 0x5A, 0x5A, 0x5A, 0x5A, 0x5A, 0x01]);
        s[0][0x4D..0x50].copy_from_slice(&[0, 0x08, 0x00]);
        s[0][0x65..0x68].copy_from_slice(&[0, 0, 2]);
        // Dossier : entrées de 0x14 à 0x14 + 2*0x48 ; un fichier « Autre », puis « LaunchMe » au bloc 4, 2100 octets.
        let dossier = &mut s[2];
        dossier[0x02..0x04].copy_from_slice(&[0xFF, 0xFF]);
        dossier[0x0D..0x10].copy_from_slice(&[0, 0, 0x14 + 2 * 0x48]);
        dossier[0x12..0x14].copy_from_slice(&[0, 0x14]);
        for (i, (nom, bloc)) in [("Autre", 3u8), ("LaunchMe", 4u8)].iter().enumerate() {
            let o = 0x14 + i * 0x48;
            dossier[o + 0x03] = 0x02;
            dossier[o + 0x0D..o + 0x10].copy_from_slice(&[0, 0x08, 0x00]);
            dossier[o + 0x11..o + 0x14].copy_from_slice(&[0, 0x08, 0x34]);
            dossier[o + 0x20..o + 0x20 + nom.len()].copy_from_slice(nom.as_bytes());
            dossier[o + 0x45..o + 0x48].copy_from_slice(&[0, 0, *bloc]);
        }
        s[4].iter_mut().for_each(|b| *b = 7);
        s[5].iter_mut().for_each(|b| *b = 9);
        let mut attendu = s[0][..132].to_vec();
        attendu.extend(&s[4]);
        attendu.extend(&s[5][..2100 - 2048]);
        let iso = d.path().join("jeu.iso");
        std::fs::write(&iso, s.concat()).unwrap();
        let p = ouvrir(&iso).unwrap().unwrap();
        assert_eq!(empreinte_3do(&p).unwrap(), Some(md5_hex(&attendu)));
    }

    #[test]
    fn une_feuille_cue_donne_toutes_ses_pistes() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("jeu.bin"), vec![0u8; 2352 * 300]).unwrap();
        std::fs::write(d.path().join("piste3.bin"), vec![0u8; 2352 * 50]).unwrap();
        let cue = d.path().join("jeu.cue");
        std::fs::write(
            &cue,
            "FILE \"jeu.bin\" BINARY\n  TRACK 01 AUDIO\n    INDEX 01 00:00:00\n  TRACK 02 MODE1/2352\n    INDEX 00 00:01:00\n    INDEX 01 00:02:00\nFILE \"piste3.bin\" BINARY\n  TRACK 03 MODE1/2352\n    INDEX 01 00:00:00\n",
        )
        .unwrap();
        let l = pistes_du_cue(&cue);
        assert_eq!(l.len(), 3);
        assert!(l[0].audio && !l[1].audio);
        assert_eq!((l[1].decalage, l[1].longueur), (150 * 2352, 150 * 2352), "INDEX 01 à 00:02:00, jusqu'à la fin du fichier");
        assert_eq!(l[2].longueur, 50 * 2352);
        // La 1re de données : la piste 2 ; la plus grande de données : la piste 2 aussi.
        let p = ouvrir_piste(&cue, Voulue::PremiereDeDonnees).unwrap().unwrap();
        assert_eq!(p.decalage, 150 * 2352);
        assert!(ouvrir_piste(&cue, Voulue::Numero(1)).unwrap().is_none(), "piste audio : rien à lire");
    }

    #[test]
    fn un_cso_se_lit_bloc_par_bloc() {
        use std::io::Write;
        let d = tempfile::tempdir().unwrap();
        // Une image de 3 blocs de 2048 : bloc 0 compressé, bloc 1 stocké tel quel, bloc 2 compressé.
        let image: Vec<u8> = (0..3 * 2048).map(|i| (i % 253) as u8).collect();
        let mut blocs = Vec::new();
        for (i, morceau) in image.chunks(2048).enumerate() {
            if i == 1 {
                blocs.push((morceau.to_vec(), true));
            } else {
                let mut z = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
                z.write_all(morceau).unwrap();
                blocs.push((z.finish().unwrap(), false));
            }
        }
        let entete = 24 + 4 * 4;
        let mut index = Vec::new();
        let mut pos = entete as u32;
        for (b, tel_quel) in &blocs {
            index.push(pos | if *tel_quel { 0x8000_0000 } else { 0 });
            pos += b.len() as u32;
        }
        index.push(pos);
        let mut o = b"CISO".to_vec();
        o.extend(24u32.to_le_bytes());
        o.extend((image.len() as u64).to_le_bytes());
        o.extend(2048u32.to_le_bytes());
        o.extend([1u8, 0, 0, 0]);
        index.iter().for_each(|x| o.extend(x.to_le_bytes()));
        blocs.iter().for_each(|(b, _)| o.extend(b));
        let f = d.path().join("jeu.cso");
        std::fs::write(&f, &o).unwrap();
        let p = ouvrir(&f).unwrap().unwrap();
        assert_eq!(p.lire(0, 3 * 2048).unwrap(), image, "tout, à cheval sur les blocs");
        assert_eq!(p.lire(1, 100).unwrap(), image[2048..2148].to_vec());
    }

    #[test]
    fn ps3_un_jeu_en_dossier() {
        let d = tempfile::tempdir().unwrap();
        let jeu = d.path().join("BLES01234").join("PS3_GAME");
        std::fs::create_dir_all(jeu.join("USRDIR")).unwrap();
        std::fs::write(jeu.join("PARAM.SFO"), b"SFO").unwrap();
        std::fs::write(jeu.join("USRDIR").join("EBOOT.BIN"), b"ELF").unwrap();
        assert_eq!(empreinte_ps3(&jeu.join("USRDIR").join("EBOOT.BIN")).unwrap(), Some(md5_hex(b"SFOELF")));
        assert_eq!(empreinte_ps3(&d.path().join("jeu.zip")).unwrap(), None, "zip : à décompresser d'abord");
    }

    #[test]
    fn jaguar_cd_la_deuxieme_session() {
        let d = tempfile::tempdir().unwrap();
        // Session 1 : une piste audio ; session 2 : la piste de données (brute) avec l'en-tête Atari.
        std::fs::write(d.path().join("a.bin"), vec![0u8; 2352 * 4]).unwrap();
        let mut s = vec![0u8; 2352 * 2];
        s[64..64 + 32].copy_from_slice(b"ATARI APPROVED DATA HEADER ATRI ");
        // Adresse de chargement à i+32 = 96, taille (big-endian) à i+36 = 100 ; le programme commence à 104.
        s[96..100].copy_from_slice(&0x0080_4000u32.to_be_bytes());
        s[100..104].copy_from_slice(&3000u32.to_be_bytes());
        s[104..104 + 3000].iter_mut().enumerate().for_each(|(i, b)| *b = (i % 7) as u8);
        std::fs::write(d.path().join("b.bin"), &s).unwrap();
        let cue = d.path().join("jeu.cue");
        std::fs::write(
            &cue,
            "REM SESSION 01\nFILE \"a.bin\" BINARY\n  TRACK 01 AUDIO\n    INDEX 01 00:00:00\nREM SESSION 02\nFILE \"b.bin\" BINARY\n  TRACK 02 AUDIO\n    INDEX 01 00:00:00\n",
        )
        .unwrap();
        assert_eq!(pistes_du_cue(&cue)[1].session, 2);
        assert_eq!(empreinte_jaguar_cd(&cue).unwrap(), Some(md5_hex(&s[104..104 + 3000])));
    }

    #[test]
    fn nintendo_ds_en_tete_arm9_arm7_et_icone() {
        let d = tempfile::tempdir().unwrap();
        let mut rom = vec![0u8; 0x4000];
        rom[0x20..0x24].copy_from_slice(&0x1000u32.to_le_bytes());
        rom[0x2C..0x30].copy_from_slice(&0x100u32.to_le_bytes());
        rom[0x30..0x34].copy_from_slice(&0x2000u32.to_le_bytes());
        rom[0x3C..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        rom[0x68..0x6C].copy_from_slice(&0x3800u32.to_le_bytes());
        rom[0x1000..0x1100].fill(9);
        rom[0x2000..0x2080].fill(7);
        rom[0x3800..0x4000].fill(5);
        let mut attendu = rom[..0x160].to_vec();
        attendu.extend(&rom[0x1000..0x1100]);
        attendu.extend(&rom[0x2000..0x2080]);
        attendu.extend(&rom[0x3800..0x4000]);
        attendu.extend(vec![0u8; 0xA00 - 0x800]);
        let f = d.path().join("jeu.nds");
        std::fs::write(&f, &rom).unwrap();
        assert_eq!(empreinte_nds(&f).unwrap(), Some(md5_hex(&attendu)), "icône incomplète : complétée de zéros");
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
        assert!(ouvrir(&d.path().join("jeu.xyz")).unwrap().is_none(), "format inconnu : rien");
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
