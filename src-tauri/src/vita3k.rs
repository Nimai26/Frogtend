//! Vita3K (PS Vita) : un UTILISATEUR Vita par personne (décision de Seb, 26/10 : « Séparer »). Relevé dans le code
//! source de Vita3K (commit 9a2da65, 26/10/2026) :
//! - portable : dossier `portable\` à côté du programme ; la Vita dans `portable\fs\`, les réglages dans
//!   `portable\config.yml` (app_init.cpp L270-295) ;
//! - un utilisateur = `ux0\user\<id 2 chiffres>\user.xml` (user.cpp L114-135), id = premier libre à partir de 00 ;
//!   un dossier sans user.xml lisible est ignoré ;
//! - l'utilisateur de la partie = clé `user-id` de config.yml (config.h L192) ; INCONNU, Vita3K prend le premier EN
//!   SILENCE et réécrit config.yml (app.cpp L185-208) : Frogtend crée l'utilisateur AVANT et règle `user-id`, Vita3K
//!   fermé (il réécrit config.yml à chaque démarrage, en gardant les clés qu'il connaît) ;
//! - parties : `ux0\user\<id>\savedata\<dossier du jeu>\` (io.cpp L158-174) — tout `savedata` est sauvegardé ;
//! - jamais `-d` (il efface en dur les parties de l'utilisateur 00, main.cpp L132-138).

use crate::erreurs::{Erreur, Resultat};
use std::path::{Path, PathBuf};

/// Dans une sauvegarde, le dossier (sous celui de l'émulateur) des parties de l'utilisateur Vita du profil.
pub const UTILISATEUR_VITA: &str = ".vita3k-utilisateur";

/// L'utilisateur Vita d'un profil, prêt pour une partie.
#[derive(Debug, Clone, PartialEq)]
pub struct UtilisateurDuProfil {
    pub id: String,
    pub cree: bool,
    /// Créé à l'instant, et première création par Frogtend sur ce Vita3K : l'utilisateur qu'il prenait jusque-là.
    pub ancien_courant: Option<String>,
}

pub fn est_vita3k(programme: &str) -> bool {
    Path::new(programme).file_name().is_some_and(|n| n.to_string_lossy().to_ascii_lowercase().starts_with("vita3k"))
}

/// Vita3K installé en portable (comme Frogtend l'installe) ? Sinon, ses données sont dans `%APPDATA%` : jamais touché.
pub fn portable(v: &Path) -> bool {
    v.join("portable").is_dir()
}

fn ux0(v: &Path) -> PathBuf {
    v.join("portable").join("fs").join("ux0")
}

fn config(v: &Path) -> PathBuf {
    v.join("portable").join("config.yml")
}

/// La valeur d'un attribut XML simple (`nom="valeur"`) dans `texte`.
fn attribut(texte: &str, nom: &str) -> Option<String> {
    let i = texte.find(&format!(" {nom}=\""))? + nom.len() + 3;
    let j = texte[i..].find('"')? + i;
    Some(texte[i..j].replace("&quot;", "\"").replace("&lt;", "<").replace("&gt;", ">").replace("&apos;", "'").replace("&amp;", "&"))
}

fn echapper(t: &str) -> String {
    t.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;").replace('\'', "&apos;")
}

/// Les utilisateurs : (id, nom), par id. Seulement ceux que Vita3K reconnaîtrait (user.xml lisible, id = dossier).
pub fn utilisateurs(v: &Path) -> Vec<(String, String)> {
    let mut l: Vec<(String, String)> = std::fs::read_dir(ux0(v).join("user"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let d = e.file_name().to_string_lossy().to_string();
            let x = std::fs::read_to_string(e.path().join("user.xml")).ok()?;
            let balise = &x[x.find("<user")?..];
            let id = attribut(balise, "id").unwrap_or_else(|| d.clone());
            (id == d).then(|| (id, attribut(balise, "name").unwrap_or_default()))
        })
        .collect();
    l.sort();
    l
}

/// Le `user.xml` d'un utilisateur, au format exact de Vita3K (user.cpp `save_user`).
pub fn user_xml(id: &str, nom: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<user id=\"{id}\" name=\"{}\">\n\t<theme use-background=\"true\">\n\t\t<content-id>default</content-id>\n\t</theme>\n\t<start-screen type=\"default\">\n\t\t<path></path>\n\t</start-screen>\n\t<backgrounds />\n</user>\n",
        echapper(nom)
    )
}

/// La valeur de `user-id` dans config.yml (`user-id: 01` ou `user-id: "01"`).
fn user_id_regle(v: &Path) -> Option<String> {
    valeur_reglee(v, "user-id")
}

/// La valeur d'une clé de premier niveau de config.yml (sans guillemets ni commentaire).
fn valeur_reglee(v: &Path, cle: &str) -> Option<String> {
    let debut = format!("{cle}:");
    std::fs::read_to_string(config(v))
        .ok()?
        .lines()
        .find_map(|l| l.strip_prefix(debut.as_str()).map(|x| x.split(" #").next().unwrap_or("").trim().trim_matches(['"', '\'']).to_string()))
        .filter(|x| !x.is_empty())
}

/// Le plein écran au démarrage d'un jeu : c'est le réglage `boot-apps-full-screen` qui le décide (main_window.cpp,
/// `launch_fullscreen = emuenv.cfg.boot_apps_full_screen`) ; l'option `-F` est acceptée mais n'est plus lue.
const PLEIN_ECRAN: &str = "boot-apps-full-screen";

/// Frogtend a activé le plein écran UNE fois (cette marque) : ensuite, un `false` est le choix de la personne, jamais
/// défait (Vita3K écrit `false` par défaut : sans marque, on ne sait pas qui l'a mis).
fn marque_plein_ecran(v: &Path) -> PathBuf {
    v.join("Profils").join(".plein-ecran-frogtend")
}

