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

/// Les anciens dossiers d'abris encore à lire (`pc.json` ▸ `anciensDossiersAbris`).
pub(crate) fn anciens_dossiers_abris(app: &AppHandle) -> Vec<std::path::PathBuf> {
    let Some(r) = app.store("pc.json").ok().and_then(|s| s.get("reglages")) else { return Vec::new() };
    r["anciensDossiersAbris"].as_array().into_iter().flatten().filter_map(|v| v.as_str()).filter(|s| !s.trim().is_empty()).map(std::path::PathBuf::from).collect()
}

/// Ce que Frogtend a écrit dans un dossier d'abris ou de captures (pour l'accord chiffré avant un déplacement).
#[tauri::command]
pub async fn dossier_contenu(sorte: String, chemin: String) -> Resultat<crate::demenagement::Contenu> {
    let d = std::path::PathBuf::from(chemin.trim());
    tauri::async_runtime::spawn_blocking(move || crate::demenagement::mesurer(&d, &sorte))
        .await
        .map_err(|_| Erreur::Disque("La lecture du dossier s'est arrêtée brutalement.".into()))
}

/// Progression d'un déplacement de dossier.
#[derive(Clone, Serialize)]
struct ProgresDemenagement {
    octets: u64,
}

/// Un passage du rangement des captures de la partie en cours (rend faux quand elle est finie).
type PassageCaptures = std::sync::Arc<dyn Fn() -> bool + Send + Sync>;

/// Celui de la partie en cours, pour que la galerie range d'abord la photo qui vient d'être prise.
static PASSAGE_EN_COURS: std::sync::Mutex<Option<PassageCaptures>> = std::sync::Mutex::new(None);

/// Un déplacement du dossier des captures est en cours : la galerie n'oublie alors aucune capture absente.
static DEPLACEMENT_CAPTURES: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Déplace ce que Frogtend a écrit dans un ancien dossier d'abris ou de captures (✅ Seb 04/11 ; seulement ses abris
/// `<numéro>\…` ou ses captures `<Système>\<jeu>\<image>`, jamais par-dessus).
/// - Abris : appelé APRÈS le changement de réglage (le noyau écrit déjà les nouveaux abris dans le nouveau dossier) ;
///   `ancien` doit être dans `anciensDossiersAbris` et `nouveau` le dossier réglé (relus ici dans `pc.json`).
/// - Captures : appelé AVANT le changement ; `ancien` doit être le dossier encore réglé.
/// Un ancien dossier qui ne serait pas admis aujourd'hui (racine d'un disque, Documents…) n'est jamais vidé : il reste lu.
#[tauri::command]
pub async fn dossier_deplacer_contenu(app: AppHandle, noyau: State<'_, Noyau>, sorte: String, ancien: String, nouveau: String) -> Resultat<crate::demenagement::Bilan> {
    let (ancien, nouveau) = (std::path::PathBuf::from(ancien.trim()), std::path::PathBuf::from(nouveau.trim()));
    let reglages = app.store("pc.json").ok().and_then(|s| s.get("reglages")).unwrap_or(Value::Null);
    let regle = |cle: &str| reglages[cle].as_str().map(str::trim).filter(|s| !s.is_empty()).map(std::path::PathBuf::from);
    match sorte.as_str() {
        "abris" => {
            let connu = anciens_dossiers_abris(&app).iter().any(|a| crate::garde::meme(a, &ancien));
            if !connu || !regle("dossierAbris").is_some_and(|r| crate::garde::meme(&r, &nouveau)) {
                return Err(Erreur::Refus("Ces dossiers d'abris ne sont pas ceux du réglage : rien n'est déplacé.".into()));
            }
            // Un abri en train de s'écrire ou d'être reposé : jamais déplacé en même temps.
            if app.state::<crate::menu_jeu::MenuJeu>().partie().is_some() || !noyau.occupes.lock().unwrap_or_else(|e| e.into_inner()).is_empty() {
                return Err(Erreur::Refus("Un jeu est en cours (partie, retrait ou installation) : attends qu'il finisse, puis recommence.".into()));
            }
        }
        "captures" => {
            if !regle("dossierCaptures").is_some_and(|r| crate::garde::meme(&r, &ancien)) {
                return Err(Erreur::Refus("Ce dossier n'est pas celui des captures réglé : rien n'est déplacé.".into()));
            }
            // Les captures d'une partie en cours iraient dans le dossier en train de partir.
            if app.state::<crate::menu_jeu::MenuJeu>().partie().is_some() {
                return Err(Erreur::Refus("Une partie est en cours : quitte le jeu, puis recommence.".into()));
            }
        }
        _ => return Err(Erreur::Refus("Seuls les abris de parties et les captures se déplacent ainsi.".into())),
    }
    if let Some(m) = refus_dossier(&nouveau, &sorte) {
        return Err(Erreur::Refus(m));
    }
    if let Some(m) = refus_dossier(&ancien, &sorte) {
        return Err(Erreur::Refus(format!("L'ancien dossier n'est pas vidé ({m}) : Frogtend continue seulement d'y lire.")));
    }
    noyau.journaliser(&format!("déplacement du dossier des {sorte} : {} → {}", ancien.display(), nouveau.display()));
    let (est_captures, ancien2, nouveau2) = (sorte == "captures", ancien.clone(), nouveau.clone());
    if est_captures {
        DEPLACEMENT_CAPTURES.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    let app2 = app.clone();
    let resultat = tauri::async_runtime::spawn_blocking(move || {
        let mut dernier = std::time::Instant::now();
        crate::demenagement::deplacer_contenu(&sorte, &ancien, &nouveau, &mut |octets| {
            if dernier.elapsed().as_millis() >= 250 {
                dernier = std::time::Instant::now();
                let _ = app2.emit("demenagement", ProgresDemenagement { octets });
            }
        })
    })
    .await
    .map_err(|_| Erreur::Disque("Le déplacement s'est arrêté brutalement.".into()))
    .and_then(|r| r);
    // Les captures déplacées restent les mêmes pour la galerie et la sauvegarde (sinon oubliées comme disparues) :
    // même si le déplacement s'est arrêté en route (ce qui a bougé est suivi).
    if est_captures {
        match crate::captures::suivre_deplacement(&noyau.dossier, &ancien2, &nouveau2) {
            Ok(n) => noyau.journaliser(&format!("déplacement : {n} capture(s) suivie(s) dans la galerie")),
            Err(e) => noyau.journaliser(&format!("déplacement : galerie non mise à jour ({e})")),
        }
        DEPLACEMENT_CAPTURES.store(false, std::sync::atomic::Ordering::SeqCst);
    }
    let bilan = resultat?;
    noyau.journaliser(&format!("déplacement : {} fichier(s) déplacé(s), {} laissé(s)", bilan.deplaces, bilan.laisses.len()));
    for l in bilan.laisses.iter().take(20) {
        noyau.journaliser(&format!("  laissé à sa place : {l}"));
    }
    Ok(bilan)
}

/// Une capture de la galerie (celles du profil ouvert seulement).
#[derive(Serialize)]
pub struct CaptureVue {
    id: String,
    jeu: i64,
    titre: String,
    plateforme: String,
    quand: u64,
    /// Le jeu a une fiche (Firehouse) : la capture peut en devenir le fond.
    fond_possible: bool,
    /// C'est le fond choisi de son jeu.
    est_le_fond: bool,
}

/// La fiche (Firehouse) d'un jeu de capture pour ce profil : son fond est rangé sous elle (c'est elle que demandent
/// le menu en jeu, l'écran de lancement et Taodbox). `None` : un import sans fiche.
fn fiche_de_capture(noyau: &Noyau, s: &crate::noyau::Session, jeu: i64) -> Option<i64> {
    noyau.fiche_de(s, jeu).ok().flatten()
}

/// La galerie du profil OUVERT : ses captures (d'un jeu — son numéro ou sa fiche —, ou toutes), plus récentes
/// d'abord. Un jeu de Firehouse doit être encore visible à ce profil (même retiré du PC) ; une capture disparue du
/// disque sort de l'index.
#[tauri::command]
pub async fn captures_liste(app: AppHandle, noyau: State<'_, Noyau>, jeu: Option<i64>) -> Resultat<Vec<CaptureVue>> {
    let regle = dossier_captures_regle(&app);
    let deplacement = DEPLACEMENT_CAPTURES.load(std::sync::atomic::Ordering::SeqCst);
    // Une partie en cours : ses nouvelles photos sont rangées d'abord (une photo prise à l'instant y est).
    let passage = PASSAGE_EN_COURS.lock().unwrap_or_else(|e| e.into_inner()).clone();
    if let Some(p) = passage {
        if !tauri::async_runtime::spawn_blocking(move || p()).await.unwrap_or(false) {
            *PASSAGE_EN_COURS.lock().unwrap_or_else(|e| e.into_inner()) = None;
        }
    }
    let profil = profil_ouvert(&noyau).await?.id;
    let s = noyau.session().await?;
    let fonds = crate::captures::fonds_choisis(&dossier_profil_de(&noyau, &profil));
    let mut l = Vec::new();
    for n in crate::captures::index(&noyau.dossier).into_iter().filter(|n| n.profil == profil) {
        if !std::path::Path::new(&n.chemin).is_file() {
            // Oubliée seulement si son dossier est là (effacée à la main) : jamais pendant un déplacement du dossier, ni
            // si elle est retrouvée sous le dossier réglé (déplacée) ; un disque débranché ne fait rien oublier (elle
            // resterait sinon hors de la sauvegarde chez Firehouse).
            if !deplacement && !regle.as_deref().is_some_and(|r| crate::captures::retrouver(&noyau.dossier, &n.id, r)) && std::path::Path::new(&n.chemin).parent().is_some_and(|d| d.is_dir()) {
                let _ = crate::captures::oublier(&noyau.dossier, &n.id);
            }
            continue;
        }
        // Un jeu de Firehouse caché à ce profil (grade, réunion) : sa capture ne se montre pas (rien n'est effacé).
        if n.jeu > 0 && !s.verrou().visible_ou_reuni(n.jeu).unwrap_or(false) {
            continue;
        }
        let fiche = fiche_de_capture(&noyau, &s, n.jeu);
        if jeu.is_some_and(|j| n.jeu != j && fiche != Some(j)) {
            continue;
        }
        let est_le_fond = fiche.is_some_and(|f| fonds.get(&f.to_string()) == Some(&n.id));
        l.push(CaptureVue { id: n.id, jeu: n.jeu, titre: n.titre, plateforme: n.plateforme, quand: n.quand, fond_possible: fiche.is_some(), est_le_fond });
    }
    l.sort_by(|a, b| b.quand.cmp(&a.quand));
    Ok(l)
}

/// Supprime une capture du profil ouvert (la personne a confirmé) ; déjà partie du disque : seulement oubliée. Un fond
/// choisi qui était elle est retiré.
#[tauri::command]
pub async fn capture_supprimer(noyau: State<'_, Noyau>, id: String) -> Resultat<()> {
    let p = profil_ouvert(&noyau).await?;
    let n = crate::captures::notee_de_ce_profil(&noyau.dossier, &id, &p.id).ok_or_else(|| Erreur::Introuvable("Capture introuvable.".into()))?;
    match std::fs::remove_file(&n.chemin) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    crate::captures::oublier(&noyau.dossier, &id)?;
    let dossier = dossier_profil_de(&noyau, &p.id);
    if let Some(f) = noyau.session().await.ok().and_then(|s| fiche_de_capture(&noyau, &s, n.jeu)) {
        if crate::captures::fonds_choisis(&dossier).get(&f.to_string()) == Some(&id) {
            crate::captures::regler_fond(&dossier, f, None)?;
        }
    }
    noyau.journaliser(&format!("capture supprimée : {}", n.chemin));
    Ok(())
}

/// Ouvre dans l'Explorateur le dossier d'une capture du profil ouvert.
#[tauri::command]
pub async fn capture_ouvrir_dossier(app: AppHandle, noyau: State<'_, Noyau>, id: String) -> Resultat<()> {
    let p = profil_ouvert(&noyau).await?;
    let n = crate::captures::de_ce_profil(&noyau.dossier, &id, &p.id).ok_or_else(|| Erreur::Introuvable("Capture introuvable.".into()))?;
    use tauri_plugin_opener::OpenerExt;
    let dossier = std::path::Path::new(&n.chemin).parent().ok_or_else(|| Erreur::Introuvable("Dossier introuvable.".into()))?.to_path_buf();
    app.opener().open_path(dossier.to_string_lossy(), None::<&str>).map_err(|e| Erreur::Refus(format!("Impossible d'ouvrir le dossier : {e}")))
}

/// « En faire le fond du jeu » (✅ Seb 06/11 : pour SON profil), rangé sous la fiche du jeu ; `actif` faux : rendre le
/// fond de Firehouse. Une capture trop petite pour un fond net (moins de 640 × 360) est refusée.
#[tauri::command]
pub async fn capture_fond(noyau: State<'_, Noyau>, id: String, actif: bool) -> Resultat<()> {
    let p = profil_ouvert(&noyau).await?;
    let n = crate::captures::de_ce_profil(&noyau.dossier, &id, &p.id).ok_or_else(|| Erreur::Introuvable("Capture introuvable.".into()))?;
    let s = noyau.session().await?;
    let fiche = fiche_de_capture(&noyau, &s, n.jeu).ok_or_else(|| Erreur::Refus("Ce jeu n'a pas de fiche Firehouse : il n'a pas de fond à remplacer.".into()))?;
    if actif {
        let octets = std::fs::read(&n.chemin)?;
        if !crate::captures::assez_grande_pour_un_fond(&octets) {
            return Err(Erreur::Refus("Cette capture est trop petite pour un fond net (moins de 640 × 360).".into()));
        }
    }
    crate::captures::regler_fond(&dossier_profil_de(&noyau, &p.id), fiche, actif.then_some(id.as_str()))?;
    Ok(())
}

/// Le dossier général des captures d'écran (`pc.json` ▸ `dossierCaptures`, ✅ Seb 04/11), s'il est réglé et admis.
fn dossier_captures_regle(app: &AppHandle) -> Option<std::path::PathBuf> {
    let r = app.store("pc.json").ok()?.get("reglages")?;
    let d = r["dossierCaptures"].as_str().filter(|s| !s.trim().is_empty()).map(std::path::PathBuf::from)?;
    // Revérifié à l'usage (l'interface peut se tromper) : jamais un dossier de Windows ni la racine d'un disque.
    refus_dossier(&d, "captures").is_none().then_some(d)
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
    noyau.regler_abris_anciens(anciens_dossiers_abris(&app));
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
    // Un émulateur ouvert seul tourne avec le profil d'avant (✅ Seb 17/11) : on ne change pas de profil sous lui.
    noyau.refuser_si_emulateur_seul("change de profil")?;
    let p = noyau.ouvrir(&id, pin.as_deref(), &connexion(&app)).await?;
    // Les jeux déjà arrivés qu'il n'y avait rien à « installer » (ROM, images disque) deviennent prêts à jouer, en
    // arrière-plan (registre, et parties restaurées qui les attendaient).
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        let n = app2.state::<Noyau>().installer_les_arrives().await;
        if n > 0 {
            let _ = app2.emit("jeux-pc-changes", n);
        }
    });
    // Les envois Switch déjà acceptés par cette personne et restés en file (Frogtend fermé, connexion coupée) reprennent.
    lancer_envois_switch(&app);
    Ok(p)
}

#[tauri::command]
pub async fn profil_fermer(noyau: State<'_, Noyau>) -> Resultat<()> {
    noyau.refuser_si_emulateur_seul("change de profil")?;
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
    noyau.refuser_si_emulateur_seul("supprime le profil")?;
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
    autres: Option<Vec<i64>>,
) -> Resultat<JeuPc> {
    // `autres` : d'autres versions (régions) du même jeu, sur la même ligne (✅ Seb 28/10).
    let j = noyau.ajouter_versions(id, version, &autres.unwrap_or_default(), &emplacement, &emplacements(&app)).await?;
    lancer_file(&app);
    Ok(j)
}

/// Retire UNE carte d'un jeu e-Reader du PC (ses fichiers seulement ; jamais la dernière carte, jamais les parties).
#[tauri::command]
pub async fn jeu_retirer_carte(noyau: State<'_, Noyau>, id: i64, version: i64) -> Resultat<JeuPc> {
    noyau.retirer_carte(id, version).await
}

/// Retire UNE version ajoutée (région) d'un jeu, reçue ou en route (jamais la version d'origine, jamais les parties).
#[tauri::command]
pub async fn jeu_retirer_version(noyau: State<'_, Noyau>, id: i64, version: i64) -> Resultat<JeuPc> {
    noyau.retirer_version(id, version).await
}

/// Les versions ajoutées d'un jeu du PC et où elles en sont (en attente, en route, en pause, en erreur, reçues).
#[tauri::command]
pub fn jeu_versions_ajoutees(noyau: State<'_, Noyau>, id: i64) -> Resultat<Vec<crate::jeux_pc::VersionAjoutee>> {
    noyau.registre().versions_ajoutees(id)
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

/// Avant de cocher plusieurs versions d'une fiche : ce que le cœur en dit (une seule règle, la sienne).
#[tauri::command]
pub fn versions_avis(fiche: Value) -> Vec<crate::partie::AvisVersion> {
    crate::partie::avis_versions(&fiche)
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
        "captures" => ("des captures d'écran", "Captures"),
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
    /// Une mise à jour PS3 : sa version (« 01.05 », contrat 1.31), dans l'ordre d'installation.
    pub version: Option<String>,
    /// Ryubing et Eden : pourquoi ce contenu trouvé n'est pas utilisé, quand la cause est TECHNIQUE (en mots simples).
    pub raison: Option<String>,
    /// Switch (✅ Seb 17/11) : la clé de son interrupteur (« maj », ou l'identifiant du DLC) ; `None` : pas d'interrupteur
    /// (une vieille mise à jour, un contenu d'une autre console).
    pub cle: Option<String>,
    /// Switch : le choix de la maison (allumé d'office).
    pub actif: bool,
}

/// Les contenus additionnels d'un jeu du PC.
#[derive(Serialize, Default)]
pub struct ContenusDuJeu {
    /// `aucun` (console pas encore gérée), `emulateur` (pas d'émulateur réglé), `ok`.
    pub etat: String,
    /// `ps3` (RPCS3 installe chaque paquet), `eden` et `ryubing` (donnés tout seuls au lancement, ✅ Seb 16/11), `wiiu`,
    /// `switch` (aucun émulateur Switch utilisable réglé).
    pub systeme: String,
    pub titre_id: Option<String>,
    pub contenus: Vec<ContenuVu>,
    /// Les fichiers servis comme contenus mais illisibles (en-tête abîmé) : dits, jamais écartés en silence.
    #[serde(default)]
    pub ignores: Vec<String>,
    /// `ryubing` : une phrase à dire (clés absentes, choix de la personne gardé…).
    pub remarque: Option<String>,
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

/// Switch : les dossiers où chercher les mises à jour et DLC d'un jeu, en plus du dossier de son fichier (regardé en
/// premier, avec ses sous-dossiers) : le dossier du jeu dans la ludothèque, son dossier d'installation et les
/// emplacements réglés de la plateforme (⚙ Options ▸ 📁 Emplacements) — la même liste que l'aperçu d'avant le 16/11.
/// Le rattachement se fait toujours par l'identifiant lu dans les en-têtes (switch_nca.rs), jamais par le titre.
async fn dossiers_switch(app: &AppHandle, noyau: &Noyau, id: i64) -> Vec<std::path::PathBuf> {
    let Ok(j) = noyau.jeu_visible(id).await else { return vec![] };
    let mut dossiers: Vec<std::path::PathBuf> = vec![std::path::PathBuf::from(&j.dossier)];
    if let Some(i) = &j.installation {
        dossiers.push(std::path::PathBuf::from(&i.dossier));
    }
    let e = emplacements(app);
    dossiers.extend(e.systemes.get(&j.plateforme).cloned().unwrap_or_default().into_iter().map(std::path::PathBuf::from));
    dossiers.retain(|d| !d.as_os_str().is_empty());
    dossiers.sort();
    dossiers.dedup();
    dossiers
}

/// Switch lancée par Eden (✅ Seb 16/11 : automatique) : ce que Frogtend donne TOUT SEUL à Eden à chaque lancement (le
/// dossier du jeu, s'il contient sa mise à jour ou ses DLC), montré sans rien écrire. `None` : pas un jeu Switch ;
/// `Err(Reglage)` : aucun émulateur Switch réglé, ou un autre qu'Eden et Ryubing (Ryubing est traité avant).
async fn contenus_eden(app: &AppHandle, noyau: &Noyau, id: i64) -> Resultat<Option<ContenusDuJeu>> {
    let j = noyau.jeu_visible(id).await?;
    if !j.plateforme.eq_ignore_ascii_case("Nintendo Switch") {
        return Ok(None);
    }
    let Some((programme, _)) = emulateur_regle(app, &j.plateforme, id, None) else { return Err(Erreur::Reglage("Aucun Eden réglé pour la Switch.".into())) };
    // Seul Eden lit une liste de dossiers de contenus (`qt-config.ini`) : un autre émulateur Switch (Citron…) n'y
    // verrait rien, et Frogtend écrirait un fichier inutile chez lui.
    let p = std::path::Path::new(&programme);
    if !p.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("eden.exe")) {
        // Un RÉGLAGE à changer (pas un refus) : l'aperçu montre alors « Règle Eden ou Ryubing… » au lieu de rien.
        return Err(Erreur::Reglage("Frogtend sait donner les mises à jour et DLC à Eden et à Ryubing seulement : choisis l'un d'eux comme émulateur de la Switch pour ce jeu.".into()));
    }
    let eden = p.parent().map(std::path::PathBuf::from).unwrap_or_default();
    let fichier = noyau.fichier_pour_l_emulateur(id, None).await.ok().flatten();
    let donnees = noyau.dossier.clone();
    let ailleurs = dossiers_switch(app, noyau, id).await;
    let r = tauri::async_runtime::spawn_blocking(move || apercu_eden(&donnees, &eden, fichier.as_deref(), &ailleurs))
        .await
        .map_err(|_| Erreur::Disque("La lecture s'est arrêtée brutalement.".into()))?;
    Ok(Some(r))
}

/// L'aperçu des contenus Eden d'un jeu (`fichier` : celui qu'Eden lancera ; `ailleurs` : les autres dossiers où
/// chercher ; `donnees` : celles de Frogtend, où vit le choix de la maison). Rien n'est écrit chez Eden (seuls le
/// cache des fichiers lus et, une fois, le choix « tous les DLC éteints » repris d'Eden et remplacé par leurs
/// identifiants). Chaque contenu porte sa clé et son interrupteur (✅ Seb 17/11 : Frogtend décide).
fn apercu_eden(donnees: &std::path::Path, eden: &std::path::Path, fichier: Option<&std::path::Path>, ailleurs: &[std::path::PathBuf]) -> ContenusDuJeu {
    use crate::eden::LectureContenus;
    let dire = |m: &str| ContenusDuJeu { etat: "ok".into(), systeme: "eden".into(), remarque: Some(m.into()), ..Default::default() };
    let Some(f) = fichier else { return dire("Le fichier de ce jeu est introuvable : aucune mise à jour ni DLC n'est utilisé.") };
    let c = match crate::eden::lire_contenus(eden, f, ailleurs) {
        LectureContenus::Lus(c) => c,
        LectureContenus::PasPortable => return dire("Eden n'a pas été installé par Frogtend : ses mises à jour et DLC se règlent dans Eden."),
        LectureContenus::SansCles => {
            return dire(
                "Eden n'a pas encore les clés de la console. Lance ce jeu une fois (▶ Jouer) : Frogtend te proposera de les poser ; ses mises à jour et DLC seront ensuite utilisés tout seuls.",
            )
        }
        LectureContenus::Illisible => return dire("Frogtend n'arrive pas à lire le fichier de ce jeu : aucune mise à jour ni DLC n'est utilisé."),
    };
    let (choix, abime) = crate::contenus_switch::choix_resolu(donnees, c.base, &crate::ryubing::ids_dlc(&c), c.complet());
    let (_, (_, dlc_allumes)) = crate::eden::etat_voulu(&c, &choix);
    let dlc_allumes = if dlc_allumes == 0 && !c.complet() { 1 } else { dlc_allumes };
    let mut contenus: Vec<ContenuVu> = c.maj.iter().map(|m| contenu_switch_vu(m, "maj", Some(crate::contenus_switch::CLE_MAJ.into()), choix.maj, choix.maj, None)).collect();
    contenus.extend(c.majs_anciennes.iter().map(|m| contenu_switch_vu(m, "maj", None, false, false, Some("pas utilisée : une plus récente est là"))));
    // Eden ne sait couper que TOUS les DLC : tant qu'un seul est allumé, il les utilise tous.
    contenus.extend(c.dlc.iter().map(|m| {
        let id = cle_du_contenu(m);
        let actif = id.as_deref().is_some_and(|i| choix.dlc_actif(i));
        contenu_switch_vu(m, "dlc", id, actif, dlc_allumes > 0, None)
    }));
    contenus.extend(contenus_nsz_vus(&c));
    let mut remarques = crate::eden::remarques_contenus(&c);
    remarques.extend(remarque_inaccessibles(&c));
    if abime.is_some() {
        remarques.insert(0, crate::contenus_switch::ABIME_SUR_LA_FICHE.into());
    }
    ContenusDuJeu {
        etat: "ok".into(),
        systeme: "eden".into(),
        titre_id: Some(format!("{:016X}", c.base)),
        contenus,
        ignores: c.illisibles,
        remarque: (!remarques.is_empty()).then(|| remarques.join("\n")),
    }
}

/// Les mises à jour et DLC .nsz d'un jeu (✅ Seb 17/11) : montrés sur la fiche, sans interrupteur, avec la raison
/// technique (aucun émulateur ne les lit) ; ils peuvent être envoyés à Firehouse.
fn contenus_nsz_vus(c: &crate::switch_nca::ContenusDuJeu) -> Vec<ContenuVu> {
    c.compresses
        .iter()
        .map(|m| {
            let genre = if m.sorte == crate::switch_nca::Sorte::MiseAJour { "maj" } else { "dlc" };
            contenu_switch_vu(m, genre, None, false, false, Some(crate::switch_nca::RAISON_NSZ))
        })
        .collect()
}

/// Les fichiers que le disque n'a pas laissé lire cette fois (utilisés par un autre programme) : dits, relus ensuite.
fn remarque_inaccessibles(c: &crate::switch_nca::ContenusDuJeu) -> Option<String> {
    (!c.inaccessibles.is_empty()).then(|| {
        format!(
            "{} fichier(s) utilisé(s) par un autre programme n'ont pas pu être lus cette fois ({}) : ils le seront à la prochaine ouverture.",
            c.inaccessibles.len(),
            crate::switch_nca::quelques_noms(&c.inaccessibles)
        )
    })
}

/// La clé d'un contenu Switch pour son interrupteur : « maj » pour une mise à jour, l'identifiant d'un DLC.
fn cle_du_contenu(m: &crate::switch_nca::ContenuLu) -> Option<String> {
    match &m.sorte {
        crate::switch_nca::Sorte::MiseAJour => Some(crate::contenus_switch::CLE_MAJ.into()),
        crate::switch_nca::Sorte::Dlc(n) => Some(crate::switch_nca::identifiant_dlc(n)),
        crate::switch_nca::Sorte::Jeu => None,
    }
}

/// Un contenu Switch vu sur la fiche (`cle` : son interrupteur, `None` pour une vieille mise à jour ; `actif` : le
/// choix de Frogtend ; `utilise` : ce que l'émulateur fera vraiment ; `raison` : une cause TECHNIQUE seulement).
fn contenu_switch_vu(m: &crate::switch_nca::ContenuLu, genre: &str, cle: Option<String>, actif: bool, utilise: bool, raison: Option<&str>) -> ContenuVu {
    ContenuVu {
        id: m.chemin.to_string_lossy().to_string(),
        nom: crate::switch_nca::nom_a_montrer(&m.chemin),
        genre: genre.into(),
        taille: std::fs::metadata(&m.chemin).map(|x| x.len()).unwrap_or(0),
        licence: false,
        installe: utilise,
        version: None,
        raison: raison.map(String::from),
        cle,
        actif,
    }
}

/// Switch lancée par Ryubing (lot 6, ✅ Seb 16/11) : ce que Frogtend pose TOUT SEUL à chaque lancement (la mise à jour
/// la plus récente et les DLC du dossier du jeu, lus dans leurs en-têtes, d'après le choix de la maison), montré sans
/// rien écrire. `None` : pas un jeu Switch lancé par Ryubing.
async fn contenus_ryubing(app: &AppHandle, noyau: &Noyau, id: i64) -> Resultat<Option<ContenusDuJeu>> {
    let j = noyau.jeu_visible(id).await?;
    if !j.plateforme.eq_ignore_ascii_case("Nintendo Switch") {
        return Ok(None);
    }
    let Some((programme, _)) = emulateur_regle(app, &j.plateforme, id, None) else { return Ok(None) };
    if !crate::ryubing::est_ryubing(&programme) {
        return Ok(None);
    }
    let d = std::path::Path::new(&programme).parent().map(std::path::PathBuf::from).unwrap_or_default();
    let profil = profil_ouvert(noyau).await?.nom;
    let fichier = noyau.fichier_pour_l_emulateur(id, None).await.ok().flatten();
    let ailleurs = dossiers_switch(app, noyau, id).await;
    let donnees = noyau.dossier.clone();
    let r = tauri::async_runtime::spawn_blocking(move || apercu_ryubing(&donnees, &d, &profil, fichier.as_deref(), &ailleurs))
        .await
        .map_err(|_| Erreur::Disque("La lecture s'est arrêtée brutalement.".into()))?;
    Ok(Some(r))
}

/// L'aperçu des contenus Ryubing d'un jeu (`fichier` : celui que `-f` lancera) pour la personne `profil`. Rien n'est
/// écrit chez Ryubing. Jamais de numéro de version brut (v786432 n'est pas la « 1.5.0 » que montre la console) ;
/// chaque contenu porte sa clé et son interrupteur ; `raison` : une cause technique seulement.
fn apercu_ryubing(donnees: &std::path::Path, d: &std::path::Path, profil: &str, fichier: Option<&std::path::Path>, ailleurs: &[std::path::PathBuf]) -> ContenusDuJeu {
    let dire = |m: &str| ContenusDuJeu { etat: "ok".into(), systeme: "ryubing".into(), remarque: Some(m.into()), ..Default::default() };
    if !crate::ryubing::portable(d) {
        return dire("Ryubing n'a pas été installé par Frogtend : ses mises à jour et DLC se choisissent dans Ryubing.");
    }
    let racine = crate::ryubing::racine(d, profil);
    let Some(f) = fichier else { return dire("Le fichier de ce jeu est introuvable : aucune mise à jour ni DLC n'est utilisé.") };
    let Some(cle) = crate::ryubing::cle_entete(d, &racine) else {
        return dire(
            "Ryubing n'a pas encore les clés de la console. Lance ce jeu une fois (▶ Jouer) : Frogtend te proposera de les poser ; ses mises à jour et DLC seront ensuite utilisés tout seuls.",
        );
    };
    let Ok(Some(c)) = crate::switch_nca::contenus_pour_dans(f, &cle, ailleurs, Some(&crate::ryubing::cache_contenus(d))) else {
        return dire("Frogtend n'arrive pas à lire le fichier de ce jeu : aucune mise à jour ni DLC n'est utilisé.");
    };
    let (choix, abime) = crate::contenus_switch::choix_resolu(donnees, c.base, &crate::ryubing::ids_dlc(&c), c.complet());
    let a = crate::ryubing::poser_contenus(d, &racine, &c, &choix, false).unwrap_or_default();
    let nom = |m: &crate::switch_nca::ContenuLu| m.chemin.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let mut contenus: Vec<ContenuVu> = c
        .maj
        .iter()
        .map(|m| {
            let utilise = choix.maj && a.maj.is_some();
            let raison = match () {
                _ if !choix.maj || utilise => None,
                _ if a.maj_gardee.is_some() => Some("pas utilisée : une mise à jour plus récente est choisie dans Ryubing"),
                _ => Some("Ryubing ne la prend pas pour l'instant"),
            };
            contenu_switch_vu(m, "maj", Some(crate::contenus_switch::CLE_MAJ.into()), choix.maj, utilise, raison)
        })
        .collect();
    contenus.extend(c.majs_anciennes.iter().map(|m| contenu_switch_vu(m, "maj", None, false, false, Some("pas utilisée : une plus récente est là"))));
    contenus.extend(c.dlc.iter().map(|m| {
        let id = cle_du_contenu(m);
        let actif = id.as_deref().is_some_and(|i| choix.dlc_actif(i));
        let utilise = a.dlc.contains(&nom(m));
        let raison = (actif && !utilise).then_some("Ryubing ne le prend pas pour l'instant");
        contenu_switch_vu(m, "dlc", id, actif, utilise, raison)
    }));
    contenus.extend(contenus_nsz_vus(&c));
    // Ce qui est à dire : les problèmes (jamais jetés), le dossier (hors fichiers illisibles, montrés à part).
    let mut remarques: Vec<String> = Vec::new();
    remarques.extend(remarque_inaccessibles(&c));
    if abime.is_some() {
        remarques.push(crate::contenus_switch::ABIME_SUR_LA_FICHE.into());
    }
    if let Some(g) = &a.maj_gardee {
        remarques.push(format!("La mise à jour « {g} », plus récente, est choisie dans Ryubing : Frogtend la garde."));
    }
    remarques.extend(a.erreurs.iter().cloned());
    remarques.extend(crate::ryubing::remarques_du_dossier(&crate::switch_nca::ContenusDuJeu { illisibles: vec![], ..c.clone() }));
    ContenusDuJeu {
        etat: "ok".into(),
        systeme: "ryubing".into(),
        titre_id: Some(format!("{:016X}", c.base)),
        contenus,
        ignores: c.illisibles,
        remarque: (!remarques.is_empty()).then(|| {
            // Une phrase par ligne, chacune avec sa majuscule (l'interface en fait un paragraphe par ligne).
            let majuscule = |s: &String| {
                let mut c = s.chars();
                c.next().map(|p| p.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
            };
            remarques.iter().map(majuscule).collect::<Vec<_>>().join("\n")
        }),
    }
}

/// Les sources de la migration des retraits d'avant (✅ Seb 17/11) : chaque Ryubing réglé pour la Switch (une racine
/// par personne, `Profils\*`) et chaque Eden réglé.
fn sources_de_migration(app: &AppHandle) -> crate::contenus_switch::Sources {
    let reglages = app.store("pc.json").ok().and_then(|s| s.get("reglages")).unwrap_or(Value::Null);
    let mut s = crate::contenus_switch::Sources::default();
    for e in crate::choix_emulateur::du_systeme(&reglages, "Nintendo Switch").0 {
        let p = std::path::PathBuf::from(&e.programme);
        let Some(d) = p.parent().map(std::path::PathBuf::from) else { continue };
        if crate::ryubing::est_ryubing(&e.programme) {
            for r in std::fs::read_dir(d.join("Profils")).into_iter().flatten().flatten() {
                if r.file_type().is_ok_and(|t| t.is_dir()) {
                    s.ryubing.push(r.path());
                }
            }
        } else if crate::eden::est_eden(&e.programme) && crate::eden::id_de(&e.programme) == "eden" {
            s.eden.push(d);
        }
    }
    s.ryubing.sort();
    s.ryubing.dedup();
    s.eden.sort();
    s.eden.dedup();
    s
}

/// La migration, une seule fois (marque `_migre`) ; un échec est noté au journal, sans rien empêcher.
async fn assurer_migration_switch(app: &AppHandle, noyau: &Noyau) {
    let donnees = noyau.dossier.clone();
    if matches!(crate::contenus_switch::lire(&donnees), crate::contenus_switch::Lecture::Lu(c) if c.migre) {
        return;
    }
    let s = sources_de_migration(app);
    let r = tauri::async_runtime::spawn_blocking(move || crate::contenus_switch::migrer_depuis(&donnees, &s)).await;
    match r {
        Ok(Ok(Some(n))) => noyau.journaliser(&format!("mises à jour et DLC Switch : {n} retrait(s) d'avant repris comme « éteint »")),
        Ok(Ok(None)) => {}
        Ok(Err(e)) => noyau.journaliser(&format!("mises à jour et DLC Switch : retraits d'avant non repris ({e})")),
        Err(_) => noyau.journaliser("mises à jour et DLC Switch : la reprise des retraits d'avant s'est arrêtée brutalement"),
    }
}

/// Changer un choix de contenus Switch est-il permis ? Jamais pendant une partie ou son lancement (`partie`), ni pendant
/// qu'un émulateur est ouvert seul (`seul` : son nom) : il réécrirait ses réglages par-dessus.
fn choix_switch_permis(partie: bool, seul: Option<String>) -> Resultat<()> {
    if partie {
        return Err(Erreur::Refus("Une partie est en cours : tu pourras changer ces choix quand elle sera finie.".into()));
    }
    if let Some(nom) = seul {
        return Err(Erreur::Refus(format!("{nom} est ouvert : ferme-le d'abord, puis change ces choix.")));
    }
    Ok(())
}

/// Allume ou éteint un contenu Switch d'un jeu (`cle` : « maj » ou l'identifiant d'un DLC, lu sur la fiche) pour toute
/// la maison, avec Eden comme avec Ryubing (✅ Seb 17/11). Refusé pendant une partie, un lancement ou un émulateur ouvert
/// seul (il réécrirait ses réglages par-dessus). Rend vrai si un contenu utilisé vient d'être éteint : les parties de
/// ce jeu seront mises à l'abri avant sa prochaine partie.
#[tauri::command]
pub async fn contenus_switch_choisir(app: AppHandle, noyau: State<'_, Noyau>, id_jeu: i64, cle: String, actif: bool) -> Resultat<bool> {
    profil_ouvert(&noyau).await?;
    choix_switch_permis(app.state::<crate::menu_jeu::MenuJeu>().partie().is_some() || noyau.lancement_en_preparation(), noyau.emulateur_seul_nom())?;
    assurer_migration_switch(&app, &noyau).await;
    // La clé doit être celle d'un contenu trouvé pour CE jeu (jamais une clé venue d'ailleurs).
    let vu = contenus_du_jeu(app.clone(), noyau.clone(), id_jeu).await?;
    let base = vu
        .titre_id
        .as_deref()
        .filter(|_| matches!(vu.systeme.as_str(), "eden" | "ryubing"))
        .and_then(|t| u64::from_str_radix(t, 16).ok())
        .ok_or_else(|| Erreur::Refus("Les mises à jour et DLC de ce jeu ne se lisent pas : rien n'est changé.".into()))?;
    if !vu.contenus.iter().any(|c| c.cle.as_deref() == Some(cle.as_str())) {
        return Err(Erreur::Refus("Ce contenu n'est plus trouvé pour ce jeu : relis la fiche.".into()));
    }
    // Revérifié juste avant d'écrire : la lecture de la fiche a pu être longue (une partie a pu commencer entre-temps).
    choix_switch_permis(app.state::<crate::menu_jeu::MenuJeu>().partie().is_some() || noyau.lancement_en_preparation(), noyau.emulateur_seul_nom())?;
    let donnees = noyau.dossier.clone();
    let c = cle.clone();
    let eteint = tauri::async_runtime::spawn_blocking(move || crate::contenus_switch::choisir(&donnees, base, &c, actif))
        .await
        .map_err(|_| Erreur::Disque("L'enregistrement s'est arrêté brutalement.".into()))??;
    noyau.journaliser(&format!(
        "mises à jour et DLC Switch du jeu {id_jeu} : {} {}{}",
        if cle == crate::contenus_switch::CLE_MAJ { "mise à jour" } else { "DLC" },
        if actif { "allumé(e)" } else { "éteint(e)" },
        if eteint { " ; parties à mettre à l'abri avant la prochaine partie" } else { "" }
    ));
    Ok(eteint)
}

// Les contenus additionnels (DLC, avatars…) d'un jeu, disponibles sur ce PC, et ce qui est déjà installé pour ce profil.
#[tauri::command]
pub async fn contenus_du_jeu(app: AppHandle, noyau: State<'_, Noyau>, id: i64) -> Resultat<ContenusDuJeu> {
    // Wii U : un .wua contient déjà ses mises à jour et DLC (Cemu les lit) : on les montre, rien à installer.
    // Lu à part : un `if let` garderait le verrou du registre pendant les attentes qui suivent.
    let jeu = Some(noyau.jeu_visible(id).await?);
    let est_switch = jeu.as_ref().is_some_and(|j| j.plateforme.eq_ignore_ascii_case("Nintendo Switch"));
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
            remarque: None,
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
                    raison: None,
                    cle: None,
                    actif: true,
                })
                .collect(),
        });
    }
    // Switch (✅ Seb 17/11) : les retraits faits avant dans les émulateurs, repris une fois comme « éteints ».
    if est_switch {
        assurer_migration_switch(&app, &noyau).await;
    }
    // Ryubing (lot 6) : posés tout seuls au lancement d'après le choix de la maison, montrés avec leurs interrupteurs.
    if let Some(r) = contenus_ryubing(&app, &noyau, id).await? {
        return Ok(r);
    }
    // Eden (✅ Seb 16-17/11) : le dossier du jeu donné tout seul au lancement, les choix de la maison en interrupteurs.
    match contenus_eden(&app, &noyau, id).await {
        Ok(Some(r)) => return Ok(r),
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
        remarque: None,
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
                raison: None,
                cle: None,
                actif: true,
            })
            .collect(),
    })
}

// --- Lot 7 (✅ Seb 17/11) : envoyer à Firehouse les mises à jour et DLC Switch qu'il n'a pas (envois_switch.rs). Détecté
// à l'ouverture du panneau 📦, en tâche de fond (jamais dans le lancement) ; envoyé après l'accord chiffré de la personne
// sur la liste qu'elle a vue (ou tout seul si elle l'a choisi), un fichier à la fois, en pause pendant une partie. ---

/// Contrat 1.47 : les noms FRANÇAIS des mises à jour et DLC Switch d'un jeu (`nom_fr` de Firehouse, sinon son nom
/// d'origine), par clé d'interrupteur (« maj », « dlc:… ») et par fichier (`id` du contenu). Lu en tâche de fond par le
/// panneau 📦 (il s'affiche d'abord avec les noms des fichiers) ; hors ligne, la dernière correspondance reçue par CE
/// profil. Rien de connu : des listes vides (l'interface garde les noms des fichiers).
#[tauri::command]
pub async fn contenus_switch_noms(app: AppHandle, noyau: State<'_, Noyau>, id: i64) -> Resultat<crate::noms_switch::NomsContenus> {
    profil_ouvert(&noyau).await?;
    let Some((c, _)) = contenus_switch_lus(&app, &noyau, id).await else { return Ok(Default::default()) };
    let s = noyau.session().await?;
    let s2 = s.clone();
    // Gardé par la session, sous son verrou : jamais ce qu'un ancien jeton a reçu.
    let garder = move |f: &std::path::Path, o: &[u8]| s2.garder_fichier(f, o).map(|_| ());
    let connus = crate::noms_switch::lire(&s.source, &s.dossier, c.base, &garder).await?;
    // Le profil a changé pendant la lecture (le cache hors ligne était celui de l'ANCIEN profil) : rien n'est rendu.
    if s.est_perimee() {
        return Err(Erreur::Perimee("Le profil a changé pendant la lecture des noms : rien n'est montré.".into()));
    }
    Ok(crate::noms_switch::noms(&connus, &c))
}

/// Les mises à jour et DLC Switch d'un jeu lus dans leurs en-têtes, et la clé qui les a lus (celle de l'émulateur
/// réglé : Ryubing de la personne, ou Eden) — la même lecture que le panneau (le cache évite de tout relire). `None` :
/// pas un jeu Switch, pas d'émulateur utilisable, pas de clés ou jeu illisible.
async fn contenus_switch_lus(app: &AppHandle, noyau: &Noyau, id: i64) -> Option<(crate::switch_nca::ContenusDuJeu, crate::switch_nca::CleEntete)> {
    let j = noyau.jeu_visible(id).await.ok()?;
    if !j.plateforme.eq_ignore_ascii_case("Nintendo Switch") {
        return None;
    }
    let (programme, _) = emulateur_regle(app, &j.plateforme, id, None)?;
    let fichier = noyau.fichier_pour_l_emulateur(id, None).await.ok().flatten()?;
    let ailleurs = dossiers_switch(app, noyau, id).await;
    let p = std::path::PathBuf::from(&programme);
    let d = p.parent().map(std::path::PathBuf::from)?;
    if crate::ryubing::est_ryubing(&programme) {
        let profil = profil_ouvert(noyau).await.ok()?.nom;
        return tauri::async_runtime::spawn_blocking(move || {
            if !crate::ryubing::portable(&d) {
                return None;
            }
            let racine = crate::ryubing::racine(&d, &profil);
            let cle = crate::ryubing::cle_entete(&d, &racine)?;
            let c = crate::switch_nca::contenus_pour_dans(&fichier, &cle, &ailleurs, Some(&crate::ryubing::cache_contenus(&d))).ok().flatten()?;
            Some((c, cle))
        })
        .await
        .ok()
        .flatten();
    }
    if p.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("eden.exe")) {
        return tauri::async_runtime::spawn_blocking(move || {
            let cle = crate::contenus::cles_eden(&d).and_then(|f| crate::switch_nca::cle_entete(&f))?;
            match crate::eden::lire_contenus(&d, &fichier, &ailleurs) {
                crate::eden::LectureContenus::Lus(c) => Some((c, cle)),
                _ => None,
            }
        })
        .await
        .ok()
        .flatten();
    }
    None
}

/// Le débit d'envoi est-il plafonné à la moitié du débit mesuré (réglage du PC `envoisFirehouse.debit`, la moitié par
/// défaut ; « sans_limite » sinon) ?
fn envois_plafonnes(app: &AppHandle) -> bool {
    let r = app.store("pc.json").ok().and_then(|s| s.get("reglages")).unwrap_or(Value::Null);
    r["envoisFirehouse"]["debit"].as_str() != Some("sans_limite")
}

/// Le compte Firehouse retenu pour une session (`username#grade` de `/moi`), s'il est connu.
fn compte_de(s: &crate::noyau::Session) -> Option<String> {
    s.verrou().meta("compte").ok().flatten()
}

/// Ce que l'envoi demande à l'application (envois_switch::Rythme) : pause pendant une partie, un lancement ou un
/// émulateur ouvert seul (la même garde que les interrupteurs des contenus) ; arrêt si le profil change ou si la
/// personne a demandé « ✋ Arrêter l'envoi ».
struct RythmeApp {
    app: AppHandle,
    session: std::sync::Arc<crate::noyau::Session>,
}

impl crate::envois_switch::Rythme for RythmeApp {
    fn occupe(&self) -> bool {
        let n = self.app.state::<Noyau>();
        self.app.state::<crate::menu_jeu::MenuJeu>().partie().is_some() || n.lancement_en_preparation() || n.emulateur_seul_nom().is_some()
    }
    fn arrete(&self) -> bool {
        self.session.est_perimee() || self.app.state::<crate::envois_switch::Travail>().annuler.load(std::sync::atomic::Ordering::SeqCst)
    }
    fn plafonne(&self) -> bool {
        envois_plafonnes(&self.app)
    }
    async fn dormir(&self, d: std::time::Duration) {
        tokio::time::sleep(d).await;
    }
    fn progres(&self, p: &crate::envois_switch::Progres) {
        let p = crate::envois_switch::Progres { profil: self.session.profil.id.clone(), ..p.clone() };
        *self.app.state::<crate::envois_switch::Travail>().progres.lock().unwrap_or_else(|e| e.into_inner()) = Some(p);
    }
    fn journaliser(&self, m: &str) {
        self.app.state::<Noyau>().journaliser(m);
    }
    fn compte(&self) -> Option<String> {
        compte_de(&self.session)
    }
}

/// Lance le travail des envois du profil ouvert en arrière-plan (un seul à la fois : s'il tourne déjà, pour un autre
/// profil peut-être, il sera relancé à sa fin ; rien si la file est vide).
fn lancer_envois_switch(app: &AppHandle) {
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        let noyau = app2.state::<Noyau>();
        let Ok(s) = noyau.session().await else { return };
        let travail = app2.state::<crate::envois_switch::Travail>();
        let profil = s.profil.id.clone();
        if !travail.commencer(&profil) {
            return;
        }
        let fin = match &s.source {
            crate::source::Source::Firehouse(c) => {
                let r = RythmeApp { app: app2.clone(), session: s.clone() };
                crate::envois_switch::travailler(c, &s.dossier, crate::envois_switch::Bornes::default(), &r).await
            }
            crate::source::Source::Simulee => None,
        };
        if let Some(crate::envois_switch::Arret::Motif(m)) = &fin {
            noyau.journaliser(&format!("envois Switch arrêtés : {m}"));
        }
        if travail.finir(&profil, fin) {
            lancer_envois_switch(&app2);
        }
    });
}

/// L'examen d'un jeu : ce que Firehouse a (route 1), et ce que ce PC a de plus, envoyable. Rend aussi ce qu'il faut pour
/// mettre les candidats en file (base, fiche possible).
async fn examiner_envois(app: &AppHandle, noyau: &Noyau, id: i64) -> Resultat<(crate::envois_switch::Examen, Vec<crate::envois_switch::Candidat>, u64, Option<i64>)> {
    use crate::envois_switch::Examen;
    let rien = |etat: &str, motif: Option<String>, suivi: crate::envois_switch::Suivi| Ok((Examen { etat: etat.into(), motif, suivi, ..Default::default() }, vec![], 0, None));
    let s = noyau.session().await?;
    let travail = app.state::<crate::envois_switch::Travail>();
    let suivi = crate::envois_switch::suivi_du_jeu(&s.dossier, &s.profil.id, id, &travail)?;
    let client = match &s.source {
        crate::source::Source::Firehouse(c) => c,
        crate::source::Source::Simulee => return rien("simule", None, suivi),
    };
    let file = crate::envois_switch::lire(&s.dossier)?;
    let compte = compte_de(&s);
    if let Some(m) = file.interdit_pour(compte.as_deref()) {
        return rien("interdit", Some(m), suivi);
    }
    if file.interdit.is_some() {
        // Un autre compte, ou un autre grade : le refus d'avant ne vaut plus.
        crate::envois_switch::modifier(&s.dossier, |l| {
            l.interdit = None;
            l.interdit_compte = None;
        })?;
    }
    let j = noyau.jeu_visible(id).await?;
    let Some((c, cle)) = contenus_switch_lus(app, noyau, id).await else { return rien("rien", None, suivi) };
    if c.maj.is_none() && c.dlc.is_empty() && c.compresses.is_empty() {
        return rien("rien", None, suivi);
    }
    // La route 1 ne coûte rien (contrat 1.42 ; pas d'ETag, besoin 52 réponse 8).
    let servi: crate::envois_switch::CeQueFirehouseA = match client.obtenir_json(&crate::envois_switch::route_contenus(c.base)).await {
        Ok(v) => v,
        Err(Erreur::JetonRefuse(m)) => return Err(Erreur::JetonRefuse(m)),
        // Une route inconnue (Firehouse d'avant le contrat 1.42) : rien n'est proposé.
        Err(Erreur::Introuvable(_)) => return rien("rien", None, suivi),
        Err(Erreur::Refus(_)) => return rien("interdit", Some("Ton compte Firehouse ne permet pas d'envoyer des mises à jour ou des DLC.".into()), suivi),
        Err(_) => return rien("injoignable", None, suivi),
    };
    // Ce qui VIENT de Firehouse (les fichiers de la fiche installée : même chemin, même taille) ne repart jamais.
    let importe = crate::import_local::est_importe(&j);
    let venus: Vec<(std::path::PathBuf, u64)> = if importe { vec![] } else { j.fichiers.iter().map(|f| (std::path::Path::new(&j.dossier).join(&f.nom), f.taille)).collect() };
    let file2 = file.clone();
    let c2 = c.clone();
    let tri = tauri::async_runtime::spawn_blocking(move || crate::envois_switch::trier(&c2, &cle, &servi, &venus, &|p: &std::path::Path| file2.contient(p), importe))
        .await
        .map_err(|_| Erreur::Disque("La lecture s'est arrêtée brutalement.".into()))?;
    let examen = Examen::depuis(&tri, file.debit, envois_plafonnes(app), suivi);
    Ok((examen, tri.a_envoyer, c.base, (!importe).then_some(j.id)))
}

/// À l'ouverture du panneau 📦 d'un jeu Switch : ce que Firehouse n'a pas (combien, combien de Go, combien de temps).
/// Relance aussi les envois restés en file (un redémarrage, une connexion revenue).
#[tauri::command]
pub async fn envois_switch_examiner(app: AppHandle, noyau: State<'_, Noyau>, id: i64) -> Resultat<crate::envois_switch::Examen> {
    let (examen, _, _, _) = examiner_envois(&app, &noyau, id).await?;
    if examen.etat == "ok" && examen.suivi.envois.iter().any(|e| !matches!(e.etat.as_str(), "range" | "deja_la" | "refuse" | "arrete")) {
        lancer_envois_switch(&app);
    }
    Ok(examen)
}

/// La personne a dit oui (ou a choisi « tout seul ») à la liste qu'elle a VUE (`noms`, `total`) : les contenus que
/// Firehouse n'a pas sont relus à l'instant (jamais une liste venue de l'interface) ; s'ils diffèrent de ce qu'elle a vu,
/// refus (« la liste a changé » : l'interface redemande). Sinon, ils vont dans la file du profil et l'envoi commence en
/// arrière-plan. Rend combien.
#[tauri::command]
pub async fn envois_switch_accepter(app: AppHandle, noyau: State<'_, Noyau>, id: i64, noms: Vec<String>, total: u64, auto: bool) -> Resultat<usize> {
    let (examen, candidats, base, media) = examiner_envois(&app, &noyau, id).await?;
    if examen.etat != "ok" {
        return Err(Erreur::Refus(examen.motif.unwrap_or_else(|| "Rien ne peut être envoyé à Firehouse pour l'instant.".into())));
    }
    if !crate::envois_switch::meme_liste(&candidats, &noms, total) {
        return Err(Erreur::Conflit("La liste des contenus à envoyer a changé : regarde-la de nouveau avant de dire oui.".into()));
    }
    let s = noyau.session().await?;
    let dossier = s.dossier.clone();
    let n = tauri::async_runtime::spawn_blocking(move || crate::envois_switch::ajouter(&dossier, id, base, media, &candidats))
        .await
        .map_err(|_| Erreur::Disque("L'enregistrement s'est arrêté brutalement.".into()))??;
    noyau.journaliser(&format!("envois Switch du jeu {id} : {n} contenu(s) mis en file{}", if auto { " (envoi automatique choisi par la personne)" } else { " après l'accord de la personne" }));
    lancer_envois_switch(&app);
    Ok(n)
}

/// « ✋ Arrêter l'envoi » : les envois non finis de ce jeu sortent de la file (reproposables plus tard) ; si l'envoi en
/// cours est à ce jeu, il s'arrête au prochain morceau. Rend combien.
#[tauri::command]
pub async fn envois_switch_annuler(app: AppHandle, noyau: State<'_, Noyau>, id: i64) -> Resultat<usize> {
    let s = noyau.session().await?;
    let travail = app.state::<crate::envois_switch::Travail>();
    // D'abord le drapeau (l'envoi en cours s'arrête), puis la file : il ne reprend jamais un fichier retiré.
    if travail.en_cours_pour(&s.profil.id, id) {
        travail.annuler.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    let dossier = s.dossier.clone();
    let n = tauri::async_runtime::spawn_blocking(move || crate::envois_switch::annuler(&dossier, id))
        .await
        .map_err(|_| Erreur::Disque("L'enregistrement s'est arrêté brutalement.".into()))??;
    noyau.journaliser(&format!("envois Switch du jeu {id} : {n} envoi(s) arrêté(s) par la personne"));
    Ok(n)
}

/// Où en sont les envois de ce jeu (rien n'est demandé à Firehouse) : relu par la fiche pendant un envoi.
#[tauri::command]
pub async fn envois_switch_suivi(app: AppHandle, noyau: State<'_, Noyau>, id: i64) -> Resultat<crate::envois_switch::Suivi> {
    let s = noyau.session().await?;
    crate::envois_switch::suivi_du_jeu(&s.dossier, &s.profil.id, id, &app.state::<crate::envois_switch::Travail>())
}

// --- Les mods Switch servis par Firehouse (contrat 1.44, ✅ Seb 18/11 ; mods_switch.rs). Lus à l'ouverture du panneau,
// téléchargés après l'accord chiffré de la personne (ou tout seuls pour une nouvelle révision si elle l'a choisi), un
// interrupteur par mod pour toute la maison, posés dans l'émulateur au LANCEMENT seulement. ---

/// Le title id de base d'un jeu Switch et la version brute de la mise à jour que l'émulateur réglé utilisera (même
/// lecture que le panneau 📦, rien n'est écrit chez l'émulateur). `None` : pas d'émulateur lisible, pas de clés.
async fn base_et_maj_switch(app: &AppHandle, noyau: &Noyau, id: i64) -> Option<(u64, Option<u32>)> {
    let j = noyau.jeu_visible(id).await.ok()?;
    let (programme, _) = emulateur_regle(app, &j.plateforme, id, None)?;
    let fichier = noyau.fichier_pour_l_emulateur(id, None).await.ok().flatten()?;
    let ailleurs = dossiers_switch(app, noyau, id).await;
    let d = std::path::Path::new(&programme).parent().map(std::path::PathBuf::from)?;
    let donnees = noyau.dossier.clone();
    if crate::ryubing::est_ryubing(&programme) {
        let profil = profil_ouvert(noyau).await.ok()?.nom;
        return tauri::async_runtime::spawn_blocking(move || {
            if !crate::ryubing::portable(&d) {
                return None;
            }
            let racine = crate::ryubing::racine(&d, &profil);
            let cle = crate::ryubing::cle_entete(&d, &racine)?;
            let c = crate::switch_nca::contenus_pour_dans(&fichier, &cle, &ailleurs, Some(&crate::ryubing::cache_contenus(&d))).ok().flatten()?;
            let (choix, _) = crate::contenus_switch::choix_resolu(&donnees, c.base, &crate::ryubing::ids_dlc(&c), c.complet());
            let a = crate::ryubing::poser_contenus(&d, &racine, &c, &choix, false).ok();
            Some((c.base, a.and_then(|a| a.maj_utilisee)))
        })
        .await
        .ok()
        .flatten();
    }
    if d.join("eden.exe").is_file() || std::path::Path::new(&programme).file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("eden.exe")) {
        return tauri::async_runtime::spawn_blocking(move || match crate::eden::lire_contenus(&d, &fichier, &ailleurs) {
            crate::eden::LectureContenus::Lus(c) => {
                let (choix, _) = crate::contenus_switch::choix_resolu(&donnees, c.base, &crate::ryubing::ids_dlc(&c), c.complet());
                Some((c.base, crate::eden::maj_utilisee(&c, &choix)))
            }
            _ => None,
        })
        .await
        .ok()
        .flatten();
    }
    None
}

/// Ce qu'il faut pour les mods d'un jeu Switch : son title id de base, le dossier où ils sont rangés, la version de la
/// mise à jour utilisée (`None` : inconnue). `None` : pas un jeu Switch installé lisible.
struct ContexteMods {
    base: u64,
    stock: std::path::PathBuf,
    utilisee: Option<u32>,
}

async fn contexte_mods_switch(app: &AppHandle, noyau: &Noyau, id: i64) -> Resultat<Option<ContexteMods>> {
    let j = noyau.jeu_visible(id).await?;
    if !j.plateforme.eq_ignore_ascii_case("Nintendo Switch") || j.dossier.is_empty() || !std::path::Path::new(&j.dossier).is_absolute() {
        return Ok(None);
    }
    let frogtend = crate::locale::dossier_frogtend(&j);
    let (base, utilisee) = match base_et_maj_switch(app, noyau, id).await {
        Some(x) => x,
        None => {
            // Sans clés ni émulateur lisible : le title id du nom (ou du ticket) du fichier ; la version reste inconnue.
            let f = noyau.fichier_pour_l_emulateur(id, None).await.ok().flatten();
            let t = tauri::async_runtime::spawn_blocking(move || f.as_deref().and_then(crate::contenus::id_switch)).await.ok().flatten();
            match t {
                Some(t) => (crate::switch_nca::base(t), None),
                None => return Ok(None),
            }
        }
    };
    Ok(Some(ContexteMods { base, stock: crate::mods_switch::dossier_stock(&frogtend, base), utilisee }))
}

/// Les mods Switch d'un jeu (à l'ouverture du panneau) : ce que Firehouse sert (ou la dernière liste lue, hors ligne)
/// et ce qui est rangé sur ce PC, avec pour chacun son état, sa compatibilité avec la mise à jour choisie et son
/// interrupteur.
#[tauri::command]
pub async fn mods_switch_lister(app: AppHandle, noyau: State<'_, Noyau>, id: i64) -> Resultat<crate::mods_switch::ListeVue> {
    profil_ouvert(&noyau).await?;
    let Some(cx) = contexte_mods_switch(&app, &noyau, id).await? else {
        return Ok(crate::mods_switch::ListeVue { etat: "rien".into(), ..Default::default() });
    };
    let s = noyau.session().await?;
    let (s2, s3) = (s.clone(), s.clone());
    // Le cache s'écrit par la session, sous son verrou (jamais après un changement de jeton).
    let garder = move |f: &std::path::Path, o: &[u8]| s3.garder_fichier(f, o).map(|_| ());
    let lecture = crate::mods_switch::lire(&s.source, &s.dossier, cx.base, &move || s2.est_perimee(), &garder).await?;
    if lecture.liste.ecartes > 0 {
        noyau.journaliser(&format!("mods Switch du jeu {id} : {} mod(s) de Firehouse écarté(s) (clé hors contrat ou en double)", lecture.liste.ecartes));
    }
    let (stock, liste, frais) = (cx.stock.clone(), lecture.liste.clone(), lecture.etat == "ok");
    let (rangs, derniere) = tauri::async_runtime::spawn_blocking(move || {
        // Le dernier avis de Firehouse sur une révision rangée (compatibilité, conflits…) : retenu pour la pose.
        if frais {
            crate::mods_switch::rafraichir(&stock, &liste);
        }
        (crate::mods_switch::ranges(&stock), crate::mods_switch::derniere_pose(&stock))
    })
    .await
    .unwrap_or_default();
    let choix = crate::contenus_switch::charger(&noyau.dossier).0.jeu(cx.base);
    let mut vue = crate::mods_switch::liste_vue(&lecture, &rangs, &choix, cx.utilisee, cx.base);
    // Les raisons qui nomment un mod réservé : seulement si CE profil le voit.
    vue.derniere_pose = derniere.map(|d| d.pour_le_profil(Some(&lecture.liste)));
    Ok(vue)
}

/// Un mod que la personne a VU avant de dire oui (sa clé, sa révision, sa taille, l'empreinte de son archive).
#[derive(Debug, Clone, Deserialize)]
pub struct ModAccorde {
    pub cle: String,
    pub revision: u64,
    pub taille: u64,
    pub sha256: String,
}

/// La garde des mods : refusé pendant une partie, un lancement, un émulateur ouvert seul.
fn mods_switch_permis(app: &AppHandle, noyau: &Noyau) -> Resultat<()> {
    choix_switch_permis(app.state::<crate::menu_jeu::MenuJeu>().partie().is_some() || noyau.lancement_en_preparation(), noyau.emulateur_seul_nom())
}

/// Les mods VUS par la personne sont-ils exactement ceux que Firehouse sert à l'instant (clé, révision, taille,
/// empreinte ; total exact) ? Rend les mods servis correspondants, sinon « la liste a changé ».
pub fn mods_accordes(servis: &[crate::mods_switch::ModServi], vus: &[ModAccorde], total: u64) -> Resultat<Vec<crate::mods_switch::ModServi>> {
    let change = || Erreur::Conflit("La liste des mods a changé : regarde-la de nouveau avant de dire oui.".into());
    let mut a_faire: Vec<crate::mods_switch::ModServi> = Vec::new();
    let mut somme: u64 = 0;
    for v in vus {
        let m = servis
            .iter()
            .find(|m| m.cle == v.cle && m.revision == v.revision && m.fichier.taille == v.taille && m.fichier.sha256.eq_ignore_ascii_case(&v.sha256))
            .ok_or_else(change)?;
        if a_faire.iter().any(|x| x.cle == m.cle) {
            return Err(change());
        }
        // Une adresse qui n'est pas celle de Firehouse : refusée avant tout appel.
        crate::mods_switch::route_archive(&m.fichier.url, &m.cle)?;
        somme = somme.checked_add(m.fichier.taille).ok_or_else(change)?;
        a_faire.push(m.clone());
    }
    if somme != total {
        return Err(change());
    }
    Ok(a_faire)
}

/// La personne a dit oui à la liste qu'elle a VUE (`cles_vues` : clé, révision, taille, sha256 ; `total` en octets) — ou
/// a choisi « mettre à jour tout seul » (`auto`, noté au journal ; le réglage vit dans son profil, `modsSwitch.auto`).
/// La liste est relue chez Firehouse à l'instant (jamais une liste venue de l'interface) : si elle diffère, refus
/// (« la liste a changé »). Sinon les mods sont téléchargés puis rangés en arrière-plan, un à la fois (suivi par
/// `mods_switch_suivi`) ; un seul téléchargement par jeu, tous profils confondus (le rangement est commun). Jamais
/// pendant une partie. Rend combien.
#[tauri::command]
pub async fn mods_switch_telecharger(app: AppHandle, noyau: State<'_, Noyau>, id: i64, cles_vues: Vec<ModAccorde>, total: u64, auto: bool) -> Resultat<usize> {
    profil_ouvert(&noyau).await?;
    mods_switch_permis(&app, &noyau)?;
    let cx = contexte_mods_switch(&app, &noyau, id).await?.ok_or_else(|| Erreur::Refus("Les mods de ce jeu ne se lisent pas : rien n'est téléchargé.".into()))?;
    let s = noyau.session().await?;
    let (s2, s3) = (s.clone(), s.clone());
    // Le cache s'écrit par la session, sous son verrou (jamais après un changement de jeton).
    let garder = move |f: &std::path::Path, o: &[u8]| s3.garder_fichier(f, o).map(|_| ());
    let lecture = crate::mods_switch::lire(&s.source, &s.dossier, cx.base, &move || s2.est_perimee(), &garder).await?;
    if lecture.etat != "ok" {
        return Err(Erreur::Reseau("Firehouse n'est pas joignable : les mods ne peuvent pas être téléchargés pour l'instant.".into()));
    }
    if cles_vues.is_empty() {
        return Err(Erreur::Refus("Aucun mod n'est choisi.".into()));
    }
    let a_faire = mods_accordes(&lecture.liste.mods, &cles_vues, total)?;
    let travail = app.state::<crate::mods_switch::Travail>();
    if !travail.commencer(id, a_faire.len()) {
        return Err(Erreur::Refus("Des mods de ce jeu sont déjà en train d'arriver.".into()));
    }
    let n = a_faire.len();
    noyau.journaliser(&format!(
        "mods Switch du jeu {id} : {n} mod(s) à télécharger ({total} octets){}",
        if auto { " (mise à jour automatique choisie par la personne)" } else { " après l'accord de la personne" }
    ));
    let app2 = app.clone();
    let servie = lecture.liste.version_servie.clone();
    let (base, stock) = (cx.base, cx.stock.clone());
    tauri::async_runtime::spawn(async move {
        let noyau = app2.state::<Noyau>();
        let travail = app2.state::<crate::mods_switch::Travail>();
        let occupe = || app2.state::<crate::menu_jeu::MenuJeu>().partie().is_some() || noyau.lancement_en_preparation() || noyau.emulateur_seul_nom().is_some();
        for m in a_faire {
            if occupe() {
                travail.modifier(id, |x| x.arret = Some("une partie a commencé : les mods qui restent seront proposés de nouveau ensuite".into()));
                break;
            }
            if s.est_perimee() {
                travail.modifier(id, |x| x.arret = Some("le profil a changé".into()));
                break;
            }
            // Le suivi est commun à tous les profils, le journal aussi : un mod réservé (« adulte ») n'y est jamais nommé.
            let deja = crate::mods_switch::ranges(&stock);
            let nom_dit = crate::mods_switch::nom_a_dire(&m, &deja);
            travail.modifier(id, |x| {
                x.en_cours = Some(nom_dit.clone());
                x.recus = 0;
                x.total = m.fichier.taille;
            });
            let app3 = app2.clone();
            let progres = move |recus: u64, _total: Option<u64>| {
                app3.state::<crate::mods_switch::Travail>().modifier(id, |x| x.recus = recus);
            };
            // Avant l'échange des dossiers : le changement de mod est noté (abri des parties), et un mod NOUVEAU en
            // conflit avec un mod allumé arrive éteint.
            let (donnees, stock2) = (noyau.dossier.clone(), stock.clone());
            let avant = Box::new(move |r: &crate::mods_switch::ManifesteMod| crate::mods_switch::noter_arrivee(&donnees, &stock2, base, r).map(|_| ()));
            match crate::mods_switch::telecharger_et_ranger(&s.source, &stock, &m, servie.as_ref(), &progres, avant).await {
                Ok(r) => {
                    travail.modifier(id, |x| x.faits += 1);
                    let eteint = !crate::contenus_switch::charger(&noyau.dossier).0.jeu(base).mod_actif(&r.cle);
                    noyau.journaliser(&format!(
                        "mods Switch du jeu {id} : « {} » (révision {}) installé{} ; parties à mettre à l'abri avant la prochaine partie",
                        nom_dit,
                        r.revision,
                        if eteint { ", éteint (un mod allumé ne va pas avec lui, ou il était éteint)" } else { "" }
                    ));
                }
                Err(e) => {
                    let jeton = matches!(e, Erreur::JetonRefuse(_));
                    noyau.journaliser(&format!("mods Switch du jeu {id} : « {nom_dit} » pas installé ({e})"));
                    travail.modifier(id, |x| x.erreurs.push(format!("{nom_dit} : {e}")));
                    if jeton {
                        break;
                    }
                }
            }
        }
        travail.modifier(id, |x| {
            x.actif = false;
            x.en_cours = None;
        });
    });
    Ok(n)
}

/// Où en sont les téléchargements de mods de ce jeu (rien n'est demandé à Firehouse ; le même pour tous les profils).
#[tauri::command]
pub async fn mods_switch_suivi(app: AppHandle, noyau: State<'_, Noyau>, id: i64) -> Resultat<crate::mods_switch::Suivi> {
    profil_ouvert(&noyau).await?;
    Ok(app.state::<crate::mods_switch::Travail>().suivi(id))
}

/// Allume ou éteint un mod rangé (pour toute la maison, Eden comme Ryubing). Allumer un mod en conflit avec un mod
/// allumé est refusé avec un motif lisible, sauf si la personne a accepté d'éteindre l'autre (`eteindre` : sa clé) —
/// les deux changent alors d'un coup. Refusé pendant une partie ou un émulateur ouvert seul. Rend vrai si quelque
/// chose a changé (les parties de ce jeu seront mises à l'abri avant sa prochaine partie).
#[tauri::command]
pub async fn mods_switch_choisir(app: AppHandle, noyau: State<'_, Noyau>, id: i64, cle: String, allume: bool, eteindre: Option<String>) -> Resultat<bool> {
    profil_ouvert(&noyau).await?;
    mods_switch_permis(&app, &noyau)?;
    let cx = contexte_mods_switch(&app, &noyau, id).await?.ok_or_else(|| Erreur::Refus("Les mods de ce jeu ne se lisent pas : rien n'est changé.".into()))?;
    // Seuls les mods que CE profil peut voir (un mod réservé absent de sa dernière liste n'existe pas pour lui).
    let (stock, session) = (cx.stock.clone(), noyau.session().await?.dossier.clone());
    let base_cx = cx.base;
    let (tous, rangs) = tauri::async_runtime::spawn_blocking(move || {
        let tous = crate::mods_switch::ranges(&stock);
        let vus = crate::mods_switch::visibles(tous.clone(), crate::mods_switch::liste_du_profil(&session, base_cx).as_ref());
        (tous, vus)
    })
    .await
    .unwrap_or_default();
    let nom = |c: &str| rangs.iter().find(|m| m.cle == c).map(|m| m.nom.clone());
    // Le journal est commun à la maison : un mod réservé n'y est jamais nommé.
    let nom_journal = |c: &str| rangs.iter().find(|m| m.cle == c).map(crate::mods_switch::nom_range_a_dire);
    let Some(nom_a) = nom(&cle) else { return Err(Erreur::Refus("Ce mod n'est pas installé sur ce PC : relis la fiche.".into())) };
    let eteindre = eteindre.filter(|b| *b != cle);
    if let Some(b) = &eteindre {
        if nom(b).is_none() {
            return Err(Erreur::Refus("L'autre mod n'est pas installé sur ce PC : relis la fiche.".into()));
        }
    }
    let donnees = noyau.dossier.clone();
    let choix = crate::contenus_switch::charger(&donnees).0.jeu(cx.base);
    if allume {
        // Sur TOUS les mods rangés (un mod qu'il ne voit pas peut être allumé pour la maison) ; motif neutre alors.
        let vus: Vec<String> = rangs.iter().map(|m| m.cle.clone()).collect();
        if let Some(m) = crate::mods_switch::conflit_pour(&tous, &vus, &choix, &cle, eteindre.as_deref()) {
            return Err(Erreur::Refus(m));
        }
    }
    let mut changements = vec![(cle.clone(), allume)];
    if let Some(b) = &eteindre {
        changements.push((b.clone(), false));
    }
    // Revérifié juste avant d'écrire (une partie a pu commencer entre-temps).
    mods_switch_permis(&app, &noyau)?;
    let base = cx.base;
    let change = tauri::async_runtime::spawn_blocking(move || crate::contenus_switch::choisir_mods(&donnees, base, &changements))
        .await
        .map_err(|_| Erreur::Disque("L'enregistrement s'est arrêté brutalement.".into()))??;
    let _ = nom_a;
    noyau.journaliser(&format!(
        "mods Switch du jeu {id} : « {} » {}{}{}",
        nom_journal(&cle).unwrap_or_default(),
        if allume { "allumé" } else { "éteint" },
        eteindre.as_deref().and_then(nom_journal).map(|b| format!(", « {b} » éteint")).unwrap_or_default(),
        if change { " ; parties à mettre à l'abri avant la prochaine partie" } else { "" }
    ));
    Ok(change)
}

/// Retire un mod rangé de ce PC (à la demande de la personne) : seulement s'il est inchangé ; ses copies dans les
/// émulateurs partent à la partie suivante. Refusé pendant une partie ou un téléchargement de mods de ce jeu.
#[tauri::command]
pub async fn mods_switch_retirer(app: AppHandle, noyau: State<'_, Noyau>, id: i64, cle: String) -> Resultat<()> {
    profil_ouvert(&noyau).await?;
    mods_switch_permis(&app, &noyau)?;
    // Tout téléchargement de mods de ce jeu, quel que soit le profil (le rangement est commun à la maison).
    if app.state::<crate::mods_switch::Travail>().actif(id) {
        return Err(Erreur::Refus("Des mods de ce jeu sont en train d'arriver : attends la fin.".into()));
    }
    let cx = contexte_mods_switch(&app, &noyau, id).await?.ok_or_else(|| Erreur::Refus("Les mods de ce jeu ne se lisent pas : rien n'est retiré.".into()))?;
    let (stock, donnees, base, c) = (cx.stock.clone(), noyau.dossier.clone(), cx.base, cle.clone());
    let session = noyau.session().await?.dossier.clone();
    tauri::async_runtime::spawn_blocking(move || -> Resultat<()> {
        // Un mod réservé que CE profil ne voit pas : « pas installé » (jamais retiré par lui).
        let vus = crate::mods_switch::visibles(crate::mods_switch::ranges(&stock), crate::mods_switch::liste_du_profil(&session, base).as_ref());
        if !vus.iter().any(|m| m.cle == c) {
            return Err(Erreur::Introuvable("Ce mod n'est pas installé.".into()));
        }
        crate::mods_switch::retirer(&stock, &c)?;
        crate::contenus_switch::noter_changement_de_mod(&donnees, base, Some(&c))
    })
    .await
    .map_err(|_| Erreur::Disque("Le retrait s'est arrêté brutalement.".into()))??;
    noyau.journaliser(&format!("mods Switch du jeu {id} : mod {cle} retiré de ce PC à la demande de la personne ; parties à mettre à l'abri avant la prochaine partie"));
    Ok(())
}

/// L'émulateur où poser les mods au lancement.
enum ModsPour {
    Ryubing { dossier: std::path::PathBuf, racine: std::path::PathBuf },
    Eden { dossier: std::path::PathBuf },
}

/// Au LANCEMENT, quand les mises à jour et DLC n'ont pas pu être préparés : les mods sont quand même remis d'accord
/// (un mod éteint ne reste jamais chargé), avec une version de mise à jour INCONNUE et le title id tiré du nom (ou du
/// ticket) du fichier. Sans title id : rien n'est posé, la raison va sur la fiche — et si un mod d'un autre profil
/// reste chargé dans l'émulateur (n'importe quel rangement du jeu), la partie ne part pas (`Err`).
async fn poser_mods_switch_sans_contenus(noyau: &Noyau, id: i64, pour: ModsPour, fichier: std::path::PathBuf) -> Resultat<()> {
    let base = tauri::async_runtime::spawn_blocking(move || crate::mods_switch::base_du_fichier(&fichier)).await.ok().flatten();
    if let Some(b) = base {
        return poser_mods_switch(noyau, id, pour, b, None).await;
    }
    let Ok(j) = noyau.jeu_visible(id).await else { return Ok(()) };
    if j.dossier.is_empty() || !std::path::Path::new(&j.dossier).is_absolute() {
        return Ok(());
    }
    let nom = match pour {
        ModsPour::Ryubing { .. } => "Ryubing",
        ModsPour::Eden { .. } => "Eden",
    };
    let frogtend = crate::locale::dossier_frogtend(&j);
    let n = crate::mods_switch::noter_raison_partout(&frogtend, nom, crate::mods_switch::RAISON_SANS_TITRE);
    if n > 0 {
        noyau.journaliser(&format!("{nom} : mods Switch du jeu {id} pas remis d'accord (jeu pas reconnu)"));
    }
    // Le contrôle, sur chaque rangement du jeu (on ne sait pas lequel est le bon).
    let session = noyau.session().await.ok().map(|s| s.dossier.clone());
    for (b, stock) in crate::mods_switch::rangements_du_jeu(&frogtend) {
        let liste = session.as_deref().and_then(|d| crate::mods_switch::liste_du_profil(d, b));
        let refus = match &pour {
            ModsPour::Ryubing { racine, .. } => crate::mods_switch::verifier_masques_ryubing(racine, b, &stock, liste.as_ref()),
            ModsPour::Eden { dossier } => crate::mods_switch::verifier_masques_eden(dossier, b, &stock, liste.as_ref()),
        };
        if let Some(m) = refus {
            noyau.journaliser(&format!("{nom} : jeu {id} pas lancé : un mod d'un autre profil reste en place"));
            return Err(Erreur::Refus(m));
        }
    }
    Ok(())
}

/// Au LANCEMENT (après l'abri et `poser_contenus`) : met les mods de Frogtend d'accord dans l'émulateur. Un échec est
/// noté au journal et le jeu se lance avec la pose d'avant — SAUF si un mod qu'un autre profil a reçu (« adulte »…)
/// reste chargé pour le profil qui lance : alors `Err` (message neutre), la partie ne part pas (brief § 2). Jamais de
/// retrait forcé sans abri.
async fn poser_mods_switch(noyau: &Noyau, id: i64, pour: ModsPour, base: u64, utilisee: Option<u32>) -> Resultat<()> {
    let Ok(j) = noyau.jeu_visible(id).await else { return Ok(()) };
    if j.dossier.is_empty() || !std::path::Path::new(&j.dossier).is_absolute() {
        return Ok(());
    }
    let stock = crate::mods_switch::dossier_stock(&crate::locale::dossier_frogtend(&j), base);
    let donnees = noyau.dossier.clone();
    // La dernière liste reçue par le profil QUI LANCE (son cache) : un mod réservé qu'elle ne contient pas n'est ni
    // posé ni nommé (brief § 2). Sans session : aucune liste, aucun mod réservé.
    let session = noyau.session().await.ok().map(|s| s.dossier.clone());
    let liste = session.as_deref().and_then(|d| crate::mods_switch::liste_du_profil(d, base));
    let (nom, r) = match pour {
        ModsPour::Ryubing { dossier, racine } => {
            let (s, l) = (stock.clone(), liste.clone());
            let r = tauri::async_runtime::spawn_blocking(move || {
                let r = crate::mods_switch::poser_ryubing_pour(&donnees, &dossier, &racine, base, &s, utilisee, l.as_ref());
                (r, crate::mods_switch::verifier_masques_ryubing(&racine, base, &s, l.as_ref()))
            })
            .await;
            ("Ryubing", r)
        }
        ModsPour::Eden { dossier } => {
            let (s, l) = (stock.clone(), liste.clone());
            let r = tauri::async_runtime::spawn_blocking(move || {
                let ouvert = !crate::lancement::processus_du_dossier(&dossier).is_empty();
                let r = crate::mods_switch::poser_eden_pour(&donnees, &dossier, base, &s, utilisee, ouvert, l.as_ref());
                (r, crate::mods_switch::verifier_masques_eden(&dossier, base, &s, l.as_ref()))
            })
            .await;
            ("Eden", r)
        }
    };
    let Ok((r, refus)) = r else {
        // La pose s'est arrêtée brutalement : on ne sait pas ce qui est en place. S'il existe un mod que ce profil ne
        // voit pas, dans le doute, la partie ne part pas.
        noyau.journaliser(&format!("{nom} : la pose des mods Switch du jeu {id} s'est arrêtée brutalement"));
        let masque = crate::mods_switch::ranges(&stock).len() > crate::mods_switch::visibles(crate::mods_switch::ranges(&stock), liste.as_ref()).len();
        return if masque { Err(Erreur::Refus(crate::mods_switch::message_masque_en_place(nom))) } else { Ok(()) };
    };
    match r {
        Ok(b) if b == crate::mods_switch::BilanMods::default() => {}
        Ok(b) => noyau.journaliser(&format!("{nom} : mods Switch du jeu {id} : {}", b.resume())),
        Err(e) => noyau.journaliser(&format!("{nom} : mods Switch du jeu {id} non posés ({e})")),
    }
    if let Some(m) = refus {
        noyau.journaliser(&format!("{nom} : jeu {id} pas lancé : un mod d'un autre profil reste en place"));
        return Err(Erreur::Refus(m));
    }
    Ok(())
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
    /// `relancer` : la personne a demandé « Relancer » (le même jeu repart aussitôt, ✅ Seb 15/11).
    Fin { jeu: i64, secondes: u64, relancer: bool },
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
    image: Option<String>,
    fps: Option<bool>,
) -> Resultat<()> {
    // Un émulateur ouvert seul (✅ Seb 17/11) : aucune partie pendant ce temps (il réécrirait ses réglages en quittant).
    // Et tant que ce lancement se prépare (jusqu'au retour, la partie est alors celle du menu en jeu), aucun émulateur
    // ne s'ouvre seul.
    let _lancement = noyau.commencer_lancement()?;
    // Mods Switch : jamais de partie pendant qu'un mod de CE jeu arrive (tous profils confondus) — la pose lirait un
    // rangement en train de changer. Refusé (pas d'attente : un téléchargement peut durer), avec un message clair.
    if app.state::<crate::mods_switch::Travail>().actif(id) {
        return Err(Erreur::Refus(crate::mods_switch::PARTIE_PENDANT_TELECHARGEMENT.into()));
    }
    let jeu = noyau.registre().jeu(id)?;
    // Un jeu d'une boutique : lancé par elle, jamais par un émulateur (même si un émulateur est réglé pour « Windows »).
    let de_boutique = jeu.as_ref().and_then(|j| j.installation.as_ref()).is_some_and(|i| i.boutique.is_some());
    let (plateforme, titre) = jeu.map(|j| (j.plateforme, j.titre)).unwrap_or_default();
    // Pour l'index des captures (la galerie montre le titre même si le jeu quitte le PC).
    let (titre_captures, plateforme_captures) = (titre.clone(), plateforme.clone());
    let mut id_emulateur = None;
    let mut dossier_emulateur = None;
    // RetroArch : le nom du cœur (le dossier où il cherche les triches du jeu). RPCS3 : sa ligne de lancement.
    let mut coeur_ra: Option<String> = None;
    // RetroArch : l'effet d'image posé au lancement, ou celui de la personne (lot 2).
    let mut effet_lance: Option<String> = None;
    let mut effet_de_la_personne = false;
    let mut ligne_rpcs3: Option<String> = None;
    // BigPEmu : l'effet, les triches et les touches préparés avant le lancement (pour le menu en jeu).
    let mut lancement_bp: Option<crate::bigpemu::Lancement> = None;
    // Ryubing : l'effet d'image posé avant le lancement (`None` : pas posé, le menu ne propose pas d'effets).
    let mut effet_ryubing: Option<crate::ryubing::EffetPose> = None;
    // Les images par seconde (✅ Seb 08/11, choix par personne) : ce qui sera affiché, et de quoi le rendre après.
    let mut etat_fps = crate::images_par_seconde::EtatLancement::default();
    let emulateur = match (!de_boutique).then(|| emulateur_regle(&app, &plateforme, id, emulateur.as_deref())).flatten() {
        Some((programme, ligne)) => {
            // Son émulateur se met à jour en ce moment : on attend la fin (jamais un jeu lancé pendant le remplacement).
            if en_installation(std::path::Path::new(&programme)) {
                return Err(Erreur::Refus("Son émulateur est en train de se mettre à jour : attends la fin (la barre d'avancement), puis relance le jeu.".into()));
            }
            dossier_emulateur = std::path::Path::new(&programme).parent().map(|d| d.to_string_lossy().to_string());
            let nom = std::path::Path::new(&programme).file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
            id_emulateur = crate::emulateurs::CATALOGUE.iter().find(|f| nom.starts_with(f.programme)).map(|f| f.id.to_string());
            if id_emulateur.as_deref() == Some("rpcs3") {
                ligne_rpcs3 = Some(ligne.clone());
            }
            if id_emulateur.as_deref() == Some("retroarch") {
                coeur_ra = std::path::Path::new(&programme)
                    .parent()
                    .zip(crate::emulateurs::coeur_de(&ligne))
                    .and_then(|(d, c)| crate::emulateurs::nom_du_coeur(d, &c));
            }
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
            let (prog, mut ligne_c) = preparer_emulateur(&app, &noyau, &plateforme, &programme, &ligne, &commandes.unwrap_or_default()).await?;
            // Le compteur d'images par seconde de la personne, posé avant le lancement ; un échec n'empêche pas de jouer.
            if let (Some(ide), Some(d), Some(profil)) = (id_emulateur.clone(), dossier_emulateur.clone(), profil_ouvert(&noyau).await.ok().map(|p| p.nom)) {
                // RPCS3 : le numéro du jeu (son réglage propre passe devant).
                let ps3 = if ide == "rpcs3" {
                    noyau.fichier_pour_l_emulateur(id, version.as_deref()).await.ok().flatten().and_then(|f| crate::contenus::titre_id_ps3(&f))
                } else {
                    None
                };
                let ligne_fps = ligne_c.clone();
                let r = tauri::async_runtime::spawn_blocking(move || {
                    crate::images_par_seconde::au_lancement_de_la_partie(&ide, std::path::Path::new(&d), &profil, fps, ps3.as_deref(), &ligne_fps)
                })
                .await
                .map_err(|_| Erreur::Disque("La préparation du compteur d'images s'est arrêtée brutalement.".into()))?;
                if let Some(m) = &r.erreur {
                    noyau.journaliser(&format!("images par seconde du jeu {id} : non posées ({m})"));
                }
                if let Some(m) = &r.raison {
                    noyau.journaliser(&format!("images par seconde du jeu {id} : {m}"));
                }
                // Dolphin : le choix sur la ligne de commande (`-C …`, jamais enregistré par lui), devant le reste.
                if !r.arguments.is_empty() {
                    let a: Vec<String> = r.arguments.iter().map(|a| format!("\"{a}\"")).collect();
                    ligne_c = format!("{} {ligne_c}", a.join(" "));
                }
                etat_fps = r;
            }
            // RetroArch : l'effet d'image choisi pour ce jeu ou sa console (lot 4a/4b), par `--set-shader` (rien n'est
            // recopié dans retroarch.cfg) ; seul `video_shader_enable` est écrit, dans le fichier du profil (RetroArch
            // ne charge aucun effet sans lui). Un refus est dit, le jeu se lance quand même.
            if let (Some("retroarch"), Some(choix)) = (id_emulateur.as_deref(), image.as_deref()) {
                let choix_effet = choix.to_string();
                let fichier = noyau.fichier_pour_l_emulateur(id, version.as_deref()).await.ok().flatten();
                let profil = profil_ouvert(&noyau).await?.nom;
                let (d, c, choix) = (std::path::PathBuf::from(dossier_emulateur.clone().unwrap_or_default()), coeur_ra.clone(), choix.to_string());
                let r = tauri::async_runtime::spawn_blocking(move || -> Result<crate::emulateurs::ChoixEffet, String> {
                    if !d.is_absolute() {
                        return Err("le dossier de RetroArch est inconnu".into());
                    }
                    let noms: Vec<String> = fichier
                        .iter()
                        .flat_map(|f| [f.file_stem(), f.parent().and_then(|p| p.file_name())])
                        .flatten()
                        .map(|s| s.to_string_lossy().to_string())
                        .collect();
                    let refs: Vec<&str> = noms.iter().map(|s| s.as_str()).collect();
                    let s = crate::emulateurs::shader_retroarch(&d, &choix, c.as_deref(), &refs)?;
                    if matches!(s, crate::emulateurs::ChoixEffet::Effet(_)) {
                        // Ajouté à la fin (jamais le fichier réécrit en entier : une écriture coupée ne le tronque pas).
                        use std::io::Write;
                        let cfg = crate::emulateurs_profils::dossier_du_profil(&d, &profil).join("frogtend.cfg");
                        std::fs::OpenOptions::new()
                            .append(true)
                            .open(&cfg)
                            .and_then(|mut f| f.write_all(b"video_shader_enable = \"true\"\n"))
                            .map_err(|e| format!("les réglages du profil n'ont pas pu être complétés ({e})"))?;
                    }
                    Ok(s)
                })
                .await
                .map_err(|_| Erreur::Disque("La préparation de l'effet d'image s'est arrêtée brutalement.".into()))?;
                match r {
                    Ok(crate::emulateurs::ChoixEffet::Effet(s)) => {
                        ligne_c = format!("\"--set-shader={}\" {ligne_c}", s.to_string_lossy());
                        effet_lance = Some(choix_effet.clone());
                    }
                    Ok(crate::emulateurs::ChoixEffet::Aucun) => {}
                    Ok(crate::emulateurs::ChoixEffet::DeLaPersonne) => {
                        effet_de_la_personne = true;
                        noyau.journaliser(&format!("effet d'image du jeu {id} : celui réglé dans RetroArch est gardé"));
                    }
                    Err(m) => {
                        noyau.journaliser(&format!("effet d'image du jeu {id} : {m}"));
                        let _ = app.emit("effet-image", serde_json::json!({ "jeu": id, "message": m }));
                    }
                }
            }
            // BigPEmu (✅ Seb 11/11 : tout ce qu'il propose, dans notre menu) : l'effet d'écran choisi, les triches
            // choisies dans le menu (le jeu à l'abri d'abord s'il en a de cochées : un échec, rien ne se lance).
            // Sans profil lisible : rien de préparé, le jeu se lance quand même.
            let profil_bp = if id_emulateur.as_deref() == Some("bigpemu") { profil_ouvert(&noyau).await.ok().map(|p| p.nom) } else { None };
            if let (Some("bigpemu"), Some(d), Some(profil)) = (id_emulateur.as_deref(), dossier_emulateur.clone(), profil_bp) {
                let fichier = noyau.fichier_pour_l_emulateur(id, version.as_deref()).await.ok().flatten();
                let choix = image.clone();
                let quand = secondes_maintenant();
                let l = tauri::async_runtime::spawn_blocking(move || {
                    let d = std::path::PathBuf::from(d);
                    let p = crate::emulateurs_profils::dossier_du_profil(&d, &profil);
                    crate::bigpemu::preparer_partie(&d, &p, fichier.as_deref(), choix.as_deref(), quand)
                })
                .await
                .map_err(|_| Erreur::Disque("La préparation de BigPEmu s'est arrêtée brutalement.".into()))??;
                if let Some(m) = &l.erreur_effet {
                    noyau.journaliser(&format!("effet d'écran du jeu {id} : {m}"));
                }
                if let Some(m) = &l.erreur_triches {
                    noyau.journaliser(&format!("BigPEmu : triches du jeu {id} non posées ({m})"));
                }
                if let Some(m) = &l.erreur_etats {
                    noyau.journaliser(&format!("BigPEmu : sauvegarde rapide coupée non remise en place, pas d'emplacements ({m})"));
                }
                if l.abri_fait {
                    noyau.journaliser(&format!("BigPEmu : triches cochées pour le jeu {id}, ses fichiers mis à l'abri avant"));
                }
                lancement_bp = Some(l);
            }
            // Ryubing (lot 5) : l'effet d'image choisi pour ce jeu ou sa console, posé maintenant (Ryubing fermé, lu à son
            // démarrage) ; un refus est noté, le jeu se lance quand même avec ce qu'avait la personne.
            let profil_ryu = if id_emulateur.as_deref() == Some("ryubing") { profil_ouvert(&noyau).await.ok().map(|p| p.nom) } else { None };
            if let (Some("ryubing"), Some(d), Some(profil)) = (id_emulateur.as_deref(), dossier_emulateur.clone(), profil_ryu) {
                let choix = image.clone();
                // Le numéro du jeu (un .nsp : son nom ou son ticket) : ses réglages propres à lui passent devant.
                let fichier = noyau.fichier_pour_l_emulateur(id, version.as_deref()).await.ok().flatten();
                let r = tauri::async_runtime::spawn_blocking(move || {
                    let d = std::path::PathBuf::from(d);
                    let titre = fichier.as_deref().and_then(crate::contenus::id_switch);
                    crate::ryubing::poser_effet(&d, &crate::ryubing::racine(&d, &profil), titre, choix.as_deref())
                })
                .await
                .map_err(|_| Erreur::Disque("La préparation de Ryubing s'est arrêtée brutalement.".into()))?;
                match r {
                    Ok(e) => {
                        if e.de_la_personne {
                            effet_de_la_personne = true;
                            noyau.journaliser(&format!("effet d'image du jeu {id} : celui réglé dans Ryubing est gardé"));
                        }
                        effet_ryubing = Some(e);
                    }
                    Err(e) => {
                        noyau.journaliser(&format!("Ryubing : effet d'image du jeu {id} non posé ({e:?})"));
                        // Dit à la personne (comme pour RetroArch) seulement s'il y avait un effet à poser.
                        if image.as_deref().is_some_and(|c| c.starts_with(crate::ryubing::PREFIXE_EFFET)) {
                            let m = match e {
                                Erreur::Refus(m) => m,
                                _ => "l'effet d'image n'a pas pu être posé ; le jeu se lance sans lui.".to_string(),
                            };
                            let _ = app.emit("effet-image", serde_json::json!({ "jeu": id, "message": m }));
                        }
                    }
                }
            }
            // Ryubing (lot 6, ✅ Seb 16/11 : automatique, sans accord) : la mise à jour la plus récente et les DLC du
            // dossier du jeu, reconnus à leurs en-têtes, posés dans `games\<base>\` de la personne (Ryubing fermé), d'après
            // le choix de la maison (✅ Seb 17/11). Un échec est noté au journal ; le jeu se lance toujours, sauf si les
            // parties à mettre à l'abri ne l'ont pas été.
            if let (Some("ryubing"), Some(d), Ok(p)) = (id_emulateur.as_deref(), dossier_emulateur.clone(), profil_ouvert(&noyau).await) {
                if let Some(fichier) = noyau.fichier_pour_l_emulateur(id, version.as_deref()).await.ok().flatten() {
                    // ✅ Seb 17/11 : le choix de la maison (contenus-switch.json) décide ; les retraits d'avant, repris une fois.
                    assurer_migration_switch(&app, &noyau).await;
                    let ailleurs = dossiers_switch(&app, &noyau, id).await;
                    let donnees = noyau.dossier.clone();
                    let d_mods = std::path::PathBuf::from(&d);
                    let racine_mods = crate::ryubing::racine(&d_mods, &p.nom);
                    let fichier_mods = fichier.clone();
                    let r = tauri::async_runtime::spawn_blocking(move || {
                        let d = std::path::PathBuf::from(d);
                        crate::ryubing::preparer_contenus_dans(&donnees, &d, &crate::ryubing::racine(&d, &p.nom), &fichier, &ailleurs)
                    })
                    .await
                    .map_err(|_| Erreur::Disque("La préparation de Ryubing s'est arrêtée brutalement.".into()))?;
                    match r {
                        // Les parties n'ont pas pu être mises à l'abri : rien n'a changé, et la partie ne se lance pas.
                        Ok(Some(c)) if c.abri_impossible.is_some() => {
                            let m = c.abri_impossible.unwrap_or_default();
                            noyau.journaliser(&format!("Ryubing : parties du jeu {id} pas mises à l'abri ({m}) : partie pas lancée"));
                            return Err(Erreur::Refus(format!(
                                "Frogtend n'a pas pu mettre tes parties de ce jeu à l'abri ({m}), comme il le fait après avoir éteint une mise à jour ou un DLC : la partie ne se lance pas, rien n'a changé."
                            )));
                        }
                        Ok(Some(c)) => {
                            for n in &c.notes {
                                noyau.journaliser(&format!("Ryubing : jeu {id} : {n}"));
                            }
                            noyau.journaliser(&format!("Ryubing : mises à jour et DLC du jeu {id} : {}", c.resume()));
                            // Les mods Switch (✅ Seb 18/11) : APRÈS l'abri et les mises à jour, d'après la mise à jour
                            // réellement utilisée.
                            poser_mods_switch(&noyau, id, ModsPour::Ryubing { dossier: d_mods, racine: racine_mods }, c.base, c.maj_utilisee).await?;
                        }
                        // Les mises à jour et DLC ne se préparent pas (clés, choix pas repris, jeu illisible) : les mods
                        // sont QUAND MÊME remis d'accord (un mod éteint ne doit pas rester chargé), version inconnue.
                        Ok(None) => poser_mods_switch_sans_contenus(&noyau, id, ModsPour::Ryubing { dossier: d_mods, racine: racine_mods }, fichier_mods).await?,
                        Err(e) => {
                            noyau.journaliser(&format!("Ryubing : mises à jour et DLC du jeu {id} non posés ({e})"));
                            poser_mods_switch_sans_contenus(&noyau, id, ModsPour::Ryubing { dossier: d_mods, racine: racine_mods }, fichier_mods).await?;
                        }
                    }
                }
            }
            // Eden (✅ Seb 16/11 : automatique partout, sans accord) : chaque dossier qui contient la mise à jour ou un
            // DLC de CE jeu (dossier du jeu, sous-dossiers, installation, emplacements de la Switch ; lus dans leurs
            // en-têtes avec les clés d'Eden), dans sa liste de contenus externes, et `[DisabledAddOns]` d'accord avec le
            // choix de la maison (✅ Seb 17/11) — Eden fermé (il réécrit ses réglages en quittant), fichier mis à l'abri,
            // preuve notée. Un échec est noté au journal ; le jeu se lance toujours, sauf si les parties à mettre à
            // l'abri ne l'ont pas été.
            if let (Some("eden"), Some(d)) = (id_emulateur.as_deref(), dossier_emulateur.clone()) {
                if let Some(fichier) = noyau.fichier_pour_l_emulateur(id, version.as_deref()).await.ok().flatten() {
                    assurer_migration_switch(&app, &noyau).await;
                    let donnees = noyau.dossier.clone();
                    let ailleurs = dossiers_switch(&app, &noyau, id).await;
                    let d_eden = d.clone();
                    let fichier_mods = fichier.clone();
                    let r = tauri::async_runtime::spawn_blocking(move || {
                        crate::eden::preparer_contenus(&donnees, std::path::Path::new(&d), &fichier, &ailleurs, id, &|e| !crate::lancement::processus_du_dossier(e).is_empty())
                    })
                    .await
                    .map_err(|_| Erreur::Disque("La préparation d'Eden s'est arrêtée brutalement.".into()))?;
                    match r {
                        // Les parties n'ont pas pu être mises à l'abri : rien n'a changé, et la partie ne se lance pas.
                        Ok(Some(c)) if c.abri_impossible.is_some() => {
                            let m = c.abri_impossible.unwrap_or_default();
                            noyau.journaliser(&format!("Eden : parties du jeu {id} pas mises à l'abri ({m}) : partie pas lancée"));
                            return Err(Erreur::Refus(format!(
                                "Frogtend n'a pas pu mettre tes parties de ce jeu à l'abri ({m}), comme il le fait après avoir éteint une mise à jour ou un DLC : la partie ne se lance pas, rien n'a changé."
                            )));
                        }
                        Ok(Some(c)) => {
                            for n in &c.notes {
                                noyau.journaliser(&format!("Eden : contenus du jeu {id} : {n}"));
                            }
                            noyau.journaliser(&format!("Eden : mises à jour et DLC du jeu {id} : {}", c.resume()));
                            // Les mods Switch (✅ Seb 18/11) : après l'abri et les mises à jour (une mise à jour coupée
                            // dans Eden : le jeu d'origine).
                            let utilisee = if c.maj_coupee { Some(0) } else { c.maj_utilisee };
                            poser_mods_switch(&noyau, id, ModsPour::Eden { dossier: std::path::PathBuf::from(&d_eden) }, c.base, utilisee).await?;
                        }
                        // Mises à jour et DLC pas préparés : les mods sont quand même remis d'accord, version inconnue
                        // (« Eden est ouvert » : `poser_eden` ne change rien et le dit).
                        Ok(None) => poser_mods_switch_sans_contenus(&noyau, id, ModsPour::Eden { dossier: std::path::PathBuf::from(&d_eden) }, fichier_mods).await?,
                        Err(e) => {
                            noyau.journaliser(&format!("Eden : mises à jour et DLC du jeu {id} non donnés ({e})"));
                            poser_mods_switch_sans_contenus(&noyau, id, ModsPour::Eden { dossier: std::path::PathBuf::from(&d_eden) }, fichier_mods).await?;
                        }
                    }
                }
            }
            Some((prog, ligne_c))
        }
        None => None,
    };
    let par_emulateur = emulateur.is_some();
    // Déjà en cours (un second clic pendant que Steam démarre le jeu, une partie, un téléchargement) : refusé, jamais
    // deux parties suivies du même jeu.
    if noyau.utilise(id) {
        return Err(Erreur::Refus("Ce jeu est déjà en train de se lancer ou de tourner.".into()));
    }
    // RetroArch, AVANT le lancement (il lit le `.cht` en chargeant le jeu) : où sont les parties, les triches du jeu
    // (lot 3) ; des triches restées cochées → la partie mise à l'abri d'abord (sinon rien ne se lance).
    let prep_ra = match (id_emulateur.as_deref(), dossier_emulateur.clone(), profil_ouvert(&noyau).await.ok()) {
        (Some("retroarch"), Some(d), Some(p)) => {
            let fichier = noyau.fichier_pour_l_emulateur(id, version.as_deref()).await.ok().flatten();
            let defaut = if version.is_some() { noyau.fichier_pour_l_emulateur(id, None).await.ok().flatten() } else { None };
            let coeur = coeur_ra.clone();
            let r = tauri::async_runtime::spawn_blocking(move || {
                let d = std::path::PathBuf::from(d);
                let profil = crate::emulateurs_profils::dossier_du_profil(&d, &p.nom);
                crate::menu_jeu::preparer_ra(&d, &profil, coeur.as_deref(), fichier.as_deref(), defaut.as_deref(), secondes_maintenant())
            })
            .await
            .map_err(|_| Erreur::Disque("La préparation des triches s'est arrêtée brutalement.".into()))?
            .map_err(Erreur::Refus)?;
            if let Some(s) = &r.cht_repris {
                noyau.journaliser(&format!("triches : {} repris pour la version lancée", s.display()));
            }
            if r.abri_fait {
                noyau.journaliser(&format!("triches : {} cochée(s) au lancement, partie mise à l'abri avant", r.triches.iter().filter(|t| t.active).count()));
            }
            Some(r)
        }
        _ => None,
    };
    // RPCS3, AVANT le lancement (fermé : il réécrirait ses réglages en quittant) : langue, clavier et zone de la console
    // pris de Windows une seule fois ; un jeu japonais se valide avec Rond (lot 4, ✅ Seb 30/10).
    if let (Some("rpcs3"), Some(d)) = (id_emulateur.as_deref(), dossier_emulateur.clone()) {
        let fichier = noyau.fichier_pour_l_emulateur(id, version.as_deref()).await.ok().flatten();
        let notes = noyau.dossier.join("langues-posees.json");
        let r = tauri::async_runtime::spawn_blocking(move || -> Resultat<Vec<String>> {
            let d = std::path::PathBuf::from(d);
            if !crate::lancement::processus_du_dossier(&d).is_empty() {
                return Ok(Vec::new());
            }
            let (langue, clavier) = crate::emulateurs_profils::langue_et_clavier_de_windows();
            let numero = fichier.as_deref().and_then(crate::contenus::titre_id_ps3);
            crate::emulateurs_profils::rpcs3_avant_la_partie(&d, &notes, langue, clavier, numero.as_deref())
        })
        .await
        .unwrap_or_else(|_| Ok(vec![]));
        match r {
            Ok(faits) => faits.iter().for_each(|f| noyau.journaliser(&format!("RPCS3 : {f}"))),
            Err(e) => noyau.journaliser(&format!("RPCS3 : langue non réglée ({e:?})")),
        }
    }
    // Les captures d'écran de la partie, rangées dans le dossier général (✅ Seb 04/11) : ce qui est déjà dans le
    // dossier de captures de l'émulateur est noté AVANT le lancement (jamais touché).
    // La famille d'Eden : ses raccourcis encore d'origine (capture, son, turbo, recommencer), lus AVANT le lancement
    // (au démarrage, Eden réécrit tout son fichier), sans rien écrire.
    let eden_d_origine = match (id_emulateur.as_deref(), dossier_emulateur.as_deref()) {
        (Some("eden" | "eglantine" | "lichii"), Some(d)) => crate::eden::actions_d_origine(std::path::Path::new(d)),
        // Ryubing : les touches en place dans le Config.json de la personne qui joue (lu après la préparation).
        (Some("ryubing"), Some(d)) => match profil_ouvert(&noyau).await {
            Ok(p) => crate::ryubing::actions_d_origine(std::path::Path::new(d), &crate::ryubing::racine(std::path::Path::new(d), &p.nom)),
            Err(_) => vec![],
        },
        _ => vec![],
    };
    let veilleur_captures = match (id_emulateur.as_deref(), dossier_emulateur.as_deref(), dossier_captures_regle(&app)) {
        (Some(e), Some(d), Some(choisi)) => {
            let d = std::path::PathBuf::from(d);
            let profil = profil_ouvert(&noyau).await.ok().map(|p| crate::emulateurs_profils::dossier_du_profil(&d, &p.nom));
            let fichier = noyau.fichier_pour_l_emulateur(id, version.as_deref()).await.ok().flatten();
            let systeme = noyau.plateformes(true).await.ok().and_then(|l| l.into_iter().find(|p| p.nom == plateforme)).and_then(|p| p.nom_affiche).unwrap_or_else(|| plateforme.clone());
            let (e, titre) = (e.to_string(), titre.clone());
            tauri::async_runtime::spawn_blocking(move || {
                let dossiers = crate::captures::dossiers_de(&e, &d, profil.as_deref(), fichier.as_deref());
                crate::captures::Veilleur::nouveau(Some(choisi), dossiers, &systeme, &titre)
            })
            .await
            .unwrap_or(Ok(None))
            .unwrap_or_else(|m| {
                noyau.journaliser(&format!("captures d'écran : {m}"));
                None
            })
            .map(|v| std::sync::Arc::new(std::sync::Mutex::new(v)))
        }
        _ => None,
    };
    // Ryubing : la racine de la personne (`-r`) existe encore, JUSTE avant de lancer — sinon il jouerait en silence sur
    // les données communes (AppDataManager.cs), avec les parties d'un autre.
    if let (Some("ryubing"), Some((_, ligne))) = (id_emulateur.as_deref(), emulateur.as_ref()) {
        crate::ryubing::verifier_la_racine(ligne)?;
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
    let famille_eden = id_emulateur.as_deref().is_some_and(crate::emulateurs::famille_eden);
    let nom_eden = id_emulateur.as_deref().and_then(crate::emulateurs::fiche).map(|f| f.nom).unwrap_or("Eden");
    let _ = app.emit("partie", EvenementPartie::Debut { jeu: id, carte_cedee });
    // Le menu en jeu : sa touche est active pendant la partie seulement.
    let dossier_jeu_pe = dossier_jeu.clone();
    // Figé pendant le menu : un émulateur sans pause réglable, ou Eden/Azahar dont la pause n'a pas pu être réglée.
    let mut figer_emulateur = match (id_emulateur.as_deref(), dossier_emulateur.as_deref()) {
        (Some(e), Some(d)) => crate::pilotage::fige_au_menu(e) || !crate::pilotage::pause_reglee(e, std::path::Path::new(d)),
        _ => false,
    };
    // Ryubing : sa pause est dans le Config.json de la personne qui joue (sa racine) ; un autre choix → figé au menu.
    if let (Some("ryubing"), Some(d)) = (id_emulateur.as_deref(), dossier_emulateur.as_deref()) {
        let racine = profil_ouvert(&noyau).await.ok().map(|p| crate::ryubing::racine(std::path::Path::new(d), &p.nom));
        figer_emulateur = !racine.is_some_and(|r| crate::ryubing::pause_reglee(&r));
    }
    // « Disque suivant » seulement pour une liste de disques (une cartouche n'en a qu'un).
    let plusieurs_disques = par_emulateur
        && noyau.fichier_pour_l_emulateur(id, version.as_deref()).await.ok().flatten().is_some_and(|f| crate::menu_jeu::liste_de_disques(&f));
    // RetroArch : où il range les sauvegardes rapides, et le nom du jeu lancé (pour les emplacements) — préparé avant.
    let (etats, nom_rom) = match prep_ra.as_ref().filter(|_| id_emulateur.as_deref() == Some("retroarch")) {
        Some(r) => (r.etats.as_ref().map(|e| e.to_string_lossy().to_string()), r.nom_rom.clone()),
        None => (None, None),
    };
    // Dolphin : ses raccourcis réglés sur une manette (gardés) ne reçoivent pas les touches de Frogtend.
    let raccourcis_clavier = if id_emulateur.as_deref() == Some("dolphin") {
        match (dossier_emulateur.as_deref(), profil_ouvert(&noyau).await.ok()) {
            (Some(d), Some(p)) => crate::pilotage::dolphin_raccourcis_au_clavier(&crate::emulateurs_profils::dossier_du_profil(std::path::Path::new(d), &p.nom).join("User")),
            _ => true,
        }
    } else if id_emulateur.as_deref() == Some("rpcs3") {
        // RPCS3 : ses raccourcis d'origine, et lancé SANS sa fenêtre principale (`--no-gui` : sinon Ctrl+S y veut dire
        // « Arrêter ») — sinon le menu ne lui envoie rien.
        ligne_rpcs3.as_deref().is_some_and(|l| l.contains("--no-gui"))
            && dossier_emulateur.as_deref().is_some_and(|d| crate::pilotage::rpcs3_raccourcis_d_origine(std::path::Path::new(d)))
    } else if id_emulateur.as_deref() == Some("bigpemu") {
        // BigPEmu : F13 / F14 / F15 en place chez la personne (pas avant son premier lancement).
        lancement_bp.as_ref().is_some_and(|l| l.raccourcis)
    } else {
        true
    };
    // RetroArch en mode « hardcore » des succès : ni sauvegarde rapide (il les refuserait sans le dire), ni triche
    // (il perdrait le hardcore). Ses triches (lot 3) : le `.cht` du jeu lu avant le lancement.
    let prep = prep_ra.filter(|_| id_emulateur.as_deref() == Some("retroarch")).unwrap_or_default();
    // RetroArch : les effets changeables en jeu (si son pilote vidéo les lit) et `video_shader_enable` d'avant (lot 2).
    let (effets_ra, shader_enable_avant) = match (id_emulateur.as_deref(), dossier_emulateur.clone()) {
        (Some("retroarch"), Some(d)) => tauri::async_runtime::spawn_blocking(move || {
            let d = std::path::PathBuf::from(d);
            let presents: Vec<String> = if crate::emulateurs::effets_impossibles(&d).is_none() {
                crate::emulateurs::EFFETS_IMAGE.iter().filter(|(_, _, c)| crate::emulateurs::fichier_effet(&d, c).is_some()).map(|(i, ..)| i.to_string()).collect()
            } else {
                vec![]
            };
            (presents, crate::emulateurs::reglage_retroarch(&d, "video_shader_enable"))
        })
        .await
        .unwrap_or_default(),
        _ => (vec![], None),
    };
    // RPCS3 : de quoi noter, à la fin, qu'une partie assez longue a fini la compilation (lot 5).
    let compile_rpcs3 = match (id_emulateur.as_deref(), dossier_emulateur.clone()) {
        (Some("rpcs3"), Some(d)) => noyau
            .fichier_pour_l_emulateur(id, version.as_deref())
            .await
            .ok()
            .flatten()
            .and_then(|f| crate::contenus::titre_id_ps3(&f).filter(|n| crate::emulateurs_profils::numero_ps3_sur(n)))
            .map(|n| (std::path::PathBuf::from(d), n)),
        _ => None,
    };
    // BigPEmu : ses effets et ses triches remplacent ceux de RetroArch dans le menu.
    let etats_bigpemu = lancement_bp.as_ref().and_then(|l| l.etats.clone());
    let reset_bigpemu = lancement_bp.as_ref().is_some_and(|l| l.reset);
    // Une triche cochée pour ce jeu (posée sans erreur) : BigPEmu montrera sa fenêtre de triches au démarrage.
    let fenetre_triches_bp = lancement_bp.as_ref().is_some_and(|l| l.fenetre_triches);
    let (effets_ra, effet_bp, triches, abri_triches, empreinte, cles_triches, dossier_profil_bp) = match lancement_bp {
        Some(l) => (
            l.effets,
            l.effet,
            l.triches.iter().map(|t| crate::menu_jeu::Triche { titre: t.titre.clone(), active: t.active }).collect(),
            l.abri_fait,
            l.empreinte,
            l.triches.into_iter().map(|t| t.cle).collect(),
            Some(l.dossier_profil.to_string_lossy().to_string()),
        ),
        // Ryubing : ses effets (des réglages de Ryubing) au prochain lancement, comme BigPEmu.
        None if id_emulateur.as_deref() == Some("ryubing") && effet_ryubing.is_some() => {
            (crate::ryubing::effets_presents(), effet_ryubing.clone().and_then(|e| e.effet), prep.triches, prep.abri_fait, None, vec![], None)
        }
        None => (effets_ra, None, prep.triches, prep.abri_fait, None, vec![], None),
    };
    let a_recharger = app.state::<crate::menu_jeu::MenuJeu>().prendre_charger(id, id_emulateur.as_deref());
    let numero = app.state::<crate::menu_jeu::MenuJeu>().commencer(crate::menu_jeu::PartieEnCours {
        au_lancement: None,
        etats_interdits: prep.etats_interdits,
        triches,
        triches_permises: !prep.etats_interdits,
        triche_ra: 0,
        parties: prep.parties.map(|d| d.to_string_lossy().to_string()),
        abri_triches,
        empreinte,
        dossier_profil: dossier_profil_bp,
        cles_triches,
        etats_bigpemu,
        message: None,
        coeur: coeur_ra.clone(),
        dossier_rom: prep.dossier_rom,
        effet_pose: effet_lance.is_some(),
        effet: effet_lance.or(effet_bp),
        effet_de_la_personne,
        effets: effets_ra,
        shader_enable_avant,
        refus_etat: false,
        fps: etat_fps.affiche,
        fps_au_lancement: etat_fps.au_lancement,
        fps_en_direct: etat_fps.en_direct,
        eden_d_origine,
        reset_bigpemu,
        fps_avant: etat_fps.avant,
        fps_pose: etat_fps.pose,
        raccourcis_clavier,
        // Après « Sauver puis relancer » : l'emplacement sauvé reste celui du menu (et celui qui est rechargé).
        emplacement: a_recharger.as_ref().map(|c| c.emplacement).unwrap_or(0),
        emplacement_ra: 0,
        etats,
        nom_rom,
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
    // L'écran de lancement (lot 4c) : l'interface montre une attente si le jeu tarde ; le cœur lui dit quand la fenêtre
    // du jeu apparaît (ou que la partie finit, ou au bout de 3 minutes). Lecture seule des fenêtres, toutes les 250 ms :
    // rien ne prend le premier plan (un émulateur en pause hors du premier plan y resterait).
    {
        let app4 = app.clone();
        std::thread::spawn(move || {
            let debut = std::time::Instant::now();
            let menu = app4.state::<crate::menu_jeu::MenuJeu>();
            while menu.en_cours(numero) && debut.elapsed() < std::time::Duration::from_secs(180) {
                if crate::plein_ecran::fenetre_visible(&crate::lancement::processus_de_la_partie(pid)) {
                    break;
                }
                // Chaque relevé lit tous les programmes : serré au début, espacé ensuite (un jeu qui compile ses shaders).
                std::thread::sleep(std::time::Duration::from_millis(if debut.elapsed().as_secs() < 10 { 250 } else { 1000 }));
            }
            let _ = app4.emit("partie-affichee", serde_json::json!({ "jeu": id, "ms": debut.elapsed().as_millis() as u64 }));
        });
    }
    // Eden, Eglantine : leur fenêtre « Recommended … settings » (posée à chaque démarrage, elle bloque le jeu à la
    // manette) reçoit « Yes » (✅ Seb 08/11) : les réglages optimaux de l'émulateur, pour cette partie seulement.
    if famille_eden {
        let app5 = app.clone();
        std::thread::spawn(move || {
            let debut = std::time::Instant::now();
            let menu = app5.state::<crate::menu_jeu::MenuJeu>();
            // Une seule boîte par démarrage (par jeu) : le fil s'arrête dès qu'elle est traitée.
            while menu.en_cours(numero) && debut.elapsed() < std::time::Duration::from_secs(300) {
                std::thread::sleep(std::time::Duration::from_secs(1));
                match crate::menu_jeu::accepter_reglages_recommandes(&crate::lancement::processus_de_la_partie(pid)) {
                    crate::menu_jeu::Reponse::Aucune => continue,
                    crate::menu_jeu::Reponse::Fermee { titre, touche } => {
                        app5.state::<Noyau>().journaliser(&format!("{nom_eden} : « {titre} » → Oui (touche {touche}), fenêtre fermée"));
                    }
                    crate::menu_jeu::Reponse::ToujoursLa { titre } => {
                        app5.state::<Noyau>().journaliser(&format!("{nom_eden} : « {titre} » toujours là après Y et O : laissée à la personne"));
                    }
                }
                break;
            }
        });
    }
    // BigPEmu : sa fenêtre de triches (ouverte à chaque démarrage d'un jeu à triche cochée ; les codes ne s'appliquent
    // qu'à sa fermeture, `cheat_codes.c`) est laissée à la personne (✅ Seb 11/11 : son écran « Press A/S » n'accepte une
    // touche qu'après 2 à 11 s, aucun signal lisible : une touche envoyée à l'aveugle ouvrirait le menu de BigPEmu).
    // Le rappel est montré AVANT le lancement, sous ▶ Jouer (`bigpemu_triches_cochees`) : au lancement, BigPEmu
    // couvre l'écran tout de suite.
    if fenetre_triches_bp {
        noyau.journaliser(&format!("BigPEmu : jeu {id} lancé avec une triche cochée (sa fenêtre de triches attend la personne)"));
    }
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
    if let Some(c) = a_recharger {
        let app5 = app.clone();
        tauri::async_runtime::spawn(async move { recharger_apres_relance(&app5, numero, c, fenetre_triches_bp).await });
    }
    // Les captures rangées pendant la partie (toutes les 2 s, tant qu'elle est la partie en cours).
    let captures_rangees = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    // Le profil qui joue : chaque capture lui est notée (lui seul la verra dans la galerie, ✅ Seb 06/11).
    let profil_captures = profil_ouvert(&noyau).await.map(|p| p.id).unwrap_or_default();
    let captures_finies = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    if let Some(v) = veilleur_captures.clone() {
        let (app3, total, fini3) = (app.clone(), captures_rangees.clone(), captures_finies.clone());
        let (profil_captures, titre_captures, plateforme_captures) = (profil_captures.clone(), titre_captures.clone(), plateforme_captures.clone());
        // Un passage : rend faux quand la partie est finie (plus rien à ranger).
        let passage: PassageCaptures = std::sync::Arc::new(move || {
            // Relu SOUS le verrou : après le dernier passage (`fini`), plus rien n'est rangé hors du compte annoncé.
            let Ok(mut v) = v.lock() else { return false };
            if fini3.load(std::sync::atomic::Ordering::SeqCst) || !app3.state::<crate::menu_jeu::MenuJeu>().en_cours(numero) {
                return false;
            }
            let (places, abandons) = v.ranger();
            total.fetch_add(places.len(), std::sync::atomic::Ordering::SeqCst);
            let noyau = app3.state::<Noyau>();
            if let Err(err) = crate::captures::noter(&noyau.dossier, &places, &profil_captures, id, &titre_captures, &plateforme_captures) {
                noyau.journaliser(&format!("captures d'écran : non notées ({err})"));
            }
            for a in abandons {
                app3.state::<Noyau>().journaliser(&format!("captures d'écran : laissée à sa place, {a}"));
            }
            true
        });
        // La galerie ouverte pendant la partie fait aussi un passage (sinon la dernière photo, prise il y a moins de
        // 2 s, n'y serait pas encore : le « décalage d'une » vu par Seb le 15/11 avec Lichii).
        *PASSAGE_EN_COURS.lock().unwrap_or_else(|e| e.into_inner()) = Some(passage.clone());
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_secs(2));
            if !passage() {
                break;
            }
        });
    }
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        let noyau = app2.state::<Noyau>();
        let fin = noyau.suivre_partie(id, pid, dossiers).await;
        // Un dernier passage : une capture prise juste avant de quitter.
        if let Some(v) = veilleur_captures {
            let fini = captures_finies.clone();
            let (places, abandons) = tauri::async_runtime::spawn_blocking(move || {
                v.lock()
                    .map(|mut v| {
                        fini.store(true, std::sync::atomic::Ordering::SeqCst);
                        v.ranger()
                    })
                    .unwrap_or_default()
            })
            .await
            .unwrap_or_default();
            let n = places.len();
            if let Err(err) = crate::captures::noter(&noyau.dossier, &places, &profil_captures, id, &titre_captures, &plateforme_captures) {
                noyau.journaliser(&format!("captures d'écran : non notées ({err})"));
            }
            for a in abandons {
                noyau.journaliser(&format!("captures d'écran : laissée à sa place, {a}"));
            }
            let total = captures_rangees.fetch_add(n, std::sync::atomic::Ordering::SeqCst) + n;
            if total > 0 {
                noyau.journaliser(&format!("captures d'écran du jeu {id} : {total} rangée(s)"));
                let _ = app2.emit("captures-rangees", serde_json::json!({ "jeu": id, "nombre": total }));
            }
        }
        // Au moins 6 minutes (la compilation mesurée chez Seb : 5 min 30) : la prochaine partie démarre vite.
        if let (Ok(f), Some((d, n))) = (&fin, &compile_rpcs3) {
            if f.secondes >= 360 {
                if let Err(e) = crate::emulateurs_profils::noter_compilation_rpcs3(d, n, &notes_compilation_rpcs3(&noyau)) {
                    noyau.journaliser(&format!("RPCS3 : compilation de {n} non notée ({e:?})"));
                }
            }
        }
        noyau.liberer(id);
        // Seulement si c'est bien la partie en cours qui finit (une partie plus récente garde sa touche et son menu).
        let menu = app2.state::<crate::menu_jeu::MenuJeu>();
        // RetroArch a recopié `video_shader_enable = "true"` en quittant : rendu à « false » si c'était le réglage de la
        // personne et que Frogtend a posé un effet (lot 2, avis lancement).
        if let Some(p) = menu.partie().filter(|_| menu.en_cours(numero)) {
            // Absent avant la partie = « false » (la valeur d'origine de RetroArch) : rendu aussi.
            if let (true, false, Some(d)) = (p.effet_pose, p.shader_enable_avant == Some(true), p.dossier_emulateur.as_deref()) {
                if let Err(e) = crate::emulateurs::rendre_shader_enable(std::path::Path::new(d)) {
                    noyau.journaliser(&format!("effet d'image : video_shader_enable non rendu ({e:?})"));
                }
            }
            // RetroArch a aussi recopié `fps_show` de la partie : rendu à sa valeur d'avant (absent = « false »).
            if let (true, Some("retroarch"), Some(d)) = (p.fps_pose, p.emulateur.as_deref(), p.dossier_emulateur.as_deref()) {
                if let Err(e) = crate::emulateurs::rendre_reglage_retroarch(std::path::Path::new(d), "fps_show", p.fps_avant.unwrap_or(false)) {
                    noyau.journaliser(&format!("images par seconde : fps_show non rendu ({e:?})"));
                }
            }
        }
        let relancer = menu.prendre_relance(numero);
        if !relancer {
            *menu.charger_apres.lock().unwrap_or_else(|e| e.into_inner()) = None;
        }
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
                let _ = app2.emit("partie", EvenementPartie::Fin { jeu: id, secondes: fin.secondes, relancer });
                noyau.journaliser(&format!("partie du jeu {id} : fin annoncée à l'écran{}", if relancer { " (relance demandée)" } else { "" }));
            }
            Err(e) => noyau.journaliser(&format!("suivi de la partie du jeu {id} : {e:?}")),
        }
    });
    Ok(())
}

/// L'émulateur ouvert seul : son ouverture et sa fermeture (événement « emulateur-seul », à part des parties : pas
/// d'écran de lancement, pas de « Partie terminée », aucun temps compté).
#[derive(Clone, Serialize)]
#[serde(tag = "sorte", rename_all = "snake_case")]
enum EvenementEmulateurSeul {
    Debut { nom: String, carte_cedee: bool },
    Fin { nom: String, secondes: u64 },
}

/// Ryubing ouvert seul : les choix de la maison (`contenus-switch.json`) remis en place dans la racine `racine`, pour
/// chaque jeu noté. Si un contenu a été éteint depuis la dernière partie, le bloc de parties de la personne est d'abord
/// mis à l'abri ; s'il ne peut pas l'être : refusé, rien n'a changé. Rend ce qu'il faut noter au journal.
fn ryubing_seul_aux_choix(donnees: &std::path::Path, ryubing: &std::path::Path, racine: &std::path::Path) -> Resultat<Vec<String>> {
    // Les retraits d'avant pas encore repris : rien n'est écrit (ce serait rallumer ce que la personne avait coupé).
    if !crate::contenus_switch::migration_faite(donnees) {
        return Ok(vec!["choix des mises à jour et DLC pas encore repris : rien n'est changé dans Ryubing".into()]);
    }
    let (choix, abime) = crate::contenus_switch::charger(donnees);
    let mut notes: Vec<String> = abime.into_iter().collect();
    let lieu = crate::contenus_switch::lieu("ryubing", racine);
    let a_abriter: Vec<u64> = choix.jeux.iter().filter(|(_, c)| c.abri_a_faire(&lieu)).filter_map(|(k, _)| u64::from_str_radix(k, 16).ok()).collect();
    if !a_abriter.is_empty() {
        match crate::ryubing::abriter_parties(ryubing, racine, "ouverture-seul") {
            Ok(a) => {
                notes.push(match a {
                    Some((d, n)) => format!("parties mises à l'abri ({n} fichier(s), {})", d.display()),
                    None => "aucune partie à mettre à l'abri".into(),
                });
                for b in &a_abriter {
                    if let Err(e) = crate::contenus_switch::noter_abri_fait(donnees, *b, &lieu) {
                        notes.push(format!("abri des parties non noté ({e})"));
                    }
                }
            }
            Err(e) => {
                return Err(Erreur::Refus(format!(
                    "Frogtend n'a pas pu mettre tes parties à l'abri ({e}), comme il le fait après avoir éteint une mise à jour ou un DLC : Ryubing ne s'ouvre pas, rien n'a changé."
                )))
            }
        }
    }
    match crate::ryubing::appliquer_choix_sans_lecture(ryubing, racine, &choix) {
        Ok(0) => {}
        Ok(n) => notes.push(format!("choix des mises à jour et DLC remis en place ({n} fichier(s))")),
        Err(e) => notes.push(format!("choix des mises à jour et DLC non remis en place ({e})")),
    }
    Ok(notes)
}

/// « ▶ Ouvrir l'émulateur seul » (✅ Seb 16-17/11) : l'émulateur `cle` du système `plateforme`, ouvert SANS jeu, avec
/// le profil de la personne (ses dossiers, sa manette, sa liste de jeux complétée), pour régler ses DLC, sa manette, ses
/// réglages. Sans plein écran ni menu en jeu (sa ligne : emulateur_seul.rs). Suivi jusqu'à sa fermeture : session
/// Firehouse sans jeu, aucune partie lancée pendant ce temps, aucun temps compté. Rend le nom de l'émulateur.
#[tauri::command]
pub async fn emulateur_ouvrir_seul(app: AppHandle, noyau: State<'_, Noyau>, plateforme: String, cle: String) -> Resultat<String> {
    profil_ouvert(&noyau).await?;
    let reglages = app.store("pc.json").ok().and_then(|s| s.get("reglages")).unwrap_or(Value::Null);
    let e = crate::choix_emulateur::du_systeme(&reglages, &plateforme)
        .0
        .into_iter()
        .find(|e| e.cle == cle)
        .ok_or_else(|| Erreur::Introuvable("Cet émulateur n'est plus réglé pour ce système.".into()))?;
    let id = crate::emulateur_seul::id_du_programme(&e.programme);
    let nom = match (e.nom.trim(), id.and_then(crate::emulateurs::fiche)) {
        (n, _) if !n.is_empty() => n.to_string(),
        (_, Some(f)) => f.nom.to_string(),
        _ => "Cet émulateur".to_string(),
    };
    let Some(id) = id.filter(|i| crate::emulateur_seul::pris_en_charge(i)) else {
        return Err(Erreur::Refus(format!("Frogtend ne sait pas encore ouvrir {nom} seul, sans jeu. Lance-le par un jeu, avec ▶ Jouer.")));
    };
    let programme = std::path::PathBuf::from(&e.programme);
    if !programme.is_file() {
        return Err(Erreur::Refus(format!(
            "Le programme de {nom} est introuvable ({}). Vérifie-le dans ⚙ Options ▸ Émulateurs.",
            programme.display()
        )));
    }
    if en_installation(&programme) {
        return Err(Erreur::Refus(format!("{nom} est en train de se mettre à jour : attends la fin (la barre d'avancement), puis réessaie.")));
    }
    let dossier = programme.parent().map(std::path::PathBuf::from).unwrap_or_default();
    // Déjà ouvert (par un jeu, ou à la main) : il réécrirait ses réglages en quittant, par-dessus ceux de la personne.
    let d = dossier.clone();
    let ouvert = tauri::async_runtime::spawn_blocking(move || !crate::lancement::processus_du_dossier(&d).is_empty()).await.unwrap_or(true);
    if ouvert {
        return Err(Erreur::Refus(format!("{nom} est déjà ouvert : ferme-le d'abord, puis réessaie.")));
    }
    // Le verrou (un émulateur seul à la fois, jamais pendant une partie ni pendant un lancement en préparation), rendu
    // à sa fermeture ou aussitôt en cas d'échec.
    let partie_en_cours = app.state::<crate::menu_jeu::MenuJeu>().partie().is_some();
    let eden_portable = crate::emulateurs::famille_eden(id) && crate::eden::portable(&dossier);
    let pid = noyau
        .ouvrir_sous_verrou(&dossier, &nom, partie_en_cours, || async {
            // Le profil de la personne, comme pour une partie, mais avec une ligne VIDE (aucun jeu, aucune option de la fiche).
            let (_, ligne) = preparer_emulateur(&app, &noyau, &plateforme, &e.programme, "", &Commandes::default()).await?;
            let arguments = crate::emulateur_seul::arguments_sans_jeu(id, &crate::installation::decouper(&ligne), eden_portable)
                .ok_or_else(|| Erreur::Refus(format!("Frogtend ne sait pas encore ouvrir {nom} seul, sans jeu.")))?;
            // Ryubing : la racine de la personne existe encore, JUSTE avant (sinon il ouvrirait les données communes).
            if id == "ryubing" {
                crate::ryubing::verifier_la_racine(&ligne)?;
                // ✅ Seb 17/11 : les choix de la maison sur les mises à jour et DLC, remis en place dans la racine de la
                // personne (elle peut lancer un jeu de sa liste) ; les parties d'abord à l'abri si un contenu vient d'être
                // éteint. Un abri impossible empêche d'ouvrir ; le reste est seulement noté.
                if let Some(racine) = crate::ryubing::racine_de_la_ligne(&ligne) {
                    assurer_migration_switch(&app, &noyau).await;
                    let (donnees, d) = (noyau.dossier.clone(), dossier.clone());
                    let notes = tauri::async_runtime::spawn_blocking(move || ryubing_seul_aux_choix(&donnees, &d, &racine))
                        .await
                        .map_err(|_| Erreur::Disque("La préparation de Ryubing s'est arrêtée brutalement.".into()))??;
                    for n in notes {
                        noyau.journaliser(&format!("Ryubing ouvert seul : {n}"));
                    }
                }
            }
            let l = crate::installation::Lanceur { programme: e.programme.clone(), arguments, dossier: dossier.to_string_lossy().to_string() };
            crate::lancement::demarrer(&l)
        })
        .await?;
    noyau.journaliser(&format!("{nom} ouvert seul, sans jeu (système {plateforme})"));
    let carte_cedee = noyau.annoncer_sans_jeu("debut").await.is_some_and(|r| r["session"].is_object());
    let _ = app.emit("emulateur-seul", EvenementEmulateurSeul::Debut { nom: nom.clone(), carte_cedee });
    let app2 = app.clone();
    let nom2 = nom.clone();
    tauri::async_runtime::spawn(async move {
        let noyau = app2.state::<Noyau>();
        let secondes = match noyau.suivre_emulateur_seul(pid, dossier.clone()).await {
            Ok(s) => s,
            Err(e) => {
                noyau.journaliser(&format!("suivi de {nom2} ouvert seul : {e:?}"));
                // La session ne reste jamais ouverte chez Firehouse (sinon ses travaux attendraient 30 min).
                noyau.annoncer_sans_jeu("fin").await;
                0
            }
        };
        noyau.liberer_emulateur_seul(&dossier);
        noyau.journaliser(&format!("{nom2} fermé (ouvert seul, {secondes} s, compté à aucun jeu)"));
        // Toujours dit à l'interface (elle débloque ▶ Jouer et sauvegarde les parties).
        let _ = app2.emit("emulateur-seul", EvenementEmulateurSeul::Fin { nom: nom2, secondes });
    });
    Ok(nom)
}

/// Le nom de l'émulateur ouvert seul, s'il y en a un : l'interface le relit à son démarrage (après un rechargement,
/// ▶ Jouer reste bloqué tant qu'il est ouvert).
#[tauri::command]
pub fn emulateur_seul_etat(noyau: State<'_, Noyau>) -> Option<String> {
    noyau.emulateur_seul_nom()
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
        // Seulement une touche proposée : une ancienne « Ctrl+Shift+M » est le son coupé de RPCS3, que Frogtend envoie.
        .filter(|t| TOUCHES_MENU.contains(&t.trim()))
        .unwrap_or_else(|| "Pause".into())
}

/// Les touches proposées pour ouvrir le menu en jeu.
pub const TOUCHES_MENU: &[&str] = &["Pause", "ScrollLock"];

/// Active (début de partie) ou retire (fin) la touche du menu en jeu. Hors partie, la touche reste aux autres.
fn armer_touche_menu(app: &AppHandle, actif: bool) {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let gs = app.global_shortcut();
    // Retirées : exactement celles armées au début (un réglage changé pendant la partie ne laisse rien derrière).
    let armees: Vec<_> = std::mem::take(&mut *TOUCHES_ARMEES.lock().unwrap_or_else(|e| e.into_inner()));
    for (s, _) in armees {
        let _ = gs.unregister(s);
    }
    if !actif {
        return;
    }
    // Sauver / charger au clavier : seulement si le menu les propose pour ce jeu (sinon la touche serait volée au jeu
    // pour rien).
    let p = app.state::<crate::menu_jeu::MenuJeu>().partie();
    let possible = |a: crate::pilotage::Action| p.as_ref().is_some_and(|p| crate::menu_jeu::etats_au_clavier(p, a));
    let (sauver, charger) = touches_etats(app);
    let mut voulues: Vec<(String, &'static str)> = vec![(touche_menu(app), "menu")];
    if let Some(t) = sauver.filter(|_| possible(crate::pilotage::Action::SauverEtat)) {
        voulues.push((t, "sauver"));
    }
    if let Some(t) = charger.filter(|_| possible(crate::pilotage::Action::ChargerEtat)) {
        voulues.push((t, "charger"));
    }
    let mut l = Vec::new();
    for (t, quoi) in voulues {
        let r = t.parse::<tauri_plugin_global_shortcut::Shortcut>().map_err(|e| e.to_string()).and_then(|s| gs.register(s).map(|_| s).map_err(|e| e.to_string()));
        match r {
            Ok(s) => l.push((s, quoi)),
            Err(e) => app.state::<Noyau>().journaliser(&format!("touche « {t} » ({quoi}) : {e}")),
        }
    }
    *TOUCHES_ARMEES.lock().unwrap_or_else(|e| e.into_inner()) = l;
}

/// La combinaison de la manette qui ouvre le menu en jeu (`pc.json` ▸ `menuJeu.manette`, Select + R1 par défaut).
/// Les raccourcis de sauvegarde rapide à la manette (`pc.json` ▸ `menuJeu.raccourcisEtats`, actifs d'office).
fn raccourcis_etats(app: &AppHandle) -> bool {
    app.store("pc.json").ok().and_then(|s| s.get("reglages")).and_then(|r| r["menuJeu"]["raccourcisEtats"].as_bool()).unwrap_or(true)
}

/// Les raccourcis de sauvegarde et de chargement rapides à la manette (`pc.json` ▸ `menuJeu.sauverManette` /
/// `chargerManette`, ✅ Seb 15/11) ; d'office Vue + LT / Vue + RT ; l'ancien interrupteur coupé (`raccourcisEtats` à
/// false) : aucun.
fn raccourcis_manette(app: &AppHandle) -> (Option<crate::osd_manette::Entree>, Option<crate::osd_manette::Entree>) {
    let r = app.store("pc.json").ok().and_then(|s| s.get("reglages")).unwrap_or(Value::Null);
    let lire = |cle: &str, defaut: &str| -> Option<crate::osd_manette::Entree> {
        match r["menuJeu"][cle].as_str() {
            Some(v) => crate::osd_manette::entree(v),
            None if raccourcis_etats(app) => crate::osd_manette::entree(defaut),
            None => None,
        }
    };
    crate::osd_manette::raccourcis_regles(combinaison_menu(app), lire("sauverManette", "Select + LT"), lire("chargerManette", "Select + RT"))
}

/// Les touches du clavier de sauvegarde et de chargement rapides (`pc.json` ▸ `menuJeu.sauverClavier` /
/// `chargerClavier`, ✅ Seb 15/11 : Ctrl + Maj + F5 et Ctrl + Maj + F9 d'office ; « aucune » : rien), jamais la touche
/// du menu ni une touche que Frogtend envoie lui-même (`touche_clavier_permise`).
fn touches_etats(app: &AppHandle) -> (Option<String>, Option<String>) {
    let r = app.store("pc.json").ok().and_then(|s| s.get("reglages")).unwrap_or(Value::Null);
    let lu = |t: &str| t.parse::<tauri_plugin_global_shortcut::Shortcut>().ok();
    let menu = lu(&touche_menu(app));
    // L'ancien interrupteur « pas de raccourci » coupé par la personne : pas de touche d'office non plus.
    let d_office = raccourcis_etats(app);
    let lire = |cle: &str, defaut: &str| -> Option<String> {
        let v = match r["menuJeu"][cle].as_str() {
            Some(v) => v.trim().to_string(),
            None if d_office => defaut.to_string(),
            None => return None,
        };
        (v != "aucune" && !v.is_empty() && lu(&v).is_some() && lu(&v) != menu && touche_clavier_permise(&v)).then_some(v)
    };
    let (s, c) = (lire("sauverClavier", TOUCHE_SAUVER), lire("chargerClavier", TOUCHE_CHARGER));
    // Les deux sur la même touche (fichier abîmé) : aucune (jamais « sauver » à la place de « charger »).
    if s.is_some() && s.as_deref().and_then(lu) == c.as_deref().and_then(lu) {
        return (None, None);
    }
    (s, c)
}

/// Les touches d'office de sauvegarde et de chargement rapides au clavier (✅ Seb 15/11).
pub const TOUCHE_SAUVER: &str = "Ctrl+Shift+F5";
pub const TOUCHE_CHARGER: &str = "Ctrl+Shift+F9";

/// Les touches proposées pour sauver ou charger (jamais F1 à F16, prises par les émulateurs ou envoyées par Frogtend,
/// ni ce que Frogtend envoie lui-même : `pilotage::touche`).
pub const TOUCHES_ETATS: &[&str] = &["Ctrl+Shift+F5", "Ctrl+Shift+F9", "Ctrl+Shift+F6", "Ctrl+Shift+F7", "Ctrl+Shift+F8", "Ctrl+Shift+S", "Ctrl+Shift+L"];

/// Une touche permise pour un raccourci de Frogtend : comprise, et jamais une touche que Frogtend envoie à un émulateur
/// (il se la volerait).
pub fn touche_clavier_permise(t: &str) -> bool {
    let Ok(r) = t.parse::<tauri_plugin_global_shortcut::Shortcut>() else { return false };
    !crate::pilotage::touches_envoyees().iter().any(|e| e.parse::<tauri_plugin_global_shortcut::Shortcut>().is_ok_and(|x| x == r))
}

/// Les touches globales armées pour la partie en cours, avec ce qu'elles font : retirées EXACTEMENT à la fin (même si
/// le réglage a changé entre-temps).
static TOUCHES_ARMEES: std::sync::Mutex<Vec<(tauri_plugin_global_shortcut::Shortcut, &'static str)>> = std::sync::Mutex::new(Vec::new());

/// Une touche globale reçue : ouvrir le menu, ou sauver / charger (menu fermé, jeu pas figé, action proposée).
pub fn touche_globale(app: &AppHandle, s: &tauri_plugin_global_shortcut::Shortcut) {
    let quoi = TOUCHES_ARMEES.lock().unwrap_or_else(|e| e.into_inner()).iter().find(|(x, _)| x == s).map(|(_, q)| *q);
    match quoi {
        Some("sauver") | Some("charger") => {
            let menu = app.state::<crate::menu_jeu::MenuJeu>();
            if menu.ouvert.load(std::sync::atomic::Ordering::SeqCst) || crate::figer::est_fige() {
                return;
            }
            let quoi = if quoi == Some("sauver") { "sauver" } else { "charger" };
            let a = app.clone();
            tauri::async_runtime::spawn(async move {
                // Ctrl et Maj sont encore tenus : la touche envoyée changerait de sens (Dolphin : Maj+F15 = charger ;
                // RPCS3 : Ctrl+Maj+S n'est plus Ctrl+S). Attendus relâchés, au plus 2 s, sinon rien.
                let mut libre = false;
                for _ in 0..40 {
                    if !crate::plein_ecran::modificateur_tenu() {
                        libre = true;
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
                if !libre {
                    a.state::<Noyau>().journaliser(&format!("raccourci {quoi} au clavier : Ctrl ou Maj encore tenus après 2 s, rien envoyé"));
                    return;
                }
                raccourci_etat(&a, quoi).await
            });
        }
        _ => ouvrir_menu(app),
    }
}

/// Le bouton Xbox ouvre le menu en jeu (`pc.json` ▸ `menuJeu.boutonXbox`, actif d'office ; ✅ Seb 28/10).
fn bouton_xbox(app: &AppHandle) -> bool {
    app.store("pc.json").ok().and_then(|s| s.get("reglages")).and_then(|r| r["menuJeu"]["boutonXbox"].as_bool()).unwrap_or(true)
}

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
    // Manette → clavier (jeu PC, réglage du jeu) : le même fil traduit les boutons en touches.
    let partie = app.state::<crate::menu_jeu::MenuJeu>().partie();
    // Le bouton Xbox : seulement si la Game Bar ne le prend pas, et hors des émulateurs qui s'en servent.
    let reglage_xbox = bouton_xbox(&app);
    let emulateur = partie.as_ref().and_then(|p| p.emulateur.clone());
    // Sans émulateur reconnu : un jeu PC (son dossier est connu), sinon un émulateur hors catalogue.
    let ps_retire = partie.as_ref().and_then(|p| p.dossier_emulateur.as_deref()).is_some_and(|d| {
        let d = std::path::Path::new(d);
        crate::manettes::rpcs3_ps_retire(d) && crate::pilotage::rpcs3_active_est_default(d, crate::pilotage::rpcs3_numero_en_cours(d).as_deref())
    });
    // Eden, Eglantine : le bouton Home retiré par Frogtend (sa disposition) → le bouton Xbox ouvre le menu de Frogtend.
    let home_retire = partie.as_ref().and_then(|p| p.dossier_emulateur.as_deref()).is_some_and(|d| crate::eden::home_retire(std::path::Path::new(d)));
    // Cemu avec la manette de Frogtend (XInput) : il ne voit pas le bouton Xbox.
    let cemu_libre = partie.as_ref().and_then(|p| p.dossier_emulateur.as_deref()).is_some_and(|d| crate::cemu::guide_libre(std::path::Path::new(d)));
    // Azahar : son bouton Home pas sur une manette (la disposition de Frogtend le laisse au clavier).
    let azahar_libre = partie.as_ref().and_then(|p| p.dossier_emulateur.as_deref()).is_some_and(|d| crate::azahar::guide_libre(std::path::Path::new(d)));
    // MAME réglé sur un autre pilote de manette que celui d'origine : on ne sait pas ce qu'il fait du bouton Xbox (sa
    // version SDL le donne à son menu, input_sdl.cpp ; le MAME officiel pour Windows : non vérifié).
    let mame_autre_pilote = emulateur.as_deref() == Some("mame")
        && partie.as_ref().and_then(|p| p.dossier_emulateur.as_deref()).is_some_and(|d| !crate::manettes::mame_pilote_d_origine(std::path::Path::new(d)));
    let moteur = match (emulateur.as_deref(), partie.as_ref().is_some_and(|p| p.dossier_jeu.is_some())) {
        (Some("rpcs3"), _) if ps_retire => crate::osd_manette::Moteur::EmulateurLibere,
        (Some(e), _) if home_retire && crate::emulateurs::famille_eden(e) => crate::osd_manette::Moteur::EmulateurLibere,
        (Some("cemu"), _) if cemu_libre => crate::osd_manette::Moteur::EmulateurLibere,
        (Some("azahar"), _) if azahar_libre => crate::osd_manette::Moteur::EmulateurLibere,
        (Some("mame"), _) if mame_autre_pilote => crate::osd_manette::Moteur::Inconnu,
        (Some(e), _) => crate::osd_manette::Moteur::Emulateur(e),
        (None, true) => crate::osd_manette::Moteur::JeuPc,
        (None, false) => crate::osd_manette::Moteur::Inconnu,
    };
    let guide = crate::osd_manette::guide_permis(crate::registre::bouton_game_bar(), reglage_xbox, moteur);
    if reglage_xbox && !guide {
        app.state::<Noyau>().journaliser(&format!(
            "bouton Xbox : n'ouvre pas le menu pendant cette partie (Game Bar : {:?}, émulateur : {})",
            crate::registre::bouton_game_bar(),
            emulateur.as_deref().unwrap_or("aucun")
        ));
    } else if guide && !crate::osd_manette::guide_lisible() {
        app.state::<Noyau>().journaliser("bouton Xbox : illisible sur ce PC pour les manettes Xbox (XInputGetStateEx absent)");
    }
    let (sauver_m, charger_m) = raccourcis_manette(&app);
    let mut osd = crate::osd_manette::Osd::nouveau(combo).avec_etats(sauver_m, charger_m).avec_guide(guide);
    let mut lecteur = crate::osd_manette::Lecteur::default();
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
                    // Vue + LT / Vue + RT : sauver ou charger dans l'emplacement choisi, sans ouvrir le menu.
                    crate::osd_manette::Evenement::Raccourci(quoi) => {
                        let a = app.clone();
                        tauri::async_runtime::spawn(async move { raccourci_etat(&a, quoi).await });
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    });
}

/// Taodbox est-il ouvert (la fenêtre principale en plein écran) ?
fn taodbox_actif(app: &AppHandle) -> bool {
    app.get_webview_window("main").and_then(|m| m.is_fullscreen().ok()).unwrap_or(false)
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
    // En Taodbox (la fenêtre principale est en plein écran, sur la télé) : le menu à l'échelle ×2, dans une grande
    // fenêtre centrée PAR-DESSUS le jeu, qu'on voit autour (✅ Seb 30/10 : « jeu visible ») ; sinon, une petite fenêtre
    // centrée.
    let taodbox = taodbox_actif(app);
    let _ = w.set_fullscreen(false);
    let (mut l, mut h) = if taodbox { (1280.0, 1000.0) } else { (560.0, 700.0) };
    if taodbox {
        if let Ok(Some(m)) = w.current_monitor() {
            let s = m.size().to_logical::<f64>(m.scale_factor());
            l = f64::min(l, s.width * 0.7);
            h = f64::min(h, s.height * 0.9);
        }
    }
    let _ = w.set_size(tauri::LogicalSize::new(l, h));
    let _ = w.center();
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
    let message = menu.prendre_message();
    Ok(Some(crate::menu_jeu::EtatMenu { taodbox, en_pause, fiche, message, ..e }))
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
    if a == crate::pilotage::Action::ImagesParSeconde {
        // Le compteur basculé en direct : la nouvelle valeur (si elle est connue) devient le choix de la personne.
        let r = if e == "mame" {
            action_mame(&app, &noyau, p.dossier_emulateur.as_deref(), a, p.emplacement).await
        } else {
            if !crate::menu_jeu::etat(&p).actions.contains(&action.as_str()) {
                return Err(Erreur::Refus("Cette action n'est pas proposée pour ce jeu.".into()));
            }
            cacher_menu(&app);
            tauri::async_runtime::spawn_blocking(move || crate::menu_jeu::agir(&e, &crate::lancement::processus_de_la_partie(p.pid), a))
                .await
                .map_err(|_| Erreur::Disque("L'action s'est arrêtée brutalement.".into()))?
        };
        if r.is_ok() {
            garder_fps(&app, None, true);
        } else if let Err(err) = &r {
            noyau.journaliser(&format!("menu en jeu : compteur d'images non basculé : {err:?}"));
        }
        return r;
    }
    if e == "mame" {
        return action_mame(&app, &noyau, p.dossier_emulateur.as_deref(), a, p.emplacement).await;
    }
    // Seulement une action que le menu propose pour ce jeu (PS3 : plus de sauvegardes rapides, ✅ Seb 03/11).
    if !crate::menu_jeu::etat(&p).actions.contains(&action.as_str()) {
        return Err(Erreur::Refus("Cette action n'est pas proposée pour ce jeu.".into()));
    }
    // RPCS3 : sauver / charger avec leurs gardes (refus dit dans le menu, encore ouvert).
    if e == "rpcs3" && matches!(a, crate::pilotage::Action::SauverEtat | crate::pilotage::Action::ChargerEtat) {
        return etat_rpcs3(&app, &noyau, p, a, true).await;
    }
    if e == "bigpemu" && p.etats_bigpemu.is_some() && matches!(a, crate::pilotage::Action::SauverEtat | crate::pilotage::Action::ChargerEtat) {
        cacher_menu(&app);
        let r = etat_bigpemu(&app, p, a).await;
        if let Err(err) = &r {
            // Le menu est caché : il se rouvre (le jeu se remet en pause) pour que l'échec se VOIE.
            noyau.journaliser(&format!("BigPEmu : sauvegarde rapide : {err:?}"));
            let a2 = app.clone();
            let _ = app.run_on_main_thread(move || ouvrir_menu(&a2));
        }
        return r;
    }
    cacher_menu(&app);
    if e == "retroarch" && matches!(a, crate::pilotage::Action::SauverEtat | crate::pilotage::Action::ChargerEtat) {
        // Le menu est déjà caché : une erreur est dite À L'ÉCRAN du jeu (sinon personne ne la verrait).
        let r = etat_retroarch(&app, p, a).await;
        if let Err(err) = &r {
            let _ = crate::menu_jeu::commande_retroarch(&format!("SHOW_MSG Frogtend : {err}"));
        }
        return r;
    }
    // Sauver / charger par une touche (DuckStation, PCSX2, Dolphin) : jamais en même temps qu'un raccourci.
    let menu_jeu = app.state::<crate::menu_jeu::MenuJeu>();
    let _seule = if matches!(a, crate::pilotage::Action::SauverEtat | crate::pilotage::Action::ChargerEtat) {
        Some(menu_jeu.prendre_etat().ok_or_else(|| Erreur::Refus("Une sauvegarde ou un chargement rapide est déjà en cours.".into()))?)
    } else {
        None
    };
    let dossier = p.dossier_emulateur.clone();
    let muet_eden = a == crate::pilotage::Action::Muet && matches!(e.as_str(), "eden" | "eglantine" | "lichii");
    let r = tauri::async_runtime::spawn_blocking(move || crate::menu_jeu::agir(&e, &crate::lancement::processus_de_la_partie(p.pid), a))
        .await
        .map_err(|_| Erreur::Disque("L'action s'est arrêtée brutalement.".into()))?;
    // Le menu est déjà caché : un échec ne se voit pas, il est au moins noté.
    if let Err(err) = &r {
        noyau.journaliser(&format!("menu en jeu : action « {action} » non faite : {err:?}"));
    }
    // Eden enregistre le son coupé : il sera remis au prochain lancement (`eden::son_remis`).
    if r.is_ok() && muet_eden {
        if let Some(d) = dossier {
            crate::eden::noter_muet(std::path::Path::new(&d));
        }
    }
    r
}

/// Note le compteur d'images par seconde dans la partie, et le donne à la fenêtre principale qui le garde comme
/// choix de la personne (`menu-fps-garder` ; la fenêtre du menu n'écrit rien elle-même).
fn garder_fps(app: &AppHandle, affiche: Option<bool>, en_direct: bool) {
    let menu = app.state::<crate::menu_jeu::MenuJeu>();
    if let Some(v) = menu.noter_fps(affiche, en_direct) {
        let _ = app.emit("menu-fps-garder", serde_json::json!({ "affiche": v }));
    }
}

/// Le compteur d'images par seconde pour les PROCHAINES parties, quand l'émulateur ne sait pas le basculer en jeu
/// (RPCS3, ou Eden dont le raccourci a été changé) : rien n'est envoyé au jeu, le choix est gardé.
#[tauri::command]
pub fn menu_jeu_fps(app: AppHandle, afficher: bool) -> Resultat<()> {
    let p = app
        .state::<crate::menu_jeu::MenuJeu>()
        .partie()
        .ok_or_else(|| Erreur::Introuvable("Aucune partie en cours.".into()))?;
    if crate::menu_jeu::etat(&p).fps.is_none_or(|f| f.en_direct) {
        return Err(Erreur::Refus("Le compteur ne se règle pas ainsi pour ce jeu.".into()));
    }
    garder_fps(&app, Some(afficher), false);
    Ok(())
}

/// Change l'effet d'image À CHAUD (RetroArch, lot 2) : le menu se ferme, le jeu repart et reçoit `SET_SHADER`
/// (RetroArch affiche « Applying shader » : rien ne le confirme à Frogtend). Une action à la fois.
#[tauri::command]
pub async fn menu_jeu_effet(app: AppHandle, effet: String) -> Resultat<()> {
    let menu = app.state::<crate::menu_jeu::MenuJeu>();
    let p = menu.partie().ok_or_else(|| Erreur::Introuvable("Aucune partie en cours.".into()))?;
    let permis = crate::menu_jeu::etat(&p).effets.is_some_and(|e| effet == "aucun" || e.presents.contains(&effet));
    if !permis {
        return Err(Erreur::Refus("Cet effet ne peut pas être posé depuis le menu.".into()));
    }
    let _seule = menu.prendre_etat().ok_or_else(|| Erreur::Refus("Une autre action est en cours.".into()))?;
    // BigPEmu ne change pas d'effet en pleine partie : le choix est seulement noté (la fenêtre principale le garde
    // pour ce jeu ou sa console, `menu-effet-garder`) ; il vaut au prochain lancement. Le menu reste ouvert.
    if matches!(p.emulateur.as_deref(), Some("bigpemu" | "ryubing")) {
        menu.noter_effet((effet != "aucun").then(|| effet.clone()));
        return Ok(());
    }
    let d = std::path::PathBuf::from(p.dossier_emulateur.clone().unwrap_or_default());
    let (e2, d2) = (effet.clone(), d.clone());
    let commande = tauri::async_runtime::spawn_blocking(move || crate::emulateurs::commande_effet(&d2, &e2))
        .await
        .map_err(|_| Erreur::Disque("L'effet s'est arrêté brutalement.".into()))?
        .map_err(Erreur::Refus)?;
    cacher_menu(&app);
    // Noté AVANT l'envoi : si la partie finit juste après, `video_shader_enable` est quand même rendu.
    menu.noter_effet((effet != "aucun").then(|| effet.clone()));
    let pids = crate::lancement::processus_de_la_partie(p.pid);
    tauri::async_runtime::spawn_blocking(move || -> Resultat<()> {
        // RetroArch doit avoir la main : en pause, il ne redessinerait pas l'image.
        if !crate::menu_jeu::reprendre(&pids) {
            return Err(Erreur::Refus("Le jeu n'a pas pu reprendre la main : rien n'a été fait.".into()));
        }
        std::thread::sleep(std::time::Duration::from_millis(150));
        crate::menu_jeu::commande_retroarch(&commande)
    })
    .await
    .map_err(|_| Erreur::Disque("L'effet s'est arrêté brutalement.".into()))??;
    Ok(())
}

/// La place minimale avant une sauvegarde d'état de RPCS3 : une écriture ratée (disque plein) le fait redémarrer le
/// jeu depuis son lancement — la partie perdue (System.cpp:3833-4057, relecture lancement 30/10).
const PLACE_MIN_RPCS3: u64 = 4 * 1024 * 1024 * 1024;

/// Les sauvegardes d'état RPCS3 faites par ce profil (RPCS3 les range par jeu pour toute la maison : on ne recharge
/// jamais celle d'un autre). `<profil>/rpcs3-etats.json` = noms de fichiers.
fn etats_rpcs3_du_profil(noyau: &Noyau, profil: &str) -> std::path::PathBuf {
    dossier_profil_de(noyau, profil).join("rpcs3-etats.json")
}

/// RPCS3 : sauver (Ctrl+S) ou charger le plus récent (Ctrl+Alt+1), depuis le menu (`depuis_menu` : il se cache
/// seulement si tout est prêt) ou un raccourci manette. Une action à la fois. Sauver : assez de place sur le disque,
/// puis le fichier neuf est attendu (30 s) et retenu pour CE profil. Charger : un état de ce jeu, fait par ce profil, et
/// par CETTE version de RPCS3 (sinon le jeu s'arrêterait) ; les sauvegardes du jeu mises à l'abri avant (un état ne
/// les contient pas : le jeu pourrait réécrire une progression plus ancienne) — une copie ratée, rien n'est envoyé.
async fn etat_rpcs3(app: &AppHandle, noyau: &Noyau, p: crate::menu_jeu::PartieEnCours, a: crate::pilotage::Action, depuis_menu: bool) -> Resultat<()> {
    use crate::pilotage::Action;
    let menu = app.state::<crate::menu_jeu::MenuJeu>();
    let _seule = menu.prendre_etat().ok_or_else(|| Erreur::Refus("Une sauvegarde rapide est déjà en cours.".into()))?;
    let d = std::path::PathBuf::from(p.dossier_emulateur.clone().ok_or_else(|| Erreur::Refus("RPCS3 est introuvable.".into()))?);
    let fichier_profil = etats_rpcs3_du_profil(noyau, &profil_ouvert(noyau).await?.id);
    let (d2, f2) = (d.clone(), fichier_profil.clone());
    let numero = tauri::async_runtime::spawn_blocking(move || -> Resultat<String> {
        let n = crate::pilotage::rpcs3_numero_en_cours(&d2).ok_or_else(|| Erreur::Refus("Le jeu PS3 en cours est introuvable dans le journal de RPCS3.".into()))?;
        if a == Action::SauverEtat {
            let libre = crate::jeux_pc::place_libre(&d2).unwrap_or(u64::MAX);
            let plus_gros = crate::pilotage::rpcs3_etats(&d2, &n).iter().filter_map(|(f, _)| std::fs::metadata(f).ok().map(|m| m.len())).max().unwrap_or(0);
            if libre < u64::max(PLACE_MIN_RPCS3, 2 * plus_gros) {
                return Err(Erreur::Refus("Pas assez de place sur le disque de RPCS3 pour une sauvegarde rapide : libère de la place d'abord (sinon le jeu repartirait du début).".into()));
            }
        } else {
            let miens: Vec<String> = std::fs::read(&f2).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default();
            let mut etats = crate::pilotage::rpcs3_etats(&d2, &n);
            etats.sort_by_key(|(_, t)| *t);
            let (plus_recent, quand) = etats.last().cloned().ok_or_else(|| Erreur::Refus("Aucune sauvegarde rapide de ce jeu : sauve d'abord.".into()))?;
            let nom = plus_recent.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
            if !miens.contains(&nom) {
                return Err(Erreur::Refus("La dernière sauvegarde rapide de ce jeu a été faite par une autre personne : sauve d'abord la tienne.".into()));
            }
            let exe = std::fs::metadata(d2.join("rpcs3.exe")).and_then(|m| m.modified()).ok();
            if exe.is_some_and(|e| quand < e) {
                return Err(Erreur::Refus("Cette sauvegarde rapide a été faite avec une ancienne version de RPCS3 : elle ne se chargerait pas, et le jeu s'arrêterait.".into()));
            }
            crate::contenus::abriter_parties_ps3(&d2, &n).map_err(|e| Erreur::Refus(format!("Ta partie n'a pas pu être mise à l'abri ({e:?}) : rien n'a été chargé.")))?;
        }
        Ok(n)
    })
    .await
    .map_err(|_| Erreur::Disque("La sauvegarde rapide s'est arrêtée brutalement.".into()))??;
    if depuis_menu {
        cacher_menu(app);
    }
    let pids = crate::lancement::processus_de_la_partie(p.pid);
    let debut = std::time::SystemTime::now() - std::time::Duration::from_secs(2);
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || -> Resultat<()> {
        let menu = app2.state::<crate::menu_jeu::MenuJeu>();
        let journal_avant = crate::pilotage::rpcs3_taille_journal(&d);
        crate::menu_jeu::agir("rpcs3", &pids, a)?;
        if a != Action::SauverEtat {
            return Ok(());
        }
        // Le fichier neuf (la touche ne confirme rien) ; la main n'est rendue qu'après (jamais deux actions mêlées).
        for _ in 0..60 {
            std::thread::sleep(std::time::Duration::from_millis(500));
            if let Some((f, _)) = crate::pilotage::rpcs3_etats(&d, &numero).into_iter().find(|(_, t)| *t >= debut) {
                let nom = f.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
                let mut miens: Vec<String> = std::fs::read(&fichier_profil).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default();
                if !miens.contains(&nom) {
                    miens.push(nom);
                }
                let _ = std::fs::write(&fichier_profil, serde_json::to_vec(&miens).unwrap_or_default());
                app2.state::<Noyau>().journaliser(&format!("RPCS3 : sauvegarde rapide de {numero} écrite"));
                return Ok(());
            }
        }
        app2.state::<Noyau>().journaliser(&format!("RPCS3 : sauvegarde rapide de {numero} NON écrite en 30 s (refusée par RPCS3 : jeu en train de sauvegarder ?)"));
        // « failed to lock SPU threads » : le mode compatible manque (dit au prochain menu, avec où le régler).
        if crate::pilotage::rpcs3_refus_spu(&d, journal_avant) {
            menu.noter_refus_etat();
        }
        Ok(())
    })
    .await
    .map_err(|_| Erreur::Disque("La sauvegarde rapide s'est arrêtée brutalement.".into()))?
}

/// RetroArch : sauver ou charger dans l'emplacement CHOISI (✅ Seb 28/10). Charger : `LOAD_STATE_SLOT <n>` (vérifié dans
/// le programme 1.22.2). Sauver : RetroArch ne sait sauver que dans SON emplacement courant (que la personne a pu
/// changer : F6/F7, son menu…) — Frogtend l'amène au bon (`STATE_SLOT_PLUS/MINUS`), mais d'abord met À L'ABRI TOUTES
/// les sauvegardes de ce jeu (une copie ratée : rien n'est envoyé), puis regarde laquelle a VRAIMENT changé : la bonne
/// (sa copie devient `.frogtend-precedent`), une autre (dit franchement, son ancien contenu gardé à côté), aucune (dit,
/// rien d'écrasé). Une seule sauvegarde ou un seul chargement à la fois (menu et raccourci manette).
async fn etat_retroarch(app: &AppHandle, p: crate::menu_jeu::PartieEnCours, a: crate::pilotage::Action) -> Resultat<()> {
    use crate::pilotage::Action;
    let menu = app.state::<crate::menu_jeu::MenuJeu>();
    let _seule = menu.prendre_etat().ok_or_else(|| Erreur::Refus("Une sauvegarde rapide est déjà en cours.".into()))?;
    let n = p.emplacement;
    let pids = crate::lancement::processus_de_la_partie(p.pid);
    let etats = p.etats.clone().map(std::path::PathBuf::from);
    let nom = p.nom_rom.clone();
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || -> Resultat<()> {
        let tous = || etats.as_deref().zip(nom.as_deref()).map(|(d, r)| crate::menu_jeu::etats_du_jeu(d, r)).unwrap_or_default();
        let de = |f: &std::path::Path| nom.as_deref().and_then(|r| crate::menu_jeu::emplacement_du_fichier(f, r));
        // RetroArch doit avoir la main (sinon, en pause hors du premier plan, il pourrait ignorer les commandes).
        if !crate::menu_jeu::reprendre(&pids) {
            return Err(Erreur::Refus("Le jeu n'a pas pu reprendre la main : rien n'a été fait.".into()));
        }
        std::thread::sleep(std::time::Duration::from_millis(150));
        if a == Action::ChargerEtat {
            if etats.is_some() && nom.is_some() && !tous().iter().any(|(f, _)| de(f) == Some(n)) {
                return Err(Erreur::Refus(format!("Aucune sauvegarde dans l'emplacement {} : sauve d'abord.", n + 1)));
            }
            return crate::menu_jeu::commande_retroarch(&format!("LOAD_STATE_SLOT {n}"));
        }
        // 1. Toutes les sauvegardes de ce jeu à l'abri (à côté, `.frogtend-abri`) : une seule copie ratée, rien n'est
        //    envoyé (jamais une partie écrasée sans copie).
        let avant = tous();
        let abri = |f: &std::path::Path| std::path::PathBuf::from(format!("{}.frogtend-abri", f.to_string_lossy()));
        for (f, _) in &avant {
            if let Err(e) = std::fs::copy(f, abri(f)) {
                for (g, _) in &avant {
                    let _ = std::fs::remove_file(abri(g));
                }
                return Err(Erreur::Refus(format!("Ta sauvegarde n'a pas pu être mise à l'abri ({e}) : rien n'a été sauvé.")));
            }
        }
        // 2. Amener RetroArch à l'emplacement voulu (noté à chaque pas : jamais de décalage caché).
        let menu = app2.state::<crate::menu_jeu::MenuJeu>();
        // (Relu sous le verrou : jamais la valeur d'une action qui vient de finir.)
        let mut ra = menu.partie().map(|x| x.emplacement_ra).unwrap_or(p.emplacement_ra);
        while ra != n {
            let (commande, suivant) = if n > ra { ("STATE_SLOT_PLUS", ra + 1) } else { ("STATE_SLOT_MINUS", ra - 1) };
            crate::menu_jeu::commande_retroarch(commande)?;
            ra = suivant;
            menu.noter_emplacement_ra(ra);
            std::thread::sleep(std::time::Duration::from_millis(60));
        }
        crate::menu_jeu::commande_retroarch("SAVE_STATE")?;
        // 3. Quelle sauvegarde a vraiment changé (au plus 4 s) ?
        let mut change: Option<std::path::PathBuf> = None;
        if etats.is_some() && nom.is_some() {
            for _ in 0..40 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                change = tous().into_iter().find(|(f, em)| avant.iter().all(|(g, e2)| g != f || e2 != em)).map(|(f, _)| f);
                if let Some(f) = change.clone() {
                    // Le temps que RetroArch finisse d'écrire : taille stable et fichier refermé (au plus 5 s). Pas
                    // fini : « pas confirmée » (les copies restent à l'abri, et Relancer ne ferme pas le jeu dessus).
                    if !crate::bigpemu::attendre_fichier(&f, std::time::Duration::from_secs(5)) {
                        change = None;
                    }
                    break;
                }
            }
        } else {
            for (g, _) in &avant {
                let _ = std::fs::remove_file(abri(g));
            }
            return Ok(());
        }
        // Les copies : celle du fichier changé devient sa « précédente » ; les autres partent (rien n'a bougé). Rien vu
        // changer dans le délai : TOUTES restent (RetroArch pourrait encore écrire juste après ; la sauvegarde suivante
        // les remplace).
        for (g, _) in avant.iter().filter(|_| change.is_some()) {
            if change.as_deref() == Some(g.as_path()) {
                let _ = std::fs::rename(abri(g), format!("{}.frogtend-precedent", g.to_string_lossy()));
            } else {
                let _ = std::fs::remove_file(abri(g));
            }
        }
        match change {
            None => Err(Erreur::Refus(format!(
                "RetroArch n'a pas confirmé la sauvegarde dans l'emplacement {} (le mode « hardcore » des succès l'interdit, par exemple). Tes sauvegardes de ce jeu sont gardées à l'abri, à côté.",
                n + 1
            ))),
            Some(f) => match de(&f) {
                Some(k) if k == n => Ok(()),
                Some(k) => {
                    menu.noter_emplacement_ra(k);
                    Err(Erreur::Refus(format!(
                        "RetroArch a sauvé dans l'emplacement {} au lieu du {} (il avait changé d'emplacement de son côté) ; l'ancien contenu de l'emplacement {} est gardé à côté. Frogtend s'est recalé : réessaie.",
                        k + 1,
                        n + 1,
                        k + 1
                    )))
                }
                None => Err(Erreur::Refus("RetroArch a sauvé sa sauvegarde automatique au lieu de l'emplacement choisi ; l'ancienne est gardée à côté.".into())),
            },
        }
    })
    .await
    .map_err(|_| Erreur::Disque("L'action s'est arrêtée brutalement.".into()))?
}

/// BigPEmu : sauver ou charger dans l'emplacement CHOISI (✅ Seb 11/11, comme la SNES). BigPEmu ne sait pas changer
/// d'emplacement en jeu : chaque emplacement passe par le sien (`bigpemu::sauver` / `charger`). Une seule sauvegarde
/// ou un seul chargement à la fois (menu et raccourci manette).
async fn etat_bigpemu(app: &AppHandle, p: crate::menu_jeu::PartieEnCours, a: crate::pilotage::Action) -> Resultat<()> {
    use crate::pilotage::Action;
    let menu = app.state::<crate::menu_jeu::MenuJeu>();
    let _seule = menu.prendre_etat().ok_or_else(|| Erreur::Refus("Une sauvegarde rapide est déjà en cours.".into()))?;
    let e = p.etats_bigpemu.clone().ok_or_else(|| Erreur::Refus("Les sauvegardes rapides de BigPEmu sont introuvables.".into()))?;
    let n = p.emplacement;
    let pids = crate::lancement::processus_de_la_partie(p.pid);
    tauri::async_runtime::spawn_blocking(move || {
        let envoyer = || crate::menu_jeu::agir("bigpemu", &pids, a);
        if a == Action::SauverEtat {
            crate::bigpemu::sauver(&e, n, envoyer, std::time::Duration::from_secs(5))
        } else {
            crate::bigpemu::charger(&e, n, envoyer, std::time::Duration::from_millis(1500))
        }
    })
    .await
    .map_err(|_| Erreur::Disque("L'action s'est arrêtée brutalement.".into()))?
}

/// Un raccourci de sauvegarde rapide (menu fermé) : la même action que le menu, puis un message À L'ÉCRAN du jeu
/// quand l'émulateur sait en montrer un (RetroArch : `SHOW_MSG`) ; un échec est noté au journal.
async fn raccourci_etat(app: &AppHandle, quoi: &'static str) {
    let noyau = app.state::<Noyau>();
    let Some(p) = app.state::<crate::menu_jeu::MenuJeu>().partie() else { return };
    let Some(e) = p.emulateur.clone() else { return };
    let a = if quoi == "sauver" { crate::pilotage::Action::SauverEtat } else { crate::pilotage::Action::ChargerEtat };
    // Seulement si le menu propose cette action pour ce jeu (jamais une touche qui n'arriverait pas).
    if !crate::menu_jeu::etat(&p).actions.contains(&quoi) {
        return;
    }
    let n = p.emplacement;
    let est_retroarch = e == "retroarch";
    let est_bigpemu = e == "bigpemu";
    let e_mame = e == "mame";
    let r = if e == "mame" {
        action_mame(app, &noyau, p.dossier_emulateur.as_deref(), a, n).await
    } else if e == "retroarch" {
        etat_retroarch(app, p, a).await
    } else if e == "rpcs3" {
        etat_rpcs3(app, &noyau, p, a, false).await
    } else if e == "bigpemu" && p.etats_bigpemu.is_some() {
        etat_bigpemu(app, p, a).await
    } else {
        // RetroArch, RPCS3, BigPEmu, MAME prennent le verrou eux-mêmes ; ici aussi : jamais deux à la fois.
        let menu = app.state::<crate::menu_jeu::MenuJeu>();
        let Some(_seule) = menu.prendre_etat() else {
            noyau.journaliser(&format!("raccourci {quoi} : une sauvegarde ou un chargement est déjà en cours"));
            return;
        };
        let pids = crate::lancement::processus_de_la_partie(p.pid);
        tauri::async_runtime::spawn_blocking(move || crate::menu_jeu::agir(&e, &pids, a))
            .await
            .unwrap_or_else(|_| Err(Erreur::Disque("L'action s'est arrêtée brutalement.".into())))
    };
    // Charger : « demandé » (RetroArch ne confirme pas un chargement).
    let message = match (&r, quoi) {
        (Ok(()), "sauver") => format!("Sauvegarde rapide : emplacement {}", n + 1),
        (Ok(()), _) => format!("Chargement demandé : emplacement {}", n + 1),
        (Err(err), _) => format!("Frogtend : {err}"),
    };
    if let Err(err) = &r {
        noyau.journaliser(&format!("raccourci {quoi} (emplacement {}) : {err:?}", n + 1));
    }
    if est_retroarch {
        let _ = crate::menu_jeu::commande_retroarch(&format!("SHOW_MSG {message}"));
    }
    // BigPEmu et MAME n'affichent pas les messages de Frogtend : un échec rouvre le menu (le jeu en pause), qui le dit.
    if let (true, Err(err)) = (est_bigpemu || e_mame, &r) {
        app.state::<crate::menu_jeu::MenuJeu>().noter_message(format!("{err}"));
        let a2 = app.clone();
        let _ = app.run_on_main_thread(move || ouvrir_menu(&a2));
    }
}

/// Une relance qui n'a pas abouti (ou un lancement à la main) : aucun rechargement ne reste en attente.
#[tauri::command]
pub fn menu_jeu_oublier_rechargement(app: AppHandle) {
    *app.state::<crate::menu_jeu::MenuJeu>().charger_apres.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// Après « Sauver puis relancer » (✅ Seb 15/11) : recharge l'emplacement sauvé dès que le jeu est PRÊT — RetroArch :
/// `GET_STATUS` dit qu'un jeu tourne ; MAME : son script tourne ; BigPEmu : sa fenêtre répond, plus 3 s. Une seule
/// fois, jamais en boucle ; même contenu exigé ; « demandé » seulement (RetroArch ne confirme pas). BigPEmu avec une
/// triche cochée : sa fenêtre de triches attend la personne au démarrage, rien n'est envoyé à l'aveugle — dit.
async fn recharger_apres_relance(app: &AppHandle, numero: u64, c: crate::menu_jeu::ChargerApres, fenetre_triches: bool) {
    let noyau = app.state::<Noyau>();
    let menu = app.state::<crate::menu_jeu::MenuJeu>();
    let n = c.emplacement + 1;
    if c.emulateur == "bigpemu" && fenetre_triches {
        menu.noter_message(format!("🎯 Avec tes triches, Frogtend ne recharge pas tout seul : menu ▸ 📂 Charger (emplacement {n}) une fois le jeu lancé."));
        noyau.journaliser("relancer : rechargement laissé à la personne (fenêtre de triches de BigPEmu)");
        return;
    }
    let debut = std::time::Instant::now();
    let pret = loop {
        // Au plus 60 s : au-delà, la personne joue déjà (un chargement tardif effacerait ce qu'elle a fait).
        if !menu.en_cours(numero) || debut.elapsed() > std::time::Duration::from_secs(60) {
            break false;
        }
        let Some(p) = menu.partie() else { break false };
        let ok = match c.emulateur.as_str() {
            "retroarch" => tauri::async_runtime::spawn_blocking(crate::menu_jeu::statut_retroarch).await.ok().flatten().is_some_and(|s| crate::menu_jeu::jeu_en_cours_retroarch(&s)),
            "mame" => p.dossier_emulateur.as_deref().is_some_and(|d| crate::pilotage::jeu_mame(std::path::Path::new(d)).is_some()),
            "bigpemu" => crate::lancement::processus_de_la_partie(p.pid).into_iter().any(|x| crate::menu_jeu::fenetres_figees(x) == Some(false)),
            _ => false,
        };
        if ok {
            break true;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    };
    if !pret {
        noyau.journaliser(&format!("relancer : jeu pas prêt à temps, emplacement {n} pas rechargé"));
        return;
    }
    tokio::time::sleep(std::time::Duration::from_millis(if c.emulateur == "bigpemu" { 3000 } else { 1500 })).await;
    let Some(p) = menu.partie().filter(|_| menu.en_cours(numero)) else { return };
    let meme = match c.emulateur.as_str() {
        "retroarch" => p.nom_rom.as_deref() == Some(c.contenu.as_str()),
        "bigpemu" => p.empreinte.as_deref() == Some(c.contenu.as_str()),
        _ => true,
    };
    if !meme || p.emplacement != c.emplacement || !crate::menu_jeu::etat(&p).actions.contains(&"charger") {
        noyau.journaliser(&format!("relancer : emplacement {n} pas rechargé (autre contenu, autre emplacement, ou chargement interdit : mode hardcore…)"));
        menu.noter_message(format!("Ta sauvegarde n'a pas été rechargée : menu ▸ 📂 Charger (emplacement {n})."));
        return;
    }
    // La personne a ouvert le menu entre-temps : rien n'est envoyé dans son dos, le menu le lui dit.
    if menu.ouvert.load(std::sync::atomic::Ordering::SeqCst) {
        noyau.journaliser(&format!("relancer : menu ouvert, emplacement {n} laissé à la personne"));
        menu.noter_message(format!("Ta sauvegarde n'a pas été rechargée (menu ouvert) : 📂 Charger (emplacement {n})."));
        return;
    }
    noyau.journaliser(&format!("relancer : rechargement de l'emplacement {n}"));
    raccourci_etat(app, "charger").await;
}

/// Active ou coupe la triche `index` du jeu en cours (RetroArch, lot 3 ✅ Seb 28/10) : le menu se cache, le jeu
/// reprend la main et la reçoit. Une erreur est dite À L'ÉCRAN du jeu.
#[tauri::command]
pub async fn menu_jeu_triche(app: AppHandle, index: usize) -> Resultat<()> {
    let p = app
        .state::<crate::menu_jeu::MenuJeu>()
        .partie()
        .ok_or_else(|| Erreur::Introuvable("Aucune partie en cours.".into()))?;
    let permis = crate::menu_jeu::etat(&p).triches.is_some_and(|l| index < l.len());
    if !permis {
        return Err(Erreur::Refus("Cette triche ne peut pas être changée depuis le menu.".into()));
    }
    let menu = app.state::<crate::menu_jeu::MenuJeu>();
    // Une seule action à la fois (avec les sauvegardes rapides), de la mise à l'abri jusqu'à la bascule.
    let _seule = menu.prendre_etat().ok_or_else(|| Erreur::Refus("Une sauvegarde rapide ou une triche est déjà en cours.".into()))?;
    // BigPEmu : le choix est noté pour le prochain lancement (il ne lit ses triches qu'au démarrage ; ses parties sont
    // mises à l'abri à ce moment-là, avant de jouer avec). Le menu reste ouvert.
    if p.emulateur.as_deref() == Some("bigpemu") {
        let (Some(profil), Some(cle), Some(t)) = (p.dossier_profil.clone(), p.cles_triches.get(index).cloned(), p.triches.get(index)) else {
            return Err(Erreur::Refus("Cette triche ne peut pas être changée depuis le menu.".into()));
        };
        let active = !t.active;
        tauri::async_runtime::spawn_blocking(move || crate::bigpemu::choisir_triche(std::path::Path::new(&profil), &cle, active))
            .await
            .map_err(|_| Erreur::Disque("La triche s'est arrêtée brutalement.".into()))??;
        menu.basculer_triche(index);
        return Ok(());
    }
    // La mise à l'abri AVANT de cacher le menu : un échec s'y dit (le jeu, en pause derrière, ne montrerait rien).
    if !menu.partie().is_some_and(|x| x.abri_triches) {
        let nom = p.nom_rom.clone().ok_or_else(|| Erreur::Refus("Le jeu lancé est inconnu : sa partie ne peut pas être mise à l'abri, aucune triche n'a été activée.".into()))?;
        let (parties, etats) = (p.parties.clone(), p.etats.clone());
        tauri::async_runtime::spawn_blocking(move || {
            crate::menu_jeu::mettre_a_l_abri_avant_triche(
                parties.as_deref().map(std::path::Path::new),
                etats.as_deref().map(std::path::Path::new),
                &nom,
                secondes_maintenant(),
            )
        })
        .await
        .map_err(|_| Erreur::Disque("La mise à l'abri s'est arrêtée brutalement.".into()))?
        .map_err(Erreur::Refus)?;
        menu.noter_abri_triches();
    }
    cacher_menu(&app);
    let r = triche_retroarch(&app, p, index).await;
    if let Err(err) = &r {
        let _ = crate::menu_jeu::commande_retroarch(&format!("SHOW_MSG Frogtend : {err}"));
        app.state::<Noyau>().journaliser(&format!("triche {} : {err:?}", index + 1));
    }
    r
}

/// RetroArch (la partie déjà à l'abri, le verrou pris par l'appelant) : son index de triche est amené pas à pas
/// (`CHEAT_INDEX_PLUS/MINUS`, noté à chaque pas : jamais de décalage caché ; 120 ms entre deux, pour que deux
/// commandes n'arrivent pas dans la même image), et `CHEAT_TOGGLE` bascule la triche (RetroArch le confirme à l'écran).
async fn triche_retroarch(app: &AppHandle, p: crate::menu_jeu::PartieEnCours, k: usize) -> Resultat<()> {
    let pids = crate::lancement::processus_de_la_partie(p.pid);
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || -> Resultat<()> {
        let menu = app2.state::<crate::menu_jeu::MenuJeu>();
        // RetroArch doit avoir la main (sinon, en pause hors du premier plan, il pourrait ignorer les commandes).
        if !crate::menu_jeu::reprendre(&pids) {
            return Err(Erreur::Refus("Le jeu n'a pas pu reprendre la main : rien n'a été fait.".into()));
        }
        std::thread::sleep(std::time::Duration::from_millis(150));
        // (Relu sous le verrou : jamais la valeur d'une action qui vient de finir.)
        let mut i = menu.partie().map(|x| x.triche_ra).unwrap_or(p.triche_ra);
        for c in crate::menu_jeu::pas_vers_triche(i, k) {
            crate::menu_jeu::commande_retroarch(c)?;
            i = if c == "CHEAT_INDEX_PLUS" { i + 1 } else { i - 1 };
            menu.noter_triche_ra(i);
            std::thread::sleep(std::time::Duration::from_millis(120));
        }
        crate::menu_jeu::commande_retroarch("CHEAT_TOGGLE")?;
        menu.basculer_triche(k);
        Ok(())
    })
    .await
    .map_err(|_| Erreur::Disque("La triche s'est arrêtée brutalement.".into()))?
}

/// Des parties à l'abri à remettre dans ce jeu réinstallé (lot 3) ; `None` : rien (ou déjà demandé).
#[tauri::command]
pub async fn parties_abri_a_reposer(noyau: State<'_, Noyau>, id: i64) -> Resultat<Option<crate::partie::AbriAReposer>> {
    noyau.abri_a_reposer(id).await
}

/// Remet ces parties dans le jeu (après le oui de la personne).
#[tauri::command]
pub async fn parties_abri_reposer(noyau: State<'_, Noyau>, id: i64) -> Resultat<usize> {
    noyau.reposer_abri(id).await
}

/// La personne a dit non : la question ne revient pas pour cet abri.
#[tauri::command]
pub fn parties_abri_ignorer(noyau: State<'_, Noyau>, id: i64, date: String) {
    noyau.noter_abri_propose(id, &date);
}

/// Le mode compatible des sauvegardes rapides d'un jeu PS3 (✅ Seb 31/10 : dans Gérer le jeu) : (actif, pour ce
/// jeu) ; `None` : pas un jeu RPCS3 ou numéro illisible.
#[tauri::command]
pub async fn rpcs3_etats_compatibles(app: AppHandle, noyau: State<'_, Noyau>, id: i64) -> Resultat<Option<(bool, bool)>> {
    let Some((_, rpcs3, Some(n), _)) = contexte_image_ps3(&app, &noyau, id).await? else { return Ok(None) };
    Ok(Some(crate::emulateurs_profils::rpcs3_etats_compatibles(&rpcs3, &n)?))
}

/// L'active ou le coupe pour ce jeu (RPCS3 fermé : il réécrirait ses réglages en quittant).
#[tauri::command]
pub async fn rpcs3_regler_etats_compatibles(app: AppHandle, noyau: State<'_, Noyau>, id: i64, actif: bool) -> Resultat<()> {
    let Some((_, rpcs3, Some(n), _)) = contexte_image_ps3(&app, &noyau, id).await? else {
        return Err(Erreur::Refus("Ce jeu ne se lance pas avec RPCS3, ou son numéro est illisible.".into()));
    };
    if !crate::lancement::processus_du_dossier(&rpcs3).is_empty() {
        return Err(Erreur::Refus("RPCS3 est ouvert : ferme le jeu d'abord.".into()));
    }
    crate::emulateurs_profils::rpcs3_regler_etats_compatibles(&rpcs3, &n, actif)?;
    noyau.journaliser(&format!("RPCS3 : mode compatible des sauvegardes rapides {} pour {n}", if actif { "activé" } else { "coupé" }));
    Ok(())
}

/// Le fichier où Frogtend note les parties PS3 complètes, par version de RPCS3 (lot 5).
fn notes_compilation_rpcs3(noyau: &Noyau) -> std::path::PathBuf {
    noyau.dossier.join("rpcs3-compile.json")
}

/// Une partie PS3 qui sera LONGUE à démarrer (RPCS3 compile le jeu) : le message à dire AVANT de jouer, sinon `None`
/// (✅ Seb 30/10, après Asura's Wrath : « ne quitte pas » — une partie coupée pendant la compilation ne garde rien).
#[tauri::command]
pub async fn jeu_compilation_ps3(app: AppHandle, noyau: State<'_, Noyau>, id: i64, emulateur: Option<String>, version: Option<String>) -> Resultat<Option<String>> {
    let jeu = noyau.registre().jeu(id)?;
    let Some(j) = jeu else { return Ok(None) };
    let Some((programme, _)) = emulateur_regle(&app, &j.plateforme, id, emulateur.as_deref()) else { return Ok(None) };
    let programme = std::path::PathBuf::from(programme);
    if !programme.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("rpcs3.exe")) {
        return Ok(None);
    }
    // La version qui sera lancée (deux régions = deux numéros, deux compilations).
    let Some(fichier) = noyau.fichier_pour_l_emulateur(id, version.as_deref()).await.ok().flatten() else { return Ok(None) };
    let notes = notes_compilation_rpcs3(&noyau);
    let raison = tauri::async_runtime::spawn_blocking(move || {
        let numero = crate::contenus::titre_id_ps3(&fichier)?;
        let rpcs3 = programme.parent()?;
        let r = crate::emulateurs_profils::compilation_rpcs3(rpcs3, &numero, &notes)?;
        // Prévenu : le message reviendra tant qu'une partie complète (≥ 6 min) n'aura pas été vue.
        let _ = crate::emulateurs_profils::noter_compilation_en_cours(rpcs3, &numero, &notes);
        Some(r)
    })
    .await
    .unwrap_or(None);
    Ok(raison.map(|r| {
        if r == "reprise" {
            "La préparation de ce jeu par RPCS3 n'est pas terminée (la dernière partie a été arrêtée trop tôt) : cela peut encore prendre plusieurs minutes. Ne quitte pas avant d'arriver au jeu.".to_string()
        } else if r == "premiere" {
            "C'est la première partie de ce jeu : RPCS3 le prépare pour ton PC, cela peut prendre plusieurs minutes (l'écran reste figé ou affiche « Compiling… »). Ne quitte pas avant d'arriver au jeu : sinon tout recommence la fois suivante.".to_string()
        } else {
            "RPCS3 a été mis à jour : la première partie recompile ce jeu, cela peut prendre plusieurs minutes (l'écran reste figé ou affiche « Compiling… »). Ne quitte pas avant d'arriver au jeu : sinon tout recommence la fois suivante.".to_string()
        }
    }))
}

/// Les effets d'image proposés pour un RetroArch (lot 4a/4b) : ceux qu'il a, et pourquoi aucun ne l'est (pilote
/// vidéo). Lecture seule.
#[derive(Serialize)]
pub struct EffetImage {
    id: String,
    present: bool,
}

#[derive(Serialize)]
pub struct EffetsImage {
    impossible: Option<String>,
    effets: Vec<EffetImage>,
}

#[tauri::command]
pub async fn effets_image(noyau: State<'_, Noyau>, programme: String, jeu: Option<i64>) -> Resultat<EffetsImage> {
    let d = std::path::Path::new(&programme).parent().map(|p| p.to_path_buf()).unwrap_or_default();
    // BigPEmu : ses propres effets d'écran (livrés avec lui, `bigpemu::EFFETS`).
    let nom = std::path::Path::new(&programme).file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    // Ryubing : ses réglages d'image (`ryubing::EFFETS`), tous présents — sauf s'il n'a pas été installé par Frogtend,
    // ou si ce jeu a ses propres réglages dans Ryubing (ils passent devant : l'effet ne s'y appliquerait pas).
    if crate::ryubing::est_ryubing(&programme) {
        let profil = profil_ouvert(&noyau).await?.nom;
        let fichier = match jeu {
            Some(j) => noyau.fichier_pour_l_emulateur(j, None).await.ok().flatten(),
            None => None,
        };
        let r = tauri::async_runtime::spawn_blocking(move || {
            let titre = fichier.as_deref().and_then(crate::contenus::id_switch);
            crate::ryubing::effets_possibles(&d, &crate::ryubing::racine(&d, &profil), titre)
        })
        .await
        .map_err(|_| Erreur::Disque("La lecture des effets d'image s'est arrêtée brutalement.".into()))?;
        let effets = crate::ryubing::effets_presents().into_iter().map(|id| EffetImage { id, present: true }).collect();
        return Ok(EffetsImage { impossible: r.err(), effets });
    }
    if nom.starts_with("bigpemu") {
        let presents = crate::bigpemu::effets_presents(&d);
        return Ok(EffetsImage {
            impossible: None,
            effets: crate::bigpemu::EFFETS
                .iter()
                .map(|e| {
                    let id = format!("{}{e}", crate::bigpemu::PREFIXE_EFFET);
                    EffetImage { present: presents.contains(&id), id }
                })
                .collect(),
        });
    }
    Ok(EffetsImage {
        impossible: crate::emulateurs::effets_impossibles(&d),
        effets: crate::emulateurs::EFFETS_IMAGE
            .iter()
            .map(|(id, _, chemin)| EffetImage { id: id.to_string(), present: crate::emulateurs::fichier_effet(&d, chemin).is_some() })
            .collect(),
    })
}

/// Ce que le cœur sait des boutons d'un jeu (lot 4d) : l'émulateur et son cœur, la manette du joueur (Xbox, ou
/// PlayStation si seule une manette Sony est branchée), et si la disposition a été changée à la main (alors l'interface
/// ne dessine rien de faux). Le dessin et les tables sont dans l'interface (`src/lib/manette/boutons.ts`).
#[derive(Serialize)]
pub struct InfosBoutons {
    plateforme: String,
    emulateur: Option<String>,
    coeur: Option<String>,
    famille: &'static str,
    personnalise: bool,
    /// Frogtend ne règle pas la manette de cet émulateur ici (Cemu, Eden et sa famille hors portable) : rien de dessiné,
    /// sans dire pour autant que la personne a changé ses boutons.
    inconnue: bool,
    /// L'émulateur joue au clavier (Eden et sa famille : aucune manette Xbox branchée au lancement, ou le clavier choisi
    /// pour ce jeu) : le dessin de la manette serait faux.
    au_clavier: bool,
    /// Ryubing : une manette branchée que Frogtend ne connaît pas encore pour lui (pas encore relevée) : le jeu se
    /// jouera au clavier.
    manette_pas_connue: bool,
    /// Dolphin, une Wii : la référence livrée de Wiimote en place dans le profil (« Wiimote + Nunchuk »…), s'il y en a une.
    wiimote: Option<&'static str>,
    clavier: Option<std::collections::BTreeMap<String, String>>,
    /// Les raccourcis de Frogtend pendant une partie, d'après les réglages de ce PC (essai de Seb, 29/10).
    raccourcis: RaccourcisFrogtend,
}

/// Les raccourcis de Frogtend en jeu : la combinaison du menu (nom du réglage, « Select + R1 »…), le bouton du milieu
/// (s'il ouvre le menu pour ce jeu), la touche du clavier, et les sauvegarde / chargement rapides à la manette.
#[derive(Serialize)]
pub struct RaccourcisFrogtend {
    combinaison: String,
    bouton_xbox: bool,
    touche: String,
    etats: bool,
    /// Les raccourcis réglés (`Select + LT`… ; vide : aucun) et les touches du clavier (vide : aucune).
    sauver_manette: String,
    charger_manette: String,
    sauver_clavier: String,
    charger_clavier: String,
}

fn raccourcis_frogtend(app: &AppHandle, moteur: crate::osd_manette::Moteur, etats_possibles: bool) -> RaccourcisFrogtend {
    let combinaison = app
        .store("pc.json")
        .ok()
        .and_then(|s| s.get("reglages"))
        .and_then(|r| r["menuJeu"]["manette"].as_str().map(String::from))
        .filter(|c| !c.trim().is_empty())
        .unwrap_or_else(|| "Select + R1".into());
    let (manette, clavier) = (raccourcis_manette(app), touches_etats(app));
    RaccourcisFrogtend {
        combinaison,
        // Annoncé seulement s'il marche vraiment sur ce PC (bouton lisible, Game Bar qui l'a lâché, émulateur permis).
        bouton_xbox: crate::osd_manette::guide_permis(crate::registre::bouton_game_bar(), bouton_xbox(app), moteur) && crate::osd_manette::guide_lisible(),
        touche: touche_menu(app),
        etats: etats_possibles && (manette.0.is_some() || manette.1.is_some() || clavier.0.is_some() || clavier.1.is_some()),
        sauver_manette: manette.0.filter(|_| etats_possibles).map(crate::osd_manette::nom_entree).unwrap_or_default().into(),
        charger_manette: manette.1.filter(|_| etats_possibles).map(crate::osd_manette::nom_entree).unwrap_or_default().into(),
        sauver_clavier: clavier.0.filter(|_| etats_possibles).unwrap_or_default(),
        charger_clavier: clavier.1.filter(|_| etats_possibles).unwrap_or_default(),
    }
}

/// `id` : un jeu du PC (fiche, Gérer le jeu) ; `None` : la partie en cours (menu en jeu). `None` rendu : jeu inconnu.
#[tauri::command]
/// `wiimote_choisie` : la référence de Wiimote choisie pour ce jeu dans « 🎮 Commandes » (fiche seulement), imposée au
/// lancement.
pub async fn boutons_du_jeu(app: AppHandle, noyau: State<'_, Noyau>, id: Option<i64>, wiimote_choisie: Option<String>) -> Resultat<Option<InfosBoutons>> {
    let profil = profil_ouvert(&noyau).await?.nom;
    // (plateforme, émulateur, dossier de l'émulateur, cœur, nom du jeu lancé, nom de son dossier, clavier)
    // Et pour les raccourcis : un jeu PC (sans émulateur) ? les sauvegardes rapides sont-elles proposées ?
    let jeu_pc;
    let etats_possibles;
    // MAME : le nom du jeu tel que MAME nomme ses fichiers de touches (`cfg\<jeu>.cfg`), s'il est connu.
    let jeu_mame;
    // Le jeu se lance-t-il par l'émulateur réglé ? (Faux : son propre lanceur, un DOSBox livré avec le jeu…)
    let par_l_emulateur;
    // Pendant une partie : un émulateur que le menu ne nomme pas (hors catalogue, MAME sans son script) : on ne sait pas.
    let sans_nom;
    let (plateforme, emulateur, dossier, coeur, rom, contenu, clavier) = match id {
        None => {
            let Some(p) = app.state::<crate::menu_jeu::MenuJeu>().partie() else { return Ok(None) };
            etats_possibles = crate::menu_jeu::etat(&p).actions.contains(&"sauver");
            jeu_pc = p.emulateur.is_none() && p.dossier_jeu.is_some();
            sans_nom = p.emulateur.is_none() && p.dossier_jeu.is_none() && p.dossier_emulateur.is_some();
            jeu_mame = p.dossier_emulateur.as_deref().filter(|_| p.emulateur.as_deref() == Some("mame")).and_then(|d| crate::pilotage::jeu_mame(std::path::Path::new(d))).map(|(n, _)| n);
            par_l_emulateur = !matches!(p.emulateur.as_deref(), Some("mame" | "dosbox-staging")) || noyau.fichier_pour_l_emulateur(p.jeu, None).await.ok().flatten().is_some();
            (p.plateforme, p.emulateur, p.dossier_emulateur.map(std::path::PathBuf::from), p.coeur, p.nom_rom, p.dossier_rom, p.clavier)
        }
        Some(id) => {
            // Un jeu caché au profil ouvert : rien (jamais ce qui a été vu pour une autre personne).
            let j = match noyau.jeu_jouable(id).await {
                Ok(j) => j,
                Err(Erreur::Introuvable(_)) => return Ok(None),
                Err(e) => return Err(e),
            };
            let de_boutique = j.installation.as_ref().is_some_and(|i| i.boutique.is_some());
            let regle = (!de_boutique).then(|| emulateur_regle(&app, &j.plateforme, id, None)).flatten();
            let (mut emulateur, mut dossier, mut coeur) = (None, None, None);
            let mut programme_et_ligne = None;
            if let Some((programme, ligne)) = regle {
                let chemin = std::path::PathBuf::from(&programme);
                let nom = chemin.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
                emulateur = crate::emulateurs::CATALOGUE.iter().find(|f| nom.starts_with(f.programme)).map(|f| f.id.to_string());
                if emulateur.is_none() && nom.starts_with("mame") && nom.ends_with(".exe") {
                    emulateur = Some("mame".into());
                }
                dossier = chemin.parent().map(|d| d.to_path_buf());
                if emulateur.as_deref() == Some("retroarch") {
                    coeur = dossier.as_deref().zip(crate::emulateurs::coeur_de(&ligne)).and_then(|(d, c)| crate::emulateurs::nom_du_coeur(d, &c));
                }
                programme_et_ligne = Some((programme, ligne));
            }
            let fichier = if emulateur.is_some() { noyau.fichier_pour_l_emulateur(id, None).await.ok().flatten() } else { None };
            let rom = fichier.as_deref().and_then(|f| f.file_stem()).map(|s| s.to_string_lossy().to_string());
            let contenu = fichier.as_deref().and_then(|f| f.parent()).and_then(|d| d.file_name()).map(|s| s.to_string_lossy().to_string());
            jeu_pc = emulateur.is_none();
            sans_nom = false;
            par_l_emulateur = emulateur.is_none() || fichier.is_some();
            // Le nom court que Frogtend donne à MAME (`installation::nom_court_mame`) ; une machine de salon : celui de
            // la machine, inconnu ici (seul `default.cfg` est lu).
            jeu_mame = match (emulateur.as_deref(), &programme_et_ligne, j.installation.as_ref().and_then(|i| i.fichier_du_jeu.as_deref())) {
                (Some("mame"), Some((programme, ligne)), Some(f)) if !crate::installation::ligne_mame_de_salon(ligne) => crate::installation::nom_court_mame(programme, f),
                _ => None,
            };
            etats_possibles = emulateur.as_deref().is_some_and(|e| crate::pilotage::actions(e).contains(&crate::pilotage::Action::SauverEtat))
                // BigPEmu : seulement une fois ses touches posées (pas avant son premier lancement pour cette personne).
                && (emulateur.as_deref() != Some("bigpemu")
                    || dossier.as_deref().is_some_and(|d| crate::bigpemu::raccourcis_poses(&crate::emulateurs_profils::dossier_du_profil(std::path::Path::new(d), &profil))));
            (j.plateforme, emulateur, dossier, coeur, rom, contenu, None)
        }
    };
    let ps_retire = dossier.as_deref().is_some_and(|d| crate::manettes::rpcs3_ps_retire(d) && crate::pilotage::rpcs3_active_est_default(d, None));
    let home_retire = dossier.as_deref().is_some_and(crate::eden::home_retire);
    let cemu_libre = dossier.as_deref().is_some_and(crate::cemu::guide_libre);
    let azahar_libre = dossier.as_deref().is_some_and(crate::azahar::guide_libre);
    // MAME réglé sur un autre pilote de manette que celui d'origine : on ne sait pas ce qu'il fait du bouton Xbox (sa
    // version SDL le donne à son menu, input_sdl.cpp ; le MAME officiel pour Windows : non vérifié).
    let mame_autre_pilote = emulateur.as_deref() == Some("mame") && dossier.as_deref().is_some_and(|d| !crate::manettes::mame_pilote_d_origine(d));
    let moteur = match (emulateur.as_deref(), jeu_pc) {
        (Some("rpcs3"), _) if ps_retire => crate::osd_manette::Moteur::EmulateurLibere,
        (Some(e), _) if home_retire && crate::emulateurs::famille_eden(e) => crate::osd_manette::Moteur::EmulateurLibere,
        (Some("cemu"), _) if cemu_libre => crate::osd_manette::Moteur::EmulateurLibere,
        (Some("azahar"), _) if azahar_libre => crate::osd_manette::Moteur::EmulateurLibere,
        (Some("mame"), _) if mame_autre_pilote => crate::osd_manette::Moteur::Inconnu,
        (Some(e), _) => crate::osd_manette::Moteur::Emulateur(e),
        (None, true) => crate::osd_manette::Moteur::JeuPc,
        (None, false) => crate::osd_manette::Moteur::Inconnu,
    };
    let raccourcis = raccourcis_frogtend(&app, moteur, etats_possibles);
    let (e2, d2, c2) = (emulateur.clone(), dossier.clone(), coeur.clone());
    // Dolphin : seule la manette GameCube est réglée par Frogtend (la Wii a ses Wiimotes : « inconnu » côté interface).
    let gamecube = plateforme.to_lowercase().contains("gamecube");
    let dosbox = emulateur.as_deref() == Some("dosbox-staging");
    let en_partie = id.is_none();
    let plateforme_wii = plateforme.clone();
    let magasin = noyau.dossier.join("references");
    let wiimote_choisie = wiimote_choisie.filter(|_| !en_partie);
    let (famille, personnalise, plusieurs_manettes, au_clavier, wiimote) = tauri::async_runtime::spawn_blocking(move || {
        // Une Wii dans Dolphin : la référence choisie pour le jeu (imposée au lancement) ; sinon la Wiimote 1 du profil
        // (`-u <profil>\User`), écrite par Frogtend au lancement ; jamais lancé : la référence du système, appliquée alors.
        let wiimote = match (e2.as_deref(), d2.as_deref()) {
            (Some("dolphin"), Some(d)) if !gamecube => match &wiimote_choisie {
                Some(n) => crate::references::trouver(&magasin, "dolphin", "Wiimote", n).and_then(|r| crate::references::wiimote_livree(&r)),
                None => {
                    let f = crate::emulateurs_profils::dossier_du_profil(d, &profil).join("User").join("Config").join("WiimoteNew.ini");
                    match std::fs::metadata(&f) {
                        Ok(m) if m.len() > 8 * 1024 * 1024 => None,
                        Ok(_) => std::fs::read_to_string(&f).ok().and_then(|t| crate::references::wiimote_en_place(&t)),
                        Err(e) if !en_partie && e.kind() == std::io::ErrorKind::NotFound => crate::references::par_defaut(&plateforme_wii)
                            .and_then(|(e, g, n)| crate::references::trouver(&magasin, e, g, n))
                            .and_then(|r| crate::references::wiimote_livree(&r)),
                        Err(_) => None,
                    }
                }
            },
            _ => None,
        };
        // DOSBox : deux manettes = deux joysticks à 2 boutons (mapper.cpp `QueryJoysticks`).
        let plusieurs_manettes = dosbox && crate::manettes::nombre_de_manettes() > 1;
        // Eden et sa famille : Frogtend ne lie la manette qu'à une manette BRANCHÉE au lancement (XInput ou PlayStation : son GUID,
        // `manettes::regler`) ; sans elle, le joueur 1 reste au clavier. Pendant la partie, le fichier le dit ; avant,
        // un joueur 1 encore au clavier sans manette Xbox branchée le restera.
        let au_clavier = match (e2.as_deref(), d2.as_deref()) {
            (Some(e), Some(d)) if crate::emulateurs::famille_eden(e) && crate::eden::portable(d) => {
                let encore_au_clavier = std::fs::read_to_string(crate::eden::fichier_reglages(d)).map_or(true, |t| crate::eden::joueur1_au_clavier(&t));
                encore_au_clavier && (en_partie || crate::manettes::guid_manette_pour_eden().is_none())
            }
            // BigPEmu lie seul une manette branchée au lancement (au premier, ou ensuite par `AutoAssign` que Frogtend
            // pose) ; sans manette Xbox branchée, le joueur 1 reste au clavier. (Pendant la partie, son fichier peut dater
            // d'avant le lancement : seule la manette branchée compte.)
            (Some("bigpemu"), Some(d)) => {
                use crate::bigpemu::Joueur1;
                let j = crate::bigpemu::joueur1(&crate::emulateurs_profils::dossier_du_profil(d, &profil));
                matches!(j, Joueur1::AuClavier | Joueur1::PasEncore) && crate::manettes::premiere_manette_xinput().is_none()
            }
            // Ryubing : la manette n'est posée que pour celle déjà relevée (`ryubing::MANETTE_RELEVEE`), branchée au
            // lancement ; sinon le joueur 1 reste au clavier. Pendant la partie, le fichier le dit.
            (Some("ryubing"), Some(d)) => {
                let au_clavier = crate::ryubing::joueur1_lu(&crate::ryubing::racine(d, &profil)) == crate::ryubing::Joueur1Lu::AuClavier;
                au_clavier && (en_partie || crate::manettes::id_manette_xinput_ryubing().as_deref() != Some(crate::ryubing::MANETTE_RELEVEE))
            }
            _ => false,
        };
        let famille = match crate::manettes::manette_branchee() {
            Some(crate::manettes::PadRpcs3::Ds4) | Some(crate::manettes::PadRpcs3::DualSense) => "playstation",
            _ => "xbox",
        };
        let personnalise = match (e2.as_deref(), d2.as_deref()) {
            (Some("retroarch"), Some(d)) => c2.as_deref().is_some_and(|c| {
                let noms: Vec<&str> = rom.iter().chain(contenu.iter()).map(|s| s.as_str()).collect();
                crate::emulateurs::remappage_retroarch(d, c, &noms)
            }),
            (Some("dolphin"), _) if !gamecube => false,
            (Some("mame"), Some(d)) => !crate::manettes::mame_disposition_d_origine(d, &crate::emulateurs_profils::dossier_du_profil(d, &profil), jeu_mame.as_deref()),
            (Some(e), Some(d)) => {
                let dossier_profil = crate::emulateurs_profils::dossier_du_profil(d, &profil);
                !crate::manettes::disposition_de_frogtend(e, d, Some(&dossier_profil))
            }
            _ => false,
        };
        (famille, personnalise && !au_clavier, plusieurs_manettes, au_clavier, wiimote)
    })
    .await
    .map_err(|_| Erreur::Disque("La lecture des boutons s'est arrêtée brutalement.".into()))?;
    let inconnue = match (emulateur.as_deref(), dossier.as_deref()) {
        (Some("cemu"), Some(d)) => !crate::cemu::portable(d),
        (Some("vita3k"), Some(d)) => !crate::vita3k::portable(d),
        // MAME et DOSBox : seulement lancés par l'émulateur réglé (un lanceur du jeu, un DOSBox livré avec lui : on ne
        // sait pas lequel tourne). MAME sur un autre pilote de manette ; DOSBox non portable (sa configuration est
        // ailleurs, ou un autre DOSBox nommé dosbox.exe), ou plusieurs manettes (deux joysticks à 2 boutons).
        (Some("mame"), Some(_)) => !par_l_emulateur || mame_autre_pilote,
        (Some("dosbox-staging"), Some(d)) => !par_l_emulateur || !d.join("dosbox-staging.conf").is_file() || plusieurs_manettes,
        (Some(e), Some(d)) if crate::emulateurs::famille_eden(e) => !crate::eden::portable(d),
        // Ryubing : pas portable (ses réglages sont ailleurs), ou des réglages propres à un jeu (leur manette passe
        // devant pour ce jeu, Migration.cs : `InputConfig` lu du fichier du jeu) — on ne sait pas lequel tourne.
        (Some("ryubing"), Some(d)) => {
            !crate::ryubing::portable(d) || profil_ouvert(&noyau).await.ok().is_none_or(|p| !crate::ryubing::configs_par_jeu(&crate::ryubing::racine(d, &p.nom)).is_empty())
        }
        _ => sans_nom,
    };
    // Ryubing : une manette XInput branchée qui n'est pas celle déjà relevée (045e:02ff) n'est jamais posée — le jeu se
    // jouera au clavier (`ryubing::MANETTE_RELEVEE`). Joueur 1 au clavier : on le dit ainsi ; joueur 1 sur la manette de
    // Frogtend : le lancement la retirera, « au clavier ».
    let (mut au_clavier, mut manette_pas_connue) = (au_clavier, false);
    if let (Some("ryubing"), Some(d), Ok(p)) = (emulateur.as_deref(), dossier.as_deref(), profil_ouvert(&noyau).await) {
        let racine = crate::ryubing::racine(d, &p.nom);
        let inconnue_branchee = tauri::async_runtime::spawn_blocking(|| crate::manettes::id_manette_xinput_ryubing().is_some_and(|m| m != crate::ryubing::MANETTE_RELEVEE))
            .await
            .unwrap_or(false);
        match crate::ryubing::joueur1_lu(&racine) {
            crate::ryubing::Joueur1Lu::AuClavier if inconnue_branchee => (au_clavier, manette_pas_connue) = (false, true),
            crate::ryubing::Joueur1Lu::DeFrogtend if inconnue_branchee => au_clavier = true,
            _ => {}
        }
    }
    Ok(Some(InfosBoutons {
        plateforme,
        emulateur,
        coeur,
        famille,
        personnalise: personnalise && !au_clavier && !manette_pas_connue,
        inconnue,
        au_clavier,
        manette_pas_connue,
        wiimote,
        clavier: clavier.filter(|c| !c.is_empty()),
        raccourcis,
    }))
}

/// L'écran « Les boutons » du menu en jeu : la fenêtre s'agrandit (jusqu'à 1 100 × 820, 90 % de l'écran au plus) pour
/// que le dessin se lise (essai de Seb, 29/10 : « trop petite et peu visible »), puis reprend sa taille. En Taodbox
/// (plein écran) : rien à faire.
#[tauri::command]
pub fn menu_jeu_agrandir(app: AppHandle, grand: bool) {
    let Some(w) = app.get_webview_window(FENETRE_MENU) else { return };
    // En Taodbox, la fenêtre est déjà grande (à l'échelle de la télé) : rien à changer.
    if w.is_fullscreen().unwrap_or(false) || taodbox_actif(&app) {
        return;
    }
    let (mut l, mut h) = if grand { (1100.0, 820.0) } else { (560.0, 700.0) };
    if let Ok(Some(m)) = w.current_monitor() {
        let s = m.size().to_logical::<f64>(m.scale_factor());
        l = f64::min(l, s.width * 0.9);
        h = f64::min(h, s.height * 0.9);
    }
    let _ = w.set_size(tauri::LogicalSize::new(l, h));
    let _ = w.center();
}

/// Change l'emplacement de sauvegarde rapide choisi dans le menu (−1 ou +1). Rien n'est envoyé à l'émulateur : il
/// est amené au bon emplacement au moment de sauver.
#[tauri::command]
pub fn menu_jeu_emplacement(menu: State<'_, crate::menu_jeu::MenuJeu>, sens: i8) -> Option<u8> {
    menu.changer_emplacement(sens.signum())
}

/// Les sauvegardes rapides du jeu en cours, par emplacement : (numéro, date) — `None` : vide.
#[tauri::command]
pub async fn menu_jeu_emplacements(app: AppHandle, noyau: State<'_, Noyau>) -> Resultat<Vec<(u8, Option<u64>)>> {
    let Some(p) = app.state::<crate::menu_jeu::MenuJeu>().partie() else { return Ok(vec![]) };
    match p.emulateur.as_deref() {
        Some("retroarch") => Ok(match (p.etats.as_deref(), p.nom_rom.as_deref()) {
            (Some(d), Some(r)) => crate::menu_jeu::emplacements_retroarch(std::path::Path::new(d), r),
            _ => vec![],
        }),
        Some("mame") => {
            let Some(d) = p.dossier_emulateur.as_deref().map(std::path::PathBuf::from) else { return Ok(vec![]) };
            let Some((nom, _)) = crate::pilotage::jeu_mame(&d) else { return Ok(vec![]) };
            let etats = crate::pilotage::dossier_etats_mame(&d, &profil_ouvert(&noyau).await?.nom, &nom);
            Ok((0..=crate::menu_jeu::EMPLACEMENT_MAX).map(|n| (n, crate::menu_jeu::date_de(&etats.join(crate::pilotage::nom_etat_mame(n))))).collect())
        }
        Some("bigpemu") => Ok(p.etats_bigpemu.map(|e| e.emplacements()).unwrap_or_default()),
        _ => Ok(vec![]),
    }
}

/// Un jeu lancé par BigPEmu a-t-il une triche cochée pour la personne du profil (un choix du menu en attente
/// compris) ? Lecture seule : pour le rappel sous ▶ Jouer (sa fenêtre de triches attend A puis B au démarrage).
#[tauri::command]
pub async fn bigpemu_triches_cochees(app: AppHandle, noyau: State<'_, Noyau>, id: i64) -> Resultat<bool> {
    let Some(jeu) = noyau.registre().jeu(id)? else { return Ok(false) };
    if jeu.installation.as_ref().is_some_and(|i| i.boutique.is_some()) {
        return Ok(false);
    }
    let Some((programme, _)) = emulateur_regle(&app, &jeu.plateforme, id, None) else { return Ok(false) };
    let programme = std::path::PathBuf::from(programme);
    let est_bigpemu = programme.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.to_lowercase().starts_with("bigpemu"));
    let Some(d) = programme.parent().filter(|_| est_bigpemu).map(std::path::PathBuf::from) else { return Ok(false) };
    let Some(fichier) = noyau.fichier_pour_l_emulateur(id, None).await.ok().flatten() else { return Ok(false) };
    let profil = profil_ouvert(&noyau).await?.nom;
    Ok(tauri::async_runtime::spawn_blocking(move || crate::bigpemu::triches_cochees(&crate::emulateurs_profils::dossier_du_profil(&d, &profil), &fichier))
        .await
        .unwrap_or(false))
}

/// MAME : sauver, charger ou recommencer par son script Lua (lot 0.79.0). La commande part AVANT que le menu se
/// ferme (sinon le jeu tournerait quelques images avant la sauvegarde) ; la sauvegarde rapide est celle du PROFIL
/// (`Profils\<profil>\sta\<jeu>\frogtend.sta`), écrite sous un autre nom puis mise à la place une fois complète.
async fn action_mame(app: &AppHandle, noyau: &Noyau, dossier: Option<&str>, a: crate::pilotage::Action, emplacement: u8) -> Resultat<()> {
    use crate::pilotage::Action;
    let dossier = std::path::PathBuf::from(dossier.ok_or_else(|| Erreur::Refus("MAME est introuvable.".into()))?);
    let (nom, sauve) = crate::pilotage::jeu_mame(&dossier).ok_or_else(|| Erreur::Refus("MAME n'a pas encore dit quel jeu tourne : réessaie dans un instant.".into()))?;
    let profil = profil_ouvert(noyau).await?;
    let etats = crate::pilotage::dossier_etats_mame(&dossier, &profil.nom, &nom);
    if matches!(a, Action::SauverEtat | Action::ChargerEtat) && !sauve {
        return Err(Erreur::Refus("Ce jeu ne gère pas bien les sauvegardes : Frogtend ne les propose pas.".into()));
    }
    let menu = app.state::<crate::menu_jeu::MenuJeu>();
    let _seule = if matches!(a, Action::SauverEtat | Action::ChargerEtat) {
        Some(menu.prendre_etat().ok_or_else(|| Erreur::Refus("Une sauvegarde rapide est déjà en cours.".into()))?)
    } else {
        None
    };
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
                let etat = e.join(crate::pilotage::nom_etat_mame(emplacement));
                if !etat.is_file() {
                    return Err(Erreur::Refus(format!("Aucune sauvegarde dans l'emplacement {} : sauve d'abord.", emplacement + 1)));
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
        let r = tauri::async_runtime::spawn_blocking(move || crate::pilotage::garder_etat_mame(&etats, std::time::Duration::from_secs(5), emplacement))
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
    let famille_eden = p.emulateur.as_deref().is_some_and(crate::emulateurs::famille_eden);
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
        if famille_eden {
            // Eden et sa famille : arrêtée seulement figée depuis 20 s ET sans plus rien écrire (`finir_la_partie_eden`) ;
            // au moins 5 s, au plus 5 min.
            let minimum = std::time::Duration::from_secs(delai.max(5));
            let app3 = app.clone();
            let app4 = app.clone();
            std::thread::spawn(move || {
                // Le menu rouvert pendant l'attente (Eden gelé par Frogtend, ou en pause derrière lui) : jamais compté.
                let en_pause = move || crate::figer::est_fige() || app3.state::<crate::menu_jeu::MenuJeu>().ouvert.load(std::sync::atomic::Ordering::SeqCst);
                let fin = crate::lancement::finir_la_partie_eden(
                    racine,
                    std::path::Path::new(&dossier),
                    minimum,
                    std::time::Duration::from_secs(300),
                    crate::menu_jeu::fenetres_figees,
                    en_pause,
                    || {
                        if app4.state::<crate::menu_jeu::MenuJeu>().annuler_relance() {
                            let _ = app4.emit("menu-relance-annulee", ());
                        }
                    },
                );
                let n = app2.state::<Noyau>();
                match fin {
                    crate::lancement::FinEden::Termine(termines, fige, ecrits) => n.journaliser(&format!(
                        "quitter : l'émulateur restait figé ({fige} s sans répondre, {ecrits} octets écrits pendant l'attente, plus rien depuis 10 s) : {termines} processus terminé(s)"
                    )),
                    crate::lancement::FinEden::Laisse => {
                        n.journaliser("quitter : l'émulateur est encore ouvert après 5 min (une fenêtre répondait, ou il écrivait encore) : laissé tel quel")
                    }
                    crate::lancement::FinEden::Ferme => {}
                }
            });
            return Ok(n);
        }
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

/// « Relancer » (✅ Seb 15/11) : une sauvegarde rapide d'abord si demandée (seulement là où elle est vérifiée,
/// `EtatMenu::sauver_avant`), puis Quitter ; la fin de CETTE partie portera « relancer » et la fenêtre principale
/// relance le jeu tel qu'il avait été lancé. `jeu` : celui que le menu montre (une autre partie : refusé).
#[tauri::command]
pub async fn menu_jeu_relancer(app: AppHandle, noyau: State<'_, Noyau>, jeu: i64, sauver: bool, recharger: bool) -> Resultat<usize> {
    let menu = app.state::<crate::menu_jeu::MenuJeu>();
    let numero = menu.numero.load(std::sync::atomic::Ordering::SeqCst);
    let p = menu.partie().filter(|_| menu.en_cours(numero)).ok_or_else(|| Erreur::Introuvable("Aucune partie en cours.".into()))?;
    if p.jeu != jeu {
        return Err(Erreur::Refus("Ce n'est plus la même partie.".into()));
    }
    if p.emulateur.is_none() {
        return Err(Erreur::Refus("Ce jeu ne se relance pas depuis le menu : quitte-le, puis relance-le.".into()));
    }
    if sauver {
        if !crate::menu_jeu::etat(&p).sauver_avant {
            return Err(Erreur::Refus("Cet émulateur ne permet pas de vérifier la sauvegarde avant de relancer.".into()));
        }
        menu_jeu_action(app.clone(), noyau.clone(), "sauver".into()).await?;
        // La sauvegarde a réussi : la partie relancée la recharge dès qu'elle est prête (jamais pour Recommencer).
        if recharger {
            let contenu = match p.emulateur.as_deref() {
                Some("retroarch") => p.nom_rom.clone().unwrap_or_default(),
                Some("bigpemu") => p.empreinte.clone().unwrap_or_default(),
                _ => String::new(),
            };
            *menu.charger_apres.lock().unwrap_or_else(|e| e.into_inner()) = Some(crate::menu_jeu::ChargerApres {
                jeu,
                emulateur: p.emulateur.clone().unwrap_or_default(),
                emplacement: p.emplacement,
                contenu,
                limite: std::time::Instant::now() + std::time::Duration::from_secs(360),
            });
        }
    }
    // La demande vaut pour CETTE partie, le temps de la fermer (au plus 6 min : l'arrêt de sécurité d'Eden en prend 5).
    *menu.relance.lock().unwrap_or_else(|e| e.into_inner()) = Some((numero, std::time::Instant::now() + std::time::Duration::from_secs(360)));
    noyau.journaliser(&format!("relancer : jeu {jeu}{}", if sauver { " (sauvegarde rapide faite)" } else { "" }));
    let n = menu_jeu_quitter(app.clone()).await;
    if !matches!(n, Ok(k) if k > 0) {
        menu.annuler_relance();
    }
    n
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
    let (base, cible) = match deja_installe(&noyau, &id) {
        // Une mise à jour à sa place (voir `emulateur_installer`).
        Some(c) => (c.parent().map(std::path::Path::to_path_buf).unwrap_or_else(|| c.clone()), c),
        None => {
            let base = std::path::PathBuf::from(&dossier);
            // Le dossier réglé d'abord ; s'il n'existe pas encore (le dossier proposé par « Prêt à jouer »), il est créé.
            exiger_dossier_de_base(&app, &base)?;
            let cible = base.join(crate::jeux_pc::nom_de_dossier(&nom));
            (base, cible)
        }
    };
    // Une seule installation à la fois dans ce dossier, jamais par-dessus le programme ouvert.
    let _occupe = occuper_installation(&base, &cible, &nom)?;
    if !base.is_dir() {
        std::fs::create_dir_all(&base).map_err(|e| Erreur::Disque(format!("Dossier impossible à créer : {} ({e}).", base.display())))?;
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
    // La taille annoncée par Firehouse (celle de l'accord) : un paquet d'une autre taille n'est jamais installé (sans
    // empreinte — BigPEmu, Ryubing — c'est la seule vérification possible).
    if let Some(annoncee) = p["taille"].as_u64().filter(|t| *t > 0) {
        let recue = std::fs::metadata(&paquet).map(|m| m.len()).unwrap_or(0);
        if recue != annoncee {
            let _ = std::fs::remove_file(&paquet);
            return Err(Erreur::Conflit(format!("Le paquet de {nom} n'a pas la taille annoncée ({recue} octets au lieu de {annoncee}) : rien n'est installé.")));
        }
    }
    if let Some(attendue) = p["sha256"].as_str().filter(|s| !s.is_empty()) {
        if crate::sauvegarde::empreinte(&paquet)?.to_lowercase() != attendue.to_lowercase() {
            let _ = std::fs::remove_file(&paquet);
            return Err(Erreur::Conflit(format!("Le paquet de {nom} est arrivé abîmé (empreinte différente) : rien n'est installé.")));
        }
    }
    let programme = p["programme"].as_str().map(String::from);
    let mut portable = (p["portable"]["type"].as_str().map(String::from), p["portable"]["nom"].as_str().map(String::from));
    // Un émulateur du catalogue qui EXIGE le mode portable (Eglantine : `user\`) l'a toujours, même si le paquet de
    // Firehouse ne le dit pas : sinon il écrirait dans %APPDATA%\eden, chez Eden (relecture lancement 08/11).
    if portable.0.is_none() {
        if let Some(dossier) = crate::emulateurs::fiche(&id).and_then(|f| f.portable).and_then(|x| x.strip_suffix('/')) {
            portable = (Some("dossier".into()), Some(dossier.into()));
        }
    }
    // La mise à jour d'Eden ou d'un dérivé (toutes ses données dans `user\`) : la nouvelle version REMPLACE l'ancienne
    // (fichiers d'avant absents du paquet mis à l'abri, `user\` jamais touché) — Eglantine change de chaîne de
    // compilation au contrat 1.40 (MSVC → MinGW) : ses anciennes DLL ne restent pas mêlées aux nouvelles.
    // Seulement un émulateur que Frogtend a installé là (jamais un dossier trouvé ou installé à la main).
    let remplacer = deja_installe(&noyau, &id).is_some() && crate::emulateurs::famille_eden(&id);
    // BigPEmu : ses données sont un LIEN vers le profil de la dernière personne (`UserData\`) ; un paquet qui en
    // porterait serait copié AU TRAVERS : refusé avant toute copie, comme pour Eden (`deposer_archive`).
    let remplacer = remplacer || (id == "bigpemu" && deja_installe(&noyau, &id).is_some());
    // Ryubing (lot 1, relecture lancement 15/11) : toutes ses données dans `portable\` ; la nouvelle version REMPLACE
    // l'ancienne (ancien programme à l'abri, jamais de DLL mêlées), un paquet qui porterait `portable/` est refusé.
    let remplacer = remplacer || (id == "ryubing" && deja_installe(&noyau, &id).is_some());
    let (pq, c, n, id_install) = (paquet.clone(), cible.clone(), nom.clone(), id.clone());
    let r = tauri::async_runtime::spawn_blocking(move || {
        let port = match (&portable.0, &portable.1) {
            (Some(t), Some(n)) => Some((t.as_str(), n.as_str())),
            _ => None,
        };
        // `user\` : ses données ; `Profils\` : celles de Frogtend (qui est qui dans Eden : `.utilisateurs-frogtend`).
        let garder: &[&str] = match id_install.as_str() {
            "bigpemu" => &["UserData", "Profils"],
            "ryubing" => &["portable", "Profils"],
            _ => &["user", "Profils"],
        };
        crate::emulateurs::installer_paquet_decrit(&n, &pq, &c, programme.as_deref(), port, remplacer.then_some(garder))
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
    /// Connecté par la page officielle de Steam (lot C1) : son `steamid` est connu.
    pub connecte: bool,
    /// La clé d'API rangée est celle d'un AUTRE compte que celui connecté (l'import par la clé est refusé).
    pub cle_autre_compte: bool,
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
        compte: c.as_ref().map(|c| c.compte.clone()).filter(|c| !c.is_empty()),
        cle_enregistree: cle,
        nb_jeux: c.as_ref().map_or(0, |c| c.jeux.len()),
        maj_le: c.as_ref().map(|c| c.maj_le.clone()).filter(|m| !m.is_empty()),
        steam_installe: crate::boutiques::dossier_steam().is_some(),
        connecte: c.as_ref().is_some_and(crate::boutiques::connecte_par_la_page),
        cle_autre_compte: cle && c.as_ref().is_some_and(crate::boutiques::cle_d_un_autre_compte),
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
    // Relu et écrit sous le verrou : une connexion par la page pendant l'attente du réseau n'est pas défaite en silence.
    let mut change = false;
    crate::boutiques::modifier_source(&dossier, "steam", |c| {
        let mut c = c.unwrap_or_default();
        let avant = crate::boutiques::compte_steam_d_avant(&c).to_string();
        change = !avant.is_empty() && avant != steamid;
        if change {
            c.menage_ancien_compte = true; // fait juste après, ou à la prochaine ouverture du profil
        }
        c.steamid_precedent.clear();
        if c.steamid != steamid {
            c.jeux.clear(); // un autre compte : l'ancienne liste ne vaut plus
            c.connecte_le.clear(); // ni la connexion par la page (elle était celle de l'autre compte)
        }
        c.compte = compte.trim().to_string();
        c.steamid = steamid.clone();
        c.steamid_cle = steamid;
        Ok(Some(c))
    })?;
    noyau.journaliser(&format!("compte Steam réglé pour le profil {}", p.id));
    // Un AUTRE compte (Seb 20/11) : les jeux de l'ancien jamais installés ni joués quittent la ludothèque.
    if change {
        menage_ancien_compte_steam(&noyau, &p.id).await;
    }
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

/// Oublie le compte Steam du profil : la clé quitte le coffre et le compte (steamid, connexion, liste importée) quitte
/// `boutiques.json` (seul `steamid_precedent` reste, pour voir un changement de compte ensuite). Les CARTES de la ludothèque restent toutes (décision de Seb du 20/11 : se connecter vaut accord,
/// et les jeux restent même après une déconnexion ; avant, toutes les cartes Steam étaient effacées).
#[tauri::command]
pub async fn boutique_steam_oublier(app: AppHandle, noyau: State<'_, Noyau>) -> Resultat<OubliSteam> {
    let p = profil_ouvert(&noyau).await?;
    // Les refus D'ABORD (lot C3) : rien n'est oublié si la déconnexion du navigateur ne peut pas même commencer.
    if app.get_webview_window(FENETRE_STEAM).is_some() {
        return Err(Erreur::Refus("Ferme d'abord la fenêtre de connexion à Steam, puis réessaie.".into()));
    }
    // Une lecture du compte en cours (import à l'ouverture) : on attend qu'elle finisse (30 s au plus, sa propre limite).
    for _ in 0..160 {
        if app.get_webview_window(FENETRE_STEAM_JETON).is_none() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    if app.get_webview_window(FENETRE_STEAM_JETON).is_some() {
        return Err(Erreur::Refus("Frogtend lit ton compte Steam en ce moment : attends quelques secondes, puis réessaie.".into()));
    }
    // Pendant l'oubli, aucune connexion ne s'ouvre (elle annoncerait l'ancien compte, encore dans les cookies).
    if OUBLI_STEAM_EN_COURS.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return Err(Erreur::Refus("L'oubli de ton compte Steam est déjà en cours.".into()));
    }
    struct Fin;
    impl Drop for Fin {
        fn drop(&mut self) {
            OUBLI_STEAM_EN_COURS.store(false, std::sync::atomic::Ordering::SeqCst);
        }
    }
    let _fin = Fin;
    oublier_compte_steam(&noyau, &p.id)?;
    // Puis le navigateur du profil : seuls les cookies de Steam (Epic et PlayStation y restent connectés).
    let navigateur_deconnecte = match deconnecter_navigateur_steam(&app, &noyau, &dossier_profil_de(&noyau, &p.id)).await {
        Ok(v) => v,
        Err(e) => {
            noyau.journaliser(&format!("oubli Steam : déconnexion du navigateur impossible ({e})"));
            false
        }
    };
    Ok(OubliSteam { navigateur_deconnecte })
}

/// Lot C3 : un oubli de compte Steam efface les cookies en ce moment (`boutique_steam_connexion` refuse alors).
static OUBLI_STEAM_EN_COURS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Ce que « Oublier ma connexion Steam » a fait (lot C3) : le compte est TOUJOURS oublié côté Frogtend ; la
/// déconnexion du navigateur du profil n'est vraie que PROUVÉE (relue : plus aucun cookie de connexion Steam).
#[derive(Debug, Serialize, PartialEq)]
pub struct OubliSteam {
    pub navigateur_deconnecte: bool,
}

/// Lot C3 : efface les SEULS cookies de Steam du navigateur du profil (`domaine_steam`), pour qu'une connexion suivante
/// demande vraiment un compte. Jamais le dossier entier ni `clear_all_browsing_data` : Epic et PlayStation y vivent.
/// Une fenêtre CACHÉE sur `about:blank` (la boutique renouvellerait sa connexion pendant l'effacement), sous l'étiquette
/// de la lecture du jeton (Tauri refuse une seconde fenêtre de même étiquette : jamais les deux à la fois). L'effacement
/// passe par le gestionnaire de cookies de WebView2 (le `delete_cookie` de tauri perd le point de tête du domaine) et
/// n'attend pas de réponse : seule la RELECTURE prouve (3 passes au plus). Jamais une valeur lue, journalisée ni rendue.
async fn deconnecter_navigateur_steam(app: &AppHandle, noyau: &Noyau, dossier: &std::path::Path) -> Resultat<bool> {
    use crate::boutiques::{connexion_steam_effacee, domaine_steam, resume_cookies_steam_restants, CookieSansValeur};
    // Pas de navigateur : rien à effacer (et ouvrir une fenêtre le CRÉERAIT).
    if !crate::gratuits::dossier_navigateur(dossier).is_dir() {
        noyau.journaliser("oubli Steam : pas de navigateur pour ce profil, rien à effacer");
        return Ok(true);
    }
    let vide = tauri::Url::parse("about:blank").map_err(|_| Erreur::Refus("Adresse invalide.".into()))?;
    let w = tauri::WebviewWindowBuilder::new(app, FENETRE_STEAM_JETON, tauri::WebviewUrl::External(vide))
        .title("Steam — Frogtend")
        .inner_size(400.0, 300.0)
        .visible(false)
        .focused(false)
        .skip_taskbar(true)
        .data_directory(crate::gratuits::dossier_navigateur(dossier))
        .on_new_window(crate::gratuits::fenetre_d_automatisation)
        .on_navigation(|u| u.as_str() == "about:blank")
        .build()
        .map_err(|_| Erreur::Refus("Frogtend lit ton compte Steam en ce moment : attends quelques secondes, puis réessaie.".into()))?;
    // `cookies()` se bloque sur le fil principal sous Windows (documentation de tauri 2.12) : dans un fil à part.
    let lire = |w: tauri::WebviewWindow| async move {
        tauri::async_runtime::spawn_blocking(move || {
            w.cookies().map(|l| {
                l.iter()
                    .map(|c| CookieSansValeur {
                        nom: c.name().to_string(),
                        domaine: c.domain().unwrap_or("").to_string(),
                        chemin: c.path().filter(|p| p.starts_with('/')).unwrap_or("/").to_string(),
                    })
                    .collect::<Vec<_>>()
            })
        })
        .await
        .ok()
        .and_then(|r| r.ok())
    };
    let mut prouve = false;
    let mut resume = "cookies illisibles".to_string();
    for passe in 0..4 {
        let Some(lus) = lire(w.clone()).await else { break };
        resume = resume_cookies_steam_restants(&lus);
        if connexion_steam_effacee(&lus) {
            prouve = true;
            break;
        }
        if passe == 3 {
            break;
        }
        let a_effacer: Vec<CookieSansValeur> = lus.into_iter().filter(|c| domaine_steam(&c.domaine)).collect();
        let _ = w.with_webview(move |pw| effacer_cookies_webview2(&pw, &a_effacer));
        tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    }
    let _ = w.close();
    noyau.journaliser(&format!("oubli Steam : navigateur {} ({resume} restant)", if prouve { "déconnecté de Steam" } else { "PAS déconnecté de Steam" }));
    Ok(prouve)
}

/// Efface ces cookies par le gestionnaire de cookies de WebView2 (fil principal, appelé par `with_webview`) : par nom
/// et adresse (couvre `x.com` et `.x.com`), puis par nom, domaine et chemin exacts, avec et sans point de tête.
#[cfg(windows)]
fn effacer_cookies_webview2(pw: &tauri::webview::PlatformWebview, cookies: &[crate::boutiques::CookieSansValeur]) {
    use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2_2;
    use windows_core_wv2::{Interface, HSTRING};
    // SAFETY : appels COM sur le fil principal de la WebView (garanti par `with_webview`), sur des interfaces valides.
    unsafe {
        let Ok(coeur) = pw.controller().CoreWebView2() else { return };
        let Ok(c2) = coeur.cast::<ICoreWebView2_2>() else { return };
        let Ok(gestionnaire) = c2.CookieManager() else { return };
        for c in cookies {
            let nom = HSTRING::from(c.nom.as_str());
            let chemin = HSTRING::from(c.chemin.as_str());
            let hote = c.domaine.trim_start_matches('.');
            let _ = gestionnaire.DeleteCookies(&nom, &HSTRING::from(format!("https://{hote}{}", c.chemin)));
            let _ = gestionnaire.DeleteCookiesWithDomainAndPath(&nom, &HSTRING::from(hote), &chemin);
            let _ = gestionnaire.DeleteCookiesWithDomainAndPath(&nom, &HSTRING::from(format!(".{hote}")), &chemin);
        }
    }
}

#[cfg(not(windows))]
fn effacer_cookies_webview2(_pw: &tauri::webview::PlatformWebview, _cookies: &[crate::boutiques::CookieSansValeur]) {}

/// Le corps de `boutique_steam_oublier` (testé) : jamais une carte de la ludothèque n'est touchée.
pub(crate) fn oublier_compte_steam(noyau: &Noyau, profil: &str) -> Resultat<()> {
    noyau.coffre.oublier(&crate::boutiques::nom_secret(profil, "steam"))?;
    // Seul le compte d'avant reste (ni clé, ni connexion, ni liste) : une connexion à un AUTRE compte ensuite est bien
    // un changement de compte (Seb 20/11).
    crate::boutiques::modifier_source(&dossier_profil_de(noyau, profil), "steam", |c| Ok(crate::boutiques::compte_oublie(c)))?;
    noyau.journaliser(&format!("compte Steam oublié pour le profil {profil} (ses jeux restent dans la ludothèque)"));
    Ok(())
}

/// Les jeux Steam du profil (liste de son compte, s'il est relié) ET ceux installés sur CE PC (relus à chaque fois :
/// lot A, un profil SANS clé d'API voit ses jeux Steam installés).
#[tauri::command]
pub async fn boutique_steam_jeux(noyau: State<'_, Noyau>) -> Resultat<Vec<crate::boutiques::JeuBoutique>> {
    // Une relecture à la fois ; la session est prise AU DÉBUT (un profil changé entre-temps ne reçoit rien).
    let _un_seul = RELECTURE.lock().await;
    let s = noyau.session().await?;
    let api = crate::boutiques::lire(&dossier_profil_de(&noyau, &s.profil.id)).map(|c| c.jeux).unwrap_or_default();
    let locaux = tauri::async_runtime::spawn_blocking(|| crate::boutiques_pc::lire_steam(crate::boutiques::dossier_steam().as_deref()))
        .await
        .map_err(|_| Erreur::Disque("La lecture de Steam s'est arrêtée brutalement.".into()))?;
    let Some(locaux) = locaux else { return Ok(api) };
    let relie = !api.is_empty();
    let l = crate::boutiques_pc::fusion_steam(api, &locaux);
    synchroniser_registre(&noyau, "steam", &locaux);
    // L'état « installé » de la ludothèque du profil suit le disque (filtre « Installés », bouton Steam). Sans compte
    // relié, les cartes ne sont rangées que par l'import automatique (`boutiques_relire_pc`), si le profil l'a gardé.
    if relie {
        if let Err(e) = noyau.ranger_installes(&s, "steam", &l) {
            noyau.journaliser(&format!("ludothèque Steam : {e:?}"));
        }
    }
    Ok(l)
}

/// Les jeux installés d'une boutique deviennent des jeux du PC (lancés par la boutique, parties suivies) ; un jeu
/// désinstallé n'en sort que si son dossier a disparu, sans temps de jeu (`synchroniser_boutique`). Un échec est noté.
fn synchroniser_registre(noyau: &Noyau, boutique: &str, installes: &[crate::boutiques::JeuBoutique]) {
    let prefixe = format!("{boutique}_");
    let pc: std::collections::BTreeSet<String> =
        installes.iter().filter(|j| j.installe).map(|j| if boutique == "steam" { j.id.clone() } else { j.id.strip_prefix(&prefixe).unwrap_or(&j.id).to_string() }).collect();
    match noyau.synchroniser_boutique(boutique, installes, &pc) {
        Ok((a, r)) if a + r > 0 => noyau.journaliser(&format!("jeux {} du PC : {a} ajouté(s), {r} retiré(s)", crate::boutiques_pc::nom_boutique(boutique))),
        Ok(_) => {}
        Err(e) => noyau.journaliser(&format!("jeux {} du PC : synchronisation impossible ({e})", crate::boutiques_pc::nom_boutique(boutique))),
    }
}

/// Une boutique telle que ce PC la voit (page 🛒 Boutiques ; import automatique).
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct EtatBoutiquePc {
    pub boutique: String,
    pub nom: String,
    /// Le lanceur est là (Jouer et Installer possibles).
    pub lanceur: bool,
    /// Ses jeux installés ont pu être lus.
    pub lu: bool,
    pub installes: usize,
    /// Ses jeux POSSÉDÉS (installés compris), lus sans connexion (lot B) ; `None` : boutique sans cette lecture, ou
    /// cache absent ou illisible.
    pub possedes: Option<usize>,
}

/// Ce que la relecture du PC a donné : l'état de chaque boutique et les jeux NOUVEAUX dans la ludothèque du profil.
#[derive(Serialize, Clone, Debug, Default)]
pub struct ReleveBoutiques {
    pub boutiques: Vec<EtatBoutiquePc>,
    /// (boutique, nombre de jeux nouveaux, dont installés), seulement les boutiques qui en ont.
    pub nouveaux: Vec<(String, usize, usize)>,
}

/// Lit sur CE PC les jeux installés ET possédés des boutiques demandées (lecture seule ; `travail` : copies de bases,
/// effacées ensuite), et la présence de leurs lanceurs.
fn lire_pc_complet(boutiques: &[&'static str], travail: &std::path::Path) -> Vec<(&'static str, Option<Vec<crate::boutiques::JeuBoutique>>, Option<Vec<crate::boutiques::JeuBoutique>>, bool)> {
    let l = crate::boutiques_pc::lire_seulement(boutiques, travail);
    let _ = std::fs::remove_dir_all(travail);
    let mut possedes = crate::boutiques_pc::lire_possedes(boutiques);
    l.into_iter()
        .map(|(b, i)| {
            let p = possedes.iter_mut().find(|(x, _)| *x == b).and_then(|(_, p)| p.take());
            (b, i, p, crate::boutiques_pc::lanceur_present(b))
        })
        .collect()
}

/// Une seule relecture à la fois (deux profils ouverts coup sur coup, un import Galaxy pendant le démarrage…).
static RELECTURE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Lit les boutiques demandées sur CE PC (en tâche de fond), fait la partie « registre du PC » UNE fois, puis range
/// les cartes du profil ouvert. Un lanceur absent ou illisible ne touche à rien (jamais « zéro jeu »).
/// `possedes` : le profil a ACCEPTÉ que les jeux possédés des comptes ouverts sur ce PC entrent chez lui (✅ Seb 18/11 :
/// demandé une fois par profil) ; sinon seuls les jeux installés sont rangés (les possédés sont seulement comptés).
pub(crate) async fn relire_pc(noyau: &Noyau, seulement: &[&str], possedes: bool) -> Resultat<ReleveBoutiques> {
    let _un_seul = RELECTURE.lock().await;
    // La session AU DÉBUT : un profil changé pendant la lecture ne reçoit rien (ses cartes, ni la liste Steam d'un autre).
    let s = noyau.session().await?;
    let travail = noyau.dossier.join("travail").join("boutiques-pc");
    let boutiques: Vec<&'static str> = crate::boutiques_pc::BOUTIQUES_PC.iter().copied().filter(|b| seulement.contains(b)).collect();
    let lecture = tauri::async_runtime::spawn_blocking(move || lire_pc_complet(&boutiques, &travail))
        .await
        .map_err(|_| Erreur::Disque("La lecture des boutiques de ce PC s'est arrêtée brutalement.".into()))?;
    let accepte = possedes;
    let mut r = ReleveBoutiques::default();
    for (b, l, possedes, lanceur) in lecture {
        let mut etat =
            EtatBoutiquePc { boutique: b.into(), nom: crate::boutiques_pc::nom_boutique(b).into(), lanceur, lu: l.is_some(), installes: 0, possedes: possedes.as_ref().map(Vec::len) };
        let Some(installes) = l else {
            // Jeux installés illisibles : rien ne bouge (même les possédés : leur état serait faux).
            r.boutiques.push(etat);
            continue;
        };
        etat.installes = installes.len();
        synchroniser_registre(noyau, b, &installes);
        // Steam : la liste du compte du profil (s'il est relié) garde ses jeux pas installés ; Epic et Ubisoft : leurs
        // jeux possédés (lot B), une seule carte pour un jeu installé ET possédé.
        let cartes = if b == "steam" {
            crate::boutiques_pc::fusion_steam(crate::boutiques::lire(&dossier_profil_de(noyau, &s.profil.id)).map(|c| c.jeux).unwrap_or_default(), &installes)
        } else if b == "gog" {
            // Lot C4a : la liste du compte GOG du profil (lue par la page, si elle l'a été) garde ses jeux pas installés,
            // complétée par ce que GOG Galaxy en sait. Le registre du PC, lui, n'a reçu que les jeux installés (plus haut).
            cartes_gog(&dossier_profil_de(noyau, &s.profil.id), &installes)
        } else {
            cartes_pc(installes, possedes, accepte)
        };
        match noyau.ranger_installes_sans_reunir(&s, b, &cartes) {
            Ok(n) if !n.is_empty() => r.nouveaux.push((b.into(), n.len(), nouveaux_installes(b, &cartes, &n))),
            Ok(_) => {}
            Err(e) => noyau.journaliser(&format!("ludothèque {} : {e:?}", crate::boutiques_pc::nom_boutique(b))),
        }
        r.boutiques.push(etat);
    }
    // Les copies d'un même jeu réunies UNE fois, pour toutes les boutiques (seules les cartes qui changent s'écrivent).
    if let Err(e) = noyau.reunir_copies(&s) {
        noyau.journaliser(&format!("ludothèque : réunion des copies impossible ({e:?})"));
    }
    let total: usize = r.nouveaux.iter().map(|(_, n, _)| n).sum();
    if total > 0 {
        noyau.journaliser(&format!("import automatique : {total} jeu(x) nouveau(x) pour le profil {} ({:?})", s.profil.id, r.nouveaux));
    }
    Ok(r)
}

/// Parmi les cartes NOUVELLES (`nouveaux`, leurs ids), combien sont des jeux installés (le toast : « Epic 497 dont 1
/// installé »).
/// Les cartes d'une boutique (sauf Steam) : ses jeux installés, plus ses jeux possédés SI le profil l'a accepté.
/// Les cartes GOG d'un profil (lot C4a) : la liste de son compte GOG et ses jeux GOG installés sur ce PC, complétés par
/// la liste de GOG Galaxy du profil (`boutiques_pc::fusion_gog`).
/// Lot C4b : les jeux de TOUS les comptes GOG du profil, réunis (une carte par jeu) avant la fusion avec le PC.
pub(crate) fn cartes_gog(dossier_profil: &std::path::Path, installes: &[crate::boutiques::JeuBoutique]) -> Vec<crate::boutiques::JeuBoutique> {
    let compte = crate::boutiques::lire_source(dossier_profil, "gog").map(|c| crate::boutiques::jeux_gog_reunis(&crate::boutiques::comptes_gog_lus(&c))).unwrap_or_default();
    let galaxy = crate::boutiques::lire_source(dossier_profil, "galaxy").map(|c| c.jeux).unwrap_or_default();
    crate::boutiques_pc::fusion_gog(compte, installes, &galaxy)
}

pub(crate) fn cartes_pc(installes: Vec<crate::boutiques::JeuBoutique>, possedes: Option<Vec<crate::boutiques::JeuBoutique>>, accepte: bool) -> Vec<crate::boutiques::JeuBoutique> {
    crate::boutiques_pc::fusion_possedes(Some(installes), possedes.filter(|_| accepte)).unwrap_or_default()
}

pub(crate) fn nouveaux_installes(boutique: &str, cartes: &[crate::boutiques::JeuBoutique], nouveaux: &[i64]) -> usize {
    let installes: std::collections::BTreeSet<i64> = cartes.iter().filter(|j| j.installe).map(|j| crate::noyau::id_carte(boutique, j)).collect();
    nouveaux.iter().filter(|id| installes.contains(id)).count()
}

/// Import automatique (lot A, Seb 18/11) : à l'ouverture d'un profil, les jeux INSTALLÉS de toutes les boutiques de
/// ce PC entrent seuls dans sa ludothèque. Appelé sans attendre par l'interface (jamais un ralentissement).
#[tauri::command]
/// `possedes` : le profil a accepté ses jeux possédés (lot B) ; absent : non.
pub async fn boutiques_relire_pc(noyau: State<'_, Noyau>, possedes: Option<bool>) -> Resultat<ReleveBoutiques> {
    relire_pc(&noyau, &crate::boutiques_pc::BOUTIQUES_PC, possedes.unwrap_or(false)).await
}

/// Le profil REFUSE ses jeux possédés (✅ Seb 18/11) : ses cartes possédées sont retirées, seulement celles qui ne sont
/// pas installées, sans temps de jeu et sans ligne au registre du PC (jamais une carte jouée). Rend leur nombre.
#[tauri::command]
pub async fn boutiques_retirer_possedes(noyau: State<'_, Noyau>) -> Resultat<usize> {
    let _un_seul = RELECTURE.lock().await;
    let s = noyau.session().await?;
    let n = noyau.retirer_possedes(&s, &crate::boutiques_pc::BOUTIQUES_POSSEDES)?;
    if n > 0 {
        noyau.journaliser(&format!("jeux possédés refusés : {n} carte(s) retirée(s) pour le profil {}", s.profil.id));
    }
    Ok(n)
}

/// L'état des boutiques de ce PC (lecture seule, rien n'est rangé) : la page 🛒 Boutiques.
#[tauri::command]
pub async fn boutiques_etat_pc(noyau: State<'_, Noyau>) -> Resultat<Vec<EtatBoutiquePc>> {
    // Le même verrou que la relecture : jamais deux copies de bases dans le même dossier de travail.
    let _un_seul = RELECTURE.lock().await;
    let travail = noyau.dossier.join("travail").join("boutiques-etat");
    tauri::async_runtime::spawn_blocking(move || {
        lire_pc_complet(&crate::boutiques_pc::BOUTIQUES_PC, &travail)
            .into_iter()
            .map(|(b, l, p, lanceur)| EtatBoutiquePc {
                boutique: b.into(),
                nom: crate::boutiques_pc::nom_boutique(b).into(),
                lanceur,
                lu: l.is_some(),
                installes: l.map_or(0, |l| l.len()),
                possedes: p.map(|p| p.len()),
            })
            .collect()
    })
    .await
    .map_err(|_| Erreur::Disque("La lecture des boutiques de ce PC s'est arrêtée brutalement.".into()))
}

/// L'adresse officielle pour jouer à un jeu de boutique ou l'installer (clé de la carte : l'appid Steam, sinon
/// « <boutique>_<clé> »), le lanceur VÉRIFIÉ d'abord : un lanceur absent est dit, jamais un échec en silence.
pub(crate) fn adresse_carte(boutique: &str, cle_carte: &str, action: crate::boutiques_pc::Action) -> Resultat<String> {
    adresse_carte_selon(boutique, cle_carte, action, crate::boutiques_pc::lanceur_present)
}

/// `adresse_carte`, la présence du lanceur donnée (pour les tests : jamais le PC de Seb).
fn adresse_carte_selon(boutique: &str, cle_carte: &str, action: crate::boutiques_pc::Action, lanceur_present: impl Fn(&str) -> bool) -> Resultat<String> {
    use crate::boutiques_pc::{adresse, message_lanceur_absent, Action};
    let cle = cle_carte.strip_prefix(&format!("{boutique}_")).unwrap_or(cle_carte);
    if !lanceur_present(boutique) {
        return Err(Erreur::Refus(message_lanceur_absent(boutique)));
    }
    let url = match (boutique, action) {
        ("epic", Action::Jouer) => crate::boutiques::installes_epic_du_pc().and_then(|l| l.into_iter().find(|j| j.app == cle)).map(|j| crate::boutiques::adresse_epic(&j)),
        ("xboxone", Action::Jouer) => crate::boutiques_pc::application_xbox(cle).and_then(|a| adresse(boutique, cle, action, Some(&a))),
        ("gog", Action::Installer) if !crate::boutiques_pc::galaxy_present() => return Err(Erreur::Refus(message_lanceur_absent("gog"))),
        ("gog", Action::Jouer) => None,
        _ => adresse(boutique, cle, action, None),
    };
    url.ok_or_else(|| {
        Erreur::Refus(match (boutique, action) {
            ("battlenet", _) => message_lanceur_absent("battlenet"),
            (_, Action::Jouer) => format!("{} ne voit plus ce jeu installé : ouvre-le ou répare-le depuis {}.", crate::boutiques_pc::nom_boutique(boutique), crate::boutiques_pc::nom_boutique(boutique)),
            _ => format!("Frogtend ne sait pas installer ce jeu : installe-le depuis {}.", crate::boutiques_pc::nom_boutique(boutique)),
        })
    })
}

/// Lot C4b : l'AVERTISSEMENT (jamais un refus : la personne peut ouvrir Galaxy quand même) quand GOG Galaxy est ouvert
/// sur un autre compte que celui qui possède ce jeu (`cle` : numéro du jeu ; `galaxy` : relu à l'instant) ; `None` : on
/// ne le sait pas, ou Galaxy l'a (installation habituelle). Pur, testé.
pub(crate) fn avertissement_installation_hors_galaxy(comptes: &[crate::boutiques::CompteGog], galaxy: &crate::boutiques::CompteGalaxyPc, cle: &str) -> Option<String> {
    let (nom, numero) = crate::boutiques::compte_hors_galaxy(comptes, galaxy, cle)?;
    Some(format!(
        "Ce jeu est sur ton autre compte GOG « {} ». GOG Galaxy semble ouvert sur un autre compte : pour l'installer, change de compte dans Galaxy.",
        crate::boutiques::nom_compte_gog(&nom, numero)
    ))
}

/// Les comptes GOG d'un profil ont-ils tous leur numéro Galaxy, et ce jeu est-il à l'un d'eux ? Sinon, inutile de lire
/// la base de Galaxy (la réponse serait « on ne sait pas »).
pub(crate) fn question_galaxy_utile(comptes: &[crate::boutiques::CompteGog], cle: &str) -> bool {
    let id = format!("gog_{cle}");
    !comptes.is_empty() && comptes.iter().all(|c| !c.id_galaxy.is_empty()) && comptes.iter().any(|c| c.jeux.iter().any(|j| j.id == id))
}

/// Lot C4b : relit À L'INSTANT le compte ouvert dans GOG Galaxy (copie de sa base, effacée ensuite) et dit si ce jeu GOG
/// est seulement sur un autre compte du profil : `Some((message, nom))`, sinon `None` (dans le doute aussi).
pub(crate) async fn jeu_gog_hors_galaxy_relu(dossier_profil: &std::path::Path, travail: std::path::PathBuf, cle: &str) -> Option<String> {
    let comptes = crate::boutiques::lire_source(dossier_profil, "gog").map(|c| crate::boutiques::comptes_gog_lus(&c)).unwrap_or_default();
    if !question_galaxy_utile(&comptes, cle) {
        return None;
    }
    let galaxy = tauri::async_runtime::spawn_blocking(move || crate::boutiques::dossier_galaxy().and_then(|s| crate::boutiques::compte_galaxy_du_pc(&s, &travail))).await.ok().flatten()?;
    avertissement_installation_hors_galaxy(&comptes, &galaxy, cle)
}

/// Lot C4b : avant « ⬇ Installer » un jeu GOG, l'avertissement à montrer (fenêtre maison : « Ouvrir quand même dans GOG
/// Galaxy » / « Annuler ») ; `None` : rien à dire.
#[tauri::command]
pub async fn boutique_gog_avertissement_installation(noyau: State<'_, Noyau>, cle: String) -> Resultat<Option<String>> {
    let p = profil_ouvert(&noyau).await?;
    let cle = cle.strip_prefix("gog_").unwrap_or(&cle).to_string();
    if !crate::gratuits::numero_gog(&cle) {
        return Ok(None);
    }
    Ok(jeu_gog_hors_galaxy_relu(&dossier_profil_de(&noyau, &p.id), noyau.dossier.join("travail").join("galaxy-installer"), &cle).await)
}

/// Jouer à un jeu de boutique ou l'installer, par SON lanceur (Epic, Ubisoft, Amazon, Xbox, GOG Galaxy…).
#[tauri::command]
pub async fn boutique_ouvrir(boutique: String, cle: String, action: String) -> Resultat<()> {
    let action = match action.as_str() {
        "jouer" => crate::boutiques_pc::Action::Jouer,
        "installer" => crate::boutiques_pc::Action::Installer,
        _ => return Err(Erreur::Refus("Action inconnue.".into())),
    };
    let b = boutique.clone();
    let url = tauri::async_runtime::spawn_blocking(move || adresse_carte(&b, &cle, action))
        .await
        .map_err(|_| Erreur::Disque("La lecture du lanceur s'est arrêtée brutalement.".into()))??;
    tauri_plugin_opener::open_url(&url, None::<&str>).map_err(|_| Erreur::Disque(format!("{} ne s'ouvre pas.", crate::boutiques_pc::nom_boutique(&boutique))))
}

/// Le refus d'un import par une clé d'API d'un autre compte que celui connecté par la page.
const MOTIF_CLE_AUTRE_COMPTE: &str =
    "Ta clé d'API Steam est celle d'un autre compte que celui connecté : règle-la à nouveau pour ce compte (Secours), ou reconnecte-toi avec le bon compte.";

/// Importe (ou met à jour) la liste des jeux Steam possédés. Le compte doit être réglé (sinon : `Refus`, et
/// l'interface ouvre le réglage d'abord, règle de Seb).
#[tauri::command]
pub async fn boutique_steam_importer(noyau: State<'_, Noyau>) -> Resultat<Vec<crate::boutiques::JeuBoutique>> {
    // La session AU DÉBUT : la liste importée ne va qu'au profil qui l'a demandée.
    let s = noyau.session().await?;
    let p = s.profil.clone();
    let dossier = dossier_profil_de(&noyau, &p.id);
    let c = crate::boutiques::lire(&dossier).filter(|c| !c.steamid.is_empty()).ok_or_else(|| Erreur::Refus("Compte Steam pas encore réglé.".into()))?;
    let cle = noyau.coffre.lire(&crate::boutiques::nom_secret(&p.id, "steam"))?.ok_or_else(|| Erreur::Refus("Compte Steam pas encore réglé.".into()))?;
    // Jamais la clé d'un compte avec le steamid d'un autre (connexion par la page à un autre compte) : la clé reste.
    if crate::boutiques::cle_d_un_autre_compte(&c) {
        return Err(Erreur::Refus(MOTIF_CLE_AUTRE_COMPTE.into()));
    }
    let steamid = c.steamid.clone();
    menage_avant_import(&noyau, &p.id).await?;
    let jeux = crate::boutiques::jeux_possedes(crate::boutiques::API_STEAM, &cle, &steamid).await?;
    // Relu sous le verrou juste avant d'écrire : un compte changé pendant l'attente ne reçoit pas cette liste.
    let ecrit = crate::boutiques::modifier_source(&dossier, "steam", |c| {
        let mut c = c
            .filter(|c| c.steamid == steamid && !crate::boutiques::cle_d_un_autre_compte(c))
            .ok_or_else(|| Erreur::Refus("Le compte Steam a changé pendant l'import : relance-le.".into()))?;
        c.jeux = jeux;
        c.maj_le = crate::noyau::maintenant();
        Ok(Some(c))
    })?;
    noyau.journaliser(&format!("import Steam : {} jeu(x) pour le profil {}", ecrit.as_ref().map_or(0, |c| c.jeux.len()), p.id));
    let (jeux, _) = ranger_steam_importe(&noyau, &s, ecrit.map(|c| c.jeux).unwrap_or_default()).await?;
    Ok(jeux)
}

/// Range la liste Steam importée dans la ludothèque de la session `s` (prise au DÉBUT de l'import), avec les jeux
/// installés lus sur ce PC si Steam y est (une carte par jeu, « installé » d'après le disque), sinon la liste seule :
/// les jeux possédés entrent même sans Steam sur ce PC. Rend la liste rangée et les ids des cartes NOUVELLES.
async fn ranger_steam_importe(noyau: &Noyau, s: &crate::noyau::Session, api: Vec<crate::boutiques::JeuBoutique>) -> Resultat<(Vec<crate::boutiques::JeuBoutique>, Vec<i64>)> {
    // Jamais en même temps qu'une relecture des boutiques du PC.
    let _un_seul = RELECTURE.lock().await;
    let locaux = tauri::async_runtime::spawn_blocking(|| crate::boutiques_pc::lire_steam(crate::boutiques::dossier_steam().as_deref()))
        .await
        .map_err(|_| Erreur::Disque("La lecture de Steam s'est arrêtée brutalement.".into()))?;
    let l = match locaux {
        Some(locaux) => {
            synchroniser_registre(noyau, "steam", &locaux);
            crate::boutiques_pc::fusion_steam(api, &locaux)
        }
        None => api,
    };
    // Dans la ludothèque, avec les autres jeux (plateforme Windows).
    let nouveaux = noyau.ranger_installes(s, "steam", &l)?;
    Ok((l, nouveaux))
}

/// Ce qu'un import Steam par la connexion a donné (lot C2).
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct ImportSteam {
    /// La connexion du profil à Steam ne vaut plus (pas de jeton, jeton refusé ou expiré) : rien n'a changé, l'interface
    /// dit « reconnecte-toi ». Les jeux déjà importés restent.
    pub reconnecter: bool,
    /// Les jeux possédés du compte.
    pub total: usize,
    /// Les cartes NOUVELLES dans la ludothèque, et combien sont installées.
    pub nouveaux: usize,
    pub nouveaux_installes: usize,
}

/// Changement de compte Steam sur `profil` (décision de Seb du 20/11) : ses cartes Steam de l'ancien compte jamais
/// installées ni jouées quittent sa ludothèque. Le ménage est noté EN ATTENTE dans `boutiques.json` au moment du
/// changement (`menage_ancien_compte`) et effacé seulement une fois fait : si un autre profil est ouvert entre-temps, il
/// se fait à la prochaine ouverture de CE profil (avant tout import, sinon les jeux du nouveau compte partiraient
/// aussi). Un échec est noté, jamais montré : les cartes restent, rien n'est perdu. Rend vrai s'il ne reste rien à faire.
pub(crate) async fn menage_ancien_compte_steam(noyau: &Noyau, profil: &str) -> bool {
    // Jamais en même temps qu'une relecture des boutiques ni qu'un rangement d'import.
    let _un_seul = RELECTURE.lock().await;
    let dossier = dossier_profil_de(noyau, profil);
    if !crate::boutiques::lire(&dossier).is_some_and(|c| c.menage_ancien_compte) {
        return true;
    }
    let s = match noyau.session().await {
        Ok(s) if s.profil.id == profil => s,
        _ => {
            noyau.journaliser(&format!("changement de compte Steam : ménage en attente pour le profil {profil} (fait à sa prochaine ouverture)"));
            return false;
        }
    };
    match noyau.retirer_ancien_compte_steam(&s) {
        Ok(n) => {
            noyau.journaliser(&format!("changement de compte Steam : {n} carte(s) de l'ancien compte retirée(s) pour le profil {profil}"));
            let fait = crate::boutiques::modifier_source(&dossier, "steam", |c| {
                Ok(c.map(|mut c| {
                    c.menage_ancien_compte = false;
                    c
                }))
            });
            if let Err(e) = &fait {
                noyau.journaliser(&format!("changement de compte Steam : ménage fait mais pas noté pour le profil {profil} ({e})"));
            }
            fait.is_ok()
        }
        Err(e) => {
            noyau.journaliser(&format!("changement de compte Steam : cartes de l'ancien compte gardées pour le profil {profil} ({e})"));
            false
        }
    }
}

/// Avant un import Steam : le ménage d'un changement de compte en attente doit être fait (sinon les jeux du NOUVEAU
/// compte, rangés ensuite, seraient retirés avec ceux de l'ancien).
async fn menage_avant_import(noyau: &Noyau, profil: &str) -> Resultat<()> {
    if menage_ancien_compte_steam(noyau, profil).await {
        Ok(())
    } else {
        Err(Erreur::Disque("Frogtend n'a pas fini de ranger les jeux de ton ancien compte Steam : réessaie dans un moment.".into()))
    }
}

const FENETRE_STEAM_JETON: &str = "boutique-steam-jeton";

/// Le jeton d'accès Steam du compte connecté dans le navigateur DU PROFIL (lot C2) : une fenêtre CACHÉE, sans focus,
/// ouvre la vraie boutique ; son script (`steam-jeton.js`) y lit le jeton et le remet par une navigation vers
/// frogtend.invalid, interceptée et annulée (0.215.2). Sans jeton, l'étape remise choisit le message (lot C3 :
/// « reconnecte-toi », page changée ou réponse inhabituelle). Le jeton ne vit qu'en mémoire : jamais écrit, journalisé
/// ni rendu, jamais dans un titre.
async fn lire_jeton_steam(app: &AppHandle, noyau: &Noyau, dossier: &std::path::Path) -> Resultat<String> {
    // Un profil sans navigateur ne s'est jamais connecté par la page : sa connexion ne vaut pas.
    if !crate::gratuits::dossier_navigateur(dossier).is_dir() {
        return Err(Erreur::Refus(crate::boutiques::MOTIF_RECONNECTER.into()));
    }
    // Juste après « Connecté », la fenêtre de connexion se ferme d'elle-même : on la laisse finir (2 s au plus).
    for _ in 0..10 {
        if app.get_webview_window(FENETRE_STEAM).is_none() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    if app.get_webview_window(FENETRE_STEAM).is_some() {
        return Err(Erreur::Refus("Termine d'abord la connexion à Steam dans sa fenêtre, puis réessaie.".into()));
    }
    if app.get_webview_window(FENETRE_STEAM_JETON).is_some() {
        return Err(Erreur::Refus("La lecture de ton compte Steam est déjà en cours : attends qu'elle finisse.".into()));
    }
    // La VRAIE page de la boutique : son jeton (`data-store_user_config`) est lu PAR LA PAGE et remis au cœur par une
    // navigation vers frogtend.invalid, interceptée et ANNULÉE ici (0.215.2, Seb 20/11 : demandé par le cœur avec les
    // cookies, Steam répondait 302). Le jeton n'est jamais chargé, ni écrit, ni journalisé, ni mis dans un titre.
    let (envoi, reception) = std::sync::mpsc::channel::<crate::gratuits::RemiseSteam>();
    let envoi = std::sync::Mutex::new(envoi);
    let boutique = tauri::Url::parse("https://store.steampowered.com/").map_err(|_| Erreur::Refus("Adresse Steam invalide.".into()))?;
    let w = tauri::WebviewWindowBuilder::new(app, FENETRE_STEAM_JETON, tauri::WebviewUrl::External(boutique))
        .title("Steam — Frogtend")
        .inner_size(1000.0, 700.0)
        .visible(false)
        .focused(false)
        .skip_taskbar(true)
        .data_directory(crate::gratuits::dossier_navigateur(dossier))
        .on_new_window(crate::gratuits::fenetre_d_automatisation)
        .on_navigation(move |u| {
            if let Some(r) = crate::gratuits::lire_remise_steam(u) {
                let _ = envoi.lock().map(|e| e.send(r));
                return false;
            }
            crate::gratuits::navigation_steam(u)
        })
        .on_page_load(|w, charge| {
            if matches!(charge.event(), tauri::webview::PageLoadEvent::Finished) && w.url().is_ok_and(|u| crate::gratuits::page_de_la_boutique_steam(&u)) {
                let _ = w.eval(crate::gratuits::SCRIPT_STEAM_JETON);
            }
        })
        .build()
        .map_err(|e| Erreur::Disque(format!("La lecture de ton compte Steam ne s'ouvre pas ({e}).")))?;
    let r = tauri::async_runtime::spawn_blocking(move || reception.recv_timeout(std::time::Duration::from_secs(30)).ok()).await.ok().flatten();
    let _ = w.close();
    match r {
        Some(crate::gratuits::RemiseSteam::Jeton(j)) => {
            noyau.journaliser("import Steam (connexion) : jeton lu par la page");
            Ok(j)
        }
        Some(crate::gratuits::RemiseSteam::Absent(etape)) => {
            noyau.journaliser(&format!("import Steam (connexion) : pas de jeton ({etape})"));
            // Lot C3 : « reconnecte-toi » seulement quand c'est le bon conseil (page changée, maintenance : autre message).
            Err(crate::gratuits::erreur_sans_jeton_steam(&etape))
        }
        None => {
            noyau.journaliser("import Steam (connexion) : la boutique n'a rien remis en 30 s");
            Err(Erreur::Reseau("Steam n'a pas répondu à temps : réessaie dans un moment.".into()))
        }
    }
}

/// Lot C2 (décision de Seb du 20/11) : importe les jeux Steam POSSÉDÉS du compte connecté par la page, SANS clé d'API.
/// Le jeton d'accès est relu à chaque fois dans le navigateur du profil, vérifié (son `sub` = le `steamid` rangé), puis
/// oublié. La liste est écrite sous le verrou (compte inchangé exigé) et rangée aussitôt dans la ludothèque.
/// Connexion expirée : `reconnecter`, rien ne change (les jeux déjà importés restent).
#[tauri::command]
pub async fn boutique_steam_importer_connecte(app: AppHandle, noyau: State<'_, Noyau>) -> Resultat<ImportSteam> {
    // La session AU DÉBUT : la liste importée ne va qu'au profil qui l'a demandée.
    let s = noyau.session().await?;
    let p = s.profil.clone();
    let dossier = dossier_profil_de(&noyau, &p.id);
    let c = crate::boutiques::lire(&dossier)
        .filter(crate::boutiques::connecte_par_la_page)
        .ok_or_else(|| Erreur::Refus("Tu n'es pas connecté à Steam : dans 🛒 Boutiques, « Se connecter à Steam ».".into()))?;
    let steamid = c.steamid.clone();
    menage_avant_import(&noyau, &p.id).await?;
    let jeux = match importer_par_jeton(&app, &noyau, &dossier, &steamid).await {
        Ok(j) => j,
        Err(Erreur::Refus(m)) if m == crate::boutiques::MOTIF_RECONNECTER => {
            noyau.journaliser(&format!("import Steam (connexion) : connexion expirée pour le profil {}", p.id));
            return Ok(ImportSteam { reconnecter: true, ..Default::default() });
        }
        Err(e) => return Err(e),
    };
    // Relu sous le verrou juste avant d'écrire : un compte changé pendant l'attente ne reçoit pas cette liste.
    let ecrit = crate::boutiques::modifier_source(&dossier, "steam", |c| {
        let mut c = c
            .filter(|c| c.steamid == steamid && crate::boutiques::connecte_par_la_page(c))
            .ok_or_else(|| Erreur::Refus("Le compte Steam a changé pendant l'import : relance-le.".into()))?;
        c.jeux = jeux;
        c.maj_le = crate::noyau::maintenant();
        Ok(Some(c))
    })?;
    let total = ecrit.as_ref().map_or(0, |c| c.jeux.len());
    noyau.journaliser(&format!("import Steam (connexion) : {total} jeu(x) pour le profil {}", p.id));
    let (l, nouveaux) = ranger_steam_importe(&noyau, &s, ecrit.map(|c| c.jeux).unwrap_or_default()).await?;
    Ok(ImportSteam { reconnecter: false, total, nouveaux_installes: compter_installes(&l, &nouveaux), nouveaux: nouveaux.len() })
}

/// Le jeton lu, vérifié, puis la liste demandée à Steam ; le jeton est oublié en sortant (jamais écrit ni journalisé).
async fn importer_par_jeton(app: &AppHandle, noyau: &Noyau, dossier: &std::path::Path, steamid: &str) -> Resultat<Vec<crate::boutiques::JeuBoutique>> {
    let jeton = lire_jeton_steam(app, noyau, dossier).await?;
    let maintenant = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    if let Err(e) = crate::boutiques::verifier_jeton_steam(&jeton, steamid, maintenant) {
        noyau.journaliser("import Steam (connexion) : jeton refusé par Frogtend (autre compte, expiré ou illisible)");
        return Err(e);
    }
    let r = crate::boutiques::jeux_possedes_par_jeton(crate::boutiques::API_STEAM, &jeton, steamid).await;
    if r.is_err() {
        noyau.journaliser("import Steam (connexion) : la liste des jeux a été refusée par Steam");
    }
    r
}

/// Parmi les cartes nouvelles (`nouveaux`), celles dont le jeu est installé sur ce PC.
fn compter_installes(l: &[crate::boutiques::JeuBoutique], nouveaux: &[i64]) -> usize {
    l.iter().filter(|j| j.installe && nouveaux.contains(&crate::noyau::id_carte("steam", j))).count()
}

// ───────────────────────────── GOG : les jeux du compte (lot C4a) ─────────────────────────────

/// Un compte GOG du profil, pour l'interface (lot C4b) : sa clé (pour « Oublier »), son pseudo (vide : « Compte GOG
/// n »), jamais un jeton.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct EtatCompteGog {
    pub cle: String,
    pub nom: String,
    /// Le compte de la session gog.com du navigateur du profil (celui du jeu offert).
    pub profil: bool,
    /// Le n de « Compte GOG n » (sans pseudo), rangé.
    pub numero: u32,
    pub nb_jeux: usize,
    pub maj_le: Option<String>,
    /// Un compte ajouté dont la connexion est perdue (aucun jeton au coffre) : « ➕ Ajouter un compte GOG » le reconnecte.
    pub reconnecter: bool,
}

/// Ce que Frogtend sait des comptes GOG du profil (lots C4a, C4b).
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct EtatGog {
    /// Le compte de la session gog.com du navigateur du profil a déjà été lu.
    pub connu: bool,
    /// Les jeux de tous les comptes réunis (une carte par jeu).
    pub nb_jeux: usize,
    pub maj_le: Option<String>,
    /// Lot C4b : tous les comptes, le compte `Profil` d'abord.
    pub comptes: Vec<EtatCompteGog>,
    /// Lot C4b : GOG Galaxy est ouvert sur un autre compte que tous ceux de Frogtend (seulement si c'est sûr).
    pub galaxy_autre_compte: bool,
    /// Lot C4b : le compte du navigateur a été oublié ; sa session n'est plus relue avant « 🔑 Se connecter à GOG ».
    pub navigateur_oublie: bool,
}

/// L'état des comptes GOG (pur sauf le coffre, testé) : `a_un_secret` dit si un compte ajouté a encore son jeton.
pub(crate) fn etat_gog(c: Option<&crate::boutiques::CompteBoutique>, a_un_secret: impl Fn(&str) -> bool) -> EtatGog {
    use crate::boutiques::NavigateurGog;
    let mut comptes = c.map(crate::boutiques::comptes_gog_lus).unwrap_or_default();
    comptes.sort_by_key(|x| x.navigateur != NavigateurGog::Profil);
    let reunis = crate::boutiques::jeux_gog_reunis(&comptes);
    EtatGog {
        connu: comptes.iter().any(|x| x.navigateur == NavigateurGog::Profil && crate::gratuits::numero_gog(&x.id_site)),
        nb_jeux: reunis.len(),
        maj_le: comptes.iter().map(|x| x.maj_le.clone()).filter(|m| !m.is_empty()).max(),
        // Le compte de Galaxy relu à la dernière lecture des comptes GOG (une ligne d'information, jamais un refus).
        galaxy_autre_compte: crate::boutiques::galaxy_sur_un_autre_compte(&comptes, c.map_or("", |c| c.compte_galaxy_pc.as_str())),
        navigateur_oublie: c.is_some_and(|c| c.navigateur_gog_oublie),
        comptes: comptes
            .iter()
            .map(|x| EtatCompteGog {
                cle: x.cle(),
                nom: x.nom.clone(),
                profil: x.navigateur == NavigateurGog::Profil,
                numero: x.numero,
                nb_jeux: x.jeux.len(),
                maj_le: Some(x.maj_le.clone()).filter(|m| !m.is_empty()),
                reconnecter: x.navigateur == NavigateurGog::Ajoute && (x.id_galaxy.is_empty() || !a_un_secret(&x.id_galaxy)),
            })
            .collect(),
    }
}

#[tauri::command]
pub async fn boutique_gog_etat(noyau: State<'_, Noyau>) -> Resultat<EtatGog> {
    let p = profil_ouvert(&noyau).await?;
    let dossier = dossier_profil_de(&noyau, &p.id);
    let c = crate::boutiques::lire_source(&dossier, "gog");
    Ok(etat_gog(c.as_ref(), |id| noyau.coffre.lire(&crate::boutiques::secret_gog_ajoute(&p.id, id)).is_ok_and(|s| s.is_some())))
}

/// Ce qu'une lecture des jeux du compte GOG a donné (lot C4a) : la forme de Steam, plus la présence de GOG Galaxy
/// (sans lui, « ⬇ Installer » ne peut rien : le message le dit).
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct ImportGog {
    pub reconnecter: bool,
    pub total: usize,
    pub nouveaux: usize,
    pub nouveaux_installes: usize,
    /// GOG Galaxy est installé sur ce PC.
    pub galaxy: bool,
    /// Lot C5 : la bibliothèque de GOG Galaxy a été lue en entier (sinon : la liste du site et les jeux déjà rangés,
    /// nouvel essai à la prochaine lecture).
    pub bibliotheque_galaxy: bool,
    /// Lot C4b : ce que la lecture a donné pour chaque compte AJOUTÉ (le compte `Profil` : les champs ci-dessus).
    pub comptes: Vec<ResultatCompteGog>,
    /// Lot C4b : la lecture du compte `Profil` a échoué (message), alors que des comptes ajoutés ont pu être lus.
    pub erreur: Option<String>,
    /// Lot C4b : combien de comptes ont été lus (0 : rien n'a été lu, l'interface n'annonce pas « Rien de nouveau »).
    pub lus: usize,
}

/// Ce que la lecture d'un compte GOG ajouté a donné (lot C4b), sans aucun jeton.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct ResultatCompteGog {
    pub cle: String,
    pub nom: String,
    /// Le n de « Compte GOG n » (sans pseudo), rangé.
    pub numero: u32,
    /// Sa connexion est perdue (jeton refusé ou absent) : « ➕ Ajouter un compte GOG » le reconnecte.
    pub reconnecter: bool,
    /// Sa bibliothèque n'a pas pu être lue (réseau, GOG) : ses jeux déjà rangés restent, nouvel essai plus tard.
    pub partiel: bool,
}

/// L'étiquette de la fenêtre CACHÉE qui lit les jeux du compte GOG : à elle seule (jamais `boutique-gog`, que la
/// connexion et la récupération du jeu offert ferment).
const FENETRE_GOG_JEUX: &str = "boutique-gog-jeux";
/// La fenêtre de connexion à GOG et de récupération du jeu offert (`gratuits_connexion`, `gratuits_recuperer`).
const FENETRE_GOG: &str = "boutique-gog";
/// Le délai SANS NOUVELLE de la page GOG (chargement, puis toutes les 5 pages lues, puis chaque morceau) : au-delà,
/// la lecture échoue. Le script dit qu'il avance toutes les 5 pages (quelques secondes).
const DELAI_GOG_SANS_NOUVELLE: std::time::Duration = std::time::Duration::from_secs(60);
/// Le plafond d'une lecture GOG entière : 2 × 100 pages au plus (plafond du script), 250 ms de pause entre deux, puis
/// 400 morceaux au plus — bien moins de 10 minutes même lentement.
const DELAI_GOG_PLAFOND: std::time::Duration = std::time::Duration::from_secs(600);

/// Lit les jeux du compte GOG connecté dans le navigateur DU PROFIL (lot C4a) : une fenêtre CACHÉE, sans focus, ouvre
/// la vraie boutique ; son script (`gog-jeux.js`) demande la liste à GOG (mêmes cookies, même navigateur) et la remet
/// par des navigations vers frogtend.invalid, interceptées et annulées : le compte, puis les morceaux que le cœur
/// demande un à un. Rend `None` si le compte n'est pas connecté (« reconnecte-toi »). Rien du compte n'est journalisé.
async fn lire_jeux_gog(app: &AppHandle, noyau: &Noyau, dossier: &std::path::Path) -> Resultat<Option<(String, Vec<crate::gratuits::JeuGogLu>)>> {
    // Un profil sans navigateur ne s'est jamais connecté à GOG par Frogtend.
    if !crate::gratuits::dossier_navigateur(dossier).is_dir() {
        return Ok(None);
    }
    if app.get_webview_window(FENETRE_GOG).is_some() {
        return Err(Erreur::Refus("Termine d'abord ce qui se passe dans la fenêtre de GOG (connexion ou jeu offert), puis réessaie.".into()));
    }
    if app.get_webview_window(FENETRE_GOG_JEUX).is_some() {
        return Err(Erreur::Refus("La lecture de tes jeux GOG est déjà en cours : attends qu'elle finisse.".into()));
    }
    let (envoi, reception) = std::sync::mpsc::channel::<crate::gratuits::RemiseGog>();
    let envoi = std::sync::Mutex::new(envoi);
    let page = tauri::Url::parse(crate::gratuits::PAGE_GOG).map_err(|_| Erreur::Refus("Adresse GOG invalide.".into()))?;
    let w = tauri::WebviewWindowBuilder::new(app, FENETRE_GOG_JEUX, tauri::WebviewUrl::External(page))
        .title("GOG — Frogtend")
        .inner_size(1000.0, 700.0)
        .visible(false)
        .focused(false)
        .skip_taskbar(true)
        .data_directory(crate::gratuits::dossier_navigateur(dossier))
        .on_new_window(crate::gratuits::fenetre_d_automatisation)
        .on_navigation(move |u| {
            if let Some(r) = crate::gratuits::lire_remise_gog(u) {
                let _ = envoi.lock().map(|e| e.send(r));
                return false;
            }
            crate::gratuits::navigation_gog(u)
        })
        .on_page_load(|w, charge| {
            if matches!(charge.event(), tauri::webview::PageLoadEvent::Finished) && w.url().is_ok_and(|u| crate::gratuits::navigation_gog(&u)) {
                let _ = w.eval(crate::gratuits::SCRIPT_GOG_JEUX);
            }
        })
        .build()
        .map_err(|e| Erreur::Disque(format!("La lecture de ton compte GOG ne s'ouvre pas ({e}).")))?;
    // Les remises, une à une, jusqu'à la fin ou au délai : chaque morceau reçu, le suivant est demandé à la page.
    let page_js = w.clone();
    let suite = tauri::async_runtime::spawn_blocking(move || {
        let fin = std::time::Instant::now() + DELAI_GOG_PLAFOND;
        let mut r = crate::gratuits::ReceptionGog::default();
        loop {
            // Le délai repart à chaque nouvelle de la page, dans la limite du plafond.
            let reste = fin.saturating_duration_since(std::time::Instant::now()).min(DELAI_GOG_SANS_NOUVELLE);
            let Ok(remise) = reception.recv_timeout(reste) else { return (None, r.recus(), r.bilan()) };
            match r.recevoir(remise) {
                crate::gratuits::SuiteGog::Demander(k) => {
                    let _ = page_js.eval(&format!("window.__frogtendGog && window.__frogtendGog({k})"));
                }
                crate::gratuits::SuiteGog::Attendre => {}
                s => return (Some(s), r.recus(), r.bilan()),
            }
        }
    })
    .await;
    let _ = w.close();
    let (suite, recus, bilan) = suite.map_err(|_| Erreur::Disque("La lecture de ton compte GOG s'est arrêtée brutalement.".into()))?;
    // 0.217.1 (Seb 21/11 : deux jeux de son compte manquaient) : ce que GOG a envoyé et ce qui a été écarté.
    if let Some(b) = bilan {
        noyau.journaliser(&format!("import GOG (connexion) : bilan de la page ({b})"));
    }
    match suite {
        Some(crate::gratuits::SuiteGog::Fini(compte, jeux)) => {
            noyau.journaliser(&format!("import GOG (connexion) : liste lue par la page ({recus} morceau(x), {} jeu(x))", jeux.len()));
            Ok(Some((compte, jeux)))
        }
        Some(crate::gratuits::SuiteGog::Absent(etape)) => {
            noyau.journaliser(&format!("import GOG (connexion) : pas de liste ({etape})"));
            match crate::gratuits::erreur_sans_jeux_gog(&etape) {
                None => Ok(None),
                Some(e) => Err(e),
            }
        }
        Some(crate::gratuits::SuiteGog::Echec(etape)) => {
            noyau.journaliser(&format!("import GOG (connexion) : remise refusée par Frogtend ({etape}, {recus} morceau(x) reçu(s)) ; rien n'est écrit"));
            Err(Erreur::Serveur(crate::gratuits::MOTIF_GOG_INHABITUEL.into()))
        }
        Some(crate::gratuits::SuiteGog::Demander(_) | crate::gratuits::SuiteGog::Attendre) | None => {
            noyau.journaliser(&format!(
                "import GOG (connexion) : la boutique s'est tue plus de {} s (ou plus de {} s en tout) ({recus} morceau(x) reçu(s)) ; rien n'est écrit",
                DELAI_GOG_SANS_NOUVELLE.as_secs(),
                DELAI_GOG_PLAFOND.as_secs()
            ));
            Err(Erreur::Reseau("GOG n'a pas répondu à temps : réessaie dans un moment.".into()))
        }
    }
}

// ───────────────────────────── GOG : connexion façon Galaxy (lot C5) ─────────────────────────────

/// La fenêtre de la connexion façon Galaxy (cachée ; montrée seulement si la personne doit se connecter).
const FENETRE_GOG_GALAXY: &str = "boutique-gog-galaxy";
/// Lot C4b : la fenêtre « ➕ Ajouter un compte GOG » (montrée tout de suite, navigation privée, dossier temporaire propre
/// à l'ajout, effacé ensuite).
const FENETRE_GOG_AJOUT: &str = "boutique-gog-ajout";
/// Le nom du jeton de renouvellement du compte GOG `Profil` au coffre de Windows (les comptes ajoutés :
/// `boutiques::secret_gog_ajoute`).
fn secret_gog(profil: &str) -> String {
    crate::boutiques::nom_secret(profil, crate::gog_galaxy::SECRET)
}

/// Lot C4b : une seule opération sur les comptes GOG à la fois (lecture de tous les comptes, ajout, oubli). Les
/// lectures se suivent ; un ajout ou un oubli pendant une lecture est refusé (jamais deux écritures croisées, jamais
/// deux fenêtres de connexion).
static COMPTES_GOG: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

const MOTIF_GOG_OCCUPE: &str = "Frogtend lit tes comptes GOG en ce moment : attends qu'il finisse, puis réessaie.";

/// Ce que la fenêtre de connexion façon Galaxy voit.
enum VuConnexionGog {
    Retour(crate::gog_galaxy::RetourConnexion),
    /// GOG affiche sa page de connexion (pas de session ouverte).
    PageDeConnexion,
    /// La personne a fermé la fenêtre montrée.
    Fermee,
    /// Lot C4b : Google refuse la connexion dans cette fenêtre (navigateur intégré).
    RefusGoogle,
}

/// Comment ouvrir la fenêtre de la connexion façon Galaxy.
struct FenetreGog {
    etiquette: &'static str,
    /// Le dossier du navigateur : celui du profil (compte `Profil`), ou le dossier temporaire d'un ajout.
    donnees: std::path::PathBuf,
    /// La personne peut avoir à se connecter : la fenêtre s'affiche si GOG le demande.
    montrer: bool,
    /// Lot C4b : un ajout de compte (fenêtre montrée tout de suite, navigation privée, aucun compte attendu).
    ajout: bool,
    /// Le pseudo du compte attendu, pour le message de l'interface (jamais au journal ; vide : inconnu).
    attendu: String,
}

/// Le code de la connexion façon Galaxy, par la page OFFICIELLE de GOG. Compte `Profil` : dans le navigateur DU PROFIL
/// (sa session GOG y est déjà : redirection silencieuse). Ajout (lot C4b) : une fenêtre montrée, en navigation privée,
/// dans un dossier temporaire. La redirection finale (`…/on_login_success?code=…`) est interceptée et ANNULÉE : rien
/// n'est chargé. Sans session : en lecture discrète (`montrer` faux), rien n'est montré (`None`, « reconnecte-toi ») ;
/// sinon la fenêtre s'affiche pour que la personne se connecte (5 minutes au plus). Le code ne vit qu'en mémoire.
async fn code_connexion_gog(app: &AppHandle, noyau: &Noyau, profil: &str, f: FenetreGog) -> Resultat<Option<crate::gog_galaxy::CodeGog>> {
    if !f.ajout && !f.donnees.is_dir() {
        return Ok(None);
    }
    if app.get_webview_window(FENETRE_GOG_GALAXY).is_some() || app.get_webview_window(FENETRE_GOG_AJOUT).is_some() {
        return Err(Erreur::Refus("La connexion à GOG est déjà en cours : attends qu'elle finisse.".into()));
    }
    if app.get_webview_window(FENETRE_GOG).is_some() {
        return Err(Erreur::Refus("Termine d'abord ce qui se passe dans la fenêtre de GOG (connexion ou jeu offert), puis réessaie.".into()));
    }
    let (ajout, montrer) = (f.ajout, f.montrer || f.ajout);
    let (envoi, reception) = std::sync::mpsc::channel::<VuConnexionGog>();
    let (envoi_nav, envoi_page, envoi_fin) = (std::sync::Mutex::new(envoi.clone()), std::sync::Mutex::new(envoi.clone()), std::sync::Mutex::new(envoi));
    let url = tauri::Url::parse(&crate::gog_galaxy::adresse_connexion()).map_err(|_| Erreur::Refus("Adresse GOG invalide.".into()))?;
    let mut b = tauri::WebviewWindowBuilder::new(app, f.etiquette, tauri::WebviewUrl::External(url))
        .title(if ajout { "Ajouter un compte GOG — Frogtend" } else { "Connexion à GOG — Frogtend" })
        .inner_size(1000.0, 760.0)
        .center()
        .visible(ajout)
        .focused(ajout)
        .skip_taskbar(!ajout)
        .data_directory(f.donnees.clone())
        .on_new_window(if montrer { crate::gog_galaxy::fenetre_de_fournisseur } else { crate::gratuits::fenetre_d_automatisation })
        .on_navigation(move |u| {
            if let Some(r) = crate::gog_galaxy::lire_retour_connexion(u) {
                let _ = envoi_nav.lock().map(|e| e.send(VuConnexionGog::Retour(r)));
                return false;
            }
            if crate::gog_galaxy::refus_google(u) {
                let _ = envoi_nav.lock().map(|e| e.send(VuConnexionGog::RefusGoogle));
            }
            crate::gog_galaxy::navigation_connexion(u, montrer)
        })
        .on_page_load(move |w, charge| {
            if matches!(charge.event(), tauri::webview::PageLoadEvent::Finished) && w.url().is_ok_and(|u| crate::gog_galaxy::page_de_connexion_affichee(&u)) {
                let _ = envoi_page.lock().map(|e| e.send(VuConnexionGog::PageDeConnexion));
            }
        });
    if ajout {
        // Navigation privée : rien du compte ajouté ne reste dans un navigateur (le jeton de renouvellement suffit).
        b = b.incognito(true);
    }
    let w = b.build().map_err(|e| Erreur::Disque(format!("La connexion à GOG ne s'ouvre pas ({e}).")))?;
    w.on_window_event(move |e| {
        if matches!(e, tauri::WindowEvent::Destroyed) {
            let _ = envoi_fin.lock().map(|e| e.send(VuConnexionGog::Fermee));
        }
    });
    let fenetre = w.clone();
    let (annonce, profil, attendu) = (app.clone(), profil.to_string(), f.attendu.clone());
    let vu = tauri::async_runtime::spawn_blocking(move || {
        let mut fin = std::time::Instant::now() + std::time::Duration::from_secs(if ajout { 300 } else { 30 });
        let mut montree = ajout;
        let mut google_dit = false;
        loop {
            let reste = fin.saturating_duration_since(std::time::Instant::now());
            match reception.recv_timeout(reste) {
                Ok(VuConnexionGog::Retour(r)) => return Ok(r),
                Ok(VuConnexionGog::RefusGoogle) => {
                    // Dit une fois ; la fenêtre reste ouverte (la personne peut se connecter par adresse et mot de passe).
                    if !google_dit {
                        google_dit = true;
                        let _ = annonce.emit("gog-google-refuse", serde_json::json!({"profil": profil}));
                    }
                }
                // Une page de GOG s'est affichée : peut-être un relais qui redirige encore lui-même ; 4 s de grâce, puis
                // « pas de session » (rien n'est montré en lecture discrète).
                Ok(VuConnexionGog::PageDeConnexion) if !montrer => {
                    if !montree {
                        montree = true;
                        fin = fin.min(std::time::Instant::now() + std::time::Duration::from_secs(4));
                    }
                }
                Ok(VuConnexionGog::PageDeConnexion) => {
                    if !montree {
                        montree = true;
                        fin = std::time::Instant::now() + std::time::Duration::from_secs(300);
                        let _ = fenetre.set_skip_taskbar(false);
                        let _ = fenetre.show();
                        let _ = fenetre.unminimize();
                        let _ = fenetre.set_focus();
                        let _ = annonce.emit("gog-connexion-montree", serde_json::json!({"profil": profil, "compte": attendu}));
                    }
                }
                Ok(VuConnexionGog::Fermee) => return Err("fenetre-fermee"),
                Err(_) => return Err(if montree && !montrer { "page-de-connexion" } else { "delai-depasse" }),
            }
        }
    })
    .await;
    let _ = w.close();
    let vu = vu.map_err(|_| Erreur::Disque("La connexion à GOG s'est arrêtée brutalement.".into()))?;
    match vu {
        Ok(crate::gog_galaxy::RetourConnexion::Code(c)) => Ok(Some(c)),
        Ok(crate::gog_galaxy::RetourConnexion::SansCode(etape)) => {
            noyau.journaliser(&format!("import GOG (connexion façon Galaxy) : pas de code ({etape})"));
            Ok(None)
        }
        Err(etape) => {
            noyau.journaliser(&format!("import GOG (connexion façon Galaxy) : pas de code ({etape})"));
            Ok(None)
        }
    }
}

/// Le jeton d'accès de la connexion façon Galaxy pour le compte `compte` (son numéro Galaxy), renouvelé avec le jeton
/// rangé au coffre sous `secret`. `connexion` : `Some((montrer, pseudo attendu))` pour le compte `Profil` (sans jeton
/// valable, la connexion silencieuse par sa session gog.com) ; `None` pour un compte ajouté (seul son jeton le relit :
/// jamais la session du navigateur du profil, qui est celle d'un autre compte). Le nouveau jeton de renouvellement va au
/// coffre ; le jeton d'accès ne vit qu'en mémoire. `None` : pas de connexion (la raison au journal, sans aucun jeton).
async fn jeton_acces_gog(app: &AppHandle, noyau: &Noyau, dossier: &std::path::Path, profil: &str, secret: &str, compte: &str, connexion: Option<(bool, String)>) -> Option<String> {
    let ranger = |j: &crate::gog_galaxy::JetonsGog| {
        if noyau.coffre.ranger(secret, &j.renouvellement).is_err() {
            noyau.journaliser("import GOG (connexion façon Galaxy) : le coffre de Windows refuse le jeton (connexion refaite à la prochaine lecture)");
            let _ = noyau.coffre.oublier(secret);
        }
    };
    match noyau.coffre.lire(secret) {
        Ok(Some(r)) => match crate::gog_galaxy::jetons_renouveles(crate::gog_galaxy::AUTH, &r).await {
            Ok(j) if crate::gog_galaxy::verifier_compte(&j, compte).is_ok() => {
                ranger(&j);
                return Some(j.acces);
            }
            Ok(_) => {
                noyau.journaliser("import GOG (connexion façon Galaxy) : le jeton rangé est celui d'un autre compte : oublié");
                let _ = noyau.coffre.oublier(secret);
            }
            Err(Erreur::Refus(_)) => {
                noyau.journaliser("import GOG (connexion façon Galaxy) : jeton rangé refusé par GOG : oublié, connexion refaite");
                let _ = noyau.coffre.oublier(secret);
            }
            Err(_) => {
                noyau.journaliser("import GOG (connexion façon Galaxy) : renouvellement impossible pour l'instant (réseau ou GOG)");
                return None;
            }
        },
        Ok(None) => {}
        Err(_) => {
            noyau.journaliser("import GOG (connexion façon Galaxy) : coffre de Windows illisible : connexion refaite");
            let _ = noyau.coffre.oublier(secret);
        }
    }
    let (montrer, attendu) = connexion?;
    let f = FenetreGog { etiquette: FENETRE_GOG_GALAXY, donnees: crate::gratuits::dossier_navigateur(dossier), montrer, ajout: false, attendu };
    let code = match code_connexion_gog(app, noyau, profil, f).await {
        Ok(Some(c)) => c,
        Ok(None) => return None,
        Err(e) => {
            noyau.journaliser(&format!("import GOG (connexion façon Galaxy) : {e}"));
            return None;
        }
    };
    match crate::gog_galaxy::jetons_par_code(crate::gog_galaxy::AUTH, &code).await {
        Ok(j) if crate::gog_galaxy::verifier_compte(&j, compte).is_ok() => {
            noyau.journaliser("import GOG (connexion façon Galaxy) : connecté");
            ranger(&j);
            Some(j.acces)
        }
        Ok(_) => {
            noyau.journaliser("import GOG (connexion façon Galaxy) : la connexion est celle d'un autre compte que celui de gog.com : refusée");
            None
        }
        Err(e) => {
            noyau.journaliser(&format!("import GOG (connexion façon Galaxy) : échange du code refusé ({e})"));
            None
        }
    }
}

/// Ce que la bibliothèque de Galaxy d'UN compte a donné, résolu par la base publique (lot C5).
struct BibliothequeResolue {
    jeux: Vec<crate::boutiques::JeuBoutique>,
    /// Le cache commun des fiches, complété (il est fondu sous le verrou : `boutiques::fusionner_fiches_gog`).
    fiches: std::collections::BTreeMap<String, crate::gog_galaxy::FicheGog>,
    /// Les numéros de la bibliothèque lue (écartés compris).
    ids: Vec<String>,
}

/// Le plafond de temps des fiches d'une lecture (la suite à la lecture d'après : rien n'est perdu).
const DELAI_FICHES_GOG: std::time::Duration = std::time::Duration::from_secs(180);
/// Après tant d'échecs de suite, la base publique est en panne : on s'arrête (la suite à la lecture d'après).
const ECHECS_FICHES_GOG_MAX: usize = 10;

/// Lit la bibliothèque de Galaxy du compte `compte` (numéro Galaxy, jeton d'accès `acces`) et résout ses jeux (titre,
/// type, jaquette) par la base publique de GOG, sans redemander les fiches déjà en cache ; une fiche en échec ne fait
/// jamais disparaître un jeu ; 429 : on s'arrête en gardant ce qui est résolu. `ranges` : les jeux DE CE COMPTE déjà
/// rangés ; `page` : la liste du site (compte `Profil` seulement). `None` : bibliothèque illisible (un jeton refusé est
/// oublié : celui de `secret`, le compte lu). Journal : des nombres, les types et les titres des jeux écartés.
async fn bibliotheque_gog(
    noyau: &Noyau,
    dossier: &std::path::Path,
    secret: &str,
    compte: &str,
    acces: String,
    page: &[crate::boutiques::JeuBoutique],
    ranges: &[crate::boutiques::JeuBoutique],
) -> Option<BibliothequeResolue> {
    let bib = match crate::gog_galaxy::lire_bibliotheque(crate::gog_galaxy::BIBLIOTHEQUE, compte, &acces).await {
        Ok(b) => b,
        Err(e) => {
            if matches!(e, Erreur::Refus(_)) {
                // Le jeton ne vaut plus : la prochaine lecture refait la connexion (celle de CE compte seulement).
                let _ = noyau.coffre.oublier(secret);
            }
            noyau.journaliser(&format!("import GOG (bibliothèque Galaxy) : illisible ({e}) ; les jeux déjà rangés restent"));
            return None;
        }
    };
    drop(acces);
    let mut fiches = crate::boutiques::lire_source(dossier, "gog").map(|c| c.fiches_gog).unwrap_or_default();
    let maintenant = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let a_demander = crate::gog_galaxy::fiches_a_demander(&bib.ids, &fiches, maintenant);
    let mut demandees = 0;
    // Un seul client pour toutes les fiches ; au plus 3 minutes ; arrêt après 10 échecs de suite ; 429 : arrêt
    // jusqu'à la prochaine lecture (sans attendre ici). Un jeu sans fiche est gardé (`choisir`).
    let client = crate::gog_galaxy::client().ok();
    let debut = std::time::Instant::now();
    let mut echecs_de_suite = 0;
    for (i, id) in a_demander.iter().enumerate() {
        let Some(c) = &client else { break };
        if debut.elapsed() > DELAI_FICHES_GOG {
            noyau.journaliser(&format!("import GOG (bibliothèque Galaxy) : {demandees} fiche(s) lue(s) en {} s ; la suite à la prochaine lecture", DELAI_FICHES_GOG.as_secs()));
            break;
        }
        if i > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(crate::gog_galaxy::PAUSE_FICHES_MS)).await;
        }
        demandees += 1;
        match crate::gog_galaxy::fiche(c, crate::gog_galaxy::GAMESDB, crate::gog_galaxy::PRODUITS, id, maintenant).await {
            Ok(f) => {
                echecs_de_suite = 0;
                fiches.insert(id.clone(), f);
            }
            Err(crate::gog_galaxy::EchecFiche::TropDeDemandes) => {
                noyau.journaliser(&format!("import GOG (bibliothèque Galaxy) : la base publique demande d'attendre après {demandees} fiche(s) ; la suite à la prochaine lecture"));
                break;
            }
            Err(crate::gog_galaxy::EchecFiche::Autre) => {
                noyau.journaliser(&format!("import GOG (bibliothèque Galaxy) : fiche du jeu n° {id} illisible (jeu gardé)"));
                echecs_de_suite += 1;
                if echecs_de_suite >= ECHECS_FICHES_GOG_MAX {
                    noyau.journaliser("import GOG (bibliothèque Galaxy) : la base publique ne répond plus ; la suite à la prochaine lecture");
                    break;
                }
            }
        }
    }
    let (jeux, bilan) = crate::gog_galaxy::choisir(&bib.ids, &fiches, page, ranges);
    noyau.journaliser(&format!("import GOG (bibliothèque Galaxy) : {}", crate::gog_galaxy::texte_bilan(&bib, &bilan, demandees, jeux.len())));
    Some(BibliothequeResolue { jeux, fiches, ids: bib.ids })
}

/// Le pseudo public d'un compte (numéro Galaxy) ; `None` si inconnu ou illisible.
async fn pseudo_gog(id_galaxy: &str) -> Option<String> {
    if id_galaxy.is_empty() {
        return None;
    }
    crate::gog_galaxy::pseudo(crate::gog_galaxy::UTILISATEURS, id_galaxy).await
}

//// Lot C4b : le nom PROVISOIRE du jeton du nouveau compte du navigateur pendant un changement de compte (et celui d'un
/// compte qu'on ajoute) : le jeton n'arrive sous son vrai nom qu'APRÈS l'écriture de la liste.
fn secret_gog_provisoire(profil: &str, quoi: &str) -> String {
    crate::boutiques::nom_secret(profil, &format!("{}#provisoire-{quoi}", crate::gog_galaxy::SECRET))
}

/// Range un jeton sous `nom` seulement si ce nom est LIBRE (jamais un `#<numéro>` existant écrasé). Rend vrai si le nom
/// porte un jeton à la fin (le sien d'avant, ou celui-ci).
fn ranger_si_libre(noyau: &Noyau, nom: &str, jeton: &str) -> bool {
    match noyau.coffre.lire(nom) {
        Ok(Some(_)) => true,
        Ok(None) => noyau.coffre.ranger(nom, jeton).is_ok(),
        Err(_) => false,
    }
}

/// Lot C4b (revue) : avant un changement de compte du navigateur, le jeton rangé sous le nom du compte `Profil` est mis
/// à l'abri sous le nom de SON VRAI compte, retrouvé par GOG lui-même (renouvellement → `user_id`) : jamais déduit de
/// la liste (une lecture interrompue puis rejouée y laisserait le jeton d'un autre compte). Le nom existant n'est
/// jamais écrasé. Rend le numéro Galaxy de ce compte (`None` : pas de jeton, ou jeton refusé par GOG, alors oublié).
/// GOG injoignable : erreur (rien n'est changé, le changement de compte attendra la lecture suivante).
async fn abriter_jeton_profil(noyau: &Noyau, profil: &str, auth: &str) -> Resultat<Option<String>> {
    let principal = secret_gog(profil);
    let Ok(Some(r)) = noyau.coffre.lire(&principal) else { return Ok(None) };
    match crate::gog_galaxy::jetons_renouveles(auth, &r).await {
        Ok(j) => {
            // Le jeton renouvelé remplace l'ancien sous le nom principal (même compte), puis sa copie sous son vrai nom.
            let _ = noyau.coffre.ranger(&principal, &j.renouvellement);
            if !ranger_si_libre(noyau, &crate::boutiques::secret_gog_ajoute(profil, &j.user_id), &j.renouvellement) {
                noyau.journaliser("import GOG (changement de compte) : le coffre de Windows refuse le jeton de l'ancien compte (à reconnecter)");
            }
            Ok(Some(j.user_id))
        }
        Err(Erreur::Refus(_)) => {
            noyau.journaliser("import GOG (changement de compte) : le jeton de l'ancien compte est refusé par GOG : oublié");
            let _ = noyau.coffre.oublier(&principal);
            Ok(None)
        }
        Err(_) => Err(Erreur::Reseau("GOG ne répond pas pour l'instant : le changement de compte GOG attendra la prochaine mise à jour.".into())),
    }
}

/// Après l'écriture de la liste (lot C4b, revue) : le jeton PROVISOIRE du nouveau compte du navigateur (`provisoire`,
/// changement de compte) prend le nom principal ; le nom du compte « ajouté » fondu (`fondu`) est oublié, son jeton
/// servant d'abord au nom principal s'il n'y en a pas (même compte ajouté en double). Testé.
fn poser_jetons_apres_ecriture(noyau: &Noyau, profil: &str, provisoire: Option<&str>, fondu: Option<&str>) {
    let principal = secret_gog(profil);
    if let Some(secret) = provisoire {
        let fait = match noyau.coffre.lire(secret).ok().flatten() {
            Some(t) => noyau.coffre.ranger(&principal, &t),
            None => noyau.coffre.oublier(&principal),
        };
        if fait.is_err() {
            noyau.journaliser("import GOG (changement de compte) : le coffre de Windows refuse le jeton du nouveau compte (connexion refaite à la prochaine lecture)");
            let _ = noyau.coffre.oublier(&principal);
        }
        let _ = noyau.coffre.oublier(secret);
    }
    if let Some(id) = fondu {
        let nom_fondu = crate::boutiques::secret_gog_ajoute(profil, id);
        // Le compte fondu sans changement de compte (même compte, ajouté en double) : son jeton sert au nom principal
        // s'il n'y en a pas ; puis un seul jeton par compte.
        if provisoire.is_none() && !noyau.coffre.lire(&principal).is_ok_and(|s| s.is_some()) {
            if let Ok(Some(t)) = noyau.coffre.lire(&nom_fondu) {
                let _ = noyau.coffre.ranger(&principal, &t);
            }
        }
        if noyau.coffre.oublier(&nom_fondu).is_err() {
            noyau.journaliser("import GOG (changement de compte) : l'ancien nom du jeton n'a pas pu être retiré du coffre");
        }
    }
}

/// Lit le compte GOG `Profil` (la session gog.com du navigateur du profil) : la liste du site, puis sa bibliothèque de
/// Galaxy, et les range (seuls SES jeux changent). `Ok(None)` : pas connecté (« reconnecte-toi »). Rend si la
/// bibliothèque de Galaxy a été lue en entier.
///
/// Changement de compte du navigateur (revue du lot C4b) : 1. le jeton du nom principal est mis à l'abri sous le nom de
/// son vrai compte (`abriter_jeton_profil`) ; 2. le nouveau compte est lu avec un jeton sous un nom PROVISOIRE (celui du
/// compte ajouté fondu y est copié) ; 3. la liste est écrite ; 4. SEULEMENT ensuite le jeton provisoire prend le nom
/// principal et le nom du compte fondu est oublié. Une lecture interrompue à n'importe quelle étape se rejoue sans perte.
async fn importer_compte_gog_profil(app: &AppHandle, noyau: &Noyau, dossier: &std::path::Path, profil: &str, comptes: &[crate::boutiques::CompteGog], montrer: bool) -> Resultat<Option<bool>> {
    use crate::boutiques::NavigateurGog;
    let ancien = comptes.iter().find(|x| x.navigateur == NavigateurGog::Profil);
    let avant = ancien.map(|x| x.id_site.clone()).unwrap_or_default();
    let Some((remis, lus)) = lire_jeux_gog(app, noyau, dossier).await? else {
        noyau.journaliser(&format!("import GOG (connexion) : pas connecté à GOG pour le profil {profil}"));
        return Ok(None);
    };
    // 0.218.2 : le numéro du SITE est celui du compte rangé ; la connexion façon Galaxy et sa bibliothèque utilisent le
    // numéro GALAXY du même compte (Seb 21/11 : « autre compte » à tort).
    let (site, galaxy) = crate::gratuits::separer_compte_gog(&remis);
    let page = crate::boutiques::jeux_du_compte_gog(&lus);
    let principal = secret_gog(profil);
    let change = ancien.is_some_and(|a| a.id_site != site);
    let fondu = comptes.iter().find(|x| x.navigateur == NavigateurGog::Ajoute && ((!x.id_galaxy.is_empty() && x.id_galaxy == galaxy) || (x.id_galaxy.is_empty() && x.id_site == site)));
    let mut ancien_id_galaxy = None;
    let secret = if change {
        let proprietaire = abriter_jeton_profil(noyau, profil, crate::gog_galaxy::AUTH).await?;
        // Le jeton mis à l'abri est celui de l'ANCIEN compte seulement s'il n'est pas celui du nouveau (lecture rejouée).
        ancien_id_galaxy = proprietaire.filter(|p| *p != galaxy);
        let provisoire = secret_gog_provisoire(profil, "navigateur");
        let jeton_fondu = fondu.filter(|f| !f.id_galaxy.is_empty()).and_then(|f| noyau.coffre.lire(&crate::boutiques::secret_gog_ajoute(profil, &f.id_galaxy)).ok().flatten());
        let _ = match jeton_fondu {
            Some(t) => noyau.coffre.ranger(&provisoire, &t),
            None => noyau.coffre.oublier(&provisoire),
        };
        provisoire
    } else {
        principal.clone()
    };
    let ce_compte = if change { fondu } else { ancien };
    let ranges: Vec<crate::boutiques::JeuBoutique> = ce_compte.map(|x| x.jeux.clone()).unwrap_or_default();
    let attendu = ce_compte.map(|x| x.nom.clone()).unwrap_or_default();
    let acces = jeton_acces_gog(app, noyau, dossier, profil, &secret, &galaxy, Some((montrer, attendu))).await;
    // Le numéro Galaxy n'est retenu que s'il est sûr : remis par la page, ou vérifié par la connexion façon Galaxy.
    let galaxy_sur = galaxy != site || acces.is_some();
    let biblio = match acces {
        Some(a) => bibliotheque_gog(noyau, dossier, &secret, &galaxy, a, &page, &ranges).await,
        None => None,
    };
    let lue = biblio.is_some();
    // Sans bibliothèque lue en entier : la liste du site PLUS les jeux déjà rangés de CE compte (jamais une liste qui
    // rétrécit ; les autres comptes ne sont pas touchés).
    let (jeux, fiches, ids) = match biblio {
        Some(b) => (b.jeux, Some(b.fiches), Some(b.ids)),
        None => (crate::gog_galaxy::union_sans_bibliotheque(page, &ranges), None, None),
    };
    let id_galaxy = if galaxy_sur { galaxy } else { String::new() };
    let nom = pseudo_gog(&id_galaxy).await;
    let lu = crate::boutiques::LectureCompteGog { id_site: site, id_galaxy, nom, jeux, ids_lus: ids, ancien_id_galaxy };
    let quand = crate::noyau::maintenant();
    let mut ch = crate::boutiques::ChangementGog::default();
    let ecrit = crate::boutiques::modifier_source(dossier, "gog", |c| {
        let (mut c, x) = crate::boutiques::appliquer_import_gog_profil(c, &avant, &lu, &quand)?;
        if let Some(f) = fiches {
            crate::boutiques::fusionner_fiches_gog(&mut c, f);
        }
        ch = x;
        Ok(Some(c))
    });
    let ecrit = match ecrit {
        Ok(e) => e,
        Err(e) => {
            // Rien n'est écrit : le jeton provisoire part, le nom principal n'a pas bougé (lecture rejouable).
            if change {
                let _ = noyau.coffre.oublier(&secret);
            }
            return Err(e);
        }
    };
    // La liste est écrite : chaque jeton prend son vrai nom.
    poser_jetons_apres_ecriture(noyau, profil, change.then_some(secret.as_str()), ch.fondu.as_deref());
    if ch.ancien_profil.is_some() {
        noyau.journaliser(&format!("import GOG (connexion) : le navigateur du profil {profil} montre un autre compte GOG ; l'ancien reste, comme compte ajouté (aucune carte retirée)"));
    }
    let n = ecrit.as_ref().and_then(|c| c.comptes_gog.iter().find(|x| x.navigateur == NavigateurGog::Profil)).map_or(0, |x| x.jeux.len());
    noyau.journaliser(&format!("import GOG (connexion) : {n} jeu(x) pour le compte du navigateur du profil {profil}"));
    Ok(Some(lue))
}

// Relit un compte GOG AJOUTÉ par son seul jeton de renouvellement (jamais la session du navigateur du profil) et range
/// SES jeux. Une lecture ratée ne touche à rien (ses jeux déjà rangés restent).
async fn importer_compte_gog_ajoute(app: &AppHandle, noyau: &Noyau, dossier: &std::path::Path, profil: &str, x: &crate::boutiques::CompteGog) -> ResultatCompteGog {
    let mut res = ResultatCompteGog { cle: x.cle(), nom: x.nom.clone(), numero: x.numero, ..Default::default() };
    if x.id_galaxy.is_empty() {
        res.reconnecter = true;
        return res;
    }
    let secret = crate::boutiques::secret_gog_ajoute(profil, &x.id_galaxy);
    let perdu = |noyau: &Noyau| !noyau.coffre.lire(&secret).is_ok_and(|s| s.is_some());
    let Some(acces) = jeton_acces_gog(app, noyau, dossier, profil, &secret, &x.id_galaxy, None).await else {
        res.reconnecter = perdu(noyau);
        res.partiel = !res.reconnecter;
        noyau.journaliser(&format!("import GOG (compte ajouté) : pas de connexion ({})", if res.reconnecter { "à reconnecter" } else { "nouvel essai plus tard" }));
        return res;
    };
    let Some(b) = bibliotheque_gog(noyau, dossier, &secret, &x.id_galaxy, acces, &[], &x.jeux).await else {
        res.reconnecter = perdu(noyau);
        res.partiel = !res.reconnecter;
        return res;
    };
    let nom = pseudo_gog(&x.id_galaxy).await;
    if let Some(n) = &nom {
        res.nom = n.clone();
    }
    let n = b.jeux.len();
    let lu = crate::boutiques::LectureCompteGog { id_site: String::new(), id_galaxy: x.id_galaxy.clone(), nom, jeux: b.jeux, ids_lus: Some(b.ids), ancien_id_galaxy: None };
    let quand = crate::noyau::maintenant();
    let fiches = b.fiches;
    match crate::boutiques::modifier_source(dossier, "gog", |c| {
        let mut c = crate::boutiques::appliquer_import_gog_ajoute(c, &lu, &quand)?;
        crate::boutiques::fusionner_fiches_gog(&mut c, fiches);
        Ok(Some(c))
    }) {
        Ok(_) => noyau.journaliser(&format!("import GOG (compte ajouté) : {n} jeu(x)")),
        Err(e) => {
            noyau.journaliser(&format!("import GOG (compte ajouté) : rien n'est écrit ({e})"));
            res.partiel = true;
        }
    }
    res
}

/// Lot C4a (décisions de Seb du 21/11) : importe les jeux POSSÉDÉS du compte GOG connecté dans le navigateur du profil
/// (la connexion faite pour le jeu offert vaut accord), jeux cachés compris, sans GOG Galaxy. Lot C5 : puis la
/// bibliothèque de GOG Galaxy (connexion façon Galaxy), qui fait foi pour les jeux qu'elle connaît ; la liste du site
/// reste le garde-fou. `montrer` : juste après « Se connecter à GOG », la page de connexion façon Galaxy peut
/// s'afficher si GOG la demande (jamais en lecture discrète). Lot C4b : puis chaque compte AJOUTÉ, l'un après l'autre,
/// par son seul jeton ; la lecture ratée d'un compte ne touche jamais les autres. Tout est rangé UNE fois à la fin.
#[tauri::command]
pub async fn boutique_gog_importer_connecte(app: AppHandle, noyau: State<'_, Noyau>, montrer: Option<bool>, reconnexion: Option<bool>) -> Resultat<ImportGog> {
    // Une lecture de tous les comptes à la fois (les suivantes attendent son tour).
    let _un_seul = COMPTES_GOG.lock().await;
    // La session AU DÉBUT : la liste lue ne va qu'au profil qui l'a demandée.
    let s = noyau.session().await?;
    let p = s.profil.clone();
    let dossier = dossier_profil_de(&noyau, &p.id);
    let comptes = |d: &std::path::Path| crate::boutiques::lire_source(d, "gog").map(|c| crate::boutiques::comptes_gog_lus(&c)).unwrap_or_default();
    let mut r = ImportGog::default();
    let mut lus = 0usize;
    let mut echec = None;
    // Lot C4b (revue) : le compte du navigateur a été OUBLIÉ : sa session n'est plus relue, sauf juste après que la
    // personne s'est reconnectée elle-même (`reconnexion` : sa fenêtre de connexion vient de se fermer).
    let oublie = crate::boutiques::lire_source(&dossier, "gog").is_some_and(|c| c.navigateur_gog_oublie);
    if oublie && reconnexion != Some(true) {
        noyau.journaliser(&format!("import GOG (connexion) : compte du navigateur oublié pour le profil {} : sa session n'est pas relue", p.id));
    } else {
        match importer_compte_gog_profil(&app, &noyau, &dossier, &p.id, &comptes(&dossier), montrer.unwrap_or(false)).await {
            Ok(Some(b)) => {
                lus += 1;
                r.bibliotheque_galaxy = b;
            }
            Ok(None) => r.reconnecter = true,
            Err(e) => {
                r.erreur = Some(e.to_string());
                echec = Some(e);
            }
        }
    }
    // Les comptes ajoutés, relus APRÈS l'écriture du compte `Profil` (un changement de compte a pu en créer un).
    for x in comptes(&dossier).into_iter().filter(|x| x.navigateur == crate::boutiques::NavigateurGog::Ajoute) {
        let res = importer_compte_gog_ajoute(&app, &noyau, &dossier, &p.id, &x).await;
        if !res.reconnecter && !res.partiel {
            lus += 1;
        }
        r.comptes.push(res);
    }
    if let Some(e) = echec {
        if r.comptes.is_empty() {
            return Err(e);
        }
    }
    r.lus = lus;
    if lus == 0 {
        return Ok(r);
    }
    // Le compte ouvert dans GOG Galaxy sur ce PC (copie de sa base, effacée ensuite), relu à chaque lecture réussie.
    noter_compte_galaxy_du_pc(&noyau, &dossier).await;
    let total = crate::boutiques::jeux_gog_reunis(&comptes(&dossier)).len();
    noyau.journaliser(&format!("import GOG : {total} jeu(x) réunis pour le profil {} ({} compte(s) lu(s))", p.id, lus));
    let (l, nouveaux) = ranger_gog_importe(&noyau, &s, &dossier).await?;
    r.total = total;
    r.nouveaux_installes = nouveaux_installes("gog", &l, &nouveaux);
    r.nouveaux = nouveaux.len();
    r.galaxy = tauri::async_runtime::spawn_blocking(crate::boutiques_pc::galaxy_present).await.unwrap_or(false);
    Ok(r)
}

/// Lot C4b : relit le compte ouvert dans GOG Galaxy sur ce PC et le note dans la source « gog » du profil (vide s'il
/// n'est pas sûr). Une écriture ratée ne gêne rien (la ligne « Galaxy sur un autre compte » attendra).
async fn noter_compte_galaxy_du_pc(noyau: &Noyau, dossier: &std::path::Path) {
    let travail = noyau.dossier.join("travail").join("galaxy-compte");
    let compte = tauri::async_runtime::spawn_blocking(move || crate::boutiques::dossier_galaxy().and_then(|s| crate::boutiques::compte_galaxy_du_pc(&s, &travail))).await.ok().flatten().map(|g| g.compte).unwrap_or_default();
    let r = crate::boutiques::modifier_source(dossier, "gog", |c| {
        let Some(mut c) = c else { return Ok(None) };
        c.compte_galaxy_pc = compte;
        Ok(Some(c))
    });
    if let Err(e) = r {
        noyau.journaliser(&format!("import GOG : compte de GOG Galaxy pas noté ({e})"));
    }
}

// ───────────────────────────── GOG : ajouter et oublier un compte (lot C4b) ─────────────────────────────

/// Ce que « ➕ Ajouter un compte GOG » a donné (lot C4b). `fait` faux : fenêtre fermée sans connexion (rien n'a changé).
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct AjoutGog {
    pub fait: bool,
    pub cle: String,
    /// Le pseudo GOG (vide : l'interface dit « Compte GOG n », `numero`).
    pub nom: String,
    pub numero: u32,
    /// C'est le seul compte GOG du profil (le message ne dit pas « ton autre compte »).
    pub premier: bool,
    /// Un compte déjà là dont la connexion était perdue : reconnecté.
    pub reconnexion: bool,
    /// Ses jeux (bibliothèque de Galaxy) ; `partiel` : pas lue en entier pour l'instant.
    pub jeux: usize,
    pub partiel: bool,
    pub nouveaux: usize,
    pub nouveaux_installes: usize,
    pub galaxy: bool,
}

/// Le dossier des navigateurs temporaires des ajouts de comptes GOG d'un profil.
fn racine_ajouts_gog(dossier_profil: &std::path::Path) -> std::path::PathBuf {
    dossier_profil.join("navigateurs-gog")
}

/// Un nom de dossier d'ajout : « ajout-<chiffres> ».
fn nom_d_ajout_gog(n: &str) -> bool {
    n.strip_prefix("ajout-").is_some_and(|h| !h.is_empty() && h.len() <= 30 && h.bytes().all(|c| c.is_ascii_digit()))
}

/// Un point d'analyse (lien symbolique, jonction) : jamais suivi ni effacé à travers.
fn point_d_analyse(m: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if m.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    m.file_type().is_symlink()
}

/// Efface le dossier temporaire d'un ajout de compte GOG, AVEC PREUVE (testé) : un nom « ajout-<chiffres> », pas un
/// point d'analyse (ni lui ni sa racine), son chemin canonisé directement SOUS `<profil>/navigateurs-gog/` ; puis
/// relu : absent. `Ok` : effacé (ou déjà absent). Sinon la raison (WebView2 tient encore des fichiers…) : rien n'est
/// forcé, la prochaine ouverture d'un ajout réessaie.
pub(crate) fn effacer_ajout_gog(racine: &std::path::Path, d: &std::path::Path) -> Result<(), &'static str> {
    if !d.file_name().and_then(|n| n.to_str()).is_some_and(nom_d_ajout_gog) {
        return Err("nom-inattendu");
    }
    let m = match std::fs::symlink_metadata(d) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err("illisible"),
    };
    if point_d_analyse(&m) || std::fs::symlink_metadata(racine).map_or(true, |r| point_d_analyse(&r)) || !m.is_dir() {
        return Err("point-d-analyse");
    }
    let (Ok(rc), Ok(dc)) = (std::fs::canonicalize(racine), std::fs::canonicalize(d)) else { return Err("illisible") };
    if dc.parent() != Some(rc.as_path()) {
        return Err("hors-du-dossier");
    }
    let _ = std::fs::remove_dir_all(&dc);
    if std::fs::symlink_metadata(d).is_ok() {
        return Err("verrouille");
    }
    Ok(())
}

/// Efface les dossiers d'ajouts restés d'une fois précédente (WebView2 les tenait encore) ; jamais pendant un ajout.
fn effacer_restes_ajouts_gog(noyau: &Noyau, racine: &std::path::Path) {
    let Ok(l) = std::fs::read_dir(racine) else { return };
    let (mut faits, mut restes) = (0, 0);
    for e in l.flatten() {
        if e.file_name().to_str().is_some_and(nom_d_ajout_gog) {
            match effacer_ajout_gog(racine, &e.path()) {
                Ok(()) => faits += 1,
                Err(_) => restes += 1,
            }
        }
    }
    if faits + restes > 0 {
        noyau.journaliser(&format!("ajout de compte GOG : {faits} dossier(s) temporaire(s) d'avant effacé(s), {restes} encore tenu(s)"));
    }
}

/// Lot C4b (Seb 22/11) : « ➕ Ajouter un compte GOG ». La page officielle de connexion de GOG s'ouvre, MONTRÉE, en
/// navigation privée, dans un dossier temporaire (rien n'est gardé : le jeton de renouvellement suffit ; dossier effacé
/// avec preuve dès la fenêtre fermée). Le compte obtenu ne doit pas être déjà là (« Ce compte GOG est déjà dans
/// Frogtend »), sauf un compte ajouté dont la connexion était perdue (reconnecté). Puis sa bibliothèque de Galaxy est
/// lue et ses jeux entrent dans la ludothèque (se connecter vaut accord). Rien du compte n'est journalisé.
#[tauri::command]
pub async fn boutique_gog_ajouter_compte(app: AppHandle, noyau: State<'_, Noyau>) -> Resultat<AjoutGog> {
    let Ok(_un_seul) = COMPTES_GOG.try_lock() else { return Err(Erreur::Refus(MOTIF_GOG_OCCUPE.into())) };
    if app.get_webview_window(FENETRE_GOG_JEUX).is_some() {
        return Err(Erreur::Refus(MOTIF_GOG_OCCUPE.into()));
    }
    let s = noyau.session().await?;
    let p = s.profil.clone();
    let dossier = dossier_profil_de(&noyau, &p.id);
    let racine = racine_ajouts_gog(&dossier);
    effacer_restes_ajouts_gog(&noyau, &racine);
    let horodatage = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let donnees = racine.join(format!("ajout-{horodatage}"));
    std::fs::create_dir_all(&donnees)?;
    let f = FenetreGog { etiquette: FENETRE_GOG_AJOUT, donnees: donnees.clone(), montrer: true, ajout: true, attendu: String::new() };
    let code = code_connexion_gog(&app, &noyau, &p.id, f).await;
    // La fenêtre est fermée : son dossier part (WebView2 peut le tenir encore un instant : quelques essais).
    let mut efface = Err("pas-essaye");
    for _ in 0..8 {
        efface = effacer_ajout_gog(&racine, &donnees);
        if efface.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    if let Err(raison) = efface {
        noyau.journaliser(&format!("ajout de compte GOG : dossier temporaire pas encore effacé ({raison}) ; nouvel essai au prochain ajout"));
    }
    let Some(code) = code? else {
        noyau.journaliser("ajout de compte GOG : fenêtre fermée sans connexion");
        return Ok(AjoutGog::default());
    };
    let j = crate::gog_galaxy::jetons_par_code(crate::gog_galaxy::AUTH, &code).await?;
    drop(code);
    let id = j.user_id.clone();
    let comptes = crate::boutiques::lire_source(&dossier, "gog").map(|c| crate::boutiques::comptes_gog_lus(&c)).unwrap_or_default();
    let secret = crate::boutiques::secret_gog_ajoute(&p.id, &id);
    let existant = comptes.iter().find(|x| x.id_galaxy == id);
    let reconnexion = match existant {
        Some(x) if x.navigateur == crate::boutiques::NavigateurGog::Ajoute && !noyau.coffre.lire(&secret).is_ok_and(|s| s.is_some()) => true,
        Some(_) => return Err(Erreur::Refus(crate::boutiques::MOTIF_GOG_DEJA_LA.into())),
        None => false,
    };
    // Revue du lot C4b : le jeton vit sous un nom PROVISOIRE tant que la liste n'est pas écrite (jamais un `#<numéro>`
    // existant écrasé, jamais un jeton rangé pour un compte qui n'est pas dans la liste).
    let provisoire = secret_gog_provisoire(&p.id, "ajout");
    noyau.coffre.ranger(&provisoire, &j.renouvellement).map_err(|_| Erreur::Coffre("Le coffre de Windows refuse la connexion de ce compte GOG : réessaie.".into()))?;
    let ranges = existant.map(|x| x.jeux.clone()).unwrap_or_default();
    let biblio = bibliotheque_gog(&noyau, &dossier, &provisoire, &id, j.acces, &[], &ranges).await;
    let nom = pseudo_gog(&id).await;
    let partiel = biblio.is_none();
    let (jeux, fiches, ids) = match biblio {
        Some(b) => (b.jeux, Some(b.fiches), Some(b.ids)),
        None => (ranges, None, None),
    };
    let lu = crate::boutiques::LectureCompteGog { id_site: String::new(), id_galaxy: id.clone(), nom, jeux, ids_lus: ids, ancien_id_galaxy: None };
    let quand = crate::noyau::maintenant();
    let ecrit = crate::boutiques::modifier_source(&dossier, "gog", |c| {
        let mut c = crate::boutiques::ajouter_compte_gog(c, &lu, reconnexion, &quand)?;
        if let Some(f) = fiches {
            crate::boutiques::fusionner_fiches_gog(&mut c, f);
        }
        Ok(Some(c))
    });
    let ecrit = match ecrit {
        Ok(e) => e,
        Err(e) => {
            // Rien n'est écrit : le jeton provisoire part.
            let _ = noyau.coffre.oublier(&provisoire);
            return Err(e);
        }
    };
    // La liste est écrite : le jeton prend son vrai nom (libre par construction), puis le nom provisoire part. Un jeton
    // refusé pendant la lecture (oublié sous le nom provisoire) : le compte est « à reconnecter ».
    if let Ok(Some(t)) = noyau.coffre.lire(&provisoire) {
        if !ranger_si_libre(&noyau, &secret, &t) {
            noyau.journaliser("ajout de compte GOG : le coffre de Windows refuse le jeton (compte à reconnecter)");
        }
    }
    let _ = noyau.coffre.oublier(&provisoire);
    let comptes = ecrit.map(|c| c.comptes_gog).unwrap_or_default();
    let x = comptes.iter().find(|x| x.id_galaxy == id).cloned().unwrap_or_default();
    let premier = comptes.len() == 1;
    noyau.journaliser(&format!(
        "ajout de compte GOG pour le profil {} : {} ({} jeu(x){})",
        p.id,
        if reconnexion { "reconnecté" } else { "ajouté" },
        x.jeux.len(),
        if partiel { ", bibliothèque pas lue en entier" } else { "" }
    ));
    noter_compte_galaxy_du_pc(&noyau, &dossier).await;
    let (l, nouveaux) = ranger_gog_importe(&noyau, &s, &dossier).await?;
    let galaxy = tauri::async_runtime::spawn_blocking(crate::boutiques_pc::galaxy_present).await.unwrap_or(false);
    Ok(AjoutGog {
        fait: true,
        cle: x.cle(),
        nom: x.nom.clone(),
        numero: x.numero,
        premier,
        reconnexion,
        jeux: x.jeux.len(),
        partiel,
        nouveaux: nouveaux.len(),
        nouveaux_installes: nouveaux_installes("gog", &l, &nouveaux),
        galaxy,
    })
}

// Ce que « Oublier ce compte » GOG a fait (lot C4b) : le compte est TOUJOURS retiré de la liste ; son jeton n'est dit
/// oublié que si le coffre l'a retiré ; le compte `Profil` : déconnexion du navigateur du profil (cookies gog.com
/// seulement), dite seulement PROUVÉE par relecture.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct OubliGog {
    pub profil: bool,
    pub secret_oublie: bool,
    pub navigateur_deconnecte: bool,
}

/// Lot C4b : « Oublier ce compte » GOG. La liste est réécrite D'ABORD (le compte n'est plus lu), PUIS son jeton quitte le
/// coffre ; le compte `Profil` : puis les seuls cookies de gog.com du navigateur du profil (Epic, Amazon, PlayStation y
/// vivent : jamais tout le navigateur). Les cartes de la ludothèque et les jeux installés ne bougent pas.
#[tauri::command]
pub async fn boutique_gog_oublier_compte(app: AppHandle, noyau: State<'_, Noyau>, cle: String) -> Resultat<OubliGog> {
    if cle.is_empty() || cle.len() > 40 || !cle.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(Erreur::Refus("Compte GOG invalide.".into()));
    }
    let Ok(_un_seul) = COMPTES_GOG.try_lock() else { return Err(Erreur::Refus(MOTIF_GOG_OCCUPE.into())) };
    let p = profil_ouvert(&noyau).await?;
    let dossier = dossier_profil_de(&noyau, &p.id);
    let comptes = crate::boutiques::lire_source(&dossier, "gog").map(|c| crate::boutiques::comptes_gog_lus(&c)).unwrap_or_default();
    let x = comptes.iter().find(|x| x.cle() == cle).ok_or_else(|| Erreur::Introuvable("Ce compte GOG n'est plus dans Frogtend.".into()))?;
    // Les refus D'ABORD : le compte `Profil` ne s'oublie pas pendant qu'une fenêtre de GOG tient son navigateur.
    if x.navigateur == crate::boutiques::NavigateurGog::Profil && [FENETRE_GOG, FENETRE_GOG_JEUX, FENETRE_GOG_GALAXY].iter().any(|e| app.get_webview_window(e).is_some()) {
        return Err(Erreur::Refus("Termine d'abord ce qui se passe dans la fenêtre de GOG (connexion ou jeu offert), puis réessaie.".into()));
    }
    let mut oublie = None;
    crate::boutiques::modifier_source(&dossier, "gog", |c| {
        let (c, x) = crate::boutiques::oublier_compte_gog(c, &cle)?;
        oublie = Some(x);
        Ok(Some(c))
    })?;
    let x = oublie.ok_or_else(|| Erreur::Introuvable("Ce compte GOG n'est plus dans Frogtend.".into()))?;
    let profil = x.navigateur == crate::boutiques::NavigateurGog::Profil;
    let secret_oublie = if !profil && x.id_galaxy.is_empty() { true } else { noyau.coffre.oublier(&crate::boutiques::secret_compte_gog(&p.id, x.navigateur, &x.id_galaxy)).is_ok() };
    noyau.journaliser(&format!(
        "compte GOG oublié pour le profil {} ({}) ; jeton {} ; ses jeux restent dans la ludothèque",
        p.id,
        if profil { "compte du navigateur" } else { "compte ajouté" },
        if secret_oublie { "retiré du coffre" } else { "PAS retiré du coffre" }
    ));
    let mut r = OubliGog { profil, secret_oublie, navigateur_deconnecte: false };
    if profil {
        r.navigateur_deconnecte = match deconnecter_navigateur_gog(&app, &noyau, &dossier).await {
            Ok(v) => v,
            Err(e) => {
                noyau.journaliser(&format!("oubli GOG : déconnexion du navigateur impossible ({e})"));
                false
            }
        };
    }
    Ok(r)
}

/// Lot C4b : efface les SEULS cookies de gog.com du navigateur du profil (`boutiques::domaine_gog`), pour que la
/// session suivante demande vraiment un compte. Jamais le dossier entier ni `clear_all_browsing_data` : Epic, Amazon et
/// PlayStation y vivent. Une fenêtre CACHÉE sur `about:blank`, sous l'étiquette de la lecture des jeux GOG (jamais les
/// deux à la fois) ; seule la RELECTURE prouve (plus aucun cookie gog.com, 3 passes au plus). Jamais une valeur lue,
/// journalisée ni rendue.
async fn deconnecter_navigateur_gog(app: &AppHandle, noyau: &Noyau, dossier: &std::path::Path) -> Resultat<bool> {
    use crate::boutiques::{domaine_gog, CookieSansValeur};
    if !crate::gratuits::dossier_navigateur(dossier).is_dir() {
        noyau.journaliser("oubli GOG : pas de navigateur pour ce profil, rien à effacer");
        return Ok(true);
    }
    let vide = tauri::Url::parse("about:blank").map_err(|_| Erreur::Refus("Adresse invalide.".into()))?;
    let w = tauri::WebviewWindowBuilder::new(app, FENETRE_GOG_JEUX, tauri::WebviewUrl::External(vide))
        .title("GOG — Frogtend")
        .inner_size(400.0, 300.0)
        .visible(false)
        .focused(false)
        .skip_taskbar(true)
        .data_directory(crate::gratuits::dossier_navigateur(dossier))
        .on_new_window(crate::gratuits::fenetre_d_automatisation)
        .on_navigation(|u| u.as_str() == "about:blank")
        .build()
        .map_err(|_| Erreur::Refus("La déconnexion de GOG dans Frogtend n'a pas pu commencer (une fenêtre de GOG est encore ouverte).".into()))?;
    // `cookies()` se bloque sur le fil principal sous Windows (documentation de tauri 2.12) : dans un fil à part.
    let lire = |w: tauri::WebviewWindow| async move {
        tauri::async_runtime::spawn_blocking(move || {
            w.cookies().map(|l| {
                l.iter()
                    .map(|c| CookieSansValeur {
                        nom: c.name().to_string(),
                        domaine: c.domain().unwrap_or("").to_string(),
                        chemin: c.path().filter(|p| p.starts_with('/')).unwrap_or("/").to_string(),
                    })
                    .collect::<Vec<_>>()
            })
        })
        .await
        .ok()
        .and_then(|r| r.ok())
    };
    let mut prouve = false;
    let mut restants = None;
    for passe in 0..4 {
        let Some(lus) = lire(w.clone()).await else { break };
        let a_effacer: Vec<CookieSansValeur> = lus.into_iter().filter(|c| domaine_gog(&c.domaine)).collect();
        restants = Some(a_effacer.len());
        if a_effacer.is_empty() {
            prouve = true;
            break;
        }
        if passe == 3 {
            break;
        }
        let _ = w.with_webview(move |pw| effacer_cookies_webview2(&pw, &a_effacer));
        tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    }
    let _ = w.close();
    noyau.journaliser(&format!(
        "oubli GOG : navigateur {} ({} cookie(s) gog.com restant(s))",
        if prouve { "déconnecté de GOG" } else { "PAS déconnecté de GOG" },
        restants.map_or("illisibles".to_string(), |n| n.to_string())
    ));
    Ok(prouve)
}

/// Range la liste du compte GOG (déjà écrite) dans la ludothèque de la session `s` (prise au DÉBUT de l'import), avec
/// les jeux GOG installés lus sur ce PC (une carte par jeu) : le registre du PC ne reçoit que les jeux INSTALLÉS (jamais
/// la liste du compte). Rend les cartes rangées et les ids des NOUVELLES.
async fn ranger_gog_importe(noyau: &Noyau, s: &crate::noyau::Session, dossier: &std::path::Path) -> Resultat<(Vec<crate::boutiques::JeuBoutique>, Vec<i64>)> {
    // Jamais en même temps qu'une relecture des boutiques du PC.
    let _un_seul = RELECTURE.lock().await;
    let travail = noyau.dossier.join("travail").join("boutiques-pc-gog");
    let installes = tauri::async_runtime::spawn_blocking(move || {
        let l = crate::boutiques_pc::lire_seulement(&["gog"], &travail);
        let _ = std::fs::remove_dir_all(&travail);
        l.into_iter().next().and_then(|(_, i)| i)
    })
    .await
    .map_err(|_| Erreur::Disque("La lecture de GOG s'est arrêtée brutalement.".into()))?;
    let installes = match installes {
        Some(i) => {
            synchroniser_registre(noyau, "gog", &i);
            i
        }
        // Registre de GOG illisible (ou jamais écrit sur ce PC) : on ne sait rien de ce qui est installé. Un jeu déjà
        // rangé « installé » le RESTE (jamais un jeu installé qui passe « pas installé » sur une lecture ratée) ; le
        // registre du PC ne bouge pas.
        None => {
            let c = s.verrou();
            let mut cartes = c.jeux_de_source("gog")?;
            cartes.extend(c.jeux_de_source("galaxy")?.into_iter().filter(|j| j.boutique.as_deref() == Some("gog")));
            crate::noyau::installes_gog_deja_ranges(&cartes)
        }
    };
    let l = cartes_gog(dossier, &installes);
    let nouveaux = noyau.ranger_installes(s, "gog", &l)?;
    Ok((l, nouveaux))
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
pub async fn gratuits_liste(noyau: State<'_, Noyau>, possedes: Option<bool>) -> Resultat<Vec<Offert>> {
    let p = profil_ouvert(&noyau).await?;
    let obtenus = crate::gratuits::lire_obtenus(&dossier_profil_de(&noyau, &p.id));
    let l = crate::gratuits::offerts_epic().await?;
    // Un jeu déjà dans le compte Epic de CE PC (catalogue du lanceur, lecture seule) : « obtenu » sans rien cliquer
    // (Seb 19/11). Ce compte est celui du LANCEUR : seulement si le profil a accepté « les jeux de mes comptes » (lot B),
    // sinon un autre membre de la famille verrait « obtenu » pour un jeu qu'il n'a pas.
    let possedes = if possedes.unwrap_or(false) {
        tokio::task::spawn_blocking(|| crate::boutiques_pc::espaces_possedes_epic(crate::boutiques_pc::catalogue_epic().as_deref())).await.ok().flatten().unwrap_or_default()
    } else {
        Default::default()
    };
    Ok(l
        .into_iter()
        .map(|j| {
            let obtenu = obtenus.jeux.iter().any(|(b, s, _)| *b == j.boutique && *s == j.slug)
                || (j.boutique == "epic" && !j.namespace.is_empty() && possedes.contains(&j.namespace));
            Offert { jeu: j, obtenu }
        })
        .collect())
}

const FENETRE_EPIC: &str = "boutique-epic";

/// La fenêtre de connexion à Epic, avec le navigateur PROPRE AU PROFIL (sa connexion Epic y reste). Le script d'Epic
/// y tourne en mode connexion (aucun clic) : quand la boutique montre le compte connecté, l'interface l'apprend par
/// l'événement `gratuits-connecte` ({boutique, profil}).
fn fenetre_epic(app: &AppHandle, dossier_profil: &std::path::Path, profil: &str, adresse: &str) -> Resultat<tauri::WebviewWindow> {
    if let Some(w) = app.get_webview_window(FENETRE_EPIC) {
        let _ = w.close();
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
    let url = tauri::Url::parse(adresse).map_err(|_| Erreur::Refus("Adresse Epic invalide.".into()))?;
    tauri::WebviewWindowBuilder::new(app, FENETRE_EPIC, tauri::WebviewUrl::External(url))
        .title("Epic Games — Frogtend")
        .inner_size(1100.0, 800.0)
        .center()
        .data_directory(crate::gratuits::dossier_navigateur(dossier_profil))
        .on_new_window(crate::gratuits::fenetre_de_connexion)
        .on_page_load(|w, charge| {
            if matches!(charge.event(), tauri::webview::PageLoadEvent::Finished) {
                let _ = w.eval(crate::gratuits::script_epic_connexion());
            }
        })
        .on_document_title_changed({
            let (app, profil) = (app.clone(), profil.to_string());
            move |_, titre| {
                if crate::gratuits::lire_titre(&titre) == Some(crate::gratuits::Obtention::Connecte) {
                    let _ = app.emit("gratuits-connecte", serde_json::json!({"boutique": "epic", "profil": profil}));
                }
            }
        })
        .build()
        .map_err(|e| Erreur::Disque(format!("La fenêtre d'Epic ne s'ouvre pas ({e}).")))
}

/// Se connecter à Epic, une fois : la page officielle s'ouvre ; Frogtend ne voit jamais le mot de passe.
#[tauri::command]
pub async fn gratuits_connexion_epic(app: AppHandle, noyau: State<'_, Noyau>) -> Resultat<()> {
    let p = profil_ouvert(&noyau).await?;
    let w = fenetre_epic(&app, &dossier_profil_de(&noyau, &p.id), &p.id, "https://www.epicgames.com/id/login?redirectUrl=https%3A%2F%2Fstore.epicgames.com%2Fen-US%2F")?;
    let _ = w.set_focus();
    Ok(())
}

const FENETRE_EPIC_VERIFICATION: &str = "boutique-epic-verification";

/// Le VRAI état du compte Epic du profil (Seb 20/11 : connecté le 18/11 avant que Frogtend sache le retenir, il voyait
/// encore « Se connecter ») : la boutique s'ouvre CACHÉE avec le navigateur du profil, le script ne clique sur rien et
/// dit seulement « connecté » ou « pas connecté ». Vrai / faux, ou None si la page n'a rien dit de sûr (30 s au plus)
/// ou si une fenêtre d'Epic est déjà ouverte (la personne s'y connecte : on ne la dérange pas). La fenêtre n'est
/// jamais montrée et se ferme toujours.
#[tauri::command]
pub async fn gratuits_verifier_epic(app: AppHandle, noyau: State<'_, Noyau>) -> Resultat<Option<bool>> {
    let p = profil_ouvert(&noyau).await?;
    if app.get_webview_window(FENETRE_EPIC).is_some() || app.get_webview_window(FENETRE_EPIC_VERIFICATION).is_some() {
        return Ok(None);
    }
    let dossier = dossier_profil_de(&noyau, &p.id);
    // Un profil qui n'a jamais ouvert de page de boutique n'a pas de navigateur : il n'est donc pas connecté à Epic, et
    // Epic n'est pas contacté sans un geste de la personne (revue du 20/11).
    if !crate::gratuits::dossier_navigateur(&dossier).is_dir() {
        return Ok(None);
    }
    let (envoi, reception) = std::sync::mpsc::channel::<crate::gratuits::Obtention>();
    let envoi = std::sync::Mutex::new(envoi);
    let url = tauri::Url::parse("https://store.epicgames.com/en-US/").map_err(|_| Erreur::Refus("Adresse Epic invalide.".into()))?;
    let w = tauri::WebviewWindowBuilder::new(&app, FENETRE_EPIC_VERIFICATION, tauri::WebviewUrl::External(url))
        .title("Epic Games — Frogtend")
        .inner_size(1100.0, 800.0)
        .visible(false)
        .data_directory(crate::gratuits::dossier_navigateur(&dossier))
        .on_new_window(crate::gratuits::fenetre_d_automatisation)
        .on_page_load(|w, charge| {
            if matches!(charge.event(), tauri::webview::PageLoadEvent::Finished) {
                let _ = w.eval(crate::gratuits::script_epic_verification());
            }
        })
        .on_document_title_changed(move |_, titre| {
            if let Some(r) = crate::gratuits::lire_titre(&titre) {
                let _ = envoi.lock().map(|e| e.send(r));
            }
        })
        .build()
        .map_err(|e| Erreur::Disque(format!("La vérification d'Epic ne s'ouvre pas ({e}).")))?;
    let r = tauri::async_runtime::spawn_blocking(move || reception.recv_timeout(std::time::Duration::from_secs(30)).ok())
        .await
        .unwrap_or(None);
    let _ = w.close();
    // La fenêtre « Se connecter » ou une obtention s'est ouverte pendant l'attente : leur résultat, plus récent, compte.
    let vu = if app.get_webview_window(FENETRE_EPIC).is_some() { None } else { r.as_ref().and_then(crate::gratuits::compte_epic_vu) };
    noyau.journaliser(&format!(
        "vérification du compte Epic : {}",
        match vu {
            Some(true) => "connecté",
            Some(false) => "pas connecté",
            None => "pas sûr (état connu gardé)",
        }
    ));
    Ok(vu)
}

const FENETRE_STEAM: &str = "boutique-steam";

/// Se connecter à Steam par sa page OFFICIELLE (lot C1, décision de Seb du 19/11), avec le navigateur PROPRE AU PROFIL
/// (sa connexion Steam y reste ; Steam Guard se fait sur la vraie page). Frogtend ne voit ni le mot de passe ni les
/// cookies : à chaque page de la boutique, `steam.js` dit seulement si un compte est connecté et son `steamid`
/// (revérifié ici). Une fois connecté : le `steamid` est rangé dans `boutiques.json` du profil (jamais d'autre donnée),
/// l'interface l'apprend par l'événement `steam-connecte` ({profil, ok}) et la fenêtre se ferme.
#[tauri::command]
pub async fn boutique_steam_connexion(app: AppHandle, noyau: State<'_, Noyau>) -> Resultat<()> {
    let p = profil_ouvert(&noyau).await?;
    if OUBLI_STEAM_EN_COURS.load(std::sync::atomic::Ordering::SeqCst) {
        return Err(Erreur::Refus("Frogtend finit d'oublier ton compte Steam : réessaie dans quelques secondes.".into()));
    }
    let dossier = dossier_profil_de(&noyau, &p.id);
    if let Some(w) = app.get_webview_window(FENETRE_STEAM) {
        let _ = w.close();
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }
    let url = tauri::Url::parse(crate::gratuits::CONNEXION_STEAM).map_err(|_| Erreur::Refus("Adresse Steam invalide.".into()))?;
    // Une seule annonce par fenêtre (chaque page de la boutique redit « connecté »).
    let fait = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let w = tauri::WebviewWindowBuilder::new(&app, FENETRE_STEAM, tauri::WebviewUrl::External(url))
        .title("Steam — Frogtend")
        .inner_size(1100.0, 800.0)
        .center()
        .data_directory(crate::gratuits::dossier_navigateur(&dossier))
        .on_new_window(crate::gratuits::fenetre_de_connexion)
        // Seulement les pages de connexion de Steam : tout autre lien est refusé (revue de l'expert, 19/11).
        // Un refus est noté au journal avec le SEUL nom du site (jamais l'adresse entière, qui peut porter un jeton) :
        // si Steam Guard a besoin d'un autre site, le premier essai le dira.
        .on_navigation({
            let app = app.clone();
            move |u| {
                let ok = crate::gratuits::navigation_steam(u);
                if !ok {
                    app.state::<Noyau>().journaliser(&format!("connexion Steam : page refusée sur « {} »", u.host_str().unwrap_or("?")));
                }
                ok
            }
        })
        .on_page_load(|w, charge| {
            if matches!(charge.event(), tauri::webview::PageLoadEvent::Finished) && w.url().is_ok_and(|u| crate::gratuits::page_de_la_boutique_steam(&u)) {
                let _ = w.eval(crate::gratuits::SCRIPT_STEAM);
            }
        })
        .on_document_title_changed({
            let (app, profil) = (app.clone(), p.id.clone());
            move |w, titre| {
                // Le titre n'est cru que sur la boutique Steam elle-même (n'importe quelle page peut écrire ce titre).
                if !w.url().is_ok_and(|u| crate::gratuits::page_de_la_boutique_steam(&u)) {
                    return;
                }
                let Some(crate::gratuits::ConnexionSteam::Connecte(steamid)) = crate::gratuits::lire_titre_steam(&titre) else { return };
                if fait.swap(true, std::sync::atomic::Ordering::SeqCst) {
                    return;
                }
                let r = crate::boutiques::noter_connexion_steam(&dossier, &steamid, &crate::noyau::maintenant());
                if let Some(noyau) = app.try_state::<Noyau>() {
                    // Jamais le steamid ni rien de la page dans le journal : seulement le fait.
                    noyau.journaliser(&match &r {
                        Ok(false) => format!("connexion Steam : compte relié pour le profil {profil}"),
                        Ok(true) => format!("connexion Steam : AUTRE compte relié pour le profil {profil}"),
                        Err(e) => format!("connexion Steam : compte pas enregistré pour le profil {profil} ({e:?})"),
                    });
                }
                let (app, profil) = (app.clone(), profil.clone());
                // Hors de ce rappel (il vient de la fenêtre elle-même) : le ménage d'un changement de compte, PUIS
                // l'annonce (l'interface lance l'import juste après), puis la fermeture.
                tauri::async_runtime::spawn(async move {
                    if r.is_ok() {
                        menage_ancien_compte_steam(&app.state::<Noyau>(), &profil).await;
                    }
                    let _ = app.emit("steam-connecte", serde_json::json!({"profil": profil, "ok": r.is_ok()}));
                    if r.is_ok() {
                        let _ = w.close();
                    }
                });
            }
        })
        .build()
        .map_err(|e| Erreur::Disque(format!("La fenêtre de Steam ne s'ouvre pas ({e}).")))?;
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
    // Après une commande, la page du jeu est RECHARGÉE en mode contrôle : seule « In Library » vaut « obtenu » (Seb 20/11 :
    // « à toi » affiché alors qu'Epic proposait encore « Obtenir »).
    let controle = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let w = tauri::WebviewWindowBuilder::new(&app, FENETRE_EPIC, tauri::WebviewUrl::External(url.clone()))
        .title("Epic Games — Frogtend")
        .inner_size(1100.0, 800.0)
        .center()
        .visible(false)
        .data_directory(crate::gratuits::dossier_navigateur(&dossier))
        .on_new_window(crate::gratuits::fenetre_d_automatisation)
        .on_page_load({
            let controle = controle.clone();
            move |w, charge| {
                if matches!(charge.event(), tauri::webview::PageLoadEvent::Finished) {
                    if controle.load(std::sync::atomic::Ordering::SeqCst) {
                        let _ = w.eval(crate::gratuits::script_epic_controle());
                    } else {
                        let _ = w.eval(crate::gratuits::SCRIPT_EPIC);
                    }
                }
            }
        })
        .on_document_title_changed(move |_, titre| {
            if let Some(r) = crate::gratuits::lire_titre(&titre) {
                let _ = envoi.lock().map(|e| e.send(r));
            }
        })
        .build()
        .map_err(|e| Erreur::Disque(format!("La fenêtre d'Epic ne s'ouvre pas ({e}).")))?;
    let (reception, r) = tauri::async_runtime::spawn_blocking(move || {
        let r = reception.recv_timeout(std::time::Duration::from_secs(150));
        (reception, r)
    })
    .await
    .map_err(|_| Erreur::Disque("L'attente s'est arrêtée brutalement.".into()))?;
    let mut r = r.unwrap_or(crate::gratuits::Obtention::Erreur("aucune réponse de la page (délai dépassé)".into()));
    if r == crate::gratuits::Obtention::Obtenu {
        // La preuve : la page du jeu rechargée doit montrer « In Library ».
        while reception.try_recv().is_ok() {}
        controle.store(true, std::sync::atomic::Ordering::SeqCst);
        let preuve = if w.navigate(url).is_ok() {
            tauri::async_runtime::spawn_blocking(move || reception.recv_timeout(std::time::Duration::from_secs(60)).ok())
                .await
                .ok()
                .flatten()
        } else {
            None
        };
        r = crate::gratuits::apres_controle(preuve);
    }
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

/// Le drapeau « fermée par Frogtend » de la fenêtre `boutique-gog` OUVERTE qui annonce sa fermeture (connexion, ou
/// récupération du jeu offert affichée pour que la personne finisse). Chaque fenêtre a SON drapeau (capturé dans son
/// gestionnaire) : une fenêtre que Frogtend ferme lui-même (rouverte, remplacée) n'annonce jamais rien, et un drapeau
/// resté levé ne touche jamais une autre fenêtre.
static DRAPEAU_FENETRE_GOG: std::sync::Mutex<Option<std::sync::Arc<std::sync::atomic::AtomicBool>>> = std::sync::Mutex::new(None);

/// Ferme une fenêtre de boutique (jeux offerts, connexion) PAR FROGTEND : pour GOG, sa fermeture n'est pas annoncée.
fn fermer_par_frogtend(boutique: &str, w: &tauri::WebviewWindow) {
    if boutique == "gog" {
        if let Some(d) = DRAPEAU_FENETRE_GOG.lock().unwrap_or_else(|e| e.into_inner()).take() {
            d.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }
    let _ = w.close();
}

/// Lot C4a : quand la PERSONNE ferme cette fenêtre de GOG (connexion, ou jeu offert qui demandait de se connecter),
/// l'interface lit les jeux du compte (se connecter vaut accord), par l'événement `gratuits-ferme` {boutique, profil}.
fn annoncer_fermeture_gog(app: &AppHandle, w: &tauri::WebviewWindow, profil: &str) {
    let drapeau = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    *DRAPEAU_FENETRE_GOG.lock().unwrap_or_else(|e| e.into_inner()) = Some(drapeau.clone());
    let (a, profil) = (app.clone(), profil.to_string());
    w.on_window_event(move |e| {
        if matches!(e, tauri::WindowEvent::Destroyed) && !drapeau.load(std::sync::atomic::Ordering::SeqCst) {
            let _ = a.emit("gratuits-ferme", serde_json::json!({"boutique": "gog", "profil": profil}));
        }
    });
}

/// GOG : jamais une fenêtre de connexion ou de jeu offert pendant la lecture des jeux du compte (même compte, même
/// navigateur).
fn refuser_pendant_lecture_gog(app: &AppHandle, boutique: &str) -> Resultat<()> {
    if boutique == "gog" && (app.get_webview_window(FENETRE_GOG_JEUX).is_some() || app.get_webview_window(FENETRE_GOG_GALAXY).is_some()) {
        return Err(Erreur::Refus("Frogtend lit tes jeux GOG en ce moment : attends la fin (moins d'une minute en général), puis réessaie.".into()));
    }
    Ok(())
}

/// Se connecter à GOG ou à Prime Gaming, une fois : la page officielle s'ouvre (navigateur PROPRE AU PROFIL) ;
/// Frogtend ne voit jamais le mot de passe.
#[tauri::command]
pub async fn gratuits_connexion(app: AppHandle, noyau: State<'_, Noyau>, boutique: String) -> Resultat<()> {
    let (page, _, _) = crate::gratuits::boutique_offerte(&boutique).ok_or_else(|| Erreur::Refus("Boutique inconnue.".into()))?;
    let p = profil_ouvert(&noyau).await?;
    refuser_pendant_lecture_gog(&app, &boutique)?;
    let etiquette = format!("boutique-{boutique}");
    if let Some(w) = app.get_webview_window(&etiquette) {
        // Fermée par Frogtend (rouverte) : pas une fin de connexion à annoncer.
        fermer_par_frogtend(&boutique, &w);
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }
    let url = tauri::Url::parse(page).map_err(|_| Erreur::Refus("Adresse invalide.".into()))?;
    let w = tauri::WebviewWindowBuilder::new(&app, &etiquette, tauri::WebviewUrl::External(url))
        .title("Connexion — Frogtend")
        .inner_size(1280.0, 860.0)
        .center()
        .data_directory(crate::gratuits::dossier_navigateur(&dossier_profil_de(&noyau, &p.id)))
        .on_new_window(crate::gratuits::fenetre_de_connexion)
        .build()
        .map_err(|e| Erreur::Disque(format!("La fenêtre ne s'ouvre pas ({e}).")))?;
    // Lot C4a : la fenêtre de connexion fermée par la personne, l'interface lit les jeux du compte GOG.
    if boutique == "gog" {
        annoncer_fermeture_gog(&app, &w, &p.id);
    }
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
    refuser_pendant_lecture_gog(&app, &boutique)?;
    let etiquette = format!("boutique-{boutique}");
    let (envoi, reception) = std::sync::mpsc::channel::<crate::gratuits::Recolte>();
    let envoi = std::sync::Mutex::new(envoi);
    if let Some(w) = app.get_webview_window(&etiquette) {
        // Fermée par Frogtend (la récupération prend la place) : pas une fin de connexion à annoncer.
        fermer_par_frogtend(&boutique, &w);
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
    let url = tauri::Url::parse(page).map_err(|_| Erreur::Refus("Adresse invalide.".into()))?;
    let w = tauri::WebviewWindowBuilder::new(&app, &etiquette, tauri::WebviewUrl::External(url))
        .title("Jeux offerts — Frogtend")
        .inner_size(1280.0, 860.0)
        .center()
        .visible(false)
        .data_directory(crate::gratuits::dossier_navigateur(&dossier))
        .on_new_window(crate::gratuits::fenetre_d_automatisation)
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
        // Lot C4a : la personne finit dans cette fenêtre (connexion à GOG, souvent) : à sa fermeture, les jeux du compte
        // GOG sont lus, comme après la fenêtre de connexion.
        if boutique == "gog" {
            annoncer_fermeture_gog(&app, &w, &p.id);
        }
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
        .on_new_window(crate::gratuits::fenetre_de_connexion)
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
        .on_new_window(crate::gratuits::fenetre_d_automatisation)
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

/// Les jeux Epic installés sur CE PC (lot A : la relecture commune) : après un import de GOG Galaxy, ses cartes Epic
/// suivent de nouveau l'état lu chez Epic.
async fn synchroniser_epic(noyau: &Noyau) {
    if let Err(e) = relire_pc(noyau, &["epic"], false).await {
        noyau.journaliser(&format!("jeux Epic du PC : {e:?}"));
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
    let jeux = tauri::async_runtime::spawn_blocking(move || crate::boutiques::lire_galaxy_et_compte(&storage, &t2))
        .await
        .map_err(|_| Erreur::Disque("La lecture de GOG Galaxy s'est arrêtée brutalement.".into()));
    // La copie de la base ne reste pas, même si la lecture a échoué : elle contient les données du compte.
    let _ = std::fs::remove_dir_all(&travail);
    let (jeux, compte) = jeux??;
    // Lot C4b : le compte ouvert dans Galaxy (seulement s'il est sûr : un seul compte dans sa base), pour dire quand il
    // n'est aucun des comptes GOG de Frogtend.
    let c = crate::boutiques::CompteBoutique { compte: "GOG Galaxy".into(), steamid: String::new(), maj_le: crate::noyau::maintenant(), jeux, id_compte: compte.unwrap_or_default(), ..Default::default() };
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
    // RetroArch ne charge seul que `<nom du jeu lancé>.cht` : le fichier prend ce nom (la ROM n'est jamais renommée,
    // seulement le fichier de codes ; relecture lancement, 28/10). Jeu pas encore sur le PC : le nom de Firehouse.
    let nom_firehouse = code["nom_fichier"].as_str().ok_or_else(|| Erreur::Serveur("Firehouse n'a pas donné le nom du fichier.".into()))?;
    let nom_ra = if emu == "retroarch" && nom_firehouse.to_lowercase().ends_with(".cht") {
        noyau
            .fichier_pour_l_emulateur(id, None)
            .await
            .ok()
            .flatten()
            .and_then(|f| f.file_stem().map(|s| format!("{}.cht", s.to_string_lossy())))
    } else {
        None
    };
    let place = crate::emulateurs_profils::place_triche(
        emu,
        dossier_emu,
        &s.profil.nom,
        code["base"].as_str().unwrap_or("donnees"),
        &dossier_rel,
        nom_ra.as_deref().unwrap_or(nom_firehouse),
    )?;
    // Jeu importé : le `fichier` de la liste repris tel quel (contrat 1.19) ; sinon la route de sa fiche.
    let octets = match &criteres {
        OuChercher::Recherche(_) => s.source.fichier_triche_route(code["fichier"].as_str().unwrap_or("")).await?,
        OuChercher::Fiche(f) => s.source.fichier_triche(*f, &cle).await?,
    };
    if let Some(p) = place.parent() {
        std::fs::create_dir_all(p)?;
    }
    // Dolphin : `GameSettings/<ID>.ini` porte aussi les réglages par jeu de la personne → seules les sections de codes
    // sont remplacées (jamais tout le fichier).
    let ancien = std::fs::read(&place).ok();
    let octets = match (emu, &ancien) {
        ("dolphin", Some(a)) => crate::emulateurs_profils::fusionner_sections_ini(&String::from_utf8_lossy(a), &String::from_utf8_lossy(&octets)).into_bytes(),
        _ => octets,
    };
    if ancien.is_some() && ancien.as_deref() != Some(octets.as_slice()) {
        // L'ancien fichier gardé à côté, jamais par-dessus une copie déjà faite.
        let ext = place.extension().and_then(|e| e.to_str()).unwrap_or("").to_string();
        let mut copie = place.with_extension(format!("{ext}.avant-frogtend"));
        let mut n = 0;
        while copie.exists() {
            n += 1;
            copie = place.with_extension(format!("{ext}.avant-frogtend-{}-{n}", secondes_maintenant()));
        }
        std::fs::copy(&place, &copie)?;
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

/// La plateforme de la PS Vita chez Firehouse (nom LaunchBox).
const PLATEFORME_VITA: &str = "Sony Playstation Vita";

/// Vita3K : le programme, vérifié, et son dossier.
fn programme_vita3k(programme: &str) -> Resultat<(std::path::PathBuf, std::path::PathBuf)> {
    let p = std::path::PathBuf::from(programme);
    if !crate::vita3k::est_vita3k(programme) || !p.is_file() {
        return Err(Erreur::Refus("Ce n'est pas le programme de Vita3K.".into()));
    }
    let d = p.parent().map(std::path::PathBuf::from).unwrap_or_default();
    Ok((p, d))
}

/// Les fichiers du firmware de la Vita que Firehouse sert et qui manquent encore à ce Vita3K (vide : rien à faire), pour
/// annoncer leur taille avant tout accord.
#[tauri::command]
pub async fn vita3k_firmware_manquant(noyau: State<'_, Noyau>, programme: String) -> Resultat<Vec<crate::bios::BiosConseil>> {
    let (_, dossier) = programme_vita3k(&programme)?;
    let v = noyau.session().await?.source.emulateurs(PLATEFORME_VITA).await?;
    let mut ignore = crate::bios::Bilan::default();
    let servis = crate::vita3k::firmwares_servis(&crate::bios::lire_bios(&v["conseil"]["bios"], &mut ignore));
    Ok(crate::vita3k::firmwares_manquants(&dossier, &servis).into_iter().map(|(_, f)| f).collect())
}

/// Le firmware de la Vita SERVI PAR FIREHOUSE (besoin 48) : chaque fichier qui manque, dans l'ordre servi, est
/// téléchargé (reprise, progression : événement « emulateur », id « vita3k-firmware »), son SHA-1 vérifié, puis installé
/// par Vita3K (`--firmware`, sans fenêtre). Vita3K fermé ; un seul accord a été donné avant (la taille annoncée). Rend
/// le nombre de fichiers installés.
#[tauri::command]
pub async fn vita3k_firmware_servi(app: AppHandle, noyau: State<'_, Noyau>, programme: String) -> Resultat<usize> {
    let (p, dossier) = programme_vita3k(&programme)?;
    exiger_programme_de_confiance(&app, &noyau, &p)?;
    if !crate::vita3k::portable(&dossier) {
        return Err(Erreur::Refus("Vita3K n'est pas installé en portable : Frogtend n'y installe rien.".into()));
    }
    // AVANT de télécharger 300 Mo : Vita3K fermé, et la place sur le disque.
    if !crate::lancement::processus_du_dossier(&dossier).is_empty() {
        return Err(Erreur::Refus("Vita3K est ouvert : ferme-le (ou quitte ta partie), puis recommence.".into()));
    }
    let _occupe = occuper_installation(dossier.parent().unwrap_or(&dossier), &dossier, "Vita3K")?;
    let s = noyau.session().await?;
    let v = s.source.emulateurs(PLATEFORME_VITA).await?;
    let mut ignore = crate::bios::Bilan::default();
    let servis = crate::vita3k::firmwares_servis(&crate::bios::lire_bios(&v["conseil"]["bios"], &mut ignore));
    let a_faire = crate::vita3k::firmwares_manquants(&dossier, &servis);
    if a_faire.is_empty() {
        return Ok(0);
    }
    // Chaque PUP, puis ce qu'il donne une fois extrait : environ trois fois leur taille, par prudence.
    let total: u64 = a_faire.iter().filter_map(|(_, f)| f.taille).sum();
    if let Some(libre) = crate::jeux_pc::place_libre(&dossier) {
        if libre < total.saturating_mul(3) {
            return Err(Erreur::Disque(format!("Pas assez de place sur le disque de Vita3K : il faut environ {} Mo libres.", total.saturating_mul(3) / 1_048_576)));
        }
    }
    let deja = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let mut faits = 0;
    for (i, (n, f)) in a_faire.iter().enumerate() {
        let (app2, deja2) = (app.clone(), deja.clone());
        let progres = move |recus: u64, _total: Option<u64>| {
            let base = deja2.load(std::sync::atomic::Ordering::Relaxed);
            let _ = app2.emit("emulateur", ProgresEmulateur { id: "vita3k-firmware".into(), recus: base + recus, total: Some(total) });
        };
        let pup = match crate::bios::telecharger_firmware(&s.source, f, &dossier, &progres).await {
            Ok(p) => p,
            Err(e) => {
                noyau.journaliser(&format!("Vita3K : {} non téléchargé ({e:?})", f.fichier));
                return Err(e);
            }
        };
        // L'étape d'installation, dite (elle peut durer quelques minutes) : `recus` = n° du fichier, `total` = combien.
        let _ = app.emit("emulateur", ProgresEmulateur { id: "vita3k-firmware-installation".into(), recus: i as u64 + 1, total: Some(a_faire.len() as u64) });
        let (d, prog, fc, pc, n) = (dossier.clone(), p.clone(), f.clone(), pup.clone(), *n);
        let r = tauri::async_runtime::spawn_blocking(move || crate::vita3k::installer_firmware(&d, &prog, n, &fc, &pc))
            .await
            .map_err(|_| Erreur::Disque("L'installation du firmware s'est arrêtée brutalement.".into()))?;
        let _ = std::fs::remove_file(&pup);
        if let Err(e) = r {
            noyau.journaliser(&format!("Vita3K : {} non installé ({e:?})", f.fichier));
            return Err(e);
        }
        deja.fetch_add(f.taille.unwrap_or(0), std::sync::atomic::Ordering::Relaxed);
        faits += 1;
        noyau.journaliser(&format!("Vita3K : {} installé ({faits}/{})", f.fichier, a_faire.len()));
    }
    let _ = std::fs::remove_dir(dossier.join(".telechargements"));
    Ok(faits)
}

/// Où en est Eden (ou Eglantine) pour le firmware : portable, clés posées, firmware déjà là, et sa version s'il a été
/// posé par Frogtend (contrat 1.38 : une version plus récente servie est proposée).
#[derive(Serialize)]
pub struct EtatFirmwareEden {
    pub portable: bool,
    pub cles: bool,
    pub firmware: bool,
    pub version: Option<String>,
    pub de_frogtend: bool,
}

#[tauri::command]
pub fn eden_firmware(programme: String) -> Resultat<EtatFirmwareEden> {
    let p = std::path::PathBuf::from(&programme);
    // Ryubing, l'alternative Switch (contrat 1.41) : même question, SES dossiers (`portable\system\`, `registered\`).
    if crate::ryubing::est_ryubing(&programme) && p.is_file() {
        let d = p.parent().map(std::path::PathBuf::from).unwrap_or_default();
        let note = crate::ryubing::note_firmware(&d);
        return Ok(EtatFirmwareEden {
            portable: crate::ryubing::portable(&d),
            cles: crate::ryubing::cles_presentes(&d),
            firmware: crate::ryubing::firmware_present(&d),
            version: note.as_ref().and_then(|n| n.version.clone()),
            de_frogtend: note.is_some(),
        });
    }
    if !crate::eden::est_eden(&programme) || !p.is_file() {
        return Err(Erreur::Refus("Ce n'est pas le programme d'Eden.".into()));
    }
    let d = p.parent().map(std::path::PathBuf::from).unwrap_or_default();
    let note = crate::eden::note_firmware(&d);
    Ok(EtatFirmwareEden {
        portable: crate::eden::portable(&d) && crate::eden::dossier_nand(&d).is_some(),
        cles: crate::eden::cles_presentes(&d),
        firmware: crate::eden::firmware_present(&d),
        version: note.as_ref().and_then(|n| n.version.clone()),
        de_frogtend: note.is_some(),
    })
}

/// Le firmware Switch SERVI PAR FIREHOUSE (conseil de la Switch, contrat 1.10 : un zip de .nca) : téléchargé avec reprise
/// et progression (événement « emulateur », id « eden-firmware »), SHA-1 vérifié, puis POSÉ par Frogtend dans le dossier
/// système VIDE d'Eden (jamais par le menu d'Eden, qui vide d'abord ce dossier). Eden fermé, portable, clés posées ; un
/// seul accord a été donné avant (sa taille annoncée). Rend le nombre de fichiers posés.
#[tauri::command]
pub async fn eden_firmware_servi(app: AppHandle, noyau: State<'_, Noyau>, programme: String, remplacer: Option<bool>) -> Resultat<usize> {
    let p = std::path::PathBuf::from(&programme);
    if crate::ryubing::est_ryubing(&programme) {
        return ryubing_firmware_servi(&app, &noyau, programme, remplacer.unwrap_or(false)).await;
    }
    if !crate::eden::est_eden(&programme) || !p.is_file() {
        return Err(Erreur::Refus("Ce n'est pas le programme d'Eden.".into()));
    }
    exiger_programme_de_confiance(&app, &noyau, &p)?;
    let dossier = p.parent().map(std::path::PathBuf::from).unwrap_or_default();
    // Son nom à lui dans les messages (Eglantine, Lichii…), jamais « Eden » pour un autre.
    let nom = crate::emulateurs::fiche(crate::eden::id_de(&programme)).map(|f| f.nom).unwrap_or("Eden");
    // AVANT de télécharger 320 Mo : tout ce qui ferait refuser la pose.
    if !crate::lancement::processus_du_dossier(&dossier).is_empty() {
        return Err(Erreur::Refus(format!("{nom} est ouvert : ferme-le (ou quitte ta partie), puis recommence.")));
    }
    let _occupe = occuper_installation(dossier.parent().unwrap_or(&dossier), &dossier, nom)?;
    if !crate::eden::portable(&dossier) || crate::eden::dossier_nand(&dossier).is_none() {
        return Err(Erreur::Refus(format!("{nom} n'est pas installé en mode portable : Frogtend n'écrit pas dans ses données de Windows.")));
    }
    if !crate::eden::cles_presentes(&dossier) {
        return Err(Erreur::Refus("Tes clés (prod.keys) ne sont pas encore dans l'émulateur : sans elles, il refuse le firmware. Pose-les d'abord.".into()));
    }
    let remplacer = remplacer.unwrap_or(false);
    if crate::eden::firmware_present(&dossier) && !(remplacer && crate::eden::note_firmware(&dossier).is_some()) {
        return Err(Erreur::Refus("Un firmware est déjà installé dans Eden : il est laissé tel quel.".into()));
    }
    let s = noyau.session().await?;
    // Le firmware de SA console : Switch pour Eden, Switch 2 pour Eglantine (contrat 1.38).
    let v = s.source.emulateurs(crate::eden::plateforme(&programme)).await?;
    let mut ignore = crate::bios::Bilan::default();
    let bios = crate::bios::lire_bios(&v["conseil"]["bios"], &mut ignore);
    let f = crate::bios::firmware_servi(&bios)
        .filter(|f| f.fichier.to_lowercase().ends_with(".zip"))
        .cloned()
        .ok_or_else(|| Erreur::Introuvable("Firehouse ne fournit pas (encore) le firmware Switch.".into()))?;
    // Le zip, puis ses fichiers extraits à côté : environ trois fois sa taille, par prudence.
    if let (Some(t), Some(libre)) = (f.taille, crate::jeux_pc::place_libre(&dossier)) {
        if libre < t.saturating_mul(3) {
            return Err(Erreur::Disque(format!("Pas assez de place sur le disque de {nom} : il faut environ {} Mo libres.", t.saturating_mul(3) / 1_048_576)));
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
        return Err(Erreur::Refus(format!("{nom} a été ouvert pendant le téléchargement : ferme-le, puis recommence.")));
    }
    let (d, z) = (dossier.clone(), zip.clone());
    let version = f.version.clone();
    let r = tauri::async_runtime::spawn_blocking(move || {
        // Une mise à jour : l'ancien firmware de Frogtend part à l'abri, puis le nouveau est posé (jamais par-dessus) ;
        // si la pose échoue, l'ancien revient tel quel (jamais un émulateur laissé sans firmware).
        let abri = if remplacer && crate::eden::firmware_present(&d) { Some(crate::eden::mettre_firmware_a_l_abri(&d)?) } else { None };
        let n = match crate::eden::poser_firmware(&d, &z) {
            Ok(n) => n,
            Err(e) => {
                if let Some(a) = &abri {
                    crate::eden::remettre_firmware(&d, a);
                }
                return Err(e);
            }
        };
        crate::eden::noter_firmware(&d, version.as_deref())?;
        Ok::<usize, Erreur>(n)
    })
        .await
        .map_err(|_| Erreur::Disque("La pose du firmware s'est arrêtée brutalement.".into()))?;
    let _ = std::fs::remove_file(&zip);
    let _ = std::fs::remove_dir(dossier.join(".telechargements"));
    if let Ok(n) = &r {
        noyau.journaliser(&format!("firmware posé dans {} : {n} fichier(s), version {}", programme, f.version.as_deref().unwrap_or("inconnue")));
    }
    r
}

/// La plateforme de Ryubing chez Firehouse (nom LaunchBox) : il est une alternative du conseil de la Switch.
const PLATEFORME_SWITCH: &str = "Nintendo Switch";

/// Le firmware Switch SERVI PAR FIREHOUSE pour l'alternative Ryubing (✅ Seb 15/11 : posé par Frogtend, sans clic ;
/// contrat 1.41 : `alternatives[].bios` de la Switch, jamais ceux du conseil faits pour Eden). Sur le modèle d'Eden :
/// Ryubing FERMÉ, portable, clés posées d'abord ; place vérifiée ; téléchargé avec reprise (événement « emulateur »,
/// id « eden-firmware », le même suivi que pour Eden), SHA-1 vérifié, puis posé par Frogtend dans un `registered` vide
/// (`ryubing::poser_firmware`, relu). Une mise à jour met l'ancien (de Frogtend) à l'abri et le remet si elle échoue.
/// Un seul accord a été donné avant (sa taille annoncée). Rend le nombre de NCA posés.
async fn ryubing_firmware_servi(app: &AppHandle, noyau: &Noyau, programme: String, remplacer: bool) -> Resultat<usize> {
    let p = std::path::PathBuf::from(&programme);
    if !p.is_file() {
        return Err(Erreur::Refus("Ce n'est pas le programme de Ryubing.".into()));
    }
    exiger_programme_de_confiance(app, noyau, &p)?;
    // Seulement un Ryubing installé par Frogtend (relecture lancement 15/11) : un Ryujinx réglé à la main a peut-être
    // son firmware ailleurs, Frogtend n'y touche pas.
    let de_frogtend = registre_emulateurs(noyau)
        .tous()
        .into_iter()
        .any(|e| e.par_frogtend && e.programme.eq_ignore_ascii_case(&programme));
    if !de_frogtend {
        return Err(Erreur::Refus("Ce Ryubing n'a pas été installé par Frogtend : Frogtend n'y pose pas de firmware. Installe-le avec le logiciel système depuis Ryubing lui-même, ou réinstalle Ryubing par Frogtend.".into()));
    }
    let dossier = p.parent().map(std::path::PathBuf::from).unwrap_or_default();
    // AVANT de télécharger 320 Mo : tout ce qui ferait refuser la pose.
    if !crate::lancement::processus_du_dossier(&dossier).is_empty() {
        return Err(Erreur::Refus("Ryubing est ouvert : ferme-le (ou quitte ta partie), puis recommence.".into()));
    }
    let _occupe = occuper_installation(dossier.parent().unwrap_or(&dossier), &dossier, "Ryubing")?;
    if !crate::ryubing::portable(&dossier) {
        return Err(Erreur::Refus("Ryubing n'est pas installé en mode portable : Frogtend n'écrit pas dans ses données de Windows.".into()));
    }
    if !crate::ryubing::cles_presentes(&dossier) {
        return Err(Erreur::Refus("Tes clés (prod.keys) ne sont pas encore dans Ryubing : sans elles, il ne lit pas le firmware. Pose-les d'abord.".into()));
    }
    if crate::ryubing::firmware_present(&dossier) && !(remplacer && crate::ryubing::note_firmware(&dossier).is_some()) {
        return Err(Erreur::Refus("Un firmware est déjà installé dans Ryubing : il est laissé tel quel.".into()));
    }
    let s = noyau.session().await?;
    let v = s.source.emulateurs(PLATEFORME_SWITCH).await?;
    let mut ignore = crate::bios::Bilan::default();
    let bios = bios_pour_le_programme(&v["conseil"], PLATEFORME_SWITCH, &programme)
        .filter(|(id, ..)| id == "ryubing")
        .map(|(_, b, _)| crate::bios::lire_bios(b, &mut ignore))
        .unwrap_or_default();
    let f = crate::bios::firmware_servi(&bios)
        .filter(|f| f.fichier.to_lowercase().ends_with(".zip"))
        .cloned()
        .ok_or_else(|| Erreur::Introuvable("Firehouse ne fournit pas (encore) le firmware Switch pour Ryubing.".into()))?;
    // Le zip, puis ses fichiers extraits à côté : environ trois fois sa taille, par prudence.
    if let (Some(t), Some(libre)) = (f.taille, crate::jeux_pc::place_libre(&dossier)) {
        if libre < t.saturating_mul(3) {
            return Err(Erreur::Disque(format!("Pas assez de place sur le disque de Ryubing : il faut environ {} Mo libres.", t.saturating_mul(3) / 1_048_576)));
        }
    }
    let app2 = app.clone();
    let progres = move |recus, total| {
        let _ = app2.emit("emulateur", ProgresEmulateur { id: "eden-firmware".into(), recus, total });
    };
    let zip = crate::bios::telecharger_firmware(&s.source, &f, &dossier, &progres).await?;
    // Ryubing a pu être ouvert pendant le téléchargement : revérifié juste avant de poser.
    if !crate::lancement::processus_du_dossier(&dossier).is_empty() {
        let _ = std::fs::remove_file(&zip);
        return Err(Erreur::Refus("Ryubing a été ouvert pendant le téléchargement : ferme-le, puis recommence.".into()));
    }
    let (d, z) = (dossier.clone(), zip.clone());
    let version = f.version.clone();
    let r = tauri::async_runtime::spawn_blocking(move || {
        // Revérifié au tout dernier moment, sur le même fil que la pose.
        if !crate::lancement::processus_du_dossier(&d).is_empty() {
            return Err(Erreur::Refus("Ryubing a été ouvert : ferme-le, puis recommence.".into()));
        }
        let abri = if remplacer && crate::ryubing::firmware_present(&d) { Some(crate::ryubing::mettre_firmware_a_l_abri(&d)?) } else { None };
        let n = match crate::ryubing::poser_firmware(&d, &z) {
            Ok(n) => n,
            Err(e) => {
                if let Some(a) = &abri {
                    crate::ryubing::remettre_firmware(&d, a);
                }
                return Err(e);
            }
        };
        crate::ryubing::noter_firmware(&d, version.as_deref())?;
        Ok::<usize, Erreur>(n)
    })
    .await
    .map_err(|_| Erreur::Disque("La pose du firmware s'est arrêtée brutalement.".into()))?;
    let _ = std::fs::remove_file(&zip);
    let _ = std::fs::remove_dir(dossier.join(".telechargements"));
    if let Ok(n) = &r {
        noyau.journaliser(&format!("firmware posé dans {programme} : {n} NCA, version {}", f.version.as_deref().unwrap_or("inconnue")));
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

/// Ce qu'un déplacement d'émulateur a changé (l'interface réécrit les programmes de `pc.json`).
#[derive(Serialize)]
pub struct EmulateurDeplace {
    ancien: String,
    nouveau: String,
    renomme: bool,
}

/// Déplace un émulateur ou un outil INSTALLÉ PAR FROGTEND dans le dossier des émulateurs (ou des outils) réglé
/// (✅ Seb 04/11 ; avis lancement) : renommé sur le même disque, sinon copie comparée puis l'original retiré ; puis
/// son chemin suit partout où Frogtend l'a noté (registre, notes, xemu.toml, Eden). Refusé s'il est ouvert, pendant
/// une partie, un retrait ou une installation. Un émulateur installé à la main n'est jamais déplacé.
#[tauri::command]
pub async fn emulateur_deplacer(app: AppHandle, noyau: State<'_, Noyau>, id: String, base: String) -> Resultat<EmulateurDeplace> {
    let reg = registre_emulateurs(&noyau);
    let e = reg.un(&id).ok_or_else(|| Erreur::Introuvable("Émulateur inconnu de Frogtend.".into()))?;
    if !e.par_frogtend {
        return Err(Erreur::Refus(format!("{} a été installé à la main : Frogtend ne le déplace pas.", e.nom)));
    }
    let programme = std::path::PathBuf::from(&e.programme);
    let source = racine_installation(&e.nom, &programme).unwrap_or_else(|| std::path::PathBuf::from(&e.dossier));
    if !source.is_dir() || !crate::garde::sous(&programme, &source) {
        return Err(Erreur::Refus(format!("{} n'est plus à sa place ({}) : rien n'est déplacé.", e.nom, source.display())));
    }
    let base = std::path::PathBuf::from(base.trim());
    exiger_dossier_de_base(&app, &base)?;
    let cible = base.join(source.file_name().ok_or_else(|| Erreur::Refus("Dossier de l'émulateur inattendu.".into()))?);
    if app.state::<crate::menu_jeu::MenuJeu>().partie().is_some() || !noyau.occupes.lock().unwrap_or_else(|e| e.into_inner()).is_empty() {
        return Err(Erreur::Refus("Un jeu est en cours (partie, retrait ou installation) : attends qu'il finisse, puis recommence.".into()));
    }
    // Ouvert (même hors de Frogtend) : jamais déplacé.
    if !crate::lancement::processus_du_dossier(&source).is_empty() {
        return Err(Erreur::Refus(format!("{} est ouvert : ferme-le, puis recommence.", e.nom)));
    }
    let _ici = occuper_installation(&base, &cible, &e.nom)?;
    let _la = match source.parent() {
        Some(p) if !crate::garde::meme(p, &base) => Some(occuper_installation(p, &source, &e.nom)?),
        _ => None,
    };
    noyau.journaliser(&format!("déplacement de {} : {} → {}", e.nom, source.display(), cible.display()));
    let (s, c) = (source.clone(), cible.clone());
    let a_effacer = tauri::async_runtime::spawn_blocking(move || crate::demenagement::deplacer_dossier(&s, &c))
        .await
        .map_err(|_| Erreur::Disque("Le déplacement s'est arrêté brutalement.".into()))??;
    let renomme = a_effacer.is_none();
    // Le registre d'abord : c'est lui qui rend l'émulateur « de confiance » à son nouvel emplacement. Puis pc.json
    // (les émulateurs réglés par console), écrit ICI pour qu'une fermeture de Frogtend ne le laisse jamais en arrière.
    let nouveau_programme = crate::demenagement::remplacer_debut(&e.programme, &source, &cible).unwrap_or_else(|| e.programme.clone());
    if let Err(err) = reg.retenir(crate::emulateurs::EmulateurInstalle { dossier: cible.to_string_lossy().into(), programme: nouveau_programme, ..e.clone() }) {
        noyau.journaliser(&format!("déplacement de {} : registre non mis à jour ({err:?})", e.nom));
        // Après une copie : tout est remis comme avant (l'original reprend son nom, la copie part).
        if let Some(vieux) = &a_effacer {
            if std::fs::rename(vieux, &source).is_ok() {
                let _ = std::fs::remove_dir_all(&cible);
                return Err(Erreur::Disque(format!("Frogtend n'a pas pu noter la nouvelle place de {} : rien n'a changé.", e.nom)));
            }
        }
        return Err(Erreur::Disque(format!("{} a été déplacé dans {}, mais Frogtend n'a pas pu noter sa nouvelle place : choisis-le de nouveau dans ⚙ Réglages ▸ Émulateurs.", e.nom, cible.display())));
    }
    if let Err(err) = suivre_dans_pc_json(&app, &source, &cible) {
        noyau.journaliser(&format!("déplacement de {} : pc.json non mis à jour ({err:?})", e.nom));
    }
    // L'original (après une copie d'un disque à l'autre), seulement maintenant : la nouvelle place est notée.
    if let Some(vieux) = a_effacer {
        if let Err(err) = std::fs::remove_dir_all(&vieux) {
            noyau.journaliser(&format!("déplacement de {} : l'ancienne copie {} n'a pas pu être effacée ({err})", e.nom, vieux.display()));
        }
    }
    for note in ["langues-posees.json", "rpcs3-compile.json", "eden-contenus.json"] {
        if let Err(err) = crate::demenagement::renommer_cles_json(&noyau.dossier.join(note), &source, &cible) {
            noyau.journaliser(&format!("déplacement de {} : note {note} non mise à jour ({err:?})", e.nom));
        }
    }
    // Les réglages où l'émulateur a écrit son propre chemin complet (avis lancement : xemu, Eden), après une copie à l'abri.
    let reglages: Vec<std::path::PathBuf> = match id.as_str() {
        "xemu" => std::iter::once(cible.join("xemu.toml"))
            .chain(std::fs::read_dir(cible.join("Profils")).into_iter().flatten().flatten().map(|p| p.path().join("xemu.toml")))
            .collect(),
        "eden" | "eglantine" | "lichii" => vec![cible.join("user").join("config").join("qt-config.ini")],
        "azahar" => vec![cible.join("user").join("config").join("qt-config.ini")],
        // Ceux-là écrivent d'ordinaire des chemins relatifs ; sur un autre PC, un chemin complet vers leur propre dossier
        // est possible (seules ces valeurs changent).
        "retroarch" => vec![cible.join("retroarch.cfg")],
        "duckstation" => vec![cible.join("settings.ini")],
        "pcsx2" => vec![cible.join("inis").join("PCSX2.ini")],
        "mgba" => vec![cible.join("config.ini")],
        _ => vec![],
    };
    for f in reglages.into_iter().filter(|f| f.is_file()) {
        let Ok(texte) = std::fs::read_to_string(&f) else { continue };
        if let Some(nouveau) = crate::demenagement::reecrire_chemins(&texte, &source, &cible) {
            let r = crate::emulateurs_profils::mettre_a_l_abri(&cible, &f).and_then(|_| std::fs::write(&f, nouveau).map_err(Erreur::from));
            if let Err(err) = r {
                noyau.journaliser(&format!("déplacement de {} : {} non mis à jour ({err:?})", e.nom, f.display()));
            }
        }
    }
    noyau.journaliser(&format!("déplacement de {} terminé ({})", e.nom, if renomme { "renommé" } else { "copié puis retiré" }));
    Ok(EmulateurDeplace { ancien: source.to_string_lossy().into(), nouveau: cible.to_string_lossy().into(), renomme })
}

/// Les émulateurs réglés par console (`pc.json` ▸ `emulateurs[*].liste[].programme`) qui désignaient l'ancien dossier
/// désignent le nouveau.
fn suivre_dans_pc_json(app: &AppHandle, ancien: &std::path::Path, nouveau: &std::path::Path) -> Resultat<()> {
    let store = app.store("pc.json").map_err(|e| Erreur::Disque(e.to_string()))?;
    let Some(mut r) = store.get("reglages") else { return Ok(()) };
    let mut change = false;
    if let Some(plateformes) = r["emulateurs"].as_object_mut() {
        for p in plateformes.values_mut() {
            for x in p["liste"].as_array_mut().into_iter().flatten() {
                if let Some(n) = x["programme"].as_str().and_then(|v| crate::demenagement::remplacer_debut(v, ancien, nouveau)) {
                    x["programme"] = Value::String(n);
                    change = true;
                }
            }
        }
    }
    if change {
        store.set("reglages", r);
        store.save().map_err(|e| Erreur::Disque(e.to_string()))?;
    }
    Ok(())
}

/// Retire un ancien dossier de base des émulateurs ou des outils, seulement s'il est VIDE (rien d'autre ne peut partir)
/// et n'est (ni ne contient) aucun dossier encore réglé (abris, captures, emplacements…).
#[tauri::command]
pub fn dossier_retirer_si_vide(app: AppHandle, chemin: String) -> bool {
    let d = std::path::PathBuf::from(chemin.trim());
    let reglages = app.store("pc.json").ok().and_then(|s| s.get("reglages")).unwrap_or(Value::Null);
    refus_dossier(&d, "emulateurs").is_none() && !crate::garde::Reglages::lire(&reglages).est_un_dossier_regle(&d) && std::fs::remove_dir(&d).is_ok()
}

/// Le dossier d'un émulateur ou d'un outil que FROGTEND a déjà installé (son programme est toujours là) : c'est là
/// qu'il se met à jour. Un émulateur installé à la main n'est jamais remplacé : `None`.
fn deja_installe(noyau: &Noyau, id: &str) -> Option<std::path::PathBuf> {
    let e = registre_emulateurs(noyau).un(id)?;
    if !e.par_frogtend || !std::path::Path::new(&e.programme).is_file() {
        return None;
    }
    racine_installation(&e.nom, std::path::Path::new(&e.programme))
}

/// Le dossier d'installation d'un programme : le dossier, au-dessus de lui, qui porte le nom donné à l'installation
/// (`nom_de_dossier(nom)`) — un programme choisi dans un sous-dossier (« outils\b.exe ») ne fait jamais installer la
/// mise à jour dans ce sous-dossier. Introuvable : `None` (la mise à jour va alors au dossier réglé, comme avant).
pub(crate) fn racine_installation(nom: &str, programme: &std::path::Path) -> Option<std::path::PathBuf> {
    let voulu = crate::jeux_pc::nom_de_dossier(nom).to_lowercase();
    programme.ancestors().skip(1).find(|d| d.file_name().is_some_and(|n| n.to_string_lossy().to_lowercase() == voulu)).map(std::path::Path::to_path_buf)
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
    // Ouvert seul (✅ Seb 17/11) : il réécrirait ses réglages en quittant, par-dessus ceux de Frogtend.
    if noyau.emulateur_seul_dans(programme) {
        return Err(Erreur::Refus("Cet émulateur est ouvert : ferme-le d'abord, puis recommence.".into()));
    }
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
    // Résolu par Firehouse (Eglantine) : sa version et sa taille sont celles du paquet qu'il servira.
    if matches!(f.origine, crate::emulateurs::Origine::Firehouse) {
        let p = noyau.session().await?.source.paquet_emulateur(&id).await?;
        return Ok(crate::emulateurs::Paquet {
            version: p["version"].as_str().unwrap_or("").to_string(),
            url: p["url"].as_str().unwrap_or("").to_string(),
            taille: p["taille"].as_u64(),
        });
    }
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
    let (base, cible) = match deja_installe(&noyau, &id) {
        // Une MISE À JOUR se fait à sa place, même si le dossier des émulateurs a changé depuis (jamais une copie neuve
        // ailleurs, sans sa configuration ni ses parties : relecture lancement 04/11).
        Some(c) => (c.parent().map(std::path::Path::to_path_buf).unwrap_or_else(|| c.clone()), c),
        None => {
            let base = std::path::PathBuf::from(&dossier);
            // Le dossier réglé d'abord ; s'il n'existe pas encore (le dossier proposé par « Prêt à jouer »), il est créé.
            exiger_dossier_de_base(&app, &base)?;
            let cible = base.join(f.nom);
            (base, cible)
        }
    };
    // Une seule installation à la fois dans ce dossier, jamais par-dessus l'émulateur ouvert.
    let _occupe = occuper_installation(&base, &cible, f.nom)?;
    if !base.is_dir() {
        std::fs::create_dir_all(&base).map_err(|e| Erreur::Disque(format!("Dossier impossible à créer : {} ({e}).", base.display())))?;
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

/// Les dossiers des émulateurs en cours d'installation ou de mise à jour (en minuscules) : aucun jeu n'est lancé depuis
/// l'un d'eux pendant ce temps (relecture interface 08/11 : la mise à jour proposée au démarrage tourne pendant qu'on
/// clique ▶ Jouer).
static CIBLES_EN_INSTALLATION: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

fn cle_de_dossier(d: &std::path::Path) -> String {
    d.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_lowercase()
}

/// Ce programme est-il dans un dossier en cours d'installation ?
fn en_installation(programme: &std::path::Path) -> bool {
    let p = cle_de_dossier(programme);
    CIBLES_EN_INSTALLATION.lock().unwrap_or_else(|e| e.into_inner()).iter().any(|c| p == *c || p.starts_with(&format!("{c}\\")))
}

/// Libère le dossier à la fin de l'installation, même après une erreur.
pub(crate) struct Occupation(String, String);

impl Drop for Occupation {
    fn drop(&mut self) {
        INSTALLATIONS.lock().unwrap_or_else(|e| e.into_inner()).retain(|x| x != &self.0);
        CIBLES_EN_INSTALLATION.lock().unwrap_or_else(|e| e.into_inner()).retain(|x| x != &self.1);
    }
}

/// Prend le dossier pour une installation : refusé (clairement, sans attendre) si une autre y est en cours, ou si le
/// programme à remplacer est ouvert.
pub(crate) fn occuper_installation(base: &std::path::Path, cible: &std::path::Path, nom: &str) -> Resultat<Occupation> {
    if !crate::lancement::processus_du_dossier(cible).is_empty() {
        return Err(Erreur::Refus(format!("{nom} est ouvert : ferme-le, puis recommence.")));
    }
    let cle = cle_de_dossier(base);
    let mut l = INSTALLATIONS.lock().unwrap_or_else(|e| e.into_inner());
    if l.contains(&cle) {
        return Err(Erreur::Refus(format!("Une installation est déjà en cours dans ce dossier ({nom} ou un autre) : attends qu'elle se termine.")));
    }
    l.push(cle.clone());
    let c = cle_de_dossier(cible);
    CIBLES_EN_INSTALLATION.lock().unwrap_or_else(|e| e.into_inner()).push(c.clone());
    Ok(Occupation(cle, c))
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
    let Some(id_conseil) = conseil["emulateur"].as_str() else { return Ok(Default::default()) };
    let dossier = std::path::Path::new(&programme).parent().map(std::path::PathBuf::from).unwrap_or_default();
    // Ceux du conseil, ou ceux d'une ALTERNATIVE qui décrit les siens (contrat 1.41 : Ryubing), jamais les uns pour
    // l'autre.
    let Some((id, bios_decrits, cles_facultatives)) = bios_pour_le_programme(conseil, &plateforme, &programme) else {
        noyau.journaliser(&format!("fichiers système de {plateforme} : rien posé (le programme n'est pas celui du conseil « {id_conseil} » ni d'une alternative qui décrit les siens)"));
        return Ok(Default::default());
    };
    if !dossier.is_dir() {
        noyau.journaliser(&format!("fichiers système de {plateforme} : rien posé (dossier de l'émulateur introuvable)"));
        return Ok(Default::default());
    }
    let mut illisibles = crate::bios::Bilan::default();
    let bios = crate::bios::lire_bios(bios_decrits, &mut illisibles);
    let mut bilan = crate::bios::poser_choisis(&s.source, &bios, &dossier, id == "retroarch", cles_facultatives).await;
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

/// Les fichiers système du conseil vont-ils à ce programme ? À l'émulateur du conseil ; et, dans la famille d'Eden, à
/// une ALTERNATIVE que Firehouse cite pour la même console (Lichii pour la Switch 2, contrat 1.40) : même dossier
/// `user\keys\`, mêmes clés et même firmware de SA console. Jamais à un autre émulateur (chemins inconnus).
pub fn programme_pour_le_conseil(conseil: &serde_json::Value, plateforme: &str, programme: &str) -> bool {
    let Some(id) = conseil["emulateur"].as_str() else { return false };
    if programme_du_conseil(id, programme) {
        return true;
    }
    crate::emulateurs::famille_eden(id)
        && crate::eden::est_eden(programme)
        && crate::eden::plateforme(programme) == plateforme
        && conseil["alternatives"].as_array().is_some_and(|l| {
            l.iter().filter_map(|a| a["emulateur"].as_str()).any(|a| crate::emulateurs::famille_eden(a) && programme_du_conseil(a, programme))
        })
}

/// Les fichiers système qui vont à ce programme, d'après le conseil relu : (id de l'émulateur, leur description,
/// clés facultatives à poser aussi). Ceux du CONSEIL pour son émulateur et la famille d'Eden (`programme_pour_le_conseil`) ;
/// sinon ceux d'une ALTERNATIVE qui décrit les siens (contrat 1.41 : `alternatives[].bios` ; Ryubing, avec son
/// `title.keys` facultatif). `bios` null ou absent = non décrits : rien (jamais ceux du conseil, faits pour Eden).
pub fn bios_pour_le_programme<'a>(conseil: &'a serde_json::Value, plateforme: &str, programme: &str) -> Option<(String, &'a serde_json::Value, bool)> {
    if programme_pour_le_conseil(conseil, plateforme, programme) {
        return Some((conseil["emulateur"].as_str()?.to_string(), &conseil["bios"], false));
    }
    let a = conseil["alternatives"].as_array()?.iter().find(|a| {
        a["emulateur"].as_str().is_some_and(|e| !crate::emulateurs::famille_eden(e) && programme_du_conseil(e, programme))
    })?;
    a["bios"].is_array().then(|| (a["emulateur"].as_str().unwrap_or_default().to_string(), &a["bios"], true))
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
            // Ryubing : rien n'a pu être réglé, dit au journal (sinon il s'ouvrirait en anglais, au clavier, sans indice).
            if id == "ryubing" && crate::ryubing::config_a_cote_du_programme(&d) {
                v.push("un Config.json à côté de Ryujinx.exe passe devant celui de la personne : Ryubing n'est pas réglé (langue, manette, pause)");
            }
            if id == "ryubing" && !crate::ryubing::portable(&d) {
                v.push("Ryubing n'est pas en mode portable : ni dossier par personne, ni réglages (ses données restent dans %APPDATA%)");
            }
            // Une manette pas encore relevée (Sunshine, Bluetooth…) : le clavier est gardé (un identifiant faux couperait
            // tout). Et les réglages propres à un jeu de la personne passent devant les siens : Frogtend n'y écrit rien.
            let mut notes_ryubing: Vec<String> = Vec::new();
            if id == "ryubing" && crate::ryubing::portable(&d) {
                if matches!(manette, crate::emulateurs_profils::Manette::Auto(_) | crate::emulateurs_profils::Manette::Imposee(_)) {
                    if let Some(m) = crate::manettes::id_manette_xinput_ryubing().filter(|m| m != crate::ryubing::MANETTE_RELEVEE) {
                        notes_ryubing.push(format!("manette {m} pas encore relevée : le joueur 1 garde le clavier"));
                    }
                }
                let par_jeu = crate::ryubing::configs_par_jeu(&crate::ryubing::racine(&d, &n));
                if !par_jeu.is_empty() {
                    notes_ryubing.push(format!(
                        "réglages propres à un jeu ({}) : ils remplacent ceux de la personne pour ce jeu, Frogtend n'y écrit rien (pause lue dedans)",
                        par_jeu.join(", ")
                    ));
                }
            }
            v.extend(notes_ryubing.iter().map(String::as_str));
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
        noyau.journaliser(&format!("{} : {a}", f.nom));
    }
    let mut complete: Vec<String> = avant.iter().map(|a| format!("\"{a}\"")).collect();
    // BigPEmu : jamais sans `-localdata`, l'image toujours en premier (bigpemu.rs).
    let ligne = if f.id == "bigpemu" { crate::bigpemu::ligne_complete(ligne) } else { ligne.to_string() };
    if !ligne.trim().is_empty() {
        complete.push(ligne);
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
    let mut l = regrouper_traces(crate::emulateurs::CATALOGUE.iter().map(|f| {
        let d = crate::emulateurs_profils::traces_hors_du_dossier(f.id);
        (f.id, f.nom, d.iter().map(|p| p.to_string_lossy().to_string()).collect())
    }));
    nuancer_traces(&mut l);
    l
}

/// `%USERPROFILE%\.switch` (les clés d'un Ryubing non portable) sert aussi à d'autres outils Switch : seulement
/// « peut-être » de Ryubing.
fn nuancer_traces(l: &mut [Traces]) {
    for t in l.iter_mut().filter(|t| t.id.starts_with("ryubing:") && t.dossiers.iter().any(|d| d.to_lowercase().ends_with("\\.switch") || d.to_lowercase().ends_with("/.switch"))) {
        t.nom = format!("{} (peut-être : d'autres outils Switch se servent aussi de ce dossier)", t.nom);
    }
}

/// Un dossier partagé par plusieurs émulateurs (Eden, Eglantine et Lichii écrivent tous dans `%APPDATA%\eden` hors
/// portable) : UNE ligne, avec leurs noms (essai de Seb 08/11 : le même dossier listé trois fois).
fn regrouper_traces<'a>(l: impl Iterator<Item = (&'a str, &'a str, Vec<String>)>) -> Vec<Traces> {
    let mut sortie: Vec<Traces> = Vec::new();
    for (id, nom, dossiers) in l {
        for d in dossiers {
            match sortie.iter_mut().find(|t| t.dossiers == [d.clone()]) {
                Some(t) => t.nom = format!("{} / {nom}", t.nom),
                None => sortie.push(Traces { id: format!("{id}:{d}"), nom: nom.into(), dossiers: vec![d] }),
            }
        }
    }
    sortie
}

#[cfg(test)]
mod tests_en_installation {
    #[test]
    fn le_dossier_switch_n_est_que_peut_etre_de_ryubing() {
        let mut l = super::regrouper_traces(
            [("ryubing", "Ryubing", vec![r"C:\Users\x\AppData\Roaming\Ryujinx".to_string(), r"C:\Users\x\.switch".to_string()])].into_iter(),
        );
        super::nuancer_traces(&mut l);
        assert_eq!(l[0].nom, "Ryubing");
        assert!(l[1].nom.contains("peut-être"));
    }

    #[test]
    fn aucun_jeu_ne_se_lance_depuis_un_emulateur_en_cours_de_mise_a_jour() {
        let d = tempfile::tempdir().unwrap();
        let cible = d.path().join("Eglantine");
        let programme = cible.join("eglantine.exe");
        assert!(!super::en_installation(&programme));
        let o = super::occuper_installation(d.path(), &cible, "Eglantine").unwrap();
        assert!(super::en_installation(&programme));
        assert!(!super::en_installation(&d.path().join("Eglantine 2").join("eglantine.exe")), "seulement CE dossier");
        drop(o);
        assert!(!super::en_installation(&programme), "libéré à la fin, même après une erreur");
    }
}

#[cfg(test)]
mod tests_traces {
    #[test]
    fn un_dossier_partage_n_est_liste_qu_une_fois_avec_ses_emulateurs() {
        let eden = r"C:\Users\x\AppData\Roaming\eden".to_string();
        let cemu = r"C:\Users\x\AppData\Roaming\Cemu".to_string();
        let t = super::regrouper_traces(
            [("cemu", "Cemu", vec![cemu.clone()]), ("eden", "Eden", vec![eden.clone()]), ("eglantine", "Eglantine", vec![eden.clone()]), ("lichii", "Lichii", vec![eden.clone()]), ("pcsx2", "PCSX2", vec![])].into_iter(),
        );
        assert_eq!(t.len(), 2);
        assert_eq!((t[0].nom.as_str(), t[0].dossiers.clone()), ("Cemu", vec![cemu]));
        assert_eq!((t[1].nom.as_str(), t[1].dossiers.clone()), ("Eden / Eglantine / Lichii", vec![eden]));
        assert_ne!(t[0].id, t[1].id, "clés distinctes pour l'affichage");
    }
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

/// Les captures restaurées reposées, et celles qui attendent encore (pas de dossier des captures, jeu caché).
#[derive(Debug, Serialize)]
pub struct CapturesReposees {
    pub reposees: usize,
    pub en_attente: usize,
}

/// Repose les captures d'écran restaurées dans le dossier des captures réglé (✅ Seb 12/11) : après une restauration,
/// et quand la personne choisit ce dossier.
#[tauri::command]
pub async fn restauration_reposer_captures(app: AppHandle, noyau: State<'_, Noyau>) -> Resultat<CapturesReposees> {
    let d = dossier_captures_regle(&app);
    let (reposees, en_attente) = noyau.reposer_captures(d.as_deref()).await?;
    Ok(CapturesReposees { reposees, en_attente })
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
    use crate::boutiques::{ajouter_compte_gog, appliquer_import_gog_profil, ecrire_source, JeuBoutique, LectureCompteGog};

    fn jg(id: &str) -> JeuBoutique {
        JeuBoutique { id: id.into(), nom: id.into(), minutes: 0, derniere: 0, installe: false, plateforme: "gog".into(), image: None, dossier: None, pret: None, succes: None, releases: vec![], titre_douteux: false }
    }

    /// Un profil de test avec deux comptes GOG : le compte Profil (« 4200 », jeux 1 et 2) et un compte ajouté (« 7700 »,
    /// « Seb », jeux 2 et 9).
    fn deux_comptes_gog(d: &std::path::Path) -> crate::boutiques::CompteBoutique {
        let lu = |site: &str, galaxy: &str, nom: Option<&str>, jeux: &[&str]| LectureCompteGog {
            id_site: site.into(),
            id_galaxy: galaxy.into(),
            nom: nom.map(String::from),
            jeux: jeux.iter().map(|i| jg(i)).collect(),
            ids_lus: None,
            ancien_id_galaxy: None,
        };
        let (c, _) = appliquer_import_gog_profil(None, "", &lu("42", "4200", None, &["gog_1", "gog_2"]), "2026-11-22T10:00:00").unwrap();
        let c = ajouter_compte_gog(Some(c), &lu("", "7700", Some("Seb"), &["gog_2", "gog_9"]), false, "2026-11-22T11:00:00").unwrap();
        ecrire_source(d, "gog", Some(&c)).unwrap();
        c
    }

    /// Lot C4b : l'état des comptes (le compte Profil d'abord, total réuni, compte ajouté sans jeton « à reconnecter »).
    #[test]
    fn l_etat_gog_liste_les_comptes_sans_numero_affiche() {
        let d = tempfile::tempdir().unwrap();
        let c = deux_comptes_gog(d.path());
        let e = super::etat_gog(Some(&c), |id| id == "7700");
        assert!(e.connu);
        assert_eq!(e.nb_jeux, 3, "le jeu commun ne compte qu'une fois");
        assert_eq!(e.maj_le.as_deref(), Some("2026-11-22T11:00:00"));
        assert_eq!(e.comptes.iter().map(|x| (x.cle.as_str(), x.nom.as_str(), x.profil, x.nb_jeux, x.reconnecter)).collect::<Vec<_>>(), [("4200", "", true, 2), ("7700", "Seb", false, 2)].map(|(a, b, c, d)| (a, b, c, d, false)));
        assert!(super::etat_gog(Some(&c), |_| false).comptes[1].reconnecter, "jeton absent : à reconnecter");
        assert!(!super::etat_gog(Some(&c), |_| false).comptes[0].reconnecter, "le compte Profil se reconnecte par sa session");
        assert!(!e.galaxy_autre_compte);
        let mut g = c.clone();
        g.compte_galaxy_pc = "123".into();
        assert!(super::etat_gog(Some(&g), |_| true).galaxy_autre_compte, "Galaxy sur un compte inconnu de Frogtend");
        g.compte_galaxy_pc = "7700".into();
        assert!(!super::etat_gog(Some(&g), |_| true).galaxy_autre_compte);
        let vide = super::etat_gog(None, |_| true);
        assert!(!vide.connu && vide.comptes.is_empty() && vide.nb_jeux == 0);
    }

    /// Lot C4b : les cartes GOG réunissent les comptes ; un jeu installé reste même s'il n'est à aucun compte.
    #[test]
    fn les_cartes_gog_reunissent_tous_les_comptes() {
        let d = tempfile::tempdir().unwrap();
        deux_comptes_gog(d.path());
        let mut installe = jg("gog_55");
        installe.installe = true;
        let l = super::cartes_gog(d.path(), &[installe]);
        let mut ids: Vec<&str> = l.iter().map(|j| j.id.as_str()).collect();
        ids.sort();
        assert_eq!(ids, ["gog_1", "gog_2", "gog_55", "gog_9"]);
    }

    /// Lot C4b : un jeu du seul compte ajouté, quand Galaxy est sur le compte Profil, ne s'installe pas par Galaxy (le
    /// message nomme le compte) et se lance par le registre ; au moindre doute, le chemin habituel.
    #[test]
    fn un_jeu_d_un_autre_compte_que_celui_de_galaxy_est_dit() {
        let d = tempfile::tempdir().unwrap();
        let c = deux_comptes_gog(d.path());
        let comptes = crate::boutiques::comptes_gog_lus(&c);
        let g = |compte: &str, cles: &[&str]| crate::boutiques::CompteGalaxyPc { compte: compte.into(), cles: cles.iter().map(|c| c.to_string()).collect() };
        assert_eq!(super::avertissement_installation_hors_galaxy(&comptes, &g("", &[]), "9"), None, "compte de Galaxy inconnu : rien d'affirmé");
        let m = super::avertissement_installation_hors_galaxy(&comptes, &g("4200", &["gog_1", "gog_2"]), "9").unwrap();
        assert_eq!(m, "Ce jeu est sur ton autre compte GOG « Seb ». GOG Galaxy semble ouvert sur un autre compte : pour l'installer, change de compte dans Galaxy.");
        assert!(!m.contains("7700"), "jamais le numéro");
        assert_eq!(super::avertissement_installation_hors_galaxy(&comptes, &g("4200", &[]), "2"), None, "jeu des deux comptes : Galaxy l'a");
        assert_eq!(super::avertissement_installation_hors_galaxy(&comptes, &g("4200", &[]), "1"), None);
        assert_eq!(super::avertissement_installation_hors_galaxy(&comptes, &g("4200", &["gog_9"]), "9"), None, "la bibliothèque complète de Galaxy l'a (jeu caché)");
        // Galaxy sur le compte ajouté : un jeu du compte Profil (sans pseudo) est nommé par son numéro rangé.
        assert!(super::avertissement_installation_hors_galaxy(&comptes, &g("7700", &[]), "1").unwrap().contains("« Compte GOG 1 »"));
        // La base de Galaxy n'est relue que si la question a un sens (tous les numéros Galaxy connus, jeu d'un compte).
        assert!(super::question_galaxy_utile(&comptes, "9"));
        assert!(!super::question_galaxy_utile(&comptes, "12345"));
        let mut sans = comptes.clone();
        sans[1].id_galaxy.clear();
        assert!(!super::question_galaxy_utile(&sans, "9"));
        assert!(!super::question_galaxy_utile(&[], "9"));
    }

    fn noyau_memoire() -> (tempfile::TempDir, crate::noyau::Noyau) {
        let d = tempfile::tempdir().unwrap();
        let n = crate::noyau::Noyau::nouveau(d.path(), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        (d, n)
    }

    /// Revue du lot C4b : avant un changement de compte, le jeton du nom principal est mis à l'abri sous le nom de SON
    /// compte, retrouvé par GOG (jamais déduit de la liste) ; un nom existant n'est jamais écrasé ; GOG injoignable :
    /// rien ne bouge.
    #[tokio::test]
    async fn le_jeton_du_navigateur_est_mis_a_l_abri_sous_son_vrai_compte() {
        use httpmock::prelude::*;
        let (_d, n) = noyau_memoire();
        let principal = crate::boutiques::nom_secret("p1", crate::gog_galaxy::SECRET);
        let s = MockServer::start_async().await;
        s.mock_async(|w, t| {
            w.method(GET).path("/token").query_param("refresh_token", "JETON-A");
            t.status(200).json_body(serde_json::json!({"access_token": "X", "refresh_token": "JETON-A2", "user_id": "4200"}));
        })
        .await;
        s.mock_async(|w, t| {
            w.method(GET).path("/token").query_param("refresh_token", "MORT");
            t.status(401);
        })
        .await;
        s.mock_async(|w, t| {
            w.method(GET).path("/token").query_param("refresh_token", "PANNE");
            t.status(500);
        })
        .await;
        n.coffre.ranger(&principal, "JETON-A").unwrap();
        assert_eq!(super::abriter_jeton_profil(&n, "p1", &s.base_url()).await.unwrap().as_deref(), Some("4200"));
        assert_eq!(n.coffre.lire(&crate::boutiques::secret_gog_ajoute("p1", "4200")).unwrap().as_deref(), Some("JETON-A2"));
        assert_eq!(n.coffre.lire(&principal).unwrap().as_deref(), Some("JETON-A2"));
        // Un nom `#4200` déjà là n'est jamais écrasé.
        n.coffre.ranger(&crate::boutiques::secret_gog_ajoute("p1", "4200"), "DEJA").unwrap();
        n.coffre.ranger(&principal, "JETON-A").unwrap();
        super::abriter_jeton_profil(&n, "p1", &s.base_url()).await.unwrap();
        assert_eq!(n.coffre.lire(&crate::boutiques::secret_gog_ajoute("p1", "4200")).unwrap().as_deref(), Some("DEJA"));
        // GOG injoignable : erreur, le jeton reste tel quel.
        n.coffre.ranger(&principal, "PANNE").unwrap();
        assert!(super::abriter_jeton_profil(&n, "p1", &s.base_url()).await.is_err());
        assert_eq!(n.coffre.lire(&principal).unwrap().as_deref(), Some("PANNE"));
        // Jeton refusé par GOG : il ne vaut plus rien, oublié.
        n.coffre.ranger(&principal, "MORT").unwrap();
        assert_eq!(super::abriter_jeton_profil(&n, "p1", &s.base_url()).await.unwrap(), None);
        assert_eq!(n.coffre.lire(&principal).unwrap(), None);
        // Sans jeton : rien.
        assert_eq!(super::abriter_jeton_profil(&n, "p1", &s.base_url()).await.unwrap(), None);
    }

    /// Revue du lot C4b : après l'écriture de la liste seulement, le jeton provisoire prend le nom principal et le nom du
    /// compte fondu est oublié ; un compte ajouté en double donne son jeton au nom principal s'il n'en a pas.
    #[test]
    fn les_jetons_prennent_leur_nom_apres_l_ecriture_seulement() {
        let (_d, n) = noyau_memoire();
        let principal = crate::boutiques::nom_secret("p1", crate::gog_galaxy::SECRET);
        let provisoire = super::secret_gog_provisoire("p1", "navigateur");
        let b = crate::boutiques::secret_gog_ajoute("p1", "7700");
        n.coffre.ranger(&principal, "A").unwrap();
        n.coffre.ranger(&provisoire, "B").unwrap();
        n.coffre.ranger(&b, "B").unwrap();
        super::poser_jetons_apres_ecriture(&n, "p1", Some(&provisoire), Some("7700"));
        assert_eq!(n.coffre.lire(&principal).unwrap().as_deref(), Some("B"));
        assert_eq!(n.coffre.lire(&provisoire).unwrap(), None);
        assert_eq!(n.coffre.lire(&b).unwrap(), None, "un seul jeton par compte");
        // Le nouveau compte n'a pas de jeton : le nom principal (celui de l'ancien, déjà à l'abri) est vidé.
        super::poser_jetons_apres_ecriture(&n, "p1", Some(&provisoire), None);
        assert_eq!(n.coffre.lire(&principal).unwrap(), None);
        // Même compte ajouté en double, nom principal vide : son jeton y passe.
        n.coffre.ranger(&b, "B2").unwrap();
        super::poser_jetons_apres_ecriture(&n, "p1", None, Some("7700"));
        assert_eq!(n.coffre.lire(&principal).unwrap().as_deref(), Some("B2"));
        assert_eq!(n.coffre.lire(&b).unwrap(), None);
        // Nom principal déjà là : jamais écrasé par le double.
        n.coffre.ranger(&b, "B3").unwrap();
        super::poser_jetons_apres_ecriture(&n, "p1", None, Some("7700"));
        assert_eq!(n.coffre.lire(&principal).unwrap().as_deref(), Some("B2"));
        assert!(super::ranger_si_libre(&n, &principal, "Z") && n.coffre.lire(&principal).unwrap().as_deref() == Some("B2"));
    }

    /// Lot C4b : le dossier temporaire d'un ajout ne s'efface qu'avec preuve (nom, place, pas de lien).
    #[test]
    fn le_dossier_d_un_ajout_gog_s_efface_seulement_avec_preuve() {
        let d = tempfile::tempdir().unwrap();
        let racine = super::racine_ajouts_gog(d.path());
        let ajout = racine.join("ajout-1795400000000");
        std::fs::create_dir_all(ajout.join("EBWebView").join("Default")).unwrap();
        std::fs::write(ajout.join("EBWebView").join("Default").join("Cookies"), b"x").unwrap();
        assert_eq!(super::effacer_ajout_gog(&racine, &ajout), Ok(()));
        assert!(!ajout.exists());
        assert_eq!(super::effacer_ajout_gog(&racine, &ajout), Ok(()), "déjà absent");
        // Un autre nom, ou un dossier hors de la racine : jamais.
        let autre = racine.join("navigateur");
        std::fs::create_dir_all(&autre).unwrap();
        assert_eq!(super::effacer_ajout_gog(&racine, &autre), Err("nom-inattendu"));
        let dehors = d.path().join("ajout-12");
        std::fs::create_dir_all(&dehors).unwrap();
        assert_eq!(super::effacer_ajout_gog(&racine, &dehors), Err("hors-du-dossier"));
        assert!(autre.exists() && dehors.exists());
        assert!(!super::nom_d_ajout_gog("ajout-") && !super::nom_d_ajout_gog("ajout-1a") && super::nom_d_ajout_gog("ajout-1"));
    }

    /// Lot C4b : une jonction nommée « ajout-… » vers un autre dossier n'est jamais suivie (le dossier visé reste).
    #[cfg(windows)]
    #[test]
    fn une_jonction_d_ajout_gog_n_est_jamais_suivie() {
        let d = tempfile::tempdir().unwrap();
        let racine = super::racine_ajouts_gog(d.path());
        std::fs::create_dir_all(&racine).unwrap();
        let cible = d.path().join("precieux");
        std::fs::create_dir_all(&cible).unwrap();
        std::fs::write(cible.join("sauvegarde.sav"), b"x").unwrap();
        let jonction = racine.join("ajout-7");
        let ok = std::process::Command::new("cmd").args(["/C", "mklink", "/J"]).arg(&jonction).arg(&cible).output().is_ok_and(|o| o.status.success());
        if !ok {
            return;
        }
        assert_eq!(super::effacer_ajout_gog(&racine, &jonction), Err("point-d-analyse"));
        assert!(cible.join("sauvegarde.sav").is_file());
        let _ = std::fs::remove_dir(&jonction);
    }

    #[test]
    fn jouer_ou_installer_un_jeu_de_boutique_dit_le_lanceur_absent_et_valide_la_cle() {
        use crate::boutiques_pc::Action;
        let la = |_: &str| true;
        let absent = |_: &str| false;
        assert_eq!(super::adresse_carte_selon("uplay", "uplay_46", Action::Jouer, la).unwrap(), "uplay://launch/46", "la clé de la carte perd son préfixe");
        assert_eq!(super::adresse_carte_selon("amazon", "amazon_amzn1.adg.product.1", Action::Installer, la).unwrap(), "amazon-games://install/amzn1.adg.product.1");
        assert_eq!(super::adresse_carte_selon("steam", "620", Action::Installer, la).unwrap(), "steam://install/620");
        let e = super::adresse_carte_selon("uplay", "uplay_46", Action::Jouer, absent).unwrap_err().to_string();
        assert!(e.contains("Ce jeu a besoin du lanceur Ubisoft Connect, absent de ce PC"), "{e}");
        assert!(super::adresse_carte_selon("uplay", "uplay_4/6", Action::Jouer, la).is_err(), "clé douteuse : rien n'est ouvert");
        let e = super::adresse_carte_selon("battlenet", "battlenet_wow", Action::Jouer, la).unwrap_err().to_string();
        assert!(e.contains("Battle.net"), "pas de lancement deviné : {e}");
    }

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
        // Une mise à jour à la place de l'installation, jamais dans le sous-dossier du programme choisi.
        use std::path::Path;
        assert_eq!(super::racine_installation("Cheat Engine", Path::new(r"E:\outils\Cheat Engine\outils\b.exe")).as_deref(), Some(Path::new(r"E:\outils\Cheat Engine")));
        assert_eq!(super::racine_installation("RetroArch", Path::new(r"E:\Emulateurs\RetroArch\retroarch.exe")).as_deref(), Some(Path::new(r"E:\Emulateurs\RetroArch")));
        assert_eq!(super::racine_installation("RPCS3", Path::new(r"E:\Ailleurs\rpcs3.exe")), None);
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

    #[test]
    fn les_cles_de_la_switch_2_vont_aussi_a_lichii_cite_en_alternative() {
        let c = serde_json::json!({"emulateur": "eglantine", "alternatives": [{"nom": "Lichii", "emulateur": "lichii", "coeur": null}]});
        let (s2, lichii) = ("Nintendo Switch 2", r"E:\Frogtend\Emulateurs\Lichii\lichii.exe");
        assert!(super::programme_pour_le_conseil(&c, s2, r"E:\Frogtend\Emulateurs\Eglantine\eglantine.exe"));
        assert!(super::programme_pour_le_conseil(&c, s2, lichii), "Lichii, alternative de la même console");
        assert!(!super::programme_pour_le_conseil(&c, "Nintendo Switch", lichii), "jamais les clés d'une autre console");
        let sans = serde_json::json!({"emulateur": "eglantine", "alternatives": []});
        assert!(!super::programme_pour_le_conseil(&sans, s2, lichii), "seulement si Firehouse le cite");
        let ra = serde_json::json!({"emulateur": "retroarch", "alternatives": [{"nom": "Mesen", "emulateur": "mesen"}]});
        assert!(!super::programme_pour_le_conseil(&ra, "Nintendo Entertainment System", r"E:\Mesen\Mesen.exe"), "hors famille d'Eden : jamais");
        assert!(!super::programme_pour_le_conseil(&serde_json::json!({}), s2, lichii));
    }

    #[test]
    fn la_fiche_montre_ce_que_ryubing_recevra_sans_rien_ecrire() {
        // Lot 6 : Ryubing portable, la clé de TEST dans la racine de Seb, le dossier de Zelda fabriqué (switch_nca.rs).
        let d = tempfile::tempdir().unwrap();
        let ryu = d.path().join("Ryubing");
        std::fs::create_dir_all(ryu.join("portable")).unwrap();
        let racine = crate::ryubing::racine(&ryu, "Seb");
        std::fs::create_dir_all(racine.join("system")).unwrap();
        let o: String = (0..32u8).map(|i| format!("{:02x}", i.wrapping_mul(7).wrapping_add(3))).collect();
        let jeux = d.path().join("Jeux");
        std::fs::create_dir_all(&jeux).unwrap();
        let jeu = crate::switch_nca::tests::dossier_de_zelda(&jeux);
        let donnees = d.path().join("donnees");
        use super::apercu_ryubing;
        // Sans clés : dit, rien de deviné.
        let r = apercu_ryubing(&donnees, &ryu, "Seb", Some(&jeu), &[]);
        assert!(r.contenus.is_empty() && r.remarque.as_deref().is_some_and(|m| m.contains("clés")), "{:?}", r.remarque);
        std::fs::write(racine.join("system").join("prod.keys"), format!("header_key = {o}\n")).unwrap();
        let r = apercu_ryubing(&donnees, &ryu, "Seb", Some(&jeu), &[]);
        assert_eq!((r.etat.as_str(), r.systeme.as_str(), r.titre_id.as_deref()), ("ok", "ryubing", Some("01007EF00011E000")));
        let vus: Vec<(&str, bool, Option<&str>, Option<&str>, bool)> = r.contenus.iter().map(|c| (c.genre.as_str(), c.installe, c.raison.as_deref(), c.cle.as_deref(), c.actif)).collect();
        assert_eq!(
            vus,
            [
                ("maj", true, None, Some("maj"), true),
                ("maj", false, Some("pas utilisée : une plus récente est là"), None, false),
                ("dlc", true, None, Some("dlc:01007EF00011F001"), true),
                ("dlc", true, None, Some("dlc:01007EF00011F002"), true)
            ]
        );
        // Jamais un numéro de version brut montré (v786432 n'est pas la version que la console affiche).
        assert!(r.contenus.iter().all(|c| c.version.is_none()));
        assert_eq!(r.ignores, ["abime.nsp"]);
        assert!(r.remarque.is_none());
        assert!(!racine.join("games").exists() && !donnees.exists(), "un aperçu n'écrit rien");
        // Après une partie, la personne coupe le DLC 1 et retire la mise à jour dans Ryubing : ça ne compte plus, la fiche
        // montre le choix de la maison (tout allumé), sans raison inventée.
        crate::ryubing::preparer_racine(&ryu, &racine).unwrap();
        std::fs::write(racine.join("system").join("prod.keys"), format!("header_key = {o}\n")).unwrap();
        crate::contenus_switch::donnees_migrees(&donnees);
        crate::ryubing::preparer_contenus(&donnees, &ryu, &racine, &jeu).unwrap().unwrap();
        let dossier = racine.join("games").join("01007ef00011e000");
        let mut l: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dossier.join("dlc.json")).unwrap()).unwrap();
        l[0]["dlc_nca_list"][0]["is_enabled"] = false.into();
        std::fs::write(dossier.join("dlc.json"), l.to_string()).unwrap();
        std::fs::write(dossier.join("updates.json"), br#"{"selected":"","paths":[]}"#).unwrap();
        let r = apercu_ryubing(&donnees, &ryu, "Seb", Some(&jeu), &[]);
        let vus: Vec<(bool, Option<&str>)> = r.contenus.iter().map(|c| (c.installe, c.raison.as_deref())).collect();
        assert_eq!(vus, [(true, None), (false, Some("pas utilisée : une plus récente est là")), (true, None), (true, None)]);
        // Éteint dans Frogtend : montré éteint, pas utilisé, sans « raison » (ce n'est pas un problème).
        crate::contenus_switch::choisir(&donnees, crate::switch_nca::tests::BASE, "dlc:01007EF00011F001", false).unwrap();
        let r = apercu_ryubing(&donnees, &ryu, "Seb", Some(&jeu), &[]);
        assert_eq!((r.contenus[2].actif, r.contenus[2].installe, r.contenus[2].raison.as_deref()), (false, false, None));
        // Un problème n'est jamais jeté : il est dit sur la fiche.
        std::fs::write(dossier.join("dlc.json"), b"{abime").unwrap();
        let r = apercu_ryubing(&donnees, &ryu, "Seb", Some(&jeu), &[]);
        assert!(r.remarque.as_deref().is_some_and(|m| m.contains("La liste des DLC de Ryubing ne se lit pas")), "{:?}", r.remarque);
        assert_eq!(r.contenus[3].raison.as_deref(), Some("Ryubing ne le prend pas pour l'instant"));
        assert!(r.contenus[3].actif && !r.contenus[3].installe, "allumé, mais pas utilisé : dit tel quel");
        // Une mise à jour PLUS RÉCENTE choisie dans Ryubing : gardée, et dite.
        std::fs::write(dossier.join("dlc.json"), b"[]").unwrap();
        let recente = jeux.join("Zelda maj [v917504].nsp");
        std::fs::copy(jeux.join("Zelda maj [v786432].nsp"), &recente).unwrap();
        // Lue par Frogtend comme la plus récente : elle devient la sienne ; une encore plus récente ailleurs, à la personne.
        let ailleurs = d.path().join("Ailleurs");
        std::fs::create_dir_all(&ailleurs).unwrap();
        std::fs::rename(&recente, ailleurs.join("Zelda maj [v917504].nsp")).unwrap();
        let s = ailleurs.join("Zelda maj [v917504].nsp").to_string_lossy().to_string();
        std::fs::write(dossier.join("updates.json"), serde_json::to_string(&serde_json::json!({ "selected": s, "paths": [s] })).unwrap()).unwrap();
        let r = apercu_ryubing(&donnees, &ryu, "Seb", Some(&jeu), &[]);
        assert_eq!(r.contenus[0].raison.as_deref(), Some("pas utilisée : une mise à jour plus récente est choisie dans Ryubing"));
        assert!(r.remarque.as_deref().is_some_and(|m| m.contains("plus récente, est choisie dans Ryubing")), "{:?}", r.remarque);
        // Un fichier des choix abîmé : dit sur la fiche.
        std::fs::write(crate::contenus_switch::chemin(&donnees), b"{abime").unwrap();
        let r = apercu_ryubing(&donnees, &ryu, "Seb", Some(&jeu), &[]);
        assert!(r.remarque.as_deref().is_some_and(|m| m.contains("est abîmé")), "{:?}", r.remarque);
        assert_eq!(r.contenus[2].raison, None, "éteint : aucune raison technique à dire");
        // Pas installé par Frogtend (pas portable) : dit.
        std::fs::remove_dir_all(ryu.join("portable")).unwrap();
        assert!(apercu_ryubing(&donnees, &ryu, "Seb", Some(&jeu), &[]).remarque.is_some_and(|m| m.contains("pas été installé par Frogtend")));
    }

    #[test]
    fn les_choix_switch_sont_refuses_pendant_une_partie_ou_un_emulateur_ouvert_seul() {
        use super::choix_switch_permis;
        use crate::erreurs::Erreur;
        assert!(choix_switch_permis(false, None).is_ok());
        assert!(matches!(choix_switch_permis(true, None), Err(Erreur::Refus(m)) if m.contains("partie est en cours")));
        assert!(matches!(choix_switch_permis(false, Some("Ryubing".into())), Err(Erreur::Refus(m)) if m.contains("Ryubing est ouvert")));
    }

    #[test]
    fn ryubing_ouvert_seul_est_refuse_si_les_parties_ne_peuvent_pas_etre_a_l_abri() {
        use super::ryubing_seul_aux_choix;
        let d = tempfile::tempdir().unwrap();
        let ryu = d.path().join("Ryubing");
        let racine = crate::ryubing::racine(&ryu, "Seb");
        let save = racine.join("bis").join("user").join("save").join("1");
        std::fs::create_dir_all(&save).unwrap();
        std::fs::write(save.join("p.sav"), b"partie").unwrap();
        // Sans la reprise des retraits d'avant : rien n'est fait, c'est seulement noté.
        let pas_migre = d.path().join("pas-migre");
        let n = ryubing_seul_aux_choix(&pas_migre, &ryu, &racine).unwrap();
        assert!(n.iter().any(|x| x.contains("pas encore repris")), "{n:?}");
        let donnees = crate::contenus_switch::donnees_migrees(&d.path().join("donnees"));
        crate::contenus_switch::choisir(&donnees, crate::switch_nca::tests::BASE, "maj", false).unwrap();
        std::fs::write(ryu.join(".frogtend-sauvegardes"), b"bloque").unwrap();
        assert!(matches!(ryubing_seul_aux_choix(&donnees, &ryu, &racine), Err(crate::erreurs::Erreur::Refus(m)) if m.contains("ne s'ouvre pas")));
        // L'abri possible : fait, au nom de l'ouverture seule (jamais celui d'un jeu en particulier).
        std::fs::remove_file(ryu.join(".frogtend-sauvegardes")).unwrap();
        let n = ryubing_seul_aux_choix(&donnees, &ryu, &racine).unwrap();
        assert!(n.iter().any(|x| x.contains("à l'abri")), "{n:?}");
        let abri = std::fs::read_dir(ryu.join(".frogtend-sauvegardes")).unwrap().flatten().next().unwrap().file_name().to_string_lossy().to_string();
        assert!(abri.contains("parties-ouverture-seul-Seb"), "{abri}");
    }

    #[test]
    fn l_apercu_d_eden_montre_les_interrupteurs_sans_rien_ecrire() {
        use super::apercu_eden;
        let d = tempfile::tempdir().unwrap();
        let jeux = d.path().join("Jeux");
        std::fs::create_dir_all(&jeux).unwrap();
        let jeu = crate::switch_nca::tests::dossier_de_zelda(&jeux);
        let eden = d.path().join("Eden");
        std::fs::create_dir_all(eden.join("user").join("keys")).unwrap();
        let donnees = d.path().join("donnees");
        // Sans clés : dit, rien d'autre.
        let r = apercu_eden(&donnees, &eden, Some(&jeu), &[]);
        assert!(r.remarque.as_deref().is_some_and(|m| m.contains("pas encore les clés")) && r.contenus.is_empty(), "{:?}", r.remarque);
        assert_eq!(r.systeme, "eden");
        let o: String = (0..32u8).map(|i| format!("{:02x}", i.wrapping_mul(7).wrapping_add(3))).collect();
        std::fs::write(eden.join("user").join("keys").join("prod.keys"), format!("header_key = {o}\n")).unwrap();
        // Tout sera utilisé (sauf la vieille mise à jour) ; plus de phrase « donné au prochain lancement ».
        let r = apercu_eden(&donnees, &eden, Some(&jeu), &[]);
        let vus: Vec<(&str, bool, Option<&str>, bool)> = r.contenus.iter().map(|c| (c.genre.as_str(), c.installe, c.raison.as_deref(), c.cle.is_some())).collect();
        assert_eq!(vus, [("maj", true, None, true), ("maj", false, Some("pas utilisée : une plus récente est là"), false), ("dlc", true, None, true), ("dlc", true, None, true)]);
        let m = r.remarque.clone().unwrap_or_default();
        assert!(!m.contains("prochain lancement") && m.contains("d'autres jeux"), "{m}");
        assert_eq!(r.ignores, ["abime.nsp"]);
        assert!(!crate::contenus::ini_eden(&eden).exists() && !donnees.exists(), "rien d'écrit");
        // Un DLC éteint sur deux : Eden les garde tous (montré utilisé, interrupteur éteint).
        crate::contenus_switch::choisir(&donnees, crate::switch_nca::tests::BASE, "dlc:01007EF00011F001", false).unwrap();
        let r = apercu_eden(&donnees, &eden, Some(&jeu), &[]);
        assert_eq!((r.contenus[2].actif, r.contenus[2].installe, r.contenus[3].actif), (false, true, true));
        // Les deux éteints, et la mise à jour : rien d'utilisé.
        crate::contenus_switch::choisir(&donnees, crate::switch_nca::tests::BASE, "dlc:01007EF00011F002", false).unwrap();
        crate::contenus_switch::choisir(&donnees, crate::switch_nca::tests::BASE, "maj", false).unwrap();
        let r = apercu_eden(&donnees, &eden, Some(&jeu), &[]);
        assert!(r.contenus.iter().all(|c| !c.installe && !c.actif && c.raison.as_deref().is_none_or(|x| x.contains("plus récente"))), "{:?}", r.contenus.iter().map(|c| (&c.raison, c.actif)).collect::<Vec<_>>());
        assert!(!crate::contenus::ini_eden(&eden).exists(), "un aperçu n'écrit rien chez Eden");
        // Pas installé par Frogtend : dit.
        std::fs::remove_dir_all(eden.join("user")).unwrap();
        assert!(apercu_eden(&donnees, &eden, Some(&jeu), &[]).remarque.is_some_and(|m| m.contains("pas été installé par Frogtend")));
        assert!(apercu_eden(&donnees, &eden, None, &[]).remarque.is_some_and(|m| m.contains("introuvable")));
    }

    #[test]
    fn ryubing_recoit_les_fichiers_de_son_alternative_jamais_ceux_d_eden() {
        // La forme servie le 15/11 (contrat 1.41), abrégée.
        let cles_eden = serde_json::json!([{"fichier": "prod.keys", "type": "fichier", "racine": "dossier_emulateur", "chemin": "user/keys/prod.keys"}]);
        let cles_ryu = serde_json::json!([{"fichier": "prod.keys", "type": "fichier", "racine": "dossier_emulateur", "chemin": "portable/system/prod.keys"}]);
        let c = serde_json::json!({"emulateur": "eden", "bios": cles_eden, "alternatives": [
            {"nom": "Ryubing", "emulateur": "ryubing", "commande": "-f \"%rom%\"", "bios": cles_ryu},
            {"nom": "Citron", "emulateur": "citron_neo", "bios": null}
        ]});
        let (sw, ryu) = ("Nintendo Switch", r"E:\Frogtend\Emulateurs\Ryubing\Ryujinx.exe");
        let (id, b, facultatives) = super::bios_pour_le_programme(&c, sw, ryu).unwrap();
        assert_eq!((id.as_str(), b, facultatives), ("ryubing", &cles_ryu, true), "les siens, avec title.keys");
        let (id, b, facultatives) = super::bios_pour_le_programme(&c, sw, r"E:\Emu\Eden\eden.exe").unwrap();
        assert_eq!((id.as_str(), b, facultatives), ("eden", &cles_eden, false), "Eden : inchangé");
        // `bios` null (non décrits) ou absent : rien, jamais ceux du conseil.
        assert!(super::bios_pour_le_programme(&c, sw, r"E:\Emu\Citron\citron_neo.exe").is_none());
        let sans = serde_json::json!({"emulateur": "eden", "bios": cles_eden, "alternatives": [{"nom": "Ryubing", "emulateur": "ryubing"}]});
        assert!(super::bios_pour_le_programme(&sans, sw, ryu).is_none());
        assert!(!super::programme_pour_le_conseil(&c, sw, ryu), "Ryubing n'est pas de la famille d'Eden");
        // Non cité : rien.
        let seul = serde_json::json!({"emulateur": "eden", "bios": cles_eden, "alternatives": []});
        assert!(super::bios_pour_le_programme(&seul, sw, ryu).is_none());
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

    /// RÉEL (✅ Seb 11/11), dans un dossier TEMPORAIRE effacé à la fin (`FROGTEND_ESSAI_DOSSIER`, par exemple
    /// `D:\temp-tests-frogtend`) : Vita3K officiel installé en portable (≈ 38 Mo), puis les trois `.PUP` servis par
    /// Firehouse (≈ 305 Mo) téléchargés, vérifiés et installés par `Vita3K --firmware`, comme le fait Frogtend.
    /// `FROGTEND_PROFIL_ESSAI=<id> FROGTEND_ESSAI_DOSSIER=<dossier> cargo test essai_firmware_vita -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_firmware_vita() {
        use crate::coffre::Coffre as _;
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").unwrap();
        let racine = std::env::var("FROGTEND_ESSAI_DOSSIER").unwrap();
        let d = tempfile::tempdir_in(racine).unwrap();
        let f = crate::emulateurs::fiche("vita3k").unwrap();
        let p = crate::emulateurs::derniere_version(f).await.unwrap();
        let paquet = d.path().join("vita3k.paquet");
        let n = crate::emulateurs::telecharger(&p.url, &paquet, &|_, _| {}).await.unwrap();
        let v = d.path().join("Vita3K");
        let programme = crate::emulateurs::installer_paquet(f, &paquet, &v).unwrap();
        println!("Vita3K {} : {n} octets → {}", p.version, programme.display());
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().unwrap();
        let s = crate::source::Source::Firehouse(crate::firehouse::Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap());
        let conseil = s.emulateurs(super::PLATEFORME_VITA).await.unwrap();
        let servis = crate::vita3k::firmwares_servis(&crate::bios::lire_bios(&conseil["conseil"]["bios"], &mut Default::default()));
        println!("{} fichiers servis", servis.len());
        let mut pups = Vec::new();
        for (i, f) in crate::vita3k::firmwares_manquants(&v, &servis) {
            let debut = std::time::Instant::now();
            let pup = crate::bios::telecharger_firmware(&s, &f, &v, &|_, _| {}).await.unwrap();
            println!("{} téléchargé et vérifié en {:?}", f.fichier, debut.elapsed());
            // Copié à part : le 2e cas (ci-dessous) le réinstalle sans retélécharger.
            let copie = d.path().join(format!("{i}.PUP"));
            std::fs::copy(&pup, &copie).unwrap();
            pups.push((i, f.clone(), copie));
            let debut = std::time::Instant::now();
            let r = crate::vita3k::installer_firmware(&v, &programme, i, &f, &pup);
            println!("{} : {r:?} en {:?}", f.fichier, debut.elapsed());
            let _ = std::fs::remove_file(&pup);
        }
        // 2e cas : une partie a déjà eu lieu avant le firmware → config.yml écrit par Frogtend (`choisir`), presque vide.
        let v2 = d.path().join("Vita3K-2");
        let programme2 = crate::emulateurs::installer_paquet(f, &paquet, &v2).unwrap();
        crate::vita3k::choisir(&v2, "00").unwrap();
        println!("2E CAS, config.yml de Frogtend : {:?}", std::fs::read_to_string(v2.join("portable").join("config.yml")).unwrap());
        for (i, f, pup) in &pups {
            println!("2E CAS {} : {:?}", f.fichier, crate::vita3k::installer_firmware(&v2, &programme2, *i, f, pup));
        }
        println!("2E CAS firmware_installe = {}", crate::vita3k::firmware_installe(&v2));
        assert!(crate::vita3k::firmware_installe(&v2));
        let fs = v.join("portable").join("fs");
        for s in ["os0", "pd0", "sa0", "vs0", "vs0/sys/external"] {
            let x = fs.join(s);
            println!("{s:<18} {}", if x.is_dir() { format!("{} éléments", std::fs::read_dir(&x).unwrap().count()) } else { "ABSENT".into() });
        }
        println!("note : {:?}", std::fs::read_to_string(fs.join(".frogtend-firmware")));
        println!("firmware_installe = {}", crate::vita3k::firmware_installe(&v));
        println!("encore à installer : {:?}", crate::vita3k::firmwares_manquants(&v, &servis).iter().map(|(_, f)| &f.fichier).collect::<Vec<_>>());
        assert!(crate::vita3k::firmware_installe(&v));
        assert!(crate::vita3k::firmwares_manquants(&v, &servis).is_empty());
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
