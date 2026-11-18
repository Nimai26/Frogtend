// Mises à jour et DLC Switch : FROGTEND EST LA SOURCE DE VÉRITÉ (✅ Seb 17/11, après l'essai « Manage DLC » de
// Ryubing). Un interrupteur par contenu (la mise à jour, chaque DLC) sur la fiche du jeu, allumé d'office, valable
// pour CE JEU quel que soit l'émulateur (Eden, Ryubing) et pour TOUTE LA MAISON (pas par personne). À chaque lancement
// Frogtend remet l'émulateur d'accord avec ce choix : un retrait fait dans l'émulateur ne compte plus.
//
// Le choix vit dans `<données de Frogtend>\contenus-switch.json` (jamais dans pc.json, que l'interface réécrit) :
// `{ "<BASE hex 16 majuscules>": { "maj": false, "dlc_eteints": ["dlc:…"], "origine": {…}, "abri": {…} }, "_migre": true }`.
// Seul ce qui est ÉTEINT est noté : un jeu absent = tout allumé ; un DLC nouveau = allumé. La mise à jour est un
// booléen (la plus récente, ou aucune) ; un DLC est noté par `switch_nca::identifiant_dlc` (« dlc:<TID>+… »), le même
// quel que soit son fichier.

use crate::erreurs::{Erreur, Resultat};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Le fichier des choix, dans les données de Frogtend.
pub const FICHIER: &str = "contenus-switch.json";
/// La clé de la mise à jour (un DLC a pour clé son identifiant « dlc:… »).
pub const CLE_MAJ: &str = "maj";
/// Migration d'un `[DisabledAddOns]` d'Eden (« DLC » : TOUS les DLC du jeu coupés) : tous les DLC trouvés à la
/// première lecture du jeu seront éteints (remplacé alors par leurs identifiants, `resoudre`).
pub const TOUS_LES_DLC: &str = "dlc:*";
/// Au-delà, le fichier n'est pas celui de Frogtend (comme `switch_nca::lire_cache`) : ignoré, jamais lu en entier.
const MAX_OCTETS: u64 = 4 << 20;

fn vrai() -> bool {
    true
}

fn est_vrai(b: &bool) -> bool {
    *b
}

/// Les parties de ce jeu sont à mettre à l'abri avant sa prochaine partie (un contenu jusque-là allumé a été éteint) :
/// `faits` = les lieux (un Eden, une racine de Ryubing) où c'est déjà fait depuis `depuis`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AbriEnAttente {
    pub depuis: String,
    #[serde(default)]
    pub faits: Vec<String>,
}

/// Le choix d'un jeu.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChoixJeu {
    /// La mise à jour la plus récente est-elle utilisée ?
    #[serde(default = "vrai", skip_serializing_if = "est_vrai")]
    pub maj: bool,
    /// Les DLC éteints (identifiants « dlc:… »).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dlc_eteints: Vec<String>,
    /// D'où vient un « éteint » repris par la migration (clé → source), pour le journal et le dépannage.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub origine: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abri: Option<AbriEnAttente>,
    /// Les mods Switch éteints (clés « mod:<clé du mod> », ✅ Seb 18/11) : un mod téléchargé est allumé sans rien noter.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mods_eteints: Vec<String>,
}

impl Default for ChoixJeu {
    fn default() -> Self {
        ChoixJeu { maj: true, dlc_eteints: vec![], origine: BTreeMap::new(), abri: None, mods_eteints: vec![] }
    }
}

/// Le préfixe de la clé d'un mod Switch dans les choix (« mod:<clé du mod> »).
pub const PREFIXE_MOD: &str = "mod:";

