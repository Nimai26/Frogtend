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

/// L'entrée du registre d'UN jeu GOG (relue au lancement).
pub fn entree_gog(id: &str) -> Option<Valeurs> {
    if id.is_empty() || !id.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    valeurs(Racine::Machine, &format!(r"{CLE_GOG}\{id}"))
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
