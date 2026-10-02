//! Cheat Engine (lot 8) : fourni en zip par Seb via Firehouse, sans option portable — ses réglages vont dans le
//! registre (`HKCU\Software\Cheat Engine`, relevé dans `Cheat Engine/ceregistry.pas`). Accord de Seb (02/10) :
//! Frogtend remet la clé du PROFIL avant de lancer Cheat Engine, et la range dans le dossier du profil à la fermeture.
//! Rien ne s'efface sans preuve : la clé n'est retirée du registre qu'après un export vérifié ; la toute première clé
//! trouvée (celle d'avant Frogtend) est gardée à part, une fois pour toutes.

use crate::erreurs::{Erreur, Resultat};
use std::path::{Path, PathBuf};
use std::process::Command;

/// La clé de Cheat Engine.
pub const CLE: &str = r"HKCU\Software\Cheat Engine";

fn reg(args: &[&str]) -> Resultat<std::process::Output> {
    Command::new("reg").args(args).output().map_err(|e| Erreur::Disque(format!("reg.exe ne répond pas ({e}).")))
}

pub fn cle_existe(cle: &str) -> bool {
    reg(&["query", cle]).is_ok_and(|o| o.status.success())
}

/// Exporte la clé dans un fichier .reg et vérifie qu'il est bien écrit.
pub fn exporter(cle: &str, fichier: &Path) -> Resultat<()> {
    if let Some(p) = fichier.parent() {
        std::fs::create_dir_all(p)?;
    }
    let provisoire = fichier.with_extension("reg.provisoire");
    let o = reg(&["export", cle, &provisoire.to_string_lossy(), "/y"])?;
    let ecrit = std::fs::metadata(&provisoire).map(|m| m.len()).unwrap_or(0);
    if !o.status.success() || ecrit < 16 {
        let _ = std::fs::remove_file(&provisoire);
        return Err(Erreur::Disque(format!("L'export de {cle} a échoué : rien n'est retiré du registre.")));
    }
    std::fs::rename(&provisoire, fichier)?;
    Ok(())
}

pub fn importer(fichier: &Path) -> Resultat<()> {
    let o = reg(&["import", &fichier.to_string_lossy()])?;
    if o.status.success() { Ok(()) } else { Err(Erreur::Disque(format!("Import de {} refusé par Windows.", fichier.display()))) }
}

/// Retire la clé, SEULEMENT si `preuve` (son export) existe.
fn retirer(cle: &str, preuve: &Path) -> Resultat<()> {
    if !preuve.is_file() {
        return Err(Erreur::Refus("Pas d'export de la clé : rien n'est retiré.".into()));
    }
    let o = reg(&["delete", cle, "/f"])?;
    if o.status.success() { Ok(()) } else { Err(Erreur::Disque(format!("Windows refuse de retirer {cle}."))) }
}

/// Le script de Frogtend pour Cheat Engine (`autorun\frogtend.lua`).
pub const SCRIPT_AUTORUN: &str = include_str!("../ressources/cheatengine/frogtend.lua");

/// Prépare le branchement automatique : pose (ou met à jour) `autorun\frogtend.lua`, et écrit le fichier qu'il lira
/// (processus du jeu, table éventuelle).
pub fn preparer_branchement(dossier_ce: &Path, pid: u32, table: Option<&Path>) -> Resultat<()> {
    let autorun = dossier_ce.join("autorun");
    std::fs::create_dir_all(&autorun)?;
    let script = autorun.join("frogtend.lua");
    if std::fs::read_to_string(&script).ok().as_deref() != Some(SCRIPT_AUTORUN) {
        std::fs::write(&script, SCRIPT_AUTORUN)?;
    }
    let mut contenu = format!("pid={pid}\n");
    if let Some(t) = table {
        contenu.push_str(&format!("table={}\n", t.to_string_lossy()));
    }
    std::fs::write(dossier_ce.join("frogtend-lancement.txt"), contenu)?;
    Ok(())
}

/// Le fichier des réglages Cheat Engine d'un profil.
pub fn fichier_du_profil(dossier_ce: &Path, profil: &str) -> PathBuf {
    crate::emulateurs_profils::dossier_du_profil(dossier_ce, profil).join("cheatengine.reg")
}

