//! 📥 Importer ▸ MAME Arcade Full Set (comme LaunchBox) : un dossier de ROM MAME (un .zip par jeu, souvent 14 000 et
//! plus) contient aussi des clones, des BIOS, des appareils et des jeux qui ne marchent pas. On les écarte avec la
//! **liste MAME de LaunchBox** (`Metadata/MAME.xml` : nom de fichier, nom, clone de, statut, drapeaux), lue en LECTURE
//! SEULE dans le fichier que la personne désigne, en attendant que Firehouse la serve (besoin API n° 15).
//!
//! Un zip absent de la liste n'est pas un jeu (BIOS comme `neogeo`, appareils) : il est écarté.
//!
//! Contrat 1.33 (besoin 15) : la liste vient maintenant de Firehouse (`/mame/machines`, MAME 0.289, telle que CE jeton
//! la voit : une machine absente — dont les jeux pour adultes pour un non-admin — est écartée comme « inconnue », sans
//! jamais être comptée à part). Elle est gardée dans le dossier DU PROFIL (ETag, hors ligne) ; le MAME.xml de
//! LaunchBox ne sert plus qu'en secours (Firehouse injoignable et rien de gardé).

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
    /// D'où vient la liste : `firehouse` (MAME 0.289, contrat 1.33) ou `fichier` (le MAME.xml de LaunchBox, en secours).
    pub source: String,
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

/// La liste MAME de Firehouse (`/mame/machines`, contrat 1.33) au format du tri : par nom court (le zip). Un BIOS, un
/// appareil ou une machine qui ne se lance pas seule n'est pas un jeu : absent (écarté comme « pas un jeu »). Les
/// drapeaux de LaunchBox sont tirés des champs de Firehouse ; prototypes et hacks, de la description (aucun champ).
pub fn depuis_firehouse(v: &serde_json::Value) -> HashMap<String, JeuMame> {
    let mut l = HashMap::new();
    for m in v["machines"].as_array().into_iter().flatten() {
        let Some(nom) = m["nom"].as_str().filter(|n| !n.is_empty()) else { continue };
        if m["bios"] == true || m["device"] == true || m["executable"] == false {
            continue;
        }
        let texte = |c: &str| m[c].as_str().unwrap_or("").to_string();
        let (description, categorie) = (texte("description"), texte("categorie"));
        let (d, c) = (description.to_lowercase(), categorie.to_lowercase());
        let mut drapeaux = Vec::new();
        let mut si = |oui: bool, nom: &str| {
            if oui {
                drapeaux.push(nom.to_string());
            }
        };
        si(m["mecanique"] == true, "Mechanical");
        si(m["bootleg"] == true, "Bootleg");
        si(m["adulte"] == true, "Mature");
        si(c.contains("slot machine") || c.contains("gambling") || c.contains("casino") || c.contains("medal game"), "Casino");
        si(c.contains("mahjong"), "Mahjong");
        si(c.contains("quiz"), "Quiz");
        si(["computer", "game console", "handheld", "calculator", "telephone", "printer", "system /"].iter().any(|x| c.contains(x)), "NonArcade");
        si(d.contains("prototype"), "Prototype");
        si(d.contains("hack"), "Hack");
        let clone = texte("clone_de");
        l.insert(
            nom.to_lowercase(),
            JeuMame {
                fichier: nom.to_string(),
                nom: if description.is_empty() { nom.to_string() } else { description },
                clone_de: (!clone.is_empty()).then_some(clone),
                statut: texte("statut"),
                annee: m["annee"].as_str().and_then(|a| a.get(..4)).and_then(|a| a.parse().ok()),
                editeur: Some(texte("fabricant")).filter(|f| !f.is_empty()),
                genre: (!categorie.is_empty()).then_some(categorie),
                drapeaux,
            },
        );
    }
    l
}

