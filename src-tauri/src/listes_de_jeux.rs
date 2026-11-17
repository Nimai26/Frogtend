//! Les listes de jeux des émulateurs (✅ Seb 17/11) : les emplacements de la plateforme y sont AJOUTÉS, jamais
//! à la place de ceux de la personne (DuckStation, PCSX2 et Dolphin REMPLAÇAIENT leur liste à chaque partie : un
//! dossier ajouté par la personne disparaissait). Frogtend note ce qu'il a posé (`Profils\.frogtend-listes-de-jeux.json`,
//! dans le dossier de l'émulateur, par fichier de réglages) :
//! - un dossier noté puis absent de la liste a été RETIRÉ par la personne : il n'est jamais remis ;
//! - un dossier noté qui n'est plus un emplacement de la plateforme (emplacement changé) est retiré de la liste et de la
//!   note ; un dossier NON noté (de la personne, ou posé par un Frogtend d'avant la note) n'est jamais retiré ;
//! - une note illisible : rien n'est ajouté ni retiré (on ne saurait plus ce qui est à qui).
//!
//! Seuls les emplacements qui existent sont ajoutés ; l'émulateur ouvert, rien n'est écrit (il réécrit ses réglages en
//! quittant).
//!
//! Formats relus dans les sources officielles (17/11) :
//! - DuckStation (`settings.ini`, `[GameList]`) : une ligne par dossier, `RecursivePaths = X` (clé répétée, lue par
//!   `GetStringList`, src/core/game_list.cpp ; `Paths` = les dossiers non récursifs). Une valeur avec `#` ou `;` est
//!   écrite entre guillemets (sinon la fin serait lue comme un commentaire : `ValueNeedsQuoting` /
//!   `StripValueComment`, src/util/ini_settings_interface.cpp).
//! - PCSX2 (`inis\PCSX2.ini`, `[GameList]`) : la même forme (SimpleIni en clés multiples, `GetAllValues`,
//!   pcsx2/INISettingsInterface.cpp ; pcsx2/GameList.cpp lit `Paths` et `RecursivePaths`).
//! - Dolphin (`User\Config\Dolphin.ini`, `[General]`) : `ISOPaths = N` puis `ISOPath0` … `ISOPath{N-1}` ; une entrée
//!   vide est ignorée (Source/Core/Core/Config/MainSettings.cpp, `GetIsoPaths`). La recherche dans les sous-dossiers est
//!   UN réglage commun, `RecursiveISOPaths`, lui aussi dans `[General]` de Dolphin.ini (`MAIN_RECURSIVE_ISO_PATHS
//!   {System::Main, "General", "RecursiveISOPaths"}`, faux d'office, Source/Core/Core/Config/UISettings.cpp) : posé
//!   seulement si la personne n'a rien réglé, dès qu'un emplacement de Frogtend est dans la liste (les jeux de Frogtend
//!   sont rangés un dossier par jeu).
//! - Eden (`user\config\qt-config.ini`, `[UI]`, src/qt_common/config/qt_config.cpp v0.2.1, `ReadPathValues` /
//!   `SavePathValues`) : `Paths\gamedirs\size`, puis pour i = 1…size `Paths\gamedirs\<i>\path`, `deep_scan`
//!   (+ `deep_scan\default`), `expanded` (+ `expanded\default`) ; une clé `\default=true` fait ignorer la valeur. Une
//!   liste VIDE est recréée par Eden avec les entrées « SDMC », « UserNAND », « SysNAND » (et l'ancien
//!   `Paths\gameListRootDir`) ; une liste non vide ne l'est jamais : la liste qu'Eden voit vraiment (`lire`) les
//!   compte, et Frogtend les écrit lui-même avant les siens quand la liste écrite était vide. Un ajout ne réécrit jamais
//!   une entrée existante (il ajoute après).
//! - Ryubing (`Config.json`, `game_dirs` : un tableau de chaînes) : ryubing.rs, `completer_liste_de_jeux`.

use crate::contenus::meme_dossier;
use crate::erreurs::{Erreur, Resultat};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// La note de ce que Frogtend a posé dans les listes de jeux d'un émulateur (par fichier de réglages).
const NOTE: &str = ".frogtend-listes-de-jeux.json";

/// Les entrées spéciales qu'Eden met dans une liste vide (qt_config.cpp, `ReadPathValues`).
pub const SPECIALES_EDEN: [&str; 3] = ["SDMC", "UserNAND", "SysNAND"];

/// Le format d'un fichier de réglages qui porte une liste de jeux.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Format {
    DuckStation,
    Pcsx2,
    Dolphin,
    Eden,
}

/// Ce que la fusion décide.
#[derive(Debug, Default, PartialEq)]
pub struct Fusion {
    /// Les dossiers à AJOUTER à la fin de la liste de la personne (dans l'ordre des emplacements).
    pub ajoutes: Vec<String>,
    /// Posés par Frogtend, qui ne sont plus des emplacements de la plateforme : à RETIRER de la liste.
    pub a_retirer: Vec<String>,
    /// Posés par Frogtend autrefois, retirés depuis par la personne : jamais remis.
    pub retires_par_la_personne: Vec<String>,
    /// La nouvelle note : ce que Frogtend a posé (ou trouvé déjà là), à ne jamais remettre s'il disparaît.
    pub poses: Vec<String>,
}

/// Le bilan d'une liste complétée (pour le journal).
#[derive(Debug, Default, PartialEq)]
pub struct Bilan {
    pub ajoutes: Vec<String>,
    /// Posés par Frogtend et retirés par lui (ce n'est plus un emplacement de la plateforme).
    pub retires: Vec<String>,
    pub retires_par_la_personne: Vec<String>,
    /// L'émulateur était ouvert : rien n'a été écrit.
    pub ouvert: bool,
}

