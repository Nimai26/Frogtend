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
//! - Un fichier OneDrive « en ligne seulement » n'est jamais lu sans l'accord de la personne (le lire le fait
//!   télécharger par OneDrive) : sauté et compté (hors du plafond de 2 Go). Avec son oui (Seb, 22/11 : « proposer de
//!   les rapatrier »), il est lu en entier pendant la copie (`Rapatriement`) — Windows le télécharge alors et le garde
//!   sur le PC, comme quand on l'ouvre —, avec un délai d'attente par fichier ; un échec de OneDrive fait échouer cet
//!   endroit seulement.
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
/// Le sel secret du profil, dans SES données (jamais réécrit une fois créé). Lot S4 (Seb, 23/11) : il est AUSSI rangé
/// chez Firehouse, dans l'espace du profil (`NOM_SEL_ENVOYE`), pour restaurer sur un autre PC — jamais au journal.
pub const NOM_SEL: &str = "abris-sel.txt";
/// Le nom du document du sel chez Firehouse (contrat 1.51 : tout document est du JSON, `{"sel": "<hex>"}`, sous le PC
/// réservé `_profil`).
pub const NOM_SEL_ENVOYE: &str = "abris-sel.json";

/// Ce qui a fait faire un abri de sorte `emplacements` (lot S3, 23/11) : une fiche d'avant le lot S3 n'en a pas, elle
/// vaut « main ». JAMAIS une nouvelle sorte (la remise d'après une réinstallation poserait `emplacements/` dans le jeu).
pub const DECLENCHEUR_MAIN: &str = "main";
pub const DECLENCHEUR_APRES_PARTIE: &str = "apres_partie";
pub const DECLENCHEUR_AVANT_RETRAIT: &str = "avant_retrait";
pub const DECLENCHEUR_AVANT_REINSTALLATION: &str = "avant_reinstallation";
pub const DECLENCHEUR_AVANT_MISE_A_JOUR: &str = "avant_mise_a_jour";
pub const DECLENCHEUR_AVANT_MOD: &str = "avant_mod";
/// La liste fermée des déclencheurs.
pub const DECLENCHEURS: [&str; 6] = [DECLENCHEUR_MAIN, DECLENCHEUR_APRES_PARTIE, DECLENCHEUR_AVANT_RETRAIT, DECLENCHEUR_AVANT_REINSTALLATION, DECLENCHEUR_AVANT_MISE_A_JOUR, DECLENCHEUR_AVANT_MOD];

/// Un déclencheur de la liste fermée.
pub fn declencheur_connu(d: &str) -> bool {
    DECLENCHEURS.contains(&d)
}

fn declencheur_main() -> String {
    DECLENCHEUR_MAIN.into()
}

/// Un emplacement de parties tel qu'il est rangé dans la fiche d'un abri.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
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
    /// Lot S4 : ses fichiers « en ligne seulement » (OneDrive) NON copiés dans cet abri. `None` : une fiche d'avant
    /// le lot S4, qui ne le disait pas par emplacement (seulement `FicheAbri::en_ligne_seulement`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub en_ligne: Option<usize>,
    /// Lot S4, fiche ENVOYÉE seulement (`emplacements_a_envoyer`) : la date de l'abri d'où vient cet emplacement.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub date: String,
    /// Lot S4, fiche envoyée : le déclencheur de cet abri (`DECLENCHEURS`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub declencheur: String,
    /// Lot S4, fiche envoyée (Seb, 23/11) : des fichiers de cet emplacement n'étaient que dans OneDrive — ses fichiers
    /// sont RÉUNIS avec ceux de sa copie précédente (le plus récent gagne, fichier par fichier).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub partiel: bool,
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
    /// Ce qui a fait faire l'abri (lot S3 : `DECLENCHEURS`) ; absent (fiche plus ancienne) : « main ».
    #[serde(default = "declencheur_main")]
    pub declencheur: String,
    /// Lot S4, fiche ENVOYÉE seulement : la boutique du jeu (pour le lot S5, sur un autre PC), quand il en vient une.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boutique: Option<BoutiqueDuJeu>,
}

/// La boutique d'un jeu (`steam`, `gog`, `epic`…) et sa clé chez elle (l'appid pour Steam).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoutiqueDuJeu {
    pub source: String,
    pub cle: String,
}

