//! Les NOMS FRANÇAIS des DLC et mises à jour Switch (contrat 1.47, besoin 54). La route 1
//! (`GET /switch/{tid_base}/contenus`) donne `connus` : tous les DLC et MAJ connus du jeu, servis ou non, avec leur nom
//! français officiel (`nom_fr`, null s'il n'existe pas) et leur nom d'origine (`nom`). Frogtend montre, dans l'ordre :
//! `nom_fr`, sinon `nom`, sinon le nom tiré du fichier (l'interface s'en charge, `nomCourt`).
//! La correspondance est gardée PAR PROFIL (dossier de la session, écrite par `Session::garder_fichier`) pour le hors
//! ligne ; elle est oubliée avec le reste quand le profil change de compte (`connus` est vide si une fiche est cachée à
//! ce jeton : ce qu'un compte a reçu n'est jamais montré à un autre).

use crate::erreurs::{Erreur, Resultat};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Le dossier du cache des noms, dans le dossier de la session (oublié par `Noyau::oublier`).
pub const DOSSIER_CACHE: &str = "contenus-switch-noms";

/// Un DLC ou une MAJ connu de Firehouse (contrat 1.47).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Connu {
    pub tid: u64,
    /// `dlc` ou `maj`.
    pub sorte: String,
    pub nom_fr: Option<String>,
    pub nom: Option<String>,
    /// MAJ : la version brute ; DLC : 0.
    pub version: Option<u64>,
}

impl Connu {
    /// Le nom à montrer : le français officiel, sinon celui d'origine.
    pub fn nom_montre(&self) -> Option<String> {
        self.nom_fr.clone().or_else(|| self.nom.clone())
    }
}

fn texte(v: &Value) -> Option<String> {
    v.as_str().map(str::trim).filter(|s| !s.is_empty()).map(String::from)
}

fn entier(v: &Value) -> Option<u64> {
    match v {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => s.trim().trim_start_matches(['v', 'V']).parse().ok(),
        _ => None,
    }
}

fn tid(v: &Value) -> Option<u64> {
    let s = v.as_str()?.trim();
    (s.len() == 16).then(|| u64::from_str_radix(s, 16).ok()).flatten()
}

/// Les contenus connus d'une réponse de la route 1 : `connus` (absent : rien), complétés par les noms des contenus
/// SERVIS (`majs[]`, `dlc[]` : `nom_fr`, `titre`) qui n'y seraient pas. Lu avec indulgence : une entrée illisible est
/// ignorée.
pub fn lire_connus(v: &Value) -> Vec<Connu> {
    let mut l: Vec<Connu> = Vec::new();
    for c in v["connus"].as_array().into_iter().flatten() {
        let Some(t) = tid(&c["tid"]) else { continue };
        let sorte = texte(&c["type"]).unwrap_or_default().to_lowercase();
        l.push(Connu { tid: t, sorte, nom_fr: texte(&c["nom_fr"]), nom: texte(&c["nom"]), version: entier(&c["version"]) });
    }
    for (cle, sorte) in [("majs", "maj"), ("dlc", "dlc")] {
        for c in v[cle].as_array().into_iter().flatten() {
            let Some(t) = tid(&c["tid"]) else { continue };
            let version = entier(&c["version"]);
            let deja = l.iter().any(|x| x.tid == t && (sorte == "dlc" || x.version == version));
            let (nom_fr, nom) = (texte(&c["nom_fr"]), texte(&c["titre"]));
            if !deja && (nom_fr.is_some() || nom.is_some()) {
                l.push(Connu { tid: t, sorte: sorte.into(), nom_fr, nom, version });
            }
        }
    }
    l
}

// --- Le cache par profil ---

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Cache {
    vu_le: String,
    connus: Vec<Connu>,
}

pub fn fichier_cache(dossier_session: &Path, tid_base: u64) -> PathBuf {
    dossier_session.join(DOSSIER_CACHE).join(format!("{tid_base:016X}.json"))
}

fn lire_cache(dossier_session: &Path, tid_base: u64) -> Option<Vec<Connu>> {
    let f = fichier_cache(dossier_session, tid_base);
    if std::fs::metadata(&f).ok()?.len() > 8 << 20 {
        return None;
    }
    serde_json::from_slice::<Cache>(&std::fs::read(f).ok()?).ok().map(|c| c.connus)
}

