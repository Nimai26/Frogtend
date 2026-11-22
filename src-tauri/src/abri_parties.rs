//! Mettre À LA MAIN les parties d'un jeu PC à l'abri (lot S2, 22/11), aux emplacements de « 📁 Où sont mes parties »
//! (lot S1), avec une FICHE par abri. Rien de fait à la main n'est envoyé chez Firehouse ici (lot S4), rien n'est remis
//! (lot S5).
//!
//! - L'abri : `<abris>/<jeu>/<date>/emplacements/<clé>/…`. La clé vient du chemin NON résolu (`cle_chemin`), jamais du
//!   chemin de ce PC (qui contient le nom de l'utilisateur) ; quand il a fallu énumérer un `<identifiant …>`, une
//!   EMPREINTE du sous-dossier pris s'y ajoute (`<clé>-<empreinte>`), mêlée au sel secret du profil (`sel_du_profil`) :
//!   on ne retrouve jamais son nom (un compte Steam, un profil du jeu).
//! - La fiche, À CÔTÉ (jamais dedans : elle partirait comme une partie) : `<abris>/<jeu>/<date>.json` (`FicheAbri`) —
//!   profil propriétaire (celui de la session), emplacements (chemins NON résolus, nature, filtre), fichiers (taille,
//!   SHA1), version de la fiche. Elle est écrite AVANT que l'abri prenne son nom (une fiche seule n'est jamais lue), et
//!   seulement quand chacun de ses fichiers est bien dans l'abri.
//! - Tout ou rien : tout est copié dans un dossier provisoire (à son nom propre), chaque copie vérifiée au SHA1
//!   (recopiée une fois si elle a changé), puis le dossier est renommé. Un emplacement en échec (fichier verrouillé,
//!   trop gros, illisible) est retiré de l'abri et dit ; les autres passent. Rien de neuf depuis le dernier abri du même
//!   profil : pas de copie.
//! - Un fichier OneDrive « en ligne seulement » n'est jamais lu (il serait téléchargé) : sauté et compté (et pas
//!   compté dans le plafond de 2 Go).
//! - `emplacements/` et `nuage-steam/` ne sont « réservés » (jamais posés dans le dossier du jeu, envoyés à part) que
//!   dans un abri qui le dit (`reservees`) : un jeu dont le dossier a un sous-dossier `emplacements` reste un jeu.
//! - Les abris ne s'effacent jamais ; un ancien abri (sans fiche) se lit tel quel, il n'est jamais réécrit.
//! - Jamais un chemin au journal (l'appelant n'écrit que le numéro du jeu et des nombres).

use crate::erreurs::{Erreur, Resultat};
use crate::nuage_steam::{AMettre, Emplacement};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// La forme de la fiche d'abri.
pub const VERSION_FICHE: u32 = 1;
/// Un abri fait à la main depuis « 📁 Où sont mes parties » (lot S2).
pub const SORTE_EMPLACEMENTS: &str = "emplacements";
/// Un abri des parties d'un jeu Steam (lot B2) : son nuage, et ses emplacements hors du nuage au format du lot S2.
pub const SORTE_NUAGE_STEAM: &str = "nuage-steam";
/// Un abri du dossier du jeu fait par Frogtend (avant un retrait, une réinstallation, un mod : `Noyau::mettre_a_l_abri`).
pub const SORTE_JEU: &str = "jeu";
/// Le dossier, dans un abri, des emplacements de parties (lot S2).
pub const PREFIXE: &str = "emplacements/";
/// La fiche des emplacements envoyée chez Firehouse avec eux (`jeux/<id>/emplacements/fiche.json`, pour le lot S5) :
/// jamais posée dans le dossier du jeu.
pub const FICHE_ENVOYEE: &str = "emplacements/fiche.json";
/// Le sel secret du profil (dans SES données, jamais envoyé).
pub const NOM_SEL: &str = "abris-sel.txt";

/// Un emplacement de parties tel qu'il est rangé dans la fiche d'un abri.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmplacementAbrite {
    /// Le nom de son dossier dans l'abri (`emplacements/<cle>/`).
    pub cle: String,
    /// NON résolu (jetons du contrat 1.25) : il vaut sur tout PC.
    pub chemin: String,
    /// `dossier`, `fichier` ou `motif`.
    pub nature: String,
    /// Seulement les fichiers de ce nom (ou de ce motif).
    #[serde(default)]
    pub filtre: Option<String>,
    /// L'empreinte salée du sous-dossier pris à la place de `<identifiant …>` (jamais son nom).
    #[serde(default)]
    pub identifiant: Option<String>,
}

/// Un fichier d'un abri (chemin dans l'abri, `/`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FichierAbrite {
    pub relatif: String,
    pub taille: u64,
    pub sha: String,
}

/// La fiche d'un abri, à côté de lui (`<date>.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FicheAbri {
    /// Toujours présente (les fiches d'avant le lot S2 n'en ont pas : `{version du jeu, complet, …}`).
    pub version_fiche: u32,
    pub sorte: String,
    /// Le profil Frogtend propriétaire (son identifiant) : celui de la session au moment de l'abri.
    pub profil: String,
    /// Rien n'a manqué : aucun emplacement en échec, aucun fichier « en ligne seulement ».
    #[serde(default)]
    pub complet: bool,
    /// La version du jeu (sorte `jeu`, lue comme avant par la remise des parties).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,
    #[serde(default)]
    pub emplacements: Vec<EmplacementAbrite>,
    #[serde(default)]
    pub fichiers: Vec<FichierAbrite>,
    #[serde(default)]
    pub octets: u64,
    #[serde(default)]
    pub en_ligne_seulement: usize,
    /// Emplacements en échec cette fois (pas dans l'abri).
    #[serde(default)]
    pub echecs: usize,
}

impl FicheAbri {
    pub fn nouvelle(sorte: &str, profil: &str, emplacements: Vec<EmplacementAbrite>, fichiers: &[AMettre], en_ligne_seulement: usize, echecs: usize) -> Self {
        FicheAbri {
            version_fiche: VERSION_FICHE,
            sorte: sorte.into(),
            profil: profil.into(),
            complet: echecs == 0 && en_ligne_seulement == 0,
            version: None,
            emplacements,
            fichiers: fichiers.iter().map(|f| FichierAbrite { relatif: f.relatif.clone(), taille: f.taille, sha: f.sha.clone() }).collect(),
            octets: fichiers.iter().map(|f| f.taille).sum(),
            en_ligne_seulement,
            echecs,
        }
    }
}

/// Un chemin de ce PC en clé de comparaison (sans tenir compte des majuscules, comme Windows).
pub fn normal(p: &Path) -> String {
    p.to_string_lossy().replace('/', "\\").to_lowercase()
}

fn empreinte(texte: &str) -> String {
    use sha1::{Digest, Sha1};
    Sha1::digest(texte.as_bytes()).iter().take(4).map(|b| format!("{b:02x}")).collect()
}

/// La clé stable d'un emplacement NON résolu (8 hexadécimaux) : la même sur tout PC, pour tout utilisateur.
pub fn cle_chemin(chemin: &str, nature: &str) -> String {
    let c = chemin.trim().replace('/', "\\");
    empreinte(&format!("frogtend-emplacement:{nature}:{}", c.trim_end_matches('\\').to_lowercase()))
}

/// L'empreinte d'un sous-dossier énuméré (un compte, un profil du jeu), mêlée au sel du profil : jamais son nom dans
/// un abri, et on ne le retrouve pas en essayant des numéros de compte.
pub fn empreinte_identifiant(id: &str, sel: &str) -> String {
    empreinte(&format!("frogtend-identifiant:{sel}:{}", id.trim().to_lowercase()))
}