impl FicheAbri {
    pub fn nouvelle(sorte: &str, profil: &str, emplacements: Vec<EmplacementAbrite>, fichiers: &[AMettre], en_ligne_seulement: usize, echecs: usize) -> Self {
        FicheAbri {
            version_fiche: VERSION_FICHE,
            declencheur: declencheur_main(),
            sorte: sorte.into(),
            profil: profil.into(),
            complet: echecs == 0 && en_ligne_seulement == 0,
            version: None,
            emplacements,
            fichiers: fichiers.iter().map(|f| FichierAbrite { relatif: f.relatif.clone(), taille: f.taille, sha: f.sha.clone() }).collect(),
            octets: fichiers.iter().map(|f| f.taille).sum(),
            en_ligne_seulement,
            echecs,
            boutique: None,
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
    /// OneDrive n'a pas rendu un fichier « en ligne seulement » (pas connecté, refusé, trop long).
    OneDrive,
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
            Echec::OneDrive => "OneDrive n’a pas pu télécharger ce fichier : vérifie que OneDrive est connecté",
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
    /// Ses fichiers « en ligne seulement » (OneDrive…) : JAMAIS lus ici (seulement par `abriter` avec un `Rapatriement`).
    pub en_ligne: Vec<EnLigne>,
}

/// Un fichier « en ligne seulement » : sa place, son chemin dans l'abri, sa taille (annoncée par OneDrive).
#[derive(Debug, Clone)]
pub struct EnLigne {
    pub source: PathBuf,
    pub relatif: String,
    pub taille: u64,
}

/// Rapatrier les fichiers « en ligne seulement » pendant la copie (après le oui de la personne).
pub struct Rapatriement<'a> {
    /// Copie `source` vers `cible` en LISANT la source en entier (c'est la lecture qui fait télécharger OneDrive).
    pub lecteur: std::sync::Arc<dyn Fn(&Path, &Path) -> std::io::Result<()> + Send + Sync>,
    /// Le délai d'attente d'un fichier de cette taille.
    pub delai: &'a (dyn Fn(u64) -> std::time::Duration + Sync),
    /// (fichiers rapatriés, total) : avant chaque fichier, et à la fin.
    pub progres: &'a (dyn Fn(usize, usize) + Sync),
}

/// La lecture de la vraie vie : la source lue en entier (OneDrive la télécharge), écrite dans `cible`.
pub fn lire_en_entier(source: &Path, cible: &Path) -> std::io::Result<()> {
    let mut a = std::fs::File::open(source)?;
    let mut b = std::fs::File::create(cible)?;
    std::io::copy(&mut a, &mut b)?;
    b.sync_all()
}

/// Le délai d'attente de la vraie vie : 2 minutes, plus une seconde par 256 Kio (une connexion lente).
pub fn delai_de_telechargement(taille: u64) -> std::time::Duration {
    std::time::Duration::from_secs(120 + taille / (256 * 1024))
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
    /// Fichiers rapatriés depuis le nuage et copiés dans l'abri (emplacements réussis seulement).
    pub rapatries: usize,
    /// Lot S4 : la boutique du jeu, notée dans la fiche de l'abri quand il est fait (pour un jeu retiré ensuite).
    pub boutique: Option<BoutiqueDuJeu>,
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
    /// La taille des fichiers « en ligne seulement » des emplacements qui passent (à télécharger, s'il le faut).
    pub fn octets_en_ligne(&self) -> u64 {
        self.bons().flat_map(|e| e.en_ligne.iter()).map(|f| f.taille).sum()
    }
    /// Le nombre de fichiers « en ligne seulement » des emplacements qui passent.
    pub fn fichiers_en_ligne(&self) -> usize {
        self.bons().map(|e| e.en_ligne.len()).sum()
    }
    /// Avant de rapatrier : le plafond par emplacement compte AUSSI ses fichiers en ligne (ils vont être copiés).
    pub fn preparer_rapatriement(&mut self, max: u64) {
        for e in self.emplacements.iter_mut().filter(|e| e.echec.is_none()) {
            if e.octets + e.en_ligne.iter().map(|f| f.taille).sum::<u64>() > max {
                e.echec = Some(Echec::TropGros);
                e.fichiers.clear();
                e.en_ligne.clear();
                e.octets = 0;
            }
        }
        self.en_ligne_seulement = self.fichiers_en_ligne();
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
            abrite: EmplacementAbrite {
                cle: nom.clone(),
                chemin: e.chemin.clone(),
                nature: e.nature.clone(),
                filtre: e.filtre.clone(),
                identifiant: e.identifiant.as_deref().map(|i| empreinte_identifiant(i, sel)),
                en_ligne: Some(0),
                ..Default::default()
            },
            dossier: e.dossier.clone(),
            fichiers: Vec::new(),
            octets: 0,
            echec: None,
            en_ligne: Vec::new(),
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
        pe.en_ligne = en_ligne.into_iter().map(|(p, r, m)| EnLigne { relatif: format!("{PREFIXE}{nom}/{}", r.replace('\\', "/")), source: p, taille: m.len() }).collect();
        // Lot S4 : dit dans la fiche, emplacement par emplacement (sa copie envoyée réunira alors l'ancienne).
        pe.abrite.en_ligne = Some(pe.en_ligne.len());
        if e.filtre.is_none() {
            reussis.push(moi);
        }
        for f in &pe.en_ligne {
            vus.insert(normal(&f.source));
        }
        if pe.fichiers.is_empty() && pe.en_ligne.is_empty() {
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

/// Un abri de sorte `emplacements` (à la main, après une partie, avant un retrait ou un mod… : lots S2 et S3) : ses
/// parties ne sont pas dans le dossier du jeu — jamais pris pour l'abri d'un retrait ni d'une réinstallation (la remise
/// poserait `emplacements/` dans le jeu). Il part chez Firehouse par `emplacements_a_envoyer` (lot S4), avec son
/// propriétaire seulement.
pub fn sorte_emplacements(dossier_abri: &Path) -> bool {
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

/// Rapatrie UN fichier « en ligne seulement » : lu en entier par `r.lecteur` dans `attente` (hors du dossier
/// provisoire de l'abri : une lecture abandonnée au bout du délai ne peut jamais y écrire), relu pour son SHA1 dans le
/// même fil, puis comparé à la copie et posé dans `cible`.
fn rapatrier_un(f: &EnLigne, attente: &Path, cible: &Path, r: &Rapatriement) -> std::result::Result<AMettre, Echec> {
    let (lecteur, source, copie) = (r.lecteur.clone(), f.source.clone(), attente.to_path_buf());
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let lu = lecteur(&source, &copie).and_then(|_| crate::nuage_steam::sha1(&source));
        let _ = tx.send(lu);
    });
    let sha_source = match rx.recv_timeout((r.delai)(f.taille)) {
        Ok(Ok(s)) => s,
        _ => return Err(Echec::OneDrive),
    };
    let sha = crate::nuage_steam::sha1(attente).map_err(|_| Echec::Copie)?;
    if sha != sha_source {
        return Err(Echec::Change);
    }
    if let Some(p) = cible.parent() {
        std::fs::create_dir_all(p).map_err(|_| Echec::Copie)?;
    }
    std::fs::rename(attente, cible).map_err(|_| Echec::Copie)?;
    let taille = std::fs::metadata(cible).map(|m| m.len()).map_err(|_| Echec::Copie)?;
    Ok(AMettre { source: f.source.clone(), relatif: f.relatif.clone(), taille, sha })
}

/// Copie le plan dans un nouvel abri `<abris>/<id>/<quand>/` (lot S2), par un dossier provisoire hors des abris lus.
/// Refusé sans assez de place sur le disque des abris. Un emplacement dont un fichier ne se copie pas (ou change) est
/// retiré de l'abri (son `echec` est posé dans le plan) ; si aucun ne passe, rien n'est fait. Rend le dossier de l'abri.
/// `rapatriement` (seulement après le oui de la personne, et `preparer_rapatriement`) : les fichiers « en ligne
/// seulement » sont téléchargés et copiés aussi ; sinon ils ne sont jamais lus.
pub fn abriter(plan: &mut PlanAbri, abris: &Path, id: i64, quand: &str, profil: &str, rapatriement: Option<&Rapatriement>) -> Resultat<PathBuf> {
    abriter_par(plan, abris, id, quand, profil, DECLENCHEUR_MAIN, rapatriement)
}

/// Le message d'un disque des abris trop plein : ce qu'il faut de place (copie + 1 Go de marge pour Windows), en Mo.
pub fn texte_place(il_faut: u64) -> String {
    format!("Plus assez de place sur le disque des abris (il faut {} Mo) : rien n'est mis à l'abri.", il_faut.div_ceil(1024 * 1024))
}

/// Comme `abriter`, en disant ce qui a fait faire l'abri (`declencheur`, de `DECLENCHEURS` ; inconnu : « main »).
/// La place se vérifie AVANT de rien copier : sans assez de place, rien n'est écrit (pas même un dossier provisoire).
pub fn abriter_par(plan: &mut PlanAbri, abris: &Path, id: i64, quand: &str, profil: &str, declencheur: &str, rapatriement: Option<&Rapatriement>) -> Resultat<PathBuf> {
    abriter_avec_place(plan, abris, id, quand, profil, declencheur, rapatriement, &|p| crate::jeux_pc::place_libre(p))
}

/// `place` : la place libre d'un disque (remplacée dans les tests).
#[allow(clippy::too_many_arguments)]
pub(crate) fn abriter_avec_place(
    plan: &mut PlanAbri,
    abris: &Path,
    id: i64,
    quand: &str,
    profil: &str,
    declencheur: &str,
    rapatriement: Option<&Rapatriement>,
    place: &dyn Fn(&Path) -> Option<u64>,
) -> Resultat<PathBuf> {
    let declencheur = if declencheur_connu(declencheur) { declencheur } else { DECLENCHEUR_MAIN };
    let fin = abris.join(id.to_string()).join(quand);
    if fin.exists() || fichier_fiche(&fin).is_some_and(|f| f.exists()) {
        return Err(Erreur::Refus("Un abri de ce jeu vient d'être fait : réessaie dans une seconde.".into()));
    }
    let a_copier = plan.octets() + if rapatriement.is_some() { plan.octets_en_ligne() } else { 0 };
    let il_faut = a_copier.saturating_add(crate::jeux_pc::MARGE_LIBRE);
    // Le disque du dossier des abris, ou du premier parent qui existe (rien n'est créé avant d'avoir vérifié).
    let disque = abris.ancestors().find(|p| p.exists()).unwrap_or(abris);
    if place(disque).is_some_and(|l| l < il_faut) {
        return Err(Erreur::Refus(texte_place(il_faut)));
    }
    std::fs::create_dir_all(abris)?;
    let tmp = provisoire(abris, id, quand);
    let r = (|| -> Resultat<()> {
        std::fs::create_dir_all(&tmp)?;
        let total = if rapatriement.is_some() { plan.fichiers_en_ligne() } else { 0 };
        let (mut faits, mut essais, mut rapatries) = (0usize, 0usize, 0usize);
        let attente = tmp.with_file_name(format!("{}-nuage", tmp.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()));
        for e in plan.emplacements.iter_mut().filter(|e| e.echec.is_none()) {
            let mut r = copier_emplacement(e, &tmp);
            let mut de_cet_endroit = 0usize;
            if let (Ok(()), Some(rap)) = (&r, rapatriement) {
                std::fs::create_dir_all(&attente)?;
                for f in std::mem::take(&mut e.en_ligne) {
                    (rap.progres)(faits, total);
                    essais += 1;
                    let essai = rapatrier_un(&f, &attente.join(essais.to_string()), &tmp.join(&f.relatif), rap);
                    faits += 1;
                    match essai {
                        Ok(a) => {
                            e.octets += a.taille;
                            e.fichiers.push(a);
                            de_cet_endroit += 1;
                        }
                        Err(x) => {
                            r = Err(x);
                            break;
                        }
                    }
                }
            }
            if let Err(x) = r {
                // Le dossier de CET emplacement dans le provisoire (le nôtre) : retiré, les autres restent.
                let _ = std::fs::remove_dir_all(tmp.join(PREFIXE.trim_end_matches('/')).join(&e.abrite.cle));
                e.echec = Some(x);
                e.en_ligne.clear();
            } else {
                rapatries += de_cet_endroit;
            }
        }
        if let Some(rap) = rapatriement {
            (rap.progres)(faits, total);
        }
        // Seulement le nôtre (son nom le dit) ; une lecture abandonnée peut le tenir encore : laissé alors.
        let _ = std::fs::remove_dir_all(&attente);
        plan.en_ligne_seulement = plan.fichiers_en_ligne();
        plan.rapatries = rapatries;
        if plan.fichiers() == 0 {
            return Err(Erreur::Refus("Aucune partie n'a pu être copiée : rien n'est mis à l'abri.".into()));
        }
        let fichiers: Vec<AMettre> = plan.bons().flat_map(|e| e.fichiers.iter().cloned()).collect();
        // Lot S4 : chaque emplacement dit combien de ses fichiers sont restés dans le nuage (rapatriés : aucun).
        let abrites = plan
            .bons()
            .filter(|e| !e.fichiers.is_empty())
            .map(|e| EmplacementAbrite { en_ligne: Some(e.en_ligne.len()), ..e.abrite.clone() })
            .collect();
        let mut fiche = FicheAbri::nouvelle(SORTE_EMPLACEMENTS, profil, abrites, &fichiers, plan.en_ligne_seulement, plan.echecs());
        fiche.declencheur = declencheur.into();
        fiche.boutique = plan.boutique.clone();
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

/// Des fichiers de cet emplacement n'étaient que dans OneDrive dans cet abri : le détail de l'emplacement le dit (lot
/// S4) ; une fiche plus ancienne ne le disait que pour tout l'abri (`en_ligne_seulement`) — tous ses emplacements
/// comptent alors comme partiels (rien n'est perdu à réunir).
fn partiel(e: &EmplacementAbrite, f: &FicheAbri) -> bool {
    match e.en_ligne {
        Some(n) => n > 0,
        None => f.en_ligne_seulement > 0,
    }
}

/// Les parties `emplacements/` de CE profil à envoyer avec sa sauvegarde (lot S4, Seb 23/11 : TOUS les abris de sorte
/// `emplacements` — à la main, après une partie, avant un retrait ou un mod — et ceux des jeux Steam, `nuage-steam`),
/// jamais celles d'un autre profil. Chaque emplacement (par sa clé) vient du plus récent de SES abris qui le contient :
/// un emplacement absent du dernier abri (fichier verrouillé, hors ligne, trop gros avant un mod) se prend dans le
/// précédent ; un emplacement PARTIEL (des fichiers restés dans OneDrive) est RÉUNI avec ses copies précédentes, le
/// plus récent gagnant fichier par fichier, jusqu'à une copie complète (Seb, 23/11 : une partie effacée par le jeu peut
/// alors revenir à la restauration). `abris` : tous les abris du jeu (dossiers datés, de tous les numéros du jeu).
/// Rend les fichiers (source, chemin dans l'abri) et la fiche fusionnée (sans aucun chemin du PC), DÉTERMINISTE : triée
/// par clé et par chemin, sans aucune date du moment (rien ne change, rien n'est renvoyé).
pub fn emplacements_a_envoyer(abris: &[(u64, PathBuf)], profil: &str) -> (Vec<(PathBuf, String)>, Option<FicheAbri>) {
    let mut a: Vec<(u64, &PathBuf, FicheAbri)> = abris
        .iter()
        .filter_map(|(q, d)| lire_fiche(d).map(|f| (*q, d, f)))
        .filter(|(_, _, f)| (f.sorte == SORTE_NUAGE_STEAM || f.sorte == SORTE_EMPLACEMENTS) && f.profil == profil)
        .collect();
    // Du plus récent au plus ancien ; deux abris de la même seconde (deux dossiers d'abris) : toujours le même ordre.
    a.sort_by(|x, y| y.0.cmp(&x.0).then_with(|| x.1.cmp(y.1)));
    let cles: BTreeSet<String> = a.iter().flat_map(|(_, _, f)| f.emplacements.iter().map(|e| e.cle.clone())).collect();
    let mut pris: Vec<EmplacementAbrite> = Vec::new();
    let mut fichiers: Vec<FichierAbrite> = Vec::new();
    let mut sources: Vec<(PathBuf, String)> = Vec::new();
    for cle in cles {
        let prefixe = format!("{PREFIXE}{cle}/");
        let mut base: Option<EmplacementAbrite> = None;
        // Par chemin sans tenir compte des majuscules (comme Windows) : le plus récent est pris le premier.
        let mut siens: std::collections::BTreeMap<String, (PathBuf, FichierAbrite)> = std::collections::BTreeMap::new();
        for (q, d, f) in &a {
            let Some(e) = f.emplacements.iter().find(|e| e.cle == cle) else { continue };
            let presents: Vec<&FichierAbrite> = f.fichiers.iter().filter(|x| x.relatif.starts_with(&prefixe) && d.join(&x.relatif).is_file()).collect();
            if presents.is_empty() {
                continue;
            }
            for x in presents {
                siens.entry(x.relatif.to_lowercase()).or_insert_with(|| (d.join(&x.relatif), x.clone()));
            }
            let p = partiel(e, f);
            if base.is_none() {
                base = Some(EmplacementAbrite { date: q.to_string(), declencheur: f.declencheur.clone(), partiel: p, ..e.clone() });
            }
            // Une copie complète : les plus anciennes n'ajoutent plus rien.
            if !p {
                break;
            }
        }
        let Some(base) = base else { continue };
        pris.push(base);
        for (_, (source, x)) in siens {
            sources.push((source, x.relatif.clone()));
            fichiers.push(x);
        }
    }
    if pris.is_empty() {
        return (vec![], None);
    }
    let octets = fichiers.iter().map(|x| x.taille).sum();
    let fiche = FicheAbri {
        version_fiche: VERSION_FICHE,
        sorte: SORTE_EMPLACEMENTS.into(),
        profil: profil.into(),
        complet: !pris.iter().any(|e| e.partiel),
        version: None,
        emplacements: pris,
        fichiers,
        octets,
        en_ligne_seulement: 0,
        echecs: 0,
        declencheur: declencheur_main(),
        // La boutique notée par l'abri le plus récent qui la connaît.
        boutique: a.iter().find_map(|(_, _, f)| f.boutique.clone()),
    };
    (sources, Some(fiche))
}

/// Le profil propriétaire d'un abri, quand sa fiche (lot S2 et après) le dit.
pub fn proprietaire(dossier_abri: &Path) -> Option<String> {
    lire_fiche(dossier_abri).map(|f| f.profil).filter(|p| !p.is_empty())
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
    /// Ce qui a fait faire un abri de sorte `emplacements` (lot S3 : `DECLENCHEURS`) ; vide pour les autres sortes.
    pub declencheur: String,
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
                let declencheur = if f.sorte == SORTE_EMPLACEMENTS { f.declencheur.clone() } else { String::new() };
                l.push((q, AbriVu { date, fichiers: f.fichiers.len(), octets: f.octets, sorte: f.sorte, a_moi: true, echecs: f.echecs, en_ligne_seulement: f.en_ligne_seulement, sans_origine: 0, declencheur }));
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
            l.push((q, AbriVu { date, fichiers: parties.len(), octets, sorte: sorte.into(), a_moi: false, echecs: 0, en_ligne_seulement: 0, sans_origine, declencheur: String::new() }));
        }
    }
    l.sort_by(|a, b| b.0.cmp(&a.0));
    l.into_iter().map(|(_, a)| a).collect()
}

// --- Lot S3 (23/11) : l'abri AUTOMATIQUE après une partie — le relevé léger, l'attente du calme, le repère « abri à
// faire ». Jamais un fichier lu pour attendre (tailles et dates seulement), jamais un SHA1 en boucle. ---

/// Le relevé LÉGER d'emplacements de parties : la taille et la date de chaque fichier, par ses métadonnées seulement
/// (`symlink_metadata`, jamais un lien suivi) — aucun fichier n'est ouvert, un fichier OneDrive « en ligne seulement »
/// n'est jamais téléchargé (il est compté à part).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Releve {
    /// (chemin `normal`, taille, date) de chaque fichier, triés : deux relevés égaux = rien n'a bougé.
    pub fichiers: Vec<(String, u64, Option<std::time::SystemTime>)>,
    /// Les octets des fichiers sur ce PC (sans les « en ligne seulement »).
    pub octets: u64,
    /// Les fichiers « en ligne seulement ».
    pub en_ligne: usize,
    pub octets_en_ligne: u64,
    /// La plus récente écriture d'un fichier sur ce PC.
    pub derniere_ecriture: Option<std::time::SystemTime>,
    /// Les emplacements présents qui n'ont pas pu être parcourus (illisibles, trop grands pour un dossier de parties) :
    /// jamais pris pour « aucune partie » (I2).
    pub arrets: usize,
}

impl Releve {
    /// Les fichiers sur ce PC (pas seulement dans le nuage).
    pub fn sur_le_pc(&self) -> usize {
        self.fichiers.len() - self.en_ligne
    }
}

/// Relève ces emplacements (mêmes règles de parcours que `planifier` : dossier entier ou fichiers d'un filtre ; un
/// emplacement illisible ou trop grand pour un dossier de parties est sauté, `planifier` le dira).
pub fn relever(lieux: &[Emplacement]) -> Releve {
    let mut vus: std::collections::BTreeMap<String, (u64, Option<std::time::SystemTime>, bool)> = Default::default();
    let mut arrets = 0;
    for e in lieux.iter().filter(|e| e.dossier.is_dir()) {
        let lus: Option<Vec<String>> = match &e.filtre {
            None => crate::nuage_steam::parcourir(&e.dossier).ok(),
            Some(f) => std::fs::read_dir(&e.dossier)
                .map(|l| l.flatten().filter(|x| x.file_type().is_ok_and(|t| t.is_file())).map(|x| x.file_name().to_string_lossy().to_string()).filter(|n| crate::nuage_steam::repond(n, f)).collect())
                .ok(),
        };
        let Some(relatifs) = lus else {
            arrets += 1;
            continue;
        };
        for r in relatifs {
            let p = e.dossier.join(&r);
            let Ok(m) = std::fs::symlink_metadata(&p) else { continue };
            if !m.is_file() {
                continue;
            }
            vus.insert(normal(&p), (m.len(), m.modified().ok(), crate::nuage_steam::en_ligne_seulement(&m)));
        }
    }
    let mut r = Releve { arrets, ..Default::default() };
    for (cle, (taille, date, en_ligne)) in vus {
        if en_ligne {
            r.en_ligne += 1;
            r.octets_en_ligne += taille;
        } else {
            r.octets += taille;
            if let Some(t) = date {
                r.derniere_ecriture = Some(r.derniere_ecriture.map_or(t, |x| x.max(t)));
            }
        }
        r.fichiers.push((cle, taille, date));
    }
    r
}

/// Les délais de l'attente du calme (remplacés dans les tests).
#[derive(Debug, Clone, Copy)]
pub struct Delais {
    /// Entre deux relevés.
    pub pas: std::time::Duration,
    /// Aucune écriture depuis au moins ce temps.
    pub calme: std::time::Duration,
    /// Au-delà, on abandonne (Seb, 23/11 : 10 minutes au plus) ; le repère « abri à faire » reste.
    pub plafond: std::time::Duration,
}

/// Après une partie : un relevé toutes les 8 s, 60 s sans écriture, 10 minutes au plus.
pub const DELAIS_APRES_PARTIE: Delais = Delais { pas: std::time::Duration::from_secs(8), calme: std::time::Duration::from_secs(60), plafond: std::time::Duration::from_secs(600) };

/// Comment finit l'attente du calme.
#[derive(Debug, Clone, PartialEq)]
pub enum FinAttente {
    /// Deux relevés identiques, aucune écriture depuis `calme`, aucun programme du dossier du jeu : le dernier relevé.
    Calme(Releve),
    /// Le plafond est atteint (le jeu écrit encore, un programme de son dossier tourne toujours).
    Depassee,
    /// Le jeu est repris (une nouvelle partie, une installation…) : l'abri attendra la fin de celle-là.
    Reprise,
}

/// Attend le CALME après une partie : `processus` (un programme du dossier du jeu tourne-t-il ?) et le relevé se font
/// hors du fil courant ; `repris` (le jeu est-il de nouveau occupé ?) se lit à chaque tour.
pub async fn attendre_calme(
    d: Delais,
    lieux: Vec<Emplacement>,
    processus: std::sync::Arc<dyn Fn() -> bool + Send + Sync>,
    repris: impl Fn() -> bool,
) -> FinAttente {
    let debut = std::time::Instant::now();
    let mut avant: Option<Releve> = None;
    loop {
        if repris() {
            return FinAttente::Reprise;
        }
        let (l, p) = (lieux.clone(), processus.clone());
        let (tourne, releve) = match tokio::task::spawn_blocking(move || (p(), relever(&l))).await {
            Ok(x) => x,
            Err(_) => return FinAttente::Depassee,
        };
        if repris() {
            return FinAttente::Reprise;
        }
        // Une date d'écriture dans le futur (horloge changée, M7) ne dit rien : comptée comme « pas d'écriture » — les
        // deux relevés identiques suffisent alors.
        let ecrit_depuis = releve.derniere_ecriture.and_then(|t| std::time::SystemTime::now().duration_since(t).ok());
        let calme = !tourne && avant.as_ref() == Some(&releve) && ecrit_depuis.is_none_or(|e| e >= d.calme);
        if calme {
            return FinAttente::Calme(releve);
        }
        if debut.elapsed() >= d.plafond {
            return FinAttente::Depassee;
        }
        avant = Some(releve);
        tokio::time::sleep(d.pas).await;
    }
}

/// Le repère « abri à faire » d'un profil (dans SES données) : l'identifiant de chaque jeu et la date, rien d'autre
/// (jamais un titre ni un chemin).
pub const NOM_A_FAIRE: &str = "abris-a-faire.json";

fn lire_a_faire(dossier_profil: &Path) -> std::collections::BTreeMap<String, String> {
    std::fs::read(dossier_profil.join(NOM_A_FAIRE)).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default()
}

fn ecrire_a_faire(dossier_profil: &Path, l: &std::collections::BTreeMap<String, String>) -> Resultat<()> {
    let f = dossier_profil.join(NOM_A_FAIRE);
    if l.is_empty() {
        if f.exists() {
            std::fs::remove_file(&f)?;
        }
        return Ok(());
    }
    std::fs::create_dir_all(dossier_profil)?;
    let provisoire = dossier_profil.join(format!("{NOM_A_FAIRE}.provisoire"));
    std::fs::write(&provisoire, serde_json::to_vec_pretty(l).unwrap_or_default())?;
    std::fs::rename(&provisoire, &f)?;
    Ok(())
}

/// Les repères sont lus et écrits un à la fois (deux fins de partie en même temps).
static VERROU_A_FAIRE: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Note « abri à faire » pour ce jeu (avant d'attendre le calme).
pub fn noter_a_faire(dossier_profil: &Path, id: i64, quand: &str) -> Resultat<()> {
    let _v = VERROU_A_FAIRE.lock().unwrap_or_else(|e| e.into_inner());
    let mut l = lire_a_faire(dossier_profil);
    l.insert(id.to_string(), quand.to_string());
    ecrire_a_faire(dossier_profil, &l)
}

/// Le repère part : l'abri est fait, ou il n'y avait rien de neuf.
pub fn retirer_a_faire(dossier_profil: &Path, id: i64) -> Resultat<()> {
    let _v = VERROU_A_FAIRE.lock().unwrap_or_else(|e| e.into_inner());
    let mut l = lire_a_faire(dossier_profil);
    if l.remove(&id.to_string()).is_some() {
        ecrire_a_faire(dossier_profil, &l)?;
    }
    Ok(())
}

/// Les jeux dont l'abri reste à faire pour ce profil.
pub fn a_faire(dossier_profil: &Path) -> Vec<i64> {
    let _v = VERROU_A_FAIRE.lock().unwrap_or_else(|e| e.into_inner());
    lire_a_faire(dossier_profil).keys().filter_map(|k| k.parse().ok()).collect()
}

/// M6 : le dernier état d'un abri après une partie qui n'a pas pu tout copier, gardé pour la fiche du jeu (survit au
/// redémarrage) : l'état, la date, des nombres — jamais un titre ni un chemin.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EtatNote {
    pub etat: String,
    pub date: String,
    #[serde(default)]
    pub en_ligne: usize,
    #[serde(default)]
    pub octets_en_ligne: u64,
}

/// Le fichier des derniers états (dans les données du profil).
pub const NOM_ETATS: &str = "abris-auto-etats.json";

fn lire_etats(dossier_profil: &Path) -> std::collections::BTreeMap<String, EtatNote> {
    std::fs::read(dossier_profil.join(NOM_ETATS)).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default()
}

fn ecrire_etats(dossier_profil: &Path, l: &std::collections::BTreeMap<String, EtatNote>) -> Resultat<()> {
    let f = dossier_profil.join(NOM_ETATS);
    if l.is_empty() {
        if f.exists() {
            std::fs::remove_file(&f)?;
        }
        return Ok(());
    }
    std::fs::create_dir_all(dossier_profil)?;
    let provisoire = dossier_profil.join(format!("{NOM_ETATS}.provisoire"));
    std::fs::write(&provisoire, serde_json::to_vec_pretty(l).unwrap_or_default())?;
    std::fs::rename(&provisoire, &f)?;
    Ok(())
}

/// Garde le dernier état (problème) de l'abri automatique de ce jeu.
pub fn noter_etat_auto(dossier_profil: &Path, id: i64, n: &EtatNote) -> Resultat<()> {
    let _v = VERROU_A_FAIRE.lock().unwrap_or_else(|e| e.into_inner());
    let mut l = lire_etats(dossier_profil);
    l.insert(id.to_string(), n.clone());
    ecrire_etats(dossier_profil, &l)
}

/// L'abri a réussi (ou rien de neuf) : plus rien à dire.
pub fn retirer_etat_auto(dossier_profil: &Path, id: i64) -> Resultat<()> {
    let _v = VERROU_A_FAIRE.lock().unwrap_or_else(|e| e.into_inner());
    let mut l = lire_etats(dossier_profil);
    if l.remove(&id.to_string()).is_some() {
        ecrire_etats(dossier_profil, &l)?;
    }
    Ok(())
}

/// Le dernier état gardé de ce jeu.
pub fn etat_auto(dossier_profil: &Path, id: i64) -> Option<EtatNote> {
    let _v = VERROU_A_FAIRE.lock().unwrap_or_else(|e| e.into_inner());
    lire_etats(dossier_profil).remove(&id.to_string())
}

#[cfg(test)]
mod tests_s3 {
    //! Lot S3 : le relevé léger, l'attente du calme (durées courtes), le repère, la place, le déclencheur.
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    const COURT: Delais = Delais { pas: Duration::from_millis(30), calme: Duration::from_millis(400), plafond: Duration::from_millis(1500) };

    fn lieu(dossier: &Path) -> Emplacement {
        Emplacement { dossier: dossier.to_path_buf(), filtre: None, chemin: r"%APPDATA%\Jeu".into(), nature: "dossier".into(), identifiant: None }
    }

    fn vieillir(f: &Path, s: u64) {
        std::fs::OpenOptions::new().write(true).open(f).unwrap().set_modified(std::time::SystemTime::now() - Duration::from_secs(s)).unwrap();
    }

    #[test]
    fn le_releve_ne_lit_que_les_tailles_et_les_dates() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("Slot")).unwrap();
        std::fs::write(d.path().join("Slot").join("a.sav"), b"abc").unwrap();
        std::fs::write(d.path().join("b.sav"), b"de").unwrap();
        let r = relever(&[lieu(d.path()), lieu(&d.path().join("Slot")), lieu(&d.path().join("Absent"))]);
        assert_eq!((r.fichiers.len(), r.octets, r.en_ligne), (2, 5, 0), "chaque fichier une fois");
        assert!(r.derniere_ecriture.is_some());
        assert_eq!(r, relever(&[lieu(d.path())]), "deux relevés sans écriture sont égaux");
        std::fs::write(d.path().join("b.sav"), b"dex").unwrap();
        assert_ne!(r, relever(&[lieu(d.path())]));
    }

    #[tokio::test]
    async fn un_fichier_reecrit_sans_cesse_n_est_jamais_calme_et_l_attente_s_arrete_au_plafond() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("partie.sav");
        std::fs::write(&f, b"0").unwrap();
        let fini = Arc::new(AtomicBool::new(false));
        let (f2, fini2) = (f.clone(), fini.clone());
        let ecrivain = std::thread::spawn(move || {
            let mut n = 0u32;
            while !fini2.load(Ordering::SeqCst) {
                n += 1;
                std::fs::write(&f2, n.to_string()).unwrap();
                std::thread::sleep(Duration::from_millis(100));
            }
        });
        let debut = std::time::Instant::now();
        let r = attendre_calme(COURT, vec![lieu(d.path())], Arc::new(|| false), || false).await;
        fini.store(true, Ordering::SeqCst);
        ecrivain.join().unwrap();
        assert_eq!(r, FinAttente::Depassee);
        assert!(debut.elapsed() >= COURT.plafond, "jamais avant le plafond");
    }

