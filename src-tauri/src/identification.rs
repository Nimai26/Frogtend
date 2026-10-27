//! Identifier les jeux importés du disque qui n'ont pas de fiche (contrat 1.32, besoin 16) : `POST /identifier` dit,
//! pour chacun, `exact` (un seul jeu de la base LaunchBox, sûr), `probable` (des candidats à montrer à la personne)
//! ou `inconnu`. Règles :
//! - seul `exact` se rattache seul ; `probable` est TOUJOURS proposé (jamais le premier candidat d'office) ;
//! - un jeu qui a une fiche VISIBLE chez Firehouse (`fiche_id`) se rattache à elle, comme par `/reconcilier` ;
//! - tout ce qui est reçu reste dans le cache DU PROFIL (jamais montré à un autre profil) ;
//! - rien du disque de la personne ne part au-delà du NOM du fichier et de celui de son seul dossier parent.

use crate::erreurs::Resultat;
use crate::jeux_pc::JeuPc;
use crate::noyau::{maintenant, Noyau};
use serde::Serialize;
use serde_json::{json, Value};
use std::path::Path;

/// Les fiches de base déjà relues chez Firehouse pendant cette session : (dossier du profil, launchbox_id).
static RELUES: std::sync::LazyLock<std::sync::Mutex<std::collections::HashSet<(std::path::PathBuf, i64)>>> = std::sync::LazyLock::new(Default::default);

/// Au plus, par lot (contrat : 413 au-delà).
pub const LOT: usize = 500;

/// Ce qu'une identification a donné.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct Bilan {
    pub envoyes: usize,
    /// Rattachés seuls (à une fiche, ou au jeu de la base).
    pub exacts: usize,
    /// À montrer à la personne.
    pub probables: usize,
    pub inconnus: usize,
    /// Arrêtée en cours de route (jeton refusé, réseau…) : pourquoi ; ce qui précède est noté.
    pub motif: Option<String>,
}

/// Un jeu à choisir par la personne.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AChoisir {
    pub local: i64,
    pub titre: String,
    pub plateforme: String,
    /// `[{launchbox_id, titre, plateforme, annee, fiche_id}]`, tels que Firehouse les a proposés.
    pub candidats: Value,
}

/// Le nom d'un fichier SEUL (jamais un chemin).
fn nom(p: &Path) -> Option<String> {
    p.file_name().map(|n| n.to_string_lossy().to_string()).filter(|n| crate::reconciliation::nom_envoyable(n))
}

/// Le fichier du jeu sur le disque (le fichier lancé, sinon le programme choisi).
fn fichier_du_jeu(j: &JeuPc) -> Option<std::path::PathBuf> {
    let i = j.installation.as_ref()?;
    match (&i.fichier_du_jeu, &i.lanceur) {
        (Some(f), _) => Some(Path::new(&i.dossier).join(f)),
        (None, Some(l)) => Some(Path::new(&l.programme).to_path_buf()),
        _ => None,
    }
}

/// La demande pour un jeu importé : sa plateforme (nom LaunchBox), le NOM de son fichier et de son seul dossier
/// parent, et ce qu'on sait de sûr (numéro Steam, numéro et titre PS3). `None` : rien d'envoyable.
pub fn demande(cle: &str, j: &JeuPc) -> Option<Value> {
    let f = fichier_du_jeu(j)?;
    let mut fichier = nom(&f)?;
    let mut dossier = f.parent().and_then(nom);
    // Un `EBOOT.BIN` (jeu PS3 en dossier) ne dit rien du jeu : le dossier du jeu le remplace.
    if fichier.eq_ignore_ascii_case("EBOOT.BIN") {
        let jeu = f.ancestors().find(|a| a.join("PS3_GAME").is_dir()).map(Path::to_path_buf).unwrap_or_else(|| Path::new(&j.dossier).to_path_buf());
        fichier = nom(&jeu)?;
        dossier = jeu.parent().and_then(nom);
    }
    let mut d = json!({ "cle": cle, "plateforme": j.plateforme, "fichier": fichier });
    if let Some(x) = dossier {
        d["dossier"] = json!(x);
    }
    if let Some(b) = j.installation.as_ref().and_then(|i| i.boutique.as_ref()).filter(|b| b.source == "steam") {
        if let Ok(n) = b.cle.parse::<u64>() {
            d["steam_appid"] = json!(n);
        }
    }
    if j.plateforme == "Sony Playstation 3" {
        if let Some((numero, titre)) = crate::contenus::numero_et_titre_ps3(&f) {
            d["ps3_title_id"] = json!(numero);
            if let Some(t) = titre {
                d["titre_sfo"] = json!(t);
            }
        }
    }
    Some(d)
}