/// Le sel secret du profil (32 hexadécimaux, dans ses données) ; créé au premier besoin.
pub fn sel_du_profil(dossier_profil: &Path) -> Resultat<String> {
    let f = dossier_profil.join(NOM_SEL);
    if let Ok(t) = std::fs::read_to_string(&f) {
        let t = t.trim().to_string();
        if t.len() >= 16 && t.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Ok(t);
        }
    }
    use rand::RngCore;
    let mut o = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut o);
    let sel: String = o.iter().map(|b| format!("{b:02x}")).collect();
    std::fs::create_dir_all(dossier_profil)?;
    let provisoire = dossier_profil.join(format!("{NOM_SEL}.provisoire"));
    std::fs::write(&provisoire, &sel)?;
    std::fs::rename(&provisoire, &f)?;
    Ok(sel)
}

/// Le nom du dossier d'un emplacement dans l'abri.
pub fn nom_dans_l_abri(e: &Emplacement, sel: &str) -> String {
    let cle = cle_chemin(&e.chemin, &e.nature);
    match e.identifiant.as_deref() {
        Some(i) => format!("{cle}-{}", empreinte_identifiant(i, sel)),
        None => cle,
    }
}

/// Pourquoi un emplacement n'est pas dans l'abri.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Echec {
    /// Trop de fichiers, de dossiers, ou trop profond : ce n'est pas un dossier de parties.
    Trop,
    /// Plus de `nuage_steam::EMPLACEMENT_MAX` : rien n'en est lu.
    TropGros,
    /// Le dossier ne se lit pas (droits…).
    Illisible,
    /// Un fichier ne se lit pas : utilisé (verrouillé) par un autre programme ?
    Verrouille,
    /// Un fichier n'a pas pu être copié.
    Copie,
    /// Un fichier a changé pendant la copie.
    Change,
}

impl Echec {
    pub fn texte(self) -> &'static str {
        match self {
            Echec::Trop => "trop de fichiers pour un dossier de parties : rien n’en est copié",
            Echec::TropGros => "plus de 2 Go : ce n’est pas un dossier de parties, rien n’en est copié",
            Echec::Illisible => "le dossier ne se lit pas sur ce PC",
            Echec::Verrouille => "un fichier ne se lit pas (le jeu ou un autre programme l’utilise peut-être)",
            Echec::Copie => "un fichier n’a pas pu être copié (le jeu ou un autre programme l’utilise peut-être)",
            Echec::Change => "un fichier a changé pendant la copie : réessaie dans un moment",
        }
    }
}

/// Un emplacement dans le plan d'un abri.
#[derive(Debug, Clone)]
pub struct PlanEmplacement {
    pub abrite: EmplacementAbrite,
    /// Le dossier de CE PC : pour l'écran seulement (jamais rangé, jamais au journal).
    pub dossier: PathBuf,
    pub fichiers: Vec<AMettre>,
    pub octets: u64,
    pub echec: Option<Echec>,
}

/// Ce que Frogtend mettrait à l'abri.
#[derive(Debug, Clone, Default)]
pub struct PlanAbri {
    /// Seulement ceux qui ont quelque chose à copier, ou un échec à dire.
    pub emplacements: Vec<PlanEmplacement>,
    /// Fichiers « en ligne seulement » (OneDrive…), jamais lus.
    pub en_ligne_seulement: usize,
    /// La plus récente écriture d'un fichier à copier (le jeu écrit-il encore ?).
    pub ecrit_le: Option<std::time::SystemTime>,
}

impl PlanAbri {
    /// Les emplacements qui seront (ou ont été) copiés.
    pub fn bons(&self) -> impl Iterator<Item = &PlanEmplacement> {
        self.emplacements.iter().filter(|e| e.echec.is_none())
    }
    pub fn fichiers(&self) -> usize {
        self.bons().map(|e| e.fichiers.len()).sum()
    }
    pub fn octets(&self) -> u64 {
        self.bons().map(|e| e.octets).sum()
    }
    pub fn echecs(&self) -> usize {
        self.emplacements.len() - self.bons().count()
    }
    /// Un fichier à copier a été écrit il y a moins de `delai` : le jeu écrit peut-être encore ses parties.
    pub fn ecrit_recemment(&self, maintenant: std::time::SystemTime, delai: std::time::Duration) -> bool {
        self.ecrit_le.is_some_and(|t| maintenant.duration_since(t).map_or(true, |d| d < delai))
    }
    /// (chemin dans l'abri, SHA1) de chaque fichier à copier : ce qui dit « rien de neuf ».
    fn cles(&self) -> BTreeSet<(String, String)> {
        self.bons().flat_map(|e| e.fichiers.iter()).map(|f| (f.relatif.to_lowercase(), f.sha.clone())).collect()
    }
}

/// Le plan d'un abri : les fichiers de ces emplacements (sans lien ni jonction). Les dossiers pris EN ENTIER passent
/// d'abord ; un emplacement contenu dans un dossier entier RÉUSSI n'est pas pris deux fois (dans un dossier en échec,
/// il est pris pour lui-même) ; un fichier de `deja` (chemins `normal`) non plus. Chaque fichier est lu une fois pour
/// son SHA1 ; un emplacement en échec ne gêne jamais les autres. `sel` : celui du profil (`sel_du_profil`).
pub fn planifier(lieux: &[Emplacement], deja: &BTreeSet<String>, sel: &str) -> PlanAbri {
    planifier_jusqua(lieux, deja, sel, crate::nuage_steam::EMPLACEMENT_MAX)
}

fn planifier_jusqua(lieux: &[Emplacement], deja: &BTreeSet<String>, sel: &str, max: u64) -> PlanAbri {
    let mut plan = PlanAbri::default();
    let mut vus = deja.clone();
    let mut noms: BTreeSet<String> = BTreeSet::new();
    // Les dossiers entiers d'abord, du plus haut au plus profond, puis les fichiers et motifs.
    let mut ordre: Vec<&Emplacement> = lieux.iter().filter(|e| e.dossier.is_dir()).collect();
    ordre.sort_by_key(|e| (e.filtre.is_some(), e.dossier.components().count()));
    let mut reussis: Vec<String> = Vec::new();
    for e in ordre {
        let moi = format!("{}\\", normal(&e.dossier));
        if reussis.iter().any(|x| moi.starts_with(x.as_str())) {
            continue;
        }
        let nom = nom_dans_l_abri(e, sel);
        if !noms.insert(nom.clone()) {
            continue;
        }
        let mut pe = PlanEmplacement {
            abrite: EmplacementAbrite { cle: nom.clone(), chemin: e.chemin.clone(), nature: e.nature.clone(), filtre: e.filtre.clone(), identifiant: e.identifiant.as_deref().map(|i| empreinte_identifiant(i, sel)) },
            dossier: e.dossier.clone(),
            fichiers: Vec::new(),
            octets: 0,
            echec: None,
        };
        let relatifs: std::result::Result<Vec<String>, Echec> = match &e.filtre {
            None => crate::nuage_steam::parcourir(&e.dossier).map_err(|a| match a {
                crate::nuage_steam::Arret::Trop => Echec::Trop,
                crate::nuage_steam::Arret::Illisible => Echec::Illisible,
            }),
            Some(f) => std::fs::read_dir(&e.dossier)
                .map(|l| l.flatten().filter(|x| x.file_type().is_ok_and(|t| t.is_file())).map(|x| x.file_name().to_string_lossy().to_string()).filter(|n| crate::nuage_steam::repond(n, f)).collect())
                .map_err(|_| Echec::Illisible),
        };
        let relatifs = match relatifs {
            Ok(l) => l,
            Err(x) => {
                pe.echec = Some(x);
                plan.emplacements.push(pe);
                continue;
            }
        };
        let a_prendre: Vec<(PathBuf, String, std::fs::Metadata)> = relatifs
            .into_iter()
            .filter_map(|r| {
                let p = e.dossier.join(&r);
                let m = std::fs::symlink_metadata(&p).ok()?;
                (m.is_file() && !vus.contains(&normal(&p))).then_some((p, r, m))
            })
            .collect();
        // La taille d'abord (rien n'est lu au-delà du plafond) ; un fichier « en ligne seulement » n'y compte pas
        // (il ne sera jamais copié).
        let (en_ligne, sur_le_pc): (Vec<_>, Vec<_>) = a_prendre.into_iter().partition(|(_, _, m)| crate::nuage_steam::en_ligne_seulement(m));
        if sur_le_pc.iter().map(|(_, _, m)| m.len()).sum::<u64>() > max {
            pe.echec = Some(Echec::TropGros);
            plan.emplacements.push(pe);
            continue;
        }
        let mut ecrit_le: Option<std::time::SystemTime> = None;
        for (p, r, m) in sur_le_pc {
            match crate::nuage_steam::sha1(&p) {
                Ok(sha) => {
                    if let Ok(t) = m.modified() {
                        ecrit_le = Some(ecrit_le.map_or(t, |x| x.max(t)));
                    }
                    pe.fichiers.push(AMettre { source: p, relatif: format!("{PREFIXE}{nom}/{}", r.replace('\\', "/")), taille: m.len(), sha })
                }
                Err(_) => {
                    pe.echec = Some(Echec::Verrouille);
                    break;
                }
            }
        }
        if pe.echec.is_some() {
            pe.fichiers.clear();
            plan.emplacements.push(pe);
            continue;
        }
        plan.en_ligne_seulement += en_ligne.len();
        if e.filtre.is_none() {
            reussis.push(moi);
        }
        if pe.fichiers.is_empty() {
            continue;
        }
        if let Some(t) = ecrit_le {
            plan.ecrit_le = Some(plan.ecrit_le.map_or(t, |x| x.max(t)));
        }
        for f in &pe.fichiers {
            vus.insert(normal(&f.source));
        }
        pe.octets = pe.fichiers.iter().map(|f| f.taille).sum();
        plan.emplacements.push(pe);
    }
    plan
}