impl crate::noyau::Noyau {
    /// La liste MAME de Firehouse pour ce profil (contrat 1.33) : relue avec l'ETag gardé (304 : la liste gardée),
    /// gardée dans le dossier du profil (jamais pour une session remplacée) ; Firehouse injoignable : celle gardée.
    /// `None` : aucune (mode simulé, ou jamais reçue et Firehouse injoignable).
    pub async fn liste_mame(&self) -> Resultat<Option<HashMap<String, JeuMame>>> {
        let s = self.session().await?;
        let (fichier, f_etag) = (s.dossier.join("mame-machines.json"), s.dossier.join("mame-machines.etag"));
        let gardee = || -> Option<HashMap<String, JeuMame>> {
            let o = std::fs::read(&fichier).ok()?;
            let v: serde_json::Value = serde_json::from_slice(&o).ok()?;
            Some(depuis_firehouse(&v)).filter(|l| !l.is_empty())
        };
        let etag = std::fs::read_to_string(&f_etag).ok().map(|e| e.trim().to_string()).filter(|e| !e.is_empty() && fichier.is_file());
        match s.source.mame_machines(etag.as_deref()).await {
            Ok(None) => Ok(gardee()),
            Ok(Some((octets, etag))) => {
                let v: serde_json::Value = serde_json::from_slice(&octets).map_err(|_| Erreur::Serveur("Firehouse a envoyé une liste MAME illisible.".into()))?;
                s.garder_fichier(&fichier, &octets)?;
                match etag {
                    Some(e) => {
                        s.garder_fichier(&f_etag, e.as_bytes())?;
                    }
                    None => {
                        let _ = std::fs::remove_file(&f_etag);
                    }
                }
                Ok(Some(depuis_firehouse(&v)).filter(|l| !l.is_empty()))
            }
            Err(Erreur::Reseau(_) | Erreur::Serveur(_) | Erreur::Introuvable(_)) => Ok(gardee()),
            Err(e) => Err(e),
        }
    }
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

    /// L'extrait RÉEL non-admin de `/mame/machines` fourni par Firehouse (27/10).
    fn liste_firehouse() -> serde_json::Value {
        serde_json::from_str(include_str!("../fixtures/simule/mame-machines-133.json")).unwrap()
    }

    #[test]
    fn la_liste_de_firehouse_se_lit_comme_celle_de_launchbox() {
        let l = depuis_firehouse(&liste_firehouse());
        assert!(!l.contains_key("neogeo") && !l.contains_key("z80"), "un BIOS, un appareil : pas des jeux");
        assert_eq!(l["pacman"].clone_de.as_deref(), Some("puckman"));
        assert_eq!(l["sf2"].nom, "Street Fighter II: The World Warrior (World 910522)");
        assert_eq!((l["sf2"].annee, l["sf2"].editeur.as_deref()), (Some(1991), Some("Capcom")));
        assert!(l["18w"].drapeaux.contains(&"Mechanical".to_string()));
        assert!(l["1942abl"].drapeaux.contains(&"Bootleg".to_string()));
        assert!(l["akissa"].drapeaux.contains(&"Mahjong".to_string()));
        assert!(l["100in1rg"].drapeaux.contains(&"NonArcade".to_string()), "une console de salon");
        // Un clone PUBLIC d'un parent adulte (absent) : un jeu ordinaire, sans « clone de ».
        assert_eq!(l["bigtwinb"].clone_de, None);
        // Les réglages d'office : sf2 gardé ; clone, mécanique, contrefaçon, mahjong, hors arcade écartés.
        let o = OptionsMame::default();
        assert_eq!(motif_d_ecart(&l["sf2"], &o), None);
        assert_eq!(motif_d_ecart(&l["pacman"], &o), Some("clone"));
        assert_eq!(motif_d_ecart(&l["18w"], &o), Some("mécanique"));
        assert_eq!(motif_d_ecart(&l["akissa"], &o), Some("jeu d'argent ou mahjong"));
        assert_eq!(motif_d_ecart(&l["100in1rg"], &o), Some("hors arcade"));
    }

