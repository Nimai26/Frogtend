//! Le noyau : les profils de ce PC, le profil ouvert et sa ludothèque.
//!
//! Tout ce qui a été reçu pour un profil vit dans SON dossier (`profils/<id>/`) et dans SA session. Fermer ou
//! changer de profil jette la session : rien n'en reste en mémoire pour le suivant.

use crate::coffre::Coffre;
use crate::erreurs::{Erreur, Resultat};
use crate::firehouse::{origine, Client, Reponse};
use crate::ludotheque::{lire_page, lire_plateformes, Cache, Filtre, JeuResume, Liste, Plateforme};
use crate::profils::{Profils, ProfilVisible};
use crate::source::{Image, Source};
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Les visuels d'un jeu que Frogtend demande en plus de la jaquette (types du contrat 1.6).
pub const SORTES_VISUELS: &[&str] =
    &["fond", "capture", "ecran_titre", "logo", "boite_3d", "cartouche", "cartouche_3d", "dos", "tranche", "autre_image"];

/// Garde-fou : jamais plus de pages que ça pour une synchronisation (200 000 jeux à 500 par page = 400).
const PAGES_MAX: u32 = 5_000;

/// Les réglages du PC dont le noyau a besoin (lus dans `pc.json`).
#[derive(Debug, Clone)]
pub struct Connexion {
    pub adresse: String,
    pub simule: bool,
}

pub struct Session {
    pub profil: ProfilVisible,
    pub dossier: PathBuf,
    pub source: Source,
    pub cache: std::sync::Mutex<Cache>,
}

impl Session {
    /// Le cache du profil. Si une opération précédente a échoué en plein travail, le verrou reste utilisable :
    /// le cache SQLite, lui, est protégé par ses transactions.
    pub fn verrou(&self) -> std::sync::MutexGuard<'_, Cache> {
        self.cache.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Taille au-delà de laquelle le journal est mis de côté (`journal.1.txt`) et recommencé.
const TAILLE_MAX_JOURNAL: u64 = 512 * 1024;

#[derive(Debug, Serialize, PartialEq)]
pub struct BilanSynchro {
    /// Jeux du PC rattachés à leur nouvelle fiche pendant cette synchronisation (contrat 1.14) : (ancien, nouveau).
    pub rattaches: Vec<(i64, i64)>,
    /// Jeux dans la ludothèque après la synchronisation.
    pub jeux: usize,
    pub plateformes: usize,
    pub pages: u32,
    pub simule: bool,
    /// `complete` (tout relu) ou `increment` (seulement ce qui a changé).
    pub mode: &'static str,
    /// Jeux reçus (tous en complète, les changés en incrément).
    pub recus: usize,
    /// Jeux sortis de la ludothèque (plus visibles pour ce profil).
    pub retires: usize,
}

pub struct Noyau {
    pub(crate) dossier: PathBuf,
    pub profils: Profils,
    pub(crate) coffre: Box<dyn Coffre>,
    pub(crate) session: Mutex<Option<Arc<Session>>>,
    /// Les jeux présents sur CE PC (tous profils confondus).
    pub(crate) registre: std::sync::Mutex<crate::jeux_pc::Registre>,
    /// Le téléchargement en cours : (jeu, drapeau d'arrêt).
    pub(crate) en_cours: std::sync::Mutex<Option<(i64, Arc<std::sync::atomic::AtomicBool>)>>,
    /// Vrai tant que la file de téléchargements tourne.
    pub(crate) file_active: std::sync::atomic::AtomicBool,
    /// La dernière partie MAME lancée a reçu le script de pause du menu en jeu (sinon, le menu ne promet aucune pause).
    pub(crate) pause_mame: std::sync::atomic::AtomicBool,
    /// Le dossier des abris de parties (réglage du PC « dossierAbris », règle « pas de pieuvre »). `None` : pas réglé.
    pub(crate) abris: std::sync::Mutex<Option<PathBuf>>,
    /// Les jeux du PC en cours d'utilisation (partie, installation) : jamais rattachés à une autre fiche pendant ce
    /// temps (contrat 1.14 : la fin de partie compterait le temps sous un numéro qui n'existe plus).
    /// Un compteur par jeu : plusieurs gardes peuvent se chevaucher (lancement puis suivi de la partie).
    pub(crate) occupes: std::sync::Mutex<std::collections::HashMap<i64, u32>>,
}

/// Un jeu marqué « occupé » tant que cette garde vit.
pub(crate) struct Occupe<'a>(&'a Noyau, i64);

impl Drop for Occupe<'_> {
    fn drop(&mut self) {
        self.0.liberer(self.1);
    }
}

pub(crate) fn maintenant() -> String {
    // Date ISO sans dépendance : secondes depuis 1970, lisibles par l'interface.
    let s = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
    s.to_string()
}

impl Noyau {
    pub fn nouveau(dossier: &Path, coffre: Box<dyn Coffre>) -> Resultat<Self> {
        let registre = crate::jeux_pc::Registre::ouvrir(&dossier.join("jeux.db"))?;
        // Un téléchargement interrompu par la fermeture de Frogtend ne reprend qu'à la demande.
        registre.interrompus_en_pause()?;
        Ok(Noyau {
            dossier: dossier.into(),
            profils: Profils::charger(&dossier.join("profils.json"))?,
            coffre,
            session: Mutex::new(None),
            registre: std::sync::Mutex::new(registre),
            en_cours: std::sync::Mutex::new(None),
            file_active: std::sync::atomic::AtomicBool::new(false),
            pause_mame: std::sync::atomic::AtomicBool::new(false),
            occupes: std::sync::Mutex::new(Default::default()),
            abris: std::sync::Mutex::new(None),
        })
    }

    /// Règle le dossier des abris de parties (vide : aucun).
    pub fn regler_abris(&self, dossier: Option<PathBuf>) {
        *self.abris.lock().unwrap_or_else(|e| e.into_inner()) = dossier.filter(|d| !d.as_os_str().is_empty());
    }

    /// Le dossier des abris de parties ; refusé s'il n'est pas réglé (Frogtend n'écrit pas ailleurs).
    pub fn dossier_abris(&self) -> Resultat<PathBuf> {
        self.abris.lock().unwrap_or_else(|e| e.into_inner()).clone().ok_or_else(|| {
            crate::erreurs::Erreur::Reglage(
                "Choisis d'abord le dossier des abris de parties (⚙ Options ▸ Emplacements) : Frogtend y copie tes parties avant de retirer un jeu.".into(),
            )
        })
    }

    /// Où lire les abris : le dossier réglé, et l'ancien emplacement (`%APPDATA%\…\sauvegardes`, avant 0.30.0) tant
    /// qu'il en reste.
    pub fn dossiers_abris_a_lire(&self) -> Vec<PathBuf> {
        let mut l: Vec<PathBuf> = self.abris.lock().unwrap_or_else(|e| e.into_inner()).iter().cloned().collect();
        l.push(self.dossier.join("sauvegardes"));
        l
    }

