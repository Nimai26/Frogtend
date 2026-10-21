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

/// Un disque FIXE de ce PC (jamais une clé USB, un lecteur optique ni un partage réseau).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DisqueLocal {
    /// `C:\`
    pub racine: String,
    pub libre: u64,
    /// Le disque de Windows.
    pub systeme: bool,
}

/// Les disques fixes de ce PC, avec leur place libre (lus sur CE PC, jamais supposés : pas de « D: » ni de « E: »).
#[cfg(windows)]
pub fn disques_locaux() -> Vec<DisqueLocal> {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};
    const DRIVE_FIXED: u32 = 3;
    let systeme = std::env::var("SystemDrive").unwrap_or_default().trim_end_matches('\\').to_uppercase();
    let masque = unsafe { GetLogicalDrives() };
    (0..26u8)
        .filter(|i| masque & (1 << i) != 0)
        .filter_map(|i| {
            let racine = format!("{}:\\", (b'A' + i) as char);
            let large: Vec<u16> = racine.encode_utf16().chain([0]).collect();
            if unsafe { GetDriveTypeW(PCWSTR(large.as_ptr())) } != DRIVE_FIXED {
                return None;
            }
            let libre = fs2::available_space(Path::new(&racine)).ok()?;
            Some(DisqueLocal { systeme: racine.trim_end_matches('\\').eq_ignore_ascii_case(&systeme), racine, libre })
        })
        .collect()
}

#[cfg(not(windows))]
pub fn disques_locaux() -> Vec<DisqueLocal> {
    vec![]
}

/// Le disque proposé pour ranger les jeux quand rien n'est réglé : celui qui a le plus de place (à place égale, pas
/// celui de Windows).
pub fn disque_propose(disques: &[DisqueLocal]) -> Option<&DisqueLocal> {
    disques.iter().max_by(|a, b| a.libre.cmp(&b.libre).then(b.systeme.cmp(&a.systeme)))
}

