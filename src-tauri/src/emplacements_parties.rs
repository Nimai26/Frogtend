//! Où sont les parties des jeux PC (lot S1, 22/11) : les emplacements servis par Firehouse (contrat 1.25/1.26
//! `sauvegardes`, PCGamingWiki), rangés PAR PROFIL et réutilisables (futur éditeur de sauvegardes, Seb 22/11).
//!
//! - Un fichier par profil : `profils/<id>/emplacements-parties.json`, JAMAIS dans un dossier commun au PC (un chemin
//!   révèle le titre d'un jeu, peut-être réservé aux admins) ; jamais un chemin au journal (le numéro du jeu seulement).
//! - Les chemins sont gardés NON résolus (jetons du contrat 1.25 : `%VAR%`, `<dossier du jeu>`, `<Steam>`,
//!   `<identifiant Steam>`, `<identifiant utilisateur>`) : ils valent sur tout PC ; la résolution est faite à l'écran.
//! - Règles du cache : un `null` reçu (Firehouse ne sait pas, ou PCGamingWiki muet) n'efface jamais un cache rempli ;
//!   une liste (même vide) remplace ce qui venait de Firehouse ; hors ligne, le cache sert ; l'entrée d'un jeu que le
//!   profil ne voit plus est retirée.
//! - Pas de relecture en masse (Firehouse 22/11 : chaque recherche part chez PCGamingWiki) : on ne redemande qu'à
//!   l'ouverture de la fiche d'un jeu, et au plus une fois tous les 30 jours (`DELAI_LU`).
//!
//! Lecture seule : rien n'est copié ici (les abris, c'est le lot S2).

use crate::erreurs::{Erreur, Resultat};
use crate::nuage_steam::Contexte;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Le fichier, dans le dossier du profil.
pub const NOM_FICHIER: &str = "emplacements-parties.json";

/// La forme du fichier ; un fichier d'une forme plus récente n'est jamais réécrit par cette version. Un fichier plus
/// ancien se LIT (rien n'est perdu) et ses jeux sont redemandés à la prochaine ouverture de leur fiche : 2 = après le
/// contrat 1.48 (les réponses rangées en 1 sont relues une fois la correction 1.49 de Firehouse servie) ; 3 = après le
/// contrat 1.50 (repères bruts « (UserID) » ramenés aux jetons fermés, 0.220.1).
pub const VERSION: u32 = 3;

/// Un jeu que Firehouse a décrit (emplacements, ou « aucun ») n'est redemandé qu'au bout de 30 jours.
pub const DELAI_LU: u64 = 30 * 24 * 3600;

/// Un jeu dont Firehouse ne savait rien (`null` : pas encore lu, ou PCGamingWiki muet un moment) se redemande au bout
/// d'un jour, à l'ouverture de sa fiche seulement (jamais une boucle).
pub const DELAI_INCONNU: u64 = 24 * 3600;

/// Le moteur du jeu (Ren'Py, RPG Maker, Unity…) et ce qui le prouve (lot S6 ; vide en S1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Moteur {
    pub nom: String,
    pub preuve: String,
}

/// Ce que Firehouse a dit des parties de ce jeu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EtatFirehouse {
    /// Jamais demandé, ou Firehouse ne sait pas (`null`).
    #[default]
    Inconnu,
    /// Firehouse a répondu, sans aucun emplacement qui vaille pour Windows (ou pour la boutique du jeu).
    Vide,
    /// Au moins un emplacement.
    Lu,
}

/// La boutique d'un jeu (`steam`, `gog`, `epic`, `xboxone`…) et sa clé chez elle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoutiqueJeu {
    pub source: String,
    pub cle: String,
}

/// Un emplacement de parties, NON résolu.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct EmplacementPartie {
    /// Avec ses jetons (contrat 1.25), tel que Firehouse l'a donné.
    pub chemin: String,
    /// `dossier`, `fichier` ou `motif` (contrat 1.26 ; déduite du chemin si Firehouse ne l'a pas dite).
    pub nature: String,
    /// Le système de la ligne de PCGamingWiki (`Windows`, `Steam`, `Microsoft Store`…).
    pub os: String,
    /// `firehouse`, `moteur`, `manifeste` ou `personne`.
    pub origine: String,
    /// La boutique synchronise elle-même ces parties dans son nuage (`nuage` de Firehouse).
    #[serde(default)]
    pub nuage_boutique: bool,
    #[serde(default)]
    pub lu_le: Option<String>,
    /// D'où Firehouse le tient (contrat 1.48 `emplacements_source.source` : `ludusavi`, `pcgamingwiki`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// La boutique de la ligne (contrat 1.48), quand elle n'est pas commune à tout jeu Windows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boutique: Option<String>,
    /// Les étiquettes de la ligne (contrat 1.48 : « save », « config »…), telles quelles.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub etiquettes: Vec<String>,
    /// La boutique d'où vient le relevé (contrat 1.49), pour information seulement : on ne filtre JAMAIS dessus.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub releve_sur: Option<String>,
    /// Lot S6, origine `moteur` : le nom du moteur (d'après les références de Firehouse : « Unity »).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moteur: Option<String>,
    /// Lot S6, origine `moteur` : ce qui le prouve (le NOM du fichier marqueur et du fichier lu, jamais un chemin).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preuve: Option<String>,
}

/// Le moteur reconnu SUR LE DISQUE par Frogtend (lot S6), gardé dans l'état local du profil ; remonté à Firehouse
/// (contrat 1.53) sans jamais un chemin.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MoteurReconnu {
    /// La clé de Firehouse (`unity`, `renpy`…).
    pub cle: String,
    pub nom: String,
    /// Les noms des fichiers marqueur et lu.
    pub preuve: String,
    /// Les noms lus (jamais au journal).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub noms_lus: Vec<String>,
    /// Le NOM du fichier marqueur seul (la `preuve` du contrat 1.53).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub marqueur: String,
    /// Le nom lu sous la forme du contrat 1.53 (« Société/Produit » pour Unity) ; `None` sans nom à lire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nom_lu: Option<String>,
}

/// Ce qu'est devenu un signalement du moteur reconnu à Firehouse (contrat 1.53, besoin 60).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EtatSignalement {
    /// Firehouse l'a retenu (`retenu: true`).
    Retenu,
    /// Firehouse l'a noté pour un admin, sans rien remplacer (`retenu: false`).
    Note,
    /// Refusé (400 : clé inconnue, nom ou preuve invalide) : jamais renvoyé.
    Refuse,
    /// 404 : jeu caché ou absent chez Firehouse : jamais renvoyé.
    Introuvable,
    /// Hors ligne, 5xx ou Firehouse occupé : réessayé à la prochaine reconnaissance.
    HorsLigne,
}

impl EtatSignalement {
    /// Un état qui ne se renvoie plus jamais.
    pub fn definitif(self) -> bool {
        !matches!(self, EtatSignalement::HorsLigne)
    }

    /// Le mot du journal.
    pub fn mot(self) -> &'static str {
        match self {
            EtatSignalement::Retenu => "retenu",
            EtatSignalement::Note => "noté",
            EtatSignalement::Refuse => "refusé",
            EtatSignalement::Introuvable => "introuvable",
            EtatSignalement::HorsLigne => "hors ligne",
        }
    }
}

/// Un signalement fait (ou essayé) pour UN jeu : (clé du moteur, nom lu) n'est envoyé qu'une fois.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignalementMoteur {
    pub cle: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nom_lu: Option<String>,
    pub etat: EtatSignalement,
    /// Quand (secondes depuis 1970).
    pub le: String,
}

/// Le corps d'un signalement à envoyer (`POST /jeu/{id}/moteur`, contrat 1.53).
#[derive(Debug, Clone, PartialEq)]
pub struct ASignaler {
    pub cle: String,
    pub nom_lu: Option<String>,
    /// Le nom du fichier marqueur, sans dossier.
    pub preuve: String,
    /// Firehouse disait « aucune partie » (`Vide`). Un AUTRE moteur nommé par Firehouse n'est pas un `desaccord`
    /// (réponse de Firehouse du 24/11 : il note de lui-même un moteur différent).
    pub desaccord: bool,
}

impl ASignaler {
    /// `nom_lu` omis sans nom à lire ; `desaccord` seulement s'il est vrai (absent vaut faux).
    pub fn corps(&self) -> Value {
        let mut c = serde_json::json!({"moteur": self.cle, "preuve": self.preuve});
        if let Some(n) = &self.nom_lu {
            c["nom_lu"] = Value::from(n.as_str());
        }
        if self.desaccord {
            c["desaccord"] = Value::from(true);
        }
        c
    }
}

/// Le `nom_lu` passe-t-il les règles de Firehouse (contrat 1.53) ? 200 caractères au plus ; « Société/Produit » permis
/// (une barre entre deux morceaux non vides) ; JAMAIS ce qui ressemble à un chemin : ni `\`, ni lettre de disque (ni
/// aucun `:`), ni chemin absolu, ni `..`, ni `%VARIABLE%` (aucun `%`), ni caractère de contrôle.
pub fn nom_lu_envoyable(v: &str) -> bool {
    !v.trim().is_empty()
        && v.chars().count() <= 200
        && !v.contains("..")
        && !v.chars().any(|c| c.is_control() || ['\\', ':', '%'].contains(&c))
        && v.split('/').all(|m| !m.trim().is_empty())
        && v.matches('/').count() <= 1
}

/// La `preuve` passe-t-elle les règles de Firehouse (contrat 1.53) ? Un simple nom de fichier, sans dossier, de 120
/// caractères au plus.
pub fn preuve_envoyable(v: &str) -> bool {
    !v.trim().is_empty() && v.chars().count() <= 120 && v != "." && !v.contains("..") && !v.chars().any(|c| c.is_control() || ['\\', '/', ':', '%'].contains(&c))
}

/// Le signalement à faire pour ce jeu, ou `None` : un moteur PROUVÉ et appliqué (des lignes `moteur` rangées) quand
/// Firehouse ne connaît pas les parties (`Inconnu`), disait « aucune partie » (désaccord) ou nomme un autre moteur ; et
/// jamais deux fois le même (clé, nom lu) une fois la réponse connue (retenu, noté, refusé, introuvable). Un nom lu ou
/// une preuve qui ressemble à un chemin : jamais envoyé.
pub fn a_signaler(e: &PartiesJeu) -> Option<ASignaler> {
    let m = e.moteur_reconnu.as_ref()?;
    if !e.emplacements.iter().any(|x| x.origine == "moteur") {
        return None;
    }
    if !(e.firehouse == EtatFirehouse::Inconnu || e.desaccord_moteur || e.moteur_contredit) {
        return None;
    }
    let nom_lu = m.nom_lu.clone().filter(|n| !n.trim().is_empty());
    if nom_lu.as_deref().is_some_and(|n| !nom_lu_envoyable(n)) {
        return None;
    }
    let preuve = if m.marqueur.is_empty() { m.preuve.split(" + ").next().unwrap_or("").to_string() } else { m.marqueur.clone() };
    if m.cle.trim().is_empty() || !preuve_envoyable(&preuve) {
        return None;
    }
    if e.signalements.iter().any(|x| x.cle == m.cle && x.nom_lu == nom_lu && x.etat.definitif()) {
        return None;
    }
    Some(ASignaler { cle: m.cle.clone(), nom_lu, preuve, desaccord: e.desaccord_moteur })
}

/// Au plus tant de signalements gardés par jeu (les plus récents).
const SIGNALEMENTS_MAX: usize = 10;

/// Range ce qu'est devenu un signalement (le précédent du même (clé, nom lu) est remplacé) et, sur une réponse de
/// Firehouse, le moteur et les `emplacements_moteur` qu'il retient APRÈS le signalement. Rien pour un jeu sans entrée.
pub fn noter_signalement(c: &mut Cache, id: i64, a: &ASignaler, etat: EtatSignalement, quand: &str, reponse: Option<&Value>) {
    let Some(e) = c.jeux.get_mut(&id.to_string()) else { return };
    e.signalements.retain(|x| !(x.cle == a.cle && x.nom_lu == a.nom_lu));
    e.signalements.push(SignalementMoteur { cle: a.cle.clone(), nom_lu: a.nom_lu.clone(), etat, le: quand.to_string() });
    let trop = e.signalements.len().saturating_sub(SIGNALEMENTS_MAX);
    e.signalements.drain(..trop);
    let Some(r) = reponse else { return };
    let texte = |v: &Value| v.as_str().map(str::trim).filter(|r| !r.is_empty()).map(String::from);
    let em = &r["emplacements_moteur"];
    if em.is_object() {
        e.moteur_firehouse = Some(em.clone());
    } else if r.get("emplacements_moteur").is_some() {
        e.moteur_firehouse = None;
    }
    match texte(&em["moteur_nom"]).or_else(|| texte(&r["moteur"])) {
        Some(nom) => {
            let preuve = texte(&em["source_url"]).map(|u| format!("Firehouse ({u})")).unwrap_or_else(|| "Firehouse".into());
            e.moteur = Some(Moteur { nom, preuve });
        }
        None if r.get("moteur").is_some() && e.moteur.as_ref().is_some_and(|m| m.preuve.starts_with("Firehouse")) => e.moteur = None,
        None => {}
    }
}

