//! Rapprocher les fichiers déjà sur un PC des jeux de Firehouse (contrat 1.21, besoin 28 ; ✅ Seb 20/10). Lot 1 : le
//! client seul — les types, le découpage en lots et les erreurs. Rien n'est encore relevé ni rattaché ici.
//!
//! Règles (contrat § 1.21, avis de l'expert contrat du 20/10) :
//! - lots ≤ 500 fichiers, ≤ 4 096 entrées par fichier, ≤ 50 000 entrées par appel (réglables par l'admin : nos limites
//!   par défaut sont plus prudentes) ; un GROUPE n'est jamais coupé (son statut se calcule dans un même appel) ;
//! - `nom` SEUL (jamais un chemin), `cle` OPAQUE (jamais un chemin : les dossiers de la personne ne partent pas) ;
//! - un 413 coupe le lot en deux (jusqu'à un fichier) ; un 404 = Firehouse sans le contrat 1.21 ; 401 et 503 arrêtent ;
//! - un fichier d'un groupe suit le statut du GROUPE (un « exact » dans un groupe « probable » n'est pas exact).

use crate::erreurs::{Erreur, Resultat};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Une entrée d'une archive (zip/7z) : nom SANS dossier, crc32 en hexadécimal (8 caractères), taille.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntreeArchive {
    pub nom: String,
    pub crc32: String,
    pub taille: u64,
}

/// Un fichier à rapprocher.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FichierLocal {
    pub cle: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub groupe: Option<String>,
    pub plateforme: String,
    pub nom: String,
    pub taille: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crc32: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entrees: Option<Vec<EntreeArchive>>,
    /// Contrat 1.22 : le SHA1 GLOBAL d'un CHD d'arcade (en-tête), jamais dans `sha1`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chd_sha1: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Candidat {
    pub media_id: i64,
    pub telechargement_id: Option<i64>,
    /// Le titre et l'année de la fiche (pris dans le catalogue gardé du profil ; Firehouse ne les envoie pas).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub titre: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub annee: Option<i64>,
}

/// Ce que Firehouse répond pour un fichier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultatFichier {
    pub cle: String,
    /// `exact`, `probable` ou `inconnu`.
    pub statut: String,
    #[serde(default)]
    pub correspondance: Option<String>,
    #[serde(default)]
    pub confiance: f64,
    #[serde(default)]
    pub media_id: Option<i64>,
    #[serde(default)]
    pub telechargement_id: Option<i64>,
    #[serde(default)]
    pub motif: Option<String>,
    #[serde(default)]
    pub candidats: Vec<Candidat>,
    #[serde(default)]
    pub groupe: Option<String>,
    /// Contrat 1.22, fichiers Arcade : ce que MAME peut lancer avec ce zip (ou ce CHD), machine par machine.
    #[serde(default)]
    pub machines: Vec<Machine>,
}

/// Une machine MAME reconnue dans un fichier (contrat 1.22).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Machine {
    pub nom_court: String,
    pub statut: String,
    #[serde(default)]
    pub correspondance: Option<String>,
    pub media_id: Option<i64>,
    pub telechargement_id: Option<i64>,
    #[serde(default)]
    pub manquantes: u32,
    /// Les BIOS / périphériques à avoir À CÔTÉ (la machine est exacte quand même ; à vérifier chez la personne).
    #[serde(default)]
    pub dependances: Vec<Dependance>,
    /// Les disques (CHD) de la machine : `present` vrai/faux si le CHD était dans le même groupe que le zip, sinon
    /// absent (aucun CHD envoyé avec le zip).
    #[serde(default)]
    pub disques: Vec<DisqueMachine>,
    #[serde(default)]
    pub complet_chez_nous: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisqueMachine {
    pub nom: String,
    #[serde(default)]
    pub chd_sha1: Option<String>,
    #[serde(default)]
    pub present: Option<bool>,
}

/// Ce qui manque à une machine reconnue pour que MAME la lance : ses BIOS / périphériques à côté du zip, ses disques
/// (CHD) ; et si la version de Firehouse elle-même n'a pas tous ses disques.
pub fn manques_machine(zip: &std::path::Path, m: &Machine) -> Option<String> {
    let mut l = dependances_absentes(zip, &m.dependances);
    l.extend(m.disques.iter().filter(|d| d.present != Some(true)).map(|d| format!("le disque {} (CHD)", d.nom)));
    let mut t = if l.is_empty() { String::new() } else { format!("il manque à côté du jeu : {}", l.join(", ")) };
    if !m.complet_chez_nous {
        if !t.is_empty() {
            t.push_str(" ; ");
        }
        t.push_str("la version de Firehouse n'a pas tous ses disques");
    }
    (!t.is_empty()).then_some(t)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dependance {
    pub nom_court: String,
    #[serde(rename = "type")]
    pub sorte: String,
}

/// Les BIOS / périphériques d'une machine qui ne sont PAS à côté de son zip (`<nom court>.zip` ou `.7z` dans le même
/// dossier, comme MAME les cherche) : « neogeo (BIOS) ».
pub fn dependances_absentes(zip: &std::path::Path, deps: &[Dependance]) -> Vec<String> {
    let Some(dossier) = zip.parent() else { return vec![] };
    deps.iter()
        .filter(|d| crate::jeux_pc::nom_de_fichier_sur(&d.nom_court).is_ok())
        .filter(|d| !["zip", "7z"].iter().any(|e| dossier.join(format!("{}.{e}", d.nom_court)).is_file()))
        .map(|d| format!("{} ({})", d.nom_court, if d.sorte == "bios" { "BIOS" } else { "périphérique" }))
        .collect()
}

/// La machine d'un fichier Arcade qui correspond à CE fichier : celle qui porte le nom court du zip (le parent d'un
/// zip fusionné ; les clones qu'il contient ne sont pas importés à part). Un CHD seul : la machine si elle est unique.
pub fn machine_du_fichier<'a>(nom: &str, r: &'a ResultatFichier) -> Option<&'a Machine> {
    let court = nom.rsplit_once('.').map(|(c, _)| c).unwrap_or(nom);
    r.machines.iter().find(|m| m.nom_court.eq_ignore_ascii_case(court)).or_else(|| (r.machines.len() == 1).then(|| &r.machines[0]))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultatGroupe {
    pub groupe: String,
    pub statut: String,
    #[serde(default)]
    pub media_id: Option<i64>,
    #[serde(default)]
    pub telechargement_id: Option<i64>,
    #[serde(default)]
    pub manquants: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Reponse {
    #[serde(default)]
    pub fichiers: Vec<ResultatFichier>,
    #[serde(default)]
    pub groupes: Vec<ResultatGroupe>,
}

/// Les plafonds d'un appel (réglables ; par défaut plus prudents que ceux de Firehouse).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limites {
    pub fichiers_lot: usize,
    pub entrees_fichier: usize,
    pub entrees_lot: usize,
}

impl Default for Limites {
    fn default() -> Self {
        Limites { fichiers_lot: 200, entrees_fichier: 4096, entrees_lot: 10_000 }
    }
}

