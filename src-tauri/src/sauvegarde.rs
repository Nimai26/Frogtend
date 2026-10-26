//! La sauvegarde d'un profil : « tout ce qui ne se retélécharge pas » (lot 3 bis, décision de Seb).
//!
//! Dans `<partage de la personne>\<profil>\<PC>\` :
//! - `configuration.json` : réglages du PC et du profil, émulateurs installés ;
//! - `bibliotheque.json` : la LISTE des jeux (version, emplacement, installation, temps de jeu) — pas les jeux ;
//! - `parties\` : les parties et codes de triche de TOUS les jeux — pour un jeu installé, les fichiers changés depuis
//!   son installation ; pour chaque émulateur, le dossier du profil (`<émulateur>\Profils\<profil>\`) ;
//!   `parties\manifeste.json` donne l'empreinte (sha256) de chacun : seul ce qui a changé est renvoyé ;
//! - `derniere-sauvegarde.json` : quand, depuis quel PC, combien.
//!
//! Frogtend écrit la version COURANTE ; l'historique est gardé par le serveur (instantanés ZFS).

use crate::erreurs::{Erreur, Resultat};
use crate::installation::{fichiers_de, Manifeste};
use crate::noyau::{maintenant, Noyau};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

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
}

/// Ce qu'une sauvegarde enverrait (annoncé avant de l'envoyer).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct EstimationSauvegarde {
    /// La taille de toutes les parties à sauvegarder maintenant.
    pub total: u64,
    /// Celle de la dernière sauvegarde faite depuis ce PC (aucune : la première).
    pub precedent: Option<u64>,
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
    #[serde(default)]
    pub non_envoyes: usize,
}

/// Ce qu'il faut sauvegarder pour un profil.
pub struct Contenu {
    pub configuration: Value,
    pub bibliotheque: Value,
    pub parties: Vec<Element>,
    /// Des préfixes (`jeux/<id>/`) de parties qui sont encore sur le PC mais ne partent pas (jeu que ce profil ne voit
    /// pas, ou plus) : ce qui en avait déjà été envoyé à CE compte est gardé tel quel dans la version courante, jamais
    /// retiré — rien de caché ne part, rien ne sort de la protection de Firehouse.
    pub gardes: Vec<String>,
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
}

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
        use std::io::{Read, Seek, SeekFrom};
        let route = self.route_fichier(relatif);
        if taille == 0 {
            let b = self.client.brute(reqwest::Method::PUT, &route, Some(vec![]), &[("X-Contenu-Sha256", sha.into())]).await?;
            return if (200..300).contains(&b.statut) { Ok(0) } else { Err(Self::refus(&b)) };
        }
        // Reprise : ce que Firehouse a déjà reçu de ce fichier (`.part`).
        let deja = self.client.brute(reqwest::Method::HEAD, &route, None, &[]).await?;
        let mut position = deja.recu.unwrap_or(0).min(taille);
        let mut fichier = std::fs::File::open(source)?;
        let mut envoyes = 0u64;
        let mut conflits = 0;
        loop {
            let fin = (position + self.morceau as u64).min(taille);
            let mut morceau = vec![0u8; (fin - position) as usize];
            fichier.seek(SeekFrom::Start(position))?;
            fichier.read_exact(&mut morceau)?;
            let entetes = [
                ("Content-Range", format!("bytes {position}-{}/{taille}", fin - 1)),
                ("X-Contenu-Sha256", sha.to_string()),
                ("Content-Type", "application/octet-stream".into()),
            ];
            let b = self.client.brute(reqwest::Method::PUT, &route, Some(morceau), &entetes).await?;
            let v: Value = serde_json::from_slice(&b.octets).unwrap_or(Value::Null);
            match b.statut {
                200..=299 => {
                    envoyes += fin - position;
                    if v["complet"] == true || fin == taille {
                        if v["complet"] != true {
                            return Err(Erreur::Serveur(format!("Firehouse n'a pas confirmé « {relatif} ».")));
                        }
                        return Ok(envoyes);
                    }
                    position = v["recu"].as_u64().unwrap_or(fin);
                }
                409 => {
                    conflits += 1;
                    match v["recu"].as_u64() {
                        // Le morceau ne commençait pas là où Firehouse en est : on reprend là.
                        Some(r) if conflits <= 3 => position = r.min(taille),
                        Some(_) => return Err(Erreur::Serveur(format!("L'envoi de « {relatif} » ne se cale pas."))),
                        // Empreinte différente au dernier morceau (le .part est effacé) : le fichier a changé pendant
                        // l'envoi, ou il est arrivé abîmé. Un seul nouvel essai complet.
                        None if conflits <= 1 => position = 0,
                        None => return Err(Erreur::Conflit(format!("« {relatif} » arrive abîmé chez Firehouse."))),
                    }
                }
                _ => return Err(Self::refus(&b)),
            }
        }
    }
    async fn supprimer_fichier(&self, relatif: &str) -> Resultat<()> {
        let b = self.client.brute(reqwest::Method::DELETE, &self.route_fichier(relatif), None, &[]).await?;
        if (200..300).contains(&b.statut) || b.statut == 404 { Ok(()) } else { Err(Self::refus(&b)) }
    }
    fn present(&self, _relatif: &str, _taille: u64) -> Option<bool> {
        None
    }
}

