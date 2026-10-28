//! Restaurer une sauvegarde (après un reformatage, ou sur un nouveau PC) : réglages, parties des émulateurs, et les
//! parties des jeux, reposées quand chaque jeu est réinstallé.
//!
//! Les fichiers sont d'abord téléchargés dans une zone d'attente (`restauration/<profil>/parties/`), chacun vérifié
//! par son empreinte (sha256). Rien de ce qui est déjà sur le PC n'est écrasé sans être identique, sauf les fichiers
//! d'un jeu qu'on vient de réinstaller (ce sont alors ceux du jeu tout neuf que la partie remplace).

use crate::erreurs::{Erreur, Resultat};
use crate::firehouse::Client;
use crate::installation::fichiers_de;
use crate::noyau::Noyau;
use crate::sauvegarde::{empreinte, nom_pour_api, ManifesteParties};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Une sauvegarde qu'on peut restaurer.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SauvegardeDisponible {
    pub profil: String,
    pub pc: String,
    /// Le contenu de `derniere-sauvegarde.json` (date, fichiers, taille…), s'il existe.
    pub derniere: Value,
    pub octets: u64,
    /// Rangée au nom du profil ouvert (même règle que l'envoi, `nom_pour_api`) : l'interface ne la recalcule jamais.
    #[serde(default)]
    pub a_moi: bool,
}

/// Marque les sauvegardes rangées au nom de ce profil (tel que Firehouse le range à l'envoi).
pub fn marquer_les_miennes(liste: &mut [SauvegardeDisponible], nom_du_profil: &str) {
    let moi = crate::sauvegarde::nom_pour_api(nom_du_profil);
    for s in liste {
        s.a_moi = s.profil == moi;
    }
}

/// Ce que la restauration a préparé.
#[derive(Debug, Clone, Serialize)]
pub struct RestaurationPrete {
    pub configuration: Value,
    pub bibliotheque: Value,
    pub fichiers: usize,
    pub taille: u64,
}

/// Ce que la restauration a remis en place.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct Reposes {
    pub reposes: usize,
    pub deja_la: usize,
    /// Fichiers différents déjà présents : gardés tels quels (la version restaurée reste en attente).
    pub conflits: Vec<String>,
    /// Les NOUVEAUX conflits d'une partie de la personne (dits une seule fois) : elle peut choisir de remplacer sa
    /// partie par la copie restaurée (✅ Seb 27/10 : sa partie actuelle mise à l'abri d'abord).
    pub a_remplacer: Vec<ConflitRestaure>,
}

impl Reposes {
    /// Ajoute le bilan `c` ; ses conflits seulement si `avec_conflits` (les parties communes à tous, déjà là et
    /// différentes, ne sont pas un souci).
    fn ajouter(&mut self, c: Reposes, avec_conflits: bool) {
        self.reposes += c.reposes;
        self.deja_la += c.deja_la;
        if avec_conflits {
            self.conflits.extend(c.conflits);
            self.a_remplacer.extend(c.a_remplacer);
        }
    }
}

/// Une partie restaurée en double : sa copie (dans la zone d'attente) et la partie en place, gardée.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ConflitRestaure {
    /// Sa clé dans le registre des parties réglées.
    pub cle: String,
    pub etiquette: String,
    pub copie: String,
    pub cible: String,
}

/// Le registre des parties d'émulateur DÉJÀ réglées (`restauration\<profil>\reglees.json`) : reposée, en conflit (dit
/// une fois), remplacée. Une entrée inscrite n'est plus retraitée tant que l'empreinte de sa copie ne change pas — une
/// partie que la personne a effacée ne revient donc plus (✅ Seb 27/10 : elle reste dans la sauvegarde). Rien n'est
/// effacé de la zone d'attente.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Reglees {
    /// La zone d'attente des émulateurs (`…\parties\emulateurs`) : les clés lui sont relatives. Vide : rien n'est suivi.
    #[serde(skip)]
    racine: PathBuf,
    /// Le dossier de l'émulateur en cours (pour vérifier qu'il est fermé avant un remplacement).
    #[serde(skip)]
    emulateur: PathBuf,
    #[serde(default)]
    pub entrees: BTreeMap<String, Reglee>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Reglee {
    /// « reposee », « conflit » ou « remplacee ».
    pub etat: String,
    /// L'empreinte de la COPIE restaurée (un fichier, ou un dossier entier).
    pub empreinte: String,
    pub cible: String,
    #[serde(default)]
    pub dossier: bool,
    #[serde(default)]
    pub etiquette: String,
    #[serde(default)]
    pub emulateur: String,
    /// Remplacée : où la partie qui était en place a été mise à l'abri.
    #[serde(default)]
    pub abri: String,
    /// Taille et date de la copie : inchangées, son empreinte n'est pas relue (des parties de plusieurs Go).
    #[serde(default)]
    pub signature: String,
}

/// La signature rapide d'une copie (fichier, ou dossier entier) : tailles et dates, sans lire les octets.
fn signature(p: &Path, dossier: bool) -> String {
    let une = |f: &Path| {
        std::fs::metadata(f).ok().map(|m| (m.len(), m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_nanos()).unwrap_or(0)))
    };
    if !dossier {
        return une(p).map(|(l, t)| format!("{l}:{t}")).unwrap_or_default();
    }
    let Ok(fichiers) = fichiers_de(p) else { return String::new() };
    let (mut total, mut derniere) = (0u64, 0u128);
    for f in &fichiers {
        let Some((l, t)) = une(&p.join(f)) else { return String::new() };
        total += l;
        derniere = derniere.max(t);
    }
    format!("{}:{total}:{derniere}", fichiers.len())
}

pub const REGLEES: &str = "reglees.json";

impl Reglees {
    /// Sans suivi (les parties des jeux, reposées une fois à l'installation).
    pub fn sans_suivi() -> Self {
        Self::default()
    }

    fn suivi(&self) -> bool {
        !self.racine.as_os_str().is_empty()
    }

    fn cle(&self, source: &Path) -> Option<String> {
        if !self.suivi() {
            return None;
        }
        source.strip_prefix(&self.racine).ok().map(|r| r.to_string_lossy().replace('\\', "/"))
    }

    /// Déjà réglée À CET ENDROIT, et la copie n'a pas changé depuis ? `None` : rien à faire. Sinon l'empreinte de la
    /// copie (vide sans suivi : l'appelant la calcule s'il en a besoin). Un émulateur réinstallé ailleurs (autre
    /// cible) reçoit de nouveau ses parties.
    fn examiner(&mut self, source: &Path, cible: &Path, dossier: bool, fichiers: Option<&[String]>) -> Resultat<Option<String>> {
        let Some(cle) = self.cle(source) else { return Ok(Some(String::new())) };
        let cible_txt = cible.to_string_lossy().to_string();
        let sig = signature(source, dossier);
        if let Some(e) = self.entrees.get(&cle) {
            if e.cible == cible_txt && !sig.is_empty() && e.signature == sig {
                return Ok(None);
            }
        }
        let empreinte_copie = match (dossier, fichiers) {
            (true, Some(f)) => empreinte_dossier(source, f)?,
            (true, None) => empreinte_dossier(source, &fichiers_de(source)?)?,
            (false, _) => empreinte(source)?,
        };
        if let Some(e) = self.entrees.get_mut(&cle) {
            if e.cible == cible_txt && e.empreinte == empreinte_copie {
                e.signature = sig;
                return Ok(None);
            }
        }
        Ok(Some(empreinte_copie))
    }

    fn noter(&mut self, source: &Path, etat: &str, empreinte: &str, cible: &Path, dossier: bool, etiquette: &str) -> Option<String> {
        let cle = self.cle(source)?;
        let emulateur = self.emulateur.to_string_lossy().to_string();
        let signature = signature(source, dossier);
        self.entrees.insert(
            cle.clone(),
            Reglee { etat: etat.into(), empreinte: empreinte.into(), cible: cible.to_string_lossy().into(), dossier, etiquette: etiquette.into(), emulateur, abri: String::new(), signature },
        );
        Some(cle)
    }

    /// Un conflit : noté (il ne sera plus redit) et proposé au remplacement.
    fn conflit(&mut self, source: &Path, empreinte: &str, cible: &Path, dossier: bool, etiquette: &str, r: &mut Reposes) {
        r.conflits.push(etiquette.to_string());
        if let Some(cle) = self.noter(source, "conflit", empreinte, cible, dossier, etiquette) {
            r.a_remplacer.push(ConflitRestaure { cle, etiquette: etiquette.into(), copie: source.to_string_lossy().into(), cible: cible.to_string_lossy().into() });
        }
    }
}

/// Le registre des parties réglées de cette zone d'attente (vide s'il n'existe pas, ou s'il est illisible : alors tout
/// est simplement réexaminé, jamais rien effacé).
pub fn lire_reglees(attente: &Path) -> Reglees {
    let mut r: Reglees = std::fs::read(attente.join(REGLEES)).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default();
    r.racine = attente.join("parties").join("emulateurs");
    r
}

fn ecrire_reglees(attente: &Path, r: &Reglees) -> Resultat<()> {
    let provisoire = attente.join(format!("{REGLEES}.frogtend"));
    std::fs::write(&provisoire, serde_json::to_vec_pretty(r).unwrap_or_default())?;
    std::fs::rename(&provisoire, attente.join(REGLEES))?;
    Ok(())
}

/// L'empreinte d'un dossier entier : ses chemins relatifs et l'empreinte de chaque fichier.
fn empreinte_dossier(d: &Path, fichiers: &[String]) -> Resultat<String> {
    use sha2::{Digest, Sha256};
    let mut tries = fichiers.to_vec();
    tries.sort();
    let mut h = Sha256::new();
    for f in &tries {
        h.update(f.as_bytes());
        h.update(b"\t");
        h.update(empreinte(&d.join(f))?.as_bytes());
        h.update(b"\n");
    }
    Ok(format!("{:x}", h.finalize()))
}

