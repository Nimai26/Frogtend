//! Les cartes e-Reader (contrat 1.30). Une carte n'est PAS une ROM : elle se lit DANS la cartouche e-Reader (un jeu
//! Game Boy Advance de la ludothèque : « e-Reader (USA) », « Card e-Reader (Japan) »), par mGBA ≥ 0.10 :
//! `mGBA.exe --ecard <bande 1>.raw --ecard <bande 2>.raw <cartouche>` (« Scan an e-Reader card in the first loaded
//! game », répétable : ConfigController.cpp de mGBA 0.10.0 et 0.10.5). Frogtend ne lance jamais une carte seule.
//!
//! Une version = une carte : un .7z de quelques Ko qui contient ses bandes (`… (Strip 1).raw`, `… (Strip 2).raw`).
//! Elles sont décompressées une fois dans `<jeu>\Frogtend\cartes\<version>\` (règle « pas de pieuvre ») et parties
//! avec la carte ou le jeu.

use crate::erreurs::{Erreur, Resultat};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Le système des cartes, tel que Firehouse le nomme (LaunchBox).
pub const PLATEFORME: &str = "Nintendo e-Reader";
/// Ce que le profil garde du conseil de Firehouse, pour lancer une carte hors ligne.
pub const FICHIER_CARTOUCHES: &str = "cartouches-ereader.json";

/// Une cartouche e-Reader proposée par Firehouse (`conseil.cartouches` de `/emulateurs`, contrat 1.30).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Cartouche {
    pub media_id: i64,
    pub nom: String,
    pub region: String,
}

/// Les cartouches du conseil « Nintendo e-Reader » (les lignes incomplètes sont ignorées).
pub fn cartouches(reponse: &Value) -> Vec<Cartouche> {
    reponse["conseil"]["cartouches"]
        .as_array()
        .map(|l| l.iter().filter_map(|c| serde_json::from_value::<Cartouche>(c.clone()).ok()).filter(|c| c.media_id > 0).collect())
        .unwrap_or_default()
}

/// Une carte japonaise ? D'après les étiquettes de son nom officiel (No-Intro : « (Japan) », « (Japan) (Promo) »).
/// Les cartes japonaises ne se lisent que dans la cartouche japonaise, les autres (USA, Australie) dans l'américaine.
pub fn japonaise(nom: &str) -> bool {
    let etiquettes: Vec<String> = nom
        .split('(')
        .skip(1)
        .filter_map(|m| m.split(')').next())
        .flat_map(|m| m.split(',').map(|r| r.trim().to_ascii_lowercase()).collect::<Vec<_>>())
        .collect();
    etiquettes.iter().any(|r| r == "japan") && !etiquettes.iter().any(|r| r == "usa" || r == "australia")
}

/// La cartouche qui lit cette carte : la japonaise pour une carte japonaise, sinon l'américaine (à défaut, une autre
/// qui n'est pas japonaise).
pub fn cartouche_pour(cartouches: &[Cartouche], japonaise: bool) -> Option<&Cartouche> {
    let japon = |c: &&Cartouche| c.region.eq_ignore_ascii_case("japan");
    if japonaise {
        cartouches.iter().find(japon)
    } else {
        cartouches.iter().find(|c| c.region.eq_ignore_ascii_case("usa")).or_else(|| cartouches.iter().find(|c| !japon(c)))
    }
}

/// Le numéro de bande d'un nom (`… (Strip 12).raw` → 12) ; sans numéro, après les autres.
fn numero_de_bande(nom: &str) -> u32 {
    let bas = nom.to_ascii_lowercase();
    bas.rfind("strip")
        .map(|i| bas[i + 5..].trim_start().chars().take_while(|c| c.is_ascii_digit()).collect::<String>())
        .and_then(|n| n.parse().ok())
        .unwrap_or(u32::MAX)
}

/// Les bandes (`.raw`) d'un dossier, dans l'ordre de leurs NUMÉROS (« Strip 2 » avant « Strip 10 »).
pub fn bandes_du_dossier(dossier: &Path) -> Vec<PathBuf> {
    let mut l = Vec::new();
    let mut a_voir = vec![dossier.to_path_buf()];
    while let Some(d) = a_voir.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            match e.file_type() {
                Ok(t) if t.is_dir() => a_voir.push(p),
                Ok(t) if t.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("raw")) => l.push(p),
                _ => {}
            }
        }
    }
    let nom = |p: &PathBuf| p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    l.sort_by_key(|p| (numero_de_bande(&nom(p)), nom(p)));
    l
}

/// Le dossier des bandes d'une carte, dans ce que Frogtend ajoute au jeu.
pub fn dossier_des_bandes(frogtend_du_jeu: &Path, version: i64) -> PathBuf {
    frogtend_du_jeu.join("cartes").join(version.to_string())
}

