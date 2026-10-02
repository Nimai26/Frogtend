//! Cœur de Frogtend : les commandes appelées par l'interface.
//!
//! Règle : les secrets (jeton Firehouse, identifiants des boutiques) restent de ce côté-ci.
//! L'interface ne les reçoit jamais.

pub mod boutiques;
pub mod cheatengine;
pub mod choix_emulateur;
pub mod coffre;
pub mod emulateurs;
pub mod emulateurs_profils;
pub mod commandes;
pub mod erreurs;
pub mod firehouse;
pub mod installation;
pub mod jeux_pc;
pub mod lancement;
pub mod manettes;
pub mod locale;
pub mod gratuits;
pub mod import_local;
pub mod mame;
pub mod ludotheque;
pub mod menu_jeu;
pub mod noyau;
pub mod partie;
pub mod pilotage;
pub mod profils;
pub mod references;
pub mod restauration;
pub mod sauvegarde;
pub mod source;
pub mod telechargements;

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

fn reponse_vide(code: u16) -> tauri::http::Response<Vec<u8>> {
    tauri::http::Response::builder().status(code).body(Vec::new()).unwrap()
}

/// La valeur d'un paramètre d'une adresse (`largeur=400`).
fn parametre(requete: Option<&str>, nom: &str) -> Option<String> {
    requete?.split('&').find_map(|p| p.strip_prefix(&format!("{nom}=")).map(String::from))
}

/// Répond aux adresses `jaquette://localhost/<id>?largeur=N` (sous Windows : `http://jaquette.localhost/…`) avec
/// la jaquette du profil OUVERT, en miniature si `largeur` est donnée. Un autre profil, ou aucun, n'obtient rien.
fn repondre_jaquette(app: &tauri::AppHandle, chemin: &str, requete: Option<&str>) -> tauri::http::Response<Vec<u8>> {
    use tauri::http::Response;
    let Ok(id) = chemin.trim_matches('/').parse::<i64>() else { return reponse_vide(400) };
    let largeur = parametre(requete, "largeur").and_then(|l| l.parse::<u32>().ok());
    let noyau = app.state::<noyau::Noyau>();
    let resultat = tauri::async_runtime::block_on(noyau.jaquette(id, largeur));
    if let Err(e) = &resultat {
        noyau.journaliser(&format!("jaquette {id} (largeur {largeur:?}) : {e:?}"));
    }
    match resultat {
        Ok(Some(img)) => Response::builder()
            .status(200)
            .header("Content-Type", img.type_contenu.unwrap_or_else(|| "application/octet-stream".into()))
            .header("Cache-Control", "no-store")
            .body(img.octets)
            .unwrap(),
        _ => reponse_vide(404),
    }
}

/// Répond aux adresses `fond://localhost/<skin>` avec la vidéo de fond de ce skin (gardée pour ce PC).
fn repondre_fond(app: &tauri::AppHandle, chemin: &str) -> tauri::http::Response<Vec<u8>> {
    let nom = chemin.trim_matches('/');
    let noyau = app.state::<noyau::Noyau>();
    match tauri::async_runtime::block_on(noyau.video_skin(&commandes::connexion(app), nom)) {
        Ok(Some(octets)) => tauri::http::Response::builder()
            .status(200)
            .header("Content-Type", "video/webm")
            .body(octets)
            .unwrap(),
        _ => reponse_vide(404),
    }
}

/// `boutique://steam/<appid>` : la jaquette officielle d'un jeu Steam, en cache (images publiques de Steam).
fn repondre_boutique(app: &tauri::AppHandle, chemin: &str) -> tauri::http::Response<Vec<u8>> {
    let cache = app.state::<noyau::Noyau>().dossier.join("images-boutiques");
    let mut morceaux = chemin.trim_matches('/').splitn(2, '/');
    let image = match (morceaux.next(), morceaux.next()) {
        (Some("steam"), Some(appid)) => tauri::async_runtime::block_on(boutiques::image_steam(&cache, appid)),
        // « image/<adresse encodée> » : une image publique d'un hôte autorisé (GOG, Steam).
        (Some("image"), Some(adresse)) => {
            let url = percent_decode(adresse);
            tauri::async_runtime::block_on(boutiques::image_par_adresse(&cache, &url))
        }
        _ => None,
    };
    match image {
        Some(octets) => tauri::http::Response::builder()
            .status(200)
            .header("Content-Type", type_d_image(&octets))
            .header("Cache-Control", "max-age=604800")
            .body(octets)
            .unwrap(),
        None => reponse_vide(404),
    }
}

/// Le type d'une image d'après ses premiers octets (les boutiques servent du JPEG, du PNG ou du WebP).
fn type_d_image(o: &[u8]) -> &'static str {
    if o.starts_with(b"\x89PNG") {
        "image/png"
    } else if o.len() > 12 && &o[..4] == b"RIFF" && &o[8..12] == b"WEBP" {
        "image/webp"
    } else {
        "image/jpeg"
    }
}

