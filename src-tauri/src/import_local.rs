//! Importer des jeux déjà présents sur le disque (menu « Importer », comme LaunchBox ; Seb, 02/10) : ROM et images
//! disque d'un dossier, jeux MS-DOS (un sous-dossier par jeu), jeu Windows (son programme), ajout manuel.
//!
//! Règles :
//! - **rien n'est déplacé, copié ni renommé** : Frogtend note où est le jeu, c'est tout (les émulateurs et les bases de
//!   jaquettes reconnaissent une ROM par son nom officiel) ;
//! - **rien ne s'efface** : retirer un jeu importé le retire de la ludothèque, jamais du disque (voir
//!   `VERSION_IMPORTEE` dans `partie.rs`) ;
//! - une opération de masse (un dossier de ROM) est **comptée et montrée** avant d'être faite.

use crate::erreurs::{Erreur, Resultat};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// La « version » d'un jeu importé dans le registre du PC : il n'a pas été téléchargé par Frogtend, ses fichiers
/// appartiennent à la personne et ne sont JAMAIS effacés par Frogtend.
pub const VERSION_IMPORTEE: i64 = -1;

/// Au-delà, on refuse de parcourir (un disque entier choisi par erreur) : la personne choisit un dossier plus précis.
pub const MAX_FICHIERS_PARCOURUS: usize = 200_000;

/// Le titre d'un jeu d'après son nom de fichier : sans extension, sans les étiquettes No-Intro / Redump / TOSEC
/// (« (Europe) », « [!] », « (Disc 1) »…), les « _ » en espaces. Le FICHIER, lui, garde son nom.
pub fn titre_depuis_nom(nom: &str) -> String {
    let base = Path::new(nom).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| nom.to_string());
    let mut t = String::new();
    let mut profondeur = 0i32;
    for c in base.chars() {
        match c {
            '(' | '[' => profondeur += 1,
            ')' | ']' => profondeur = (profondeur - 1).max(0),
            c if profondeur == 0 => t.push(if c == '_' { ' ' } else { c }),
            _ => {}
        }
    }
    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.is_empty() { base } else { t }
}

/// Une ROM (ou image disque) trouvée.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RomTrouvee {
    /// Chemin complet du fichier (tel quel, jamais renommé).
    pub chemin: String,
    pub titre: String,
    pub taille: u64,
}

/// Les fichiers qu'une liste de pistes (`.cue`, `.m3u`, `.gdi`) désigne : ils ne sont pas des jeux à part.
fn pistes_designees(fichier: &Path) -> Vec<PathBuf> {
    let Ok(texte) = std::fs::read_to_string(fichier) else { return vec![] };
    let dossier = fichier.parent().unwrap_or(Path::new("."));
    let ext = fichier.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    texte
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            match ext.as_str() {
                // FILE "Jeu (Track 1).bin" BINARY
                "cue" => l.strip_prefix("FILE ").or_else(|| l.strip_prefix("file ")).and_then(|r| {
                    if let Some(r) = r.strip_prefix('"') { r.split('"').next() } else { r.split_whitespace().next() }
                }),
                // Une ligne par disque ; « # » = commentaire.
                "m3u" => (!l.is_empty() && !l.starts_with('#')).then_some(l),
                // 1 0 4 2352 "piste01.bin" 0  ou  1 0 4 2352 piste01.bin 0
                "gdi" => {
                    if let Some(i) = l.find('"') {
                        l[i + 1..].split('"').next()
                    } else {
                        l.split_whitespace().nth(4)
                    }
                }
                _ => None,
            }
        })
        .map(|n| dossier.join(n))
        .collect()
}

