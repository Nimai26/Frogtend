//! 🏷 L'icône du LANCEUR d'une boutique installé sur ce PC, pour les pastilles d'origine des jeux (Seb, 19/11 ; PLAN
//! « Icônes des boutiques sur les pastilles »). Frogtend n'embarque JAMAIS un logo de marque : l'icône est lue par
//! Windows dans le programme du lanceur lui-même, trouvé de façon PROUVÉE dans le registre (sa clé de protocole), en
//! LECTURE SEULE. Rien n'est écrit sur le disque (règle « pas de pieuvre ») : le résultat est gardé en mémoire pour la
//! session. Le programme n'est jamais lancé : Windows lit seulement ses ressources (fichier ouvert comme données).
//! Ordre de secours (côté interface, `src/lib/boutiques/icones.svelte.ts`) : cette icône, puis plus tard celle de
//! Firehouse (BESOINS-API n° 55), puis le texte.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// La taille demandée à Windows (pixels) : assez pour une pastille nette en 4K, sans PNG démesuré.
pub const TAILLE_ICONE: i32 = 48;

/// Les clés de protocole d'une boutique, dans l'ordre d'essai (relevé chez Seb le 19/11, en lecture seule). Xbox (une
/// application UWP) et les origines qui ne sont pas des boutiques (Firehouse, disque) n'en ont pas : le texte reste.
/// Battle.net n'a pas de protocole prouvé : son icône vient de sa clé de désinstallation (`icone_battlenet`).
pub fn protocoles(boutique: &str) -> &'static [&'static str] {
    match boutique {
        "steam" => &["steam"],
        "epic" => &["com.epicgames.launcher"],
        "gog" => &["goggalaxy"],
        "uplay" => &["uplay"],
        "amazon" => &["amazon-games"],
        // L'application EA déclare « origin2 » (EALauncher.exe) et « link2ea » (Link2EA.exe), relevés le 19/11.
        "origin" | "ea" => &["origin2", "link2ea"],
        _ => &[],
    }
}

/// Une boutique dont on sait chercher l'icône du lanceur.
pub fn boutique_connue(boutique: &str) -> bool {
    !protocoles(boutique).is_empty() || boutique == "battlenet"
}

/// Une référence d'icône (`DefaultIcon`, `DisplayIcon`) : « "C:\x\steam.exe",1 », « C:\x\a.exe,-101 », « C:\x\a.exe ».
/// Rend (programme, index) ; un index négatif désigne l'identifiant de la ressource (convention de Windows). `None` :
/// texte vide ou mal formé (guillemet non fermé, reste qui n'est pas « ,<nombre> »).
pub fn analyser_icone(texte: &str) -> Option<(PathBuf, i32)> {
    let t = texte.trim();
    let (chemin, reste) = if let Some(s) = t.strip_prefix('"') {
        let fin = s.find('"')?;
        (&s[..fin], s[fin + 1..].trim())
    } else {
        match t.rsplit_once(',') {
            Some((c, i)) if i.trim().parse::<i32>().is_ok() => (c, &t[c.len()..]),
            _ => (t, ""),
        }
    };
    let index = if reste.is_empty() { 0 } else { reste.strip_prefix(',')?.trim().parse::<i32>().ok()? };
    let chemin = chemin.trim();
    (!chemin.is_empty()).then(|| (PathBuf::from(chemin), index))
}

/// Le programme d'une commande `shell\open\command` : « "C:\x\steam.exe" -- "%1" » (entre guillemets), ou sans
/// guillemets « C:\Program Files\EA\EALauncher.exe "%1" » (jusqu'au premier « .exe » suivi d'un blanc, d'un guillemet
/// ou de la fin). `None` : rien de reconnaissable.
pub fn programme_de_commande(texte: &str) -> Option<PathBuf> {
    let t = texte.trim();
    if let Some(s) = t.strip_prefix('"') {
        let fin = s.find('"')?;
        let c = s[..fin].trim();
        return (!c.is_empty()).then(|| PathBuf::from(c));
    }
    // `to_ascii_lowercase` garde la longueur en octets : les positions valent pour le texte d'origine.
    let bas = t.to_ascii_lowercase();
    let mut depart = 0;
    while let Some(p) = bas[depart..].find(".exe") {
        let fin = depart + p + 4;
        if bas[fin..].chars().next().is_none_or(|c| c.is_whitespace() || c == '"') {
            return Some(PathBuf::from(&t[..fin]));
        }
        depart = fin;
    }
    None
}

