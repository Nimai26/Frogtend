//! Lire les conteneurs Switch SANS RIEN DEVINER (lot 6 de Ryubing, ✅ Seb 16/11 : mises à jour et DLC posés
//! automatiquement). Relu dans LibHac (la bibliothèque de Ryubing, git.ryujinx.app/projects/LibHac) :
//! - un .nsp est un PFS0 : `PFS0`, nombre d'entrées (u32), taille de la table des noms (u32), 4 octets réservés, puis
//!   une entrée de 0x18 octets par fichier (début u64, taille u64, place du nom u32, réservé u32), puis les noms ; les
//!   données commencent juste après (FsSystem/Impl/PartitionFileSystemFormats.cs) ;
//! - un .xci : l'en-tête `HEAD` à 0x100 (ou à 0x1100 quand la copie garde la zone des clés de 0x1000 octets, Xci.cs),
//!   la partition racine à l'adresse lue en 0x130 ; c'est un HFS0 (entrées de 0x40 octets : début, taille, nom, taille
//!   hachée, début haché, empreinte), dont la partition `secure` porte le jeu (XciHeader.cs, Xci.cs) ;
//! - l'en-tête d'un NCA : ses 0xC00 premiers octets, chiffrés en AES-128-XTS par la clé `header_key` (32 octets : la
//!   clé des données puis celle du « tweak »), par secteurs de 0x200 numérotés depuis 0, le numéro écrit en GRAND-
//!   BOUTISTE dans le tweak (Tools/FsSystem/Aes128XtsTransform.cs, `FillArrayFromSector`) — ce n'est pas le XTS
//!   habituel ; magie `NCA3` en 0x200, type de contenu en 0x205 (Program, Meta, Control, Manual, Data, PublicData :
//!   NcaStructs.cs), identifiant du titre en 0x210, petit-boutiste (NcaUtils/NcaHeader.cs).
//!
//! Lecture seule et bornée : seules les tables et les 0xC00 octets de chaque NCA sont lus, jamais tout un fichier de
//! 16 Go. La clé ne s'affiche ni ne se journalise jamais (`CleEntete` n'a pas d'affichage de son contenu).

use crate::erreurs::{Erreur, Resultat};
use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
use aes::Aes128;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// La taille de l'en-tête d'un NCA.
pub const TAILLE_ENTETE: usize = 0xC00;
const SECTEUR: usize = 0x200;
/// Bornes de lecture d'une table de partition (un fichier abîmé ne fait jamais lire des Go).
const MAX_ENTREES: usize = 4096;
const MAX_NOMS: usize = 1 << 20;
/// Un dossier de jeux : au plus ce nombre de conteneurs lus à chaque lancement.
pub const MAX_VOISINS: usize = 400;

/// Un fichier dans un conteneur : son nom et sa place ABSOLUE dans le fichier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entree {
    pub nom: String,
    pub debut: u64,
    pub taille: u64,
}

fn lire_a(f: &mut std::fs::File, pos: u64, n: usize) -> Resultat<Vec<u8>> {
    let mut b = vec![0u8; n];
    f.seek(SeekFrom::Start(pos))?;
    f.read_exact(&mut b).map_err(|_| Erreur::Disque("Conteneur Switch tronqué.".into()))?;
    Ok(b)
}

fn u32_le(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn u64_le(b: &[u8], o: usize) -> u64 {
    let mut a = [0u8; 8];
    a.copy_from_slice(&b[o..o + 8]);
    u64::from_le_bytes(a)
}

/// La table d'une partition PFS0 ou HFS0 qui commence en `debut` (fichier de `longueur` octets).
fn table_partition(f: &mut std::fs::File, debut: u64, longueur: u64) -> Resultat<Vec<Entree>> {
    let tete = lire_a(f, debut, 0x10)?;
    let taille_entree = match &tete[..4] {
        b"PFS0" => 0x18,
        b"HFS0" => 0x40,
        _ => return Err(Erreur::Refus("Ce fichier n'est pas un conteneur Switch lisible (ni PFS0 ni HFS0).".into())),
    };
    let n = u32_le(&tete, 4) as usize;
    let taille_noms = u32_le(&tete, 8) as usize;
    let abime = || Erreur::Refus("Conteneur Switch abîmé (sa table sort du fichier).".into());
    if n > MAX_ENTREES || taille_noms > MAX_NOMS {
        return Err(Erreur::Refus("Table d'un conteneur Switch invraisemblable : pas lue.".into()));
    }
    let taille_table = (n * taille_entree + taille_noms) as u64;
    let debut_table = debut.checked_add(0x10).ok_or_else(abime)?;
    let donnees = debut_table.checked_add(taille_table).filter(|d| *d <= longueur).ok_or_else(abime)?;
    let table = lire_a(f, debut_table, taille_table as usize)?;
    let noms = &table[n * taille_entree..];
    let mut l = Vec::with_capacity(n);
    for i in 0..n {
        let e = &table[i * taille_entree..(i + 1) * taille_entree];
        let (o, t, place) = (u64_le(e, 0), u64_le(e, 8), u32_le(e, 16) as usize);
        let nom: String = noms.get(place..).unwrap_or_default().iter().take_while(|b| **b != 0).map(|b| *b as char).collect();
        let abs = donnees.checked_add(o).ok_or_else(|| Erreur::Refus("Conteneur Switch abîmé.".into()))?;
        if nom.is_empty() || abs.checked_add(t).is_none_or(|fin| fin > longueur) {
            return Err(Erreur::Refus("Conteneur Switch abîmé (une entrée sort du fichier).".into()));
        }
        l.push(Entree { nom, debut: abs, taille: t });
    }
    Ok(l)
}

/// La forme d'un conteneur, lue dans son contenu (`PFS0` ou en-tête `HEAD`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Forme {
    Nsp,
    Xci,
}

impl Forme {
    /// L'extension que Ryubing attend, EN MINUSCULES : il choisit la lecture d'après l'extension (tout ce qui n'est pas
    /// `.xci` est lu comme un PFS0) et la compare avec la casse pour les DLC (PartitionFileSystemUtils.cs L19-31,
    /// ContentManager.cs L225-236) : un `DLC.NSP` ne serait pas chargé.
    pub fn extension(self) -> &'static str {
        match self {
            Forme::Nsp => "nsp",
            Forme::Xci => "xci",
        }
    }
}

/// La forme et les fichiers d'un .nsp (PFS0) ou de la partition `secure` d'un .xci (HFS0), reconnus à leur contenu,
/// pas à leur extension.
pub fn ouvrir(chemin: &Path) -> Resultat<(Forme, Vec<Entree>)> {
    let mut f = std::fs::File::open(chemin)?;
    let longueur = f.metadata()?.len();
    let magie = lire_a(&mut f, 0, 4)?;
    if magie == b"PFS0" {
        return Ok((Forme::Nsp, table_partition(&mut f, 0, longueur)?));
    }
    for base in [0u64, 0x1000] {
        if longueur < base + 0x140 {
            continue;
        }
        let h = lire_a(&mut f, base + 0x100, 0x40)?;
        if &h[..4] != b"HEAD" {
            continue;
        }
        let racine = base.checked_add(u64_le(&h, 0x30)).filter(|r| *r < longueur).ok_or_else(|| Erreur::Refus("Ce .xci est abîmé (sa partition racine sort du fichier).".into()))?;
        let partitions = table_partition(&mut f, racine, longueur)?;
        let secure = partitions.iter().find(|p| p.nom == "secure").ok_or_else(|| Erreur::Refus("Ce .xci n'a pas de partition « secure ».".into()))?;
        return Ok((Forme::Xci, table_partition(&mut f, secure.debut, longueur)?));
    }
    Err(Erreur::Refus("Ce fichier n'est ni un .nsp ni un .xci lisible.".into()))
}

/// Les fichiers d'un conteneur (voir `ouvrir`).
pub fn entrees(chemin: &Path) -> Resultat<Vec<Entree>> {
    ouvrir(chemin).map(|(_, l)| l)
}

/// La clé des en-têtes de NCA (`header_key` de `prod.keys`). Son contenu ne s'affiche jamais.
#[derive(Clone)]
pub struct CleEntete {
    donnees: Aes128,
    tweak: Aes128,
}

impl std::fmt::Debug for CleEntete {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CleEntete(…)")
    }
}

impl CleEntete {
    pub fn depuis(octets: &[u8; 32]) -> Self {
        CleEntete { donnees: Aes128::new(GenericArray::from_slice(&octets[..16])), tweak: Aes128::new(GenericArray::from_slice(&octets[16..])) }
    }
}

fn hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok()).collect()
}

/// `header_key = <64 chiffres hexadécimaux>` dans le texte d'un `prod.keys`.
pub fn cle_entete_du_texte(texte: &str) -> Option<CleEntete> {
    texte.lines().find_map(|l| {
        let (nom, valeur) = l.split_once('=')?;
        if !nom.trim().eq_ignore_ascii_case("header_key") {
            return None;
        }
        let o: [u8; 32] = hex(valeur.trim())?.try_into().ok()?;
        Some(CleEntete::depuis(&o))
    })
}

