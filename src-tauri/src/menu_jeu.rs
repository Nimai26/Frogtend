//! Le menu universel en jeu (lot 4 ter) : pendant une partie, une touche (Pause/Attn par défaut, réglable) ouvre la
//! fenêtre « menu-jeu » de Frogtend par-dessus le jeu. L'émulateur, réglé par `pilotage`, se met en pause en perdant
//! le premier plan. Le menu rend ensuite la main au jeu et lui transmet l'ordre choisi.
//!
//! Voir `docs/RECHERCHE-MENU-EN-JEU.md`.

use crate::erreurs::{Erreur, Resultat};
use crate::pilotage::Action;
use serde::Serialize;
use std::sync::Mutex;
use std::time::Duration;

/// La partie en cours, pour le menu.
#[derive(Clone, Debug, Serialize)]
pub struct PartieEnCours {
    pub jeu: i64,
    pub titre: String,
    pub plateforme: String,
    /// Le processus lancé par Frogtend (émulateur ou lanceur du jeu).
    pub pid: u32,
    /// L'émulateur, s'il est connu de Frogtend (`retroarch`, `duckstation`…).
    pub emulateur: Option<String>,
    /// Le dossier du programme de l'émulateur (aucun pour un jeu PC) : seul ce qui s'y trouve peut être arrêté de force
    /// après « Quitter ».
    pub dossier_emulateur: Option<String>,
    /// Jeu PC : figé pendant le menu (option du jeu, jamais avec un anti-triche) ; seuls les programmes de
    /// `dossier_jeu` le sont.
    pub figer: bool,
    pub dossier_jeu: Option<String>,
    /// Jeu PC : sa correspondance manette → clavier (bouton → touche), s'il en a une.
    pub clavier: Option<std::collections::BTreeMap<String, String>>,
    /// Manette → clavier : une touche tenue se répète comme sur un vrai clavier (réglage du jeu, oui par défaut).
    pub repeter_touches: bool,
    /// Émulateur figé pendant le menu (aucune pause réglable : Cemu, Xenia, xemu, Vita3K ; ou Eden/Azahar dont la
    /// pause n'a pas pu être réglée) : il n'est « en pause » qu'une fois VRAIMENT figé.
    pub figer_emulateur: bool,
    /// Le jeu lancé est une liste de disques (`.m3u`, la seule que RetroArch, DuckStation et Dolphin font défiler) :
    /// sinon « Disque suivant » n'est pas proposé (essai de Seb, 28/10 : il l'était pour une cartouche SNES).
    pub plusieurs_disques: bool,
    /// L'emplacement de sauvegarde rapide choisi dans le menu (0 à 9 ; RetroArch et MAME : ✅ Seb 28/10).
    pub emplacement: u8,
    /// RetroArch : son emplacement courant, tel que Frogtend l'a réglé (`state_slot = "0"` au lancement, puis les
    /// `STATE_SLOT_PLUS/MINUS` envoyés) — RetroArch ne sait pas sauver dans un emplacement précis.
    pub emplacement_ra: u8,
    /// RetroArch : le dossier des sauvegardes rapides du profil, et le nom du fichier lancé sans extension (pour
    /// vérifier qu'une sauvegarde est bien écrite, et garder l'ancienne).
    pub etats: Option<String>,
    pub nom_rom: Option<String>,
    /// Les touches que Frogtend envoie arrivent-elles à l'émulateur ? (Faux : Dolphin dont les raccourcis sont réglés
    /// sur une manette — le menu ne propose alors que Reprendre et Quitter.)
    pub raccourcis_clavier: bool,
    /// RetroArch en mode « hardcore » des succès (succès actifs ET hardcore) : il refuse les sauvegardes rapides — le
    /// menu ne les propose pas.
    pub etats_interdits: bool,
    /// RetroArch : les triches du jeu (son `.cht`, lu au lancement) et leur état, tenu par Frogtend (lot 3, ✅ Seb
    /// 28/10). Vide : aucun fichier. Proposées seulement si `triches_permises` (hors mode hardcore : une triche le
    /// ferait perdre pour la session — chaîne relevée dans RetroArch 1.22.2).
    pub triches: Vec<Triche>,
    pub triches_permises: bool,
    /// RetroArch : son index de triche courant, tel que Frogtend l'a amené (0 au chargement, NON vérifié en vrai ;
    /// puis les `CHEAT_INDEX_PLUS/MINUS` envoyés). Ses touches T / Y / U sont coupées par Frogtend.
    pub triche_ra: usize,
    /// RetroArch : le dossier des parties du profil (`savefile_directory`), pour les mettre à l'abri avant la
    /// PREMIÈRE triche de la partie ; et si c'est fait.
    pub parties: Option<String>,
    pub abri_triches: bool,
    /// RetroArch : le nom du cœur qui fait tourner le jeu, et celui du dossier du jeu lancé (pour montrer les boutons de
    /// la manette d'origine, et voir un remappage « par dossier » : lot 4d).
    pub coeur: Option<String>,
    pub dossier_rom: Option<String>,
}

/// Une triche d'un fichier `.cht` de RetroArch : son nom et si elle est active.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Triche {
    pub titre: String,
    pub active: bool,
}

/// Un `.cht` plus gros n'est pas lu (une base de codes n'en approche jamais : un fichier abîmé ou autre chose).
pub const CHT_MAX: u64 = 4 * 1024 * 1024;
/// Au-delà, les triches ne sont pas montrées (le menu se parcourt à la manette).
const TRICHES_MAX: usize = 500;

/// Les triches d'un `.cht` de RetroArch (`cheats = N`, `cheatK_desc`, `cheatK_enable`) ; tolère un fichier abîmé :
/// une triche sans nom s'appelle « Triche K », une valeur illisible vaut « non active ». `cheats` absent : rien.
pub fn lire_cht(texte: &str) -> Vec<Triche> {
    let mut valeurs = std::collections::HashMap::new();
    for l in texte.lines() {
        if let Some((c, v)) = l.split_once('=') {
            valeurs.insert(c.trim().to_string(), v.trim().trim_matches('"').to_string());
        }
    }
    let n = valeurs.get("cheats").and_then(|v| v.parse::<usize>().ok()).unwrap_or(0).min(TRICHES_MAX);
    (0..n)
        .map(|k| Triche {
            titre: valeurs.get(&format!("cheat{k}_desc")).filter(|d| !d.trim().is_empty()).cloned().unwrap_or_else(|| format!("Triche {}", k + 1)),
            active: valeurs.get(&format!("cheat{k}_enable")).is_some_and(|v| v.eq_ignore_ascii_case("true")),
        })
        .collect()
}

/// Lit le `.cht` d'un jeu s'il existe et reste raisonnable (sinon : aucune triche).
pub fn lire_fichier_cht(f: &std::path::Path) -> Vec<Triche> {
    match std::fs::metadata(f) {
        Ok(m) if m.is_file() && m.len() <= CHT_MAX => std::fs::read(f).map(|o| lire_cht(&String::from_utf8_lossy(&o))).unwrap_or_default(),
        _ => vec![],
    }
}

/// Le `.cht` que RetroArch charge seul au lancement : `<cheat_database_path>/<nom du cœur>/<nom du jeu lancé>.cht`.
pub fn fichier_cht(triches: &std::path::Path, coeur: &str, nom_rom: &str) -> std::path::PathBuf {
    triches.join(coeur).join(format!("{nom_rom}.cht"))
}

/// Les commandes pour amener l'index de triche de RetroArch de `depuis` à `vers`, pas à pas, sans compter sur un
/// passage de la dernière à la première (non vérifié).
pub fn pas_vers_triche(depuis: usize, vers: usize) -> Vec<&'static str> {
    if vers >= depuis {
        vec!["CHEAT_INDEX_PLUS"; vers - depuis]
    } else {
        vec!["CHEAT_INDEX_MINUS"; depuis - vers]
    }
}

/// Les extensions des parties chez RetroArch et ses cœurs (`.srm` de RetroArch, `.rtc` des horloges, et celles que
/// des cœurs écrivent eux-mêmes : N64, PS1, DS, Saturn, Sega CD…). Une liste FERMÉE : quand les parties sont rangées à
/// côté du jeu (`savefiles_in_content_dir`), le jeu lui-même (`.sfc`, `.chd`, `.iso`…) n'est jamais pris pour une
/// partie (relecture lancement, 28/10).
const EXTENSIONS_PARTIES: &[&str] = &["srm", "rtc", "sav", "ldci", "mcr", "eep", "fla", "sra", "mpk", "nv", "dsv", "brm", "bcr", "bkr", "ram"];

