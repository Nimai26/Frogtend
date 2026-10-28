//! Les erreurs que le cœur renvoie à l'interface.
//!
//! Chaque erreur a une SORTE (pour que l'interface réagisse) et un MOTIF en français, lisible par une personne
//! non experte. Jamais de chemin d'API, jamais de jeton, jamais de détail technique brut dans le motif.

use serde::Serialize;

/// Vers l'interface : TOUJOURS `{sorte, motif}` (et `nouveau` pour une fiche réunie) — écrit à la main pour qu'une
/// erreur qui porte une donnée ne change pas cette forme.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum Erreur {
    /// 401 : le jeton est révoqué ou expiré. On le dit, on ne réessaie pas en boucle.
    #[error("{0}")]
    JetonRefuse(String),
    /// 404 : absent, ou caché à ce grade (Firehouse ne fait pas la différence, et nous non plus).
    #[error("{0}")]
    Introuvable(String),
    /// 409 : fichier abîmé sur le serveur, ou fiche à relire.
    #[error("{0}")]
    Conflit(String),
    /// Autre refus du serveur (400, 403…), avec son motif.
    #[error("{0}")]
    Refus(String),
    /// 416 : la reprise d'un téléchargement n'est plus possible (le morceau déjà reçu ne correspond plus au fichier).
    /// SEULE erreur qui autorise à effacer le `.part` déjà reçu : toute autre (404, 409, 410, 503…) le garde.
    #[error("{0}")]
    Plage(String),
    /// Le serveur ne répond pas correctement (5xx, réponse illisible).
    #[error("{0}")]
    Serveur(String),
    /// Pas de réseau, adresse injoignable, délai dépassé.
    #[error("{0}")]
    Reseau(String),
    /// Le coffre de Windows n'a pas pu lire ou écrire le jeton.
    #[error("{0}")]
    Coffre(String),
    /// Code PIN faux ou manquant.
    #[error("{0}")]
    Pin(String),
    /// Profil inconnu, ou aucun profil ouvert.
    #[error("{0}")]
    Profil(String),
    /// Réglage invalide (adresse…).
    #[error("{0}")]
    Reglage(String),
    /// Firehouse parle une autre version MAJEURE du contrat (`X-Api-Jeux-Version`).
    #[error("{0}")]
    Version(String),
    /// Lecture ou écriture sur le disque de ce PC.
    #[error("{0}")]
    Disque(String),
    /// 410 (contrat 1.14) : la fiche a été RÉUNIE à `nouveau` chez Firehouse.
    #[error("{motif}")]
    Fusionne { motif: String, nouveau: i64 },
    /// L'opération venait d'une session remplacée (profil, jeton ou connexion changés pendant qu'elle tournait) : rien
    /// n'a été gardé ni rendu. Pas un échec à montrer : l'interface l'ignore.
    #[error("{0}")]
    Perimee(String),
    /// Refusé avant d'agir parce que la personne doit d'abord savoir quelque chose (une image Xbox que xemu ne lit
    /// pas…) : l'interface l'explique dans une fenêtre, pas dans un simple message qui passe.
    #[error("{0}")]
    Prevenir(String),
    /// Une carte e-Reader dont la cartouche (`cartouche` : sa fiche chez Firehouse) n'est pas sur ce PC : l'interface
    /// propose de l'y mettre (téléchargement, après un oui).
    #[error("{motif}")]
    CartoucheAbsente { motif: String, cartouche: i64 },
}

impl Serialize for Erreur {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let sorte = match self {
            Erreur::JetonRefuse(_) => "jeton_refuse",
            Erreur::Introuvable(_) => "introuvable",
            Erreur::Conflit(_) => "conflit",
            Erreur::Refus(_) => "refus",
            Erreur::Plage(_) => "plage",
            Erreur::Serveur(_) => "serveur",
            Erreur::Reseau(_) => "reseau",
            Erreur::Coffre(_) => "coffre",
            Erreur::Pin(_) => "pin",
            Erreur::Profil(_) => "profil",
            Erreur::Reglage(_) => "reglage",
            Erreur::Version(_) => "version",
            Erreur::Disque(_) => "disque",
            Erreur::Fusionne { .. } => "fusionne",
            Erreur::Perimee(_) => "perimee",
            Erreur::Prevenir(_) => "prevenir",
            Erreur::CartoucheAbsente { .. } => "cartouche_absente",
        };
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("sorte", sorte)?;
        m.serialize_entry("motif", &self.to_string())?;
        if let Erreur::Fusionne { nouveau, .. } = self {
            m.serialize_entry("nouveau", nouveau)?;
        }
        if let Erreur::CartoucheAbsente { cartouche, .. } = self {
            m.serialize_entry("cartouche", cartouche)?;
        }
        m.end()
    }
}

