//! La sauvegarde d'un profil : « tout ce qui ne se retélécharge pas » (lot 3 bis, décision de Seb).
//!
//! Dans `<partage de la personne>\<profil>\<PC>\` :
//! - `configuration.json` : réglages du PC et du profil, émulateurs installés ;
//! - `bibliotheque.json` : la LISTE des jeux (version, emplacement, installation, temps de jeu) — pas les jeux ;
//! - `parties\` : les parties et codes de triche de TOUS les jeux — pour un jeu installé, les fichiers changés depuis
//!   son installation ; pour chaque émulateur, le dossier du profil (`<émulateur>\Profils\<profil>\`) ;
//!   `parties\manifeste.json` donne l'empreinte (sha256) de chacun : seul ce qui a changé est renvoyé ;
//!   `parties\captures\<jeu>\<id>.<ext>` : les captures d'écran du profil (✅ Seb 12/11), chacune avec sa fiche à
//!   côté (`<id>.json` : jeu, titre, console, date, nom, fond choisi) ;
//! - `derniere-sauvegarde.json` : quand, depuis quel PC, combien.
//!
//! Frogtend écrit la version COURANTE ; l'historique est gardé par le serveur (instantanés ZFS).

//!
//! Lot S4 (23/11) : les abris de parties des jeux PC partent aussi (`jeux/<id>/emplacements/<clé>/…`, avec leur fiche
//! d'abord), chacun avec son profil propriétaire seulement ; un fichier en échec est SAUTÉ et compté (sa copie déjà
//! chez Firehouse reste) ; 401, 507, réseau et 503 arrêtent tout (rien d'écrit) ; la place libre est vérifiée AVANT
//! d'envoyer ; l'envoi se met en pause pendant une partie ; le manifeste passe à des documents par jeu s'il approche
//! 5 Mo ; jamais un chemin ni un titre au journal.