/// La table profil → utilisateur que Frogtend tient pour ce Vita3K (`Profils\.utilisateurs-frogtend`).
fn table(v: &Path) -> Vec<(String, String)> {
    std::fs::read_to_string(v.join("Profils").join(".utilisateurs-frogtend"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once('\t').map(|(i, p)| (i.to_string(), p.to_string())))
        .collect()
}

fn noter(v: &Path, id: &str, profil: &str) -> Resultat<()> {
    let mut t = table(v);
    t.retain(|(_, p)| p != profil);
    t.push((id.to_string(), profil.to_string()));
    std::fs::create_dir_all(v.join("Profils"))?;
    std::fs::write(v.join("Profils").join(".utilisateurs-frogtend"), t.iter().map(|(i, p)| format!("{i}\t{p}\n")).collect::<String>())?;
    Ok(())
}

fn trouver(v: &Path, l: &[(String, String)], profil: &str) -> Option<String> {
    let t = table(v);
    if let Some((id, _)) = t.iter().find(|(i, p)| p == profil && l.iter().any(|(x, _)| x == i)) {
        return Some(id.clone());
    }
    l.iter().find(|(i, n)| n == profil && !t.iter().any(|(x, p)| x == i && p != profil)).map(|(i, _)| i.clone())
}

/// L'utilisateur Vita de ce profil, créé s'il manque (Vita3K FERMÉ : l'appelant le vérifie). Seul un dossier
/// d'utilisateur NEUF est écrit (user.xml provisoire puis renommé, relu).
pub fn utilisateur_du_profil(v: &Path, profil: &str) -> Resultat<UtilisateurDuProfil> {
    let l = utilisateurs(v);
    if let Some(id) = trouver(v, &l, profil) {
        noter(v, &id, profil)?;
        // Une reprise des parties d'avant pas encore réussie : retentée.
        let ancien_courant = crate::emulateurs_profils::reprise_a_faire(v, profil);
        return Ok(UtilisateurDuProfil { id, cree: false, ancien_courant });
    }
    // Le premier id libre (2 chiffres), jamais un dossier déjà là (même illisible).
    let id = (0..100).map(|n| format!("{n:02}")).find(|i| !ux0(v).join("user").join(i).exists()).ok_or_else(|| {
        Erreur::Prevenir("Vita3K a déjà 100 utilisateurs : Frogtend ne peut pas en créer un pour ce profil.".into())
    })?;
    let dossier = ux0(v).join("user").join(&id);
    std::fs::create_dir_all(&dossier)?;
    let provisoire = dossier.join("user.xml.frogtend");
    std::fs::write(&provisoire, user_xml(&id, profil))?;
    std::fs::rename(&provisoire, dossier.join("user.xml"))?;
    if !utilisateurs(v).iter().any(|(i, n)| *i == id && n == profil) {
        return Err(Erreur::Disque("L'utilisateur Vita3K de ce profil n'a pas pu être enregistré.".into()));
    }
    noter(v, &id, profil)?;
    // La reprise des parties d'avant Frogtend : seulement à la toute première création par Frogtend sur ce Vita3K.
    let marque = v.join("Profils").join(".reprise-vita3k");
    let ancien_courant = if marque.exists() {
        None
    } else {
        std::fs::write(&marque, profil)?;
        let a = user_id_regle(v).filter(|r| l.iter().any(|(i, _)| i == r)).or_else(|| l.first().map(|(i, _)| i.clone()));
        if let Some(a) = &a {
            crate::emulateurs_profils::noter_reprise(v, profil, a)?;
        }
        a
    };
    Ok(UtilisateurDuProfil { id, cree: true, ancien_courant })
}

/// Règle la prochaine partie dans config.yml, Vita3K FERMÉ : l'utilisateur (`user-id`) et, la PREMIÈRE fois
/// seulement, le plein écran au démarrage du jeu (comme les autres émulateurs lancés par Frogtend ; ensuite, ce que la
/// personne règle dans Vita3K est gardé). Seules ces lignes changent (ajoutées si elles manquent) ; l'ancien fichier
/// est copié à l'abri avant ; écrit par un provisoire renommé, puis relu.
pub fn choisir(v: &Path, id: &str) -> Resultat<()> {
    let plein_ecran = !marque_plein_ecran(v).exists();
    if user_id_regle(v).as_deref() == Some(id) && !plein_ecran {
        return Ok(());
    }
    let f = config(v);
    let avant = match std::fs::read_to_string(&f) {
        Ok(t) => Some(t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => return Err(Erreur::Prevenir("Les réglages de Vita3K sont illisibles : Frogtend n'y touche pas.".into())),
    };
    let mut apres = remplacer_ligne(&avant.clone().unwrap_or_else(|| "---\n".into()), "user-id", &format!("user-id: \"{id}\""));
    if plein_ecran {
        apres = remplacer_ligne(&apres, PLEIN_ECRAN, &format!("{PLEIN_ECRAN}: true"));
    }
    if let Some(a) = &avant {
        let base = v.join(".frogtend-sauvegardes");
        std::fs::create_dir_all(&base)?;
        let nom = format!("{}-config.yml", crate::noyau::maintenant());
        let mut abri = base.join(&nom);
        let mut n = 2;
        while abri.exists() {
            abri = base.join(format!("{nom}-{n}"));
            n += 1;
        }
        std::fs::write(&abri, a)?;
    }
    let provisoire = f.with_extension("yml.frogtend");
    std::fs::write(&provisoire, &apres)?;
    std::fs::rename(&provisoire, &f)?;
    if user_id_regle(v).as_deref() != Some(id) {
        return Err(Erreur::Disque("L'utilisateur de Vita3K n'a pas pu être réglé.".into()));
    }
    // La marque seulement une fois le plein écran relu (sinon, il sera réessayé à la partie suivante).
    if plein_ecran && valeur_reglee(v, PLEIN_ECRAN).as_deref() == Some("true") {
        std::fs::create_dir_all(v.join("Profils"))?;
        std::fs::write(marque_plein_ecran(v), "")?;
    }
    Ok(())
}

// --- La langue de la console et l'écran d'accueil dès la première partie (règle de Seb, 08/11). Relevé dans le code
// de Vita3K (branche master, 10/11/2026) :
// - `sys-lang` vaut ENGLISH_US (1) d'origine (config/config.h) ; l'ordre des langues = `locale_tag_for_sys_lang`
//   (lang/src/lang.cpp) : ja 0, en 1, fr 2, es 3, de 4, it 5, nl 6, pt-PT 7, ru 8, ko 9, zh-Hant 10, zh-Hans 11, fi 12,
//   sv 13, da 14, no 15, pl 16, pt-BR 17, en-GB 18, tr 19 ; Vita3K réécrit config.yml avec toutes ses clés : « 1 » ne
//   dit pas qui l'a choisi → posée UNE seule fois (marque), comme pour Cemu et RPCS3 ;
// - l'interface suit déjà Windows tant que `user-lang` est vide (gui-qt/src/gui_language.cpp, `QLocale::system()`) ;
// - `show-welcome` vrai d'origine : la fenêtre d'accueil (en anglais, gui-qt/src/main_window.cpp) s'ouvre à CHAQUE
//   démarrage, devant le jeu → mise à faux une seule fois ; la personne peut la remettre dans Vita3K ;
// - la manette : SDL, disposition d'origine par POSITION (app.cpp `reset_controller_binding` : bas = Croix, droite =
//   Rond, gauche = Carré, haut = Triangle), pour toute manette (Xbox comme PlayStation) : rien à écrire ;
// - le bouton du milieu (SDL_GAMEPAD_BUTTON_GUIDE) met le jeu en pause, en dur (main_window.cpp `on_ps_button`) : il
//   reste à Vita3K.

/// Frogtend a posé la langue UNE fois sur ce Vita3K (cette marque) : ensuite, ce que la personne règle est gardé.
fn marque_premiere_partie(v: &Path) -> PathBuf {
    v.join("Profils").join(".reglages-frogtend")
}

/// Frogtend a retiré l'accueil UNE fois (cette marque) : la personne peut le remettre dans Vita3K.
fn marque_accueil(v: &Path) -> PathBuf {
    v.join("Profils").join(".accueil-frogtend")
}

/// Le firmware de la Vita est-il installé ? `install_pup` (packages/src/pup.cpp, relu le 10/11) extrait `vs0` dans
/// `portable\fs\` ; ses modules système sont dans `vs0\sys\external`. Tant qu'il manque, l'accueil de Vita3K reste :
/// c'est lui qui dit où le trouver (besoin 48 : Firehouse le servira).
pub fn firmware_installe(v: &Path) -> bool {
    // Le système ET les polices (la documentation de Vita3K exige les deux) : sans les polices, l'accueil reste.
    let fs = v.join("portable").join("fs");
    fs.join("vs0").join("sys").join("external").is_dir() && fs.join("sa0").is_dir()
}

// --- Le firmware de la Vita SERVI PAR FIREHOUSE (besoin 48, Firehouse 3.160.0 : conseil « Sony Playstation Vita »,
// trois `.PUP` `type: installation`, dans l'ordre : système, pré-installation, polices). Relevé dans le code de Vita3K
// (master, 10/11) : `Vita3K --firmware <fichier.PUP>` installe puis QUITTE sans fenêtre (main.cpp, `QuitRequested`) ;
// `install_pup` (packages/src/pup.cpp) extrait os0, pd0, sa0 et vs0 dans le dossier `fs` (portable\fs) : le système
// donne `vs0\sys\external`, la pré-installation `pd0`, les polices `sa0`. Frogtend note les empreintes installées
// (`fs\.frogtend-firmware`, une par ligne) : un fichier déjà posé n'est jamais retéléchargé.

/// Les fichiers du firmware de la Vita que Firehouse sert dans ce conseil, dans SON ordre.
pub fn firmwares_servis(bios: &[crate::bios::BiosConseil]) -> Vec<crate::bios::BiosConseil> {
    bios.iter()
        .filter(|b| b.sorte == "installation" && b.disponible && b.url.is_some() && b.sha1.as_deref().is_some_and(|s| s.len() == 40) && b.fichier.to_lowercase().ends_with(".pup"))
        .cloned()
        .collect()
}

fn note_firmware(v: &Path) -> PathBuf {
    v.join("portable").join("fs").join(".frogtend-firmware")
}

/// Les empreintes des fichiers du firmware posés par Frogtend.
/// Chaque ligne de la note : « <sha1> <dossier donné> » (le dossier relatif à `fs`, `/` comme séparateur).
fn empreintes_posees(v: &Path) -> Vec<(String, String)> {
    std::fs::read_to_string(note_firmware(v))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let mut m = l.split_whitespace();
            Some((m.next()?.to_lowercase(), m.next().unwrap_or("").to_string()))
        })
        .collect()
}