/// Les parties d'un jeu chez RetroArch (`<jeu>.srm`, `<jeu>.rtc`…), à la racine du dossier des parties et jusqu'à deux
/// niveaux de sous-dossiers (rangement par cœur, par contenu) — à mettre à l'abri avant la première triche.
pub fn parties_du_jeu(parties: &std::path::Path, nom_rom: &str) -> Vec<std::path::PathBuf> {
    let est_partie = |nom: &str| {
        nom.strip_prefix(nom_rom)
            .and_then(|r| r.strip_prefix('.'))
            .is_some_and(|ext| EXTENSIONS_PARTIES.iter().any(|e| ext.eq_ignore_ascii_case(e)))
    };
    let mut l = Vec::new();
    let mut dossiers = vec![(parties.to_path_buf(), 0)];
    while let Some((d, niveau)) = dossiers.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                if niveau < 2 {
                    dossiers.push((p, niveau + 1));
                }
            } else if p.file_name().and_then(|n| n.to_str()).is_some_and(est_partie) {
                l.push(p);
            }
        }
    }
    l
}

/// Les triches de RetroArch sont-elles permises ? Pas en mode hardcore des succès (succès actifs ET hardcore : une
/// triche le ferait perdre pour la session) — la même règle que les sauvegardes rapides. `retroarch.cfg` porte
/// toujours la clé hardcore (RetroArch y recopie tout) : seuls les succès actifs du profil la rendent effective
/// (relecture lancement, 28/10 : chez Seb, hardcore « true » et succès coupés).
pub fn triches_permises_retroarch(retroarch_cfg: &str, profil_cfg: &str) -> bool {
    !retroarch_hardcore(retroarch_cfg, profil_cfg)
}

/// Ce que Frogtend sait de RetroArch avant de lancer un jeu : où sont ses parties et sauvegardes rapides, le nom du
/// jeu lancé, ses triches et si elles sont permises.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PreparationRa {
    pub nom_rom: Option<String>,
    /// Le nom du dossier du jeu lancé (remappages « par dossier » de RetroArch).
    pub dossier_rom: Option<String>,
    pub etats: Option<std::path::PathBuf>,
    pub parties: Option<std::path::PathBuf>,
    pub etats_interdits: bool,
    pub triches: Vec<Triche>,
    /// Des triches cochées s'appliqueront au chargement : les parties du jeu ont été mises à l'abri avant.
    pub abri_fait: bool,
    /// Le `.cht` d'une autre version du même jeu a été repris pour celle-ci (dit au journal).
    pub cht_repris: Option<std::path::PathBuf>,
}

/// Avant le lancement par RetroArch (il lit le `.cht` en chargeant le jeu). `profil` : le dossier du profil
/// (`frogtend.cfg` y est déjà écrit pour cette partie) ; `fichier` : le jeu lancé ; `defaut` : le fichier de la version
/// d'habitude (une triche posée depuis la page Triches porte son nom). Les parties et sauvegardes rapides vont à côté
/// du jeu quand `savefiles_in_content_dir` / `savestates_in_content_dir` le disent. Des triches cochées (restées de la
/// partie d'avant) : les parties sont mises à l'abri AVANT (une copie ratée : `Err`, rien ne se lance).
pub fn preparer_ra(
    retroarch: &std::path::Path,
    profil: &std::path::Path,
    coeur: Option<&str>,
    fichier: Option<&std::path::Path>,
    defaut: Option<&std::path::Path>,
    quand: u64,
) -> Result<PreparationRa, String> {
    let principal = std::fs::read_to_string(retroarch.join("retroarch.cfg")).unwrap_or_default();
    let du_profil = std::fs::read_to_string(profil.join("frogtend.cfg")).unwrap_or_default();
    let a_cote = |cle: &str| cle_retroarch(&principal, &du_profil, cle).then(|| fichier.and_then(|f| f.parent()).map(|d| d.to_path_buf())).flatten();
    let nom_rom = fichier.and_then(|f| f.file_stem()).map(|s| s.to_string_lossy().to_string());
    let mut p = PreparationRa {
        etats: a_cote("savestates_in_content_dir").or_else(|| Some(profil.join("states"))),
        parties: a_cote("savefiles_in_content_dir").or_else(|| Some(profil.join("saves"))),
        etats_interdits: retroarch_hardcore(&principal, &du_profil),
        nom_rom: nom_rom.clone(),
        dossier_rom: fichier.and_then(|f| f.parent()).and_then(|d| d.file_name()).map(|s| s.to_string_lossy().to_string()),
        ..Default::default()
    };
    let (Some(c), Some(r)) = (coeur, nom_rom.as_deref()) else { return Ok(p) };
    let cht = fichier_cht(&profil.join("cheats"), c, r);
    // Une autre version du jeu lancée : le `.cht` posé pour la version d'habitude est repris sous son nom (seulement
    // s'il n'en a pas déjà un ; la ROM n'est jamais renommée) — TOUTES ses triches coupées : des codes d'une autre
    // région ne s'appliquent jamais sans que la personne les choisisse (relecture lancement, 28/10).
    if !cht.exists() {
        if let Some(d) = defaut.and_then(|f| f.file_stem()).map(|s| s.to_string_lossy().to_string()).filter(|d| d != r) {
            let source = fichier_cht(&profil.join("cheats"), c, &d);
            let texte = std::fs::metadata(&source).ok().filter(|m| m.is_file() && m.len() <= CHT_MAX).and_then(|_| std::fs::read(&source).ok());
            if let Some(o) = texte {
                if std::fs::write(&cht, triches_coupees(&String::from_utf8_lossy(&o))).is_ok() {
                    p.cht_repris = Some(source);
                }
            }
        }
    }
    p.triches = lire_fichier_cht(&cht);
    if !p.etats_interdits && p.triches.iter().any(|t| t.active) {
        mettre_a_l_abri_avant_triche(p.parties.as_deref(), p.etats.as_deref(), r, quand)?;
        p.abri_fait = true;
    }
    Ok(p)
}

/// Les parties et sauvegardes rapides du jeu copiées à côté (`<fichier>.avant-triche-<secondes>`) avant une triche :
/// jamais par-dessus une copie (un nom libre est cherché), pas de nouvelle copie si la dernière est identique (les
/// copies ne s'accumulent pas à chaque partie). Une copie ratée : celles faites sont retirées, `Err`.
pub fn mettre_a_l_abri_avant_triche(parties: Option<&std::path::Path>, etats: Option<&std::path::Path>, nom_rom: &str, quand: u64) -> Result<(), String> {
    let mut a_copier: Vec<std::path::PathBuf> = parties.map(|d| parties_du_jeu(d, nom_rom)).unwrap_or_default();
    if let Some(e) = etats {
        a_copier.extend(etats_du_jeu(e, nom_rom).into_iter().map(|(f, _)| f));
    }
    let mut faites = Vec::new();
    for f in &a_copier {
        if derniere_copie_identique(f) {
            continue;
        }
        let mut copie = std::path::PathBuf::from(format!("{}.avant-triche-{quand}", f.to_string_lossy()));
        let mut n = 1;
        while copie.exists() {
            copie = std::path::PathBuf::from(format!("{}.avant-triche-{quand}-{n}", f.to_string_lossy()));
            n += 1;
        }
        if let Err(e) = std::fs::copy(f, &copie) {
            for c in &faites {
                let _ = std::fs::remove_file(c);
            }
            return Err(format!("Ta partie n'a pas pu être mise à l'abri ({e}) : aucune triche n'a été activée."));
        }
        faites.push(copie);
    }
    Ok(())
}

