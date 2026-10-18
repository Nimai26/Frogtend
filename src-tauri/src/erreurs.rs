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
        };
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("sorte", sorte)?;
        m.serialize_entry("motif", &self.to_string())?;
        if let Erreur::Fusionne { nouveau, .. } = self {
            m.serialize_entry("nouveau", nouveau)?;
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
        Erreur::Disque(format!("Impossible d'accéder au disque ({e})."))
    }
}
