//! Le « pack MAME » (contrat 1.33, besoin 17 ; ✅ Seb 27/10) : tout le romset servi par Firehouse dans UN dossier de ROM
//! (`<disque>\Frogtend\MAME` proposé d'office), téléchargé fichier par fichier, sur l'accord de la personne (tout profil)
//! après une annonce chiffrée ; les CHD seulement si elle les demande (à part). Règles :
//! - le dossier est NEUF (vide) ou déjà un pack de Frogtend (`.frogtend-pack.json`) : jamais le dossier de ROM de la
//!   personne ; jamais un fichier présent qui n'est pas au pack n'est remplacé (laissé, et dit) ;
//! - la place est revérifiée avant CHAQUE fichier ; une coupure s'arrête là et reprend plus tard (les `.part`) ;
//! - un fichier abîmé chez Firehouse (409) est noté et la suite continue ; un jeton refusé arrête tout ;
//! - une CHD partagée par plusieurs machines (même `dossier/nom`) n'est reçue qu'UNE fois ;
//! - rien n'est jamais effacé.

use crate::erreurs::{Erreur, Resultat};
use crate::noyau::Noyau;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// Le registre du pack, dans son dossier : ce que Frogtend y a posé.
pub const REGISTRE: &str = ".frogtend-pack.json";
/// La place gardée libre sur le disque, en plus du fichier à recevoir.
const MARGE: u64 = 1 << 30;

/// Demande d'arrêter le téléchargement en cours (bouton « Arrêter »).
pub static ARRET: AtomicBool = AtomicBool::new(false);

/// Un fichier du pack (une CHD partagée n'y est qu'une fois).
#[derive(Debug, Clone, PartialEq)]
pub struct FichierPack {
    /// `dossier/nom` (ou `nom`) : la clé, et le chemin relatif dans le pack.
    pub cle: String,
    pub taille: u64,
    pub sha1: Option<String>,
    pub route: String,
    pub chd: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Registre {
    pub version: String,
    /// clé → taille reçue.
    pub recus: BTreeMap<String, u64>,
}

/// Ce qu'on annonce avant tout.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct Annonce {
    pub version: String,
    pub a_recevoir: usize,
    pub octets: u64,
    pub deja_la: usize,
    /// Des fichiers du pack d'une version plus ancienne (taille changée) : la mise à jour viendra, rien n'est remplacé.
    pub anciens: usize,
    /// Une réception du pack est déjà en cours sur ce PC.
    pub en_cours: bool,
    /// Place libre sur le disque du pack (inconnue : None).
    pub libre: Option<u64>,
    /// La CHD (disques) : combien, et leur taille (proposées à part).
    pub chd_fichiers: usize,
    pub chd_octets: u64,
}

/// Le bilan d'un téléchargement du pack.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct Bilan {
    pub recus: usize,
    pub octets: u64,
    /// Fichiers laissés : (nom, pourquoi).
    pub laisses: Vec<(String, String)>,
    /// Arrêté avant la fin : pourquoi (la suite reprendra).
    pub arret: Option<String>,
}

/// Les fichiers du romset (`/mame/romset`), CHD comprises ou non, chacun une seule fois (clé `dossier/nom`), seulement
/// ceux dont le nom est sûr (un nom, ou un dossier + un nom : jamais un chemin qui sortirait du pack).
pub fn fichiers(romset: &Value, avec_chd: bool) -> Vec<FichierPack> {
    let mut vus = std::collections::BTreeSet::new();
    let mut l = Vec::new();
    for f in romset["fichiers"].as_array().into_iter().flatten() {
        let chd = f["chd"] == true;
        if chd && !avec_chd {
            continue;
        }
        let Some(nom) = f["nom"].as_str() else { continue };
        let cle = match f["dossier"].as_str().filter(|d| !d.is_empty()) {
            Some(d) => format!("{d}/{nom}"),
            None => nom.to_string(),
        };
        let (Ok(_), Some(route)) = (crate::jeux_pc::chemin_de_fichier_sur(&cle), f["url"].as_str().and_then(|u| u.strip_prefix(crate::firehouse::PREFIXE))) else { continue };
        if !vus.insert(cle.to_lowercase()) {
            continue;
        }
        l.push(FichierPack { cle, taille: f["taille"].as_u64().unwrap_or(0), sha1: f["sha1"].as_str().map(String::from), route: route.to_string(), chd });
    }
    l
}

/// Le registre du pack (vide s'il n'y en a pas encore).
pub fn lire_registre(dossier: &Path) -> Registre {
    std::fs::read(dossier.join(REGISTRE)).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default()
}

fn ecrire_registre(dossier: &Path, r: &Registre) -> Resultat<()> {
    let f = dossier.join(REGISTRE);
    let provisoire = dossier.join(format!("{REGISTRE}.frogtend"));
    std::fs::write(&provisoire, serde_json::to_vec(r).unwrap_or_default())?;
    std::fs::rename(&provisoire, &f)?;
    Ok(())
}