/// Un `.cht` dont toutes les triches sont coupées (`cheatN_enable = "false"`), le reste tel quel.
pub fn triches_coupees(texte: &str) -> String {
    texte
        .lines()
        .map(|l| match l.split_once('=') {
            Some((c, _)) if c.trim().starts_with("cheat") && c.trim().ends_with("_enable") => format!("{} = \"false\"", c.trim()),
            _ => l.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

/// Au-delà, une copie n'est pas comparée (elle est refaite) : jamais de gros fichier lu pour rien.
const COMPARAISON_MAX: u64 = 64 * 1024 * 1024;

/// Deux fichiers ont-ils le même contenu ? Lus par blocs, arrêt à la première différence ; trop gros : non.
fn memes_octets(a: &std::path::Path, b: &std::path::Path) -> bool {
    use std::io::Read;
    let (Ok(ma), Ok(mb)) = (std::fs::metadata(a), std::fs::metadata(b)) else { return false };
    if ma.len() != mb.len() || ma.len() > COMPARAISON_MAX {
        return false;
    }
    let (Ok(mut fa), Ok(mut fb)) = (std::fs::File::open(a), std::fs::File::open(b)) else { return false };
    let (mut ba, mut bb) = (vec![0u8; 64 * 1024], vec![0u8; 64 * 1024]);
    loop {
        let Ok(na) = fa.read(&mut ba) else { return false };
        if na == 0 {
            return true;
        }
        if fb.read_exact(&mut bb[..na]).is_err() || ba[..na] != bb[..na] {
            return false;
        }
    }
}

/// La plus récente copie `.avant-triche-*` de ce fichier a-t-elle exactement le même contenu ?
fn derniere_copie_identique(f: &std::path::Path) -> bool {
    let (Some(dossier), Some(nom)) = (f.parent(), f.file_name().and_then(|n| n.to_str())) else { return false };
    let prefixe = format!("{nom}.avant-triche-");
    let derniere = std::fs::read_dir(dossier)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_name().to_str().is_some_and(|n| n.starts_with(&prefixe)))
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .max_by_key(|(t, _)| *t)
        .map(|(_, p)| p);
    derniere.is_some_and(|c| memes_octets(&c, f))
}

/// RetroArch interdit-il les sauvegardes rapides ? (`cheevos_enable` ET `cheevos_hardcore_mode_enable` : la valeur du
/// fichier de Frogtend pour le profil l'emporte sur celle de `retroarch.cfg`.)
pub fn retroarch_hardcore(retroarch_cfg: &str, profil_cfg: &str) -> bool {
    cle_retroarch(retroarch_cfg, profil_cfg, "cheevos_enable") && cle_retroarch(retroarch_cfg, profil_cfg, "cheevos_hardcore_mode_enable")
}

/// Un réglage oui/non de RetroArch : celui du fichier de Frogtend pour le profil l'emporte sur `retroarch.cfg` (la
/// dernière ligne d'un fichier compte) ; absent : non.
pub fn cle_retroarch(retroarch_cfg: &str, profil_cfg: &str, cle: &str) -> bool {
    let lire = |t: &str| {
        t.lines().rev().find_map(|l| {
            let (c, v) = l.split_once('=')?;
            (c.trim() == cle).then(|| v.trim().trim_matches('"').eq_ignore_ascii_case("true"))
        })
    };
    lire(profil_cfg).or_else(|| lire(retroarch_cfg)).unwrap_or(false)
}

/// Les emplacements de sauvegarde rapide proposés dans le menu (0 à 9).
pub const EMPLACEMENT_MAX: u8 = 9;

/// Le nom du fichier d'une sauvegarde rapide de RetroArch : `<jeu>.state` pour l'emplacement 0, `<jeu>.state<N>`
/// ensuite (relevé chez Seb : `states\Snes9x\Super Mario World (Europe) (Rev 1).state`).
pub fn nom_etat_retroarch(nom_rom: &str, n: u8) -> String {
    if n == 0 {
        format!("{nom_rom}.state")
    } else {
        format!("{nom_rom}.state{n}")
    }
}

/// Le fichier d'une sauvegarde rapide de RetroArch dans le dossier des états du profil (ou un sous-dossier par cœur,
/// `sort_savestates_enable`), s'il existe.
pub fn fichier_etat_retroarch(etats: &std::path::Path, nom_rom: &str, n: u8) -> Option<std::path::PathBuf> {
    let nom = nom_etat_retroarch(nom_rom, n);
    let direct = etats.join(&nom);
    if direct.is_file() {
        return Some(direct);
    }
    std::fs::read_dir(etats).ok()?.flatten().map(|e| e.path()).filter(|p| p.is_dir()).map(|d| d.join(&nom)).find(|p| p.is_file())
}

/// Les sauvegardes rapides d'un jeu : (emplacement, date de modification en secondes depuis 1970) — pour le menu
/// (« vide » ou « sauvé le … », et la question avant d'en remplacer une).
pub fn emplacements_retroarch(etats: &std::path::Path, nom_rom: &str) -> Vec<(u8, Option<u64>)> {
    (0..=EMPLACEMENT_MAX)
        .map(|n| (n, fichier_etat_retroarch(etats, nom_rom, n).and_then(|f| date_de(&f))))
        .collect()
}

/// TOUTES les sauvegardes rapides d'un jeu chez RetroArch (emplacements 0 à 9 et `.state.auto`), à la racine du
/// dossier des états et jusqu'à deux niveaux de sous-dossiers (rangement par cœur, par contenu) : (fichier, empreinte
/// = taille + date complète). Sert à les mettre TOUTES à l'abri avant une sauvegarde, puis à voir laquelle a changé
/// (RetroArch ne sait sauver que dans son emplacement courant, que la personne a pu changer : relecture de l'expert).
pub fn etats_du_jeu(etats: &std::path::Path, nom_rom: &str) -> Vec<(std::path::PathBuf, (u64, std::time::SystemTime))> {
    let est_etat = |nom: &str| {
        let Some(reste) = nom.strip_prefix(nom_rom).and_then(|r| r.strip_prefix(".state")) else { return false };
        reste.is_empty() || reste == ".auto" || (reste.len() <= 2 && reste.chars().all(|c| c.is_ascii_digit()))
    };
    let mut l = Vec::new();
    let mut dossiers = vec![(etats.to_path_buf(), 0)];
    while let Some((d, niveau)) = dossiers.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                if niveau < 2 {
                    dossiers.push((p, niveau + 1));
                }
            } else if p.file_name().and_then(|n| n.to_str()).is_some_and(est_etat) {
                if let Some(em) = empreinte(&p) {
                    l.push((p, em));
                }
            }
        }
    }
    l
}

/// Taille et date de modification complète (pas à la seconde : deux sauvegardes rapprochées se distinguent).
pub fn empreinte(f: &std::path::Path) -> Option<(u64, std::time::SystemTime)> {
    let m = std::fs::metadata(f).ok()?;
    Some((m.len(), m.modified().ok()?))
}

/// L'emplacement d'un fichier de sauvegarde rapide de RetroArch (`None` : `.state.auto` ou autre chose).
pub fn emplacement_du_fichier(f: &std::path::Path, nom_rom: &str) -> Option<u8> {
    let reste = f.file_name()?.to_str()?.strip_prefix(nom_rom)?.strip_prefix(".state")?;
    if reste.is_empty() {
        Some(0)
    } else {
        reste.parse::<u8>().ok().filter(|n| *n <= EMPLACEMENT_MAX)
    }
}

pub fn date_de(f: &std::path::Path) -> Option<u64> {
    std::fs::metadata(f).ok()?.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_secs())
}

/// Le fichier lancé peut-il avoir plusieurs disques ? Une liste `.m3u`, ou un `.pbp` (PS1 : plusieurs disques dans un
/// seul fichier, que RetroArch et DuckStation font défiler sans liste).
pub fn liste_de_disques(fichier: &std::path::Path) -> bool {
    fichier.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("m3u") || e.eq_ignore_ascii_case("pbp"))
}

/// Les systèmes à disquettes dont on RETOURNE la face par « Disque suivant » (commande `DISK_NEXT` de RetroArch) :
/// toujours proposé (non vérifié sur un vrai cœur FDS).
const SYSTEMES_A_FACES: &[&str] = &["Nintendo Famicom Disk System"];