/// Les ROM d'un dossier (et de ses sous-dossiers si `recursif`) dont l'extension est dans `extensions` (sans point,
/// en minuscules). Les pistes désignées par un `.cue`, `.m3u` ou `.gdi` du même dossier ne sont pas comptées à part :
/// le jeu, c'est la liste (comme LaunchBox).
pub fn chercher_roms(dossier: &Path, extensions: &[String], recursif: bool) -> Resultat<Vec<RomTrouvee>> {
    if !dossier.is_dir() {
        return Err(Erreur::Disque(format!("Dossier introuvable : {}.", dossier.display())));
    }
    let voulues: BTreeSet<String> = extensions.iter().map(|e| e.trim().trim_start_matches('.').to_lowercase()).filter(|e| !e.is_empty()).collect();
    if voulues.is_empty() {
        return Err(Erreur::Refus("Indique au moins une extension de fichier (par exemple « sfc » ou « cue »).".into()));
    }
    let mut fichiers = Vec::new();
    let mut a_voir = vec![dossier.to_path_buf()];
    while let Some(d) = a_voir.pop() {
        for e in std::fs::read_dir(&d)?.flatten() {
            let p = e.path();
            let Ok(t) = e.file_type() else { continue };
            if t.is_dir() {
                if recursif {
                    a_voir.push(p);
                }
            } else if t.is_file() {
                fichiers.push(p);
                if fichiers.len() > MAX_FICHIERS_PARCOURUS {
                    return Err(Erreur::Refus(format!(
                        "Plus de {MAX_FICHIERS_PARCOURUS} fichiers dans ce dossier : choisis un dossier plus précis (celui d'un système)."
                    )));
                }
            }
        }
    }
    let ext_de = |p: &Path| p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let designees: BTreeSet<PathBuf> = fichiers
        .iter()
        .filter(|p| matches!(ext_de(p).as_str(), "cue" | "m3u" | "gdi") && voulues.contains(&ext_de(p)))
        .flat_map(|p| pistes_designees(p))
        .map(|p| normaliser_chemin(&p))
        .collect();
    let mut l: Vec<RomTrouvee> = fichiers
        .into_iter()
        .filter(|p| voulues.contains(&ext_de(p)) && !designees.contains(&normaliser_chemin(p)))
        .map(|p| RomTrouvee {
            titre: titre_depuis_nom(&p.file_name().unwrap_or_default().to_string_lossy()),
            taille: std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0),
            chemin: p.to_string_lossy().to_string(),
        })
        .collect();
    l.sort_by(|a, b| a.titre.to_lowercase().cmp(&b.titre.to_lowercase()).then(a.chemin.cmp(&b.chemin)));
    Ok(l)
}

/// Un chemin comparable (Windows ne distingue pas les majuscules ; « / » et « \ » se valent).
fn normaliser_chemin(p: &Path) -> PathBuf {
    PathBuf::from(p.to_string_lossy().replace('/', "\\").to_lowercase())
}

/// Un jeu MS-DOS trouvé : un sous-dossier, et le programme qui le lance le plus probablement.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct JeuDosTrouve {
    pub dossier: String,
    pub titre: String,
    /// Les programmes candidats (chemins RELATIFS au dossier du jeu), le plus probable d'abord.
    pub programmes: Vec<String>,
}

/// Les jeux MS-DOS d'un dossier : chaque sous-dossier qui contient un `.exe`, `.com` ou `.bat`.
pub fn chercher_jeux_dos(dossier: &Path) -> Resultat<Vec<JeuDosTrouve>> {
    if !dossier.is_dir() {
        return Err(Erreur::Disque(format!("Dossier introuvable : {}.", dossier.display())));
    }
    const A_ECARTER: &[&str] = &["install", "setup", "setsound", "sound", "config", "unins", "dos4gw", "cwsdpmi", "readme", "patch"];
    let mut l = Vec::new();
    let mut sous: Vec<PathBuf> = std::fs::read_dir(dossier)?.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    sous.sort();
    for d in sous {
        let titre = titre_depuis_nom(&d.file_name().unwrap_or_default().to_string_lossy());
        let mots: Vec<String> = titre.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|m| m.len() >= 3).map(String::from).collect();
        let mut programmes: Vec<(i32, String)> = crate::installation::fichiers_de(&d)?
            .into_iter()
            .filter_map(|r| {
                let bas = r.to_lowercase();
                let nom = bas.rsplit('/').next().unwrap_or(&bas).to_string();
                let ext = nom.rsplit('.').next().unwrap_or("");
                if !matches!(ext, "exe" | "com" | "bat") {
                    return None;
                }
                let racine_nom = nom.trim_end_matches(&format!(".{ext}")).to_string();
                let mut note = 10 - 4 * r.matches('/').count() as i32;
                if A_ECARTER.iter().any(|m| racine_nom.contains(m)) {
                    note -= 20;
                }
                note += 8 * mots.iter().filter(|m| racine_nom.contains(&m[..m.len().min(8)])).count() as i32;
                if ["play", "start", "go", "run", "jeu", "game"].contains(&racine_nom.as_str()) {
                    note += 6;
                }
                Some((note, r))
            })
            .collect();
        if programmes.is_empty() {
            continue;
        }
        programmes.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        l.push(JeuDosTrouve {
            dossier: d.to_string_lossy().to_string(),
            titre,
            programmes: programmes.into_iter().map(|(_, r)| r).collect(),
        });
    }
    Ok(l)
}

