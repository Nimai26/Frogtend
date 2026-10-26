//! Vita3K (PS Vita) : un UTILISATEUR Vita par personne (décision de Seb, 26/10 : « Séparer »). Relevé dans le code
//! source de Vita3K (commit 9a2da65, 26/10/2026) :
//! - portable : dossier `portable\` à côté du programme ; la Vita dans `portable\fs\`, les réglages dans
//!   `portable\config.yml` (app_init.cpp L270-295) ;
//! - un utilisateur = `ux0\user\<id 2 chiffres>\user.xml` (user.cpp L114-135), id = premier libre à partir de 00 ;
//!   un dossier sans user.xml lisible est ignoré ;
//! - l'utilisateur de la partie = clé `user-id` de config.yml (config.h L192) ; INCONNU, Vita3K prend le premier EN
//!   SILENCE et réécrit config.yml (app.cpp L185-208) : Frogtend crée l'utilisateur AVANT et règle `user-id`, Vita3K
//!   fermé (il réécrit config.yml à chaque démarrage, en gardant les clés qu'il connaît) ;
//! - parties : `ux0\user\<id>\savedata\<dossier du jeu>\` (io.cpp L158-174) — tout `savedata` est sauvegardé ;
//! - jamais `-d` (il efface en dur les parties de l'utilisateur 00, main.cpp L132-138).

use crate::erreurs::{Erreur, Resultat};
use std::path::{Path, PathBuf};

/// Dans une sauvegarde, le dossier (sous celui de l'émulateur) des parties de l'utilisateur Vita du profil.
pub const UTILISATEUR_VITA: &str = ".vita3k-utilisateur";

/// L'utilisateur Vita d'un profil, prêt pour une partie.
#[derive(Debug, Clone, PartialEq)]
pub struct UtilisateurDuProfil {
    pub id: String,
    pub cree: bool,
    /// Créé à l'instant, et première création par Frogtend sur ce Vita3K : l'utilisateur qu'il prenait jusque-là.
    pub ancien_courant: Option<String>,
}

pub fn est_vita3k(programme: &str) -> bool {
    Path::new(programme).file_name().is_some_and(|n| n.to_string_lossy().to_ascii_lowercase().starts_with("vita3k"))
}

/// Vita3K installé en portable (comme Frogtend l'installe) ? Sinon, ses données sont dans `%APPDATA%` : jamais touché.
pub fn portable(v: &Path) -> bool {
    v.join("portable").is_dir()
}

fn ux0(v: &Path) -> PathBuf {
    v.join("portable").join("fs").join("ux0")
}

fn config(v: &Path) -> PathBuf {
    v.join("portable").join("config.yml")
}

/// La valeur d'un attribut XML simple (`nom="valeur"`) dans `texte`.
fn attribut(texte: &str, nom: &str) -> Option<String> {
    let i = texte.find(&format!(" {nom}=\""))? + nom.len() + 3;
    let j = texte[i..].find('"')? + i;
    Some(texte[i..j].replace("&quot;", "\"").replace("&lt;", "<").replace("&gt;", ">").replace("&apos;", "'").replace("&amp;", "&"))
}

fn echapper(t: &str) -> String {
    t.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;").replace('\'', "&apos;")
}

/// Les utilisateurs : (id, nom), par id. Seulement ceux que Vita3K reconnaîtrait (user.xml lisible, id = dossier).
pub fn utilisateurs(v: &Path) -> Vec<(String, String)> {
    let mut l: Vec<(String, String)> = std::fs::read_dir(ux0(v).join("user"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let d = e.file_name().to_string_lossy().to_string();
            let x = std::fs::read_to_string(e.path().join("user.xml")).ok()?;
            let balise = &x[x.find("<user")?..];
            let id = attribut(balise, "id").unwrap_or_else(|| d.clone());
            (id == d).then(|| (id, attribut(balise, "name").unwrap_or_default()))
        })
        .collect();
    l.sort();
    l
}

/// Le `user.xml` d'un utilisateur, au format exact de Vita3K (user.cpp `save_user`).
pub fn user_xml(id: &str, nom: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<user id=\"{id}\" name=\"{}\">\n\t<theme use-background=\"true\">\n\t\t<content-id>default</content-id>\n\t</theme>\n\t<start-screen type=\"default\">\n\t\t<path></path>\n\t</start-screen>\n\t<backgrounds />\n</user>\n",
        echapper(nom)
    )
}

