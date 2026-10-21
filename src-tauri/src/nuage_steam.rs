//! Les parties d'un jeu Steam gardées pour son nuage, mises à l'abri par Frogtend (lot B2 des jeux de boutique ;
//! Seb, 21/10 : « nuage Steam d'abord »). LECTURE SEULE chez Steam et dans les dossiers de Windows : Frogtend n'écrit
//! que dans le dossier des abris.
//!
//! Steam décrit les fichiers d'un jeu dans `userdata\<compte>\<appid>\remotecache.vdf` : chemin, racine, taille et
//! SHA1 (vérifié sur 50 vrais fichiers le 21/10). La racine 0 est `remote\` à côté ; les autres (Auto-Cloud : Mes
//! documents, AppData…) ont des NUMÉROS que Valve ne documente pas (seuls leurs noms le sont) : Frogtend ne les devine
//! pas, il retrouve chaque fichier par sa taille et son SHA1 parmi les dossiers de Windows lus dans le registre (Mes
//! documents peut être redirigé vers OneDrive : c'est le cas chez Seb). Un fichier OneDrive « en ligne seulement »
//! n'est jamais lu (il serait téléchargé).

use crate::erreurs::{Erreur, Resultat};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};

/// Le steamid64 d'un compte moins cette base = son numéro de dossier dans `userdata` (identifiant 32 bits).
const BASE_STEAMID: u64 = 76_561_197_960_265_728;

/// Au-delà, la personne confirme avant la copie (des parties de jeu font rarement plus).
pub const GROS: u64 = 2 << 30;

/// Le dossier `userdata\<compte>` d'un steamid64 (`None` : pas un steamid64).
pub fn compte(steamid: &str) -> Option<String> {
    let n: u64 = steamid.trim().parse().ok()?;
    n.checked_sub(BASE_STEAMID).filter(|c| *c > 0 && *c <= u32::MAX as u64).map(|c| c.to_string())
}

/// La marque d'un compte Steam dans les abris (`nuage-steam/<marque>/…`) : 8 hexadécimaux, jamais le numéro du
/// compte (les abris partent chez Firehouse). Deux comptes Steam du PC ne mêlent jamais leurs parties.
pub fn marque(compte: &str) -> String {
    use sha1::{Digest, Sha1};
    Sha1::digest(format!("frogtend-steam:{compte}").as_bytes()).iter().take(4).map(|b| format!("{b:02x}")).collect()
}

/// Au plus ces fichiers dans `remote\` (un dossier plus gros n'est pas celui des parties d'un jeu).
const REMOTE_MAX: usize = 20_000;

/// Les fichiers sous `racine` (chemins relatifs en `/`), SANS jamais suivre un lien ni une jonction (un lien dans
/// `remote\` ferait lire un disque entier) ; au-delà de `REMOTE_MAX`, refus.
fn fichiers_sans_lien(racine: &Path) -> Resultat<Vec<String>> {
    let mut l = Vec::new();
    let mut pile = vec![racine.to_path_buf()];
    while let Some(d) = pile.pop() {
        for e in std::fs::read_dir(&d)?.flatten() {
            let p = e.path();
            let Ok(m) = std::fs::symlink_metadata(&p) else { continue };
            if m.file_type().is_symlink() || point_de_reparse(&m) {
                continue;
            }
            if m.is_dir() {
                pile.push(p);
            } else if m.is_file() {
                if let Ok(r) = p.strip_prefix(racine) {
                    l.push(r.to_string_lossy().replace('\\', "/"));
                }
                if l.len() > REMOTE_MAX {
                    return Err(Erreur::Refus("Le dossier du nuage Steam de ce jeu contient trop de fichiers pour être des parties : rien n'est copié.".into()));
                }
            }
        }
    }
    l.sort();
    Ok(l)
}

/// Une jonction, un lien ou un fichier géré par un service (OneDrive…) : FILE_ATTRIBUTE_REPARSE_POINT.
fn point_de_reparse(m: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // Un fichier OneDrive porte aussi cet attribut : seuls les DOSSIERS qui l'ont sont écartés.
        m.is_dir() && m.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        let _ = m;
        false
    }
}