/// Un chemin qu'on accepte de lire : absolu sur une lettre de lecteur (« C:\… »), jamais réseau (« \\serveur »,
/// « \\?\ », « // »), sans « .. » ni variable non développée (« % »), d'au plus 259 signes, et un programme `.exe`.
/// Le lecteur local (pas un lecteur réseau mappé) et l'existence du fichier se vérifient à part (`programme_local`).
pub fn chemin_admissible(p: &Path) -> bool {
    let t = p.to_string_lossy();
    let o = t.as_bytes();
    o.len() >= 4
        && t.encode_utf16().count() < 260
        && o[0].is_ascii_alphabetic()
        && o[1] == b':'
        && (o[2] == b'\\' || o[2] == b'/')
        && !t.contains('%')
        && !t.split(['\\', '/']).any(|x| x == "..")
        && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe"))
}

/// Les programmes candidats d'une clé de protocole, dans l'ordre : `DefaultIcon` (avec son index), puis le programme
/// de `shell\open\command` (index 0). Les chemins non admissibles sont écartés (Steam écrit « steam.exe » tout court
/// dans `DefaultIcon` : c'est alors sa commande qui sert).
pub fn candidats(default_icon: Option<&str>, commande: Option<&str>) -> Vec<(PathBuf, i32)> {
    let mut l: Vec<(PathBuf, i32)> = vec![];
    if let Some(r) = default_icon.and_then(analyser_icone) {
        l.push(r);
    }
    if let Some(p) = commande.and_then(programme_de_commande) {
        l.push((p, 0));
    }
    l.retain(|(p, _)| chemin_admissible(p));
    l
}

/// L'icône de Battle.net : `DisplayIcon` de sa clé de désinstallation (celle dont `UninstallString` porte
/// « uid=battle.net », comme `boutiques_pc::lire_battlenet`).
pub fn icone_battlenet(desinstall: &[(String, BTreeMap<String, String>)]) -> Option<(PathBuf, i32)> {
    desinstall
        .iter()
        .find(|(_, v)| v.get("UninstallString").is_some_and(|u| u.to_lowercase().contains("uid=battle.net")))
        .and_then(|(_, v)| analyser_icone(v.get("DisplayIcon")?))
        .filter(|(p, _)| chemin_admissible(p))
}

/// Des pixels BGRA (ordre de Windows) en RGBA. Si AUCUN pixel n'a d'alpha (vieille icône sans couche alpha), la
/// transparence vient du masque (32 bits aussi : noir = opaque) ; sans masque, tout est opaque.
pub fn bgra_vers_rgba(bgra: &[u8], masque: Option<&[u8]>) -> Vec<u8> {
    let avec_alpha = bgra.chunks_exact(4).any(|p| p[3] != 0);
    bgra.chunks_exact(4)
        .enumerate()
        .flat_map(|(i, p)| {
            let a = if avec_alpha {
                p[3]
            } else {
                match masque.and_then(|m| m.get(i * 4)) {
                    Some(0) | None => 255,
                    Some(_) => 0,
                }
            };
            [p[2], p[1], p[0], a]
        })
        .collect()
}

/// Encode des pixels RGBA en PNG. `None` : taille incohérente, image entièrement transparente, ou échec d'encodage.
pub fn encoder_png(rgba: &[u8], l: u32, h: u32) -> Option<Vec<u8>> {
    // La longueur exacte se vérifie ici : `from_raw` accepte un tampon plus long que l'image.
    if l == 0 || h == 0 || l > 256 || h > 256 || rgba.len() != (l * h * 4) as usize || !rgba.chunks_exact(4).any(|p| p[3] != 0) {
        return None;
    }
    let img = image::RgbaImage::from_raw(l, h, rgba.to_vec())?;
    let mut o = std::io::Cursor::new(vec![]);
    img.write_to(&mut o, image::ImageFormat::Png).ok()?;
    Some(o.into_inner())
}