impl Bilan {
    /// Une ligne de journal, ou rien s'il n'y a rien à dire.
    pub fn resume(&self) -> Option<String> {
        let mut v = Vec::new();
        if self.ouvert {
            v.push("émulateur ouvert : liste de jeux pas touchée".to_string());
        }
        if !self.ajoutes.is_empty() {
            v.push(format!("ajouté(s) à la liste de jeux : {}", self.ajoutes.join(", ")));
        }
        if !self.retires.is_empty() {
            v.push(format!("retiré(s) de la liste de jeux (plus un emplacement) : {}", self.retires.join(", ")));
        }
        if !self.retires_par_la_personne.is_empty() {
            v.push(format!("retiré(s) de la liste par la personne, pas remis : {}", self.retires_par_la_personne.join(", ")));
        }
        (!v.is_empty()).then(|| v.join(" ; "))
    }
}

fn dans(l: &[String], d: &str) -> bool {
    l.iter().any(|x| meme_dossier(x, d))
}

/// La fusion : `actuels` (la liste que l'émulateur lit), `voulus` (les emplacements de la plateforme), `poses` (la
/// note d'avant), `existe` (le dossier est-il là ?).
/// - un voulu déjà dans la liste est noté (s'il disparaît ensuite, c'est la personne qui l'a retiré) ;
/// - un voulu noté mais absent de la liste n'est jamais remis ;
/// - un voulu qui n'existe pas n'est ni ajouté ni noté ;
/// - un noté qui n'est plus voulu sort de la note, et de la liste s'il y est encore.
pub fn fusionner(actuels: &[String], voulus: &[String], poses: &[String], existe: &dyn Fn(&str) -> bool) -> Fusion {
    let mut f = Fusion::default();
    for p in poses {
        if dans(voulus, p) {
            if !dans(&f.poses, p) {
                f.poses.push(p.clone());
            }
        } else if dans(actuels, p) && !dans(&f.a_retirer, p) {
            f.a_retirer.push(p.clone());
        }
    }
    for v in voulus {
        let v = v.trim();
        if v.is_empty() || dans(&f.ajoutes, v) {
            continue;
        }
        let deja = dans(actuels, v);
        let note = dans(&f.poses, v);
        if deja {
            if !note {
                f.poses.push(v.to_string());
            }
        } else if note {
            if !dans(&f.retires_par_la_personne, v) {
                f.retires_par_la_personne.push(v.to_string());
            }
        } else if existe(v) {
            f.ajoutes.push(v.to_string());
            f.poses.push(v.to_string());
        }
    }
    f
}

/// Le fichier de la note, dans le dossier de l'émulateur.
fn fichier_note(emulateur: &Path) -> PathBuf {
    emulateur.join("Profils").join(NOTE)
}

/// La clé d'un fichier de réglages dans la note (sans casse, « \ »).
fn cle(fichier: &Path) -> String {
    fichier.to_string_lossy().replace('/', "\\").to_lowercase()
}