/// La clé d'un mod servi par Firehouse est-elle de la forme du contrat 1.44 (« <source>-<12 hex> ») ?
pub fn cle_de_mod_valide(cle: &str) -> bool {
    let Some((source, hexa)) = cle.rsplit_once('-') else { return false };
    cle.len() <= 120
        && !source.is_empty()
        && source.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        && hexa.len() == 12
        && hexa.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

impl ChoixJeu {
    /// Ce DLC (son identifiant) est-il allumé ?
    pub fn dlc_actif(&self, id: &str) -> bool {
        !self.dlc_eteints.iter().any(|d| d == id || d == TOUS_LES_DLC)
    }

    /// Le contenu `cle` (« maj » ou un identifiant de DLC) est-il allumé ?
    pub fn actif(&self, cle: &str) -> bool {
        if cle == CLE_MAJ {
            self.maj
        } else if cle.starts_with(PREFIXE_MOD) {
            !self.mods_eteints.iter().any(|m| m == cle)
        } else {
            self.dlc_actif(cle)
        }
    }

    /// Les parties sont-elles à mettre à l'abri au lieu `lieu` avant la prochaine partie ?
    pub fn abri_a_faire(&self, lieu: &str) -> bool {
        self.abri.as_ref().is_some_and(|a| !a.faits.iter().any(|f| f == lieu))
    }

    /// Rien à noter : tout allumé, aucun abri en attente, aucune origine (un jeu repris par la migration reste noté :
    /// « Ouvrir Ryubing seul » remet alors aussi ses DLC rallumés à `is_enabled: true`).
    fn par_defaut(&self) -> bool {
        self.maj && self.dlc_eteints.is_empty() && self.abri.is_none() && self.origine.is_empty() && self.mods_eteints.is_empty()
    }

    /// Le mod Switch de clé `cle` (sans le préfixe) est-il allumé ?
    pub fn mod_actif(&self, cle: &str) -> bool {
        self.actif(&format!("{PREFIXE_MOD}{cle}"))
    }
}

/// Le contenu du fichier.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Choix {
    /// Les retraits faits avant le 17/11 dans les émulateurs ont été repris (une seule fois).
    #[serde(rename = "_migre", default)]
    pub migre: bool,
    #[serde(flatten)]
    pub jeux: BTreeMap<String, ChoixJeu>,
}

impl Choix {
    /// Le choix d'un jeu (tout allumé s'il n'est pas noté).
    pub fn jeu(&self, base: u64) -> ChoixJeu {
        self.jeux.get(&cle_du_jeu(base)).cloned().unwrap_or_default()
    }

    fn poser(&mut self, base: u64, c: ChoixJeu) {
        if c.par_defaut() {
            self.jeux.remove(&cle_du_jeu(base));
        } else {
            self.jeux.insert(cle_du_jeu(base), c);
        }
    }
}

/// La clé d'un jeu : son identifiant de base, 16 chiffres hexadécimaux MAJUSCULES.
pub fn cle_du_jeu(base: u64) -> String {
    format!("{base:016X}")
}

pub fn chemin(donnees: &Path) -> PathBuf {
    donnees.join(FICHIER)
}

/// Ce que donne la lecture du fichier.
#[derive(Debug, Clone, PartialEq)]
pub enum Lecture {
    Absent,
    Lu(Choix),
    /// Abîmé ou trop gros (la raison, en mots simples) : ignoré, tout est allumé.
    Abime(String),
}

/// Lit le fichier des choix (plafonné en taille : un fichier énorme n'est jamais lu).
pub fn lire(donnees: &Path) -> Lecture {
    let f = chemin(donnees);
    match std::fs::metadata(&f) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Lecture::Absent,
        Err(e) => return Lecture::Abime(format!("illisible ({e})")),
        Ok(m) if m.len() > MAX_OCTETS => return Lecture::Abime("anormalement gros".into()),
        Ok(_) => {}
    }
    match std::fs::read(&f) {
        Ok(o) => match serde_json::from_slice::<Choix>(&o) {
            Ok(c) => Lecture::Lu(c),
            Err(e) => Lecture::Abime(format!("abîmé ({e})")),
        },
        Err(e) => Lecture::Abime(format!("illisible ({e})")),
    }
}