    #[tokio::test]
    async fn le_calme_vient_apres_le_delai_sans_ecriture() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("partie.sav");
        std::fs::write(&f, b"fin").unwrap();
        // Écrit à l'instant : pas calme tout de suite, calme une fois le délai passé (sans réécriture).
        let debut = std::time::Instant::now();
        let r = attendre_calme(COURT, vec![lieu(d.path())], Arc::new(|| false), || false).await;
        assert!(matches!(&r, FinAttente::Calme(x) if x.octets == 3), "{r:?}");
        assert!(debut.elapsed() >= Duration::from_millis(300), "attendu le calme : {:?}", debut.elapsed());
        // Écrit il y a longtemps : calme dès le second relevé.
        vieillir(&f, 3600);
        let debut = std::time::Instant::now();
        assert!(matches!(attendre_calme(COURT, vec![lieu(d.path())], Arc::new(|| false), || false).await, FinAttente::Calme(_)));
        assert!(debut.elapsed() < Duration::from_millis(400));
    }

    #[tokio::test]
    async fn un_programme_du_jeu_reste_ouvert_ou_le_jeu_est_relance_jamais_d_abri() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("partie.sav");
        std::fs::write(&f, b"fin").unwrap();
        vieillir(&f, 3600);
        // Un programme du dossier du jeu tourne toujours : jamais calme, abandon au plafond.
        assert_eq!(attendre_calme(COURT, vec![lieu(d.path())], Arc::new(|| true), || false).await, FinAttente::Depassee);
        // ▶ Jouer pendant l'attente : abandon tout de suite.
        let repris = AtomicBool::new(false);
        let r = attendre_calme(COURT, vec![lieu(d.path())], Arc::new(|| false), || repris.swap(true, Ordering::SeqCst)).await;
        assert_eq!(r, FinAttente::Reprise);
    }

    /// M7 : une date d'écriture dans le futur (horloge changée) ne bloque jamais le calme.
    #[tokio::test]
    async fn une_date_d_ecriture_dans_le_futur_n_empeche_pas_le_calme() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("partie.sav");
        std::fs::write(&f, b"fin").unwrap();
        std::fs::OpenOptions::new().write(true).open(&f).unwrap().set_modified(std::time::SystemTime::now() + Duration::from_secs(86_400)).unwrap();
        assert!(matches!(attendre_calme(COURT, vec![lieu(d.path())], Arc::new(|| false), || false).await, FinAttente::Calme(_)));
    }

    /// I2 : un emplacement présent mais impossible à parcourir est compté à part (jamais « aucune partie »).
    #[test]
    fn un_emplacement_trop_profond_est_compte_comme_un_arret() {
        let d = tempfile::tempdir().unwrap();
        let mut p = d.path().to_path_buf();
        for i in 0..14 {
            p = p.join(format!("n{i}"));
        }
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(p.join("x.sav"), b"x").unwrap();
        let r = relever(&[lieu(d.path())]);
        assert_eq!((r.fichiers.len(), r.arrets), (0, 1));
        assert_eq!(relever(&[lieu(&d.path().join("Absent"))]).arrets, 0, "absent : pas un arrêt");
    }

    #[test]
    fn le_dernier_etat_garde_ne_contient_que_l_etat_la_date_et_des_nombres() {
        let d = tempfile::tempdir().unwrap();
        assert!(etat_auto(d.path(), 900).is_none());
        let n = EtatNote { etat: "trop_gros".into(), date: "1790000000".into(), en_ligne: 0, octets_en_ligne: 0 };
        noter_etat_auto(d.path(), 900, &n).unwrap();
        assert_eq!(etat_auto(d.path(), 900), Some(n));
        retirer_etat_auto(d.path(), 900).unwrap();
        assert!(etat_auto(d.path(), 900).is_none() && !d.path().join(NOM_ETATS).exists());
    }

    #[test]
    fn le_repere_abri_a_faire_ne_garde_que_l_identifiant_et_la_date() {
        let d = tempfile::tempdir().unwrap();
        assert!(a_faire(d.path()).is_empty());
        noter_a_faire(d.path(), 900, "1790000000").unwrap();
        noter_a_faire(d.path(), 12, "1790000001").unwrap();
        assert_eq!(a_faire(d.path()), [12, 900]);
        let brut = std::fs::read_to_string(d.path().join(NOM_A_FAIRE)).unwrap();
        assert!(brut.contains("\"900\": \"1790000000\""), "{brut}");
        retirer_a_faire(d.path(), 900).unwrap();
        assert_eq!(a_faire(d.path()), [12]);
        retirer_a_faire(d.path(), 12).unwrap();
        assert!(!d.path().join(NOM_A_FAIRE).exists(), "plus rien à faire : plus de repère");
    }

    fn plan_simple(d: &Path) -> PlanAbri {
        let saves = d.join("Saves");
        std::fs::create_dir_all(&saves).unwrap();
        std::fs::write(saves.join("a.sav"), b"partie").unwrap();
        vieillir(&saves.join("a.sav"), 3600);
        planifier(&[lieu(&saves)], &Default::default(), "0123456789abcdef0123456789abcdef")
    }

    #[test]
    fn sans_assez_de_place_rien_n_est_ecrit_et_le_motif_dit_combien() {
        let d = tempfile::tempdir().unwrap();
        let mut plan = plan_simple(d.path());
        let abris = d.path().join("Abris");
        let r = abriter_avec_place(&mut plan, &abris, 900, "1790000000", "p1", DECLENCHEUR_APRES_PARTIE, None, &|_| Some(10));
        match r {
            Err(Erreur::Refus(m)) => assert!(m.starts_with("Plus assez de place sur le disque des abris (il faut 1025 Mo)"), "{m}"),
            autre => panic!("{autre:?}"),
        }
        assert!(!abris.exists(), "rien de partiel : pas même le dossier des abris");
    }

    #[test]
    fn un_abri_automatique_garde_la_sorte_emplacements_et_dit_son_declencheur() {
        let d = tempfile::tempdir().unwrap();
        let mut plan = plan_simple(d.path());
        let abris = d.path().join("Abris");
        let fin = abriter_par(&mut plan, &abris, 900, "1790000000", "p1", DECLENCHEUR_AVANT_REINSTALLATION, None).unwrap();
        let fiche = lire_fiche(&fin).unwrap();
        assert_eq!((fiche.sorte.as_str(), fiche.declencheur.as_str()), (SORTE_EMPLACEMENTS, DECLENCHEUR_AVANT_REINSTALLATION));
        // Régression : jamais pris pour un abri du dossier du jeu (la remise d'après réinstallation l'écarte), et ses
        // `emplacements/` sont réservés (jamais posés dans le jeu).
        assert!(sorte_emplacements(&fin));
        assert!(reservees(&fin).emplacements);
        let vu = lister(&[abris.join("900")], "p1", None);
        assert_eq!(vu[0].declencheur, DECLENCHEUR_AVANT_REINSTALLATION);
        // Un déclencheur inconnu : « main » ; une fiche d'avant le lot S3 (sans le champ) : « main ».
        let mut plan = plan_simple(d.path());
        let fin2 = abriter_par(&mut plan, &abris, 900, "1790000001", "p1", "n'importe quoi", None).unwrap();
        assert_eq!(lire_fiche(&fin2).unwrap().declencheur, DECLENCHEUR_MAIN);
        let ancienne: FicheAbri = serde_json::from_str(r#"{"version_fiche":1,"sorte":"emplacements","profil":"p1"}"#).unwrap();
        assert_eq!(ancienne.declencheur, DECLENCHEUR_MAIN);
    }
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
        let fin = abriter(&mut plan, &abris, -7, "1000", "profil-lea", None).unwrap();
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
        assert!(sorte_emplacements(&fin));
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
        abriter(&mut plan, &abris, -7, "1000", "p1", None).unwrap();
        let plan = planifier(&l, &BTreeSet::new());
        assert!(deja_a_l_abri(&sources, "p1", &plan), "rien n'a changé");
        assert!(!deja_a_l_abri(&sources, "p2", &plan), "l'abri d'un autre profil ne compte pas");
        std::fs::write(saves.join("Slot1").join("partie.sav"), b"partie2").unwrap();
        let mut plan = planifier(&l, &BTreeSet::new());
        assert!(!deja_a_l_abri(&sources, "p1", &plan), "une partie a changé");
        abriter(&mut plan, &abris, -7, "1001", "p1", None).unwrap();
        assert!(deja_a_l_abri(&sources, "p1", &planifier(&l, &BTreeSet::new())));
        // Le même instant : jamais écrasé.
        assert!(matches!(abriter(&mut planifier(&l, &BTreeSet::new()), &abris, -7, "1001", "p1", None), Err(Erreur::Refus(_))));
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
            let fin = abriter(&mut plan, &abris, -7, "1000", "p1", None).unwrap();
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
        let fin = abriter(&mut plan, &abris, -7, "1000", "p1", None).unwrap();
        assert_eq!(plan.emplacements[i].echec, Some(Echec::Change));
        assert_eq!(lire_fiche(&fin).unwrap().fichiers.len(), 2);
        // Aucun emplacement ne passe : rien n'est fait (ni abri, ni fiche, ni provisoire).
        let mut plan = planifier(&l, &BTreeSet::new());
        for e in plan.emplacements.iter_mut() {
            for f in e.fichiers.iter_mut() {
                f.sha = "0".repeat(40);
            }
        }
        assert!(matches!(abriter(&mut plan, &abris, -7, "2000", "p1", None), Err(Erreur::Refus(_))));
        assert!(!abris.join("-7").join("2000").exists() && !abris.join("-7").join("2000.json").exists());
        assert!(!abris.join(".frogtend-en-cours").exists());
        // Rien à copier du tout : refusé aussi.
        assert!(abriter(&mut PlanAbri::default(), &abris, -7, "3000", "p1", None).is_err());
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
        let fin = abriter(&mut plan, &d.path().join("Abris"), 5, "1", "p1", None).unwrap();
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
            // Le motif n'a rien sur le PC : rien à copier sans le oui, jamais un échec ; gardé pour proposer de télécharger.
            let motif = petit.emplacements.iter().find(|e| e.dossier == low).unwrap();
            assert_eq!((motif.echec, motif.fichiers.len(), motif.en_ligne.len()), (None, 0, 2));
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
                abrites.push(EmplacementAbrite { cle: cle.to_string(), chemin: format!(r"%APPDATA%\{cle}"), nature: "dossier".into(), filtre: None, identifiant: None, ..Default::default() });
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

    /// Lot S4 : pose un abri (sorte, profil, déclencheur) : (clé, fichier, octets) ; `en_ligne` par emplacement
    /// (`None` : une fiche d'avant le lot S4) et pour tout l'abri.
    #[allow(clippy::too_many_arguments)]
    fn poser_s4(du_jeu: &Path, date: &str, sorte: &str, profil: &str, fichiers: &[(&str, &str, &[u8])], en_ligne: Option<usize>, en_ligne_abri: usize) -> (u64, PathBuf) {
        let dir = du_jeu.join(date);
        let mut mettre = Vec::new();
        let mut lieux: Vec<EmplacementAbrite> = Vec::new();
        for (cle, nom, o) in fichiers {
            let r = format!("emplacements/{cle}/{nom}");
            std::fs::create_dir_all(dir.join("emplacements").join(cle)).unwrap();
            std::fs::write(dir.join(&r), o).unwrap();
            mettre.push(AMettre { source: PathBuf::new(), relatif: r, taille: o.len() as u64, sha: sha_de(o) });
            if !lieux.iter().any(|l| l.cle == *cle) {
                lieux.push(EmplacementAbrite { cle: cle.to_string(), chemin: format!(r"%APPDATA%\{cle}"), nature: "dossier".into(), en_ligne, ..Default::default() });
            }
        }
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(du_jeu.join(format!("{date}.json")), serde_json::to_vec(&FicheAbri::nouvelle(sorte, profil, lieux, &mettre, en_ligne_abri, 0)).unwrap()).unwrap();
        (date.parse::<u64>().unwrap(), dir)
    }

    fn lus(f: &[(PathBuf, String)]) -> Vec<(String, Vec<u8>)> {
        f.iter().map(|(s, r)| (r.clone(), std::fs::read(s).unwrap())).collect()
    }

    /// Lot S4 : les abris de sorte `emplacements` (à la main, après une partie…) partent avec leur propriétaire, mêlés
    /// à ceux d'un jeu Steam par leur clé (le plus récent gagne) ; jamais ceux d'un autre profil ; la fiche envoyée
    /// est de sorte `emplacements`, triée, identique d'un appel à l'autre.
    #[test]
    fn les_abris_emplacements_partent_avec_leur_profil_meles_au_nuage_steam_par_cle() {
        let d = tempfile::tempdir().unwrap();
        let du_jeu = d.path().join("110");
        let a1 = poser_s4(&du_jeu, "100", SORTE_NUAGE_STEAM, "seb", &[("kkkk", "save", b"steam-100")], Some(0), 0);
        let a2 = poser_s4(&du_jeu, "200", SORTE_EMPLACEMENTS, "seb", &[("kkkk", "save", b"main-200"), ("aaaa", "x", b"x-200")], Some(0), 0);
        let a3 = poser_s4(&du_jeu, "300", SORTE_EMPLACEMENTS, "lea", &[("kkkk", "save", b"lea-300")], Some(0), 0);
        let tous = [a3.clone(), a1.clone(), a2.clone()];
        let (f, fiche) = emplacements_a_envoyer(&tous, "seb");
        assert_eq!(lus(&f), [("emplacements/aaaa/x".to_string(), b"x-200".to_vec()), ("emplacements/kkkk/save".to_string(), b"main-200".to_vec())], "triés par clé ; l'abri de Léa ne fait rien sortir");
        let fiche = fiche.unwrap();
        assert_eq!((fiche.sorte.as_str(), fiche.complet), (SORTE_EMPLACEMENTS, true));
        assert_eq!(fiche.emplacements.iter().map(|e| (e.cle.as_str(), e.date.as_str())).collect::<Vec<_>>(), [("aaaa", "200"), ("kkkk", "200")]);
        // Déterministe : même ordre d'entrée ou non, même fiche, octet pour octet.
        let (_, autre) = emplacements_a_envoyer(&[a2.clone(), a1.clone(), a3.clone()], "seb");
        assert_eq!(serde_json::to_vec(&autre.unwrap()).unwrap(), serde_json::to_vec(&fiche).unwrap());
        // Un abri Steam plus récent du même profil, même clé : c'est lui qui part.
        let a4 = poser_s4(&du_jeu, "400", SORTE_NUAGE_STEAM, "seb", &[("kkkk", "save", b"steam-400")], Some(0), 0);
        let (f, _) = emplacements_a_envoyer(&[a1, a2, a3, a4], "seb");
        assert_eq!(lus(&f)[1], ("emplacements/kkkk/save".to_string(), b"steam-400".to_vec()));
        // Léa : seulement les siens.
        let (f, _) = emplacements_a_envoyer(&tous, "lea");
        assert_eq!(lus(&f), [("emplacements/kkkk/save".to_string(), b"lea-300".to_vec())]);
    }

    /// Seb (23/11) : un emplacement dont des fichiers ne sont que dans OneDrive garde leurs anciennes copies (réunion
    /// avec la copie précédente, le plus récent gagnant fichier par fichier, `partiel`) ; une fiche d'avant le lot S4
    /// (sans le détail par emplacement) qui dit des fichiers en ligne : même réunion.
    #[test]
    fn un_emplacement_partiel_onedrive_garde_les_anciennes_copies() {
        let d = tempfile::tempdir().unwrap();
        let du_jeu = d.path().join("110");
        let a0 = poser_s4(&du_jeu, "50", SORTE_EMPLACEMENTS, "seb", &[("kkkk", "vieux.sav", b"v-50")], Some(0), 0);
        let a1 = poser_s4(&du_jeu, "100", SORTE_EMPLACEMENTS, "seb", &[("kkkk", "a.sav", b"a-100"), ("kkkk", "b.sav", b"b-100")], Some(0), 0);
        // b.sav n'est plus que dans OneDrive : pas copié cette fois.
        let a2 = poser_s4(&du_jeu, "200", SORTE_EMPLACEMENTS, "seb", &[("kkkk", "a.sav", b"a-200")], Some(1), 1);
        let (f, fiche) = emplacements_a_envoyer(&[a0.clone(), a1.clone(), a2.clone()], "seb");
        assert_eq!(lus(&f), [("emplacements/kkkk/a.sav".to_string(), b"a-200".to_vec()), ("emplacements/kkkk/b.sav".to_string(), b"b-100".to_vec())], "réuni jusqu'à la copie complète (100), pas au-delà");
        let fiche = fiche.unwrap();
        assert!(fiche.emplacements[0].partiel && !fiche.complet);
        assert_eq!(fiche.emplacements[0].en_ligne, Some(1));
        // Une copie complète plus récente : plus rien de réuni.
        let a3 = poser_s4(&du_jeu, "300", SORTE_EMPLACEMENTS, "seb", &[("kkkk", "a.sav", b"a-300")], Some(0), 0);
        let (f, fiche) = emplacements_a_envoyer(&[a0.clone(), a1.clone(), a2.clone(), a3], "seb");
        assert_eq!(lus(&f), [("emplacements/kkkk/a.sav".to_string(), b"a-300".to_vec())]);
        assert!(!fiche.unwrap().emplacements[0].partiel);
        // Une fiche ANCIENNE (sans détail) qui disait 1 fichier en ligne : tous ses emplacements sont réunis.
        let du_vieux = d.path().join("111");
        let v1 = poser_s4(&du_vieux, "100", SORTE_NUAGE_STEAM, "seb", &[("kkkk", "a.sav", b"a-100"), ("kkkk", "b.sav", b"b-100")], None, 0);
        let v2 = poser_s4(&du_vieux, "200", SORTE_NUAGE_STEAM, "seb", &[("kkkk", "a.sav", b"a-200")], None, 1);
        let (f, fiche) = emplacements_a_envoyer(&[v1, v2], "seb");
        assert_eq!(lus(&f).len(), 2);
        assert!(fiche.unwrap().emplacements[0].partiel);
        // Un abri fait par le plan : chaque emplacement dit ses fichiers restés en ligne (Some(0) sinon).
        assert!(serde_json::to_string(&FicheAbri::nouvelle(SORTE_EMPLACEMENTS, "p", vec![EmplacementAbrite::default()], &[], 0, 0)).unwrap().find("\"partiel\"").is_none(), "rien d'ajouté aux fiches des abris");
    }

    /// M5 (lot S4) : la boutique du jeu est notée dans la fiche dès l'abri, et la fiche envoyée la reprend (un jeu
    /// retiré ensuite n'est plus dans le registre).
    #[test]
    fn la_boutique_est_notee_des_l_abri_et_reprise_a_l_envoi() {
        let (d, _, _, l) = pc();
        let abris = d.path().join("Abris");
        let mut plan = planifier(&l, &BTreeSet::new());
        plan.boutique = Some(BoutiqueDuJeu { source: "gog".into(), cle: "1207658924".into() });
        let fin = abriter(&mut plan, &abris, -7, "1000", "p1", None).unwrap();
        assert_eq!(lire_fiche(&fin).unwrap().boutique, plan.boutique);
        let (_, fiche) = emplacements_a_envoyer(&[(1000, fin)], "p1");
        assert_eq!(fiche.unwrap().boutique, Some(BoutiqueDuJeu { source: "gog".into(), cle: "1207658924".into() }));
    }

    #[test]
    fn la_liste_ne_montre_que_les_abris_du_profil_et_les_anciens_tels_quels() {
        let (d, _, _, l) = pc();
        let abris = d.path().join("Abris");
        let du_jeu = abris.join("-7");
        abriter(&mut planifier(&l, &BTreeSet::new()), &abris, -7, "1000", "p1", None).unwrap();
        abriter(&mut planifier(&l, &BTreeSet::new()), &abris, -7, "1001", "p2", None).unwrap();
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
        assert!(!sorte_emplacements(&du_jeu.join("900")) && !sorte_emplacements(&du_jeu.join("800")) && sorte_emplacements(&du_jeu.join("1000")));
    }

    /// Pose (ou retire) l'attribut « hors ligne » : le fichier compte alors « en ligne seulement » (il reste lisible
    /// dans le test, comme un fichier OneDrive que la lecture ferait télécharger). `false` : impossible ici.
    #[cfg(windows)]
    fn en_ligne(f: &Path, oui: bool) -> bool {
        use windows::Win32::Storage::FileSystem::{SetFileAttributesW, FILE_ATTRIBUTE_ARCHIVE, FILE_ATTRIBUTE_OFFLINE};
        let h = windows::core::HSTRING::from(f.as_os_str());
        unsafe { SetFileAttributesW(&h, if oui { FILE_ATTRIBUTE_OFFLINE | FILE_ATTRIBUTE_ARCHIVE } else { FILE_ATTRIBUTE_ARCHIVE }) }.is_ok()
    }

    fn outils(lecteur: std::sync::Arc<dyn Fn(&Path, &Path) -> std::io::Result<()> + Send + Sync>, delai: std::time::Duration) -> (std::sync::Arc<dyn Fn(&Path, &Path) -> std::io::Result<()> + Send + Sync>, impl Fn(u64) -> std::time::Duration + Sync, std::sync::Arc<std::sync::Mutex<Vec<(usize, usize)>>>) {
        (lecteur, move |_| delai, std::sync::Arc::new(std::sync::Mutex::new(Vec::new())))
    }

    /// Seb 22/11 (Age of Empires IV, 9 fichiers seulement dans OneDrive) : rien n'est lu sans le oui ; avec lui, chaque
    /// fichier est lu en entier pendant la copie, vérifié au SHA1, et la progression est dite.
    #[test]
    #[cfg(windows)]
    fn les_fichiers_en_ligne_ne_sont_lus_qu_apres_le_oui_puis_rapatries() {
        let (d, saves, low, l) = pc();
        if !en_ligne(&low.join("a.save"), true) {
            eprintln!("attribut « hors ligne » impossible à poser ici : test sauté");
            return;
        }
        // Rien n'est ouvert pendant le plan : le fichier tenu sans partage ne fait échouer personne.
        {
            use std::os::windows::fs::OpenOptionsExt;
            let _tenu = std::fs::OpenOptions::new().read(true).share_mode(0).open(low.join("a.save")).unwrap();
            let plan = planifier(&l, &BTreeSet::new());
            assert_eq!((plan.echecs(), plan.fichiers_en_ligne(), plan.octets_en_ligne()), (0, 1, 2), "jamais lu pour le compter");
            // Sans le oui : copié sans lui, compté « en ligne seulement ».
            let mut sans = plan.clone();
            let fin = abriter(&mut sans, &d.path().join("Abris"), -7, "1000", "p1", None).unwrap();
            let f = lire_fiche(&fin).unwrap();
            assert_eq!((f.fichiers.len(), f.en_ligne_seulement, f.complet, f.emplacements.len()), (2, 1, false, 1), "l'endroit tout en ligne n'est pas dans la fiche");
            // Lot S4 : chaque emplacement de la fiche dit ses fichiers restés dans le nuage (ici aucun).
            assert!(f.emplacements.iter().all(|e| e.en_ligne == Some(0)), "{:?}", f.emplacements);
        }
        // Avec le oui : lu en entier (ici, la vraie lecture), copié, vérifié, compté.
        let (lecteur, delai, vus) = outils(std::sync::Arc::new(|a: &Path, b: &Path| lire_en_entier(a, b)), std::time::Duration::from_secs(30));
        let v = vus.clone();
        let progres = move |a: usize, b: usize| v.lock().unwrap().push((a, b));
        let rap = Rapatriement { lecteur, delai: &delai, progres: &progres };
        let mut plan = planifier(&l, &BTreeSet::new());
        plan.preparer_rapatriement(crate::nuage_steam::EMPLACEMENT_MAX);
        let fin = abriter(&mut plan, &d.path().join("Abris"), -7, "2000", "p1", Some(&rap)).unwrap();
        let cle = cle_chemin(r"%USERPROFILE%\AppData\LocalLow\Studio\*.save", "motif");
        assert_eq!(std::fs::read(fin.join("emplacements").join(&cle).join("a.save")).unwrap(), b"aa");
        let f = lire_fiche(&fin).unwrap();
        assert_eq!((f.fichiers.len(), f.en_ligne_seulement, f.complet, plan.rapatries), (3, 0, true, 1));
        assert!(f.emplacements.iter().all(|e| e.en_ligne == Some(0)), "rapatriés : plus rien en ligne");
        assert_eq!(f.fichiers.iter().find(|x| x.relatif.ends_with("a.save")).unwrap().sha, sha_de(b"aa"));
        assert_eq!(*vus.lock().unwrap(), [(0, 1), (1, 1)], "« Téléchargement depuis OneDrive… N/M »");
        assert!(!d.path().join("Abris").join(".frogtend-en-cours").exists(), "rien de provisoire laissé");
        let _ = (saves, en_ligne(&low.join("a.save"), false));
    }

    /// OneDrive pas connecté, ou trop long : CET endroit échoue (raison claire), les autres passent ; jamais bloqué.
    #[test]
    #[cfg(windows)]
    fn un_telechargement_rate_ou_trop_long_fait_echouer_son_endroit_seulement() {
        let (d, _saves, low, l) = pc();
        if !en_ligne(&low.join("a.save"), true) {
            eprintln!("attribut « hors ligne » impossible à poser ici : test sauté");
            return;
        }
        let rien = |_: usize, _: usize| {};
        for (nom, lecteur) in [
            ("refus", std::sync::Arc::new(|_: &Path, _: &Path| Err(std::io::Error::other("OneDrive n'est pas connecté"))) as std::sync::Arc<dyn Fn(&Path, &Path) -> std::io::Result<()> + Send + Sync>),
            ("trop long", std::sync::Arc::new(|_: &Path, _: &Path| {
                std::thread::sleep(std::time::Duration::from_secs(3));
                Ok(())
            })),
        ] {
            let delai = |_: u64| std::time::Duration::from_millis(200);
            let rap = Rapatriement { lecteur, delai: &delai, progres: &rien };
            let mut plan = planifier(&l, &BTreeSet::new());
            plan.preparer_rapatriement(crate::nuage_steam::EMPLACEMENT_MAX);
            let debut = std::time::Instant::now();
            let fin = abriter(&mut plan, &d.path().join("Abris"), -7, if nom == "refus" { "1000" } else { "1001" }, "p1", Some(&rap)).unwrap();
            assert!(debut.elapsed() < std::time::Duration::from_secs(2), "{nom} : jamais bloqué");
            let e = plan.emplacements.iter().find(|e| e.dossier == low).unwrap();
            assert_eq!(e.echec, Some(Echec::OneDrive), "{nom}");
            assert!(Echec::OneDrive.texte().contains("vérifie que OneDrive est connecté"));
            let f = lire_fiche(&fin).unwrap();
            assert_eq!((f.fichiers.len(), f.echecs), (2, 1), "{nom} : le dossier passe");
            assert!(!fin.join("emplacements").join(cle_chemin(r"%USERPROFILE%\AppData\LocalLow\Studio\*.save", "motif")).exists());
        }
        let _ = en_ligne(&low.join("a.save"), false);
    }

    /// Le plafond de 2 Go par endroit compte les fichiers à télécharger dès qu'on choisit de les rapatrier.
    #[test]
    #[cfg(windows)]
    fn le_plafond_compte_les_fichiers_a_telecharger() {
        let (_d, _saves, low, l) = pc();
        std::fs::write(low.join("b.save"), vec![0u8; 64]).unwrap();
        if !en_ligne(&low.join("b.save"), true) {
            eprintln!("attribut « hors ligne » impossible à poser ici : test sauté");
            return;
        }
        let mut plan = planifier_jusqua(&l, &BTreeSet::new(), SEL, 20);
        assert_eq!(plan.echecs(), 0, "sans rapatrier : le fichier en ligne ne compte pas");
        plan.preparer_rapatriement(20);
        let e = plan.emplacements.iter().find(|e| e.dossier == low).unwrap();
        assert_eq!((e.echec, plan.fichiers_en_ligne()), (Some(Echec::TropGros), 0));
        let _ = en_ligne(&low.join("b.save"), false);
    }
}
