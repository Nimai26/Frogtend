//! La ludothèque d'un profil : le catalogue reçu de Firehouse, gardé en cache sur ce PC (SQLite, un fichier par
//! profil), pour rester consultable hors ligne.
//!
//! Règle : rien de ce qui a été reçu pour un profil n'est lu pour un autre. Chaque profil a son propre fichier.
//!
//! La forme exacte de `/plateformes` et `/catalogue` n'est pas encore écrite dans le contrat : la lecture est
//! TOLÉRANTE (voir `lire_plateformes` et `lire_page`) et c'est le seul endroit à ajuster.

use crate::erreurs::Resultat;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Plateforme {
    pub nom: String,
    pub jeux: u64,
}

/// Ce qu'une carte de la grille affiche d'un jeu.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct JeuResume {
    pub id: i64,
    pub titre: String,
    #[serde(default)]
    pub annee: Option<i64>,
    #[serde(default)]
    pub plateforme: String,
    #[serde(default)]
    pub genres: Vec<String>,
    #[serde(default)]
    pub developpeur: Option<String>,
    #[serde(default)]
    pub editeur: Option<String>,
    #[serde(default)]
    pub statut: Option<String>,
    /// Firehouse a-t-il une jaquette pour ce jeu ? (`None` : inconnu, on essaie.)
    #[serde(default)]
    pub jaquette: Option<bool>,
    /// Nombre de versions rangées dans Firehouse.
    #[serde(default)]
    pub versions: Option<u32>,
    /// Dernière modification de la fiche dans Firehouse.
    #[serde(default)]
    pub maj_le: Option<String>,
}

/// Une page de catalogue lue, et s'il en reste après.
#[derive(Debug, PartialEq)]
pub struct Page {
    pub jeux: Vec<JeuResume>,
    pub encore: Option<bool>,
}

/// Le premier tableau trouvé : la valeur elle-même, ou l'un des champs usuels d'un objet.
fn tableau<'a>(v: &'a Value, champs: &[&str]) -> Option<&'a Vec<Value>> {
    v.as_array().or_else(|| champs.iter().find_map(|c| v.get(*c).and_then(Value::as_array)))
}

/// Lit la réponse de `/plateformes` : un tableau, ou un objet qui en contient un.
/// Chaque élément donne un nom (`nom`, `plateforme`, `name`) et un nombre de jeux (`jeux`, `nombre`, `total`…).
pub fn lire_plateformes(v: &Value) -> Vec<Plateforme> {
    let Some(liste) = tableau(v, &["plateformes", "items", "resultats"]) else { return vec![] };
    liste
        .iter()
        .filter_map(|p| {
            let nom = ["nom", "plateforme", "name"].iter().find_map(|c| p.get(*c)?.as_str())?;
            let jeux = ["jeux", "nombre", "nb_jeux", "total", "count"].iter().find_map(|c| p.get(*c)?.as_u64());
            Some(Plateforme { nom: nom.into(), jeux: jeux.unwrap_or(0) })
        })
        .collect()
}

/// Lit une page de `/catalogue`.
pub fn lire_page(v: &Value, page: u32) -> Page {
    let jeux: Vec<JeuResume> = tableau(v, &["jeux", "items", "resultats", "catalogue"])
        .map(|l| l.iter().filter_map(|j| serde_json::from_value(j.clone()).ok()).collect())
        .unwrap_or_default();
    let encore = if let Some(pages) = v.get("pages").and_then(Value::as_u64) {
        Some(u64::from(page) < pages)
    } else if let Some(s) = v.get("suivante") {
        Some(!s.is_null() && s != &Value::Bool(false))
    } else {
        None // inconnu : on s'arrête à la première page vide
    };
    Page { jeux, encore }
}