/// La valeur de `user-id` dans config.yml (`user-id: 01` ou `user-id: "01"`).
fn user_id_regle(v: &Path) -> Option<String> {
    std::fs::read_to_string(config(v))
        .ok()?
        .lines()
        .find_map(|l| l.strip_prefix("user-id:").map(|x| x.split(" #").next().unwrap_or("").trim().trim_matches(['"', '\'']).to_string()))
        .filter(|x| !x.is_empty())
}

/// La table profil → utilisateur que Frogtend tient pour ce Vita3K (`Profils\.utilisateurs-frogtend`).
fn table(v: &Path) -> Vec<(String, String)> {
    std::fs::read_to_string(v.join("Profils").join(".utilisateurs-frogtend"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once('\t').map(|(i, p)| (i.to_string(), p.to_string())))
        .collect()
}

fn noter(v: &Path, id: &str, profil: &str) -> Resultat<()> {
    let mut t = table(v);
    t.retain(|(_, p)| p != profil);
    t.push((id.to_string(), profil.to_string()));
    std::fs::create_dir_all(v.join("Profils"))?;
    std::fs::write(v.join("Profils").join(".utilisateurs-frogtend"), t.iter().map(|(i, p)| format!("{i}\t{p}\n")).collect::<String>())?;
    Ok(())
}

fn trouver(v: &Path, l: &[(String, String)], profil: &str) -> Option<String> {
    let t = table(v);
    if let Some((id, _)) = t.iter().find(|(i, p)| p == profil && l.iter().any(|(x, _)| x == i)) {
        return Some(id.clone());
    }
    l.iter().find(|(i, n)| n == profil && !t.iter().any(|(x, p)| x == i && p != profil)).map(|(i, _)| i.clone())
}

/// L'utilisateur Vita de ce profil, créé s'il manque (Vita3K FERMÉ : l'appelant le vérifie). Seul un dossier
/// d'utilisateur NEUF est écrit (user.xml provisoire puis renommé, relu).
pub fn utilisateur_du_profil(v: &Path, profil: &str) -> Resultat<UtilisateurDuProfil> {
    let l = utilisateurs(v);
    if let Some(id) = trouver(v, &l, profil) {
        noter(v, &id, profil)?;
        return Ok(UtilisateurDuProfil { id, cree: false, ancien_courant: None });
    }
    // Le premier id libre (2 chiffres), jamais un dossier déjà là (même illisible).
    let id = (0..100).map(|n| format!("{n:02}")).find(|i| !ux0(v).join("user").join(i).exists()).ok_or_else(|| {
        Erreur::Prevenir("Vita3K a déjà 100 utilisateurs : Frogtend ne peut pas en créer un pour ce profil.".into())
    })?;
    let dossier = ux0(v).join("user").join(&id);
    std::fs::create_dir_all(&dossier)?;
    let provisoire = dossier.join("user.xml.frogtend");
    std::fs::write(&provisoire, user_xml(&id, profil))?;
    std::fs::rename(&provisoire, dossier.join("user.xml"))?;
    if !utilisateurs(v).iter().any(|(i, n)| *i == id && n == profil) {
        return Err(Erreur::Disque("L'utilisateur Vita3K de ce profil n'a pas pu être enregistré.".into()));
    }
    noter(v, &id, profil)?;
    // La reprise des parties d'avant Frogtend : seulement à la toute première création par Frogtend sur ce Vita3K.
    let marque = v.join("Profils").join(".reprise-vita3k");
    let ancien_courant = if marque.exists() {
        None
    } else {
        std::fs::write(&marque, profil)?;
        user_id_regle(v).filter(|r| l.iter().any(|(i, _)| i == r)).or_else(|| l.first().map(|(i, _)| i.clone()))
    };
    Ok(UtilisateurDuProfil { id, cree: true, ancien_courant })
}

/// Règle l'utilisateur de la prochaine partie (`user-id` de config.yml), Vita3K FERMÉ. Seule cette ligne change
/// (ajoutée si elle manque) ; l'ancien fichier est copié à l'abri avant ; écrit par un provisoire renommé, puis relu.
pub fn choisir(v: &Path, id: &str) -> Resultat<()> {
    if user_id_regle(v).as_deref() == Some(id) {
        return Ok(());
    }
    let f = config(v);
    let avant = match std::fs::read_to_string(&f) {
        Ok(t) => Some(t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => return Err(Erreur::Prevenir("Les réglages de Vita3K sont illisibles : Frogtend n'y touche pas.".into())),
    };
    let ligne = format!("user-id: \"{id}\"");
    let apres = match &avant {
        Some(t) if t.lines().any(|l| l.starts_with("user-id:")) => {
            let fin = if t.contains("\r\n") { "\r\n" } else { "\n" };
            let mut r: String = t.lines().map(|l| if l.starts_with("user-id:") { ligne.clone() } else { l.to_string() }).collect::<Vec<_>>().join(fin);
            if t.ends_with('\n') {
                r.push_str(fin);
            }
            r
        }
        Some(t) => format!("{}{}{ligne}\n", t, if t.is_empty() || t.ends_with('\n') { "" } else { "\n" }),
        None => format!("---\n{ligne}\n"),
    };
    if let Some(a) = &avant {
        let base = v.join(".frogtend-sauvegardes");
        std::fs::create_dir_all(&base)?;
        let nom = format!("{}-config.yml", crate::noyau::maintenant());
        let mut abri = base.join(&nom);
        let mut n = 2;
        while abri.exists() {
            abri = base.join(format!("{nom}-{n}"));
            n += 1;
        }
        std::fs::write(&abri, a)?;
    }
    let provisoire = f.with_extension("yml.frogtend");
    std::fs::write(&provisoire, &apres)?;
    std::fs::rename(&provisoire, &f)?;
    if user_id_regle(v).as_deref() != Some(id) {
        return Err(Erreur::Disque("L'utilisateur de Vita3K n'a pas pu être réglé.".into()));
    }
    Ok(())
}

/// L'utilisateur Vita de ce profil s'il existe, SANS rien créer (pour la sauvegarde).
pub fn utilisateur_existant(v: &Path, profil: &str) -> Option<String> {
    trouver(v, &utilisateurs(v), profil)
}

/// Le dossier des parties d'un utilisateur.
pub fn parties(v: &Path, id: &str) -> PathBuf {
    ux0(v).join("user").join(id).join("savedata")
}

/// Un `param.sfo` lu (1 Mo au plus).
fn lire_sfo(f: &Path) -> Option<Vec<u8>> {
    (std::fs::metadata(f).ok()?.len() <= 1 << 20).then(|| std::fs::read(f).ok()).flatten()
}

/// Le `sce_sys\param.sfo` d'un jeu PS Vita : dans son dossier, ou dans son .vpk/.zip — UN SEUL jeu (un zip qui en
/// contient plusieurs, de numéros différents : aucun, on ne devine pas lequel).
fn sfo_du_jeu(jeu: &Path) -> Option<Vec<u8>> {
    use std::io::Read;
    if jeu.is_dir() {
        return lire_sfo(&jeu.join("sce_sys").join("param.sfo"));
    }
    let mut z = zip::ZipArchive::new(std::fs::File::open(jeu).ok()?).ok()?;
    let noms: Vec<String> = z
        .file_names()
        .filter(|n| {
            let n = n.replace('\\', "/").to_ascii_lowercase();
            n == "sce_sys/param.sfo" || n.ends_with("/sce_sys/param.sfo")
        })
        .map(String::from)
        .collect();
    let mut sfos = Vec::new();
    for nom in noms {
        let f = z.by_name(&nom).ok()?;
        let mut o = Vec::new();
        f.take(1 << 20).read_to_end(&mut o).ok()?;
        sfos.push(o);
    }
    let mut ids: Vec<String> = sfos.iter().filter_map(|s| crate::contenus::valeur_sfo(s, "TITLE_ID")).collect();
    ids.sort();
    ids.dedup();
    (ids.len() == 1).then(|| sfos.into_iter().next()).flatten()
}

fn id_valide(s: &[u8]) -> Option<String> {
    crate::contenus::valeur_sfo(s, "TITLE_ID").filter(|t| t.len() == 9 && t.chars().all(|c| c.is_ascii_alphanumeric()))
}

/// Le numéro d'un jeu PS Vita (TITLE_ID), seulement 9 lettres et chiffres (il devient un nom de dossier).
pub fn titre_id(jeu: &Path) -> Option<String> {
    id_valide(&sfo_du_jeu(jeu)?)
}

/// Ce jeu peut-il se lancer par son numéro, sans repasser son fichier ? Seulement s'il est installé dans ce Vita3K À
/// LA MÊME VERSION (`APP_VER` du fichier = celle de `ux0\patch\<id>` s'il y en a une, sinon de `ux0\app\<id>`) :
/// une autre version (mise à jour, autre sortie) repasse par le fichier et s'installe, comme avant.
pub fn pret_a_lancer(v: &Path, jeu: &Path) -> Option<String> {
    let s = sfo_du_jeu(jeu)?;
    let id = id_valide(&s)?;
    let version = crate::contenus::valeur_sfo(&s, "APP_VER");
    let installe = ["patch", "app"].iter().find_map(|d| lire_sfo(&ux0(v).join(d).join(&id).join("sce_sys").join("param.sfo")))?;
    (crate::contenus::valeur_sfo(&installe, "APP_VER") == version).then_some(id)
}

/// Un jeu déjà installé se lance par son numéro (`-r <TITLE_ID>`, config.cpp L367) : l'argument qui EST le fichier du
/// jeu est remplacé — Vita3K ne le réinstalle plus. Sans un tel argument (ligne réglée à la main), rien ne change.
pub fn lancer_par_numero(arguments: &mut Vec<String>, jeu: &Path, titre: &str) {
    let j = jeu.to_string_lossy().to_string();
    if let Some(i) = arguments.iter().position(|a| *a == j) {
        arguments.splice(i..=i, ["-r".to_string(), titre.to_string()]);
    }
}

/// Décision de Seb (26/10) : les parties de l'utilisateur que Vita3K prenait AVANT Frogtend sont COPIÉES chez la
/// première personne qui joue (`u.ancien_courant`), jeu par jeu ; jamais déplacées, jamais par-dessus.
pub fn reprendre_parties_d_avant(v: &Path, u: &UtilisateurDuProfil) -> Resultat<usize> {
    let mut copies = 0;
    if let Some(source) = u.ancien_courant.as_deref().filter(|s| *s != u.id) {
        for e in std::fs::read_dir(parties(v, source)).into_iter().flatten().flatten() {
            let cible = parties(v, &u.id).join(e.file_name());
            if e.file_type().is_ok_and(|t| t.is_dir()) && !cible.exists() {
                crate::eden::copier_dossier_entier(&e.path(), &cible)?;
                copies += 1;
            }
        }
    }
    Ok(copies)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vita_portable() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("portable")).unwrap();
        d
    }

    #[test]
    fn le_fichier_d_un_utilisateur_a_la_forme_de_vita3k() {
        let x = user_xml("01", "Léa & \"Zoé\"");
        assert!(x.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<user id=\"01\" name=\"Léa &amp; &quot;Zoé&quot;\">"));
        assert_eq!(attribut(&x, "name").as_deref(), Some("Léa & \"Zoé\""));
    }

    #[test]
    fn chaque_profil_a_son_utilisateur_et_la_partie_choisit_le_sien() {
        let d = vita_portable();
        let v = d.path();
        // L'utilisateur que Vita3K crée seul (00), réglé dans config.yml avec d'autres réglages.
        std::fs::create_dir_all(ux0(v).join("user").join("00")).unwrap();
        std::fs::write(ux0(v).join("user").join("00").join("user.xml"), user_xml("00", "Vita3K")).unwrap();
        std::fs::write(config(v), "---\nboot-apps-full-screen: true\nuser-id: 00\nlog-level: 2\n").unwrap();
        let seb = utilisateur_du_profil(v, "Seb").unwrap();
        assert_eq!((seb.id.as_str(), seb.cree, seb.ancien_courant.as_deref()), ("01", true, Some("00")));
        let lea = utilisateur_du_profil(v, "Léa").unwrap();
        assert_eq!((lea.id.as_str(), lea.ancien_courant.clone()), ("02", None), "une seule reprise par Vita3K");
        assert_eq!(utilisateur_du_profil(v, "Seb").unwrap().id, "01");
        assert_eq!(utilisateur_existant(v, "Léa").as_deref(), Some("02"));
        // La partie de Léa : seule la ligne user-id change, l'ancien fichier à l'abri.
        choisir(v, "02").unwrap();
        assert_eq!(std::fs::read_to_string(config(v)).unwrap(), "---\nboot-apps-full-screen: true\nuser-id: \"02\"\nlog-level: 2\n");
        assert_eq!(std::fs::read_dir(v.join(".frogtend-sauvegardes")).unwrap().count(), 1);
        choisir(v, "02").unwrap();
        assert_eq!(std::fs::read_dir(v.join(".frogtend-sauvegardes")).unwrap().count(), 1, "déjà réglé : rien de réécrit");
        // Un commentaire après la valeur, des fins de ligne Windows : lus et gardés.
        std::fs::write(config(v), "---\r\nuser-id: 02 # Léa\r\nlog-level: 2\r\n").unwrap();
        assert_eq!(user_id_regle(v).as_deref(), Some("02"));
        choisir(v, "01").unwrap();
        assert_eq!(std::fs::read_to_string(config(v)).unwrap(), "---\r\nuser-id: \"01\"\r\nlog-level: 2\r\n");
        assert!(est_vita3k("C:\\Vita3K\\Vita3K.exe") && !est_vita3k("C:\\Cemu\\Cemu.exe"));
        // Sans la ligne : ajoutée.
        std::fs::write(config(v), "---\nlog-level: 2").unwrap();
        choisir(v, "01").unwrap();
        assert_eq!(std::fs::read_to_string(config(v)).unwrap(), "---\nlog-level: 2\nuser-id: \"01\"\n");
    }

    fn sfo(titre: &str) -> Vec<u8> {
        sfo_version(titre, None)
    }

    /// Un PARAM.SFO minimal : TITLE_ID, et APP_VER si donnée (clés texte).
    fn sfo_version(titre: &str, version: Option<&str>) -> Vec<u8> {
        let mut paires: Vec<(&str, String)> = vec![("TITLE_ID", format!("{titre}\0"))];
        if let Some(v) = version {
            paires.insert(0, ("APP_VER", format!("{v}\0")));
        }
        let mut cles: Vec<u8> = Vec::new();
        let mut donnees: Vec<u8> = Vec::new();
        let mut index = Vec::new();
        for (c, v) in &paires {
            index.extend((cles.len() as u16).to_le_bytes());
            index.extend(0x0204u16.to_le_bytes());
            index.extend((v.len() as u32).to_le_bytes());
            index.extend((v.len() as u32).to_le_bytes());
            index.extend((donnees.len() as u32).to_le_bytes());
            cles.extend(c.as_bytes());
            cles.push(0);
            donnees.extend(v.as_bytes());
        }
        while cles.len() % 4 != 0 {
            cles.push(0);
        }
        let debut_cles = 0x14 + index.len();
        let debut_donnees = debut_cles + cles.len();
        let mut o = Vec::new();
        o.extend(b"\0PSF");
        o.extend(0x0101u32.to_le_bytes());
        o.extend((debut_cles as u32).to_le_bytes());
        o.extend((debut_donnees as u32).to_le_bytes());
        o.extend((paires.len() as u32).to_le_bytes());
        o.extend(index);
        o.extend(cles);
        o.extend(donnees);
        o
    }

    fn zip_de(f: &Path, entrees: &[(&str, Vec<u8>)]) {
        let mut z = zip::ZipWriter::new(std::fs::File::create(f).unwrap());
        for (n, o) in entrees {
            z.start_file(*n, zip::write::SimpleFileOptions::default()).unwrap();
            std::io::Write::write_all(&mut z, o).unwrap();
        }
        z.finish().unwrap();
    }

    #[test]
    fn une_autre_version_ou_un_zip_a_plusieurs_jeux_repasse_par_le_fichier() {
        let d = vita_portable();
        let v = d.path();
        let app = ux0(v).join("app").join("PCSB00245").join("sce_sys");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(app.join("param.sfo"), sfo_version("PCSB00245", Some("01.00"))).unwrap();
        let maj = d.path().join("p4g 1.03.vpk");
        zip_de(&maj, &[("sce_sys/param.sfo", sfo_version("PCSB00245", Some("01.03")))]);
        assert_eq!(pret_a_lancer(v, &maj), None, "une autre version : installée par le fichier");
        // Installée (patch 1.03) : directement.
        let patch = ux0(v).join("patch").join("PCSB00245").join("sce_sys");
        std::fs::create_dir_all(&patch).unwrap();
        std::fs::write(patch.join("param.sfo"), sfo_version("PCSB00245", Some("01.03"))).unwrap();
        assert_eq!(pret_a_lancer(v, &maj).as_deref(), Some("PCSB00245"));
        // Un zip qui contient deux jeux : on ne devine pas.
        let deux = d.path().join("deux.zip");
        zip_de(&deux, &[("a/sce_sys/param.sfo", sfo("PCSB00245")), ("b/sce_sys/param.sfo", sfo("PCSE00120"))]);
        assert_eq!(titre_id(&deux), None);
        // `xsce_sys/param.sfo` n'est pas le param.sfo d'un jeu.
        let faux = d.path().join("faux.zip");
        zip_de(&faux, &[("xsce_sys/param.sfo", sfo("PCSB00245"))]);
        assert_eq!(titre_id(&faux), None);
    }

    #[test]
    fn un_jeu_deja_installe_se_lance_par_son_numero() {
        let d = vita_portable();
        let v = d.path();
        // Le .vpk (un zip) du jeu.
        let vpk = d.path().join("Persona 4 Golden (Europe).vpk");
        {
            let mut z = zip::ZipWriter::new(std::fs::File::create(&vpk).unwrap());
            z.start_file("sce_sys/param.sfo", zip::write::SimpleFileOptions::default()).unwrap();
            std::io::Write::write_all(&mut z, &sfo("PCSB00245")).unwrap();
            z.finish().unwrap();
        }
        assert_eq!(titre_id(&vpk).as_deref(), Some("PCSB00245"));
        assert_eq!(pret_a_lancer(v, &vpk), None, "pas encore installé");
        let app = ux0(v).join("app").join("PCSB00245").join("sce_sys");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(app.join("param.sfo"), sfo("PCSB00245")).unwrap();
        assert_eq!(pret_a_lancer(v, &vpk).as_deref(), Some("PCSB00245"));
        assert_eq!(titre_id(&ux0(v).join("app").join("PCSB00245")).as_deref(), Some("PCSB00245"), "un dossier aussi");
        let mut args = vec!["-F".to_string(), vpk.to_string_lossy().to_string()];
        lancer_par_numero(&mut args, &vpk, "PCSB00245");
        assert_eq!(args, ["-F", "-r", "PCSB00245"]);
        // Une ligne réglée à la main où le fichier est collé à autre chose : rien ne change.
        let mut args = vec![format!("--x={}", vpk.display())];
        lancer_par_numero(&mut args, &vpk, "PCSB00245");
        assert_eq!(args.len(), 1);
        // Un numéro qui n'en est pas un (il deviendrait un chemin) : aucun.
        std::fs::write(app.join("param.sfo"), sfo("../../x")).unwrap();
        assert_eq!(titre_id(&ux0(v).join("app").join("PCSB00245")), None);
    }

    #[test]
    fn les_parties_d_avant_vont_a_la_premiere_personne_et_jamais_par_dessus() {
        let d = vita_portable();
        let v = d.path();
        std::fs::create_dir_all(ux0(v).join("user").join("00")).unwrap();
        std::fs::write(ux0(v).join("user").join("00").join("user.xml"), user_xml("00", "Vita3K")).unwrap();
        for (jeu, o) in [("PCSB00001", &b"persona"[..]), ("PCSB00002", b"gravity")] {
            std::fs::create_dir_all(parties(v, "00").join(jeu)).unwrap();
            std::fs::write(parties(v, "00").join(jeu).join("sce_sys.bin"), o).unwrap();
        }
        let seb = utilisateur_du_profil(v, "Seb").unwrap();
        std::fs::create_dir_all(parties(v, &seb.id).join("PCSB00002")).unwrap();
        std::fs::write(parties(v, &seb.id).join("PCSB00002").join("sce_sys.bin"), b"gravity de Seb").unwrap();
        assert_eq!(reprendre_parties_d_avant(v, &seb).unwrap(), 1);
        assert_eq!(std::fs::read(parties(v, &seb.id).join("PCSB00001").join("sce_sys.bin")).unwrap(), b"persona");
        assert_eq!(std::fs::read(parties(v, &seb.id).join("PCSB00002").join("sce_sys.bin")).unwrap(), b"gravity de Seb");
        assert!(parties(v, "00").join("PCSB00001").join("sce_sys.bin").is_file(), "copiées, jamais déplacées");
    }

    #[test]
    fn apres_une_restauration_la_personne_suivante_ne_recoit_jamais_les_parties_d_une_autre() {
        let d = vita_portable();
        let v = d.path();
        let seb = utilisateur_du_profil(v, "Seb").unwrap();
        assert_eq!((seb.id.as_str(), seb.ancien_courant.clone()), ("00", None));
        std::fs::create_dir_all(parties(v, &seb.id).join("PCSB00001")).unwrap();
        choisir(v, &seb.id).unwrap();
        let lea = utilisateur_du_profil(v, "Léa").unwrap();
        assert_eq!(lea.ancien_courant, None);
        assert_eq!(reprendre_parties_d_avant(v, &lea).unwrap(), 0);
    }
}
