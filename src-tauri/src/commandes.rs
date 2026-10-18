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
    if !dossier.trim().is_empty() {
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
    noyau
        .synchroniser(|page, jeux| {
            let _ = app.emit("synchro", ProgresSynchro { page, jeux });
        })
        .await
}

#[tauri::command]
pub async fn ludotheque_synchronisee_le(noyau: State<'_, Noyau>) -> Resultat<Option<String>> {
    noyau.synchronise_le().await
}

#[tauri::command]
pub async fn ludotheque_plateformes(noyau: State<'_, Noyau>, locale: Option<bool>) -> Resultat<Vec<Plateforme>> {
    noyau.plateformes(locale.unwrap_or(false)).await
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
pub async fn livret_ouvrir(noyau: State<'_, Noyau>, id: i64, doc: i64) -> Resultat<u32> {
    noyau.livret(id, doc).await
}

/// Prépare une musique ou une vidéo du jeu (la télécharge d'abord si besoin : le clic de la personne, taille
/// montrée, vaut son accord) ; elle se lit ensuite par `document.localhost/<jeu>/<document>/fichier`.
#[tauri::command]
pub async fn document_preparer(noyau: State<'_, Noyau>, id: i64, doc: i64) -> Resultat<crate::documents::Document> {
    noyau.preparer_document(id, doc).await
}

/// La place libre dans un dossier (`None` : introuvable), pour l'écran des emplacements.
#[tauri::command]
pub fn espace_libre(chemin: String) -> Option<u64> {
    crate::jeux_pc::place_libre(std::path::Path::new(&chemin))
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
}

/// Ce qu'il faut pour les contenus PS3 d'un jeu : le jeu, son RPCS3, le compte RPCS3 du profil, les contenus trouvés.
async fn contexte_contenus(
    app: &AppHandle,
    noyau: &Noyau,
    id: i64,
) -> Resultat<Option<(std::path::PathBuf, Option<String>, Option<String>, Vec<crate::contenus::Contenu>)>> {
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
    Ok(Some((rpcs3, compte, titre_id, contenus)))
}

/// Switch : les mises à jour et DLC d'un jeu trouvés sur ce PC, et l'Eden qui les lira.
async fn contexte_switch(app: &AppHandle, noyau: &Noyau, id: i64) -> Resultat<Option<(std::path::PathBuf, Vec<crate::contenus::ContenuSwitch>)>> {
    let j = noyau.jeu_visible(id).await?;
    if !j.plateforme.eq_ignore_ascii_case("Nintendo Switch") {
        return Ok(None);
    }
    let Some((programme, _)) = emulateur_regle(app, &j.plateforme, id, None) else { return Err(Erreur::Reglage("Aucun Eden réglé pour la Switch.".into())) };
    let eden = std::path::Path::new(&programme).parent().map(std::path::PathBuf::from).unwrap_or_default();
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
            contenus: titres
                .into_iter()
                .filter(|t| t.genre != "jeu")
                .map(|t| ContenuVu {
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
                contenus: l
                    .into_iter()
                    .map(|c| {
                        let d = parent(&c.chemin);
                        ContenuVu { installe: lus.iter().any(|x| crate::contenus::meme_dossier(x, &d)), licence: false, id: c.id, nom: c.nom, genre: c.genre, taille: c.taille, dossier: Some(d) }
                    })
                    .collect(),
            });
        }
        Ok(None) => {}
        Err(Erreur::Reglage(_)) => return Ok(ContenusDuJeu { etat: "emulateur".into(), systeme: "switch".into(), ..Default::default() }),
        Err(e) => return Err(e),
    }
    let (rpcs3, compte, titre_id, contenus) = match contexte_contenus(&app, &noyau, id).await {
        Ok(Some(x)) => x,
        Ok(None) => return Ok(ContenusDuJeu { etat: "aucun".into(), ..Default::default() }),
        Err(Erreur::Reglage(_)) => return Ok(ContenusDuJeu { etat: "emulateur".into(), systeme: "ps3".into(), ..Default::default() }),
        Err(e) => return Err(e),
    };
    let faits = crate::contenus::installes(&rpcs3);
    Ok(ContenusDuJeu {
        etat: "ok".into(),
        systeme: "ps3".into(),
        titre_id,
        contenus: contenus
            .into_iter()
            .map(|c| ContenuVu {
                installe: faits.contains(&c.id) && (c.rap.is_none() || compte.as_deref().is_some_and(|k| crate::contenus::licence_posee(&rpcs3, k, &c.id))),
                licence: c.rap.is_some(),
                id: c.id,
                nom: c.nom,
                genre: c.genre,
                taille: c.taille,
                dossier: None,
            })
            .collect(),
    })
}