/// Lit les noms connus d'un jeu (la route 1, ou le cache du profil si Firehouse ne répond pas) et garde la réponse
/// par `garder` (`Session::garder_fichier` : jamais après un changement de jeton). 401 remonte ; 404 → rien (cache
/// vidé) ; une panne de réseau ou une erreur 5xx → le cache ; toute autre erreur → rien (les noms du fichier restent).
pub async fn lire(source: &crate::source::Source, dossier_session: &Path, tid_base: u64, garder: crate::mods_switch::Garder<'_>) -> Resultat<Vec<Connu>> {
    let r = match source {
        crate::source::Source::Firehouse(c) => c.obtenir_json::<Value>(&crate::envois_switch::route_contenus(tid_base)).await,
        crate::source::Source::Simulee => return Ok(vec![]),
    };
    let ecrire = |connus: &[Connu]| -> Resultat<()> {
        let c = Cache { vu_le: crate::noyau::maintenant(), connus: connus.to_vec() };
        garder(&fichier_cache(dossier_session, tid_base), &serde_json::to_vec(&c).map_err(|e| Erreur::Disque(e.to_string()))?)
    };
    match r {
        Ok(v) => {
            let l = lire_connus(&v);
            ecrire(&l)?;
            Ok(l)
        }
        Err(Erreur::JetonRefuse(m)) => Err(Erreur::JetonRefuse(m)),
        Err(Erreur::Introuvable(_)) => {
            ecrire(&[])?;
            Ok(vec![])
        }
        Err(Erreur::Reseau(_) | Erreur::Serveur(_)) => Ok(lire_cache(dossier_session, tid_base).unwrap_or_default()),
        Err(_) => Ok(vec![]),
    }
}

// --- Les noms par contenu ---

/// La version affichée d'une mise à jour écrite dans son nom de fichier (« [v1.2.0] » → « 1.2.0 »).
pub fn version_affichee_du_nom(nom: &str) -> Option<String> {
    nom.split('[').skip(1).filter_map(|m| m.split(']').next()).find_map(|e| {
        let e = e.trim();
        let r = e.strip_prefix(['v', 'V'])?;
        let parties: Vec<&str> = r.split('.').collect();
        ((2..=4).contains(&parties.len()) && parties.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))).then(|| r.to_string())
    })
}

/// Le nom d'une mise à jour : celui de la MAJ connue de MÊME version ; sinon « Mise à jour » et la version lue dans
/// le nom du fichier (affichée, sinon brute) ; sinon rien (le nom du fichier).
pub fn nom_de_maj(connus: &[Connu], base: u64, version: Option<u32>, fichier: &str) -> Option<String> {
    if let Some(v) = version {
        let tid_maj = base + 0x800;
        if let Some(n) = connus.iter().filter(|c| c.sorte == "maj" && c.tid == tid_maj && c.version == Some(v as u64)).find_map(Connu::nom_montre) {
            return Some(n);
        }
    }
    if let Some(a) = version_affichee_du_nom(fichier) {
        return Some(format!("Mise à jour {a}"));
    }
    version.map(|v| format!("Mise à jour (version {v})"))
}

/// Le nom d'un DLC (ses identifiants, le premier connu qui a un nom).
pub fn nom_de_dlc(connus: &[Connu], tids: &[u64]) -> Option<String> {
    tids.iter().find_map(|t| connus.iter().filter(|c| c.tid == *t && c.sorte != "maj").find_map(Connu::nom_montre))
}

/// Les noms à montrer pour les contenus lus d'un jeu : par clé d'interrupteur (« maj », « dlc:… ») et par fichier
/// (`id` de `ContenuVu` : le chemin), y compris les anciennes mises à jour et les .nsz.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct NomsContenus {
    pub par_cle: BTreeMap<String, String>,
    pub par_id: BTreeMap<String, String>,
}

