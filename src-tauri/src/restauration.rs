//! Restaurer une sauvegarde (après un reformatage, ou sur un nouveau PC) : réglages, parties des émulateurs, et les
//! parties des jeux, reposées quand chaque jeu est réinstallé.
//!
//! Les fichiers sont d'abord téléchargés dans une zone d'attente (`restauration/<profil>/parties/`), chacun vérifié
//! par son empreinte (sha256). Rien de ce qui est déjà sur le PC n'est écrasé sans être identique, sauf les fichiers
//! d'un jeu qu'on vient de réinstaller (ce sont alors ceux du jeu tout neuf que la partie remplace).

use crate::erreurs::{Erreur, Resultat};
use crate::firehouse::Client;
use crate::installation::fichiers_de;
use crate::noyau::Noyau;
use crate::sauvegarde::{empreinte, nom_pour_api, ManifesteParties};
use serde::Serialize;
use serde_json::Value;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Une sauvegarde qu'on peut restaurer.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SauvegardeDisponible {
    pub profil: String,
    pub pc: String,
    /// Le contenu de `derniere-sauvegarde.json` (date, fichiers, taille…), s'il existe.
    pub derniere: Value,
    pub octets: u64,
}

/// Ce que la restauration a préparé.
#[derive(Debug, Clone, Serialize)]
pub struct RestaurationPrete {
    pub configuration: Value,
    pub bibliotheque: Value,
    pub fichiers: usize,
    pub taille: u64,
}

/// Ce que la restauration a remis en place.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct Reposes {
    pub reposes: usize,
    pub deja_la: usize,
    /// Fichiers différents déjà présents : gardés tels quels (la version restaurée reste en attente).
    pub conflits: Vec<String>,
}

pub async fn lister(client: &Client) -> Resultat<Vec<SauvegardeDisponible>> {
    let v: Value = client.obtenir_json("/sauvegarde").await?;
    Ok(v["profils"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|p| {
            Some(SauvegardeDisponible {
                profil: p["profil"].as_str()?.into(),
                pc: p["pc"].as_str()?.into(),
                derniere: p["derniere"].clone(),
                octets: p["octets"].as_u64().unwrap_or(0),
            })
        })
        .collect())
}

/// Télécharge un fichier de la sauvegarde (reprise `Range`), et vérifie son empreinte avant de le garder.
async fn telecharger(client: &Client, route: &str, cible: &Path, sha: &str, taille: u64) -> Resultat<()> {
    if cible.is_file() && std::fs::metadata(cible)?.len() == taille && empreinte(cible)? == sha {
        return Ok(()); // déjà là (restauration reprise)
    }
    if let Some(p) = cible.parent() {
        std::fs::create_dir_all(p)?;
    }
    let part = cible.with_extension("part-frogtend");
    let debut = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0).min(taille);
    let mut flux = client.flux(route, debut).await?;
    let mut sortie = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(flux.reprise && debut > 0)
        .truncate(!(flux.reprise && debut > 0))
        .open(&part)?;
    while let Some(m) = Client::morceau(&mut flux).await? {
        sortie.write_all(&m)?;
    }
    sortie.flush()?;
    drop(sortie);
    if std::fs::metadata(&part)?.len() != taille || empreinte(&part)? != sha {
        let _ = std::fs::remove_file(&part);
        return Err(Erreur::Conflit(format!("« {} » est arrivé abîmé : relance la restauration.", cible.display())));
    }
    std::fs::rename(&part, cible)?;
    Ok(())
}

/// Copie les fichiers de `de` dans `vers` : un fichier absent est posé, un fichier identique compté, un fichier
/// DIFFÉRENT gardé tel quel (conflit) — sauf si `ecraser`.
pub fn reposer(de: &Path, vers: &Path, ecraser: bool) -> Resultat<Reposes> {
    let mut r = Reposes::default();
    if !de.is_dir() {
        return Ok(r);
    }
    for f in fichiers_de(de)? {
        let (source, cible) = (de.join(&f), vers.join(&f));
        if cible.is_file() {
            if empreinte(&cible)? == empreinte(&source)? {
                r.deja_la += 1;
                continue;
            }
            if !ecraser {
                r.conflits.push(f);
                continue;
            }
        }
        if let Some(p) = cible.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::copy(&source, &cible)?;
        r.reposes += 1;
    }
    Ok(r)
}

impl Noyau {
    fn zone_restauration(&self, profil_id: &str) -> PathBuf {
        self.dossier.join("restauration").join(profil_id)
    }

