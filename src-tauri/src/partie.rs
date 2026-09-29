//! Installer un jeu du PC, choisir ce qu'on lance, jouer (et le dire à Firehouse), mettre les parties à l'abri,
//! retirer un jeu du PC.
//!
//! Règles : rien ne s'installe sans l'accord de la personne (l'appelant le demande) ; les parties sont mises à l'abri
//! AVANT tout retrait ; on n'efface que ce que Frogtend a mis sur le disque, et les abris ne s'effacent jamais.

use crate::erreurs::{Erreur, Resultat};
use crate::installation::{
    arguments_silencieux, candidats, changes_depuis, copier, decompresser, executer_et_attendre, lanceur_emulateur,
    methode, nature, relever, Candidat, Lanceur, Manifeste, Methode, Nature,
};
use crate::jeux_pc::{Etat, Installation, JeuPc};
use crate::lancement::{demarrer, Suivi};
use crate::noyau::{maintenant, Noyau};
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Nom du dossier d'installation, DANS le dossier du jeu (à côté des fichiers reçus).
pub const DOSSIER_INSTALLATION: &str = "Jeu";
/// Toutes les combien Firehouse doit entendre « vivant » pendant une partie (il oublie la session après 30 min).
const INTERVALLE_VIVANT: Duration = Duration::from_secs(10 * 60);
/// Toutes les combien on regarde si le jeu tourne encore.
const INTERVALLE_SUIVI: Duration = Duration::from_secs(2);

/// Ce que l'interface montre avant d'installer.
#[derive(Debug, Clone, Serialize)]
pub struct Preparation {
    pub nature: Nature,
    pub methode: Methode,
    /// Où le jeu sera installé.
    pub destination: String,
    /// Les notes de la version (à lire avant d'installer : elles ne valent que pour elle).
    pub notes: Option<String>,
    pub titre: String,
}

/// Un abri de parties : où, combien.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Abri {
    pub dossier: String,
    pub fichiers: usize,
    pub octets: u64,
}

/// La fin d'une partie.
#[derive(Debug, Clone, Serialize)]
pub struct FinDePartie {
    pub jeu: i64,
    pub secondes: u64,
}

/// Le fichier donné à l'émulateur : pour un jeu en plusieurs fichiers, celui qui les décrit (.cue, .m3u…).
fn fichier_principal(fichiers: &[String]) -> Option<String> {
    for ext in [".m3u", ".cue", ".gdi", ".chd", ".iso"] {
        if let Some(f) = fichiers.iter().find(|f| f.to_lowercase().ends_with(ext)) {
            return Some(f.clone());
        }
    }
    fichiers.first().cloned()
}

impl Noyau {
    /// Un jeu du PC, visible par le profil ouvert.
    async fn jeu_visible(&self, id: i64) -> Resultat<JeuPc> {
        let s = self.session().await?;
        let j = self.registre().jeu(id)?.ok_or_else(|| Erreur::Introuvable("Ce jeu n'est pas sur ce PC.".into()))?;
        if !s.verrou().contient(id)? {
            return Err(Erreur::Introuvable("Ce jeu n'est pas sur ce PC.".into()));
        }
        Ok(j)
    }

    fn fichier_manifeste(&self, id: i64) -> PathBuf {
        self.dossier_medias(id).join("manifeste.json")
    }

    /// Ce qui va se passer si on installe ce jeu : sa nature, la méthode, la destination, les notes de SA version.
    pub async fn preparer_installation(&self, id: i64) -> Resultat<Preparation> {
        let j = self.jeu_visible(id).await?;
        if j.etat != Etat::Telecharge {
            return Err(Erreur::Refus("Le jeu n'est pas encore entièrement téléchargé.".into()));
        }
        let fiche = self.fiche_locale(id).unwrap_or(Value::Null);
        let version = fiche["versions"]
            .as_array()
            .and_then(|l| l.iter().find(|v| v["telechargement_id"].as_i64() == Some(j.version)))
            .cloned()
            .unwrap_or(Value::Null);
        let n = nature(version["qualite"].as_str().unwrap_or(""));
        let noms: Vec<String> = j.fichiers.iter().map(|f| f.nom.clone()).collect();
        let m = methode(n, Path::new(&j.dossier), &noms)?;
        let destination = match m {
            Methode::Aucune => j.dossier.clone(),
            _ => Path::new(&j.dossier).join(DOSSIER_INSTALLATION).to_string_lossy().into(),
        };
        Ok(Preparation {
            nature: n,
            methode: m,
            destination,
            notes: version["notes"].as_str().filter(|t| !t.trim().is_empty()).map(String::from),
            titre: j.titre,
        })
    }