/// La clé des en-têtes lue dans un `prod.keys` (absent, illisible ou sans `header_key` : `None`).
pub fn cle_entete(prod_keys: &Path) -> Option<CleEntete> {
    if std::fs::metadata(prod_keys).ok()?.len() > 1 << 20 {
        return None;
    }
    cle_entete_du_texte(&std::fs::read_to_string(prod_keys).ok()?)
}

/// Multiplie le tweak par α dans GF(2^128), petit-boutiste (le XTS habituel, `FillTweakBuffer`).
fn fois_alpha(t: &mut [u8; 16]) {
    let retenue = t[15] >> 7;
    for i in (1..16).rev() {
        t[i] = (t[i] << 1) | (t[i - 1] >> 7);
    }
    t[0] <<= 1;
    if retenue != 0 {
        t[0] ^= 0x87;
    }
}

/// (Dé)chiffre un secteur de 0x200 octets à la façon de Nintendo (numéro de secteur grand-boutiste dans le tweak).
fn xts_secteur(cle: &CleEntete, secteur: &mut [u8], numero: u64, chiffrer: bool) {
    let mut t = [0u8; 16];
    t[8..].copy_from_slice(&numero.to_be_bytes());
    cle.tweak.encrypt_block(GenericArray::from_mut_slice(&mut t));
    for bloc in secteur.chunks_exact_mut(16) {
        for (b, x) in bloc.iter_mut().zip(t.iter()) {
            *b ^= x;
        }
        if chiffrer {
            cle.donnees.encrypt_block(GenericArray::from_mut_slice(bloc));
        } else {
            cle.donnees.decrypt_block(GenericArray::from_mut_slice(bloc));
        }
        for (b, x) in bloc.iter_mut().zip(t.iter()) {
            *b ^= x;
        }
        fois_alpha(&mut t);
    }
}

/// Le type de contenu d'un NCA (`NcaContentType` de LibHac, dans cet ordre).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeContenu {
    Programme,
    Meta,
    Controle,
    Manuel,
    Donnees,
    DonneesPubliques,
}

impl TypeContenu {
    fn de(octet: u8) -> Option<Self> {
        Some(match octet {
            0 => Self::Programme,
            1 => Self::Meta,
            2 => Self::Controle,
            3 => Self::Manuel,
            4 => Self::Donnees,
            5 => Self::DonneesPubliques,
            _ => return None,
        })
    }
}

/// Ce que dit l'en-tête d'un NCA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnteteNca {
    pub type_contenu: TypeContenu,
    pub title_id: u64,
}

/// Les 0x400 premiers octets clairs d'un en-tête (déjà en clair, ou déchiffrés comme LibHac : secteurs 0 et 1 suffisent
/// pour la magie, le type et l'identifiant).
fn lire_entete_clair(b: &[u8]) -> Option<EnteteNca> {
    if &b[0x200..0x203] != b"NCA" || !matches!(b[0x203], b'2' | b'3') {
        return None;
    }
    Some(EnteteNca { type_contenu: TypeContenu::de(b[0x205])?, title_id: u64_le(b, 0x210) })
}

/// Déchiffre l'en-tête d'un NCA (`bloc` : ses 0xC00 premiers octets). `None` : pas un NCA, ou une autre clé.
pub fn dechiffrer_entete(cle: &CleEntete, bloc: &[u8]) -> Option<EnteteNca> {
    if bloc.len() < TAILLE_ENTETE {
        return None;
    }
    // Un en-tête déjà en clair (rare : certains outils de copie) se lit tel quel, comme LibHac (`CheckIfDecrypted`).
    if let Some(e) = lire_entete_clair(bloc) {
        return Some(e);
    }
    let mut b = bloc[..2 * SECTEUR].to_vec();
    for (i, s) in b.chunks_exact_mut(SECTEUR).enumerate() {
        xts_secteur(cle, s, i as u64, false);
    }
    lire_entete_clair(&b)
}

/// Chiffre un en-tête comme Nintendo (NCA3 : chaque secteur porte son numéro) — pour fabriquer des NCA de test, et
/// pour l'empreinte de la clé du cache (`empreinte_cle`, jamais réversible).
pub fn chiffrer_entete(cle: &CleEntete, clair: &[u8]) -> Vec<u8> {
    let mut b = clair.to_vec();
    for (i, s) in b.chunks_exact_mut(SECTEUR).enumerate() {
        xts_secteur(cle, s, i as u64, true);
    }
    b
}

/// Un NCA d'un conteneur : son chemin dans le conteneur, tel que Ryubing l'écrit (`/<nom>.nca`), et son en-tête.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NcaLu {
    pub chemin: String,
    pub entete: EnteteNca,
}

/// La forme d'un conteneur, ses NCA dont l'en-tête se lit avec cette clé, et le nombre de ceux qui ne se lisent pas
/// (Ryubing les ignore aussi, `TryOpenNca`).
fn lire_ncas(chemin: &Path, cle: &CleEntete) -> Resultat<(Forme, Vec<NcaLu>, usize)> {
    let (forme, l) = ouvrir(chemin)?;
    let mut f = std::fs::File::open(chemin)?;
    let (mut lus, mut illisibles) = (Vec::new(), 0);
    for e in l.iter().filter(|e| e.nom.to_ascii_lowercase().ends_with(".nca")) {
        if e.taille < TAILLE_ENTETE as u64 {
            illisibles += 1;
            continue;
        }
        let bloc = lire_a(&mut f, e.debut, TAILLE_ENTETE)?;
        match dechiffrer_entete(cle, &bloc) {
            Some(entete) => lus.push(NcaLu { chemin: format!("/{}", e.nom), entete }),
            None => illisibles += 1,
        }
    }
    Ok((forme, lus, illisibles))
}

/// Les NCA d'un conteneur dont l'en-tête se lit avec cette clé, et le nombre de ceux qui ne se lisent pas.
pub fn ncas(chemin: &Path, cle: &CleEntete) -> Resultat<(Vec<NcaLu>, usize)> {
    lire_ncas(chemin, cle).map(|(_, l, n)| (l, n))
}

/// L'identifiant de base d'un titre, comme Ryubing (`GetProgramIdBase` : `& ~0x1FFF`) : le jeu, sa mise à jour
/// (+0x800) et ses DLC (+0x1000 + n) y mènent tous.
pub fn base(title_id: u64) -> u64 {
    title_id & !0x1FFF
}

/// Ce qu'est un conteneur, lu dans ses NCA.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sorte {
    Jeu,
    MiseAJour,
    /// Les NCA PublicData : (chemin dans le conteneur, identifiant du DLC).
    Dlc(Vec<(String, u64)>),
}

/// Un conteneur Switch lu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContenuLu {
    pub chemin: PathBuf,
    pub sorte: Sorte,
    pub base: u64,
    /// La version d'une mise à jour, lue dans le nom (`[v786432]`, `v196608`) ; `None` : le nom ne la dit pas. C'est le
    /// numéro BRUT de Nintendo (v786432 = la 1.5.0 de Zelda) : il sert à trier, jamais à être affiché.
    pub version: Option<u32>,
    /// Sa forme lue (PFS0 ou HEAD).
    pub forme: Forme,
}

/// La version écrite dans un nom : un « v » seul suivi de chiffres (`[v786432]`, `_v196608_`, ` v65536`) ; s'il y en a
/// plusieurs, la plus grande.
pub fn version_du_nom(nom: &str) -> Option<u32> {
    let c: Vec<char> = nom.chars().collect();
    let mut trouvee: Option<u32> = None;
    for i in 0..c.len() {
        if !matches!(c[i], 'v' | 'V') || (i > 0 && c[i - 1].is_alphanumeric()) {
            continue;
        }
        let chiffres: String = c[i + 1..].iter().take_while(|x| x.is_ascii_digit()).collect();
        let apres = c.get(i + 1 + chiffres.len());
        if !chiffres.is_empty() && apres.is_none_or(|x| !x.is_alphanumeric()) {
            if let Ok(v) = chiffres.parse::<u32>() {
                trouvee = Some(trouvee.map_or(v, |t| t.max(v)));
            }
        }
    }
    trouvee
}

/// Le nom d'un conteneur à montrer : les `_` en espaces, sans le numéro de version brut (`v196608` seul ; `[v786432]`
/// est retiré par l'interface avec les autres étiquettes), les espaces en trop ôtés.
pub fn nom_a_montrer(chemin: &Path) -> String {
    let brut = chemin.file_stem().map(|n| n.to_string_lossy().replace('_', " ")).unwrap_or_default();
    brut.split_whitespace()
        .filter(|m| !(m.len() > 1 && m.starts_with(['v', 'V']) && m[1..].chars().all(|c| c.is_ascii_digit())))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Lit ce qu'est un conteneur, dans cet ordre : un jeu (un NCA Meta à l'identifiant du jeu, `…000`), sinon une mise à
