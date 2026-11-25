//! Les MODS SWITCH servis par Firehouse (contrat 1.44, besoin 53 ; ✅ Seb 18/11). Firehouse les cherche, un admin les
//! valide un par un, il les range et les sert avec reprise ; Frogtend ne cherche ni ne télécharge JAMAIS un mod Switch
//! ailleurs (différent des mods PC du contrat 1.19, `mods.rs`). Décisions de Seb et avis des experts (18/11) :
//! - un mod vaut pour TOUTE LA MAISON (comme les DLC) : Frogtend est la source de vérité (`contenus-switch.json`,
//!   clés `mod:<clé>`, seul l'ÉTEINT est noté) ; une case cochée dans l'émulateur ne compte pas ;
//! - un mod qui ne va pas avec la mise à jour UTILISÉE reste visible, grisé avec sa raison, et n'est jamais posé
//!   (l'émulateur l'ignorerait en silence) ; une version inconnue ne grise pas (« pas vérifié ») ;
//! - les mods sont gardés À CÔTÉ DU JEU (`<dossier Frogtend du jeu>\mods-switch\<TID>\<clé>\`) et COPIÉS (jamais de
//!   jonction) dans l'émulateur à chaque partie, Ryubing (`<racine de la personne>\mods\contents\<tid>\<nom>\` +
//!   `games\<tid>\mods.json`) ou Eden (`<load>\<TID>\<nom>\` + `[DisabledAddOns]`) ;
//! - parties mises à l'abri avant la première partie qui suit TOUT changement de mod ; `reglage_requis` jamais posé
//!   seul ; noms « Update », « Update@… », « DLC », « SDMC » refusés ; un nom pris par la personne reçoit un suffixe ;
//! - un dossier posé par Frogtend puis modifié par la personne n'est jamais écrasé ni effacé.
//!
//! Ryubing (relu dans `src/Ryujinx.HLE/HOS/ModLoader.cs`, miroir GitHub Leuconoe/Ryubing du 05/11/2026 — le dépôt
//! officiel git.ryujinx.app refuse les lectures automatiques) : `CollectMods` ne cherche que dans `<racine>\mods\`
//! (`GetModsBasePath`, sous-dossier `contents`) et `<racine>\sdcard\atmosphere\` ; un mod hors de ces dossiers n'est
//! JAMAIS lu, même listé dans mods.json (d'où la copie). `AddModsFromDirectory` (L163-201) cherche l'entrée de
//! `games\<tid x16>\mods.json` par `modDir.Parent.FullName.Equals(x.Path)` — égalité EXACTE (casse comprise) du chemin
//! complet du dossier du mod — et un mod absent de la liste est ALLUMÉ (`modData?.Enabled ?? true`). Le JSON est en
//! snake_case (`JsonHelper` : `SnakeCaseNamingPolicy`) : `{"mods": [{"name", "path", "enabled"}]}` (Mod.cs,
//! ModMetadata.cs).

use crate::erreurs::{Erreur, Resultat};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Les sous-dossiers d'un mod que Frogtend garde et pose (tout le reste de l'archive est ignoré).
pub const SOUS_DOSSIERS: [&str; 3] = ["exefs", "romfs", "cheats"];
/// La note de Frogtend à côté des mods posés dans un émulateur (jamais dans le dossier d'un mod).
pub const NOTE_POSE: &str = ".frogtend-mods.json";
/// Marge gardée libre sur le disque en plus de l'archive et de son contenu.
const MARGE: u64 = 256 * 1024 * 1024;

// --- Le contrat (1.44) : lecture tolérante ---

/// Un entier du contrat : un nombre, ou un texte de chiffres (« 786432 »).
fn entier(v: &Value) -> Option<u64> {
    match v {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => s.trim().trim_start_matches(['v', 'V']).parse().ok(),
        _ => None,
    }
}

fn texte(v: &Value) -> String {
    v.as_str().unwrap_or("").to_string()
}

fn texte_ou_rien(v: &Value) -> Option<String> {
    v.as_str().map(str::trim).filter(|s| !s.is_empty()).map(String::from)
}

fn liste_de_textes(v: &Value) -> Vec<String> {
    v.as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from)).collect()
}

/// La mise à jour SERVIE par Firehouse pour ce jeu (`version_servie`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VersionServie {
    pub version: Option<u64>,
    pub version_affichee: Option<String>,
    pub build_id: Option<String>,
}

impl VersionServie {
    fn depuis(v: &Value) -> Option<Self> {
        v.is_object().then(|| VersionServie { version: entier(&v["version"]), version_affichee: texte_ou_rien(&v["version_affichee"]), build_id: texte_ou_rien(&v["build_id"]) })
    }
}

/// Une version visée par un mod.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VersionVisee {
    pub build_id: Option<String>,
    pub version_brute: Option<u64>,
    pub version_affichee: Option<String>,
}

/// Les versions visées par un mod : la chaîne « toutes » (romfs seul), une liste, ou une forme inconnue (jamais grisé).
#[derive(Debug, Clone, Default, PartialEq)]
pub enum Versions {
    Toutes,
    Liste(Vec<VersionVisee>),
    #[default]
    Inconnues,
}

impl Versions {
    fn depuis(v: &Value) -> Self {
        match v {
            Value::String(s) if s.trim().eq_ignore_ascii_case("toutes") => Versions::Toutes,
            Value::Array(a) => Versions::Liste(
                a.iter()
                    .filter(|x| x.is_object())
                    .map(|x| VersionVisee { build_id: texte_ou_rien(&x["build_id"]), version_brute: entier(&x["version_brute"]), version_affichee: texte_ou_rien(&x["version_affichee"]) })
                    .collect(),
            ),
            _ => Versions::Inconnues,
        }
    }

    fn valeur(&self) -> Value {
        match self {
            Versions::Toutes => json!("toutes"),
            Versions::Liste(l) => serde_json::to_value(l).unwrap_or(Value::Null),
            Versions::Inconnues => Value::Null,
        }
    }
}

impl Serialize for Versions {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        self.valeur().serialize(s)
    }
}

impl<'de> Deserialize<'de> for Versions {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        Ok(Versions::depuis(&Value::deserialize(d)?))
    }
}

/// Ce que contient un mod (`contenu {exefs, romfs, cheats}`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ContenuMod {
    pub exefs: bool,
    pub romfs: bool,
    pub cheats: bool,
}

/// Le fichier d'un mod (`fichier {url, taille, sha256}`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FichierServi {
    pub url: String,
    pub taille: u64,
    pub sha256: String,
}

/// Un mod servi par Firehouse.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ModServi {
    pub cle: String,
    pub nom: String,
    pub revision: u64,
    pub description: String,
    pub auteur: String,
    pub source: String,
    pub licence: String,
    pub valide_le: String,
    pub versions: Versions,
    pub compatible: Option<bool>,
    pub raison: Option<String>,
    pub emulateurs_essayes: Vec<String>,
    pub reglage_requis: String,
    pub conflits: Vec<String>,
    pub touche_sauvegardes: Option<bool>,
    pub contenu: ContenuMod,
    pub fichier: FichierServi,
    /// Contrat 1.46 (la fiche écrite par l'IA de Firehouse ; tout est `None` tant qu'elle n'est pas écrite) : la sorte
    /// (`technique`, `contenu`, `cosmetique`, `triche`, `autre`), l'intérêt (1 à 5) et le public (`tous`, `ado`,
    /// `adulte` — un mod « adulte » n'est servi qu'à un jeton admin).
    #[serde(rename = "type")]
    pub type_mod: Option<String>,
    pub interet: Option<u8>,
    pub public: Option<String>,
}

/// Le public « adulte » (contrat 1.46) : servi seulement à un jeton ADMIN.
pub const PUBLIC_ADULTE: &str = "adulte";

impl ModServi {
    /// Un mod lu dans la réponse ; `None` si sa clé n'a pas la forme du contrat (il est alors écarté).
    pub fn depuis(v: &Value) -> Option<Self> {
        let cle = texte(&v["cle"]);
        if !crate::contenus_switch::cle_de_mod_valide(&cle) {
            return None;
        }
        let nom = texte_ou_rien(&v["nom"]).unwrap_or_else(|| cle.clone());
        let c = &v["contenu"];
        let f = &v["fichier"];
        Some(ModServi {
            cle,
            nom,
            revision: entier(&v["revision"]).unwrap_or(0),
            description: texte(&v["description"]),
            auteur: texte(&v["auteur"]),
            source: texte(&v["source"]),
            licence: texte(&v["licence"]),
            valide_le: texte(&v["valide_le"]),
            versions: Versions::depuis(&v["versions"]),
            compatible: v["compatible"].as_bool(),
            raison: texte_ou_rien(&v["raison"]),
            emulateurs_essayes: liste_de_textes(&v["emulateurs_essayes"]),
            reglage_requis: texte(&v["reglage_requis"]).trim().to_string(),
            conflits: liste_de_textes(&v["conflits"]),
            touche_sauvegardes: v["touche_sauvegardes"].as_bool(),
            contenu: ContenuMod { exefs: c["exefs"].as_bool().unwrap_or(false), romfs: c["romfs"].as_bool().unwrap_or(false), cheats: c["cheats"].as_bool().unwrap_or(false) },
            fichier: FichierServi { url: texte(&f["url"]), taille: entier(&f["taille"]).unwrap_or(0), sha256: texte(&f["sha256"]).to_ascii_lowercase() },
            type_mod: texte_ou_rien(&v["type"]).map(|t| t.to_lowercase()),
            interet: entier(&v["interet"]).filter(|n| (1..=5).contains(n)).map(|n| n as u8),
            public: texte_ou_rien(&v["public"]).map(|t| t.to_lowercase()),
        })
    }
}

/// La liste des mods d'un jeu, lue.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ListeMods {
    pub version_servie: Option<VersionServie>,
    pub mods: Vec<ModServi>,
    /// Les mods écartés (clé hors contrat, en double) : dits au journal.
    pub ecartes: usize,
}

/// Lit la réponse de `GET /switch/{tid_base}/mods` (champs inconnus ignorés ; un mod illisible est écarté, les autres
/// gardés).
pub fn lire_reponse(v: &Value) -> ListeMods {
    let mut l = ListeMods { version_servie: VersionServie::depuis(&v["version_servie"]), ..Default::default() };
    for m in v["mods"].as_array().into_iter().flatten() {
        match ModServi::depuis(m) {
            Some(m) if !l.mods.iter().any(|x| x.cle == m.cle) => l.mods.push(m),
            _ => l.ecartes += 1,
        }
    }
    l
}

/// La route des mods d'un jeu (title id de BASE, 16 hexadécimaux majuscules, comme le contrat 1.42).
pub fn route_mods(tid_base: u64) -> String {
    format!("/switch/{tid_base:016X}/mods")
}

/// La route de l'archive d'un mod, tirée de `fichier.url` : SEULEMENT un chemin de l'API de Firehouse, exactement
/// `/api/jeux/v1/switch/mods/<clé>/archive`, avec la clé du mod. Une adresse complète, un autre hôte, une autre route
/// ou une autre clé sont refusés AVANT tout appel (le jeton ne part jamais ailleurs).
pub fn route_archive(url: &str, cle: &str) -> Resultat<String> {
    let refus = || Erreur::Refus("L'adresse du fichier de ce mod n'est pas celle de Firehouse : il n'est pas téléchargé.".into());
    let route = url.strip_prefix(crate::firehouse::PREFIXE).ok_or_else(refus)?;
    let reste = route.strip_prefix("/switch/mods/").and_then(|r| r.strip_suffix("/archive")).ok_or_else(refus)?;
    if reste != cle || !crate::contenus_switch::cle_de_mod_valide(reste) {
        return Err(refus());
    }
    Ok(route.to_string())
}

/// Une réponse simulée (mode sans Firehouse), tirée de la FORME du contrat 1.44 — ce n'est PAS un exemple réel (il n'y
/// en a pas encore : il viendra quand Seb aura validé les mods de BotW).
pub fn reponse_simulee(tid_base: u64) -> Value {
    json!({
        "ok": true,
        "_note": "forme du contrat 1.44, pas d'exemple réel",
        "tid_base": format!("{tid_base:016X}"),
        "version_servie": {"version": 786432, "version_affichee": "1.6.0", "build_id": "8E9978D50BDD20B4C8395A106C27FFDE"},
        "mods": [{
            "cle": "exemple-0123456789ab", "nom": "Mod d'exemple (simulé)", "revision": 1,
            "description": "Un mod imaginaire du mode simulé.", "auteur": "Frogtend", "source": "", "licence": "aucune",
            "valide_le": "2026-11-18T00:00:00", "versions": "toutes", "versions_annoncees": [],
            "compatible": null, "raison": "", "emulateurs_essayes": [], "reglage_requis": "", "conflits": [],
            "touche_sauvegardes": false, "contenu": {"exefs": false, "romfs": true, "cheats": false},
            "fichier": {"url": "/api/jeux/v1/switch/mods/exemple-0123456789ab/archive", "taille": 1, "sha256": "0".repeat(64)}
        }]
    })
}

// --- Le cache par PROFIL (hors ligne) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheMods {
    vue_le: String,
    reponse: Value,
}

/// La dernière liste lue pour ce jeu, dans le dossier de la SESSION (propre au profil : jamais ce qu'un compte a reçu
/// montré à un autre).
pub fn fichier_cache(dossier_session: &Path, tid_base: u64) -> PathBuf {
    dossier_session.join("mods-switch").join(format!("{tid_base:016X}.json"))
}

