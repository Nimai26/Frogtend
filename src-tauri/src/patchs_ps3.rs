//! Les patchs RPCS3 servis par Firehouse (contrat 1.18 ; ✅ Seb 19/10 : « par Firehouse »). Relevé dans les sources
//! de RPCS3 (`Utilities/bin_patch.cpp`, master du 19/10) :
//! - la liste officielle est `<RPCS3>\patches\patch.yml` (commence par `Version: 1.2`, rejetée en entier sinon) ;
//!   Frogtend la remplace seulement par celle de Firehouse, sha256 vérifié, l'ancienne copiée à l'abri ;
//!   `imported_patch.yml` et les `<numéro>_patch.yml` (ceux de la personne) ne sont JAMAIS touchés ;
//! - les patchs activés sont dans `<RPCS3>\config\patch_config.yml`, relu à CHAQUE démarrage de jeu :
//!   `hash → description → titre → numéro → version → {Enabled: true, Configurable Values: …}`. Frogtend n'y change
//!   QUE l'activation d'un patch demandé par la personne ; tout le reste (autres jeux, valeurs réglées) est recopié.
//!   Un fichier illisible n'est jamais réécrit (RPCS3 perdrait toutes les activations) ; l'écriture se fait par un
//!   fichier temporaire puis un renommage (jamais un fichier à moitié écrit).

use crate::erreurs::{Erreur, Resultat};
use serde_json::Value;
use std::path::{Path, PathBuf};
use yaml_rust2::{Yaml, YamlLoader};

/// Plafonds avant toute lecture (la liste officielle fait 1,2 Mo au 19/10).
pub const LISTE_MAX: u64 = 64 * 1024 * 1024;
const CONFIG_MAX: u64 = 8 * 1024 * 1024;

pub fn chemin_liste(rpcs3: &Path) -> PathBuf {
    rpcs3.join("patches").join("patch.yml")
}

pub fn chemin_config(rpcs3: &Path) -> PathBuf {
    rpcs3.join("config").join("patch_config.yml")
}

/// L'empreinte sha256 de la liste posée dans RPCS3 (`None` : absente, ou démesurée).
pub fn empreinte_liste(rpcs3: &Path) -> Option<String> {
    let f = chemin_liste(rpcs3);
    (std::fs::metadata(&f).ok()?.len() <= LISTE_MAX).then(|| crate::sauvegarde::empreinte(&f).ok()).flatten()
}

/// La version du moteur de patchs annoncée par la 1re ligne utile d'une liste (`Version: 1.2`).
pub fn version_moteur(debut: &str) -> Option<String> {
    let l = debut.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with('#'))?;
    let v = l.strip_prefix("Version:")?.trim().trim_matches(['"', '\'']);
    (!v.is_empty() && v.len() <= 8 && v.chars().all(|c| c.is_ascii_digit() || c == '.')).then(|| v.to_string())
}

/// Pose la liste téléchargée (déjà vérifiée : sha256 et taille) à la place de celle de RPCS3, si elle annonce bien la
/// version de moteur attendue ; l'ancienne est d'abord copiée à l'abri. Rien d'autre du dossier `patches` n'est touché.
pub fn poser_liste(rpcs3: &Path, telechargee: &Path, moteur: &str) -> Resultat<()> {
    let debut = {
        use std::io::Read;
        let mut b = vec![0u8; 4096];
        let n = std::fs::File::open(telechargee)?.read(&mut b)?;
        String::from_utf8_lossy(&b[..n]).to_string()
    };
    if version_moteur(&debut).as_deref() != Some(moteur) {
        let _ = std::fs::remove_file(telechargee);
        return Err(Erreur::Serveur("La liste de patchs reçue n'est pas celle attendue : rien n'est changé.".into()));
    }
    let cible = chemin_liste(rpcs3);
    crate::emulateurs_profils::mettre_a_l_abri(rpcs3, &cible)?;
    std::fs::rename(telechargee, &cible)?;
    Ok(())
}

/// Un reste de téléchargement de Frogtend dans `patches` : EXACTEMENT `patch-<16 hex>.yml.frogtend` ou
/// `patch-<16 hex>.yml.part` (jamais un fichier de la personne qui y ressemblerait).
pub fn reste_de_frogtend(nom: &str) -> bool {
    let Some(r) = nom.strip_prefix("patch-") else { return false };
    let (hex, fin) = r.split_at(r.len().min(16));
    hex.len() == 16 && hex.chars().all(|c| c.is_ascii_hexdigit()) && (fin == ".yml.frogtend" || fin == ".yml.part")
}