/// jour (un Meta `…800`), sinon un DLC (au moins un NCA PublicData). Relevé sur les vrais fichiers de Zelda (16/11) :
/// le NCA Program d'une mise à jour porte l'identifiant du JEU (01007ef00011e000), seul son Meta porte +0x800 ; et un
/// .xci peut grouper le jeu, une mise à jour (et des DLC) : c'est alors un jeu. `None` : rien de lisible avec cette clé,
/// ou plusieurs jeux différents (ambigu : rien n'est deviné).
pub fn lire_contenu(chemin: &Path, cle: &CleEntete) -> Resultat<Option<ContenuLu>> {
    let (forme, l, _) = lire_ncas(chemin, cle)?;
    let version = chemin.file_name().and_then(|n| version_du_nom(&n.to_string_lossy()));
    let une_base = |ids: &mut dyn Iterator<Item = u64>| -> Option<u64> {
        let mut b: Vec<u64> = ids.map(base).collect();
        b.sort_unstable();
        b.dedup();
        (b.len() == 1).then(|| b[0])
    };
    let lu = |sorte: Sorte, b: u64| Some(ContenuLu { chemin: chemin.to_path_buf(), sorte, base: b, version, forme });
    let metas: Vec<u64> = l.iter().filter(|n| n.entete.type_contenu == TypeContenu::Meta).map(|n| n.entete.title_id).collect();
    let jeux: Vec<u64> = metas.iter().copied().filter(|t| t & 0x1FFF == 0).collect();
    let majs: Vec<u64> = metas.iter().copied().filter(|t| t & 0x1FFF == 0x800).collect();
    if !jeux.is_empty() {
        return Ok(une_base(&mut jeux.into_iter()).and_then(|b| lu(Sorte::Jeu, b)));
    }
    if !majs.is_empty() {
        return Ok(une_base(&mut majs.into_iter()).and_then(|b| lu(Sorte::MiseAJour, b)));
    }
    let publiques: Vec<&NcaLu> = l.iter().filter(|n| n.entete.type_contenu == TypeContenu::DonneesPubliques).collect();
    if publiques.is_empty() {
        return Ok(None);
    }
    let Some(b) = une_base(&mut publiques.iter().map(|n| n.entete.title_id)) else { return Ok(None) };
    Ok(lu(Sorte::Dlc(publiques.iter().map(|n| (n.chemin.clone(), n.entete.title_id)).collect()), b))
}

/// Ce qui va avec un jeu, trouvé dans le dossier de son fichier (et, avec `contenus_pour_dans`, ailleurs).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContenusDuJeu {
    pub base: u64,
    /// La mise à jour la plus récente de ce jeu (version du nom ; sans version, après celles qui en ont une).
    pub maj: Option<ContenuLu>,
    /// Les autres mises à jour de ce jeu, plus anciennes (montrées, jamais posées).
    pub majs_anciennes: Vec<ContenuLu>,
    pub dlc: Vec<ContenuLu>,
    /// Les conteneurs regardés qui n'ont pas pu être lus (abîmés, autre clé) : leurs noms.
    pub illisibles: Vec<String>,
    /// Les mises à jour et DLC de CE jeu dont l'extension n'est pas celle, en minuscules, de leur forme (`.NSP`, un
    /// PFS0 nommé `.xci`…) : Ryubing ne les lirait pas, ils ne sont pas posés. Leurs noms.
    pub mauvaise_extension: Vec<String>,
    /// Les copies en double d'une mise à jour (même version) ou d'un DLC (mêmes identifiants) déjà trouvés : une seule
    /// sert, celle du dossier du jeu d'abord. Leurs noms (jamais un chemin).
    pub doubles: Vec<String>,
    /// Plafond de LECTURE atteint (`MAX_VOISINS` fichiers lus, ou `BUDGET` de temps) : la suite sera lue les fois
    /// suivantes (grâce au cache).
    pub tronque: bool,
    /// Plafond du PARCOURS atteint (profondeur, nombre de dossiers, de fichiers) : ce qui est au-delà n'est jamais lu.
    pub parcours_incomplet: bool,
    /// Les conteneurs lisibles des dossiers utiles qui sont à un AUTRE jeu (jeu, mise à jour ou DLC) : Eden lit tout un
    /// dossier, il les verra aussi (à dire).
    pub autres_jeux: usize,
    /// Ce qu'a coûté la recherche (pour le journal).
    pub lecture: Lecture,
}

/// Ce qu'a coûté une recherche de contenus.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Lecture {
    pub duree_ms: u64,
    /// Fichiers lus sur le disque.
    pub lus: usize,
    /// Fichiers repris du cache, sans les relire.
    pub en_cache: usize,
    /// Le cache n'a pas pu s'écrire (à dire au journal).
    pub erreur_cache: Option<String>,
}

impl Lecture {
    /// « contenus Switch lus en 120 ms, 3 fichier(s) lu(s), 40 en cache ».
    pub fn resume(&self) -> String {
        let mut s = format!("contenus Switch lus en {} ms, {} fichier(s) lu(s), {} en cache", self.duree_ms, self.lus, self.en_cache);
        if let Some(e) = &self.erreur_cache {
            s += &format!(" ; cache non écrit ({e})");
        }
        s
    }
}

/// Le jeu `jeu` (.xci ou .nsp) et, dans SON dossier : sa mise à jour la plus récente et ses DLC — reconnus à
/// l'identifiant lu dans leurs en-têtes, jamais à leur nom. `None` : le fichier n'est pas un jeu lisible avec cette clé.
pub fn contenus_pour(jeu: &Path, cle: &CleEntete) -> Resultat<Option<ContenusDuJeu>> {
    contenus_pour_dans(jeu, cle, &[], None)
}

/// Profondeur des sous-dossiers regardés sous chaque dossier de recherche.
pub const MAX_PROFONDEUR: usize = 4;
/// Au plus ce nombre de dossiers parcourus PAR dossier de recherche (un grand emplacement n'en cache pas un autre).
pub const MAX_DOSSIERS: usize = 2000;
/// Au plus ce nombre de fichiers .nsp/.xci retenus avant toute lecture.
const MAX_CANDIDATS: usize = 20_000;
/// Au-delà de ce temps, plus aucun fichier n'est lu (la suite le sera la fois suivante) : le lancement reste rapide.
pub const BUDGET: std::time::Duration = std::time::Duration::from_millis(1500);
/// Un cache plus gros n'est pas lu (abîmé) : il est refait.
const MAX_OCTETS_CACHE: u64 = 16 << 20;

/// Un fichier à regarder : chemin, taille et date (prises dans l'entrée du dossier, sans autre accès au disque).
struct Candidat {
    chemin: PathBuf,
    taille: u64,
    secondes: u64,
    nanos: u32,
}

/// Les .nsp et .xci à regarder : le dossier du jeu d'abord (et ses sous-dossiers), puis chaque dossier de `ailleurs`
/// (dossier d'installation, emplacements réglés de la plateforme…), chacun avec ses sous-dossiers jusqu'à
/// `MAX_PROFONDEUR`. L'extension est testée sur le nom avant tout accès au fichier ; les liens ne sont pas suivis.
/// Rend aussi `true` si un plafond du parcours a été atteint.
fn candidats(jeu: &Path, ailleurs: &[PathBuf]) -> (Vec<Candidat>, bool) {
    let cle = |p: &Path| p.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_lowercase();
    let mut vus_dossiers = std::collections::HashSet::new();
    let mut vus_fichiers = std::collections::HashSet::new();
    let mut l = Vec::new();
    let mut incomplet = false;
    let racines: Vec<PathBuf> = jeu.parent().map(Path::to_path_buf).into_iter().chain(ailleurs.iter().cloned()).collect();
    'racines: for racine in racines {
        // En largeur : les fichiers proches d'abord. Le plafond de dossiers compte pour CE dossier de recherche.
        let mut parcourus = 0usize;
        let mut file = std::collections::VecDeque::from([(racine, 0usize)]);
        while let Some((d, profondeur)) = file.pop_front() {
            if !vus_dossiers.insert(cle(&d)) {
                continue;
            }
            parcourus += 1;
            if parcourus > MAX_DOSSIERS {
                incomplet = true;
                break;
            }
            let Ok(entrees) = std::fs::read_dir(&d) else { continue };
            let mut entrees: Vec<std::fs::DirEntry> = entrees.flatten().collect();
            entrees.sort_by_key(|e| e.file_name());
            for e in entrees {
                let nom = e.file_name();
                let ext_ok = Path::new(&nom).extension().is_some_and(|x| x.eq_ignore_ascii_case("nsp") || x.eq_ignore_ascii_case("xci"));
                let Ok(t) = e.file_type() else { continue };
                if t.is_dir() {
                    if profondeur < MAX_PROFONDEUR {
                        file.push_back((e.path(), profondeur + 1));
                    } else {
                        incomplet = true;
                    }
                } else if ext_ok && t.is_file() {
                    let p = e.path();
                    if cle(&p) != cle(jeu) && vus_fichiers.insert(cle(&p)) {
                        if l.len() >= MAX_CANDIDATS {
                            incomplet = true;
                            break 'racines;
                        }
                        // Sous Windows, la taille et la date viennent avec l'entrée du dossier (aucune lecture de plus).
                        let Ok(m) = e.metadata() else { continue };
                        let (secondes, nanos) = date(&m);
                        l.push(Candidat { chemin: p, taille: m.len(), secondes, nanos });
                    }
                }
            }
        }
    }
    (l, incomplet)
}

