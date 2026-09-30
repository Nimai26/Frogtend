//! La sauvegarde d'un profil : « tout ce qui ne se retélécharge pas » (lot 3 bis, décision de Seb).
//!
//! Dans `<partage de la personne>\<profil>\<PC>\` :
//! - `configuration.json` : réglages du PC et du profil, émulateurs installés ;
//! - `bibliotheque.json` : la LISTE des jeux (version, emplacement, installation, temps de jeu) — pas les jeux ;
//! - `parties\` : les parties et codes de triche de TOUS les jeux — pour un jeu installé, les fichiers changés depuis
//!   son installation ; pour chaque émulateur, le dossier du profil (`<émulateur>\Profils\<profil>\`) ;
//!   `parties\manifeste.json` donne l'empreinte (sha256) de chacun : seul ce qui a changé est renvoyé ;
//! - `derniere-sauvegarde.json` : quand, depuis quel PC, combien.
//!
//! Frogtend écrit la version COURANTE ; l'historique est gardé par le serveur (instantanés ZFS).

use crate::erreurs::{Erreur, Resultat};
use crate::installation::{changes_depuis, fichiers_de, Manifeste};
use crate::noyau::{maintenant, Noyau};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Un fichier à sauvegarder : où il est sur le PC, où il va dans `parties\`.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub source: PathBuf,
    /// Chemin dans `parties\`, séparateur `/`.
    pub relatif: String,
}

/// Le manifeste des parties sauvegardées : empreinte et taille de chaque fichier.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ManifesteParties {
    pub fichiers: BTreeMap<String, (String, u64)>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Bilan {
    pub envoyes: usize,
    pub inchanges: usize,
    pub retires: usize,
    /// Octets envoyés cette fois-ci.
    pub octets: u64,
    /// Nombre et taille de toutes les parties sauvegardées.
    pub fichiers: usize,
    pub taille: u64,
    pub date: String,
    pub pc: String,
    pub dossier: String,
}

/// Ce qu'il faut sauvegarder pour un profil.
pub struct Contenu {
    pub configuration: Value,
    pub bibliotheque: Value,
    pub parties: Vec<Element>,
}

pub fn empreinte(chemin: &Path) -> Resultat<String> {
    let mut f = std::fs::File::open(chemin)?;
    let mut h = Sha256::new();
    let mut tampon = vec![0u8; 256 * 1024];
    loop {
        let n = f.read(&mut tampon)?;
        if n == 0 {
            break;
        }
        h.update(&tampon[..n]);
    }
    Ok(h.finalize().iter().map(|o| format!("{o:02x}")).collect())
}

/// Écrit un fichier sûrement : `.part` puis renommage (jamais un fichier à moitié écrit à la place du bon).
fn ecrire_sur(cible: &Path, octets: &[u8]) -> Resultat<()> {
    if let Some(p) = cible.parent() {
        std::fs::create_dir_all(p)?;
    }
    let part = cible.with_extension("part-frogtend");
    std::fs::write(&part, octets)?;
    std::fs::rename(&part, cible)?;
    Ok(())
}

fn copier_sur(source: &Path, cible: &Path) -> Resultat<u64> {
    if let Some(p) = cible.parent() {
        std::fs::create_dir_all(p)?;
    }
    let part = cible.with_extension("part-frogtend");
    let n = std::fs::copy(source, &part)?;
    // Vérifier le résultat : la copie a la taille de l'original.
    if std::fs::metadata(&part)?.len() != std::fs::metadata(source)?.len() {
        let _ = std::fs::remove_file(&part);
        return Err(Erreur::Disque(format!("La copie de {} est incomplète.", source.display())));
    }
    std::fs::rename(&part, cible)?;
    Ok(n)
}

/// Envoie la sauvegarde dans `dossier` (celui du profil pour ce PC, sur le partage — ou un dossier local en test).
pub fn envoyer(dossier: &Path, contenu: &Contenu, pc: &str) -> Resultat<Bilan> {
    std::fs::create_dir_all(dossier.join("parties"))?;
    let chemin_manifeste = dossier.join("parties").join("manifeste.json");
    let ancien: ManifesteParties = std::fs::read(&chemin_manifeste)
        .ok()
        .and_then(|o| serde_json::from_slice(&o).ok())
        .unwrap_or_default();

    let mut nouveau = ManifesteParties::default();
    let mut bilan = Bilan { pc: pc.into(), dossier: dossier.to_string_lossy().into(), date: maintenant(), ..Default::default() };
    for e in &contenu.parties {
        if !e.source.is_file() {
            continue;
        }
        let taille = std::fs::metadata(&e.source)?.len();
        let sha = empreinte(&e.source)?;
        let cible = dossier.join("parties").join(&e.relatif);
        let deja = ancien.fichiers.get(&e.relatif) == Some(&(sha.clone(), taille))
            && std::fs::metadata(&cible).is_ok_and(|m| m.len() == taille);
        if deja {
            bilan.inchanges += 1;
        } else {
            bilan.octets += copier_sur(&e.source, &cible)?;
            bilan.envoyes += 1;
        }
        bilan.taille += taille;
        nouveau.fichiers.insert(e.relatif.clone(), (sha, taille));
    }
    // Ce qui n'existe plus sur le PC sort de la version courante (l'historique du serveur le garde).
    for r in ancien.fichiers.keys() {
        if !nouveau.fichiers.contains_key(r) {
            let p = dossier.join("parties").join(r);
            if p.is_file() {
                std::fs::remove_file(p)?;
                bilan.retires += 1;
            }
        }
    }
    bilan.fichiers = nouveau.fichiers.len();

    ecrire_sur(&dossier.join("configuration.json"), &serde_json::to_vec_pretty(&contenu.configuration).unwrap())?;
    ecrire_sur(&dossier.join("bibliotheque.json"), &serde_json::to_vec_pretty(&contenu.bibliotheque).unwrap())?;
    ecrire_sur(&chemin_manifeste, &serde_json::to_vec_pretty(&nouveau).unwrap())?;
    ecrire_sur(&dossier.join("derniere-sauvegarde.json"), &serde_json::to_vec_pretty(&bilan).unwrap())?;
    Ok(bilan)
}

fn lire_json(p: &Path) -> Value {
    std::fs::read(p).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or(Value::Null)
}

impl Noyau {
    /// Rassemble ce qu'il faut sauvegarder pour le profil ouvert. `programmes_emulateurs` : les programmes des
    /// émulateurs réglés sur ce PC (leurs dossiers portent les dossiers des profils).
    pub async fn contenu_sauvegarde(&self, programmes_emulateurs: &[String]) -> Resultat<Contenu> {
        let s = self.session().await?;
        let profil = s.profil.clone();
        let configuration = json!({
            "version_frogtend": env!("CARGO_PKG_VERSION"),
            "profil": { "id": profil.id, "nom": profil.nom, "protege": profil.protege },
            "reglages_pc": lire_json(&self.dossier.join("pc.json")),
            "reglages_profil": lire_json(&self.dossier.join("profils").join(format!("{}.json", profil.id))),
            "emulateurs_installes": lire_json(&self.dossier.join("emulateurs.json")),
        });

        let jeux = self.jeux_du_pc().await?;
        let mut parties = Vec::new();
        let mut liste = Vec::new();
        for vu in &jeux {
            let j = &vu.jeu;
            liste.push(json!({
                "id": j.id, "version": j.version, "titre": j.titre, "plateforme": j.plateforme,
                "dossier": j.dossier, "etat": j.etat, "total": j.total, "installation": j.installation,
                "temps_jeu": j.temps_jeu, "derniere_partie": j.derniere_partie,
            }));
            // Les parties d'un jeu installé : ce qui a changé depuis son installation.
            let Some(i) = &j.installation else { continue };
            let Ok(o) = std::fs::read(self.dossier_medias(j.id).join("manifeste.json")) else { continue };
            let Ok(m) = serde_json::from_slice::<Manifeste>(&o) else { continue };
            let racine = PathBuf::from(&i.dossier);
            if !racine.is_dir() {
                continue;
            }
            for r in changes_depuis(&racine, &m)? {
                parties.push(Element { source: racine.join(&r), relatif: format!("jeux/{}/{r}", j.id) });
            }
        }

        // Les parties du profil dans chaque émulateur.
        let mut vus = std::collections::BTreeSet::new();
        for programme in programmes_emulateurs {
            let Some(dossier) = Path::new(programme).parent() else { continue };
            if !vus.insert(dossier.to_path_buf()) {
                continue;
            }
            let nom_emulateur = crate::jeux_pc::nom_de_dossier(&dossier.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
            let du_profil = crate::emulateurs_profils::dossier_du_profil(dossier, &profil.nom);
            if !du_profil.is_dir() {
                continue;
            }
            for r in fichiers_de(&du_profil)? {
                parties.push(Element { source: du_profil.join(&r), relatif: format!("emulateurs/{nom_emulateur}/{r}") });
            }
        }

        Ok(Contenu {
            configuration,
            bibliotheque: json!({ "profil": profil.nom, "jeux": liste }),
            parties,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contenu(parties: Vec<Element>) -> Contenu {
        Contenu { configuration: json!({"a": 1}), bibliotheque: json!({"jeux": []}), parties }
    }

    #[test]
    fn seul_ce_qui_a_change_est_renvoye_et_ce_qui_a_disparu_sort_de_la_version_courante() {
        let pc = tempfile::tempdir().unwrap();
        let partage = tempfile::tempdir().unwrap();
        let a = pc.path().join("PARTIE1.SAV");
        let b = pc.path().join("PARTIE2.SAV");
        std::fs::write(&a, b"un").unwrap();
        std::fs::write(&b, b"deux").unwrap();
        let el = |p: &Path, r: &str| Element { source: p.into(), relatif: r.into() };

        let b1 = envoyer(partage.path(), &contenu(vec![el(&a, "jeux/110/PARTIE1.SAV"), el(&b, "jeux/110/PARTIE2.SAV")]), "VENKMAN").unwrap();
        assert_eq!((b1.envoyes, b1.inchanges, b1.fichiers, b1.taille), (2, 0, 2, 6)); // « un » + « deux » = 6 octets
        assert_eq!(std::fs::read(partage.path().join("parties/jeux/110/PARTIE1.SAV")).unwrap(), b"un");

        // Rien n'a changé : rien n'est renvoyé.
        let b2 = envoyer(partage.path(), &contenu(vec![el(&a, "jeux/110/PARTIE1.SAV"), el(&b, "jeux/110/PARTIE2.SAV")]), "VENKMAN").unwrap();
        assert_eq!((b2.envoyes, b2.inchanges, b2.octets), (0, 2, 0));

        // Une partie change, une autre disparaît.
        std::fs::write(&a, b"un, plus loin").unwrap();
        let b3 = envoyer(partage.path(), &contenu(vec![el(&a, "jeux/110/PARTIE1.SAV")]), "VENKMAN").unwrap();
        assert_eq!((b3.envoyes, b3.inchanges, b3.retires), (1, 0, 1));
        assert_eq!(std::fs::read(partage.path().join("parties/jeux/110/PARTIE1.SAV")).unwrap(), b"un, plus loin");
        assert!(!partage.path().join("parties/jeux/110/PARTIE2.SAV").exists());

        // Les documents sont là, sans fichier à moitié écrit.
        for f in ["configuration.json", "bibliotheque.json", "parties/manifeste.json", "derniere-sauvegarde.json"] {
            assert!(partage.path().join(f).is_file(), "{f}");
        }
        assert!(fichiers_de(partage.path()).unwrap().iter().all(|f| !f.ends_with("part-frogtend")));
    }

    #[test]
    fn un_fichier_efface_sur_le_serveur_est_renvoye() {
        let pc = tempfile::tempdir().unwrap();
        let partage = tempfile::tempdir().unwrap();
        let a = pc.path().join("x.sav");
        std::fs::write(&a, b"x").unwrap();
        let c = contenu(vec![Element { source: a.clone(), relatif: "jeux/1/x.sav".into() }]);
        envoyer(partage.path(), &c, "PC").unwrap();
        std::fs::remove_file(partage.path().join("parties/jeux/1/x.sav")).unwrap();
        assert_eq!(envoyer(partage.path(), &c, "PC").unwrap().envoyes, 1);
    }

    #[test]
    fn l_empreinte_est_le_sha256() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("f");
        std::fs::write(&f, b"abc").unwrap();
        assert_eq!(empreinte(&f).unwrap(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[tokio::test]
    async fn le_contenu_rassemble_parties_des_jeux_et_des_emulateurs() {
        use crate::coffre::CoffreMemoire;
        use crate::noyau::Connexion;
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, None).unwrap().id;
        n.ouvrir(&id, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        n.synchroniser(|_, _| {}).await.unwrap();

        // Un émulateur avec le dossier du profil Seb.
        let emu = d.path().join("Emulateurs").join("RetroArch");
        let saves = crate::emulateurs_profils::dossier_du_profil(&emu, "Seb").join("saves");
        std::fs::create_dir_all(&saves).unwrap();
        std::fs::write(saves.join("Super Metroid (USA).srm"), b"srm").unwrap();
        let programme = emu.join("retroarch.exe").to_string_lossy().to_string();

        let c = n.contenu_sauvegarde(&[programme]).await.unwrap();
        assert_eq!(c.configuration["profil"]["nom"], "Seb");
        let rel: Vec<_> = c.parties.iter().map(|e| e.relatif.as_str()).collect();
        assert_eq!(rel, vec!["emulateurs/RetroArch/saves/Super Metroid (USA).srm"]);
    }
}