    /// Les sauvegardes du compte de ce jeton, chez Firehouse.
    pub async fn sauvegardes_disponibles(&self) -> Resultat<Vec<SauvegardeDisponible>> {
        let s = self.session().await?;
        match &s.source {
            crate::source::Source::Firehouse(c) => lister(c).await,
            crate::source::Source::Simulee => Ok(vec![]),
        }
    }

    /// Télécharge une sauvegarde (`profil` et `pc` tels que Firehouse les liste) dans la zone d'attente du profil
    /// ouvert. `progres(fichiers faits, fichiers en tout)`.
    pub async fn preparer_restauration(
        &self,
        profil: &str,
        pc: &str,
        progres: &(dyn Fn(usize, usize) + Send + Sync),
    ) -> Resultat<RestaurationPrete> {
        let s = self.session().await?;
        let crate::source::Source::Firehouse(c) = &s.source else {
            return Err(Erreur::Refus("En mode simulé, il n'y a rien à restaurer.".into()));
        };
        let (p, m) = (nom_pour_api(profil), nom_pour_api(pc));
        let base = format!("/sauvegarde/{}/{}", crate::source::encoder(&p), crate::source::encoder(&m));
        let document = |nom: &'static str| {
            let route = format!("{base}/document/{nom}");
            async move { c.obtenir_json::<Value>(&route).await }
        };
        let configuration = document("configuration.json").await?;
        let bibliotheque = document("bibliotheque.json").await?;
        let manifeste: ManifesteParties = serde_json::from_value(document("manifeste.json").await?)
            .map_err(|_| Erreur::Serveur("Le manifeste de la sauvegarde est illisible.".into()))?;