/// Une carte fait quelques Ko : au-delà, ce n'est pas une carte (rien n'est décompressé).
pub const TAILLE_MAX_CARTE: u64 = 16 << 20;

/// Les bandes d'une carte, décompressées de son archive la première fois (puis reprises telles quelles). `jeu` : le
/// dossier du jeu, dans lequel `dossier` doit être, sans lien ni jonction entre les deux.
pub fn bandes(archive: &Path, jeu: &Path, dossier: &Path) -> Resultat<Vec<PathBuf>> {
    if !crate::garde::sous(dossier, jeu) || crate::garde::lien_entre(jeu, dossier) {
        return Err(Erreur::Refus("Le dossier des cartes n'est pas un dossier ordinaire du jeu : rien n'est décompressé.".into()));
    }
    let deja = bandes_du_dossier(dossier);
    if !deja.is_empty() {
        return Ok(deja);
    }
    if std::fs::metadata(archive).is_ok_and(|m| m.len() > TAILLE_MAX_CARTE) {
        return Err(Erreur::Prevenir("Ce fichier est bien trop gros pour une carte e-Reader : il n'est pas lu. Tes fichiers restent intacts.".into()));
    }
    // Un reste sans bande (décompression d'une autre forme) : ce dossier est à Frogtend seul, il est refait.
    if dossier.is_dir() {
        std::fs::remove_dir_all(dossier)?;
    }
    if let Some(p) = dossier.parent() {
        std::fs::create_dir_all(p)?;
    }
    crate::decompression::extraire_dans(archive, dossier, None)?;
    let l = bandes_du_dossier(dossier);
    if l.is_empty() {
        let _ = std::fs::remove_dir_all(dossier);
        return Err(Erreur::Prevenir(
            "Cette carte e-Reader ne contient aucune bande lisible (fichier « .raw ») : elle ne peut pas être lue. Tes fichiers restent intacts.".into(),
        ));
    }
    Ok(l)
}

/// Ajoute les bandes aux arguments DÉJÀ découpés de mGBA (jamais dans le texte de la ligne : un chemin n'y est pas
/// relu comme une variable), juste AVANT l'argument qui porte la cartouche — une option après le fichier pourrait être
/// ignorée. Sans cet argument (impossible), elles vont en tête.
pub fn inserer_bandes(arguments: &mut Vec<String>, cartouche: &Path, bandes: &[PathBuf]) {
    let c = cartouche.to_string_lossy().to_string();
    let i = arguments.iter().position(|a| a.contains(&c)).unwrap_or(0);
    let ajout: Vec<String> = bandes.iter().flat_map(|b| ["--ecard".to_string(), b.to_string_lossy().to_string()]).collect();
    arguments.splice(i..i, ajout);
}