/// Ce qu'on garde d'un conteneur déjà lu (le cache) : sa sorte, son jeu, sa forme. `None` : rien de lisible avec la clé.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct LuEnCache {
    sorte: SorteEnCache,
    base: u64,
    forme: FormeEnCache,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
enum SorteEnCache {
    Jeu,
    MiseAJour,
    Dlc(Vec<(String, u64)>),
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
enum FormeEnCache {
    Nsp,
    Xci,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct EntreeCache {
    taille: u64,
    secondes: u64,
    nanos: u32,
    lu: Option<LuEnCache>,
    /// Illisible (abîmé), inchangé depuis : pas relu, toujours dit.
    #[serde(default)]
    illisible: bool,
}

/// Le cache des conteneurs lus (fichier de Frogtend, dans le dossier de l'émulateur) : par chemin, la taille et la date
/// du fichier ; une entrée ne sert que si les deux sont inchangées, et tout le cache est oublié si la clé change.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
struct Cache {
    /// Une empreinte de la clé (un bloc nul chiffré) : jamais la clé.
    cle: String,
    fichiers: std::collections::BTreeMap<String, EntreeCache>,
}

/// Le cache ne garde jamais plus de ces entrées (les fichiers vus à la dernière recherche d'abord).
const MAX_CACHE: usize = 5000;

fn empreinte(cle: &CleEntete) -> String {
    chiffrer_entete(cle, &[0u8; TAILLE_ENTETE])[..16].iter().map(|b| format!("{b:02x}")).collect()
}

fn date(m: &std::fs::Metadata) -> (u64, u32) {
    m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| (d.as_secs(), d.subsec_nanos())).unwrap_or((0, 0))
}

fn vers_cache(c: &ContenuLu) -> LuEnCache {
    LuEnCache {
        sorte: match &c.sorte {
            Sorte::Jeu => SorteEnCache::Jeu,
            Sorte::MiseAJour => SorteEnCache::MiseAJour,
            Sorte::Dlc(l) => SorteEnCache::Dlc(l.clone()),
        },
        base: c.base,
        forme: match c.forme {
            Forme::Nsp => FormeEnCache::Nsp,
            Forme::Xci => FormeEnCache::Xci,
        },
    }
}

fn depuis_cache(p: &Path, l: &LuEnCache) -> ContenuLu {
    ContenuLu {
        chemin: p.to_path_buf(),
        sorte: match &l.sorte {
            SorteEnCache::Jeu => Sorte::Jeu,
            SorteEnCache::MiseAJour => Sorte::MiseAJour,
            SorteEnCache::Dlc(v) => Sorte::Dlc(v.clone()),
        },
        base: l.base,
        version: p.file_name().and_then(|n| version_du_nom(&n.to_string_lossy())),
        forme: match l.forme {
            FormeEnCache::Nsp => Forme::Nsp,
            FormeEnCache::Xci => Forme::Xci,
        },
    }
}

fn lire_cache(f: &Path) -> Cache {
    if std::fs::metadata(f).map_or(true, |m| m.len() > MAX_OCTETS_CACHE) {
        return Cache::default();
    }
    std::fs::read(f).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default()
}

impl ContenuLu {
    /// Ce qui identifie ce contenu, quel que soit son fichier : une mise à jour par son jeu ET sa version (une autre
    /// version est une autre mise à jour), un DLC par les identifiants de ses NCA. `None` : rien de sûr (une mise à
    /// jour dont le nom ne dit pas la version).
    pub fn identifiant(&self) -> Option<String> {
        match &self.sorte {
            Sorte::Jeu => None,
            Sorte::MiseAJour => self.version.map(|v| format!("maj:{:016X}:v{v}", self.base)),
            Sorte::Dlc(n) => Some(identifiant_dlc(n)),
        }
    }
}

/// L'identifiant d'un DLC d'après ses NCA PublicData (chemin, identifiant) : « dlc:<id>+<id>… », trié.
pub fn identifiant_dlc(ncas: &[(String, u64)]) -> String {
    let mut ids: Vec<u64> = ncas.iter().map(|(_, t)| *t).collect();
    ids.sort_unstable();
    ids.dedup();
    format!("dlc:{}", ids.iter().map(|t| format!("{t:016X}")).collect::<Vec<_>>().join("+"))
}

/// Comme `contenus_pour`, en cherchant aussi dans les sous-dossiers du dossier du jeu et dans `ailleurs` (bornés :
/// `MAX_PROFONDEUR`, `MAX_DOSSIERS` par dossier). Au plus `MAX_VOISINS` conteneurs sont LUS sur le disque à chaque
/// appel, et rien au-delà de `BUDGET` : ceux déjà lus et inchangés viennent du cache `cache` (s'il est donné), si bien
/// que la suite est lue aux appels suivants. Le rattachement se fait TOUJOURS par l'identifiant lu dans les en-têtes,
/// jamais par le nom ; un même contenu trouvé deux fois ne compte qu'une fois (le premier trouvé : le dossier du jeu
/// d'abord), les autres copies sont dites dans `doubles`.
pub fn contenus_pour_dans(jeu: &Path, cle: &CleEntete, ailleurs: &[PathBuf], cache: Option<&Path>) -> Resultat<Option<ContenusDuJeu>> {
    let debut = std::time::Instant::now();
    let Some(lu) = lire_contenu(jeu, cle)? else { return Ok(None) };
    if lu.sorte != Sorte::Jeu {
        return Ok(None);
    }
    let mut r = ContenusDuJeu { base: lu.base, ..Default::default() };
    let (voisins, incomplet) = candidats(jeu, ailleurs);
    r.parcours_incomplet = incomplet;
    let empreinte = empreinte(cle);
    let mut ancien: Cache = cache.map(lire_cache).unwrap_or_default();
    let deja = ancien.clone();
    if ancien.cle != empreinte {
        ancien = Cache { cle: empreinte.clone(), ..Default::default() };
    }
    let mut neuf = Cache { cle: empreinte, ..Default::default() };
    let nom = |p: &Path| p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let mut majs: Vec<ContenuLu> = Vec::new();
    let mut vus: std::collections::HashSet<String> = std::collections::HashSet::new();
    // Les autres jeux, par dossier : seuls ceux des dossiers qui portent des contenus de CE jeu sont à dire.
    let mut autres: Vec<PathBuf> = Vec::new();
    for v in voisins {
        let p = v.chemin;
        let chemin = p.to_string_lossy().to_string();
        let entree = |lu: Option<LuEnCache>, illisible: bool| EntreeCache { taille: v.taille, secondes: v.secondes, nanos: v.nanos, lu, illisible };
        let lu = match ancien.fichiers.get(&chemin).filter(|e| e.taille == v.taille && e.secondes == v.secondes && e.nanos == v.nanos) {
            Some(e) if e.illisible => {
                r.lecture.en_cache += 1;
                r.illisibles.push(nom(&p));
                neuf.fichiers.insert(chemin, e.clone());
                continue;
            }
            Some(e) => {
                r.lecture.en_cache += 1;
                e.lu.as_ref().map(|l| depuis_cache(&p, l))
            }
            None => {
                if r.lecture.lus >= MAX_VOISINS || debut.elapsed() >= BUDGET {
                    r.tronque = true;
                    continue;
                }
                r.lecture.lus += 1;
                match lire_contenu(&p, cle) {
                    Ok(c) => c,
                    Err(_) => {
                        // Illisible : noté (taille et date) pour n'être pas relu tant qu'il ne change pas.
                        r.illisibles.push(nom(&p));
                        neuf.fichiers.insert(chemin, entree(None, true));
                        continue;
                    }
                }
            }
        };
        neuf.fichiers.insert(chemin, entree(lu.as_ref().map(vers_cache), false));
        match lu {
            Some(c) if c.base == r.base && c.sorte != Sorte::Jeu => {
                // Ryubing lit d'après l'extension, casse comprise : seule l'extension exacte de sa forme est posée.
                if p.extension().and_then(|x| x.to_str()) != Some(c.forme.extension()) {
                    r.mauvaise_extension.push(nom(&p));
                    continue;
                }
                // Une copie d'un contenu déjà trouvé (le dossier du jeu passe en premier) : une seule sert.
                if let Some(id) = c.identifiant() {
                    if !vus.insert(id) {
                        r.doubles.push(nom(&p));
                        continue;
                    }
                }
                match c.sorte {
                    Sorte::MiseAJour => majs.push(c),
                    _ => r.dlc.push(c),
                }
            }
            Some(c) if c.base != r.base => autres.extend(p.parent().map(Path::to_path_buf)),
            _ => {}
        }
    }
    if let Some(f) = cache {
        // Les entrées d'avant sont gardées (un autre jeu, un autre emplacement) tant que le cache reste sous son plafond ;
        // celles de cette recherche passent d'abord.
        for (k, v) in ancien.fichiers {
            if neuf.fichiers.len() >= MAX_CACHE {
                break;
            }
            neuf.fichiers.entry(k).or_insert(v);
        }
        while neuf.fichiers.len() > MAX_CACHE {
            neuf.fichiers.pop_last();
        }
        if neuf != deja {
            if let Err(e) = ecrire_cache(f, &neuf) {
                r.lecture.erreur_cache = Some(e.to_string());
            }
        }
    }
    // La plus récente d'abord ; à version égale (ou sans version), l'ordre de la recherche.
    majs.sort_by(|a, b| b.version.cmp(&a.version));
    let mut majs = majs.into_iter();
    r.maj = majs.next();
    r.majs_anciennes = majs.collect();
    let utiles = r.dossiers_utiles();
    r.autres_jeux = autres.iter().filter(|d| utiles.iter().any(|u| u == *d)).count();
    r.lecture.duree_ms = debut.elapsed().as_millis() as u64;
    Ok(Some(r))
}

/// Écrit à côté puis renomme : une coupure ne laisse jamais un cache à moitié écrit.
fn ecrire_cache(f: &Path, c: &Cache) -> std::io::Result<()> {
    let provisoire = f.with_extension("json.tmp");
    std::fs::write(&provisoire, serde_json::to_vec(c).map_err(std::io::Error::other)?)?;
    std::fs::rename(&provisoire, f)
}

impl ContenusDuJeu {
    /// La recherche a-t-elle tout regardé (aucun plafond de lecture ni de parcours atteint) ? Sinon, un DLC peut
    /// encore manquer : « tous les DLC » ne se décide pas.
    pub fn complet(&self) -> bool {
        !self.tronque && !self.parcours_incomplet
    }

