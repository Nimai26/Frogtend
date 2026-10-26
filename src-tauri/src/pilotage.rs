//! Le pilotage des émulateurs par le menu universel en jeu (lot 4 ter). Voir `docs/RECHERCHE-MENU-EN-JEU.md`.
//!
//! Avant chaque partie, Frogtend règle l'émulateur pour que :
//! - il se mette **en pause quand il perd le premier plan** : ouvrir le menu de Frogtend = la pause, le refermer = la
//!   reprise, et la manette ne pilote plus le jeu pendant ce temps ;
//! - il obéisse à des **touches que Frogtend lui envoie** : F13 à F16, qu'aucun clavier n'a (donc sans conflit) ;
//! - RetroArch écoute ses **commandes réseau** (port 55355). ⚠️ Relu le 16/10 dans RetroArch 1.22.2 : il n'a AUCUN
//!   réglage pour n'écouter que la machine (`network_cmd_bind_address` n'existe pas), le port s'ouvre sur tout le
//!   réseau et Windows peut demander d'autoriser l'accès. Décision de Seb (16/10) : garder ce comportement et prévenir
//!   la personne une fois (interface, `prevenirPareFeu`) — répondre « Annuler » ne gêne rien, Frogtend parle depuis
//!   la machine ;
//! - Dolphin ne soit pas en plein écran EXCLUSIF (sinon rien ne peut s'afficher par-dessus) ;
//! - quitter ne demande pas de confirmation (c'est le menu de Frogtend qui la demande).
//!
//! Relevé dans les sources officielles (30/09/2026), fichiers et lignes dans la recherche.

use crate::emulateurs_profils::modifier_ini;
use crate::erreurs::{Erreur, Resultat};
use std::path::{Path, PathBuf};

/// Le port des commandes réseau de RetroArch (son défaut, `network_cmd_port`).
pub const PORT_RETROARCH: u16 = 55355;

/// Une action du menu en jeu qu'on transmet à l'émulateur.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Reset,
    DisqueSuivant,
    SauverEtat,
    ChargerEtat,
    // Les options du jeu (lot OSD, i) : des bascules, sans état connu de Frogtend (l'émulateur le tient).
    /// Afficher ou masquer les images par seconde.
    ImagesParSeconde,
    /// Avance rapide (activer ou couper).
    AvanceRapide,
    /// Une capture d'écran (rangée par l'émulateur, dans SON dossier de captures).
    Capture,
    /// Couper ou remettre le son.
    Muet,
}

/// Une touche envoyée par Frogtend, avec ses modificateurs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Touche {
    pub touche: &'static str,
    pub maj: bool,
    pub ctrl: bool,
    pub alt: bool,
}

const fn t(touche: &'static str) -> Touche {
    Touche { touche, maj: false, ctrl: false, alt: false }
}

/// La touche que Frogtend envoie pour une action (DuckStation, PCSX2, Dolphin : réglées par Frogtend ; DOSBox
/// Staging : ses raccourcis d'origine).
pub fn touche(emulateur: &str, a: Action) -> Option<Touche> {
    match (emulateur, a) {
        ("duckstation" | "pcsx2" | "dolphin", Action::Reset) => Some(t("F13")),
        ("duckstation" | "dolphin", Action::DisqueSuivant) => Some(t("F14")),
        ("duckstation" | "pcsx2" | "dolphin", Action::SauverEtat) => Some(t("F15")),
        ("duckstation" | "pcsx2", Action::ChargerEtat) => Some(t("F16")),
        // Dolphin ne connaît que F13 à F15 (DInput) : Maj+F15.
        ("dolphin", Action::ChargerEtat) => Some(Touche { maj: true, ..t("F15") }),
        // DOSBox Staging (sdl_gui.cpp, bios_disk.cpp) : Ctrl+Alt+Début redémarre, Ctrl+F4 change de disque.
        ("dosbox-staging", Action::Reset) => Some(Touche { ctrl: true, alt: true, ..t("Home") }),
        ("dosbox-staging", Action::DisqueSuivant) => Some(Touche { ctrl: true, ..t("F4") }),
        _ => None,
    }
}

/// Ce que le menu peut faire dans cet émulateur (en plus de reprendre et de quitter, toujours possibles).
pub fn actions(emulateur: &str) -> Vec<Action> {
    use Action::*;
    match emulateur {
        // RetroArch : ses commandes réseau (command.h de la 1.22.2 : FPS_TOGGLE, FAST_FORWARD, SCREENSHOT, MUTE).
        "retroarch" => vec![Reset, DisqueSuivant, SauverEtat, ChargerEtat, ImagesParSeconde, AvanceRapide, Capture, Muet],
        "duckstation" | "dolphin" => vec![Reset, DisqueSuivant, SauverEtat, ChargerEtat],
        "pcsx2" => vec![Reset, SauverEtat, ChargerEtat],
        // MAME : par son script Lua (`COMMANDE_MAME`), seulement s'il est posé ; sauver/charger retirés pour un jeu qui ne
        // gère pas les sauvegardes d'état (`menu_jeu::etat`, ✅ Seb 19/10).
        "mame" => vec![Reset, SauverEtat, ChargerEtat],
        "dosbox-staging" => vec![Reset, DisqueSuivant],
        _ => vec![],
    }
}

/// L'émulateur se met-il en pause quand il perd le premier plan (réglé par `preparer` ; RPCS3 par
/// `commandes::preparer_emulateur`) ?
pub fn se_met_en_pause(emulateur: &str) -> bool {
    matches!(emulateur, "retroarch" | "duckstation" | "pcsx2" | "dolphin" | "ppsspp" | "dosbox-staging" | "rpcs3" | "mame" | "eden" | "azahar")
}

