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
use crate::noyau::{maintenant, LancementEnCours, Noyau};
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
/// Les dates des abris FINIS d'un dossier d'abris de jeu (noms tout en chiffres ; un `.en-cours` n'en est pas un). Un
/// abri de sorte `emplacements` (lots S2/S3, à la main ou automatique : ses parties ne sont pas dans le dossier du jeu)
/// n'en est pas un ici : il ne fait jamais reposer la question de la réinstallation.
fn dates_d_abri(du_jeu: &Path) -> Vec<String> {
    std::fs::read_dir(du_jeu)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|d| !d.is_empty() && d.chars().all(|c| c.is_ascii_digit()))
        .filter(|d| !crate::abri_parties::sorte_emplacements(&du_jeu.join(d)))
        .collect()
}

/// Les dossiers de données dont le profil change en ce moment (lot S3, M13 : voir `Noyau::changer_de_profil`).
static PROFILS_EN_CHANGEMENT: std::sync::Mutex<std::collections::BTreeSet<PathBuf>> = std::sync::Mutex::new(std::collections::BTreeSet::new());

/// Un changement de profil en cours : aucun lancement ne commence tant qu'elle vit.
pub(crate) struct ChangementDeProfil<'a>(&'a Noyau);

impl Drop for ChangementDeProfil<'_> {
    fn drop(&mut self) {
        PROFILS_EN_CHANGEMENT.lock().unwrap_or_else(|e| e.into_inner()).remove(&self.0.dossier);
    }
}

/// La première date (secondes) à partir de `depuis` qu'aucun abri de ce jeu n'a prise (ni l'abri, ni son provisoire, ni
/// sa fiche).
fn date_libre(du_jeu: &Path, depuis: &str) -> String {
    let mut n: u64 = depuis.parse().unwrap_or(0);
    loop {
        let d = n.to_string();
        let pris = du_jeu.join(&d).exists() || du_jeu.join(format!("{d}.en-cours")).exists() || du_jeu.join(format!("{d}.json")).exists();
        if !pris {
            return d;
        }
        n += 1;
    }
}

/// Un programme ou une bibliothèque (d'une ancienne version, il casserait le jeu) : jamais remis d'une autre version.
fn est_programme(r: &str) -> bool {
    let r = r.to_ascii_lowercase();
    [".exe", ".dll", ".asi", ".bat", ".cmd", ".com", ".sys", ".ocx"].iter().any(|e| r.ends_with(e))
}

/// Ce qu'il y a à remettre (voir `Noyau::abri_a_reposer`). Lit les abris ; compare par la taille d'abord, puis par
/// blocs (jamais un gros fichier entier en mémoire).
fn plan_abri(j: &JeuPc, i: &Installation, sources: &[PathBuf], manifeste: &Path) -> Resultat<Option<AbriAReposer>> {
    let racine = PathBuf::from(&i.dossier);
    if !racine.is_dir() {
        return Ok(None);
    }
    // Le plus récent de chaque fichier : (date, chemin dans l'abri, version de l'abri).
    let mut plus_recents: std::collections::BTreeMap<String, (String, PathBuf, Option<i64>)> = Default::default();
    let mut date_lue = String::new();
    for du_jeu in sources {
        for date in dates_d_abri(du_jeu) {
            let fiche: serde_json::Value = std::fs::read(du_jeu.join(format!("{date}.json"))).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default();
            let version = fiche["version"].as_i64();
            let dossier = du_jeu.join(&date);
            // Les parties du nuage Steam se rendent par Steam ; les emplacements HORS du dossier du jeu (lot S2) seront
            // remis à leur place (lot S5) — seulement quand l'abri le dit (un dossier `emplacements` du jeu reste un
            // dossier du jeu).
            let reservees = crate::abri_parties::reservees(&dossier);
            for r in crate::installation::fichiers_de(&dossier)? {
                if reservees.ecarte(&r) {
                    continue;
                }
                let plus_ancien = plus_recents.get(&r).is_some_and(|(dt, ..)| dt.as_str() >= date.as_str());
                if !plus_ancien {
                    plus_recents.insert(r.clone(), (date.clone(), dossier.join(&r), version));
                }
            }
            if date > date_lue {
                date_lue = date;
            }
        }
    }
    if plus_recents.is_empty() {
        return Ok(None);
    }
    let manifeste: crate::installation::Manifeste = std::fs::read(manifeste).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default();
    let frogtend = crate::locale::dossier_frogtend(j);
    let mut a_poser = Vec::new();
    let mut gardes = 0;
    let mut octets = 0;
    for (r, (_, source, version)) in plus_recents {
        let cible = racine.join(&r);
        if cible.starts_with(&frogtend) {
            continue;
        }
        let meme_version = version == Some(j.version);
        let poser = if !cible.exists() {
            // Absent : posé ; mais jamais un programme d'une autre version (ou d'un abri sans fiche).
            if meme_version || !est_programme(&r) {
                true
            } else {
                gardes += 1;
                false
            }
        } else if crate::installation::memes_contenus(&cible, &source) {
            false
        } else {
            let inchange = manifeste.fichiers.get(&r).is_some() && manifeste.fichiers.get(&r) == crate::installation::etat_fichier(&cible).as_ref();
            if inchange && meme_version {
                true
            } else {
                gardes += 1;
                false
            }
        };
        if poser {
            octets += std::fs::metadata(&source).map(|m| m.len()).unwrap_or(0);
            a_poser.push((r, source));
        }
    }
    if a_poser.is_empty() {
        return Ok(None);
    }
    Ok(Some(AbriAReposer { date: date_lue, fichiers: a_poser.len(), octets, gardes, plan: a_poser }))
}

/// Des parties à l'abri à remettre dans un jeu réinstallé (lot 3).
#[derive(Debug, Clone, Serialize)]
pub struct AbriAReposer {
    /// La date (secondes) de l'abri le plus récent LU (la question ne revient pas avant un abri plus récent).
    pub date: String,
    pub fichiers: usize,
    pub octets: u64,
    /// Fichiers que la personne a déjà changés depuis la réinstallation (ou d'une autre version) : jamais touchés.
    pub gardes: usize,
    #[serde(skip)]
    pub plan: Vec<(String, PathBuf)>,
}

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

/// L'adresse qui demande à la boutique de lancer ce jeu ; `None` pour une boutique qui ne se lance pas par une adresse.
pub(crate) fn adresse_boutique(b: &crate::jeux_pc::LancementBoutique) -> Option<String> {
    match b.source.as_str() {
        "steam" if !b.cle.is_empty() && b.cle.len() <= 12 && b.cle.chars().all(|c| c.is_ascii_digit()) => Some(format!("steam://rungameid/{}", b.cle)),
        // Epic (A6d) : son manifeste relu à l'instant (namespace et item ne sont pas gardés au registre).
        "epic" => crate::boutiques::installes_epic_du_pc().and_then(|l| l.into_iter().find(|j| j.app == b.cle)).map(|j| crate::boutiques::adresse_epic(&j)),
        // Lot A (18/11) : Ubisoft, Amazon (adresses officielles, clé validée).
        "uplay" | "amazon" => crate::boutiques_pc::adresse(&b.source, &b.cle, crate::boutiques_pc::Action::Jouer, None),
        // Xbox : l'application du paquet relue à l'instant (`shell:AppsFolder\<famille>!<application>`).
        "xboxone" => crate::boutiques_pc::application_xbox(&b.cle).and_then(|a| crate::boutiques_pc::adresse("xboxone", &b.cle, crate::boutiques_pc::Action::Jouer, Some(&a))),
        _ => None,
    }
}

/// Le nom d'une boutique pour les messages.
pub(crate) fn nom_boutique(source: &str) -> &'static str {
    crate::boutiques_pc::nom_boutique(source)
}

/// GOG (A6, ✅ Seb 21/10 : PAR GALAXY, pour son nuage, ses succès et son temps de jeu) : `GalaxyClient.exe
/// /command=runGame /gameId=<id> /path=<dossier>` — ligne NON documentée par GOG (forums GOG, LaunchBox, Lutris), à
/// vérifier au premier essai avec Seb. Le client est celui que Galaxy note dans le registre (lecture seule).
fn commande_galaxy(cle: &str, dossier: &Path) -> Option<(PathBuf, Vec<String>)> {
    if cle.is_empty() || cle.len() > 20 || !cle.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let base = crate::registre::lire_texte_hklm(r"SOFTWARE\WOW6432Node\GOG.com\GalaxyClient\paths", "client")?;
    let exe = crate::registre::lire_texte_hklm(r"SOFTWARE\WOW6432Node\GOG.com\GalaxyClient", "clientExecutable").unwrap_or_else(|| "GalaxyClient.exe".into());
    // Jamais un autre programme que le client de Galaxy, même si le registre disait autre chose.
    if !exe.eq_ignore_ascii_case("GalaxyClient.exe") {
        return None;
    }
    let client = PathBuf::from(base).join(exe);
    client.is_file().then(|| (client, vec!["/command=runGame".into(), format!("/gameId={cle}"), format!("/path={}", dossier.display())]))
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

/// Comment lancer un jeu GOG (lot C4c-1, ✅ Seb 25/11) : la DÉCISION seule, testable sans rien lancer.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PlanGog {
    /// La tâche du `goggame-<id>.info`, lancée directement, sans GOG Galaxy.
    Direct(crate::boutiques_pc::TacheGog),
    /// Secours : GOG Galaxy (`commande_galaxy`), parce que la tâche directe manque.
    Galaxy { client: PathBuf, arguments: Vec<String>, motif: crate::boutiques_pc::MotifSecours },
    /// Secours sans Galaxy : le programme que GOG note dans le registre.
    Registre { programme: crate::boutiques_pc::ProgrammeGog, motif: crate::boutiques_pc::MotifSecours },
    /// Rien de sûr à lancer : le message pour la personne.
    Refus(String),
}

impl PlanGog {
    /// Le mot noté au journal (game / principale / galaxy / registre / refus) et la raison du secours.
    pub(crate) fn voie(&self) -> (&'static str, Option<crate::boutiques_pc::MotifSecours>) {
        match self {
            PlanGog::Direct(t) => (t.categorie.mot(), None),
            PlanGog::Galaxy { motif, .. } => ("galaxy", Some(*motif)),
            PlanGog::Registre { motif, .. } => ("registre", Some(*motif)),
            PlanGog::Refus(_) => ("refus", None),
        }
    }
}

/// Le plan de lancement d'un jeu GOG : la tâche directe D'ABORD ; le secours seulement si elle manque — GOG Galaxy
/// s'il est là (et que le jeu n'est pas connu comme celui d'un autre compte, `hors_galaxy`), sinon le programme du
/// registre, sinon un refus. `galaxy` et `registre` ne sont lus que si l'on va au secours.
pub(crate) fn plan_gog(
    direct: Result<crate::boutiques_pc::TacheGog, crate::boutiques_pc::MotifSecours>,
    hors_galaxy: bool,
    galaxy: impl FnOnce() -> Option<(PathBuf, Vec<String>)>,
    registre: impl FnOnce() -> Option<crate::boutiques_pc::ProgrammeGog>,
) -> PlanGog {
    let motif = match direct {
        Ok(t) => return PlanGog::Direct(t),
        Err(m) => m,
    };
    if !hors_galaxy {
        if let Some((client, arguments)) = galaxy() {
            return PlanGog::Galaxy { client, arguments, motif };
        }
    }
    match registre() {
        Some(programme) => PlanGog::Registre { programme, motif },
        None => PlanGog::Refus(if hors_galaxy {
            // Lot C4b : un jeu de l'autre compte GOG, que GOG ne note pas installé sur ce PC.
            "GOG ne note pas ce jeu comme installé sur ce PC : installe-le d'abord avec GOG Galaxy, connecté au compte qui le possède.".into()
        } else {
            "GOG ne note plus ce jeu sur ce PC : réinstalle-le (ou ouvre-le depuis son dossier).".into()
        }),
    }
}

/// Lance un programme d'un jeu GOG (tâche directe ou registre), sans console ni shell ; les arguments passent tels
/// quels (`raw_arg`). L'erreur de Windows est rendue telle quelle (son numéro va au journal) ; `erreur_programme_gog`
/// la dit à la personne.
fn demarrer_programme_gog(p: &crate::boutiques_pc::ProgrammeGog) -> std::io::Result<u32> {
    let mut c = std::process::Command::new(&p.exe);
    c.current_dir(&p.dossier_travail);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        if !p.arguments.is_empty() {
            c.raw_arg(&p.arguments);
        }
    }
    Ok(c.spawn()?.id())
}

/// Le message d'un lancement GOG qui échoue. Les droits d'administrateur (740) ne sont jamais pris tout seuls : la
/// personne peut passer par GOG Galaxy (une élévation sur accord attend une décision de Seb).
fn erreur_programme_gog(e: &std::io::Error) -> Erreur {
    if e.raw_os_error() == Some(740) {
        Erreur::Refus("Ce jeu demande les droits d'administrateur : Frogtend ne les prend pas tout seul. Tu peux le lancer depuis GOG Galaxy.".into())
    } else {
        Erreur::Disque("Le jeu GOG ne démarre pas.".into())
    }
}

