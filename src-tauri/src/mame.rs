//! 📥 Importer ▸ MAME Arcade Full Set (comme LaunchBox) : un dossier de ROM MAME (un .zip par jeu, souvent 14 000 et
//! plus) contient aussi des clones, des BIOS, des appareils et des jeux qui ne marchent pas. On les écarte avec la
//! **liste MAME de LaunchBox** (`Metadata/MAME.xml` : nom de fichier, nom, clone de, statut, drapeaux), lue en LECTURE
//! SEULE dans le fichier que la personne désigne, en attendant que Firehouse la serve (besoin API n° 15).
//!
//! Un zip absent de la liste n'est pas un jeu (BIOS comme `neogeo`, appareils) : il est écarté.

use crate::erreurs::{Erreur, Resultat};
use crate::import_local::RomTrouvee;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

/// Un jeu de la liste MAME.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct JeuMame {
    pub fichier: String,
    pub nom: String,
    pub clone_de: Option<String>,
    /// `good`, `imperfect` ou `preliminary` (ne marche pas).
    pub statut: String,
    pub annee: Option<i64>,
    pub editeur: Option<String>,
    pub genre: Option<String>,
    /// Les drapeaux à `true` (`IsBootleg`, `IsCasino`, `IsMature`…), sans le préfixe « Is ».
    pub drapeaux: Vec<String>,
}

/// La valeur d'une balise simple dans un bloc (`<Nom>valeur</Nom>`), entités XML de base décodées.
fn balise(bloc: &str, nom: &str) -> Option<String> {
    let ouvre = format!("<{nom}>");
    let debut = bloc.find(&ouvre)? + ouvre.len();
    let fin = bloc[debut..].find(&format!("</{nom}>"))? + debut;
    let v = bloc[debut..fin]
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&");
    let v = v.trim().to_string();
    (!v.is_empty()).then_some(v)
}

/// Lit la liste MAME de LaunchBox (`<LaunchBox><MameFile>…</MameFile>…`), par nom de fichier (en minuscules).
pub fn lire_liste(texte: &str) -> HashMap<String, JeuMame> {
    let mut l = HashMap::new();
    for bloc in texte.split("<MameFile>").skip(1) {
        let bloc = bloc.split("</MameFile>").next().unwrap_or(bloc);
        let Some(fichier) = balise(bloc, "FileName") else { continue };
        let mut drapeaux = Vec::new();
        let mut reste = bloc;
        while let Some(i) = reste.find("<Is") {
            let r = &reste[i + 3..];
            let Some(f) = r.find('>') else { break };
            let nom = &r[..f];
            if r[f + 1..].starts_with("true<") && nom.chars().all(|c| c.is_ascii_alphanumeric()) {
                drapeaux.push(nom.to_string());
            }
            reste = &r[f..];
        }
        l.insert(
            fichier.to_lowercase(),
            JeuMame {
                nom: balise(bloc, "Name").unwrap_or_else(|| fichier.clone()),
                clone_de: balise(bloc, "CloneOf"),
                statut: balise(bloc, "Status").unwrap_or_default(),
                annee: balise(bloc, "Year").and_then(|a| a.get(..4).and_then(|a| a.parse().ok())),
                editeur: balise(bloc, "Publisher"),
                genre: balise(bloc, "Genre"),
                drapeaux,
                fichier,
            },
        );
    }
    l
}

/// Ce qu'on GARDE (les cases de la fenêtre d'import). Par défaut, comme LaunchBox : les jeux d'arcade jouables,
/// sans clones ni variantes douteuses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OptionsMame {
    pub clones: bool,
    /// Statut « preliminary » : le jeu ne marche pas dans MAME.
    pub non_jouables: bool,
    pub imparfaits: bool,
    pub contrefacons: bool,
    pub prototypes: bool,
    pub hacks: bool,
    pub adultes: bool,
    /// Casino, machines à sous (fruit), mahjong.
    pub jeux_d_argent: bool,
    pub mecaniques: bool,
    pub quiz: bool,
    /// Ce qui n'est pas une borne d'arcade (ordinateurs, consoles, appareils).
    pub hors_arcade: bool,
}