/// Les émulateurs SANS pause réglable (code source relu le 16/10 : Cemu, Xenia Canary, xemu, Vita3K — au mieux une
/// touche qui BASCULE la pause, et la manette lue en arrière-plan pour Cemu, Xenia et Vita3K) : FIGÉS d'office pendant
/// le menu (décision de Seb, 16/10 : « Oui, toujours »), comme un jeu PC dont l'option est cochée. Le menu ne dit « en
/// pause » qu'une fois le gel réussi (`menu-jeu-pause`).
pub fn fige_au_menu(emulateur: &str) -> bool {
    // mGBA (26/10) : sa pause en arrière-plan n'est pas posée par Frogtend, il est figé comme eux.
    matches!(emulateur, "cemu" | "xenia" | "xemu" | "vita3k" | "mgba")
}

/// Eden et Azahar : leur pause en arrière-plan est-elle VRAIMENT réglée (après `preparer`) ? Sinon (pas portable, ou
/// « pas de pause » choisi par la personne), ils sont figés pendant le menu comme ceux de `fige_au_menu`.
pub fn pause_reglee(id: &str, emulateur: &Path) -> bool {
    match id {
        "eden" | "azahar" => std::fs::read_to_string(emulateur.join("user").join("config").join("qt-config.ini"))
            .is_ok_and(|t| t.lines().any(|l| l.replace(' ', "") == "pauseWhenInBackground=true")),
        _ => true,
    }
}

/// MAME n'a pas de pause à la perte du premier plan (demande ouverte depuis 2019, mametesters 7339). Frogtend le
/// lance avec un petit script Lua (`-autoboot_script`, officiel) qui se met en pause tant qu'un fichier témoin
/// existe : Frogtend le crée à l'ouverture du menu, le retire à sa fermeture. Le script tourne aussi pendant la pause
/// (MAME 0.289 : `machine.cpp` appelle `frame_update` en pause, qui appelle les rappels périodiques, `video.cpp:253`).
/// Il ne relance QUE ce qu'il a mis en pause : une pause faite par la personne (touche P) reste — sauf après Sauver,
/// Charger ou Recommencer, que MAME fait toujours repartir (`resume()`), comme avec ses propres touches.
/// Chemins RELATIFS au dossier de MAME (son dossier de travail au lancement) : aucun souci d'accents dans un chemin.
pub const DOSSIER_MAME: &str = "frogtend";
pub const SCRIPT_MAME: &str = "frogtend\\menu-en-jeu.lua";
pub const TEMOIN_MAME: &str = "frogtend\\menu-ouvert";
/// Sauver / charger / recommencer (lot 0.79.0, idée de l'expert lancement 16/10) : une commande d'une ligne
/// (`<numéro> <save|load|reset> [chemin]`) que le script exécute une seule fois, puis efface. MAME 0.289
/// (`luaengine.cpp`) : `machine:save(nom)`, `:load(nom)`, `:soft_reset()` ; un nom ABSOLU est pris tel quel
/// (`machine.cpp`) ; les trois relancent MAME (`resume()`), et rien ne signale la fin : Frogtend vérifie le fichier.
pub const COMMANDE_MAME: &str = "frogtend\\commande";
/// Écrit par le script au démarrage : `<nom court>|<sauve|ne-sauve-pas>` (`system.supports_save`).
pub const JEU_MAME: &str = "frogtend\\jeu";

const LUA_MAME: &str = r#"-- Frogtend : la pause du menu en jeu. Écrit par Frogtend à chaque partie (ne pas modifier : il serait refait).
-- Tant que le fichier témoin existe (menu de Frogtend ouvert), MAME est en pause ; ensuite il repart, seulement si
-- c'est ce script qui l'avait mis en pause. Une commande de Frogtend (sauver, charger, recommencer) est exécutée une
-- seule fois, puis effacée.
-- Une seule fois, même si MAME relance le script à chaque remise à zéro (F3).
if frogtend_menu_en_jeu then return end
frogtend_menu_en_jeu = true
local temoin = "frogtend\\menu-ouvert"
local commande = "frogtend\\commande"
local par_frogtend = false
local apres_commande = false
local derniere = nil
local tours = 0
-- Ce jeu : son nom court, et s'il gère les sauvegardes d'état (sinon Frogtend ne les propose pas).
pcall(function()
  local s = manager.machine.system
  local f = io.open("frogtend\\jeu", "w")
  if f then
    f:write(s.name .. "|" .. (s.supports_save and "sauve" or "ne-sauve-pas"))
    f:close()
  end
end)
emu.register_periodic(function()
  tours = tours + 1
  if tours < 10 then return end
  tours = 0
  local m = manager.machine
  local c = io.open(commande, "r")
  if c then
    local ligne = c:read("*l") or ""
    c:close()
    os.remove(commande)
    pcall(function()
      local n, verbe, chemin = ligne:match("^(%d+) (%a+) ?(.*)$")
      if n and n ~= derniere then
        derniere = n
        if verbe == "save" and chemin ~= "" then m:save(chemin)
        elseif verbe == "load" and chemin ~= "" then m:load(chemin)
        elseif verbe == "reset" then m:soft_reset() end
        -- MAME repart seul : le menu se ferme, la pause n'est plus la nôtre.
        par_frogtend = false
        apres_commande = true
      end
    end)
  end
  local f = io.open(temoin, "r")
  local ouvert = f ~= nil
  if f then f:close() end
  -- Après une commande : jamais de nouvelle pause tant que le témoin du menu qui se ferme est encore là.
  if apres_commande then
    if not ouvert then apres_commande = false end
    return
  end
  if ouvert and not par_frogtend and not m.paused then
    emu.pause()
    par_frogtend = true
  elseif not ouvert and par_frogtend then
    if m.paused then emu.unpause() end
    par_frogtend = false
  end
end)
"#;