/// Un nom envoyable : un nom de fichier SEUL (jamais un chemin), sans caractère de contrôle.
pub fn nom_envoyable(nom: &str) -> bool {
    crate::jeux_pc::nom_de_fichier_sur(nom).is_ok() && nom.len() <= 255
}

/// Découpe en lots sous les plafonds, sans JAMAIS couper un groupe. Un fichier aux entrées trop nombreuses part sans
/// elles (MAME : « probable » par le nom court ; archive : son sha1 seul). Un groupe plus gros qu'un lot part seul.
pub fn decouper(fichiers: Vec<FichierLocal>, l: Limites) -> Vec<Vec<FichierLocal>> {
    // Les unités : chaque groupe entier (dans l'ordre de sa première apparition), chaque fichier seul.
    let mut unites: Vec<Vec<FichierLocal>> = Vec::new();
    let mut index: std::collections::HashMap<String, usize> = Default::default();
    for mut f in fichiers {
        if f.entrees.as_ref().is_some_and(|e| e.len() > l.entrees_fichier) {
            f.entrees = None;
        }
        match f.groupe.clone() {
            Some(g) => match index.get(&g) {
                Some(&i) => unites[i].push(f),
                None => {
                    index.insert(g, unites.len());
                    unites.push(vec![f]);
                }
            },
            None => unites.push(vec![f]),
        }
    }
    let entrees = |u: &[FichierLocal]| u.iter().map(|f| f.entrees.as_ref().map_or(0, Vec::len)).sum::<usize>();
    let mut lots: Vec<Vec<FichierLocal>> = Vec::new();
    let mut courant: Vec<FichierLocal> = Vec::new();
    for u in unites {
        let deborde = !courant.is_empty() && (courant.len() + u.len() > l.fichiers_lot.max(1) || entrees(&courant) + entrees(&u) > l.entrees_lot);
        if deborde {
            lots.push(std::mem::take(&mut courant));
        }
        courant.extend(u);
    }
    if !courant.is_empty() {
        lots.push(courant);
    }
    lots
}

/// Le statut qui compte pour un fichier : celui de son GROUPE s'il en a un (contrat : « exact seulement si TOUS les
/// fichiers du groupe le sont »), sinon le sien. Rend (statut, media_id, telechargement_id).
pub fn statut_effectif<'a>(f: &'a ResultatFichier, r: &'a Reponse) -> (&'a str, Option<i64>, Option<i64>) {
    match f.groupe.as_deref().and_then(|g| r.groupes.iter().find(|x| x.groupe == g)) {
        Some(g) if g.statut == "exact" => ("exact", g.media_id, g.telechargement_id),
        // Groupe incomplet : au mieux probable, avec le jeu que dit la ligne du fichier.
        Some(g) => (if g.statut == "inconnu" && f.statut == "inconnu" { "inconnu" } else { "probable" }, f.media_id, f.telechargement_id),
        None => (f.statut.as_str(), f.media_id, f.telechargement_id),
    }
}

/// Ce qui s'est passé pour un appel entier.
#[derive(Debug, Default, PartialEq)]
pub struct Bilan {
    pub reponse: Reponse,
    /// Les clés qui n'ont pas pu partir (un fichier seul encore trop gros pour Firehouse).
    pub non_envoyes: Vec<String>,
}

/// Rapproche tous les fichiers : par lots, en coupant en deux un lot refusé pour sa taille (413).
pub async fn reconcilier(source: &crate::source::Source, fichiers: Vec<FichierLocal>, l: Limites) -> Resultat<Bilan> {
    let mut bilan = Bilan::default();
    let mut file: std::collections::VecDeque<Vec<FichierLocal>> = decouper(fichiers, l).into();
    while let Some(lot) = file.pop_front() {
        let (statut, corps) = source.reconcilier_brut(&json!({ "fichiers": lot })).await?;
        match statut {
            200 => {
                let r: Reponse = serde_json::from_slice::<Value>(&corps)
                    .ok()
                    .filter(|v| v["ok"].as_bool() != Some(false))
                    .and_then(|v| serde_json::from_value(v).ok())
                    .ok_or_else(|| Erreur::Serveur("Firehouse a envoyé une réponse illisible.".into()))?;
                // Seules les réponses aux fichiers de CE lot sont gardées.
                let cles: std::collections::HashSet<&str> = lot.iter().map(|f| f.cle.as_str()).collect();
                let groupes: std::collections::HashSet<&str> = lot.iter().filter_map(|f| f.groupe.as_deref()).collect();
                bilan.reponse.fichiers.extend(r.fichiers.into_iter().filter(|f| cles.contains(f.cle.as_str())));
                bilan.reponse.groupes.extend(r.groupes.into_iter().filter(|g| groupes.contains(g.groupe.as_str())));
            }
            413 if lot.len() > 1 => {
                // En deux, sans couper un groupe : les unités sont refaites par `decouper` avec un plafond moitié. Un
                // lot qui ne se coupe pas (un seul groupe) n'est JAMAIS renvoyé tel quel (sinon : sans fin).
                let moitie = Limites { fichiers_lot: (lot.len() / 2).max(1), ..l };
                let n = lot.len();
                let coupes = decouper(lot, moitie);
                if coupes.len() == 1 && coupes[0].len() == n {
                    bilan.non_envoyes.extend(coupes.into_iter().flatten().map(|f| f.cle));
                } else {
                    for (i, x) in coupes.into_iter().enumerate() {
                        file.insert(i, x);
                    }
                }
            }
            413 => bilan.non_envoyes.extend(lot.into_iter().map(|f| f.cle)),
            404 => return Err(Erreur::Refus("Firehouse ne sait pas encore rapprocher les fichiers (il lui faut le contrat 1.21).".into())),
            code => return Err(crate::firehouse::erreur_du_statut(code, &corps)),
        }
    }
    Ok(bilan)
}

// --- Lot 2 : l'inventaire des fichiers d'un profil, l'annonce chiffrée, le calcul des empreintes (lecture seule). ---

use crate::empreintes::{Cache, Empreinte, Nature};
use std::path::{Path, PathBuf};

/// Un fichier prévu pour le rapprochement : ce qui partira (sans empreintes au départ), d'où il vient (JAMAIS envoyé),
/// et le jeu importé auquel il appartient.
#[derive(Debug, Clone, PartialEq)]
pub struct Prevu {
    pub fichier: FichierLocal,
    pub chemin: PathBuf,
    pub nature: Nature,
    pub jeu: i64,
    /// Le chemin de la version (le fichier lancé) à laquelle appartient ce fichier.
    pub version: String,
}