/// Copie les fichiers d'un emplacement dans le dossier provisoire, chacun vérifié par son SHA1 (recopié une fois s'il a
/// changé).
fn copier_emplacement(e: &PlanEmplacement, tmp: &Path) -> std::result::Result<(), Echec> {
    for f in &e.fichiers {
        let cible = tmp.join(&f.relatif);
        if let Some(p) = cible.parent() {
            std::fs::create_dir_all(p).map_err(|_| Echec::Copie)?;
        }
        let mut bon = false;
        for _ in 0..2 {
            std::fs::copy(&f.source, &cible).map_err(|_| Echec::Copie)?;
            if crate::nuage_steam::sha1(&cible).map_err(|_| Echec::Copie)? == f.sha {
                bon = true;
                break;
            }
        }
        if !bon {
            return Err(Echec::Change);
        }
    }
    Ok(())
}

/// La fiche d'un abri `<…>/<jeu>/<date>` : `<…>/<jeu>/<date>.json`.
pub fn fichier_fiche(dossier_abri: &Path) -> Option<PathBuf> {
    let nom = dossier_abri.file_name()?.to_string_lossy().to_string();
    Some(dossier_abri.with_file_name(format!("{nom}.json")))
}

/// La fiche (lot S2 et après) d'un abri ; `None` : un abri d'avant (sans fiche, ou une fiche sans `version_fiche`).
pub fn lire_fiche(dossier_abri: &Path) -> Option<FicheAbri> {
    let o = std::fs::read(fichier_fiche(dossier_abri)?).ok()?;
    serde_json::from_slice::<FicheAbri>(&o).ok().filter(|f| f.version_fiche >= 1)
}

/// Un abri fait à la main (lot S2) : jamais pris pour l'abri d'un retrait ni d'une réinstallation, ni envoyé par la
/// sauvegarde du profil (lot S4).
pub fn fait_a_la_main(dossier_abri: &Path) -> bool {
    lire_fiche(dossier_abri).is_some_and(|f| f.sorte == SORTE_EMPLACEMENTS)
}

/// Les sous-dossiers d'un abri (ou de parties restaurées) qui ne sont PAS des fichiers du dossier du jeu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Reservees {
    /// `emplacements/` : des parties rangées hors du dossier du jeu (lot S2).
    pub emplacements: bool,
    /// `nuage-steam/` : les parties du nuage Steam (lot B2).
    pub nuage: bool,
}

impl Reservees {
    /// Ce fichier (chemin `/` dans l'abri) est-il dans un dossier réservé ?
    pub fn ecarte(&self, r: &str) -> bool {
        (self.emplacements && r.starts_with(PREFIXE)) || (self.nuage && r.starts_with("nuage-steam/"))
    }
    /// Les dossiers réservés de premier niveau (pour `restauration::reposer_sauf`).
    pub fn dossiers(&self) -> Vec<&'static str> {
        let mut l = Vec::new();
        if self.emplacements {
            l.push("emplacements");
        }
        if self.nuage {
            l.push("nuage-steam");
        }
        l
    }
}

/// Ce qui est réservé dans `racine` (un abri, ou les parties d'un jeu restaurées) — seulement quand elle le DIT : une
/// fiche S2 (sorte `emplacements` ou `nuage-steam`), la fiche envoyée (`emplacements/fiche.json`), ou la liste Frogtend
/// d'un abri Steam (`nuage-steam/<marque>/abri.json`). Sinon, un sous-dossier `emplacements` est un dossier du jeu.
pub fn reservees(racine: &Path) -> Reservees {
    let s2 = lire_fiche(racine).is_some_and(|f| f.sorte == SORTE_EMPLACEMENTS || f.sorte == SORTE_NUAGE_STEAM);
    let envoyee = std::fs::read(racine.join(FICHE_ENVOYEE)).ok().and_then(|o| serde_json::from_slice::<FicheAbri>(&o).ok()).is_some_and(|f| f.version_fiche >= 1);
    let liste_steam = std::fs::read_dir(racine.join("nuage-steam")).into_iter().flatten().flatten().any(|e| e.path().join("abri.json").is_file());
    Reservees { emplacements: s2 || envoyee, nuage: s2 || liste_steam }
}

/// Pose un abri copié dans `tmp` sous son nom `fin`, sa fiche d'abord (fichier provisoire renommé) : seulement si chaque
/// fichier de la fiche est bien dans `tmp` à sa taille ; si le dossier ne prend pas son nom, la fiche part (jamais une
/// fiche sans abri, jamais un abri à moitié fait).
pub fn poser_avec_fiche(tmp: &Path, fin: &Path, fiche: &FicheAbri) -> Resultat<()> {
    let f = fichier_fiche(fin).ok_or_else(|| Erreur::Disque("Le nom de l'abri est invalide.".into()))?;
    if f.exists() || fin.exists() {
        return Err(Erreur::Refus("Un abri de ce jeu vient d'être fait : réessaie dans une seconde.".into()));
    }
    for x in &fiche.fichiers {
        if std::fs::symlink_metadata(tmp.join(&x.relatif)).map(|m| m.is_file() && m.len() == x.taille).ok() != Some(true) {
            return Err(Erreur::Disque("Un fichier manque dans l'abri après la copie : rien n'est mis à l'abri.".into()));
        }
    }
    let provisoire = f.with_extension("json.provisoire");
    std::fs::write(&provisoire, serde_json::to_vec_pretty(fiche).map_err(|_| Erreur::Disque("La fiche de l'abri est impossible à écrire.".into()))?)?;
    if let Err(e) = std::fs::rename(&provisoire, &f) {
        let _ = std::fs::remove_file(&provisoire);
        return Err(e.into());
    }
    if let Err(e) = std::fs::rename(tmp, fin) {
        let _ = std::fs::remove_file(&f);
        return Err(e.into());
    }
    Ok(())
}

/// Un nom de dossier provisoire propre à CETTE mise à l'abri (jamais partagé avec une autre, même au même instant).
fn provisoire(abris: &Path, id: i64, quand: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    abris.join(".frogtend-en-cours").join(format!("main-{id}-{quand}-{}-{n}", std::process::id()))
}

