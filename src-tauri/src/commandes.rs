//! Les commandes que l'interface appelle (`invoke`). Minces : le travail est fait par le noyau.
//!
//! Aucune ne renvoie un jeton. Les réglages de connexion sont relus dans `pc.json` à chaque ouverture de profil :
//! l'interface ne peut pas envoyer le jeton ailleurs qu'à l'adresse enregistrée.

use crate::erreurs::{Erreur, Resultat};
use crate::ludotheque::{Filtre, JeuResume, Liste, Plateforme};
use crate::noyau::{BilanSynchro, Connexion, FicheLue, Noyau};
use crate::profils::ProfilVisible;
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_store::StoreExt;

const ADRESSE_PAR_DEFAUT: &str = "https://jeux.hikari-no-sekai.fr";

/// Les réglages de connexion de ce PC, tels que l'interface les a enregistrés.
fn connexion(app: &AppHandle) -> Connexion {
    let reglages = app.store("pc.json").ok().and_then(|s| s.get("reglages")).unwrap_or(Value::Null);
    let f = &reglages["firehouse"];
    Connexion {
        adresse: f["adresse"].as_str().unwrap_or(ADRESSE_PAR_DEFAUT).to_string(),
        simule: f["simule"].as_bool().unwrap_or(false),
    }
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

#[tauri::command]
pub fn profil_creer(
    noyau: State<'_, Noyau>,
    nom: String,
    pin: Option<String>,
    jeton: Option<String>,
) -> Resultat<ProfilVisible> {
    noyau.creer_profil(&nom, pin.as_deref(), jeton.as_deref())
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

#[tauri::command]
pub async fn profil_changer_jeton(app: AppHandle, noyau: State<'_, Noyau>, jeton: String) -> Resultat<()> {
    noyau.changer_jeton(&jeton, &connexion(&app)).await
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
pub async fn ludotheque_plateformes(noyau: State<'_, Noyau>) -> Resultat<Vec<Plateforme>> {
    noyau.plateformes().await
}

#[tauri::command]
pub async fn ludotheque_genres(noyau: State<'_, Noyau>, plateforme: Option<String>) -> Resultat<Vec<String>> {
    noyau.genres(plateforme.as_deref()).await
}

#[tauri::command]
pub async fn ludotheque_lister(noyau: State<'_, Noyau>, filtre: Filtre) -> Resultat<Liste> {
    noyau.lister(&filtre).await
}

#[tauri::command]
pub async fn ludotheque_au_hasard(noyau: State<'_, Noyau>, plateforme: Option<String>) -> Resultat<Option<JeuResume>> {
    noyau.au_hasard(plateforme.as_deref()).await
}

#[tauri::command]
pub async fn ludotheque_fiche(noyau: State<'_, Noyau>, id: i64) -> Resultat<FicheLue> {
    noyau.fiche(id).await
}

#[tauri::command]
pub async fn ludotheque_annexe_texte(noyau: State<'_, Noyau>, id: i64, i: u32, cle: String) -> Resultat<Value> {
    noyau.annexe_texte(id, i, &cle).await
}

#[tauri::command]
pub async fn skins_obtenir(noyau: State<'_, Noyau>) -> Resultat<Option<Value>> {
    noyau.skins().await
}

#[tauri::command]
pub async fn skin_personnel(noyau: State<'_, Noyau>) -> Resultat<Option<Value>> {
    noyau.skin_personnel().await
}