/// Qui écrit le cache : `Session::garder_fichier` en vrai (la péremption est vérifiée ET le fichier écrit sous le verrou
/// de la session — `perimer`/`oublier` ne peuvent pas passer entre les deux) ; rend `Erreur::Perimee` si la session a
/// été remplacée.
pub type Garder<'a> = &'a (dyn Fn(&Path, &[u8]) -> Resultat<()> + Send + Sync);

/// Garde la liste reçue — jamais si la session est périmée (autre profil, autre jeton) : ce qu'un ancien jeton a reçu
/// n'entre jamais dans le cache.
fn garder_cache(dossier_session: &Path, tid_base: u64, v: &Value, garder: Garder) -> Resultat<()> {
    let c = CacheMods { vue_le: crate::noyau::maintenant(), reponse: v.clone() };
    garder(&fichier_cache(dossier_session, tid_base), &serde_json::to_vec(&c).map_err(|e| Erreur::Disque(e.to_string()))?)
}

fn lire_cache(dossier_session: &Path, tid_base: u64) -> Option<CacheMods> {
    let f = fichier_cache(dossier_session, tid_base);
    if std::fs::metadata(&f).ok()?.len() > 8 << 20 {
        return None;
    }
    serde_json::from_slice(&std::fs::read(f).ok()?).ok()
}

/// Ce que donne la lecture des mods d'un jeu.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Lecture {
    /// « ok » (lu à l'instant), « hors_ligne » (Firehouse injoignable : la dernière liste lue, `vue_le`), « injoignable »
    /// (et rien de lu avant).
    pub etat: String,
    pub vue_le: Option<String>,
    pub liste: ListeMods,
}

fn perimee_erreur() -> Erreur {
    Erreur::Perimee("Le profil ou son jeton a changé pendant l'opération : rien de ce qui a été reçu n'est gardé.".into())
}

/// Lit les mods d'un jeu chez Firehouse et garde la liste pour le hors-ligne (`perimee` : la session a-t-elle été
/// remplacée ? alors rien n'est rendu ; `garder` : l'écriture du cache, sous le verrou de la session, qui refuse aussi
/// une session périmée — la liste n'est rendue que si elle a pu être gardée). 404 (jeu caché pour ce jeton, ou route
/// inconnue) → liste vide (et le cache vidé). SEULES une panne de réseau ou une erreur 5xx de Firehouse donnent la
/// dernière liste lue (« hors_ligne ») ou « injoignable » ; toute autre erreur (401, 400, 403, 409, réponse
/// illisible…) remonte telle quelle.
pub async fn lire(source: &crate::source::Source, dossier_session: &Path, tid_base: u64, perimee: &(dyn Fn() -> bool + Send + Sync), garder: Garder<'_>) -> Resultat<Lecture> {
    let r = source.mods_switch(tid_base).await;
    if perimee() {
        return Err(perimee_erreur());
    }
    match r {
        Ok(v) => {
            garder_cache(dossier_session, tid_base, &v, garder)?;
            Ok(Lecture { etat: "ok".into(), vue_le: None, liste: lire_reponse(&v) })
        }
        Err(Erreur::Introuvable(_)) => {
            garder_cache(dossier_session, tid_base, &json!({"mods": []}), garder)?;
            Ok(Lecture { etat: "ok".into(), vue_le: None, liste: ListeMods::default() })
        }
        Err(Erreur::Reseau(_) | Erreur::Serveur(_)) => Ok(match lire_cache(dossier_session, tid_base) {
            Some(c) => Lecture { etat: "hors_ligne".into(), vue_le: Some(c.vue_le), liste: lire_reponse(&c.reponse) },
            None => Lecture { etat: "injoignable".into(), vue_le: None, liste: ListeMods::default() },
        }),
        Err(e) => Err(e),
    }
}

// --- La compatibilité (✅ Seb 18/11, choix 3a) ---

/// L'état d'un mod face à la mise à jour utilisée.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Compat {
    /// « compatible », « pas_verifie » (jamais grisé) ou « incompatible » (grisé, jamais posé).
    pub etat: &'static str,
    pub raison: Option<String>,
}

impl Compat {
    pub fn grise(&self) -> bool {
        self.etat == "incompatible"
    }
}

/// La compatibilité d'un mod (`versions`, `compatible`, `raison` : ce que Firehouse en dit) avec la mise à jour
/// UTILISÉE (`utilisee` : sa version brute, 0 sans mise à jour, `None` inconnue), dans l'ordre : « toutes » →
/// compatible ; version inconnue → pas vérifié ; une version visée de même version brute → compatible ; la mise à jour
/// servie par Firehouse → son jugement (`null` = pas vérifié) ; sinon grisé, avec une raison sans jargon.
pub fn compatibilite(versions: &Versions, compatible: Option<bool>, raison: Option<&str>, servie: Option<&VersionServie>, utilisee: Option<u32>) -> Compat {
    let ok = Compat { etat: "compatible", raison: None };
    let pas_verifie = |r: &str| Compat { etat: "pas_verifie", raison: Some(r.into()) };
    if *versions == Versions::Toutes {
        return ok;
    }
    let Some(v) = utilisee.map(u64::from) else { return pas_verifie("la version du jeu utilisée n'est pas connue : pas vérifié") };
    if let Versions::Liste(l) = versions {
        if l.iter().any(|x| x.version_brute == Some(v)) {
            return ok;
        }
    }
    let affichee_servie = servie.and_then(|s| s.version_affichee.clone());
    if servie.and_then(|s| s.version) == Some(v) {
        return match compatible {
            Some(true) => ok,
            None => pas_verifie("pas encore vérifié avec cette version du jeu"),
            Some(false) => Compat {
                etat: "incompatible",
                raison: Some(raison.map(String::from).unwrap_or_else(|| match &affichee_servie {
                    Some(a) => format!("ne va pas avec la version {a} de ce jeu"),
                    None => "ne va pas avec la version de ce jeu".into(),
                })),
            },
        };
    }
    let l = match versions {
        // Une liste VIDE ne dit rien des versions visées : jamais grisé.
        Versions::Liste(l) if !l.is_empty() => l,
        _ => return pas_verifie("les versions visées par ce mod ne sont pas connues : pas vérifié"),
    };
    // Une entrée sans version brute (un Build ID dont Firehouse ne connaît pas encore la version : « inconnue »,
    // jamais « aucune ») : on ne peut pas trancher, jamais grisé (besoin 53, règle de Firehouse du 25/11).
    if l.iter().any(|x| x.version_brute.is_none()) {
        return pas_verifie("pas encore vérifié avec cette version du jeu");
    }
    let mut cibles: Vec<String> = l.iter().filter_map(|x| x.version_affichee.clone()).collect();
    cibles.dedup();
    let pour = if cibles.is_empty() { "fait pour une autre version du jeu".to_string() } else { format!("fait pour la version {}", cibles.join(" ou ")) };
    let utilise = if v == 0 { "la version d'origine (sans mise à jour)".to_string() } else { "une autre mise à jour".to_string() };
    Compat { etat: "incompatible", raison: Some(format!("{pour}, ce jeu utilise {utilise}")) }
}

// --- Le rangement à côté du jeu ---

/// Le dossier des mods Switch rangés par Frogtend, dans le dossier Frogtend du jeu.
pub const DOSSIER_MODS: &str = "mods-switch";

/// Le dossier des mods Switch d'un jeu : `<dossier Frogtend du jeu>\mods-switch\<TID>\` (pas `mods\`, celui des mods
/// PC). `frogtend` : `locale::dossier_frogtend(&jeu)`.
pub fn dossier_stock(frogtend: &Path, tid_base: u64) -> PathBuf {
    frogtend.join(DOSSIER_MODS).join(format!("{tid_base:016X}"))
}

/// Un fichier rangé (relatif au dossier du mod, séparateur `/`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FichierRange {
    pub chemin: String,
    pub taille: u64,
    pub sha256: String,
}

/// Le manifeste d'un mod rangé (`<stock>\<clé>.json`) : tout ce qu'il faut pour le poser HORS LIGNE (compatibilité,
/// conflits, réglage requis) et pour le retirer avec preuve.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManifesteMod {
    pub cle: String,
    pub nom: String,
    pub revision: u64,
    /// L'empreinte de l'archive reçue.
    pub sha256: String,
    /// La taille de l'archive reçue.
    pub taille: u64,
    #[serde(default)]
    pub versions: Versions,
    #[serde(default)]
    pub version_servie: Option<VersionServie>,
    #[serde(default)]
    pub compatible: Option<bool>,
    #[serde(default)]
    pub raison: Option<String>,
    #[serde(default)]
    pub conflits: Vec<String>,
    #[serde(default)]
    pub reglage_requis: String,
    #[serde(default)]
    pub touche_sauvegardes: Option<bool>,
    #[serde(default)]
    pub contenu: ContenuMod,
    pub range_le: String,
    pub fichiers: Vec<FichierRange>,
    /// Contrat 1.46 : la fiche de Firehouse (pour la montrer hors ligne), et le public.
    #[serde(default)]
    pub description: String,
    #[serde(default, rename = "type")]
    pub type_mod: Option<String>,
    #[serde(default)]
    pub interet: Option<u8>,
    #[serde(default)]
    pub public: Option<String>,
    /// Firehouse a dit « adulte » au moins une fois pour ce mod : tant que sa fiche dit `null`, il reste réservé.
    #[serde(default)]
    pub adulte_vu: bool,
}

impl ManifesteMod {
    /// Un mod RÉSERVÉ (✅ brief § 2, contrat 1.46 ; relecture des experts) : tout mod dont le public n'est pas
    /// EXPLICITEMENT « tous » ou « ado » — « adulte », fiche pas (encore) écrite (`null`, rangé avant la 1.46 ou avant
    /// sa fiche), valeur inconnue. Les mods rangés sont communs à la maison : un mod réservé n'est montré et posé que
    /// pour un profil dont la DERNIÈRE liste reçue le contient (la liste fraîche, ou son cache hors ligne), et n'est
    /// jamais nommé dans ce qui est commun (journal, suivi, Eden).
    pub fn reserve(&self) -> bool {
        !public_pour_tous(self.public.as_deref())
    }

    /// Un mod à TAIRE dans ce qui est commun (journal, raisons partagées, noms de dossier d'Eden) : réservé, ou déjà vu
    /// « adulte » une fois (même reclassé depuis).
    pub fn a_taire(&self) -> bool {
        self.reserve() || self.adulte_vu
    }
}

/// Un public qu'on peut montrer à tout profil : « tous » ou « ado » (contrat 1.46), rien d'autre.
pub fn public_pour_tous(p: Option<&str>) -> bool {
    matches!(p, Some("tous" | "ado"))
}

/// Les mods rangés que CE profil peut voir et poser (`liste` : sa dernière liste reçue ; `None` : il n'en a pas) : ceux
/// de sa liste, plus ceux dont le public est « tous » ou « ado ». Tout autre (« adulte », `null`, inconnu) absent de sa
/// liste est ÉCARTÉ (jamais effacé : le profil qui le reçoit le retrouve tel quel).
pub fn visibles(rangs: Vec<ManifesteMod>, liste: Option<&ListeMods>) -> Vec<ManifesteMod> {
    rangs.into_iter().filter(|r| visible_pour(r, liste)).collect()
}

/// Ce mod rangé est-il visible pour le profil dont `liste` est la dernière liste ? Un mod « adulte » (ou déjà vu
/// « adulte ») n'est visible que si sa liste le contient AVEC `public: "adulte"` : une entrée `null` ou « tous » d'un
/// cache plus ancien (servie avant la classification de la nuit) ne suffit jamais.
fn visible_pour(r: &ManifesteMod, liste: Option<&ListeMods>) -> bool {
    let entree = liste.and_then(|l| l.mods.iter().find(|m| m.cle == r.cle));
    if r.adulte_vu || r.public.as_deref() == Some(PUBLIC_ADULTE) {
        return entree.is_some_and(|m| m.public.as_deref() == Some(PUBLIC_ADULTE));
    }
    !r.reserve() || entree.is_some()
}

/// La dernière liste de mods reçue par un profil pour ce jeu (son cache, dans le dossier de SA session).
pub fn liste_du_profil(dossier_session: &Path, tid_base: u64) -> Option<ListeMods> {
    lire_cache(dossier_session, tid_base).map(|c| lire_reponse(&c.reponse))
}

fn manifeste_de(stock: &Path, cle: &str) -> PathBuf {
    stock.join(format!("{cle}.json"))
}

/// Les mods rangés pour ce jeu (manifestes lus, plafonnés, vérifiés : clé du contrat, nom de fichier = clé, dossier
/// présent, chemins sûrs). Un manifeste douteux n'existe pas pour Frogtend.
pub fn ranges(stock: &Path) -> Vec<ManifesteMod> {
    let Ok(l) = std::fs::read_dir(stock) else { return vec![] };
    let mut r: Vec<ManifesteMod> = l
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter(|e| e.metadata().is_ok_and(|m| m.is_file() && m.len() <= 4 << 20))
        .filter_map(|e| {
            let m: ManifesteMod = serde_json::from_slice(&std::fs::read(e.path()).ok()?).ok()?;
            let sain = crate::contenus_switch::cle_de_mod_valide(&m.cle)
                && e.file_name().to_string_lossy() == format!("{}.json", m.cle)
                && stock.join(&m.cle).is_dir()
                && m.fichiers.iter().all(|f| crate::decompression::chemin_sur(&f.chemin).as_deref() == Some(f.chemin.as_str()) && premier_sous_dossier(&f.chemin).is_some());
            sain.then_some(m)
        })
        .collect();
    r.sort_by(|a, b| a.nom.to_lowercase().cmp(&b.nom.to_lowercase()));
    r
}