/// Le bilan d'une installation de contenus.
#[derive(Serialize, Default)]
pub struct BilanContenus {
    pub installes: usize,
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
    let total = crate::decompression::examiner(&archive)?.map(|a| a.taille).unwrap_or(0);
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
        let ini = crate::contenus::ini_eden(&eden);
        let apres = crate::contenus::ajouter_dossiers_eden(&std::fs::read_to_string(&ini).unwrap_or_default(), &dossiers);
        crate::emulateurs_profils::remplacer_config(&eden, &ini, &apres)?;
        // Vérifier le résultat : les dossiers sont bien dans la liste relue.
        let relus = crate::contenus::dossiers_externes_eden(&std::fs::read_to_string(&ini).unwrap_or_default());
        let manquants: Vec<(String, String)> =
            dossiers.iter().filter(|d| !relus.iter().any(|r| crate::contenus::meme_dossier(r, d))).map(|d| (d.clone(), "pas enregistré dans Eden".to_string())).collect();
        noyau.journaliser(&format!("contenus Switch : {} dossier(s) ajouté(s) à Eden", dossiers.len() - manquants.len()));
        return Ok(BilanContenus { installes: l.iter().filter(|c| choisis.contains(&c.id)).count() - manquants.len().min(choisis.len()), refuses: manquants });
    }
    let (rpcs3, _, _, contenus) =
        contexte_contenus(&app, &noyau, id).await?.ok_or_else(|| Erreur::Refus("Ce jeu n'a pas de contenus gérés par Frogtend.".into()))?;
    // La personne a dit oui : le compte RPCS3 de son profil est créé s'il manque (même nom qu'au lancement).
    let p = profil_ouvert(&noyau).await?;
    let compte = crate::emulateurs_profils::compte_rpcs3(&rpcs3, &crate::jeux_pc::nom_de_dossier(&p.nom))?;
    let programme = emulateur_regle(&app, &noyau.registre().jeu(id)?.map(|j| j.plateforme).unwrap_or_default(), id, None)
        .map(|(p, _)| std::path::PathBuf::from(p))
        .ok_or_else(|| Erreur::Reglage("Aucun RPCS3 réglé pour la PS3.".into()))?;
    let travail = noyau.dossier.join("travail").join("contenus");
    let a_faire: Vec<crate::contenus::Contenu> = contenus.into_iter().filter(|c| choisis.contains(&c.id)).collect();
    noyau.journaliser(&format!("contenus PS3 : installation de {} contenu(s) pour le compte {compte}", a_faire.len()));
    let bilan = tauri::async_runtime::spawn_blocking(move || {
        let mut b = BilanContenus::default();
        for c in &a_faire {
            let r = crate::contenus::installer_ps3(c, &rpcs3, &compte, &travail, &|pkg| {
                let args = vec!["--headless".to_string(), "--installpkg".to_string(), pkg.to_string_lossy().to_string()];
                let code = crate::installation::executer_et_attendre(&programme, &args, &rpcs3)?;
                if code != 0 {
                    return Err(Erreur::Disque(format!("RPCS3 a répondu {code} en installant le paquet.")));
                }
                Ok(())
            });
            match r {
                Ok(()) => b.installes += 1,
                Err(e) => b.refuses.push((c.nom.clone(), format!("{e:?}"))),
            }
        }
        let _ = std::fs::remove_dir(&travail); // seulement s'il est vide
        b
    })
    .await
    .map_err(|_| Erreur::Disque("L'installation s'est arrêtée brutalement.".into()))?;
    noyau.journaliser(&format!("contenus PS3 : {} installé(s), {} refusé(s)", bilan.installes, bilan.refuses.len()));
    Ok(bilan)
}