    /// Installe le jeu (après l'accord de la personne). `automatique` : installeur silencieux si possible.
    /// Rend l'installation ; son lanceur reste à choisir (sauf ROM et image disque : l'émulateur s'en charge).
    pub async fn installer(&self, id: i64, automatique: bool) -> Resultat<Installation> {
        let j = self.jeu_visible(id).await?;
        let p = self.preparer_installation(id).await?;
        let recus = PathBuf::from(&j.dossier);
        let dest = PathBuf::from(&p.destination);
        let journal = self.dossier_medias(id).join("installation.log");
        std::fs::create_dir_all(self.dossier_medias(id))?;

        let mut fichier_du_jeu = None;
        match &p.methode {
            Methode::Installeur { fichier, format } => {
                std::fs::create_dir_all(&dest)?;
                let exe = recus.join(fichier);
                let args = if automatique { arguments_silencieux(*format, &dest, &journal) } else { vec![] };
                let dossier = recus.clone();
                let code = tokio::task::spawn_blocking(move || executer_et_attendre(&exe, &args, &dossier))
                    .await
                    .map_err(|_| Erreur::Disque("L'installeur s'est arrêté brutalement.".into()))??;
                if code != 0 {
                    self.journaliser(&format!("installation du jeu {id} : code {code}"));
                    return Err(Erreur::Refus(format!(
                        "L'installeur s'est terminé en erreur (code {code}). Le détail est dans {}.",
                        journal.display()
                    )));
                }
            }
            Methode::InstalleurGuide { fichier } => {
                std::fs::create_dir_all(&dest)?;
                let exe = recus.join(fichier);
                let dossier = recus.clone();
                tokio::task::spawn_blocking(move || executer_et_attendre(&exe, &[], &dossier))
                    .await
                    .map_err(|_| Erreur::Disque("L'installeur s'est arrêté brutalement.".into()))??;
            }
            Methode::Archive { fichier, format } => {
                let archive = recus.join(fichier);
                let (d, f) = (dest.clone(), *format);
                tokio::task::spawn_blocking(move || decompresser(&archive, f, &d))
                    .await
                    .map_err(|_| Erreur::Disque("La décompression s'est arrêtée brutalement.".into()))??;
            }
            Methode::Aucune => {
                if matches!(p.nature, Nature::Rom | Nature::ImageDisque) {
                    fichier_du_jeu = fichier_principal(&j.fichiers.iter().map(|f| f.nom.clone()).collect::<Vec<_>>());
                }
            }
        }

        // Vérifier le résultat : l'installation a-t-elle mis quelque chose dans la destination ?
        let vide = std::fs::read_dir(&dest).map(|mut l| l.next().is_none()).unwrap_or(true);
        if vide {
            return Err(Erreur::Refus(format!(
                "L'installation est finie, mais rien n'est dans {}. Si l'installeur a mis le jeu ailleurs, indique ce dossier.",
                dest.display()
            )));
        }
        self.retenir_installation(&j, &dest, p.methode, fichier_du_jeu, p.nature)
    }

    /// Le jeu a été installé ailleurs que prévu (installeur guidé) : la personne indique où.
    pub async fn installe_ailleurs(&self, id: i64, dossier: &str) -> Resultat<Installation> {
        let j = self.jeu_visible(id).await?;
        let p = self.preparer_installation(id).await?;
        let d = PathBuf::from(dossier);
        if !d.is_dir() {
            return Err(Erreur::Disque(format!("Dossier introuvable : {dossier}.")));
        }
        self.retenir_installation(&j, &d, p.methode, None, p.nature)
    }