/// Le dossier du pack est-il sûr ? Un chemin complet, sans « ; » (MAME ne le lirait pas), NEUF (absent ou vide) ou déjà
/// un pack de Frogtend — jamais un dossier qui a déjà autre chose (le dossier de ROM de la personne).
pub fn verifier_dossier(dossier: &Path) -> Resultat<()> {
    if !dossier.is_absolute() || dossier.parent().is_none() {
        return Err(Erreur::Refus("Choisis un dossier complet (pas la racine d'un disque) pour le pack MAME.".into()));
    }
    crate::installation::verifier_rompath(&dossier.to_string_lossy())?;
    if dossier.join(REGISTRE).is_file() {
        return Ok(());
    }
    if std::fs::read_dir(dossier).is_ok_and(|mut l| l.next().is_some()) {
        return Err(Erreur::Refus(format!(
            "Le dossier « {} » contient déjà autre chose : le pack MAME a besoin d'un dossier à lui (Frogtend n'écrit jamais au milieu de tes fichiers). Choisis un dossier vide ou nouveau.",
            dossier.display()
        )));
    }
    Ok(())
}

/// Le dossier proposé d'office : `<disque>\Frogtend\MAME` sur le disque qui a le plus de place (comme pour les jeux).
pub fn dossier_propose() -> Option<PathBuf> {
    let d = crate::jeux_pc::disques_locaux();
    crate::jeux_pc::disque_propose(&d).map(|x| Path::new(&x.racine).join("Frogtend").join("MAME"))
}

/// L'annonce : ce qu'il reste à recevoir dans ce dossier (et la place libre), et les CHD à part.
pub fn annoncer(romset: &Value, dossier: &Path, avec_chd: bool) -> Annonce {
    let r = lire_registre(dossier);
    let tous = fichiers(romset, avec_chd);
    let manque: Vec<&FichierPack> = tous.iter().filter(|f| !r.recus.contains_key(&f.cle)).collect();
    let anciens = tous.iter().filter(|f| r.recus.get(&f.cle).is_some_and(|t| *t != f.taille)).count();
    let chd: Vec<FichierPack> = fichiers(romset, true).into_iter().filter(|f| f.chd).collect();
    let chd_manquantes: Vec<&FichierPack> = chd.iter().filter(|f| !r.recus.contains_key(&f.cle)).collect();
    let mut cherche = dossier.to_path_buf();
    while !cherche.is_dir() {
        match cherche.parent() {
            Some(p) => cherche = p.to_path_buf(),
            None => break,
        }
    }
    Annonce {
        version: romset["version"].as_str().unwrap_or("").to_string(),
        a_recevoir: manque.len(),
        octets: manque.iter().map(|f| f.taille).sum(),
        deja_la: tous.len() - manque.len() - anciens,
        anciens,
        en_cours: EN_COURS.load(Ordering::SeqCst),
        libre: crate::jeux_pc::place_libre(&cherche),
        chd_fichiers: chd_manquantes.len(),
        chd_octets: chd_manquantes.iter().map(|f| f.taille).sum(),
    }
}

impl Noyau {
    /// Le romset de Firehouse pour ce profil (`/mame/romset`, contrat 1.33) : gardé dans le dossier du profil avec son
    /// ETag (il dépend du grade), relu à chaque fois ; Firehouse injoignable : celui gardé.
    pub async fn romset_mame(&self) -> Resultat<Value> {
        let s = self.session().await?;
        let (fichier, f_etag) = (s.dossier.join("mame-romset.json"), s.dossier.join("mame-romset.etag"));
        let gardee = || std::fs::read(&fichier).ok().and_then(|o| serde_json::from_slice::<Value>(&o).ok());
        let etag = std::fs::read_to_string(&f_etag).ok().map(|e| e.trim().to_string()).filter(|e| !e.is_empty() && fichier.is_file());
        let reponse = match &s.source {
            crate::source::Source::Firehouse(c) => c.obtenir_gros("/mame/romset", etag.as_deref(), 100 << 20).await,
            crate::source::Source::Simulee => return Err(Erreur::Refus("Pas de pack MAME en mode simulé.".into())),
        };
        match reponse {
            Ok(crate::firehouse::Reponse::NonModifie) => gardee().ok_or_else(|| Erreur::Serveur("La liste du pack MAME gardée est illisible.".into())),
            Ok(crate::firehouse::Reponse::Corps { octets, etag, .. }) => {
                let v: Value = serde_json::from_slice(&octets).map_err(|_| Erreur::Serveur("Firehouse a envoyé une liste du pack MAME illisible.".into()))?;
                s.garder_fichier(&fichier, &octets)?;
                if let Some(e) = etag {
                    s.garder_fichier(&f_etag, e.as_bytes())?;
                }
                Ok(v)
            }
            Err(Erreur::Reseau(_) | Erreur::Serveur(_)) if gardee().is_some() => Ok(gardee().unwrap_or_default()),
            Err(e) => Err(e),
        }
    }