/// Les chemins des versions d'un jeu importé (pour un rattachement à une fiche).
fn versions(j: &JeuPc) -> Vec<String> {
    let mut l: Vec<String> = j.installation.as_ref().map(|i| i.versions.iter().map(|v| v.chemin.clone()).collect()).unwrap_or_default();
    if l.is_empty() {
        l.extend(fichier_du_jeu(j).map(|f| f.to_string_lossy().to_string()));
    }
    l
}

impl Noyau {
    /// Un jeu importé déjà relié à une fiche (par `/reconcilier` ou ici), quel que soit le profil qui l'a relié : le
    /// registre est commun au PC, son lien n'est jamais écrasé par une identification sur le titre.
    fn deja_relie(&self, local: i64) -> Resultat<bool> {
        Ok(self.registre().correspondances()?.iter().any(|(l, ..)| *l == local))
    }

    /// Identifie ces jeux importés (ceux du profil, sans fiche, jamais identifiés : l'appelant les choisit), par lots de
    /// 500, et note la réponse dans le cache du profil (jamais pour une session remplacée). Un `exact` avec une fiche
    /// visible est rattaché à elle. Une erreur en cours de route (401, 413, réseau) s'arrête là : le bilan dit ce qui a
    /// été fait, et pourquoi la suite ne l'est pas. En mode simulé, rien n'est noté (tout serait « inconnu »).
    pub async fn identifier(&self, jeux: &[JeuPc]) -> Resultat<Bilan> {
        let s = self.session().await?;
        let mut b = Bilan::default();
        if s.source.est_simulee() {
            return Ok(b);
        }
        let demandes: Vec<(i64, Value)> = jeux.iter().filter_map(|j| demande(&j.id.to_string(), j).map(|d| (j.id, d))).collect();
        for lot in demandes.chunks(LOT) {
            let corps = json!({ "jeux": lot.iter().map(|(_, d)| d.clone()).collect::<Vec<_>>() });
            let r = match s.source.identifier(&corps).await {
                Ok(r) => r,
                Err(e @ crate::erreurs::Erreur::Perimee(_)) => return Err(e),
                Err(e) if b.envoyes > 0 => {
                    b.motif = Some(e.to_string());
                    break;
                }
                Err(e) => return Err(e),
            };
            let quand = maintenant();
            for x in r["resultats"].as_array().cloned().unwrap_or_default() {
                let Some(local) = x["cle"].as_str().and_then(|c| c.parse::<i64>().ok()).filter(|l| lot.iter().any(|(id, _)| id == l)) else { continue };
                b.envoyes += 1;
                let methode = x["methode"].as_str();
                match (x["statut"].as_str(), x["launchbox_id"].as_i64()) {
                    (Some("exact"), Some(lb)) => {
                        b.exacts += 1;
                        if let (Some(f), Some(j)) = (x["fiche_id"].as_i64(), jeux.iter().find(|j| j.id == local)) {
                            if !self.deja_relie(local)? {
                                let v: Vec<_> = versions(j).into_iter().map(|c| (c, f, None, "exact".to_string(), None)).collect();
                                drop(s.verrou_vivant()?); // contrôle seulement : session encore la bonne
                                self.registre().noter_correspondance(local, &v, "auto", &quand)?;
                            }
                        }
                        let c = s.verrou_vivant()?;
                        c.noter_identification(local, "exact", Some(lb), methode, &json!([]), Some("auto"), &quand)?;
                        c.marquer_jaquette(local, true)?;
                    }
                    // Probable : jamais retenu seul (le `launchbox_id` du résultat n'est pas lu : null, contrat 1.32).
                    (Some("probable"), _) if x["candidats"].as_array().is_some_and(|c| !c.is_empty()) => {
                        b.probables += 1;
                        let c = s.verrou_vivant()?;
                        c.noter_identification(local, "probable", None, methode, &x["candidats"], None, &quand)?;
                        c.marquer_jaquette(local, false)?;
                    }
                    _ => {
                        b.inconnus += 1;
                        let c = s.verrou_vivant()?;
                        c.noter_identification(local, "inconnu", None, methode, &json!([]), None, &quand)?;
                        c.marquer_jaquette(local, false)?;
                    }
                }
            }
        }
        self.journaliser(&format!(
            "identification de {} jeu(x) importé(s) : {} exact(s), {} à choisir, {} inconnu(s){}",
            b.envoyes,
            b.exacts,
            b.probables,
            b.inconnus,
            b.motif.as_deref().map(|m| format!(" ; arrêtée : {m}")).unwrap_or_default()
        ));
        Ok(b)
    }

