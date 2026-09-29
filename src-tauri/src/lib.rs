//! Cœur de Frogtend : les commandes appelées par l'interface.
//!
//! Règle : les secrets (jeton Firehouse, identifiants des boutiques) restent de ce côté-ci.
//! L'interface ne les reçoit jamais.

use serde::Serialize;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![infos_application])
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
