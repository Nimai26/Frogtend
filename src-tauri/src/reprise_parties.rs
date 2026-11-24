//! Lot S5 (24/11) : remettre à leur place, sur CE PC, les parties des jeux PC rangées HORS du dossier du jeu, d'après
//! la sauvegarde de Firehouse (`jeux/<id>/emplacements/<clé>/…`, avec sa fiche `jeux/<id>/emplacements/fiche.json`).
//!
//! Règles (décisions de Seb et des experts, 24/11) :
//! - le chemin NON résolu de la fiche est résolu sur CE PC (`nuage_steam::candidats`) ; un `<identifiant …>` n'est
//!   retrouvé que par son empreinte salée (`abri_parties::empreinte_identifiant`), parmi les dossiers qui EXISTENT —
//!   jamais deviné : sinon la partie attend ;
//! - absent → posé (copie provisoire relue, puis renommée) ; identique → rien ; DIFFÉRENT → conflit, jamais écrasé, et
//!   rien de cet emplacement n'est posé (jamais un mélange de deux parties) : la personne choisit ;
//! - jeu non installé, dossier non résolu, identifiant non retrouvé → en attente (posé à la fin de son installation) ;
//!   jeu en cours → refusé ; emplacement `partiel` → dit ; `nuage-steam/` → jamais restauré (Steam s'en charge) ;
//!   seulement les jeux visibles du profil (un jeu caché n'existe pas, pas même en nombre) ;
//! - « Reprendre celle de Firehouse » : la partie de ce PC est d'abord mise à l'abri (abri complet de sorte
//!   `emplacements`, relu identique), puis remplacée, puis la partie reprise est mise à l'abri à son tour (la sauvegarde
//!   suivante ne renvoie jamais l'ancienne par-dessus) ;
//! - ce qui est réglé (posé, gardé, remplacé) est noté dans `restauration/<profil>/emplacements-reglees.json` avec la
//!   signature de la copie : plus jamais retraité tant qu'elle ne change pas. Rien n'est effacé de la zone d'attente ;
//! - le SEL des empreintes est relu chez Firehouse (`_profil/document/abris-sel.json`) et rangé seulement s'il manque ;
//!   jamais au journal, dans une erreur ni dans un message ;
//! - journal : des numéros de jeu et des nombres, jamais un chemin, un titre ou un pseudo.

use crate::abri_parties::{self as ap, EtatSel, FicheAbri};
use crate::erreurs::{Erreur, Resultat};
use crate::firehouse::Client;
use crate::noyau::{Noyau, Session};
use crate::nuage_steam::{Contexte, Emplacement};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Le registre des emplacements déjà réglés (dans la zone de restauration du profil).
pub const REGLEES: &str = "emplacements-reglees.json";
/// D'où viennent les parties en attente de chaque jeu (profil et PC de la sauvegarde, sa date).
pub const ORIGINES: &str = "emplacements-origines.json";

/// Pourquoi une partie attend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Attente {
    /// Le jeu n'est pas installé sur ce PC : posée à la fin de son installation.
    NonInstalle,
    /// Le dossier ne se résout pas sur ce PC (jeton inconnu, fiche incohérente).
    NonResolu,
    /// Le sous-dossier `<identifiant …>` n'est pas retrouvé (le jeu ne l'a pas encore créé, ou le sel manque).
    Identifiant,
    /// Le jeu tourne (ou Frogtend s'en sert) : rien n'est écrit.
    EnCours,
    /// La copie de Firehouse n'est pas entière sur ce PC (un fichier de la fiche manque ou n'a pas son empreinte : une
    /// reprise coupée) : rien n'est posé ni noté, elle sera reprise.
    Incomplet,
}

/// Ce que la reprise a fait (des nombres d'emplacements, sauf `fichiers`).
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct BilanReprise {
    /// Jeux dont au moins une partie a été examinée.
    pub jeux: usize,
    /// Emplacements posés (au moins un fichier qui manquait).
    pub posees: usize,
    /// Fichiers posés.
    pub fichiers: usize,
    /// Emplacements déjà là à l'identique.
    pub deja_la: usize,
    pub non_installes: usize,
    pub non_resolues: usize,
    pub identifiants: usize,
    pub en_cours: usize,
    /// Emplacements en conflit (la partie de ce PC est différente) : une question, jamais écrasée.
    pub conflits: usize,
    /// Emplacements posés ou en conflit dont la copie est PARTIELLE (des fichiers étaient restés dans OneDrive).
    pub partielles: usize,
    /// Emplacements dont la copie téléchargée n'est pas entière (reprise coupée) : à reprendre.
    #[serde(default)]
    pub incompletes: usize,
    /// La reprise d'un jeu : le sel de ce PC diffère de celui de Firehouse (jamais la valeur).
    #[serde(default)]
    pub sel_ecart: bool,
}

impl BilanReprise {
    fn attendre(&mut self, a: Attente) {
        match a {
            Attente::NonInstalle => self.non_installes += 1,
            Attente::NonResolu => self.non_resolues += 1,
            Attente::Identifiant => self.identifiants += 1,
            Attente::EnCours => self.en_cours += 1,
            Attente::Incomplet => self.incompletes += 1,
        }
    }
    /// Des parties qui attendent (toutes raisons).
    pub fn en_attente(&self) -> usize {
        self.non_installes + self.non_resolues + self.identifiants + self.en_cours + self.incompletes
    }
}

/// Un côté d'un conflit : la date (secondes depuis 1970, `None` : inconnue), la taille et le nombre de fichiers.
/// Jamais un chemin ni une empreinte.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct Cote {
    pub date: Option<u64>,
    pub octets: u64,
    pub fichiers: usize,
}

/// Un conflit à montrer : la partie de ce PC et celle de Firehouse.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ConflitEmplacement {
    /// Le numéro du jeu dans la zone d'attente.
    pub jeu: i64,
    /// La clé de l'emplacement (son nom dans l'abri : 8 hexadécimaux, et l'empreinte salée d'un identifiant).
    pub cle: String,
    pub titre: String,
    pub ici: Cote,
    pub firehouse: Cote,
    pub partiel: bool,
}

/// Le registre des emplacements réglés.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Reglees {
    #[serde(default)]
    pub entrees: BTreeMap<String, Reglee>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Reglee {
    /// « posee », « gardee » (la personne garde la sienne) ou « remplacee ».
    pub etat: String,
    /// La signature de la copie de Firehouse réglée (une nouvelle copie est réexaminée).
    pub signature: String,
}

/// D'où viennent les parties d'un jeu.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Origine {
    pub profil: String,
    pub pc: String,
    /// `derniere.date` de cette sauvegarde (secondes, texte), si elle est connue.
    #[serde(default)]
    pub date: Option<String>,
}

fn lire_json<T: serde::de::DeserializeOwned + Default>(f: &Path) -> T {
    std::fs::read(f).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default()
}

fn ecrire_json<T: Serialize>(f: &Path, v: &T) -> Resultat<()> {
    if let Some(p) = f.parent() {
        std::fs::create_dir_all(p)?;
    }
    let provisoire = crate::sauvegarde::fichier_part(f);
    std::fs::write(&provisoire, serde_json::to_vec_pretty(v).unwrap_or_default())?;
    std::fs::rename(&provisoire, f)?;
    Ok(())
}

pub fn lire_reglees(zone: &Path) -> Reglees {
    lire_json(&zone.join(REGLEES))
}

pub fn ecrire_reglees(zone: &Path, r: &Reglees) -> Resultat<()> {
    ecrire_json(&zone.join(REGLEES), r)
}

pub fn lire_origines(zone: &Path) -> BTreeMap<String, Origine> {
    lire_json(&zone.join(ORIGINES))
}

/// Note d'où viennent les parties de ces jeux (une reprise plus récente remplace l'origine d'avant).
pub fn noter_origines(zone: &Path, ids: &[i64], o: &Origine) -> Resultat<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let mut l = lire_origines(zone);
    for id in ids {
        l.insert(id.to_string(), o.clone());
    }
    ecrire_json(&zone.join(ORIGINES), &l)
}

fn cle_reglee(jeu: i64, cle: &str) -> String {
    format!("{jeu}/{cle}")
}

/// Un fichier d'un emplacement en attente : son chemin dans le dossier de parties (`/`), sa copie d'attente, sa taille.
#[derive(Debug, Clone)]
pub struct FichierRepris {
    pub relatif: String,
    pub source: PathBuf,
    pub taille: u64,
}

/// L'état d'un emplacement sur ce PC.
#[derive(Debug, Clone, PartialEq)]
pub enum Statut {
    /// Au moins un fichier manque, aucun n'est différent : à poser.
    Absent,
    /// Tout est déjà là à l'identique.
    Identique,
    /// Au moins un fichier est différent : les fichiers différents (chemins dans le dossier de parties).
    Conflit(Vec<String>),
}

/// Un emplacement examiné.
#[derive(Debug, Clone)]
pub enum Examen {
    /// Aucun fichier de cet emplacement dans la zone d'attente.
    Vide,
    Attente(Attente),
    Resolu { lieu: Emplacement, fichiers: Vec<FichierRepris>, statut: Statut, signature: String },
}

/// La signature de la copie d'un emplacement, d'après sa fiche (chemins, SHA1, tailles) : une copie qui change est
/// réexaminée.
pub fn signature(fiche: &FicheAbri, cle: &str) -> String {
    use sha2::{Digest, Sha256};
    let prefixe = format!("{}{cle}/", ap::PREFIXE);
    let mut l: Vec<String> = fiche.fichiers.iter().filter(|f| f.relatif.starts_with(&prefixe)).map(|f| format!("{}\t{}\t{}", f.relatif.to_lowercase(), f.sha, f.taille)).collect();
    l.sort();
    format!("{:x}", Sha256::digest(l.join("\n").as_bytes()))
}

/// Un chemin relatif sans détour (`/`, sans « .. », sans lecteur).
fn relatif_sur(r: &str) -> bool {
    !r.is_empty() && !r.contains(['\\', ':']) && !r.split('/').any(|s| s.is_empty() || s == "." || s == "..")
}

/// Les fichiers de cet emplacement dans la zone d'attente du jeu, TOUS présents et chacun relu à l'empreinte (SHA1) de
/// la fiche. `Err(NonResolu)` : un chemin de la fiche n'est pas sûr (jamais suivi) ; `Err(Incomplet)` : un seul
/// fichier manque ou diffère (reprise coupée) — rien de cet emplacement n'est posé. Vide : la fiche n'en a aucun.
fn fichiers_en_attente(attente_jeu: &Path, fiche: &FicheAbri, cle: &str) -> std::result::Result<Vec<FichierRepris>, Attente> {
    let prefixe = format!("{}{cle}/", ap::PREFIXE);
    let mut l = Vec::new();
    for f in fiche.fichiers.iter().filter(|f| f.relatif.starts_with(&prefixe)) {
        let r = &f.relatif[prefixe.len()..];
        if !relatif_sur(&f.relatif) || !relatif_sur(r) {
            return Err(Attente::NonResolu);
        }
        let source = attente_jeu.join(&f.relatif);
        let entier = std::fs::symlink_metadata(&source).is_ok_and(|m| m.is_file() && m.len() == f.taille)
            && crate::nuage_steam::sha1(&source).is_ok_and(|x| x.eq_ignore_ascii_case(&f.sha));
        if !entier {
            return Err(Attente::Incomplet);
        }
        l.push(FichierRepris { relatif: r.to_string(), source, taille: f.taille });
    }
    Ok(l)
}

/// La fiche a-t-elle au moins un fichier pour cet emplacement ?
fn a_des_fichiers(fiche: &FicheAbri, cle: &str) -> bool {
    let prefixe = format!("{}{cle}/", ap::PREFIXE);
    fiche.fichiers.iter().any(|f| f.relatif.starts_with(&prefixe))
}

/// Le dossier de CE PC d'un emplacement de la fiche : un seul, jamais deviné. `sel` : celui du profil (sans lui, un
/// `<identifiant …>` ne se retrouve pas).
pub fn resoudre(e: &ap::EmplacementAbrite, ctx: &Contexte, sel: Option<&str>) -> std::result::Result<Emplacement, Attente> {
    // La clé de la fiche doit venir de ce chemin (une fiche incohérente n'écrit nulle part).
    let cle = ap::cle_chemin(&e.chemin, &e.nature);
    let (debut, empreinte) = match e.cle.split_once('-') {
        Some((d, x)) => (d, Some(x)),
        None => (e.cle.as_str(), None),
    };
    if debut != cle || empreinte != e.identifiant.as_deref() {
        return Err(Attente::NonResolu);
    }
    let (candidats, _) = crate::nuage_steam::candidats(&e.chemin, Some(&e.nature), ctx);
    let l: Vec<Emplacement> = match (&e.identifiant, sel) {
        (Some(_), None) => return Err(Attente::Identifiant),
        (Some(h), Some(sel)) => candidats.into_iter().filter(|c| c.identifiant.as_deref().is_some_and(|i| ap::empreinte_identifiant(i, sel) == *h)).collect(),
        (None, _) => candidats.into_iter().filter(|c| c.identifiant.is_none()).collect(),
    };
    let lieu = match (l.len(), e.identifiant.is_some()) {
        (1, _) => l.into_iter().next().unwrap_or_default(),
        (_, true) => return Err(Attente::Identifiant),
        (_, false) => return Err(Attente::NonResolu),
    };
    // Le nuage Steam (`<Steam>\userdata\…`) n'est JAMAIS restauré par Frogtend : Steam s'en charge (Seb, 24/11).
    if dans_userdata_steam(&lieu.dossier, ctx) {
        return Err(Attente::NonResolu);
    }
    Ok(lieu)
}