#[cfg(test)]
mod tests_forme {
    use super::*;

    #[test]
    fn une_erreur_garde_la_forme_sorte_motif_pour_l_interface() {
        let v = serde_json::to_value(Erreur::Introuvable("absent".into())).unwrap();
        assert_eq!(v, serde_json::json!({"sorte": "introuvable", "motif": "absent"}));
        let v = serde_json::to_value(Erreur::JetonRefuse("x".into())).unwrap();
        assert_eq!(v["sorte"], "jeton_refuse");
        let v = serde_json::to_value(Erreur::Fusionne { motif: "réuni à la fiche 324".into(), nouveau: 324 }).unwrap();
        assert_eq!(v, serde_json::json!({"sorte": "fusionne", "motif": "réuni à la fiche 324", "nouveau": 324}));
        let v = serde_json::to_value(Erreur::Prevenir("à convertir".into())).unwrap();
        assert_eq!(v, serde_json::json!({"sorte": "prevenir", "motif": "à convertir"}));
    }
}

pub type Resultat<T> = Result<T, Erreur>;

impl From<rusqlite::Error> for Erreur {
    fn from(e: rusqlite::Error) -> Self {
        Erreur::Disque(format!("Impossible de lire le cache de la ludothèque ({e})."))
    }
}

impl From<std::io::Error> for Erreur {
    fn from(e: std::io::Error) -> Self {
        Erreur::Disque(message_disque(&e))
    }
}

/// Une erreur de fichier dite simplement (essai de Seb, 28/10 : « Impossible d'accéder au disque (Le processus ne
/// peut pas accéder au fichier… (os error 32)) »), avec le code de Windows à la fin pour qui doit chercher.
pub fn message_disque(e: &std::io::Error) -> String {
    let phrase = match e.raw_os_error() {
        Some(32 | 33) => "Un fichier est utilisé par un autre programme (une installation en cours, l'émulateur ouvert, ou l'antivirus qui l'examine). Attends un instant, puis réessaie.",
        Some(5) => "Windows refuse l'accès à un fichier ou un dossier (dossier protégé, fichier en lecture seule, ou l'antivirus).",
        Some(112 | 39) => "Le disque est plein : libère de la place, puis réessaie.",
        Some(2 | 3) => "Un fichier ou un dossier est introuvable (disque débranché ?).",
        Some(21) => "Le disque n'est pas prêt (débranché ?).",
        _ => match e.kind() {
            std::io::ErrorKind::NotFound => "Un fichier ou un dossier est introuvable (disque débranché ?).",
            std::io::ErrorKind::PermissionDenied => "Windows refuse l'accès à ce dossier (dossier protégé, ou l'antivirus).",
            _ => "Problème d'accès au disque.",
        },
    };
    let connu = !phrase.starts_with("Problème");
    match e.raw_os_error() {
        Some(n) if connu => format!("{phrase} (code Windows {n})"),
        _ => format!("{phrase} ({e})"),
    }
}

#[cfg(test)]
mod tests_disque {
    #[test]
    fn une_erreur_de_fichier_est_dite_simplement() {
        let m = super::message_disque(&std::io::Error::from_raw_os_error(32));
        assert!(m.starts_with("Un fichier est utilisé par un autre programme") && m.ends_with("(code Windows 32)"), "{m}");
        assert!(super::message_disque(&std::io::Error::from_raw_os_error(112)).contains("plein"));
        assert!(super::message_disque(&std::io::Error::from_raw_os_error(5)).contains("refuse l'accès"));
        assert!(super::message_disque(&std::io::Error::new(std::io::ErrorKind::NotFound, "x")).contains("introuvable"));
        assert!(super::message_disque(&std::io::Error::other("bizarre")).starts_with("Problème d'accès au disque"));
    }
}