/// Le premier élément d'un chemin, s'il est exefs / romfs / cheats (sans casse) : rendu en minuscules.
fn premier_sous_dossier(chemin: &str) -> Option<&'static str> {
    let p = chemin.split('/').next()?;
    SOUS_DOSSIERS.iter().find(|s| s.eq_ignore_ascii_case(p)).copied()
}

/// Tous les fichiers d'un dossier (relatifs, `/`), sans suivre de lien ; un lien ou une jonction rend une erreur (un
/// dossier qui en contient n'est jamais considéré comme « inchangé »).
fn fichiers_du_dossier(d: &Path) -> std::io::Result<Vec<String>> {
    let mut l = Vec::new();
    let mut pile = vec![(d.to_path_buf(), String::new())];
    while let Some((p, rel)) = pile.pop() {
        for e in std::fs::read_dir(&p)? {
            let e = e?;
            let t = e.file_type()?;
            let nom = e.file_name().to_string_lossy().to_string();
            let r = if rel.is_empty() { nom } else { format!("{rel}/{nom}") };
            if t.is_symlink() {
                return Err(std::io::Error::other("lien"));
            } else if t.is_dir() {
                pile.push((e.path(), r));
            } else {
                l.push(r);
            }
        }
        if l.len() > 200_000 {
            return Err(std::io::Error::other("trop de fichiers"));
        }
    }
    l.sort();
    Ok(l)
}

fn sha256_de(f: &Path) -> Resultat<String> {
    crate::sauvegarde::empreinte(f)
}

/// Le dossier rangé d'un mod est-il exactement ce que dit son manifeste (mêmes fichiers, mêmes tailles, mêmes
/// empreintes, rien de plus) ?
pub fn range_intact(stock: &Path, m: &ManifesteMod) -> bool {
    dossier_conforme(&stock.join(&m.cle), &m.fichiers)
}

/// Le dossier `d` contient-il exactement `fichiers` (mêmes chemins, tailles, empreintes, rien de plus) ?
fn dossier_conforme(d: &Path, fichiers: &[FichierRange]) -> bool {
    let Ok(l) = fichiers_du_dossier(d) else { return false };
    let mut attendus: Vec<&str> = fichiers.iter().map(|f| f.chemin.as_str()).collect();
    attendus.sort();
    l == attendus
        && fichiers.iter().all(|f| {
            let p = d.join(f.chemin.replace('/', "\\"));
            std::fs::metadata(&p).is_ok_and(|x| x.len() == f.taille) && sha256_de(&p).is_ok_and(|s| s.eq_ignore_ascii_case(&f.sha256))
        })
}

/// Un nom libre dans le rangement pour mettre un dossier de côté (`<clé>.<sorte>-<horodatage>[-n]`) : jamais
/// par-dessus un dossier existant.
fn de_cote_libre(stock: &Path, cle: &str, sorte: &str) -> PathBuf {
    let base = format!("{cle}.{sorte}-{}", crate::noyau::maintenant());
    let mut p = stock.join(&base);
    let mut n = 2;
    while p.exists() {
        p = stock.join(format!("{base}-{n}"));
        n += 1;
    }
    p
}

/// `ranger_avec` sans rien à faire avant l'échange des dossiers.
pub fn ranger(stock: &Path, archive: &Path, m: &ModServi, servie: Option<&VersionServie>) -> Resultat<ManifesteMod> {
    ranger_avec(stock, archive, m, servie, &|_| Ok(()))
}

/// Range un mod depuis son archive reçue (`archive`, déjà vérifiée : taille et sha256). L'archive doit être un ZIP
/// dont CHAQUE entrée est « stockée » (jamais compressée, contrat 1.44), aux chemins sûrs, plafonnée à la taille
/// annoncée ; racine = un dossier de mod, ou `exefs/`, `romfs/`, `cheats/` directement ; seuls ces trois dossiers sont
/// gardés. Une révision déjà rangée est gardée tant que la nouvelle n'est pas vérifiée, puis remplacée (un dossier
/// rangé modifié sur le disque est mis de côté, jamais effacé). `avant_echange` (le nouveau manifeste) est appelé une
/// fois la nouvelle révision vérifiée, AVANT l'échange des dossiers : noter le changement de mod (abri des parties) ;
/// une erreur annule tout (l'ancienne révision reste).
pub fn ranger_avec(stock: &Path, archive: &Path, m: &ModServi, servie: Option<&VersionServie>, avant_echange: &dyn Fn(&ManifesteMod) -> Resultat<()>) -> Resultat<ManifesteMod> {
    if crate::mods::format_mod(archive) != Some(crate::mods::FormatMod::Zip) {
        return Err(Erreur::Refus("Le fichier de ce mod n'est pas une archive que Frogtend sait ouvrir : il n'est pas installé.".into()));
    }
    let mut z = zip::ZipArchive::new(std::fs::File::open(archive)?).map_err(|e| Erreur::Disque(format!("Archive du mod illisible ({e}).")))?;
    for i in 0..z.len() {
        let e = z.by_index_raw(i).map_err(|e| Erreur::Disque(format!("Archive du mod abîmée ({e}).")))?;
        if e.compression() != zip::CompressionMethod::Stored {
            return Err(Erreur::Refus("Le fichier de ce mod a une forme que Frogtend ne sait pas installer : rien n'est installé.".into()));
        }
    }
    let entrees = crate::mods::lister_plafonne(archive, m.fichier.taille)?;
    // La racine : un seul dossier qui n'est pas exefs/romfs/cheats → c'est le dossier du mod.
    let prefixe = crate::mods::racine_unique(&entrees).filter(|r| premier_sous_dossier(r).is_none()).map(|r| format!("{r}/"));
    // (chemin sûr dans l'archive → chemin rangé, taille)
    let mut garder: BTreeMap<String, (String, u64)> = BTreeMap::new();
    for e in &entrees {
        let rel = match &prefixe {
            Some(p) => e.chemin.strip_prefix(p.as_str()).unwrap_or(&e.chemin).to_string(),
            None => e.chemin.clone(),
        };
        match premier_sous_dossier(&rel) {
            Some(s) => {
                let range = format!("{s}{}", &rel[s.len()..]);
                garder.insert(e.chemin.clone(), (range, e.taille));
            }
            // Tout le reste (lisez-moi, images…) est ignoré : jamais posé dans l'émulateur.
            None => {}
        }
    }
    if garder.is_empty() {
        return Err(Erreur::Refus("Ce mod ne contient rien que l'émulateur sache utiliser : rien n'est installé.".into()));
    }
    let total: u64 = garder.values().map(|(_, t)| *t).sum();
    std::fs::create_dir_all(stock)?;
    if crate::jeux_pc::place_libre(stock).is_some_and(|l| l < total + MARGE) {
        return Err(Erreur::Refus("Il n'y a plus assez de place sur ce disque pour installer ce mod.".into()));
    }
    let nouveau = stock.join(format!("{}.nouveau", m.cle));
    if nouveau.exists() {
        // Un reste d'une pose interrompue de Frogtend (dossier à lui, jamais montré à la personne).
        std::fs::remove_dir_all(&nouveau)?;
    }
    let mut fichiers: Vec<FichierRange> = Vec::new();
    let resultat = (|| -> Resultat<()> {
        for i in 0..z.len() {
            let mut e = z.by_index(i).map_err(|e| Erreur::Disque(format!("Archive du mod abîmée ({e}).")))?;
            if e.is_dir() {
                continue;
            }
            let Some(sur) = crate::decompression::chemin_sur(e.name()) else { continue };
            let Some((range, taille)) = garder.get(&sur).cloned() else { continue };
            let cible = nouveau.join(range.replace('/', "\\"));
            if let Some(p) = cible.parent() {
                std::fs::create_dir_all(p)?;
            }
            let mut sortie = std::fs::File::create(&cible)?;
            let mut h = <sha2::Sha256 as sha2::Digest>::new();
            let mut tampon = vec![0u8; 256 * 1024];
            let mut ecrits = 0u64;
            loop {
                let n = e.read(&mut tampon).map_err(|x| Erreur::Disque(format!("Archive du mod abîmée ({x}).")))?;
                if n == 0 {
                    break;
                }
                ecrits += n as u64;
                if ecrits > taille {
                    return Err(Erreur::Refus("Le fichier de ce mod est abîmé : rien n'est installé.".into()));
                }
                sha2::Digest::update(&mut h, &tampon[..n]);
                sortie.write_all(&tampon[..n])?;
            }
            sortie.flush()?;
            drop(sortie);
            if ecrits != taille || std::fs::metadata(&cible)?.len() != taille {
                return Err(Erreur::Disque("Un fichier du mod a été mal écrit : rien n'est installé.".into()));
            }
            let sha: String = sha2::Digest::finalize(h).iter().map(|b| format!("{b:02x}")).collect();
            fichiers.push(FichierRange { chemin: range, taille, sha256: sha });
        }
        Ok(())
    })();
    if let Err(e) = resultat {
        let _ = std::fs::remove_dir_all(&nouveau);
        return Err(e);
    }
    fichiers.sort_by(|a, b| a.chemin.cmp(&b.chemin));
    let man = ManifesteMod {
        cle: m.cle.clone(),
        nom: m.nom.clone(),
        revision: m.revision,
        sha256: m.fichier.sha256.clone(),
        taille: m.fichier.taille,
        versions: m.versions.clone(),
        version_servie: servie.cloned(),
        compatible: m.compatible,
        raison: m.raison.clone(),
        conflits: m.conflits.clone(),
        reglage_requis: m.reglage_requis.clone(),
        touche_sauvegardes: m.touche_sauvegardes,
        contenu: m.contenu.clone(),
        range_le: crate::noyau::maintenant(),
        fichiers,
        description: m.description.clone(),
        type_mod: m.type_mod.clone(),
        interet: m.interet,
        public: m.public.clone(),
        // « Adulte » une fois : retenu pour toujours (une révision suivante sans fiche reste réservée).
        adulte_vu: m.public.as_deref() == Some(PUBLIC_ADULTE) || ranges(stock).iter().any(|x| x.cle == m.cle && x.adulte_vu),
    };
    // La nouvelle révision est vérifiée sur le disque AVANT tout échange (relue : chemins, tailles, empreintes).
    if !dossier_conforme(&nouveau, &man.fichiers) {
        let _ = std::fs::remove_dir_all(&nouveau);
        return Err(Erreur::Disque("Le mod reçu ne se relit pas comme prévu : rien n'est changé.".into()));
    }
    if let Err(e) = avant_echange(&man) {
        let _ = std::fs::remove_dir_all(&nouveau);
        return Err(e);
    }
    // L'ancienne révision : mise de côté sous un nom neuf (jamais par-dessus) ; effacée à la fin seulement si elle était
    // INTACTE (vérifiée ici même) et que la nouvelle se relit.
    let dossier = stock.join(&m.cle);
    let f = manifeste_de(stock, &m.cle);
    let manifeste_avant = std::fs::read(&f).ok();
    let mut de_cote: Option<(PathBuf, bool)> = None;
    if dossier.exists() {
        let intact = ranges(stock).into_iter().find(|x| x.cle == m.cle).is_some_and(|a| range_intact(stock, &a));
        let d = de_cote_libre(stock, &m.cle, if intact { "ancien" } else { "garde" });
        if let Err(e) = std::fs::rename(&dossier, &d) {
            let _ = std::fs::remove_dir_all(&nouveau);
            return Err(e.into());
        }
        de_cote = Some((d, intact));
    }
    // En cas d'échec : la nouvelle (vérifiée, à Frogtend) part, l'ancienne et son manifeste reviennent.
    let remettre = |de_cote: &Option<(PathBuf, bool)>| {
        if dossier_conforme(&dossier, &man.fichiers) {
            let _ = std::fs::remove_dir_all(&dossier);
        }
        let _ = std::fs::remove_dir_all(&nouveau);
        if let Some((d, _)) = de_cote {
            let _ = std::fs::rename(d, &dossier);
        }
        match &manifeste_avant {
            Some(o) => {
                let _ = std::fs::write(&f, o);
            }
            None => {
                let _ = std::fs::remove_file(&f);
            }
        }
    };
    if let Err(e) = std::fs::rename(&nouveau, &dossier) {
        remettre(&de_cote);
        return Err(e.into());
    }
    let tmp = f.with_extension("json.tmp");
    let ecrit = serde_json::to_vec_pretty(&man).map_err(|e| Erreur::Disque(e.to_string())).and_then(|o| {
        std::fs::write(&tmp, o)?;
        std::fs::rename(&tmp, &f)?;
        Ok(())
    });
    // Vérifier le résultat, pas le retour.
    if ecrit.is_err() || !ranges(stock).iter().any(|x| *x == man) || !range_intact(stock, &man) {
        remettre(&de_cote);
        return Err(ecrit.err().unwrap_or_else(|| Erreur::Disque("Le mod rangé ne se relit pas comme prévu : l'ancienne révision est remise.".into())));
    }
    if let Some((d, true)) = de_cote {
        let _ = std::fs::remove_dir_all(d);
    }
    Ok(man)
}

