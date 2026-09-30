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
) -> Resultat<()> {
    let plateforme = noyau.registre().jeu(id)?.map(|j| j.plateforme).unwrap_or_default();
    let emulateur = match emulateur_regle(&app, &plateforme, id, emulateur.as_deref()) {
        Some((programme, ligne)) => {
            Some(preparer_emulateur(&app, &noyau, &plateforme, &programme, &ligne, &commandes.unwrap_or_default()).await?)
        }
        None => None,
    };
    let (pid, dossiers, carte_cedee) = noyau.jouer(id, emulateur).await?;
    let _ = app.emit("partie", EvenementPartie::Debut { jeu: id, carte_cedee });
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        let noyau = app2.state::<Noyau>();
        match noyau.suivre_partie(id, pid, dossiers).await {
            Ok(fin) => {
                let _ = app2.emit("partie", EvenementPartie::Fin { jeu: id, secondes: fin.secondes });
            }
            Err(e) => noyau.journaliser(&format!("suivi de la partie du jeu {id} : {e:?}")),
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn parties_abri(noyau: State<'_, Noyau>, id: i64) -> Resultat<crate::partie::Abri> {
    noyau.mettre_a_l_abri(id).await
}

#[tauri::command]
pub async fn jeu_retirer(noyau: State<'_, Noyau>, id: i64) -> Resultat<crate::partie::Abri> {
    noyau.retirer_du_pc(id).await
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
pub fn emulateur_adopter(noyau: State<'_, Noyau>, id: String, programme: String) -> Resultat<crate::emulateurs::EmulateurInstalle> {
    let f = crate::emulateurs::fiche(&id).ok_or_else(|| Erreur::Introuvable("Émulateur inconnu.".into()))?;
    let p = std::path::PathBuf::from(&programme);
    if !p.is_file() {
        return Err(Erreur::Disque(format!("Programme introuvable : {programme}.")));
    }
    let e = crate::emulateurs::EmulateurInstalle {
        id: f.id.into(),
        nom: f.nom.into(),
        version: None,
        dossier: p.parent().unwrap_or(&p).to_string_lossy().into(),
        programme,
        par_frogtend: false,
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
    let (id, n, d, j) = (f.id.to_string(), profil.nom.clone(), dossier.clone(), jeux.clone());
    let avant = tauri::async_runtime::spawn_blocking(move || {
        // Les références sont déposées dans les profils de l'émulateur (on les y retrouve).
        let utilisateur = crate::emulateurs_profils::dossier_du_profil(&d, &n).join("User");
        crate::references::deposer(&crate::references::toutes(&magasin, &id), &d, &utilisateur)?;
        crate::emulateurs_profils::preparer(&id, &d, &n, &j, &manette)
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