/// Le nom court d'un dossier donné par le firmware, pour la note (« vs0/sys/external », « pd0 », « sa0 »).
fn nom_de_trace(v: &Path, trace: &Path) -> String {
    trace.strip_prefix(v.join("portable").join("fs")).map(|r| r.to_string_lossy().replace('\\', "/")).unwrap_or_default()
}

/// Le dossier que donne le `n`-ième fichier du firmware servi une fois installé : d'après son nom chez Firehouse
/// (« preinstall », « polices »), sinon d'après sa place dans l'ordre servi (besoin 48 : système, pré-installation,
/// polices). `None` : un fichier en trop, inconnu — jamais installé à l'aveugle.
fn trace_du_fichier(v: &Path, fichier: &str, n: usize) -> Option<PathBuf> {
    let fs = v.join("portable").join("fs");
    let nom = fichier.to_lowercase();
    let systeme = fs.join("vs0").join("sys").join("external");
    Some(if nom.contains("preinst") {
        fs.join("pd0")
    } else if nom.contains("polices") || nom.contains("font") {
        fs.join("sa0")
    } else if nom == "psvupdat.pup" {
        systeme
    } else {
        match n {
            0 => systeme,
            1 => fs.join("pd0"),
            2 => fs.join("sa0"),
            _ => return None,
        }
    })
}

/// L'empreinte d'un dossier pour voir si une installation l'a changé : nombre de fichiers et date la plus récente.
fn empreinte_dossier(d: &Path) -> Option<(usize, std::time::SystemTime)> {
    if !d.is_dir() {
        return None;
    }
    let mut n = 0;
    let mut recent = std::time::SystemTime::UNIX_EPOCH;
    let mut a_voir = vec![d.to_path_buf()];
    while let Some(x) = a_voir.pop() {
        for e in std::fs::read_dir(&x).ok()?.flatten() {
            let m = e.metadata().ok()?;
            if m.is_dir() {
                a_voir.push(e.path());
            } else {
                n += 1;
                recent = recent.max(m.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH));
            }
        }
    }
    Some((n, recent))
}

/// Ce qui manque encore parmi les fichiers servis (avec sa place dans l'ordre servi) : rien hors portable ; un fichier
/// que Frogtend n'a pas encore posé, seulement si son dossier n'existe pas encore, ou si c'est Frogtend qui l'avait
/// donné (une version plus récente servie) — jamais par-dessus ce que la personne a installé elle-même. (`pref-path`
/// ne compte pas : Vita3K l'écrase en portable, relevé du 26/10.)
pub fn firmwares_manquants(v: &Path, servis: &[crate::bios::BiosConseil]) -> Vec<(usize, crate::bios::BiosConseil)> {
    if !portable(v) {
        return vec![];
    }
    let posees = empreintes_posees(v);
    servis
        .iter()
        .enumerate()
        .filter(|(n, f)| {
            let sha1 = f.sha1.clone().unwrap_or_default().to_lowercase();
            let Some(trace) = trace_du_fichier(v, &f.fichier, *n) else { return false };
            let nom = nom_de_trace(v, &trace);
            !posees.iter().any(|(s, _)| *s == sha1) && (!trace.is_dir() || posees.iter().any(|(_, d)| *d == nom))
        })
        .map(|(n, f)| (n, f.clone()))
        .collect()
}

/// Installe le `n`-ième fichier du firmware, déjà téléchargé et vérifié, par Vita3K lui-même (`--firmware`), Vita3K
/// FERMÉ, sans fenêtre, 10 minutes au plus. Réussi seulement si le dossier qu'il doit donner est apparu ou a changé
/// (Vita3K rend « réussi » même quand il échoue) ; alors noté.
pub fn installer_firmware(v: &Path, programme: &Path, n: usize, f: &crate::bios::BiosConseil, pup: &Path) -> Resultat<()> {
    if !portable(v) {
        return Err(Erreur::Refus("Vita3K n'est pas installé en portable : Frogtend n'y installe rien.".into()));
    }
    if !crate::lancement::processus_du_dossier(v).is_empty() {
        return Err(Erreur::Refus("Vita3K est ouvert : ferme-le (ou quitte ta partie), puis recommence.".into()));
    }
    let trace = trace_du_fichier(v, &f.fichier, n).ok_or_else(|| Erreur::Refus(format!("{} : fichier inconnu, pas installé.", f.fichier)))?;
    // Au TOUT PREMIER démarrage, Vita3K crée son config.yml sans le relire : le chemin de `fs` reste vide et
    // `--firmware` plante (`create_directories: ""`) — config.cpp `init_config`, main.cpp, constaté le 11/11 sur un
    // Vita3K neuf. Il crée donc d'abord ce fichier lui-même (`--version` : écrit config.yml puis quitte, sans fenêtre).
    if !config(v).is_file() {
        // 60 s : un antivirus peut examiner longuement le programme à son tout premier démarrage.
        match attendre_vita3k(v, programme, &["--version".as_ref()], 60)? {
            None => return Err(Erreur::Disque("Vita3K n'a pas créé sa configuration en une minute : arrêté, firmware pas installé.".into())),
            Some(s) if !s.success() || !config(v).is_file() => {
                return Err(Erreur::Disque("Vita3K n'a pas créé sa configuration : firmware pas installé.".into()))
            }
            Some(_) => {}
        }
    }
    let avant = empreinte_dossier(&trace);
    let Some(statut) = attendre_vita3k(v, programme, &["--firmware".as_ref(), pup.as_os_str()], 600)? else {
        return Err(Erreur::Disque(format!("Vita3K n'a pas fini d'installer {} en 10 minutes : arrêté.", f.fichier)));
    };
    constater_installation(v, f, &trace, avant, statut.success())
}

