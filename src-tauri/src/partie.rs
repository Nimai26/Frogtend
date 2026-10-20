//! Installer un jeu du PC, choisir ce qu'on lance, jouer (et le dire à Firehouse), mettre les parties à l'abri,
//! retirer un jeu du PC.
//!
//! Règles : rien ne s'installe sans l'accord de la personne (l'appelant le demande) ; les parties sont mises à l'abri
//! AVANT tout retrait ; on n'efface que ce que Frogtend a mis sur le disque, et les abris ne s'effacent jamais.

use crate::erreurs::{Erreur, Resultat};
use crate::installation::{
    arguments_silencieux, candidats, copier, decompresser, executer_et_attendre, lanceur_emulateur,
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
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct Abri {
    pub dossier: String,
    pub fichiers: usize,
    pub octets: u64,
    /// Au retrait d'un jeu installé AILLEURS que dans son dossier Frogtend (installeur guidé, « installé ailleurs ») :
    /// son dossier, qui reste installé — son désinstalleur n'est lancé qu'après un oui de la personne
    /// (`desinstaller_reste`). `None` : rien de tel.
    pub reste_installe: Option<String>,
}

/// La fin d'une partie.
#[derive(Debug, Clone, Serialize)]
pub struct FinDePartie {
    pub jeu: i64,
    pub secondes: u64,
}

/// Le temps laissé à une boutique pour démarrer le jeu (mise à jour, préparation des shaders, fenêtre d'options…).
const ATTENTE_BOUTIQUE: std::time::Duration = std::time::Duration::from_secs(180);

/// L'adresse qui demande à la boutique de lancer ce jeu ; `None` pour une boutique que Frogtend ne sait pas encore
/// lancer (Galaxy : sa ligne de commande n'est pas documentée officiellement, essai à faire avec Seb).
pub(crate) fn adresse_boutique(b: &crate::jeux_pc::LancementBoutique) -> Option<String> {
    match b.source.as_str() {
        "steam" if !b.cle.is_empty() && b.cle.len() <= 12 && b.cle.chars().all(|c| c.is_ascii_digit()) => Some(format!("steam://rungameid/{}", b.cle)),
        _ => None,
    }
}

/// Parmi les processus d'un dossier, le « premier » du jeu : celui dont le parent n'est pas lui-même dans le dossier
/// (le jeu, pas un de ses enfants) ; à défaut le plus petit numéro.
fn racine_du_dossier(pids: &[u32]) -> Option<u32> {
    let mut s = sysinfo::System::new();
    s.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    pids.iter()
        .copied()
        .find(|p| s.process(sysinfo::Pid::from_u32(*p)).and_then(|x| x.parent()).is_none_or(|parent| !pids.contains(&parent.as_u32())))
        .or_else(|| pids.iter().copied().min())
}

/// Demande à la boutique de lancer le jeu, puis attend qu'un programme apparaisse DANS son dossier (jamais le client de
/// la boutique, hors du dossier) : c'est lui que la partie suit (menu en jeu, quitter, manette → clavier).
async fn lancer_par_la_boutique(b: &crate::jeux_pc::LancementBoutique, dossier: &Path) -> Resultat<u32> {
    let adresse = adresse_boutique(b).ok_or_else(|| Erreur::Refus("Frogtend ne sait pas encore lancer un jeu de cette boutique : ouvre-le depuis elle.".into()))?;
    if !crate::lancement::processus_du_dossier(dossier).is_empty() {
        return Err(Erreur::Refus("Ce jeu tourne déjà.".into()));
    }
    tauri_plugin_opener::open_url(&adresse, None::<&str>).map_err(|_| Erreur::Disque("Steam ne s'ouvre pas : est-il installé ?".into()))?;
    let d = dossier.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let fin = Instant::now() + ATTENTE_BOUTIQUE;
        while Instant::now() < fin {
            let pids = crate::lancement::processus_du_dossier(&d);
            if let Some(p) = racine_du_dossier(&pids) {
                return Ok(p);
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        Err(Erreur::Refus(
            "Le jeu n'a pas démarré depuis Steam (3 minutes). Regarde dans Steam : une mise à jour, une fenêtre d'options ou une connexion attend peut-être ta réponse."
                .into(),
        ))
    })
    .await
    .map_err(|_| Erreur::Disque("L'attente du jeu s'est arrêtée brutalement.".into()))?
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

/// Un lanceur par `cmd.exe` ou `explorer.exe` (`nom` : le nom du programme, en minuscules) qui n'ouvre qu'UN fichier
/// existant : `cmd.exe /C <.bat|.cmd>` ou `explorer.exe <.lnk>`. `permis(fichier, raccourci)` dit si ce fichier est à
/// une place admise (du jeu, choisi par la personne, et pour un raccourci le Bureau ou le menu Démarrer).
fn lanceur_systeme_du_jeu(l: &Lanceur, nom: &str, permis: &dyn Fn(&Path, bool) -> bool) -> bool {
    let fichier = |f: &str, exts: &[&str]| {
        let p = Path::new(f);
        p.is_file() && p.extension().and_then(|e| e.to_str()).is_some_and(|e| exts.iter().any(|x| e.eq_ignore_ascii_case(x)))
    };
    match nom {
        "cmd.exe" => matches!(l.arguments.as_slice(), [c, f] if c.eq_ignore_ascii_case("/C") && fichier(f, &["bat", "cmd"]) && permis(Path::new(f), false)),
        "explorer.exe" => matches!(l.arguments.as_slice(), [f] if fichier(f, &["lnk"]) && permis(Path::new(f), true)),
        _ => false,
    }
}

/// Les dossiers où un installeur pose ses raccourcis : le Bureau et le menu Démarrer (de la personne et de tous).
fn dossiers_de_raccourcis() -> Vec<PathBuf> {
    let env = |n: &str| std::env::var_os(n).map(PathBuf::from);
    let menu = |b: PathBuf| b.join("Microsoft").join("Windows").join("Start Menu");
    [env("USERPROFILE").map(|u| u.join("Desktop")), env("PUBLIC").map(|p| p.join("Desktop")), env("APPDATA").map(menu), env("ProgramData").map(menu)]
        .into_iter()
        .flatten()
        .collect()
}

/// L'installation telle que CETTE partie la lance : une autre version (région) pour cette partie seulement.
fn installation_de_la_partie(j: &JeuPc, version: Option<&str>) -> Resultat<Installation> {
    let mut i = j.installation.clone().ok_or_else(|| Erreur::Refus("Installe d'abord le jeu.".into()))?;
    if let Some(v) = version {
        let v = i.versions.iter().find(|x| x.chemin.eq_ignore_ascii_case(v)).ok_or_else(|| Erreur::Refus("Cette version n'est pas celle de ce jeu.".into()))?;
        let p = Path::new(&v.chemin);
        i.dossier = p.parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_default();
        i.fichier_du_jeu = p.file_name().map(|n| n.to_string_lossy().to_string());
    }
    Ok(i)
}

impl Noyau {
    /// Le fichier qu'un émulateur recevra pour cette partie (`None` : un lanceur choisi, ou rien à passer) — le même
    /// choix que [`Noyau::jouer`].
    pub(crate) async fn fichier_pour_l_emulateur(&self, id: i64, version: Option<&str>) -> Resultat<Option<PathBuf>> {
        let j = self.jeu_jouable(id).await?;
        let i = installation_de_la_partie(&j, version)?;
        Ok(match (&i.lanceur, &i.fichier_du_jeu) {
            (None, Some(f)) => Some(Path::new(&i.dossier).join(f)),
            _ => None,
        })
    }

    /// Un jeu du PC que le profil ouvert peut LANCER : visible par lui, ou un jeu importé par un autre profil et relié
    /// à une fiche qu'il voit (✅ Seb 20/10 : jouer seulement ; gérer reste à celui qui l'a importé).
    pub(crate) async fn jeu_jouable(&self, id: i64) -> Resultat<JeuPc> {
        let s = self.session().await?;
        let j = self.registre().jeu(id)?.ok_or_else(|| Erreur::Introuvable("Ce jeu n'est pas sur ce PC.".into()))?;
        // Le verrou du cache est relâché avant `fiche_de` (qui le reprend) : jamais deux fois à la suite.
        let visible = s.verrou().visible_ou_reuni(id)?;
        if visible || (id < 0 && self.fiche_de(&s, id)?.is_some()) {
            return Ok(j);
        }
        Err(Erreur::Introuvable("Ce jeu n'est pas sur ce PC.".into()))
    }

    /// Un jeu du PC, visible par le profil ouvert.
    pub(crate) async fn jeu_visible(&self, id: i64) -> Resultat<JeuPc> {
        let s = self.session().await?;
        let j = self.registre().jeu(id)?.ok_or_else(|| Erreur::Introuvable("Ce jeu n'est pas sur ce PC.".into()))?;
        if !s.verrou().visible_ou_reuni(id)? {
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
        // Une version MAME (contrat 1.13) se donne telle quelle à MAME : une ROM (jamais décompressée).
        let n = if version["mame"].is_object() { Nature::Rom } else { nature(version["qualite"].as_str().unwrap_or("")) };
        let noms: Vec<String> = j.fichiers.iter().map(|f| f.nom.clone()).collect();
        let m = methode(n, Path::new(&j.dossier), &noms)?;
        let destination = match m {
            Methode::Aucune => j.dossier.clone(),
            _ => Path::new(&j.dossier).join(DOSSIER_INSTALLATION).to_string_lossy().into(),
        };
        let mut notes = version["notes"].as_str().filter(|t| !t.trim().is_empty()).map(String::from);
        if version["mame"]["complet"] == Value::Bool(false) {
            let a = "⚠️ Version incomplète chez Firehouse (une partie du jeu, son disque CHD, manque encore) : il peut ne pas démarrer.";
            notes = Some(match notes {
                Some(n) => format!("{a}\n\n{n}"),
                None => a.into(),
            });
        }
        Ok(Preparation { nature: n, methode: m, destination, notes, titre: j.titre })
    }

    /// Installe le jeu (après l'accord de la personne). `automatique` : installeur silencieux si possible.
    /// Rend l'installation ; son lanceur reste à choisir (sauf ROM et image disque : l'émulateur s'en charge).
    pub async fn installer(&self, id: i64, automatique: bool) -> Resultat<Installation> {
        // Jamais rattaché à une autre fiche pendant l'installation (contrat 1.14).
        let _occupe = self.occuper(id);
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
                    let noms: Vec<String> = j.fichiers.iter().map(|f| f.nom.clone()).collect();
                    // MAME : le zip du jeu (son nom court), jamais la CHD rangée à côté.
                    fichier_du_jeu = match self.mame_de(&j) {
                        Some(nom) => Some(noms.iter().find(|f| f.eq_ignore_ascii_case(&format!("{nom}.zip"))).cloned().ok_or_else(|| {
                            Erreur::Refus(format!("Le fichier {nom}.zip de ce jeu d'arcade manque dans ce que Firehouse a envoyé : il ne pourrait pas démarrer."))
                        })?),
                        None => fichier_principal(&noms),
                    };
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
        let i = self.retenir_installation(&j, &dest, p.methode, fichier_du_jeu, p.nature)?;
        self.reposer_parties_restaurees(id, &i).await;
        Ok(i)
    }

    /// Des parties restaurées attendent ce jeu : elles sont reposées dans son dossier tout neuf (APRÈS le relevé des
    /// fichiers, pour être reconnues comme des parties et resauvegardées). Un échec est noté, sans défaire
    /// l'installation.
    async fn reposer_parties_restaurees(&self, id: i64, i: &Installation) {
        let Ok(s) = self.session().await else { return };
        let anciens: Vec<i64> = s.verrou().fusions().unwrap_or_default().into_iter().filter(|(_, n, _)| *n == id).map(|(a, _, _)| a).collect();
        match self.reposer_parties_du_jeu(&s.profil.id, id, &anciens, Path::new(&i.dossier)) {
            Ok(r) if r.reposes > 0 => self.journaliser(&format!("jeu {id} : {} partie(s) restaurée(s) reposée(s)", r.reposes)),
            Ok(_) => {}
            Err(e) => self.journaliser(&format!("jeu {id} : parties restaurées non reposées : {e:?}")),
        }
    }

    /// Le jeu a été installé ailleurs que prévu (installeur guidé) : la personne indique où.
    pub async fn installe_ailleurs(&self, id: i64, dossier: &str) -> Resultat<Installation> {
        let _occupe = self.occuper(id);
        let j = self.jeu_visible(id).await?;
        let p = self.preparer_installation(id).await?;
        let d = PathBuf::from(dossier);
        if !d.is_dir() {
            return Err(Erreur::Disque(format!("Dossier introuvable : {dossier}.")));
        }
        // Durcissement : ce dossier devient celui du jeu (lancé, puis peut-être désinstallé). Jamais un chemin relatif
        // ou détourné, ni la racine d'un disque, Windows, Program Files, le dossier de l'utilisateur ou un de ses
        // dossiers de base.
        if crate::garde::cle(&d).is_none() || crate::garde::dossier_systeme(&d) {
            return Err(Erreur::Refus(format!(
                "« {dossier} » ne peut pas être le dossier d'un jeu : choisis le dossier où le jeu est installé (par exemple « D:\\Jeux\\Mon jeu »)."
            )));
        }
        // Ni un dossier qui CONTIENT d'autres jeux du PC (le dossier d'une console, `E:\Jeux\Windows`) : le relevé
        // des parties porterait sur tous, et leur retrait pourrait les toucher.
        let autres = self.registre().tous()?.into_iter().filter(|x| x.id != id).any(|x| {
            crate::garde::sous(Path::new(&x.dossier), &d)
                || x.installation.as_ref().is_some_and(|i| crate::garde::sous(Path::new(&i.dossier), &d))
        });
        if autres {
            return Err(Erreur::Refus(format!(
                "« {dossier} » contient d'autres jeux de ta ludothèque : choisis le dossier de CE jeu seulement."
            )));
        }
        let i = self.retenir_installation(&j, &d, p.methode, None, p.nature)?;
        self.reposer_parties_restaurees(id, &i).await;
        Ok(i)
    }

    /// Le nom court MAME de la version installée (contrat 1.13), d'après la fiche gardée sur le PC.
    fn mame_de(&self, j: &JeuPc) -> Option<String> {
        let fiche = self.fiche_locale(j.id)?;
        let v = fiche["versions"].as_array()?.iter().find(|v| v["telechargement_id"].as_i64() == Some(j.version))?.clone();
        v["mame"]["nom"].as_str().filter(|n| crate::jeux_pc::nom_de_fichier_sur(n).is_ok()).map(String::from)
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
            versions: vec![],
            boutique: None,
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

    /// La version lancée par défaut d'un jeu importé (une de ses versions).
    pub async fn choisir_version(&self, id: i64, chemin: &str) -> Resultat<()> {
        let j = self.jeu_visible(id).await?;
        let mut i = j.installation.ok_or_else(|| Erreur::Refus("Le jeu n'est pas installé.".into()))?;
        let v = i.versions.iter().find(|x| x.chemin.eq_ignore_ascii_case(chemin)).ok_or_else(|| Erreur::Refus("Cette version n'est pas celle de ce jeu.".into()))?;
        if !Path::new(&v.chemin).is_file() {
            return Err(Erreur::Disque(format!("Fichier introuvable : {} (disque débranché ?).", v.chemin)));
        }
        let p = Path::new(&v.chemin);
        i.dossier = p.parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_default();
        i.fichier_du_jeu = p.file_name().map(|n| n.to_string_lossy().to_string());
        self.registre().changer_installation(id, Some(&i))
    }

    /// Le fichier que l'émulateur reçoit pour ce jeu (version par défaut), s'il y en a un et si « décompresser pour
    /// jouer » concerne sa plateforme (jamais l'arcade).
    pub async fn fichier_lance(&self, id: i64) -> Resultat<Option<PathBuf>> {
        let j = self.jeu_visible(id).await?;
        if !crate::decompression::concerne(&j.plateforme) {
            return Ok(None);
        }
        Ok(j.installation.filter(|i| i.lanceur.is_none()).and_then(|i| i.fichier_du_jeu.map(|f| Path::new(&i.dossier).join(f))))
    }

    /// Après « décompresser pour jouer » : le jeu se lance désormais par `nouveau` au lieu de l'archive `ancien`
    /// (version par défaut et liste des versions). L'archive reste sur le disque.
    pub async fn remplacer_fichier(&self, id: i64, ancien: &Path, nouveau: &Path) -> Resultat<()> {
        let j = self.jeu_visible(id).await?;
        let mut i = j.installation.ok_or_else(|| Erreur::Refus("Le jeu n'est pas installé.".into()))?;
        if !nouveau.is_file() {
            return Err(Erreur::Disque(format!("Fichier introuvable : {}.", nouveau.display())));
        }
        let pareil = |c: &Path| c.to_string_lossy().eq_ignore_ascii_case(&ancien.to_string_lossy());
        let mut change = false;
        if i.fichier_du_jeu.as_ref().is_some_and(|f| pareil(&Path::new(&i.dossier).join(f))) {
            i.dossier = nouveau.parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_default();
            i.fichier_du_jeu = nouveau.file_name().map(|n| n.to_string_lossy().to_string());
            change = true;
        }
        // Seule l'archive est remplacée : les autres disques d'un jeu multi-disques restent.
        let n = nouveau.to_string_lossy().to_string();
        for v in i.versions.iter_mut() {
            if pareil(Path::new(&v.chemin)) {
                v.chemin = n.clone();
                change = true;
            }
            for d in v.disques.iter_mut().filter(|d| pareil(Path::new(d.as_str()))) {
                *d = n.clone();
                change = true;
            }
        }
        if !change {
            return Err(Erreur::Refus("Cette archive n'est pas celle de ce jeu.".into()));
        }
        self.registre().changer_installation(id, Some(&i))
    }

    /// Retient ce qu'on lance pour jouer (choisi par la personne).
    pub async fn choisir_lanceur(&self, id: i64, lanceur: Lanceur) -> Resultat<()> {
        let _occupe = self.occuper(id);
        let j = self.jeu_visible(id).await?;
        let mut i = j.installation.ok_or_else(|| Erreur::Refus("Le jeu n'est pas encore installé.".into()))?;
        if i.boutique.is_some() {
            return Err(Erreur::Refus("Ce jeu est lancé par sa boutique : rien à choisir.".into()));
        }
        // Durcissement : un programme de Windows (nommé court, ou n'importe quel chemin sous `%SystemRoot%` : cmd,
        // powershell, rundll32…) n'est accepté que s'il s'agit de `cmd.exe` ou `explorer.exe` ouvrant UN fichier — un
        // .bat/.cmd du jeu (`/C <fichier>`), un raccourci .lnk du jeu, du Bureau ou du menu Démarrer — exactement la
        // forme que Frogtend propose ; jamais une commande libre. Tout autre programme : sans arguments (le jeu
        // lui-même, comme les candidats et « Un autre programme… »).
        let sous_windows = std::env::var_os("SystemRoot").is_some_and(|w| crate::garde::sous(Path::new(&lanceur.programme), Path::new(&w)));
        let nom = Path::new(&lanceur.programme).file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
        let court = ["cmd.exe", "explorer.exe"].iter().any(|p| lanceur.programme.eq_ignore_ascii_case(p));
        let dossier_du_jeu = PathBuf::from(&i.dossier);
        // Une place admise : dans le dossier du jeu, ou choisi par la personne dans une fenêtre ouverte par le cœur
        // (étape 4), ou, pour un raccourci, le Bureau ou le menu Démarrer.
        let permis = |f: &Path, raccourci: bool| {
            crate::garde::sous(f, &dossier_du_jeu)
                || raccourci && dossiers_de_raccourcis().iter().any(|r| crate::garde::sous(f, r))
                || self.consommer_choix(false, f)
        };
        if court || sous_windows {
            if !lanceur_systeme_du_jeu(&lanceur, &nom, &permis) {
                return Err(Erreur::Refus(
                    "Ce lanceur n'est pas accepté : « cmd.exe » ne sert qu'à ouvrir un fichier .bat ou .cmd, « explorer.exe » un raccourci .lnk — du jeu, ou choisis-le avec « 📂 Un autre programme… ».".into(),
                ));
            }
        } else {
            if !Path::new(&lanceur.programme).is_file() {
                return Err(Erreur::Disque(format!("Programme introuvable : {}.", lanceur.programme)));
            }
            if !lanceur.arguments.is_empty() {
                return Err(Erreur::Refus("Ce lanceur n'est pas accepté : le programme du jeu se lance tel quel, sans arguments.".into()));
            }
            if !permis(Path::new(&lanceur.programme), false) {
                return Err(Erreur::Refus(
                    "Ce programme n'est pas dans le dossier du jeu : choisis-le toi-même avec « 📂 Un autre programme… ».".into(),
                ));
            }
        }
        i.lanceur = Some(lanceur);
        self.registre().changer_installation(id, Some(&i))
    }

    /// Lance le jeu. `emulateur` : (programme, ligne de commande) pour une ROM ou une image disque.
    /// Annonce le début de la session à Firehouse. Rend le processus, les dossiers à surveiller, et si Firehouse a
    /// ouvert une session pour ce PC (`session` non nulle : PC qui calcule pour lui). Au 30/09, cela ne met en pause
    /// qu'une partie de ses travaux sur la carte graphique : on dit « session annoncée », pas « carte libérée ».
    pub async fn jouer(&self, id: i64, emulateur: Option<(String, String)>, version: Option<&str>) -> Resultat<(u32, Vec<PathBuf>, bool)> {
        let _occupe = self.occuper(id);
        let j = self.jeu_jouable(id).await?;
        let i = installation_de_la_partie(&j, version)?;
        self.pause_mame.store(false, std::sync::atomic::Ordering::SeqCst);
        // Un jeu d'une boutique : lancé PAR elle (mises à jour, sauvegardes dans le nuage, compte), puis suivi.
        if let Some(b) = &i.boutique {
            let pid = lancer_par_la_boutique(b, Path::new(&i.dossier)).await?;
            let reponse = self.annoncer(id, "debut").await;
            return Ok((pid, vec![PathBuf::from(&i.dossier)], reponse.is_some_and(|r| r["session"].is_object())));
        }
        let (lanceur, dossiers) = match (&i.lanceur, &i.fichier_du_jeu) {
            (Some(l), _) => (l.clone(), vec![PathBuf::from(&i.dossier)]),
            (None, Some(f)) => {
                let (programme, ligne) = emulateur.ok_or_else(|| {
                    Erreur::Reglage(format!(
                        "Aucun émulateur n'est réglé pour {}. Règle-le dans « Réglages » ▸ « Émulateurs ».",
                        j.plateforme
                    ))
                })?;
                // Une machine de salon par MAME (contrat 1.16 : `studio2 -cart "%rom%"`) reçoit sa cartouche à sa place,
                // même zippée : décidé d'après la LIGNE, jamais d'après l'extension.
                let arcade = crate::installation::nom_court_mame(&programme, f).filter(|_| !crate::installation::ligne_mame_de_salon(&ligne));
                let l = match arcade {
                    // MAME : « mame <nom court> -rompath <dossier du jeu> » (contrat 1.13), jamais le chemin du zip.
                    Some(nom) => {
                        // Refusé AVANT de rien écrire dans le dossier de MAME.
                        crate::installation::verifier_rompath(&i.dossier)?;
                        // Le menu en jeu met MAME en pause par son script — jamais à la place d'un script de démarrage
                        // réglé par la personne. S'il ne peut être posé, MAME se lance quand même, et le menu ne promet
                        // aucune pause.
                        let dossier_mame = Path::new(&programme).parent().map(Path::to_path_buf).unwrap_or_default();
                        let script = if !crate::installation::autoboot_libre(&dossier_mame, &ligne) {
                            None
                        } else {
                            match crate::pilotage::preparer_mame(&dossier_mame) {
                                Ok(()) => Some(crate::pilotage::SCRIPT_MAME),
                                Err(e) => {
                                    self.journaliser(&format!("MAME : script du menu en jeu non posé ({e:?})"));
                                    None
                                }
                            }
                        };
                        self.pause_mame.store(script.is_some(), std::sync::atomic::Ordering::SeqCst);
                        crate::installation::lanceur_mame(&programme, &ligne, &nom, Path::new(&i.dossier), script)?
                    }
                    None => lanceur_emulateur(&programme, &ligne, &Path::new(&i.dossier).join(f))?,
                };
                let d = PathBuf::from(&l.dossier);
                (l, vec![d])
            }
            (None, None) => return Err(Erreur::Refus("Choisis d'abord ce qui lance le jeu.".into())),
        };
        let pid = demarrer(&lanceur)?;
        let reponse = self.annoncer(id, "debut").await;
        let carte_cedee = reponse.is_some_and(|r| r["session"].is_object());
        Ok((pid, dossiers, carte_cedee))
    }

    /// Annonce une étape de session à Firehouse. Un échec (hors ligne, 403…) n'empêche jamais de jouer : il est noté,
    /// et on ne réessaie pas.
    async fn annoncer(&self, id: i64, etat: &str) -> Option<Value> {
        let s = self.session().await.ok()?;
        // La fiche que ce profil voit (un import relié : celle de Firehouse ; un import seul : aucune).
        let fiche = self.fiche_de(&s, id).ok().flatten();
        match s.source.session(etat, fiche).await {
            Ok(v) => Some(v),
            Err(e) => {
                self.journaliser(&format!("session « {etat} » du jeu {id} : {e:?}"));
                None
            }
        }
    }

    /// Suit une partie jusqu'à sa fin : « vivant » toutes les 10 min, « fin » à la sortie, temps de jeu compté.
    pub async fn suivre_partie(&self, id: i64, pid: u32, dossiers: Vec<PathBuf>) -> Resultat<FinDePartie> {
        // Toute la partie : la fin compte le temps de jeu sous CE numéro (contrat 1.14 : jamais rattaché pendant).
        let _occupe = self.occuper(id);
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
        let _occupe = self.occuper(id);
        let j = self.jeu_visible(id).await?;
        let i = j.installation.clone().ok_or_else(|| Erreur::Refus("Le jeu n'est pas installé : rien à mettre à l'abri.".into()))?;
        let m: Manifeste = match std::fs::read(self.fichier_manifeste(id)) {
            Ok(o) => serde_json::from_slice(&o).unwrap_or_default(),
            Err(_) => return Ok(Abri::default()),
        };
        let racine = PathBuf::from(&i.dossier);
        if !racine.is_dir() {
            return Err(Erreur::Disque(format!("Le dossier du jeu est introuvable : {}.", racine.display())));
        }
        let changes = crate::locale::parties_changees(&j, &racine, &m)?;
        if changes.is_empty() {
            return Ok(Abri::default());
        }
        let abri = self.dossier_abris()?.join(id.to_string()).join(maintenant());
        let octets = copier(&racine, &changes, &abri)?;
        // Vérifier le résultat : chaque fichier est bien dans l'abri, à la même taille.
        for r in &changes {
            let a = std::fs::metadata(abri.join(r)).map(|m| m.len()).ok();
            let b = std::fs::metadata(racine.join(r)).map(|m| m.len()).ok();
            if a != b {
                return Err(Erreur::Disque(format!("La copie de « {r} » a échoué : le jeu n'est pas retiré.")));
            }
        }
        Ok(Abri { dossier: abri.to_string_lossy().into(), fichiers: changes.len(), octets, reste_installe: None })
    }

    /// Retire un jeu complet du PC : ses parties d'abord à l'abri (sinon rien n'est fait), puis son désinstalleur
    /// s'il en a un, puis ce que Frogtend a mis sur le disque (installation, fichiers reçus, médias). Les abris
    /// restent.
    pub async fn retirer_du_pc(&self, id: i64) -> Resultat<Abri> {
        let _occupe = self.occuper(id);
        let j = self.jeu_visible(id).await?;
        if !matches!(j.etat, Etat::Telecharge) {
            return Err(Erreur::Refus("Ce jeu est en cours de téléchargement : annule plutôt le téléchargement.".into()));
        }
        // Un jeu d'une boutique : c'est elle qui le désinstalle ; Frogtend ne retire pas sa ligne (son temps de jeu).
        if j.installation.as_ref().is_some_and(|i| i.boutique.is_some()) {
            return Err(Erreur::Refus("Ce jeu est installé par sa boutique : désinstalle-le depuis elle.".into()));
        }
        let mut abri = if j.installation.is_some() {
            self.mettre_a_l_abri(id).await?
        } else {
            Abri::default()
        };
        // Un jeu IMPORTÉ du disque appartient à la personne : on le retire de la ludothèque, jamais du disque.
        if crate::import_local::est_importe(&j) {
            self.registre().retirer(id)?;
            self.session().await?.verrou().retirer_local(id)?;
            return Ok(abri);
        }
        let recus = PathBuf::from(&j.dossier);
        if let Some(i) = &j.installation {
            let installe = PathBuf::from(&i.dossier);
            // Le dossier d'installation est-il DANS le dossier du jeu créé par Frogtend ? Comparé sans détour (`..`
            // refusé, sans casse, composant par composant), et sans lien ni jonction entre les deux (durcissement).
            let de_frogtend = !crate::garde::meme(&installe, &recus)
                && crate::garde::sous(&installe, &recus)
                && !crate::garde::lien_entre(&recus, &installe);
            // Le désinstalleur d'Inno Setup, s'il y en a un (il nettoie ce que l'installeur a mis ailleurs) : SEULEMENT
            // dans un dossier installé par Frogtend — jamais celui d'un dossier « installé ailleurs », qui pourrait être
            // le désinstalleur d'un autre logiciel.
            let desinstalleur = installe.join("unins000.exe");
            if desinstalleur.is_file() && de_frogtend {
                let args = vec!["/VERYSILENT".into(), "/SUPPRESSMSGBOXES".into(), "/NORESTART".into()];
                let d = installe.clone();
                let _ = tokio::task::spawn_blocking(move || executer_et_attendre(&desinstalleur, &args, &d)).await;
            } else if desinstalleur.is_file() {
                // Hors du dossier Frogtend : pas lancé tout seul. La personne le voit (`reste_installe`), et peut le
                // lancer elle-même après un oui (`desinstaller_reste`, une fois, pendant 10 minutes).
                self.journaliser(&format!("jeu {id} : désinstalleur d'un dossier hors de Frogtend non lancé ({})", installe.display()));
                if let Some(k) = crate::garde::cle(&installe) {
                    self.desinstalleurs.lock().unwrap_or_else(|e| e.into_inner()).insert(k, std::time::Instant::now());
                }
                abri.reste_installe = Some(installe.to_string_lossy().to_string());
            }
            if de_frogtend && installe.is_dir() {
                std::fs::remove_dir_all(&installe)?;
            }
        }
        for f in &j.fichiers {
            let rel = crate::jeux_pc::chemin_de_fichier_sur(&f.nom)?;
            let p = recus.join(&rel);
            if p.is_file() {
                std::fs::remove_file(&p)?;
            }
            // Un sous-dossier reçu (CHD MAME), seulement s'il est vide.
            if rel.components().count() > 1 {
                if let Some(d) = p.parent() {
                    let _ = std::fs::remove_dir(d);
                }
            }
        }
        self.retirer_medias(&j)?;
        let _ = std::fs::remove_dir(&recus); // seulement s'il est vide
        self.registre().retirer(id)?;
        Ok(abri)
    }

    /// Lance le désinstalleur d'un jeu retiré qui restait installé AILLEURS (`Abri::reste_installe`), après le oui de la
    /// personne : seulement ce dossier-là, rendu par le retrait il y a moins de 10 minutes, une seule fois ; avec la
    /// fenêtre du désinstalleur (elle voit ce qu'il fait). Rend son code de sortie.
    pub async fn desinstaller_reste(&self, dossier: &str) -> Resultat<i32> {
        let d = PathBuf::from(dossier);
        let k = crate::garde::cle(&d).ok_or_else(|| Erreur::Refus("Dossier inconnu.".into()))?;
        let quand = self.desinstalleurs.lock().unwrap_or_else(|e| e.into_inner()).remove(&k);
        if !quand.is_some_and(|q| q.elapsed() < Duration::from_secs(600)) {
            return Err(Erreur::Refus("Ce désinstalleur n'a pas été proposé par Frogtend (ou il y a trop longtemps) : retire d'abord le jeu.".into()));
        }
        let desinstalleur = d.join("unins000.exe");
        if !desinstalleur.is_file() {
            return Err(Erreur::Disque(format!("Le désinstalleur n'est plus là ({}).", desinstalleur.display())));
        }
        self.journaliser(&format!("désinstalleur lancé après accord : {}", desinstalleur.display()));
        tokio::task::spawn_blocking(move || executer_et_attendre(&desinstalleur, &[], &d))
            .await
            .map_err(|_| Erreur::Disque("Le désinstalleur s'est arrêté brutalement.".into()))?
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
            std::fs::create_dir_all(dossier.join(nom).parent().unwrap()).unwrap();
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
    async fn un_jeu_mame_est_une_rom_son_zip_est_lance_jamais_sa_chd() {
        // Relevé réel du 16/10 : « Jeu — MAME 0.289 » (une nature non dite) ; sfiiia.zip + sfiii/cap-sf3-3.chd.
        let (_d, n) = noyau_avec(&[("sfiiia.zip", b"PK\x03\x04zip"), ("sfiii/cap-sf3-3.chd", b"MComprHD")], "Jeu — MAME 0.289").await;
        let fiche = serde_json::json!({"id": 110, "titre": "Street Fighter III", "versions": [
            {"telechargement_id": 200, "qualite": "Jeu — MAME 0.289", "notes": "", "fichiers": [],
             "mame": {"nom": "sfiiia", "version": "0.289", "parent": "sfiii", "complet": false}}]});
        std::fs::write(n.dossier_medias(110).join("fiche.json"), fiche.to_string()).unwrap();
        let p = n.preparer_installation(110).await.unwrap();
        assert_eq!(p.nature, Nature::Rom, "jamais décompressé");
        assert_eq!(p.methode, Methode::Aucune);
        assert!(p.notes.unwrap().contains("incomplète"), "une CHD manque : c'est dit avant d'installer");
        let i = n.installer(110, true).await.unwrap();
        assert_eq!(i.fichier_du_jeu.as_deref(), Some("sfiiia.zip"), "le zip, jamais la CHD");
        // Retirer le jeu retire aussi le sous-dossier reçu (vide).
        n.retirer_du_pc(110).await.unwrap();
        assert!(!std::path::Path::new(&i.dossier).join("sfiii").exists());
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

        // Durcissement : `cmd.exe` n'ouvre qu'UN .bat/.cmd DU JEU, `explorer.exe` qu'un .lnk du jeu — jamais une
        // commande libre ni un fichier d'ailleurs.
        let bat = Path::new(&i.dossier).join("DUNE.BAT").to_string_lossy().to_string();
        let dossier = i.dossier.clone();
        let l = |programme: &str, arguments: Vec<String>| Lanceur { programme: programme.into(), arguments, dossier: dossier.clone() };
        let ailleurs = tempfile::tempdir().unwrap();
        let bat_ailleurs = ailleurs.path().join("x.bat");
        std::fs::write(&bat_ailleurs, b"@echo off").unwrap();
        for refuse in [
            l("cmd.exe", vec!["/C".into(), "del".into(), "/S".into(), r"C:\".into()]),
            l("cmd.exe", vec!["/C".into(), bat_ailleurs.to_string_lossy().to_string()]),
            l("cmd.exe", vec!["/K".into(), bat.clone()]),
            l("explorer.exe", vec![r"C:\Windows".into()]),
            l("explorer.exe", vec![bat.clone()]),
            // Par son chemin complet : traité comme `cmd.exe`, et toujours sous la forme stricte.
            l(r"C:\Windows\System32\cmd.exe", vec!["/C".into(), "del".into(), r"C:\x".into()]),
            l(r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe", vec!["-c".into(), "x".into()]),
            // Le programme du jeu : sans arguments.
            l(&Path::new(&i.dossier).join("DUNE.EXE").to_string_lossy(), vec!["--x".into()]),
        ] {
            assert!(matches!(n.choisir_lanceur(110, refuse.clone()).await, Err(Erreur::Refus(_))), "{:?}", refuse.arguments);
        }
        n.choisir_lanceur(110, l("CMD.EXE", vec!["/c".into(), bat.clone()])).await.unwrap();
        if let Some(w) = std::env::var_os("SystemRoot") {
            let cmd = Path::new(&w).join("System32").join("cmd.exe").to_string_lossy().to_string();
            n.choisir_lanceur(110, l(&cmd, vec!["/C".into(), bat])).await.unwrap();
        }
        n.choisir_lanceur(110, l(&Path::new(&i.dossier).join("DUNE.EXE").to_string_lossy(), vec![])).await.unwrap();
        // Étape 4 : un programme hors du dossier du jeu n'est accepté que choisi par la personne (fenêtre du cœur),
        // une seule fois.
        let exe_ailleurs = ailleurs.path().join("Jeu.exe");
        std::fs::write(&exe_ailleurs, b"MZ").unwrap();
        let hors = l(&exe_ailleurs.to_string_lossy(), vec![]);
        assert!(matches!(n.choisir_lanceur(110, hors.clone()).await, Err(Erreur::Refus(_))), "pas choisi : refusé");
        n.retenir_choix(true, &exe_ailleurs);
        assert!(n.choisir_lanceur(110, hors.clone()).await.is_err(), "un choix de DOSSIER ne vaut pas pour un programme");
        n.retenir_choix(false, &exe_ailleurs);
        n.choisir_lanceur(110, hors.clone()).await.unwrap();
        assert!(n.choisir_lanceur(110, hors).await.is_err(), "un choix ne sert qu'une fois");
    }

    #[tokio::test]
    async fn une_rom_ne_s_installe_pas_et_demande_un_emulateur() {
        let (_d, n) = noyau_avec(&[("Dune (Europe).cue", b"FILE"), ("Dune (Europe).bin", b"\0\0")], "Image disque").await;
        let i = n.installer(110, true).await.unwrap();
        assert_eq!(i.fichier_du_jeu.as_deref(), Some("Dune (Europe).cue"), "le .cue décrit les pistes");
        assert!(matches!(n.jouer(110, None, None).await, Err(Erreur::Reglage(_))));
        assert!(Path::new(&i.dossier).join("Dune (Europe).bin").is_file(), "rien n'est renommé ni déplacé");
    }

    #[tokio::test]
    async fn jouer_suit_la_partie_et_compte_le_temps() {
        let archive = zip(&[("JEU.BAT", b"@echo off\r\nping -n 2 127.0.0.1 >nul\r\n")]);
        let (_d, n) = noyau_avec(&[("jeu.zip", &archive)], "Jeu — prêt à jouer").await;
        n.installer(110, true).await.unwrap();
        let c = n.candidats_lancement(110).await.unwrap();
        n.choisir_lanceur(110, c[0].lanceur.clone()).await.unwrap();
        let (pid, dossiers, carte) = n.jouer(110, None, None).await.unwrap();
        assert!(!carte, "en simulé, aucune carte n'est cédée");
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

        // Sans dossier des abris réglé : refusé, et le jeu reste (ses parties ne sont pas perdues).
        assert!(matches!(n.retirer_du_pc(110).await, Err(Erreur::Reglage(_))));
        assert!(installe.join("SAVES").join("PARTIE1.SAV").is_file() && n.registre().jeu(110).unwrap().is_some());

        n.regler_abris(Some(d.path().join("Abris")));
        let abri = n.retirer_du_pc(110).await.unwrap();
        assert_eq!(abri.fichiers, 2);
        let a = PathBuf::from(&abri.dossier);
        assert_eq!(std::fs::read(a.join("SAVES").join("PARTIE1.SAV")).unwrap(), b"partie");
        assert!(a.starts_with(d.path().join("Abris").join("110")), "dans le dossier des abris réglé");
        assert!(!d.path().join("app").join("sauvegardes").exists(), "rien dans %APPDATA%");
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

    #[tokio::test]
    async fn une_jonction_dans_le_dossier_installe_n_est_jamais_suivie_a_l_effacement() {
        let archive = zip(&[("DUNE.EXE", b"MZ")]);
        let (d, n) = noyau_avec(&[("dune.zip", &archive)], "Jeu — prêt à jouer").await;
        let i = n.installer(110, true).await.unwrap();
        let ailleurs = d.path().join("Ailleurs");
        std::fs::create_dir_all(&ailleurs).unwrap();
        std::fs::write(ailleurs.join("precieux.sav"), b"partie").unwrap();
        let lien = Path::new(&i.dossier).join("Sauvegardes");
        let ok = std::process::Command::new("cmd").args(["/C", "mklink", "/J"]).arg(&lien).arg(&ailleurs).output().map(|o| o.status.success()).unwrap_or(false);
        if !ok {
            eprintln!("JONCTION impossible ici : test sans objet");
            return;
        }
        n.regler_abris(Some(d.path().join("Abris")));
        n.retirer_du_pc(110).await.unwrap();
        assert!(!Path::new(&i.dossier).join("DUNE.EXE").exists(), "l'installation de Frogtend part");
        assert_eq!(std::fs::read(ailleurs.join("precieux.sav")).unwrap(), b"partie", "ce que vise la jonction reste");
    }

    #[tokio::test]
    async fn installe_ailleurs_refuse_un_dossier_systeme_et_un_effacement_ne_traverse_jamais_une_jonction() {
        let (d, n) = noyau_avec(&[("setup.exe", b"MZ installeur inconnu")], "Repack").await;
        // Jamais Windows ni le dossier de l'utilisateur comme dossier d'un jeu.
        for systeme in ["SystemRoot", "USERPROFILE"] {
            if let Some(s) = std::env::var_os(systeme) {
                assert!(matches!(n.installe_ailleurs(110, &s.to_string_lossy()).await, Err(Erreur::Refus(_))), "{systeme}");
            }
        }
        // Un dossier « installé » DANS le dossier du jeu, mais qui est une JONCTION vers ailleurs : rien d'ailleurs
        // n'est effacé au retrait.
        let recus = PathBuf::from(&n.registre().jeu(110).unwrap().unwrap().dossier);
        let ailleurs = d.path().join("Ailleurs");
        std::fs::create_dir_all(&ailleurs).unwrap();
        std::fs::write(ailleurs.join("precieux.sav"), b"partie").unwrap();
        let lien = recus.join("Jeu");
        let ok = std::process::Command::new("cmd").args(["/C", "mklink", "/J"]).arg(&lien).arg(&ailleurs).output().map(|o| o.status.success()).unwrap_or(false);
        if !ok {
            return; // pas de jonction possible ici
        }
        n.installe_ailleurs(110, &lien.to_string_lossy()).await.unwrap();
        n.retirer_du_pc(110).await.unwrap();
        assert_eq!(std::fs::read(ailleurs.join("precieux.sav")).unwrap(), b"partie", "rien d'ailleurs n'est effacé");
        let _ = std::fs::remove_dir(&lien);
    }
}
