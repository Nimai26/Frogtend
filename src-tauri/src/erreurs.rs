//! Les erreurs que le cœur renvoie à l'interface.
//!
//! Chaque erreur a une SORTE (pour que l'interface réagisse) et un MOTIF en français, lisible par une personne
//! non experte. Jamais de chemin d'API, jamais de jeton, jamais de détail technique brut dans le motif.

use serde::Serialize;

#[derive(Debug, thiserror::Error, Serialize, PartialEq)]
#[serde(tag = "sorte", content = "motif", rename_all = "snake_case")]
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
    /// Lecture ou écriture sur le disque de ce PC.
    #[error("{0}")]
    Disque(String),
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