    fn retenir_installation(
        &self,
        j: &JeuPc,
        dest: &Path,
        methode: Methode,
        fichier_du_jeu: Option<String>,
        nature: Nature,
    ) -> Resultat<Installation> {
        // Le relevé des fichiers : ce qui changera ensuite (les parties) sera reconnu. Pas pour une ROM : ses parties
        // sont chez l'émulateur.
        if !matches!(nature, Nature::Rom | Nature::ImageDisque) {
            let m = relever(dest)?;
            std::fs::write(self.fichier_manifeste(j.id), serde_json::to_vec(&m).unwrap())?;
        }
        let i = Installation {
            dossier: dest.to_string_lossy().into(),
            methode,
            lanceur: None,
            fichier_du_jeu,
            installe_le: maintenant(),
        };
        self.registre().changer_installation(j.id, Some(&i))?;
        Ok(i)
    }

    /// Les programmes qui pourraient lancer le jeu installé, les plus probables d'abord.
    pub async fn candidats_lancement(&self, id: i64) -> Resultat<Vec<Candidat>> {
        let j = self.jeu_visible(id).await?;
        let i = j.installation.ok_or_else(|| Erreur::Refus("Le jeu n'est pas encore installé.".into()))?;
        candidats(Path::new(&i.dossier), &j.titre)
    }

    /// Retient ce qu'on lance pour jouer (choisi par la personne).
    pub async fn choisir_lanceur(&self, id: i64, lanceur: Lanceur) -> Resultat<()> {
        let j = self.jeu_visible(id).await?;
        let mut i = j.installation.ok_or_else(|| Erreur::Refus("Le jeu n'est pas encore installé.".into()))?;
        let systeme = ["cmd.exe", "explorer.exe"].iter().any(|p| lanceur.programme.eq_ignore_ascii_case(p));
        if !systeme && !Path::new(&lanceur.programme).is_file() {
            return Err(Erreur::Disque(format!("Programme introuvable : {}.", lanceur.programme)));
        }
        i.lanceur = Some(lanceur);
        self.registre().changer_installation(id, Some(&i))
    }

    /// Lance le jeu. `emulateur` : (programme, ligne de commande) pour une ROM ou une image disque.
    /// Annonce le début de la session à Firehouse. Rend le processus et les dossiers à surveiller.
    pub async fn jouer(&self, id: i64, emulateur: Option<(String, String)>) -> Resultat<(u32, Vec<PathBuf>)> {
        let j = self.jeu_visible(id).await?;
        let i = j.installation.clone().ok_or_else(|| Erreur::Refus("Installe d'abord le jeu.".into()))?;
        let (lanceur, dossiers) = match (&i.lanceur, &i.fichier_du_jeu) {
            (Some(l), _) => (l.clone(), vec![PathBuf::from(&i.dossier)]),
            (None, Some(f)) => {
                let (programme, ligne) = emulateur.ok_or_else(|| {
                    Erreur::Reglage(format!(
                        "Aucun émulateur n'est réglé pour {}. Règle-le dans « Réglages » ▸ « Émulateurs ».",
                        j.plateforme
                    ))
                })?;
                let l = lanceur_emulateur(&programme, &ligne, &Path::new(&i.dossier).join(f));
                let d = PathBuf::from(&l.dossier);
                (l, vec![d])
            }
            (None, None) => return Err(Erreur::Refus("Choisis d'abord ce qui lance le jeu.".into())),
        };
        let pid = demarrer(&lanceur)?;
        self.annoncer(id, "debut").await;
        Ok((pid, dossiers))
    }

    /// Annonce une étape de session à Firehouse. Un échec (hors ligne…) n'empêche jamais de jouer : il est noté.
    async fn annoncer(&self, id: i64, etat: &str) {
        if let Ok(s) = self.session().await {
            if let Err(e) = s.source.session(etat, id).await {
                self.journaliser(&format!("session « {etat} » du jeu {id} : {e:?}"));
            }
        }
    }

    /// Suit une partie jusqu'à sa fin : « vivant » toutes les 10 min, « fin » à la sortie, temps de jeu compté.
    pub async fn suivre_partie(&self, id: i64, pid: u32, dossiers: Vec<PathBuf>) -> Resultat<FinDePartie> {
        let debut = Instant::now();
        let mut dernier_vivant = Instant::now();
        let mut suivi = Suivi::nouveau(pid, dossiers);
        loop {
            let (s, en_cours) = tokio::task::spawn_blocking(move || {
                let r = suivi.en_cours();
                (suivi, r)
            })
            .await
            .map_err(|_| Erreur::Disque("Le suivi de la partie s'est arrêté.".into()))?;
            suivi = s;
            if !en_cours {
                break;
            }
            if dernier_vivant.elapsed() >= INTERVALLE_VIVANT {
                self.annoncer(id, "vivant").await;
                dernier_vivant = Instant::now();
            }
            tokio::time::sleep(INTERVALLE_SUIVI).await;
        }
        self.annoncer(id, "fin").await;
        let secondes = debut.elapsed().as_secs();
        self.registre().compter_partie(id, secondes, &maintenant())?;
        Ok(FinDePartie { jeu: id, secondes })
    }

