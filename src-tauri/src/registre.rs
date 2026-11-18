//! Lire le registre de Windows par son API (jamais `reg.exe` : sa sortie est dans la page de code de la console, un
//! chemin accentué y est abîmé). LECTURE seulement, à UNE exception près, après le oui de la personne (✅ Seb 28/10,
//! « Proposer de le couper ») : le bouton Xbox qui ouvre la Game Bar (`bouton_game_bar`), réversible.

/// Une valeur texte (REG_SZ ou REG_EXPAND_SZ, développée) sous `HKEY_CURRENT_USER`.
pub fn lire_texte_hkcu(cle: &str, valeur: &str) -> Option<String> {
    lire(true, cle, valeur)
}

/// Une valeur texte sous `HKEY_LOCAL_MACHINE` (vue 64 bits, puis 32 bits via `WOW6432Node` si on la donne).
pub fn lire_texte_hklm(cle: &str, valeur: &str) -> Option<String> {
    lire(false, cle, valeur)
}

#[cfg(windows)]
fn lire(utilisateur: bool, cle: &str, valeur: &str) -> Option<String> {
    use windows::core::HSTRING;
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_EXPAND_SZ, RRF_RT_REG_SZ};
    let racine = if utilisateur { HKEY_CURRENT_USER } else { HKEY_LOCAL_MACHINE };
    let (c, v) = (HSTRING::from(cle), HSTRING::from(valeur));
    let mut taille: u32 = 0;
    // SAFETY: appels Win32 en lecture ; le tampon a la taille annoncée par le premier appel.
    unsafe {
        if RegGetValueW(racine, &c, &v, RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ, None, None, Some(&mut taille)).is_err() || taille == 0 || taille > 64 * 1024 {
            return None;
        }
        let mut tampon = vec![0u16; (taille as usize).div_ceil(2)];
        if RegGetValueW(racine, &c, &v, RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ, None, Some(tampon.as_mut_ptr().cast()), Some(&mut taille)).is_err() {
            return None;
        }
        let fin = tampon.iter().position(|x| *x == 0).unwrap_or(tampon.len());
        String::from_utf16(&tampon[..fin]).ok().filter(|t| !t.is_empty())
    }
}

#[cfg(not(windows))]
fn lire(_: bool, _: &str, _: &str) -> Option<String> {
    None
}

/// La racine d'une clé lue par les lecteurs de boutiques (import automatique, lot A du 18/11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Racine {
    /// `HKEY_LOCAL_MACHINE` : lue dans la vue 64 bits PUIS dans la vue 32 bits (`WOW6432Node`).
    Machine,
    /// `HKEY_CURRENT_USER` (une seule vue).
    Utilisateur,
}

/// Les sous-clés d'une clé et leurs valeurs TEXTE (REG_SZ, REG_EXPAND_SZ développée), en LECTURE SEULE. Sous
/// `HKEY_LOCAL_MACHINE`, les deux vues sont lues (64 puis 32 bits) et une sous-clé n'est gardée qu'une fois (la 64 bits
/// d'abord). `None` : la clé n'existe dans AUCUNE vue (le lanceur n'est pas là) ; `Some(vide)` : elle existe, vide.
/// Au plus `PLAFOND_SOUS_CLES` sous-clés (au-delà : `None`, « inconnu », jamais une liste partielle qui ferait croire
/// des jeux désinstallés) et 200 valeurs par sous-clé.
pub fn sous_cles(racine: Racine, cle: &str) -> Option<Vec<(String, std::collections::BTreeMap<String, String>)>> {
    let mut trouve = false;
    let mut l: Vec<(String, std::collections::BTreeMap<String, String>)> = vec![];
    for vue in vues(racine) {
        let enfants = match enumerer(racine, cle, vue) {
            Enumeration::Absente => continue,
            Enumeration::Tronquee => return None,
            Enumeration::Lue(e) => e,
        };
        trouve = true;
        for (nom, valeurs) in enfants {
            if !l.iter().any(|(n, _)| n.eq_ignore_ascii_case(&nom)) {
                l.push((nom, valeurs));
            }
        }
    }
    trouve.then_some(l)
}

/// Au-delà, une clé n'est pas lue (une liste tronquée ne vaut rien).
pub const PLAFOND_SOUS_CLES: usize = 4000;