/// Le dossier des jeux proposé sur un disque : `<disque>\Frogtend\Jeux` (tout ce que Frogtend range est sous
/// `Frogtend`, jamais mêlé à des dossiers existants).
pub fn dossier_jeux_sur(d: &DisqueLocal) -> PathBuf {
    Path::new(&d.racine).join("Frogtend").join("Jeux")
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
    let vu = |c: &String| {
        let libre = place_libre(Path::new(c));
        EmplacementPropose { chemin: c.clone(), libre, assez: libre.is_some_and(|l| l >= taille + MARGE_LIBRE) }
    };
    let mut l: Vec<EmplacementPropose> = emplacements.pour(systeme).iter().map(vu).collect();
    // Les dossiers propres à ce système n'existent plus (PC formaté, disque changé) : ceux par défaut ensuite, plutôt
    // que rien.
    if l.iter().all(|e| e.libre.is_none()) {
        for c in &emplacements.defaut {
            if !l.iter().any(|e| e.chemin.eq_ignore_ascii_case(c)) {
                l.push(vu(c));
            }
        }
    }
    l
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

/// Le chemin d'un fichier d'une version, RELATIF au dossier du jeu : un nom, ou « sous-dossier/nom » (contrat 1.13 :
/// une CHD MAME vit dans le dossier de son parent, `<rompath>/<dossier>/`). Chaque partie est contrôlée comme un nom.
pub fn chemin_de_fichier_sur(chemin: &str) -> Resultat<std::path::PathBuf> {
    let parties: Vec<&str> = chemin.split('/').collect();
    if parties.is_empty() || parties.len() > 2 {
        return Err(Erreur::Refus(format!("Firehouse a envoyé un chemin de fichier inutilisable : « {chemin} ».")));
    }
    let mut p = std::path::PathBuf::new();
    for x in parties {
        p.push(nom_de_fichier_sur(x)?);
    }
    Ok(p)
}

#[cfg(test)]
#[test]
fn un_chemin_de_fichier_reste_dans_le_dossier_du_jeu() {
    assert_eq!(chemin_de_fichier_sur("sfiii.zip").unwrap(), std::path::PathBuf::from("sfiii.zip"));
    assert_eq!(chemin_de_fichier_sur("sfiii/cap-sf3-3.chd").unwrap(), std::path::Path::new("sfiii").join("cap-sf3-3.chd"));
    for piege in ["../x", "a/../b", "a/b/c", "C:/x", "a\\b", "/x", "a/"] {
        assert!(chemin_de_fichier_sur(piege).is_err(), "{piege}");
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

/// Ce qui a été fait pour installer un jeu, et ce qu'on lance pour y jouer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Installation {
    /// Le dossier où le jeu est installé (ou celui des fichiers reçus, s'il n'y avait rien à installer).
    pub dossier: String,
    /// Comment il a été installé (installeur, archive, rien…).
    pub methode: crate::installation::Methode,
    /// Ce qu'on lance pour jouer ; `None` tant que la personne ne l'a pas choisi.
    pub lanceur: Option<crate::installation::Lanceur>,
    /// Pour une ROM ou une image disque : le fichier donné à l'émulateur (jamais renommé).
    #[serde(default)]
    pub fichier_du_jeu: Option<String>,
    pub installe_le: String,
    /// Les versions d'un jeu importé (régions, révisions), la préférée d'abord (fr, eu, us/en, autres — Seb, 02/10).
    /// Celle qu'on lance par défaut est `dossier` + `fichier_du_jeu`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub versions: Vec<VersionLocale>,
    /// Un jeu d'une BOUTIQUE (Steam…) installé par elle : Frogtend le LANCE par la boutique et suit la partie, mais
    /// n'écrit, ne retire et ne désinstalle JAMAIS rien dans son dossier (lot A des jeux de boutique, 20/10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boutique: Option<LancementBoutique>,
    /// Les fichiers que Frogtend a DÉCOMPRESSÉS à côté des fichiers reçus (disques d'un multi-disques zippé, contrat
    /// 1.23 `archive_a_extraire`), relatifs au dossier du jeu : le retrait les efface avec les fichiers reçus.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extraits: Vec<String>,
}

/// Comment une boutique lance un de ses jeux.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LancementBoutique {
    /// `steam` (l'adresse `steam://rungameid/<cle>`).
    pub source: String,
    /// L'identifiant du jeu dans la boutique (Steam : l'appid).
    pub cle: String,
}

/// Une version d'un jeu importé : un fichier (ou le 1er disque d'un jeu sur plusieurs disques).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VersionLocale {
    /// Chemin complet du fichier lancé (tel quel, jamais renommé).
    pub chemin: String,
    /// Ses étiquettes, lisibles (« France », « USA, Rev 1 »…).
    pub libelle: String,
    /// 0 fr, 1 eu, 2 us/en, 3 autres.
    pub rang: u8,
    /// Plus petit = meilleur ([!] vérifié, puis normal, puis alternatif, hack, bêta, mauvais dump).
    pub qualite: i32,
    /// Tous ses disques, dans l'ordre (vide : un seul fichier).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disques: Vec<String>,
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
    /// `None` tant que le jeu n'est pas installé.
    pub installation: Option<Installation>,
    /// Temps de jeu cumulé, en secondes.
    pub temps_jeu: u64,
    pub derniere_partie: Option<String>,
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
        // Contrat 1.14 : les jeux du PC rattachés à une fiche réunie (ancien → nouveau), pour ne rien perdre de vue
        // (abris de parties rangés sous l'ancien numéro, réglages de l'interface à faire suivre).
        db.execute_batch("CREATE TABLE IF NOT EXISTS reunions (ancien INTEGER PRIMARY KEY, nouveau INTEGER NOT NULL, le TEXT, applique_le TEXT NOT NULL);")?;
        // Contrat 1.21 (✅ Seb 20/10) : un jeu IMPORTÉ reconnu chez Firehouse. Une CORRESPONDANCE à côté : la ligne du jeu
        // importé ne change jamais (numéro, fichiers, temps de jeu) ; défaire = retirer ces lignes. Une ligne par
        // version (le fichier lancé) ; `telechargement_id` peut manquer.
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS correspondances (local INTEGER NOT NULL, chemin TEXT NOT NULL, media_id INTEGER NOT NULL,
               telechargement_id INTEGER, statut TEXT NOT NULL, correspondance TEXT, confirme TEXT NOT NULL, le TEXT NOT NULL,
               PRIMARY KEY (local, chemin));
             CREATE TABLE IF NOT EXISTS ecartes (local INTEGER NOT NULL, media_id INTEGER NOT NULL, le TEXT NOT NULL, PRIMARY KEY (local, media_id));",
        )?;
        // Colonnes ajoutées par le lot 3 (un registre de la 0.4.0 est complété, rien n'est perdu).
        for colonne in ["installation TEXT", "temps_jeu INTEGER NOT NULL DEFAULT 0", "derniere_partie TEXT"] {
            let nom = colonne.split(' ').next().unwrap();
            let existe: bool = db
                .prepare("SELECT 1 FROM pragma_table_info('jeux_pc') WHERE name = ?1")?
                .exists([nom])?;
            if !existe {
                db.execute(&format!("ALTER TABLE jeux_pc ADD COLUMN {colonne}"), [])?;
            }
        }
        Ok(Registre { db, fichier: fichier.into() })
    }

    /// Rattache un jeu du PC à sa nouvelle fiche (contrat 1.14) : SEUL changement du registre, en une transaction —
    /// sa ligne garde tout (version, dossier, installation, temps de jeu), seul son numéro change. Refusé (faux) si
    /// le nouveau numéro est déjà sur le PC (deux installations du même jeu : la personne décide).
    pub fn rattacher(&mut self, ancien: i64, nouveau: i64, le: Option<&str>, quand: &str) -> Resultat<bool> {
        let tx = self.db.transaction()?;
        let n = tx.execute(
            "UPDATE jeux_pc SET id = ?2 WHERE id = ?1 AND NOT EXISTS (SELECT 1 FROM jeux_pc WHERE id = ?2)",
            params![ancien, nouveau],
        )?;
        if n == 1 {
            tx.execute(
                "INSERT OR REPLACE INTO reunions (ancien, nouveau, le, applique_le) VALUES (?1, ?2, ?3, ?4)",
                params![ancien, nouveau, le, quand],
            )?;
        }
        tx.commit()?;
        Ok(n == 1)
    }

    /// Rattache les versions d'un jeu importé à une fiche de Firehouse (remplace ce qui était noté pour ce jeu).
    /// `confirme` : « auto » (tout exact) ou « personne ».
    pub fn noter_correspondance(&mut self, local: i64, versions: &[(String, i64, Option<i64>, String, Option<String>)], confirme: &str, quand: &str) -> Resultat<()> {
        let tx = self.db.transaction()?;
        tx.execute("DELETE FROM correspondances WHERE local = ?1", params![local])?;
        for (chemin, media_id, telechargement_id, statut, correspondance) in versions {
            tx.execute(
                "INSERT INTO correspondances VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![local, chemin, media_id, telechargement_id, statut, correspondance, confirme, quand],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Défait le rattachement d'un jeu importé (rien d'autre ne change).
    pub fn oublier_correspondance(&self, local: i64) -> Resultat<()> {
        self.db.execute("DELETE FROM correspondances WHERE local = ?1", params![local])?;
        Ok(())
    }

    /// La personne dit qu'un jeu importé N'EST PAS cette fiche : elle ne lui est plus jamais proposée (ni rattachée
    /// seule).
    pub fn ecarter(&self, local: i64, media_id: i64, quand: &str) -> Resultat<()> {
        self.db.execute("INSERT OR REPLACE INTO ecartes VALUES (?1, ?2, ?3)", params![local, media_id, quand])?;
        self.db.execute("DELETE FROM correspondances WHERE local = ?1 AND media_id = ?2", params![local, media_id])?;
        Ok(())
    }

    /// La personne revient sur « ce n'est pas lui » : les fiches écartées de ce jeu lui sont de nouveau proposées.
    pub fn reprendre_ecartes(&self, local: i64) -> Resultat<()> {
        self.db.execute("DELETE FROM ecartes WHERE local = ?1", params![local])?;
        Ok(())
    }

    /// Les fiches écartées : (local, fiche).
    pub fn ecartes(&self) -> Resultat<Vec<(i64, i64)>> {
        let mut st = self.db.prepare("SELECT local, media_id FROM ecartes")?;
        let l = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<_, _>>()?;
        Ok(l)
    }

    /// Les jeux importés rattachés : (local, fiche de Firehouse, comment).
    pub fn correspondances(&self) -> Resultat<Vec<(i64, i64, String)>> {
        // Seulement les jeux importés ENCORE sur ce PC : un lien gardé après un retrait (✅ Seb 20/10, il revient au
        // réimport) ne compte pas tant que le jeu est absent (le téléchargement depuis Firehouse reste permis).
        let mut st = self.db.prepare(
            "SELECT c.local, MIN(c.media_id), MIN(c.confirme) FROM correspondances c JOIN jeux_pc j ON j.id = c.local GROUP BY c.local ORDER BY MIN(j.ajoute_le), c.local",
        )?;
        let l = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?.collect::<Result<_, _>>()?;
        Ok(l)
    }

    /// Les rattachements faits sur ce PC : (ancien, nouveau).
    /// (ancien, numéro ACTUEL) : une chaîne A→B puis B→C donne A→C et B→C (gardée contre les boucles).
    pub fn reunions(&self) -> Resultat<Vec<(i64, i64)>> {
        // Le rattachement le plus récent d'abord : ses réglages priment sur une chaîne (copie « le premier gagne »).
        let mut st = self.db.prepare("SELECT ancien, nouveau FROM reunions ORDER BY applique_le DESC, ancien")?;
        let l: Vec<(i64, i64)> = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<_, _>>()?;
        let carte: std::collections::HashMap<i64, i64> = l.iter().copied().collect();
        Ok(l
            .into_iter()
            .map(|(a, mut n)| {
                for _ in 0..16 {
                    match carte.get(&n) {
                        Some(&s) if s != a => n = s,
                        _ => break,
                    }
                }
                (a, n)
            })
            .collect())
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
            installation: r.get::<_, Option<String>>(11)?.and_then(|t| serde_json::from_str(&t).ok()),
            temps_jeu: r.get::<_, i64>(12)? as u64,
            derniere_partie: r.get(13)?,
        })
    }

    const COLONNES: &'static str = "id, version, titre, plateforme, dossier, etat, total, fichiers, message, ajoute_le, \
         ajoute_par, installation, temps_jeu, derniere_partie";

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

    /// Retient (ou oublie, avec `None`) l'installation d'un jeu.
    pub fn changer_installation(&self, id: i64, i: Option<&Installation>) -> Resultat<()> {
        let texte = i.map(|i| serde_json::to_string(i).unwrap());
        self.db.execute("UPDATE jeux_pc SET installation = ?2 WHERE id = ?1", params![id, texte])?;
        Ok(())
    }

    /// Ajoute une partie au temps de jeu.
    pub fn compter_partie(&self, id: i64, secondes: u64, quand: &str) -> Resultat<()> {
        let n = self.db.execute(
            "UPDATE jeux_pc SET temps_jeu = temps_jeu + ?2, derniere_partie = ?3 WHERE id = ?1",
            params![id, secondes as i64, quand],
        )?;
        // Une partie lancée sous l'ancien numéro d'un jeu rattaché entre-temps (contrat 1.14) : comptée au nouveau.
        if n == 0 {
            if let Some((_, nouveau)) = self.reunions()?.into_iter().find(|(a, _)| *a == id) {
                self.db.execute(
                    "UPDATE jeux_pc SET temps_jeu = temps_jeu + ?2, derniere_partie = ?3 WHERE id = ?1",
                    params![nouveau, secondes as i64, quand],
                )?;
            }
        }
        Ok(())
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

    #[test]
    fn le_dossier_des_jeux_propose_va_sur_le_disque_le_plus_libre_sous_frogtend() {
        let d = |r: &str, libre: u64, systeme: bool| DisqueLocal { racine: r.into(), libre, systeme };
        assert_eq!(disque_propose(&[]), None, "aucun disque : rien d'inventé");
        let l = [d("C:\\", 50, true), d("F:\\", 900, false), d("G:\\", 10, false)];
        assert_eq!(disque_propose(&l).unwrap().racine, "F:\\");
        // À place égale : pas le disque de Windows.
        let l = [d("C:\\", 500, true), d("H:\\", 500, false)];
        assert_eq!(disque_propose(&l).unwrap().racine, "H:\\");
        // Un seul disque (un portable) : celui de Windows, bien sûr.
        assert_eq!(dossier_jeux_sur(&d("C:\\", 80, true)), PathBuf::from("C:\\Frogtend\\Jeux"));
        // Un dossier de système disparu : ceux par défaut ensuite (le dossier choisi sur le PC neuf).
        let t = tempfile::tempdir().unwrap();
        let e = Emplacements {
            defaut: vec![t.path().to_string_lossy().into()],
            systemes: [("Sony Playstation".to_string(), vec!["Q:\\Ancien PC\\PS1".to_string()])].into(),
        };
        let p = proposer(&e, "Sony Playstation", 1);
        assert_eq!(p.len(), 2);
        assert!(p[0].libre.is_none() && p[1].assez, "{p:?}");
        // Les vrais disques de ce PC : tous fixes, avec une racine « X:\ ».
        for x in disques_locaux() {
            assert!(x.racine.len() == 3 && x.racine.ends_with(":\\"), "{x:?}");
        }
    }

    #[test]
    fn une_correspondance_se_note_se_remplace_et_se_defait_sans_toucher_au_jeu() {
        let d = tempfile::tempdir().unwrap();
        let mut r = Registre::ouvrir(&d.path().join("jeux.db")).unwrap();
        r.ajouter(&jeu(-5, d.path())).unwrap();
        let avant = r.jeu(-5).unwrap();
        r.noter_correspondance(-5, &[("E:/a.nes".into(), 120, Some(-1), "exact".into(), Some("sha1".into()))], "auto", "1").unwrap();
        assert_eq!(r.correspondances().unwrap(), [(-5, 120, "auto".to_string())]);
        r.noter_correspondance(-5, &[("E:/a.nes".into(), 121, None, "probable".into(), None)], "personne", "2").unwrap();
        assert_eq!(r.correspondances().unwrap(), [(-5, 121, "personne".to_string())], "remplacée, une seule");
        r.oublier_correspondance(-5).unwrap();
        assert!(r.correspondances().unwrap().is_empty());
        r.noter_correspondance(-5, &[("E:/a.nes".into(), 120, None, "exact".into(), None)], "auto", "3").unwrap();
        r.ecarter(-5, 120, "4").unwrap();
        assert!(r.correspondances().unwrap().is_empty(), "écarter défait le rattachement à cette fiche");
        assert_eq!(r.ecartes().unwrap(), [(-5, 120)]);
        r.reprendre_ecartes(-5).unwrap();
        assert!(r.ecartes().unwrap().is_empty());
        assert_eq!(r.jeu(-5).unwrap(), avant, "le jeu importé n'a jamais changé");
    }

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
            installation: None,
            temps_jeu: 0,
            derniere_partie: None,
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
    fn un_jeu_rattache_deux_fois_suit_la_chaine_et_garde_son_temps_de_jeu() {
        let d = tempfile::tempdir().unwrap();
        let mut r = Registre::ouvrir(&d.path().join("jeux.db")).unwrap();
        r.ajouter(&jeu(1, d.path())).unwrap();
        assert!(r.rattacher(1, 2, None, "a").unwrap());
        assert!(r.rattacher(2, 3, None, "b").unwrap());
        // A→B puis B→C : les deux anciens mènent au numéro actuel.
        assert_eq!(r.reunions().unwrap(), vec![(2, 3), (1, 3)], "le plus récent d'abord");
        // Une partie lancée sous l'ancien numéro, finie après le rattachement : comptée au nouveau.
        let avant = r.jeu(3).unwrap().unwrap().temps_jeu;
        r.compter_partie(1, 60, "c").unwrap();
        assert_eq!(r.jeu(3).unwrap().unwrap().temps_jeu, avant + 60);
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
    fn l_installation_et_le_temps_de_jeu_sont_retenus() {
        let d = tempfile::tempdir().unwrap();
        let r = Registre::ouvrir(&d.path().join("jeux.db")).unwrap();
        r.ajouter(&jeu(110, d.path())).unwrap();
        let i = Installation {
            dossier: "E:\\Jeux\\Dune\\Jeu".into(),
            methode: crate::installation::Methode::Aucune,
            lanceur: None,
            fichier_du_jeu: None,
            installe_le: "1".into(),
            versions: vec![],
            boutique: None,
            extraits: vec![],
        };
        r.changer_installation(110, Some(&i)).unwrap();
        r.compter_partie(110, 600, "2").unwrap();
        r.compter_partie(110, 30, "3").unwrap();
        let j = r.jeu(110).unwrap().unwrap();
        assert_eq!(j.installation, Some(i));
        assert_eq!((j.temps_jeu, j.derniere_partie.as_deref()), (630, Some("3")));
    }

    #[test]
    fn un_registre_de_la_0_4_0_est_complete_sans_rien_perdre() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("jeux.db");
        {
            let db = Connection::open(&f).unwrap();
            db.execute_batch(
                "CREATE TABLE jeux_pc (id INTEGER PRIMARY KEY, version INTEGER NOT NULL, titre TEXT NOT NULL,
                 plateforme TEXT NOT NULL, dossier TEXT NOT NULL, etat TEXT NOT NULL, total INTEGER NOT NULL,
                 fichiers TEXT NOT NULL, message TEXT, ajoute_le TEXT NOT NULL, ajoute_par TEXT NOT NULL);
                 INSERT INTO jeux_pc VALUES (110, 200, 'Dune', 'MS-DOS', 'E:', 'telecharge', 9, '[]', NULL, '1', 'seb');",
            )
            .unwrap();
        }
        let r = Registre::ouvrir(&f).unwrap();
        let j = r.jeu(110).unwrap().unwrap();
        assert_eq!((j.titre.as_str(), j.etat, j.temps_jeu), ("Dune", Etat::Telecharge, 0));
        assert_eq!(j.installation, None);
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
