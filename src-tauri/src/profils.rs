//! Les profils de ce PC : un par personne, chacun avec son jeton (dans le coffre), son cache et ses réglages.
//!
//! Le code PIN n'est jamais gardé en clair : seule son empreinte (argon2) est écrite dans `profils.json`.

use crate::erreurs::{Erreur, Resultat};
use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const ESSAIS_AVANT_BLOCAGE: u32 = 5;
pub const DUREE_BLOCAGE: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Profil {
    pub id: String,
    pub nom: String,
    /// Empreinte argon2 du PIN, ou `None` si le profil n'est pas protégé.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub empreinte_pin: Option<String>,
}

/// Ce que l'interface voit d'un profil : jamais l'empreinte.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProfilVisible {
    pub id: String,
    pub nom: String,
    pub protege: bool,
}

impl From<&Profil> for ProfilVisible {
    fn from(p: &Profil) -> Self {
        ProfilVisible { id: p.id.clone(), nom: p.nom.clone(), protege: p.empreinte_pin.is_some() }
    }
}

/// Un PIN acceptable : 4 à 8 chiffres.
pub fn verifier_forme_pin(pin: &str) -> Resultat<()> {
    if (4..=8).contains(&pin.len()) && pin.chars().all(|c| c.is_ascii_digit()) {
        Ok(())
    } else {
        Err(Erreur::Pin("Le code PIN doit faire de 4 à 8 chiffres.".into()))
    }
}

fn empreinte(pin: &str) -> Resultat<String> {
    let sel = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(pin.as_bytes(), &sel)
        .map(|h| h.to_string())
        .map_err(|_| Erreur::Pin("Impossible de protéger le code PIN.".into()))
}

fn pin_correct(pin: &str, empreinte: &str) -> bool {
    PasswordHash::new(empreinte)
        .map(|h| Argon2::default().verify_password(pin.as_bytes(), &h).is_ok())
        .unwrap_or(false)
}

/// Un identifiant de profil : lettres minuscules et chiffres tirés du nom, plus un suffixe aléatoire.
fn nouvel_id(nom: &str) -> String {
    use rand::Rng;
    let base: String = nom
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            c => c,
        })
        .filter(|c| c.is_ascii_alphanumeric())
        .take(16)
        .collect();
    let suffixe: u32 = rand::thread_rng().gen_range(1000..10000);
    format!("{}-{suffixe}", if base.is_empty() { "profil" } else { &base })
}

/// La liste des profils, lue et écrite dans un fichier JSON du dossier de l'application.
pub struct Profils {
    fichier: PathBuf,
    liste: Mutex<Vec<Profil>>,
    /// Essais de PIN ratés, par profil : (nombre, instant du dernier échec).
    echecs: Mutex<HashMap<String, (u32, Instant)>>,
}

