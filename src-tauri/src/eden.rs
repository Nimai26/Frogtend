//! Eden (Switch) : un UTILISATEUR Switch par personne (décision de Seb, 26/10 : « Séparer »). Relevé dans le code
//! source d'Eden (commit eb88a40, 25/10/2026) :
//! - les utilisateurs sont dans `user\nand\system\save\8000000000000010\su\avators\profiles.dat` (portable) : 0x10
//!   octets nuls, puis 8 entrées de 0xC8 octets (UUID 16, UUID bis 16, horodatage u64, nom UTF-8 sur 32 octets,
//!   0x80 octets « extra » : avatar…) = 1 616 octets (profile_manager.cpp L30-44) ; une entrée à UUID nul est vide,
//!   les entrées sont tassées au chargement (L415-430) : l'index de `-u` est le rang parmi les entrées non vides ;
//! - `-u <nom>` inconnu ne change RIEN (le jeu part avec l'utilisateur précédent, launch_params.cpp L105) : Frogtend
//!   passe donc l'INDEX, relu juste avant de lancer ;
//! - Eden n'écrit `profiles.dat` que s'il change un utilisateur (jamais à la fermeture) ; il le relit à son démarrage :
//!   Frogtend ne l'écrit qu'Eden FERMÉ, et n'y change que l'entrée qu'il ajoute ;
//! - les parties d'un utilisateur : `user\nand\user\save\0000000000000000\<UUID à l'envers, hexa majuscule>\<titre>`
//!   (savedata_factory.cpp L133) ; et `user\save\account\<UUID dans l'ordre, minuscule>\…` s'il existe.

use crate::erreurs::{Erreur, Resultat};
use std::path::{Path, PathBuf};

const ENTETE: usize = 0x10;
const ENTREE: usize = 0xC8;
const MAX: usize = 8;
const TAILLE: usize = ENTETE + ENTREE * MAX;
const NOM: usize = 32;

/// Dans une sauvegarde, le dossier (sous celui de l'émulateur) des parties de l'utilisateur Eden du profil.
pub const UTILISATEUR_EDEN: &str = ".eden-utilisateur";

/// Un utilisateur Switch d'Eden.
#[derive(Debug, Clone, PartialEq)]
pub struct Utilisateur {
    pub uuid: [u8; 16],
    pub nom: String,
}

/// L'utilisateur Eden d'un profil, prêt pour une partie.
#[derive(Debug, Clone, PartialEq)]
pub struct UtilisateurDuProfil {
    /// Son rang, pour `-u`.
    pub index: usize,
    pub uuid: [u8; 16],
    /// Créé à l'instant : l'utilisateur qu'Eden prenait JUSQUE-LÀ (ses parties d'avant Frogtend), s'il y en avait un.
    pub ancien_courant: Option<[u8; 16]>,
    pub cree: bool,
}

/// Le programme est-il Eden, ou un de ses dérivés Eglantine et Lichii (Switch 2, mêmes dossiers :
/// common/fs/path_util.cpp) ?
pub fn est_eden(programme: &str) -> bool {
    Path::new(programme).file_name().is_some_and(|n| {
        let n = n.to_string_lossy().to_ascii_lowercase();
        n.starts_with("eden") || n.starts_with("eglantine") || n.starts_with("lichii")
    })
}

/// L'id Frogtend d'un programme de la famille d'Eden (« eglantine », « lichii », sinon « eden »).
pub fn id_de(programme: &str) -> &'static str {
    let n = Path::new(programme).file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    if n.starts_with("eglantine") {
        "eglantine"
    } else if n.starts_with("lichii") {
        "lichii"
    } else {
        "eden"
    }
}

/// La plateforme d'un programme de la famille d'Eden : Eglantine et Lichii = Switch 2 (leurs clés et leur firmware
/// sont ceux de la Switch 2), sinon Switch.
pub fn plateforme(programme: &str) -> &'static str {
    let n = Path::new(programme).file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    if n.starts_with("eglantine") || n.starts_with("lichii") { "Nintendo Switch 2" } else { "Nintendo Switch" }
}

/// Le firmware posé par Frogtend (`.frogtend-firmware.json` à côté du programme) : sa version, et SES fichiers (les
/// seuls que Frogtend retirera pour une mise à jour).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct NoteFirmware {
    pub version: Option<String>,
    pub fichiers: Vec<String>,
}

fn fichier_note_firmware(eden: &Path) -> PathBuf {
    eden.join(".frogtend-firmware.json")
}

pub fn note_firmware(eden: &Path) -> Option<NoteFirmware> {
    std::fs::read(fichier_note_firmware(eden)).ok().and_then(|o| serde_json::from_slice(&o).ok())
}

/// Note le firmware posé : tous les .nca de `registered` (il était vide avant la pose).
pub fn noter_firmware(eden: &Path, version: Option<&str>) -> Resultat<()> {
    let Some(d) = dossier_firmware(eden) else { return Ok(()) };
    let mut fichiers: Vec<String> = std::fs::read_dir(&d)?.flatten().map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| nom_de_nca(n)).collect();
    fichiers.sort();
    let note = NoteFirmware { version: version.map(String::from), fichiers };
    let f = fichier_note_firmware(eden);
    let provisoire = f.with_extension("json.tmp");
    std::fs::write(&provisoire, serde_json::to_vec_pretty(&note).unwrap_or_default())?;
    std::fs::rename(&provisoire, &f)?;
    Ok(())
}

/// Pour une mise à jour du firmware : les fichiers que FROGTEND a posés partent à l'abri
/// (`.frogtend-sauvegardes\firmware-<version>`), jamais effacés. S'il reste un autre fichier système (posé par la
/// personne ou par l'émulateur), tout est remis en place et la mise à jour refusée.
pub fn mettre_firmware_a_l_abri(eden: &Path) -> Resultat<(PathBuf, NoteFirmware)> {
    let note = note_firmware(eden).ok_or_else(|| Erreur::Refus("Ce firmware n'a pas été posé par Frogtend : il est laissé tel quel.".into()))?;
    let registered = dossier_firmware(eden).ok_or_else(|| Erreur::Refus("Le dossier système a été déplacé : Frogtend n'y écrit pas.".into()))?;
    let horodatage = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let abri = eden.join(".frogtend-sauvegardes").join(format!("firmware-{}-{horodatage}", crate::jeux_pc::nom_de_dossier(note.version.as_deref().unwrap_or("inconnu"))));
    std::fs::create_dir_all(&abri)?;
    let mut partis: Vec<String> = Vec::new();
    let remettre = |partis: &[String]| {
        for n in partis {
            let _ = std::fs::rename(abri.join(n), registered.join(n));
        }
        let _ = std::fs::remove_dir(&abri); // seulement s'il est vide
    };
    for n in note.fichiers.iter().filter(|n| nom_de_nca(n)) {
        let de = registered.join(n);
        if !de.is_file() {
            continue;
        }
        if let Err(e) = std::fs::rename(&de, abri.join(n)) {
            remettre(&partis);
            return Err(Erreur::Disque(format!("{n} n'a pas pu être mis à l'abri ({e}) : rien n'a changé.")));
        }
        partis.push(n.clone());
    }
    if firmware_present(eden) {
        remettre(&partis);
        return Err(Erreur::Refus("Le dossier système contient aussi des fichiers que Frogtend n'a pas posés : la mise à jour est refusée, rien n'a changé.".into()));
    }
    let _ = std::fs::remove_file(fichier_note_firmware(eden));
    Ok((abri, note))
}

/// Remet le firmware mis à l'abri (une mise à jour qui a échoué) : ses fichiers reviennent dans `registered`, sa note
/// aussi. Ce qui ne revient pas reste à l'abri (jamais effacé).
pub fn remettre_firmware(eden: &Path, abri: &(PathBuf, NoteFirmware)) {
    let Some(registered) = dossier_firmware(eden) else { return };
    let _ = std::fs::create_dir_all(&registered);
    for n in &abri.1.fichiers {
        let de = abri.0.join(n);
        if de.is_file() && !registered.join(n).exists() {
            let _ = std::fs::rename(&de, registered.join(n));
        }
    }
    let f = fichier_note_firmware(eden);
    let _ = std::fs::write(&f, serde_json::to_vec_pretty(&abri.1).unwrap_or_default());
    let _ = std::fs::remove_dir(&abri.0);
}

/// Eden est-il installé en portable (son dossier `user` à côté de lui) ? Sinon, ses données sont dans `%APPDATA%` :
/// Frogtend n'y écrit jamais (règle « pas de pieuvre »).
pub fn portable(eden: &Path) -> bool {
    eden.join("user").is_dir()
}

pub fn fichier_profils(eden: &Path) -> PathBuf {
    eden.join("user").join("nand").join("system").join("save").join("8000000000000010").join("su").join("avators").join("profiles.dat")
}

/// Le nom tel qu'Eden le garde : 32 octets UTF-8 au plus, coupé sur un caractère entier.
pub fn nom_eden(nom: &str) -> String {
    let mut n = String::new();
    for c in nom.chars() {
        if n.len() + c.len_utf8() > NOM {
            break;
        }
        n.push(c);
    }
    n
}