/// Encode en base64 standard (« A-Z a-z 0-9 + / », complété par « = ») ; l'inverse de `boutiques_pc::base64`.
pub fn base64_encoder(octets: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(octets.len().div_ceil(3) * 4);
    for b in octets.chunks(3) {
        let n = (u32::from(b[0]) << 16) | (u32::from(*b.get(1).unwrap_or(&0)) << 8) | u32::from(*b.get(2).unwrap_or(&0));
        s.push(A[(n >> 18) as usize & 63] as char);
        s.push(A[(n >> 12) as usize & 63] as char);
        s.push(if b.len() > 1 { A[(n >> 6) as usize & 63] as char } else { '=' });
        s.push(if b.len() > 2 { A[n as usize & 63] as char } else { '=' });
    }
    s
}

/// L'adresse `data:` d'un PNG, que l'interface met dans une `<img>` (la CSP permet `data:` en `img-src`).
pub fn adresse_png(png: &[u8]) -> String {
    format!("data:image/png;base64,{}", base64_encoder(png))
}

// ───────────────────────────── Sur ce PC ─────────────────────────────

/// Le programme est un fichier d'un lecteur LOCAL (fixe : jamais un lecteur réseau mappé, qui pourrait bloquer des
/// secondes sur un serveur absent ; vu AVANT de toucher au fichier).
#[cfg(windows)]
fn programme_local(p: &Path) -> bool {
    use windows::core::HSTRING;
    use windows::Win32::Storage::FileSystem::GetDriveTypeW;
    const DRIVE_FIXED: u32 = 3;
    if !chemin_admissible(p) {
        return false;
    }
    let fixe = |q: &Path| {
        let racine: String = q.to_string_lossy().chars().take(2).chain(['\\']).collect();
        // SAFETY: lecture du type d'un lecteur, chaîne terminée par HSTRING.
        (unsafe { GetDriveTypeW(&HSTRING::from(racine)) }) == DRIVE_FIXED
    };
    if !fixe(p) || !p.is_file() {
        return false;
    }
    // Un lien ou une jonction d'un disque local peut mener ailleurs (un serveur) : le VRAI fichier passe les mêmes
    // contrôles (relecture de l'expert lancement, 19/11). `canonicalize` rend « \\?\C:\… » ; « \\?\UNC\… » est refusé.
    match std::fs::canonicalize(p) {
        Ok(vrai) => {
            let t = vrai.to_string_lossy().into_owned();
            match t.strip_prefix(r"\\?\") {
                Some(reste) if !reste.starts_with(r"UNC\") => {
                    let q = Path::new(reste);
                    chemin_admissible(q) && fixe(q)
                }
                _ => false,
            }
        }
        Err(_) => false,
    }
}

