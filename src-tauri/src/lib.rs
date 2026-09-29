//! Cœur de Frogtend : les commandes appelées par l'interface.
//!
//! Règle : les secrets (jeton Firehouse, identifiants des boutiques) restent de ce côté-ci.
//! L'interface ne les reçoit jamais.

pub mod coffre;
pub mod commandes;
pub mod erreurs;
pub mod firehouse;
pub mod ludotheque;
pub mod noyau;
pub mod profils;
pub mod source;

use serde::Serialize;
use tauri::Manager;

/// Ce que l'écran « À propos » affiche sur l'application.
#[derive(Debug, Serialize, PartialEq)]
pub struct InfosApplication {
    pub nom: String,
    pub version: String,
    pub identifiant: String,
}

/// Construit les informations de l'application à partir de sa configuration.
pub fn infos_depuis(config: &tauri::Config) -> InfosApplication {
    InfosApplication {
        nom: config.product_name.clone().unwrap_or_else(|| "Frogtend".into()),
        version: config.version.clone().unwrap_or_else(|| env!("CARGO_PKG_VERSION").into()),
        identifiant: config.identifier.clone(),
    }
}

#[tauri::command]
fn infos_application(app: tauri::AppHandle) -> InfosApplication {
    infos_depuis(app.config())
}

/// Répond aux adresses `jaquette://localhost/<id>` (sous Windows : `http://jaquette.localhost/<id>`) avec la
/// jaquette du profil OUVERT. Un autre profil, ou aucun, n'obtient rien.
fn repondre_jaquette(app: &tauri::AppHandle, chemin: &str) -> tauri::http::Response<Vec<u8>> {
    use tauri::http::Response;
    let vide = |code: u16| Response::builder().status(code).body(Vec::new()).unwrap();
    let Ok(id) = chemin.trim_matches('/').parse::<i64>() else { return vide(400) };
    let noyau = app.state::<noyau::Noyau>();
    match tauri::async_runtime::block_on(noyau.jaquette(id)) {
        Ok(Some(img)) => Response::builder()
            .status(200)
            .header("Content-Type", img.type_contenu.unwrap_or_else(|| "application/octet-stream".into()))
            .header("Cache-Control", "no-store")
            .body(img.octets)
            .unwrap(),
        _ => vide(404),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let dossier = app.path().app_data_dir()?;
            let n = noyau::Noyau::nouveau(&dossier, Box::new(coffre::CoffreWindows))
                .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;
            app.manage(n);
            Ok(())
        })
        .register_asynchronous_uri_scheme_protocol("jaquette", |ctx, requete, repondeur| {
            let app = ctx.app_handle().clone();
            let chemin = requete.uri().path().to_string();
            // Hors du fil de l'interface : la jaquette peut venir du réseau.
            std::thread::spawn(move || repondeur.respond(repondre_jaquette(&app, &chemin)));
        })
        .invoke_handler(tauri::generate_handler![
            infos_application,
            commandes::profils_lister,
            commandes::profil_creer,
            commandes::profil_ouvrir,
            commandes::profil_fermer,
            commandes::profil_actif,
            commandes::profil_changer_jeton,
            commandes::profil_reconnecter,
            commandes::profil_changer_pin,
            commandes::profil_renommer,
            commandes::profil_supprimer,
            commandes::ludotheque_synchroniser,
            commandes::ludotheque_synchronisee_le,
            commandes::ludotheque_plateformes,
            commandes::ludotheque_genres,
            commandes::ludotheque_lister,
            commandes::ludotheque_au_hasard,
            commandes::ludotheque_fiche,
            commandes::ludotheque_annexe_texte,
            commandes::skins_obtenir,
            commandes::skin_personnel,
        ])
        .run(tauri::generate_context!())
        .expect("impossible de démarrer Frogtend");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La configuration réelle de l'application, lue comme Tauri la lit.
    fn config_reelle() -> tauri::Config {
        let texte = include_str!("../tauri.conf.json");
        serde_json::from_str(texte).expect("tauri.conf.json illisible")
    }

    #[test]
    fn l_identifiant_est_celui_decide_par_seb() {
        assert_eq!(config_reelle().identifier, "fr.hikari-no-sekai.frogtend");
    }

    #[test]
    fn les_infos_reprennent_le_nom_du_produit() {
        let infos = infos_depuis(&config_reelle());
        assert_eq!(infos.nom, "Frogtend");
        assert_eq!(infos.identifiant, "fr.hikari-no-sekai.frogtend");
    }

    #[test]
    fn l_installateur_est_par_utilisateur_et_en_francais() {
        let brut: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let nsis = &brut["bundle"]["windows"]["nsis"];
        assert_eq!(nsis["installMode"], "currentUser");
        assert_eq!(nsis["languages"][0], "French");
        assert_eq!(brut["bundle"]["createUpdaterArtifacts"], true);
    }

    #[test]
    fn les_mises_a_jour_viennent_des_releases_github_du_depot() {
        let brut: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let adresse = brut["plugins"]["updater"]["endpoints"][0].as_str().unwrap();
        assert!(adresse.starts_with("https://github.com/Nimai26/Frogtend/releases/"));
        assert!(!brut["plugins"]["updater"]["pubkey"].as_str().unwrap().is_empty());
    }
}