/// Les choix (vides si le fichier manque ou est abîmé) et, s'il est abîmé, une phrase pour le journal.
pub fn charger(donnees: &Path) -> (Choix, Option<String>) {
    match lire(donnees) {
        Lecture::Absent => (Choix::default(), None),
        Lecture::Lu(c) => (c, None),
        Lecture::Abime(m) => (Choix::default(), Some(format!("le fichier des choix des mises à jour et DLC Switch est {m} : il est ignoré (tout est utilisé)"))),
    }
}

/// Ce que la fiche dit d'un fichier des choix abîmé (le détail va au journal).
pub const ABIME_SUR_LA_FICHE: &str =
    "Le fichier de tes choix des mises à jour et DLC Switch est abîmé : rien n'est changé dans les émulateurs tant qu'il l'est. Frogtend le met de côté et le refait d'après ce que les émulateurs utilisent ; vérifie ensuite ces choix.";

/// Écrit les choix (provisoire puis renommage, relu après). Un fichier abîmé n'est JAMAIS écrasé : il est d'abord mis
/// de côté (`contenus-switch.abime-<horodatage>.json`) ; si ce n'est pas possible, rien n'est écrit.
pub fn ecrire(donnees: &Path, c: &Choix) -> Resultat<()> {
    std::fs::create_dir_all(donnees)?;
    let f = chemin(donnees);
    if let Lecture::Abime(_) = lire(donnees) {
        let mut de_cote = donnees.join(format!("contenus-switch.abime-{}.json", crate::noyau::maintenant()));
        let mut n = 2;
        while de_cote.exists() {
            de_cote = donnees.join(format!("contenus-switch.abime-{}-{n}.json", crate::noyau::maintenant()));
            n += 1;
        }
        std::fs::rename(&f, &de_cote).map_err(|e| Erreur::Disque(format!("le fichier abîmé des choix n'a pas pu être mis de côté ({e}) : rien n'est enregistré")))?;
    }
    let texte = serde_json::to_vec_pretty(c).map_err(|e| Erreur::Disque(format!("choix impossibles à écrire ({e})")))?;
    // Un provisoire propre à cette écriture (jamais partagé avec une autre écriture en cours).
    static NUMERO: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = NUMERO.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let provisoire = donnees.join(format!("{FICHIER}.{}-{n}.tmp", std::process::id()));
    std::fs::write(&provisoire, &texte)?;
    if let Err(e) = std::fs::rename(&provisoire, &f) {
        let _ = std::fs::remove_file(&provisoire);
        return Err(e.into());
    }
    // Vérifier le résultat, pas le retour.
    match lire(donnees) {
        Lecture::Lu(relu) if relu == *c => Ok(()),
        _ => Err(Erreur::Disque("les choix des mises à jour et DLC ne se relisent pas après l'écriture".into())),
    }
}

/// Toute lecture-modification-écriture du fichier se fait sous ce verrou (deux interrupteurs basculés coup sur coup, un
/// lancement qui note son abri pendant ce temps) : aucun choix n'est jamais perdu. Jamais pris deux fois d'affilée.
static VERROU: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn verrou() -> std::sync::MutexGuard<'static, ()> {
    VERROU.lock().unwrap_or_else(|e| e.into_inner())
}

/// La reprise des retraits d'avant est-elle faite (fichier lisible, marque `_migre`) ? Sinon, rien ne s'écrit dans
/// les émulateurs : ce serait rallumer ce que la personne avait coupé.
pub fn migration_faite(donnees: &Path) -> bool {
    matches!(lire(donnees), Lecture::Lu(c) if c.migre)
}