/// Copie le plan dans un nouvel abri `<abris>/<id>/<quand>/` (lot S2), par un dossier provisoire hors des abris lus.
/// Refusé sans assez de place sur le disque des abris. Un emplacement dont un fichier ne se copie pas (ou change) est
/// retiré de l'abri (son `echec` est posé dans le plan) ; si aucun ne passe, rien n'est fait. Rend le dossier de l'abri.
pub fn abriter(plan: &mut PlanAbri, abris: &Path, id: i64, quand: &str, profil: &str) -> Resultat<PathBuf> {
    let fin = abris.join(id.to_string()).join(quand);
    if fin.exists() || fichier_fiche(&fin).is_some_and(|f| f.exists()) {
        return Err(Erreur::Refus("Un abri de ce jeu vient d'être fait : réessaie dans une seconde.".into()));
    }
    std::fs::create_dir_all(abris)?;
    if crate::jeux_pc::place_libre(abris).is_some_and(|l| l < plan.octets().saturating_add(crate::jeux_pc::MARGE_LIBRE)) {
        return Err(Erreur::Refus("Pas assez de place sur le disque des abris pour copier ces parties.".into()));
    }
    let tmp = provisoire(abris, id, quand);
    let r = (|| -> Resultat<()> {
        std::fs::create_dir_all(&tmp)?;
        for e in plan.emplacements.iter_mut().filter(|e| e.echec.is_none()) {
            if let Err(x) = copier_emplacement(e, &tmp) {
                // Le dossier de CET emplacement dans le provisoire (le nôtre) : retiré, les autres restent.
                let _ = std::fs::remove_dir_all(tmp.join(PREFIXE.trim_end_matches('/')).join(&e.abrite.cle));
                e.echec = Some(x);
            }
        }
        if plan.fichiers() == 0 {
            return Err(Erreur::Refus("Aucune partie n'a pu être copiée : rien n'est mis à l'abri.".into()));
        }
        let fichiers: Vec<AMettre> = plan.bons().flat_map(|e| e.fichiers.iter().cloned()).collect();
        let fiche = FicheAbri::nouvelle(SORTE_EMPLACEMENTS, profil, plan.bons().map(|e| e.abrite.clone()).collect(), &fichiers, plan.en_ligne_seulement, plan.echecs());
        if let Some(p) = fin.parent() {
            std::fs::create_dir_all(p)?;
        }
        poser_avec_fiche(&tmp, &fin, &fiche)
    })();
    if r.is_err() {
        // Le dossier provisoire est le nôtre (son nom le dit) : rien d'autre n'est touché.
        let _ = std::fs::remove_dir_all(&tmp);
    }
    let _ = std::fs::remove_dir(abris.join(".frogtend-en-cours")); // seulement s'il est vide
    r?;
    Ok(fin)
}

/// Les abris FINIS d'un dossier d'abris de jeu (noms tout en chiffres), du plus récent au plus ancien.
fn dates(du_jeu: &Path) -> Vec<(u64, PathBuf)> {
    let mut l: Vec<(u64, PathBuf)> = std::fs::read_dir(du_jeu)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            (!n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())).then(|| n.parse::<u64>().ok().map(|q| (q, e.path()))).flatten()
        })
        .collect();
    l.sort_by(|a, b| b.0.cmp(&a.0));
    l
}

/// Le plus récent abri fait à la main PAR CE PROFIL contient-il déjà exactement ces fichiers (chemins et SHA1) ?
/// `sources` : les dossiers d'abris de ce jeu (et de ses anciens numéros).
pub fn deja_a_l_abri(sources: &[PathBuf], profil: &str, plan: &PlanAbri) -> bool {
    let mut l: Vec<(u64, FicheAbri)> = sources.iter().flat_map(|d| dates(d)).filter_map(|(q, d)| lire_fiche(&d).map(|f| (q, f))).filter(|(_, f)| f.sorte == SORTE_EMPLACEMENTS && f.profil == profil).collect();
    l.sort_by(|a, b| b.0.cmp(&a.0));
    let Some((_, f)) = l.first() else { return false };
    let avant: BTreeSet<(String, String)> = f.fichiers.iter().map(|x| (x.relatif.to_lowercase(), x.sha.clone())).collect();
    avant == plan.cles()
}

/// Les parties `emplacements/` des abris Steam (sorte `nuage-steam`) de CE profil à envoyer avec sa sauvegarde :
/// chaque emplacement est pris dans le plus récent de SES abris qui le contient (un emplacement absent du dernier abri
/// — fichier verrouillé, hors ligne, trop gros avant un mod — se prend dans le précédent : sa copie déjà envoyée ne sort
/// jamais de la sauvegarde). `abris` : tous les abris du jeu (dossiers datés, de tous les numéros du jeu). Rend les
/// fichiers (source, chemin dans l'abri) et la fiche fusionnée (sans aucun chemin du PC), ou rien.
pub fn emplacements_a_envoyer(abris: &[(u64, PathBuf)], profil: &str) -> (Vec<(PathBuf, String)>, Option<FicheAbri>) {
    let mut a: Vec<(u64, &PathBuf, FicheAbri)> = abris.iter().filter_map(|(q, d)| lire_fiche(d).map(|f| (*q, d, f))).filter(|(_, _, f)| f.sorte == SORTE_NUAGE_STEAM && f.profil == profil).collect();
    a.sort_by(|x, y| y.0.cmp(&x.0));
    let mut pris: Vec<EmplacementAbrite> = Vec::new();
    let mut fichiers: Vec<FichierAbrite> = Vec::new();
    let mut sources: Vec<(PathBuf, String)> = Vec::new();
    for (_, d, f) in &a {
        for e in &f.emplacements {
            if pris.iter().any(|p| p.cle == e.cle) {
                continue;
            }
            let prefixe = format!("{PREFIXE}{}/", e.cle);
            let siens: Vec<&FichierAbrite> = f.fichiers.iter().filter(|x| x.relatif.starts_with(&prefixe) && d.join(&x.relatif).is_file()).collect();
            if siens.is_empty() {
                continue;
            }
            pris.push(e.clone());
            for x in siens {
                sources.push((d.join(&x.relatif), x.relatif.clone()));
                fichiers.push(x.clone());
            }
        }
    }
    if pris.is_empty() {
        return (vec![], None);
    }
    let octets = fichiers.iter().map(|x| x.taille).sum();
    let fiche = FicheAbri { version_fiche: VERSION_FICHE, sorte: SORTE_NUAGE_STEAM.into(), profil: profil.into(), complet: true, version: None, emplacements: pris, fichiers, octets, en_ligne_seulement: 0, echecs: 0 };
    (sources, Some(fiche))
}

/// Un abri d'un jeu, montré dans « 📁 Où sont mes parties » (seulement ceux du profil ouvert, et les anciens sans
/// propriétaire connu ; jamais un chemin).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AbriVu {
    /// Secondes depuis 1970.
    pub date: String,
    pub fichiers: usize,
    pub octets: u64,
    /// `emplacements` (fait à la main), `nuage-steam` (parties d'un jeu Steam), `jeu` (le dossier du jeu, avant un mod,
    /// une réinstallation ou un retrait), `retrait` (le même, d'avant le lot S2), ou la sorte d'une fiche plus récente.
    pub sorte: String,
    /// Fait par ce profil (sa fiche le dit) ; `false` : un ancien abri, sans propriétaire connu.
    pub a_moi: bool,
    /// Emplacements qui n'ont pas pu être copiés cette fois-là.
    pub echecs: usize,
    pub en_ligne_seulement: usize,
    /// Fichiers d'un ancien abri Steam pris hors du nuage, dont le chemin d'origine n'a pas été gardé (avant le lot S2) :
    /// ils restent lisibles, Frogtend ne sait pas où les remettre.
    pub sans_origine: usize,
}

/// La liste Frogtend d'un ancien abri Steam (`nuage-steam/<marque>/abri.json`) : pas une partie.
fn liste_steam(r: &str) -> bool {
    r.starts_with("nuage-steam/") && r.split('/').count() == 3 && r.ends_with("/abri.json")
}