/// Prépare MAME pour le menu en jeu, dans SON dossier : le script (réécrit seulement s'il a changé), et rien de resté
/// d'une partie précédente (un témoin ferait démarrer en pause ; une vieille commande « charger » se jouerait).
pub fn preparer_mame(dossier_mame: &Path) -> Resultat<()> {
    std::fs::create_dir_all(dossier_mame.join(DOSSIER_MAME))?;
    let script = dossier_mame.join(SCRIPT_MAME);
    if std::fs::read_to_string(&script).ok().as_deref() != Some(LUA_MAME) {
        std::fs::write(&script, LUA_MAME)?;
    }
    for f in [TEMOIN_MAME, COMMANDE_MAME, JEU_MAME] {
        let _ = std::fs::remove_file(dossier_mame.join(f));
    }
    Ok(())
}

/// Ce que le script a dit du jeu : son nom court (vérifié : lettres, chiffres, `_`) et s'il gère les sauvegardes.
pub fn jeu_mame(dossier_mame: &Path) -> Option<(String, bool)> {
    let t = std::fs::read_to_string(dossier_mame.join(JEU_MAME)).ok()?;
    let (nom, sauve) = t.trim().split_once('|')?;
    let ok = !nom.is_empty() && nom.len() <= 32 && nom.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    ok.then(|| (nom.to_string(), sauve == "sauve"))
}

/// Le dossier des sauvegardes d'état d'un jeu de MAME pour un profil (dans `Profils\<profil>` : la sauvegarde du
/// profil le couvre ; jamais les emplacements de la personne, `sta\` ni « quick »).
pub fn dossier_etats_mame(dossier_mame: &Path, profil: &str, nom: &str) -> PathBuf {
    crate::emulateurs_profils::dossier_du_profil(dossier_mame, profil).join("sta").join(nom)
}

/// La sauvegarde rapide de Frogtend, et celle en cours d'écriture (MAME efface un fichier après une écriture ratée :
/// jamais sur la bonne).
pub const ETAT_MAME: &str = "frogtend.sta";
pub const ETAT_MAME_NOUVEAU: &str = "frogtend-nouveau.sta";
pub const ETAT_MAME_PRECEDENT: &str = "frogtend-precedent.sta";

/// Une commande pour le script de MAME : écrite à côté puis renommée (le script ne lit jamais une ligne à moitié
/// écrite), numérotée (exécutée une seule fois).
pub fn commande_mame(dossier_mame: &Path, verbe: &str, chemin: Option<&Path>) -> Resultat<()> {
    if !matches!(verbe, "save" | "load" | "reset") {
        return Err(Erreur::Refus(format!("Commande inconnue pour MAME : {verbe}.")));
    }
    let chemin = chemin.map(|c| c.to_string_lossy().to_string()).unwrap_or_default();
    if chemin.contains(['\n', '\r']) {
        return Err(Erreur::Refus("Chemin inattendu : rien n'est envoyé à MAME.".into()));
    }
    let numero = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let ligne = format!("{numero} {verbe} {chemin}").trim_end().to_string();
    let f = dossier_mame.join(COMMANDE_MAME);
    let tmp = dossier_mame.join(format!("{COMMANDE_MAME}.tmp"));
    std::fs::write(&tmp, format!("{ligne}\n"))?;
    // Le script peut tenir l'ancienne commande ouverte un instant : quelques essais.
    let mut essai = 0;
    loop {
        match std::fs::rename(&tmp, &f) {
            Ok(()) => return Ok(()),
            Err(_) if essai < 5 => {
                essai += 1;
                std::thread::sleep(std::time::Duration::from_millis(60));
            }
            Err(e) => {
                let _ = std::fs::remove_file(&tmp);
                return Err(e.into());
            }
        }
    }
}

/// Après une commande « save » : attend que MAME ait écrit la nouvelle sauvegarde en entier (présente, non vide, de
/// taille stable), puis la met à la place de l'ancienne (gardée en « précédente »). Rien d'écrit dans le délai : la
/// bonne sauvegarde n'est pas touchée.
pub fn garder_etat_mame(dossier: &Path, delai: std::time::Duration) -> Resultat<()> {
    let nouveau = dossier.join(ETAT_MAME_NOUVEAU);
    let debut = std::time::Instant::now();
    let mut taille = 0u64;
    loop {
        let t = std::fs::metadata(&nouveau).map(|m| m.len()).unwrap_or(0);
        if t > 0 && t == taille {
            break;
        }
        taille = t;
        if debut.elapsed() > delai {
            let _ = std::fs::remove_file(&nouveau);
            return Err(Erreur::Refus("MAME n'a pas pu sauver la partie : ta sauvegarde d'avant est intacte.".into()));
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    let actuel = dossier.join(ETAT_MAME);
    let precedent = dossier.join(ETAT_MAME_PRECEDENT);
    let avait = actuel.is_file();
    if avait {
        std::fs::rename(&actuel, &precedent)?;
    }
    if let Err(e) = std::fs::rename(&nouveau, &actuel) {
        // La nouvelle ne se pose pas (MAME la tient encore, antivirus) : la bonne reprend sa place, jamais perdue.
        if avait {
            let _ = std::fs::rename(&precedent, &actuel);
        }
        return Err(e.into());
    }
    Ok(())
}

/// Le menu s'ouvre (`true`) ou se ferme : le témoin que le script de MAME surveille.
pub fn menu_mame(dossier_mame: &Path, ouvert: bool) {
    let t = dossier_mame.join(TEMOIN_MAME);
    if ouvert {
        if t.parent().is_some_and(|d| d.is_dir()) {
            let _ = std::fs::write(&t, b"");
        }
    } else {
        // Le script peut tenir le témoin ouvert un instant (Windows refuse alors l'effacement) : quelques essais, sinon
        // le jeu resterait en pause.
        for _ in 0..5 {
            if std::fs::remove_file(&t).is_ok() || !t.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(40));
        }
    }
}

/// La commande réseau de RetroArch pour une action.
pub fn commande_retroarch(a: Action) -> &'static str {
    match a {
        Action::Reset => "RESET",
        Action::DisqueSuivant => "DISK_NEXT",
        Action::SauverEtat => "SAVE_STATE",
        Action::ChargerEtat => "LOAD_STATE",
        Action::ImagesParSeconde => "FPS_TOGGLE",
        Action::AvanceRapide => "FAST_FORWARD",
        Action::Capture => "SCREENSHOT",
        Action::Muet => "MUTE",
    }
}