/// Change le choix d'un contenu (`cle` : « maj » ou l'identifiant d'un DLC). Rend vrai si un contenu jusque-là
/// ALLUMÉ vient d'être éteint : les parties de ce jeu sont alors à mettre à l'abri avant sa prochaine partie.
pub fn choisir(donnees: &Path, base: u64, cle: &str, actif: bool) -> Resultat<bool> {
    // Un mod Switch : TOUT changement met les parties à l'abri (avis des experts, 18/11).
    if let Some(m) = cle.strip_prefix(PREFIXE_MOD) {
        return choisir_mods(donnees, base, &[(m.to_string(), actif)]);
    }
    if cle != CLE_MAJ && (!cle.starts_with("dlc:") || cle == TOUS_LES_DLC) {
        return Err(Erreur::Refus("Ce contenu n'est pas reconnu.".into()));
    }
    let _v = verrou();
    let mut tous = match lire(donnees) {
        Lecture::Lu(c) => c,
        // Absent ou abîmé (mis de côté par `ecrire`) : on repart de « tout allumé ».
        _ => Choix::default(),
    };
    let mut c = tous.jeu(base);
    let avant = c.actif(cle);
    if cle == CLE_MAJ {
        c.maj = actif;
    } else if actif {
        c.dlc_eteints.retain(|d| d != cle);
    } else if !c.dlc_eteints.iter().any(|d| d == cle) {
        c.dlc_eteints.push(cle.to_string());
        c.dlc_eteints.sort();
    }
    let eteint = avant && !actif;
    if eteint {
        c.abri = Some(AbriEnAttente { depuis: crate::noyau::maintenant(), faits: vec![] });
    }
    tous.poser(base, c);
    ecrire(donnees, &tous)?;
    Ok(eteint)
}

/// Change d'un coup le choix de plusieurs mods Switch d'un jeu (`changements` : (clé du mod, allumé) ; « allumer A et
/// éteindre B » ne passe jamais par un état intermédiaire). Rend vrai si quelque chose a changé : les parties de ce
/// jeu sont alors à mettre à l'abri avant sa prochaine partie (TOUT changement de mod, allumé comme éteint).
pub fn choisir_mods(donnees: &Path, base: u64, changements: &[(String, bool)]) -> Resultat<bool> {
    if changements.iter().any(|(c, _)| !cle_de_mod_valide(c)) {
        return Err(Erreur::Refus("Ce mod n'est pas reconnu.".into()));
    }
    let _v = verrou();
    let mut tous = match lire(donnees) {
        Lecture::Lu(c) => c,
        _ => Choix::default(),
    };
    let mut c = tous.jeu(base);
    let mut change = false;
    for (m, actif) in changements {
        let cle = format!("{PREFIXE_MOD}{m}");
        if c.actif(&cle) == *actif {
            continue;
        }
        change = true;
        if *actif {
            c.mods_eteints.retain(|x| *x != cle);
        } else {
            c.mods_eteints.push(cle);
            c.mods_eteints.sort();
        }
    }
    if !change {
        return Ok(false);
    }
    c.abri = Some(AbriEnAttente { depuis: crate::noyau::maintenant(), faits: vec![] });
    tous.poser(base, c);
    ecrire(donnees, &tous)?;
    Ok(true)
}

/// Un mod de ce jeu a changé sans que son interrupteur bouge (premier téléchargement, nouvelle révision, retrait) :
/// les parties sont à mettre à l'abri avant la prochaine partie. `oublier` : la clé du mod retiré (son « éteint »
/// n'a plus de sens).
pub fn noter_changement_de_mod(donnees: &Path, base: u64, oublier: Option<&str>) -> Resultat<()> {
    let _v = verrou();
    let mut tous = match lire(donnees) {
        Lecture::Lu(c) => c,
        _ => Choix::default(),
    };
    let mut c = tous.jeu(base);
    if let Some(m) = oublier {
        let cle = format!("{PREFIXE_MOD}{m}");
        c.mods_eteints.retain(|x| *x != cle);
    }
    c.abri = Some(AbriEnAttente { depuis: crate::noyau::maintenant(), faits: vec![] });
    tous.poser(base, c);
    ecrire(donnees, &tous)
}