/// Demande à la boutique de lancer le jeu, puis attend qu'un programme apparaisse DANS son dossier (jamais le client de
/// la boutique, hors du dossier) : c'est lui que la partie suit (menu en jeu, quitter, manette → clavier).
/// GOG (lot C4c-1) : `direct` est la tâche lue dans `goggame-<id>.info` (`tache_jeu_gog`) ; lancée directement, son
/// processus est suivi tel quel (le dossier du jeu reste surveillé par le suivi de la partie). Un échec de ce
/// lancement est un vrai échec, dit tel quel : jamais de bascule sur Galaxy après coup.
/// `hors_galaxy` (lot C4b, lu seulement si l'on va au secours) : un jeu GOG dont on SAIT (base de Galaxy relue à
/// l'instant, `jeu_gog_hors_galaxy_relu`) qu'il est seulement sur un autre compte que celui ouvert dans GOG Galaxy se
/// lance par le registre (Galaxy ne le lancerait pas).
/// `journal` : note une ligne (ids, nombres et mots seulement — jamais de chemin ni de titre). `avis` : reçoit la voie
/// GOG prise (game / principale / galaxy / registre) DÈS le lancement, avant l'attente du jeu. Rend le processus et,
/// pour GOG, la voie prise.
async fn lancer_par_la_boutique(
    b: &crate::jeux_pc::LancementBoutique,
    dossier: &Path,
    direct: Option<Result<crate::boutiques_pc::TacheGog, crate::boutiques_pc::MotifSecours>>,
    hors_galaxy: bool,
    journal: &(dyn Fn(&str) + Sync),
    avis: &(dyn Fn(&'static str) + Sync),
) -> Resultat<(u32, Option<&'static str>)> {
    let nom = nom_boutique(&b.source);
    if !crate::lancement::processus_du_dossier(dossier).is_empty() {
        return Err(Erreur::Refus("Ce jeu tourne déjà.".into()));
    }
    // Lot A : un lanceur absent est DIT (jamais un ▶ Jouer qui échoue en silence) ; GOG se lance aussi sans Galaxy.
    let source_b = b.source.clone();
    if b.source != "gog" && !tokio::task::spawn_blocking(move || crate::boutiques_pc::lanceur_present(&source_b)).await.unwrap_or(false) {
        return Err(Erreur::Refus(crate::boutiques_pc::message_lanceur_absent(&b.source)));
    }
    let (b2, d2) = (b.clone(), dossier.to_path_buf());
    let adresse = if b.source == "gog" { None } else { tokio::task::spawn_blocking(move || adresse_boutique(&b2)).await.unwrap_or(None) };
    let mut voie = None;
    if b.source == "gog" {
        // Lot C4c-1 : la tâche directe d'abord ; Galaxy, puis le registre, ne sont lus que si elle manque.
        let direct = direct.unwrap_or(Err(crate::boutiques_pc::MotifSecours::SansFichier));
        let cle = b.cle.clone();
        let plan = tokio::task::spawn_blocking(move || {
            plan_gog(
                direct,
                hors_galaxy,
                || commande_galaxy(&cle, &d2),
                || {
                    // Sans GOG Galaxy (lot A) : le programme que GOG note dans le registre (dans le dossier du jeu,
                    // jamais un autre).
                    crate::boutiques_pc::entree_gog(&cle)
                        .and_then(|v| crate::boutiques_pc::programme_gog(&v))
                        .filter(|p| crate::boutiques::cle_chemin(&p.exe).starts_with(&format!("{}\\", crate::boutiques::cle_chemin(&d2))))
                },
            )
        })
        .await
        .map_err(|_| Erreur::Disque("Le choix du programme du jeu s'est arrêté brutalement.".into()))?;
        let (mot, motif) = plan.voie();
        let raison = motif.map(|m| format!(" ({})", m.en_mots())).unwrap_or_default();
        voie = Some(mot);
        match plan {
            PlanGog::Direct(t) => {
                // Lancé sans Galaxy : son processus est suivi DIRECTEMENT (daté dès son départ).
                let pid = demarrer_programme_gog(&t.programme).map_err(|e| {
                    journal(&format!("lancement GOG direct (tâche {mot}) en échec, erreur Windows {}", e.raw_os_error().map(|n| n.to_string()).unwrap_or_else(|| "inconnue".into())));
                    erreur_programme_gog(&e)
                })?;
                crate::lancement::noter_depart(pid);
                journal(&format!("GOG lancé directement (tâche {mot}), processus {pid}"));
                avis(mot);
                return Ok((pid, voie));
            }
            PlanGog::Refus(m) => {
                journal("GOG : rien de sûr à lancer (ni tâche, ni Galaxy, ni registre)");
                return Err(Erreur::Refus(m));
            }
            PlanGog::Registre { programme, .. } => {
                journal(&format!("GOG lancé par le registre{raison}"));
                demarrer_programme_gog(&programme).map_err(|e| erreur_programme_gog(&e))?;
                avis(mot);
            }
            PlanGog::Galaxy { client, arguments: args, .. } => {
                journal(&format!("GOG lancé par GOG Galaxy en secours{raison}"));
                // Le client de Galaxy, sans console et sans shell ; il reste ouvert après le jeu (c'est le sien).
                let mut c = std::process::Command::new(&client);
                // La forme des forums (`/path="D:\…"`), écrite telle quelle : Rust citerait l'argument entier
                // (`"/path=D:\…"`), et la façon dont Galaxy lit sa ligne n'est pas documentée. Sans « \ » final (il
                // neutraliserait le guillemet) ; un « " » ne peut pas être dans un chemin Windows.
                c.args(args.iter().filter(|a| !a.starts_with("/path=")));
                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    c.raw_arg(format!("/path=\"{}\"", dossier.to_string_lossy().trim_end_matches('\\')));
                }
                if let Some(d) = client.parent() {
                    c.current_dir(d);
                }
                c.spawn().map_err(|_| Erreur::Disque("GOG Galaxy ne démarre pas.".into()))?;
                // Dit tout de suite (avant l'attente du jeu, qui peut durer 3 minutes).
                avis(mot);
            }
        }
    } else if let Some(adresse) = adresse {
        tauri_plugin_opener::open_url(&adresse, None::<&str>).map_err(|_| Erreur::Disque(format!("{nom} ne s'ouvre pas : est-il installé ?")))?;
    } else if b.source == "epic" {
        // Le manifeste d'Epic n'est plus là (jeu désinstallé par Epic, dossier resté) : le dire, jamais deviner.
        return Err(Erreur::Refus("Epic ne voit plus ce jeu installé : réinstalle-le ou répare-le depuis l'Epic Games Launcher.".into()));
    } else if b.source == "battlenet" {
        // Aucune adresse de Battle.net prouvée par une source lue (lot A) : pas de lancement deviné.
        return Err(Erreur::Refus(crate::boutiques_pc::message_lanceur_absent("battlenet")));
    } else if b.source == "xboxone" {
        return Err(Erreur::Refus("Windows ne voit plus ce jeu Xbox installé : réinstalle-le depuis l'application Xbox.".into()));
    } else {
        return Err(Erreur::Refus(format!("{nom} ne voit plus ce jeu installé : ouvre-le ou répare-le depuis {nom}.")));
    }
    let d = dossier.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let fin = Instant::now() + ATTENTE_BOUTIQUE;
        while Instant::now() < fin {
            let pids = crate::lancement::processus_du_dossier(&d);
            if let Some(p) = racine_du_dossier(&pids) {
                // Daté dès qu'il est vu : un numéro repris plus tard par un autre programme ne comptera pas.
                crate::lancement::noter_depart(p);
                return Ok((p, voie));
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        Err(Erreur::Refus(format!(
            "Le jeu n'a pas démarré depuis {nom} (3 minutes). Regarde dans {nom} : une mise à jour, une fenêtre d'options ou une connexion attend peut-être ta réponse."
        )))
    })
    .await
    .map_err(|_| Erreur::Disque("L'attente du jeu s'est arrêtée brutalement.".into()))?
}

/// Les jeux en cours d'installation, par dossier de données de Frogtend et numéro (un seul passage à la fois par jeu).
static INSTALLATIONS_EN_COURS: std::sync::Mutex<Vec<(PathBuf, i64)>> = std::sync::Mutex::new(Vec::new());

/// Rendue à la fin de l'installation, même après une erreur. Lot C4c-2 : prise aussi par l'installation d'un jeu GOG
/// (`commandes::gog_installer`, numéro de sa carte).
pub(crate) struct InstallationEnCours((PathBuf, i64));

impl InstallationEnCours {
    pub(crate) fn prendre(donnees: &Path, id: i64) -> Resultat<Self> {
        let cle = (donnees.to_path_buf(), id);
        let mut l = INSTALLATIONS_EN_COURS.lock().unwrap_or_else(|e| e.into_inner());
        if l.contains(&cle) {
            return Err(Erreur::Refus("Ce jeu est déjà en train de s'installer : attends qu'il ait fini.".into()));
        }
        l.push(cle.clone());
        Ok(InstallationEnCours(cle))
    }
}

impl Drop for InstallationEnCours {
    fn drop(&mut self) {
        INSTALLATIONS_EN_COURS.lock().unwrap_or_else(|e| e.into_inner()).retain(|x| *x != self.0);
    }
}

/// Une version dont l'« installation » n'écrit RIEN (une ROM ou une image disque donnée telle quelle, MAME, un jeu
/// Switch avec ses contenus) : elle est prête à jouer dès son arrivée (essai de Seb, 28/10 : « Installer » ne sert à rien
/// pour une Super Nintendo). Jamais des disques zippés à décompresser ni une forme `archive` (du vrai travail, avec
/// l'accord de la personne) ; jamais sans fiche (on ne devine pas). Seule autre écriture, comme à toute installation :
/// les parties restaurées qui attendaient ce jeu, reposées dans son dossier.
pub(crate) fn prete_a_l_arrivee(version: &Value) -> bool {
    if version.is_null() || !roles(version, &["archive_a_extraire"]).is_empty() {
        return false;
    }
    // Un jeu Switch avec ses contenus : sa forme est la sienne (Eden ne lit les contenus qu'après un oui).
    if contenus_emulateur(version) {
        return true;
    }
    forme_fichier(version) && (version_simple(version) || version["mame"].is_object())
}

impl Noyau {
    /// Un jeu qui vient d'arriver (ou arrivé avant cette règle) : installé tout seul si cela n'écrit rien (voir
    /// `prete_a_l_arrivee`), pour le profil qui l'a fait venir. Un échec (zip MAME absent, disque manquant…) est noté au
    /// journal ; « 📦 Installer » reste le recours.
    pub(crate) async fn installer_a_l_arrivee(&self, id: i64, profil: &str) {
        let Ok(s) = self.session().await else { return };
        if s.profil.id != profil {
            return;
        }
        drop(s);
        let Ok(Some(j)) = self.registre().jeu(id) else { return };
        if id <= 0 || j.etat != Etat::Telecharge || j.installation.is_some() || crate::import_local::est_importe(&j) {
            return;
        }
        if !prete_a_l_arrivee(&self.version_de(&j)) {
            return;
        }
        // Double garde : la préparation dit bien « rien à faire » pour une ROM ou une image disque.
        match self.preparer_installation(id).await {
            Ok(p) if p.methode == Methode::Aucune && matches!(p.nature, Nature::Rom | Nature::ImageDisque) => {}
            _ => return,
        }
        match self.installer(id, false).await {
            Ok(_) => self.journaliser(&format!("jeu {id} : prêt à jouer dès son arrivée (rien à installer)")),
            Err(e) => self.journaliser(&format!("jeu {id} : pas prêt à l'arrivée ({e:?}) — « Installer » reste possible")),
        }
    }

    /// À l'ouverture d'un profil : les jeux déjà arrivés mais jamais « installés » alors que rien n'était à faire
    /// (avant cette règle) deviennent prêts à jouer. Aucun téléchargement ni décompression : le registre, et les
    /// parties restaurées qui attendaient ce jeu reposées dans SON dossier (comme à toute installation).
    pub async fn installer_les_arrives(&self) -> usize {
        let Ok(s) = self.session().await else { return 0 };
        let profil = s.profil.id.clone();
        drop(s);
        let candidats: Vec<i64> =
            self.registre().tous().unwrap_or_default().into_iter().filter(|j| j.id > 0 && j.etat == Etat::Telecharge && j.installation.is_none()).map(|j| j.id).collect();
        let mut n = 0;
        for id in candidats {
            // Seulement un jeu que ce profil voit.
            if self.jeu_visible(id).await.is_err() {
                continue;
            }
            self.installer_a_l_arrivee(id, &profil).await;
            if self.registre().jeu(id).ok().flatten().is_some_and(|j| j.installation.is_some()) {
                n += 1;
            }
        }
        n
    }
}

/// Une installation qui n'est qu'une NOTE : méthode `aucune` (rien copié, installé ni décompressé), sans lanceur choisi,
/// sans fichier donné à un émulateur, sans versions, boutique ni fichiers décompressés. La défaire ne touche aucun fichier.
pub(crate) fn note_seule(i: &Installation) -> bool {
    i.methode == Methode::Aucune && i.lanceur.is_none() && i.fichier_du_jeu.is_none() && i.versions.is_empty() && i.boutique.is_none() && i.extraits.is_empty()
}

/// La ligne porte-t-elle plusieurs RÉGIONS d'un jeu (pas les cartes e-Reader, qui ont leur propre lancement) ?
pub(crate) fn plusieurs_versions(j: &JeuPc) -> bool {
    !crate::jeux_pc::a_cartes(&j.plateforme) && j.fichiers.iter().any(|f| f.version.is_some_and(|v| v != j.version))
}

/// Une version qui peut COHABITER avec d'autres sur la même ligne (plusieurs régions d'un jeu, ✅ Seb 28/10) : une ROM
/// ou une image disque donnée telle quelle à l'émulateur — jamais MAME (dossier `-rompath`, CHD), jamais une forme
/// `archive`, des disques zippés à extraire ou des contenus d'émulateur (DLC) : rien n'y est décompressé ni installé,
/// donc rien d'une version ne peut toucher une autre.
pub(crate) fn version_simple(version: &Value) -> bool {
    if version.is_null() || version["mame"].is_object() || contenus_emulateur(version) || !forme_fichier(version) {
        return false;
    }
    if !roles(version, &["archive_a_extraire"]).is_empty() {
        return false;
    }
    match nature(version["qualite"].as_str().unwrap_or("")) {
        Nature::Rom | Nature::ImageDisque => true,
        Nature::NonDite => !roles(version, &["liste_disques"]).is_empty(),
        _ => false,
    }
}

/// Ce que le cœur dit de chaque version d'une fiche avant qu'on coche (l'interface ne refait jamais la règle) : peut-elle
/// cohabiter avec d'autres (`version_simple`), son rang de région et sa qualité (ordre de Seb : FR, EU, US/EN,
/// autres), ses noms de fichiers (deux versions au même nom ne se cochent pas ensemble).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AvisVersion {
    pub telechargement_id: i64,
    pub cochable: bool,
    pub rang: u8,
    pub qualite: i32,
    pub fichiers: Vec<String>,
}