/// Ce que donne l'énumération d'une clé dans une vue.
enum Enumeration {
    Absente,
    Tronquee,
    Lue(Vec<(String, std::collections::BTreeMap<String, String>)>),
}

/// Une liste de noms lus : tronquée si le plafond est atteint et qu'il en restait.
fn borner(noms: Vec<String>, il_en_reste: bool) -> Option<Vec<String>> {
    (!(il_en_reste && noms.len() >= PLAFOND_SOUS_CLES)).then_some(noms)
}

/// Les valeurs texte d'UNE clé (première vue où elle existe). `None` : la clé n'existe pas.
pub fn valeurs(racine: Racine, cle: &str) -> Option<std::collections::BTreeMap<String, String>> {
    vues(racine).into_iter().find_map(|vue| valeurs_de(racine, cle, vue))
}

/// Les vues à lire : 64 puis 32 bits pour la machine, la seule vue pour le compte.
fn vues(racine: Racine) -> Vec<u32> {
    match racine {
        Racine::Machine => vec![0x0100, 0x0200], // KEY_WOW64_64KEY, KEY_WOW64_32KEY
        Racine::Utilisateur => vec![0],
    }
}

#[cfg(windows)]
fn ouvrir(racine: Racine, cle: &str, vue: u32) -> Option<windows::Win32::System::Registry::HKEY> {
    use windows::core::HSTRING;
    use windows::Win32::System::Registry::{RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, REG_SAM_FLAGS};
    let base = if racine == Racine::Machine { HKEY_LOCAL_MACHINE } else { HKEY_CURRENT_USER };
    let mut h = HKEY::default();
    // SAFETY: ouverture EN LECTURE (KEY_READ) ; la poignée est fermée par l'appelant.
    unsafe { RegOpenKeyExW(base, &HSTRING::from(cle), Some(0), KEY_READ | REG_SAM_FLAGS(vue), &mut h) }.is_ok().then_some(h)
}

#[cfg(windows)]
fn fermer(h: windows::Win32::System::Registry::HKEY) {
    // SAFETY: poignée ouverte par `ouvrir`, fermée une seule fois.
    unsafe {
        let _ = windows::Win32::System::Registry::RegCloseKey(h);
    }
}

/// Les valeurs texte d'une clé ouverte.
#[cfg(windows)]
fn valeurs_ouvertes(h: windows::Win32::System::Registry::HKEY) -> std::collections::BTreeMap<String, String> {
    use windows::core::PWSTR;
    use windows::Win32::System::Registry::{RegEnumValueW, REG_EXPAND_SZ, REG_SZ};
    let mut m = std::collections::BTreeMap::new();
    // Les mêmes tampons pour toutes les valeurs (jamais 80 Ko alloués par valeur).
    let mut nom = vec![0u16; 16_384];
    let mut donnees = vec![0u8; 64 * 1024];
    for i in 0..200u32 {
        let mut lnom = nom.len() as u32;
        let mut type_ = 0u32;
        let mut ldonnees = donnees.len() as u32;
        // SAFETY: tampons de la taille annoncée ; lecture seule.
        let r = unsafe { RegEnumValueW(h, i, Some(PWSTR(nom.as_mut_ptr())), &mut lnom, None, Some(&mut type_), Some(donnees.as_mut_ptr()), Some(&mut ldonnees)) };
        if r.0 == 259 {
            break; // ERROR_NO_MORE_ITEMS
        }
        if r.is_err() || (type_ != REG_SZ.0 && type_ != REG_EXPAND_SZ.0) {
            continue; // trop long, ou pas un texte
        }
        let n = String::from_utf16_lossy(&nom[..lnom as usize]);
        let mots: Vec<u16> = donnees[..ldonnees as usize].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        let fin = mots.iter().position(|x| *x == 0).unwrap_or(mots.len());
        let mut v = String::from_utf16_lossy(&mots[..fin]);
        if type_ == REG_EXPAND_SZ.0 {
            v = developper(&v);
        }
        m.insert(n, v);
    }
    m
}

