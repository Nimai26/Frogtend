//! La connexion au dossier de sauvegarde (partage réseau SMB de Firehouse), avec le compte PROPRE à la personne.
//!
//! Frogtend ouvre une connexion à ce partage sans lui donner de lettre de lecteur (rien n'apparaît dans
//! l'explorateur), par l'appel de Windows prévu pour cela (`WNetAddConnection2W`) : le mot de passe ne passe jamais
//! par une ligne de commande. La connexion est refermée après usage.

use crate::erreurs::{Erreur, Resultat};

/// Une connexion ouverte à un partage ; refermée quand elle est lâchée.
pub struct Connexion {
    distant: Vec<u16>,
}

fn large(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Le motif lisible d'un code d'erreur Windows de connexion réseau.
pub fn motif(code: u32) -> String {
    let texte = match code {
        53 => "le serveur est introuvable sur le réseau (hors de la maison ?)",
        67 => "le partage est introuvable",
        86 | 1326 => "le compte ou le mot de passe est refusé",
        1219 => "une autre connexion à ce serveur existe déjà avec un autre compte",
        1311 => "aucun serveur de connexion n'est joignable",
        1231 | 1232 => "le serveur n'est pas joignable (réseau de la maison seulement)",
        5 => "accès refusé",
        _ => "erreur réseau",
    };
    format!("{texte} (code Windows {code})")
}

impl Connexion {
    /// Se connecte à `\\serveur\partage\dossier` avec ce compte.
    #[cfg(windows)]
    pub fn ouvrir(chemin: &str, compte: &str, mot_de_passe: &str) -> Resultat<Self> {
        use windows_sys::Win32::NetworkManagement::WNet::{WNetAddConnection2W, NETRESOURCEW, RESOURCETYPE_DISK};
        let mut distant = large(chemin);
        let compte_l = large(compte);
        let secret = large(mot_de_passe);
        let ressource = NETRESOURCEW {
            dwScope: 0,
            dwType: RESOURCETYPE_DISK,
            dwDisplayType: 0,
            dwUsage: 0,
            lpLocalName: std::ptr::null_mut(),
            lpRemoteName: distant.as_mut_ptr(),
            lpComment: std::ptr::null_mut(),
            lpProvider: std::ptr::null_mut(),
        };
        // SAFETY : chaînes UTF-16 terminées par zéro, vivantes pendant l'appel ; aucune lettre de lecteur.
        let code = unsafe { WNetAddConnection2W(&ressource, secret.as_ptr(), compte_l.as_ptr(), 0) };
        if code != 0 {
            return Err(Erreur::Disque(format!("Impossible d'ouvrir le dossier de sauvegarde : {}.", motif(code))));
        }
        Ok(Connexion { distant })
    }

    #[cfg(not(windows))]
    pub fn ouvrir(_chemin: &str, _compte: &str, _mot_de_passe: &str) -> Resultat<Self> {
        Err(Erreur::Refus("Le dossier de sauvegarde n'est accessible que sous Windows.".into()))
    }
}

impl Drop for Connexion {
    fn drop(&mut self) {
        #[cfg(windows)]
        {
            use windows_sys::Win32::NetworkManagement::WNet::WNetCancelConnection2W;
            // SAFETY : chaîne UTF-16 terminée par zéro ; on referme la connexion ouverte par `ouvrir`.
            unsafe {
                WNetCancelConnection2W(self.distant.as_ptr(), 0, 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_codes_windows_sont_expliques() {
        assert!(motif(1326).contains("mot de passe"));
        assert!(motif(53).contains("introuvable"));
        assert!(motif(1219).contains("autre compte"));
        assert!(motif(9999).contains("code Windows 9999"));
    }

    /// Essai RÉEL (avec l'accord de Seb, UNE fois) : identifiants demandés à Firehouse avec le jeton du profil, rangés
    /// dans le coffre Windows ; puis écriture, relecture et effacement d'un petit fichier dans
    /// `<partage>\<profil>\<PC>\`. `FROGTEND_PROFIL_ESSAI=<id> cargo test essai_partage_reel -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_partage_reel() {
        use crate::coffre::{Coffre, CoffreWindows};
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let nom_profil = std::env::var("FROGTEND_NOM_PROFIL_ESSAI").unwrap_or_else(|_| "Sebastien".into());
        let jeton = CoffreWindows.lire(&profil).unwrap().expect("pas de jeton pour ce profil");
        let c = crate::firehouse::Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap();

        let moi: serde_json::Value = c.obtenir_json("/moi").await.unwrap();
        println!("/moi.sauvegarde : {}", moi["sauvegarde"]);
        let id: serde_json::Value = c.obtenir_json("/sauvegarde/identifiants").await.unwrap();
        let (compte, partage) = (id["compte"].as_str().unwrap().to_string(), id["partage"].as_str().unwrap().to_string());
        let mot_de_passe = id["mot_de_passe"].as_str().unwrap().to_string();
        println!("identifiants reçus : compte {compte}, partage {partage}, mot de passe de {} caractères (non affiché)", mot_de_passe.len());
        let cle = format!("sauvegarde:{profil}");
        CoffreWindows.ranger(&cle, &mot_de_passe).unwrap();
        assert_eq!(CoffreWindows.lire(&cle).unwrap().as_deref(), Some(mot_de_passe.as_str()));
        println!("mot de passe rangé dans le coffre Windows (entrée « {cle} »)");

        let secret = CoffreWindows.lire(&cle).unwrap().unwrap();
        let connexion = match Connexion::ouvrir(&partage, &compte, &secret) {
            Ok(c) => c,
            Err(e) => panic!("CONNEXION : {e:?}"),
        };
        let pc = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "PC".into());
        let dossier = std::path::Path::new(&partage).join(&nom_profil).join(&pc);
        std::fs::create_dir_all(&dossier).unwrap();
        let fichier = dossier.join("essai-frogtend.txt");
        let contenu = format!("Essai de Frogtend {} le {}\n", env!("CARGO_PKG_VERSION"), crate::noyau::maintenant());
        std::fs::write(&fichier, &contenu).unwrap();
        let relu = std::fs::read_to_string(&fichier).unwrap();
        assert_eq!(relu, contenu);
        println!("ÉCRIT puis RELU : {} ({} octets) → identique", fichier.display(), relu.len());
        std::fs::remove_file(&fichier).unwrap();
        assert!(!fichier.exists());
        println!("EFFACÉ. Dossier {} laissé en place (vide).", dossier.display());
        drop(connexion);
        println!("connexion refermée");
    }
}
