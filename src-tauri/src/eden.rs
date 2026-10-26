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

/// Le programme est-il Eden ?
pub fn est_eden(programme: &str) -> bool {
    Path::new(programme).file_name().is_some_and(|n| n.to_string_lossy().to_ascii_lowercase().starts_with("eden"))
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
        return Ok(UtilisateurDuProfil { index: i, uuid: l[i].uuid, ancien_courant: None, cree: false });
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
                l.get(utilisateur_courant(eden)).or_else(|| l.first()).map(|u| u.uuid)
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
        Some(i) if relu == octets => Ok(UtilisateurDuProfil { index: i, uuid, ancien_courant, cree: true }),
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
    Ok(copies)
}

/// L'utilisateur Eden de ce profil s'il existe, SANS rien créer (pour la sauvegarde).
pub fn utilisateur_existant(eden: &Path, profil: &str) -> Option<[u8; 16]> {
    let nom = nom_eden(profil);
    lire(&std::fs::read(fichier_profils(eden)).ok()?).into_iter().find(|u| u.nom == nom).map(|u| u.uuid)
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

#[cfg(test)]
mod tests {
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
        // Léa existait déjà : rien à reprendre pour elle.
        let ul = utilisateur_du_profil(eden, "Léa").unwrap();
        assert!(!ul.cree);
        assert_eq!(reprendre_parties_d_avant(eden, &ul).unwrap(), 0);
        // Seb est créé : il reçoit les parties de l'utilisateur COURANT d'avant (Léa), jamais celles d'« Eden ».
        let seb = utilisateur_du_profil(eden, "Seb").unwrap();
        assert_eq!(seb.ancien_courant, Some(lea.uuid));
        let save_seb = dossiers_parties(eden, &seb.uuid)[0].1.clone();
        assert_eq!(reprendre_parties_d_avant(eden, &seb).unwrap(), 1);
        assert_eq!(std::fs::read(save_seb.join("0100000000010000").join("save.bin")).unwrap(), b"zelda de Lea");
        assert!(!save_seb.join("0100000000020000").exists());
        assert!(dossiers_parties(eden, &lea.uuid)[0].1.join("0100000000010000").join("save.bin").is_file(), "copiées, jamais déplacées");
        assert!(!PathBuf::from(format!("{}.frogtend", save_seb.join("0100000000010000").display())).exists());
        // Une seule fois par Eden.
        let zoe = utilisateur_du_profil(eden, "Zoé").unwrap();
        assert_eq!(reprendre_parties_d_avant(eden, &zoe).unwrap(), 0);
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
    fn huit_utilisateurs_au_plus() {
        let d = tempfile::tempdir().unwrap();
        let l: Vec<Utilisateur> = (0..8).map(|i| Utilisateur { uuid: [i as u8 + 1; 16], nom: format!("P{i}") }).collect();
        poser_fichier(d.path(), &ecrire(&l));
        assert!(matches!(utilisateur_du_profil(d.path(), "Seb"), Err(Erreur::Prevenir(_))));
        assert_eq!(lire(&std::fs::read(fichier_profils(d.path())).unwrap()), l, "rien n'a changé");
        assert_eq!(utilisateur_du_profil(d.path(), "P7").unwrap().index, 7);
    }
}