/// Remet à jour ce que le manifeste d'un mod rangé retient du jugement de Firehouse (versions visées, compatibilité,
/// conflits, réglage requis), quand Firehouse sert la MÊME révision (même empreinte) : la pose hors ligne suit alors
/// son dernier avis. Rend le nombre de manifestes réécrits.
pub fn rafraichir(stock: &Path, liste: &ListeMods) -> usize {
    // Jamais pendant un téléchargement, un rangement ou un retrait de ce rangement (ce sera pour la lecture suivante).
    let Ok(_garde) = occuper_stock(stock) else { return 0 };
    static NUMERO: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let mut n = 0;
    for mut r in ranges(stock) {
        let Some(m) = liste.mods.iter().find(|m| m.cle == r.cle && m.revision == r.revision && m.fichier.sha256.eq_ignore_ascii_case(&r.sha256)) else { continue };
        let avant = r.clone();
        r.versions = m.versions.clone();
        r.version_servie = liste.version_servie.clone();
        r.compatible = m.compatible;
        r.raison = m.raison.clone();
        r.conflits = m.conflits.clone();
        r.reglage_requis = m.reglage_requis.clone();
        r.touche_sauvegardes = m.touche_sauvegardes;
        r.description = m.description.clone();
        r.type_mod = m.type_mod.clone();
        r.interet = m.interet;
        r.public = m.public.clone();
        r.adulte_vu |= m.public.as_deref() == Some(PUBLIC_ADULTE);
        if r == avant {
            continue;
        }
        let f = manifeste_de(stock, &r.cle);
        // Un provisoire propre à cet appel (jamais celui de `ranger_avec`).
        let tmp = stock.join(format!("{}.json.rafraichi-{}-{}.tmp", r.cle, std::process::id(), NUMERO.fetch_add(1, std::sync::atomic::Ordering::SeqCst)));
        let ok = serde_json::to_vec_pretty(&r).ok().is_some_and(|o| std::fs::write(&tmp, o).is_ok() && std::fs::rename(&tmp, &f).is_ok());
        if !ok {
            let _ = std::fs::remove_file(&tmp);
        }
        if ok {
            n += 1;
        }
    }
    n
}

/// Retire un mod rangé (à la demande de la personne) : seulement s'il est INCHANGÉ (preuve par le manifeste), comme
/// `mods.rs`. Ses copies dans les émulateurs partent à la partie suivante (si elles sont intactes).
pub fn retirer(stock: &Path, cle: &str) -> Resultat<()> {
    let _garde = occuper_stock(stock)?;
    let m = ranges(stock).into_iter().find(|m| m.cle == cle).ok_or_else(|| Erreur::Introuvable("Ce mod n'est pas installé.".into()))?;
    if !range_intact(stock, &m) {
        return Err(Erreur::Refus("Les fichiers de ce mod ont été modifiés sur le disque : Frogtend ne les efface pas.".into()));
    }
    // D'abord mis hors de vue sous un nom à Frogtend (plus jamais pris pour le mod), puis effacé, puis le manifeste :
    // une coupure en route laisse au pire un `.retire-…` (à Frogtend) et jamais un mod à moitié effacé.
    let retire = de_cote_libre(stock, cle, "retire");
    std::fs::rename(stock.join(cle), &retire)?;
    std::fs::remove_dir_all(&retire)?;
    std::fs::remove_file(manifeste_de(stock, cle))?;
    Ok(())
}

// --- Le téléchargement (après l'accord de la personne) ---

/// Le nom du fichier reçu : la clé, la révision et le début de l'empreinte (une nouvelle révision ne reprend jamais
/// le `.part` d'une autre).
pub fn nom_archive(m: &ModServi) -> String {
    format!("{}-r{}-{}.zip", m.cle, m.revision, &m.fichier.sha256[..12.min(m.fichier.sha256.len())])
}

/// Télécharge l'archive d'un mod (reprise `Range`, plafond = taille annoncée, taille et sha256 vérifiés) dans
/// `<stock>\.telechargements\`. Rend le chemin du fichier complet et vérifié. Un 409 est rendu tel quel (jamais de
/// nouvel essai : la liste est à relire).
pub async fn telecharger(source: &crate::source::Source, stock: &Path, m: &ModServi, progres: &(dyn Fn(u64, Option<u64>) + Send + Sync)) -> Resultat<PathBuf> {
    let route = route_archive(&m.fichier.url, &m.cle)?;
    if m.fichier.sha256.len() != 64 || !m.fichier.sha256.chars().all(|c| c.is_ascii_hexdigit()) || m.fichier.taille == 0 {
        return Err(Erreur::Refus("Firehouse ne dit pas tout ce qu'il faut pour vérifier ce mod : il n'est pas téléchargé.".into()));
    }
    let dossier = stock.join(".telechargements");
    std::fs::create_dir_all(&dossier)?;
    if crate::jeux_pc::place_libre(&dossier).is_some_and(|l| l < m.fichier.taille.saturating_mul(2) + MARGE) {
        return Err(Erreur::Refus("Il n'y a plus assez de place sur ce disque pour ce mod.".into()));
    }
    let nom = nom_archive(m);
    // Les `.part` d'une AUTRE révision de ce mod (fichiers de Frogtend, incomplets par nature) : retirés.
    for e in std::fs::read_dir(&dossier).into_iter().flatten().flatten() {
        let n = e.file_name().to_string_lossy().to_string();
        if n.starts_with(&format!("{}-r", m.cle)) && n.ends_with(".part") && n != nom.replace(".zip", ".part") {
            let _ = std::fs::remove_file(e.path());
        }
    }
    let cible = dossier.join(&nom);
    let recus = source.telecharger_plafonne(&route, &cible, Some(&m.fichier.sha256), &[], Some(m.fichier.taille), progres).await?;
    let taille = std::fs::metadata(&cible).map(|x| x.len()).unwrap_or(0);
    if recus != m.fichier.taille || taille != m.fichier.taille {
        let _ = std::fs::remove_file(&cible);
        return Err(Erreur::Serveur("Le fichier reçu n'a pas la taille annoncée par Firehouse : il n'est pas gardé.".into()));
    }
    Ok(cible)
}

/// Télécharge puis range un mod ; l'archive reçue est retirée une fois le mod rangé.
/// Le rangement est occupé (`occuper_stock`) du début du téléchargement à la fin du rangement. `avant_echange` : voir
/// `ranger_avec` (noter l'abri des parties, éteindre un mod en conflit).
pub async fn telecharger_et_ranger(
    source: &crate::source::Source,
    stock: &Path,
    m: &ModServi,
    servie: Option<&VersionServie>,
    progres: &(dyn Fn(u64, Option<u64>) + Send + Sync),
    avant_echange: Box<dyn Fn(&ManifesteMod) -> Resultat<()> + Send>,
) -> Resultat<ManifesteMod> {
    let _garde = occuper_stock(stock)?;
    let archive = telecharger(source, stock, m, progres).await?;
    let (s, a, mm, sv) = (stock.to_path_buf(), archive.clone(), m.clone(), servie.cloned());
    let r = tokio::task::spawn_blocking(move || ranger_avec(&s, &a, &mm, sv.as_ref(), &*avant_echange))
        .await
        .map_err(|_| Erreur::Disque("L'installation du mod s'est arrêtée brutalement.".into()))?;
    let _ = std::fs::remove_file(&archive);
    r
}

/// Ce qu'il faut noter à l'arrivée d'un mod (avant l'échange des dossiers, `donnees` : où vit contenus-switch.json) :
/// le changement de mod (abri des parties avant la prochaine partie) et, pour un mod NOUVEAU dont un mod en conflit
/// est déjà allumé, « éteint » (la personne choisira lequel garder).
pub fn noter_arrivee(donnees: &Path, stock: &Path, base: u64, m: &ManifesteMod) -> Resultat<bool> {
    let rangs = ranges(stock);
    let nouveau = !rangs.iter().any(|r| r.cle == m.cle);
    let choix = crate::contenus_switch::charger(donnees).0.jeu(base);
    let en_conflit = nouveau
        && rangs.iter().any(|b| b.cle != m.cle && choix.mod_actif(&b.cle) && (m.conflits.contains(&b.cle) || b.conflits.contains(&m.cle)));
    if en_conflit {
        crate::contenus_switch::choisir_mods(donnees, base, &[(m.cle.clone(), false)])?;
    }
    crate::contenus_switch::noter_changement_de_mod(donnees, base, None)?;
    Ok(en_conflit)
}

// --- Les noms de dossier dans les émulateurs ---

/// Un nom qu'Eden réserve (`[DisabledAddOns]` : « Update », « Update@… », « DLC ») ou qui désigne la carte SD
/// (« SDMC ») : jamais le nom d'un mod de Frogtend (sans casse).
pub fn nom_reserve(nom: &str) -> bool {
    let n = nom.trim().to_lowercase();
    n == "update" || n.starts_with("update@") || n == "dlc" || n == "sdmc"
}

/// Le nom de dossier d'un mod, lisible et sûr, tiré de son nom (jamais de séparateur, de nom réservé de Windows, de
/// point final) ; un nom réservé des émulateurs est préfixé par « Mod ».
pub fn nom_de_dossier(nom_mod: &str, cle: &str) -> String {
    let mut s = String::new();
    for c in nom_mod.chars() {
        let c = if c.is_alphanumeric() || " -_.()[]+,'&!".contains(c) { c } else { ' ' };
        if !(c == ' ' && s.ends_with(' ')) {
            s.push(c);
        }
    }
    let mut s: String = s.trim().chars().take(60).collect();
    while s.ends_with(['.', ' ']) {
        s.pop();
    }
    let base = s.split('.').next().unwrap_or("").trim().to_ascii_uppercase();
    let windows = ["CON", "PRN", "AUX", "NUL"].contains(&base.as_str()) || (base.len() == 4 && (base.starts_with("COM") || base.starts_with("LPT")) && base.as_bytes()[3].is_ascii_digit());
    if s.is_empty() || windows {
        s = format!("Mod {cle}");
    }
    if nom_reserve(&s) {
        s = format!("Mod {s}");
    }
    s
}

/// Un nom libre : `voulu`, sinon « voulu (2) », « voulu (3) »… (`pris` : le nom est-il déjà pris, sans casse ?).
pub fn nom_libre(voulu: &str, pris: &dyn Fn(&str) -> bool) -> String {
    if !pris(voulu) {
        return voulu.to_string();
    }
    (2..10_000).map(|n| format!("{voulu} ({n})")).find(|n| !pris(n)).unwrap_or_else(|| format!("{voulu} ({})", crate::noyau::maintenant()))
}

// --- La pose dans un émulateur (au LANCEMENT seulement) ---

fn vrai() -> bool {
    true
}

/// Ce que Frogtend a posé dans le dossier des mods d'un jeu d'un émulateur : par clé, le nom du dossier, la révision,
/// l'empreinte de l'archive, s'il est utilisé, et la taille + date de chaque fichier (`.frogtend-mods.json`, à côté
/// des mods — jamais dedans).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PoseNotee {
    pub dossier: String,
    pub revision: u64,
    /// L'empreinte de l'archive du mod posé : même révision, autre empreinte = reposé.
    #[serde(default)]
    pub sha256: String,
    /// L'émulateur l'utilise-t-il (mods.json, `[DisabledAddOns]`) ? Faux pour un dossier modifié gardé mais coupé.
    #[serde(default = "vrai")]
    pub actif: bool,
    pub fichiers: BTreeMap<String, (u64, u64)>,
}

/// Un nom de dossier SIMPLE (lu dans la note : jamais un chemin, jamais un nom que Windows ou Frogtend réserve).
fn nom_simple(n: &str) -> bool {
    let base = n.split('.').next().unwrap_or("").trim().to_ascii_uppercase();
    let windows = ["CON", "PRN", "AUX", "NUL"].contains(&base.as_str()) || (base.len() == 4 && (base.starts_with("COM") || base.starts_with("LPT")) && base.as_bytes()[3].is_ascii_digit());
    !n.trim().is_empty()
        && n.chars().count() <= 120
        && !n.chars().any(|c| c.is_control() || "/\\:*?\"<>|".contains(c))
        && n != "."
        && n != ".."
        && !n.ends_with(['.', ' '])
        && !n.to_lowercase().starts_with(".frogtend")
        && !windows
}

/// La note d'un dossier de mods ; `None` si elle ne se lit pas. Une ligne douteuse (clé hors contrat, nom de dossier
/// qui n'est pas un nom simple) est ignorée.
fn lire_note(d: &Path) -> Option<BTreeMap<String, PoseNotee>> {
    match std::fs::read(d.join(NOTE_POSE)) {
        Ok(o) => {
            let l: BTreeMap<String, PoseNotee> = serde_json::from_slice(&o).ok()?;
            Some(l.into_iter().filter(|(c, p)| crate::contenus_switch::cle_de_mod_valide(c) && nom_simple(&p.dossier)).collect())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Some(BTreeMap::new()),
        Err(_) => None,
    }
}

fn ecrire_note(d: &Path, n: &BTreeMap<String, PoseNotee>) -> Resultat<()> {
    let f = d.join(NOTE_POSE);
    if n.is_empty() {
        if f.exists() {
            std::fs::remove_file(&f)?;
        }
        return Ok(());
    }
    std::fs::create_dir_all(d)?;
    let tmp = d.join(format!("{NOTE_POSE}.tmp"));
    std::fs::write(&tmp, serde_json::to_vec_pretty(n).map_err(|e| Erreur::Disque(e.to_string()))?)?;
    std::fs::rename(&tmp, &f)?;
    Ok(())
}

fn date_ns(m: &std::fs::Metadata) -> u64 {
    m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_nanos() as u64).unwrap_or(0)
}

