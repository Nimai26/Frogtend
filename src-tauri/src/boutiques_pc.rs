//! Les jeux INSTALLÉS sur ce PC par chaque boutique (import automatique, lot A, ✅ Seb 18/11) : ils entrent SEULS dans
//! la ludothèque, relus à chaque ouverture d'un profil, sans clé d'API, sans GOG Galaxy, sans connexion. Jouer et
//! Installer passent par le lanceur officiel.
//!
//! Méthode (étude du 18/11 sur Venkman, en lecture seule, et méthode de Playnite, relue sans recopier son code) :
//! - Steam : `HKCU\Software\Valve\Steam\SteamPath`, `libraryfolders.vdf`, `appmanifest_<appid>.acf` ;
//! - Epic : `HKLM\SOFTWARE\Epic Games\EpicGamesLauncher\AppDataPath`, ses `Manifests\*.item`, `LauncherInstalled.dat` ;
//! - GOG (sans Galaxy) : `HKLM\SOFTWARE\GOG.com\Games\<id>` (`gameName`, `path`, `exe`…) ;
//! - Ubisoft Connect : `HKLM\SOFTWARE\Ubisoft\Launcher\Installs\<id>\InstallDir` (avec des « / »), nom pris dans la
//!   clé de désinstallation `Uplay Install <id>` ;
//! - Amazon Games : `%LOCALAPPDATA%\Amazon Games\Data\Games\Sql\GameInstallInfo.sqlite`, table `DbSet` (`Installed`) ;
//! - Xbox / Microsoft Store : les paquets du compte (`HKCU\…\AppModel\Repository\Packages`, `PackageRootFolder`) qui
//!   ont un `MicrosoftGame.config` (un JEU, pas une application) ;
//! - Battle.net : les clés de désinstallation dont la commande est `Battle.net… --uid=<id>`.
//!
//! RÈGLES : LECTURE SEULE chez les lanceurs (jamais une écriture dans leurs fichiers ni dans leur registre) ; une base
//! SQLite est lue sur une COPIE (avec -wal et -shm) effacée ensuite ; **un lanceur absent ou illisible rend `None`**,
//! jamais « zéro jeu » (sinon toutes ses cartes partiraient).

use crate::boutiques::JeuBoutique;
use crate::registre::{sous_cles, valeurs, Racine};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Les boutiques lues sur le PC, dans l'ordre d'affichage. Le nom est celui des clés de GOG Galaxy (`xboxone`, `uplay`,
/// `battlenet`…) : un même jeu vu par Galaxy et par le PC a le même numéro (`ludotheque::id_boutique`).
pub const BOUTIQUES_PC: [&str; 7] = ["steam", "epic", "gog", "uplay", "amazon", "xboxone", "battlenet"];

/// Le plafond d'une base SQLite copiée pour être lue (Amazon) : au-delà, elle n'est pas lue (jamais un disque rempli).
pub const PLAFOND_BASE: u64 = 64 * 1024 * 1024;

/// Les valeurs d'une clé du registre (nom → texte).
pub type Valeurs = BTreeMap<String, String>;

/// Le nom d'une boutique pour les personnes.
pub fn nom_boutique(b: &str) -> &'static str {
    match b {
        "steam" => "Steam",
        "epic" => "Epic Games",
        "gog" => "GOG",
        "uplay" => "Ubisoft Connect",
        "amazon" => "Amazon Games",
        "xboxone" | "xbox" => "Xbox",
        "battlenet" => "Battle.net",
        _ => "sa boutique",
    }
}

/// Un identifiant de boutique sûr dans une adresse ou un nom de fichier (lettres, chiffres, « . _ - »).
pub fn cle_sure(s: &str) -> bool {
    crate::boutiques::id_epic_sur(s)
}

/// Le nom d'un jeu sans ses marques déposées (« Horizon Zero Dawn™ » → « Horizon Zero Dawn »).
pub fn sans_marques(n: &str) -> String {
    n.replace(['™', '®', '©'], "").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Un jeu installé, tel que la ludothèque le range (`id` : l'appid pour Steam, « <boutique>_<clé> » sinon).
fn jeu(boutique: &str, cle: &str, nom: &str, dossier: &Path) -> JeuBoutique {
    let nom = sans_marques(nom);
    JeuBoutique {
        id: if boutique == "steam" { cle.to_string() } else { format!("{boutique}_{cle}") },
        nom: if nom.is_empty() { cle.to_string() } else { nom },
        minutes: 0,
        derniere: 0,
        installe: true,
        plateforme: boutique.to_string(),
        image: None,
        dossier: Some(dossier.to_string_lossy().into_owned()),
        pret: Some(true),
        succes: None,
        releases: vec![],
        titre_douteux: false,
    }
}

/// Trie par nom et ne garde qu'une fois chaque jeu.
fn ranger(mut l: Vec<JeuBoutique>) -> Vec<JeuBoutique> {
    l.sort_by(|a, b| a.nom.to_lowercase().cmp(&b.nom.to_lowercase()).then(a.id.cmp(&b.id)));
    let mut vus = std::collections::BTreeSet::new();
    l.retain(|j| vus.insert(j.id.clone()));
    l
}

// ───────────────────────────── Steam ─────────────────────────────

/// Les outils de Steam qui ne sont pas des jeux (redistribuables, moteurs de compatibilité, SteamVR…).
pub fn est_outil_steam(appid: &str, nom: &str) -> bool {
    const OUTILS: [&str; 9] = ["228980", "250820", "1070560", "1391110", "1628350", "1493710", "1826330", "961940", "1887720"];
    let n = nom.to_lowercase();
    OUTILS.contains(&appid) || n.contains("redistributable") || n.starts_with("steamworks") || n.starts_with("proton ") || n.starts_with("steam linux runtime") || n == "steamvr"
}

/// Les jeux Steam installés (`dossier_steam` : `boutiques::dossier_steam`). `None` : Steam absent, ou sa bibliothèque
/// principale illisible.
pub fn lire_steam(dossier_steam: Option<&Path>) -> Option<Vec<JeuBoutique>> {
    let d = dossier_steam?;
    std::fs::read_dir(d.join("steamapps")).ok()?;
    let l = crate::boutiques::installes_steam_dossiers(d)
        .into_iter()
        .filter(|(appid, i)| !est_outil_steam(appid, i.nom.as_deref().unwrap_or("")))
        .map(|(appid, i)| {
            let mut j = jeu("steam", &appid, i.nom.as_deref().unwrap_or(&appid), &i.dossier);
            j.pret = Some(i.pret);
            j
        })
        .collect();
    Some(ranger(l))
}

/// La liste Steam d'un profil : celle de son compte (clé d'API, si elle est reliée), avec l'état lu sur ce PC, PLUS les
/// jeux installés que cette liste n'a pas (un profil sans clé voit ses jeux installés).
pub fn fusion_steam(api: Vec<JeuBoutique>, locaux: &[JeuBoutique]) -> Vec<JeuBoutique> {
    let mut l = api;
    for j in &mut l {
        let i = locaux.iter().find(|x| x.id == j.id);
        j.dossier = i.and_then(|i| i.dossier.clone());
        j.pret = i.and_then(|i| i.pret);
        j.installe = i.is_some();
    }
    for x in locaux {
        if !l.iter().any(|j| j.id == x.id) {
            l.push(x.clone());
        }
    }
    l
}

/// Les jeux GOG d'un profil (lot C4a, sur le modèle de `fusion_steam`) : la liste de son COMPTE (lue par la page, peut
/// être vide), avec l'état « installé » lu sur ce PC (registre de GOG), PLUS les jeux installés que la liste n'a pas.
/// Les ids sont ceux de GOG Galaxy (`gog_<id>`) : une seule carte par jeu. Ce que GOG Galaxy sait du même jeu (liste
/// `galaxy` du profil, si elle a été lue) complète la carte : copies sur d'autres boutiques (réunion des copies),
/// titre, temps de jeu, succès et jaquette verticale — la carte du compte passe AVANT celle de Galaxy (`noyau::ranger_dans`),
/// elle ne doit rien perdre de ce que Galaxy montrait.
pub fn fusion_gog(compte: Vec<JeuBoutique>, installes: &[JeuBoutique], galaxy: &[JeuBoutique]) -> Vec<JeuBoutique> {
    let mut l = compte;
    for j in &mut l {
        let i = installes.iter().find(|x| x.id == j.id);
        j.dossier = i.and_then(|i| i.dossier.clone());
        j.pret = i.and_then(|i| i.pret);
        j.installe = i.is_some();
    }
    for x in installes {
        if !l.iter().any(|j| j.id == x.id) {
            l.push(x.clone());
        }
    }
    for j in &mut l {
        let Some(g) = galaxy.iter().find(|g| g.id == j.id) else { continue };
        // Le titre de Galaxy d'abord (sans ™), s'il en a un vrai (Galaxy met sa clé « gog_… » quand il n'en a pas).
        let titre = sans_marques(&g.nom);
        if !titre.is_empty() && g.nom != g.id {
            j.nom = titre;
        }
        if j.releases.is_empty() {
            j.releases = g.releases.clone();
        }
        j.succes = j.succes.or(g.succes);
        j.minutes = j.minutes.max(g.minutes);
        j.derniere = j.derniere.max(g.derniere);
        // La jaquette verticale de Galaxy (images.gog.com) plutôt que la vignette horizontale de la liste du compte.
        if g.image.as_deref().is_some_and(crate::boutiques::image_autorisee) {
            j.image = g.image.clone();
        }
    }
    ranger(l)
}

// ───────────────────────────── Epic ─────────────────────────────

/// Les jeux Epic installés (`boutiques::installes_epic_du_pc`), mis en forme.
pub fn lire_epic(installes: Option<Vec<crate::boutiques::JeuEpic>>) -> Option<Vec<JeuBoutique>> {
    Some(ranger(installes?.iter().map(|e| jeu("epic", &e.app, &e.nom, &e.dossier)).collect()))
}

// ───────────────────────────── GOG ─────────────────────────────

/// La clé des jeux GOG dans le registre (vue 32 bits : `WOW6432Node`, lue aussi).
pub const CLE_GOG: &str = r"SOFTWARE\GOG.com\Games";

/// Un jeu GOG du registre : son numéro, s'il en est un (chiffres seulement).
fn id_gog(sous_cle: &str, v: &Valeurs) -> Option<String> {
    let id = v.get("gameID").or(v.get("productID")).map(String::as_str).unwrap_or(sous_cle).trim().to_string();
    (!id.is_empty() && id.len() <= 20 && id.bytes().all(|c| c.is_ascii_digit())).then_some(id)
}

/// Un DLC GOG (pas une carte) : il dépend d'un autre jeu (`dependsOn`), ou son `goggame-<id>.info` dit un autre jeu
/// racine (`rootGameId`, méthode de Playnite).
fn dlc_gog(id: &str, v: &Valeurs, dossier: &Path) -> bool {
    if v.get("dependsOn").is_some_and(|d| !d.trim().is_empty() && d.trim() != id) {
        return true;
    }
    let info = dossier.join(format!("goggame-{id}.info"));
    std::fs::metadata(&info)
        .ok()
        .filter(|m| m.len() <= 1024 * 1024)
        .and_then(|_| std::fs::read(&info).ok())
        .and_then(|o| serde_json::from_slice::<serde_json::Value>(&o).ok())
        .and_then(|v| v["rootGameId"].as_str().map(String::from))
        .is_some_and(|r| !r.is_empty() && r != id)
}

/// Les jeux GOG installés, d'après les entrées du registre (`sous_cles(Machine, CLE_GOG)`). `None` : aucune entrée lue
/// (GOG n'a jamais rien installé ici).
pub fn lire_gog(entrees: Option<Vec<(String, Valeurs)>>) -> Option<Vec<JeuBoutique>> {
    let l = entrees?
        .iter()
        .filter_map(|(sc, v)| {
            let id = id_gog(sc, v)?;
            let dossier = PathBuf::from(v.get("path")?.trim());
            if !dossier.is_dir() || dlc_gog(&id, v, &dossier) {
                return None;
            }
            let nom = v.get("gameName").or(v.get("startMenu")).cloned().unwrap_or_else(|| id.clone());
            Some(jeu("gog", &id, &nom, &dossier))
        })
        .collect();
    Some(ranger(l))
}

/// Le programme d'un jeu GOG à lancer SANS Galaxy (valeurs `exe`, `workingDir`, `launchParam` du registre) : le
/// programme doit être DANS le dossier du jeu (jamais un autre programme, même si le registre disait autre chose).
#[derive(Debug, Clone, PartialEq)]
pub struct ProgrammeGog {
    pub exe: PathBuf,
    pub dossier_travail: PathBuf,
    pub arguments: String,
}

pub fn programme_gog(v: &Valeurs) -> Option<ProgrammeGog> {
    let dossier = PathBuf::from(v.get("path")?.trim());
    let exe = PathBuf::from(v.get("exe")?.trim());
    let racine = crate::boutiques::cle_chemin(&dossier);
    let dans = |p: &Path| {
        let c = crate::boutiques::cle_chemin(p);
        c == racine || c.starts_with(&format!("{racine}\\"))
    };
    if racine.is_empty() || exe == dossier || !dans(&exe) || !exe.is_file() || !exe.extension().is_some_and(|x| x.eq_ignore_ascii_case("exe")) {
        return None;
    }
    // Le dossier de travail aussi : dans le dossier du jeu, sinon le dossier du jeu.
    let travail = v.get("workingDir").map(|w| PathBuf::from(w.trim())).filter(|w| w.is_dir() && dans(w)).unwrap_or(dossier);
    Some(ProgrammeGog { exe, dossier_travail: travail, arguments: v.get("launchParam").map(|a| a.trim().to_string()).unwrap_or_default() })
}

/// Le plafond d'un `goggame-<id>.info` lu pour lancer un jeu (le même que pour reconnaître un DLC).
pub const PLAFOND_INFO_GOG: u64 = 1024 * 1024;

/// La tâche retenue pour lancer un jeu GOG SANS Galaxy (lot C4c-1, ✅ Seb 25/11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategorieTache {
    /// La tâche de catégorie « game » (Cyberpunk : `bin\x64\Cyberpunk2077.exe`, pas son lanceur REDprelauncher).
    Jeu,
    /// À défaut, la tâche PRINCIPALE (`isPrimary`) : le petit programme de démarrage d'un jeu Unreal, comme le font
    /// Heroic et Playnite.
    Principale,
}

impl CategorieTache {
    /// Le mot noté au journal.
    pub fn mot(self) -> &'static str {
        match self {
            CategorieTache::Jeu => "game",
            CategorieTache::Principale => "principale",
        }
    }
}

/// Une tâche de `goggame-<id>.info`, prête à lancer : son programme (dans le dossier du jeu) et sa catégorie.
#[derive(Debug, Clone, PartialEq)]
pub struct TacheGog {
    pub programme: ProgrammeGog,
    pub categorie: CategorieTache,
}

/// Pourquoi aucune tâche n'est lancée directement (le secours prend alors le relais : Galaxy, puis le registre).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotifSecours {
    /// Pas de `goggame-<id>.info` dans le dossier du jeu (ou une clé qui n'est pas un numéro GOG).
    SansFichier,
    /// Le fichier dépasse le plafond (`PLAFOND_INFO_GOG`).
    TropGros,
    /// Le fichier n'est pas un JSON lisible.
    Illisible,
    /// Le fichier parle d'un autre jeu (`gameId` ou `rootGameId` différent).
    AutreJeu,
    /// Aucune tâche « game » ni principale utilisable (URLTask, chemin refusé, programme absent…).
    AucuneTache,
}

impl MotifSecours {
    /// La raison en mots, pour le journal (jamais de chemin ni de titre).
    pub fn en_mots(self) -> &'static str {
        match self {
            MotifSecours::SansFichier => "pas de fichier goggame.info",
            MotifSecours::TropGros => "fichier goggame.info trop gros",
            MotifSecours::Illisible => "fichier goggame.info illisible",
            MotifSecours::AutreJeu => "fichier goggame.info d'un autre jeu",
            MotifSecours::AucuneTache => "aucune tâche de jeu utilisable",
        }
    }
}