impl Default for OptionsMame {
    fn default() -> Self {
        OptionsMame {
            clones: false,
            non_jouables: false,
            imparfaits: true,
            contrefacons: false,
            prototypes: false,
            hacks: false,
            adultes: false,
            jeux_d_argent: false,
            mecaniques: false,
            quiz: true,
            hors_arcade: false,
        }
    }
}

/// Pourquoi un jeu est écarté (le premier motif trouvé), ou `None` s'il est gardé.
pub fn motif_d_ecart(j: &JeuMame, o: &OptionsMame) -> Option<&'static str> {
    let a = |d: &str| j.drapeaux.iter().any(|x| x == d);
    if !o.hors_arcade && a("NonArcade") {
        return Some("hors arcade");
    }
    if !o.mecaniques && a("Mechanical") {
        return Some("mécanique");
    }
    if !o.jeux_d_argent && (a("Casino") || a("Fruit") || a("Mahjong")) {
        return Some("jeu d'argent ou mahjong");
    }
    if !o.adultes && a("Mature") {
        return Some("pour adultes");
    }
    if !o.non_jouables && j.statut.eq_ignore_ascii_case("preliminary") {
        return Some("ne marche pas dans MAME");
    }
    if !o.imparfaits && j.statut.eq_ignore_ascii_case("imperfect") {
        return Some("imparfait");
    }
    if !o.clones && j.clone_de.is_some() {
        return Some("clone");
    }
    if !o.contrefacons && a("Bootleg") {
        return Some("contrefaçon");
    }
    if !o.prototypes && a("Prototype") {
        return Some("prototype");
    }
    if !o.hacks && a("Hack") {
        return Some("hack");
    }
    if !o.quiz && a("Quiz") {
        return Some("quiz");
    }
    None
}

/// Un jeu MAME retenu.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct JeuMameRetenu {
    #[serde(flatten)]
    pub rom: RomTrouvee,
    pub annee: Option<i64>,
    pub editeur: Option<String>,
    pub genre: Option<String>,
}

/// Le tri d'un dossier MAME.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct TriMame {
    pub retenus: Vec<JeuMameRetenu>,
    /// Combien de zips écartés, par motif (dont « pas un jeu (BIOS, appareil) »).
    pub ecartes: BTreeMap<String, usize>,
}

/// Trie les .zip / .7z d'un dossier MAME d'après la liste.
pub fn trier(dossier: &Path, liste: &HashMap<String, JeuMame>, o: &OptionsMame) -> Resultat<TriMame> {
    if liste.is_empty() {
        return Err(Erreur::Refus("La liste MAME est vide ou illisible : choisis le fichier MAME.xml de LaunchBox.".into()));
    }
    let roms = crate::import_local::chercher_roms(dossier, &["zip".into(), "7z".into()], false)?;
    let mut t = TriMame::default();
    for r in roms {
        let fichier = Path::new(&r.chemin).file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
        let Some(j) = liste.get(&fichier) else {
            *t.ecartes.entry("pas un jeu (BIOS, appareil) ou inconnu".into()).or_default() += 1;
            continue;
        };
        if let Some(m) = motif_d_ecart(j, o) {
            *t.ecartes.entry(m.into()).or_default() += 1;
            continue;
        }
        t.retenus.push(JeuMameRetenu {
            rom: RomTrouvee { titre: j.nom.clone(), ..r },
            annee: j.annee,
            editeur: j.editeur.clone(),
            genre: j.genre.clone(),
        });
    }
    t.retenus.sort_by(|a, b| a.rom.titre.to_lowercase().cmp(&b.rom.titre.to_lowercase()));
    Ok(t)
}