/// Envoie la sauvegarde à la destination : seul ce qui a changé (d'après le manifeste), et ce qui a disparu sort de
/// la version courante.
pub async fn envoyer(d: &impl Destination, contenu: &Contenu, pc: &str, emplacement: &str) -> Resultat<Bilan> {
    let ancien: ManifesteParties = d
        .lire_document("manifeste.json")
        .await?
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    let mut nouveau = ManifesteParties::default();
    let mut bilan = Bilan { pc: pc.into(), dossier: emplacement.into(), date: maintenant(), ..Default::default() };
    for e in &contenu.parties {
        if !e.source.is_file() {
            continue;
        }
        // Un fichier disparu entre-temps (un émulateur qui écrit pendant la sauvegarde) : sauté, jamais tout arrêté.
        let Ok(m) = std::fs::metadata(&e.source) else { continue };
        let taille = m.len();
        let sha = empreinte(&e.source)?;
        let deja = ancien.fichiers.get(&e.relatif) == Some(&(sha.clone(), taille))
            && d.present(&e.relatif, taille).unwrap_or(true);
        if deja {
            bilan.inchanges += 1;
        } else {
            bilan.octets += d.envoyer_fichier(&e.relatif, &e.source, &sha, taille).await?;
            bilan.envoyes += 1;
        }
        bilan.taille += taille;
        nouveau.fichiers.insert(e.relatif.clone(), (sha, taille));
    }
    // Ce qui n'existe plus sur le PC sort de la version courante (l'historique du serveur le garde). Ce qui est encore
    // sur le PC mais ne part plus (`gardes`) reste tel qu'il avait été envoyé.
    for (r, e) in &ancien.fichiers {
        if nouveau.fichiers.contains_key(r) {
            continue;
        }
        if contenu.gardes.iter().any(|g| r.starts_with(g.as_str())) {
            nouveau.fichiers.insert(r.clone(), e.clone());
            continue;
        }
        d.supprimer_fichier(r).await?;
        bilan.retires += 1;
    }
    bilan.non_envoyes = contenu.gardes.len();
    bilan.fichiers = nouveau.fichiers.len();
    d.ecrire_document("configuration.json", &contenu.configuration).await?;
    d.ecrire_document("bibliotheque.json", &contenu.bibliotheque).await?;
    // Le manifeste en dernier : tant qu'il n'est pas écrit, la sauvegarde précédente reste la référence.
    d.ecrire_document("manifeste.json", &serde_json::to_value(&nouveau).unwrap()).await?;
    d.ecrire_document("derniere-sauvegarde.json", &serde_json::to_value(&bilan).unwrap()).await?;
    Ok(bilan)
}