pub fn noms(connus: &[Connu], c: &crate::switch_nca::ContenusDuJeu) -> NomsContenus {
    use crate::switch_nca::Sorte;
    let mut r = NomsContenus::default();
    let nom_fichier = |m: &crate::switch_nca::ContenuLu| m.chemin.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let tous = c.maj.iter().chain(c.majs_anciennes.iter()).chain(c.dlc.iter()).chain(c.compresses.iter());
    for m in tous {
        let n = match &m.sorte {
            Sorte::MiseAJour => nom_de_maj(connus, c.base, m.version, &nom_fichier(m)),
            Sorte::Dlc(ncas) => nom_de_dlc(connus, &ncas.iter().map(|(_, t)| *t).collect::<Vec<_>>()),
            Sorte::Jeu => None,
        };
        if let Some(n) = n {
            r.par_id.insert(m.chemin.to_string_lossy().to_string(), n);
        }
    }
    if let Some(m) = &c.maj {
        if let Some(n) = nom_de_maj(connus, c.base, m.version, &nom_fichier(m)) {
            r.par_cle.insert(crate::contenus_switch::CLE_MAJ.into(), n);
        }
    }
    for m in &c.dlc {
        if let Sorte::Dlc(ncas) = &m.sorte {
            if let Some(n) = nom_de_dlc(connus, &ncas.iter().map(|(_, t)| *t).collect::<Vec<_>>()) {
                r.par_cle.insert(crate::switch_nca::identifiant_dlc(ncas), n);
            }
        }
    }
    r
}