    /// Les dossiers où se trouvent la mise à jour retenue et les DLC de ce jeu (dans l'ordre, sans doublon).
    pub fn dossiers_utiles(&self) -> Vec<PathBuf> {
        let mut l: Vec<PathBuf> = Vec::new();
        for c in self.maj.iter().chain(self.dlc.iter()) {
            if let Some(d) = c.chemin.parent() {
                if !l.iter().any(|x| x == d) {
                    l.push(d.to_path_buf());
                }
            }
        }
        l
    }
}

/// Des noms de fichiers à dire : au plus 5, puis « et N autre(s) ».
pub fn quelques_noms(l: &[String]) -> String {
    let mut s = l.iter().take(5).cloned().collect::<Vec<_>>().join(", ");
    if l.len() > 5 {
        s += &format!(" et {} autre(s)", l.len() - 5);
    }
    s
}

/// L'identifiant de base d'un jeu lu dans ses en-têtes (`None` : pas de clé lisible, pas un jeu).
pub fn base_du_jeu(jeu: &Path, cle: &CleEntete) -> Option<u64> {
    lire_contenu(jeu, cle).ok().flatten().filter(|c| c.sorte == Sorte::Jeu).map(|c| c.base)
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// Une clé de TEST (jamais une vraie).
    pub fn cle_de_test() -> CleEntete {
        let mut o = [0u8; 32];
        for (i, b) in o.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(7).wrapping_add(3);
        }
        CleEntete::depuis(&o)
    }

    /// Un NCA de test chiffré : en-tête NCA3 de `type_contenu` et `title_id`, suivi de `reste` octets.
    pub fn nca(cle: &CleEntete, type_contenu: u8, title_id: u64, reste: usize) -> Vec<u8> {
        let mut h = vec![0u8; TAILLE_ENTETE];
        for (i, b) in h.iter_mut().enumerate() {
            *b = (i % 251) as u8; // du « bruit » : jamais une magie par hasard
        }
        h[0x200..0x204].copy_from_slice(b"NCA3");
        h[0x205] = type_contenu;
        h[0x210..0x218].copy_from_slice(&title_id.to_le_bytes());
        let mut c = chiffrer_entete(cle, &h);
        c.extend(std::iter::repeat_n(0xAB, reste));
        c
    }

    /// Une partition PFS0 (`hfs0` : HFS0) des fichiers donnés.
    pub fn partition(fichiers: &[(&str, Vec<u8>)], hfs0: bool) -> Vec<u8> {
        let taille_entree = if hfs0 { 0x40 } else { 0x18 };
        let mut noms = Vec::new();
        let mut places = Vec::new();
        for (n, _) in fichiers {
            places.push(noms.len() as u32);
            noms.extend(n.as_bytes());
            noms.push(0);
        }
        while noms.len() % 4 != 0 {
            noms.push(0);
        }
        let mut o = if hfs0 { b"HFS0".to_vec() } else { b"PFS0".to_vec() };
        o.extend((fichiers.len() as u32).to_le_bytes());
        o.extend((noms.len() as u32).to_le_bytes());
        o.extend(0u32.to_le_bytes());
        let mut debut = 0u64;
        for (i, (_, d)) in fichiers.iter().enumerate() {
            let mut e = vec![0u8; taille_entree];
            e[0..8].copy_from_slice(&debut.to_le_bytes());
            e[8..16].copy_from_slice(&(d.len() as u64).to_le_bytes());
            e[16..20].copy_from_slice(&places[i].to_le_bytes());
            o.extend(e);
            debut += d.len() as u64;
        }
        o.extend(noms);
        for (_, d) in fichiers {
            o.extend(d);
        }
        o
    }

    /// Un .xci de test : `HEAD` en 0x100 (après `prefixe` octets de zone des clés), racine HFS0 avec `normal` et `secure`.
    pub fn xci(fichiers: &[(&str, Vec<u8>)], prefixe: usize) -> Vec<u8> {
        let secure = partition(fichiers, true);
        let racine = partition(&[("normal", partition(&[], true)), ("secure", secure)], true);
        let mut o = vec![0u8; prefixe + 0x200];
        o[prefixe + 0x100..prefixe + 0x104].copy_from_slice(b"HEAD");
        o[prefixe + 0x130..prefixe + 0x138].copy_from_slice(&0x200u64.to_le_bytes());
        o[prefixe + 0x138..prefixe + 0x140].copy_from_slice(&(racine.len() as u64).to_le_bytes());
        o.extend(racine);
        o
    }

    pub const BASE: u64 = 0x0100_7EF0_0011_E000;