/// La note entière. Absente : vide. Illisible : une erreur (rien ne doit alors être ajouté ni retiré).
fn lire_note(emulateur: &Path) -> Resultat<BTreeMap<String, Vec<String>>> {
    match std::fs::read(fichier_note(emulateur)) {
        Ok(o) => serde_json::from_slice(&o).map_err(|_| Erreur::Refus("la note des listes de jeux de Frogtend ne se lit pas : liste de jeux pas touchée".into())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
        Err(e) => Err(e.into()),
    }
}

/// Ce que Frogtend a posé dans la liste de ce fichier (note illisible : une erreur).
pub fn poses(emulateur: &Path, fichier: &Path) -> Resultat<Vec<String>> {
    Ok(lire_note(emulateur)?.remove(&cle(fichier)).unwrap_or_default())
}

/// Retient ce que Frogtend a posé dans la liste de ce fichier (écrit seulement si ça change ; un fichier provisoire
/// puis renommé : jamais une note à moitié écrite).
pub fn noter(emulateur: &Path, fichier: &Path, poses: &[String]) -> Resultat<()> {
    let mut n = lire_note(emulateur)?;
    if n.get(&cle(fichier)).map(Vec::as_slice) == Some(poses) {
        return Ok(());
    }
    if poses.is_empty() {
        n.remove(&cle(fichier));
    } else {
        n.insert(cle(fichier), poses.to_vec());
    }
    let f = fichier_note(emulateur);
    if let Some(p) = f.parent() {
        std::fs::create_dir_all(p)?;
    }
    let texte = serde_json::to_vec_pretty(&n).map_err(|e| Erreur::Disque(format!("note des listes de jeux impossible à écrire ({e})")))?;
    let provisoire = f.with_extension("json.tmp");
    std::fs::write(&provisoire, texte)?;
    std::fs::rename(&provisoire, &f)?;
    Ok(())
}

/// Une valeur d'INI sans ses guillemets.
fn sans_guillemets(v: &str) -> String {
    let v = v.trim();
    match v.strip_prefix('"').and_then(|x| x.strip_suffix('"')) {
        Some(x) => x.to_string(),
        None => v.to_string(),
    }
}

/// Le nombre d'entrées d'une liste numérotée (`size`, `ISOPaths`).
fn nombre(texte: &str, section: &str, cle: &str) -> usize {
    crate::manettes::lire_ini(texte, section, cle).into_iter().next().and_then(|v| v.trim().parse().ok()).unwrap_or(0).min(4096)
}

/// Une entrée de la liste d'Eden, telle qu'Eden la lit.
#[derive(Clone, Debug, PartialEq)]
struct EntreeEden {
    chemin: String,
    profond: bool,
    deplie: bool,
}

/// Un booléen d'Eden comme `ReadBooleanSetting` le lit : `\default=true` → la valeur d'origine ; sinon la valeur.
fn booleen_eden(texte: &str, cle: &str, defaut: bool) -> bool {
    let l = |c: &str| crate::manettes::lire_ini(texte, "UI", c).into_iter().next().map(|v| v.trim().to_string());
    if l(&format!("{cle}\\default")).as_deref() == Some("true") {
        return defaut;
    }
    match l(cle).as_deref() {
        Some("true") => true,
        Some("false") => false,
        _ => defaut,
    }
}

/// Les entrées ÉCRITES dans le fichier (chemin non vide).
fn ecrites_eden(texte: &str) -> Vec<EntreeEden> {
    (1..=nombre(texte, "UI", "Paths\\gamedirs\\size"))
        .filter_map(|i| {
            let p = format!("Paths\\gamedirs\\{i}\\");
            let chemin = crate::manettes::lire_ini(texte, "UI", &format!("{p}path")).into_iter().next().map(|v| crate::emulateurs_profils::lire_valeur_qt(&v))?;
            (!chemin.is_empty()).then(|| EntreeEden { chemin, profond: booleen_eden(texte, &format!("{p}deep_scan"), false), deplie: booleen_eden(texte, &format!("{p}expanded"), true) })
        })
        .collect()
}

/// La liste qu'Eden voit vraiment : celle écrite, ou, si elle est vide, celle qu'il recrée (ses entrées spéciales et
/// l'ancien dossier unique `gameListRootDir`).
fn effectives_eden(texte: &str) -> Vec<EntreeEden> {
    let l = ecrites_eden(texte);
    if !l.is_empty() {
        return l;
    }
    let mut v: Vec<EntreeEden> = SPECIALES_EDEN.iter().map(|s| EntreeEden { chemin: s.to_string(), profond: false, deplie: true }).collect();
    let ancien = crate::manettes::lire_ini(texte, "UI", "Paths\\gameListRootDir").into_iter().next().map(|v| crate::emulateurs_profils::lire_valeur_qt(&v));
    if let Some(a) = ancien.filter(|a| !a.is_empty() && a != ".") {
        let profond = crate::manettes::lire_ini(texte, "UI", "Paths\\gameListDeepScan").first().is_some_and(|v| v.trim() == "true");
        v.push(EntreeEden { chemin: a, profond, deplie: true });
    }
    v
}

/// La liste de jeux lue dans un fichier de réglages, comme l'émulateur la lit (pour DuckStation et PCSX2 : les
/// dossiers récursifs ET les autres ; pour Eden, les entrées spéciales comprises, même quand il les recrée seul).
pub fn lire(format: Format, texte: &str) -> Vec<String> {
    let ini = |s: &str, c: &str| crate::manettes::lire_ini(texte, s, c);
    match format {
        Format::DuckStation | Format::Pcsx2 => ini("GameList", "RecursivePaths").into_iter().chain(ini("GameList", "Paths")).map(|v| sans_guillemets(&v)).filter(|v| !v.is_empty()).collect(),
        Format::Dolphin => (0..nombre(texte, "General", "ISOPaths"))
            .filter_map(|i| ini("General", &format!("ISOPath{i}")).into_iter().next())
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .collect(),
        Format::Eden => effectives_eden(texte).into_iter().map(|e| e.chemin).collect(),
    }
}

/// Les clés d'une entrée Eden numéro `i` (chacune avec son `\default`, comme Eden les écrit).
fn cles_eden(i: usize, e: &EntreeEden) -> Vec<(String, String)> {
    let p = format!("Paths\\gamedirs\\{i}\\");
    vec![
        (format!("{p}path"), crate::emulateurs_profils::valeur_qt(&e.chemin.replace('\\', "/"))),
        (format!("{p}deep_scan\\default"), (!e.profond).to_string()),
        (format!("{p}deep_scan"), e.profond.to_string()),
        (format!("{p}expanded\\default"), e.deplie.to_string()),
        (format!("{p}expanded"), e.deplie.to_string()),
    ]
}

/// Écrit la liste Eden `entrees` à partir de l'index `de` (1 pour toute la liste), puis sa taille.
fn ecrire_eden(texte: &str, de: usize, entrees: &[EntreeEden]) -> String {
    let mut v: Vec<(String, String)> = Vec::new();
    for (k, e) in entrees.iter().enumerate() {
        v.extend(cles_eden(de + k, e));
    }
    v.push(("Paths\\gamedirs\\size".into(), (de - 1 + entrees.len()).to_string()));
    let refs: Vec<(&str, &str)> = v.iter().map(|(c, x)| (c.as_str(), x.as_str())).collect();
    crate::emulateurs_profils::regler_cles_qt(texte, "UI", &refs).unwrap_or_else(|| texte.to_string())
}

/// Le texte du fichier avec `nouveaux` ajoutés à la fin de la liste (les entrées existantes ne sont jamais réécrites).
pub fn ajouter(format: Format, texte: &str, nouveaux: &[String]) -> String {
    if nouveaux.is_empty() {
        return texte.to_string();
    }
    match format {
        Format::DuckStation | Format::Pcsx2 => {
            // Les lignes `RecursivePaths` existantes gardées telles quelles (guillemets compris), les nouvelles après.
            let mut l = crate::manettes::lire_ini(texte, "GameList", "RecursivePaths");
            for n in nouveaux {
                l.push(if format == Format::DuckStation && n.contains(['#', ';']) { format!("\"{n}\"") } else { n.clone() });
            }
            crate::emulateurs_profils::ecrire_ini(texte, "GameList", &[("RecursivePaths", l)])
        }
        Format::Dolphin => {
            let n0 = nombre(texte, "General", "ISOPaths");
            let mut v: Vec<(String, Vec<String>)> = vec![("ISOPaths".into(), vec![(n0 + nouveaux.len()).to_string()])];
            for (k, n) in nouveaux.iter().enumerate() {
                v.push((format!("ISOPath{}", n0 + k), vec![n.clone()]));
            }
            let refs: Vec<(&str, Vec<String>)> = v.iter().map(|(c, vs)| (c.as_str(), vs.clone())).collect();
            crate::emulateurs_profils::ecrire_ini(texte, "General", &refs)
        }
        Format::Eden => {
            // Les jeux de Frogtend sont rangés un dossier par jeu : recherche dans les sous-dossiers.
            let nouvelles: Vec<EntreeEden> = nouveaux.iter().map(|n| EntreeEden { chemin: n.clone(), profond: true, deplie: true }).collect();
            if ecrites_eden(texte).is_empty() {
                // Liste écrite vide : celle qu'Eden aurait recréée d'abord (il ne la recrée plus une fois non vide).
                let mut toutes = effectives_eden(texte);
                toutes.extend(nouvelles);
                ecrire_eden(texte, 1, &toutes)
            } else {
                ecrire_eden(texte, nombre(texte, "UI", "Paths\\gamedirs\\size") + 1, &nouvelles)
            }
        }
    }
}

/// Le texte du fichier sans les dossiers `partants` (les autres gardés, dans leur ordre).
pub fn retirer(format: Format, texte: &str, partants: &[String]) -> String {
    if partants.is_empty() {
        return texte.to_string();
    }
    match format {
        Format::DuckStation | Format::Pcsx2 => {
            let mut t = texte.to_string();
            for cle in ["RecursivePaths", "Paths"] {
                let avant = crate::manettes::lire_ini(&t, "GameList", cle);
                let apres: Vec<String> = avant.iter().filter(|v| !dans(partants, &sans_guillemets(v))).cloned().collect();
                if apres.len() != avant.len() {
                    t = crate::emulateurs_profils::ecrire_ini(&t, "GameList", &[(cle, apres)]);
                }
            }
            t
        }
        Format::Dolphin => {
            let n0 = nombre(texte, "General", "ISOPaths");
            let gardes: Vec<String> = lire(Format::Dolphin, texte).into_iter().filter(|v| !dans(partants, v)).collect();
            let mut v: Vec<(String, Vec<String>)> = vec![("ISOPaths".into(), vec![gardes.len().to_string()])];
            for i in 0..n0.max(gardes.len()) {
                v.push((format!("ISOPath{i}"), gardes.get(i).map(|g| vec![g.clone()]).unwrap_or_default()));
            }
            let refs: Vec<(&str, Vec<String>)> = v.iter().map(|(c, vs)| (c.as_str(), vs.clone())).collect();
            crate::emulateurs_profils::ecrire_ini(texte, "General", &refs)
        }
        Format::Eden => {
            // Toute la liste réécrite, chaque entrée telle qu'Eden la lisait (au-delà de la taille, Eden ne lit rien).
            let gardes: Vec<EntreeEden> = effectives_eden(texte).into_iter().filter(|e| !dans(partants, &e.chemin)).collect();
            ecrire_eden(texte, 1, &gardes)
        }
    }
}

/// Dolphin : la recherche dans les sous-dossiers, seulement si la personne n'a rien réglé (jamais son choix défait).
fn dolphin_recursif(texte: &str) -> String {
    if crate::manettes::lire_ini(texte, "General", "RecursiveISOPaths").is_empty() {
        crate::emulateurs_profils::ecrire_ini(texte, "General", &[("RecursiveISOPaths", vec!["True".into()])])
    } else {
        texte.to_string()
    }
}

/// Complète la liste de jeux de `fichier` (format `format`, dans l'émulateur `emulateur`) avec les emplacements
/// `voulus` qui existent, sans rien retirer de la personne. `ouvert` : l'émulateur tourne (rien n'est écrit). Le
/// fichier est mis à l'abri avant, relu après (chaque ajout doit y être relu comme l'émulateur le lit, chaque retrait
/// absent ; sinon l'ancien texte revient).
pub fn completer(emulateur: &Path, fichier: &Path, format: Format, voulus: &[String], ouvert: bool) -> Resultat<Bilan> {
    completer_avec(emulateur, fichier, format, voulus, ouvert, &|d| Path::new(d).is_dir())
}

fn completer_avec(emulateur: &Path, fichier: &Path, format: Format, voulus: &[String], ouvert: bool, existe: &dyn Fn(&str) -> bool) -> Resultat<Bilan> {
    if ouvert {
        return Ok(Bilan { ouvert: true, ..Default::default() });
    }
    // La note d'abord : illisible, rien n'est touché.
    let notes = poses(emulateur, fichier)?;
    let existait = fichier.is_file();
    let avant = match std::fs::read_to_string(fichier) {
        Ok(t) => t,
        // Pas encore créé : ce que l'émulateur y aurait mis lui-même (comme `modifier_ini`).
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => crate::emulateurs_profils::base_neuve(emulateur, fichier),
        // Illisible : jamais remplacé.
        Err(e) => return Err(e.into()),
    };
    let f = fusionner(&lire(format, &avant), voulus, &notes, existe);
    let mut apres = retirer(format, &avant, &f.a_retirer);
    apres = ajouter(format, &apres, &f.ajoutes);
    if format == Format::Dolphin && lire(Format::Dolphin, &apres).iter().any(|x| dans(voulus, x)) {
        apres = dolphin_recursif(&apres);
    }
    if apres != avant {
        crate::emulateurs_profils::remplacer_config(emulateur, fichier, &apres)?;
        // Vérifier le résultat : chaque ajout relu comme l'émulateur le lit, chaque retrait parti.
        let relue = lire(format, &std::fs::read_to_string(fichier).unwrap_or_default());
        if !f.ajoutes.iter().all(|a| dans(&relue, a)) || f.a_retirer.iter().any(|r| dans(&relue, r)) {
            if existait {
                std::fs::write(fichier, &avant)?;
            } else {
                let _ = std::fs::remove_file(fichier);
            }
            return Err(Erreur::Disque("la liste de jeux ne se relisait pas après l'écriture : remise comme avant".into()));
        }
    }
    if f.poses != notes {
        noter(emulateur, fichier, &f.poses)?;
    }
    Ok(Bilan { ajoutes: f.ajoutes, retires: f.a_retirer, retires_par_la_personne: f.retires_par_la_personne, ouvert: false })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(l: &[&str]) -> Vec<String> {
        l.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn la_fusion_ajoute_garde_et_ne_remet_jamais_un_dossier_retire() {
        let tous = |_: &str| true;
        // Ajout à la fin ; la liste de la personne est gardée (rien à réécrire pour elle).
        let f = fusionner(&s(&["C:\\Mes jeux"]), &s(&["E:\\Jeux\\PS1"]), &[], &tous);
        assert_eq!(f.ajoutes, s(&["E:\\Jeux\\PS1"]));
        assert_eq!(f.poses, s(&["E:\\Jeux\\PS1"]));
        // Déjà là (autre casse, « / », barre finale) : rien d'ajouté, mais noté.
        let f = fusionner(&s(&["e:/jeux/ps1/"]), &s(&["E:\\Jeux\\PS1"]), &[], &tous);
        assert!(f.ajoutes.is_empty());
        assert_eq!(f.poses, s(&["E:\\Jeux\\PS1"]));
        // Noté puis absent : retiré par la personne, jamais remis.
        let f = fusionner(&s(&["C:\\Mes jeux"]), &s(&["E:\\Jeux\\PS1"]), &s(&["e:\\jeux\\ps1"]), &tous);
        assert!(f.ajoutes.is_empty());
        assert_eq!(f.retires_par_la_personne, s(&["E:\\Jeux\\PS1"]));
        assert_eq!(f.poses, s(&["e:\\jeux\\ps1"]), "la note garde le refus");
        // Un emplacement qui n'existe pas : ni ajouté ni noté.
        let f = fusionner(&[], &s(&["Z:\\Absent", "E:\\Jeux"]), &[], &|d| d != "Z:\\Absent");
        assert_eq!(f.ajoutes, s(&["E:\\Jeux"]));
        assert_eq!(f.poses, s(&["E:\\Jeux"]));
        // Deux fois le même emplacement : ajouté une fois.
        let f = fusionner(&[], &s(&["E:\\Jeux", "e:/jeux/"]), &[], &tous);
        assert_eq!(f.ajoutes, s(&["E:\\Jeux"]));
    }

    #[test]
    fn un_dossier_de_frogtend_qui_n_est_plus_un_emplacement_est_retire_jamais_celui_de_la_personne() {
        let tous = |_: &str| true;
        // L'emplacement est passé de E:\Vieux à F:\Neuf : E:\Vieux (noté) sort de la liste et de la note ; C:\A moi
        // (jamais noté) reste.
        let f = fusionner(&s(&["C:\\A moi", "E:\\Vieux"]), &s(&["F:\\Neuf"]), &s(&["E:\\Vieux"]), &tous);
        assert_eq!(f.a_retirer, s(&["E:\\Vieux"]));
        assert_eq!(f.ajoutes, s(&["F:\\Neuf"]));
        assert_eq!(f.poses, s(&["F:\\Neuf"]));
        // Déjà retiré par la personne : seulement oublié de la note.
        let f = fusionner(&s(&["C:\\A moi"]), &s(&["F:\\Neuf"]), &s(&["E:\\Vieux", "F:\\Neuf"]), &tous);
        assert!(f.a_retirer.is_empty() && f.ajoutes.is_empty());
        assert_eq!(f.poses, s(&["F:\\Neuf"]));
        // Un dossier non noté (ancien Frogtend sans note, ou la personne) n'est jamais retiré.
        let f = fusionner(&s(&["E:\\Vieux"]), &s(&["F:\\Neuf"]), &[], &tous);
        assert!(f.a_retirer.is_empty());
    }

    #[test]
    fn duckstation_et_pcsx2_ajoutent_une_ligne_par_dossier() {
        let avant = "[Main]\r\nLanguage = fr\r\n\r\n[GameList]\r\nRecursivePaths = C:\\vieux\r\nPaths = D:\\plat\r\nRecursivePaths = \"F:\\a#b\"\r\n\r\n[Pad1]\r\nType = x\r\n";
        assert_eq!(lire(Format::DuckStation, avant), s(&["C:\\vieux", "F:\\a#b", "D:\\plat"]));
        let apres = ajouter(Format::DuckStation, avant, &s(&["E:\\Jeux\\PS1", "E:\\a;b"]));
        assert_eq!(crate::manettes::lire_ini(&apres, "GameList", "RecursivePaths"), s(&["C:\\vieux", "\"F:\\a#b\"", "E:\\Jeux\\PS1", "\"E:\\a;b\""]));
        assert_eq!(lire(Format::DuckStation, &apres), s(&["C:\\vieux", "F:\\a#b", "E:\\Jeux\\PS1", "E:\\a;b", "D:\\plat"]));
        assert!(apres.contains("Paths = D:\\plat") && apres.contains("Language = fr") && apres.contains("Type = x"));
        // Retirer : seulement celui-là (guillemets compris), le reste gardé.
        let moins = retirer(Format::DuckStation, &apres, &s(&["E:\\a;b"]));
        assert_eq!(lire(Format::DuckStation, &moins), s(&["C:\\vieux", "F:\\a#b", "E:\\Jeux\\PS1", "D:\\plat"]));
        // PCSX2 (SimpleIni) : jamais de guillemets (ils seraient lus avec le chemin).
        let apres = ajouter(Format::Pcsx2, "[GameList]\r\nRecursivePaths = C:\\vieux\r\n", &s(&["E:\\a;b"]));
        assert_eq!(lire(Format::Pcsx2, &apres), s(&["C:\\vieux", "E:\\a;b"]));
    }

    #[test]
    fn dolphin_ajoute_apres_les_dossiers_de_la_personne() {
        let avant = "[General]\r\nISOPaths = 2\r\nISOPath0 = C:/GC\r\nISOPath1 = \r\nISOPath2 = vieux\r\n[Core]\r\nx = 1\r\n";
        assert_eq!(lire(Format::Dolphin, avant), s(&["C:/GC"]), "entrée vide ignorée, au-delà du compte ignorée");
        let apres = ajouter(Format::Dolphin, avant, &s(&["E:\\Jeux\\GC"]));
        assert_eq!(lire(Format::Dolphin, &apres), s(&["C:/GC", "E:\\Jeux\\GC"]));
        assert!(apres.contains("x = 1"));
        // Retirer : renuméroté, les numéros en trop effacés.
        let moins = retirer(Format::Dolphin, "[General]\r\nISOPaths = 3\r\nISOPath0 = A\r\nISOPath1 = B\r\nISOPath2 = C\r\n", &s(&["B"]));
        assert_eq!(lire(Format::Dolphin, &moins), s(&["A", "C"]));
        assert_eq!(crate::manettes::lire_ini(&moins, "General", "ISOPaths"), s(&["2"]));
        assert!(crate::manettes::lire_ini(&moins, "General", "ISOPath2").is_empty());
        // La recherche dans les sous-dossiers : posée seulement si la personne n'a rien réglé.
        assert_eq!(crate::manettes::lire_ini(&dolphin_recursif("[General]\r\n"), "General", "RecursiveISOPaths"), s(&["True"]));
        assert_eq!(crate::manettes::lire_ini(&dolphin_recursif("[General]\r\nRecursiveISOPaths = False\r\n"), "General", "RecursiveISOPaths"), s(&["False"]));
    }

    #[test]
    fn eden_garde_ses_entrees_speciales_et_ajoute_apres() {
        // La forme écrite par Eden (SavePathValues, clés `\default` comprises).
        let avant = "[UI]\r\ntheme=dark\r\nPaths\\gamedirs\\1\\path=SDMC\r\nPaths\\gamedirs\\1\\deep_scan\\default=true\r\nPaths\\gamedirs\\1\\deep_scan=false\r\nPaths\\gamedirs\\2\\path=UserNAND\r\nPaths\\gamedirs\\3\\path=SysNAND\r\nPaths\\gamedirs\\4\\path=\"C:/Mes jeux\"\r\nPaths\\gamedirs\\4\\deep_scan\\default=false\r\nPaths\\gamedirs\\4\\deep_scan=true\r\nPaths\\gamedirs\\size=4\r\n[Controls]\r\nx=1\r\n";
        assert_eq!(lire(Format::Eden, avant), s(&["SDMC", "UserNAND", "SysNAND", "C:/Mes jeux"]));
        let apres = ajouter(Format::Eden, avant, &s(&["E:\\Jeux\\Switch"]));
        assert_eq!(lire(Format::Eden, &apres), s(&["SDMC", "UserNAND", "SysNAND", "C:/Mes jeux", "E:/Jeux/Switch"]));
        let l = |t: &str, c: &str| crate::manettes::lire_ini(t, "UI", c);
        assert_eq!(l(&apres, "Paths\\gamedirs\\size"), s(&["5"]));
        assert_eq!(l(&apres, "Paths\\gamedirs\\5\\deep_scan"), s(&["true"]));
        assert_eq!(l(&apres, "Paths\\gamedirs\\5\\deep_scan\\default"), s(&["false"]), "sinon Eden ignorerait la valeur");
        assert_eq!(l(&apres, "Paths\\gamedirs\\1\\deep_scan\\default"), s(&["true"]), "les entrées d'Eden jamais réécrites");
        assert!(apres.contains("theme=dark") && apres.contains("x=1"));
        assert!(!apres.contains("gamedirs\\5\\path = "), "la forme d'Eden : sans espaces");
        // Retirer une entrée du milieu : les suivantes remontent avec LEURS réglages.
        let moins = retirer(Format::Eden, &apres, &s(&["E:/Jeux/Switch"]));
        assert_eq!(lire(Format::Eden, &moins), s(&["SDMC", "UserNAND", "SysNAND", "C:/Mes jeux"]));
        let moins = retirer(Format::Eden, avant, &s(&["UserNAND"]));
        assert_eq!(lire(Format::Eden, &moins), s(&["SDMC", "SysNAND", "C:/Mes jeux"]));
        assert_eq!(booleen_eden(&moins, "Paths\\gamedirs\\3\\deep_scan", false), true, "C:/Mes jeux garde sa recherche profonde");
        assert_eq!(booleen_eden(&moins, "Paths\\gamedirs\\2\\deep_scan", false), false);
        // Liste vide (fichier neuf, ou taille 0) : les entrées spéciales d'abord, comme Eden les aurait recréées.
        for vide in ["", "[UI]\r\nPaths\\gamedirs\\size=0\r\n"] {
            assert_eq!(lire(Format::Eden, vide), s(&["SDMC", "UserNAND", "SysNAND"]), "celle qu'Eden voit vraiment");
            let apres = ajouter(Format::Eden, vide, &s(&["E:\\Jeux\\Switch"]));
            assert_eq!(lire(Format::Eden, &apres), s(&["SDMC", "UserNAND", "SysNAND", "E:/Jeux/Switch"]));
            assert_eq!(l(&apres, "Paths\\gamedirs\\1\\expanded"), s(&["true"]));
        }
        // L'ancien dossier unique est repris, comme Eden le fait pour une liste vide.
        let ancien = "[UI]\r\nPaths\\gameListRootDir=D:/Vieux\r\nPaths\\gameListDeepScan=true\r\n";
        let apres = ajouter(Format::Eden, ancien, &s(&["E:\\S"]));
        assert_eq!(lire(Format::Eden, &apres), s(&["SDMC", "UserNAND", "SysNAND", "D:/Vieux", "E:/S"]));
        assert_eq!(l(&apres, "Paths\\gamedirs\\4\\deep_scan"), s(&["true"]));
    }

    #[test]
    fn eden_l_ancien_dossier_unique_deja_emplacement_n_est_pas_en_double() {
        let d = tempfile::tempdir().unwrap();
        let eden = d.path().join("Eden");
        let jeux = d.path().join("Switch");
        std::fs::create_dir_all(&jeux).unwrap();
        let j = jeux.to_string_lossy().replace('\\', "/");
        let ini = crate::contenus::ini_eden(&eden);
        std::fs::create_dir_all(ini.parent().unwrap()).unwrap();
        let texte = format!("[UI]\r\nPaths\\gameListRootDir={j}\r\n");
        std::fs::write(&ini, &texte).unwrap();
        let b = completer(&eden, &ini, Format::Eden, &[jeux.to_string_lossy().to_string()], false).unwrap();
        assert!(b.ajoutes.is_empty(), "déjà dans la liste qu'Eden recrée");
        assert_eq!(std::fs::read_to_string(&ini).unwrap(), texte, "rien d'écrit : Eden la recrée seul");
    }

    #[test]
    fn la_liste_est_completee_notee_et_jamais_ecrite_emulateur_ouvert() {
        let d = tempfile::tempdir().unwrap();
        let emu = d.path().join("DuckStation");
        std::fs::create_dir_all(&emu).unwrap();
        let jeux = d.path().join("Jeux PS1");
        std::fs::create_dir_all(&jeux).unwrap();
        let ini = emu.join("settings.ini");
        std::fs::write(&ini, "[GameList]\r\nRecursivePaths = C:\\A moi\r\n").unwrap();
        let voulus = vec![jeux.to_string_lossy().to_string(), d.path().join("Absent").to_string_lossy().to_string()];
        // Ouvert : rien n'est écrit, rien n'est noté.
        let b = completer(&emu, &ini, Format::DuckStation, &voulus, true).unwrap();
        assert!(b.ouvert && b.ajoutes.is_empty());
        assert_eq!(std::fs::read_to_string(&ini).unwrap(), "[GameList]\r\nRecursivePaths = C:\\A moi\r\n");
        assert!(poses(&emu, &ini).unwrap().is_empty());
        // Fermé : ajouté (l'absent non), noté, fichier mis à l'abri.
        let b = completer(&emu, &ini, Format::DuckStation, &voulus, false).unwrap();
        assert_eq!(b.ajoutes, vec![voulus[0].clone()]);
        let lu = lire(Format::DuckStation, &std::fs::read_to_string(&ini).unwrap());
        assert_eq!(lu, vec!["C:\\A moi".to_string(), voulus[0].clone()]);
        assert_eq!(poses(&emu, &ini).unwrap(), vec![voulus[0].clone()]);
        assert!(emu.join(".frogtend-sauvegardes").is_dir());
        assert!(!fichier_note(&emu).with_extension("json.tmp").exists(), "la note est renommée, jamais laissée provisoire");
        // Relancé : rien ne change.
        assert!(completer(&emu, &ini, Format::DuckStation, &voulus, false).unwrap().ajoutes.is_empty());
        // La personne retire le dossier de Frogtend dans l'émulateur : il n'est jamais remis, le sien reste.
        std::fs::write(&ini, "[GameList]\r\nRecursivePaths = C:\\A moi\r\n").unwrap();
        let b = completer(&emu, &ini, Format::DuckStation, &voulus, false).unwrap();
        assert!(b.ajoutes.is_empty());
        assert_eq!(b.retires_par_la_personne, vec![voulus[0].clone()]);
        assert_eq!(std::fs::read_to_string(&ini).unwrap(), "[GameList]\r\nRecursivePaths = C:\\A moi\r\n");
        assert!(b.resume().unwrap().contains("pas remis"));
    }

    #[test]
    fn un_emplacement_change_retire_l_ancien_de_frogtend() {
        let d = tempfile::tempdir().unwrap();
        let emu = d.path().join("PCSX2");
        let ini = emu.join("inis").join("PCSX2.ini");
        std::fs::create_dir_all(ini.parent().unwrap()).unwrap();
        let (vieux, neuf) = (d.path().join("Vieux"), d.path().join("Neuf"));
        std::fs::create_dir_all(&vieux).unwrap();
        std::fs::create_dir_all(&neuf).unwrap();
        let (v, n) = (vieux.to_string_lossy().to_string(), neuf.to_string_lossy().to_string());
        std::fs::write(&ini, "[UI]\r\nSettingsVersion = 1\r\n[GameList]\r\nRecursivePaths = C:\\A moi\r\n").unwrap();
        completer(&emu, &ini, Format::Pcsx2, std::slice::from_ref(&v), false).unwrap();
        let b = completer(&emu, &ini, Format::Pcsx2, std::slice::from_ref(&n), false).unwrap();
        assert_eq!(b.retires, vec![v.clone()]);
        assert_eq!(lire(Format::Pcsx2, &std::fs::read_to_string(&ini).unwrap()), vec!["C:\\A moi".to_string(), n.clone()]);
        assert_eq!(poses(&emu, &ini).unwrap(), vec![n]);
    }

    /// Un profil Dolphin déjà rempli par un Frogtend d'avant (liste remplacée, pas de note) : la recherche dans les
    /// sous-dossiers est posée quand même, sans rien ajouter.
    #[test]
    fn dolphin_deja_rempli_recoit_la_recherche_dans_les_sous_dossiers() {
        let d = tempfile::tempdir().unwrap();
        let emu = d.path().join("Dolphin");
        let jeux = d.path().join("GC");
        std::fs::create_dir_all(&jeux).unwrap();
        let j = jeux.to_string_lossy().to_string();
        let ini = emu.join("Profils").join("Seb").join("User").join("Config").join("Dolphin.ini");
        std::fs::create_dir_all(ini.parent().unwrap()).unwrap();
        std::fs::write(&ini, format!("[General]\r\nISOPaths = 1\r\nISOPath0 = {j}\r\n")).unwrap();
        let b = completer(&emu, &ini, Format::Dolphin, std::slice::from_ref(&j), false).unwrap();
        assert!(b.ajoutes.is_empty());
        let t = std::fs::read_to_string(&ini).unwrap();
        assert_eq!(crate::manettes::lire_ini(&t, "General", "RecursiveISOPaths"), s(&["True"]));
        assert_eq!(lire(Format::Dolphin, &t), vec![j.clone()]);
        // Le choix de la personne (False) n'est jamais défait.
        std::fs::write(&ini, format!("[General]\r\nISOPaths = 1\r\nISOPath0 = {j}\r\nRecursiveISOPaths = False\r\n")).unwrap();
        completer(&emu, &ini, Format::Dolphin, std::slice::from_ref(&j), false).unwrap();
        assert_eq!(crate::manettes::lire_ini(&std::fs::read_to_string(&ini).unwrap(), "General", "RecursiveISOPaths"), s(&["False"]));
        // Aucun emplacement de Frogtend dans la liste : rien n'est posé.
        std::fs::write(&ini, "[General]\r\nISOPaths = 1\r\nISOPath0 = C:\\A moi\r\n").unwrap();
        completer(&emu, &ini, Format::Dolphin, &[d.path().join("Absent").to_string_lossy().to_string()], false).unwrap();
        assert!(crate::manettes::lire_ini(&std::fs::read_to_string(&ini).unwrap(), "General", "RecursiveISOPaths").is_empty());
    }

    #[test]
    fn une_note_illisible_ne_touche_a_rien() {
        let d = tempfile::tempdir().unwrap();
        let emu = d.path().join("DuckStation");
        let jeux = d.path().join("Jeux");
        std::fs::create_dir_all(emu.join("Profils")).unwrap();
        std::fs::create_dir_all(&jeux).unwrap();
        let ini = emu.join("settings.ini");
        std::fs::write(&ini, "[GameList]\r\n").unwrap();
        std::fs::write(fichier_note(&emu), b"{ abime").unwrap();
        assert!(completer(&emu, &ini, Format::DuckStation, &[jeux.to_string_lossy().to_string()], false).is_err());
        assert_eq!(std::fs::read_to_string(&ini).unwrap(), "[GameList]\r\n", "rien d'ajouté");
        assert_eq!(std::fs::read(fichier_note(&emu)).unwrap(), b"{ abime", "rien de noté");
    }

    #[test]
    fn une_liste_qui_ne_se_relit_pas_est_remise_comme_avant() {
        let d = tempfile::tempdir().unwrap();
        let emu = d.path().join("DuckStation");
        std::fs::create_dir_all(&emu).unwrap();
        let ini = emu.join("settings.ini");
        let avant = "[GameList]\r\nRecursivePaths = C:\\A moi\r\n";
        std::fs::write(&ini, avant).unwrap();
        // Un chemin coupé par un retour à la ligne : DuckStation ne le relirait pas en entier.
        let r = completer_avec(&emu, &ini, Format::DuckStation, &["E:\\x\ny".to_string()], false, &|_| true);
        assert!(matches!(r, Err(Erreur::Disque(_))));
        assert_eq!(std::fs::read_to_string(&ini).unwrap(), avant);
        assert!(poses(&emu, &ini).unwrap().is_empty(), "rien de noté");
    }
}