/// Ce que le profil sait des parties d'UN jeu.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PartiesJeu {
    /// Le numéro de la carte de la ludothèque.
    pub id: i64,
    #[serde(default)]
    pub boutique: Option<BoutiqueJeu>,
    #[serde(default)]
    pub moteur: Option<Moteur>,
    #[serde(default)]
    pub firehouse: EtatFirehouse,
    /// Quand Firehouse a décrit ce jeu pour la dernière fois (secondes depuis 1970).
    #[serde(default)]
    pub lu_le: Option<String>,
    /// Quand Frogtend l'a demandé pour la dernière fois (même un `null`) : le frein des 30 jours.
    #[serde(default)]
    pub demande_le: Option<String>,
    /// Par quoi Firehouse a reconnu le jeu (besoin 56 : l'identifiant de la boutique, le titre…), quand il le dit.
    #[serde(default)]
    pub reconnu_par: Option<String>,
    /// La source de Firehouse (contrat 1.48 `emplacements_source` : `ludusavi`, `pcgamingwiki`), son nom du jeu et sa page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_nom: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    /// Ce que Firehouse dit du moteur (contrat 1.48 `emplacements_moteur`), gardé tel quel pour le lot S6.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moteur_firehouse: Option<Value>,
    /// Les systèmes dont Firehouse connaît des emplacements qui ne valent pas pour ce jeu (« Steam » pour un jeu Epic…).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub autres_systemes: Vec<String>,
    /// Firehouse a écarté le `dossier` envoyé (contrat 1.49 `emplacements_source.dossier_ecarte`) : gardé pour
    /// information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dossier_ecarte: Option<Value>,
    /// Lot S6 : le moteur reconnu sur le disque (deux preuves), s'il l'est.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moteur_reconnu: Option<MoteurReconnu>,
    /// Lot S6 : Firehouse dit « aucune partie » (`Vide`), mais le moteur est prouvé et des parties existent sur le
    /// disque : le moteur l'emporte, et le désaccord est noté (pour la remontée, besoin 60).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub desaccord_moteur: bool,
    /// Lot S6 : Firehouse nomme pour ce jeu un AUTRE moteur connu que celui reconnu sur le disque (état local, pour la
    /// remontée du besoin 60).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub moteur_contredit: bool,
    /// Contrat 1.53 : les signalements du moteur reconnu faits à Firehouse (jamais renvoyés une fois répondus).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub signalements: Vec<SignalementMoteur>,
    #[serde(default)]
    pub emplacements: Vec<EmplacementPartie>,
}

/// Le fichier entier d'un profil.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cache {
    #[serde(default = "version_un")]
    pub version: u32,
    /// Par numéro de carte (en texte : une clé JSON).
    #[serde(default)]
    pub jeux: BTreeMap<String, PartiesJeu>,
}

fn version_un() -> u32 {
    VERSION
}

impl Default for Cache {
    fn default() -> Self {
        Cache { version: VERSION, jeux: BTreeMap::new() }
    }
}

impl Cache {
    pub fn jeu(&self, id: i64) -> Option<&PartiesJeu> {
        self.jeux.get(&id.to_string())
    }
}

pub fn fichier(dossier_profil: &Path) -> PathBuf {
    dossier_profil.join(NOM_FICHIER)
}

/// Le cache du profil : vide s'il n'existe pas ; une ERREUR s'il existe mais ne se lit pas (il n'est alors jamais
/// remplacé : l'éditeur de sauvegardes y rangera aussi ce que la personne a choisi).
pub fn lire(dossier_profil: &Path) -> Resultat<Cache> {
    match std::fs::read(fichier(dossier_profil)) {
        Ok(o) => {
            let mut c = serde_json::from_slice::<Cache>(&o).map_err(|_| Erreur::Disque("Le fichier des emplacements de parties de ce profil est illisible : Frogtend ne l'écrase pas.".into()))?;
            // Une forme plus ancienne : tout est gardé, mais chaque jeu sera redemandé (une fois, à l'ouverture de sa fiche).
            if c.version < VERSION {
                for e in c.jeux.values_mut() {
                    e.demande_le = None;
                }
                c.version = VERSION;
            }
            Ok(c)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Cache::default()),
        Err(e) => Err(e.into()),
    }
}

/// Écrit le cache par un fichier provisoire renommé ensuite (une coupure ne laisse jamais un fichier à moitié écrit).
pub fn ecrire(dossier_profil: &Path, c: &Cache) -> Resultat<()> {
    if c.version > VERSION {
        return Err(Erreur::Disque("Les emplacements de parties ont été écrits par une version plus récente de Frogtend : celle-ci ne les écrase pas (mets Frogtend à jour).".into()));
    }
    std::fs::create_dir_all(dossier_profil)?;
    let provisoire = dossier_profil.join(format!("{NOM_FICHIER}.provisoire"));
    let octets = serde_json::to_vec_pretty(c).map_err(|_| Erreur::Disque("Emplacements de parties impossibles à écrire.".into()))?;
    std::fs::write(&provisoire, octets)?;
    if let Err(e) = std::fs::rename(&provisoire, fichier(dossier_profil)) {
        let _ = std::fs::remove_file(&provisoire);
        return Err(e.into());
    }
    Ok(())
}

/// Un seul changement à la fois (tous profils confondus : les écritures sont rares et courtes).
static VERROU: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Change le cache d'un profil sous le verrou (relu juste avant) ; écrit seulement s'il a changé.
pub fn modifier<R>(dossier_profil: &Path, f: impl FnOnce(&mut Cache) -> R) -> Resultat<R> {
    let _v = VERROU.lock().unwrap_or_else(|e| e.into_inner());
    let avant = lire(dossier_profil)?;
    let mut c = avant.clone();
    let r = f(&mut c);
    if c != avant {
        ecrire(dossier_profil, &c)?;
    }
    Ok(r)
}

/// Les listes fermées du contrat 1.48 dont Frogtend se sert : les NOMS de `os` (la réponse les donne en objet `{nom:
/// description}`, sans correspondance lisible par machine avec les boutiques) ; le reste est gardé tel quel dans `brut`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct References {
    pub os: Vec<String>,
    #[serde(default)]
    pub brut: Value,
}

/// La liste de secours, quand la route des références ne s'est jamais lue (les 11 noms du relevé réel du 22/11).
/// C'est la route qui fait foi.
pub fn references_de_secours() -> References {
    References {
        os: ["Windows", "Steam", "Microsoft Store", "Epic Games Store", "GOG.com", "Ubisoft Connect", "EA app", "Amazon Games", "DOS", "Linux", "Mac"].map(String::from).to_vec(),
        brut: Value::Null,
    }
}

/// Lit la réponse de `/sauvegardes/references` : les noms de `os` (les clés de l'objet), sans rien deviner ; `None` si
/// `os` n'y est pas.
pub fn lire_references(v: &Value) -> Option<References> {
    let os: Vec<String> = v["os"].as_object()?.keys().map(|k| k.trim().to_string()).filter(|k| !k.is_empty()).collect();
    Some(References { os, brut: v.clone() })
}

impl References {
    /// Les noms de dossier trop courants que Firehouse dit ignorer (contrat 1.49 `dossiers_ignores`), en minuscules ;
    /// vide tant qu'il ne les sert pas (la liste codée de Frogtend sert toujours, en plus).
    pub fn dossiers_ignores(&self) -> Vec<String> {
        self.brut["dossiers_ignores"].as_array().into_iter().flatten().filter_map(|x| x.as_str()).map(|x| x.trim().to_lowercase()).filter(|x| !x.is_empty()).collect()
    }
}

/// Les noms de dossier d'installation trop courants pour reconnaître un jeu (double garde avec Firehouse, 22/11) : ils
/// ne sont jamais envoyés en `dossier`, pas plus qu'un nom de moins de 4 caractères.
pub const DOSSIERS_COURANTS: &[&str] = &[
    "content", "game", "games", "binaries", "win64", "win32", "x64", "x86", "bin", "data", "app", "build", "release", "common", "windows",
    "launcher", "jeux", "program files", "program files (x86)", "steamapps", "pc",
];

/// Le nom d'un dossier d'installation peut-il servir à reconnaître le jeu chez Firehouse ? Non s'il est trop court,
/// trop courant (liste codée, `dossiers_ignores` des références) ou s'il est celui du dossier des jeux réglé dans
/// Frogtend (`autres`).
pub fn dossier_reconnaissable(nom: &str, refs: &References, autres: &[String]) -> bool {
    let n = nom.trim().to_lowercase();
    n.chars().count() >= 4 && !DOSSIERS_COURANTS.contains(&n.as_str()) && !refs.dossiers_ignores().contains(&n) && !autres.iter().any(|a| a.trim().to_lowercase() == n)
}

/// Le nom de `os` propre à une boutique de Frogtend (table de Frogtend : les références ne disent pas laquelle vaut pour
/// quelle boutique ; elles servent à VALIDER le nom).
pub fn os_de_boutique(source: &str) -> Option<&'static str> {
    Some(match source {
        "steam" => "Steam",
        "epic" => "Epic Games Store",
        "gog" => "GOG.com",
        "xboxone" | "xbox" => "Microsoft Store",
        "uplay" => "Ubisoft Connect",
        "origin" | "ea" => "EA app",
        "amazon" => "Amazon Games",
        _ => return None,
    })
}

/// Le nom d'une boutique chez Firehouse (« gog », « GOG.com », « xbox », « Microsoft Store », « ubisoft »…) désigne-t-il
/// la boutique Frogtend `source` (`steam`, `gog`, `epic`, `xboxone`, `uplay`, `amazon`, `origin`, `battlenet`) ?
pub fn meme_boutique(source: &str, nom: &str) -> bool {
    let n = nom.trim().to_lowercase();
    let alias: &[&str] = match source {
        "steam" => &["steam"],
        "gog" => &["gog", "gog.com", "gog galaxy"],
        "epic" => &["epic", "epic games", "epic games store", "epic games launcher", "egs"],
        "xboxone" | "xbox" => &["xboxone", "xbox", "microsoft", "microsoft store", "microsoft_store", "msstore", "game pass"],
        "uplay" => &["uplay", "ubisoft", "ubisoft connect"],
        "amazon" => &["amazon", "amazon games"],
        "origin" | "ea" => &["origin", "ea", "ea app"],
        "battlenet" => &["battlenet", "battle.net"],
        _ => &[],
    };
    n == source || alias.contains(&n.as_str())
}

/// Les valeurs de `os` qui valent pour un jeu de cette boutique : « Windows » toujours ; celle de SA boutique en plus, si
/// les références la connaissent ; jamais Linux, Mac ni DOS.
pub fn os_gardes(boutique: Option<&str>, refs: &References) -> Vec<String> {
    let mut l = vec!["Windows".to_string()];
    if let Some(o) = boutique.and_then(os_de_boutique).filter(|o| refs.os.iter().any(|x| x == o)) {
        l.push(o.to_string());
    }
    l
}

/// Un chemin de Mac ou de Linux servi pour Windows (contrat 1.48, relevé réel TerraScape : `%USERPROFILE%\.config\…`,
/// `%USERPROFILE%\Library\…` en « Steam ») : le premier dossier après `%USERPROFILE%` est `.config`, `.local` ou
/// `Library`. Écarté en attendant la correction 1.49 de Firehouse.
pub fn chemin_de_mac_ou_linux(chemin: &str) -> bool {
    let c = chemin.trim().replace('/', "\\").to_ascii_lowercase();
    let Some(reste) = c.strip_prefix("%userprofile%\\") else { return false };
    let premier = reste.split('\\').next().unwrap_or("");
    [".config", ".local", "library"].contains(&premier)
}

/// Les systèmes (valeurs de `os`) dont Firehouse a donné des emplacements écartés pour ce jeu — pour dire honnêtement
/// « connus seulement pour Steam » au lieu de « aucun » (relevé réel TerraScape, contrat 1.48). Le registre et les `os`
/// inconnus n'y comptent pas.
pub fn systemes_ecartes(sauvegardes: &Value, boutique: Option<&str>, refs: &References) -> Vec<String> {
    let gardes = os_gardes(boutique, refs);
    let mut l: Vec<String> = Vec::new();
    for s in sauvegardes.as_array().into_iter().flatten() {
        if s["os_inconnu"].as_bool() == Some(true) || s["nature"].as_str() == Some("registre") {
            continue;
        }
        let Some(os) = s["os"].as_str().map(str::trim).filter(|o| !o.is_empty()) else { continue };
        if s["chemin"].as_str().is_some_and(chemin_de_mac_ou_linux) {
            if !l.iter().any(|x| x == "Mac ou Linux") {
                l.push("Mac ou Linux".into());
            }
            continue;
        }
        let autre_boutique = s["boutique"].as_str().map(str::trim).filter(|b| !b.is_empty()).is_some_and(|b| !boutique.is_some_and(|x| meme_boutique(x, b)));
        if (!gardes.iter().any(|g| g == os) || autre_boutique) && !l.iter().any(|x| x == os) {
            l.push(os.to_string());
        }
    }
    l
}

/// Les emplacements d'une réponse de Firehouse (`sauvegardes`) qui valent pour ce jeu, sans doublon ; `None` : Firehouse
/// ne sait pas (`null`, ou absent). Écartés : un `os` inconnu (`os_inconnu`), une autre boutique, le registre
/// (`nature: registre`, jamais un fichier).
pub fn depuis_firehouse(sauvegardes: &Value, boutique: Option<&str>, refs: &References, source: Option<&str>, quand: &str) -> Option<Vec<EmplacementPartie>> {
    let liste = sauvegardes.as_array()?;
    let gardes = os_gardes(boutique, refs);
    let mut l: Vec<EmplacementPartie> = Vec::new();
    for s in liste {
        if s["os_inconnu"].as_bool() == Some(true) {
            continue;
        }
        let Some(os) = s["os"].as_str().filter(|o| gardes.iter().any(|g| g == o)) else { continue };
        let b = s["boutique"].as_str().map(str::trim).filter(|b| !b.is_empty());
        // Une ligne d'une AUTRE boutique : jamais pour ce jeu.
        if b.is_some_and(|b| !boutique.is_some_and(|x| meme_boutique(x, b))) {
            continue;
        }
        let Some(chemin) = s["chemin"].as_str().map(str::trim).filter(|c| !c.is_empty()) else { continue };
        if chemin_de_mac_ou_linux(chemin) {
            continue;
        }
        let nature = crate::nuage_steam::nature_de(chemin, s["nature"].as_str());
        if !["dossier", "fichier", "motif"].contains(&nature.as_str()) {
            continue;
        }
        // Le même chemin pour « Windows » et pour la boutique : une seule ligne (la première).
        if l.iter().any(|e| e.chemin.eq_ignore_ascii_case(chemin) && e.nature == nature) {
            continue;
        }
        l.push(EmplacementPartie {
            chemin: chemin.to_string(),
            nature,
            os: os.to_string(),
            origine: "firehouse".into(),
            nuage_boutique: s["nuage"].as_bool() == Some(true),
            lu_le: Some(quand.to_string()),
            source: source.map(String::from),
            boutique: b.map(String::from),
            etiquettes: s["etiquettes"].as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from)).collect(),
            releve_sur: s["releve_sur"].as_str().map(str::trim).filter(|r| !r.is_empty()).map(String::from),
            ..Default::default()
        });
    }
    Some(l)
}