/// Les utilisateurs d'un `profiles.dat` de la bonne taille, dans l'ordre (celui des index de `-u`) ; d'une autre
/// taille : aucun.
pub fn lire(octets: &[u8]) -> Vec<Utilisateur> {
    if octets.len() != TAILLE {
        return vec![];
    }
    (0..MAX)
        .filter_map(|i| {
            let e = &octets[ENTETE + i * ENTREE..ENTETE + (i + 1) * ENTREE];
            let uuid: [u8; 16] = e[..16].try_into().ok()?;
            if uuid == [0; 16] {
                return None;
            }
            let brut = &e[40..40 + NOM];
            let fin = brut.iter().position(|b| *b == 0).unwrap_or(NOM);
            Some(Utilisateur { uuid, nom: String::from_utf8_lossy(&brut[..fin]).to_string() })
        })
        .collect()
}

/// Écrit `u` dans l'entrée `i` d'un `profiles.dat` (les autres entrées ne sont pas touchées).
fn poser(octets: &mut [u8], i: usize, u: &Utilisateur) {
    let e = &mut octets[ENTETE + i * ENTREE..ENTETE + (i + 1) * ENTREE];
    e.fill(0);
    e[..16].copy_from_slice(&u.uuid);
    e[16..32].copy_from_slice(&u.uuid);
    let nom = nom_eden(&u.nom);
    e[40..40 + nom.len()].copy_from_slice(nom.as_bytes());
}

/// Un `profiles.dat` neuf pour ces utilisateurs (entrées tassées).
pub fn ecrire(utilisateurs: &[Utilisateur]) -> Vec<u8> {
    let mut o = vec![0u8; TAILLE];
    for (i, u) in utilisateurs.iter().take(MAX).enumerate() {
        poser(&mut o, i, u);
    }
    o
}

/// Un UUID aléatoire non nul (Eden n'exige rien d'autre, uuid.cpp L180).
fn uuid_neuf() -> [u8; 16] {
    loop {
        let u: [u8; 16] = rand::random();
        if u != [0; 16] {
            return u;
        }
    }
}

/// Le dossier NAND d'Eden portable : `user\nand`, sauf si `nand_directory` de qt-config.ini (Eden y écrit son chemin
/// complet, frontend_common/config.cpp L281/L584) l'a mis ailleurs. `None` : HORS du dossier d'Eden — Frogtend n'y
/// écrit pas (règle « pas de pieuvre »).
pub fn dossier_nand(eden: &Path) -> Option<PathBuf> {
    let defaut = eden.join("user").join("nand");
    let regle = std::fs::read_to_string(eden.join("user").join("config").join("qt-config.ini"))
        .ok()
        .and_then(|t| t.lines().find_map(|l| l.trim().strip_prefix("nand_directory=").map(|v| v.trim().trim_matches('"').to_string())))
        .filter(|v| !v.is_empty());
    let Some(v) = regle else { return Some(defaut) };
    let norme = |p: &Path| p.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_lowercase();
    // Jamais un « .. » qui ressortirait du dossier d'Eden après l'avoir nommé.
    let dans_eden = !v.contains("..") && norme(Path::new(&v)).starts_with(&format!("{}\\", norme(eden)));
    dans_eden.then(|| PathBuf::from(v.replace('/', "\\")))
}

/// Le dossier du firmware (les .nca du système) : `<nand>\system\Contents\registered` (file_sys/bis_factory.cpp L19-31).
pub fn dossier_firmware(eden: &Path) -> Option<PathBuf> {
    dossier_nand(eden).map(|n| n.join("system").join("Contents").join("registered"))
}

/// Les clés de la personne sont-elles là (`user\keys\prod.keys`, crypto/key_manager.cpp L575) ? Sans elles, Eden
/// refuse le firmware et les jeux.
pub fn cles_presentes(eden: &Path) -> bool {
    eden.join("user").join("keys").join("prod.keys").is_file()
}

/// Un firmware est-il déjà là (au moins un .nca dans `registered`, à plat ou dans un sous-dossier) ?
pub fn firmware_present(eden: &Path) -> bool {
    let Some(d) = dossier_firmware(eden) else { return false };
    let est_nca = |p: &Path| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("nca"));
    std::fs::read_dir(&d).into_iter().flatten().flatten().any(|e| {
        let p = e.path();
        est_nca(&p) || (p.is_dir() && std::fs::read_dir(&p).into_iter().flatten().flatten().any(|x| est_nca(&x.path())))
    })
}

