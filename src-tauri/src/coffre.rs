//! Le jeton d'appareil de chaque profil, rangé dans le coffre de Windows (Gestionnaire d'identification).
//!
//! Le jeton n'est JAMAIS écrit dans un fichier, jamais journalisé, jamais renvoyé à l'interface.

use crate::erreurs::{Erreur, Resultat};
use std::collections::HashMap;
use std::sync::Mutex;

const SERVICE: &str = "fr.hikari-no-sekai.frogtend";

/// Un coffre à jetons. Celui de Windows en vrai ; un coffre en mémoire pour les tests.
pub trait Coffre: Send + Sync {
    fn ranger(&self, profil: &str, jeton: &str) -> Resultat<()>;
    fn lire(&self, profil: &str) -> Resultat<Option<String>>;
    fn oublier(&self, profil: &str) -> Resultat<()>;
}

/// Le coffre de Windows.
pub struct CoffreWindows;

fn entree(profil: &str) -> Resultat<keyring::Entry> {
    keyring::Entry::new(SERVICE, &format!("jeton:{profil}"))
        .map_err(|_| Erreur::Coffre("Le coffre de Windows est inaccessible.".into()))
}

impl Coffre for CoffreWindows {
    fn ranger(&self, profil: &str, jeton: &str) -> Resultat<()> {
        entree(profil)?
            .set_password(jeton)
            .map_err(|_| Erreur::Coffre("Impossible de ranger le jeton dans le coffre de Windows.".into()))
    }

    fn lire(&self, profil: &str) -> Resultat<Option<String>> {
        match entree(profil)?.get_password() {
            Ok(j) => Ok(Some(j)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(Erreur::Coffre("Impossible de lire le jeton dans le coffre de Windows.".into())),
        }
    }

    fn oublier(&self, profil: &str) -> Resultat<()> {
        match entree(profil)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(Erreur::Coffre("Impossible de retirer le jeton du coffre de Windows.".into())),
        }
    }
}

/// Un coffre en mémoire, pour les tests.
#[derive(Default)]
pub struct CoffreMemoire(Mutex<HashMap<String, String>>);

impl Coffre for CoffreMemoire {
    fn ranger(&self, profil: &str, jeton: &str) -> Resultat<()> {
        self.0.lock().unwrap().insert(profil.into(), jeton.into());
        Ok(())
    }
    fn lire(&self, profil: &str) -> Resultat<Option<String>> {
        Ok(self.0.lock().unwrap().get(profil).cloned())
    }
    fn oublier(&self, profil: &str) -> Resultat<()> {
        self.0.lock().unwrap().remove(profil);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_coffre_en_memoire_range_lit_et_oublie() {
        let c = CoffreMemoire::default();
        assert_eq!(c.lire("seb").unwrap(), None);
        c.ranger("seb", "secret").unwrap();
        assert_eq!(c.lire("seb").unwrap().as_deref(), Some("secret"));
        c.oublier("seb").unwrap();
        assert_eq!(c.lire("seb").unwrap(), None);
    }

    /// Vrai aller-retour dans le coffre de Windows, avec un profil de test effacé à la fin.
    #[test]
    #[cfg(windows)]
    fn le_coffre_de_windows_garde_le_jeton() {
        let c = CoffreWindows;
        let profil = "test-automatique-frogtend";
        c.ranger(profil, "valeur-de-test").unwrap();
        assert_eq!(c.lire(profil).unwrap().as_deref(), Some("valeur-de-test"));
        c.oublier(profil).unwrap();
        assert_eq!(c.lire(profil).unwrap(), None);
    }
}
