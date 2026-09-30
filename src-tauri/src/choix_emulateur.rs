//! Plusieurs émulateurs (ou cœurs RetroArch) par système, un par défaut pour le système, un par défaut possible pour
//! chaque jeu, et un choix ponctuel au lancement (Seb, 30/09 : « on doit être souple »).
//!
//! Réglages du PC (`pc.json` ▸ `reglages`) :
//! - `emulateurs[<système>] = { liste: [{cle, nom, programme, ligne}], defaut: <cle> }` ;
//!   l'ancienne forme `{programme, ligne, nom}` (avant 0.10) est lue comme une liste d'un seul ;
//! - `emulateursJeux[<id du jeu>] = <cle>` : le défaut propre à un jeu.

use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct EmulateurRegle {
    pub cle: String,
    pub nom: String,
    pub programme: String,
    pub ligne: String,
}

fn lire(e: &Value, cle_par_defaut: &str) -> Option<EmulateurRegle> {
    let programme = e["programme"].as_str().filter(|p| !p.is_empty())?.to_string();
    Some(EmulateurRegle {
        cle: e["cle"].as_str().filter(|c| !c.is_empty()).unwrap_or(cle_par_defaut).to_string(),
        nom: e["nom"].as_str().unwrap_or("").to_string(),
        programme,
        ligne: e["ligne"].as_str().unwrap_or("").to_string(),
    })
}

/// Les émulateurs réglés pour un système, et la clé de celui par défaut.
pub fn du_systeme(reglages: &Value, plateforme: &str) -> (Vec<EmulateurRegle>, Option<String>) {
    let s = &reglages["emulateurs"][plateforme];
    if let Some(l) = s["liste"].as_array() {
        let liste: Vec<EmulateurRegle> = l.iter().enumerate().filter_map(|(i, e)| lire(e, &format!("{}", i + 1))).collect();
        let defaut = s["defaut"].as_str().map(String::from).filter(|d| liste.iter().any(|e| &e.cle == d));
        let defaut = defaut.or_else(|| liste.first().map(|e| e.cle.clone()));
        return (liste, defaut);
    }
    // Ancienne forme : un seul émulateur.
    match lire(s, "1") {
        Some(e) => {
            let c = e.cle.clone();
            (vec![e], Some(c))
        }
        None => (vec![], None),
    }
}

/// L'émulateur à utiliser pour un jeu : celui demandé pour ce lancement, sinon le défaut du jeu, sinon celui du
/// système. Une clé qui n'existe plus (émulateur retiré) retombe sur le suivant.
pub fn pour_le_jeu(reglages: &Value, plateforme: &str, jeu: i64, demande: Option<&str>) -> Option<EmulateurRegle> {
    let (liste, defaut) = du_systeme(reglages, plateforme);
    let du_jeu = reglages["emulateursJeux"][jeu.to_string()].as_str().map(String::from);
    for cle in [demande.map(String::from), du_jeu, defaut].into_iter().flatten() {
        if let Some(e) = liste.iter().find(|e| e.cle == cle) {
            return Some(e.clone());
        }
    }
    liste.into_iter().next()
}

/// Tous les programmes d'émulateurs réglés sur ce PC (pour la sauvegarde des parties), sans doublon.
pub fn tous_les_programmes(reglages: &Value) -> Vec<String> {
    let mut l: Vec<String> = Vec::new();
    for plateforme in reglages["emulateurs"].as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()).unwrap_or_default() {
        for e in du_systeme(reglages, &plateforme).0 {
            if !l.contains(&e.programme) {
                l.push(e.programme);
            }
        }
    }
    l
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn reglages() -> Value {
        json!({
            "emulateurs": {
                "Super Nintendo": {
                    "liste": [
                        {"cle": "snes9x", "nom": "RetroArch — snes9x", "programme": "E:\\Emu\\RetroArch\\retroarch.exe", "ligne": "-L cores\\snes9x_libretro.dll"},
                        {"cle": "bsnes", "nom": "RetroArch — bsnes", "programme": "E:\\Emu\\RetroArch\\retroarch.exe", "ligne": "-L cores\\bsnes_libretro.dll"}
                    ],
                    "defaut": "snes9x"
                },
                "MS-DOS": {"programme": "E:\\Emu\\DOSBox\\dosbox.exe", "ligne": "", "nom": "DOSBox Staging"}
            },
            "emulateursJeux": {"42": "bsnes", "43": "disparu"}
        })
    }

    #[test]
    fn le_jeu_passe_avant_le_systeme_et_le_choix_du_lancement_avant_tout() {
        let r = reglages();
        assert_eq!(pour_le_jeu(&r, "Super Nintendo", 1, None).unwrap().cle, "snes9x", "défaut du système");
        assert_eq!(pour_le_jeu(&r, "Super Nintendo", 42, None).unwrap().cle, "bsnes", "défaut du jeu");
        assert_eq!(pour_le_jeu(&r, "Super Nintendo", 42, Some("snes9x")).unwrap().cle, "snes9x", "choix du lancement");
        assert_eq!(pour_le_jeu(&r, "Super Nintendo", 43, None).unwrap().cle, "snes9x", "un défaut retiré retombe");
        assert!(pour_le_jeu(&r, "Nintendo 64", 1, None).is_none());
    }

    #[test]
    fn l_ancienne_forme_est_lue_comme_une_liste_d_un_seul() {
        let r = reglages();
        let (l, d) = du_systeme(&r, "MS-DOS");
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].nom, "DOSBox Staging");
        assert_eq!(d.as_deref(), Some("1"));
        assert_eq!(pour_le_jeu(&r, "MS-DOS", 110, None).unwrap().programme, "E:\\Emu\\DOSBox\\dosbox.exe");
    }

    #[test]
    fn tous_les_programmes_sans_doublon() {
        assert_eq!(tous_les_programmes(&reglages()), vec!["E:\\Emu\\DOSBox\\dosbox.exe", "E:\\Emu\\RetroArch\\retroarch.exe"]);
    }
}
