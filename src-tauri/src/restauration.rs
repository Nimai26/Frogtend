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
    let mut debut = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0).min(taille);
    let mut flux = match client.flux(route, debut).await {
        Ok(f) => f,
        // 416 : le morceau déjà reçu ne correspond plus ; ce fichier de Frogtend (refait de toute façon si son empreinte
        // est fausse) repart de zéro, une seule fois — sinon le même refus reviendrait à chaque essai.
        Err(Erreur::Plage(_)) if debut > 0 => {
            std::fs::remove_file(&part)?;
            debut = 0;
            client.flux(route, 0).await?
        }
        Err(e) => return Err(e),
    };
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

fn lire_json_ou_rien(p: &Path) -> Value {
    std::fs::read(p).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or(Value::Null)
}

/// Le nom sous lequel les parties d'un émulateur sont rangées dans la sauvegarde : celui de son dossier.
fn nom_du_dossier_de(programme: &str) -> Option<String> {
    let d = Path::new(programme).parent()?.file_name()?.to_string_lossy().to_string();
    Some(crate::jeux_pc::nom_de_dossier(&d))
}

/// Les programmes d'émulateurs de l'ancien PC, lus dans ses réglages restaurés (toute valeur `programme`, quelle que
/// soit la forme des réglages) : (nom de leur dossier, nom de leur programme en minuscules).
pub fn anciens_programmes(configuration: &Value) -> Vec<(String, String)> {
    fn chercher(v: &Value, l: &mut Vec<(String, String)>) {
        match v {
            Value::Object(m) => {
                for (k, x) in m {
                    if let (true, Some(p)) = (k == "programme", x.as_str()) {
                        let exe = p.rsplit(['\\', '/']).next().unwrap_or("").to_lowercase();
                        if let (Some(nom), false) = (nom_du_dossier_de(&p.replace('/', "\\")), exe.is_empty()) {
                            l.push((nom, exe));
                        }
                    } else {
                        chercher(x, l);
                    }
                }
            }
            Value::Array(a) => a.iter().for_each(|x| chercher(x, l)),
            _ => {}
        }
    }
    let mut l = Vec::new();
    chercher(&configuration["reglages_pc"], &mut l);
    l.sort();
    l.dedup();
    l
}

/// Le dossier d'attente des parties de CET émulateur : celui à son nom de dossier ; sinon celui de l'ancien émulateur
/// qui avait le même programme (`rpcs3.exe`), s'il est le SEUL (deux anciens RetroArch : on ne choisit pas).
pub fn cle_en_attente(zone: &Path, programme: &str, anciens: &[(String, String)]) -> Option<String> {
    let nom = nom_du_dossier_de(programme)?;
    if zone.join(&nom).is_dir() {
        return Some(nom);
    }
    let exe = programme.rsplit(['\\', '/']).next()?.to_lowercase();
    let mut l: Vec<&String> = anciens.iter().filter(|(d, e)| *e == exe && zone.join(d).is_dir()).map(|(d, _)| d).collect();
    l.dedup();
    match l.as_slice() {
        [un] => Some((*un).clone()),
        _ => None,
    }
}

/// Les dossiers d'attente de TOUS les émulateurs présents, chacun donné à UN SEUL émulateur (jamais les mêmes parties
/// copiées chez deux émulateurs qui divergeraient ensuite) : par le nom exact du dossier d'abord ; un rapprochement par
/// le programme seulement s'il est seul à le revendiquer. Dans le doute : personne.
pub fn cles_en_attente(zone: &Path, programmes: &[String], anciens: &[(String, String)]) -> Vec<(String, String)> {
    let mut l: Vec<(String, String, bool)> = Vec::new();
    for p in programmes {
        let exact = nom_du_dossier_de(p).is_some_and(|n| zone.join(&n).is_dir());
        if let Some(c) = cle_en_attente(zone, p, anciens) {
            if !l.iter().any(|(x, _, _)| x == p) {
                l.push((p.clone(), c, exact));
            }
        }
    }
    l.iter()
        .filter(|(_, c, exact)| {
            let memes: Vec<&(String, String, bool)> = l.iter().filter(|(_, x, _)| x == c).collect();
            let exacts = memes.iter().filter(|(_, _, e)| *e).count();
            if *exact { exacts == 1 } else { exacts == 0 && memes.len() == 1 }
        })
        .map(|(p, c, _)| (p.clone(), c.clone()))
        .collect()
}