/// Un dossier sous le `userdata` de Steam (celui de ce PC, ou tout chemin `…\Steam\userdata\…`).
fn dans_userdata_steam(d: &Path, ctx: &Contexte) -> bool {
    let n = format!("{}\\", ap::normal(d));
    let sous_steam = ctx.steam.as_ref().is_some_and(|s| n.starts_with(&format!("{}\\", ap::normal(&s.join("userdata")))));
    sous_steam || n.contains("\\steam\\userdata\\")
}

/// Deux fichiers identiques (taille, puis empreinte).
fn identiques(a: &Path, b: &Path) -> bool {
    match (std::fs::metadata(a), std::fs::metadata(b)) {
        (Ok(x), Ok(y)) if x.is_file() && y.is_file() && x.len() == y.len() => {
            matches!((crate::sauvegarde::empreinte(a), crate::sauvegarde::empreinte(b)), (Ok(p), Ok(q)) if p == q)
        }
        _ => false,
    }
}

fn cible_de(lieu: &Emplacement, relatif: &str) -> PathBuf {
    relatif.split('/').fold(lieu.dossier.clone(), |p, s| p.join(s))
}

/// Examine un emplacement de la fiche : ses fichiers en attente, son dossier sur ce PC, son état.
pub fn examiner(attente_jeu: &Path, fiche: &FicheAbri, e: &ap::EmplacementAbrite, ctx: &Contexte, sel: Option<&str>) -> Examen {
    let fichiers = match fichiers_en_attente(attente_jeu, fiche, &e.cle) {
        Ok(l) => l,
        Err(a) => return Examen::Attente(a),
    };
    if fichiers.is_empty() {
        return Examen::Vide;
    }
    let lieu = match resoudre(e, ctx, sel) {
        Ok(l) => l,
        Err(a) => return Examen::Attente(a),
    };
    // Un emplacement d'un fichier ou d'un motif : seulement des fichiers DE ce dossier, qui répondent au filtre.
    if let Some(f) = &lieu.filtre {
        if fichiers.iter().any(|x| x.relatif.contains('/') || !crate::nuage_steam::repond(&x.relatif, f)) {
            return Examen::Attente(Attente::NonResolu);
        }
    }
    let mut differents = Vec::new();
    let mut absents = 0;
    for f in &fichiers {
        let cible = cible_de(&lieu, &f.relatif);
        match std::fs::symlink_metadata(&cible) {
            Err(_) => absents += 1,
            Ok(_) if identiques(&f.source, &cible) => {}
            Ok(_) => differents.push(f.relatif.clone()),
        }
    }
    let statut = if !differents.is_empty() {
        Statut::Conflit(differents)
    } else if absents > 0 {
        Statut::Absent
    } else {
        Statut::Identique
    };
    Examen::Resolu { lieu, fichiers, statut, signature: signature(fiche, &e.cle) }
}

/// Copie un fichier par un provisoire relu (même empreinte), puis le renomme à sa place. `remplacer` : la place peut
/// être prise (la personne a choisi la partie de Firehouse, la sienne est à l'abri) ; sinon jamais par-dessus.
fn poser_fichier(source: &Path, cible: &Path, remplacer: bool) -> Resultat<()> {
    if let Some(p) = cible.parent() {
        std::fs::create_dir_all(p)?;
    }
    let provisoire = crate::sauvegarde::fichier_part(cible);
    std::fs::copy(source, &provisoire)?;
    if !identiques(source, &provisoire) {
        let _ = std::fs::remove_file(&provisoire);
        return Err(Erreur::Disque("Une partie est arrivée abîmée sur le disque : rien n'est posé pour elle, réessaie.".into()));
    }
    if !remplacer {
        // Jamais par-dessus un fichier apparu entre-temps : un lien dur ne se crée PAS sur un nom pris (atomique) ; un
        // disque qui n'en fait pas (FAT, exFAT) : une création exclusive (`create_new`), puis la copie relue.
        let r = match std::fs::hard_link(&provisoire, cible) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Err(Erreur::Refus("Une partie est apparue entre-temps : elle n'est pas remplacée.".into())),
            Err(_) => match std::fs::OpenOptions::new().write(true).create_new(true).open(cible) {
                Ok(mut f) => {
                    let copie = std::fs::File::open(&provisoire).and_then(|mut p| std::io::copy(&mut p, &mut f)).and_then(|_| f.sync_all());
                    drop(f);
                    match copie {
                        Ok(()) if identiques(source, cible) => Ok(()),
                        // Le fichier que NOUS venons de créer, abîmé : retiré (il n'était pas là avant).
                        _ => {
                            let _ = std::fs::remove_file(cible);
                            Err(Erreur::Disque("Une partie est arrivée abîmée sur le disque : rien n'est posé pour elle, réessaie.".into()))
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Err(Erreur::Refus("Une partie est apparue entre-temps : elle n'est pas remplacée.".into())),
                Err(e) => Err(e.into()),
            },
        };
        let _ = std::fs::remove_file(&provisoire);
        return r;
    }
    if let Err(e) = std::fs::rename(&provisoire, cible) {
        let _ = std::fs::remove_file(&provisoire);
        return Err(e.into());
    }
    Ok(())
}

/// Défait un remplacement coupé au milieu : chaque fichier déjà remplacé est remis depuis l'abri d'avant (`Some(copie)`,
/// relu), chaque fichier ajouté (il n'existait pas) est retiré s'il est bien la copie de Firehouse. Rend l'erreur à dire.
fn defaire(faits: &[(PathBuf, Option<PathBuf>, PathBuf)]) -> Erreur {
    let mut entier = true;
    for (cible, avant, source) in faits.iter().rev() {
        let ok = match avant {
            Some(copie) => poser_fichier(copie, cible, true).is_ok() && identiques(copie, cible),
            None => !identiques(source, cible) || std::fs::remove_file(cible).is_ok(),
        };
        entier &= ok;
    }
    if entier {
        Erreur::Disque("Le remplacement s'est arrêté au milieu : ta partie de ce PC a été remise comme avant (relue). Rien n'a changé.".into())
    } else {
        Erreur::Disque("Le remplacement s'est arrêté au milieu et ta partie de ce PC n'a pas pu être remise entièrement : elle est intacte dans 🛟 Mes abris.".into())
    }
}

/// Pose les fichiers qui MANQUENT d'un emplacement sans conflit (règles de `restauration::reposer_sauf(…, false, …)` :
/// absent posé, identique rien, différent jamais écrasé), seulement ceux de la fiche ; rend le nombre posé.
pub fn poser_absents(lieu: &Emplacement, fichiers: &[FichierRepris]) -> Resultat<usize> {
    let mut n = 0;
    for f in fichiers {
        let cible = cible_de(lieu, &f.relatif);
        if std::fs::symlink_metadata(&cible).is_ok() {
            continue;
        }
        poser_fichier(&f.source, &cible, false)?;
        n += 1;
    }
    Ok(n)
}

/// La fiche envoyée des emplacements d'un jeu en attente (`jeux/<id>/emplacements/fiche.json`).
pub fn fiche_en_attente(attente_jeu: &Path) -> Option<FicheAbri> {
    let o = std::fs::read(attente_jeu.join(ap::FICHE_ENVOYEE)).ok()?;
    serde_json::from_slice::<FicheAbri>(&o).ok().filter(|f| f.version_fiche >= 1)
}

/// Une date ISO 8601 UTC (`2026-11-24T10:53:09Z`, `+00:00` admis, fractions ignorées) en secondes depuis 1970.
pub fn secondes_iso(t: &str) -> Option<u64> {
    let t = t.trim();
    let (date, heure) = t.split_once(['T', ' '])?;
    let mut d = date.split('-');
    let (a, m, j): (i64, i64, i64) = (d.next()?.parse().ok()?, d.next()?.parse().ok()?, d.next()?.parse().ok()?);
    let heure = heure.trim_end_matches('Z').trim_end_matches("+00:00");
    let heure = heure.split('.').next()?;
    let mut h = heure.split(':');
    let (hh, mm, ss): (i64, i64, i64) = (h.next()?.parse().ok()?, h.next()?.parse().ok()?, h.next().unwrap_or("0").parse().ok()?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&j) || hh > 23 || mm > 59 || ss > 60 || a < 1970 {
        return None;
    }
    // Jours depuis 1970 (algorithme « days from civil », calendrier grégorien).
    let y = if m <= 2 { a - 1 } else { a };
    let ere = y.div_euclid(400);
    let yoe = y - ere * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + j - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let jours = ere * 146097 + doe - 719468;
    u64::try_from(jours * 86400 + hh * 3600 + mm * 60 + ss).ok()
}

/// Relit le sel du profil chez Firehouse (`/sauvegarde/<profil>/_profil/document/abris-sel.json`) et le range dans
/// `dossier_profil` s'il y manque. Jamais la valeur dans un message.
pub async fn relire_sel(c: &Client, profil_api: &str, dossier_profil: &Path) -> EtatSel {
    let route = format!("/sauvegarde/{}/{}/document/{}", crate::source::encoder(profil_api), crate::sauvegarde::PC_PROFIL, ap::NOM_SEL_ENVOYE);
    let lu = tokio::time::timeout(DELAI_SEL, c.brute(reqwest::Method::GET, &route, None, &[])).await;
    let Ok(Ok(b)) = lu else { return EtatSel::Injoignable };
    let corps = serde_json::from_slice::<Value>(&b.octets).ok();
    match b.statut {
        200..=299 => match corps.as_ref().and_then(|v| v["sel"].as_str()).map(|s| s.trim().to_string()) {
            Some(s) if ap::sel_valable(&s) => ap::ranger_sel_recu(dossier_profil, &s).unwrap_or(EtatSel::Injoignable),
            _ => EtatSel::Injoignable,
        },
        401 => EtatSel::JetonRefuse,
        // Seulement le 404 du contrat (`{"detail": "document absent"}`) : un autre 404 (route inconnue d'un ancien
        // Firehouse, relais) ne dit RIEN du sel.
        404 if corps.as_ref().and_then(|v| v["detail"].as_str()) == Some("document absent") => EtatSel::Inconnu,
        _ => EtatSel::Injoignable,
    }
}

/// Au plus ce délai pour relire le sel (au-delà : injoignable, aucun sel créé).
pub const DELAI_SEL: std::time::Duration = std::time::Duration::from_secs(20);

/// La classe d'une erreur, pour le journal (jamais son motif : il pourrait porter un nom).
fn classe(e: &Erreur) -> &'static str {
    match e {
        Erreur::Disque(_) => "disque",
        Erreur::Refus(_) => "refus",
        Erreur::Reseau(_) => "réseau",
        Erreur::Serveur(_) => "serveur",
        _ => "autre",
    }
}

/// Un travail de fichiers sans bloquer les autres tâches : à part (`block_in_place`) sur le moteur à plusieurs fils de
/// l'application ; tel quel ailleurs (les tests, à un seul fil).
pub(crate) fn bloquant<T>(f: impl FnOnce() -> T) -> T {
    match tokio::runtime::Handle::try_current().map(|h| h.runtime_flavor()) {
        Ok(tokio::runtime::RuntimeFlavor::MultiThread) => tokio::task::block_in_place(f),
        _ => f(),
    }
}

/// Le texte d'un état du sel pour le journal (jamais la valeur).
fn texte_sel(e: EtatSel) -> &'static str {
    match e {
        EtatSel::Range => "rangé depuis Firehouse",
        EtatSel::Identique => "identique à celui de Firehouse",
        EtatSel::Ecart => "DIFFÉRENT de celui de Firehouse (gardé tel quel)",
        EtatSel::Inconnu => "inconnu de Firehouse",
        EtatSel::Injoignable => "pas relu (Firehouse injoignable)",
        EtatSel::JetonRefuse => "pas relu (jeton refusé)",
    }
}