/// `%ProgramData%\x` → le chemin réel (variables d'environnement de ce compte).
fn developper(v: &str) -> String {
    let mut r = String::new();
    let mut reste = v;
    while let Some(debut) = reste.find('%') {
        let Some(fin) = reste[debut + 1..].find('%') else { break };
        let nom = &reste[debut + 1..debut + 1 + fin];
        r.push_str(&reste[..debut]);
        match std::env::var(nom) {
            Ok(x) if !nom.is_empty() => r.push_str(&x),
            _ => r.push_str(&reste[debut..debut + fin + 2]),
        }
        reste = &reste[debut + fin + 2..];
    }
    r.push_str(reste);
    r
}

#[cfg(windows)]
fn valeurs_de(racine: Racine, cle: &str, vue: u32) -> Option<std::collections::BTreeMap<String, String>> {
    let h = ouvrir(racine, cle, vue)?;
    let m = valeurs_ouvertes(h);
    fermer(h);
    Some(m)
}

#[cfg(windows)]
fn enumerer(racine: Racine, cle: &str, vue: u32) -> Enumeration {
    use windows::core::PWSTR;
    use windows::Win32::System::Registry::RegEnumKeyExW;
    let Some(h) = ouvrir(racine, cle, vue) else { return Enumeration::Absente };
    let mut noms = vec![];
    let mut nom = vec![0u16; 256];
    let mut il_en_reste = false;
    for i in 0..=PLAFOND_SOUS_CLES as u32 {
        let mut l = nom.len() as u32;
        // SAFETY: tampon de 256 caractères (taille maximale d'un nom de clé).
        let r = unsafe { RegEnumKeyExW(h, i, Some(PWSTR(nom.as_mut_ptr())), &mut l, None, None, None, None) };
        if r.is_err() {
            break;
        }
        if noms.len() == PLAFOND_SOUS_CLES {
            il_en_reste = true;
            break;
        }
        noms.push(String::from_utf16_lossy(&nom[..l as usize]));
    }
    fermer(h);
    let Some(noms) = borner(noms, il_en_reste) else { return Enumeration::Tronquee };
    Enumeration::Lue(noms.into_iter().map(|n| (n.clone(), valeurs_de(racine, &format!(r"{cle}\{n}"), vue).unwrap_or_default())).collect())
}

#[cfg(not(windows))]
fn valeurs_de(_: Racine, _: &str, _: u32) -> Option<std::collections::BTreeMap<String, String>> {
    None
}

#[cfg(not(windows))]
fn enumerer(_: Racine, _: &str, _: u32) -> Enumeration {
    Enumeration::Absente
}

/// Le réglage de Windows « Ouvrir la Game Bar avec ce bouton d'une manette » (le bouton Xbox) : clé du compte
/// (`HKEY_CURRENT_USER`), DWORD 1 = oui, 0 = non ; absent = oui (constaté chez Seb, 28/10 : absent, et le bouton
/// l'ouvrait). Sources : tutoriels Windows 11 (elevenforum, cloudspress), pas de documentation officielle.
const GAME_BAR: &str = r"Software\Microsoft\GameBar";
const GAME_BAR_BOUTON: &str = "UseNexusForGameBarEnabled";

/// Le bouton Xbox ouvre-t-il la Game Bar ? `None` : hors de Windows.
#[cfg(windows)]
pub fn bouton_game_bar() -> Option<bool> {
    Some(lire_dword_hkcu(GAME_BAR, GAME_BAR_BOUTON).map(|v| v != 0).unwrap_or(true))
}

#[cfg(not(windows))]
pub fn bouton_game_bar() -> Option<bool> {
    None
}

/// Règle ce seul réglage (après le oui de la personne), puis le relit : `Ok` seulement si la relecture le confirme.
#[cfg(windows)]
pub fn regler_bouton_game_bar(actif: bool) -> Result<(), String> {
    use windows::core::HSTRING;
    use windows::Win32::System::Registry::{RegSetKeyValueW, HKEY_CURRENT_USER, REG_DWORD};
    let n: u32 = actif.into();
    // SAFETY: écriture d'un DWORD de 4 octets dans une clé du compte ; le tampon vit pendant l'appel.
    let r = unsafe { RegSetKeyValueW(HKEY_CURRENT_USER, &HSTRING::from(GAME_BAR), &HSTRING::from(GAME_BAR_BOUTON), REG_DWORD.0, Some((&n as *const u32).cast()), std::mem::size_of::<u32>() as u32) };
    if r.is_err() {
        return Err(format!("Windows a refusé de changer ce réglage (code {}).", r.0));
    }
    match bouton_game_bar() {
        Some(v) if v == actif => Ok(()),
        _ => Err("Le réglage relu n'a pas la valeur voulue.".into()),
    }
}