/// Le relevé (taille, date) de chaque fichier d'un dossier posé.
fn releve(d: &Path) -> std::io::Result<BTreeMap<String, (u64, u64)>> {
    let mut r = BTreeMap::new();
    for f in fichiers_du_dossier(d)? {
        let m = std::fs::metadata(d.join(f.replace('/', "\\")))?;
        r.insert(f, (m.len(), date_ns(&m)));
    }
    Ok(r)
}

/// Le dossier posé est-il encore exactement ce que Frogtend y a mis ?
fn pose_intacte(d: &Path, p: &PoseNotee) -> bool {
    releve(d).is_ok_and(|r| r == p.fichiers)
}

/// Le préfixe des copies en cours (À CÔTÉ du dossier du jeu, dans `contents\` ou `load\` : un nom qui n'est pas un
/// title id, jamais lu comme un mod par Ryubing ni par Eden).
const PREFIXE_COPIE: &str = ".frogtend-copie-";
/// La marque posée dans une copie en cours : seule une copie qui la porte est effacée (elle est à Frogtend).
const MARQUE_COPIE: &str = ".frogtend-copie-en-cours";

/// Copie le mod rangé `source` dans un dossier provisoire à Frogtend, à côté de `dossier_jeu` (même disque : il sera
/// renommé à sa place). Rend ce dossier. Un reste d'une copie interrompue n'est effacé que s'il porte la marque.
fn copier_mod(source: &Path, dossier_jeu: &Path, cle: &str) -> Resultat<PathBuf> {
    let parent = dossier_jeu.parent().ok_or_else(|| Erreur::Disque("dossier des mods inattendu".into()))?;
    let tid = dossier_jeu.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let base = format!("{PREFIXE_COPIE}{tid}-{cle}");
    let mut tmp = parent.join(&base);
    let mut n = 2;
    while tmp.exists() {
        if tmp.join(MARQUE_COPIE).is_file() {
            std::fs::remove_dir_all(&tmp)?;
            break;
        }
        tmp = parent.join(format!("{base}-{n}"));
        n += 1;
    }
    let l = fichiers_du_dossier(source).map_err(|e| Erreur::Disque(format!("mod rangé illisible ({e})")))?;
    std::fs::create_dir_all(&tmp)?;
    std::fs::write(tmp.join(MARQUE_COPIE), b"")?;
    let copie = (|| -> Resultat<()> {
        for f in &l {
            let s = source.join(f.replace('/', "\\"));
            let c = tmp.join(f.replace('/', "\\"));
            if let Some(p) = c.parent() {
                std::fs::create_dir_all(p)?;
            }
            std::fs::copy(&s, &c)?;
            if std::fs::metadata(&c)?.len() != std::fs::metadata(&s)?.len() {
                return Err(Erreur::Disque("la copie d'un mod est incomplète".into()));
            }
        }
        Ok(())
    })();
    if let Err(e) = copie {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(e);
    }
    std::fs::remove_file(tmp.join(MARQUE_COPIE))?;
    Ok(tmp)
}

/// Un mod à mettre d'accord dans un émulateur.
#[derive(Debug, Clone)]
pub struct Voulu {
    pub m: ManifesteMod,
    /// Le dossier rangé (`<stock>\<clé>`).
    pub source: PathBuf,
    /// Allumé, compatible, sans réglage requis ni conflit : à poser. Sinon, à retirer s'il était posé (et intact).
    pub poser: bool,
    /// Un mod réservé (`ManifesteMod::reserve`) : son nom n'est jamais dit au journal ni dans les raisons communes
    /// (seulement dans `BilanMods::reservees`, montrées au profil qui le voit).
    pub reserve: bool,
    /// Réservé ET absent de la dernière liste du profil qui lance : jamais posé pour lui.
    pub masque: bool,
    /// L'émulateur est commun à la maison (Eden) : un mod réservé y prend un nom de dossier NEUTRE (« Mod <clé> »), et
    /// un mod masqué y est COUPÉ (`[DisabledAddOns]`) plutôt que retiré (ni recopie à chaque alternance de profils, ni
    /// nom visible).
    pub commun: bool,
}

/// Le nom à dire d'un mod réservé (jamais son vrai nom au journal).
pub const MOD_RESERVE: &str = "un mod réservé";
/// Le nom d'un mod servi tel qu'on peut le dire dans ce qui est commun à tous les profils (suivi, journal) : seulement
/// si son public est « tous » ou « ado » — et jamais si sa version rangée a déjà été « adulte » (`adulte_vu`, une
/// révision servie avec `public: null` d'un mod classé « adulte »).
pub fn nom_a_dire(m: &ModServi, rangs: &[ManifesteMod]) -> String {
    let deja_adulte = rangs.iter().any(|r| r.cle == m.cle && (r.adulte_vu || r.reserve()));
    if public_pour_tous(m.public.as_deref()) && !deja_adulte {
        m.nom.clone()
    } else {
        MOD_RESERVE.into()
    }
}

/// Le nom d'un mod rangé tel qu'on peut le dire dans ce qui est commun (journal) : jamais celui d'un mod réservé.
pub fn nom_range_a_dire(m: &ManifesteMod) -> String {
    if m.a_taire() {
        MOD_RESERVE.into()
    } else {
        m.nom.clone()
    }
}

/// Le nom à dire d'un mod qui n'est plus rangé (on ne sait plus s'il était réservé : jamais nommé non plus).
pub const MOD_RETIRE: &str = "un mod retiré de ce PC";

/// L'état d'un mod de Frogtend dans un émulateur après la mise d'accord.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EtatPose {
    pub cle: String,
    /// Le nom du dossier dans l'émulateur.
    pub dossier: String,
    /// Le chemin complet du dossier (pour mods.json de Ryubing).
    pub chemin: PathBuf,
    /// L'émulateur doit-il l'utiliser ?
    pub actif: bool,
}

/// Le bilan de la mise d'accord d'un émulateur.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct BilanMods {
    pub poses: Vec<String>,
    pub retires: Vec<String>,
    /// Les dossiers de Frogtend encore là (posés, ou gardés parce que modifiés), d'après la note.
    pub presents: Vec<EtatPose>,
    /// Les noms des dossiers partis à ce lancement (pour défaire leur mention dans l'émulateur).
    pub partis: Vec<(String, PathBuf)>,
    /// Ce qui n'a pas été fait, et pourquoi (en mots simples : journal et fiche).
    pub raisons: Vec<String>,
    /// Rien n'a été changé dans les dossiers (abri impossible, place insuffisante, note illisible) : la pose d'avant
    /// reste ; l'émulateur n'est pas touché non plus.
    pub inchange: bool,
    /// Les parties ont été mises à l'abri avant de changer les mods (dit au journal).
    pub abrite: bool,
    /// Les raisons qui nomment un mod RÉSERVÉ (clé, raison) : jamais au journal, sur la fiche du seul profil qui le voit.
    pub reservees: Vec<(String, String)>,
}

impl BilanMods {
    pub fn resume(&self) -> String {
        let mut l = Vec::new();
        if self.abrite {
            l.push("parties mises à l'abri avant".to_string());
        }
        if !self.poses.is_empty() {
            l.push(format!("posé(s) : {}", self.poses.join(", ")));
        }
        if !self.retires.is_empty() {
            l.push(format!("retiré(s) : {}", self.retires.join(", ")));
        }
        let actifs = self.presents.iter().filter(|p| p.actif).count();
        l.push(format!("{actifs} mod(s) de Frogtend utilisé(s)"));
        l.extend(self.raisons.iter().cloned());
        l.join(" ; ")
    }
}

/// Une étape du plan de mise d'accord.
#[derive(Debug, Clone, PartialEq)]
pub enum Etape {
    /// Copier ce mod sous ce nom (nouveau, ou dossier disparu).
    Poser { cle: String, nom: String },
    /// Remplacer le dossier intact (autre révision, autre empreinte).
    Remplacer { cle: String, nom: String },
    /// Retirer le dossier intact (éteint, grisé, en conflit, retiré du rangement).
    Retirer { cle: String, nom: String },
    /// Un dossier modifié gardé : l'émulateur l'utilise désormais (ou plus).
    Basculer { cle: String, actif: bool },
    /// La note parle d'un dossier qui n'est plus là : la ligne part.
    Oublier { cle: String, nom: String },
    /// Un mod réservé posé AVANT les noms neutres sous son vrai nom, dans un émulateur commun : son dossier intact est
    /// renommé en « Mod <clé> » (même contenu : rien à recopier), la note et les coupures suivent.
    Renommer { cle: String, ancien: String, nouveau: String },
}

/// Le nom de dossier neutre d'un mod réservé dans un émulateur commun.
pub fn nom_neutre(cle: &str) -> String {
    format!("Mod {cle}")
}

/// Ce nom de dossier est-il déjà le nom neutre de ce mod (« Mod <clé> », ou « Mod <clé> (2) ») ?
fn est_nom_neutre(nom: &str, cle: &str) -> bool {
    let n = nom_neutre(cle);
    nom == n || nom.strip_prefix(&n).is_some_and(|reste| reste.starts_with(" (") && reste.ends_with(')'))
}

/// Le plan de mise d'accord d'un dossier de mods (rien n'est écrit).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Plan {
    pub etapes: Vec<Etape>,
    /// Les dossiers modifiés par la personne, gardés : (clé, raison).
    pub gardes: Vec<(String, String)>,
    /// Ce qu'il faudra copier, en octets.
    pub a_copier: u64,
}

impl Plan {
    /// Le plan change-t-il ce que l'émulateur chargera (parties à mettre à l'abri d'abord) ?
    pub fn change(&self) -> bool {
        self.etapes.iter().any(|e| !matches!(e, Etape::Oublier { .. }))
    }
}

/// Calcule le plan : ce qu'il faut poser, remplacer, retirer, basculer — sans rien écrire.
pub fn planifier(dossier_jeu: &Path, voulus: &[Voulu], note: &BTreeMap<String, PoseNotee>) -> Plan {
    let mut p = Plan::default();
    let a_poser: Vec<&Voulu> = voulus.iter().filter(|v| v.poser).collect();
    let mut noms_pris: Vec<String> = note.values().map(|n| n.dossier.to_lowercase()).collect();
    // Un mod réservé dans un émulateur commun, posé sous son VRAI nom (avant les noms neutres) : renommé, s'il est
    // intact (un dossier modifié par la personne n'est jamais touché). Rend le nom à utiliser ensuite.
    let renommer = |p: &mut Plan, v: &Voulu, n: &PoseNotee, noms_pris: &mut Vec<String>| -> String {
        let d = dossier_jeu.join(&n.dossier);
        if !(v.commun && v.reserve) || est_nom_neutre(&n.dossier, &v.m.cle) || !d.exists() || !pose_intacte(&d, n) {
            return n.dossier.clone();
        }
        let nouveau = nom_libre(&nom_neutre(&v.m.cle), &|x: &str| dossier_jeu.join(x).exists() || noms_pris.contains(&x.to_lowercase()));
        noms_pris.push(nouveau.to_lowercase());
        p.etapes.push(Etape::Renommer { cle: v.m.cle.clone(), ancien: n.dossier.clone(), nouveau: nouveau.clone() });
        nouveau
    };
    for (cle, n) in note {
        if a_poser.iter().any(|v| v.m.cle == *cle) {
            continue;
        }
        let d = dossier_jeu.join(&n.dossier);
        // Masqué pour ce profil dans un émulateur commun : coupé, jamais retiré ni recopié (son dossier, intact ou
        // modifié, reste à sa place pour le profil qui le voit).
        if let Some(v) = voulus.iter().find(|v| v.m.cle == *cle && v.masque && v.commun) {
            renommer(&mut p, v, n, &mut noms_pris);
            if d.exists() && n.actif {
                p.etapes.push(Etape::Basculer { cle: cle.clone(), actif: false });
            }
            continue;
        }
        if !d.exists() {
            p.etapes.push(Etape::Oublier { cle: cle.clone(), nom: n.dossier.clone() });
        } else if pose_intacte(&d, n) {
            p.etapes.push(Etape::Retirer { cle: cle.clone(), nom: n.dossier.clone() });
        } else {
            p.gardes.push((cle.clone(), format!("le dossier « {} » a été modifié : Frogtend le garde (coupé dans l'émulateur)", n.dossier)));
            if n.actif {
                p.etapes.push(Etape::Basculer { cle: cle.clone(), actif: false });
            }
        }
    }
    for v in a_poser {
        let taille: u64 = v.m.fichiers.iter().map(|f| f.taille).sum();
        match note.get(&v.m.cle) {
            Some(n) => {
                let nom = renommer(&mut p, v, n, &mut noms_pris);
                let d = dossier_jeu.join(&n.dossier);
                if !d.exists() {
                    p.etapes.push(Etape::Poser { cle: v.m.cle.clone(), nom: n.dossier.clone() });
                    p.a_copier = p.a_copier.saturating_add(taille);
                } else if !pose_intacte(&d, n) {
                    p.gardes.push((v.m.cle.clone(), format!("le dossier « {} » a été modifié : Frogtend ne l'écrase pas", n.dossier)));
                    if !n.actif {
                        p.etapes.push(Etape::Basculer { cle: v.m.cle.clone(), actif: true });
                    }
                } else if n.revision != v.m.revision || !n.sha256.eq_ignore_ascii_case(&v.m.sha256) {
                    p.etapes.push(Etape::Remplacer { cle: v.m.cle.clone(), nom });
                    p.a_copier = p.a_copier.saturating_add(taille);
                } else if !n.actif {
                    p.etapes.push(Etape::Basculer { cle: v.m.cle.clone(), actif: true });
                }
            }
            None => {
                // Un mod réservé dans un émulateur commun : un nom NEUTRE, jamais tiré de son vrai nom.
                let voulu = if v.commun && v.reserve { nom_neutre(&v.m.cle) } else { nom_de_dossier(&v.m.nom, &v.m.cle) };
                let nom = nom_libre(&voulu, &|x: &str| dossier_jeu.join(x).exists() || noms_pris.contains(&x.to_lowercase()));
                noms_pris.push(nom.to_lowercase());
                p.etapes.push(Etape::Poser { cle: v.m.cle.clone(), nom });
                p.a_copier = p.a_copier.saturating_add(taille);
            }
        }
    }
    p
}

