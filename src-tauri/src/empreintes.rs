//! Les empreintes d'un fichier pour le rapprochement (contrat 1.21 ; avis de l'expert lancement du 20/10). LECTURE
//! seule : jamais rien d'écrit à côté d'un jeu, jamais rien de décompressé sur le disque, jamais un fichier entier en
//! mémoire (lecture en flux).

use crate::reconciliation::EntreeArchive;
use std::io::Read;
use std::path::Path;

/// Le CRC32 et le SHA1 d'un fichier, en UNE lecture (tampons de 256 Ko). Rend (crc32 en 8 hexadécimaux, sha1 en 40).
pub fn crc_sha1(f: &Path) -> std::io::Result<(String, String)> {
    use sha1::{Digest, Sha1};
    let mut r = std::fs::File::open(f)?;
    let mut crc = crc32fast::Hasher::new();
    let mut sha = Sha1::new();
    let mut t = vec![0u8; 256 * 1024];
    loop {
        let n = r.read(&mut t)?;
        if n == 0 {
            break;
        }
        crc.update(&t[..n]);
        sha.update(&t[..n]);
    }
    Ok((format!("{:08x}", crc.finalize()), sha.finalize().iter().map(|b| format!("{b:02x}")).collect()))
}

/// Les entrées d'une archive dans la forme du contrat : nom SANS dossier, crc32, taille ; dossiers et entrées « à
/// supprimer » écartés ; doublons de nom GARDÉS (seul l'ensemble crc+taille compte, réponse de Firehouse au besoin
/// 29). Une entrée non vide sans CRC → `None` (un ensemble partiel ne donnerait que « probable »).
pub fn entrees_du_contrat(rep: &[crate::sept_z::EntreeSeptZ]) -> Option<Vec<EntreeArchive>> {
    let mut l = Vec::new();
    for e in rep.iter().filter(|e| !e.dossier && !e.anti) {
        let crc = match e.crc {
            Some(c) => c,
            None if e.taille == 0 => 0,
            None => return None,
        };
        let nom = e.nom.rsplit(['/', '\\']).next().unwrap_or(&e.nom).to_string();
        if nom.is_empty() {
            continue;
        }
        l.push(EntreeArchive { nom, crc32: format!("{crc:08x}"), taille: e.taille });
    }
    (!l.is_empty()).then_some(l)
}

/// Ce qu'on lit d'un fichier selon sa nature (contrat § 1.21 « sens des empreintes »).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Nature {
    /// Une archive zip/7z de cartouche : ses entrées, plus le sha1 du fichier entier s'il est sous le plafond.
    Archive,
    /// Un zip MAME : nom court + entrées, JAMAIS de sha1 (les romsets pèsent des dizaines de Go).
    Mame,
    /// Un fichier en clair (.nes, .sfc…) : crc32 + taille du fichier ENTIER, en-tête compris.
    EnClair,
    /// Une image disque ou un CHD : rien n'est lu (la 1.21 ne les reconnaît pas exactement ; CHD : besoin 29).
    Disque,
}

const EXTENSIONS_DISQUE: &[&str] = &["iso", "bin", "cue", "img", "chd", "gdi", "cdi", "m3u", "mds", "mdf", "ccd", "nrg", "pbp", "cso", "rvz", "gcz", "wbfs", "wux", "xci", "nsp", "pkg", "vpk"];

/// La nature d'un fichier d'après la plateforme et son nom.
pub fn nature(plateforme: &str, nom: &str) -> Nature {
    let ext = nom.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    if EXTENSIONS_DISQUE.contains(&ext.as_str()) {
        Nature::Disque
    } else if plateforme == "Arcade" && (ext == "zip" || ext == "7z") {
        Nature::Mame
    } else if ext == "zip" || ext == "7z" {
        Nature::Archive
    } else {
        Nature::EnClair
    }
}

/// Le fichier est-il sur un disque réseau (`\\serveur\…` ou un lecteur réseau) ? Là, rien n'est lu en entier sans
/// l'accord de la personne (règle de Seb : pas d'opération de masse sur un disque réseau sans l'annoncer).
pub fn sur_le_reseau(f: &Path) -> bool {
    let texte = f.to_string_lossy();
    if texte.starts_with(r"\\?\UNC\") || (texte.starts_with(r"\\") && !texte.starts_with(r"\\?\")) {
        return true;
    }
    #[cfg(windows)]
    {
        use windows::core::HSTRING;
        use windows::Win32::Storage::FileSystem::GetDriveTypeW;
        let racine: String = texte.chars().take(2).collect();
        if racine.len() == 2 && racine.ends_with(':') {
            // SAFETY: lecture du type d'un lecteur ; 4 = DRIVE_REMOTE.
            return unsafe { GetDriveTypeW(&HSTRING::from(format!("{racine}\\"))) } == 4;
        }
    }
    false
}

/// Ce qui est su d'un fichier.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Empreinte {
    pub crc32: Option<String>,
    pub sha1: Option<String>,
    pub entrees: Option<Vec<EntreeArchive>>,
}