/// Un fichier décrit par `remotecache.vdf`.
#[derive(Debug, Clone, PartialEq)]
pub struct FichierNuage {
    pub chemin: String,
    pub racine: String,
    pub taille: Option<u64>,
    /// SHA1 en 40 hexadécimaux minuscules (vide : inconnu).
    pub sha: String,
    /// `syncstate` différent de 1 : Steam n'a pas fini de synchroniser (conflit possible).
    pub en_attente: bool,
}

#[derive(Debug, PartialEq)]
enum Jeton {
    Texte(String),
    Ouvre,
    Ferme,
}

/// Les jetons du format texte de Valve (`"clé" "valeur"`, `{`, `}` ; `\\` et `\"` dans les textes). Tolérant : un
/// fichier tronqué rend ce qu'il a pu lire, jamais une panique.
fn jetons(t: &str) -> Vec<Jeton> {
    let mut l = Vec::new();
    let mut c = t.chars().peekable();
    while let Some(x) = c.next() {
        match x {
            '{' => l.push(Jeton::Ouvre),
            '}' => l.push(Jeton::Ferme),
            '"' => {
                let mut s = String::new();
                while let Some(y) = c.next() {
                    match y {
                        '"' => break,
                        '\\' => match c.next() {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some(z) => s.push(z),
                            None => break,
                        },
                        _ => s.push(y),
                    }
                }
                l.push(Jeton::Texte(s));
            }
            _ => {}
        }
    }
    l
}

