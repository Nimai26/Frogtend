//! Cœur de Frogtend : les commandes appelées par l'interface.
//!
//! Règle : les secrets (jeton Firehouse, identifiants des boutiques) restent de ce côté-ci.
//! L'interface ne les reçoit jamais.

pub mod bios;
pub mod boutiques;
pub mod cheatengine;
pub mod choix_emulateur;
pub mod clavier;
pub mod coffre;
pub mod emulateurs;
pub mod emulateurs_profils;
pub mod commandes;
pub mod erreurs;
pub mod figer;
pub mod firehouse;
pub mod garde;
pub mod installation;
pub mod jeux_pc;
pub mod lancement;
pub mod livret;
pub mod manettes;
pub mod manettes_sdl;
pub mod medias;
pub mod locale;
pub mod gratuits;
pub mod import_local;
pub mod mame;
pub mod succes;
pub mod disque;
pub mod documents;
pub mod contenus;
pub mod decompression;
pub mod osd_manette;
pub mod ludotheque;
pub mod menu_jeu;
pub mod noyau;
pub mod partie;
pub mod pilotage;
pub mod mods;
pub mod patchs_ps3;
pub mod plein_ecran;
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
/// la jaquette du profil OUVERT, en miniature si `largeur` est donnée ; avec `&visuel=fond,capture…`, le premier de
/// ces visuels du jeu (le fond du menu en jeu). Un autre profil, ou aucun, n'obtient rien.
fn repondre_jaquette(app: &tauri::AppHandle, chemin: &str, requete: Option<&str>) -> tauri::http::Response<Vec<u8>> {
    use tauri::http::Response;
    let Ok(id) = chemin.trim_matches('/').parse::<i64>() else { return reponse_vide(400) };
    let largeur = parametre(requete, "largeur").and_then(|l| l.parse::<u32>().ok());
    let noyau = app.state::<noyau::Noyau>();
    // `?media=<id>` : UNE image précise de la galerie du jeu (lot Médias M2), sans repli.
    if let Some(media) = parametre(requete, "media") {
        let Ok(media) = media.parse::<i64>() else { return reponse_vide(400) };
        return match tauri::async_runtime::block_on(noyau.media_precis(id, media, largeur.unwrap_or(1000))) {
            Ok(Some(img)) => Response::builder()
                .status(200)
                .header("Content-Type", img.type_contenu.unwrap_or_else(|| "application/octet-stream".into()))
                .header("Cache-Control", "no-store")
                .body(img.octets)
                .unwrap(),
            Ok(None) => reponse_vide(404),
            Err(e) => {
                noyau.journaliser(&format!("galerie {id}/{media} : {e:?}"));
                reponse_vide(404)
            }
        };
    }
    // `?visuel=fond,capture` : le premier de ces visuels du jeu (le fond du menu en jeu), au lieu de la jaquette.
    let visuel = parametre(requete, "visuel").map(|v| v.replace("%2C", ",").replace("%2c", ","));
    let resultat = match &visuel {
        Some(v) => {
            let sortes: Vec<&str> = v.split(',').collect();
            tauri::async_runtime::block_on(noyau.visuel(id, &sortes, largeur.unwrap_or(crate::locale::LARGEUR_GRANDE)))
        }
        None => tauri::async_runtime::block_on(noyau.jaquette(id, largeur)),
    };
    if let Err(e) = &resultat {
        noyau.journaliser(&format!("jaquette {id} (largeur {largeur:?}, visuel {visuel:?}) : {e:?}"));
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

/// Répond aux adresses `document://localhost/<jeu>/<document>/<page>?largeur=N` (sous Windows :
/// `http://document.localhost/…`) avec la page d'un livret DÉJÀ sur le PC, rendue en image pour le profil OUVERT (le
/// jeu doit lui être visible, et le document être dans la liste de CE jeu). Rien n'est téléchargé ici.
fn repondre_document(app: &tauri::AppHandle, chemin: &str, requete: Option<&str>, range: Option<&str>) -> tauri::http::Response<Vec<u8>> {
    let morceaux: Vec<&str> = chemin.trim_matches('/').split('/').collect();
    if let [jeu, doc, "fichier"] = morceaux[..] {
        return repondre_fichier_document(app, jeu, doc, range);
    }
    let [jeu, doc, page] = morceaux[..] else { return reponse_vide(400) };
    let (Ok(jeu), Ok(doc), Ok(page)) = (jeu.parse::<i64>(), doc.parse::<i64>(), page.parse::<u32>()) else { return reponse_vide(400) };
    let largeur = parametre(requete, "largeur").and_then(|l| l.parse::<u32>().ok()).unwrap_or(1000);
    let noyau = app.state::<noyau::Noyau>();
    match tauri::async_runtime::block_on(noyau.page_livret(jeu, doc, page, largeur)) {
        Ok(Some(img)) => tauri::http::Response::builder()
            .status(200)
            .header("Content-Type", "image/png")
            .header("Cache-Control", "no-store")
            .body(img.octets)
            .unwrap(),
        Ok(None) => reponse_vide(404),
        Err(e) => {
            noyau.journaliser(&format!("livret {jeu}/{doc} page {page} : {e:?}"));
            reponse_vide(404)
        }
    }
}

/// `document.localhost/<jeu>/<document>/fichier` : une musique ou une vidéo DÉJÀ sur le PC, servie par morceaux
/// (`Range` → 206, ce qui permet d'avancer dans la lecture), pour le profil OUVERT seulement.
fn repondre_fichier_document(app: &tauri::AppHandle, jeu: &str, doc: &str, range: Option<&str>) -> tauri::http::Response<Vec<u8>> {
    let (Ok(jeu), Ok(doc)) = (jeu.parse::<i64>(), doc.parse::<i64>()) else { return reponse_vide(400) };
    let noyau = app.state::<noyau::Noyau>();
    let (fichier, sorte) = match tauri::async_runtime::block_on(noyau.fichier_document(jeu, doc)) {
        Ok(Some(f)) => f,
        Ok(None) => return reponse_vide(404),
        Err(e) => {
            noyau.journaliser(&format!("document {jeu}/{doc} : {e:?}"));
            return reponse_vide(404);
        }
    };
    reponse_morceau(&fichier, sorte, range)
}

/// La réponse à une demande de morceau d'un fichier (sans garde : `repondre_fichier_document` l'a faite).
fn reponse_morceau(fichier: &std::path::Path, sorte: &str, range: Option<&str>) -> tauri::http::Response<Vec<u8>> {
    let Ok(total) = std::fs::metadata(fichier).map(|m| m.len()) else { return reponse_vide(404) };
    let (debut, fin, partiel) = match documents::plage(range, total) {
        Ok(Some((d, f))) => (d, f, true),
        // Sans plage (rare : les lecteurs audio et vidéo en envoient toujours une) : le fichier entier, d'un bloc
        // (quelques dizaines de Mo au plus pour un document).
        Ok(None) if total == 0 => return reponse_vide(404),
        Ok(None) => (0, total - 1, false),
        Err(()) => {
            return tauri::http::Response::builder()
                .status(416)
                .header("Content-Range", format!("bytes */{total}"))
                .body(Vec::new())
                .unwrap()
        }
    };
    let Ok(octets) = documents::lire_morceau(fichier, debut, fin) else { return reponse_vide(500) };
    let mut r = tauri::http::Response::builder()
        .status(if partiel { 206 } else { 200 })
        .header("Content-Type", sorte)
        .header("Accept-Ranges", "bytes")
        // Tauri (wry) ne le pose pas lui-même : sans lui, le lecteur ne sait pas ce qu'il reçoit.
        .header("Content-Length", octets.len())
        .header("Cache-Control", "no-store");
    if partiel {
        r = r.header("Content-Range", format!("bytes {debut}-{fin}/{total}"));
    }
    r.body(octets).unwrap()
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
            // Règle « pas de pieuvre » (0.30.0) : les médias gardés dans les données de Frogtend rejoignent le dossier
            // de leur jeu (copie vérifiée). Un disque absent : ce sera pour un prochain démarrage.
            // Le dossier des abris de parties (réglage du PC) ; les abris de l'ancien emplacement y sont rangés.
            n.regler_abris(commandes::dossier_abris_regle(app.handle()));
            // Un jeu laissé figé par un Frogtend arrêté brutalement repart (d'après la note du menu en jeu).
            let relances = figer::initialiser(dossier.join("menu-fige.txt"));
            // Si Frogtend plante pendant « manette → clavier », aucune touche ne reste enfoncée dans Windows.
            clavier::initialiser();
            if relances > 0 {
                n.journaliser(&format!("Menu en jeu : {relances} fils d'un jeu resté figé relancés au démarrage"));
            }
            app.manage(n);
            // Les rangements (copies vérifiées, disques parfois lents ou absents) se font à part : la fenêtre s'ouvre
            // tout de suite.
            let h = app.handle().clone();
            std::thread::spawn(move || {
                let n = h.state::<noyau::Noyau>();
                if let Err(e) = n.migrer_medias() {
                    n.journaliser(&format!("migration des médias : {e:?}"));
                }
                if let Err(e) = n.migrer_abris() {
                    n.journaliser(&format!("migration des abris : {e:?}"));
                }
            });
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
        .register_asynchronous_uri_scheme_protocol("document", |ctx, requete, repondeur| {
            let app = ctx.app_handle().clone();
            let chemin = requete.uri().path().to_string();
            let parametres = requete.uri().query().map(String::from);
            let range = requete.headers().get("range").and_then(|v| v.to_str().ok()).map(String::from);
            // Hors du fil de l'interface : le rendu d'une page prend un moment.
            std::thread::spawn(move || {
                let reponse = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    repondre_document(&app, &chemin, parametres.as_deref(), range.as_deref())
                }))
                .unwrap_or_else(|_| reponse_vide(500));
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
            commandes::ludotheque_infos,
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
            commandes::documents_jeu,
            commandes::medias_jeu,
            commandes::jeux_reunions,
            commandes::livret_ouvrir,
            commandes::document_preparer,
            commandes::espace_libre,
            commandes::installation_preparer,
            commandes::installation_lancer,
            commandes::installation_ailleurs,
            commandes::lancement_candidats,
            commandes::lanceur_choisir,
            commandes::jeu_jouer,
            commandes::parties_abri,
            commandes::jeu_retirer,
            commandes::jeu_desinstaller_reste,
            commandes::parcourir,
            commandes::rpcs3_image,
            commandes::rpcs3_regler_image,
            commandes::rpcs3_patchs,
            commandes::rpcs3_patch_regler,
            commandes::mods_installes,
            commandes::mod_installer,
            commandes::mod_desinstaller,
            commandes::emulateurs_recommandes,
            commandes::emulateurs_installes,
            commandes::emulateur_fiche,
            commandes::emulateur_derniere_version,
            commandes::emulateur_installer,
            commandes::emulateur_adopter,
            commandes::retroarch_etat,
            commandes::retroarch_installer_coeur,
            commandes::bios_poser,
            commandes::firmware_servi,
            commandes::rpcs3_micrologiciel_servi,
            commandes::emulateur_regler_manette,
            commandes::rpcs3_installer_micrologiciel,
            commandes::rpcs3_micrologiciel,
            commandes::taodbox_lance,
            commandes::taodbox_precharger,
            commandes::emulateur_installer_firehouse,
            commandes::jeu_triches,
            commandes::cheatengine_lancer,
            commandes::gratuits_liste,
            commandes::gratuits_connexion_epic,
            commandes::gratuits_obtenir_epic,
            commandes::gratuits_connexion_playstation,
            commandes::gratuits_psplus,
            commandes::gratuits_connexion,
            commandes::gratuits_recuperer,
            commandes::abris_regler,
            commandes::contenus_du_jeu,
            commandes::contenus_installer,
            commandes::archive_du_jeu,
            commandes::decompresser_jeu,
            commandes::ra_etat,
            commandes::ra_regler,
            commandes::ra_oublier,
            commandes::ra_connecter_emulateurs,
            commandes::succes_retro,
            commandes::succes_steam,
            commandes::jeu_choisir_version,
            commandes::import_types_fichiers,
            commandes::import_chercher_roms,
            commandes::import_mesurer,
            commandes::import_copier,
            commandes::import_chercher_dos,
            commandes::import_chercher_mame,
            commandes::import_installer_dos,
            commandes::import_arguments_jeu_dos,
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
            commandes::jeu_anti_triche,
            commandes::jeu_plein_ecran_suggestion,
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
        .build(tauri::generate_context!())
        .expect("impossible de démarrer Frogtend")
        .run(|app, evenement| {
            // Frogtend se ferme pendant que le menu fige un jeu : le jeu repart d'abord (jamais un jeu laissé figé).
            if let tauri::RunEvent::Exit = evenement {
                commandes::degeler_la_partie(app);
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_morceau_de_document_se_sert_avec_ses_en_tetes() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("6842.mp4");
        std::fs::write(&f, b"0123456789").unwrap();
        let entete = |r: &tauri::http::Response<Vec<u8>>, n: &str| r.headers().get(n).map(|v| v.to_str().unwrap().to_string());
        // Une plage : 206, taille du morceau, place dans le fichier.
        let r = reponse_morceau(&f, "video/mp4", Some("bytes=2-5"));
        assert_eq!(r.status(), 206);
        assert_eq!(r.body(), b"2345");
        assert_eq!(entete(&r, "Content-Length").as_deref(), Some("4"));
        assert_eq!(entete(&r, "Content-Range").as_deref(), Some("bytes 2-5/10"));
        assert_eq!(entete(&r, "Content-Type").as_deref(), Some("video/mp4"));
        assert_eq!(entete(&r, "Accept-Ranges").as_deref(), Some("bytes"));
        // Sans plage (ou une plage mal formée) : 200, tout le fichier, sans Content-Range.
        for range in [None, Some("bytes=a-b")] {
            let r = reponse_morceau(&f, "video/mp4", range);
            assert_eq!(r.status(), 200);
            assert_eq!(r.body(), b"0123456789");
            assert_eq!(entete(&r, "Content-Length").as_deref(), Some("10"));
            assert!(entete(&r, "Content-Range").is_none());
        }
        // Hors du fichier : 416 avec la taille.
        let r = reponse_morceau(&f, "video/mp4", Some("bytes=10-"));
        assert_eq!(r.status(), 416);
        assert_eq!(entete(&r, "Content-Range").as_deref(), Some("bytes */10"));
        // Fichier absent : 404.
        assert_eq!(reponse_morceau(&d.path().join("absent.mp4"), "video/mp4", None).status(), 404);
    }

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
