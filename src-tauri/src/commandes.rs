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
pub fn abris_regler(noyau: State<'_, Noyau>, dossier: String) -> Resultat<usize> {
    let d = std::path::PathBuf::from(dossier.trim());
    if !dossier.trim().is_empty() {
        std::fs::create_dir_all(&d)?;
    }
    noyau.regler_abris(Some(d));
    noyau.migrer_abris()
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
    noyau.changer_jeton(&jeton, &c).await?;
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
pub async fn profil_supprimer(noyau: State<'_, Noyau>, id: String, pin: Option<String>) -> Resultat<()> {
    noyau.supprimer_profil(&id, pin.as_deref()).await
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
) -> Resultat<Option<(std::path::PathBuf, String, Option<String>, Vec<crate::contenus::Contenu>)>> {
    let p = profil_ouvert(noyau).await?;
    let j = noyau.registre().jeu(id)?.ok_or_else(|| Erreur::Introuvable("Ce jeu n'est pas sur ce PC.".into()))?;
    if crate::succes::console_de(&j.plateforme) != Some(82) {
        return Ok(None);
    }
    let Some((programme, _)) = emulateur_regle(app, &j.plateforme, id, None) else { return Err(Erreur::Reglage("Aucun RPCS3 réglé pour la PS3.".into())) };
    let rpcs3 = std::path::Path::new(&programme).parent().map(std::path::PathBuf::from).unwrap_or_default();
    let compte = crate::emulateurs_profils::compte_rpcs3(&rpcs3, &p.nom)?;
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
    let j = noyau.registre().jeu(id)?.ok_or_else(|| Erreur::Introuvable("Ce jeu n'est pas sur ce PC.".into()))?;
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
    let jeu = noyau.registre().jeu(id)?;
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
                        ContenuVu { installe: lus.iter().any(|x| x.eq_ignore_ascii_case(&d)), licence: false, id: c.id, nom: c.nom, genre: c.genre, taille: c.taille, dossier: Some(d) }
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
                installe: faits.contains(&c.id) && (c.rap.is_none() || crate::contenus::licence_posee(&rpcs3, &compte, &c.id)),
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
            dossiers.iter().filter(|d| !relus.iter().any(|r| r.eq_ignore_ascii_case(d))).map(|d| (d.clone(), "pas enregistré dans Eden".to_string())).collect();
        noyau.journaliser(&format!("contenus Switch : {} dossier(s) ajouté(s) à Eden", dossiers.len() - manquants.len()));
        return Ok(BilanContenus { installes: l.iter().filter(|c| choisis.contains(&c.id)).count() - manquants.len().min(choisis.len()), refuses: manquants });
    }
    let (rpcs3, compte, _, contenus) =
        contexte_contenus(&app, &noyau, id).await?.ok_or_else(|| Erreur::Refus("Ce jeu n'a pas de contenus gérés par Frogtend.".into()))?;
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
    noyau: State<'_, Noyau>,
    id: i64,
    dossier: String,
) -> Resultat<crate::jeux_pc::Installation> {
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
pub async fn jeu_jouer(
    app: AppHandle,
    noyau: State<'_, Noyau>,
    id: i64,
    commandes: Option<Commandes>,
    emulateur: Option<String>,
    version: Option<String>,
) -> Resultat<()> {
    let jeu = noyau.registre().jeu(id)?;
    let (plateforme, titre) = jeu.map(|j| (j.plateforme, j.titre)).unwrap_or_default();
    let mut id_emulateur = None;
    let emulateur = match emulateur_regle(&app, &plateforme, id, emulateur.as_deref()) {
        Some((programme, ligne)) => {
            let nom = std::path::Path::new(&programme).file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
            id_emulateur = crate::emulateurs::CATALOGUE.iter().find(|f| nom.starts_with(f.programme)).map(|f| f.id.to_string());
            Some(preparer_emulateur(&app, &noyau, &plateforme, &programme, &ligne, &commandes.unwrap_or_default()).await?)
        }
        None => None,
    };
    let (pid, dossiers, carte_cedee) = noyau.jouer(id, emulateur, version.as_deref()).await?;
    let _ = app.emit("partie", EvenementPartie::Debut { jeu: id, carte_cedee });
    // Le menu en jeu : sa touche est active pendant la partie seulement.
    app.state::<crate::menu_jeu::MenuJeu>().commencer(crate::menu_jeu::PartieEnCours {
        jeu: id,
        titre,
        plateforme,
        pid,
        emulateur: id_emulateur,
    });
    armer_touche_menu(&app, true);
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        let noyau = app2.state::<Noyau>();
        let fin = noyau.suivre_partie(id, pid, dossiers).await;
        app2.state::<crate::menu_jeu::MenuJeu>().finir();
        armer_touche_menu(&app2, false);
        cacher_menu(&app2);
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

/// Ouvre (ou montre) la fenêtre du menu en jeu par-dessus le jeu, et lui donne le premier plan.
pub fn ouvrir_menu(app: &AppHandle) {
    if app.state::<crate::menu_jeu::MenuJeu>().partie().is_none() {
        return;
    }
    let w = match app.get_webview_window(FENETRE_MENU) {
        Some(w) => w,
        None => match tauri::WebviewWindowBuilder::new(app, FENETRE_MENU, tauri::WebviewUrl::App("menu-jeu".into()))
            .title("Frogtend — menu")
            .inner_size(560.0, 640.0)
            .center()
            .resizable(false)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .build()
        {
            Ok(w) => w,
            Err(e) => {
                app.state::<Noyau>().journaliser(&format!("menu en jeu : {e}"));
                return;
            }
        },
    };
    let _ = w.show();
    let _ = w.unminimize();
    let _ = w.set_focus();
    let _ = app.emit_to(FENETRE_MENU, "menu-jeu", ());
}

fn cacher_menu(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(FENETRE_MENU) {
        let _ = w.hide();
    }
}

/// Ce que le menu en jeu montre (`null` hors partie).
#[tauri::command]
pub fn menu_jeu_etat(menu: State<'_, crate::menu_jeu::MenuJeu>) -> Option<crate::menu_jeu::EtatMenu> {
    menu.partie().map(|p| crate::menu_jeu::etat(&p))
}

/// Reprendre : le menu se cache, le jeu reprend le premier plan (et sort seul de sa pause).
#[tauri::command]
pub async fn menu_jeu_reprendre(app: AppHandle) -> Resultat<()> {
    cacher_menu(&app);
    let Some(p) = app.state::<crate::menu_jeu::MenuJeu>().partie() else { return Ok(()) };
    tauri::async_runtime::spawn_blocking(move || crate::menu_jeu::reprendre(&crate::lancement::processus_de_la_partie(p.pid)))
        .await
        .map_err(|_| Erreur::Disque("La reprise s'est arrêtée brutalement.".into()))?;
    Ok(())
}

/// Une action du menu (`reset`, `disque`, `sauver`, `charger`) : le jeu reprend la main et reçoit l'ordre.
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
    tauri::async_runtime::spawn_blocking(move || {
        crate::menu_jeu::quitter(p.emulateur.as_deref(), &crate::lancement::processus_de_la_partie(p.pid))
    })
    .await
    .map_err(|_| Erreur::Disque("La fermeture s'est arrêtée brutalement.".into()))?
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

#[tauri::command]
pub async fn ra_oublier(noyau: State<'_, Noyau>) -> Resultat<()> {
    let p = profil_ouvert(&noyau).await?;
    noyau.coffre.oublier(&crate::boutiques::nom_secret(&p.id, "retroachievements"))?;
    noyau.coffre.oublier(&crate::boutiques::nom_secret(&p.id, "retroachievements-jeton"))?;
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
    let j = noyau.registre().jeu(id)?.ok_or_else(|| Erreur::Introuvable("Ce jeu n'est pas sur ce PC.".into()))?;
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

/// 📥 Importer ▸ Fichiers ROM : les ROM d'un dossier (rien n'est encore ajouté : la personne voit la liste d'abord).
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
pub async fn import_installer_dos(noyau: State<'_, Noyau>, dosbox: String, source: String, destination: String, titre: String) -> Resultat<Vec<String>> {
    let (dosbox, source, destination) = (std::path::PathBuf::from(dosbox), std::path::PathBuf::from(source), std::path::PathBuf::from(destination));
    if !dosbox.is_file() {
        return Err(Erreur::Reglage("DOSBox introuvable : règle l'émulateur de MS-DOS dans ⚙ Options ▸ Émulateurs.".into()));
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
        .map_err(|_| Erreur::Disque("La lecture de GOG Galaxy s'est arrêtée brutalement.".into()))??;
    // La copie de la base ne reste pas : elle contient les données du compte.
    let _ = std::fs::remove_dir_all(&travail);
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

/// Installe le micrologiciel PS3 dans RPCS3 (`--installfw`, rpcs3.cpp) depuis le fichier PS3UPDAT.PUP que la
/// personne a téléchargé sur le site de Sony et choisi. RPCS3 montre sa propre progression.
#[tauri::command]
pub fn rpcs3_installer_micrologiciel(programme: String, pup: String) -> Resultat<()> {
    let p = std::path::Path::new(&programme);
    let nom = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
    if nom != "rpcs3.exe" || !p.is_file() {
        return Err(Erreur::Refus("Ce n'est pas le programme de RPCS3.".into()));
    }
    if !pup.to_lowercase().ends_with(".pup") || !std::path::Path::new(&pup).is_file() {
        return Err(Erreur::Refus("Choisis le fichier PS3UPDAT.PUP.".into()));
    }
    std::process::Command::new(p)
        .arg("--installfw")
        .arg(&pup)
        .current_dir(p.parent().unwrap_or(p))
        .spawn()
        .map_err(|e| Erreur::Disque(format!("RPCS3 ne démarre pas ({e}).")))?;
    Ok(())
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

/// Installe un cœur RetroArch depuis le buildbot officiel de libretro.
#[tauri::command]
pub async fn retroarch_installer_coeur(programme: String, coeur: String) -> Resultat<String> {
    if coeur.contains(['/', '\\']) || !coeur.ends_with("_libretro.dll") {
        return Err(Erreur::Refus("Nom de cœur invalide.".into()));
    }
    let dossier = std::path::Path::new(&programme).parent().map(std::path::PathBuf::from).unwrap_or_default();
    let paquet = dossier.join(format!("{coeur}.zip.telechargement"));
    crate::emulateurs::telecharger(&crate::emulateurs::url_coeur(&coeur), &paquet, &|_, _| {}).await?;
    let r = crate::emulateurs::installer_coeur(&paquet, &dossier, &coeur);
    let _ = std::fs::remove_file(&paquet);
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
        crate::emulateurs_profils::regler_succes(&id, &d, &n, compte_ra.as_ref().map(|(a, b)| (a.as_str(), b.as_str())))?;
        Ok::<_, Erreur>(args)
    })
        .await
        .map_err(|_| Erreur::Disque("La préparation de l'émulateur s'est arrêtée brutalement.".into()))??;
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
}