impl Profils {
    pub fn charger(fichier: &Path) -> Resultat<Self> {
        let liste = match std::fs::read_to_string(fichier) {
            Ok(t) => serde_json::from_str(&t)
                .map_err(|_| Erreur::Disque("Le fichier des profils est illisible.".into()))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e.into()),
        };
        Ok(Profils { fichier: fichier.into(), liste: Mutex::new(liste), echecs: Mutex::default() })
    }

    fn enregistrer(&self, liste: &[Profil]) -> Resultat<()> {
        if let Some(dossier) = self.fichier.parent() {
            std::fs::create_dir_all(dossier)?;
        }
        // Écriture à côté puis renommage : un arrêt brutal ne laisse jamais un fichier à moitié écrit.
        let provisoire = self.fichier.with_extension("json.tmp");
        std::fs::write(&provisoire, serde_json::to_string_pretty(liste).unwrap())?;
        std::fs::rename(&provisoire, &self.fichier)?;
        Ok(())
    }

    pub fn lister(&self) -> Vec<ProfilVisible> {
        self.liste.lock().unwrap().iter().map(ProfilVisible::from).collect()
    }

    pub fn trouver(&self, id: &str) -> Resultat<Profil> {
        self.liste
            .lock()
            .unwrap()
            .iter()
            .find(|p| p.id == id)
            .cloned()
            .ok_or_else(|| Erreur::Profil("Ce profil n'existe plus sur ce PC.".into()))
    }

    pub fn creer(&self, nom: &str, pin: Option<&str>) -> Resultat<ProfilVisible> {
        let nom = nom.trim();
        if nom.is_empty() || nom.chars().count() > 40 {
            return Err(Erreur::Profil("Le nom du profil doit faire de 1 à 40 caractères.".into()));
        }
        let mut liste = self.liste.lock().unwrap();
        if liste.iter().any(|p| p.nom.eq_ignore_ascii_case(nom)) {
            return Err(Erreur::Profil(format!("Un profil « {nom} » existe déjà sur ce PC.")));
        }
        let empreinte_pin = match pin {
            Some(p) => {
                verifier_forme_pin(p)?;
                Some(empreinte(p)?)
            }
            None => None,
        };
        let profil = Profil { id: nouvel_id(nom), nom: nom.into(), empreinte_pin };
        liste.push(profil.clone());
        self.enregistrer(&liste)?;
        Ok(ProfilVisible::from(&profil))
    }

    /// Vérifie le PIN d'un profil (rien à vérifier s'il n'est pas protégé). Bloque après trop d'échecs.
    pub fn verifier_pin(&self, id: &str, pin: Option<&str>) -> Resultat<()> {
        let profil = self.trouver(id)?;
        let Some(emp) = &profil.empreinte_pin else { return Ok(()) };

        let mut echecs = self.echecs.lock().unwrap();
        if let Some((n, dernier)) = echecs.get(id) {
            if *n >= ESSAIS_AVANT_BLOCAGE && dernier.elapsed() < DUREE_BLOCAGE {
                let reste = (DUREE_BLOCAGE - dernier.elapsed()).as_secs() + 1;
                return Err(Erreur::Pin(format!("Trop d'essais : attends {reste} s avant de réessayer.")));
            }
        }
        if pin.is_some_and(|p| pin_correct(p, emp)) {
            echecs.remove(id);
            Ok(())
        } else {
            let e = echecs.entry(id.into()).or_insert((0, Instant::now()));
            if e.0 >= ESSAIS_AVANT_BLOCAGE {
                *e = (0, Instant::now());
            }
            e.0 += 1;
            e.1 = Instant::now();
            Err(Erreur::Pin("Code PIN incorrect.".into()))
        }
    }

    /// Change (ou retire, avec `None`) le PIN, après vérification de l'ancien.
    pub fn changer_pin(&self, id: &str, ancien: Option<&str>, nouveau: Option<&str>) -> Resultat<()> {
        self.verifier_pin(id, ancien)?;
        let empreinte_pin = match nouveau {
            Some(p) => {
                verifier_forme_pin(p)?;
                Some(empreinte(p)?)
            }
            None => None,
        };
        let mut liste = self.liste.lock().unwrap();
        let p = liste.iter_mut().find(|p| p.id == id).ok_or_else(|| Erreur::Profil("Profil introuvable.".into()))?;
        p.empreinte_pin = empreinte_pin;
        self.enregistrer(&liste)
    }

    pub fn renommer(&self, id: &str, nom: &str) -> Resultat<()> {
        let nom = nom.trim();
        if nom.is_empty() || nom.chars().count() > 40 {
            return Err(Erreur::Profil("Le nom du profil doit faire de 1 à 40 caractères.".into()));
        }
        let mut liste = self.liste.lock().unwrap();
        if liste.iter().any(|p| p.id != id && p.nom.eq_ignore_ascii_case(nom)) {
            return Err(Erreur::Profil(format!("Un profil « {nom} » existe déjà sur ce PC.")));
        }
        let p = liste.iter_mut().find(|p| p.id == id).ok_or_else(|| Erreur::Profil("Profil introuvable.".into()))?;
        p.nom = nom.into();
        self.enregistrer(&liste)
    }

    /// Retire le profil de la liste (le jeton et le cache sont effacés par l'appelant).
    pub fn supprimer(&self, id: &str) -> Resultat<()> {
        let mut liste = self.liste.lock().unwrap();
        let avant = liste.len();
        liste.retain(|p| p.id != id);
        if liste.len() == avant {
            return Err(Erreur::Profil("Profil introuvable.".into()));
        }
        self.enregistrer(&liste)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profils() -> (tempfile::TempDir, Profils) {
        let d = tempfile::tempdir().unwrap();
        let p = Profils::charger(&d.path().join("profils.json")).unwrap();
        (d, p)
    }

    #[test]
    fn un_profil_cree_est_relu_apres_redemarrage() {
        let (d, p) = profils();
        let seb = p.creer("Sébastien", Some("1234")).unwrap();
        assert!(seb.id.starts_with("sebastien-"));
        assert!(seb.protege);
        let relu = Profils::charger(&d.path().join("profils.json")).unwrap();
        assert_eq!(relu.lister(), vec![seb]);
    }

    #[test]
    fn le_pin_n_est_jamais_ecrit_en_clair() {
        let (d, p) = profils();
        p.creer("Seb", Some("482913")).unwrap();
        let texte = std::fs::read_to_string(d.path().join("profils.json")).unwrap();
        assert!(!texte.contains("482913"));
        assert!(texte.contains("$argon2"));
    }

    #[test]
    fn la_liste_visible_ne_montre_pas_l_empreinte() {
        let (_d, p) = profils();
        p.creer("Seb", Some("1234")).unwrap();
        let json = serde_json::to_string(&p.lister()).unwrap();
        assert!(!json.contains("argon2"));
        assert!(json.contains("\"protege\":true"));
    }

    #[test]
    fn le_pin_doit_faire_4_a_8_chiffres() {
        let (_d, p) = profils();
        for faux in ["123", "123456789", "12a4", ""] {
            assert!(matches!(p.creer("X", Some(faux)), Err(Erreur::Pin(_))), "{faux}");
        }
    }

    #[test]
    fn deux_profils_ne_peuvent_pas_avoir_le_meme_nom() {
        let (_d, p) = profils();
        p.creer("Léa", None).unwrap();
        assert!(matches!(p.creer("Léa", None), Err(Erreur::Profil(_))));
        assert!(matches!(p.creer("léa", None), Err(Erreur::Profil(_))));
    }

    #[test]
    fn le_bon_pin_ouvre_le_mauvais_non() {
        let (_d, p) = profils();
        let id = p.creer("Seb", Some("1234")).unwrap().id;
        assert!(p.verifier_pin(&id, Some("1234")).is_ok());
        assert!(matches!(p.verifier_pin(&id, Some("0000")), Err(Erreur::Pin(_))));
        assert!(matches!(p.verifier_pin(&id, None), Err(Erreur::Pin(_))));
    }

    #[test]
    fn un_profil_sans_pin_s_ouvre_sans_code() {
        let (_d, p) = profils();
        let id = p.creer("Léa", None).unwrap().id;
        assert!(p.verifier_pin(&id, None).is_ok());
    }

    #[test]
    fn trop_d_essais_bloquent_meme_le_bon_pin() {
        let (_d, p) = profils();
        let id = p.creer("Seb", Some("1234")).unwrap().id;
        for _ in 0..ESSAIS_AVANT_BLOCAGE {
            let _ = p.verifier_pin(&id, Some("9999"));
        }
        let e = p.verifier_pin(&id, Some("1234")).unwrap_err();
        assert!(matches!(&e, Erreur::Pin(m) if m.contains("Trop d'essais")), "{e:?}");
    }

    #[test]
    fn changer_ou_retirer_le_pin_demande_l_ancien() {
        let (_d, p) = profils();
        let id = p.creer("Seb", Some("1234")).unwrap().id;
        assert!(p.changer_pin(&id, Some("0000"), Some("5678")).is_err());
        p.changer_pin(&id, Some("1234"), Some("5678")).unwrap();
        assert!(p.verifier_pin(&id, Some("5678")).is_ok());
        p.changer_pin(&id, Some("5678"), None).unwrap();
        assert!(!p.trouver(&id).unwrap().empreinte_pin.is_some());
    }

    #[test]
    fn supprimer_retire_le_profil() {
        let (_d, p) = profils();
        let id = p.creer("Seb", None).unwrap().id;
        p.supprimer(&id).unwrap();
        assert!(p.lister().is_empty());
        assert!(p.supprimer(&id).is_err());
    }
}