/// Le début du refus « une sauvegarde à reprendre existe » (l'interface le reconnaît pour proposer de la restaurer).
pub const A_REPRENDRE: &str = "Une sauvegarde de ce profil existe déjà chez Firehouse";

/// Avant la PREMIÈRE sauvegarde d'un profil sur ce PC (rien envoyé d'ici, rien restauré, rien écarté) : si
/// Firehouse a déjà, au même nom et au même PC, une sauvegarde avec des parties, l'envoyer la VIDERAIT (ce qui manque
/// ici en sortirait). Refus : la personne la restaure, ou l'écarte en le disant.
pub async fn verifier_reprise(d: &impl Destination) -> Resultat<()> {
    let ancien: ManifesteParties = d.lire_document("manifeste.json").await?.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default();
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

    /// Ce qu'une sauvegarde enverrait, à annoncer AVANT (une première sauvegarde PS3 ou Xbox peut peser des centaines de
    /// Mo) : la taille de toutes les parties, et celle de la dernière sauvegarde. Rien n'est lu au-delà des tailles.
    pub async fn estimer_sauvegarde(&self, programmes_emulateurs: &[String]) -> Resultat<EstimationSauvegarde> {
        let c = self.contenu_sauvegarde(programmes_emulateurs).await?;
        let total = c.parties.iter().filter_map(|e| std::fs::metadata(&e.source).ok()).map(|m| m.len()).sum();
        let precedent = self.derniere_sauvegarde().await.ok().flatten().map(|b| b.taille);
        Ok(EstimationSauvegarde { total, precedent })
    }

    /// Sauvegarde le profil ouvert chez Firehouse (`<username du jeton>/<profil>/<pc>/`). `ecarter` : la personne a
    /// choisi de REMPLACER une sauvegarde existante qu'elle n'a pas restaurée (sinon refus, `verifier_reprise`).
    pub async fn sauvegarder(&self, programmes_emulateurs: &[String], pc: &str, ecarter: bool) -> Resultat<Bilan> {
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
        let contenu = self.contenu_sauvegarde(programmes_emulateurs).await?;
        let api = Api { client, profil: nom_pour_api(&s.profil.nom), pc: nom_pour_api(pc), morceau: MORCEAU };
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
        let bilan = match envoyer(&api, &contenu, pc, "Firehouse").await {
            Ok(b) => b,
            // Firehouse sans les routes de sauvegarde (avant le contrat 1.5).
            Err(Erreur::Introuvable(_)) => {
                return Err(Erreur::Refus(
                    "Firehouse ne propose pas encore la sauvegarde (il faut une version plus récente).".into(),
                ))
            }
            Err(e) => {
                self.journaliser(&format!("sauvegarde du profil {} : {e:?}", s.profil.id));
                return Err(e);
            }
        };
        let f = self.fichier_derniere_sauvegarde(&s.profil.id);
        if let Some(p) = f.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::write(f, serde_json::to_vec_pretty(&bilan).unwrap())?;
        Ok(bilan)
    }

    /// Rassemble ce qu'il faut sauvegarder pour le profil ouvert. `programmes_emulateurs` : les programmes des
    /// émulateurs réglés sur ce PC (leurs dossiers portent les dossiers des profils).
    pub async fn contenu_sauvegarde(&self, programmes_emulateurs: &[String]) -> Resultat<Contenu> {
        let s = self.session().await?;
        let profil = s.profil.clone();
        let configuration = json!({
            "version_frogtend": env!("CARGO_PKG_VERSION"),
            "profil": { "id": profil.id, "nom": profil.nom, "protege": profil.protege },
            "reglages_pc": lire_json(&self.dossier.join("pc.json")),
            "reglages_profil": lire_json(&self.dossier.join("profils").join(format!("{}.json", profil.id))),
            "emulateurs_installes": lire_json(&self.dossier.join("emulateurs.json")),
        });

        // Jamais le jeu importé par une AUTRE personne de ce PC (contrat 1.21 : ce profil peut seulement y jouer).
        let jeux: Vec<_> = self.jeux_du_pc().await?.into_iter().filter(|v| !v.d_un_autre).collect();
        // Les jeux du PC que ce profil ne voit pas : leurs parties ne partent pas, ce qui en avait été envoyé reste.
        let visibles: std::collections::BTreeSet<i64> = jeux.iter().map(|v| v.jeu.id).collect();
        let mut gardes: std::collections::BTreeSet<i64> =
            self.registre().ids()?.into_iter().filter(|id| *id > 0 && !visibles.contains(id)).collect();
        let mut parties = Vec::new();
        let mut liste = Vec::new();
        for vu in &jeux {
            let j = &vu.jeu;
            // Un jeu à cartes (e-Reader) : TOUTES ses cartes, pour les remettre à une restauration (pas seulement la 1re).
            let cartes: Vec<i64> = if crate::jeux_pc::a_cartes(&j.plateforme) {
                let mut v: Vec<i64> = j.fichiers.iter().map(|f| f.version.unwrap_or(j.version)).collect();
                v.dedup();
                v
            } else {
                vec![]
            };
            liste.push(json!({
                "id": j.id, "version": j.version, "titre": j.titre, "plateforme": j.plateforme,
                "dossier": j.dossier, "etat": j.etat, "total": j.total, "installation": j.installation,
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
        let mut plus_recente: std::collections::BTreeMap<i64, (u64, PathBuf)> = std::collections::BTreeMap::new();
        for racine in self.dossiers_abris_a_lire() {
            let Ok(jeux) = std::fs::read_dir(&racine) else { continue };
            for d in jeux.flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
                let Some(id) = d.file_name().and_then(|n| n.to_str()).and_then(|n| n.parse::<i64>().ok()) else { continue };
                for date in std::fs::read_dir(&d).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
                    let quand = date.file_name().and_then(|n| n.to_str()).and_then(|n| n.parse::<u64>().ok()).unwrap_or(0);
                    if plus_recente.get(&id).is_none_or(|(q, _)| quand > *q) {
                        plus_recente.insert(id, (quand, date));
                    }
                }
            }
        }
        // Seulement les jeux que CE profil a le droit de voir (brief § 2 : les abris sont ceux du PC, pas du profil ;
        // après un passage admin → mineur, ceux des jeux cachés ne partent jamais dans l'espace du mineur). Un abri
        // qui ne part plus reste sur le PC, et l'historique de Firehouse garde les envois d'avant.
        let mut par_jeu: std::collections::BTreeMap<i64, Vec<(u64, PathBuf)>> = std::collections::BTreeMap::new();
        for (id, abri) in plus_recente {
            let id = reunions.get(&id).copied().unwrap_or(id);
            if !s.verrou().visible_ou_reuni(id)? {
                gardes.insert(id);
                continue;
            }
            par_jeu.entry(id).or_default().push(abri);
        }
        // Les parties du nuage Steam (lot B2) d'un AUTRE compte Steam du PC ne partent jamais avec ce profil : seulement
        // celles du compte relié à ce profil (sans compte relié : celles qu'il a mises à l'abri, un seul compte).
        let marque_steam = crate::boutiques::lire(&self.dossier.join("profils").join(&profil.id))
            .and_then(|c| crate::nuage_steam::compte(&c.steamid))
            .map(|c| crate::nuage_steam::marque(&c));
        let d_un_autre_compte = |r: &str| {
            r.strip_prefix("nuage-steam/").and_then(|x| x.split('/').next()).is_some_and(|m| marque_steam.as_deref().is_some_and(|s| s != m))
        };
        for (id, mut abris) in par_jeu {
            abris.sort_by(|a, b| b.0.cmp(&a.0));
            for (_, recente) in abris {
                for r in fichiers_de(&recente)? {
                    if d_un_autre_compte(&r) {
                        continue;
                    }
                    let relatif = format!("jeux/{id}/{r}");
                    if !parties.iter().any(|e| e.relatif == relatif) {
                        parties.push(Element { source: recente.join(&r), relatif });
                    }
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
                    for (sorte, d) in crate::eden::dossiers_parties(dossier, &uuid) {
                        if !d.is_dir() {
                            continue;
                        }
                        for r in fichiers_de(&d)? {
                            parties.push(Element { source: d.join(&r), relatif: format!("emulateurs/{nom_emulateur}/{}/{sorte}/{r}", crate::eden::UTILISATEUR_EDEN) });
                        }
                    }
                }
            }
            let du_profil = crate::emulateurs_profils::dossier_du_profil(dossier, &profil.nom);
            if !du_profil.is_dir() {
                continue;
            }
            // Sans les fichiers qui portent une connexion RetroAchievements (jeton) : ils restent sur ce PC.
            // Ni une sauvegarde de MAME en cours d'écriture (elle n'est gardée qu'une fois complète).
            for r in fichiers_de(&du_profil)?
                .into_iter()
                .filter(|r| !crate::emulateurs_profils::porte_un_secret(r) && !r.ends_with(crate::pilotage::ETAT_MAME_NOUVEAU))
            {
                parties.push(Element { source: du_profil.join(&r), relatif: format!("emulateurs/{nom_emulateur}/{r}") });
            }
        }

        // Ce qui ATTEND encore d'une restauration (PC formaté ou neuf : jeu pas encore remis, émulateur pas encore
        // installé) n'est pas sur le PC mais n'est pas perdu : protégé chez Firehouse, jamais retiré de la sauvegarde.
        let attente = self.dossier.join("restauration").join(&profil.id);
        let mut gardes: Vec<String> = gardes.into_iter().map(|id| format!("jeux/{id}/")).collect();
        gardes.extend(gardes_ouverts);
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

        Ok(Contenu { configuration, bibliotheque: json!({ "profil": profil.nom, "jeux": liste }), parties, gardes })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contenu(parties: Vec<Element>) -> Contenu {
        Contenu { configuration: json!({"a": 1}), bibliotheque: json!({"jeux": []}), parties, gardes: vec![] }
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
            let api = Api { client: &c, profil: "Sebastien".into(), pc: "VENKMAN".into(), morceau: 4 };
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
            let api = Api { client: &c, profil: "Sebastien".into(), pc: "VENKMAN".into(), morceau: 4 };
            assert_eq!(api.envoyer_fichier("x", &f, &sha, 10).await.unwrap(), 2);
            decale.assert();
            fin.assert();
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
            let api = Api { client: &c, profil: "Sebastien".into(), pc: "VENKMAN".into(), morceau: 4 };
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
            let api = Api { client: &c, profil: "Sebastien".into(), pc: "VENKMAN".into(), morceau: MORCEAU };
            let contenu = Contenu {
                configuration: serde_json::json!({}),
                bibliotheque: serde_json::json!({"jeux": []}),
                parties: vec![Element { source: f, relatif: "jeux/110/C/DUNECD/DUNE37S0.SAV".into() }],
                gardes: vec![],
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
        let e = n.estimer_sauvegarde(&[emu.join("retroarch.exe").to_string_lossy().to_string()]).await.unwrap();
        assert_eq!(e, EstimationSauvegarde { total: 3, precedent: None });
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
        let c = n.contenu_sauvegarde(&[programme]).await.unwrap();
        let rel: Vec<_> = c.parties.iter().map(|e| e.relatif.as_str()).collect();
        assert_eq!(rel, vec!["emulateurs/Eden/.eden-utilisateur/save/0100000000010000/save.bin"]);
        assert_eq!(std::fs::read(&c.parties[0].source).unwrap(), b"zelda de Seb", "jamais les parties de Léa");
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
        assert!(matches!(n.sauvegarder(&[], "Venkman", false).await, Err(Erreur::Refus(_))));
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
}