/// Les fichiers des jeux importés à rapprocher : chaque version (le fichier lancé, ou TOUS ses disques — alors un
/// groupe) ; jamais les jeux MS-DOS ni Windows (le contrat n'a rien pour eux) ; clés opaques (`f0`, `f1`…), noms SEULS.
pub fn inventaire(jeux: &[crate::jeux_pc::JeuPc]) -> Vec<Prevu> {
    let mut l = Vec::new();
    for j in jeux.iter().filter(|j| crate::import_local::est_importe(j) && !matches!(j.plateforme.as_str(), "MS-DOS" | "Windows")) {
        let Some(i) = &j.installation else { continue };
        let versions: Vec<(String, Vec<String>)> = if i.versions.is_empty() {
            match &i.fichier_du_jeu {
                Some(f) => {
                    let c = Path::new(&i.dossier).join(f).to_string_lossy().into_owned();
                    vec![(c.clone(), vec![c])]
                }
                None => continue,
            }
        } else {
            i.versions.iter().map(|v| (v.chemin.clone(), if v.disques.is_empty() { vec![v.chemin.clone()] } else { v.disques.clone() })).collect()
        };
        for (k, (version, mut fichiers)) in versions.into_iter().enumerate() {
            // Arcade (contrat 1.22) : les CHD du jeu, dans `<dossier>\<nom court>\` (là où MAME les cherche), partent
            // AVEC le zip, dans son groupe (Firehouse dit alors si le disque est présent et tranche entre machines).
            let arcade = j.plateforme == "Arcade";
            if arcade {
                if let Some(z) = fichiers.first().map(PathBuf::from).filter(|z| z.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip") || e.eq_ignore_ascii_case("7z"))) {
                    if let (Some(dossier), Some(court)) = (z.parent(), z.file_stem()) {
                        if let Ok(l) = std::fs::read_dir(dossier.join(court)) {
                            let mut chd: Vec<String> = l.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("chd"))).map(|p| p.to_string_lossy().into_owned()).collect();
                            chd.sort();
                            fichiers.extend(chd);
                        }
                    }
                }
            }
            let groupe = (fichiers.len() > 1).then(|| format!("j{}v{k}", j.id.unsigned_abs()));
            for chemin in fichiers {
                let p = PathBuf::from(&chemin);
                let Some(nom) = p.file_name().map(|n| n.to_string_lossy().into_owned()).filter(|n| nom_envoyable(n)) else { continue };
                // Les CHD hors arcade : hors du périmètre du contrat (1.22 : arcade seulement).
                if nom.to_ascii_lowercase().ends_with(".chd") && !arcade {
                    continue;
                }
                let n = l.len();
                l.push(Prevu {
                    nature: crate::empreintes::nature(&j.plateforme, &nom),
                    fichier: FichierLocal { cle: format!("f{n}"), groupe: groupe.clone(), plateforme: j.plateforme.clone(), nom, taille: 0, sha1: None, crc32: None, entrees: None, chd_sha1: None },
                    chemin: p,
                    jeu: j.id,
                    version: version.clone(),
                });
            }
        }
    }
    l
}

/// Ce que coûtera le rapprochement, dit AVANT (règle de Seb : une opération de masse s'annonce et se chiffre).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Annonce {
    pub fichiers: usize,
    /// Fichiers lus EN ENTIER (crc32/sha1), et leur poids.
    pub a_lire: usize,
    pub octets: u64,
    /// Archives dont seul le répertoire est lu.
    pub repertoires: usize,
    /// Fichiers sur un disque réseau (lus en entier seulement avec l'accord de la personne), et ce qu'ils pèsent à lire.
    pub reseau: usize,
    pub octets_reseau: u64,
    /// Déjà connus du cache (rien à relire).
    pub connus: usize,
    /// Absents du disque.
    pub absents: usize,
}

/// Le plafond du sha1 d'une archive entière (avis de l'expert lancement, 20/10) : au-delà, ses entrées seules.
pub const PLAFOND_SHA1: u64 = 512 * 1024 * 1024;

/// Faut-il lire ce fichier en entier ?
fn lu_en_entier(p: &Prevu, taille: u64, plafond_sha1: u64) -> bool {
    match p.nature {
        Nature::EnClair => true,
        Nature::Archive => taille <= plafond_sha1,
        Nature::Mame | Nature::Disque => false,
    }
}

pub fn annoncer(prevus: &[Prevu], cache: &Cache, plafond_sha1: u64) -> Annonce {
    let mut a = Annonce { fichiers: prevus.len(), ..Default::default() };
    for p in prevus {
        let Ok(m) = std::fs::metadata(&p.chemin) else {
            a.absents += 1;
            continue;
        };
        if cache.lire(&p.chemin, &m).is_some() {
            a.connus += 1;
            continue;
        }
        let reseau = crate::empreintes::sur_le_reseau(&p.chemin);
        if matches!(p.nature, Nature::Archive | Nature::Mame) {
            a.repertoires += 1;
        }
        if lu_en_entier(p, m.len(), plafond_sha1) {
            // Sur le réseau : compté à part (lu seulement si la personne coche la case).
            if reseau {
                a.reseau += 1;
                a.octets_reseau += m.len();
            } else {
                a.a_lire += 1;
                a.octets += m.len();
            }
        }
    }
    a
}

/// Calcule ce qui manque (cache d'abord) et rend les fichiers prêts à partir ; les absents sont retirés. Sur un disque
/// réseau sans `reseau_ok` : répertoires seulement, aucune lecture entière. `progres(faits, total)`.
pub fn calculer(prevus: Vec<Prevu>, cache: &Cache, plafond_sha1: u64, reseau_ok: bool, progres: &mut dyn FnMut(usize, usize)) -> Vec<Prevu> {
    let total = prevus.len();
    let mut prets: Vec<(Prevu, std::fs::Metadata, Option<Empreinte>)> = prevus
        .into_iter()
        .filter_map(|p| {
            let m = std::fs::metadata(&p.chemin).ok().filter(|m| m.is_file())?;
            let e = cache.lire(&p.chemin, &m);
            Some((p, m, e))
        })
        .collect();
    // Les répertoires des archives à lire, en UN passage du lecteur isolé.
    let a_lister: Vec<usize> = prets.iter().enumerate().filter(|(_, (p, _, e))| e.is_none() && matches!(p.nature, Nature::Archive | Nature::Mame)).map(|(i, _)| i).collect();
    let chemins: Vec<PathBuf> = a_lister.iter().map(|i| prets[*i].0.chemin.clone()).collect();
    let mut faits = 0;
    let reps = crate::sept_z::repertoires(&chemins, &mut |n| progres(n.min(total), total));
    let mut entrees: std::collections::HashMap<usize, Option<Vec<EntreeArchive>>> = Default::default();
    for (k, r) in a_lister.iter().zip(reps) {
        entrees.insert(*k, r.ok().and_then(|l| crate::empreintes::entrees_du_contrat(&l)));
    }
    for (i, (p, m, e)) in prets.iter_mut().enumerate() {
        let e = match e.take() {
            Some(e) => e,
            None => {
                let mut x = Empreinte { entrees: entrees.remove(&i).flatten(), ..Default::default() };
                // Un CHD d'arcade : 124 octets d'en-tête seulement (même sur un disque réseau).
                if p.fichier.nom.to_ascii_lowercase().ends_with(".chd") {
                    x.chd_sha1 = crate::empreintes::chd_sha1(&p.chemin);
                }
                let permis = reseau_ok || !crate::empreintes::sur_le_reseau(&p.chemin);
                if permis && lu_en_entier(p, m.len(), plafond_sha1) {
                    if let Ok((crc, sha)) = crate::empreintes::crc_sha1(&p.chemin) {
                        match p.nature {
                            Nature::EnClair => x.crc32 = Some(crc),
                            _ => x.sha1 = Some(sha),
                        }
                    }
                }
                // Gardé seulement s'il est complet (un fichier réseau non lu sera relu une autre fois, avec accord).
                if permis || !lu_en_entier(p, m.len(), plafond_sha1) {
                    cache.garder(&p.chemin, m, &x);
                }
                x
            }
        };
        p.fichier.taille = m.len();
        p.fichier.crc32 = e.crc32;
        p.fichier.sha1 = e.sha1;
        p.fichier.entrees = e.entrees;
        p.fichier.chd_sha1 = e.chd_sha1;
        faits += 1;
        progres(chemins.len().max(faits).min(total), total);
    }
    prets.into_iter().map(|(p, _, _)| p).collect()
}