/// Texte de tri et de recherche : minuscules, sans accents.
pub fn normaliser(t: &str) -> String {
    t.to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ä' | 'ã' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' => 'i',
            'ô' | 'ö' | 'ó' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            c => c,
        })
        .collect()
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Filtre {
    #[serde(default)]
    pub plateforme: Option<String>,
    #[serde(default)]
    pub texte: Option<String>,
    #[serde(default)]
    pub genre: Option<String>,
    /// `titre` (par défaut), `annee`, `annee_desc`.
    #[serde(default)]
    pub tri: Option<String>,
    #[serde(default)]
    pub limite: Option<u32>,
    #[serde(default)]
    pub decalage: Option<u32>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Liste {
    pub jeux: Vec<JeuResume>,
    /// Nombre de jeux qui correspondent au filtre (toutes pages).
    pub total: u64,
    /// Nombre de jeux de la ludothèque, sans filtre.
    pub total_ludotheque: u64,
}

pub struct Cache {
    db: Connection,
}

impl Cache {
    pub fn ouvrir(fichier: &Path) -> Resultat<Self> {
        if let Some(d) = fichier.parent() {
            std::fs::create_dir_all(d)?;
        }
        let db = Connection::open(fichier)?;
        Self::preparer(db)
    }

    pub fn en_memoire() -> Resultat<Self> {
        Self::preparer(Connection::open_in_memory()?)
    }

    fn preparer(db: Connection) -> Resultat<Self> {
        db.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS jeux (
               id INTEGER PRIMARY KEY, titre TEXT NOT NULL, titre_tri TEXT NOT NULL, annee INTEGER,
               plateforme TEXT NOT NULL, genres TEXT NOT NULL, brut TEXT NOT NULL);
             CREATE INDEX IF NOT EXISTS jeux_plateforme ON jeux(plateforme, titre_tri);
             CREATE TABLE IF NOT EXISTS plateformes (nom TEXT PRIMARY KEY, jeux INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS fiches (id INTEGER PRIMARY KEY, brut TEXT NOT NULL, lue_le TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS meta (cle TEXT PRIMARY KEY, valeur TEXT NOT NULL);",
        )?;
        Ok(Cache { db })
    }

    /// Remplace TOUT le catalogue d'un coup (transaction) : un jeu devenu invisible pour ce profil disparaît.
    pub fn remplacer(&mut self, plateformes: &[Plateforme], jeux: &[JeuResume], quand: &str) -> Resultat<()> {
        let tx = self.db.transaction()?;
        tx.execute("DELETE FROM jeux", [])?;
        tx.execute("DELETE FROM plateformes", [])?;
        // Les fiches gardées pour des jeux qui ne sont plus visibles partent aussi.
        {
            let mut ins = tx.prepare(
                "INSERT OR REPLACE INTO jeux (id, titre, titre_tri, annee, plateforme, genres, brut)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?;
            for j in jeux {
                ins.execute(params![
                    j.id,
                    j.titre,
                    normaliser(&j.titre),
                    j.annee,
                    j.plateforme,
                    serde_json::to_string(&j.genres).unwrap(),
                    serde_json::to_string(j).unwrap(),
                ])?;
            }
            let mut ins = tx.prepare("INSERT OR REPLACE INTO plateformes (nom, jeux) VALUES (?1, ?2)")?;
            for p in plateformes {
                ins.execute(params![p.nom, p.jeux as i64])?;
            }
        }
        tx.execute("DELETE FROM fiches WHERE id NOT IN (SELECT id FROM jeux)", [])?;
        tx.execute("INSERT OR REPLACE INTO meta (cle, valeur) VALUES ('synchronise_le', ?1)", [quand])?;
        tx.commit()?;
        Ok(())
    }

    pub fn synchronise_le(&self) -> Resultat<Option<String>> {
        Ok(self.meta("synchronise_le")?)
    }

    pub fn meta(&self, cle: &str) -> Resultat<Option<String>> {
        Ok(self.db.query_row("SELECT valeur FROM meta WHERE cle = ?1", [cle], |r| r.get(0)).optional()?)
    }

    pub fn ecrire_meta(&self, cle: &str, valeur: &str) -> Resultat<()> {
        self.db.execute("INSERT OR REPLACE INTO meta (cle, valeur) VALUES (?1, ?2)", [cle, valeur])?;
        Ok(())
    }

    /// Les plateformes, avec le nombre de jeux présents dans le cache (plus fiable que celui annoncé).
    pub fn plateformes(&self) -> Resultat<Vec<Plateforme>> {
        let mut st = self.db.prepare(
            "SELECT nom, jeux FROM (
               SELECT p.nom AS nom, COALESCE(n.c, 0) AS jeux FROM plateformes p
               LEFT JOIN (SELECT plateforme AS pf, COUNT(*) AS c FROM jeux GROUP BY pf) n ON n.pf = p.nom
               UNION
               SELECT plateforme AS nom, COUNT(*) AS jeux FROM jeux
               WHERE plateforme NOT IN (SELECT nom FROM plateformes) GROUP BY plateforme)
             WHERE jeux > 0
             ORDER BY nom COLLATE NOCASE",
        )?;
        let liste = st
            .query_map([], |r| Ok(Plateforme { nom: r.get(0)?, jeux: r.get::<_, i64>(1)? as u64 }))?
            .collect::<Result<_, _>>()?;
        Ok(liste)
    }

    /// Les genres présents (pour le filtre), triés.
    pub fn genres(&self, plateforme: Option<&str>) -> Resultat<Vec<String>> {
        let mut st = self.db.prepare(
            "SELECT DISTINCT g.value FROM jeux, json_each(jeux.genres) g
             WHERE (?1 IS NULL OR plateforme = ?1) ORDER BY 1 COLLATE NOCASE",
        )?;
        let liste = st.query_map([plateforme], |r| r.get(0))?.collect::<Result<_, _>>()?;
        Ok(liste)
    }

    pub fn lister(&self, f: &Filtre) -> Resultat<Liste> {
        let texte = f.texte.as_deref().map(str::trim).filter(|t| !t.is_empty()).map(|t| format!("%{}%", normaliser(t)));
        let genre = f.genre.as_deref().filter(|g| !g.is_empty()).map(|g| serde_json::to_string(g).unwrap());
        let condition = "(?1 IS NULL OR plateforme = ?1) AND (?2 IS NULL OR titre_tri LIKE ?2)
                         AND (?3 IS NULL OR instr(genres, ?3) > 0)";
        let ordre = match f.tri.as_deref() {
            Some("annee") => "annee IS NULL, annee, titre_tri",
            Some("annee_desc") => "annee IS NULL, annee DESC, titre_tri",
            _ => "titre_tri",
        };
        let limite = f.limite.unwrap_or(200).min(1000);
        let decalage = f.decalage.unwrap_or(0);
        let p = params![f.plateforme, texte, genre];

        let total: i64 = self.db.query_row(&format!("SELECT COUNT(*) FROM jeux WHERE {condition}"), p, |r| r.get(0))?;
        let total_ludotheque: i64 = self.db.query_row("SELECT COUNT(*) FROM jeux", [], |r| r.get(0))?;
        let mut st = self.db.prepare(&format!(
            "SELECT brut FROM jeux WHERE {condition} ORDER BY {ordre} LIMIT {limite} OFFSET {decalage}"
        ))?;
        let jeux = st
            .query_map(params![f.plateforme, texte, genre], |r| r.get::<_, String>(0))?
            .filter_map(|b| b.ok().and_then(|b| serde_json::from_str(&b).ok()))
            .collect();
        Ok(Liste { jeux, total: total as u64, total_ludotheque: total_ludotheque as u64 })
    }

    /// Un jeu tiré au hasard (dans la plateforme, si elle est donnée).
    pub fn au_hasard(&self, plateforme: Option<&str>) -> Resultat<Option<JeuResume>> {
        let brut: Option<String> = self
            .db
            .query_row(
                "SELECT brut FROM jeux WHERE (?1 IS NULL OR plateforme = ?1) ORDER BY random() LIMIT 1",
                [plateforme],
                |r| r.get(0),
            )
            .optional()?;
        Ok(brut.and_then(|b| serde_json::from_str(&b).ok()))
    }

    pub fn jeu(&self, id: i64) -> Resultat<Option<JeuResume>> {
        let brut: Option<String> =
            self.db.query_row("SELECT brut FROM jeux WHERE id = ?1", [id], |r| r.get(0)).optional()?;
        Ok(brut.and_then(|b| serde_json::from_str(&b).ok()))
    }

    pub fn contient(&self, id: i64) -> Resultat<bool> {
        Ok(self.db.query_row("SELECT 1 FROM jeux WHERE id = ?1", [id], |_| Ok(())).optional()?.is_some())
    }

    pub fn garder_fiche(&self, id: i64, fiche: &Value, quand: &str) -> Resultat<()> {
        self.db.execute(
            "INSERT OR REPLACE INTO fiches (id, brut, lue_le) VALUES (?1, ?2, ?3)",
            params![id, fiche.to_string(), quand],
        )?;
        Ok(())
    }

    pub fn fiche(&self, id: i64) -> Resultat<Option<Value>> {
        let brut: Option<String> =
            self.db.query_row("SELECT brut FROM fiches WHERE id = ?1", [id], |r| r.get(0)).optional()?;
        Ok(brut.and_then(|b| serde_json::from_str(&b).ok()))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    fn jeu(id: i64, titre: &str, plateforme: &str, annee: Option<i64>, genres: &[&str]) -> JeuResume {
        JeuResume {
            id,
            titre: titre.into(),
            plateforme: plateforme.into(),
            annee,
            genres: genres.iter().map(|g| g.to_string()).collect(),
            ..Default::default()
        }
    }

    fn cache_exemple() -> Cache {
        let mut c = Cache::en_memoire().unwrap();
        c.remplacer(
            &[Plateforme { nom: "MS-DOS".into(), jeux: 2 }, Plateforme { nom: "Super Nintendo".into(), jeux: 1 }],
            &[
                jeu(110, "Dune", "MS-DOS", Some(1992), &["Aventure", "Stratégie"]),
                jeu(111, "Écho du passé", "MS-DOS", Some(1995), &["Aventure"]),
                jeu(200, "Super Metroid", "Super Nintendo", Some(1994), &["Action"]),
            ],
            "2026-09-29T10:00:00Z",
        )
        .unwrap();
        c
    }

    #[test]
    fn lit_les_plateformes_sous_plusieurs_formes() {
        let attendu = vec![Plateforme { nom: "MS-DOS".into(), jeux: 812 }];
        assert_eq!(lire_plateformes(&json!([{"nom": "MS-DOS", "jeux": 812}])), attendu);
        assert_eq!(lire_plateformes(&json!({"plateformes": [{"nom": "MS-DOS", "nombre": 812}]})), attendu);
        assert_eq!(lire_plateformes(&json!({"items": [{"name": "MS-DOS", "total": 812}]})), attendu);
        assert!(lire_plateformes(&json!({"rien": 1})).is_empty());
    }

    #[test]
    fn lit_une_page_de_catalogue_et_sait_s_il_en_reste() {
        let v = json!({"page": 1, "pages": 3, "jeux": [{"id": 110, "titre": "Dune", "plateforme": "MS-DOS"}]});
        let p = lire_page(&v, 1);
        assert_eq!(p.jeux.len(), 1);
        assert_eq!(p.jeux[0].titre, "Dune");
        assert_eq!(p.encore, Some(true));
        assert_eq!(lire_page(&v, 3).encore, Some(false));
        assert_eq!(lire_page(&json!([{"id": 1, "titre": "X"}]), 1).encore, None);
        assert_eq!(lire_page(&json!({"jeux": [], "suivante": null}), 1).encore, Some(false));
    }

    /// Le bloc JSON qui suit un titre de docs/EXEMPLES-API-JEUX-V1.md (réponses RÉELLES de Firehouse).
    pub(crate) fn exemple_reel(titre: &str) -> Value {
        let doc = include_str!("../../docs/EXEMPLES-API-JEUX-V1.md");
        let apres = &doc[doc.find(titre).unwrap_or_else(|| panic!("exemple « {titre} » absent"))..];
        let debut = apres.find("```json").unwrap() + 7;
        let fin = debut + apres[debut..].find("```").unwrap();
        serde_json::from_str(&apres[debut..fin]).unwrap()
    }

    #[test]
    fn lit_les_vraies_reponses_de_firehouse() {
        let p = lire_plateformes(&exemple_reel("## `GET /plateformes`"));
        assert_eq!(p, vec![Plateforme { nom: "MS-DOS".into(), jeux: 1 }, Plateforme { nom: "Nintendo 64".into(), jeux: 1 }]);

        let page = lire_page(&exemple_reel("## `GET /catalogue"), 1);
        assert_eq!(page.encore, Some(false)); // "suivante": null
        assert_eq!(page.jeux.len(), 2);
        let dune = &page.jeux[0];
        assert_eq!((dune.id, dune.titre.as_str(), dune.annee), (110, "Dune", Some(1992)));
        assert_eq!(dune.genres, vec!["Aventure", "Stratégie"]);
        assert_eq!((dune.jaquette, dune.versions), (Some(true), Some(1)));
        assert_eq!(page.jeux[1].statut.as_deref(), Some("recherche"));
    }

    #[test]
    fn une_page_suivante_numerotee_veut_dire_qu_il_en_reste() {
        assert_eq!(lire_page(&json!({"suivante": 2, "jeux": [{"id": 1, "titre": "A"}]}), 1).encore, Some(true));
    }

    #[test]
    fn un_jeu_mal_forme_est_ignore_sans_casser_la_page() {
        let v = json!({"jeux": [{"id": 1, "titre": "Bon"}, {"titre": "sans id"}, "n'importe quoi"]});
        assert_eq!(lire_page(&v, 1).jeux.len(), 1);
    }

    #[test]
    fn filtre_par_plateforme_texte_sans_accents_et_genre() {
        let c = cache_exemple();
        let tous = c.lister(&Filtre::default()).unwrap();
        assert_eq!((tous.total, tous.total_ludotheque), (3, 3));
        assert_eq!(c.lister(&Filtre { plateforme: Some("MS-DOS".into()), ..Default::default() }).unwrap().total, 2);
        let echo = c.lister(&Filtre { texte: Some("echo".into()), ..Default::default() }).unwrap();
        assert_eq!(echo.jeux[0].titre, "Écho du passé");
        let strat = c.lister(&Filtre { genre: Some("Stratégie".into()), ..Default::default() }).unwrap();
        assert_eq!(strat.jeux.iter().map(|j| j.id).collect::<Vec<_>>(), vec![110]);
    }

    #[test]
    fn trie_par_titre_ou_par_annee() {
        let c = cache_exemple();
        let titres = |tri: Option<&str>| {
            c.lister(&Filtre { tri: tri.map(String::from), ..Default::default() })
                .unwrap()
                .jeux
                .into_iter()
                .map(|j| j.id)
                .collect::<Vec<_>>()
        };
        assert_eq!(titres(None), vec![110, 111, 200]);
        assert_eq!(titres(Some("annee")), vec![110, 200, 111]);
        assert_eq!(titres(Some("annee_desc")), vec![111, 200, 110]);
    }

    #[test]
    fn remplacer_retire_les_jeux_devenus_invisibles_et_leurs_fiches() {
        let mut c = cache_exemple();
        c.garder_fiche(200, &json!({"id": 200}), "t").unwrap();
        c.remplacer(&[], &[jeu(110, "Dune", "MS-DOS", None, &[])], "t2").unwrap();
        assert!(!c.contient(200).unwrap());
        assert_eq!(c.fiche(200).unwrap(), None);
        assert_eq!(c.synchronise_le().unwrap().as_deref(), Some("t2"));
    }

    #[test]
    fn les_plateformes_comptent_les_jeux_du_cache() {
        let c = cache_exemple();
        let p = c.plateformes().unwrap();
        assert_eq!(p.iter().map(|p| (p.nom.as_str(), p.jeux)).collect::<Vec<_>>(), vec![("MS-DOS", 2), ("Super Nintendo", 1)]);
        assert_eq!(c.genres(Some("MS-DOS")).unwrap(), vec!["Aventure", "Stratégie"]);
    }

    #[test]
    fn un_jeu_au_hasard_reste_dans_la_plateforme() {
        let c = cache_exemple();
        for _ in 0..10 {
            assert_eq!(c.au_hasard(Some("Super Nintendo")).unwrap().unwrap().id, 200);
        }
    }

    #[test]
    fn le_cache_survit_a_la_fermeture() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("seb").join("ludotheque.db");
        {
            let mut c = Cache::ouvrir(&f).unwrap();
            c.remplacer(&[], &[jeu(1, "A", "PC", None, &[])], "t").unwrap();
        }
        assert_eq!(Cache::ouvrir(&f).unwrap().lister(&Filtre::default()).unwrap().total, 1);
    }
}