/// La place libre là où le dossier `d` sera créé (son premier parent existant).
fn place_pour(d: &Path) -> Option<u64> {
    d.ancestors().find(|a| a.is_dir()).and_then(crate::jeux_pc::place_libre)
}

fn presents(dossier_jeu: &Path, note: &BTreeMap<String, PoseNotee>) -> Vec<EtatPose> {
    note.iter()
        .map(|(c, n)| EtatPose { cle: c.clone(), dossier: n.dossier.clone(), chemin: dossier_jeu.join(&n.dossier), actif: n.actif })
        .filter(|e| e.chemin.exists())
        .collect()
}

/// Met le dossier des mods d'UN jeu dans un émulateur (`dossier_jeu` : `…\contents\<tid>` ou `<load>\<TID>`) d'accord
/// avec `voulus`. Le PLAN est calculé d'abord ; s'il change quelque chose (ou si `abri_demande`), les parties sont mises
/// à l'abri (`abriter`) AVANT d'écrire — impossible : rien ne change (la pose d'avant reste). La place est vérifiée
/// avant de copier. La note est écrite après CHAQUE pose ou retrait. Un dossier de Frogtend modifié par la personne
/// n'est jamais écrasé ni effacé ; un dossier de la personne n'est jamais touché (un nom pris reçoit un suffixe).
pub fn synchroniser(dossier_jeu: &Path, voulus: &[Voulu], abri_demande: bool, abriter: &dyn Fn() -> Resultat<()>) -> Resultat<BilanMods> {
    let mut b = BilanMods::default();
    let Some(mut note) = lire_note(dossier_jeu) else {
        b.raisons.push("la note des mods posés par Frogtend ne se lit pas : rien n'est changé".into());
        b.inchange = true;
        return Ok(b);
    };
    let plan = planifier(dossier_jeu, voulus, &note);
    // Un mod qu'on peut nommer : connu du rangement et pas réservé. Sinon (réservé, ou retiré du rangement) son nom ne
    // va jamais au journal : « un mod réservé », et ses raisons à part (`reservees`).
    let nomme = |cle: &str| voulus.iter().any(|v| v.m.cle == cle && !v.reserve);
    let etiquette = |cle: &str, nom: &str| -> String {
        if nomme(cle) {
            nom.to_string()
        } else if voulus.iter().any(|v| v.m.cle == cle) {
            MOD_RESERVE.into()
        } else {
            MOD_RETIRE.into()
        }
    };
    for (cle, r) in &plan.gardes {
        if nomme(cle) {
            b.raisons.push(r.clone());
        } else {
            b.reservees.push((cle.clone(), r.clone()));
        }
    }
    if plan.change() || abri_demande {
        if let Err(e) = abriter() {
            b.raisons.push(format!("tes parties n'ont pas pu être mises à l'abri ({e}) : les mods ne sont pas changés, la partie se lance avec ceux d'avant"));
            b.inchange = true;
            b.presents = presents(dossier_jeu, &note);
            return Ok(b);
        }
        b.abrite = true;
    }
    if plan.a_copier > 0 && place_pour(dossier_jeu).is_some_and(|l| l < plan.a_copier.saturating_add(MARGE)) {
        b.raisons.push("il n'y a pas assez de place sur le disque de l'émulateur pour poser les mods : rien n'est changé".into());
        b.inchange = true;
        b.presents = presents(dossier_jeu, &note);
        return Ok(b);
    }
    for e in plan.etapes {
        match e {
            Etape::Oublier { cle, nom } => {
                note.remove(&cle);
                b.partis.push((nom.clone(), dossier_jeu.join(&nom)));
                ecrire_note(dossier_jeu, &note)?;
            }
            Etape::Retirer { cle, nom } => {
                let d = dossier_jeu.join(&nom);
                std::fs::remove_dir_all(&d)?;
                note.remove(&cle);
                ecrire_note(dossier_jeu, &note)?;
                b.retires.push(etiquette(&cle, &nom));
                b.partis.push((nom, d));
            }
            Etape::Renommer { cle, ancien, nouveau } => {
                let (de, vers) = (dossier_jeu.join(&ancien), dossier_jeu.join(&nouveau));
                std::fs::rename(&de, &vers)?;
                if let Some(n) = note.get_mut(&cle) {
                    n.dossier = nouveau;
                    // La date et la taille de chaque fichier ne changent pas avec un renommage ; relu quand même.
                    if let Ok(r) = releve(&vers) {
                        n.fichiers = r;
                    }
                }
                ecrire_note(dossier_jeu, &note)?;
                // L'ancien nom sort de l'émulateur (mods.json, `[DisabledAddOns]`).
                b.partis.push((ancien, de));
            }
            Etape::Basculer { cle, actif } => {
                if let Some(n) = note.get_mut(&cle) {
                    n.actif = actif;
                }
                ecrire_note(dossier_jeu, &note)?;
            }
            Etape::Poser { cle, nom } | Etape::Remplacer { cle, nom } => {
                let Some(v) = voulus.iter().find(|v| v.m.cle == cle) else { continue };
                std::fs::create_dir_all(dossier_jeu)?;
                let tmp = match copier_mod(&v.source, dossier_jeu, &cle) {
                    Ok(t) => t,
                    Err(e) => {
                        let r = format!("« {} » pas posé ({e})", v.m.nom);
                        if v.reserve {
                            b.reservees.push((cle.clone(), r));
                        } else {
                            b.raisons.push(r);
                        }
                        continue;
                    }
                };
                let d = dossier_jeu.join(&nom);
                // Remplacer : le dossier est intact (vérifié par le plan) ; la note le dit encore à Frogtend. Un échec
                // en route : la copie provisoire (à Frogtend) part, la note reste juste pour ce qui est fait.
                let place = (|| -> std::io::Result<()> {
                    if d.exists() {
                        std::fs::remove_dir_all(&d)?;
                    }
                    std::fs::rename(&tmp, &d)
                })();
                if let Err(e) = place {
                    let _ = std::fs::remove_dir_all(&tmp);
                    if !d.exists() {
                        note.remove(&cle);
                        let _ = ecrire_note(dossier_jeu, &note);
                    }
                    return Err(e.into());
                }
                let r = releve(&d)?;
                note.insert(cle.clone(), PoseNotee { dossier: nom.clone(), revision: v.m.revision, sha256: v.m.sha256.clone(), actif: true, fichiers: r });
                ecrire_note(dossier_jeu, &note)?;
                b.poses.push(etiquette(&cle, &nom));
            }
        }
    }
    b.presents = presents(dossier_jeu, &note);
    Ok(b)
}

/// Le sous-dossier `nom` de `parent` tel qu'il est écrit sur le disque (sans casse), sinon `nom` tel quel.
fn sous_dossier_existant(parent: &Path, nom: &str) -> PathBuf {
    std::fs::read_dir(parent)
        .into_iter()
        .flatten()
        .flatten()
        .find(|e| e.file_type().is_ok_and(|t| t.is_dir()) && e.file_name().to_string_lossy().eq_ignore_ascii_case(nom))
        .map(|e| e.path())
        .unwrap_or_else(|| parent.join(nom))
}

/// Les mods voulus d'un jeu, d'après le rangement (`stock`), le choix de la maison et la mise à jour utilisée. Les
/// raisons de ne pas poser (grisé, réglage requis, conflit avec un mod allumé) vont dans `raisons`.
pub fn voulus(stock: &Path, choix: &crate::contenus_switch::ChoixJeu, utilisee: Option<u32>, raisons: &mut Vec<String>) -> Vec<Voulu> {
    voulus_pour(stock, choix, utilisee, None, raisons, &mut Vec::new())
}

/// `voulus` pour le profil dont `liste` est la dernière liste reçue : un mod réservé qu'elle ne contient pas n'est
/// JAMAIS posé (retiré de l'émulateur s'il y était, comme un mod éteint), jamais nommé, et ne compte pour aucun conflit.
/// Les raisons qui nomment un mod réservé visible vont dans `reservees` (clé, raison), jamais dans `raisons`.
pub fn voulus_pour(
    stock: &Path,
    choix: &crate::contenus_switch::ChoixJeu,
    utilisee: Option<u32>,
    liste: Option<&ListeMods>,
    raisons: &mut Vec<String>,
    reservees: &mut Vec<(String, String)>,
) -> Vec<Voulu> {
    let tous = ranges(stock);
    let vus = visibles(tous.clone(), liste);
    tous.iter()
        .map(|m| {
            let visible = vus.iter().any(|v| v.cle == m.cle);
            let allume = visible && choix.mod_actif(&m.cle);
            let c = compatibilite(&m.versions, m.compatible, m.raison.as_deref(), m.version_servie.as_ref(), utilisee);
            let mut dire = |cle: &str, reserve: bool, t: String| {
                if reserve {
                    reservees.push((cle.to_string(), t));
                } else {
                    raisons.push(t);
                }
            };
            let mut poser = allume;
            if allume && c.grise() {
                dire(&m.cle, m.a_taire(), format!("« {} » pas posé : {}", m.nom, c.raison.clone().unwrap_or_default()));
                poser = false;
            }
            if allume && !m.reglage_requis.is_empty() {
                dire(&m.cle, m.a_taire(), format!("« {} » pas posé : réglage d'émulateur requis : {}", m.nom, m.reglage_requis));
                poser = false;
            }
            if allume {
                if let Some(b) = vus.iter().find(|b| b.cle != m.cle && choix.mod_actif(&b.cle) && (m.conflits.contains(&b.cle) || b.conflits.contains(&m.cle))) {
                    dire(&m.cle, m.a_taire() || b.a_taire(), format!("« {} » pas posé : il ne va pas avec « {} », allumé aussi (éteins l'un des deux)", m.nom, b.nom));
                    poser = false;
                }
            }
            Voulu { source: stock.join(&m.cle), m: m.clone(), poser, reserve: m.a_taire(), masque: !visible, commun: false }
        })
        .collect()
}

/// Ce que la dernière pose a dit (`<rangement>\.derniere-pose.json`), montré sur la fiche.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DernierePose {
    pub quand: String,
    pub emulateur: String,
    pub raisons: Vec<String>,
    /// Les raisons qui nomment un mod RÉSERVÉ (clé, raison) : montrées seulement au profil qui voit ce mod (ajoutées
    /// alors à `raisons` par `pour_le_profil`), jamais envoyées telles quelles à l'interface.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reservees: Vec<(String, String)>,
}

impl DernierePose {
    /// Ce que CE profil peut lire de la dernière pose (`liste` : sa dernière liste reçue).
    pub fn pour_le_profil(mut self, liste: Option<&ListeMods>) -> Self {
        for (cle, r) in std::mem::take(&mut self.reservees) {
            if liste.is_some_and(|l| l.mods.iter().any(|m| m.cle == cle)) {
                self.raisons.push(r);
            }
        }
        self
    }
}

const FICHIER_DERNIERE_POSE: &str = ".derniere-pose.json";

/// Note ce que la pose a dit (seulement si le rangement existe : rien n'est créé pour un jeu sans mods).
pub fn noter_derniere_pose(stock: &Path, emulateur: &str, raisons: &[String]) {
    noter_derniere_pose_avec(stock, emulateur, raisons, &[]);
}

/// `noter_derniere_pose`, avec les raisons qui nomment un mod réservé (gardées à part, par clé).
pub fn noter_derniere_pose_avec(stock: &Path, emulateur: &str, raisons: &[String], reservees: &[(String, String)]) {
    if !stock.is_dir() {
        return;
    }
    let d = DernierePose { quand: crate::noyau::maintenant(), emulateur: emulateur.into(), raisons: raisons.to_vec(), reservees: reservees.to_vec() };
    if let Ok(o) = serde_json::to_vec_pretty(&d) {
        let tmp = stock.join(format!("{FICHIER_DERNIERE_POSE}.tmp"));
        if std::fs::write(&tmp, o).is_ok() {
            let _ = std::fs::rename(&tmp, stock.join(FICHIER_DERNIERE_POSE));
        }
    }
}

/// Le title id de BASE d'un jeu d'après son fichier (nom « [0100…] » ou ticket du .nsp), sans clés : quand les mises
/// à jour et DLC n'ont pas pu être lus.
pub fn base_du_fichier(fichier: &Path) -> Option<u64> {
    crate::contenus::id_switch(fichier).map(crate::switch_nca::base)
}

/// Ce que dit la fiche quand le jeu n'a pas pu être reconnu au lancement.
pub const RAISON_SANS_TITRE: &str = "Frogtend n'a pas reconnu ce jeu à son lancement : ses mods n'ont pas été remis à jour dans l'émulateur (ceux d'avant restent en place).";