#[derive(Default)]
pub struct MenuJeu {
    pub partie: Mutex<Option<PartieEnCours>>,
    /// Le numéro de la partie en cours : la lecture de la manette d'une partie s'arrête quand il change.
    pub numero: std::sync::atomic::AtomicU64,
    /// La fenêtre du menu est-elle ouverte ? Tenu par `ouvrir_menu` et `cacher_menu` (la lecture de la manette le
    /// consulte 60 fois par seconde sans interroger la fenêtre).
    pub ouvert: std::sync::atomic::AtomicBool,
    /// Une sauvegarde ou un chargement rapide en cours (voir `prendre_etat`).
    pub etat_en_cours: std::sync::atomic::AtomicBool,
}

/// Rendue à la fin de la sauvegarde ou du chargement, même après une erreur.
pub struct EtatEnCours<'a>(&'a MenuJeu);

impl Drop for EtatEnCours<'_> {
    fn drop(&mut self) {
        self.0.etat_en_cours.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

impl MenuJeu {
    pub fn partie(&self) -> Option<PartieEnCours> {
        self.partie.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
    /// Retient la partie et rend son numéro.
    pub fn commencer(&self, p: PartieEnCours) -> u64 {
        *self.partie.lock().unwrap_or_else(|e| e.into_inner()) = Some(p);
        self.numero.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1
    }
    /// Oublie la partie `numero` (une partie plus récente n'est jamais effacée par la fin d'une ancienne).
    pub fn finir(&self, numero: u64) {
        if self.numero.load(std::sync::atomic::Ordering::SeqCst) == numero {
            *self.partie.lock().unwrap_or_else(|e| e.into_inner()) = None;
        }
    }
    /// Change l'emplacement de sauvegarde rapide choisi (−1 ou +1, de 0 à 9) ; rend le nouveau.
    pub fn changer_emplacement(&self, sens: i8) -> Option<u8> {
        let mut g = self.partie.lock().unwrap_or_else(|e| e.into_inner());
        let p = g.as_mut()?;
        // En boucle (de 9 on passe à 0 et inversement) : jamais un bouton qui se grise sous la manette.
        p.emplacement = (p.emplacement as i16 + sens as i16).rem_euclid(EMPLACEMENT_MAX as i16 + 1) as u8;
        Some(p.emplacement)
    }
    /// Prend la main pour UNE sauvegarde / un chargement à la fois (le menu et un raccourci manette ne se chevauchent
    /// jamais) ; `None` : une autre est en cours.
    pub fn prendre_etat(&self) -> Option<EtatEnCours<'_>> {
        (!self.etat_en_cours.swap(true, std::sync::atomic::Ordering::SeqCst)).then_some(EtatEnCours(self))
    }
    /// RetroArch : son index de triche courant, après chaque commande envoyée.
    pub fn noter_triche_ra(&self, i: usize) {
        if let Some(p) = self.partie.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            p.triche_ra = i;
        }
    }
    /// Une triche basculée (après le `CHEAT_TOGGLE` envoyé).
    pub fn basculer_triche(&self, k: usize) {
        if let Some(t) = self.partie.lock().unwrap_or_else(|e| e.into_inner()).as_mut().and_then(|p| p.triches.get_mut(k)) {
            t.active = !t.active;
        }
    }
    /// Les parties du jeu sont à l'abri pour cette partie (avant la première triche).
    pub fn noter_abri_triches(&self) {
        if let Some(p) = self.partie.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            p.abri_triches = true;
        }
    }
    /// RetroArch : son emplacement courant, après les commandes envoyées.
    pub fn noter_emplacement_ra(&self, n: u8) {
        if let Some(p) = self.partie.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            p.emplacement_ra = n;
        }
    }
    /// La partie `numero` est-elle toujours celle en cours ?
    pub fn en_cours(&self, numero: u64) -> bool {
        self.numero.load(std::sync::atomic::Ordering::SeqCst) == numero && self.partie().is_some()
    }
}

/// Ce que le menu montre : la partie et les actions possibles.
#[derive(Debug, Serialize, PartialEq)]
pub struct EtatMenu {
    pub jeu: i64,
    pub titre: String,
    pub plateforme: String,
    pub emulateur: Option<String>,
    /// Actions possibles en plus de reprendre et quitter : `reset`, `disque`, `sauver`, `charger` ; options du jeu :
    /// `fps`, `rapide`, `capture`, `muet` (`pilotage::actions`).
    pub actions: Vec<&'static str>,
    /// L'émulateur se met-il en pause quand le menu s'ouvre ? (Non pour un jeu PC.)
    pub en_pause: bool,
    /// Ouvert en Taodbox (fenêtre principale en plein écran) : le menu est à l'échelle ×2. Rempli par la commande.
    pub taodbox: bool,
    /// La fiche Firehouse que montre ce jeu pour le profil ouvert (un import relié : celle de Firehouse ; sinon
    /// `None`) — pour son fond. Rempli par la commande.
    pub fiche: Option<i64>,
    /// L'emplacement de sauvegarde rapide choisi, quand l'émulateur sait en changer depuis Frogtend (RetroArch, MAME) ;
    /// `None` : celui de l'émulateur (non choisi ici).
    pub emplacement: Option<u8>,
    /// Les triches du jeu, quand Frogtend sait les activer pendant la partie (RetroArch hors mode hardcore) ; une liste
    /// vide : aucune triche posée pour ce jeu ; `None` : pas de triches depuis le menu.
    pub triches: Option<Vec<Triche>>,
    /// Les parties du jeu sont-elles déjà à l'abri pour CETTE partie ? (Sinon la page met en garde avant la première
    /// triche ; la fenêtre du menu est réutilisée d'une partie à l'autre : c'est le cœur qui sait.)
    pub abri_triches: bool,
}

pub fn nom_action(a: Action) -> &'static str {
    match a {
        Action::Reset => "reset",
        Action::DisqueSuivant => "disque",
        Action::SauverEtat => "sauver",
        Action::ChargerEtat => "charger",
        Action::ImagesParSeconde => "fps",
        Action::AvanceRapide => "rapide",
        Action::Capture => "capture",
        Action::Muet => "muet",
    }
}

pub fn action_de(nom: &str) -> Option<Action> {
    match nom {
        "reset" => Some(Action::Reset),
        "disque" => Some(Action::DisqueSuivant),
        "sauver" => Some(Action::SauverEtat),
        "charger" => Some(Action::ChargerEtat),
        "fps" => Some(Action::ImagesParSeconde),
        "rapide" => Some(Action::AvanceRapide),
        "capture" => Some(Action::Capture),
        "muet" => Some(Action::Muet),
        _ => None,
    }
}

pub fn etat(p: &PartieEnCours) -> EtatMenu {
    let e = p.emulateur.as_deref().unwrap_or("");
    EtatMenu {
        fiche: None,
        jeu: p.jeu,
        titre: p.titre.clone(),
        plateforme: p.plateforme.clone(),
        emulateur: p.emulateur.clone(),
        actions: crate::pilotage::actions(e)
            .into_iter()
            // Un jeu de MAME qui ne gère pas les sauvegardes d'état (dit par le script au démarrage) : ni sauver ni
            // charger — une sauvegarde peu fiable serait une partie perdue (✅ Seb 19/10).
            // Des touches qui n'arriveraient pas (Dolphin, raccourcis sur une manette) : rien à proposer.
            .filter(|_| p.raccourcis_clavier || e == "retroarch" || e == "mame")
            // Le mode « hardcore » des succès : pas de sauvegarde rapide (RetroArch les refuserait).
            .filter(|a| !p.etats_interdits || !matches!(a, crate::pilotage::Action::SauverEtat | crate::pilotage::Action::ChargerEtat))
            // DOSBox Staging change de disque par sa propre configuration : gardé tel quel.
            .filter(|a| {
                !matches!(a, crate::pilotage::Action::DisqueSuivant) || p.plusieurs_disques || e == "dosbox-staging" || SYSTEMES_A_FACES.contains(&p.plateforme.as_str())
            })
            .filter(|a| {
                e != "mame"
                    || !matches!(a, crate::pilotage::Action::SauverEtat | crate::pilotage::Action::ChargerEtat)
                    || p.dossier_emulateur.as_deref().and_then(|d| crate::pilotage::jeu_mame(std::path::Path::new(d))).is_some_and(|(_, sauve)| sauve)
            })
            .map(nom_action)
            .collect(),
        en_pause: crate::pilotage::se_met_en_pause(e) && !p.figer_emulateur,
        taodbox: false,
        emplacement: matches!(e, "retroarch" | "mame").then_some(p.emplacement),
        triches: (e == "retroarch" && p.triches_permises && !p.etats_interdits).then(|| p.triches.clone()),
        abri_triches: p.abri_triches,
    }
}