/// Extrait l'icône `index` du programme à la taille voulue et la rend en PNG. Toutes les poignées sont rendues à
/// Windows (`DestroyIcon`, `DeleteObject`, `ReleaseDC`) quel que soit le chemin de sortie.
#[cfg(windows)]
fn extraire_png(programme: &Path, index: i32, taille: i32) -> Result<Vec<u8>, String> {
    use windows::Win32::Graphics::Gdi::{DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP, HDC};
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, PrivateExtractIconsW, HICON, ICONINFO};

    let mut nom = [0u16; 260];
    let large: Vec<u16> = programme.as_os_str().to_string_lossy().encode_utf16().collect();
    if large.len() >= nom.len() {
        return Err("chemin trop long".into());
    }
    nom[..large.len()].copy_from_slice(&large);
    let mut icones = [HICON::default()];
    // SAFETY: `nom` est terminé par un zéro ; un seul emplacement d'icône est fourni et annoncé. Windows ouvre le
    // programme comme un fichier de données (ressources), sans exécuter son code.
    let n = unsafe { PrivateExtractIconsW(&nom, index, taille, taille, Some(&mut icones), None, 0) };
    if n == 0 || n == u32::MAX || icones[0].is_invalid() {
        return Err("aucune icône à cet index".into());
    }
    let icone = icones[0];

    /// Les lignes 32 bits (BGRA, de haut en bas) d'un bitmap.
    unsafe fn lignes(dc: HDC, bm: HBITMAP, l: i32, h: i32) -> Option<Vec<u8>> {
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER { biSize: size_of::<BITMAPINFOHEADER>() as u32, biWidth: l, biHeight: -h, biPlanes: 1, biBitCount: 32, biCompression: BI_RGB.0, ..Default::default() },
            ..Default::default()
        };
        let mut px = vec![0u8; (l * h * 4) as usize];
        // SAFETY: tampon de l × h × 4 octets pour h lignes de 32 bits.
        let lues = unsafe { GetDIBits(dc, bm, 0, h as u32, Some(px.as_mut_ptr().cast()), &mut info, DIB_RGB_COLORS) };
        (lues == h).then_some(px)
    }

    let mut ii = ICONINFO::default();
    // SAFETY: icône valide ; les deux bitmaps rendus par GetIconInfo sont détruits plus bas.
    let r = unsafe {
        if GetIconInfo(icone, &mut ii).is_err() {
            let _ = DestroyIcon(icone);
            return Err("icône illisible".into());
        }
        let resultat = (|| {
            if ii.hbmColor.is_invalid() {
                return Err("icône monochrome".to_string());
            }
            let mut bm = BITMAP::default();
            if GetObjectW(ii.hbmColor.into(), size_of::<BITMAP>() as i32, Some((&mut bm as *mut BITMAP).cast())) == 0 {
                return Err("taille de l'icône illisible".into());
            }
            let (l, h) = (bm.bmWidth, bm.bmHeight);
            if l <= 0 || h <= 0 || l > 256 || h > 256 {
                return Err("taille de l'icône hors limites".into());
            }
            let dc = GetDC(None);
            if dc.is_invalid() {
                return Err("contexte d'affichage indisponible".into());
            }
            let couleur = lignes(dc, ii.hbmColor, l, h);
            let masque = if ii.hbmMask.is_invalid() { None } else { lignes(dc, ii.hbmMask, l, h) };
            ReleaseDC(None, dc);
            let couleur = couleur.ok_or("pixels de l'icône illisibles")?;
            encoder_png(&bgra_vers_rgba(&couleur, masque.as_deref()), l as u32, h as u32).ok_or_else(|| "encodage PNG impossible".to_string())
        })();
        if !ii.hbmColor.is_invalid() {
            let _ = DeleteObject(ii.hbmColor.into());
        }
        if !ii.hbmMask.is_invalid() {
            let _ = DeleteObject(ii.hbmMask.into());
        }
        let _ = DestroyIcon(icone);
        resultat
    };
    r
}

/// Les programmes candidats d'une boutique sur ce PC (registre en lecture seule : compte, puis machine).
#[cfg(windows)]
fn candidats_du_pc(boutique: &str) -> Vec<(PathBuf, i32)> {
    use crate::registre::{lire_texte_hkcu, lire_texte_hklm};
    if boutique == "battlenet" {
        return icone_battlenet(&crate::boutiques_pc::desinstallations()).into_iter().collect();
    }
    let mut l = vec![];
    for p in protocoles(boutique) {
        for compte in [true, false] {
            let lire = |sous: &str| if compte { lire_texte_hkcu(&format!(r"Software\Classes\{p}\{sous}"), "") } else { lire_texte_hklm(&format!(r"SOFTWARE\Classes\{p}\{sous}"), "") };
            for c in candidats(lire("DefaultIcon").as_deref(), lire(r"shell\open\command").as_deref()) {
                if !l.contains(&c) {
                    l.push(c);
                }
            }
        }
    }
    l
}

/// L'icône du lanceur de `boutique` en PNG, ou le motif de l'échec (sans chemin : rien de sensible au journal).
#[cfg(windows)]
pub fn icone_du_lanceur(boutique: &str) -> Result<Vec<u8>, String> {
    let l = candidats_du_pc(boutique);
    if l.is_empty() {
        return Err("lanceur absent (aucune clé de protocole lisible)".into());
    }
    let mut motif = "programme du lanceur introuvable sur un disque local".to_string();
    for (p, i) in l {
        if !programme_local(&p) {
            continue;
        }
        match extraire_png(&p, i, TAILLE_ICONE) {
            Ok(png) => return Ok(png),
            Err(e) => motif = e,
        }
    }
    Err(motif)
}