pub async fn lister(client: &Client) -> Resultat<Vec<SauvegardeDisponible>> {
    let v: Value = client.obtenir_json("/sauvegarde").await?;
    Ok(v["profils"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|p| {
            Some(SauvegardeDisponible {
                profil: p["profil"].as_str()?.into(),
                pc: p["pc"].as_str()?.into(),
                derniere: p["derniere"].clone(),
                octets: p["octets"].as_u64().unwrap_or(0),
                a_moi: false,
            })
        })
        .collect())
}

/// Télécharge un fichier de la sauvegarde (reprise `Range`), et vérifie son empreinte avant de le garder.
async fn telecharger(client: &Client, route: &str, cible: &Path, sha: &str, taille: u64) -> Resultat<()> {
    if cible.is_file() && std::fs::metadata(cible)?.len() == taille && empreinte(cible)? == sha {
        return Ok(()); // déjà là (restauration reprise)
    }
    if let Some(p) = cible.parent() {
        std::fs::create_dir_all(p)?;
    }
    let part = cible.with_extension("part-frogtend");
    let mut debut = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0).min(taille);
    let mut flux = match client.flux(route, debut).await {
        Ok(f) => f,
        // 416 : le morceau déjà reçu ne correspond plus ; ce fichier de Frogtend (refait de toute façon si son empreinte
        // est fausse) repart de zéro, une seule fois — sinon le même refus reviendrait à chaque essai.
        Err(Erreur::Plage(_)) if debut > 0 => {
            std::fs::remove_file(&part)?;
            debut = 0;
            client.flux(route, 0).await?
        }
        Err(e) => return Err(e),
    };
    let mut sortie = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(flux.reprise && debut > 0)
        .truncate(!(flux.reprise && debut > 0))
        .open(&part)?;
    while let Some(m) = Client::morceau(&mut flux).await? {
        sortie.write_all(&m)?;
    }
    sortie.flush()?;
    drop(sortie);
    if std::fs::metadata(&part)?.len() != taille || empreinte(&part)? != sha {
        let _ = std::fs::remove_file(&part);
        return Err(Erreur::Conflit(format!("« {} » est arrivé abîmé : relance la restauration.", cible.display())));
    }
    std::fs::rename(&part, cible)?;
    Ok(())
}

fn lire_json_ou_rien(p: &Path) -> Value {
    std::fs::read(p).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or(Value::Null)
}

/// Le nom sous lequel les parties d'un émulateur sont rangées dans la sauvegarde : celui de son dossier.
fn nom_du_dossier_de(programme: &str) -> Option<String> {
    let d = Path::new(programme).parent()?.file_name()?.to_string_lossy().to_string();
    Some(crate::jeux_pc::nom_de_dossier(&d))
}

/// Les programmes d'émulateurs de l'ancien PC, lus dans ses réglages restaurés (toute valeur `programme`, quelle que
/// soit la forme des réglages) : (nom de leur dossier, nom de leur programme en minuscules).
pub fn anciens_programmes(configuration: &Value) -> Vec<(String, String)> {
    fn chercher(v: &Value, l: &mut Vec<(String, String)>) {
        match v {
            Value::Object(m) => {
                for (k, x) in m {
                    if let (true, Some(p)) = (k == "programme", x.as_str()) {
                        let exe = p.rsplit(['\\', '/']).next().unwrap_or("").to_lowercase();
                        if let (Some(nom), false) = (nom_du_dossier_de(&p.replace('/', "\\")), exe.is_empty()) {
                            l.push((nom, exe));
                        }
                    } else {
                        chercher(x, l);
                    }
                }
            }
            Value::Array(a) => a.iter().for_each(|x| chercher(x, l)),
            _ => {}
        }
    }
    let mut l = Vec::new();
    chercher(&configuration["reglages_pc"], &mut l);
    l.sort();
    l.dedup();
    l
}

/// Le dossier d'attente des parties de CET émulateur : celui à son nom de dossier ; sinon celui de l'ancien émulateur
/// qui avait le même programme (`rpcs3.exe`), s'il est le SEUL (deux anciens RetroArch : on ne choisit pas).
pub fn cle_en_attente(zone: &Path, programme: &str, anciens: &[(String, String)]) -> Option<String> {
    cle_par_restaurations(zone, programme, &[anciens.to_vec()])
}

/// Pareil, avec les anciens émulateurs de PLUSIEURS restaurations (la dernière d'abord) : une restauration plus ancienne
/// n'est consultée que si la dernière ne connaît AUCUN candidat (jamais l'union : deux anciens RPCS3 seraient ambigus).
pub fn cle_par_restaurations(zone: &Path, programme: &str, restaurations: &[Vec<(String, String)>]) -> Option<String> {
    let nom = nom_du_dossier_de(programme)?;
    if zone.join(&nom).is_dir() {
        return Some(nom);
    }
    let exe = programme.rsplit(['\\', '/']).next()?.to_lowercase();
    for anciens in restaurations {
        let mut l: Vec<&String> = anciens.iter().filter(|(d, e)| *e == exe && zone.join(d).is_dir()).map(|(d, _)| d).collect();
        l.dedup();
        match l.as_slice() {
            [] => continue,
            [un] => return Some((*un).clone()),
            _ => return None,
        }
    }
    None
}

/// Les anciens émulateurs de chaque restauration gardée dans la zone d'attente, la dernière d'abord
/// (`configuration.json`, puis les `configuration-<date>.json` du plus récent au plus ancien).
pub fn anciens_des_restaurations(attente: &Path) -> Vec<Vec<(String, String)>> {
    let mut gardees: Vec<PathBuf> = std::fs::read_dir(attente)
        .map(|l| l.flatten().map(|e| e.path()).filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("configuration-") && n.ends_with(".json"))).collect())
        .unwrap_or_default();
    // L'instant dans le nom (`configuration-<secondes>[-<n>].json`) : du plus récent au plus ancien.
    let ordre = |p: &PathBuf| {
        let n = p.file_stem().and_then(|n| n.to_str()).unwrap_or("").trim_start_matches("configuration-").to_string();
        let mut m = n.splitn(2, '-');
        (m.next().and_then(|x| x.parse::<u64>().ok()).unwrap_or(0), m.next().and_then(|x| x.parse::<u32>().ok()).unwrap_or(1))
    };
    gardees.sort_by_key(|p| std::cmp::Reverse(ordre(p)));
    std::iter::once(attente.join("configuration.json")).chain(gardees).map(|p| anciens_programmes(&lire_json_ou_rien(&p))).collect()
}

/// Les dossiers d'attente de TOUS les émulateurs présents, chacun donné à UN SEUL émulateur (jamais les mêmes parties
/// copiées chez deux émulateurs qui divergeraient ensuite) : par le nom exact du dossier d'abord ; un rapprochement par
/// le programme seulement s'il est seul à le revendiquer. Dans le doute : personne.
pub fn cles_en_attente(zone: &Path, programmes: &[String], anciens: &[Vec<(String, String)>]) -> Vec<(String, String)> {
    let mut l: Vec<(String, String, bool)> = Vec::new();
    for p in programmes {
        let exact = nom_du_dossier_de(p).is_some_and(|n| zone.join(&n).is_dir());
        if let Some(c) = cle_par_restaurations(zone, p, anciens) {
            if !l.iter().any(|(x, _, _)| x == p) {
                l.push((p.clone(), c, exact));
            }
        }
    }
    l.iter()
        .filter(|(_, c, exact)| {
            let memes: Vec<&(String, String, bool)> = l.iter().filter(|(_, x, _)| x == c).collect();
            let exacts = memes.iter().filter(|(_, _, e)| *e).count();
            if *exact { exacts == 1 } else { exacts == 0 && memes.len() == 1 }
        })
        .map(|(p, c, _)| (p.clone(), c.clone()))
        .collect()
}

/// Copie les fichiers de `de` dans `vers` : un fichier absent est posé, un fichier identique compté, un fichier
/// DIFFÉRENT gardé tel quel (conflit) — sauf si `ecraser`.
pub fn reposer(de: &Path, vers: &Path, ecraser: bool) -> Resultat<Reposes> {
    reposer_sauf(de, vers, ecraser, &[], &mut Reglees::sans_suivi())
}

/// Les parties d'un compte RPCS3 (`savedata\<partie>\…`, `trophy\<jeu>\…`) : chaque partie est un DOSSIER de fichiers
/// qui vont ensemble (PARAM.SFO, SAVE.DAT, icônes ; TROPUSR.DAT et TROPTRNS.DAT). Elle est reposée ENTIÈRE si elle
/// n'est pas sur ce PC ; déjà là à l'identique, rien ; si elle diffère en quoi que ce soit, on n'y touche pas du tout
/// et c'est un conflit (`savedata/<partie>`) — jamais un mélange de deux parties.
pub fn reposer_parties_entieres(de: &Path, vers: &Path, reglees: &mut Reglees) -> Resultat<Reposes> {
    let mut r = Reposes::default();
    for sorte in crate::emulateurs_profils::PARTIES_RPCS3 {
        let c = reposer_dossiers_entiers(&de.join(sorte), &vers.join(sorte), sorte, reglees)?;
        r.ajouter(c, true);
    }
    Ok(r)
}

/// Chaque sous-dossier de `de` (une partie : ses fichiers vont ensemble) reposé ENTIER dans `vers` s'il n'y est pas ;
/// à l'identique, rien ; s'il diffère en quoi que ce soit, on n'y touche pas et c'est un conflit (`<etiquette>/<nom>`).
pub fn reposer_dossiers_entiers(de: &Path, vers: &Path, etiquette: &str, reglees: &mut Reglees) -> Resultat<Reposes> {
    let mut r = Reposes::default();
    for e in std::fs::read_dir(de).into_iter().flatten().flatten() {
        if !e.file_type().is_ok_and(|t| t.is_dir()) {
            continue;
        }
        let nom = e.file_name().to_string_lossy().to_string();
        reposer_un_dossier(&e.path(), &vers.join(&nom), &format!("{etiquette}/{nom}"), &mut r, reglees)?;
    }
    Ok(r)
}