    /// Télécharge le pack dans `dossier` (déjà vérifié, accord donné), fichier par fichier, en reprenant où il en était.
    /// Une seule réception à la fois sur ce PC. Chaque fichier est reçu À L'ÉCART (`.frogtend-recu\`), vérifié, puis
    /// posé SANS JAMAIS ÉCRASER (un lien dur, qui échoue si la place est prise).
    pub async fn telecharger_pack_mame(&self, dossier: &Path, avec_chd: bool, progres: &(dyn Fn(usize, usize, u64) + Send + Sync)) -> Resultat<Bilan> {
        if EN_COURS.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
            return Err(Erreur::Refus("Le pack MAME est déjà en train d'arriver : attends la fin (ou arrête-le).".into()));
        }
        struct Libere;
        impl Drop for Libere {
            fn drop(&mut self) {
                EN_COURS.store(false, Ordering::SeqCst);
            }
        }
        let _libere = Libere;
        ARRET.store(false, Ordering::SeqCst);
        verifier_dossier(dossier)?;
        let romset = self.romset_mame().await?;
        let s = self.session().await?;
        std::fs::create_dir_all(dossier)?;
        let ecart = dossier.join(".frogtend-recu");
        let mut reg = lire_registre(dossier);
        reg.version = romset["version"].as_str().unwrap_or("").to_string();
        // Écrit tout de suite : le dossier est dès maintenant « un pack de Frogtend » (une coupure pendant le premier
        // gros fichier ne le fait jamais refuser ensuite comme « contient déjà autre chose »).
        ecrire_registre_a_part(dossier, &reg).await?;
        let mut b = Bilan::default();
        // Les fichiers du pack d'une AUTRE version (taille changée) : la mise à jour viendra ; jamais remplacés ici.
        let tous = fichiers(&romset, avec_chd);
        for f in tous.iter().filter(|f| reg.recus.get(&f.cle).is_some_and(|t| *t != f.taille)) {
            b.laisses.push((f.cle.clone(), "version plus ancienne dans ton pack : la mise à jour du pack viendra (rien n'est remplacé)".into()));
        }
        let a_faire: Vec<&FichierPack> = tous.iter().filter(|f| !reg.recus.contains_key(&f.cle)).collect();
        let total = a_faire.len();
        let mut ecrit_le = std::time::Instant::now();
        let mut a_ecrire = false;
        for (i, f) in a_faire.iter().enumerate() {
            if a_ecrire && (ecrit_le.elapsed().as_secs() >= 20) {
                ecrire_registre_a_part(dossier, &reg).await?;
                (ecrit_le, a_ecrire) = (std::time::Instant::now(), false);
            }
            progres(i, total, b.octets);
            if ARRET.load(Ordering::SeqCst) {
                b.arret = Some("arrêté à ta demande".into());
                break;
            }
            let rel = crate::jeux_pc::chemin_de_fichier_sur(&f.cle)?;
            let cible = dossier.join(&rel);
            // Déjà là sans être noté (une coupure entre la pose et le registre) : à la bonne taille — et à la bonne
            // empreinte si Firehouse la donne —, il est inscrit tel quel ; sinon laissé. Jamais remplacé.
            if cible.exists() {
                let taille = std::fs::metadata(&cible).map(|m| m.len()).unwrap_or(0);
                if taille == f.taille && empreinte_conforme(&cible, f.sha1.clone()).await {
                    reg.recus.insert(f.cle.clone(), taille);
                    a_ecrire = true;
                } else {
                    b.laisses.push((f.cle.clone(), "un autre fichier de ce nom est déjà là : laissé tel quel".into()));
                }
                continue;
            }
            let libre = crate::jeux_pc::place_libre(dossier).unwrap_or(u64::MAX);
            if libre < f.taille + MARGE {
                b.arret = Some(format!("plus assez de place sur le disque (il faudrait {} Go pour le fichier suivant)", (f.taille + MARGE).div_ceil(1 << 30)));
                break;
            }
            let recu = ecart.join(&rel);
            if let Some(p) = recu.parent() {
                std::fs::create_dir_all(p)?;
            }
            let deja = b.octets;
            match s.source.telecharger_route(&f.route, &recu, &|o, _| progres(i, total, deja + o)).await {
                Ok(_) => {
                    let taille = std::fs::metadata(&recu).map(|m| m.len()).unwrap_or(0);
                    if taille != f.taille || !empreinte_conforme(&recu, f.sha1.clone()).await {
                        let _ = std::fs::remove_file(&recu); // le NÔTRE, reçu à l'instant à l'écart : jamais gardé abîmé
                        b.laisses.push((f.cle.clone(), "arrivé abîmé : pas gardé (il sera redemandé la prochaine fois)".into()));
                        continue;
                    }
                    if let Some(p) = cible.parent() {
                        std::fs::create_dir_all(p)?;
                    }
                    match poser_sans_ecraser(&recu, &cible) {
                        Ok(()) => {
                            reg.recus.insert(f.cle.clone(), taille);
                            a_ecrire = true;
                            b.recus += 1;
                            b.octets += taille;
                        }
                        Err(_) => {
                            // Notre copie, reçue à l'écart et vérifiée : retirée (jamais laissée à traîner, où un
                            // import la prendrait pour une ROM en double).
                            let _ = std::fs::remove_file(&recu);
                            b.laisses.push((f.cle.clone(), "un autre fichier de ce nom est arrivé entre-temps : laissé tel quel".into()));
                        }
                    }
                }
                Err(Erreur::Conflit(m)) => b.laisses.push((f.cle.clone(), format!("abîmé chez Firehouse ({m})"))),
                // Plus servi à ce profil (caché, réuni à une autre fiche) : noté, la suite continue.
                Err(Erreur::Introuvable(_) | Erreur::Fusionne { .. }) => b.laisses.push((f.cle.clone(), "plus servi par Firehouse".into())),
                Err(e @ Erreur::JetonRefuse(_)) => {
                    ecrire_registre_a_part(dossier, &reg).await?;
                    return Err(e);
                }
                Err(e) => {
                    b.arret = Some(e.to_string());
                    break;
                }
            }
        }
        ecrire_registre_a_part(dossier, &reg).await?;
        let _ = std::fs::remove_dir(&ecart); // seulement s'il est vide (les `.part` d'une réception arrêtée restent)
        progres(total, total, b.octets);
        self.journaliser(&format!(
            "pack MAME ({}) : {} fichier(s) reçu(s), {} laissé(s){}",
            dossier.display(),
            b.recus,
            b.laisses.len(),
            b.arret.as_deref().map(|a| format!(", arrêté : {a}")).unwrap_or_default()
        ));
        Ok(b)
    }
}