#[cfg(not(windows))]
pub fn icone_du_lanceur(_: &str) -> Result<Vec<u8>, String> {
    Err("seulement sous Windows".into())
}

/// Ce qui a déjà été cherché dans cette session (une fois par boutique : ni registre ni programme relus ensuite).
fn memoire() -> &'static Mutex<BTreeMap<String, Option<String>>> {
    static M: OnceLock<Mutex<BTreeMap<String, Option<String>>>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// L'icône du lanceur d'une boutique (`data:image/png;base64,…`), ou `None` : boutique sans lanceur connu (Xbox,
/// Firehouse, disque…), lanceur absent, icône illisible. Un échec s'écrit UNE fois au journal par `journal`.
pub fn icone_memorisee(boutique: &str, journal: impl FnOnce(&str)) -> Option<String> {
    if !boutique_connue(boutique) {
        return None;
    }
    if let Some(d) = memoire().lock().ok().and_then(|m| m.get(boutique).cloned()) {
        return d;
    }
    let r = match icone_du_lanceur(boutique) {
        Ok(png) => Some(adresse_png(&png)),
        Err(e) => {
            journal(&format!("icône du lanceur {boutique} : pas d'icône ({e}), la pastille garde son texte"));
            None
        }
    };
    if let Ok(mut m) = memoire().lock() {
        m.insert(boutique.to_string(), r.clone());
    }
    r
}