/// Envoie une commande réseau à RetroArch (127.0.0.1, voir `pilotage::lignes_retroarch`).
pub fn commande_retroarch(commande: &str) -> Resultat<()> {
    let s = std::net::UdpSocket::bind("127.0.0.1:0").map_err(|e| Erreur::Disque(format!("Commande impossible ({e}).")))?;
    s.send_to(format!("{commande}\n").as_bytes(), ("127.0.0.1", crate::pilotage::PORT_RETROARCH))
        .map_err(|e| Erreur::Disque(format!("RetroArch ne répond pas ({e}).")))?;
    Ok(())
}

#[cfg(windows)]
mod fenetres {
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_EXTENDEDKEY,
        KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, MOUSEEVENTF_MOVE, MOUSEINPUT, VIRTUAL_KEY,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetForegroundWindow, GetWindow, GetWindowThreadProcessId, IsIconic, IsWindowVisible, PostMessageW,
        SetForegroundWindow, ShowWindow, GW_OWNER, SW_RESTORE, WM_CLOSE,
    };

    /// Une entrée neutre (la souris « bouge » de 0) : Windows laisse alors notre processus prendre le premier plan
    /// (SetForegroundWindow l'accepte quand « le processus a reçu le dernier événement d'entrée »). Sans elle, un menu
    /// ouvert depuis la manette resterait derrière le jeu (pas de pause, la manette pilote encore le jeu).
    pub fn entree_neutre() {
        let i = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 { mi: MOUSEINPUT { dx: 0, dy: 0, mouseData: 0, dwFlags: MOUSEEVENTF_MOVE, time: 0, dwExtraInfo: 0 } },
        };
        unsafe {
            SendInput(&[i], std::mem::size_of::<INPUT>() as i32);
        }
    }

    /// Ce processus a-t-il abandonné : plus AUCUNE fenêtre visible ? (Une fenêtre qui ne répond plus peut être un
    /// émulateur en pleine écriture d'une sauvegarde : on n'y touche pas — expert, 05/10.)
    pub fn abandonne(pid: u32) -> bool {
        du_processus(&[pid]).is_empty()
    }

    /// La fenêtre au premier plan appartient-elle à Frogtend ?
    pub fn premier_plan_a_frogtend() -> bool {
        let mut pid = 0u32;
        unsafe {
            GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut pid));
        }
        pid == std::process::id()
    }

    /// Les fenêtres principales (visibles, sans propriétaire) de ces processus.
    pub fn du_processus(pids: &[u32]) -> Vec<HWND> {
        struct Ctx<'a> {
            pids: &'a [u32],
            trouvees: Vec<HWND>,
        }
        unsafe extern "system" fn une(h: HWND, l: LPARAM) -> BOOL {
            let ctx = &mut *(l.0 as *mut Ctx);
            let mut pid = 0u32;
            GetWindowThreadProcessId(h, Some(&mut pid));
            let principale = GetWindow(h, GW_OWNER).map(|o| o.0.is_null()).unwrap_or(true);
            if ctx.pids.contains(&pid) && IsWindowVisible(h).as_bool() && principale {
                ctx.trouvees.push(h);
            }
            BOOL(1)
        }
        let mut ctx = Ctx { pids, trouvees: Vec::new() };
        unsafe {
            let _ = EnumWindows(Some(une), LPARAM(&mut ctx as *mut _ as isize));
        }
        ctx.trouvees
    }

    /// Le processus qui porte la fenêtre principale du jeu (celui sur lequel brancher Cheat Engine).
    pub fn pid_de_la_fenetre(pids: &[u32]) -> Option<u32> {
        // Jamais Frogtend ni ce qui l'a lancé : Cheat Engine se branche sur le JEU.
        let h = du_processus(&crate::lancement::sauf_frogtend(pids)).into_iter().next()?;
        let mut pid = 0u32;
        unsafe {
            GetWindowThreadProcessId(h, Some(&mut pid));
        }
        (pid != 0).then_some(pid)
    }

    /// Rend le premier plan au jeu. Rend `false` si aucune fenêtre n'a été trouvée.
    pub fn au_premier_plan(pids: &[u32]) -> bool {
        let Some(h) = du_processus(&crate::lancement::sauf_frogtend(pids)).into_iter().next() else { return false };
        unsafe {
            if IsIconic(h).as_bool() {
                let _ = ShowWindow(h, SW_RESTORE);
            }
            SetForegroundWindow(h).as_bool()
        }
    }

    /// Demande poliment aux fenêtres du jeu de se fermer (comme la croix) : l'émulateur enregistre ce qu'il doit.
    pub fn fermer(pids: &[u32]) -> usize {
        // Jamais les fenêtres de Frogtend ni de ce qui l'a lancé : seulement celles de la partie.
        let l = du_processus(&crate::lancement::sauf_frogtend(pids));
        for h in &l {
            unsafe {
                let _ = PostMessageW(Some(*h), WM_CLOSE, WPARAM(0), LPARAM(0));
            }
        }
        l.len()
    }

    fn cle(scan: u16, etendue: bool, relachee: bool) -> INPUT {
        let mut f: KEYBD_EVENT_FLAGS = KEYEVENTF_SCANCODE;
        if etendue {
            f |= KEYEVENTF_EXTENDEDKEY;
        }
        if relachee {
            f |= KEYEVENTF_KEYUP;
        }
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VIRTUAL_KEY(0), wScan: scan, dwFlags: f, time: 0, dwExtraInfo: 0 } },
        }
    }

    /// Appuie sur une touche (codes de balayage : DirectInput les lit, comme Qt) avec ses modificateurs, en la tenant
    /// un instant (certains émulateurs agissent au relâchement, à la trame suivante).
    pub fn touche(scan: u16, etendue: bool, modificateurs: &[u16]) {
        let mut bas: Vec<INPUT> = modificateurs.iter().map(|m| cle(*m, false, false)).collect();
        bas.push(cle(scan, etendue, false));
        let mut haut = vec![cle(scan, etendue, true)];
        haut.extend(modificateurs.iter().rev().map(|m| cle(*m, false, true)));
        unsafe {
            SendInput(&bas, std::mem::size_of::<INPUT>() as i32);
        }
        std::thread::sleep(std::time::Duration::from_millis(80));
        unsafe {
            SendInput(&haut, std::mem::size_of::<INPUT>() as i32);
        }
    }
}

/// Les codes de balayage (jeu 1) des touches envoyées.
fn code(nom: &str) -> Option<(u16, bool)> {
    Some(match nom {
        "F13" => (0x64, false),
        "F14" => (0x65, false),
        "F15" => (0x66, false),
        "F16" => (0x67, false),
        "F4" => (0x3E, false),
        "Home" => (0x47, true),
        _ => return None,
    })
}

const MAJ: u16 = 0x2A;
const CTRL: u16 = 0x1D;
const ALT: u16 = 0x38;

/// Le processus du jeu en cours qui a la fenêtre (sinon le premier lancé, s'il vit encore). Pour un jeu PC
/// (`dossier_jeu`), aussi les programmes de SON dossier : un jeu lancé par sa boutique, ou dont le premier programme
/// s'est fermé en relançant le vrai jeu, n'est pas un descendant de ce que Frogtend a lancé.
pub fn processus_du_jeu(racine: u32, dossier_jeu: Option<&std::path::Path>) -> u32 {
    let mut pids = crate::lancement::processus_de_la_partie(racine);
    if let Some(d) = dossier_jeu {
        for p in crate::lancement::processus_du_dossier(d) {
            if !pids.contains(&p) {
                pids.push(p);
            }
        }
    }
    #[cfg(windows)]
    if let Some(p) = fenetres::pid_de_la_fenetre(&pids) {
        return p;
    }
    if pids.contains(&racine) {
        racine
    } else {
        pids.first().copied().unwrap_or(racine)
    }
}

/// Prépare la prise du premier plan par Frogtend (menu ouvert depuis la manette) : une entrée neutre.
pub fn autoriser_premier_plan() {
    #[cfg(windows)]
    fenetres::entree_neutre();
}