/// Copie les fichiers de `de` dans `vers` : un fichier absent est posé, un fichier identique compté, un fichier
/// DIFFÉRENT gardé tel quel (conflit) — sauf si `ecraser`.
pub fn reposer(de: &Path, vers: &Path, ecraser: bool) -> Resultat<Reposes> {
    reposer_sauf(de, vers, ecraser, &[])
}

/// Les parties d'un compte RPCS3 (`savedata\<partie>\…`, `trophy\<jeu>\…`) : chaque partie est un DOSSIER de fichiers
/// qui vont ensemble (PARAM.SFO, SAVE.DAT, icônes ; TROPUSR.DAT et TROPTRNS.DAT). Elle est reposée ENTIÈRE si elle
/// n'est pas sur ce PC ; déjà là à l'identique, rien ; si elle diffère en quoi que ce soit, on n'y touche pas du tout
/// et c'est un conflit (`savedata/<partie>`) — jamais un mélange de deux parties.
pub fn reposer_parties_entieres(de: &Path, vers: &Path) -> Resultat<Reposes> {
    let mut r = Reposes::default();
    for sorte in crate::emulateurs_profils::PARTIES_RPCS3 {
        let c = reposer_dossiers_entiers(&de.join(sorte), &vers.join(sorte), sorte)?;
        r.reposes += c.reposes;
        r.deja_la += c.deja_la;
        r.conflits.extend(c.conflits);
    }
    Ok(r)
}

/// Chaque sous-dossier de `de` (une partie : ses fichiers vont ensemble) reposé ENTIER dans `vers` s'il n'y est pas ;
/// à l'identique, rien ; s'il diffère en quoi que ce soit, on n'y touche pas et c'est un conflit (`<etiquette>/<nom>`).
pub fn reposer_dossiers_entiers(de: &Path, vers: &Path, etiquette: &str) -> Resultat<Reposes> {
    let mut r = Reposes::default();
    {
        for e in std::fs::read_dir(de).into_iter().flatten().flatten() {
            if !e.file_type().is_ok_and(|t| t.is_dir()) {
                continue;
            }
            let nom = e.file_name().to_string_lossy().to_string();
            let (source, cible) = (e.path(), vers.join(&nom));
            let fichiers = fichiers_de(&source)?;
            if !cible.exists() {
                // Par un dossier provisoire, vérifié puis renommé : jamais une partie à moitié reposée.
                r.reposes += crate::eden::copier_dossier_entier(&source, &cible)?;
                continue;
            }
            let mut identique = fichiers_de(&cible)? == fichiers;
            for f in &fichiers {
                if !identique {
                    break;
                }
                identique = empreinte(&cible.join(f))? == empreinte(&source.join(f))?;
            }
            if identique {
                r.deja_la += fichiers.len();
            } else {
                r.conflits.push(format!("{etiquette}/{nom}"));
            }
        }
    }
    Ok(r)
}