/// Avant de lancer Cheat Engine pour un profil : la clé d'origine est gardée une fois ; une clé restée là (Cheat
/// Engine ouvert hors de Frogtend) est exportée puis retirée ; enfin les réglages du profil sont remis.
pub fn preparer(cle: &str, dossier_ce: &Path, profil: &str) -> Resultat<()> {
    let abri = dossier_ce.join(".frogtend-sauvegardes");
    if cle_existe(cle) {
        let origine = abri.join("registre-avant-frogtend.reg");
        let f = if origine.exists() { abri.join(format!("registre-{}.reg", crate::noyau::maintenant())) } else { origine };
        exporter(cle, &f)?;
        retirer(cle, &f)?;
    }
    let du_profil = fichier_du_profil(dossier_ce, profil);
    if du_profil.is_file() {
        importer(&du_profil)?;
    }
    Ok(())
}

/// À la fermeture de Cheat Engine : ses réglages vont dans le dossier du profil, puis la clé quitte le registre.
pub fn ranger(cle: &str, dossier_ce: &Path, profil: &str) -> Resultat<()> {
    if !cle_existe(cle) {
        return Ok(());
    }
    let f = fichier_du_profil(dossier_ce, profil);
    exporter(cle, &f)?;
    retirer(cle, &f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_branchement_pose_le_script_et_le_fichier_du_jeu() {
        let d = tempfile::tempdir().unwrap();
        preparer_branchement(d.path(), 4242, Some(Path::new("E:\\Tables\\Dune.CT"))).unwrap();
        assert_eq!(std::fs::read_to_string(d.path().join("autorun/frogtend.lua")).unwrap(), SCRIPT_AUTORUN);
        assert_eq!(std::fs::read_to_string(d.path().join("frogtend-lancement.txt")).unwrap(), "pid=4242\ntable=E:\\Tables\\Dune.CT\n");
        assert!(SCRIPT_AUTORUN.contains("openProcess(pid)") && SCRIPT_AUTORUN.contains("os.remove(fichier)"));
        preparer_branchement(d.path(), 7, None).unwrap();
        assert_eq!(std::fs::read_to_string(d.path().join("frogtend-lancement.txt")).unwrap(), "pid=7\n");
    }

    /// Sur une clé d'ESSAI (jamais celle de Cheat Engine), retirée à la fin.
    #[test]
    fn la_cle_va_et_vient_par_profil_sans_jamais_rien_perdre() {
        let cle = format!(r"HKCU\Software\FrogtendEssai{}", std::process::id());
        let d = tempfile::tempdir().unwrap();
        let ajouter = |nom: &str, v: &str| assert!(reg(&["add", &cle, "/v", nom, "/t", "REG_SZ", "/d", v, "/f"]).unwrap().status.success());
        let lire = |nom: &str| String::from_utf8_lossy(&reg(&["query", &cle, "/v", nom]).unwrap().stdout).to_string();

        // Une clé d'avant Frogtend : gardée à part, puis retirée.
        ajouter("Origine", "seb");
        preparer(&cle, d.path(), "Seb").unwrap();
        assert!(!cle_existe(&cle));
        assert!(d.path().join(".frogtend-sauvegardes/registre-avant-frogtend.reg").is_file());

        // Seb règle Cheat Engine ; à la fermeture, ses réglages vont dans son profil.
        ajouter("Theme", "sombre");
        ranger(&cle, d.path(), "Seb").unwrap();
        assert!(!cle_existe(&cle));
        assert!(fichier_du_profil(d.path(), "Seb").is_file());

        // Léa : pas de réglages à elle → clé vide ; ceux de Seb ne la suivent pas.
        preparer(&cle, d.path(), "Léa").unwrap();
        assert!(!cle_existe(&cle));
        // Seb revient : ses réglages reviennent.
        preparer(&cle, d.path(), "Seb").unwrap();
        assert!(lire("Theme").contains("sombre"));
        ranger(&cle, d.path(), "Seb").unwrap();
        assert!(!cle_existe(&cle));

        // Retirer sans preuve est refusé.
        ajouter("X", "1");
        assert!(retirer(&cle, &d.path().join("absent.reg")).is_err());
        assert!(cle_existe(&cle));
        let _ = reg(&["delete", &cle, "/f"]);
    }
}