use crate::erreurs::{Erreur, Resultat};
use crate::installation::{fichiers_de, Manifeste};
use crate::noyau::{maintenant, Noyau};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Un fichier à sauvegarder : où il est sur le PC, où il va dans `parties\`.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub source: PathBuf,
    /// Chemin dans `parties\`, séparateur `/`.
    pub relatif: String,
}

/// Le manifeste des parties sauvegardées : empreinte et taille de chaque fichier.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ManifesteParties {
    pub fichiers: BTreeMap<String, (String, u64)>,
    /// Lot S4 : les jeux dont les lignes sont rangées à part (`manifeste-<clé>.json`, motif du contrat 1.48) parce
    /// que le manifeste approchait 5 Mo. Vide (le cas courant) : tout est dans `fichiers`, comme avant.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub manifestes_jeux: Vec<String>,
}

/// Le manifeste unique passe à des documents par jeu au-delà de cette taille (un document : 5 Mo au plus).
pub const SEUIL_MANIFESTE: usize = 4 * 1024 * 1024;
/// La taille maximale d'un document chez Firehouse (`/moi ▸ sauvegarde.document_max_octets`).
pub const DOCUMENT_MAX: usize = 5 * 1024 * 1024;

/// La clé du document à part d'un jeu (`jeux/<id>/…` → `jeu-110`, `jeu-m5` pour un jeu importé) ; `None` : pas un
/// fichier de jeu (émulateurs, captures : ils restent dans `manifeste.json`).
pub fn cle_manifeste(relatif: &str) -> Option<String> {
    let id: i64 = relatif.strip_prefix("jeux/")?.split('/').next()?.parse().ok()?;
    Some(if id < 0 { format!("jeu-m{}", id.unsigned_abs()) } else { format!("jeu-{id}") })
}

/// Le nom du document à part d'un jeu.
pub fn nom_manifeste_jeu(cle: &str) -> String {
    format!("manifeste-{cle}.json")
}

/// Le manifeste courant, documents à part compris (lot S4). `strict` : un document à part qui manque est une erreur
/// (restauration) ; sinon ses lignes sont tenues pour absentes (l'envoi renverra ces fichiers).
pub async fn lire_manifeste(d: &impl Destination, strict: bool) -> Resultat<ManifesteParties> {
    let mut m: ManifesteParties = d.lire_document("manifeste.json").await?.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default();
    for cle in std::mem::take(&mut m.manifestes_jeux) {
        match d.lire_document(&nom_manifeste_jeu(&cle)).await?.and_then(|v| serde_json::from_value::<ManifesteParties>(v).ok()) {
            Some(j) => m.fichiers.extend(j.fichiers),
            None if strict => return Err(Erreur::Serveur("Une partie du manifeste de la sauvegarde manque chez Firehouse.".into())),
            None => {}
        }
    }
    Ok(m)
}

/// Écrit le manifeste : en un document s'il reste sous `seuil`, sinon un document par jeu (`manifeste-<clé>.json`)
/// d'abord, puis `manifeste.json` (le reste et la liste des documents à part) EN DERNIER.
pub async fn ecrire_manifeste(d: &impl Destination, m: &ManifesteParties, seuil: usize) -> Resultat<()> {
    let tout = serde_json::to_vec(m).map_err(|_| Erreur::Disque("Le manifeste est impossible à écrire.".into()))?;
    if tout.len() <= seuil {
        return d.ecrire_document("manifeste.json", &serde_json::to_value(m).unwrap_or_default()).await;
    }
    let mut principal = ManifesteParties::default();
    let mut par_jeu: BTreeMap<String, ManifesteParties> = BTreeMap::new();
    for (r, l) in &m.fichiers {
        match cle_manifeste(r) {
            Some(c) => {
                par_jeu.entry(c).or_default().fichiers.insert(r.clone(), l.clone());
            }
            None => {
                principal.fichiers.insert(r.clone(), l.clone());
            }
        }
    }
    for (cle, j) in &par_jeu {
        let v = serde_json::to_value(j).unwrap_or_default();
        if serde_json::to_vec(&v).map(|o| o.len()).unwrap_or(usize::MAX) > DOCUMENT_MAX {
            return Err(Erreur::Refus("Les parties d'un jeu comptent trop de fichiers pour le manifeste de Firehouse (5 Mo).".into()));
        }
        d.ecrire_document(&nom_manifeste_jeu(cle), &v).await?;
    }
    principal.manifestes_jeux = par_jeu.into_keys().collect();
    d.ecrire_document("manifeste.json", &serde_json::to_value(&principal).unwrap_or_default()).await
}

/// Ce qu'une sauvegarde enverrait (annoncé avant de l'envoyer).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct EstimationSauvegarde {
    /// La taille de toutes les parties à sauvegarder maintenant.
    pub total: u64,
    /// Celle de la dernière sauvegarde faite depuis ce PC (aucune : la première).
    pub precedent: Option<u64>,
    /// Lot S4 : les octets qui partiraient VRAIMENT (contre le dernier manifeste), et combien de fichiers ; `None` :
    /// Firehouse n'a pas pu être lu (hors ligne, mode simulé, ludothèque jamais synchronisée).
    #[serde(default)]
    pub a_envoyer: Option<u64>,
    #[serde(default)]
    pub fichiers_a_envoyer: Option<usize>,
    /// La place utilisée chez Firehouse (tous profils et PC de la personne) et son plafond, s'ils ont été lus.
    #[serde(default)]
    pub utilise: Option<u64>,
    #[serde(default)]
    pub quota: Option<u64>,
    /// Ce qui manquerait de place chez Firehouse pour cet envoi (`manque`, le calcul de l'envoi) ; `None` : inconnu.
    #[serde(default)]
    pub manque: Option<u64>,
}

/// La place de la personne chez Firehouse (`GET /sauvegarde` : `utilise_octets`, `quota_octets`).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct Place {
    pub utilise: u64,
    pub quota: u64,
}

/// Le bilan d'une sauvegarde.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Bilan {
    pub envoyes: usize,
    pub inchanges: usize,
    pub retires: usize,
    /// Octets envoyés cette fois-ci.
    pub octets: u64,
    /// Nombre et taille de toutes les parties sauvegardées.
    pub fichiers: usize,
    pub taille: u64,
    pub date: String,
    pub pc: String,
    pub dossier: String,
    /// Jeux dont les parties restent seulement sur ce PC (ce profil ne les voit pas chez Firehouse) — jamais nommés.
    /// Lot S4 : seulement les jeux de CE profil (un abri à lui, ou des parties qu'il avait déjà envoyées).
    #[serde(default)]
    pub non_envoyes: usize,
    /// Lot S4 : fichiers qui n'ont pas pu partir cette fois (leur copie déjà chez Firehouse reste) ; Frogtend réessaiera.
    #[serde(default)]
    pub sautes: usize,
    /// Lot S4 : le sel des empreintes du profil n'a pas pu être rangé chez Firehouse (contrat 1.51 pas encore servi) :
    /// réessayé à la prochaine sauvegarde. Jamais un échec de la sauvegarde.
    #[serde(default)]
    pub sel_en_attente: bool,
}

/// Ce qu'il faut sauvegarder pour un profil.
#[derive(Default)]
pub struct Contenu {
    pub configuration: Value,
    pub bibliotheque: Value,
    pub parties: Vec<Element>,
    /// Des préfixes (`jeux/<id>/`) de parties qui sont encore sur le PC mais ne partent pas (jeu que ce profil ne voit
    /// pas, ou plus) : ce qui en avait déjà été envoyé à CE compte est gardé tel quel dans la version courante, jamais
    /// retiré — rien de caché ne part, rien ne sort de la protection de Firehouse.
    pub gardes: Vec<String>,
    /// Lot S4 : les jeux que ce profil ne voit pas et dont des parties restent sur ce PC.
    pub invisibles: BTreeSet<i64>,
    /// Parmi eux, ceux dont un abri appartient à CE profil (sa fiche le dit) : comptés dans `non_envoyes` ; ceux d'un
    /// autre profil ne le sont jamais (un jeu pour adultes d'un admin n'existe pas pour un mineur, pas même en nombre).
    pub propres: BTreeSet<i64>,
    /// Lot S4 (Seb, 23/11) : le sel des empreintes d'identifiant du profil, rangé dans l'espace du profil chez Firehouse.
    pub sel: Option<String>,
    /// Pour une estimation : les fichiers que Frogtend aurait écrits (fiches), gardés en mémoire — jamais écrits.
    pub en_memoire: HashMap<PathBuf, Vec<u8>>,
}

/// Le cache des sha256 des fichiers envoyés, par profil (`profils/<id>/empreintes-parties.json`, données locales de
/// Frogtend) : (chemin complet sur ce PC — sa racine comprise) → (taille, date de modification en ns, sha256). Un
/// fichier dont la taille ou la date a bougé est relu. « Et sur 50 Go ? » : un fichier inchangé n'est jamais relu,
/// ni par l'estimation ni par l'envoi qui la suit.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CacheEmpreintes {
    #[serde(default)]
    fichiers: BTreeMap<String, (u64, u64, String)>,
    #[serde(skip)]
    vus: BTreeSet<String>,
    #[serde(skip)]
    lus: usize,
    /// Tous les fichiers d'une sauvegarde ont été vus : l'écriture peut oublier les autres (abri retiré, partie effacée).
    #[serde(skip)]
    complet: bool,
}

impl CacheEmpreintes {
    pub const NOM: &'static str = "empreintes-parties.json";

    /// La clé d'un fichier : son chemin complet (la racine comprise : deux dossiers d'abris ne se confondent jamais).
    pub fn cle(source: &Path) -> String {
        source.to_string_lossy().into_owned()
    }

    pub fn lire(f: &Path) -> Self {
        std::fs::read(f).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default()
    }

    /// Écrit le cache (après un passage complet : seulement les fichiers vus cette fois, un abri retiré en sort).
    pub fn ecrire(&self, f: &Path) -> Resultat<()> {
        let gardes: BTreeMap<&String, &(u64, u64, String)> = self.fichiers.iter().filter(|(k, _)| !self.complet || self.vus.contains(*k)).collect();
        if let Some(p) = f.parent() {
            std::fs::create_dir_all(p)?;
        }
        let provisoire = f.with_extension("json.provisoire");
        std::fs::write(&provisoire, serde_json::to_vec(&json!({ "fichiers": gardes })).unwrap_or_default())?;
        std::fs::rename(&provisoire, f)?;
        Ok(())
    }

    /// Combien de fichiers ont été LUS pour leur empreinte (tests : un abri déjà connu n'est jamais relu).
    pub fn lus(&self) -> usize {
        self.lus
    }

    /// Le sha256 de `source` : du cache pour un fichier inchangé (`cle`), sinon lu.
    fn empreinte(&mut self, cle: Option<&String>, source: &Path, m: &std::fs::Metadata) -> Resultat<String> {
        let date = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_nanos() as u64);
        if let Some(cle) = cle {
            self.vus.insert(cle.clone());
            if let (Some(dt), Some((t, d0, s))) = (date, self.fichiers.get(cle)) {
                if *t == m.len() && *d0 == dt {
                    return Ok(s.clone());
                }
            }
        }
        self.lus += 1;
        let s = empreinte(source)?;
        if let (Some(cle), Some(dt)) = (cle, date) {
            self.fichiers.insert(cle.clone(), (m.len(), dt, s.clone()));
        }
        Ok(s)
    }
}

/// Une erreur qui ARRÊTE tout l'envoi (lot S4) : jeton refusé (401), espace plein (507), réseau, Firehouse en
/// difficulté (5xx, 503)… Les autres (409, un fichier local illisible, 400/413) ne font que SAUTER ce fichier.
pub fn arrete_tout(e: &Erreur) -> bool {
    !matches!(e, Erreur::Conflit(_) | Erreur::Disque(_) | Erreur::Refus(_) | Erreur::Plage(_))
}

/// La classe d'une erreur, pour le journal (jamais son texte : il peut citer un fichier).
pub fn classe(e: &Erreur) -> String {
    serde_json::to_value(e).ok().and_then(|v| v["sorte"].as_str().map(String::from)).unwrap_or_else(|| "inconnue".into())
}

/// Le refus « il manque N chez Firehouse » (place vérifiée avant d'envoyer).
pub fn manque_de_place(manque: u64) -> Erreur {
    Erreur::Plein {
        motif: format!(
            "Il manque {} chez Firehouse pour envoyer tes parties : rien n'a été envoyé. Libère de la place ou demande plus d'espace à Seb.",
            crate::erreurs::taille_lisible(manque)
        ),
        manque_octets: Some(manque),
    }
}

/// Le préfixe `jeux/<id>/emplacements/` d'une fiche envoyée (`jeux/<id>/emplacements/fiche.json`).
fn prefixe_de_fiche(relatif: &str) -> Option<String> {
    let reste = relatif.strip_prefix("jeux/")?;
    let (id, apres) = reste.split_once('/')?;
    (apres == crate::abri_parties::FICHE_ENVOYEE && id.parse::<i64>().is_ok()).then(|| format!("jeux/{id}/{}", crate::abri_parties::PREFIXE))
}

pub fn empreinte(chemin: &Path) -> Resultat<String> {
    let mut f = std::fs::File::open(chemin)?;
    let mut h = Sha256::new();
    let mut tampon = vec![0u8; 256 * 1024];
    loop {
        let n = f.read(&mut tampon)?;
        if n == 0 {
            break;
        }
        h.update(&tampon[..n]);
    }
    Ok(h.finalize().iter().map(|o| format!("{o:02x}")).collect())
}

/// Taille maximale d'un morceau envoyé à Firehouse (contrat 1.5 : 8 Mo au plus).
pub const MORCEAU: usize = 8 * 1024 * 1024;

/// Où va la sauvegarde : l'API de Firehouse (partout), ou un dossier local (tests).
#[allow(async_fn_in_trait)]
pub trait Destination {
    /// Un document JSON (`configuration.json`, `bibliotheque.json`, `manifeste.json`, `derniere-sauvegarde.json`).
    async fn lire_document(&self, nom: &str) -> Resultat<Option<Value>>;
    async fn ecrire_document(&self, nom: &str, v: &Value) -> Resultat<()>;
    /// Envoie un fichier de `parties/` ; rend les octets envoyés.
    async fn envoyer_fichier(&self, relatif: &str, source: &Path, sha: &str, taille: u64) -> Resultat<u64>;
    async fn supprimer_fichier(&self, relatif: &str) -> Resultat<()>;
    /// `Some(vrai)` si la destination sait que ce fichier est bien là à cette taille ; `None` si elle ne peut pas le
    /// dire à peu de frais (on fait alors confiance au manifeste).
    fn present(&self, relatif: &str, taille: u64) -> Option<bool>;
    /// Lot S4 : un document de l'espace du PROFIL (pas de ce PC) — le sel des empreintes (rangé ici dans `abris-sel.txt`,
    /// envoyé en document `abris-sel.json` sous le PC réservé `_profil`, contrat 1.51).
    async fn ecrire_document_profil(&self, nom: &str, v: &Value) -> Resultat<()>;
    /// Lot S4 : la place utilisée et le plafond de la personne ; `None` : inconnus (rien n'est vérifié).
    async fn place(&self) -> Resultat<Option<Place>>;
}

/// Le PC réservé à l'espace du profil chez Firehouse (contrat 1.51) : jamais une sauvegarde d'un PC.
pub const PC_PROFIL: &str = "_profil";

/// Un dossier local (tests, ou copie de secours).
pub struct DossierLocal(pub PathBuf);

impl DossierLocal {
    fn ecrire_sur(cible: &Path, octets: &[u8]) -> Resultat<()> {
        if let Some(p) = cible.parent() {
            std::fs::create_dir_all(p)?;
        }
        let part = cible.with_extension("part-frogtend");
        std::fs::write(&part, octets)?;
        std::fs::rename(&part, cible)?;
        Ok(())
    }
}

impl Destination for DossierLocal {
    async fn lire_document(&self, nom: &str) -> Resultat<Option<Value>> {
        Ok(std::fs::read(self.0.join(nom)).ok().and_then(|o| serde_json::from_slice(&o).ok()))
    }
    async fn ecrire_document(&self, nom: &str, v: &Value) -> Resultat<()> {
        Self::ecrire_sur(&self.0.join(nom), &serde_json::to_vec_pretty(v).unwrap())
    }
    async fn envoyer_fichier(&self, relatif: &str, source: &Path, _sha: &str, taille: u64) -> Resultat<u64> {
        let cible = self.0.join("parties").join(relatif);
        if let Some(p) = cible.parent() {
            std::fs::create_dir_all(p)?;
        }
        let part = cible.with_extension("part-frogtend");
        let n = std::fs::copy(source, &part)?;
        if std::fs::metadata(&part)?.len() != taille {
            let _ = std::fs::remove_file(&part);
            return Err(Erreur::Disque(format!("La copie de {} est incomplète.", source.display())));
        }
        std::fs::rename(&part, cible)?;
        Ok(n)
    }
    async fn supprimer_fichier(&self, relatif: &str) -> Resultat<()> {
        let p = self.0.join("parties").join(relatif);
        if p.is_file() {
            std::fs::remove_file(p)?;
        }
        Ok(())
    }
    fn present(&self, relatif: &str, taille: u64) -> Option<bool> {
        Some(std::fs::metadata(self.0.join("parties").join(relatif)).is_ok_and(|m| m.len() == taille))
    }
    async fn ecrire_document_profil(&self, nom: &str, v: &Value) -> Resultat<()> {
        Self::ecrire_sur(&self.0.join(PC_PROFIL).join(nom), &serde_json::to_vec_pretty(v).unwrap())
    }
    async fn place(&self) -> Resultat<Option<Place>> {
        Ok(None)
    }
}

/// Un nom accepté par l'API pour un profil ou un PC : lettres et chiffres sans accents, espace, « . », « _ », « - »,
/// 64 caractères au plus (contrat 1.5).
pub fn nom_pour_api(s: &str) -> String {
    // Les accents s'enlèvent, la casse reste (« Sébastien » → « Sebastien »).
    let sans_accent = |c: char| -> char {
        let bas = c.to_lowercase().next().unwrap_or(c);
        let base = match bas {
            'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' | 'ì' => 'i',
            'ô' | 'ö' | 'ó' | 'ò' | 'õ' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            'ÿ' => 'y',
            _ => return c,
        };
        if c.is_uppercase() { base.to_ascii_uppercase() } else { base }
    };
    let n: String = s
        .chars()
        .map(sans_accent)
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, ' ' | '.' | '_' | '-') { c } else { '_' })
        .take(64)
        .collect();
    let n = n.trim_matches(|c| c == ' ' || c == '.').to_string();
    if n.is_empty() || n.contains("..") { "profil".into() } else { n }
}

/// L'API de sauvegarde de Firehouse (contrat 1.5), pour `<username du jeton>/<profil>/<pc>/`.
pub struct Api<'a> {
    pub client: &'a crate::firehouse::Client,
    pub profil: String,
    pub pc: String,
    /// Taille des morceaux (8 Mo ; plus petit dans les tests).
    pub morceau: usize,
    /// Lot S4 : une partie est-elle en cours ? L'envoi se met alors en pause entre deux morceaux (jamais la connexion
    /// prise au jeu ni au streaming).
    pub pause: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
}

/// L'attente entre deux regards pendant une pause (une partie en cours).
pub const ATTENTE_PAUSE: std::time::Duration = std::time::Duration::from_secs(5);

/// La cadence des morceaux de la sauvegarde (lot S4, comme les envois Switch) : taille fixe, pause tant qu'une partie
/// est en cours.
pub struct CadenceSauvegarde {
    pub morceau: usize,
    pub pause: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
    pub attente: std::time::Duration,
}

impl crate::firehouse::Cadence for CadenceSauvegarde {
    async fn avant(&mut self, _position: u64) -> Resultat<usize> {
        if let Some(p) = &self.pause {
            while p() {
                tokio::time::sleep(self.attente).await;
            }
        }
        Ok(self.morceau)
    }
}

impl Api<'_> {
    fn base(&self) -> String {
        format!("/sauvegarde/{}/{}", crate::source::encoder(&self.profil), crate::source::encoder(&self.pc))
    }
    fn route_fichier(&self, relatif: &str) -> String {
        format!("{}/fichier?chemin={}", self.base(), crate::source::encoder(relatif))
    }
    fn refus(b: &crate::firehouse::Brute) -> Erreur {
        crate::firehouse::erreur_du_statut(b.statut, &b.octets)
    }
}

impl Destination for Api<'_> {
    async fn lire_document(&self, nom: &str) -> Resultat<Option<Value>> {
        let b = self.client.brute(reqwest::Method::GET, &format!("{}/document/{nom}", self.base()), None, &[]).await?;
        match b.statut {
            200 => Ok(serde_json::from_slice(&b.octets).ok()),
            404 => Ok(None),
            _ => Err(Self::refus(&b)),
        }
    }
    async fn ecrire_document(&self, nom: &str, v: &Value) -> Resultat<()> {
        let corps = serde_json::to_vec(v).unwrap();
        let b = self
            .client
            .brute(reqwest::Method::PUT, &format!("{}/document/{nom}", self.base()), Some(corps), &[("Content-Type", "application/json".into())])
            .await?;
        if (200..300).contains(&b.statut) { Ok(()) } else { Err(Self::refus(&b)) }
    }
    async fn envoyer_fichier(&self, relatif: &str, source: &Path, sha: &str, taille: u64) -> Resultat<u64> {
        let route = self.route_fichier(relatif);
        if taille == 0 {
            let b = self.client.brute(reqwest::Method::PUT, &route, Some(vec![]), &[("X-Contenu-Sha256", sha.into())]).await?;
            return if (200..300).contains(&b.statut) { Ok(0) } else { Err(Self::refus(&b)) };
        }
        // Reprise : ce que Firehouse a déjà reçu de ce fichier (`.part`).
        let deja = self.client.brute(reqwest::Method::HEAD, &route, None, &[]).await?;
        // Contrat 1.51 : un fichier déjà COMPLET chez Firehouse, de la même empreinte, n'est pas renvoyé (`X-Complet: 1`
        // et `X-Sha256`). Une empreinte différente : il repart en entier et remplace l'ancien.
        if deja.complet && deja.sha256.as_deref() == Some(sha) {
            return Ok(0);
        }
        // La boucle des morceaux est partagée avec les envois Switch (firehouse.rs) ; ici : morceaux fixes (pause
        // pendant une partie), le 409 sans `recu` fait repartir de zéro une seule fois, et la fin est dite par `complet`.
        // `X-Recu` = total (un `.part` entier, `X-Complet: 0` en 1.51) : la boucle renvoie le DERNIER morceau, jamais un
        // morceau vide (firehouse.rs).
        let depart = deja.recu.unwrap_or(0);
        let envoi = crate::firehouse::EnvoiParMorceaux {
            route: &route,
            source,
            total: taille,
            depart,
            entetes: vec![("X-Contenu-Sha256", sha.to_string())],
            // Jamais un chemin dans un message (il pourrait finir au journal ou à l'écran) : lot S4.
            nom: "un fichier de parties",
        };
        let regles = crate::firehouse::ReglesMorceaux { fini: |v| v["complet"] == true, zero_sans_recu: true };
        let mut cadence = CadenceSauvegarde { morceau: self.morceau, pause: self.pause.clone(), attente: ATTENTE_PAUSE };
        let (envoyes, _) = self.client.envoyer_par_morceaux(&envoi, &regles, &mut cadence).await?;
        Ok(envoyes)
    }
    async fn supprimer_fichier(&self, relatif: &str) -> Resultat<()> {
        let b = self.client.brute(reqwest::Method::DELETE, &self.route_fichier(relatif), None, &[]).await?;
        if (200..300).contains(&b.statut) || b.statut == 404 { Ok(()) } else { Err(Self::refus(&b)) }
    }
    fn present(&self, _relatif: &str, _taille: u64) -> Option<bool> {
        None
    }
    async fn ecrire_document_profil(&self, nom: &str, v: &Value) -> Resultat<()> {
        let route = format!("/sauvegarde/{}/{PC_PROFIL}/document/{nom}", crate::source::encoder(&self.profil));
        let corps = serde_json::to_vec(v).unwrap();
        let b = self.client.brute(reqwest::Method::PUT, &route, Some(corps), &[("Content-Type", "application/json".into())]).await?;
        if (200..300).contains(&b.statut) { Ok(()) } else { Err(Self::refus(&b)) }
    }
    async fn place(&self) -> Resultat<Option<Place>> {
        lire_place(self.client).await
    }
}

/// La place de la personne chez Firehouse (`GET /sauvegarde`, contrat 1.50 : `utilise_octets` = tout son dossier,
/// tous profils et PC, documents et envois en cours compris). `None` : Firehouse ne le dit pas.
pub async fn lire_place(client: &crate::firehouse::Client) -> Resultat<Option<Place>> {
    let b = client.brute(reqwest::Method::GET, "/sauvegarde", None, &[]).await?;
    match b.statut {
        200..=299 => {
            let v: Value = serde_json::from_slice(&b.octets).unwrap_or(Value::Null);
            Ok(match (v["utilise_octets"].as_u64(), v["quota_octets"].as_u64()) {
                (Some(utilise), Some(quota)) if quota > 0 => Some(Place { utilise, quota }),
                _ => None,
            })
        }
        404 => Ok(None),
        s => Err(crate::firehouse::erreur_du_statut(s, &b.octets)),
    }
}

/// Un fichier prêt à partir : son empreinte, sa taille, et s'il est déjà chez Firehouse tel quel.
struct Ligne<'a> {
    e: &'a Element,
    sha: String,
    taille: u64,
    deja: bool,
}

/// Ce qu'un envoi ferait, calculé AVANT de rien envoyer (lot S4) : sert à l'estimation et à la place à vérifier.
pub struct Preparation<'a> {
    lignes: Vec<Ligne<'a>>,
    /// Fichiers illisibles sur ce PC (sautés, comptés) : leur ligne de l'ancien manifeste reste.
    illisibles: BTreeSet<String>,
    ancien: ManifesteParties,
    /// Les octets qui partiront VRAIMENT, et combien de fichiers.
    pub a_envoyer: u64,
    pub fichiers_a_envoyer: usize,
    /// La taille de toutes les parties.
    pub total: u64,
}

/// L'empreinte d'un fichier à envoyer, lue hors du fil asynchrone.
enum Lu {
    /// Disparu entre-temps (un émulateur qui écrit pendant la sauvegarde) : sauté, jamais tout arrêté.
    Absent,
    /// Illisible (verrouillé…) : sauté et compté, sa copie déjà chez Firehouse reste.
    Illisible,
    Pret(u64, String),
}

/// Ce qui manquerait de place chez Firehouse pour cet envoi (0 : assez, ou place inconnue) — LE calcul de la place,
/// pour l'envoi comme pour l'estimation.
pub fn manque(p: &Preparation, place: Option<Place>) -> u64 {
    let Some(place) = place else { return 0 };
    if p.a_envoyer == 0 {
        return 0;
    }
    let libre = place.quota.saturating_sub(place.utilise);
    // Contrat 1.51 : l'ancien fichier est déduit pendant tout l'envoi de son remplaçant (plus de double compte). Le
    // calcul reste prudent (la place rendue par les fichiers remplacés n'est pas comptée) ; le 507 chiffré tranche.
    p.a_envoyer.saturating_sub(libre)
}

/// Le manifeste tiendra-t-il chez Firehouse (5 Mo par document) ? Vérifié AVANT d'envoyer quoi que ce soit (lot S4).
pub fn verifier_manifeste(m: &ManifesteParties) -> Resultat<()> {
    let taille = |m: &ManifesteParties| serde_json::to_vec(m).map(|o| o.len()).unwrap_or(usize::MAX);
    if taille(m) <= SEUIL_MANIFESTE {
        return Ok(());
    }
    let mut principal = ManifesteParties::default();
    let mut par_jeu: BTreeMap<String, ManifesteParties> = BTreeMap::new();
    for (r, l) in &m.fichiers {
        match cle_manifeste(r) {
            Some(c) => par_jeu.entry(c).or_default().fichiers.insert(r.clone(), l.clone()),
            None => principal.fichiers.insert(r.clone(), l.clone()),
        };
    }
    principal.manifestes_jeux = par_jeu.keys().cloned().collect();
    if taille(&principal) > DOCUMENT_MAX || par_jeu.values().any(|j| taille(j) > DOCUMENT_MAX) {
        return Err(Erreur::Refus("Tes parties comptent trop de fichiers pour le manifeste de Firehouse (5 Mo par document) : rien n'a été envoyé.".into()));
    }
    Ok(())
}

/// Prépare l'envoi : lit le dernier manifeste et l'empreinte de chaque fichier (du cache pour un fichier inchangé),
/// hors du fil asynchrone. Rien n'est envoyé.
pub async fn preparer<'a>(d: &impl Destination, contenu: &'a Contenu, cache: &mut CacheEmpreintes) -> Resultat<Preparation<'a>> {
    let ancien = lire_manifeste(d, false).await?;
    let mut p = Preparation { lignes: Vec::new(), illisibles: BTreeSet::new(), ancien, a_envoyer: 0, fichiers_a_envoyer: 0, total: 0 };
    let travaux: Vec<(PathBuf, Option<Vec<u8>>)> = contenu.parties.iter().map(|e| (e.source.clone(), contenu.en_memoire.get(&e.source).cloned())).collect();
    let mut c = std::mem::take(cache);
    let (lus, c) = tokio::task::spawn_blocking(move || {
        let lus: Vec<Lu> = travaux
            .into_iter()
            .map(|(source, memoire)| {
                if let Some(o) = memoire {
                    return Lu::Pret(o.len() as u64, Sha256::digest(&o).iter().map(|b| format!("{b:02x}")).collect());
                }
                let Ok(m) = std::fs::metadata(&source) else { return Lu::Absent };
                if !m.is_file() {
                    return Lu::Absent;
                }
                match c.empreinte(Some(&CacheEmpreintes::cle(&source)), &source, &m) {
                    Ok(s) => Lu::Pret(m.len(), s),
                    Err(_) => Lu::Illisible,
                }
            })
            .collect();
        c.complet = true;
        (lus, c)
    })
    .await
    .map_err(|_| Erreur::Disque("La lecture des parties s'est arrêtée brutalement.".into()))?;
    *cache = c;
    for (e, lu) in contenu.parties.iter().zip(lus) {
        let (taille, sha) = match lu {
            Lu::Absent => continue,
            Lu::Illisible => {
                p.illisibles.insert(e.relatif.clone());
                continue;
            }
            Lu::Pret(t, s) => (t, s),
        };
        let ancienne = p.ancien.fichiers.get(&e.relatif);
        let deja = ancienne == Some(&(sha.clone(), taille)) && d.present(&e.relatif, taille).unwrap_or(true);
        if !deja {
            p.a_envoyer += taille;
            p.fichiers_a_envoyer += 1;
        }
        p.total += taille;
        p.lignes.push(Ligne { e, sha, taille, deja });
    }
    // Le manifeste qui en sortira (au plus : les lignes prêtes et les anciennes gardées) doit tenir chez Firehouse.
    let mut futur = p.ancien.clone();
    for l in &p.lignes {
        futur.fichiers.insert(l.e.relatif.clone(), (l.sha.clone(), l.taille));
    }
    verifier_manifeste(&futur)?;
    Ok(p)
}

/// Envoie la sauvegarde à la destination : seul ce qui a changé (d'après le manifeste), et ce qui a disparu sort de
/// la version courante.
pub async fn envoyer(d: &impl Destination, contenu: &Contenu, pc: &str, emplacement: &str) -> Resultat<Bilan> {
    envoyer_avec(d, contenu, pc, emplacement, &mut CacheEmpreintes::default()).await
}

/// `envoyer`, avec le cache des empreintes des abris du profil.
///
/// Lot S4 : la place libre est vérifiée AVANT (rien n'est envoyé s'il en manque) ; un fichier en échec (409, 400/413,
/// illisible ici) est SAUTÉ et compté, et sa ligne de l'ancien manifeste RESTE (sa copie chez Firehouse n'est jamais
/// retirée ; sans ancienne ligne, il repartira la prochaine fois) ; une erreur qui `arrete_tout` (401, 507, réseau,
/// 5xx) arrête tout, sans écrire ni manifeste ni dernière sauvegarde ; la fiche des emplacements d'un jeu part AVANT
/// ses fichiers — si elle échoue, les fichiers `emplacements/` de ce jeu ne partent pas et n'entrent pas au manifeste.
pub async fn envoyer_avec(d: &impl Destination, contenu: &Contenu, pc: &str, emplacement: &str, cache: &mut CacheEmpreintes) -> Resultat<Bilan> {
    envoyer_sous(d, contenu, pc, emplacement, cache, None).await
}

/// `envoyer_avec` ; `plafond_auto` (une sauvegarde AUTOMATIQUE) : au-delà de ces octets à envoyer vraiment, rien ne
/// part (`GROSSE_AUTO`) — la personne la lancera elle-même. Un seul passage : rien n'est lu deux fois.
pub async fn envoyer_sous(d: &impl Destination, contenu: &Contenu, pc: &str, emplacement: &str, cache: &mut CacheEmpreintes, plafond_auto: Option<u64>) -> Resultat<Bilan> {
    let p = preparer(d, contenu, cache).await?;
    let m = manque(&p, d.place().await?);
    if m > 0 {
        return Err(manque_de_place(m));
    }
    if plafond_auto.is_some_and(|max| p.a_envoyer > max) {
        return Err(Erreur::Refus(format!("{GROSSE_AUTO} : {} de parties à envoyer.", crate::erreurs::taille_lisible(p.a_envoyer))));
    }
    let mut nouveau = ManifesteParties::default();
    let mut bilan = Bilan { pc: pc.into(), dossier: emplacement.into(), date: maintenant(), ..Default::default() };
    // Les fiches d'emplacements en échec : les fichiers de leur jeu sous `emplacements/` ne partent pas.
    let mut bloques: Vec<String> = Vec::new();
    for r in &p.illisibles {
        bilan.sautes += 1;
        if let Some(l) = p.ancien.fichiers.get(r) {
            nouveau.fichiers.insert(r.clone(), l.clone());
        }
        bloques.extend(prefixe_de_fiche(r));
    }
    for l in &p.lignes {
        let r = &l.e.relatif;
        if bloques.iter().any(|b| r.starts_with(b.as_str())) {
            bilan.sautes += 1;
            if let Some(a) = p.ancien.fichiers.get(r) {
                nouveau.fichiers.insert(r.clone(), a.clone());
            }
            continue;
        }
        if l.deja {
            bilan.inchanges += 1;
        } else {
            match d.envoyer_fichier(r, &l.e.source, &l.sha, l.taille).await {
                Ok(n) => {
                    bilan.octets += n;
                    bilan.envoyes += 1;
                }
                Err(e) if arrete_tout(&e) => return Err(e),
                Err(_) => {
                    bilan.sautes += 1;
                    if let Some(a) = p.ancien.fichiers.get(r) {
                        nouveau.fichiers.insert(r.clone(), a.clone());
                    }
                    bloques.extend(prefixe_de_fiche(r));
                    continue;
                }
            }
        }
        bilan.taille += l.taille;
        nouveau.fichiers.insert(r.clone(), (l.sha.clone(), l.taille));
    }
    // Ce qui n'existe plus sur le PC sort de la version courante (l'historique du serveur le garde). Ce qui est encore
    // sur le PC mais ne part plus (`gardes`), ou dont la fiche n'a pas pu partir, reste tel qu'il avait été envoyé.
    for (r, e) in &p.ancien.fichiers {
        if nouveau.fichiers.contains_key(r) {
            continue;
        }
        if contenu.gardes.iter().chain(bloques.iter()).any(|g| r.starts_with(g.as_str())) {
            nouveau.fichiers.insert(r.clone(), e.clone());
            continue;
        }
        match d.supprimer_fichier(r).await {
            Ok(()) => bilan.retires += 1,
            Err(x) if arrete_tout(&x) => return Err(x),
            // Une suppression refusée : la ligne reste (réessayée la prochaine fois).
            Err(_) => {
                nouveau.fichiers.insert(r.clone(), e.clone());
            }
        }
    }
    // Seulement les jeux de CE profil : un abri à lui, ou des parties qu'il avait déjà envoyées.
    bilan.non_envoyes = contenu
        .invisibles
        .iter()
        .filter(|id| {
            let prefixe = format!("jeux/{id}/");
            contenu.propres.contains(id) || p.ancien.fichiers.keys().any(|r| r.starts_with(&prefixe))
        })
        .count();
    bilan.fichiers = nouveau.fichiers.len();
    d.ecrire_document("configuration.json", &contenu.configuration).await?;
    d.ecrire_document("bibliotheque.json", &contenu.bibliotheque).await?;
    // Le sel du profil (Seb, 23/11) : un refus (contrat 1.51 pas encore servi) n'est jamais un échec de la sauvegarde.
    if let Some(sel) = &contenu.sel {
        if d.ecrire_document_profil(crate::abri_parties::NOM_SEL_ENVOYE, &json!({ "sel": sel })).await.is_err() {
            bilan.sel_en_attente = true;
        }
    }
    // Le manifeste en dernier : tant qu'il n'est pas écrit, la sauvegarde précédente reste la référence.
    ecrire_manifeste(d, &nouveau, SEUIL_MANIFESTE).await?;
    d.ecrire_document("derniere-sauvegarde.json", &serde_json::to_value(&bilan).unwrap()).await?;
    Ok(bilan)
}

/// Le début du refus « une sauvegarde est déjà en cours » (l'interface la refait quand le cœur dit `sauvegarde-finie`).
pub const DEJA_EN_COURS: &str = "Une sauvegarde est déjà en cours sur ce PC";

/// Le début du refus d'une sauvegarde AUTOMATIQUE trop grosse (rien n'est envoyé ; l'interface le dit une fois).
pub const GROSSE_AUTO: &str = "Sauvegarde automatique remise à plus tard";

/// Les dossiers de données dont une sauvegarde est en cours (lot S4 : une seule à la fois, côté cœur).
static SAUVEGARDES_EN_COURS: std::sync::Mutex<BTreeSet<PathBuf>> = std::sync::Mutex::new(BTreeSet::new());

/// Une sauvegarde en cours : rendue à la fin, quoi qu'il arrive.
struct VerrouSauvegarde(PathBuf);

impl VerrouSauvegarde {
    fn prendre(dossier: &Path) -> Option<Self> {
        let mut l = SAUVEGARDES_EN_COURS.lock().unwrap_or_else(|e| e.into_inner());
        l.insert(dossier.to_path_buf()).then(|| VerrouSauvegarde(dossier.to_path_buf()))
    }
}

impl Drop for VerrouSauvegarde {
    fn drop(&mut self) {
        SAUVEGARDES_EN_COURS.lock().unwrap_or_else(|e| e.into_inner()).remove(&self.0);
    }
}

/// Le dossier des fiches des captures envoyées (`profils\<id>\fiches-captures\<id de la capture>.json`).
pub const FICHES_CAPTURES: &str = "fiches-captures";

/// Le début du refus « une sauvegarde à reprendre existe » (l'interface le reconnaît pour proposer de la restaurer).
pub const A_REPRENDRE: &str = "Une sauvegarde de ce profil existe déjà chez Firehouse";

/// Avant la PREMIÈRE sauvegarde d'un profil sur ce PC (rien envoyé d'ici, rien restauré, rien écarté) : si
/// Firehouse a déjà, au même nom et au même PC, une sauvegarde avec des parties, l'envoyer la VIDERAIT (ce qui manque
/// ici en sortirait). Refus : la personne la restaure, ou l'écarte en le disant.
pub async fn verifier_reprise(d: &impl Destination) -> Resultat<()> {
    let ancien = lire_manifeste(d, true).await?;
    // Sa liste de jeux compte aussi (une sauvegarde sans parties mais avec 30 jeux à remettre).
    let jeux = d.lire_document("bibliotheque.json").await?.and_then(|v| v["jeux"].as_array().map(|l| l.len())).unwrap_or(0);
    if ancien.fichiers.is_empty() && jeux == 0 {
        return Ok(());
    }
    Err(Erreur::Refus(format!(
        "{A_REPRENDRE} pour ce PC ({} fichier(s) de parties, {jeux} jeu(x)) : restaure-la d'abord, ou choisis de la remplacer.",
        ancien.fichiers.len()
    )))
}

/// Une copie écrite par Frogtend pour partir (fiche, réglages sans secret) : réécrite seulement si elle a changé ; pour
/// une estimation (`ecrire` faux), gardée en mémoire — jamais écrite.
fn poser_copie(f: &Path, octets: &[u8], ecrire: bool, en_memoire: &mut HashMap<PathBuf, Vec<u8>>) -> Resultat<()> {
    if !ecrire {
        en_memoire.insert(f.to_path_buf(), octets.to_vec());
        return Ok(());
    }
    if std::fs::read(f).ok().as_deref() != Some(octets) {
        if let Some(p) = f.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::write(f, octets)?;
    }
    Ok(())
}

fn lire_json(p: &Path) -> Value {
    std::fs::read(p).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or(Value::Null)
}

impl Noyau {
    fn fichier_derniere_sauvegarde(&self, profil: &str) -> PathBuf {
        self.dossier.join("profils").join(profil).join("derniere-sauvegarde.json")
    }

    /// La marque « la sauvegarde existante chez Firehouse a été reprise (restaurée) ou écartée » : ce profil peut
    /// désormais l'écrire.
    fn fichier_reprise(&self, profil: &str) -> PathBuf {
        self.dossier.join("profils").join(profil).join("sauvegarde-reprise.json")
    }

    /// Note que la personne a repris (restauré) ou écarté la sauvegarde existante.
    /// `cible` : la sauvegarde concernée (profil, PC), telle que Firehouse la range — la garde n'est levée que pour ELLE.
    pub async fn marquer_reprise(&self, comment: &str, profil: &str, pc: &str) -> Resultat<()> {
        let s = self.session().await?;
        let f = self.fichier_reprise(&s.profil.id);
        if let Some(p) = f.parent() {
            std::fs::create_dir_all(p)?;
        }
        let v = json!({ "comment": comment, "profil": nom_pour_api(profil), "pc": nom_pour_api(pc), "le": maintenant() });
        std::fs::write(f, serde_json::to_vec_pretty(&v).unwrap())?;
        Ok(())
    }

    /// La sauvegarde (profil, PC) de Firehouse a-t-elle été reprise (restaurée) ou écartée par ce profil ? Restaurer
    /// celle d'un AUTRE PC ne lève pas la garde de celle de CE PC.
    fn reprise_de(&self, profil_id: &str, profil: &str, pc: &str) -> bool {
        let v = lire_json(&self.fichier_reprise(profil_id));
        v["profil"].as_str() == Some(nom_pour_api(profil).as_str()) && v["pc"].as_str() == Some(nom_pour_api(pc).as_str())
    }

    /// La dernière sauvegarde réussie du profil ouvert, depuis ce PC.
    pub async fn derniere_sauvegarde(&self) -> Resultat<Option<Bilan>> {
        let s = self.session().await?;
        Ok(std::fs::read(self.fichier_derniere_sauvegarde(&s.profil.id)).ok().and_then(|o| serde_json::from_slice(&o).ok()))
    }

    /// Le cache des empreintes des abris du profil.
    fn fichier_cache_empreintes(&self, profil: &str) -> PathBuf {
        self.dossier.join("profils").join(profil).join(CacheEmpreintes::NOM)
    }

    /// Ce qu'une sauvegarde enverrait, à annoncer AVANT (une première sauvegarde PS3 ou Xbox peut peser des centaines de
    /// Mo) : la taille de toutes les parties, et celle de la dernière sauvegarde. Lot S4 : aussi ce qui partira
    /// VRAIMENT (contre le dernier manifeste de Firehouse), la place utilisée et le plafond — Firehouse injoignable :
    /// ces trois-là restent inconnus, jamais une erreur. Rien n'est écrit chez Firehouse ni pour partir (ni fiche, ni
    /// copie) ; seul le cache LOCAL des empreintes est mis à jour (l'envoi qui suit ne relit rien).
    pub async fn estimer_sauvegarde(&self, programmes_emulateurs: &[String], pc: &str) -> Resultat<EstimationSauvegarde> {
        // La session lue UNE fois (un changement de profil pendant l'estimation ne mêle jamais deux profils).
        let s = self.session().await?;
        let c = self.contenu_pour(&s, programmes_emulateurs, false)?;
        let total = c
            .parties
            .iter()
            .filter_map(|e| c.en_memoire.get(&e.source).map(|o| o.len() as u64).or_else(|| std::fs::metadata(&e.source).ok().map(|m| m.len())))
            .sum();
        let precedent = std::fs::read(self.fichier_derniere_sauvegarde(&s.profil.id)).ok().and_then(|o| serde_json::from_slice::<Bilan>(&o).ok()).map(|b| b.taille);
        let mut est = EstimationSauvegarde { total, precedent, ..Default::default() };
        let crate::source::Source::Firehouse(client) = &s.source else { return Ok(est) };
        if !matches!(s.verrou().synchronise_le(), Ok(Some(_))) {
            return Ok(est);
        }
        let api = Api { client, profil: nom_pour_api(&s.profil.nom), pc: nom_pour_api(pc), morceau: MORCEAU, pause: None };
        let fichier_cache = self.fichier_cache_empreintes(&s.profil.id);
        let mut cache = CacheEmpreintes::lire(&fichier_cache);
        let p = match preparer(&api, &c, &mut cache).await {
            Ok(p) => Some(p),
            // Hors ligne : rien de plus n'est demandé (pas une seconde attente du réseau pour rien).
            Err(Erreur::Reseau(_)) => return Ok(est),
            Err(_) => None,
        };
        let _ = cache.ecrire(&fichier_cache);
        let place = api.place().await.ok().flatten();
        if let Some(pl) = place {
            est.utilise = Some(pl.utilise);
            est.quota = Some(pl.quota);
        }
        if let Some(p) = p {
            est.a_envoyer = Some(p.a_envoyer);
            est.fichiers_a_envoyer = Some(p.fichiers_a_envoyer);
            // Le même calcul que l'envoi (une seule règle, rendue à l'interface).
            est.manque = Some(manque(&p, place));
        }
        Ok(est)
    }

    /// La place utilisée chez Firehouse et le plafond (Options ▸ Sauvegarde) ; `None` : inconnus (hors ligne, simulé).
    pub async fn place_sauvegarde(&self) -> Resultat<Option<Place>> {
        let s = self.session().await?;
        match &s.source {
            crate::source::Source::Firehouse(c) => lire_place(c).await,
            crate::source::Source::Simulee => Ok(None),
        }
    }

    /// Sauvegarde le profil ouvert chez Firehouse (`<username du jeton>/<profil>/<pc>/`). `ecarter` : la personne a
    /// choisi de REMPLACER une sauvegarde existante qu'elle n'a pas restaurée (sinon refus, `verifier_reprise`).
    /// `en_partie` : une partie est-elle en cours (l'envoi se met alors en pause) ? Lot S4 : UNE sauvegarde à la fois par
    /// dossier de données (`DEJA_EN_COURS`). `plafond_auto` (une sauvegarde automatique) : au-delà de ces octets à
    /// envoyer vraiment, rien ne part (`GROSSE_AUTO`), sans estimation séparée.
    pub async fn sauvegarder(
        &self,
        programmes_emulateurs: &[String],
        pc: &str,
        ecarter: bool,
        en_partie: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
        plafond_auto: Option<u64>,
    ) -> Resultat<Bilan> {
        let Some(_verrou) = VerrouSauvegarde::prendre(&self.dossier) else {
            return Err(Erreur::Refus(format!("{DEJA_EN_COURS} : elle sera refaite juste après.")));
        };
        let s = self.session().await?;
        let client = match &s.source {
            crate::source::Source::Firehouse(c) => c,
            crate::source::Source::Simulee => {
                return Err(Erreur::Refus("En mode simulé, rien n'est sauvegardé chez Firehouse.".into()))
            }
        };
        // Une ludothèque jamais synchronisée (profil neuf, ou vidée par un changement de compte) ne dit pas quels jeux
        // ce profil voit : ses parties sortiraient toutes de la version courante. On attend la synchronisation.
        if s.verrou().synchronise_le()?.is_none() {
            return Err(Erreur::Refus(
                "Synchronise d'abord ta ludothèque (🔄) : la sauvegarde a besoin de savoir quels jeux tu vois.".into(),
            ));
        }
        let contenu = self.contenu_pour(&s, programmes_emulateurs, true)?;
        let api = Api { client, profil: nom_pour_api(&s.profil.nom), pc: nom_pour_api(pc), morceau: MORCEAU, pause: en_partie };
        let premiere = !self.fichier_derniere_sauvegarde(&s.profil.id).is_file() && !self.reprise_de(&s.profil.id, &s.profil.nom, pc);
        if premiere && ecarter {
            self.marquer_reprise("ecartee", &s.profil.nom, pc).await?;
            self.journaliser(&format!("profil {} : sauvegarde existante chez Firehouse écartée par la personne", s.profil.id));
        } else if premiere {
            match verifier_reprise(&api).await {
                Ok(()) | Err(Erreur::Introuvable(_)) => {}
                Err(e) => return Err(e),
            }
        }
        let fichier_cache = self.fichier_cache_empreintes(&s.profil.id);
        let mut cache = CacheEmpreintes::lire(&fichier_cache);
        let r = envoyer_sous(&api, &contenu, pc, "Firehouse", &mut cache, plafond_auto).await;
        // Les empreintes calculées valent même après un arrêt (le cache n'oublie rien tant qu'il n'a pas tout vu).
        if let Err(e) = cache.ecrire(&fichier_cache) {
            self.journaliser(&format!("sauvegarde du profil {} : cache des empreintes non écrit ({})", s.profil.id, classe(&e)));
        }
        let bilan = match r {
            Ok(b) => b,
            // Firehouse sans les routes de sauvegarde (avant le contrat 1.5).
            Err(Erreur::Introuvable(_)) => {
                return Err(Erreur::Refus(
                    "Firehouse ne propose pas encore la sauvegarde (il faut une version plus récente).".into(),
                ))
            }
            Err(e) => {
                // Jamais un chemin ni un titre au journal commun (lot S4) : l'id du profil et la classe de l'erreur.
                self.journaliser(&format!("sauvegarde du profil {} : arrêtée ({})", s.profil.id, classe(&e)));
                return Err(e);
            }
        };
        self.journaliser(&format!(
            "sauvegarde du profil {} : {} fichier(s) envoyé(s), {} inchangé(s), {} retiré(s), {} sauté(s), {} octet(s){}",
            s.profil.id,
            bilan.envoyes,
            bilan.inchanges,
            bilan.retires,
            bilan.sautes,
            bilan.octets,
            // Jamais la valeur du sel : seulement qu'il attend.
            if bilan.sel_en_attente { " ; clé du profil pas encore rangée chez Firehouse (réessayée)" } else { "" }
        ));
        let f = self.fichier_derniere_sauvegarde(&s.profil.id);
        if let Some(p) = f.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::write(f, serde_json::to_vec_pretty(&bilan).unwrap())?;
        Ok(bilan)
    }

    /// Les captures d'écran du profil (✅ Seb 12/11 : « les screens, comme les saves ») : chaque image part avec sa
    /// fiche à côté, écrite dans `profils\<id>\fiches-captures\`. Rend aussi ce qui reste tel quel chez Firehouse :
    /// une capture d'un jeu que ce profil ne voit plus, ou dont l'image manque (disque débranché) — jamais retirée
    /// de la sauvegarde pour autant.
    /// `ecrire` faux (une estimation) : les fiches restent en mémoire (`en_memoire`), rien n'est écrit ni retiré.
    fn captures_a_sauvegarder(&self, s: &crate::noyau::Session, profil: &str, ecrire: bool, en_memoire: &mut HashMap<PathBuf, Vec<u8>>) -> Resultat<(Vec<Element>, Vec<String>)> {
        let dossier_profil = self.dossier.join("profils").join(profil);
        let fiches = dossier_profil.join(FICHES_CAPTURES);
        let fonds = crate::captures::fonds_choisis(&dossier_profil);
        let (mut parties, mut gardes) = (Vec::new(), Vec::new());
        let mut ecrites = std::collections::BTreeSet::new();
        let mut cles = std::collections::BTreeSet::new();
        for n in crate::captures::index(&self.dossier).into_iter().filter(|n| n.profil == profil) {
            // Une clé déjà prise par une autre capture : la sienne propre (jamais deux images à la même place).
            let cle = if cles.insert(n.cle_sauvegarde().to_string()) { n.cle_sauvegarde().to_string() } else { n.id.clone() };
            cles.insert(cle.clone());
            let base = format!("captures/{}/{cle}", n.jeu);
            // Comme la galerie : un jeu de Firehouse caché à ce profil (grade, réunion) ne part pas.
            if n.jeu > 0 && !s.verrou().visible_ou_reuni(n.jeu).unwrap_or(false) {
                gardes.push(format!("captures/{}/", n.jeu));
                continue;
            }
            let image = PathBuf::from(&n.chemin);
            if !image.is_file() {
                gardes.push(format!("{base}."));
                continue;
            }
            let ext = image.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_else(|| "png".into());
            let fond = self.fiche_de(s, n.jeu).ok().flatten().is_some_and(|f| fonds.get(&f.to_string()) == Some(&n.id));
            // Les noms des dossiers où elle était rangée (`<Système>\<jeu>\`) : la restauration la remet sous les mêmes.
            let nom_de = |p: Option<&Path>| p.and_then(|p| p.file_name()).map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
            let fiche = json!({
                "jeu": n.jeu, "titre": n.titre, "plateforme": n.plateforme, "quand": n.quand,
                "nom": nom_de(Some(&image)), "dossier_jeu": nom_de(image.parent()), "systeme": nom_de(image.parent().and_then(|p| p.parent())),
                "fond": fond,
            });
            let f = fiches.join(format!("{cle}.json"));
            let octets = serde_json::to_vec_pretty(&fiche).unwrap_or_default();
            // Réécrite seulement si elle a changé (son empreinte ne bouge pas : rien n'est renvoyé).
            poser_copie(&f, &octets, ecrire, en_memoire)?;
            ecrites.insert(f.clone());
            parties.push(Element { source: image, relatif: format!("{base}.{ext}") });
            parties.push(Element { source: f, relatif: format!("{base}.json") });
        }
        // Les fiches qui ne partent plus (capture supprimée, jeu caché, image absente) : réécrites au besoin, celles déjà
        // chez Firehouse y restent (gardes).
        for e in std::fs::read_dir(&fiches).into_iter().flatten().flatten().filter(|_| ecrire) {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "json") && !ecrites.contains(&p) {
                let _ = std::fs::remove_file(&p);
            }
        }
        Ok((parties, gardes))
    }

    /// Rassemble ce qu'il faut sauvegarder pour le profil ouvert. `programmes_emulateurs` : les programmes des
    /// émulateurs réglés sur ce PC (leurs dossiers portent les dossiers des profils).
    pub async fn contenu_sauvegarde(&self, programmes_emulateurs: &[String]) -> Resultat<Contenu> {
        self.contenu_sauvegarde_avec(programmes_emulateurs, true).await
    }

    /// `contenu_sauvegarde` ; `ecrire` faux (une estimation) : les copies que Frogtend écrirait pour partir (fiches des
    /// captures et des abris, réglages de Ryubing sans secret) restent en mémoire (`Contenu::en_memoire`).
    pub async fn contenu_sauvegarde_avec(&self, programmes_emulateurs: &[String], ecrire: bool) -> Resultat<Contenu> {
        let s = self.session().await?;
        self.contenu_pour(&s, programmes_emulateurs, ecrire)
    }

    /// `contenu_sauvegarde_avec` pour une session déjà lue (lot S4 : une seule lecture par sauvegarde ou estimation).
    pub fn contenu_pour(&self, s: &crate::noyau::Session, programmes_emulateurs: &[String], ecrire: bool) -> Resultat<Contenu> {
        let profil = s.profil.clone();
        let mut en_memoire: HashMap<PathBuf, Vec<u8>> = HashMap::new();
        let configuration = json!({
            "version_frogtend": env!("CARGO_PKG_VERSION"),
            "profil": { "id": profil.id, "nom": profil.nom, "protege": profil.protege },
            "reglages_pc": lire_json(&self.dossier.join("pc.json")),
            "reglages_profil": lire_json(&self.dossier.join("profils").join(format!("{}.json", profil.id))),
            "emulateurs_installes": lire_json(&self.dossier.join("emulateurs.json")),
        });

        // Jamais le jeu importé par une AUTRE personne de ce PC (contrat 1.21 : ce profil peut seulement y jouer).
        let jeux: Vec<_> = self.jeux_du_pc_de(s)?.into_iter().filter(|v| !v.d_un_autre).collect();
        // Les jeux du PC que ce profil ne voit pas : leurs parties ne partent pas, ce qui en avait été envoyé reste.
        let visibles: std::collections::BTreeSet<i64> = jeux.iter().map(|v| v.jeu.id).collect();
        let mut gardes: std::collections::BTreeSet<i64> =
            self.registre().ids()?.into_iter().filter(|id| *id > 0 && !visibles.contains(id)).collect();
        let mut parties = Vec::new();
        let mut liste = Vec::new();
        for vu in &jeux {
            let j = &vu.jeu;
            // Un jeu à cartes (e-Reader) ou à plusieurs régions : TOUTES ses versions, pour les remettre à une
            // restauration (pas seulement la 1re).
            // Jamais une version en erreur (une restauration la retéléchargerait) ; le total suit ce qui est remis.
            let en_erreur: Vec<i64> =
                self.registre().versions_ajoutees(j.id).unwrap_or_default().into_iter().filter(|v| v.etat == crate::jeux_pc::Etat::Erreur).map(|v| v.version).collect();
            let cartes: Vec<i64> = if crate::jeux_pc::a_cartes(&j.plateforme) || crate::partie::plusieurs_versions(j) {
                let mut v: Vec<i64> = j.fichiers.iter().map(|f| f.version.unwrap_or(j.version)).filter(|v| !en_erreur.contains(v)).collect();
                v.sort();
                v.dedup();
                v
            } else {
                vec![]
            };
            let total: u64 = j.fichiers.iter().filter(|f| !en_erreur.contains(&f.version.unwrap_or(j.version))).map(|f| f.taille).sum();
            liste.push(json!({
                "id": j.id, "version": j.version, "titre": j.titre, "plateforme": j.plateforme,
                "dossier": j.dossier, "etat": j.etat, "total": total, "installation": j.installation,
                "temps_jeu": j.temps_jeu, "derniere_partie": j.derniere_partie, "cartes": cartes,
            }));
            // Les parties d'un jeu installé : ce qui a changé depuis son installation.
            let Some(i) = &j.installation else { continue };
            let Ok(o) = std::fs::read(self.dossier_medias(j.id).join("manifeste.json")) else { continue };
            let Ok(m) = serde_json::from_slice::<Manifeste>(&o) else { continue };
            let racine = PathBuf::from(&i.dossier);
            if !racine.is_dir() {
                continue;
            }
            for r in crate::locale::parties_changees(j, &racine, &m)? {
                parties.push(Element { source: racine.join(&r), relatif: format!("jeux/{}/{r}", j.id) });
            }
        }

        // Les parties MISES À L'ABRI (avant un retrait ou une réinstallation) : la copie la plus récente de chaque jeu,
        // même s'il n'est plus sur le PC — c'est parfois la seule copie qui reste. Elle va au même endroit que les
        // parties d'un jeu installé (`jeux/<id>/…`), et sera reposée à la réinstallation. Un fichier déjà pris dans
        // le jeu installé (plus récent) passe avant.
        // Les abris peuvent être à deux endroits (le dossier réglé, l'ancien emplacement) : pour chaque jeu, la copie la
        // plus récente de TOUS les endroits.
        // Un abri rangé sous l'ANCIEN numéro d'un jeu rattaché à sa nouvelle fiche (contrat 1.14) compte pour le
        // nouveau (il n'est ni déplacé ni renommé sur le disque) : les deux sont RÉUNIS, la copie la plus récente
        // primant fichier par fichier (une partie présente seulement dans l'ancien abri ne sort jamais de la sauvegarde).
        let reunions: std::collections::HashMap<i64, i64> = self.registre().reunions()?.into_iter().collect();
        let mut plus_recente: std::collections::BTreeMap<i64, (u64, PathBuf, bool)> = std::collections::BTreeMap::new();
        // Lot S2 : TOUS les abris de chaque jeu, pour en tirer les emplacements de ce profil (`emplacements_a_envoyer`).
        let mut tous_les_abris: std::collections::BTreeMap<i64, Vec<(u64, PathBuf)>> = std::collections::BTreeMap::new();
        // Lot S4 : les numéros (bruts) des jeux dont un abri appartient à CE profil (sa fiche le dit).
        let mut a_moi: BTreeSet<i64> = BTreeSet::new();
        let racines = self.dossiers_abris_a_lire();
        for racine in &racines {
            let Ok(jeux) = std::fs::read_dir(racine) else { continue };
            for d in jeux.flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
                let Some(id) = d.file_name().and_then(|n| n.to_str()).and_then(|n| n.parse::<i64>().ok()) else { continue };
                for date in std::fs::read_dir(&d).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
                    let quand = date.file_name().and_then(|n| n.to_str()).and_then(|n| n.parse::<u64>().ok()).unwrap_or(0);
                    let fiche = crate::abri_parties::lire_fiche(&date);
                    if fiche.as_ref().is_some_and(|f| !f.profil.is_empty() && f.profil == profil.id) {
                        a_moi.insert(id);
                    }
                    // Lot S4 (Seb, 23/11) : TOUS les abris entrent ici — ceux de sorte `emplacements` (à la main, après
                    // une partie, avant un retrait…) partent par `emplacements_a_envoyer`, avec leur propriétaire.
                    tous_les_abris.entry(id).or_default().push((quand, date.clone()));
                    // … mais jamais dans `plus_recente` : leurs parties ne sont pas dans le dossier du jeu (la remise
                    // poserait `emplacements/` dans le jeu), et ils ne cachent jamais l'abri d'avant (la seule copie d'un
                    // jeu retiré, parfois).
                    if fiche.as_ref().is_some_and(|f| f.sorte == crate::abri_parties::SORTE_EMPLACEMENTS) {
                        continue;
                    }
                    // Lot S4 (relecture lancement) : un abri du dossier du jeu (sorte `jeu`) ou du nuage Steam dont la fiche dit
                    // un AUTRE profil est noté comme tel : s'il est le plus récent, ce profil n'envoie rien de ce jeu et
                    // GARDE sa copie déjà chez Firehouse (jamais un recul vers un abri plus ancien, jamais un retrait). Un
                    // ancien abri sans fiche (ou sans propriétaire) : comme avant.
                    let d_un_autre = fiche.as_ref().is_some_and(|f| {
                        (f.sorte == crate::abri_parties::SORTE_JEU || f.sorte == crate::abri_parties::SORTE_NUAGE_STEAM) && !f.profil.is_empty() && f.profil != profil.id
                    });
                    if plus_recente.get(&id).is_none_or(|(q, _, _)| quand > *q) {
                        plus_recente.insert(id, (quand, date, d_un_autre));
                    }
                }
            }
        }
        // Seulement les jeux que CE profil a le droit de voir (brief § 2 : les abris sont ceux du PC, pas du profil ;
        // après un passage admin → mineur, ceux des jeux cachés ne partent jamais dans l'espace du mineur). Un abri
        // qui ne part plus reste sur le PC, et l'historique de Firehouse garde les envois d'avant.
        let mut propres: BTreeSet<i64> = BTreeSet::new();
        let mut par_jeu: std::collections::BTreeMap<i64, Vec<(u64, PathBuf)>> = std::collections::BTreeMap::new();
        for (brut, (quand, abri, d_un_autre)) in plus_recente {
            let id = reunions.get(&brut).copied().unwrap_or(brut);
            if d_un_autre {
                gardes.insert(id);
                continue;
            }
            let abri = (quand, abri);
            if !s.verrou().visible_ou_reuni(id)? {
                gardes.insert(id);
                if a_moi.contains(&brut) {
                    propres.insert(id);
                }
                continue;
            }
            par_jeu.entry(id).or_default().push(abri);
        }
        // Lot S4 : les jeux dont partent des EMPLACEMENTS viennent de TOUS les abris (un jeu GOG ou Epic n'a souvent que
        // des abris de sorte `emplacements`) ; un jeu invisible n'est gardé (et compté) que si l'abri est à CE profil.
        let mut ids_emplacements: BTreeSet<i64> = BTreeSet::new();
        for brut in tous_les_abris.keys() {
            let id = reunions.get(brut).copied().unwrap_or(*brut);
            if s.verrou().visible_ou_reuni(id)? {
                ids_emplacements.insert(id);
            } else if a_moi.contains(brut) {
                gardes.insert(id);
                propres.insert(id);
            }
        }
        // Les parties du nuage Steam (lot B2) d'un AUTRE compte Steam du PC ne partent jamais avec ce profil : seulement
        // celles du compte relié à ce profil (sans compte relié : celles qu'il a mises à l'abri, un seul compte).
        let marque_steam = crate::boutiques::lire(&self.dossier.join("profils").join(&profil.id))
            .and_then(|c| crate::nuage_steam::compte(&c.steamid))
            .map(|c| crate::nuage_steam::marque(&c));
        let d_un_autre_compte = |r: &str| {
            r.strip_prefix("nuage-steam/").and_then(|x| x.split('/').next()).is_some_and(|m| marque_steam.as_deref().is_some_and(|s| s != m))
        };
        // Les chemins déjà pris (jamais deux fois le même ; « et sur 50 000 fichiers ? » : un ensemble, pas une liste).
        let mut pris: std::collections::HashSet<String> = parties.iter().map(|e| e.relatif.clone()).collect();
        for (id, mut abris) in par_jeu {
            abris.sort_by(|a, b| b.0.cmp(&a.0));
            for (_, recente) in abris {
                // `emplacements/` et `nuage-steam/` ne sont à part que dans un abri qui le dit (un jeu dont le dossier a
                // un sous-dossier `emplacements` reste un jeu) ; les emplacements partent plus bas, par profil.
                let reservees = crate::abri_parties::reservees(&recente);
                for r in fichiers_de(&recente)? {
                    if (reservees.nuage && d_un_autre_compte(&r)) || (reservees.emplacements && r.starts_with(crate::abri_parties::PREFIXE)) {
                        continue;
                    }
                    let relatif = format!("jeux/{id}/{r}");
                    if pris.insert(relatif.clone()) {
                        parties.push(Element { source: recente.join(&r), relatif });
                    }
                }
            }
        }
        // Lots S2 et S4 : les parties prises hors du dossier du jeu (`emplacements/` : jeux Steam hors du nuage, et TOUS
        // les abris de sorte `emplacements` — Seb, 23/11) ne partent qu'avec le profil propriétaire de l'abri, chaque
        // emplacement depuis le plus récent de SES abris qui le contient (l'abri plus récent d'un autre profil, ou un
        // abri où il manque, ne le fait jamais sortir de sa sauvegarde) ; avec elles, la fiche fusionnée
        // (`jeux/<id>/emplacements/fiche.json` : chemins NON résolus, pour les remettre au lot S5), envoyée AVANT eux.
        let fiches = self.dossier.join("profils").join(&profil.id).join("fiches-abris");
        for id in ids_emplacements {
            let abris: Vec<(u64, PathBuf)> = tous_les_abris.iter().filter(|(a, _)| reunions.get(*a).copied().unwrap_or(**a) == id).flat_map(|(_, l)| l.iter().cloned()).collect();
            let (fichiers, fiche) = crate::abri_parties::emplacements_a_envoyer(&abris, &profil.id);
            let Some(mut fiche) = fiche else { continue };
            // Pour le lot S5, sur un autre PC : la boutique du jeu, quand il en vient une — celle du registre, sinon celle
            // notée dans l'abri quand il a été fait (un jeu retiré depuis).
            let du_registre = self.registre().jeu(id).ok().flatten().and_then(|j| j.installation).and_then(|i| i.boutique).map(|b| crate::abri_parties::BoutiqueDuJeu { source: b.source, cle: b.cle });
            fiche.boutique = du_registre.or(fiche.boutique);
            let f = fiches.join(format!("{id}.json"));
            let octets = serde_json::to_vec_pretty(&fiche).map_err(|_| Erreur::Disque("La fiche des emplacements est impossible à écrire.".into()))?;
            poser_copie(&f, &octets, ecrire, &mut en_memoire)?;
            let relatif_fiche = format!("jeux/{id}/{}", crate::abri_parties::FICHE_ENVOYEE);
            if !pris.insert(relatif_fiche.clone()) {
                continue;
            }
            parties.push(Element { source: f, relatif: relatif_fiche });
            for (source, r) in fichiers {
                let relatif = format!("jeux/{id}/{r}");
                if pris.insert(relatif.clone()) {
                    parties.push(Element { source, relatif });
                }
            }
        }

        // Les parties du profil dans chaque émulateur.
        let mut vus = std::collections::BTreeSet::new();
        let mut gardes_ouverts: Vec<String> = Vec::new();
        for programme in programmes_emulateurs {
            let Some(dossier) = Path::new(programme).parent() else { continue };
            if !vus.insert(dossier.to_path_buf()) {
                continue;
            }
            let nom_emulateur = crate::jeux_pc::nom_de_dossier(&dossier.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
            // RPCS3 : les parties et les trophées du compte RPCS3 de ce profil (`dev_hdd0\home\<compte>`), lus SANS créer
            // le compte ; jamais `exdata` (les licences). Rangés sans le numéro du compte : la restauration les remet
            // dans le compte du profil sur le nouveau PC, quel que soit son numéro.
            // Un émulateur OUVERT peut être en train d'écrire une partie : ses parties de compte sont sautées cette fois
            // (jamais une partie à moitié écrite envoyée), et celles déjà chez Firehouse y restent (gardées).
            let ouvert = (crate::emulateurs_profils::est_rpcs3(programme)
                || crate::eden::est_eden(programme)
                || crate::cemu::est_cemu(programme)
                || crate::vita3k::est_vita3k(programme))
                && !crate::lancement::processus_du_dossier(dossier).is_empty();
            if ouvert {
                for sous in [crate::emulateurs_profils::COMPTE_RPCS3, crate::eden::UTILISATEUR_EDEN, crate::cemu::COMPTE_CEMU, crate::vita3k::UTILISATEUR_VITA] {
                    gardes_ouverts.push(format!("emulateurs/{nom_emulateur}/{sous}/"));
                }
            }
            if !ouvert && crate::emulateurs_profils::est_rpcs3(programme) {
                if let Some(compte) = crate::emulateurs_profils::compte_rpcs3_existant(dossier, &crate::jeux_pc::nom_de_dossier(&profil.nom)) {
                    let home = dossier.join("dev_hdd0").join("home").join(compte);
                    for sous in crate::emulateurs_profils::PARTIES_RPCS3 {
                        let d = home.join(sous);
                        if !d.is_dir() {
                            continue;
                        }
                        for r in fichiers_de(&d)? {
                            parties.push(Element {
                                source: d.join(&r),
                                relatif: format!("emulateurs/{nom_emulateur}/{}/{sous}/{r}", crate::emulateurs_profils::COMPTE_RPCS3),
                            });
                        }
                    }
                }
            }
            // Eden (portable) : les parties de l'utilisateur Switch de ce profil, lu SANS le créer ; rangées sans son UUID
            // (la restauration les remet dans l'utilisateur du profil sur le nouveau PC).
            // Cemu (portable) : les parties du compte Wii U de ce profil (lu SANS le créer), jeu par jeu, avec les données
            // communes de chacun de ses jeux (`user\common`, comme sur la vraie Wii U).
            if !ouvert && crate::cemu::est_cemu(programme) && crate::cemu::portable(dossier) {
                if let Some(id) = crate::cemu::compte_existant(dossier, &crate::jeux_pc::nom_de_dossier(&profil.nom)) {
                    for (jeu, d) in crate::cemu::parties_du_compte(dossier, id) {
                        let commun = d.parent().map(|u| u.join("common")).unwrap_or_default();
                        for (sous, src) in [("compte", d), ("common", commun)] {
                            if !src.is_dir() {
                                continue;
                            }
                            for r in fichiers_de(&src)? {
                                parties.push(Element { source: src.join(&r), relatif: format!("emulateurs/{nom_emulateur}/{}/{jeu}/{sous}/{r}", crate::cemu::COMPTE_CEMU) });
                            }
                        }
                    }
                }
            }
            // Vita3K (portable) : les parties de l'utilisateur Vita de ce profil (lu SANS le créer).
            if !ouvert && crate::vita3k::est_vita3k(programme) && crate::vita3k::portable(dossier) {
                if let Some(id) = crate::vita3k::utilisateur_existant(dossier, &crate::jeux_pc::nom_de_dossier(&profil.nom)) {
                    let d = crate::vita3k::parties(dossier, &id);
                    if d.is_dir() {
                        for r in fichiers_de(&d)? {
                            parties.push(Element { source: d.join(&r), relatif: format!("emulateurs/{nom_emulateur}/{}/savedata/{r}", crate::vita3k::UTILISATEUR_VITA) });
                        }
                    }
                }
            }
            if !ouvert && crate::eden::est_eden(programme) && crate::eden::portable(dossier) {
                if let Some(uuid) = crate::eden::utilisateur_existant(dossier, &crate::jeux_pc::nom_de_dossier(&profil.nom)) {
                    // Avec les parties « Device », communes à tous (comme sur la vraie Switch).
                    let device = [("device", crate::eden::dossier_device(dossier))];
                    for (sorte, d) in crate::eden::dossiers_parties(dossier, &uuid).into_iter().chain(device) {
                        if !d.is_dir() {
                            continue;
                        }
                        for r in fichiers_de(&d)? {
                            // Les parties communes : seulement celles des JEUX (jamais une donnée système).
                            if sorte == "device" && !r.split('/').next().is_some_and(crate::eden::est_un_jeu) {
                                continue;
                            }
                            parties.push(Element { source: d.join(&r), relatif: format!("emulateurs/{nom_emulateur}/{}/{sorte}/{r}", crate::eden::UTILISATEUR_EDEN) });
                        }
                    }
                }
            }
            let du_profil = crate::emulateurs_profils::dossier_du_profil(dossier, &profil.nom);
            if !du_profil.is_dir() {
                continue;
            }
            // Ryubing : `Profils\<nom>\` est TOUTE sa racine de données (`-r`), avec des jonctions vers le firmware et
            // les caches communs. Seuls partent ses parties, leur index, ses utilisateurs et ses réglages
            // (`ryubing::a_sauvegarder`) — jamais les clés, le firmware ni les caches. Ouvert : rien de lui cette fois
            // (une partie ou l'index peuvent être en cours d'écriture), ce qui est déjà chez Firehouse y reste.
            if crate::ryubing::est_ryubing(programme) {
                if !crate::lancement::processus_du_dossier(dossier).is_empty() {
                    gardes_ouverts.push(format!("emulateurs/{nom_emulateur}/"));
                    continue;
                }
                for r in crate::ryubing::fichiers_a_sauvegarder(&du_profil)?.into_iter().filter(|r| !crate::emulateurs_profils::porte_un_secret(r)) {
                    let mut source = du_profil.join(&r);
                    // Ses réglages partent par une COPIE sans le mot de passe du réseau local (jamais le fichier de la
                    // personne modifié), rangée chez Frogtend, dans le dossier de CE profil ; illisibles, ils ne partent pas.
                    if r == crate::ryubing::FICHIER_CONFIG {
                        let Some(copie) = std::fs::read_to_string(&source).ok().and_then(|t| crate::ryubing::config_a_envoyer(&t)) else { continue };
                        let f = self.dossier.join("profils").join(&profil.id).join("copies-sauvegarde").join(&nom_emulateur).join(&r);
                        poser_copie(&f, copie.as_bytes(), ecrire, &mut en_memoire)?;
                        source = f;
                    }
                    parties.push(Element { source, relatif: format!("emulateurs/{nom_emulateur}/{r}") });
                }
                continue;
            }
            // Sans les fichiers qui portent une connexion RetroAchievements (jeton) : ils restent sur ce PC.
            // Ni une sauvegarde de MAME en cours d'écriture (elle n'est gardée qu'une fois complète), ni une sauvegarde
            // rapide de BigPEmu mise de côté le temps d'une opération (remise en place au prochain lancement).
            for r in fichiers_de(&du_profil)?
                .into_iter()
                .filter(|r| !crate::emulateurs_profils::porte_un_secret(r) && !r.ends_with(crate::pilotage::ETAT_MAME_NOUVEAU) && !r.ends_with(".frogtend-de-cote"))
            {
                parties.push(Element { source: du_profil.join(&r), relatif: format!("emulateurs/{nom_emulateur}/{r}") });
            }
        }

        let (captures, gardes_captures) = self.captures_a_sauvegarder(&s, &profil.id, ecrire, &mut en_memoire)?;
        parties.extend(captures);

        // Ce qui ATTEND encore d'une restauration (PC formaté ou neuf : jeu pas encore remis, émulateur pas encore
        // installé, capture pas encore reposée) n'est pas sur le PC mais n'est pas perdu : protégé chez Firehouse,
        // jamais retiré de la sauvegarde.
        let attente = self.dossier.join("restauration").join(&profil.id);
        let invisibles = gardes.clone();
        let mut gardes: Vec<String> = gardes.into_iter().map(|id| format!("jeux/{id}/")).collect();
        gardes.extend(gardes_ouverts);
        gardes.extend(gardes_captures);
        for jeu in std::fs::read_dir(attente.join("parties").join("captures")).into_iter().flatten().flatten() {
            for f in std::fs::read_dir(jeu.path()).into_iter().flatten().flatten() {
                gardes.push(format!("captures/{}/{}", jeu.file_name().to_string_lossy(), f.file_name().to_string_lossy()));
            }
        }
        for sorte in ["jeux", "emulateurs"] {
            for e in std::fs::read_dir(attente.join("parties").join(sorte)).into_iter().flatten().flatten() {
                if e.file_type().is_ok_and(|t| t.is_dir()) {
                    gardes.push(format!("{sorte}/{}/", e.file_name().to_string_lossy()));
                }
            }
        }
        gardes.sort();
        gardes.dedup();
        // Les jeux de la sauvegarde restaurée pas encore remis restent dans sa liste (à remettre plus tard).
        let restauree = lire_json(&attente.join("bibliotheque.json"));
        let ici: std::collections::BTreeSet<i64> = liste.iter().filter_map(|j| j["id"].as_i64()).collect();
        for j in restauree["jeux"].as_array().into_iter().flatten() {
            if j["id"].as_i64().is_some_and(|id| !ici.contains(&id)) {
                liste.push(j.clone());
            }
        }

        // Le sel du profil (Seb, 23/11 : envoyé, pour restaurer sur un autre PC sans question) — lu, jamais créé ici.
        let sel = std::fs::read_to_string(self.dossier.join("profils").join(&profil.id).join(crate::abri_parties::NOM_SEL))
            .ok()
            .map(|t| t.trim().to_string())
            .filter(|t| t.len() >= 16 && t.bytes().all(|b| b.is_ascii_hexdigit()));

        Ok(Contenu {
            configuration,
            bibliotheque: json!({ "profil": profil.nom, "jeux": liste }),
            parties,
            gardes,
            invisibles,
            propres,
            sel,
            en_memoire,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contenu(parties: Vec<Element>) -> Contenu {
        Contenu { configuration: json!({"a": 1}), bibliotheque: json!({"jeux": []}), parties, ..Default::default() }
    }

    #[tokio::test]
    async fn une_sauvegarde_existante_n_est_jamais_videe_par_un_profil_neuf() {
        let pc = tempfile::tempdir().unwrap();
        let partage = tempfile::tempdir().unwrap();
        let d = DossierLocal(partage.path().into());
        // Rien chez Firehouse : la première sauvegarde part.
        verifier_reprise(&d).await.unwrap();
        let a = pc.path().join("PARTIE.SAV");
        std::fs::write(&a, b"un").unwrap();
        envoyer(&d, &contenu(vec![Element { source: a, relatif: "jeux/7/PARTIE.SAV".into() }]), "VENKMAN", "test").await.unwrap();
        // Un profil recréé au même nom, sur le même PC, sans ses parties : refusé (sinon PARTIE.SAV sortirait).
        let e = verifier_reprise(&d).await.unwrap_err();
        assert!(format!("{e:?}").contains(A_REPRENDRE), "{e:?}");
        assert!(partage.path().join("parties/jeux/7/PARTIE.SAV").is_file());
    }

    #[tokio::test]
    async fn les_parties_d_un_jeu_devenu_invisible_restent_dans_la_version_courante() {
        let pc = tempfile::tempdir().unwrap();
        let partage = tempfile::tempdir().unwrap();
        let a = pc.path().join("PARTIE.SAV");
        std::fs::write(&a, b"un").unwrap();
        let el = Element { source: a.clone(), relatif: "jeux/7/PARTIE.SAV".into() };
        envoyer(&DossierLocal(partage.path().into()), &contenu(vec![el]), "VENKMAN", "test").await.unwrap();
        // Le jeu 7 n'est plus visible pour ce compte (retiré de Firehouse, caché) : sa partie ne part plus, mais ce qui
        // en avait été envoyé reste protégé (jamais retiré), et le bilan le compte sans le nommer.
        let mut c = contenu(vec![]);
        c.gardes = vec!["jeux/7/".into()];
        c.invisibles = [7].into();
        let b = envoyer(&DossierLocal(partage.path().into()), &c, "VENKMAN", "test").await.unwrap();
        assert_eq!((b.envoyes, b.retires, b.fichiers, b.non_envoyes), (0, 0, 1, 1));
        assert_eq!(std::fs::read(partage.path().join("parties/jeux/7/PARTIE.SAV")).unwrap(), b"un");
        // Gardé aussi au passage suivant (le manifeste le porte toujours).
        let b = envoyer(&DossierLocal(partage.path().into()), &c, "VENKMAN", "test").await.unwrap();
        assert_eq!((b.retires, b.fichiers), (0, 1));
    }

    #[tokio::test]
    async fn seul_ce_qui_a_change_est_renvoye_et_ce_qui_a_disparu_sort_de_la_version_courante() {
        let pc = tempfile::tempdir().unwrap();
        let partage = tempfile::tempdir().unwrap();
        let a = pc.path().join("PARTIE1.SAV");
        let b = pc.path().join("PARTIE2.SAV");
        std::fs::write(&a, b"un").unwrap();
        std::fs::write(&b, b"deux").unwrap();
        let el = |p: &Path, r: &str| Element { source: p.into(), relatif: r.into() };

        let b1 = envoyer(&DossierLocal(partage.path().into()), &contenu(vec![el(&a, "jeux/110/PARTIE1.SAV"), el(&b, "jeux/110/PARTIE2.SAV")]), "VENKMAN", "test").await.unwrap();
        assert_eq!((b1.envoyes, b1.inchanges, b1.fichiers, b1.taille), (2, 0, 2, 6)); // « un » + « deux » = 6 octets
        assert_eq!(std::fs::read(partage.path().join("parties/jeux/110/PARTIE1.SAV")).unwrap(), b"un");

        // Rien n'a changé : rien n'est renvoyé.
        let b2 = envoyer(&DossierLocal(partage.path().into()), &contenu(vec![el(&a, "jeux/110/PARTIE1.SAV"), el(&b, "jeux/110/PARTIE2.SAV")]), "VENKMAN", "test").await.unwrap();
        assert_eq!((b2.envoyes, b2.inchanges, b2.octets), (0, 2, 0));

        // Une partie change, une autre disparaît.
        std::fs::write(&a, b"un, plus loin").unwrap();
        let b3 = envoyer(&DossierLocal(partage.path().into()), &contenu(vec![el(&a, "jeux/110/PARTIE1.SAV")]), "VENKMAN", "test").await.unwrap();
        assert_eq!((b3.envoyes, b3.inchanges, b3.retires), (1, 0, 1));
        assert_eq!(std::fs::read(partage.path().join("parties/jeux/110/PARTIE1.SAV")).unwrap(), b"un, plus loin");
        assert!(!partage.path().join("parties/jeux/110/PARTIE2.SAV").exists());

        // Les documents sont là, sans fichier à moitié écrit.
        for f in ["configuration.json", "bibliotheque.json", "manifeste.json", "derniere-sauvegarde.json"] {
            assert!(partage.path().join(f).is_file(), "{f}");
        }
        assert!(fichiers_de(partage.path()).unwrap().iter().all(|f| !f.ends_with("part-frogtend")));
    }

    #[tokio::test]
    async fn un_fichier_efface_sur_le_serveur_est_renvoye() {
        let pc = tempfile::tempdir().unwrap();
        let partage = tempfile::tempdir().unwrap();
        let a = pc.path().join("x.sav");
        std::fs::write(&a, b"x").unwrap();
        let c = contenu(vec![Element { source: a.clone(), relatif: "jeux/1/x.sav".into() }]);
        envoyer(&DossierLocal(partage.path().into()), &c, "PC", "test").await.unwrap();
        std::fs::remove_file(partage.path().join("parties/jeux/1/x.sav")).unwrap();
        assert_eq!(envoyer(&DossierLocal(partage.path().into()), &c, "PC", "test").await.unwrap().envoyes, 1);
    }

    mod api {
        use super::*;
        use httpmock::prelude::*;
        use httpmock::Method::HEAD;

        fn client(s: &MockServer) -> crate::firehouse::Client {
            crate::firehouse::Client::nouveau(&s.base_url(), "j").unwrap()
        }
        const ROUTE: &str = "/api/jeux/v1/sauvegarde/Sebastien/VENKMAN/fichier";

        #[test]
        fn les_noms_respectent_le_contrat() {
            assert_eq!(nom_pour_api("Sébastien"), "Sebastien");
            assert_eq!(nom_pour_api("Léa & Zoé"), "Lea _ Zoe");
            assert_eq!(nom_pour_api(".."), "profil");
            assert_eq!(nom_pour_api("VENKMAN"), "VENKMAN");
            assert_eq!(nom_pour_api("Seb."), "Seb");
        }

        #[tokio::test]
        async fn un_fichier_part_en_morceaux_avec_son_empreinte() {
            let s = MockServer::start();
            let d = tempfile::tempdir().unwrap();
            let f = d.path().join("PARTIE1.SAV");
            std::fs::write(&f, b"0123456789").unwrap(); // 10 octets, morceaux de 4
            let sha = empreinte(&f).unwrap();
            s.mock(|w, t| {
                w.method(HEAD).path(ROUTE);
                t.status(200).header("X-Recu", "0");
            });
            let m1 = s.mock(|w, t| {
                w.method(PUT).path(ROUTE).query_param("chemin", "jeux/110/PARTIE1.SAV")
                    .header("content-range", "bytes 0-3/10").header("x-contenu-sha256", sha.as_str()).body("0123");
                t.status(200).json_body(serde_json::json!({"recu": 4, "total": 10}));
            });
            let m2 = s.mock(|w, t| {
                w.method(PUT).path(ROUTE).header("content-range", "bytes 4-7/10").body("4567");
                t.status(200).json_body(serde_json::json!({"recu": 8, "total": 10}));
            });
            let m3 = s.mock(|w, t| {
                w.method(PUT).path(ROUTE).header("content-range", "bytes 8-9/10").body("89");
                t.status(200).json_body(serde_json::json!({"ok": true, "complet": true}));
            });
            let c = client(&s);
            let api = Api { client: &c, profil: "Sebastien".into(), pc: "VENKMAN".into(), morceau: 4, pause: None };
            assert_eq!(api.envoyer_fichier("jeux/110/PARTIE1.SAV", &f, &sha, 10).await.unwrap(), 10);
            m1.assert();
            m2.assert();
            m3.assert();
        }

        #[tokio::test]
        async fn un_envoi_coupe_reprend_la_ou_firehouse_en_est() {
            let s = MockServer::start();
            let d = tempfile::tempdir().unwrap();
            let f = d.path().join("etat.state");
            std::fs::write(&f, b"0123456789").unwrap();
            let sha = empreinte(&f).unwrap();
            // Firehouse a déjà 4 octets du .part : on reprend à 4.
            s.mock(|w, t| {
                w.method(HEAD).path(ROUTE);
                t.status(200).header("X-Recu", "4");
            });
            // Le morceau 4-7 est refusé : en fait Firehouse en est à 8 (409 {recu}).
            let decale = s.mock(|w, t| {
                w.method(PUT).path(ROUTE).header("content-range", "bytes 4-7/10");
                t.status(409).json_body(serde_json::json!({"recu": 8}));
            });
            let fin = s.mock(|w, t| {
                w.method(PUT).path(ROUTE).header("content-range", "bytes 8-9/10").body("89");
                t.status(200).json_body(serde_json::json!({"ok": true, "complet": true}));
            });
            let c = client(&s);
            let api = Api { client: &c, profil: "Sebastien".into(), pc: "VENKMAN".into(), morceau: 4, pause: None };
            assert_eq!(api.envoyer_fichier("x", &f, &sha, 10).await.unwrap(), 2);
            decale.assert();
            fin.assert();
        }

        #[tokio::test]
        async fn un_envoi_qui_ne_se_cale_pas_apres_trois_reprises_est_dit() {
            let s = MockServer::start();
            let d = tempfile::tempdir().unwrap();
            let f = d.path().join("x");
            std::fs::write(&f, b"0123456789").unwrap();
            s.mock(|w, t| {
                w.method(HEAD).path(ROUTE);
                t.status(200).header("X-Recu", "0");
            });
            // Firehouse répond toujours « reprends à 4 » : 3 reprises au plus, puis c'est dit (jamais de boucle).
            let m = s.mock(|w, t| {
                w.method(PUT).path(ROUTE);
                t.status(409).json_body(serde_json::json!({"recu": 4}));
            });
            let c = client(&s);
            let api = Api { client: &c, profil: "Sebastien".into(), pc: "VENKMAN".into(), morceau: 4, pause: None };
            let r = api.envoyer_fichier("x", &f, "sha", 10).await;
            assert!(matches!(&r, Err(Erreur::Conflit(m)) if m.contains("ne se cale pas")), "{r:?} (lot S4 : sauté, jamais l arrêt de tout)");
            m.assert_hits(4);
        }

        #[tokio::test]
        async fn une_empreinte_refusee_deux_fois_est_dite() {
            let s = MockServer::start();
            let d = tempfile::tempdir().unwrap();
            let f = d.path().join("x");
            std::fs::write(&f, b"abc").unwrap();
            s.mock(|w, t| {
                w.method(HEAD).path(ROUTE);
                t.status(404);
            });
            let m = s.mock(|w, t| {
                w.method(PUT).path(ROUTE);
                t.status(409).json_body(serde_json::json!({"detail": "empreinte différente"}));
            });
            let c = client(&s);
            let api = Api { client: &c, profil: "Sebastien".into(), pc: "VENKMAN".into(), morceau: 4, pause: None };
            assert!(matches!(api.envoyer_fichier("x", &f, "faux", 3).await, Err(Erreur::Conflit(_))));
            m.assert_hits(2); // un seul nouvel essai complet
        }

        #[tokio::test]
        async fn une_sauvegarde_complete_par_l_api() {
            let s = MockServer::start();
            let base = "/api/jeux/v1/sauvegarde/Sebastien/VENKMAN";
            s.mock(|w, t| {
                w.method(GET).path(format!("{base}/document/manifeste.json"));
                t.status(404);
            });
            let docs: Vec<_> = ["configuration.json", "bibliotheque.json", "manifeste.json", "derniere-sauvegarde.json"]
                .iter()
                .map(|nom| {
                    s.mock(|w, t| {
                        w.method(PUT).path(format!("{base}/document/{nom}"));
                        t.status(200).json_body(serde_json::json!({"ok": true}));
                    })
                })
                .collect();
            s.mock(|w, t| {
                w.method(HEAD).path(format!("{base}/fichier"));
                t.status(404);
            });
            let fichier = s.mock(|w, t| {
                w.method(PUT).path(format!("{base}/fichier")).query_param("chemin", "jeux/110/C/DUNECD/DUNE37S0.SAV");
                t.status(200).json_body(serde_json::json!({"ok": true, "complet": true}));
            });
            let d = tempfile::tempdir().unwrap();
            let f = d.path().join("DUNE37S0.SAV");
            std::fs::write(&f, b"partie").unwrap();
            let c = client(&s);
            let api = Api { client: &c, profil: "Sebastien".into(), pc: "VENKMAN".into(), morceau: MORCEAU, pause: None };
            let contenu = Contenu {
                configuration: serde_json::json!({}),
                bibliotheque: serde_json::json!({"jeux": []}),
                parties: vec![Element { source: f, relatif: "jeux/110/C/DUNECD/DUNE37S0.SAV".into() }],
                ..Default::default()
            };
            let b = envoyer(&api, &contenu, "VENKMAN", "Firehouse").await.unwrap();
            assert_eq!((b.envoyes, b.fichiers, b.taille), (1, 1, 6));
            fichier.assert();
            for d in &docs {
                d.assert(); // configuration, bibliothèque, manifeste, dernière sauvegarde : chacun une fois
            }
        }
    }

    #[test]
    fn l_empreinte_est_le_sha256() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("f");
        std::fs::write(&f, b"abc").unwrap();
        assert_eq!(empreinte(&f).unwrap(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[tokio::test]
    async fn le_contenu_rassemble_parties_des_jeux_et_des_emulateurs() {
        use crate::coffre::CoffreMemoire;
        use crate::noyau::Connexion;
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, None).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();

        // Un émulateur avec le dossier du profil Seb.
        let emu = d.path().join("Emulateurs").join("RetroArch");
        let saves = crate::emulateurs_profils::dossier_du_profil(&emu, "Seb").join("saves");
        std::fs::create_dir_all(&saves).unwrap();
        std::fs::write(saves.join("Super Metroid (USA).srm"), b"srm").unwrap();
        let programme = emu.join("retroarch.exe").to_string_lossy().to_string();

        let c = n.contenu_sauvegarde(&[programme]).await.unwrap();
        assert_eq!(c.configuration["profil"]["nom"], "Seb");
        let rel: Vec<_> = c.parties.iter().map(|e| e.relatif.as_str()).collect();
        assert_eq!(rel, vec!["emulateurs/RetroArch/saves/Super Metroid (USA).srm"]);
        // Annoncé avant d'envoyer : la taille de toutes les parties, aucune sauvegarde précédente depuis ce PC.
        let e = n.estimer_sauvegarde(&[emu.join("retroarch.exe").to_string_lossy().to_string()], "VENKMAN").await.unwrap();
        assert_eq!(e, EstimationSauvegarde { total: 3, precedent: None, ..Default::default() });
    }

    #[tokio::test]
    async fn les_parties_wii_u_du_compte_cemu_du_profil_partent_avec_les_donnees_communes_de_ses_jeux() {
        use crate::coffre::CoffreMemoire;
        use crate::noyau::Connexion;
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, None).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        let cemu = d.path().join("Emulateurs").join("Cemu");
        std::fs::create_dir_all(cemu.join("portable")).unwrap();
        let programme = cemu.join("Cemu.exe").to_string_lossy().to_string();
        assert!(n.contenu_sauvegarde(std::slice::from_ref(&programme)).await.unwrap().parties.is_empty());
        let seb = crate::cemu::compte_du_profil(&cemu, "Seb").unwrap().id;
        let lea = crate::cemu::compte_du_profil(&cemu, "Léa").unwrap().id;
        for (compte, jeu, o) in [(seb, "00050000/101c9500", &b"zelda de Seb"[..]), (lea, "00050000/10145000", b"mario de Lea")] {
            let p = crate::cemu::dossier_du_jeu(&cemu, jeu, compte);
            std::fs::create_dir_all(&p).unwrap();
            std::fs::write(p.join("save.dat"), o).unwrap();
        }
        let commun = crate::cemu::dossier_du_jeu(&cemu, "00050000/101c9500", seb).parent().unwrap().join("common");
        std::fs::create_dir_all(&commun).unwrap();
        std::fs::write(commun.join("options.dat"), b"commun").unwrap();
        let c = n.contenu_sauvegarde(&[programme]).await.unwrap();
        let rel: Vec<_> = c.parties.iter().map(|e| e.relatif.as_str()).collect();
        assert_eq!(
            rel,
            vec!["emulateurs/Cemu/.cemu-compte/00050000/101c9500/compte/save.dat", "emulateurs/Cemu/.cemu-compte/00050000/101c9500/common/options.dat"],
            "les parties de SEB (et le commun de SES jeux), jamais celles de Léa"
        );
    }

    #[tokio::test]
    async fn les_parties_switch_de_l_utilisateur_eden_du_profil_partent() {
        use crate::coffre::CoffreMemoire;
        use crate::noyau::Connexion;
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, None).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        let eden = d.path().join("Emulateurs").join("Eden");
        std::fs::create_dir_all(eden.join("user")).unwrap();
        let programme = eden.join("eden.exe").to_string_lossy().to_string();
        // Pas d'utilisateur pour Seb : rien, et rien de créé.
        assert!(n.contenu_sauvegarde(std::slice::from_ref(&programme)).await.unwrap().parties.is_empty());
        assert!(!crate::eden::fichier_profils(&eden).exists());
        let seb = crate::eden::utilisateur_du_profil(&eden, "Seb").unwrap().uuid;
        let lea = crate::eden::utilisateur_du_profil(&eden, "Léa").unwrap().uuid;
        for (u, o) in [(&seb, &b"zelda de Seb"[..]), (&lea, b"zelda de Lea")] {
            let s = crate::eden::dossiers_parties(&eden, u)[0].1.join("0100000000010000");
            std::fs::create_dir_all(&s).unwrap();
            std::fs::write(s.join("save.bin"), o).unwrap();
        }
        let device = crate::eden::dossier_device(&eden).join("0100000000030000");
        std::fs::create_dir_all(&device).unwrap();
        std::fs::write(device.join("commun.bin"), b"commun").unwrap();
        let c = n.contenu_sauvegarde(&[programme]).await.unwrap();
        let rel: Vec<_> = c.parties.iter().map(|e| e.relatif.as_str()).collect();
        assert_eq!(
            rel,
            vec!["emulateurs/Eden/.eden-utilisateur/save/0100000000010000/save.bin", "emulateurs/Eden/.eden-utilisateur/device/0100000000030000/commun.bin"]
        );
        assert_eq!(std::fs::read(&c.parties[0].source).unwrap(), b"zelda de Seb", "jamais les parties de Léa");
    }

    #[tokio::test]
    async fn seules_les_parties_ryubing_de_la_racine_du_profil_partent() {
        use crate::coffre::CoffreMemoire;
        use crate::noyau::Connexion;
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, None).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        let ryu = d.path().join("Emulateurs").join("Ryubing");
        let sys = ryu.join("portable").join("system");
        std::fs::create_dir_all(&sys).unwrap();
        std::fs::write(sys.join("prod.keys"), b"cles").unwrap();
        let nca = crate::ryubing::dossier_firmware(&ryu).join("0123456789abcdef0123456789abcdef.nca");
        std::fs::create_dir_all(&nca).unwrap();
        std::fs::write(nca.join("00"), b"firmware").unwrap();
        let programme = ryu.join("Ryujinx.exe").to_string_lossy().to_string();
        for (qui, o) in [("Seb", &b"zelda de Seb"[..]), ("Léa", b"zelda de Lea")] {
            let r = crate::ryubing::racine(&ryu, qui);
            crate::ryubing::preparer_partie(&ryu, &r, crate::ryubing::Joueur1::Rien).unwrap();
            let s = r.join("bis").join("user").join("save").join("0000000000000001").join("0");
            std::fs::create_dir_all(&s).unwrap();
            std::fs::write(s.join("partie.bin"), o).unwrap();
            let i = r.join("bis").join("system").join("save").join("8000000000000000").join("0");
            std::fs::create_dir_all(&i).unwrap();
            std::fs::write(i.join("imkvdb.arc"), b"index").unwrap();
            // Un cache de shaders (par la jonction `games\`, commun) : jamais sauvegardé.
            std::fs::create_dir_all(r.join("games").join("0100000000010000")).unwrap();
            std::fs::write(r.join("games").join("0100000000010000").join("cache.bin"), b"x").unwrap();
        }
        let c = n.contenu_sauvegarde(std::slice::from_ref(&programme)).await.unwrap();
        let rel: Vec<_> = c.parties.iter().map(|e| e.relatif.as_str()).collect();
        assert_eq!(
            rel,
            vec![
                "emulateurs/Ryubing/bis/user/save/0000000000000001/0/partie.bin",
                "emulateurs/Ryubing/bis/system/save/8000000000000000/0/imkvdb.arc",
                "emulateurs/Ryubing/Config.json",
                "emulateurs/Ryubing/.frogtend-ryubing-reglages",
            ],
            "ni clés, ni firmware, ni caches"
        );
        assert_eq!(std::fs::read(&c.parties[0].source).unwrap(), b"zelda de Seb", "jamais les parties de Léa");
        // Ses réglages partent SANS le mot de passe du réseau local (une copie) ; son fichier reste tel quel.
        let cfg = crate::ryubing::config_de(&crate::ryubing::racine(&ryu, "Seb"));
        let avec = std::fs::read_to_string(&cfg).unwrap().replace("\"multiplayer_ldn_passphrase\": \"\"", "\"multiplayer_ldn_passphrase\": \"secret-ldn\"");
        assert!(avec.contains("secret-ldn"));
        std::fs::write(&cfg, &avec).unwrap();
        let c = n.contenu_sauvegarde(std::slice::from_ref(&programme)).await.unwrap();
        let e = c.parties.iter().find(|e| e.relatif == "emulateurs/Ryubing/Config.json").unwrap();
        assert_ne!(e.source, cfg);
        assert!(!std::fs::read_to_string(&e.source).unwrap().contains("secret-ldn"));
        assert_eq!(std::fs::read_to_string(&cfg).unwrap(), avec);
    }

    #[tokio::test]
    async fn les_parties_ps3_du_compte_du_profil_partent_sans_les_licences() {
        use crate::coffre::CoffreMemoire;
        use crate::noyau::Connexion;
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, None).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        let rpcs3 = d.path().join("Emulateurs").join("RPCS3");
        let programme = rpcs3.join("rpcs3.exe").to_string_lossy().to_string();
        // Pas encore de compte RPCS3 pour Seb : rien, et aucun compte créé par la sauvegarde.
        assert!(n.contenu_sauvegarde(std::slice::from_ref(&programme)).await.unwrap().parties.is_empty());
        assert!(!rpcs3.join("dev_hdd0").exists());
        // Le compte 00000002 est celui de Léa, le 00000003 celui de Seb.
        let home = rpcs3.join("dev_hdd0").join("home");
        for (compte, nom) in [("00000002", "Lea"), ("00000003", "Seb")] {
            std::fs::create_dir_all(home.join(compte)).unwrap();
            std::fs::write(home.join(compte).join("localusername"), nom).unwrap();
        }
        for (r, o) in [
            ("00000003/savedata/BCES00569-GAME/SAVE.DAT", &b"partie de Seb"[..]),
            ("00000003/trophy/NPWR00001_00/TROPUSR.DAT", b"trophees"),
            ("00000003/exdata/EP9001-BCES00569_00-CARPACK200000000.rif", b"licence"),
            ("00000002/savedata/BCES00569-GAME/SAVE.DAT", b"partie de Lea"),
        ] {
            std::fs::create_dir_all(home.join(r).parent().unwrap()).unwrap();
            std::fs::write(home.join(r), o).unwrap();
        }
        let c = n.contenu_sauvegarde(&[programme]).await.unwrap();
        let rel: Vec<_> = c.parties.iter().map(|e| e.relatif.as_str()).collect();
        assert_eq!(
            rel,
            vec!["emulateurs/RPCS3/.rpcs3-compte/savedata/BCES00569-GAME/SAVE.DAT", "emulateurs/RPCS3/.rpcs3-compte/trophy/NPWR00001_00/TROPUSR.DAT"],
            "les parties et trophées de SEB seulement, jamais ses licences"
        );
    }

    #[tokio::test]
    async fn les_captures_du_profil_partent_et_reviennent_sur_un_pc_neuf() {
        use crate::coffre::CoffreMemoire;
        use crate::noyau::Connexion;
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, None).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();
        // Une capture d'un jeu importé (-5), une dont l'image manque (disque débranché), une d'un jeu caché, une d'une autre personne.
        let rangee = d.path().join("Captures").join("Atari Jaguar").join("Bubsy");
        std::fs::create_dir_all(&rangee).unwrap();
        let img = rangee.join("2026-11-12 01-50-00.bmp");
        std::fs::write(&img, b"BMimage").unwrap();
        crate::captures::noter(&n.dossier, std::slice::from_ref(&img), &id, -5, "Bubsy", "Atari Jaguar").unwrap();
        let absente = rangee.join("absente.bmp");
        let cachee = rangee.join("cachee.bmp");
        std::fs::write(&cachee, b"x").unwrap();
        crate::captures::noter(&n.dossier, &[absente.clone()], &id, -5, "Bubsy", "Atari Jaguar").unwrap();
        crate::captures::noter(&n.dossier, &[cachee], &id, 987_654_321, "Caché", "X").unwrap();
        let autre = rangee.join("autre.bmp");
        std::fs::write(&autre, b"y").unwrap();
        crate::captures::noter(&n.dossier, &[autre], "quelqu-un-d-autre", -5, "Bubsy", "Atari Jaguar").unwrap();
        let cle = crate::captures::identifiant(&img);

        let c = n.contenu_sauvegarde(&[]).await.unwrap();
        let rel: Vec<_> = c.parties.iter().map(|e| e.relatif.clone()).collect();
        assert_eq!(rel, vec![format!("captures/-5/{cle}.bmp"), format!("captures/-5/{cle}.json")], "seulement la sienne, avec sa fiche");
        assert!(c.gardes.contains(&format!("captures/-5/{}.", crate::captures::identifiant(&absente))), "image absente : gardée chez Firehouse");
        assert!(c.gardes.contains(&"captures/987654321/".to_string()), "jeu caché : gardé, jamais envoyé");
        let fiche: Value = serde_json::from_slice(&std::fs::read(&c.parties[1].source).unwrap()).unwrap();
        assert_eq!((fiche["systeme"].as_str(), fiche["dossier_jeu"].as_str(), fiche["nom"].as_str()), (Some("Atari Jaguar"), Some("Bubsy"), Some("2026-11-12 01-50-00.bmp")));
        let partage = tempfile::tempdir().unwrap();
        let dl = DossierLocal(partage.path().into());
        envoyer(&dl, &c, "VENKMAN", "test").await.unwrap();

        // Le PC neuf : plus d'image ni d'index, la sauvegarde téléchargée dans la zone d'attente.
        std::fs::remove_file(&img).unwrap();
        std::fs::write(n.dossier.join("captures.json"), b"[]").unwrap();
        let attente = n.dossier.join("restauration").join(&id).join("parties").join("captures");
        std::fs::create_dir_all(attente.join("-5")).unwrap();
        for f in [format!("{cle}.bmp"), format!("{cle}.json")] {
            std::fs::copy(partage.path().join("parties/captures/-5").join(&f), attente.join("-5").join(&f)).unwrap();
        }
        assert_eq!(n.reposer_captures(None).await.unwrap(), (0, 1), "sans dossier des captures : elle attend");
        assert!(n.contenu_sauvegarde(&[]).await.unwrap().gardes.contains(&format!("captures/-5/{cle}.bmp")), "en attente : jamais retirée de Firehouse");
        let neuf = d.path().join("Nouveau");
        assert_eq!(n.reposer_captures(Some(&neuf)).await.unwrap(), (1, 0));
        let revenue = neuf.join("Atari Jaguar").join("Bubsy").join("2026-11-12 01-50-00.bmp");
        assert_eq!(std::fs::read(&revenue).unwrap(), b"BMimage");
        let l = crate::captures::index(&n.dossier);
        assert_eq!(l.len(), 1);
        assert_eq!((l[0].profil.as_str(), l[0].jeu, l[0].titre.as_str(), l[0].cle_sauvegarde()), (id.as_str(), -5, "Bubsy", cle.as_str()));
        assert!(!attente.join("-5").exists(), "posée, relue, notée : retirée de l'attente");
        // Sauvegardée de nouveau : même place, rien renvoyé.
        let b = envoyer(&dl, &n.contenu_sauvegarde(&[]).await.unwrap(), "VENKMAN", "test").await.unwrap();
        assert_eq!((b.envoyes, b.retires), (0, 0), "{b:?}");
        // Restaurée une 2e fois : l'image identique est reprise, pas de doublon.
        std::fs::create_dir_all(attente.join("-5")).unwrap();
        for f in [format!("{cle}.bmp"), format!("{cle}.json")] {
            std::fs::copy(partage.path().join("parties/captures/-5").join(&f), attente.join("-5").join(&f)).unwrap();
        }
        assert_eq!(n.reposer_captures(Some(&neuf)).await.unwrap(), (1, 0));
        assert_eq!(std::fs::read_dir(neuf.join("Atari Jaguar").join("Bubsy")).unwrap().count(), 1);
        assert_eq!(crate::captures::index(&n.dossier).len(), 1);
    }

    #[tokio::test]
    async fn ce_qui_attend_d_une_restauration_n_est_jamais_retire_de_firehouse() {
        use crate::coffre::CoffreMemoire;
        use crate::noyau::Connexion;
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, None).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();
        // PC formaté : la sauvegarde restaurée attend (jeu 42 pas remis, RPCS3 pas encore installé).
        let attente = d.path().join("app").join("restauration").join(&id);
        for r in ["parties/jeux/42/SAVE.DAT", "parties/emulateurs/RPCS3/dev_hdd0/x"] {
            std::fs::create_dir_all(attente.join(r).parent().unwrap()).unwrap();
            std::fs::write(attente.join(r), b"partie").unwrap();
        }
        std::fs::write(attente.join("bibliotheque.json"), br#"{"jeux":[{"id":42,"titre":"Dune"}]}"#).unwrap();
        let c = n.contenu_sauvegarde(&[]).await.unwrap();
        assert!(c.gardes.contains(&"jeux/42/".to_string()) && c.gardes.contains(&"emulateurs/RPCS3/".to_string()), "{:?}", c.gardes);
        assert!(c.bibliotheque["jeux"].as_array().unwrap().iter().any(|j| j["id"] == 42), "le jeu à remettre reste listé");

        // Envoyé à Firehouse : ce qui y était (la sauvegarde restaurée) n'en sort pas.
        let partage = tempfile::tempdir().unwrap();
        let dl = DossierLocal(partage.path().into());
        let ancien = attente.join("parties/jeux/42/SAVE.DAT");
        envoyer(&dl, &contenu(vec![Element { source: ancien, relatif: "jeux/42/SAVE.DAT".into() }]), "VENKMAN", "test").await.unwrap();
        let b = envoyer(&dl, &c, "VENKMAN", "test").await.unwrap();
        assert_eq!(b.retires, 0);
        assert!(partage.path().join("parties/jeux/42/SAVE.DAT").is_file());
        // Et la garde d'une sauvegarde existante compte aussi ses jeux, même sans parties.
        let vide = tempfile::tempdir().unwrap();
        let dv = DossierLocal(vide.path().into());
        dv.ecrire_document("bibliotheque.json", &json!({"jeux": [{"id": 1}]})).await.unwrap();
        assert!(verifier_reprise(&dv).await.is_err());
    }

    #[tokio::test]
    async fn les_abris_d_un_jeu_cache_a_ce_profil_ne_partent_jamais() {
        use crate::coffre::CoffreMemoire;
        use crate::noyau::Connexion;
        let d = tempfile::tempdir().unwrap();
        let app = d.path().join("app");
        let n = Noyau::nouveau(&app, Box::new(CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Léa", None, None).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();
        // Les abris sont ceux du PC : Dune (visible pour Léa) et un jeu qu'elle ne voit pas (celui d'un admin).
        for (jeu, f) in [("110", "DUNE.SAV"), ("987654", "CACHE.SAV")] {
            let a = app.join("sauvegardes").join(jeu).join("1790000000");
            std::fs::create_dir_all(&a).unwrap();
            std::fs::write(a.join(f), b"x").unwrap();
        }
        let c = n.contenu_sauvegarde(&[]).await.unwrap();
        let r: Vec<&str> = c.parties.iter().map(|e| e.relatif.as_str()).collect();
        assert_eq!(r, vec!["jeux/110/DUNE.SAV"]);
        assert_eq!(c.gardes, vec!["jeux/987654/".to_string()], "ce qui en avait été envoyé reste protégé");
        assert!(app.join("sauvegardes").join("987654").exists(), "l'abri reste sur le PC");
    }

    #[tokio::test]
    async fn une_ludotheque_jamais_synchronisee_n_est_pas_sauvegardee() {
        use crate::coffre::CoffreMemoire;
        use crate::noyau::Connexion;
        let s = httpmock::MockServer::start();
        let (_d, n) = {
            let d = tempfile::tempdir().unwrap();
            let n = Noyau::nouveau(d.path(), Box::new(CoffreMemoire::default())).unwrap();
            (d, n)
        };
        let tout = s.mock(|w, t| {
            w.path_matches(httpmock::Regex::new(".*").unwrap());
            t.status(200).json_body(json!({"ok": true}));
        });
        let id = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: s.base_url(), simule: false }).await.unwrap();
        // Refusé avant d'envoyer quoi que ce soit : aucune requête.
        assert!(matches!(n.sauvegarder(&[], "Venkman", false, None, None).await, Err(Erreur::Refus(_))));
        tout.assert_hits(0);
    }

    #[tokio::test]
    async fn les_parties_mises_a_l_abri_d_un_jeu_retire_sont_sauvegardees() {
        use crate::coffre::CoffreMemoire;
        use crate::noyau::Connexion;
        let d = tempfile::tempdir().unwrap();
        let app = d.path().join("app");
        let n = Noyau::nouveau(&app, Box::new(CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, None).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();

        // Dune retiré du PC : deux mises à l'abri, la plus récente l'emporte.
        let ancienne = app.join("sauvegardes").join("110").join("1790000000").join("C").join("DUNECD");
        let recente = app.join("sauvegardes").join("110").join("1790724553").join("C").join("DUNECD");
        std::fs::create_dir_all(&ancienne).unwrap();
        std::fs::create_dir_all(&recente).unwrap();
        std::fs::write(ancienne.join("DUNE37S0.SAV"), b"vieille").unwrap();
        std::fs::write(recente.join("DUNE37S0.SAV"), b"recente").unwrap();

        let c = n.contenu_sauvegarde(&[]).await.unwrap();
        assert_eq!(c.parties.len(), 1);
        assert_eq!(c.parties[0].relatif, "jeux/110/C/DUNECD/DUNE37S0.SAV");
        assert_eq!(std::fs::read(&c.parties[0].source).unwrap(), b"recente");

        // Le dossier des abris est réglé : l'ancien emplacement y est rangé (copie vérifiée), puis un abri encore plus
        // récent y arrive ; c'est lui qui part.
        n.regler_abris(Some(d.path().join("Abris")));
        assert_eq!(n.migrer_abris().unwrap(), 2);
        assert!(!app.join("sauvegardes").exists(), "l'ancien emplacement est vidé");
        assert_eq!(std::fs::read(d.path().join("Abris").join("110").join("1790724553").join("C").join("DUNECD").join("DUNE37S0.SAV")).unwrap(), b"recente");
        let plus = d.path().join("Abris").join("110").join("1790800000");
        std::fs::create_dir_all(&plus).unwrap();
        std::fs::write(plus.join("DUNE37S0.SAV"), b"encore").unwrap();
        let c = n.contenu_sauvegarde(&[]).await.unwrap();
        assert_eq!(c.parties.len(), 1);
        assert_eq!(std::fs::read(&c.parties[0].source).unwrap(), b"encore");
    }

    /// Lot S4 (Seb, 23/11) : un abri fait À LA MAIN part désormais avec son propriétaire (fiche d'abord) et ne cache
    /// toujours pas l'abri d'avant ; les parties prises hors du dossier du jeu (`emplacements/`) ne partent qu'avec le
    /// profil propriétaire de l'abri.
    #[tokio::test]
    async fn un_abri_fait_a_la_main_part_avec_son_profil_et_les_emplacements_suivent_leur_profil() {
        use crate::abri_parties::{FicheAbri, SORTE_EMPLACEMENTS, SORTE_NUAGE_STEAM};
        use crate::coffre::CoffreMemoire;
        use crate::noyau::Connexion;
        let d = tempfile::tempdir().unwrap();
        let app = d.path().join("app");
        let n = Noyau::nouveau(&app, Box::new(CoffreMemoire::default())).unwrap();
        let seb = n.creer_profil("Seb", None, None).unwrap().id;
        let lea = n.creer_profil("Léa", None, None).unwrap().id;
        n.regler_abris(Some(d.path().join("Abris")));
        let du_jeu = d.path().join("Abris").join("110");
        let poser = |date: &str, fichiers: &[(&str, &[u8])], fiche: Option<FicheAbri>| {
            for (r, o) in fichiers {
                let f = du_jeu.join(date).join(r);
                std::fs::create_dir_all(f.parent().unwrap()).unwrap();
                std::fs::write(f, o).unwrap();
            }
            if let Some(x) = fiche {
                std::fs::write(du_jeu.join(format!("{date}.json")), serde_json::to_vec(&x).unwrap()).unwrap();
            }
        };
        // L'abri du retrait (lot 3), puis un abri à la main plus récent.
        poser("1790000000", &[("C/DUNECD/DUNE37S0.SAV", b"retrait")], None);
        let main = crate::nuage_steam::AMettre { source: PathBuf::new(), relatif: "emplacements/0a1b2c3d/x.sav".into(), taille: 4, sha: String::new() };
        let lieu = crate::abri_parties::EmplacementAbrite { cle: "0a1b2c3d".into(), chemin: r"%APPDATA%\Jeu\0a1b2c3d".into(), nature: "dossier".into(), en_ligne: Some(0), ..Default::default() };
        poser("1790000100", &[("emplacements/0a1b2c3d/x.sav", b"main")], Some(FicheAbri::nouvelle(SORTE_EMPLACEMENTS, &seb, vec![lieu], &[main], 0, 0)));
        n.ouvrir(&seb, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();
        let c = n.contenu_sauvegarde(&[]).await.unwrap();
        assert_eq!(
            c.parties.iter().map(|e| e.relatif.as_str()).collect::<Vec<_>>(),
            ["jeux/110/C/DUNECD/DUNE37S0.SAV", "jeux/110/emplacements/fiche.json", "jeux/110/emplacements/0a1b2c3d/x.sav"],
            "l'abri du retrait part toujours ; celui fait à la main aussi (sa fiche d'abord), sans jamais cacher le premier"
        );
        let fiche: FicheAbri = serde_json::from_slice(&std::fs::read(&c.parties[1].source).unwrap()).unwrap();
        assert_eq!((fiche.sorte.as_str(), fiche.emplacements[0].date.as_str(), fiche.emplacements[0].declencheur.as_str()), (SORTE_EMPLACEMENTS, "1790000100", "main"));
        // Un abri d'un jeu Steam au format du lot S2 : sa fiche dit ses emplacements (clé, chemin NON résolu).
        let steam = |date: &str, profil: &str, emplacements: &[(&str, &[u8])]| {
            let mut l: Vec<(String, &[u8])> = vec![("nuage-steam/m1/remote/campaign".into(), b"c")];
            let mut fichiers = Vec::new();
            let mut abrites = Vec::new();
            for (cle, o) in emplacements {
                let r = format!("emplacements/{cle}/save_1");
                fichiers.push(crate::nuage_steam::AMettre { source: PathBuf::new(), relatif: r.clone(), taille: o.len() as u64, sha: String::new() });
                abrites.push(crate::abri_parties::EmplacementAbrite { cle: cle.to_string(), chemin: format!(r"%APPDATA%\Jeu\{cle}"), nature: "dossier".into(), filtre: None, identifiant: None, ..Default::default() });
                l.push((r, o));
            }
            std::fs::create_dir_all(du_jeu.join(date).join("nuage-steam").join("m1")).unwrap();
            std::fs::write(du_jeu.join(date).join("nuage-steam").join("m1").join("abri.json"), b"[]").unwrap();
            let refs: Vec<(&str, &[u8])> = l.iter().map(|(r, o)| (r.as_str(), *o)).collect();
            poser(date, &refs, Some(FicheAbri::nouvelle(SORTE_NUAGE_STEAM, profil, abrites, &fichiers, 0, 0)));
        };
        steam("1790000200", &seb, &[("0a1b2c3d", b"seb-1"), ("eeeeeeee", b"seb-2")]);
        let envoyes = |c: &Contenu| {
            let mut r: Vec<(String, Vec<u8>)> = c.parties.iter().filter(|e| !e.relatif.ends_with("abri.json")).map(|e| (e.relatif.clone(), std::fs::read(&e.source).unwrap())).collect();
            r.sort();
            r
        };
        let c = n.contenu_sauvegarde(&[]).await.unwrap();
        let r = envoyes(&c);
        assert_eq!(r.iter().map(|(x, _)| x.as_str()).collect::<Vec<_>>(), ["jeux/110/emplacements/0a1b2c3d/save_1", "jeux/110/emplacements/eeeeeeee/save_1", "jeux/110/emplacements/fiche.json", "jeux/110/nuage-steam/m1/remote/campaign"]);
        // Correctif 6 : la fiche voyage avec eux, sans aucun chemin du PC ni nom d'utilisateur.
        let fiche: FicheAbri = serde_json::from_slice(&r[2].1).unwrap();
        assert_eq!((fiche.profil.as_str(), fiche.emplacements.len()), (seb.as_str(), 2));
        assert!(!String::from_utf8_lossy(&r[2].1).contains(&*d.path().to_string_lossy().replace('\\', "\\\\")));
        // Correctif 2 : un abri plus récent de Seb où « eeeeeeee » manque (verrouillé, hors ligne, trop gros avant un
        // mod) ne fait pas sortir sa copie précédente.
        steam("1790000300", &seb, &[("0a1b2c3d", b"seb-1b")]);
        let r = envoyes(&n.contenu_sauvegarde(&[]).await.unwrap());
        assert_eq!(r[0], ("jeux/110/emplacements/0a1b2c3d/save_1".to_string(), b"seb-1b".to_vec()));
        assert_eq!(r[1], ("jeux/110/emplacements/eeeeeeee/save_1".to_string(), b"seb-2".to_vec()));
        // Correctif 1 : un abri encore plus récent de LÉA (même compte Steam) ne fait jamais sortir ceux de Seb.
        steam("1790000400", &lea, &[("0a1b2c3d", b"lea")]);
        let r = envoyes(&n.contenu_sauvegarde(&[]).await.unwrap());
        assert_eq!(r.iter().filter(|(x, _)| x.contains("/emplacements/") && !x.ends_with("fiche.json")).map(|(_, o)| o.clone()).collect::<Vec<_>>(), [b"seb-1b".to_vec(), b"seb-2".to_vec()]);
        n.ouvrir(&lea, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();
        let r = envoyes(&n.contenu_sauvegarde(&[]).await.unwrap());
        assert_eq!(r.iter().filter(|(x, _)| x.contains("/emplacements/") && !x.ends_with("fiche.json")).map(|(_, o)| o.clone()).collect::<Vec<_>>(), [b"lea".to_vec()], "jamais les parties d'un autre profil");
        assert!(r.iter().any(|(x, _)| x == "jeux/110/nuage-steam/m1/remote/campaign"));
    }

    /// Correctif 3 (revue du 22/11) : un jeu dont le dossier a un sous-dossier `emplacements` (un abri sans fiche S2)
    /// part tel quel, comme avant.
    #[tokio::test]
    async fn un_dossier_emplacements_du_jeu_reste_une_partie_du_jeu() {
        use crate::coffre::CoffreMemoire;
        use crate::noyau::Connexion;
        let d = tempfile::tempdir().unwrap();
        let app = d.path().join("app");
        let n = Noyau::nouveau(&app, Box::new(CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, None).unwrap().id;
        n.regler_abris(Some(d.path().join("Abris")));
        let a = d.path().join("Abris").join("110").join("1790000000").join("emplacements");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::write(a.join("carte.dat"), b"x").unwrap();
        n.ouvrir(&id, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();
        let c = n.contenu_sauvegarde(&[]).await.unwrap();
        assert_eq!(c.parties.iter().map(|e| e.relatif.as_str()).collect::<Vec<_>>(), ["jeux/110/emplacements/carte.dat"]);
    }
}

/// Lot S4 (23/11) : l'envoi des abris chez Firehouse — par propriétaire, robuste, sans dépasser la place.
#[cfg(test)]
mod tests_s4 {
    use super::*;
    use crate::abri_parties::{EmplacementAbrite, FicheAbri, SORTE_EMPLACEMENTS};
    use crate::coffre::CoffreMemoire;
    use crate::noyau::Connexion;
    use std::sync::Mutex;

    /// Une destination en mémoire dont chaque fichier peut échouer à la demande (comme Firehouse qui refuse).
    #[derive(Default)]
    struct Fausse {
        docs: Mutex<BTreeMap<String, Value>>,
        docs_profil: Mutex<BTreeMap<String, Value>>,
        fichiers: Mutex<BTreeMap<String, Vec<u8>>>,
        supprimes: Mutex<Vec<String>>,
        /// relatif → l'erreur que Firehouse rend pour lui (envoi ou suppression).
        erreurs: Mutex<BTreeMap<String, fn() -> Erreur>>,
        erreur_profil: Mutex<Option<fn() -> Erreur>>,
        place: Option<Place>,
    }

    impl Fausse {
        fn echouer(&self, r: &str, e: fn() -> Erreur) {
            self.erreurs.lock().unwrap().insert(r.into(), e);
        }
        fn reussir(&self, r: &str) {
            self.erreurs.lock().unwrap().remove(r);
        }
        fn manifeste(&self) -> Option<Value> {
            self.docs.lock().unwrap().get("manifeste.json").cloned()
        }
        fn lignes(&self) -> Vec<String> {
            self.manifeste().map(|m| m["fichiers"].as_object().unwrap().keys().cloned().collect()).unwrap_or_default()
        }
    }

    impl Destination for Fausse {
        async fn lire_document(&self, nom: &str) -> Resultat<Option<Value>> {
            Ok(self.docs.lock().unwrap().get(nom).cloned())
        }
        async fn ecrire_document(&self, nom: &str, v: &Value) -> Resultat<()> {
            self.docs.lock().unwrap().insert(nom.into(), v.clone());
            Ok(())
        }
        async fn envoyer_fichier(&self, relatif: &str, source: &Path, _sha: &str, taille: u64) -> Resultat<u64> {
            if let Some(e) = self.erreurs.lock().unwrap().get(relatif) {
                return Err(e());
            }
            self.fichiers.lock().unwrap().insert(relatif.into(), std::fs::read(source)?);
            Ok(taille)
        }
        async fn supprimer_fichier(&self, relatif: &str) -> Resultat<()> {
            if let Some(e) = self.erreurs.lock().unwrap().get(relatif) {
                return Err(e());
            }
            self.fichiers.lock().unwrap().remove(relatif);
            self.supprimes.lock().unwrap().push(relatif.into());
            Ok(())
        }
        fn present(&self, _relatif: &str, _taille: u64) -> Option<bool> {
            None
        }
        async fn ecrire_document_profil(&self, nom: &str, v: &Value) -> Resultat<()> {
            if let Some(e) = *self.erreur_profil.lock().unwrap() {
                return Err(e());
            }
            self.docs_profil.lock().unwrap().insert(nom.into(), v.clone());
            Ok(())
        }
        async fn place(&self) -> Resultat<Option<Place>> {
            Ok(self.place)
        }
    }

    fn contenu(parties: Vec<Element>) -> Contenu {
        Contenu { configuration: json!({}), bibliotheque: json!({"jeux": []}), parties, ..Default::default() }
    }

    fn fichier(d: &Path, nom: &str, o: &[u8]) -> PathBuf {
        let f = d.join(nom);
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(&f, o).unwrap();
        f
    }

    fn conflit() -> Erreur {
        Erreur::Conflit("« un fichier de parties » arrive abîmé chez Firehouse.".into())
    }

    #[tokio::test]
    async fn un_fichier_en_echec_est_saute_compte_garde_sa_ligne_et_n_est_jamais_retire_de_firehouse() {
        let d = tempfile::tempdir().unwrap();
        let a = fichier(d.path(), "a.sav", b"a1");
        let b = fichier(d.path(), "b.sav", b"b1");
        let el = |p: &Path, r: &str| Element { source: p.into(), relatif: r.into() };
        let f = Fausse::default();
        let c = contenu(vec![el(&a, "jeux/7/a.sav"), el(&b, "jeux/7/b.sav")]);
        assert_eq!(envoyer(&f, &c, "PC", "t").await.unwrap().envoyes, 2);
        // b.sav change, mais Firehouse le refuse (409) : sauté, compté, sa copie d'avant reste, a.sav part quand même.
        std::fs::write(&a, b"a2").unwrap();
        std::fs::write(&b, b"b2").unwrap();
        f.echouer("jeux/7/b.sav", conflit);
        let bilan = envoyer(&f, &c, "PC", "t").await.unwrap();
        assert_eq!((bilan.envoyes, bilan.sautes, bilan.retires), (1, 1, 0));
        assert_eq!(f.fichiers.lock().unwrap()["jeux/7/a.sav"], b"a2");
        assert_eq!(f.fichiers.lock().unwrap()["jeux/7/b.sav"], b"b1", "la copie d'avant reste chez Firehouse");
        let m = f.manifeste().unwrap();
        assert_eq!(m["fichiers"]["jeux/7/b.sav"][0], empreinte_octets(b"b1"), "sa ligne de l'ancien manifeste reste");
        // Le même fichier, toujours refusé, au tour suivant : jamais retiré (ni supprimé chez Firehouse).
        let bilan = envoyer(&f, &c, "PC", "t").await.unwrap();
        assert_eq!((bilan.sautes, bilan.retires), (1, 0));
        assert!(f.supprimes.lock().unwrap().is_empty());
        // Accepté enfin : il part.
        f.reussir("jeux/7/b.sav");
        assert_eq!(envoyer(&f, &c, "PC", "t").await.unwrap().envoyes, 1);
        assert_eq!(f.fichiers.lock().unwrap()["jeux/7/b.sav"], b"b2");
        // Un NOUVEAU fichier refusé (400) : sauté, pas de ligne (il repartira la prochaine fois).
        let n = fichier(d.path(), "n.sav", b"neuf");
        f.echouer("jeux/7/n.sav", || Erreur::Refus("Firehouse a refusé la demande (code 400).".into()));
        let c2 = contenu(vec![el(&a, "jeux/7/a.sav"), el(&b, "jeux/7/b.sav"), el(&n, "jeux/7/n.sav")]);
        assert_eq!(envoyer(&f, &c2, "PC", "t").await.unwrap().sautes, 1);
        assert!(!f.lignes().contains(&"jeux/7/n.sav".to_string()));
        // Une suppression refusée : la ligne reste.
        f.echouer("jeux/7/a.sav", conflit);
        let bilan = envoyer(&f, &contenu(vec![el(&b, "jeux/7/b.sav")]), "PC", "t").await.unwrap();
        assert_eq!(bilan.retires, 0);
        assert!(f.lignes().contains(&"jeux/7/a.sav".to_string()));
    }

    fn empreinte_octets(o: &[u8]) -> String {
        Sha256::digest(o).iter().map(|b| format!("{b:02x}")).collect()
    }

    #[tokio::test]
    async fn jeton_refuse_plein_reseau_et_503_arretent_tout_sans_ecrire_le_manifeste() {
        let arrets: [fn() -> Erreur; 4] = [
            || Erreur::JetonRefuse("jeton".into()),
            || Erreur::Plein { motif: "plein".into(), manque_octets: None },
            || Erreur::Reseau("coupé".into()),
            || Erreur::Serveur("Firehouse est occupé (maintenance en cours). Réessaie dans une minute.".into()),
        ];
        for arret in arrets {
            let d = tempfile::tempdir().unwrap();
            let a = fichier(d.path(), "a.sav", b"a");
            let b = fichier(d.path(), "b.sav", b"b");
            let f = Fausse::default();
            let c = contenu(vec![Element { source: a.clone(), relatif: "jeux/1/a.sav".into() }]);
            envoyer(&f, &c, "PC", "t").await.unwrap();
            let avant = (f.manifeste(), f.docs.lock().unwrap().get("derniere-sauvegarde.json").cloned());
            f.echouer("jeux/1/b.sav", arret);
            let c = contenu(vec![Element { source: b, relatif: "jeux/1/b.sav".into() }, Element { source: a, relatif: "jeux/1/a.sav".into() }]);
            let e = envoyer(&f, &c, "PC", "t").await.unwrap_err();
            assert!(arrete_tout(&e), "{e:?}");
            assert_eq!((f.manifeste(), f.docs.lock().unwrap().get("derniere-sauvegarde.json").cloned()), avant, "rien d'écrit après un arrêt");
            assert!(f.supprimes.lock().unwrap().is_empty(), "rien retiré");
        }
        // 409, disque, 400/413 : seulement sautés.
        for e in [Erreur::Conflit(String::new()), Erreur::Disque(String::new()), Erreur::Refus(String::new())] {
            assert!(!arrete_tout(&e), "{e:?}");
        }
    }

    #[tokio::test]
    async fn une_fiche_d_emplacements_en_echec_ne_laisse_partir_aucun_fichier_emplacements_de_son_jeu() {
        let d = tempfile::tempdir().unwrap();
        let fiche = fichier(d.path(), "fiche.json", b"{}");
        let x = fichier(d.path(), "x.sav", b"x");
        let autre = fichier(d.path(), "autre.sav", b"o");
        let f = Fausse::default();
        f.echouer("jeux/5/emplacements/fiche.json", conflit);
        let c = contenu(vec![
            Element { source: fiche.clone(), relatif: "jeux/5/emplacements/fiche.json".into() },
            Element { source: x.clone(), relatif: "jeux/5/emplacements/0a1b2c3d/x.sav".into() },
            Element { source: autre.clone(), relatif: "jeux/6/emplacements/fiche.json".into() },
        ]);
        let b = envoyer(&f, &c, "PC", "t").await.unwrap();
        assert_eq!((b.envoyes, b.sautes), (1, 2));
        assert_eq!(f.lignes(), ["jeux/6/emplacements/fiche.json"], "aucun emplacements/ du jeu 5 au manifeste");
        assert!(!f.fichiers.lock().unwrap().contains_key("jeux/5/emplacements/0a1b2c3d/x.sav"));
        // Déjà envoyés (fiche et fichier) : une fiche qui échoue ensuite garde les deux tels quels.
        f.reussir("jeux/5/emplacements/fiche.json");
        envoyer(&f, &c, "PC", "t").await.unwrap();
        std::fs::write(&fiche, b"{\"v\": 2}").unwrap();
        f.echouer("jeux/5/emplacements/fiche.json", conflit);
        let b = envoyer(&f, &contenu(vec![Element { source: fiche, relatif: "jeux/5/emplacements/fiche.json".into() }]), "PC", "t").await.unwrap();
        assert_eq!(b.retires, 1, "seulement le jeu 6, qui n'est plus sur le PC");
        assert!(f.lignes().contains(&"jeux/5/emplacements/0a1b2c3d/x.sav".to_string()));
        assert!(!f.supprimes.lock().unwrap().iter().any(|r| r.starts_with("jeux/5/")), "rien du jeu 5 retiré de Firehouse");
    }

    #[tokio::test]
    async fn le_sel_du_profil_part_et_un_refus_n_est_jamais_un_echec() {
        let f = Fausse::default();
        let mut c = contenu(vec![]);
        c.sel = Some("0123456789abcdef0123456789abcdef".into());
        let b = envoyer(&f, &c, "PC", "t").await.unwrap();
        assert!(!b.sel_en_attente);
        assert_eq!(f.docs_profil.lock().unwrap()[crate::abri_parties::NOM_SEL_ENVOYE], json!({"sel": "0123456789abcdef0123456789abcdef"}));
        // Contrat 1.51 pas encore servi : 400/404 → noté, réessayé la prochaine fois, la sauvegarde réussit.
        *f.erreur_profil.lock().unwrap() = Some(|| Erreur::Introuvable(String::new()));
        let b = envoyer(&f, &c, "PC", "t").await.unwrap();
        assert!(b.sel_en_attente);
        assert!(f.manifeste().is_some());
    }

    #[tokio::test]
    async fn un_deuxieme_envoi_sans_changement_n_envoie_aucun_octet() {
        let d = tempfile::tempdir().unwrap();
        let a = fichier(d.path(), "a.sav", b"partie");
        let f = Fausse::default();
        let c = contenu(vec![Element { source: a, relatif: "jeux/1/a.sav".into() }]);
        assert_eq!(envoyer(&f, &c, "PC", "t").await.unwrap().octets, 6);
        let b = envoyer(&f, &c, "PC", "t").await.unwrap();
        assert_eq!((b.octets, b.envoyes, b.inchanges), (0, 0, 1));
    }

    #[tokio::test]
    async fn sans_assez_de_place_chez_firehouse_rien_ne_part() {
        let d = tempfile::tempdir().unwrap();
        let a = fichier(d.path(), "a.sav", &[0u8; 1000]);
        let f = Fausse { place: Some(Place { utilise: 99_500, quota: 100_000 }), ..Default::default() };
        let c = contenu(vec![Element { source: a.clone(), relatif: "jeux/1/a.sav".into() }]);
        match envoyer(&f, &c, "PC", "t").await {
            Err(Erreur::Plein { manque_octets: Some(500), motif }) => assert!(motif.contains("Il manque") && !motif.contains("jeux/"), "{motif}"),
            r => panic!("{r:?}"),
        }
        assert!(f.fichiers.lock().unwrap().is_empty() && f.docs.lock().unwrap().is_empty(), "rien envoyé, rien écrit");
        // Assez de place : il part. Remplacé ensuite : contrat 1.51, l'ancien n'est plus compté deux fois (plus de marge).
        let f = Fausse { place: Some(Place { utilise: 0, quota: 1500 }), ..Default::default() };
        envoyer(&f, &c, "PC", "t").await.unwrap();
        std::fs::write(&a, [1u8; 1000]).unwrap();
        assert_eq!(envoyer(&f, &c, "PC", "t").await.unwrap().envoyes, 1);
        // Rien à envoyer : jamais refusé, même plein.
        let f2 = Fausse { place: Some(Place { utilise: 100, quota: 100 }), ..Default::default() };
        assert!(envoyer(&f2, &contenu(vec![]), "PC", "t").await.is_ok());
    }

    #[tokio::test]
    async fn un_gros_manifeste_passe_en_documents_par_jeu_et_l_ancien_se_lit_toujours() {
        let f = Fausse::default();
        let mut m = ManifesteParties::default();
        m.fichiers.insert("jeux/110/a.sav".into(), ("aa".into(), 1));
        m.fichiers.insert("jeux/-5/b.sav".into(), ("bb".into(), 2));
        m.fichiers.insert("emulateurs/RetroArch/saves/x.srm".into(), ("cc".into(), 3));
        // Petit : un seul document, comme avant.
        ecrire_manifeste(&f, &m, SEUIL_MANIFESTE).await.unwrap();
        assert_eq!(f.docs.lock().unwrap().len(), 1);
        assert_eq!(lire_manifeste(&f, true).await.unwrap(), m);
        // Au-delà du seuil : un document par jeu (motif du contrat), le reste et la liste dans manifeste.json.
        ecrire_manifeste(&f, &m, 10).await.unwrap();
        let docs = f.docs.lock().unwrap().clone();
        assert_eq!(docs.keys().cloned().collect::<Vec<_>>(), ["manifeste-jeu-110.json", "manifeste-jeu-m5.json", "manifeste.json"]);
        assert_eq!(docs["manifeste.json"]["manifestes_jeux"], json!(["jeu-110", "jeu-m5"]));
        assert_eq!(docs["manifeste.json"]["fichiers"].as_object().unwrap().len(), 1);
        assert!(docs.keys().all(|k| k == "manifeste.json" || k.strip_prefix("manifeste-").and_then(|r| r.strip_suffix(".json")).is_some_and(|c| c.len() <= 80 && c.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b)))));
        assert_eq!(lire_manifeste(&f, true).await.unwrap(), m, "relu en entier");
        // Un document à part qui manque : erreur en lecture stricte (restauration), lignes absentes sinon.
        f.docs.lock().unwrap().remove("manifeste-jeu-110.json");
        assert!(lire_manifeste(&f, true).await.is_err());
        assert_eq!(lire_manifeste(&f, false).await.unwrap().fichiers.len(), 2);
    }

    #[test]
    fn les_empreintes_des_abris_ne_sont_lues_qu_une_fois() {
        let d = tempfile::tempdir().unwrap();
        let f = fichier(d.path(), "110/1790000000/emplacements/k/a.sav", b"abri");
        let date = std::fs::metadata(&f).unwrap().modified().unwrap();
        let cle = "110/1790000000/emplacements/k/a.sav".to_string();
        let mut cache = CacheEmpreintes::default();
        let s1 = cache.empreinte(Some(&cle), &f, &std::fs::metadata(&f).unwrap()).unwrap();
        assert_eq!((s1.as_str(), cache.lus()), (empreinte_octets(b"abri").as_str(), 1));
        let cf = d.path().join("cache.json");
        cache.ecrire(&cf).unwrap();
        // Relu depuis le disque : le fichier n'est plus jamais lu (même taille, même date). Preuve : son contenu
        // changé en douce (même taille, date remise) n'est pas vu.
        std::fs::write(&f, b"ABRI").unwrap();
        std::fs::OpenOptions::new().write(true).open(&f).unwrap().set_modified(date).unwrap();
        let mut cache = CacheEmpreintes::lire(&cf);
        assert_eq!(cache.empreinte(Some(&cle), &f, &std::fs::metadata(&f).unwrap()).unwrap(), s1);
        assert_eq!(cache.lus(), 0);
        // Un fichier qui n'est pas d'un abri (sans clé) : toujours lu.
        cache.empreinte(None, &f, &std::fs::metadata(&f).unwrap()).unwrap();
        assert_eq!(cache.lus(), 1);
    }

    /// Pendant une partie, l'envoi attend entre deux morceaux (jamais la connexion prise au jeu ni au streaming).
    #[tokio::test]
    async fn l_envoi_se_met_en_pause_pendant_une_partie() {
        use crate::firehouse::Cadence as _;
        let regards = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let r = regards.clone();
        // « En partie » aux deux premiers regards, puis fini.
        let pause: Arc<dyn Fn() -> bool + Send + Sync> = Arc::new(move || r.fetch_add(1, std::sync::atomic::Ordering::SeqCst) < 2);
        let mut c = CadenceSauvegarde { morceau: 4, pause: Some(pause), attente: std::time::Duration::from_millis(1) };
        assert_eq!(c.avant(0).await.unwrap(), 4);
        assert_eq!(regards.load(std::sync::atomic::Ordering::SeqCst), 3, "attendu tant que la partie durait");
        let mut sans = CadenceSauvegarde { morceau: 8, pause: None, attente: ATTENTE_PAUSE };
        assert_eq!(sans.avant(0).await.unwrap(), 8);
    }

    #[test]
    fn une_seule_sauvegarde_a_la_fois_par_dossier_de_donnees() {
        let d = tempfile::tempdir().unwrap();
        let v = VerrouSauvegarde::prendre(d.path()).unwrap();
        assert!(VerrouSauvegarde::prendre(d.path()).is_none());
        drop(v);
        assert!(VerrouSauvegarde::prendre(d.path()).is_some());
    }

    #[tokio::test]
    async fn une_sauvegarde_deja_en_cours_est_refusee_par_le_coeur() {
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, None).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        let _v = VerrouSauvegarde::prendre(&n.dossier).unwrap();
        match n.sauvegarder(&[], "PC", false, None, None).await {
            Err(Erreur::Refus(m)) => assert!(m.starts_with(DEJA_EN_COURS), "{m}"),
            r => panic!("{r:?}"),
        }
    }

    /// Pose un abri de sorte `emplacements` (lot S3) : (clé, fichier, octets) ; `en_ligne` par emplacement.
    fn abri_s3(du_jeu: &Path, date: &str, profil: &str, declencheur: &str, fichiers: &[(&str, &str, &[u8])], en_ligne: Option<usize>) {
        let mut mettre = Vec::new();
        let mut lieux: Vec<EmplacementAbrite> = Vec::new();
        for (cle, nom, o) in fichiers {
            let r = format!("emplacements/{cle}/{nom}");
            fichier(&du_jeu.join(date), &r, o);
            mettre.push(crate::nuage_steam::AMettre { source: PathBuf::new(), relatif: r, taille: o.len() as u64, sha: String::new() });
            if !lieux.iter().any(|l| l.cle == *cle) {
                lieux.push(EmplacementAbrite { cle: cle.to_string(), chemin: format!("%APPDATA%\\Jeu\\{cle}"), nature: "dossier".into(), en_ligne, ..Default::default() });
            }
        }
        let mut f = FicheAbri::nouvelle(SORTE_EMPLACEMENTS, profil, lieux, &mettre, en_ligne.unwrap_or(0), 0);
        f.declencheur = declencheur.into();
        std::fs::write(du_jeu.join(format!("{date}.json")), serde_json::to_vec(&f).unwrap()).unwrap();
    }

    fn gog(dossier: &Path) -> crate::boutiques::JeuBoutique {
        crate::boutiques::JeuBoutique {
            id: "gog_1207658924".into(),
            nom: "Alpha".into(),
            minutes: 0,
            derniere: 0,
            installe: true,
            plateforme: "gog".into(),
            image: None,
            dossier: Some(dossier.to_string_lossy().into()),
            pret: None,
            succes: None,
            releases: vec![],
            titre_douteux: false,
        }
    }

    /// Les emplacements envoyés : (chemin chez Firehouse, contenu), sans la fiche.
    fn emplacements_de(c: &Contenu) -> Vec<(String, Vec<u8>)> {
        c.parties.iter().filter(|e| e.relatif.contains("/emplacements/") && !e.relatif.ends_with("fiche.json")).map(|e| (e.relatif.clone(), std::fs::read(&e.source).unwrap())).collect()
    }

    #[tokio::test]
    async fn un_jeu_gog_qui_n_a_que_des_abris_s3_part_avec_son_profil_jamais_avec_un_autre() {
        let d = tempfile::tempdir().unwrap();
        let app = d.path().join("app");
        let n = Noyau::nouveau(&app, Box::new(CoffreMemoire::default())).unwrap();
        let seb = n.creer_profil("Seb", None, None).unwrap().id;
        let lea = n.creer_profil("Léa", None, None).unwrap().id;
        n.regler_abris(Some(d.path().join("Abris")));
        let dossier_jeu = d.path().join("GOG").join("Alpha");
        std::fs::create_dir_all(&dossier_jeu).unwrap();
        let id = crate::ludotheque::id_boutique("gog", "1207658924");
        for p in [&seb, &lea] {
            n.ouvrir(p, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
            n.synchroniser(|_, _| {}).await.unwrap();
            let s = n.session().await.unwrap();
            n.ranger_installes(&s, "gog", &[gog(&dossier_jeu)]).unwrap();
        }
        n.synchroniser_boutique("gog", &[gog(&dossier_jeu)], &["1207658924".to_string()].into()).unwrap();
        let du_jeu = d.path().join("Abris").join(id.to_string());
        // Seb après sa partie, puis Léa (plus récente, même emplacement : même compte Windows).
        abri_s3(&du_jeu, "1790000100", &seb, crate::abri_parties::DECLENCHEUR_APRES_PARTIE, &[("0a1b2c3d", "save.dat", b"seb")], Some(0));
        abri_s3(&du_jeu, "1790000200", &lea, crate::abri_parties::DECLENCHEUR_APRES_PARTIE, &[("0a1b2c3d", "save.dat", b"lea")], Some(0));
        n.ouvrir(&seb, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        let c = n.contenu_sauvegarde(&[]).await.unwrap();
        let rel: Vec<&str> = c.parties.iter().map(|e| e.relatif.as_str()).collect();
        let fiche_r = format!("jeux/{id}/emplacements/fiche.json");
        assert_eq!(rel, [fiche_r.as_str(), &format!("jeux/{id}/emplacements/0a1b2c3d/save.dat")], "la fiche d'abord");
        assert_eq!(emplacements_de(&c)[0].1, b"seb", "l'abri plus récent de Léa ne fait pas sortir celui de Seb");
        let fiche: FicheAbri = serde_json::from_slice(&std::fs::read(&c.parties[0].source).unwrap()).unwrap();
        assert_eq!(fiche.boutique, Some(crate::abri_parties::BoutiqueDuJeu { source: "gog".into(), cle: "1207658924".into() }));
        assert_eq!((fiche.profil.as_str(), fiche.emplacements[0].declencheur.as_str()), (seb.as_str(), "apres_partie"));
        // Léa : seulement les siens.
        n.ouvrir(&lea, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        let c = n.contenu_sauvegarde(&[]).await.unwrap();
        assert_eq!(emplacements_de(&c).iter().map(|(_, o)| o.clone()).collect::<Vec<_>>(), [b"lea".to_vec()]);
        // Envoyée deux fois : la fiche ne change pas (déterministe), rien ne repart.
        let f = Fausse::default();
        envoyer(&f, &c, "PC", "t").await.unwrap();
        let c = n.contenu_sauvegarde(&[]).await.unwrap();
        assert_eq!(envoyer(&f, &c, "PC", "t").await.unwrap().octets, 0);
    }

    #[tokio::test]
    async fn le_compteur_d_un_mineur_ne_compte_jamais_les_abris_de_l_admin() {
        let d = tempfile::tempdir().unwrap();
        let app = d.path().join("app");
        let n = Noyau::nouveau(&app, Box::new(CoffreMemoire::default())).unwrap();
        let admin = n.creer_profil("Seb", None, None).unwrap().id;
        let mineur = n.creer_profil("Zoé", None, None).unwrap().id;
        n.regler_abris(Some(d.path().join("Abris")));
        // Un jeu que personne ne voit dans le catalogue simulé (celui « pour adultes » de l'admin).
        let du_jeu = d.path().join("Abris").join("987654");
        abri_s3(&du_jeu, "1790000100", &admin, crate::abri_parties::DECLENCHEUR_MAIN, &[("0a1b2c3d", "x.sav", b"admin")], Some(0));
        n.ouvrir(&mineur, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();
        let c = n.contenu_sauvegarde(&[]).await.unwrap();
        assert!(c.parties.is_empty() && c.propres.is_empty(), "rien de l'admin");
        assert_eq!(envoyer(&Fausse::default(), &c, "PC", "t").await.unwrap().non_envoyes, 0, "pas même en nombre");
        // Le même abri au nom du mineur (un jeu qui lui est devenu caché) : compté, jamais envoyé.
        abri_s3(&du_jeu, "1790000200", &mineur, crate::abri_parties::DECLENCHEUR_MAIN, &[("0a1b2c3d", "x.sav", b"zoe")], Some(0));
        let c = n.contenu_sauvegarde(&[]).await.unwrap();
        assert!(c.parties.is_empty());
        assert_eq!(envoyer(&Fausse::default(), &c, "PC", "t").await.unwrap().non_envoyes, 1);
        let _ = admin;
    }

    /// I1 (relecture lancement) : un abri du dossier du jeu est commun au PC. Léa retire le jeu (son abri, plus récent) :
    /// la sauvegarde de Seb ne recule pas vers son abri plus ancien et ne retire rien de ce qu'il avait envoyé. Même
    /// règle pour un abri du nuage Steam d'un autre profil (M7).
    #[tokio::test]
    async fn lea_retire_le_jeu_et_la_sauvegarde_de_seb_ne_perd_rien() {
        let d = tempfile::tempdir().unwrap();
        let app = d.path().join("app");
        let n = Noyau::nouveau(&app, Box::new(CoffreMemoire::default())).unwrap();
        let seb = n.creer_profil("Seb", None, None).unwrap().id;
        let lea = n.creer_profil("Léa", None, None).unwrap().id;
        n.regler_abris(Some(d.path().join("Abris")));
        let du_jeu = d.path().join("Abris").join("110");
        let abri_jeu = |date: &str, profil: &str, sorte: &str, o: &[u8]| {
            fichier(&du_jeu.join(date), "C/DUNE.SAV", o);
            let f = FicheAbri::nouvelle(sorte, profil, vec![], &[], 0, 0);
            std::fs::write(du_jeu.join(format!("{date}.json")), serde_json::to_vec(&f).unwrap()).unwrap();
        };
        abri_jeu("1790000100", &seb, crate::abri_parties::SORTE_JEU, b"seb");
        n.ouvrir(&seb, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();
        let f = Fausse::default();
        envoyer(&f, &n.contenu_sauvegarde(&[]).await.unwrap(), "PC", "t").await.unwrap();
        assert_eq!(f.fichiers.lock().unwrap()["jeux/110/C/DUNE.SAV"], b"seb");
        for sorte in [crate::abri_parties::SORTE_JEU, crate::abri_parties::SORTE_NUAGE_STEAM] {
            // Léa (même PC) retire le jeu : son abri est le plus récent.
            abri_jeu(if sorte == crate::abri_parties::SORTE_JEU { "1790000200" } else { "1790000300" }, &lea, sorte, b"lea");
            let c = n.contenu_sauvegarde(&[]).await.unwrap();
            assert!(!c.parties.iter().any(|e| e.relatif == "jeux/110/C/DUNE.SAV"), "jamais l'abri de Léa, jamais un recul vers l'ancien de Seb");
            assert!(c.gardes.contains(&"jeux/110/".to_string()));
            let b = envoyer(&f, &c, "PC", "t").await.unwrap();
            assert_eq!(b.retires, 0, "{sorte}");
            assert_eq!(f.fichiers.lock().unwrap()["jeux/110/C/DUNE.SAV"], b"seb", "la copie de Seb reste chez Firehouse");
            assert!(f.lignes().contains(&"jeux/110/C/DUNE.SAV".to_string()));
        }
    }

    #[tokio::test]
    async fn une_grosse_sauvegarde_automatique_n_envoie_rien_en_un_seul_passage() {
        let d = tempfile::tempdir().unwrap();
        let a = fichier(d.path(), "a.sav", &[7u8; 100]);
        let f = Fausse::default();
        let c = contenu(vec![Element { source: a, relatif: "jeux/1/a.sav".into() }]);
        match envoyer_sous(&f, &c, "PC", "t", &mut CacheEmpreintes::default(), Some(50)).await {
            Err(Erreur::Refus(m)) => assert!(m.starts_with(GROSSE_AUTO), "{m}"),
            r => panic!("{r:?}"),
        }
        assert!(f.fichiers.lock().unwrap().is_empty() && f.docs.lock().unwrap().is_empty());
        assert_eq!(envoyer_sous(&f, &c, "PC", "t", &mut CacheEmpreintes::default(), Some(100)).await.unwrap().envoyes, 1);
    }

    /// I5 : le cache vaut pour TOUS les fichiers envoyés (pas seulement les abris), sa clé porte la racine : un fichier
    /// inchangé n'est jamais relu, ni par l'estimation ni par l'envoi qui la suit.
    #[tokio::test]
    async fn un_fichier_inchange_n_est_jamais_relu() {
        let d = tempfile::tempdir().unwrap();
        let a = fichier(&d.path().join("Emulateurs"), "saves/a.srm", b"partie");
        let f = Fausse::default();
        let c = contenu(vec![Element { source: a.clone(), relatif: "emulateurs/RetroArch/saves/a.srm".into() }]);
        let mut cache = CacheEmpreintes::default();
        preparer(&f, &c, &mut cache).await.unwrap();
        assert_eq!(cache.lus(), 1);
        let cf = d.path().join("cache.json");
        cache.ecrire(&cf).unwrap();
        let mut cache = CacheEmpreintes::lire(&cf);
        envoyer_avec(&f, &c, "PC", "t", &mut cache).await.unwrap();
        assert_eq!(cache.lus(), 0, "l'envoi qui suit ne relit rien");
        assert_ne!(CacheEmpreintes::cle(&d.path().join("A").join("x")), CacheEmpreintes::cle(&d.path().join("B").join("x")), "deux racines, deux clés");
    }

    /// M6 : un manifeste qui ne tiendrait pas chez Firehouse (5 Mo par document) est refusé AVANT d'envoyer quoi que ce
    /// soit.
    #[tokio::test]
    async fn un_manifeste_trop_gros_est_refuse_avant_tout_envoi() {
        let d = tempfile::tempdir().unwrap();
        let f = Fausse::default();
        let mut gros = ManifesteParties::default();
        for i in 0..70_000 {
            gros.fichiers.insert(format!("jeux/1/partie-{i:06}.sav"), ("a".repeat(64), 1));
        }
        assert!(verifier_manifeste(&gros).is_err());
        f.docs.lock().unwrap().insert("manifeste.json".into(), serde_json::to_value(&gros).unwrap());
        let n = fichier(d.path(), "n.sav", b"neuf");
        let r = envoyer(&f, &contenu(vec![Element { source: n, relatif: "jeux/2/n.sav".into() }]), "PC", "t").await;
        assert!(matches!(&r, Err(Erreur::Refus(m)) if m.contains("5 Mo")), "{r:?}");
        assert!(f.fichiers.lock().unwrap().is_empty(), "rien envoyé");
        // Réparti par jeu, un manifeste de même taille passe s'il tient par document.
        let mut reparti = ManifesteParties::default();
        for i in 0..70_000 {
            reparti.fichiers.insert(format!("jeux/{}/p.sav", i % 4), ("a".repeat(64), 1));
            reparti.fichiers.insert(format!("jeux/{}/partie-{i:06}.sav", i % 4), ("a".repeat(64), 1));
        }
        assert!(verifier_manifeste(&reparti).is_ok());
    }

    /// Le motif du contrat (`^manifeste-[a-z0-9][a-z0-9._-]{0,79}\.json$`) vaut pour tout numéro de jeu.
    #[test]
    fn les_noms_des_manifestes_par_jeu_suivent_le_motif_du_contrat() {
        for id in [1i64, 110, -5, i64::MAX, i64::MIN + 1] {
            let cle = cle_manifeste(&format!("jeux/{id}/a.sav")).unwrap();
            let nom = nom_manifeste_jeu(&cle);
            let corps = nom.strip_prefix("manifeste-").and_then(|r| r.strip_suffix(".json")).unwrap();
            assert!(!corps.is_empty() && corps.len() <= 80, "{nom}");
            assert!(corps.as_bytes()[0].is_ascii_lowercase() || corps.as_bytes()[0].is_ascii_digit(), "{nom}");
            assert!(corps.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b)), "{nom}");
        }
        assert_eq!(cle_manifeste("emulateurs/RetroArch/x"), None);
    }

    #[tokio::test]
    async fn l_estimation_n_ecrit_rien_et_dit_ce_qui_partirait() {
        let d = tempfile::tempdir().unwrap();
        let app = d.path().join("app");
        let n = Noyau::nouveau(&app, Box::new(CoffreMemoire::default())).unwrap();
        let seb = n.creer_profil("Seb", None, None).unwrap().id;
        n.regler_abris(Some(d.path().join("Abris")));
        abri_s3(&d.path().join("Abris").join("110"), "1790000100", &seb, "main", &[("0a1b2c3d", "x.sav", b"12345")], Some(0));
        n.ouvrir(&seb, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();
        let dp = app.join("profils").join(&seb);
        let avant: Vec<String> = fichiers_de(&dp).unwrap_or_default();
        let e = n.estimer_sauvegarde(&[], "PC").await.unwrap();
        assert!(e.total >= 5, "{e:?}");
        assert_eq!(e.a_envoyer, None, "mode simulé : Firehouse n'est pas lu");
        assert_eq!(fichiers_de(&dp).unwrap_or_default(), avant, "aucune fiche, aucun cache écrit");
        assert!(!dp.join("fiches-abris").exists());
        // La vraie sauvegarde, elle, écrit la fiche à envoyer.
        n.contenu_sauvegarde(&[]).await.unwrap();
        assert!(dp.join("fiches-abris").join("110.json").is_file());
    }

    /// RÉEL, LECTURE SEULE, sur les abris locaux de Seb (lot S4, à lancer par lui avec son accord) : combien de jeux,
    /// de fichiers et d'octets d'emplacements partiraient pour CE profil, sans RIEN envoyer ni écrire (aucune requête,
    /// aucune fiche, aucun cache). Les jeux cachés à ce profil ne sont pas écartés ici (le catalogue n'est pas lu) : un
    /// majorant. N'affiche que des numéros de jeux et des nombres (jamais un chemin ni un titre).
    /// `FROGTEND_ABRIS=<dossier des abris>[;<autre>] FROGTEND_PROFIL_ESSAI=<id du profil> cargo test essai_abris_s4 -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn essai_abris_s4() {
        let racines: Vec<PathBuf> = std::env::var("FROGTEND_ABRIS").expect("FROGTEND_ABRIS").split(';').filter(|s| !s.trim().is_empty()).map(PathBuf::from).collect();
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let mut par_jeu: BTreeMap<i64, Vec<(u64, PathBuf)>> = BTreeMap::new();
        let (mut abris, mut d_un_autre, mut sans_fiche) = (0usize, 0usize, 0usize);
        for r in &racines {
            for d in std::fs::read_dir(r).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
                let Some(id) = d.file_name().and_then(|n| n.to_str()).and_then(|n| n.parse::<i64>().ok()) else { continue };
                for date in std::fs::read_dir(&d).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
                    let Some(q) = date.file_name().and_then(|n| n.to_str()).and_then(|n| n.parse::<u64>().ok()) else { continue };
                    abris += 1;
                    match crate::abri_parties::lire_fiche(&date) {
                        Some(f) if f.profil != profil => d_un_autre += 1,
                        Some(_) => {}
                        None => sans_fiche += 1,
                    }
                    par_jeu.entry(id).or_default().push((q, date));
                }
            }
        }
        let (mut jeux, mut fichiers, mut octets, mut partiels) = (0usize, 0usize, 0u64, 0usize);
        for (id, l) in &par_jeu {
            let (f, fiche) = crate::abri_parties::emplacements_a_envoyer(l, &profil);
            let Some(fiche) = fiche else { continue };
            let o: u64 = f.iter().filter_map(|(s, _)| std::fs::metadata(s).ok()).map(|m| m.len()).sum();
            let p = fiche.emplacements.iter().filter(|e| e.partiel).count();
            println!("== jeu {id} : {} emplacement(s), {} fichier(s), {} octet(s), {p} partiel(s)", fiche.emplacements.len(), f.len(), o);
            jeux += 1;
            fichiers += f.len();
            octets += o;
            partiels += p;
        }
        println!("== {abris} abri(s) lus ({d_un_autre} d'un autre profil, {sans_fiche} sans fiche)");
        println!("== partiraient pour ce profil : {jeux} jeu(x), {fichiers} fichier(s), {octets} octet(s) ({}), {partiels} emplacement(s) partiel(s) OneDrive", crate::erreurs::taille_lisible(octets));
    }

    mod api {
        use super::*;
        use httpmock::prelude::*;
        use httpmock::Method::HEAD;

        const BASE: &str = "/api/jeux/v1/sauvegarde/Sebastien/VENKMAN";

        fn api(c: &crate::firehouse::Client) -> Api<'_> {
            Api { client: c, profil: "Sebastien".into(), pc: "VENKMAN".into(), morceau: MORCEAU, pause: None }
        }

        #[tokio::test]
        async fn un_fichier_deja_complet_et_identique_chez_firehouse_ne_repart_pas() {
            let s = MockServer::start();
            let d = tempfile::tempdir().unwrap();
            let f = fichier(d.path(), "x.sav", b"partie");
            let sha = empreinte(&f).unwrap();
            // Contrat 1.51 (simulé) : HEAD d'un fichier complet → X-Complet, X-Total, X-Sha256.
            let head = s.mock(|w, t| {
                w.method(HEAD).path(format!("{BASE}/fichier"));
                t.status(200).header("X-Recu", "0").header("X-Complet", "1").header("X-Total", "6").header("X-Sha256", sha.as_str());
            });
            let put = s.mock(|w, t| {
                w.method(PUT).path(format!("{BASE}/fichier"));
                t.status(200).json_body(json!({"ok": true, "complet": true}));
            });
            let c = crate::firehouse::Client::nouveau(&s.base_url(), "j").unwrap();
            assert_eq!(api(&c).envoyer_fichier("jeux/1/x.sav", &f, &sha, 6).await.unwrap(), 0);
            head.assert();
            put.assert_hits(0);
        }

        #[tokio::test]
        async fn en_1_50_un_fichier_complet_repond_x_recu_0_et_repart_en_entier() {
            let s = MockServer::start();
            let d = tempfile::tempdir().unwrap();
            let f = fichier(d.path(), "x.sav", b"partie");
            let sha = empreinte(&f).unwrap();
            s.mock(|w, t| {
                w.method(HEAD).path(format!("{BASE}/fichier"));
                t.status(200).header("X-Recu", "0");
            });
            let put = s.mock(|w, t| {
                w.method(PUT).path(format!("{BASE}/fichier")).header("content-range", "bytes 0-5/6");
                t.status(200).json_body(json!({"ok": true, "complet": true}));
            });
            let c = crate::firehouse::Client::nouveau(&s.base_url(), "j").unwrap();
            assert_eq!(api(&c).envoyer_fichier("jeux/1/x.sav", &f, &sha, 6).await.unwrap(), 6);
            put.assert();
            // Une autre empreinte chez Firehouse (1.51) : il repart aussi.
            let s2 = MockServer::start();
            s2.mock(|w, t| {
                w.method(HEAD).path(format!("{BASE}/fichier"));
                t.status(200).header("X-Recu", "0").header("X-Complet", "1").header("X-Sha256", "ab");
            });
            let put2 = s2.mock(|w, t| {
                w.method(PUT).path(format!("{BASE}/fichier"));
                t.status(200).json_body(json!({"ok": true, "complet": true}));
            });
            let c2 = crate::firehouse::Client::nouveau(&s2.base_url(), "j").unwrap();
            api(&c2).envoyer_fichier("jeux/1/x.sav", &f, &sha, 6).await.unwrap();
            put2.assert();
        }

        /// Un `.part` déjà entier chez Firehouse (X-Recu = total) : le dernier morceau est renvoyé, jamais un morceau vide.
        #[tokio::test]
        async fn un_part_deja_entier_renvoie_le_dernier_morceau_jamais_un_morceau_vide() {
            let s = MockServer::start();
            let d = tempfile::tempdir().unwrap();
            let f = fichier(d.path(), "x.sav", b"0123456789");
            let sha = empreinte(&f).unwrap();
            s.mock(|w, t| {
                w.method(HEAD).path(format!("{BASE}/fichier"));
                t.status(200).header("X-Recu", "10");
            });
            let dernier = s.mock(|w, t| {
                w.method(PUT).path(format!("{BASE}/fichier")).header("content-range", "bytes 8-9/10").body("89");
                t.status(200).json_body(json!({"ok": true, "complet": true}));
            });
            let c = crate::firehouse::Client::nouveau(&s.base_url(), "j").unwrap();
            let a = Api { client: &c, profil: "Sebastien".into(), pc: "VENKMAN".into(), morceau: 4, pause: None };
            assert_eq!(a.envoyer_fichier("jeux/1/x.sav", &f, &sha, 10).await.unwrap(), 2);
            dernier.assert();
        }

        /// Réponse de Firehouse (23/11) : en 1.51, un `.part` entier répond au HEAD `X-Recu` = total et `X-Complet: 0`,
        /// et tout morceau le finalise si l'empreinte colle (200 `{recu, total, complet}`).
        #[tokio::test]
        async fn en_1_51_un_part_entier_se_finalise_par_le_dernier_morceau() {
            let s = MockServer::start();
            let d = tempfile::tempdir().unwrap();
            let f = fichier(d.path(), "x.sav", b"0123456789");
            let sha = empreinte(&f).unwrap();
            s.mock(|w, t| {
                w.method(HEAD).path(format!("{BASE}/fichier"));
                t.status(200).header("X-Recu", "10").header("X-Complet", "0").header("X-Total", "10");
            });
            let dernier = s.mock(|w, t| {
                w.method(PUT).path(format!("{BASE}/fichier")).header("content-range", "bytes 8-9/10");
                t.status(200).json_body(json!({"ok": true, "recu": 10, "total": 10, "complet": true}));
            });
            let c = crate::firehouse::Client::nouveau(&s.base_url(), "j").unwrap();
            let a = Api { client: &c, profil: "Sebastien".into(), pc: "VENKMAN".into(), morceau: 4, pause: None };
            a.envoyer_fichier("jeux/1/x.sav", &f, &sha, 10).await.unwrap();
            dernier.assert_hits(1);
        }

        /// En 1.50, un `.part` entier est une impasse (409 `{recu: total}` à chaque morceau) : le dernier morceau est
        /// renvoyé UNE fois, puis ce fichier est sauté (`Conflit`), jamais une boucle ni l'arrêt de tout.
        #[tokio::test]
        async fn en_1_50_un_part_entier_est_saute_apres_un_seul_renvoi() {
            let s = MockServer::start();
            let d = tempfile::tempdir().unwrap();
            let f = fichier(d.path(), "x.sav", b"0123456789");
            let sha = empreinte(&f).unwrap();
            s.mock(|w, t| {
                w.method(HEAD).path(format!("{BASE}/fichier"));
                t.status(200).header("X-Recu", "0");
            });
            // Le premier morceau : Firehouse a déjà tout (409 {recu: total}).
            s.mock(|w, t| {
                w.method(PUT).path(format!("{BASE}/fichier")).header("content-range", "bytes 0-3/10");
                t.status(409).json_body(json!({"recu": 10}));
            });
            let dernier = s.mock(|w, t| {
                w.method(PUT).path(format!("{BASE}/fichier")).header("content-range", "bytes 8-9/10");
                t.status(409).json_body(json!({"recu": 10}));
            });
            let c = crate::firehouse::Client::nouveau(&s.base_url(), "j").unwrap();
            let a = Api { client: &c, profil: "Sebastien".into(), pc: "VENKMAN".into(), morceau: 4, pause: None };
            let r = a.envoyer_fichier("jeux/1/x.sav", &f, &sha, 10).await;
            assert!(matches!(&r, Err(e @ Erreur::Conflit(_)) if !arrete_tout(e)), "{r:?}");
            dernier.assert_hits(1);
        }

        /// 1.51 : une empreinte refusée sur un `.part` entier l'efface (409 `{recu: 0}`) : de zéro, une seule fois.
        #[tokio::test]
        async fn un_409_recu_0_fait_repartir_de_zero_une_fois() {
            let s = MockServer::start();
            let d = tempfile::tempdir().unwrap();
            let f = fichier(d.path(), "x.sav", b"01234567");
            let sha = empreinte(&f).unwrap();
            s.mock(|w, t| {
                w.method(HEAD).path(format!("{BASE}/fichier"));
                t.status(200).header("X-Recu", "4");
            });
            let suite = s.mock(|w, t| {
                w.method(PUT).path(format!("{BASE}/fichier")).header("content-range", "bytes 4-7/8");
                t.status(409).json_body(json!({"recu": 0}));
            });
            let debut = s.mock(|w, t| {
                w.method(PUT).path(format!("{BASE}/fichier")).header("content-range", "bytes 0-3/8");
                t.status(200).json_body(json!({"ok": true, "complet": true}));
            });
            let c = crate::firehouse::Client::nouveau(&s.base_url(), "j").unwrap();
            let a = Api { client: &c, profil: "Sebastien".into(), pc: "VENKMAN".into(), morceau: 4, pause: None };
            a.envoyer_fichier("jeux/1/x.sav", &f, &sha, 8).await.unwrap();
            suite.assert_hits(1);
            debut.assert_hits(1);
        }

        /// Le sel part sous son nom exact (`abris-sel.json`, contrat 1.51) dans l'espace du profil (`_profil`).
        #[tokio::test]
        async fn le_sel_part_sous_son_nom_et_sa_route_exacts() {
            let s = MockServer::start();
            let m = s.mock(|w, t| {
                w.method(PUT).path("/api/jeux/v1/sauvegarde/Sebastien/_profil/document/abris-sel.json").json_body(json!({"sel": "0123456789abcdef0123456789abcdef"}));
                t.status(200).json_body(json!({"ok": true, "nom": "abris-sel.json", "octets": 45}));
            });
            let c = crate::firehouse::Client::nouveau(&s.base_url(), "j").unwrap();
            api(&c).ecrire_document_profil(crate::abri_parties::NOM_SEL_ENVOYE, &json!({"sel": "0123456789abcdef0123456789abcdef"})).await.unwrap();
            m.assert();
        }

        /// Une vraie sauvegarde dont le sel est refusé (404 `{"detail": "document absent"}`, 1.51 pas encore servie) :
        /// la sauvegarde réussit, l'attente est notée et journalisée SANS la valeur, et le sel local n'est jamais réécrit.
        #[tokio::test]
        async fn un_sel_refuse_est_note_et_journalise_sans_sa_valeur() {
            let s = MockServer::start();
            s.mock(|w, t| {
                w.method(GET).path(format!("{BASE}/document/manifeste.json"));
                t.status(404);
            });
            s.mock(|w, t| {
                w.method(GET).path("/api/jeux/v1/sauvegarde");
                t.status(200).json_body(serde_json::from_slice::<Value>(include_bytes!("../fixtures/sauvegarde-liste-150-reel.json")).unwrap());
            });
            s.mock(|w, t| {
                w.method(PUT).path_contains("/VENKMAN/document/");
                t.status(200).json_body(json!({"ok": true}));
            });
            let sel = s.mock(|w, t| {
                w.method(PUT).path(format!("/api/jeux/v1/sauvegarde/Sebastien/_profil/document/{}", crate::abri_parties::NOM_SEL_ENVOYE));
                t.status(404).json_body(json!({"detail": "document absent"}));
            });
            let d = tempfile::tempdir().unwrap();
            let app = d.path().join("app");
            let n = Noyau::nouveau(&app, Box::new(CoffreMemoire::default())).unwrap();
            let id = n.creer_profil("Sébastien", None, Some("j")).unwrap().id;
            let fichier_sel = app.join("profils").join(&id).join(crate::abri_parties::NOM_SEL);
            std::fs::create_dir_all(fichier_sel.parent().unwrap()).unwrap();
            std::fs::write(&fichier_sel, "fedcba9876543210fedcba9876543210").unwrap();
            n.ouvrir(&id, None, &Connexion { adresse: s.base_url(), simule: false }).await.unwrap();
            let jeu: crate::ludotheque::JeuResume = serde_json::from_value(json!({"id": 110, "titre": "Dune", "plateforme": "MS-DOS"})).unwrap();
            n.session().await.unwrap().verrou_vivant().unwrap().remplacer(&[], &[jeu], "2026-11-23T00:00:00").unwrap();
            let b = n.sauvegarder(&[], "VENKMAN", true, None, None).await.unwrap();
            assert!(b.sel_en_attente);
            sel.assert();
            let journal = std::fs::read_to_string(app.join("journal.txt")).unwrap();
            assert!(journal.contains("clé du profil pas encore rangée"), "{journal}");
            assert!(!journal.contains("fedcba9876543210"), "jamais la valeur du sel : {journal}");
            assert_eq!(std::fs::read_to_string(&fichier_sel).unwrap(), "fedcba9876543210fedcba9876543210", "jamais régénéré");
        }

        /// I5 : l'estimation (Firehouse joint) rend ce qui partirait, la place, et ce qui MANQUE (le calcul de l'envoi) ;
        /// elle écrit le cache local des empreintes, jamais rien chez Firehouse. Hors ligne : rien de plus n'est demandé.
        #[tokio::test]
        async fn l_estimation_dit_ce_qui_manque_ecrit_le_cache_local_et_n_envoie_rien() {
            let s = MockServer::start();
            s.mock(|w, t| {
                w.method(GET).path(format!("{BASE}/document/manifeste.json"));
                t.status(404);
            });
            s.mock(|w, t| {
                w.method(GET).path("/api/jeux/v1/sauvegarde");
                t.status(200).json_body(json!({"ok": true, "profils": [], "utilise_octets": 999_998u64, "quota_octets": 1_000_000u64}));
            });
            let ecritures = s.mock(|w, t| {
                w.method(PUT);
                t.status(200);
            });
            let d = tempfile::tempdir().unwrap();
            let app = d.path().join("app");
            let n = Noyau::nouveau(&app, Box::new(CoffreMemoire::default())).unwrap();
            let id = n.creer_profil("Sébastien", None, Some("j")).unwrap().id;
            n.regler_abris(Some(d.path().join("Abris")));
            abri_s3(&d.path().join("Abris").join("110"), "1790000100", &id, "main", &[("0a1b2c3d", "x.sav", b"12345")], Some(0));
            n.ouvrir(&id, None, &Connexion { adresse: s.base_url(), simule: false }).await.unwrap();
            let jeu: crate::ludotheque::JeuResume = serde_json::from_value(json!({"id": 110, "titre": "Dune", "plateforme": "MS-DOS"})).unwrap();
            n.session().await.unwrap().verrou_vivant().unwrap().remplacer(&[], &[jeu], "2026-11-23T00:00:00").unwrap();
            let e = n.estimer_sauvegarde(&[], "VENKMAN").await.unwrap();
            assert_eq!((e.utilise, e.quota, e.fichiers_a_envoyer), (Some(999_998), Some(1_000_000), Some(2)), "{e:?}");
            assert!(e.manque.is_some_and(|m| m > 0), "{e:?}");
            ecritures.assert_hits(0);
            assert!(app.join("profils").join(&id).join(CacheEmpreintes::NOM).is_file(), "le cache local est écrit");
            assert!(!app.join("profils").join(&id).join("fiches-abris").exists(), "aucune fiche écrite");
            // Hors ligne (adresse fermée) : l'estimation rend ce qu'elle sait, sans erreur.
            let n2 = Noyau::nouveau(&d.path().join("app2"), Box::new(CoffreMemoire::default())).unwrap();
            let id2 = n2.creer_profil("Seb", None, Some("j")).unwrap().id;
            n2.ouvrir(&id2, None, &Connexion { adresse: "http://127.0.0.1:9".into(), simule: false }).await.unwrap();
            let jeu: crate::ludotheque::JeuResume = serde_json::from_value(json!({"id": 110, "titre": "Dune", "plateforme": "MS-DOS"})).unwrap();
            n2.session().await.unwrap().verrou_vivant().unwrap().remplacer(&[], &[jeu], "2026-11-23T00:00:00").unwrap();
            let e = n2.estimer_sauvegarde(&[], "VENKMAN").await.unwrap();
            assert_eq!((e.a_envoyer, e.utilise, e.manque), (None, None, None));
        }

        #[test]
        fn la_vraie_forme_de_moi_sauvegarde_s_accorde_aux_reglages_de_frogtend() {
            let v: Value = serde_json::from_slice(include_bytes!("../fixtures/moi-sauvegarde-150-reel.json")).unwrap();
            assert_eq!(v["document_max_octets"].as_u64(), Some(DOCUMENT_MAX as u64));
            assert_eq!(v["morceau_max"].as_u64(), Some(MORCEAU as u64));
            assert_eq!(v["quota_octets"].as_u64(), Some(107_374_182_400));
            assert_eq!(v["documents_motif"], "manifeste-<clé du jeu>.json");
            let docs: Vec<&str> = v["documents"].as_array().unwrap().iter().filter_map(|x| x.as_str()).collect();
            for nom in ["configuration.json", "bibliotheque.json", "manifeste.json", "derniere-sauvegarde.json"] {
                assert!(docs.contains(&nom), "{nom}");
            }
        }

        #[test]
        fn un_507_est_plein_avec_ses_chiffres_quand_firehouse_les_donne() {
            let reel = include_bytes!("../fixtures/simule/sauvegarde-507-150-simule.json");
            match crate::firehouse::erreur_du_statut(507, reel) {
                Erreur::Plein { manque_octets: None, motif } => assert!(motif.contains("plein"), "{motif}"),
                e => panic!("{e:?}"),
            }
            let simule = include_bytes!("../fixtures/simule/sauvegarde-507-151-simule.json");
            match crate::firehouse::erreur_du_statut(507, simule) {
                Erreur::Plein { manque_octets: Some(52_428_800), motif } => assert!(motif.contains("50,0 Mo"), "{motif}"),
                e => panic!("{e:?}"),
            }
        }

        #[tokio::test]
        async fn la_place_se_lit_dans_la_vraie_forme_de_firehouse() {
            let s = MockServer::start();
            let reel: Value = serde_json::from_slice(include_bytes!("../fixtures/sauvegarde-liste-150-reel.json")).unwrap();
            s.mock(|w, t| {
                w.method(GET).path("/api/jeux/v1/sauvegarde");
                t.status(200).json_body(reel.clone());
            });
            let c = crate::firehouse::Client::nouveau(&s.base_url(), "j").unwrap();
            assert_eq!(lire_place(&c).await.unwrap(), Some(Place { utilise: 30_746_015, quota: 107_374_182_400 }));
        }

        /// Une vraie sauvegarde par l'API qui reçoit 401 au milieu : arrêtée, rien d'écrit (ni manifeste, ni dernière
        /// sauvegarde) ; et le journal commun ne reçoit ni chemin, ni nom de fichier, ni titre.
        #[tokio::test]
        async fn un_401_arrete_tout_et_le_journal_ne_dit_ni_chemin_ni_titre() {
            let s = MockServer::start();
            s.mock(|w, t| {
                w.method(GET).path(format!("{BASE}/document/manifeste.json"));
                t.status(404);
            });
            s.mock(|w, t| {
                w.method(GET).path("/api/jeux/v1/sauvegarde");
                t.status(200).json_body(json!({"ok": true, "profils": [], "utilise_octets": 0, "quota_octets": 107374182400u64}));
            });
            s.mock(|w, t| {
                w.method(HEAD).path(format!("{BASE}/fichier"));
                t.status(404);
            });
            s.mock(|w, t| {
                w.method(PUT).path(format!("{BASE}/fichier"));
                t.status(401);
            });
            let docs = s.mock(|w, t| {
                w.method(PUT).path_contains("/document/");
                t.status(200).json_body(json!({"ok": true}));
            });
            let d = tempfile::tempdir().unwrap();
            let app = d.path().join("app");
            let n = Noyau::nouveau(&app, Box::new(CoffreMemoire::default())).unwrap();
            let id = n.creer_profil("Sébastien", None, Some("j")).unwrap().id;
            n.regler_abris(Some(d.path().join("Abris")));
            abri_s3(&d.path().join("Abris").join("110"), "1790000100", &id, "main", &[("0a1b2c3d", "Partie secrète.sav", b"x")], Some(0));
            n.ouvrir(&id, None, &Connexion { adresse: s.base_url(), simule: false }).await.unwrap();
            // Une ludothèque déjà synchronisée (la sauvegarde l'exige) : le jeu 110 est noté visible.
            let jeu: crate::ludotheque::JeuResume = serde_json::from_value(json!({"id": 110, "titre": "Dune", "plateforme": "MS-DOS"})).unwrap();
            n.session().await.unwrap().verrou_vivant().unwrap().remplacer(&[], &[jeu], "2026-11-23T00:00:00").unwrap();
            match n.sauvegarder(&[], "VENKMAN", true, None, None).await {
                Err(Erreur::JetonRefuse(_)) => {}
                r => panic!("{r:?}"),
            }
            docs.assert_hits(0);
            let journal = std::fs::read_to_string(app.join("journal.txt")).unwrap_or_default();
            assert!(journal.contains("arrêtée (jeton_refuse)"), "{journal}");
            for interdit in ["Partie secrète", "jeux/", "emplacements", "Dune", &*d.path().to_string_lossy()] {
                assert!(!journal.contains(interdit), "« {interdit} » au journal : {journal}");
            }
            assert!(!app.join("profils").join(&id).join("derniere-sauvegarde.json").exists());
        }
    }
}