/// Les rangements de mods d'un jeu (`frogtend` : son dossier Frogtend) : (title id de base, dossier).
pub fn rangements_du_jeu(frogtend: &Path) -> Vec<(u64, PathBuf)> {
    std::fs::read_dir(frogtend.join(DOSSIER_MODS))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| {
            let nom = e.file_name().to_string_lossy().to_string();
            (nom.len() == 16).then(|| u64::from_str_radix(&nom, 16).ok()).flatten().map(|b| (b, e.path()))
        })
        .collect()
}

/// Note `raison` comme dernière pose dans CHAQUE rangement de mods du jeu (`frogtend` : son dossier Frogtend) — quand
/// on ne sait pas lequel est le bon. Rend combien.
pub fn noter_raison_partout(frogtend: &Path, emulateur: &str, raison: &str) -> usize {
    let mut n = 0;
    for e in std::fs::read_dir(frogtend.join(DOSSIER_MODS)).into_iter().flatten().flatten() {
        let nom = e.file_name().to_string_lossy().to_string();
        if e.file_type().is_ok_and(|t| t.is_dir()) && nom.len() == 16 && nom.chars().all(|c| c.is_ascii_hexdigit()) {
            noter_derniere_pose(&e.path(), emulateur, &[raison.to_string()]);
            n += 1;
        }
    }
    n
}

pub fn derniere_pose(stock: &Path) -> Option<DernierePose> {
    let f = stock.join(FICHIER_DERNIERE_POSE);
    if std::fs::metadata(&f).ok()?.len() > 1 << 20 {
        return None;
    }
    serde_json::from_slice(&std::fs::read(f).ok()?).ok()
}

/// Avant une partie Ryubing (après `poser_contenus`) : met les mods de Frogtend d'accord dans la racine de la personne
/// (`<racine>\mods\contents\<tid>\<nom>\`, copiés) et dans `games\<tid>\mods.json` (les entrées de la personne
/// gardées). Ryubing non portable ou ouvert : rien (raison). Parties mises à l'abri avant tout changement.
pub fn poser_ryubing(donnees: &Path, ryubing: &Path, racine: &Path, base: u64, stock: &Path, utilisee: Option<u32>) -> Resultat<BilanMods> {
    poser_ryubing_pour(donnees, ryubing, racine, base, stock, utilisee, None)
}

/// `poser_ryubing` pour le profil qui lance (`liste` : SA dernière liste reçue, `liste_du_profil`) : un mod réservé
/// qu'elle ne contient pas n'est jamais posé (retiré s'il l'était), ni nommé.
pub fn poser_ryubing_pour(donnees: &Path, ryubing: &Path, racine: &Path, base: u64, stock: &Path, utilisee: Option<u32>, liste: Option<&ListeMods>) -> Resultat<BilanMods> {
    let r = poser_ryubing_dans(donnees, ryubing, racine, base, stock, utilisee, liste);
    if let Ok(b) = &r {
        noter_derniere_pose_avec(stock, "Ryubing", &b.raisons, &b.reservees);
    }
    r
}

fn poser_ryubing_dans(donnees: &Path, ryubing: &Path, racine: &Path, base: u64, stock: &Path, utilisee: Option<u32>, liste: Option<&ListeMods>) -> Resultat<BilanMods> {
    let mut b = BilanMods::default();
    // Le chemin du mod dans mods.json doit être la chaîne EXACTE que Ryubing compose : `GetFullPath(-r)` + « \mods »
    // (texte de Ryubing) + les noms de `contents` et du dossier du jeu TELS QU'ILS SONT SUR LE DISQUE (Ryubing les
    // obtient par `EnumerateDirectories`, ModLoader.cs `CollectMods`/`FindApplicationDir`) + le nom du mod.
    let racine = PathBuf::from(racine.to_string_lossy().replace('/', "\\"));
    let contents = sous_dossier_existant(&racine.join("mods"), "contents");
    let dossier_jeu = sous_dossier_existant(&contents, &format!("{base:016x}"));
    if !stock.is_dir() && !dossier_jeu.join(NOTE_POSE).is_file() {
        return Ok(b);
    }
    if !crate::ryubing::portable(ryubing) {
        b.raisons.push("Ryubing n'a pas été installé par Frogtend : les mods n'y sont pas posés".into());
        return Ok(b);
    }
    if !crate::lancement::processus_du_dossier(ryubing).is_empty() {
        b.raisons.push("Ryubing est ouvert : les mods ne sont pas changés".into());
        return Ok(b);
    }
    let choix = crate::contenus_switch::charger(donnees).0.jeu(base);
    let lieu = crate::contenus_switch::lieu("ryubing", &racine);
    let (mut raisons, mut reservees) = (Vec::new(), Vec::new());
    let v = voulus_pour(stock, &choix, utilisee, liste, &mut raisons, &mut reservees);
    let abriter = || -> Resultat<()> {
        crate::ryubing::abriter_parties(ryubing, &racine, &format!("{base:016X}"))?;
        let _ = crate::contenus_switch::noter_abri_fait(donnees, base, &lieu);
        Ok(())
    };
    let mut r = synchroniser(&dossier_jeu, &v, choix.abri_a_faire(&lieu), &abriter)?;
    r.raisons.splice(0..0, raisons);
    r.reservees.splice(0..0, reservees);
    if r.inchange {
        return Ok(r);
    }
    let nos: Vec<(String, String, bool)> = r.presents.iter().map(|p| (p.dossier.clone(), p.chemin.to_string_lossy().to_string(), p.actif)).collect();
    let partis: Vec<String> = r.partis.iter().map(|(_, c)| c.to_string_lossy().to_string()).collect();
    if let Err(e) = crate::ryubing::poser_mods_json(ryubing, &racine, base, &nos, &partis) {
        r.raisons.push(format!("liste des mods de Ryubing pas mise à jour ({e})"));
    }
    Ok(r)
}

/// Avant une partie Eden : met les mods de Frogtend d'accord dans `<load>\<TID>\<nom>\` (copiés) et dans
/// `[DisabledAddOns]` (par nom de dossier ; les coupures de la personne gardées). Eden ouvert, dossier des mods réglé
/// hors d'Eden : rien (raison). Parties mises à l'abri avant tout changement. Jamais de qt-config.ini créé.
pub fn poser_eden(donnees: &Path, eden: &Path, base: u64, stock: &Path, utilisee: Option<u32>, ouvert: bool) -> Resultat<BilanMods> {
    poser_eden_pour(donnees, eden, base, stock, utilisee, ouvert, None)
}

/// `poser_eden` pour le profil qui lance (`liste` : SA dernière liste reçue) : un mod réservé qu'elle ne contient pas
/// n'est jamais posé (retiré s'il l'était — Eden est commun à la maison), ni nommé.
pub fn poser_eden_pour(donnees: &Path, eden: &Path, base: u64, stock: &Path, utilisee: Option<u32>, ouvert: bool, liste: Option<&ListeMods>) -> Resultat<BilanMods> {
    let r = poser_eden_dans(donnees, eden, base, stock, utilisee, ouvert, liste);
    if let Ok(b) = &r {
        noter_derniere_pose_avec(stock, "Eden", &b.raisons, &b.reservees);
    }
    r
}

fn poser_eden_dans(donnees: &Path, eden: &Path, base: u64, stock: &Path, utilisee: Option<u32>, ouvert: bool, liste: Option<&ListeMods>) -> Resultat<BilanMods> {
    let mut b = BilanMods::default();
    let Some(load) = crate::eden::dossier_load(eden) else {
        b.raisons.push("le dossier des mods d'Eden est réglé hors du dossier d'Eden : les mods n'y sont pas posés".into());
        return Ok(b);
    };
    let dossier_jeu = sous_dossier_existant(&load, &format!("{base:016X}"));
    if !stock.is_dir() && !dossier_jeu.join(NOTE_POSE).is_file() {
        return Ok(b);
    }
    if ouvert {
        b.raisons.push("Eden est ouvert : les mods ne sont pas changés".into());
        return Ok(b);
    }
    let ini = crate::contenus::ini_eden(eden);
    // Jamais de qt-config.ini créé (ni réduit à `[DisabledAddOns]`) : sans lui, rien n'est coupé.
    let texte = match std::fs::read_to_string(&ini) {
        Ok(t) if !t.trim().is_empty() => Some(t),
        Ok(_) => None,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };
    if texte.as_deref().is_some_and(|t| !crate::eden::coupures_lisibles(t)) {
        b.raisons.push("la section [DisabledAddOns] d'Eden ne se lit pas comme prévu : les mods ne sont pas changés".into());
        return Ok(b);
    }
    let choix = crate::contenus_switch::charger(donnees).0.jeu(base);
    let lieu = crate::contenus_switch::lieu("eden", eden);
    let (mut raisons, mut reservees) = (Vec::new(), Vec::new());
    let mut v = voulus_pour(stock, &choix, utilisee, liste, &mut raisons, &mut reservees);
    // Eden est commun à la maison : noms neutres pour un mod réservé, et un mod masqué est coupé, pas retiré.
    for x in &mut v {
        x.commun = true;
    }
    let abriter = || -> Resultat<()> {
        crate::eden::abriter_parties_du_jeu(eden, base)?;
        let _ = crate::contenus_switch::noter_abri_fait(donnees, base, &lieu);
        Ok(())
    };
    let mut r = synchroniser(&dossier_jeu, &v, choix.abri_a_faire(&lieu), &abriter)?;
    r.raisons.splice(0..0, raisons);
    r.reservees.splice(0..0, reservees);
    if r.inchange {
        return Ok(r);
    }
    // Les coupures : nos dossiers (allumés ou gardés éteints), et les noms de nos dossiers partis (rallumés : le nom
    // ne désigne plus rien de la personne).
    let mut mods: Vec<(String, bool)> = r.presents.iter().map(|p| (p.dossier.clone(), p.actif)).collect();
    mods.extend(r.partis.iter().map(|(n, _)| (n.clone(), true)));
    let Some(texte) = texte else {
        if r.presents.iter().any(|p| !p.actif) {
            r.raisons.push("Eden n'a pas encore de fichier de réglages : un mod gardé ne peut pas y être coupé".into());
        }
        return Ok(r);
    };
    let texte = std::fs::read_to_string(&ini).unwrap_or(texte);
    if let Some(apres) = crate::eden::texte_avec_coupures_et_mods(&texte, base, None, (0, 0), &mods) {
        crate::emulateurs_profils::remplacer_config(eden, &ini, &apres)?;
        let relu = std::fs::read_to_string(&ini).unwrap_or_default();
        if crate::eden::texte_avec_coupures_et_mods(&relu, base, None, (0, 0), &mods).is_some() {
            r.raisons.push("les mods coupés n'ont pas été enregistrés dans Eden".into());
        } else {
            let cache = crate::eden::cache_liste_du_jeu(eden, base);
            if cache.is_file() {
                let _ = std::fs::remove_file(cache);
            }
        }
    }
    Ok(r)
}

// --- Le contrôle avant de lancer : aucun mod d'un autre profil ne doit rester chargé ---

/// Le message (neutre : jamais le nom du mod) quand un mod réservé d'un autre profil reste chargé.
pub fn message_masque_en_place(emulateur: &str) -> String {
    format!(
        "Un mod d'un autre profil est encore en place dans {emulateur} : Frogtend n'a pas pu le couper (émulateur ouvert, parties pas mises à l'abri, place ou réglages manquants). La partie ne se lance pas ; ferme {emulateur} puis réessaie, ou lance-la depuis le profil qui l'a installé."
    )
}

/// Les clés des mods rangés que ce profil ne voit pas (`liste` : sa dernière liste reçue).
fn cles_masquees(stock: &Path, liste: Option<&ListeMods>) -> Vec<String> {
    let tous = ranges(stock);
    let vus = visibles(tous.clone(), liste);
    tous.into_iter().filter(|m| !vus.iter().any(|v| v.cle == m.cle)).map(|m| m.cle).collect()
}

/// Un mod masqué (`masques`) est-il encore CHARGÉ dans le dossier des mods d'un jeu (`coupe` : ce dossier est-il coupé
/// dans l'émulateur ?) ? Une note illisible alors qu'il y a des mods masqués : dans le doute, oui.
fn masque_charge(dossier_jeu: &Path, masques: &[String], coupe: &dyn Fn(&str) -> bool, emulateur: &str) -> Option<String> {
    if masques.is_empty() {
        return None;
    }
    let Some(note) = lire_note(dossier_jeu) else { return dossier_jeu.exists().then(|| message_note_illisible(emulateur)) };
    note.iter()
        .any(|(cle, n)| masques.contains(cle) && dossier_jeu.join(&n.dossier).exists() && !coupe(&n.dossier))
        .then(|| message_masque_en_place(emulateur))
}

/// Le message quand la note des mods posés ne se lit plus (dans le doute, la partie ne part pas).
pub fn message_note_illisible(emulateur: &str) -> String {
    format!(
        "Frogtend ne sait plus quels mods sont en place pour ce jeu dans {emulateur} (sa note est illisible) : par prudence, la partie ne se lance pas. Lance le jeu depuis le profil qui a installé ses mods, ou demande de l'aide."
    )
}