/// Lit `remotecache.vdf` : un bloc par fichier, dans le bloc du jeu.
pub fn lire_remotecache(texte: &str) -> Vec<FichierNuage> {
    let j = jetons(texte);
    let mut l = Vec::new();
    let mut profondeur = 0usize;
    let mut i = 0;
    while i < j.len() {
        match &j[i] {
            Jeton::Ouvre => profondeur += 1,
            Jeton::Ferme => profondeur = profondeur.saturating_sub(1),
            Jeton::Texte(nom) if profondeur == 1 && j.get(i + 1) == Some(&Jeton::Ouvre) => {
                // Le bloc d'un fichier : ses paires clé / valeur, jusqu'à son « } » (un bloc imbriqué est sauté).
                let mut f = FichierNuage { chemin: nom.clone(), racine: String::new(), taille: None, sha: String::new(), en_attente: false };
                let mut k = i + 2;
                let mut p = 1usize;
                while k < j.len() && p > 0 {
                    match (&j[k], j.get(k + 1)) {
                        (Jeton::Ouvre, _) => p += 1,
                        (Jeton::Ferme, _) => p -= 1,
                        (Jeton::Texte(cle), Some(Jeton::Texte(v))) if p == 1 => {
                            match cle.to_lowercase().as_str() {
                                "root" => f.racine = v.clone(),
                                "size" => f.taille = v.parse().ok(),
                                "sha" => f.sha = v.to_lowercase(),
                                "syncstate" => f.en_attente = v != "1",
                                _ => {}
                            }
                            k += 1;
                        }
                        _ => {}
                    }
                    k += 1;
                }
                l.push(f);
                i = k;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    l
}

/// Les dossiers de Windows où l'Auto-Cloud de Steam peut ranger des parties, lus dans le registre (« User Shell
/// Folders », développés) : (nom dans l'abri, dossier). Mes documents d'abord.
pub fn dossiers_windows() -> Vec<(&'static str, PathBuf)> {
    const CLE: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders";
    let mut l = Vec::new();
    for (nom, valeur) in [
        ("Documents", "Personal"),
        ("AppData-Local", "Local AppData"),
        ("AppData-Roaming", "AppData"),
        ("AppData-LocalLow", "{A520A1A4-1780-4FF6-BD18-167343C5AF16}"),
        ("Parties-enregistrees", "{4C5C32FF-BB9D-43B0-B5B4-2D72E54EAAA4}"),
    ] {
        if let Some(d) = crate::registre::lire_texte_hkcu(CLE, valeur).map(PathBuf::from).filter(|d| d.is_dir()) {
            l.push((nom, d));
        }
    }
    // LocalLow et « Parties enregistrées » ne sont souvent pas notés dans le registre (relevé du 21/10) : leur place
    // habituelle, s'ils existent.
    if let Some(u) = std::env::var_os("USERPROFILE").map(PathBuf::from) {
        for (nom, d) in [("AppData-LocalLow", u.join("AppData").join("LocalLow")), ("Parties-enregistrees", u.join("Saved Games"))] {
            if !l.iter().any(|(n, _)| *n == nom) && d.is_dir() {
                l.push((nom, d));
            }
        }
    }
    l
}

/// Un fichier « en ligne seulement » (OneDrive…) : le lire le téléchargerait.
fn en_ligne_seulement(m: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // FILE_ATTRIBUTE_OFFLINE, RECALL_ON_OPEN, RECALL_ON_DATA_ACCESS.
        m.file_attributes() & (0x1000 | 0x4_0000 | 0x40_0000) != 0
    }
    #[cfg(not(windows))]
    {
        let _ = m;
        false
    }
}

/// Le SHA1 d'un fichier, lu par morceaux (jamais en entier en mémoire).
fn sha1(f: &Path) -> std::io::Result<String> {
    use sha1::{Digest, Sha1};
    let mut r = std::fs::File::open(f)?;
    let mut h = Sha1::new();
    let mut t = vec![0u8; 256 * 1024];
    loop {
        let n = r.read(&mut t)?;
        if n == 0 {
            break;
        }
        h.update(&t[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Un fichier à copier : sa source, sa place dans l'abri (`nuage-steam/<dossier>/<chemin>`), sa taille, son SHA1.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AMettre {
    #[serde(skip)]
    pub source: PathBuf,
    pub relatif: String,
    pub taille: u64,
    pub sha: String,
}

/// Ce que Frogtend mettrait à l'abri, et ce qu'il laisse (dit à la personne).
#[derive(Debug, Clone, Default, Serialize)]
pub struct Plan {
    pub fichiers: Vec<AMettre>,
    pub octets: u64,
    /// Décrits par Steam mais introuvables tels quels sur ce PC (ou à une racine qui n'est pas une partie).
    pub introuvables: usize,
    /// Dans OneDrive « en ligne seulement » : pas lus.
    pub en_ligne_seulement: usize,
    /// Steam n'a pas fini de les synchroniser.
    pub en_attente: usize,
    /// Plus récents que ce que Steam a noté (une partie jouée depuis sa dernière synchronisation) : copiés tels quels.
    pub modifies: usize,
}

/// Ce qu'on trouve au chemin d'un fichier décrit par Steam, dans un dossier.
enum Trouve {
    /// Le fichier tel que Steam le décrit (taille, SHA1).
    Exact(PathBuf, u64, String),
    /// Un fichier à cet endroit, mais différent (plus récent que la dernière synchronisation ?).
    Autre(PathBuf, u64),
    /// Seulement dans un nuage (OneDrive…) : jamais lu.
    EnLigne,
    Absent,
}

fn retrouver(base: &Path, rel: &str, f: &FichierNuage) -> Trouve {
    let p = base.join(rel);
    let Ok(m) = std::fs::symlink_metadata(&p) else { return Trouve::Absent };
    if !m.is_file() {
        return Trouve::Absent;
    }
    if en_ligne_seulement(&m) {
        return Trouve::EnLigne;
    }
    if f.taille.is_some_and(|t| t != m.len()) {
        return Trouve::Autre(p, m.len());
    }
    match sha1(&p) {
        Ok(sha) if f.sha.is_empty() || f.sha == sha => Trouve::Exact(p, m.len(), sha),
        Ok(_) => Trouve::Autre(p, m.len()),
        Err(_) => Trouve::Absent,
    }
}

/// Le plan d'un jeu : `app` = `userdata\<compte>\<appid>`, `marque` = celle du compte (`marque`) ; `None` si Steam
/// ne garde rien de ce jeu.
pub fn planifier(app: &Path, marque: &str, windows: &[(&'static str, PathBuf)]) -> Resultat<Option<Plan>> {
    let remote = app.join("remote");
    let vdf = std::fs::read(app.join("remotecache.vdf")).ok().map(|o| String::from_utf8_lossy(&o).into_owned());
    if vdf.is_none() && !remote.is_dir() {
        return Ok(None);
    }
    let mut plan = Plan::default();
    let mut vus = std::collections::BTreeSet::new();
    for f in vdf.as_deref().map(lire_remotecache).unwrap_or_default() {
        // Racine 1 : les configurations de manette de Steam (relevé du 21/10), pas des parties.
        if f.racine == "1" {
            continue;
        }
        if f.en_attente {
            plan.en_attente += 1;
        }
        // Un chemin qui sortirait de son dossier (« .. », absolu) n'est jamais suivi.
        let Some(rel) = crate::decompression::chemin_sur(&f.chemin) else {
            plan.introuvables += 1;
            continue;
        };
        // Racine 0 : `remote\` ; sinon les dossiers de Windows, chacun vérifié (taille + SHA1), jamais deviné.
        let bases: Vec<(&str, &Path)> = if f.racine == "0" { vec![("remote", remote.as_path())] } else { windows.iter().map(|(n, d)| (*n, d.as_path())).collect() };
        let mut exact = None;
        let mut autres = Vec::new();
        let mut nuage = false;
        for (nom, base) in bases {
            match retrouver(base, &rel, &f) {
                Trouve::Exact(p, t, sha) => {
                    exact = Some((nom, p, t, sha));
                    break;
                }
                Trouve::Autre(p, t) => autres.push((nom, p, t)),
                Trouve::EnLigne => nuage = true,
                Trouve::Absent => {}
            }
        }
        // Tel que Steam le décrit ; sinon, s'il n'est qu'à UN endroit, la partie plus récente que Steam n'a pas encore
        // synchronisée (jamais perdue) ; à plusieurs endroits différents, on ne devine pas.
        let choisi = match (exact, autres.as_slice()) {
            (Some(x), _) => Some(x),
            (None, [(nom, p, t)]) => match sha1(p) {
                Ok(sha) => {
                    plan.modifies += 1;
                    Some((*nom, p.clone(), *t, sha))
                }
                Err(_) => None,
            },
            _ => None,
        };
        match choisi {
            Some((nom, source, taille, sha)) => {
                let relatif = format!("nuage-steam/{marque}/{nom}/{rel}");
                if vus.insert(relatif.to_lowercase()) {
                    plan.octets += taille;
                    plan.fichiers.push(AMettre { source, relatif, taille, sha });
                }
            }
            None if nuage => plan.en_ligne_seulement += 1,
            None => plan.introuvables += 1,
        }
    }
    // Ce qui est dans `remote\` sans être décrit (Steam peut ne pas avoir tout noté) : copié aussi.
    if remote.is_dir() {
        for r in fichiers_sans_lien(&remote)? {
            let relatif = format!("nuage-steam/{marque}/remote/{r}");
            if vus.contains(&relatif.to_lowercase()) {
                continue;
            }
            let p = remote.join(&r);
            let Ok(m) = std::fs::symlink_metadata(&p) else { continue };
            if !m.is_file() || en_ligne_seulement(&m) {
                continue;
            }
            let sha = sha1(&p)?;
            vus.insert(relatif.to_lowercase());
            plan.octets += m.len();
            plan.fichiers.push(AMettre { source: p, relatif, taille: m.len(), sha });
        }
    }
    plan.fichiers.sort_by(|a, b| a.relatif.cmp(&b.relatif));
    Ok(Some(plan))
}

/// La liste rangée dans chaque abri du nuage Steam, par compte (pour ne pas refaire un abri identique).
fn liste(marque: &str) -> String {
    format!("nuage-steam/{marque}/abri.json")
}

/// Le plus récent abri de ce jeu POUR CE COMPTE contient-il déjà exactement ces fichiers (chemins et SHA1) ?
pub fn deja_a_l_abri(abris_du_jeu: &Path, marque: &str, plan: &Plan) -> bool {
    let mut dates: Vec<(u64, PathBuf)> = std::fs::read_dir(abris_du_jeu)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().to_str().and_then(|n| n.parse::<u64>().ok()).map(|n| (n, e.path())))
        .collect();
    dates.sort_by(|a, b| b.0.cmp(&a.0));
    let Some(o) = dates.iter().find_map(|(_, d)| std::fs::read(d.join(liste(marque))).ok()) else { return false };
    let Ok(avant) = serde_json::from_slice::<Vec<AMettre>>(&o) else { return false };
    let cle = |l: &[AMettre]| l.iter().map(|f| (f.relatif.to_lowercase(), f.sha.clone())).collect::<std::collections::BTreeSet<_>>();
    cle(&avant) == cle(&plan.fichiers)
}

/// Copie le plan dans un nouvel abri `<abris>/<id>/<quand>/`, en passant par un dossier provisoire hors des abris
/// lus (un abri à moitié fait n'est jamais pris pour une sauvegarde). Chaque copie est vérifiée par son SHA1 ; un
/// fichier qui change pendant la copie (Steam synchronise) est recopié une fois, sinon tout est refusé.
pub fn abriter(plan: &Plan, marque: &str, abris: &Path, id: i64, quand: &str) -> Resultat<PathBuf> {
    let fin = abris.join(id.to_string()).join(quand);
    if fin.exists() {
        return Err(Erreur::Refus("Un abri de ce jeu vient d'être fait : réessaie dans une seconde.".into()));
    }
    let tmp = abris.join(".frogtend-en-cours").join(format!("{id}-{quand}"));
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp)?;
    }
    let r = (|| -> Resultat<()> {
        for f in &plan.fichiers {
            let cible = tmp.join(&f.relatif);
            if let Some(p) = cible.parent() {
                std::fs::create_dir_all(p)?;
            }
            let mut bon = false;
            for _ in 0..2 {
                std::fs::copy(&f.source, &cible)?;
                if sha1(&cible)? == f.sha {
                    bon = true;
                    break;
                }
            }
            if !bon {
                return Err(Erreur::Disque(format!(
                    "« {} » a changé pendant la copie (Steam synchronise peut-être) : rien n'est mis à l'abri, réessaie dans un moment.",
                    f.relatif
                )));
            }
        }
        let l = tmp.join(liste(marque));
        if let Some(p) = l.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::write(l, serde_json::to_vec_pretty(&plan.fichiers).unwrap_or_default())?;
        if let Some(p) = fin.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::rename(&tmp, &fin)?;
        Ok(())
    })();
    if r.is_err() {
        // Le dossier provisoire est le nôtre (son nom le dit) : rien d'autre n'est touché.
        let _ = std::fs::remove_dir_all(&tmp);
    }
    let _ = std::fs::remove_dir(abris.join(".frogtend-en-cours")); // seulement s'il est vide
    r?;
    Ok(fin)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La forme réelle de `remotecache.vdf` (relevé du 21/10, valeurs changées).
    const VDF: &str = "\"1466860\"\n{\n\t\"ChangeNumber\"\t\t\"7\"\n\t\"OSType\"\t\t\"0\"\n\t\"campaign\"\n\t{\n\t\t\"root\"\t\t\"0\"\n\t\t\"size\"\t\t\"5\"\n\t\t\"localtime\"\t\t\"1\"\n\t\t\"sha\"\t\t\"SHA\"\n\t\t\"syncstate\"\t\t\"1\"\n\t}\n\t\"my games/Jeu/SaveData/save_1\"\n\t{\n\t\t\"root\"\t\t\"2\"\n\t\t\"size\"\t\t\"3\"\n\t\t\"sha\"\t\t\"SHA2\"\n\t\t\"syncstate\"\t\t\"3\"\n\t}\n}\n";

    fn sha_de(o: &[u8]) -> String {
        use sha1::{Digest, Sha1};
        Sha1::digest(o).iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn un_steamid64_donne_son_dossier_userdata() {
        assert_eq!(compte("76561198000000000").as_deref(), Some("39734272"));
        assert_eq!(compte("abc"), None);
        assert_eq!(compte("76561197960265728"), None, "la base seule n'est pas un compte");
        assert_eq!(compte("1"), None);
    }

    #[test]
    fn remotecache_se_lit_meme_tronque() {
        let l = lire_remotecache(VDF);
        assert_eq!(l.len(), 2);
        assert_eq!((l[0].chemin.as_str(), l[0].racine.as_str(), l[0].taille, l[0].sha.as_str(), l[0].en_attente), ("campaign", "0", Some(5), "sha", false));
        assert!(l[1].en_attente, "syncstate 3 : pas fini");
        assert!(lire_remotecache("").is_empty());
        assert_eq!(lire_remotecache(&VDF[..VDF.len() / 2]).len(), 1, "tronqué : ce qui a pu être lu");
        assert_eq!(lire_remotecache("\"a\" { \"b\\\\c\" { \"root\" \"0\" } }")[0].chemin, "b\\c");
    }

    #[test]
    fn le_plan_retrouve_chaque_fichier_par_son_sha1_et_ne_devine_jamais() {
        let d = tempfile::tempdir().unwrap();
        let app = d.path().join("userdata").join("1").join("42");
        let (docs, local) = (d.path().join("OneDrive-Documents"), d.path().join("Local"));
        std::fs::create_dir_all(app.join("remote")).unwrap();
        std::fs::create_dir_all(docs.join("my games/Jeu/SaveData")).unwrap();
        std::fs::create_dir_all(local.join("my games/Jeu/SaveData")).unwrap();
        std::fs::write(app.join("remote/campaign"), b"12345").unwrap();
        std::fs::write(app.join("remote/en-plus"), b"x").unwrap();
        // Le même chemin dans deux dossiers : seul celui qui a le bon SHA1 est pris.
        std::fs::write(local.join("my games/Jeu/SaveData/save_1"), b"zzz").unwrap();
        std::fs::write(docs.join("my games/Jeu/SaveData/save_1"), b"abc").unwrap();
        let vdf = VDF.replace("\"SHA\"", &format!("\"{}\"", sha_de(b"12345"))).replace("\"SHA2\"", &format!("\"{}\"", sha_de(b"abc")));
        let vdf = vdf.replace("\t}\n}\n", "\t}\n\t\"../../Windows/win.ini\"\n\t{\n\t\t\"root\"\t\t\"2\"\n\t}\n\t\"absent\"\n\t{\n\t\t\"root\"\t\t\"3\"\n\t}\n}\n");
        // Une partie jouée depuis la dernière synchronisation (SHA1 différent), à UN seul endroit : gardée, jamais perdue.
        let vdf = vdf.replace("}\n}\n", "}\n\t\"my games/Jeu/SaveData/save_2\"\n\t{\n\t\t\"root\"\t\t\"2\"\n\t\t\"sha\"\t\t\"0000\"\n\t}\n\t\"manette.vdf\"\n\t{\n\t\t\"root\"\t\t\"1\"\n\t}\n}\n");
        std::fs::write(docs.join("my games/Jeu/SaveData/save_2"), b"nouveau").unwrap();
        std::fs::write(app.join("remotecache.vdf"), vdf).unwrap();
        // Une jonction (ici un lien de dossier) dans `remote\` n'est jamais suivie.
        #[cfg(windows)]
        let _ = std::os::windows::fs::symlink_dir(&local, app.join("remote").join("lien"));
        let windows = vec![("AppData-Local", local.clone()), ("Documents", docs.clone())];
        let p = planifier(&app, "m1", &windows).unwrap().unwrap();
        let rel: Vec<&str> = p.fichiers.iter().map(|f| f.relatif.as_str()).collect();
        assert_eq!(
            rel,
            vec![
                "nuage-steam/m1/Documents/my games/Jeu/SaveData/save_1",
                "nuage-steam/m1/Documents/my games/Jeu/SaveData/save_2",
                "nuage-steam/m1/remote/campaign",
                "nuage-steam/m1/remote/en-plus"
            ]
        );
        assert_eq!((p.octets, p.introuvables, p.en_attente, p.modifies), (16, 2, 1, 1), "racine 1 (manette) sautée");

        // L'abri : tout y est, vérifié ; un second abri identique n'est pas utile.
        let abris = d.path().join("Abris");
        let fin = abriter(&p, "m1", &abris, -7, "100").unwrap();
        assert_eq!(std::fs::read(fin.join("nuage-steam/m1/Documents/my games/Jeu/SaveData/save_1")).unwrap(), b"abc");
        assert!(!abris.join(".frogtend-en-cours").exists());
        assert!(deja_a_l_abri(&abris.join("-7"), "m1", &p));
        std::fs::write(app.join("remote/campaign"), b"54321").unwrap();
        let p2 = planifier(&app, "m1", &windows).unwrap().unwrap();
        assert!(!deja_a_l_abri(&abris.join("-7"), "m1", &p2), "une partie a changé");
        // La source n'a jamais été touchée.
        assert_eq!(std::fs::read(docs.join("my games/Jeu/SaveData/save_1")).unwrap(), b"abc");
    }

    #[test]
    fn un_fichier_qui_change_pendant_la_copie_ne_fait_pas_un_faux_abri() {
        let d = tempfile::tempdir().unwrap();
        let s = d.path().join("s");
        std::fs::write(&s, b"neuf").unwrap();
        let plan = Plan { fichiers: vec![AMettre { source: s, relatif: "nuage-steam/remote/s".into(), taille: 4, sha: sha_de(b"ancien") }], octets: 4, ..Default::default() };
        let abris = d.path().join("Abris");
        assert!(abriter(&plan, "m1", &abris, -7, "100").is_err());
        assert!(!abris.join("-7").exists() && !abris.join(".frogtend-en-cours").exists());
    }

    #[test]
    fn un_jeu_sans_nuage_n_a_pas_de_plan() {
        let d = tempfile::tempdir().unwrap();
        assert!(planifier(d.path(), "m1", &[]).unwrap().is_none());
    }

    /// RÉEL, lecture seule : le plan d'un jeu Steam de ce PC (rien n'est copié ; aucun chemin ni compte affiché).
    /// `FROGTEND_STEAM_APP=268500 cargo test essai_nuage_steam -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn essai_nuage_steam() {
        let app = std::env::var("FROGTEND_STEAM_APP").unwrap();
        let steam = crate::boutiques::dossier_steam().expect("Steam");
        let comptes: Vec<PathBuf> = std::fs::read_dir(steam.join("userdata")).unwrap().flatten().map(|e| e.path()).collect();
        assert_eq!(comptes.len(), 1, "un seul compte attendu sur ce PC");
        let w = dossiers_windows();
        println!("dossiers de Windows : {:?}", w.iter().map(|(n, _)| *n).collect::<Vec<_>>());
        let p = planifier(&comptes[0].join(&app), "essai", &w).unwrap();
        // L'abri complet, dans un dossier temporaire (effacé à la fin) : chaque copie vérifiée par son SHA1.
        if let Some(p) = &p {
            let t = tempfile::tempdir().unwrap();
            let fin = abriter(p, "essai", t.path(), -1, "1").unwrap();
            let copies = crate::installation::fichiers_de(&fin).unwrap().len();
            println!("abri d'essai : {copies} fichiers (liste comprise), déjà à l'abri ensuite : {}", deja_a_l_abri(&t.path().join("-1"), "essai", p));
        }
        match p {
            None => println!("{app} : rien dans le nuage"),
            Some(p) => println!(
                "{app} : {} fichiers, {} octets ; introuvables {}, en ligne seulement {}, en attente {} ; dossiers : {:?}",
                p.fichiers.len(),
                p.octets,
                p.introuvables,
                p.en_ligne_seulement,
                p.en_attente,
                p.fichiers.iter().map(|f| f.relatif.split('/').nth(1).unwrap_or("")).collect::<std::collections::BTreeSet<_>>()
            ),
        }
    }
}