/// Pose le firmware servi (le .zip de Firehouse, déjà vérifié) dans un `registered` VIDE, Eden FERMÉ : extrait dans
/// un dossier provisoire d'Eden, seuls des `<hexa>.nca` / `<hexa>.cnmt.nca` sont pris, chacun posé SANS JAMAIS
/// ÉCRASER puis relu (taille). JAMAIS par le menu « Install Firmware » d'Eden, qui VIDE d'abord `registered`
/// (qt_common/util/content.cpp L104-115). Rend le nombre de fichiers posés.
pub fn poser_firmware(eden: &Path, zip: &Path) -> Resultat<usize> {
    if !portable(eden) {
        return Err(Erreur::Refus("Eden n'est pas installé en mode portable : Frogtend n'écrit pas dans ses données de Windows.".into()));
    }
    let Some(registered) = dossier_firmware(eden) else {
        return Err(Erreur::Refus("Le dossier système d'Eden a été déplacé hors de son dossier : Frogtend n'y écrit pas.".into()));
    };
    if firmware_present(eden) {
        return Err(Erreur::Refus("Un firmware est déjà installé dans Eden : il est laissé tel quel.".into()));
    }
    let tmp = eden.join("user").join(".frogtend-firmware");
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp)?; // le NÔTRE, d'une pose coupée
    }
    let r = (|| -> Resultat<usize> {
        crate::installation::decompresser(zip, crate::installation::Format::Zip, &tmp)?;
        let mut nca = Vec::new();
        let mut pile = vec![tmp.clone()];
        while let Some(d) = pile.pop() {
            for e in std::fs::read_dir(&d)?.flatten() {
                let p = e.path();
                if p.is_dir() {
                    pile.push(p);
                } else if nom_de_nca(&p.file_name().unwrap_or_default().to_string_lossy()) {
                    nca.push(p);
                }
            }
        }
        if nca.is_empty() {
            return Err(Erreur::Serveur("Le firmware reçu ne contient aucun fichier du système (.nca).".into()));
        }
        // Deux fichiers au même nom (dans deux sous-dossiers) : refusé AVANT de poser quoi que ce soit.
        let mut noms = std::collections::HashSet::new();
        if !nca.iter().all(|f| noms.insert(f.file_name().unwrap_or_default().to_string_lossy().to_lowercase())) {
            return Err(Erreur::Serveur("Le firmware reçu contient deux fois le même fichier : rien n'est posé.".into()));
        }
        std::fs::create_dir_all(&registered)?;
        // Tout ou rien : au moindre échec, les fichiers posés PAR CETTE POSE (les nôtres, à l'instant) sont retirés,
        // pour qu'Eden ne garde jamais un firmware à moitié posé (qui ferait refuser toute nouvelle pose).
        let mut poses: Vec<PathBuf> = Vec::new();
        let r = nca.iter().try_for_each(|f| -> Resultat<()> {
            let nom = f.file_name().unwrap_or_default();
            let taille = std::fs::metadata(f)?.len();
            let cible = registered.join(nom);
            crate::pack_mame::deplacer_sans_ecraser(f, &cible).map_err(|e| Erreur::Disque(format!("{} n'a pas pu être posé ({e}).", nom.to_string_lossy())))?;
            poses.push(cible.clone());
            if std::fs::metadata(&cible).map(|m| m.len()).ok() != Some(taille) {
                return Err(Erreur::Disque(format!("{} est abîmé après la pose.", nom.to_string_lossy())));
            }
            Ok(())
        });
        if let Err(e) = r {
            for p in &poses {
                let _ = std::fs::remove_file(p);
            }
            return Err(e);
        }
        Ok(poses.len())
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    r
}

/// Un nom de fichier du système : `<32 hexa>.nca` ou `<32 hexa>.cnmt.nca` (file_sys/registered_cache.cpp L59-64).
pub(crate) fn nom_de_nca(nom: &str) -> bool {
    let n = nom.to_ascii_lowercase();
    let Some(base) = n.strip_suffix(".cnmt.nca").or_else(|| n.strip_suffix(".nca")) else { return false };
    base.len() == 32 && base.chars().all(|c| c.is_ascii_hexdigit())
}

/// Les utilisateurs Eden qui appartiennent à un profil de Frogtend (`Profils\.utilisateurs-frogtend`, UUID en hexa
/// par ligne) : jamais une source de reprise des « parties d'avant » (ce sont les parties de quelqu'un).
fn table(eden: &Path) -> Vec<String> {
    std::fs::read_to_string(eden.join("Profils").join(".utilisateurs-frogtend")).unwrap_or_default().lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect()
}

fn hex(u: &[u8; 16]) -> String {
    u.iter().map(|o| format!("{o:02x}")).collect()
}

fn noter_profil(eden: &Path, uuid: &[u8; 16]) -> Resultat<()> {
    let mut t = table(eden);
    if t.contains(&hex(uuid)) {
        return Ok(());
    }
    t.push(hex(uuid));
    std::fs::create_dir_all(eden.join("Profils"))?;
    std::fs::write(eden.join("Profils").join(".utilisateurs-frogtend"), t.iter().map(|l| format!("{l}\n")).collect::<String>())?;
    Ok(())
}

/// Le rang de l'utilisateur qu'Eden utilisait jusque-là (`current_user` de `user\config\qt-config.ini`), 0 sinon.
fn utilisateur_courant(eden: &Path) -> usize {
    std::fs::read_to_string(eden.join("user").join("config").join("qt-config.ini"))
        .ok()
        .and_then(|t| t.lines().find_map(|l| l.trim().strip_prefix("current_user=").and_then(|v| v.trim().parse().ok())))
        .unwrap_or(0)
}

/// Une copie qui n'en écrase jamais une autre : `<nom>`, sinon `<nom>-2`, `-3`…
fn abri_libre(dossier: &Path, nom: &str) -> PathBuf {
    let mut p = dossier.join(nom);
    let mut n = 2;
    while p.exists() {
        p = dossier.join(format!("{nom}-{n}"));
        n += 1;
    }
    p
}

/// L'utilisateur Eden de ce profil, créé s'il manque. Eden doit être FERMÉ (il écraserait le fichier) : l'appelant le
/// vérifie. Un fichier des utilisateurs illisible ou d'une autre taille : REFUSÉ sans rien écrire (jamais un fichier
/// refait qui perdrait les utilisateurs de la maison) ; absent : créé (« Eden » d'abord, comme Eden l'aurait fait).
/// Avant toute écriture, l'ancien fichier est copié dans `<Eden>\.frogtend-sauvegardes\` ; seule l'entrée ajoutée
/// change ; écriture dans un fichier provisoire puis renommé, puis RELUE.
pub fn utilisateur_du_profil(eden: &Path, profil: &str) -> Resultat<UtilisateurDuProfil> {
    let f = fichier_profils(eden);
    let illisible = || {
        Erreur::Prevenir(
            "Le fichier des utilisateurs d'Eden est illisible ou abîmé : Frogtend n'y touche pas. Ouvre Eden une fois \
             (Émulation ▸ Configurer ▸ Profils) pour qu'il le remette en état, puis réessaie."
                .into(),
        )
    };
    let avant = match std::fs::read(&f) {
        Ok(o) if o.len() == TAILLE => Some(o),
        Ok(_) => return Err(illisible()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => return Err(illisible()),
    };
    let l = avant.as_deref().map(lire).unwrap_or_default();
    let nom = nom_eden(profil);
    if let Some(i) = l.iter().position(|u| u.nom == nom) {
        // Une reprise des parties d'avant pas encore réussie : retentée.
        noter_profil(eden, &l[i].uuid)?;
        let ancien_courant = crate::emulateurs_profils::reprise_a_faire(eden, &nom).and_then(|s| uuid_de_hex(&s));
        return Ok(UtilisateurDuProfil { index: i, uuid: l[i].uuid, ancien_courant, cree: false });
    }
    let uuid = uuid_neuf();
    let nouveau = Utilisateur { uuid, nom: nom.clone() };
    let (octets, ancien_courant) = match &avant {
        Some(a) => {
            let libre = (0..MAX).find(|i| a[ENTETE + i * ENTREE..ENTETE + i * ENTREE + 16] == [0u8; 16]).ok_or_else(|| {
                Erreur::Prevenir(
                    "Eden a déjà 8 utilisateurs (le maximum de la Switch) : Frogtend ne peut pas en créer un pour ce profil. Retire un utilisateur dans Eden (Émulation ▸ Configurer ▸ Profils), puis réessaie.".into(),
                )
            })?;
            // La reprise : seulement à la TOUTE PREMIÈRE création par Frogtend sur cet Eden (au lancement OU à la
            // restauration : la marque est posée ici) — jamais depuis l'utilisateur créé pour une autre personne.
            let marque = eden.join("Profils").join(".reprise-eden");
            let ancien = if marque.exists() {
                None
            } else {
                std::fs::create_dir_all(eden.join("Profils"))?;
                std::fs::write(&marque, &nom)?;
                let a = l.get(utilisateur_courant(eden)).or_else(|| l.first()).map(|u| u.uuid);
                if let Some(a) = a {
                    crate::emulateurs_profils::noter_reprise(eden, &nom, &hex(&a))?;
                }
                a
            };
            let abri = abri_libre(&eden.join(".frogtend-sauvegardes"), &format!("{}-profiles.dat", crate::noyau::maintenant()));
            std::fs::create_dir_all(abri.parent().unwrap_or(eden))?;
            std::fs::write(&abri, a)?;
            let mut o = a.clone();
            poser(&mut o, libre, &nouveau);
            (o, ancien)
        }
        None => {
            // Un Eden neuf : rien à reprendre, et plus jamais ensuite.
            std::fs::create_dir_all(eden.join("Profils"))?;
            std::fs::write(eden.join("Profils").join(".reprise-eden"), &nom)?;
            (ecrire(&[Utilisateur { uuid: uuid_neuf(), nom: "Eden".into() }, nouveau]), None)
        }
    };
    if let Some(p) = f.parent() {
        std::fs::create_dir_all(p)?;
    }
    let provisoire = f.with_extension("dat.frogtend");
    std::fs::write(&provisoire, &octets)?;
    std::fs::rename(&provisoire, &f)?;
    // Vérifier le résultat : relu, l'utilisateur est là ; les autres entrées sont intactes.
    let relu = std::fs::read(&f)?;
    match lire(&relu).iter().position(|u| u.uuid == uuid) {
        Some(i) if relu == octets => {
            noter_profil(eden, &uuid)?;
            Ok(UtilisateurDuProfil { index: i, uuid, ancien_courant, cree: true })
        }
        _ => Err(Erreur::Disque("L'utilisateur Eden de ce profil n'a pas pu être enregistré.".into())),
    }
}

/// Copie un dossier dans `cible` (qui ne doit pas exister) par un dossier provisoire, vérifié puis renommé : jamais une
/// copie à moitié faite à sa place.
pub fn copier_dossier_entier(source: &Path, cible: &Path) -> Resultat<usize> {
    let fichiers = crate::installation::fichiers_de(source)?;
    let provisoire = PathBuf::from(format!("{}.frogtend", cible.to_string_lossy()));
    if provisoire.exists() {
        std::fs::remove_dir_all(&provisoire)?; // un reste d'une copie interrompue, à Frogtend seul
    }
    crate::installation::copier(source, &fichiers, &provisoire)?;
    let taille = |p: &Path| std::fs::metadata(p).map(|m| m.len()).ok();
    if fichiers.iter().any(|f| taille(&provisoire.join(f)).is_none() || taille(&provisoire.join(f)) != taille(&source.join(f))) {
        let _ = std::fs::remove_dir_all(&provisoire);
        return Err(Erreur::Disque(format!("La copie de « {} » est incomplète : rien n'est posé.", source.display())));
    }
    if let Some(p) = cible.parent() {
        std::fs::create_dir_all(p)?;
    }
    std::fs::rename(&provisoire, cible)?;
    Ok(fichiers.len())
}

/// Décision de Seb (26/10) : les parties qu'Eden avait AVANT Frogtend (celles de l'utilisateur qu'il prenait
/// jusque-là, `u.ancien_courant`, rempli seulement à la toute première création par Frogtend sur cet Eden) sont
/// COPIÉES dans l'utilisateur de la PREMIÈRE personne qui joue ; jamais déplacées, jamais par-dessus une partie (un jeu
/// déjà présent est laissé). Rend le nombre de jeux copiés.
pub fn reprendre_parties_d_avant(eden: &Path, u: &UtilisateurDuProfil) -> Resultat<usize> {
    let mut copies = 0;
    // L'utilisateur d'avant appartient (désormais) au profil de quelqu'un : JAMAIS ses parties chez un autre.
    if u.ancien_courant.is_some_and(|s| table(eden).contains(&hex(&s))) {
        crate::emulateurs_profils::reprise_faite(eden);
        return Ok(0);
    }
    if let Some(source) = u.ancien_courant.filter(|s| *s != u.uuid) {
        for ((_, de), (_, vers)) in dossiers_parties(eden, &source).into_iter().zip(dossiers_parties(eden, &u.uuid)) {
            for e in std::fs::read_dir(&de).into_iter().flatten().flatten() {
                let cible = vers.join(e.file_name());
                if !e.file_type().is_ok_and(|t| t.is_dir()) || cible.exists() {
                    continue;
                }
                copier_dossier_entier(&e.path(), &cible)?;
                copies += 1;
            }
        }
    }
    // Réussie (une erreur plus haut la laisse « à faire ») : plus jamais retentée.
    if u.ancien_courant.is_some() {
        crate::emulateurs_profils::reprise_faite(eden);
    }
    Ok(copies)
}

fn uuid_de_hex(s: &str) -> Option<[u8; 16]> {
    if s.len() != 32 {
        return None;
    }
    let mut u = [0u8; 16];
    for (i, o) in u.iter_mut().enumerate() {
        *o = u8::from_str_radix(s.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(u)
}

/// L'utilisateur Eden de ce profil s'il existe, SANS rien créer (pour la sauvegarde).
pub fn utilisateur_existant(eden: &Path, profil: &str) -> Option<[u8; 16]> {
    let nom = nom_eden(profil);
    lire(&std::fs::read(fichier_profils(eden)).ok()?).into_iter().find(|u| u.nom == nom).map(|u| u.uuid)
}

/// Les parties « Device » (communes à tous les utilisateurs, comme sur la vraie Switch : UUID nul,
/// savedata_factory.cpp), jeu par jeu.
pub fn dossier_device(eden: &Path) -> PathBuf {
    eden.join("user").join("nand").join("user").join("save").join("0000000000000000").join("0".repeat(32))
}

/// Un dossier de parties d'un JEU Switch (son identifiant : 16 caractères hexadécimaux commençant par « 01 ») — jamais
/// une donnée système rangée au même endroit.
pub fn est_un_jeu(nom: &str) -> bool {
    // Switch : « 01… » ; Switch 2 : « 04… » (Zelda Ocarina of Time : 0400486024DEC000, contrat 1.38).
    nom.len() == 16 && (nom.starts_with("01") || nom.starts_with("04")) && nom.chars().all(|c| c.is_ascii_hexdigit())
}

/// Les dossiers des parties d'un utilisateur : (`save` : le NAND, `account` : le nouvel emplacement s'il existe).
pub fn dossiers_parties(eden: &Path, uuid: &[u8; 16]) -> [(&'static str, PathBuf); 2] {
    let envers: String = uuid.iter().rev().map(|b| format!("{b:02X}")).collect();
    let ordre: String = uuid.iter().map(|b| format!("{b:02x}")).collect();
    let user = eden.join("user");
    [
        ("save", user.join("nand").join("user").join("save").join("0000000000000000").join(envers)),
        ("account", user.join("save").join("account").join(ordre)),
    ]
}

// ---------------------------------------------------------------------------------------------------------------
// La manette du joueur 1 et la langue de la console (✅ Seb 08/11 : « le même principe que RPCS3 »), dans
// `user/config/qt-config.ini` d'un Eden ou d'un Eglantine PORTABLE et FERMÉ (sinon : rien ; il réécrirait ses
// réglages en quittant, et hors portable ses réglages sont dans %APPDATA%, où Frogtend n'écrit jamais).
// ---------------------------------------------------------------------------------------------------------------

/// Le fichier de réglages d'un Eden portable.
pub fn fichier_reglages(eden: &Path) -> PathBuf {
    eden.join("user").join("config").join("qt-config.ini")
}

/// Le GUID SDL d'une manette XInput tel qu'Eden le lie (SDL3, pilote XInput : `SDL_xinputjoystick.c`, et Eden met le
/// CRC du nom à zéro, `sdl_driver.cpp:30-37`, relu le 08/11) : bus USB, fournisseur, produit et version de la manette,
/// signature « x » et sous-type. Il change d'une manette à l'autre (Xbox One/Series : 045e:02ff ; la manette virtuelle
/// de Sunshine : celle d'une Xbox 360) : Frogtend le recalcule à chaque partie, pour la manette branchée.
pub fn guid_xinput(fournisseur: u16, produit: u16, version: u16, sous_type: u8) -> String {
    let mut o = [0u8; 16];
    o[0] = 0x03;
    o[4..6].copy_from_slice(&fournisseur.to_le_bytes());
    o[8..10].copy_from_slice(&produit.to_le_bytes());
    o[12..14].copy_from_slice(&version.to_le_bytes());
    o[14] = b'x';
    o[15] = sous_type;
    o.iter().map(|b| format!("{b:02x}")).collect()
}

/// La disposition que Frogtend écrit (celle qu'écrit la configuration automatique d'Eden pour une manette XInput, par
/// POSITION comme sur une Switch : le A de la Switch, à droite, est le B de la manette Xbox). Le bouton Home est laissé
/// vide : le bouton Xbox ouvre le menu de Frogtend (« Frogtend partout », comme le bouton PS de RPCS3).
pub fn disposition_manette(g: &str) -> Vec<(&'static str, String)> {
    let b = |n: u8| format!("\"engine:sdl,button:{n},port:0,guid:{g}\"");
    let gachette = |axe: u8| format!("\"engine:sdl,invert:+,port:0,guid:{g},axis:{axe},threshold:0.500000\"");
    let croix = |d: &str| format!("\"engine:sdl,direction:{d},port:0,guid:{g},hat:0\"");
    let stick = |x: u8, y: u8| format!("\"engine:sdl,offset_y:0.000000,invert_x:+,invert_y:+,deadzone:0.150000,port:0,guid:{g},axis_x:{x},offset_x:-0.000000,axis_y:{y}\"");
    vec![
        ("player_0_button_a", b(1)),
        ("player_0_button_b", b(0)),
        ("player_0_button_x", b(3)),
        ("player_0_button_y", b(2)),
        ("player_0_button_lstick", b(8)),
        ("player_0_button_rstick", b(9)),
        ("player_0_button_l", b(4)),
        ("player_0_button_r", b(5)),
        ("player_0_button_zl", gachette(2)),
        ("player_0_button_zr", gachette(5)),
        ("player_0_button_plus", b(7)),
        ("player_0_button_minus", b(6)),
        ("player_0_button_dleft", croix("left")),
        ("player_0_button_dup", croix("up")),
        ("player_0_button_dright", croix("right")),
        ("player_0_button_ddown", croix("down")),
        ("player_0_button_slleft", b(4)),
        ("player_0_button_srleft", b(5)),
        ("player_0_button_slright", b(4)),
        ("player_0_button_srright", b(5)),
        ("player_0_button_home", "[empty]".to_string()),
        ("player_0_button_screenshot", "[empty]".to_string()),
        ("player_0_lstick", stick(0, 1)),
        ("player_0_rstick", stick(3, 4)),
    ]
}

/// La valeur d'une clé de `[Controls]` (sans guillemets).
fn valeur_controls(texte: &str, cle: &str) -> Option<String> {
    crate::manettes::lire_ini(texte, "Controls", cle).first().map(|v| crate::emulateurs_profils::lire_valeur_qt(v))
}

/// Le joueur 1 est-il encore au clavier (le réglage d'origine d'Eden, ou rien d'écrit) ?
pub fn joueur1_au_clavier(texte: &str) -> bool {
    valeur_controls(texte, "player_0_button_a").is_none_or(|v| v.is_empty() || v.starts_with("engine:keyboard"))
}

/// Le GUID de la manette du joueur 1 (lu sur son bouton A), s'il est sur SDL.
fn guid_du_joueur1(texte: &str) -> Option<String> {
    let a = valeur_controls(texte, "player_0_button_a")?;
    a.split(',').find_map(|x| x.strip_prefix("guid:")).map(String::from)
}

/// Le joueur 1 a-t-il exactement la disposition de Frogtend (quelle que soit la manette) ?
pub fn joueur1_de_frogtend(texte: &str) -> bool {
    guid_du_joueur1(texte).is_some_and(|g| {
        disposition_manette(&g).iter().all(|(c, v)| valeur_controls(texte, c).as_deref() == Some(crate::emulateurs_profils::lire_valeur_qt(v).as_str()))
    })
}

/// Le bouton Home est-il retiré à Eden (la disposition de Frogtend, Home vide) ? Alors le bouton Xbox ouvre le menu
/// de Frogtend ; un Home remis par la personne le rend à Eden.
pub fn home_retire(eden: &Path) -> bool {
    portable(eden)
        && lire_reglages(eden).is_ok_and(|t| {
            joueur1_de_frogtend(&t) && valeur_controls(&t, "player_0_button_home").is_some_and(|v| v.is_empty() || v == "[empty]")
        })
}

/// Le fichier de réglages, lu avec un plafond (un fichier de plusieurs Mo n'est pas celui d'Eden : jamais touché).
fn lire_reglages(eden: &Path) -> Resultat<String> {
    let f = fichier_reglages(eden);
    // Pas encore créé (installation neuve, avant sa première ouverture) : vide — Frogtend le crée avec SES clés, et
    // l'émulateur garde ses valeurs d'origine pour toutes les autres (`Config::Initialize` → `SetUpIni` charge le
    // fichier, `Reload` lit chaque clé ou sa valeur d'origine, puis réécrit le fichier complet ; relu le 08/11).
    if !f.exists() {
        return Ok(String::new());
    }
    if std::fs::metadata(&f)?.len() > 8 * 1024 * 1024 {
        return Err(Erreur::Refus("Le fichier de réglages d'Eden est anormalement gros : Frogtend n'y touche pas.".into()));
    }
    Ok(std::fs::read_to_string(f)?)
}

/// Peut-on écrire les réglages de cet Eden (portable, fermé) ? Son fichier peut ne pas exister encore : réglé DÈS la
/// première partie d'une installation neuve (✅ Seb 08/11, Lichii : « ce doit être systématique à l'installation de
/// TOUT nouvel émulateur »). Un chemin occupé par autre chose qu'un fichier : jamais touché.
fn reglages_modifiables(eden: &Path) -> bool {
    let f = fichier_reglages(eden);
    portable(eden) && crate::lancement::processus_du_dossier(eden).is_empty() && (f.is_file() || !f.exists())
}

/// Écrit des clés QSettings (chacune avec sa ligne `\default=false`, sinon Qt garde la valeur d'origine) dans une
/// section, configuration d'avant mise à l'abri. Rend vrai si le fichier a changé.
fn ecrire_qt(eden: &Path, texte: &str, section: &str, valeurs: &[(&str, String)]) -> Resultat<bool> {
    let defauts: Vec<String> = valeurs.iter().map(|(c, _)| format!("{c}\\default")).collect();
    let mut toutes: Vec<(&str, &str)> = Vec::new();
    for d in &defauts {
        toutes.push((d.as_str(), "false"));
    }
    for (c, v) in valeurs {
        toutes.push((c, v.as_str()));
    }
    let Some(apres) = crate::emulateurs_profils::regler_cles_qt(texte, section, &toutes) else { return Ok(false) };
    crate::emulateurs_profils::remplacer_config(eden, &fichier_reglages(eden), &apres)?;
    Ok(true)
}

/// La manette du joueur 1 : la disposition XInput de Frogtend pour la manette branchée (`guid`, voir `guid_xinput`),
/// seulement si le joueur 1 est encore au clavier, déjà à Frogtend (une autre manette : réécrit pour elle), ou à la
/// demande de la personne (`forcer` : « Remettre la manette par défaut ») — jamais par-dessus une manette qu'elle a
/// réglée elle-même. Sans manette XInput branchée : rien d'écrit. Rend vrai si le joueur 1 a la disposition de Frogtend.
pub fn regler_manette(eden: &Path, forcer: bool, guid: Option<&str>) -> Resultat<bool> {
    if !reglages_modifiables(eden) {
        return Ok(false);
    }
    let texte = lire_reglages(eden)?;
    let a_frogtend = joueur1_de_frogtend(&texte);
    let Some(g) = guid else { return Ok(a_frogtend) };
    if a_frogtend && guid_du_joueur1(&texte).as_deref() == Some(g) {
        return Ok(true);
    }
    if !forcer && !a_frogtend && !joueur1_au_clavier(&texte) {
        return Ok(false);
    }
    let mut v = disposition_manette(g);
    v.push(("player_0_connected", "true".into()));
    ecrire_qt(eden, &texte, "Controls", &v)?;
    Ok(true)
}

/// « Clavier et souris » choisi pour ce jeu : la disposition de Frogtend (seulement elle) est rendue aux réglages
/// d'origine d'Eden (`\default=true` : Qt reprend alors la valeur par défaut, le clavier). Rend vrai si le fichier a
/// changé.
pub fn rendre_le_clavier(eden: &Path) -> Resultat<bool> {
    if !reglages_modifiables(eden) {
        return Ok(false);
    }
    let texte = lire_reglages(eden)?;
    let Some(g) = guid_du_joueur1(&texte).filter(|_| joueur1_de_frogtend(&texte)) else { return Ok(false) };
    let defauts: Vec<String> = disposition_manette(&g).iter().map(|(c, _)| format!("{c}\\default")).collect();
    let v: Vec<(&str, &str)> = defauts.iter().map(|d| (d.as_str(), "true")).collect();
    let Some(apres) = crate::emulateurs_profils::regler_cles_qt(&texte, "Controls", &v) else { return Ok(false) };
    crate::emulateurs_profils::remplacer_config(eden, &fichier_reglages(eden), &apres)?;
    Ok(true)
}

/// La langue de la console Switch (énumération `Language` d'Eden) d'après la langue de Windows (LANGID).
pub fn langue_switch(langid: u16) -> Option<u8> {
    Some(match (langid & 0x3FF, langid) {
        (_, 0x0409 | 0x1009) => 1,
        (_, 0x0C0C) => 13,
        (_, 0x0416) => 17,
        (_, 0x0404 | 0x0C04 | 0x1404) => 16,
        (_, 0x080A) => 14,
        (0x09, _) => 12,
        (0x0C, _) => 2,
        (0x07, _) => 3,
        (0x10, _) => 4,
        (0x0A, _) => 5,
        (0x04, _) => 15,
        (0x12, _) => 7,
        (0x13, _) => 8,
        (0x16, _) => 9,
        (0x19, _) => 10,
        (0x11, _) => 0,
        _ => return None,
    })
}

/// La région de la console (énumération `Region` d'Eden) d'après la langue de Windows.
pub fn region_switch(langid: u16) -> Option<u8> {
    Some(match (langid & 0x3FF, langid) {
        (_, 0x0409 | 0x1009 | 0x0C0C | 0x080A | 0x0416) => 1,
        (_, 0x0C09) => 3,
        (_, 0x0404 | 0x0C04 | 0x1404) => 6,
        (0x11, _) => 0,
        (0x12, _) => 5,
        (0x04, _) => 4,
        (0x09 | 0x0C | 0x07 | 0x10 | 0x0A | 0x13 | 0x16 | 0x19, _) => 2,
        _ => return None,
    })
}

/// La marque « langue déjà prise de Windows » (dans le dossier de l'émulateur, jamais ailleurs).
fn marque_langue(eden: &Path) -> PathBuf {
    eden.join("user").join(".frogtend-langue")
}

/// La langue et la région de la console prises de Windows, UNE seule fois par émulateur (une marque le retient :
/// la personne qui choisit ensuite l'anglais dans Eden le garde), et seulement une clé encore à sa valeur d'origine
/// (`\default` absent ou vrai). Rend vrai si le fichier a changé.
pub fn regler_langue(eden: &Path, langid: u16) -> Resultat<bool> {
    if !reglages_modifiables(eden) || marque_langue(eden).exists() {
        return Ok(false);
    }
    let texte = lire_reglages(eden)?;
    let d_origine = |cle: &str| crate::manettes::lire_ini(&texte, "System", &format!("{cle}\\default")).first().is_none_or(|v| v.trim() == "true");
    let mut v: Vec<(&str, String)> = Vec::new();
    if let Some(l) = langue_switch(langid).filter(|_| d_origine("language_index")) {
        v.push(("language_index", l.to_string()));
    }
    if let Some(r) = region_switch(langid).filter(|_| d_origine("region_index")) {
        v.push(("region_index", r.to_string()));
    }
    let change = !v.is_empty() && ecrire_qt(eden, &texte, "System", &v)?;
    std::fs::write(marque_langue(eden), b"")?;
    Ok(change)
}

/// Quitter sans question (✅ Seb 08/11 : la confirmation d'Eglantine coupe « Quitter le jeu » du menu de Frogtend, qui
/// a déjà demandé, et bloque le tout-manette) : `[UI] confirmStop` à 1, « selon le jeu » (`Ask_Based_On_Game` ;
/// l'énumération `ConfirmStop` de `settings_enums.h` : Ask_Always, Ask_Based_On_Game, Ask_Never ; lue le 08/11 dans
/// daily-20261107). `MainWindow::ConfirmClose` (la croix, donc le WM_CLOSE de Frogtend) et `ConfirmShutdownGame`
/// (« Arrêter ») ne demandent alors plus rien, SAUF si le jeu a verrouillé sa sortie (`GetExitLocked` : il écrit,
/// sa sauvegarde par exemple) — jamais « Ask_Never », qui fermerait aussi pendant cette écriture (expert lancement,
/// 08/11). Section `[UI]` lue seulement dans les réglages généraux (`QtConfig::ReadValues`, `if (global)`) : rien par
/// jeu. UNE seule fois par émulateur (une marque le retient : Qt réécrit `\default=true` dès qu'une valeur redevient
/// celle d'origine, `Config::WritePreparedSetting`, donc la personne qui remet « Toujours demander » le garde), et
/// seulement si la clé est encore à sa valeur d'origine. Rend vrai si le fichier a changé.
pub fn quitter_sans_question(eden: &Path) -> Resultat<bool> {
    if !reglages_modifiables(eden) || marque_sortie(eden).exists() {
        return Ok(false);
    }
    let texte = lire_reglages(eden)?;
    let d_origine = crate::manettes::lire_ini(&texte, "UI", "confirmStop\\default").first().is_none_or(|v| v.trim() == "true");
    let change = d_origine && ecrire_qt(eden, &texte, "UI", &[("confirmStop", "1".to_string())])?;
    std::fs::write(marque_sortie(eden), b"")?;
    Ok(change)
}

/// La marque « question de sortie déjà réglée » (dans le dossier de l'émulateur, jamais ailleurs).
fn marque_sortie(eden: &Path) -> PathBuf {
    eden.join("user").join(".frogtend-sortie")
}

#[cfg(test)]
mod tests_reglages {
    use super::*;

    fn eden_portable(texte: &str) -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("user").join("config")).unwrap();
        std::fs::write(fichier_reglages(d.path()), texte).unwrap();
        d
    }

    const CLAVIER: &str = "[Controls]\r\nplayer_0_button_a\\default=false\r\nplayer_0_button_a=\"engine:keyboard,code:67,toggle:0\"\r\nplayer_0_button_home\\default=false\r\nplayer_0_button_home=\"engine:keyboard,code:0,toggle:0\"\r\n\r\n[System]\r\nlanguage_index\\default=true\r\nlanguage_index=1\r\nregion_index\\default=true\r\nregion_index=1\r\n";

    #[test]
    fn la_confirmation_de_sortie_est_retiree_une_fois_jamais_par_dessus_un_choix() {
        let d = eden_portable("[UI]\r\nconfirmStop\\default=true\r\nconfirmStop=0\r\n\r\n[System]\r\nlanguage_index=1\r\n");
        assert!(quitter_sans_question(d.path()).unwrap());
        let t = std::fs::read_to_string(fichier_reglages(d.path())).unwrap();
        assert!(t.contains("confirmStop\\default=false\r\nconfirmStop=1\r\n") && !t.contains("confirmStop=0"), "{t}");
        assert!(t.contains("[System]\r\nlanguage_index=1\r\n"), "le reste est gardé : {t}");
        assert!(!quitter_sans_question(d.path()).unwrap(), "une seconde fois : rien");
        // La personne remet « Toujours demander » dans Eglantine (Qt écrit alors `\default=true` et 0) : gardé.
        let remis = "[UI]\r\nconfirmStop\\default=true\r\nconfirmStop=0\r\n";
        std::fs::write(fichier_reglages(d.path()), remis).unwrap();
        assert!(!quitter_sans_question(d.path()).unwrap());
        assert_eq!(std::fs::read_to_string(fichier_reglages(d.path())).unwrap(), remis);
        // Un choix déjà fait avant Frogtend (« Jamais ») : jamais touché, et la marque est posée.
        let d2 = eden_portable("[UI]\r\nconfirmStop\\default=false\r\nconfirmStop=2\r\n");
        assert!(!quitter_sans_question(d2.path()).unwrap());
        assert_eq!(std::fs::read_to_string(fichier_reglages(d2.path())).unwrap(), "[UI]\r\nconfirmStop\\default=false\r\nconfirmStop=2\r\n");
        assert!(marque_sortie(d2.path()).exists());
    }

    #[test]
    fn la_manette_est_posee_sur_un_joueur_au_clavier_puis_gardee() {
        let d = eden_portable(CLAVIER);
        assert!(!home_retire(d.path()));
        assert!(!regler_manette(d.path(), false, None).unwrap(), "sans manette branchée : rien");
        assert_eq!(std::fs::read_to_string(fichier_reglages(d.path())).unwrap(), CLAVIER);
        assert!(regler_manette(d.path(), false, Some(UNE)).unwrap());
        let t = std::fs::read_to_string(fichier_reglages(d.path())).unwrap();
        assert!(t.contains("player_0_button_a\\default=false\r\nplayer_0_button_a=\"engine:sdl,button:1,port:0,guid:030000005e040000ff02000000007801\"\r\n"), "{t}");
        assert!(t.contains("player_0_button_home=[empty]\r\n") && !t.contains("engine:keyboard"), "{t}");
        assert!(t.contains("region_index=1\r\n"), "le reste intact");
        assert!(joueur1_de_frogtend(&t) && home_retire(d.path()));
        // L'ancienne configuration est gardée à l'abri.
        assert!(walk(d.path()).iter().any(|p| p != &fichier_reglages(d.path()) && std::fs::read_to_string(p).is_ok_and(|x| x == CLAVIER)));
        // Une seconde fois : rien ne change.
        assert!(regler_manette(d.path(), false, Some(UNE)).unwrap());
        assert_eq!(std::fs::read_to_string(fichier_reglages(d.path())).unwrap(), t);
        // Une autre manette (celle de Sunshine) : réécrit pour elle ; aucune branchée : gardé.
        assert!(regler_manette(d.path(), false, Some(AUTRE)).unwrap());
        let t2 = std::fs::read_to_string(fichier_reglages(d.path())).unwrap();
        assert!(t2.contains(&format!("guid:{AUTRE}\"")) && !t2.contains(UNE) && joueur1_de_frogtend(&t2));
        assert!(regler_manette(d.path(), false, None).unwrap());
        // « Clavier et souris » pour un jeu : rendu aux valeurs d'origine d'Eden, Home compris.
        assert!(rendre_le_clavier(d.path()).unwrap());
        let t3 = std::fs::read_to_string(fichier_reglages(d.path())).unwrap();
        assert!(t3.contains("player_0_button_a\\default=true\r\n") && t3.contains("player_0_button_home\\default=true\r\n"), "{t3}");
        assert!(!rendre_le_clavier(d.path()).unwrap(), "une seconde fois : rien");
    }

    const UNE: &str = "030000005e040000ff02000000007801";
    const AUTRE: &str = "030000005e0400008e02000014017801";

    #[test]
    fn le_guid_a_la_forme_qu_eden_lie() {
        // Relevé dans la configuration écrite par Eden chez Seb (manette Xbox, 08/11).
        assert_eq!(guid_xinput(0x045e, 0x02ff, 0, 1), UNE);
        assert_eq!(guid_xinput(0x045e, 0x028e, 0x0114, 1), AUTRE);
    }

    fn walk(d: &Path) -> Vec<PathBuf> {
        let mut v = Vec::new();
        for e in std::fs::read_dir(d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                v.extend(walk(&p));
            } else {
                v.push(p);
            }
        }
        v
    }

    #[test]
    fn une_manette_reglee_par_la_personne_n_est_jamais_remplacee_sans_sa_demande() {
        let a_elle = "[Controls]\nplayer_0_button_a\\default=false\nplayer_0_button_a=\"engine:sdl,button:0,port:0,guid:03000000aaaa\"\n";
        let d = eden_portable(a_elle);
        assert!(!regler_manette(d.path(), false, Some(UNE)).unwrap());
        assert_eq!(std::fs::read_to_string(fichier_reglages(d.path())).unwrap(), a_elle);
        assert!(regler_manette(d.path(), true, Some(UNE)).unwrap(), "à sa demande");
        assert!(joueur1_de_frogtend(&std::fs::read_to_string(fichier_reglages(d.path())).unwrap()));
    }

    #[test]
    fn rien_hors_du_mode_portable_ni_sans_fichier() {
        let d = tempfile::tempdir().unwrap();
        assert!(!regler_manette(d.path(), true, Some(UNE)).unwrap() && !regler_langue(d.path(), 0x040C).unwrap());
        assert!(!d.path().join("user").exists(), "jamais basculé en portable");
    }

    #[test]
    fn une_installation_neuve_est_reglee_des_sa_premiere_partie() {
        // Lichii chez Seb (08/11) : installé, puis lancé — son qt-config.ini n'existait pas encore.
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("user")).unwrap();
        assert!(regler_manette(d.path(), false, Some(UNE)).unwrap());
        assert!(regler_langue(d.path(), 0x040C).unwrap());
        assert!(quitter_sans_question(d.path()).unwrap());
        let t = std::fs::read_to_string(fichier_reglages(d.path())).unwrap();
        assert!(joueur1_de_frogtend(&t) && guid_du_joueur1(&t).as_deref() == Some(UNE), "{t}");
        assert!(t.contains("player_0_button_home=[empty]"), "{t}");
        assert!(t.contains("language_index=2") && t.contains("region_index=2") && t.contains("confirmStop=1"), "{t}");
        // Ensuite, comme d'habitude : rien de plus.
        assert!(!regler_langue(d.path(), 0x0409).unwrap());
        assert!(regler_manette(d.path(), false, Some(UNE)).unwrap());
        assert_eq!(std::fs::read_to_string(fichier_reglages(d.path())).unwrap(), t);
        // Un DOSSIER à la place du fichier : jamais touché.
        let d2 = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(fichier_reglages(d2.path())).unwrap();
        assert!(!regler_manette(d2.path(), false, Some(UNE)).unwrap());
    }

    #[test]
    fn la_langue_vient_de_windows_sauf_choix_de_la_personne() {
        let d = eden_portable(CLAVIER);
        assert!(regler_langue(d.path(), 0x040C).unwrap());
        let t = std::fs::read_to_string(fichier_reglages(d.path())).unwrap();
        assert!(t.contains("language_index\\default=false\r\nlanguage_index=2\r\n") && t.contains("region_index\\default=false\r\nregion_index=2\r\n"), "{t}");
        // Ensuite (ou un choix de la personne) : jamais retouché.
        assert!(!regler_langue(d.path(), 0x0409).unwrap());
        // Même si la personne a remis la valeur d'origine (l'anglais, `\default=true`) : une seule fois.
        let remis = std::fs::read_to_string(fichier_reglages(d.path())).unwrap().replace("language_index\\default=false\r\nlanguage_index=2", "language_index\\default=true\r\nlanguage_index=1");
        std::fs::write(fichier_reglages(d.path()), &remis).unwrap();
        assert!(!regler_langue(d.path(), 0x040C).unwrap());
        assert_eq!(std::fs::read_to_string(fichier_reglages(d.path())).unwrap(), remis);
        std::fs::write(fichier_reglages(d.path()), CLAVIER).unwrap();
        assert_eq!(langue_switch(0x1009), Some(1), "anglais du Canada : américain, région USA");
        assert_eq!((langue_switch(0x0C0C), region_switch(0x0C0C)), (Some(13), Some(1)), "français du Canada");
        assert_eq!((langue_switch(0x080C), region_switch(0x080C)), (Some(2), Some(2)), "Belgique");
        assert_eq!(langue_switch(0x0401), None, "arabe : rien de deviné");
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn eglantine_est_de_la_famille_et_son_firmware_se_met_a_jour() {
        assert!(est_eden(r"E:\Emu\Eglantine\eglantine.exe") && est_eden("eden.exe") && !est_eden("ryujinx.exe"));
        assert_eq!(plateforme(r"E:\Emu\Eglantine\eglantine.exe"), "Nintendo Switch 2");
        assert!(est_eden(r"E:\Emu\Lichii\lichii.exe"));
        assert_eq!(plateforme(r"E:\Emu\Lichii\lichii.exe"), "Nintendo Switch 2", "Lichii : les clés et le firmware de la Switch 2");
        assert_eq!(plateforme("eden.exe"), "Nintendo Switch");
        assert!(est_un_jeu("0400486024DEC000") && est_un_jeu("0100000000010000") && !est_un_jeu("8000000000000010"));
        let d = tempfile::tempdir().unwrap();
        let reg = d.path().join("user").join("nand").join("system").join("Contents").join("registered");
        std::fs::create_dir_all(&reg).unwrap();
        let a = "00e57b67b4dbd750a2df0574cddc123d.nca";
        std::fs::write(reg.join(a), b"nca").unwrap();
        noter_firmware(d.path(), Some("23.0.1")).unwrap();
        assert_eq!(note_firmware(d.path()).unwrap(), NoteFirmware { version: Some("23.0.1".into()), fichiers: vec![a.into()] });
        // Un fichier système qui n'est pas de Frogtend : refusé, rien ne bouge.
        let autre = "11111111111111111111111111111111.nca";
        std::fs::write(reg.join(autre), b"x").unwrap();
        assert!(mettre_firmware_a_l_abri(d.path()).is_err());
        assert!(reg.join(a).is_file() && note_firmware(d.path()).is_some());
        std::fs::remove_file(reg.join(autre)).unwrap();
        let abri = mettre_firmware_a_l_abri(d.path()).unwrap();
        assert!(!firmware_present(d.path()) && note_firmware(d.path()).is_none());
        // Une pose qui échoue : l'ancien revient, avec sa note.
        remettre_firmware(d.path(), &abri);
        assert!(reg.join(a).is_file() && note_firmware(d.path()).unwrap().version.as_deref() == Some("23.0.1"));
        mettre_firmware_a_l_abri(d.path()).unwrap();
        let abris: Vec<_> = std::fs::read_dir(d.path().join(".frogtend-sauvegardes")).unwrap().flatten().collect();
        assert_eq!(abris.len(), 1, "un refus ne laisse pas d'abri vide");
        assert!(abris[0].path().join(a).is_file(), "à l'abri, jamais effacé");
    }
    #[test]
    fn le_firmware_se_pose_seulement_dans_un_dossier_vide_et_jamais_ailleurs() {
        use std::io::Write;
        use super::*;
        let d = tempfile::tempdir().unwrap();
        let eden = d.path().join("Eden");
        std::fs::create_dir_all(eden.join("user").join("config")).unwrap();
        // Le zip servi : deux .nca, un fichier étranger (jamais posé), dans un sous-dossier.
        let zip = d.path().join("firmware-switch.zip");
        let mut z = zip::ZipWriter::new(std::fs::File::create(&zip).unwrap());
        let o = zip::write::SimpleFileOptions::default();
        for (n, c) in [("18.1.0/0123456789abcdef0123456789abcdef.nca", &b"AAAA"[..]), ("18.1.0/fedcba9876543210fedcba9876543210.cnmt.nca", b"BB"), ("18.1.0/lisez-moi.txt", b"x")] {
            z.start_file(n, o).unwrap();
            z.write_all(c).unwrap();
        }
        z.finish().unwrap();
        assert!(!firmware_present(&eden) && !cles_presentes(&eden));
        assert_eq!(poser_firmware(&eden, &zip).unwrap(), 2);
        let reg = eden.join("user").join("nand").join("system").join("Contents").join("registered");
        assert_eq!(std::fs::read(reg.join("0123456789abcdef0123456789abcdef.nca")).unwrap(), b"AAAA");
        assert!(!reg.join("lisez-moi.txt").exists());
        assert!(!eden.join("user").join(".frogtend-firmware").exists(), "rien ne reste à l'écart");
        assert!(firmware_present(&eden));
        // Déjà un firmware (d'une autre version peut-être) : jamais touché.
        assert!(matches!(poser_firmware(&eden, &zip), Err(Erreur::Refus(_))));
        assert_eq!(std::fs::read(reg.join("0123456789abcdef0123456789abcdef.nca")).unwrap(), b"AAAA");
        // Un NAND déplacé HORS du dossier d'Eden : rien n'est écrit.
        let autre = d.path().join("Eden2");
        std::fs::create_dir_all(autre.join("user").join("config")).unwrap();
        std::fs::write(autre.join("user").join("config").join("qt-config.ini"), format!("[Data%20Storage]\nnand_directory={}/Ailleurs/nand/\n", d.path().to_string_lossy().replace('\\', "/"))).unwrap();
        assert!(dossier_nand(&autre).is_none());
        assert!(matches!(poser_firmware(&autre, &zip), Err(Erreur::Refus(_))));
        assert!(!d.path().join("Ailleurs").exists());
        // Le chemin qu'Eden écrit lui-même (dans son dossier) est accepté.
        std::fs::write(autre.join("user").join("config").join("qt-config.ini"), format!("nand_directory={}/user/nand/\n", autre.to_string_lossy().replace('\\', "/"))).unwrap();
        assert!(dossier_nand(&autre).is_some());
        std::fs::write(autre.join("user").join("config").join("qt-config.ini"), format!("nand_directory={}/user/../../Ailleurs/\n", autre.to_string_lossy().replace('\\', "/"))).unwrap();
        assert!(dossier_nand(&autre).is_none(), "« .. » : refusé");
        // Eden non portable : jamais.
        assert!(matches!(poser_firmware(&d.path().join("SansUser"), &zip), Err(Erreur::Refus(_))));
        // Un zip avec deux fois le même fichier : rien n'est posé (jamais un firmware à moitié).
        let neuf = d.path().join("Eden3");
        std::fs::create_dir_all(neuf.join("user")).unwrap();
        let double = d.path().join("double.zip");
        let mut z = zip::ZipWriter::new(std::fs::File::create(&double).unwrap());
        for n in ["a/0123456789abcdef0123456789abcdef.nca", "b/0123456789abcdef0123456789abcdef.nca", "b/fedcba9876543210fedcba9876543210.nca"] {
            z.start_file(n, o).unwrap();
            z.write_all(b"x").unwrap();
        }
        z.finish().unwrap();
        assert!(poser_firmware(&neuf, &double).is_err());
        assert!(!firmware_present(&neuf), "rien de posé : une nouvelle pose reste possible");
        assert!(nom_de_nca("0123456789ABCDEF0123456789abcdef.NCA") && !nom_de_nca("evil.nca") && !nom_de_nca("../0123456789abcdef0123456789abcdef.nca"));
    }

    use super::*;

    fn poser_fichier(eden: &Path, o: &[u8]) {
        std::fs::create_dir_all(fichier_profils(eden).parent().unwrap()).unwrap();
        std::fs::write(fichier_profils(eden), o).unwrap();
    }

    #[test]
    fn le_fichier_des_profils_a_la_forme_d_eden() {
        let u = Utilisateur { uuid: core::array::from_fn(|i| i as u8 + 1), nom: "Sébastien".into() };
        let o = ecrire(&[u.clone()]);
        assert_eq!(o.len(), 1616);
        assert_eq!(&o[..0x10], &[0u8; 0x10]);
        assert_eq!(&o[0x10..0x20], &u.uuid);
        assert_eq!(&o[0x20..0x30], &u.uuid, "UUID bis = le même");
        assert_eq!(&o[0x30..0x38], &[0u8; 8], "horodatage nul");
        assert_eq!(&o[0x38..0x38 + 10], "Sébastien".as_bytes());
        assert_eq!(lire(&o), vec![u]);
        assert!(lire(&o[..100]).is_empty(), "un fichier tronqué : aucun utilisateur");
        assert_eq!(nom_eden(&"é".repeat(20)).len(), 32, "32 octets au plus, jamais au milieu d'un caractère");
    }

    #[test]
    fn le_dossier_des_parties_est_l_uuid_a_l_envers() {
        let uuid: [u8; 16] = core::array::from_fn(|i| i as u8 + 1);
        let [(_, save), (_, compte)] = dossiers_parties(Path::new("E"), &uuid);
        assert!(save.to_string_lossy().ends_with("100F0E0D0C0B0A090807060504030201"), "{save:?}");
        assert!(save.parent().unwrap().ends_with("0000000000000000"));
        assert!(compte.to_string_lossy().ends_with("0102030405060708090a0b0c0d0e0f10"));
    }

    #[test]
    fn chaque_profil_a_son_utilisateur_cree_une_fois_et_garde_son_rang() {
        let d = tempfile::tempdir().unwrap();
        let eden = d.path();
        // Pas encore de fichier : « Eden » d'abord (comme Eden l'aurait fait), puis Seb.
        let seb = utilisateur_du_profil(eden, "Seb").unwrap();
        assert_eq!((seb.index, seb.cree, seb.ancien_courant), (1, true, None));
        let l = lire(&std::fs::read(fichier_profils(eden)).unwrap());
        assert_eq!(l.iter().map(|u| u.nom.as_str()).collect::<Vec<_>>(), ["Eden", "Seb"]);
        // Retrouvé tel quel, sans rien réécrire.
        let encore = utilisateur_du_profil(eden, "Seb").unwrap();
        assert_eq!((encore.index, encore.uuid, encore.cree), (1, seb.uuid, false));
        assert!(!eden.join(".frogtend-sauvegardes").exists(), "rien à sauvegarder : rien n'existait");
        // Léa : ajoutée après, l'ancien fichier copié à l'abri avant.
        let lea = utilisateur_du_profil(eden, "Léa").unwrap();
        assert_eq!(lea.index, 2);
        assert_ne!(lea.uuid, seb.uuid);
        assert_eq!(std::fs::read_dir(eden.join(".frogtend-sauvegardes")).unwrap().count(), 1);
        assert_eq!(utilisateur_existant(eden, "Léa"), Some(lea.uuid));
        assert_eq!(utilisateur_existant(eden, "Zoé"), None);
        assert!(!fichier_profils(eden).with_extension("dat.frogtend").exists());
    }

    #[test]
    fn un_fichier_abime_n_est_jamais_refait() {
        let d = tempfile::tempdir().unwrap();
        // Tronqué : refusé, rien d'écrit (les utilisateurs de la maison ne sont jamais perdus).
        poser_fichier(d.path(), &[1u8; 700]);
        assert!(matches!(utilisateur_du_profil(d.path(), "Seb"), Err(Erreur::Prevenir(_))));
        assert_eq!(std::fs::read(fichier_profils(d.path())).unwrap(), vec![1u8; 700]);
        assert!(!d.path().join(".frogtend-sauvegardes").exists());
        // Trop grand : pareil.
        poser_fichier(d.path(), &vec![0u8; 2000]);
        assert!(matches!(utilisateur_du_profil(d.path(), "Seb"), Err(Erreur::Prevenir(_))));
    }

    #[test]
    fn les_autres_utilisateurs_restent_intacts_et_une_place_libre_au_milieu_sert() {
        let d = tempfile::tempdir().unwrap();
        // Trois utilisateurs, la 2e entrée vide (un utilisateur retiré dans Eden), un avatar et une date chez « Eden ».
        let mut o = ecrire(&[Utilisateur { uuid: [1; 16], nom: "Eden".into() }, Utilisateur { uuid: [2; 16], nom: "x".into() }, Utilisateur { uuid: [3; 16], nom: "Léa".into() }]);
        o[ENTETE + 32..ENTETE + 40].copy_from_slice(&123u64.to_le_bytes());
        o[ENTETE + 0x48..ENTETE + 0x50].fill(9);
        o[ENTETE + ENTREE..ENTETE + 2 * ENTREE].fill(0);
        poser_fichier(d.path(), &o);
        let seb = utilisateur_du_profil(d.path(), "Seb").unwrap();
        let apres = std::fs::read(fichier_profils(d.path())).unwrap();
        assert_eq!(&apres[..ENTETE + ENTREE], &o[..ENTETE + ENTREE], "l'entrée d'« Eden » (date, avatar) intacte");
        assert_eq!(&apres[ENTETE + 2 * ENTREE..], &o[ENTETE + 2 * ENTREE..], "celle de Léa aussi");
        // Seb dans la place libre du milieu : son rang est 1 (Eden, Seb, Léa).
        assert_eq!(seb.index, 1);
        assert_eq!(lire(&apres).iter().map(|u| u.nom.as_str()).collect::<Vec<_>>(), ["Eden", "Seb", "Léa"]);
        assert_eq!(seb.ancien_courant, Some([1; 16]), "l'utilisateur courant d'avant (rang 0)");
    }

    #[test]
    fn les_parties_d_avant_vont_a_la_premiere_personne_qui_joue_une_seule_fois() {
        let d = tempfile::tempdir().unwrap();
        let eden = d.path();
        // Un Eden déjà utilisé : [Eden, Léa] ; le courant est Léa (rang 1), qui a Zelda ; « Eden » a Mario.
        let (ancien, lea) = (Utilisateur { uuid: [7; 16], nom: "Eden".into() }, Utilisateur { uuid: [8; 16], nom: "Léa".into() });
        poser_fichier(eden, &ecrire(&[ancien.clone(), lea.clone()]));
        std::fs::create_dir_all(eden.join("user").join("config")).unwrap();
        std::fs::write(eden.join("user").join("config").join("qt-config.ini"), "[System]\ncurrent_user\\default=false\ncurrent_user=1\n").unwrap();
        for (u, jeu, o) in [(&lea, "0100000000010000", &b"zelda de Lea"[..]), (&ancien, "0100000000020000", b"mario")] {
            let s = dossiers_parties(eden, &u.uuid)[0].1.join(jeu);
            std::fs::create_dir_all(&s).unwrap();
            std::fs::write(s.join("save.bin"), o).unwrap();
        }
        // Seb est créé (l'utilisateur « Léa » n'appartient encore à aucun profil) : il reçoit les parties de
        // l'utilisateur COURANT d'avant, jamais celles d'« Eden ».
        let seb = utilisateur_du_profil(eden, "Seb").unwrap();
        assert_eq!(seb.ancien_courant, Some(lea.uuid));
        let save_seb = dossiers_parties(eden, &seb.uuid)[0].1.clone();
        assert_eq!(reprendre_parties_d_avant(eden, &seb).unwrap(), 1);
        assert_eq!(std::fs::read(save_seb.join("0100000000010000").join("save.bin")).unwrap(), b"zelda de Lea");
        assert!(!save_seb.join("0100000000020000").exists());
        assert!(dossiers_parties(eden, &lea.uuid)[0].1.join("0100000000010000").join("save.bin").is_file(), "copiées, jamais déplacées");
        assert!(!PathBuf::from(format!("{}.frogtend", save_seb.join("0100000000010000").display())).exists());
        // Léa existait déjà : rien à reprendre pour elle.
        let ul = utilisateur_du_profil(eden, "Léa").unwrap();
        assert!(!ul.cree);
        assert_eq!(reprendre_parties_d_avant(eden, &ul).unwrap(), 0);
        // Une seule fois par Eden.
        let zoe = utilisateur_du_profil(eden, "Zoé").unwrap();
        assert_eq!(reprendre_parties_d_avant(eden, &zoe).unwrap(), 0);
    }

    #[test]
    fn jamais_les_parties_d_un_utilisateur_qui_appartient_a_un_profil() {
        let d = tempfile::tempdir().unwrap();
        let eden = d.path();
        // L'utilisateur courant d'avant s'appelle « Léa » et le profil de Léa l'a ADOPTÉ (même nom) : ce sont ses parties.
        let lea = Utilisateur { uuid: [8; 16], nom: "Léa".into() };
        poser_fichier(eden, &ecrire(&[lea.clone()]));
        let s = dossiers_parties(eden, &lea.uuid)[0].1.join("0100000000010000");
        std::fs::create_dir_all(&s).unwrap();
        std::fs::write(s.join("save.bin"), b"zelda de Lea").unwrap();
        assert!(!utilisateur_du_profil(eden, "Léa").unwrap().cree);
        // Seb est créé ensuite : la reprise décidée pointe vers Léa → abandonnée, rien copié, plus rien à faire.
        let seb = utilisateur_du_profil(eden, "Seb").unwrap();
        assert_eq!(seb.ancien_courant, Some(lea.uuid));
        assert_eq!(reprendre_parties_d_avant(eden, &seb).unwrap(), 0);
        assert!(!dossiers_parties(eden, &seb.uuid)[0].1.join("0100000000010000").exists());
        assert!(!crate::emulateurs_profils::reprise_en_attente(eden));
    }

    #[test]
    fn une_partie_deja_la_n_est_jamais_ecrasee_par_la_reprise() {
        let d = tempfile::tempdir().unwrap();
        let eden = d.path();
        let ancien = Utilisateur { uuid: [7; 16], nom: "Eden".into() };
        poser_fichier(eden, &ecrire(&[ancien.clone()]));
        let s = dossiers_parties(eden, &ancien.uuid)[0].1.join("0100000000020000");
        std::fs::create_dir_all(&s).unwrap();
        std::fs::write(s.join("save.bin"), b"mario").unwrap();
        let seb = utilisateur_du_profil(eden, "Seb").unwrap();
        let cible = dossiers_parties(eden, &seb.uuid)[0].1.join("0100000000020000");
        std::fs::create_dir_all(&cible).unwrap();
        std::fs::write(cible.join("save.bin"), b"mario de Seb").unwrap();
        assert_eq!(reprendre_parties_d_avant(eden, &seb).unwrap(), 0);
        assert_eq!(std::fs::read(cible.join("save.bin")).unwrap(), b"mario de Seb");
    }

    #[test]
    fn apres_une_restauration_la_personne_suivante_ne_recoit_jamais_les_parties_d_une_autre() {
        // PC neuf : Eden n'a jamais tourné ; la restauration crée l'utilisateur de Seb (avec ses parties).
        let d = tempfile::tempdir().unwrap();
        let seb = utilisateur_du_profil(d.path(), "Seb").unwrap();
        assert_eq!(seb.ancien_courant, None);
        let s = dossiers_parties(d.path(), &seb.uuid)[0].1.join("0100000000010000");
        std::fs::create_dir_all(&s).unwrap();
        std::fs::write(s.join("save.bin"), b"zelda de Seb").unwrap();
        // Léa joue ensuite (Eden a remis Seb comme utilisateur courant) : jamais les parties de Seb.
        std::fs::create_dir_all(d.path().join("user").join("config")).unwrap();
        std::fs::write(d.path().join("user").join("config").join("qt-config.ini"), "current_user=1\n").unwrap();
        let lea = utilisateur_du_profil(d.path(), "Léa").unwrap();
        assert_eq!(lea.ancien_courant, None);
        assert_eq!(reprendre_parties_d_avant(d.path(), &lea).unwrap(), 0);
        assert!(!dossiers_parties(d.path(), &lea.uuid)[0].1.exists());
    }

    #[test]
    fn seuls_les_jeux_ont_des_parties_communes() {
        assert!(est_un_jeu("0100000000010000"));
        assert!(!est_un_jeu("0000000000000010"), "une donnée système");
        assert!(!est_un_jeu("01000000000100"));
        assert!(!est_un_jeu("010000000001000G"));
    }

    #[test]
    fn huit_utilisateurs_au_plus() {
        let d = tempfile::tempdir().unwrap();
        let l: Vec<Utilisateur> = (0..8).map(|i| Utilisateur { uuid: [i as u8 + 1; 16], nom: format!("P{i}") }).collect();
        poser_fichier(d.path(), &ecrire(&l));
        assert!(matches!(utilisateur_du_profil(d.path(), "Seb"), Err(Erreur::Prevenir(_))));
        assert_eq!(lire(&std::fs::read(fichier_profils(d.path())).unwrap()), l, "rien n'a changé");
        assert_eq!(utilisateur_du_profil(d.path(), "P7").unwrap().index, 7);
    }
}