    /// Met à l'abri ce qui a changé dans le dossier du jeu depuis son installation (les parties, ses réglages) :
    /// une copie dans `sauvegardes/<jeu>/<date>/`. Les abris ne s'effacent jamais tout seuls.
    pub async fn mettre_a_l_abri(&self, id: i64) -> Resultat<Abri> {
        let j = self.jeu_visible(id).await?;
        let i = j.installation.ok_or_else(|| Erreur::Refus("Le jeu n'est pas installé : rien à mettre à l'abri.".into()))?;
        let m: Manifeste = match std::fs::read(self.fichier_manifeste(id)) {
            Ok(o) => serde_json::from_slice(&o).unwrap_or_default(),
            Err(_) => return Ok(Abri { dossier: String::new(), fichiers: 0, octets: 0 }),
        };
        let racine = PathBuf::from(&i.dossier);
        if !racine.is_dir() {
            return Err(Erreur::Disque(format!("Le dossier du jeu est introuvable : {}.", racine.display())));
        }
        let changes = changes_depuis(&racine, &m)?;
        if changes.is_empty() {
            return Ok(Abri { dossier: String::new(), fichiers: 0, octets: 0 });
        }
        let abri = self.dossier.join("sauvegardes").join(id.to_string()).join(maintenant());
        let octets = copier(&racine, &changes, &abri)?;
        // Vérifier le résultat : chaque fichier est bien dans l'abri, à la même taille.
        for r in &changes {
            let a = std::fs::metadata(abri.join(r)).map(|m| m.len()).ok();
            let b = std::fs::metadata(racine.join(r)).map(|m| m.len()).ok();
            if a != b {
                return Err(Erreur::Disque(format!("La copie de « {r} » a échoué : le jeu n'est pas retiré.")));
            }
        }
        Ok(Abri { dossier: abri.to_string_lossy().into(), fichiers: changes.len(), octets })
    }