/// Lance Vita3K sans fenêtre avec ces arguments et attend sa fin, `secondes` au plus (`None` : arrêté au bout du
/// temps — seulement ce processus-là, lancé par Frogtend).
fn attendre_vita3k(v: &Path, programme: &Path, args: &[&std::ffi::OsStr], secondes: u64) -> Resultat<Option<std::process::ExitStatus>> {
    let mut enfant = crate::lancement::outil_sans_fenetre(&programme.to_string_lossy())
        .args(args)
        .current_dir(v)
        .spawn()
        .map_err(|e| Erreur::Disque(format!("Vita3K ne démarre pas ({e}).")))?;
    let debut = std::time::Instant::now();
    loop {
        if let Some(s) = enfant.try_wait().map_err(|e| Erreur::Disque(format!("Vita3K : {e}.")))? {
            return Ok(Some(s));
        }
        if debut.elapsed() > std::time::Duration::from_secs(secondes) {
            let _ = enfant.kill();
            let _ = enfant.wait();
            return Ok(None);
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
}

/// Le constat après `--firmware` : l'installation a réussi si Vita3K a fini sans erreur ET si le dossier attendu est
/// apparu ou a changé ; alors l'empreinte du fichier est notée.
fn constater_installation(v: &Path, f: &crate::bios::BiosConseil, trace: &Path, avant: Option<(usize, std::time::SystemTime)>, fini: bool) -> Resultat<()> {
    let apres = empreinte_dossier(trace);
    if !fini || apres.is_none() || apres == avant {
        return Err(Erreur::Disque(format!("Vita3K n'a pas installé {} (fichier refusé ?).", f.fichier)));
    }
    let sha1 = f.sha1.clone().unwrap_or_default().to_lowercase();
    let mut posees = empreintes_posees(v);
    if !posees.iter().any(|(s, _)| *s == sha1) {
        posees.push((sha1, nom_de_trace(v, trace)));
    }
    std::fs::create_dir_all(v.join("portable").join("fs"))?;
    let texte: String = posees.iter().map(|(s, d)| format!("{s} {d}\n")).collect();
    std::fs::write(note_firmware(v), texte)?;
    Ok(())
}

/// La langue de la console Vita pour une langue de Windows (LANGID) ; `None` : une langue que la Vita n'a pas.
pub fn langue_console(langid: u16) -> Option<u8> {
    Some(match langid & 0x3FF {
        0x11 => 0,
        // Anglais : celui des États-Unis (et du Canada…) ; britannique pour le Royaume-Uni, l'Australie, la
        // Nouvelle-Zélande, l'Irlande et l'Afrique du Sud.
        0x09 if matches!(langid, 0x0809 | 0x0C09 | 0x1409 | 0x1809 | 0x1C09) => 18,
        0x09 => 1,
        0x0C => 2,
        0x0A => 3,
        0x07 => 4,
        0x10 => 5,
        0x13 => 6,
        0x16 if langid == 0x0416 => 17,
        0x16 => 7,
        0x19 => 8,
        0x12 => 9,
        // Chinois : traditionnel à Taïwan, Hong Kong et Macao (et le traditionnel neutre), simplifié ailleurs.
        0x04 if matches!(langid, 0x0404 | 0x0C04 | 0x1404 | 0x7C04) => 10,
        0x04 => 11,
        0x0B => 12,
        0x1D => 13,
        0x06 => 14,
        0x14 => 15,
        0x15 => 16,
        0x1F => 19,
        _ => return None,
    })
}

/// Avant une partie sur ce Vita3K (portable et FERMÉ, après `choisir`) : la langue de la console prise de Windows
/// (seulement absente ou à sa valeur d'origine), UNE seule fois ; l'accueil retiré (seulement absent ou vrai), UNE
/// seule fois et seulement une fois le firmware installé. L'ancien fichier est copié à l'abri ; écrit par un
/// provisoire renommé, puis chaque clé posée est relue. Rend vrai si config.yml a changé.
pub fn premiere_partie(v: &Path, langid: u16) -> Resultat<bool> {
    if !portable(v) {
        return Ok(false);
    }
    let langue_a_faire = !marque_premiere_partie(v).exists();
    let accueil_a_faire = !marque_accueil(v).exists() && firmware_installe(v);
    if !langue_a_faire && !accueil_a_faire {
        return Ok(false);
    }
    let f = config(v);
    let avant = match std::fs::read_to_string(&f) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => "---\n".into(),
        Err(_) => return Err(Erreur::Prevenir("Les réglages de Vita3K sont illisibles : Frogtend n'y touche pas.".into())),
    };
    let mut apres = avant.clone();
    let mut posees: Vec<(&str, String)> = Vec::new();
    if langue_a_faire {
        if let Some(n) = langue_console(langid) {
            if valeur_reglee(v, "sys-lang").is_none_or(|x| x == "1") {
                apres = remplacer_ligne(&apres, "sys-lang", &format!("sys-lang: {n}"));
                posees.push(("sys-lang", n.to_string()));
            }
        }
    }
    if accueil_a_faire && valeur_reglee(v, "show-welcome").is_none_or(|x| x == "true") {
        apres = remplacer_ligne(&apres, "show-welcome", "show-welcome: false");
        posees.push(("show-welcome", "false".into()));
    }
    let change = apres != avant;
    if change {
        if f.exists() {
            let base = v.join(".frogtend-sauvegardes");
            std::fs::create_dir_all(&base)?;
            let nom = format!("{}-config.yml", crate::noyau::maintenant());
            let mut abri = base.join(&nom);
            let mut n = 2;
            while abri.exists() {
                abri = base.join(format!("{nom}-{n}"));
                n += 1;
            }
            std::fs::write(&abri, &avant)?;
        }
        let provisoire = f.with_extension("yml.frogtend");
        std::fs::write(&provisoire, &apres)?;
        if let Err(e) = std::fs::rename(&provisoire, &f) {
            let _ = std::fs::remove_file(&provisoire);
            return Err(e.into());
        }
        // Relu : les marques seulement si c'est bien écrit (sinon, réessayé à la partie suivante).
        if posees.iter().any(|(cle, val)| valeur_reglee(v, cle).as_deref() != Some(val.as_str())) {
            return Err(Erreur::Disque("Les réglages de Vita3K n'ont pas pu être posés.".into()));
        }
    }
    std::fs::create_dir_all(v.join("Profils"))?;
    if langue_a_faire {
        std::fs::write(marque_premiere_partie(v), "")?;
    }
    if accueil_a_faire {
        std::fs::write(marque_accueil(v), "")?;
    }
    Ok(change)
}

/// La disposition des boutons de Vita3K est-elle celle d'origine ? `controller-binds` (15 boutons) et
/// `controller-axis-binds` (6 axes) : absents, vides ou d'une autre longueur, Vita3K reprend celle d'origine
/// (app_init.cpp) ; sinon l'identité (bouton SDL n → bouton SDL n, app.cpp `reset_controller_binding`).
pub fn disposition_d_origine(v: &Path) -> bool {
    if !portable(v) {
        return false;
    }
    let texte = std::fs::read_to_string(config(v)).unwrap_or_default();
    let identite = |cle: &str, taille: usize| {
        let l = liste_yaml(&texte, cle);
        l.len() != taille || l.iter().enumerate().all(|(i, x)| *x == i as i64)
    };
    identite("controller-binds", 15) && identite("controller-axis-binds", 6)
}

/// Les nombres d'une liste YAML de premier niveau (`cle: [0, 1]` ou `cle:` suivi de lignes `  - 0`). Une valeur
/// illisible compte comme -1 (jamais l'identité).
fn liste_yaml(texte: &str, cle: &str) -> Vec<i64> {
    let debut = format!("{cle}:");
    let mut lignes = texte.lines().skip_while(|l| !l.starts_with(debut.as_str()));
    let Some(premiere) = lignes.next() else { return vec![] };
    let reste = premiere[debut.len()..].split(" #").next().unwrap_or("").trim();
    let nombre = |s: &str| s.trim().parse::<i64>().unwrap_or(-1);
    if let Some(dedans) = reste.strip_prefix('[') {
        return dedans.trim_end_matches(']').split(',').filter(|s| !s.trim().is_empty()).map(nombre).collect();
    }
    if !reste.is_empty() {
        return vec![-1];
    }
    lignes
        .take_while(|l| l.starts_with(' ') || l.starts_with('-'))
        .filter_map(|l| l.trim().strip_prefix('-').map(nombre))
        .collect()
}

/// Remplace la ligne de premier niveau `cle:` de ce texte YAML par `ligne` (ajoutée à la fin si elle manque), en
/// gardant les fins de ligne du fichier.
fn remplacer_ligne(t: &str, cle: &str, ligne: &str) -> String {
    let debut = format!("{cle}:");
    if t.lines().any(|l| l.starts_with(debut.as_str())) {
        let fin = if t.contains("\r\n") { "\r\n" } else { "\n" };
        let mut r: String = t.lines().map(|l| if l.starts_with(debut.as_str()) { ligne.to_string() } else { l.to_string() }).collect::<Vec<_>>().join(fin);
        if t.ends_with('\n') {
            r.push_str(fin);
        }
        r
    } else {
        let fin = if t.contains("\r\n") { "\r\n" } else { "\n" };
        format!("{t}{}{ligne}{fin}", if t.is_empty() || t.ends_with('\n') { "" } else { fin })
    }
}

/// L'utilisateur Vita de ce profil s'il existe, SANS rien créer (pour la sauvegarde).
pub fn utilisateur_existant(v: &Path, profil: &str) -> Option<String> {
    trouver(v, &utilisateurs(v), profil)
}

/// Le dossier des parties d'un utilisateur.
pub fn parties(v: &Path, id: &str) -> PathBuf {
    ux0(v).join("user").join(id).join("savedata")
}

/// Un `param.sfo` lu (1 Mo au plus).
fn lire_sfo(f: &Path) -> Option<Vec<u8>> {
    (std::fs::metadata(f).ok()?.len() <= 1 << 20).then(|| std::fs::read(f).ok()).flatten()
}

/// Le `sce_sys\param.sfo` d'un jeu PS Vita : dans son dossier, ou dans son .vpk/.zip — UN SEUL jeu (un zip qui en
/// contient plusieurs, de numéros différents : aucun, on ne devine pas lequel).
fn sfo_du_jeu(jeu: &Path) -> Option<Vec<u8>> {
    use std::io::Read;
    if jeu.is_dir() {
        return lire_sfo(&jeu.join("sce_sys").join("param.sfo"));
    }
    let mut z = zip::ZipArchive::new(std::fs::File::open(jeu).ok()?).ok()?;
    let noms: Vec<String> = z
        .file_names()
        .filter(|n| {
            let n = n.replace('\\', "/").to_ascii_lowercase();
            n == "sce_sys/param.sfo" || n.ends_with("/sce_sys/param.sfo")
        })
        .map(String::from)
        .collect();
    let mut sfos = Vec::new();
    for nom in noms {
        let f = z.by_name(&nom).ok()?;
        let mut o = Vec::new();
        f.take(1 << 20).read_to_end(&mut o).ok()?;
        sfos.push(o);
    }
    let mut ids: Vec<String> = sfos.iter().filter_map(|s| crate::contenus::valeur_sfo(s, "TITLE_ID")).collect();
    ids.sort();
    ids.dedup();
    (ids.len() == 1).then(|| sfos.into_iter().next()).flatten()
}

fn id_valide(s: &[u8]) -> Option<String> {
    crate::contenus::valeur_sfo(s, "TITLE_ID").filter(|t| t.len() == 9 && t.chars().all(|c| c.is_ascii_alphanumeric()))
}

/// Le numéro d'un jeu PS Vita (TITLE_ID), seulement 9 lettres et chiffres (il devient un nom de dossier).
pub fn titre_id(jeu: &Path) -> Option<String> {
    id_valide(&sfo_du_jeu(jeu)?)
}

/// Ce jeu peut-il se lancer par son numéro, sans repasser son fichier ? Seulement s'il est installé dans ce Vita3K À
/// LA MÊME VERSION (`APP_VER` du fichier = celle de `ux0\patch\<id>` s'il y en a une, sinon de `ux0\app\<id>`) :
/// une autre version (mise à jour, autre sortie) repasse par le fichier et s'installe, comme avant.
pub fn pret_a_lancer(v: &Path, jeu: &Path) -> Option<String> {
    let s = sfo_du_jeu(jeu)?;
    let id = id_valide(&s)?;
    let version = crate::contenus::valeur_sfo(&s, "APP_VER");
    let installe = ["patch", "app"].iter().find_map(|d| lire_sfo(&ux0(v).join(d).join(&id).join("sce_sys").join("param.sfo")))?;
    (crate::contenus::valeur_sfo(&installe, "APP_VER") == version).then_some(id)
}

/// Un jeu déjà installé se lance par son numéro (`-r <TITLE_ID>`, config.cpp L367) : l'argument qui EST le fichier du
/// jeu est remplacé — Vita3K ne le réinstalle plus. Sans un tel argument (ligne réglée à la main), rien ne change.
pub fn lancer_par_numero(arguments: &mut Vec<String>, jeu: &Path, titre: &str) {
    let j = jeu.to_string_lossy().to_string();
    if let Some(i) = arguments.iter().position(|a| *a == j) {
        arguments.splice(i..=i, ["-r".to_string(), titre.to_string()]);
    }
}

/// Décision de Seb (26/10) : les parties de l'utilisateur que Vita3K prenait AVANT Frogtend sont COPIÉES chez la
/// première personne qui joue (`u.ancien_courant`), jeu par jeu ; jamais déplacées, jamais par-dessus.
pub fn reprendre_parties_d_avant(v: &Path, u: &UtilisateurDuProfil) -> Resultat<usize> {
    let mut copies = 0;
    // L'utilisateur d'avant appartient (désormais) au profil de quelqu'un : JAMAIS ses parties chez un autre.
    if u.ancien_courant.as_deref().is_some_and(|s| table(v).iter().any(|(i, _)| i == s)) {
        crate::emulateurs_profils::reprise_faite(v);
        return Ok(0);
    }
    if let Some(source) = u.ancien_courant.as_deref().filter(|s| *s != u.id) {
        for e in std::fs::read_dir(parties(v, source)).into_iter().flatten().flatten() {
            let cible = parties(v, &u.id).join(e.file_name());
            if e.file_type().is_ok_and(|t| t.is_dir()) && !cible.exists() {
                crate::eden::copier_dossier_entier(&e.path(), &cible)?;
                copies += 1;
            }
        }
    }
    // Réussie (une erreur plus haut la laisse « à faire ») : plus jamais retentée.
    if u.ancien_courant.is_some() {
        crate::emulateurs_profils::reprise_faite(v);
    }
    Ok(copies)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vita_portable() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("portable")).unwrap();
        d
    }

    #[test]
    fn le_fichier_d_un_utilisateur_a_la_forme_de_vita3k() {
        let x = user_xml("01", "Léa & \"Zoé\"");
        assert!(x.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<user id=\"01\" name=\"Léa &amp; &quot;Zoé&quot;\">"));
        assert_eq!(attribut(&x, "name").as_deref(), Some("Léa & \"Zoé\""));
    }

    #[test]
    fn chaque_profil_a_son_utilisateur_et_la_partie_choisit_le_sien() {
        let d = vita_portable();
        let v = d.path();
        // L'utilisateur que Vita3K crée seul (00), réglé dans config.yml avec d'autres réglages.
        std::fs::create_dir_all(ux0(v).join("user").join("00")).unwrap();
        std::fs::write(ux0(v).join("user").join("00").join("user.xml"), user_xml("00", "Vita3K")).unwrap();
        std::fs::write(config(v), "---\nboot-apps-full-screen: true\nuser-id: 00\nlog-level: 2\n").unwrap();
        let seb = utilisateur_du_profil(v, "Seb").unwrap();
        assert_eq!((seb.id.as_str(), seb.cree, seb.ancien_courant.as_deref()), ("01", true, Some("00")));
        let lea = utilisateur_du_profil(v, "Léa").unwrap();
        assert_eq!((lea.id.as_str(), lea.ancien_courant.clone()), ("02", None), "une seule reprise par Vita3K");
        assert_eq!(utilisateur_du_profil(v, "Seb").unwrap().id, "01");
        assert_eq!(utilisateur_existant(v, "Léa").as_deref(), Some("02"));
        // La partie de Léa : seule la ligne user-id change, l'ancien fichier à l'abri.
        choisir(v, "02").unwrap();
        assert_eq!(std::fs::read_to_string(config(v)).unwrap(), "---\nboot-apps-full-screen: true\nuser-id: \"02\"\nlog-level: 2\n", "plein écran déjà là : inchangé");
        assert_eq!(std::fs::read_dir(v.join(".frogtend-sauvegardes")).unwrap().count(), 1);
        choisir(v, "02").unwrap();
        assert_eq!(std::fs::read_dir(v.join(".frogtend-sauvegardes")).unwrap().count(), 1, "déjà réglé : rien de réécrit");
        assert!(marque_plein_ecran(v).exists(), "le plein écran était déjà là : noté comme réglé");
        // La PREMIÈRE fois (pas encore de marque), Vita3K a écrit `false` par défaut : Frogtend met `true`. Un
        // commentaire après la valeur, des fins de ligne Windows : lus et gardés.
        std::fs::remove_file(marque_plein_ecran(v)).unwrap();
        std::fs::write(config(v), "---\r\nuser-id: 02 # Léa\r\nlog-level: 2\r\nboot-apps-full-screen: false\r\n").unwrap();
        assert_eq!(user_id_regle(v).as_deref(), Some("02"));
        choisir(v, "01").unwrap();
        assert_eq!(std::fs::read_to_string(config(v)).unwrap(), "---\r\nuser-id: \"01\"\r\nlog-level: 2\r\nboot-apps-full-screen: true\r\n");
        // Ensuite, la personne le retire dans Vita3K : son choix est GARDÉ (seul l'utilisateur change).
        std::fs::write(config(v), "---\nuser-id: \"01\"\nboot-apps-full-screen: false\n").unwrap();
        choisir(v, "02").unwrap();
        assert_eq!(std::fs::read_to_string(config(v)).unwrap(), "---\nuser-id: \"02\"\nboot-apps-full-screen: false\n");
        assert!(est_vita3k("C:\\Vita3K\\Vita3K.exe") && !est_vita3k("C:\\Cemu\\Cemu.exe"));
        // Sans la ligne : ajoutée.
        std::fs::write(config(v), "---\nlog-level: 2").unwrap();
        choisir(v, "01").unwrap();
        assert_eq!(std::fs::read_to_string(config(v)).unwrap(), "---\nlog-level: 2\nuser-id: \"01\"\n", "plein écran déjà réglé une fois : plus touché");
    }

    /// Le firmware de la Vita posé comme par `install_pup` (seulement le dossier qui le signale).
    fn avec_firmware(v: &Path) {
        std::fs::create_dir_all(v.join("portable").join("fs").join("vs0").join("sys").join("external")).unwrap();
        std::fs::create_dir_all(v.join("portable").join("fs").join("sa0")).unwrap();
    }

    #[test]
    fn la_premiere_partie_pose_la_langue_et_retire_l_accueil_une_seule_fois() {
        let d = vita_portable();
        let v = d.path();
        // Vita3K jamais ouvert, sans firmware : la langue seule, l'accueil reste (il dit où trouver le firmware).
        assert!(premiere_partie(v, 0x040C).unwrap());
        assert_eq!(std::fs::read_to_string(config(v)).unwrap(), "---\nsys-lang: 2\n");
        assert!(!premiere_partie(v, 0x040C).unwrap(), "rien de plus sans firmware");
        // Le firmware installé : l'accueil retiré, une seule fois.
        avec_firmware(v);
        assert!(premiere_partie(v, 0x040C).unwrap());
        assert_eq!(std::fs::read_to_string(config(v)).unwrap(), "---\nsys-lang: 2\nshow-welcome: false\n");
        // La personne remet l'anglais et l'accueil : gardés.
        std::fs::write(config(v), "---\nsys-lang: 1\nshow-welcome: true\n").unwrap();
        assert!(!premiere_partie(v, 0x040C).unwrap());
        assert_eq!(std::fs::read_to_string(config(v)).unwrap(), "---\nsys-lang: 1\nshow-welcome: true\n");
        // Un Vita3K déjà ouvert (valeurs d'origine), avec firmware : les deux changées, l'ancien fichier à l'abri.
        let d2 = vita_portable();
        let v2 = d2.path();
        avec_firmware(v2);
        std::fs::write(config(v2), "---\r\nsys-lang: 1\r\nshow-welcome: true\r\nuser-id: \"01\"\r\n").unwrap();
        assert!(premiere_partie(v2, 0x0407).unwrap());
        assert_eq!(std::fs::read_to_string(config(v2)).unwrap(), "---\r\nsys-lang: 4\r\nshow-welcome: false\r\nuser-id: \"01\"\r\n");
        assert_eq!(std::fs::read_dir(v2.join(".frogtend-sauvegardes")).unwrap().count(), 1);
        // Un autre choix déjà fait (japonais) : gardé ; une langue que la Vita n'a pas : l'accueil seul.
        let d3 = vita_portable();
        let v3 = d3.path();
        avec_firmware(v3);
        std::fs::write(config(v3), "---\nsys-lang: 0\n").unwrap();
        assert!(premiere_partie(v3, 0x040C).unwrap());
        assert_eq!(std::fs::read_to_string(config(v3)).unwrap(), "---\nsys-lang: 0\nshow-welcome: false\n");
        let d4 = vita_portable();
        avec_firmware(d4.path());
        assert!(premiere_partie(d4.path(), 0x0418).unwrap(), "roumain");
        assert_eq!(std::fs::read_to_string(config(d4.path())).unwrap(), "---\nshow-welcome: false\n");
        // Une valeur d'accueil inhabituelle (« yes ») : pas touchée, la langue posée sans erreur.
        let d5 = vita_portable();
        avec_firmware(d5.path());
        std::fs::write(config(d5.path()), "---\nshow-welcome: yes\n").unwrap();
        assert!(premiere_partie(d5.path(), 0x040C).unwrap());
        assert_eq!(std::fs::read_to_string(config(d5.path())).unwrap(), "---\nshow-welcome: yes\nsys-lang: 2\n");
        // Hors portable : rien.
        let hors = tempfile::tempdir().unwrap();
        assert!(!premiere_partie(hors.path(), 0x040C).unwrap());
        assert!(!hors.path().join("Profils").exists());
        assert_eq!((langue_console(0x0809), langue_console(0x0C09), langue_console(0x0416), langue_console(0x0816)), (Some(18), Some(18), Some(17), Some(7)));
        assert_eq!((langue_console(0x0404), langue_console(0x7C04), langue_console(0x0804)), (Some(10), Some(10), Some(11)));
    }

    #[test]
    fn choisir_puis_la_premiere_partie_gardent_l_utilisateur_et_le_plein_ecran() {
        let d = vita_portable();
        let v = d.path();
        avec_firmware(v);
        choisir(v, "01").unwrap();
        assert!(premiere_partie(v, 0x040C).unwrap());
        let t = std::fs::read_to_string(config(v)).unwrap();
        assert_eq!(t, "---\nuser-id: \"01\"\nboot-apps-full-screen: true\nsys-lang: 2\nshow-welcome: false\n");
        assert!(marque_plein_ecran(v).exists() && marque_premiere_partie(v).exists() && marque_accueil(v).exists());
        // Partie suivante : seul l'utilisateur peut changer.
        choisir(v, "02").unwrap();
        assert!(!premiere_partie(v, 0x040C).unwrap());
        assert_eq!(std::fs::read_to_string(config(v)).unwrap(), t.replace("\"01\"", "\"02\""));
    }

    #[test]
    fn un_config_illisible_n_est_jamais_touche() {
        let d = vita_portable();
        let v = d.path();
        std::fs::write(config(v), [0xFFu8, 0xFE, 0x00, 0xD8]).unwrap();
        assert!(premiere_partie(v, 0x040C).is_err());
        assert!(!marque_premiere_partie(v).exists());
    }

    fn pup(fichier: &str, sha1: char) -> crate::bios::BiosConseil {
        let mut ignore = crate::bios::Bilan::default();
        let s: String = std::iter::repeat_n(sha1, 40).collect();
        crate::bios::lire_bios(
            &serde_json::json!([{"fichier": fichier, "type": "installation", "racine": "installation_par_l_emulateur", "disponible": true, "sha1": s, "taille": 10, "url": format!("/api/jeux/v1/bios/{s}")}]),
            &mut ignore,
        )
        .remove(0)
    }

    #[test]
    fn le_firmware_servi_dans_l_ordre_et_seulement_ce_qui_manque() {
        let servis = vec![pup("PSVUPDAT.PUP", 'a'), pup("PSP2UPDAT-preinstall.PUP", 'b'), pup("PSP2UPDAT-polices.PUP", 'c')];
        let mut tous = servis.clone();
        tous.push(pup("autre.zip", 'd'));
        assert_eq!(firmwares_servis(&tous).iter().map(|f| f.fichier.as_str()).collect::<Vec<_>>(), ["PSVUPDAT.PUP", "PSP2UPDAT-preinstall.PUP", "PSP2UPDAT-polices.PUP"]);
        assert!(firmwares_manquants(tempfile::tempdir().unwrap().path(), &servis).is_empty(), "hors portable : rien");
        let d = vita_portable();
        let v = d.path();
        let noms = |l: Vec<(usize, crate::bios::BiosConseil)>| l.into_iter().map(|(n, f)| format!("{n}:{}", f.fichier)).collect::<Vec<_>>();
        assert_eq!(noms(firmwares_manquants(v, &servis)).len(), 3);
        // Le système installé par Frogtend (constaté, noté) : les deux autres restent.
        let fs = v.join("portable").join("fs");
        let systeme = fs.join("vs0").join("sys").join("external");
        std::fs::create_dir_all(&systeme).unwrap();
        std::fs::write(systeme.join("libc.suprx"), "x").unwrap();
        constater_installation(v, &servis[0], &systeme, None, true).unwrap();
        assert_eq!(std::fs::read_to_string(note_firmware(v)).unwrap(), format!("{} vs0/sys/external\n", "a".repeat(40)));
        assert_eq!(noms(firmwares_manquants(v, &servis)), ["1:PSP2UPDAT-preinstall.PUP", "2:PSP2UPDAT-polices.PUP"]);
        // Une version plus récente du système servie : c'est Frogtend qui l'avait posé → proposée.
        let mut nouveau = servis.clone();
        nouveau[0] = pup("PSVUPDAT.PUP", 'e');
        assert_eq!(noms(firmwares_manquants(v, &nouveau))[0], "0:PSVUPDAT.PUP");
        // Le système installé par la personne elle-même, rien d'autre : seulement la pré-installation et les polices.
        let d2 = vita_portable();
        std::fs::create_dir_all(d2.path().join("portable").join("fs").join("vs0").join("sys").join("external")).unwrap();
        assert_eq!(noms(firmwares_manquants(d2.path(), &servis)), ["1:PSP2UPDAT-preinstall.PUP", "2:PSP2UPDAT-polices.PUP"]);
        // Tout installé par la personne : rien.
        for s in ["pd0", "sa0"] {
            std::fs::create_dir_all(d2.path().join("portable").join("fs").join(s)).unwrap();
        }
        assert!(firmwares_manquants(d2.path(), &servis).is_empty());
        // Le type d'après le nom, sinon d'après la place ; un 4e fichier inconnu : jamais installé.
        assert_eq!(trace_du_fichier(v, "PSP2UPDAT-polices.PUP", 0), Some(fs.join("sa0")));
        assert_eq!(trace_du_fichier(v, "PSP2UPDAT.PUP", 2), Some(fs.join("sa0")));
        assert_eq!(trace_du_fichier(v, "PSP2UPDAT.PUP", 3), None);
        let mut en_trop = servis.clone();
        en_trop.push(pup("autre.PUP", 'f'));
        assert_eq!(firmwares_servis(&en_trop).len(), 4);
        assert_eq!(noms(firmwares_manquants(d2.path(), &en_trop)), Vec::<String>::new());
    }

    #[test]
    fn un_echec_de_vita3k_n_est_jamais_note_comme_installe() {
        let d = vita_portable();
        let v = d.path();
        let f = pup("PSP2UPDAT-polices.PUP", 'c');
        let sa0 = v.join("portable").join("fs").join("sa0");
        // Rien n'est apparu : échec, même si Vita3K a fini « sans erreur ».
        assert!(constater_installation(v, &f, &sa0, None, true).is_err());
        // Un dossier qui était déjà là et n'a pas changé : échec aussi.
        std::fs::create_dir_all(sa0.join("data")).unwrap();
        std::fs::write(sa0.join("data").join("x.pvf"), "x").unwrap();
        let avant = empreinte_dossier(&sa0);
        assert!(constater_installation(v, &f, &sa0, avant, true).is_err());
        // Vita3K en erreur : échec.
        assert!(constater_installation(v, &f, &sa0, None, false).is_err());
        assert!(!note_firmware(v).exists(), "rien de noté");
        // Le dossier est apparu : noté.
        assert!(constater_installation(v, &f, &sa0, None, true).is_ok());
        assert!(std::fs::read_to_string(note_firmware(v)).unwrap().ends_with(" sa0\n"));
        // Un fichier écarté par le conseil (indisponible) n'est jamais servi.
        let mut ignore = crate::bios::Bilan::default();
        let indispo = crate::bios::lire_bios(&serde_json::json!([{"fichier": "PSVUPDAT.PUP", "type": "installation", "racine": "installation_par_l_emulateur", "disponible": false, "sha1": "a".repeat(40), "url": "/x"}]), &mut ignore);
        assert!(firmwares_servis(&indispo).is_empty());
    }

    #[test]
    fn la_disposition_de_vita3k_d_origine_ou_changee() {
        let d = vita_portable();
        let v = d.path();
        assert!(!disposition_d_origine(tempfile::tempdir().unwrap().path()), "hors portable : Frogtend ne sait pas");
        assert!(disposition_d_origine(v), "pas de config.yml");
        let origine = "---\ncontroller-binds:\n  - 0\n  - 1\n  - 2\n  - 3\n  - 4\n  - 5\n  - 6\n  - 7\n  - 8\n  - 9\n  - 10\n  - 11\n  - 12\n  - 13\n  - 14\ncontroller-axis-binds: [0, 1, 2, 3, 4, 5]\nuser-id: 00\n";
        std::fs::write(config(v), origine).unwrap();
        assert!(disposition_d_origine(v));
        std::fs::write(config(v), origine.replace("  - 0\n  - 1\n", "  - 1\n  - 0\n")).unwrap();
        assert!(!disposition_d_origine(v), "Croix et Rond échangés");
        std::fs::write(config(v), origine.replace("[0, 1, 2, 3, 4, 5]", "[2, 3, 0, 1, 4, 5]")).unwrap();
        assert!(!disposition_d_origine(v), "sticks échangés");
        std::fs::write(config(v), "---\ncontroller-binds: []\n").unwrap();
        assert!(disposition_d_origine(v), "vide : Vita3K reprend celle d'origine");
    }

    fn sfo(titre: &str) -> Vec<u8> {
        sfo_version(titre, None)
    }

    /// Un PARAM.SFO minimal : TITLE_ID, et APP_VER si donnée (clés texte).
    fn sfo_version(titre: &str, version: Option<&str>) -> Vec<u8> {
        let mut paires: Vec<(&str, String)> = vec![("TITLE_ID", format!("{titre}\0"))];
        if let Some(v) = version {
            paires.insert(0, ("APP_VER", format!("{v}\0")));
        }
        let mut cles: Vec<u8> = Vec::new();
        let mut donnees: Vec<u8> = Vec::new();
        let mut index = Vec::new();
        for (c, v) in &paires {
            index.extend((cles.len() as u16).to_le_bytes());
            index.extend(0x0204u16.to_le_bytes());
            index.extend((v.len() as u32).to_le_bytes());
            index.extend((v.len() as u32).to_le_bytes());
            index.extend((donnees.len() as u32).to_le_bytes());
            cles.extend(c.as_bytes());
            cles.push(0);
            donnees.extend(v.as_bytes());
        }
        while cles.len() % 4 != 0 {
            cles.push(0);
        }
        let debut_cles = 0x14 + index.len();
        let debut_donnees = debut_cles + cles.len();
        let mut o = Vec::new();
        o.extend(b"\0PSF");
        o.extend(0x0101u32.to_le_bytes());
        o.extend((debut_cles as u32).to_le_bytes());
        o.extend((debut_donnees as u32).to_le_bytes());
        o.extend((paires.len() as u32).to_le_bytes());
        o.extend(index);
        o.extend(cles);
        o.extend(donnees);
        o
    }

    fn zip_de(f: &Path, entrees: &[(&str, Vec<u8>)]) {
        let mut z = zip::ZipWriter::new(std::fs::File::create(f).unwrap());
        for (n, o) in entrees {
            z.start_file(*n, zip::write::SimpleFileOptions::default()).unwrap();
            std::io::Write::write_all(&mut z, o).unwrap();
        }
        z.finish().unwrap();
    }

    #[test]
    fn une_autre_version_ou_un_zip_a_plusieurs_jeux_repasse_par_le_fichier() {
        let d = vita_portable();
        let v = d.path();
        let app = ux0(v).join("app").join("PCSB00245").join("sce_sys");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(app.join("param.sfo"), sfo_version("PCSB00245", Some("01.00"))).unwrap();
        let maj = d.path().join("p4g 1.03.vpk");
        zip_de(&maj, &[("sce_sys/param.sfo", sfo_version("PCSB00245", Some("01.03")))]);
        assert_eq!(pret_a_lancer(v, &maj), None, "une autre version : installée par le fichier");
        // Installée (patch 1.03) : directement.
        let patch = ux0(v).join("patch").join("PCSB00245").join("sce_sys");
        std::fs::create_dir_all(&patch).unwrap();
        std::fs::write(patch.join("param.sfo"), sfo_version("PCSB00245", Some("01.03"))).unwrap();
        assert_eq!(pret_a_lancer(v, &maj).as_deref(), Some("PCSB00245"));
        // Un zip qui contient deux jeux : on ne devine pas.
        let deux = d.path().join("deux.zip");
        zip_de(&deux, &[("a/sce_sys/param.sfo", sfo("PCSB00245")), ("b/sce_sys/param.sfo", sfo("PCSE00120"))]);
        assert_eq!(titre_id(&deux), None);
        // `xsce_sys/param.sfo` n'est pas le param.sfo d'un jeu.
        let faux = d.path().join("faux.zip");
        zip_de(&faux, &[("xsce_sys/param.sfo", sfo("PCSB00245"))]);
        assert_eq!(titre_id(&faux), None);
    }

    #[test]
    fn un_jeu_deja_installe_se_lance_par_son_numero() {
        let d = vita_portable();
        let v = d.path();
        // Le .vpk (un zip) du jeu.
        let vpk = d.path().join("Persona 4 Golden (Europe).vpk");
        {
            let mut z = zip::ZipWriter::new(std::fs::File::create(&vpk).unwrap());
            z.start_file("sce_sys/param.sfo", zip::write::SimpleFileOptions::default()).unwrap();
            std::io::Write::write_all(&mut z, &sfo("PCSB00245")).unwrap();
            z.finish().unwrap();
        }
        assert_eq!(titre_id(&vpk).as_deref(), Some("PCSB00245"));
        assert_eq!(pret_a_lancer(v, &vpk), None, "pas encore installé");
        let app = ux0(v).join("app").join("PCSB00245").join("sce_sys");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(app.join("param.sfo"), sfo("PCSB00245")).unwrap();
        assert_eq!(pret_a_lancer(v, &vpk).as_deref(), Some("PCSB00245"));
        assert_eq!(titre_id(&ux0(v).join("app").join("PCSB00245")).as_deref(), Some("PCSB00245"), "un dossier aussi");
        let mut args = vec!["-F".to_string(), vpk.to_string_lossy().to_string()];
        lancer_par_numero(&mut args, &vpk, "PCSB00245");
        assert_eq!(args, ["-F", "-r", "PCSB00245"]);
        // Une ligne réglée à la main où le fichier est collé à autre chose : rien ne change.
        let mut args = vec![format!("--x={}", vpk.display())];
        lancer_par_numero(&mut args, &vpk, "PCSB00245");
        assert_eq!(args.len(), 1);
        // Un numéro qui n'en est pas un (il deviendrait un chemin) : aucun.
        std::fs::write(app.join("param.sfo"), sfo("../../x")).unwrap();
        assert_eq!(titre_id(&ux0(v).join("app").join("PCSB00245")), None);
    }

    #[test]
    fn les_parties_d_avant_vont_a_la_premiere_personne_et_jamais_par_dessus() {
        let d = vita_portable();
        let v = d.path();
        std::fs::create_dir_all(ux0(v).join("user").join("00")).unwrap();
        std::fs::write(ux0(v).join("user").join("00").join("user.xml"), user_xml("00", "Vita3K")).unwrap();
        for (jeu, o) in [("PCSB00001", &b"persona"[..]), ("PCSB00002", b"gravity")] {
            std::fs::create_dir_all(parties(v, "00").join(jeu)).unwrap();
            std::fs::write(parties(v, "00").join(jeu).join("sce_sys.bin"), o).unwrap();
        }
        let seb = utilisateur_du_profil(v, "Seb").unwrap();
        std::fs::create_dir_all(parties(v, &seb.id).join("PCSB00002")).unwrap();
        std::fs::write(parties(v, &seb.id).join("PCSB00002").join("sce_sys.bin"), b"gravity de Seb").unwrap();
        assert_eq!(reprendre_parties_d_avant(v, &seb).unwrap(), 1);
        assert_eq!(std::fs::read(parties(v, &seb.id).join("PCSB00001").join("sce_sys.bin")).unwrap(), b"persona");
        assert_eq!(std::fs::read(parties(v, &seb.id).join("PCSB00002").join("sce_sys.bin")).unwrap(), b"gravity de Seb");
        assert!(parties(v, "00").join("PCSB00001").join("sce_sys.bin").is_file(), "copiées, jamais déplacées");
    }

    #[test]
    fn apres_une_restauration_la_personne_suivante_ne_recoit_jamais_les_parties_d_une_autre() {
        let d = vita_portable();
        let v = d.path();
        let seb = utilisateur_du_profil(v, "Seb").unwrap();
        assert_eq!((seb.id.as_str(), seb.ancien_courant.clone()), ("00", None));
        std::fs::create_dir_all(parties(v, &seb.id).join("PCSB00001")).unwrap();
        choisir(v, &seb.id).unwrap();
        let lea = utilisateur_du_profil(v, "Léa").unwrap();
        assert_eq!(lea.ancien_courant, None);
        assert_eq!(reprendre_parties_d_avant(v, &lea).unwrap(), 0);
    }
}