/// Remplace la liste des patchs de RPCS3 par celle de Firehouse (après l'accord de la personne) : décrite par `etat`
/// (`GET …/rpcs3/patchs`), téléchargée à côté sans jamais dépasser la taille annoncée, sha256 vérifié, version du
/// moteur relue, l'ancienne copiée à l'abri. Rend la taille reçue.
pub async fn installer_liste(source: &crate::source::Source, rpcs3: &Path, etat: &Value) -> Resultat<u64> {
    let sha = etat["sha256"].as_str().filter(|s| s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit()));
    let moteur = etat["version_moteur"].as_str().filter(|v| !v.is_empty() && v.len() <= 8 && v.chars().all(|c| c.is_ascii_digit() || c == '.'));
    let route = etat["fichier"].as_str().and_then(|f| f.strip_prefix(crate::firehouse::PREFIXE)).filter(|r| r.starts_with("/emulateurs/rpcs3/patchs/fichier"));
    let taille = etat["taille"].as_u64().filter(|t| *t > 0 && *t <= LISTE_MAX);
    let (Some(sha), Some(moteur), Some(route), Some(taille)) = (sha, moteur, route, taille) else {
        return Err(Erreur::Serveur("Firehouse a décrit la liste des patchs de façon inattendue : rien n'est changé.".into()));
    };
    let dossier = rpcs3.join("patches");
    // L'empreinte dans le nom : un reste d'une ANCIENNE liste n'est jamais complété avec la nouvelle (et il est retiré).
    let nom = format!("patch-{}.yml.frogtend", &sha[..16]);
    let part = format!("patch-{}.yml.part", &sha[..16]);
    if let Ok(l) = std::fs::read_dir(&dossier) {
        for e in l.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if reste_de_frogtend(&n) && n != nom && n != part && e.path().is_file() {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    let attente = dossier.join(&nom);
    let n = source.telecharger_plafonne(route, &attente, Some(sha), &[], Some(taille), &|_, _| {}).await.map_err(crate::source::erreur_patchs)?;
    if n != taille {
        let _ = std::fs::remove_file(&attente);
        return Err(Erreur::Serveur("La liste des patchs reçue n'a pas la taille annoncée : rien n'est changé.".into()));
    }
    poser_liste(rpcs3, &attente, moteur)?;
    Ok(n)
}

/// La clé de version sous laquelle RPCS3 cherche l'activation d'un patch (`apply`, `bin_patch.cpp`) : la version du
/// jeu si le patch la cite, sinon « All » s'il vaut pour toutes ; `None` : pas pour cette version.
pub fn cle_version(versions_du_jeu: &Value, version_jeu: Option<&str>) -> Option<String> {
    match versions_du_jeu {
        Value::String(s) if s == "All" => Some("All".into()),
        Value::Array(l) => {
            let l: Vec<&str> = l.iter().filter_map(Value::as_str).collect();
            match version_jeu {
                Some(v) if l.contains(&v) => Some(v.to_string()),
                _ if l.contains(&"All") => Some("All".into()),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Un nœud du fichier de RPCS3, dans l'ordre : une carte, ou une valeur déjà écrite en YAML.
#[derive(Debug, Clone, PartialEq)]
enum Noeud {
    Carte(Vec<(String, Noeud)>),
    Valeur(String),
}

/// Le texte d'une clé (RPCS3 lit toutes ses clés comme du texte : « 01.00 » reste « 01.00 »). Une clé lue comme un
/// entier ou un booléen (« 007 », « 0x10 », « yes ») ne se réécrirait pas à l'identique : le fichier n'est alors pas
/// touché.
fn texte(y: &Yaml) -> Option<String> {
    match y {
        Yaml::String(s) | Yaml::Real(s) => Some(s.clone()),
        _ => None,
    }
}

/// Une chaîne entre guillemets doubles (apostrophes, « : », « # », accents : sans risque).
fn guillemets(s: &str) -> String {
    let mut r = String::with_capacity(s.len() + 2);
    r.push('"');
    for c in s.chars() {
        match c {
            '\\' => r.push_str("\\\\"),
            '"' => r.push_str("\\\""),
            '\n' => r.push_str("\\n"),
            '\r' => r.push_str("\\r"),
            '\t' => r.push_str("\\t"),
            c if c.is_control() => r.push_str(&format!("\\u{:04X}", c as u32)),
            c => r.push(c),
        }
    }
    r.push('"');
    r
}

/// Le fichier de RPCS3 a 6 niveaux au plus : au-delà, ce n'est pas lui (et jamais une pile épuisée).
const PROFONDEUR_MAX: usize = 12;

fn depuis(y: &Yaml, profondeur: usize) -> Resultat<Noeud> {
    let illisible = || Erreur::Refus("Le fichier des patchs de RPCS3 a une forme inattendue : Frogtend n'y touche pas.".into());
    if profondeur > PROFONDEUR_MAX {
        return Err(illisible());
    }
    Ok(match y {
        Yaml::Hash(h) => Noeud::Carte(h.iter().map(|(k, v)| Ok((texte(k).ok_or_else(illisible)?, depuis(v, profondeur + 1)?))).collect::<Resultat<_>>()?),
        Yaml::Boolean(b) => Noeud::Valeur(b.to_string()),
        Yaml::Integer(i) => Noeud::Valeur(i.to_string()),
        Yaml::Real(s) => Noeud::Valeur(s.clone()),
        Yaml::String(s) => Noeud::Valeur(guillemets(s)),
        Yaml::Null => Noeud::Valeur("~".into()),
        _ => return Err(illisible()),
    })
}

/// Le fichier des activations lu (vide ou absent : rien). Illisible : refusé, il n'est jamais réécrit.
fn lire(texte_config: &str) -> Resultat<Vec<(String, Noeud)>> {
    if texte_config.trim().is_empty() {
        return Ok(Vec::new());
    }
    let docs = YamlLoader::load_from_str(texte_config)
        .map_err(|_| Erreur::Refus("Le fichier des patchs de RPCS3 ne se lit pas : Frogtend n'y touche pas.".into()))?;
    match docs.first() {
        None | Some(Yaml::Null) => Ok(Vec::new()),
        Some(y) => match depuis(y, 0)? {
            Noeud::Carte(c) => Ok(c),
            Noeud::Valeur(_) => Err(Erreur::Refus("Le fichier des patchs de RPCS3 a une forme inattendue : Frogtend n'y touche pas.".into())),
        },
    }
}

fn ecrire(carte: &[(String, Noeud)], profondeur: usize, sortie: &mut String) {
    for (k, v) in carte {
        sortie.push_str(&"  ".repeat(profondeur));
        sortie.push_str(&guillemets(k));
        sortie.push(':');
        match v {
            Noeud::Carte(c) if c.is_empty() => sortie.push_str(" {}\n"),
            Noeud::Carte(c) => {
                sortie.push('\n');
                ecrire(c, profondeur + 1, sortie);
            }
            Noeud::Valeur(t) => {
                sortie.push(' ');
                sortie.push_str(t);
                sortie.push('\n');
            }
        }
    }
}

/// La sous-carte `cle` de `carte` (créée vide si elle manque ; une valeur à cette place devient une carte).
fn sous_carte<'a>(carte: &'a mut Vec<(String, Noeud)>, cle: &str) -> &'a mut Vec<(String, Noeud)> {
    let i = match carte.iter().position(|(k, _)| k == cle) {
        Some(i) => i,
        None => {
            carte.push((cle.to_string(), Noeud::Carte(Vec::new())));
            carte.len() - 1
        }
    };
    if !matches!(carte[i].1, Noeud::Carte(_)) {
        // Ancienne forme de RPCS3 (`01.00: true`) : devient `{Enabled: true}`.
        let ancien = std::mem::replace(&mut carte[i].1, Noeud::Carte(Vec::new()));
        if let (Noeud::Valeur(v), Noeud::Carte(c)) = (ancien, &mut carte[i].1) {
            if v == "true" {
                c.push(("Enabled".into(), Noeud::Valeur("true".into())));
            }
        }
    }
    match &mut carte[i].1 {
        Noeud::Carte(c) => c,
        Noeud::Valeur(_) => unreachable!("remplacée par une carte juste au-dessus"),
    }
}

/// Retire les cartes devenues vides sur le chemin (jamais ailleurs).
fn elaguer(carte: &mut Vec<(String, Noeud)>, chemin: &[&str]) {
    let Some((tete, reste)) = chemin.split_first() else { return };
    if let Some(i) = carte.iter().position(|(k, _)| k == tete) {
        if let Noeud::Carte(c) = &mut carte[i].1 {
            elaguer(c, reste);
            if c.is_empty() {
                carte.remove(i);
            }
        }
    }
}

/// Un texte venu de Firehouse, avant d'entrer dans le fichier de RPCS3 : non vide, raisonnable, sans caractère de
/// contrôle.
fn texte_sur(t: &str) -> bool {
    !t.trim().is_empty() && t.chars().count() <= 300 && !t.chars().any(char::is_control)
}

/// Une clé de patch de RPCS3 : `PPU-`, `SPU-`, `PRX-` ou `OVL-` suivi d'une empreinte hexadécimale.
pub fn hash_sur(h: &str) -> bool {
    let Some((genre, hex)) = h.split_once('-') else { return false };
    ["PPU", "SPU", "PRX", "OVL"].contains(&genre) && !hex.is_empty() && hex.len() <= 64 && hex.chars().all(|c| c.is_ascii_hexdigit())
}

/// Un patch désigné exactement comme RPCS3 le range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Patch {
    pub hash: String,
    pub description: String,
    pub titre: String,
    pub numero: String,
    pub version: String,
}

impl Patch {
    fn verifier(&self) -> Resultat<()> {
        let ok = hash_sur(&self.hash)
            && texte_sur(&self.description)
            && texte_sur(&self.titre)
            && crate::emulateurs_profils::numero_ps3_sur(&self.numero)
            && (self.version == "All" || (self.version.len() == 5 && self.version.as_bytes()[2] == b'.'));
        if ok {
            Ok(())
        } else {
            Err(Erreur::Refus("Patch inattendu : rien n'est écrit.".into()))
        }
    }

    fn chemin(&self) -> [&str; 5] {
        [&self.hash, &self.description, &self.titre, &self.numero, &self.version]
    }
}

/// Le texte du fichier des activations avec ce patch activé ou non ; `None` : rien ne change. Tout le reste est gardé.
pub fn regler(texte_config: &str, p: &Patch, actif: bool) -> Resultat<Option<String>> {
    p.verifier()?;
    let mut carte = lire(texte_config)?;
    let avant = carte.clone();
    let c = p.chemin();
    let mut feuille = &mut carte;
    for cle in c {
        feuille = sous_carte(feuille, cle);
    }
    feuille.retain(|(k, _)| k != "Enabled");
    if actif {
        feuille.insert(0, ("Enabled".into(), Noeud::Valeur("true".into())));
    }
    elaguer(&mut carte, &c);
    if carte == avant {
        return Ok(None);
    }
    let mut sortie = String::new();
    ecrire(&carte, 0, &mut sortie);
    Ok(Some(sortie))
}

/// Les patchs ACTIVÉS pour ce jeu dans le fichier des activations (illisible : aucun, sans erreur).
pub fn actives(texte_config: &str, numero: &str) -> Vec<Patch> {
    let mut r = Vec::new();
    let Ok(carte) = lire(texte_config) else { return r };
    let cartes = |n: &Noeud| match n {
        Noeud::Carte(c) => c.clone(),
        Noeud::Valeur(_) => Vec::new(),
    };
    for (hash, n1) in &carte {
        for (description, n2) in cartes(n1) {
            for (titre, n3) in cartes(&n2) {
                for (serie, n4) in cartes(&n3) {
                    if serie != numero {
                        continue;
                    }
                    for (version, n5) in cartes(&n4) {
                        let oui = match &n5 {
                            Noeud::Carte(c) => c.iter().any(|(k, v)| k == "Enabled" && *v == Noeud::Valeur("true".into())),
                            Noeud::Valeur(v) => v == "true",
                        };
                        if oui {
                            r.push(Patch { hash: hash.clone(), description: description.clone(), titre: titre.clone(), numero: serie.clone(), version });
                        }
                    }
                }
            }
        }
    }
    r
}

/// Le fichier des activations de RPCS3 (absent : vide ; démesuré ou illisible : erreur, il ne sera pas réécrit).
pub fn lire_config(rpcs3: &Path) -> Resultat<String> {
    let f = chemin_config(rpcs3);
    match std::fs::metadata(&f) {
        Ok(m) if m.len() > CONFIG_MAX => Err(Erreur::Refus("Le fichier des patchs de RPCS3 est démesuré : Frogtend n'y touche pas.".into())),
        Ok(_) => Ok(std::fs::read_to_string(&f)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(e.into()),
    }
}

/// Écrit le fichier des activations : l'ancien copié à l'abri, le nouveau écrit à côté puis renommé (jamais à moitié).
pub fn ecrire_config(rpcs3: &Path, nouveau: &str) -> Resultat<()> {
    let f = chemin_config(rpcs3);
    crate::emulateurs_profils::mettre_a_l_abri(rpcs3, &f)?;
    if let Some(p) = f.parent() {
        std::fs::create_dir_all(p)?;
    }
    let tmp = f.with_file_name("patch_config.yml.frogtend-tmp");
    std::fs::write(&tmp, nouveau)?;
    std::fs::rename(&tmp, &f).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn p(description: &str, version: &str) -> Patch {
        Patch {
            hash: "PPU-69f53b470d81ea961c1c2ff264ade6ab8077d2a1".into(),
            description: description.into(),
            titre: "Asura's Wrath".into(),
            numero: "BLES01227".into(),
            version: version.into(),
        }
    }

    #[test]
    fn la_cle_de_version_suit_rpcs3() {
        assert_eq!(cle_version(&json!("All"), Some("01.00")).as_deref(), Some("All"));
        assert_eq!(cle_version(&json!(["01.00", "01.01"]), Some("01.01")).as_deref(), Some("01.01"));
        assert_eq!(cle_version(&json!(["01.00", "All"]), Some("01.02")).as_deref(), Some("All"));
        assert_eq!(cle_version(&json!(["01.00"]), Some("01.02")), None, "pas pour cette version");
        assert_eq!(cle_version(&json!(["01.00"]), None), None);
        assert_eq!(cle_version(&json!(42), Some("01.00")), None);
    }

    #[test]
    fn activer_un_patch_dans_un_fichier_vide_puis_le_relire() {
        let t = regler("", &p("Disable Motion Blur", "All"), true).unwrap().unwrap();
        assert_eq!(
            t,
            "\"PPU-69f53b470d81ea961c1c2ff264ade6ab8077d2a1\":\n  \"Disable Motion Blur\":\n    \"Asura's Wrath\":\n      \"BLES01227\":\n        \"All\":\n          \"Enabled\": true\n"
        );
        // Relu comme RPCS3 (yaml-cpp) le lirait.
        assert_eq!(actives(&t, "BLES01227"), [p("Disable Motion Blur", "All")]);
        assert!(actives(&t, "BLUS30721").is_empty());
        // Déjà activé : rien à écrire. Désactivé : le fichier redevient vide (nœuds vides retirés).
        assert_eq!(regler(&t, &p("Disable Motion Blur", "All"), true).unwrap(), None);
        assert_eq!(regler(&t, &p("Disable Motion Blur", "All"), false).unwrap().as_deref(), Some(""));
    }

    #[test]
    fn tout_le_reste_du_fichier_est_garde_tel_que_rpcs3_l_ecrit() {
        // La forme écrite par RPCS3 (yaml-cpp) : clés sans guillemets, versions « 01.00 », valeurs réglées, l'ancienne
        // forme `01.00: true`, un autre jeu, des titres piégés (apostrophe, deux-points, dièse, guillemet), CRLF.
        let rpcs3 = "PPU-aaaa:\r\n  Unlock FPS:\r\n    \"Demon's Souls: \\\"#1\\\"\":\r\n      BLES00932:\r\n        01.00:\r\n          Enabled: true\r\n          Configurable Values:\r\n            Ratio: 2.4\r\n  21:9 (3840x1600):\r\n    Demon's Souls:\r\n      BLES00932:\r\n        01.00: true\r\n";
        let t = regler(rpcs3, &p("Unlock FPS", "All"), true).unwrap().unwrap();
        let docs = YamlLoader::load_from_str(&t).unwrap();
        let d = &docs[0];
        let ds = &d["PPU-aaaa"]["Unlock FPS"]["Demon's Souls: \"#1\""]["BLES00932"]["01.00"];
        assert_eq!(ds["Enabled"].as_bool(), Some(true));
        assert_eq!(ds["Configurable Values"]["Ratio"].as_f64(), Some(2.4));
        assert_eq!(d["PPU-aaaa"]["21:9 (3840x1600)"]["Demon's Souls"]["BLES00932"]["01.00"].as_bool(), Some(true), "ancienne forme gardée telle quelle (RPCS3 la lit encore)");
        assert_eq!(d["PPU-69f53b470d81ea961c1c2ff264ade6ab8077d2a1"]["Unlock FPS"]["Asura's Wrath"]["BLES01227"]["All"]["Enabled"].as_bool(), Some(true));
        // Désactiver Asura ne touche pas Demon's Souls ; une valeur réglée garde son patch même désactivé.
        let t2 = regler(&t, &p("Unlock FPS", "All"), false).unwrap().unwrap();
        assert!(!t2.contains("BLES01227") && t2.contains("\"Ratio\": 2.4"), "{t2}");
        let mut sans = p("Unlock FPS", "01.00");
        sans.hash = "PPU-aaaa".into();
        sans.titre = "Demon's Souls: \"#1\"".into();
        sans.numero = "BLES00932".into();
        let t3 = regler(&t2, &sans, false).unwrap().unwrap();
        let d3 = &YamlLoader::load_from_str(&t3).unwrap()[0];
        assert!(d3["PPU-aaaa"]["Unlock FPS"]["Demon's Souls: \"#1\""]["BLES00932"]["01.00"]["Enabled"].is_badvalue());
        assert_eq!(d3["PPU-aaaa"]["Unlock FPS"]["Demon's Souls: \"#1\""]["BLES00932"]["01.00"]["Configurable Values"]["Ratio"].as_f64(), Some(2.4));
    }

    #[test]
    fn un_fichier_illisible_ou_un_patch_douteux_ne_sont_jamais_ecrits() {
        assert!(regler("PPU-a: [1, 2\n  : :", &p("X", "All"), true).is_err());
        assert!(regler("- liste\n- pas une carte\n", &p("X", "All"), true).is_err());
        assert!(actives("PPU-a: [1, 2\n  : :", "BLES01227").is_empty());
        let mut d = p("X", "All");
        d.hash = "../x".into();
        assert!(regler("", &d, true).is_err());
        let mut d = p("X\nY", "All");
        assert!(regler("", &d, true).is_err());
        d = p("X", "1.0/..");
        assert!(regler("", &d, true).is_err());
        d = p("X", "All");
        d.numero = "../../x".into();
        assert!(regler("", &d, true).is_err());
    }

    #[test]
    fn la_liste_n_est_posee_que_si_elle_annonce_le_bon_moteur() {
        assert_eq!(version_moteur("Version: 1.2\n\nAnchors:\n").as_deref(), Some("1.2"));
        assert_eq!(version_moteur("# commentaire\nVersion: \"1.2\"\n").as_deref(), Some("1.2"));
        assert_eq!(version_moteur("Anchors:\n").as_deref(), None);
        let d = tempfile::tempdir().unwrap();
        let rpcs3 = d.path();
        std::fs::create_dir_all(rpcs3.join("patches")).unwrap();
        std::fs::write(rpcs3.join("patches").join("imported_patch.yml"), "a moi").unwrap();
        std::fs::write(chemin_liste(rpcs3), "Version: 1.1\nancienne\n").unwrap();
        let t = rpcs3.join("patches").join("patch.yml.frogtend-nouveau");
        std::fs::write(&t, "Version: 9.9\n").unwrap();
        assert!(poser_liste(rpcs3, &t, "1.2").is_err());
        assert!(!t.exists(), "le fichier refusé est retiré");
        assert_eq!(std::fs::read_to_string(chemin_liste(rpcs3)).unwrap(), "Version: 1.1\nancienne\n");
        std::fs::write(&t, "Version: 1.2\nnouvelle\n").unwrap();
        poser_liste(rpcs3, &t, "1.2").unwrap();
        assert_eq!(std::fs::read_to_string(chemin_liste(rpcs3)).unwrap(), "Version: 1.2\nnouvelle\n");
        assert_eq!(std::fs::read_to_string(rpcs3.join("patches").join("imported_patch.yml")).unwrap(), "a moi");
        // L'ancienne est à l'abri.
        let abri: Vec<_> = walk(&rpcs3.join(".frogtend-sauvegardes"));
        assert!(abri.iter().any(|f| std::fs::read_to_string(f).unwrap_or_default().contains("ancienne")), "{abri:?}");
    }

    #[test]
    fn ecrire_le_fichier_des_activations_garde_l_ancien_a_l_abri() {
        let d = tempfile::tempdir().unwrap();
        let rpcs3 = d.path();
        assert_eq!(lire_config(rpcs3).unwrap(), "");
        ecrire_config(rpcs3, "\"a\": 1\n").unwrap();
        ecrire_config(rpcs3, "\"b\": 2\n").unwrap();
        assert_eq!(lire_config(rpcs3).unwrap(), "\"b\": 2\n");
        assert!(!rpcs3.join("config").join("patch_config.yml.frogtend-tmp").exists());
        assert!(walk(&rpcs3.join(".frogtend-sauvegardes")).iter().any(|f| std::fs::read_to_string(f).unwrap_or_default() == "\"a\": 1\n"));
        // Démesuré : refusé.
        std::fs::File::create(chemin_config(rpcs3)).unwrap().set_len(CONFIG_MAX + 1).unwrap();
        assert!(lire_config(rpcs3).is_err());
    }

    /// RÉEL, sur une COPIE de RPCS3 (`FROGTEND_RPCS3_COPIE` : un dossier avec `config\` copié) : la vraie liste de
    /// Firehouse téléchargée (sha256 vérifié) et posée, puis « Disable Motion Blur » activé pour Asura's Wrath, relu.
    /// `FROGTEND_PROFIL_ESSAI=<id> FROGTEND_RPCS3_COPIE=<dossier> cargo test essai_patchs_copie -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_patchs_copie() {
        use crate::coffre::Coffre as _;
        let rpcs3 = PathBuf::from(std::env::var("FROGTEND_RPCS3_COPIE").unwrap());
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").unwrap();
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton");
        let s = crate::source::Source::Firehouse(crate::firehouse::Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap());
        let etat = s.patchs_rpcs3_etat().await.unwrap();
        let sha = etat["sha256"].as_str().unwrap();
        let n = installer_liste(&s, &rpcs3, &etat).await.unwrap();
        println!("LISTE posée : {n} octets, empreinte locale = annoncée : {}", empreinte_liste(&rpcs3).as_deref() == Some(sha));
        let jeu = s.patchs_rpcs3_jeu("BLES01227", Some("01.00")).await.unwrap();
        let flou = jeu["patchs"].as_array().unwrap().iter().find(|p| p["description"] == "Disable Motion Blur" && p["titre"] == "Asura's Wrath").unwrap();
        let p = Patch {
            hash: flou["hash"].as_str().unwrap().into(),
            description: "Disable Motion Blur".into(),
            titre: "Asura's Wrath".into(),
            numero: "BLES01227".into(),
            version: cle_version(&flou["versions_du_jeu"], Some("01.00")).unwrap(),
        };
        let avant = lire_config(&rpcs3).unwrap();
        let apres = regler(&avant, &p, true).unwrap().unwrap();
        ecrire_config(&rpcs3, &apres).unwrap();
        let relu = lire_config(&rpcs3).unwrap();
        println!("patch_config.yml :\n{relu}");
        assert_eq!(actives(&relu, "BLES01227"), [p]);
        // Le patch existe bien dans la liste posée (même hash, même titre, même numéro).
        let liste = std::fs::read_to_string(chemin_liste(&rpcs3)).unwrap();
        assert!(liste.contains(flou["hash"].as_str().unwrap()) && liste.contains("Disable Motion Blur"));
    }

    #[tokio::test]
    async fn la_liste_de_firehouse_s_installe_d_apres_son_etat_reel_et_jamais_au_dela() {
        use httpmock::prelude::*;
        use sha2::Digest;
        let contenu = "Version: 1.2\n\nAnchors:\n";
        let sha = format!("{:x}", sha2::Sha256::digest(contenu.as_bytes()));
        let m = MockServer::start();
        m.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/emulateurs/rpcs3/patchs/fichier").query_param("v", "1.2");
            t.status(200).body(contenu);
        });
        let s = crate::source::Source::Firehouse(crate::firehouse::Client::nouveau(&m.base_url(), "jeton").unwrap());
        // L'état sous sa forme RÉELLE (relevé du 19/10) : `fichier` est un chemin complet depuis la racine.
        let etat = |taille: u64, sha: &str, fichier: &str, moteur: &str| {
            json!({"ok": true, "version_moteur": moteur, "sha256": sha, "taille": taille, "maj_le": "2026-10-19T16:37:00", "vu_le": "2026-10-19T16:37:15", "fichier": fichier})
        };
        let bon = "/api/jeux/v1/emulateurs/rpcs3/patchs/fichier?v=1.2";
        let d = tempfile::tempdir().unwrap();
        let rpcs3 = d.path();
        let patches = rpcs3.join("patches");
        std::fs::create_dir_all(&patches).unwrap();
        // Des restes d'une ancienne liste (les nôtres) et des fichiers de la personne qui leur ressemblent.
        for f in ["patch-0123456789abcdef.yml.part", "patch-0123456789abcdef.yml.frogtend", "patch-x.yml.frogtend-bak", "imported_patch.yml", "BLES01227_patch.yml"] {
            std::fs::write(patches.join(f), "x").unwrap();
        }
        let n = contenu.len() as u64;
        // Une description douteuse : rien n'est demandé au serveur.
        assert!(installer_liste(&s, rpcs3, &etat(n, &sha, "/api/jeux/v1/autre/chose", "1.2")).await.is_err());
        assert!(installer_liste(&s, rpcs3, &etat(0, &sha, bon, "1.2")).await.is_err());
        assert!(installer_liste(&s, rpcs3, &etat(LISTE_MAX + 1, &sha, bon, "1.2")).await.is_err());
        assert!(installer_liste(&s, rpcs3, &etat(n, &sha, bon, "1.2; rm")).await.is_err());
        assert!(!chemin_liste(rpcs3).exists());
        // Plus gros qu'annoncé : coupé, rien ne reste.
        assert!(installer_liste(&s, rpcs3, &etat(5, &sha, bon, "1.2")).await.is_err());
        assert!(!chemin_liste(rpcs3).exists());
        assert!(std::fs::read_dir(&patches).unwrap().flatten().all(|e| !e.file_name().to_string_lossy().starts_with(&format!("patch-{}", &sha[..16]))));
        // Le bon état : posée, et seuls NOS restes d'avant ont disparu.
        assert_eq!(installer_liste(&s, rpcs3, &etat(n, &sha, bon, "1.2")).await.unwrap(), n);
        assert_eq!(std::fs::read_to_string(chemin_liste(rpcs3)).unwrap(), contenu);
        let reste: Vec<String> = std::fs::read_dir(&patches).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
        for f in ["patch-x.yml.frogtend-bak", "imported_patch.yml", "BLES01227_patch.yml", "patch.yml"] {
            assert!(reste.contains(&f.to_string()), "{reste:?}");
        }
        assert!(!reste.iter().any(|f| f.starts_with("patch-0123456789abcdef")), "{reste:?}");
        assert_eq!(std::fs::read_to_string(patches.join("imported_patch.yml")).unwrap(), "x");
    }

    #[test]
    fn un_fichier_qui_ne_se_reecrirait_pas_a_l_identique_n_est_pas_touche() {
        assert!(reste_de_frogtend("patch-0123456789abcdef.yml.part") && reste_de_frogtend("patch-0123456789ABCDEF.yml.frogtend"));
        assert!(!reste_de_frogtend("patch-x.yml.frogtend") && !reste_de_frogtend("patch-0123456789abcdef.yml.frogtend-bak") && !reste_de_frogtend("patch.yml"));
        // Un titre lu comme un entier (« 007 ») ou un booléen (« true » ; « yes » reste un texte en YAML 1.2) : réécrit,
        // il changerait ; refusé.
        assert!(regler("PPU-aa:\n  Unlock FPS:\n    007:\n      BLES00932:\n        01.00:\n          Enabled: true\n", &p("X", "All"), true).is_err());
        assert!(regler("PPU-aa:\n  true:\n    T:\n      BLES00932:\n        All: true\n", &p("X", "All"), true).is_err());
        // Bien trop imbriqué : ce n'est pas le fichier de RPCS3 (et jamais une pile épuisée).
        let mut profond = String::new();
        for i in 0..40 {
            profond.push_str(&format!("{}a{i}:\n", "  ".repeat(i)));
        }
        assert!(regler(&profond, &p("X", "All"), true).is_err());
    }

    fn walk(d: &Path) -> Vec<PathBuf> {
        let mut r = Vec::new();
        if let Ok(l) = std::fs::read_dir(d) {
            for e in l.flatten() {
                if e.path().is_dir() {
                    r.extend(walk(&e.path()));
                } else {
                    r.push(e.path());
                }
            }
        }
        r
    }
}