/// Ce que la mise en ludothèque du pack a donné.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct BilanLudotheque {
    /// Jeux ajoutés (un par fiche, ses clones en versions).
    pub jeux: usize,
    pub versions: usize,
    pub deja: usize,
    /// Fiches que ce profil ne voit pas : leurs jeux ne sont pas montrés (seulement comptés).
    pub sans_fiche: usize,
}

impl Noyau {
    /// Étape B : met les jeux REÇUS du pack dans la ludothèque — un jeu importé par FICHE de Firehouse (ses clones en
    /// versions, la machine d'origine par défaut), relié « à coup sûr » à sa fiche (c'est Firehouse qui a servi ces
    /// fichiers). Rien n'est copié ni lu ; une fiche que ce profil ne voit pas n'est jamais montrée.
    pub async fn ludotheque_pack_mame(&self, dossier: &Path) -> Resultat<BilanLudotheque> {
        let romset = self.romset_mame().await?;
        let reg = lire_registre(dossier);
        let liste = self.liste_mame().await.ok().flatten().unwrap_or_default();
        // media_id → [(nom du zip, machine, telechargement_id)] des zips reçus.
        let mut par_fiche: BTreeMap<i64, Vec<(String, String, Option<i64>)>> = BTreeMap::new();
        for f in romset["fichiers"].as_array().into_iter().flatten() {
            let (Some(media), Some(nom)) = (f["media_id"].as_i64(), f["nom"].as_str()) else { continue };
            if f["chd"] == true || !reg.recus.contains_key(nom) || crate::jeux_pc::nom_de_fichier_sur(nom).is_err() {
                continue;
            }
            par_fiche.entry(media).or_default().push((nom.to_string(), f["machine"].as_str().unwrap_or("").to_string(), f["telechargement_id"].as_i64()));
        }
        let s = self.session().await?;
        let mut b = BilanLudotheque::default();
        let mut a_importer = Vec::new();
        let mut fiches = Vec::new();
        for (media, zips) in &par_fiche {
            let Some(carte) = s.verrou().jeu(*media)? else {
                b.sans_fiche += 1;
                continue;
            };
            for (nom, _, _) in zips {
                a_importer.push(crate::import_local::JeuAImporter {
                    titre: carte.titre.clone(),
                    plateforme: carte.plateforme.clone(),
                    dossier: dossier.to_string_lossy().into(),
                    fichier: Some(nom.clone()),
                    programme: None,
                    arguments: vec![],
                    annee: carte.annee,
                    editeur: None,
                    genres: carte.genres.clone(),
                });
            }
            fiches.push((*media, carte, zips.clone()));
        }
        drop(s);
        // Les jeux déjà là AVANT ce passage (leur version par défaut est celle choisie par la personne : jamais changée).
        let avant: std::collections::HashSet<i64> = self.registre().tous()?.into_iter().map(|j| j.id).collect();
        let bi = self.importer_locaux(&a_importer).await?;
        (b.jeux, b.versions, b.deja) = (bi.ajoutes, bi.versions, bi.deja);
        // Chaque fichier reçu → le jeu importé qui le porte (créé à l'instant, ou importé avant à la main depuis le pack).
        let mut par_chemin: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
        for j in self.registre().tous()?.into_iter().filter(crate::import_local::est_importe) {
            if let Some(i) = &j.installation {
                let mut chemins: Vec<String> = i.versions.iter().map(|v| v.chemin.clone()).collect();
                chemins.extend(i.fichier_du_jeu.as_ref().map(|f| Path::new(&i.dossier).join(f).to_string_lossy().to_string()));
                for c in chemins {
                    par_chemin.insert(c.to_lowercase(), j.id);
                }
            }
        }
        // Par jeu importé : ses fichiers du pack et leur fiche. Relié seulement si TOUS viennent d'une même fiche (deux
        // fiches au même titre ne sont jamais mélangées) et s'il n'est pas déjà relié.
        let mut par_jeu: BTreeMap<i64, Vec<(String, i64, Option<i64>, String)>> = BTreeMap::new();
        for (media, _, zips) in &fiches {
            for (nom, machine, td) in zips {
                let chemin = dossier.join(nom).to_string_lossy().to_string();
                if let Some(local) = par_chemin.get(&chemin.to_lowercase()) {
                    par_jeu.entry(*local).or_default().push((chemin, *media, *td, machine.clone()));
                }
            }
        }
        let quand = crate::noyau::maintenant();
        let relies: std::collections::HashSet<i64> = self.registre().correspondances()?.into_iter().map(|(l, ..)| l).collect();
        for (local, l) in par_jeu {
            let media = l[0].1;
            if l.iter().all(|x| x.1 == media) && !relies.contains(&local) {
                let v: Vec<_> = l.iter().map(|(c, m, td, _)| (c.clone(), *m, *td, "exact".to_string(), Some("pack".to_string()))).collect();
                self.registre().noter_correspondance(local, &v, "auto", &quand)?;
            }
            // La machine d'origine (sans « clone de ») par défaut — seulement pour un jeu CRÉÉ à ce passage.
            if avant.contains(&local) {
                continue;
            }
            let parent = l.iter().find(|(.., m)| liste.get(&m.to_lowercase()).is_some_and(|x| x.clone_de.is_none())).map(|(c, ..)| c.clone());
            let Some(mut j) = self.registre().jeu(local)? else { continue };
            if let (Some(p), Some(i)) = (parent, j.installation.as_mut()) {
                let nom = Path::new(&p).file_name().map(|n| n.to_string_lossy().to_string());
                if i.fichier_du_jeu != nom {
                    i.fichier_du_jeu = nom;
                    let i = i.clone();
                    self.registre().changer_installation(local, Some(&i))?;
                }
            }
        }
        self.journaliser(&format!("pack MAME : {} jeu(x) mis en ludothèque, {} version(s), {} déjà là", b.jeux, b.versions, b.deja));
        Ok(b)
    }
}