/// APRÈS la pose Ryubing (quelle qu'en soit l'issue) : un mod qu'un autre profil a reçu est-il encore chargé dans la
/// racine de la personne qui lance (`liste` : SA dernière liste) ? `Some(message)` : la partie ne doit pas partir.
pub fn verifier_masques_ryubing(racine: &Path, base: u64, stock: &Path, liste: Option<&ListeMods>) -> Option<String> {
    let racine = PathBuf::from(racine.to_string_lossy().replace('/', "\\"));
    let contents = sous_dossier_existant(&racine.join("mods"), "contents");
    let dossier_jeu = sous_dossier_existant(&contents, &format!("{base:016x}"));
    let masques = cles_masquees(stock, liste);
    let mj: Option<Value> = std::fs::read(crate::ryubing::fichier_mods_json(&racine, base)).ok().and_then(|o| serde_json::from_slice(&o).ok());
    let coupe = |nom: &str| {
        let chemin = dossier_jeu.join(nom).to_string_lossy().to_string();
        mj.as_ref().is_some_and(|v| v["mods"].as_array().into_iter().flatten().any(|e| e["path"].as_str() == Some(chemin.as_str()) && e["enabled"] == false))
    };
    masque_charge(&dossier_jeu, &masques, &coupe, "Ryubing")
}

/// APRÈS la pose Eden (quelle qu'en soit l'issue : Eden ouvert, abri impossible, place, erreur) : un mod qu'un autre
/// profil a reçu est-il encore chargé dans Eden, commun à la maison (pas coupé par `[DisabledAddOns]` ; sans
/// qt-config.ini, rien ne peut l'être) ? `Some(message)` : la partie ne doit pas partir.
pub fn verifier_masques_eden(eden: &Path, base: u64, stock: &Path, liste: Option<&ListeMods>) -> Option<String> {
    let Some(load) = crate::eden::dossier_load(eden) else { return None };
    let dossier_jeu = sous_dossier_existant(&load, &format!("{base:016X}"));
    let masques = cles_masquees(stock, liste);
    let coupes: Vec<String> = std::fs::read_to_string(crate::contenus::ini_eden(eden))
        .ok()
        .map(|t| crate::eden::lire_coupures(&t).into_iter().find(|(x, _)| *x == base).map(|(_, l)| l).unwrap_or_default())
        .unwrap_or_default();
    masque_charge(&dossier_jeu, &masques, &|nom| coupes.iter().any(|c| c == nom), "Eden")
}

// --- Le travail en arrière-plan (téléchargements) ---

/// Où en sont les téléchargements de mods d'un jeu (lu par la fiche).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Suivi {
    pub actif: bool,
    /// Le mod en cours (son nom).
    pub en_cours: Option<String>,
    pub recus: u64,
    pub total: u64,
    pub faits: usize,
    pub a_faire: usize,
    /// Les échecs, en mots simples (« nom : raison »).
    pub erreurs: Vec<String>,
    /// Pourquoi le travail s'est arrêté avant la fin (une partie a commencé…).
    pub arret: Option<String>,
}

/// Ce que dit « Jouer » pendant qu'un mod de ce jeu arrive.
pub const PARTIE_PENDANT_TELECHARGEMENT: &str =
    "Des mods de ce jeu sont en train d'arriver : la partie pourra se lancer dès qu'ils seront installés (suis leur arrivée sur la fiche du jeu).";

/// L'état partagé des téléchargements de mods, PAR JEU (le rangement des mods est commun à toute la maison : un seul
/// travail par jeu, quel que soit le profil).
#[derive(Default)]
pub struct Travail {
    pub suivis: std::sync::Mutex<BTreeMap<i64, Suivi>>,
}

impl Travail {
    fn verrou(&self) -> std::sync::MutexGuard<'_, BTreeMap<i64, Suivi>> {
        self.suivis.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Commence le travail d'un jeu ; `false` s'il tourne déjà (pour n'importe quel profil).
    pub fn commencer(&self, jeu: i64, a_faire: usize) -> bool {
        let mut v = self.verrou();
        if v.get(&jeu).is_some_and(|s| s.actif) {
            return false;
        }
        v.insert(jeu, Suivi { actif: true, a_faire, ..Default::default() });
        true
    }

    pub fn modifier(&self, jeu: i64, f: impl FnOnce(&mut Suivi)) {
        if let Some(s) = self.verrou().get_mut(&jeu) {
            f(s);
        }
    }

    pub fn suivi(&self, jeu: i64) -> Suivi {
        self.verrou().get(&jeu).cloned().unwrap_or_default()
    }

    /// Un téléchargement de mods de ce jeu est-il en cours (tous profils confondus) ?
    pub fn actif(&self, jeu: i64) -> bool {
        self.suivi(jeu).actif
    }
}

/// Les rangements de mods en cours d'écriture (téléchargement + rangement, retrait) : un seul à la fois par rangement,
/// pour tout Frogtend (jamais deux écritures du même `.part`, jamais deux `ranger` en même temps).
static STOCKS_OCCUPES: std::sync::Mutex<std::collections::BTreeSet<String>> = std::sync::Mutex::new(std::collections::BTreeSet::new());

/// La garde d'un rangement occupé (libéré à sa fin, même sur une erreur).
pub struct GardeStock(String);

impl Drop for GardeStock {
    fn drop(&mut self) {
        STOCKS_OCCUPES.lock().unwrap_or_else(|e| e.into_inner()).remove(&self.0);
    }
}

/// Occupe le rangement `stock` ; refusé s'il l'est déjà.
pub fn occuper_stock(stock: &Path) -> Resultat<GardeStock> {
    let cle = stock.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_lowercase();
    let mut o = STOCKS_OCCUPES.lock().unwrap_or_else(|e| e.into_inner());
    if !o.insert(cle.clone()) {
        return Err(Erreur::Refus("Les mods de ce jeu sont déjà en train de changer : attends la fin.".into()));
    }
    Ok(GardeStock(cle))
}

// --- Ce que voit l'interface ---

/// Un mod vu sur la fiche.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ModVu {
    pub cle: String,
    pub nom: String,
    pub description: String,
    pub auteur: String,
    pub licence: String,
    pub source: String,
    /// La révision servie (ou rangée si Firehouse ne le sert plus).
    pub revision: u64,
    pub revision_rangee: Option<u64>,
    /// « pas_telecharge », « telecharge », « revision_dispo » ou « incompatible ».
    pub etat: String,
    /// « compatible », « pas_verifie » ou « incompatible ».
    pub compat: String,
    pub raison: Option<String>,
    pub allume: bool,
    /// Taille de l'archive servie (à télécharger).
    pub taille: u64,
    /// L'empreinte de l'archive servie (ou rangée si Firehouse ne le sert plus) : rendue telle quelle dans l'accord
    /// (`ModAccorde.sha256`), pour que le cœur refuse si le contenu a changé entre la lecture et le oui.
    pub sha256: String,
    pub conflits: Vec<String>,
    pub reglage_requis: String,
    pub touche_sauvegardes: Option<bool>,
    pub contenu: ContenuMod,
    pub emulateurs_essayes: Vec<String>,
    /// Firehouse le sert-il encore (sinon : seulement rangé sur ce PC) ?
    pub servi: bool,
    /// Contrat 1.46 : la sorte du mod (`technique`, `contenu`, `cosmetique`, `triche`, `autre`) et son intérêt (1 à 5),
    /// `null` tant que Firehouse n'a pas écrit sa fiche.
    #[serde(rename = "type")]
    pub type_mod: Option<String>,
    pub interet: Option<u8>,
}

/// La liste vue sur la fiche.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ListeVue {
    /// « ok », « hors_ligne », « injoignable », ou « rien » (pas un jeu Switch lisible).
    pub etat: String,
    pub vue_le: Option<String>,
    pub tid_base: Option<String>,
    pub version_servie: Option<VersionServie>,
    /// La version brute de la mise à jour utilisée est-elle connue ?
    pub version_connue: bool,
    pub mods: Vec<ModVu>,
    pub remarque: Option<String>,
    /// Ce que la dernière pose dans un émulateur a dit (mod pas posé et pourquoi, abri impossible…), à montrer.
    pub derniere_pose: Option<DernierePose>,
}

/// Construit la liste vue : les mods servis (lus) et les mods rangés sur ce PC, avec leur état et leur compatibilité
/// avec la mise à jour `utilisee`.
pub fn liste_vue(lecture: &Lecture, rangs: &[ManifesteMod], choix: &crate::contenus_switch::ChoixJeu, utilisee: Option<u32>, tid_base: u64) -> ListeVue {
    let mut mods = Vec::new();
    for m in &lecture.liste.mods {
        let r = rangs.iter().find(|x| x.cle == m.cle);
        let c = compatibilite(&m.versions, m.compatible, m.raison.as_deref(), lecture.liste.version_servie.as_ref(), utilisee);
        let etat = match r {
            _ if c.grise() => "incompatible",
            Some(x) if x.revision < m.revision || (x.revision == m.revision && !x.sha256.eq_ignore_ascii_case(&m.fichier.sha256)) => "revision_dispo",
            Some(_) => "telecharge",
            None => "pas_telecharge",
        };
        mods.push(ModVu {
            cle: m.cle.clone(),
            nom: m.nom.clone(),
            description: m.description.clone(),
            auteur: m.auteur.clone(),
            licence: m.licence.clone(),
            source: m.source.clone(),
            revision: m.revision,
            revision_rangee: r.map(|x| x.revision),
            etat: etat.into(),
            compat: c.etat.into(),
            raison: c.raison,
            allume: r.is_some() && choix.mod_actif(&m.cle),
            taille: m.fichier.taille,
            sha256: m.fichier.sha256.clone(),
            conflits: m.conflits.clone(),
            reglage_requis: m.reglage_requis.clone(),
            touche_sauvegardes: m.touche_sauvegardes,
            contenu: m.contenu.clone(),
            emulateurs_essayes: m.emulateurs_essayes.clone(),
            servi: true,
            type_mod: m.type_mod.clone(),
            interet: m.interet,
        });
    }
    // Un mod rangé RÉSERVÉ (« adulte ») que la liste de CE profil ne contient pas : écarté, jamais montré (pas même
    // « plus servi »). Il reste rangé pour le profil qui le voit.
    let vus = visibles(rangs.to_vec(), Some(&lecture.liste));
    for r in vus.iter().filter(|r| !lecture.liste.mods.iter().any(|m| m.cle == r.cle)) {
        let c = compatibilite(&r.versions, r.compatible, r.raison.as_deref(), r.version_servie.as_ref(), utilisee);
        mods.push(ModVu {
            cle: r.cle.clone(),
            nom: r.nom.clone(),
            description: r.description.clone(),
            auteur: String::new(),
            licence: String::new(),
            source: String::new(),
            revision: r.revision,
            revision_rangee: Some(r.revision),
            etat: if c.grise() { "incompatible" } else { "telecharge" }.into(),
            compat: c.etat.into(),
            raison: c.raison,
            allume: choix.mod_actif(&r.cle),
            taille: r.taille,
            sha256: r.sha256.clone(),
            conflits: r.conflits.clone(),
            reglage_requis: r.reglage_requis.clone(),
            touche_sauvegardes: r.touche_sauvegardes,
            contenu: r.contenu.clone(),
            emulateurs_essayes: vec![],
            servi: false,
            type_mod: r.type_mod.clone(),
            interet: r.interet,
        });
    }
    // Les plus intéressants d'abord (`interet` décroissant, sans fiche en dernier), puis par nom.
    trier_par_interet(&mut mods);
    ListeVue {
        etat: lecture.etat.clone(),
        vue_le: lecture.vue_le.clone(),
        tid_base: Some(format!("{tid_base:016X}")),
        version_servie: lecture.liste.version_servie.clone(),
        version_connue: utilisee.is_some(),
        mods,
        remarque: None,
        derniere_pose: None,
    }
}

/// Trie la liste vue : `interet` décroissant (sans fiche en dernier), puis par nom (sans casse).
pub fn trier_par_interet(mods: &mut [ModVu]) {
    mods.sort_by(|a, b| b.interet.unwrap_or(0).cmp(&a.interet.unwrap_or(0)).then_with(|| a.nom.to_lowercase().cmp(&b.nom.to_lowercase())));
}

/// Allumer `cle` est-il refusé par un conflit avec un mod rangé ALLUMÉ (dans un sens ou dans l'autre) ? `sauf` : le
/// mod que la personne a accepté d'éteindre en même temps. Rend le motif lisible.
pub fn conflit(rangs: &[ManifesteMod], choix: &crate::contenus_switch::ChoixJeu, cle: &str, sauf: Option<&str>) -> Option<String> {
    let vus: Vec<String> = rangs.iter().map(|m| m.cle.clone()).collect();
    conflit_pour(rangs, &vus, choix, cle, sauf)
}

/// `conflit` vérifié sur TOUS les mods rangés (`tous`), y compris ceux que ce profil ne voit pas (`vus` : les clés
/// qu'il voit) : contre un mod qu'il ne voit pas, le motif est NEUTRE (jamais son nom).
pub fn conflit_pour(tous: &[ManifesteMod], vus: &[String], choix: &crate::contenus_switch::ChoixJeu, cle: &str, sauf: Option<&str>) -> Option<String> {
    let a = tous.iter().find(|m| m.cle == cle)?;
    tous.iter()
        .filter(|b| b.cle != cle && Some(b.cle.as_str()) != sauf && choix.mod_actif(&b.cle))
        .find(|b| a.conflits.contains(&b.cle) || b.conflits.contains(&a.cle))
        .map(|b| {
            if vus.contains(&b.cle) {
                format!("« {} » ne va pas avec « {} », déjà allumé : il faut l'éteindre pour allumer celui-ci.", a.nom, b.nom)
            } else {
                format!("« {} » ne va pas avec un autre mod allumé sur ce PC : il ne peut pas être allumé pour l'instant.", a.nom)
            }
        })
}

#[cfg(test)]
mod tests;