// --- Lot 3 : ce que dit Firehouse, jeu par jeu (✅ Seb 20/10 : seuls les « exact » se rattachent seuls ; des versions
// qui ne disent pas le même jeu sont MONTRÉES, rien n'est rattaché). ---

/// Une version d'un jeu importé (le fichier lancé, ou ses disques ensemble), telle que Firehouse la voit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionRapprochee {
    /// Le chemin du fichier lancé (sur CE PC ; jamais envoyé à Firehouse).
    pub version: String,
    /// `exact`, `probable`, `inconnu` ou `non_envoye`.
    pub statut: String,
    pub media_id: Option<i64>,
    pub telechargement_id: Option<i64>,
    pub correspondance: Option<String>,
    pub motif: Option<String>,
    pub candidats: Vec<Candidat>,
    /// Le titre et l'année de la fiche `media_id` (catalogue gardé du profil).
    #[serde(default)]
    pub titre_fiche: Option<String>,
    #[serde(default)]
    pub annee_fiche: Option<i64>,
}

/// Un jeu importé et ce qu'il est devenu.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JeuRapproche {
    pub jeu: i64,
    pub titre: String,
    pub plateforme: String,
    pub versions: Vec<VersionRapprochee>,
    /// `rattache` (toutes ses versions « exact », le même jeu), `a_voir` (à montrer à la personne), `inconnu`.
    pub decision: String,
    /// La fiche de Firehouse quand `rattache`.
    pub media_id: Option<i64>,
    /// Qui l'a rattaché : « auto » (tout exact) ou « personne ».
    #[serde(default)]
    pub rattache_par: Option<String>,
}

/// Remet un compte rendu à jour d'après ce qui est noté sur ce PC (rattachements faits ou défaits, fiches écartées par
/// la personne) et donne leurs titres aux fiches proposées.
pub fn actualiser(jeux: &mut [JeuRapproche], correspondances: &[(i64, i64, String)], ecartes: &[(i64, i64)], titre_de: &dyn Fn(i64) -> Option<(String, Option<i64>)>) {
    for j in jeux.iter_mut() {
        let ecarte = |m: i64| ecartes.iter().any(|(l, x)| *l == j.jeu && *x == m);
        for v in j.versions.iter_mut() {
            v.candidats.retain(|c| !ecarte(c.media_id));
            if v.media_id.is_some_and(ecarte) {
                v.media_id = None;
                v.telechargement_id = None;
                v.statut = "ecarte".into();
            }
            if let Some((t, a)) = v.media_id.and_then(titre_de) {
                v.titre_fiche = Some(t);
                v.annee_fiche = a;
            }
            for c in v.candidats.iter_mut() {
                if let Some((t, a)) = titre_de(c.media_id) {
                    c.titre = Some(t);
                    c.annee = a;
                }
            }
        }
        // Un rattachement n'est montré que si CE profil voit la fiche (✅ Seb : visible de ceux qui la voient).
        match correspondances.iter().find(|(l, m, _)| *l == j.jeu && titre_de(*m).is_some()) {
            Some((_, m, par)) => {
                j.decision = "rattache".into();
                j.media_id = Some(*m);
                j.rattache_par = Some(par.clone());
            }
            None => {
                j.rattache_par = None;
                j.media_id = None;
                let propose = j.versions.iter().any(|v| v.media_id.is_some() || !v.candidats.is_empty());
                j.decision = if propose { "a_voir" } else { "inconnu" }.into();
            }
        }
    }
}

/// Regroupe les réponses par jeu importé et décide.
pub fn par_jeu(prets: &[Prevu], bilan: &Bilan, jeux: &[crate::jeux_pc::JeuPc]) -> Vec<JeuRapproche> {
    let resultat = |cle: &str| bilan.reponse.fichiers.iter().find(|f| f.cle == cle);
    let mut l: Vec<JeuRapproche> = Vec::new();
    for j in jeux {
        let mine: Vec<&Prevu> = prets.iter().filter(|p| p.jeu == j.id).collect();
        if mine.is_empty() {
            continue;
        }
        let mut versions: Vec<VersionRapprochee> = Vec::new();
        for p in &mine {
            if versions.iter().any(|v| v.version == p.version) {
                continue;
            }
            let v = match resultat(&p.fichier.cle) {
                // Arcade (contrat 1.22) : le verdict de LA machine de ce zip (le groupe est calculé à la 1.21 : on ne le
                // lit pas pour l'arcade), avec ses BIOS / périphériques vérifiés à côté du zip.
                Some(r) => match machine_du_fichier(&p.fichier.nom, r) {
                    Some(m) => {
                        let motif = manques_machine(&p.chemin, m).or_else(|| r.motif.clone().filter(|_| m.statut != "exact"));
                        VersionRapprochee { version: p.version.clone(), statut: m.statut.clone(), media_id: m.media_id, telechargement_id: m.telechargement_id, correspondance: m.correspondance.clone(), motif, candidats: vec![], titre_fiche: None, annee_fiche: None }
                    }
                    None => {
                        let (statut, media_id, telechargement_id) = statut_effectif(r, &bilan.reponse);
                        VersionRapprochee { version: p.version.clone(), statut: statut.into(), media_id, telechargement_id, correspondance: r.correspondance.clone(), motif: r.motif.clone(), candidats: r.candidats.clone(), titre_fiche: None, annee_fiche: None }
                    }
                },
                None => VersionRapprochee { version: p.version.clone(), statut: "non_envoye".into(), media_id: None, telechargement_id: None, correspondance: None, motif: Some("pas envoyé (trop gros pour un appel)".into()), candidats: vec![], titre_fiche: None, annee_fiche: None },
            };
            versions.push(v);
        }
        let toutes_exactes = versions.iter().all(|v| v.statut == "exact");
        let un_seul_jeu = versions.iter().filter_map(|v| v.media_id).collect::<std::collections::BTreeSet<_>>();
        let (decision, media_id) = if toutes_exactes && un_seul_jeu.len() == 1 {
            ("rattache", un_seul_jeu.into_iter().next())
        } else if versions.iter().all(|v| v.statut == "inconnu") {
            ("inconnu", None)
        } else {
            ("a_voir", None)
        };
        l.push(JeuRapproche { jeu: j.id, titre: j.titre.clone(), plateforme: j.plateforme.clone(), versions, decision: decision.into(), media_id, rattache_par: None });
    }
    l
}