/// Comme [`reposer`], sans les sous-dossiers `sauf` (de premier niveau) : ils ont leur propre place (le compte RPCS3,
/// l'utilisateur Eden).
pub fn reposer_sauf(de: &Path, vers: &Path, ecraser: bool, sauf: &[&str]) -> Resultat<Reposes> {
    let mut r = Reposes::default();
    if !de.is_dir() {
        return Ok(r);
    }
    let a_part: Vec<String> = sauf.iter().map(|s| format!("{s}/")).collect();
    for f in fichiers_de(de)?.into_iter().filter(|f| !a_part.iter().any(|a| f.starts_with(a))) {
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

        // La liste des jeux restaurés, gardée sur le PC : une sauvegarde suivante la garde tant qu'ils ne sont pas remis.
        // Et les réglages : ils disent où étaient les émulateurs de l'ancien PC (pour leur rendre leurs parties).
        let attente = self.zone_restauration(&s.profil.id);
        std::fs::create_dir_all(&attente)?;
        std::fs::write(attente.join("bibliotheque.json"), serde_json::to_vec_pretty(&bibliotheque).unwrap_or_default())?;
        std::fs::write(attente.join("configuration.json"), serde_json::to_vec_pretty(&configuration).unwrap_or_default())?;
        let zone = attente.join("parties");
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

    /// Repose les parties des émulateurs (en attente) dans le dossier du profil de chaque émulateur présent. Elles sont
    /// rangées sous le nom du DOSSIER de l'émulateur de l'ancien PC ; s'il a changé (installé à la main, sous-dossier
    /// numéroté…), elles sont retrouvées par le nom de son programme (`rpcs3.exe`), seulement s'il n'y a aucun doute.
    /// `programmes_emulateurs` : les programmes des émulateurs réglés sur ce PC.
    pub async fn reposer_parties_emulateurs(&self, programmes_emulateurs: &[String]) -> Resultat<Reposes> {
        let s = self.session().await?;
        let attente = self.zone_restauration(&s.profil.id);
        let zone = attente.join("parties").join("emulateurs");
        // Les programmes des émulateurs de l'ANCIEN PC (réglages restaurés) : leur dossier → leur programme.
        let anciens = anciens_programmes(&lire_json_ou_rien(&attente.join("configuration.json")));
        let mut total = Reposes::default();
        for (programme, nom) in cles_en_attente(&zone, programmes_emulateurs, &anciens) {
            let Some(dossier) = Path::new(&programme).parent() else { continue };
            let mut r = reposer_sauf(
                &zone.join(&nom),
                &crate::emulateurs_profils::dossier_du_profil(dossier, &s.profil.nom),
                false,
                &[crate::emulateurs_profils::COMPTE_RPCS3, crate::eden::UTILISATEUR_EDEN],
            )?;
            // Eden : les parties de l'utilisateur Switch du profil reviennent dans SON utilisateur (créé s'il manque,
            // AVANT les parties : sinon Eden les prendrait pour celles d'un utilisateur disparu) — Eden portable et fermé
            // seulement ; sinon elles attendent (gardées chez Firehouse).
            let attente_eden = zone.join(&nom).join(crate::eden::UTILISATEUR_EDEN);
            if attente_eden.is_dir()
                && crate::eden::est_eden(&programme)
                && crate::eden::portable(dossier)
                && crate::lancement::processus_du_dossier(dossier).is_empty()
            {
                // Un échec (8 utilisateurs, fichier abîmé) est DIT et n'arrête pas les autres émulateurs ; les parties
                // attendent (gardées chez Firehouse).
                match crate::eden::utilisateur_du_profil(dossier, &crate::jeux_pc::nom_de_dossier(&s.profil.nom)) {
                    Ok(u) => {
                        for (sorte, vers) in crate::eden::dossiers_parties(dossier, &u.uuid) {
                            let etiquette = if sorte == "save" { "Switch" } else { "Switch (compte)" };
                            let c = reposer_dossiers_entiers(&attente_eden.join(sorte), &vers, etiquette)?;
                            r.reposes += c.reposes;
                            r.deja_la += c.deja_la;
                            r.conflits.extend(c.conflits);
                        }
                    }
                    Err(e) => r.conflits.push(format!("Switch : {e}")),
                }
            }
            // RPCS3 : les parties du compte RPCS3 du profil reviennent dans SON compte sur ce PC (recréé si besoin, son
            // numéro peut différer de l'ancien PC) ; jamais par-dessus une partie différente.
            let compte = zone.join(&nom).join(crate::emulateurs_profils::COMPTE_RPCS3);
            if compte.is_dir() && crate::emulateurs_profils::est_rpcs3(&programme) {
                let numero = crate::emulateurs_profils::compte_rpcs3(dossier, &crate::jeux_pc::nom_de_dossier(&s.profil.nom))?;
                let c = reposer_parties_entieres(&compte, &dossier.join("dev_hdd0").join("home").join(numero))?;
                r.reposes += c.reposes;
                r.deja_la += c.deja_la;
                // Dit simplement : « PS3 : partie BCES00569-GAME ».
                r.conflits.extend(c.conflits.into_iter().map(|f| format!("PS3 : {f}")));
            }
            total.reposes += r.reposes;
            total.deja_la += r.deja_la;
            total.conflits.extend(r.conflits.into_iter().map(|f| format!("{nom}/{f}")));
        }
        Ok(total)
    }

    /// Juste après l'installation d'un jeu : si ses parties attendent d'être restaurées, elles sont reposées dans son
    /// dossier (elles remplacent les fichiers du jeu tout neuf), puis retirées de la zone d'attente.
    /// `anciens` : les anciens numéros de ce jeu connus du profil (fiches réunies), en plus de ceux du registre.
    pub(crate) fn reposer_parties_du_jeu(&self, profil_id: &str, id: i64, anciens: &[i64], dossier: &Path) -> Resultat<Reposes> {
        // Les parties attendues sous ce numéro, et sous les ANCIENS numéros de ce jeu (fiches réunies chez Firehouse,
        // contrat 1.14 : une sauvegarde faite avant le rattachement).
        // Les anciens d'abord, le nouveau en dernier : la partie la plus récente (sous le nouveau numéro) l'emporte.
        let mut numeros: Vec<i64> = self.registre().reunions()?.into_iter().filter(|(_, n)| *n == id).map(|(a, _)| a).collect();
        numeros.extend(anciens.iter().copied().filter(|a| *a != id));
        numeros.sort();
        numeros.dedup();
        numeros.push(id);
        let mut r = Reposes::default();
        for n in numeros {
            let attente = self.zone_restauration(profil_id).join("parties").join("jeux").join(n.to_string());
            if !attente.is_dir() {
                continue;
            }
            let x = reposer(&attente, dossier, true)?;
            std::fs::remove_dir_all(&attente)?;
            r.reposes += x.reposes;
            r.deja_la += x.deja_la;
            r.conflits.extend(x.conflits);
        }
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use serde_json::json;

    #[test]
    fn les_parties_d_un_emulateur_reinstalle_ailleurs_le_retrouvent_par_son_programme() {
        let d = tempfile::tempdir().unwrap();
        let zone = d.path();
        for n in ["rpcs3-v0.0.38-win64", "Cemu_2.5", "RetroArch", "RetroArch-ancien"] {
            std::fs::create_dir_all(zone.join(n)).unwrap();
        }
        // Les réglages de l'ancien PC (toutes formes : liste par système, programme seul).
        let conf = json!({"reglages_pc": {"reglages": {"emulateurs": {
            "Sony Playstation 3": {"defaut": "1", "liste": [{"cle": "1", "programme": "D:\\Emus\\rpcs3-v0.0.38-win64\\rpcs3.exe"}]},
            "Nintendo Wii U": {"programme": "E:/Emus/Cemu_2.5/Cemu.exe"},
            "SNES": {"liste": [{"programme": "D:\\RetroArch\\retroarch.exe"}, {"programme": "D:\\RetroArch-ancien\\retroarch.exe"}]}
        }}}});
        let a = anciens_programmes(&conf);
        assert_eq!(a.len(), 4, "{a:?}");
        // Même nom de dossier : tel quel.
        assert_eq!(cle_en_attente(zone, "C:\\Frogtend\\Emulateurs\\RetroArch\\retroarch.exe", &a).as_deref(), Some("RetroArch"));
        // Réinstallé par Frogtend dans « RPCS3 » : retrouvé par rpcs3.exe ; Cemu dans un autre sous-dossier numéroté.
        assert_eq!(cle_en_attente(zone, "C:\\Frogtend\\Emulateurs\\RPCS3\\rpcs3.exe", &a).as_deref(), Some("rpcs3-v0.0.38-win64"));
        assert_eq!(cle_en_attente(zone, "C:\\Emus\\Cemu\\Cemu_2.6\\cemu.exe", &a).as_deref(), Some("Cemu_2.5"));
        // Deux anciens RetroArch et un nouveau nom : jamais deviné.
        assert_eq!(cle_en_attente(zone, "C:\\Emus\\RA\\retroarch.exe", &a), None);
        assert_eq!(cle_en_attente(zone, "C:\\Emus\\Dolphin\\Dolphin.exe", &a), None);

        // Plusieurs émulateurs présents : un dossier d'attente à UN seul. Deux RPCS3 pour un seul ancien : personne ;
        // RetroArch au même nom + un autre retroarch.exe : le nom exact seulement.
        let p = |s: &str| s.to_string();
        let c = cles_en_attente(
            zone,
            &[p("C:\\E\\RPCS3\\rpcs3.exe"), p("D:\\rpcs3-test\\rpcs3.exe"), p("C:\\E\\RetroArch\\retroarch.exe"), p("C:\\E\\Cemu\\Cemu.exe")],
            &a,
        );
        assert_eq!(c, vec![(p("C:\\E\\RetroArch\\retroarch.exe"), p("RetroArch")), (p("C:\\E\\Cemu\\Cemu.exe"), p("Cemu_2.5"))]);
    }

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
    async fn les_parties_switch_reviennent_dans_l_utilisateur_eden_du_profil() {
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, None).unwrap().id;
        n.ouvrir(&id, None, &crate::noyau::Connexion { adresse: String::new(), simule: true }).await.unwrap();
        let zone = d.path().join("app").join("restauration").join(&id).join("parties").join("emulateurs").join("Eden");
        let r = ".eden-utilisateur/save/0100000000010000/save.bin";
        std::fs::create_dir_all(zone.join(r).parent().unwrap()).unwrap();
        std::fs::write(zone.join(r), b"zelda").unwrap();
        let eden = d.path().join("Emulateurs").join("Eden");
        let programme: String = eden.join("eden.exe").to_string_lossy().into();
        // Eden pas portable : rien n'est écrit chez lui, les parties attendent.
        std::fs::create_dir_all(&eden).unwrap();
        assert_eq!(n.reposer_parties_emulateurs(std::slice::from_ref(&programme)).await.unwrap().reposes, 0);
        assert!(!crate::eden::fichier_profils(&eden).exists());
        // Portable : l'utilisateur de Seb est créé, puis ses parties reposées chez lui (jamais dans `Profils`).
        std::fs::create_dir_all(eden.join("user")).unwrap();
        let rep = n.reposer_parties_emulateurs(std::slice::from_ref(&programme)).await.unwrap();
        assert_eq!((rep.reposes, rep.conflits.len()), (1, 0));
        let seb = crate::eden::utilisateur_existant(&eden, "Seb").unwrap();
        let save = crate::eden::dossiers_parties(&eden, &seb)[0].1.join("0100000000010000").join("save.bin");
        assert_eq!(std::fs::read(&save).unwrap(), b"zelda");
        assert!(!crate::emulateurs_profils::dossier_du_profil(&eden, "Seb").join(".eden-utilisateur").exists());
        // Il a rejoué depuis : jamais écrasé, conflit dit.
        std::fs::write(&save, b"zelda plus loin").unwrap();
        let rep = n.reposer_parties_emulateurs(&[programme]).await.unwrap();
        assert_eq!(rep.conflits, ["Eden/Switch/0100000000010000"]);
        assert_eq!(std::fs::read(&save).unwrap(), b"zelda plus loin");
    }

    #[tokio::test]
    async fn les_parties_ps3_reviennent_dans_le_compte_rpcs3_du_profil_meme_sous_un_autre_numero() {
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, None).unwrap().id;
        n.ouvrir(&id, None, &crate::noyau::Connexion { adresse: String::new(), simule: true }).await.unwrap();
        // En attente (sauvegarde restaurée) : une partie PS3 du compte de Seb, et un fichier du dossier du profil.
        let zone = d.path().join("app").join("restauration").join(&id).join("parties").join("emulateurs").join("RPCS3");
        for (r, o) in [
            (".rpcs3-compte/savedata/BCES00569-GAME/SAVE.DAT", &b"partie"[..]),
            (".rpcs3-compte/savedata/BCES00569-GAME/ICON0.PNG", b"icone"),
            ("frogtend.txt", b"profil"),
        ] {
            std::fs::create_dir_all(zone.join(r).parent().unwrap()).unwrap();
            std::fs::write(zone.join(r), o).unwrap();
        }
        // Le nouveau PC : RPCS3 installé, le compte 00000002 déjà pris par Léa.
        let rpcs3 = d.path().join("Emulateurs").join("RPCS3");
        let lea = rpcs3.join("dev_hdd0").join("home").join("00000002");
        std::fs::create_dir_all(&lea).unwrap();
        std::fs::write(lea.join("localusername"), "Lea").unwrap();
        let r = n.reposer_parties_emulateurs(&[rpcs3.join("rpcs3.exe").to_string_lossy().into()]).await.unwrap();
        assert_eq!((r.reposes, r.conflits.len()), (3, 0));
        let seb = rpcs3.join("dev_hdd0").join("home").join("00000003");
        assert_eq!(std::fs::read_to_string(seb.join("localusername")).unwrap(), "Seb");
        assert_eq!(std::fs::read(seb.join("savedata/BCES00569-GAME/SAVE.DAT")).unwrap(), b"partie");
        let profil = crate::emulateurs_profils::dossier_du_profil(&rpcs3, "Seb");
        assert!(profil.join("frogtend.txt").is_file());
        assert!(!profil.join(".rpcs3-compte").exists(), "jamais rangées dans le dossier du profil");
        assert!(!lea.join("savedata").exists(), "jamais dans le compte de quelqu'un d'autre");
        // Reposée une seconde fois, à l'identique : rien à faire.
        let r = n.reposer_parties_emulateurs(&[rpcs3.join("rpcs3.exe").to_string_lossy().into()]).await.unwrap();
        assert_eq!((r.reposes, r.conflits.len()), (0, 0));
        // Une partie différente déjà là (elle a rejoué : SAVE.DAT changé, l'icône partie) : on n'y touche PAS DU TOUT —
        // jamais un mélange des deux parties — et c'est dit simplement.
        std::fs::write(seb.join("savedata/BCES00569-GAME/SAVE.DAT"), b"plus recente").unwrap();
        std::fs::remove_file(seb.join("savedata/BCES00569-GAME/ICON0.PNG")).unwrap();
        let r = n.reposer_parties_emulateurs(&[rpcs3.join("rpcs3.exe").to_string_lossy().into()]).await.unwrap();
        assert_eq!(r.conflits, ["RPCS3/PS3 : savedata/BCES00569-GAME"]);
        assert_eq!(std::fs::read(seb.join("savedata/BCES00569-GAME/SAVE.DAT")).unwrap(), b"plus recente");
        assert!(!seb.join("savedata/BCES00569-GAME/ICON0.PNG").exists(), "rien de l'ancienne partie mêlé à la nouvelle");
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
        let rj = n.reposer_parties_du_jeu(&id, 110, &[], &jeu).unwrap();
        assert_eq!(rj.reposes, 1);
        assert_eq!(std::fs::read(jeu.join("C/DUNECD/DUNE37S0.SAV")).unwrap(), b"partie");
        assert_eq!(n.reposer_parties_du_jeu(&id, 110, &[], &jeu).unwrap().reposes, 0, "reposées une seule fois");
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

    /// Relit (LECTURE SEULE) les sauvegardes du compte sur le vrai Firehouse : liste, puis les documents d'un couple.
    /// `FROGTEND_PROFIL_ESSAI=<id> FROGTEND_COUPLE=Profil/PC cargo test essai_lire_sauvegarde -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_lire_sauvegarde() {
        use crate::coffre::Coffre as _;
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton pour ce profil");
        let c = Client::nouveau("https://jeux.hikari-no-sekai.fr", &jeton).unwrap();
        for s in lister(&c).await.unwrap() {
            println!("sauvegarde : {} / {} — {} octets — {}", s.profil, s.pc, s.octets, s.derniere);
        }
        let couple = std::env::var("FROGTEND_COUPLE").expect("FROGTEND_COUPLE");
        let (p, m) = couple.split_once('/').unwrap();
        let base = format!("/sauvegarde/{}/{}", crate::source::encoder(p), crate::source::encoder(m));
        let conf: Value = c.obtenir_json(&format!("{base}/document/configuration.json")).await.unwrap();
        let cles: Vec<&String> = conf.as_object().map(|o| o.keys().collect()).unwrap_or_default();
        println!("configuration.json : {cles:?}, profil {}", conf["profil"]);
        let biblio: Value = c.obtenir_json(&format!("{base}/document/bibliotheque.json")).await.unwrap();
        for j in biblio["jeux"].as_array().cloned().unwrap_or_default() {
            println!("jeu : {} — {} ({})", j["id"], j["titre"], j["etat"]);
        }
        let man: ManifesteParties = serde_json::from_value(c.obtenir_json(&format!("{base}/document/manifeste.json")).await.unwrap()).unwrap();
        println!("manifeste : {} fichier(s)", man.fichiers.len());
        for (r, (sha, t)) in &man.fichiers {
            println!("  {r} — {t} octets — {sha}");
        }
    }

    /// Essai sur le VRAI Firehouse (contrat 1.5), jamais lancé par la suite de tests (`#[ignore]`), avec l'accord de
    /// Seb. Il écrit UNIQUEMENT dans un dossier d'essai (« Essai-Frogtend / Venkman-essai »), avec des fichiers
    /// fabriqués ici : aucune vraie partie, aucun vrai profil. Il retire ensuite tout ce qu'il a créé (fichiers, puis le couple entier).
    /// `FROGTEND_PROFIL_ESSAI=<id du profil> cargo test essai_sauvegarde_reelle -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn essai_sauvegarde_reelle() {
        use crate::coffre::Coffre as _;
        use crate::sauvegarde::{envoyer, Api, Contenu, Destination as _, Element, MORCEAU};
        let profil = std::env::var("FROGTEND_PROFIL_ESSAI").expect("FROGTEND_PROFIL_ESSAI");
        let jeton = crate::coffre::CoffreWindows.lire(&profil).unwrap().expect("pas de jeton pour ce profil");
        let adresse = std::env::var("FROGTEND_ADRESSE_ESSAI").unwrap_or_else(|_| "https://jeux.hikari-no-sekai.fr".into());
        let c = Client::nouveau(&adresse, &jeton).unwrap();
        let (p, m) = ("Essai-Frogtend", "Venkman-essai");

        // Des fichiers fabriqués : un nom à parenthèses, un chemin accentué, un gros fichier (3 morceaux).
        let d = tempfile::tempdir().unwrap();
        let fichiers: Vec<(&str, Vec<u8>)> = vec![
            ("jeux/110/concat (1).sav", b"partie de Dune".to_vec()),
            ("emulateurs/RetroArch/saves/Zoé et l'été.srm", vec![7u8; 8192]),
            ("emulateurs/PCSX2/memcards/gros.ps2", (0..20 * 1024 * 1024u32).map(|i| (i.wrapping_mul(2654435761) >> 13) as u8).collect()),
        ];
        let mut parties = Vec::new();
        for (r, o) in &fichiers {
            let f = d.path().join("source").join(r.replace('/', "\\"));
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(&f, o).unwrap();
            parties.push(Element { source: f, relatif: r.to_string() });
        }
        let contenu = Contenu { configuration: json!({"essai": true}), bibliotheque: json!({"jeux": []}), parties, gardes: vec![] };
        let api = Api { client: &c, profil: p.into(), pc: m.into(), morceau: MORCEAU };

        // 1. Envoi complet.
        let b = envoyer(&api, &contenu, m, "Firehouse").await.expect("1er envoi");
        println!("1er envoi : {} envoyé(s), {} octets", b.envoyes, b.octets);
        assert_eq!((b.envoyes, b.fichiers), (3, 3));
        // 2. Rien n'a changé : rien n'est renvoyé.
        let b = envoyer(&api, &contenu, m, "Firehouse").await.expect("2e envoi");
        println!("2e envoi : {} envoyé(s), {} inchangé(s)", b.envoyes, b.inchanges);
        assert_eq!((b.envoyes, b.inchanges), (0, 3));

        // 3. Reprise après coupure : un premier morceau envoyé « à la main », puis l'envoi reprend là.
        let reprise = d.path().join("reprise.bin");
        let octets: Vec<u8> = (0..10 * 1024 * 1024u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&reprise, &octets).unwrap();
        let sha = empreinte(&reprise).unwrap();
        let route = format!("/sauvegarde/{p}/{m}/fichier?chemin={}", crate::source::encoder("essai/reprise.bin"));
        let premier = c
            .brute(
                reqwest::Method::PUT,
                &route,
                Some(octets[..MORCEAU].to_vec()),
                &[("Content-Range", format!("bytes 0-{}/{}", MORCEAU - 1, octets.len())), ("X-Contenu-Sha256", sha.clone())],
            )
            .await
            .unwrap();
        println!("1er morceau à la main : code {}", premier.statut);
        let tete = c.brute(reqwest::Method::HEAD, &route, None, &[]).await.unwrap();
        println!("HEAD : code {}, X-Recu {:?}", tete.statut, tete.recu);
        assert_eq!(tete.recu, Some(MORCEAU as u64), "Firehouse dit ce qu'il a déjà reçu");
        let n = api.envoyer_fichier("essai/reprise.bin", &reprise, &sha, octets.len() as u64).await.expect("reprise");
        assert_eq!(n, (octets.len() - MORCEAU) as u64, "seule la fin est renvoyée");

        // 4. Tout relire, chaque fichier vérifié par son empreinte.
        let liste = lister(&c).await.unwrap();
        assert!(liste.iter().any(|s| s.profil == p && s.pc == m), "la sauvegarde d'essai est listée");
        let base = format!("/sauvegarde/{p}/{m}");
        let lu: Value = c.obtenir_json(&format!("{base}/document/configuration.json")).await.unwrap();
        assert_eq!(lu["essai"], true);
        let manifeste: ManifesteParties =
            serde_json::from_value(c.obtenir_json(&format!("{base}/document/manifeste.json")).await.unwrap()).unwrap();
        assert_eq!(manifeste.fichiers.len(), 3);
        for (r, o) in &fichiers {
            let (sha, taille) = manifeste.fichiers.get(*r).expect(r).clone();
            let cible = d.path().join("relu").join(format!("{}", sha));
            telecharger(&c, &format!("{base}/fichier?chemin={}", crate::source::encoder(r)), &cible, &sha, taille).await.expect(r);
            assert_eq!(&std::fs::read(&cible).unwrap(), o, "{r} relu identique");
            println!("relu et vérifié : {r} ({taille} octets)");
        }

        // 5. Ménage : on retire les fichiers de l'essai (et seulement eux).
        for r in fichiers.iter().map(|(r, _)| *r).chain(["essai/reprise.bin"]) {
            api.supprimer_fichier(r).await.expect(r);
        }
        let apres = c.brute(reqwest::Method::GET, &format!("{base}/fichier?chemin={}", crate::source::encoder(fichiers[0].0)), None, &[]).await.unwrap();
        println!("après le ménage : GET d'un fichier → code {}", apres.statut);
        assert_eq!(apres.statut, 404);
        // Puis tout le couple d'essai, documents compris (DELETE /sauvegarde/{profil}/{pc}, Firehouse 30/09).
        // Garde-fou : jamais un autre couple que celui de l'essai.
        assert_eq!((p, m), ("Essai-Frogtend", "Venkman-essai"));
        let r = c.brute(reqwest::Method::DELETE, &base, None, &[]).await.unwrap();
        println!("DELETE {base} → code {} {}", r.statut, String::from_utf8_lossy(&r.octets));
        assert!((200..300).contains(&r.statut));
        let liste = lister(&c).await.unwrap();
        assert!(!liste.iter().any(|s| s.profil == p && s.pc == m), "plus rien de l'essai chez Firehouse");
        println!("vérifié : l'essai n'apparaît plus dans GET /sauvegarde");
    }
}
