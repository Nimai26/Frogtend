//! Lire le registre de Windows par son API (jamais `reg.exe` : sa sortie est dans la page de code de la console, un
//! chemin accentué y est abîmé). LECTURE seulement : Frogtend n'écrit jamais dans le registre (l'installeur le fait).

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

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(windows)]
    fn une_valeur_de_windows_se_lit_et_une_absente_rend_rien() {
        // Toujours présente sur Windows.
        let v = super::lire_texte_hklm(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion", "ProductName").unwrap();
        assert!(v.contains("Windows"), "{v}");
        assert_eq!(super::lire_texte_hkcu(r"Software\Frogtend-absent-xyz", "Rien"), None);
    }
}