/// Un jeu à ajouter à la ludothèque, tel que la personne l'a validé.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JeuAImporter {
    pub titre: String,
    /// Nom LaunchBox de la plateforme (« Windows », « MS-DOS », « Super Nintendo Entertainment System »…).
    pub plateforme: String,
    /// Le dossier du jeu.
    pub dossier: String,
    /// Pour un émulateur : le fichier donné à l'émulateur, RELATIF à `dossier` (ROM, image, programme DOS).
    #[serde(default)]
    pub fichier: Option<String>,
    /// Pour un jeu Windows : le programme à lancer (chemin complet).
    #[serde(default)]
    pub programme: Option<String>,
}

/// La clé stable d'un jeu importé (le même fichier importé deux fois = le même jeu).
pub fn cle_import(j: &JeuAImporter) -> String {
    let cible = match (&j.programme, &j.fichier) {
        (Some(p), _) => PathBuf::from(p),
        (None, Some(f)) => Path::new(&j.dossier).join(f),
        (None, None) => PathBuf::from(&j.dossier),
    };
    normaliser_chemin(&cible).to_string_lossy().to_string()
}

/// Vérifie un jeu à importer : un titre, une plateforme, et ce qui le lance existe vraiment sur le disque.
pub fn verifier(j: &JeuAImporter) -> Resultat<()> {
    if j.titre.trim().is_empty() || j.plateforme.trim().is_empty() {
        return Err(Erreur::Refus("Il faut un titre et une plateforme.".into()));
    }
    if !Path::new(&j.dossier).is_dir() {
        return Err(Erreur::Disque(format!("Dossier introuvable : {}.", j.dossier)));
    }
    match (&j.programme, &j.fichier) {
        (Some(p), _) if !Path::new(p).is_file() => Err(Erreur::Disque(format!("Programme introuvable : {p}."))),
        (None, Some(f)) if f.contains("..") || !Path::new(&j.dossier).join(f).is_file() => {
            Err(Erreur::Disque(format!("Fichier introuvable : {f}.")))
        }
        (None, None) => Err(Erreur::Refus("Choisis le programme ou le fichier du jeu.".into())),
        _ => Ok(()),
    }
}

/// Le bilan d'un import.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct BilanImport {
    pub ajoutes: usize,
    /// Déjà dans la ludothèque (même fichier) : laissés tels quels.
    pub deja: usize,
    /// (titre, motif) de ceux qui n'ont pas pu être ajoutés.
    pub refuses: Vec<(String, String)>,
}