/// Le choix d'un jeu dont on vient de lire les DLC (`ids` : leurs identifiants ; `complet` : la recherche a tout
/// regardé) : « tous les DLC éteints » (repris d'Eden) devient la liste de ces DLC, écrite une fois — seulement si la
/// recherche est complète (sinon un DLC pas encore lu serait allumé à tort). Rend aussi la phrase à journaliser si le
/// fichier est abîmé (il n'est alors pas réécrit ici).
pub fn choix_resolu(donnees: &Path, base: u64, ids: &[String], complet: bool) -> (ChoixJeu, Option<String>) {
    let _v = verrou();
    let (mut tous, abime) = charger(donnees);
    let mut c = tous.jeu(base);
    if complet && abime.is_none() && !ids.is_empty() && c.dlc_eteints.iter().any(|d| d == TOUS_LES_DLC) {
        let origine = c.origine.remove(TOUS_LES_DLC);
        c.dlc_eteints.retain(|d| d != TOUS_LES_DLC);
        for id in ids {
            if !c.dlc_eteints.contains(id) {
                c.dlc_eteints.push(id.clone());
                if let Some(o) = &origine {
                    c.origine.insert(id.clone(), o.clone());
                }
            }
        }
        c.dlc_eteints.sort();
        // Écrit seulement si le fichier était lisible (jamais par-dessus un fichier abîmé sans l'avoir mis de côté :
        // `ecrire` s'en charge) ; un échec ne change rien au choix rendu.
        tous.poser(base, c.clone());
        let _ = ecrire(donnees, &tous);
    }
    (c, abime)
}

/// Note que les parties de ce jeu sont à l'abri au lieu `lieu` (une fois tous les lieux faits, rien n'est retiré :
/// un autre Eden ou une autre personne de Ryubing aura aussi son abri).
pub fn noter_abri_fait(donnees: &Path, base: u64, lieu: &str) -> Resultat<()> {
    let _v = verrou();
    let Lecture::Lu(mut tous) = lire(donnees) else { return Ok(()) };
    let mut c = tous.jeu(base);
    if let Some(a) = c.abri.as_mut() {
        if !a.faits.iter().any(|f| f == lieu) {
            a.faits.push(lieu.to_string());
            tous.poser(base, c);
            ecrire(donnees, &tous)?;
        }
    }
    Ok(())
}

/// La clé d'un lieu de parties pour `AbriEnAttente::faits` : « eden:<dossier> » ou « ryubing:<racine> ».
pub fn lieu(sorte: &str, dossier: &Path) -> String {
    format!("{sorte}:{}", dossier.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_lowercase())
}

/// Copie `fichiers` (relatifs à `base`) dans un abri neuf `<parent>\<horodatage>-<nom>` (jamais par-dessus un abri
/// existant), puis relit chaque copie (même taille). Aucun abri s'il n'y a aucun fichier. Rien n'est jamais déplacé ni
/// effacé.
pub fn copier_a_l_abri(base: &Path, fichiers: &[String], parent: &Path, nom: &str) -> Resultat<Option<(PathBuf, usize)>> {
    if fichiers.is_empty() {
        return Ok(None);
    }
    let nom = format!("{}-{nom}", crate::noyau::maintenant());
    let mut abri = parent.join(&nom);
    let mut n = 2;
    while abri.exists() {
        abri = parent.join(format!("{nom}-{n}"));
        n += 1;
    }
    crate::installation::copier(base, fichiers, &abri)?;
    let taille = |p: &Path| std::fs::metadata(p).map(|m| m.len()).ok();
    if let Some(r) = fichiers.iter().find(|r| taille(&abri.join(r)).is_none() || taille(&abri.join(r)) != taille(&base.join(r))) {
        return Err(Erreur::Disque(format!("la copie de tes parties est incomplète ({r})")));
    }
    Ok(Some((abri, fichiers.len())))
}

// --- La migration, une seule fois (✅ Seb 17/11) : les retraits déjà faits dans les émulateurs deviennent des
// « éteints ». Les anciens fichiers ne sont JAMAIS effacés ; ils ne sont simplement plus lus ensuite. ---

/// Un retrait d'avant, trouvé dans un émulateur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Retrait {
    /// La mise à jour n'était pas utilisée.
    Maj,
    /// Ce DLC (son identifiant ; `TOUS_LES_DLC` : tous ceux du jeu).
    Dlc(String),
}