#[cfg(not(windows))]
pub fn regler_bouton_game_bar(_: bool) -> Result<(), String> {
    Err("Seulement sous Windows.".into())
}

#[cfg(windows)]
fn lire_dword_hkcu(cle: &str, valeur: &str) -> Option<u32> {
    use windows::core::HSTRING;
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
    let mut v: u32 = 0;
    let mut taille: u32 = 4;
    // SAFETY: lecture d'un DWORD dans un tampon de 4 octets.
    unsafe { RegGetValueW(HKEY_CURRENT_USER, &HSTRING::from(cle), &HSTRING::from(valeur), RRF_RT_REG_DWORD, None, Some((&mut v as *mut u32).cast()), Some(&mut taille)) }
        .is_ok()
        .then_some(v)
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(windows)]
    fn une_valeur_de_windows_se_lit_et_une_absente_rend_rien() {
        // Toujours présente sur Windows.
        let v = super::lire_texte_hklm(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion", "ProductName").unwrap();
        assert!(v.contains("Windows"), "{v}");
        assert_eq!(super::lire_texte_hkcu(r"Software\Frogtend-absent-xyz", "Rien"), None);
        // Lecture seule ici (jamais d'écriture dans le registre de ce PC par un test) : absent = le bouton l'ouvre.
        assert!(super::bouton_game_bar().is_some());
        assert_eq!(super::lire_dword_hkcu(r"Software\Frogtend-absent-xyz", "Rien"), None);
    }

    /// LECTURE SEULE : les sous-clés et leurs valeurs, dans les deux vues de la machine.
    #[test]
    #[cfg(windows)]
    fn les_sous_cles_et_leurs_valeurs_se_lisent_dans_les_deux_vues() {
        use super::{sous_cles, valeurs, Racine};
        // Toujours présente sur Windows, avec des sous-clés (vue 64 ET vue 32 bits).
        let l = sous_cles(Racine::Machine, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall").unwrap();
        let noms: Vec<&str> = l.iter().map(|(n, _)| n.as_str()).collect();
        let mut tries = noms.iter().map(|n| n.to_lowercase()).collect::<Vec<_>>();
        tries.sort();
        let avant = tries.len();
        tries.dedup();
        assert_eq!(avant, tries.len(), "une sous-clé présente dans les deux vues n'est gardée qu'une fois");
        let v = valeurs(Racine::Machine, r"SOFTWARE\Microsoft\Windows NT\CurrentVersion").unwrap();
        assert!(v.get("ProductName").is_some_and(|p| p.contains("Windows")));
        assert!(sous_cles(Racine::Machine, r"SOFTWARE\Frogtend-absent-xyz").is_none(), "clé absente : None, pas une liste vide");
        assert!(valeurs(Racine::Utilisateur, r"Software\Frogtend-absent-xyz").is_none());
    }

    #[test]
    fn une_liste_de_sous_cles_tronquee_ne_vaut_rien() {
        let n = |k: usize| (0..k).map(|i| i.to_string()).collect::<Vec<_>>();
        assert_eq!(super::borner(n(3), false).map(|l| l.len()), Some(3));
        assert_eq!(super::borner(n(super::PLAFOND_SOUS_CLES), false).map(|l| l.len()), Some(super::PLAFOND_SOUS_CLES), "pile le plafond, rien après");
        assert_eq!(super::borner(n(super::PLAFOND_SOUS_CLES), true), None, "il en restait : inconnu");
    }

    #[test]
    fn une_variable_d_environnement_se_developpe() {
        std::env::set_var("FROGTEND_ESSAI_REG", r"C:\Donnees");
        assert_eq!(super::developper(r"%FROGTEND_ESSAI_REG%\Epic"), r"C:\Donnees\Epic");
        assert_eq!(super::developper("%INCONNUE_XYZ%\\a"), "%INCONNUE_XYZ%\\a", "inconnue : laissée telle quelle");
        assert_eq!(super::developper("100 % sûr"), "100 % sûr");
    }
}