/// Une partie (un dossier) reposée ENTIÈRE si elle n'est pas là (dossier provisoire vérifié puis renommé) ; à
/// l'identique, rien ; différente en quoi que ce soit : on n'y touche pas, conflit `etiquette`.
pub fn reposer_un_dossier(source: &Path, cible: &Path, etiquette: &str, r: &mut Reposes, reglees: &mut Reglees) -> Resultat<()> {
    let fichiers = fichiers_de(source)?;
    // Déjà réglée ici (reposée, dite en conflit, remplacée) : plus retraitée — même si la personne l'a effacée.
    let Some(e) = reglees.examiner(source, cible, true, Some(&fichiers))? else { return Ok(()) };
    if !cible.exists() {
        r.reposes += crate::eden::copier_dossier_entier(source, cible)?;
        reglees.noter(source, "reposee", &e, cible, true, etiquette);
        return Ok(());
    }
    let mut identique = fichiers_de(cible)? == fichiers;
    for f in &fichiers {
        if !identique {
            break;
        }
        identique = empreinte(&cible.join(f))? == empreinte(&source.join(f))?;
    }
    if identique {
        r.deja_la += fichiers.len();
        reglees.noter(source, "reposee", &e, cible, true, etiquette);
    } else {
        reglees.conflit(source, &e, cible, true, etiquette, r);
    }
    Ok(())
}

/// Comme [`reposer`], sans les sous-dossiers `sauf` (de premier niveau) : ils ont leur propre place (le compte RPCS3,
/// l'utilisateur Eden).
pub fn reposer_sauf(de: &Path, vers: &Path, ecraser: bool, sauf: &[&str], reglees: &mut Reglees) -> Resultat<Reposes> {
    let mut r = Reposes::default();
    if !de.is_dir() {
        return Ok(r);
    }
    let a_part: Vec<String> = sauf.iter().map(|s| format!("{s}/")).collect();
    for f in fichiers_de(de)?.into_iter().filter(|f| !a_part.iter().any(|a| f.starts_with(a))) {
        let (source, cible) = (de.join(&f), vers.join(&f));
        // Déjà réglée ici : plus retraitée (une partie effacée par la personne ne revient pas).
        let Some(mut es) = reglees.examiner(&source, &cible, false, None)? else { continue };
        if cible.is_file() {
            if es.is_empty() {
                es = empreinte(&source)?;
            }
            if empreinte(&cible)? == es {
                r.deja_la += 1;
                reglees.noter(&source, "reposee", &es, &cible, false, &f);
                continue;
            }
            if !ecraser {
                reglees.conflit(&source, &es, &cible, false, &f, &mut r);
                continue;
            }
        }
        if let Some(p) = cible.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::copy(&source, &cible)?;
        // Relue avant d'être notée « reposée » (une copie coupée serait réessayée).
        if reglees.suivi() && empreinte(&cible)? != es {
            return Err(Erreur::Disque(format!("La partie « {f} » est abîmée après sa copie : elle sera réessayée.")));
        }
        reglees.noter(&source, "reposee", &es, &cible, false, &f);
        r.reposes += 1;
    }
    Ok(r)
}

impl Noyau {
    fn zone_restauration(&self, profil_id: &str) -> PathBuf {
        self.dossier.join("restauration").join(profil_id)
    }

    /// Les sauvegardes du compte de ce jeton, chez Firehouse.
    pub async fn sauvegardes_disponibles(&self) -> Resultat<Vec<SauvegardeDisponible>> {
        let s = self.session().await?;
        match &s.source {
            crate::source::Source::Firehouse(c) => {
                let mut l = lister(c).await?;
                marquer_les_miennes(&mut l, &s.profil.nom);
                Ok(l)
            }
            crate::source::Source::Simulee => Ok(vec![]),
        }
    }

    /// Télécharge une sauvegarde (`profil` et `pc` tels que Firehouse les liste) dans la zone d'attente du profil
    /// ouvert. `progres(fichiers faits, fichiers en tout)`.
    pub async fn preparer_restauration(
        &self,
        profil: &str,
        pc: &str,
        progres: &(dyn Fn(usize, usize) + Send + Sync),
    ) -> Resultat<RestaurationPrete> {
        let s = self.session().await?;
        let crate::source::Source::Firehouse(c) = &s.source else {
            return Err(Erreur::Refus("En mode simulé, il n'y a rien à restaurer.".into()));
        };
        let (p, m) = (nom_pour_api(profil), nom_pour_api(pc));
        let base = format!("/sauvegarde/{}/{}", crate::source::encoder(&p), crate::source::encoder(&m));
        let document = |nom: &'static str| {
            let route = format!("{base}/document/{nom}");
            async move { c.obtenir_json::<Value>(&route).await }
        };
        let configuration = document("configuration.json").await?;
        let bibliotheque = document("bibliotheque.json").await?;
        let manifeste: ManifesteParties = serde_json::from_value(document("manifeste.json").await?)
            .map_err(|_| Erreur::Serveur("Le manifeste de la sauvegarde est illisible.".into()))?;