/// Range la réponse de Firehouse (`/jeu/{id}/triche` ou `/triche/recherche`, entière) pour ce jeu : un `null` — ou
/// `emplacements_source.indisponible` (contrat 1.48 : INCONNU, jamais « rien à sauvegarder ») — ne change rien de ce
/// qui était connu (seule l'heure de la demande avance) ; une liste remplace les emplacements venus de Firehouse (ceux
/// d'une autre origine restent).
pub fn appliquer(c: &mut Cache, id: i64, boutique: Option<BoutiqueJeu>, reponse: &Value, refs: &References, quand: &str) {
    let e = c.jeux.entry(id.to_string()).or_insert_with(|| PartiesJeu { id, ..Default::default() });
    e.id = id;
    e.demande_le = Some(quand.to_string());
    if boutique.is_some() {
        e.boutique = boutique;
    }
    let texte = |v: &Value| v.as_str().map(str::trim).filter(|r| !r.is_empty()).map(String::from);
    // Le moteur (contrat 1.48) : rangé tel quel, les règles viendront au lot S6.
    if let Some(nom) = texte(&reponse["emplacements_moteur"]["moteur_nom"]).or_else(|| texte(&reponse["moteur"])) {
        let preuve = texte(&reponse["emplacements_moteur"]["source_url"]).map(|u| format!("Firehouse ({u})")).unwrap_or_else(|| "Firehouse".into());
        e.moteur = Some(Moteur { nom, preuve });
    }
    if reponse["emplacements_moteur"].is_object() {
        e.moteur_firehouse = Some(reponse["emplacements_moteur"].clone());
    }
    let src = &reponse["emplacements_source"];
    if src["indisponible"].as_bool() == Some(true) {
        return;
    }
    if let Some(r) = texte(&src["reconnu_par"]).or_else(|| texte(&reponse["reconnu_par"])) {
        e.reconnu_par = Some(r);
    }
    if !src["dossier_ecarte"].is_null() && src.is_object() {
        e.dossier_ecarte = Some(src["dossier_ecarte"].clone());
    }
    let source = texte(&src["source"]);
    let b = e.boutique.as_ref().map(|b| b.source.clone());
    let Some(neufs) = depuis_firehouse(&reponse["sauvegardes"], b.as_deref(), refs, source.as_deref(), quand) else { return };
    e.autres_systemes = systemes_ecartes(&reponse["sauvegardes"], b.as_deref(), refs);
    e.emplacements.retain(|x| x.origine != "firehouse");
    // Rien pour ce PC, mais des emplacements pour un autre système (TerraScape, 1.48 : tout en « Steam » pour un jeu
    // Epic) ou seulement des règles de moteur (lot S6) : INCONNU sur ce PC, jamais « aucun » — et rien de deviné.
    e.firehouse = if !neufs.is_empty() {
        EtatFirehouse::Lu
    } else if reponse["emplacements_moteur"].is_object() || !e.autres_systemes.is_empty() {
        EtatFirehouse::Inconnu
    } else {
        EtatFirehouse::Vide
    };
    e.emplacements.extend(neufs);
    retirer_doublons(e);
    e.lu_le = Some(quand.to_string());
    if src.is_object() {
        e.source = source;
        e.source_nom = texte(&src["nom"]);
        e.source_url = texte(&src["source_url"]);
    }
}

/// Le fichier des références, COMMUN au PC (contrat 1.48 : identiques pour tous les grades, rien d'un jeu).
pub const NOM_REFERENCES: &str = "sauvegardes-references.json";

/// Les références se relisent au plus une fois par semaine.
pub const DELAI_REFERENCES: u64 = 7 * 24 * 3600;

/// Les références gardées sur ce PC : (lues le, références) ; `None` si jamais lues (ou illisibles).
pub fn references_gardees(dossier: &Path) -> Option<(u64, References)> {
    let v: Value = serde_json::from_slice(&std::fs::read(dossier.join(NOM_REFERENCES)).ok()?).ok()?;
    Some((v["lu_le"].as_str()?.parse().ok()?, lire_references(&v["references"])?))
}

/// Garde la réponse de `/sauvegardes/references` (fichier provisoire renommé ensuite).
pub fn garder_references(dossier: &Path, reponse: &Value, quand: &str) -> Resultat<()> {
    std::fs::create_dir_all(dossier)?;
    let provisoire = dossier.join(format!("{NOM_REFERENCES}.provisoire"));
    let octets = serde_json::to_vec_pretty(&serde_json::json!({"lu_le": quand, "references": reponse})).map_err(|_| Erreur::Disque("Références impossibles à écrire.".into()))?;
    std::fs::write(&provisoire, octets)?;
    if let Err(e) = std::fs::rename(&provisoire, dossier.join(NOM_REFERENCES)) {
        let _ = std::fs::remove_file(&provisoire);
        return Err(e.into());
    }
    Ok(())
}

/// Faut-il (re)demander ce jeu à Firehouse ? Jamais avant 30 jours pour un jeu décrit, un jour pour un jeu inconnu.
pub fn a_redemander(e: Option<&PartiesJeu>, maintenant: u64) -> bool {
    let Some(e) = e else { return true };
    let Some(d) = e.demande_le.as_deref().and_then(|d| d.parse::<u64>().ok()) else { return true };
    let delai = if e.firehouse == EtatFirehouse::Inconnu { DELAI_INCONNU } else { DELAI_LU };
    maintenant.saturating_sub(d) >= delai
}

/// Pour les jeux que `garder` refuse (un jeu que le profil ne voit plus : grade changé), oublie TOUT ce que Frogtend en
/// sait : ce qui vient de Firehouse, le moteur reconnu et ses emplacements (lot S6 : avant, une ligne `moteur` ou
/// `manifeste` restait, et avec elle le chemin — donc le titre — d'un jeu qui n'existe plus pour ce profil). Seuls les
/// choix que la PERSONNE de ce profil a faits elle-même (`personne`, futur éditeur ; correctif 10 du lot S1) restent :
/// ils viennent d'elle et resservent si le jeu lui redevient visible ; l'entrée part s'il n'y en a pas. Rend combien
/// de jeux ont été touchés.
pub fn retirer_si(c: &mut Cache, garder: impl Fn(i64) -> bool) -> usize {
    let mut n = 0;
    c.jeux.retain(|_, e| {
        if garder(e.id) {
            return true;
        }
        n += 1;
        e.emplacements.retain(|x| x.origine == "personne");
        if e.emplacements.is_empty() {
            return false;
        }
        let (id, boutique, emplacements) = (e.id, e.boutique.take(), std::mem::take(&mut e.emplacements));
        *e = PartiesJeu { id, boutique, emplacements, ..Default::default() };
        true
    });
    n
}

/// Le rang d'une origine : `personne` > Firehouse > `manifeste` > `moteur`.
fn rang(origine: &str) -> u8 {
    match origine {
        "personne" => 0,
        "firehouse" => 1,
        "manifeste" => 2,
        "moteur" => 3,
        _ => 4,
    }
}

/// Retire les doublons (même chemin NON résolu, sans tenir compte des majuscules ni du sens des barres, et même
/// nature) : la ligne de la meilleure origine reste (`personne` > Firehouse > moteur), sinon la première.
pub fn retirer_doublons(e: &mut PartiesJeu) {
    let cle = |x: &EmplacementPartie| (x.chemin.trim().replace('/', "\\").trim_end_matches('\\').to_lowercase(), x.nature.clone());
    let l = std::mem::take(&mut e.emplacements);
    let garde: Vec<bool> = l.iter().enumerate().map(|(i, x)| !l.iter().enumerate().any(|(j, y)| j != i && cle(x) == cle(y) && (rang(&y.origine), j) < (rang(&x.origine), i))).collect();
    e.emplacements = l.into_iter().zip(garde).filter(|(_, g)| *g).map(|(x, _)| x).collect();
}

/// Range pour ce jeu ce que le DISQUE dit de son moteur (lot S6). Les lignes `moteur` d'avant sont toujours remplacées.
/// Prouvé (deux preuves) : ses emplacements s'ajoutent quand Firehouse ne sait pas (`Inconnu`) ou en connaît (`Lu` :
/// les doublons partent, Firehouse et la personne l'emportent) ; quand Firehouse dit « aucune partie » (`Vide`),
/// seulement si des parties EXISTENT à ces endroits (`parties_sur_disque`) — le moteur l'emporte et le désaccord est
/// noté. Rien de prouvé : plus aucune ligne `moteur`. Une entrée absente n'est créée que pour un moteur prouvé.
pub fn appliquer_moteur(c: &mut Cache, id: i64, r: &crate::moteurs_parties::Reconnaissance, parties_sur_disque: bool) {
    use crate::moteurs_parties::Reconnaissance;
    let prouve = match r {
        Reconnaissance::Prouve(x) => Some(x),
        _ => None,
    };
    if prouve.is_none() && c.jeu(id).is_none() {
        return;
    }
    let e = c.jeux.entry(id.to_string()).or_insert_with(|| PartiesJeu { id, ..Default::default() });
    e.emplacements.retain(|x| x.origine != "moteur");
    e.desaccord_moteur = false;
    e.moteur_contredit = false;
    let Some(x) = prouve else {
        e.moteur_reconnu = None;
        return;
    };
    e.moteur_reconnu = Some(MoteurReconnu { cle: x.cle.clone(), nom: x.nom.clone(), preuve: x.preuve(), noms_lus: x.noms_lus.clone(), marqueur: x.marqueur.clone(), nom_lu: x.nom_lu.clone() });
    e.moteur_contredit = x.firehouse_contredit;
    let applique = match e.firehouse {
        EtatFirehouse::Vide => parties_sur_disque,
        EtatFirehouse::Inconnu | EtatFirehouse::Lu => true,
    };
    if !applique || x.emplacements.is_empty() {
        return;
    }
    e.desaccord_moteur = e.firehouse == EtatFirehouse::Vide;
    e.emplacements.extend(x.emplacements.iter().cloned());
    retirer_doublons(e);
}

/// Un fichier de JOURNAL du jeu (pas une partie) : `*.log`, `output_log.txt`, `Player.log`, `Player-prev.log`.
pub fn est_un_journal(nom: &str) -> bool {
    let n = nom.to_lowercase();
    n.ends_with(".log") || n == "output_log.txt"
}

/// Des PARTIES existent-elles à l'un de ces emplacements sur ce PC (lot S6, pour renverser un « aucune partie » de
/// Firehouse) ? Au moins un fichier qui n'est pas un journal du jeu (`est_un_journal`) ; sans lien ni jonction suivis,
/// rien n'est lu.
pub fn contient_des_parties(e: &PartiesJeu, ctx: &Contexte) -> bool {
    e.emplacements.iter().any(|x| {
        crate::nuage_steam::candidats(&x.chemin, Some(&x.nature), ctx).0.iter().any(|c| {
            if !c.dossier.is_dir() {
                return false;
            }
            let nom_ok = |r: &str| !est_un_journal(r.rsplit(['/', '\\']).next().unwrap_or(r));
            match &c.filtre {
                None => crate::nuage_steam::parcourir(&c.dossier).is_ok_and(|l| l.iter().any(|r| nom_ok(r))),
                Some(f) => std::fs::read_dir(&c.dossier).is_ok_and(|l| {
                    l.flatten().any(|x| x.file_type().is_ok_and(|t| t.is_file()) && {
                        let n = x.file_name().to_string_lossy().to_string();
                        crate::nuage_steam::repond(&n, f) && nom_ok(&n)
                    })
                }),
            }
        })
    })
}

/// Un emplacement tel qu'il est sur CE PC, pour l'écran seulement (jamais rangé, jamais au journal).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EmplacementVu {
    /// Le rang de l'emplacement dans le cache.
    pub n: usize,
    /// Le dossier résolu sur ce PC ; `None` : rien à montrer (illisible ici, ou aucun dossier de profil encore).
    pub dossier: Option<String>,
    /// Seulement les fichiers de ce nom (ou de ce motif).
    pub filtre: Option<String>,
    pub nature: String,
    pub origine: String,
    pub os: String,
    /// La source de Firehouse (`ludusavi`, `pcgamingwiki`), pour dire « d'après … ».
    pub source: Option<String>,
    /// Lot S6, origine `moteur` : le nom du moteur (« 📁 d'après le moteur Unity »).
    pub moteur: Option<String>,
    pub nuage_boutique: bool,
    /// Frogtend sait lire ce chemin sur ce PC (jetons connus, pas une racine).
    pub lisible: bool,
    pub existe: bool,
    pub fichiers: u64,
    pub octets: u64,
    /// Parmi `fichiers`, ceux qui sont seulement dans le nuage (OneDrive « en ligne seulement ») : pas sur ce PC
    /// (jamais lus pour être comptés : leur taille est celle que OneDrive annonce).
    #[serde(default)]
    pub en_ligne: u64,
    /// Trop de fichiers, de dossiers, ou trop profond pour un dossier de parties : pas comptés.
    pub trop: bool,
    /// Le dossier existe mais ne se lit pas (droits…) : pas compté.
    pub illisible: bool,
}

/// Compte les fichiers d'un emplacement (sans suivre un lien ni une jonction, rien n'est lu).
/// Ce qu'un emplacement contient sur ce PC.
#[derive(Debug, Clone, Copy, Default)]
struct Mesure {
    existe: bool,
    fichiers: u64,
    octets: u64,
    en_ligne: u64,
    trop: bool,
    illisible: bool,
}

/// Compte les fichiers d'un emplacement (sans suivre un lien ni une jonction, rien n'est lu) ; `compter` faux : dit
/// seulement s'il existe (« Ouvrir le dossier »).
fn mesurer(e: &crate::nuage_steam::Emplacement, compter: bool) -> Mesure {
    if !e.dossier.is_dir() {
        return Mesure::default();
    }
    if !compter {
        return Mesure { existe: true, ..Default::default() };
    }
    // (taille, en ligne seulement) d'un fichier, sans le lire.
    let vu = |p: &Path| std::fs::symlink_metadata(p).map(|m| (m.len(), crate::nuage_steam::en_ligne_seulement(&m))).unwrap_or((0, false));
    let compter = |l: Vec<PathBuf>| {
        let v: Vec<(u64, bool)> = l.iter().map(|p| vu(p)).collect();
        Mesure { existe: true, fichiers: v.len() as u64, octets: v.iter().map(|x| x.0).sum(), en_ligne: v.iter().filter(|x| x.1).count() as u64, ..Default::default() }
    };
    match &e.filtre {
        None => match crate::nuage_steam::parcourir(&e.dossier) {
            Ok(l) => compter(l.iter().map(|r| e.dossier.join(r)).collect()),
            Err(crate::nuage_steam::Arret::Trop) => Mesure { existe: true, trop: true, ..Default::default() },
            Err(crate::nuage_steam::Arret::Illisible) => Mesure { existe: true, illisible: true, ..Default::default() },
        },
        Some(f) => match std::fs::read_dir(&e.dossier) {
            Ok(entrees) => {
                let l: Vec<PathBuf> = entrees
                    .flatten()
                    .filter(|x| x.file_type().is_ok_and(|t| t.is_file()))
                    .filter(|x| crate::nuage_steam::repond(&x.file_name().to_string_lossy(), f))
                    .map(|x| x.path())
                    .collect();
                compter(l)
            }
            Err(_) => Mesure { existe: true, illisible: true, ..Default::default() },
        },
    }
}