/// Ce processus de l'émulateur a-t-il abandonné (plus aucune fenêtre visible) ? (`false` hors Windows.)
pub fn abandonne(pid: u32) -> bool {
    #[cfg(windows)]
    {
        fenetres::abandonne(pid)
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
        false
    }
}

/// Appuie sur une touche (code de balayage) avec ses modificateurs, au premier plan actuel.
pub fn envoyer_touche(scan: u16, etendue: bool, modificateurs: &[u16]) {
    #[cfg(windows)]
    fenetres::touche(scan, etendue, modificateurs);
    #[cfg(not(windows))]
    let _ = (scan, etendue, modificateurs);
}

/// Les fenêtres principales visibles de ces processus (pour le plein écran au lancement).
#[cfg(windows)]
pub fn fenetres_du_processus(pids: &[u32]) -> Vec<windows::Win32::Foundation::HWND> {
    fenetres::du_processus(pids)
}

/// La fenêtre au premier plan est-elle à Frogtend ? (`true` hors Windows.)
pub fn frogtend_au_premier_plan() -> bool {
    #[cfg(windows)]
    {
        fenetres::premier_plan_a_frogtend()
    }
    #[cfg(not(windows))]
    {
        true
    }
}

/// Rend le premier plan au jeu (reprise : l'émulateur sort seul de la pause).
pub fn reprendre(pids: &[u32]) -> bool {
    #[cfg(windows)]
    {
        fenetres::au_premier_plan(pids)
    }
    #[cfg(not(windows))]
    {
        let _ = pids;
        false
    }
}

/// Transmet une action à l'émulateur : le jeu reprend le premier plan, puis reçoit l'ordre.
pub fn agir(emulateur: &str, pids: &[u32], a: Action) -> Resultat<()> {
    if emulateur == "retroarch" {
        reprendre(pids);
        std::thread::sleep(Duration::from_millis(150));
        return commande_retroarch(crate::pilotage::commande_retroarch(a));
    }
    let t = crate::pilotage::touche(emulateur, a)
        .ok_or_else(|| Erreur::Refus("Cet émulateur ne sait pas le faire depuis le menu.".into()))?;
    let (scan, etendue) = code(t.touche).ok_or_else(|| Erreur::Refus(format!("Touche inconnue : {}.", t.touche)))?;
    if !reprendre(pids) {
        return Err(Erreur::Introuvable("La fenêtre du jeu est introuvable.".into()));
    }
    // Le temps que l'émulateur reprenne le premier plan et sorte de sa pause.
    std::thread::sleep(Duration::from_millis(250));
    let mut m = Vec::new();
    if t.ctrl {
        m.push(CTRL);
    }
    if t.alt {
        m.push(ALT);
    }
    if t.maj {
        m.push(MAJ);
    }
    #[cfg(windows)]
    fenetres::touche(scan, etendue, &m);
    #[cfg(not(windows))]
    let _ = (scan, etendue, m);
    Ok(())
}