        // La liste des jeux restaurés, gardée sur le PC : une sauvegarde suivante la garde tant qu'ils ne sont pas remis.
        // Et les réglages : ils disent où étaient les émulateurs de l'ancien PC (pour leur rendre leurs parties).
        let attente = self.zone_restauration(&s.profil.id);
        std::fs::create_dir_all(&attente)?;
        // Une NOUVELLE restauration (demandée par la personne) remet tout en jeu : l'ancien registre est gardé à côté.
        if attente.join(REGLEES).is_file() {
            std::fs::rename(attente.join(REGLEES), attente.join(format!("reglees-{}.json", crate::noyau::maintenant().replace(':', "-"))))?;
        }
        // Les réglages de la restauration d'avant sont gardés à côté : ses parties encore en attente, rangées sous un
        // ancien nom de dossier, retrouvent leur émulateur par le programme (`anciens_des_restaurations`).
        if attente.join("configuration.json").is_file() {
            // Deux préparations dans la même seconde : « -2 », « -3 »… (jamais écrasée, jamais perdue).
            let instant = crate::noyau::maintenant().replace(':', "-");
            let garde = (1..)
                .map(|i| attente.join(if i == 1 { format!("configuration-{instant}.json") } else { format!("configuration-{instant}-{i}.json") }))
                .find(|p| !p.exists())
                .unwrap_or_default();
            std::fs::rename(attente.join("configuration.json"), garde)?;
        }
        std::fs::write(attente.join("bibliotheque.json"), serde_json::to_vec_pretty(&bibliotheque).unwrap_or_default())?;
        std::fs::write(attente.join("configuration.json"), serde_json::to_vec_pretty(&configuration).unwrap_or_default())?;
        let zone = attente.join("parties");
        let total = manifeste.fichiers.len();
        let mut taille = 0;
        for (i, (relatif, (sha, t))) in manifeste.fichiers.iter().enumerate() {
            // Un chemin de la sauvegarde ne sort jamais de la zone d'attente.
            if relatif.split('/').any(|s| s == ".." || s.is_empty()) || relatif.contains(['\\', ':']) {
                return Err(Erreur::Refus(format!("Chemin refusé dans la sauvegarde : « {relatif} ».")));
            }
            let route = format!("{base}/fichier?chemin={}", crate::source::encoder(relatif));
            telecharger(c, &route, &zone.join(relatif), sha, *t).await?;
            taille += t;
            progres(i + 1, total);
        }
        Ok(RestaurationPrete { configuration, bibliotheque, fichiers: total, taille })
    }

    /// Repose les parties des émulateurs (en attente) dans le dossier du profil de chaque émulateur présent. Elles sont
    /// rangées sous le nom du DOSSIER de l'émulateur de l'ancien PC ; s'il a changé (installé à la main, sous-dossier
    /// numéroté…), elles sont retrouvées par le nom de son programme (`rpcs3.exe`), seulement s'il n'y a aucun doute.
    /// `programmes_emulateurs` : les programmes des émulateurs réglés sur ce PC.
    pub async fn reposer_parties_emulateurs(&self, programmes_emulateurs: &[String]) -> Resultat<Reposes> {
        let s = self.session().await?;
        let attente = self.zone_restauration(&s.profil.id);
        let zone = attente.join("parties").join("emulateurs");
        // Les programmes des émulateurs de l'ANCIEN PC (réglages restaurés) : leur dossier → leur programme.
        let anciens = anciens_des_restaurations(&attente);
        let mut reglees = lire_reglees(&attente);
        let r = self.reposer_avec(&s, &zone, programmes_emulateurs, &anciens, &mut reglees);
        // Écrit même après une erreur : ce qui a été réglé avant ne sera pas redit (l'erreur d'origine passe d'abord).
        let ecrit = if attente.is_dir() { ecrire_reglees(&attente, &reglees) } else { Ok(()) };
        let r = r?;
        ecrit?;
        Ok(r)
    }

    fn reposer_avec(
        &self,
        s: &crate::noyau::Session,
        zone: &Path,
        programmes_emulateurs: &[String],
        anciens: &[Vec<(String, String)>],
        reglees: &mut Reglees,
    ) -> Resultat<Reposes> {
        let mut total = Reposes::default();
        for (programme, nom) in cles_en_attente(zone, programmes_emulateurs, anciens) {
            let Some(dossier) = Path::new(&programme).parent() else { continue };
            reglees.emulateur = dossier.to_path_buf();
            let mut r = reposer_sauf(
                &zone.join(&nom),
                &crate::emulateurs_profils::dossier_du_profil(dossier, &s.profil.nom),
                false,
                &[crate::emulateurs_profils::COMPTE_RPCS3, crate::eden::UTILISATEUR_EDEN, crate::cemu::COMPTE_CEMU, crate::vita3k::UTILISATEUR_VITA],
                reglees,
            )?;
            // Eden : les parties de l'utilisateur Switch du profil reviennent dans SON utilisateur (créé s'il manque,
            // AVANT les parties : sinon Eden les prendrait pour celles d'un utilisateur disparu) — Eden portable et fermé
            // seulement ; sinon elles attendent (gardées chez Firehouse).
            // Cemu : les parties du compte Wii U du profil reviennent dans SON compte (créé s'il manque), jeu par jeu ;
            // les données communes du jeu (`user\common`) seulement si elles manquent — Cemu portable et fermé.
            let attente_cemu = zone.join(&nom).join(crate::cemu::COMPTE_CEMU);
            if attente_cemu.is_dir()
                && crate::cemu::est_cemu(&programme)
                && crate::cemu::portable(dossier)
                && crate::lancement::processus_du_dossier(dossier).is_empty()
            {
                match crate::cemu::compte_du_profil(dossier, &crate::jeux_pc::nom_de_dossier(&s.profil.nom)) {
                    Ok(c) => {
                        for haut in std::fs::read_dir(&attente_cemu).into_iter().flatten().flatten() {
                            for bas in std::fs::read_dir(haut.path()).into_iter().flatten().flatten() {
                                let jeu = format!("{}/{}", haut.file_name().to_string_lossy(), bas.file_name().to_string_lossy());
                                let du_compte = crate::cemu::dossier_du_jeu(dossier, &jeu, c.id);
                                let commun = du_compte.parent().map(|u| u.join("common")).unwrap_or_default();
                                for (sous, vers) in [("compte", du_compte), ("common", commun)] {
                                    let de = bas.path().join(sous);
                                    if !de.is_dir() {
                                        continue;
                                    }
                                    // Les données communes du jeu : déjà là, différentes (une autre personne a joué ici)
                                    // — gardées telles quelles, sans le compter comme un souci.
                                    let mut c = Reposes::default();
                                    reposer_un_dossier(&de, &vers, &format!("Wii U : {jeu} ({sous})"), &mut c, reglees)?;
                                    r.ajouter(c, sous == "compte");
                                }
                            }
                        }
                    }
                    Err(e) => r.conflits.push(format!("Wii U : {e}")),
                }
            }
            // Vita3K : les parties de l'utilisateur Vita du profil reviennent chez lui (créé s'il manque), jeu par jeu —
            // Vita3K portable et fermé.
            let attente_vita = zone.join(&nom).join(crate::vita3k::UTILISATEUR_VITA).join("savedata");
            if attente_vita.is_dir()
                && crate::vita3k::est_vita3k(&programme)
                && crate::vita3k::portable(dossier)
                && crate::lancement::processus_du_dossier(dossier).is_empty()
            {
                match crate::vita3k::utilisateur_du_profil(dossier, &crate::jeux_pc::nom_de_dossier(&s.profil.nom)) {
                    Ok(u) => {
                        let c = reposer_dossiers_entiers(&attente_vita, &crate::vita3k::parties(dossier, &u.id), "PS Vita", reglees)?;
                        r.ajouter(c, true);
                    }
                    Err(e) => r.conflits.push(format!("PS Vita : {e}")),
                }
            }
            let attente_eden = zone.join(&nom).join(crate::eden::UTILISATEUR_EDEN);
            if attente_eden.is_dir()
                && crate::eden::est_eden(&programme)
                && crate::eden::portable(dossier)
                && crate::lancement::processus_du_dossier(dossier).is_empty()
            {
                // Un échec (8 utilisateurs, fichier abîmé) est DIT et n'arrête pas les autres émulateurs ; les parties
                // attendent (gardées chez Firehouse).
                match crate::eden::utilisateur_du_profil(dossier, &crate::jeux_pc::nom_de_dossier(&s.profil.nom)) {
                    Ok(u) => {
                        for (sorte, vers) in crate::eden::dossiers_parties(dossier, &u.uuid) {
                            let etiquette = if sorte == "save" { "Switch" } else { "Switch (compte)" };
                            let c = reposer_dossiers_entiers(&attente_eden.join(sorte), &vers, etiquette, reglees)?;
                            r.ajouter(c, true);
                        }
                        // Les parties « Device » (communes) : remises si elles manquent ; déjà là et différentes (une
                        // autre personne a joué ici), gardées telles quelles, sans le compter comme un souci.
                        let c = reposer_dossiers_entiers(&attente_eden.join("device"), &crate::eden::dossier_device(dossier), "Switch (commun)", reglees)?;
                        let communes = c.conflits.len();
                        r.ajouter(c, false);
                        if communes > 0 {
                            r.conflits.push(format!("Switch : {communes} partie(s) commune(s) déjà présente(s) ici et différente(s), gardée(s) telle(s) quelle(s)"));
                        }
                    }
                    Err(e) => r.conflits.push(format!("Switch : {e}")),
                }
            }
            // RPCS3 : les parties du compte RPCS3 du profil reviennent dans SON compte sur ce PC (recréé si besoin, son
            // numéro peut différer de l'ancien PC) ; jamais par-dessus une partie différente.
            let compte = zone.join(&nom).join(crate::emulateurs_profils::COMPTE_RPCS3);
            if compte.is_dir() && crate::emulateurs_profils::est_rpcs3(&programme) {
                let numero = crate::emulateurs_profils::compte_rpcs3(dossier, &crate::jeux_pc::nom_de_dossier(&s.profil.nom))?;
                let mut c = reposer_parties_entieres(&compte, &dossier.join("dev_hdd0").join("home").join(numero), reglees)?;
                // Dit simplement : « PS3 : partie BCES00569-GAME ».
                c.conflits = c.conflits.into_iter().map(|f| format!("PS3 : {f}")).collect();
                for a in &mut c.a_remplacer {
                    a.etiquette = format!("PS3 : {}", a.etiquette);
                }
                r.ajouter(c, true);
            }
            r.conflits = r.conflits.into_iter().map(|f| format!("{nom}/{f}")).collect();
            for a in &mut r.a_remplacer {
                a.etiquette = format!("{nom}/{}", a.etiquette);
            }
            total.ajouter(r, true);
        }
        Ok(total)
    }

    /// Les parties restaurées encore en double (une proposition fermée sans répondre n'est jamais perdue).
    pub async fn conflits_restaures(&self) -> Resultat<Vec<ConflitRestaure>> {
        let s = self.session().await?;
        let r = lire_reglees(&self.zone_restauration(&s.profil.id));
        Ok(r.entrees
            .iter()
            .filter(|(_, e)| e.etat == "conflit")
            .map(|(cle, e)| ConflitRestaure { cle: cle.clone(), etiquette: e.etiquette.clone(), copie: r.racine.join(cle).to_string_lossy().into(), cible: e.cible.clone() })
            .collect())
    }

    /// ✅ Seb 27/10 : une partie restaurée en double REMPLACE la partie en place, à la demande de la personne — la
    /// partie en place est d'abord COPIÉE à l'abri (`restauration\<profil>\remplacees\<date>\…`) et relue identique ;
    /// l'émulateur fermé ; la copie restaurée relue inchangée avant, et la partie posée relue après. Rend l'abri.
    pub async fn remplacer_par_restauree(&self, cle: &str) -> Resultat<String> {
        let s = self.session().await?;
        let attente = self.zone_restauration(&s.profil.id);
        let mut reglees = lire_reglees(&attente);
        let e = reglees.entrees.get(cle).cloned().filter(|e| e.etat == "conflit").ok_or_else(|| Erreur::Introuvable("Cette partie en double n'est plus en attente.".into()))?;
        if cle.split('/').any(|x| x == ".." || x.is_empty()) {
            return Err(Erreur::Refus("Chemin refusé.".into()));
        }
        let (source, cible) = (reglees.racine.join(cle), PathBuf::from(&e.cible));
        // Jamais écrire hors du dossier de l'émulateur (le registre est un fichier qu'on relit).
        if e.emulateur.is_empty() || !cible.starts_with(&e.emulateur) || e.cible.contains("..") {
            return Err(Erreur::Refus("Cette partie n'est pas dans le dossier d'un émulateur : rien n'est remplacé.".into()));
        }
        if !crate::lancement::processus_du_dossier(Path::new(&e.emulateur)).is_empty() {
            return Err(Erreur::Refus("L'émulateur est ouvert : ferme-le (ou quitte ta partie), puis recommence.".into()));
        }
        let abri = attente.join("remplacees").join(crate::noyau::maintenant().replace(':', "-")).join(cle);
        let (de, vers, ab, dossier, attendue) = (source.clone(), cible.clone(), abri.clone(), e.dossier, e.empreinte.clone());
        tokio::task::spawn_blocking(move || -> Resultat<()> {
            let empreinte_de = |p: &Path| -> Resultat<String> { if dossier { empreinte_dossier(p, &fichiers_de(p)?) } else { empreinte(p) } };
            // La copie restaurée n'a pas changé depuis le conflit.
            if empreinte_de(&de)? != attendue {
                return Err(Erreur::Refus("La copie restaurée a changé depuis : rien n'est remplacé.".into()));
            }
            // La partie en place, d'abord à l'abri, relue identique.
            if vers.exists() {
                let avant = empreinte_de(&vers)?;
                if dossier {
                    crate::eden::copier_dossier_entier(&vers, &ab)?;
                } else {
                    if let Some(p) = ab.parent() {
                        std::fs::create_dir_all(p)?;
                    }
                    std::fs::copy(&vers, &ab)?;
                }
                if empreinte_de(&ab)? != avant {
                    return Err(Erreur::Disque("La mise à l'abri de ta partie a échoué : rien n'est remplacé.".into()));
                }
            }
            // Puis la copie restaurée à sa place, relue.
            if dossier {
                // L'ancienne partie mise de côté (renommée), la restaurée posée, puis seulement l'ancienne retirée
                // (déjà à l'abri, relue identique) : la place n'est jamais vide, un échec remet tout comme avant.
                let mise_de_cote = PathBuf::from(format!("{}.frogtend-ancien", vers.to_string_lossy()));
                if mise_de_cote.exists() {
                    return Err(Erreur::Refus("Un remplacement précédent n'est pas fini : rien n'est remplacé.".into()));
                }
                let avait = vers.exists();
                if avait {
                    std::fs::rename(&vers, &mise_de_cote)?;
                }
                if let Err(e) = crate::eden::copier_dossier_entier(&de, &vers) {
                    if avait {
                        let _ = std::fs::rename(&mise_de_cote, &vers);
                    }
                    return Err(e);
                }
                if avait {
                    std::fs::remove_dir_all(&mise_de_cote)?;
                }
            } else {
                let provisoire = PathBuf::from(format!("{}.frogtend", vers.to_string_lossy()));
                std::fs::copy(&de, &provisoire)?;
                std::fs::rename(&provisoire, &vers)?;
            }
            if empreinte_de(&vers)? != attendue {
                return Err(Erreur::Disque("La partie restaurée est abîmée après sa pose : ta partie d'avant est à l'abri.".into()));
            }
            Ok(())
        })
        .await
        .map_err(|_| Erreur::Disque("Le remplacement s'est arrêté brutalement.".into()))??;
        if let Some(x) = reglees.entrees.get_mut(cle) {
            x.etat = "remplacee".into();
            x.abri = abri.to_string_lossy().into();
        }
        ecrire_reglees(&attente, &reglees)?;
        self.journaliser(&format!("restauration : partie remplacée par la copie restaurée ({}), l'ancienne à l'abri", e.etiquette));
        Ok(abri.to_string_lossy().into())
    }

    /// Juste après l'installation d'un jeu : si ses parties attendent d'être restaurées, elles sont reposées dans son
    /// dossier (elles remplacent les fichiers du jeu tout neuf), puis retirées de la zone d'attente.
    /// `anciens` : les anciens numéros de ce jeu connus du profil (fiches réunies), en plus de ceux du registre.
    pub(crate) fn reposer_parties_du_jeu(&self, profil_id: &str, id: i64, anciens: &[i64], dossier: &Path) -> Resultat<Reposes> {
        // Les parties attendues sous ce numéro, et sous les ANCIENS numéros de ce jeu (fiches réunies chez Firehouse,
        // contrat 1.14 : une sauvegarde faite avant le rattachement).
        // Les anciens d'abord, le nouveau en dernier : la partie la plus récente (sous le nouveau numéro) l'emporte.
        let mut numeros: Vec<i64> = self.registre().reunions()?.into_iter().filter(|(_, n)| *n == id).map(|(a, _)| a).collect();
        numeros.extend(anciens.iter().copied().filter(|a| *a != id));
        numeros.sort();
        numeros.dedup();
        numeros.push(id);
        let mut r = Reposes::default();
        for n in numeros {
            let attente = self.zone_restauration(profil_id).join("parties").join("jeux").join(n.to_string());
            if !attente.is_dir() {
                continue;
            }
            let x = reposer(&attente, dossier, true)?;
            std::fs::remove_dir_all(&attente)?;
            r.reposes += x.reposes;
            r.deja_la += x.deja_la;
            r.conflits.extend(x.conflits);
        }
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use serde_json::json;

    #[test]
    fn seules_les_sauvegardes_au_nom_du_profil_sont_les_siennes() {
        let s = |profil: &str| SauvegardeDisponible { profil: profil.into(), pc: "VENKMAN".into(), derniere: Value::Null, octets: 0, a_moi: true };
        let mut l = vec![s("Sebastien"), s("Lea _ Zoe"), s("Seb")];
        marquer_les_miennes(&mut l, "Sébastien");
        assert_eq!(l.iter().map(|x| x.a_moi).collect::<Vec<_>>(), [true, false, false]);
        // La même règle qu'à l'envoi : « Seb. » est rangé « Seb » (le calcul de l'interface ne le savait pas).
        marquer_les_miennes(&mut l, "Seb.");
        assert_eq!(l.iter().map(|x| x.a_moi).collect::<Vec<_>>(), [false, false, true]);
        marquer_les_miennes(&mut l, "Léa & Zoé");
        assert!(l[1].a_moi && !l[0].a_moi);
    }

    #[test]
    fn les_parties_d_un_emulateur_reinstalle_ailleurs_le_retrouvent_par_son_programme() {
        let d = tempfile::tempdir().unwrap();
        let zone = d.path();
        for n in ["rpcs3-v0.0.38-win64", "Cemu_2.5", "RetroArch", "RetroArch-ancien"] {
            std::fs::create_dir_all(zone.join(n)).unwrap();
        }
        // Les réglages de l'ancien PC (toutes formes : liste par système, programme seul).
        let conf = json!({"reglages_pc": {"reglages": {"emulateurs": {
            "Sony Playstation 3": {"defaut": "1", "liste": [{"cle": "1", "programme": "D:\\Emus\\rpcs3-v0.0.38-win64\\rpcs3.exe"}]},
            "Nintendo Wii U": {"programme": "E:/Emus/Cemu_2.5/Cemu.exe"},
            "SNES": {"liste": [{"programme": "D:\\RetroArch\\retroarch.exe"}, {"programme": "D:\\RetroArch-ancien\\retroarch.exe"}]}
        }}}});
        let a = anciens_programmes(&conf);
        assert_eq!(a.len(), 4, "{a:?}");
        // Même nom de dossier : tel quel.
        assert_eq!(cle_en_attente(zone, "C:\\Frogtend\\Emulateurs\\RetroArch\\retroarch.exe", &a).as_deref(), Some("RetroArch"));
        // Réinstallé par Frogtend dans « RPCS3 » : retrouvé par rpcs3.exe ; Cemu dans un autre sous-dossier numéroté.
        assert_eq!(cle_en_attente(zone, "C:\\Frogtend\\Emulateurs\\RPCS3\\rpcs3.exe", &a).as_deref(), Some("rpcs3-v0.0.38-win64"));
        assert_eq!(cle_en_attente(zone, "C:\\Emus\\Cemu\\Cemu_2.6\\cemu.exe", &a).as_deref(), Some("Cemu_2.5"));
        // Deux anciens RetroArch et un nouveau nom : jamais deviné.
        assert_eq!(cle_en_attente(zone, "C:\\Emus\\RA\\retroarch.exe", &a), None);
        assert_eq!(cle_en_attente(zone, "C:\\Emus\\Dolphin\\Dolphin.exe", &a), None);

        // Plusieurs émulateurs présents : un dossier d'attente à UN seul. Deux RPCS3 pour un seul ancien : personne ;
        // RetroArch au même nom + un autre retroarch.exe : le nom exact seulement.
        let p = |s: &str| s.to_string();
        let c = cles_en_attente(
            zone,
            &[p("C:\\E\\RPCS3\\rpcs3.exe"), p("D:\\rpcs3-test\\rpcs3.exe"), p("C:\\E\\RetroArch\\retroarch.exe"), p("C:\\E\\Cemu\\Cemu.exe")],
            &[a.clone()],
        );
        // Deux restaurations de suite : la dernière ne connaît pas RPCS3 → la précédente le retrouve ; la dernière
        // le connaît (même ambigu) → la précédente n'est jamais consultée.
        let sans_rpcs3: Vec<(String, String)> = a.iter().filter(|(_, e)| e != "rpcs3.exe").cloned().collect();
        assert_eq!(cle_par_restaurations(zone, "C:\\F\\RPCS3\\rpcs3.exe", &[sans_rpcs3, a.clone()]).as_deref(), Some("rpcs3-v0.0.38-win64"));
        assert_eq!(cle_par_restaurations(zone, "C:\\Emus\\RA\\retroarch.exe", &[a.clone(), vec![(p("RetroArch-ancien"), p("retroarch.exe"))]]), None);
        assert_eq!(c, vec![(p("C:\\E\\RetroArch\\retroarch.exe"), p("RetroArch")), (p("C:\\E\\Cemu\\Cemu.exe"), p("Cemu_2.5"))]);
    }

    #[test]
    fn les_reglages_des_restaurations_gardees_sont_lus_de_la_derniere_a_la_plus_ancienne() {
        let d = tempfile::tempdir().unwrap();
        let conf = |prog: &str| json!({"reglages_pc": {"reglages": {"emulateurs": {"X": {"programme": prog}}}}}).to_string();
        std::fs::write(d.path().join("configuration.json"), conf("D:\\C\\c.exe")).unwrap();
        // Les noms que `maintenant()` donne (secondes), dont deux préparations dans la même seconde.
        std::fs::write(d.path().join("configuration-999999999.json"), conf("D:\\Z\\z.exe")).unwrap();
        std::fs::write(d.path().join("configuration-1760000000.json"), conf("D:\\A\\a.exe")).unwrap();
        std::fs::write(d.path().join("configuration-1761000000.json"), conf("D:\\B\\b.exe")).unwrap();
        std::fs::write(d.path().join("configuration-1761000000-2.json"), conf("D:\\B2\\b2.exe")).unwrap();
        let l = anciens_des_restaurations(d.path());
        let exes: Vec<&str> = l.iter().map(|g| g[0].1.as_str()).collect();
        assert_eq!(exes, ["c.exe", "b2.exe", "b.exe", "a.exe", "z.exe"]);
        // Aucune restauration : une génération vide, rien d'autre.
        let vide = tempfile::tempdir().unwrap();
        assert_eq!(anciens_des_restaurations(vide.path()), vec![Vec::<(String, String)>::new()]);
    }

    #[test]
    fn reposer_ne_remplace_jamais_une_partie_differente_sans_le_dire() {
        let d = tempfile::tempdir().unwrap();
        let (de, vers) = (d.path().join("de"), d.path().join("vers"));
        std::fs::create_dir_all(de.join("saves")).unwrap();
        std::fs::create_dir_all(vers.join("saves")).unwrap();
        std::fs::write(de.join("saves/a.srm"), b"A").unwrap();
        std::fs::write(de.join("saves/b.srm"), b"B restauree").unwrap();
        std::fs::write(de.join("saves/c.srm"), b"C").unwrap();
        std::fs::write(vers.join("saves/b.srm"), b"B jouee sur ce PC").unwrap();
        std::fs::write(vers.join("saves/c.srm"), b"C").unwrap();

        let r = reposer(&de, &vers, false).unwrap();
        assert_eq!((r.reposes, r.deja_la, r.conflits.clone()), (1, 1, vec!["saves/b.srm".to_string()]));
        assert_eq!(std::fs::read(vers.join("saves/b.srm")).unwrap(), b"B jouee sur ce PC");
        assert_eq!(std::fs::read(vers.join("saves/a.srm")).unwrap(), b"A");

        // Un jeu tout juste réinstallé : la partie restaurée remplace le fichier du jeu neuf.
        assert_eq!(reposer(&de, &vers, true).unwrap().reposes, 1);
        assert_eq!(std::fs::read(vers.join("saves/b.srm")).unwrap(), b"B restauree");
    }

    #[tokio::test]
    async fn les_parties_ps_vita_reviennent_chez_l_utilisateur_vita3k_du_profil_et_partent_en_sauvegarde() {
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, None).unwrap().id;
        n.ouvrir(&id, None, &crate::noyau::Connexion { adresse: String::new(), simule: true }).await.unwrap();
        let zone = d.path().join("app").join("restauration").join(&id).join("parties").join("emulateurs").join("Vita3K");
        let r = ".vita3k-utilisateur/savedata/PCSB00001/sce_sys.bin";
        std::fs::create_dir_all(zone.join(r).parent().unwrap()).unwrap();
        std::fs::write(zone.join(r), b"persona").unwrap();
        let vita = d.path().join("Emulateurs").join("Vita3K");
        std::fs::create_dir_all(vita.join("portable")).unwrap();
        let programme: String = vita.join("Vita3K.exe").to_string_lossy().into();
        let rep = n.reposer_parties_emulateurs(std::slice::from_ref(&programme)).await.unwrap();
        assert_eq!((rep.reposes, rep.conflits.len()), (1, 0));
        let u = crate::vita3k::utilisateur_existant(&vita, "Seb").unwrap();
        let p = crate::vita3k::parties(&vita, &u).join("PCSB00001").join("sce_sys.bin");
        assert_eq!(std::fs::read(&p).unwrap(), b"persona");
        assert!(!crate::emulateurs_profils::dossier_du_profil(&vita, "Seb").join(".vita3k-utilisateur").exists());
        // Et elles repartent dans la sauvegarde de Seb.
        let c = n.contenu_sauvegarde(&[programme]).await.unwrap();
        let rel: Vec<_> = c.parties.iter().map(|e| e.relatif.as_str()).collect();
        assert!(rel.contains(&"emulateurs/Vita3K/.vita3k-utilisateur/savedata/PCSB00001/sce_sys.bin"), "{rel:?}");
    }

    #[tokio::test]
    async fn les_parties_wii_u_reviennent_dans_le_compte_cemu_du_profil() {
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, None).unwrap().id;
        n.ouvrir(&id, None, &crate::noyau::Connexion { adresse: String::new(), simule: true }).await.unwrap();
        let zone = d.path().join("app").join("restauration").join(&id).join("parties").join("emulateurs").join("Cemu");
        for (r, o) in [(".cemu-compte/00050000/101c9500/compte/save.dat", &b"zelda"[..]), (".cemu-compte/00050000/101c9500/common/options.dat", b"commun")] {
            std::fs::create_dir_all(zone.join(r).parent().unwrap()).unwrap();
            std::fs::write(zone.join(r), o).unwrap();
        }
        let cemu = d.path().join("Emulateurs").join("Cemu");
        std::fs::create_dir_all(cemu.join("portable")).unwrap();
        // Un compte déjà là (celui de Cemu) : celui de Seb est créé à côté.
        crate::cemu::compte_du_profil(&cemu, "default").unwrap();
        let programme: String = cemu.join("Cemu.exe").to_string_lossy().into();
        let r = n.reposer_parties_emulateurs(std::slice::from_ref(&programme)).await.unwrap();
        assert_eq!((r.reposes, r.conflits.len()), (2, 0));
        let seb = crate::cemu::compte_existant(&cemu, "Seb").unwrap();
        assert_ne!(seb, 0x8000_0001);
        let p = crate::cemu::dossier_du_jeu(&cemu, "00050000/101c9500", seb);
        assert_eq!(std::fs::read(p.join("save.dat")).unwrap(), b"zelda");
        assert_eq!(std::fs::read(p.parent().unwrap().join("common").join("options.dat")).unwrap(), b"commun");
        assert!(!crate::emulateurs_profils::dossier_du_profil(&cemu, "Seb").join(".cemu-compte").exists());
        // Il a rejoué depuis : jamais écrasé, et la partie déjà reposée n'est plus retraitée (aucun faux conflit).
        std::fs::write(p.join("save.dat"), b"zelda plus loin").unwrap();
        let r = n.reposer_parties_emulateurs(&[programme]).await.unwrap();
        assert!(r.conflits.is_empty() && r.a_remplacer.is_empty());
        assert_eq!(std::fs::read(p.join("save.dat")).unwrap(), b"zelda plus loin");
    }

    #[tokio::test]
    async fn les_parties_switch_reviennent_dans_l_utilisateur_eden_du_profil() {
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, None).unwrap().id;
        n.ouvrir(&id, None, &crate::noyau::Connexion { adresse: String::new(), simule: true }).await.unwrap();
        let zone = d.path().join("app").join("restauration").join(&id).join("parties").join("emulateurs").join("Eden");
        let r = ".eden-utilisateur/save/0100000000010000/save.bin";
        std::fs::create_dir_all(zone.join(r).parent().unwrap()).unwrap();
        std::fs::write(zone.join(r), b"zelda").unwrap();
        let rd = ".eden-utilisateur/device/0100000000030000/commun.bin";
        std::fs::create_dir_all(zone.join(rd).parent().unwrap()).unwrap();
        std::fs::write(zone.join(rd), b"commun").unwrap();
        let eden = d.path().join("Emulateurs").join("Eden");
        let programme: String = eden.join("eden.exe").to_string_lossy().into();
        // Eden pas portable : rien n'est écrit chez lui, les parties attendent.
        std::fs::create_dir_all(&eden).unwrap();
        assert_eq!(n.reposer_parties_emulateurs(std::slice::from_ref(&programme)).await.unwrap().reposes, 0);
        assert!(!crate::eden::fichier_profils(&eden).exists());
        // Portable : l'utilisateur de Seb est créé, puis ses parties reposées chez lui (jamais dans `Profils`).
        std::fs::create_dir_all(eden.join("user")).unwrap();
        let rep = n.reposer_parties_emulateurs(std::slice::from_ref(&programme)).await.unwrap();
        assert_eq!((rep.reposes, rep.conflits.len()), (2, 0));
        assert_eq!(std::fs::read(crate::eden::dossier_device(&eden).join("0100000000030000").join("commun.bin")).unwrap(), b"commun");
        let seb = crate::eden::utilisateur_existant(&eden, "Seb").unwrap();
        let save = crate::eden::dossiers_parties(&eden, &seb)[0].1.join("0100000000010000").join("save.bin");
        assert_eq!(std::fs::read(&save).unwrap(), b"zelda");
        assert!(!crate::emulateurs_profils::dossier_du_profil(&eden, "Seb").join(".eden-utilisateur").exists());
        // Il a rejoué depuis : jamais écrasé, et la partie déjà reposée n'est plus retraitée (aucun faux conflit).
        std::fs::write(&save, b"zelda plus loin").unwrap();
        let rep = n.reposer_parties_emulateurs(&[programme]).await.unwrap();
        assert!(rep.conflits.is_empty());
        assert_eq!(std::fs::read(&save).unwrap(), b"zelda plus loin");
    }

    #[tokio::test]
    async fn les_parties_ps3_reviennent_dans_le_compte_rpcs3_du_profil_meme_sous_un_autre_numero() {
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, None).unwrap().id;
        n.ouvrir(&id, None, &crate::noyau::Connexion { adresse: String::new(), simule: true }).await.unwrap();
        // En attente (sauvegarde restaurée) : une partie PS3 du compte de Seb, et un fichier du dossier du profil.
        let zone = d.path().join("app").join("restauration").join(&id).join("parties").join("emulateurs").join("RPCS3");
        for (r, o) in [
            (".rpcs3-compte/savedata/BCES00569-GAME/SAVE.DAT", &b"partie"[..]),
            (".rpcs3-compte/savedata/BCES00569-GAME/ICON0.PNG", b"icone"),
            ("frogtend.txt", b"profil"),
        ] {
            std::fs::create_dir_all(zone.join(r).parent().unwrap()).unwrap();
            std::fs::write(zone.join(r), o).unwrap();
        }
        // Le nouveau PC : RPCS3 installé, le compte 00000002 déjà pris par Léa.
        let rpcs3 = d.path().join("Emulateurs").join("RPCS3");
        let lea = rpcs3.join("dev_hdd0").join("home").join("00000002");
        std::fs::create_dir_all(&lea).unwrap();
        std::fs::write(lea.join("localusername"), "Lea").unwrap();
        let r = n.reposer_parties_emulateurs(&[rpcs3.join("rpcs3.exe").to_string_lossy().into()]).await.unwrap();
        assert_eq!((r.reposes, r.conflits.len()), (3, 0));
        let seb = rpcs3.join("dev_hdd0").join("home").join("00000003");
        assert_eq!(std::fs::read_to_string(seb.join("localusername")).unwrap(), "Seb");
        assert_eq!(std::fs::read(seb.join("savedata/BCES00569-GAME/SAVE.DAT")).unwrap(), b"partie");
        let profil = crate::emulateurs_profils::dossier_du_profil(&rpcs3, "Seb");
        assert!(profil.join("frogtend.txt").is_file());
        assert!(!profil.join(".rpcs3-compte").exists(), "jamais rangées dans le dossier du profil");
        assert!(!lea.join("savedata").exists(), "jamais dans le compte de quelqu'un d'autre");
        // Reposée une seconde fois, à l'identique : rien à faire.
        let r = n.reposer_parties_emulateurs(&[rpcs3.join("rpcs3.exe").to_string_lossy().into()]).await.unwrap();
        assert_eq!((r.reposes, r.conflits.len()), (0, 0));
        // Elle a rejoué depuis (SAVE.DAT changé, l'icône partie) : on n'y touche PAS DU TOUT — jamais un mélange des deux
        // parties — et la partie déjà reposée n'est plus retraitée (aucun faux conflit).
        std::fs::write(seb.join("savedata/BCES00569-GAME/SAVE.DAT"), b"plus recente").unwrap();
        std::fs::remove_file(seb.join("savedata/BCES00569-GAME/ICON0.PNG")).unwrap();
        let r = n.reposer_parties_emulateurs(&[rpcs3.join("rpcs3.exe").to_string_lossy().into()]).await.unwrap();
        assert!(r.conflits.is_empty());
        // Un VRAI conflit : la partie en place était là AVANT (registre oublié = une autre restauration) : dit UNE fois,
        // proposé au remplacement ; le remplacement met la partie en place à l'abri, relue.
        std::fs::remove_file(d.path().join("app").join("restauration").join(&id).join(REGLEES)).unwrap();
        let r = n.reposer_parties_emulateurs(&[rpcs3.join("rpcs3.exe").to_string_lossy().into()]).await.unwrap();
        assert_eq!(r.conflits, ["RPCS3/PS3 : savedata/BCES00569-GAME"]);
        assert_eq!(r.a_remplacer.len(), 1);
        assert_eq!(r.a_remplacer[0].etiquette, "RPCS3/PS3 : savedata/BCES00569-GAME");
        let r2 = n.reposer_parties_emulateurs(&[rpcs3.join("rpcs3.exe").to_string_lossy().into()]).await.unwrap();
        assert!(r2.conflits.is_empty(), "un conflit n'est dit qu'une fois");
        let abri = PathBuf::from(n.remplacer_par_restauree(&r.a_remplacer[0].cle).await.unwrap());
        assert_eq!(std::fs::read(seb.join("savedata/BCES00569-GAME/SAVE.DAT")).unwrap(), b"partie");
        assert_eq!(std::fs::read(seb.join("savedata/BCES00569-GAME/ICON0.PNG")).unwrap(), b"icone");
        assert_eq!(std::fs::read(abri.join("SAVE.DAT")).unwrap(), b"plus recente", "la partie d'avant à l'abri");
        assert!(!abri.join("ICON0.PNG").exists());
        assert!(n.remplacer_par_restauree(&r.a_remplacer[0].cle).await.is_err(), "une seule fois");
        // Effacée ensuite par la personne : elle ne revient PLUS (✅ Seb 27/10), mais reste dans la zone (sauvegarde).
        std::fs::remove_dir_all(seb.join("savedata/BCES00569-GAME")).unwrap();
        let r = n.reposer_parties_emulateurs(&[rpcs3.join("rpcs3.exe").to_string_lossy().into()]).await.unwrap();
        assert_eq!((r.reposes, r.conflits.len()), (0, 0));
        assert!(!seb.join("savedata/BCES00569-GAME").exists());
        assert!(zone.join(".rpcs3-compte/savedata/BCES00569-GAME/SAVE.DAT").is_file(), "jamais effacée de la zone");
    }

    #[tokio::test]
    async fn une_carte_memoire_en_double_est_dite_une_fois_et_remplacee_a_la_demande() {
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, None).unwrap().id;
        n.ouvrir(&id, None, &crate::noyau::Connexion { adresse: String::new(), simule: true }).await.unwrap();
        let zone = d.path().join("app").join("restauration").join(&id).join("parties").join("emulateurs").join("DuckStation");
        std::fs::create_dir_all(zone.join("memcards")).unwrap();
        std::fs::write(zone.join("memcards").join("ff7.mcd"), b"restauree").unwrap();
        std::fs::write(zone.join("memcards").join("tekken.mcd"), b"tekken").unwrap();
        let duck = d.path().join("Emulateurs").join("DuckStation");
        let profil = crate::emulateurs_profils::dossier_du_profil(&duck, "Seb");
        std::fs::create_dir_all(profil.join("memcards")).unwrap();
        std::fs::write(profil.join("memcards").join("ff7.mcd"), b"la mienne").unwrap();
        let programme: String = duck.join("duckstation.exe").to_string_lossy().into();
        let r = n.reposer_parties_emulateurs(std::slice::from_ref(&programme)).await.unwrap();
        assert_eq!((r.reposes, r.a_remplacer.len()), (1, 1));
        assert_eq!(std::fs::read(profil.join("memcards").join("ff7.mcd")).unwrap(), b"la mienne", "jamais remplacée sans le demander");
        assert!(n.reposer_parties_emulateurs(std::slice::from_ref(&programme)).await.unwrap().conflits.is_empty(), "dit une fois");
        // Tekken effacée par la personne : ne revient plus.
        std::fs::remove_file(profil.join("memcards").join("tekken.mcd")).unwrap();
        assert_eq!(n.reposer_parties_emulateurs(std::slice::from_ref(&programme)).await.unwrap().reposes, 0);
        assert!(!profil.join("memcards").join("tekken.mcd").exists());
        // Remplacer : la sienne à l'abri, la restaurée en place.
        let abri = PathBuf::from(n.remplacer_par_restauree(&r.a_remplacer[0].cle).await.unwrap());
        assert_eq!(std::fs::read(profil.join("memcards").join("ff7.mcd")).unwrap(), b"restauree");
        assert_eq!(std::fs::read(&abri).unwrap(), b"la mienne");
        assert!(n.remplacer_par_restauree("../../ailleurs").await.is_err());
        assert!(n.conflits_restaures().await.unwrap().is_empty(), "remplacée : plus en double");
        // DuckStation réinstallé AILLEURS : ses parties reviennent au nouvel endroit (même déjà reposées avant).
        let autre = d.path().join("Autre").join("DuckStation");
        let ailleurs: String = autre.join("duckstation.exe").to_string_lossy().into();
        let r = n.reposer_parties_emulateurs(std::slice::from_ref(&ailleurs)).await.unwrap();
        assert_eq!(r.reposes, 2);
        assert!(crate::emulateurs_profils::dossier_du_profil(&autre, "Seb").join("memcards").join("tekken.mcd").is_file());
    }

    #[tokio::test]
    async fn un_conflit_ferme_sans_reponse_reste_relisible() {
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, None).unwrap().id;
        n.ouvrir(&id, None, &crate::noyau::Connexion { adresse: String::new(), simule: true }).await.unwrap();
        let attente = d.path().join("app").join("restauration").join(&id);
        let zone = attente.join("parties").join("emulateurs").join("DuckStation");
        std::fs::create_dir_all(zone.join("memcards")).unwrap();
        std::fs::write(zone.join("memcards").join("ff7.mcd"), b"restauree").unwrap();
        let duck = d.path().join("Emulateurs").join("DuckStation");
        let profil = crate::emulateurs_profils::dossier_du_profil(&duck, "Seb");
        std::fs::create_dir_all(profil.join("memcards")).unwrap();
        std::fs::write(profil.join("memcards").join("ff7.mcd"), b"la mienne").unwrap();
        let programme: String = duck.join("duckstation.exe").to_string_lossy().into();
        assert_eq!(n.reposer_parties_emulateurs(std::slice::from_ref(&programme)).await.unwrap().a_remplacer.len(), 1);
        let c = n.conflits_restaures().await.unwrap();
        assert_eq!(c.len(), 1);
        assert!(c[0].copie.ends_with("ff7.mcd"));
        // Un registre trafiqué qui viserait hors de l'émulateur : refusé.
        let mut r = lire_reglees(&attente);
        r.entrees.values_mut().for_each(|e| e.cible = d.path().join("Ailleurs").join("x.mcd").to_string_lossy().into());
        ecrire_reglees(&attente, &r).unwrap();
        assert!(n.remplacer_par_restauree(&c[0].cle).await.is_err());
        assert!(!d.path().join("Ailleurs").exists());
    }

    #[tokio::test]
    async fn une_sauvegarde_se_telecharge_et_chaque_fichier_est_verifie() {
        let s = MockServer::start();
        let base = "/api/jeux/v1/sauvegarde/Sebastien/VENKMAN";
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("x");
        std::fs::write(&f, b"partie").unwrap();
        let sha = empreinte(&f).unwrap();
        s.mock(|w, t| {
            w.method(GET).path(format!("{base}/document/configuration.json"));
            t.status(200).json_body(json!({"reglages_profil": {"reglages": {"apparence": {"skin": "dracula"}}}}));
        });
        s.mock(|w, t| {
            w.method(GET).path(format!("{base}/document/bibliotheque.json"));
            t.status(200).json_body(json!({"jeux": [{"id": 110, "titre": "Dune"}]}));
        });
        s.mock(|w, t| {
            w.method(GET).path(format!("{base}/document/manifeste.json"));
            t.status(200).json_body(json!({"fichiers": {
                "jeux/110/C/DUNECD/DUNE37S0.SAV": [sha, 6],
                "emulateurs/RetroArch/saves/Super Metroid (USA).srm": [sha, 6]}}));
        });
        s.mock(|w, t| {
            w.method(GET).path(format!("{base}/fichier"));
            t.status(200).body(b"partie");
        });
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Sébastien", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &crate::noyau::Connexion { adresse: s.base_url(), simule: false }).await.unwrap();

        let r = n.preparer_restauration("Sebastien", "VENKMAN", &|_, _| {}).await.unwrap();
        assert_eq!((r.fichiers, r.taille), (2, 12));
        assert_eq!(r.configuration["reglages_profil"]["reglages"]["apparence"]["skin"], "dracula");

        // Les parties de l'émulateur vont dans le dossier du profil de RetroArch.
        let emu = d.path().join("Emulateurs").join("RetroArch");
        std::fs::create_dir_all(&emu).unwrap();
        let rep = n.reposer_parties_emulateurs(&[emu.join("retroarch.exe").to_string_lossy().into()]).await.unwrap();
        assert_eq!(rep.reposes, 1);
        assert!(crate::emulateurs_profils::dossier_du_profil(&emu, "Sébastien").join("saves/Super Metroid (USA).srm").is_file());

        // Celles de Dune attendent sa réinstallation, puis y sont reposées.
        let jeu = d.path().join("Dune");
        std::fs::create_dir_all(jeu.join("C/DUNECD")).unwrap();
        let rj = n.reposer_parties_du_jeu(&id, 110, &[], &jeu).unwrap();
        assert_eq!(rj.reposes, 1);
        assert_eq!(std::fs::read(jeu.join("C/DUNECD/DUNE37S0.SAV")).unwrap(), b"partie");
        assert_eq!(n.reposer_parties_du_jeu(&id, 110, &[], &jeu).unwrap().reposes, 0, "reposées une seule fois");
    }

    #[tokio::test]
    async fn un_fichier_abime_est_refuse() {
        let s = MockServer::start();
        let base = "/api/jeux/v1/sauvegarde/Seb/PC";
        for (nom, corps) in [("configuration.json", json!({})), ("bibliotheque.json", json!({})),
            ("manifeste.json", json!({"fichiers": {"a.sav": ["0000", 6]}}))] {
            s.mock(|w, t| {
                w.method(GET).path(format!("{base}/document/{nom}"));
                t.status(200).json_body(corps.clone());
            });
        }
        s.mock(|w, t| {
            w.method(GET).path(format!("{base}/fichier"));
            t.status(200).body(b"partie");
        });
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(d.path(), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &crate::noyau::Connexion { adresse: s.base_url(), simule: false }).await.unwrap();
        assert!(matches!(n.preparer_restauration("Seb", "PC", &|_, _| {}).await, Err(Erreur::Conflit(_))));
    }

    /// Relit (LECTURE SEULE) les sauvegardes du compte sur le vrai Firehouse : liste, puis les documents d'un couple.
    /// `FROGTEND_PROFIL_ESSAI=<id> FROGTEND_COUPLE=Profil/PC cargo test essai_lire_sauvegarde -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_lire_sauvegarde() {
        use crate::coffre::Coffre as _;
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton pour ce profil");
        let c = Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap();
        for s in lister(&c).await.unwrap() {
            println!("sauvegarde : {} / {} — {} octets — {}", s.profil, s.pc, s.octets, s.derniere);
        }
        let couple = std::env::var("FROGTEND_COUPLE").expect("FROGTEND_COUPLE");
        let (p, m) = couple.split_once('/').unwrap();
        let base = format!("/sauvegarde/{}/{}", crate::source::encoder(p), crate::source::encoder(m));
        let conf: Value = c.obtenir_json(&format!("{base}/document/configuration.json")).await.unwrap();
        let cles: Vec<&String> = conf.as_object().map(|o| o.keys().collect()).unwrap_or_default();
        println!("configuration.json : {cles:?}, profil {}", conf["profil"]);
        let biblio: Value = c.obtenir_json(&format!("{base}/document/bibliotheque.json")).await.unwrap();
        for j in biblio["jeux"].as_array().cloned().unwrap_or_default() {
            println!("jeu : {} — {} ({})", j["id"], j["titre"], j["etat"]);
        }
        let man: ManifesteParties = serde_json::from_value(c.obtenir_json(&format!("{base}/document/manifeste.json")).await.unwrap()).unwrap();
        println!("manifeste : {} fichier(s)", man.fichiers.len());
        for (r, (sha, t)) in &man.fichiers {
            println!("  {r} — {t} octets — {sha}");
        }
    }

    /// Essai sur le VRAI Firehouse (contrat 1.5), jamais lancé par la suite de tests (`#[ignore]`), avec l'accord de
    /// Seb. Il écrit UNIQUEMENT dans un dossier d'essai (« Essai-Frogtend / Venkman-essai »), avec des fichiers
    /// fabriqués ici : aucune vraie partie, aucun vrai profil. Il retire ensuite tout ce qu'il a créé (fichiers, puis le couple entier).
    /// `FROGTEND_PROFIL_ESSAI=<id du profil> cargo test essai_sauvegarde_reelle -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_sauvegarde_reelle() {
        use crate::coffre::Coffre as _;
        use crate::sauvegarde::{envoyer, Api, Contenu, Destination as _, Element, MORCEAU};
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton pour ce profil");
        let adresse = std::env::var("FROGTEND_ADRESSE_ESSAI").unwrap_or_else(|_| "https://jeux.hikari-no-sekai.fr".into());
        let c = Client::nouveau(&adresse, &jeton).unwrap();
        let (p, m) = ("Essai-Frogtend", "Venkman-essai");

        // Des fichiers fabriqués : un nom à parenthèses, un chemin accentué, un gros fichier (3 morceaux).
        let d = tempfile::tempdir().unwrap();
        let fichiers: Vec<(&str, Vec<u8>)> = vec![
            ("jeux/110/concat (1).sav", b"partie de Dune".to_vec()),
            ("emulateurs/RetroArch/saves/Zoé et l'été.srm", vec![7u8; 8192]),
            ("emulateurs/PCSX2/memcards/gros.ps2", (0..20 * 1024 * 1024u32).map(|i| (i.wrapping_mul(2654435761) >> 13) as u8).collect()),
        ];
        let mut parties = Vec::new();
        for (r, o) in &fichiers {
            let f = d.path().join("source").join(r.replace('/', "\\"));
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(&f, o).unwrap();
            parties.push(Element { source: f, relatif: r.to_string() });
        }
        let contenu = Contenu { configuration: json!({"essai": true}), bibliotheque: json!({"jeux": []}), parties, gardes: vec![] };
        let api = Api { client: &c, profil: p.into(), pc: m.into(), morceau: MORCEAU };

        // 1. Envoi complet.
        let b = envoyer(&api, &contenu, m, "Firehouse").await.expect("1er envoi");
        println!("1er envoi : {} envoyé(s), {} octets", b.envoyes, b.octets);
        assert_eq!((b.envoyes, b.fichiers), (3, 3));
        // 2. Rien n'a changé : rien n'est renvoyé.
        let b = envoyer(&api, &contenu, m, "Firehouse").await.expect("2e envoi");
        println!("2e envoi : {} envoyé(s), {} inchangé(s)", b.envoyes, b.inchanges);
        assert_eq!((b.envoyes, b.inchanges), (0, 3));

        // 3. Reprise après coupure : un premier morceau envoyé « à la main », puis l'envoi reprend là.
        let reprise = d.path().join("reprise.bin");
        let octets: Vec<u8> = (0..10 * 1024 * 1024u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&reprise, &octets).unwrap();
        let sha = empreinte(&reprise).unwrap();
        let route = format!("/sauvegarde/{p}/{m}/fichier?chemin={}", crate::source::encoder("essai/reprise.bin"));
        let premier = c
            .brute(
                reqwest::Method::PUT,
                &route,
                Some(octets[..MORCEAU].to_vec()),
                &[("Content-Range", format!("bytes 0-{}/{}", MORCEAU - 1, octets.len())), ("X-Contenu-Sha256", sha.clone())],
            )
            .await
            .unwrap();
        println!("1er morceau à la main : code {}", premier.statut);
        let tete = c.brute(reqwest::Method::HEAD, &route, None, &[]).await.unwrap();
        println!("HEAD : code {}, X-Recu {:?}", tete.statut, tete.recu);
        assert_eq!(tete.recu, Some(MORCEAU as u64), "Firehouse dit ce qu'il a déjà reçu");
        let n = api.envoyer_fichier("essai/reprise.bin", &reprise, &sha, octets.len() as u64).await.expect("reprise");
        assert_eq!(n, (octets.len() - MORCEAU) as u64, "seule la fin est renvoyée");

        // 4. Tout relire, chaque fichier vérifié par son empreinte.
        let liste = lister(&c).await.unwrap();
        assert!(liste.iter().any(|s| s.profil == p && s.pc == m), "la sauvegarde d'essai est listée");
        let base = format!("/sauvegarde/{p}/{m}");
        let lu: Value = c.obtenir_json(&format!("{base}/document/configuration.json")).await.unwrap();
        assert_eq!(lu["essai"], true);
        let manifeste: ManifesteParties =
            serde_json::from_value(c.obtenir_json(&format!("{base}/document/manifeste.json")).await.unwrap()).unwrap();
        assert_eq!(manifeste.fichiers.len(), 3);
        for (r, o) in &fichiers {
            let (sha, taille) = manifeste.fichiers.get(*r).expect(r).clone();
            let cible = d.path().join("relu").join(format!("{}", sha));
            telecharger(&c, &format!("{base}/fichier?chemin={}", crate::source::encoder(r)), &cible, &sha, taille).await.expect(r);
            assert_eq!(&std::fs::read(&cible).unwrap(), o, "{r} relu identique");
            println!("relu et vérifié : {r} ({taille} octets)");
        }

        // 5. Ménage : on retire les fichiers de l'essai (et seulement eux).
        for r in fichiers.iter().map(|(r, _)| *r).chain(["essai/reprise.bin"]) {
            api.supprimer_fichier(r).await.expect(r);
        }
        let apres = c.brute(reqwest::Method::GET, &format!("{base}/fichier?chemin={}", crate::source::encoder(fichiers[0].0)), None, &[]).await.unwrap();
        println!("après le ménage : GET d'un fichier → code {}", apres.statut);
        assert_eq!(apres.statut, 404);
        // Puis tout le couple d'essai, documents compris (DELETE /sauvegarde/{profil}/{pc}, Firehouse 30/09).
        // Garde-fou : jamais un autre couple que celui de l'essai.
        assert_eq!((p, m), ("Essai-Frogtend", "Venkman-essai"));
        let r = c.brute(reqwest::Method::DELETE, &base, None, &[]).await.unwrap();
        println!("DELETE {base} → code {} {}", r.statut, String::from_utf8_lossy(&r.octets));
        assert!((200..300).contains(&r.statut));
        let liste = lister(&c).await.unwrap();
        assert!(!liste.iter().any(|s| s.profil == p && s.pc == m), "plus rien de l'essai chez Firehouse");
        println!("vérifié : l'essai n'apparaît plus dans GET /sauvegarde");
    }
}