/// La version du calcul : un cache d'une autre version est recalculé.
const VERSION_CALCUL: i64 = 1;

/// Le cache des empreintes de CE PC (`empreintes.db` dans les données de Frogtend) : un fichier inchangé (même chemin,
/// même taille, même date) n'est jamais relu. Les RÉPONSES de Firehouse n'y sont jamais (elles dépendent du jeton).
pub struct Cache(rusqlite::Connection);

fn date_de(m: &std::fs::Metadata) -> i64 {
    m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64).unwrap_or(0)
}

impl Cache {
    pub fn ouvrir(fichier: &Path) -> crate::erreurs::Resultat<Cache> {
        let disque = |e: rusqlite::Error| crate::erreurs::Erreur::Disque(format!("Cache des empreintes illisible ({e})."));
        let c = rusqlite::Connection::open(fichier).map_err(disque)?;
        c.execute_batch("CREATE TABLE IF NOT EXISTS empreintes (chemin TEXT PRIMARY KEY, taille INTEGER, date INTEGER, version INTEGER, valeur TEXT)").map_err(disque)?;
        Ok(Cache(c))
    }

    /// L'empreinte gardée d'un fichier, s'il n'a pas changé depuis.
    pub fn lire(&self, f: &Path, m: &std::fs::Metadata) -> Option<Empreinte> {
        self.0
            .query_row(
                "SELECT valeur FROM empreintes WHERE chemin = ?1 AND taille = ?2 AND date = ?3 AND version = ?4",
                rusqlite::params![f.to_string_lossy(), m.len() as i64, date_de(m), VERSION_CALCUL],
                |r| r.get::<_, String>(0),
            )
            .ok()
            .and_then(|v| serde_json::from_str(&v).ok())
    }

    pub fn garder(&self, f: &Path, m: &std::fs::Metadata, e: &Empreinte) {
        let _ = self.0.execute(
            "INSERT OR REPLACE INTO empreintes VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![f.to_string_lossy(), m.len() as i64, date_de(m), VERSION_CALCUL, serde_json::to_string(e).unwrap_or_default()],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sept_z::EntreeSeptZ;

    #[test]
    fn le_cache_rend_une_empreinte_tant_que_le_fichier_n_a_pas_change() {
        let d = tempfile::tempdir().unwrap();
        let c = Cache::ouvrir(&d.path().join("empreintes.db")).unwrap();
        let f = d.path().join("rom.nes");
        std::fs::write(&f, b"abc").unwrap();
        let e = Empreinte { crc32: Some("352441c2".into()), ..Default::default() };
        c.garder(&f, &std::fs::metadata(&f).unwrap(), &e);
        assert_eq!(c.lire(&f, &std::fs::metadata(&f).unwrap()), Some(e));
        std::fs::write(&f, b"abcd").unwrap();
        assert_eq!(c.lire(&f, &std::fs::metadata(&f).unwrap()), None, "taille changée : relu");
        assert!(sur_le_reseau(Path::new(r"\\serveur\jeux\a.nes")) && !sur_le_reseau(d.path()));
    }

    fn e(nom: &str, taille: u64, crc: Option<u32>, dossier: bool) -> EntreeSeptZ {
        EntreeSeptZ { nom: nom.into(), taille, dossier, anti: false, flux: taille > 0, attributs: None, crc }
    }

    #[test]
    fn crc_et_sha1_en_une_lecture() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("rom.nes");
        std::fs::write(&f, b"abc").unwrap();
        // Valeurs connues de « abc ».
        assert_eq!(crc_sha1(&f).unwrap(), ("352441c2".to_string(), "a9993e364706816aba3e25717850c26c9cd0d89d".to_string()));
    }

    #[test]
    fn les_entrees_suivent_le_contrat() {
        let l = entrees_du_contrat(&[e("18w2/", 0, None, true), e("18w2/18w_b1.rom1", 4, Some(0xdeadbeef), false), e("18w_b1.rom1", 4, Some(1), false), e("vide.txt", 0, None, false)]).unwrap();
        assert_eq!(l.iter().map(|x| (x.nom.as_str(), x.crc32.as_str())).collect::<Vec<_>>(), [("18w_b1.rom1", "deadbeef"), ("18w_b1.rom1", "00000001"), ("vide.txt", "00000000")], "dossier retiré, doublons gardés");
        assert_eq!(entrees_du_contrat(&[e("a.sfc", 5, None, false)]), None, "une entrée sans CRC : pas d'entrées du tout");
        assert_eq!(entrees_du_contrat(&[e("d/", 0, None, true)]), None);
    }

    #[test]
    fn la_nature_decide_ce_qu_on_lit() {
        assert_eq!(nature("Arcade", "005.zip"), Nature::Mame);
        assert_eq!(nature("Super Nintendo Entertainment System", "Mario.7z"), Nature::Archive);
        assert_eq!(nature("Nintendo Entertainment System", "Zelda.nes"), Nature::EnClair);
        assert_eq!(nature("Sega Dreamcast", "Jeu.CHD"), Nature::Disque);
        assert_eq!(nature("Sony Playstation", "Jeu.cue"), Nature::Disque);
    }
}