    /// La personne choisit le jeu de la base pour un jeu importé : seulement un des candidats proposés. Un candidat qui
    /// a une fiche visible (`fiche_id`) rattache le jeu à elle — jamais par-dessus un lien déjà fait.
    pub async fn choisir_identification(&self, j: &JeuPc, launchbox_id: i64) -> Resultat<()> {
        let s = self.session().await?;
        let (_, _, methode, candidats, _) = s
            .verrou()
            .identification(j.id)?
            .ok_or_else(|| crate::erreurs::Erreur::Introuvable("Ce jeu n'a pas été identifié.".into()))?;
        let c = candidats
            .as_array()
            .and_then(|l| l.iter().find(|c| c["launchbox_id"].as_i64() == Some(launchbox_id)).cloned())
            .ok_or_else(|| crate::erreurs::Erreur::Refus("Firehouse n'a pas proposé ce jeu pour celui-ci.".into()))?;
        if self.deja_relie(j.id)? {
            return Err(crate::erreurs::Erreur::Refus("Ce jeu est déjà relié à une fiche : défais d'abord ce lien.".into()));
        }
        let quand = maintenant();
        if let Some(f) = c["fiche_id"].as_i64() {
            let v: Vec<_> = versions(j).into_iter().map(|ch| (ch, f, None, "probable".to_string(), None)).collect();
            drop(s.verrou_vivant()?); // contrôle seulement : session encore la bonne
            self.registre().noter_correspondance(j.id, &v, "personne", &quand)?;
        }
        let k = s.verrou_vivant()?;
        k.noter_identification(j.id, "choisi", Some(launchbox_id), methode.as_deref(), &json!([c]), Some("personne"), &quand)?;
        k.marquer_jaquette(j.id, true)?;
        drop(k);
        self.journaliser(&format!("jeu importé {} identifié par la personne (base {launchbox_id})", j.id));
        Ok(())
    }

    /// « Aucun de ceux-là » : plus jamais proposé (jusqu'à une nouvelle identification demandée par la personne).
    pub async fn ecarter_identification(&self, local: i64) -> Resultat<()> {
        let s = self.session().await?;
        let c = s.verrou_vivant()?;
        c.noter_identification(local, "ecarte", None, None, &json!([]), Some("personne"), &maintenant())?;
        c.marquer_jaquette(local, false)?;
        Ok(())
    }

    /// Les jeux à faire choisir à la personne (statut `probable`), parmi `jeux` — jamais un jeu déjà relié depuis.
    pub async fn a_choisir(&self, jeux: &[JeuPc]) -> Resultat<Vec<AChoisir>> {
        let s = self.session().await?;
        let relies: Vec<i64> = self.registre().correspondances()?.into_iter().map(|(l, ..)| l).collect();
        let c = s.verrou();
        let mut l = Vec::new();
        for j in jeux.iter().filter(|j| !relies.contains(&j.id)) {
            if let Some((statut, _, _, candidats, _)) = c.identification(j.id)? {
                if statut == "probable" {
                    l.push(AChoisir { local: j.id, titre: j.titre.clone(), plateforme: j.plateforme.clone(), candidats });
                }
            }
        }
        Ok(l)
    }

    /// Le jeu de la base LaunchBox retenu pour un jeu importé (exact ou choisi), s'il y en a un.
    pub async fn base_du_jeu(&self, local: i64) -> Resultat<Option<i64>> {
        let s = self.session().await?;
        let id = s.verrou().identification(local)?.filter(|(st, ..)| st == "exact" || st == "choisi").and_then(|(_, lb, ..)| lb);
        Ok(id)
    }