/// Lit le fichier de la liste (jusqu'à 200 Mo) puis trie le dossier.
pub fn trier_avec_fichier(dossier: &Path, fichier_liste: &Path, o: &OptionsMame) -> Resultat<TriMame> {
    let m = std::fs::metadata(fichier_liste).map_err(|_| Erreur::Disque(format!("Liste MAME introuvable : {}.", fichier_liste.display())))?;
    if m.len() > 200 * 1024 * 1024 {
        return Err(Erreur::Refus("Ce fichier est trop gros pour être la liste MAME de LaunchBox.".into()));
    }
    let texte = std::fs::read_to_string(fichier_liste).map_err(|_| Erreur::Disque("La liste MAME n'est pas lisible (texte attendu).".into()))?;
    trier(dossier, &lire_liste(&texte), o)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTE: &str = r#"<?xml version="1.0" standalone="yes"?>
<LaunchBox>
  <MameFile><FileName>pacman</FileName><Name>Pac-Man (Midway)</Name><Status>good</Status><Publisher>Namco (Midway license)</Publisher><Year>1980</Year><IsBootleg>false</IsBootleg><IsCasino>false</IsCasino><Genre>Maze</Genre></MameFile>
  <MameFile><FileName>puckman</FileName><Name>Puck Man</Name><CloneOf>pacman</CloneOf><Status>good</Status><Year>1980</Year></MameFile>
  <MameFile><FileName>100lions</FileName><Name>100 Lions</Name><Status>preliminary</Status><IsCasino>true</IsCasino></MameFile>
  <MameFile><FileName>sf2</FileName><Name>Street Fighter II &amp; co</Name><Status>imperfect</Status><Year>1991?</Year></MameFile>
  <MameFile><FileName>boot1</FileName><Name>Contrefaçon</Name><Status>good</Status><IsBootleg>true</IsBootleg></MameFile>
</LaunchBox>"#;

    #[test]
    fn la_liste_de_launchbox_se_lit() {
        let l = lire_liste(LISTE);
        assert_eq!(l.len(), 5);
        let p = &l["pacman"];
        assert_eq!((p.nom.as_str(), p.annee, p.statut.as_str()), ("Pac-Man (Midway)", Some(1980), "good"));
        assert!(p.drapeaux.is_empty());
        assert_eq!(l["puckman"].clone_de.as_deref(), Some("pacman"));
        assert_eq!(l["100lions"].drapeaux, ["Casino"]);
        assert_eq!(l["sf2"].nom, "Street Fighter II & co");
        assert_eq!(l["sf2"].annee, Some(1991));
    }

    #[test]
    fn le_tri_ecarte_comme_launchbox() {
        let d = tempfile::tempdir().unwrap();
        for f in ["pacman.zip", "puckman.zip", "100lions.zip", "sf2.zip", "boot1.zip", "neogeo.zip", "notes.txt"] {
            std::fs::write(d.path().join(f), "x").unwrap();
        }
        let l = lire_liste(LISTE);
        let t = trier(d.path(), &l, &OptionsMame::default()).unwrap();
        let titres: Vec<&str> = t.retenus.iter().map(|j| j.rom.titre.as_str()).collect();
        assert_eq!(titres, ["Pac-Man (Midway)", "Street Fighter II & co"]);
        assert!(t.retenus[0].rom.chemin.ends_with("pacman.zip"), "le zip garde son nom");
        assert_eq!(t.ecartes["clone"], 1);
        assert_eq!(t.ecartes["contrefaçon"], 1);
        assert_eq!(t.ecartes["jeu d'argent ou mahjong"], 1);
        assert_eq!(t.ecartes["pas un jeu (BIOS, appareil) ou inconnu"], 1);

        let tout = OptionsMame { clones: true, contrefacons: true, ..Default::default() };
        assert_eq!(trier(d.path(), &l, &tout).unwrap().retenus.len(), 4);
        assert!(trier(d.path(), &HashMap::new(), &OptionsMame::default()).is_err());
    }
}

#[cfg(test)]
mod essais {
    /// Sur le vrai disque de Seb, en LECTURE SEULE : le tri de son dossier MAME avec sa liste LaunchBox.
    #[test]
    #[ignore]
    fn essai_tri_reel() {
        let debut = std::time::Instant::now();
        let t = super::trier_avec_fichier(
            std::path::Path::new("E:/Games/MAME"),
            std::path::Path::new("D:/LaunchBox/Metadata/MAME.xml"),
            &super::OptionsMame::default(),
        )
        .unwrap();
        println!("{} retenus en {:?}", t.retenus.len(), debut.elapsed());
        println!("{:?}", t.ecartes);
        for j in t.retenus.iter().take(5) {
            println!("{} ({:?}) <- {}", j.rom.titre, j.annee, j.rom.chemin);
        }
    }
}