/// Une réception du pack en cours sur ce PC (une seule à la fois).
pub static EN_COURS: AtomicBool = AtomicBool::new(false);

/// L'empreinte sha1 d'un fichier vaut-elle celle donnée par Firehouse ? Sans empreinte donnée : oui (la taille seule
/// est vérifiée). Lue hors du fil asynchrone (plusieurs Go).
async fn empreinte_conforme(f: &Path, sha1: Option<String>) -> bool {
    let Some(attendu) = sha1 else { return true };
    let f = f.to_path_buf();
    tokio::task::spawn_blocking(move || crate::bios::sha1_fichier(&f).is_ok_and(|e| e.eq_ignore_ascii_case(&attendu))).await.unwrap_or(false)
}

/// Pose un fichier reçu à sa place SANS JAMAIS ÉCRASER : un lien dur échoue si la place est prise (l'appelant retire
/// alors sa copie reçue à l'écart) ; sur un disque qui n'en veut pas, on revérifie juste avant de déplacer.
fn poser_sans_ecraser(recu: &Path, cible: &Path) -> std::io::Result<()> {
    match std::fs::hard_link(recu, cible) {
        Ok(()) => std::fs::remove_file(recu),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Err(e),
        Err(_) if cible.exists() => Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists)),
        Err(_) => std::fs::rename(recu, cible),
    }
}