    /// Range dans le dossier des abris réglé ceux de l'ancien emplacement (copie vérifiée, puis l'ancienne copie est
    /// retirée ; un abri déjà présent à la même date n'est pas écrasé). Rend le nombre d'abris rangés.
    pub fn migrer_abris(&self) -> Resultat<usize> {
        let Ok(cible) = self.dossier_abris() else { return Ok(0) };
        let ancien = self.dossier.join("sauvegardes");
        let Ok(jeux) = std::fs::read_dir(&ancien) else { return Ok(0) };
        let mut n = 0;
        for jeu in jeux.flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
            let Some(id) = jeu.file_name().map(|x| x.to_os_string()) else { continue };
            for date in std::fs::read_dir(&jeu)?.flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
                let Some(nom) = date.file_name() else { continue };
                let vers = cible.join(&id).join(nom);
                if vers.exists() {
                    continue;
                }
                crate::locale::deplacer_verifie(&date, &vers)?;
                n += 1;
            }
            let _ = std::fs::remove_dir(&jeu); // seulement s'il est vide
        }
        let _ = std::fs::remove_dir(&ancien);
        if n > 0 {
            self.journaliser(&format!("{n} abri(s) de parties rangé(s) dans {}", cible.display()));
        }
        Ok(n)
    }

    /// Écrit une ligne dans le journal de ce PC (`journal.txt`). Jamais de jeton ni de secret : seulement ce qui
    /// aide à comprendre un échec (quelle jaquette, quel motif).
    pub fn journaliser(&self, message: &str) {
        use std::io::Write;
        let fichier = self.dossier.join("journal.txt");
        if std::fs::metadata(&fichier).is_ok_and(|m| m.len() > TAILLE_MAX_JOURNAL) {
            let _ = std::fs::rename(&fichier, self.dossier.join("journal.1.txt"));
        }
        let _ = std::fs::create_dir_all(&self.dossier);
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&fichier) {
            let _ = writeln!(f, "{} {message}", maintenant());
        }
    }

    fn dossier_profil(&self, id: &str) -> PathBuf {
        self.dossier.join("profils").join(id)
    }

    /// Crée un profil. Le jeton (facultatif en mode simulé) part dans le coffre.
    pub fn creer_profil(&self, nom: &str, pin: Option<&str>, jeton: Option<&str>) -> Resultat<ProfilVisible> {
        let jeton = jeton.map(str::trim).filter(|j| !j.is_empty());
        let p = self.profils.creer(nom, pin)?;
        if let Some(j) = jeton {
            if let Err(e) = self.coffre.ranger(&p.id, j) {
                let _ = self.profils.supprimer(&p.id);
                return Err(e);
            }
        }
        Ok(p)
    }

    /// Vérifie un jeton auprès de Firehouse (`/moi`) AVANT de le ranger : rend qui le porte.
    pub async fn verifier_jeton(connexion: &Connexion, jeton: &str) -> Resultat<Value> {
        Client::nouveau(&connexion.adresse, jeton.trim())?.obtenir_json("/moi").await
    }

    pub fn a_un_jeton(&self, id: &str) -> Resultat<bool> {
        Ok(self.coffre.lire(id)?.is_some())
    }

    /// Ouvre un profil (après son PIN) : la session précédente est jetée.
    pub async fn ouvrir(&self, id: &str, pin: Option<&str>, connexion: &Connexion) -> Resultat<ProfilVisible> {
        self.profils.verifier_pin(id, pin)?;
        let profil = ProfilVisible::from(&self.profils.trouver(id)?);
        let source = self.source_pour(id, connexion)?;
        let dossier = self.dossier_profil(id);
        let cache = Cache::ouvrir(&dossier.join("ludotheque.db"))?;
        *self.session.lock().await =
            Some(Arc::new(Session { profil: profil.clone(), dossier, source, cache: std::sync::Mutex::new(cache) }));
        Ok(profil)
    }

    fn source_pour(&self, id: &str, connexion: &Connexion) -> Resultat<Source> {
        if connexion.simule {
            return Ok(Source::Simulee);
        }
        let jeton = self.coffre.lire(id)?.ok_or_else(|| {
            Erreur::JetonRefuse("Ce profil n'a pas encore de jeton Firehouse. Ajoute-le dans « Mon profil ».".into())
        })?;
        Ok(Source::Firehouse(Client::nouveau(&connexion.adresse, &jeton)?))
    }

    pub async fn fermer(&self) {
        *self.session.lock().await = None;
    }

    pub async fn actif(&self) -> Option<ProfilVisible> {
        self.session.lock().await.as_ref().map(|s| s.profil.clone())
    }

    pub async fn session(&self) -> Resultat<Arc<Session>> {
        self.session
            .lock()
            .await
            .clone()
            .ok_or_else(|| Erreur::Profil("Aucun profil n'est ouvert.".into()))
    }

    /// Remplace le jeton du profil ouvert, puis rouvre la session avec lui.
    pub async fn changer_jeton(&self, jeton: &str, connexion: &Connexion) -> Resultat<()> {
        let s = self.session().await?;
        let jeton = jeton.trim();
        if jeton.is_empty() {
            return Err(Erreur::Coffre("Le jeton est vide.".into()));
        }
        self.coffre.ranger(&s.profil.id, jeton)?;
        let source = self.source_pour(&s.profil.id, connexion)?;
        let cache = Cache::ouvrir(&s.dossier.join("ludotheque.db"))?;
        *self.session.lock().await = Some(Arc::new(Session {
            profil: s.profil.clone(),
            dossier: s.dossier.clone(),
            source,
            cache: std::sync::Mutex::new(cache),
        }));
        Ok(())
    }

    /// Refait la connexion du profil ouvert avec les réglages actuels (adresse, mode simulé), sans redemander son PIN.
    pub async fn reconnecter(&self, connexion: &Connexion) -> Resultat<()> {
        let s = self.session().await?;
        let source = self.source_pour(&s.profil.id, connexion)?;
        let cache = Cache::ouvrir(&s.dossier.join("ludotheque.db"))?;
        *self.session.lock().await = Some(Arc::new(Session {
            profil: s.profil.clone(),
            dossier: s.dossier.clone(),
            source,
            cache: std::sync::Mutex::new(cache),
        }));
        Ok(())
    }

    /// Supprime un profil de ce PC : son jeton, son cache, ses jaquettes. Demande son PIN.
    pub async fn supprimer_profil(&self, id: &str, pin: Option<&str>) -> Resultat<()> {
        self.profils.verifier_pin(id, pin)?;
        if self.actif().await.is_some_and(|p| p.id == id) {
            self.fermer().await;
        }
        self.coffre.oublier(id)?;
        // Les secrets des boutiques du profil (lot 9) partent avec lui.
        for boutique in ["steam", "retroachievements", "retroachievements-jeton"] {
            self.coffre.oublier(&crate::boutiques::nom_secret(id, boutique))?;
        }
        let dossier = self.dossier_profil(id);
        // Vérifier ce qu'on supprime : uniquement le dossier de CE profil, sous le dossier de l'application.
        if dossier.starts_with(self.dossier.join("profils")) && dossier.is_dir() {
            std::fs::remove_dir_all(&dossier)?;
        }
        self.profils.supprimer(id)
    }

    /// Synchronise la ludothèque du profil ouvert avec Firehouse.
    ///
    /// Première fois (cache vide) : TOUT le catalogue, qui remplace le cache d'un coup. Ensuite : seulement ce qui
    /// a changé depuis la dernière modification connue (`depuis`), et la page 1 donne `ids_visibles` : tout ce qui
    /// n'y est plus sort du cache. Si un jeu visible est inconnu du cache (il vient de redevenir visible), on
    /// refait une synchronisation complète. `progres(page, jeux)` est appelé après chaque page reçue.
    pub async fn synchroniser(&self, progres: impl Fn(u32, usize)) -> Resultat<BilanSynchro> {
        let mut fusions = None;
        // UNE session pour tout : un changement de profil pendant la synchronisation n'écrit jamais les fusions d'un
        // profil dans le cache d'un autre.
        let s = self.session().await?;
        let mut bilan = self.synchroniser_catalogue(&s, progres, &mut fusions).await?;
        // Les fiches réunies (contrat 1.14) : jamais une raison d'échouer la synchronisation.
        bilan.rattaches = self.suivre_fusions(&s, fusions, bilan.mode == "complete").await;
        Ok(bilan)
    }

    /// Marque un jeu « occupé » (partie, installation) tant que la garde vit.
    pub(crate) fn occuper(&self, id: i64) -> Occupe<'_> {
        self.marquer_occupe(id);
        Occupe(self, id)
    }

    /// Marque un jeu « occupé » sans garde (à rendre par `liberer`) : du lancement d'une partie jusqu'à la fin de son
    /// suivi, qui se passent dans deux tâches différentes.
    pub(crate) fn marquer_occupe(&self, id: i64) {
        *self.occupes.lock().unwrap_or_else(|e| e.into_inner()).entry(id).or_insert(0) += 1;
    }

    pub(crate) fn liberer(&self, id: i64) {
        let mut o = self.occupes.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(n) = o.get_mut(&id) {
            *n -= 1;
            if *n == 0 {
                o.remove(&id);
            }
        }
    }

    /// Contrat 1.14 : garde les fiches réunies de ce profil (rattrapage UNIQUE de celles d'avant, que `depuis` ne
    /// renvoie plus : une seule page 1 sans `depuis`, jamais une relecture complète), puis rattache les jeux du PC.
    async fn suivre_fusions(&self, s: &Session, recues: Option<Vec<crate::ludotheque::Fusion>>, complete: bool) -> Vec<(i64, i64)> {
        let deja = s.verrou().meta("fusions_lues").ok().flatten().as_deref() == Some("1.14");
        let mut fusions = recues;
        let mut completes = complete && fusions.is_some();
        // Seulement face à un Firehouse 1.14 (la page 1 porte `fusions`) : sinon, rien à rattraper.
        if !deja && !completes && fusions.is_some() {
            if let Ok(v) = s.source.catalogue(1, None).await {
                if let Some(l) = lire_page(&v, 1).fusions {
                    fusions = Some(l);
                    completes = true;
                }
            }
        }
        if let Some(l) = &fusions {
            let mut c = s.verrou();
            if let Err(e) = c.garder_fusions(l, completes) {
                // Le rattrapage sera refait à la prochaine synchronisation.
                let _ = c.ecrire_meta("fusions_lues", "0");
                drop(c);
                self.journaliser(&format!("fusions non gardées : {e:?}"));
            } else if completes {
                let _ = c.ecrire_meta("fusions_lues", "1.14");
            }
        }
        self.appliquer_fusions(s).await
    }

    /// Rattache chaque jeu du PC dont la fiche a été réunie, SEULEMENT si tout concorde : le jeu n'est pas en cours
    /// d'utilisation (téléchargement, partie, installation), la nouvelle fiche est visible pour CE profil, elle n'est
    /// pas déjà sur le PC, et elle porte la même version avec les mêmes fichiers (nom et taille). Sinon rien n'est
    /// touché (le jeu reste visible et lançable par sa nouvelle fiche, `visible_ou_reuni`) et c'est noté au journal.
    pub(crate) async fn appliquer_fusions(&self, s: &Session) -> Vec<(i64, i64)> {
        let mut faits = Vec::new();
        let fusions = match s.verrou().fusions() {
            Ok(f) => f,
            Err(_) => return faits,
        };
        for (ancien, nouveau, le) in fusions {
            if ancien <= 0 {
                continue;
            }
            let Ok(Some(j)) = self.registre().jeu(ancien) else { continue };
            // Des médias (et le relevé des parties) encore à l'ancien emplacement, rangés sous l'ancien numéro : pas
            // de rattachement avant leur migration.
            if self.ancien_dossier_medias(ancien).is_dir() {
                continue;
            }
            if self.utilise(ancien) || j.etat == crate::jeux_pc::Etat::EnCours {
                continue; // réessayé à la prochaine synchronisation
            }
            // L'ancien numéro porté de nouveau par une fiche vivante : le jeu garde la sienne.
            if s.verrou().contient(ancien).unwrap_or(true) {
                continue;
            }
            if !s.verrou().contient(nouveau).unwrap_or(false) {
                continue;
            }
            if self.registre().jeu(nouveau).ok().flatten().is_some() {
                self.journaliser(&format!("jeu {ancien} réuni à {nouveau}, déjà sur ce PC : rien n'est fusionné tout seul"));
                continue;
            }
            let Ok(fiche) = s.source.fiche(nouveau).await else { continue };
            let version = fiche["versions"].as_array().and_then(|l| l.iter().find(|v| v["telechargement_id"].as_i64() == Some(j.version)).cloned());
            // 0 = une version de recensement (jamais arrivée) ; un numéro négatif est une vraie version (relevé réel du
            // 18/10 : la fiche 324 porte la version -602).
            let Some(version) = version.filter(|_| j.version != 0) else {
                self.journaliser(&format!("jeu {ancien} réuni à {nouveau} : sa version {} n'y est pas, rattachement à faire avec la personne", j.version));
                continue;
            };
            let mut attendus: Vec<(String, u64)> = version["fichiers"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|f| {
                    let nom = f["nom"].as_str().unwrap_or("").to_string();
                    let nom = match f["dossier"].as_str().filter(|d| !d.is_empty()) {
                        Some(d) => format!("{d}/{nom}"),
                        None => nom,
                    };
                    (nom, f["taille"].as_u64().unwrap_or(0))
                })
                .collect();
            let mut locaux: Vec<(String, u64)> = j.fichiers.iter().map(|f| (f.nom.clone(), f.taille)).collect();
            attendus.sort();
            locaux.sort();
            if attendus != locaux {
                self.journaliser(&format!("jeu {ancien} réuni à {nouveau} : fichiers différents, rattachement à faire avec la personne"));
                continue;
            }
            // Revérifié juste avant d'écrire, sans attente entre les deux, et sous le verrou des jeux occupés : une
            // partie ou un téléchargement lancé pendant la lecture de la nouvelle fiche l'emporte.
            let occupes = self.occupes.lock().unwrap_or_else(|e| e.into_inner());
            let telechargement = self.en_cours.lock().unwrap_or_else(|e| e.into_inner()).as_ref().map(|(i, _)| *i) == Some(ancien);
            let en_cours = self.registre().jeu(ancien).ok().flatten().map(|j| j.etat == crate::jeux_pc::Etat::EnCours).unwrap_or(true);
            if occupes.contains_key(&ancien) || telechargement || en_cours {
                continue;
            }
            let r = self.registre().rattacher(ancien, nouveau, le.as_deref(), &maintenant());
            drop(occupes);
            match r {
                Ok(true) => {}
                Ok(false) => continue,
                Err(e) => {
                    self.journaliser(&format!("rattachement {ancien} → {nouveau} impossible : {e:?}"));
                    continue;
                }
            }
            // La fiche gardée avec le jeu porte désormais le nouveau numéro ; l'ancienne est gardée à côté (rien ne
            // s'efface), notes et annexes de la version installée inchangées.
            if let Ok(Some(j)) = self.registre().jeu(nouveau) {
                let medias = crate::locale::medias_du_jeu(&j);
                let f = medias.join("fiche.json");
                if let Ok(o) = std::fs::read(&f) {
                    let avant = medias.join(format!("fiche-avant-reunion-{ancien}.json"));
                    if !avant.exists() {
                        let _ = std::fs::write(&avant, &o);
                    }
                    if let Ok(mut v) = serde_json::from_slice::<Value>(&o) {
                        v["id"] = Value::from(nouveau);
                        let _ = crate::medias::ecrire(&f, &serde_json::to_vec_pretty(&v).unwrap());
                    }
                }
            }
            self.journaliser(&format!("jeu du PC {ancien} rattaché à sa nouvelle fiche {nouveau} (fiches réunies chez Firehouse)"));
            faits.push((ancien, nouveau));
        }
        faits
    }

    /// Un jeu en cours de téléchargement, de partie ou d'installation.
    fn utilise(&self, id: i64) -> bool {
        self.occupes.lock().unwrap_or_else(|e| e.into_inner()).contains_key(&id)
            || self.en_cours.lock().unwrap_or_else(|e| e.into_inner()).as_ref().map(|(i, _)| *i) == Some(id)
    }

    /// La synchronisation du catalogue elle-même ; `fusions` reçoit celles de la page 1 (contrat 1.14).
    async fn synchroniser_catalogue(
        &self,
        s: &Session,
        progres: impl Fn(u32, usize),
        fusions: &mut Option<Vec<crate::ludotheque::Fusion>>,
    ) -> Resultat<BilanSynchro> {
        let mut depuis = {
            let c = s.verrou();
            // Un cache d'un schéma plus ancien (sans `images`…) : une relecture complète, une fois.
            let a_jour = c.meta("schema_catalogue")?.as_deref() == Some(crate::ludotheque::SCHEMA_CATALOGUE);
            if c.nombre_de_jeux()? > 0 && a_jour { c.derniere_modification()? } else { None }
        };
        let plateformes = lire_plateformes(&s.source.plateformes().await?);
        loop {
            let mut jeux: Vec<JeuResume> = Vec::new();
            let mut ids_visibles: Option<Vec<i64>> = None;
            let mut page = 1;
            loop {
                // Une page coupée par le réseau (la relecture complète dure quelques minutes) : un nouvel essai, pas
                // tout à recommencer.
                let v = match s.source.catalogue(page, depuis.as_deref()).await {
                    Err(Erreur::Reseau(_)) => s.source.catalogue(page, depuis.as_deref()).await?,
                    r => r?,
                };
                let p = lire_page(&v, page);
                if page == 1 {
                    ids_visibles = p.ids_visibles;
                    *fusions = p.fusions;
                }
                let vide = p.jeux.is_empty();
                jeux.extend(p.jeux);
                progres(page, jeux.len());
                match p.encore {
                    Some(true) => {}
                    Some(false) => break,
                    None if vide => break,
                    None => {}
                }
                if page >= PAGES_MAX {
                    break;
                }
                page += 1;
            }
            // Un même jeu reçu deux fois (pages qui bougent pendant la lecture) n'est gardé qu'une fois.
            jeux.sort_by_key(|j| j.id);
            jeux.dedup_by_key(|j| j.id);
            let simule = s.source.est_simulee();

            if let (Some(_), Some(ids)) = (&depuis, ids_visibles) {
                let mut c = s.verrou();
                let recus: std::collections::HashSet<i64> = jeux.iter().map(|j| j.id).collect();
                let mut inconnu = false;
                for id in &ids {
                    if !recus.contains(id) && !c.contient(*id)? {
                        inconnu = true;
                        break;
                    }
                }
                if !inconnu {
                    let (recus, retires) = c.appliquer_increment(&plateformes, &jeux, &ids, &maintenant())?;
                    return Ok(BilanSynchro {
                        jeux: c.nombre_de_jeux()? as usize,
                        plateformes: plateformes.len(),
                        pages: page,
                        simule,
                        mode: "increment",
                        recus,
                        retires,
                        rattaches: vec![],
                    });
                }
                // Un jeu visible nous est inconnu : on relit tout.
                drop(c);
                depuis = None;
                continue;
            }
            // Synchronisation complète (première fois, ou serveur sans `ids_visibles`).
            if depuis.is_some() {
                depuis = None;
                continue;
            }
            let mut c = s.verrou();
            let avant = c.nombre_de_jeux()? as usize;
            c.remplacer(&plateformes, &jeux, &maintenant())?;
            let apres = jeux.len();
            return Ok(BilanSynchro {
                jeux: apres,
                plateformes: plateformes.len(),
                pages: page,
                simule,
                mode: "complete",
                recus: apres,
                retires: avant.saturating_sub(apres),
                rattaches: vec![],
            });
        }
    }

    /// Qui porte le jeton du profil ouvert (`/moi`).
    pub async fn compte(&self) -> Resultat<Value> {
        self.session().await?.source.moi().await
    }

    /// Enregistre le skin dans le compte Firehouse de la personne du profil ouvert.
    pub async fn enregistrer_skin(&self, nom: &str) -> Resultat<()> {
        self.session().await?.source.enregistrer_theme(nom).await
    }

    /// Le registre des jeux du PC (utilisable même après une erreur en plein travail).
    pub(crate) fn registre(&self) -> std::sync::MutexGuard<'_, crate::jeux_pc::Registre> {
        self.registre.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// `locale` : seulement les jeux du PC (« Ma ludothèque ») ; sinon tout le catalogue du profil.
    fn restriction(&self, locale: bool) -> Resultat<Option<Vec<i64>>> {
        Ok(if locale { Some(self.registre().ids()?) } else { None })
    }

    pub async fn plateformes(&self, locale: bool) -> Resultat<Vec<Plateforme>> {
        let s = self.session().await?;
        let seulement = self.restriction(locale)?;
        let c = s.verrou();
        c.plateformes(seulement.as_deref())
    }

    pub async fn genres(&self, plateforme: Option<&str>, locale: bool) -> Resultat<Vec<String>> {
        let s = self.session().await?;
        let seulement = self.restriction(locale)?;
        let c = s.verrou();
        c.genres(plateforme, seulement.as_deref())
    }

    pub async fn lister(&self, filtre: &Filtre) -> Resultat<Liste> {
        let s = self.session().await?;
        let seulement = self.restriction(filtre.ludotheque)?;
        let c = s.verrou();
        // Les jeux de Firehouse installés sur ce PC (filtre « installé ou non »).
        let installes: Vec<i64> = self.registre().tous()?.into_iter().filter(|j| j.installation.is_some()).map(|j| j.id).collect();
        c.lister(filtre, seulement.as_deref(), &installes)
    }

    pub async fn au_hasard(&self, plateforme: Option<&str>, locale: bool) -> Resultat<Option<JeuResume>> {
        let s = self.session().await?;
        let seulement = self.restriction(locale)?;
        let c = s.verrou();
        c.au_hasard(plateforme, seulement.as_deref())
    }

    pub async fn synchronise_le(&self) -> Resultat<Option<String>> {
        let s = self.session().await?;
        let c = s.verrou();
        c.synchronise_le()
    }

    /// La fiche complète d'un jeu : de Firehouse si possible (et gardée), sinon celle du cache (hors ligne).
    pub async fn fiche(&self, id: i64) -> Resultat<FicheLue> {
        let s = self.session().await?;
        // Une fiche réunie (contrat 1.14) que ce profil connaît déjà, et qui n'est pas un jeu du PC pas encore rattaché
        // (celui-ci garde sa fiche) : la nouvelle, une seule fois (jamais de boucle).
        let du_pc = self.registre().jeu(id)?.is_some();
        if !du_pc {
            // `reuni_vers` laisse à une fiche vivante son numéro.
            let vers = s.verrou().reuni_vers(id)?;
            if let Some(n) = vers {
                let mut f = self.fiche_sans_reunion(&s, n).await?;
                f.reunie_dans = Some(n);
                return Ok(f);
            }
        }
        match self.fiche_sans_reunion(&s, id).await {
            // Réunie depuis la dernière synchronisation : la nouvelle, une seule fois.
            // Jamais pour un jeu du PC pas encore rattaché : il garde sa fiche (et son bouton « Jouer »).
            Err(Erreur::Fusionne { nouveau, .. }) if nouveau != id => {
                // Gardée pour la suite (plus d'aller-retour jusqu'à la prochaine synchronisation).
                {
                    let mut c = s.verrou();
                    let f = crate::ludotheque::Fusion { ancien: id, nouveau, le: None };
                    let _ = c.retirer_reunie(id).and_then(|_| c.garder_fusions(&[f], false));
                }
                if du_pc {
                    // Un jeu du PC pas encore rattaché garde sa fiche (et son bouton « Jouer ») : relue sur le disque.
                    return self.fiche_sans_reunion(&s, id).await;
                }
                let mut f = self.fiche_sans_reunion(&s, nouveau).await?;
                f.reunie_dans = Some(nouveau);
                Ok(f)
            }
            r => r,
        }
    }

    async fn fiche_sans_reunion(&self, s: &Session, id: i64) -> Resultat<FicheLue> {
        // Un jeu de la ludothèque du PC : sa fiche est sur le disque (hors ligne), si ce profil a le droit de le voir.
        // Une fiche réunie (contrat 1.14) dont le jeu n'est pas encore rattaché compte aussi : il reste jouable.
        if s.verrou().visible_ou_reuni(id)? && self.registre().jeu(id)?.is_some() {
            if let Some(f) = self.fiche_locale(id) {
                return Ok(FicheLue { fiche: f, hors_ligne: false, locale: true, reunie_dans: None });
            }
        }
        match s.source.fiche(id).await {
            Ok(f) => {
                s.verrou().garder_fiche(id, &f, &maintenant())?;
                Ok(FicheLue { fiche: f, hors_ligne: false, locale: false, reunie_dans: None })
            }
            Err(Erreur::Reseau(motif)) => match s.verrou().fiche(id)? {
                Some(f) => Ok(FicheLue { fiche: f, hors_ligne: true, locale: false, reunie_dans: None }),
                None => Err(Erreur::Reseau(motif)),
            },
            Err(e) => Err(e),
        }
    }

    pub async fn annexe_texte(&self, id: i64, i: u32, cle: &str) -> Resultat<Value> {
        let s = self.session().await?;
        if s.verrou().contient(id)? {
            if let Some(v) = self.annexe_texte_locale(id, i) {
                return Ok(v);
            }
        }
        s.source.annexe_texte(id, i, cle).await
    }

    /// Une jaquette du profil ouvert, en miniature si `largeur` est donnée : depuis son dossier si elle y est (et que
    /// son empreinte n'a pas changé), sinon depuis Firehouse (et gardée). Une absence est retenue une semaine.
    /// Range les jeux d'une source de boutique (\`steam\`, \`galaxy\`) dans la ludothèque du profil, sous la plateforme
    /// « Windows » (décision de Seb, 02/10 : avec les autres jeux, filtrables). Un jeu Steam vu aussi par GOG Galaxy
    /// garde le même id : il n'apparaît qu'une fois (Steam prime s'il est importé à part).
    pub async fn ranger_boutique(&self, source: &str, jeux: &[crate::boutiques::JeuBoutique]) -> Resultat<usize> {
        let s = self.session().await?;
        let mut c = s.verrou();
        let steam_a_part = source == "galaxy" && c.sources_boutiques()?.iter().any(|x| x == "steam");
        let resumes: Vec<JeuResume> = jeux
            .iter()
            .filter_map(|j| {
                // Steam : l'appid ; Galaxy : « gog_123 », « steam_620 »… → (boutique, id dans la boutique).
                let (boutique, cle) = if source == "steam" {
                    ("steam".to_string(), j.id.clone())
                } else {
                    let (b, c) = j.id.split_once('_').unwrap_or(("autre", &j.id));
                    (b.to_string(), c.to_string())
                };
                if steam_a_part && boutique == "steam" {
                    return None;
                }
                Some(JeuResume {
                    id: crate::ludotheque::id_boutique(&boutique, &cle),
                    titre: j.nom.clone(),
                    plateforme: "Windows".into(),
                    statut: Some("boutique".into()),
                    jaquette: Some(true),
                    source: Some(source.into()),
                    boutique: Some(boutique),
                    cle_boutique: Some(j.id.clone()),
                    installe: Some(j.installe),
                    image: j.image.clone(),
                    minutes: Some(j.minutes),
                    succes: j.succes,
                    ..Default::default()
                })
            })
            .collect();
        c.remplacer_boutique(source, &resumes)
    }

    pub async fn jaquette(&self, id: i64, largeur: Option<u32>) -> Resultat<Option<Image>> {
        let s = self.session().await?;
        let Some(jeu) = s.verrou().jeu(id)? else {
            return Ok(None); // un jeu qui n'est pas dans SA ludothèque : rien.
        };
        // Un jeu de boutique : son image publique (GOG, Steam), gardée en cache sur ce PC.
        if id < 0 {
            let cache = self.dossier.join("images-boutiques");
            let octets = match (&jeu.image, jeu.boutique.as_deref(), jeu.cle_boutique.as_deref()) {
                (Some(url), _, _) => crate::boutiques::image_par_adresse(&cache, url).await,
                (None, Some("steam"), Some(cle)) => crate::boutiques::image_steam(&cache, cle.trim_start_matches("steam_")).await,
                _ => None,
            };
            return Ok(octets.map(|o| Image { type_contenu: type_image(&o), octets: o }));
        }
        // Un jeu de la ludothèque du PC : sa jaquette est sur le disque (hors ligne).
        if let Some(octets) = self.jaquette_locale(id, largeur) {
            return Ok(Some(Image { type_contenu: type_image(&octets), octets }));
        }
        if jeu.jaquette == Some(false) {
            return Ok(None); // Firehouse dit ne pas en avoir : inutile de demander.
        }
        // Jamais l'original (contrat 1.6 : 1 à 13 Mo) : au plus la grande miniature de 1 000 pixels.
        let largeur = Some(largeur.unwrap_or(crate::locale::LARGEUR_GRANDE).clamp(100, 1000).div_ceil(100) * 100);
        let taille = largeur.map_or("orig".to_string(), |l| l.to_string());
        let empreinte: String = jeu
            .jaquette_empreinte
            .as_deref()
            .unwrap_or("x")
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .take(32)
            .collect();
        let prefixe = format!("{id}-{taille}-");
        let dossier = s.dossier.join("jaquettes");
        let fichier = dossier.join(format!("{prefixe}{empreinte}.img"));
        let absent = dossier.join(format!("{prefixe}{empreinte}.absent"));
        if let Ok(octets) = std::fs::read(&fichier) {
            return Ok(Some(Image { type_contenu: type_image(&octets), octets }));
        }
        if let Ok(m) = std::fs::metadata(&absent) {
            if m.modified().ok().and_then(|t| t.elapsed().ok()).is_some_and(|d| d.as_secs() < 7 * 24 * 3600) {
                return Ok(None);
            }
        }
        std::fs::create_dir_all(&dossier)?;
        let recue = s.source.media(id, "jaquette", largeur).await?;
        // L'ancienne jaquette de ce jeu (autre empreinte) n'a plus lieu d'être.
        if let Ok(liste) = std::fs::read_dir(&dossier) {
            for e in liste.flatten() {
                let n = e.file_name().to_string_lossy().to_string();
                if n.starts_with(&prefixe) && e.path() != fichier {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
        match recue {
            Some(img) => {
                std::fs::write(&fichier, &img.octets)?;
                Ok(Some(Image { type_contenu: img.type_contenu.or_else(|| type_image(&img.octets)), octets: img.octets }))
            }
            None => {
                if !s.source.est_simulee() {
                    std::fs::write(&absent, b"")?;
                }
                Ok(None)
            }
        }
    }

    /// Un visuel d'un jeu (contrat 1.6, relevé en réel le 15/10) : le premier des `sortes` que le jeu a, en 1 000 px au
    /// plus (jamais l'original). `jaquette` dans la liste passe par `jaquette` (son cache, les jeux de boutique, la
    /// copie hors ligne) : c'est le repli de la grille (`?visuel=boite_3d,jaquette`).
    ///
    /// Gardé pour ce PC et CE profil. Quand le catalogue donne le visuel et son empreinte (`images`) : rangé par jeu
    /// (`<profil>\visuels\<id>\<type>-<largeur>-<empreinte>.img`, écrit puis renommé : jamais un fichier tronqué
    /// gardé), sans durée — relu seulement quand l'empreinte change ; alors les anciennes versions partent, TOUTES
    /// largeurs de l'empreinte en cours gardées (la grille et Taodbox ne s'effacent pas l'une l'autre). Un type absent
    /// d'`images` n'est jamais demandé, et ce qui en était gardé part (image retirée par un admin). Sinon (catalogue
    /// pas encore relu, ou empreinte absente) : l'ancien fonctionnement, redemandé après 7 jours. Hors ligne : ce qui
    /// est gardé.
    pub async fn visuel(&self, id: i64, sortes: &[&str], largeur: u32) -> Resultat<Option<Image>> {
        let s = self.session().await?;
        let Some(jeu) = s.verrou().jeu(id)? else {
            return Ok(None); // un jeu qui n'est pas dans SA ludothèque : rien.
        };
        let largeur = largeur.clamp(100, 1000).div_ceil(100) * 100;
        let dossier = s.dossier.join("visuels");
        let recent = |p: &Path| {
            std::fs::metadata(p).ok().and_then(|m| m.modified().ok()).and_then(|t| t.elapsed().ok()).is_some_and(|d| d.as_secs() < 7 * 24 * 3600)
        };
        let du_jeu = dossier.join(id.to_string());
        // Retire ce qui est gardé de ce jeu pour ce type : tout (`garder` = None), ou tout SAUF l'empreinte en cours
        // (toutes ses largeurs). Les anciens noms (`<id>-<type>-<largeur>.img` à la racine) partent aussi.
        let purger = |sorte: &str, garder: Option<&str>| {
            let prefixe = format!("{sorte}-");
            if let Ok(l) = std::fs::read_dir(&du_jeu) {
                for e in l.flatten() {
                    let n = e.file_name().to_string_lossy().to_string();
                    if n.starts_with(&prefixe) && !garder.is_some_and(|g| n.ends_with(&format!("-{g}.img"))) {
                        let _ = std::fs::remove_file(e.path());
                    }
                }
            }
            for ancien in [format!("{id}-{sorte}-{largeur}.img"), format!("{id}-{sorte}-{largeur}.absent")] {
                let _ = std::fs::remove_file(dossier.join(ancien));
            }
        };
        for sorte in sortes {
            if *sorte == "jaquette" {
                if let Some(img) = self.jaquette(id, Some(largeur)).await? {
                    return Ok(Some(img));
                }
                continue;
            }
            if id <= 0 || !SORTES_VISUELS.contains(sorte) {
                continue;
            }
            let connu = match &jeu.images {
                Some(images) => match images.get(*sorte) {
                    Some(i) => i.empreinte.as_deref().map(|e| e.chars().filter(char::is_ascii_alphanumeric).take(32).collect::<String>()).filter(|e| !e.is_empty()),
                    None => {
                        purger(sorte, None);
                        continue; // le jeu n'a pas ce visuel : rien à demander
                    }
                },
                None => None,
            };
            if let Some(empreinte) = connu {
                let fichier = du_jeu.join(format!("{sorte}-{largeur}-{empreinte}.img"));
                if let Ok(octets) = std::fs::read(&fichier) {
                    return Ok(Some(Image { type_contenu: type_image(&octets), octets }));
                }
                match s.source.media(id, sorte, Some(largeur)).await {
                    Ok(Some(img)) => {
                        // Le garder est un plus (un disque plein n'empêche pas de montrer l'image reçue) : écrit à côté
                        // puis renommé ; les anciennes versions de ce visuel partent.
                        if crate::medias::ecrire(&fichier, &img.octets).is_ok() {
                            purger(sorte, Some(&empreinte));
                        }
                        return Ok(Some(Image { type_contenu: img.type_contenu.or_else(|| type_image(&img.octets)), octets: img.octets }));
                    }
                    Ok(None) => continue,
                    Err(e) if matches!(e, Erreur::Reseau(_) | Erreur::Serveur(_)) => continue,
                    Err(e) => return Err(e),
                }
            }
            // Catalogue pas encore relu : l'ancien fonctionnement (7 jours).
            let fichier = dossier.join(format!("{id}-{sorte}-{largeur}.img"));
            let absent = dossier.join(format!("{id}-{sorte}-{largeur}.absent"));
            if recent(&fichier) {
                if let Ok(octets) = std::fs::read(&fichier) {
                    return Ok(Some(Image { type_contenu: type_image(&octets), octets }));
                }
            }
            if recent(&absent) {
                continue;
            }
            match s.source.media(id, sorte, Some(largeur)).await {
                Ok(Some(img)) => {
                    let _ = std::fs::create_dir_all(&dossier).and_then(|_| std::fs::write(&fichier, &img.octets));
                    let _ = std::fs::remove_file(&absent);
                    return Ok(Some(Image { type_contenu: img.type_contenu.or_else(|| type_image(&img.octets)), octets: img.octets }));
                }
                Ok(None) => {
                    if !s.source.est_simulee() {
                        let _ = std::fs::create_dir_all(&dossier).and_then(|_| std::fs::write(&absent, b""));
                    }
                }
                // Hors ligne ou serveur en panne : l'ancien visuel gardé, même vieux de plus de 7 jours ; sinon le type
                // suivant (un 503 sur le fond n'empêche pas d'essayer la capture).
                Err(e) => {
                    if let Ok(octets) = std::fs::read(&fichier) {
                        return Ok(Some(Image { type_contenu: type_image(&octets), octets }));
                    }
                    if !matches!(e, Erreur::Reseau(_) | Erreur::Serveur(_)) {
                        return Err(e);
                    }
                }
            }
        }
        Ok(None)
    }

    /// Les skins servis par Firehouse (`/themes`, SANS jeton : utilisable dès l'écran « Qui joue ? »), gardés en
    /// cache pour ce PC avec leur `ETag`. `None` en mode simulé. Hors ligne : ceux gardés.
    pub async fn skins(&self, connexion: &Connexion) -> Resultat<Option<Value>> {
        if connexion.simule {
            return Ok(None);
        }
        let fichier = self.dossier.join("skins.json");
        let garde = self.skins_gardes();
        let etag = garde.as_ref().and_then(|v| v["etag"].as_str().map(String::from));
        let catalogue_garde = garde.map(|v| v["catalogue"].clone());
        match Client::public(&connexion.adresse)?.obtenir("/themes", etag.as_deref()).await {
            Ok(Reponse::NonModifie) => Ok(catalogue_garde),
            Ok(Reponse::Corps { octets, etag, .. }) => {
                let catalogue: Value = serde_json::from_slice(&octets)
                    .map_err(|_| Erreur::Serveur("Firehouse a envoyé des skins illisibles.".into()))?;
                std::fs::create_dir_all(&self.dossier)?;
                std::fs::write(&fichier, serde_json::json!({"etag": etag, "catalogue": catalogue}).to_string())?;
                Ok(Some(catalogue))
            }
            Err(Erreur::Reseau(_)) if catalogue_garde.is_some() => Ok(catalogue_garde),
            Err(e) => Err(e),
        }
    }

    fn skins_gardes(&self) -> Option<Value> {
        let t = std::fs::read_to_string(self.dossier.join("skins.json")).ok()?;
        serde_json::from_str(&t).ok()
    }

    /// La vidéo de fond d'un skin (webm), gardée pour ce PC. Son adresse est `video_api`, un chemin depuis la
    /// RACINE du serveur (contrat 1.3) : on la joint à l'origine de l'adresse, jamais à la base de l'API.
    pub async fn video_skin(&self, connexion: &Connexion, nom: &str) -> Resultat<Option<Vec<u8>>> {
        if connexion.simule {
            return Ok(None);
        }
        let Some(catalogue) = self.skins_gardes().map(|v| v["catalogue"].clone()) else { return Ok(None) };
        let Some(chemin) = catalogue["themes"][nom]["video_api"].as_str().map(String::from) else { return Ok(None) };
        if !chemin.starts_with('/') {
            return Ok(None);
        }
        let version: String = catalogue["version"].as_str().unwrap_or("x").chars().filter(char::is_ascii_alphanumeric).take(32).collect();
        let nom_sur: String = nom.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
        let dossier = self.dossier.join("videos");
        let fichier = dossier.join(format!("{nom_sur}-{version}.webm"));
        if let Ok(o) = std::fs::read(&fichier) {
            return Ok(Some(o));
        }
        let octets = Client::public(&connexion.adresse)?.obtenir_absolu(&format!("{}{chemin}", origine(&connexion.adresse)?)).await?;
        std::fs::create_dir_all(&dossier)?;
        // Les vidéos d'une ancienne version des skins partent.
        if let Ok(liste) = std::fs::read_dir(&dossier) {
            for e in liste.flatten() {
                if e.file_name().to_string_lossy().starts_with(&format!("{nom_sur}-")) {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
        std::fs::write(&fichier, &octets)?;
        Ok(Some(octets))
    }

    pub async fn skin_personnel(&self) -> Resultat<Option<Value>> {
        let s = self.session().await?;
        match s.source.theme().await {
            Err(Erreur::Reseau(_)) => Ok(None),
            r => r,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FicheLue {
    pub fiche: Value,
    /// Vrai si Firehouse était injoignable et que la fiche vient du cache.
    pub hors_ligne: bool,
    /// Vrai si la fiche vient des médias gardés sur le PC (jeu de la ludothèque).
    pub locale: bool,
    /// La fiche demandée a été RÉUNIE à celle-ci (contrat 1.14) : l'interface montre la nouvelle à son adresse.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reunie_dans: Option<i64>,
}

/// Le type d'une image d'après ses premiers octets.
pub fn type_image(o: &[u8]) -> Option<String> {
    let t = if o.starts_with(&[0x89, b'P', b'N', b'G']) {
        "image/png"
    } else if o.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "image/jpeg"
    } else if o.len() > 12 && &o[0..4] == b"RIFF" && &o[8..12] == b"WEBP" {
        "image/webp"
    } else if o.starts_with(b"GIF8") {
        "image/gif"
    } else {
        return None;
    };
    Some(t.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coffre::CoffreMemoire;
    use httpmock::prelude::*;
    use serde_json::json;

    const SIMULE: Connexion = Connexion { adresse: String::new(), simule: true };

    fn noyau() -> (tempfile::TempDir, Noyau) {
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(d.path(), Box::new(CoffreMemoire::default())).unwrap();
        (d, n)
    }

    #[tokio::test]
    async fn rien_n_est_lisible_sans_profil_ouvert() {
        let (_d, n) = noyau();
        assert!(matches!(n.lister(&Filtre::default()).await, Err(Erreur::Profil(_))));
    }

    #[tokio::test]
    async fn synchroniser_en_simule_remplit_la_ludotheque() {
        let (_d, n) = noyau();
        let id = n.creer_profil("Seb", Some("1234"), None).unwrap().id;
        n.ouvrir(&id, Some("1234"), &SIMULE).await.unwrap();
        let pages = std::sync::Mutex::new(0);
        let b = n.synchroniser(|p, _| *pages.lock().unwrap() = p).await.unwrap();
        assert!(b.jeux > 30 && b.simule);
        assert_eq!(*pages.lock().unwrap(), b.pages);
        assert_eq!(n.lister(&Filtre::default()).await.unwrap().total as usize, b.jeux);
        assert!(n.plateformes(false).await.unwrap().iter().any(|p| p.nom == "MS-DOS"));
    }

    #[tokio::test]
    async fn un_mauvais_pin_n_ouvre_rien() {
        let (_d, n) = noyau();
        let id = n.creer_profil("Seb", Some("1234"), None).unwrap().id;
        assert!(matches!(n.ouvrir(&id, Some("0000"), &SIMULE).await, Err(Erreur::Pin(_))));
        assert_eq!(n.actif().await, None);
    }

    #[tokio::test]
    async fn changer_de_profil_ne_montre_jamais_la_ludotheque_du_precedent() {
        let (_d, n) = noyau();
        let seb = n.creer_profil("Seb", None, None).unwrap().id;
        let lea = n.creer_profil("Léa", None, None).unwrap().id;
        n.ouvrir(&seb, None, &SIMULE).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();
        assert!(n.lister(&Filtre::default()).await.unwrap().total > 0);

        n.ouvrir(&lea, None, &SIMULE).await.unwrap();
        assert_eq!(n.lister(&Filtre::default()).await.unwrap().total, 0);
        assert!(n.jaquette(110, None).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn en_vrai_sans_jeton_le_profil_ne_s_ouvre_pas() {
        let (_d, n) = noyau();
        let id = n.creer_profil("Seb", None, None).unwrap().id;
        let vrai = Connexion { adresse: "https://jeux.hikari-no-sekai.fr".into(), simule: false };
        assert!(matches!(n.ouvrir(&id, None, &vrai).await, Err(Erreur::JetonRefuse(_))));
    }

    #[tokio::test]
    async fn synchronisation_reelle_paginee_puis_jaquette_gardee() {
        let serveur = MockServer::start();
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/plateformes");
            t.status(200).json_body(json!([{"nom": "MS-DOS", "jeux": 2}]));
        });
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue").query_param("page", "1");
            t.status(200).json_body(json!({"ok": true, "suivante": 2, "jeux": [{"id": 1, "titre": "A", "plateforme": "MS-DOS", "jaquette": true}]}));
        });
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue").query_param("page", "2");
            t.status(200).json_body(json!({"ok": true, "suivante": null, "jeux": [{"id": 2, "titre": "B", "plateforme": "MS-DOS", "jaquette": false}]}));
        });
        let png = [0x89u8, b'P', b'N', b'G', 1, 2, 3];
        let media = serveur.mock(|w, t| {
            // Jamais l'original (contrat 1.6) : sans largeur demandée, la grande miniature de 1 000 pixels.
            w.method(GET).path("/api/jeux/v1/media/1/jaquette").query_param("largeur", "1000");
            t.status(200).body(png);
        });

        let (d, n) = noyau();
        let id = n.creer_profil("Seb", None, Some("jeton-test")).unwrap().id;
        let c = Connexion { adresse: serveur.base_url(), simule: false };
        n.ouvrir(&id, None, &c).await.unwrap();
        let b = n.synchroniser(|_, _| {}).await.unwrap();
        assert_eq!((b.jeux, b.pages), (2, 2));

        let img = n.jaquette(1, None).await.unwrap().unwrap();
        assert_eq!(img.type_contenu.as_deref(), Some("image/png"));
        n.jaquette(1, None).await.unwrap().unwrap();
        media.assert_hits(1); // la seconde fois vient du disque
        // Le jeu 2 n'a pas de jaquette selon Firehouse : aucune requête.
        assert!(n.jaquette(2, None).await.unwrap().is_none());
        assert!(d.path().join("profils").join(&id).join("jaquettes").join("1-1000-x.img").is_file());
    }

    #[tokio::test]
    async fn hors_ligne_la_fiche_deja_lue_reste_consultable() {
        let (_d, n) = noyau();
        let id = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        // Premier passage en simulé pour garder la fiche de Dune…
        n.ouvrir(&id, None, &SIMULE).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();
        assert!(!n.fiche(110).await.unwrap().hors_ligne);
        // … puis Firehouse injoignable.
        let panne = Connexion { adresse: "http://127.0.0.1:9".into(), simule: false };
        n.ouvrir(&id, None, &panne).await.unwrap();
        let f = n.fiche(110).await.unwrap();
        assert!(f.hors_ligne);
        assert_eq!(f.fiche["titre"], "Dune");
        assert!(n.lister(&Filtre::default()).await.unwrap().total > 0);
    }

    #[tokio::test]
    async fn supprimer_un_profil_efface_son_jeton_et_son_dossier_seulement() {
        let (d, n) = noyau();
        let seb = n.creer_profil("Seb", Some("1234"), Some("j1")).unwrap().id;
        let lea = n.creer_profil("Léa", None, Some("j2")).unwrap().id;
        for id in [&seb, &lea] {
            n.ouvrir(id, if id == &seb { Some("1234") } else { None }, &SIMULE).await.unwrap();
            n.synchroniser(|_, _| {}).await.unwrap();
        }
        // Les clés de ses comptes (Steam, RetroAchievements) partent avec lui ; celles de Léa restent.
        for b in ["steam", "retroachievements", "retroachievements-jeton"] {
            n.coffre.ranger(&crate::boutiques::nom_secret(&seb, b), "cle").unwrap();
            n.coffre.ranger(&crate::boutiques::nom_secret(&lea, b), "cle").unwrap();
        }
        assert!(n.supprimer_profil(&seb, Some("0000")).await.is_err());
        n.supprimer_profil(&seb, Some("1234")).await.unwrap();
        for b in ["steam", "retroachievements", "retroachievements-jeton"] {
            assert!(n.coffre.lire(&crate::boutiques::nom_secret(&seb, b)).unwrap().is_none());
            assert!(n.coffre.lire(&crate::boutiques::nom_secret(&lea, b)).unwrap().is_some());
        }
        assert!(!d.path().join("profils").join(&seb).exists());
        assert!(d.path().join("profils").join(&lea).join("ludotheque.db").exists());
        assert!(!n.a_un_jeton(&seb).unwrap());
        assert!(n.a_un_jeton(&lea).unwrap());
    }

    #[tokio::test]
    async fn reconnecter_change_de_source_sans_redemander_le_pin() {
        let (_d, n) = noyau();
        let id = n.creer_profil("Seb", Some("1234"), None).unwrap().id;
        n.ouvrir(&id, Some("1234"), &SIMULE).await.unwrap();
        let vrai = Connexion { adresse: "https://jeux.hikari-no-sekai.fr".into(), simule: false };
        // Sans jeton, le passage au vrai serveur est refusé, et le dit.
        assert!(matches!(n.reconnecter(&vrai).await, Err(Erreur::JetonRefuse(_))));
        n.reconnecter(&SIMULE).await.unwrap();
        assert!(n.session().await.unwrap().source.est_simulee());
    }

    #[tokio::test]
    async fn la_seconde_synchronisation_est_incrementale_et_retire_les_invisibles() {
        let serveur = MockServer::start();
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/plateformes");
            t.status(200).json_body(json!({"ok": true, "plateformes": [{"nom": "MS-DOS", "jeux": 2}]}));
        });
        // Synchronisation incrémentale : seulement le jeu 1 a changé ; le jeu 2 n'est plus visible.
        let increment = serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue").query_param("depuis", "2026-09-29T10:00:00");
            t.status(200).json_body(json!({"ok": true, "suivante": null, "ids_visibles": [1],
                "jeux": [{"id": 1, "titre": "A (nouvelle version)", "plateforme": "MS-DOS", "maj_le": "2026-09-30T08:00:00"}]}));
        });
        let complete = serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue");
            t.status(200).json_body(json!({"ok": true, "suivante": null, "jeux": [
                {"id": 1, "titre": "A", "plateforme": "MS-DOS", "maj_le": "2026-09-29T10:00:00"},
                {"id": 2, "titre": "B", "plateforme": "MS-DOS", "maj_le": "2026-09-28T10:00:00"}]}));
        });
        let (_d, n) = noyau();
        let id = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: serveur.base_url(), simule: false }).await.unwrap();

        let b1 = n.synchroniser(|_, _| {}).await.unwrap();
        assert_eq!((b1.mode, b1.jeux), ("complete", 2));
        let b2 = n.synchroniser(|_, _| {}).await.unwrap();
        assert_eq!((b2.mode, b2.recus, b2.retires, b2.jeux), ("increment", 1, 1, 1));
        assert_eq!(n.lister(&Filtre::default()).await.unwrap().jeux[0].titre, "A (nouvelle version)");
        increment.assert_hits(1);
        complete.assert_hits(1);
    }

    #[tokio::test]
    async fn le_visuel_choisi_suit_son_empreinte_et_se_replie_sur_la_jaquette() {
        // Contrat 1.6 (relevé réel du 15/10) : le catalogue donne les visuels du jeu et leur empreinte.
        let serveur = MockServer::start();
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/plateformes");
            t.status(200).json_body(json!({"ok": true, "plateformes": [{"nom": "SNES", "jeux": 2}]}));
        });
        let mut catalogue = serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue");
            t.status(200).json_body(json!({"ok": true, "suivante": null, "jeux": [
                {"id": 1, "titre": "A", "plateforme": "SNES", "jaquette": true, "jaquette_empreinte": "j1",
                 "images": {"jaquette": {"empreinte": "j1", "largeur": 1281, "hauteur": 2311}, "boite_3d": {"empreinte": "b1", "largeur": 900, "hauteur": 1200}}},
                {"id": 2, "titre": "B (MS-DOS, aucun visuel)", "plateforme": "SNES", "jaquette": false, "images": {}}]}));
        });
        let boite = serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/media/1/boite_3d").query_param("largeur", "400");
            t.status(200).header("Content-Type", "image/webp").body(b"RIFF\0\0\0\0WEBPVP8 b1".to_vec());
        });
        let jaquette = serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/media/1/jaquette");
            t.status(200).header("Content-Type", "image/webp").body(b"RIFF\0\0\0\0WEBPVP8 j1".to_vec());
        });
        let (_d, n) = noyau();
        let id = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: serveur.base_url(), simule: false }).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();
        let visuels = n.session().await.unwrap().dossier.join("visuels");

        // Le visuel choisi, gardé sous son empreinte, sans durée : redemandé jamais tant qu'elle ne change pas.
        assert!(n.visuel(1, &["boite_3d", "jaquette"], 400).await.unwrap().unwrap().octets.ends_with(b"b1"));
        assert!(n.visuel(1, &["boite_3d", "jaquette"], 400).await.unwrap().is_some());
        boite.assert_hits(1);
        assert!(visuels.join("1").join("boite_3d-400-b1.img").is_file());
        // Une autre largeur du même visuel (Taodbox, le menu en jeu) : gardée À CÔTÉ, jamais l'une efface l'autre.
        let mut boite_1000 = serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/media/1/boite_3d").query_param("largeur", "1000");
            t.status(200).header("Content-Type", "image/webp").body(b"RIFF\0\0\0\0WEBPVP8 b1".to_vec());
        });
        assert!(n.visuel(1, &["boite_3d"], 1000).await.unwrap().is_some());
        assert!(n.visuel(1, &["boite_3d"], 400).await.unwrap().is_some());
        assert!(n.visuel(1, &["boite_3d"], 1000).await.unwrap().is_some());
        boite.assert_hits(1);
        boite_1000.assert_hits(1);
        boite_1000.delete();
        // Un type que le jeu n'a pas : JAMAIS demandé, repli sur la jaquette.
        let cartouche = serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/media/1/cartouche");
            t.status(200).body(b"x".to_vec());
        });
        assert!(n.visuel(1, &["cartouche", "jaquette"], 400).await.unwrap().unwrap().octets.ends_with(b"j1"));
        cartouche.assert_hits(0);
        jaquette.assert_hits(1);
        // Un jeu sans aucun visuel (MS-DOS au 16/10) : aucune requête, rien à montrer.
        assert!(n.visuel(2, &["boite_3d", "jaquette"], 400).await.unwrap().is_none());
        cartouche.assert_hits(0);

        // L'admin change la boîte 3D (nouvelle empreinte) puis retire la cartouche… : la boîte est relue, l'ancienne
        // part ; plus tard la boîte retirée : son fichier part aussi.
        catalogue.delete();
        catalogue = serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue");
            t.status(200).json_body(json!({"ok": true, "suivante": null, "jeux": [
                {"id": 1, "titre": "A", "plateforme": "SNES", "jaquette": true, "jaquette_empreinte": "j1", "maj_le": "2026-10-18T10:00:00",
                 "images": {"jaquette": {"empreinte": "j1"}, "boite_3d": {"empreinte": "b2", "largeur": 900, "hauteur": 1200}}},
                {"id": 2, "titre": "B", "plateforme": "SNES", "jaquette": false, "images": {}}], "ids_visibles": [1, 2]}));
        });
        n.synchroniser(|_, _| {}).await.unwrap();
        assert!(n.visuel(1, &["boite_3d"], 400).await.unwrap().is_some());
        boite.assert_hits(2);
        assert!(visuels.join("1").join("boite_3d-400-b2.img").is_file());
        assert!(!visuels.join("1").join("boite_3d-400-b1.img").exists(), "l'ancienne version part");
        assert!(!visuels.join("1").join("boite_3d-1000-b1.img").exists(), "toutes ses largeurs");
        catalogue.delete();
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue");
            t.status(200).json_body(json!({"ok": true, "suivante": null, "jeux": [
                {"id": 1, "titre": "A", "plateforme": "SNES", "jaquette": true, "jaquette_empreinte": "j1", "maj_le": "2026-10-18T11:00:00",
                 "images": {"jaquette": {"empreinte": "j1"}}},
                {"id": 2, "titre": "B", "plateforme": "SNES", "jaquette": false, "images": {}}], "ids_visibles": [1, 2]}));
        });
        n.synchroniser(|_, _| {}).await.unwrap();
        assert!(n.visuel(1, &["boite_3d"], 400).await.unwrap().is_none());
        assert!(!visuels.join("1").join("boite_3d-400-b2.img").exists(), "un visuel retiré par un admin quitte le cache");
        boite.assert_hits(2);
    }

    /// Essai réel du lot Médias M3 (Taodbox) : le logo seul et le fond d'un jeu qui en a (116), rien pour un jeu sans
    /// images (110, Dune), le second appel servi par le cache.
    /// `FROGTEND_PROFIL_ESSAI=<id> cargo test essai_taodbox -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_taodbox() {
        use crate::coffre::Coffre as _;
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").unwrap();
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton");
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(d.path(), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Essai", None, Some(&jeton)).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: "https://jeux.hikari-no-sekai.fr".into(), simule: false }).await.unwrap();
        let b = n.synchroniser(|_, _| {}).await.unwrap();
        println!("SYNCHRO {} jeux", b.jeux);
        for (jeu, sortes, largeur) in [(116, vec!["logo"], 600), (116, vec!["fond", "capture", "ecran_titre"], 1000), (110, vec!["logo"], 600), (110, vec!["fond", "capture", "ecran_titre"], 1000)] {
            for passage in 1..=2 {
                let debut = std::time::Instant::now();
                let img = n.visuel(jeu, &sortes, largeur).await.unwrap();
                println!(
                    "VISUEL {jeu} {sortes:?} passage {passage} : {:?} en {:?}",
                    img.as_ref().map(|i| (i.octets.len(), i.type_contenu.clone(), i.octets.get(8..12).map(|m| String::from_utf8_lossy(m).to_string()))),
                    debut.elapsed()
                );
            }
        }
        println!("FIN");
    }

    /// RÉEL, lecture seule chez Firehouse, dans un profil d'ESSAI (dossier temporaire ; le catalogue de Seb n'est pas
    /// touché) : synchronisation complète, `images` gardées, puis le visuel `FROGTEND_VISUEL` du jeu `FROGTEND_JEU`.
    /// `FROGTEND_PROFIL_ESSAI=<id> FROGTEND_JEU=116 FROGTEND_VISUEL=boite_3d cargo test essai_visuels -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_visuels() {
        use crate::coffre::Coffre as _;
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").unwrap();
        let jeu: i64 = std::env::var("FROGTEND_JEU").unwrap().parse().unwrap();
        let sorte = std::env::var("FROGTEND_VISUEL").unwrap();
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton");
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(d.path(), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Essai", None, Some(&jeton)).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: "https://jeux.hikari-no-sekai.fr".into(), simule: false }).await.unwrap();
        let debut = std::time::Instant::now();
        let b = n.synchroniser(|_, _| {}).await.unwrap();
        println!("SYNCHRO {} jeux en {:?}", b.jeux, debut.elapsed());
        let s = n.session().await.unwrap();
        let (avec, vides, inconnues) = {
            let c = s.verrou();
            let tous = c.lister(&Filtre::default(), None, &[]).unwrap();
            let l: Vec<JeuResume> = tous.jeux;
            (l.iter().filter(|j| j.images.as_ref().is_some_and(|m| !m.is_empty())).count(), l.iter().filter(|j| j.images.as_ref().is_some_and(|m| m.is_empty())).count(), l.iter().filter(|j| j.images.is_none()).count())
        };
        println!("IMAGES : avec {avec}, vides {vides}, inconnues {inconnues} (sur la première page de la liste)");
        let j = s.verrou().jeu(jeu).unwrap().expect("jeu");
        println!("JEU {jeu} images = {:?}", j.images.as_ref().map(|m| m.iter().map(|(k, v)| format!("{k}:{:?}x{:?}", v.largeur, v.hauteur)).collect::<Vec<_>>()));
        let img = n.visuel(jeu, &[sorte.as_str(), "jaquette"], 400).await.unwrap();
        println!("VISUEL {sorte} : {:?} octets", img.as_ref().map(|i| i.octets.len()));
        for f in std::fs::read_dir(s.dossier.join("visuels")).into_iter().flatten().flatten() {
            println!("GARDÉ {}", f.file_name().to_string_lossy());
        }
        let schema = s.verrou().meta("schema_catalogue").unwrap();
        println!("SCHÉMA {schema:?}");
        // La galerie (M2) : la liste, puis la 2e capture par son id.
        let l = n.medias(jeu).await.unwrap();
        println!("GALERIE {} visuels", l.len());
        if let Some(m) = l.iter().filter(|m| m.sorte == "capture").nth(1) {
            let i = n.media_precis(jeu, m.id, 300).await.unwrap();
            println!("CAPTURE 2 (id {}) : {:?} octets", m.id, i.map(|i| i.octets.len()));
        }
        for f in std::fs::read_dir(s.dossier.join("visuels").join(jeu.to_string())).into_iter().flatten().flatten() {
            println!("GALERIE GARDÉE {}", f.file_name().to_string_lossy());
        }
    }

    /// RÉEL, lecture seule : la liste des visuels d'un jeu (`/jeu/{id}/medias`) et deux captures DIFFÉRENTES demandées
    /// par leur `id` (`/media/{id}/capture?id=<media>&largeur=300`).
    /// `FROGTEND_PROFIL_ESSAI=<id> FROGTEND_JEU=116 cargo test essai_medias -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_medias() {
        use crate::coffre::Coffre as _;
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").unwrap();
        let jeu: i64 = std::env::var("FROGTEND_JEU").unwrap().parse().unwrap();
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton");
        let c = Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap();
        let v: Value = c.obtenir_json(&format!("/jeu/{jeu}/medias")).await.unwrap();
        let l = v["medias"].as_array().cloned().unwrap_or_default();
        println!("MEDIAS {} ; empreinte {} ; premier {}", l.len(), v["medias_empreinte"], l.first().cloned().unwrap_or_default());
        let captures: Vec<&Value> = l.iter().filter(|m| m["type"] == "capture").collect();
        for m in captures.iter().take(2) {
            let r = c.obtenir(&format!("/media/{jeu}/capture?id={}&largeur=300", m["id"]), None).await;
            let o = match r {
                Ok(Reponse::Corps { octets, .. }) => { use sha2::Digest; format!("{} o, sha {:.12x}", octets.len(), sha2::Sha256::digest(&octets)) }
                autre => format!("{:?}", autre.map(|_| ())),
            };
            println!("CAPTURE id={} principal={} → {}", m["id"], m["principal"], o);
        }
        // Un numéro inconnu, et un numéro de capture demandé sous une autre sorte.
        let mid = captures.first().map(|m| m["id"].to_string()).unwrap_or_default();
        for route in [format!("/media/{jeu}/capture?id=1&largeur=300"), format!("/media/{jeu}/logo?id={mid}&largeur=300")] {
            let r = c.obtenir(&route, None).await;
            let o = match r {
                Ok(Reponse::Corps { octets, type_contenu, .. }) => { use sha2::Digest; format!("{} o {:?}, sha {:.12x}", octets.len(), type_contenu, sha2::Sha256::digest(&octets)) }
                autre => format!("{:?}", autre.map(|_| ())),
            };
            println!("ROUTE {route} → {o}");
        }
    }

    /// RÉEL, lecture seule : contrat 1.14 (« fusionné dans N ») — la réponse brute de `/jeu/1927` et la liste `fusions`
    /// de la page 1 du catalogue. `FROGTEND_PROFIL_ESSAI=<id> cargo test essai_fusions -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_fusions() {
        use crate::coffre::Coffre as _;
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").unwrap();
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton");
        let c = Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap();
        let r = c.brute(reqwest::Method::GET, "/jeu/1927", None, &[]).await.unwrap();
        println!("JEU 1927 : {}, corps {}", r.statut, String::from_utf8_lossy(&r.octets));
        let v: Value = c.obtenir_json("/catalogue?page=1").await.unwrap();
        let f = v["fusions"].as_array().cloned().unwrap_or_default();
        println!("FUSIONS {} ; premières {:?}", f.len(), f.iter().take(3).collect::<Vec<_>>());
        let anciens: Vec<String> = f.iter().filter_map(|x| x["ancien"].as_i64()).map(|x| x.to_string()).collect();
        println!("ANCIENS {}", anciens.join(","));
    }

    /// Un jeu du PC (version 200, un fichier « jeu.exe » de 6 octets), dans le dossier donné.
    fn jeu_du_pc(id: i64, dossier: &Path) -> crate::jeux_pc::JeuPc {
        crate::jeux_pc::JeuPc {
            id,
            version: 200,
            titre: "Super C".into(),
            plateforme: "NES".into(),
            dossier: dossier.to_string_lossy().into(),
            etat: crate::jeux_pc::Etat::Telecharge,
            total: 6,
            fichiers: vec![crate::jeux_pc::FichierJeu { n: 0, nom: "jeu.exe".into(), taille: 6 }],
            message: None,
            ajoute_le: "1".into(),
            ajoute_par: "seb".into(),
            installation: None,
            temps_jeu: 3600,
            derniere_partie: Some("2026-10-01".into()),
        }
    }

    /// Un Firehouse 1.14 : 324 visible, 1927 réuni à 324 (relevé réel du 18/10) ; la fiche 324 porte `fichiers_324`.
    fn serveur_reunion(fichiers_324: Value) -> MockServer {
        serveur_reunion_version(fichiers_324, 200)
    }

    fn serveur_reunion_version(fichiers_324: Value, version: i64) -> MockServer {
        let s = MockServer::start();
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/plateformes");
            t.status(200).json_body(json!({"ok": true, "plateformes": [{"nom": "NES", "jeux": 2}]}));
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue");
            t.status(200).json_body(json!({"ok": true, "suivante": null,
                "jeux": [{"id": 324, "titre": "Super C", "plateforme": "NES"}, {"id": 500, "titre": "Autre", "plateforme": "NES"}],
                "fusions": [{"ancien": 1927, "nouveau": 324, "le": "2026-10-05T14:02:37"}, {"ancien": 1, "nouveau": 1}, {"ancien": "x"}]}));
        });
        s.mock(move |w, t| {
            w.method(GET).path("/api/jeux/v1/jeu/324");
            t.status(200).json_body(json!({"ok": true, "jeu": {"id": 324, "titre": "Super C", "plateforme": "NES",
                "versions": [{"telechargement_id": version, "qualite": "ROM", "fichiers": fichiers_324}]}}));
        });
        s
    }

    async fn ouvert(s: &MockServer) -> (tempfile::TempDir, Noyau) {
        let (d, n) = noyau();
        let id = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: s.base_url(), simule: false }).await.unwrap();
        (d, n)
    }

    #[tokio::test]
    async fn un_jeu_du_pc_dont_la_fiche_est_reunie_est_rattache_sans_rien_perdre() {
        let s = serveur_reunion(json!([{"nom": "jeu.exe", "taille": 6, "dossier": ""}]));
        let (d, n) = ouvert(&s).await;
        let dossier = d.path().join("Jeux").join("Probotector 2");
        std::fs::create_dir_all(dossier.join("Frogtend").join("medias")).unwrap();
        std::fs::write(dossier.join("Frogtend").join("medias").join("fiche.json"), json!({"id": 1927, "titre": "Probotector 2"}).to_string()).unwrap();
        n.registre().ajouter(&jeu_du_pc(1927, &dossier)).unwrap();
        n.registre().compter_partie(1927, 3600, "2026-10-01").unwrap();

        let b = n.synchroniser(|_, _| {}).await.unwrap();
        assert_eq!(b.rattaches, vec![(1927, 324)]);
        // Tout est là sous le nouveau numéro : version, dossier, temps de jeu ; l'ancien n'existe plus.
        let j = n.registre().jeu(324).unwrap().expect("rattaché");
        assert_eq!((j.version, j.temps_jeu, j.dossier.clone()), (200, 3600, dossier.to_string_lossy().to_string()));
        assert!(n.registre().jeu(1927).unwrap().is_none());
        assert_eq!(n.registre().reunions().unwrap(), vec![(1927, 324)]);
        // La fiche gardée porte le nouveau numéro ; l'ancienne est gardée à côté.
        let medias = dossier.join("Frogtend").join("medias");
        let f: Value = serde_json::from_slice(&std::fs::read(medias.join("fiche.json")).unwrap()).unwrap();
        assert_eq!(f["id"], 324);
        assert!(medias.join("fiche-avant-reunion-1927.json").is_file());
        // Il reste visible (Ma ludothèque, lancement).
        assert_eq!(n.jeux_du_pc().await.unwrap().len(), 1);
        assert!(n.jeu_visible(324).await.is_ok());
        // Les fusions mal formées sont laissées ; une seconde synchronisation ne refait rien.
        assert!(n.synchroniser(|_, _| {}).await.unwrap().rattaches.is_empty());
    }

    #[tokio::test]
    async fn un_ancien_numero_rendu_a_une_fiche_vivante_reste_le_sien() {
        let s = MockServer::start();
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/plateformes");
            t.status(200).json_body(json!({"ok": true, "plateformes": [{"nom": "NES", "jeux": 2}]}));
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue");
            t.status(200).json_body(json!({"ok": true, "suivante": null,
                "jeux": [{"id": 324, "titre": "Super C", "plateforme": "NES"}, {"id": 1927, "titre": "Probotector 2", "plateforme": "NES"}],
                "fusions": [{"ancien": 1927, "nouveau": 324}]}));
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/jeu/1927");
            t.status(200).json_body(json!({"ok": true, "jeu": {"id": 1927, "titre": "Probotector 2"}}));
        });
        let (d, n) = ouvert(&s).await;
        n.registre().ajouter(&jeu_du_pc(1927, &d.path().join("A"))).unwrap();
        assert!(n.synchroniser(|_, _| {}).await.unwrap().rattaches.is_empty());
        assert!(n.registre().jeu(1927).unwrap().is_some());
        assert_eq!(n.session().await.unwrap().verrou().reuni_vers(1927).unwrap(), None);
        let f = n.fiche(1927).await.unwrap();
        assert_eq!((f.fiche["id"].as_i64(), f.reunie_dans), (Some(1927), None));
    }

    #[tokio::test]
    async fn les_parties_de_l_ancien_et_du_nouveau_numero_sont_reunies() {
        let s = serveur_reunion(json!([{"nom": "jeu.exe", "taille": 6}]));
        let (d, n) = ouvert(&s).await;
        n.registre().ajouter(&jeu_du_pc(1927, &d.path().join("A"))).unwrap();
        assert!(n.registre().rattacher(1927, 324, None, "x").unwrap());
        // Un abri fait avant le rattachement (sous 1927), un autre après (sous 324).
        let vieux = d.path().join("sauvegardes").join("1927").join("100");
        let neuf = d.path().join("sauvegardes").join("324").join("200");
        std::fs::create_dir_all(&vieux).unwrap();
        std::fs::create_dir_all(&neuf).unwrap();
        std::fs::write(vieux.join("seule.dat"), b"seule").unwrap();
        std::fs::write(vieux.join("commun.dat"), b"vieux").unwrap();
        std::fs::write(neuf.join("commun.dat"), b"neuf").unwrap();
        let mut c: Vec<(String, Vec<u8>)> =
            n.contenu_sauvegarde(&[]).await.unwrap().parties.iter().map(|e| (e.relatif.clone(), std::fs::read(&e.source).unwrap())).collect();
        c.sort();
        // Rien ne sort de la sauvegarde ; pour un même fichier, la copie la plus récente.
        assert_eq!(c, vec![("jeux/324/commun.dat".into(), b"neuf".to_vec()), ("jeux/324/seule.dat".into(), b"seule".to_vec())]);

        // Un autre PC : le jeu y est ajouté directement sous 324 (jamais rattaché), ses parties restaurées attendent
        // sous 1927 — l'ancien numéro connu du profil suffit.
        let profil = n.session().await.unwrap().profil.id.clone();
        let zone = d.path().join("restauration").join(&profil).join("parties").join("jeux").join("1927");
        std::fs::create_dir_all(&zone).unwrap();
        std::fs::write(zone.join("save.dat"), b"partie").unwrap();
        let jeu = d.path().join("B");
        std::fs::create_dir_all(&jeu).unwrap();
        assert_eq!(n.reposer_parties_du_jeu(&profil, 324, &[1927], &jeu).unwrap().reposes, 1);
        assert_eq!(std::fs::read(jeu.join("save.dat")).unwrap(), b"partie");
    }

    #[tokio::test]
    async fn une_fiche_reunie_decouverte_a_l_ouverture_est_retenue() {
        let s = MockServer::start();
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/plateformes");
            t.status(200).json_body(json!({"ok": true, "plateformes": [{"nom": "NES", "jeux": 2}]}));
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue");
            t.status(200).json_body(json!({"ok": true, "suivante": null, "jeux": [
                {"id": 324, "titre": "Super C", "plateforme": "NES"}, {"id": 1927, "titre": "Probotector 2", "plateforme": "NES"}]}));
        });
        let ancien = s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/jeu/1927");
            t.status(410).json_body(json!({"ok": false, "fusionne_dans": 324, "detail": "réuni"}));
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/jeu/324");
            t.status(200).json_body(json!({"ok": true, "jeu": {"id": 324, "titre": "Super C"}}));
        });
        let (_d, n) = ouvert(&s).await;
        n.synchroniser(|_, _| {}).await.unwrap();
        assert_eq!(n.fiche(1927).await.unwrap().reunie_dans, Some(324));
        // Retenue : l'ancienne adresse mène à la nouvelle sans redemander l'ancienne.
        assert_eq!(n.fiche(1927).await.unwrap().reunie_dans, Some(324));
        ancien.assert_hits(1);
        // Une chaîne s'arrête sur une fiche vivante : 5 → 1927 (réunie) → 324 (vivante).
        let c = n.session().await.unwrap();
        c.verrou().garder_fusions(&[crate::ludotheque::Fusion { ancien: 5, nouveau: 1927, le: None }], false).unwrap();
        assert_eq!(c.verrou().reuni_vers(5).unwrap(), Some(324));
    }

    #[tokio::test]
    async fn une_liste_complete_de_fusions_remplace_l_ancienne() {
        let s = serveur_reunion(json!([]));
        let (_d, n) = ouvert(&s).await;
        let f = |a, b| crate::ludotheque::Fusion { ancien: a, nouveau: b, le: None };
        let session = n.session().await.unwrap();
        session.verrou().garder_fusions(&[f(7, 324)], false).unwrap();
        session.verrou().garder_fusions(&[f(8, 324)], false).unwrap();
        assert_eq!(session.verrou().fusions().unwrap().len(), 2, "une liste partielle s'ajoute");
        session.verrou().garder_fusions(&[f(8, 324)], true).unwrap();
        assert_eq!(session.verrou().fusions().unwrap(), vec![(8, 324, None)], "une liste complète remplace");
    }

    #[tokio::test]
    async fn un_jeu_occupe_n_est_jamais_rattache() {
        let s = serveur_reunion(json!([{"nom": "jeu.exe", "taille": 6}]));
        let (d, n) = ouvert(&s).await;
        n.registre().ajouter(&jeu_du_pc(1927, &d.path().join("A"))).unwrap();
        // Deux gardes sur le même jeu (lancement puis suivi) : la première qui tombe ne libère pas l'autre.
        n.marquer_occupe(1927);
        {
            let _g = n.occuper(1927);
        }
        assert!(n.synchroniser(|_, _| {}).await.unwrap().rattaches.is_empty());
        n.liberer(1927);
        assert_eq!(n.synchroniser(|_, _| {}).await.unwrap().rattaches, vec![(1927, 324)]);
    }

    #[tokio::test]
    async fn un_telechargement_tombe_sur_une_fiche_reunie_reprend_sous_le_nouveau_numero() {
        let s = MockServer::start();
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/plateformes");
            t.status(200).json_body(json!({"ok": true, "plateformes": [{"nom": "NES", "jeux": 1}]}));
        });
        // Un catalogue d'avant la réunion (1927 encore vivant, pas de `fusions`) : seule la file la découvre.
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue");
            t.status(200).json_body(json!({"ok": true, "suivante": null, "jeux": [
                {"id": 324, "titre": "Super C", "plateforme": "NES"}, {"id": 1927, "titre": "Probotector 2", "plateforme": "NES"}]}));
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/jeu/324");
            t.status(200).json_body(json!({"ok": true, "jeu": {"id": 324, "titre": "Super C", "plateforme": "NES",
                "versions": [{"telechargement_id": 200, "qualite": "ROM", "fichiers": [{"nom": "jeu.exe", "taille": 6}]}]}}));
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/fichier/1927/200/0");
            t.status(410).json_body(json!({"ok": false, "fusionne_dans": 324, "detail": "ce jeu a été réuni à la fiche 324"}));
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/fichier/324/200/0");
            t.status(200).body("abcdef");
        });
        let (d, n) = ouvert(&s).await;
        n.synchroniser(|_, _| {}).await.unwrap();
        let dossier = d.path().join("A");
        std::fs::create_dir_all(&dossier).unwrap();
        let mut j = jeu_du_pc(1927, &dossier);
        j.etat = crate::jeux_pc::Etat::Attente;
        n.registre().ajouter(&j).unwrap();
        n.executer_file(n.session().await.unwrap(), &|_| {}).await.unwrap();
        assert!(n.registre().jeu(1927).unwrap().is_none());
        let j = n.registre().jeu(324).unwrap().expect("rattaché");
        assert_eq!(j.etat, crate::jeux_pc::Etat::Telecharge, "{:?}", j.message);
        assert_eq!(std::fs::read(dossier.join("jeu.exe")).unwrap(), b"abcdef");
        // L'ancien numéro est sorti du catalogue gardé ; sa réunion est connue.
        let c = n.session().await.unwrap();
        assert!(!c.verrou().contient(1927).unwrap());
        assert_eq!(c.verrou().reuni_vers(1927).unwrap(), Some(324));
    }

    #[tokio::test]
    async fn un_rattachement_douteux_ne_touche_a_rien_mais_le_jeu_reste_jouable() {
        // Fichiers différents sur la nouvelle fiche : pas de rattachement automatique.
        let s = serveur_reunion(json!([{"nom": "autre.exe", "taille": 9}]));
        let (d, n) = ouvert(&s).await;
        n.registre().ajouter(&jeu_du_pc(1927, &d.path().join("A"))).unwrap();
        assert!(n.synchroniser(|_, _| {}).await.unwrap().rattaches.is_empty());
        assert!(n.registre().jeu(1927).unwrap().is_some(), "rien n'est touché");
        // … mais il reste visible et lançable par sa nouvelle fiche (ses parties continuent d'être sauvegardées).
        assert_eq!(n.jeux_du_pc().await.unwrap().len(), 1);
        assert!(n.jeu_visible(1927).await.is_ok());
        // Sa fiche reste la sienne, gardée avec lui (bouton « Jouer ») : jamais renvoyé vers la fiche 324.
        let medias = d.path().join("A").join("Frogtend").join("medias");
        std::fs::create_dir_all(&medias).unwrap();
        std::fs::write(medias.join("fiche.json"), json!({"id": 1927, "titre": "Probotector 2"}).to_string()).unwrap();
        let f = n.fiche(1927).await.unwrap();
        assert!(f.locale && f.reunie_dans.is_none(), "fiche locale, sans renvoi");
        assert_eq!(f.fiche["id"], 1927);

        // Le nouveau numéro est DÉJÀ sur le PC : rien n'est fusionné tout seul.
        let s = serveur_reunion(json!([{"nom": "jeu.exe", "taille": 6}]));
        let (d, n) = ouvert(&s).await;
        n.registre().ajouter(&jeu_du_pc(1927, &d.path().join("A"))).unwrap();
        n.registre().ajouter(&jeu_du_pc(324, &d.path().join("B"))).unwrap();
        assert!(n.synchroniser(|_, _| {}).await.unwrap().rattaches.is_empty());
        assert!(n.registre().jeu(1927).unwrap().is_some() && n.registre().jeu(324).unwrap().is_some());

        // Une partie en cours : réessayé plus tard.
        let s = serveur_reunion(json!([{"nom": "jeu.exe", "taille": 6}]));
        let (d, n) = ouvert(&s).await;
        n.registre().ajouter(&jeu_du_pc(1927, &d.path().join("A"))).unwrap();
        {
            let _partie = n.occuper(1927);
            assert!(n.synchroniser(|_, _| {}).await.unwrap().rattaches.is_empty());
        }
        assert_eq!(n.synchroniser(|_, _| {}).await.unwrap().rattaches, vec![(1927, 324)], "après la partie");
    }

    #[tokio::test]
    async fn une_version_au_numero_negatif_se_rattache_et_ses_parties_suivent() {
        // Relevé réel du 18/10 : la fiche 324 porte la version -602 (seul 0 est une version de recensement).
        let s = serveur_reunion_version(json!([{"nom": "jeu.exe", "taille": 6}]), -602);
        let (d, n) = ouvert(&s).await;
        let mut j = jeu_du_pc(1927, &d.path().join("A"));
        j.version = -602;
        n.registre().ajouter(&j).unwrap();
        assert_eq!(n.synchroniser(|_, _| {}).await.unwrap().rattaches, vec![(1927, 324)]);
        // Une version 0 (recensement, jamais arrivée) : jamais rattachée toute seule.
        let s0 = serveur_reunion_version(json!([{"nom": "jeu.exe", "taille": 6}]), 0);
        let (d0, n0) = ouvert(&s0).await;
        let mut j0 = jeu_du_pc(1927, &d0.path().join("A"));
        j0.version = 0;
        n0.registre().ajouter(&j0).unwrap();
        assert!(n0.synchroniser(|_, _| {}).await.unwrap().rattaches.is_empty());

        // La restauration retrouve les parties attendues sous l'ANCIEN numéro (sauvegarde d'avant le rattachement),
        // et la plus récente (sous le nouveau) l'emporte.
        let profil = n.session().await.unwrap().profil.id.clone();
        let zone = d.path().join("restauration").join(&profil).join("parties").join("jeux");
        std::fs::create_dir_all(zone.join("1927")).unwrap();
        std::fs::write(zone.join("1927").join("save.dat"), b"ancienne").unwrap();
        std::fs::write(zone.join("1927").join("seule.dat"), b"seule").unwrap();
        std::fs::create_dir_all(zone.join("324")).unwrap();
        std::fs::write(zone.join("324").join("save.dat"), b"recente").unwrap();
        let jeu = d.path().join("A");
        std::fs::create_dir_all(&jeu).unwrap();
        let r = n.reposer_parties_du_jeu(&profil, 324, &[], &jeu).unwrap();
        assert_eq!(std::fs::read(jeu.join("save.dat")).unwrap(), b"recente");
        assert_eq!(std::fs::read(jeu.join("seule.dat")).unwrap(), b"seule");
        assert!(r.reposes >= 2);
        assert!(!zone.join("1927").exists() && !zone.join("324").exists());
    }

    #[tokio::test]
    async fn les_fusions_d_avant_sont_rattrapees_une_seule_fois() {
        let s = MockServer::start();
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/plateformes");
            t.status(200).json_body(json!({"ok": true, "plateformes": [{"nom": "NES", "jeux": 1}]}));
        });
        // L'incrément (avec `depuis`) ne renvoie plus les fusions anciennes ; la page 1 complète, oui.
        let complet = s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue").matches(|r| !r.query_params.as_ref().is_some_and(|q| q.iter().any(|(k, _)| k == "depuis")));
            t.status(200).json_body(json!({"ok": true, "suivante": null, "jeux": [{"id": 324, "titre": "Super C", "plateforme": "NES", "maj_le": "2026-10-01T00:00:00"}],
                "fusions": [{"ancien": 1927, "nouveau": 324, "le": "2026-10-05T14:02:37"}]}));
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue").query_param_exists("depuis");
            t.status(200).json_body(json!({"ok": true, "suivante": null, "jeux": [], "ids_visibles": [324], "fusions": []}));
        });
        let (_d, n) = ouvert(&s).await;
        n.synchroniser(|_, _| {}).await.unwrap();
        complet.assert_hits(1);
        // Un profil d'AVANT la 1.14 (marque absente) : une seule page 1 sans `depuis`, puis plus jamais.
        n.session().await.unwrap().verrou().ecrire_meta("fusions_lues", "0").unwrap();
        n.session().await.unwrap().verrou().garder_fusions(&[], true).unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();
        complet.assert_hits(2);
        n.synchroniser(|_, _| {}).await.unwrap();
        complet.assert_hits(2);
        assert_eq!(n.session().await.unwrap().verrou().reuni_vers(1927).unwrap(), Some(324));
    }

    /// RÉEL, profil d'ESSAI temporaire (rien de Seb n'est touché) : un faux jeu du PC sous l'ancien numéro 1927, avec
    /// la vraie version et les vrais fichiers de la fiche 324 ; une vraie synchronisation doit le rattacher à 324.
    /// `FROGTEND_PROFIL_ESSAI=<id> cargo test essai_reunion -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_reunion() {
        use crate::coffre::Coffre as _;
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").unwrap();
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton");
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(d.path(), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Essai", None, Some(&jeton)).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: "https://jeux.hikari-no-sekai.fr".into(), simule: false }).await.unwrap();
        let fiche = n.session().await.unwrap().source.fiche(324).await.unwrap();
        let v = fiche["versions"][0].clone();
        let mut j = jeu_du_pc(1927, &d.path().join("Jeu"));
        j.version = v["telechargement_id"].as_i64().unwrap();
        j.fichiers = v["fichiers"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let nom = f["nom"].as_str().unwrap().to_string();
                let nom = match f["dossier"].as_str().filter(|d| !d.is_empty()) {
                    Some(dd) => format!("{dd}/{nom}"),
                    None => nom,
                };
                crate::jeux_pc::FichierJeu { n: i as u32, nom, taille: f["taille"].as_u64().unwrap() }
            })
            .collect();
        println!("FICHE 324 « {} » : version {}, {} fichier(s)", fiche["titre"], j.version, j.fichiers.len());
        n.registre().ajouter(&j).unwrap();
        let debut = std::time::Instant::now();
        println!("SYNCHRO commence");
        let b = n.synchroniser(|p, j| if p % 10 == 0 { println!("page {p} : {j} jeux") }).await.unwrap();
        println!("SYNCHRO {} jeux en {:?} ; rattachés {:?}", b.jeux, debut.elapsed(), b.rattaches);
        // (Un verrou du registre à la fois : deux dans la même instruction se bloqueraient.)
        let ancien = n.registre().jeu(1927).unwrap().is_some();
        let nouveau = n.registre().jeu(324).unwrap().is_some();
        println!("REGISTRE 1927 : {ancien} ; 324 : {nouveau}");
        let fusions = n.session().await.unwrap().verrou().fusions().unwrap();
        println!("FUSIONS gardées : {}", fusions.len());
        // Un jeu du PC NON rattaché (version 0 : douteux) dont la fiche a été réunie : sa fiche reste la sienne.
        let (x, vers, _) = fusions.iter().find(|(a, _, _)| *a != 1927).cloned().unwrap();
        let mut k = jeu_du_pc(x, &d.path().join("Douteux"));
        k.version = 0;
        n.registre().ajouter(&k).unwrap();
        let m = d.path().join("Douteux").join("Frogtend").join("medias");
        std::fs::create_dir_all(&m).unwrap();
        std::fs::write(m.join("fiche.json"), json!({"id": x, "titre": "douteux"}).to_string()).unwrap();
        let f = n.fiche(x).await.unwrap();
        println!("DOUTEUX {x} (réuni à {vers}) : fiche {} locale {} renvoi {:?}", f.fiche["id"], f.locale, f.reunie_dans);
        println!("RATTACHES au second passage : {:?}", n.appliquer_fusions(&n.session().await.unwrap()).await);
        println!("FIN");
    }

    #[tokio::test]
    async fn un_ancien_catalogue_est_relu_en_entier_une_fois() {
        let serveur = MockServer::start();
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/plateformes");
            t.status(200).json_body(json!({"ok": true, "plateformes": [{"nom": "SNES", "jeux": 1}]}));
        });
        // La relecture complète : SANS `depuis`.
        let complet = serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue").matches(|r| !r.query_params.as_ref().is_some_and(|q| q.iter().any(|(k, _)| k == "depuis")));
            t.status(200).json_body(json!({"ok": true, "suivante": null, "jeux": [
                {"id": 1, "titre": "A", "plateforme": "SNES", "maj_le": "2026-10-01T00:00:00", "images": {"jaquette": {"largeur": 10, "hauteur": 20}}}]}));
        });
        let increment = serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue").query_param_exists("depuis");
            t.status(200).json_body(json!({"ok": true, "suivante": null, "jeux": [], "ids_visibles": [1]}));
        });
        let (_d, n) = noyau();
        let id = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: serveur.base_url(), simule: false }).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();
        complet.assert_hits(1);
        // Ensuite : l'incrément.
        n.synchroniser(|_, _| {}).await.unwrap();
        increment.assert_hits(1);
        complet.assert_hits(1);
        // Un cache d'avant (sans marqueur de schéma) : relu en entier UNE fois, puis l'incrément reprend.
        n.session().await.unwrap().verrou().ecrire_meta("schema_catalogue", "1").unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();
        complet.assert_hits(2);
        n.synchroniser(|_, _| {}).await.unwrap();
        complet.assert_hits(2);
        increment.assert_hits(2);
        let j = n.session().await.unwrap().verrou().jeu(1).unwrap().unwrap();
        assert_eq!(j.images.unwrap()["jaquette"].hauteur, Some(20));
    }

    #[tokio::test]
    async fn le_fond_du_menu_prend_le_premier_visuel_que_firehouse_a_et_le_garde() {
        let serveur = MockServer::start();
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/plateformes");
            t.status(200).json_body(json!({"ok": true, "plateformes": [{"nom": "SNES", "jeux": 1}]}));
        });
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue");
            t.status(200).json_body(json!({"ok": true, "suivante": null, "jeux": [{"id": 1, "titre": "A", "plateforme": "SNES"}]}));
        });
        // Pas de fond pour ce jeu, mais une capture (en 1 000 px au plus, jamais l'original).
        let fond = serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/media/1/fond").query_param("largeur", "1000");
            t.status(404).json_body(json!({"ok": false}));
        });
        let capture = serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/media/1/capture").query_param("largeur", "1000");
            t.status(200).header("Content-Type", "image/webp").body(b"RIFF\0\0\0\0WEBPVP8 ".to_vec());
        });
        let (_d, n) = noyau();
        let id = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: serveur.base_url(), simule: false }).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();

        let img = n.visuel(1, &["fond", "capture"], 4000).await.unwrap().expect("la capture");
        assert!(img.octets.starts_with(b"RIFF"));
        // Une deuxième fois : tout vient du disque (absence du fond et capture retenues).
        assert!(n.visuel(1, &["fond", "capture"], 1000).await.unwrap().is_some());
        fond.assert_hits(1);
        capture.assert_hits(1);
        // Un type inconnu ou un chemin piégé n'est JAMAIS demandé à Firehouse ; un jeu hors de SA ludothèque : rien.
        let autre = serveur.mock(|w, t| {
            w.method(GET).path_contains("/api/jeux/v1/media/");
            t.status(200).body(b"x".to_vec());
        });
        assert!(n.visuel(1, &["../../moi", "boite_imaginaire"], 1000).await.unwrap().is_none());
        assert!(n.visuel(99, &["capture"], 1000).await.unwrap().is_none());
        assert!(n.visuel(-3, &["capture"], 1000).await.unwrap().is_none());
        autre.assert_hits(0);
        capture.assert_hits(1);
        // Sur le disque : seulement ces deux fichiers, dans le dossier du profil.
        let mut fichiers: Vec<String> = std::fs::read_dir(_d.path().join("profils").join(&id).join("visuels"))
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        fichiers.sort();
        assert_eq!(fichiers, ["1-capture-1000.img", "1-fond-1000.absent"]);
    }

    #[tokio::test]
    async fn hors_ligne_le_fond_garde_est_rendu_et_un_autre_profil_ne_le_voit_jamais() {
        let serveur = MockServer::start();
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/plateformes");
            t.status(200).json_body(json!({"ok": true, "plateformes": [{"nom": "SNES", "jeux": 1}]}));
        });
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue");
            t.status(200).json_body(json!({"ok": true, "suivante": null, "jeux": [{"id": 1, "titre": "A", "plateforme": "SNES"}]}));
        });
        // Le fond : en panne (503) ; la capture : servie.
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/media/1/fond");
            t.status(503).json_body(json!({"ok": false}));
        });
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/media/1/capture");
            t.status(200).header("Content-Type", "image/webp").body(b"RIFF\0\0\0\0WEBPVP8 ".to_vec());
        });
        let (d, n) = noyau();
        let seb = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        let lea = n.creer_profil("Léa", None, Some("k")).unwrap().id;
        let co = Connexion { adresse: serveur.base_url(), simule: false };
        n.ouvrir(&seb, None, &co).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();
        // Un 503 sur le fond n'arrête pas la recherche : la capture vient.
        assert!(n.visuel(1, &["fond", "capture"], 1000).await.unwrap().is_some());
        // Vieillie de 8 jours, puis plus de serveur : la capture gardée est rendue quand même.
        let f = d.path().join("profils").join(&seb).join("visuels").join("1-capture-1000.img");
        let vieux = std::time::SystemTime::now() - std::time::Duration::from_secs(8 * 24 * 3600);
        std::fs::File::options().write(true).open(&f).unwrap().set_modified(vieux).unwrap();
        let hors_ligne = Connexion { adresse: "http://127.0.0.1:9".into(), simule: false };
        n.ouvrir(&seb, None, &hors_ligne).await.unwrap();
        assert!(n.visuel(1, &["capture"], 1000).await.unwrap().is_some(), "hors ligne : le visuel gardé");
        // Léa a le même jeu dans SA ludothèque : elle ne lit jamais le fichier de Seb (hors ligne, rien pour elle).
        n.ouvrir(&lea, None, &co).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();
        n.ouvrir(&lea, None, &hors_ligne).await.unwrap();
        assert!(n.visuel(1, &["capture"], 1000).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn un_jeu_redevenu_visible_declenche_une_synchronisation_complete() {
        let serveur = MockServer::start();
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/plateformes");
            t.status(200).json_body(json!({"ok": true, "plateformes": []}));
        });
        // L'incrément annonce l'id 3, visible mais ni changé ni connu : il faut tout relire.
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue").query_param_exists("depuis");
            t.status(200).json_body(json!({"ok": true, "suivante": null, "ids_visibles": [1, 3], "jeux": []}));
        });
        // Première synchronisation : seul le jeu 1 est visible.
        let mut premiere = serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue");
            t.status(200).json_body(json!({"ok": true, "suivante": null, "jeux": [
                {"id": 1, "titre": "A", "plateforme": "PC", "maj_le": "2026-09-29T10:00:00"}]}));
        });
        let (_d, n) = noyau();
        let id = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: serveur.base_url(), simule: false }).await.unwrap();
        assert_eq!(n.synchroniser(|_, _| {}).await.unwrap().jeux, 1);
        premiere.delete();
        // Puis le jeu 3 redevient visible (changement de grade), avec un maj_le ancien.
        let complete = serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue").matches(|r| {
                !r.query_params.as_ref().is_some_and(|q| q.iter().any(|(k, _)| k == "depuis"))
            });
            t.status(200).json_body(json!({"ok": true, "suivante": null, "jeux": [
                {"id": 1, "titre": "A", "plateforme": "PC", "maj_le": "2026-09-29T10:00:00"},
                {"id": 3, "titre": "C", "plateforme": "PC", "maj_le": "2020-01-01T00:00:00"}]}));
        });
        let b = n.synchroniser(|_, _| {}).await.unwrap();
        assert_eq!((b.mode, b.jeux), ("complete", 2));
        complete.assert_hits(1);
    }

    #[tokio::test]
    async fn une_jaquette_changee_est_retelechargee_et_l_ancienne_effacee() {
        let serveur = MockServer::start();
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/plateformes");
            t.status(200).json_body(json!({"ok": true, "plateformes": []}));
        });
        let mut cat = serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue");
            t.status(200).json_body(json!({"ok": true, "suivante": null, "jeux": [
                {"id": 1, "titre": "A", "plateforme": "PC", "jaquette": true, "jaquette_empreinte": "v1", "maj_le": "t1"}]}));
        });
        let media = serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/media/1/jaquette").query_param("largeur", "400");
            t.status(200).body([0xFFu8, 0xD8, 0xFF, 1]);
        });
        let (d, n) = noyau();
        let id = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: serveur.base_url(), simule: false }).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();
        // 350 est arrondi à la centaine supérieure : 400.
        n.jaquette(1, Some(350)).await.unwrap().unwrap();
        n.jaquette(1, Some(400)).await.unwrap().unwrap();
        media.assert_hits(1);

        // La jaquette change dans Firehouse : nouvelle empreinte (reçue par une synchronisation complète ici).
        cat.delete();
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/catalogue");
            t.status(200).json_body(json!({"ok": true, "suivante": null, "ids_visibles": [1], "jeux": [
                {"id": 1, "titre": "A", "plateforme": "PC", "jaquette": true, "jaquette_empreinte": "v2", "maj_le": "t2"}]}));
        });
        n.synchroniser(|_, _| {}).await.unwrap();
        n.jaquette(1, Some(400)).await.unwrap().unwrap();
        media.assert_hits(2);
        let fichiers: Vec<String> = std::fs::read_dir(d.path().join("profils").join(&id).join("jaquettes"))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(fichiers, vec!["1-400-v2.img".to_string()]);
    }

    #[tokio::test]
    async fn les_skins_se_lisent_sans_jeton_et_la_video_se_joint_a_l_origine() {
        let serveur = MockServer::start();
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/themes");
            t.status(200).header("etag", "W/\"s1\"").json_body(json!({"ok": true, "version": "s1", "themes": {
                "firehouse": {"base": {}, "resolus": {}, "video_api": "/api/jeux/v1/skin/firehouse/video"}}}));
        });
        let video = serveur.mock(|w, t| {
            // Le chemin est joint à l'ORIGINE : jamais /api/jeux/v1/api/jeux/v1/...
            w.method(GET).path("/api/jeux/v1/skin/firehouse/video");
            t.status(200).body(b"webm");
        });
        let (_d, n) = noyau();
        let c = Connexion { adresse: serveur.base_url(), simule: false };
        // Aucun profil ouvert : l'écran « Qui joue ? » a déjà ses skins.
        let s = n.skins(&c).await.unwrap().unwrap();
        assert_eq!(s["version"], "s1");
        assert_eq!(n.video_skin(&c, "firehouse").await.unwrap().unwrap(), b"webm");
        n.video_skin(&c, "firehouse").await.unwrap().unwrap();
        video.assert_hits(1); // gardée sur le disque
        assert_eq!(n.skins(&SIMULE).await.unwrap(), None);
    }

    #[tokio::test]
    async fn un_jeton_se_verifie_par_moi_avant_d_etre_range() {
        let serveur = MockServer::start();
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/moi").header("authorization", "Bearer bon");
            t.status(200).json_body(json!({"ok": true, "nom": "Seb", "grade": "admin"}));
        });
        serveur.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/moi");
            t.status(401).json_body(json!({"detail": "connexion requise"}));
        });
        let c = Connexion { adresse: serveur.base_url(), simule: false };
        assert_eq!(Noyau::verifier_jeton(&c, " bon ").await.unwrap()["grade"], "admin");
        assert!(matches!(Noyau::verifier_jeton(&c, "mauvais").await, Err(Erreur::JetonRefuse(_))));
    }

    /// Essai des MINIATURES sur le vrai Firehouse (lecture seule, jeton du profil lu dans le coffre).
    /// `FROGTEND_PROFIL_ESSAI=<id> cargo test essai_miniatures -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_installation_cheatengine() {
        // Accord de Seb (02/10). Télécharge le paquet Cheat Engine servi par Firehouse dans un dossier TEMPORAIRE,
        // vérifie son empreinte, l'installe comme Frogtend le ferait, trouve le programme, puis tout est effacé.
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton pour ce profil");
        let s = crate::source::Source::Firehouse(Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap());
        let p = s.paquet_emulateur("cheatengine").await.unwrap();
        let d = tempfile::tempdir().unwrap();
        let paquet = d.path().join("ce.paquet");
        let route = p["url"].as_str().unwrap().strip_prefix(crate::firehouse::PREFIXE).unwrap().to_string();
        let debut = std::time::Instant::now();
        let n = s.telecharger_route(&route, &paquet, &|_, _| {}).await.unwrap();
        println!("reçu {n} octets en {} s (annoncé {})", debut.elapsed().as_secs(), p["taille"]);
        let sha = crate::sauvegarde::empreinte(&paquet).unwrap();
        println!("empreinte {} (attendue {})", sha, p["sha256"]);
        assert_eq!(Some(sha.as_str()), p["sha256"].as_str());
        let r = crate::emulateurs::installer_paquet_decrit("Cheat Engine", &paquet, &d.path().join("Cheat Engine"), p["programme"].as_str(), None).unwrap();
        println!("programme : {r:?}");
        let ce = d.path().join("Cheat Engine");
        for f in ["autorun/SetMemoryRegion.lua", "autorun/FontSizeAllForms.lua", "LISEZMOI-FIREHOUSE.txt"] {
            println!("{f} : {}", if ce.join(f).is_file() { "présent" } else { "ABSENT" });
        }
    }

    #[tokio::test]
    #[ignore]
    async fn essai_contrat_13_14_sur_firehouse() {
        // LECTURE SEULE : les routes des besoins 13 et 14 (Firehouse 2.26.0). Rien n'est téléchargé ni installé.
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton pour ce profil");
        let s = crate::source::Source::Firehouse(Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap());
        let e = s.emulateurs("Nintendo Switch").await.unwrap();
        for x in e["emulateurs"].as_array().cloned().unwrap_or_default() {
            println!("Switch : {} id={} programme={} portable={} telechargement={}", x["nom"], x["id"], x["programme"], x["portable"], x["telechargement"]);
        }
        println!("Switch autres : {}", e["autres"].to_string().chars().take(300).collect::<String>());
        if let crate::source::Source::Firehouse(c) = &s {
            for id in ["ryubing", "ef_ryu", "citron_neo"] {
                let b = c.brute(reqwest::Method::GET, &format!("/emulateurs/{id}/paquet"), None, &[]).await.unwrap();
                println!("brut {id} : {} {}", b.statut, String::from_utf8_lossy(&b.octets).chars().take(400).collect::<String>());
            }
        }
        for id in ["eden", "citron_neo", "ryubing", "xenia", "cemu", "cheatengine"] {
            match s.paquet_emulateur(id).await {
                Ok(p) => println!("paquet {id} : {} {} {} octets programme={} sha={}", p["version"], p["nom_fichier"], p["taille"], p["programme"], p["sha256"]),
                Err(err) => println!("paquet {id} : {err:?}"),
            }
        }
        for jeu in [110, 111] {
            let t = s.triches(jeu).await.unwrap();
            println!("triches {jeu} : {} code(s), {} mod(s), note={}, page_mods={}", t["codes"].as_array().map_or(0, |l| l.len()), t["mods"].as_array().map_or(0, |l| l.len()), t["note"], t["page_mods"]);
            for c in t["codes"].as_array().cloned().unwrap_or_default().iter().take(4) {
                println!("   {} | {} | {} | {} | dossier={} base={} confiance={} corresp={}", c["cle"], c["emulateur"], c["titre"], c["nom_fichier"], c["dossier"], c["base"], c["confiance"], c["correspondance"]);
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn essai_assistant_sur_firehouse() {
        // UNE question, sur Dune : la forme réelle de la réponse (lot 7). Rien n'est exécuté.
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton pour ce profil");
        let s = crate::source::Source::Firehouse(Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap());
        let debut = std::time::Instant::now();
        match s.assistant("Comment je lance ce jeu ?", Some(110), &[]).await {
            Ok(v) => {
                println!("en {} s", debut.elapsed().as_secs());
                println!("texte : {}", v["texte"].as_str().unwrap_or("").chars().take(600).collect::<String>());
                println!("actions : {}", v["actions_proposees"]);
                println!("clés : {:?}", v.as_object().map(|o| o.keys().collect::<Vec<_>>()));
            }
            Err(e) => println!("ERREUR {e:?} en {} s", debut.elapsed().as_secs()),
        }
    }

    #[tokio::test]
    #[ignore]
    async fn essai_recherche_sur_firehouse() {
        // LECTURE SEULE : la forme de GET /recherche (lot 6). Aucune demande n'est envoyée.
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton pour ce profil");
        let c = Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap();
        for texte in ["dune", "zelda ocarina", "xyzzyqq"] {
            let b = c.brute(reqwest::Method::GET, &format!("/recherche?texte={}", crate::source::encoder(texte)), None, &[]).await.unwrap();
            let t = String::from_utf8_lossy(&b.octets);
            println!("« {texte} » → {} : {}", b.statut, &t[..t.len().min(1500)]);
        }
    }

    #[tokio::test]
    #[ignore]
    async fn essai_plateformes_et_emulateurs_sur_firehouse() {
        // LECTURE SEULE : les plateformes du compte, et l'émulateur recommandé par Firehouse pour chacune.
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton pour ce profil");
        let c = Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap();
        let s = crate::source::Source::Firehouse(c);
        for nom in std::env::var("FROGTEND_PLATEFORMES_ESSAI").unwrap_or_default().split(';').filter(|n| !n.is_empty()) {
            match s.emulateurs(nom).await {
                Ok(v) => {
                    let l: Vec<String> = v["emulateurs"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default()
                        .iter()
                        .map(|e| format!("{}{}", e["nom"].as_str().unwrap_or("?"), if e["recommande"] == true { " (recommandé)" } else { "" }))
                        .collect();
                    println!("{nom:<30} → {}", l.join(", "));
                }
                Err(e) => println!("{nom:<30} → {e:?}"),
            }
        }
        let p = s.plateformes().await.unwrap();
        for pl in p["plateformes"].as_array().or(p.as_array()).cloned().unwrap_or_default() {
            let nom = pl["nom"].as_str().unwrap_or("?").to_string();
            let e = s.emulateurs(&nom).await.map(|v| {
                v["emulateurs"].as_array().cloned().unwrap_or_default().iter().take(3).map(|e| e["nom"].as_str().unwrap_or("?").to_string()).collect::<Vec<_>>().join(", ")
            });
            println!("{nom:<35} {:>5} jeu(x) — {}", pl["jeux"], e.unwrap_or_else(|e| format!("{e:?}")));
        }
    }

    #[tokio::test]
    #[ignore]
    async fn essai_miniatures_sur_firehouse() {
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton pour ce profil");
        let c = Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap();
        for id in [110, 111] {
            for largeur in ["", "?largeur=100", "?largeur=200", "?largeur=300", "?largeur=400", "?largeur=500", "?largeur=1000"] {
                let debut = std::time::Instant::now();
                match c.obtenir(&format!("/media/{id}/jaquette{largeur}"), None).await {
                    Ok(Reponse::Corps { octets, type_contenu, .. }) => println!(
                        "jeu {id} {largeur:<14} → {} octets, {:?}, {} ms",
                        octets.len(),
                        type_contenu,
                        debut.elapsed().as_millis()
                    ),
                    Ok(Reponse::NonModifie) => println!("jeu {id} {largeur:<14} → 304"),
                    Err(e) => println!("jeu {id} {largeur:<14} → ERREUR {e:?} ({} ms)", debut.elapsed().as_millis()),
                }
            }
        }
    }

    /// Essai des routes PUBLIQUES du vrai Firehouse (sans jeton), jamais lancé par la suite (`#[ignore]`).
    /// `cargo test essai_public -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_public_sur_firehouse() {
        let adresse =
            std::env::var("FROGTEND_ADRESSE_ESSAI").unwrap_or_else(|_| "https://jeux.hikari-no-sekai.fr".into());
        let (_d, n) = noyau();
        let c = Connexion { adresse, simule: false };
        let s = n.skins(&c).await.unwrap().expect("skins absents");
        let themes = s["themes"].as_object().unwrap();
        println!("SKINS sans jeton : {} (version {})", themes.len(), s["version"]);
        let avec_video: Vec<_> = themes.iter().filter(|(_, t)| t["video_api"].is_string()).map(|(k, t)| (k, &t["video_api"])).collect();
        println!("SKINS avec vidéo : {avec_video:?}");
        let v = n.video_skin(&c, "firehouse").await.unwrap().expect("vidéo absente");
        println!("VIDÉO firehouse : {} octets, début {:02X?}", v.len(), &v[..4]);
        assert_eq!(&v[..4], &[0x1A, 0x45, 0xDF, 0xA3], "un webm commence par l'en-tête EBML");
        let s2 = n.skins(&c).await.unwrap().unwrap();
        assert_eq!(s["version"], s2["version"]);
    }

    /// Essai sur le VRAI Firehouse, jamais lancé par la suite de tests (`#[ignore]`).
    /// Lecture seule. Jeton lu dans FROGTEND_JETON_ESSAI (jamais affiché), dossier temporaire effacé à la fin.
    /// `cargo test essai_reel -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_reel_sur_firehouse() {
        // Le jeton : FROGTEND_JETON_ESSAI, sinon celui du profil FROGTEND_PROFIL_ESSAI de l'application installée,
        // lu dans le coffre de Windows (ainsi, aucun jeton ne passe par une commande ni par un journal).
        let jeton = std::env::var("FROGTEND_JETON_ESSAI").unwrap_or_else(|_| {
            let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_JETON_ESSAI ou FROGTEND_PROFIL_ESSAI");
            crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton pour ce profil")
        });
        let adresse =
            std::env::var("FROGTEND_ADRESSE_ESSAI").unwrap_or_else(|_| "https://jeux.hikari-no-sekai.fr".into());
        let (_d, n) = noyau();
        let id = n.creer_profil("Essai", None, Some(&jeton)).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse, simule: false }).await.unwrap();

        let b = n.synchroniser(|p, j| println!("  page {p} : {j} jeux")).await.unwrap();
        println!("SYNCHRO : {} jeux, {} plateformes, {} page(s)", b.jeux, b.plateformes, b.pages);
        for p in n.plateformes(false).await.unwrap() {
            println!("  plateforme « {} » : {} jeu(x)", p.nom, p.jeux);
        }
        let liste = n.lister(&Filtre::default()).await.unwrap();
        for j in &liste.jeux {
            println!("  jeu {} « {} » ({:?}) jaquette={:?} versions={:?}", j.id, j.titre, j.annee, j.jaquette, j.versions);
        }
        if let Some(j) = liste.jeux.first() {
            let f = n.fiche(j.id).await.unwrap();
            println!(
                "FICHE {} : « {} », {} version(s), {} annexe(s), résumé de {} caractères",
                j.id,
                f.fiche["titre"].as_str().unwrap_or("?"),
                f.fiche["versions"].as_array().map_or(0, Vec::len),
                f.fiche["annexes"].as_array().map_or(0, Vec::len),
                f.fiche["resume"].as_str().map_or(0, |r| r.chars().count())
            );
            if let Some(a) = f.fiche["annexes"].as_array().and_then(|l| l.iter().find(|a| a["texte"] == true)) {
                let t = n.annexe_texte(j.id, a["i"].as_u64().unwrap() as u32, a["cle"].as_str().unwrap()).await.unwrap();
                println!("ANNEXE « {} » : {} caractères", a["titre"], t["texte"].as_str().map_or(0, |x| x.len()));
            }
            match n.jaquette(j.id, None).await.unwrap() {
                Some(img) => println!("JAQUETTE {} : {:?}, {} octets", j.id, img.type_contenu, img.octets.len()),
                None => println!("JAQUETTE {} : aucune", j.id),
            }
        }
        let c = Connexion { adresse: std::env::var("FROGTEND_ADRESSE_ESSAI").unwrap_or_else(|_| "https://jeux.hikari-no-sekai.fr".into()), simule: false };
        let skins = n.skins(&c).await.unwrap().expect("skins absents");
        println!("SKINS : {} (version {})", skins["themes"].as_object().map_or(0, |t| t.len()), skins["version"]);
        let deuxieme = n.skins(&c).await.unwrap().expect("skins absents au second appel");
        println!("COMPTE : {}", n.compte().await.unwrap());
        match n.video_skin(&c, "firehouse").await.unwrap() {
            Some(v) => println!("VIDÉO firehouse : {} octets", v.len()),
            None => println!("VIDÉO firehouse : aucune"),
        }
        if let Some(j) = liste.jeux.first() {
            match n.jaquette(j.id, Some(400)).await.unwrap() {
                Some(img) => println!("MINIATURE 400 : {:?}, {} octets", img.type_contenu, img.octets.len()),
                None => println!("MINIATURE 400 : aucune"),
            }
        }
        let b2 = n.synchroniser(|_, _| {}).await.unwrap();
        println!("2e SYNCHRO : mode {}, {} reçu(s), {} retiré(s), {} jeux", b2.mode, b2.recus, b2.retires, b2.jeux);
        assert_eq!(skins, deuxieme, "le cache ETag doit rendre les mêmes skins");
        println!("SKIN PERSONNEL : {:?}", n.skin_personnel().await.unwrap());
    }

    #[test]
    fn le_journal_ecrit_une_ligne_et_se_met_de_cote_quand_il_est_gros() {
        let (d, n) = noyau();
        n.journaliser("jaquette 110 : essai");
        let texte = std::fs::read_to_string(d.path().join("journal.txt")).unwrap();
        assert!(texte.trim_end().ends_with("jaquette 110 : essai"));
        std::fs::write(d.path().join("journal.txt"), vec![b'x'; (TAILLE_MAX_JOURNAL + 1) as usize]).unwrap();
        n.journaliser("suite");
        assert!(d.path().join("journal.1.txt").is_file());
        assert!(std::fs::read_to_string(d.path().join("journal.txt")).unwrap().contains("suite"));
    }

    #[test]
    fn le_cache_reste_utilisable_apres_une_erreur_en_plein_travail() {
        let s = Session {
            profil: ProfilVisible { id: "x".into(), nom: "X".into(), protege: false },
            dossier: std::env::temp_dir(),
            source: Source::Simulee,
            cache: std::sync::Mutex::new(Cache::en_memoire().unwrap()),
        };
        let s = std::sync::Arc::new(s);
        let s2 = s.clone();
        // Un travail qui échoue brutalement en tenant le verrou…
        let _ = std::thread::spawn(move || {
            let _garde = s2.verrou();
            panic!("échec simulé");
        })
        .join();
        // … n'empêche pas la suite de lire le cache.
        assert_eq!(s.verrou().nombre_de_jeux().unwrap(), 0);
    }

    #[test]
    fn reconnait_les_types_d_image() {
        assert_eq!(type_image(&[0xFF, 0xD8, 0xFF, 0]).as_deref(), Some("image/jpeg"));
        assert_eq!(type_image(b"RIFF\0\0\0\0WEBPVP8").as_deref(), Some("image/webp"));
        assert_eq!(type_image(b"xx"), None);
    }
}