/// Les lignes à ajouter au fichier de réglages de RetroArch (`--appendconfig`).
pub fn lignes_retroarch() -> String {
    format!(
        "network_cmd_enable = \"true\"\nnetwork_cmd_port = \"{PORT_RETROARCH}\"\npause_nonactive = \"true\"\ninput_menu_toggle_gamepad_combo = \"0\"\n"
    )
}

fn un(v: &str) -> Vec<String> {
    vec![v.to_string()]
}

/// Règle l'émulateur pour le menu en jeu. `utilisateur` : le dossier utilisateur Dolphin du profil. PPSSPP : ses
/// réglages sont dans `memstick`, qui mène déjà au profil (à appeler après `relier_memstick`).
pub fn preparer(id: &str, emulateur: &Path, utilisateur: Option<&Path>) -> Resultat<()> {
    let clavier = |t: &str| format!("Keyboard/{t}");
    match id {
        "duckstation" => {
            let ini = emulateur.join("settings.ini");
            modifier_ini(
                emulateur,
                &ini,
                "Main",
                &[("PauseOnFocusLoss", un("true")), ("DisableBackgroundInput", un("true")), ("ConfirmPowerOff", un("false"))],
            )?;
            modifier_ini(
                emulateur,
                &ini,
                "Hotkeys",
                &[
                    ("Reset", vec![clavier("F13")]),
                    ("SwitchToNextDisc", vec![clavier("F14")]),
                    ("SaveSelectedSaveState", vec![clavier("F15")]),
                    ("LoadSelectedSaveState", vec![clavier("F16")]),
                ],
            )?;
        }
        "pcsx2" => {
            let ini = emulateur.join("inis").join("PCSX2.ini");
            modifier_ini(emulateur, &ini, "UI", &[("PauseOnFocusLoss", un("true")), ("ConfirmShutdown", un("false"))])?;
            modifier_ini(
                emulateur,
                &ini,
                "Hotkeys",
                &[
                    ("ResetVM", vec![clavier("F13")]),
                    ("SaveStateToSlot1", vec![clavier("F15")]),
                    ("LoadStateFromSlot1", vec![clavier("F16")]),
                ],
            )?;
        }
        "dolphin" => {
            let Some(u) = utilisateur else { return Ok(()) };
            let config = u.join("Config");
            modifier_ini(emulateur, &config.join("Dolphin.ini"), "Interface", &[("PauseOnFocusLost", un("True")), ("ConfirmStop", un("False"))])?;
            modifier_ini(emulateur, &config.join("GFX.ini"), "Settings", &[("BorderlessFullscreen", un("True"))])?;
            let hotkeys = config.join("Hotkeys.ini");
            // Un Hotkeys.ini absent : Dolphin prendrait ses raccourcis d'origine ; en le créant, on les remet
            // (Échap = arrêter, F10 = pause, Alt+Entrée = plein écran) pour ne rien perdre.
            let mut v: Vec<(&str, Vec<String>)> = vec![
                ("Device", un("DInput/0/Keyboard Mouse")),
                ("General/Reset", un("F13")),
                ("General/Change Disc", un("F14")),
                ("Save State/Save to Selected Slot", un("F15")),
                ("Load State/Load from Selected Slot", un("@(Shift+F15)")),
            ];
            if !hotkeys.is_file() {
                v.extend([
                    ("General/Stop", un("ESCAPE")),
                    ("General/Toggle Pause", un("F10")),
                    ("General/Toggle Fullscreen", un("@(Alt+RETURN)")),
                ]);
            }
            modifier_ini(emulateur, &hotkeys, "Hotkeys", &v)?;
        }
        "ppsspp" => {
            let ini = emulateur.join("memstick").join("PSP").join("SYSTEM").join("ppsspp.ini");
            modifier_ini(emulateur, &ini, "General", &[("PauseOnLostFocus", un("True"))])?;
        }
        "dosbox-staging" => {
            let conf = emulateur.join("dosbox-staging.conf");
            modifier_ini(emulateur, &conf, "sdl", &[("pause_when_inactive", un("true"))])?;
        }
        // Eden et Azahar (Qt) : `[UI] pauseWhenInBackground` (Eden uisettings.h:113-119, main_window.cpp:1404-1420 ;
        // Azahar uisettings.h:92, citra_qt.cpp:1035-1051) — ils ne reprennent que s'ils avaient eux-mêmes mis en pause.
        // QSettings ignore la valeur tant que `\default` vaut true : les deux lignes. Seulement en mode portable
        // (dossier `user` déjà là) : le créer ferait basculer l'émulateur en portable, loin des réglages de la personne.
        "eden" | "azahar" => {
            let user = emulateur.join("user");
            let ini = user.join("config").join("qt-config.ini");
            // « Pas de pause » réglé par la personne elle-même (les deux lignes à false) : jamais défait (l'émulateur
            // sera figé à la place, voir `pause_reglee`).
            let choix_de_la_personne = std::fs::read_to_string(&ini).is_ok_and(|t| {
                let l: Vec<String> = t.lines().map(|l| l.replace(' ', "")).collect();
                l.iter().any(|x| x == "pauseWhenInBackground\\default=false") && l.iter().any(|x| x == "pauseWhenInBackground=false")
            });
            if user.is_dir() && !choix_de_la_personne {
                modifier_ini(
                    emulateur,
                    &ini,
                    "UI",
                    &[("pauseWhenInBackground\\default", un("false")), ("pauseWhenInBackground", un("true"))],
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    /// RÉEL : le vrai MAME (`FROGTEND_MAME` = son dossier, pas dans Temp) lance Pong (sans ROM) avec le script de
    /// Frogtend, plus un journal des pauses ; Frogtend crée puis retire le témoin. Attendu : « pause » puis « reprise ».
    /// `FROGTEND_MAME=<dossier> cargo test essai_pause_mame -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn essai_pause_mame() {
        let d = std::path::PathBuf::from(std::env::var("FROGTEND_MAME").unwrap());
        super::preparer_mame(&d).unwrap();
        let journal = d.join("frogtend").join("journal.txt");
        let _ = std::fs::remove_file(&journal);
        let essai = format!(
            "{}\nlocal function noter(t) local f = io.open(\"frogtend\\\\journal.txt\", \"a\") f:write(t .. \"\\n\") f:close() end\nfrogtend_essai = {{ emu.add_machine_pause_notifier(function() noter(\"pause\") end), emu.add_machine_resume_notifier(function() noter(\"reprise\") manager.machine:exit() end) }}\n",
            super::LUA_MAME
        );
        std::fs::write(d.join("frogtend").join("essai.lua"), essai).unwrap();
        let mut p = std::process::Command::new(d.join("mame.exe"))
            .args(["pong", "-window", "-nomax", "-skip_gameinfo", "-sound", "none", "-autoboot_script", "frogtend\\essai.lua"])
            .current_dir(&d)
            .spawn()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_secs(6));
        super::menu_mame(&d, true);
        std::thread::sleep(std::time::Duration::from_secs(3));
        println!("JOURNAL APRÈS OUVERTURE : {:?}", std::fs::read_to_string(&journal).unwrap_or_default());
        super::menu_mame(&d, false);
        let debut = std::time::Instant::now();
        let fin = loop {
            if let Some(s) = p.try_wait().unwrap() {
                break Some(s);
            }
            if debut.elapsed().as_secs() > 20 {
                let _ = p.kill();
                break None;
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        };
        println!("SORTIE {fin:?} ; JOURNAL FINAL : {:?}", std::fs::read_to_string(&journal).unwrap_or_default());
    }

    /// RÉEL (lot 0.79.0) : sauver, charger, recommencer par le VRAI script de Frogtend, dans un vrai MAME (Pong, sans
    /// ROM), avec un profil accentué. `FROGTEND_MAME=<dossier d'un MAME d'essai>` ; `FROGTEND_JEU_MAME` (pong par défaut).
    /// `FROGTEND_MAME=<dossier> cargo test essai_sauver_mame -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn essai_sauver_mame() {
        let d = std::path::PathBuf::from(std::env::var("FROGTEND_MAME").unwrap());
        let jeu = std::env::var("FROGTEND_JEU_MAME").unwrap_or_else(|_| "pong".into());
        super::preparer_mame(&d).unwrap();
        let mut p = std::process::Command::new(d.join("mame.exe"))
            .args([jeu.as_str(), "-window", "-nomax", "-skip_gameinfo", "-sound", "none", "-autoboot_script", super::SCRIPT_MAME])
            .current_dir(&d)
            .spawn()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_secs(6));
        let vivant = |p: &mut std::process::Child| p.try_wait().unwrap().is_none();
        let dit = super::jeu_mame(&d);
        println!("LE SCRIPT DIT : {dit:?}");
        let (nom, _) = dit.expect("le script n'a rien écrit");
        let etats = super::dossier_etats_mame(&d, "Sébastien", &nom);
        std::fs::create_dir_all(&etats).unwrap();
        // Sauver deux fois : la 2e prend la place, la 1re devient « précédente ».
        for i in 1..=2 {
            super::commande_mame(&d, "save", Some(&etats.join(super::ETAT_MAME_NOUVEAU))).unwrap();
            let r = super::garder_etat_mame(&etats, std::time::Duration::from_secs(5));
            println!("SAUVER {i} : {r:?} ; fichier {:?} octets ; MAME vivant {}", std::fs::metadata(etats.join(super::ETAT_MAME)).map(|m| m.len()).ok(), vivant(&mut p));
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
        println!("PRÉCÉDENTE : {:?} octets", std::fs::metadata(etats.join(super::ETAT_MAME_PRECEDENT)).map(|m| m.len()).ok());
        super::commande_mame(&d, "load", Some(&etats.join(super::ETAT_MAME))).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(2));
        println!("CHARGER : commande lue {} ; MAME vivant {}", !d.join(super::COMMANDE_MAME).exists(), vivant(&mut p));
        super::commande_mame(&d, "reset", None).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(3));
        println!("RECOMMENCER : commande lue {} ; MAME vivant {}", !d.join(super::COMMANDE_MAME).exists(), vivant(&mut p));
        // Après une remise à zéro, le script tourne encore (une 2e sauvegarde marche).
        super::commande_mame(&d, "save", Some(&etats.join(super::ETAT_MAME_NOUVEAU))).unwrap();
        println!("SAUVER APRÈS RECOMMENCER : {:?}", super::garder_etat_mame(&etats, std::time::Duration::from_secs(5)));
        let _ = p.kill();
        let _ = p.wait();
    }

    /// RÉEL : le chemin du menu — ouvert (pause), Sauver envoyé AVANT que le menu se ferme, puis fermé : MAME repart et
    /// ne se remet PAS en pause. Un journal Lua note chaque pause et reprise. `FROGTEND_MAME=<dossier>`.
    #[test]
    #[ignore]
    fn essai_menu_sauver_mame() {
        let d = std::path::PathBuf::from(std::env::var("FROGTEND_MAME").unwrap());
        super::preparer_mame(&d).unwrap();
        let journal = d.join("frogtend").join("journal.txt");
        let _ = std::fs::remove_file(&journal);
        let essai = format!(
            "{}\nlocal function noter(t) local f = io.open(\"frogtend\\\\journal.txt\", \"a\") f:write(t .. \"\\n\") f:close() end\nfrogtend_essai = {{ emu.add_machine_pause_notifier(function() noter(\"pause\") end), emu.add_machine_resume_notifier(function() noter(\"reprise\") end) }}\n",
            super::LUA_MAME
        );
        std::fs::write(d.join("frogtend").join("essai.lua"), essai).unwrap();
        let mut p = std::process::Command::new(d.join("mame.exe"))
            .args(["pong", "-window", "-nomax", "-skip_gameinfo", "-sound", "none", "-autoboot_script", "frogtend\\essai.lua"])
            .current_dir(&d)
            .spawn()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_secs(6));
        let (nom, _) = super::jeu_mame(&d).expect("le script n'a rien écrit");
        let etats = super::dossier_etats_mame(&d, "Sébastien", &nom);
        std::fs::create_dir_all(&etats).unwrap();
        // Le menu s'ouvre : pause.
        super::menu_mame(&d, true);
        std::thread::sleep(std::time::Duration::from_secs(2));
        let lire = || std::fs::read_to_string(&journal).unwrap_or_default().split_whitespace().map(String::from).collect::<Vec<_>>();
        println!("MENU OUVERT : {:?}", lire());
        // Sauver : la commande d'abord, puis le menu se ferme (comme `action_mame`).
        super::commande_mame(&d, "save", Some(&etats.join(super::ETAT_MAME_NOUVEAU))).unwrap();
        super::menu_mame(&d, false);
        println!("SAUVER : {:?}", super::garder_etat_mame(&etats, std::time::Duration::from_secs(5)));
        std::thread::sleep(std::time::Duration::from_secs(3));
        let fin = lire();
        println!("APRÈS : {fin:?}");
        let _ = p.kill();
        let _ = p.wait();
        assert_eq!(fin.last().map(String::as_str), Some("reprise"), "MAME doit tourner à la fin : {fin:?}");
        assert!(etats.join(super::ETAT_MAME).is_file());
    }

    /// RÉEL : la préparation du menu en jeu sur un vrai Eden ou Azahar portable (`FROGTEND_EMU` = id,
    /// `FROGTEND_DOSSIER` = son dossier). Vérifier ensuite que l'émulateur relit et réécrit la valeur. ⚠️ Il ÉCRIT dans
    /// ce dossier : jamais sur l'émulateur de la personne, seulement sur une COPIE d'essai.
    /// `FROGTEND_EMU=azahar FROGTEND_DOSSIER=<dossier> cargo test essai_pause_qt -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn essai_pause_qt() {
        let id = std::env::var("FROGTEND_EMU").unwrap();
        let d = std::path::PathBuf::from(std::env::var("FROGTEND_DOSSIER").unwrap());
        super::preparer(&id, &d, None).unwrap();
        let t = std::fs::read_to_string(d.join("user").join("config").join("qt-config.ini")).unwrap();
        println!("APRÈS FROGTEND : {:?}", t.lines().filter(|l| l.contains("pauseWhenInBackground")).collect::<Vec<_>>());
    }

    #[test]
    fn eden_et_azahar_se_mettent_en_pause_en_arriere_plan_seulement_en_portable() {
        for id in ["eden", "azahar"] {
            let d = tempfile::tempdir().unwrap();
            // Pas de dossier « user » : rien n'est créé (l'émulateur resterait près des réglages de la personne).
            super::preparer(id, d.path(), None).unwrap();
            assert!(!d.path().join("user").exists(), "{id}");
            // Portable : la pause est réglée, le reste du fichier gardé.
            let ini = d.path().join("user").join("config").join("qt-config.ini");
            std::fs::create_dir_all(ini.parent().unwrap()).unwrap();
            std::fs::write(&ini, "[UI]\nfullscreen\\default=false\nfullscreen=true\n").unwrap();
            super::preparer(id, d.path(), None).unwrap();
            let t = std::fs::read_to_string(&ini).unwrap();
            assert!(t.contains("pauseWhenInBackground\\default=false") || t.contains("pauseWhenInBackground\\default = false"), "{id} : {t}");
            assert!(t.contains("fullscreen=true") || t.contains("fullscreen = true"), "{id} : {t}");
            assert!(super::se_met_en_pause(id));
        }
        for id in ["cemu", "xenia", "xemu", "vita3k"] {
            assert!(super::fige_au_menu(id) && !super::se_met_en_pause(id), "{id}");
        }
        // Réglée seulement si le fichier le dit ; « pas de pause » choisi par la personne : jamais défait.
        let d = tempfile::tempdir().unwrap();
        assert!(!super::pause_reglee("eden", d.path()), "pas portable");
        let ini = d.path().join("user").join("config").join("qt-config.ini");
        std::fs::create_dir_all(ini.parent().unwrap()).unwrap();
        std::fs::write(&ini, "[UI]\npauseWhenInBackground\\default=false\npauseWhenInBackground=false\n").unwrap();
        super::preparer("eden", d.path(), None).unwrap();
        assert!(!super::pause_reglee("eden", d.path()), "le choix de la personne est gardé");
        std::fs::write(&ini, "[UI]\npauseWhenInBackground\\default=true\npauseWhenInBackground=false\n").unwrap();
        super::preparer("eden", d.path(), None).unwrap();
        assert!(super::pause_reglee("eden", d.path()), "la valeur d'origine de l'émulateur est remplacée");
        assert!(super::pause_reglee("retroarch", d.path()));
        assert!(!super::fige_au_menu("retroarch"));
    }

    #[test]
    fn mame_se_met_en_pause_par_son_script_et_son_temoin() {
        let d = tempfile::tempdir().unwrap();
        // Un témoin resté d'une partie précédente (Frogtend fermé de force) : retiré à la préparation.
        std::fs::create_dir_all(d.path().join(super::DOSSIER_MAME)).unwrap();
        std::fs::write(d.path().join(super::TEMOIN_MAME), b"").unwrap();
        super::preparer_mame(d.path()).unwrap();
        let script = std::fs::read_to_string(d.path().join(super::SCRIPT_MAME)).unwrap();
        assert!(script.contains("emu.pause()") && script.contains("emu.unpause()") && script.contains("par_frogtend"));
        assert!(script.contains(r#""frogtend\\menu-ouvert""#), "le même témoin que Frogtend, en chemin relatif");
        assert!(!d.path().join(super::TEMOIN_MAME).exists());
        super::menu_mame(d.path(), true);
        assert!(d.path().join(super::TEMOIN_MAME).exists());
        super::menu_mame(d.path(), false);
        assert!(!d.path().join(super::TEMOIN_MAME).exists());
        assert!(super::se_met_en_pause("mame"));
        // Sans dossier préparé (un MAME lancé autrement) : rien n'est créé.
        let vide = tempfile::tempdir().unwrap();
        super::menu_mame(vide.path(), true);
        assert!(!vide.path().join(super::TEMOIN_MAME).exists());
    }

    #[test]
    fn mame_sauve_charge_et_recommence_par_une_commande_de_son_script() {
        let d = tempfile::tempdir().unwrap();
        let mame = d.path();
        std::fs::create_dir_all(mame.join(super::DOSSIER_MAME)).unwrap();
        // Restes d'une partie précédente : une vieille commande « charger » ne se jouerait jamais au démarrage.
        std::fs::write(mame.join(super::COMMANDE_MAME), "1 load C:\\x.sta\n").unwrap();
        std::fs::write(mame.join(super::JEU_MAME), "pong|sauve").unwrap();
        super::preparer_mame(mame).unwrap();
        assert!(!mame.join(super::COMMANDE_MAME).exists() && !mame.join(super::JEU_MAME).exists());
        let script = std::fs::read_to_string(mame.join(super::SCRIPT_MAME)).unwrap();
        for attendu in ["m:save(chemin)", "m:load(chemin)", "m:soft_reset()", "supports_save", r#""frogtend\\commande""#, r#""frogtend\\jeu""#, "pcall"] {
            assert!(script.contains(attendu), "{attendu}");
        }
        // Ce que dit le script du jeu : nom court vérifié.
        assert_eq!(super::jeu_mame(mame), None);
        std::fs::write(mame.join(super::JEU_MAME), "sf2|sauve").unwrap();
        assert_eq!(super::jeu_mame(mame), Some(("sf2".into(), true)));
        std::fs::write(mame.join(super::JEU_MAME), "pong|ne-sauve-pas").unwrap();
        assert_eq!(super::jeu_mame(mame), Some(("pong".into(), false)));
        std::fs::write(mame.join(super::JEU_MAME), "..\\x|sauve").unwrap();
        assert_eq!(super::jeu_mame(mame), None);
        // La commande : une ligne numérotée, jamais un verbe inconnu ni un chemin sur deux lignes.
        let etats = super::dossier_etats_mame(mame, "Sébastien", "sf2");
        assert!(etats.starts_with(mame.join("Profils")) && etats.ends_with(std::path::Path::new("sta").join("sf2")), "{etats:?}");
        super::commande_mame(mame, "save", Some(&etats.join(super::ETAT_MAME_NOUVEAU))).unwrap();
        let ligne = std::fs::read_to_string(mame.join(super::COMMANDE_MAME)).unwrap();
        let (n, reste) = ligne.trim_end().split_once(' ').unwrap();
        assert!(n.chars().all(|c| c.is_ascii_digit()), "{ligne}");
        assert!(reste.starts_with("save ") && reste.ends_with("frogtend-nouveau.sta"), "{ligne}");
        assert!(!mame.join(format!("{}.tmp", super::COMMANDE_MAME)).exists());
        super::commande_mame(mame, "reset", None).unwrap();
        assert!(std::fs::read_to_string(mame.join(super::COMMANDE_MAME)).unwrap().trim_end().ends_with(" reset"));
        assert!(super::commande_mame(mame, "exit", None).is_err());
        assert!(super::commande_mame(mame, "save", Some(std::path::Path::new("a\nreset"))).is_err());
    }

    #[test]
    fn une_sauvegarde_de_mame_ratee_ne_touche_jamais_la_bonne() {
        let d = tempfile::tempdir().unwrap();
        let etats = d.path();
        std::fs::write(etats.join(super::ETAT_MAME), b"ancienne").unwrap();
        // Rien d'écrit par MAME dans le délai : la bonne reste.
        assert!(super::garder_etat_mame(etats, std::time::Duration::from_millis(300)).is_err());
        assert_eq!(std::fs::read(etats.join(super::ETAT_MAME)).unwrap(), b"ancienne");
        // Un fichier vide (écriture ratée) : pareil, et il est retiré.
        std::fs::write(etats.join(super::ETAT_MAME_NOUVEAU), b"").unwrap();
        assert!(super::garder_etat_mame(etats, std::time::Duration::from_millis(300)).is_err());
        assert!(!etats.join(super::ETAT_MAME_NOUVEAU).exists());
        assert_eq!(std::fs::read(etats.join(super::ETAT_MAME)).unwrap(), b"ancienne");
        // Écrite en entier : elle prend la place, l'ancienne devient « précédente ».
        std::fs::write(etats.join(super::ETAT_MAME_NOUVEAU), b"nouvelle").unwrap();
        super::garder_etat_mame(etats, std::time::Duration::from_secs(3)).unwrap();
        assert_eq!(std::fs::read(etats.join(super::ETAT_MAME)).unwrap(), b"nouvelle");
        assert_eq!(std::fs::read(etats.join(super::ETAT_MAME_PRECEDENT)).unwrap(), b"ancienne");
        assert!(!etats.join(super::ETAT_MAME_NOUVEAU).exists());
    }

    use super::*;
    use crate::manettes::lire_ini;

    #[test]
    fn duckstation_et_pcsx2_se_mettent_en_pause_et_ecoutent_f13_a_f16() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("settings.ini"), "[Main]\r\nSettingsVersion = 3\r\n\r\n[Hotkeys]\r\nTogglePause = Keyboard/Space\r\n").unwrap();
        preparer("duckstation", d.path(), None).unwrap();
        let t = std::fs::read_to_string(d.path().join("settings.ini")).unwrap();
        assert_eq!(lire_ini(&t, "Main", "PauseOnFocusLoss"), vec!["true"]);
        assert_eq!(lire_ini(&t, "Main", "DisableBackgroundInput"), vec!["true"]);
        assert_eq!(lire_ini(&t, "Hotkeys", "Reset"), vec!["Keyboard/F13"]);
        assert_eq!(lire_ini(&t, "Hotkeys", "TogglePause"), vec!["Keyboard/Space"], "les raccourcis de la personne restent");
        assert_eq!(lire_ini(&t, "Main", "SettingsVersion"), vec!["3"]);

        preparer("pcsx2", d.path(), None).unwrap();
        let t = std::fs::read_to_string(d.path().join("inis").join("PCSX2.ini")).unwrap();
        assert_eq!(lire_ini(&t, "UI", "PauseOnFocusLoss"), vec!["true"]);
        assert_eq!(lire_ini(&t, "Hotkeys", "LoadStateFromSlot1"), vec!["Keyboard/F16"]);
    }

    #[test]
    fn dolphin_passe_sans_bordure_et_garde_ses_raccourcis_d_origine() {
        let d = tempfile::tempdir().unwrap();
        let u = d.path().join("User");
        preparer("dolphin", d.path(), Some(&u)).unwrap();
        let c = u.join("Config");
        assert_eq!(lire_ini(&std::fs::read_to_string(c.join("GFX.ini")).unwrap(), "Settings", "BorderlessFullscreen"), vec!["True"]);
        assert_eq!(lire_ini(&std::fs::read_to_string(c.join("Dolphin.ini")).unwrap(), "Interface", "PauseOnFocusLost"), vec!["True"]);
        let h = std::fs::read_to_string(c.join("Hotkeys.ini")).unwrap();
        assert_eq!(lire_ini(&h, "Hotkeys", "General/Reset"), vec!["F13"]);
        assert_eq!(lire_ini(&h, "Hotkeys", "General/Stop"), vec!["ESCAPE"], "créé : les raccourcis d'origine sont remis");

        // Un Hotkeys.ini déjà là : on n'y remet pas nos défauts (la personne a peut-être changé Échap).
        std::fs::write(c.join("Hotkeys.ini"), "[Hotkeys]\r\nGeneral/Stop = Q\r\n").unwrap();
        preparer("dolphin", d.path(), Some(&u)).unwrap();
        let h = std::fs::read_to_string(c.join("Hotkeys.ini")).unwrap();
        assert_eq!(lire_ini(&h, "Hotkeys", "General/Stop"), vec!["Q"]);
    }

    #[test]
    fn ppsspp_et_dosbox_se_mettent_en_pause() {
        let d = tempfile::tempdir().unwrap();
        preparer("ppsspp", d.path(), None).unwrap();
        let t = std::fs::read_to_string(d.path().join("memstick/PSP/SYSTEM/ppsspp.ini")).unwrap();
        assert_eq!(lire_ini(&t, "General", "PauseOnLostFocus"), vec!["True"]);
        std::fs::write(d.path().join("dosbox-staging.conf"), "[sdl]\nfullscreen = true\n\n[cpu]\ncycles = auto\n").unwrap();
        preparer("dosbox-staging", d.path(), None).unwrap();
        let t = std::fs::read_to_string(d.path().join("dosbox-staging.conf")).unwrap();
        assert_eq!(lire_ini(&t, "sdl", "pause_when_inactive"), vec!["true"]);
        assert_eq!(lire_ini(&t, "cpu", "cycles"), vec!["auto"]);
    }

    #[test]
    fn chaque_action_a_son_moyen() {
        for e in ["retroarch", "duckstation", "pcsx2", "dolphin", "dosbox-staging"] {
            for a in actions(e) {
                if e == "retroarch" {
                    assert!(!commande_retroarch(a).is_empty());
                } else {
                    assert!(touche(e, a).is_some(), "{e} {a:?}");
                }
            }
        }
        assert!(!actions("pcsx2").contains(&Action::DisqueSuivant), "PCSX2 ne sait pas changer de disque de l'extérieur");
        // Aucun réglage que RetroArch ignorerait (il n'a pas de `network_cmd_bind_address`, relu dans la 1.22.2).
        assert!(!lignes_retroarch().contains("network_cmd_bind_address"));
    }
}