/// Le registre écrit hors du fil asynchrone (compact).
async fn ecrire_registre_a_part(dossier: &Path, r: &Registre) -> Resultat<()> {
    let (d, r) = (dossier.to_path_buf(), r.clone());
    tokio::task::spawn_blocking(move || ecrire_registre(&d, &r)).await.map_err(|_| Erreur::Disque("Le registre du pack n'a pas pu être écrit.".into()))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn romset() -> Value {
        json!({"version": "0.289", "fichiers": [
            {"media_id": 1, "telechargement_id": -1, "machine": "sf2", "nom": "sf2.zip", "taille": 3, "sha1": null, "dossier": null, "chd": false, "url": "/api/jeux/v1/fichier/1/-1/0"},
            {"media_id": 2, "telechargement_id": -2, "machine": "ddr2m", "nom": "ddr2m.zip", "taille": 2, "sha1": null, "dossier": null, "chd": false, "url": "/api/jeux/v1/fichier/2/-2/0"},
            {"media_id": 2, "telechargement_id": -2, "machine": "ddr2m", "nom": "gn895jaa.chd", "taille": 100, "sha1": null, "dossier": "ddr2m", "chd": true, "url": "/api/jeux/v1/fichier/2/-2/1"},
            {"media_id": 3, "telechargement_id": -3, "machine": "ddr2mc", "nom": "gn895jaa.chd", "taille": 100, "sha1": null, "dossier": "ddr2m", "chd": true, "url": "/api/jeux/v1/fichier/3/-3/1"},
            {"media_id": 4, "telechargement_id": -4, "machine": "x", "nom": "../evasion.zip", "taille": 1, "sha1": null, "dossier": null, "chd": false, "url": "/api/jeux/v1/fichier/4/-4/0"}
        ]})
    }

    #[test]
    fn une_chd_partagee_n_est_comptee_qu_une_fois_et_un_chemin_douteux_jamais() {
        let sans = fichiers(&romset(), false);
        assert_eq!(sans.iter().map(|f| f.cle.as_str()).collect::<Vec<_>>(), ["sf2.zip", "ddr2m.zip"], "jamais « ../evasion.zip »");
        let avec = fichiers(&romset(), true);
        assert_eq!(avec.iter().filter(|f| f.chd).count(), 1, "la CHD partagée : une seule fois");
        assert_eq!(avec.iter().find(|f| f.chd).unwrap().cle, "ddr2m/gn895jaa.chd");
    }

    #[test]
    fn le_pack_veut_un_dossier_a_lui() {
        let d = tempfile::tempdir().unwrap();
        let neuf = d.path().join("MAME");
        assert!(verifier_dossier(&neuf).is_ok(), "absent : il sera créé");
        std::fs::create_dir_all(&neuf).unwrap();
        assert!(verifier_dossier(&neuf).is_ok(), "vide");
        std::fs::write(neuf.join("mes-roms.zip"), b"x").unwrap();
        assert!(matches!(verifier_dossier(&neuf), Err(Erreur::Refus(_))), "le dossier de ROM de la personne : jamais");
        std::fs::write(neuf.join(REGISTRE), b"{}").unwrap();
        assert!(verifier_dossier(&neuf).is_ok(), "déjà un pack de Frogtend");
        assert!(verifier_dossier(&d.path().join("a;b")).is_err(), "MAME ne lit pas « ; »");
        assert!(verifier_dossier(Path::new("relatif")).is_err());
    }

    #[test]
    fn l_annonce_dit_ce_qu_il_reste_et_les_chd_a_part() {
        let d = tempfile::tempdir().unwrap();
        let a = annoncer(&romset(), d.path(), false);
        assert_eq!((a.version.as_str(), a.a_recevoir, a.octets, a.deja_la, a.chd_fichiers, a.chd_octets), ("0.289", 2, 5, 0, 1, 100));
        ecrire_registre(d.path(), &Registre { version: "0.289".into(), recus: BTreeMap::from([("sf2.zip".into(), 3)]) }).unwrap();
        let a = annoncer(&romset(), d.path(), false);
        assert_eq!((a.a_recevoir, a.octets, a.deja_la), (1, 2, 1));
    }

    #[test]
    fn la_pose_n_ecrase_jamais() {
        let d = tempfile::tempdir().unwrap();
        let (recu, cible) = (d.path().join("recu.zip"), d.path().join("sf2.zip"));
        std::fs::write(&recu, b"du pack").unwrap();
        std::fs::write(&cible, b"le sien").unwrap();
        assert!(poser_sans_ecraser(&recu, &cible).is_err());
        assert_eq!(std::fs::read(&cible).unwrap(), b"le sien");
        std::fs::remove_file(&cible).unwrap();
        poser_sans_ecraser(&recu, &cible).unwrap();
        assert_eq!(std::fs::read(&cible).unwrap(), b"du pack");
        assert!(!recu.exists());
    }

    /// Les réceptions du pack ne se font qu'une à la fois sur le PC (`EN_COURS`, global) : ces essais passent un par un.
    static UN_A_LA_FOIS: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[tokio::test]
    async fn le_pack_se_telecharge_sans_jamais_ecraser_et_reprend() {
        let _un = UN_A_LA_FOIS.lock().unwrap_or_else(|e| e.into_inner());
        use httpmock::prelude::*;
        let s = MockServer::start();
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/mame/romset");
            t.status(200).header("etag", "e1").json_body(romset());
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/fichier/1/-1/0");
            t.status(200).body(b"SF2");
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/fichier/2/-2/0");
            t.status(409).json_body(json!({"ok": false, "erreur": "fichier abîmé"}));
        });
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &crate::noyau::Connexion { adresse: s.base_url(), simule: false }).await.unwrap();
        let pack = d.path().join("Pack");
        let b = n.telecharger_pack_mame(&pack, false, &|_, _, _| {}).await.unwrap();
        assert_eq!(b.recus, 1);
        assert_eq!(std::fs::read(pack.join("sf2.zip")).unwrap(), b"SF2");
        assert_eq!(b.laisses.len(), 1, "le fichier abîmé chez Firehouse : noté, la suite a continué");
        assert_eq!(lire_registre(&pack).recus.get("sf2.zip"), Some(&3));
        // La fois suivante : sf2 n'est pas redemandé ; un ddr2m.zip posé à la main n'est JAMAIS remplacé.
        std::fs::write(pack.join("ddr2m.zip"), b"le mien").unwrap();
        let b = n.telecharger_pack_mame(&pack, false, &|_, _, _| {}).await.unwrap();
        assert_eq!(b.recus, 0);
        assert_eq!(std::fs::read(pack.join("ddr2m.zip")).unwrap(), b"le mien");
        assert!(b.laisses[0].1.contains("déjà là"));
        // Une coupure entre la pose et le registre : sf2 complet mais oublié du registre → inscrit tel quel, jamais
        // redemandé ni signalé comme étranger.
        let mut r = lire_registre(&pack);
        r.recus.clear();
        ecrire_registre(&pack, &r).unwrap();
        std::fs::remove_file(pack.join("ddr2m.zip")).unwrap();
        n.telecharger_pack_mame(&pack, false, &|_, _, _| {}).await.unwrap();
        assert_eq!(lire_registre(&pack).recus.get("sf2.zip"), Some(&3));
        // Une seule réception à la fois.
        EN_COURS.store(true, Ordering::SeqCst);
        assert!(matches!(n.telecharger_pack_mame(&pack, false, &|_, _, _| {}).await, Err(Erreur::Refus(_))));
        EN_COURS.store(false, Ordering::SeqCst);
    }

    #[tokio::test]
    async fn un_pack_coupe_au_premier_fichier_garde_son_dossier() {
        let _un = UN_A_LA_FOIS.lock().unwrap_or_else(|e| e.into_inner());
        use httpmock::prelude::*;
        let s = MockServer::start();
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/mame/romset");
            t.status(200).json_body(romset());
        });
        // Firehouse tombe dès le premier fichier : la réception s'arrête (elle reprendra).
        s.mock(|w, t| {
            w.method(GET).path_matches(httpmock::Regex::new("/api/jeux/v1/fichier/.*").unwrap());
            t.status(503);
        });
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &crate::noyau::Connexion { adresse: s.base_url(), simule: false }).await.unwrap();
        let pack = d.path().join("Pack");
        let b = n.telecharger_pack_mame(&pack, false, &|_, _, _| {}).await.unwrap();
        assert!(b.arret.is_some());
        assert!(pack.join(REGISTRE).is_file(), "le registre est écrit dès le début");
        assert!(verifier_dossier(&pack).is_ok(), "jamais refusé ensuite comme « contient déjà autre chose »");
    }

    #[tokio::test]
    async fn les_jeux_du_pack_entrent_un_par_fiche_relies_a_leur_fiche() {
        use httpmock::prelude::*;
        let s = MockServer::start();
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/mame/romset");
            t.status(200).json_body(json!({"version": "0.289", "fichiers": [
                {"media_id": 10, "telechargement_id": -10, "machine": "puckman", "nom": "puckman.zip", "taille": 1, "chd": false, "url": "/api/jeux/v1/fichier/10/-10/0"},
                {"media_id": 10, "telechargement_id": -10, "machine": "pacman", "nom": "pacman.zip", "taille": 1, "chd": false, "url": "/api/jeux/v1/fichier/10/-10/1"},
                {"media_id": 20, "telechargement_id": -20, "machine": "cache", "nom": "cache.zip", "taille": 1, "chd": false, "url": "/api/jeux/v1/fichier/20/-20/0"},
                {"media_id": 30, "telechargement_id": -30, "machine": "galaga", "nom": "galaga.zip", "taille": 1, "chd": false, "url": "/api/jeux/v1/fichier/30/-30/0"}
            ]}));
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/mame/machines");
            t.status(200).json_body(serde_json::from_str::<Value>(include_str!("../fixtures/simule/mame-machines-133.json")).unwrap());
        });
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &crate::noyau::Connexion { adresse: s.base_url(), simule: false }).await.unwrap();
        // Ce profil voit la fiche 10 (Pac-Man), pas la 20.
        n.session()
            .await
            .unwrap()
            .verrou()
            .remplacer(
                &[],
                &[
                    crate::ludotheque::JeuResume { id: 10, titre: "Pac-Man".into(), plateforme: "Arcade".into(), ..Default::default() },
                    crate::ludotheque::JeuResume { id: 30, titre: "Galaga".into(), plateforme: "Arcade".into(), ..Default::default() },
                ],
                "x",
            )
            .unwrap();
        let pack = d.path().join("Pack");
        std::fs::create_dir_all(&pack).unwrap();
        for z in ["puckman.zip", "pacman.zip", "cache.zip", "galaga.zip"] {
            std::fs::write(pack.join(z), b"z").unwrap();
        }
        ecrire_registre(&pack, &Registre { version: "0.289".into(), recus: [("puckman.zip", 1), ("pacman.zip", 1), ("cache.zip", 1), ("galaga.zip", 1)].map(|(k, v)| (k.to_string(), v)).into() }).unwrap();
        // Galaga a été importé AVANT à la main (import MAME : titre de la machine) : il doit être relié à sa fiche.
        n.importer_locaux(&[crate::import_local::JeuAImporter {
            titre: "Galaga (Namco rev. B)".into(),
            plateforme: "Arcade".into(),
            dossier: pack.to_string_lossy().into(),
            fichier: Some("galaga.zip".into()),
            programme: None,
            arguments: vec![],
            annee: None,
            editeur: None,
            genres: vec![],
        }])
        .await
        .unwrap();
        let galaga = crate::ludotheque::id_boutique("local", &crate::import_local::cle_de_jeu("Arcade", "Galaga (Namco rev. B)"));
        let b = n.ludotheque_pack_mame(&pack).await.unwrap();
        assert_eq!((b.jeux, b.sans_fiche), (1, 1), "un jeu pour la fiche visible ; rien de la fiche cachée");
        let local = crate::ludotheque::id_boutique("local", &crate::import_local::cle_de_jeu("Arcade", "Pac-Man"));
        let j = n.registre().jeu(local).unwrap().unwrap();
        let i = j.installation.unwrap();
        assert_eq!(i.fichier_du_jeu.as_deref(), Some("puckman.zip"), "la machine d'origine par défaut");
        assert_eq!(i.versions.len(), 2, "le clone en version");
        assert!(i.versions.iter().all(|v| Path::new(&v.chemin).starts_with(&pack)), "dans le dossier du pack");
        assert!(n.registre().correspondances().unwrap().iter().any(|(l, m, _)| *l == local && *m == 10), "relié à sa fiche");
        assert!(n.registre().correspondances().unwrap().iter().any(|(l, m, _)| *l == galaga && *m == 30), "l'import d'avant relié aussi");
        // La personne choisit le clone ; un 2e passage ne défait jamais son choix, ni ne met rien en double.
        let mut i2 = n.registre().jeu(local).unwrap().unwrap().installation.unwrap();
        i2.fichier_du_jeu = Some("pacman.zip".into());
        n.registre().changer_installation(local, Some(&i2)).unwrap();
        let b = n.ludotheque_pack_mame(&pack).await.unwrap();
        assert_eq!(b.jeux, 0);
        assert_eq!(n.registre().jeu(local).unwrap().unwrap().installation.unwrap().fichier_du_jeu.as_deref(), Some("pacman.zip"));
    }

    #[tokio::test]
    async fn un_fichier_plus_servi_ne_bloque_pas_la_suite() {
        let _un = UN_A_LA_FOIS.lock().unwrap_or_else(|e| e.into_inner());
        use httpmock::prelude::*;
        let s = MockServer::start();
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/mame/romset");
            t.status(200).json_body(romset());
        });
        // sf2 (le 1er) n'est plus servi à ce profil : 404 ; ddr2m arrive quand même.
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/fichier/1/-1/0");
            t.status(404).json_body(json!({"ok": false, "erreur": "introuvable"}));
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/fichier/2/-2/0");
            t.status(200).body(b"DD");
        });
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &crate::noyau::Connexion { adresse: s.base_url(), simule: false }).await.unwrap();
        let pack = d.path().join("Pack");
        let b = n.telecharger_pack_mame(&pack, false, &|_, _, _| {}).await.unwrap();
        assert_eq!((b.recus, b.arret.clone()), (1, None));
        assert_eq!(std::fs::read(pack.join("ddr2m.zip")).unwrap(), b"DD");
        assert!(!pack.join(".frogtend-recu").exists(), "rien ne reste à l'écart");
    }
}