impl crate::noyau::Noyau {
    /// Ajoute des jeux du disque à la ludothèque du profil ouvert ET au registre du PC (pour « Jouer ») : rien n'est
    /// copié ni renommé ; le jeu est noté `VERSION_IMPORTEE` (Frogtend n'effacera jamais ses fichiers).
    pub async fn importer_locaux(&self, jeux: &[JeuAImporter]) -> Resultat<BilanImport> {
        use crate::installation::{Lanceur, Methode};
        use crate::jeux_pc::{Etat, Installation, JeuPc};
        let s = self.session().await?;
        let mut bilan = BilanImport::default();
        let mut lignes = Vec::new();
        for j in jeux {
            if let Err(e) = verifier(j) {
                bilan.refuses.push((j.titre.clone(), format!("{e:?}")));
                continue;
            }
            let cle = cle_import(j);
            let id = crate::ludotheque::id_boutique("local", &cle);
            if self.registre().jeu(id)?.is_some() {
                bilan.deja += 1;
                continue;
            }
            let lanceur = j.programme.as_ref().map(|p| Lanceur {
                programme: p.clone(),
                arguments: vec![],
                dossier: Path::new(p).parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_else(|| j.dossier.clone()),
            });
            let quand = crate::noyau::maintenant();
            self.registre().ajouter(&JeuPc {
                id,
                version: VERSION_IMPORTEE,
                titre: j.titre.trim().to_string(),
                plateforme: j.plateforme.clone(),
                dossier: j.dossier.clone(),
                etat: Etat::Telecharge,
                total: 0,
                fichiers: vec![],
                message: None,
                ajoute_le: quand.clone(),
                ajoute_par: s.profil.id.clone(),
                installation: None,
                temps_jeu: 0,
                derniere_partie: None,
            })?;
            let installation = Installation {
                dossier: j.dossier.clone(),
                methode: Methode::Aucune,
                lanceur: lanceur.clone(),
                fichier_du_jeu: if lanceur.is_some() { None } else { j.fichier.clone() },
                installe_le: quand,
            };
            self.registre().changer_installation(id, Some(&installation))?;
            lignes.push(crate::ludotheque::JeuResume {
                id,
                titre: j.titre.trim().to_string(),
                plateforme: j.plateforme.clone(),
                statut: Some("importe".into()),
                jaquette: Some(false),
                source: Some("local".into()),
                boutique: Some("local".into()),
                cle_boutique: Some(cle),
                installe: Some(true),
                ..Default::default()
            });
            bilan.ajoutes += 1;
        }
        s.verrou().ajouter_locaux(&lignes)?;
        self.journaliser(&format!("import local : {} ajouté(s), {} déjà là, {} refusé(s)", bilan.ajoutes, bilan.deja, bilan.refuses.len()));
        Ok(bilan)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn un_jeu_importe_se_joue_et_ne_s_efface_jamais_du_disque() {
        use crate::coffre::CoffreMemoire;
        use crate::noyau::{Connexion, Noyau};
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(CoffreMemoire::default())).unwrap();
        let profil = n.creer_profil("Seb", None, None).unwrap().id;
        n.ouvrir(&profil, None, &Connexion { adresse: String::new(), simule: true }).await.unwrap();
        let jeux = d.path().join("Mes jeux");
        ecrire(&jeux, "Mario (USA).sfc", "rom");
        ecrire(&jeux, "Doom/DOOM.EXE", "exe");
        // Un désinstalleur présent : il ne doit JAMAIS être lancé pour un jeu importé.
        ecrire(&jeux, "Doom/unins000.exe", "x");
        let racine = jeux.to_string_lossy().to_string();
        let a = vec![
            JeuAImporter { titre: "Mario".into(), plateforme: "Super Nintendo Entertainment System".into(), dossier: racine.clone(), fichier: Some("Mario (USA).sfc".into()), programme: None },
            JeuAImporter { titre: "Doom".into(), plateforme: "Windows".into(), dossier: jeux.join("Doom").to_string_lossy().into(), fichier: None, programme: Some(jeux.join("Doom/DOOM.EXE").to_string_lossy().into()) },
            JeuAImporter { titre: "Absent".into(), plateforme: "Windows".into(), dossier: racine.clone(), fichier: None, programme: Some(jeux.join("rien.exe").to_string_lossy().into()) },
        ];
        let b = n.importer_locaux(&a).await.unwrap();
        assert_eq!((b.ajoutes, b.deja, b.refuses.len()), (2, 0, 1));
        assert_eq!(n.importer_locaux(&a[..1]).await.unwrap().deja, 1, "le même fichier n'est pas ajouté deux fois");

        // Dans la ludothèque du profil, rangé « local » et installé.
        let l = n.lister(&crate::ludotheque::Filtre { boutique: Some("local".into()), ludotheque: true, ..Default::default() }).await.unwrap();
        assert_eq!(l.total, 2);
        let mario = l.jeux.iter().find(|j| j.titre == "Mario").unwrap().clone();
        let i = n.registre().jeu(mario.id).unwrap().unwrap().installation.unwrap();
        assert_eq!(i.fichier_du_jeu.as_deref(), Some("Mario (USA).sfc"), "la ROM garde son nom");

        // Retirer : la ludothèque oublie le jeu, le disque garde TOUT.
        let doom = l.jeux.iter().find(|j| j.titre == "Doom").unwrap().id;
        n.retirer_du_pc(doom).await.unwrap();
        n.retirer_du_pc(mario.id).await.unwrap();
        assert!(jeux.join("Doom/DOOM.EXE").is_file() && jeux.join("Doom/unins000.exe").is_file());
        assert!(jeux.join("Mario (USA).sfc").is_file());
        assert!(n.registre().jeu(doom).unwrap().is_none());
        let l = n.lister(&crate::ludotheque::Filtre { boutique: Some("local".into()), ludotheque: true, ..Default::default() }).await.unwrap();
        assert_eq!(l.total, 0);
    }