    #[test]
    fn un_jeton_admin_voit_les_jeux_pour_adultes_et_la_case_les_garde_ou_non() {
        // Relevés RÉELS du jeton admin (27/10) : une machine adulte (le champ `adulte` n'est servi qu'aux admins), une
        // machine à sous, un quiz ; et des descriptions de prototype et de hack.
        let v = serde_json::json!({"machines": [
            {"nom": "3kokushi", "description": "Sankokushi (Japan)", "annee": "1996", "fabricant": "Mitchell", "clone_de": "", "bios": false, "device": false, "mecanique": false, "executable": true, "statut": "imperfect", "categorie": "Tabletop / Shanghai * Mature *", "bootleg": false, "adulte": true},
            {"nom": "100lions", "description": "100 Lions", "clone_de": "", "bios": false, "device": false, "executable": true, "statut": "good", "categorie": "Slot Machine / Video Slot"},
            {"nom": "6578cjz1", "description": "Quiz", "clone_de": "", "bios": false, "device": false, "executable": true, "statut": "good", "categorie": "Quiz / Questions in Chinese"},
            {"nom": "proto", "description": "Jeu (prototype)", "clone_de": "", "executable": true, "statut": "good", "categorie": "Shooter"},
            {"nom": "hack", "description": "Jeu (hack)", "clone_de": "", "executable": true, "statut": "good", "categorie": "Shooter"}
        ]});
        let l = depuis_firehouse(&v);
        let o = OptionsMame::default();
        assert_eq!(motif_d_ecart(&l["3kokushi"], &o), Some("pour adultes"));
        assert_eq!(motif_d_ecart(&l["3kokushi"], &OptionsMame { adultes: true, ..o.clone() }), None, "la case les garde");
        assert_eq!(motif_d_ecart(&l["100lions"], &o), Some("jeu d'argent ou mahjong"));
        assert_eq!(motif_d_ecart(&l["6578cjz1"], &OptionsMame { quiz: false, ..o.clone() }), Some("quiz"));
        assert_eq!(motif_d_ecart(&l["proto"], &o), Some("prototype"));
        assert_eq!(motif_d_ecart(&l["hack"], &o), Some("hack"));
    }

    #[test]
    fn un_zip_absent_de_la_liste_est_ecarte_sans_rien_dire_de_plus() {
        // Une machine qu'un jeton non-admin ne voit pas (adulte) : absente → « pas un jeu ou inconnu », jamais à part.
        let d = tempfile::tempdir().unwrap();
        for z in ["sf2.zip", "pacman.zip", "neogeo.zip", "18w.zip", "machine-cachee.zip"] {
            std::fs::write(d.path().join(z), b"PK").unwrap();
        }
        let t = trier(d.path(), &depuis_firehouse(&liste_firehouse()), &OptionsMame::default()).unwrap();
        assert_eq!(t.retenus.iter().map(|r| r.rom.titre.as_str()).collect::<Vec<_>>(), ["Street Fighter II: The World Warrior (World 910522)"]);
        assert_eq!(t.ecartes.get("pas un jeu (BIOS, appareil) ou inconnu"), Some(&2));
        assert_eq!(t.ecartes.len(), 3, "{:?}", t.ecartes);
    }

    #[tokio::test]
    async fn la_liste_est_gardee_dans_le_profil_et_relue_par_etag() {
        use httpmock::prelude::*;
        let s = MockServer::start();
        let corps = liste_firehouse().to_string();
        let premiere = s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/mame/machines").matches(|r| !r.headers.as_ref().is_some_and(|h| h.iter().any(|(k, _)| k.eq_ignore_ascii_case("if-none-match"))));
            t.status(200).header("etag", r#""v1""#).header("content-type", "application/json").body(corps.clone());
        });
        let relue = s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/mame/machines").header("if-none-match", r#""v1""#);
            t.status(304);
        });
        let d = tempfile::tempdir().unwrap();
        let n = crate::noyau::Noyau::nouveau(&d.path().join("app"), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &crate::noyau::Connexion { adresse: s.base_url(), simule: false }).await.unwrap();
        assert!(n.liste_mame().await.unwrap().unwrap().contains_key("sf2"));
        // La seconde fois : 304, la liste gardée sert.
        assert!(n.liste_mame().await.unwrap().unwrap().contains_key("sf2"));
        premiere.assert_hits(1);
        relue.assert_hits(1);
        assert!(n.session().await.unwrap().dossier.join("mame-machines.json").is_file(), "dans le dossier DU PROFIL");
    }

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