pub fn avis_versions(fiche: &Value) -> Vec<AvisVersion> {
    fiche["versions"]
        .as_array()
        .map(|l| {
            l.iter()
                .filter_map(|v| {
                    let telechargement_id = v["telechargement_id"].as_i64()?;
                    let fichiers: Vec<String> = v["fichiers"].as_array().map(|f| f.iter().filter_map(|x| x["nom"].as_str().map(String::from)).collect()).unwrap_or_default();
                    let lance = lancer_de(v, &fichiers).or_else(|| fichier_principal(&fichiers)).unwrap_or_default();
                    let (rang, qualite, _, _) = crate::import_local::etiquettes(&lance);
                    Some(AvisVersion { telechargement_id, cochable: version_simple(v), rang, qualite, fichiers })
                })
                .collect()
        })
        .unwrap_or_default()
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

/// La version s'installe-t-elle « en fichiers » (contrat 1.23 : `installation.forme` absente, vide ou `fichier`) ?
fn forme_fichier(version: &Value) -> bool {
    matches!(version["installation"]["forme"].as_str().unwrap_or(""), "" | "fichier")
}

/// Une version en forme `archive` (contrat 1.23/1.24) : (son UNIQUE fichier `archive_a_extraire`, reçu ; le sous-dossier
/// `extraire_dans`, chemin relatif sûr, vide = `Jeu/` même). `None` : une autre forme. Plus d'une archive, ou un chemin
/// douteux : refus (jamais deviner).
fn forme_archive(version: &Value, noms: &[String]) -> Resultat<Option<(String, String)>> {
    if version["installation"]["forme"].as_str() != Some("archive") {
        return Ok(None);
    }
    let annoncees = roles(version, &["archive_a_extraire"]);
    let [archive] = annoncees.as_slice() else {
        return Err(Erreur::Refus("Firehouse n'a pas dit quelle archive installer pour ce jeu : rien n'est installé.".into()));
    };
    if !noms.contains(archive) {
        return Err(Erreur::Refus("L'archive de ce jeu n'est pas parmi les fichiers reçus : télécharge-le de nouveau.".into()));
    }
    let sous = version["installation"]["extraire_dans"].as_str().unwrap_or("");
    let sous = if sous.trim().is_empty() {
        String::new()
    } else {
        crate::decompression::chemin_sur(sous).ok_or_else(|| Erreur::Refus("Firehouse indique un dossier d'installation inutilisable : rien n'est installé.".into()))?
    };
    Ok(Some((archive.clone(), sous)))
}

/// Le programme à lancer que dit Firehouse pour une forme `archive` (`installation.lancer`, relatif au dossier
/// d'extraction), s'il y est vraiment, sans arguments (sinon la personne choisit).
fn lanceur_de_l_archive(version: &Value, dest: &Path) -> Option<Lanceur> {
    if !version["installation"]["arguments"].as_str().unwrap_or("").trim().is_empty() {
        return None;
    }
    let rel = crate::decompression::chemin_sur(version["installation"]["lancer"].as_str()?)?;
    let exe = dest.join(&rel);
    if !exe.is_file() || !crate::garde::sous(&exe, dest) || crate::garde::lien_entre(dest, &exe) {
        return None;
    }
    let dossier = exe.parent()?.to_string_lossy().to_string();
    Some(Lanceur { programme: exe.to_string_lossy().into(), arguments: vec![], dossier })
}

/// Un jeu de console servi avec ses mises à jour et DLC, que l'émulateur lit à côté (contrat 1.23 : Switch → Eden).
pub(crate) fn contenus_emulateur(version: &Value) -> bool {
    version["installation"]["forme"].as_str() == Some("contenus_emulateur")
}

/// Le fichier lancé d'une version `contenus_emulateur` : `installation.lancer` s'il est donné, sinon L'UNIQUE fichier
/// au rôle `jeu` — jamais une mise à jour ni un DLC (le premier fichier ne l'est que par hasard).
fn fichier_au_role_jeu(version: &Value, noms: &[String]) -> Resultat<String> {
    let l = version["installation"]["lancer"].as_str().filter(|l| !l.is_empty());
    if let Some(n) = l.and_then(|l| noms.iter().find(|n| n.as_str() == l)) {
        return Ok(n.clone());
    }
    let jeux: Vec<String> = roles(version, &["jeu"]).into_iter().filter(|r| noms.contains(r)).collect();
    match jeux.as_slice() {
        [un] => Ok(un.clone()),
        _ => Err(Erreur::Refus("Firehouse n'a pas dit lequel de ces fichiers est le jeu (les autres sont ses mises à jour et DLC) : rien n'est installé.".into())),
    }
}

/// Les fichiers de la version qui ont l'un de ces rôles (contrat 1.23), nommés comme dans `JeuPc.fichiers`
/// (« dossier/nom »).
pub(crate) fn roles(version: &Value, voulus: &[&str]) -> Vec<String> {
    version["fichiers"]
        .as_array()
        .map(|l| {
            l.iter()
                .filter(|f| voulus.contains(&f["role"].as_str().unwrap_or("jeu")))
                .map(|f| {
                    let nom = f["nom"].as_str().unwrap_or("");
                    match f["dossier"].as_str().filter(|d| !d.is_empty()) {
                        Some(d) => format!("{d}/{nom}"),
                        None => nom.to_string(),
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Le fichier à lancer que dit Firehouse (contrat 1.23 `installation.lancer`), seulement s'il est l'un des fichiers
/// reçus (`noms`) : jamais un chemin venu d'ailleurs.
fn lancer_de(version: &Value, noms: &[String]) -> Option<String> {
    if !forme_fichier(version) {
        return None;
    }
    let l = version["installation"]["lancer"].as_str().filter(|l| !l.is_empty())?;
    noms.iter().find(|n| n.as_str() == l).cloned()
}

/// Une liste de disques (.m3u) plus grande est refusée : ce n'en est pas une.
const M3U_MAX: u64 = 64 * 1024;

/// Chaque disque listé par ce .m3u est-il là, à côté de lui ? Sinon l'émulateur ne démarrerait pas : on le dit au
/// lieu de lancer. Lignes vides et commentaires (`#`) ignorés ; « fichier|libellé » (RetroArch) compris.
fn verifier_m3u(m3u: &Path) -> Resultat<()> {
    let taille = std::fs::metadata(m3u).map(|m| m.len()).map_err(|_| Erreur::Refus(format!("La liste des disques « {} » manque.", m3u.display())))?;
    if taille > M3U_MAX {
        return Err(Erreur::Refus("La liste des disques de ce jeu est anormalement grande : il n'est pas lancé.".into()));
    }
    // Pas de l'UTF-8 (Shift-JIS, Latin-1…) : les noms ne peuvent pas être relus sûrement ; on ne refuse pas à tort.
    let Ok(texte) = String::from_utf8(std::fs::read(m3u)?) else { return Ok(()) };
    let dossier = m3u.parent().unwrap_or(Path::new(""));
    let mut disques = 0;
    for ligne in texte.trim_start_matches('\u{feff}').lines() {
        let l = ligne.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        let nom = l.split('|').next().unwrap_or("").trim();
        let rel = crate::decompression::chemin_sur(nom)
            .ok_or_else(|| Erreur::Refus(format!("La liste des disques nomme « {nom} », hors du dossier du jeu : il n'est pas lancé.")))?;
        if !dossier.join(&rel).is_file() {
            return Err(Erreur::Refus(format!("Il manque le disque « {nom} » à côté de la liste des disques : le jeu ne pourrait pas démarrer.")));
        }
        disques += 1;
    }
    if disques == 0 {
        return Err(Erreur::Refus("La liste des disques de ce jeu est vide.".into()));
    }
    Ok(())
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

    /// Lot C4c-1 : ce jeu GOG installé se lance-t-il DIRECTEMENT (une tâche sûre dans son `goggame-<id>.info`) ? Rien
    /// d'autre n'est rendu (ni chemin ni tâche) : la fiche ne dit la contrepartie que dans ce cas.
    pub async fn gog_lancement_direct(&self, id: i64) -> Resultat<bool> {
        let j = self.jeu_jouable(id).await?;
        let Some(i) = j.installation.filter(|i| i.boutique.as_ref().is_some_and(|b| b.source == "gog")) else { return Ok(false) };
        let (d, cle) = (PathBuf::from(&i.dossier), i.boutique.map(|b| b.cle).unwrap_or_default());
        Ok(tokio::task::spawn_blocking(move || crate::boutiques_pc::tache_jeu_gog(&d, &cle).is_ok()).await.unwrap_or(false))
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
        let version = self.version_de(&j);
        // Une version MAME (contrat 1.13) se donne telle quelle à MAME : une ROM (jamais décompressée).
        let mut n = if version["mame"].is_object() { Nature::Rom } else { nature(version["qualite"].as_str().unwrap_or("")) };
        // Un multi-disques (contrat 1.23 : un .m3u et ses disques) est une image disque, même de nature non dite.
        // Un jeu de console avec ses mises à jour et DLC (contrat 1.23 `contenus_emulateur`) aussi : jamais « installé ».
        if n == Nature::NonDite
            && ((forme_fichier(&version) && !roles(&version, &["liste_disques", "archive_a_extraire"]).is_empty()) || contenus_emulateur(&version))
        {
            n = Nature::ImageDisque;
        }
        let noms: Vec<String> = j.fichiers.iter().map(|f| f.nom.clone()).collect();
        // Une nature non dite dont tous les fichiers sont des ROM ou des images disque de SA plateforme (MK8 Deluxe,
        // 26/11 : un .xci servi « Jeu (nature non dite) ») : l'émulateur, jamais la voie PC. Un .exe ou une archive y reste.
        if n == Nature::NonDite && forme_fichier(&version) {
            if let Some(par_fichiers) = crate::installation::nature_par_les_fichiers(&j.plateforme, &noms) {
                n = par_fichiers;
            }
        }
        // Un jeu PC en archive (contrat 1.23/1.24, forme `archive`) : la FORME fait foi, jamais la qualité (« Jeu — ROM »
        // sur le relevé réel du 21/10) ; UNE archive à extraire dans `Jeu/<extraire_dans>`.
        if let Some((archive, sous)) = forme_archive(&version, &noms)? {
            let format = crate::installation::format_de(&Path::new(&j.dossier).join(crate::jeux_pc::chemin_de_fichier_sur(&archive)?))?;
            if !matches!(format, crate::installation::Format::Zip | crate::installation::Format::SeptZip) {
                return Err(Erreur::Refus("L'archive de ce jeu n'est ni un .zip ni un .7z : Frogtend ne sait pas l'installer.".into()));
            }
            let mut destination = Path::new(&j.dossier).join(DOSSIER_INSTALLATION);
            if !sous.is_empty() {
                destination = destination.join(sous);
            }
            let notes = version["notes"].as_str().filter(|t| !t.trim().is_empty()).map(String::from);
            return Ok(Preparation {
                nature: Nature::PretAJouer,
                methode: Methode::Archive { fichier: archive, format },
                destination: destination.to_string_lossy().into(),
                notes,
                titre: j.titre,
            });
        }
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
        self.installer_suivi(id, automatique, std::sync::Arc::new(|_, _| {})).await
    }

    /// Comme `installer`, en disant l'avancement de la décompression d'un jeu en archive (`progres(faits, total)` en
    /// octets) : un gros jeu prend des minutes, l'écran ne doit jamais sembler figé.
    pub async fn installer_suivi(&self, id: i64, automatique: bool, progres: std::sync::Arc<dyn Fn(u64, u64) + Send + Sync>) -> Resultat<Installation> {
        // UNE installation à la fois pour un jeu (la file à l'arrivée, le rattrapage à l'ouverture du profil, un clic sur
        // « Installer ») : deux en même temps reposeraient les parties restaurées l'une pendant que l'autre vide leur
        // zone d'attente (relecture de l'expert, 28/10).
        let _seule = InstallationEnCours::prendre(&self.dossier, id)?;
        // Jamais rattaché à une autre fiche pendant l'installation (contrat 1.14).
        let _occupe = self.occuper(id);
        let j = self.jeu_visible(id).await?;
        // Un jeu importé appartient à la personne : Frogtend n'écrit rien dans son dossier.
        if crate::import_local::est_importe(&j) {
            return Err(Erreur::Refus("Ce jeu vient de ton disque : il n'y a rien à installer.".into()));
        }
        let p = self.preparer_installation(id).await?;
        let recus = PathBuf::from(&j.dossier);
        let dest = PathBuf::from(&p.destination);
        // Une version en forme `archive` (contrat 1.23/1.24) : son extraction et son programme suivent le contrat.
        let version_archive = Some(self.version_de(&j)).filter(|v| v["installation"]["forme"].as_str() == Some("archive"));
        let journal = self.dossier_medias(id).join("installation.log");
        std::fs::create_dir_all(self.dossier_medias(id))?;

        // Lot S3 (Seb, 23/11) : une RÉINSTALLATION (le jeu est déjà installé) — ses parties rangées hors de son dossier
        // d'abord à l'abri, avant l'installeur ; un échec l'arrête, sauf « continuer sans abri » choisi par la personne.
        // (Une forme `archive` déjà là est refusée plus bas sans rien toucher : pas d'abri pour rien.)
        let refusee_plus_bas = version_archive.is_some() && matches!(p.methode, Methode::Archive { .. }) && std::fs::read_dir(&dest).is_ok_and(|mut l| l.next().is_some());
        if j.installation.is_some() && !refusee_plus_bas {
            crate::commandes::abri_avant(self, id, crate::abri_parties::DECLENCHEUR_AVANT_REINSTALLATION, true).await?;
        }
        let mut fichier_du_jeu = None;
        let mut extraits = Vec::new();
        let mut nouveaux: Vec<PathBuf> = Vec::new();
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
                let archive = recus.join(crate::jeux_pc::chemin_de_fichier_sur(fichier)?);
                let (d, f) = (dest.clone(), *format);
                if version_archive.is_some() {
                    // Forme `archive` (contrat 1.23/1.24) : extraction sûre et complète, jamais par-dessus une
                    // installation déjà là (dit AVANT tout travail : retirer le jeu garde les parties à l'abri).
                    if std::fs::read_dir(&d).is_ok_and(|mut l| l.next().is_some()) {
                        return Err(Erreur::Refus(
                            "Ce jeu est déjà installé ici : pour le réinstaller, retire-le d'abord (⚙ Gérer ▸ Retirer : tes parties sont mises à l'abri), puis installe-le de nouveau.".into(),
                        ));
                    }
                    let libre = crate::jeux_pc::place_libre(&recus);
                    let suivre = progres.clone();
                    tokio::task::spawn_blocking(move || crate::decompression::extraire_dans(&archive, &d, libre, &mut |f, t| suivre(f, t)))
                        .await
                        .map_err(|_| Erreur::Disque("La décompression s'est arrêtée brutalement.".into()))??;
                } else {
                    tokio::task::spawn_blocking(move || decompresser(&archive, f, &d))
                        .await
                        .map_err(|_| Erreur::Disque("La décompression s'est arrêtée brutalement.".into()))??;
                }
            }
            Methode::Aucune => {
                if matches!(p.nature, Nature::Rom | Nature::ImageDisque) {
                    // Plusieurs régions sur la ligne : le fichier lancé par défaut est pris dans SA version seulement.
                    let multi = plusieurs_versions(&j);
                    let noms: Vec<String> = j.fichiers.iter().filter(|f| !multi || f.version.unwrap_or(j.version) == j.version).map(|f| f.nom.clone()).collect();
                    // MAME : le zip du jeu (son nom court), jamais la CHD rangée à côté.
                    fichier_du_jeu = match self.mame_de(&j) {
                        Some(nom) => Some(noms.iter().find(|f| f.eq_ignore_ascii_case(&format!("{nom}.zip"))).cloned().ok_or_else(|| {
                            Erreur::Refus(format!("Le fichier {nom}.zip de ce jeu d'arcade manque dans ce que Firehouse a envoyé : il ne pourrait pas démarrer."))
                        })?),
                        None => {
                            let version = self.version_de(&j);
                            // Contrat 1.23 : les disques zippés se décompressent À CÔTÉ du .m3u qui les liste.
                            let archives = roles(&version, &["archive_a_extraire"])
                                .into_iter()
                                .filter(|a| noms.contains(a))
                                .map(|a| crate::jeux_pc::chemin_de_fichier_sur(&a).map(|r| recus.join(r)))
                                .collect::<Resultat<Vec<_>>>()?;
                            if !archives.is_empty() {
                                // Une réinstallation : ce qui a changé (une partie dans un disque) d'abord à l'abri.
                                if j.installation.is_some() {
                                    self.mettre_a_l_abri(id).await?;
                                }
                                let libre = crate::jeux_pc::place_libre(&recus);
                                let (poses, n) = tokio::task::spawn_blocking(move || crate::decompression::extraire_a_cote(&archives, libre))
                                    .await
                                    .map_err(|_| Erreur::Disque("La décompression s'est arrêtée brutalement.".into()))??;
                                nouveaux = n;
                                extraits = poses
                                    .iter()
                                    .filter_map(|p| p.strip_prefix(&recus).ok())
                                    .map(|r| r.to_string_lossy().replace('\\', "/"))
                                    .collect();
                            }
                            let connus: Vec<String> = noms.iter().chain(&extraits).cloned().collect();
                            if contenus_emulateur(&version) {
                                Some(fichier_au_role_jeu(&version, &noms)?)
                            } else {
                                lancer_de(&version, &connus).or_else(|| fichier_principal(&noms))
                            }
                        }
                    };
                    // Un .m3u : chaque disque qu'il liste doit être là, sinon l'émulateur ne démarrerait pas (et ce
                    // qui vient d'être décompressé est retiré : rien d'orphelin).
                    if let Some(f) = fichier_du_jeu.as_deref().filter(|f| f.to_lowercase().ends_with(".m3u")) {
                        if let Err(e) = crate::jeux_pc::chemin_de_fichier_sur(f).and_then(|r| verifier_m3u(&recus.join(r))) {
                            for p in &nouveaux {
                                let _ = std::fs::remove_file(p);
                            }
                            return Err(e);
                        }
                    }
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
        let i = self.retenir_installation(&j, &dest, p.methode, fichier_du_jeu, p.nature, extraits)?;
        let i = if plusieurs_versions(&j) { self.retenir_versions(&j, i)? } else { i };
        self.reposer_parties_restaurees(id, &i).await;
        // Forme `archive` : le programme que dit Firehouse (relatif au dossier d'extraction, confirmé le 21/10), s'il y est
        // vraiment, devient le lanceur (par `choisir_lanceur` et ses contrôles) ; sinon la personne choisit.
        if let Some(l) = version_archive.as_ref().and_then(|v| lanceur_de_l_archive(v, &dest)) {
            match self.choisir_lanceur(id, l).await {
                Ok(()) => return Ok(self.registre().jeu(id)?.and_then(|j| j.installation).unwrap_or(i)),
                Err(e) => self.journaliser(&format!("jeu {id} : programme de Firehouse non retenu ({e}), la personne choisira")),
            }
        }
        Ok(i)
    }

    /// La version installée, telle que dans la fiche gardée sur le PC (`Null` sans fiche : hors ligne).
    pub(crate) fn version_de(&self, j: &JeuPc) -> Value {
        let fiche = self.fiche_locale(j.id).unwrap_or(Value::Null);
        fiche["versions"]
            .as_array()
            .and_then(|l| l.iter().find(|v| v["telechargement_id"].as_i64() == Some(j.version)))
            .cloned()
            .unwrap_or(Value::Null)
    }

    /// Des parties restaurées attendent ce jeu : elles sont reposées dans son dossier tout neuf (APRÈS le relevé des
    /// fichiers, pour être reconnues comme des parties et resauvegardées). Un échec est noté, sans défaire
    /// l'installation.
    pub(crate) async fn reposer_parties_restaurees(&self, id: i64, i: &Installation) {
        let Ok(s) = self.session().await else { return };
        let anciens: Vec<i64> = s.verrou().fusions().unwrap_or_default().into_iter().filter(|(_, n, _)| *n == id).map(|(a, _, _)| a).collect();
        match self.reposer_parties_du_jeu(&s.profil.id, id, &anciens, Path::new(&i.dossier)) {
            Ok(r) if r.reposes > 0 => self.journaliser(&format!("jeu {id} : {} partie(s) restaurée(s) reposée(s)", r.reposes)),
            Ok(_) => {}
            Err(e) => self.journaliser(&format!("jeu {id} : parties restaurées non reposées : {e:?}")),
        }
        // Lot S5 : ses parties rangées HORS de son dossier (emplacements) qui attendaient son installation, posées sans
        // jamais rien écraser (un conflit attend la personne : « 📥 Reprendre mes parties de Firehouse »).
        let mut ids = anciens.clone();
        ids.extend(self.registre().reunions().unwrap_or_default().into_iter().filter(|(_, n)| *n == id).map(|(a, _)| a));
        ids.push(id);
        let windows = crate::nuage_steam::dossiers_windows();
        if let Err(e) = crate::reprise_parties::bloquant(|| self.poser_parties_pc(&s, Some(&ids), &windows, true)) {
            self.journaliser(&format!("jeu {id} : parties PC restaurées non posées ({})", if matches!(e, crate::erreurs::Erreur::Disque(_)) { "disque" } else { "autre" }));
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
        let i = self.retenir_installation(&j, &d, p.methode, None, p.nature, vec![])?;
        self.reposer_parties_restaurees(id, &i).await;
        Ok(i)
    }

    /// Défait la NOTE d'une installation qui n'a rien écrit (méthode `aucune`, ni lanceur ni fichier donné à un
    /// émulateur) : le jeu redevient « téléchargé, pas installé ». Rien n'est effacé sur le disque : ses fichiers reçus
    /// restent où ils sont ; le relevé des fichiers de cette installation (`manifeste.json`, propre à Frogtend) est
    /// seulement mis de côté (renommé), pour ne jamais faire passer un fichier ajouté plus tard pour une partie.
    /// Refus pour toute autre installation (un installeur, une archive décompressée, un lanceur déjà choisi…).
    pub async fn installation_annuler(&self, id: i64) -> Resultat<()> {
        // Jamais pendant une installation de ce jeu (la même exclusivité qu'`installer_suivi`).
        let _seule = InstallationEnCours::prendre(&self.dossier, id)?;
        if self.utilise(id) {
            return Err(Erreur::Refus("Ce jeu est occupé (partie, téléchargement ou mise à l'abri en cours) : réessaie après.".into()));
        }
        let _occupe = self.occuper(id);
        let j = self.jeu_visible(id).await?;
        if crate::import_local::est_importe(&j) {
            return Err(Erreur::Refus("Ce jeu vient de ton disque : il n'y a pas d'installation à défaire.".into()));
        }
        let i = j.installation.as_ref().ok_or_else(|| Erreur::Refus("Le jeu n'est pas installé.".into()))?;
        if !note_seule(i) {
            return Err(Erreur::Refus(
                "Cette installation a déjà mis en place le jeu (programme choisi, fichiers installés ou décompressés) : elle ne se défait pas ainsi.".into(),
            ));
        }
        let manifeste = self.fichier_manifeste(id);
        // Des parties apparues depuis la note (parties restaurées reposées à l'installation, une partie jouée) : sans
        // la note, elles ne seraient plus reconnues ni sauvegardées. D'abord une COPIE à l'abri (rien n'est déplacé) ;
        // impossible (dossier des abris non réglé…) : refus, rien ne change.
        if manifeste.is_file() {
            let m: Manifeste = std::fs::read(&manifeste).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default();
            let racine = PathBuf::from(&i.dossier);
            let parties = if racine.is_dir() { crate::locale::parties_changees(&j, &racine, &m)? } else { vec![] };
            if !parties.is_empty() {
                let abri = self.mettre_a_l_abri(id).await.map_err(|e| {
                    Erreur::Refus(format!("Des parties ont été posées dans ce jeu, et leur mise à l'abri a échoué ({e}) : l'installation n'est pas défaite."))
                })?;
                self.journaliser(&format!("jeu {id} : {} fichier(s) changé(s) depuis la note mis à l'abri avant de la défaire", abri.fichiers));
            }
        }
        // Le registre d'abord : un échec y laisse tout comme avant (la note et son relevé).
        self.registre().changer_installation(id, None)?;
        if manifeste.is_file() {
            let medias = self.dossier_medias(id);
            let base = format!("manifeste.annule-{}", maintenant());
            let mut de_cote = medias.join(format!("{base}.json"));
            let mut k = 1;
            while de_cote.exists() {
                de_cote = medias.join(format!("{base}-{k}.json"));
                k += 1;
            }
            if let Err(e) = std::fs::rename(&manifeste, &de_cote) {
                self.journaliser(&format!("jeu {id} : relevé de l'installation défaite non mis de côté ({e})"));
            }
        }
        self.journaliser(&format!("jeu {id} : note d'installation défaite (méthode « aucune », sans lanceur) — aucun fichier du jeu touché"));
        Ok(())
    }

    /// L'installation de ce jeu est-elle à REFAIRE (sans rien retélécharger) ? Une note seule (voir `note_seule`) d'un
    /// jeu que la préparation dit maintenant ROM ou image disque : il a été pris pour un jeu PC (MK8 Deluxe, 26/11).
    pub async fn installation_refaisable(&self, id: i64) -> Resultat<bool> {
        let j = self.jeu_visible(id).await?;
        if id <= 0 || j.etat != Etat::Telecharge || crate::import_local::est_importe(&j) || !j.installation.as_ref().is_some_and(note_seule) {
            return Ok(false);
        }
        Ok(self.preparer_installation(id).await.is_ok_and(|p| p.methode == Methode::Aucune && matches!(p.nature, Nature::Rom | Nature::ImageDisque)))
    }

    /// Le nom court MAME de la version installée (contrat 1.13), d'après la fiche gardée sur le PC.
    fn mame_de(&self, j: &JeuPc) -> Option<String> {
        let fiche = self.fiche_locale(j.id)?;
        let v = fiche["versions"].as_array()?.iter().find(|v| v["telechargement_id"].as_i64() == Some(j.version))?.clone();
        v["mame"]["nom"].as_str().filter(|n| crate::jeux_pc::nom_de_fichier_sur(n).is_ok()).map(String::from)
    }

    /// Les versions d'une ligne à plusieurs régions, inscrites dans l'installation (le choix au lancement, ⭐ celle
    /// d'habitude) : le fichier lancé de CHACUNE, pris dans ses fichiers seulement, avec ses étiquettes. Celle de la
    /// ligne d'abord ; la version par défaut (`dossier` + `fichier_du_jeu`) n'est pas changée.
    pub(crate) fn retenir_versions(&self, j: &JeuPc, mut i: Installation) -> Resultat<Installation> {
        let fiche = self.fiche_locale(j.id).unwrap_or(Value::Null);
        let mut numeros = vec![j.version];
        for f in &j.fichiers {
            let v = f.version.unwrap_or(j.version);
            if !numeros.contains(&v) {
                numeros.push(v);
            }
        }
        let versions: Vec<_> = numeros.into_iter().filter_map(|n| self.entree_du_choix(j, n, &fiche)).collect();
        // Une réinstallation garde la version ⭐ choisie avant (si elle est toujours là).
        if let Some(avant) = j.installation.as_ref().and_then(|a| a.fichier_du_jeu.as_ref().map(|f| Path::new(&a.dossier).join(f))) {
            if let Some(v) = versions.iter().find(|v| Path::new(&v.chemin) == avant) {
                let p = Path::new(&v.chemin);
                i.dossier = p.parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_default();
                i.fichier_du_jeu = p.file_name().map(|n| n.to_string_lossy().to_string());
            }
        }
        i.versions = versions;
        self.registre().changer_installation(j.id, Some(&i))?;
        Ok(i)
    }

    /// L'entrée du choix au lancement d'UNE version de la ligne : son fichier lancé (pris dans SES fichiers), ses
    /// étiquettes. `None` : incomplète sur le disque (une version encore en route), ou elle ne démarrerait pas
    /// (chemin douteux, .m3u dont un disque manque : noté au journal).
    fn entree_du_choix(&self, j: &JeuPc, n: i64, fiche: &Value) -> Option<crate::jeux_pc::VersionLocale> {
        let siens: Vec<&crate::jeux_pc::FichierJeu> = j.fichiers.iter().filter(|f| f.version.unwrap_or(j.version) == n).collect();
        let complete = !siens.is_empty()
            && siens.iter().all(|f| crate::jeux_pc::chemin_de_fichier_sur(&f.nom).is_ok_and(|r| std::fs::metadata(Path::new(&j.dossier).join(r)).is_ok_and(|m| m.len() == f.taille)));
        if !complete {
            return None;
        }
        let noms: Vec<String> = siens.iter().map(|f| f.nom.clone()).collect();
        let v = fiche["versions"].as_array().and_then(|l| l.iter().find(|v| v["telechargement_id"].as_i64() == Some(n))).cloned().unwrap_or(Value::Null);
        let fichier = lancer_de(&v, &noms).or_else(|| fichier_principal(&noms))?;
        let chemin = match crate::jeux_pc::chemin_de_fichier_sur(&fichier).map(|r| Path::new(&j.dossier).join(r)) {
            Ok(c) => c,
            Err(e) => {
                self.journaliser(&format!("jeu {} : version {n} écartée du choix ({e})", j.id));
                return None;
            }
        };
        if fichier.to_lowercase().ends_with(".m3u") {
            if let Err(e) = verifier_m3u(&chemin) {
                self.journaliser(&format!("jeu {} : version {n} écartée du choix ({e})", j.id));
                return None;
            }
        }
        let (rang, qualite, libelle, _) = crate::import_local::etiquettes(&fichier);
        let libelle = if libelle.trim().is_empty() { v["nom"].as_str().unwrap_or(&fichier).to_string() } else { libelle };
        Some(crate::jeux_pc::VersionLocale { chemin: chemin.to_string_lossy().into(), libelle, rang, qualite, disques: vec![] })
    }

    /// Une version AJOUTÉE vient d'arriver : son entrée s'ajoute au choix au lancement, sans rien reconstruire (un
    /// fichier décompressé pour jouer et la version ⭐ restent tels quels). Une liste vide (une seule version jusque-là)
    /// reçoit d'abord l'entrée de la version d'habitude, telle qu'elle est lancée aujourd'hui.
    pub(crate) fn ajouter_version_au_choix(&self, j: &JeuPc, n: i64) -> Resultat<()> {
        let Some(mut i) = j.installation.clone() else { return Ok(()) };
        let fiche = self.fiche_locale(j.id).unwrap_or(Value::Null);
        let Some(nouvelle) = self.entree_du_choix(j, n, &fiche) else { return Ok(()) };
        if i.versions.is_empty() {
            if let Some(f) = i.fichier_du_jeu.clone() {
                let (rang, qualite, libelle, _) = crate::import_local::etiquettes(&f);
                let libelle = if libelle.trim().is_empty() { "Version d'origine".to_string() } else { libelle };
                let chemin = Path::new(&i.dossier).join(&f).to_string_lossy().to_string();
                i.versions.push(crate::jeux_pc::VersionLocale { chemin, libelle, rang, qualite, disques: vec![] });
            }
        }
        if !i.versions.iter().any(|v| Path::new(&v.chemin) == Path::new(&nouvelle.chemin)) {
            i.versions.push(nouvelle);
        }
        self.registre().changer_installation(j.id, Some(&i))?;
        Ok(())
    }

    fn retenir_installation(
        &self,
        j: &JeuPc,
        dest: &Path,
        methode: Methode,
        fichier_du_jeu: Option<String>,
        nature: Nature,
        extraits: Vec<String>,
    ) -> Resultat<Installation> {
        // Le relevé des fichiers : ce qui changera ensuite (les parties) sera reconnu. Pas pour une ROM : ses parties
        // sont chez l'émulateur. Sauf des disques décompressés par Frogtend : un émulateur peut écrire dedans.
        if !matches!(nature, Nature::Rom | Nature::ImageDisque) || !extraits.is_empty() {
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
            extraits,
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

    /// La version lancée par défaut d'un jeu (importé, ou de Firehouse en plusieurs régions) : une de ses versions.
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
    /// `carte` : la carte à lire d'un jeu e-Reader (sa version ; aucune : la première).
    /// `avis` (lot C4c-1) : reçoit la voie d'un jeu GOG (game / principale / galaxy / registre) DÈS son lancement, avant
    /// l'attente du jeu ; la même voie est rendue en dernier (`None` hors GOG).
    pub async fn jouer(
        &self,
        id: i64,
        emulateur: Option<(String, String)>,
        version: Option<&str>,
        carte: Option<i64>,
        avis: &(dyn Fn(&'static str) + Sync),
    ) -> Resultat<(u32, Vec<PathBuf>, bool, Option<&'static str>)> {
        let _occupe = self.occuper(id);
        let j = self.jeu_jouable(id).await?;
        let i = installation_de_la_partie(&j, version)?;
        self.pause_mame.store(false, std::sync::atomic::Ordering::SeqCst);
        // Un jeu d'une boutique : lancé PAR elle (mises à jour, sauvegardes dans le nuage, compte), puis suivi.
        if let Some(b) = &i.boutique {
            // GOG (lot C4c-1) : la tâche du `goggame-<id>.info` d'abord, lue hors du fil asynchrone.
            let direct = if b.source == "gog" {
                let (d, cle) = (PathBuf::from(&i.dossier), b.cle.clone());
                Some(tokio::task::spawn_blocking(move || crate::boutiques_pc::tache_jeu_gog(&d, &cle)).await.unwrap_or(Err(crate::boutiques_pc::MotifSecours::Illisible)))
            } else {
                None
            };
            // La base de Galaxy n'est relue que si l'on va au secours (aucune tâche directe).
            let hors_galaxy = match (matches!(direct, Some(Err(_))), self.actif().await) {
                (true, Some(p)) => crate::commandes::jeu_gog_hors_galaxy_relu(&self.dossier.join("profils").join(&p.id), self.dossier.join("travail").join("galaxy-lancer"), &b.cle).await.is_some(),
                _ => false,
            };
            let journal = |m: &str| self.journaliser(&format!("jeu {id} : {m}"));
            let (pid, voie) = lancer_par_la_boutique(b, Path::new(&i.dossier), direct, hors_galaxy, &journal, avis).await?;
            let reponse = self.annoncer(id, "debut").await;
            return Ok((pid, vec![PathBuf::from(&i.dossier)], reponse.is_some_and(|r| r["session"].is_object()), voie));
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
                // mGBA déjà ouvert (une carte e-Reader lue DANS sa cartouche, ou la cartouche seule) : jamais un second,
                // qui écrirait la même sauvegarde de cartouche que le premier.
                if crate::ereader::est_mgba(&programme) && crate::lancement::programme_ouvert(Path::new(&programme)) {
                    return Err(Erreur::Refus("mGBA est déjà ouvert (une carte ou sa cartouche) : ferme-le d'abord, puis relance.".into()));
                }
                let l = match arcade {
                    // Une carte e-Reader : lue DANS sa cartouche, jamais lancée seule (ce n'est pas une ROM).
                    _ if crate::jeux_pc::a_cartes(&j.plateforme) => {
                        let (cartouche, bandes) = self.preparer_carte(&j, &programme, carte).await?;
                        self.reprendre_partie_mgba(&programme, &cartouche).await;
                        let mut l = lanceur_emulateur(&programme, &ligne, &cartouche)?;
                        crate::ereader::inserer_bandes(&mut l.arguments, &cartouche, &bandes);
                        l
                    }
                    // Vita3K : un jeu déjà installé se lance par son numéro (jamais réinstallé à chaque partie) ; la première
                    // fois, son fichier l'installe, comme avant.
                    None if crate::vita3k::est_vita3k(&programme) => {
                        let jeu = Path::new(&i.dossier).join(f);
                        let mut l = lanceur_emulateur(&programme, &ligne, &jeu)?;
                        let dossier_v = Path::new(&programme).parent().map(Path::to_path_buf).unwrap_or_default();
                        if let Some(t) = crate::vita3k::pret_a_lancer(&dossier_v, &jeu) {
                            crate::vita3k::lancer_par_numero(&mut l.arguments, &jeu, &t);
                        }
                        l
                    }
                    None if crate::ereader::est_mgba(&programme) => {
                        let rom = Path::new(&i.dossier).join(f);
                        self.reprendre_partie_mgba(&programme, &rom).await;
                        lanceur_emulateur(&programme, &ligne, &rom)?
                    }
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
        Ok((pid, dossiers, carte_cedee, None))
    }

    /// mGBA range désormais les parties de chaque personne dans son profil (`-C savegamePath`) : une partie qu'il avait
    /// écrite À CÔTÉ de la ROM (avant Frogtend, ou un mGBA trouvé sur le PC) y est COPIÉE au premier lancement — jamais
    /// déplacée, jamais par-dessus une partie du profil. Un échec n'empêche pas de jouer : il est noté.
    async fn reprendre_partie_mgba(&self, programme: &str, rom: &Path) {
        let Some(profil) = self.actif().await else { return };
        let Some(emulateur) = Path::new(programme).parent() else { return };
        match crate::emulateurs_profils::reprendre_partie_mgba(rom, emulateur, &profil.nom) {
            Ok(Some(p)) => self.journaliser(&format!("mGBA : partie d'à côté de la ROM copiée dans le profil ({})", p.display())),
            Ok(None) => {}
            Err(e) => self.journaliser(&format!("mGBA : partie d'à côté de la ROM non copiée ({e:?})")),
        }
    }

    /// Une carte e-Reader prête à lire : la cartouche qui la lit (un jeu Game Boy Advance de CE PC) et ses bandes, à
    /// glisser avant elle. Refusé sans rien lancer : autre émulateur que mGBA, carte absente ou incomplète, cartouche
    /// absente (l'interface propose alors de la mettre sur le PC) ou encore en route.
    async fn preparer_carte(&self, j: &JeuPc, programme: &str, carte: Option<i64>) -> Resultat<(PathBuf, Vec<PathBuf>)> {
        if !crate::ereader::est_mgba(programme) {
            return Err(Erreur::Prevenir(
                "Une carte e-Reader se lit avec l'émulateur mGBA (version 0.10 ou plus récente), pas avec celui qui est réglé. \
                 Choisis mGBA pour « e-Reader » dans « Réglages » ▸ « Émulateurs »."
                    .into(),
            ));
        }
        let version = carte.unwrap_or(j.version);
        let f = j
            .fichiers
            .iter()
            .find(|f| f.version.unwrap_or(j.version) == version)
            .ok_or_else(|| Erreur::Introuvable("Cette carte n'est pas sur ce PC.".into()))?;
        let archive = Path::new(&j.dossier).join(crate::jeux_pc::chemin_de_fichier_sur(&f.nom)?);
        if !std::fs::metadata(&archive).is_ok_and(|m| m.len() == f.taille) {
            return Err(Erreur::Refus("Cette carte n'est pas encore entièrement sur ce PC.".into()));
        }
        let cartouches = self.cartouches_ereader().await;
        let japonaise = crate::ereader::japonaise(&f.nom);
        let c = crate::ereader::cartouche_pour(&cartouches, japonaise).ok_or_else(|| {
            Erreur::Prevenir(if cartouches.is_empty() {
                "Frogtend ne sait pas encore quelle cartouche e-Reader lit cette carte : connecte-toi une fois à Firehouse, puis réessaie.".into()
            } else {
                "Firehouse ne propose aucune cartouche e-Reader qui lise cette carte.".into()
            })
        })?;
        // La cartouche : un jeu de CE PC que ce profil peut lancer, entièrement là.
        let absente = || Erreur::CartoucheAbsente {
            motif: format!("Pour lire cette carte, il faut la cartouche « {} » (Game Boy Advance) sur ce PC.", c.nom),
            cartouche: c.media_id,
        };
        // Sa ligne sur CE PC : la fiche (réunie depuis à une autre ? contrat 1.14), sinon un jeu importé du disque rattaché
        // à elle (contrat 1.21) — le même rapprochement que pour « Mettre dans ma ludothèque ».
        let ligne_cartouche = {
            let s = self.session().await?;
            let cache = s.verrou();
            let id = cache.reuni_vers(c.media_id)?.unwrap_or(c.media_id);
            if self.registre().jeu(id)?.is_some() {
                Some(id)
            } else {
                self.registre()
                    .correspondances()?
                    .into_iter()
                    .find(|(_, media, _)| *media == id || cache.reuni_vers(*media).ok().flatten() == Some(id))
                    .map(|(local, _, _)| local)
            }
        };
        let jc = match ligne_cartouche {
            Some(idc) => self.jeu_jouable(idc).await.map_err(|_| absente())?,
            None => return Err(absente()),
        };
        if jc.etat != crate::jeux_pc::Etat::Telecharge {
            return Err(Erreur::Prevenir(format!(
                "La cartouche « {} » n'est pas encore entièrement sur ce PC (elle arrive, ou son téléchargement est arrêté) : \
                 regarde dans les téléchargements, puis réessaie quand elle sera là.",
                c.nom
            )));
        }
        // Installée : son fichier lancé ; seulement téléchargée (une ROM n'a rien à installer) : son fichier reçu.
        let installe = installation_de_la_partie(&jc, None).ok().and_then(|i| i.fichier_du_jeu.map(|f| Path::new(&i.dossier).join(f)));
        let recu = || jc.fichiers.first().and_then(|f| crate::jeux_pc::chemin_de_fichier_sur(&f.nom).ok()).map(|r| Path::new(&jc.dossier).join(r));
        let fichier_cartouche = installe.filter(|p| p.is_file()).or_else(|| recu().filter(|p| p.is_file())).ok_or_else(absente)?;
        let dossier = crate::ereader::dossier_des_bandes(&crate::locale::dossier_frogtend(j), version);
        let jeu = PathBuf::from(&j.dossier);
        let bandes = tokio::task::spawn_blocking(move || crate::ereader::bandes(&archive, &jeu, &dossier))
            .await
            .map_err(|_| Erreur::Disque("La lecture de la carte s'est arrêtée brutalement.".into()))??;
        self.journaliser(&format!("jeu {} : carte {version} lue dans la cartouche {} ({} bande(s))", j.id, jc.id, bandes.len()));
        Ok((fichier_cartouche, bandes))
    }

    /// Les cartouches e-Reader du conseil de Firehouse, gardées dans le profil pour jouer hors ligne. Firehouse est
    /// relu à chaque partie (sa liste fait foi), mais jamais plus de 5 s : au-delà, la liste gardée sert. Une réponse
    /// sans cartouche n'efface jamais la liste gardée.
    async fn cartouches_ereader(&self) -> Vec<crate::ereader::Cartouche> {
        let Ok(s) = self.session().await else { return vec![] };
        let garde = s.dossier.join(crate::ereader::FICHIER_CARTOUCHES);
        let gardee = || std::fs::read(&garde).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default();
        match tokio::time::timeout(Duration::from_secs(5), s.source.emulateurs(crate::ereader::PLATEFORME)).await {
            Ok(Ok(v)) if !crate::ereader::cartouches(&v).is_empty() => {
                let l = crate::ereader::cartouches(&v);
                // Jamais écrit pour une session remplacée pendant la lecture (un autre profil).
                if !s.perimee.load(std::sync::atomic::Ordering::SeqCst) {
                    let _ = std::fs::write(&garde, serde_json::to_vec_pretty(&l).unwrap_or_default());
                }
                l
            }
            _ => gardee(),
        }
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
                // Ce qui garde la partie en cours, au journal (pour comprendre une partie qui ne finit pas).
                self.journaliser(&format!("partie du jeu {id} toujours en cours ({} s) : {}", debut.elapsed().as_secs(), suivi.vivants().join(", ")));
                self.annoncer(id, "vivant").await;
                dernier_vivant = Instant::now();
            }
            tokio::time::sleep(INTERVALLE_SUIVI).await;
        }
        let secondes = debut.elapsed().as_secs();
        // Des repères au journal (Seb 18/11 : une partie restée « en cours » après la fermeture d'Eden, sans trace).
        self.journaliser(&format!("partie du jeu {id} : plus aucun programme de la partie ({secondes} s)"));
        // L'annonce de fin ne retient jamais la fin de la partie plus de 90 s (Firehouse lent ou injoignable : la
        // session expire seule au bout de 30 min).
        if tokio::time::timeout(Duration::from_secs(90), self.annoncer(id, "fin")).await.is_err() {
            self.journaliser(&format!("session « fin » du jeu {id} : pas de réponse en 90 s, la partie se termine quand même"));
        }
        self.registre().compter_partie(id, secondes, &maintenant())?;
        self.journaliser(&format!("partie du jeu {id} terminée et comptée"));
        Ok(FinDePartie { jeu: id, secondes })
    }

    // --- « ▶ Ouvrir l'émulateur seul » (✅ Seb 17/11) : un émulateur ouvert SANS jeu. Suivi comme une partie (session
    // Firehouse, aucune partie lancée pendant ce temps), mais SANS jeu : pas de `media_id`, et le temps passé n'est
    // compté à aucun jeu. ---

    /// La clé d'un dossier d'émulateur (sans casse, « \ », sans barre finale).
    fn cle_emulateur_seul(dossier: &Path) -> String {
        dossier.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_lowercase()
    }

    /// Une partie commence à se lancer (`jeu_jouer`, de son début à son retour : la préparation peut être longue) :
    /// refusé si un émulateur est ouvert seul ; tant que la garde vit, aucun émulateur ne s'ouvre seul. La vérification
    /// et la marque se font sous le MÊME verrou que `marquer_emulateur_seul` : jamais les deux à la fois.
    pub(crate) fn commencer_lancement(&self) -> Resultat<LancementEnCours<'_>> {
        let l = self.emulateurs_seuls.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((_, nom)) = l.first() {
            return Err(Erreur::Refus(format!("{nom} est ouvert seul : ferme-le d'abord, puis lance le jeu.")));
        }
        // Lot S3 (M13) : sous le MÊME verrou que le changement de profil — jamais un lancement qui se glisse pendant
        // qu'un profil s'ouvre ou se ferme.
        let changements = PROFILS_EN_CHANGEMENT.lock().unwrap_or_else(|e| e.into_inner());
        if changements.contains(&self.dossier) {
            return Err(Erreur::Refus("Le profil est en train de changer : réessaie dans un instant.".into()));
        }
        self.lancements.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        drop(changements);
        Ok(LancementEnCours(self))
    }

    /// Note l'émulateur de `dossier` (`nom`) ouvert seul. Refusé pendant une partie (`partie_en_cours`) ou un lancement
    /// en préparation, et quand un émulateur est déjà ouvert seul (un à la fois).
    pub(crate) fn marquer_emulateur_seul(&self, dossier: &Path, nom: &str, partie_en_cours: bool) -> Resultat<()> {
        let c = Self::cle_emulateur_seul(dossier);
        let mut l = self.emulateurs_seuls.lock().unwrap_or_else(|e| e.into_inner());
        if partie_en_cours || self.lancements.load(std::sync::atomic::Ordering::SeqCst) > 0 {
            return Err(Erreur::Refus("Une partie est en cours ou se lance : quitte le jeu, puis réessaie.".into()));
        }
        if l.iter().any(|(x, _)| *x == c) {
            return Err(Erreur::Refus(format!("{nom} est déjà ouvert seul.")));
        }
        if let Some((_, autre)) = l.first() {
            return Err(Erreur::Refus(format!("{autre} est déjà ouvert seul : ferme-le d'abord.")));
        }
        l.push((c, nom.to_string()));
        Ok(())
    }

    /// Ouvre un émulateur seul sous son verrou : `lancer` (préparation puis démarrage) rend le numéro du processus ; un
    /// échec rend le verrou aussitôt (jamais un émulateur « ouvert » pour rien).
    pub(crate) async fn ouvrir_sous_verrou<F, Fut>(&self, dossier: &Path, nom: &str, partie_en_cours: bool, lancer: F) -> Resultat<u32>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Resultat<u32>>,
    {
        self.marquer_emulateur_seul(dossier, nom, partie_en_cours)?;
        match lancer().await {
            Ok(pid) => Ok(pid),
            Err(e) => {
                self.liberer_emulateur_seul(dossier);
                Err(e)
            }
        }
    }

    /// L'émulateur de `dossier` est fermé.
    pub(crate) fn liberer_emulateur_seul(&self, dossier: &Path) {
        let c = Self::cle_emulateur_seul(dossier);
        self.emulateurs_seuls.lock().unwrap_or_else(|e| e.into_inner()).retain(|(x, _)| *x != c);
    }

    /// Une partie se prépare (`commencer_lancement`) : ses réglages d'émulateur s'écrivent peut-être en ce moment.
    pub(crate) fn lancement_en_preparation(&self) -> bool {
        self.lancements.load(std::sync::atomic::Ordering::SeqCst) > 0
    }

    /// Le nom de l'émulateur ouvert seul, s'il y en a un (l'interface le relit après un rechargement).
    pub(crate) fn emulateur_seul_nom(&self) -> Option<String> {
        self.emulateurs_seuls.lock().unwrap_or_else(|e| e.into_inner()).first().map(|(_, n)| n.clone())
    }

    /// Refusé pendant qu'un émulateur est ouvert seul (`quoi` : ce qui attend, « changer de profil »…) : il tourne avec
    /// le profil de la personne et réécrira ses réglages en quittant.
    pub(crate) fn refuser_si_emulateur_seul(&self, quoi: &str) -> Resultat<()> {
        match self.emulateur_seul_nom() {
            Some(nom) => Err(Erreur::Refus(format!("{nom} est ouvert avec ce profil : ferme-le d'abord, puis {quoi}."))),
            None => Ok(()),
        }
    }

    /// Le changement de profil (ouvrir, fermer) : refusé pendant qu'un jeu tourne ou se lance (Seb, 23/11 : comme pour
    /// l'émulateur ouvert seul) — `partie_en_cours` : le menu en jeu suit une partie. Ses parties appartiennent au profil
    /// qui joue : changer de profil sous lui les rangerait chez un autre. Tant que la garde vit, aucun lancement ne
    /// commence (M13 : vérifié et noté sous le même verrou que `commencer_lancement`).
    pub(crate) fn changer_de_profil(&self, partie_en_cours: bool, quoi: &str) -> Resultat<ChangementDeProfil<'_>> {
        let mut changements = PROFILS_EN_CHANGEMENT.lock().unwrap_or_else(|e| e.into_inner());
        if partie_en_cours || self.lancement_en_preparation() {
            return Err(Erreur::Refus(format!("Un jeu est en cours avec ce profil : quitte-le d'abord, puis {quoi}.")));
        }
        if !changements.insert(self.dossier.clone()) {
            return Err(Erreur::Refus("Le profil est déjà en train de changer : réessaie dans un instant.".into()));
        }
        Ok(ChangementDeProfil(self))
    }

    /// Ce programme est-il dans le dossier d'un émulateur ouvert seul (ses réglages ne s'écrivent pas) ?
    pub(crate) fn emulateur_seul_dans(&self, programme: &Path) -> bool {
        let p = Self::cle_emulateur_seul(programme);
        self.emulateurs_seuls.lock().unwrap_or_else(|e| e.into_inner()).iter().any(|(c, _)| p == *c || p.starts_with(&format!("{c}\\")))
    }

    /// Annonce une étape de session SANS jeu à Firehouse (la route accepte `media_id` absent). Un échec n'empêche
    /// jamais d'ouvrir l'émulateur : il est noté, sans nouvel essai.
    pub(crate) async fn annoncer_sans_jeu(&self, etat: &str) -> Option<Value> {
        let s = self.session().await.ok()?;
        match s.source.session(etat, None).await {
            Ok(v) => Some(v),
            Err(e) => {
                self.journaliser(&format!("session « {etat} » (émulateur ouvert seul) : {e:?}"));
                None
            }
        }
    }

    /// Suit un émulateur ouvert seul jusqu'à sa fermeture : « vivant » toutes les 10 min, « fin » à la sortie, SANS
    /// compter le temps à aucun jeu. Rend la durée (secondes).
    pub async fn suivre_emulateur_seul(&self, pid: u32, dossier: PathBuf) -> Resultat<u64> {
        let debut = Instant::now();
        let mut dernier_vivant = Instant::now();
        let mut suivi = Suivi::nouveau(pid, vec![dossier]);
        loop {
            let (s, en_cours) = tokio::task::spawn_blocking(move || {
                let r = suivi.en_cours();
                (suivi, r)
            })
            .await
            .map_err(|_| Erreur::Disque("Le suivi de l'émulateur s'est arrêté.".into()))?;
            suivi = s;
            if !en_cours {
                break;
            }
            if dernier_vivant.elapsed() >= INTERVALLE_VIVANT {
                self.annoncer_sans_jeu("vivant").await;
                dernier_vivant = Instant::now();
            }
            tokio::time::sleep(INTERVALLE_SUIVI).await;
        }
        self.annoncer_sans_jeu("fin").await;
        Ok(debut.elapsed().as_secs())
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
        let du_jeu = self.dossier_abris()?.join(id.to_string());
        // Lot S3 : un abri de ce jeu a déjà pris cette seconde (l'abri des parties juste avant, un essai répété) : la
        // seconde suivante libre (jamais un renommage sur un dossier existant, erreur 145).
        let date = date_libre(&du_jeu, &maintenant());
        let abri = du_jeu.join(&date);
        // Copié à côté puis renommé : un abri à moitié fait n'a jamais le nom d'un abri (lot 3, avis lancement 30/10).
        let provisoire = du_jeu.join(format!("{date}.en-cours"));
        let octets = copier(&racine, &changes, &provisoire)?;
        // Vérifier le résultat : chaque fichier est bien dans l'abri, à la même taille.
        for r in &changes {
            let a = std::fs::metadata(provisoire.join(r)).map(|m| m.len()).ok();
            let b = std::fs::metadata(racine.join(r)).map(|m| m.len()).ok();
            if a != b {
                return Err(Erreur::Disque(format!("La copie de « {r} » a échoué : le jeu n'est pas retiré.")));
            }
        }
        // Lot S2 : la fiche dit aussi le profil propriétaire (celui de la session) et chaque fichier (taille, SHA1) ;
        // `version` (la version du jeu) reste lue comme avant par la remise des parties.
        let mut faits = Vec::new();
        for r in &changes {
            let c = provisoire.join(r);
            let taille = std::fs::metadata(&c).map(|m| m.len()).unwrap_or(0);
            faits.push(crate::nuage_steam::AMettre { source: c.clone(), relatif: r.clone(), taille, sha: crate::nuage_steam::sha1(&c)? });
        }
        std::fs::rename(&provisoire, &abri)?;
        // La fiche de l'abri, À CÔTÉ (jamais dedans : elle partirait comme une partie).
        let profil = self.actif().await.map(|p| p.id).unwrap_or_default();
        let mut fiche = crate::abri_parties::FicheAbri::nouvelle(crate::abri_parties::SORTE_JEU, &profil, vec![], &faits, 0, 0);
        fiche.version = Some(j.version);
        if let Err(e) = std::fs::write(du_jeu.join(format!("{date}.json")), serde_json::to_vec_pretty(&fiche).unwrap_or_default()) {
            // Sans fiche, seuls les fichiers absents reviendront un jour : noté.
            self.journaliser(&format!("jeu {id} : fiche de l'abri non écrite ({e})"));
        }
        Ok(Abri { dossier: abri.to_string_lossy().into(), fichiers: changes.len(), octets, reste_installe: None })
    }

    /// Ce qu'il y a à remettre d'un abri dans ce jeu réinstallé (lot 3, ✅ Seb 30/10) : le plus récent de chaque fichier,
    /// sur tous les abris de ce jeu (et de ses anciens numéros), hors du nuage Steam. Par fichier : absent du jeu → posé ;
    /// inchangé depuis l'installation et même version que l'abri → remplacé ; joué depuis (ou autre version) → jamais
    /// touché. `None` : rien à remettre, ou déjà proposé pour cet abri.
    pub async fn abri_a_reposer(&self, id: i64) -> Resultat<Option<AbriAReposer>> {
        if id <= 0 {
            return Ok(None); // un jeu importé n'est jamais retiré par Frogtend : pas d'abri de retrait
        }
        let j = match self.jeu_visible(id).await {
            Ok(j) => j,
            Err(Erreur::Introuvable(_)) => return Ok(None),
            Err(e) => return Err(e),
        };
        let Some(i) = j.installation.clone().filter(|i| i.boutique.is_none()) else { return Ok(None) };
        let sources = self.sources_abri(id)?;
        // Rapide (chaque ▶ Jouer) : seulement les NOMS des abris ; déjà répondu pour le plus récent → rien d'autre lu.
        let Some(plus_recent) = sources.iter().flat_map(|d| dates_d_abri(d)).max() else { return Ok(None) };
        let proposes: serde_json::Value = std::fs::read(self.dossier.join("abris-proposes.json")).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default();
        if proposes[id.to_string()].as_str().is_some_and(|d| d >= plus_recent.as_str()) {
            return Ok(None);
        }
        let manifeste = self.fichier_manifeste(id);
        tauri::async_runtime::spawn_blocking(move || plan_abri(&j, &i, &sources, &manifeste))
            .await
            .map_err(|_| Erreur::Disque("La lecture des abris s'est arrêtée brutalement.".into()))?
    }

    /// Les dossiers d'abris de ce jeu (et de ses anciens numéros), dans tous les dossiers d'abris à lire.
    pub(crate) fn sources_abri(&self, id: i64) -> Resultat<Vec<PathBuf>> {
        let mut numeros: Vec<i64> = self.registre().reunions()?.into_iter().filter(|(_, n)| *n == id).map(|(a, _)| a).collect();
        numeros.push(id);
        Ok(self.dossiers_abris_a_lire().iter().flat_map(|d| numeros.iter().map(move |n| d.join(n.to_string()))).collect())
    }

    /// Remet les parties à l'abri dans le jeu (après le oui de la personne). Jeu en cours ou occupé : refus. Un fichier
    /// remplacé est d'abord gardé dans le dossier Frogtend du jeu (`avant-abri/<date>/`) ; chaque copie est écrite à
    /// côté, vérifiée (taille), puis renommée ; l'abri reste. Rend le nombre de fichiers remis.
    pub async fn reposer_abri(&self, id: i64) -> Resultat<usize> {
        if self.utilise(id) {
            return Err(Erreur::Refus("Ce jeu est occupé (partie, téléchargement ou mise à l'abri en cours) : réessaie après.".into()));
        }
        let _occupe = self.occuper(id);
        let j = self.jeu_visible(id).await?;
        let i = j.installation.clone().filter(|i| i.boutique.is_none()).ok_or_else(|| Erreur::Refus("Le jeu n'est pas installé par Frogtend.".into()))?;
        if !crate::lancement::processus_du_dossier(Path::new(&i.dossier)).is_empty() {
            return Err(Erreur::Refus("Le jeu est ouvert : ferme-le d'abord.".into()));
        }
        let sources = self.sources_abri(id)?;
        let manifeste = self.fichier_manifeste(id);
        let j2 = j.clone();
        let (p, remis) = tauri::async_runtime::spawn_blocking(move || -> Resultat<(Option<AbriAReposer>, usize)> {
            let Some(p) = plan_abri(&j2, &i, &sources, &manifeste)? else { return Ok((None, 0)) };
            let racine = PathBuf::from(&i.dossier);
            if crate::jeux_pc::place_libre(&racine).is_some_and(|l| l < p.octets.saturating_mul(2)) {
                return Err(Erreur::Refus("Pas assez de place sur le disque du jeu pour remettre tes parties.".into()));
            }
            let garde = crate::locale::dossier_frogtend(&j2).join("avant-abri").join(maintenant());
            for (r, source) in &p.plan {
                let cible = racine.join(r);
                if let Some(d) = cible.parent() {
                    std::fs::create_dir_all(d)?;
                }
                if cible.exists() {
                    let copie = garde.join(r);
                    if let Some(d) = copie.parent() {
                        std::fs::create_dir_all(d)?;
                    }
                    std::fs::copy(&cible, &copie)?;
                }
                let provisoire = PathBuf::from(format!("{}.frogtend-tmp", cible.to_string_lossy()));
                std::fs::copy(source, &provisoire)?;
                if std::fs::metadata(&provisoire).map(|m| m.len()).ok() != std::fs::metadata(source).map(|m| m.len()).ok() {
                    let _ = std::fs::remove_file(&provisoire);
                    return Err(Erreur::Disque(format!("La copie de « {r} » a échoué.")));
                }
                std::fs::rename(&provisoire, &cible)?;
            }
            let n = p.plan.len();
            Ok((Some(p), n))
        })
        .await
        .map_err(|_| Erreur::Disque("La remise des parties s'est arrêtée brutalement.".into()))??;
        if let Some(p) = p {
            self.noter_abri_propose(id, &p.date);
            self.journaliser(&format!("jeu {id} : {remis} fichier(s) remis depuis l'abri du {}", p.date));
        }
        Ok(remis)
    }

    /// La personne a répondu (oui ou non) pour cet abri : la question ne revient pas, sauf pour un abri plus récent.
    pub fn noter_abri_propose(&self, id: i64, date: &str) {
        let f = self.dossier.join("abris-proposes.json");
        let mut v: serde_json::Value = std::fs::read(&f).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_else(|| serde_json::json!({}));
        if !v.is_object() {
            v = serde_json::json!({});
        }
        v[id.to_string()] = serde_json::Value::String(date.to_string());
        let _ = std::fs::write(&f, serde_json::to_vec_pretty(&v).unwrap_or_default());
    }

    /// Retire un jeu complet du PC : ses parties d'abord à l'abri (sinon rien n'est fait), puis son désinstalleur
    /// s'il en a un, puis ce que Frogtend a mis sur le disque (installation, fichiers reçus, médias). Les abris
    /// restent.
    pub async fn retirer_du_pc(&self, id: i64) -> Resultat<Abri> {
        /// Un retrait de mods de Vortex dans le dossier d'installation ou le dossier reçu du jeu ?
        fn retraits_de_vortex(j: &crate::jeux_pc::JeuPc) -> bool {
            [j.installation.as_ref().map(|i| PathBuf::from(&i.dossier)), Some(PathBuf::from(&j.dossier))]
                .into_iter()
                .flatten()
                .any(|d| !crate::mods_vortex::retraits(&d).is_empty())
        }
        let _occupe = self.occuper(id);
        let j = self.jeu_visible(id).await?;
        if !matches!(j.etat, Etat::Telecharge) {
            return Err(Erreur::Refus("Ce jeu est en cours de téléchargement : annule plutôt le téléchargement.".into()));
        }
        // Un jeu d'une boutique : c'est elle qui le désinstalle ; Frogtend ne retire pas sa ligne (son temps de jeu).
        if j.installation.as_ref().is_some_and(|i| i.boutique.is_some()) {
            return Err(Erreur::Refus("Ce jeu est installé par sa boutique : désinstalle-le depuis elle.".into()));
        }
        // 0.227.1 : les mods de Vortex retirés par Frogtend sont rangés DANS le dossier du jeu (souvent leur seule copie,
        // Vortex désinstallé) : jamais effacés avec le jeu. Refusé avant tout le reste (rien n'a changé).
        if !crate::import_local::est_importe(&j) && retraits_de_vortex(&j) {
            return Err(Erreur::Refus(
                "Ce jeu garde dans son dossier des mods de Vortex retirés par Frogtend, peut-être leur seule copie : Frogtend ne l'efface pas. Remets ou garde ces mods d'abord.".into(),
            ));
        }
        // Lot S3 (Seb, 23/11) : les parties rangées HORS du dossier du jeu, à l'abri AVANT tout le reste (arrêt des
        // versions en route, désinstalleur qui peut les effacer) ; un échec arrête le retrait sans RIEN avoir changé,
        // sauf « continuer sans abri » choisi par la personne. Un jeu importé n'est jamais effacé : pas d'abri.
        if !crate::import_local::est_importe(&j) {
            crate::commandes::abri_avant(self, id, crate::abri_parties::DECLENCHEUR_AVANT_RETRAIT, true).await?;
        }
        // Une autre version de ce jeu en route : arrêtée d'abord (la file ne la reprend plus) ; ses `.part` partent
        // avec le reste.
        self.arreter_versions(id).await?;
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
            // Le reste d'une version qui arrivait (jamais un fichier hors de la liste du jeu).
            let part = PathBuf::from(format!("{}.part", p.to_string_lossy()));
            if part.is_file() {
                let _ = std::fs::remove_file(&part);
            }
            // Un sous-dossier reçu (CHD MAME), seulement s'il est vide.
            if rel.components().count() > 1 {
                if let Some(d) = p.parent() {
                    let _ = std::fs::remove_dir(d);
                }
            }
        }
        // Les disques décompressés par Frogtend (leurs changements sont déjà à l'abri), seulement DANS le dossier du
        // jeu et sans lien entre les deux.
        for e in j.installation.as_ref().map(|i| i.extraits.as_slice()).unwrap_or_default() {
            let Some(rel) = crate::decompression::chemin_sur(e) else { continue };
            let p = recus.join(&rel);
            if p.is_file() && crate::garde::sous(&p, &recus) && !crate::garde::lien_entre(&recus, &p) {
                std::fs::remove_file(&p)?;
            }
        }
        self.retirer_medias(&j)?;
        // `Jeu/` resté vide (un jeu en archive installé dans `Jeu/<extraire_dans>`), puis le dossier du jeu.
        let _ = std::fs::remove_dir(recus.join(DOSSIER_INSTALLATION)); // seulement s'il est vide
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

    #[test]
    fn prete_a_l_arrivee_seulement_quand_rien_n_est_a_faire() {
        let v = |j: Value| prete_a_l_arrivee(&j);
        assert!(v(serde_json::json!({"qualite": "Jeu — ROM"})), "une ROM");
        assert!(v(serde_json::json!({"qualite": "Image disque"})), "une image disque");
        assert!(v(serde_json::json!({"qualite": "Jeu — ROM", "mame": {"nom": "sf2"}})), "MAME : le zip tel quel");
        assert!(!v(Value::Null), "sans fiche : on ne devine pas");
        assert!(!v(serde_json::json!({"qualite": "Jeu — prêt à jouer"})), "un programme : il faut choisir le lanceur");
        assert!(!v(serde_json::json!({"qualite": "Repack"})));
        assert!(!v(serde_json::json!({"qualite": "Jeu — ROM", "installation": {"forme": "archive"}})), "forme archive");
        let zippes = serde_json::json!({"qualite": "Image disque", "fichiers": [{"nom": "a.zip", "role": "archive_a_extraire"}]});
        assert!(!v(zippes), "des disques zippés : du vrai travail, avec l'accord");
        let switch = serde_json::json!({"qualite": "", "installation": {"forme": "contenus_emulateur"}, "fichiers": [{"nom": "j.nsp", "role": "jeu"}, {"nom": "maj.nsp", "role": "mise_a_jour"}]});
        assert_eq!(v(switch.clone()), contenus_emulateur(&switch), "un jeu Switch avec ses contenus : prêt (Eden ne les lit qu'après un oui)");
    }

    #[test]
    fn une_seule_installation_a_la_fois_pour_un_jeu() {
        let d = Path::new("donnees-essai-exclusivite");
        let a = InstallationEnCours::prendre(d, 424242).unwrap();
        assert!(InstallationEnCours::prendre(d, 424242).is_err(), "la 2e est refusée");
        assert!(InstallationEnCours::prendre(d, 424243).is_ok(), "un autre jeu : permis");
        assert!(InstallationEnCours::prendre(Path::new("autres-donnees"), 424242).is_ok(), "d'autres données : permis");
        drop(a);
        assert!(InstallationEnCours::prendre(d, 424242).is_ok(), "rendue à la fin");
    }

    /// Un noyau avec un profil (simulé) qui voit Dune (110), et Dune « téléchargé » sur le PC avec les fichiers donnés.
    async fn noyau_avec(fichiers: &[(&str, &[u8])], qualite: &str) -> (tempfile::TempDir, Noyau) {
        noyau_avec_sur("MS-DOS", fichiers, qualite).await
    }

    /// Comme `noyau_avec`, le jeu rangé sous la plateforme donnée.
    async fn noyau_avec_sur(plateforme: &str, fichiers: &[(&str, &[u8])], qualite: &str) -> (tempfile::TempDir, Noyau) {
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, None).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();

        let dossier = d.path().join("Jeux").join(plateforme).join("Dune (1992)");
        std::fs::create_dir_all(&dossier).unwrap();
        let mut fj = Vec::new();
        for (i, (nom, o)) in fichiers.iter().enumerate() {
            std::fs::create_dir_all(dossier.join(nom).parent().unwrap()).unwrap();
            std::fs::write(dossier.join(nom), o).unwrap();
            fj.push(FichierJeu { n: i as u32, nom: nom.to_string(), taille: o.len() as u64, ..Default::default() });
        }
        let total = fj.iter().map(|f| f.taille).sum();
        n.registre()
            .ajouter(&JeuPc {
                id: 110,
                version: 200,
                titre: "Dune".into(),
                plateforme: plateforme.into(),
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

    /// La forme du relevé réel 1.23 (Xak, PC-8801), en deux disques : un .m3u des .d88, un zip TOSEC par disque.
    async fn multidisques_zippe(m3u: &[u8]) -> (tempfile::TempDir, Noyau) {
        let (z1, z2) = (zip(&[("Xak (Disk 1 of 2).d88", &[1u8; 10])]), zip(&[("Xak (Disk 2 of 2).d88", &[2u8; 20])]));
        let (d, n) = noyau_avec(&[("Xak.m3u", m3u), ("Xak (Disk 1 of 2).zip", &z1), ("Xak (Disk 2 of 2).zip", &z2)], "Jeu — nature non dite").await;
        let fiche = serde_json::json!({"id": 110, "titre": "Xak", "versions": [
            {"telechargement_id": 200, "qualite": "Jeu — nature non dite", "notes": "", "fichiers": [
                {"nom": "Xak.m3u", "taille": m3u.len(), "dossier": "", "role": "liste_disques"},
                {"nom": "Xak (Disk 1 of 2).zip", "taille": z1.len(), "dossier": "", "role": "archive_a_extraire"},
                {"nom": "Xak (Disk 2 of 2).zip", "taille": z2.len(), "dossier": "", "role": "archive_a_extraire"}],
             "installation": {"forme": "fichier", "extraire_dans": "", "lancer": "Xak.m3u", "arguments": ""}}]});
        std::fs::write(n.dossier_medias(110).join("fiche.json"), fiche.to_string()).unwrap();
        (d, n)
    }

    #[tokio::test]
    async fn un_multidisques_zippe_se_decompresse_a_cote_et_se_lance_par_son_m3u() {
        let (d, n) = multidisques_zippe(b"\xef\xbb\xbfXak (Disk 1 of 2).d88\r\n# commentaire\r\nXak (Disk 2 of 2).d88|Disque 2\r\n").await;
        n.regler_abris(Some(d.path().join("Abris")));
        let p = n.preparer_installation(110).await.unwrap();
        assert_eq!((p.nature, p.methode), (Nature::ImageDisque, Methode::Aucune), "un multi-disques, jamais « une archive »");
        let i = n.installer(110, true).await.unwrap();
        assert_eq!(i.fichier_du_jeu.as_deref(), Some("Xak.m3u"));
        assert_eq!(i.extraits, vec!["Xak (Disk 1 of 2).d88", "Xak (Disk 2 of 2).d88"]);
        let dossier = PathBuf::from(&i.dossier);
        assert!(dossier.join("Xak (Disk 2 of 2).d88").is_file() && dossier.join("Xak (Disk 1 of 2).zip").is_file(), "disques posés, zips gardés");

        // L'émulateur écrit une partie dans un disque : le retrait la met d'abord à l'abri, puis efface tout.
        std::thread::sleep(std::time::Duration::from_millis(1100));
        std::fs::write(dossier.join("Xak (Disk 2 of 2).d88"), [9u8; 20]).unwrap();
        let abri = n.retirer_du_pc(110).await.unwrap();
        assert_eq!(abri.fichiers, 1, "la partie est à l'abri");
        assert_eq!(std::fs::read(PathBuf::from(&abri.dossier).join("Xak (Disk 2 of 2).d88")).unwrap(), vec![9u8; 20]);
        assert!(!dossier.exists(), "disques décompressés et fichiers reçus effacés");
    }

    #[tokio::test]
    async fn un_m3u_qui_nomme_un_disque_absent_ou_hors_du_dossier_n_est_pas_installe() {
        for m3u in [&b"Xak (Disk 1 of 2).d88\nXak (Disk 3 of 2).d88\n"[..], b"..\\..\\Windows\\win.ini\n", b"# rien\n"] {
            let (_d, n) = multidisques_zippe(m3u).await;
            assert!(matches!(n.installer(110, true).await, Err(Erreur::Refus(_))), "{}", String::from_utf8_lossy(m3u));
            let j = n.registre().jeu(110).unwrap().unwrap();
            assert!(j.installation.is_none());
            assert!(!Path::new(&j.dossier).join("Xak (Disk 1 of 2).d88").exists(), "aucun disque orphelin");
        }
    }

    #[tokio::test]
    async fn un_jeu_pc_en_archive_s_extrait_entier_et_se_lance_par_son_programme_meme_dit_rom() {
        // Relevé réel 1.24 (Megaman's Christmas Carol 3) : qualité « Jeu — ROM », forme archive, zip SANS dossier racine
        // (confirmé par Firehouse), avec sous-dossier et dossier vide.
        let mut o = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut o);
            let opt = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored).large_file(true);
            for (nom, c) in [("ChristmasCarol3.exe", &b"MZ"[..]), ("data.win", b"donnees"), ("Music/theme.ogg", b"ogg")] {
                z.start_file(nom, opt).unwrap();
                std::io::Write::write_all(&mut z, c).unwrap();
            }
            z.add_directory("Saves/", opt).unwrap();
            z.finish().unwrap();
        }
        let archive = o.into_inner();
        let (d, n) = noyau_avec(&[("Megaman's Christmas Carol 3.zip", &archive)], "Jeu — ROM").await;
        n.regler_abris(Some(d.path().join("Abris")));
        let r: Value = serde_json::from_str(include_str!("../fixtures/simule/version-archive-pc-124.json")).unwrap();
        let mut v = r["version"].clone();
        v["telechargement_id"] = 200.into();
        v["fichiers"][0]["taille"] = archive.len().into();
        std::fs::write(n.dossier_medias(110).join("fiche.json"), serde_json::json!({"id": 110, "titre": "Megaman", "versions": [v]}).to_string()).unwrap();

        let p = n.preparer_installation(110).await.unwrap();
        assert_eq!(p.nature, Nature::PretAJouer, "la forme fait foi, jamais « ROM »");
        assert!(matches!(p.methode, Methode::Archive { .. }));
        assert!(p.destination.ends_with("Megaman's Christmas Carol 3"));
        let i = n.installer(110, true).await.unwrap();
        let dest = PathBuf::from(&i.dossier);
        assert!(dest.join("Music/theme.ogg").is_file() && dest.join("Saves").is_dir(), "sous-dossier et dossier vide");
        let l = i.lanceur.expect("le programme de Firehouse est retenu");
        assert_eq!(PathBuf::from(&l.programme), dest.join("ChristmasCarol3.exe"));

        // Jamais par-dessus : une seconde extraction dans le même dossier est refusée, rien n'est touché.
        std::fs::write(dest.join("Saves/partie.sav"), b"ma partie").unwrap();
        assert!(crate::decompression::extraire_dans(&PathBuf::from(&n.registre().jeu(110).unwrap().unwrap().dossier).join("Megaman's Christmas Carol 3.zip"), &dest, None, &mut |_, _| {}).is_err());
        assert_eq!(std::fs::read(dest.join("Saves/partie.sav")).unwrap(), b"ma partie");

        // Le retrait ne laisse ni `Jeu/` vide ni dossier du jeu.
        let jeu = PathBuf::from(&n.registre().jeu(110).unwrap().unwrap().dossier);
        n.retirer_du_pc(110).await.unwrap();
        assert!(!jeu.exists());
    }

    #[test]
    fn la_commande_de_galaxy_refuse_un_identifiant_douteux() {
        assert_eq!(commande_galaxy("12;calc", Path::new("C:\\Jeux")), None);
        assert_eq!(commande_galaxy("", Path::new("C:\\Jeux")), None);
        // Lot A : GOG se lance aussi sans Galaxy ; le nom vient de la boutique (Ubisoft, Amazon… aussi).
        assert_eq!(nom_boutique("gog"), "GOG");
        assert_eq!(nom_boutique("uplay"), "Ubisoft Connect");
        assert_eq!(adresse_boutique(&crate::jeux_pc::LancementBoutique { source: "uplay".into(), cle: "46".into() }).as_deref(), Some("uplay://launch/46"));
        assert_eq!(adresse_boutique(&crate::jeux_pc::LancementBoutique { source: "amazon".into(), cle: "x/../y".into() }), None);
        assert_eq!(adresse_boutique(&crate::jeux_pc::LancementBoutique { source: "battlenet".into(), cle: "wow".into() }), None, "Battle.net : rien de deviné");
        // Lot C4c-1 : GOG ne passe plus par une adresse (sa tâche directe, ou le secours).
        assert_eq!(adresse_boutique(&crate::jeux_pc::LancementBoutique { source: "gog".into(), cle: "1423049311".into() }), None);
    }

    #[test]
    fn un_jeu_gog_se_lance_directement_et_galaxy_n_est_qu_un_secours() {
        use crate::boutiques_pc::{CategorieTache, MotifSecours, ProgrammeGog, TacheGog};
        let prog = |e: &str| ProgrammeGog { exe: PathBuf::from(e), dossier_travail: PathBuf::from(r"D:\J"), arguments: String::new() };
        let tache = TacheGog { programme: prog(r"D:\J\bin\x64\Cyberpunk2077.exe"), categorie: CategorieTache::Jeu };
        let galaxy = || Some((PathBuf::from(r"C:\Galaxy\GalaxyClient.exe"), vec!["/command=runGame".to_string()]));
        let registre = || Some(prog(r"D:\J\REDprelauncher.exe"));
        let jamais_g = || -> Option<(PathBuf, Vec<String>)> { panic!("Galaxy ne doit pas être lu") };
        let jamais_r = || -> Option<ProgrammeGog> { panic!("le registre ne doit pas être lu") };
        // 1. La tâche directe d'abord : ni Galaxy ni le registre ne sont lus, même « hors Galaxy ».
        let p = plan_gog(Ok(tache.clone()), false, jamais_g, jamais_r);
        assert_eq!(p, PlanGog::Direct(tache.clone()));
        assert_eq!(p.voie(), ("game", None));
        assert_eq!(plan_gog(Ok(tache.clone()), true, jamais_g, jamais_r), PlanGog::Direct(tache.clone()));
        let principale = TacheGog { categorie: CategorieTache::Principale, ..tache };
        assert_eq!(plan_gog(Ok(principale), false, jamais_g, jamais_r).voie().0, "principale");
        // 2. Sans tâche : GOG Galaxy s'il est là.
        let p = plan_gog(Err(MotifSecours::AucuneTache), false, galaxy, jamais_r);
        assert!(matches!(&p, PlanGog::Galaxy { client, motif: MotifSecours::AucuneTache, .. } if client.ends_with("GalaxyClient.exe")));
        assert_eq!(p.voie(), ("galaxy", Some(MotifSecours::AucuneTache)));
        // 3. Sans Galaxy, ou un jeu d'un autre compte (Galaxy ne le lancerait pas) : le programme du registre.
        let p = plan_gog(Err(MotifSecours::SansFichier), false, || None, registre);
        assert_eq!(p, PlanGog::Registre { programme: prog(r"D:\J\REDprelauncher.exe"), motif: MotifSecours::SansFichier });
        assert_eq!(p.voie().0, "registre");
        assert_eq!(plan_gog(Err(MotifSecours::Illisible), true, jamais_g, registre).voie().0, "registre", "hors Galaxy : Galaxy n'est pas lu");
        // 4. Rien : le refus d'aujourd'hui, selon le cas.
        let p = plan_gog(Err(MotifSecours::AucuneTache), false, || None, || None);
        assert!(matches!(&p, PlanGog::Refus(m) if m.contains("ne note plus ce jeu")));
        assert_eq!(p.voie(), ("refus", None));
        assert!(matches!(plan_gog(Err(MotifSecours::AucuneTache), true, jamais_g, || None), PlanGog::Refus(m) if m.contains("connecté au compte qui le possède")));
    }

    #[test]
    fn un_programme_gog_absent_est_un_vrai_echec_dit_tel_quel() {
        // Le lancement direct qui échoue : un message clair, jamais une bascule sur Galaxy (rien n'est lancé ici).
        let p = crate::boutiques_pc::ProgrammeGog { exe: PathBuf::from(r"Z:\absent\Jeu.exe"), dossier_travail: PathBuf::from(r"Z:\absent"), arguments: "-a".into() };
        let e = demarrer_programme_gog(&p).unwrap_err();
        assert!(matches!(erreur_programme_gog(&e), Erreur::Disque(m) if m == "Le jeu GOG ne démarre pas."));
        // Les droits d'administrateur (740) : jamais pris tout seuls, et jamais une impasse (Galaxy reste possible).
        let admin = std::io::Error::from_raw_os_error(740);
        assert!(matches!(erreur_programme_gog(&admin), Erreur::Refus(m) if m == "Ce jeu demande les droits d'administrateur : Frogtend ne les prend pas tout seul. Tu peux le lancer depuis GOG Galaxy."));
    }

    /// RÉEL, lecture seule (A6) : la commande qui lancerait un jeu GOG de ce PC par Galaxy (rien n'est lancé).
    /// `FROGTEND_GOG_JEU=1209025141 FROGTEND_GOG_DOSSIER="D:\…\Horizon Zero Dawn" cargo test essai_commande_galaxy -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn essai_commande_galaxy() {
        let (jeu, dossier) = (std::env::var("FROGTEND_GOG_JEU").unwrap(), std::env::var("FROGTEND_GOG_DOSSIER").unwrap());
        let (client, args) = commande_galaxy(&jeu, Path::new(&dossier)).expect("Galaxy trouvé dans le registre");
        println!("client : {} (existe : {})", client.display(), client.is_file());
        println!("arguments : {args:?}");
    }

    #[test]
    fn un_jeu_switch_avec_maj_et_dlc_lance_son_xci_jamais_un_nsp() {
        // Relevé réel 1.23 (Fire Emblem Three Houses : .xci + 6 DLC + 1 maj, forme contenus_emulateur, lancer vide).
        let r: Value = serde_json::from_str(include_str!("../fixtures/simule/version-switch-contenus-123.json")).unwrap();
        let mut v = r["version"].clone();
        assert!(contenus_emulateur(&v) && !forme_fichier(&v));
        let noms: Vec<String> = roles(&v, &["jeu", "dlc", "maj"]);
        assert_eq!(noms.len(), 8);
        assert_eq!(fichier_au_role_jeu(&v, &noms).unwrap(), "Fire Emblem Three Houses.xci");
        // Le .xci en dernier : toujours lui (jamais « le premier fichier »).
        v["fichiers"].as_array_mut().unwrap().rotate_left(1);
        let noms: Vec<String> = roles(&v, &["jeu", "dlc", "maj"]);
        assert!(noms[0].ends_with(".nsp"));
        assert_eq!(fichier_au_role_jeu(&v, &noms).unwrap(), "Fire Emblem Three Houses.xci");
        // Aucun fichier au rôle jeu (ou deux) : refus, jamais un .nsp.
        let sans: Vec<String> = noms.iter().filter(|n| n.ends_with(".nsp")).cloned().collect();
        assert!(fichier_au_role_jeu(&v, &sans).is_err());
        // `lancer` donné par Firehouse : suivi s'il est un fichier reçu.
        v["installation"]["lancer"] = "Fire Emblem Three Houses [v1.2.0][v327680] (nsw2u).nsp".into();
        assert_eq!(fichier_au_role_jeu(&v, &noms).unwrap(), "Fire Emblem Three Houses [v1.2.0][v327680] (nsw2u).nsp");
        v["installation"]["lancer"] = "ailleurs.xci".into();
        assert_eq!(fichier_au_role_jeu(&v, &noms).unwrap(), "Fire Emblem Three Houses.xci");
    }

    #[test]
    fn lancer_n_est_suivi_que_s_il_est_un_fichier_recu() {
        let v = serde_json::json!({"installation": {"forme": "fichier", "lancer": "Xak.m3u"}});
        assert_eq!(lancer_de(&v, &["Xak.m3u".into()]).as_deref(), Some("Xak.m3u"));
        assert_eq!(lancer_de(&v, &["Autre.m3u".into()]), None);
        assert_eq!(lancer_de(&serde_json::json!({"installation": {"forme": "installeur", "lancer": "Xak.m3u"}}), &["Xak.m3u".into()]), None);
        // Le relevé réel : les rôles sont lus dans la fiche.
        let r: Value = serde_json::from_str(include_str!("../fixtures/simule/version-multidisques-123.json")).unwrap();
        assert_eq!(roles(&r["version"], &["archive_a_extraire"]).len(), 4);
        assert_eq!(roles(&r["version"], &["liste_disques"]), vec!["Xak (1989)(Microcabin).m3u"]);
        assert_eq!(lancer_de(&r["version"], &roles(&r["version"], &["liste_disques"])).as_deref(), Some("Xak (1989)(Microcabin).m3u"));
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
        assert!(matches!(n.jouer(110, None, None, None, &|_| {}).await, Err(Erreur::Reglage(_))));
        assert!(Path::new(&i.dossier).join("Dune (Europe).bin").is_file(), "rien n'est renommé ni déplacé");
    }

    /// Les fichiers DU JEU sous un dossier (sans le sous-dossier `Frogtend` des données propres à Frogtend) : (chemin
    /// relatif, contenu), triés — pour prouver que rien n'a bougé.
    fn photo(d: &Path) -> Vec<(String, Vec<u8>)> {
        fn parcourir(racine: &Path, d: &Path, l: &mut Vec<(String, Vec<u8>)>) {
            for e in std::fs::read_dir(d).unwrap().filter_map(|e| e.ok()) {
                let c = e.path();
                if c.is_dir() && d == racine && e.file_name() == "Frogtend" {
                    continue;
                }
                if c.is_dir() {
                    parcourir(racine, &c, l);
                } else {
                    l.push((c.strip_prefix(racine).unwrap().to_string_lossy().to_string(), std::fs::read(&c).unwrap()));
                }
            }
        }
        let mut l = Vec::new();
        parcourir(d, d, &mut l);
        l.sort();
        l
    }

    #[tokio::test]
    async fn annuler_le_choix_du_lanceur_defait_la_note_sans_toucher_au_disque() {
        // Une nature non dite et un programme du jeu : installé « méthode aucune », le lanceur reste à choisir.
        let (_d, n) = noyau_avec(&[("DUNE.EXE", b"MZ... un jeu"), ("DUNE.DAT", b"donnees")], "Jeu (nature non dite)").await;
        let i = n.installer(110, true).await.unwrap();
        assert_eq!(i.methode, Methode::Aucune);
        assert!(i.lanceur.is_none() && i.fichier_du_jeu.is_none());
        let dossier = PathBuf::from(&i.dossier);
        let avant = photo(&dossier);
        assert!(n.fichier_manifeste(110).is_file(), "le relevé d'un jeu PC est écrit à l'installation");

        n.installation_annuler(110).await.unwrap();
        assert!(n.registre().jeu(110).unwrap().unwrap().installation.is_none(), "le jeu redevient « téléchargé, pas installé »");
        assert_eq!(n.registre().jeu(110).unwrap().unwrap().etat, Etat::Telecharge);
        assert_eq!(photo(&dossier), avant, "aucun fichier du jeu effacé, déplacé ni changé");
        assert!(!n.fichier_manifeste(110).exists(), "le relevé n'est plus celui d'une installation");
        let de_cote = std::fs::read_dir(n.dossier_medias(110)).unwrap().filter_map(|e| e.ok()).any(|e| e.file_name().to_string_lossy().starts_with("manifeste.annule-"));
        assert!(de_cote, "le relevé est mis de côté, jamais effacé");
        assert!(n.installation_annuler(110).await.is_err(), "plus rien à défaire");
        // Le jeu s'installe de nouveau normalement.
        assert!(n.installer(110, true).await.is_ok());
    }

    #[tokio::test]
    async fn annuler_met_d_abord_a_l_abri_les_parties_apparues_depuis_la_note() {
        // Une partie posée dans le jeu après la note (partie restaurée reposée à l'installation, ou jouée).
        let (d, n) = noyau_avec(&[("DUNE.EXE", b"MZ... un jeu")], "").await;
        let i = n.installer(110, true).await.unwrap();
        let dossier = PathBuf::from(&i.dossier);
        std::fs::write(dossier.join("PARTIE1.SAV"), b"partie").unwrap();
        let avant = photo(&dossier);
        // Sans dossier des abris : refus, rien ne change (la note et son relevé restent).
        assert!(matches!(n.installation_annuler(110).await, Err(Erreur::Refus(_))));
        assert_eq!(n.registre().jeu(110).unwrap().unwrap().installation, Some(i.clone()));
        assert!(n.fichier_manifeste(110).is_file());
        // Avec : une COPIE à l'abri d'abord, puis la note défaite ; la partie reste aussi dans le jeu.
        n.regler_abris(Some(d.path().join("Abris")));
        n.installation_annuler(110).await.unwrap();
        assert!(n.registre().jeu(110).unwrap().unwrap().installation.is_none());
        assert_eq!(photo(&dossier), avant, "rien n'est déplacé ni effacé");
        let abris: Vec<PathBuf> = std::fs::read_dir(d.path().join("Abris").join("110")).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.is_dir()).collect();
        assert_eq!(abris.len(), 1);
        assert_eq!(std::fs::read(abris[0].join("PARTIE1.SAV")).unwrap(), b"partie");
    }

    #[tokio::test]
    async fn annuler_refuse_un_lanceur_choisi_ou_une_autre_methode() {
        // Un lanceur déjà choisi : refus, rien ne change.
        let (_d, n) = noyau_avec(&[("DUNE.EXE", b"MZ... un jeu")], "").await;
        let mut i = n.installer(110, true).await.unwrap();
        i.lanceur = Some(Lanceur { programme: Path::new(&i.dossier).join("DUNE.EXE").to_string_lossy().into(), arguments: vec![], dossier: i.dossier.clone() });
        n.registre().changer_installation(110, Some(&i)).unwrap();
        assert!(matches!(n.installation_annuler(110).await, Err(Erreur::Refus(_))));
        assert_eq!(n.registre().jeu(110).unwrap().unwrap().installation, Some(i.clone()), "la note reste entière");
        // Un fichier donné à un émulateur (image disque installée) : refus.
        let (_d2, n2) = noyau_avec(&[("Dune.iso", b"ISO")], "Image disque").await;
        assert!(n2.installer(110, true).await.unwrap().fichier_du_jeu.is_some());
        assert!(matches!(n2.installation_annuler(110).await, Err(Erreur::Refus(_))));
        // Une archive décompressée : refus, et ce qui a été décompressé reste.
        let (_d3, n3) = noyau_avec(&[("jeu.zip", &zip(&[("JEU.EXE", b"MZ")]))], "").await;
        let i3 = n3.installer(110, true).await.unwrap();
        assert!(matches!(i3.methode, Methode::Archive { .. }));
        let avant = photo(Path::new(&i3.dossier));
        assert!(matches!(n3.installation_annuler(110).await, Err(Erreur::Refus(_))));
        assert!(n3.registre().jeu(110).unwrap().unwrap().installation.is_some());
        assert_eq!(photo(Path::new(&i3.dossier)), avant);
    }

    #[tokio::test]
    async fn refaire_l_installation_de_mk8_prise_pour_un_jeu_pc() {
        // L'état EXACT de Seb le 26/11 (jeux.db), que la fiche gardée dise encore « nature non dite » ou déjà
        // « image disque » (corrigée par Firehouse).
        let mk8 = "Mario Kart 8 Deluxe (v3.01 Incl All DLC) (EU) SuperXCI - MBC.xci";
        for qualite in ["Jeu (nature non dite)", "Jeu — image disque"] {
            let (_d, n) = noyau_avec_sur("Nintendo Switch", &[(mk8, b"HEAD")], qualite).await;
            let dossier = n.registre().jeu(110).unwrap().unwrap().dossier;
            let seb = serde_json::json!({"dossier": dossier, "methode": {"sorte": "aucune"}, "lanceur": null, "fichier_du_jeu": null, "installe_le": "1795699748"});
            let i: Installation = serde_json::from_value(seb).unwrap();
            n.registre().changer_installation(110, Some(&i)).unwrap();
            let avant = photo(Path::new(&dossier));
            assert!(n.installation_refaisable(110).await.unwrap(), "{qualite} : à refaire");

            // « 📦 Refaire l'installation » : la note défaite, puis l'installation normale.
            n.installation_annuler(110).await.unwrap();
            let p = n.preparer_installation(110).await.unwrap();
            assert_eq!(p.nature, Nature::ImageDisque, "{qualite}");
            let i = n.installer(110, true).await.unwrap();
            assert_eq!(i.methode, Methode::Aucune);
            assert_eq!(i.fichier_du_jeu.as_deref(), Some(mk8), "{qualite} : le .xci donné à l'émulateur, jamais renommé");
            assert_eq!(photo(Path::new(&dossier)), avant, "{qualite} : rien retéléchargé, effacé ni renommé");
            assert!(!n.installation_refaisable(110).await.unwrap(), "{qualite} : plus rien à refaire");
        }
    }

    #[tokio::test]
    async fn une_nature_non_dite_xci_va_a_l_emulateur_un_exe_reste_un_jeu_pc() {
        let (_d, n) = noyau_avec_sur("Nintendo Switch", &[("Jeu.xci", b"HEAD")], "Jeu (nature non dite)").await;
        let p = n.preparer_installation(110).await.unwrap();
        assert_eq!((p.nature, p.methode), (Nature::ImageDisque, Methode::Aucune));
        assert_eq!(n.installer(110, true).await.unwrap().fichier_du_jeu.as_deref(), Some("Jeu.xci"));

        let (_d2, n2) = noyau_avec_sur("Nintendo Switch", &[("Jeu.exe", b"MZ... un jeu")], "Jeu (nature non dite)").await;
        let p2 = n2.preparer_installation(110).await.unwrap();
        assert_eq!(p2.nature, Nature::NonDite, "un .exe : la voie PC");
        let i2 = n2.installer(110, true).await.unwrap();
        assert!(i2.fichier_du_jeu.is_none(), "son lanceur reste à choisir");
        assert!(!n2.installation_refaisable(110).await.unwrap(), "un vrai jeu PC n'est jamais « à refaire »");

        // Une archive d'un jeu Switch de nature non dite : jamais donnée telle quelle à l'émulateur.
        let (_d3, n3) = noyau_avec_sur("Nintendo Switch", &[("jeu.zip", &zip(&[("Jeu.xci", b"HEAD")]))], "").await;
        assert!(matches!(n3.preparer_installation(110).await.unwrap().methode, Methode::Archive { .. }));
    }

    /// Une carte e-Reader en 7z, avec ses bandes.
    fn carte_7z(bandes: &[&str]) -> Vec<u8> {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("carte.7z");
        let mut w = sevenz_rust::SevenZWriter::create(&f).unwrap();
        for nom in bandes {
            let mut e = sevenz_rust::SevenZArchiveEntry::new();
            e.name = nom.to_string();
            e.has_stream = true;
            w.push_archive_entry(e, Some(&b"dotcode"[..])).unwrap();
        }
        w.finish().unwrap();
        std::fs::read(&f).unwrap()
    }

    #[tokio::test]
    async fn une_carte_e_reader_se_lit_dans_sa_cartouche_avec_ses_bandes() {
        let (usa, japon) = (carte_7z(&["Carte (Strip 2).raw", "Carte (Strip 1).raw"]), carte_7z(&["Kado (Strip 1).raw"]));
        let (_d, n) = noyau_avec(&[("e-Reader (USA).gba", b"ROM"), ("Carte (USA).7z", &usa), ("Kado (Japan).7z", &japon)], "ROM").await;
        // Le jeu simulé sert à la fois de carte (ses .7z, la japonaise en version 201) et de cartouche (le .gba).
        let mut j = n.registre().jeu(110).unwrap().unwrap();
        j.fichiers[0].version = Some(300); // la « cartouche » n'est pas une carte
        j.fichiers[2].version = Some(201);
        j.plateforme = crate::ereader::PLATEFORME.into();
        n.registre().retirer(110).unwrap();
        n.registre().ajouter(&j).unwrap();
        let i = Installation {
            dossier: j.dossier.clone(),
            methode: Methode::Aucune,
            lanceur: None,
            fichier_du_jeu: Some("e-Reader (USA).gba".into()),
            installe_le: "1".into(),
            versions: vec![],
            boutique: None,
            extraits: vec![],
        };
        n.registre().changer_installation(110, Some(&i)).unwrap();
        let j = n.registre().jeu(110).unwrap().unwrap();
        let mgba = r"C:\Emulateurs\mGBA\mGBA.exe";
        // Un autre émulateur que mGBA : refusé.
        assert!(matches!(n.preparer_carte(&j, r"C:\RetroArch\retroarch.exe", None).await, Err(Erreur::Prevenir(_))));
        // Aucune cartouche connue (simulé, hors ligne, rien de gardé) : dit, rien de décompressé.
        assert!(matches!(n.preparer_carte(&j, mgba, None).await, Err(Erreur::Prevenir(_))));
        let bandes = Path::new(&j.dossier).join("Frogtend").join("cartes");
        assert!(!bandes.exists());
        // La liste gardée par le profil (hors ligne) : l'américaine est ce jeu, la japonaise n'est pas sur ce PC.
        let s = n.session().await.unwrap();
        let liste = serde_json::json!([
            {"media_id": 110, "nom": "e-Reader (USA)", "region": "USA"},
            {"media_id": 999, "nom": "Card e-Reader (Japan)", "region": "Japan"}]);
        std::fs::write(s.dossier.join(crate::ereader::FICHIER_CARTOUCHES), liste.to_string()).unwrap();
        let (cartouche, l) = n.preparer_carte(&j, mgba, None).await.unwrap();
        assert!(cartouche.ends_with("e-Reader (USA).gba"), "{cartouche:?}");
        let b1 = bandes.join("200").join("Carte (Strip 1).raw");
        let b2 = bandes.join("200").join("Carte (Strip 2).raw");
        assert_eq!(l, [b1.clone(), b2.clone()]);
        assert_eq!(std::fs::read(&b1).unwrap(), b"dotcode");
        // La ligne complète : bandes AVANT la cartouche, chacune un argument.
        let mut lanceur = lanceur_emulateur(mgba, "-f", &cartouche).unwrap();
        crate::ereader::inserer_bandes(&mut lanceur.arguments, &cartouche, &l);
        let attendu: Vec<String> = ["-f".into(), "--ecard".into(), b1.display().to_string(), "--ecard".into(), b2.display().to_string(), cartouche.display().to_string()].to_vec();
        assert_eq!(lanceur.arguments, attendu);
        // La cartouche seulement TÉLÉCHARGÉE (une ROM n'a rien à installer) : son fichier reçu sert.
        n.registre().changer_installation(110, None).unwrap();
        let (sans_installation, _) = n.preparer_carte(&j, mgba, None).await.unwrap();
        assert!(sans_installation.ends_with("e-Reader (USA).gba"));
        // Encore en route : dit, jamais proposée au téléchargement.
        n.registre().changer_etat(110, Etat::EnCours, None).unwrap();
        assert!(matches!(n.preparer_carte(&j, mgba, None).await, Err(Erreur::Prevenir(_))));
        n.registre().changer_etat(110, Etat::Telecharge, None).unwrap();
        // La carte japonaise : sa cartouche manque, l'interface la propose (son numéro).
        match n.preparer_carte(&j, mgba, Some(201)).await {
            Err(Erreur::CartoucheAbsente { cartouche, .. }) => assert_eq!(cartouche, 999),
            r => panic!("{r:?}"),
        }
        assert!(matches!(n.preparer_carte(&j, mgba, Some(555)).await, Err(Erreur::Introuvable(_))));
        // Retirer une carte : ses bandes partent avec elle, et rien d'autre.
        std::fs::create_dir_all(bandes.join("201")).unwrap();
        std::fs::write(bandes.join("201").join("Kado (Strip 1).raw"), b"x").unwrap();
        n.retirer_carte(110, 201).await.unwrap();
        assert!(!bandes.join("201").exists() && b1.is_file(), "seulement les bandes de la carte retirée");
        // Une carte incomplète (téléchargement en cours) : refusée.
        std::fs::write(Path::new(&j.dossier).join("Carte (USA).7z"), b"tronque").unwrap();
        std::fs::remove_dir_all(bandes.join("200")).unwrap();
        assert!(matches!(n.preparer_carte(&j, mgba, None).await, Err(Erreur::Refus(_))));
        // Les bandes partent avec le jeu.
        std::fs::create_dir_all(bandes.join("200")).unwrap();
        n.retirer_medias(&j).unwrap();
        assert!(!bandes.exists());
        assert!(Path::new(&j.dossier).join("Carte (USA).7z").is_file(), "jamais la carte elle-même");
    }

    #[tokio::test]
    async fn jouer_suit_la_partie_et_compte_le_temps() {
        let archive = zip(&[("JEU.BAT", b"@echo off\r\nping -n 2 127.0.0.1 >nul\r\n")]);
        let (_d, n) = noyau_avec(&[("jeu.zip", &archive)], "Jeu — prêt à jouer").await;
        n.installer(110, true).await.unwrap();
        let c = n.candidats_lancement(110).await.unwrap();
        n.choisir_lanceur(110, c[0].lanceur.clone()).await.unwrap();
        let (pid, dossiers, carte, voie) = n.jouer(110, None, None, None, &|_| {}).await.unwrap();
        assert_eq!(voie, None, "pas un jeu GOG : aucune voie GOG");
        assert!(!carte, "en simulé, aucune carte n'est cédée");
        let fin = n.suivre_partie(110, pid, dossiers).await.unwrap();
        assert!(fin.secondes <= 30);
        let j = n.registre().jeu(110).unwrap().unwrap();
        assert!(j.derniere_partie.is_some());
    }

    /// « ▶ Ouvrir l'émulateur seul » : suivi jusqu'à sa fermeture, mais le temps n'est compté à AUCUN jeu.
    #[tokio::test]
    async fn un_emulateur_ouvert_seul_est_suivi_sans_compter_de_temps() {
        let archive = zip(&[("JEU.BAT", b"@echo off\r\n")]);
        let (d, n) = noyau_avec(&[("jeu.zip", &archive)], "Jeu — prêt à jouer").await;
        n.installer(110, true).await.unwrap();
        // Un faux émulateur : un .bat qui vit ~1 s, dans son propre dossier (jamais un vrai émulateur).
        let emu = d.path().join("Emulateur");
        std::fs::create_dir_all(&emu).unwrap();
        std::fs::write(emu.join("emu.bat"), b"@echo off\r\nping -n 2 127.0.0.1 >nul\r\n").unwrap();
        let l = crate::installation::Lanceur { programme: "cmd.exe".into(), arguments: vec!["/c".into(), emu.join("emu.bat").to_string_lossy().to_string()], dossier: emu.to_string_lossy().to_string() };
        let pid = n.ouvrir_sous_verrou(&emu, "Eden", false, || async { crate::lancement::demarrer(&l) }).await.unwrap();
        assert!(n.marquer_emulateur_seul(&emu, "Eden", false).is_err(), "jamais deux fois le même");
        assert!(n.emulateur_seul_nom().is_some() && n.emulateur_seul_dans(&emu.join("emu.exe")) && !n.emulateur_seul_dans(&d.path().join("Autre").join("x.exe")));
        assert_eq!(n.emulateur_seul_nom().as_deref(), Some("Eden"));
        assert!(n.annoncer_sans_jeu("debut").await.is_some(), "en simulé, la session sans jeu est acceptée");
        let secondes = n.suivre_emulateur_seul(pid, emu.clone()).await.unwrap();
        assert!(secondes <= 30);
        n.liberer_emulateur_seul(&emu);
        assert!(!n.emulateur_seul_nom().is_some());
        let j = n.registre().jeu(110).unwrap().unwrap();
        assert!(j.derniere_partie.is_none(), "aucun temps compté au jeu");
        assert!(n.registre().tous().unwrap().iter().all(|j| j.derniere_partie.is_none()));
    }

    /// Un émulateur ouvert seul et une partie ne se chevauchent jamais, dans un sens comme dans l'autre ; le profil ne
    /// change pas pendant qu'il est ouvert ; un lancement raté rend le verrou.
    #[tokio::test]
    async fn emulateur_seul_et_partie_se_refusent_l_un_l_autre() {
        let archive = zip(&[("JEU.BAT", b"@echo off\r\n")]);
        let (d, n) = noyau_avec(&[("jeu.zip", &archive)], "Jeu — prêt à jouer").await;
        let emu = d.path().join("Eden");
        // Une partie en cours (menu en jeu) ou un lancement en préparation : l'émulateur ne s'ouvre pas seul.
        assert!(matches!(n.marquer_emulateur_seul(&emu, "Eden", true), Err(Erreur::Refus(_))));
        {
            let _lancement = n.commencer_lancement().unwrap();
            assert!(matches!(n.marquer_emulateur_seul(&emu, "Eden", false), Err(Erreur::Refus(_))), "jeu_jouer en préparation");
        }
        // La préparation finie (ou ratée) : il s'ouvre.
        n.marquer_emulateur_seul(&emu, "Eden", false).unwrap();
        // Ouvert seul : aucune partie ne commence à se lancer (jeu_jouer s'arrête là), ni un 2e émulateur.
        match n.commencer_lancement() {
            Err(Erreur::Refus(m)) => assert!(m.contains("Eden est ouvert seul")),
            _ => panic!("une partie pendant l'émulateur ouvert seul"),
        }
        assert!(n.marquer_emulateur_seul(&d.path().join("Ryubing"), "Ryubing", false).is_err(), "un à la fois");
        // Le profil ne se ferme ni ne change.
        assert!(matches!(n.refuser_si_emulateur_seul("change de profil"), Err(Erreur::Refus(_))));
        n.liberer_emulateur_seul(&emu);
        assert!(n.refuser_si_emulateur_seul("change de profil").is_ok());
        assert!(n.commencer_lancement().is_ok());
        // La préparation de l'émulateur échoue : le verrou est rendu aussitôt.
        let r = n.ouvrir_sous_verrou(&emu, "Eden", false, || async { Err::<u32, _>(Erreur::Disque("préparation ratée".into())) }).await;
        assert!(matches!(r, Err(Erreur::Disque(_))));
        assert!(!n.emulateur_seul_nom().is_some(), "verrou rendu");
    }

    /// Lot S3 (M12) : un abri du dossier du jeu prend la seconde suivante libre (jamais un renommage sur un abri déjà là).
    #[test]
    fn un_abri_prend_la_seconde_suivante_libre() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(super::date_libre(d.path(), "1790000000"), "1790000000");
        std::fs::create_dir_all(d.path().join("1790000000")).unwrap();
        std::fs::create_dir_all(d.path().join("1790000001.en-cours")).unwrap();
        std::fs::write(d.path().join("1790000002.json"), b"{}").unwrap();
        assert_eq!(super::date_libre(d.path(), "1790000000"), "1790000003");
    }

    /// Lot S3 (Seb, 23/11) : jamais de changement de profil pendant qu'un jeu tourne ou se lance.
    #[tokio::test]
    async fn on_ne_change_pas_de_profil_pendant_une_partie() {
        let (_d, n) = noyau_avec(&[("dune.zip", &zip(&[("DUNE.EXE", b"MZ")]))], "Jeu — prêt à jouer").await;
        match n.changer_de_profil(true, "change de profil") {
            Err(Erreur::Refus(m)) => assert!(m.contains("Un jeu est en cours") && m.contains("change de profil"), "{m}"),
            _ => panic!("un refus attendu"),
        }
        assert!(n.changer_de_profil(false, "change de profil").is_ok());
        // Une partie qui se lance (jeu_jouer en cours de préparation) : refusé aussi.
        let lancement = n.commencer_lancement().unwrap();
        assert!(matches!(n.changer_de_profil(false, "change de profil"), Err(Erreur::Refus(_))));
        drop(lancement);
        // M13 : pendant un changement de profil, aucun lancement ne commence ; après, si.
        let changement = n.changer_de_profil(false, "change de profil").unwrap();
        assert!(matches!(n.commencer_lancement(), Err(Erreur::Refus(m)) if m.contains("en train de changer")));
        assert!(n.changer_de_profil(false, "change de profil").is_err(), "un changement à la fois");
        drop(changement);
        assert!(n.commencer_lancement().is_ok());
    }

    /// Lot S3 : Dune installé devient un jeu PC (Windows), avec une partie dans `SAVES` de son dossier, cet
    /// emplacement connu du profil, et le dossier des abris réglé. Rend (dossier installé, la partie, les abris).
    async fn jeu_pc_installe(d: &tempfile::TempDir, n: &Noyau) -> (PathBuf, PathBuf, PathBuf) {
        let i = n.installer(110, true).await.unwrap();
        let mut j = n.registre().jeu(110).unwrap().unwrap();
        n.registre().retirer(110).unwrap();
        j.plateforme = "Windows".into();
        let installation = j.installation.take();
        n.registre().ajouter(&j).unwrap();
        n.registre().changer_installation(110, installation.as_ref()).unwrap();
        let profil = n.actif().await.unwrap().id;
        crate::commandes::tests_abri_auto::connaitre_saves(n, &profil, 110);
        let installe = PathBuf::from(&i.dossier);
        std::fs::create_dir_all(installe.join("SAVES")).unwrap();
        let sav = installe.join("SAVES").join("PARTIE1.SAV");
        std::fs::write(&sav, b"partie").unwrap();
        crate::commandes::tests_abri_auto::vieillir(&sav);
        let abris = d.path().join("Abris");
        n.regler_abris(Some(abris.clone()));
        (installe, sav, abris)
    }

    /// Les abris de sorte `emplacements` (lot S2/S3) du jeu 110 : (déclencheur, contenu de la partie).
    fn abris_emplacements(abris: &Path) -> Vec<(String, Vec<u8>)> {
        let mut l: Vec<(String, Vec<u8>)> = std::fs::read_dir(abris.join("110"))
            .unwrap()
            .flatten()
            .filter(|e| e.path().is_dir())
            .filter_map(|e| crate::abri_parties::lire_fiche(&e.path()).filter(|f| f.sorte == crate::abri_parties::SORTE_EMPLACEMENTS).map(|f| (e.path(), f)))
            .map(|(p, f)| (f.declencheur.clone(), std::fs::read(p.join(&f.fichiers[0].relatif)).unwrap()))
            .collect();
        l.sort();
        l
    }

    #[tokio::test]
    async fn s3_les_parties_sont_a_l_abri_avant_le_desinstalleur_et_un_abri_impossible_ne_retire_rien() {
        let (d, n) = noyau_avec(&[("dune.zip", &zip(&[("DUNE.EXE", b"MZ")]))], "Jeu — prêt à jouer").await;
        let (installe, sav, abris) = jeu_pc_installe(&d, &n).await;
        // Un faux désinstalleur (pas un vrai programme : Windows refuse de le lancer, le retrait continue).
        std::fs::write(installe.join("unins000.exe"), b"pas un programme").unwrap();
        // La partie vient d'être écrite : l'abri est impossible, RIEN n'est retiré.
        std::fs::write(&sav, b"toute neuve").unwrap();
        match n.retirer_du_pc(110).await {
            Err(Erreur::AbriImpossible { operation, cause, .. }) => assert_eq!((operation.as_str(), cause.as_str()), ("avant_retrait", "echec")),
            autre => panic!("{autre:?}"),
        }
        assert!(sav.is_file() && installe.join("unins000.exe").is_file() && n.registre().jeu(110).unwrap().is_some(), "rien retiré");
        // M11 : l'abri des parties passe AVANT tout le reste — pas même l'abri du dossier du jeu n'a été fait.
        assert!(!abris.join("110").exists(), "rien d'autre n'a été touché");
        assert!(!n.utilise(110));
        // Le calme revenu : l'abri d'abord, puis le retrait (le dossier du jeu part, la partie reste dans l'abri).
        crate::commandes::tests_abri_auto::vieillir(&sav);
        // Aussitôt (M12) : l'abri des parties vient de prendre cette seconde, celui du dossier du jeu prend la suivante.
        n.retirer_du_pc(110).await.unwrap();
        assert!(!installe.exists() && n.registre().jeu(110).unwrap().is_none());
        assert_eq!(abris_emplacements(&abris), [("avant_retrait".to_string(), b"toute neuve".to_vec())]);
    }

    /// 0.227.1 : un jeu qui garde des mods de Vortex retirés par Frogtend (leur seule copie, peut-être) n'est jamais
    /// effacé du PC : refusé avant tout, rien n'a bougé.
    #[tokio::test]
    async fn un_jeu_qui_garde_des_mods_de_vortex_retires_n_est_jamais_efface() {
        let (d, n) = noyau_avec(&[("dune.zip", &zip(&[("DUNE.EXE", b"MZ")]))], "Jeu — prêt à jouer").await;
        let (installe, _sav, abris) = jeu_pc_installe(&d, &n).await;
        let r = crate::mods_vortex::Retrait {
            version: 1,
            etat: crate::mods_vortex::EtatRetrait::Fini,
            commence_le: "100".into(),
            depot: String::new(),
            fichiers: vec![],
            dossiers_retires: vec![],
            deploiement_range: true,
            sauvegardes_vortex: vec![],
        };
        let dossier = crate::mods_vortex::dossier_retraits(&installe).join("100");
        std::fs::create_dir_all(dossier.join("fichiers")).unwrap();
        std::fs::write(dossier.join("fichiers").join("mod.archive"), b"seule copie").unwrap();
        std::fs::write(dossier.join("retrait.json"), serde_json::to_vec(&r).unwrap()).unwrap();
        match n.retirer_du_pc(110).await {
            Err(Erreur::Refus(m)) => assert!(m.contains("mods de Vortex") && !m.contains(":\\"), "{m}"),
            autre => panic!("{autre:?}"),
        }
        assert_eq!(std::fs::read(dossier.join("fichiers").join("mod.archive")).unwrap(), b"seule copie");
        assert!(installe.join("DUNE.EXE").is_file() || installe.is_dir());
        assert!(n.registre().jeu(110).unwrap().is_some() && !abris.join("110").exists(), "rien d'autre n'a été touché");
        assert!(!n.utilise(110));
    }

    #[tokio::test]
    async fn s3_continuer_sans_abri_retire_le_jeu_seulement_apres_ce_choix() {
        let (d, n) = noyau_avec(&[("dune.zip", &zip(&[("DUNE.EXE", b"MZ")]))], "Jeu — prêt à jouer").await;
        let (installe, sav, abris) = jeu_pc_installe(&d, &n).await;
        std::fs::write(&sav, b"toute neuve").unwrap();
        assert!(matches!(n.retirer_du_pc(110).await, Err(Erreur::AbriImpossible { .. })));
        crate::commandes::accorder_sans_abri(&n, 110, crate::abri_parties::DECLENCHEUR_AVANT_RETRAIT).await.unwrap();
        n.retirer_du_pc(110).await.unwrap();
        assert!(!installe.exists());
        assert!(abris_emplacements(&abris).is_empty(), "sans abri des parties, comme choisi");
    }

    #[tokio::test]
    async fn s3_une_reinstallation_met_d_abord_les_parties_a_l_abri() {
        let (d, n) = noyau_avec(&[("dune.zip", &zip(&[("DUNE.EXE", b"MZ")]))], "Jeu — prêt à jouer").await;
        let (_installe, sav, abris) = jeu_pc_installe(&d, &n).await;
        // Impossible (partie écrite à l'instant) : rien n'est réinstallé.
        std::fs::write(&sav, b"neuve").unwrap();
        assert!(matches!(n.installer(110, true).await, Err(Erreur::AbriImpossible { ref operation, .. }) if operation == "avant_reinstallation"));
        assert_eq!(std::fs::read(&sav).unwrap(), b"neuve");
        crate::commandes::tests_abri_auto::vieillir(&sav);
        n.installer(110, true).await.unwrap();
        assert_eq!(abris_emplacements(&abris), [("avant_reinstallation".to_string(), b"neuve".to_vec())]);
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
    async fn les_parties_a_l_abri_se_remettent_sans_jamais_ecraser_une_partie_jouee_depuis() {
        let archive = zip(&[("DUNE.EXE", b"MZ"), ("DUNE.CFG", b"v1")]);
        let (d, n) = noyau_avec(&[("dune.zip", &archive)], "Jeu — prêt à jouer").await;
        let i = n.installer(110, true).await.unwrap();
        let installe = PathBuf::from(&i.dossier);
        std::fs::create_dir_all(installe.join("SAVES")).unwrap();
        std::fs::write(installe.join("SAVES").join("PARTIE1.SAV"), b"partie").unwrap();
        std::fs::write(installe.join("DUNE.CFG"), b"v2 !").unwrap();
        n.regler_abris(Some(d.path().join("Abris")));
        assert!(n.abri_a_reposer(110).await.unwrap().is_none(), "pas encore d'abri");
        let abri = n.mettre_a_l_abri(110).await.unwrap();
        let a = PathBuf::from(&abri.dossier);
        // La fiche à côté, jamais dedans ; aucun dossier « en cours » laissé.
        let date = a.file_name().unwrap().to_string_lossy().to_string();
        assert!(a.parent().unwrap().join(format!("{date}.json")).is_file());
        assert!(!a.join(format!("{date}.json")).exists());
        // Le nuage Steam n'est jamais remis.
        std::fs::create_dir_all(a.join("nuage-steam").join("x")).unwrap();
        std::fs::write(a.join("nuage-steam").join("x").join("abri.json"), b"{}").unwrap();
        // Lot S2 : la fiche de cet abri dit son profil (celui de la session) et la version du jeu, lue comme avant.
        let f = crate::abri_parties::lire_fiche(&a).unwrap();
        assert_eq!((f.sorte.as_str(), f.profil.is_empty(), f.version, f.fichiers.len(), f.complet), (crate::abri_parties::SORTE_JEU, false, Some(n.registre().jeu(110).unwrap().unwrap().version), 2, true));
        // Tout est déjà identique : rien à remettre.
        assert!(n.abri_a_reposer(110).await.unwrap().is_none());
        // La partie a disparu (réinstallation) ; le réglage a été joué depuis : seule la partie revient.
        std::fs::remove_file(installe.join("SAVES").join("PARTIE1.SAV")).unwrap();
        std::fs::write(installe.join("DUNE.CFG"), b"v3 joue depuis").unwrap();
        let p = n.abri_a_reposer(110).await.unwrap().expect("une partie à remettre");
        assert_eq!((p.fichiers, p.gardes), (1, 1));
        assert_eq!(n.reposer_abri(110).await.unwrap(), 1);
        assert_eq!(std::fs::read(installe.join("SAVES").join("PARTIE1.SAV")).unwrap(), b"partie");
        assert_eq!(std::fs::read(installe.join("DUNE.CFG")).unwrap(), b"v3 joue depuis", "jamais écrasé");
        assert!(!installe.join("nuage-steam").exists());
        assert!(a.join("SAVES").join("PARTIE1.SAV").is_file(), "l'abri reste");
        // Une fois répondu pour cet abri, la question ne revient pas.
        std::fs::remove_file(installe.join("SAVES").join("PARTIE1.SAV")).unwrap();
        assert!(n.abri_a_reposer(110).await.unwrap().is_none());
        // Un abri fait À LA MAIN plus récent (lot S2, sa fiche le dit) ne la fait pas revenir non plus.
        let main = a.parent().unwrap().join("9999999999");
        std::fs::create_dir_all(main.join("emplacements").join("0a1b2c3d")).unwrap();
        std::fs::write(main.join("emplacements").join("0a1b2c3d").join("x.sav"), b"x").unwrap();
        let fiche = crate::abri_parties::FicheAbri::nouvelle(crate::abri_parties::SORTE_EMPLACEMENTS, "p1", vec![], &[], 0, 0);
        std::fs::write(a.parent().unwrap().join("9999999999.json"), serde_json::to_vec(&fiche).unwrap()).unwrap();
        assert!(n.abri_a_reposer(110).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn un_fichier_d_origine_est_remplace_et_garde_a_cote_un_programme_d_une_autre_version_jamais() {
        let archive = zip(&[("DUNE.EXE", b"MZ"), ("DUNE.CFG", b"v1")]);
        let (d, n) = noyau_avec(&[("dune.zip", &archive)], "Jeu — prêt à jouer").await;
        let i = n.installer(110, true).await.unwrap();
        let installe = PathBuf::from(&i.dossier);
        std::fs::write(installe.join("DUNE.CFG"), b"mes reglages").unwrap();
        n.regler_abris(Some(d.path().join("Abris")));
        let abri = PathBuf::from(n.mettre_a_l_abri(110).await.unwrap().dossier);
        // Un programme ajouté dans l'abri (un mod), d'une AUTRE version : jamais remis.
        let date = abri.file_name().unwrap().to_string_lossy().to_string();
        std::fs::write(abri.join("MOD.DLL"), b"vieux").unwrap();
        std::fs::write(abri.parent().unwrap().join(format!("{date}.json")), br#"{"version": 999, "complet": true}"#).unwrap();
        // Réinstallé : le réglage est revenu à celui d'origine (comme au relevé) — même version que l'abri ?
        // Ici la fiche dit 999 : le réglage n'est pas remplacé non plus, il est gardé.
        let m = n.fichier_manifeste(110);
        let releve: crate::installation::Manifeste = serde_json::from_slice(&std::fs::read(&m).unwrap()).unwrap();
        std::fs::write(installe.join("DUNE.CFG"), b"v1").unwrap();
        let mut releve2 = releve.clone();
        releve2.fichiers.insert("DUNE.CFG".into(), crate::installation::etat_fichier(&installe.join("DUNE.CFG")).unwrap());
        std::fs::write(&m, serde_json::to_vec(&releve2).unwrap()).unwrap();
        let p = n.abri_a_reposer(110).await.unwrap();
        assert!(p.is_none() || p.as_ref().unwrap().fichiers == 0, "autre version : rien");
        // Même version : le réglage d'origine est remplacé, l'original gardé à côté ; le programme reste interdit ? non,
        // même version : posé.
        std::fs::write(abri.parent().unwrap().join(format!("{date}.json")), format!(r#"{{"version": {}, "complet": true}}"#, n.registre().jeu(110).unwrap().unwrap().version)).unwrap();
        let p = n.abri_a_reposer(110).await.unwrap().expect("à remettre");
        assert_eq!(p.fichiers, 2);
        assert_eq!(n.reposer_abri(110).await.unwrap(), 2);
        assert_eq!(std::fs::read(installe.join("DUNE.CFG")).unwrap(), b"mes reglages");
        let garde = crate::locale::dossier_frogtend(&n.registre().jeu(110).unwrap().unwrap()).join("avant-abri");
        assert!(crate::installation::fichiers_de(&garde).unwrap().iter().any(|f| f.ends_with("DUNE.CFG")), "l'original gardé à côté");
        assert!(!installe.join("DUNE.CFG.frogtend-tmp").exists());
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
