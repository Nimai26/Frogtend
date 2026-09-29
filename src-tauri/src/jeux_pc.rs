//! Les jeux présents sur CE PC (le registre), et où les ranger (les emplacements par système).
//!
//! Un jeu téléchargé est rangé UNE fois sur le PC. Chaque profil ne voit dans sa ludothèque que les jeux du PC que
//! Firehouse lui montre (son propre catalogue) : le filtre se fait en croisant ce registre avec le cache du profil.

use crate::erreurs::{Erreur, Resultat};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Marge gardée libre sur un disque en plus de la taille du jeu (1 Gio) : un disque plein rend Windows instable.
pub const MARGE_LIBRE: u64 = 1024 * 1024 * 1024;

/// Les emplacements de jeux de ce PC (réglés dans l'interface, enregistrés dans `pc.json`).
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct Emplacements {
    /// Pour les systèmes qui n'ont pas les leurs.
    #[serde(default)]
    pub defaut: Vec<String>,
    /// Par système (nom LaunchBox) : un ou plusieurs dossiers, dans l'ordre de préférence.
    #[serde(default)]
    pub systemes: BTreeMap<String, Vec<String>>,
}

impl Emplacements {
    /// Les dossiers à essayer pour un système : les siens, sinon ceux par défaut.
    pub fn pour(&self, systeme: &str) -> &[String] {
        match self.systemes.get(systeme) {
            Some(l) if !l.is_empty() => l,
            _ => &self.defaut,
        }
    }
}

/// Un emplacement proposé, avec ce qu'on sait de sa place.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EmplacementPropose {
    pub chemin: String,
    /// `None` : dossier introuvable (disque débranché, partage réseau absent).
    pub libre: Option<u64>,
    /// Vrai s'il reste assez de place pour ce jeu (marge comprise).
    pub assez: bool,
}

/// La place libre sur le disque d'un dossier, ou `None` s'il est introuvable.
pub fn place_libre(chemin: &Path) -> Option<u64> {
    if !chemin.is_dir() {
        return None;
    }
    fs2::available_space(chemin).ok()
}

/// Les emplacements d'un système avec leur place, dans l'ordre de préférence. Le premier qui convient est proposé.
pub fn proposer(emplacements: &Emplacements, systeme: &str, taille: u64) -> Vec<EmplacementPropose> {
    emplacements
        .pour(systeme)
        .iter()
        .map(|c| {
            let libre = place_libre(Path::new(c));
            EmplacementPropose { chemin: c.clone(), libre, assez: libre.is_some_and(|l| l >= taille + MARGE_LIBRE) }
        })
        .collect()
}