/// Les emplacements des jeux en attente (numéro dans la zone d'attente, son dossier, sa fiche), filtrés par `ids`.
fn jeux_en_attente(zone: &Path, ids: Option<&[i64]>) -> Vec<(i64, PathBuf, FicheAbri)> {
    let jeux = zone.join("parties").join("jeux");
    let mut l: Vec<(i64, PathBuf, FicheAbri)> = std::fs::read_dir(&jeux)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let n: i64 = e.file_name().to_str()?.parse().ok()?;
            if ids.is_some_and(|l| !l.contains(&n)) {
                return None;
            }
            let d = e.path();
            fiche_en_attente(&d).map(|f| (n, d, f))
        })
        .collect();
    l.sort_by_key(|(n, _, _)| *n);
    l
}

/// Ce jeu de la sauvegarde existe-t-il pour ce profil ? Un jeu de Firehouse (numéro positif) : s'il le voit. Un jeu
/// importé (numéro négatif, propre à ce profil) : oui — `meme_jeu` vérifie ensuite que c'est bien lui.
pub(crate) fn visible(s: &Session, n: i64) -> bool {
    n <= 0 || s.verrou().visible_ou_reuni(n).unwrap_or(false)
}

/// Le jeu de ce PC est-il celui de la fiche ? Un jeu de Firehouse : son numéro suffit. Un jeu importé (son numéro change
/// d'un PC à l'autre) : seulement si la fiche dit sa boutique et sa clé, et que ce sont celles du jeu de ce PC — jamais
/// deviné.
fn meme_jeu(n: i64, fiche: &FicheAbri, inst: &Installe) -> bool {
    n > 0 || fiche.boutique.as_ref().zip(inst.boutique.as_ref()).is_some_and(|(a, b)| a.source == b.source && a.cle == b.cle)
}

/// Le jeu installé sur ce PC (son dossier et sa boutique), ou `None`.
struct Installe {
    id: i64,
    /// Le dossier de l'installation (les programmes qui tournent y sont cherchés).
    dossier: PathBuf,
    /// Lot S6 : le `<dossier du jeu>` des parties, celui de l'exécutable (`Installation::dossier_des_parties`).
    parties: PathBuf,
    boutique: Option<crate::emplacements_parties::BoutiqueJeu>,
}

impl Noyau {
    /// Le sel pour un abri du profil `profil_id` (lot S5) : celui de ce PC ; s'il manque et que ce profil est celui de
    /// la session reliée à Firehouse, celui de Firehouse d'abord (jamais un nouveau qui en écarterait un ancien) ; un 404
    /// ou Firehouse injoignable : un nouveau, comme avant.
    ///
    /// Règle de Seb (24/11) : un sel NEUF n'est créé que sur le 404 « document absent » (ou en mode simulé, sans
    /// Firehouse). Injoignable, trop lent, réponse illisible, ou un autre profil que celui de la session (son jeton
    /// n'est pas ouvert) : AUCUN sel — rend `""`, et les endroits à `<identifiant …>` attendent (`Echec::SelInconnu`),
    /// les autres sont abrités. Un 401 est une erreur dite comme un 401.
    pub(crate) async fn sel_pour_abri(&self, profil_id: &str) -> Resultat<String> {
        let dp = self.dossier.join("profils").join(profil_id);
        if let Some(s) = ap::sel_local(&dp) {
            return Ok(s);
        }
        let Ok(s) = self.session().await else { return Ok(String::new()) };
        if s.profil.id != profil_id {
            return Ok(String::new());
        }
        let crate::source::Source::Firehouse(c) = &s.source else { return ap::sel_du_profil(&dp) };
        let e = relire_sel(c, &crate::sauvegarde::nom_pour_api(&s.profil.nom), &dp).await;
        self.journaliser(&format!("sel des abris du profil : {}", texte_sel(e)));
        match e {
            EtatSel::Inconnu => ap::sel_du_profil(&dp),
            EtatSel::JetonRefuse => Err(crate::firehouse::erreur_du_statut(401, b"")),
            EtatSel::Injoignable => Ok(String::new()),
            EtatSel::Range | EtatSel::Identique | EtatSel::Ecart => Ok(ap::sel_local(&dp).unwrap_or_default()),
        }
    }

    /// Relit le sel de la sauvegarde `profil` (son nom chez Firehouse) pour le profil ouvert, avant de restaurer.
    pub(crate) async fn relire_sel_pour_restaurer(&self, c: &Client, s: &Session, profil: &str) -> EtatSel {
        let dp = self.dossier.join("profils").join(&s.profil.id);
        let e = relire_sel(c, &crate::sauvegarde::nom_pour_api(profil), &dp).await;
        self.journaliser(&format!("restauration : sel des abris {}", texte_sel(e)));
        e
    }

    fn installe(&self, id: i64) -> Option<Installe> {
        let j = self.registre().jeu(id).ok().flatten()?;
        let i = j.installation?;
        let dossier = PathBuf::from(&i.dossier);
        let parties = i.dossier_des_parties();
        dossier.is_dir().then(|| Installe {
            id,
            dossier,
            parties,
            boutique: i.boutique.map(|b| crate::emplacements_parties::BoutiqueJeu { source: b.source, cle: b.cle }),
        })
    }

    /// Le numéro du jeu sur ce PC pour un numéro de la sauvegarde (fiches réunies).
    fn numero_ici(&self, s: &Session, n: i64) -> i64 {
        if let Some(x) = self.registre().reunions().unwrap_or_default().into_iter().find(|(a, _)| *a == n).map(|(_, b)| b) {
            return x;
        }
        s.verrou().fusions().unwrap_or_default().into_iter().find(|(a, _, _)| *a == n).map(|(_, b, _)| b).unwrap_or(n)
    }