/// Un morceau de chemin relatif écrit par GOG, sûr : ni jeton de Galaxy (`<?…?>`), ni `..`, ni chemin absolu (racine,
/// lettre de disque, partage).
fn chemin_relatif_sur(p: &str) -> bool {
    let p = p.trim();
    // « .. » refusé comme MORCEAU du chemin (`Component::ParentDir`) : « Jeu..exe » reste un nom de fichier valable.
    let remonte = Path::new(p).components().any(|c| matches!(c, std::path::Component::ParentDir));
    !p.is_empty() && !p.contains("<?") && !remonte && !p.contains(':') && !p.starts_with(['\\', '/']) && !Path::new(p).has_root()
}

/// Une tâche `FileTask` → le programme à lancer, ou `None` si elle est refusée (voir `tache_jeu_gog`).
fn programme_de_tache(dossier: &Path, t: &serde_json::Value) -> Option<ProgrammeGog> {
    if t["type"].as_str() != Some("FileTask") {
        return None;
    }
    let chemin = t["path"].as_str().filter(|p| chemin_relatif_sur(p))?;
    let exe = dossier.join(chemin.trim());
    // Un vrai .exe, DANS le dossier du jeu, sans jonction ni lien entre les deux.
    if !exe.extension().is_some_and(|x| x.eq_ignore_ascii_case("exe"))
        || !exe.is_file()
        || !crate::garde::sous(&exe, dossier)
        || crate::garde::meme(&exe, dossier)
        || crate::garde::lien_entre(dossier, &exe)
    {
        return None;
    }
    // Le dossier de travail : celui de la tâche s'il est sûr et existe, le dossier du jeu sans lui ; jamais ailleurs.
    let travail = match &t["workingDir"] {
        serde_json::Value::Null => dossier.to_path_buf(),
        serde_json::Value::String(w) if w.trim().is_empty() => dossier.to_path_buf(),
        serde_json::Value::String(w) if chemin_relatif_sur(w) => {
            let w = dossier.join(w.trim());
            if !w.is_dir() || !crate::garde::sous(&w, dossier) || crate::garde::lien_entre(dossier, &w) {
                return None;
            }
            w
        }
        _ => return None,
    };
    // Les arguments : une chaîne seulement (passée telle quelle), sans jeton de Galaxy à remplacer.
    let arguments = match &t["arguments"] {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(a) if !a.contains("<?") => a.trim().to_string(),
        _ => return None,
    };
    Some(ProgrammeGog { exe, dossier_travail: travail, arguments })
}

/// La tâche qui lance un jeu GOG SANS Galaxy (lot C4c-1, ✅ Seb 25/11), d'après SEULEMENT `goggame-<cle>.info` du
/// dossier du jeu (lecture seule, plafonnée) : d'abord une `FileTask` de catégorie « game » (dans la langue du jeu ou
/// « * » ; le 64 bits préféré, une tâche seulement 32 bits acceptée à défaut ; puis la principale préférée, sinon la
/// première), sinon la tâche principale si c'est une `FileTask` (celle de la langue du jeu d'abord). Le fichier doit parler de CE jeu (`gameId`, et `rootGameId` s'il est là). Tout ce qui est douteux est
/// refusé : le secours (Galaxy, puis le registre) prend alors le relais. Jamais de panique.
pub fn tache_jeu_gog(dossier: &Path, cle: &str) -> Result<TacheGog, MotifSecours> {
    use std::io::Read;
    if cle.is_empty() || cle.len() > 20 || !cle.bytes().all(|c| c.is_ascii_digit()) {
        return Err(MotifSecours::SansFichier);
    }
    let info = dossier.join(format!("goggame-{cle}.info"));
    let f = std::fs::File::open(&info).map_err(|_| MotifSecours::SansFichier)?;
    let mut octets = Vec::new();
    f.take(PLAFOND_INFO_GOG + 1).read_to_end(&mut octets).map_err(|_| MotifSecours::Illisible)?;
    if octets.len() as u64 > PLAFOND_INFO_GOG {
        return Err(MotifSecours::TropGros);
    }
    let v: serde_json::Value = serde_json::from_slice(&octets).map_err(|_| MotifSecours::Illisible)?;
    if !v.is_object() {
        return Err(MotifSecours::Illisible);
    }
    if v["gameId"].as_str() != Some(cle) || (!v["rootGameId"].is_null() && v["rootGameId"].as_str() != Some(cle)) {
        return Err(MotifSecours::AutreJeu);
    }
    let Some(taches) = v["playTasks"].as_array() else { return Err(MotifSecours::AucuneTache) };
    // Les langues du jeu installé (en-tête) ; sans elles, toute langue convient.
    let langues: Vec<String> = v["languages"].as_array().map(|l| l.iter().filter_map(|x| x.as_str()).map(str::to_lowercase).collect()).unwrap_or_default();
    let langue_ok = |t: &serde_json::Value| match t["languages"].as_array() {
        None => true,
        Some(l) if l.is_empty() => true,
        Some(l) => l.iter().filter_map(|x| x.as_str()).any(|x| x == "*" || langues.is_empty() || langues.contains(&x.to_lowercase())),
    };
    let bits_ok = |t: &serde_json::Value| match t["osBitness"].as_array() {
        None => true,
        Some(b) => b.iter().any(|x| x.as_str() == Some("64")),
    };
    let principale = |t: &serde_json::Value| t["isPrimary"].as_bool() == Some(true);
    let mut jeux: Vec<&serde_json::Value> = taches.iter().filter(|t| t["type"].as_str() == Some("FileTask") && t["category"].as_str() == Some("game") && langue_ok(t)).collect();
    // Le 64 bits d'abord (une tâche seulement 32 bits reste acceptée : Windows 64 bits la lance), puis la principale,
    // puis l'ordre du fichier (tri stable).
    jeux.sort_by_key(|t| (!bits_ok(t), !principale(t)));
    if let Some(p) = jeux.iter().find_map(|t| programme_de_tache(dossier, t)) {
        return Ok(TacheGog { programme: p, categorie: CategorieTache::Jeu });
    }
    // À défaut, la principale : celle de la langue du jeu d'abord, puis n'importe laquelle.
    let mut principales: Vec<&serde_json::Value> = taches.iter().filter(|t| principale(t)).collect();
    principales.sort_by_key(|t| !langue_ok(t));
    if let Some(p) = principales.iter().find_map(|t| programme_de_tache(dossier, t)) {
        return Ok(TacheGog { programme: p, categorie: CategorieTache::Principale });
    }
    Err(MotifSecours::AucuneTache)
}

/// L'entrée du registre d'UN jeu GOG (relue au lancement).
pub fn entree_gog(id: &str) -> Option<Valeurs> {
    if id.is_empty() || !id.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    valeurs(Racine::Machine, &format!(r"{CLE_GOG}\{id}"))
}

/// Le dossier du client de GOG Galaxy noté dans le registre (lecture seule), s'il existe.
pub fn dossier_galaxy() -> Option<PathBuf> {
    valeurs(Racine::Machine, r"SOFTWARE\GOG.com\GalaxyClient\paths")
        .and_then(|v| v.get("client").cloned())
        .map(|c| PathBuf::from(c.trim()))
        .filter(|d| d.is_absolute() && d.is_dir())
}

/// Les dossiers des jeux GOG installés (registre, lecture seule) : leurs programmes ne sont jamais pris pour Galaxy.
pub fn dossiers_gog() -> Vec<PathBuf> {
    sous_cles(Racine::Machine, CLE_GOG).unwrap_or_default().iter().filter_map(|(_, v)| v.get("path").map(|p| PathBuf::from(p.trim()))).filter(|p| p.is_absolute()).collect()
}

/// GOG Galaxy est-il installé (son client noté dans le registre, et présent) ?
pub fn galaxy_present() -> bool {
    valeurs(Racine::Machine, r"SOFTWARE\GOG.com\GalaxyClient\paths")
        .and_then(|v| v.get("client").cloned())
        .is_some_and(|c| Path::new(c.trim()).join("GalaxyClient.exe").is_file())
}

// ───────────────────────────── Ubisoft ─────────────────────────────

pub const CLE_UBISOFT: &str = r"SOFTWARE\Ubisoft\Launcher";

/// Les clés de désinstallation de ce PC (machine, deux vues, puis compte) : (nom de la clé, valeurs).
pub fn desinstallations() -> Vec<(String, Valeurs)> {
    const U: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
    let mut l = sous_cles(Racine::Machine, U).unwrap_or_default();
    l.extend(sous_cles(Racine::Utilisateur, U).unwrap_or_default());
    l
}

/// Les jeux Ubisoft installés. `lanceur` : les valeurs de `CLE_UBISOFT` (`None` : Ubisoft Connect absent → `None`) ;
/// `installs` : les sous-clés de `Installs` (absentes : aucun jeu) ; le nom vient de la clé de désinstallation
/// `Uplay Install <id>`, sinon du dossier (méthode de Playnite).
pub fn lire_ubisoft(lanceur: Option<Valeurs>, installs: Option<Vec<(String, Valeurs)>>, desinstall: &[(String, Valeurs)]) -> Option<Vec<JeuBoutique>> {
    lanceur?;
    let l = installs
        .unwrap_or_default()
        .iter()
        .filter_map(|(id, v)| {
            if id.is_empty() || id.len() > 12 || !id.bytes().all(|c| c.is_ascii_digit()) {
                return None;
            }
            let dossier = PathBuf::from(v.get("InstallDir")?.trim().replace('/', "\\").trim_end_matches('\\'));
            if !dossier.is_dir() {
                return None;
            }
            let nom = desinstall
                .iter()
                .find(|(n, _)| n.eq_ignore_ascii_case(&format!("Uplay Install {id}")))
                .and_then(|(_, d)| d.get("DisplayName").cloned())
                .filter(|n| !n.trim().is_empty())
                .or_else(|| dossier.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_else(|| id.clone());
            Some(jeu("uplay", id, &nom, &dossier))
        })
        .collect();
    Some(ranger(l))
}

// ───────────────────────────── Amazon ─────────────────────────────

/// La base des jeux installés d'Amazon Games (`%LOCALAPPDATA%\Amazon Games\Data\Games\Sql\GameInstallInfo.sqlite`).
pub fn base_amazon() -> Option<PathBuf> {
    let f = PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("Amazon Games").join("Data").join("Games").join("Sql").join("GameInstallInfo.sqlite");
    f.is_file().then_some(f)
}

/// Un identifiant Amazon sûr (« amzn1.adg.product.… »).
fn id_amazon_sur(s: &str) -> bool {
    !s.is_empty() && s.len() <= 128 && s.chars().all(|c| c.is_ascii_alphanumeric() || ".-_".contains(c))
}

/// Les jeux Amazon installés, lus sur une COPIE de la base (avec -wal et -shm) dans `travail`, effacée ensuite. `None` :
/// pas de base, base trop grosse (`PLAFOND_BASE`), place manquante ou base illisible.
pub fn lire_amazon(base: Option<&Path>, travail: &Path) -> Option<Vec<JeuBoutique>> {
    let base = base?;
    let dossier = base.parent()?;
    let nom = base.file_name()?.to_string_lossy().into_owned();
    let fichiers = [nom.clone(), format!("{nom}-wal"), format!("{nom}-shm")];
    let taille: u64 = fichiers.iter().filter_map(|f| std::fs::metadata(dossier.join(f)).ok()).map(|m| m.len()).sum();
    if taille > PLAFOND_BASE {
        return None;
    }
    struct Efface<'a>(&'a Path);
    impl Drop for Efface<'_> {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(self.0);
        }
    }
    std::fs::create_dir_all(travail).ok()?;
    let _efface = Efface(travail);
    if crate::jeux_pc::place_libre(travail).is_some_and(|l| l < taille + (16 << 20)) {
        return None;
    }
    for f in &fichiers {
        if dossier.join(f).is_file() {
            std::fs::copy(dossier.join(f), travail.join(f)).ok()?;
        }
    }
    let lire = || -> rusqlite::Result<Vec<(String, String, String)>> {
        let c = rusqlite::Connection::open(travail.join(&nom))?;
        let mut q = c.prepare("SELECT Id, ProductTitle, InstallDirectory FROM DbSet WHERE Installed = 1")?;
        let l = q
            .query_map([], |r| Ok((r.get::<_, Option<String>>(0)?.unwrap_or_default(), r.get::<_, Option<String>>(1)?.unwrap_or_default(), r.get::<_, Option<String>>(2)?.unwrap_or_default())))?
            .flatten()
            .collect();
        Ok(l)
    };
    let lignes = lire().ok()?;
    let l = lignes
        .iter()
        .filter(|(id, _, _)| id_amazon_sur(id))
        .filter_map(|(id, titre, d)| {
            let dossier = PathBuf::from(d.trim().replace('/', "\\"));
            dossier.is_dir().then(|| jeu("amazon", id, titre, &dossier))
        })
        .collect();
    Some(ranger(l))
}

// ───────────────────────────── Xbox ─────────────────────────────

/// Les paquets AppX du compte (registre de Windows, lecture seule).
pub const CLE_PAQUETS: &str = r"Software\Classes\Local Settings\Software\Microsoft\Windows\CurrentVersion\AppModel\Repository\Packages";

/// Le nom de famille d'un paquet (« Microsoft.MinecraftUWP_1.26.6501.0_x64__8wekyb3d8bbwe » →
/// « Microsoft.MinecraftUWP_8wekyb3d8bbwe ») : le nom, puis l'éditeur (dernier morceau).
pub fn famille_paquet(nom_complet: &str) -> Option<String> {
    let m: Vec<&str> = nom_complet.split('_').collect();
    if m.len() != 5 || m[0].is_empty() || m[4].is_empty() {
        return None;
    }
    let f = format!("{}_{}", m[0], m[4]);
    cle_sure(&f).then_some(f)
}

/// Un attribut XML simple (`nom="valeur"`) dans le premier élément `<balise …>` du texte.
fn attribut(xml: &str, balise: &str, nom: &str) -> Option<String> {
    let debut = xml.find(&format!("<{balise} ")).or_else(|| xml.find(&format!("<{balise}\n"))).or_else(|| xml.find(&format!("<{balise}\r")))?;
    let el = &xml[debut..];
    let el = &el[..el.find('>')?];
    let mut reste = el;
    while let Some(p) = reste.find(nom) {
        let avant_ok = reste[..p].ends_with(|c: char| c.is_whitespace());
        let apres = reste[p + nom.len()..].trim_start();
        if avant_ok {
            if let Some(a) = apres.strip_prefix('=') {
                let a = a.trim_start();
                let q = a.chars().next()?;
                if q == '"' || q == '\'' {
                    let v = &a[1..];
                    return Some(texte_xml(&v[..v.find(q)?]));
                }
            }
        }
        reste = &reste[p + nom.len()..];
    }
    None
}