    /// Retire un jeu complet du PC : ses parties d'abord à l'abri (sinon rien n'est fait), puis son désinstalleur
    /// s'il en a un, puis ce que Frogtend a mis sur le disque (installation, fichiers reçus, médias). Les abris
    /// restent.
    pub async fn retirer_du_pc(&self, id: i64) -> Resultat<Abri> {
        let j = self.jeu_visible(id).await?;
        if !matches!(j.etat, Etat::Telecharge) {
            return Err(Erreur::Refus("Ce jeu est en cours de téléchargement : annule plutôt le téléchargement.".into()));
        }
        let abri = if j.installation.is_some() {
            self.mettre_a_l_abri(id).await?
        } else {
            Abri { dossier: String::new(), fichiers: 0, octets: 0 }
        };
        let recus = PathBuf::from(&j.dossier);
        if let Some(i) = &j.installation {
            let installe = PathBuf::from(&i.dossier);
            // Le désinstalleur d'Inno Setup, s'il y en a un (il nettoie ce que l'installeur a mis ailleurs).
            let desinstalleur = installe.join("unins000.exe");
            if desinstalleur.is_file() {
                let args = vec!["/VERYSILENT".into(), "/SUPPRESSMSGBOXES".into(), "/NORESTART".into()];
                let d = installe.clone();
                let _ = tokio::task::spawn_blocking(move || executer_et_attendre(&desinstalleur, &args, &d)).await;
            }
            // On n'efface un dossier d'installation que s'il est DANS le dossier du jeu créé par Frogtend.
            if installe != recus && installe.starts_with(&recus) && installe.is_dir() {
                std::fs::remove_dir_all(&installe)?;
            }
        }
        for f in &j.fichiers {
            let p = recus.join(crate::jeux_pc::nom_de_fichier_sur(&f.nom)?);
            if p.is_file() {
                std::fs::remove_file(&p)?;
            }
        }
        let _ = std::fs::remove_dir(&recus); // seulement s'il est vide
        let medias = self.dossier_medias(id);
        if medias.starts_with(self.dossier.join("medias")) && medias.is_dir() {
            std::fs::remove_dir_all(&medias)?;
        }
        self.registre().retirer(id)?;
        Ok(abri)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coffre::CoffreMemoire;
    use crate::jeux_pc::{Emplacements, FichierJeu};
    use crate::noyau::Connexion;

    /// Un noyau avec un profil (simulé) qui voit Dune (110), et Dune « téléchargé » sur le PC avec les fichiers donnés.
    async fn noyau_avec(fichiers: &[(&str, &[u8])], qualite: &str) -> (tempfile::TempDir, Noyau) {
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, None).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();

        let dossier = d.path().join("Jeux").join("MS-DOS").join("Dune (1992)");
        std::fs::create_dir_all(&dossier).unwrap();
        let mut fj = Vec::new();
        for (i, (nom, o)) in fichiers.iter().enumerate() {
            std::fs::write(dossier.join(nom), o).unwrap();
            fj.push(FichierJeu { n: i as u32, nom: nom.to_string(), taille: o.len() as u64 });
        }
        let total = fj.iter().map(|f| f.taille).sum();
        n.registre()
            .ajouter(&JeuPc {
                id: 110,
                version: 200,
                titre: "Dune".into(),
                plateforme: "MS-DOS".into(),
                dossier: dossier.to_string_lossy().into(),
                etat: Etat::Telecharge,
                total,
                fichiers: fj,
                message: None,
                ajoute_le: "1".into(),
                ajoute_par: id,
                installation: None,
                temps_jeu: 0,
                derniere_partie: None,
            })
            .unwrap();
        let medias = n.dossier_medias(110);
        std::fs::create_dir_all(&medias).unwrap();
        let fiche = serde_json::json!({"id": 110, "titre": "Dune", "versions": [
            {"telechargement_id": 200, "qualite": qualite, "notes": "Lancez DUNE.BAT.", "fichiers": []}]});
        std::fs::write(medias.join("fiche.json"), fiche.to_string()).unwrap();
        let _ = Emplacements::default();
        (d, n)
    }

