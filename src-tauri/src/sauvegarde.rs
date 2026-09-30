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
use crate::installation::{changes_depuis, fichiers_de, Manifeste};
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
}

/// Ce qu'il faut sauvegarder pour un profil.
pub struct Contenu {
    pub configuration: Value,
    pub bibliotheque: Value,
    pub parties: Vec<Element>,
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
        let taille = std::fs::metadata(&e.source)?.len();
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
    // Ce qui n'existe plus sur le PC sort de la version courante (l'historique du serveur le garde).
    for r in ancien.fichiers.keys() {
        if !nouveau.fichiers.contains_key(r) {
            d.supprimer_fichier(r).await?;
            bilan.retires += 1;
        }
    }
    bilan.fichiers = nouveau.fichiers.len();
    d.ecrire_document("configuration.json", &contenu.configuration).await?;
    d.ecrire_document("bibliotheque.json", &contenu.bibliotheque).await?;
    // Le manifeste en dernier : tant qu'il n'est pas écrit, la sauvegarde précédente reste la référence.
    d.ecrire_document("manifeste.json", &serde_json::to_value(&nouveau).unwrap()).await?;
    d.ecrire_document("derniere-sauvegarde.json", &serde_json::to_value(&bilan).unwrap()).await?;
    Ok(bilan)
}

fn lire_json(p: &Path) -> Value {
    std::fs::read(p).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or(Value::Null)
}

impl Noyau {
    fn fichier_derniere_sauvegarde(&self, profil: &str) -> PathBuf {
        self.dossier.join("profils").join(profil).join("derniere-sauvegarde.json")
    }

    /// La dernière sauvegarde réussie du profil ouvert, depuis ce PC.
    pub async fn derniere_sauvegarde(&self) -> Resultat<Option<Bilan>> {
        let s = self.session().await?;
        Ok(std::fs::read(self.fichier_derniere_sauvegarde(&s.profil.id)).ok().and_then(|o| serde_json::from_slice(&o).ok()))
    }

    /// Sauvegarde le profil ouvert chez Firehouse (`<username du jeton>/<profil>/<pc>/`).
    pub async fn sauvegarder(&self, programmes_emulateurs: &[String], pc: &str) -> Resultat<Bilan> {
        let s = self.session().await?;
        let client = match &s.source {
            crate::source::Source::Firehouse(c) => c,
            crate::source::Source::Simulee => {
                return Err(Erreur::Refus("En mode simulé, rien n'est sauvegardé chez Firehouse.".into()))
            }
        };
        let contenu = self.contenu_sauvegarde(programmes_emulateurs).await?;
        let api = Api { client, profil: nom_pour_api(&s.profil.nom), pc: nom_pour_api(pc), morceau: MORCEAU };
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

        let jeux = self.jeux_du_pc().await?;
        let mut parties = Vec::new();
        let mut liste = Vec::new();
        for vu in &jeux {
            let j = &vu.jeu;
            liste.push(json!({
                "id": j.id, "version": j.version, "titre": j.titre, "plateforme": j.plateforme,
                "dossier": j.dossier, "etat": j.etat, "total": j.total, "installation": j.installation,
                "temps_jeu": j.temps_jeu, "derniere_partie": j.derniere_partie,
            }));
            // Les parties d'un jeu installé : ce qui a changé depuis son installation.
            let Some(i) = &j.installation else { continue };
            let Ok(o) = std::fs::read(self.dossier_medias(j.id).join("manifeste.json")) else { continue };
            let Ok(m) = serde_json::from_slice::<Manifeste>(&o) else { continue };
            let racine = PathBuf::from(&i.dossier);
            if !racine.is_dir() {
                continue;
            }
            for r in changes_depuis(&racine, &m)? {
                parties.push(Element { source: racine.join(&r), relatif: format!("jeux/{}/{r}", j.id) });
            }
        }

        // Les parties MISES À L'ABRI (avant un retrait ou une réinstallation) : la copie la plus récente de chaque jeu,
        // même s'il n'est plus sur le PC — c'est parfois la seule copie qui reste. Elle va au même endroit que les
        // parties d'un jeu installé (`jeux/<id>/…`), et sera reposée à la réinstallation. Un fichier déjà pris dans
        // le jeu installé (plus récent) passe avant.
        if let Ok(entrees) = std::fs::read_dir(self.dossier.join("sauvegardes")) {
            let mut ids: Vec<PathBuf> = entrees.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
            ids.sort();
            for d in ids {
                let Some(id) = d.file_name().and_then(|n| n.to_str()).and_then(|n| n.parse::<i64>().ok()) else { continue };
                let Some(recente) = std::fs::read_dir(&d)
                    .ok()
                    .into_iter()
                    .flatten()
                    .flatten()
                    .map(|e| e.path())
                    .filter(|p| p.is_dir())
                    .max_by_key(|p| p.file_name().and_then(|n| n.to_str()).and_then(|n| n.parse::<u64>().ok()).unwrap_or(0))
                else {
                    continue;
                };
                for r in fichiers_de(&recente)? {
                    let relatif = format!("jeux/{id}/{r}");
                    if !parties.iter().any(|e| e.relatif == relatif) {
                        parties.push(Element { source: recente.join(&r), relatif });
                    }
                }
            }
        }

        // Les parties du profil dans chaque émulateur.
        let mut vus = std::collections::BTreeSet::new();
        for programme in programmes_emulateurs {
            let Some(dossier) = Path::new(programme).parent() else { continue };
            if !vus.insert(dossier.to_path_buf()) {
                continue;
            }
            let nom_emulateur = crate::jeux_pc::nom_de_dossier(&dossier.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
            let du_profil = crate::emulateurs_profils::dossier_du_profil(dossier, &profil.nom);
            if !du_profil.is_dir() {
                continue;
            }
            for r in fichiers_de(&du_profil)? {
                parties.push(Element { source: du_profil.join(&r), relatif: format!("emulateurs/{nom_emulateur}/{r}") });
            }
        }

        Ok(Contenu {
            configuration,
            bibliotheque: json!({ "profil": profil.nom, "jeux": liste }),
            parties,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contenu(parties: Vec<Element>) -> Contenu {
        Contenu { configuration: json!({"a": 1}), bibliotheque: json!({"jeux": []}), parties }
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
    }
}
