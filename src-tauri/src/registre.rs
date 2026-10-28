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
}