/// Commande : l'icône du lanceur d'une boutique installé sur ce PC (`None` : la pastille garde son texte).
#[tauri::command]
pub async fn boutique_icone(noyau: tauri::State<'_, crate::noyau::Noyau>, boutique: String) -> crate::erreurs::Resultat<Option<String>> {
    // Registre et ressources du programme : lus hors du fil de l'interface. Une tâche qui échoue rend « pas d'icône ».
    let (icone, ligne) = tauri::async_runtime::spawn_blocking(move || {
        let mut ligne = None;
        let icone = icone_memorisee(&boutique, |m| ligne = Some(m.to_string()));
        (icone, ligne)
    })
    .await
    .unwrap_or((None, None));
    if let Some(l) = ligne {
        noyau.journaliser(&l);
    }
    Ok(icone)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_reference_d_icone_entre_guillemets_avec_index() {
        assert_eq!(analyser_icone(r#""C:\x\steam.exe",1"#), Some((PathBuf::from(r"C:\x\steam.exe"), 1)));
        assert_eq!(analyser_icone(r#"  "C:\Program Files\a b\x.exe" , 2 "#), Some((PathBuf::from(r"C:\Program Files\a b\x.exe"), 2)));
        assert_eq!(analyser_icone(r#""C:\x\a.exe""#), Some((PathBuf::from(r"C:\x\a.exe"), 0)));
    }

    #[test]
    fn une_reference_d_icone_sans_guillemets() {
        assert_eq!(analyser_icone(r"C:\x\a.exe"), Some((PathBuf::from(r"C:\x\a.exe"), 0)));
        assert_eq!(analyser_icone(r"C:\Program Files (x86)\Epic Games\EpicGamesLauncher.exe,0"), Some((PathBuf::from(r"C:\Program Files (x86)\Epic Games\EpicGamesLauncher.exe"), 0)));
        // Une virgule dans un nom de dossier n'est pas un index.
        assert_eq!(analyser_icone(r"C:\Jeux, etc\a.exe"), Some((PathBuf::from(r"C:\Jeux, etc\a.exe"), 0)));
    }

    #[test]
    fn un_index_negatif_designe_la_ressource() {
        assert_eq!(analyser_icone(r"C:\x\a.exe,-101"), Some((PathBuf::from(r"C:\x\a.exe"), -101)));
        assert_eq!(analyser_icone(r#""C:\x\a.exe",-5"#), Some((PathBuf::from(r"C:\x\a.exe"), -5)));
    }

    #[test]
    fn une_reference_mal_formee_est_refusee() {
        assert_eq!(analyser_icone(""), None);
        assert_eq!(analyser_icone("   "), None);
        assert_eq!(analyser_icone(r#""C:\x\a.exe"#), None, "guillemet non fermé");
        assert_eq!(analyser_icone(r#""C:\x\a.exe" -- "%1""#), None, "un reste qui n'est pas un index");
        assert_eq!(analyser_icone(r#""C:\x\a.exe",un"#), None);
        assert_eq!(analyser_icone(r#""",1"#), None);
    }

    #[test]
    fn le_programme_d_une_commande() {
        assert_eq!(programme_de_commande(r#""D:\LaunchBox\Games\Steam\steam.exe" -- "%1""#), Some(PathBuf::from(r"D:\LaunchBox\Games\Steam\steam.exe")));
        assert_eq!(programme_de_commande(r#"C:\Program Files\Electronic Arts\EA Desktop\EA Desktop\EALauncher.exe "%1""#), Some(PathBuf::from(r"C:\Program Files\Electronic Arts\EA Desktop\EA Desktop\EALauncher.exe")));
        assert_eq!(programme_de_commande(r"C:\x\A.EXE"), Some(PathBuf::from(r"C:\x\A.EXE")));
        // « .exe » au milieu d'un nom de dossier n'arrête pas la lecture.
        assert_eq!(programme_de_commande(r"C:\a.exed\b.exe %1"), Some(PathBuf::from(r"C:\a.exed\b.exe")));
        assert_eq!(programme_de_commande(r#""C:\x\a.exe"#), None);
        assert_eq!(programme_de_commande("rundll32 url.dll"), None);
        assert_eq!(programme_de_commande(""), None);
    }

    #[test]
    fn seul_un_programme_exe_local_et_absolu_est_admis() {
        assert!(chemin_admissible(Path::new(r"C:\x\steam.exe")));
        assert!(chemin_admissible(Path::new(r"d:/Jeux/Galaxy/GalaxyClient.EXE")));
        assert!(!chemin_admissible(Path::new(r"\\serveur\partage\steam.exe")), "UNC");
        assert!(!chemin_admissible(Path::new(r"\\?\C:\x\steam.exe")), "forme longue");
        assert!(!chemin_admissible(Path::new(r"//serveur/partage/steam.exe")));
        assert!(!chemin_admissible(Path::new("steam.exe")), "relatif");
        assert!(!chemin_admissible(Path::new(r"C:steam.exe")), "relatif au lecteur");
        assert!(!chemin_admissible(Path::new(r"C:\x\icone.ico")), "pas un .exe");
        assert!(!chemin_admissible(Path::new(r"C:\x\a.dll")));
        assert!(!chemin_admissible(Path::new(r"C:\x\..\Windows\a.exe")), "remontée");
        assert!(!chemin_admissible(Path::new(r"%ProgramFiles%\x\a.exe")), "variable non développée");
        assert!(!chemin_admissible(Path::new(&format!(r"C:\{}\a.exe", "x".repeat(260)))), "trop long");
    }

    #[test]
    fn steam_qui_ecrit_un_nom_seul_dans_defaulticon_passe_par_sa_commande() {
        let c = candidats(Some("steam.exe"), Some(r#""D:\LaunchBox\Games\Steam\steam.exe" -- "%1""#));
        assert_eq!(c, vec![(PathBuf::from(r"D:\LaunchBox\Games\Steam\steam.exe"), 0)]);
        let c = candidats(Some(r"C:\Epic\EpicGamesLauncher.exe,0"), None);
        assert_eq!(c, vec![(PathBuf::from(r"C:\Epic\EpicGamesLauncher.exe"), 0)]);
        assert!(candidats(Some(r"\\nas\x\a.exe,0"), Some(r#""\\nas\x\a.exe" "%1""#)).is_empty());
        assert!(candidats(None, None).is_empty());
    }

    #[test]
    fn battlenet_prend_l_icone_de_sa_cle_de_desinstallation() {
        let v = |l: &[(&str, &str)]| l.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect::<BTreeMap<_, _>>();
        let d = vec![
            ("Overwatch".to_string(), v(&[("UninstallString", r#""C:\B\Battle.net Uninstaller.exe" --uid=prometheus"#), ("DisplayIcon", r"C:\O\Overwatch.exe")])),
            ("Battle.net".to_string(), v(&[("UninstallString", r#""C:\B\Battle.net Uninstaller.exe" --uid=battle.net"#), ("DisplayIcon", r#""C:\B\Battle.net Launcher.exe",0"#)])),
        ];
        assert_eq!(icone_battlenet(&d), Some((PathBuf::from(r"C:\B\Battle.net Launcher.exe"), 0)));
        assert_eq!(icone_battlenet(&d[..1]), None);
    }

    #[test]
    fn les_boutiques_sans_lanceur_lisible_gardent_le_texte() {
        for b in ["xboxone", "xbox", "firehouse", "disque", "", "autre"] {
            assert!(!boutique_connue(b), "{b}");
            assert_eq!(icone_memorisee(b, |_| panic!("rien au journal pour {b}")), None);
        }
        for b in ["steam", "epic", "gog", "uplay", "amazon", "battlenet", "origin", "ea"] {
            assert!(boutique_connue(b), "{b}");
        }
    }

    #[test]
    fn les_pixels_bgra_deviennent_rgba_avec_leur_alpha() {
        assert_eq!(bgra_vers_rgba(&[1, 2, 3, 128, 10, 20, 30, 0], None), vec![3, 2, 1, 128, 30, 20, 10, 0]);
    }

    #[test]
    fn une_icone_sans_alpha_prend_la_transparence_du_masque() {
        let couleur = [1, 2, 3, 0, 4, 5, 6, 0];
        let masque = [0, 0, 0, 0, 255, 255, 255, 0];
        assert_eq!(bgra_vers_rgba(&couleur, Some(&masque)), vec![3, 2, 1, 255, 6, 5, 4, 0]);
        assert_eq!(bgra_vers_rgba(&couleur, None), vec![3, 2, 1, 255, 6, 5, 4, 255]);
    }

    #[test]
    fn le_png_encode_se_relit_a_la_meme_taille() {
        let rgba: Vec<u8> = (0..48 * 48).flat_map(|i| [(i % 256) as u8, 10, 20, 255]).collect();
        let png = encoder_png(&rgba, 48, 48).expect("png");
        let img = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(img.dimensions(), (48, 48));
        assert_eq!(img.as_raw(), &rgba);
        assert_eq!(encoder_png(&rgba, 47, 48), None, "taille incohérente");
        assert_eq!(encoder_png(&[0; 16], 2, 2), None, "entièrement transparente");
        assert_eq!(encoder_png(&[], 0, 0), None);
    }

    #[test]
    fn le_base64_se_relit_par_le_decodeur_des_boutiques() {
        assert_eq!(base64_encoder(b""), "");
        assert_eq!(base64_encoder(b"f"), "Zg==");
        assert_eq!(base64_encoder(b"fo"), "Zm8=");
        assert_eq!(base64_encoder(b"foo"), "Zm9v");
        assert_eq!(base64_encoder(b"foobar"), "Zm9vYmFy");
        let o: Vec<u8> = (0..=255u8).collect();
        assert_eq!(crate::boutiques_pc::base64(base64_encoder(&o).as_bytes()), Some(o));
        assert!(adresse_png(b"x").starts_with("data:image/png;base64,"));
    }

    /// Essai RÉEL sur ce PC, en LECTURE SEULE (registre, programmes des lanceurs : rien n'est lancé ni écrit). À
    /// lancer seul : `cargo test essai_icone_lanceur_steam -- --ignored --exact --nocapture`.
    #[test]
    #[ignore]
    #[cfg(windows)]
    fn essai_icone_lanceur_steam() {
        for b in ["steam", "epic", "gog", "uplay", "amazon", "battlenet", "origin"] {
            match icone_du_lanceur(b) {
                Ok(png) => {
                    let img = image::load_from_memory(&png).unwrap().to_rgba8();
                    // La transparence lue : une icône entièrement opaque trahirait un alpha perdu (carré autour).
                    let opaques = img.pixels().filter(|p| p[3] == 255).count();
                    let vides = img.pixels().filter(|p| p[3] == 0).count();
                    println!("{b} : icône {}×{}, PNG de {} octets, {opaques} pixels opaques, {vides} transparents", img.width(), img.height(), png.len());
                }
                Err(e) => println!("{b} : pas d'icône ({e})"),
            }
        }
        let png = icone_du_lanceur("steam").expect("Steam est installé sur Venkman");
        assert_eq!(image::load_from_memory(&png).unwrap().width(), TAILLE_ICONE as u32);
    }
}