/// Le programme est-il mGBA ? (seul émulateur qui lit les cartes, d'après le conseil de Firehouse).
pub fn est_mgba(programme: &str) -> bool {
    Path::new(programme).file_name().is_some_and(|n| n.to_string_lossy().to_ascii_lowercase().starts_with("mgba"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_cartouches_viennent_du_conseil_de_firehouse() {
        // Relevé RÉEL du 26/10 (lecture seule) : `/emulateurs?plateforme=Nintendo e-Reader`.
        let v = serde_json::json!({"conseil": {"emulateur": "mgba", "cartouches": [
            {"media_id": 3666, "nom": "Card e-Reader (Japan)", "region": "Japan"},
            {"media_id": 3681, "nom": "e-Reader (USA)", "region": "USA"},
            {"nom": "incomplète"}
        ]}});
        let c = cartouches(&v);
        assert_eq!(c.len(), 2);
        assert_eq!(cartouche_pour(&c, true).unwrap().media_id, 3666);
        assert_eq!(cartouche_pour(&c, false).unwrap().media_id, 3681);
        assert!(cartouches(&serde_json::json!({"conseil": null})).is_empty());
        // Une seule cartouche, japonaise : jamais pour une carte américaine.
        assert!(cartouche_pour(&c[..1], false).is_none());
    }

    #[test]
    fn la_region_d_une_carte_se_lit_dans_son_nom() {
        assert!(japonaise("Pokemon-e TCG - 01-A001 (Japan).7z"));
        assert!(japonaise("Carte (Japan) (Promo).7z"));
        assert!(!japonaise("Air Hockey-e - 00-B000 (USA) (Promo).7z"));
        assert!(!japonaise("Pokemon-e TCG - 001 (USA, Australia).7z"));
        assert!(!japonaise("Sans étiquette.7z"));
    }

    #[test]
    fn les_bandes_sont_rangees_par_numero() {
        let d = tempfile::tempdir().unwrap();
        for n in ["Jeu (Strip 10).raw", "Jeu (Strip 2).raw", "Jeu (Strip 1).raw", "lisez-moi.txt"] {
            std::fs::write(d.path().join(n), b"x").unwrap();
        }
        let noms: Vec<String> = bandes_du_dossier(d.path()).iter().map(|p| p.file_name().unwrap().to_string_lossy().to_string()).collect();
        assert_eq!(noms, ["Jeu (Strip 1).raw", "Jeu (Strip 2).raw", "Jeu (Strip 10).raw"]);
    }

    fn carte_7z(f: &Path, fichiers: &[(&str, &[u8])]) {
        let mut w = sevenz_rust::SevenZWriter::create(f).unwrap();
        for (nom, c) in fichiers {
            let mut e = sevenz_rust::SevenZArchiveEntry::new();
            e.name = nom.to_string();
            e.has_stream = true;
            w.push_archive_entry(e, Some(*c)).unwrap();
        }
        w.finish().unwrap();
    }

    #[test]
    fn une_carte_se_decompresse_une_fois_dans_le_dossier_du_jeu() {
        let d = tempfile::tempdir().unwrap();
        let archive = d.path().join("Air Hockey-e - 00-B000 (USA) (Promo).7z");
        carte_7z(&archive, &[("Air Hockey-e (Strip 2).raw", b"bande deux"), ("Air Hockey-e (Strip 1).raw", b"bande un")]);
        let dossier = dossier_des_bandes(&d.path().join("Frogtend"), 42);
        let l = bandes(&archive, d.path(), &dossier).unwrap();
        assert_eq!(l.len(), 2);
        assert!(l[0].ends_with("Air Hockey-e (Strip 1).raw"));
        // Vérifier le résultat : les octets sont là.
        assert_eq!(std::fs::read(&l[1]).unwrap(), b"bande deux");
        // La fois suivante : reprises telles quelles, sans rouvrir l'archive.
        std::fs::remove_file(&archive).unwrap();
        assert_eq!(bandes(&archive, d.path(), &dossier).unwrap(), l);
    }

    #[test]
    fn une_carte_sans_bande_est_refusee_sans_rien_laisser() {
        let d = tempfile::tempdir().unwrap();
        let archive = d.path().join("vide.7z");
        carte_7z(&archive, &[("lisez-moi.txt", b"rien")]);
        let dossier = dossier_des_bandes(&d.path().join("Frogtend"), 7);
        assert!(matches!(bandes(&archive, d.path(), &dossier), Err(Erreur::Prevenir(_))));
        assert!(!dossier.exists());
        assert!(archive.is_file(), "l'archive de la carte n'est jamais touchée");
    }

    #[test]
    fn une_carte_trop_grosse_ou_hors_du_jeu_n_est_pas_decompressee() {
        let d = tempfile::tempdir().unwrap();
        let archive = d.path().join("grosse.7z");
        std::fs::File::create(&archive).unwrap().set_len(TAILLE_MAX_CARTE + 1).unwrap();
        let dossier = dossier_des_bandes(&d.path().join("Frogtend"), 1);
        assert!(matches!(bandes(&archive, d.path(), &dossier), Err(Erreur::Prevenir(_))));
        assert!(!dossier.exists());
        let ailleurs = tempfile::tempdir().unwrap();
        assert!(matches!(bandes(&archive, ailleurs.path(), &dossier), Err(Erreur::Refus(_))));
    }

    #[test]
    fn les_bandes_vont_juste_avant_la_cartouche() {
        let b = [PathBuf::from("C:\\J\\100%A% (Strip 1).raw"), PathBuf::from("C:\\J\\a (Strip 2).raw")];
        let c = Path::new("C:\\G\\e-Reader (USA).7z");
        let mut args: Vec<String> = ["-C", "savegamePath=x", "-f", "C:\\G\\e-Reader (USA).7z"].map(String::from).to_vec();
        inserer_bandes(&mut args, c, &b);
        assert_eq!(args, ["-C", "savegamePath=x", "-f", "--ecard", "C:\\J\\100%A% (Strip 1).raw", "--ecard", "C:\\J\\a (Strip 2).raw", "C:\\G\\e-Reader (USA).7z"]);
        // Une ligne réglée à la main qui met des options APRÈS la cartouche : les bandes restent avant elle.
        let mut args: Vec<String> = ["C:\\G\\e-Reader (USA).7z", "-f"].map(String::from).to_vec();
        inserer_bandes(&mut args, c, &b[1..]);
        assert_eq!(args, ["--ecard", "C:\\J\\a (Strip 2).raw", "C:\\G\\e-Reader (USA).7z", "-f"]);
        assert!(est_mgba("C:\\Emu\\mGBA\\mGBA.exe"));
        assert!(!est_mgba("C:\\Emu\\RetroArch\\retroarch.exe"));
    }
}