/// Une réponse de la forme réelle (Fire Emblem Three Houses, contrat 1.47) pour les tests et le mode simulé.
pub fn exemple_fire_emblem() -> Value {
    json!({
        "tid_base": "010055D009F78000", "jeu": 42,
        "majs": [{"tid": "010055D009F78800", "version": 327680, "nom_fr": "Mise à jour 1.2.0", "titre": "Update 1.2.0"}],
        "dlc": [],
        "connus": [
            {"tid": "010055D009F79001", "type": "dlc", "nom_fr": "Histoire additionnelle - Ombres embrasées", "nom": "Side Story: Cindered Shadows", "version": 0},
            {"tid": "010055D009F79002", "type": "dlc", "nom_fr": "Nouvelles tenues", "nom": "New Outfits", "version": 0},
            {"tid": "010055D009F79003", "type": "dlc", "nom_fr": null, "nom": "New Skirmishes", "version": 0},
            {"tid": "010055D009F78800", "type": "maj", "nom_fr": "Mise à jour 1.2.0", "nom": "Update", "version": 327680, "version_affichee": "1.2.0", "publiee_le": "2019-11-07"}
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::switch_nca::{ContenuLu, ContenusDuJeu, Forme, Sorte};
    use httpmock::prelude::*;

    const BASE: u64 = 0x0100_55D0_09F7_8000;

    fn fe() -> ContenusDuJeu {
        let lu = |nom: &str, sorte: Sorte, version: Option<u32>| ContenuLu { chemin: PathBuf::from(format!("D:\\FE\\{nom}")), sorte, base: BASE, version, forme: Forme::Nsp };
        ContenusDuJeu {
            base: BASE,
            maj: Some(lu("Fire Emblem Three Houses [v1.2.0][v327680] (nsw2u).nsp", Sorte::MiseAJour, Some(327680))),
            majs_anciennes: vec![lu("FE [010055D009F78800][v196608].nsp", Sorte::MiseAJour, Some(196608))],
            dlc: vec![
                lu("FE [Side Story][010055D009F79001].nsp", Sorte::Dlc(vec![("/a.nca".into(), BASE + 0x1001)]), None),
                lu("FE [Outfits][010055D009F79002].nsp", Sorte::Dlc(vec![("/b.nca".into(), BASE + 0x1002)]), None),
                lu("FE [Skirmishes][010055D009F79003].nsp", Sorte::Dlc(vec![("/c.nca".into(), BASE + 0x1003)]), None),
                lu("FE [Inconnu][010055D009F79099].nsp", Sorte::Dlc(vec![("/z.nca".into(), BASE + 0x1099)]), None),
            ],
            ..Default::default()
        }
    }

    #[test]
    fn la_forme_reelle_de_fire_emblem_donne_les_noms_francais() {
        let connus = lire_connus(&exemple_fire_emblem());
        let n = noms(&connus, &fe());
        assert_eq!(n.par_cle["maj"], "Mise à jour 1.2.0");
        assert_eq!(n.par_cle["dlc:010055D009F79001"], "Histoire additionnelle - Ombres embrasées");
        assert_eq!(n.par_cle["dlc:010055D009F79002"], "Nouvelles tenues");
        // `nom_fr` null : le nom d'origine.
        assert_eq!(n.par_cle["dlc:010055D009F79003"], "New Skirmishes");
        // Inconnu de Firehouse : rien (le nom du fichier reste).
        assert!(!n.par_cle.contains_key("dlc:010055D009F79099"));
        // Une ancienne MAJ, version pas connue : « Mise à jour (version N) » (rien d'affichable dans son nom).
        assert_eq!(n.par_id["D:\\FE\\FE [010055D009F78800][v196608].nsp"], "Mise à jour (version 196608)");
    }

    #[test]
    fn sans_connus_la_maj_garde_la_version_de_son_nom() {
        let v = json!({"majs": [], "dlc": []});
        assert!(lire_connus(&v).is_empty(), "champ absent : rien");
        let n = noms(&lire_connus(&v), &fe());
        assert_eq!(n.par_cle["maj"], "Mise à jour 1.2.0", "lue dans « [v1.2.0] »");
        assert!(!n.par_cle.keys().any(|k| k.starts_with("dlc:")));
        assert_eq!(version_affichee_du_nom("Jeu [v327680].nsp"), None);
        assert_eq!(version_affichee_du_nom("Jeu [V1.0.3].nsp").as_deref(), Some("1.0.3"));
        // Les noms des contenus SERVIS complètent `connus`.
        let v = json!({"dlc": [{"tid": "010055D009F79002", "nom_fr": "Nouvelles tenues", "titre": "New Outfits"}]});
        assert_eq!(nom_de_dlc(&lire_connus(&v), &[BASE + 0x1002]).as_deref(), Some("Nouvelles tenues"));
        // Entrées illisibles ignorées.
        assert!(lire_connus(&json!({"connus": [{"tid": 5}, "x", {"tid": "zz"}]})).is_empty());
    }

    fn garder_libre(f: &Path, o: &[u8]) -> Resultat<()> {
        std::fs::create_dir_all(f.parent().unwrap())?;
        std::fs::write(f, o)?;
        Ok(())
    }

    #[tokio::test]
    async fn les_noms_sont_gardes_par_profil_pour_le_hors_ligne() {
        let d = tempfile::tempdir().unwrap();
        let (seb, autre) = (d.path().join("seb"), d.path().join("autre"));
        let route = "/api/jeux/v1/switch/010055D009F78000/contenus";
        let m = MockServer::start();
        let mut ok = m.mock(|w, r| {
            w.method(GET).path(route);
            r.status(200).json_body(exemple_fire_emblem());
        });
        let s = crate::source::Source::Firehouse(crate::firehouse::Client::nouveau(&m.base_url(), "j").unwrap());
        assert_eq!(lire(&s, &seb, BASE, &garder_libre).await.unwrap().len(), 4);
        ok.delete();
        let mut panne = m.mock(|w, r| {
            w.method(GET).path(route);
            r.status(503);
        });
        // Hors ligne : le cache de CE profil ; jamais celui d'un autre.
        assert_eq!(lire(&s, &seb, BASE, &garder_libre).await.unwrap().len(), 4);
        assert!(lire(&s, &autre, BASE, &garder_libre).await.unwrap().is_empty());
        panne.delete();
        // 404 (fiche cachée) : rien, et le cache est vidé.
        let _cache = m.mock(|w, r| {
            w.method(GET).path(route);
            r.status(404);
        });
        assert!(lire(&s, &seb, BASE, &garder_libre).await.unwrap().is_empty());
        assert!(lire_cache(&seb, BASE).unwrap().is_empty());
        // 401 remonte.
        let m2 = MockServer::start();
        let _j = m2.mock(|w, r| {
            w.method(GET).path(route);
            r.status(401);
        });
        let s2 = crate::source::Source::Firehouse(crate::firehouse::Client::nouveau(&m2.base_url(), "j").unwrap());
        assert!(matches!(lire(&s2, &seb, BASE, &garder_libre).await, Err(Erreur::JetonRefuse(_))));
        // Une session périmée : `garder` refuse, rien n'est rendu.
        let refuse = |_: &Path, _: &[u8]| -> Resultat<()> { Err(Erreur::Perimee("remplacée".into())) };
        let m3 = MockServer::start();
        let _r = m3.mock(|w, r| {
            w.method(GET).path(route);
            r.status(200).json_body(exemple_fire_emblem());
        });
        let s3 = crate::source::Source::Firehouse(crate::firehouse::Client::nouveau(&m3.base_url(), "j").unwrap());
        assert!(matches!(lire(&s3, &autre, BASE, &refuse).await, Err(Erreur::Perimee(_))));
        assert!(!fichier_cache(&autre, BASE).exists());
    }
}