/// Où chercher les retraits d'avant.
#[derive(Debug, Clone, Default)]
pub struct Sources {
    /// Les racines de chaque personne dans chaque Ryubing (`Profils\<nom>`).
    pub ryubing: Vec<PathBuf>,
    /// Les dossiers des Eden réglés.
    pub eden: Vec<PathBuf>,
}

/// Reprend une fois les retraits d'avant (`retraits` : (jeu, retrait, d'où il vient)). Rend le nombre d'« éteints »
/// notés, ou `None` si la migration était déjà faite.
pub fn migrer(donnees: &Path, retraits: &[(u64, Retrait, String)]) -> Resultat<Option<usize>> {
    let _v = verrou();
    let mut tous = match lire(donnees) {
        Lecture::Lu(c) if c.migre => return Ok(None),
        Lecture::Lu(c) => c,
        _ => Choix::default(),
    };
    let mut n = 0;
    for (base, r, origine) in retraits {
        let mut c = tous.jeu(*base);
        let cle = match r {
            Retrait::Maj if c.maj => {
                c.maj = false;
                CLE_MAJ.to_string()
            }
            Retrait::Dlc(id) if !c.dlc_eteints.contains(id) => {
                c.dlc_eteints.push(id.clone());
                c.dlc_eteints.sort();
                id.clone()
            }
            _ => continue,
        };
        c.origine.insert(cle, origine.clone());
        tous.poser(*base, c);
        n += 1;
    }
    tous.migre = true;
    ecrire(donnees, &tous)?;
    Ok(Some(n))
}

/// La migration depuis les émulateurs réglés : Ryubing (notes de Frogtend, `dlc.json` et `updates.json` de chaque
/// personne), Eden (`eden-contenus-refuses.json`, `[DisabledAddOns]`). `None` : déjà faite.
pub fn migrer_depuis(donnees: &Path, s: &Sources) -> Resultat<Option<usize>> {
    if matches!(lire(donnees), Lecture::Lu(c) if c.migre) {
        return Ok(None);
    }
    let mut l: Vec<(u64, Retrait, String)> = Vec::new();
    for r in &s.ryubing {
        l.extend(crate::ryubing::retraits_d_avant(r));
    }
    for e in &s.eden {
        l.extend(crate::eden::retraits_d_avant(donnees, e));
    }
    migrer(donnees, &l)
}