#[tauri::command]
pub async fn installation_preparer(noyau: State<'_, Noyau>, id: i64) -> Resultat<crate::partie::Preparation> {
    noyau.preparer_installation(id).await
}

/// Installe le jeu (la personne a donné son accord dans l'interface).
#[tauri::command]
pub async fn installation_lancer(
    noyau: State<'_, Noyau>,
    id: i64,
    automatique: bool,
) -> Resultat<crate::jeux_pc::Installation> {
    noyau.installer(id, automatique).await
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
) -> Resultat<()> {
    let jeu = noyau.registre().jeu(id)?;
    let (plateforme, titre) = jeu.map(|j| (j.plateforme, j.titre)).unwrap_or_default();
    let mut id_emulateur = None;
    let mut dossier_emulateur = None;
    let emulateur = match emulateur_regle(&app, &plateforme, id, emulateur.as_deref()) {
        Some((programme, ligne)) => {
            dossier_emulateur = std::path::Path::new(&programme).parent().map(|d| d.to_string_lossy().to_string());
            let nom = std::path::Path::new(&programme).file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
            id_emulateur = crate::emulateurs::CATALOGUE.iter().find(|f| nom.starts_with(f.programme)).map(|f| f.id.to_string());
            // MAME vient du paquet de Firehouse, pas du catalogue : reconnu par son programme.
            if id_emulateur.is_none() && nom.starts_with("mame") && nom.ends_with(".exe") {
                id_emulateur = Some("mame".into());
            }
            Some(preparer_emulateur(&app, &noyau, &plateforme, &programme, &ligne, &commandes.unwrap_or_default()).await?)
        }
        None => None,
    };
    let par_emulateur = emulateur.is_some();
    // Un jeu resté figé d'une partie précédente repart d'abord.
    crate::figer::degeler();
    // « Occupé » du lancement jusqu'à la fin du suivi de la partie (deux tâches) : jamais rattaché entre les deux.
    noyau.marquer_occupe(id);
    let (pid, dossiers, carte_cedee) = match noyau.jouer(id, emulateur, version.as_deref()).await {
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
    let numero = app.state::<crate::menu_jeu::MenuJeu>().commencer(crate::menu_jeu::PartieEnCours {
        jeu: id,
        titre,
        plateforme,
        pid,
        emulateur: id_emulateur,
        dossier_emulateur,
        figer,
        // Manette → clavier : un jeu PC lancé sans émulateur seulement (un émulateur a ses propres manettes).
        clavier: clavier.filter(|c| !c.is_empty() && dossier_jeu.is_some()),
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
    let mut clavier = partie
        .and_then(|p| p.clavier)
        .map(|c| crate::clavier::Moteur::nouveau(&c, combo))
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
pub fn menu_jeu_etat(app: AppHandle, menu: State<'_, crate::menu_jeu::MenuJeu>) -> Option<crate::menu_jeu::EtatMenu> {
    let taodbox = app.get_webview_window("main").and_then(|m| m.is_fullscreen().ok()).unwrap_or(false);
    menu.partie().map(|p| {
        let e = crate::menu_jeu::etat(&p);
        // Un jeu PC n'est « en pause » que s'il est VRAIMENT figé (pas seulement parce que l'option est cochée).
        let en_pause = e.en_pause || ((p.figer || p.figer_emulateur) && crate::figer::est_fige());
        crate::menu_jeu::EtatMenu { taodbox, en_pause, ..e }
    })
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
pub async fn menu_jeu_action(app: AppHandle, action: String) -> Resultat<()> {
    let a = crate::menu_jeu::action_de(&action).ok_or_else(|| Erreur::Refus(format!("Action inconnue : {action}.")))?;
    let p = app
        .state::<crate::menu_jeu::MenuJeu>()
        .partie()
        .ok_or_else(|| Erreur::Introuvable("Aucune partie en cours.".into()))?;
    let e = p.emulateur.clone().ok_or_else(|| Erreur::Refus("Ce jeu ne se pilote pas depuis le menu.".into()))?;
    cacher_menu(&app);
    tauri::async_runtime::spawn_blocking(move || crate::menu_jeu::agir(&e, &crate::lancement::processus_de_la_partie(p.pid), a))
        .await
        .map_err(|_| Erreur::Disque("L'action s'est arrêtée brutalement.".into()))?
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
    let n = tauri::async_runtime::spawn_blocking(move || {
        crate::menu_jeu::quitter(p.emulateur.as_deref(), &crate::lancement::processus_de_la_partie(racine))
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
    noyau.retirer_du_pc(id).await
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
    if !base.is_dir() {
        return Err(Erreur::Disque(format!("Dossier introuvable : {dossier}.")));
    }
    let p = noyau.session().await?.source.paquet_emulateur(&id).await?;
    let url = p["url"].as_str().unwrap_or("").to_string();
    let version = p["version"].as_str().unwrap_or("").to_string();
    let nom_sur = crate::jeux_pc::nom_de_dossier(&nom);
    let cible = base.join(&nom_sur);
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
}

#[tauri::command]
pub async fn succes_retro(noyau: State<'_, Noyau>, id: i64) -> Resultat<SuccesRetro> {
    let p = profil_ouvert(&noyau).await?;
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
        r.jeu = crate::succes::progression(crate::succes::API_RA, &cle, &compte, id_ra).await.ok();
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
        let installes = crate::boutiques::installes_steam(&d);
        for j in &mut l {
            j.installe = installes.contains(&j.id);
        }
    }
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

/// 📥 Importer ▸ MAME Arcade Full Set : le tri du dossier d'après la liste MAME de LaunchBox (rien n'est encore ajouté).
#[tauri::command]
pub async fn import_chercher_mame(dossier: String, liste: String, options: crate::mame::OptionsMame) -> Resultat<crate::mame::TriMame> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::mame::trier_avec_fichier(std::path::Path::new(&dossier), std::path::Path::new(&liste), &options)
    })
    .await
    .map_err(|_| Erreur::Disque("Le tri s'est arrêté brutalement.".into()))?
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
pub async fn import_copier(noyau: State<'_, Noyau>, elements: Vec<String>, destination: String) -> Resultat<Vec<String>> {
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
pub async fn import_ajouter(noyau: State<'_, Noyau>, jeux: Vec<crate::import_local::JeuAImporter>) -> Resultat<crate::import_local::BilanImport> {
    noyau.importer_locaux(&jeux).await
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
    Ok(crate::boutiques::lire_source(&dossier_profil_de(&noyau, &p.id), "galaxy").map(|c| c.jeux).unwrap_or_default())
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
) -> Resultat<()> {
    let p = std::path::PathBuf::from(&programme);
    let nom = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
    if !p.is_file() || !nom.starts_with("cheatengine") && !nom.starts_with("cheat engine") {
        return Err(Erreur::Refus("Ce n'est pas le programme de Cheat Engine.".into()));
    }
    exiger_programme_de_confiance(&app, &noyau, &p)?;
    let dossier = p.parent().map(std::path::PathBuf::from).unwrap_or_default();
    let profil = noyau.actif().await.ok_or_else(|| Erreur::Profil("Aucun profil n'est ouvert.".into()))?.nom;
    let (d, pr) = (dossier.clone(), profil.clone());
    tauri::async_runtime::spawn_blocking(move || crate::cheatengine::preparer(crate::cheatengine::CLE, &d, &pr))
        .await
        .map_err(|_| Erreur::Disque("La préparation s'est arrêtée brutalement.".into()))??;
    let table = table.filter(|t| t.to_lowercase().ends_with(".ct") && std::path::Path::new(t).is_file());
    let mut c = std::process::Command::new(&p);
    c.current_dir(&dossier);
    if brancher.unwrap_or(false) {
        // « Lancer avec Cheat Engine » : notre script autorun se branche seul sur le jeu en cours et charge la table.
        let partie = app
            .state::<crate::menu_jeu::MenuJeu>()
            .partie()
            .ok_or_else(|| Erreur::Refus("Lance d'abord le jeu : Cheat Engine se branchera dessus.".into()))?;
        let pid = tauri::async_runtime::spawn_blocking(move || crate::menu_jeu::processus_du_jeu(partie.pid))
            .await
            .map_err(|_| Erreur::Disque("La recherche du jeu s'est arrêtée brutalement.".into()))?;
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

/// Les triches et mods connus d'un jeu (contrat 13).
#[tauri::command]
pub async fn jeu_triches(noyau: State<'_, Noyau>, id: i64) -> Resultat<Value> {
    noyau.session().await?.source.triches(id).await
}

/// Pose un fichier de triche de Firehouse dans le dossier du profil de l'émulateur (la personne l'a demandé). Un
/// fichier différent déjà là est d'abord copié à côté (`.avant-frogtend`). Rend le chemin écrit.
#[tauri::command]
pub async fn triche_installer(noyau: State<'_, Noyau>, id: i64, cle: String, programme: String, ligne: String) -> Resultat<String> {
    let s = noyau.session().await?;
    let liste = s.source.triches(id).await?;
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
    let octets = s.source.fichier_triche(id, &cle).await?;
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
    let source = std::path::PathBuf::from(&i.dossier);
    let copie = crate::locale::dossier_frogtend(&j).join("copies").join(format!("avant-mod-{}", crate::noyau::maintenant()));
    let (s, c) = (source.clone(), copie.clone());
    let frogtend = crate::locale::dossier_frogtend(&j);
    tauri::async_runtime::spawn_blocking(move || -> Resultat<()> {
        // La liste est faite AVANT de copier, et laisse de côté le dossier Frogtend (médias, copies précédentes).
        let fichiers: Vec<String> =
            crate::installation::fichiers_de(&s)?.into_iter().filter(|r| !s.join(r).starts_with(&frogtend)).collect();
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
    .map_err(|_| Erreur::Disque("La copie s'est arrêtée brutalement.".into()))??;
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
pub async fn emulateur_derniere_version(id: String) -> Resultat<crate::emulateurs::Paquet> {
    let f = crate::emulateurs::fiche(&id).ok_or_else(|| Erreur::Introuvable("Émulateur inconnu.".into()))?;
    crate::emulateurs::derniere_version(f).await
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
    if !base.is_dir() {
        return Err(Erreur::Disque(format!("Dossier introuvable : {dossier}.")));
    }
    let paquet_info = crate::emulateurs::derniere_version(f).await?;
    let cible = base.join(f.nom);
    let paquet = base.join(".telechargements").join(format!("{}.paquet", f.id));
    let (app2, id2) = (app.clone(), id.clone());
    crate::emulateurs::telecharger(&paquet_info.url, &paquet, &move |recus, total| {
        let _ = app2.emit("emulateur", ProgresEmulateur { id: id2.clone(), recus, total });
    })
    .await?;
    let (p, c) = (paquet.clone(), cible.clone());
    let programme = tauri::async_runtime::spawn_blocking(move || crate::emulateurs::installer_paquet(f, &p, &c))
        .await
        .map_err(|_| Erreur::Disque("L'installation s'est arrêtée brutalement.".into()))??;
    let _ = std::fs::remove_file(&paquet);
    let _ = std::fs::remove_dir(base.join(".telechargements"));
    // RetroArch : ses profils de manette officiels, s'ils ne sont pas dans le paquet (annoncés avec l'installation).
    if f.id == "retroarch" && !crate::manettes::retroarch_a_ses_profils(&cible) {
        installer_profils_retroarch(&cible).await?;
    }
    let e = crate::emulateurs::EmulateurInstalle {
        id: f.id.into(),
        nom: f.nom.into(),
        version: Some(paquet_info.version),
        dossier: cible.to_string_lossy().into(),
        programme: programme.to_string_lossy().into(),
        par_frogtend: true,
        installe_le: crate::noyau::maintenant(),
    };
    registre_emulateurs(&noyau).retenir(e.clone())?;
    Ok(e)
}

/// Retient un émulateur installé à la main (trouvé, ou montré par la personne).
#[tauri::command]
pub fn emulateur_adopter(
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
pub async fn bios_poser(noyau: State<'_, Noyau>, plateforme: String, programme: String) -> Resultat<crate::bios::Bilan> {
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
pub async fn retroarch_installer_coeur(noyau: State<'_, Noyau>, programme: String, coeur: String) -> Resultat<String> {
    if !coeur.ends_with("_libretro.dll") || !crate::emulateurs::nom_de_coeur_sur(&coeur) {
        return Err(Erreur::Refus("Nom de cœur invalide.".into()));
    }
    let dossier = std::path::Path::new(&programme).parent().map(std::path::PathBuf::from).unwrap_or_default();
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

/// Télécharge et ajoute les profils de manette officiels de RetroArch. Rend le nombre de profils ajoutés.
async fn installer_profils_retroarch(retroarch: &std::path::Path) -> Resultat<usize> {
    let paquet = retroarch.join("autoconfig.zip.telechargement");
    crate::emulateurs::telecharger(crate::manettes::URL_PROFILS_RETROARCH, &paquet, &|_, _| {}).await?;
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
pub async fn emulateur_regler_manette(noyau: State<'_, Noyau>, id: String, programme: String) -> Resultat<ManetteReglee> {
    crate::emulateurs::fiche(&id).ok_or_else(|| Erreur::Introuvable("Émulateur inconnu.".into()))?;
    let dossier = std::path::Path::new(&programme).parent().map(std::path::PathBuf::from).unwrap_or_default();
    if !dossier.is_dir() {
        return Err(Erreur::Disque(format!("Dossier introuvable : {}.", dossier.display())));
    }
    let profil = noyau.actif().await.ok_or_else(|| Erreur::Profil("Aucun profil n'est ouvert.".into()))?;
    let mut profils_ajoutes = 0;
    if id == "retroarch" && !crate::manettes::retroarch_a_ses_profils(&dossier) {
        profils_ajoutes = installer_profils_retroarch(&dossier).await?;
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
            None
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
pub async fn sauvegarde_lancer(app: AppHandle, noyau: State<'_, Noyau>) -> Resultat<crate::sauvegarde::Bilan> {
    let pc = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "PC".into());
    noyau.sauvegarder(&programmes_emulateurs(&app), &pc).await
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
    let reposes = noyau.reposer_parties_emulateurs(&programmes_emulateurs(&app)).await?;
    Ok((prete, reposes))
}

/// Repose les parties d'émulateurs restaurées (après avoir installé un émulateur qui manquait).
#[tauri::command]
pub async fn restauration_reposer_emulateurs(app: AppHandle, noyau: State<'_, Noyau>) -> Resultat<crate::restauration::Reposes> {
    noyau.reposer_parties_emulateurs(&programmes_emulateurs(&app)).await
}

#[cfg(test)]
mod tests {
    #[test]
    fn taodbox_se_reconnait_dans_la_ligne_de_commande() {
        let a = |l: &[&str]| super::lance_en_taodbox(l.iter().map(|s| s.to_string()));
        assert!(a(&["frogtend.exe", "--taodbox"]));
        assert!(a(&["frogtend.exe", "--TAODBOX"]));
        assert!(!a(&["frogtend.exe"]));
        assert!(!a(&["frogtend.exe", "--taodboxx"]));
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