    #[test]
    fn le_titre_perd_ses_etiquettes_mais_pas_le_fichier() {
        assert_eq!(titre_depuis_nom("Super Mario World (USA) [!].sfc"), "Super Mario World");
        assert_eq!(titre_depuis_nom("Final Fantasy VII (France) (Disc 1).cue"), "Final Fantasy VII");
        assert_eq!(titre_depuis_nom("Sonic_the_Hedgehog_(Europe).md"), "Sonic the Hedgehog");
        assert_eq!(titre_depuis_nom("(Proto).bin"), "(Proto)", "rien d'autre : on garde le nom");
        assert_eq!(titre_depuis_nom("Dune"), "Dune");
    }

    fn ecrire(d: &Path, nom: &str, contenu: &str) {
        if let Some(p) = d.join(nom).parent() {
            std::fs::create_dir_all(p).unwrap();
        }
        std::fs::write(d.join(nom), contenu).unwrap();
    }

    #[test]
    fn les_pistes_d_un_cue_ou_d_un_m3u_ne_sont_pas_des_jeux() {
        let d = tempfile::tempdir().unwrap();
        ecrire(d.path(), "FF7 (France) (Disc 1).cue", "FILE \"FF7 (France) (Disc 1).bin\" BINARY\n  TRACK 01 MODE2/2352\n");
        ecrire(d.path(), "FF7 (France) (Disc 1).bin", "x");
        ecrire(d.path(), "Crash (Europe).cue", "FILE \"Crash (Europe) (Track 1).bin\" BINARY\nFILE \"crash (europe) (track 2).bin\" BINARY\n");
        ecrire(d.path(), "Crash (Europe) (Track 1).bin", "x");
        ecrire(d.path(), "Crash (Europe) (Track 2).bin", "x");
        ecrire(d.path(), "Seul (Japan).bin", "x");
        ecrire(d.path(), "notes.txt", "x");
        ecrire(d.path(), "sous/Ape Escape (Europe).cue", "FILE \"Ape Escape (Europe).bin\" BINARY\n");
        ecrire(d.path(), "sous/Ape Escape (Europe).bin", "x");
        let ext = vec!["cue".to_string(), ".BIN".to_string()];
        let l = chercher_roms(d.path(), &ext, false).unwrap();
        let titres: Vec<&str> = l.iter().map(|r| r.titre.as_str()).collect();
        assert_eq!(titres, ["Crash", "FF7", "Seul"]);
        assert!(l[1].chemin.ends_with("FF7 (France) (Disc 1).cue"), "le fichier garde son nom");
        assert_eq!(chercher_roms(d.path(), &ext, true).unwrap().len(), 4, "récursif : Ape Escape en plus");

        ecrire(d.path(), "m3u/FF8.m3u", "# disques\nFF8 (Disc 1).chd\nFF8 (Disc 2).chd\n");
        ecrire(d.path(), "m3u/FF8 (Disc 1).chd", "x");
        ecrire(d.path(), "m3u/FF8 (Disc 2).chd", "x");
        let l = chercher_roms(&d.path().join("m3u"), &["m3u".into(), "chd".into()], false).unwrap();
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].titre, "FF8");
        assert!(chercher_roms(d.path(), &[], false).is_err());
    }

    #[test]
    fn un_jeu_dos_par_sous_dossier_avec_son_programme_probable() {
        let d = tempfile::tempdir().unwrap();
        ecrire(d.path(), "Dune (1992)/INSTALL.EXE", "x");
        ecrire(d.path(), "Dune (1992)/DUNE.EXE", "x");
        ecrire(d.path(), "Dune (1992)/SETSOUND.EXE", "x");
        ecrire(d.path(), "Prince of Persia/PRINCE.EXE", "x");
        ecrire(d.path(), "Vide/LISEZMOI.TXT", "x");
        let l = chercher_jeux_dos(d.path()).unwrap();
        assert_eq!(l.len(), 2, "un dossier sans programme n'est pas un jeu");
        assert_eq!(l[0].titre, "Dune");
        assert_eq!(l[0].programmes[0], "DUNE.EXE");
        assert_eq!(l[1].programmes[0], "PRINCE.EXE");
    }

    #[test]
    fn un_jeu_a_importer_est_verifie_sur_le_disque() {
        let d = tempfile::tempdir().unwrap();
        ecrire(d.path(), "jeu.sfc", "x");
        let racine = d.path().to_string_lossy().to_string();
        let mut j = JeuAImporter { titre: "Jeu".into(), plateforme: "SNES".into(), dossier: racine.clone(), fichier: Some("jeu.sfc".into()), programme: None };
        assert!(verifier(&j).is_ok());
        j.fichier = Some("../ailleurs.sfc".into());
        assert!(verifier(&j).is_err());
        j.fichier = Some("absent.sfc".into());
        assert!(verifier(&j).is_err());
        j.fichier = None;
        assert!(verifier(&j).is_err());
        j.programme = Some(d.path().join("jeu.sfc").to_string_lossy().to_string());
        assert!(verifier(&j).is_ok());
        let a = JeuAImporter { titre: "A".into(), plateforme: "X".into(), dossier: "D:/Jeux".into(), fichier: Some("Jeu.SFC".into()), programme: None };
        let b = JeuAImporter { titre: "B".into(), plateforme: "X".into(), dossier: "d:\\jeux".into(), fichier: Some("jeu.sfc".into()), programme: None };
        assert_eq!(cle_import(&a), cle_import(&b), "même fichier, même jeu");
    }
}

#[cfg(test)]
mod essais {
    /// Sur le vrai disque de Seb, en LECTURE SEULE : ce que la recherche trouverait (rien n'est ajouté).
    #[test]
    #[ignore]
    fn essai_recherche_reelle() {
        let d = std::env::var("FROGTEND_ESSAI_DOSSIER").unwrap_or_else(|_| "E:/Games/Nintendo Entertainement System".into());
        let ext: Vec<String> = std::env::var("FROGTEND_ESSAI_EXT").unwrap_or_else(|_| "nes".into()).split(',').map(String::from).collect();
        let debut = std::time::Instant::now();
        let l = super::chercher_roms(std::path::Path::new(&d), &ext, true).unwrap();
        println!("{} jeu(x) en {:?}", l.len(), debut.elapsed());
        for r in l.iter().take(6) {
            println!("{} <- {}", r.titre, r.chemin);
        }
    }
}