/// Pour les tests : des données de Frogtend où la reprise des retraits d'avant est faite (rien d'éteint).
#[cfg(test)]
pub fn donnees_migrees(donnees: &Path) -> PathBuf {
    ecrire(donnees, &Choix { migre: true, ..Default::default() }).unwrap();
    donnees.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seul_l_eteint_est_note_et_le_fichier_se_relit() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(lire(d.path()), Lecture::Absent);
        let base = 0x0100_7EF0_0011_E000u64;
        assert!(charger(d.path()).0.jeu(base).actif("dlc:0100"), "jeu absent : tout allumé");
        // Éteindre un DLC allumé : vrai (abri à faire), noté par son identifiant, base en MAJUSCULES.
        assert!(choisir(d.path(), base, "dlc:01007EF00011F001", false).unwrap());
        let texte = std::fs::read_to_string(chemin(d.path())).unwrap();
        assert!(texte.contains("\"01007EF00011E000\"") && texte.contains("dlc:01007EF00011F001"), "{texte}");
        assert!(!texte.contains("\"maj\""), "la mise à jour allumée n'est pas notée : {texte}");
        assert!(!std::fs::read_dir(d.path()).unwrap().flatten().any(|e| e.file_name().to_string_lossy().ends_with(".tmp")), "aucun provisoire laissé");
        let c = charger(d.path()).0.jeu(base);
        assert!(!c.dlc_actif("dlc:01007EF00011F001") && c.dlc_actif("dlc:01007EF00011F002") && c.maj);
        assert!(c.abri_a_faire("eden:e:\\eden"));
        // Éteindre ce qui l'est déjà : pas un nouvel abri.
        assert!(!choisir(d.path(), base, "dlc:01007EF00011F001", false).unwrap());
        // La mise à jour.
        assert!(choisir(d.path(), base, CLE_MAJ, false).unwrap());
        assert!(!charger(d.path()).0.jeu(base).maj);
        // L'abri fait à un lieu, pas à un autre.
        noter_abri_fait(d.path(), base, "eden:e:\\eden").unwrap();
        let c = charger(d.path()).0.jeu(base);
        assert!(!c.abri_a_faire("eden:e:\\eden") && c.abri_a_faire("ryubing:e:\\r\\profils\\seb"));
        // Tout rallumé : l'abri attend encore (il n'a pas été fait partout).
        choisir(d.path(), base, CLE_MAJ, true).unwrap();
        choisir(d.path(), base, "dlc:01007EF00011F001", true).unwrap();
        assert!(charger(d.path()).0.jeu(base).abri.is_some());
        // Une clé inconnue est refusée.
        assert!(choisir(d.path(), base, "n'importe quoi", false).is_err());
        assert!(choisir(d.path(), base, TOUS_LES_DLC, false).is_err());
    }

    #[test]
    fn un_fichier_abime_ou_trop_gros_est_ignore_puis_mis_de_cote_jamais_ecrase() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(chemin(d.path()), b"{ abime").unwrap();
        assert!(matches!(lire(d.path()), Lecture::Abime(_)));
        let (c, dit) = charger(d.path());
        assert!(c.jeux.is_empty() && dit.unwrap().contains("ignoré"));
        // Écrire par-dessus : l'abîmé est d'abord mis de côté, intact.
        choisir(d.path(), 0x0100_0000_0001_0000, CLE_MAJ, false).unwrap();
        let de_cote: Vec<PathBuf> = std::fs::read_dir(d.path()).unwrap().flatten().map(|e| e.path()).filter(|p| p.to_string_lossy().contains(".abime-")).collect();
        assert_eq!(de_cote.len(), 1);
        assert_eq!(std::fs::read(&de_cote[0]).unwrap(), b"{ abime");
        assert!(matches!(lire(d.path()), Lecture::Lu(_)));
        // Trop gros : jamais lu en entier, ignoré.
        let f = std::fs::File::create(chemin(d.path())).unwrap();
        f.set_len(MAX_OCTETS + 1).unwrap();
        drop(f);
        assert_eq!(lire(d.path()), Lecture::Abime("anormalement gros".into()));
        assert!(charger(d.path()).0.jeu(1).maj);
    }

    #[test]
    fn la_migration_se_fait_une_seule_fois_et_tous_les_dlc_se_resolvent_a_la_lecture() {
        let d = tempfile::tempdir().unwrap();
        let base = 0x0100_7EF0_0011_E000u64;
        let l = vec![
            (base, Retrait::Dlc("dlc:01007EF00011F001".into()), "Ryubing (Sebastien)".to_string()),
            (base, Retrait::Dlc("dlc:01007EF00011F001".into()), "Eden".to_string()),
            (base, Retrait::Maj, "Eden".to_string()),
            (0x0100_0000_0002_0000, Retrait::Dlc(TOUS_LES_DLC.into()), "Eden".to_string()),
        ];
        assert_eq!(migrer(d.path(), &l).unwrap(), Some(3));
        let c = charger(d.path()).0;
        assert!(c.migre);
        let z = c.jeu(base);
        assert!(!z.maj && z.dlc_eteints == ["dlc:01007EF00011F001"] && z.origine["dlc:01007EF00011F001"] == "Ryubing (Sebastien)");
        assert!(z.abri.is_none(), "la migration reprend un état, elle n'éteint rien qui était utilisé");
        // Une seule fois : un nouveau retrait trouvé ensuite n'est plus repris.
        assert_eq!(migrer(d.path(), &[(base, Retrait::Dlc("dlc:X".into()), "Eden".into())]).unwrap(), None);
        // « Tous les DLC » d'Eden : remplacé par les DLC lus, une fois.
        let ids = vec!["dlc:01000000000210001".to_string(), "dlc:01000000000210002".to_string()];
        // Recherche pas finie : rien n'est décidé (un DLC pas encore lu serait allumé à tort).
        let (c, _) = choix_resolu(d.path(), 0x0100_0000_0002_0000, &ids, false);
        assert_eq!(c.dlc_eteints, [TOUS_LES_DLC]);
        assert!(!c.dlc_actif(&ids[0]) && !c.dlc_actif("dlc:pas encore lu"));
        assert!(charger(d.path()).0.jeu(0x0100_0000_0002_0000).dlc_eteints.iter().any(|x| x == TOUS_LES_DLC));
        let (c, _) = choix_resolu(d.path(), 0x0100_0000_0002_0000, &ids, true);
        assert_eq!(c.dlc_eteints, ids);
        assert!(!charger(d.path()).0.jeu(0x0100_0000_0002_0000).dlc_eteints.iter().any(|x| x == TOUS_LES_DLC));
        // Un DLC nouveau ensuite : allumé.
        assert!(charger(d.path()).0.jeu(0x0100_0000_0002_0000).dlc_actif("dlc:nouveau"));
    }

    #[test]
    fn deux_choix_en_meme_temps_sont_tous_deux_gardes() {
        let d = tempfile::tempdir().unwrap();
        let base = 0x0100_7EF0_0011_E000u64;
        for _ in 0..5 {
            let fils: Vec<_> = (0..8)
                .map(|i| {
                    let p = d.path().to_path_buf();
                    std::thread::spawn(move || choisir(&p, base, &format!("dlc:{i:016X}"), false).unwrap())
                })
                .collect();
            for f in fils {
                f.join().unwrap();
            }
            let c = charger(d.path()).0.jeu(base);
            assert_eq!(c.dlc_eteints.len(), 8, "{:?}", c.dlc_eteints);
            for i in 0..8 {
                choisir(d.path(), base, &format!("dlc:{i:016X}"), true).unwrap();
            }
        }
        assert!(!std::fs::read_dir(d.path()).unwrap().flatten().any(|e| e.file_name().to_string_lossy().ends_with(".tmp")));
    }

    #[test]
    fn un_dlc_rallume_apres_la_migration_reste_note() {
        // « Ouvrir Ryubing seul » doit encore voir ce jeu pour remettre ce DLC à `is_enabled: true`.
        let d = tempfile::tempdir().unwrap();
        migrer(d.path(), &[(1 << 56, Retrait::Dlc("dlc:A".into()), "Ryubing".into())]).unwrap();
        choisir(d.path(), 1 << 56, "dlc:A", true).unwrap();
        let c = charger(d.path()).0;
        assert!(c.jeux.contains_key(&cle_du_jeu(1 << 56)) && c.jeu(1 << 56).dlc_actif("dlc:A"));
        assert!(migration_faite(d.path()));
    }

    #[test]
    fn l_abri_copie_et_relit_sans_jamais_ecraser() {
        let d = tempfile::tempdir().unwrap();
        let base = d.path().join("emu");
        std::fs::create_dir_all(base.join("a")).unwrap();
        std::fs::write(base.join("a").join("partie.bin"), b"partie").unwrap();
        let parent = base.join(".frogtend-sauvegardes");
        let (abri, n) = copier_a_l_abri(&base, &["a/partie.bin".to_string()], &parent, "parties-X").unwrap().unwrap();
        assert_eq!(n, 1);
        assert_eq!(std::fs::read(abri.join("a").join("partie.bin")).unwrap(), b"partie");
        assert!(base.join("a").join("partie.bin").is_file(), "jamais déplacée");
        let (abri2, _) = copier_a_l_abri(&base, &["a/partie.bin".to_string()], &parent, "parties-X").unwrap().unwrap();
        assert_ne!(abri, abri2, "jamais par-dessus un abri existant");
        assert_eq!(copier_a_l_abri(&base, &[], &parent, "x").unwrap(), None);
    }
}