fn texte_xml(t: &str) -> String {
    t.replace("&quot;", "\"").replace("&apos;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

/// Un fichier texte de paquet lu en entier, s'il n'est pas démesuré (1 Mo).
fn lire_petit(f: &Path) -> Option<String> {
    std::fs::metadata(f).ok().filter(|m| m.is_file() && m.len() <= 1024 * 1024)?;
    std::fs::read(f).ok().map(|o| String::from_utf8_lossy(&o).into_owned())
}

/// Un JEU du Microsoft Store / Xbox installé pour ce compte : sa famille de paquet, l'identifiant de son application
/// (`AppxManifest.xml`, `<Application Id=…>`), son nom et son dossier.
#[derive(Debug, Clone, PartialEq)]
pub struct JeuXbox {
    pub famille: String,
    pub application: String,
    pub nom: String,
    pub dossier: PathBuf,
}

/// Lit un paquet : seulement s'il a un `MicrosoftGame.config` (un jeu) et une application.
pub fn lire_paquet_xbox(nom_complet: &str, v: &Valeurs) -> Option<JeuXbox> {
    let famille = famille_paquet(nom_complet)?;
    let dossier = PathBuf::from(v.get("PackageRootFolder")?.trim());
    let config = lire_petit(&dossier.join("MicrosoftGame.config"))?;
    let manifeste = lire_petit(&dossier.join("AppxManifest.xml"))?;
    let application = attribut(&manifeste, "Application", "Id").filter(|a| cle_sure(a))?;
    let lisible = |n: &String| !n.trim().is_empty() && !n.starts_with("ms-resource:") && !n.starts_with('@');
    let nom = attribut(&config, "ShellVisuals", "DefaultDisplayName")
        .filter(lisible)
        .or_else(|| v.get("DisplayName").cloned().filter(lisible))
        .or_else(|| attribut(&config, "Identity", "Name"))
        .unwrap_or_else(|| famille.clone());
    Some(JeuXbox { famille, application, nom, dossier })
}

/// Les jeux Xbox installés (paquets `sous_cles(Utilisateur, CLE_PAQUETS)`). `None` : registre des paquets illisible.
pub fn installes_xbox(paquets: Option<Vec<(String, Valeurs)>>) -> Option<Vec<JeuXbox>> {
    let mut l: Vec<JeuXbox> = paquets?.iter().filter_map(|(n, v)| lire_paquet_xbox(n, v)).collect();
    l.sort_by(|a, b| a.famille.cmp(&b.famille));
    l.dedup_by(|a, b| a.famille == b.famille);
    Some(l)
}

pub fn lire_xbox(paquets: Option<Vec<(String, Valeurs)>>) -> Option<Vec<JeuBoutique>> {
    Some(ranger(installes_xbox(paquets)?.iter().map(|x| jeu("xboxone", &x.famille, &x.nom, &x.dossier)).collect()))
}

// ───────────────────────────── Battle.net ─────────────────────────────

/// L'identifiant `--uid=` d'une commande de désinstallation de Battle.net (« …Battle.net… --uid=wow_enus … »).
fn uid_battlenet(commande: &str) -> Option<String> {
    let bas = commande.to_lowercase();
    let debut = bas.find("battle.net")?;
    let p = bas[debut..].find("--uid=")? + debut + "--uid=".len();
    let uid: String = commande[p..].chars().take_while(|c| !c.is_whitespace() && *c != '"').collect();
    cle_sure(&uid).then_some(uid)
}

/// Les jeux Battle.net installés (clés de désinstallation, méthode de Playnite). `None` : le client Battle.net n'est
/// pas installé (aucune clé `-uid=battle.net`). Les versions d'essai (« Test », « Beta ») sont écartées.
pub fn lire_battlenet(desinstall: &[(String, Valeurs)]) -> Option<Vec<JeuBoutique>> {
    let client = desinstall.iter().any(|(_, v)| v.get("UninstallString").is_some_and(|u| u.to_lowercase().contains("uid=battle.net")));
    if !client {
        return None;
    }
    let l = desinstall
        .iter()
        .filter_map(|(_, v)| {
            let uid = uid_battlenet(v.get("UninstallString")?)?;
            if uid.eq_ignore_ascii_case("battle.net") {
                return None;
            }
            let nom = v.get("DisplayName")?.trim().to_string();
            if nom.ends_with("Test") || nom.ends_with("Beta") {
                return None;
            }
            let dossier = PathBuf::from(v.get("InstallLocation")?.trim());
            dossier.is_dir().then(|| jeu("battlenet", &uid, &nom, &dossier))
        })
        .collect();
    Some(ranger(l))
}

// ───────────────────────────── Jeux POSSÉDÉS (lot B) ─────────────────────────────
//
// Les jeux possédés mais pas installés, lus SANS connexion dans les caches que les lanceurs tiennent eux-mêmes
// (relevé du 18/11 chez Seb, en lecture seule ; ✅ Seb 18/11 : « ça me semble probable », format non documenté →
// lecture tolérante). Un cache absent, trop gros ou illisible rend `None`, jamais « zéro jeu ».

/// Le plafond d'un cache lu pour les jeux possédés (Epic : 1,9 Mo chez Seb ; Ubisoft : 36 Ko).
pub const PLAFOND_CACHE: u64 = 16 * 1024 * 1024;

/// Les boutiques dont Frogtend lit les jeux possédés sans connexion.
pub const BOUTIQUES_POSSEDES: [&str; 2] = ["epic", "uplay"];

/// Un fichier lu en entier s'il ne dépasse pas `plafond` (jamais un fichier démesuré en mémoire).
fn lire_plafonne(f: &Path, plafond: u64) -> Option<Vec<u8>> {
    let m = std::fs::metadata(f).ok()?;
    if !m.is_file() || m.len() > plafond {
        return None;
    }
    let o = std::fs::read(f).ok()?;
    (o.len() as u64 <= plafond).then_some(o)
}

/// Un jeu POSSÉDÉ, pas installé (même clé que le jeu installé de la même boutique : une seule carte).
fn possede(boutique: &str, cle: &str, nom: &str, image: Option<String>) -> JeuBoutique {
    let mut j = jeu(boutique, cle, nom, Path::new(""));
    j.installe = false;
    j.dossier = None;
    j.pret = None;
    j.image = image;
    j
}

/// Décode du base64 standard (« A-Z a-z 0-9 + / », « = » à la fin, blancs ignorés). `None` : un autre signe, ou une
/// longueur impossible.
pub fn base64(texte: &[u8]) -> Option<Vec<u8>> {
    let val = |c: u8| -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a') as u32 + 26),
            b'0'..=b'9' => Some((c - b'0') as u32 + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    };
    let mut sortie = Vec::with_capacity(texte.len() / 4 * 3);
    let (mut acc, mut n, mut fin) = (0u32, 0u8, false);
    for &c in texte {
        if c.is_ascii_whitespace() {
            continue;
        }
        if c == b'=' {
            fin = true;
            continue;
        }
        if fin {
            return None; // des signes après le « = » final
        }
        acc = (acc << 6) | val(c)?;
        n += 1;
        if n == 4 {
            sortie.extend_from_slice(&[(acc >> 16) as u8, (acc >> 8) as u8, acc as u8]);
            acc = 0;
            n = 0;
        }
    }
    match n {
        0 => {}
        2 => sortie.push((acc >> 4) as u8),
        3 => sortie.extend_from_slice(&[(acc >> 10) as u8, (acc >> 2) as u8]),
        _ => return None,
    }
    Some(sortie)
}

// ── Epic : `<AppDataPath>\Catalog\catcache.bin` ──

/// Les images d'un jeu du catalogue Epic, par ordre de préférence (la boîte en hauteur d'abord).
pub const IMAGES_EPIC: [&str; 3] = ["DieselGameBoxTall", "DieselGameBox", "Thumbnail"];

/// Les catégories du catalogue Epic qui ne sont jamais un jeu (relevé du 18/11 : extensions, bonus numériques,
/// logiciels, moteurs, greffons, versions de test).
const CATEGORIES_PAS_JEU: [&str; 6] = ["addons", "digitalextras", "software", "engines", "plugins", "testing"];

/// Un titre qui n'est pas un jeu à jouer : démo, bande-son, bêta ou test public, éditeur, artbook… (relevé du 18/11 :
/// « Q.U.B.E. 2 Soundtrack », « Mortal Shell Tech Beta », « KillingFloor2Beta », « Chivalry 2 - Public Testing »,
/// « Football Manager 2024 Pre-game editor »). Mots entiers seulement (« DEMON'S TILT » reste un jeu).
pub fn titre_pas_un_jeu(titre: &str) -> bool {
    const MOTS: [&str; 10] = ["demo", "soundtrack", "ost", "beta", "playtest", "testing", "editor", "artbook", "wallpaper", "benchmark"];
    let b = titre.to_lowercase();
    b.split(|c: char| !c.is_alphanumeric()).any(|m| MOTS.contains(&m)) || b.trim_end().ends_with("beta") || b.contains("art book") || b.contains("dedicated server")
}

/// Le titre d'un jeu du catalogue Epic, réparé. Relevé du 18/11 : `catcache.bin` est écrit en ASCII, chaque signe hors
/// ASCII y devient « ? » (33 titres sur 500 chez Seb : « Les Sims? 4 », « PAYDAY?2 », « Dragon?Age?: Inquisition -
/// ?dition Jeu de l'ann?e »). Réparé SEULEMENT quand c'est sûr : collé à une lettre des deux côtés, une espace
/// insécable perdue redevient une espace (« PAYDAY?2 », « Dragon?Age ») et « Jorel?s » prend son apostrophe ; une
/// marque perdue juste avant « : , ; ) » disparaît (« Dishonored?: ») ; « ?dition » redevient « Édition » ; « ? » seul
/// entre deux espaces redevient un tiret ; des « ? » seuls à la fin, après une espace, disparaissent (émoji perdu). Un
/// « ? » en fin de mot (suivi d'une espace ou en fin de titre) RESTE : c'est peut-être un vrai (« WHAT THE GOLF? »,
/// « Why? Simulator »). Une lettre accentuée perdue au milieu d'un mot (« ann?e ») ne se devine pas : « ? » reste.
/// Un titre de plus de 300 signes est rendu tel quel (jamais un long calcul).
pub fn reparer_titre_epic(t: &str) -> String {
    const PLAFOND_TITRE: usize = 300;
    let c: Vec<char> = t.chars().take(PLAFOND_TITRE + 1).collect();
    if c.len() > PLAFOND_TITRE {
        return t.to_string();
    }
    // Le début de la suite de « ? » qui termine le titre (calculé UNE fois) ; enlevée seulement si elle suit une espace.
    let mut fin = c.len();
    while fin > 0 && c[fin - 1] == '?' {
        fin -= 1;
    }
    let fin_perdue = fin < c.len() && fin > 0 && c[fin - 1] == ' ';
    let mot = |x: Option<char>| x.is_some_and(char::is_alphanumeric);
    let mut s = String::with_capacity(t.len());
    for i in 0..c.len() {
        if c[i] != '?' {
            s.push(c[i]);
            continue;
        }
        let (av, ap) = (i.checked_sub(1).map(|k| c[k]), c.get(i + 1).copied());
        if i >= fin && fin_perdue {
            // « RAWMEN: Food Fighter Arena ?? » : des signes perdus (émoji).
        } else if !mot(av) && c.get(i + 1..i + 7).is_some_and(|x| x.iter().copied().eq("dition".chars())) {
            s.push('É');
        } else if mot(av) && mot(ap) {
            let ap = ap.unwrap_or(' ');
            if ap.is_ascii_digit() || (av.is_some_and(char::is_lowercase) && ap.is_uppercase()) {
                s.push(' ');
            } else if ap == 's' && !mot(c.get(i + 2).copied()) {
                s.push('\'');
            } else {
                s.push('?'); // une lettre accentuée perdue : jamais devinée
            }
        } else if av == Some(' ') && ap == Some(' ') {
            s.push('–');
        } else if mot(av) && matches!(ap, Some(':' | ',' | ';' | ')')) {
            // Une marque perdue (« Dishonored?: », « Star Wars?: »).
        } else {
            s.push('?');
        }
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Un jeu du catalogue Epic, s'il en est un : catégorie `games`, pas un DLC (`mainGameItem.id` rempli), pas une
/// extension ni un outil, une version Windows avec son `appId` (= l'AppName du manifeste : la clé du jeu installé).
fn jeu_du_catalogue_epic(e: &serde_json::Value) -> Option<JeuBoutique> {
    let cats: Vec<&str> = e["categories"].as_array()?.iter().filter_map(|c| c["path"].as_str()).collect();
    let pas_jeu = |c: &&str| CATEGORIES_PAS_JEU.iter().any(|x| *c == *x || c.starts_with(&format!("{x}/")));
    if !cats.contains(&"games") || cats.iter().any(pas_jeu) {
        return None;
    }
    if e["mainGameItem"]["id"].as_str().is_some_and(|m| !m.trim().is_empty()) || e["mainGameItem"].as_str().is_some_and(|m| !m.trim().is_empty()) {
        return None; // un DLC (il dépend d'un autre jeu)
    }
    let titre = e["title"].as_str()?.trim();
    if titre.is_empty() || titre_pas_un_jeu(titre) {
        return None;
    }
    // Une version pour Windows (sans plateforme dite : on la garde) ; jamais une version iOS ou Android seule.
    let windows = |r: &serde_json::Value| r["platform"].as_array().is_none_or(|p| p.is_empty() || p.iter().any(|x| matches!(x.as_str(), Some("Windows" | "Win32"))));
    let app = e["releaseInfo"].as_array()?.iter().filter(|r| windows(r)).find_map(|r| r["appId"].as_str().filter(|a| cle_sure(a)))?;
    let images = e["keyImages"].as_array();
    let image = IMAGES_EPIC.iter().find_map(|t| {
        images?.iter().filter(|i| i["type"].as_str() == Some(t)).find_map(|i| i["url"].as_str().filter(|u| crate::boutiques::image_autorisee(u)).map(String::from))
    });
    let repare = reparer_titre_epic(titre);
    let mut j = possede("epic", app, &repare, image);
    j.titre_douteux = repare != titre || repare.contains('?');
    Some(j)
}

/// Les jeux possédés d'après le contenu de `catcache.bin` (du JSON encodé en base64 : une liste d'objets du catalogue).
/// `None` : fichier illisible (pas du base64, pas une liste JSON).
pub fn lire_catalogue_epic(octets: &[u8]) -> Option<Vec<JeuBoutique>> {
    let json = base64(octets)?;
    let v: serde_json::Value = serde_json::from_slice(&json).ok()?;
    Some(ranger(v.as_array()?.iter().filter_map(jeu_du_catalogue_epic).collect()))
}

/// Le cache du catalogue de l'Epic Games Launcher (`<AppDataPath>\Catalog\catcache.bin`), s'il existe.
pub fn catalogue_epic() -> Option<PathBuf> {
    let f = crate::boutiques::dossier_donnees_epic()?.join("Catalog").join("catcache.bin");
    f.is_file().then_some(f)
}

/// Les jeux Epic possédés (lecture seule, taille plafonnée). `None` : pas de cache, trop gros, ou illisible.
pub fn possedes_epic(fichier: Option<&Path>) -> Option<Vec<JeuBoutique>> {
    lire_catalogue_epic(&lire_plafonne(fichier?, PLAFOND_CACHE)?)
}

/// Les espaces de noms des JEUX Epic possédés (mêmes règles que `jeu_du_catalogue_epic` : jamais un DLC, une
/// extension, un outil) : un jeu offert dont l'espace y figure est déjà dans le compte (Seb 19/11 : Knightica restait
/// « Obtenir »). `None` : catalogue absent ou illisible (rien n'est alors supposé).
pub fn espaces_lus_epic(octets: &[u8]) -> Option<std::collections::HashSet<String>> {
    let v: serde_json::Value = serde_json::from_slice(&base64(octets)?).ok()?;
    Some(
        v.as_array()?
            .iter()
            .filter(|e| jeu_du_catalogue_epic(e).is_some())
            .filter_map(|e| e["namespace"].as_str().filter(|n| !n.is_empty()).map(String::from))
            .collect(),
    )
}

/// `espaces_lus_epic` sur le catalogue de ce PC (lecture seule, taille plafonnée).
pub fn espaces_possedes_epic(fichier: Option<&Path>) -> Option<std::collections::HashSet<String>> {
    espaces_lus_epic(&lire_plafonne(fichier?, PLAFOND_CACHE)?)
}

// ── Ubisoft : `<cache>\configuration\configurations` et `<cache>\ownership\<compte>` ──
//
// Méthode (relevé du 18/11 chez Seb, et celle de Playnite, relue sans recopier son code) : `configurations` est une
// suite de messages protobuf {1 : numéro du produit, 3 : sa configuration en YAML} ; Playnite prend pour « possédé »
// tout produit qui a un `start_game`. Ce cache garde AUSSI des produits que le compte n'a pas (chez Seb : 23
// produits, 4 jeux). La possession se lit donc dans `ownership\<compte>` : 4 octets, une signature de 256 octets, la
// longueur (u32 petit-boutiste), puis un protobuf {1 : un produit possédé (1 : son numéro, …), 2 : un numéro possédé}.
// Le produit y porte aussi sa clé de produit : Frogtend ne lit QUE les numéros, jamais la clé.

/// Le début de l'adresse des images d'Ubisoft Connect (CDN public, celui du lanceur).
pub const IMAGES_UBISOFT: &str = "https://ubistatic3-a.akamaihd.net/orbit/uplay_launcher_3_0/assets/";

/// Un champ protobuf : un entier, ou des octets (texte, message, liste compacte).
enum Champ<'a> {
    Entier(u64),
    Octets(&'a [u8]),
}

fn varint(o: &[u8], i: &mut usize) -> Option<u64> {
    let mut r = 0u64;
    for k in 0..10 {
        let c = *o.get(*i)?;
        *i += 1;
        r |= ((c & 0x7f) as u64) << (7 * k);
        if c < 0x80 {
            return Some(r);
        }
    }
    None
}

/// Les champs d'un message protobuf. `None` : message abîmé (type inconnu, longueur qui déborde).
fn champs(o: &[u8]) -> Option<Vec<(u64, Champ<'_>)>> {
    let (mut i, mut l) = (0usize, vec![]);
    while i < o.len() {
        let k = varint(o, &mut i)?;
        let n = k >> 3;
        if n == 0 {
            return None;
        }
        let v = match k & 7 {
            0 => Champ::Entier(varint(o, &mut i)?),
            1 | 5 => {
                let fin = i.checked_add(if k & 7 == 1 { 8 } else { 4 })?;
                let v = Champ::Octets(o.get(i..fin)?);
                i = fin;
                v
            }
            2 => {
                let lg = usize::try_from(varint(o, &mut i)?).ok()?;
                let fin = i.checked_add(lg)?;
                let v = Champ::Octets(o.get(i..fin)?);
                i = fin;
                v
            }
            _ => return None,
        };
        l.push((n, v));
    }
    Some(l)
}

/// Les numéros des produits possédés, lus dans un fichier `ownership` (jamais la clé de produit). `None` : fichier
/// abîmé (longueur annoncée fausse, protobuf illisible) ou sans aucun produit.
pub fn lire_possession_ubisoft(o: &[u8]) -> Option<std::collections::BTreeSet<u64>> {
    let longueur = u32::from_le_bytes(o.get(260..264)?.try_into().ok()?) as usize;
    let donnees = o.get(264..)?;
    if longueur == 0 || donnees.len() != longueur {
        return None;
    }
    let mut l = std::collections::BTreeSet::new();
    for (n, v) in champs(donnees)? {
        match (n, v) {
            (1, Champ::Octets(p)) => {
                // Seulement le champ 1 (le numéro) : les autres (dont la clé de produit) ne sont jamais lus.
                if let Some((_, Champ::Entier(id))) = champs(p)?.into_iter().find(|(m, _)| *m == 1) {
                    l.insert(id);
                }
            }
            (2, Champ::Entier(id)) => {
                l.insert(id);
            }
            // Une liste « compacte » (forme protobuf possible) : une suite d'entiers.
            (2, Champ::Octets(p)) => {
                let mut i = 0;
                while i < p.len() {
                    l.insert(varint(p, &mut i)?);
                }
            }
            _ => {}
        }
    }
    (!l.is_empty()).then_some(l)
}

/// Un texte YAML simple (le nom d'un produit peut être un nombre).
fn yaml_texte(y: &yaml_rust2::Yaml) -> Option<String> {
    match y {
        yaml_rust2::Yaml::String(s) => Some(s.clone()),
        yaml_rust2::Yaml::Integer(i) => Some(i.to_string()),
        yaml_rust2::Yaml::Real(r) => Some(r.clone()),
        _ => None,
    }
}

/// Un « oui » YAML (`yes`, `true`).
fn yaml_oui(y: &yaml_rust2::Yaml) -> bool {
    match y {
        yaml_rust2::Yaml::Boolean(b) => *b,
        yaml_rust2::Yaml::String(s) => matches!(s.to_lowercase().as_str(), "yes" | "true" | "on"),
        _ => false,
    }
}

/// Un nom de fichier d'image du lanceur (« da1733d5….jpg ») : sûr dans une adresse.
fn image_ubisoft_sure(f: &str) -> bool {
    let b = f.to_lowercase();
    f.len() <= 128 && cle_sure(f) && [".jpg", ".jpeg", ".png", ".webp"].iter().any(|x| b.ends_with(x))
}

/// Les jeux possédés d'après `configurations` et les numéros possédés : un produit possédé qui a un `start_game`, qui
/// n'est ni un contenu additionnel (`is_ulc`, ou cité dans les `addons` d'un autre), ni vendu par une autre boutique
/// (`third_party_platform`). Le nom et l'image passent par `localizations.default` quand ce sont des clés (« l1 »,
/// « THUMBIMAGE »). `None` : fichier illisible.
pub fn lire_configurations_ubisoft(o: &[u8], possedes: &std::collections::BTreeSet<u64>) -> Option<Vec<JeuBoutique>> {
    let mut produits = vec![];
    for (n, v) in champs(o)? {
        let (1, Champ::Octets(p)) = (n, v) else { continue };
        let (mut id, mut yaml) = (None, None);
        for (m, w) in champs(p)? {
            match (m, w) {
                (1, Champ::Entier(x)) => id = Some(x),
                (3, Champ::Octets(y)) => yaml = Some(String::from_utf8_lossy(y).into_owned()),
                _ => {}
            }
        }
        // Une configuration YAML illisible : ce produit seulement est laissé.
        if let (Some(id), Some(y)) = (id, yaml) {
            if let Some(d) = yaml_rust2::YamlLoader::load_from_str(&y).ok().and_then(|d| d.into_iter().next()) {
                produits.push((id, d));
            }
        }
    }
    if produits.is_empty() {
        return None;
    }
    let addons: std::collections::BTreeSet<i64> = produits.iter().flat_map(|(_, d)| d["root"]["addons"].as_vec().into_iter().flatten().filter_map(|a| a["id"].as_i64())).collect();
    let l = produits
        .iter()
        .filter(|(id, d)| {
            let r = &d["root"];
            possedes.contains(id) && r["start_game"].as_hash().is_some() && !yaml_oui(&r["is_ulc"]) && r["third_party_platform"].is_badvalue() && !i64::try_from(*id).is_ok_and(|i| addons.contains(&i))
        })
        .filter_map(|(id, d)| {
            let r = &d["root"];
            let loc = &d["localizations"]["default"];
            let traduit = |cle: &yaml_rust2::Yaml| yaml_texte(cle).map(|c| loc[c.as_str()].as_str().map(String::from).unwrap_or(c)).filter(|t| !t.trim().is_empty());
            let nom = traduit(&r["name"]).or_else(|| traduit(&r["installer"]["game_identifier"]))?;
            let image = traduit(&r["thumb_image"]).filter(|f| image_ubisoft_sure(f)).map(|f| format!("{IMAGES_UBISOFT}{f}"));
            Some(possede("uplay", &id.to_string(), &nom, image))
        })
        .collect();
    Some(ranger(l))
}

/// Le dossier `cache` d'Ubisoft Connect : à côté du lanceur (`InstallDir` du registre, relevé du 18/11), sinon dans
/// `%LOCALAPPDATA%\Ubisoft Game Launcher` (Playnite) ; seulement s'il a son `configuration\configurations`.
pub fn cache_ubisoft(lanceur: Option<&Valeurs>) -> Option<PathBuf> {
    let mut l = vec![];
    if let Some(d) = lanceur.and_then(|v| v.get("InstallDir")).map(|d| d.trim().replace('/', "\\")).filter(|d| !d.is_empty()) {
        l.push(PathBuf::from(d.trim_end_matches('\\')).join("cache"));
    }
    if let Some(a) = std::env::var_os("LOCALAPPDATA") {
        l.push(PathBuf::from(a).join("Ubisoft Game Launcher").join("cache"));
    }
    l.into_iter().find(|c| c.join("configuration").join("configurations").is_file())
}

/// Les jeux Ubisoft possédés (lecture seule). La possession est celle du DERNIER compte connecté (le fichier
/// `ownership` le plus récent). `None` : pas de cache, trop gros, ou illisible.
pub fn possedes_ubisoft(cache: Option<&Path>) -> Option<Vec<JeuBoutique>> {
    let cache = cache?;
    let config = lire_plafonne(&cache.join("configuration").join("configurations"), PLAFOND_CACHE)?;
    let compte = std::fs::read_dir(cache.join("ownership"))
        .ok()?
        .flatten()
        .filter_map(|e| {
            let m = e.metadata().ok().filter(|m| m.is_file() && m.len() <= 4 * 1024 * 1024)?;
            Some((m.modified().ok(), e.path()))
        })
        .max()?
        .1;
    let possedes = lire_possession_ubisoft(&lire_plafonne(&compte, 4 * 1024 * 1024)?)?;
    lire_configurations_ubisoft(&config, &possedes)
}

/// Lit les jeux possédés des boutiques demandées (seulement celles de `BOUTIQUES_POSSEDES`).
pub fn lire_possedes(boutiques: &[&str]) -> Lecture {
    BOUTIQUES_POSSEDES
        .iter()
        .filter(|b| boutiques.contains(b))
        .map(|b| {
            let l = match *b {
                "epic" => possedes_epic(catalogue_epic().as_deref()),
                "uplay" => possedes_ubisoft(cache_ubisoft(valeurs(Racine::Machine, CLE_UBISOFT).as_ref()).as_deref()),
                _ => None,
            };
            (*b, l)
        })
        .collect()
}

/// Les cartes d'une boutique : ses jeux installés, PLUS ses jeux possédés qui ne le sont pas. Même clé = une seule
/// carte, installée (elle reçoit l'image du catalogue si elle n'en avait pas). `None` : jeux installés illisibles —
/// rien ne bouge, même si le catalogue se lit (l'état « installé » des cartes serait faux).
pub fn fusion_possedes(installes: Option<Vec<JeuBoutique>>, possedes: Option<Vec<JeuBoutique>>) -> Option<Vec<JeuBoutique>> {
    let mut l = installes?;
    for p in possedes.unwrap_or_default() {
        match l.iter_mut().find(|j| j.id == p.id) {
            Some(j) => {
                if j.image.is_none() {
                    j.image = p.image;
                }
            }
            None => l.push(p),
        }
    }
    Some(ranger(l))
}

// ───────────────────────────── Tout le PC ─────────────────────────────

/// Ce qu'une lecture du PC a donné, boutique par boutique (`None` : lanceur absent ou illisible).
pub type Lecture = Vec<(&'static str, Option<Vec<JeuBoutique>>)>;

/// Lit TOUTES les boutiques de ce PC (lecture seule ; `travail` : dossier des copies de bases, effacé ensuite).
pub fn lire_tout(travail: &Path) -> Lecture {
    lire_seulement(&BOUTIQUES_PC, travail)
}

/// Lit les boutiques demandées seulement.
pub fn lire_seulement(boutiques: &[&str], travail: &Path) -> Lecture {
    let desinstall = if boutiques.iter().any(|b| *b == "uplay" || *b == "battlenet") { desinstallations() } else { vec![] };
    BOUTIQUES_PC
        .iter()
        .filter(|b| boutiques.contains(b))
        .map(|b| {
            let l = match *b {
                "steam" => lire_steam(crate::boutiques::dossier_steam().as_deref()),
                "epic" => lire_epic(crate::boutiques::installes_epic_du_pc()),
                "gog" => lire_gog(sous_cles(Racine::Machine, CLE_GOG)),
                "uplay" => lire_ubisoft(valeurs(Racine::Machine, CLE_UBISOFT), sous_cles(Racine::Machine, &format!(r"{CLE_UBISOFT}\Installs")), &desinstall),
                "amazon" => lire_amazon(base_amazon().as_deref(), &travail.join("amazon")),
                "xboxone" => lire_xbox(sous_cles(Racine::Utilisateur, CLE_PAQUETS)),
                "battlenet" => lire_battlenet(&desinstall),
                _ => None,
            };
            (*b, l)
        })
        .collect()
}

// ───────────────────────────── Jouer, installer ─────────────────────────────

/// Une adresse de protocole est-elle déclarée à Windows (le lanceur qui la sert est installé) ?
pub fn protocole_present(p: &str) -> bool {
    valeurs(Racine::Utilisateur, &format!(r"Software\Classes\{p}")).is_some() || valeurs(Racine::Machine, &format!(r"SOFTWARE\Classes\{p}")).is_some()
}

/// Le lanceur d'une boutique est-il sur ce PC (pour Jouer et Installer) ? Xbox : Windows lui-même ; GOG : ses jeux
/// se lancent sans Galaxy (Installer demande Galaxy, voir `adresse`).
pub fn lanceur_present(b: &str) -> bool {
    match b {
        "steam" => crate::boutiques::dossier_steam().is_some(),
        "epic" => protocole_present("com.epicgames.launcher"),
        "gog" | "xboxone" => true,
        "uplay" => protocole_present("uplay"),
        "amazon" => protocole_present("amazon-games"),
        "battlenet" => lire_battlenet(&desinstallations()).is_some(),
        _ => false,
    }
}

/// Ce que la personne demande à la boutique.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Jouer,
    Installer,
}

/// L'adresse officielle qui demande à la boutique de lancer ou d'installer un jeu, la clé VALIDÉE d'abord. `None` :
/// pas d'adresse prouvée pour cette boutique et cette action (Battle.net ; GOG sans Galaxy pour installer ; Epic pour
/// jouer, qui a besoin de son manifeste : `boutiques::adresse_epic`).
/// Sources : Steam (Valve Developer Community, « Steam browser protocol ») ; Epic (raccourci posé par Epic, relevé du
/// 21/10 ; installation : forme de Playnite `apps/<AppName>?action=install`) ; Ubisoft (`uplay://launch/<id>`, Playnite ;
/// `uplay://install/<id>`, avis des experts du 18/11, NON relu dans une source) ; Amazon (`amazon-games://play|install/
/// <id>`, Playnite) ; GOG (`goggalaxy://openGameView/<clé>`, relevé dans Galaxy le 02/10) ; Xbox
/// (`shell:AppsFolder\<famille>!<application>` : identifiant de modèle d'application, documentation de Microsoft ;
/// `ms-windows-store://pdp/?PFN=<famille>` : « Launch the Microsoft Store app », Microsoft).
pub fn adresse(boutique: &str, cle: &str, action: Action, application_xbox: Option<&str>) -> Option<String> {
    match (boutique, action) {
        ("steam", _) if !cle.is_empty() && cle.len() <= 12 && cle.bytes().all(|c| c.is_ascii_digit()) => {
            Some(if action == Action::Jouer { format!("steam://rungameid/{cle}") } else { format!("steam://install/{cle}") })
        }
        ("epic", Action::Installer) if cle_sure(cle) => Some(format!("com.epicgames.launcher://apps/{cle}?action=install")),
        ("uplay", _) if !cle.is_empty() && cle.len() <= 12 && cle.bytes().all(|c| c.is_ascii_digit()) => {
            Some(if action == Action::Jouer { format!("uplay://launch/{cle}") } else { format!("uplay://install/{cle}") })
        }
        ("amazon", _) if id_amazon_sur(cle) => Some(if action == Action::Jouer { format!("amazon-games://play/{cle}") } else { format!("amazon-games://install/{cle}") }),
        ("gog", Action::Installer) if !cle.is_empty() && cle.len() <= 20 && cle.bytes().all(|c| c.is_ascii_digit()) => Some(format!("goggalaxy://openGameView/gog_{cle}")),
        ("xboxone", Action::Jouer) if famille_sure(cle) => application_xbox.filter(|a| cle_sure(a)).map(|a| format!(r"shell:AppsFolder\{cle}!{a}")),
        ("xboxone", Action::Installer) if famille_sure(cle) => Some(format!("ms-windows-store://pdp/?PFN={cle}")),
        _ => None,
    }
}

/// Une famille de paquet sûre (« Nom_Éditeur »).
fn famille_sure(f: &str) -> bool {
    cle_sure(f) && f.contains('_')
}

/// L'application d'un jeu Xbox de ce PC (relue au lancement).
pub fn application_xbox(famille: &str) -> Option<String> {
    installes_xbox(sous_cles(Racine::Utilisateur, CLE_PAQUETS))?.into_iter().find(|x| x.famille.eq_ignore_ascii_case(famille)).map(|x| x.application)
}

/// Le message quand le lanceur d'une boutique manque (jamais un ▶ Jouer qui échoue en silence).
pub fn message_lanceur_absent(boutique: &str) -> String {
    match boutique {
        "battlenet" => "Frogtend ne sait pas encore lancer un jeu Battle.net : ouvre-le depuis Battle.net.".into(),
        "gog" => "Ce jeu s'installe par GOG Galaxy, absent de ce PC.".into(),
        b => format!("Ce jeu a besoin du lanceur {}, absent de ce PC.", nom_boutique(b)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valeurs(l: &[(&str, &str)]) -> Valeurs {
        l.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
    }

    #[test]
    fn les_noms_perdent_leurs_marques_et_les_cles_douteuses_sont_refusees() {
        assert_eq!(sans_marques("Horizon Zero Dawn™  Complete Edition"), "Horizon Zero Dawn Complete Edition");
        assert_eq!(sans_marques("Far Cry® 3"), "Far Cry 3");
        assert!(cle_sure("Fortnite") && !cle_sure("a/b") && !cle_sure("") && !cle_sure("x y"));
        assert_eq!(nom_boutique("uplay"), "Ubisoft Connect");
        assert_eq!(nom_boutique("inconnue"), "sa boutique");
        let j = jeu("gog", "1", "Jeu™", Path::new(r"D:\J"));
        assert_eq!((j.id.as_str(), j.nom.as_str(), j.installe, j.plateforme.as_str()), ("gog_1", "Jeu", true, "gog"));
        assert_eq!(jeu("steam", "620", "Portal 2", Path::new("D:\\P")).id, "620", "Steam : l'appid seul (comme son import)");
        let r = ranger(vec![jeu("gog", "2", "b", Path::new("x")), jeu("gog", "1", "A", Path::new("x")), jeu("gog", "1", "A", Path::new("x"))]);
        assert_eq!(r.iter().map(|j| j.id.as_str()).collect::<Vec<_>>(), ["gog_1", "gog_2"], "trié, chaque jeu une fois");
    }

    #[test]
    fn steam_sans_cle_d_api_lit_ses_jeux_installes_et_ecarte_les_outils() {
        let d = tempfile::tempdir().unwrap();
        let s = d.path().join("Steam");
        std::fs::create_dir_all(s.join("steamapps/common/Portal 2")).unwrap();
        std::fs::create_dir_all(s.join("steamapps/common/Steamworks Shared")).unwrap();
        let m = |id: &str, nom: &str, dir: &str| format!("\"AppState\"\n{{\n\t\"appid\"\t\t\"{id}\"\n\t\"name\"\t\t\"{nom}\"\n\t\"StateFlags\"\t\t\"4\"\n\t\"installdir\"\t\t\"{dir}\"\n}}\n");
        std::fs::write(s.join("steamapps/appmanifest_620.acf"), m("620", "Portal 2", "Portal 2")).unwrap();
        std::fs::write(s.join("steamapps/appmanifest_228980.acf"), m("228980", "Steamworks Common Redistributables", "Steamworks Shared")).unwrap();
        let l = lire_steam(Some(&s)).unwrap();
        assert_eq!(l.iter().map(|j| (j.id.as_str(), j.nom.as_str())).collect::<Vec<_>>(), [("620", "Portal 2")]);
        assert_eq!(l[0].dossier.as_deref(), Some(s.join("steamapps").join("common").join("Portal 2").to_string_lossy().as_ref()));
        assert!(lire_steam(None).is_none(), "Steam absent : None, pas zéro jeu");
        assert!(lire_steam(Some(&d.path().join("absent"))).is_none(), "bibliothèque illisible : None");
        assert!(est_outil_steam("1", "SteamVR") && est_outil_steam("2", "Proton 9.0") && !est_outil_steam("3", "Portal"));
    }

    #[test]
    fn la_liste_steam_du_compte_recoit_les_jeux_installes_qu_elle_n_a_pas() {
        let mut api = jeu("steam", "400", "Portal", Path::new("x"));
        api.installe = false;
        api.dossier = None;
        api.minutes = 30;
        let mut ancien = jeu("steam", "500", "L4D", Path::new("x"));
        ancien.installe = true;
        let locaux = vec![jeu("steam", "400", "Portal", Path::new(r"D:\S\Portal")), jeu("steam", "620", "Portal 2", Path::new(r"D:\S\P2"))];
        let l = fusion_steam(vec![api, ancien], &locaux);
        assert_eq!(l.iter().map(|j| (j.id.as_str(), j.installe)).collect::<Vec<_>>(), [("400", true), ("500", false), ("620", true)]);
        assert_eq!((l[0].minutes, l[0].dossier.as_deref()), (30, Some(r"D:\S\Portal")), "le temps du compte est gardé");
        assert_eq!(fusion_steam(vec![], &locaux).len(), 2, "sans clé d'API : les jeux installés");
    }

    #[test]
    fn la_liste_gog_du_compte_recoit_les_installes_et_ce_que_galaxy_sait() {
        let du_compte = |id: &str, nom: &str| {
            let mut j = jeu("gog", id, nom, Path::new("x"));
            j.installe = false;
            j.dossier = None;
            j.pret = None;
            j.image = Some(format!("https://images-1.gog-statics.com/1e8a5a1fb0e6d4d4bfa4{id}_392.jpg"));
            j
        };
        let compte = vec![du_compte("1", "Alpha"), du_compte("2", "Beta"), du_compte("2", "Beta")];
        // Installés sur ce PC (registre) : le jeu 2 du compte, et le jeu 3 qui n'est pas dans la liste (autre compte).
        let installes = vec![jeu("gog", "2", "Beta", Path::new(r"D:\G\Beta")), jeu("gog", "3", "Gamma", Path::new(r"D:\G\Gamma"))];
        // GOG Galaxy connaît le jeu 1 : ses copies, son temps, ses succès et sa jaquette verticale sont repris.
        let mut g1 = du_compte("1", "Alpha™ (Galaxy)");
        g1.releases = vec!["gog_1".into(), "steam_620".into()];
        g1.minutes = 90;
        g1.succes = Some((3, 10));
        g1.image = Some("https://images.gog.com/abc_glx_vertical_cover.webp".into());
        let mut g2 = du_compte("2", "Beta");
        g2.image = Some("https://exemple.com/piege.jpg".into());
        let l = fusion_gog(compte, &installes, &[g1, g2]);
        assert_eq!(l.iter().map(|j| (j.id.as_str(), j.installe)).collect::<Vec<_>>(), [("gog_1", false), ("gog_2", true), ("gog_3", true)], "une carte par jeu");
        assert_eq!(l[0].nom, "Alpha (Galaxy)", "le titre de Galaxy d'abord, sans ™");
        assert_eq!(l[1].nom, "Beta", "sans Galaxy : celui du compte");
        let mut sans_titre = du_compte("2", "x");
        sans_titre.nom = "gog_2".into();
        assert_eq!(fusion_gog(vec![du_compte("2", "Beta")], &[], &[sans_titre])[0].nom, "Beta", "la clé de Galaxy n'est pas un titre");
        assert_eq!((l[0].minutes, l[0].succes, l[0].releases.len()), (90, Some((3, 10)), 2));
        assert_eq!(l[0].image.as_deref(), Some("https://images.gog.com/abc_glx_vertical_cover.webp"));
        assert_eq!(l[1].dossier.as_deref(), Some(r"D:\G\Beta"));
        assert!(l[1].image.as_deref().is_some_and(|i| i.contains("gog-statics")), "une image de Galaxy non autorisée ne remplace rien");
        // Sans compte (liste vide) : les jeux installés seuls, comme avant le lot C4a.
        assert_eq!(fusion_gog(vec![], &installes, &[]).len(), 2);
    }

    #[test]
    fn epic_absent_rend_none_et_ses_jeux_gardent_l_appname() {
        assert!(lire_epic(None).is_none());
        let e = crate::boutiques::JeuEpic { app: "Fortnite".into(), namespace: "fn".into(), item: "it".into(), nom: "Fortnite".into(), dossier: PathBuf::from(r"D:\F") };
        let l = lire_epic(Some(vec![e])).unwrap();
        assert_eq!(l[0].id, "epic_Fortnite", "la clé de GOG Galaxy : epic_<AppName>");
    }

    #[test]
    fn gog_se_lit_dans_le_registre_sans_galaxy_et_ecarte_les_dlc() {
        let d = tempfile::tempdir().unwrap();
        let hzd = d.path().join("Horizon Zero Dawn");
        let dlc = d.path().join("DLC");
        let autre = d.path().join("Autre");
        for x in [&hzd, &dlc, &autre] {
            std::fs::create_dir_all(x).unwrap();
        }
        std::fs::write(hzd.join("HorizonZeroDawn.exe"), b"MZ").unwrap();
        std::fs::write(autre.join("goggame-3.info"), r#"{"gameId":"3","rootGameId":"1209025141"}"#).unwrap();
        let h = hzd.to_string_lossy().to_string();
        // La forme réelle (relevé du 18/11, Venkman).
        let entrees = vec![
            ("1209025141".to_string(), valeurs(&[("gameID", "1209025141"), ("gameName", "Horizon Zero Dawn™ Complete Edition"), ("path", &h), ("exe", &format!(r"{h}\HorizonZeroDawn.exe")), ("workingDir", &h), ("launchParam", ""), ("dependsOn", "")])),
            ("2".to_string(), valeurs(&[("gameID", "2"), ("gameName", "Un DLC"), ("path", &dlc.to_string_lossy()), ("dependsOn", "1209025141")])),
            ("3".to_string(), valeurs(&[("gameID", "3"), ("gameName", "DLC par info"), ("path", &autre.to_string_lossy())])),
            ("4".to_string(), valeurs(&[("gameID", "4"), ("gameName", "Désinstallé"), ("path", r"Z:\absent")])),
            ("x".to_string(), valeurs(&[("gameID", "../x"), ("path", &h)])),
        ];
        let l = lire_gog(Some(entrees.clone())).unwrap();
        assert_eq!(l.iter().map(|j| (j.id.as_str(), j.nom.as_str())).collect::<Vec<_>>(), [("gog_1209025141", "Horizon Zero Dawn Complete Edition")]);
        assert!(lire_gog(None).is_none());
        assert_eq!(lire_gog(Some(vec![])).unwrap(), vec![], "clé présente et vide : zéro jeu (vrai)");
        let p = programme_gog(&entrees[0].1).unwrap();
        assert_eq!(p.exe, hzd.join("HorizonZeroDawn.exe"));
        // Un programme HORS du dossier du jeu n'est jamais lancé.
        let mut v = entrees[0].1.clone();
        v.insert("exe".into(), r"C:\Windows\System32\cmd.exe".into());
        assert_eq!(programme_gog(&v), None);
        // Un dossier de travail hors du jeu : le dossier du jeu à la place.
        let mut v = entrees[0].1.clone();
        v.insert("workingDir".into(), std::env::temp_dir().to_string_lossy().into_owned());
        assert_eq!(programme_gog(&v).unwrap().dossier_travail, hzd);
        assert_eq!(entree_gog("1/../2"), None);
    }

    /// Un dossier de jeu GOG simulé : ses programmes (fichiers vides) et son `goggame-<cle>.info`.
    fn jeu_gog_simule(cle: &str, info: &str, programmes: &[&str]) -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        for p in programmes {
            let f = d.path().join(p);
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(f, b"MZ").unwrap();
        }
        std::fs::write(d.path().join(format!("goggame-{cle}.info")), info).unwrap();
        d
    }

    /// Une forme réelle (`goggame-<id>.info` relevé le 25/11 sur Venkman, lecture seule) dont on change les tâches.
    fn info_avec_taches(cle: &str, taches: serde_json::Value) -> String {
        serde_json::json!({"buildId": "1", "clientId": "2", "gameId": cle, "language": "French", "languages": ["fr-FR"], "name": "Jeu",
            "osBitness": ["64"], "playTasks": taches, "rootGameId": cle, "version": 1})
        .to_string()
    }

    #[test]
    fn cyberpunk_se_lance_par_sa_tache_game_jamais_par_redprelauncher() {
        let info = include_str!("../fixtures/gog/cyberpunk-goggame-1423049311.info");
        let d = jeu_gog_simule("1423049311", info, &["REDprelauncher.exe", r"bin\x64\Cyberpunk2077.exe"]);
        let t = tache_jeu_gog(d.path(), "1423049311").unwrap();
        assert_eq!(t.categorie, CategorieTache::Jeu);
        assert_eq!(t.programme.exe, d.path().join(r"bin\x64\Cyberpunk2077.exe"), "la tâche « game » cachée, pas la principale (lanceur)");
        assert_eq!(t.programme.dossier_travail, d.path(), "sans workingDir : le dossier du jeu");
        assert_eq!(t.programme.arguments, "");
        // Le programme du jeu absent : la principale (une FileTask du dossier), comme Heroic et Playnite.
        let d = jeu_gog_simule("1423049311", info, &["REDprelauncher.exe"]);
        let t = tache_jeu_gog(d.path(), "1423049311").unwrap();
        assert_eq!((t.categorie, t.programme.exe.clone()), (CategorieTache::Principale, d.path().join("REDprelauncher.exe")));
        // Aucun des deux : le secours.
        let d = jeu_gog_simule("1423049311", info, &[]);
        assert_eq!(tache_jeu_gog(d.path(), "1423049311"), Err(MotifSecours::AucuneTache), "exe absent");
    }

    #[test]
    fn horizon_zero_dawn_se_lance_par_sa_principale_qui_est_le_jeu() {
        let info = include_str!("../fixtures/gog/hzd-goggame-1209025141.info");
        let d = jeu_gog_simule("1209025141", info, &["HorizonZeroDawn.exe"]);
        let t = tache_jeu_gog(d.path(), "1209025141").unwrap();
        assert_eq!((t.categorie, t.programme.exe), (CategorieTache::Jeu, d.path().join("HorizonZeroDawn.exe")));
    }

    #[test]
    fn un_jeu_unreal_sans_tache_game_lance_son_petit_programme_principal() {
        let info = info_avec_taches("7", serde_json::json!([
            {"category": "launcher", "isPrimary": true, "languages": ["*"], "name": "Jeu", "path": "Jeu.exe", "type": "FileTask", "arguments": "-nosplash", "workingDir": "Binaires"},
            {"category": "document", "name": "Manuel", "path": "manuel.pdf", "type": "FileTask"},
            {"category": "other", "isPrimary": false, "link": "https://www.gog.com", "name": "Site", "type": "URLTask"}
        ]));
        let d = jeu_gog_simule("7", &info, &["Jeu.exe", r"Binaires\Win64\Jeu-Win64-Shipping.exe"]);
        let t = tache_jeu_gog(d.path(), "7").unwrap();
        assert_eq!(t.categorie, CategorieTache::Principale);
        assert_eq!(t.programme.exe, d.path().join("Jeu.exe"));
        assert_eq!(t.programme.arguments, "-nosplash", "les arguments en chaîne, tels quels");
        assert_eq!(t.programme.dossier_travail, d.path().join("Binaires"));
        assert_eq!(CategorieTache::Principale.mot(), "principale");
        assert_eq!(CategorieTache::Jeu.mot(), "game");
    }

    #[test]
    fn la_tache_game_est_celle_de_la_langue_du_jeu_et_en_64_bits() {
        let taches = serde_json::json!([
            {"category": "game", "languages": ["en-US"], "path": "en.exe", "type": "FileTask"},
            {"category": "game", "languages": ["fr-FR"], "path": "fr32.exe", "osBitness": ["32"], "type": "FileTask"},
            {"category": "game", "languages": ["FR-fr"], "path": "fr.exe", "osBitness": ["32", "64"], "type": "FileTask"},
            {"category": "game", "isPrimary": true, "languages": ["de-DE"], "path": "de.exe", "type": "FileTask"}
        ]);
        let d = jeu_gog_simule("8", &info_avec_taches("8", taches), &["en.exe", "fr32.exe", "fr.exe", "de.exe"]);
        assert_eq!(tache_jeu_gog(d.path(), "8").unwrap().programme.exe, d.path().join("fr.exe"), "la langue installée (sans casse), 64 bits ; la principale d'une autre langue écartée");
        // Plusieurs « game » valables : la principale préférée.
        let taches = serde_json::json!([
            {"category": "game", "languages": ["*"], "path": "a.exe", "type": "FileTask"},
            {"category": "game", "isPrimary": true, "path": "b.exe", "type": "FileTask"}
        ]);
        let d = jeu_gog_simule("8", &info_avec_taches("8", taches), &["a.exe", "b.exe"]);
        assert_eq!(tache_jeu_gog(d.path(), "8").unwrap().programme.exe, d.path().join("b.exe"));
        // Sans principale : la première.
        let taches = serde_json::json!([{"category": "game", "path": "a.exe", "type": "FileTask"}, {"category": "game", "path": "b.exe", "type": "FileTask"}]);
        let d = jeu_gog_simule("8", &info_avec_taches("8", taches), &["a.exe", "b.exe"]);
        assert_eq!(tache_jeu_gog(d.path(), "8").unwrap().programme.exe, d.path().join("a.exe"));
    }

    #[test]
    fn une_tache_game_32_bits_n_est_prise_qu_a_defaut_de_64_bits() {
        // Seulement 32 bits : acceptée (Windows 64 bits la lance), comme tâche « game ».
        let taches = serde_json::json!([{"category": "game", "osBitness": ["32"], "path": "jeu32.exe", "type": "FileTask"}]);
        let d = jeu_gog_simule("9", &info_avec_taches("9", taches), &["jeu32.exe"]);
        let t = tache_jeu_gog(d.path(), "9").unwrap();
        assert_eq!((t.categorie, t.programme.exe), (CategorieTache::Jeu, d.path().join("jeu32.exe")));
        // Le 64 bits passe avant, même si la 32 bits est la principale et vient d'abord.
        let taches = serde_json::json!([
            {"category": "game", "isPrimary": true, "osBitness": ["32"], "path": "jeu32.exe", "type": "FileTask"},
            {"category": "game", "osBitness": ["64"], "path": "jeu64.exe", "type": "FileTask"}
        ]);
        let d = jeu_gog_simule("9", &info_avec_taches("9", taches.clone()), &["jeu32.exe", "jeu64.exe"]);
        assert_eq!(tache_jeu_gog(d.path(), "9").unwrap().programme.exe, d.path().join("jeu64.exe"));
        // Le 64 bits absent du disque : la 32 bits.
        let d = jeu_gog_simule("9", &info_avec_taches("9", taches), &["jeu32.exe"]);
        assert_eq!(tache_jeu_gog(d.path(), "9").unwrap().programme.exe, d.path().join("jeu32.exe"));
    }

    #[test]
    fn le_repli_sur_la_principale_prefere_la_langue_du_jeu() {
        let taches = serde_json::json!([
            {"category": "launcher", "isPrimary": true, "languages": ["de-DE"], "path": "de.exe", "type": "FileTask"},
            {"category": "launcher", "isPrimary": true, "languages": ["fr-FR"], "path": "fr.exe", "type": "FileTask"}
        ]);
        let d = jeu_gog_simule("19", &info_avec_taches("19", taches.clone()), &["de.exe", "fr.exe"]);
        let t = tache_jeu_gog(d.path(), "19").unwrap();
        assert_eq!((t.categorie, t.programme.exe), (CategorieTache::Principale, d.path().join("fr.exe")));
        // Aucune dans la langue : n'importe quelle principale.
        let d = jeu_gog_simule("19", &info_avec_taches("19", taches), &["de.exe"]);
        assert_eq!(tache_jeu_gog(d.path(), "19").unwrap().programme.exe, d.path().join("de.exe"));
    }

    #[test]
    fn un_dlc_a_cote_ne_change_pas_la_tache_du_jeu() {
        let info = include_str!("../fixtures/gog/hzd-goggame-1209025141.info");
        let d = jeu_gog_simule("1209025141", info, &["HorizonZeroDawn.exe", "DLC.exe"]);
        // Le DLC (rootGameId = le jeu) a son propre fichier, jamais lu pour lancer le jeu.
        let dlc = serde_json::json!({"gameId": "2", "rootGameId": "1209025141", "playTasks": [{"category": "game", "isPrimary": true, "path": "DLC.exe", "type": "FileTask"}]});
        std::fs::write(d.path().join("goggame-2.info"), dlc.to_string()).unwrap();
        assert_eq!(tache_jeu_gog(d.path(), "1209025141").unwrap().programme.exe, d.path().join("HorizonZeroDawn.exe"));
        // Le fichier du DLC lu sous la clé du DLC : il parle d'un autre jeu racine, refusé.
        assert_eq!(tache_jeu_gog(d.path(), "2"), Err(MotifSecours::AutreJeu));
    }

    #[test]
    fn un_fichier_goggame_douteux_mene_au_secours_sans_panique() {
        let d = jeu_gog_simule("10", "{ \"gameId\": \"10\", ", &["Jeu.exe"]);
        assert_eq!(tache_jeu_gog(d.path(), "10"), Err(MotifSecours::Illisible), "JSON abîmé");
        let d = jeu_gog_simule("10", "[1, 2]", &[]);
        assert_eq!(tache_jeu_gog(d.path(), "10"), Err(MotifSecours::Illisible), "pas un objet");
        let d = tempfile::tempdir().unwrap();
        assert_eq!(tache_jeu_gog(d.path(), "10"), Err(MotifSecours::SansFichier));
        assert_eq!(tache_jeu_gog(d.path(), "1/../2"), Err(MotifSecours::SansFichier), "une clé qui n'est pas un numéro");
        // Trop gros (fichier creux : jamais lu en entier).
        let f = std::fs::File::create(d.path().join("goggame-10.info")).unwrap();
        f.set_len(PLAFOND_INFO_GOG + 1).unwrap();
        drop(f);
        assert_eq!(tache_jeu_gog(d.path(), "10"), Err(MotifSecours::TropGros));
        // Un autre jeu.
        let taches = serde_json::json!([{"category": "game", "path": "Jeu.exe", "type": "FileTask"}]);
        let d = jeu_gog_simule("10", &info_avec_taches("11", taches.clone()), &["Jeu.exe"]);
        assert_eq!(tache_jeu_gog(d.path(), "10"), Err(MotifSecours::AutreJeu), "gameId différent");
        let mut v: serde_json::Value = serde_json::from_str(&info_avec_taches("10", taches)).unwrap();
        v["rootGameId"] = "12".into();
        let d = jeu_gog_simule("10", &v.to_string(), &["Jeu.exe"]);
        assert_eq!(tache_jeu_gog(d.path(), "10"), Err(MotifSecours::AutreJeu), "rootGameId différent");
        v["rootGameId"] = serde_json::Value::Null;
        v.as_object_mut().unwrap().remove("rootGameId");
        let d = jeu_gog_simule("10", &v.to_string(), &["Jeu.exe"]);
        assert!(tache_jeu_gog(d.path(), "10").is_ok(), "sans rootGameId : accepté");
        for m in [MotifSecours::SansFichier, MotifSecours::TropGros, MotifSecours::Illisible, MotifSecours::AutreJeu, MotifSecours::AucuneTache] {
            assert!(!m.en_mots().contains('\\') && !m.en_mots().is_empty());
        }
    }

    #[test]
    fn une_tache_au_chemin_douteux_ou_aux_arguments_en_tableau_est_refusee() {
        let d = tempfile::tempdir().unwrap();
        let jeu = d.path().join("Jeu");
        std::fs::create_dir_all(jeu.join("bin")).unwrap();
        std::fs::write(jeu.join("bin").join("Jeu.exe"), b"MZ").unwrap();
        std::fs::write(d.path().join("Dehors.exe"), b"MZ").unwrap();
        std::fs::write(jeu.join("lisezmoi.txt"), b"x").unwrap();
        let dehors = d.path().join("Dehors.exe").to_string_lossy().into_owned();
        let essai = |tache: serde_json::Value| {
            std::fs::write(jeu.join("goggame-20.info"), info_avec_taches("20", serde_json::json!([tache]))).unwrap();
            tache_jeu_gog(&jeu, "20")
        };
        let base = || serde_json::json!({"category": "game", "isPrimary": true, "path": r"bin\Jeu.exe", "type": "FileTask"});
        assert!(essai(base()).is_ok(), "la tâche de référence passe");
        let avec = |cle: &str, val: serde_json::Value| {
            let mut t = base();
            t[cle] = val;
            essai(t)
        };
        assert_eq!(avec("path", r"..\Dehors.exe".into()), Err(MotifSecours::AucuneTache), "« .. »");
        assert_eq!(avec("path", r"bin\..\..\Dehors.exe".into()), Err(MotifSecours::AucuneTache), "« .. » au milieu");
        std::fs::write(jeu.join("Jeu..exe"), b"MZ").unwrap();
        assert_eq!(avec("path", "Jeu..exe".into()).unwrap().programme.exe, jeu.join("Jeu..exe"), "« .. » dans un NOM de fichier : accepté");
        assert_eq!(avec("path", dehors.clone().into()), Err(MotifSecours::AucuneTache), "absolu");
        assert_eq!(avec("path", r"\Dehors.exe".into()), Err(MotifSecours::AucuneTache), "depuis la racine");
        assert_eq!(avec("path", r"<?APPLICATION_DATA_LOCAL?>\Jeu.exe".into()), Err(MotifSecours::AucuneTache), "« <? »");
        assert_eq!(avec("path", "lisezmoi.txt".into()), Err(MotifSecours::AucuneTache), "pas un .exe");
        assert_eq!(avec("path", "absent.exe".into()), Err(MotifSecours::AucuneTache), "exe absent");
        assert_eq!(avec("type", "URLTask".into()), Err(MotifSecours::AucuneTache), "URLTask seule");
        assert_eq!(avec("arguments", serde_json::json!(["-a", "-b"])), Err(MotifSecours::AucuneTache), "arguments en tableau");
        assert_eq!(avec("arguments", "<?GAME_DIR?>".into()), Err(MotifSecours::AucuneTache), "jeton de Galaxy dans les arguments");
        assert_eq!(avec("arguments", " -skipintro ".into()).unwrap().programme.arguments, "-skipintro");
        assert_eq!(avec("workingDir", "..".into()), Err(MotifSecours::AucuneTache), "dossier de travail hors du jeu");
        assert_eq!(avec("workingDir", d.path().to_string_lossy().into_owned().into()), Err(MotifSecours::AucuneTache), "dossier de travail absolu");
        assert_eq!(avec("workingDir", "absent".into()), Err(MotifSecours::AucuneTache), "dossier de travail absent");
        assert_eq!(avec("workingDir", "bin".into()).unwrap().programme.dossier_travail, jeu.join("bin"));
        assert_eq!(avec("workingDir", "".into()).unwrap().programme.dossier_travail, jeu);
        // Une URLTask principale ne sert jamais de recours.
        let url = serde_json::json!({"category": "launcher", "isPrimary": true, "link": "https://exemple.com", "type": "URLTask"});
        assert_eq!(essai(url), Err(MotifSecours::AucuneTache));
    }

    /// Une jonction entre le dossier du jeu et le programme : refusée (le programme serait ailleurs). Windows seulement.
    #[cfg(windows)]
    #[test]
    fn un_programme_derriere_une_jonction_est_refuse() {
        let d = tempfile::tempdir().unwrap();
        let (jeu, ailleurs) = (d.path().join("Jeu"), d.path().join("Ailleurs"));
        std::fs::create_dir_all(&jeu).unwrap();
        std::fs::create_dir_all(&ailleurs).unwrap();
        std::fs::write(ailleurs.join("Jeu.exe"), b"MZ").unwrap();
        let lien = jeu.join("bin");
        let ok = crate::lancement::outil_sans_fenetre("cmd").args(["/C", "mklink", "/J"]).arg(&lien).arg(&ailleurs).output().is_ok_and(|o| o.status.success());
        if !ok {
            eprintln!("jonction impossible ici : essai sauté");
            return;
        }
        std::fs::write(jeu.join("goggame-21.info"), info_avec_taches("21", serde_json::json!([{"category": "game", "path": r"bin\Jeu.exe", "type": "FileTask"}]))).unwrap();
        let r = tache_jeu_gog(&jeu, "21");
        let _ = std::fs::remove_dir(&lien);
        assert_eq!(r, Err(MotifSecours::AucuneTache));
    }

    #[test]
    fn ubisoft_lit_ses_installations_avec_des_barres_obliques_et_son_nom_de_desinstallation() {
        let d = tempfile::tempdir().unwrap();
        let fc = d.path().join("Far Cry 3");
        std::fs::create_dir_all(&fc).unwrap();
        // Ubisoft écrit « D:/Games/Far Cry 3/ » (des « / »).
        let avec_obliques = format!("{}/", fc.to_string_lossy().replace('\\', "/"));
        let installs = vec![("46".to_string(), valeurs(&[("InstallDir", &avec_obliques)])), ("47".to_string(), valeurs(&[("InstallDir", "Z:/absent/")]))];
        let desinstall = vec![("Uplay Install 46".to_string(), valeurs(&[("DisplayName", "Far Cry® 3")]))];
        let lanceur = Some(valeurs(&[("InstallDir", r"D:\Ubisoft\")]));
        let l = lire_ubisoft(lanceur.clone(), Some(installs.clone()), &desinstall).unwrap();
        assert_eq!(l.iter().map(|j| (j.id.as_str(), j.nom.as_str())).collect::<Vec<_>>(), [("uplay_46", "Far Cry 3")]);
        assert!(!l[0].dossier.as_deref().unwrap().contains('/'), "dossier normalisé");
        assert_eq!(lire_ubisoft(lanceur.clone(), Some(installs), &[]).unwrap()[0].nom, "Far Cry 3", "sans clé de désinstallation : le dossier");
        assert!(lire_ubisoft(None, None, &[]).is_none(), "Ubisoft Connect absent");
        assert_eq!(lire_ubisoft(lanceur, None, &[]).unwrap(), vec![], "Installs vide (chez Seb) : zéro jeu");
    }

    #[test]
    fn amazon_se_lit_sur_une_copie_effacee_et_respecte_le_plafond() {
        // Données de TEST (Amazon Games n'est pas installé chez Seb) : la table de Playnite, `DbSet`.
        let d = tempfile::tempdir().unwrap();
        let sql = d.path().join("Sql");
        std::fs::create_dir_all(&sql).unwrap();
        let jeu1 = d.path().join("Jeux").join("Rogue");
        std::fs::create_dir_all(&jeu1).unwrap();
        let base = sql.join("GameInstallInfo.sqlite");
        let c = rusqlite::Connection::open(&base).unwrap();
        c.execute_batch("CREATE TABLE DbSet(Id TEXT, ProductTitle TEXT, InstallDirectory TEXT, Installed INTEGER);").unwrap();
        c.execute("INSERT INTO DbSet VALUES ('amzn1.adg.product.1111', 'Rogue Legacy™', ?1, 1), ('amzn1.adg.product.2', 'Pas installé', ?1, 0), ('x/../y', 'Douteux', ?1, 1), ('amzn1.adg.product.3', 'Disparu', 'Z:\\absent', 1)", [jeu1.to_string_lossy()]).unwrap();
        drop(c);
        let travail = d.path().join("copie");
        let l = lire_amazon(Some(&base), &travail).unwrap();
        assert_eq!(l.iter().map(|j| (j.id.as_str(), j.nom.as_str())).collect::<Vec<_>>(), [("amazon_amzn1.adg.product.1111", "Rogue Legacy")]);
        assert!(!travail.exists(), "la copie ne reste pas");
        assert!(base.is_file(), "l'original n'est pas touché");
        assert!(lire_amazon(None, &travail).is_none());
        // Au-delà du plafond : rien n'est copié, rien n'est lu.
        let gros = sql.join("GameInstallInfo.sqlite-wal");
        let f = std::fs::File::create(&gros).unwrap();
        f.set_len(PLAFOND_BASE + 1).unwrap();
        drop(f);
        assert!(lire_amazon(Some(&base), &travail).is_none());
        assert!(!travail.exists());
        // Une base illisible : None (jamais « zéro jeu »).
        std::fs::remove_file(&gros).unwrap();
        std::fs::write(&base, b"pas une base").unwrap();
        assert!(lire_amazon(Some(&base), &travail).is_none());
    }

    #[test]
    fn xbox_garde_les_paquets_qui_sont_des_jeux() {
        let d = tempfile::tempdir().unwrap();
        let mc = d.path().join("Microsoft.MinecraftUWP_1.26.6501.0_x64__8wekyb3d8bbwe");
        let appli = d.path().join("Microsoft.WindowsCalculator_11.0_x64__8wekyb3d8bbwe");
        std::fs::create_dir_all(&mc).unwrap();
        std::fs::create_dir_all(&appli).unwrap();
        // La forme réelle (Minecraft, relevé du 18/11), réduite.
        std::fs::write(mc.join("MicrosoftGame.config"), "<?xml version=\"1.0\"?>\n<Game configVersion=\"1\">\n  <Identity Name=\"Microsoft.MinecraftUWP\"\n            Version=\"1.26.6501.0\" />\n  <ShellVisuals DefaultDisplayName=\"Minecraft for Windows\"\n      PublisherDisplayName=\"Microsoft Studios\"/>\n</Game>").unwrap();
        std::fs::write(mc.join("AppxManifest.xml"), "<Package><Applications><Application Id=\"Game\" Executable=\"GameLaunchHelper.exe\" EntryPoint=\"Windows.FullTrustApplication\"></Application></Applications></Package>").unwrap();
        std::fs::write(appli.join("AppxManifest.xml"), "<Application Id=\"App\"></Application>").unwrap();
        let paquets = vec![
            ("Microsoft.MinecraftUWP_1.26.6501.0_x64__8wekyb3d8bbwe".to_string(), valeurs(&[("PackageRootFolder", &mc.to_string_lossy()), ("DisplayName", "Minecraft for Windows")])),
            ("Microsoft.WindowsCalculator_11.0_x64__8wekyb3d8bbwe".to_string(), valeurs(&[("PackageRootFolder", &appli.to_string_lossy())])),
        ];
        let l = installes_xbox(Some(paquets.clone())).unwrap();
        assert_eq!(l.len(), 1, "une application sans MicrosoftGame.config n'est pas un jeu");
        assert_eq!((l[0].famille.as_str(), l[0].application.as_str(), l[0].nom.as_str()), ("Microsoft.MinecraftUWP_8wekyb3d8bbwe", "Game", "Minecraft for Windows"));
        assert_eq!(lire_xbox(Some(paquets)).unwrap()[0].id, "xboxone_Microsoft.MinecraftUWP_8wekyb3d8bbwe");
        assert!(lire_xbox(None).is_none());
        assert_eq!(famille_paquet("pas_un_nom"), None);
        // Un nom « ms-resource: » n'est jamais montré.
        assert_eq!(attribut("<ShellVisuals DefaultDisplayName=\"ms-resource:Nom\"/>", "ShellVisuals", "DefaultDisplayName").as_deref(), Some("ms-resource:Nom"));
        assert_eq!(attribut("<Identity xName=\"a\" Name='B &amp; C'/>", "Identity", "Name").as_deref(), Some("B & C"));
    }

    #[test]
    fn battlenet_se_lit_dans_les_cles_de_desinstallation() {
        // Données de TEST (Battle.net n'est pas installé chez Seb), d'après la méthode de Playnite.
        let d = tempfile::tempdir().unwrap();
        let wow = d.path().join("World of Warcraft");
        std::fs::create_dir_all(&wow).unwrap();
        let w = wow.to_string_lossy().to_string();
        let mut l = vec![
            ("Battle.net".to_string(), valeurs(&[("DisplayName", "Battle.net"), ("UninstallString", r#""C:\Program Files (x86)\Battle.net\Battle.net Launcher.exe" --lang=frFR --uid=battle.net"#)])),
            ("World of Warcraft".to_string(), valeurs(&[("DisplayName", "World of Warcraft"), ("InstallLocation", &w), ("UninstallString", r#""C:\ProgramData\Battle.net\Agent\Blizzard Uninstaller.exe" --lang=frFR --uid=wow_frfr --displayname="World of Warcraft""#)])),
            ("Beta".to_string(), valeurs(&[("DisplayName", "Overwatch Beta"), ("InstallLocation", &w), ("UninstallString", "Battle.net --uid=pro_beta ")])),
            ("Autre".to_string(), valeurs(&[("DisplayName", "Autre"), ("InstallLocation", &w), ("UninstallString", "unins000.exe")])),
        ];
        let r = lire_battlenet(&l).unwrap();
        assert_eq!(r.iter().map(|j| (j.id.as_str(), j.nom.as_str())).collect::<Vec<_>>(), [("battlenet_wow_frfr", "World of Warcraft")]);
        l.remove(0);
        assert!(lire_battlenet(&l).is_none(), "sans le client : None");
        assert_eq!(uid_battlenet("Battle.net.exe --uid=x/../y"), None);
    }

    #[test]
    fn les_adresses_des_lanceurs_sont_validees() {
        assert_eq!(adresse("steam", "620", Action::Jouer, None).as_deref(), Some("steam://rungameid/620"));
        assert_eq!(adresse("steam", "620", Action::Installer, None).as_deref(), Some("steam://install/620"));
        assert_eq!(adresse("steam", "6;20", Action::Jouer, None), None);
        assert_eq!(adresse("uplay", "46", Action::Jouer, None).as_deref(), Some("uplay://launch/46"));
        assert_eq!(adresse("uplay", "46", Action::Installer, None).as_deref(), Some("uplay://install/46"));
        assert_eq!(adresse("uplay", "4/6", Action::Jouer, None), None);
        assert_eq!(adresse("amazon", "amzn1.adg.product.1", Action::Jouer, None).as_deref(), Some("amazon-games://play/amzn1.adg.product.1"));
        assert_eq!(adresse("amazon", "amzn1.adg.product.1", Action::Installer, None).as_deref(), Some("amazon-games://install/amzn1.adg.product.1"));
        assert_eq!(adresse("epic", "Fortnite", Action::Installer, None).as_deref(), Some("com.epicgames.launcher://apps/Fortnite?action=install"));
        assert_eq!(adresse("epic", "Fortnite", Action::Jouer, None), None, "jouer : par le manifeste (adresse_epic)");
        assert_eq!(adresse("gog", "1209025141", Action::Installer, None).as_deref(), Some("goggalaxy://openGameView/gog_1209025141"));
        assert_eq!(adresse("gog", "1209025141", Action::Jouer, None), None, "jouer GOG : Galaxy ou le programme");
        assert_eq!(adresse("xboxone", "Microsoft.MinecraftUWP_8wekyb3d8bbwe", Action::Jouer, Some("Game")).as_deref(), Some(r"shell:AppsFolder\Microsoft.MinecraftUWP_8wekyb3d8bbwe!Game"));
        assert_eq!(adresse("xboxone", "Microsoft.MinecraftUWP_8wekyb3d8bbwe", Action::Installer, None).as_deref(), Some("ms-windows-store://pdp/?PFN=Microsoft.MinecraftUWP_8wekyb3d8bbwe"));
        assert_eq!(adresse("xboxone", "x&y_z", Action::Installer, None), None);
        assert_eq!(adresse("battlenet", "wow", Action::Jouer, None), None, "Battle.net : aucune adresse prouvée");
        assert!(message_lanceur_absent("uplay").contains("Ubisoft Connect, absent de ce PC"));
    }

    #[test]
    fn le_base64_se_decode_et_refuse_le_reste() {
        assert_eq!(base64(b"SGVsbG8=").unwrap(), b"Hello");
        assert_eq!(base64(b"SGVsbG8h").unwrap(), b"Hello!");
        assert_eq!(base64(b"SGVs\r\nbG8=").unwrap(), b"Hello", "les retours à la ligne sont ignorés");
        assert_eq!(base64(b"SGk").unwrap(), b"Hi", "sans le « = » final");
        assert_eq!(base64(b"").unwrap(), b"");
        assert!(base64(b"SGVs*G8=").is_none(), "un signe étranger");
        assert!(base64(b"S").is_none(), "une longueur impossible");
        assert!(base64(b"SG==SG").is_none(), "des signes après la fin");
    }

    /// Un catalogue Epic inventé, de la forme relevée le 18/11 chez Seb (`catcache.bin` : une liste JSON en base64).
    fn catalogue_epic_invente() -> Vec<u8> {
        let entree = |id: &str, titre: &str, cats: &[&str], main: &str, app: &str, plat: &[&str], images: &[(&str, &str)]| {
            serde_json::json!({
                "id": id, "namespace": "ns", "title": titre,
                "categories": cats.iter().map(|c| serde_json::json!({"name": "", "path": c})).collect::<Vec<_>>(),
                "mainGameItem": {"namespace": if main.is_empty() { "" } else { "ns" }, "id": main},
                "releaseInfo": [{"appId": app, "compatibleApps": [], "platform": plat}],
                "keyImages": images.iter().map(|(t, u)| serde_json::json!({"type": t, "url": u})).collect::<Vec<_>>(),
                "customAttributes": {"FolderName": {"type": "STRING", "value": "x"}}, "dlcItemList": []
            })
        };
        let tall = "https://cdn1.epicgames.com/item/ns/TerraScape_1200x1600-abc";
        let l = serde_json::json!([
            entree("e6eb", "TerraScape", &["games", "applications"], "", "6ef17d9f63c143ea88a4a4bcbcf1e91c", &["Windows"], &[("DieselGameBox", "https://cdn1.epicgames.com/item/ns/large"), ("DieselGameBoxTall", tall)]),
            entree("1", "Borderlands 3™", &["public", "games", "applications"], "", "Catnip", &["Windows", "Mac"], &[("Thumbnail", "https://cdn1.epicgames.com/item/ns/thumb")]),
            entree("2", "Un DLC", &["games", "applications"], "4fe75bbc", "DlcApp", &["Windows"], &[]),
            entree("3", "Unreal Engine", &["engines", "engines/ue5"], "", "UE_5.7", &["Windows"], &[]),
            entree("4", "Twinmotion Audience", &["audience", "public"], "", "Twin", &["Windows"], &[]),
            entree("5", "Death Stranding Content", &["public", "games", "applications", "digitalextras"], "", "DSC", &["Windows"], &[]),
            entree("6", "Q.U.B.E. 2 Soundtrack", &["games", "applications"], "", "QubeOst", &["Windows"], &[]),
            entree("7", "Mortal Shell Tech Beta", &["games", "applications"], "", "MSBeta", &["Windows"], &[]),
            entree("8", "KillingFloor2Beta", &["games", "applications"], "", "KF2B", &["Windows"], &[]),
            entree("9", "Teacup", &["games", "applications"], "", "TeacupIos", &["iOS"], &[]),
            entree("10", "DEMON'S TILT", &["games", "applications"], "", "Tilt", &["Windows"], &[("DieselGameBoxTall", "https://autre-hote.com/x.png")]),
            entree("11", "Douteux", &["games", "applications"], "", "a/../b", &["Windows"], &[]),
            entree("12", "Ark Map Pack", &["games", "addons"], "", "ArkMap", &["Windows"], &[]),
        ]);
        // Comme Epic : du base64 SANS retour à la ligne.
        let t = serde_json::to_vec(&l).unwrap();
        const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut s = vec![];
        for c in t.chunks(3) {
            let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
            for k in 0..4 {
                s.push(if k <= c.len() { A[(n >> (18 - 6 * k) & 63) as usize] } else { b'=' });
            }
        }
        s
    }

    #[test]
    fn epic_dit_les_espaces_des_jeux_possedes_pour_reconnaitre_un_jeu_offert() {
        let e = espaces_lus_epic(&catalogue_epic_invente()).unwrap();
        assert_eq!(e.into_iter().collect::<Vec<_>>(), ["ns"], "les jeux du catalogue inventé sont tous dans « ns »");
        assert!(espaces_lus_epic(b"pas du base64 !").is_none(), "illisible : rien n'est supposé");
        assert!(espaces_lus_epic(b"W10=").unwrap().is_empty(), "une liste vide : aucun jeu possédé");
        assert!(espaces_possedes_epic(None).is_none());
    }

    #[test]
    fn epic_lit_ses_jeux_possedes_dans_le_catalogue_et_ecarte_le_reste() {
        let l = lire_catalogue_epic(&catalogue_epic_invente()).unwrap();
        assert_eq!(
            l.iter().map(|j| (j.id.as_str(), j.nom.as_str())).collect::<Vec<_>>(),
            [("epic_Catnip", "Borderlands 3"), ("epic_Tilt", "DEMON'S TILT"), ("epic_6ef17d9f63c143ea88a4a4bcbcf1e91c", "TerraScape")],
            "DLC, moteur, audience, bonus, bande-son, bêtas, iOS seul, clé douteuse et extension écartés"
        );
        assert!(l.iter().all(|j| !j.installe && j.dossier.is_none() && j.plateforme == "epic"));
        let t = l.iter().find(|j| j.nom == "TerraScape").unwrap();
        assert_eq!(t.image.as_deref(), Some("https://cdn1.epicgames.com/item/ns/TerraScape_1200x1600-abc"), "la boîte en hauteur d'abord");
        assert_eq!(l[0].image.as_deref(), Some("https://cdn1.epicgames.com/item/ns/thumb"), "sinon la vignette");
        assert_eq!(l[1].image, None, "un hôte non autorisé : pas d'image");
        // Un fichier abîmé : None, jamais « zéro jeu ».
        assert!(lire_catalogue_epic(b"pas du base64 !").is_none());
        assert!(lire_catalogue_epic(b"e30=").is_none(), "un objet JSON, pas une liste");
        let mut coupe = catalogue_epic_invente();
        coupe.truncate(coupe.len() / 2 / 4 * 4);
        assert!(lire_catalogue_epic(&coupe).is_none(), "un JSON coupé");
        assert_eq!(lire_catalogue_epic(b"W10=").unwrap(), vec![], "[] : une liste vide est vraie");
        assert!(titre_pas_un_jeu("Football Manager 2024 Pre-game editor") && titre_pas_un_jeu("Chivalry 2 - Public Testing") && !titre_pas_un_jeu("Ghostrunner"));
    }

    #[test]
    fn les_titres_abimes_du_catalogue_epic_sont_repares_sans_rien_inventer() {
        // Les titres RÉELS du relevé du 18/11 (Epic écrit « ? » à la place de chaque signe hors ASCII).
        for (abime, repare) in [
            // Un « ? » en fin de mot reste : il est peut-être vrai.
            ("WHAT THE GOLF?", "WHAT THE GOLF?"),
            ("Why? Simulator", "Why? Simulator"),
            ("Les Sims? 4", "Les Sims? 4"),
            ("Rocket League?", "Rocket League?"),
            ("PAYDAY?2", "PAYDAY 2"),
            ("Dishonored?: Death of the Outsider?", "Dishonored: Death of the Outsider?"),
            ("LEGO? Star Wars?: The Skywalker Saga", "LEGO? Star Wars: The Skywalker Saga"),
            ("Dragon?Age?: Inquisition - ?dition Jeu de l'ann?e", "Dragon Age: Inquisition - Édition Jeu de l'ann?e"),
            ("Monument?Valley?II", "Monument Valley II"),
            ("Trine?2: Complete Story", "Trine 2: Complete Story"),
            ("Jorel?s Brother and The Most Important Game of the Galaxy", "Jorel's Brother and The Most Important Game of the Galaxy"),
            ("Firestone ? Idle Clicker Online RPG", "Firestone – Idle Clicker Online RPG"),
            ("RAWMEN: Food Fighter Arena ??", "RAWMEN: Food Fighter Arena"),
            ("Ogu et la For?t secr?te", "Ogu et la For?t secr?te"),
            ("Le D?tective du c?t? Obscur", "Le D?tective du c?t? Obscur"),
            ("Borderlands 3", "Borderlands 3"),
        ] {
            assert_eq!(reparer_titre_epic(abime), repare, "{abime}");
        }
        // Plafond : au-delà de 300 signes, le titre brut (jamais un long calcul).
        let long = "a?".repeat(200);
        assert_eq!(reparer_titre_epic(&long), long);
        let limite = format!("{}?2", "a".repeat(298));
        assert_eq!(reparer_titre_epic(&limite), format!("{} 2", "a".repeat(298)), "300 signes : encore réparé");
        // Un titre abîmé ou réparé est marqué « douteux » (la carte déjà rangée garde le sien).
        let l = lire_catalogue_epic(&catalogue_epic_invente()).unwrap();
        assert!(l.iter().all(|j| !j.titre_douteux), "aucun titre abîmé dans ce catalogue");
        let entree = |t: &str| serde_json::json!({"title": t, "categories": [{"path": "games"}], "mainGameItem": {"id": ""}, "releaseInfo": [{"appId": "A", "platform": ["Windows"]}]});
        assert!(jeu_du_catalogue_epic(&entree("PAYDAY?2")).unwrap().titre_douteux, "réparé");
        assert!(jeu_du_catalogue_epic(&entree("WHAT THE GOLF?")).unwrap().titre_douteux, "un « ? » reste");
        assert!(!jeu_du_catalogue_epic(&entree("Borderlands 3")).unwrap().titre_douteux);
    }

    #[test]
    fn le_catalogue_epic_se_lit_sur_le_disque_avec_un_plafond() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("catcache.bin");
        std::fs::write(&f, catalogue_epic_invente()).unwrap();
        assert_eq!(possedes_epic(Some(&f)).unwrap().len(), 3);
        assert!(possedes_epic(None).is_none());
        assert!(possedes_epic(Some(&d.path().join("absent"))).is_none());
        // Au-delà du plafond : rien n'est lu (fichier creux, aucune place prise).
        let gros = d.path().join("gros.bin");
        std::fs::File::create(&gros).unwrap().set_len(PLAFOND_CACHE + 1).unwrap();
        assert!(possedes_epic(Some(&gros)).is_none());
    }

    /// Un message protobuf : (numéro du champ, entier ou octets).
    fn pb(champs: &[(u64, Result<u64, Vec<u8>>)]) -> Vec<u8> {
        let v = |mut x: u64, o: &mut Vec<u8>| loop {
            let b = (x & 0x7f) as u8;
            x >>= 7;
            if x == 0 {
                o.push(b);
                break;
            }
            o.push(b | 0x80);
        };
        let mut o = vec![];
        for (n, c) in champs {
            match c {
                Ok(e) => {
                    v(n << 3, &mut o);
                    v(*e, &mut o);
                }
                Err(b) => {
                    v(n << 3 | 2, &mut o);
                    v(b.len() as u64, &mut o);
                    o.extend_from_slice(b);
                }
            }
        }
        o
    }

    /// Un fichier `ownership` inventé, de la forme relevée chez Seb (en-tête, signature, longueur, protobuf ; la clé de
    /// produit est INVENTÉE).
    fn possession_inventee(ids_produits: &[u64], ids: &[u64]) -> Vec<u8> {
        let mut c: Vec<(u64, Result<u64, Vec<u8>>)> = ids_produits.iter().map(|i| (1, Err(pb(&[(1, Ok(*i)), (2, Ok(*i)), (4, Err(b"AAAA-BBBB-CCCC-DDDD".to_vec()))])))).collect();
        c.extend(ids.iter().map(|i| (2, Ok(*i))));
        let d = pb(&c);
        let mut o = vec![0, 1, 0, 0];
        o.extend(std::iter::repeat_n(0x5a, 256));
        o.extend((d.len() as u32).to_le_bytes());
        o.extend(d);
        o
    }

    fn configuration(id: u64, yaml: &str) -> (u64, Result<u64, Vec<u8>>) {
        (1, Err(pb(&[(1, Ok(id)), (2, Ok(id)), (3, Err(yaml.as_bytes().to_vec()))])))
    }

    #[test]
    fn ubisoft_lit_ses_jeux_possedes_et_seulement_ceux_du_compte() {
        let possedes = lire_possession_ubisoft(&possession_inventee(&[54, 95], &[54, 95, 116, 3499])).unwrap();
        assert_eq!(possedes.into_iter().collect::<Vec<_>>(), [54, 95, 116, 3499]);
        // La forme réelle (Assassin's Creed III, relevé du 18/11), réduite : nom et image par `localizations`.
        let ac3 = "version: 2.0\nroot:\n  name: l1\n  thumb_image: THUMBIMAGE\n  start_game:\n    online:\n      executables:\n        - path:\n            relative: AC3SP.exe\n  addons:\n    - id: 116\n      name: RELATED_GAMENAME_116\nlocalizations:\n  default:\n    l1: Assassin's Creed® III\n    THUMBIMAGE: da1733d50f6fbcf5b344d23ebb1ed52c.jpg\n";
        let sc = "version: 2.0\nroot:\n  name: \"Tom Clancy's Splinter Cell Chaos Theory®\"\n  thumb_image: 5afce18505053ddcecd8f0cbfc8041e8.jpg\n  start_game:\n    offline:\n      executables: []\n";
        let dlc = "version: 2.0\nroot:\n  name: Le DLC\n  start_game:\n    offline: {}\n";
        let ulc = "version: 2.0\nroot:\n  is_ulc: yes\n  name: Contenu\n  start_game:\n    offline: {}\n";
        let tiers = "version: 2.0\nroot:\n  name: Acheté sur Steam\n  third_party_platform:\n    name: steam\n  start_game:\n    offline: {}\n";
        let pas_jeu = "version: 2.0\nroot:\n  name: 30th Anniversary\n";
        let pas_a_moi = "version: 2.0\nroot:\n  name: Trackmania\n  start_game:\n    offline: {}\n";
        let config = pb(&[
            configuration(54, ac3),
            configuration(95, sc),
            configuration(116, dlc),
            configuration(17254, ulc),
            configuration(3499, pas_jeu),
            configuration(400, tiers),
            configuration(5595, pas_a_moi),
            configuration(7, ":\n  - [abîmé"),
        ]);
        let mut a_moi = lire_possession_ubisoft(&possession_inventee(&[54, 95, 116, 400, 17254], &[3499])).unwrap();
        let l = lire_configurations_ubisoft(&config, &a_moi).unwrap();
        assert_eq!(l.iter().map(|j| (j.id.as_str(), j.nom.as_str())).collect::<Vec<_>>(), [("uplay_54", "Assassin's Creed III"), ("uplay_95", "Tom Clancy's Splinter Cell Chaos Theory")]);
        assert_eq!(l[0].image.as_deref(), Some("https://ubistatic3-a.akamaihd.net/orbit/uplay_launcher_3_0/assets/da1733d50f6fbcf5b344d23ebb1ed52c.jpg"));
        assert!(l.iter().all(|j| !j.installe && j.dossier.is_none()));
        assert!(l.iter().all(|j| crate::boutiques::image_autorisee(j.image.as_deref().unwrap())));
        // Trackmania est dans le cache, pas dans la possession : jamais une carte.
        a_moi.insert(5595);
        assert_eq!(lire_configurations_ubisoft(&config, &a_moi).unwrap().len(), 3);
        // Fichiers abîmés : None.
        assert!(lire_configurations_ubisoft(&[0xff, 0xff], &a_moi).is_none());
        assert!(lire_configurations_ubisoft(&[], &a_moi).is_none(), "aucun produit");
        let mut p = possession_inventee(&[54], &[]);
        p.pop();
        assert!(lire_possession_ubisoft(&p).is_none(), "longueur annoncée fausse");
        assert!(lire_possession_ubisoft(&p[..100]).is_none(), "fichier coupé");
        assert!(lire_possession_ubisoft(&possession_inventee(&[], &[])).is_none(), "aucun produit possédé");
        assert!(!image_ubisoft_sure("../x.jpg") && !image_ubisoft_sure("x.exe") && image_ubisoft_sure("abc.png"));
    }

    #[test]
    fn ubisoft_lit_le_compte_le_plus_recent_dans_son_cache() {
        let d = tempfile::tempdir().unwrap();
        let cache = d.path().join("Ubisoft Game Launcher").join("cache");
        std::fs::create_dir_all(cache.join("configuration")).unwrap();
        std::fs::create_dir_all(cache.join("ownership")).unwrap();
        let jeu = |nom: &str| format!("root:\n  name: {nom}\n  start_game:\n    offline: {{}}\n");
        std::fs::write(cache.join("configuration").join("configurations"), pb(&[configuration(54, &jeu("AC3")), configuration(95, &jeu("SC"))])).unwrap();
        std::fs::write(cache.join("ownership").join("ancien"), possession_inventee(&[54], &[])).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(30));
        std::fs::write(cache.join("ownership").join("recent"), possession_inventee(&[95], &[])).unwrap();
        let l = possedes_ubisoft(Some(&cache)).unwrap();
        assert_eq!(l.iter().map(|j| j.id.as_str()).collect::<Vec<_>>(), ["uplay_95"], "le dernier compte connecté");
        // Le cache se trouve à côté du lanceur (InstallDir du registre, avec des « / »).
        let lanceur = valeurs(&[("InstallDir", &format!("{}/", d.path().join("Ubisoft Game Launcher").to_string_lossy().replace('\\', "/")))]);
        assert_eq!(cache_ubisoft(Some(&lanceur)), Some(cache.clone()));
        std::fs::remove_dir_all(cache.join("ownership")).unwrap();
        assert!(possedes_ubisoft(Some(&cache)).is_none(), "sans possession : rien (jamais tout le cache)");
        assert!(possedes_ubisoft(None).is_none());
    }

    #[test]
    fn un_jeu_installe_et_possede_ne_fait_qu_une_carte() {
        // Lot A : TerraScape installé (clé = AppName du manifeste) ; lot B : le même dans le catalogue (appId).
        let installe = lire_epic(Some(vec![crate::boutiques::JeuEpic {
            app: "6ef17d9f63c143ea88a4a4bcbcf1e91c".into(),
            namespace: "bfb8".into(),
            item: "e6eb".into(),
            nom: "TerraScape".into(),
            dossier: PathBuf::from(r"D:\Jeux\TerraScape"),
        }]));
        let possedes = lire_catalogue_epic(&catalogue_epic_invente());
        let l = fusion_possedes(installe.clone(), possedes.clone()).unwrap();
        assert_eq!(l.len(), 3, "TerraScape une seule fois");
        let t = l.iter().find(|j| j.nom == "TerraScape").unwrap();
        assert!(t.installe && t.dossier.as_deref() == Some(r"D:\Jeux\TerraScape"), "la carte installée gagne");
        assert!(t.image.as_deref().is_some_and(|u| u.contains("1200x1600")), "et reçoit la jaquette du catalogue");
        assert_eq!(l.iter().filter(|j| j.installe).count(), 1);
        // Catalogue illisible : les jeux installés seuls. Jeux installés illisibles : rien (jamais un état faux).
        assert_eq!(fusion_possedes(installe, None).unwrap().len(), 1);
        assert!(fusion_possedes(None, possedes).is_none());
    }

    /// RÉEL, LECTURE SEULE : ce que les lecteurs trouvent sur ce PC (copies effacées ensuite ; rien n'est lancé).
    /// `cargo test --lib essai_boutiques_pc -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn essai_boutiques_pc() {
        let d = tempfile::tempdir().unwrap();
        let debut = std::time::Instant::now();
        for (b, l) in lire_tout(d.path()) {
            match l {
                None => println!("{b} : lanceur absent ou illisible"),
                Some(l) => {
                    println!("{b} : {} jeu(x) — lanceur présent : {}", l.len(), lanceur_present(b));
                    for j in &l {
                        println!("   {} [{}] → {:?}", j.nom, j.id, j.dossier);
                    }
                }
            }
        }
        println!("lu en {:?}", debut.elapsed());
        assert!(std::fs::read_dir(d.path()).unwrap().next().is_none(), "aucune copie ne reste");
    }

    /// RÉEL, LECTURE SEULE : les jeux POSSÉDÉS (lot B), lus EN PLACE, sans aucune copie (le fichier `ownership`
    /// d'Ubisoft porte les clés de produit : jamais copié ; seuls les numéros sont lus, rien n'est affiché d'autre).
    /// `cargo test --lib essai_possedes -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn essai_possedes() {
        let debut = std::time::Instant::now();
        let l = possedes_epic(catalogue_epic().as_deref()).expect("catalogue Epic");
        println!("Epic : {} jeux possédés (lu en {:?})", l.len(), debut.elapsed());
        for j in l.iter().step_by((l.len() / 10).max(1)).take(10) {
            println!("   {} [{}] image : {}", j.nom, j.id, j.image.is_some());
        }
        println!("   sans image : {}", l.iter().filter(|j| j.image.is_none()).count());
        println!("   titres encore abîmés : {:?}", l.iter().filter(|j| j.nom.contains('?')).map(|j| &j.nom).collect::<Vec<_>>());
        let installes = lire_epic(crate::boutiques::installes_epic_du_pc()).unwrap_or_default();
        let f = fusion_possedes(Some(installes.clone()), Some(l)).unwrap();
        println!("Epic : {} installé(s) {:?} → {} carte(s), {} installée(s)", installes.len(), installes.iter().map(|j| &j.id).collect::<Vec<_>>(), f.len(), f.iter().filter(|j| j.installe).count());
        let ubi = cache_ubisoft(crate::registre::valeurs(Racine::Machine, CLE_UBISOFT).as_ref());
        match possedes_ubisoft(ubi.as_deref()) {
            Some(l) => {
                println!("Ubisoft : {} jeu(x) possédé(s)", l.len());
                for j in &l {
                    println!("   {} [{}] image : {:?}", j.nom, j.id, j.image);
                }
            }
            None => println!("Ubisoft : cache absent ou illisible"),
        }
    }

    /// RÉEL, LECTURE SEULE : les clés lues sur le PC tombent-elles sur celles de GOG Galaxy (même carte) ? La base de
    /// Galaxy est lue sur une copie, effacée ensuite (comme `boutiques::lire_galaxy`).
    /// `cargo test --lib essai_cles_galaxy -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn essai_cles_galaxy() {
        let d = tempfile::tempdir().unwrap();
        let copie = d.path().join("galaxy");
        let galaxy = crate::boutiques::lire_galaxy(&crate::boutiques::dossier_galaxy().expect("Galaxy"), &copie).unwrap();
        let _ = std::fs::remove_dir_all(&copie);
        assert!(!copie.exists(), "la copie de la base ne reste pas");
        let cles: std::collections::BTreeSet<&str> = galaxy.iter().map(|j| j.id.as_str()).collect();
        for p in ["epic", "uplay", "xboxone", "amazon", "battlenet"] {
            let ex: Vec<&str> = cles.iter().copied().filter(|c| c.starts_with(&format!("{p}_"))).take(4).collect();
            println!("Galaxy {p} : {} clé(s), exemples {ex:?}", cles.iter().filter(|c| c.starts_with(&format!("{p}_"))).count());
        }
        // Epic : la clé de Galaxy est-elle l'AppName, l'item ou l'espace de noms ? (comparés pour les jeux Epic du PC)
        for e in crate::boutiques::installes_epic_du_pc().unwrap_or_default() {
            for (quoi, v) in [("AppName", &e.app), ("CatalogItemId", &e.item), ("CatalogNamespace", &e.namespace)] {
                println!("Epic {} : {quoi} dans Galaxy ? {}", e.nom, cles.contains(format!("epic_{v}").as_str()));
            }
            println!("Epic {} : un jeu de ce nom dans Galaxy ? {:?}", e.nom, galaxy.iter().filter(|g| g.nom.eq_ignore_ascii_case(&e.nom)).map(|g| g.id.as_str()).collect::<Vec<_>>());
        }
        for (b, l) in lire_tout(&d.path().join("pc")) {
            for j in l.unwrap_or_default() {
                let cle = if b == "steam" { format!("steam_{}", j.id) } else { j.id.clone() };
                println!("{b} {} [{cle}] : {}", j.nom, if cles.contains(cle.as_str()) { "MÊME CLÉ que Galaxy" } else { "absente de Galaxy" });
            }
        }
    }
}