    fn zip(fichiers: &[(&str, &[u8])]) -> Vec<u8> {
        let mut o = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut o);
            for (nom, contenu) in fichiers {
                z.start_file(*nom, zip::write::SimpleFileOptions::default()).unwrap();
                std::io::Write::write_all(&mut z, contenu).unwrap();
            }
            z.finish().unwrap();
        }
        o.into_inner()
    }

    #[tokio::test]
    async fn une_archive_s_installe_puis_on_choisit_son_lanceur() {
        let archive = zip(&[("DUNE.BAT", b"@echo off"), ("DUNE.EXE", b"MZ"), ("unins000.exe", b"MZ")]);
        let (_d, n) = noyau_avec(&[("dune.zip", &archive)], "Jeu — prêt à jouer").await;
        let p = n.preparer_installation(110).await.unwrap();
        assert!(matches!(p.methode, Methode::Archive { .. }));
        assert_eq!(p.notes.as_deref(), Some("Lancez DUNE.BAT."));
        assert!(p.destination.ends_with("Jeu"));

        let i = n.installer(110, true).await.unwrap();
        assert!(Path::new(&i.dossier).join("DUNE.BAT").is_file());
        let c = n.candidats_lancement(110).await.unwrap();
        assert_eq!(c[0].relatif, "DUNE.BAT");
        n.choisir_lanceur(110, c[0].lanceur.clone()).await.unwrap();
        assert!(n.registre().jeu(110).unwrap().unwrap().installation.unwrap().lanceur.is_some());
    }

    #[tokio::test]
    async fn une_rom_ne_s_installe_pas_et_demande_un_emulateur() {
        let (_d, n) = noyau_avec(&[("Dune (Europe).cue", b"FILE"), ("Dune (Europe).bin", b"\0\0")], "Image disque").await;
        let i = n.installer(110, true).await.unwrap();
        assert_eq!(i.fichier_du_jeu.as_deref(), Some("Dune (Europe).cue"), "le .cue décrit les pistes");
        assert!(matches!(n.jouer(110, None).await, Err(Erreur::Reglage(_))));
        assert!(Path::new(&i.dossier).join("Dune (Europe).bin").is_file(), "rien n'est renommé ni déplacé");
    }

    #[tokio::test]
    async fn jouer_suit_la_partie_et_compte_le_temps() {
        let archive = zip(&[("JEU.BAT", b"@echo off\r\nping -n 2 127.0.0.1 >nul\r\n")]);
        let (_d, n) = noyau_avec(&[("jeu.zip", &archive)], "Jeu — prêt à jouer").await;
        n.installer(110, true).await.unwrap();
        let c = n.candidats_lancement(110).await.unwrap();
        n.choisir_lanceur(110, c[0].lanceur.clone()).await.unwrap();
        let (pid, dossiers) = n.jouer(110, None).await.unwrap();
        let fin = n.suivre_partie(110, pid, dossiers).await.unwrap();
        assert!(fin.secondes <= 30);
        let j = n.registre().jeu(110).unwrap().unwrap();
        assert!(j.derniere_partie.is_some());
    }

    #[tokio::test]
    async fn les_parties_sont_mises_a_l_abri_avant_le_retrait() {
        let archive = zip(&[("DUNE.EXE", b"MZ"), ("DUNE.CFG", b"v1")]);
        let (d, n) = noyau_avec(&[("dune.zip", &archive)], "Jeu — prêt à jouer").await;
        let i = n.installer(110, true).await.unwrap();
        let installe = PathBuf::from(&i.dossier);
        // On joue : une partie apparaît, un réglage change.
        std::fs::create_dir_all(installe.join("SAVES")).unwrap();
        std::fs::write(installe.join("SAVES").join("PARTIE1.SAV"), b"partie").unwrap();
        std::fs::write(installe.join("DUNE.CFG"), b"v2 !").unwrap();

        let abri = n.retirer_du_pc(110).await.unwrap();
        assert_eq!(abri.fichiers, 2);
        let a = PathBuf::from(&abri.dossier);
        assert_eq!(std::fs::read(a.join("SAVES").join("PARTIE1.SAV")).unwrap(), b"partie");
        assert!(a.starts_with(d.path().join("app").join("sauvegardes").join("110")));
        // Le jeu est parti ; l'abri reste.
        assert!(!installe.exists());
        assert!(!d.path().join("Jeux").join("MS-DOS").join("Dune (1992)").join("dune.zip").exists());
        assert!(n.registre().jeu(110).unwrap().is_none());
        assert!(a.is_dir());
    }

    #[tokio::test]
    async fn un_retrait_ne_touche_pas_ce_qui_n_est_pas_a_frogtend() {
        let archive = zip(&[("DUNE.EXE", b"MZ")]);
        let (d, n) = noyau_avec(&[("dune.zip", &archive)], "Jeu — prêt à jouer").await;
        n.installer(110, true).await.unwrap();
        let dossier = d.path().join("Jeux").join("MS-DOS").join("Dune (1992)");
        std::fs::write(dossier.join("mes notes.txt"), b"a moi").unwrap();
        n.retirer_du_pc(110).await.unwrap();
        assert!(dossier.join("mes notes.txt").is_file(), "un fichier posé à côté par la personne reste");
    }

    #[tokio::test]
    async fn une_installation_qui_ne_met_rien_le_dit() {
        // Un « installeur guidé » qui ne met rien dans le dossier prévu (la personne a choisi ailleurs).
        let (d, n) = noyau_avec(&[("setup.exe", b"MZ installeur inconnu")], "Repack").await;
        let p = n.preparer_installation(110).await.unwrap();
        assert!(matches!(p.methode, Methode::InstalleurGuide { .. }));
        // (on ne lance pas un vrai installeur dans un test : on vérifie le repli « installé ailleurs »)
        let ailleurs = d.path().join("Ailleurs");
        std::fs::create_dir_all(&ailleurs).unwrap();
        std::fs::write(ailleurs.join("Jeu.exe"), b"MZ").unwrap();
        let i = n.installe_ailleurs(110, &ailleurs.to_string_lossy()).await.unwrap();
        assert_eq!(n.candidats_lancement(110).await.unwrap()[0].relatif, "Jeu.exe");
        // Retirer ne supprime pas un dossier qui n'est pas dans celui du jeu.
        n.retirer_du_pc(110).await.unwrap();
        assert!(Path::new(&i.dossier).join("Jeu.exe").is_file());
    }
}