/// Quitte proprement : la commande de RetroArch, sinon la fermeture de la fenêtre (comme la croix). Rend le nombre
/// de fenêtres fermées (0 : rien trouvé).
pub fn quitter(emulateur: Option<&str>, pids: &[u32]) -> Resultat<usize> {
    if emulateur == Some("retroarch") && commande_retroarch("QUIT").is_ok() {
        return Ok(1);
    }
    #[cfg(windows)]
    {
        Ok(fenetres::fermer(pids))
    }
    #[cfg(not(windows))]
    {
        let _ = pids;
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn partie(emulateur: Option<&str>) -> PartieEnCours {
        PartieEnCours {
            jeu: 1,
            titre: "Jeu".into(),
            plateforme: "X".into(),
            pid: 1,
            emulateur: emulateur.map(String::from),
            dossier_emulateur: None,
            figer: false,
            dossier_jeu: None,
            clavier: None,
            repeter_touches: true,
            figer_emulateur: false,
            plusieurs_disques: true,
            emplacement: 0,
            emplacement_ra: 0,
            etats: None,
            nom_rom: None,
            raccourcis_clavier: true,
            etats_interdits: false,
            triches: vec![],
            triches_permises: true,
            triche_ra: 0,
            parties: None,
            abri_triches: false,
            coeur: None,
            dossier_rom: None,
        }
    }

    #[test]
    fn un_cht_de_retroarch_se_lit_meme_abime() {
        let t = lire_cht("cheats = 3\ncheat0_desc = \"Vies infinies\"\ncheat0_enable = \"true\"\ncheat1_desc = \"\"\ncheat2_desc = \"Or\"\ncheat2_enable = \"false\"\n");
        assert_eq!(t.len(), 3);
        assert_eq!(t[0], Triche { titre: "Vies infinies".into(), active: true });
        assert_eq!(t[1], Triche { titre: "Triche 2".into(), active: false }, "sans nom ni état");
        assert!(!t[2].active);
        assert!(lire_cht("cheat0_desc = \"X\"\n").is_empty(), "`cheats` absent : rien");
        assert!(lire_cht("cheats = abc\n").is_empty());
        assert_eq!(lire_cht("cheats = 999999\n").len(), TRICHES_MAX, "plafonné");
        assert!(lire_cht("\u{FFFD}\u{0}=\n==\n").is_empty(), "illisible : rien, sans panique");
        let d = tempfile::tempdir().unwrap();
        let f = fichier_cht(d.path(), "Snes9x", "Mario (Europe)");
        assert!(f.ends_with("Snes9x/Mario (Europe).cht"));
        assert!(lire_fichier_cht(&f).is_empty(), "absent");
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(&f, b"cheats = 1\ncheat0_desc = \"\xE9t\xE9\"\n").unwrap();
        assert_eq!(lire_fichier_cht(&f).len(), 1, "texte non UTF-8 : lu quand même");
    }

    #[test]
    fn avant_le_lancement_les_triches_cochees_mettent_la_partie_a_l_abri() {
        let d = tempfile::tempdir().unwrap();
        let (ra, profil) = (d.path().join("RA"), d.path().join("RA/Profils/Seb"));
        for x in ["saves", "states", "cheats/Snes9x"] {
            std::fs::create_dir_all(profil.join(x)).unwrap();
        }
        std::fs::write(ra.join("retroarch.cfg"), "cheevos_hardcore_mode_enable = \"true\"\n").unwrap();
        std::fs::write(profil.join("frogtend.cfg"), "cheevos_enable = \"false\"\n").unwrap();
        std::fs::write(profil.join("saves/Mario (Europe).srm"), b"partie").unwrap();
        let jeux = d.path().join("Jeux");
        std::fs::create_dir_all(&jeux).unwrap();
        let (europe, japon) = (jeux.join("Mario (Europe).sfc"), jeux.join("Mario (Japan).sfc"));
        // Sans triche : rien de copié, chemins du profil.
        let p = preparer_ra(&ra, &profil, Some("Snes9x"), Some(&europe), None, 100).unwrap();
        assert_eq!(p.nom_rom.as_deref(), Some("Mario (Europe)"));
        assert_eq!(p.parties, Some(profil.join("saves")));
        assert!(!p.etats_interdits && p.triches.is_empty() && !p.abri_fait);
        // Une triche cochée : la partie copiée AVANT, une seule fois tant qu'elle ne change pas.
        std::fs::write(profil.join("cheats/Snes9x/Mario (Europe).cht"), "cheats = 1\ncheat0_desc = \"Vies\"\ncheat0_enable = \"true\"\n").unwrap();
        let p = preparer_ra(&ra, &profil, Some("Snes9x"), Some(&europe), None, 100).unwrap();
        assert!(p.abri_fait && p.triches[0].active);
        assert_eq!(std::fs::read(profil.join("saves/Mario (Europe).srm.avant-triche-100")).unwrap(), b"partie");
        preparer_ra(&ra, &profil, Some("Snes9x"), Some(&europe), None, 200).unwrap();
        assert!(!profil.join("saves/Mario (Europe).srm.avant-triche-200").exists(), "copie identique : pas de doublon");
        std::fs::write(profil.join("saves/Mario (Europe).srm"), b"partie 2").unwrap();
        preparer_ra(&ra, &profil, Some("Snes9x"), Some(&europe), None, 100).unwrap();
        assert_eq!(std::fs::read(profil.join("saves/Mario (Europe).srm.avant-triche-100-1")).unwrap(), b"partie 2", "jamais par-dessus");
        // Une autre version : le `.cht` de la version d'habitude repris sous son nom.
        let p = preparer_ra(&ra, &profil, Some("Snes9x"), Some(&japon), Some(&europe), 300).unwrap();
        assert!(p.cht_repris.is_some() && profil.join("cheats/Snes9x/Mario (Japan).cht").is_file());
        assert_eq!(p.triches.len(), 1);
        assert!(!p.triches[0].active && !p.abri_fait, "reprise d'une autre version : rien d'actif");
        // Hardcore actif pour ce profil : interdites, rien de copié.
        std::fs::write(profil.join("frogtend.cfg"), "cheevos_enable = \"true\"\n").unwrap();
        let p = preparer_ra(&ra, &profil, Some("Snes9x"), Some(&europe), None, 400).unwrap();
        assert!(p.etats_interdits && !p.abri_fait);
        // Parties rangées à côté du jeu.
        std::fs::write(profil.join("frogtend.cfg"), "savefiles_in_content_dir = \"true\"\n").unwrap();
        assert_eq!(preparer_ra(&ra, &profil, None, Some(&europe), None, 500).unwrap().parties, Some(jeux.clone()));
    }

    #[test]
    fn l_index_de_triche_avance_pas_a_pas_sans_boucler() {
        assert!(pas_vers_triche(2, 2).is_empty());
        assert_eq!(pas_vers_triche(0, 3), ["CHEAT_INDEX_PLUS"; 3]);
        assert_eq!(pas_vers_triche(4, 1), ["CHEAT_INDEX_MINUS"; 3]);
    }

    #[test]
    fn les_parties_d_un_jeu_sont_trouvees_meme_rangees_par_coeur() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("Snes9x")).unwrap();
        for f in ["Mario.srm", "Snes9x/Mario.rtc", "Mario 2.srm", "Mario.srm.frogtend-abri", "Mariokart.srm", "Mario.sfc", "Mario.chd", "Mario.m3u"] {
            std::fs::write(d.path().join(f), b"x").unwrap();
        }
        let mut l: Vec<String> = parties_du_jeu(d.path(), "Mario").iter().map(|p| p.file_name().unwrap().to_string_lossy().into()).collect();
        l.sort();
        assert_eq!(l, ["Mario.rtc", "Mario.srm"], "jamais le jeu lui-même (parties rangées à côté du jeu)");
    }

    #[test]
    fn un_cht_repris_a_toutes_ses_triches_coupees_et_les_copies_se_comparent_par_blocs() {
        let t = triches_coupees("cheats = 2\ncheat0_enable = \"true\"\ncheat0_desc = \"enable\"\ncheat1_enable = true\n");
        assert!(!t.contains("\"true\"") && !t.contains("= true"), "{t}");
        assert!(t.contains("cheat0_desc = \"enable\"") && t.contains("cheats = 2"));
        assert_eq!(lire_cht(&t).iter().filter(|x| x.active).count(), 0);
        let d = tempfile::tempdir().unwrap();
        let (a, b, c) = (d.path().join("a"), d.path().join("b"), d.path().join("c"));
        let gros: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&a, &gros).unwrap();
        std::fs::write(&b, &gros).unwrap();
        let mut autre = gros.clone();
        autre[150_000] ^= 1;
        std::fs::write(&c, &autre).unwrap();
        assert!(memes_octets(&a, &b));
        assert!(!memes_octets(&a, &c), "différence loin dans le fichier");
        assert!(!memes_octets(&a, &d.path().join("absent")));
    }

    #[test]
    fn les_triches_seulement_pour_retroarch_hors_hardcore() {
        let avec = Triche { titre: "Vies".into(), active: false };
        let ra = PartieEnCours { triches: vec![avec.clone()], ..partie(Some("retroarch")) };
        assert_eq!(etat(&ra).triches, Some(vec![avec.clone()]));
        assert_eq!(etat(&PartieEnCours { triches: vec![], ..partie(Some("retroarch")) }).triches, Some(vec![]), "aucun fichier : dit");
        assert_eq!(etat(&PartieEnCours { etats_interdits: true, ..ra.clone() }).triches, None, "hardcore");
        assert_eq!(etat(&PartieEnCours { triches_permises: false, ..ra.clone() }).triches, None, "hardcore réglé");
        assert_eq!(etat(&PartieEnCours { triches: vec![avec], ..partie(Some("duckstation")) }).triches, None);
        assert!(triches_permises_retroarch("", ""));
        // Chez Seb : la clé hardcore toujours là (recopiée par RetroArch), succès coupés → permises.
        assert!(triches_permises_retroarch("cheevos_hardcore_mode_enable = \"true\"\ncheevos_enable = \"false\"\n", ""));
        assert!(!triches_permises_retroarch("cheevos_hardcore_mode_enable = \"true\"\n", "cheevos_enable = \"true\"\n"), "succès du profil + hardcore");
        assert!(triches_permises_retroarch("cheevos_hardcore_mode_enable = \"true\"\n", "cheevos_enable = \"true\"\ncheevos_hardcore_mode_enable = \"false\"\n"));
    }

    #[test]
    fn une_triche_basculee_et_l_index_sont_retenus() {
        let m = MenuJeu::default();
        m.commencer(PartieEnCours { triches: vec![Triche { titre: "A".into(), active: false }], ..partie(Some("retroarch")) });
        m.basculer_triche(0);
        m.basculer_triche(7); // hors liste : rien
        m.noter_triche_ra(3);
        m.noter_abri_triches();
        let p = m.partie().unwrap();
        assert!(p.triches[0].active);
        assert_eq!(p.triche_ra, 3);
        assert!(p.abri_triches);
    }

    #[test]
    fn les_emplacements_de_retroarch_sont_trouves_meme_rangés_par_coeur() {
        assert_eq!(nom_etat_retroarch("Mario", 0), "Mario.state");
        assert_eq!(nom_etat_retroarch("Mario", 3), "Mario.state3");
        let d = tempfile::tempdir().unwrap();
        // Rangé par cœur (`sort_savestates_enable`, relevé chez Seb) et à la racine.
        std::fs::create_dir_all(d.path().join("Snes9x")).unwrap();
        std::fs::write(d.path().join("Snes9x").join("Super Mario World (Europe) (Rev 1).state"), b"0").unwrap();
        std::fs::write(d.path().join("Super Mario World (Europe) (Rev 1).state2"), b"2").unwrap();
        let l = emplacements_retroarch(d.path(), "Super Mario World (Europe) (Rev 1)");
        assert_eq!(l.len(), 10);
        assert!(l[0].1.is_some() && l[1].1.is_none() && l[2].1.is_some(), "{l:?}");
        assert!(fichier_etat_retroarch(d.path(), "Autre jeu", 0).is_none());
    }

    #[test]
    fn toutes_les_sauvegardes_du_jeu_sont_vues_jamais_les_copies_de_frogtend() {
        let d = tempfile::tempdir().unwrap();
        let r = "Super Mario World (Europe) (Rev 1)";
        std::fs::create_dir_all(d.path().join("Snes9x").join("Mario")).unwrap();
        for f in [format!("Snes9x/{r}.state"), format!("Snes9x/{r}.state3"), format!("Snes9x/{r}.state.auto"), format!("Snes9x/Mario/{r}.state9")] {
            std::fs::write(d.path().join(f), b"x").unwrap();
        }
        // Les copies de Frogtend, une vignette et un autre jeu : jamais comptés.
        for f in [format!("{r}.state.frogtend-precedent"), format!("{r}.state3.frogtend-abri"), format!("{r}.state.png"), "Autre.state".into()] {
            std::fs::write(d.path().join("Snes9x").join(f), b"y").unwrap();
        }
        let l = etats_du_jeu(d.path(), r);
        assert_eq!(l.len(), 4, "{l:?}");
        let mut empl: Vec<Option<u8>> = l.iter().map(|(f, _)| emplacement_du_fichier(f, r)).collect();
        empl.sort();
        assert_eq!(empl, [None, Some(0), Some(3), Some(9)], "l'automatique n'a pas d'emplacement");
    }

    #[test]
    fn le_mode_hardcore_des_succes_interdit_les_sauvegardes_rapides() {
        assert!(retroarch_hardcore("cheevos_enable = \"true\"\ncheevos_hardcore_mode_enable = \"true\"\n", ""));
        assert!(!retroarch_hardcore("cheevos_enable = \"true\"\ncheevos_hardcore_mode_enable = \"true\"\n", "cheevos_enable = \"false\"\n"), "le profil de Frogtend l'emporte (chez Seb : succès coupés)");
        assert!(!retroarch_hardcore("cheevos_hardcore_mode_enable = \"true\"\n", ""), "sans succès actifs");
        let p = PartieEnCours { etats_interdits: true, ..partie(Some("retroarch")) };
        let a = etat(&p).actions;
        assert!(!a.contains(&"sauver") && !a.contains(&"charger") && a.contains(&"reset"));
    }

    #[test]
    fn l_emplacement_choisi_reste_entre_0_et_9() {
        let m = MenuJeu::default();
        assert_eq!(m.changer_emplacement(1), None, "aucune partie");
        m.commencer(partie(Some("retroarch")));
        assert_eq!(m.changer_emplacement(-1), Some(EMPLACEMENT_MAX), "en boucle : de 0 à 9");
        assert_eq!(m.changer_emplacement(1), Some(0), "et de 9 à 0");
        for _ in 0..12 {
            m.changer_emplacement(1);
        }
        assert_eq!(m.partie().unwrap().emplacement, 2);
        m.changer_emplacement(-1);
        m.changer_emplacement(-1);
        m.changer_emplacement(-1);
        assert_eq!(m.partie().unwrap().emplacement, EMPLACEMENT_MAX);
        assert_eq!(etat(&m.partie().unwrap()).emplacement, Some(EMPLACEMENT_MAX));
        assert_eq!(etat(&partie(Some("pcsx2"))).emplacement, None, "celui de l'émulateur : pas choisi ici");
        // Dolphin aux raccourcis réglés sur une manette : rien à proposer (les touches n'y arriveraient pas).
        let dolphin = PartieEnCours { raccourcis_clavier: false, ..partie(Some("dolphin")) };
        assert!(etat(&dolphin).actions.is_empty());
    }

    #[test]
    fn disque_suivant_seulement_pour_une_liste_de_disques() {
        let cartouche = PartieEnCours { plusieurs_disques: false, ..partie(Some("retroarch")) };
        assert!(!etat(&cartouche).actions.contains(&"disque"), "une cartouche SNES : pas de disque suivant");
        assert!(etat(&partie(Some("retroarch"))).actions.contains(&"disque"));
        assert!(etat(&PartieEnCours { plusieurs_disques: false, ..partie(Some("dosbox-staging")) }).actions.contains(&"disque"));
        assert!(liste_de_disques(std::path::Path::new("E:\\Jeux\\FF7.M3U")) && liste_de_disques(std::path::Path::new("x.m3u")));
        assert!(liste_de_disques(std::path::Path::new("Final Fantasy VII (France).PBP")), "un PBP PS1 peut contenir plusieurs disques");
        let fds = PartieEnCours { plusieurs_disques: false, plateforme: "Nintendo Famicom Disk System".into(), ..partie(Some("retroarch")) };
        assert!(etat(&fds).actions.contains(&"disque"), "Famicom Disk System : retourner la disquette");
        assert!(!liste_de_disques(std::path::Path::new("Super Mario World (USA).sfc")) && !liste_de_disques(std::path::Path::new("FF7 (Disc 1).cue")));
    }

    #[test]
    fn un_emulateur_fige_n_est_en_pause_qu_une_fois_fige() {
        assert!(etat(&partie(Some("eden"))).en_pause);
        assert!(!etat(&PartieEnCours { figer_emulateur: true, ..partie(Some("eden")) }).en_pause, "Eden non réglé : figé, pas promis");
        assert!(!etat(&partie(Some("cemu"))).en_pause);
    }

    #[test]
    fn le_menu_propose_ce_que_l_emulateur_sait_faire() {
        let e = etat(&partie(Some("retroarch")));
        assert_eq!(e.actions, vec!["reset", "disque", "sauver", "charger", "fps", "rapide", "capture", "muet"]);
        assert!(e.en_pause);
        let e = etat(&partie(Some("pcsx2")));
        assert!(!e.actions.contains(&"disque"));
        assert!(etat(&partie(Some("rpcs3"))).en_pause, "RPCS3 : pause à la perte du premier plan (0.46.3)");
        let e = etat(&partie(None));
        assert!(e.actions.is_empty() && !e.en_pause, "jeu PC : reprendre et quitter seulement");
        assert!(!etat(&PartieEnCours { figer: true, ..partie(None) }).en_pause, "l'option seule ne dit pas « en pause » : le gel réel le dit");
        for n in ["reset", "disque", "sauver", "charger", "fps", "rapide", "capture", "muet"] {
            assert_eq!(nom_action(action_de(n).unwrap()), n);
        }
        // Les options du jeu : RetroArch seulement (ses commandes réseau), jamais promises ailleurs.
        let ra = etat(&partie(Some("retroarch")));
        assert!(["fps", "rapide", "capture", "muet"].iter().all(|o| ra.actions.contains(o)));
        assert!(!etat(&partie(Some("duckstation"))).actions.contains(&"fps"));
        assert_eq!(crate::pilotage::commande_retroarch(crate::pilotage::Action::ImagesParSeconde), "FPS_TOGGLE");
        assert_eq!(crate::pilotage::commande_retroarch(crate::pilotage::Action::Muet), "MUTE");
        assert!(action_de("formater").is_none());
    }

    #[test]
    fn mame_ne_propose_sauver_et_charger_que_pour_un_jeu_qui_les_gere() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join(crate::pilotage::DOSSIER_MAME)).unwrap();
        let p = PartieEnCours { dossier_emulateur: Some(d.path().to_string_lossy().into()), ..partie(Some("mame")) };
        // Le script n'a encore rien dit : par prudence, ni sauver ni charger (les options du jeu, toujours).
        assert_eq!(etat(&p).actions, ["reset", "fps", "rapide", "capture", "muet"]);
        std::fs::write(d.path().join(crate::pilotage::JEU_MAME), "pong|ne-sauve-pas").unwrap();
        assert_eq!(etat(&p).actions, ["reset", "fps", "rapide", "capture", "muet"]);
        std::fs::write(d.path().join(crate::pilotage::JEU_MAME), "sf2|sauve").unwrap();
        assert_eq!(etat(&p).actions, ["reset", "sauver", "charger", "fps", "rapide", "capture", "muet"]);
        assert!(etat(&p).en_pause);
    }

    #[test]
    fn chaque_touche_envoyee_a_son_code() {
        for emu in ["duckstation", "pcsx2", "dolphin", "dosbox-staging"] {
            for a in crate::pilotage::actions(emu) {
                let t = crate::pilotage::touche(emu, a).unwrap();
                assert!(code(t.touche).is_some(), "{emu} {a:?} : {}", t.touche);
            }
        }
    }

    #[test]
    fn les_touches_proposees_dans_les_options_sont_comprises() {
        // Les choix de ⚙ Options ▸ Émulateurs (src/lib/reglages/Emulateurs.svelte).
        for t in ["Pause", "ScrollLock", "Ctrl+Shift+M"] {
            assert!(t.parse::<tauri_plugin_global_shortcut::Shortcut>().is_ok(), "{t}");
        }
    }

    #[test]
    fn la_partie_en_cours_se_retient_et_s_oublie() {
        let m = MenuJeu::default();
        assert!(m.partie().is_none());
        let n = m.commencer(partie(Some("dolphin")));
        assert_eq!(m.partie().unwrap().emulateur.as_deref(), Some("dolphin"));
        assert!(m.en_cours(n));
        // Une deuxième partie commence avant la fin de la première : la fin de la première ne l'efface pas.
        let n2 = m.commencer(partie(Some("rpcs3")));
        assert!(!m.en_cours(n), "la lecture de la manette de la 1re partie s'arrête");
        m.finir(n);
        assert_eq!(m.partie().unwrap().emulateur.as_deref(), Some("rpcs3"));
        m.finir(n2);
        assert!(m.partie().is_none());
    }
}