        let zone = self.zone_restauration(&s.profil.id).join("parties");
        let total = manifeste.fichiers.len();
        let mut taille = 0;
        for (i, (relatif, (sha, t))) in manifeste.fichiers.iter().enumerate() {
            // Un chemin de la sauvegarde ne sort jamais de la zone d'attente.
            if relatif.split('/').any(|s| s == ".." || s.is_empty()) || relatif.contains(['\\', ':']) {
                return Err(Erreur::Refus(format!("Chemin refusé dans la sauvegarde : « {relatif} ».")));
            }
            let route = format!("{base}/fichier?chemin={}", crate::source::encoder(relatif));
            telecharger(c, &route, &zone.join(relatif), sha, *t).await?;
            taille += t;
            progres(i + 1, total);
        }
        Ok(RestaurationPrete { configuration, bibliotheque, fichiers: total, taille })
    }

    /// Repose les parties des émulateurs (en attente) dans le dossier du profil de chaque émulateur présent.
    /// `programmes_emulateurs` : les programmes des émulateurs réglés sur ce PC.
    pub async fn reposer_parties_emulateurs(&self, programmes_emulateurs: &[String]) -> Resultat<Reposes> {
        let s = self.session().await?;
        let zone = self.zone_restauration(&s.profil.id).join("parties").join("emulateurs");
        let mut total = Reposes::default();
        for programme in programmes_emulateurs {
            let Some(dossier) = Path::new(programme).parent() else { continue };
            let nom = crate::jeux_pc::nom_de_dossier(&dossier.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
            let r = reposer(&zone.join(&nom), &crate::emulateurs_profils::dossier_du_profil(dossier, &s.profil.nom), false)?;
            total.reposes += r.reposes;
            total.deja_la += r.deja_la;
            total.conflits.extend(r.conflits.into_iter().map(|f| format!("{nom}/{f}")));
        }
        Ok(total)
    }

    /// Juste après l'installation d'un jeu : si ses parties attendent d'être restaurées, elles sont reposées dans son
    /// dossier (elles remplacent les fichiers du jeu tout neuf), puis retirées de la zone d'attente.
    pub(crate) fn reposer_parties_du_jeu(&self, profil_id: &str, id: i64, dossier: &Path) -> Resultat<Reposes> {
        let attente = self.zone_restauration(profil_id).join("parties").join("jeux").join(id.to_string());
        if !attente.is_dir() {
            return Ok(Reposes::default());
        }
        let r = reposer(&attente, dossier, true)?;
        std::fs::remove_dir_all(&attente)?;
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use serde_json::json;

    #[test]
    fn reposer_ne_remplace_jamais_une_partie_differente_sans_le_dire() {
        let d = tempfile::tempdir().unwrap();
        let (de, vers) = (d.path().join("de"), d.path().join("vers"));
        std::fs::create_dir_all(de.join("saves")).unwrap();
        std::fs::create_dir_all(vers.join("saves")).unwrap();
        std::fs::write(de.join("saves/a.srm"), b"A").unwrap();
        std::fs::write(de.join("saves/b.srm"), b"B restauree").unwrap();
        std::fs::write(de.join("saves/c.srm"), b"C").unwrap();
        std::fs::write(vers.join("saves/b.srm"), b"B jouee sur ce PC").unwrap();
        std::fs::write(vers.join("saves/c.srm"), b"C").unwrap();

        let r = reposer(&de, &vers, false).unwrap();
        assert_eq!((r.reposes, r.deja_la, r.conflits.clone()), (1, 1, vec!["saves/b.srm".to_string()]));
        assert_eq!(std::fs::read(vers.join("saves/b.srm")).unwrap(), b"B jouee sur ce PC");
        assert_eq!(std::fs::read(vers.join("saves/a.srm")).unwrap(), b"A");

        // Un jeu tout juste réinstallé : la partie restaurée remplace le fichier du jeu neuf.
        assert_eq!(reposer(&de, &vers, true).unwrap().reposes, 1);
        assert_eq!(std::fs::read(vers.join("saves/b.srm")).unwrap(), b"B restauree");
    }

    #[tokio::test]
    async fn une_sauvegarde_se_telecharge_et_chaque_fichier_est_verifie() {
        let s = MockServer::start();
        let base = "/api/jeux/v1/sauvegarde/Sebastien/VENKMAN";
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("x");
        std::fs::write(&f, b"partie").unwrap();
        let sha = empreinte(&f).unwrap();
        s.mock(|w, t| {
            w.method(GET).path(format!("{base}/document/configuration.json"));
            t.status(200).json_body(json!({"reglages_profil": {"reglages": {"apparence": {"skin": "dracula"}}}}));
        });
        s.mock(|w, t| {
            w.method(GET).path(format!("{base}/document/bibliotheque.json"));
            t.status(200).json_body(json!({"jeux": [{"id": 110, "titre": "Dune"}]}));
        });
        s.mock(|w, t| {
            w.method(GET).path(format!("{base}/document/manifeste.json"));
            t.status(200).json_body(json!({"fichiers": {
                "jeux/110/C/DUNECD/DUNE37S0.SAV": [sha, 6],
                "emulateurs/RetroArch/saves/Super Metroid (USA).srm": [sha, 6]}}));
        });
        s.mock(|w, t| {
            w.method(GET).path(format!("{base}/fichier"));
            t.status(200).body(b"partie");
        });
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Sébastien", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &crate::noyau::Connexion { adresse: s.base_url(), simule: false }).await.unwrap();

        let r = n.preparer_restauration("Sebastien", "VENKMAN", &|_, _| {}).await.unwrap();
        assert_eq!((r.fichiers, r.taille), (2, 12));
        assert_eq!(r.configuration["reglages_profil"]["reglages"]["apparence"]["skin"], "dracula");

        // Les parties de l'émulateur vont dans le dossier du profil de RetroArch.
        let emu = d.path().join("Emulateurs").join("RetroArch");
        std::fs::create_dir_all(&emu).unwrap();
        let rep = n.reposer_parties_emulateurs(&[emu.join("retroarch.exe").to_string_lossy().into()]).await.unwrap();
        assert_eq!(rep.reposes, 1);
        assert!(crate::emulateurs_profils::dossier_du_profil(&emu, "Sébastien").join("saves/Super Metroid (USA).srm").is_file());

        // Celles de Dune attendent sa réinstallation, puis y sont reposées.
        let jeu = d.path().join("Dune");
        std::fs::create_dir_all(jeu.join("C/DUNECD")).unwrap();
        let rj = n.reposer_parties_du_jeu(&id, 110, &jeu).unwrap();
        assert_eq!(rj.reposes, 1);
        assert_eq!(std::fs::read(jeu.join("C/DUNECD/DUNE37S0.SAV")).unwrap(), b"partie");
        assert_eq!(n.reposer_parties_du_jeu(&id, 110, &jeu).unwrap().reposes, 0, "reposées une seule fois");
    }

    #[tokio::test]
    async fn un_fichier_abime_est_refuse() {
        let s = MockServer::start();
        let base = "/api/jeux/v1/sauvegarde/Seb/PC";
        for (nom, corps) in [("configuration.json", json!({})), ("bibliotheque.json", json!({})),
            ("manifeste.json", json!({"fichiers": {"a.sav": ["0000", 6]}}))] {
            s.mock(|w, t| {
                w.method(GET).path(format!("{base}/document/{nom}"));
                t.status(200).json_body(corps.clone());
            });
        }
        s.mock(|w, t| {
            w.method(GET).path(format!("{base}/fichier"));
            t.status(200).body(b"partie");
        });
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(d.path(), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &crate::noyau::Connexion { adresse: s.base_url(), simule: false }).await.unwrap();
        assert!(matches!(n.preparer_restauration("Seb", "PC", &|_, _| {}).await, Err(Erreur::Conflit(_))));
    }
}