/// Les emplacements d'un jeu résolus sur CE PC, avec ce qu'ils contiennent (lecture seule ; `compter` faux : sans
/// parcourir les dossiers).
pub fn resoudre(e: &PartiesJeu, ctx: &Contexte, compter: bool) -> Vec<EmplacementVu> {
    let mut l = Vec::new();
    for (n, x) in e.emplacements.iter().enumerate() {
        let vu = |dossier: Option<String>, filtre: Option<String>, lisible: bool, m: Mesure| EmplacementVu {
            n,
            dossier,
            filtre,
            nature: x.nature.clone(),
            origine: x.origine.clone(),
            os: x.os.clone(),
            source: x.source.clone(),
            moteur: x.moteur.clone(),
            nuage_boutique: x.nuage_boutique,
            lisible,
            existe: m.existe,
            fichiers: m.fichiers,
            octets: m.octets,
            en_ligne: m.en_ligne,
            trop: m.trop,
            illisible: m.illisible,
        };
        let (candidats, illisibles) = crate::nuage_steam::candidats(&x.chemin, Some(&x.nature), ctx);
        if candidats.is_empty() {
            l.push(vu(None, None, illisibles == 0, Mesure::default()));
            continue;
        }
        for c in candidats {
            let m = mesurer(&c, compter);
            l.push(vu(Some(c.dossier.to_string_lossy().to_string()), c.filtre.clone(), true, m));
        }
    }
    l
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn secours() -> References {
        references_de_secours()
    }

    /// La vraie réponse de Firehouse pour Cyberpunk 2077 (relevé S0 du 22/11, `/triche/recherche` titre + Windows).
    fn cyberpunk() -> Value {
        serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/triche-cyberpunk-sauvegardes.json")).unwrap()).unwrap()
    }

    fn gog() -> Option<BoutiqueJeu> {
        Some(BoutiqueJeu { source: "gog".into(), cle: "1423049311".into() })
    }

    #[test]
    fn les_emplacements_de_cyberpunk_suivent_la_boutique() {
        let r = cyberpunk();
        // Un jeu GOG : Windows gardé ; OS X et Microsoft Store écartés.
        let l = depuis_firehouse(&r["sauvegardes"], Some("gog"), &secours(), None, "100").unwrap();
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].chemin, r"%USERPROFILE%\Saved Games\CD Projekt Red\Cyberpunk 2077");
        assert_eq!((l[0].nature.as_str(), l[0].os.as_str(), l[0].origine.as_str(), l[0].nuage_boutique, l[0].lu_le.as_deref()), ("dossier", "Windows", "firehouse", false, Some("100")));
        // Un jeu Xbox : Microsoft Store gardé aussi ; jamais OS X.
        let x = depuis_firehouse(&r["sauvegardes"], Some("xboxone"), &secours(), None, "100").unwrap();
        assert_eq!(x.iter().map(|e| e.os.as_str()).collect::<Vec<_>>(), ["Windows", "Microsoft Store"]);
        assert!(x[1].chemin.contains(r"\Packages\CDPROJEKTSA."));
        // Sans boutique (jeu de Firehouse, importé) : Windows seul.
        assert_eq!(depuis_firehouse(&r["sauvegardes"], None, &secours(), None, "100").unwrap().len(), 1);
        // La configuration n'est pas un emplacement de parties.
        assert!(x.iter().all(|e| e.chemin != r"%LOCALAPPDATA%\CD Projekt Red\Cyberpunk 2077"));
    }

    #[test]
    fn chaque_boutique_garde_son_os_et_jamais_linux_ni_os_x_ni_dos() {
        assert_eq!(os_gardes(None, &secours()), ["Windows"]);
        assert_eq!(os_gardes(Some("steam"), &secours()), ["Windows", "Steam"]);
        assert_eq!(os_gardes(Some("epic"), &secours()), ["Windows", "Epic Games Store"]);
        assert_eq!(os_gardes(Some("gog"), &secours()), ["Windows", "GOG.com"]);
        assert_eq!(os_gardes(Some("xboxone"), &secours()), ["Windows", "Microsoft Store"]);
        assert_eq!(os_gardes(Some("uplay"), &secours()), ["Windows", "Ubisoft Connect"]);
        assert_eq!(os_gardes(Some("amazon"), &secours()), ["Windows", "Amazon Games"]);
        assert_eq!(os_gardes(Some("origin"), &secours()), ["Windows", "EA app"]);
        assert_eq!(os_gardes(Some("battlenet"), &secours()), ["Windows"]);
        let s = json!([
            {"os": "Linux", "chemin": "$HOME/.local/share/x", "nature": "dossier"},
            {"os": "Mac", "chemin": "~/Library/x", "nature": "dossier"},
            {"os": "DOS", "chemin": r"<dossier du jeu>\SAVE", "nature": "dossier"},
            {"os": "Steam", "chemin": r"<Steam>\userdata\<identifiant Steam>\1\remote", "steam_userdata": true, "nuage": true, "boutique": "steam"},
            {"os": "Epic Games Store", "chemin": r"%LOCALAPPDATA%\Jeu\Saved", "nature": "dossier"},
            {"os": "Windows", "chemin": r"%LOCALAPPDATA%\Jeu\Saved", "nature": "dossier"},
            {"os": "Windows", "chemin": r"<dossier du jeu>\save*.sav"}
        ]);
        let epic = depuis_firehouse(&s, Some("epic"), &secours(), None, "1").unwrap();
        assert_eq!(epic.iter().map(|e| e.os.as_str()).collect::<Vec<_>>(), ["Epic Games Store", "Windows"], "le même chemin une seule fois");
        assert_eq!(epic[1].nature, "motif", "nature déduite quand Firehouse ne la dit pas");
        assert_eq!(systemes_ecartes(&s, Some("epic"), &secours()), ["Linux", "Mac", "DOS", "Steam"]);
        let steam = depuis_firehouse(&s, Some("steam"), &secours(), None, "1").unwrap();
        assert_eq!(steam[0].os, "Steam");
        assert!(steam[0].nuage_boutique, "Steam synchronise ce dossier");
        assert!(steam.iter().all(|e| !["Linux", "Mac", "DOS"].contains(&e.os.as_str())));
        assert_eq!(depuis_firehouse(&Value::Null, Some("epic"), &secours(), None, "1"), None);
        assert_eq!(depuis_firehouse(&json!([]), Some("epic"), &secours(), None, "1"), Some(vec![]));
    }

    #[test]
    fn un_null_n_efface_jamais_et_une_liste_remplace() {
        let mut c = Cache::default();
        // Jamais demandé : un null donne « inconnu ».
        appliquer(&mut c, -7, gog(), &json!({"sauvegardes": null}), &secours(), "10");
        assert_eq!((c.jeu(-7).unwrap().firehouse, c.jeu(-7).unwrap().demande_le.as_deref()), (EtatFirehouse::Inconnu, Some("10")));
        // Une réponse : rangée.
        appliquer(&mut c, -7, gog(), &cyberpunk(), &secours(), "20");
        let e = c.jeu(-7).unwrap().clone();
        assert_eq!((e.firehouse, e.lu_le.as_deref(), e.emplacements.len()), (EtatFirehouse::Lu, Some("20"), 1));
        assert_eq!(e.boutique, gog());
        assert_eq!(e.moteur, None, "le moteur viendra du lot S6");
        // Un null ensuite (PCGamingWiki muet) : rien n'est perdu ; seule l'heure de la demande avance.
        appliquer(&mut c, -7, gog(), &json!({"codes": []}), &secours(), "30");
        let apres = c.jeu(-7).unwrap();
        assert_eq!((apres.firehouse, apres.lu_le.as_deref(), apres.emplacements.clone()), (EtatFirehouse::Lu, Some("20"), e.emplacements.clone()));
        assert_eq!(apres.demande_le.as_deref(), Some("30"));
        // Une personne a ajouté un emplacement (futur éditeur) : une liste vide de Firehouse ne l'efface pas.
        c.jeux.get_mut("-7").unwrap().emplacements.push(EmplacementPartie { chemin: r"D:\Mes parties".into(), nature: "dossier".into(), os: "Windows".into(), origine: "personne".into(), nuage_boutique: false, lu_le: None, ..Default::default() });
        appliquer(&mut c, -7, gog(), &json!({"sauvegardes": []}), &secours(), "40");
        let vide = c.jeu(-7).unwrap();
        assert_eq!(vide.firehouse, EtatFirehouse::Vide);
        assert_eq!(vide.emplacements.iter().map(|e| e.origine.as_str()).collect::<Vec<_>>(), ["personne"], "[] remplace ce qui venait de Firehouse seulement");
        // Seulement des lignes Mac : rien pour ce PC, et c'est dit (« connu seulement pour Mac »), jamais « aucun ».
        appliquer(&mut c, 3, None, &json!({"sauvegardes": [{"os": "Mac", "chemin": "~/x"}], "reconnu_par": "titre"}), &secours(), "50");
        let mac = c.jeu(3).unwrap();
        assert_eq!((mac.firehouse, mac.reconnu_par.as_deref(), mac.autres_systemes.clone()), (EtatFirehouse::Inconnu, Some("titre"), vec!["Mac".to_string()]));
    }

    #[test]
    fn pas_de_nouvelle_demande_avant_trente_jours() {
        let jour = 24 * 3600;
        assert!(a_redemander(None, 1_000), "jamais demandé");
        let mut c = Cache::default();
        appliquer(&mut c, 1, None, &cyberpunk(), &secours(), "1000000");
        let e = c.jeu(1).cloned();
        assert!(!a_redemander(e.as_ref(), 1_000_000 + jour));
        assert!(!a_redemander(e.as_ref(), 1_000_000 + 29 * jour));
        assert!(a_redemander(e.as_ref(), 1_000_000 + 30 * jour));
        // « Aucun emplacement » : 30 jours aussi.
        appliquer(&mut c, 2, None, &json!({"sauvegardes": []}), &secours(), "1000000");
        assert!(!a_redemander(c.jeu(2), 1_000_000 + 29 * jour));
        // Inconnu (null) : redemandé le lendemain seulement, à l'ouverture de la fiche.
        appliquer(&mut c, 3, None, &json!({"sauvegardes": null}), &secours(), "1000000");
        assert!(!a_redemander(c.jeu(3), 1_000_000 + 3600));
        assert!(a_redemander(c.jeu(3), 1_000_000 + jour));
        // Une heure illisible : redemandé.
        let mut x = c.jeu(1).unwrap().clone();
        x.demande_le = Some("hier".into());
        assert!(a_redemander(Some(&x), 0));
    }

    #[test]
    fn chaque_profil_a_son_cache_et_un_fichier_illisible_n_est_jamais_ecrase() {
        let d = tempfile::tempdir().unwrap();
        let (a, b) = (d.path().join("profils").join("p1"), d.path().join("profils").join("p2"));
        modifier(&a, |c| appliquer(c, -7, gog(), &cyberpunk(), &secours(), "20")).unwrap();
        assert_eq!(fichier(&a), a.join("emplacements-parties.json"), "dans le dossier du profil, jamais un dossier commun");
        assert!(lire(&a).unwrap().jeu(-7).is_some());
        assert!(lire(&b).unwrap().jeux.is_empty(), "l'autre profil ne voit rien");
        assert!(!b.exists(), "rien d'écrit pour l'autre profil");
        modifier(&b, |c| appliquer(c, 5, None, &json!({"sauvegardes": []}), &secours(), "20")).unwrap();
        assert!(lire(&a).unwrap().jeu(5).is_none() && lire(&b).unwrap().jeu(-7).is_none());
        // Relu tel qu'écrit.
        let relu = lire(&a).unwrap();
        assert_eq!(relu.version, VERSION);
        assert_eq!(relu.jeu(-7).unwrap().emplacements[0].chemin, r"%USERPROFILE%\Saved Games\CD Projekt Red\Cyberpunk 2077");
        assert!(!a.join("emplacements-parties.json.provisoire").exists());
        // Rien de changé : pas réécrit.
        let avant = std::fs::metadata(fichier(&a)).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        modifier(&a, |_| ()).unwrap();
        assert_eq!(std::fs::metadata(fichier(&a)).unwrap().modified().unwrap(), avant);
        // Illisible : erreur, jamais écrasé.
        std::fs::write(fichier(&b), b"{abime").unwrap();
        assert!(lire(&b).is_err());
        assert!(modifier(&b, |c| appliquer(c, 9, None, &json!({"sauvegardes": []}), &secours(), "1")).is_err());
        assert_eq!(std::fs::read(fichier(&b)).unwrap(), b"{abime");
        // Écrit par une version plus récente : jamais réécrit.
        std::fs::write(fichier(&b), br#"{"version": 9, "jeux": {}}"#).unwrap();
        assert!(modifier(&b, |c| appliquer(c, 9, None, &json!({"sauvegardes": []}), &secours(), "1")).is_err());
    }

    #[test]
    fn un_jeu_que_le_profil_ne_voit_plus_quitte_le_cache() {
        let mut c = Cache::default();
        appliquer(&mut c, 1, None, &cyberpunk(), &secours(), "1");
        appliquer(&mut c, 2, None, &cyberpunk(), &secours(), "1");
        assert_eq!(retirer_si(&mut c, |id| id != 2), 1);
        assert!(c.jeu(1).is_some() && c.jeu(2).is_none());
    }

    #[test]
    fn les_jetons_se_resolvent_sur_ce_pc_et_les_fichiers_se_comptent() {
        let d = tempfile::tempdir().unwrap();
        let parties = d.path().join("Parties");
        let jeu = d.path().join("Jeux").join("Cyberpunk 2077");
        std::fs::create_dir_all(parties.join("CD Projekt Red").join("Cyberpunk 2077").join("ManualSave-0")).unwrap();
        std::fs::write(parties.join("CD Projekt Red").join("Cyberpunk 2077").join("ManualSave-0").join("sav.dat"), b"12345").unwrap();
        std::fs::write(parties.join("CD Projekt Red").join("Cyberpunk 2077").join("UserSettings.json"), b"{}").unwrap();
        std::fs::create_dir_all(jeu.join("saves")).unwrap();
        std::fs::write(jeu.join("saves").join("slot1.sav"), b"abc").unwrap();
        std::fs::write(jeu.join("saves").join("notes.txt"), b"x").unwrap();
        let ctx = Contexte { jeu: Some(jeu.clone()), steam: None, compte: None, windows: vec![("Parties-enregistrees", parties.clone())], installation: None, boutique: None };
        let e = PartiesJeu {
            id: -7,
            emplacements: vec![
                EmplacementPartie { chemin: r"%USERPROFILE%\Saved Games\CD Projekt Red\Cyberpunk 2077".into(), nature: "dossier".into(), os: "Windows".into(), origine: "firehouse".into(), nuage_boutique: false, lu_le: None, ..Default::default() },
                EmplacementPartie { chemin: r"<dossier du jeu>\saves\*.sav".into(), nature: "motif".into(), os: "Windows".into(), origine: "firehouse".into(), nuage_boutique: false, lu_le: None, ..Default::default() },
                EmplacementPartie { chemin: r"%USERPROFILE%\Saved Games\Absent".into(), nature: "dossier".into(), os: "Windows".into(), origine: "firehouse".into(), nuage_boutique: false, lu_le: None, ..Default::default() },
                EmplacementPartie { chemin: r"<Jeton inconnu>\x".into(), nature: "dossier".into(), os: "Windows".into(), origine: "firehouse".into(), nuage_boutique: false, lu_le: None, ..Default::default() },
                EmplacementPartie { chemin: r"HKEY_CURRENT_USER\Software\Jeu".into(), nature: "dossier".into(), os: "Windows".into(), origine: "firehouse".into(), nuage_boutique: false, lu_le: None, ..Default::default() },
            ],
            ..Default::default()
        };
        let l = resoudre(&e, &ctx, true);
        assert_eq!(l.len(), 5);
        let cp = parties.join("CD Projekt Red").join("Cyberpunk 2077");
        assert_eq!(l[0].dossier.as_deref(), Some(cp.to_string_lossy().as_ref()), "%USERPROFILE%\\Saved Games : le vrai dossier de ce PC");
        assert_eq!((l[0].lisible, l[0].existe, l[0].fichiers, l[0].octets, l[0].trop), (true, true, 2, 7, false), "sous-dossiers compris");
        assert_eq!(l[1].dossier.as_deref(), Some(jeu.join("saves").to_string_lossy().as_ref()));
        assert_eq!((l[1].filtre.as_deref(), l[1].fichiers, l[1].octets), (Some("*.sav"), 1, 3), "seulement les fichiers du motif");
        assert_eq!((l[2].existe, l[2].fichiers, l[2].lisible), (false, 0, true), "pas encore de parties");
        assert_eq!((l[3].dossier.as_deref(), l[3].lisible), (None, false), "jeton inconnu : jamais deviné");
        assert_eq!((l[4].dossier.as_deref(), l[4].lisible), (None, false), "registre : pas un dossier");
        assert_eq!(l.iter().map(|v| v.n).collect::<Vec<_>>(), [0, 1, 2, 3, 4]);
        // Sans dossier du jeu connu : illisible, rien de deviné.
        let sans = Contexte { jeu: None, steam: None, compte: None, windows: vec![], installation: None, boutique: None };
        assert!(!resoudre(&e, &sans, true)[1].lisible);
    }

    #[test]
    fn l_identifiant_utilisateur_enumere_les_profils_du_jeu() {
        let d = tempfile::tempdir().unwrap();
        let docs = d.path().join("Docs");
        for u in ["111", "222"] {
            std::fs::create_dir_all(docs.join("Jeu").join(u).join("saves")).unwrap();
        }
        std::fs::write(docs.join("Jeu").join("111").join("saves").join("a.sav"), b"a").unwrap();
        let ctx = Contexte { jeu: None, steam: None, compte: None, windows: vec![("Documents", docs.clone())], installation: None, boutique: None };
        let e = PartiesJeu {
            id: 1,
            emplacements: vec![EmplacementPartie { chemin: r"%USERPROFILE%\Documents\Jeu\<identifiant utilisateur>\saves".into(), nature: "dossier".into(), os: "Windows".into(), origine: "firehouse".into(), nuage_boutique: false, lu_le: None, ..Default::default() }],
            ..Default::default()
        };
        let l = resoudre(&e, &ctx, true);
        assert_eq!(l.len(), 2, "un par profil du jeu");
        assert_eq!(l.iter().map(|v| v.fichiers).collect::<Vec<_>>(), [1, 0]);
        // Le dossier parent absent : aucun dossier de profil, mais lisible.
        let vide = Contexte { jeu: None, steam: None, compte: None, windows: vec![("Documents", d.path().join("Absent"))], installation: None, boutique: None };
        let l = resoudre(&e, &vide, true);
        assert_eq!((l.len(), l[0].dossier.as_deref(), l[0].lisible), (1, None, true));
    }

    /// Contrat 1.48 : les réponses RÉELLES relevées le 22/11 (lecture seule) ; 1.49 : SIMULÉE (pas encore servie).
    fn fixture(nom: &str) -> Value {
        serde_json::from_str(&std::fs::read_to_string(format!("{}/fixtures/{nom}", env!("CARGO_MANIFEST_DIR"))).unwrap()).unwrap()
    }

    fn refs_148() -> References {
        lire_references(&fixture("sauvegardes-references-148-reel.json")).unwrap()
    }

    fn epic() -> Option<BoutiqueJeu> {
        Some(BoutiqueJeu { source: "epic".into(), cle: "TerraScape".into() })
    }

    #[test]
    fn les_references_reelles_valident_les_noms_de_la_table_de_frogtend() {
        let r = refs_148();
        // L'ordre ne compte pas (un objet JSON).
        let trie = |mut l: Vec<String>| {
            l.sort();
            l
        };
        assert_eq!(trie(r.os.clone()), trie(references_de_secours().os), "la liste de secours est celle du relevé (11 noms)");
        assert_eq!(r.os.len(), 11);
        for (b, os) in [("steam", "Steam"), ("epic", "Epic Games Store"), ("gog", "GOG.com"), ("xboxone", "Microsoft Store"), ("uplay", "Ubisoft Connect"), ("origin", "EA app"), ("amazon", "Amazon Games")] {
            assert_eq!(os_gardes(Some(b), &r), ["Windows", os], "{b}");
        }
        assert_eq!(os_gardes(None, &r), ["Windows"]);
        // La route fait foi : un nom de la table qu'elle ne connaît pas n'est pas gardé.
        let seule = lire_references(&json!({"os": {"Windows": "tout jeu", "Steam": "un jeu Steam"}})).unwrap();
        assert_eq!(os_gardes(Some("gog"), &seule), ["Windows"]);
        assert_eq!(os_gardes(Some("steam"), &seule), ["Windows", "Steam"]);
        assert_eq!(lire_references(&json!({"ok": false})), None, "sans `os` : rien de deviné (la liste gardée ou de secours sert)");
        // `boutique` des lignes (relevé : null, « steam », « microsoft ») rapprochée de nos sources.
        assert!(meme_boutique("xboxone", "microsoft") && meme_boutique("steam", "steam") && !meme_boutique("gog", "steam") && !meme_boutique("epic", "microsoft"));
    }

    #[test]
    fn la_reponse_reelle_de_cyberpunk_gog_est_rangee_avec_sa_source() {
        let r = refs_148();
        let reponse = fixture("triche-cyberpunk-148-reel.json");
        let mut c = Cache::default();
        appliquer(&mut c, -7, gog(), &reponse, &r, "100");
        let e = c.jeu(-7).unwrap();
        assert_eq!(e.firehouse, EtatFirehouse::Lu);
        assert_eq!((e.source.as_deref(), e.reconnu_par.as_deref(), e.source_nom.as_deref()), (Some("ludusavi"), Some("gog_id"), Some("Cyberpunk 2077")));
        assert_eq!(e.source_url.as_deref(), Some("https://www.pcgamingwiki.com/wiki/Cyberpunk_2077"));
        // Windows seul : Mac et la ligne Microsoft Store (`boutique: microsoft`) sont écartés pour un jeu GOG.
        assert_eq!(e.emplacements.len(), 1);
        assert_eq!(e.emplacements[0].chemin, r"%USERPROFILE%\Saved Games\CD Projekt Red\Cyberpunk 2077");
        assert_eq!((e.emplacements[0].source.as_deref(), e.emplacements[0].etiquettes.clone(), e.emplacements[0].boutique.clone()), (Some("ludusavi"), vec!["save".to_string()], None));
        // Le même jeu chez Xbox : la ligne Microsoft Store (`microsoft` = notre `xboxone`) en plus, jamais Mac.
        let mut x = Cache::default();
        appliquer(&mut x, -8, Some(BoutiqueJeu { source: "xboxone".into(), cle: "CDPROJEKTSA.Cyberpunk2077_0xkrdv0gjmk28".into() }), &reponse, &r, "100");
        let l = &x.jeu(-8).unwrap().emplacements;
        assert_eq!(l.iter().map(|e| e.os.as_str()).collect::<Vec<_>>(), ["Windows", "Microsoft Store"]);
        assert_eq!(l[1].boutique.as_deref(), Some("microsoft"));
    }

    #[test]
    fn terrascape_epic_en_148_n_est_pas_devine() {
        // Relevé réel : tous ses emplacements en « Steam » (`boutique: steam`), certains chemins de Mac ou de Linux.
        // Pour un jeu Epic, rien ne vaut : INCONNU sur ce PC, « connu seulement pour Steam », rien de gardé.
        let mut c = Cache::default();
        appliquer(&mut c, -9, epic(), &fixture("triche-terrascape-epic-148-reel.json"), &refs_148(), "100");
        let e = c.jeu(-9).unwrap();
        assert_eq!(e.firehouse, EtatFirehouse::Inconnu, "jamais « aucun emplacement »");
        assert!(e.emplacements.is_empty(), "rien de deviné");
        assert_eq!(e.autres_systemes, ["Mac ou Linux", "Steam"], "les chemins de Mac ou Linux servis en « Steam » sont dits à part");
        assert_eq!(e.reconnu_par.as_deref(), Some("dossier"));
        // Inconnu : redemandé dès le lendemain (Firehouse corrige en 1.49).
        assert!(a_redemander(Some(e), 100 + DELAI_INCONNU));
    }

    #[test]
    fn terrascape_epic_en_149_garde_les_chemins_windows_sans_filtrer_sur_releve_sur() {
        // SIMULÉ (1.49) : les chemins Windows communs en « Windows », `releve_sur: steam`, `boutique: null`.
        let mut c = Cache::default();
        appliquer(&mut c, -9, epic(), &fixture("simule/triche-terrascape-epic-149-simule.json"), &refs_148(), "100");
        let e = c.jeu(-9).unwrap();
        assert_eq!(e.firehouse, EtatFirehouse::Lu);
        assert_eq!(
            e.emplacements.iter().map(|x| x.chemin.as_str()).collect::<Vec<_>>(),
            [r"%USERPROFILE%\AppData\LocalLow\Bitfall Studios\TerraScape\*.save", r"%USERPROFILE%\AppData\LocalLow\Bitfall Studios\TerraScape\saves\*.save", r"%USERPROFILE%\AppData\LocalLow\Bitfall Studios\TerraScape\savesMP\*.save"]
        );
        assert!(e.emplacements.iter().all(|x| x.releve_sur.as_deref() == Some("steam") && x.boutique.is_none() && x.nature == "motif"), "rangé pour information");
        assert_eq!(e.autres_systemes, ["Linux", "Mac"]);
    }

    #[test]
    fn indisponible_registre_et_os_inconnu_ne_font_rien_deviner() {
        let r = refs_148();
        let reponse = fixture("triche-cyberpunk-148-reel.json");
        // `indisponible: true` (forme du relevé, valeur changée) : INCONNU, jamais « rien à sauvegarder ».
        let mut indispo = reponse.clone();
        indispo["emplacements_source"] = json!({"source": null, "reconnu_par": null, "source_url": null, "lu_le": null, "nom": null, "indisponible": true});
        indispo["sauvegardes"] = json!([]);
        let mut c = Cache::default();
        appliquer(&mut c, -7, gog(), &indispo, &r, "10");
        assert_eq!(c.jeu(-7).unwrap().firehouse, EtatFirehouse::Inconnu, "jamais « vide »");
        appliquer(&mut c, -7, gog(), &reponse, &r, "20");
        appliquer(&mut c, -7, gog(), &indispo, &r, "30");
        let e = c.jeu(-7).unwrap();
        assert_eq!((e.firehouse, e.emplacements.len(), e.lu_le.as_deref(), e.demande_le.as_deref()), (EtatFirehouse::Lu, 1, Some("20"), Some("30")), "un cache rempli reste");
        // Le registre (`nature: registre`) et un `os` inconnu : ignorés, jamais un fichier.
        let mut autres = reponse.clone();
        autres["sauvegardes"] = json!([
            {"os": "Windows", "chemin": r"HKEY_CURRENT_USER\Software\CD Projekt Red", "nature": "registre", "boutique": null, "etiquettes": ["save"]},
            {"os": "Windows Store", "os_inconnu": true, "chemin": r"%LOCALAPPDATA%\Ancien", "nature": "dossier", "boutique": null, "etiquettes": ["save"]}
        ]);
        let mut d = Cache::default();
        appliquer(&mut d, 5, None, &autres, &r, "1");
        assert!(d.jeu(5).unwrap().emplacements.is_empty());
        assert_eq!(d.jeu(5).unwrap().firehouse, EtatFirehouse::Vide, "rien d'autre de connu");
    }

    #[test]
    fn le_moteur_est_range_tel_quel_pour_le_lot_s6() {
        // Forme du contrat 1.48 (le relevé réel n'en a pas : `moteur` et `emplacements_moteur` nuls).
        let mut reponse = fixture("triche-cyberpunk-148-reel.json");
        reponse["sauvegardes"] = json!([]);
        reponse["moteur"] = json!("renpy");
        reponse["emplacements_moteur"] = json!({"moteur": "renpy", "moteur_nom": "Ren'Py", "sauvegardes": [r"%APPDATA%\RenPy\<save_directory Ren'Py>"], "nom_a_lire": "config.save_directory", "source_url": "https://www.renpy.org/doc/html/config.html", "certitude": "officielle", "note": null});
        let mut c = Cache::default();
        appliquer(&mut c, 4242, None, &reponse, &refs_148(), "10");
        let e = c.jeu(4242).unwrap();
        assert_eq!(e.moteur.as_ref().map(|m| m.nom.as_str()), Some("Ren'Py"));
        assert!(e.moteur.as_ref().unwrap().preuve.contains("renpy.org"));
        assert_eq!(e.moteur_firehouse.as_ref().unwrap()["certitude"], "officielle");
        assert_eq!(e.firehouse, EtatFirehouse::Inconnu, "des règles de moteur pas encore lues : pas « aucun emplacement »");
        // Le relevé réel : pas de moteur.
        let mut d = Cache::default();
        appliquer(&mut d, 1, gog(), &fixture("triche-cyberpunk-148-reel.json"), &refs_148(), "10");
        assert_eq!((d.jeu(1).unwrap().moteur.clone(), d.jeu(1).unwrap().moteur_firehouse.clone()), (None, None));
    }

    #[test]
    fn chaque_jeton_est_resolu_ou_refuse_jamais_devine() {
        let d = tempfile::tempdir().unwrap();
        let jeu = d.path().join("Epic Games").join("TerraScape");
        std::fs::create_dir_all(&jeu).unwrap();
        let ctx = Contexte { jeu: Some(jeu.clone()), steam: None, compte: None, windows: vec![], installation: None, boutique: Some("epic".into()) };
        let un = |chemin: &str| {
            let e = PartiesJeu { id: 1, emplacements: vec![EmplacementPartie { chemin: chemin.into(), nature: "dossier".into(), os: "Windows".into(), origine: "firehouse".into(), ..Default::default() }], ..Default::default() };
            resoudre(&e, &ctx, true).remove(0)
        };
        // Résolus quand le dossier du jeu est connu (`<racine de la boutique>` : un jeu de boutique autre que Steam).
        assert_eq!(un(r"<racine de la boutique>\<nom du dossier du jeu>\Saves").dossier.as_deref(), Some(jeu.join("Saves").to_string_lossy().as_ref()));
        assert_eq!(un(r"<dossier du jeu>\Saves").dossier.as_deref(), Some(jeu.join("Saves").to_string_lossy().as_ref()));
        // Le même jeton deux fois : chaque occurrence est remplacée.
        assert_eq!(un(r"<dossier du jeu>\<nom du dossier du jeu>\<nom du dossier du jeu>").dossier.as_deref(), Some(jeu.join("TerraScape").join("TerraScape").to_string_lossy().as_ref()));
        // `<racine de la boutique>` : refusé pour un jeu de Firehouse, importé, ou Steam (Firehouse écrit `<Steam>`).
        for b in [None, Some("steam")] {
            let c = Contexte { jeu: Some(jeu.clone()), steam: None, compte: None, windows: vec![], installation: None, boutique: b.map(String::from) };
            let e = PartiesJeu { id: 1, emplacements: vec![EmplacementPartie { chemin: r"<racine de la boutique>\TerraScape\Saves".into(), nature: "dossier".into(), os: "Windows".into(), origine: "firehouse".into(), ..Default::default() }], ..Default::default() };
            assert!(!resoudre(&e, &c, true)[0].lisible, "{b:?}");
        }
        // Un « % » dans le nom du dossier du jeu ne fait pas refuser l'emplacement.
        let pourcent = d.path().join("Epic Games").join("Jeu 100%");
        let c = Contexte { jeu: Some(pourcent.clone()), steam: None, compte: None, windows: vec![], installation: None, boutique: Some("epic".into()) };
        let e = PartiesJeu { id: 1, emplacements: vec![EmplacementPartie { chemin: r"<dossier du jeu>\Saves".into(), nature: "dossier".into(), os: "Windows".into(), origine: "firehouse".into(), ..Default::default() }], ..Default::default() };
        assert_eq!(resoudre(&e, &c, true)[0].dossier.as_deref(), Some(pourcent.join("Saves").to_string_lossy().as_ref()));
        // Les jetons de moteur, d'une boutique non lue, ou d'une bibliothèque Steam sans jeu Steam : refusés. Lot S6 : un
        // jeton de moteur n'est JAMAIS résolu tel quel ; `moteurs_parties` le remplace par le nom LU avant de ranger.
        for j in [r"%APPDATA%\RenPy\<save_directory Ren'Py>", r"%USERPROFILE%\AppData\LocalLow\<société Unity>\<produit Unity>", r"%LOCALAPPDATA%\<projet Unreal>\Saved\SaveGames", r"%APPDATA%\Godot\app_userdata\<projet Godot>", r"%LOCALAPPDATA%\<nom du jeu GameMaker>", r"<Ubisoft Connect>\savegames", r"<SteamLibrary>\steamapps\common\X", r"<jeton jamais vu>\x"] {
            let v = un(j);
            assert_eq!((v.dossier.clone(), v.lisible), (None, false), "{j}");
        }
        // Une bibliothèque Steam : seulement pour un jeu rangé dans `steamapps\common`.
        let lib = d.path().join("SteamLibrary");
        let s = Contexte { jeu: Some(lib.join("steamapps").join("common").join("X")), steam: None, compte: None, windows: vec![], installation: None, boutique: None };
        let e = PartiesJeu { id: 1, emplacements: vec![EmplacementPartie { chemin: r"<SteamLibrary>\steamapps\common\X\saves".into(), nature: "dossier".into(), os: "Steam".into(), origine: "firehouse".into(), ..Default::default() }], ..Default::default() };
        assert_eq!(resoudre(&e, &s, true)[0].dossier.as_deref(), Some(lib.join("steamapps").join("common").join("X").join("saves").to_string_lossy().as_ref()));
        // Les variables de Windows servies (1.48) : celles de ce PC, ou refusées si Windows ne les donne pas.
        for (jeton, var) in [("%WINDIR%", "SystemRoot"), ("%PROGRAMFILES%", "ProgramFiles"), ("%TEMP%", "TEMP"), ("%PUBLIC%", "PUBLIC"), ("%PROGRAMDATA%", "ProgramData"), ("%ALLUSERSPROFILE%", "ALLUSERSPROFILE")] {
            let v = un(&format!(r"{jeton}\Jeu\Parties"));
            match std::env::var(var).ok().filter(|x| !x.is_empty()) {
                Some(x) => assert_eq!(v.dossier.as_deref(), Some(std::path::Path::new(&x).join("Jeu").join("Parties").to_string_lossy().as_ref()), "{jeton}"),
                None => assert_eq!(v.dossier, None, "{jeton}"),
            }
        }
        // Sans dossier du jeu connu : refusés.
        let sans = Contexte { jeu: None, steam: None, compte: None, windows: vec![], installation: None, boutique: None };
        let e = PartiesJeu { id: 1, emplacements: vec![EmplacementPartie { chemin: r"<racine de la boutique>\<nom du dossier du jeu>".into(), nature: "dossier".into(), os: "Windows".into(), origine: "firehouse".into(), ..Default::default() }], ..Default::default() };
        assert!(!resoudre(&e, &sans, true)[0].lisible);
    }

    #[test]
    fn les_references_se_gardent_en_commun() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(references_gardees(d.path()), None);
        garder_references(d.path(), &fixture("sauvegardes-references-148-reel.json"), "500").unwrap();
        let (le, r) = references_gardees(d.path()).unwrap();
        assert_eq!((le, r.os.len()), (500, 11));
        assert!(!d.path().join("sauvegardes-references.json.provisoire").exists());
    }

    /// Revue des experts (22/11), correctif 6 : TerraScape réel 1.48 vu par un jeu STEAM — les chemins de Linux
    /// (`%USERPROFILE%\.config\…`) et de Mac (`%USERPROFILE%\Library\…`) servis en « Steam » ne sont jamais gardés.
    #[test]
    fn terrascape_vu_par_un_jeu_steam_ne_garde_que_les_chemins_windows() {
        let mut c = Cache::default();
        let steam = Some(BoutiqueJeu { source: "steam".into(), cle: "1234".into() });
        appliquer(&mut c, -3, steam, &fixture("triche-terrascape-epic-148-reel.json"), &refs_148(), "1");
        let e = c.jeu(-3).unwrap();
        assert_eq!(e.firehouse, EtatFirehouse::Lu);
        assert_eq!(e.emplacements.len(), 3, "les 3 dossiers LocalLow, pas les 6 fantômes");
        assert!(e.emplacements.iter().all(|x| x.chemin.contains(r"\AppData\LocalLow\")));
        assert_eq!(e.autres_systemes, ["Mac ou Linux"]);
        assert!(chemin_de_mac_ou_linux(r"%USERPROFILE%\.config\x") && chemin_de_mac_ou_linux(r"%userprofile%\Library\x") && chemin_de_mac_ou_linux(r"%USERPROFILE%\.local\share"));
        assert!(!chemin_de_mac_ou_linux(r"%USERPROFILE%\Documents\Library") && !chemin_de_mac_ou_linux(r"%APPDATA%\.config"));
    }

    /// Correctif 6 : un cache de la forme 1 se LIT (rien n'est perdu), ses jeux sont redemandés une fois, et il n'est
    /// réécrit qu'en forme 2.
    #[test]
    fn un_cache_de_la_forme_1_se_relit_et_ses_jeux_sont_redemandes() {
        let d = tempfile::tempdir().unwrap();
        let v1 = json!({"version": 1, "jeux": {"-7": {"id": -7, "firehouse": "lu", "lu_le": "20", "demande_le": "20", "emplacements": [
            {"chemin": r"%USERPROFILE%\Saved Games\CD Projekt Red\Cyberpunk 2077", "nature": "dossier", "os": "Windows", "origine": "firehouse", "nuage_boutique": false, "lu_le": "20"}]}}});
        std::fs::write(fichier(d.path()), serde_json::to_vec(&v1).unwrap()).unwrap();
        let c = lire(d.path()).unwrap();
        assert_eq!(c.version, VERSION);
        let e = c.jeu(-7).unwrap();
        assert_eq!((e.firehouse, e.emplacements.len(), e.demande_le.as_deref()), (EtatFirehouse::Lu, 1, None));
        assert!(a_redemander(Some(e), 21), "relu à la prochaine ouverture de sa fiche");
        modifier(d.path(), |c| appliquer(c, -7, gog(), &json!({"sauvegardes": null}), &refs_148(), "30")).unwrap();
        let relu: Value = serde_json::from_slice(&std::fs::read(fichier(d.path())).unwrap()).unwrap();
        assert_eq!(relu["version"], VERSION);
        assert_eq!(relu["jeux"]["-7"]["emplacements"].as_array().unwrap().len(), 1, "un null n'efface rien");
    }

    /// 0.220.1 : un cache de la forme 2 (avant le contrat 1.50 : « Users\(UserID)\Savegames » d'Age of Empires IV) se
    /// lit en entier, ses jeux sont redemandés une fois, et il n'est réécrit qu'en forme 3.
    #[test]
    fn un_cache_de_la_forme_2_est_relu_apres_le_contrat_1_50() {
        assert_eq!(VERSION, 3);
        let d = tempfile::tempdir().unwrap();
        let v2 = json!({"version": 2, "jeux": {"-5": {"id": -5, "firehouse": "lu", "lu_le": "20", "demande_le": "20", "emplacements": [
            {"chemin": r"%USERPROFILE%\Documents\My Games\Age of Empires IV\Users\(UserID)\Savegames", "nature": "dossier", "os": "Windows", "origine": "firehouse", "nuage_boutique": false, "lu_le": "20"}]}}});
        std::fs::write(fichier(d.path()), serde_json::to_vec(&v2).unwrap()).unwrap();
        let c = lire(d.path()).unwrap();
        let e = c.jeu(-5).unwrap();
        assert_eq!((e.emplacements.len(), e.demande_le.as_deref()), (1, None), "lu en entier, redemandé");
        assert!(a_redemander(Some(e), 21));
        // Jamais écrasé sans être lu : le fichier reste tel quel tant que rien ne change.
        modifier(d.path(), |_| ()).unwrap();
        let brut: Value = serde_json::from_slice(&std::fs::read(fichier(d.path())).unwrap()).unwrap();
        assert_eq!(brut, v2);
        // La réponse 1.50 remplace l'emplacement d'origine Firehouse ; le fichier passe en forme 3.
        let r = json!({"emplacements_source": {"source": "ludusavi", "reconnu_par": "steam_appid", "indisponible": false},
            "sauvegardes": [{"os": "Windows", "chemin": r"%USERPROFILE%\Documents\My Games\Age of Empires IV\Users\<identifiant Steam>\Savegames", "nature": "dossier", "boutique": null}]});
        modifier(d.path(), |c| appliquer(c, -5, None, &r, &refs_148(), "30")).unwrap();
        let relu: Value = serde_json::from_slice(&std::fs::read(fichier(d.path())).unwrap()).unwrap();
        assert_eq!(relu["version"], 3);
        assert!(relu["jeux"]["-5"]["emplacements"][0]["chemin"].as_str().unwrap().contains("<identifiant Steam>"));
    }

    /// Correctif 10 : un jeu que le profil ne voit plus perd ce qui venait de Firehouse, jamais les choix de la personne.
    #[test]
    fn un_jeu_invisible_garde_les_choix_de_la_personne() {
        let mut c = Cache::default();
        appliquer(&mut c, 1, gog(), &cyberpunk(), &secours(), "1");
        c.jeux.get_mut("1").unwrap().emplacements.push(EmplacementPartie { chemin: r"D:\Mes parties".into(), nature: "dossier".into(), os: "Windows".into(), origine: "personne".into(), ..Default::default() });
        appliquer(&mut c, 2, None, &cyberpunk(), &secours(), "1");
        assert_eq!(retirer_si(&mut c, |_| false), 2);
        assert!(c.jeu(2).is_none(), "rien d'autre : l'entrée part");
        let e = c.jeu(1).unwrap();
        assert_eq!(e.emplacements.iter().map(|x| x.origine.as_str()).collect::<Vec<_>>(), ["personne"]);
        assert_eq!((e.firehouse, e.source.clone(), e.lu_le.clone(), e.reconnu_par.clone()), (EtatFirehouse::Inconnu, None, None, None), "plus rien de Firehouse");
    }

    /// Correctif 5 (et Firehouse 22/11) : un nom de dossier trop courant ne sert jamais à reconnaître un jeu.
    #[test]
    fn un_nom_de_dossier_trop_courant_n_est_jamais_envoye() {
        let r = refs_148();
        for n in ["Content", "Game", "Games", "Binaries", "Win64", "Win32", "bin", "x64", "x86", "Jeux", "Data", "App", "Build", "Release", "common", "Windows", "Launcher", "Program Files", "steamapps", "PC", "abc"] {
            assert!(!dossier_reconnaissable(n, &r, &[]), "{n}");
        }
        assert!(dossier_reconnaissable("Cyberpunk 2077", &r, &[]));
        assert!(dossier_reconnaissable("Hades", &r, &[]), "4 caractères : permis");
        assert!(!dossier_reconnaissable("Mes jeux PC", &r, &["Mes jeux PC".into()]), "le dossier des jeux réglé dans Frogtend");
        // 1.49 : la liste de Firehouse s'ajoute à la nôtre.
        let r149 = lire_references(&json!({"os": {"Windows": ""}, "dossiers_ignores": ["Retail", "shipping"]})).unwrap();
        assert!(!dossier_reconnaissable("retail", &r149, &[]) && !dossier_reconnaissable("Shipping", &r149, &[]));
        assert!(dossier_reconnaissable("Retail", &r, &[]));
    }

    /// Correctif 3 : une racine n'est jamais un emplacement de parties ENTIER ; un chemin qui descend plus bas, si.
    #[test]
    fn une_racine_n_est_jamais_un_emplacement_entier() {
        let d = tempfile::tempdir().unwrap();
        let lib = d.path().join("SteamLibrary");
        let jeu = lib.join("steamapps").join("common").join("Jeu");
        std::fs::create_dir_all(jeu.join("saves")).unwrap();
        let steam = Contexte { jeu: Some(jeu.clone()), steam: None, compte: None, windows: vec![], installation: None, boutique: Some("steam".into()) };
        let epic_jeu = d.path().join("Epic Games").join("Jeu");
        std::fs::create_dir_all(epic_jeu.join("saves")).unwrap();
        let epic = Contexte { jeu: Some(epic_jeu.clone()), steam: None, compte: None, windows: vec![], installation: None, boutique: Some("epic".into()) };
        let un = |chemin: &str, ctx: &Contexte| {
            let e = PartiesJeu { id: 1, emplacements: vec![EmplacementPartie { chemin: chemin.into(), nature: "dossier".into(), os: "Windows".into(), origine: "firehouse".into(), ..Default::default() }], ..Default::default() };
            resoudre(&e, ctx, false).remove(0)
        };
        let refuse = |chemin: &str, ctx: &Contexte| {
            let v = un(chemin, ctx);
            assert_eq!((v.dossier.clone(), v.lisible), (None, false), "{chemin}");
        };
        // Le dossier du jeu, celui qui le contient, la bibliothèque Steam, `steamapps`, `steamapps\common`.
        refuse(r"<dossier du jeu>", &steam);
        refuse(r"<dossier du jeu>\..", &steam);
        refuse(&lib.to_string_lossy(), &steam);
        refuse(&lib.join("steamapps").to_string_lossy(), &steam);
        refuse(&lib.join("steamapps").join("common").to_string_lossy(), &steam);
        refuse(r"<SteamLibrary>", &steam);
        // `<racine de la boutique>` seule : elle contient d'autres jeux.
        refuse(r"<racine de la boutique>", &epic);
        refuse(&epic_jeu.parent().unwrap().to_string_lossy(), &epic);
        // %TEMP%, AppData et ses trois dossiers, le profil, un disque entier.
        for j in [r"%TEMP%", r"%USERPROFILE%\AppData", r"%USERPROFILE%\AppData\Local", r"%USERPROFILE%\AppData\Roaming", r"%LOCALAPPDATA%", r"%APPDATA%", r"%USERPROFILE%", "D:\\"] {
            refuse(j, &epic);
        }
        if std::env::var_os("USERPROFILE").is_some() {
            let low = std::path::Path::new(&std::env::var_os("USERPROFILE").unwrap()).join("AppData").join("LocalLow");
            refuse(&low.to_string_lossy(), &epic);
        }
        // Plus bas : accepté.
        assert_eq!(un(r"<racine de la boutique>\Jeu\saves", &epic).dossier.as_deref(), Some(epic_jeu.join("saves").to_string_lossy().as_ref()));
        assert_eq!(un(r"<SteamLibrary>\steamapps\common\Jeu\saves", &steam).dossier.as_deref(), Some(jeu.join("saves").to_string_lossy().as_ref()));
        // `compter` faux (« Ouvrir le dossier ») : il existe, rien n'est compté.
        let v = un(r"<dossier du jeu>\saves", &epic);
        assert_eq!((v.existe, v.fichiers, v.trop, v.illisible), (true, 0, false, false));
    }

    /// Correctif 4 : « trop » et « illisible » ne se confondent plus.
    #[test]
    fn trop_et_illisible_se_distinguent() {
        let d = tempfile::tempdir().unwrap();
        let mut profond = d.path().join("Profond");
        for i in 0..14 {
            profond = profond.join(format!("n{i}"));
        }
        std::fs::create_dir_all(&profond).unwrap();
        let ctx = Contexte { jeu: Some(d.path().to_path_buf()), steam: None, compte: None, windows: vec![], installation: None, boutique: None };
        let e = PartiesJeu { id: 1, emplacements: vec![EmplacementPartie { chemin: r"<dossier du jeu>\Profond".into(), nature: "dossier".into(), os: "Windows".into(), origine: "firehouse".into(), ..Default::default() }], ..Default::default() };
        let v = resoudre(&e, &ctx, true).remove(0);
        assert_eq!((v.existe, v.trop, v.illisible), (true, true, false), "trop profond : pas un dossier de parties");
        assert_eq!(crate::nuage_steam::parcourir(&d.path().join("Absent")), Err(crate::nuage_steam::Arret::Illisible));
    }

    /// Contrat 1.49 : `dossier_ecarte` est rangé pour information.
    #[test]
    fn le_dossier_ecarte_par_firehouse_est_range() {
        let mut r = fixture("triche-cyberpunk-148-reel.json");
        r["emplacements_source"]["dossier_ecarte"] = json!({"dossier": "Game", "raison": "aucun mot commun avec le titre"});
        let mut c = Cache::default();
        appliquer(&mut c, 1, gog(), &r, &refs_148(), "1");
        assert_eq!(c.jeu(1).unwrap().dossier_ecarte.as_ref().unwrap()["dossier"], "Game");
        appliquer(&mut c, 2, gog(), &fixture("triche-cyberpunk-148-reel.json"), &refs_148(), "1");
        assert_eq!(c.jeu(2).unwrap().dossier_ecarte, None);
    }

    /// Seb 22/11 (Age of Empires IV sous OneDrive) : « 📁 Où sont mes parties » compte les fichiers « en ligne
    /// seulement » à part, sans jamais les lire.
    #[test]
    fn les_fichiers_en_ligne_seulement_sont_comptes_a_part() {
        let d = tempfile::tempdir().unwrap();
        let docs = d.path().join("OneDrive").join("Documents");
        let saves = docs.join("My Games").join("Jeu");
        std::fs::create_dir_all(&saves).unwrap();
        std::fs::write(saves.join("a.sav"), b"aaaa").unwrap();
        std::fs::write(saves.join("b.sav"), b"bb").unwrap();
        let ctx = Contexte { jeu: None, steam: None, compte: None, windows: vec![("Documents", docs.clone())], installation: None, boutique: None };
        let e = PartiesJeu { id: 1, emplacements: vec![EmplacementPartie { chemin: r"%USERPROFILE%\Documents\My Games\Jeu".into(), nature: "dossier".into(), os: "Windows".into(), origine: "firehouse".into(), ..Default::default() }], ..Default::default() };
        assert_eq!(resoudre(&e, &ctx, true)[0].en_ligne, 0);
        #[cfg(windows)]
        {
            use windows::Win32::Storage::FileSystem::{SetFileAttributesW, FILE_ATTRIBUTE_ARCHIVE, FILE_ATTRIBUTE_OFFLINE};
            let f = windows::core::HSTRING::from(saves.join("a.sav").as_os_str());
            if unsafe { SetFileAttributesW(&f, FILE_ATTRIBUTE_OFFLINE | FILE_ATTRIBUTE_ARCHIVE) }.is_err() {
                eprintln!("attribut « hors ligne » impossible à poser ici : test sauté");
                return;
            }
            let v = resoudre(&e, &ctx, true).remove(0);
            assert_eq!((v.fichiers, v.octets, v.en_ligne), (2, 6, 1));
            let _ = unsafe { SetFileAttributesW(&f, FILE_ATTRIBUTE_ARCHIVE) };
        }
    }

    /// Lot S6 : un constat de moteur prouvé (Unity), tel que `moteurs_parties::reconnaitre` le rend.
    fn constat_unity() -> crate::moteurs_parties::Reconnaissance {
        let ligne = |chemin: &str| EmplacementPartie { chemin: chemin.into(), nature: "dossier".into(), os: "Windows".into(), origine: "moteur".into(), moteur: Some("Unity".into()), preuve: Some("UnityPlayer.dll + app.info".into()), ..Default::default() };
        crate::moteurs_parties::Reconnaissance::Prouve(crate::moteurs_parties::Constat {
            cle: "unity".into(),
            nom: "Unity".into(),
            marqueur: "UnityPlayer.dll".into(),
            lus_dans: vec!["app.info".into()],
            noms_lus: vec!["Studio".into(), "Jeu".into()],
            emplacements: vec![ligne(r"%USERPROFILE%\AppData\LocalLow\Studio\Jeu"), ligne(r"<dossier du jeu>\Saves")],
            nom_lu: Some("Studio/Jeu".into()),
            ..Default::default()
        })
    }

    fn origines(c: &Cache, id: i64) -> Vec<(String, String)> {
        c.jeu(id).map(|e| e.emplacements.iter().map(|x| (x.origine.clone(), x.chemin.clone())).collect()).unwrap_or_default()
    }

    /// Lot S6 : `personne` > Firehouse `Lu` > moteur prouvé ; `Inconnu` : le moteur s'applique ; `Vide` : seulement
    /// si des parties existent (désaccord noté) ; les doublons partent ; rien de prouvé : plus aucune ligne `moteur`.
    #[test]
    fn le_moteur_prouve_s_applique_selon_ce_que_dit_firehouse() {
        use crate::moteurs_parties::Reconnaissance;
        // Inconnu (jamais demandé) : appliqué d'office, l'entrée est créée.
        let mut c = Cache::default();
        appliquer_moteur(&mut c, 1, &Reconnaissance::Aucun, true);
        assert!(c.jeu(1).is_none(), "rien de prouvé : aucune entrée créée");
        appliquer_moteur(&mut c, 1, &constat_unity(), false);
        let e = c.jeu(1).unwrap();
        assert_eq!(e.emplacements.len(), 2);
        assert_eq!(e.moteur_reconnu.as_ref().map(|m| (m.cle.as_str(), m.preuve.as_str())), Some(("unity", "UnityPlayer.dll + app.info")));
        assert!(!e.desaccord_moteur);
        // Rejoué : jamais en double.
        appliquer_moteur(&mut c, 1, &constat_unity(), false);
        assert_eq!(c.jeu(1).unwrap().emplacements.len(), 2);
        // Firehouse répond ensuite (`Lu`) avec le même endroit (autre casse) : sa ligne reste, celle du moteur part.
        let r = json!({"sauvegardes": [{"os": "Windows", "chemin": r"%USERPROFILE%\AppData\LocalLow\studio\jeu", "nature": "dossier"}]});
        appliquer(&mut c, 1, None, &r, &secours(), "10");
        assert_eq!(origines(&c, 1), [("moteur".to_string(), r"<dossier du jeu>\Saves".to_string()), ("firehouse".to_string(), r"%USERPROFILE%\AppData\LocalLow\studio\jeu".to_string())]);
        appliquer_moteur(&mut c, 1, &constat_unity(), false);
        assert_eq!(origines(&c, 1), [("firehouse".to_string(), r"%USERPROFILE%\AppData\LocalLow\studio\jeu".to_string()), ("moteur".to_string(), r"<dossier du jeu>\Saves".to_string())], "Firehouse l'emporte, l'autre endroit du moteur s'ajoute");
        // Une ligne de la personne : elle l'emporte sur tout.
        c.jeux.get_mut("1").unwrap().emplacements.insert(0, EmplacementPartie { chemin: r"<dossier du jeu>\SAVES".into(), nature: "dossier".into(), os: "Windows".into(), origine: "personne".into(), ..Default::default() });
        appliquer_moteur(&mut c, 1, &constat_unity(), false);
        assert_eq!(origines(&c, 1).iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), ["personne", "firehouse"]);
        // `Vide` : sans partie sur le disque, Firehouse a raison ; avec, le moteur l'emporte et le désaccord est noté.
        let mut v = Cache::default();
        appliquer(&mut v, 2, None, &json!({"sauvegardes": []}), &secours(), "10");
        appliquer_moteur(&mut v, 2, &constat_unity(), false);
        let e = v.jeu(2).unwrap();
        assert!(e.emplacements.is_empty() && !e.desaccord_moteur && e.moteur_reconnu.is_some());
        appliquer_moteur(&mut v, 2, &constat_unity(), true);
        let e = v.jeu(2).unwrap();
        assert_eq!((e.emplacements.len(), e.desaccord_moteur, e.firehouse), (2, true, EtatFirehouse::Vide));
        // Le moteur n'est plus prouvé (le jeu a changé) : ses lignes partent, le désaccord aussi.
        appliquer_moteur(&mut v, 2, &Reconnaissance::Douteux, true);
        let e = v.jeu(2).unwrap();
        assert!(e.emplacements.is_empty() && !e.desaccord_moteur && e.moteur_reconnu.is_none());
        // Relu tel qu'écrit (les champs nouveaux se gardent).
        let d = tempfile::tempdir().unwrap();
        modifier(d.path(), |x| appliquer_moteur(x, 3, &constat_unity(), false)).unwrap();
        let relu = lire(d.path()).unwrap();
        assert_eq!(relu.jeu(3).unwrap().emplacements[0].moteur.as_deref(), Some("Unity"));
        assert_eq!(relu.jeu(3).unwrap().emplacements[0].preuve.as_deref(), Some("UnityPlayer.dll + app.info"));
    }

    /// Lot S6 (correction de `retirer_si`) : un jeu que le profil ne voit plus perd TOUTES ses lignes — Firehouse,
    /// moteur, manifeste — et son moteur reconnu ; seuls les choix de la personne restent.
    #[test]
    fn un_jeu_invisible_perd_ses_lignes_de_moteur() {
        let mut c = Cache::default();
        appliquer(&mut c, 1, gog(), &cyberpunk(), &secours(), "1");
        appliquer_moteur(&mut c, 1, &constat_unity(), false);
        c.jeux.get_mut("1").unwrap().emplacements.push(EmplacementPartie { chemin: r"%APPDATA%\Jeu".into(), nature: "dossier".into(), os: "Windows".into(), origine: "manifeste".into(), ..Default::default() });
        appliquer_moteur(&mut c, 2, &constat_unity(), false);
        appliquer(&mut c, 3, None, &cyberpunk(), &secours(), "1");
        appliquer_moteur(&mut c, 3, &constat_unity(), false);
        c.jeux.get_mut("3").unwrap().emplacements.push(EmplacementPartie { chemin: r"D:\Mes parties".into(), nature: "dossier".into(), os: "Windows".into(), origine: "personne".into(), ..Default::default() });
        assert_eq!(retirer_si(&mut c, |_| false), 3);
        assert!(c.jeu(1).is_none(), "Firehouse, moteur et manifeste : l'entrée part");
        assert!(c.jeu(2).is_none(), "seulement le moteur : l'entrée part");
        let e = c.jeu(3).unwrap();
        assert_eq!(origines(&c, 3), [("personne".to_string(), r"D:\Mes parties".to_string())]);
        assert_eq!((e.moteur_reconnu.clone(), e.moteur.clone(), e.desaccord_moteur, e.moteur_firehouse.clone()), (None, None, false, None));
    }

    #[test]
    fn les_doublons_gardent_la_meilleure_origine() {
        let l = |o: &str, c: &str| EmplacementPartie { chemin: c.into(), nature: "dossier".into(), origine: o.into(), ..Default::default() };
        let mut e = PartiesJeu { id: 1, emplacements: vec![l("moteur", r"<dossier du jeu>\Save"), l("firehouse", "<dossier du jeu>/save/"), l("personne", r"<DOSSIER DU JEU>\SAVE"), l("moteur", r"%APPDATA%\X"), l("moteur", r"%appdata%\x")], ..Default::default() };
        retirer_doublons(&mut e);
        assert_eq!(e.emplacements.iter().map(|x| (x.origine.as_str(), x.chemin.as_str())).collect::<Vec<_>>(), [("personne", r"<DOSSIER DU JEU>\SAVE"), ("moteur", r"%APPDATA%\X")]);
    }

    /// Relecture S6 : « des parties existent » ne compte jamais les journaux du jeu ; un désaccord sur le moteur nommé
    /// par Firehouse est noté ; deux natures du même chemin ne sont pas des doublons.
    #[test]
    fn les_journaux_ne_sont_pas_des_parties_et_le_moteur_contredit_est_note() {
        let d = tempfile::tempdir().unwrap();
        let low = d.path().join("LocalLow");
        let jeu = low.join("Studio").join("Jeu");
        std::fs::create_dir_all(jeu.join("Unity")).unwrap();
        for f in ["Player.log", "Player-prev.log", "output_log.txt", "Unity/x.log"] {
            std::fs::write(jeu.join(f), b"journal").unwrap();
        }
        let ctx = Contexte { jeu: None, steam: None, compte: None, windows: vec![("AppData-LocalLow", low.clone())], installation: None, boutique: None };
        let e = PartiesJeu { id: 1, emplacements: vec![EmplacementPartie { chemin: r"%USERPROFILE%\AppData\LocalLow\Studio\Jeu".into(), nature: "dossier".into(), origine: "moteur".into(), ..Default::default() }], ..Default::default() };
        assert!(!contient_des_parties(&e, &ctx), "seulement des journaux");
        std::fs::write(jeu.join("save.dat"), b"partie").unwrap();
        assert!(contient_des_parties(&e, &ctx));
        let m = PartiesJeu { id: 1, emplacements: vec![EmplacementPartie { chemin: r"%USERPROFILE%\AppData\LocalLow\Studio\Jeu\*.log".into(), nature: "motif".into(), origine: "moteur".into(), ..Default::default() }], ..Default::default() };
        assert!(!contient_des_parties(&m, &ctx));
        // Le moteur contredit.
        let mut c = Cache::default();
        let mut r = constat_unity();
        if let crate::moteurs_parties::Reconnaissance::Prouve(x) = &mut r {
            x.firehouse_contredit = true;
        }
        appliquer_moteur(&mut c, 1, &r, false);
        assert!(c.jeu(1).unwrap().moteur_contredit);
        appliquer_moteur(&mut c, 1, &constat_unity(), false);
        assert!(!c.jeu(1).unwrap().moteur_contredit);
        // Même chemin, natures différentes : les deux restent.
        let l = |n: &str| EmplacementPartie { chemin: r"<dossier du jeu>\Save".into(), nature: n.into(), origine: "moteur".into(), ..Default::default() };
        let mut e = PartiesJeu { id: 1, emplacements: vec![l("dossier"), l("fichier"), l("dossier")], ..Default::default() };
        retirer_doublons(&mut e);
        assert_eq!(e.emplacements.iter().map(|x| x.nature.as_str()).collect::<Vec<_>>(), ["dossier", "fichier"]);
    }

    /// Relecture S6 (e) : le `<dossier du jeu>` sorti d'une enveloppe d'archive ne change ni `<SteamLibrary>` d'un jeu
    /// Steam, ni `<racine de la boutique>` d'un jeu Epic : ils partent du dossier de l'installation d'ORIGINE.
    #[test]
    fn la_racine_de_la_boutique_part_de_l_installation_d_origine() {
        let d = tempfile::tempdir().unwrap();
        let lib = d.path().join("SteamLibrary");
        let steam = lib.join("steamapps").join("common").join("Jeu");
        std::fs::create_dir_all(steam.join("Jeu").join("saves")).unwrap();
        std::fs::write(steam.join("Jeu").join("Jeu.exe"), b"x").unwrap();
        let jeu = crate::jeux_pc::dossier_des_parties_de(&steam, None);
        assert_eq!(jeu, steam.join("Jeu"), "une enveloppe");
        let un = |chemin: &str, ctx: &Contexte| {
            let e = PartiesJeu { id: 1, emplacements: vec![EmplacementPartie { chemin: chemin.into(), nature: "dossier".into(), os: "Windows".into(), origine: "firehouse".into(), ..Default::default() }], ..Default::default() };
            resoudre(&e, ctx, false).remove(0).dossier
        };
        let s = Contexte { jeu: Some(jeu.clone()), steam: None, compte: None, windows: vec![], installation: Some(steam.clone()), boutique: Some("steam".into()) };
        assert_eq!(un(r"<SteamLibrary>\steamapps\common\Jeu\saves", &s).as_deref(), Some(steam.join("saves").to_string_lossy().as_ref()));
        assert_eq!(un(r"<dossier du jeu>\saves", &s).as_deref(), Some(jeu.join("saves").to_string_lossy().as_ref()));
        assert_eq!(un(r"<racine de la boutique>\Jeu\saves", &s), None, "jamais pour Steam");
        // Le dossier de l'installation d'origine reste une racine interdite.
        assert_eq!(un(&steam.to_string_lossy(), &s), None);
        let epic = d.path().join("Epic Games").join("Jeu");
        let e = Contexte { jeu: Some(epic.join("Jeu")), steam: None, compte: None, windows: vec![], installation: Some(epic.clone()), boutique: Some("epic".into()) };
        assert_eq!(un(r"<racine de la boutique>\Jeu\Saved", &e).as_deref(), Some(epic.join("Saved").to_string_lossy().as_ref()));
    }

    /// Contrat 1.53 : un signalement seulement pour un moteur prouvé ET appliqué, quand Firehouse ne sait pas, disait
    /// « aucune » (le seul `desaccord`) ou nomme un autre moteur ; une fois répondu, jamais renvoyé ; hors ligne, à
    /// refaire.
    #[test]
    fn le_signalement_du_moteur_suit_ce_que_dit_firehouse() {
        let unity = |e: &PartiesJeu| a_signaler(e);
        // Inconnu (jamais demandé) : signalé, sans désaccord.
        let mut c = Cache::default();
        appliquer_moteur(&mut c, 1, &constat_unity(), false);
        let a = unity(c.jeu(1).unwrap()).unwrap();
        assert_eq!(a, ASignaler { cle: "unity".into(), nom_lu: Some("Studio/Jeu".into()), preuve: "UnityPlayer.dll".into(), desaccord: false });
        assert_eq!(a.corps(), json!({"moteur": "unity", "nom_lu": "Studio/Jeu", "preuve": "UnityPlayer.dll"}), "desaccord absent = faux");
        // Hors ligne : à refaire ; puis retenu : jamais renvoyé, et le moteur retenu par Firehouse est rangé.
        noter_signalement(&mut c, 1, &a, EtatSignalement::HorsLigne, "20", None);
        assert_eq!(unity(c.jeu(1).unwrap()), Some(a.clone()));
        let rep = json!({"ok": true, "retenu": true, "moteur": "unity", "emplacements_moteur": {"moteur": "unity", "moteur_nom": "Unity", "sauvegardes": [], "nom_a_lire": "companyName"}});
        noter_signalement(&mut c, 1, &a, EtatSignalement::Retenu, "30", Some(&rep));
        let e = c.jeu(1).unwrap();
        assert_eq!(e.signalements, [SignalementMoteur { cle: "unity".into(), nom_lu: Some("Studio/Jeu".into()), etat: EtatSignalement::Retenu, le: "30".into() }]);
        assert_eq!(e.moteur_firehouse.as_ref().unwrap()["moteur"], "unity");
        assert_eq!(e.moteur.as_ref().map(|m| m.nom.as_str()), Some("Unity"));
        assert!(unity(e).is_none(), "jamais renvoyé");
        // Un AUTRE nom lu (le jeu a changé) : un nouveau signalement.
        let mut x = c.jeu(1).unwrap().clone();
        x.moteur_reconnu.as_mut().unwrap().nom_lu = Some("Studio/Jeu 2".into());
        assert!(unity(&x).is_some());
        // Refusé (400) et introuvable (404) : jamais renvoyés.
        for etat in [EtatSignalement::Refuse, EtatSignalement::Introuvable, EtatSignalement::Note] {
            let mut c = Cache::default();
            appliquer_moteur(&mut c, 1, &constat_unity(), false);
            noter_signalement(&mut c, 1, &a, etat, "40", None);
            assert!(unity(c.jeu(1).unwrap()).is_none(), "{etat:?}");
        }
        // Firehouse `Lu`, même moteur ou aucun nommé : rien à signaler ; un autre moteur nommé : signalé, SANS désaccord.
        let mut l = Cache::default();
        appliquer(&mut l, 2, None, &json!({"sauvegardes": [{"os": "Windows", "chemin": r"%APPDATA%\Jeu", "nature": "dossier"}]}), &secours(), "10");
        appliquer_moteur(&mut l, 2, &constat_unity(), false);
        assert!(unity(l.jeu(2).unwrap()).is_none());
        l.jeux.get_mut("2").unwrap().moteur_contredit = true;
        assert_eq!(unity(l.jeu(2).unwrap()).map(|a| a.desaccord), Some(false));
        // `Vide` : seulement appliqué (des parties sur le disque), et alors avec `desaccord: true`.
        let mut v = Cache::default();
        appliquer(&mut v, 3, None, &json!({"sauvegardes": []}), &secours(), "10");
        appliquer_moteur(&mut v, 3, &constat_unity(), false);
        assert!(unity(v.jeu(3).unwrap()).is_none(), "pas appliqué : rien");
        appliquer_moteur(&mut v, 3, &constat_unity(), true);
        let a3 = unity(v.jeu(3).unwrap()).unwrap();
        assert_eq!(a3.corps()["desaccord"], true);
        // Un moteur sans nom à lire : `nom_lu` OMIS.
        let mut sans = v.jeu(3).unwrap().clone();
        sans.moteur_reconnu.as_mut().unwrap().nom_lu = None;
        assert!(unity(&sans).unwrap().corps().get("nom_lu").is_none());
        // Un nom lu de forme chemin : jamais envoyé.
        for mauvais in [r"C:\Jeux", "C:/Jeux", "/etc/x", r"a", "../x", "a/../b", "%APPDATA%", "a/b/c", "/a", "a/", &"x".repeat(201)] {
            sans.moteur_reconnu.as_mut().unwrap().nom_lu = Some(mauvais.to_string());
            assert!(unity(&sans).is_none(), "{mauvais}");
        }
        // Une preuve qui n'est pas un simple nom de fichier : jamais envoyée.
        sans.moteur_reconnu.as_mut().unwrap().nom_lu = None;
        for mauvais in ["js/rpg_core.js", r"Content\Paks", "", &"x".repeat(121)] {
            sans.moteur_reconnu.as_mut().unwrap().marqueur = mauvais.to_string();
            sans.moteur_reconnu.as_mut().unwrap().preuve = mauvais.to_string();
            assert!(unity(&sans).is_none(), "{mauvais}");
        }
        // Un jeu que le profil ne voit plus : ses signalements partent avec lui (le nom lu ne reste nulle part).
        let mut c = Cache::default();
        appliquer_moteur(&mut c, 1, &constat_unity(), false);
        noter_signalement(&mut c, 1, &a, EtatSignalement::Retenu, "30", None);
        retirer_si(&mut c, |_| false);
        assert!(c.jeu(1).is_none());
    }

    #[test]
    fn les_noms_lus_suivent_les_regles_de_firehouse() {
        for bon in ["MonJeu-1600000000", "Studio Été/Mon Produit", "Projet"] {
            assert!(nom_lu_envoyable(bon), "{bon}");
        }
        assert!(preuve_envoyable("UnityPlayer.dll") && preuve_envoyable("renpy") && preuve_envoyable("Game.rgss3a"));
    }
}