    /// La jaquette d'un jeu importé identifié (sa fiche de base, contrat 1.32), gardée dans le profil comme les autres
    /// (`<profil>\base\<lb>-<largeur>-<empreinte>.img`) ; hors ligne, celle déjà gardée. Aucune : rien (la carte de
    /// remplacement reste).
    pub async fn jaquette_base(&self, local: i64, largeur: Option<u32>) -> Resultat<Option<crate::source::Image>> {
        let Some(lb) = self.base_du_jeu(local).await? else { return Ok(None) };
        let gardee = self.session().await?.verrou().fiche_base(lb)?;
        let fiche = match gardee {
            Some(f) => Some(f),
            None => self.fiche_base(local).await?,
        };
        let Some(fiche) = fiche else { return Ok(None) };
        let Some(empreinte) = empreinte_jaquette(&fiche) else { return Ok(None) };
        let l = largeur.unwrap_or(crate::locale::LARGEUR_GRANDE).clamp(100, 1000).div_ceil(100) * 100;
        let s = self.session().await?;
        let fichier = s.dossier.join("base").join(format!("{lb}-{l}-{empreinte}.img"));
        if let Ok(octets) = std::fs::read(&fichier) {
            crate::noyau::vue(&fichier);
            return Ok(Some(crate::source::Image { type_contenu: crate::noyau::type_image(&octets), octets }));
        }
        match s.source.media_base(lb, "jaquette", l).await {
            Ok(Some(img)) => {
                drop(s.verrou_vivant()?); // contrôle seulement : session encore la bonne
                std::fs::create_dir_all(s.dossier.join("base"))?;
                s.garder_fichier(&fichier, &img.octets)?;
                Ok(Some(img))
            }
            Ok(None) | Err(crate::erreurs::Erreur::Reseau(_) | crate::erreurs::Erreur::Serveur(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// La fiche de base d'un jeu importé identifié (contrat 1.32) : gardée dans le profil (hors ligne), relue chez
    /// Firehouse au plus une fois par session (et jamais plus de 3 s d'attente quand une fiche est gardée) ; 404 →
    /// oubliée (identification comprise). Jamais rendue ni écrite pour une session remplacée.
    pub async fn fiche_base(&self, local: i64) -> Resultat<Option<Value>> {
        let Some(lb) = self.base_du_jeu(local).await? else { return Ok(None) };
        let s = self.session().await?;
        let gardee = s.verrou().fiche_base(lb)?;
        let cle = (s.dossier.clone(), lb);
        if gardee.is_some() && RELUES.lock().unwrap_or_else(|e| e.into_inner()).contains(&cle) {
            return Ok(gardee);
        }
        let reponse = match &gardee {
            Some(_) => tokio::time::timeout(std::time::Duration::from_secs(3), s.source.base(lb))
                .await
                .unwrap_or_else(|_| Err(crate::erreurs::Erreur::Reseau("Firehouse ne répond pas assez vite.".into()))),
            None => s.source.base(lb).await,
        };
        match reponse {
            Ok(Some(f)) => {
                s.verrou_vivant()?.garder_fiche_base(lb, &f, &maintenant())?;
                RELUES.lock().unwrap_or_else(|e| e.into_inner()).insert(cle);
                Ok(Some(f))
            }
            Ok(None) => {
                s.verrou_vivant()?.oublier_fiche_base(lb)?;
                Ok(None)
            }
            // Hors ligne : celle qu'on a gardée.
            Err(crate::erreurs::Erreur::Reseau(_) | crate::erreurs::Erreur::Serveur(_)) => {
                drop(s.verrou_vivant()?); // contrôle seulement : session encore la bonne
                Ok(gardee)
            }
            Err(e) => Err(e),
        }
    }
}

/// L'empreinte de la jaquette principale d'une fiche de base (sinon la première jaquette), pour nommer son fichier.
fn empreinte_jaquette(fiche: &Value) -> Option<String> {
    let images = fiche["images"].as_array()?;
    let i = images.iter().find(|i| i["type"] == "jaquette" && i["principal"] == true).or_else(|| images.iter().find(|i| i["type"] == "jaquette"))?;
    Some(i["empreinte"].as_str().unwrap_or("x").chars().filter(char::is_ascii_alphanumeric).take(32).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jeux_pc::{Etat, Installation, LancementBoutique};

    fn importe(id: i64, plateforme: &str, dossier: &str, fichier: &str) -> JeuPc {
        JeuPc {
            id,
            version: 0,
            titre: "Jeu".into(),
            plateforme: plateforme.into(),
            dossier: dossier.into(),
            etat: Etat::Telecharge,
            total: 0,
            fichiers: vec![],
            message: None,
            ajoute_le: "1".into(),
            ajoute_par: "p".into(),
            installation: Some(Installation {
                dossier: dossier.into(),
                methode: crate::installation::Methode::Aucune,
                lanceur: None,
                fichier_du_jeu: Some(fichier.into()),
                installe_le: "1".into(),
                versions: vec![],
                boutique: None,
                extraits: vec![],
            }),
            temps_jeu: 0,
            derniere_partie: None,
        }
    }

    #[test]
    fn la_demande_ne_dit_que_des_noms_jamais_un_chemin() {
        let j = importe(-5, "Nintendo Entertainment System", r"E:\Jeux\NES\Collection", "Abadox - The Deadly Inner War (USA).zip");
        let d = demande("-5", &j).unwrap();
        assert_eq!(d, json!({"cle": "-5", "plateforme": "Nintendo Entertainment System", "fichier": "Abadox - The Deadly Inner War (USA).zip", "dossier": "Collection"}));
        assert!(!d.to_string().contains(r"E:\\"), "jamais un chemin");
        // Un jeu Steam : son numéro (« exact » chez Firehouse).
        let mut s = importe(-6, "Windows", r"C:\Steam\steamapps\common\Hades", "Hades.exe");
        s.installation.as_mut().unwrap().boutique = Some(LancementBoutique { source: "steam".into(), cle: "1145360".into() });
        assert_eq!(demande("-6", &s).unwrap()["steam_appid"], json!(1145360));
        assert_eq!(demande("-6", &s).unwrap()["dossier"], json!("Hades"));
    }

    #[tokio::test]
    async fn exact_se_rattache_seul_probable_attend_la_personne() {
        use httpmock::prelude::*;
        let s = MockServer::start();
        // Les réponses du relevé RÉEL du 27/10 (forme exacte) : Hades exact par steam_appid, Abadox probable, inconnu.
        let ident = s.mock(|w, t| {
            w.method(POST).path("/api/jeux/v1/identifier");
            t.status(200).json_body(json!({"ok": true, "resultats": [
                {"cle": "-6", "statut": "exact", "methode": "steam_appid", "launchbox_id": 167721, "fiche_id": null, "candidats": [], "motif": null},
                {"cle": "-5", "statut": "probable", "methode": "titre", "launchbox_id": null, "fiche_id": null,
                 "candidats": [{"launchbox_id": 1501, "titre": "Abadox: The Deadly Inner War", "plateforme": "Nintendo Entertainment System", "annee": 1989, "fiche_id": 7742}], "motif": null},
                {"cle": "-8", "statut": "inconnu", "methode": null, "launchbox_id": null, "fiche_id": null, "candidats": [], "motif": "aucun jeu reconnu"}
            ]}));
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/base/167721");
            t.status(200).json_body(json!({"ok": true, "launchbox_id": 167721, "titre": "Hades", "resume": "…", "images": []}));
        });
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &crate::noyau::Connexion { adresse: s.base_url(), simule: false }).await.unwrap();
        let abadox = importe(-5, "Nintendo Entertainment System", r"E:\NES", "Abadox - The Deadly Inner War (USA).zip");
        let hades = importe(-6, "Windows", r"C:\Jeux\Hades", "Hades.exe");
        let rien = importe(-8, "Windows", r"C:\Jeux\X", "x.exe");
        for j in [&abadox, &hades, &rien] {
            n.registre().ajouter(j).unwrap();
        }
        let b = n.identifier(&[abadox.clone(), hades.clone(), rien.clone()]).await.unwrap();
        ident.assert();
        assert_eq!(b, Bilan { envoyes: 3, exacts: 1, probables: 1, inconnus: 1, motif: None });
        assert_eq!(n.base_du_jeu(-6).await.unwrap(), Some(167721), "exact : rattaché seul");
        assert_eq!(n.base_du_jeu(-5).await.unwrap(), None, "probable : jamais retenu seul");
        assert_eq!(n.fiche_base(-6).await.unwrap().unwrap()["titre"], "Hades");
        // La personne choisit : seulement un candidat proposé ; celui-ci a une fiche visible → rattaché à elle.
        let l = n.a_choisir(&[abadox.clone(), hades.clone()]).await.unwrap();
        assert_eq!(l.len(), 1);
        assert!(n.choisir_identification(&abadox, 999).await.is_err(), "jamais un jeu non proposé");
        n.choisir_identification(&abadox, 1501).await.unwrap();
        assert_eq!(n.base_du_jeu(-5).await.unwrap(), Some(1501));
        assert!(n.registre().correspondances().unwrap().iter().any(|(local, media, _)| *local == -5 && *media == 7742));
        assert!(n.a_choisir(&[abadox]).await.unwrap().is_empty());
        // Un autre profil ne voit rien de ce que Seb a reçu.
        let autre = n.creer_profil("Léa", None, Some("j")).unwrap().id;
        n.ouvrir(&autre, None, &crate::noyau::Connexion { adresse: s.base_url(), simule: false }).await.unwrap();
        assert_eq!(n.base_du_jeu(-6).await.unwrap(), None);
    }

    #[tokio::test]
    async fn la_jaquette_principale_vient_de_la_fiche_de_base_reelle() {
        use httpmock::prelude::*;
        let s = MockServer::start();
        s.mock(|w, t| {
            w.method(POST).path("/api/jeux/v1/identifier");
            t.status(200).json_body(json!({"ok": true, "resultats": [
                {"cle": "-5", "statut": "exact", "methode": "fichier_launchbox", "launchbox_id": 1501, "fiche_id": null, "candidats": [], "motif": null}]}));
        });
        // Le relevé RÉEL de `/base/1501` (27/10, lecture seule) : 32 images, la jaquette principale d'Amérique du Nord.
        let reel: Value = serde_json::from_str(include_str!("../fixtures/simule/base-1501.json")).unwrap();
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/base/1501");
            t.status(200).json_body(reel.clone());
        });
        let image = s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/base/1501/media/jaquette").query_param("largeur", "300");
            t.status(200).header("content-type", "image/webp").body(b"RIFFimage");
        });
        let d = tempfile::tempdir().unwrap();
        let n = Noyau::nouveau(&d.path().join("app"), Box::new(crate::coffre::CoffreMemoire::default())).unwrap();
        let id = n.creer_profil("Seb", None, Some("j")).unwrap().id;
        n.ouvrir(&id, None, &crate::noyau::Connexion { adresse: s.base_url(), simule: false }).await.unwrap();
        let abadox = importe(-5, "Nintendo Entertainment System", r"E:\NES", "Abadox (USA).zip");
        n.registre().ajouter(&abadox).unwrap();
        n.identifier(&[abadox]).await.unwrap();
        let img = n.jaquette_base(-5, Some(300)).await.unwrap().unwrap();
        assert_eq!(img.octets, b"RIFFimage");
        let base = n.session().await.unwrap().dossier.join("base");
        let fichiers: Vec<String> = std::fs::read_dir(&base).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
        assert_eq!(fichiers, ["1501-300-4ccf7eb96e2c0415ee557ee4a9ee5c83.img"], "la jaquette PRINCIPALE, par son empreinte");
        // La fois suivante : gardée, aucune requête de plus.
        n.jaquette_base(-5, Some(300)).await.unwrap().unwrap();
        image.assert_hits(1);
    }

    #[test]
    fn un_jeu_ps3_en_dossier_envoie_le_nom_du_jeu_pas_eboot() {
        let d = tempfile::tempdir().unwrap();
        let jeu = d.path().join("Gran Turismo 5");
        std::fs::create_dir_all(jeu.join("PS3_GAME").join("USRDIR")).unwrap();
        let j = importe(-7, "Sony Playstation 3", &jeu.join("PS3_GAME").join("USRDIR").to_string_lossy(), "EBOOT.BIN");
        let r = demande("-7", &j).unwrap();
        assert_eq!(r["fichier"], json!("Gran Turismo 5"));
        assert!(r.get("ps3_title_id").is_none(), "pas de PARAM.SFO lisible ici : rien d'inventé");
    }
}
