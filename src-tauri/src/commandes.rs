//! Les commandes que l'interface appelle (`invoke`). Minces : le travail est fait par le noyau.
//!
//! Aucune ne renvoie un jeton. Les réglages de connexion sont relus dans `pc.json` à chaque ouverture de profil :
//! l'interface ne peut pas envoyer le jeton ailleurs qu'à l'adresse enregistrée.

use crate::erreurs::{Erreur, Resultat};
use crate::ludotheque::{Filtre, JeuResume, Liste, Plateforme};
use crate::noyau::{BilanSynchro, Connexion, FicheLue, Noyau};
use crate::profils::ProfilVisible;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::jeux_pc::{EmplacementPropose, Emplacements, JeuPc};
use crate::locale::JeuPcVu;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_store::StoreExt;

const ADRESSE_PAR_DEFAUT: &str = "https://jeux.hikari-no-sekai.fr";

/// Les réglages de connexion de ce PC, tels que l'interface les a enregistrés.
pub(crate) fn connexion(app: &AppHandle) -> Connexion {
    let reglages = app.store("pc.json").ok().and_then(|s| s.get("reglages")).unwrap_or(Value::Null);
    let f = &reglages["firehouse"];
    Connexion {
        adresse: f["adresse"].as_str().unwrap_or(ADRESSE_PAR_DEFAUT).to_string(),
        simule: f["simule"].as_bool().unwrap_or(false),
    }
}

/// Le dossier des abris de parties réglé dans `pc.json` (`dossierAbris`), s'il l'est.
pub(crate) fn dossier_abris_regle(app: &AppHandle) -> Option<std::path::PathBuf> {
    let r = app.store("pc.json").ok()?.get("reglages")?;
    r["dossierAbris"].as_str().filter(|s| !s.trim().is_empty()).map(std::path::PathBuf::from)
}

/// Le dossier des abris de parties vient d'être réglé : le noyau l'utilise, et y range les abris de l'ancien
/// emplacement (copie vérifiée). Rend le nombre d'abris rangés.
#[tauri::command]
pub async fn abris_regler(app: AppHandle, noyau: State<'_, Noyau>, dossier: String) -> Resultat<usize> {
    let d = std::path::PathBuf::from(dossier.trim());
    // Durcissement (étape 6) : seulement le dossier des abris RÉGLÉ dans `pc.json` (l'interface l'y enregistre juste
    // avant), jamais un autre dossier créé et rempli d'abris.
    if !dossier.trim().is_empty() {
        if let Some(m) = refus_dossier(&d, "abris") {
            return Err(Erreur::Refus(m));
        }
        let regle = dossier_abris_regle(&app);
        if !regle.as_deref().is_some_and(|r| crate::garde::meme(&d, r)) {
            return Err(Erreur::Refus("Ce dossier n'est pas le dossier des abris réglé : règle-le d'abord dans Réglages ▸ Emplacements.".into()));
        }
        std::fs::create_dir_all(&d)?;
    }
    noyau.regler_abris(Some(d));
    // La copie (parfois des Go) se fait à part : la fenêtre ne se fige pas.
    tauri::async_runtime::spawn_blocking(move || app.state::<Noyau>().migrer_abris())
        .await
        .map_err(|_| Erreur::Disque("Le rangement des abris s'est arrêté brutalement.".into()))?
}

#[derive(Serialize)]
pub struct ProfilDetaille {
    #[serde(flatten)]
    pub profil: ProfilVisible,
    pub a_un_jeton: bool,
}

#[tauri::command]
pub fn profils_lister(noyau: State<'_, Noyau>) -> Resultat<Vec<ProfilDetaille>> {
    noyau
        .profils
        .lister()
        .into_iter()
        .map(|p| Ok(ProfilDetaille { a_un_jeton: noyau.a_un_jeton(&p.id)?, profil: p }))
        .collect()
}

#[derive(Serialize)]
pub struct ProfilCree {
    #[serde(flatten)]
    pub profil: ProfilVisible,
    /// Qui porte le jeton, d'après Firehouse (`/moi`) ; absent en mode simulé ou sans jeton.
    pub compte: Option<Value>,
}

/// Crée un profil. Hors mode simulé, le jeton est d'abord vérifié auprès de Firehouse (`/moi`) : un jeton refusé
/// n'est jamais rangé.
#[tauri::command]
pub async fn profil_creer(
    app: AppHandle,
    noyau: State<'_, Noyau>,
    nom: String,
    pin: Option<String>,
    jeton: Option<String>,
) -> Resultat<ProfilCree> {
    let c = connexion(&app);
    let jeton = jeton.map(|j| j.trim().to_string()).filter(|j| !j.is_empty());
    let compte = match (&jeton, c.simule) {
        (Some(j), false) => Some(Noyau::verifier_jeton(&c, j).await?),
        _ => None,
    };
    let profil = noyau.creer_profil(&nom, pin.as_deref(), jeton.as_deref())?;
    // Le compte attendu pour ce profil (voir `Profil::compte`).
    if let Some(cle) = compte.as_ref().and_then(crate::noyau::cle_compte) {
        noyau.profils.retenir_compte(&profil.id, &cle)?;
    }
    Ok(ProfilCree { profil, compte })
}

/// Qui porte le jeton du profil ouvert, d'après Firehouse.
#[tauri::command]
pub async fn profil_compte(noyau: State<'_, Noyau>) -> Resultat<Value> {
    noyau.compte().await
}

#[tauri::command]
pub async fn profil_ouvrir(
    app: AppHandle,
    noyau: State<'_, Noyau>,
    id: String,
    pin: Option<String>,
) -> Resultat<ProfilVisible> {
    noyau.ouvrir(&id, pin.as_deref(), &connexion(&app)).await
}

#[tauri::command]
pub async fn profil_fermer(noyau: State<'_, Noyau>) -> Resultat<()> {
    noyau.fermer().await;
    Ok(())
}

#[tauri::command]
pub async fn profil_actif(noyau: State<'_, Noyau>) -> Resultat<Option<ProfilVisible>> {
    Ok(noyau.actif().await)
}

/// Remplace le jeton du profil ouvert, après l'avoir vérifié auprès de Firehouse (hors mode simulé).
#[tauri::command]
pub async fn profil_changer_jeton(app: AppHandle, noyau: State<'_, Noyau>, jeton: String) -> Resultat<Option<Value>> {
    let c = connexion(&app);
    let compte = if c.simule { None } else { Some(Noyau::verifier_jeton(&c, &jeton).await?) };
    // Le compte du nouveau jeton (`username#grade`) : un autre que celui du cache vide d'abord ce qui avait été reçu
    // pour l'ancien ; inconnu (mode simulé, compte sans nom) : dans le doute, vidé aussi.
    let cle = compte.as_ref().and_then(crate::noyau::cle_compte);
    noyau.changer_jeton(&jeton, &c, cle.as_deref()).await?;
    Ok(compte)
}

#[tauri::command]
pub async fn profil_reconnecter(app: AppHandle, noyau: State<'_, Noyau>) -> Resultat<()> {
    noyau.reconnecter(&connexion(&app)).await
}

#[tauri::command]
pub async fn profil_changer_pin(
    noyau: State<'_, Noyau>,
    ancien: Option<String>,
    nouveau: Option<String>,
) -> Resultat<()> {
    let id = noyau.actif().await.ok_or_else(|| Erreur::Profil("Aucun profil n'est ouvert.".into()))?.id;
    noyau.profils.changer_pin(&id, ancien.as_deref(), nouveau.as_deref())
}

#[tauri::command]
pub async fn profil_renommer(noyau: State<'_, Noyau>, nom: String) -> Resultat<ProfilVisible> {
    let id = noyau.actif().await.ok_or_else(|| Erreur::Profil("Aucun profil n'est ouvert.".into()))?.id;
    noyau.profils.renommer(&id, &nom)?;
    Ok(ProfilVisible::from(&noyau.profils.trouver(&id)?))
}

#[tauri::command]
pub async fn profil_supprimer(app: AppHandle, noyau: State<'_, Noyau>, id: String, pin: Option<String>) -> Resultat<()> {
    let nom = noyau.profils.trouver(&id)?.nom;
    noyau.supprimer_profil(&id, pin.as_deref()).await?;
    app.state::<crate::nexus::Nexus>().oublier_profil(&id);
    // Sa connexion RetroAchievements ne reste pas dans les émulateurs.
    oublier_succes_partout(&app, &noyau, &nom);
    Ok(())
}

#[derive(Clone, Serialize)]
struct ProgresSynchro {
    page: u32,
    jeux: usize,
}

#[tauri::command]
pub async fn ludotheque_synchroniser(app: AppHandle, noyau: State<'_, Noyau>) -> Resultat<BilanSynchro> {
    let bilan = noyau
        .synchroniser(|page, jeux| {
            let _ = app.emit("synchro", ProgresSynchro { page, jeux });
        })
        .await?;
    // Le plafond des images gardées (réglage du PC, `plafondImagesGo`, 2 Go d'office) : jamais une raison d'échouer.
    if !bilan.simule {
        let go = app
            .store("pc.json")
            .ok()
            .and_then(|s| s.get("reglages"))
            .and_then(|r| r["plafondImagesGo"].as_f64())
            .filter(|g| *g > 0.0)
            .unwrap_or(2.0);
        if let Err(e) = noyau.plafonner_images((go * 1024.0 * 1024.0 * 1024.0) as u64).await {
            noyau.journaliser(&format!("plafond des images impossible : {e:?}"));
        }
    }
    Ok(bilan)
}

#[tauri::command]
pub async fn ludotheque_synchronisee_le(noyau: State<'_, Noyau>) -> Resultat<Option<String>> {
    noyau.synchronise_le().await
}

/// Les jeux possédés sur plusieurs boutiques, réunis en une carte (A6c) : les ids de leurs copies, la maîtresse
/// d'abord (pour demander laquelle lancer, et n'en montrer qu'une carte en Taodbox).
#[tauri::command]
pub async fn ludotheque_groupes(noyau: State<'_, Noyau>) -> Resultat<Vec<Vec<i64>>> {
    let s = noyau.session().await?;
    let c = s.verrou();
    c.groupes()
}

#[tauri::command]
pub async fn ludotheque_plateformes(noyau: State<'_, Noyau>, locale: Option<bool>) -> Resultat<Vec<Plateforme>> {
    noyau.plateformes(locale.unwrap_or(false)).await
}

/// La fiche d'un système (contrat 1.28), gardée pour le profil ouvert ; `None` : pas (encore) reçue.
#[tauri::command]
pub async fn plateforme_fiche(noyau: State<'_, Noyau>, nom: String) -> Resultat<Option<Value>> {
    noyau.fiche_systeme(&nom).await
}

#[tauri::command]
pub async fn ludotheque_genres(
    noyau: State<'_, Noyau>,
    plateforme: Option<String>,
    locale: Option<bool>,
) -> Resultat<Vec<String>> {
    noyau.genres(plateforme.as_deref(), locale.unwrap_or(false)).await
}

#[tauri::command]
pub async fn ludotheque_lister(noyau: State<'_, Noyau>, filtre: Filtre) -> Resultat<Liste> {
    noyau.lister(&filtre).await
}

#[tauri::command]
pub async fn ludotheque_au_hasard(
    noyau: State<'_, Noyau>,
    plateforme: Option<String>,
    locale: Option<bool>,
) -> Resultat<Option<JeuResume>> {
    noyau.au_hasard(plateforme.as_deref(), locale.unwrap_or(false)).await
}

#[tauri::command]
pub async fn ludotheque_fiche(noyau: State<'_, Noyau>, id: i64) -> Resultat<FicheLue> {
    noyau.fiche(id).await
}

/// Les infos d'un jeu du PC relues en direct (contrat 1.15).
#[tauri::command]
pub async fn ludotheque_infos(noyau: State<'_, Noyau>, id: i64) -> Resultat<Option<Value>> {
    noyau.infos_jeu(id).await
}

#[tauri::command]
pub async fn ludotheque_annexe_texte(noyau: State<'_, Noyau>, id: i64, i: u32, cle: String) -> Resultat<Value> {
    noyau.annexe_texte(id, i, &cle).await
}

/// Les skins de Firehouse (sans jeton : aussi sur l'écran « Qui joue ? »).
#[tauri::command]
pub async fn skins_obtenir(app: AppHandle, noyau: State<'_, Noyau>) -> Resultat<Option<Value>> {
    noyau.skins(&connexion(&app)).await
}

/// Enregistre le skin choisi dans le compte Firehouse de la personne du profil ouvert.
#[tauri::command]
pub async fn skin_enregistrer(noyau: State<'_, Noyau>, nom: String) -> Resultat<()> {
    noyau.enregistrer_skin(&nom).await
}

#[tauri::command]
pub async fn skin_personnel(noyau: State<'_, Noyau>) -> Resultat<Option<Value>> {
    noyau.skin_personnel().await
}

/// Les emplacements de jeux de ce PC, tels que l'interface les a enregistrés.
fn emplacements(app: &AppHandle) -> Emplacements {
    app.store("pc.json")
        .ok()
        .and_then(|s| s.get("reglages"))
        .and_then(|r| serde_json::from_value(r["emplacements"].clone()).ok())
        .unwrap_or_default()
}

/// Lance la file de téléchargements (si elle ne tourne pas déjà), avec le jeton du profil ouvert.
pub(crate) fn lancer_file(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let noyau = app.state::<Noyau>();
        let Ok(session) = noyau.session().await else { return };
        let emettre = |ev: crate::locale::Evenement| {
            let _ = app.emit("telechargement", ev);
        };
        if let Err(e) = noyau.executer_file(session, &emettre).await {
            noyau.journaliser(&format!("file de téléchargements : {e:?}"));
        }
    });
}

/// Les jeux du PC que le profil ouvert a le droit de voir, avec leur progression.
#[tauri::command]
pub async fn jeux_du_pc(noyau: State<'_, Noyau>) -> Resultat<Vec<JeuPcVu>> {
    noyau.jeux_du_pc().await
}

/// Les emplacements réglés pour un système, avec leur place libre, pour un jeu de cette taille.
#[tauri::command]
pub fn emplacements_proposer(
    app: AppHandle,
    noyau: State<'_, Noyau>,
    plateforme: String,
    taille: u64,
) -> Vec<EmplacementPropose> {
    noyau.proposer_emplacements(&emplacements(&app), &plateforme, taille)
}

/// Met un jeu dans la ludothèque de ce PC (médias gardés, fichiers en file), puis lance la file.
#[tauri::command]
pub async fn jeu_ajouter(
    app: AppHandle,
    noyau: State<'_, Noyau>,
    id: i64,
    version: i64,
    emplacement: String,
) -> Resultat<JeuPc> {
    let j = noyau.ajouter(id, version, &emplacement, &emplacements(&app)).await?;
    lancer_file(&app);
    Ok(j)
}

/// Retire UNE carte d'un jeu e-Reader du PC (ses fichiers seulement ; jamais la dernière carte, jamais les parties).
#[tauri::command]
pub async fn jeu_retirer_carte(noyau: State<'_, Noyau>, id: i64, version: i64) -> Resultat<JeuPc> {
    noyau.retirer_carte(id, version).await
}

#[tauri::command]
pub fn telechargement_pause(noyau: State<'_, Noyau>, id: i64) -> Resultat<()> {
    noyau.mettre_en_pause(id)
}

#[tauri::command]
pub fn telechargement_reprendre(app: AppHandle, noyau: State<'_, Noyau>, id: i64) -> Resultat<()> {
    noyau.reprendre(id)?;
    lancer_file(&app);
    Ok(())
}

#[tauri::command]
pub async fn telechargement_annuler(noyau: State<'_, Noyau>, id: i64) -> Resultat<()> {
    noyau.annuler(id).await
}

/// Ouvre un document gardé sur le PC (manuel…) avec le programme choisi dans Windows.
#[tauri::command]
pub async fn annexe_ouvrir(app: AppHandle, noyau: State<'_, Noyau>, id: i64, i: u32) -> Resultat<()> {
    let s = noyau.session().await?;
    if !s.verrou().contient(id)? {
        return Err(Erreur::Introuvable("Ce jeu ne t'est pas visible.".into()));
    }
    let chemin = noyau.annexe_fichier_locale(id, i).ok_or_else(|| {
        Erreur::Introuvable("Ce document n'est pas sur le PC : mets d'abord le jeu dans ta ludothèque.".into())
    })?;
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(chemin.to_string_lossy(), None::<&str>)
        .map_err(|_| Erreur::Disque("Windows n'a pas pu ouvrir ce document.".into()))
}

/// Les jeux du PC rattachés à une fiche réunie chez Firehouse (contrat 1.14) : (ancien, nouveau), pour que
/// l'interface fasse suivre les réglages faits sous l'ancien numéro.
#[tauri::command]
pub fn jeux_reunions(noyau: State<'_, Noyau>) -> Resultat<Vec<(i64, i64)>> {
    noyau.registre().reunions()
}

/// Les visuels d'un jeu (galerie de la fiche, contrat 1.6).
#[tauri::command]
pub async fn medias_jeu(noyau: State<'_, Noyau>, id: i64) -> Resultat<Vec<crate::medias::Media>> {
    noyau.medias(id).await
}

/// Les documents d'un jeu (contrat 1.7) : livret, musique, vidéos.
#[tauri::command]
pub async fn documents_jeu(noyau: State<'_, Noyau>, id: i64) -> Resultat<Vec<crate::documents::Document>> {
    noyau.documents(id).await
}

/// Ouvre le livret d'un jeu (le télécharge d'abord s'il n'est pas sur le PC : le clic de la personne, taille montrée,
/// vaut son accord) ; rend son nombre de pages. Les pages se lisent ensuite par `document.localhost`.
#[tauri::command]
pub async fn livret_ouvrir(app: AppHandle, noyau: State<'_, Noyau>, id: i64, doc: i64) -> Resultat<u32> {
    crate::documents::oublier_arret(id, doc);
    noyau.livret_suivi(id, doc, suivi_document(&app, id, doc)).await
}

#[derive(Clone, Serialize)]
struct ProgresDocument {
    jeu: i64,
    doc: i64,
    recus: u64,
    total: Option<u64>,
}

/// L'avancement du téléchargement d'un document (événement « document-progres »), au plus un envoi toutes les 300 ms
/// (le dernier passe toujours).
fn suivi_document(app: &AppHandle, jeu: i64, doc: i64) -> crate::documents::Progres {
    let app = app.clone();
    let dernier = std::sync::Mutex::new(None::<std::time::Instant>);
    std::sync::Arc::new(move |recus, total| {
        let mut d = dernier.lock().unwrap_or_else(|e| e.into_inner());
        let fin = total.is_some_and(|t| recus >= t);
        if !fin && d.is_some_and(|t| t.elapsed() < std::time::Duration::from_millis(300)) {
            return;
        }
        *d = Some(std::time::Instant::now());
        let _ = app.emit("document-progres", ProgresDocument { jeu, doc, recus, total });
    })
}

/// Arrête le téléchargement d'un document (livret, musique, vidéo) : ce qui est reçu est gardé pour la reprise.
#[tauri::command]
pub fn document_arreter(id: i64, doc: i64) {
    crate::documents::arreter(id, doc);
}

/// Prépare une musique ou une vidéo du jeu (la télécharge d'abord si besoin : le clic de la personne, taille
/// montrée, vaut son accord) ; elle se lit ensuite par `document.localhost/<jeu>/<document>/fichier`.
#[tauri::command]
pub async fn document_preparer(app: AppHandle, noyau: State<'_, Noyau>, id: i64, doc: i64) -> Resultat<crate::documents::Document> {
    crate::documents::oublier_arret(id, doc);
    noyau.preparer_document_suivi(id, doc, suivi_document(&app, id, doc)).await
}

/// La place libre dans un dossier (`None` : introuvable), pour l'écran des emplacements.
#[tauri::command]
pub fn espace_libre(chemin: String) -> Option<u64> {
    crate::jeux_pc::place_libre(std::path::Path::new(&chemin))
}

/// Un dossier proposé d'office (rien n'est encore réglé) : où, et la place libre sur son disque.
#[derive(Serialize)]
pub struct DossierPropose {
    pub chemin: String,
    /// Le disque (`D:\`) : le dossier des émulateurs proposé ira sur le même.
    pub disque: String,
    pub libre: u64,
    /// Tous les disques fixes de ce PC (`C:\`, `D:\`…) : une lettre venue d'un autre PC n'est reprise que si elle existe.
    pub disques: Vec<String>,
    /// Le disque proposé est externe (USB) : il n'y en a pas d'autre ; c'est dit à la personne.
    pub externe: bool,
}

/// Le dossier des jeux proposé quand aucun n'est réglé (PC neuf, ancien dossier absent après une restauration) :
/// `<disque fixe le plus libre>\Frogtend\Jeux`. Rien n'est créé ici.
#[tauri::command]
pub fn dossier_jeux_propose() -> Option<DossierPropose> {
    let l = crate::jeux_pc::disques_locaux();
    let d = crate::jeux_pc::disque_propose(&l)?;
    Some(DossierPropose {
        chemin: crate::jeux_pc::dossier_jeux_sur(d).to_string_lossy().into(),
        disque: d.racine.clone(),
        libre: d.libre,
        disques: l.iter().map(|x| x.racine.clone()).collect(),
        externe: d.externe,
    })
}

/// Le bouton Xbox ouvre-t-il la Game Bar de Windows ? (`None` : hors de Windows.)
#[tauri::command]
pub fn game_bar_bouton() -> Option<bool> {
    crate::registre::bouton_game_bar()
}

/// Change CE seul réglage de Windows, après le oui de la personne (fenêtre de l'interface) ; noté au journal.
#[tauri::command]
pub fn game_bar_bouton_regler(noyau: State<'_, Noyau>, actif: bool) -> Resultat<()> {
    let avant = crate::registre::bouton_game_bar();
    crate::registre::regler_bouton_game_bar(actif).map_err(Erreur::Refus)?;
    noyau.journaliser(&format!("Windows : bouton Xbox → Game Bar {} (avant : {avant:?}), avec l'accord de la personne", if actif { "réactivé" } else { "désactivé" }));
    Ok(())
}

/// Un dossier choisi pour y RANGER des jeux : refusé (et pourquoi, avec un sous-dossier proposé) s'il est la racine
/// d'un disque, Windows, Program Files ou un dossier de base de l'utilisateur (Documents, Bureau…). `None` : admis.
/// `sorte` : `emulateurs`, `outils` ou `abris` pour les autres dossiers de Frogtend (même règle), sinon les jeux.
#[tauri::command]
pub fn dossier_jeux_refus(chemin: String, sorte: Option<String>) -> Option<String> {
    refus_dossier(std::path::Path::new(chemin.trim()), sorte.as_deref().unwrap_or("jeux"))
}

pub(crate) fn refus_dossier_jeux(p: &std::path::Path) -> Option<String> {
    refus_dossier(p, "jeux")
}

/// La règle des dossiers de Frogtend (jeux, émulateurs, outils, abris) : jamais la racine d'un disque, Windows,
/// Program Files ni un dossier de base de l'utilisateur. Le cœur la revérifie à l'usage (l'interface peut se tromper).
pub(crate) fn refus_dossier(p: &std::path::Path, sorte: &str) -> Option<String> {
    if !crate::garde::dossier_systeme(p) {
        return None;
    }
    let (quoi, sous_dossier) = match sorte {
        "emulateurs" => ("des émulateurs", "Emulateurs"),
        "outils" => ("des outils", "Outils"),
        "abris" => ("des abris de parties", "Abris de parties"),
        _ => ("des jeux", "Jeux"),
    };
    if !p.is_absolute() {
        return Some(format!("« {} » n'est pas un chemin complet : donne-le en entier, par exemple « D:\\{sous_dossier} ».", p.display()));
    }
    // Un sous-dossier proposé qui convient lui-même (jamais sous Windows) ; sinon, pour les jeux, le dossier que
    // Frogtend propose d'office.
    let candidat = p.join(sous_dossier);
    let candidat = if !crate::garde::dossier_systeme(&candidat) {
        Some(candidat)
    } else if sous_dossier == "Jeux" {
        crate::jeux_pc::disque_propose(&crate::jeux_pc::disques_locaux()).map(crate::jeux_pc::dossier_jeux_sur)
    } else {
        None
    };
    let propose = candidat.map(|c| format!(" — par exemple « {} »", c.display())).unwrap_or_default();
    Some(format!(
        "« {} » ne convient pas pour ranger {quoi} (la racine d'un disque, un dossier de Windows, ou un dossier de base comme Documents ou le Bureau). Choisis ou crée un sous-dossier{propose}.",
        p.display()
    ))
}

/// Crée le dossier des jeux PROPOSÉ (la personne a dit oui). Seul un dossier `<disque fixe>\Frogtend\Jeux` est admis :
/// jamais un chemin quelconque venu de l'interface.
#[tauri::command]
pub fn dossier_jeux_creer(chemin: String) -> Resultat<String> {
    let admis = crate::jeux_pc::disques_locaux().iter().map(crate::jeux_pc::dossier_jeux_sur).any(|p| p.to_string_lossy().eq_ignore_ascii_case(&chemin));
    if !admis {
        return Err(Erreur::Refus("Ce dossier n'est pas celui que Frogtend a proposé : choisis-le avec « 📂 Choisir un autre dossier ».".into()));
    }
    std::fs::create_dir_all(&chemin)?;
    if !std::path::Path::new(&chemin).is_dir() {
        return Err(Erreur::Disque(format!("Le dossier {chemin} n'a pas pu être créé.")));
    }
    Ok(chemin)
}

/// L'émulateur réglé pour un système (`pc.json` ▸ `emulateurs`) : (programme, ligne de commande).
/// Parmi ceux réglés pour le système : celui demandé pour ce lancement, sinon le défaut du jeu, sinon celui du système.
fn emulateur_regle(app: &AppHandle, plateforme: &str, jeu: i64, demande: Option<&str>) -> Option<(String, String)> {
    let r = app.store("pc.json").ok()?.get("reglages")?;
    let e = crate::choix_emulateur::pour_le_jeu(&r, plateforme, jeu, demande)?;
    Some((e.programme, e.ligne))
}

/// Un contenu additionnel vu depuis le panneau d'un jeu.
#[derive(Serialize)]
pub struct ContenuVu {
    pub id: String,
    pub nom: String,
    pub genre: String,
    pub taille: u64,
    pub licence: bool,
    /// Installé (et sa licence posée dans le compte de CE profil s'il en a une) ; Switch : son dossier est lu par Eden.
    pub installe: bool,
    /// Switch : le dossier qui sera ajouté à Eden (tout son contenu devient visible).
    pub dossier: Option<String>,
    /// Une mise à jour PS3 : sa version (« 01.05 », contrat 1.31), dans l'ordre d'installation.
    pub version: Option<String>,
}

/// Les contenus additionnels d'un jeu du PC.
#[derive(Serialize, Default)]
pub struct ContenusDuJeu {
    /// `aucun` (console pas encore gérée), `emulateur` (pas d'émulateur réglé), `ok`.
    pub etat: String,
    /// `ps3` (RPCS3 installe chaque paquet) ou `switch` (Eden lit un dossier).
    pub systeme: String,
    pub titre_id: Option<String>,
    pub contenus: Vec<ContenuVu>,
    /// Les fichiers servis comme contenus mais illisibles (en-tête abîmé) : dits, jamais écartés en silence.
    #[serde(default)]
    pub ignores: Vec<String>,
}

/// Image et fluidité d'un jeu PS3 dans RPCS3 : son RPCS3 réglé (jamais reçu de l'interface) et son numéro (lu sur le
/// disque, vérifié). `None` : pas un jeu PS3 lancé par RPCS3.
/// Un jeu PS3 lancé par RPCS3 : (programme, dossier de RPCS3, numéro du disque vérifié, version que RPCS3 lancera).
type ContextePs3 = (std::path::PathBuf, std::path::PathBuf, Option<String>, Option<String>);

async fn contexte_image_ps3(app: &AppHandle, noyau: &Noyau, id: i64) -> Resultat<Option<ContextePs3>> {
    let j = noyau.jeu_visible(id).await?;
    if crate::succes::console_de(&j.plateforme) != Some(82) {
        return Ok(None);
    }
    let Some((programme, _)) = emulateur_regle(app, &j.plateforme, id, None) else { return Ok(None) };
    let programme = std::path::PathBuf::from(programme);
    if !programme.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("rpcs3.exe")) {
        return Ok(None);
    }
    let rpcs3 = programme.parent().map(std::path::PathBuf::from).unwrap_or_default();
    let fichier_du_jeu = j.installation.as_ref().map(|i| match &i.fichier_du_jeu {
        Some(f) => std::path::Path::new(&i.dossier).join(f),
        None => std::path::PathBuf::from(&i.dossier),
    });
    let r = rpcs3.clone();
    let (numero, version) = tauri::async_runtime::spawn_blocking(move || {
        let f = fichier_du_jeu?;
        let n = crate::contenus::titre_id_ps3(&f).filter(|n| crate::emulateurs_profils::numero_ps3_sur(n))?;
        let v = crate::contenus::version_ps3(&f, &r, &n);
        Some((n, v))
    })
    .await
    .map_err(|_| Erreur::Disque("La lecture du disque du jeu s'est arrêtée brutalement.".into()))?
    .map_or((None, None), |(n, v)| (Some(n), v));
    Ok(Some((programme, rpcs3, numero, version)))
}

/// Les réglages d'image et de fluidité d'un jeu PS3 (RPCS3).
#[derive(Serialize)]
pub struct ImagePs3 {
    /// Le numéro du jeu ; `None` : illisible (rien ne peut être réglé pour ce jeu).
    numero: Option<String>,
    /// (clé, valeur actuelle) : celle du jeu, sinon de RPCS3.
    valeurs: Vec<(String, String)>,
    /// Les clés réglées pour CE jeu.
    par_jeu: Vec<String>,
    /// Le cache de shaders sur le disque est coupé dans RPCS3 (pour ce jeu ou pour tous) : à dire, sans y toucher.
    cache_coupe: bool,
}

/// Image et fluidité d'un jeu PS3 : ses réglages actuels ; `None` : pas un jeu PS3 lancé par RPCS3.
#[tauri::command]
pub async fn rpcs3_image(app: AppHandle, noyau: State<'_, Noyau>, id: i64) -> Resultat<Option<ImagePs3>> {
    let Some((_, rpcs3, numero, _)) = contexte_image_ps3(&app, &noyau, id).await? else { return Ok(None) };
    let Some(n) = numero else { return Ok(Some(ImagePs3 { numero: None, valeurs: vec![], par_jeu: vec![], cache_coupe: false })) };
    let (valeurs, par_jeu) = crate::emulateurs_profils::rpcs3_image(&rpcs3, &n)?;
    // Un simple avertissement : un fichier illisible ici ne ferme jamais l'écran.
    let cache_coupe = crate::emulateurs_profils::rpcs3_cache_shaders_coupe(&rpcs3, &n).unwrap_or(false);
    Ok(Some(ImagePs3 { numero: Some(n), valeurs, par_jeu, cache_coupe }))
}

/// Règle l'image et la fluidité d'un jeu PS3 (seulement les clés données) ; `vide` : revenir aux valeurs de RPCS3.
/// RPCS3 fermé ; son programme de confiance ; fichier du jeu copié à l'abri.
#[tauri::command]
pub async fn rpcs3_regler_image(app: AppHandle, noyau: State<'_, Noyau>, id: i64, valeurs: Vec<(String, String)>, vide: bool) -> Resultat<()> {
    let (programme, rpcs3, numero, _) =
        contexte_image_ps3(&app, &noyau, id).await?.ok_or_else(|| Erreur::Refus("Ce jeu ne se lance pas avec RPCS3.".into()))?;
    let n = numero.ok_or_else(|| Erreur::Refus("Frogtend ne reconnaît pas le disque de ce jeu : rien n'est réglé.".into()))?;
    exiger_emulateur_pour_ecrire(&app, &noyau, &programme)?;
    rpcs3_ferme(&rpcs3)?;
    crate::emulateurs_profils::rpcs3_regler_image(&rpcs3, &n, &valeurs, vide)?;
    noyau.journaliser(&format!("RPCS3 : image du jeu {n} réglée ({} réglage(s){})", valeurs.len(), if vide { ", valeurs de RPCS3" } else { "" }));
    Ok(())
}

// --- Patchs PS3 par Firehouse (contrat 1.18 ; ✅ Seb 19/10). Les patchs proposés et activés viennent TOUJOURS de la
// liste de Firehouse pour ce jeu (jamais d'un texte envoyé par l'interface) ; la liste de RPCS3 n'est remplacée
// qu'après un accord, sha256 vérifié. ---

/// Un patch d'un jeu PS3, tel que l'écran le montre.
#[derive(Serialize)]
pub struct PatchPs3 {
    hash: String,
    description: String,
    titre: String,
    auteur: String,
    notes: String,
    /// Contrat 1.20 : la traduction de Firehouse (`None` : pas encore traduite ; l'écran montre l'anglais, marqué).
    /// Seulement à l'écran : `description` (l'anglais d'origine) reste ce que RPCS3 attend dans son fichier.
    description_fr: Option<String>,
    notes_fr: Option<String>,
    actif: bool,
}

/// Ce que l'écran montre d'un patch en plus de ce que RPCS3 range (auteur, notes, traductions du contrat 1.20).
struct InfosPatch {
    auteur: String,
    notes: String,
    description_fr: Option<String>,
    notes_fr: Option<String>,
}

/// Les patchs d'un jeu PS3, et la liste de RPCS3 (présente, à jour, sa taille).
#[derive(Serialize)]
pub struct PatchsPs3 {
    numero: Option<String>,
    version: Option<String>,
    patchs: Vec<PatchPs3>,
    liste_presente: bool,
    liste_a_jour: bool,
    taille_liste: u64,
    /// Firehouse injoignable : seules les retouches actives sont montrées (pour les désactiver).
    hors_ligne: bool,
}

/// Les patchs de Firehouse pour ce jeu qui valent pour SA version (avec la clé de version de RPCS3), leur auteur et
/// leurs notes.
async fn patchs_firehouse(source: &crate::source::Source, numero: &str, version: Option<&str>) -> Resultat<Vec<(crate::patchs_ps3::Patch, InfosPatch)>> {
    let v = source.patchs_rpcs3_jeu(numero, version).await?;
    let liste = v["patchs"].as_array().cloned().unwrap_or_default();
    Ok(liste
        .iter()
        .filter_map(|p| {
            let texte = |k: &str| p[k].as_str().unwrap_or("").to_string();
            let cle = crate::patchs_ps3::cle_version(&p["versions_du_jeu"], version)?;
            let patch = crate::patchs_ps3::Patch { hash: texte("hash"), description: texte("description"), titre: texte("titre"), numero: numero.to_string(), version: cle };
            let fr = |k: &str| p[k].as_str().map(str::to_string);
            let infos = InfosPatch { auteur: texte("auteur"), notes: texte("notes"), description_fr: fr("description_fr"), notes_fr: fr("notes_fr") };
            crate::patchs_ps3::hash_sur(&patch.hash).then_some((patch, infos))
        })
        .collect())
}

/// La liste de Firehouse est-elle celle posée dans RPCS3 ? (présente, à jour, taille annoncée, état reçu)
async fn etat_liste_patchs(source: &crate::source::Source, rpcs3: &std::path::Path) -> Resultat<(bool, bool, u64, Value)> {
    let etat = source.patchs_rpcs3_etat().await?;
    let r = rpcs3.to_path_buf();
    let locale = tauri::async_runtime::spawn_blocking(move || crate::patchs_ps3::empreinte_liste(&r)).await.unwrap_or(None);
    let annoncee = etat["sha256"].as_str().unwrap_or("").to_ascii_lowercase();
    let a_jour = locale.as_deref().is_some_and(|l| !annoncee.is_empty() && l.eq_ignore_ascii_case(&annoncee));
    Ok((locale.is_some(), a_jour, etat["taille"].as_u64().unwrap_or(0), etat))
}

/// Les patchs d'un jeu PS3 ; `None` : pas un jeu PS3 lancé par RPCS3. Firehouse injoignable (ou sa liste
/// indisponible) : seules les retouches ACTIVES, lues dans RPCS3, pour pouvoir les désactiver (`hors_ligne`). Une
/// retouche active que Firehouse ne propose plus est montrée aussi (sans auteur ni remarque), pour la même raison.
#[tauri::command]
pub async fn rpcs3_patchs(app: AppHandle, noyau: State<'_, Noyau>, id: i64) -> Resultat<Option<PatchsPs3>> {
    let Some((_, rpcs3, numero, version)) = contexte_image_ps3(&app, &noyau, id).await? else { return Ok(None) };
    let Some(n) = numero else {
        return Ok(Some(PatchsPs3 { numero: None, version, patchs: vec![], liste_presente: false, liste_a_jour: false, taille_liste: 0, hors_ligne: false }));
    };
    let r = rpcs3.clone();
    // Illisible : aucune active (l'écran s'ouvre quand même ; une écriture serait refusée plus loin).
    let config = tauri::async_runtime::spawn_blocking(move || crate::patchs_ps3::lire_config(&r)).await.ok().and_then(Result::ok).unwrap_or_default();
    let actifs = crate::patchs_ps3::actives(&config, &n);
    let session = noyau.session().await?;
    let firehouse = match etat_liste_patchs(&session.source, &rpcs3).await {
        Ok(e) => patchs_firehouse(&session.source, &n, version.as_deref()).await.map(|p| (e, p)),
        Err(e) => Err(e),
    };
    let ((liste_presente, liste_a_jour, taille_liste, _), proposes, hors_ligne) = match firehouse {
        Ok((e, p)) => (e, p, false),
        Err(Erreur::JetonRefuse(m)) => return Err(Erreur::JetonRefuse(m)),
        Err(_) => ((false, false, 0, Value::Null), Vec::new(), true),
    };
    let mut patchs: Vec<PatchPs3> = proposes
        .into_iter()
        .map(|(p, i)| PatchPs3 {
            actif: actifs.contains(&p),
            hash: p.hash,
            description: p.description,
            titre: p.titre,
            auteur: i.auteur,
            notes: i.notes,
            description_fr: i.description_fr,
            notes_fr: i.notes_fr,
        })
        .collect();
    for a in actifs {
        if !patchs.iter().any(|p| p.hash == a.hash && p.description == a.description && p.titre == a.titre) {
            patchs.push(PatchPs3 {
                actif: true,
                hash: a.hash,
                description: a.description,
                titre: a.titre,
                auteur: String::new(),
                notes: String::new(),
                description_fr: None,
                notes_fr: None,
            });
        }
    }
    Ok(Some(PatchsPs3 { numero: Some(n), version, patchs, liste_presente, liste_a_jour, taille_liste, hors_ligne }))
}

/// Active ou désactive un patch d'un jeu PS3. Pour l'ACTIVER, le patch est repris de la liste de Firehouse pour ce
/// jeu (hash, description, titre), et la liste de RPCS3 doit être celle de Firehouse : `liste` = la personne a accepté
/// de la poser (taille annoncée), sinon refus. Pour le DÉSACTIVER, il suffit qu'il soit actif dans RPCS3 (sans
/// réseau, même retiré de la liste de Firehouse).
#[tauri::command]
// Une commande de l'interface : le patch désigné par ses trois noms, l'accord pour la liste (Tauri).
#[allow(clippy::too_many_arguments)]
pub async fn rpcs3_patch_regler(
    app: AppHandle,
    noyau: State<'_, Noyau>,
    id: i64,
    hash: String,
    description: String,
    titre: String,
    actif: bool,
    liste: bool,
) -> Resultat<()> {
    let (programme, rpcs3, numero, version) =
        contexte_image_ps3(&app, &noyau, id).await?.ok_or_else(|| Erreur::Refus("Ce jeu ne se lance pas avec RPCS3.".into()))?;
    let n = numero.ok_or_else(|| Erreur::Refus("Frogtend ne reconnaît pas le disque de ce jeu : rien n'est réglé.".into()))?;
    exiger_emulateur_pour_ecrire(&app, &noyau, &programme)?;
    rpcs3_ferme(&rpcs3)?;
    let meme = |p: &crate::patchs_ps3::Patch| p.hash == hash && p.description == description && p.titre == titre;
    let patch = if actif {
        let session = noyau.session().await?;
        let patch = patchs_firehouse(&session.source, &n, version.as_deref())
            .await?
            .into_iter()
            .map(|(p, _)| p)
            .find(|p| meme(p))
            .ok_or_else(|| Erreur::Refus("Ce patch n'est plus proposé par Firehouse pour ce jeu : rien n'est changé.".into()))?;
        let (_, a_jour, _, etat) = etat_liste_patchs(&session.source, &rpcs3).await?;
        if !a_jour {
            if !liste {
                return Err(Erreur::Refus("La liste des patchs de RPCS3 doit d'abord être installée.".into()));
            }
            // RPCS3 a pu être ouvert pendant l'attente du réseau.
            rpcs3_ferme(&rpcs3)?;
            match crate::patchs_ps3::installer_liste(&session.source, &rpcs3, &etat).await {
                Ok(t) => noyau.journaliser(&format!("RPCS3 : liste des patchs installée depuis Firehouse ({t} octets, sha256 vérifié)")),
                Err(e) => {
                    noyau.journaliser(&format!("RPCS3 : liste des patchs refusée ({e})"));
                    return Err(e);
                }
            }
        }
        patch
    } else {
        let r = rpcs3.clone();
        let config = tauri::async_runtime::spawn_blocking(move || crate::patchs_ps3::lire_config(&r))
            .await
            .map_err(|_| Erreur::Disque("La lecture des patchs s'est arrêtée brutalement.".into()))??;
        crate::patchs_ps3::actives(&config, &n)
            .into_iter()
            .find(|p| meme(p))
            .ok_or_else(|| Erreur::Refus("Ce patch n'est pas activé pour ce jeu : rien n'est changé.".into()))?
    };
    rpcs3_ferme(&rpcs3)?;
    let r = rpcs3.clone();
    let p = patch.clone();
    let change = tauri::async_runtime::spawn_blocking(move || -> Resultat<bool> {
        let avant = crate::patchs_ps3::lire_config(&r)?;
        match crate::patchs_ps3::regler(&avant, &p, actif)? {
            Some(apres) => crate::patchs_ps3::ecrire_config(&r, &apres).map(|_| true),
            None => Ok(false),
        }
    })
    .await
    .map_err(|_| Erreur::Disque("L'écriture des patchs s'est arrêtée brutalement.".into()))??;
    if change {
        noyau.journaliser(&format!("RPCS3 : patch « {} » {} pour le jeu {n} ({})", patch.description, if actif { "activé" } else { "désactivé" }, patch.version));
    }
    Ok(())
}

/// Ce qu'il faut pour les contenus PS3 d'un jeu : le jeu, son RPCS3, le compte RPCS3 du profil, les contenus trouvés.
async fn contexte_contenus(
    app: &AppHandle,
    noyau: &Noyau,
    id: i64,
) -> Resultat<Option<(std::path::PathBuf, Option<String>, Option<String>, Vec<crate::contenus::Contenu>, Vec<String>)>> {
    let p = profil_ouvert(noyau).await?;
    // Seulement un jeu que ce profil voit (comme partout ailleurs).
    let j = noyau.jeu_visible(id).await?;
    if crate::succes::console_de(&j.plateforme) != Some(82) {
        return Ok(None);
    }
    let Some((programme, _)) = emulateur_regle(app, &j.plateforme, id, None) else { return Err(Erreur::Reglage("Aucun RPCS3 réglé pour la PS3.".into())) };
    let rpcs3 = std::path::Path::new(&programme).parent().map(std::path::PathBuf::from).unwrap_or_default();
    // Le compte RPCS3 de ce profil, s'il existe déjà (créé seulement quand on installe : regarder n'écrit rien). Même
    // nom que celui du lancement (nom de dossier du profil).
    let compte = crate::emulateurs_profils::compte_rpcs3_existant(&rpcs3, &crate::jeux_pc::nom_de_dossier(&p.nom));
    // Où chercher : le dossier du jeu, son installation, et les emplacements de la PS3.
    let mut dossiers: Vec<std::path::PathBuf> = vec![std::path::PathBuf::from(&j.dossier)];
    let mut fichier_du_jeu = None;
    if let Some(i) = &j.installation {
        dossiers.push(std::path::PathBuf::from(&i.dossier));
        fichier_du_jeu = Some(match &i.fichier_du_jeu {
            Some(f) => std::path::Path::new(&i.dossier).join(f),
            None => std::path::PathBuf::from(&i.dossier),
        });
    }
    let e = emplacements(app);
    dossiers.extend(e.systemes.get(&j.plateforme).cloned().unwrap_or_default().into_iter().map(std::path::PathBuf::from));
    dossiers.sort();
    dossiers.dedup();
    let titre = j.titre.clone();
    // Une version de Firehouse servie avec ses contenus (contrat 1.24) : SES fichiers aux rôles `dlc` et `licence`, sans
    // deviner ni trier (ils sont à ce jeu). Hors ligne (pas de fiche gardée) : la recherche ordinaire ci-dessous.
    let version = noyau.version_de(&j);
    if j.installation.is_some() && crate::partie::contenus_emulateur(&version) {
        let recus = std::path::PathBuf::from(&j.dossier);
        let presents = |role: &str| -> Vec<std::path::PathBuf> {
            crate::partie::roles(&version, &[role])
                .iter()
                .filter_map(|n| crate::jeux_pc::chemin_de_fichier_sur(n).ok())
                .map(|r| recus.join(r))
                .filter(|p| p.is_file() && !crate::garde::lien_entre(&recus, p))
                .collect()
        };
        // Contrat 1.31 : la version de chaque mise à jour (« 01.05 »), dans l'ordre servi (celui de l'installation).
        let versions: std::collections::HashMap<std::path::PathBuf, String> = version["fichiers"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|f| f["role"] == "maj")
            .filter_map(|f| {
                let r = crate::jeux_pc::chemin_de_fichier_sur(f["nom"].as_str()?).ok()?;
                Some((recus.join(r), f["version"].as_str().filter(|v| !v.is_empty())?.to_string()))
            })
            .collect();
        let pkgs: Vec<(std::path::PathBuf, &str, Option<String>)> = presents("maj")
            .into_iter()
            .map(|p| {
                let v = versions.get(&p).cloned();
                (p, "maj", v)
            })
            .chain(presents("dlc").into_iter().map(|p| (p, "dlc", None)))
            .collect();
        let raps = presents("licence");
        let (contenus, titre_id, ignores) = tauri::async_runtime::spawn_blocking(move || {
            let simples: Vec<(std::path::PathBuf, &str)> = pkgs.iter().map(|(p, g, _)| (p.clone(), *g)).collect();
            (
                crate::contenus::ps3_avec_versions(&pkgs, &raps),
                fichier_du_jeu.as_deref().and_then(crate::contenus::titre_id_ps3),
                crate::contenus::illisibles(&simples),
            )
        })
        .await
        .map_err(|_| Erreur::Disque("La lecture s'est arrêtée brutalement.".into()))?;
        return Ok(Some((rpcs3, compte, titre_id, contenus, ignores)));
    }
    let (contenus, titre_id) = tauri::async_runtime::spawn_blocking(move || {
        let mut l: Vec<crate::contenus::Contenu> = dossiers.iter().filter(|d| d.is_dir()).flat_map(|d| crate::contenus::chercher_ps3(d)).collect();
        l.sort_by(|a, b| a.id.cmp(&b.id));
        l.dedup_by(|a, b| a.id == b.id);
        let titre_id = fichier_du_jeu.as_deref().and_then(crate::contenus::titre_id_ps3);
        let du_jeu: Vec<crate::contenus::Contenu> = crate::contenus::du_jeu(&l, titre_id.as_deref(), &titre).into_iter().cloned().collect();
        (du_jeu, titre_id)
    })
    .await
    .map_err(|_| Erreur::Disque("La recherche s'est arrêtée brutalement.".into()))?;
    // Recherche sur le disque (jeu importé) : n'importe quel fichier peut s'y trouver, rien à signaler.
    Ok(Some((rpcs3, compte, titre_id, contenus, vec![])))
}

/// Switch : les mises à jour et DLC d'un jeu trouvés sur ce PC, et l'Eden qui les lira.
async fn contexte_switch(app: &AppHandle, noyau: &Noyau, id: i64) -> Resultat<Option<(std::path::PathBuf, Vec<crate::contenus::ContenuSwitch>)>> {
    let j = noyau.jeu_visible(id).await?;
    if !j.plateforme.eq_ignore_ascii_case("Nintendo Switch") {
        return Ok(None);
    }
    let Some((programme, _)) = emulateur_regle(app, &j.plateforme, id, None) else { return Err(Erreur::Reglage("Aucun Eden réglé pour la Switch.".into())) };
    // Seul Eden lit une liste de dossiers de contenus (`qt-config.ini`) : un autre émulateur Switch (Ryubing, Citron…)
    // n'y verrait rien, et Frogtend écrirait un fichier inutile chez lui.
    let p = std::path::Path::new(&programme);
    if !p.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("eden.exe")) {
        // Un RÉGLAGE à changer (pas un refus) : l'aperçu montre alors « Règle Eden… » au lieu de rien.
        return Err(Erreur::Reglage("Frogtend sait donner les mises à jour et DLC à Eden seulement : choisis Eden comme émulateur de la Switch pour ce jeu.".into()));
    }
    let eden = p.parent().map(std::path::PathBuf::from).unwrap_or_default();
    let mut dossiers: Vec<std::path::PathBuf> = vec![std::path::PathBuf::from(&j.dossier)];
    let mut fichier = None;
    if let Some(i) = &j.installation {
        dossiers.push(std::path::PathBuf::from(&i.dossier));
        fichier = i.fichier_du_jeu.as_ref().map(|f| std::path::Path::new(&i.dossier).join(f));
    }
    let e = emplacements(app);
    dossiers.extend(e.systemes.get(&j.plateforme).cloned().unwrap_or_default().into_iter().map(std::path::PathBuf::from));
    dossiers.sort();
    dossiers.dedup();
    let titre = j.titre.clone();
    let l = tauri::async_runtime::spawn_blocking(move || {
        let mut l: Vec<crate::contenus::ContenuSwitch> = dossiers.iter().filter(|d| d.is_dir()).flat_map(|d| crate::contenus::chercher_switch(d)).collect();
        l.sort_by(|a, b| a.chemin.cmp(&b.chemin));
        l.dedup_by(|a, b| a.chemin == b.chemin);
        // L'identifiant du jeu : celui de son fichier s'il le porte (un .xci ne le dit pas sans les clés de la console).
        let id_jeu = fichier.as_deref().and_then(crate::contenus::id_switch).map(|i| crate::contenus::classer_switch(i).1);
        crate::contenus::switch_du_jeu(&l, id_jeu, &titre).into_iter().cloned().collect::<Vec<_>>()
    })
    .await
    .map_err(|_| Erreur::Disque("La recherche s'est arrêtée brutalement.".into()))?;
    Ok(Some((eden, l)))
}

/// Les contenus additionnels (DLC, avatars…) d'un jeu, disponibles sur ce PC, et ce qui est déjà installé pour ce profil.
#[tauri::command]
pub async fn contenus_du_jeu(app: AppHandle, noyau: State<'_, Noyau>, id: i64) -> Resultat<ContenusDuJeu> {
    // Wii U : un .wua contient déjà ses mises à jour et DLC (Cemu les lit) : on les montre, rien à installer.
    // Lu à part : un `if let` garderait le verrou du registre pendant les attentes qui suivent.
    let jeu = Some(noyau.jeu_visible(id).await?);
    if let Some(j) = jeu.filter(|j| j.plateforme.eq_ignore_ascii_case("Nintendo Wii U")) {
        let wua = j.installation.as_ref().and_then(|i| i.fichier_du_jeu.as_ref().map(|f| std::path::Path::new(&i.dossier).join(f)));
        let Some(wua) = wua.filter(|w| w.extension().is_some_and(|e| e.eq_ignore_ascii_case("wua"))) else {
            return Ok(ContenusDuJeu { etat: "aucun".into(), ..Default::default() });
        };
        let titres = tauri::async_runtime::spawn_blocking(move || crate::contenus::titres_wua(&wua))
            .await
            .map_err(|_| Erreur::Disque("La lecture s'est arrêtée brutalement.".into()))??;
        return Ok(ContenusDuJeu {
            etat: "ok".into(),
            systeme: "wiiu".into(),
            titre_id: None,
            ignores: vec![],
            contenus: titres
                .into_iter()
                .filter(|t| t.genre != "jeu")
                .map(|t| ContenuVu {
                    version: None,
                    nom: format!("{} v{}", if t.genre == "maj" { "Mise à jour" } else { "DLC" }, t.version),
                    id: format!("{}_v{}", t.id, t.version),
                    genre: t.genre,
                    taille: 0,
                    licence: false,
                    installe: true,
                    dossier: None,
                })
                .collect(),
        });
    }
    match contexte_switch(&app, &noyau, id).await {
        Ok(Some((eden, l))) => {
            let lus = crate::contenus::dossiers_externes_eden(&std::fs::read_to_string(crate::contenus::ini_eden(&eden)).unwrap_or_default());
            let parent = |c: &str| std::path::Path::new(c).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
            return Ok(ContenusDuJeu {
                etat: "ok".into(),
                systeme: "switch".into(),
                titre_id: None,
                ignores: vec![],
                contenus: l
                    .into_iter()
                    .map(|c| {
                        let d = parent(&c.chemin);
                        ContenuVu { version: None, installe: lus.iter().any(|x| crate::contenus::meme_dossier(x, &d)), licence: false, id: c.id, nom: c.nom, genre: c.genre, taille: c.taille, dossier: Some(d) }
                    })
                    .collect(),
            });
        }
        Ok(None) => {}
        Err(Erreur::Reglage(_)) => return Ok(ContenusDuJeu { etat: "emulateur".into(), systeme: "switch".into(), ..Default::default() }),
        Err(e) => return Err(e),
    }
    let (rpcs3, compte, titre_id, contenus, ignores) = match contexte_contenus(&app, &noyau, id).await {
        Ok(Some(x)) => x,
        Ok(None) => return Ok(ContenusDuJeu { etat: "aucun".into(), ..Default::default() }),
        Err(Erreur::Reglage(_)) => return Ok(ContenusDuJeu { etat: "emulateur".into(), systeme: "ps3".into(), ..Default::default() }),
        Err(e) => return Err(e),
    };
    let faits = crate::contenus::installes(&rpcs3);
    let installee = titre_id.as_deref().and_then(|n| crate::contenus::version_maj_installee(&rpcs3, n));
    Ok(ContenusDuJeu {
        etat: "ok".into(),
        systeme: "ps3".into(),
        titre_id,
        ignores,
        contenus: contenus
            .into_iter()
            .map(|c| ContenuVu {
                installe: (c.pkg.is_none() || faits.contains(&c.id) || crate::contenus::maj_depassee(&c, installee.as_deref()))
                    && (c.rap.is_none() || compte.as_deref().is_some_and(|k| crate::contenus::licence_posee(&rpcs3, k, &c.id))),
                licence: c.rap.is_some(),
                id: c.id,
                nom: c.nom,
                genre: c.genre,
                taille: c.taille,
                version: c.version,
                dossier: None,
            })
            .collect(),
    })
}

/// Le bilan d'une installation de contenus.
#[derive(Serialize, Default)]
pub struct BilanContenus {
    pub installes: usize,
    /// Des mises à jour déjà dépassées par celle qui est installée (rien n'a été renvoyé à RPCS3).
    pub deja: usize,
    pub refuses: Vec<(String, String)>,
}

/// Progression de « décompresser pour jouer ».
#[derive(Clone, Serialize)]
struct ProgresDecompression {
    jeu: i64,
    ecrits: u64,
    total: u64,
}

/// Le jeu est-il une archive qui contient une image disque (que l'émulateur ne lira pas telle quelle) ?
#[tauri::command]
pub async fn archive_du_jeu(noyau: State<'_, Noyau>, id: i64) -> Resultat<Option<crate::decompression::ArchiveDeJeu>> {
    let Some(f) = noyau.fichier_lance(id).await? else { return Ok(None) };
    tauri::async_runtime::spawn_blocking(move || crate::decompression::examiner(&f))
        .await
        .map_err(|_| Erreur::Disque("La lecture s'est arrêtée brutalement.".into()))?
}

/// Décompresse le jeu à côté de son archive (la personne a vu la taille et dit oui), puis le lance par le fichier
/// décompressé. L'archive est gardée. Rend le chemin de ce qu'on lancera.
#[tauri::command]
pub async fn decompresser_jeu(app: AppHandle, noyau: State<'_, Noyau>, id: i64) -> Resultat<String> {
    let archive = noyau.fichier_lance(id).await?.ok_or_else(|| Erreur::Refus("Ce jeu n'a pas de fichier à décompresser.".into()))?;
    // Hors du fil asynchrone : un 7z est lu par un Frogtend enfant (attente bloquante).
    let a = archive.clone();
    let total = tauri::async_runtime::spawn_blocking(move || crate::decompression::examiner(&a))
        .await
        .map_err(|_| Erreur::Disque("La lecture s'est arrêtée brutalement.".into()))??
        .map(|a| a.taille)
        .unwrap_or(0);
    let libre = archive.parent().and_then(crate::jeux_pc::place_libre);
    noyau.journaliser(&format!("décompression du jeu {id} : {}", archive.display()));
    let (a, app2) = (archive.clone(), app.clone());
    let nouveau = tauri::async_runtime::spawn_blocking(move || {
        let (mut ecrits, mut dernier) = (0u64, std::time::Instant::now());
        crate::decompression::decompresser(&a, libre, &mut |n| {
            ecrits += n;
            if dernier.elapsed().as_millis() >= 250 || ecrits == total {
                dernier = std::time::Instant::now();
                let _ = app2.emit("decompression", ProgresDecompression { jeu: id, ecrits, total });
            }
        })
    })
    .await
    .map_err(|_| Erreur::Disque("La décompression s'est arrêtée brutalement.".into()))??;
    noyau.remplacer_fichier(id, &archive, &nouveau).await?;
    noyau.journaliser(&format!("décompression du jeu {id} terminée : {}", nouveau.display()));
    Ok(nouveau.to_string_lossy().to_string())
}

/// Installe les contenus COCHÉS par la personne (elle a vu la liste, la taille, et dit oui) : chaque .pkg par RPCS3
/// (`--headless --installpkg`, qui se ferme seul), sa licence dans le compte RPCS3 de CE profil.
#[tauri::command]
pub async fn contenus_installer(app: AppHandle, noyau: State<'_, Noyau>, id: i64, choisis: Vec<String>) -> Resultat<BilanContenus> {
    // Switch : Eden lit les dossiers des contenus choisis (ajoutés à sa liste ; rien n'est copié).
    if let Some((eden, l)) = contexte_switch(&app, &noyau, id).await? {
        exiger_emulateur_pour_ecrire(&app, &noyau, &eden.join("eden.exe"))?;
        // Eden en mode portable (son dossier `user` à côté de lui) : sinon il lit sa configuration ailleurs
        // (`%APPDATA%\eden`) et la liste écrite ici ne servirait à rien, malgré une relecture « réussie ».
        if !eden.join("user").is_dir() {
            return Err(Erreur::Refus(
                "Cet Eden range ses réglages hors de son dossier : Frogtend ne peut pas lui donner les contenus. Installe Eden par Frogtend (⚙ Options ▸ Émulateurs) pour qu'il le fasse.".into(),
            ));
        }
        let dossiers: Vec<String> = {
            let mut d: Vec<String> = l
                .iter()
                .filter(|c| choisis.contains(&c.id))
                .filter_map(|c| std::path::Path::new(&c.chemin).parent().map(|p| p.to_string_lossy().to_string()))
                .collect();
            d.sort();
            d.dedup();
            d
        };
        // Eden ouvert réécrirait son fichier en quittant (la liste ajoutée disparaîtrait) : il doit être fermé.
        if !crate::lancement::processus_du_dossier(&eden).is_empty() {
            return Err(Erreur::Refus("Eden est ouvert : ferme-le (ou quitte ta partie), puis recommence.".into()));
        }
        // Les dossiers d'anciens jeux retirés qui attendaient qu'Eden soit fermé partent d'abord.
        if let Err(e) = crate::contenus::ranger_dossiers_eden(&noyau.dossier, None, &|d| !crate::lancement::processus_du_dossier(d).is_empty()) {
            noyau.journaliser(&format!("contenus Switch : liste d'Eden pas rangée ({e})"));
        }
        let ini = crate::contenus::ini_eden(&eden);
        let texte = std::fs::read_to_string(&ini).unwrap_or_default();
        let avant = crate::contenus::dossiers_externes_eden(&texte);
        let apres = crate::contenus::ajouter_dossiers_eden(&texte, &dossiers);
        crate::emulateurs_profils::remplacer_config(&eden, &ini, &apres)?;
        // Vérifier le résultat : les dossiers sont bien dans la liste relue.
        let relus = crate::contenus::dossiers_externes_eden(&std::fs::read_to_string(&ini).unwrap_or_default());
        let manquants: Vec<(String, String)> =
            dossiers.iter().filter(|d| !relus.iter().any(|r| crate::contenus::meme_dossier(r, d))).map(|d| (d.clone(), "pas enregistré dans Eden".to_string())).collect();
        // La preuve de ce que Frogtend a posé (pour le retirer avec le jeu), seulement les dossiers bien relus.
        let poses: Vec<String> = dossiers.iter().filter(|d| relus.iter().any(|r| crate::contenus::meme_dossier(r, d))).cloned().collect();
        if let Err(e) = crate::contenus::noter_et_ecrire_eden(&noyau.dossier, &eden, &avant, &poses, id) {
            noyau.journaliser(&format!("contenus Switch : preuve des dossiers non notée ({e})"));
        }
        noyau.journaliser(&format!("contenus Switch : {} dossier(s) ajouté(s) à Eden", dossiers.len() - manquants.len()));
        return Ok(BilanContenus { deja: 0, installes: l.iter().filter(|c| choisis.contains(&c.id)).count() - manquants.len().min(choisis.len()), refuses: manquants });
    }
    let (rpcs3, _, _, contenus, _) =
        contexte_contenus(&app, &noyau, id).await?.ok_or_else(|| Erreur::Refus("Ce jeu n'a pas de contenus gérés par Frogtend.".into()))?;
    let programme = emulateur_regle(&app, &noyau.registre().jeu(id)?.map(|j| j.plateforme).unwrap_or_default(), id, None)
        .map(|(p, _)| std::path::PathBuf::from(p))
        .ok_or_else(|| Erreur::Reglage("Aucun RPCS3 réglé pour la PS3.".into()))?;
    exiger_emulateur_pour_ecrire(&app, &noyau, &programme)?;
    if !crate::contenus::dev_hdd0_a_sa_place(&rpcs3) {
        return Err(Erreur::Refus("Ce RPCS3 range son disque dur (dev_hdd0) ailleurs que dans son dossier : Frogtend n'y installe rien.".into()));
    }
    // RPCS3 ouvert réécrirait ses fichiers par-dessus ceux de l'installation : il doit être fermé (avant d'écrire quoi
    // que ce soit, même le compte).
    if !crate::lancement::processus_du_dossier(&rpcs3).is_empty() {
        return Err(Erreur::Refus("RPCS3 est ouvert : ferme-le d'abord, puis recommence l'installation.".into()));
    }
    // La personne a dit oui : le compte RPCS3 de son profil est créé s'il manque (même nom qu'au lancement).
    let p = profil_ouvert(&noyau).await?;
    let compte = crate::emulateurs_profils::compte_rpcs3(&rpcs3, &crate::jeux_pc::nom_de_dossier(&p.nom))?;
    let travail = noyau.dossier.join("travail").join("contenus");
    // Une mise à jour choisie sans une précédente (ni choisie, ni installée) : refusée, jamais installée hors de l'ordre.
    let manquantes = {
        let faits = crate::contenus::installes(&rpcs3);
        let installee = contenus.iter().find(|c| c.version.is_some()).and_then(|c| crate::contenus::version_maj_installee(&rpcs3, &c.jeu));
        crate::contenus::precedentes_manquantes(&contenus, &choisis, &faits, installee.as_deref())
    };
    let a_faire: Vec<crate::contenus::Contenu> = contenus.into_iter().filter(|c| choisis.contains(&c.id)).collect();
    noyau.journaliser(&format!("contenus PS3 : installation de {} contenu(s) pour le compte {compte}", a_faire.len()));
    let bilan = tauri::async_runtime::spawn_blocking(move || {
        let b = crate::contenus::installer_chaine_ps3(
            &a_faire,
            &manquantes,
            &rpcs3,
            &compte,
            &travail,
            &|| !crate::lancement::processus_du_dossier(&rpcs3).is_empty(),
            &|pkg| {
                let args = vec!["--headless".to_string(), "--installpkg".to_string(), pkg.to_string_lossy().to_string()];
                let code = crate::installation::executer_et_attendre(&programme, &args, &rpcs3)?;
                if code != 0 {
                    return Err(Erreur::Disque(format!("RPCS3 a répondu {code} en installant le paquet.")));
                }
                Ok(())
            },
        );
        let _ = std::fs::remove_dir(&travail); // seulement s'il est vide
        b
    })
    .await
    .map_err(|_| Erreur::Disque("L'installation s'est arrêtée brutalement.".into()))?;
    for n in &bilan.notes {
        noyau.journaliser(n);
    }
    noyau.journaliser(&format!("contenus PS3 : {} installé(s), {} déjà à jour, {} refusé(s)", bilan.installes, bilan.a_jour, bilan.refuses.len()));
    Ok(BilanContenus { installes: bilan.installes, deja: bilan.a_jour, refuses: bilan.refuses })
}

#[tauri::command]
pub async fn installation_preparer(noyau: State<'_, Noyau>, id: i64) -> Resultat<crate::partie::Preparation> {
    noyau.preparer_installation(id).await
}

/// Installe le jeu (la personne a donné son accord dans l'interface). `suivi` : l'avancement de la décompression d'un
/// jeu en archive est envoyé (événement « mod-progres », étape « decompression ») sous ce numéro.
#[tauri::command]
pub async fn installation_lancer(
    app: AppHandle,
    noyau: State<'_, Noyau>,
    id: i64,
    automatique: bool,
    suivi: Option<u32>,
) -> Resultat<crate::jeux_pc::Installation> {
    let Some(suivi) = suivi else { return noyau.installer(id, automatique).await };
    let f = suivi_mod(&app, suivi);
    noyau.installer_suivi(id, automatique, std::sync::Arc::new(move |faits, total| f("decompression", faits, total))).await
}

#[tauri::command]
pub async fn installation_ailleurs(
    app: AppHandle,
    noyau: State<'_, Noyau>,
    id: i64,
    dossier: String,
) -> Resultat<crate::jeux_pc::Installation> {
    // Durcissement : jamais un dossier réglé lui-même (un emplacement de jeux, le dossier des émulateurs, des outils,
    // des abris) — relus dans `pc.json` par le cœur.
    let reglages = app.store("pc.json").ok().and_then(|s| s.get("reglages")).unwrap_or(Value::Null);
    if crate::garde::Reglages::lire(&reglages).est_un_dossier_regle(std::path::Path::new(&dossier)) {
        return Err(Erreur::Refus(format!(
            "« {dossier} » est un dossier réglé de Frogtend (emplacement de jeux, émulateurs, outils ou abris) : choisis le dossier du jeu lui-même."
        )));
    }
    // Étape 4 : ce dossier vient d'un choix de la personne (fenêtre ouverte par le cœur), jamais d'un chemin libre.
    if !noyau.consommer_choix(true, std::path::Path::new(&dossier)) {
        return Err(Erreur::Refus("Choisis le dossier du jeu avec l'explorateur (« 📂 Le jeu est installé ailleurs… »).".into()));
    }
    noyau.installe_ailleurs(id, &dossier).await
}

#[tauri::command]
pub async fn lancement_candidats(noyau: State<'_, Noyau>, id: i64) -> Resultat<Vec<crate::installation::Candidat>> {
    noyau.candidats_lancement(id).await
}

#[tauri::command]
pub async fn lanceur_choisir(noyau: State<'_, Noyau>, id: i64, lanceur: crate::installation::Lanceur) -> Resultat<()> {
    noyau.choisir_lanceur(id, lanceur).await
}

#[derive(Clone, Serialize)]
#[serde(tag = "sorte", rename_all = "snake_case")]
enum EvenementPartie {
    Debut { jeu: i64, carte_cedee: bool },
    Fin { jeu: i64, secondes: u64 },
}

/// Le réglage « Commandes » d'un jeu, choisi par la personne (réglages de son profil).
#[derive(Debug, Default, Deserialize)]
pub struct Commandes {
    /// `auto`, `clavier` (clavier et souris) ou `reference`.
    #[serde(default)]
    pub mode: String,
    pub genre: Option<String>,
    pub reference: Option<String>,
}

/// Une image Xbox que xemu ne lit pas (ISO Redump, archive) : refusée avec ce qu'il faut faire. Lue hors du fil
/// asynchrone (un disque en veille peut tarder) ; illisible ou d'une autre forme : on laisse xemu juger. Le bouton
/// « Décompresser pour jouer » n'est cité que s'il apparaîtra (archive que Frogtend sait ouvrir, avec une image dedans).
async fn verifier_image_xbox(fichier: std::path::PathBuf, autre_version: bool) -> Resultat<()> {
    use crate::disque::ImageXbox;
    let nom = fichier.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let (image, decompressable) = tauri::async_runtime::spawn_blocking(move || {
        let image = crate::disque::image_xbox(&fichier);
        // Le bouton ne travaille que sur la version habituelle du jeu.
        let d = !autre_version && image == ImageXbox::Archive && crate::decompression::examiner(&fichier).ok().flatten().is_some();
        (image, d)
    })
    .await
    .unwrap_or((ImageXbox::Inconnue, false));
    match image {
        ImageXbox::Redump => Err(Erreur::Prevenir(format!(
            "« {nom} » est une copie complète du disque Xbox, dans une forme que l'émulateur de la Xbox ne sait pas lire : \
             le jeu ne se lancerait pas. Tes fichiers restent intacts.\n\nIl faut une version de ce jeu préparée pour l'émulateur (une image \
             « XISO »). Frogtend ne sait pas encore la préparer lui-même : c'est prévu."
        ))),
        ImageXbox::Archive if decompressable => Err(Erreur::Prevenir(format!(
            "« {nom} » est encore compressé : l'émulateur de la Xbox ne sait pas l'ouvrir.\n\nDécompresse-le d'abord : sur \
             le bureau de Frogtend (hors Taodbox), sélectionne le jeu dans la ludothèque et choisis « 🗜 Décompresser \
             pour jouer » dans son panneau."
        ))),
        ImageXbox::Archive => Err(Erreur::Prevenir(format!(
            "« {nom} » est encore compressé : l'émulateur de la Xbox ne sait pas l'ouvrir, et Frogtend ne sait pas le \
             décompresser ici lui-même. Tes fichiers restent intacts."
        ))),
        ImageXbox::Xiso | ImageXbox::Inconnue => Ok(()),
    }
}

/// Lance le jeu, puis le suit jusqu'à sa fermeture (événements « partie »).
#[tauri::command]
// Une commande de l'interface : chaque réglage du lancement est un argument nommé (Tauri).
#[allow(clippy::too_many_arguments)]
pub async fn jeu_jouer(
    app: AppHandle,
    noyau: State<'_, Noyau>,
    id: i64,
    commandes: Option<Commandes>,
    emulateur: Option<String>,
    version: Option<String>,
    figer: Option<bool>,
    plein_ecran: Option<String>,
    clavier: Option<std::collections::BTreeMap<String, String>>,
    repeter_touches: Option<bool>,
    carte: Option<i64>,
) -> Resultat<()> {
    let jeu = noyau.registre().jeu(id)?;
    // Un jeu d'une boutique : lancé par elle, jamais par un émulateur (même si un émulateur est réglé pour « Windows »).
    let de_boutique = jeu.as_ref().and_then(|j| j.installation.as_ref()).is_some_and(|i| i.boutique.is_some());
    let (plateforme, titre) = jeu.map(|j| (j.plateforme, j.titre)).unwrap_or_default();
    let mut id_emulateur = None;
    let mut dossier_emulateur = None;
    let emulateur = match (!de_boutique).then(|| emulateur_regle(&app, &plateforme, id, emulateur.as_deref())).flatten() {
        Some((programme, ligne)) => {
            dossier_emulateur = std::path::Path::new(&programme).parent().map(|d| d.to_string_lossy().to_string());
            let nom = std::path::Path::new(&programme).file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
            id_emulateur = crate::emulateurs::CATALOGUE.iter().find(|f| nom.starts_with(f.programme)).map(|f| f.id.to_string());
            // MAME vient du paquet de Firehouse, pas du catalogue : reconnu par son programme.
            if id_emulateur.is_none() && nom.starts_with("mame") && nom.ends_with(".exe") {
                id_emulateur = Some("mame".into());
            }
            // xemu ne lit qu'une XISO : dit AVANT de rien préparer (son profil copie un disque dur de plusieurs Go).
            if id_emulateur.as_deref() == Some("xemu") {
                if let Some(f) = noyau.fichier_pour_l_emulateur(id, version.as_deref()).await? {
                    verifier_image_xbox(f, version.is_some()).await?;
                }
            }
            Some(preparer_emulateur(&app, &noyau, &plateforme, &programme, &ligne, &commandes.unwrap_or_default()).await?)
        }
        None => None,
    };
    let par_emulateur = emulateur.is_some();
    // Déjà en cours (un second clic pendant que Steam démarre le jeu, une partie, un téléchargement) : refusé, jamais
    // deux parties suivies du même jeu.
    if noyau.utilise(id) {
        return Err(Erreur::Refus("Ce jeu est déjà en train de se lancer ou de tourner.".into()));
    }
    // Un jeu resté figé d'une partie précédente repart d'abord.
    crate::figer::degeler();
    // « Occupé » du lancement jusqu'à la fin du suivi de la partie (deux tâches) : jamais rattaché entre les deux.
    noyau.marquer_occupe(id);
    let (pid, dossiers, carte_cedee) = match noyau.jouer(id, emulateur, version.as_deref(), carte).await {
        Ok(r) => r,
        Err(e) => {
            noyau.liberer(id);
            return Err(e);
        }
    };
    // MAME sans le script de pause (lanceur choisi, ROM non zip, script de la personne…) : le menu ne promet rien.
    if id_emulateur.as_deref() == Some("mame") && !noyau.pause_mame.load(std::sync::atomic::Ordering::SeqCst) {
        id_emulateur = None;
    }
    // Figer pendant le menu : un jeu lancé sans émulateur seulement, et jamais avec un anti-triche (revérifié ici, hors
    // du fil des commandes : un gros dossier prend quelques secondes).
    let dossier_jeu = (!par_emulateur).then(|| dossiers.first().map(|d| d.to_string_lossy().to_string())).flatten();
    let mut figer = figer.unwrap_or(false) && dossier_jeu.is_some();
    if let (true, Some(d)) = (figer, dossier_jeu.clone()) {
        let v = tauri::async_runtime::spawn_blocking(move || crate::figer::anti_triche(std::path::Path::new(&d)))
            .await
            .unwrap_or(crate::figer::Verification::Inconnu);
        if v != crate::figer::Verification::Aucun {
            noyau.journaliser(&format!("Menu en jeu : jeu {id} pas figé ({v:?})"));
            figer = false;
        }
    }
    let _ = app.emit("partie", EvenementPartie::Debut { jeu: id, carte_cedee });
    // Le menu en jeu : sa touche est active pendant la partie seulement.
    let dossier_jeu_pe = dossier_jeu.clone();
    // Figé pendant le menu : un émulateur sans pause réglable, ou Eden/Azahar dont la pause n'a pas pu être réglée.
    let figer_emulateur = match (id_emulateur.as_deref(), dossier_emulateur.as_deref()) {
        (Some(e), Some(d)) => crate::pilotage::fige_au_menu(e) || !crate::pilotage::pause_reglee(e, std::path::Path::new(d)),
        _ => false,
    };
    // « Disque suivant » seulement pour une liste de disques (une cartouche n'en a qu'un).
    let plusieurs_disques = par_emulateur
        && noyau.fichier_pour_l_emulateur(id, version.as_deref()).await.ok().flatten().is_some_and(|f| crate::menu_jeu::liste_de_disques(&f));
    let numero = app.state::<crate::menu_jeu::MenuJeu>().commencer(crate::menu_jeu::PartieEnCours {
        plusieurs_disques,
        jeu: id,
        titre,
        plateforme,
        pid,
        emulateur: id_emulateur,
        dossier_emulateur,
        figer,
        // Manette → clavier : un jeu PC lancé sans émulateur seulement (un émulateur a ses propres manettes).
        clavier: clavier.filter(|c| !c.is_empty() && dossier_jeu.is_some()),
        repeter_touches: repeter_touches.unwrap_or(true),
        dossier_jeu,
        figer_emulateur,
    });
    armer_touche_menu(&app, true);
    suivre_manette_menu(&app, numero);
    // Plein écran au lancement (jeu PC sans émulateur, réglage du jeu) : dès que sa fenêtre est prête, jamais pendant
    // le menu en jeu ni un jeu figé ; un échec est dit à la personne.
    if let (Some(d), m) = (dossier_jeu_pe, crate::plein_ecran::mode(plein_ecran.as_deref().unwrap_or(""))) {
        if m != crate::plein_ecran::Mode::Aucun {
            let app3 = app.clone();
            std::thread::spawn(move || {
                let menu = app3.state::<crate::menu_jeu::MenuJeu>();
                let etat = || {
                    if !menu.en_cours(numero) {
                        crate::plein_ecran::Etat::Finie
                    } else if menu.ouvert.load(std::sync::atomic::Ordering::SeqCst) || crate::figer::est_fige() {
                        crate::plein_ecran::Etat::Attendre
                    } else {
                        crate::plein_ecran::Etat::Libre
                    }
                };
                let r = crate::plein_ecran::appliquer(std::path::Path::new(&d), &m, &etat);
                app3.state::<Noyau>().journaliser(&format!("plein écran du jeu {id} : {r:?}"));
                if let Err(e) = r {
                    let _ = app3.emit("plein-ecran", serde_json::json!({ "jeu": id, "message": e }));
                }
            });
        }
    }
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        let noyau = app2.state::<Noyau>();
        let fin = noyau.suivre_partie(id, pid, dossiers).await;
        noyau.liberer(id);
        // Seulement si c'est bien la partie en cours qui finit (une partie plus récente garde sa touche et son menu).
        let menu = app2.state::<crate::menu_jeu::MenuJeu>();
        if menu.en_cours(numero) {
            menu.finir(numero);
            armer_touche_menu(&app2, false);
            cacher_menu(&app2);
        }
        // En Taodbox (fenêtre en plein écran), on revient au canapé dès la fin du jeu.
        if let Some(w) = app2.get_webview_window("main") {
            if w.is_fullscreen().unwrap_or(false) {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }
        match fin {
            Ok(fin) => {
                let _ = app2.emit("partie", EvenementPartie::Fin { jeu: id, secondes: fin.secondes });
            }
            Err(e) => noyau.journaliser(&format!("suivi de la partie du jeu {id} : {e:?}")),
        }
    });
    Ok(())
}

/// Garde à l'avance le fond et le logo des jeux du PC pour Taodbox hors ligne (lot Médias M4) : 50 jeux au plus par
/// passage (quelques centaines de Ko chacun). Rend le nombre de jeux prêts.
#[tauri::command]
pub async fn taodbox_precharger(noyau: State<'_, Noyau>) -> Resultat<usize> {
    noyau.precharger_taodbox(50).await
}

/// Hors ligne : les fiches des jeux importés reliés, lues à l'avance par petits passages (150 au plus) après la
/// synchronisation. Rend le nombre de fiches gardées.
#[tauri::command]
pub async fn fiches_reliees_prelire(noyau: State<'_, Noyau>) -> Resultat<usize> {
    noyau.prelire_fiches_reliees(150).await
}

/// Frogtend a-t-il été lancé en Taodbox (`--taodbox`, par exemple comme application de Sunshine) ?
#[tauri::command]
pub fn taodbox_lance() -> bool {
    lance_en_taodbox(std::env::args())
}

pub fn lance_en_taodbox(mut args: impl Iterator<Item = String>) -> bool {
    args.any(|a| a.eq_ignore_ascii_case("--taodbox"))
}

/// Le nom de la fenêtre du menu en jeu.
pub const FENETRE_MENU: &str = "menu-jeu";

/// La touche du menu en jeu (`pc.json` ▸ `menuJeu.touche`, « Pause » par défaut).
fn touche_menu(app: &AppHandle) -> String {
    app.store("pc.json")
        .ok()
        .and_then(|s| s.get("reglages"))
        .and_then(|r| r["menuJeu"]["touche"].as_str().map(String::from))
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| "Pause".into())
}

/// Active (début de partie) ou retire (fin) la touche du menu en jeu. Hors partie, la touche reste aux autres.
fn armer_touche_menu(app: &AppHandle, actif: bool) {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let t = touche_menu(app);
    let gs = app.global_shortcut();
    let r = if actif {
        if gs.is_registered(t.as_str()) {
            Ok(())
        } else {
            gs.register(t.as_str())
        }
    } else {
        gs.unregister(t.as_str())
    };
    if let Err(e) = r {
        app.state::<Noyau>().journaliser(&format!("touche du menu en jeu « {t} » : {e}"));
    }
}

/// La combinaison de la manette qui ouvre le menu en jeu (`pc.json` ▸ `menuJeu.manette`, Select + R1 par défaut).
fn combinaison_menu(app: &AppHandle) -> u16 {
    let nom = app
        .store("pc.json")
        .ok()
        .and_then(|s| s.get("reglages"))
        .and_then(|r| r["menuJeu"]["manette"].as_str().map(String::from))
        .unwrap_or_default();
    crate::osd_manette::combinaison(&nom)
}

/// Pendant la partie `numero`, le cœur lit les manettes (60 fois par seconde) : la combinaison tenue 1 s ouvre le
/// menu ; menu ouvert, la croix, A et B deviennent des commandes pour sa page (`menu-manette`). S'arrête avec la partie.
fn suivre_manette_menu(app: &AppHandle, numero: u64) {
    let app = app.clone();
    let combo = combinaison_menu(&app);
    let mut osd = crate::osd_manette::Osd::nouveau(combo);
    let mut lecteur = crate::osd_manette::Lecteur::default();
    // Manette → clavier (jeu PC, réglage du jeu) : le même fil traduit les boutons en touches.
    let partie = app.state::<crate::menu_jeu::MenuJeu>().partie();
    let dossier_jeu = partie.as_ref().and_then(|p| p.dossier_jeu.clone()).map(std::path::PathBuf::from);
    let repeter = partie.as_ref().is_some_and(|p| p.repeter_touches);
    let mut clavier = partie
        .and_then(|p| p.clavier)
        // La répétition d'une touche tenue : au délai et à la cadence du clavier de Windows, lus au lancement.
        .map(|c| crate::clavier::Moteur::nouveau(&c, combo).avec_repetition(repeter.then(crate::clavier::Repetition::de_ce_pc)))
        .filter(|m| !m.est_vide());
    std::thread::spawn(move || {
        // Le premier plan est vérifié à CHAQUE envoi (le chemin du programme, seulement quand il change).
        let mut premier_plan = crate::clavier::PremierPlan::default();
        let mut eleve_dit = false;
        let mut refus_dit = false;
        let mut refus_sdl_dit = false;
        loop {
            let menu = app.state::<crate::menu_jeu::MenuJeu>();
            if !menu.en_cours(numero) {
                // Fin de partie : rien ne reste appuyé.
                if let Some(m) = clavier.as_mut() {
                    crate::clavier::envoyer(&m.relacher_tout());
                }
                break;
            }
            let ouvert = menu.ouvert.load(std::sync::atomic::Ordering::SeqCst);
            let t = std::time::Instant::now();
            let etat = lecteur.lire(t);
            // Les manettes vues par SDL (PlayStation, Switch…) et un échec de SDL : au journal, pour les essais.
            for v in crate::manettes_sdl::nouvelles_vues() {
                app.state::<Noyau>().journaliser(&format!("manette : {v}"));
            }
            if !refus_sdl_dit {
                if let Some(e) = crate::manettes_sdl::erreur() {
                    refus_sdl_dit = true;
                    app.state::<Noyau>().journaliser(&format!("manettes SDL indisponibles ({e}) : seules les manettes Xbox sont lues"));
                }
            }
            let evenements = osd.pas(etat, t, ouvert);
            let va_ouvrir = evenements.contains(&crate::osd_manette::Evenement::Ouvrir);
            if let (Some(m), Some(d)) = (clavier.as_mut(), dossier_jeu.as_deref()) {
                let (au_jeu, eleve) = premier_plan.au_jeu(d);
                // Seulement vers le jeu au premier plan, jamais pendant le menu ni un jeu figé, ni quand le menu va
                // s'ouvrir : alors tout est relâché TANT QUE le jeu a encore la main.
                if !ouvert && !va_ouvrir && !crate::figer::est_fige() && au_jeu {
                    let (a_envoyer, acceptes) = crate::clavier::envoyer(&m.pas(&etat, t));
                    if acceptes < a_envoyer && !refus_dit {
                        refus_dit = true;
                        app.state::<Noyau>().journaliser("manette → clavier : Windows a refusé des frappes");
                    }
                } else {
                    crate::clavier::envoyer(&m.relacher_tout());
                }
                // Un jeu en administrateur ne reçoit RIEN de Frogtend (Windows bloque sans le dire) : dit une fois.
                if au_jeu && eleve && !eleve_dit {
                    eleve_dit = true;
                    app.state::<Noyau>().journaliser("manette → clavier : le jeu tourne en administrateur, les touches sont bloquées par Windows");
                    let _ = app.emit(
                        "clavier",
                        serde_json::json!({ "message": "ce jeu tourne en administrateur : Windows empêche Frogtend de lui envoyer les touches de la manette" }),
                    );
                }
            }
            for ev in evenements {
                match ev {
                    crate::osd_manette::Evenement::Ouvrir => {
                        // Une entrée neutre d'abord : Windows nous laisse alors prendre le premier plan.
                        crate::menu_jeu::autoriser_premier_plan();
                        let a = app.clone();
                        let _ = app.run_on_main_thread(move || ouvrir_menu(&a));
                        // Vérifier le résultat : le menu a-t-il bien la main ? (Sinon : pas de pause, la manette pilote
                        // encore le jeu.) Noté pour comprendre un échec chez la personne.
                        std::thread::sleep(std::time::Duration::from_millis(300));
                        if !crate::menu_jeu::frogtend_au_premier_plan() {
                            app.state::<Noyau>().journaliser("menu en jeu ouvert à la manette, mais sans le premier plan");
                        }
                    }
                    crate::osd_manette::Evenement::Commande(c) => {
                        let _ = app.emit_to(FENETRE_MENU, "menu-manette", c);
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    });
}

/// Ouvre (ou montre) la fenêtre du menu en jeu par-dessus le jeu, et lui donne le premier plan.
pub fn ouvrir_menu(app: &AppHandle) {
    if app.state::<crate::menu_jeu::MenuJeu>().partie().is_none() {
        return;
    }
    let w = match app.get_webview_window(FENETRE_MENU) {
        Some(w) => w,
        None => match tauri::WebviewWindowBuilder::new(app, FENETRE_MENU, tauri::WebviewUrl::App("menu-jeu".into()))
            .title("Frogtend — menu")
            // 700 de haut : le menu principal et ses 9 entrées (Options, Cheat Engine) tiennent sans défiler.
            .inner_size(560.0, 700.0)
            .center()
            .resizable(false)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .build()
        {
            Ok(w) => {
                // Fermé par Alt+F4 (ou autrement) : on le cache comme « Reprendre » — le jeu repart, et la manette peut
                // rouvrir le menu (sinon il resterait « ouvert » et le jeu figé).
                let a = app.clone();
                w.on_window_event(move |e| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = e {
                        api.prevent_close();
                        cacher_menu(&a);
                    }
                });
                w
            }
            Err(e) => {
                app.state::<Noyau>().journaliser(&format!("menu en jeu : {e}"));
                return;
            }
        },
    };
    // En Taodbox (la fenêtre principale est en plein écran, sur la télé) : le menu aussi, à l'échelle ×2 ; sinon, une
    // petite fenêtre centrée par-dessus le jeu.
    let taodbox = app.get_webview_window("main").and_then(|m| m.is_fullscreen().ok()).unwrap_or(false);
    let _ = w.set_fullscreen(taodbox);
    if !taodbox {
        let _ = w.set_size(tauri::LogicalSize::new(560.0, 700.0));
        let _ = w.center();
    }
    let _ = w.show();
    let _ = w.unminimize();
    let _ = w.set_focus();
    app.state::<crate::menu_jeu::MenuJeu>().ouvert.store(true, std::sync::atomic::Ordering::SeqCst);
    let _ = app.emit_to(FENETRE_MENU, "menu-jeu", serde_json::json!({ "taodbox": taodbox }));
    geler_la_partie(app);
    temoin_mame(app, true);
}

/// MAME : le témoin que son script surveille (voir `pilotage::preparer_mame`).
fn temoin_mame(app: &AppHandle, ouvert: bool) {
    if let Some(p) = app.state::<crate::menu_jeu::MenuJeu>().partie() {
        if let (Some("mame"), Some(d)) = (p.emulateur.as_deref(), p.dossier_emulateur.as_deref()) {
            crate::pilotage::menu_mame(std::path::Path::new(d), ouvert);
        }
    }
}

fn cacher_menu(app: &AppHandle) {
    app.state::<crate::menu_jeu::MenuJeu>().ouvert.store(false, std::sync::atomic::Ordering::SeqCst);
    // Le jeu repart AVANT tout le reste (reprendre, action, quitter : un jeu figé ne se ferme pas).
    crate::figer::degeler();
    temoin_mame(app, false);
    if let Some(w) = app.get_webview_window(FENETRE_MENU) {
        let _ = w.hide();
    }
    // La page du menu le sait aussi : une musique ou une vidéo ouverte s'arrête (sinon elle jouerait, fenêtre cachée).
    let _ = app.emit_to(FENETRE_MENU, "menu-jeu-cache", ());
}

/// Fige le jeu PC de la partie si son option le demande (hors du fil principal : la liste des processus prend un
/// instant). `figer::geler_si` revérifie sous son verrou que le menu est encore ouvert : refermé avant, rien n'est
/// figé. Le menu apprend ensuite que le jeu est vraiment figé (`menu-jeu-pause`).
fn geler_la_partie(app: &AppHandle) {
    let menu = app.state::<crate::menu_jeu::MenuJeu>();
    let Some(p) = menu.partie() else { return };
    // Un jeu PC dont l'option est cochée (son dossier), ou un émulateur sans pause réglable (SA partie seulement :
    // les programmes lancés par Frogtend pour cette partie, dans son dossier — jamais un autre outil rangé là).
    let quoi = if p.figer { "le jeu" } else { "l'émulateur" };
    let (dossier, partie) = if p.figer {
        (p.dossier_jeu.clone(), None)
    } else if p.figer_emulateur {
        (p.dossier_emulateur.clone(), Some(p.pid))
    } else {
        (None, None)
    };
    let Some(dossier) = dossier else { return };
    let app = app.clone();
    std::thread::spawn(move || {
        // Les programmes du jeu : ceux lancés par Frogtend ET ceux de son dossier lancés autrement (raccourci ouvert par
        // l'explorateur, jeu relancé par sa boutique) — toujours limités au dossier du jeu.
        let pids = match partie {
            Some(racine) => crate::lancement::processus_dans(&crate::lancement::processus_de_la_partie(racine), std::path::Path::new(&dossier)),
            None => crate::lancement::processus_du_dossier(std::path::Path::new(&dossier)),
        };
        let menu = app.state::<crate::menu_jeu::MenuJeu>();
        let n = crate::figer::geler_si(&pids, || menu.ouvert.load(std::sync::atomic::Ordering::SeqCst));
        if n > 0 && crate::figer::est_fige() {
            let _ = app.emit_to(FENETRE_MENU, "menu-jeu-pause", true);
        } else if menu.ouvert.load(std::sync::atomic::Ordering::SeqCst) && !crate::figer::est_fige() {
            let raison = if pids.is_empty() { "aucun programme trouvé dans son dossier" } else { "ses programmes refusent (droits administrateur ?)" };
            app.state::<Noyau>().journaliser(&format!("Menu en jeu : {quoi} n'a pas pu être figé ({raison})"));
        }
    });
}

/// Frogtend se ferme : le menu est tenu pour fermé, puis le jeu repart (jamais un jeu laissé figé).
pub fn degeler_la_partie(app: &AppHandle) {
    app.state::<crate::menu_jeu::MenuJeu>().ouvert.store(false, std::sync::atomic::Ordering::SeqCst);
    crate::figer::degeler();
    temoin_mame(app, false);
}

/// La touche plein écran que les textes du jeu installé annoncent (ReadMe, options), s'il y en a une.
#[tauri::command]
pub async fn jeu_plein_ecran_suggestion(noyau: State<'_, Noyau>, id: i64) -> Resultat<Option<String>> {
    let Some(dossier) = noyau.registre().jeu(id)?.and_then(|j| j.installation).map(|i| i.dossier) else { return Ok(None) };
    tauri::async_runtime::spawn_blocking(move || crate::plein_ecran::suggestion(std::path::Path::new(&dossier)).map(String::from))
        .await
        .map_err(|_| Erreur::Disque("La lecture des textes du jeu s'est arrêtée brutalement.".into()))
}

/// Peut-on figer ce jeu pendant le menu ? Refusé s'il a un anti-triche, ou si son dossier est trop grand pour le savoir.
#[tauri::command]
pub async fn jeu_anti_triche(noyau: State<'_, Noyau>, id: i64) -> Resultat<crate::figer::Verification> {
    let dossier = noyau
        .registre()
        .jeu(id)?
        .and_then(|j| j.installation)
        .map(|i| i.dossier)
        .ok_or_else(|| Erreur::Refus("Installe d'abord le jeu.".into()))?;
    tauri::async_runtime::spawn_blocking(move || crate::figer::anti_triche(std::path::Path::new(&dossier)))
        .await
        .map_err(|_| Erreur::Disque("La recherche d'un anti-triche s'est arrêtée brutalement.".into()))
}

/// Ce que le menu en jeu montre (`null` hors partie).
#[tauri::command]
pub async fn menu_jeu_etat(app: AppHandle, menu: State<'_, crate::menu_jeu::MenuJeu>, noyau: State<'_, Noyau>) -> Result<Option<crate::menu_jeu::EtatMenu>, ()> {
    let taodbox = app.get_webview_window("main").and_then(|m| m.is_fullscreen().ok()).unwrap_or(false);
    let Some(p) = menu.partie() else { return Ok(None) };
    let e = crate::menu_jeu::etat(&p);
    // Un jeu PC n'est « en pause » que s'il est VRAIMENT figé (pas seulement parce que l'option est cochée).
    let en_pause = e.en_pause || ((p.figer || p.figer_emulateur) && crate::figer::est_fige());
    // La fiche du jeu pour ce profil (un import relié : celle de Firehouse), pour son fond.
    let fiche = match noyau.session().await {
        Ok(s) => noyau.fiche_de(&s, p.jeu).ok().flatten(),
        Err(_) => None,
    };
    Ok(Some(crate::menu_jeu::EtatMenu { taodbox, en_pause, fiche, ..e }))
}

/// Reprendre : le menu se cache, le jeu reprend le premier plan (et sort seul de sa pause).
#[tauri::command]
pub async fn menu_jeu_reprendre(app: AppHandle) -> Resultat<()> {
    cacher_menu(&app);
    let Some(p) = app.state::<crate::menu_jeu::MenuJeu>().partie() else { return Ok(()) };
    let rendu = tauri::async_runtime::spawn_blocking(move || crate::menu_jeu::reprendre(&crate::lancement::processus_de_la_partie(p.pid)))
        .await
        .map_err(|_| Erreur::Disque("La reprise s'est arrêtée brutalement.".into()))?;
    // Sans le premier plan, un émulateur qui se met en pause en le perdant (RPCS3) resterait en pause : on le note.
    if !rendu {
        app.state::<Noyau>().journaliser("Menu en jeu : le premier plan n'a pas pu être rendu au jeu");
    }
    Ok(())
}

/// Une action du menu (`reset`, `disque`, `sauver`, `charger` ; options : `fps`, `rapide`, `capture`, `muet`) : le jeu
/// reprend la main et reçoit l'ordre.
#[tauri::command]
pub async fn menu_jeu_action(app: AppHandle, noyau: State<'_, Noyau>, action: String) -> Resultat<()> {
    let a = crate::menu_jeu::action_de(&action).ok_or_else(|| Erreur::Refus(format!("Action inconnue : {action}.")))?;
    let p = app
        .state::<crate::menu_jeu::MenuJeu>()
        .partie()
        .ok_or_else(|| Erreur::Introuvable("Aucune partie en cours.".into()))?;
    let e = p.emulateur.clone().ok_or_else(|| Erreur::Refus("Ce jeu ne se pilote pas depuis le menu.".into()))?;
    if e == "mame" {
        return action_mame(&app, &noyau, p.dossier_emulateur.as_deref(), a).await;
    }
    cacher_menu(&app);
    tauri::async_runtime::spawn_blocking(move || crate::menu_jeu::agir(&e, &crate::lancement::processus_de_la_partie(p.pid), a))
        .await
        .map_err(|_| Erreur::Disque("L'action s'est arrêtée brutalement.".into()))?
}

/// MAME : sauver, charger ou recommencer par son script Lua (lot 0.79.0). La commande part AVANT que le menu se
/// ferme (sinon le jeu tournerait quelques images avant la sauvegarde) ; la sauvegarde rapide est celle du PROFIL
/// (`Profils\<profil>\sta\<jeu>\frogtend.sta`), écrite sous un autre nom puis mise à la place une fois complète.
async fn action_mame(app: &AppHandle, noyau: &Noyau, dossier: Option<&str>, a: crate::pilotage::Action) -> Resultat<()> {
    use crate::pilotage::Action;
    let dossier = std::path::PathBuf::from(dossier.ok_or_else(|| Erreur::Refus("MAME est introuvable.".into()))?);
    let (nom, sauve) = crate::pilotage::jeu_mame(&dossier).ok_or_else(|| Erreur::Refus("MAME n'a pas encore dit quel jeu tourne : réessaie dans un instant.".into()))?;
    let profil = profil_ouvert(noyau).await?;
    let etats = crate::pilotage::dossier_etats_mame(&dossier, &profil.nom, &nom);
    if matches!(a, Action::SauverEtat | Action::ChargerEtat) && !sauve {
        return Err(Erreur::Refus("Ce jeu ne gère pas bien les sauvegardes : Frogtend ne les propose pas.".into()));
    }
    // Les fichiers hors du fil asynchrone (la commande peut attendre que le script lâche l'ancienne).
    let (d, e) = (dossier.clone(), etats.clone());
    tauri::async_runtime::spawn_blocking(move || -> Resultat<()> {
        match a {
            Action::Reset => crate::pilotage::commande_mame(&d, "reset", None),
            Action::SauverEtat => {
                std::fs::create_dir_all(&e)?;
                let _ = std::fs::remove_file(e.join(crate::pilotage::ETAT_MAME_NOUVEAU));
                crate::pilotage::commande_mame(&d, "save", Some(&e.join(crate::pilotage::ETAT_MAME_NOUVEAU)))
            }
            Action::ChargerEtat => {
                let etat = e.join(crate::pilotage::ETAT_MAME);
                if !etat.is_file() {
                    return Err(Erreur::Refus("Aucune sauvegarde rapide pour ce jeu : sauve d'abord.".into()));
                }
                crate::pilotage::commande_mame(&d, "load", Some(&etat))
            }
            // Les options du jeu : une bascule dans MAME (compteur d'images, vitesse débridée, son coupé), ou une capture
            // (rangée par MAME dans son dossier `snap`).
            Action::ImagesParSeconde => crate::pilotage::commande_mame(&d, "fps", None),
            Action::AvanceRapide => crate::pilotage::commande_mame(&d, "rapide", None),
            Action::Capture => crate::pilotage::commande_mame(&d, "capture", None),
            Action::Muet => crate::pilotage::commande_mame(&d, "muet", None),
            _ => Err(Erreur::Refus("MAME ne sait pas le faire depuis le menu.".into())),
        }
    })
    .await
    .map_err(|_| Erreur::Disque("L'action s'est arrêtée brutalement.".into()))??;
    cacher_menu(app);
    if a == Action::SauverEtat {
        let r = tauri::async_runtime::spawn_blocking(move || crate::pilotage::garder_etat_mame(&etats, std::time::Duration::from_secs(5)))
            .await
            .map_err(|_| Erreur::Disque("La sauvegarde s'est arrêtée brutalement.".into()))?;
        noyau.journaliser(&format!("MAME : sauvegarde rapide de {nom} {}", if r.is_ok() { "gardée" } else { "ratée (l'ancienne est intacte)" }));
        if r.is_err() {
            // Le menu est caché : il se rouvre (le jeu se remet en pause) pour que l'échec se VOIE.
            let a2 = app.clone();
            let _ = app.run_on_main_thread(move || ouvrir_menu(&a2));
        }
        r?;
    }
    Ok(())
}

/// Quitter le jeu proprement (la personne a confirmé dans le menu). Rend le nombre de fenêtres fermées.
#[tauri::command]
pub async fn menu_jeu_quitter(app: AppHandle) -> Resultat<usize> {
    let p = app
        .state::<crate::menu_jeu::MenuJeu>()
        .partie()
        .ok_or_else(|| Erreur::Introuvable("Aucune partie en cours.".into()))?;
    cacher_menu(&app);
    let racine = p.pid;
    let dossier_emulateur = p.dossier_emulateur.clone();
    let dossier_jeu = p.dossier_jeu.clone();
    let n = tauri::async_runtime::spawn_blocking(move || {
        // Un jeu PC (dont un jeu lancé par sa boutique, dont le premier programme a pu se fermer) : aussi les programmes
        // de SON dossier — jamais rien hors de lui.
        let mut pids = crate::lancement::processus_de_la_partie(racine);
        if let Some(d) = dossier_jeu {
            for p in crate::lancement::processus_du_dossier(std::path::Path::new(&d)) {
                if !pids.contains(&p) {
                    pids.push(p);
                }
            }
        }
        crate::menu_jeu::quitter(p.emulateur.as_deref(), &pids)
    })
    .await
    .map_err(|_| Erreur::Disque("La fermeture s'est arrêtée brutalement.".into()))??;
    // La personne a demandé de quitter et la demande polie est partie : si l'ÉMULATEUR tourne encore 10 s après, sans
    // fenêtre ou sans répondre (RPCS3 garde son programme en vie, caché), Frogtend termine ses processus — seulement
    // ceux du dossier de l'émulateur de cette partie. Un jeu PC n'est jamais arrêté de force.
    if let (true, Some(dossier)) = (n > 0, dossier_emulateur) {
        // Le délai laissé à l'émulateur pour se fermer seul (`pc.json` ▸ `menuJeu.delaiQuitter`, 10 s par défaut).
        let delai = app
            .store("pc.json")
            .ok()
            .and_then(|s| s.get("reglages"))
            .and_then(|r| r["menuJeu"]["delaiQuitter"].as_u64())
            .unwrap_or(10)
            .clamp(3, 120);
        let app2 = app.clone();
        std::thread::spawn(move || {
            let termines = crate::lancement::finir_la_partie(
                racine,
                std::path::Path::new(&dossier),
                std::time::Duration::from_secs(delai),
                crate::menu_jeu::abandonne,
            );
            if termines > 0 {
                app2.state::<Noyau>().journaliser(&format!("quitter : l'émulateur restait en vie sans fenêtre, {termines} processus terminé(s)"));
            }
        });
    }
    Ok(n)
}

/// La version lancée par défaut d'un jeu importé.
#[tauri::command]
pub async fn jeu_choisir_version(noyau: State<'_, Noyau>, id: i64, chemin: String) -> Resultat<()> {
    noyau.choisir_version(id, &chemin).await
}

#[tauri::command]
pub async fn parties_abri(noyau: State<'_, Noyau>, id: i64) -> Resultat<crate::partie::Abri> {
    noyau.mettre_a_l_abri(id).await
}

#[tauri::command]
pub async fn jeu_retirer(noyau: State<'_, Noyau>, id: i64) -> Resultat<crate::partie::Abri> {
    let abri = noyau.retirer_du_pc(id).await?;
    // Les dossiers de contenus Switch que Frogtend avait donnés à Eden pour ce jeu (et qui ont disparu avec lui) quittent
    // sa liste ; Eden ouvert → plus tard. Jamais un échec du retrait.
    match crate::contenus::ranger_dossiers_eden(&noyau.dossier, Some(id), &|d| !crate::lancement::processus_du_dossier(d).is_empty()) {
        Ok(n) if n > 0 => noyau.journaliser(&format!("contenus Switch : {n} dossier(s) retiré(s) de la liste d'Eden")),
        Ok(_) => {}
        Err(e) => noyau.journaliser(&format!("contenus Switch : liste d'Eden pas rangée ({e})")),
    }
    Ok(abri)
}

/// Le désinstalleur d'un jeu retiré qui restait installé ailleurs, après le oui de la personne (proposé par le retrait).
#[tauri::command]
pub async fn jeu_desinstaller_reste(noyau: State<'_, Noyau>, dossier: String) -> Resultat<i32> {
    noyau.desinstaller_reste(&dossier).await
}

/// Ce que devient l'installation d'un émulateur décrit par Firehouse.
#[derive(Serialize)]
#[serde(tag = "sorte", rename_all = "snake_case")]
pub enum InstallationFirehouse {
    Installe { emulateur: crate::emulateurs::EmulateurInstalle },
    /// Plusieurs programmes dans le paquet : la personne choisit (puis `emulateur_adopter`).
    AChoisir { dossier: String, version: String, candidats: Vec<String> },
}

/// Installe un émulateur que Frogtend ne connaît pas en dur, d'après Firehouse (`/emulateurs/{id}/paquet`, contrat
/// 14) : Firehouse résout la dernière version, Frogtend télécharge, vérifie l'empreinte si elle est donnée, installe.
/// La personne a donné son accord dans l'interface (taille annoncée).
#[tauri::command]
pub async fn emulateur_installer_firehouse(
    app: AppHandle,
    noyau: State<'_, Noyau>,
    id: String,
    nom: String,
    dossier: String,
) -> Resultat<InstallationFirehouse> {
    let base = std::path::PathBuf::from(&dossier);
    // Le dossier réglé d'abord ; s'il n'existe pas encore (le dossier proposé par « Prêt à jouer »), il est créé.
    exiger_dossier_de_base(&app, &base)?;
    let nom_sur = crate::jeux_pc::nom_de_dossier(&nom);
    let cible = base.join(&nom_sur);
    // Une seule installation à la fois dans ce dossier, jamais par-dessus le programme ouvert.
    let _occupe = occuper_installation(&base, &cible, &nom)?;
    if !base.is_dir() {
        std::fs::create_dir_all(&base).map_err(|e| Erreur::Disque(format!("Dossier impossible à créer : {dossier} ({e}).")))?;
    }
    let p = noyau.session().await?.source.paquet_emulateur(&id).await?;
    let url = p["url"].as_str().unwrap_or("").to_string();
    let version = p["version"].as_str().unwrap_or("").to_string();
    let paquet = base.join(".telechargements").join(format!("{}.paquet", crate::jeux_pc::nom_de_dossier(&id)));
    let (app2, id2) = (app.clone(), id.clone());
    let progres = move |recus, total| {
        let _ = app2.emit("emulateur", ProgresEmulateur { id: id2.clone(), recus, total });
    };
    // Servi par Firehouse lui-même (Cheat Engine préparé par Seb) : par l'API, avec le jeton ; sinon la source
    // officielle, en https seulement.
    if let Some(route) = url.strip_prefix(crate::firehouse::PREFIXE) {
        noyau.session().await?.source.telecharger_route(route, &paquet, &progres).await?;
    } else if url.starts_with("https://") {
        crate::emulateurs::telecharger(&url, &paquet, &progres).await?;
    } else {
        return Err(Erreur::Serveur("Firehouse n'a pas donné d'adresse sûre (https) pour ce paquet.".into()));
    }
    if let Some(attendue) = p["sha256"].as_str().filter(|s| !s.is_empty()) {
        if crate::sauvegarde::empreinte(&paquet)?.to_lowercase() != attendue.to_lowercase() {
            let _ = std::fs::remove_file(&paquet);
            return Err(Erreur::Conflit(format!("Le paquet de {nom} est arrivé abîmé (empreinte différente) : rien n'est installé.")));
        }
    }
    let programme = p["programme"].as_str().map(String::from);
    let portable = (p["portable"]["type"].as_str().map(String::from), p["portable"]["nom"].as_str().map(String::from));
    let (pq, c, n) = (paquet.clone(), cible.clone(), nom.clone());
    let r = tauri::async_runtime::spawn_blocking(move || {
        let port = match (&portable.0, &portable.1) {
            (Some(t), Some(n)) => Some((t.as_str(), n.as_str())),
            _ => None,
        };
        crate::emulateurs::installer_paquet_decrit(&n, &pq, &c, programme.as_deref(), port)
    })
    .await
    .map_err(|_| Erreur::Disque("L'installation s'est arrêtée brutalement.".into()))??;
    let _ = std::fs::remove_file(&paquet);
    let _ = std::fs::remove_dir(base.join(".telechargements"));
    match r {
        crate::emulateurs::ProgrammeTrouve::Trouve { programme } => {
            let e = crate::emulateurs::EmulateurInstalle {
                id: id.clone(),
                nom,
                version: Some(version),
                dossier: cible.to_string_lossy().into(),
                programme,
                par_frogtend: true,
                installe_le: crate::noyau::maintenant(),
            };
            registre_emulateurs(&noyau).retenir(e.clone())?;
            Ok(InstallationFirehouse::Installe { emulateur: e })
        }
        crate::emulateurs::ProgrammeTrouve::AChoisir { candidats } => {
            Ok(InstallationFirehouse::AChoisir { dossier: cible.to_string_lossy().into(), version, candidats })
        }
    }
}

/// L'état du compte Steam du profil ouvert (jamais la clé : seulement si elle est enregistrée).
#[derive(Serialize)]
pub struct EtatSteam {
    pub compte: Option<String>,
    pub cle_enregistree: bool,
    pub nb_jeux: usize,
    pub maj_le: Option<String>,
    /// Steam est-il installé sur ce PC (pour jouer et installer) ?
    pub steam_installe: bool,
}

async fn profil_ouvert(noyau: &Noyau) -> Resultat<ProfilVisible> {
    noyau.actif().await.ok_or_else(|| Erreur::Profil("Aucun profil n'est ouvert.".into()))
}

fn dossier_profil_de(noyau: &Noyau, id: &str) -> std::path::PathBuf {
    noyau.dossier.join("profils").join(id)
}

#[tauri::command]
pub async fn boutique_steam_etat(noyau: State<'_, Noyau>) -> Resultat<EtatSteam> {
    let p = profil_ouvert(&noyau).await?;
    let c = crate::boutiques::lire(&dossier_profil_de(&noyau, &p.id));
    let cle = noyau.coffre.lire(&crate::boutiques::nom_secret(&p.id, "steam"))?.is_some();
    Ok(EtatSteam {
        compte: c.as_ref().map(|c| c.compte.clone()),
        cle_enregistree: cle,
        nb_jeux: c.as_ref().map_or(0, |c| c.jeux.len()),
        maj_le: c.as_ref().map(|c| c.maj_le.clone()).filter(|m| !m.is_empty()),
        steam_installe: crate::boutiques::dossier_steam().is_some(),
    })
}

/// Règle le compte Steam du profil : `cle` absente = garder celle déjà enregistrée. Le compte est VÉRIFIÉ auprès de
/// Steam avant que quoi que ce soit soit enregistré ; la clé va dans le coffre de Windows seulement.
#[tauri::command]
pub async fn boutique_steam_regler(noyau: State<'_, Noyau>, compte: String, cle: Option<String>) -> Resultat<EtatSteam> {
    let p = profil_ouvert(&noyau).await?;
    let secret = crate::boutiques::nom_secret(&p.id, "steam");
    let cle = match cle.map(|c| c.trim().to_string()).filter(|c| !c.is_empty()) {
        Some(c) if c.len() == 32 && c.chars().all(|x| x.is_ascii_hexdigit()) => c,
        Some(_) => return Err(Erreur::Refus("Une clé d'API Steam fait 32 caractères (chiffres et lettres A à F).".into())),
        None => noyau.coffre.lire(&secret)?.ok_or_else(|| Erreur::Refus("Donne ta clé d'API Steam.".into()))?,
    };
    let steamid = crate::boutiques::steamid(crate::boutiques::API_STEAM, &cle, &compte).await?;
    noyau.coffre.ranger(&secret, &cle)?;
    let dossier = dossier_profil_de(&noyau, &p.id);
    let mut c = crate::boutiques::lire(&dossier).unwrap_or_default();
    if c.steamid != steamid {
        c.jeux.clear(); // un autre compte : l'ancienne liste ne vaut plus
    }
    c.compte = compte.trim().to_string();
    c.steamid = steamid;
    crate::boutiques::ecrire(&dossier, Some(&c))?;
    noyau.journaliser(&format!("compte Steam réglé pour le profil {}", p.id));
    boutique_steam_etat(noyau).await
}

/// Le compte RetroAchievements du profil (sans la clé, qui ne quitte jamais le coffre).
#[derive(Serialize)]
pub struct EtatRetro {
    pub compte: Option<String>,
    pub cle_enregistree: bool,
    /// Le jeton de connexion des émulateurs (RetroArch, PCSX2) est-il au coffre ?
    pub emulateurs_connectes: bool,
}

#[tauri::command]
pub async fn ra_etat(noyau: State<'_, Noyau>) -> Resultat<EtatRetro> {
    let p = profil_ouvert(&noyau).await?;
    let c = crate::boutiques::lire_source(&dossier_profil_de(&noyau, &p.id), "retroachievements");
    Ok(EtatRetro {
        compte: c.map(|c| c.compte).filter(|c| !c.is_empty()),
        cle_enregistree: noyau.coffre.lire(&crate::boutiques::nom_secret(&p.id, "retroachievements"))?.is_some(),
        emulateurs_connectes: noyau.coffre.lire(&crate::boutiques::nom_secret(&p.id, "retroachievements-jeton"))?.is_some(),
    })
}

/// Connecte les émulateurs au compte RetroAchievements du profil : le mot de passe sert UNE fois à obtenir leur jeton
/// de connexion (comme ils le font eux-mêmes), puis il est oublié ; seul le jeton va au coffre de Windows.
#[tauri::command]
pub async fn ra_connecter_emulateurs(noyau: State<'_, Noyau>, mot_de_passe: String) -> Resultat<EtatRetro> {
    let p = profil_ouvert(&noyau).await?;
    let compte = crate::boutiques::lire_source(&dossier_profil_de(&noyau, &p.id), "retroachievements")
        .map(|c| c.compte)
        .filter(|c| !c.is_empty())
        .ok_or_else(|| Erreur::Refus("Règle d'abord ton compte RetroAchievements.".into()))?;
    let (nom, jeton) = crate::succes::jeton_connexion(crate::succes::HOTE_RA, &compte, &mot_de_passe).await?;
    drop(mot_de_passe);
    if !nom.eq_ignore_ascii_case(&compte) {
        return Err(Erreur::Refus(format!("Ce mot de passe est celui de « {nom} », pas de « {compte} ».")));
    }
    noyau.coffre.ranger(&crate::boutiques::nom_secret(&p.id, "retroachievements-jeton"), &jeton)?;
    noyau.journaliser(&format!("émulateurs connectés à RetroAchievements pour le profil {}", p.id));
    ra_etat(noyau).await
}

/// Règle le compte RetroAchievements : VÉRIFIÉ auprès de RetroAchievements avant d'être enregistré ; la clé d'API Web
/// va dans le coffre de Windows seulement (`cle` absente : garder celle déjà enregistrée).
#[tauri::command]
pub async fn ra_regler(noyau: State<'_, Noyau>, compte: String, cle: Option<String>) -> Resultat<EtatRetro> {
    let p = profil_ouvert(&noyau).await?;
    let secret = crate::boutiques::nom_secret(&p.id, "retroachievements");
    let cle = match cle.map(|c| c.trim().to_string()).filter(|c| !c.is_empty()) {
        Some(c) if c.len() >= 16 && c.chars().all(|x| x.is_ascii_alphanumeric()) => c,
        Some(_) => return Err(Erreur::Refus("Ce n'est pas une clé d'API Web de RetroAchievements (lettres et chiffres).".into())),
        None => noyau.coffre.lire(&secret)?.ok_or_else(|| Erreur::Refus("Donne ta clé d'API Web de RetroAchievements.".into()))?,
    };
    let nom = crate::succes::verifier_compte(crate::succes::API_RA, &cle, &compte).await?;
    noyau.coffre.ranger(&secret, &cle)?;
    let c = crate::boutiques::CompteBoutique { compte: nom, maj_le: crate::noyau::maintenant(), ..Default::default() };
    crate::boutiques::ecrire_source(&dossier_profil_de(&noyau, &p.id), "retroachievements", Some(&c))?;
    noyau.journaliser(&format!("compte RetroAchievements réglé pour le profil {}", p.id));
    ra_etat(noyau).await
}

/// La connexion RetroAchievements d'un profil est retirée de tous les émulateurs réglés sur ce PC.
fn oublier_succes_partout(app: &AppHandle, noyau: &Noyau, nom_profil: &str) {
    for programme in programmes_emulateurs(app) {
        let Some(dossier) = std::path::Path::new(&programme).parent() else { continue };
        if let Err(e) = crate::emulateurs_profils::oublier_succes(dossier, nom_profil) {
            noyau.journaliser(&format!("connexion RetroAchievements pas retirée de {} : {e:?}", dossier.display()));
        }
    }
}

#[tauri::command]
pub async fn ra_oublier(app: AppHandle, noyau: State<'_, Noyau>) -> Resultat<()> {
    let p = profil_ouvert(&noyau).await?;
    noyau.coffre.oublier(&crate::boutiques::nom_secret(&p.id, "retroachievements"))?;
    noyau.coffre.oublier(&crate::boutiques::nom_secret(&p.id, "retroachievements-jeton"))?;
    // Le jeton ne reste pas non plus dans les fichiers des émulateurs.
    oublier_succes_partout(&app, &noyau, &p.nom);
    crate::boutiques::ecrire_source(&dossier_profil_de(&noyau, &p.id), "retroachievements", None)
}

/// Une version d'un jeu du PC, vue par RetroAchievements.
#[derive(Serialize)]
pub struct VersionRetro {
    pub chemin: String,
    pub compatible: bool,
    pub courante: bool,
    /// Faux : Frogtend ne sait pas encore lire ce format (`.chd`…), on ne peut rien dire de cette version.
    pub verifiable: bool,
}

/// Les succès RetroAchievements d'un jeu du PC : ses versions compatibles ou non, celles qui le seraient (noms
/// officiels), et la progression de la personne.
#[derive(Serialize, Default)]
pub struct SuccesRetro {
    /// `aucun` (pas d'une console RetroAchievements), `compte` (compte pas réglé), `pas_verifiable` (format pas
    /// encore lu), `zip` (une image zippée à décompresser d'abord, comme les .iso PS3), `ok`.
    pub etat: String,
    pub versions: Vec<VersionRetro>,
    pub jeu: Option<crate::succes::Progression>,
    /// Si aucune de ses versions n'est reconnue : les versions compatibles connues de RetroAchievements.
    pub compatibles: Vec<crate::succes::VersionCompatible>,
    /// Le jeu est reconnu, mais RetroAchievements n'a pas répondu (hors ligne).
    #[serde(default)]
    pub hors_ligne: bool,
}

#[tauri::command]
pub async fn succes_retro(noyau: State<'_, Noyau>, id: i64) -> Resultat<SuccesRetro> {
    retro_du_jeu(&noyau, id).await
}

/// Les succès RetroAchievements d'un jeu du PC (le jeu reconnu par l'empreinte de son fichier, sinon par son titre).
async fn retro_du_jeu(noyau: &Noyau, id: i64) -> Resultat<SuccesRetro> {
    let p = profil_ouvert(noyau).await?;
    let j = noyau.jeu_visible(id).await?;
    let Some(console) = crate::succes::console_de(&j.plateforme) else {
        return Ok(SuccesRetro { etat: "aucun".into(), ..Default::default() });
    };
    let compte = crate::boutiques::lire_source(&dossier_profil_de(&noyau, &p.id), "retroachievements").map(|c| c.compte);
    let cle = noyau.coffre.lire(&crate::boutiques::nom_secret(&p.id, "retroachievements"))?;
    let (Some(compte), Some(cle)) = (compte, cle) else {
        return Ok(SuccesRetro { etat: "compte".into(), ..Default::default() });
    };
    if !crate::succes::empreinte_possible(console) {
        return Ok(SuccesRetro { etat: "pas_verifiable".into(), ..Default::default() });
    }
    let Some(i) = j.installation.clone() else {
        return Ok(SuccesRetro { etat: "pas_verifiable".into(), ..Default::default() });
    };
    let courant = i.fichier_du_jeu.as_ref().map(|f| std::path::Path::new(&i.dossier).join(f).to_string_lossy().to_string());
    let mut chemins: Vec<String> = i.versions.iter().map(|v| v.chemin.clone()).collect();
    if chemins.is_empty() {
        chemins.extend(courant.clone());
    }
    let cache = noyau.dossier.join("cache").join("retroachievements");
    let liste = crate::succes::liste_en_cache(&cache, crate::succes::API_RA, &cle, console).await?;
    let c2 = chemins.clone();
    let empreintes = tauri::async_runtime::spawn_blocking(move || {
        c2.iter().map(|c| crate::succes::empreinte_gardee(&cache, console, std::path::Path::new(c)).ok().flatten()).collect::<Vec<_>>()
    })
    .await
    .map_err(|_| Erreur::Disque("Le calcul des empreintes s'est arrêté brutalement.".into()))?;
    let mut jeu_ra = None;
    let mut versions = Vec::new();
    if empreintes.iter().all(Option::is_none) {
        let zip = chemins.iter().any(|c| c.to_lowercase().ends_with(".zip") || c.to_lowercase().ends_with(".7z")) && est_disque_lourd(console);
        return Ok(SuccesRetro { etat: if zip { "zip" } else { "pas_verifiable" }.into(), ..Default::default() });
    }
    for (c, e) in chemins.iter().zip(empreintes) {
        let verifiable = e.is_some();
        let trouve = e.and_then(|e| liste.iter().find(|g| g.empreintes.contains(&e)));
        if jeu_ra.is_none() {
            jeu_ra = trouve.map(|g| g.id);
        }
        versions.push(VersionRetro {
            chemin: c.clone(),
            compatible: trouve.is_some(),
            courante: courant.as_ref().is_some_and(|x| x.eq_ignore_ascii_case(c)),
            verifiable,
        });
    }
    let mut r = SuccesRetro { etat: "ok".into(), versions, ..Default::default() };
    // Aucune version reconnue : le jeu par son titre, et les versions qui seraient compatibles.
    let id_ra = jeu_ra.or_else(|| {
        let t = crate::succes::titre_comparable(&j.titre);
        liste.iter().find(|g| crate::succes::titre_comparable(&g.titre) == t).map(|g| g.id)
    });
    if let Some(id_ra) = id_ra {
        if jeu_ra.is_none() {
            r.compatibles = crate::succes::empreintes_du_jeu(crate::succes::API_RA, &cle, id_ra).await.unwrap_or_default();
        }
        match crate::succes::progression(crate::succes::API_RA, &cle, &compte, id_ra).await {
            Ok(p) => r.jeu = Some(p),
            // Jeu reconnu mais RetroAchievements injoignable : dit, jamais « aucun succès ».
            Err(_) => r.hors_ligne = true,
        }
    }
    Ok(r)
}

/// Les consoles dont une image zippée ne se vérifie pas sans la décompresser en entier (disques de plusieurs Go).
fn est_disque_lourd(console: u32) -> bool {
    matches!(console, 12 | 16 | 19 | 21 | 41 | 82)
}

/// Les succès Steam d'un jeu (obtenus, total), ou `None` s'il n'en a pas ou si le compte Steam n'est pas réglé.
#[tauri::command]
pub async fn succes_steam(noyau: State<'_, Noyau>, appid: String) -> Resultat<Option<(u32, u32)>> {
    let p = profil_ouvert(&noyau).await?;
    if !appid.chars().all(|c| c.is_ascii_digit()) {
        return Err(Erreur::Refus("Jeu Steam invalide.".into()));
    }
    let Some(c) = crate::boutiques::lire(&dossier_profil_de(&noyau, &p.id)).filter(|c| !c.steamid.is_empty()) else { return Ok(None) };
    let Some(cle) = noyau.coffre.lire(&crate::boutiques::nom_secret(&p.id, "steam"))? else { return Ok(None) };
    crate::succes::succes_steam(crate::boutiques::API_STEAM, &cle, &c.steamid, &appid).await
}

/// Les succès en détail d'un jeu de boutique (lot C) : `source` = `galaxy` (`cle` = sa clé Galaxy) ou `steam` (`cle` =
/// son appid). Un jeu Steam : en attendant le détail par l'API de Steam (lot C1, à relever avec le compte relié de
/// Seb), celui que GOG Galaxy garde pour lui (`steam_<appid>`). Lecture seule, sur une copie de la base.
#[tauri::command]
pub async fn succes_liste(noyau: State<'_, Noyau>, source: String, cle: String) -> Resultat<Option<crate::succes::ListeSucces>> {
    let cle_galaxy = match source.as_str() {
        "galaxy" => cle,
        "steam" if !cle.is_empty() && cle.len() <= 12 && cle.bytes().all(|c| c.is_ascii_digit()) => format!("steam_{cle}"),
        _ => return Ok(None),
    };
    // Seulement pour un profil ouvert (comme l'import de Galaxy).
    profil_ouvert(&noyau).await?;
    let Some(storage) = crate::boutiques::dossier_galaxy() else { return Ok(None) };
    // Une lecture à la fois (la base fait des dizaines de Mo : jamais deux copies ensemble) ; le second appel attend.
    static UNE_A_LA_FOIS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _tour = UNE_A_LA_FOIS.lock().await;
    let travail = noyau.dossier.join("travail").join("galaxy-succes");
    let l = tauri::async_runtime::spawn_blocking(move || {
        // Un reste d'une copie interrompue (Frogtend fermé net) part d'abord : c'est la nôtre, son nom le dit.
        let _ = std::fs::remove_dir_all(&travail);
        crate::boutiques::succes_galaxy(&storage, &travail, &cle_galaxy)
    })
    .await
    .map_err(|_| Erreur::Disque("La lecture des succès s'est arrêtée brutalement.".into()))??;
    Ok(l.map(|succes| crate::succes::ListeSucces { source: "galaxy".into(), succes }))
}

/// La fiche d'un jeu de boutique (lot D, Seb 21/10 : prise DANS les boutiques) : `source` = `steam` (`cle` = l'appid :
/// la fiche française de la boutique Steam, sinon celle de Galaxy) ou `galaxy` (`cle` = la clé Galaxy ; une clé
/// « steam_<appid> » prend d'abord la fiche française de Steam). `None` : aucune fiche.
#[tauri::command]
pub async fn boutique_fiche(noyau: State<'_, Noyau>, source: String, cle: String) -> Resultat<Option<crate::boutiques::FicheBoutique>> {
    profil_ouvert(&noyau).await?;
    let appid = match source.as_str() {
        "steam" => Some(cle.clone()),
        "galaxy" => cle.strip_prefix("steam_").map(String::from),
        _ => None,
    };
    if let Some(appid) = &appid {
        // Steam a refusé (trop de demandes, panne) il y a moins de 5 minutes : on ne le relance pas, et on n'écrit rien
        // (jamais une fiche « vide » gardée une semaine pour un refus passager).
        static PAUSE_STEAM: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);
        if PAUSE_STEAM.lock().unwrap_or_else(|e| e.into_inner()).is_some_and(|t| t.elapsed().as_secs() < 300) {
            return Ok(None);
        }
        let cache = noyau.dossier.join("cache").join("boutiques");
        match crate::boutiques::fiche_steam(&cache, appid).await {
            Ok(Some(f)) => return Ok(Some(f)),
            Ok(None) => {}
            Err(_) => {
                *PAUSE_STEAM.lock().unwrap_or_else(|e| e.into_inner()) = Some(std::time::Instant::now());
                return Ok(None);
            }
        }
    }
    let cle_galaxy = match (source.as_str(), &appid) {
        ("steam", Some(a)) => format!("steam_{a}"),
        _ => cle,
    };
    let Some(storage) = crate::boutiques::dossier_galaxy() else { return Ok(None) };
    if cle_galaxy.is_empty() || !cle_galaxy.chars().all(|c| c.is_ascii_alphanumeric() || "_-.".contains(c)) {
        return Ok(None);
    }
    // La fiche lue une fois par semaine (texte public : rien du compte), jamais une copie de la base à chaque clic.
    let cache = noyau.dossier.join("cache").join("boutiques").join(format!("galaxy-fiche-{cle_galaxy}.json"));
    let recente = std::fs::metadata(&cache).ok().and_then(|m| m.modified().ok()).and_then(|t| t.elapsed().ok()).is_some_and(|d| d.as_secs() < 7 * 24 * 3600);
    if recente {
        if let Some(f) = std::fs::read(&cache).ok().and_then(|o| serde_json::from_slice::<Option<crate::boutiques::FicheBoutique>>(&o).ok()) {
            return Ok(f);
        }
    }
    // Une lecture de la base à la fois (comme les succès), copie effacée ensuite.
    static UNE_A_LA_FOIS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _tour = UNE_A_LA_FOIS.lock().await;
    let travail = noyau.dossier.join("travail").join("galaxy-fiche");
    let f = tauri::async_runtime::spawn_blocking(move || {
        let _ = std::fs::remove_dir_all(&travail);
        crate::boutiques::fiche_galaxy(&storage, &travail, &cle_galaxy)
    })
    .await
    .map_err(|_| Erreur::Disque("La lecture de GOG Galaxy s'est arrêtée brutalement.".into()))??;
    if let Some(d) = cache.parent() {
        let _ = std::fs::create_dir_all(d);
        let _ = std::fs::write(&cache, serde_json::to_vec(&f).unwrap_or_default());
    }
    Ok(f)
}

/// Les succès en détail d'un jeu DU PC (menu en jeu, Taodbox : lot C5) : un jeu Steam du registre → ceux de son appid.
/// `None` : aucun succès connu pour ce jeu (RetroAchievements : lot C3).
#[tauri::command]
pub async fn succes_jeu(noyau: State<'_, Noyau>, id: i64) -> Resultat<Option<crate::succes::ListeSucces>> {
    let j = noyau.jeu_visible(id).await?;
    match j.installation.and_then(|i| i.boutique) {
        Some(b) if b.source == "steam" => return succes_liste(noyau, "steam".into(), b.cle).await,
        // Un jeu GOG du PC (A6) : ceux que Galaxy garde pour lui.
        Some(b) if b.source == "gog" => return succes_liste(noyau, "galaxy".into(), format!("gog_{}", b.cle)).await,
        Some(_) => return Ok(None),
        None => {}
    }
    // Un jeu d'émulateur : RetroAchievements (lot C3), d'après le compte du profil.
    let r = retro_du_jeu(&noyau, id).await?;
    if r.hors_ligne {
        return Err(Erreur::Reseau("RetroAchievements ne répond pas : réessaie dans un moment.".into()));
    }
    Ok(r.jeu.map(|p| p.liste).filter(|l| !l.is_empty()).map(|succes| crate::succes::ListeSucces { source: "retroachievements".into(), succes }))
}

/// Ce que Frogtend mettrait à l'abri des parties d'un jeu Steam (lot B2), à montrer avant.
#[derive(Serialize)]
pub struct ApercuNuage {
    fichiers: usize,
    octets: u64,
    introuvables: usize,
    en_ligne_seulement: usize,
    en_attente: usize,
    /// Plus récents que la dernière synchronisation de Steam : copiés tels quels.
    modifies: usize,
    /// Pris hors du nuage, aux emplacements que Firehouse donne.
    hors_nuage: usize,
    /// Emplacements de Firehouse illisibles (racine perdue, jeton inconnu).
    emplacements_illisibles: usize,
    /// Le dernier abri contient déjà exactement ces fichiers.
    deja: bool,
    /// Plus de `nuage_steam::GROS` : la personne confirme.
    gros: bool,
}

/// Le plan du nuage Steam d'un jeu Steam de ce PC, pour le compte Steam du profil (lecture seule).
/// Rend le plan et la marque du compte (`nuage_steam::marque`).
async fn plan_nuage(noyau: &Noyau, id: i64) -> Resultat<(crate::nuage_steam::Plan, String)> {
    let j = noyau.jeu_visible(id).await?;
    let appid = j
        .installation
        .as_ref()
        .and_then(|i| i.boutique.as_ref())
        .filter(|b| b.source == "steam" && !b.cle.is_empty() && b.cle.bytes().all(|c| c.is_ascii_digit()))
        .map(|b| b.cle.clone())
        .ok_or_else(|| Erreur::Refus("Ce n'est pas un jeu Steam de ce PC.".into()))?;
    let steam = crate::boutiques::dossier_steam().ok_or_else(|| Erreur::Refus("Steam n'est pas installé sur ce PC.".into()))?;
    let userdata = steam.join("userdata");
    let p = profil_ouvert(noyau).await?;
    // Le compte Steam du profil ; sans compte relié, le seul compte de ce PC (jamais deviner entre plusieurs).
    let compte = match crate::boutiques::lire(&dossier_profil_de(noyau, &p.id)).and_then(|c| crate::nuage_steam::compte(&c.steamid)) {
        Some(c) => c,
        None => {
            let l: Vec<String> = std::fs::read_dir(&userdata)
                .into_iter()
                .flatten()
                .flatten()
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().to_str().filter(|n| n.bytes().all(|c| c.is_ascii_digit()) && *n != "0").map(String::from))
                .collect();
            match l.as_slice() {
                [un] => un.clone(),
                [] => return Err(Erreur::Refus("Steam n'a encore rien rangé pour un compte sur ce PC.".into())),
                _ => return Err(Erreur::Refus("Plusieurs comptes Steam sur ce PC : relie le tien (⚙ Options ▸ Boutiques ▸ Steam) pour que Frogtend prenne tes parties.".into())),
            }
        }
    };
    let app = userdata.join(&compte).join(&appid);
    if !userdata.join(&compte).is_dir() {
        return Err(Erreur::Refus("Ton compte Steam n'a encore rien rangé sur ce PC : rien à mettre à l'abri.".into()));
    }
    let marque = crate::nuage_steam::marque(&compte);
    // Les emplacements de parties HORS du nuage, d'après Firehouse (contrat 1.25 `sauvegardes`, PCGamingWiki) ; hors
    // ligne, seulement le nuage.
    let sauvegardes = match criteres_jeu_importe(noyau, id).await {
        Ok(c) => match noyau.session().await {
            Ok(s) => triches_avec(&s.source, &c).await.map(|v| v["sauvegardes"].clone()).unwrap_or(Value::Null),
            Err(_) => Value::Null,
        },
        Err(_) => Value::Null,
    };
    let jeu = j.installation.as_ref().map(|i| std::path::PathBuf::from(&i.dossier));
    let m = marque.clone();
    let plan = tauri::async_runtime::spawn_blocking(move || -> Resultat<Option<crate::nuage_steam::Plan>> {
        let windows = crate::nuage_steam::dossiers_windows();
        let nuage = crate::nuage_steam::planifier(&app, &m, &windows)?;
        let ctx = crate::nuage_steam::Contexte { jeu, steam: Some(steam), compte: Some(compte), windows };
        let (dossiers, illisibles) = crate::nuage_steam::emplacements(&sauvegardes, &ctx);
        if nuage.is_none() && dossiers.is_empty() {
            return Ok(None);
        }
        let mut plan = nuage.unwrap_or_default();
        plan.emplacements_illisibles = illisibles;
        crate::nuage_steam::ajouter_emplacements(&mut plan, &m, &dossiers);
        Ok(Some(plan))
    })
    .await
    .map_err(|_| Erreur::Disque("La lecture des parties s'est arrêtée brutalement.".into()))??
    .ok_or_else(|| Erreur::Refus("Frogtend ne trouve aucune partie de ce jeu sur ce PC (ni dans le nuage de Steam, ni aux emplacements connus de Firehouse).".into()))?;
    Ok((plan, marque))
}

/// Ce que Frogtend mettrait à l'abri des parties d'un jeu Steam.
#[tauri::command]
pub async fn nuage_steam_apercu(noyau: State<'_, Noyau>, id: i64) -> Resultat<ApercuNuage> {
    // Sans dossier des abris, rien ne pourrait être copié : dit dès l'aperçu.
    let abris = noyau.dossier_abris()?;
    let (plan, marque) = plan_nuage(&noyau, id).await?;
    let deja = crate::nuage_steam::deja_a_l_abri(&abris.join(id.to_string()), &marque, &plan);
    Ok(ApercuNuage {
        fichiers: plan.fichiers.len(),
        octets: plan.octets,
        introuvables: plan.introuvables,
        en_ligne_seulement: plan.en_ligne_seulement,
        en_attente: plan.en_attente,
        modifies: plan.modifies,
        hors_nuage: plan.hors_nuage,
        emplacements_illisibles: plan.emplacements_illisibles,
        deja,
        gros: plan.octets > crate::nuage_steam::GROS,
    })
}

/// Met à l'abri les parties d'un jeu Steam (lecture seule chez Steam ; le plan est refait ici, jamais repris de
/// l'interface). Refusé pendant une partie ; au-delà de `GROS`, seulement avec `accepte`.
#[tauri::command]
pub async fn nuage_steam_abriter(noyau: State<'_, Noyau>, id: i64, accepte: bool) -> Resultat<crate::partie::Abri> {
    // Une partie en cours (lancée par Frogtend, ou par Steam : un programme de son dossier tourne) : ses parties
    // changeraient pendant la copie.
    let dossier_jeu = noyau.jeu_visible(id).await?.installation.map(|i| std::path::PathBuf::from(i.dossier));
    if noyau.utilise(id) || dossier_jeu.is_some_and(|d| !crate::lancement::processus_du_dossier(&d).is_empty()) {
        return Err(Erreur::Refus("Le jeu est en cours : quitte-le d'abord, pour que ses parties ne changent pas pendant la copie.".into()));
    }
    let _occupe = noyau.occuper(id);
    let (plan, marque) = plan_nuage(&noyau, id).await?;
    if plan.fichiers.is_empty() {
        return Err(Erreur::Refus("Aucune partie de ce jeu n'est sur ce PC (Steam les garde peut-être seulement dans son nuage).".into()));
    }
    if plan.octets > crate::nuage_steam::GROS && !accepte {
        return Err(Erreur::Refus("Ces parties sont très grosses : confirme d'abord.".into()));
    }
    let abris = noyau.dossier_abris()?;
    let quand = crate::noyau::maintenant();
    let (n, octets) = (plan.fichiers.len(), plan.octets);
    let fin = tauri::async_runtime::spawn_blocking(move || crate::nuage_steam::abriter(&plan, &marque, &abris, id, &quand))
        .await
        .map_err(|_| Erreur::Disque("La mise à l'abri s'est arrêtée brutalement.".into()))??;
    noyau.journaliser(&format!("jeu {id} : {n} fichier(s) du nuage Steam mis à l'abri"));
    Ok(crate::partie::Abri { dossier: fin.to_string_lossy().into(), fichiers: n, octets, reste_installe: None })
}

/// Oublie le compte Steam du profil : la clé quitte le coffre, la liste importée est retirée.
#[tauri::command]
pub async fn boutique_steam_oublier(noyau: State<'_, Noyau>) -> Resultat<()> {
    let p = profil_ouvert(&noyau).await?;
    noyau.coffre.oublier(&crate::boutiques::nom_secret(&p.id, "steam"))?;
    noyau.ranger_boutique("steam", &[]).await?;
    crate::boutiques::ecrire(&dossier_profil_de(&noyau, &p.id), None)
}

/// Les jeux Steam du profil, avec ce qui est installé sur CE PC (relu à chaque fois).
#[tauri::command]
pub async fn boutique_steam_jeux(noyau: State<'_, Noyau>) -> Resultat<Vec<crate::boutiques::JeuBoutique>> {
    let p = profil_ouvert(&noyau).await?;
    let mut l = crate::boutiques::lire(&dossier_profil_de(&noyau, &p.id)).map(|c| c.jeux).unwrap_or_default();
    if let Some(d) = crate::boutiques::dossier_steam() {
        // Installés (sur le disque, même si une mise à jour attend ; jamais pendant le 1er téléchargement), avec leur
        // dossier et s'ils sont prêts à lancer.
        let installes = crate::boutiques::installes_steam_dossiers(&d);
        for j in &mut l {
            let i = installes.get(&j.id);
            j.dossier = i.map(|i| i.dossier.to_string_lossy().into_owned());
            j.pret = i.map(|i| i.pret);
            j.installe = i.is_some();
        }
        // Les installés deviennent des jeux du PC (lancés par Steam, parties suivies) ; les désinstallés en sortent.
        let installes_pc: std::collections::BTreeSet<String> = installes.keys().cloned().collect();
        match noyau.synchroniser_steam(&l, &installes_pc) {
            Ok((a, r)) if a + r > 0 => noyau.journaliser(&format!("jeux Steam du PC : {a} ajouté(s), {r} retiré(s)")),
            Ok(_) => {}
            Err(e) => noyau.journaliser(&format!("jeux Steam du PC : {e:?}")),
        }
        // L'état « installé » de la ludothèque du profil suit le disque (filtre « Installés », bouton Steam).
        if !l.is_empty() {
            if let Err(e) = noyau.ranger_boutique("steam", &l).await {
                noyau.journaliser(&format!("ludothèque Steam : {e:?}"));
            }
        }
    }
    // Les jeux Epic installés (A6d) : lus à l'ouverture du profil, comme Steam.
    synchroniser_epic(&noyau).await;
    Ok(l)
}

/// Importe (ou met à jour) la liste des jeux Steam possédés. Le compte doit être réglé (sinon : `Refus`, et
/// l'interface ouvre le réglage d'abord, règle de Seb).
#[tauri::command]
pub async fn boutique_steam_importer(noyau: State<'_, Noyau>) -> Resultat<Vec<crate::boutiques::JeuBoutique>> {
    let p = profil_ouvert(&noyau).await?;
    let dossier = dossier_profil_de(&noyau, &p.id);
    let mut c = crate::boutiques::lire(&dossier).filter(|c| !c.steamid.is_empty()).ok_or_else(|| Erreur::Refus("Compte Steam pas encore réglé.".into()))?;
    let cle = noyau.coffre.lire(&crate::boutiques::nom_secret(&p.id, "steam"))?.ok_or_else(|| Erreur::Refus("Compte Steam pas encore réglé.".into()))?;
    c.jeux = crate::boutiques::jeux_possedes(crate::boutiques::API_STEAM, &cle, &c.steamid).await?;
    c.maj_le = crate::noyau::maintenant();
    crate::boutiques::ecrire(&dossier, Some(&c))?;
    noyau.journaliser(&format!("import Steam : {} jeu(x) pour le profil {}", c.jeux.len(), p.id));
    let jeux = boutique_steam_jeux(noyau.clone()).await?;
    // Dans la ludothèque, avec les autres jeux (plateforme Windows).
    noyau.ranger_boutique("steam", &jeux).await?;
    Ok(jeux)
}

/// Un jeu offert en ce moment, et s'il est déjà obtenu par ce profil.
#[derive(Serialize)]
pub struct Offert {
    #[serde(flatten)]
    pub jeu: crate::gratuits::JeuOffert,
    pub obtenu: bool,
}

/// Les jeux offerts en ce moment (Epic : liste publique, sans compte).
#[tauri::command]
pub async fn gratuits_liste(noyau: State<'_, Noyau>) -> Resultat<Vec<Offert>> {
    let p = profil_ouvert(&noyau).await?;
    let obtenus = crate::gratuits::lire_obtenus(&dossier_profil_de(&noyau, &p.id));
    let l = crate::gratuits::offerts_epic().await?;
    Ok(l
        .into_iter()
        .map(|j| {
            let obtenu = obtenus.jeux.iter().any(|(b, s, _)| *b == j.boutique && *s == j.slug);
            Offert { jeu: j, obtenu }
        })
        .collect())
}

const FENETRE_EPIC: &str = "boutique-epic";

/// La fenêtre de la boutique Epic, avec le navigateur PROPRE AU PROFIL (sa connexion Epic y reste).
fn fenetre_epic(app: &AppHandle, dossier_profil: &std::path::Path, adresse: &str, visible: bool) -> Resultat<tauri::WebviewWindow> {
    if let Some(w) = app.get_webview_window(FENETRE_EPIC) {
        let _ = w.close();
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
    let url = tauri::Url::parse(adresse).map_err(|_| Erreur::Refus("Adresse Epic invalide.".into()))?;
    tauri::WebviewWindowBuilder::new(app, FENETRE_EPIC, tauri::WebviewUrl::External(url))
        .title("Epic Games — Frogtend")
        .inner_size(1100.0, 800.0)
        .center()
        .visible(visible)
        .data_directory(crate::gratuits::dossier_navigateur(dossier_profil))
        .build()
        .map_err(|e| Erreur::Disque(format!("La fenêtre d'Epic ne s'ouvre pas ({e}).")))
}

/// Se connecter à Epic, une fois : la page officielle s'ouvre ; Frogtend ne voit jamais le mot de passe.
#[tauri::command]
pub async fn gratuits_connexion_epic(app: AppHandle, noyau: State<'_, Noyau>) -> Resultat<()> {
    let p = profil_ouvert(&noyau).await?;
    let w = fenetre_epic(&app, &dossier_profil_de(&noyau, &p.id), "https://www.epicgames.com/id/login?redirectUrl=https%3A%2F%2Fstore.epicgames.com%2Fen-US%2F", true)?;
    let _ = w.set_focus();
    Ok(())
}

/// Obtenir un jeu offert d'Epic, automatiquement : la page officielle s'ouvre CACHÉE, le script de Frogtend clique
/// « Get » puis « Place Order ». Si ça bloque (connexion, captcha, page changée), la fenêtre s'affiche : la personne
/// finit elle-même (décision de Seb : B, sinon A).
#[tauri::command]
pub async fn gratuits_obtenir_epic(app: AppHandle, noyau: State<'_, Noyau>, slug: String) -> Resultat<crate::gratuits::Obtention> {
    if slug.is_empty() || !slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(Erreur::Refus("Jeu Epic invalide.".into()));
    }
    let p = profil_ouvert(&noyau).await?;
    let dossier = dossier_profil_de(&noyau, &p.id);
    let (envoi, reception) = std::sync::mpsc::channel::<crate::gratuits::Obtention>();
    let envoi = std::sync::Mutex::new(envoi);
    if let Some(w) = app.get_webview_window(FENETRE_EPIC) {
        let _ = w.close();
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
    let url = tauri::Url::parse(&format!("https://store.epicgames.com/en-US/p/{slug}")).map_err(|_| Erreur::Refus("Adresse Epic invalide.".into()))?;
    let w = tauri::WebviewWindowBuilder::new(&app, FENETRE_EPIC, tauri::WebviewUrl::External(url))
        .title("Epic Games — Frogtend")
        .inner_size(1100.0, 800.0)
        .center()
        .visible(false)
        .data_directory(crate::gratuits::dossier_navigateur(&dossier))
        .on_page_load(|w, charge| {
            if matches!(charge.event(), tauri::webview::PageLoadEvent::Finished) {
                let _ = w.eval(crate::gratuits::SCRIPT_EPIC);
            }
        })
        .on_document_title_changed(move |_, titre| {
            if let Some(r) = crate::gratuits::lire_titre(&titre) {
                let _ = envoi.lock().map(|e| e.send(r));
            }
        })
        .build()
        .map_err(|e| Erreur::Disque(format!("La fenêtre d'Epic ne s'ouvre pas ({e}).")))?;
    let r = tauri::async_runtime::spawn_blocking(move || reception.recv_timeout(std::time::Duration::from_secs(150)))
        .await
        .map_err(|_| Erreur::Disque("L'attente s'est arrêtée brutalement.".into()))?
        .unwrap_or(crate::gratuits::Obtention::Erreur("aucune réponse de la page (délai dépassé)".into()));
    match &r {
        crate::gratuits::Obtention::Obtenu | crate::gratuits::Obtention::Deja => {
            let _ = w.close();
            let mut o = crate::gratuits::lire_obtenus(&dossier);
            if !o.jeux.iter().any(|(b, s, _)| b == "epic" && *s == slug) {
                o.jeux.push(("epic".into(), slug.clone(), crate::noyau::maintenant()));
            }
            crate::gratuits::ecrire_obtenus(&dossier, &o)?;
        }
        _ => {
            // On montre la page : la personne se connecte, résout le captcha, ou clique elle-même.
            let _ = w.show();
            let _ = w.set_focus();
        }
    }
    noyau.journaliser(&format!("jeu offert Epic « {slug} » : {r:?}"));
    Ok(r)
}

/// 📥 Importer ▸ Fichiers ROM : les types de fichiers d'un dossier et leur nombre (rien à taper : la personne coche).
#[tauri::command]
pub async fn import_types_fichiers(dossier: String, recursif: bool) -> Resultat<Vec<(String, usize)>> {
    tauri::async_runtime::spawn_blocking(move || crate::import_local::types_de_fichiers(std::path::Path::new(&dossier), recursif))
        .await
        .map_err(|_| Erreur::Disque("La lecture du dossier s'est arrêtée brutalement.".into()))?
}

/// 📥 Importer ▸ Fichiers ROM : les ROM d'un dossier, parmi les types de fichiers cochés (rien n'est encore ajouté :
/// la personne voit la liste d'abord).
#[tauri::command]
pub async fn import_chercher_roms(dossier: String, extensions: Vec<String>, recursif: bool) -> Resultat<RomsTrouvees> {
    tauri::async_runtime::spawn_blocking(move || {
        let l = crate::import_local::chercher_roms(std::path::Path::new(&dossier), &extensions, recursif)?;
        // Les zips de contenus additionnels (DLC, avatars…) ne sont pas des jeux : écartés, et comptés.
        let (contenus, roms): (Vec<_>, Vec<_>) = l.into_iter().partition(|r| crate::contenus::est_un_contenu(std::path::Path::new(&r.chemin)));
        Ok(RomsTrouvees { roms, contenus: contenus.len() })
    })
    .await
    .map_err(|_| Erreur::Disque("La recherche s'est arrêtée brutalement.".into()))?
}

/// Les ROM trouvées, et combien de contenus additionnels ont été écartés (ils apparaissent dans le panneau du jeu).
#[derive(Serialize)]
pub struct RomsTrouvees {
    pub roms: Vec<crate::import_local::RomTrouvee>,
    pub contenus: usize,
}

/// 📥 Importer ▸ MAME Arcade Full Set : le tri du dossier (rien n'est encore ajouté), d'après la liste MAME de Firehouse
/// (contrat 1.33) ; en secours seulement (Firehouse injoignable, rien de gardé), le MAME.xml de LaunchBox choisi.
#[tauri::command]
pub async fn import_chercher_mame(noyau: State<'_, Noyau>, dossier: String, liste: Option<String>, options: crate::mame::OptionsMame) -> Resultat<crate::mame::TriMame> {
    let de_firehouse = noyau.liste_mame().await?;
    let liste = liste.filter(|l| !l.trim().is_empty());
    if de_firehouse.is_none() && liste.is_none() {
        return Err(Erreur::Refus(
            "Firehouse ne répond pas et sa liste MAME n'a jamais été reçue sur ce PC : réessaie plus tard, ou choisis le fichier Metadata\\MAME.xml de ton LaunchBox.".into(),
        ));
    }
    // Le secours (le MAME.xml de la personne) contient les jeux pour adultes : pour un profil qui n'est pas admin, ils
    // sont écartés d'office (✅ Seb 27/10 ; avec la liste de Firehouse, c'est lui qui en décide).
    let options = crate::mame::OptionsMame { adultes: options.adultes && (de_firehouse.is_some() || noyau.est_admin().await), ..options };
    tauri::async_runtime::spawn_blocking(move || match de_firehouse {
        Some(l) => crate::mame::trier(std::path::Path::new(&dossier), &l, &options).map(|t| crate::mame::TriMame { source: "firehouse".into(), ..t }),
        None => crate::mame::trier_avec_fichier(std::path::Path::new(&dossier), std::path::Path::new(&liste.unwrap_or_default()), &options)
            .map(|t| crate::mame::TriMame { source: "fichier".into(), ..t }),
    })
    .await
    .map_err(|_| Erreur::Disque("Le tri s'est arrêté brutalement.".into()))?
}

/// Le pack MAME (contrat 1.33) : le dossier proposé d'office (`<disque>\Frogtend\MAME`, le disque qui a le plus de
/// place), et ce qu'il reste à y recevoir — annoncé AVANT tout accord.
#[tauri::command]
pub async fn pack_mame_annonce(noyau: State<'_, Noyau>, dossier: Option<String>, avec_chd: bool) -> Resultat<(String, crate::pack_mame::Annonce)> {
    let dossier = match dossier.filter(|d| !d.trim().is_empty()) {
        Some(d) => std::path::PathBuf::from(d),
        None => crate::pack_mame::dossier_propose().ok_or_else(|| Erreur::Disque("Aucun disque n'a été trouvé pour le pack MAME : choisis un dossier.".into()))?,
    };
    crate::pack_mame::verifier_dossier(&dossier)?;
    let romset = noyau.romset_mame().await?;
    // (La réception en cours sur ce PC est dite dans l'annonce : l'écran ne propose pas d'en lancer une autre.)
    Ok((dossier.to_string_lossy().into(), crate::pack_mame::annoncer(&romset, &dossier, avec_chd)))
}

#[derive(Clone, Serialize)]
struct ProgresPack {
    fait: usize,
    total: usize,
    octets: u64,
}

/// Télécharge le pack MAME (la personne a vu l'annonce et dit oui). `maj` : la version annoncée, quand elle a aussi
/// accepté la mise à jour des fichiers changés (les anciens gardés à l'abri). Progression : événement « pack-mame ».
#[tauri::command]
pub async fn pack_mame_lancer(app: AppHandle, noyau: State<'_, Noyau>, dossier: String, avec_chd: bool, maj: Option<String>) -> Resultat<crate::pack_mame::Bilan> {
    let app2 = app.clone();
    noyau
        .telecharger_pack_mame(std::path::Path::new(&dossier), avec_chd, maj.as_deref(), &move |fait, total, octets| {
            let _ = app2.emit("pack-mame", ProgresPack { fait, total, octets });
        })
        .await
}

/// Met les jeux reçus du pack dans la ludothèque, un par fiche, reliés à leur fiche (étape B).
#[tauri::command]
pub async fn pack_mame_ludotheque(noyau: State<'_, Noyau>, dossier: String) -> Resultat<crate::pack_mame::BilanLudotheque> {
    noyau.ludotheque_pack_mame(std::path::Path::new(&dossier)).await
}

/// Arrête le téléchargement du pack MAME après le fichier en cours (il reprendra où il en était).
#[tauri::command]
pub fn pack_mame_arreter() {
    crate::pack_mame::ARRET.store(true, std::sync::atomic::Ordering::SeqCst);
}

/// 📥 Importer ▸ Installer un jeu DOS : DOSBox s'ouvre avec la SOURCE (dossier, image de CD ou de disquette) et la
/// DESTINATION (vide ou nouvelle : Frogtend n'écrit jamais par-dessus) ; la personne installe ; à la fermeture de
/// DOSBox, Frogtend rend les programmes trouvés dans la destination (le plus probable d'abord).
#[tauri::command]
pub async fn import_installer_dos(app: AppHandle, noyau: State<'_, Noyau>, dosbox: String, source: String, destination: String, titre: String) -> Resultat<Vec<String>> {
    let (dosbox, source, destination) = (std::path::PathBuf::from(dosbox), std::path::PathBuf::from(source), std::path::PathBuf::from(destination));
    if !dosbox.is_file() {
        return Err(Erreur::Reglage("DOSBox introuvable : règle l'émulateur de MS-DOS dans ⚙ Options ▸ Émulateurs.".into()));
    }
    exiger_programme_de_confiance(&app, &noyau, &dosbox)?;
    // Les arguments sont des commandes DOSBox (« mount ») : jamais un autre programme (RetroArch…).
    if !dosbox.file_name().is_some_and(|n| n.to_string_lossy().to_lowercase().starts_with("dosbox")) {
        return Err(Erreur::Reglage(
            "Pour installer un jeu DOS, il faut DOSBox : ajoute-le à MS-DOS dans ⚙ Options ▸ Émulateurs.".into(),
        ));
    }
    if std::fs::read_dir(&destination).is_ok_and(|mut l| l.next().is_some()) {
        return Err(Erreur::Refus(format!("Le dossier {} n'est pas vide : Frogtend n'installe pas par-dessus.", destination.display())));
    }
    // Durcissement (étape 6) : le jeu s'installe dans un sous-dossier du dossier CHOISI par la personne (fenêtre du
    // cœur), jamais dans un dossier système.
    let reglages = app.store("pc.json").ok().and_then(|s| s.get("reglages")).unwrap_or(Value::Null);
    let dans_un_emplacement = crate::garde::Reglages::lire(&reglages).emplacements.iter().any(|e| crate::garde::sous(&destination, e));
    let parent_choisi = destination.parent().is_some_and(|p| noyau.a_ete_choisi(true, p)) || dans_un_emplacement;
    if !parent_choisi || crate::garde::cle(&destination).is_none() || crate::garde::dossier_systeme(&destination) {
        return Err(Erreur::Refus("Choisis avec l'explorateur où installer le jeu (« Où installer le jeu »).".into()));
    }
    std::fs::create_dir_all(&destination)?;
    let arguments = crate::import_local::arguments_installation_dos(&source, &destination)?;
    noyau.journaliser(&format!("installation DOS : DOSBox {arguments:?}"));
    let d = destination.clone();
    let programmes = tauri::async_runtime::spawn_blocking(move || -> Resultat<Vec<String>> {
        crate::installation::executer_et_attendre(&dosbox, &arguments, &d)?;
        crate::import_local::programmes_dos(&d, &titre)
    })
    .await
    .map_err(|_| Erreur::Disque("L'installation s'est arrêtée brutalement.".into()))??;
    Ok(programmes)
}

/// Les arguments de DOSBox pour jouer à un jeu DOS installé par Frogtend.
#[tauri::command]
pub fn import_arguments_jeu_dos(destination: String, programme: String) -> Resultat<Vec<String>> {
    crate::import_local::arguments_jeu_dos(std::path::Path::new(&destination), &programme)
}

/// Avant de copier des jeux dans un emplacement : combien d'octets (pistes et CHD compris) et la place libre.
#[tauri::command]
pub async fn import_mesurer(elements: Vec<String>, destination: String) -> Resultat<(u64, Option<u64>)> {
    tauri::async_runtime::spawn_blocking(move || {
        let e: Vec<std::path::PathBuf> = elements.iter().map(std::path::PathBuf::from).collect();
        let d = std::path::Path::new(&destination);
        let libre = crate::jeux_pc::place_libre(d).or_else(|| d.ancestors().find_map(crate::jeux_pc::place_libre));
        (crate::import_local::taille_a_copier(&e), libre)
    })
    .await
    .map_err(|_| Erreur::Disque("La mesure s'est arrêtée brutalement.".into()))
}

/// Copie des jeux dans un emplacement (jamais déplacés, jamais par-dessus) ; rend leurs nouveaux chemins.
#[tauri::command]
pub async fn import_copier(app: AppHandle, noyau: State<'_, Noyau>, elements: Vec<String>, destination: String) -> Resultat<Vec<String>> {
    // Durcissement (étape 6) : on ne copie que DANS un emplacement de jeux réglé (ou dessous : `<défaut>\<système>`).
    let reglages = app.store("pc.json").ok().and_then(|s| s.get("reglages")).unwrap_or(Value::Null);
    let d = std::path::Path::new(&destination);
    if !crate::garde::Reglages::lire(&reglages).emplacements.iter().any(|e| crate::garde::sous(d, e)) {
        return Err(Erreur::Refus(format!("« {destination} » n'est pas dans un emplacement de jeux réglé (Réglages ▸ Emplacements).")));
    }
    noyau.journaliser(&format!("import : copie de {} jeu(x) dans {destination}", elements.len()));
    tauri::async_runtime::spawn_blocking(move || {
        let e: Vec<std::path::PathBuf> = elements.iter().map(std::path::PathBuf::from).collect();
        crate::import_local::copier_jeux(&e, std::path::Path::new(&destination)).map(|l| l.iter().map(|p| p.to_string_lossy().to_string()).collect())
    })
    .await
    .map_err(|_| Erreur::Disque("La copie s'est arrêtée brutalement.".into()))?
}

/// 📥 Importer ▸ Jeux MS-DOS : un jeu par sous-dossier (rien n'est encore ajouté).
#[tauri::command]
pub async fn import_chercher_dos(dossier: String) -> Resultat<Vec<crate::import_local::JeuDosTrouve>> {
    tauri::async_runtime::spawn_blocking(move || crate::import_local::chercher_jeux_dos(std::path::Path::new(&dossier)))
        .await
        .map_err(|_| Erreur::Disque("La recherche s'est arrêtée brutalement.".into()))?
}

/// Ajoute à la ludothèque les jeux validés par la personne (rien n'est copié, déplacé ni renommé).
#[tauri::command]
pub async fn import_ajouter(app: AppHandle, noyau: State<'_, Noyau>, jeux: Vec<crate::import_local::JeuAImporter>) -> Resultat<crate::import_local::BilanImport> {
    // Durcissement (étape 4) : ce que chaque jeu fera lancer est vérifié AVANT l'import ; un refusé est dit dans le
    // bilan, les autres entrent.
    let reglages = app.store("pc.json").ok().and_then(|s| s.get("reglages")).unwrap_or(Value::Null);
    let r = crate::garde::Reglages::lire(&reglages);
    let mut connus: Vec<std::path::PathBuf> = registre_emulateurs(&noyau).tous().into_iter().map(|e| e.programme.into()).collect();
    connus.extend(crate::emulateurs::detecter().into_iter().map(|e| std::path::PathBuf::from(e.programme)));
    let de_confiance = |p: &std::path::Path| crate::garde::programme_de_confiance(p, &r, &connus);
    let choisi = |p: &std::path::Path| noyau.consommer_choix(false, p);
    let mut admis = Vec::new();
    let mut refuses = Vec::new();
    for j in jeux {
        match crate::import_local::programme_admis(&j, &de_confiance, &choisi) {
            Ok(()) => admis.push(j),
            Err(m) => refuses.push((j.titre.clone(), m)),
        }
    }
    let mut bilan = noyau.importer_locaux(&admis).await?;
    bilan.refuses.extend(refuses);
    Ok(bilan)
}

/// Se connecter à GOG ou à Prime Gaming, une fois : la page officielle s'ouvre (navigateur PROPRE AU PROFIL) ;
/// Frogtend ne voit jamais le mot de passe.
#[tauri::command]
pub async fn gratuits_connexion(app: AppHandle, noyau: State<'_, Noyau>, boutique: String) -> Resultat<()> {
    let (page, _, _) = crate::gratuits::boutique_offerte(&boutique).ok_or_else(|| Erreur::Refus("Boutique inconnue.".into()))?;
    let p = profil_ouvert(&noyau).await?;
    let etiquette = format!("boutique-{boutique}");
    if let Some(w) = app.get_webview_window(&etiquette) {
        let _ = w.close();
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
    let url = tauri::Url::parse(page).map_err(|_| Erreur::Refus("Adresse invalide.".into()))?;
    let w = tauri::WebviewWindowBuilder::new(&app, &etiquette, tauri::WebviewUrl::External(url))
        .title("Connexion — Frogtend")
        .inner_size(1280.0, 860.0)
        .center()
        .data_directory(crate::gratuits::dossier_navigateur(&dossier_profil_de(&noyau, &p.id)))
        .build()
        .map_err(|e| Erreur::Disque(format!("La fenêtre ne s'ouvre pas ({e}).")))?;
    let _ = w.set_focus();
    Ok(())
}

/// Récupère les jeux offerts de GOG ou de Prime Gaming : la page officielle s'ouvre CACHÉE ; si ça bloque (connexion,
/// offres à finir ailleurs, erreur), elle s'affiche et la personne finit (décision de Seb : B, sinon A).
#[tauri::command]
pub async fn gratuits_recuperer(app: AppHandle, noyau: State<'_, Noyau>, boutique: String) -> Resultat<crate::gratuits::Recolte> {
    let (_, page, script) = crate::gratuits::boutique_offerte(&boutique).ok_or_else(|| Erreur::Refus("Boutique inconnue.".into()))?;
    let p = profil_ouvert(&noyau).await?;
    let dossier = dossier_profil_de(&noyau, &p.id);
    let etiquette = format!("boutique-{boutique}");
    let (envoi, reception) = std::sync::mpsc::channel::<crate::gratuits::Recolte>();
    let envoi = std::sync::Mutex::new(envoi);
    if let Some(w) = app.get_webview_window(&etiquette) {
        let _ = w.close();
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
    let url = tauri::Url::parse(page).map_err(|_| Erreur::Refus("Adresse invalide.".into()))?;
    let w = tauri::WebviewWindowBuilder::new(&app, &etiquette, tauri::WebviewUrl::External(url))
        .title("Jeux offerts — Frogtend")
        .inner_size(1280.0, 860.0)
        .center()
        .visible(false)
        .data_directory(crate::gratuits::dossier_navigateur(&dossier))
        .on_page_load(move |w, charge| {
            if matches!(charge.event(), tauri::webview::PageLoadEvent::Finished) {
                let _ = w.eval(script);
            }
        })
        .on_document_title_changed(move |_, titre| {
            if let Some(r) = crate::gratuits::lire_titre_recolte(&titre) {
                let _ = envoi.lock().map(|e| e.send(r));
            }
        })
        .build()
        .map_err(|e| Erreur::Disque(format!("La fenêtre ne s'ouvre pas ({e}).")))?;
    let r = tauri::async_runtime::spawn_blocking(move || reception.recv_timeout(std::time::Duration::from_secs(300)))
        .await
        .map_err(|_| Erreur::Disque("L'attente s'est arrêtée brutalement.".into()))?
        .unwrap_or(crate::gratuits::Recolte { etat: "erreur".into(), motif: Some("aucune réponse de la page (délai dépassé)".into()), ..Default::default() });
    if (r.etat == "faite" && r.a_finir == 0) || r.etat == "aucun" || r.etat == "pas_abonne" {
        let _ = w.close();
    } else {
        let _ = w.show();
        let _ = w.set_focus();
    }
    noyau.journaliser(&format!("jeux offerts {boutique} : {} obtenu(s), {} déjà, {} à finir, état {}", r.obtenus.len(), r.deja, r.a_finir, r.etat));
    Ok(r)
}

const FENETRE_PLAYSTATION: &str = "boutique-playstation";

/// Se connecter à PlayStation, une fois : le Store officiel s'ouvre (« Se connecter » en haut) ; la connexion reste
/// dans le navigateur PROPRE AU PROFIL ; Frogtend ne voit jamais le mot de passe.
#[tauri::command]
pub async fn gratuits_connexion_playstation(app: AppHandle, noyau: State<'_, Noyau>) -> Resultat<()> {
    let p = profil_ouvert(&noyau).await?;
    if let Some(w) = app.get_webview_window(FENETRE_PLAYSTATION) {
        let _ = w.close();
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
    let url = tauri::Url::parse(crate::gratuits::CONNEXION_PLAYSTATION).map_err(|_| Erreur::Refus("Adresse PlayStation invalide.".into()))?;
    let w = tauri::WebviewWindowBuilder::new(&app, FENETRE_PLAYSTATION, tauri::WebviewUrl::External(url))
        .title("PlayStation Store — Frogtend")
        .inner_size(1100.0, 800.0)
        .center()
        .data_directory(crate::gratuits::dossier_navigateur(&dossier_profil_de(&noyau, &p.id)))
        .build()
        .map_err(|e| Erreur::Disque(format!("La fenêtre de PlayStation ne s'ouvre pas ({e}).")))?;
    let _ = w.set_focus();
    Ok(())
}

/// Ajouter les jeux PS Plus du mois à la bibliothèque PlayStation du profil : le Store s'ouvre CACHÉ, le script ne
/// clique que sur « Ajouter à la bibliothèque ». S'il faut se connecter ou si ça bloque, la fenêtre s'affiche.
/// Rien n'entre dans la ludothèque de Frogtend (décision de Seb).
#[tauri::command]
pub async fn gratuits_psplus(app: AppHandle, noyau: State<'_, Noyau>) -> Resultat<crate::gratuits::RecoltePsPlus> {
    use crate::gratuits::RecoltePsPlus;
    let p = profil_ouvert(&noyau).await?;
    let dossier = dossier_profil_de(&noyau, &p.id);
    let (envoi, reception) = std::sync::mpsc::channel::<RecoltePsPlus>();
    let envoi = std::sync::Mutex::new(envoi);
    if let Some(w) = app.get_webview_window(FENETRE_PLAYSTATION) {
        let _ = w.close();
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
    let url = tauri::Url::parse(crate::gratuits::PAGE_PSPLUS).map_err(|_| Erreur::Refus("Adresse PlayStation invalide.".into()))?;
    let w = tauri::WebviewWindowBuilder::new(&app, FENETRE_PLAYSTATION, tauri::WebviewUrl::External(url))
        .title("PlayStation Store — Frogtend")
        .inner_size(1100.0, 800.0)
        .center()
        .visible(false)
        .data_directory(crate::gratuits::dossier_navigateur(&dossier))
        .on_page_load(|w, charge| {
            if matches!(charge.event(), tauri::webview::PageLoadEvent::Finished) {
                let _ = w.eval(crate::gratuits::SCRIPT_PSPLUS);
            }
        })
        .on_document_title_changed(move |_, titre| {
            if let Some(r) = crate::gratuits::lire_titre_psplus(&titre) {
                let _ = envoi.lock().map(|e| e.send(r));
            }
        })
        .build()
        .map_err(|e| Erreur::Disque(format!("La fenêtre de PlayStation ne s'ouvre pas ({e}).")))?;
    // Jusqu'à 30 jeux visités, quelques secondes chacun.
    let r = tauri::async_runtime::spawn_blocking(move || reception.recv_timeout(std::time::Duration::from_secs(600)))
        .await
        .map_err(|_| Erreur::Disque("L'attente s'est arrêtée brutalement.".into()))?
        .unwrap_or(RecoltePsPlus::Erreur { motif: "aucune réponse du Store (délai dépassé)".into() });
    if matches!(r, RecoltePsPlus::Faite { .. }) {
        let _ = w.close();
    } else {
        let _ = w.show();
        let _ = w.set_focus();
    }
    noyau.journaliser(&format!("jeux PS Plus : {r:?}"));
    Ok(r)
}

/// GOG Galaxy sur ce PC, et ce qui en a été importé pour le profil.
#[derive(Serialize)]
pub struct EtatGalaxy {
    pub galaxy_installe: bool,
    pub nb_jeux: usize,
    pub maj_le: Option<String>,
}

#[tauri::command]
pub async fn boutique_galaxy_etat(noyau: State<'_, Noyau>) -> Resultat<EtatGalaxy> {
    let p = profil_ouvert(&noyau).await?;
    let c = crate::boutiques::lire_source(&dossier_profil_de(&noyau, &p.id), "galaxy");
    Ok(EtatGalaxy {
        galaxy_installe: crate::boutiques::dossier_galaxy().is_some(),
        nb_jeux: c.as_ref().map_or(0, |c| c.jeux.len()),
        maj_le: c.map(|c| c.maj_le).filter(|m| !m.is_empty()),
    })
}

/// Les jeux importés de GOG Galaxy pour le profil.
#[tauri::command]
pub async fn boutique_galaxy_jeux(noyau: State<'_, Noyau>) -> Resultat<Vec<crate::boutiques::JeuBoutique>> {
    let p = profil_ouvert(&noyau).await?;
    let l = crate::boutiques::lire_source(&dossier_profil_de(&noyau, &p.id), "galaxy").map(|c| c.jeux).unwrap_or_default();
    synchroniser_gog(&noyau, &l);
    synchroniser_epic(&noyau).await;
    Ok(l)
}

/// Les jeux Epic installés sur CE PC (manifestes de l'Epic Games Launcher, lecture seule) deviennent des jeux du PC
/// (A6d) : lancés par Epic et suivis comme Steam ; une carte dans la ludothèque si GOG Galaxy ne la montre pas déjà.
async fn synchroniser_epic(noyau: &Noyau) {
    let Some(d) = crate::boutiques::dossier_manifestes_epic() else { return };
    let Ok(installes) = tauri::async_runtime::spawn_blocking(move || crate::boutiques::installes_epic(&d)).await else { return };
    let jeux: Vec<crate::boutiques::JeuBoutique> = installes
        .iter()
        .map(|e| crate::boutiques::JeuBoutique {
            id: format!("epic_{}", e.app),
            nom: e.nom.clone(),
            minutes: 0,
            derniere: 0,
            installe: true,
            plateforme: "epic".into(),
            image: None,
            dossier: Some(e.dossier.to_string_lossy().into_owned()),
            pret: Some(true),
            succes: None,
            releases: vec![],
        })
        .collect();
    let pc: std::collections::BTreeSet<String> = installes.iter().map(|e| e.app.clone()).collect();
    match noyau.synchroniser_boutique("epic", &jeux, &pc) {
        Ok((a, r)) if a + r > 0 => noyau.journaliser(&format!("jeux Epic du PC : {a} ajouté(s), {r} retiré(s)")),
        Ok(_) => {}
        Err(e) => noyau.journaliser(&format!("jeux Epic du PC : synchronisation impossible ({e})")),
    }
    if let Err(e) = noyau.ranger_boutique("epic", &jeux).await {
        noyau.journaliser(&format!("ludothèque Epic : {e:?}"));
    }
}

/// Les jeux GOG installés sur CE PC deviennent des jeux du PC (A6a) : lancés par Galaxy et suivis comme les jeux
/// Steam. La liste vient de la base Galaxy de CE PC (gardée à l'import) ; un jeu n'est retiré que si son dossier a
/// disparu (jamais sur la seule foi d'une liste). Un échec est noté, sans gêner l'import.
fn synchroniser_gog(noyau: &Noyau, jeux: &[crate::boutiques::JeuBoutique]) {
    let installes_pc: std::collections::BTreeSet<String> = jeux
        .iter()
        .filter(|j| j.dossier.as_deref().is_some_and(|d| std::path::Path::new(d).is_dir()))
        .filter_map(|j| j.id.strip_prefix("gog_").map(String::from))
        .collect();
    match noyau.synchroniser_boutique("gog", jeux, &installes_pc) {
        Ok((a, r)) if a + r > 0 => noyau.journaliser(&format!("jeux GOG du PC : {a} ajouté(s), {r} retiré(s)")),
        Ok(_) => {}
        Err(e) => noyau.journaliser(&format!("jeux GOG du PC : synchronisation impossible ({e})")),
    }
}

/// Importe les jeux de GOG Galaxy (GOG et boutiques reliées), en LECTURE SEULE, sur une copie de sa base (accord de
/// Seb, 02/10). Aucun secret : Galaxy est déjà connecté sur ce PC.
#[tauri::command]
pub async fn boutique_galaxy_importer(noyau: State<'_, Noyau>) -> Resultat<Vec<crate::boutiques::JeuBoutique>> {
    let p = profil_ouvert(&noyau).await?;
    let storage = crate::boutiques::dossier_galaxy().ok_or_else(|| Erreur::Introuvable("GOG Galaxy n'est pas installé sur ce PC (ou n'a jamais été ouvert).".into()))?;
    let travail = noyau.dossier.join("travail").join("galaxy");
    let t2 = travail.clone();
    let jeux = tauri::async_runtime::spawn_blocking(move || crate::boutiques::lire_galaxy(&storage, &t2))
        .await
        .map_err(|_| Erreur::Disque("La lecture de GOG Galaxy s'est arrêtée brutalement.".into()));
    // La copie de la base ne reste pas, même si la lecture a échoué : elle contient les données du compte.
    let _ = std::fs::remove_dir_all(&travail);
    let jeux = jeux??;
    let c = crate::boutiques::CompteBoutique { compte: "GOG Galaxy".into(), steamid: String::new(), maj_le: crate::noyau::maintenant(), jeux };
    crate::boutiques::ecrire_source(&dossier_profil_de(&noyau, &p.id), "galaxy", Some(&c))?;
    noyau.journaliser(&format!("import GOG Galaxy : {} jeu(x) pour le profil {}", c.jeux.len(), p.id));
    noyau.ranger_boutique("galaxy", &c.jeux).await?;
    synchroniser_gog(&noyau, &c.jeux);
    synchroniser_epic(&noyau).await;
    Ok(c.jeux)
}

/// Ouvre un jeu dans GOG Galaxy (sa page : jouer, installer), pour toutes ses boutiques reliées.
#[tauri::command]
pub fn boutique_galaxy_ouvrir(app: AppHandle, cle: String) -> Resultat<()> {
    use tauri_plugin_opener::OpenerExt;
    if cle.is_empty() || cle.len() > 120 || !cle.chars().all(|c| c.is_ascii_alphanumeric() || "_-:.".contains(c)) {
        return Err(Erreur::Refus("Jeu GOG Galaxy invalide.".into()));
    }
    app.opener()
        .open_url(format!("goggalaxy://openGameView/{cle}"), None::<&str>)
        .map_err(|_| Erreur::Disque("GOG Galaxy ne s'ouvre pas : est-il installé ?".into()))
}

/// Jouer à un jeu Steam, ou l'installer : Steam fait le travail (`steam://`).
#[tauri::command]
pub fn boutique_steam_ouvrir(app: AppHandle, appid: String, action: String) -> Resultat<()> {
    use tauri_plugin_opener::OpenerExt;
    if appid.is_empty() || !appid.chars().all(|c| c.is_ascii_digit()) {
        return Err(Erreur::Refus("Jeu Steam invalide.".into()));
    }
    let url = match action.as_str() {
        "jouer" => format!("steam://rungameid/{appid}"),
        "installer" => format!("steam://install/{appid}"),
        // « Vérifier l'intégrité des fichiers » (protocole de Steam, Valve Developer Community « Steam browser
        // protocol ») : Steam remet SES fichiers, après un mod retiré à la suite d'une mise à jour (lot B3).
        "verifier" => format!("steam://validate/{appid}"),
        _ => return Err(Erreur::Refus("Action inconnue.".into())),
    };
    app.opener().open_url(url, None::<&str>).map_err(|_| Erreur::Disque("Steam ne s'ouvre pas : est-il installé ?".into()))
}

/// Lance Cheat Engine pour le profil ouvert (accord de Seb pour le registre, 02/10) : ses réglages du profil sont remis
/// avant, et rangés dans le dossier du profil quand il se ferme (tous ses processus, lanceur compris).
#[tauri::command]
pub async fn cheatengine_lancer(
    app: AppHandle,
    noyau: State<'_, Noyau>,
    programme: String,
    table: Option<String>,
    brancher: Option<bool>,
    accepte: Option<bool>,
) -> Resultat<()> {
    let p = std::path::PathBuf::from(&programme);
    let nom = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
    if !p.is_file() || !nom.starts_with("cheatengine") && !nom.starts_with("cheat engine") {
        return Err(Erreur::Refus("Ce n'est pas le programme de Cheat Engine.".into()));
    }
    exiger_programme_de_confiance(&app, &noyau, &p)?;
    // Brancher sur un jeu PC (Seb, 21/10) : jamais avec un anti-triche connu ; un jeu en ligne ou inconnu, seulement
    // après le oui de la personne (l'interface a montré `cheatengine_risque`).
    // UN seul relevé de la partie, pour la vérification ET le branchement (revérifié juste avant de brancher).
    let branchee = match brancher.unwrap_or(false) {
        true => Some(
            app.state::<crate::menu_jeu::MenuJeu>()
                .partie()
                .ok_or_else(|| Erreur::Refus("Lance d'abord le jeu : Cheat Engine se branchera dessus.".into()))?,
        ),
        false => None,
    };
    if let Some(partie) = &branchee {
        if partie.dossier_jeu.is_some() {
            let r = risque_cheatengine(&noyau, partie.jeu).await?;
            if let Some(a) = r.anti_triche {
                return Err(Erreur::Refus(format!("Ce jeu est protégé par {a} : Frogtend n'y branche pas Cheat Engine (ton compte pourrait être banni).")));
            }
            if (r.en_ligne.is_some() || r.inconnu) && !accepte.unwrap_or(false) {
                return Err(Erreur::Refus("Ce jeu a peut-être du jeu en ligne : confirme d'abord que tu acceptes le risque.".into()));
            }
        }
    }
    let dossier = p.parent().map(std::path::PathBuf::from).unwrap_or_default();
    let profil = noyau.actif().await.ok_or_else(|| Erreur::Profil("Aucun profil n'est ouvert.".into()))?.nom;
    let (d, pr) = (dossier.clone(), profil.clone());
    tauri::async_runtime::spawn_blocking(move || crate::cheatengine::preparer(crate::cheatengine::CLE, &d, &pr))
        .await
        .map_err(|_| Erreur::Disque("La préparation s'est arrêtée brutalement.".into()))??;
    let table = table.filter(|t| t.to_lowercase().ends_with(".ct") && std::path::Path::new(t).is_file());
    let mut c = std::process::Command::new(&p);
    c.current_dir(&dossier);
    if let Some(partie) = branchee {
        // « Lancer avec Cheat Engine » : notre script autorun se branche seul sur le jeu en cours et charge la table.
        let jeu = partie.jeu;
        let pid = tauri::async_runtime::spawn_blocking(move || {
            crate::menu_jeu::processus_du_jeu(partie.pid, partie.dossier_jeu.as_deref().map(std::path::Path::new))
        })
        .await
        .map_err(|_| Erreur::Disque("La recherche du jeu s'est arrêtée brutalement.".into()))?;
        // La partie vérifiée est-elle toujours celle en cours ? (Un autre jeu lancé entre-temps n'a pas été vérifié.)
        if app.state::<crate::menu_jeu::MenuJeu>().partie().map(|p| p.jeu) != Some(jeu) {
            return Err(Erreur::Refus("La partie a changé pendant la vérification : réessaie.".into()));
        }
        crate::cheatengine::preparer_branchement(&dossier, pid, table.as_deref().map(std::path::Path::new))?;
    } else if let Some(t) = table {
        c.arg(t);
    }
    let pid = c.spawn().map_err(|e| Erreur::Disque(format!("Cheat Engine ne démarre pas ({e}).")))?.id();
    noyau.journaliser(&format!("Cheat Engine lancé pour {profil}"));
    // À sa fermeture (Cheat Engine peut relancer sa version 64 bits : on suit tout ce qui vient de son dossier).
    tauri::async_runtime::spawn_blocking(move || {
        let mut s = crate::lancement::Suivi::nouveau(pid, vec![dossier.clone()]);
        while s.en_cours() {
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
        let _ = crate::cheatengine::ranger(crate::cheatengine::CLE, &dossier, &profil);
    });
    Ok(())
}

/// Ce que Frogtend sait d'un jeu IMPORTÉ (sans fiche chez Firehouse) pour chercher ses codes et mods (contrat 1.19,
/// 25a) : titre, plateforme (toujours : sans elle, aucun mod), nom du fichier du jeu (jamais son chemin), numéro de
/// série PS3. `None` : un jeu de Firehouse (sa fiche suffit). Lu avant de prendre la session (qui ne se garde pas).
/// Où chercher les codes et mods d'un jeu : sa fiche chez Firehouse, ou la recherche du contrat 1.19 (jeu importé).
enum OuChercher {
    Fiche(i64),
    Recherche(Vec<(&'static str, String)>),
}

async fn criteres_jeu_importe(noyau: &Noyau, id: i64) -> Resultat<OuChercher> {
    let j = noyau.jeu_visible(id).await?;
    if !crate::import_local::est_importe(&j) {
        return Ok(OuChercher::Fiche(id));
    }
    // Un jeu importé relié à une fiche que ce profil voit (contrat 1.21) : les codes et mods de SA fiche.
    let s = noyau.session().await?;
    if let Some(f) = noyau.fiche_de(&s, id)? {
        return Ok(OuChercher::Fiche(f));
    }
    let fichier = j.installation.as_ref().and_then(|i| i.fichier_du_jeu.as_ref().map(|f| std::path::Path::new(&i.dossier).join(f)));
    let rom = fichier.as_deref().and_then(|f| f.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let serial = match (crate::succes::console_de(&j.plateforme), fichier) {
        (Some(82), Some(f)) => tauri::async_runtime::spawn_blocking(move || crate::contenus::titre_id_ps3(&f))
            .await
            .ok()
            .flatten()
            .filter(|n| crate::emulateurs_profils::numero_ps3_sur(n))
            .unwrap_or_default(),
        _ => String::new(),
    };
    // Un numéro PS3 suffit ; un « EBOOT.BIN » (jeu PS3 en dossier) ne dit rien du jeu : pas envoyé.
    let rom = if !serial.is_empty() || rom.eq_ignore_ascii_case("EBOOT.BIN") { String::new() } else { rom };
    // Un jeu Steam (lot B, 21/10) : son numéro Steam, que Firehouse sait lire (contrat 1.19).
    let steam_appid = j
        .installation
        .as_ref()
        .and_then(|i| i.boutique.as_ref())
        .filter(|b| b.source == "steam" && !b.cle.is_empty() && b.cle.bytes().all(|c| c.is_ascii_digit()))
        .map(|b| b.cle.clone())
        .unwrap_or_default();
    Ok(OuChercher::Recherche(vec![
        ("titre", j.titre),
        ("plateforme", j.plateforme),
        ("rom", rom),
        ("serial", serial),
        ("steam_appid", steam_appid),
    ]))
}

/// Ce que risque un compte si Cheat Engine se branche sur ce jeu (Seb, 21/10) : un anti-triche connu = refus ; un jeu
/// en ligne, ou un jeu de boutique dont on ne sait rien = avertissement puis oui de la personne.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct RisqueTriche {
    pub anti_triche: Option<String>,
    /// Ce que dit PCGamingWiki (par Firehouse) du jeu en ligne : `true`, `limited`… (`None` : pas de jeu en ligne connu).
    pub en_ligne: Option<String>,
    /// On ne sait pas (dossier trop grand pour être regardé, ou Firehouse ne sait rien d'un jeu de boutique).
    pub inconnu: bool,
}

/// Le risque d'après le dossier du jeu (`local`) et le bloc `en_ligne` de Firehouse (`null` = inconnu, jamais « sans »).
fn risque_de(local: crate::figer::Verification, en_ligne: &Value, boutique: bool) -> RisqueTriche {
    let texte = |v: &Value| v.as_str().map(str::trim).filter(|t| !t.is_empty() && !t.eq_ignore_ascii_case("false")).map(String::from);
    // Contrat 1.25 : `anticheat` (un nom) ET `anticheats` (la liste complète) — tous dits, sans doublon.
    let mut noms: Vec<String> = texte(&en_ligne["anticheat"]).into_iter().collect();
    for a in en_ligne["anticheats"].as_array().into_iter().flatten().filter_map(texte) {
        if !noms.iter().any(|n| n.eq_ignore_ascii_case(&a)) {
            noms.push(a);
        }
    }
    // Firehouse SAIT quelque chose dès qu'il a répondu sur l'anti-triche, même « aucun » (`"false"`, liste vide).
    let firehouse_sait = !noms.is_empty() || !en_ligne["anticheat"].is_null() || en_ligne["anticheats"].is_array();
    let mut r = RisqueTriche { anti_triche: (!noms.is_empty()).then(|| noms.join(", ")), en_ligne: texte(&en_ligne["online"]), inconnu: false };
    match local {
        crate::figer::Verification::AntiTriche(a) => r.anti_triche = Some(a.to_string()),
        crate::figer::Verification::Inconnu => r.inconnu = true,
        crate::figer::Verification::Aucun => {}
    }
    if boutique && !firehouse_sait && en_ligne["online"].is_null() {
        r.inconnu = true;
    }
    r
}

/// Le risque de brancher Cheat Engine sur ce jeu (un jeu PC ; un jeu d'émulateur ne risque rien).
async fn risque_cheatengine(noyau: &Noyau, id: i64) -> Resultat<RisqueTriche> {
    let j = noyau.jeu_visible(id).await?;
    let Some(i) = j.installation.clone().filter(|i| i.fichier_du_jeu.is_none()) else { return Ok(RisqueTriche::default()) };
    let boutique = i.boutique.is_some();
    let dossier = std::path::PathBuf::from(&i.dossier);
    let local = tauri::async_runtime::spawn_blocking(move || crate::figer::anti_triche(&dossier))
        .await
        .map_err(|_| Erreur::Disque("La recherche d'un anti-triche s'est arrêtée brutalement.".into()))?;
    // Hors ligne ou Firehouse en panne : on ne sait pas ce qu'il dit (jamais « sans anti-triche »).
    let en_ligne = match criteres_jeu_importe(noyau, id).await {
        Ok(c) => match noyau.session().await {
            Ok(s) => triches_avec(&s.source, &c).await.map(|v| v["en_ligne"].clone()).unwrap_or(Value::Null),
            Err(_) => Value::Null,
        },
        Err(_) => Value::Null,
    };
    Ok(risque_de(local, &en_ligne, boutique))
}

/// Le risque de brancher Cheat Engine sur ce jeu, à montrer AVANT (refus ou avertissement).
#[tauri::command]
pub async fn cheatengine_risque(noyau: State<'_, Noyau>, id: i64) -> Resultat<RisqueTriche> {
    risque_cheatengine(&noyau, id).await
}

/// Les codes et mods d'un jeu : sa fiche chez Firehouse, ou la recherche pour un jeu importé (`criteres`).
async fn triches_avec(source: &crate::source::Source, criteres: &OuChercher) -> Resultat<Value> {
    match criteres {
        OuChercher::Recherche(c) => source.triche_recherche(c).await,
        OuChercher::Fiche(f) => source.triches(*f).await,
    }
}

/// Les triches et mods connus d'un jeu (contrat 13 ; jeu importé : la recherche du contrat 1.19).
#[tauri::command]
pub async fn jeu_triches(noyau: State<'_, Noyau>, id: i64) -> Resultat<Value> {
    let criteres = criteres_jeu_importe(&noyau, id).await?;
    triches_avec(&noyau.session().await?.source, &criteres).await
}

// --- Mods GitHub (contrat 1.19 ; ✅ Seb 19/10, lot 0.81.0) : le mod et son fichier sont REPRIS de la liste de
// Firehouse (jamais d'une adresse envoyée par l'interface) ; installés dans le dossier du jeu après le oui. ---

/// Un jeu du PC prêt à recevoir (ou perdre) un mod.
struct PourMod {
    j: crate::jeux_pc::JeuPc,
    /// Où le mod se pose : le dossier du jeu (chez Steam pour un jeu Steam).
    dossier: std::path::PathBuf,
    /// Où Frogtend garde manifestes, téléchargements et originaux remplacés : `<jeu>\Frogtend\mods` pour un jeu de
    /// Firehouse (comme avant) ; dans les données de Frogtend pour un jeu Steam (Seb, 21/10 : rien d'autre chez Steam).
    garde: std::path::PathBuf,
    /// Un jeu Steam : sa version installée à cet instant (`buildid`).
    steam: Option<Option<String>>,
}

/// Le dossier d'un jeu du PC prêt à recevoir un mod : installé, pas en cours d'utilisation, sans anti-triche détecté ;
/// un jeu de boutique : Steam seulement (lot B3), prêt (ni mise à jour ni partie en cours chez Steam).
async fn jeu_pour_un_mod(noyau: &Noyau, id: i64) -> Resultat<PourMod> {
    let j = noyau.jeu_visible(id).await?;
    let i = j.installation.as_ref().ok_or_else(|| Erreur::Refus("Installe d'abord le jeu.".into()))?;
    let dossier = std::path::PathBuf::from(&i.dossier);
    if !dossier.is_dir() {
        return Err(Erreur::Disque(format!("Le dossier du jeu est introuvable : {}.", dossier.display())));
    }
    if i.fichier_du_jeu.is_some() {
        return Err(Erreur::Refus("Ce jeu se lance par un émulateur : Frogtend n'y installe pas de mod de jeu PC.".into()));
    }
    if noyau.utilise(id) {
        return Err(Erreur::Refus("Le jeu est en cours (partie, téléchargement ou installation) : réessaie après.".into()));
    }
    let Some(b) = &i.boutique else {
        let garde = crate::mods::dossier_mods(&dossier);
        return Ok(PourMod { j, dossier, garde, steam: None });
    };
    if b.source != "steam" {
        return Err(Erreur::Refus("Frogtend sait poser des mods dans les jeux Steam seulement, pour l'instant.".into()));
    }
    let buildid = steam_pret_a_ecrire(&b.cle, &dossier)?;
    // Les données de Frogtend pour ce jeu, JAMAIS chez Steam ni par un lien.
    let garde = std::path::Path::new(&j.dossier).join("mods");
    if crate::garde::sous(&garde, &dossier) || crate::garde::meme(&garde, &dossier) || crate::garde::sous(&dossier, &garde) {
        return Err(Erreur::Refus("Le dossier de Frogtend pour ce jeu est mal placé : rien n'est fait.".into()));
    }
    std::fs::create_dir_all(&garde)?;
    if crate::garde::lien_entre(std::path::Path::new(&j.dossier), &garde) {
        return Err(Erreur::Refus("Le dossier de Frogtend pour ce jeu passe par un lien : rien n'est fait.".into()));
    }
    Ok(PourMod { j, dossier, garde, steam: Some(buildid) })
}

/// Un jeu Steam relu chez Steam À L'INSTANT (jamais l'état gardé par l'interface) avant d'écrire dans son dossier :
/// au même endroit, prêt (ni mise à jour ni vérification), pas en cours (ni pour Steam, ni un programme de son
/// dossier). Rend sa version installée (`buildid`).
fn steam_pret_a_ecrire(appid: &str, dossier: &std::path::Path) -> Resultat<Option<String>> {
    let installe = crate::boutiques::dossier_steam()
        .map(|s| crate::boutiques::installes_steam_dossiers(&s))
        .and_then(|mut l| l.remove(appid))
        .filter(|x| crate::garde::meme(&x.dossier, dossier))
        .ok_or_else(|| Erreur::Refus("Steam ne voit plus ce jeu à cet endroit : ouvre Steam, puis réessaie.".into()))?;
    if !installe.pret {
        return Err(Erreur::Refus("Steam prépare ce jeu (mise à jour, vérification…) : attends qu'il ait fini.".into()));
    }
    if installe.lance || !crate::lancement::processus_du_dossier(dossier).is_empty() {
        return Err(Erreur::Refus("Le jeu est en cours : quitte-le d'abord.".into()));
    }
    Ok(installe.buildid)
}

/// Juste avant de poser ou de retirer un mod dans un jeu Steam (le téléchargement a pu durer) : toujours prêt, et
/// toujours la MÊME version qu'à la vérification (sinon Steam vient de le mettre à jour : on recommence).
fn steam_inchange(j: &crate::jeux_pc::JeuPc, dossier: &std::path::Path, vu: &Option<String>) -> Resultat<()> {
    let Some(b) = j.installation.as_ref().and_then(|i| i.boutique.as_ref()) else { return Ok(()) };
    if steam_pret_a_ecrire(&b.cle, dossier)? != *vu {
        return Err(Erreur::Refus("Steam vient de mettre le jeu à jour : rien n'est fait. Réessaie maintenant.".into()));
    }
    Ok(())
}

/// Avant un mod dans un jeu Steam : ses parties gardées pour le nuage sont mises à l'abri (lot B2) s'il y en a et si
/// elles ont changé. Un jeu dont Steam ne garde rien : noté au journal (le mod ne touche que le dossier du jeu, et
/// tout fichier qu'il remplace y est déjà gardé).
async fn abriter_avant_mod(noyau: &Noyau, id: i64) -> Resultat<()> {
    let (mut plan, marque) = match plan_nuage(noyau, id).await {
        Ok(x) => x,
        Err(e) => {
            noyau.journaliser(&format!("jeu {id} : pas de parties du nuage Steam à mettre à l'abri avant le mod ({e})"));
            return Ok(());
        }
    };
    // Avant un mod, jamais une très grosse copie sans question : au-delà de `GROS`, les parties du nuage seulement.
    if plan.octets > crate::nuage_steam::GROS {
        plan.fichiers.retain(|f| !f.relatif.contains("/hors-nuage/"));
        plan.octets = plan.fichiers.iter().map(|f| f.taille).sum();
        noyau.journaliser(&format!("jeu {id} : parties hors du nuage trop grosses pour être copiées avant le mod (abri du nuage seulement)"));
    }
    let abris = noyau.dossier_abris()?;
    if plan.fichiers.is_empty() || crate::nuage_steam::deja_a_l_abri(&abris.join(id.to_string()), &marque, &plan) {
        return Ok(());
    }
    let quand = crate::noyau::maintenant();
    tauri::async_runtime::spawn_blocking(move || crate::nuage_steam::abriter(&plan, &marque, &abris, id, &quand))
        .await
        .map_err(|_| Erreur::Disque("La mise à l'abri s'est arrêtée brutalement.".into()))??;
    Ok(())
}

/// Un jeu Steam (avis de Firehouse, contrat 1.19 `en_ligne`) : un anti-triche connu = jamais de mod ; un jeu en ligne
/// ou inconnu = seulement après le oui de la personne (même règle que Cheat Engine, Seb 21/10).
async fn risque_mod_steam(noyau: &Noyau, id: i64, accepte: bool) -> Resultat<()> {
    let r = risque_cheatengine(noyau, id).await?;
    if let Some(a) = r.anti_triche {
        return Err(Erreur::Refus(format!("Ce jeu est protégé par {a} : Frogtend n'y installe pas de mod (ton compte pourrait être banni).")));
    }
    if (r.en_ligne.is_some() || r.inconnu) && !accepte {
        return Err(Erreur::Refus("Ce jeu a peut-être du jeu en ligne : confirme d'abord que tu acceptes le risque.".into()));
    }
    Ok(())
}

/// Où en est l'installation d'un mod (lot 0.82.3) : l'interface en tire la progression, le débit et le temps restant.
#[derive(Clone, Serialize)]
struct ProgresMod {
    /// Le numéro de suivi donné par l'interface (deux installations ne se mélangent jamais).
    suivi: u32,
    etape: &'static str,
    faits: u64,
    total: u64,
    /// Octets par seconde depuis le dernier envoi (0 : inconnu, ou une étape qui compte des fichiers).
    debit: u64,
}

/// Le suivi d'une installation de mod : un événement `mod-progres` au plus toutes les 400 ms (comme les
/// téléchargements de jeux), toujours à un changement d'étape et à la fin ; le débit est calculé ici.
/// La réponse attendue à « où poser ce mod rangé dans un seul dossier ? » (une seule question à la fois).
static REPONSE_RACINE: std::sync::Mutex<Option<std::sync::mpsc::Sender<Option<bool>>>> = std::sync::Mutex::new(None);

/// Ce qu'on demande à l'interface quand l'archive d'un mod est rangée dans UN seul dossier.
#[derive(Clone, Serialize)]
struct QuestionRacine {
    mod_nom: String,
    dossier: String,
}

/// Le mod est-il rangé dans un seul dossier (« MonMod/… ») ? Alors la personne choisit (✅ Seb 25/10) : poser son
/// CONTENU directement (`true`) ou le dossier tel quel (`false`). Rien n'est posé avant sa réponse ; pas de réponse
/// (fenêtre fermée, 10 min) : rien n'est posé. `demander` envoie la question à l'interface.
async fn choisir_racine(archive: &std::path::Path, mod_nom: &str, demander: impl Fn(QuestionRacine)) -> Resultat<bool> {
    let a = archive.to_path_buf();
    let racine = tauri::async_runtime::spawn_blocking(move || crate::mods::lister(&a).map(|l| crate::mods::racine_unique(&l)))
        .await
        .map_err(|_| Erreur::Disque("La lecture du mod s'est arrêtée brutalement.".into()))??;
    let Some(dossier) = racine else { return Ok(false) };
    let (tx, rx) = std::sync::mpsc::channel();
    *REPONSE_RACINE.lock().unwrap_or_else(|e| e.into_inner()) = Some(tx);
    demander(QuestionRacine { mod_nom: mod_nom.to_string(), dossier });
    let r = tauri::async_runtime::spawn_blocking(move || rx.recv_timeout(std::time::Duration::from_secs(600)).ok().flatten())
        .await
        .map_err(|_| Erreur::Disque("La question s'est arrêtée brutalement.".into()))?;
    r.ok_or_else(|| Erreur::Refus("Installation annulée : rien n'a été posé.".into()))
}

/// La réponse de la personne : `Some(true)` son contenu directement, `Some(false)` le dossier tel quel, `None` annuler.
#[tauri::command]
pub fn mod_racine_repondre(choix: Option<bool>) {
    if let Some(tx) = REPONSE_RACINE.lock().unwrap_or_else(|e| e.into_inner()).take() {
        let _ = tx.send(choix);
    }
}

fn suivi_mod(app: &AppHandle, suivi: u32) -> impl Fn(&'static str, u64, u64) + Send + Sync + Clone + 'static {
    let app = app.clone();
    let dernier = std::sync::Arc::new(std::sync::Mutex::new(None::<(std::time::Instant, &'static str, u64)>));
    move |etape, faits, total| {
        let mut d = dernier.lock().unwrap_or_else(|e| e.into_inner());
        let maintenant = std::time::Instant::now();
        let debit = match *d {
            Some((t, e, avant)) if e == etape => {
                let ms = maintenant.duration_since(t).as_millis() as u64;
                // Le dernier envoi passe toujours (fin d'étape) ; un total nul n'en fait jamais une rafale.
                if ms < 400 && (total == 0 || faits < total) {
                    return;
                }
                if etape == "pose" || ms == 0 { 0 } else { faits.saturating_sub(avant).saturating_mul(1000) / ms }
            }
            _ => 0,
        };
        *d = Some((maintenant, etape, faits));
        let _ = app.emit("mod-progres", ProgresMod { suivi, etape, faits, total, debit });
    }
}

/// Un anti-triche dans le jeu : jamais de mod (un compte pourrait être banni).
async fn refuser_anti_triche(dossier: &std::path::Path, boutique: bool) -> Resultat<()> {
    let d = dossier.to_path_buf();
    let verification = tauri::async_runtime::spawn_blocking(move || crate::figer::anti_triche(&d))
        .await
        .map_err(|_| Erreur::Disque("La vérification anti-triche s'est arrêtée brutalement : rien n'est installé.".into()))?;
    verdict_anti_triche(verification, boutique)
}

/// Un anti-triche trouvé : refus. Trop de fichiers pour tout regarder : refus pour un jeu de BOUTIQUE (souvent en
/// ligne, comme pour figer le jeu) ; un jeu de Firehouse garde le comportement d'avant (ses gros repacks).
fn verdict_anti_triche(v: crate::figer::Verification, boutique: bool) -> Resultat<()> {
    match v {
        crate::figer::Verification::AntiTriche(a) => Err(Erreur::Refus(format!("Ce jeu est protégé par {a} : Frogtend n'y installe pas de mod."))),
        crate::figer::Verification::Inconnu if boutique => Err(Erreur::Refus(
            "Ce jeu a trop de fichiers pour vérifier qu'aucun anti-triche ne le protège : dans le doute, Frogtend n'y installe pas de mod.".into(),
        )),
        _ => Ok(()),
    }
}

/// Le dossier du programme du jeu, relatif au dossier du jeu (séparateur `/`).
fn dossier_du_programme(j: &crate::jeux_pc::JeuPc, dossier: &std::path::Path) -> Resultat<String> {
    let lanceur = j.installation.as_ref().and_then(|i| i.lanceur.as_ref()).map(|l| std::path::PathBuf::from(&l.dossier));
    // Sous le dossier du jeu (sans tenir compte des majuscules, comme Windows) : le reste du chemin.
    match lanceur.as_deref().filter(|l| crate::garde::sous(l, dossier)) {
        Some(l) => Ok(l.components().skip(dossier.components().count()).map(|c| c.as_os_str().to_string_lossy().to_string()).collect::<Vec<_>>().join("/")),
        None => Err(Erreur::Refus("Le programme du jeu n'est pas dans son dossier : choisis la racine du jeu.".into())),
    }
}

/// Un mod installé, et si Steam a mis le jeu à jour depuis sa pose (le mod a pu être défait).
#[derive(Serialize)]
pub struct ModInstalleVu {
    #[serde(flatten)]
    m: crate::mods::Manifeste,
    steam_mis_a_jour: bool,
}

/// Le dossier où Frogtend garde les mods d'un jeu (voir `PourMod::garde`), sans rien vérifier d'autre (lecture).
fn garde_mods(j: &crate::jeux_pc::JeuPc) -> Option<std::path::PathBuf> {
    let i = j.installation.as_ref()?;
    match &i.boutique {
        Some(b) if b.source == "steam" => Some(std::path::Path::new(&j.dossier).join("mods")),
        Some(_) => None,
        None => Some(crate::mods::dossier_mods(std::path::Path::new(&i.dossier))),
    }
}

/// La version installée d'un jeu Steam, relue chez Steam (`None` : inconnue).
fn buildid_steam(j: &crate::jeux_pc::JeuPc) -> Option<String> {
    let b = j.installation.as_ref()?.boutique.as_ref().filter(|b| b.source == "steam")?;
    crate::boutiques::installes_steam_dossiers(&crate::boutiques::dossier_steam()?).remove(&b.cle)?.buildid
}

/// Les mods installés dans un jeu du PC (manifestes du dossier de garde).
#[tauri::command]
pub async fn mods_installes(noyau: State<'_, Noyau>, id: i64) -> Resultat<Vec<ModInstalleVu>> {
    let j = noyau.jeu_visible(id).await?;
    let Some(garde) = garde_mods(&j) else { return Ok(Vec::new()) };
    Ok(tauri::async_runtime::spawn_blocking(move || {
        let actuel = buildid_steam(&j);
        crate::mods::installes_dans(&garde)
            .into_iter()
            .map(|m| ModInstalleVu {
                // Seulement quand les deux versions sont connues et diffèrent (jamais une alerte sur une inconnue).
                steam_mis_a_jour: matches!((&m.steam_buildid, &actuel), (Some(a), Some(b)) if a != b),
                m,
            })
            .collect()
    })
    .await
    .unwrap_or_default())
}

/// Télécharge et installe un mod (la personne a dit oui, taille et licence annoncées). `programme` : poser dans le
/// dossier du programme du jeu plutôt qu'à sa racine (mod « à confirmer », choisi par la personne). Les parties sont
/// d'abord mises à l'abri.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn mod_installer(
    app: AppHandle,
    noyau: State<'_, Noyau>,
    id: i64,
    mod_nom: String,
    fichier: String,
    programme: bool,
    suivi: u32,
    risque_accepte: Option<bool>,
) -> Resultat<crate::mods::Bilan> {
    let PourMod { j, dossier, garde, steam } = jeu_pour_un_mod(&noyau, id).await?;
    if steam.is_some() {
        risque_mod_steam(&noyau, id, risque_accepte.unwrap_or(false)).await?;
    }
    let criteres = criteres_jeu_importe(&noyau, id).await?;
    let liste = triches_avec(&noyau.session().await?.source, &criteres).await?;
    let m = liste["mods"]
        .as_array()
        .and_then(|l| l.iter().find(|m| m["nom"].as_str() == Some(mod_nom.as_str())).cloned())
        .ok_or_else(|| Erreur::Introuvable("Ce mod n'est plus proposé par Firehouse.".into()))?;
    if let Some(r) = m["refus"].as_str() {
        return Err(Erreur::Refus(format!("Firehouse ne donne pas de fichier pour ce mod ({r}).")));
    }
    let f = m["fichiers"]
        .as_array()
        .and_then(|l| l.iter().find(|x| x["nom"].as_str() == Some(fichier.as_str())))
        .and_then(crate::mods::FichierMod::depuis)
        .ok_or_else(|| Erreur::Refus("Frogtend ne sait pas installer ce fichier d'un clic : suis la page du mod.".into()))?;
    refuser_anti_triche(&dossier, j.installation.as_ref().is_some_and(|i| i.boutique.is_some())).await?;
    // Où poser : le dossier du mod (`cible.dossier`) ; ou, pour un mod « à confirmer », le dossier du programme.
    if programme && steam.is_some() {
        return Err(Erreur::Refus("Frogtend ne connaît pas le programme d'un jeu Steam : choisis la racine du jeu.".into()));
    }
    let sous = if programme { dossier_du_programme(&j, &dossier)? } else { f.dossier.clone() };
    // Occupé dès maintenant : ni une partie ni un second clic ne passent entre la vérification et la pose.
    let _occupe = noyau.occuper(id);
    if steam.is_some() {
        abriter_avant_mod(&noyau, id).await?;
    } else {
        noyau.mettre_a_l_abri(id).await?;
    }
    let archive = garde.join("telechargements").join(f.nom_telechargement());
    let suivi = suivi_mod(&app, suivi);
    let s2 = suivi.clone();
    crate::mods::telecharger(&f, &archive, &move |faits, total| s2("telechargement", faits, total)).await?;
    let quand = crate::noyau::maintenant();
    // L'empreinte de l'archive reçue au manifeste (calculée si Firehouse n'en donnait pas).
    let sha = match f.sha256.clone() {
        Some(s) => s,
        None => crate::sauvegarde::empreinte(&archive)?,
    };
    let (d, g, a, f2, nom) = (dossier.clone(), garde.clone(), archive.clone(), f.clone(), mod_nom.clone());
    if let Some(vu) = &steam {
        steam_inchange(&j, &dossier, vu)?;
    }
    let buildid = steam.flatten();
    let app2 = app.clone();
    let sans_racine = match choisir_racine(&archive, &mod_nom, move |q| {
        let _ = app2.emit("mod-racine", q);
    })
    .await
    {
        Ok(c) => c,
        Err(e) => {
            let _ = std::fs::remove_file(&archive);
            return Err(e);
        }
    };
    let r = tauri::async_runtime::spawn_blocking(move || crate::mods::installer_dans(&d, &g, buildid, &a, &f2, &nom, &sha, &sous, sans_racine, &quand, &suivi))
        .await
        .map_err(|_| Erreur::Disque("L'installation du mod s'est arrêtée brutalement.".into()))?;
    let _ = std::fs::remove_file(&archive);
    let _ = std::fs::remove_dir(archive.parent().unwrap_or(&garde));
    match &r {
        Ok(b) => noyau.journaliser(&format!("mod « {mod_nom} » installé dans « {} » ({} ajouté(s), {} remplacé(s) — originaux à l'abri)", j.titre, b.ajoutes, b.remplaces)),
        Err(e) => noyau.journaliser(&format!("mod « {mod_nom} » non installé dans « {} » ({e})", j.titre)),
    }
    r
}

/// Désinstalle un mod : rend les fichiers GARDÉS parce que la personne les avait modifiés (copiés à l'abri), et ceux
/// laissés à Steam (remplacés par une mise à jour depuis la pose).
#[tauri::command]
pub async fn mod_desinstaller(noyau: State<'_, Noyau>, id: i64, cle: String) -> Resultat<crate::mods::Retrait> {
    let PourMod { j, dossier, garde, steam } = jeu_pour_un_mod(&noyau, id).await?;
    // Occupé dès maintenant : ni une partie ni un second clic ne passent entre la vérification et la pose.
    let _occupe = noyau.occuper(id);
    if steam.is_some() {
        abriter_avant_mod(&noyau, id).await?;
    } else {
        noyau.mettre_a_l_abri(id).await?;
    }
    // Steam a mis le jeu à jour depuis la pose : un fichier qui n'est plus celui du mod est le sien.
    let apres_maj = match (&steam, crate::mods::installes_dans(&garde).into_iter().find(|m| m.cle == cle).and_then(|m| m.steam_buildid)) {
        (Some(Some(actuel)), Some(pose)) => *actuel != pose,
        _ => false,
    };
    if let Some(vu) = &steam {
        steam_inchange(&j, &dossier, vu)?;
    }
    let (d, g, c) = (dossier.clone(), garde.clone(), cle.clone());
    let r = tauri::async_runtime::spawn_blocking(move || crate::mods::desinstaller_dans(&d, &g, &c, apres_maj))
        .await
        .map_err(|_| Erreur::Disque("Le retrait du mod s'est arrêté brutalement.".into()))?;
    if let Ok(x) = &r {
        noyau.journaliser(&format!(
            "mod {cle} retiré de « {} » ({} fichier(s) gardé(s), modifiés par la personne ; {} laissé(s) à Steam)",
            j.titre,
            x.gardes.len(),
            x.laisses_a_la_boutique.len()
        ));
    }
    r
}

// --- Nexus Mods (lot 0.82.0 ; ✅ Seb 19-20/10) : la personne clique « Mod Manager Download » sur le site ; le lien
// nxm:// arrive par la boîte aux lettres ; il va au jeu d'où la page a été ouverte ; rien ne s'installe sans son oui.
// La clé d'API PERSONNELLE du profil vit dans le coffre de Windows et ne part que vers api.nexusmods.com. ---

/// Le compte Nexus du profil (sans la clé, qui ne quitte jamais le coffre).
#[derive(Serialize)]
pub struct EtatNexus {
    pub compte: Option<String>,
    pub cle_enregistree: bool,
}

#[tauri::command]
pub async fn nexus_etat(noyau: State<'_, Noyau>) -> Resultat<EtatNexus> {
    let p = profil_ouvert(&noyau).await?;
    let c = crate::boutiques::lire_source(&dossier_profil_de(&noyau, &p.id), "nexus");
    Ok(EtatNexus {
        compte: c.map(|c| c.compte).filter(|c| !c.is_empty()),
        cle_enregistree: noyau.coffre.lire(&crate::boutiques::nom_secret(&p.id, "nexus"))?.is_some(),
    })
}

/// Règle la clé d'API personnelle : VÉRIFIÉE auprès de Nexus avant d'être rangée au coffre.
#[tauri::command]
pub async fn nexus_regler(noyau: State<'_, Noyau>, cle: String) -> Resultat<EtatNexus> {
    let p = profil_ouvert(&noyau).await?;
    let cle = cle.trim().to_string();
    let nom = crate::nexus::valider(crate::nexus::API, &cle).await?;
    noyau.coffre.ranger(&crate::boutiques::nom_secret(&p.id, "nexus"), &cle)?;
    drop(cle);
    let c = crate::boutiques::CompteBoutique { compte: nom, maj_le: crate::noyau::maintenant(), ..Default::default() };
    crate::boutiques::ecrire_source(&dossier_profil_de(&noyau, &p.id), "nexus", Some(&c))?;
    noyau.journaliser(&format!("clé Nexus Mods réglée pour le profil {}", p.id));
    nexus_etat(noyau).await
}

#[tauri::command]
pub async fn nexus_oublier(noyau: State<'_, Noyau>) -> Resultat<()> {
    let p = profil_ouvert(&noyau).await?;
    noyau.coffre.oublier(&crate::boutiques::nom_secret(&p.id, "nexus"))?;
    crate::boutiques::ecrire_source(&dossier_profil_de(&noyau, &p.id), "nexus", None)
}

/// Ouvre la page Nexus d'un mod de la liste de Firehouse (reprise dans la liste, jamais une adresse de l'interface) et
/// retient le clic : le lien nxm:// qui reviendra ira à CE jeu.
#[tauri::command]
pub async fn nexus_ouvrir(app: AppHandle, noyau: State<'_, Noyau>, id: i64, mod_nom: String) -> Resultat<()> {
    let p = profil_ouvert(&noyau).await?;
    noyau.jeu_visible(id).await?;
    let criteres = criteres_jeu_importe(&noyau, id).await?;
    let liste = triches_avec(&noyau.session().await?.source, &criteres).await?;
    let m = liste["mods"]
        .as_array()
        .and_then(|l| l.iter().find(|m| m["nom"].as_str() == Some(mod_nom.as_str())).cloned())
        .ok_or_else(|| Erreur::Introuvable("Ce mod n'est plus proposé par Firehouse.".into()))?;
    let url = ["url", "page"].iter().filter_map(|k| m[*k].as_str()).find(|u| crate::nexus::page_nexus(u).is_some()).ok_or_else(|| Erreur::Refus("Ce mod n'a pas de page sur Nexus.".into()))?;
    let (domaine, mod_id) = crate::nexus::page_nexus(url).ok_or_else(|| Erreur::Refus("Ce mod n'a pas de page sur Nexus.".into()))?;
    app.state::<crate::nexus::Nexus>().noter_clic(&p.id, id, &mod_nom, &domaine, mod_id);
    use tauri_plugin_opener::OpenerExt;
    app.opener().open_url(url, None::<&str>).map_err(|_| Erreur::Disque("Le navigateur ne s'ouvre pas.".into()))
}

/// Relève les liens nxm:// arrivés (profil ouvert seulement) et rend ceux de ce profil, sans leur clé.
#[tauri::command]
pub async fn nexus_liens(app: AppHandle, noyau: State<'_, Noyau>) -> Resultat<Vec<crate::nexus::LienRecu>> {
    let p = profil_ouvert(&noyau).await?;
    let donnees = app.path().app_data_dir().map_err(|_| Erreur::Disque("Dossier des données introuvable.".into()))?;
    let nexus = app.state::<crate::nexus::Nexus>();
    nexus.recevoir(&p.id, crate::nexus::relever(&donnees));
    Ok(nexus.liens(&p.id))
}

#[tauri::command]
pub async fn nexus_oublier_lien(app: AppHandle, noyau: State<'_, Noyau>, n: u32) -> Resultat<()> {
    let p = profil_ouvert(&noyau).await?;
    app.state::<crate::nexus::Nexus>().oublier(&p.id, n);
    Ok(())
}

/// Ce que la personne doit savoir avant de dire oui : le jeu, le mod, le nom et la taille du fichier (dits par Nexus).
#[derive(Serialize)]
pub struct InfosNexus {
    pub jeu_id: i64,
    pub titre: String,
    pub mod_nom: String,
    pub fichier: String,
    pub taille_ko: u64,
    /// Une archive que Frogtend sait poser (zip ou 7z).
    pub pris_en_charge: bool,
    pub expire: bool,
    /// Le jeu visé est un jeu Steam (lot B4 : racine seulement, risque demandé avant), dit par le cœur.
    pub steam: bool,
}

/// Le lien reçu, la clé du profil et le jeu visé (un lien sans clic depuis un jeu n'est pas installé).
async fn lien_nexus(app: &AppHandle, noyau: &Noyau, n: u32) -> Resultat<(crate::nexus::Recu, String, i64, String)> {
    let p = profil_ouvert(noyau).await?;
    let r = app.state::<crate::nexus::Nexus>().recu(&p.id, n).ok_or_else(|| Erreur::Introuvable("Ce lien de Nexus n'est plus là : reclique sur le site.".into()))?;
    let (Some(jeu), Some(mod_nom)) = (r.jeu_id, r.mod_nom.clone()) else {
        return Err(Erreur::Refus("Frogtend ne sait pas à quel jeu va ce mod : ouvre sa page depuis ⚙ Gérer le jeu ▸ 🧩 Mods.".into()));
    };
    let cle = noyau
        .coffre
        .lire(&crate::boutiques::nom_secret(&p.id, "nexus"))?
        .ok_or_else(|| Erreur::Refus("Règle d'abord ta clé Nexus dans ⚙ Options ▸ Comptes ▸ Nexus Mods.".into()))?;
    Ok((r, cle, jeu, mod_nom))
}

fn secondes_maintenant() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[tauri::command]
pub async fn nexus_lien_infos(app: AppHandle, noyau: State<'_, Noyau>, n: u32) -> Resultat<InfosNexus> {
    let (r, cle, jeu, mod_nom) = lien_nexus(&app, &noyau, n).await?;
    let j = noyau.jeu_visible(jeu).await?;
    let f = crate::nexus::fichier(crate::nexus::API, &r.lien, &cle).await?;
    let steam = j.installation.as_ref().and_then(|i| i.boutique.as_ref()).is_some_and(|b| b.source == "steam");
    Ok(InfosNexus {
        jeu_id: jeu,
        steam,
        titre: j.titre,
        mod_nom,
        pris_en_charge: crate::nexus::nom_pris_en_charge(&f.nom),
        fichier: f.nom,
        taille_ko: f.taille_ko,
        expire: crate::nexus::expire(&r.lien, secondes_maintenant()),
    })
}

/// Télécharge (taille de Nexus = plafond) et installe un mod de Nexus après le oui ; le fichier reçu doit être
/// EXACTEMENT celui demandé (`md5_search`), sinon rien n'est installé. Les parties sont d'abord mises à l'abri.
#[tauri::command]
pub async fn nexus_installer(app: AppHandle, noyau: State<'_, Noyau>, n: u32, programme: bool, suivi: u32, risque_accepte: Option<bool>) -> Resultat<crate::mods::Bilan> {
    let (r, cle, id, mod_nom) = lien_nexus(&app, &noyau, n).await?;
    if crate::nexus::expire(&r.lien, secondes_maintenant()) {
        return Err(Erreur::Refus("Ce lien de Nexus a expiré : reclique « Mod Manager Download » sur la page du mod.".into()));
    }
    let PourMod { j, dossier, garde, steam } = jeu_pour_un_mod(&noyau, id).await?;
    // Occupé aussitôt (aucune attente depuis la vérification) : ni une partie ni un second clic ne passent pendant les
    // appels à Nexus.
    let _occupe = noyau.occuper(id);
    if steam.is_some() {
        risque_mod_steam(&noyau, id, risque_accepte.unwrap_or(false)).await?;
    }
    refuser_anti_triche(&dossier, j.installation.as_ref().is_some_and(|i| i.boutique.is_some())).await?;
    if programme && steam.is_some() {
        return Err(Erreur::Refus("Frogtend ne connaît pas le programme d'un jeu Steam : choisis la racine du jeu.".into()));
    }
    let sous = if programme { dossier_du_programme(&j, &dossier)? } else { String::new() };
    let infos = crate::nexus::fichier(crate::nexus::API, &r.lien, &cle).await?;
    if !crate::nexus::nom_pris_en_charge(&infos.nom) {
        return Err(Erreur::Refus("Ce fichier n'est ni un .zip ni un .7z (un .rar, peut-être) : Frogtend ne sait pas encore le poser. Choisis un autre fichier sur la page du mod s'il y en a un, ou installe-le à la main.".into()));
    }
    let plafond = crate::nexus::plafond(infos.taille_ko);
    if plafond > crate::mods::DECOMPRESSE_MAX {
        return Err(Erreur::Refus("Ce fichier de Nexus est trop gros pour être installé d'un clic.".into()));
    }
    if steam.is_some() {
        abriter_avant_mod(&noyau, id).await?;
    } else {
        noyau.mettre_a_l_abri(id).await?;
    }
    let adresses = crate::nexus::adresses(crate::nexus::API, &r.lien, &cle).await?;
    let archive = garde.join("telechargements").join(format!("nexus-{}-{}.archive", r.lien.mod_id, r.lien.fichier_id));
    let client = crate::nexus::client_telechargement()?;
    let suivi = suivi_mod(&app, suivi);
    let _ramasse = crate::nexus::Ramasse(archive.clone());
    let mut recu = Err(Erreur::Reseau("Nexus n'a pas donné d'adresse de téléchargement.".into()));
    for a in &adresses {
        let s2 = suivi.clone();
        // Nexus ne donne qu'une taille arrondie : le total affiché est la sienne (en Ko).
        let annonce = infos.taille_ko.saturating_mul(1024);
        recu = crate::mods::telecharger_plafonne(&client, a, plafond, &archive, &move |faits, _| s2("telechargement", faits.min(annonce), annonce)).await;
        if recu.is_ok() {
            break;
        }
    }
    let taille = recu?;
    // Le format réel, sur les octets reçus (jamais d'après le nom).
    if crate::mods::format_mod(&archive).is_none() {
        return Err(Erreur::Refus("Ce fichier n'est ni un .zip ni un .7z (un .rar, peut-être) : Frogtend ne sait pas encore le poser. Choisis un autre fichier sur la page du mod s'il y en a un, ou installe-le à la main.".into()));
    }
    let a = archive.clone();
    let md5 = tauri::async_runtime::spawn_blocking(move || crate::nexus::md5_de(&a)).await.map_err(|_| Erreur::Disque("Vérification du fichier interrompue.".into()))??;
    if !crate::nexus::verifier_md5(crate::nexus::API, &r.lien, &md5, &cle).await? {
        return Err(Erreur::Conflit("Nexus ne reconnaît pas le fichier reçu comme celui de ce mod : rien n'est installé.".into()));
    }
    drop(cle);
    let f = crate::mods::FichierMod {
        nom: infos.nom.clone(),
        taille,
        sha256: None,
        url: String::new(),
        depot: format!("nexus/{}/{}", r.lien.jeu, r.lien.mod_id),
        tag: r.lien.fichier_id.to_string(),
        a_confirmer: true,
        dossier: String::new(),
    };
    let quand = crate::noyau::maintenant();
    let (d, g, a, f2, nom) = (dossier.clone(), garde.clone(), archive.clone(), f.clone(), mod_nom.clone());
    if let Some(vu) = &steam {
        steam_inchange(&j, &dossier, vu)?;
    }
    let buildid = steam.flatten();
    let app2 = app.clone();
    // L'archive est retirée par `_ramasse` quoi qu'il arrive (aussi si la personne annule).
    let sans_racine = choisir_racine(&archive, &mod_nom, move |q| {
        let _ = app2.emit("mod-racine", q);
    })
    .await?;
    let res = tauri::async_runtime::spawn_blocking(move || {
        crate::mods::installer_dans(&d, &g, buildid, &a, &f2, &nom, &crate::sauvegarde::empreinte(&a)?, &sous, sans_racine, &quand, &suivi)
    })
        .await
        .map_err(|_| Erreur::Disque("L'installation du mod s'est arrêtée brutalement.".into()))?;
    match &res {
        Ok(b) => {
            app.state::<crate::nexus::Nexus>().oublier(&profil_ouvert(&noyau).await?.id, n);
            noyau.journaliser(&format!("mod Nexus « {mod_nom} » installé dans « {} » ({} ajouté(s), {} remplacé(s) — originaux à l'abri)", j.titre, b.ajoutes, b.remplaces))
        }
        Err(e) => noyau.journaliser(&format!("mod Nexus « {mod_nom} » non installé dans « {} » ({e})", j.titre)),
    }
    res
}

/// Pose un fichier de triche de Firehouse dans le dossier du profil de l'émulateur (la personne l'a demandé). Un
/// fichier différent déjà là est d'abord copié à côté (`.avant-frogtend`). Rend le chemin écrit.
#[tauri::command]
pub async fn triche_installer(app: AppHandle, noyau: State<'_, Noyau>, id: i64, cle: String, programme: String, ligne: String) -> Resultat<String> {
    exiger_emulateur_pour_ecrire(&app, &noyau, std::path::Path::new(&programme))?;
    let criteres = criteres_jeu_importe(&noyau, id).await?;
    let s = noyau.session().await?;
    let liste = triches_avec(&s.source, &criteres).await?;
    let code = liste["codes"]
        .as_array()
        .and_then(|l| l.iter().find(|c| c["cle"].as_str() == Some(cle.as_str())).cloned())
        .ok_or_else(|| Erreur::Introuvable("Ce code n'est plus proposé par Firehouse.".into()))?;
    let chemin = std::path::Path::new(&programme);
    let nom_exe = chemin.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
    let emu = crate::emulateurs::CATALOGUE.iter().find(|f| nom_exe.starts_with(f.programme)).map(|f| f.id).unwrap_or("");
    let dossier_emu = chemin.parent().ok_or_else(|| Erreur::Disque("Programme de l'émulateur introuvable.".into()))?;
    // RetroArch : « cheats/{coeur} » → le nom du cœur choisi pour ce jeu (corename de son fichier info).
    let mut dossier_rel = code["dossier"].as_str().unwrap_or("").to_string();
    if dossier_rel.contains("{coeur}") {
        let coeur = crate::emulateurs::coeur_de(&ligne)
            .and_then(|c| crate::emulateurs::nom_du_coeur(dossier_emu, &c))
            .ok_or_else(|| Erreur::Refus("Le nom du cœur RetroArch de ce jeu est introuvable (fichier info du cœur absent).".into()))?;
        dossier_rel = dossier_rel.replace("{coeur}", &coeur);
    }
    let place = crate::emulateurs_profils::place_triche(
        emu,
        dossier_emu,
        &s.profil.nom,
        code["base"].as_str().unwrap_or("donnees"),
        &dossier_rel,
        code["nom_fichier"].as_str().ok_or_else(|| Erreur::Serveur("Firehouse n'a pas donné le nom du fichier.".into()))?,
    )?;
    // Jeu importé : le `fichier` de la liste repris tel quel (contrat 1.19) ; sinon la route de sa fiche.
    let octets = match &criteres {
        OuChercher::Recherche(_) => s.source.fichier_triche_route(code["fichier"].as_str().unwrap_or("")).await?,
        OuChercher::Fiche(f) => s.source.fichier_triche(*f, &cle).await?,
    };
    if let Some(p) = place.parent() {
        std::fs::create_dir_all(p)?;
    }
    if place.is_file() && std::fs::read(&place).ok().as_deref() != Some(octets.as_slice()) {
        std::fs::copy(&place, place.with_extension(format!("{}.avant-frogtend", place.extension().and_then(|e| e.to_str()).unwrap_or(""))))?;
    }
    std::fs::write(&place, &octets)?;
    noyau.journaliser(&format!("triche posée pour le jeu {id} : {}", place.display()));
    Ok(place.to_string_lossy().into())
}

/// La taille du dossier d'installation d'un jeu (pour annoncer une copie avant un mod).
#[tauri::command]
pub fn jeu_taille_installation(noyau: State<'_, Noyau>, id: i64) -> Resultat<u64> {
    let j = noyau.registre().jeu(id)?.ok_or_else(|| Erreur::Introuvable("Jeu introuvable sur ce PC.".into()))?;
    let i = j.installation.ok_or_else(|| Erreur::Refus("Le jeu n'est pas installé.".into()))?;
    let racine = std::path::PathBuf::from(&i.dossier);
    Ok(crate::installation::fichiers_de(&racine)?.iter().filter_map(|r| std::fs::metadata(racine.join(r)).ok()).map(|m| m.len()).sum())
}

/// Copie le jeu avant un mod (décision de Seb : proposée, pas obligatoire), DANS le dossier du jeu (règle « pas de
/// pieuvre ») : `<jeu>\Frogtend\copies\avant-mod-<date>\`. Ce que Frogtend a déjà mis sous `Frogtend` n'est pas copié.
#[tauri::command]
pub async fn jeu_copie_avant_mod(noyau: State<'_, Noyau>, id: i64) -> Resultat<String> {
    let j = noyau.registre().jeu(id)?.ok_or_else(|| Erreur::Introuvable("Jeu introuvable sur ce PC.".into()))?;
    let i = j.installation.clone().ok_or_else(|| Erreur::Refus("Le jeu n'est pas installé.".into()))?;
    // Un jeu de boutique : jamais de copie complète (Seb, 21/10 : parfois 100 Go ; « Vérifier l'intégrité » de la
    // boutique répare le jeu, et les fichiers remplacés par un mod sont gardés à part).
    if i.boutique.is_some() {
        return Err(Erreur::Refus("Un jeu de boutique n'est pas copié en entier : sa boutique sait le réparer, et Frogtend garde les fichiers qu'un mod remplace.".into()));
    }
    // Ni partie ni mod pendant la copie (elle peut prendre longtemps).
    let _occupe = noyau.occuper(id);
    let source = std::path::PathBuf::from(&i.dossier);
    let copie = crate::locale::dossier_frogtend(&j).join("copies").join(format!("avant-mod-{}", crate::noyau::maintenant()));
    let (s, c) = (source.clone(), copie.clone());
    let frogtend = crate::locale::dossier_frogtend(&j);
    let r = tauri::async_runtime::spawn_blocking(move || -> Resultat<()> {
        // La liste est faite AVANT de copier, et laisse de côté le dossier Frogtend (médias, copies précédentes).
        let fichiers: Vec<String> =
            crate::installation::fichiers_de(&s)?.into_iter().filter(|r| !s.join(r).starts_with(&frogtend)).collect();
        // La place d'abord : jamais une copie à moitié faute de place (et une marge pour Windows).
        let total: u64 = fichiers.iter().filter_map(|r| std::fs::metadata(s.join(r)).ok()).map(|m| m.len()).sum();
        if crate::jeux_pc::place_libre(&s).is_some_and(|l| l < total + crate::jeux_pc::MARGE_LIBRE) {
            return Err(Erreur::Refus("Pas assez de place sur ce disque pour copier le jeu : libère de la place, ou continue sans copie.".into()));
        }
        for r in fichiers {
            let cible = c.join(&r);
            if let Some(p) = cible.parent() {
                std::fs::create_dir_all(p)?;
            }
            std::fs::copy(s.join(&r), cible)?;
        }
        Ok(())
    })
    .await
    .map_err(|_| Erreur::Disque("La copie s'est arrêtée brutalement.".into()))?;
    if let Err(e) = r {
        // Une copie à moitié faite ne sert à rien : retirée (seulement elle, toute neuve).
        let _ = std::fs::remove_dir_all(&copie);
        return Err(e);
    }
    noyau.journaliser(&format!("copie avant mod du jeu {id} : {}", copie.display()));
    Ok(copie.to_string_lossy().into())
}

/// Installe le micrologiciel PS3 dans RPCS3 depuis un fichier PS3UPDAT.PUP, SANS fenêtre ni question
/// (`--headless --installfw`, relevé dans rpcs3.cpp le 03/10 : aucune boîte de dialogue sans interface). RPCS3 rend 0
/// même en cas d'échec : le résultat se vérifie en relisant la version installée. Rend cette version (« 4.93 »).
#[tauri::command]
pub async fn rpcs3_installer_micrologiciel(app: AppHandle, noyau: State<'_, Noyau>, programme: String, pup: String) -> Resultat<String> {
    exiger_programme_de_confiance(&app, &noyau, std::path::Path::new(&programme))?;
    installer_pup(programme, pup, None).await
}

/// La plateforme de la PS3 chez Firehouse (nom LaunchBox).
const PLATEFORME_PS3: &str = "Sony Playstation 3";

/// Le firmware que Firehouse sert pour une console (conseil, `type: installation`), s'il y en a un : pour l'annoncer
/// (nom, taille, note) avant de le proposer.
#[tauri::command]
pub async fn firmware_servi(noyau: State<'_, Noyau>, plateforme: String) -> Resultat<Option<crate::bios::BiosConseil>> {
    let v = noyau.session().await?.source.emulateurs(&plateforme).await?;
    let mut ignore = crate::bios::Bilan::default();
    let bios = crate::bios::lire_bios(&v["conseil"]["bios"], &mut ignore);
    Ok(crate::bios::firmware_servi(&bios).cloned())
}

/// Le micrologiciel PS3 SERVI PAR FIREHOUSE (conseil de la PS3, contrat 1.9) : téléchargé avec reprise et progression
/// (événement « emulateur », id « rpcs3-micrologiciel »), SHA-1 vérifié, puis installé sans fenêtre par RPCS3. Un seul
/// accord a été donné avant (sa taille annoncée).
#[tauri::command]
pub async fn rpcs3_micrologiciel_servi(app: AppHandle, noyau: State<'_, Noyau>, programme: String) -> Resultat<String> {
    let p = std::path::PathBuf::from(&programme);
    if p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase() != "rpcs3.exe" || !p.is_file() {
        return Err(Erreur::Refus("Ce n'est pas le programme de RPCS3.".into()));
    }
    exiger_programme_de_confiance(&app, &noyau, &p)?;
    let dossier = p.parent().map(std::path::PathBuf::from).unwrap_or_default();
    // AVANT de télécharger 200 Mo : RPCS3 fermé, et la place sur le disque.
    rpcs3_ferme(&dossier)?;
    let _occupe = occuper_installation(dossier.parent().unwrap_or(&dossier), &dossier, "RPCS3")?;
    let s = noyau.session().await?;
    let v = s.source.emulateurs(PLATEFORME_PS3).await?;
    let mut ignore = crate::bios::Bilan::default();
    let bios = crate::bios::lire_bios(&v["conseil"]["bios"], &mut ignore);
    let f = crate::bios::firmware_servi(&bios)
        .filter(|f| f.fichier.to_lowercase().ends_with(".pup"))
        .cloned()
        .ok_or_else(|| Erreur::Introuvable("Firehouse ne fournit pas (encore) le micrologiciel PS3.".into()))?;
    // Le PUP, puis RPCS3 qui l'extrait dans dev_flash : environ trois fois sa taille, par prudence.
    if let (Some(t), Some(libre)) = (f.taille, crate::jeux_pc::place_libre(&dossier)) {
        if libre < t.saturating_mul(3) {
            return Err(Erreur::Disque(format!(
                "Pas assez de place sur le disque de RPCS3 : il faut environ {} Mo libres.",
                t.saturating_mul(3) / 1_048_576
            )));
        }
    }
    let app2 = app.clone();
    let progres = move |recus, total| {
        let _ = app2.emit("emulateur", ProgresEmulateur { id: "rpcs3-micrologiciel".into(), recus, total });
    };
    let pup = crate::bios::telecharger_firmware(&s.source, &f, &dossier, &progres).await?;
    let r = installer_pup(programme, pup.to_string_lossy().into(), crate::bios::version_dans_note(&f.note)).await;
    let _ = std::fs::remove_file(&pup);
    let _ = std::fs::remove_dir(dossier.join(".telechargements"));
    r
}

const PLATEFORME_SWITCH: &str = "Nintendo Switch";

/// Où en est Eden pour le firmware : portable, clés posées, firmware déjà là.
#[derive(Serialize)]
pub struct EtatFirmwareEden {
    pub portable: bool,
    pub cles: bool,
    pub firmware: bool,
}

#[tauri::command]
pub fn eden_firmware(programme: String) -> Resultat<EtatFirmwareEden> {
    let p = std::path::PathBuf::from(&programme);
    if !crate::eden::est_eden(&programme) || !p.is_file() {
        return Err(Erreur::Refus("Ce n'est pas le programme d'Eden.".into()));
    }
    let d = p.parent().map(std::path::PathBuf::from).unwrap_or_default();
    Ok(EtatFirmwareEden { portable: crate::eden::portable(&d) && crate::eden::dossier_nand(&d).is_some(), cles: crate::eden::cles_presentes(&d), firmware: crate::eden::firmware_present(&d) })
}

/// Le firmware Switch SERVI PAR FIREHOUSE (conseil de la Switch, contrat 1.10 : un zip de .nca) : téléchargé avec reprise
/// et progression (événement « emulateur », id « eden-firmware »), SHA-1 vérifié, puis POSÉ par Frogtend dans le dossier
/// système VIDE d'Eden (jamais par le menu d'Eden, qui vide d'abord ce dossier). Eden fermé, portable, clés posées ; un
/// seul accord a été donné avant (sa taille annoncée). Rend le nombre de fichiers posés.
#[tauri::command]
pub async fn eden_firmware_servi(app: AppHandle, noyau: State<'_, Noyau>, programme: String) -> Resultat<usize> {
    let p = std::path::PathBuf::from(&programme);
    if !crate::eden::est_eden(&programme) || !p.is_file() {
        return Err(Erreur::Refus("Ce n'est pas le programme d'Eden.".into()));
    }
    exiger_programme_de_confiance(&app, &noyau, &p)?;
    let dossier = p.parent().map(std::path::PathBuf::from).unwrap_or_default();
    // AVANT de télécharger 320 Mo : tout ce qui ferait refuser la pose.
    if !crate::lancement::processus_du_dossier(&dossier).is_empty() {
        return Err(Erreur::Refus("Eden est ouvert : ferme-le (ou quitte ta partie), puis recommence.".into()));
    }
    let _occupe = occuper_installation(dossier.parent().unwrap_or(&dossier), &dossier, "Eden")?;
    if !crate::eden::portable(&dossier) || crate::eden::dossier_nand(&dossier).is_none() {
        return Err(Erreur::Refus("Eden n'est pas installé en mode portable : Frogtend n'écrit pas dans ses données de Windows.".into()));
    }
    if !crate::eden::cles_presentes(&dossier) {
        return Err(Erreur::Refus("Tes clés (prod.keys) ne sont pas encore dans Eden : sans elles, Eden refuse le firmware. Pose-les d'abord.".into()));
    }
    if crate::eden::firmware_present(&dossier) {
        return Err(Erreur::Refus("Un firmware est déjà installé dans Eden : il est laissé tel quel.".into()));
    }
    let s = noyau.session().await?;
    let v = s.source.emulateurs(PLATEFORME_SWITCH).await?;
    let mut ignore = crate::bios::Bilan::default();
    let bios = crate::bios::lire_bios(&v["conseil"]["bios"], &mut ignore);
    let f = crate::bios::firmware_servi(&bios)
        .filter(|f| f.fichier.to_lowercase().ends_with(".zip"))
        .cloned()
        .ok_or_else(|| Erreur::Introuvable("Firehouse ne fournit pas (encore) le firmware Switch.".into()))?;
    // Le zip, puis ses fichiers extraits à côté : environ trois fois sa taille, par prudence.
    if let (Some(t), Some(libre)) = (f.taille, crate::jeux_pc::place_libre(&dossier)) {
        if libre < t.saturating_mul(3) {
            return Err(Erreur::Disque(format!("Pas assez de place sur le disque d'Eden : il faut environ {} Mo libres.", t.saturating_mul(3) / 1_048_576)));
        }
    }
    let app2 = app.clone();
    let progres = move |recus, total| {
        let _ = app2.emit("emulateur", ProgresEmulateur { id: "eden-firmware".into(), recus, total });
    };
    let zip = crate::bios::telecharger_firmware(&s.source, &f, &dossier, &progres).await?;
    // Eden a pu être ouvert pendant le téléchargement : revérifié juste avant de poser.
    if !crate::lancement::processus_du_dossier(&dossier).is_empty() {
        let _ = std::fs::remove_file(&zip);
        return Err(Erreur::Refus("Eden a été ouvert pendant le téléchargement : ferme-le, puis recommence.".into()));
    }
    let (d, z) = (dossier.clone(), zip.clone());
    let r = tauri::async_runtime::spawn_blocking(move || crate::eden::poser_firmware(&d, &z))
        .await
        .map_err(|_| Erreur::Disque("La pose du firmware s'est arrêtée brutalement.".into()))?;
    let _ = std::fs::remove_file(&zip);
    let _ = std::fs::remove_dir(dossier.join(".telechargements"));
    if let Ok(n) = &r {
        noyau.journaliser(&format!("firmware Switch posé dans Eden : {n} fichier(s)"));
    }
    r
}

/// Installe un PS3UPDAT.PUP dans RPCS3, sans fenêtre (`--headless --installfw`), et rend la version installée.
/// `attendue` (la version annoncée par Firehouse) : l'installation n'est réussie que si RPCS3 la montre ensuite.
/// Relevé en réel le 16/10 : RPCS3 installe bien (4.91 → 4.93 par-dessus), puis plante en se fermant (bogue connu,
/// ticket RPCS3 16421) ; il refuse aussi de tourner depuis un dossier temporaire. Son code de sortie ne dit donc rien :
/// seul le résultat sur le disque compte. Les fichiers gardent la date de l'archive : on compare leur CONTENU.
async fn installer_pup(programme: String, pup: String, attendue: Option<String>) -> Resultat<String> {
    let p = std::path::PathBuf::from(&programme);
    let nom = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
    if nom != "rpcs3.exe" || !p.is_file() {
        return Err(Erreur::Refus("Ce n'est pas le programme de RPCS3.".into()));
    }
    if !pup.to_lowercase().ends_with(".pup") || !std::path::Path::new(&pup).is_file() {
        return Err(Erreur::Refus("Choisis le fichier PS3UPDAT.PUP.".into()));
    }
    let dossier = p.parent().map(std::path::PathBuf::from).unwrap_or_default();
    rpcs3_ferme(&dossier)?;
    tauri::async_runtime::spawn_blocking(move || {
        // RPCS3 rend 0 même quand il échoue : on compare la trace du micrologiciel AVANT et APRÈS.
        let avant = trace_micrologiciel(&dossier);
        let mut enfant = crate::lancement::outil_sans_fenetre(&p.to_string_lossy())
            .args(["--headless", "--installfw"])
            .arg(&pup)
            .current_dir(&dossier)
            .spawn()
            .map_err(|e| Erreur::Disque(format!("RPCS3 ne démarre pas ({e}).")))?;
        // Jamais d'attente sans fin : 10 minutes au plus, puis RPCS3 (lancé par Frogtend pour ça) est arrêté.
        let debut = std::time::Instant::now();
        loop {
            if enfant.try_wait().map_err(|e| Erreur::Disque(format!("RPCS3 : {e}.")))?.is_some() {
                break;
            }
            if debut.elapsed() > std::time::Duration::from_secs(600) {
                let _ = enfant.kill();
                let _ = enfant.wait();
                return Err(Erreur::Disque("RPCS3 n'a pas fini d'installer le micrologiciel en 10 minutes : arrêté.".into()));
            }
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
        let version = crate::emulateurs_profils::micrologiciel_rpcs3(&dossier)
            .ok_or_else(|| Erreur::Disque("RPCS3 n'a pas installé le micrologiciel (fichier abîmé ?).".into()))?;
        match attendue {
            Some(a) if version != a => Err(Erreur::Disque(format!("RPCS3 n'a pas installé la version {a} : le micrologiciel est en version {version}."))),
            Some(_) => Ok(version),
            None if avant.is_some() && trace_micrologiciel(&dossier) == avant => {
                Err(Erreur::Disque(format!("RPCS3 n'a rien changé : le micrologiciel reste en version {version}.")))
            }
            None => Ok(version),
        }
    })
    .await
    .map_err(|_| Erreur::Disque("L'installation du micrologiciel s'est arrêtée brutalement.".into()))?
}

/// RPCS3 ne doit pas tourner pendant qu'on change son micrologiciel (une partie en cours lit dev_flash).
fn rpcs3_ferme(dossier: &std::path::Path) -> Resultat<()> {
    if crate::lancement::processus_du_dossier(dossier).is_empty() {
        Ok(())
    } else {
        Err(Erreur::Refus("RPCS3 est ouvert : ferme-le (ou quitte ta partie), puis recommence.".into()))
    }
}

/// La trace du micrologiciel installé : le contenu de `dev_flash\vsh\etc\version.txt` (RPCS3 garde la date de
/// l'archive : seule la version change).
fn trace_micrologiciel(dossier: &std::path::Path) -> Option<Vec<u8>> {
    std::fs::read(dossier.join("dev_flash").join("vsh").join("etc").join("version.txt")).ok()
}

/// La version du micrologiciel PS3 installé dans ce RPCS3, s'il y en a un.
#[tauri::command]
pub fn rpcs3_micrologiciel(programme: String) -> Option<String> {
    std::path::Path::new(&programme).parent().and_then(crate::emulateurs_profils::micrologiciel_rpcs3)
}

/// Poser une question à l'assistant jeux de Firehouse (lot 7). La conversation est gardée par l'interface.
#[tauri::command]
pub async fn assistant_demander(
    noyau: State<'_, Noyau>,
    question: String,
    media_id: Option<i64>,
    historique: Vec<Value>,
) -> Resultat<Value> {
    let q = question.trim();
    if q.is_empty() {
        return Err(Erreur::Refus("Écris d'abord ta question.".into()));
    }
    noyau.session().await?.source.assistant(q, media_id, &historique).await
}

/// Note au journal une action proposée par l'assistant, acceptée ou refusée par la personne (jamais d'effet
/// silencieux : brief § 4).
#[tauri::command]
pub fn assistant_journal(noyau: State<'_, Noyau>, action: String, decision: String) {
    noyau.journaliser(&format!("assistant : action « {action} » → {decision}"));
}

/// Chercher un jeu dans toute la base LaunchBox de Firehouse, pour le demander (lot 6).
#[tauri::command]
pub async fn jeu_rechercher(noyau: State<'_, Noyau>, texte: String) -> Resultat<Value> {
    let t = texte.trim();
    if t.chars().count() < 2 {
        return Err(Erreur::Refus("Tape au moins deux lettres.".into()));
    }
    noyau.session().await?.source.rechercher(t).await
}

/// Demander un jeu absent à Firehouse (la personne a confirmé dans l'interface). Réponse neutre si refusé.
#[tauri::command]
pub async fn jeu_demander(noyau: State<'_, Noyau>, launchbox_id: i64) -> Resultat<()> {
    let s = noyau.session().await?;
    s.source.demander(launchbox_id).await?;
    noyau.journaliser(&format!("demande d'un jeu absent : LaunchBox {launchbox_id}"));
    Ok(())
}

/// Les émulateurs recommandés par Firehouse pour un système.
#[tauri::command]
pub async fn emulateurs_recommandes(noyau: State<'_, Noyau>, plateforme: String) -> Resultat<Value> {
    noyau.session().await?.source.emulateurs(&plateforme).await
}

fn registre_emulateurs(noyau: &Noyau) -> crate::emulateurs::Registre {
    crate::emulateurs::Registre::nouveau(&noyau.dossier.join("emulateurs.json"))
}

/// Ouvre la fenêtre de choix de Windows (fichier ou dossier) PAR LE CŒUR, au-dessus de la fenêtre qui la demande, et
/// retient le chemin choisi (durcissement, étape 4) : les commandes sensibles (lanceur, import, « installé ailleurs »,
/// adoption d'un émulateur) n'acceptent un chemin libre que s'il vient d'un tel choix. `None` : fenêtre fermée.
#[tauri::command]
pub async fn parcourir(
    window: tauri::WebviewWindow,
    noyau: State<'_, Noyau>,
    dossier: bool,
    titre: String,
    extensions: Option<Vec<String>>,
) -> Resultat<Option<String>> {
    use tauri_plugin_dialog::DialogExt;
    let mut f = window.dialog().file().set_parent(&window).set_title(titre);
    if let Some(e) = extensions.filter(|e| !e.is_empty()) {
        let e: Vec<&str> = e.iter().map(String::as_str).collect();
        f = f.add_filter("Fichiers", &e);
    }
    let choisi = tauri::async_runtime::spawn_blocking(move || if dossier { f.blocking_pick_folder() } else { f.blocking_pick_file() })
        .await
        .map_err(|_| Erreur::Disque("La fenêtre de choix s'est fermée brutalement.".into()))?;
    let Some(c) = choisi else { return Ok(None) };
    let p = c.into_path().map_err(|e| Erreur::Disque(format!("Chemin illisible ({e}).")))?;
    noyau.retenir_choix(dossier, &p);
    Ok(Some(p.to_string_lossy().to_string()))
}

/// Durcissement (étape 5) : le cœur va ÉCRIRE dans le dossier de ce programme d'émulateur (BIOS, cœur, réglages de
/// manette, triches) : il doit être de confiance (réglé, au registre, détecté), ou venir d'être choisi par la personne
/// (l'assistant pose les BIOS d'un émulateur montré à la main avant de l'enregistrer).
fn exiger_emulateur_pour_ecrire(app: &AppHandle, noyau: &Noyau, programme: &std::path::Path) -> Resultat<()> {
    if programme.is_file() && noyau.a_ete_choisi(false, programme) {
        return Ok(());
    }
    exiger_programme_de_confiance(app, noyau, programme)
}

/// Durcissement (étape 5) : un émulateur ne s'installe que dans le dossier des émulateurs ou des outils RÉGLÉ dans
/// `pc.json` (relu ici), jamais dans un dossier quelconque.
fn exiger_dossier_de_base(app: &AppHandle, dossier: &std::path::Path) -> Resultat<()> {
    // Même réglé, jamais la racine d'un disque, Windows ou Documents (réglage gardé tel quel, seulement refusé ici).
    let reglages = app.store("pc.json").ok().and_then(|s| s.get("reglages")).unwrap_or(Value::Null);
    let outils = reglages["dossierOutils"].as_str().is_some_and(|o| !o.trim().is_empty() && crate::garde::meme(dossier, std::path::Path::new(o.trim())));
    if let Some(m) = refus_dossier(dossier, if outils { "outils" } else { "emulateurs" }) {
        return Err(Erreur::Refus(format!("{m} (⚙ Options ▸ 📁 Emplacements)")));
    }
    if crate::garde::dossier_de_base_admis(&reglages, dossier) {
        return Ok(());
    }
    Err(Erreur::Refus(format!(
        "Frogtend n'installe un émulateur que dans le dossier des émulateurs (ou des outils) réglé : « {} » n'en est pas un. Règle-le d'abord dans Réglages ▸ Émulateurs.",
        dossier.display()
    )))
}

/// Durcissement : un programme que le cœur va LANCER doit être de confiance — réglé dans `pc.json` (relu ici, pas
/// transmis par l'interface), inscrit au registre des émulateurs de Frogtend (installé ou adopté), ou trouvé dans son
/// dossier habituel. Sinon : refusé, avec la raison.
fn exiger_programme_de_confiance(app: &AppHandle, noyau: &Noyau, programme: &std::path::Path) -> Resultat<()> {
    let reglages = app.store("pc.json").ok().and_then(|s| s.get("reglages")).unwrap_or(Value::Null);
    let r = crate::garde::Reglages::lire(&reglages);
    let mut connus: Vec<std::path::PathBuf> = registre_emulateurs(noyau).tous().into_iter().map(|e| e.programme.into()).collect();
    connus.extend(crate::emulateurs::detecter().into_iter().map(|e| std::path::PathBuf::from(e.programme)));
    if crate::garde::programme_de_confiance(programme, &r, &connus) {
        return Ok(());
    }
    noyau.journaliser(&format!("programme refusé (ni réglé ni connu) : {}", programme.display()));
    Err(Erreur::Refus(format!(
        "Frogtend ne lance que les émulateurs et outils réglés sur ce PC : « {} » n'en fait pas partie. Règle-le d'abord dans Réglages ▸ Émulateurs.",
        programme.display()
    )))
}

/// Les émulateurs de ce PC : ceux installés par Frogtend, et ceux trouvés dans leurs dossiers habituels.
#[tauri::command]
pub fn emulateurs_installes(noyau: State<'_, Noyau>) -> Vec<crate::emulateurs::EmulateurInstalle> {
    let mut l = registre_emulateurs(&noyau).tous();
    for e in crate::emulateurs::detecter() {
        if !l.iter().any(|x| x.id == e.id) {
            l.push(e);
        }
    }
    l
}

#[derive(Serialize)]
pub struct InfoEmulateur {
    pub id: String,
    pub nom: String,
    pub ligne: String,
    /// Frogtend sait l'installer depuis sa source officielle.
    pub installable: bool,
}

/// Ce que Frogtend sait d'un émulateur recommandé par Firehouse (son nom), ou `None`.
#[tauri::command]
pub fn emulateur_fiche(nom: String) -> Option<InfoEmulateur> {
    crate::emulateurs::fiche_pour(&nom).map(|f| InfoEmulateur {
        id: f.id.into(),
        nom: f.nom.into(),
        ligne: f.ligne.into(),
        installable: true,
    })
}

/// La dernière version officielle d'un émulateur (version, adresse, taille).
#[tauri::command]
pub async fn emulateur_derniere_version(noyau: State<'_, Noyau>, id: String) -> Resultat<crate::emulateurs::Paquet> {
    let f = crate::emulateurs::fiche(&id).ok_or_else(|| Erreur::Introuvable("Émulateur inconnu.".into()))?;
    // RetroArch hébergé : la version et la taille annoncées sont celles du paquet que Firehouse servira (sinon la
    // mise à jour serait proposée sans fin, et l'accord donné sur une autre taille).
    if f.id == "retroarch" {
        if let Some(h) = retroarch_prevu(&noyau).await {
            // `url` : la route de l'API (sans hôte), qui dit à l'interface que le paquet vient de Firehouse.
            return Ok(crate::emulateurs::Paquet { version: h.version, url: format!("{}{}", crate::firehouse::PREFIXE, h.route), taille: h.taille });
        }
    }
    crate::emulateurs::derniere_version(f).await
}

/// Le RetroArch que Firehouse héberge (contrat 1.34, `heberge: true`) : `None` s'il ne l'héberge pas, s'il est
/// injoignable, ou s'il manque l'empreinte, la route ou la version.
struct RetroarchHeberge {
    version: String,
    route: String,
    sha256: String,
    taille: Option<u64>,
}

async fn retroarch_prevu(noyau: &Noyau) -> Option<RetroarchHeberge> {
    let s = noyau.session().await.ok()?;
    lire_retroarch_heberge(&s.source.paquet_emulateur("retroarch").await.ok()?)
}

fn lire_retroarch_heberge(p: &serde_json::Value) -> Option<RetroarchHeberge> {
    if p["heberge"] != true {
        return None;
    }
    Some(RetroarchHeberge {
        version: p["version"].as_str().filter(|v| !v.is_empty())?.to_string(),
        route: p["url"].as_str()?.strip_prefix(crate::firehouse::PREFIXE)?.to_string(),
        sha256: p["sha256"].as_str().filter(|x| !x.is_empty())?.to_string(),
        taille: p["taille"].as_u64(),
    })
}

#[derive(Clone, Serialize)]
struct ProgresEmulateur {
    id: String,
    recus: u64,
    total: Option<u64>,
}

/// Installe (ou met à jour) un émulateur dans `dossier` (le dossier des émulateurs réglé par la personne), depuis
/// sa source officielle. La personne a donné son accord dans l'interface (taille annoncée).
#[tauri::command]
pub async fn emulateur_installer(
    app: AppHandle,
    noyau: State<'_, Noyau>,
    id: String,
    dossier: String,
) -> Resultat<crate::emulateurs::EmulateurInstalle> {
    let f = crate::emulateurs::fiche(&id).ok_or_else(|| Erreur::Introuvable("Émulateur inconnu.".into()))?;
    let base = std::path::PathBuf::from(&dossier);
    // Le dossier réglé d'abord ; s'il n'existe pas encore (le dossier proposé par « Prêt à jouer »), il est créé.
    exiger_dossier_de_base(&app, &base)?;
    let cible = base.join(f.nom);
    // Une seule installation à la fois dans ce dossier, jamais par-dessus l'émulateur ouvert.
    let _occupe = occuper_installation(&base, &cible, f.nom)?;
    if !base.is_dir() {
        std::fs::create_dir_all(&base).map_err(|e| Erreur::Disque(format!("Dossier impossible à créer : {dossier} ({e}).")))?;
    }
    let paquet = base.join(".telechargements").join(format!("{}.paquet", f.id));
    let (app2, id2) = (app.clone(), id.clone());
    let progres = move |recus, total| {
        let _ = app2.emit("emulateur", ProgresEmulateur { id: id2.clone(), recus, total });
    };
    // RetroArch HÉBERGÉ par Firehouse (contrat 1.34 : la version des cœurs stables qu'il sert, empreinte vérifiée
    // pendant la réception) ; sinon (pas hébergé, hors ligne, échec) sa source officielle, comme avant.
    let heberge = if f.id == "retroarch" { retroarch_heberge(&noyau, &paquet, &progres).await } else { None };
    let version = match heberge {
        Some(v) => v,
        None => {
            // Rien de Firehouse ne reste (ni paquet, ni paquet partiel) : la source officielle repart de zéro.
            let _ = std::fs::remove_file(&paquet);
            let _ = std::fs::remove_file(paquet.with_extension("part"));
            let paquet_info = crate::emulateurs::derniere_version(f).await?;
            crate::emulateurs::telecharger(&paquet_info.url, &paquet, &progres).await?;
            paquet_info.version
        }
    };
    let (p, c) = (paquet.clone(), cible.clone());
    let programme = tauri::async_runtime::spawn_blocking(move || crate::emulateurs::installer_paquet(f, &p, &c))
        .await
        .map_err(|_| Erreur::Disque("L'installation s'est arrêtée brutalement.".into()))??;
    let _ = std::fs::remove_file(&paquet);
    let _ = std::fs::remove_dir(base.join(".telechargements"));
    // RetroArch : ses profils de manette officiels, s'ils ne sont pas dans le paquet (annoncés avec l'installation).
    if f.id == "retroarch" && !crate::manettes::retroarch_a_ses_profils(&cible) {
        installer_profils_retroarch(&noyau, &cible).await?;
    }
    // RPCS3 : pas de question (ni de mise à jour) à son démarrage, même lancé hors de Frogtend (Game Bar…).
    if f.id == "rpcs3" {
        if let Err(e) = crate::emulateurs_profils::rpcs3_sans_boites(&cible) {
            noyau.journaliser(&format!("RPCS3 installé : réglages de démarrage non posés ({e})"));
        }
    }
    let e = crate::emulateurs::EmulateurInstalle {
        id: f.id.into(),
        nom: f.nom.into(),
        version: Some(version),
        dossier: cible.to_string_lossy().into(),
        programme: programme.to_string_lossy().into(),
        par_frogtend: true,
        installe_le: crate::noyau::maintenant(),
    };
    registre_emulateurs(&noyau).retenir(e.clone())?;
    Ok(e)
}

/// Les installations d'émulateurs et d'outils en cours, par dossier de base (en minuscules) : deux installations
/// dans le même dossier écriraient le même paquet (`.telechargements`) et la même cible (essai de Seb, 28/10 :
/// « Impossible d'accéder au disque »).
static INSTALLATIONS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// Libère le dossier à la fin de l'installation, même après une erreur.
pub(crate) struct Occupation(String);

impl Drop for Occupation {
    fn drop(&mut self) {
        INSTALLATIONS.lock().unwrap_or_else(|e| e.into_inner()).retain(|x| x != &self.0);
    }
}

/// Prend le dossier pour une installation : refusé (clairement, sans attendre) si une autre y est en cours, ou si le
/// programme à remplacer est ouvert.
pub(crate) fn occuper_installation(base: &std::path::Path, cible: &std::path::Path, nom: &str) -> Resultat<Occupation> {
    if !crate::lancement::processus_du_dossier(cible).is_empty() {
        return Err(Erreur::Refus(format!("{nom} est ouvert : ferme-le, puis recommence.")));
    }
    let cle = base.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_lowercase();
    let mut l = INSTALLATIONS.lock().unwrap_or_else(|e| e.into_inner());
    if l.contains(&cle) {
        return Err(Erreur::Refus(format!("Une installation est déjà en cours dans ce dossier ({nom} ou un autre) : attends qu'elle se termine.")));
    }
    l.push(cle.clone());
    Ok(Occupation(cle))
}

/// RetroArch servi par Firehouse (contrat 1.34, `heberge: true`) : téléchargé avec son empreinte sha256 vérifiée
/// pendant la réception. Rend sa version ; `None` (et rien de gardé) au moindre manque ou échec.
async fn retroarch_heberge(noyau: &Noyau, paquet: &std::path::Path, progres: &(dyn Fn(u64, Option<u64>) + Send + Sync)) -> Option<String> {
    let s = noyau.session().await.ok()?;
    let h = lire_retroarch_heberge(&s.source.paquet_emulateur("retroarch").await.ok()?)?;
    if let Some(d) = paquet.parent() {
        std::fs::create_dir_all(d).ok()?;
    }
    s.source.telecharger_controle(&h.route, paquet, Some(&h.sha256), &[], progres).await.ok()?;
    Some(h.version)
}

/// Retient un émulateur installé à la main (trouvé, ou montré par la personne).
#[tauri::command]
pub fn emulateur_adopter(
    app: AppHandle,
    noyau: State<'_, Noyau>,
    id: String,
    programme: String,
    nom: Option<String>,
    version: Option<String>,
) -> Resultat<crate::emulateurs::EmulateurInstalle> {
    // Un émulateur connu de Frogtend, ou décrit par Firehouse (son nom est alors donné).
    let nom = match crate::emulateurs::fiche(&id) {
        Some(f) => f.nom.to_string(),
        None => nom.filter(|n| !n.trim().is_empty()).ok_or_else(|| Erreur::Introuvable("Émulateur inconnu.".into()))?,
    };
    let p = std::path::PathBuf::from(&programme);
    if !p.is_file() {
        return Err(Erreur::Disque(format!("Programme introuvable : {programme}.")));
    }
    // Étape 4 : adopté, il devient « de confiance » (lançable) — seulement un émulateur trouvé dans son dossier
    // habituel, installé dans le dossier des émulateurs ou des outils réglé, ou choisi par la personne dans une fenêtre
    // ouverte par le cœur. Jamais un chemin libre.
    let reglages = app.store("pc.json").ok().and_then(|s| s.get("reglages")).unwrap_or(Value::Null);
    let bases: Vec<std::path::PathBuf> = ["dossierEmulateurs", "dossierOutils"]
        .iter()
        .filter_map(|k| reglages[*k].as_str().filter(|s| !s.trim().is_empty()).map(std::path::PathBuf::from))
        .collect();
    let admis = crate::emulateurs::detecter().iter().any(|e| crate::garde::meme(std::path::Path::new(&e.programme), &p))
        || bases.iter().any(|b| crate::garde::sous(&p, b))
        || noyau.consommer_choix(false, &p);
    if !admis {
        return Err(Erreur::Refus("Choisis le programme de l'émulateur avec l'explorateur : Frogtend n'adopte pas un chemin qu'il n'a pas vu choisir.".into()));
    }
    let e = crate::emulateurs::EmulateurInstalle {
        id,
        nom,
        par_frogtend: version.is_some(),
        version,
        dossier: p.parent().unwrap_or(&p).to_string_lossy().into(),
        programme,
        installe_le: crate::noyau::maintenant(),
    };
    registre_emulateurs(&noyau).retenir(e.clone())?;
    Ok(e)
}

#[derive(Serialize)]
pub struct EtatRetroArch {
    pub coeur: Option<String>,
    pub coeur_present: bool,
    pub dossier_bios: String,
    /// Les BIOS demandés qui manquent.
    pub bios_manquants: Vec<String>,
}

/// Ce qui manque à RetroArch pour un système : son cœur, ses BIOS.
#[tauri::command]
pub fn retroarch_etat(programme: String, ligne: String, bios: Vec<String>) -> EtatRetroArch {
    let dossier = std::path::Path::new(&programme).parent().map(std::path::PathBuf::from).unwrap_or_default();
    let coeur = crate::emulateurs::coeur_de(&ligne);
    let coeur_present = coeur.as_ref().is_some_and(|c| dossier.join("cores").join(c).is_file());
    let bios_dir = crate::emulateurs::dossier_bios_retroarch(&dossier);
    let bios_manquants = bios.into_iter().filter(|b| !b.is_empty() && !bios_dir.join(b).is_file()).collect();
    EtatRetroArch { coeur, coeur_present, dossier_bios: bios_dir.to_string_lossy().into(), bios_manquants }
}

/// Pose les fichiers système (BIOS) obligatoires que Firehouse fournit pour une console, d'après SON conseil relu ici
/// (jamais d'après l'interface), pour l'émulateur `programme` — seulement s'il est bien celui du conseil. Un seul accord
/// a été donné avant (« Prêt à jouer »). PCSX2 : son BIOS par défaut réglé (contrat 1.11).
#[tauri::command]
pub async fn bios_poser(app: AppHandle, noyau: State<'_, Noyau>, plateforme: String, programme: String) -> Resultat<crate::bios::Bilan> {
    exiger_emulateur_pour_ecrire(&app, &noyau, std::path::Path::new(&programme))?;
    let s = noyau.session().await?;
    let reponse = s.source.emulateurs(&plateforme).await?;
    let conseil = &reponse["conseil"];
    let Some(id) = conseil["emulateur"].as_str() else { return Ok(Default::default()) };
    let dossier = std::path::Path::new(&programme).parent().map(std::path::PathBuf::from).unwrap_or_default();
    if !programme_du_conseil(id, &programme) || !dossier.is_dir() {
        return Ok(Default::default());
    }
    let mut illisibles = crate::bios::Bilan::default();
    let bios = crate::bios::lire_bios(&conseil["bios"], &mut illisibles);
    let mut bilan = crate::bios::poser(&s.source, &bios, &dossier, id == "retroarch").await;
    bilan.echecs.extend(illisibles.echecs);
    if id == "pcsx2" {
        if let Err(e) = crate::bios::regler_pcsx2(&dossier, &bios) {
            noyau.journaliser(&format!("PCSX2 : BIOS par défaut non réglé ({e})"));
        }
    }
    if !bilan.echecs.is_empty() {
        noyau.journaliser(&format!("fichiers système de {plateforme} : {:?}", bilan.echecs));
    }
    Ok(bilan)
}

/// Le programme est-il celui de l'émulateur du conseil (id des sources de Firehouse : « retroarch », « pcsx2 »,
/// « dosbox_staging »…) ? Par la fiche de Frogtend quand il la connaît, sinon par le nom du programme.
pub fn programme_du_conseil(id: &str, programme: &str) -> bool {
    let nom = std::path::Path::new(programme).file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    match crate::emulateurs::fiche(&id.replace('_', "-")) {
        Some(f) => nom.starts_with(f.programme),
        None => !id.is_empty() && nom.replace(['-', '_'], "").contains(&id.to_lowercase().replace(['-', '_'], "")),
    }
}

/// Installe un cœur RetroArch SERVI PAR FIREHOUSE (vérifié ; jamais pris ailleurs).
#[tauri::command]
pub async fn retroarch_installer_coeur(app: AppHandle, noyau: State<'_, Noyau>, programme: String, coeur: String) -> Resultat<String> {
    if !coeur.ends_with("_libretro.dll") || !crate::emulateurs::nom_de_coeur_sur(&coeur) {
        return Err(Erreur::Refus("Nom de cœur invalide.".into()));
    }
    exiger_emulateur_pour_ecrire(&app, &noyau, std::path::Path::new(&programme))?;
    let dossier = std::path::Path::new(&programme).parent().map(std::path::PathBuf::from).unwrap_or_default();
    // Jamais pendant une autre installation dans le même dossier (RetroArch mis à jour…), ni RetroArch ouvert.
    let _occupe = occuper_installation(dossier.parent().unwrap_or(&dossier), &dossier, "RetroArch")?;
    installer_coeur_servi(&noyau.session().await?.source, &dossier, &coeur).await
}

/// Le cœur RetroArch SERVI PAR FIREHOUSE (contrat 1.12) : téléchargé par l'API (reprise), vérifié (zip puis `.dll`),
/// posé dans `cores\`. Un cœur que Firehouse ne sert pas n'est pris nulle part ailleurs.
pub async fn installer_coeur_servi(source: &crate::source::Source, retroarch: &std::path::Path, coeur: &str) -> Resultat<String> {
    if !crate::emulateurs::nom_de_coeur_sur(coeur) {
        return Err(Erreur::Refus("Nom de cœur invalide.".into()));
    }
    let base = coeur.trim_end_matches(".dll");
    // Déjà là : rien à télécharger (jamais remplacé).
    let deja = retroarch.join("cores").join(format!("{base}.dll"));
    if deja.is_file() {
        return Ok(deja.to_string_lossy().into());
    }
    let liste = source.coeurs_retroarch().await?;
    let servi = crate::emulateurs::coeur_servi(&liste, coeur)
        .cloned()
        .ok_or_else(|| Erreur::Introuvable(format!("Le cœur {base} n'est pas fourni par Firehouse : Frogtend ne le prend pas ailleurs.")))?;
    let route = servi
        .url
        .strip_prefix(crate::firehouse::PREFIXE)
        .ok_or_else(|| Erreur::Serveur("Firehouse n'a pas donné de route pour ce cœur.".into()))?
        .to_string();
    if !crate::emulateurs::nom_de_coeur_sur(&servi.coeur) {
        return Err(Erreur::Serveur("Firehouse a donné un nom de cœur invalide.".into()));
    }
    let attente = retroarch.join(".telechargements");
    // Le zip n'est pas immuable (contrat 1.12) : son empreinte dans le nom fait qu'un nouveau zip ne reprend jamais
    // le `.part` d'un ancien ; les restes d'une version précédente de CE cœur (fichiers de Frogtend) sont retirés.
    let nom = format!("{}-{}.zip", servi.coeur, servi.sha256_zip.chars().filter(char::is_ascii_hexdigit).take(16).collect::<String>());
    if let Ok(l) = std::fs::read_dir(&attente) {
        for e in l.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if n.starts_with(&format!("{}-", servi.coeur)) && !n.starts_with(&nom) && e.path().is_file() {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    let paquet = attente.join(&nom);
    source.telecharger_route(&route, &paquet, &|_, _| {}).await?;
    let (p, ra) = (paquet.clone(), retroarch.to_path_buf());
    let r = tauri::async_runtime::spawn_blocking(move || crate::emulateurs::installer_coeur(&p, &ra, &servi))
        .await
        .map_err(|_| Erreur::Disque("L'installation du cœur s'est arrêtée brutalement.".into()))?;
    let _ = std::fs::remove_file(&paquet);
    let _ = std::fs::remove_dir(&attente);
    Ok(r?.to_string_lossy().into())
}

/// Télécharge et ajoute les profils de manette officiels de RetroArch (jamais par-dessus un profil déjà là). Rend le
/// nombre de profils ajoutés. Contrat 1.34 : la version STABLE figée servie par Firehouse (empreinte vérifiée) ; si
/// Firehouse ne la sert pas, celle du buildbot de RetroArch.
async fn installer_profils_retroarch(noyau: &Noyau, retroarch: &std::path::Path) -> Resultat<usize> {
    let paquet = retroarch.join("autoconfig.zip.telechargement");
    // Chez Firehouse : seulement avec une session, une adresse de l'API et une empreinte (vérifiée pendant la
    // réception, le `.part` retiré si elle ne colle pas). Au moindre échec : le buildbot de RetroArch, comme avant.
    let depuis_firehouse = async {
        let s = noyau.session().await.ok()?;
        let a = s.source.autoconfig_retroarch().await.ok()??;
        let sha = a["sha256"].as_str().filter(|x| !x.is_empty())?.to_string();
        let route = a["url"].as_str()?.strip_prefix(crate::firehouse::PREFIXE)?.to_string();
        s.source.telecharger_controle(&route, &paquet, Some(&sha), &[], &|_, _| {}).await.ok()
    }
    .await;
    if depuis_firehouse.is_none() {
        let _ = std::fs::remove_file(&paquet);
        let _ = std::fs::remove_file(paquet.with_extension("part"));
        crate::emulateurs::telecharger(crate::manettes::URL_PROFILS_RETROARCH, &paquet, &|_, _| {}).await?;
    }
    let (p, d) = (paquet.clone(), retroarch.to_path_buf());
    let r = tauri::async_runtime::spawn_blocking(move || crate::emulateurs::installer_profils_manette(&p, &d))
        .await
        .map_err(|_| Erreur::Disque("L'installation des profils de manette s'est arrêtée brutalement.".into()))?;
    let _ = std::fs::remove_file(&paquet);
    r
}

#[derive(Serialize)]
pub struct ManetteReglee {
    /// Une configuration a été écrite.
    pub reglee: bool,
    /// Profils de manette ajoutés (RetroArch).
    pub profils_ajoutes: usize,
}

/// « Remettre la manette par défaut » pour un émulateur (joueur 1, profil ouvert). La personne a donné son accord
/// dans l'interface ; la configuration d'avant est copiée à l'abri, les touches du clavier sont gardées.
#[tauri::command]
pub async fn emulateur_regler_manette(app: AppHandle, noyau: State<'_, Noyau>, id: String, programme: String) -> Resultat<ManetteReglee> {
    crate::emulateurs::fiche(&id).ok_or_else(|| Erreur::Introuvable("Émulateur inconnu.".into()))?;
    exiger_emulateur_pour_ecrire(&app, &noyau, std::path::Path::new(&programme))?;
    let dossier = std::path::Path::new(&programme).parent().map(std::path::PathBuf::from).unwrap_or_default();
    if !dossier.is_dir() {
        return Err(Erreur::Disque(format!("Dossier introuvable : {}.", dossier.display())));
    }
    let profil = noyau.actif().await.ok_or_else(|| Erreur::Profil("Aucun profil n'est ouvert.".into()))?;
    let mut profils_ajoutes = 0;
    if id == "retroarch" && !crate::manettes::retroarch_a_ses_profils(&dossier) {
        profils_ajoutes = installer_profils_retroarch(&noyau, &dossier).await?;
    }
    let d = dossier.clone();
    let reglee = tauri::async_runtime::spawn_blocking(move || {
        let utilisateur = crate::emulateurs_profils::dossier_du_profil(&d, &profil.nom).join("User");
        crate::manettes::regler(&id, &d, Some(&utilisateur), true)
    })
    .await
    .map_err(|_| Erreur::Disque("Le réglage de la manette s'est arrêté brutalement.".into()))??;
    Ok(ManetteReglee { reglee, profils_ajoutes })
}

/// Les réglages de manette de référence d'un émulateur.
#[tauri::command]
pub fn references_manette(noyau: State<'_, Noyau>, id: String) -> Vec<crate::references::Reference> {
    crate::references::toutes(&noyau.dossier.join("references"), &id)
}

/// Les profils de manette enregistrés dans l'émulateur par le profil ouvert (à reprendre comme référence).
#[tauri::command]
pub async fn profils_manette_emulateur(
    noyau: State<'_, Noyau>,
    id: String,
    programme: String,
) -> Resultat<Vec<crate::references::ProfilNatif>> {
    let dossier = std::path::Path::new(&programme).parent().map(std::path::PathBuf::from).unwrap_or_default();
    let profil = noyau.actif().await.ok_or_else(|| Erreur::Profil("Aucun profil n'est ouvert.".into()))?;
    let utilisateur = crate::emulateurs_profils::dossier_du_profil(&dossier, &profil.nom).join("User");
    Ok(crate::references::profils_natifs(&id, &dossier, &utilisateur))
}

/// Fait d'un profil de l'émulateur une référence de ce PC (la personne l'a choisi dans l'interface).
#[tauri::command]
pub async fn reference_reprendre(
    noyau: State<'_, Noyau>,
    id: String,
    programme: String,
    genre: String,
    chemin: String,
    nom: String,
) -> Resultat<crate::references::Reference> {
    let dossier = std::path::Path::new(&programme).parent().map(std::path::PathBuf::from).unwrap_or_default();
    let profil = noyau.actif().await.ok_or_else(|| Erreur::Profil("Aucun profil n'est ouvert.".into()))?;
    let utilisateur = crate::emulateurs_profils::dossier_du_profil(&dossier, &profil.nom).join("User");
    // Seulement un profil de l'émulateur, jamais un fichier pris ailleurs.
    let natifs = crate::references::profils_natifs(&id, &dossier, &utilisateur);
    let p = natifs
        .iter()
        .find(|p| p.chemin == chemin && p.genre == genre)
        .ok_or_else(|| Erreur::Refus("Ce fichier n'est pas un profil de manette de l'émulateur.".into()))?;
    crate::references::reprendre(&noyau.dossier.join("references"), &id, &genre, std::path::Path::new(&p.chemin), &nom)
}

/// Prépare l'émulateur pour le profil ouvert (ses parties à lui, les dossiers de jeux de Frogtend) et rend
/// (programme, ligne de commande complétée).
async fn preparer_emulateur(
    app: &AppHandle,
    noyau: &Noyau,
    plateforme: &str,
    programme: &str,
    ligne: &str,
    commandes: &Commandes,
) -> Resultat<(String, String)> {
    let chemin = std::path::Path::new(programme);
    let nom = chemin.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
    // MAME (paquet de Firehouse, hors catalogue) : ses dossiers de parties par profil, sur sa ligne de commande.
    if nom.starts_with("mame") && nom.ends_with(".exe") {
        let profil = noyau.actif().await.ok_or_else(|| Erreur::Profil("Aucun profil n'est ouvert.".into()))?;
        let dossier = chemin.parent().map(std::path::PathBuf::from).unwrap_or_default();
        let (args, notes) = tauri::async_runtime::spawn_blocking(move || crate::emulateurs_profils::preparer_mame_profil(&dossier, &profil.nom))
            .await
            .map_err(|_| Erreur::Disque("La préparation de MAME s'est arrêtée brutalement.".into()))??;
        for n in notes {
            noyau.journaliser(&n);
        }
        let mut complete: Vec<String> = args.iter().map(|a| format!("\"{a}\"")).collect();
        if !ligne.trim().is_empty() {
            complete.push(ligne.to_string());
        }
        return Ok((programme.into(), complete.join(" ")));
    }
    let Some(f) = crate::emulateurs::CATALOGUE.iter().find(|f| nom.starts_with(f.programme)) else {
        return Ok((programme.into(), ligne.into())); // émulateur inconnu : rien à préparer
    };
    let profil = noyau.actif().await.ok_or_else(|| Erreur::Profil("Aucun profil n'est ouvert.".into()))?;
    // Où sont les jeux de ce système : ses emplacements propres, sinon « <défaut><système> ».
    let e = emplacements(app);
    let jeux: Vec<String> = match e.systemes.get(plateforme) {
        Some(l) if !l.is_empty() => l.clone(),
        _ => e
            .defaut
            .iter()
            .map(|d| std::path::Path::new(d).join(crate::jeux_pc::nom_de_dossier(plateforme)).to_string_lossy().into())
            .collect(),
    };
    let dossier = chemin.parent().map(std::path::PathBuf::from).unwrap_or_default();
    // La manette : le choix fait pour ce jeu, sinon la référence du système.
    let magasin = noyau.dossier.join("references");
    let manette = match commandes.mode.as_str() {
        "clavier" => crate::emulateurs_profils::Manette::Clavier,
        "reference" => {
            let (g, n) = (commandes.genre.as_deref().unwrap_or(""), commandes.reference.as_deref().unwrap_or(""));
            let r = crate::references::trouver(&magasin, f.id, g, n)
                .ok_or_else(|| Erreur::Introuvable(format!("Le réglage de manette « {n} » n'existe plus pour {}.", f.nom)))?;
            crate::emulateurs_profils::Manette::Imposee(r)
        }
        _ => crate::emulateurs_profils::Manette::Auto(
            crate::references::par_defaut(plateforme)
                .filter(|(e, ..)| *e == f.id)
                .and_then(|(e, g, n)| crate::references::trouver(&magasin, e, g, n)),
        ),
    };
    // Le compte RetroAchievements DE CE PROFIL (nom ; jeton des émulateurs, au coffre). DuckStation n'en a pas
    // besoin : la personne s'y connecte elle-même, une fois.
    let nom_ra = crate::boutiques::lire_source(&dossier_profil_de(noyau, &profil.id), "retroachievements").map(|c| c.compte).filter(|c| !c.is_empty());
    let jeton_ra = noyau.coffre.lire(&crate::boutiques::nom_secret(&profil.id, "retroachievements-jeton"))?;
    let compte_ra: Option<(String, String)> = match (nom_ra, jeton_ra) {
        (Some(n), Some(j)) => Some((n, j)),
        (Some(n), None) if f.id == "duckstation" => Some((n, String::new())),
        _ => None,
    };
    let (id, n, d, j) = (f.id.to_string(), profil.nom.clone(), dossier.clone(), jeux.clone());
    let avant = tauri::async_runtime::spawn_blocking(move || {
        // Les références sont déposées dans les profils de l'émulateur (on les y retrouve).
        let utilisateur = crate::emulateurs_profils::dossier_du_profil(&d, &n).join("User");
        crate::references::deposer(&crate::references::toutes(&magasin, &id), &d, &utilisateur)?;
        let args = crate::emulateurs_profils::preparer(&id, &d, &n, &j, &manette)?;
        // RPCS3 en plein écran sans interface : aucune boîte (invisible, elle le bloquerait). Un échec n'empêche pas de
        // jouer, mais il est noté (sinon le blocage reviendrait sans indice).
        // Et il se met en pause quand le menu de Frogtend passe devant, sans lire la manette derrière lui.
        let alerte = if id == "rpcs3" {
            let a = crate::emulateurs_profils::rpcs3_sans_boites(&d).err().map(|e| format!("boîtes d'accueil non désactivées ({e:?})"));
            let b = crate::emulateurs_profils::rpcs3_pause_au_menu(&d).err().map(|e| format!("pause au menu non réglée ({e:?})"));
            let v: Vec<String> = a.into_iter().chain(b).collect();
            (!v.is_empty()).then(|| v.join(" ; "))
        } else {
            let mut v: Vec<&str> = Vec::new();
            // Rien n'a pu être réglé (installé hors mode portable, ou déjà ouvert) : noté, sinon le jeu s'ouvre en
            // fenêtre, avec l'utilisateur d'avant, sans indice.
            if id == "vita3k" && (!crate::vita3k::portable(&d) || !crate::lancement::processus_du_dossier(&d).is_empty()) {
                v.push("Vita3K non réglé (pas en mode portable, ou déjà ouvert) : utilisateur et plein écran inchangés");
            }
            // La reprise des parties d'avant Frogtend n'a pas abouti (retentée trois fois au plus).
            if crate::emulateurs_profils::reprise_en_attente(&d) {
                v.push("une reprise des parties d'avant Frogtend n'a pas encore abouti sur cet émulateur");
            }
            (!v.is_empty()).then(|| v.join(" ; "))
        };
        crate::emulateurs_profils::regler_succes(&id, &d, &n, compte_ra.as_ref().map(|(a, b)| (a.as_str(), b.as_str())))?;
        Ok::<_, Erreur>((args, alerte))
    })
        .await
        .map_err(|_| Erreur::Disque("La préparation de l'émulateur s'est arrêtée brutalement.".into()))??;
    let (avant, alerte) = avant;
    if let Some(a) = alerte {
        noyau.journaliser(&format!("RPCS3 : {a}"));
    }
    let mut complete: Vec<String> = avant.iter().map(|a| format!("\"{a}\"")).collect();
    if !ligne.trim().is_empty() {
        complete.push(ligne.to_string());
    }
    Ok((programme.into(), complete.join(" ")))
}

#[derive(Serialize)]
pub struct Traces {
    pub id: String,
    pub nom: String,
    pub dossiers: Vec<String>,
}

/// L'« effet pieuvre » : les dossiers que des émulateurs ont laissés dans le dossier utilisateur de Windows.
#[tauri::command]
pub fn emulateurs_traces() -> Vec<Traces> {
    crate::emulateurs::CATALOGUE
        .iter()
        .filter_map(|f| {
            let d = crate::emulateurs_profils::traces_hors_du_dossier(f.id);
            (!d.is_empty()).then(|| Traces {
                id: f.id.into(),
                nom: f.nom.into(),
                dossiers: d.iter().map(|p| p.to_string_lossy().to_string()).collect(),
            })
        })
        .collect()
}

/// Les programmes des émulateurs réglés sur ce PC (`pc.json` ▸ `emulateurs`).
fn programmes_emulateurs(app: &AppHandle) -> Vec<String> {
    app.store("pc.json")
        .ok()
        .and_then(|s| s.get("reglages"))
        .map(|r| crate::choix_emulateur::tous_les_programmes(&r))
        .unwrap_or_default()
}

/// Sauvegarde le profil ouvert chez Firehouse : configuration, liste des jeux, parties et codes de triche.
#[tauri::command]
pub async fn sauvegarde_lancer(app: AppHandle, noyau: State<'_, Noyau>, ecarter: Option<bool>) -> Resultat<crate::sauvegarde::Bilan> {
    let pc = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "PC".into());
    noyau.sauvegarder(&programmes_emulateurs(&app), &pc, ecarter.unwrap_or(false)).await
}

/// Ce qu'une sauvegarde enverrait (à annoncer avant de l'envoyer).
#[tauri::command]
pub async fn sauvegarde_estimer(app: AppHandle, noyau: State<'_, Noyau>) -> Resultat<crate::sauvegarde::EstimationSauvegarde> {
    noyau.estimer_sauvegarde(&programmes_emulateurs(&app)).await
}

#[tauri::command]
pub async fn sauvegarde_derniere(noyau: State<'_, Noyau>) -> Resultat<Option<crate::sauvegarde::Bilan>> {
    noyau.derniere_sauvegarde().await
}

/// Les sauvegardes du compte de ce jeton, chez Firehouse (pour restaurer).
#[tauri::command]
pub async fn restauration_liste(noyau: State<'_, Noyau>) -> Resultat<Vec<crate::restauration::SauvegardeDisponible>> {
    noyau.sauvegardes_disponibles().await
}

#[derive(Clone, Serialize)]
struct ProgresRestauration {
    faits: usize,
    total: usize,
}

/// Télécharge une sauvegarde dans la zone d'attente du profil ouvert, puis repose les parties des émulateurs
/// présents. Rend la configuration et la bibliothèque, que l'interface applique (avec l'accord de la personne).
#[tauri::command]
pub async fn restauration_preparer(
    app: AppHandle,
    noyau: State<'_, Noyau>,
    profil: String,
    pc: String,
) -> Resultat<(crate::restauration::RestaurationPrete, crate::restauration::Reposes)> {
    let app2 = app.clone();
    let prete = noyau
        .preparer_restauration(&profil, &pc, &move |faits, total| {
            let _ = app2.emit("restauration", ProgresRestauration { faits, total });
        })
        .await?;
    // La personne a choisi de restaurer : la sauvegarde existante est reprise, ce profil peut ensuite l'écrire.
    noyau.marquer_reprise("restauree", &profil, &pc).await?;
    let reposes = noyau.reposer_parties_emulateurs(&programmes_emulateurs(&app)).await?;
    Ok((prete, reposes))
}

/// Repose les parties d'émulateurs restaurées (après avoir installé un émulateur qui manquait).
#[tauri::command]
pub async fn restauration_reposer_emulateurs(app: AppHandle, noyau: State<'_, Noyau>) -> Resultat<crate::restauration::Reposes> {
    noyau.reposer_parties_emulateurs(&programmes_emulateurs(&app)).await
}

/// Remplace la partie en place par sa copie restaurée (la personne l'a demandé ; sa partie actuelle mise à l'abri
/// d'abord). Rend l'abri.
#[tauri::command]
pub async fn restauration_remplacer(noyau: State<'_, Noyau>, cle: String) -> Resultat<String> {
    noyau.remplacer_par_restauree(&cle).await
}

/// Les parties restaurées encore en double (pour les proposer de nouveau).
#[tauri::command]
pub async fn restauration_conflits(noyau: State<'_, Noyau>) -> Resultat<Vec<crate::restauration::ConflitRestaure>> {
    noyau.conflits_restaures().await
}

#[cfg(test)]
mod tests {
    #[test]
    fn un_dossier_de_jeux_n_est_jamais_une_racine_ni_documents() {
        let r = super::refus_dossier_jeux(std::path::Path::new(r"D:\")).unwrap();
        assert!(r.contains(r"D:\Jeux"), "{r}");
        if let Some(u) = std::env::var_os("USERPROFILE") {
            assert!(super::refus_dossier_jeux(&std::path::Path::new(&u).join("Documents")).is_some());
            assert!(super::refus_dossier_jeux(&std::path::Path::new(&u).join("Documents").join("Mes jeux")).is_none());
        }
        assert!(super::refus_dossier_jeux(std::path::Path::new(r"E:\Jeux")).is_none());
        // Les autres dossiers de Frogtend : même règle, leur nom dans le message et le sous-dossier proposé.
        let r = super::refus_dossier(std::path::Path::new(r"D:\"), "emulateurs").unwrap();
        assert!(r.contains("des émulateurs") && r.contains(r"D:\Emulateurs"), "{r}");
        assert!(super::refus_dossier(std::path::Path::new(r"D:\"), "abris").unwrap().contains("abris de parties"));
        assert!(super::refus_dossier(std::path::Path::new(r"D:\Emulateurs"), "emulateurs").is_none());
        assert!(super::refus_dossier_jeux(std::path::Path::new("Jeux")).unwrap().contains("chemin complet"));
        // Sous Windows : jamais « C:\Windows\Jeux » proposé (il serait refusé à son tour).
        if let Some(w) = std::env::var_os("SystemRoot") {
            let r = super::refus_dossier_jeux(std::path::Path::new(&w)).unwrap();
            assert!(!r.to_lowercase().contains(&std::path::Path::new(&w).join("Jeux").to_string_lossy().to_lowercase()), "{r}");
        }
    }

    #[test]
    fn une_seule_installation_a_la_fois_dans_un_dossier() {
        let d = tempfile::tempdir().unwrap();
        let (a, b) = (d.path().join("Emus"), d.path().join("Autres"));
        let premiere = super::occuper_installation(&a, &a.join("RetroArch"), "RetroArch").unwrap();
        // Même dossier (autre écriture du chemin) : refusé tout de suite ; un autre dossier : permis.
        let ecrit_autrement = std::path::PathBuf::from(a.to_string_lossy().to_uppercase() + "\\");
        let e = super::occuper_installation(&ecrit_autrement, &a.join("RetroArch"), "RetroArch").err().unwrap().to_string();
        assert!(e.contains("déjà en cours"), "{e}");
        let autre = super::occuper_installation(&b, &b.join("DuckStation"), "DuckStation").unwrap();
        // Libéré à la fin, même après une erreur (la garde est rendue).
        drop(premiere);
        assert!(super::occuper_installation(&a, &a.join("RetroArch"), "RetroArch").is_ok());
        drop(autre);
    }

    #[test]
    fn retroarch_heberge_lu_seulement_complet() {
        use serde_json::json;
        let p = json!({"heberge": true, "version": "1.22.2", "url": "/api/jeux/v1/emulateurs/retroarch/fichier", "sha256": "ab", "taille": 202509078});
        let h = super::lire_retroarch_heberge(&p).unwrap();
        assert_eq!((h.version.as_str(), h.route.as_str(), h.taille), ("1.22.2", "/emulateurs/retroarch/fichier", Some(202509078)));
        // Pas hébergé, ou sans empreinte, ou une adresse hors de l'API : la source officielle.
        for manque in [json!({"heberge": false}), json!({"heberge": true, "version": "1", "url": "/api/jeux/v1/x", "sha256": ""}), json!({"heberge": true, "version": "1", "url": "https://x/y", "sha256": "ab"})] {
            assert!(super::lire_retroarch_heberge(&manque).is_none(), "{manque}");
        }
    }

    /// Un zip de mod aux entrées données.
    fn zip_de_mod(d: &std::path::Path, entrees: &[&str]) -> std::path::PathBuf {
        use std::io::Write;
        let p = d.join("mod.zip");
        let mut z = zip::ZipWriter::new(std::fs::File::create(&p).unwrap());
        for e in entrees {
            z.start_file(*e, zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(b"x").unwrap();
        }
        z.finish().unwrap();
        p
    }

    #[tokio::test]
    async fn un_mod_range_dans_un_seul_dossier_attend_le_choix_de_la_personne() {
        let d = tempfile::tempdir().unwrap();
        // Rangé dans « MonMod/ » : la question part, la réponse « son contenu directement » revient.
        let a = zip_de_mod(d.path(), &["MonMod/plugin.dll", "MonMod/config/x.ini"]);
        let r = super::choisir_racine(&a, "Mon mod", |q| {
            assert_eq!(q.dossier, "MonMod");
            std::thread::spawn(|| {
                std::thread::sleep(std::time::Duration::from_millis(50));
                super::mod_racine_repondre(Some(true));
            });
        })
        .await
        .unwrap();
        assert!(r, "son contenu directement");
        // Annuler : rien n'est posé (refus).
        let r = super::choisir_racine(&a, "Mon mod", |_| {
            std::thread::spawn(|| super::mod_racine_repondre(None));
        })
        .await;
        assert!(matches!(r, Err(crate::erreurs::Erreur::Refus(_))));
        // Plusieurs choses à la racine : aucune question, posé tel quel.
        let b = d.path().join("b");
        std::fs::create_dir_all(&b).unwrap();
        let a = zip_de_mod(&b, &["plugin.dll", "Data/x.esp"]);
        assert!(!super::choisir_racine(&a, "Autre", |_| panic!("pas de question")).await.unwrap());
    }

    #[test]
    fn taodbox_se_reconnait_dans_la_ligne_de_commande() {
        let a = |l: &[&str]| super::lance_en_taodbox(l.iter().map(|s| s.to_string()));
        assert!(a(&["frogtend.exe", "--taodbox"]));
        assert!(a(&["frogtend.exe", "--TAODBOX"]));
        assert!(!a(&["frogtend.exe"]));
        assert!(!a(&["frogtend.exe", "--taodboxx"]));
    }

    #[test]
    fn le_risque_de_cheat_engine_suit_firehouse_et_le_disque() {
        use crate::figer::Verification;
        use serde_json::json;
        // Relevés réels du 21/10 (`/triche/recherche?steam_appid=`) : GTA V, XCOM 2, Skyrim SE.
        let gta = json!({"online": "true", "anticheat": "BattlEye", "source": "pcgamingwiki"});
        let xcom = json!({"online": "limited", "anticheat": null, "source": "pcgamingwiki"});
        let skyrim = json!({"online": null, "anticheat": null, "source": "pcgamingwiki"});
        let r = super::risque_de(Verification::Aucun, &gta, true);
        assert_eq!(r.anti_triche.as_deref(), Some("BattlEye"), "refusé");
        let r = super::risque_de(Verification::Aucun, &xcom, true);
        assert_eq!((r.anti_triche, r.en_ligne.as_deref(), r.inconnu), (None, Some("limited"), false), "avertir");
        assert!(super::risque_de(Verification::Aucun, &skyrim, true).inconnu, "un jeu de boutique inconnu : avertir");
        assert!(super::risque_de(Verification::Aucun, &serde_json::Value::Null, true).inconnu, "hors ligne : inconnu");
        assert_eq!(super::risque_de(Verification::Aucun, &skyrim, false), super::RisqueTriche::default(), "un jeu de Firehouse sans rien : libre");
        assert_eq!(super::risque_de(Verification::Aucun, &json!({"online": "false"}), false), super::RisqueTriche::default());
        // Le disque a le dernier mot sur un anti-triche, et un dossier trop grand rend inconnu.
        assert_eq!(super::risque_de(Verification::AntiTriche("Easy Anti-Cheat"), &skyrim, false).anti_triche.as_deref(), Some("Easy Anti-Cheat"));
        assert!(super::risque_de(Verification::Inconnu, &skyrim, false).inconnu);
        // Contrat 1.25 : la liste `anticheats` compte aussi (tous dits, sans doublon), même sans `anticheat`.
        let deux = json!({"online": null, "anticheat": "BattlEye", "anticheats": ["BattlEye", "Easy Anti-Cheat"]});
        assert_eq!(super::risque_de(Verification::Aucun, &deux, true).anti_triche.as_deref(), Some("BattlEye, Easy Anti-Cheat"));
        let liste = json!({"online": null, "anticheat": null, "anticheats": ["Easy Anti-Cheat"]});
        let r = super::risque_de(Verification::Aucun, &liste, true);
        assert_eq!((r.anti_triche.as_deref(), r.inconnu), (Some("Easy Anti-Cheat"), false));
        // « Aucun anti-triche » dit par Firehouse : il sait, le jeu n'est pas « inconnu ».
        let r = super::risque_de(Verification::Aucun, &json!({"online": null, "anticheat": "false"}), true);
        assert_eq!((r.anti_triche, r.inconnu), (None, false));
    }

    #[test]
    fn un_mod_est_refuse_avec_un_anti_triche_et_dans_le_doute_pour_un_jeu_de_boutique() {
        use crate::figer::Verification;
        assert!(super::verdict_anti_triche(Verification::AntiTriche("BattlEye"), false).is_err());
        assert!(super::verdict_anti_triche(Verification::Inconnu, true).is_err(), "boutique : dans le doute, rien");
        assert!(super::verdict_anti_triche(Verification::Inconnu, false).is_ok(), "un gros repack de Firehouse : comme avant");
        assert!(super::verdict_anti_triche(Verification::Aucun, true).is_ok());
    }

    #[tokio::test]
    async fn le_coeur_vient_de_firehouse_verifie_et_jamais_d_ailleurs() {
        use httpmock::prelude::*;
        use std::io::Write;
        // Le zip servi : snes9x_libretro.dll à la racine (contrat 1.12).
        let d = tempfile::tempdir().unwrap();
        let mut octets = Vec::new();
        {
            let mut z = zip::ZipWriter::new(std::io::Cursor::new(&mut octets));
            z.start_file("snes9x_libretro.dll", zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(b"MZ snes9x").unwrap();
            z.finish().unwrap();
        }
        let sha = |o: &[u8]| {
            use sha2::Digest;
            format!("{:x}", sha2::Sha256::digest(o))
        };
        let serveur = MockServer::start();
        let liste = serveur.mock(|w, r| {
            w.method(GET).path("/api/jeux/v1/emulateurs/retroarch/coeurs");
            r.status(200).json_body(serde_json::json!({"ok": true, "coeurs": [{
                "coeur": "snes9x_libretro", "source": "stable", "version": "1.22.2", "version_retroarch": "1.22.2",
                "build_le": null, "sha256_zip": sha(&octets), "sha256_dll": sha(b"MZ snes9x"), "taille": octets.len(),
                "recupere_le": "2026-10-05T03:38:18", "url": "/api/jeux/v1/emulateurs/retroarch/coeurs/snes9x_libretro/fichier"}]}));
        });
        let fichier = serveur.mock(|w, r| {
            w.method(GET).path("/api/jeux/v1/emulateurs/retroarch/coeurs/snes9x_libretro/fichier");
            r.status(200).body(octets.clone());
        });
        let source = crate::source::Source::Firehouse(crate::firehouse::Client::nouveau(&serveur.base_url(), "jeton").unwrap());
        let ra = d.path().join("RetroArch");
        let p = super::installer_coeur_servi(&source, &ra, "snes9x_libretro.dll").await.unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"MZ snes9x");
        assert!(!ra.join(".telechargements").exists(), "le paquet est retiré après l'installation");
        // Un cœur que Firehouse ne sert pas : refusé, rien téléchargé ailleurs.
        assert!(matches!(super::installer_coeur_servi(&source, &ra, "bsnes_libretro.dll").await, Err(crate::erreurs::Erreur::Introuvable(_))));
        assert!(!ra.join("cores").join("bsnes_libretro.dll").exists());
        liste.assert_hits(2);
        fichier.assert_hits(1);
    }

    #[test]
    fn les_bios_ne_vont_qu_a_l_emulateur_du_conseil() {
        assert!(super::programme_du_conseil("retroarch", r"E:\Emulateurs\RetroArch\retroarch.exe"));
        assert!(super::programme_du_conseil("pcsx2", r"E:\Emulateurs\PCSX2\pcsx2-qt.exe"));
        assert!(super::programme_du_conseil("rpcs3", r"E:\Emulateurs\RPCS3\rpcs3.exe"));
        assert!(!super::programme_du_conseil("pcsx2", r"E:\Emulateurs\DuckStation\duckstation-qt-x64-ReleaseLTCG.exe"));
        assert!(!super::programme_du_conseil("retroarch", r"E:\Emulateurs\PCSX2\pcsx2-qt.exe"));
        assert!(!super::programme_du_conseil("retroarch", r"E:\Emulateurs\DuckStation\duckstation-qt-x64-ReleaseLTCG.exe"));
        assert!(!super::programme_du_conseil("", r"E:\x.exe"));
    }

    /// RÉEL, sur une COPIE de brouillon de RPCS3 (`FROGTEND_RPCS3_BROUILLON=<dossier avec rpcs3.exe>`) : le micrologiciel
    /// servi est téléchargé (≈ 206 Mo), vérifié, installé une première fois, puis une deuxième PAR-DESSUS.
    /// `FROGTEND_PROFIL_ESSAI=<id> FROGTEND_RPCS3_BROUILLON=<dossier> cargo test essai_micrologiciel_installe -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_micrologiciel_installe() {
        use crate::coffre::Coffre as _;
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").unwrap();
        let rpcs3 = std::path::PathBuf::from(std::env::var("FROGTEND_RPCS3_BROUILLON").unwrap());
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().unwrap();
        let s = crate::source::Source::Firehouse(crate::firehouse::Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap());
        let v = s.emulateurs(super::PLATEFORME_PS3).await.unwrap();
        let bios = crate::bios::lire_bios(&v["conseil"]["bios"], &mut Default::default());
        let f = crate::bios::firmware_servi(&bios).unwrap().clone();
        let pup = crate::bios::telecharger_firmware(&s, &f, &rpcs3, &|_, _| {}).await.unwrap();
        let exe = rpcs3.join("rpcs3.exe").to_string_lossy().to_string();
        let debut = std::time::Instant::now();
        let attendue = crate::bios::version_dans_note(&f.note);
        println!("1RE INSTALLATION : {:?} en {:?}", super::installer_pup(exe.clone(), pup.to_string_lossy().into(), attendue.clone()).await, debut.elapsed());
        std::thread::sleep(std::time::Duration::from_secs(2));
        println!("2E INSTALLATION (par-dessus) : {:?}", super::installer_pup(exe, pup.to_string_lossy().into(), attendue).await);
        let _ = std::fs::remove_file(&pup);
    }

    #[tokio::test]
    async fn un_part_d_une_autre_version_ne_bloque_jamais_le_telechargement() {
        use httpmock::prelude::*;
        let serveur = MockServer::start();
        // La reprise est refusée (416 : le `.part` est plus long que le nouveau fichier) ; sans reprise : le fichier.
        let refus = serveur.mock(|w, r| {
            w.method(GET).path("/api/jeux/v1/f").header_exists("range");
            r.status(416);
        });
        let entier = serveur.mock(|w, r| {
            w.method(GET)
                .path("/api/jeux/v1/f")
                .matches(|r| !r.headers.as_ref().is_some_and(|h| h.iter().any(|(k, _)| k.eq_ignore_ascii_case("range"))));
            r.status(200).body(b"nouveau".to_vec());
        });
        let d = tempfile::tempdir().unwrap();
        let cible = d.path().join("x.zip");
        std::fs::write(cible.with_extension("part"), b"ancienne version plus longue").unwrap();
        let source = crate::source::Source::Firehouse(crate::firehouse::Client::nouveau(&serveur.base_url(), "jeton").unwrap());
        assert_eq!(source.telecharger_route("/f", &cible, &|_, _| {}).await.unwrap(), 7);
        assert_eq!(std::fs::read(&cible).unwrap(), b"nouveau");
        refus.assert_hits(1);
        entier.assert_hits(1);
    }
}

// --- Rapprocher les jeux importés des fiches de Firehouse (contrat 1.21, besoin 28 ; ✅ Seb 20/10) : annoncé et
// chiffré d'abord, lecture seule des fichiers, seuls les « exact » concordants se rattachent seuls (une
// correspondance à côté : le jeu importé ne change jamais), le reste est montré. ---

/// Les jeux importés que le profil ouvert voit (jamais ceux d'un autre profil : la réponse dépend du jeton).
/// Identifie les jeux importés de ce profil qui n'ont pas de fiche (contrat 1.32). `tout` : aussi ceux déjà
/// identifiés (relancé par la personne) ; sinon seulement les nouveaux.
#[tauri::command]
pub async fn identification_lancer(noyau: State<'_, Noyau>, tout: Option<bool>) -> Resultat<crate::identification::Bilan> {
    let jeux = {
        let s = noyau.session().await?;
        let deja = s.verrou().identifies()?;
        let mut l = Vec::new();
        for j in imports_de(&noyau, &s)? {
            // Déjà relié à une fiche visible : rien à identifier. Un choix de la personne n'est jamais renvoyé.
            let choisi = s.verrou().identification(j.id)?.is_some_and(|(st, ..)| st == "choisi");
            if noyau.fiche_de(&s, j.id)?.is_some() || choisi || (!tout.unwrap_or(false) && deja.contains(&j.id)) {
                continue;
            }
            l.push(j);
        }
        l
    };
    noyau.identifier(&jeux).await
}

/// Ce qu'une identification enverrait, à annoncer AVANT : (jeux à envoyer, déjà cherchés, lots de 500).
#[tauri::command]
pub async fn identification_annonce(noyau: State<'_, Noyau>) -> Resultat<(usize, usize, usize)> {
    let s = noyau.session().await?;
    let deja = s.verrou().identifies()?;
    let (mut nouveaux, mut cherches) = (0, 0);
    for j in imports_de(&noyau, &s)? {
        // Relié à une fiche visible, ou rien d'envoyable (aucun nom de fichier) : jamais compté.
        if noyau.fiche_de(&s, j.id)?.is_some() || crate::identification::demande("x", &j).is_none() {
            continue;
        }
        if deja.contains(&j.id) {
            cherches += 1;
        } else {
            nouveaux += 1;
        }
    }
    Ok((nouveaux, cherches, nouveaux.div_ceil(crate::identification::LOT)))
}

/// Les jeux importés à faire choisir à la personne (identification « probable »).
#[tauri::command]
pub async fn identification_a_choisir(noyau: State<'_, Noyau>) -> Resultat<Vec<crate::identification::AChoisir>> {
    noyau.a_choisir(&imports_du_profil(&noyau).await?).await
}

/// La personne choisit, parmi les candidats proposés, le jeu qu'est ce jeu importé.
#[tauri::command]
pub async fn identification_choisir(noyau: State<'_, Noyau>, jeu: i64, launchbox_id: i64) -> Resultat<()> {
    let j = imports_du_profil(&noyau).await?.into_iter().find(|x| x.id == jeu).ok_or_else(|| Erreur::Introuvable("Ce jeu importé n'est plus sur ce PC.".into()))?;
    noyau.choisir_identification(&j, launchbox_id).await
}

/// « Aucun de ceux-là » : plus proposé.
#[tauri::command]
pub async fn identification_ecarter(noyau: State<'_, Noyau>, jeu: i64) -> Resultat<()> {
    noyau.ecarter_identification(jeu).await
}

/// La fiche de base (résumé, genres, images…) d'un jeu importé identifié, ou rien.
#[tauri::command]
pub async fn jeu_fiche_base(noyau: State<'_, Noyau>, jeu: i64) -> Resultat<Option<serde_json::Value>> {
    noyau.fiche_base(jeu).await
}

async fn imports_du_profil(noyau: &Noyau) -> Resultat<Vec<crate::jeux_pc::JeuPc>> {
    let s = noyau.session().await?;
    imports_de(noyau, &s)
}

/// Les jeux importés que voit le profil de CETTE session.
fn imports_de(noyau: &Noyau, s: &crate::noyau::Session) -> Resultat<Vec<crate::jeux_pc::JeuPc>> {
    let tous = noyau.registre().tous()?;
    let c = s.verrou();
    let mut l = Vec::new();
    for j in tous {
        if crate::import_local::est_importe(&j) && c.contient(j.id)? {
            l.push(j);
        }
    }
    Ok(l)
}

fn cache_empreintes(noyau: &Noyau) -> Resultat<crate::empreintes::Cache> {
    crate::empreintes::Cache::ouvrir(&noyau.dossier.join("empreintes.db"))
}

/// Ce que coûtera le rapprochement (avant tout : la personne voit et dit oui).
#[tauri::command]
pub async fn rapprocher_annonce(noyau: State<'_, Noyau>) -> Resultat<crate::reconciliation::Annonce> {
    let jeux = imports_du_profil(&noyau).await?;
    let cache = cache_empreintes(&noyau)?;
    tauri::async_runtime::spawn_blocking(move || {
        let p = crate::reconciliation::inventaire(&jeux);
        crate::reconciliation::annoncer(&p, &cache, crate::reconciliation::PLAFOND_SHA1)
    })
    .await
    .map_err(|_| Erreur::Disque("Le relevé s'est arrêté brutalement.".into()))
}

/// Le compte rendu d'un rapprochement (gardé dans le dossier du PROFIL : il dépend de son jeton).
#[derive(Clone, Serialize, serde::Deserialize, Default)]
pub struct Rapprochement {
    pub le: String,
    pub jeux: Vec<crate::reconciliation::JeuRapproche>,
    pub non_envoyes: usize,
}

#[derive(Clone, Serialize)]
struct ProgresRapprochement {
    etape: &'static str,
    faits: usize,
    total: usize,
}

/// Relève les empreintes (lecture seule ; `reseau` : la personne permet de lire en entier sur un disque réseau),
/// interroge Firehouse, rattache les « exact » concordants et garde le compte rendu.
#[tauri::command]
pub async fn rapprocher(app: AppHandle, noyau: State<'_, Noyau>, reseau: bool) -> Resultat<Rapprochement> {
    // Une seule reconnaissance à la fois (un second clic, ou la page rouverte pendant la première).
    static EN_COURS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if EN_COURS.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return Err(Erreur::Refus("Une reconnaissance est déjà en cours : attends sa fin.".into()));
    }
    struct Fin;
    impl Drop for Fin {
        fn drop(&mut self) {
            EN_COURS.store(false, std::sync::atomic::Ordering::SeqCst);
        }
    }
    let _fin = Fin;
    // UNE session du début à la fin : si la personne change de profil pendant la lecture, rien ne part avec le jeton
    // d'un autre profil, et rien n'est écrit (contrat : la réponse dépend du jeton).
    let s = noyau.session().await?;
    let p = s.profil.clone();
    let jeux = imports_de(&noyau, &s)?;
    let cache = cache_empreintes(&noyau)?;
    let (j2, a2) = (jeux.clone(), app.clone());
    let prets = tauri::async_runtime::spawn_blocking(move || {
        let mut dernier = std::time::Instant::now();
        crate::reconciliation::calculer(crate::reconciliation::inventaire(&j2), &cache, crate::reconciliation::PLAFOND_SHA1, reseau, &mut |faits, total| {
            if dernier.elapsed().as_millis() >= 400 || faits == total {
                dernier = std::time::Instant::now();
                let _ = a2.emit("rapprochement", ProgresRapprochement { etape: "lecture", faits, total });
            }
        })
    })
    .await
    .map_err(|_| Erreur::Disque("Le relevé s'est arrêté brutalement.".into()))?;
    let _ = app.emit("rapprochement", ProgresRapprochement { etape: "firehouse", faits: 0, total: prets.len() });
    let change = || Erreur::Refus("Le profil a changé pendant la reconnaissance : rien n'est enregistré. Relance-la (ce qui est déjà lu ne sera pas relu).".into());
    if s.est_perimee() {
        return Err(change());
    }
    let bilan = crate::reconciliation::reconcilier(&s.source, prets.iter().map(|x| x.fichier.clone()).collect(), crate::reconciliation::Limites::default())
        .await
        .map_err(|e| match e {
            Erreur::Reseau(m) => Erreur::Reseau(format!("{m} Relance : tes fichiers déjà lus ne seront pas relus.")),
            e => e,
        })?;
    if s.est_perimee() {
        return Err(change());
    }
    let jeux_rapproches = crate::reconciliation::par_jeu(&prets, &bilan, &jeux);
    let quand = crate::noyau::maintenant();
    // Les « exact » concordants se rattachent seuls ; un rattachement confirmé par la personne n'est jamais remplacé.
    let confirmes: std::collections::BTreeSet<i64> = noyau.registre().correspondances()?.into_iter().filter(|(_, _, c)| c == "personne").map(|(l, _, _)| l).collect();
    let ecartes = noyau.registre().ecartes()?;
    for j in jeux_rapproches.iter().filter(|j| j.decision == "rattache" && !confirmes.contains(&j.jeu) && !j.media_id.is_some_and(|m| ecartes.contains(&(j.jeu, m)))) {
        let v: Vec<_> = j.versions.iter().filter_map(|v| Some((v.version.clone(), v.media_id?, v.telechargement_id, v.statut.clone(), v.correspondance.clone()))).collect();
        noyau.registre().noter_correspondance(j.jeu, &v, "auto", &quand)?;
    }
    let r = Rapprochement { le: quand, jeux: jeux_rapproches, non_envoyes: bilan.non_envoyes.len() };
    let f = dossier_profil_de(&noyau, &p.id).join("rapprochement.json");
    std::fs::write(&f, serde_json::to_vec(&r).unwrap_or_default())?;
    let r = rapprochement_dernier(noyau.clone()).await?.unwrap_or(r);
    noyau.journaliser(&format!(
        "rapprochement : {} jeu(x) importé(s), {} rattaché(s), {} à voir (profil {})",
        r.jeux.len(),
        r.jeux.iter().filter(|j| j.decision == "rattache").count(),
        r.jeux.iter().filter(|j| j.decision == "a_voir").count(),
        p.id
    ));
    Ok(r)
}

/// Le dernier compte rendu de ce profil.
#[tauri::command]
pub async fn rapprochement_dernier(noyau: State<'_, Noyau>) -> Resultat<Option<Rapprochement>> {
    let p = profil_ouvert(&noyau).await?;
    let f = dossier_profil_de(&noyau, &p.id).join("rapprochement.json");
    let Some(mut r) = std::fs::read(f).ok().and_then(|o| serde_json::from_slice::<Rapprochement>(&o).ok()) else { return Ok(None) };
    // Toujours d'après ce qui est noté MAINTENANT sur ce PC (rattaché, défait, écarté), avec les titres des fiches.
    let (correspondances, ecartes) = (noyau.registre().correspondances()?, noyau.registre().ecartes()?);
    let s = noyau.session().await?;
    let c = s.verrou();
    // Une fiche réunie à une autre (contrat 1.14) : le titre de la fiche finale.
    let titre_de = |id: i64| c.jeu(c.reuni_vers(id).ok().flatten().unwrap_or(id)).ok().flatten().map(|j| (j.titre, j.annee));
    crate::reconciliation::actualiser(&mut r.jeux, &correspondances, &ecartes, &titre_de);
    Ok(Some(r))
}

/// La personne revient sur « ce n'est pas lui » : les fiches écartées de ce jeu sont de nouveau proposées.
#[tauri::command]
pub async fn rapprochement_reprendre(noyau: State<'_, Noyau>, jeu: i64) -> Resultat<()> {
    if !imports_du_profil(&noyau).await?.iter().any(|x| x.id == jeu) {
        return Err(Erreur::Introuvable("Ce jeu importé n'est pas visible pour ce profil.".into()));
    }
    noyau.registre().reprendre_ecartes(jeu)
}

/// La personne dit que ce jeu importé N'EST PAS cette fiche : elle ne lui sera plus proposée.
#[tauri::command]
pub async fn rapprochement_ecarter(noyau: State<'_, Noyau>, jeu: i64, media_id: i64) -> Resultat<()> {
    if !imports_du_profil(&noyau).await?.iter().any(|x| x.id == jeu) {
        return Err(Erreur::Introuvable("Ce jeu importé n'est pas visible pour ce profil.".into()));
    }
    noyau.registre().ecarter(jeu, media_id, &crate::noyau::maintenant())
}

/// La personne rattache elle-même un jeu « à voir » à la fiche qu'elle choisit (parmi celles proposées par Firehouse
/// pour ce jeu dans le dernier compte rendu : jamais une fiche venue d'ailleurs).
#[tauri::command]
pub async fn rapprochement_rattacher(noyau: State<'_, Noyau>, jeu: i64, media_id: i64) -> Resultat<()> {
    let r = rapprochement_dernier(noyau.clone()).await?.ok_or_else(|| Erreur::Refus("Lance d'abord un rapprochement.".into()))?;
    let j = r.jeux.iter().find(|x| x.jeu == jeu).ok_or_else(|| Erreur::Introuvable("Ce jeu n'est pas dans le dernier rapprochement.".into()))?;
    let propose = |v: &crate::reconciliation::VersionRapprochee| v.media_id == Some(media_id) || v.candidats.iter().any(|c| c.media_id == media_id);
    if !j.versions.iter().any(propose) {
        return Err(Erreur::Refus("Firehouse n'a pas proposé cette fiche pour ce jeu.".into()));
    }
    if !imports_du_profil(&noyau).await?.iter().any(|x| x.id == jeu) {
        return Err(Erreur::Introuvable("Ce jeu importé n'est plus sur ce PC.".into()));
    }
    let v: Vec<_> = j
        .versions
        .iter()
        .filter(|v| propose(v))
        .map(|v| (v.version.clone(), media_id, v.candidats.iter().find(|c| c.media_id == media_id).and_then(|c| c.telechargement_id).or(v.telechargement_id.filter(|_| v.media_id == Some(media_id))), v.statut.clone(), v.correspondance.clone()))
        .collect();
    noyau.registre().noter_correspondance(jeu, &v, "personne", &crate::noyau::maintenant())?;
    noyau.journaliser(&format!("jeu importé {jeu} rattaché à la fiche {media_id} par la personne"));
    Ok(())
}

/// Défait le rattachement d'un jeu importé (le jeu lui-même ne change pas).
#[tauri::command]
pub async fn rapprochement_defaire(noyau: State<'_, Noyau>, jeu: i64) -> Resultat<()> {
    if !imports_du_profil(&noyau).await?.iter().any(|x| x.id == jeu) {
        return Err(Erreur::Introuvable("Ce jeu importé n'est pas visible pour ce profil.".into()));
    }
    noyau.registre().oublier_correspondance(jeu)
}