    /// Pose ce qui manque des parties PC en attente (`ids` : seulement ces numéros de la sauvegarde ; `None` : tous).
    /// `apres_installation` : appelé à la fin d'une installation (le jeu est alors « occupé » par elle, ce n'est pas un
    /// refus). Rend le bilan ; jamais un écrasement.
    pub(crate) fn poser_parties_pc(&self, s: &Session, ids: Option<&[i64]>, windows: &[(&'static str, PathBuf)], apres_installation: bool) -> Resultat<BilanReprise> {
        let zone = self.zone_restauration(&s.profil.id);
        let dp = self.dossier.join("profils").join(&s.profil.id);
        let sel = ap::sel_local(&dp);
        let mut reglees = lire_reglees(&zone);
        let mut b = BilanReprise::default();
        let mut change = false;
        for (n, attente, fiche) in jeux_en_attente(&zone, ids) {
            // Un jeu que ce profil ne voit pas n'existe pas (pas même en nombre).
            if !visible(s, n) {
                continue;
            }
            let a_regler: Vec<&ap::EmplacementAbrite> =
                fiche.emplacements.iter().filter(|e| reglees.entrees.get(&cle_reglee(n, &e.cle)).is_none_or(|r| r.signature != signature(&fiche, &e.cle))).collect();
            if a_regler.is_empty() {
                continue;
            }
            b.jeux += 1;
            let ici = self.numero_ici(s, n);
            let Some(inst) = self.installe(ici) else {
                for e in a_regler {
                    if a_des_fichiers(&fiche, &e.cle) {
                        b.attendre(Attente::NonInstalle);
                    }
                }
                continue;
            };
            // Un jeu importé (numéro propre à chaque PC) : seulement si c'est bien le même jeu (sa boutique et sa clé).
            if !meme_jeu(n, &fiche, &inst) {
                b.non_resolues += a_regler.len();
                continue;
            }
            // Occupé d'abord (une partie ne démarre pas pendant la pose), puis le jeu vérifié arrêté.
            if !apres_installation && self.utilise(inst.id) {
                b.en_cours += a_regler.len();
                continue;
            }
            let _occupe = self.occuper(inst.id);
            if !crate::lancement::processus_du_dossier(&inst.dossier).is_empty() {
                b.en_cours += a_regler.len();
                continue;
            }
            let ctx = crate::commandes::contexte_des_parties(&dp, inst.boutique.as_ref(), Some(inst.parties.clone()), Some(inst.dossier.clone()))(windows.to_vec());
            for e in a_regler {
                match examiner(&attente, &fiche, e, &ctx, sel.as_deref()) {
                    Examen::Vide => {}
                    Examen::Attente(a) => b.attendre(a),
                    Examen::Resolu { lieu, fichiers, statut, signature } => {
                        if e.partiel {
                            b.partielles += 1;
                        }
                        match statut {
                            Statut::Conflit(_) => b.conflits += 1,
                            Statut::Identique => {
                                b.deja_la += 1;
                                reglees.entrees.insert(cle_reglee(n, &e.cle), Reglee { etat: "posee".into(), signature });
                                change = true;
                            }
                            Statut::Absent => match poser_absents(&lieu, &fichiers) {
                                Ok(k) => {
                                    b.posees += 1;
                                    b.fichiers += k;
                                    reglees.entrees.insert(cle_reglee(n, &e.cle), Reglee { etat: "posee".into(), signature });
                                    change = true;
                                }
                                // Une pose ratée (disque, partie apparue) : réessayée la prochaine fois, les autres passent.
                                Err(x) => {
                                    self.journaliser(&format!("jeu {n} : une partie PC restaurée non posée ({})", classe(&x)));
                                    b.attendre(Attente::NonResolu);
                                }
                            },
                        }
                    }
                }
            }
        }
        if change {
            ecrire_reglees(&zone, &reglees)?;
        }
        if b.jeux > 0 {
            self.journaliser(&format!(
                "parties PC restaurées : {} jeu(x), {} posée(s) ({} fichier(s)), {} déjà là, {} en attente, {} conflit(s), {} partielle(s)",
                b.jeux,
                b.posees,
                b.fichiers,
                b.deja_la,
                b.en_attente(),
                b.conflits,
                b.partielles
            ));
        }
        Ok(b)
    }

    /// Les conflits des parties PC en attente (`ids` : seulement ces jeux), avec les deux côtés. La date « Chez
    /// Firehouse » : `X-Modifie` du premier fichier en conflit (un seul `HEAD` par conflit, jamais tous), sinon
    /// `derniere.date` de la sauvegarde d'où il vient.
    pub(crate) async fn conflits_parties_pc(&self, ids: Option<&[i64]>, windows: &[(&'static str, PathBuf)]) -> Resultat<Vec<ConflitEmplacement>> {
        let s = self.session().await?;
        let zone = self.zone_restauration(&s.profil.id);
        let dp = self.dossier.join("profils").join(&s.profil.id);
        let sel = ap::sel_local(&dp);
        let reglees = lire_reglees(&zone);
        let origines = lire_origines(&zone);
        // (conflit, premier fichier différent) : le HEAD se fait après, hors des verrous.
        let mut l: Vec<(ConflitEmplacement, String)> = Vec::new();
        for (n, attente, fiche) in jeux_en_attente(&zone, ids) {
            if !visible(&s, n) {
                continue;
            }
            let ici = self.numero_ici(&s, n);
            let Some(inst) = self.installe(ici).filter(|i| meme_jeu(n, &fiche, i)) else { continue };
            let ctx = crate::commandes::contexte_des_parties(&dp, inst.boutique.as_ref(), Some(inst.parties.clone()), Some(inst.dossier.clone()))(windows.to_vec());
            let titre = s.verrou().jeu(ici).ok().flatten().map(|j| j.titre).unwrap_or_default();
            for e in &fiche.emplacements {
                if reglees.entrees.get(&cle_reglee(n, &e.cle)).is_some_and(|r| r.signature == signature(&fiche, &e.cle)) {
                    continue;
                }
                let Examen::Resolu { lieu, fichiers, statut: Statut::Conflit(differents), .. } = examiner(&attente, &fiche, e, &ctx, sel.as_deref()) else { continue };
                let mut c = ConflitEmplacement { jeu: n, cle: e.cle.clone(), titre: titre.clone(), ici: Cote::default(), firehouse: Cote::default(), partiel: e.partiel };
                for f in &fichiers {
                    c.firehouse.octets += f.taille;
                    c.firehouse.fichiers += 1;
                    if let Ok(m) = std::fs::metadata(cible_de(&lieu, &f.relatif)) {
                        c.ici.octets += m.len();
                        c.ici.fichiers += 1;
                        let t = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs());
                        c.ici.date = c.ici.date.max(t);
                    }
                }
                c.firehouse.date = origines.get(&n.to_string()).and_then(|o| o.date.as_deref()).and_then(|d| d.trim().parse().ok());
                let premier = format!("jeux/{n}/{}{}/{}", ap::PREFIXE, e.cle, differents.first().cloned().unwrap_or_default());
                l.push((c, premier));
            }
        }
        // `X-Modifie` (contrat 1.52) quand Firehouse le dit, pour le premier fichier en conflit de chacun.
        if let crate::source::Source::Firehouse(client) = &s.source {
            for (c, premier) in &mut l {
                let Some(o) = origines.get(&c.jeu.to_string()) else { continue };
                let route = format!(
                    "/sauvegarde/{}/{}/fichier?chemin={}",
                    crate::source::encoder(&crate::sauvegarde::nom_pour_api(&o.profil)),
                    crate::source::encoder(&crate::sauvegarde::nom_pour_api(&o.pc)),
                    crate::source::encoder(premier)
                );
                let tete = tokio::time::timeout(std::time::Duration::from_secs(15), client.brute(reqwest::Method::HEAD, &route, None, &[])).await;
                // Au premier échec réseau (ou délai) et au premier 401 : plus aucun HEAD, les suivants gardent
                // `derniere.date`.
                let Ok(Ok(b)) = tete else { break };
                if b.statut == 401 {
                    break;
                }
                if let Some(t) = b.modifie.as_deref().filter(|_| b.complet && (200..300).contains(&b.statut)).and_then(secondes_iso) {
                    c.firehouse.date = Some(t);
                }
            }
        }
        Ok(l.into_iter().map(|(c, _)| c).collect())
    }

    /// « Garder celle de ce PC » : la copie de Firehouse n'est plus proposée (elle reste chez Firehouse jusqu'à la
    /// prochaine sauvegarde, et dans la zone d'attente). Rien n'est écrit dans le dossier de parties.
    pub(crate) async fn garder_partie_pc(&self, jeu: i64, cle: &str) -> Resultat<()> {
        let s = self.session().await?;
        let zone = self.zone_restauration(&s.profil.id);
        let attente = zone.join("parties").join("jeux").join(jeu.to_string());
        let fiche = fiche_en_attente(&attente).ok_or_else(|| Erreur::Introuvable("Cette partie n'est plus en attente.".into()))?;
        if !fiche.emplacements.iter().any(|e| e.cle == cle) {
            return Err(Erreur::Introuvable("Cette partie n'est plus en attente.".into()));
        }
        let mut r = lire_reglees(&zone);
        r.entrees.insert(cle_reglee(jeu, cle), Reglee { etat: "gardee".into(), signature: signature(&fiche, cle) });
        ecrire_reglees(&zone, &r)?;
        self.journaliser(&format!("jeu {jeu} : partie de ce PC gardée (celle de Firehouse écartée)"));
        Ok(())
    }

    /// « Reprendre celle de Firehouse » : la partie de ce PC d'abord mise à l'abri (abri COMPLET de l'emplacement, de
    /// sorte `emplacements`, relu identique) ; puis chaque fichier de Firehouse posé à sa place (copie relue) ; puis la
    /// partie reprise mise à l'abri à son tour. Les fichiers de ce PC absents de la copie de Firehouse ne sont jamais
    /// effacés. Rend vrai si l'abri d'après a pu se faire.
    pub(crate) async fn reprendre_partie_pc(&self, jeu: i64, cle: &str, windows: &[(&'static str, PathBuf)]) -> Resultat<bool> {
        let s = self.session().await?;
        let zone = self.zone_restauration(&s.profil.id);
        let dp = self.dossier.join("profils").join(&s.profil.id);
        let attente = zone.join("parties").join("jeux").join(jeu.to_string());
        if !visible(&s, jeu) {
            return Err(Erreur::Introuvable("Ce jeu n'est pas dans ta ludothèque.".into()));
        }
        let fiche = fiche_en_attente(&attente).ok_or_else(|| Erreur::Introuvable("Cette partie n'est plus en attente.".into()))?;
        let e = fiche.emplacements.iter().find(|e| e.cle == cle).cloned().ok_or_else(|| Erreur::Introuvable("Cette partie n'est plus en attente.".into()))?;
        let ici = self.numero_ici(&s, jeu);
        let inst = self.installe(ici).ok_or_else(|| Erreur::Refus("Ce jeu n'est pas installé sur ce PC : sa partie l'attendra.".into()))?;
        if !meme_jeu(jeu, &fiche, &inst) {
            return Err(Erreur::Refus("Frogtend n'est pas sûr que ce soit le même jeu sur ce PC : rien n'est remplacé.".into()));
        }
        // Occupé d'abord (aucune partie ne démarre pendant le remplacement), puis vérifié arrêté.
        if self.utilise(inst.id) {
            return Err(Erreur::Refus("Le jeu est en cours : quitte-le d'abord, puis recommence.".into()));
        }
        let _occupe = self.occuper(inst.id);
        if !crate::lancement::processus_du_dossier(&inst.dossier).is_empty() {
            return Err(Erreur::Refus("Le jeu est en cours : quitte-le d'abord, puis recommence.".into()));
        }
        let sel = ap::sel_local(&dp);
        let ctx = crate::commandes::contexte_des_parties(&dp, inst.boutique.as_ref(), Some(inst.parties.clone()), Some(inst.dossier.clone()))(windows.to_vec());
        let Examen::Resolu { lieu, fichiers, statut, signature } = examiner(&attente, &fiche, &e, &ctx, sel.as_deref()) else {
            return Err(Erreur::Refus("Cette partie ne se retrouve plus sur ce PC : rien n'est remplacé.".into()));
        };
        let abris = self.dossier_abris()?;
        let sel_abri = self.sel_pour_abri(&s.profil.id).await?;
        let profil = s.profil.id.clone();
        let quand: u64 = crate::noyau::maintenant().parse().unwrap_or(0);
        let deja_identique = statut == Statut::Identique;
        let l2 = lieu.clone();
        let (fait_apres, poses) = tokio::task::spawn_blocking(move || -> Resultat<(bool, usize)> {
            // 1. La partie de ce PC, à l'abri, COMPLÈTE (aucun endroit en échec, aucun fichier resté dans OneDrive).
            let mut plan = ap::planifier(std::slice::from_ref(&l2), &Default::default(), &sel_abri);
            if plan.echecs() > 0 || plan.en_ligne_seulement > 0 {
                let motif = plan.emplacements.iter().find_map(|e| e.echec).map(|x| x.texte()).unwrap_or("un fichier est resté dans OneDrive");
                return Err(Erreur::Refus(format!("Ta partie de ce PC ne peut pas être mise à l'abri en entier ({motif}) : rien n'est remplacé.")));
            }
            // Chaque fichier de ce PC → sa copie dans l'abri (pour défaire un remplacement coupé).
            let mut copies: BTreeMap<String, PathBuf> = BTreeMap::new();
            if plan.fichiers() > 0 {
                let fin = ap::abriter_par(&mut plan, &abris, ici, &quand.to_string(), &profil, ap::DECLENCHEUR_AVANT_RESTAURATION, None)?;
                // Relu : chaque fichier de l'abri a l'empreinte de la fiche ET celle du fichier de ce PC, maintenant.
                let f = ap::lire_fiche(&fin).ok_or_else(|| Erreur::Disque("L'abri de ta partie est illisible : rien n'est remplacé.".into()))?;
                let sources: BTreeMap<String, PathBuf> = plan.bons().flat_map(|e| e.fichiers.iter().map(|x| (x.relatif.clone(), x.source.clone()))).collect();
                for x in &f.fichiers {
                    let copie = crate::nuage_steam::sha1(&fin.join(&x.relatif)).ok();
                    let ici_maintenant = sources.get(&x.relatif).and_then(|p| crate::nuage_steam::sha1(p).ok());
                    if copie.as_deref() != Some(x.sha.as_str()) || ici_maintenant.as_deref() != Some(x.sha.as_str()) {
                        return Err(Erreur::Disque("L'abri de ta partie n'est pas identique à ta partie : rien n'est remplacé.".into()));
                    }
                    if let Some(p) = sources.get(&x.relatif) {
                        copies.insert(ap::normal(p), fin.join(&x.relatif));
                    }
                }
            }
            // 2. La partie de Firehouse, fichier par fichier (copie relue, puis renommée). Un échec au milieu : ce qui a
            // déjà été remplacé est REMIS depuis l'abri d'avant (relu), ce qui a été ajouté est retiré.
            let mut poses = 0;
            let mut faits: Vec<(PathBuf, Option<PathBuf>, PathBuf)> = Vec::new();
            if !deja_identique {
                for x in &fichiers {
                    let cible = cible_de(&l2, &x.relatif);
                    if identiques(&x.source, &cible) {
                        continue;
                    }
                    let avant = copies.get(&ap::normal(&cible)).cloned();
                    if poser_fichier(&x.source, &cible, true).is_err() {
                        return Err(defaire(&faits));
                    }
                    faits.push((cible, avant, x.source.clone()));
                    poses += 1;
                }
            }
            // 3. La partie reprise, à l'abri à son tour (la plus récente : la sauvegarde suivante l'envoie, elle) ;
            // réessayé une fois. Sans lui, la sauvegarde n'envoie plus cet endroit (`reprise_sans_abri_apres`).
            let mut fait = false;
            for k in 1..=2u64 {
                let mut apres = ap::planifier(std::slice::from_ref(&l2), &Default::default(), &sel_abri);
                if apres.fichiers() > 0 && apres.echecs() == 0 && ap::abriter_par(&mut apres, &abris, ici, &(quand + k).to_string(), &profil, ap::DECLENCHEUR_RESTAUREE, None).is_ok() {
                    fait = true;
                    break;
                }
            }
            Ok((fait, poses))
        })
        .await
        .map_err(|_| Erreur::Disque("Le remplacement s'est arrêté brutalement.".into()))??;
        let mut r = lire_reglees(&zone);
        r.entrees.insert(cle_reglee(jeu, cle), Reglee { etat: "remplacee".into(), signature });
        ecrire_reglees(&zone, &r)?;
        self.journaliser(&format!("jeu {jeu} : partie de Firehouse reprise ({poses} fichier(s)), celle de ce PC à l'abri ; abri d'après : {}", if fait_apres { "fait" } else { "PAS fait" }));
        Ok(fait_apres)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abri_parties::{EmplacementAbrite, FichierAbrite};

    const SEL: &str = "0123456789abcdef0123456789abcdef";

    fn fiche(emplacements: Vec<EmplacementAbrite>, fichiers: Vec<(&str, &[u8])>) -> FicheAbri {
        FicheAbri {
            version_fiche: 1,
            sorte: ap::SORTE_EMPLACEMENTS.into(),
            profil: "p1".into(),
            complet: true,
            version: None,
            emplacements,
            fichiers: fichiers
                .into_iter()
                .map(|(r, o)| {
                    use sha1::{Digest, Sha1};
                    FichierAbrite { relatif: r.into(), taille: o.len() as u64, sha: format!("{:x}", Sha1::digest(o)) }
                })
                .collect(),
            octets: 0,
            en_ligne_seulement: 0,
            echecs: 0,
            declencheur: "main".into(),
            boutique: None,
            a_confirmer: false,
            pas_a_moi: false,
        }
    }

    fn ecrire(d: &Path, r: &str, o: &[u8]) {
        let f = r.split('/').fold(d.to_path_buf(), |p, s| p.join(s));
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(f, o).unwrap();
    }

    fn ctx(docs: &Path, jeu: Option<PathBuf>) -> Contexte {
        Contexte { jeu, steam: None, compte: None, windows: vec![("Documents", docs.to_path_buf()), ("Parties-enregistrees", docs.join("Saved Games"))], installation: None, boutique: None }
    }

    fn emplacement(chemin: &str, nature: &str, identifiant: Option<&str>) -> EmplacementAbrite {
        let cle = ap::cle_chemin(chemin, nature);
        let h = identifiant.map(|i| ap::empreinte_identifiant(i, SEL));
        EmplacementAbrite {
            cle: match &h {
                Some(h) => format!("{cle}-{h}"),
                None => cle,
            },
            chemin: chemin.into(),
            nature: nature.into(),
            identifiant: h,
            ..Default::default()
        }
    }

    #[test]
    fn une_date_iso_devient_des_secondes() {
        assert_eq!(secondes_iso("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(secondes_iso("2026-11-24T10:53:09Z"), Some(1_795_517_589));
        assert_eq!(secondes_iso("2024-02-29T12:00:00.123+00:00"), Some(1_709_208_000));
        assert_eq!(secondes_iso("pas une date"), None);
        assert_eq!(secondes_iso("2026-13-01T00:00:00Z"), None);
    }

    /// Absent → posé ; identique → rien ; différent → conflit, rien de l'emplacement n'est posé ni écrasé.
    #[test]
    fn absent_pose_identique_rien_different_conflit_sans_melange() {
        let d = tempfile::tempdir().unwrap();
        let docs = d.path().join("Docs");
        let attente = d.path().join("attente").join("110");
        let e = emplacement(r"%USERPROFILE%\Documents\Studio\Jeu", "dossier", None);
        let f = fiche(vec![e.clone()], vec![(&format!("emplacements/{}/slot1.sav", e.cle), b"un"), (&format!("emplacements/{}/sous/slot2.sav", e.cle), b"deux")]);
        ecrire(&attente, &format!("emplacements/{}/slot1.sav", e.cle), b"un");
        ecrire(&attente, &format!("emplacements/{}/sous/slot2.sav", e.cle), b"deux");
        let c = ctx(&docs, None);
        // Le dossier n'existe pas encore (jeu jamais lancé ici) : il est créé, tout est posé.
        let Examen::Resolu { lieu, fichiers, statut, .. } = examiner(&attente, &f, &e, &c, Some(SEL)) else { panic!() };
        assert_eq!(statut, Statut::Absent);
        assert_eq!(poser_absents(&lieu, &fichiers).unwrap(), 2);
        let jeu = docs.join("Studio").join("Jeu");
        assert_eq!(std::fs::read(jeu.join("sous").join("slot2.sav")).unwrap(), b"deux");
        assert!(!jeu.join("slot1.sav.part-frogtend").exists());
        // Identique : rien.
        assert!(matches!(examiner(&attente, &f, &e, &c, Some(SEL)), Examen::Resolu { statut: Statut::Identique, .. }));
        // Il a rejoué : différent → conflit ; un fichier qui manque n'est PAS posé (jamais un mélange).
        std::fs::write(jeu.join("slot1.sav"), b"plus loin").unwrap();
        std::fs::remove_file(jeu.join("sous").join("slot2.sav")).unwrap();
        let Examen::Resolu { statut, .. } = examiner(&attente, &f, &e, &c, Some(SEL)) else { panic!() };
        assert_eq!(statut, Statut::Conflit(vec!["slot1.sav".into()]));
        assert_eq!(std::fs::read(jeu.join("slot1.sav")).unwrap(), b"plus loin", "jamais écrasé");
    }

    /// Un `<identifiant …>` se retrouve par son empreinte salée parmi les dossiers qui existent ; sans sel, ou sans le
    /// bon dossier : en attente, jamais deviné.
    #[test]
    fn un_identifiant_se_retrouve_par_son_empreinte_jamais_devine() {
        let d = tempfile::tempdir().unwrap();
        let docs = d.path().join("Docs");
        let chemin = r"%USERPROFILE%\Documents\Rockstar Games\GTA V\Profiles\<identifiant utilisateur>";
        let e = emplacement(chemin, "dossier", Some("A1B2"));
        let f = fiche(vec![e.clone()], vec![(&format!("emplacements/{}/SGTA50000", e.cle), b"gta")]);
        let attente = d.path().join("attente");
        ecrire(&attente, &format!("emplacements/{}/SGTA50000", e.cle), b"gta");
        let c = ctx(&docs, None);
        // Aucun profil du jeu sur ce PC : en attente.
        assert!(matches!(examiner(&attente, &f, &e, &c, Some(SEL)), Examen::Attente(Attente::Identifiant)));
        // Deux profils, dont un seul a la bonne empreinte : celui-là.
        let profils = docs.join("Rockstar Games").join("GTA V").join("Profiles");
        std::fs::create_dir_all(profils.join("ZZZZ")).unwrap();
        std::fs::create_dir_all(profils.join("A1B2")).unwrap();
        let Examen::Resolu { lieu, .. } = examiner(&attente, &f, &e, &c, Some(SEL)) else { panic!() };
        assert_eq!(lieu.dossier, profils.join("A1B2"));
        // Sans sel, ou avec un autre sel : jamais deviné.
        assert!(matches!(examiner(&attente, &f, &e, &c, None), Examen::Attente(Attente::Identifiant)));
        assert!(matches!(examiner(&attente, &f, &e, &c, Some("autre")), Examen::Attente(Attente::Identifiant)));
    }

    /// Un jeton inconnu, une fiche incohérente (clé qui ne vient pas du chemin), un chemin qui sort : rien n'est écrit.
    #[test]
    fn une_fiche_douteuse_n_ecrit_nulle_part() {
        let d = tempfile::tempdir().unwrap();
        let docs = d.path().join("Docs");
        let attente = d.path().join("attente");
        let c = ctx(&docs, None);
        let inconnu = emplacement(r"<jeton inconnu>\Jeu", "dossier", None);
        let f = fiche(vec![inconnu.clone()], vec![(&format!("emplacements/{}/a.sav", inconnu.cle), b"a")]);
        ecrire(&attente, &format!("emplacements/{}/a.sav", inconnu.cle), b"a");
        assert!(matches!(examiner(&attente, &f, &inconnu, &c, Some(SEL)), Examen::Attente(Attente::NonResolu)));
        let mut triche = emplacement(r"%USERPROFILE%\Documents\Jeu", "dossier", None);
        triche.cle = "deadbeef".into();
        let f = fiche(vec![triche.clone()], vec![("emplacements/deadbeef/a.sav", b"a")]);
        ecrire(&attente, "emplacements/deadbeef/a.sav", b"a");
        assert!(matches!(examiner(&attente, &f, &triche, &c, Some(SEL)), Examen::Attente(Attente::NonResolu)));
        let e = emplacement(r"%USERPROFILE%\Documents\Jeu", "dossier", None);
        let f = fiche(vec![e.clone()], vec![(&format!("emplacements/{}/../../evasion.sav", e.cle), b"a")]);
        assert!(matches!(examiner(&attente, &f, &e, &c, Some(SEL)), Examen::Attente(Attente::NonResolu)));
        // Un emplacement « fichier » : seulement un fichier de ce nom, jamais un sous-dossier.
        let un = emplacement(r"%USERPROFILE%\Documents\Jeu\options.ini", "fichier", None);
        let f = fiche(vec![un.clone()], vec![(&format!("emplacements/{}/autre.exe", un.cle), b"x")]);
        ecrire(&attente, &format!("emplacements/{}/autre.exe", un.cle), b"x");
        assert!(matches!(examiner(&attente, &f, &un, &c, Some(SEL)), Examen::Attente(Attente::NonResolu)));
        assert!(!docs.exists(), "rien n'a été écrit");
    }

    /// La copie provisoire porte le nom COMPLET suivi du suffixe (jamais `with_extension`).
    #[test]
    fn le_provisoire_garde_le_nom_complet() {
        assert_eq!(crate::sauvegarde::fichier_part(Path::new("D:\\x\\PARTIE.SAV")), PathBuf::from("D:\\x\\PARTIE.SAV.part-frogtend"));
        assert_ne!(crate::sauvegarde::fichier_part(Path::new("PARTIE.SAV")), crate::sauvegarde::fichier_part(Path::new("PARTIE.BAK")));
    }

    /// Ajoute au registre un jeu installé dans `dossier`.
    fn installer(n: &Noyau, id: i64, dossier: &Path) {
        let j = jeu_installe(id, dossier);
        n.registre().ajouter(&j).unwrap();
        n.registre().changer_installation(id, j.installation.as_ref()).unwrap();
    }

    fn jeu_installe(id: i64, dossier: &Path) -> crate::jeux_pc::JeuPc {
        std::fs::create_dir_all(dossier).unwrap();
        crate::jeux_pc::JeuPc {
            id,
            version: 0,
            titre: "Jeu".into(),
            plateforme: "Windows".into(),
            dossier: dossier.to_string_lossy().into(),
            etat: crate::jeux_pc::Etat::Telecharge,
            total: 0,
            fichiers: vec![],
            message: None,
            ajoute_le: "1".into(),
            ajoute_par: "p".into(),
            installation: Some(crate::jeux_pc::Installation {
                dossier: dossier.to_string_lossy().into(),
                methode: crate::installation::Methode::Aucune,
                lanceur: None,
                fichier_du_jeu: None,
                installe_le: "1".into(),
                versions: vec![],
                boutique: None,
                extraits: vec![],
            }),
            temps_jeu: 0,
            derniere_partie: None,
        }
    }

    /// Range dans la zone d'attente du profil les parties PC d'un jeu (fiche et fichiers), comme une restauration.
    fn en_attente(zone: &Path, jeu: i64, emplacements: Vec<EmplacementAbrite>, fichiers: &[(String, &[u8])]) {
        let d = zone.join("parties").join("jeux").join(jeu.to_string());
        let f = fiche(emplacements, fichiers.iter().map(|(r, o)| (r.as_str(), *o)).collect());
        ecrire(&d, ap::FICHE_ENVOYEE, &serde_json::to_vec(&f).unwrap());
        for (r, o) in fichiers {
            ecrire(&d, r, o);
        }
    }

    /// Les trois moments du lot S5 sur un profil simulé : pose d'office de ce qui manque, conflit jamais écrasé (gardé
    /// ou repris, la partie de ce PC à l'abri d'abord), jeu non installé en attente, jeu caché jamais compté, jeu en
    /// cours refusé, emplacement partiel dit, nuage Steam jamais posé.
    #[tokio::test]
    async fn les_parties_pc_restaurees_sont_posees_sans_rien_ecraser_et_un_conflit_se_regle() {
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, None).unwrap().id;
        n.ouvrir(&id, None, &crate::noyau::Connexion { adresse: String::new(), simule: true }).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();
        n.regler_abris(Some(d.path().join("Abris")));
        let docs = d.path().join("Docs");
        let windows = vec![("Documents", docs.clone())];
        let zone = n.zone_restauration(&id);
        let a = EmplacementAbrite { partiel: true, ..emplacement(r"%USERPROFILE%\Documents\Cryo\Dune", "dossier", None) };
        let b = emplacement(r"%USERPROFILE%\Documents\Cryo\Dune Options", "dossier", None);
        let fa = format!("emplacements/{}/DUNE.SAV", a.cle);
        let fb = format!("emplacements/{}/options.ini", b.cle);
        en_attente(&zone, 110, vec![a.clone(), b.clone()], &[(fa.clone(), b"partie de firehouse"), (fb.clone(), b"options de firehouse")]);
        ecrire(&zone.join("parties").join("jeux").join("110"), "nuage-steam/m1/remote/x.sav", b"nuage");
        noter_origines(&zone, &[110], &Origine { profil: "Seb".into(), pc: "VENKMAN".into(), date: Some("1700000000".into()) }).unwrap();
        // Dune II (111) n'est pas installé ; 999 n'est pas dans la ludothèque de ce profil.
        let c = emplacement(r"%USERPROFILE%\Documents\Westwood\Dune2", "dossier", None);
        en_attente(&zone, 111, vec![c.clone()], &[(format!("emplacements/{}/d2.sav", c.cle), b"d2")]);
        let x = emplacement(r"%USERPROFILE%\Documents\Cache", "dossier", None);
        en_attente(&zone, 999, vec![x.clone()], &[(format!("emplacements/{}/secret.sav", x.cle), b"secret")]);
        // Sur ce PC : les options DIFFÈRENT (il a joué ici aussi).
        std::fs::create_dir_all(docs.join("Cryo").join("Dune Options")).unwrap();
        std::fs::write(docs.join("Cryo").join("Dune Options").join("options.ini"), b"mes options").unwrap();
        installer(&n, 110, &d.path().join("Jeux").join("Dune"));
        let s = n.session().await.unwrap();

        // Le jeu « occupé » (une partie en cours) : rien n'est écrit.
        {
            let _o = n.occuper(110);
            let r = n.poser_parties_pc(&s, None, &windows, false).unwrap();
            assert_eq!((r.en_cours, r.posees), (2, 0));
            assert!(!docs.join("Cryo").join("Dune").exists());
        }
        let r = n.poser_parties_pc(&s, None, &windows, false).unwrap();
        assert_eq!((r.jeux, r.posees, r.fichiers, r.conflits, r.non_installes, r.partielles), (2, 1, 1, 1, 1, 1), "{r:?}");
        assert_eq!(std::fs::read(docs.join("Cryo").join("Dune").join("DUNE.SAV")).unwrap(), b"partie de firehouse");
        assert_eq!(std::fs::read(docs.join("Cryo").join("Dune Options").join("options.ini")).unwrap(), b"mes options", "jamais écrasée");
        assert!(!docs.join("Cache").exists() && !docs.join("Westwood").exists());
        // Le nuage Steam n'est jamais posé, nulle part (un emplacement qui vise userdata : voir
        // `rien_n_est_jamais_pose_dans_userdata_de_steam`) : son fichier n'existe que dans la zone d'attente.
        fn partout(d: &Path, nom: &str, l: &mut Vec<PathBuf>) {
            for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
                if e.path().is_dir() {
                    partout(&e.path(), nom, l);
                } else if e.file_name() == nom {
                    l.push(e.path());
                }
            }
        }
        let mut trouves = Vec::new();
        partout(d.path(), "x.sav", &mut trouves);
        assert!(trouves.iter().all(|p| p.starts_with(&zone)), "{trouves:?}");
        // Posée : plus jamais retraitée (il rejoue : aucun faux conflit).
        std::fs::write(docs.join("Cryo").join("Dune").join("DUNE.SAV"), b"il a rejoue").unwrap();
        let r = n.poser_parties_pc(&s, None, &windows, false).unwrap();
        assert_eq!((r.posees, r.conflits), (0, 1));

        // Le conflit : deux colonnes, la date de Firehouse = celle de la sauvegarde (mode simulé : aucun HEAD).
        let l = n.conflits_parties_pc(None, &windows).await.unwrap();
        assert_eq!(l.len(), 1);
        assert_eq!((l[0].jeu, l[0].cle.as_str(), l[0].titre.as_str()), (110, b.cle.as_str(), "Dune"));
        assert_eq!((l[0].ici.octets, l[0].firehouse.octets, l[0].firehouse.date), (11, 20, Some(1_700_000_000)));
        assert!(l[0].ici.date.is_some());
        let brut = serde_json::to_string(&l).unwrap();
        assert!(!brut.contains("Docs") && !brut.contains("options.ini"), "aucun chemin : {brut}");

        // « Reprendre celle de Firehouse » : la sienne à l'abri (relu), puis remplacée, puis la reprise à l'abri.
        assert!(n.reprendre_partie_pc(110, &b.cle, &windows).await.unwrap(), "l'abri d'après est fait");
        assert_eq!(std::fs::read(docs.join("Cryo").join("Dune Options").join("options.ini")).unwrap(), b"options de firehouse");
        let abris: Vec<PathBuf> = std::fs::read_dir(d.path().join("Abris").join("110")).unwrap().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
        let fiches: Vec<FicheAbri> = abris.iter().filter_map(|p| ap::lire_fiche(p)).collect();
        let avant = abris.iter().find(|p| ap::lire_fiche(p).is_some_and(|f| f.declencheur == ap::DECLENCHEUR_AVANT_RESTAURATION)).expect("l'abri d'avant");
        assert_eq!(std::fs::read(avant.join(format!("emplacements/{}/options.ini", b.cle))).unwrap(), b"mes options", "sa partie à l'abri");
        assert!(fiches.iter().any(|f| f.declencheur == ap::DECLENCHEUR_RESTAUREE), "la partie reprise à l'abri aussi");
        assert!(n.conflits_parties_pc(None, &windows).await.unwrap().is_empty());
        assert_eq!(n.poser_parties_pc(&s, None, &windows, false).unwrap().conflits, 0, "réglé : plus redit");

        // « Garder celle de ce PC » : une nouvelle copie de Firehouse arrive, la personne garde la sienne.
        en_attente(&zone, 110, vec![a.clone(), b.clone()], &[(fa.clone(), b"partie de firehouse"), (fb.clone(), b"options encore plus neuves")]);
        assert_eq!(n.conflits_parties_pc(Some(&[110]), &windows).await.unwrap().len(), 1);
        n.garder_partie_pc(110, &b.cle).await.unwrap();
        assert!(n.conflits_parties_pc(Some(&[110]), &windows).await.unwrap().is_empty());
        assert_eq!(std::fs::read(docs.join("Cryo").join("Dune Options").join("options.ini")).unwrap(), b"options de firehouse", "rien n'est écrit");
        assert!(n.garder_partie_pc(110, "inconnue").await.is_err());
        // Un jeu caché n'est jamais repris (jamais dit autrement qu'« absent »).
        assert!(matches!(n.reprendre_partie_pc(999, &x.cle, &windows).await, Err(Erreur::Introuvable(_))));

        // La fin de l'installation de Dune II : ce qui l'attendait est posé (le jeu « occupé » par l'installation).
        installer(&n, 111, &d.path().join("Jeux").join("Dune II"));
        let _o = n.occuper(111);
        let r = n.poser_parties_pc(&s, Some(&[111]), &windows, true).unwrap();
        assert_eq!((r.posees, r.en_cours), (1, 0));
        assert_eq!(std::fs::read(docs.join("Westwood").join("Dune2").join("d2.sav")).unwrap(), b"d2");
        // Le journal : jamais un chemin ni un titre.
        let journal = std::fs::read_to_string(d.path().join("app").join("journal.txt")).unwrap_or_default();
        assert!(!journal.contains("Docs") && !journal.contains("options.ini"), "{journal}");
    }

    /// Un jeu importé (numéro négatif, propre à chaque PC) : ses parties ne vont qu'au MÊME jeu (boutique et clé de la
    /// fiche), jamais à un autre jeu qui aurait pris ce numéro ici.
    #[tokio::test]
    async fn un_jeu_importe_ne_recoit_que_ses_propres_parties() {
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, None).unwrap().id;
        n.ouvrir(&id, None, &crate::noyau::Connexion { adresse: String::new(), simule: true }).await.unwrap();
        let docs = d.path().join("Docs");
        let windows = vec![("Documents", docs.clone())];
        let zone = n.zone_restauration(&id);
        let e = emplacement(r"%USERPROFILE%\Documents\Jeu Steam", "dossier", None);
        let d5 = zone.join("parties").join("jeux").join("-5");
        let mut f = fiche(vec![e.clone()], vec![(&format!("emplacements/{}/a.sav", e.cle), b"a")]);
        f.boutique = Some(ap::BoutiqueDuJeu { source: "steam".into(), cle: "268500".into() });
        ecrire(&d5, ap::FICHE_ENVOYEE, &serde_json::to_vec(&f).unwrap());
        ecrire(&d5, &format!("emplacements/{}/a.sav", e.cle), b"a");
        let mut j = jeu_installe(-5, &d.path().join("Jeux").join("Autre"));
        n.registre().ajouter(&j).unwrap();
        n.registre().changer_installation(-5, j.installation.as_ref()).unwrap();
        let s = n.session().await.unwrap();
        // Un AUTRE jeu sous le numéro -5 sur ce PC : rien n'est posé.
        let r = n.poser_parties_pc(&s, None, &windows, false).unwrap();
        assert_eq!((r.posees, r.non_resolues), (0, 1));
        assert!(!docs.exists());
        // Le même jeu (même boutique, même clé) : posé.
        j.installation.as_mut().unwrap().boutique = Some(crate::jeux_pc::LancementBoutique { source: "steam".into(), cle: "268500".into() });
        n.registre().changer_installation(-5, j.installation.as_ref()).unwrap();
        let r = n.poser_parties_pc(&s, None, &windows, false).unwrap();
        assert_eq!(r.posees, 1);
        assert_eq!(std::fs::read(docs.join("Jeu Steam").join("a.sav")).unwrap(), b"a");
    }

    /// Le sel avant un abri : relu chez Firehouse quand il manque (rangé, jamais au journal) ; 404 : un nouveau ; un sel
    /// local déjà là : aucune demande.
    #[tokio::test]
    async fn le_sel_est_relu_chez_firehouse_avant_d_en_creer_un() {
        use httpmock::prelude::*;
        let s = MockServer::start();
        let route = "/api/jeux/v1/sauvegarde/Seb/_profil/document/abris-sel.json";
        let present = s.mock(|w, t| {
            w.method(GET).path(route);
            t.status(200).json_body(serde_json::json!({"sel": SEL}));
        });
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &crate::noyau::Connexion { adresse: s.base_url(), simule: false }).await.unwrap();
        assert_eq!(n.sel_pour_abri(&id).await.unwrap(), SEL);
        assert_eq!(ap::sel_local(&d.path().join("app").join("profils").join(&id)).as_deref(), Some(SEL));
        // Déjà là : plus demandé.
        assert_eq!(n.sel_pour_abri(&id).await.unwrap(), SEL);
        present.assert_hits(1);
        let journal = std::fs::read_to_string(d.path().join("app").join("journal.txt")).unwrap_or_default();
        assert!(!journal.contains(SEL), "jamais le sel au journal");
        // Un autre profil, que Firehouse ne connaît pas (404) : un nouveau sel, comme avant.
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/sauvegarde/Lea/_profil/document/abris-sel.json");
            t.status(404).json_body(serde_json::json!({"detail": "document absent"}));
        });
        let lea = n.creer_profil("Lea", None, Some("j2")).unwrap().id;
        n.ouvrir(&lea, None, &crate::noyau::Connexion { adresse: s.base_url(), simule: false }).await.unwrap();
        let nouveau = n.sel_pour_abri(&lea).await.unwrap();
        assert_eq!(nouveau.len(), 32);
        assert_ne!(nouveau, SEL);
        // Un autre profil que celui de la session : jamais un sel créé (son jeton n'est pas ouvert).
        let zoe = n.creer_profil("Zoe", None, Some("j3")).unwrap().id;
        assert_eq!(n.sel_pour_abri(&zoe).await.unwrap(), "");
        assert!(ap::sel_local(&d.path().join("app").join("profils").join(&zoe)).is_none());
    }

    /// Règle de Seb (24/11) : un sel neuf SEULEMENT sur le 404 « document absent ». Injoignable, réponse illisible, un
    /// autre 404 : aucun sel (rend vide) ; 401 : une erreur dite comme un 401. Jamais rien de rangé.
    #[tokio::test]
    async fn sans_le_404_du_contrat_aucun_sel_n_est_cree() {
        use httpmock::prelude::*;
        let s = MockServer::start();
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let cas: [(&str, u16, &str); 4] = [("A", 401, "{}"), ("B", 500, "{}"), ("C", 404, r#"{"detail":"Not Found"}"#), ("D", 200, r#"{"sel":"pas-hexa"}"#)];
        for (nom, code, corps) in cas {
            s.mock(|w, t| {
                w.method(GET).path(format!("/api/jeux/v1/sauvegarde/{nom}/_profil/document/abris-sel.json"));
                t.status(code).header("content-type", "application/json").body(corps);
            });
            let id = n.creer_profil(nom, None, Some("j")).unwrap().id;
            n.ouvrir(&id, None, &crate::noyau::Connexion { adresse: s.base_url(), simule: false }).await.unwrap();
            let r = n.sel_pour_abri(&id).await;
            if code == 401 {
                assert!(matches!(r, Err(Erreur::JetonRefuse(_))), "{nom} : {r:?}");
            } else {
                assert_eq!(r.unwrap(), "", "{nom}");
            }
            assert!(ap::sel_local(&d.path().join("app").join("profils").join(&id)).is_none(), "{nom} : aucun sel créé");
        }
        // Hors ligne (personne n'écoute) : aucun sel.
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let adresse = format!("http://{}", l.local_addr().unwrap());
        drop(l);
        let id = n.creer_profil("E", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &crate::noyau::Connexion { adresse, simule: false }).await.unwrap();
        assert_eq!(n.sel_pour_abri(&id).await.unwrap(), "");
        assert!(ap::sel_local(&d.path().join("app").join("profils").join(&id)).is_none());
        // Sans sel : l'endroit à identifiant attend (dit), les autres sont abrités.
        let p = d.path().join("Docs");
        std::fs::create_dir_all(p.join("Profiles").join("A1")).unwrap();
        std::fs::write(p.join("Profiles").join("A1").join("x.sav"), b"x").unwrap();
        std::fs::create_dir_all(p.join("Options")).unwrap();
        std::fs::write(p.join("Options").join("o.ini"), b"o").unwrap();
        let lieux = vec![
            Emplacement { dossier: p.join("Profiles").join("A1"), filtre: None, chemin: r"%USERPROFILE%\Documents\Profiles\<identifiant utilisateur>".into(), nature: "dossier".into(), identifiant: Some("A1".into()) },
            Emplacement { dossier: p.join("Options"), filtre: None, chemin: r"%USERPROFILE%\Documents\Options".into(), nature: "dossier".into(), identifiant: None },
        ];
        let plan = ap::planifier(&lieux, &Default::default(), "");
        assert_eq!((plan.fichiers(), plan.echecs()), (1, 1));
        assert!(plan.emplacements.iter().any(|e| e.echec == Some(ap::Echec::SelInconnu)));
    }

    /// « 📥 Reprendre mes parties de Firehouse » sur UN jeu : seulement `jeux/<id>/emplacements/` (fiche d'abord),
    /// jamais `nuage-steam/` ni un autre jeu ; le sel relu et rangé ; puis la date « Chez Firehouse » d'un conflit :
    /// `X-Modifie` quand Firehouse le donne, sinon `derniere.date`.
    #[tokio::test]
    async fn reprendre_un_jeu_ne_telecharge_que_ses_parties_pc() {
        use httpmock::prelude::*;
        use sha2::{Digest, Sha256};
        let sha = |o: &[u8]| format!("{:x}", Sha256::digest(o));
        let s = MockServer::start();
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/plateformes");
            t.status(200).json_body(serde_json::json!({"ok": true, "plateformes": [{"nom": "Windows", "jeux": 1}]}));
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue");
            t.status(200).json_body(serde_json::json!({"ok": true, "suivante": null, "jeux": [{"id": 7, "titre": "Cyberpunk", "plateforme": "Windows"}]}));
        });
        let e = emplacement(r"%USERPROFILE%\Saved Games\CD Projekt Red\Cyberpunk 2077", "dossier", None);
        let a = EmplacementAbrite { partiel: true, ..emplacement(r"%USERPROFILE%\Saved Games\CD Projekt Red\Autre", "dossier", None) };
        let fe = format!("emplacements/{}/ManualSave-1/sav.dat", e.cle);
        let fa = format!("emplacements/{}/b.dat", a.cle);
        let f = fiche(vec![e.clone(), a.clone()], vec![(&fe, b"cyber"), (&fa, b"autre")]);
        let fiche_octets = serde_json::to_vec(&f).unwrap();
        let base = "/api/jeux/v1/sauvegarde/Seb/VENKMAN";
        let fichiers: Vec<(String, Vec<u8>)> = vec![
            (format!("jeux/7/{}", ap::FICHE_ENVOYEE), fiche_octets.clone()),
            (format!("jeux/7/{fe}"), b"cyber".to_vec()),
            (format!("jeux/7/{fa}"), b"autre".to_vec()),
            ("jeux/7/nuage-steam/m1/remote/x".into(), b"nuage".to_vec()),
            ("jeux/8/emplacements/0a1b2c3d/z".into(), b"autre jeu".to_vec()),
        ];
        let lignes: serde_json::Map<String, Value> = fichiers.iter().map(|(r, o)| (r.clone(), serde_json::json!([sha(o), o.len()]))).collect();
        s.mock(|w, t| {
            w.method(GET).path(format!("{base}/document/manifeste.json"));
            t.status(200).json_body(serde_json::json!({ "fichiers": lignes }));
        });
        s.mock(|w, t| {
            w.method(GET).path(format!("{base}/document/derniere-sauvegarde.json"));
            t.status(200).json_body(serde_json::json!({"date": "1700000000"}));
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/sauvegarde/Seb/_profil/document/abris-sel.json");
            t.status(200).json_body(serde_json::json!({"sel": SEL}));
        });
        let mut servis = Vec::new();
        for (r, o) in &fichiers {
            let (r, o) = (r.clone(), o.clone());
            servis.push(s.mock(move |w, t| {
                w.method(GET).path(format!("{base}/fichier")).query_param("chemin", r.clone());
                t.status(200).body(o.clone());
            }));
        }
        // X-Modifie seulement pour le fichier en conflit de Cyberpunk (contrat 1.52) ; pas pour l'autre.
        s.mock(|w, t| {
            w.method(httpmock::Method::HEAD).path(format!("{base}/fichier")).query_param("chemin", format!("jeux/7/{fe}"));
            t.status(200).header("X-Complet", "1").header("X-Modifie", "2026-11-24T10:53:09Z");
        });
        s.mock(|w, t| {
            w.method(httpmock::Method::HEAD).path(format!("{base}/fichier")).query_param("chemin", format!("jeux/7/{fa}"));
            t.status(200).header("X-Complet", "1");
        });
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &crate::noyau::Connexion { adresse: s.base_url(), simule: false }).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();

        let ap_ = n.apercu_parties_jeu("Seb", "VENKMAN", 7).await.unwrap();
        assert_eq!((ap_.fichiers, ap_.octets, ap_.jeux), (3, fiche_octets.len() as u64 + 10, 1));
        assert!(matches!(n.apercu_parties_jeu("Seb", "VENKMAN", 8).await, Err(Erreur::Introuvable(_))), "un jeu caché n'existe pas");
        let suivi = std::sync::Mutex::new(Vec::new());
        n.telecharger_parties_jeu("Seb", "VENKMAN", 7, &|f, t, _, _| suivi.lock().unwrap().push((f, t))).await.unwrap();
        assert_eq!(suivi.lock().unwrap().last(), Some(&(3, 3)));
        let zone = n.zone_restauration(&id).join("parties");
        assert!(zone.join("jeux/7/emplacements/fiche.json").is_file());
        assert!(!zone.join("jeux/7/nuage-steam").exists() && !zone.join("jeux/8").exists(), "jamais le nuage Steam ni un autre jeu");
        servis[3].assert_hits(0);
        servis[4].assert_hits(0);
        assert_eq!(ap::sel_local(&d.path().join("app").join("profils").join(&id)).as_deref(), Some(SEL), "le sel relu et rangé");

        // Le jeu installé, des parties DIFFÉRENTES déjà là pour les deux emplacements : deux conflits.
        let parties = d.path().join("Parties");
        let windows = vec![("Parties-enregistrees", parties.clone())];
        installer(&n, 7, &d.path().join("Jeux").join("Cyberpunk"));
        ecrire(&parties, "CD Projekt Red/Cyberpunk 2077/ManualSave-1/sav.dat", b"la mienne");
        ecrire(&parties, "CD Projekt Red/Autre/b.dat", b"la mienne aussi");
        let s2 = n.session().await.unwrap();
        let r = n.poser_parties_pc(&s2, Some(&[7]), &windows, false).unwrap();
        assert_eq!((r.conflits, r.partielles, r.posees), (2, 1, 0));
        let l = n.conflits_parties_pc(Some(&[7]), &windows).await.unwrap();
        let date = |cle: &str| l.iter().find(|c| c.cle == cle).and_then(|c| c.firehouse.date);
        assert_eq!(date(&e.cle), Some(1_795_517_589), "X-Modifie");
        assert_eq!(date(&a.cle), Some(1_700_000_000), "sans X-Modifie : derniere.date");
        assert!(l.iter().find(|c| c.cle == a.cle).unwrap().partiel);
    }

    /// Un profil simulé qui voit Dune (110), installé ; la zone d'attente et le dossier du jeu.
    async fn simule_avec_dune(d: &Path) -> (Noyau, String, PathBuf, PathBuf) {
        let n = Noyau::nouveau(&d.join("app"), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, None).unwrap().id;
        n.ouvrir(&id, None, &crate::noyau::Connexion { adresse: String::new(), simule: true }).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();
        let jeu = d.join("Jeux").join("Dune");
        installer(&n, 110, &jeu);
        let zone = n.zone_restauration(&id);
        (n, id, zone, jeu)
    }

    /// BLOQUANT de la revue : une reprise coupée au milieu (un fichier manque, ou n'a pas l'empreinte de la fiche) ne
    /// pose RIEN de l'emplacement et ne le note pas : il est posé, entier, quand la copie est complète.
    #[tokio::test]
    async fn une_copie_incomplete_n_est_jamais_posee_ni_notee() {
        let d = tempfile::tempdir().unwrap();
        let (n, _, zone, _) = simule_avec_dune(d.path()).await;
        let docs = d.path().join("Docs");
        let windows = vec![("Documents", docs.clone())];
        let e = emplacement(r"%USERPROFILE%\Documents\Cryo\Dune", "dossier", None);
        let (f1, f2) = (format!("emplacements/{}/a.sav", e.cle), format!("emplacements/{}/b.sav", e.cle));
        let attente = zone.join("parties").join("jeux").join("110");
        let f = fiche(vec![e.clone()], vec![(&f1, b"aaa"), (&f2, b"bbb")]);
        ecrire(&attente, ap::FICHE_ENVOYEE, &serde_json::to_vec(&f).unwrap());
        // Coupée : b.sav manque.
        ecrire(&attente, &f1, b"aaa");
        let s = n.session().await.unwrap();
        let r = n.poser_parties_pc(&s, None, &windows, false).unwrap();
        assert_eq!((r.posees, r.incompletes), (0, 1));
        assert!(!docs.exists(), "rien de posé");
        // b.sav arrivé abîmé (même taille, autre contenu) : toujours rien.
        ecrire(&attente, &f2, b"xxx");
        assert_eq!(n.poser_parties_pc(&s, None, &windows, false).unwrap().incompletes, 1);
        assert!(!docs.exists());
        assert!(lire_reglees(&zone).entrees.is_empty(), "jamais noté");
        // Entière : posée, en entier.
        ecrire(&attente, &f2, b"bbb");
        assert_eq!(n.poser_parties_pc(&s, None, &windows, false).unwrap().posees, 1);
        assert_eq!(std::fs::read(docs.join("Cryo").join("Dune").join("b.sav")).unwrap(), b"bbb");
    }

    /// Un téléchargement coupé au milieu ne laisse JAMAIS la fiche dans la zone d'attente (elle arrive en dernier).
    #[tokio::test]
    async fn une_reprise_coupee_ne_laisse_pas_de_fiche() {
        use httpmock::prelude::*;
        use sha2::{Digest, Sha256};
        let sha = |o: &[u8]| format!("{:x}", Sha256::digest(o));
        let s = MockServer::start();
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/plateformes");
            t.status(200).json_body(serde_json::json!({"ok": true, "plateformes": [{"nom": "Windows", "jeux": 1}]}));
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue");
            t.status(200).json_body(serde_json::json!({"ok": true, "suivante": null, "jeux": [{"id": 7, "titre": "Cyberpunk", "plateforme": "Windows"}]}));
        });
        let e = emplacement(r"%USERPROFILE%\Saved Games\Jeu", "dossier", None);
        let (ra, rb) = (format!("emplacements/{}/a.sav", e.cle), format!("emplacements/{}/b.sav", e.cle));
        let (fa, fb) = (format!("jeux/7/{ra}"), format!("jeux/7/{rb}"));
        let fiche_octets = serde_json::to_vec(&fiche(vec![e.clone()], vec![(&ra, b"a"), (&rb, b"b")])).unwrap();
        let rf = "jeux/7/emplacements/fiche.json".to_string();
        let base = "/api/jeux/v1/sauvegarde/Seb/PC";
        let mut lignes = serde_json::Map::new();
        lignes.insert(rf.clone(), serde_json::json!([sha(&fiche_octets), fiche_octets.len()]));
        lignes.insert(fa.clone(), serde_json::json!([sha(b"a"), 1]));
        lignes.insert(fb.clone(), serde_json::json!([sha(b"b"), 1]));
        s.mock(|w, t| {
            w.method(GET).path(format!("{base}/document/manifeste.json"));
            t.status(200).json_body(serde_json::json!({ "fichiers": lignes }));
        });
        s.mock(|w, t| {
            w.method(GET).path(format!("{base}/fichier")).query_param("chemin", fa.clone());
            t.status(200).body(b"a");
        });
        // La coupure : b.sav ne vient pas.
        s.mock(|w, t| {
            w.method(GET).path(format!("{base}/fichier")).query_param("chemin", fb.clone());
            t.status(503).json_body(serde_json::json!({"detail": "occupé"}));
        });
        let servie = s.mock(|w, t| {
            w.method(GET).path(format!("{base}/fichier")).query_param("chemin", rf.clone());
            t.status(200).body(fiche_octets.clone());
        });
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &crate::noyau::Connexion { adresse: s.base_url(), simule: false }).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();
        assert!(n.telecharger_parties_jeu("Seb", "PC", 7, &|_, _, _, _| {}).await.is_err());
        servie.assert_hits(0);
        assert!(!n.zone_restauration(&id).join("parties").join("jeux").join("7").join("emplacements").join("fiche.json").exists());
    }

    /// Un remplacement coupé au milieu : ce qui a déjà été remplacé est REMIS depuis l'abri d'avant (relu), et c'est dit.
    #[tokio::test]
    async fn un_remplacement_coupe_au_milieu_est_defait() {
        let d = tempfile::tempdir().unwrap();
        let (n, _, zone, _) = simule_avec_dune(d.path()).await;
        n.regler_abris(Some(d.path().join("Abris")));
        let docs = d.path().join("Docs");
        let windows = vec![("Documents", docs.clone())];
        let e = emplacement(r"%USERPROFILE%\Documents\Cryo\Dune", "dossier", None);
        let (f1, f2) = (format!("emplacements/{}/a.sav", e.cle), format!("emplacements/{}/b.sav", e.cle));
        en_attente(&zone, 110, vec![e.clone()], &[(f1, b"a de firehouse"), (f2, b"b de firehouse")]);
        let ici = docs.join("Cryo").join("Dune");
        ecrire(&ici, "a.sav", b"mon a");
        // b.sav est un DOSSIER ici : sa pose échoue après celle de a.sav.
        ecrire(&ici, "b.sav/dedans", b"d");
        match n.reprendre_partie_pc(110, &e.cle, &windows).await {
            Err(Erreur::Disque(m)) => assert!(m.contains("remise comme avant"), "{m}"),
            autre => panic!("{autre:?}"),
        }
        assert_eq!(std::fs::read(ici.join("a.sav")).unwrap(), b"mon a", "remise depuis l'abri");
        assert!(lire_reglees(&zone).entrees.is_empty(), "rien noté : la question reste");
    }

    /// Un abri d'avant impossible (aucun dossier des abris) : rien n'est remplacé.
    #[tokio::test]
    async fn sans_abri_d_avant_rien_n_est_remplace() {
        let d = tempfile::tempdir().unwrap();
        let (n, _, zone, _) = simule_avec_dune(d.path()).await;
        let docs = d.path().join("Docs");
        let windows = vec![("Documents", docs.clone())];
        let e = emplacement(r"%USERPROFILE%\Documents\Cryo\Dune", "dossier", None);
        en_attente(&zone, 110, vec![e.clone()], &[(format!("emplacements/{}/a.sav", e.cle), b"de firehouse")]);
        ecrire(&docs.join("Cryo").join("Dune"), "a.sav", b"le mien");
        assert!(n.reprendre_partie_pc(110, &e.cle, &windows).await.is_err());
        assert_eq!(std::fs::read(docs.join("Cryo").join("Dune").join("a.sav")).unwrap(), b"le mien");
    }

    /// Un programme du dossier du jeu qui tourne : rien n'est posé ni remplacé.
    #[cfg(windows)]
    #[tokio::test]
    async fn un_jeu_qui_tourne_ne_recoit_rien() {
        let d = tempfile::tempdir().unwrap();
        let (n, _, zone, jeu) = simule_avec_dune(d.path()).await;
        n.regler_abris(Some(d.path().join("Abris")));
        let docs = d.path().join("Docs");
        let windows = vec![("Documents", docs.clone())];
        let e = emplacement(r"%USERPROFILE%\Documents\Cryo\Dune", "dossier", None);
        en_attente(&zone, 110, vec![e.clone()], &[(format!("emplacements/{}/a.sav", e.cle), b"a")]);
        let systeme = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        std::fs::copy(Path::new(&systeme).join("System32").join("PING.EXE"), jeu.join("jeu.exe")).unwrap();
        let mut enfant = std::process::Command::new(jeu.join("jeu.exe")).args(["-n", "30", "127.0.0.1"]).stdout(std::process::Stdio::null()).spawn().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));
        let s = n.session().await.unwrap();
        let r = n.poser_parties_pc(&s, None, &windows, false);
        let rr = n.reprendre_partie_pc(110, &e.cle, &windows).await;
        let _ = enfant.kill();
        let _ = enfant.wait();
        assert_eq!(r.unwrap().en_cours, 1);
        assert!(rr.is_err());
        assert!(!docs.exists());
    }

    /// La fin d'une installation (`reposer_parties_restaurees`) pose ce qui attendait (ici dans le dossier du jeu).
    #[tokio::test]
    async fn la_fin_d_une_installation_pose_ce_qui_attendait() {
        let d = tempfile::tempdir().unwrap();
        let (n, _, zone, jeu) = simule_avec_dune(d.path()).await;
        let e = emplacement(r"<dossier du jeu>\SAVES", "dossier", None);
        en_attente(&zone, 110, vec![e.clone()], &[(format!("emplacements/{}/DUNE.SAV", e.cle), b"partie")]);
        let i = n.registre().jeu(110).unwrap().unwrap().installation.unwrap();
        n.reposer_parties_restaurees(110, &i).await;
        assert_eq!(std::fs::read(jeu.join("SAVES").join("DUNE.SAV")).unwrap(), b"partie");
    }

    /// Le nuage Steam n'est JAMAIS restauré : une fiche dont l'emplacement se résout dans
    /// `<Steam>\userdata\<compte>\<appid>` attend, rien n'y est écrit.
    #[test]
    fn rien_n_est_jamais_pose_dans_userdata_de_steam() {
        let d = tempfile::tempdir().unwrap();
        let steam = d.path().join("Steam");
        std::fs::create_dir_all(steam.join("userdata").join("12345").join("268500")).unwrap();
        let c = Contexte { jeu: None, steam: Some(steam.clone()), compte: Some("12345".into()), windows: vec![], installation: None, boutique: Some("steam".into()) };
        let e = emplacement(r"<Steam>\userdata\<identifiant Steam>\268500\remote", "dossier", None);
        let attente = d.path().join("attente");
        let f = fiche(vec![e.clone()], vec![(&format!("emplacements/{}/x.sav", e.cle), b"x")]);
        ecrire(&attente, &format!("emplacements/{}/x.sav", e.cle), b"x");
        assert!(matches!(examiner(&attente, &f, &e, &c, Some(SEL)), Examen::Attente(Attente::NonResolu)));
        assert!(!steam.join("userdata").join("12345").join("268500").join("remote").exists());
    }

    /// Un abri d'avant reprise plus récent que tout abri « restauree » : la sauvegarde n'envoie RIEN de ce jeu.
    #[test]
    fn un_abri_d_avant_reprise_sans_abri_d_apres_bloque_l_envoi() {
        let d = tempfile::tempdir().unwrap();
        let abri = |q: u64, declencheur: &str| {
            let dir = d.path().join("110").join(q.to_string());
            std::fs::create_dir_all(&dir).unwrap();
            let mut f = fiche(vec![emplacement(r"%USERPROFILE%\Documents\Jeu", "dossier", None)], vec![]);
            f.declencheur = declencheur.into();
            std::fs::write(ap::fichier_fiche(&dir).unwrap(), serde_json::to_vec(&f).unwrap()).unwrap();
            (q, dir)
        };
        let main = abri(10, ap::DECLENCHEUR_MAIN);
        let avant = abri(20, ap::DECLENCHEUR_AVANT_RESTAURATION);
        assert!(ap::reprise_sans_abri_apres(&[main.clone(), avant.clone()], "p1"));
        assert!(!ap::reprise_sans_abri_apres(&[main.clone(), avant.clone()], "autre"), "seulement ce profil");
        let apres = abri(21, ap::DECLENCHEUR_RESTAUREE);
        assert!(!ap::reprise_sans_abri_apres(&[main, avant, apres], "p1"));
    }

    /// Les `HEAD` de `X-Modifie` s'arrêtent au premier 401 : les conflits suivants gardent `derniere.date`.
    #[tokio::test]
    async fn les_head_s_arretent_au_premier_401() {
        use httpmock::prelude::*;
        let s = MockServer::start();
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/plateformes");
            t.status(200).json_body(serde_json::json!({"ok": true, "plateformes": [{"nom": "Windows", "jeux": 2}]}));
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue");
            t.status(200).json_body(serde_json::json!({"ok": true, "suivante": null, "jeux": [{"id": 7, "titre": "A", "plateforme": "Windows"}, {"id": 8, "titre": "B", "plateforme": "Windows"}]}));
        });
        let refus = s.mock(|w, t| {
            w.method(httpmock::Method::HEAD).path("/api/jeux/v1/sauvegarde/Seb/PC/fichier");
            t.status(401);
        });
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &crate::noyau::Connexion { adresse: s.base_url(), simule: false }).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();
        let zone = n.zone_restauration(&id);
        let docs = d.path().join("Docs");
        for (jeu, nom) in [(7, "A"), (8, "B")] {
            let chemin = format!(r"%USERPROFILE%\Documents\{nom}");
            let e = emplacement(&chemin, "dossier", None);
            en_attente(&zone, jeu, vec![e.clone()], &[(format!("emplacements/{}/x.sav", e.cle), b"firehouse")]);
            ecrire(&docs.join(nom), "x.sav", b"le mien");
            installer(&n, jeu, &d.path().join("Jeux").join(nom));
        }
        noter_origines(&zone, &[7, 8], &Origine { profil: "Seb".into(), pc: "PC".into(), date: Some("1700000000".into()) }).unwrap();
        let l = n.conflits_parties_pc(None, &[("Documents", docs.clone())]).await.unwrap();
        assert_eq!(l.len(), 2);
        assert!(l.iter().all(|c| c.firehouse.date == Some(1_700_000_000)));
        refus.assert_hits(1);
    }

    /// Lot S6 : des parties rangées DANS le dossier du jeu, de nature `motif` (RPG Maker VX Ace : `<dossier du
    /// jeu>\Save*.rvdata2`), se reprennent avec les règles du lot S5 : absent → posé, identique → rien, différent →
    /// conflit jamais écrasé ; un fichier hors du motif, ou plus bas que le dossier visé, n'est jamais posé.
    #[tokio::test]
    async fn une_partie_rangee_dans_le_dossier_du_jeu_par_motif_se_reprend() {
        let d = tempfile::tempdir().unwrap();
        let (n, _, zone, jeu) = simule_avec_dune(d.path()).await;
        let windows: Vec<(&'static str, PathBuf)> = vec![];
        let e = emplacement(r"<dossier du jeu>\Save*.rvdata2", "motif", None);
        let (s1, s2) = (format!("emplacements/{}/Save01.rvdata2", e.cle), format!("emplacements/{}/Save02.rvdata2", e.cle));
        en_attente(&zone, 110, vec![e.clone()], &[(s1, b"partie 1"), (s2, b"partie 2")]);
        // Une partie 2 déjà là, identique : seule la 1 manque.
        ecrire(&jeu, "Save02.rvdata2", b"partie 2");
        let s = n.session().await.unwrap();
        let r = n.poser_parties_pc(&s, None, &windows, false).unwrap();
        assert_eq!((r.posees, r.fichiers, r.conflits), (1, 1, 0));
        assert_eq!(std::fs::read(jeu.join("Save01.rvdata2")).unwrap(), b"partie 1", "posée dans le dossier du jeu");
        // Identique : rien.
        let attente = zone.join("parties").join("jeux").join("110");
        let f = fiche_en_attente(&attente).unwrap();
        let ctx = crate::commandes::contexte_des_parties(&n.dossier.join("profils").join(&s.profil.id), None, Some(jeu.clone()), None)(vec![]);
        assert!(matches!(examiner(&attente, &f, &e, &ctx, None), Examen::Resolu { statut: Statut::Identique, .. }));
        // La personne a rejoué : conflit, jamais écrasé.
        std::fs::write(jeu.join("Save01.rvdata2"), b"plus loin").unwrap();
        assert!(matches!(examiner(&attente, &f, &e, &ctx, None), Examen::Resolu { statut: Statut::Conflit(_), .. }));
        assert_eq!(std::fs::read(jeu.join("Save01.rvdata2")).unwrap(), b"plus loin");
        // Une fiche qui glisse un fichier hors du motif (les données du jeu) ou plus bas : rien de posé.
        for r in ["Map001.rvdata2", "Data/Save03.rvdata2"] {
            let x = fiche(vec![e.clone()], vec![(&format!("emplacements/{}/{r}", e.cle), b"x")]);
            ecrire(&attente, &format!("emplacements/{}/{r}", e.cle), b"x");
            assert!(matches!(examiner(&attente, &x, &e, &ctx, None), Examen::Attente(Attente::NonResolu)), "{r}");
        }
        assert!(!jeu.join("Map001.rvdata2").exists() && !jeu.join("Data").exists());
    }

    /// Lot S6 : Ren'Py écrit ses parties À DEUX ENDROITS (`%APPDATA%\RenPy\<dossier>` et `<dossier du jeu>\game\saves`) :
    /// les deux se reprennent.
    #[tokio::test]
    async fn les_deux_endroits_de_ren_py_se_reprennent() {
        let d = tempfile::tempdir().unwrap();
        let (n, _, zone, jeu) = simule_avec_dune(d.path()).await;
        let roaming = d.path().join("Roaming");
        let windows = vec![("AppData-Roaming", roaming.clone())];
        let a = emplacement(r"%APPDATA%\RenPy\MonJeu-1600000000", "dossier", None);
        let b = emplacement(r"<dossier du jeu>\game\saves", "dossier", None);
        en_attente(&zone, 110, vec![a.clone(), b.clone()], &[(format!("emplacements/{}/1-1-LT1.save", a.cle), b"p"), (format!("emplacements/{}/persistent", a.cle), b"q"), (format!("emplacements/{}/1-1-LT1.save", b.cle), b"p")]);
        let s = n.session().await.unwrap();
        assert_eq!(n.poser_parties_pc(&s, None, &windows, false).unwrap().posees, 2);
        assert_eq!(std::fs::read(roaming.join("RenPy").join("MonJeu-1600000000").join("persistent")).unwrap(), b"q");
        assert_eq!(std::fs::read(jeu.join("game").join("saves").join("1-1-LT1.save")).unwrap(), b"p");
    }

    /// Relecture S6 (essai réel) : `<dossier du jeu>` est, pour la reprise aussi, le dossier de l'exécutable quand il est
    /// dans celui de l'installation (une archive à sous-dossier).
    #[tokio::test]
    async fn la_reprise_pose_dans_le_dossier_de_l_executable() {
        let d = tempfile::tempdir().unwrap();
        let (n, _, zone, jeu) = simule_avec_dune(d.path()).await;
        let sous = jeu.join("Dune-1.0-pc");
        std::fs::create_dir_all(&sous).unwrap();
        let mut i = n.registre().jeu(110).unwrap().unwrap().installation.unwrap();
        i.lanceur = Some(crate::installation::Lanceur { programme: sous.join("dune.exe").to_string_lossy().into(), arguments: vec![], dossier: String::new() });
        n.registre().changer_installation(110, Some(&i)).unwrap();
        let e = emplacement(r"<dossier du jeu>\game\saves", "dossier", None);
        en_attente(&zone, 110, vec![e.clone()], &[(format!("emplacements/{}/1-1.save", e.cle), b"partie")]);
        let s = n.session().await.unwrap();
        assert_eq!(n.poser_parties_pc(&s, None, &[], false).unwrap().posees, 1);
        assert_eq!(std::fs::read(sous.join("game").join("saves").join("1-1.save")).unwrap(), b"partie");
        assert!(!jeu.join("game").exists(), "jamais à côté de l'archive");
    }
}