/// Les abris d'un jeu pour ce profil, du plus récent au plus ancien : ceux dont la fiche dit ce profil, et les anciens
/// (sans fiche) — sauf un ancien abri Steam d'un AUTRE compte Steam que celui du profil (`marque_steam`, quand il est
/// connu), et un abri sans fiche qui contient `emplacements/` (son propriétaire ne se sait plus : jamais montré). Un
/// abri d'un autre profil n'est jamais montré ni compté. Lecture seule.
pub fn lister(sources: &[PathBuf], profil: &str, marque_steam: Option<&str>) -> Vec<AbriVu> {
    let mut l: Vec<(u64, AbriVu)> = Vec::new();
    for du_jeu in sources {
        for (q, d) in dates(du_jeu) {
            let date = d.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            if let Some(f) = lire_fiche(&d) {
                if f.profil != profil {
                    continue;
                }
                l.push((q, AbriVu { date, fichiers: f.fichiers.len(), octets: f.octets, sorte: f.sorte, a_moi: true, echecs: f.echecs, en_ligne_seulement: f.en_ligne_seulement, sans_origine: 0 }));
                continue;
            }
            // Un ancien abri : ses fichiers, tels quels (jamais réécrit).
            let Ok(fichiers) = crate::installation::fichiers_de(&d) else { continue };
            if fichiers.iter().any(|r| r.starts_with(PREFIXE)) {
                continue;
            }
            let parties: Vec<&String> = fichiers.iter().filter(|r| !liste_steam(r)).collect();
            if parties.is_empty() {
                continue;
            }
            let marques: BTreeSet<&str> = parties.iter().filter_map(|r| r.strip_prefix("nuage-steam/")).filter_map(|r| r.split('/').next()).collect();
            if let Some(m) = marque_steam {
                if !marques.is_empty() && !marques.contains(m) {
                    continue;
                }
            }
            let steam = !marques.is_empty();
            let octets = parties.iter().map(|r| std::fs::symlink_metadata(d.join(r)).map(|m| m.len()).unwrap_or(0)).sum();
            let sans_origine = parties.iter().filter(|r| r.starts_with("nuage-steam/") && r.split('/').nth(2) == Some("hors-nuage")).count();
            let sorte = if steam { SORTE_NUAGE_STEAM } else { "retrait" };
            l.push((q, AbriVu { date, fichiers: parties.len(), octets, sorte: sorte.into(), a_moi: false, echecs: 0, en_ligne_seulement: 0, sans_origine }));
        }
    }
    l.sort_by(|a, b| b.0.cmp(&a.0));
    l.into_iter().map(|(_, a)| a).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEL: &str = "0123456789abcdef0123456789abcdef";

    fn sha_de(o: &[u8]) -> String {
        use sha1::{Digest, Sha1};
        Sha1::digest(o).iter().map(|b| format!("{b:02x}")).collect()
    }

    fn lieu(dossier: &Path, chemin: &str, nature: &str, filtre: Option<&str>) -> Emplacement {
        Emplacement { dossier: dossier.to_path_buf(), filtre: filtre.map(String::from), chemin: chemin.into(), nature: nature.into(), identifiant: None }
    }

    fn planifier(l: &[Emplacement], d: &BTreeSet<String>) -> PlanAbri {
        super::planifier(l, d, SEL)
    }

    /// Un PC fabriqué : deux emplacements de parties (un dossier, un motif), jamais un vrai dossier de Seb.
    fn pc() -> (tempfile::TempDir, PathBuf, PathBuf, Vec<Emplacement>) {
        let d = tempfile::tempdir().unwrap();
        let saves = d.path().join("Utilisateurs").join("Léa").join("Saved Games").join("Jeu");
        let low = d.path().join("Utilisateurs").join("Léa").join("AppData").join("LocalLow").join("Studio");
        std::fs::create_dir_all(saves.join("Slot1")).unwrap();
        std::fs::create_dir_all(&low).unwrap();
        std::fs::write(saves.join("Slot1").join("partie.sav"), b"partie1").unwrap();
        std::fs::write(saves.join("profil.ini"), b"ini").unwrap();
        std::fs::write(low.join("a.save"), b"aa").unwrap();
        std::fs::write(low.join("notes.txt"), b"pas une partie").unwrap();
        let l = vec![
            lieu(&saves, r"%USERPROFILE%\Saved Games\Jeu", "dossier", None),
            lieu(&low, r"%USERPROFILE%\AppData\LocalLow\Studio\*.save", "motif", Some("*.save")),
            // Le motif DANS le dossier déjà pris en entier : pas deux fois.
            lieu(&saves.join("Slot1"), r"%USERPROFILE%\Saved Games\Jeu\Slot1\*.sav", "motif", Some("*.sav")),
            // Pas (encore) de parties ici : ignoré.
            lieu(&d.path().join("Absent"), r"%APPDATA%\Absent", "dossier", None),
        ];
        (d, saves, low, l)
    }

    #[test]
    fn l_abri_est_verifie_au_sha1_avec_sa_fiche_au_chemin_non_resolu() {
        let (d, saves, low, l) = pc();
        let mut plan = planifier(&l, &BTreeSet::new());
        assert_eq!((plan.emplacements.len(), plan.fichiers(), plan.octets(), plan.echecs()), (2, 3, 12, 0));
        let abris = d.path().join("Abris");
        let fin = abriter(&mut plan, &abris, -7, "1000", "profil-lea").unwrap();
        assert_eq!(fin, abris.join("-7").join("1000"));
        let cle = cle_chemin(r"%USERPROFILE%\Saved Games\Jeu", "dossier");
        assert_eq!(std::fs::read(fin.join("emplacements").join(&cle).join("Slot1").join("partie.sav")).unwrap(), b"partie1");
        let cle_low = cle_chemin(r"%USERPROFILE%\AppData\LocalLow\Studio\*.save", "motif");
        assert_eq!(std::fs::read(fin.join("emplacements").join(&cle_low).join("a.save")).unwrap(), b"aa");
        assert!(!fin.join("emplacements").join(&cle_low).join("notes.txt").exists(), "seulement le motif");
        assert!(!abris.join(".frogtend-en-cours").exists());
        // La fiche, à côté : profil, chemins NON résolus, SHA1 ; jamais le chemin de ce PC ni le nom de l'utilisateur.
        let brut = std::fs::read_to_string(abris.join("-7").join("1000.json")).unwrap();
        assert!(!brut.contains("Léa") && !brut.contains(&*d.path().to_string_lossy().replace('\\', "\\\\")), "{brut}");
        let f = lire_fiche(&fin).unwrap();
        assert_eq!((f.version_fiche, f.sorte.as_str(), f.profil.as_str(), f.complet, f.octets), (VERSION_FICHE, SORTE_EMPLACEMENTS, "profil-lea", true, 12));
        assert_eq!(f.emplacements.iter().map(|e| (e.chemin.as_str(), e.nature.as_str(), e.filtre.as_deref())).collect::<Vec<_>>(), [(r"%USERPROFILE%\Saved Games\Jeu", "dossier", None), (r"%USERPROFILE%\AppData\LocalLow\Studio\*.save", "motif", Some("*.save"))]);
        let p = f.fichiers.iter().find(|x| x.relatif.ends_with("Slot1/partie.sav")).unwrap();
        assert_eq!((p.taille, p.sha.as_str()), (7, sha_de(b"partie1").as_str()));
        assert!(fait_a_la_main(&fin));
        // Les parties n'ont pas bougé.
        assert_eq!(std::fs::read(saves.join("profil.ini")).unwrap(), b"ini");
        assert_eq!(std::fs::read(low.join("a.save")).unwrap(), b"aa");
        // La clé ne dépend ni des majuscules, ni des « / », ni du « \ » final.
        assert_eq!(cle_chemin(r"%userprofile%/Saved Games/Jeu\", "dossier"), cle);
        assert_ne!(cle_chemin(r"%USERPROFILE%\Saved Games\Jeu", "fichier"), cle);
    }

    #[test]
    fn complet_seulement_sans_echec_ni_fichier_en_ligne() {
        assert!(FicheAbri::nouvelle(SORTE_EMPLACEMENTS, "p", vec![], &[], 0, 0).complet);
        assert!(!FicheAbri::nouvelle(SORTE_EMPLACEMENTS, "p", vec![], &[], 0, 1).complet);
        assert!(!FicheAbri::nouvelle(SORTE_EMPLACEMENTS, "p", vec![], &[], 2, 0).complet);
    }

    #[test]
    fn rien_de_neuf_pas_de_copie_et_l_abri_d_un_autre_profil_ne_compte_pas() {
        let (d, saves, _, l) = pc();
        let abris = d.path().join("Abris");
        let sources = vec![abris.join("-7")];
        let mut plan = planifier(&l, &BTreeSet::new());
        assert!(!deja_a_l_abri(&sources, "p1", &plan), "aucun abri encore");
        abriter(&mut plan, &abris, -7, "1000", "p1").unwrap();
        let plan = planifier(&l, &BTreeSet::new());
        assert!(deja_a_l_abri(&sources, "p1", &plan), "rien n'a changé");
        assert!(!deja_a_l_abri(&sources, "p2", &plan), "l'abri d'un autre profil ne compte pas");
        std::fs::write(saves.join("Slot1").join("partie.sav"), b"partie2").unwrap();
        let mut plan = planifier(&l, &BTreeSet::new());
        assert!(!deja_a_l_abri(&sources, "p1", &plan), "une partie a changé");
        abriter(&mut plan, &abris, -7, "1001", "p1").unwrap();
        assert!(deja_a_l_abri(&sources, "p1", &planifier(&l, &BTreeSet::new())));
        // Le même instant : jamais écrasé.
        assert!(matches!(abriter(&mut planifier(&l, &BTreeSet::new()), &abris, -7, "1001", "p1"), Err(Erreur::Refus(_))));
        assert_eq!(std::fs::read(abris.join("-7").join("1000").join("emplacements").join(cle_chemin(r"%USERPROFILE%\Saved Games\Jeu", "dossier")).join("Slot1").join("partie.sav")).unwrap(), b"partie1", "l'ancien abri reste tel quel");
    }

    #[test]
    fn un_fichier_verrouille_fait_echouer_son_emplacement_et_les_autres_passent() {
        let (d, _saves, low, l) = pc();
        // Le jeu tient un fichier sans partage (comme une partie ouverte en écriture exclusive).
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            let _verrou = std::fs::OpenOptions::new().read(true).share_mode(0).open(low.join("a.save")).unwrap();
            let mut plan = planifier(&l, &BTreeSet::new());
            assert_eq!(plan.echecs(), 1);
            let e = plan.emplacements.iter().find(|e| e.echec.is_some()).unwrap();
            assert_eq!((e.echec, e.dossier.clone(), e.fichiers.len()), (Some(Echec::Verrouille), low.clone(), 0));
            let abris = d.path().join("Abris");
            let fin = abriter(&mut plan, &abris, -7, "1000", "p1").unwrap();
            let f = lire_fiche(&fin).unwrap();
            assert_eq!((f.emplacements.len(), f.echecs, f.fichiers.len(), f.complet), (1, 1, 2, false), "seulement le dossier qui se lit");
            assert!(!fin.join("emplacements").join(cle_chemin(r"%USERPROFILE%\AppData\LocalLow\Studio\*.save", "motif")).exists());
        }
        let _ = (d, low);
    }

    #[test]
    fn tout_ou_rien_un_fichier_qui_change_pendant_la_copie_retire_son_emplacement() {
        let (d, _, low, l) = pc();
        let abris = d.path().join("Abris");
        // Le SHA1 noté ne correspond plus (le fichier a changé entre le plan et la copie) : cet emplacement est retiré.
        let mut plan = planifier(&l, &BTreeSet::new());
        let i = plan.emplacements.iter().position(|e| e.dossier == low).unwrap();
        plan.emplacements[i].fichiers[0].sha = sha_de(b"autre chose");
        let fin = abriter(&mut plan, &abris, -7, "1000", "p1").unwrap();
        assert_eq!(plan.emplacements[i].echec, Some(Echec::Change));
        assert_eq!(lire_fiche(&fin).unwrap().fichiers.len(), 2);
        // Aucun emplacement ne passe : rien n'est fait (ni abri, ni fiche, ni provisoire).
        let mut plan = planifier(&l, &BTreeSet::new());
        for e in plan.emplacements.iter_mut() {
            for f in e.fichiers.iter_mut() {
                f.sha = "0".repeat(40);
            }
        }
        assert!(matches!(abriter(&mut plan, &abris, -7, "2000", "p1"), Err(Erreur::Refus(_))));
        assert!(!abris.join("-7").join("2000").exists() && !abris.join("-7").join("2000.json").exists());
        assert!(!abris.join(".frogtend-en-cours").exists());
        // Rien à copier du tout : refusé aussi.
        assert!(abriter(&mut PlanAbri::default(), &abris, -7, "3000", "p1").is_err());
        assert!(!abris.join("-7").join("3000").exists());
    }

    #[test]
    fn la_fiche_n_est_posee_que_si_chaque_fichier_est_dans_l_abri() {
        let d = tempfile::tempdir().unwrap();
        let tmp = d.path().join("tmp");
        std::fs::create_dir_all(tmp.join("emplacements").join("k")).unwrap();
        std::fs::write(tmp.join("emplacements").join("k").join("a"), b"12").unwrap();
        let fin = d.path().join("7").join("100");
        std::fs::create_dir_all(fin.parent().unwrap()).unwrap();
        let un = |relatif: &str, taille: u64| AMettre { source: PathBuf::new(), relatif: relatif.into(), taille, sha: String::new() };
        let manque = FicheAbri::nouvelle(SORTE_EMPLACEMENTS, "p", vec![], &[un("emplacements/k/a", 2), un("emplacements/k/b", 1)], 0, 0);
        assert!(poser_avec_fiche(&tmp, &fin, &manque).is_err());
        let mauvaise_taille = FicheAbri::nouvelle(SORTE_EMPLACEMENTS, "p", vec![], &[un("emplacements/k/a", 3)], 0, 0);
        assert!(poser_avec_fiche(&tmp, &fin, &mauvaise_taille).is_err());
        assert!(!fin.exists() && !d.path().join("7").join("100.json").exists() && tmp.is_dir());
        assert!(poser_avec_fiche(&tmp, &fin, &FicheAbri::nouvelle(SORTE_EMPLACEMENTS, "p", vec![], &[un("emplacements/k/a", 2)], 0, 0)).is_ok());
        assert!(fin.join("emplacements").join("k").join("a").is_file() && lire_fiche(&fin).is_some());
        // Deux dossiers provisoires au même instant : jamais le même nom (ni celui de l'abri Steam, `{id}-{quand}`).
        let (a, b) = (provisoire(d.path(), 7, "100"), provisoire(d.path(), 7, "100"));
        assert_ne!(a, b);
        assert!(!a.ends_with("7-100"));
    }

    #[test]
    fn un_emplacement_trop_gros_ou_trop_garni_est_dit_et_n_empeche_rien() {
        let (_d, saves, _, l) = pc();
        // Plafond abaissé pour le test (2 Go en vrai).
        let plan = planifier_jusqua(&l, &BTreeSet::new(), SEL, 8);
        let gros = plan.emplacements.iter().find(|e| e.dossier == saves).unwrap();
        assert_eq!((gros.echec, gros.fichiers.len()), (Some(Echec::TropGros), 0));
        // Le dossier entier est en échec : le motif plus précis qu'il contient (Slot1\*.sav) est pris pour lui-même.
        assert!(plan.emplacements.iter().any(|e| e.dossier == saves.join("Slot1") && e.echec.is_none() && e.fichiers.len() == 1), "{:?}", plan.emplacements.iter().map(|e| (&e.dossier, e.echec)).collect::<Vec<_>>());
        assert_eq!((plan.fichiers(), plan.echecs()), (2, 1), "le motif du dossier en échec et l'autre motif passent");
        // Un dossier trop profond : pas un dossier de parties.
        let mut profond = saves.join("p");
        for i in 0..14 {
            profond = profond.join(format!("n{i}"));
        }
        std::fs::create_dir_all(&profond).unwrap();
        let plan = planifier(&l, &BTreeSet::new());
        assert_eq!(plan.emplacements.iter().find(|e| e.dossier == saves).unwrap().echec, Some(Echec::Trop));
        assert!(!Echec::Trop.texte().is_empty() && Echec::TropGros.texte().contains("2 Go"));
        // Réussi, le dossier entier couvre le motif : pas deux fois (vu dans le premier test : 2 emplacements).
    }

    #[test]
    fn un_identifiant_enumere_ne_laisse_qu_une_empreinte_salee() {
        let d = tempfile::tempdir().unwrap();
        let l: Vec<Emplacement> = ["76561198000000000", "A1B2"]
            .iter()
            .map(|id| {
                let p = d.path().join("Profiles").join(id);
                std::fs::create_dir_all(&p).unwrap();
                std::fs::write(p.join("SGTA50000"), id.as_bytes()).unwrap();
                Emplacement { identifiant: Some(id.to_string()), ..lieu(&p, r"%USERPROFILE%\Documents\Rockstar Games\GTA V\Profiles\<identifiant utilisateur>", "dossier", None) }
            })
            .collect();
        let mut plan = planifier(&l, &BTreeSet::new());
        assert_eq!(plan.fichiers(), 2, "un emplacement par profil du jeu");
        let fin = abriter(&mut plan, &d.path().join("Abris"), 5, "1", "p1").unwrap();
        let brut = std::fs::read_to_string(fichier_fiche(&fin).unwrap()).unwrap();
        assert!(!brut.contains("76561198000000000") && !brut.contains("A1B2") && !brut.contains(SEL), "jamais le numéro d'un compte ni le sel : {brut}");
        let f = lire_fiche(&fin).unwrap();
        let cle = cle_chemin(&l[0].chemin, "dossier");
        assert_eq!(f.emplacements[0].cle, format!("{cle}-{}", empreinte_identifiant("76561198000000000", SEL)));
        assert_eq!(f.emplacements[0].chemin, l[0].chemin, "le chemin NON résolu");
        // Sans le sel du profil, l'empreinte ne se retrouve pas en essayant le numéro.
        assert_ne!(empreinte_identifiant("76561198000000000", SEL), empreinte_identifiant("76561198000000000", "autre sel"));
        // Le sel : créé une fois dans les données du profil, puis relu tel quel.
        let profil = d.path().join("profils").join("p1");
        let s1 = sel_du_profil(&profil).unwrap();
        assert_eq!((s1.len(), sel_du_profil(&profil).unwrap()), (32, s1.clone()));
        assert_ne!(sel_du_profil(&d.path().join("profils").join("p2")).unwrap(), s1, "un sel par profil");
    }

    #[test]
    fn un_fichier_en_ligne_seulement_est_saute_compte_et_hors_du_plafond() {
        let (_d, _, low, l) = pc();
        #[cfg(windows)]
        {
            use windows::Win32::Storage::FileSystem::{SetFileAttributesW, FILE_ATTRIBUTE_ARCHIVE, FILE_ATTRIBUTE_OFFLINE};
            // Un gros fichier « en ligne seulement » (taille annoncée, rien sur le disque en vrai) : jamais compté au plafond.
            let gros = low.join("b.save");
            std::fs::write(&gros, vec![0u8; 64]).unwrap();
            let f = windows::core::HSTRING::from(low.join("a.save").as_os_str());
            let g = windows::core::HSTRING::from(gros.as_os_str());
            if unsafe { SetFileAttributesW(&f, FILE_ATTRIBUTE_OFFLINE | FILE_ATTRIBUTE_ARCHIVE) }.is_err() || unsafe { SetFileAttributesW(&g, FILE_ATTRIBUTE_OFFLINE | FILE_ATTRIBUTE_ARCHIVE) }.is_err() {
                eprintln!("attribut « hors ligne » impossible à poser ici : test sauté");
                return;
            }
            let plan = planifier(&l, &BTreeSet::new());
            assert_eq!((plan.en_ligne_seulement, plan.fichiers(), plan.echecs()), (2, 2, 0), "jamais lu, jamais un échec");
            let petit = planifier_jusqua(&l, &BTreeSet::new(), SEL, 10);
            assert_eq!(petit.emplacements.iter().filter(|e| e.dossier == low).count(), 0, "le motif n'a rien sur le PC : rien à copier, pas un échec");
            assert_eq!(petit.echecs(), 0);
            let _ = unsafe { SetFileAttributesW(&f, FILE_ATTRIBUTE_ARCHIVE) };
            let _ = unsafe { SetFileAttributesW(&g, FILE_ATTRIBUTE_ARCHIVE) };
        }
        let _ = low;
    }

    #[test]
    fn la_plus_recente_ecriture_est_notee() {
        let (_d, saves, _, l) = pc();
        let vieux = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
        for f in [saves.join("Slot1").join("partie.sav"), saves.join("profil.ini")] {
            std::fs::OpenOptions::new().write(true).open(&f).unwrap().set_modified(vieux).unwrap();
        }
        let plan = planifier(&l[..1], &BTreeSet::new());
        let maintenant = std::time::SystemTime::now();
        assert!(!plan.ecrit_recemment(maintenant, std::time::Duration::from_secs(60)), "écrit il y a une heure");
        std::fs::write(saves.join("profil.ini"), b"neuf").unwrap();
        assert!(planifier(&l[..1], &BTreeSet::new()).ecrit_recemment(std::time::SystemTime::now(), std::time::Duration::from_secs(60)), "le jeu vient d'écrire");
        assert!(!PlanAbri::default().ecrit_recemment(maintenant, std::time::Duration::from_secs(60)));
    }

    #[test]
    fn seul_un_abri_qui_le_dit_reserve_emplacements_et_nuage_steam() {
        let d = tempfile::tempdir().unwrap();
        // Un jeu de Firehouse dont le dossier a un sous-dossier « emplacements » : un abri sans fiche S2.
        let a = d.path().join("110").join("100");
        std::fs::create_dir_all(a.join("emplacements")).unwrap();
        assert_eq!(reservees(&a), Reservees::default());
        assert!(!reservees(&a).ecarte("emplacements/carte.dat"));
        // Un abri de sorte « jeu » (le dossier du jeu) : rien de réservé non plus.
        std::fs::write(d.path().join("110").join("100.json"), serde_json::to_vec(&FicheAbri::nouvelle(SORTE_JEU, "p", vec![], &[], 0, 0)).unwrap()).unwrap();
        assert_eq!(reservees(&a), Reservees::default());
        // Un abri S2 : les deux.
        std::fs::write(d.path().join("110").join("100.json"), serde_json::to_vec(&FicheAbri::nouvelle(SORTE_NUAGE_STEAM, "p", vec![], &[], 0, 0)).unwrap()).unwrap();
        let r = reservees(&a);
        assert!(r.ecarte("emplacements/0a1b2c3d/x") && r.ecarte("nuage-steam/m/remote/x") && !r.ecarte("SAVE/x"));
        assert_eq!(r.dossiers(), ["emplacements", "nuage-steam"]);
        // Des parties restaurées : la fiche envoyée réserve `emplacements/` ; la liste d'un abri Steam, `nuage-steam/`.
        let z = d.path().join("zone");
        std::fs::create_dir_all(z.join("emplacements")).unwrap();
        std::fs::create_dir_all(z.join("nuage-steam").join("m1")).unwrap();
        assert_eq!(reservees(&z), Reservees::default());
        std::fs::write(z.join(FICHE_ENVOYEE), serde_json::to_vec(&FicheAbri::nouvelle(SORTE_NUAGE_STEAM, "p", vec![], &[], 0, 0)).unwrap()).unwrap();
        std::fs::write(z.join("nuage-steam").join("m1").join("abri.json"), b"[]").unwrap();
        assert_eq!(reservees(&z), Reservees { emplacements: true, nuage: true });
    }

    /// Correctifs 1 et 2 (revue du 22/11) : les emplacements à envoyer viennent des abris Steam de CE profil, chacun du
    /// plus récent qui le contient — un emplacement absent du dernier (verrouillé, hors ligne, trop gros avant un mod)
    /// garde sa copie précédente ; l'abri d'un autre profil n'y change rien.
    #[test]
    fn chaque_emplacement_envoye_vient_du_plus_recent_abri_du_profil_qui_le_contient() {
        let d = tempfile::tempdir().unwrap();
        let du_jeu = d.path().join("-7");
        let poser = |date: &str, profil: &str, emplacements: &[(&str, &[u8])]| {
            let dir = du_jeu.join(date);
            let mut fichiers = Vec::new();
            let mut abrites = Vec::new();
            std::fs::create_dir_all(dir.join("nuage-steam").join("m1")).unwrap();
            std::fs::write(dir.join("nuage-steam").join("m1").join("abri.json"), b"[]").unwrap();
            for (cle, o) in emplacements {
                let r = format!("emplacements/{cle}/save");
                std::fs::create_dir_all(dir.join("emplacements").join(cle)).unwrap();
                std::fs::write(dir.join(&r), o).unwrap();
                fichiers.push(AMettre { source: PathBuf::new(), relatif: r, taille: o.len() as u64, sha: sha_de(o) });
                abrites.push(EmplacementAbrite { cle: cle.to_string(), chemin: format!(r"%APPDATA%\{cle}"), nature: "dossier".into(), filtre: None, identifiant: None });
            }
            std::fs::write(du_jeu.join(format!("{date}.json")), serde_json::to_vec(&FicheAbri::nouvelle(SORTE_NUAGE_STEAM, profil, abrites, &fichiers, 0, 0)).unwrap()).unwrap();
            (date.parse::<u64>().unwrap(), dir)
        };
        let a1 = poser("100", "seb", &[("aaaa", b"seb-a1"), ("bbbb", b"seb-b1")]);
        // Fichier verrouillé : le dernier abri de Seb n'a plus « bbbb ».
        let a2 = poser("200", "seb", &[("aaaa", b"seb-a2")]);
        // Hors ligne (ou trop gros avant un mod) : aucun emplacement du tout.
        let a3 = poser("300", "seb", &[]);
        // Léa, plus récente : jamais prise pour Seb.
        let a4 = poser("400", "lea", &[("aaaa", b"lea"), ("cccc", b"lea-c")]);
        let tous = [a1, a2, a3, a4];
        let (f, fiche) = emplacements_a_envoyer(&tous, "seb");
        let lus: Vec<(String, Vec<u8>)> = f.iter().map(|(s, r)| (r.clone(), std::fs::read(s).unwrap())).collect();
        assert_eq!(lus, [("emplacements/aaaa/save".to_string(), b"seb-a2".to_vec()), ("emplacements/bbbb/save".to_string(), b"seb-b1".to_vec())]);
        let fiche = fiche.unwrap();
        assert_eq!((fiche.profil.as_str(), fiche.emplacements.len(), fiche.fichiers.len()), ("seb", 2, 2));
        assert!(!serde_json::to_string(&fiche).unwrap().contains(&*d.path().to_string_lossy().replace('\\', "\\\\")), "aucun chemin du PC");
        let (l, _) = emplacements_a_envoyer(&tous, "lea");
        assert_eq!(l.iter().map(|(_, r)| r.as_str()).collect::<Vec<_>>(), ["emplacements/aaaa/save", "emplacements/cccc/save"]);
        assert_eq!(emplacements_a_envoyer(&tous, "zoe"), (vec![], None));
    }

    #[test]
    fn la_liste_ne_montre_que_les_abris_du_profil_et_les_anciens_tels_quels() {
        let (d, _, _, l) = pc();
        let abris = d.path().join("Abris");
        let du_jeu = abris.join("-7");
        abriter(&mut planifier(&l, &BTreeSet::new()), &abris, -7, "1000", "p1").unwrap();
        abriter(&mut planifier(&l, &BTreeSet::new()), &abris, -7, "1001", "p2").unwrap();
        // Un abri d'avant un retrait (lot 3 : fiche « version du jeu », sans `version_fiche`).
        std::fs::create_dir_all(du_jeu.join("900").join("SAVE")).unwrap();
        std::fs::write(du_jeu.join("900").join("SAVE").join("1.sav"), b"12345").unwrap();
        std::fs::write(du_jeu.join("900.json"), br#"{"version": 3, "complet": true, "fichiers": 1, "octets": 5}"#).unwrap();
        // Un ancien abri Steam (avant le S2) : son nuage, et des parties hors du nuage sans chemin d'origine.
        let ancien = du_jeu.join("800").join("nuage-steam").join("m1");
        std::fs::create_dir_all(ancien.join("remote")).unwrap();
        std::fs::create_dir_all(ancien.join("hors-nuage").join("abcd1234-SaveData")).unwrap();
        std::fs::write(ancien.join("remote").join("campaign"), b"c").unwrap();
        std::fs::write(ancien.join("hors-nuage").join("abcd1234-SaveData").join("save_1"), b"s1").unwrap();
        std::fs::write(ancien.join("abri.json"), b"[]").unwrap();
        // Celui d'un AUTRE compte Steam du PC.
        std::fs::create_dir_all(du_jeu.join("700").join("nuage-steam").join("m2").join("remote")).unwrap();
        std::fs::write(du_jeu.join("700").join("nuage-steam").join("m2").join("remote").join("x"), b"x").unwrap();
        // Un abri sans fiche qui contient des emplacements (fiche perdue) : son propriétaire ne se sait plus.
        std::fs::create_dir_all(du_jeu.join("600").join("emplacements").join("k")).unwrap();
        std::fs::write(du_jeu.join("600").join("emplacements").join("k").join("x"), b"x").unwrap();
        // Un abri pas fini : jamais montré.
        std::fs::create_dir_all(du_jeu.join("1002.en-cours")).unwrap();
        let avant: Vec<(String, Vec<u8>)> = crate::installation::fichiers_de(&du_jeu.join("800")).unwrap().into_iter().map(|r| (r.clone(), std::fs::read(du_jeu.join("800").join(&r)).unwrap())).collect();

        let vus = lister(&[du_jeu.clone()], "p1", Some("m1"));
        assert_eq!(vus.iter().map(|a| a.date.as_str()).collect::<Vec<_>>(), ["1000", "900", "800"], "jamais l'abri de p2, ni celui de l'autre compte Steam, ni un abri d'emplacements sans fiche");
        assert_eq!((vus[0].fichiers, vus[0].octets, vus[0].sorte.as_str(), vus[0].a_moi), (3, 12, SORTE_EMPLACEMENTS, true));
        assert_eq!((vus[1].fichiers, vus[1].octets, vus[1].sorte.as_str(), vus[1].a_moi), (1, 5, "retrait", false));
        assert_eq!((vus[2].fichiers, vus[2].sorte.as_str(), vus[2].a_moi, vus[2].sans_origine), (2, SORTE_NUAGE_STEAM, false, 1), "la liste Frogtend n'est pas une partie");
        // Compte Steam du profil inconnu : l'autre compte n'est pas écarté (rien de deviné), p2 jamais.
        let tous = lister(&[du_jeu.clone()], "p1", None);
        assert_eq!(tous.iter().map(|a| a.date.as_str()).collect::<Vec<_>>(), ["1000", "900", "800", "700"]);
        assert_eq!(lister(&[du_jeu.clone()], "p2", Some("m1")).iter().filter(|a| a.a_moi).map(|a| a.date.as_str()).collect::<Vec<_>>(), ["1001"]);
        // Lu, jamais réécrit.
        for (r, o) in avant {
            assert_eq!(std::fs::read(du_jeu.join("800").join(&r)).unwrap(), o, "{r}");
        }
        assert!(!du_jeu.join("800.json").exists(), "aucune fiche ajoutée à un ancien abri");
        assert!(!fait_a_la_main(&du_jeu.join("900")) && !fait_a_la_main(&du_jeu.join("800")) && fait_a_la_main(&du_jeu.join("1000")));
    }
}