/// En mode simulé : aucune fiche simulée ne correspond à de vrais fichiers ; tout est « inconnu » (même forme que
/// Firehouse).
pub fn reponse_simulee(corps: &Value) -> Value {
    let fichiers: Vec<Value> = corps["fichiers"]
        .as_array()
        .map(|l| {
            l.iter()
                .map(|f| {
                    let mut r = json!({"cle": f["cle"], "statut": "inconnu", "correspondance": null, "confiance": 0.0, "media_id": null,
                        "telechargement_id": null, "motif": "mode simulé : aucune fiche à rapprocher", "candidats": []});
                    if !f["groupe"].is_null() {
                        r["groupe"] = f["groupe"].clone();
                    }
                    r
                })
                .collect()
        })
        .unwrap_or_default();
    let mut groupes: Vec<String> = fichiers.iter().filter_map(|f| f["groupe"].as_str().map(String::from)).collect();
    groupes.dedup();
    json!({"ok": true, "fichiers": fichiers, "groupes": groupes.into_iter().map(|g| json!({"groupe": g, "statut": "inconnu", "media_id": null, "telechargement_id": null, "manquants": 0})).collect::<Vec<_>>()})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn releve() -> Value {
        serde_json::from_str(include_str!("../fixtures/simule/reconcilier-releve.json")).unwrap()
    }

    fn fichier(cle: &str, groupe: Option<&str>, entrees: usize) -> FichierLocal {
        FichierLocal {
            cle: cle.into(),
            groupe: groupe.map(String::from),
            plateforme: "Arcade".into(),
            nom: format!("{cle}.zip"),
            taille: 1,
            sha1: None,
            crc32: None,
            entrees: (entrees > 0).then(|| (0..entrees).map(|i| EntreeArchive { nom: format!("e{i}"), crc32: "00000000".into(), taille: 1 }).collect()),
            chd_sha1: None,
        }
    }

    #[test]
    fn la_vraie_requete_se_relit_a_l_identique_et_la_vraie_reponse_se_lit() {
        let v = releve();
        let f: Vec<FichierLocal> = serde_json::from_value(v["requete"]["fichiers"].clone()).unwrap();
        assert_eq!(serde_json::to_value(&f).unwrap(), v["requete"]["fichiers"], "le corps envoyé est exactement celui du relevé");
        let r: Reponse = serde_json::from_value(v["reponse"].clone()).unwrap();
        let par = |c: &str| r.fichiers.iter().find(|x| x.cle == c).unwrap();
        assert_eq!(statut_effectif(par("sha1-cartouche"), &r), ("exact", Some(120), Some(-1)));
        assert_eq!(statut_effectif(par("mame-exact"), &r), ("exact", Some(12830), Some(-1_244_226_129_853_379)));
        assert_eq!(statut_effectif(par("mame-autre-version"), &r).0, "probable");
        assert_eq!(statut_effectif(par("inconnu"), &r), ("inconnu", None, None));
        // La ROM est « exact » seule, mais son groupe est incomplet : probable.
        assert_eq!(par("groupe-rom").statut, "exact");
        assert_eq!(statut_effectif(par("groupe-rom"), &r), ("probable", Some(8419), Some(-14927)));
    }

    #[test]
    fn les_lots_respectent_les_plafonds_et_ne_coupent_jamais_un_groupe() {
        let l = Limites { fichiers_lot: 3, entrees_fichier: 5, entrees_lot: 8 };
        let f = vec![fichier("a", None, 2), fichier("b", Some("g"), 0), fichier("c", None, 9), fichier("d", Some("g"), 3), fichier("e", None, 4), fichier("f", None, 0)];
        let lots = decouper(f, l);
        for lot in &lots {
            assert!(lot.len() <= 3);
            assert!(lot.iter().map(|f| f.entrees.as_ref().map_or(0, Vec::len)).sum::<usize>() <= 8);
        }
        let g: Vec<usize> = lots.iter().enumerate().filter(|(_, l)| l.iter().any(|f| f.groupe.as_deref() == Some("g"))).map(|(i, _)| i).collect();
        assert_eq!(g.len(), 1, "le groupe g est dans un seul lot");
        assert!(lots.iter().flatten().find(|f| f.cle == "c").unwrap().entrees.is_none(), "9 entrées > 5 : envoyé sans");
        assert_eq!(lots.iter().map(Vec::len).sum::<usize>(), 6);
        assert!(nom_envoyable("Super Mario (E).nes") && !nom_envoyable("E:\\Games\\x.nes") && !nom_envoyable("a/b.nes") && !nom_envoyable(".."));
    }

    fn importe(id: i64, plateforme: &str, dossier: &Path, fichier: Option<&str>, versions: Vec<crate::jeux_pc::VersionLocale>) -> crate::jeux_pc::JeuPc {
        crate::jeux_pc::JeuPc {
            id,
            version: crate::import_local::VERSION_IMPORTEE,
            titre: "Jeu".into(),
            plateforme: plateforme.into(),
            dossier: dossier.to_string_lossy().into(),
            etat: crate::jeux_pc::Etat::Telecharge,
            total: 0,
            fichiers: vec![],
            message: None,
            ajoute_le: String::new(),
            ajoute_par: String::new(),
            installation: Some(crate::jeux_pc::Installation {
                dossier: dossier.to_string_lossy().into(),
                methode: crate::installation::Methode::Aucune,
                lanceur: None,
                fichier_du_jeu: fichier.map(String::from),
                installe_le: String::new(),
                versions,
                boutique: None,
                extraits: vec![],
            }),
            temps_jeu: 0,
            derniere_partie: None,
        }
    }

    #[test]
    fn l_inventaire_et_le_calcul_lisent_ce_que_dit_le_contrat_et_rien_d_autre() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("Zelda (E).nes"), b"abc").unwrap();
        let src = d.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mario (E).sfc"), b"0123456789").unwrap();
        sevenz_rust::compress_to_path(&src, &d.path().join("Mario (E).7z")).unwrap();
        std::fs::write(d.path().join("Shenmue.chd"), b"MComprHD").unwrap();
        for disque in ["FF7 (Disc 1).cue", "FF7 (Disc 2).cue"] {
            std::fs::write(d.path().join(disque), b"FILE").unwrap();
        }
        let v = |c: &str, disques: Vec<String>| crate::jeux_pc::VersionLocale { chemin: c.into(), libelle: String::new(), rang: 0, qualite: 0, disques };
        let chemin = |n: &str| d.path().join(n).to_string_lossy().into_owned();
        let jeux = vec![
            importe(-1, "Nintendo Entertainment System", d.path(), Some("Zelda (E).nes"), vec![]),
            importe(-2, "Super Nintendo Entertainment System", d.path(), None, vec![v(&chemin("Mario (E).7z"), vec![])]),
            importe(-3, "Sony Playstation", d.path(), None, vec![v(&chemin("FF7 (Disc 1).cue"), vec![chemin("FF7 (Disc 1).cue"), chemin("FF7 (Disc 2).cue")])]),
            importe(-4, "MS-DOS", d.path(), Some("DUNE.EXE"), vec![]),
            importe(-6, "Sega Dreamcast", d.path(), Some("Shenmue.chd"), vec![]),
            importe(5, "Nintendo Entertainment System", d.path(), Some("Zelda (E).nes"), vec![]),
        ];
        let p = inventaire(&jeux);
        assert_eq!(p.len(), 4, "ni MS-DOS, ni un CHD (besoin 29), ni un jeu de Firehouse");
        assert!(p.iter().all(|x| !x.fichier.nom.contains(['\\', '/'])), "noms SEULS");
        assert!(p.iter().all(|x| x.fichier.cle.starts_with('f') && !x.fichier.cle.contains(':')), "clés opaques");
        let disques: Vec<_> = p.iter().filter(|x| x.jeu == -3).collect();
        assert!(disques.len() == 2 && disques[0].fichier.groupe.is_some() && disques[0].fichier.groupe == disques[1].fichier.groupe);
        let cache = Cache::ouvrir(&d.path().join("empreintes.db")).unwrap();
        let a = annoncer(&p, &cache, PLAFOND_SHA1);
        assert_eq!((a.fichiers, a.a_lire, a.repertoires, a.absents), (4, 2, 1, 0));
        let prets = calculer(p.clone(), &cache, PLAFOND_SHA1, false, &mut |_, _| {});
        let par = |j: i64| prets.iter().find(|x| x.jeu == j).unwrap();
        assert_eq!(par(-1).fichier.crc32.as_deref(), Some("352441c2"));
        let mario = &par(-2).fichier;
        assert!(mario.sha1.is_some() && mario.crc32.is_none());
        assert_eq!(mario.entrees.as_ref().unwrap()[0], EntreeArchive { nom: "Mario (E).sfc".into(), crc32: format!("{:08x}", crc32fast::hash(b"0123456789")), taille: 10 });
        assert!(par(-3).fichier.crc32.is_none() && par(-3).fichier.sha1.is_none() && par(-3).fichier.taille == 4, "un disque : nom et taille seulement");
        // Relu depuis le cache : plus rien à lire.
        assert_eq!(annoncer(&p, &cache, PLAFOND_SHA1).connus, 4);
        // La décision jeu par jeu : Zelda exact → rattaché ; Mario probable → à voir ; FF7 (groupe incomplet) → à voir.
        let r = |cle: &str, statut: &str, media: Option<i64>, groupe: Option<&str>| ResultatFichier { cle: cle.into(), statut: statut.into(), correspondance: None, confiance: 0.0, media_id: media, telechargement_id: None, motif: None, candidats: vec![], groupe: groupe.map(String::from), machines: vec![] };
        let cle = |j: i64| prets.iter().find(|x| x.jeu == j).unwrap().fichier.cle.clone();
        let g = par(-3).fichier.groupe.clone().unwrap();
        let ff7: Vec<String> = prets.iter().filter(|x| x.jeu == -3).map(|x| x.fichier.cle.clone()).collect();
        let bilan = Bilan {
            reponse: Reponse {
                fichiers: vec![r(&cle(-1), "exact", Some(7), None), r(&cle(-2), "probable", Some(8), None), r(&ff7[0], "exact", Some(9), Some(&g)), r(&ff7[1], "inconnu", None, Some(&g))],
                groupes: vec![ResultatGroupe { groupe: g, statut: "probable".into(), media_id: None, telechargement_id: None, manquants: 1 }],
            },
            non_envoyes: vec![],
        };
        let d = par_jeu(&prets, &bilan, &jeux);
        let dec = |j: i64| d.iter().find(|x| x.jeu == j).map(|x| (x.decision.as_str(), x.media_id)).unwrap();
        assert_eq!(dec(-1), ("rattache", Some(7)));
        assert_eq!(dec(-2), ("a_voir", None));
        assert_eq!(dec(-3), ("a_voir", None), "un exact dans un groupe incomplet n'est pas rattaché");
        assert_eq!(d.iter().find(|x| x.jeu == -3).unwrap().versions.len(), 1, "les deux disques = une version");
    }

    /// Contrat 1.22 : un jeu d'arcade et ses CHD (dans `<nom court>\`) partent ensemble, dans un groupe, avec le SHA1
    /// GLOBAL du CHD (124 octets lus) ; un CHD de console reste hors périmètre.
    #[test]
    fn un_jeu_d_arcade_part_avec_ses_chd_et_leur_sha1_global() {
        let d = tempfile::tempdir().unwrap();
        let mut zip = zip::ZipWriter::new(std::fs::File::create(d.path().join("raycrisj.zip")).unwrap());
        zip.start_file("rom.bin", zip::write::SimpleFileOptions::default()).unwrap();
        std::io::Write::write_all(&mut zip, b"rom").unwrap();
        zip.finish().unwrap();
        let mut entete = vec![0u8; 124];
        entete[..8].copy_from_slice(b"MComprHD");
        entete[12..16].copy_from_slice(&5u32.to_be_bytes());
        entete[84..104].copy_from_slice(&[0xab; 20]);
        std::fs::create_dir_all(d.path().join("raycrisj")).unwrap();
        std::fs::write(d.path().join("raycrisj").join("raycris.chd"), &entete).unwrap();
        let jeux = vec![importe(-7, "Arcade", d.path(), Some("raycrisj.zip"), vec![])];
        let p = inventaire(&jeux);
        assert_eq!(p.len(), 2);
        assert!(p[0].fichier.groupe.is_some() && p[0].fichier.groupe == p[1].fichier.groupe, "le zip et son CHD : un groupe");
        let prets = calculer(p, &Cache::ouvrir(&d.path().join("c.db")).unwrap(), PLAFOND_SHA1, false, &mut |_, _| {});
        let chd = prets.iter().find(|x| x.fichier.nom == "raycris.chd").unwrap();
        assert_eq!(chd.fichier.chd_sha1.as_deref(), Some("abababababababababababababababababababab"));
        assert!(chd.fichier.sha1.is_none(), "jamais dans « sha1 »");
        assert!(prets.iter().find(|x| x.fichier.nom == "raycrisj.zip").unwrap().fichier.entrees.is_some());
    }

    /// La DÉCISION sur le relevé réel 1.22 : 2020bb relié (BIOS neogeo absent : signalé) ; fbaitmca relié par SA
    /// machine (pas le groupe 1.21 « probable »), son disque présent, son BIOS sys573 signalé ; un jeu dont le disque
    /// n'est pas là : relié mais signalé.
    #[test]
    fn la_decision_sur_le_releve_1_22_relie_les_machines_et_dit_ce_qui_manque() {
        let v: Value = serde_json::from_str(include_str!("../fixtures/simule/reconcilier-122.json")).unwrap();
        let reponse: Reponse = serde_json::from_value(v["reponse"].clone()).unwrap();
        let requete: Vec<FichierLocal> = serde_json::from_value(v["requete"]["fichiers"].clone()).unwrap();
        let d = tempfile::tempdir().unwrap();
        let prevu = |cle: &str, jeu: i64, version: &str| {
            let f = requete.iter().find(|x| x.cle == cle).unwrap().clone();
            Prevu { chemin: d.path().join(&f.nom), nature: crate::empreintes::Nature::Mame, fichier: f, jeu, version: version.into() }
        };
        let prets = vec![prevu("zip-fusionne", -1, "a"), prevu("groupe-zip", -2, "b"), prevu("groupe-chd", -2, "b")];
        let jeux = vec![importe(-1, "Arcade", d.path(), Some("2020bb.zip"), vec![]), importe(-2, "Arcade", d.path(), Some("fbaitmca.zip"), vec![])];
        let decide = par_jeu(&prets, &Bilan { reponse, non_envoyes: vec![] }, &jeux);
        let jeu = |id: i64| decide.iter().find(|x| x.jeu == id).unwrap();
        assert_eq!((jeu(-1).decision.as_str(), jeu(-1).media_id), ("rattache", Some(12984)));
        assert!(jeu(-1).versions[0].motif.as_deref().unwrap().contains("neogeo (BIOS)"));
        assert_eq!((jeu(-2).decision.as_str(), jeu(-2).versions[0].telechargement_id), ("rattache", Some(-1_218_799_250_261_530)));
        let m = jeu(-2).versions[0].motif.clone().unwrap();
        assert!(m.contains("sys573") && !m.contains("disque"), "disque présent, BIOS absent : {m}");
        // Le même jeu sans son CHD à côté : le disque est signalé.
        let sans = Machine { disques: vec![DisqueMachine { nom: "889aa".into(), chd_sha1: None, present: None }], dependances: vec![], ..serde_json::from_value(v["reponse"]["fichiers"][3]["machines"][0].clone()).unwrap() };
        assert!(manques_machine(&d.path().join("fbaitmca.zip"), &sans).unwrap().contains("le disque 889aa (CHD)"));
    }

    /// Le relevé RÉEL du contrat 1.22 : la requête se relit à l'identique ; un zip fusionné est « exact » pour SA
    /// machine (le parent) même si le haut dit « probable » ; un CHD seul donne sa machine ; les BIOS sont vérifiés.
    #[test]
    fn le_vrai_releve_1_22_se_lit_machine_par_machine() {
        let v: Value = serde_json::from_str(include_str!("../fixtures/simule/reconcilier-122.json")).unwrap();
        let f: Vec<FichierLocal> = serde_json::from_value(v["requete"]["fichiers"].clone()).unwrap();
        assert_eq!(serde_json::to_value(&f).unwrap(), v["requete"]["fichiers"], "le corps envoyé est exactement celui du relevé");
        assert_eq!(f[2].chd_sha1.as_deref(), Some("015cb0e6c4421cc38809de28c4793b4491386aee"));
        let r: Reponse = serde_json::from_value(v["reponse"].clone()).unwrap();
        let par = |c: &str| r.fichiers.iter().find(|x| x.cle == c).unwrap();
        assert_eq!(par("zip-fusionne").statut, "probable", "le haut reste à la 1.21");
        let m = machine_du_fichier("2020bb.zip", par("zip-fusionne")).unwrap();
        assert_eq!((m.nom_court.as_str(), m.statut.as_str(), m.telechargement_id), ("2020bb", "exact", Some(-1_234_836_732_104_409)));
        assert_eq!(machine_du_fichier("2020bba.zip", par("zip-clone")).unwrap().nom_court, "2020bba");
        assert_eq!(machine_du_fichier("disque.chd", par("chd-seul")).unwrap().nom_court, "raycrisj", "un seul : celle-là");
        assert_eq!(machine_du_fichier("disque.chd", par("groupe-chd")).map(|m| m.nom_court.as_str()), None, "plusieurs machines, aucune au nom du fichier");
        // Le BIOS neogeo : à côté du zip, ou signalé manquant.
        let d = tempfile::tempdir().unwrap();
        let zip = d.path().join("2020bb.zip");
        assert_eq!(dependances_absentes(&zip, &m.dependances), ["neogeo (BIOS)"]);
        std::fs::write(d.path().join("neogeo.zip"), b"x").unwrap();
        assert!(dependances_absentes(&zip, &m.dependances).is_empty());
    }

    #[test]
    fn le_compte_rendu_suit_ce_qui_est_note_sur_le_pc() {
        let v = |m: Option<i64>, candidats: Vec<i64>| VersionRapprochee {
            version: "a".into(),
            statut: if m.is_some() { "probable" } else { "inconnu" }.into(),
            media_id: m,
            telechargement_id: None,
            correspondance: None,
            motif: None,
            candidats: candidats.into_iter().map(|c| Candidat { media_id: c, telechargement_id: None, titre: None, annee: None }).collect(),
            titre_fiche: None,
            annee_fiche: None,
        };
        let j = |id: i64, ver: VersionRapprochee, decision: &str| JeuRapproche { jeu: id, titre: "x".into(), plateforme: "p".into(), versions: vec![ver], decision: decision.into(), media_id: None, rattache_par: None };
        let mut l = vec![j(-1, v(Some(10), vec![]), "a_voir"), j(-2, v(Some(20), vec![]), "rattache"), j(-3, v(Some(30), vec![31]), "a_voir")];
        let titres = |id: i64| (id == 10).then(|| ("Zelda".to_string(), Some(1987)));
        // -1 rattaché par la personne ; -2 défait (plus noté) ; -3 : la fiche 30 écartée, le candidat 31 reste.
        actualiser(&mut l, &[(-1, 10, "personne".into())], &[(-3, 30)], &titres);
        assert_eq!((l[0].decision.as_str(), l[0].media_id, l[0].rattache_par.as_deref()), ("rattache", Some(10), Some("personne")));
        assert_eq!(l[0].versions[0].titre_fiche.as_deref(), Some("Zelda"));
        assert_eq!((l[1].decision.as_str(), l[1].media_id), ("a_voir", None), "défait : redevient à voir");
        assert_eq!((l[2].versions[0].statut.as_str(), l[2].versions[0].media_id, l[2].decision.as_str()), ("ecarte", None, "a_voir"));
        actualiser(&mut l, &[], &[(-3, 30), (-3, 31)], &titres);
        assert_eq!(l[2].decision, "inconnu", "plus rien à proposer");
    }

    #[test]
    fn deux_versions_qui_ne_disent_pas_le_meme_jeu_sont_montrees() {
        let d = tempfile::tempdir().unwrap();
        for f in ["A (E).nes", "A (U).nes"] {
            std::fs::write(d.path().join(f), b"x").unwrap();
        }
        let v = |n: &str| crate::jeux_pc::VersionLocale { chemin: d.path().join(n).to_string_lossy().into(), libelle: String::new(), rang: 0, qualite: 0, disques: vec![] };
        let jeux = vec![importe(-1, "Nintendo Entertainment System", d.path(), None, vec![v("A (E).nes"), v("A (U).nes")])];
        let prets = calculer(inventaire(&jeux), &Cache::ouvrir(&d.path().join("c.db")).unwrap(), PLAFOND_SHA1, false, &mut |_, _| {});
        let r = |cle: &str, media: i64| ResultatFichier { cle: cle.into(), statut: "exact".into(), correspondance: None, confiance: 1.0, media_id: Some(media), telechargement_id: None, motif: None, candidats: vec![], groupe: None, machines: vec![] };
        let bilan = Bilan { reponse: Reponse { fichiers: vec![r(&prets[0].fichier.cle, 1), r(&prets[1].fichier.cle, 2)], groupes: vec![] }, non_envoyes: vec![] };
        assert_eq!(par_jeu(&prets, &bilan, &jeux)[0].decision, "a_voir");
    }

    /// Essai RÉEL (lecture seule des ROM ; seuls les NOMS et empreintes partent, jamais un chemin) : les premiers
    /// fichiers de quelques dossiers de Seb, envoyés au vrai Firehouse avec le jeton du profil (lu au coffre, jamais
    /// affiché). `FROGTEND_PROFIL_ESSAI=<id> cargo test essai_reconcilier -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_reconcilier_vrais_fichiers() {
        use crate::coffre::Coffre as _;
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton");
        let source = crate::source::Source::Firehouse(crate::firehouse::Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap());
        let n = 30;
        let premiers = |d: &str, ext: &str| -> Vec<PathBuf> {
            let mut l: Vec<PathBuf> = std::fs::read_dir(d).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case(ext))).collect();
            l.sort();
            l.truncate(n);
            l
        };
        let mut jeux = Vec::new();
        let mut id = -1000;
        for (dossier, ext, plateforme) in [
            (r"E:\Games\Nintendo Entertainement System\Europe", "nes", "Nintendo Entertainment System"),
            (r"E:\Games\Nintendo Entertainement System\Europe", "7z", "Nintendo Entertainment System"),
            (r"E:\Games\Super Nintendo Entertainement System\A", "7z", "Super Nintendo Entertainment System"),
            (r"E:\Games\MAME", "zip", "Arcade"),
        ] {
            for f in premiers(dossier, ext) {
                id -= 1;
                jeux.push(importe(id, plateforme, f.parent().unwrap(), Some(&f.file_name().unwrap().to_string_lossy()), vec![]));
            }
        }
        let d = tempfile::tempdir().unwrap();
        let debut = std::time::Instant::now();
        let prets = calculer(inventaire(&jeux), &Cache::ouvrir(&d.path().join("c.db")).unwrap(), PLAFOND_SHA1, false, &mut |_, _| {});
        println!("{} fichiers lus en {:?}", prets.len(), debut.elapsed());
        let bilan = reconcilier(&source, prets.iter().map(|p| p.fichier.clone()).collect(), Limites::default()).await.unwrap();
        let decisions = par_jeu(&prets, &bilan, &jeux);
        let mut compte: std::collections::BTreeMap<(String, String), usize> = Default::default();
        for j in &decisions {
            *compte.entry((j.plateforme.clone(), j.decision.clone())).or_default() += 1;
        }
        println!("PAR PLATEFORME ET DÉCISION : {compte:#?}");
        for j in decisions.iter().filter(|j| j.decision != "rattache") {
            let v = &j.versions[0];
            let nom = Path::new(&v.version).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            println!("  {nom} [{}] → {} {:?} ({:?})", j.plateforme, v.statut, v.media_id, v.motif);
        }
        assert_eq!(bilan.reponse.fichiers.len(), prets.len(), "une réponse par fichier");
    }

    #[tokio::test]
    async fn un_lot_trop_gros_se_coupe_un_404_et_un_401_s_arretent() {
        use httpmock::prelude::*;
        let m = MockServer::start();
        // Plus de 2 fichiers : 413 ; sinon la réponse « inconnu » de chaque fichier.
        m.mock(|w, t| {
            w.method(POST).path("/api/jeux/v1/reconcilier").header("authorization", "Bearer jeton").matches(|r| {
                let v: Value = serde_json::from_slice(r.body.as_deref().unwrap_or_default()).unwrap_or_default();
                v["fichiers"].as_array().is_some_and(|l| l.len() > 2)
            });
            t.status(413).json_body(json!({"ok": false, "erreur": "500 fichiers au plus"}));
        });
        m.mock(|w, t| {
            w.method(POST).path("/api/jeux/v1/reconcilier");
            // Une réponse pour chaque clé possible ; seules celles du lot envoyé sont gardées.
            let f: Vec<Value> = (0..5).map(|i| json!({"cle": format!("f{i}"), "statut": "inconnu", "confiance": 0.0, "candidats": []})).collect();
            t.status(200).json_body(json!({"ok": true, "fichiers": f, "groupes": []}));
        });
        let s = crate::source::Source::Firehouse(crate::firehouse::Client::nouveau(&m.base_url(), "jeton").unwrap());
        let f: Vec<FichierLocal> = (0..5).map(|i| fichier(&format!("f{i}"), None, 0)).collect();
        let b = reconcilier(&s, f, Limites { fichiers_lot: 5, ..Limites::default() }).await.unwrap();
        assert_eq!(b.reponse.fichiers.len(), 5, "le lot de 5 a été coupé jusqu'à passer, une réponse par fichier, aucune en double");
        assert!(b.non_envoyes.is_empty());
        let m2 = MockServer::start();
        m2.mock(|w, t| {
            w.method(POST);
            t.status(404);
        });
        let s2 = crate::source::Source::Firehouse(crate::firehouse::Client::nouveau(&m2.base_url(), "jeton").unwrap());
        let e = reconcilier(&s2, vec![fichier("a", None, 0)], Limites::default()).await.unwrap_err().to_string();
        assert!(e.contains("1.21"), "{e}");
        let m3 = MockServer::start();
        m3.mock(|w, t| {
            w.method(POST);
            t.status(401);
        });
        let s3 = crate::source::Source::Firehouse(crate::firehouse::Client::nouveau(&m3.base_url(), "jeton").unwrap());
        assert!(matches!(reconcilier(&s3, vec![fichier("a", None, 0)], Limites::default()).await, Err(Erreur::JetonRefuse(_))));
        // Un 413 sur un lot qui n'est qu'UN groupe : noté non envoyé, jamais renvoyé sans fin.
        let m4 = MockServer::start();
        let refus = m4.mock(|w, t| {
            w.method(POST);
            t.status(413);
        });
        let s4 = crate::source::Source::Firehouse(crate::firehouse::Client::nouveau(&m4.base_url(), "jeton").unwrap());
        let groupe: Vec<FichierLocal> = (0..3).map(|i| fichier(&format!("g{i}"), Some("g"), 0)).collect();
        let b = reconcilier(&s4, groupe, Limites::default()).await.unwrap();
        assert_eq!(b.non_envoyes.len(), 3);
        assert_eq!(refus.hits(), 1, "envoyé une seule fois");
        // Mode simulé : tout inconnu, même forme.
        let b = reconcilier(&crate::source::Source::Simulee, vec![fichier("a", Some("g"), 0)], Limites::default()).await.unwrap();
        assert_eq!((b.reponse.fichiers[0].statut.as_str(), b.reponse.groupes.len()), ("inconnu", 1));
    }
}