/// Un nom de dossier sûr pour Windows, tiré d'un titre (les FICHIERS du jeu, eux, gardent toujours leur nom).
pub fn nom_de_dossier(titre: &str) -> String {
    const INTERDITS: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    let propre: String = titre
        .chars()
        .map(|c| if INTERDITS.contains(&c) || c.is_control() { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let propre = propre.trim_end_matches(['.', ' ']).chars().take(80).collect::<String>();
    // Noms réservés de Windows.
    let reserve = ["CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "LPT1", "LPT2", "LPT3"];
    if propre.is_empty() || reserve.contains(&propre.to_uppercase().as_str()) {
        format!("Jeu {propre}").trim().to_string()
    } else {
        propre
    }
}

/// Un nom de FICHIER reçu de Firehouse est gardé tel quel (jamais renommé : ROM, images disque). On refuse
/// seulement ce qui sortirait du dossier du jeu.
pub fn nom_de_fichier_sur(nom: &str) -> Resultat<&str> {
    let dangereux = nom.is_empty()
        || nom.contains(['/', '\\', ':'])
        || nom == "."
        || nom == ".."
        || nom.chars().any(|c| c.is_control());
    if dangereux {
        Err(Erreur::Refus(format!("Firehouse a envoyé un nom de fichier inutilisable : « {nom} ».")))
    } else {
        Ok(nom)
    }
}

/// Où en est un jeu du PC.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Etat {
    /// En file, pas encore commencé.
    Attente,
    EnCours,
    /// Arrêté par la personne (ou par la fermeture de Frogtend) : reprendra où il en était.
    Pause,
    /// Tous les fichiers sont là, à la bonne taille.
    Telecharge,
    Erreur,
}

impl Etat {
    fn texte(self) -> &'static str {
        match self {
            Etat::Attente => "attente",
            Etat::EnCours => "en_cours",
            Etat::Pause => "pause",
            Etat::Telecharge => "telecharge",
            Etat::Erreur => "erreur",
        }
    }
    fn depuis(t: &str) -> Etat {
        match t {
            "attente" => Etat::Attente,
            "en_cours" => Etat::EnCours,
            "pause" => Etat::Pause,
            "telecharge" => Etat::Telecharge,
            _ => Etat::Erreur,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FichierJeu {
    pub n: u32,
    pub nom: String,
    pub taille: u64,
}

/// Un jeu présent (ou en cours d'arrivée) sur ce PC.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct JeuPc {
    pub id: i64,
    /// `telechargement_id` de la version choisie (0 est un vrai numéro : version du recensement).
    pub version: i64,
    pub titre: String,
    pub plateforme: String,
    /// Le dossier du jeu (dans l'emplacement choisi).
    pub dossier: String,
    pub etat: Etat,
    pub total: u64,
    pub fichiers: Vec<FichierJeu>,
    /// Le dernier motif d'erreur, s'il y en a un.
    pub message: Option<String>,
    pub ajoute_le: String,
    pub ajoute_par: String,
}

impl JeuPc {
    /// Les octets déjà sur le disque (fichiers finis et `.part`) : on compte ce qui est vraiment là.
    pub fn recus(&self) -> u64 {
        let d = Path::new(&self.dossier);
        self.fichiers
            .iter()
            .map(|f| {
                let fini = std::fs::metadata(d.join(&f.nom)).map(|m| m.len()).ok();
                fini.unwrap_or_else(|| std::fs::metadata(d.join(format!("{}.part", f.nom))).map(|m| m.len()).unwrap_or(0))
                    .min(f.taille)
            })
            .sum()
    }
}

/// Le registre des jeux du PC (SQLite, `jeux.db` dans le dossier de l'application).
pub struct Registre {
    db: Connection,
    pub fichier: PathBuf,
}

impl Registre {
    pub fn ouvrir(fichier: &Path) -> Resultat<Self> {
        if let Some(d) = fichier.parent() {
            std::fs::create_dir_all(d)?;
        }
        let db = Connection::open(fichier)?;
        db.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS jeux_pc (
               id INTEGER PRIMARY KEY, version INTEGER NOT NULL, titre TEXT NOT NULL, plateforme TEXT NOT NULL,
               dossier TEXT NOT NULL, etat TEXT NOT NULL, total INTEGER NOT NULL, fichiers TEXT NOT NULL,
               message TEXT, ajoute_le TEXT NOT NULL, ajoute_par TEXT NOT NULL);",
        )?;
        Ok(Registre { db, fichier: fichier.into() })
    }

    pub fn ajouter(&self, j: &JeuPc) -> Resultat<()> {
        let deja: Option<String> =
            self.db.query_row("SELECT titre FROM jeux_pc WHERE id = ?1", [j.id], |r| r.get(0)).optional()?;
        if deja.is_some() {
            return Err(Erreur::Refus(format!("« {} » est déjà dans la ludothèque de ce PC.", j.titre)));
        }
        self.db.execute(
            "INSERT INTO jeux_pc (id, version, titre, plateforme, dossier, etat, total, fichiers, message, ajoute_le, ajoute_par)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                j.id,
                j.version,
                j.titre,
                j.plateforme,
                j.dossier,
                j.etat.texte(),
                j.total as i64,
                serde_json::to_string(&j.fichiers).unwrap(),
                j.message,
                j.ajoute_le,
                j.ajoute_par
            ],
        )?;
        Ok(())
    }

    fn lire(r: &rusqlite::Row) -> rusqlite::Result<JeuPc> {
        let fichiers: String = r.get(7)?;
        Ok(JeuPc {
            id: r.get(0)?,
            version: r.get(1)?,
            titre: r.get(2)?,
            plateforme: r.get(3)?,
            dossier: r.get(4)?,
            etat: Etat::depuis(&r.get::<_, String>(5)?),
            total: r.get::<_, i64>(6)? as u64,
            fichiers: serde_json::from_str(&fichiers).unwrap_or_default(),
            message: r.get(8)?,
            ajoute_le: r.get(9)?,
            ajoute_par: r.get(10)?,
        })
    }

    const COLONNES: &'static str =
        "id, version, titre, plateforme, dossier, etat, total, fichiers, message, ajoute_le, ajoute_par";

    pub fn jeu(&self, id: i64) -> Resultat<Option<JeuPc>> {
        Ok(self
            .db
            .query_row(&format!("SELECT {} FROM jeux_pc WHERE id = ?1", Self::COLONNES), [id], Self::lire)
            .optional()?)
    }

    pub fn tous(&self) -> Resultat<Vec<JeuPc>> {
        let mut st = self.db.prepare(&format!("SELECT {} FROM jeux_pc ORDER BY ajoute_le", Self::COLONNES))?;
        let l = st.query_map([], Self::lire)?.collect::<Result<_, _>>()?;
        Ok(l)
    }

    /// Le prochain jeu à télécharger (le plus ancien en attente).
    pub fn prochain(&self) -> Resultat<Option<JeuPc>> {
        Ok(self
            .db
            .query_row(
                &format!("SELECT {} FROM jeux_pc WHERE etat = 'attente' ORDER BY ajoute_le LIMIT 1", Self::COLONNES),
                [],
                Self::lire,
            )
            .optional()?)
    }

    pub fn changer_etat(&self, id: i64, etat: Etat, message: Option<&str>) -> Resultat<()> {
        self.db.execute("UPDATE jeux_pc SET etat = ?2, message = ?3 WHERE id = ?1", params![id, etat.texte(), message])?;
        Ok(())
    }

    /// Au démarrage : un téléchargement « en cours » a été interrompu par la fermeture de Frogtend. Il passe en
    /// pause : il ne reprend qu'à la demande (jamais de téléchargement sans accord).
    pub fn interrompus_en_pause(&self) -> Resultat<usize> {
        Ok(self.db.execute(
            "UPDATE jeux_pc SET etat = 'pause', message = 'Frogtend a été fermé pendant le téléchargement.'
             WHERE etat = 'en_cours'",
            [],
        )?)
    }

    /// Retire un jeu du registre (ses fichiers, eux, ne sont effacés que par l'appelant, après vérification).
    pub fn retirer(&self, id: i64) -> Resultat<()> {
        self.db.execute("DELETE FROM jeux_pc WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Les ids de tous les jeux du PC (pour croiser avec le catalogue d'un profil).
    pub fn ids(&self) -> Resultat<Vec<i64>> {
        let mut st = self.db.prepare("SELECT id FROM jeux_pc")?;
        let l = st.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
        Ok(l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jeu(id: i64, dossier: &Path) -> JeuPc {
        JeuPc {
            id,
            version: 200,
            titre: "Dune".into(),
            plateforme: "MS-DOS".into(),
            dossier: dossier.to_string_lossy().into(),
            etat: Etat::Attente,
            total: 10,
            fichiers: vec![FichierJeu { n: 0, nom: "Dune (1992) [MS-DOS].exe".into(), taille: 10 }],
            message: None,
            ajoute_le: "1".into(),
            ajoute_par: "seb".into(),
        }
    }

    #[test]
    fn les_emplacements_d_un_systeme_passent_avant_ceux_par_defaut() {
        let mut e = Emplacements { defaut: vec!["D:\\Jeux".into()], ..Default::default() };
        assert_eq!(e.pour("MS-DOS"), ["D:\\Jeux".to_string()]);
        e.systemes.insert("MS-DOS".into(), vec!["E:\\DOS".into(), "F:\\DOS".into()]);
        assert_eq!(e.pour("MS-DOS"), ["E:\\DOS".to_string(), "F:\\DOS".to_string()]);
        e.systemes.insert("Arcade".into(), vec![]);
        assert_eq!(e.pour("Arcade"), ["D:\\Jeux".to_string()], "une liste vide retombe sur le défaut");
    }

    #[test]
    fn un_dossier_introuvable_est_signale_sans_etre_propose() {
        let d = tempfile::tempdir().unwrap();
        let e = Emplacements {
            defaut: vec!["Z:\\disque-debranche-frogtend".into(), d.path().to_string_lossy().into()],
            ..Default::default()
        };
        let p = proposer(&e, "MS-DOS", 1000);
        assert_eq!(p[0].libre, None);
        assert!(!p[0].assez);
        assert!(p[1].libre.unwrap() > 0);
        // Un jeu plus gros que le disque : pas assez de place.
        assert!(!proposer(&e, "MS-DOS", u64::MAX / 2)[1].assez);
    }

    #[test]
    fn un_nom_de_dossier_est_sur_pour_windows() {
        assert_eq!(nom_de_dossier("The Legend of Zelda: Ocarina of Time"), "The Legend of Zelda Ocarina of Time");
        assert_eq!(nom_de_dossier("Dune II / La Bataille?"), "Dune II La Bataille");
        assert_eq!(nom_de_dossier("CON"), "Jeu CON");
        assert_eq!(nom_de_dossier("Fin. . "), "Fin");
    }

    #[test]
    fn un_nom_de_fichier_n_est_jamais_modifie_mais_ne_sort_pas_du_dossier() {
        assert_eq!(nom_de_fichier_sur("Super Mario World (USA).sfc").unwrap(), "Super Mario World (USA).sfc");
        assert_eq!(nom_de_fichier_sur("Dune (1992) [MS-DOS].exe").unwrap(), "Dune (1992) [MS-DOS].exe");
        for mauvais in ["..", "a/b.rom", "..\\x", "C:truc", ""] {
            assert!(nom_de_fichier_sur(mauvais).is_err(), "{mauvais}");
        }
    }

    #[test]
    fn le_registre_garde_les_jeux_et_refuse_un_doublon() {
        let d = tempfile::tempdir().unwrap();
        let r = Registre::ouvrir(&d.path().join("jeux.db")).unwrap();
        r.ajouter(&jeu(110, d.path())).unwrap();
        assert!(r.ajouter(&jeu(110, d.path())).is_err());
        assert_eq!(r.prochain().unwrap().unwrap().id, 110);
        r.changer_etat(110, Etat::EnCours, None).unwrap();
        assert_eq!(r.prochain().unwrap(), None);
        assert_eq!(r.interrompus_en_pause().unwrap(), 1);
        assert_eq!(r.jeu(110).unwrap().unwrap().etat, Etat::Pause);
        assert_eq!(r.ids().unwrap(), vec![110]);
    }

    #[test]
    fn les_octets_recus_se_comptent_sur_le_disque() {
        let d = tempfile::tempdir().unwrap();
        let j = jeu(110, d.path());
        assert_eq!(j.recus(), 0);
        std::fs::write(d.path().join("Dune (1992) [MS-DOS].exe.part"), b"1234").unwrap();
        assert_eq!(j.recus(), 4);
        std::fs::write(d.path().join("Dune (1992) [MS-DOS].exe"), b"0123456789").unwrap();
        assert_eq!(j.recus(), 10);
    }
}