    #[test]
    fn la_cle_se_lit_dans_prod_keys_et_ne_s_affiche_jamais() {
        let o: String = (0..32u8).map(|i| format!("{:02x}", i.wrapping_mul(7).wrapping_add(3))).collect();
        let texte = format!("aes_kek_generation_source = 00\nheader_key = {o}\nkey_area_key_application_00 = 11\n");
        let cle = cle_entete_du_texte(&texte).unwrap();
        assert_eq!(format!("{cle:?}"), "CleEntete(…)", "jamais la clé dans un journal");
        // C'est bien la même clé que celle de test : elle relit un en-tête qu'elle a chiffré.
        let n = nca(&cle_de_test(), 5, BASE + 0x1001, 0);
        assert_eq!(dechiffrer_entete(&cle, &n).map(|e| e.title_id), Some(BASE + 0x1001));
        assert!(cle_entete_du_texte("header_key = 1234\n").is_none(), "une clé trop courte n'est pas une clé");
        assert!(cle_entete_du_texte("title_key = 00\n").is_none());
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("prod.keys"), texte).unwrap();
        assert!(cle_entete(&d.path().join("prod.keys")).is_some());
        assert!(cle_entete(&d.path().join("absent.keys")).is_none());
    }

    #[test]
    fn le_tweak_de_nintendo_porte_le_numero_de_secteur_en_grand_boutiste() {
        // Vecteur calculé à la main : secteur 1, tweak = 00…01 (le dernier octet), pas 01 00… (XTS habituel).
        let cle = cle_de_test();
        let mut a = [0u8; 0x200];
        let mut b = [0u8; 0x200];
        xts_secteur(&cle, &mut a, 1, true);
        // Le même secteur chiffré avec le tweak petit-boutiste donnerait autre chose.
        let mut t = [0u8; 16];
        t[0] = 1;
        cle.tweak.encrypt_block(GenericArray::from_mut_slice(&mut t));
        for bloc in b.chunks_exact_mut(16) {
            for (x, y) in bloc.iter_mut().zip(t.iter()) {
                *x ^= y;
            }
            cle.donnees.encrypt_block(GenericArray::from_mut_slice(bloc));
            for (x, y) in bloc.iter_mut().zip(t.iter()) {
                *x ^= y;
            }
            fois_alpha(&mut t);
        }
        assert_ne!(a, b);
        // Et l'aller-retour rend le clair.
        xts_secteur(&cle, &mut a, 1, false);
        assert_eq!(a, [0u8; 0x200]);
        // α : la retenue du dernier octet revient en 0x87 sur le premier.
        let mut t = [0u8; 16];
        t[15] = 0x80;
        fois_alpha(&mut t);
        assert_eq!((t[0], t[15]), (0x87, 0));
    }

    #[test]
    fn un_entete_dit_son_type_et_son_titre_et_une_autre_cle_ne_lit_rien() {
        let cle = cle_de_test();
        for (octet, attendu) in [(0, TypeContenu::Programme), (1, TypeContenu::Meta), (2, TypeContenu::Controle), (5, TypeContenu::DonneesPubliques)] {
            let n = nca(&cle, octet, BASE, 0);
            assert_eq!(dechiffrer_entete(&cle, &n), Some(EnteteNca { type_contenu: attendu, title_id: BASE }));
        }
        let autre = CleEntete::depuis(&[9u8; 32]);
        assert_eq!(dechiffrer_entete(&autre, &nca(&cle, 0, BASE, 0)), None);
        assert_eq!(dechiffrer_entete(&cle, &[0u8; 0x100]), None, "trop court");
        assert_eq!(dechiffrer_entete(&cle, &nca(&cle, 6, BASE, 0)), None, "type inconnu");
    }

    #[test]
    fn un_nsp_et_un_xci_donnent_leurs_nca_avec_le_chemin_de_ryubing() {
        let cle = cle_de_test();
        let d = tempfile::tempdir().unwrap();
        let nsp = d.path().join("dlc.nsp");
        std::fs::write(
            &nsp,
            partition(&[("aa11.cnmt.nca", nca(&cle, 1, BASE + 0x1001, 16)), ("bb22.nca", nca(&cle, 5, BASE + 0x1001, 100)), ("x.tik", vec![1; 0x2C0])], false),
        )
        .unwrap();
        let (l, illisibles) = ncas(&nsp, &cle).unwrap();
        assert_eq!(illisibles, 0);
        assert_eq!(l.iter().map(|n| n.chemin.as_str()).collect::<Vec<_>>(), ["/aa11.cnmt.nca", "/bb22.nca"]);
        for prefixe in [0, 0x1000] {
            let x = d.path().join(format!("jeu{prefixe}.xci"));
            std::fs::write(&x, xci(&[("cc33.nca", nca(&cle, 0, BASE, 64)), ("dd44.nca", nca(&cle, 2, BASE, 8))], prefixe)).unwrap();
            let (l, _) = ncas(&x, &cle).unwrap();
            assert_eq!(l.len(), 2, "zone des clés de {prefixe} octets");
            assert_eq!(l[0], NcaLu { chemin: "/cc33.nca".into(), entete: EnteteNca { type_contenu: TypeContenu::Programme, title_id: BASE } });
        }
        // Une entrée qui sort du fichier : refusé, jamais lu au hasard.
        let mut abime = partition(&[("ee.nca", nca(&cle, 0, BASE, 0))], false);
        abime.truncate(abime.len() - 10);
        std::fs::write(d.path().join("abime.nsp"), abime).unwrap();
        assert!(entrees(&d.path().join("abime.nsp")).is_err());
        std::fs::write(d.path().join("rien.nsp"), b"pas un conteneur du tout").unwrap();
        assert!(entrees(&d.path().join("rien.nsp")).is_err());
    }

    #[test]
    fn un_gros_conteneur_creux_se_lit_sans_etre_charge() {
        // Un « .xci » de 16 Go creux : seules ses tables et ses en-têtes sont lus.
        let cle = cle_de_test();
        let d = tempfile::tempdir().unwrap();
        let x = d.path().join("gros.xci");
        std::fs::write(&x, xci(&[("cc33.nca", nca(&cle, 0, BASE, 0)), ("m.cnmt.nca", nca(&cle, 1, BASE, 0))], 0)).unwrap();
        let f = std::fs::OpenOptions::new().write(true).open(&x).unwrap();
        f.set_len(16 << 30).unwrap();
        drop(f);
        assert_eq!(base_du_jeu(&x, &cle), Some(BASE));
    }

    #[test]
    fn la_version_se_lit_dans_le_nom() {
        assert_eq!(version_du_nom("The Legend of Zelda Breath of the Wild [01007EF00011E800][v786432](nsw2u).nsp"), Some(786432));
        assert_eq!(version_du_nom("T.L.O.Z.BOTW__DLC_Pack_2_The_Champions_Ballad___switch-xci.com__v196608_.nsp"), Some(196608));
        assert_eq!(version_du_nom("Dragon_Quest_Builders_2 v65536.nsp"), Some(65536));
        assert_eq!(version_du_nom("The Legend of Zelda Breath of the Wild (v0).xci"), Some(0));
        assert_eq!(version_du_nom("Jeu Deluxe v2b.nsp"), None);
        // Plusieurs : la plus grande, où qu'elle soit.
        assert_eq!(version_du_nom("Zelda [v786432] v3.nsp"), Some(786432));
        assert_eq!(version_du_nom("Zelda v3 [v786432].nsp"), Some(786432));
        // Jamais montré tel quel.
        assert_eq!(
            nom_a_montrer(Path::new("T.L.O.Z.BOTW__DLC_Pack_2_The_Champions_Ballad___switch-xci.com__v196608_.nsp")),
            "T.L.O.Z.BOTW DLC Pack 2 The Champions Ballad switch-xci.com"
        );
        assert_eq!(nom_a_montrer(Path::new("Dragon_Quest_Builders_2 v65536.nsp")), "Dragon Quest Builders 2");
        assert_eq!(version_du_nom("Overcooked 2 [0100ABCD].nsp"), None);
    }

    /// Le dossier de Zelda fabriqué dans la forme RELEVÉE sur les vrais fichiers (16/11) : le .xci porte le Meta du jeu
    /// ET celui d'une mise à jour ; une mise à jour .nsp a son Meta à +0x800 mais son Program à l'identifiant du jeu.
    /// Deux mises à jour, deux DLC, un autre jeu nommé comme Zelda, un fichier abîmé, le DLC d'un autre jeu dans un
    /// sous-dossier.
    pub fn dossier_de_zelda(d: &Path) -> PathBuf {
        let cle = cle_de_test();
        let jeu = d.join("Zelda (v0).xci");
        let fichiers = [("p.nca", nca(&cle, 0, BASE, 0)), ("c.nca", nca(&cle, 2, BASE, 0)), ("m.cnmt.nca", nca(&cle, 1, BASE, 0)), ("n.cnmt.nca", nca(&cle, 1, BASE + 0x800, 0))];
        std::fs::write(&jeu, xci(&fichiers, 0)).unwrap();
        let maj = |v: u32, nom: &str| {
            std::fs::write(d.join(format!("{nom}[v{v}].nsp")), partition(&[("m.cnmt.nca", nca(&cle, 1, BASE + 0x800, 0)), ("p.nca", nca(&cle, 0, BASE, 0))], false)).unwrap();
        };
        maj(786432, "Zelda maj ");
        maj(65536, "Zelda vieille maj ");
        std::fs::write(d.join("DLC_Pack_1.nsp"), partition(&[("a1.nca", nca(&cle, 5, BASE + 0x1001, 0)), ("m.cnmt.nca", nca(&cle, 1, BASE + 0x1001, 0))], false)).unwrap();
        std::fs::write(d.join("DLC_Pack_2.nsp"), partition(&[("b2.nca", nca(&cle, 5, BASE + 0x1002, 0))], false)).unwrap();
        // Un AUTRE jeu dans le même dossier, nommé comme Zelda : jamais pris.
        let autre = 0x0100_AAAA_0000_0000u64;
        std::fs::write(d.join("Zelda DLC Pack 3.nsp"), partition(&[("z.nca", nca(&cle, 5, autre + 0x1001, 0))], false)).unwrap();
        std::fs::write(d.join("Zelda maj [v999999].nsp"), partition(&[("z.cnmt.nca", nca(&cle, 1, autre + 0x800, 0)), ("z.nca", nca(&cle, 0, autre, 0))], false)).unwrap();
        std::fs::write(d.join("abime.nsp"), b"PFS0abime").unwrap();
        std::fs::create_dir(d.join("sous")).unwrap();
        // Dans un sous-dossier (regardé depuis le 16/11), le DLC d'un AUTRE jeu : jamais pris.
        std::fs::write(d.join("sous").join("DLC_Pack_9.nsp"), partition(&[("c9.nca", nca(&cle, 5, autre + 0x1009, 0))], false)).unwrap();
        jeu
    }

    #[test]
    fn le_jeu_prend_sa_maj_la_plus_recente_et_ses_dlc_par_l_identifiant_lu() {
        let d = tempfile::tempdir().unwrap();
        let jeu = dossier_de_zelda(d.path());
        let r = contenus_pour(&jeu, &cle_de_test()).unwrap().unwrap();
        assert_eq!(r.base, BASE);
        assert_eq!(r.maj.as_ref().map(|m| (m.version, m.sorte.clone())), Some((Some(786432), Sorte::MiseAJour)));
        assert_eq!(r.majs_anciennes.len(), 1);
        let dlc: Vec<(String, Sorte)> = r.dlc.iter().map(|c| (c.chemin.file_name().unwrap().to_string_lossy().to_string(), c.sorte.clone())).collect();
        assert_eq!(
            dlc,
            [
                ("DLC_Pack_1.nsp".to_string(), Sorte::Dlc(vec![("/a1.nca".into(), BASE + 0x1001)])),
                ("DLC_Pack_2.nsp".to_string(), Sorte::Dlc(vec![("/b2.nca".into(), BASE + 0x1002)]))
            ],
            "jamais le DLC d'un autre jeu (même nom, ou dans un sous-dossier)"
        );
        assert_eq!(r.dossiers_utiles(), [d.path().to_path_buf()]);
        assert_eq!(r.illisibles, ["abime.nsp"]);
        assert_eq!(r.autres_jeux, 2, "le DLC et la mise à jour de l'autre jeu sont comptés (Eden lira tout le dossier)");
        // Une mise à jour donnée comme jeu : pas un jeu.
        assert_eq!(contenus_pour(&d.path().join("Zelda maj [v786432].nsp"), &cle_de_test()).unwrap(), None);
        // Sans la bonne clé : rien de deviné.
        assert_eq!(contenus_pour(&jeu, &CleEntete::depuis(&[1u8; 32])).unwrap(), None);
    }

    #[test]
    fn un_jeu_avec_sa_maj_sur_la_cartouche_reste_un_jeu() {
        let cle = cle_de_test();
        let d = tempfile::tempdir().unwrap();
        let x = d.path().join("j.xci");
        // Relevé réel (Zelda v0) : Meta du jeu et Meta d'une mise à jour sur la même cartouche, Program au jeu.
        std::fs::write(&x, xci(&[("p.nca", nca(&cle, 0, BASE, 0)), ("m.cnmt.nca", nca(&cle, 1, BASE, 0)), ("n.cnmt.nca", nca(&cle, 1, BASE + 0x800, 0))], 0)).unwrap();
        assert_eq!(lire_contenu(&x, &cle).unwrap().map(|c| (c.sorte, c.base)), Some((Sorte::Jeu, BASE)));
        // Une mise à jour : son Program porte l'identifiant du JEU, seul son Meta dit +0x800.
        let maj = d.path().join("maj.nsp");
        std::fs::write(&maj, partition(&[("m.cnmt.nca", nca(&cle, 1, BASE + 0x800, 0)), ("p.nca", nca(&cle, 0, BASE, 0))], false)).unwrap();
        assert_eq!(lire_contenu(&maj, &cle).unwrap().map(|c| (c.sorte, c.base)), Some((Sorte::MiseAJour, BASE)));
        // Sans Meta lisible : rien n'est deviné (un Program seul peut être un jeu comme une mise à jour).
        std::fs::write(&maj, partition(&[("p.nca", nca(&cle, 0, BASE, 0))], false)).unwrap();
        assert_eq!(lire_contenu(&maj, &cle).unwrap(), None);
        // Deux jeux différents sur la même cartouche : ambigu, rien n'est deviné.
        std::fs::write(&x, xci(&[("m.cnmt.nca", nca(&cle, 1, BASE, 0)), ("q.cnmt.nca", nca(&cle, 1, 0x0100_AAAA_0000_0000, 0))], 0)).unwrap();
        assert_eq!(lire_contenu(&x, &cle).unwrap(), None);
        // Un .xci GROUPÉ (jeu + mise à jour + DLC) : un jeu, jamais pris pour un DLC.
        let groupe = [
            ("p.nca", nca(&cle, 0, BASE, 0)),
            ("m.cnmt.nca", nca(&cle, 1, BASE, 0)),
            ("n.cnmt.nca", nca(&cle, 1, BASE + 0x800, 0)),
            ("d.nca", nca(&cle, 5, BASE + 0x1001, 0)),
            ("o.cnmt.nca", nca(&cle, 1, BASE + 0x1001, 0)),
        ];
        std::fs::write(&x, xci(&groupe, 0)).unwrap();
        assert_eq!(lire_contenu(&x, &cle).unwrap().map(|c| (c.sorte, c.forme)), Some((Sorte::Jeu, Forme::Xci)));
        // Une mise à jour groupée avec un DLC (sans Meta du jeu) : la mise à jour.
        std::fs::write(&maj, partition(&[("m.cnmt.nca", nca(&cle, 1, BASE + 0x800, 0)), ("d.nca", nca(&cle, 5, BASE + 0x1001, 0))], false)).unwrap();
        assert_eq!(lire_contenu(&maj, &cle).unwrap().map(|c| c.sorte), Some(Sorte::MiseAJour));
    }

    #[test]
    fn seule_l_extension_exacte_de_sa_forme_est_posee() {
        // Ryubing choisit la lecture d'après l'extension, casse comprise.
        let cle = cle_de_test();
        let d = tempfile::tempdir().unwrap();
        let jeu = d.path().join("jeu.xci");
        std::fs::write(&jeu, xci(&[("m.cnmt.nca", nca(&cle, 1, BASE, 0))], 0)).unwrap();
        let dlc = |n: u64| partition(&[(&format!("{n}.nca") as &str, nca(&cle, 5, BASE + 0x1000 + n, 0))], false);
        std::fs::write(d.path().join("bon.nsp"), dlc(1)).unwrap();
        std::fs::write(d.path().join("MAJUSCULES.NSP"), dlc(2)).unwrap();
        std::fs::write(d.path().join("un pfs0.xci"), dlc(3)).unwrap();
        let r = contenus_pour(&jeu, &cle).unwrap().unwrap();
        assert_eq!(r.dlc.len(), 1);
        assert_eq!(r.mauvaise_extension, ["MAJUSCULES.NSP", "un pfs0.xci"]);
        assert!(!r.tronque);
    }

    #[test]
    fn un_dossier_trop_grand_est_lu_en_partie_et_le_dit() {
        let cle = cle_de_test();
        let d = tempfile::tempdir().unwrap();
        let jeu = d.path().join("jeu.xci");
        std::fs::write(&jeu, xci(&[("m.cnmt.nca", nca(&cle, 1, BASE, 0))], 0)).unwrap();
        for i in 0..=MAX_VOISINS {
            std::fs::write(d.path().join(format!("{i:04}.nsp")), b"x").unwrap();
        }
        let r = contenus_pour(&jeu, &cle).unwrap().unwrap();
        assert!(r.tronque);
        assert_eq!(r.illisibles.len(), MAX_VOISINS);
    }

    #[test]
    fn un_emplacement_separe_est_regarde_par_l_identifiant_avec_un_cache() {
        // La forme de Seb (A-TESTER 0.41.0) : le jeu dans son dossier, ses mises à jour et DLC dans un emplacement à
        // part de la plateforme, mêlés à ceux d'autres jeux, nommés n'importe comment.
        let cle = cle_de_test();
        let d = tempfile::tempdir().unwrap();
        let jeux = d.path().join("Jeux").join("Fire Emblem");
        let maj_dlc = d.path().join("Nintendo Switch Maj & DLC");
        std::fs::create_dir_all(&jeux).unwrap();
        std::fs::create_dir_all(maj_dlc.join("Fire Emblem")).unwrap();
        let jeu = jeux.join("FE3H.xci");
        std::fs::write(&jeu, xci(&[("m.cnmt.nca", nca(&cle, 1, BASE, 0))], 0)).unwrap();
        std::fs::write(maj_dlc.join("sans rapport [v65536].nsp"), partition(&[("m.cnmt.nca", nca(&cle, 1, BASE + 0x800, 0))], false)).unwrap();
        std::fs::write(maj_dlc.join("Fire Emblem").join("Cindered Shadows.nsp"), partition(&[("a.nca", nca(&cle, 5, BASE + 0x1001, 0))], false)).unwrap();
        let autre = 0x0100_AAAA_0000_0000u64;
        std::fs::write(maj_dlc.join("Fire Emblem Warriors DLC.nsp"), partition(&[("z.nca", nca(&cle, 5, autre + 0x1001, 0))], false)).unwrap();
        // Sans l'emplacement : rien.
        let r = contenus_pour(&jeu, &cle).unwrap().unwrap();
        assert!(r.maj.is_none() && r.dlc.is_empty());
        // Avec : trouvés par leurs en-têtes, chemins absolus ; l'autre jeu est compté dans le dossier qui sera lu.
        let cache = d.path().join("cache.json");
        let ailleurs = [maj_dlc.clone(), jeux.clone()];
        let r = contenus_pour_dans(&jeu, &cle, &ailleurs, Some(&cache)).unwrap().unwrap();
        assert_eq!(r.maj.as_ref().map(|m| m.chemin.clone()), Some(maj_dlc.join("sans rapport [v65536].nsp")));
        assert_eq!(r.dlc.iter().map(|c| c.chemin.clone()).collect::<Vec<_>>(), [maj_dlc.join("Fire Emblem").join("Cindered Shadows.nsp")]);
        assert_eq!(r.dossiers_utiles(), [maj_dlc.clone(), maj_dlc.join("Fire Emblem")]);
        assert_eq!(r.autres_jeux, 1);
        assert!(!r.tronque);
        // Le cache : la clé n'y est jamais ; relu, il rend la même chose.
        let texte = std::fs::read_to_string(&cache).unwrap();
        assert!(texte.contains("Cindered Shadows") && !texte.contains("header_key"));
        let r2 = contenus_pour_dans(&jeu, &cle, &ailleurs, Some(&cache)).unwrap().unwrap();
        assert_eq!((r2.lecture.lus, r2.lecture.en_cache), (0, 3), "tout vient du cache");
        assert_eq!((r2.maj, r2.dlc, r2.autres_jeux), (r.maj.clone(), r.dlc.clone(), r.autres_jeux));
        // Un fichier changé (autre taille) est relu.
        std::fs::write(maj_dlc.join("Fire Emblem").join("Cindered Shadows.nsp"), b"abime").unwrap();
        let r3 = contenus_pour_dans(&jeu, &cle, &ailleurs, Some(&cache)).unwrap().unwrap();
        assert!(r3.dlc.is_empty() && r3.illisibles == ["Cindered Shadows.nsp"]);
        // Une autre clé : le cache entier est oublié (rien de lu avec elle).
        assert_eq!(contenus_pour_dans(&jeu, &CleEntete::depuis(&[1u8; 32]), &ailleurs, Some(&cache)).unwrap(), None);
    }

    #[test]
    fn un_contenu_en_double_ne_compte_qu_une_fois_celui_du_dossier_du_jeu_d_abord() {
        let cle = cle_de_test();
        let d = tempfile::tempdir().unwrap();
        let (jeux, place) = (d.path().join("jeu"), d.path().join("place"));
        std::fs::create_dir_all(&jeux).unwrap();
        std::fs::create_dir_all(&place).unwrap();
        let jeu = jeux.join("jeu.xci");
        std::fs::write(&jeu, xci(&[("m.cnmt.nca", nca(&cle, 1, BASE, 0))], 0)).unwrap();
        let dlc = partition(&[("a.nca", nca(&cle, 5, BASE + 0x1001, 0))], false);
        let maj = partition(&[("m.cnmt.nca", nca(&cle, 1, BASE + 0x800, 0))], false);
        // L'emplacement est trié avant le dossier du jeu par son nom : le dossier du jeu passe quand même d'abord.
        std::fs::write(place.join("a DLC.nsp"), &dlc).unwrap();
        std::fs::write(place.join("a maj [v65536].nsp"), &maj).unwrap();
        std::fs::write(jeux.join("z DLC.nsp"), &dlc).unwrap();
        std::fs::write(jeux.join("z maj [v65536].nsp"), &maj).unwrap();
        // Une mise à jour plus récente, elle, n'est pas un double.
        std::fs::write(place.join("b maj [v131072].nsp"), &maj).unwrap();
        let cache = d.path().join("cache.json");
        let r = contenus_pour_dans(&jeu, &cle, &[place.clone()], Some(&cache)).unwrap().unwrap();
        assert_eq!(r.dlc.iter().map(|c| c.chemin.clone()).collect::<Vec<_>>(), [jeux.join("z DLC.nsp")]);
        assert_eq!(r.maj.as_ref().map(|m| m.chemin.clone()), Some(place.join("b maj [v131072].nsp")));
        assert_eq!(r.majs_anciennes.iter().map(|c| c.chemin.clone()).collect::<Vec<_>>(), [jeux.join("z maj [v65536].nsp")]);
        assert_eq!(r.doubles, ["a DLC.nsp", "a maj [v65536].nsp"]);
        // Un fichier illisible n'est pas relu tant qu'il ne change pas ; toujours dit.
        std::fs::write(place.join("abime.nsp"), b"PFS0abime").unwrap();
        let r = contenus_pour_dans(&jeu, &cle, &[place.clone()], Some(&cache)).unwrap().unwrap();
        assert_eq!((r.lecture.lus, r.illisibles.clone()), (1, vec!["abime.nsp".to_string()]));
        let r = contenus_pour_dans(&jeu, &cle, &[place.clone()], Some(&cache)).unwrap().unwrap();
        assert_eq!((r.lecture.lus, r.illisibles.clone()), (0, vec!["abime.nsp".to_string()]));
        assert!(r.lecture.resume().contains("0 fichier(s) lu(s)"));
        assert_eq!(quelques_noms(&(0..7).map(|i| i.to_string()).collect::<Vec<_>>()), "0, 1, 2, 3, 4 et 2 autre(s)");
    }

    #[test]
    fn un_parcours_trop_profond_est_dit_a_part() {
        let cle = cle_de_test();
        let d = tempfile::tempdir().unwrap();
        let jeu = d.path().join("jeu.xci");
        std::fs::write(&jeu, xci(&[("m.cnmt.nca", nca(&cle, 1, BASE, 0))], 0)).unwrap();
        let mut profond = d.path().join("place");
        for i in 0..=MAX_PROFONDEUR {
            profond = profond.join(format!("n{i}"));
        }
        std::fs::create_dir_all(&profond).unwrap();
        std::fs::write(profond.join("dlc.nsp"), partition(&[("a.nca", nca(&cle, 5, BASE + 0x1001, 0))], false)).unwrap();
        let r = contenus_pour_dans(&jeu, &cle, &[d.path().join("place")], None).unwrap().unwrap();
        assert!(r.parcours_incomplet && !r.tronque && r.dlc.is_empty());
    }

    #[test]
    fn un_grand_emplacement_est_lu_par_tranches_grace_au_cache() {
        let cle = cle_de_test();
        let d = tempfile::tempdir().unwrap();
        let jeux = d.path().join("jeu");
        let place = d.path().join("place");
        std::fs::create_dir_all(&jeux).unwrap();
        std::fs::create_dir_all(&place).unwrap();
        let jeu = jeux.join("jeu.xci");
        std::fs::write(&jeu, xci(&[("m.cnmt.nca", nca(&cle, 1, BASE, 0))], 0)).unwrap();
        let autre = 0x0100_AAAA_0000_0000u64;
        for i in 0..MAX_VOISINS {
            std::fs::write(place.join(format!("a{i:04}.nsp")), partition(&[("z.nca", nca(&cle, 5, autre + 0x1001, 0))], false)).unwrap();
        }
        // Le DLC du jeu, rangé APRÈS les 400 premiers : pas lu la première fois (dit), lu la suivante.
        std::fs::write(place.join("z.nsp"), partition(&[("a.nca", nca(&cle, 5, BASE + 0x1001, 0))], false)).unwrap();
        let cache = d.path().join("cache.json");
        let r = contenus_pour_dans(&jeu, &cle, &[place.clone()], Some(&cache)).unwrap().unwrap();
        assert!(r.tronque && r.dlc.is_empty());
        let r = contenus_pour_dans(&jeu, &cle, &[place.clone()], Some(&cache)).unwrap().unwrap();
        assert!(!r.tronque && r.dlc.len() == 1, "{r:?}");
    }

    #[test]
    fn une_table_qui_sort_du_fichier_est_refusee_sans_debordement() {
        let cle = cle_de_test();
        let d = tempfile::tempdir().unwrap();
        // La racine d'un .xci placée au-delà du fichier (ou à u64::MAX : jamais un débordement).
        for adresse in [0x10_0000u64, u64::MAX - 0x10] {
            let mut x = xci(&[("p.nca", nca(&cle, 0, BASE, 0))], 0);
            x[0x130..0x138].copy_from_slice(&adresse.to_le_bytes());
            std::fs::write(d.path().join("x.xci"), &x).unwrap();
            assert!(entrees(&d.path().join("x.xci")).is_err());
        }
        // Une table annoncée plus grande que le fichier.
        let mut p = partition(&[("a.nca", nca(&cle, 0, BASE, 0))], false);
        p[8..12].copy_from_slice(&0x8_0000u32.to_le_bytes());
        std::fs::write(d.path().join("p.nsp"), &p).unwrap();
        assert!(entrees(&d.path().join("p.nsp")).is_err());
    }
}