/// Décode « %2F » et compagnie (adresse passée dans le chemin du protocole).
fn percent_decode(s: &str) -> String {
    let o = s.as_bytes();
    let mut v = Vec::with_capacity(o.len());
    let mut i = 0;
    while i < o.len() {
        if o[i] == b'%' && i + 2 < o.len() {
            if let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                v.push(b);
                i += 3;
                continue;
            }
        }
        v.push(o[i]);
        i += 1;
    }
    String::from_utf8_lossy(&v).into_owned()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let dossier = app.path().app_data_dir()?;
            let n = noyau::Noyau::nouveau(&dossier, Box::new(coffre::CoffreWindows))
                .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;
            app.manage(n);
            app.manage(menu_jeu::MenuJeu::default());
            // La touche du menu en jeu (armée seulement pendant une partie) ouvre le menu par-dessus le jeu.
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(|app, _raccourci, evenement| {
                        if evenement.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                            commandes::ouvrir_menu(app);
                        }
                    })
                    .build(),
            )?;
            Ok(())
        })
        .register_asynchronous_uri_scheme_protocol("jaquette", |ctx, requete, repondeur| {
            let app = ctx.app_handle().clone();
            let chemin = requete.uri().path().to_string();
            let parametres = requete.uri().query().map(String::from);
            // Hors du fil de l'interface : la jaquette peut venir du réseau.
            std::thread::spawn(move || {
                // Quoi qu'il arrive, l'interface reçoit une réponse (sinon l'image attendrait pour rien).
                let reponse = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    repondre_jaquette(&app, &chemin, parametres.as_deref())
                }))
                .unwrap_or_else(|_| {
                    app.state::<noyau::Noyau>().journaliser(&format!("jaquette {chemin} : échec imprévu"));
                    reponse_vide(500)
                });
                repondeur.respond(reponse)
            });
        })
        .register_asynchronous_uri_scheme_protocol("fond", |ctx, requete, repondeur| {
            let app = ctx.app_handle().clone();
            let chemin = requete.uri().path().to_string();
            std::thread::spawn(move || repondeur.respond(repondre_fond(&app, &chemin)));
        })
        .register_asynchronous_uri_scheme_protocol("boutique", |ctx, requete, repondeur| {
            let app = ctx.app_handle().clone();
            let chemin = requete.uri().path().to_string();
            std::thread::spawn(move || repondeur.respond(repondre_boutique(&app, &chemin)));
        })
        .invoke_handler(tauri::generate_handler![
            infos_application,
            commandes::profils_lister,
            commandes::profil_creer,
            commandes::profil_ouvrir,
            commandes::profil_fermer,
            commandes::profil_actif,
            commandes::profil_changer_jeton,
            commandes::profil_compte,
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
            commandes::skin_enregistrer,
            commandes::jeux_du_pc,
            commandes::emplacements_proposer,
            commandes::jeu_ajouter,
            commandes::telechargement_pause,
            commandes::telechargement_reprendre,
            commandes::telechargement_annuler,
            commandes::annexe_ouvrir,
            commandes::espace_libre,
            commandes::installation_preparer,
            commandes::installation_lancer,
            commandes::installation_ailleurs,
            commandes::lancement_candidats,
            commandes::lanceur_choisir,
            commandes::jeu_jouer,
            commandes::parties_abri,
            commandes::jeu_retirer,
            commandes::emulateurs_recommandes,
            commandes::emulateurs_installes,
            commandes::emulateur_fiche,
            commandes::emulateur_derniere_version,
            commandes::emulateur_installer,
            commandes::emulateur_adopter,
            commandes::retroarch_etat,
            commandes::retroarch_installer_coeur,
            commandes::emulateur_regler_manette,
            commandes::rpcs3_installer_micrologiciel,
            commandes::taodbox_lance,
            commandes::emulateur_installer_firehouse,
            commandes::jeu_triches,
            commandes::cheatengine_lancer,
            commandes::gratuits_liste,
            commandes::gratuits_connexion_epic,
            commandes::gratuits_obtenir_epic,
            commandes::gratuits_connexion_playstation,
            commandes::gratuits_psplus,
            commandes::import_chercher_roms,
            commandes::import_chercher_dos,
            commandes::import_chercher_mame,
            commandes::import_ajouter,
            commandes::boutique_galaxy_etat,
            commandes::boutique_galaxy_jeux,
            commandes::boutique_galaxy_importer,
            commandes::boutique_galaxy_ouvrir,
            commandes::boutique_steam_etat,
            commandes::boutique_steam_regler,
            commandes::boutique_steam_oublier,
            commandes::boutique_steam_jeux,
            commandes::boutique_steam_importer,
            commandes::boutique_steam_ouvrir,
            commandes::triche_installer,
            commandes::jeu_taille_installation,
            commandes::jeu_copie_avant_mod,
            commandes::assistant_demander,
            commandes::assistant_journal,
            commandes::jeu_rechercher,
            commandes::jeu_demander,
            commandes::menu_jeu_etat,
            commandes::menu_jeu_reprendre,
            commandes::menu_jeu_action,
            commandes::menu_jeu_quitter,
            commandes::references_manette,
            commandes::profils_manette_emulateur,
            commandes::reference_reprendre,
            commandes::emulateurs_traces,
            commandes::sauvegarde_lancer,
            commandes::sauvegarde_derniere,
            commandes::restauration_liste,
            commandes::restauration_preparer,
            commandes::restauration_reposer_emulateurs,
            commandes::skin_personnel,
        ])
        .run(tauri::generate_context!())
        .expect("impossible de démarrer Frogtend");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_type_d_image_se_reconnait() {
        assert_eq!(type_d_image(b"RIFF\x00\x00\x00\x00WEBPVP8 "), "image/webp");
        assert_eq!(type_d_image(b"\x89PNG\r\n"), "image/png");
        assert_eq!(type_d_image(b"\xff\xd8\xff"), "image/jpeg");
    }

    #[test]
    fn une_adresse_d_image_se_decode() {
        assert_eq!(percent_decode("https%3A%2F%2Fimages.gog.com%2Fa.webp%3Fnamespace%3Dgamesdb"), "https://images.gog.com/a.webp?namespace=gamesdb");
        assert_eq!(percent_decode("sans%"), "sans%");
        assert_eq!(percent_decode("%e9t%C3%A9"), "\u{FFFD}té");
    }

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