/// Sur les vrais fichiers de Zelda de Seb, en LECTURE SEULE (jamais une clé affichée).
#[cfg(test)]
mod essais_reels {
    use super::*;

    #[test]
    #[ignore]
    fn essai_zelda_reel() {
        let cle = cle_entete(Path::new(r"E:\Frogtend\Emulateurs\Ryubing\Profils\Sebastien\system\prod.keys")).expect("clé des en-têtes absente");
        let dossier = Path::new(r"D:\Jeux\Switch\The Legend of Zelda Breath of the Wild (2017)");
        let mut l: Vec<PathBuf> = std::fs::read_dir(dossier).unwrap().flatten().map(|e| e.path()).filter(|p| p.is_file()).collect();
        l.sort();
        for p in &l {
            let debut = std::time::Instant::now();
            println!("== {}", p.file_name().unwrap().to_string_lossy());
            match ncas(p, &cle) {
                Ok((n, illisibles)) => {
                    for x in &n {
                        println!("   {} | {:?} | {:016x}", x.chemin, x.entete.type_contenu, x.entete.title_id);
                    }
                    println!("   {} NCA lus, {} illisibles, {:?}", n.len(), illisibles, debut.elapsed());
                }
                Err(e) => println!("   pas lu : {e:?}"),
            }
            println!("   -> {:?}", lire_contenu(p, &cle).map(|c| c.map(|c| (c.sorte, format!("{:016x}", c.base), c.version))));
        }
        let jeu = dossier.join("The Legend of Zelda Breath of the Wild (v0).xci");
        let r = contenus_pour(&jeu, &cle).unwrap().expect("le .xci de Zelda est un jeu");
        println!("contenus_pour : {r:#?}");
        // Le relevé du 16/11, devenu attendu.
        assert_eq!(r.base, 0x0100_7EF0_0011_E000);
        assert!(r.maj.as_ref().is_some_and(|m| m.version == Some(786432)));
        let dlc: Vec<u64> = r.dlc.iter().flat_map(|c| match &c.sorte {
            Sorte::Dlc(n) => n.iter().map(|x| x.1).collect(),
            _ => vec![],
        }).collect();
        assert_eq!(dlc, [0x0100_7EF0_0011_F001, 0x0100_7EF0_0011_F002]);
    }
}
