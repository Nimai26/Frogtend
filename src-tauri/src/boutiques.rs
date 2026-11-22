//! Les jeux que la personne possède dans ses boutiques (lot 9). Décision de Seb (02/10) : comme LaunchBox, chaque compte
//! se règle dans les Options (ou au premier import) ; les jeux importés restent dans le Frogtend de la personne et ne
//! remontent jamais dans Firehouse.
//!
//! **Steam** (API officielle, relevée le 02/10) :
//! - `ISteamUser/ResolveVanityURL/v1/?key=&vanityurl=` → `{response: {success: 1, steamid}}` (URL personnalisée) ;
//! - `IPlayerService/GetOwnedGames/v1/?key=&steamid=&include_appinfo=1&include_played_free_games=1` →
//!   `{response: {game_count, games: [{appid, name, playtime_forever, rtime_last_played, img_icon_url}]}}` ;
//! - jeux installés sur CE PC : `steamapps\libraryfolders.vdf` (bibliothèques) et `appmanifest_<appid>.acf` ;
//! - lancer / installer : `steam://rungameid/<appid>`, `steam://install/<appid>` (Steam fait le travail).
//!
//! La clé d'API est un SECRET : coffre de Windows (`<profil>#steam`), jamais écrite, jamais affichée, jamais
//! journalisée ; les erreurs réseau sont rendues SANS l'adresse (elle contient la clé).

use crate::erreurs::{Erreur, Resultat};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const API: &str = "https://api.steampowered.com";

/// Un jeu possédé dans une boutique.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct JeuBoutique {
    pub id: String,
    pub nom: String,
    /// Minutes jouées (selon la boutique).
    pub minutes: u64,
    /// Dernière partie (secondes depuis 1970), 0 = jamais.
    pub derniere: u64,
    /// Installé sur CE PC (vu au moment de l'import ou de la lecture).
    #[serde(default)]
    pub installe: bool,
    /// La boutique d'origine (« gog », « epic », « steam »…) pour les jeux venus de GOG Galaxy ; vide pour Steam.
    #[serde(default)]
    pub plateforme: String,
    /// La jaquette (adresse publique), quand la source la donne.
    #[serde(default)]
    pub image: Option<String>,
    /// Le dossier d'installation sur CE PC, quand la boutique le dit (Steam : manifeste ; GOG : base de Galaxy). Lu
    /// seulement : Frogtend n'écrit, ne retire et ne désinstalle JAMAIS rien dans ce dossier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dossier: Option<String>,
    /// Prêt à lancer tout de suite (Steam : aucune mise à jour en attente ni en cours) ; `None` : la boutique ne le dit
    /// pas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pret: Option<bool>,
    /// Succès (obtenus, total) connus de GOG Galaxy (`UserAchievements`) : GOG, Steam, Xbox, EA — pas Epic, que
    /// Galaxy ne synchronise pas (relevé le 02/10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub succes: Option<(u32, u32)>,
    /// Les copies du même jeu que GOG Galaxy relie (`GamePieces` « allGameReleases » : « steam_620 », « gog_123 »…), pour
    /// n'en montrer qu'une carte (A6c, Seb 21/10).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub releases: Vec<String>,
    /// Le titre est peut-être abîmé (catalogue Epic écrit en ASCII : un « ? » restant, ou un titre réparé) : une carte
    /// déjà rangée garde alors le sien (le titre juste du manifeste, lu quand le jeu était installé).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub titre_douteux: bool,
}

/// Ce que Frogtend garde d'une boutique pour un profil (`profils\<id>\boutiques.json`), sans aucun secret.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct CompteBoutique {
    /// Ce que la personne a donné : URL personnalisée ou identifiant numérique.
    pub compte: String,
    /// L'identifiant Steam (64 bits) résolu.
    pub steamid: String,
    pub maj_le: String,
    pub jeux: Vec<JeuBoutique>,
    /// Steam (lot C1) : quand la personne s'est connectée par la page officielle (vide = jamais, ou compte réglé à la
    /// main avec la clé d'API). Seul le `steamid` est gardé de cette connexion, jamais un jeton ni un cookie.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub connecte_le: String,
    /// Steam : le compte pour lequel la clé d'API a été vérifiée (`boutique_steam_regler`). Vide dans un fichier
    /// d'avant le lot C1 : c'est alors `steamid`, tant qu'aucune connexion par la page ne l'a changé.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub steamid_cle: String,
    /// Steam : le compte d'avant un « Oublier ma connexion » (sans clé ni connexion), pour que la connexion d'un AUTRE
    /// compte ensuite soit bien vue comme un changement de compte (Seb 20/11). Vidé à la connexion suivante.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub steamid_precedent: String,
    /// Steam : le ménage d'un changement de compte n'a pas pu se faire (profil changé entre-temps) ; il est fait à la
    /// prochaine ouverture du profil, avant tout import.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub menage_ancien_compte: bool,
    /// GOG (lot C4a, source « gog ») : le numéro du compte (`userId`, chiffres seulement) connecté dans le navigateur du
    /// profil, relu à chaque lecture de ses jeux. Rien d'autre du compte n'est gardé (ni nom, ni e-mail, ni jeton). Les
    /// champs `steamid*` ne servent pas à GOG ; `connecte_le` y dit la dernière lecture réussie.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub id_compte: String,
    /// GOG : le compte d'avant, quand une lecture a trouvé un AUTRE compte connecté (pour le changement de compte du
    /// lot C4b) ; aucune carte n'est retirée au lot C4a.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub id_compte_precedent: String,
    /// GOG (lot C5) : les fiches de la base publique de GOG déjà lues, par numéro de jeu (titre, type, jaquette ; un
    /// numéro introuvable aussi) : jamais redemandées. Rien du compte.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub fiches_gog: std::collections::BTreeMap<String, crate::gog_galaxy::FicheGog>,
    /// GOG (lot C4b, Seb 22/11 : « les deux comptes ») : TOUS les comptes GOG du profil, chacun avec SES jeux. Un seul
    /// compte `Profil` (celui de la session gog.com du navigateur du profil, qui reçoit aussi le jeu offert) ; les autres
    /// sont « ajoutés » (connexion façon Galaxy seule, jeton de renouvellement au coffre). Vide dans un fichier d'avant
    /// la 0.219 : `comptes_gog_lus` le reconstruit depuis `id_compte` et `jeux` (gardés une version de plus, pour un
    /// retour arrière possible : ils suivent le compte `Profil`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comptes_gog: Vec<CompteGog>,
    /// GOG (lot C4b) : le compte ouvert dans GOG Galaxy sur ce PC, relu à chaque lecture des comptes GOG (seulement s'il
    /// est sûr : un seul compte dans la base de Galaxy) ; vide : inconnu.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub compte_galaxy_pc: String,
    /// GOG (lot C4b) : le compte du navigateur du profil a été OUBLIÉ ; sa session gog.com n'est plus relue (ni à
    /// l'ouverture, ni à la main) tant que la personne ne s'est pas reconnectée elle-même (« 🔑 Se connecter à GOG »).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub navigateur_gog_oublie: bool,
}

/// D'où vient la connexion d'un compte GOG (lot C4b).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NavigateurGog {
    /// La session gog.com du navigateur du profil (jeu offert, liste du site, connexion façon Galaxy silencieuse).
    #[default]
    Profil,
    /// Un compte ajouté par « ➕ Ajouter un compte GOG » : seul son jeton de renouvellement (coffre) le relit.
    Ajoute,
}

/// Un compte GOG d'un profil (lot C4b), sans aucun secret.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct CompteGog {
    /// Le numéro du compte sur le site (`userId` de gog.com) ; vide pour un compte ajouté (le site n'est pas lu).
    #[serde(default)]
    pub id_site: String,
    /// Le numéro Galaxy du compte (`galaxyUserId`, celui de la connexion façon Galaxy) : son IDENTITÉ. Vide pour un
    /// compte repris d'un fichier d'avant la 0.219, rempli à sa lecture suivante.
    #[serde(default)]
    pub id_galaxy: String,
    /// Le pseudo GOG (public, `users.gog.com`) ; peut être vide (l'interface dit alors « Compte GOG n »). Jamais au journal.
    #[serde(default)]
    pub nom: String,
    #[serde(default)]
    pub jeux: Vec<JeuBoutique>,
    #[serde(default)]
    pub maj_le: String,
    #[serde(default)]
    pub connecte_le: String,
    #[serde(default)]
    pub navigateur: NavigateurGog,
    /// Les numéros de jeux de la dernière bibliothèque de Galaxy lue pour ce compte (écartés compris) : le cache commun
    /// des fiches (`fiches_gog`) garde ceux de TOUS les comptes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ids_lus: Vec<String>,
    /// Le numéro du « Compte GOG n » affiché sans pseudo : attribué à l'arrivée du compte et rangé (jamais sa place dans
    /// la liste, qui change quand un compte est oublié).
    #[serde(default)]
    pub numero: u32,
}

impl CompteGog {
    /// La clé du compte pour l'interface et les commandes (son numéro Galaxy, sinon « site-<numéro du site> »). Jamais
    /// affichée : l'interface montre le pseudo, sinon « Compte GOG n ».
    pub fn cle(&self) -> String {
        if self.id_galaxy.is_empty() {
            format!("site-{}", self.id_site)
        } else {
            self.id_galaxy.clone()
        }
    }
}

/// Le nom du secret (coffre de Windows) du jeton de renouvellement d'un compte GOG : celui du compte `Profil` reste
/// `<profil>#gog-renouvellement` (comme avant la 0.219) ; un compte ajouté : `<profil>#gog-renouvellement#<id_galaxy>`.
pub fn secret_compte_gog(profil: &str, navigateur: NavigateurGog, id_galaxy: &str) -> String {
    match navigateur {
        NavigateurGog::Profil => nom_secret(profil, crate::gog_galaxy::SECRET),
        NavigateurGog::Ajoute => secret_gog_ajoute(profil, id_galaxy),
    }
}

/// Le secret d'un compte GOG AJOUTÉ (son numéro Galaxy).
pub fn secret_gog_ajoute(profil: &str, id_galaxy: &str) -> String {
    nom_secret(profil, &format!("{}#{id_galaxy}", crate::gog_galaxy::SECRET))
}

/// Les comptes GOG d'une source « gog » (lot C4b) : `comptes_gog`, ou, dans un fichier d'avant la 0.219 (`comptes_gog`
/// vide, `id_compte` rempli), un compte `Profil` repris de `id_compte` et `jeux` (son numéro Galaxy sera rempli à la
/// lecture suivante). Pur, testé.
pub fn comptes_gog_lus(c: &CompteBoutique) -> Vec<CompteGog> {
    if !c.comptes_gog.is_empty() || !crate::gratuits::numero_gog(&c.id_compte) {
        return c.comptes_gog.clone();
    }
    vec![CompteGog {
        id_site: c.id_compte.clone(),
        id_galaxy: String::new(),
        nom: String::new(),
        jeux: c.jeux.clone(),
        maj_le: c.maj_le.clone(),
        connecte_le: c.connecte_le.clone(),
        navigateur: NavigateurGog::Profil,
        ids_lus: vec![],
        numero: 1,
    }]
}

/// Range `comptes` dans la source et garde les anciens champs (`id_compte`, `jeux`, `maj_le`, `connecte_le`) en
/// accord avec le compte `Profil`, pour un retour arrière d'une version.
pub fn poser_comptes_gog(c: &mut CompteBoutique, mut comptes: Vec<CompteGog>) {
    // Un compte sans numéro « Compte GOG n » en reçoit un, jamais déjà pris (les autres gardent le leur).
    let mut suivant = comptes.iter().map(|x| x.numero).max().unwrap_or(0);
    for x in comptes.iter_mut().filter(|x| x.numero == 0) {
        suivant += 1;
        x.numero = suivant;
    }
    let profil = comptes.iter().find(|x| x.navigateur == NavigateurGog::Profil);
    match profil {
        Some(p) if crate::gratuits::numero_gog(&p.id_site) => {
            c.id_compte = p.id_site.clone();
            c.jeux = p.jeux.clone();
            c.maj_le = p.maj_le.clone();
            c.connecte_le = p.connecte_le.clone();
        }
        _ => {
            if !c.id_compte.is_empty() {
                c.id_compte_precedent = std::mem::take(&mut c.id_compte);
            }
            c.jeux.clear();
        }
    }
    c.compte = "GOG".into();
    c.comptes_gog = comptes;
}

/// Les jeux de TOUS les comptes GOG réunis (lot C4b) : une seule carte par jeu (même id `gog_<n>`), le compte `Profil`
/// d'abord ; minutes et dernière partie au plus grand ; copies, image et titre du premier qui en a. Pur, testé.
pub fn jeux_gog_reunis(comptes: &[CompteGog]) -> Vec<JeuBoutique> {
    let mut ordre: Vec<&CompteGog> = comptes.iter().filter(|c| c.navigateur == NavigateurGog::Profil).collect();
    ordre.extend(comptes.iter().filter(|c| c.navigateur != NavigateurGog::Profil));
    let mut l: Vec<JeuBoutique> = Vec::new();
    let mut place: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for c in ordre {
        for j in &c.jeux {
            match place.get(&j.id) {
                Some(&i) => {
                    let x = &mut l[i];
                    x.minutes = x.minutes.max(j.minutes);
                    x.derniere = x.derniere.max(j.derniere);
                    if x.releases.is_empty() {
                        x.releases = j.releases.clone();
                    }
                    if x.image.is_none() {
                        x.image = j.image.clone();
                    }
                    if x.nom.trim().is_empty() || x.nom == x.id || x.nom.starts_with("Jeu GOG ") {
                        x.nom = j.nom.clone();
                    }
                }
                None => {
                    place.insert(j.id.clone(), l.len());
                    l.push(j.clone());
                }
            }
        }
    }
    l
}

/// Ce que la base de GOG Galaxy de CE PC dit de son compte (lot C4b), relu au moment même (installer, lancer) : le
/// compte ouvert (un seul compte dans sa base) et TOUTES les clés de sa bibliothèque (`LibraryReleases`, jeux cachés et
/// DLC compris : jamais un « absent » conclu d'une liste filtrée).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CompteGalaxyPc {
    pub compte: String,
    pub cles: BTreeSet<String>,
}

/// Le compte GOG (son nom, possiblement vide, et son numéro « Compte GOG n ») qui possède ce jeu SANS que le compte
/// ouvert dans GOG Galaxy le possède (lot C4b) : `cle` = numéro du jeu GOG ; `galaxy` : relu à l'instant
/// (`compte_galaxy_du_pc`). `None` dans le doute : compte de Galaxy inconnu, un compte du profil au numéro Galaxy
/// inconnu, jeu d'aucun compte, ou jeu présent dans la bibliothèque complète de Galaxy. Pur, testé.
pub fn compte_hors_galaxy(comptes: &[CompteGog], galaxy: &CompteGalaxyPc, cle: &str) -> Option<(String, u32)> {
    if !crate::gratuits::numero_gog(&galaxy.compte) || !crate::gratuits::numero_gog(cle) || comptes.iter().any(|c| c.id_galaxy.is_empty()) {
        return None;
    }
    let id = format!("gog_{cle}");
    if galaxy.cles.contains(&id) {
        return None;
    }
    let possesseurs: Vec<&CompteGog> = comptes.iter().filter(|c| c.jeux.iter().any(|j| j.id == id)).collect();
    if possesseurs.is_empty() || possesseurs.iter().any(|c| c.id_galaxy == galaxy.compte) {
        return None;
    }
    Some((possesseurs[0].nom.clone(), possesseurs[0].numero))
}

/// Le nom d'un compte GOG pour les messages : son pseudo, sinon « Compte GOG n » (`numero`, attribué à son arrivée et
/// rangé : jamais sa place dans la liste, jamais son numéro de compte).
pub fn nom_compte_gog(nom: &str, numero: u32) -> String {
    if nom.trim().is_empty() {
        format!("Compte GOG {}", numero.max(1))
    } else {
        nom.to_string()
    }
}

/// GOG Galaxy est-il ouvert sur un AUTRE compte que tous ceux de Frogtend (lot C4b) ? Seulement si on le sait : le
/// compte de Galaxy connu (un seul compte dans sa base), au moins un compte de Frogtend, TOUS au numéro Galaxy connu,
/// et aucun qui corresponde. Pur, testé.
pub fn galaxy_sur_un_autre_compte(comptes: &[CompteGog], compte_galaxy: &str) -> bool {
    if !crate::gratuits::numero_gog(compte_galaxy) || comptes.is_empty() || comptes.iter().any(|c| c.id_galaxy.is_empty()) {
        return false;
    }
    !comptes.iter().any(|c| c.id_galaxy == compte_galaxy)
}

/// Le compte ouvert dans GOG Galaxy sur ce PC et toutes les clés de sa bibliothèque (lot C4b), relus sur une COPIE de
/// sa base (jamais l'original ; une ancienne copie retirée d'abord), effacée ensuite (elle contient les données du
/// compte) ; la place vérifiée avant. `None` : Galaxy absent, base illisible, ou compte pas sûr (rien d'affirmé).
pub fn compte_galaxy_du_pc(storage: &Path, travail: &Path) -> Option<CompteGalaxyPc> {
    struct Efface<'a>(&'a Path);
    impl Drop for Efface<'_> {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(self.0);
        }
    }
    let _efface = Efface(travail);
    let fichiers = ["galaxy-2.0.db", "galaxy-2.0.db-wal", "galaxy-2.0.db-shm"];
    let besoin: u64 = fichiers.iter().filter_map(|f| std::fs::metadata(storage.join(f)).ok()).map(|m| m.len()).sum::<u64>() + (64 << 20);
    std::fs::create_dir_all(travail).ok()?;
    if crate::jeux_pc::place_libre(travail).is_some_and(|l| l < besoin) {
        return None;
    }
    for f in fichiers {
        let (src, dst) = (storage.join(f), travail.join(f));
        // Jamais un vieux journal (-wal, -shm) d'une copie d'avant mêlé à la base copiée maintenant.
        let _ = std::fs::remove_file(&dst);
        if src.is_file() {
            std::fs::copy(&src, &dst).ok()?;
        }
    }
    let c = rusqlite::Connection::open(travail.join("galaxy-2.0.db")).ok()?;
    let compte = compte_unique_galaxy(&c).ok().flatten()?;
    let u: i64 = compte.parse().ok()?;
    let mut q = c.prepare("SELECT releaseKey FROM LibraryReleases WHERE userId = ?1").ok()?;
    let cles: BTreeSet<String> = q.query_map([u], |r| r.get::<_, String>(0)).ok()?.flatten().collect();
    Some(CompteGalaxyPc { compte, cles })
}

/// Ce qui reste d'un compte Steam oublié : seulement le compte précédent (ni clé, ni connexion, ni liste). `None` s'il
/// n'y en avait aucun.
pub fn compte_oublie(c: Option<CompteBoutique>) -> Option<CompteBoutique> {
    let c = c?;
    let precedent = if c.steamid.is_empty() { c.steamid_precedent } else { c.steamid };
    (!precedent.is_empty()).then(|| CompteBoutique { steamid_precedent: precedent, menage_ancien_compte: c.menage_ancien_compte, ..Default::default() })
}

/// Le compte Steam qui comptait AVANT (le rangé, sinon celui d'avant un oubli) : vide = aucun.
pub fn compte_steam_d_avant(c: &CompteBoutique) -> &str {
    if c.steamid.is_empty() {
        &c.steamid_precedent
    } else {
        &c.steamid
    }
}

/// Le profil s'est-il connecté à Steam par la page officielle (et son `steamid` est-il bon) ?
pub fn connecte_par_la_page(c: &CompteBoutique) -> bool {
    !c.connecte_le.is_empty() && crate::gratuits::steamid_valide(&c.steamid)
}

/// Le compte Steam de la clé d'API rangée ; vide = inconnu.
pub fn steamid_de_la_cle(c: &CompteBoutique) -> &str {
    if !c.steamid_cle.is_empty() {
        &c.steamid_cle
    } else if c.connecte_le.is_empty() {
        &c.steamid
    } else {
        ""
    }
}

/// La clé d'API rangée est-elle celle d'un AUTRE compte que celui connecté par la page ? L'import par la clé est alors
/// refusé (jamais la clé de A avec le compte de B) ; la clé n'est pas effacée pour autant.
pub fn cle_d_un_autre_compte(c: &CompteBoutique) -> bool {
    let k = steamid_de_la_cle(c);
    !k.is_empty() && k != c.steamid
}

/// Retient la connexion à Steam par sa page officielle : le `steamid` (déjà vérifié) et l'heure, relus et écrits sous
/// le verrou. Un AUTRE compte que celui déjà enregistré : l'ancienne liste importée ne vaut plus pour lui (comme
/// `boutique_steam_regler`) ; le compte de la clé d'API est gardé à part pour ne jamais les mélanger.
/// Rend vrai quand un AUTRE compte était déjà enregistré (changement de compte : l'appelant retire alors de la
/// ludothèque les cartes de l'ancien compte jamais installées ni jouées, décision de Seb du 20/11).
pub fn noter_connexion_steam(dossier_profil: &Path, steamid: &str, quand: &str) -> Resultat<bool> {
    if !crate::gratuits::steamid_valide(steamid) {
        return Err(Erreur::Refus("Identifiant Steam invalide.".into()));
    }
    let mut change = false;
    modifier_source(dossier_profil, "steam", |c| {
        let mut c = c.unwrap_or_default();
        if c.steamid_cle.is_empty() {
            c.steamid_cle = steamid_de_la_cle(&c).to_string();
        }
        // L'ancien compte : celui rangé, ou celui d'avant un « Oublier » (Seb 20/11). Le ménage est noté EN ATTENTE ici,
        // sous le verrou, et effacé seulement quand il est fait (`menage_ancien_compte`).
        let avant = compte_steam_d_avant(&c).to_string();
        change = !avant.is_empty() && avant != steamid;
        if change {
            c.menage_ancien_compte = true;
        }
        c.steamid_precedent.clear();
        if c.steamid != steamid {
            c.jeux.clear();
            c.maj_le.clear();
            c.compte = steamid.to_string();
        }
        if c.compte.is_empty() {
            c.compte = steamid.to_string();
        }
        c.steamid = steamid.to_string();
        c.connecte_le = quand.to_string();
        Ok(Some(c))
    })?;
    Ok(change)
}

/// Le nom du secret d'une boutique dans le coffre.
pub fn nom_secret(profil: &str, boutique: &str) -> String {
    format!("{profil}#{boutique}")
}

/// Accepte « NimaiTakahashi », « https://steamcommunity.com/id/NimaiTakahashi/ » ou un identifiant à 17 chiffres
/// (« …/profiles/7656119… »). Rend (personnalisé ?, valeur).
pub fn lire_compte_steam(saisie: &str) -> Option<(bool, String)> {
    let s = saisie.trim().trim_end_matches('/');
    let dernier = s.rsplit('/').next().unwrap_or(s).trim();
    if dernier.is_empty() || dernier.len() > 64 || !dernier.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return None;
    }
    let numerique = dernier.len() == 17 && dernier.chars().all(|c| c.is_ascii_digit());
    Some((!numerique, dernier.to_string()))
}

fn client() -> Resultat<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("Frogtend")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|_| Erreur::Reseau("Connexion impossible.".into()))
}

/// Une requête à l'API Steam : la réponse lue (200), ou le code d'un autre statut. Les erreurs ne contiennent JAMAIS
/// l'adresse (elle porte la clé ou le jeton).
async fn requete(base: &str, chemin: &str, parametres: &[(&str, &str)]) -> Resultat<Result<Value, u16>> {
    let r = client()?
        .get(format!("{base}{chemin}"))
        .query(parametres)
        .send()
        .await
        .map_err(|e| Erreur::Reseau(format!("Steam ne répond pas ({}).", e.without_url())))?;
    match r.status().as_u16() {
        200 => r.json().await.map(Ok).map_err(|_| Erreur::Serveur("Réponse de Steam illisible.".into())),
        c => Ok(Err(c)),
    }
}

const MOTIF_ATTENDRE: &str = "Steam demande d'attendre un peu avant de réessayer.";

/// Le message d'un statut d'erreur de Steam ; `refus` : celui d'un 401 ou d'un 403.
fn erreur_de_statut(code: u16, refus: &str) -> Erreur {
    match code {
        401 | 403 => Erreur::Refus(refus.into()),
        429 => Erreur::Refus(MOTIF_ATTENDRE.into()),
        c => Erreur::Serveur(format!("Steam a répondu une erreur (code {c}).")),
    }
}

/// Une requête à l'API Steam avec la clé d'API.
async fn appeler(base: &str, chemin: &str, parametres: &[(&str, &str)]) -> Resultat<Value> {
    requete(base, chemin, parametres).await?.map_err(|c| erreur_de_statut(c, "Steam refuse la clé d'API : vérifie-la (ou crée-en une nouvelle)."))
}

/// L'identifiant Steam 64 bits d'un compte (URL personnalisée résolue par Steam).
pub async fn steamid(base: &str, cle: &str, saisie: &str) -> Resultat<String> {
    let (perso, valeur) = lire_compte_steam(saisie).ok_or_else(|| Erreur::Refus("Compte Steam illisible : donne l'URL de ton profil ou son nom.".into()))?;
    if !perso {
        return Ok(valeur);
    }
    let v = appeler(base, "/ISteamUser/ResolveVanityURL/v1/", &[("key", cle), ("vanityurl", &valeur)]).await?;
    match (v["response"]["success"].as_i64(), v["response"]["steamid"].as_str()) {
        (Some(1), Some(id)) => Ok(id.to_string()),
        _ => Err(Erreur::Introuvable(format!("Steam ne connaît pas le profil « {valeur} ». Vérifie ton URL personnalisée."))),
    }
}

/// Les jeux possédés (y compris gratuits joués). Un profil Steam privé rend une liste vide : on le dit.
pub async fn jeux_possedes(base: &str, cle: &str, steamid: &str) -> Resultat<Vec<JeuBoutique>> {
    let v = appeler(
        base,
        "/IPlayerService/GetOwnedGames/v1/",
        &[("key", cle), ("steamid", steamid), ("include_appinfo", "1"), ("include_played_free_games", "1"), ("format", "json")],
    )
    .await?;
    lire_jeux_possedes(&v, "Steam ne montre aucun jeu : dans Steam ▸ Profil ▸ Modifier ▸ Confidentialité, mets « Détails du jeu » sur Public.")
}

/// Le refus quand la connexion du profil à Steam ne vaut plus (pas de jeton, jeton refusé ou expiré) : l'interface le
/// reconnaît pour dire « reconnecte-toi » sans alarmer.
pub const MOTIF_RECONNECTER: &str = "Steam ne te reconnaît plus : reconnecte-toi à Steam dans 🛒 Boutiques.";

/// La boutique Steam, où elle donne le jeton d'accès du compte connecté (`/pointssummary/ajaxgetasyncconfig`).
pub const BOUTIQUE_STEAM: &str = "https://store.steampowered.com";

/// Lot C3 (« Oublier ma connexion Steam ») : un domaine de cookie appartient-il à Steam ? Suffixe EXACT
/// (`steampowered.com`, `steamcommunity.com` et leurs sous-domaines, point de tête ou non), jamais un « contient » :
/// `evilsteampowered.com` n'est pas Steam. Le navigateur du profil sert aussi à Epic et PlayStation : seul ce filtre
/// décide de ce qui s'efface.
pub fn domaine_steam(domaine: &str) -> bool {
    let d = domaine.trim_start_matches('.').to_ascii_lowercase();
    ["steampowered.com", "steamcommunity.com"].iter().any(|s| d == *s || d.ends_with(&format!(".{s}")))
}

/// Un domaine de GOG (lot C4b : « Oublier » le compte GOG du navigateur du profil) : `gog.com` et ses sous-domaines,
/// point de tête ou non, jamais un « contient » (`evilgog.com` n'est pas GOG). Seul ce filtre décide de ce qui s'efface.
pub fn domaine_gog(domaine: &str) -> bool {
    let d = domaine.trim_start_matches('.').to_ascii_lowercase();
    d == "gog.com" || d.ends_with(".gog.com")
}

/// Un cookie qui porte la connexion Steam : `steamLoginSecure` (jeton court) ou `steamRefresh_*` (celui qui le
/// renouvelle en silence — revue de l'expert du lancement, 21/11).
pub fn cookie_de_connexion_steam(nom: &str) -> bool {
    nom == "steamLoginSecure" || nom.starts_with("steamRefresh_")
}

/// Un cookie du navigateur, SANS sa valeur (lot C3 : on n'a besoin que de quoi l'effacer et le compter).
#[derive(Debug, Clone, PartialEq)]
pub struct CookieSansValeur {
    pub nom: String,
    pub domaine: String,
    pub chemin: String,
}

/// La preuve de la déconnexion : plus AUCUN cookie de connexion Steam, sur aucun domaine Steam.
pub fn connexion_steam_effacee(cookies: &[CookieSansValeur]) -> bool {
    !cookies.iter().any(|c| domaine_steam(&c.domaine) && cookie_de_connexion_steam(&c.nom))
}

/// Pour le journal : combien de cookies Steam, dont combien de connexion. Jamais un nom de compte ni une valeur.
pub fn resume_cookies_steam_restants(cookies: &[CookieSansValeur]) -> String {
    let steam: Vec<&CookieSansValeur> = cookies.iter().filter(|c| domaine_steam(&c.domaine)).collect();
    let connexion = steam.iter().filter(|c| cookie_de_connexion_steam(&c.nom)).count();
    format!("{} cookie(s) Steam, dont {connexion} de connexion", steam.len())
}

/// L'en-tête `Cookie` formé des seuls cookies de la boutique Steam du navigateur du profil. Il ne vit qu'en mémoire, le
/// temps de lire le jeton ; son `Debug` ne montre RIEN (journal, erreur).
pub struct CookiesSteam(String);

impl std::fmt::Debug for CookiesSteam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CookiesSteam(<cachés>)")
    }
}

/// Forme l'en-tête `Cookie` à partir des cookies lus (nom, valeur, domaine) : seulement ceux du domaine EXACT
/// `store.steampowered.com` (avec ou sans point de tête), aux noms et valeurs sûrs (jamais « ; », espace ni saut de
/// ligne). `None` sans le cookie de connexion `steamLoginSecure` : la personne n'est pas (ou plus) connectée.
pub fn cookies_de_la_boutique_steam(cookies: &[(String, String, Option<String>)]) -> Option<CookiesSteam> {
    let sur = |t: &str| !t.is_empty() && t.len() <= 4096 && t.bytes().all(|c| c.is_ascii_graphic() && c != b';' && c != b',');
    let gardes: Vec<&(String, String, Option<String>)> = cookies
        .iter()
        .filter(|(n, v, d)| {
            d.as_deref().is_some_and(|d| d.trim_start_matches('.').eq_ignore_ascii_case("store.steampowered.com")) && sur(n) && !n.contains('=') && sur(v)
        })
        .collect();
    if !gardes.iter().any(|(n, _, _)| n == "steamLoginSecure") {
        return None;
    }
    Some(CookiesSteam(gardes.iter().map(|(n, v, _)| format!("{n}={v}")).collect::<Vec<_>>().join("; ")))
}

/// Lot C2 : le jeton d'accès (`webapi_token`) du compte connecté, demandé à la boutique Steam DEPUIS LE CŒUR avec les
/// cookies du profil (jamais par une page : un titre de page finit dans l'historique du navigateur, sur le disque).
/// Aucune redirection n'est suivie (les cookies ne partent jamais ailleurs). Pas de jeton, redirection vers la
/// connexion, 401 ou 403 : `MOTIF_RECONNECTER`. Les erreurs ne contiennent jamais l'adresse, les cookies ni le jeton.
pub async fn jeton_par_cookies(base: &str, cookies: &CookiesSteam) -> Resultat<String> {
    jeton_par_cookies_etape(base, cookies).await.0
}

/// Comme `jeton_par_cookies`, avec l'ÉTAPE atteinte, pour le journal (0.215.1) : un statut ou un mot, JAMAIS une valeur
/// (ni cookie, ni jeton, ni adresse).
pub async fn jeton_par_cookies_etape(base: &str, cookies: &CookiesSteam) -> (Resultat<String>, String) {
    let client = match reqwest::Client::builder()
        .user_agent("Frogtend")
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()
    {
        Ok(c) => c,
        Err(_) => return (Err(Erreur::Reseau("Connexion impossible.".into())), "client impossible".into()),
    };
    let r = match client.get(format!("{base}/pointssummary/ajaxgetasyncconfig")).header(reqwest::header::COOKIE, cookies.0.as_str()).send().await {
        Ok(r) => r,
        Err(e) => return (Err(Erreur::Reseau(format!("Steam ne répond pas ({}).", e.without_url()))), "pas de réponse".into()),
    };
    let illisible = || Erreur::Serveur("Steam a changé sa page : Frogtend ne sait plus y lire tes jeux. En attendant, la clé d'API (🛟 Secours) reste possible.".into());
    let statut = r.status().as_u16();
    match statut {
        200 => {}
        300..=399 | 401 | 403 => return (Err(Erreur::Refus(MOTIF_RECONNECTER.into())), format!("statut {statut} (refus ou renvoi vers la connexion)")),
        c => return (Err(erreur_de_statut(c, MOTIF_RECONNECTER)), format!("statut {c}")),
    }
    let v: Value = match r.json().await {
        Ok(v) => v,
        Err(_) => return (Err(illisible()), "statut 200, réponse pas en JSON".into()),
    };
    if !v.is_object() {
        return (Err(illisible()), "statut 200, réponse inattendue".into());
    }
    match &v["data"]["webapi_token"] {
        Value::String(j) if crate::gratuits::forme_de_jwt(j) => (Ok(j.clone()), "jeton reçu".into()),
        Value::String(j) if !j.is_empty() => (Err(illisible()), "statut 200, jeton de forme inattendue".into()),
        Value::String(_) | Value::Null => {
            (Err(Erreur::Refus(MOTIF_RECONNECTER.into())), format!("statut 200, pas de jeton (success = {})", v["success"]))
        }
        _ => (Err(illisible()), "statut 200, jeton de type inattendu".into()),
    }
}

/// Le résumé des cookies lus, pour le journal (0.215.1) : leur NOMBRE par domaine et la présence du cookie de connexion,
/// jamais une valeur ni un autre nom.
pub fn resume_cookies_steam(cookies: &[(String, String, Option<String>)], page_chargee: bool) -> String {
    let boutique = |d: &Option<String>| d.as_deref().is_some_and(|d| d.trim_start_matches('.').eq_ignore_ascii_case("store.steampowered.com"));
    let de_la_boutique = cookies.iter().filter(|(_, _, d)| boutique(d)).count();
    let connexion = cookies.iter().any(|(n, _, d)| n == "steamLoginSecure" && boutique(d));
    format!(
        "boutique {}, {} cookie(s) lu(s) dont {} de la boutique, cookie de connexion {}",
        if page_chargee { "chargée" } else { "PAS chargée en 20 s" },
        cookies.len(),
        de_la_boutique,
        if connexion { "présent" } else { "ABSENT" }
    )
}

/// Lot C2 : les jeux possédés avec le JETON d'accès du compte connecté par la page (`access_token`, à la place de la
/// clé d'API). Le jeton n'est jamais gardé, ni écrit, ni journalisé ; les erreurs ne contiennent jamais l'adresse.
/// 429 : UNE seule nouvelle tentative, après quelques secondes (jamais de boucle).
pub async fn jeux_possedes_par_jeton(base: &str, jeton: &str, steamid: &str) -> Resultat<Vec<JeuBoutique>> {
    jeux_possedes_par_jeton_apres(base, jeton, steamid, std::time::Duration::from_secs(5)).await
}

async fn jeux_possedes_par_jeton_apres(base: &str, jeton: &str, steamid: &str, attente: std::time::Duration) -> Resultat<Vec<JeuBoutique>> {
    let parametres =
        [("access_token", jeton), ("steamid", steamid), ("include_appinfo", "1"), ("include_played_free_games", "1"), ("format", "json")];
    let mut r = requete(base, "/IPlayerService/GetOwnedGames/v1/", &parametres).await?;
    if r == Err(429) {
        tokio::time::sleep(attente).await;
        r = requete(base, "/IPlayerService/GetOwnedGames/v1/", &parametres).await?;
    }
    let v = r.map_err(|c| erreur_de_statut(c, MOTIF_RECONNECTER))?;
    lire_jeux_possedes(&v, "Steam n'a pas rendu la liste de tes jeux : réessaie dans un moment (ou reconnecte-toi à Steam).")
}

/// Le JWT de Steam vaut-il pour CE compte ? La charge (2ᵉ morceau, base64url) est lue SANS vérifier la signature (Steam
/// la vérifie lui-même) : son `sub` doit être le `steamid` rangé (et valide) ; un `exp` passé (secondes) le dit
/// expiré. Le jeton n'apparaît jamais dans le message.
pub fn verifier_jeton_steam(jeton: &str, steamid: &str, maintenant: u64) -> Resultat<()> {
    let autre = || Erreur::Refus("Le compte Steam ouvert n'est pas celui connecté à Frogtend : reconnecte-toi avec le bon compte.".into());
    if !crate::gratuits::forme_de_jwt(jeton) || !crate::gratuits::steamid_valide(steamid) {
        return Err(Erreur::Refus("Steam a rendu une réponse inattendue : réessaie dans un moment.".into()));
    }
    let charge = jeton.split('.').nth(1).and_then(base64url).and_then(|o| serde_json::from_slice::<Value>(&o).ok());
    let Some(charge) = charge.filter(Value::is_object) else {
        return Err(Erreur::Refus("Steam a rendu une réponse inattendue : réessaie dans un moment.".into()));
    };
    // `sub` est un texte chez Steam ; un nombre est accepté s'il dit la même chose.
    let sub = match &charge["sub"] {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => return Err(autre()),
    };
    if sub != steamid {
        return Err(autre());
    }
    if charge["exp"].as_u64().is_some_and(|e| e <= maintenant) {
        return Err(Erreur::Refus(MOTIF_RECONNECTER.into()));
    }
    Ok(())
}

/// Décode du base64url (sans remplissage, ou avec) ; `None` si un caractère n'en est pas.
pub(crate) fn base64url(s: &str) -> Option<Vec<u8>> {
    let s = s.trim_end_matches('=');
    let valeur = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'-' | b'+' => 62,
            b'_' | b'/' => 63,
            _ => return None,
        } as u32)
    };
    if s.len() % 4 == 1 {
        return None;
    }
    let mut o = Vec::with_capacity(s.len() * 3 / 4);
    for bloc in s.as_bytes().chunks(4) {
        let mut n = 0u32;
        for (i, c) in bloc.iter().enumerate() {
            n |= valeur(*c)? << (18 - 6 * i);
        }
        o.push((n >> 16) as u8);
        if bloc.len() > 2 {
            o.push((n >> 8) as u8);
        }
        if bloc.len() > 3 {
            o.push(n as u8);
        }
    }
    Some(o)
}

/// Lit la réponse de `GetOwnedGames` ; `sans_liste` : le message quand Steam ne donne pas de liste. Un compte SANS
/// aucun jeu (`game_count` 0) rend une liste vide.
fn lire_jeux_possedes(v: &Value, sans_liste: &str) -> Resultat<Vec<JeuBoutique>> {
    let Some(jeux) = v["response"]["games"].as_array() else {
        if v["response"]["game_count"].as_u64() == Some(0) {
            return Ok(vec![]);
        }
        return Err(Erreur::Refus(sans_liste.into()));
    };
    let mut l: Vec<JeuBoutique> = jeux
        .iter()
        .filter_map(|j| {
            Some(JeuBoutique {
                id: j["appid"].as_u64()?.to_string(),
                nom: j["name"].as_str().unwrap_or("?").to_string(),
                minutes: j["playtime_forever"].as_u64().unwrap_or(0),
                derniere: j["rtime_last_played"].as_u64().unwrap_or(0),
                installe: false,
                plateforme: String::new(),
                image: None,
                dossier: None,
                pret: None,
                succes: None,
                releases: vec![],
                titre_douteux: false,
            })
        })
        .collect();
    l.sort_by(|a, b| a.nom.to_lowercase().cmp(&b.nom.to_lowercase()));
    Ok(l)
}

/// Une forme comparable d'un chemin Windows (« / » ou « \ », majuscules, séparateur final, préfixe `\\?\`) : le
/// registre donne `d:/launchbox/games/steam` et le vdf `D:\LaunchBox\Games\Steam` pour le MÊME dossier (relevé sur
/// Venkman, 20/10) ; Ubisoft écrit ses dossiers avec des « / ». Un chemin réseau garde ses deux « \ » de tête.
pub(crate) fn cle_chemin(p: &Path) -> String {
    let t = p.to_string_lossy().replace('/', "\\");
    let t = if let Some(r) = t.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{r}")
    } else {
        t.strip_prefix(r"\\?\").map(String::from).unwrap_or(t)
    };
    t.trim_end_matches('\\').to_lowercase()
}

/// Les bibliothèques Steam de ce PC (`libraryfolders.vdf`, lignes `"path" "D:\\SteamLibrary"`), chacune une fois.
pub fn bibliotheques_steam(dossier_steam: &Path) -> Vec<PathBuf> {
    let mut l = vec![dossier_steam.to_path_buf()];
    // Lu en octets : un vdf qui ne serait pas tout à fait de l'UTF-8 ne fait pas perdre toutes les bibliothèques.
    let vdf = std::fs::read(dossier_steam.join("steamapps").join("libraryfolders.vdf")).map(|o| String::from_utf8_lossy(&o).into_owned()).unwrap_or_default();
    for ligne in vdf.lines() {
        let morceaux: Vec<&str> = ligne.split('"').collect();
        if morceaux.len() >= 4 && morceaux[1].eq_ignore_ascii_case("path") {
            let p = PathBuf::from(morceaux[3].replace("\\\\", "\\"));
            if !l.iter().any(|x| cle_chemin(x) == cle_chemin(&p)) {
                l.push(p);
            }
        }
    }
    l
}

/// Ce qu'un `appmanifest_<appid>.acf` dit d'un jeu : son numéro, son dossier (UN nom, sous `steamapps\common`) et son
/// état (`StateFlags`).
#[derive(Debug, Clone, PartialEq)]
pub struct ManifesteSteam {
    pub appid: String,
    pub installdir: String,
    pub etat: u64,
    /// La version installée (`buildid` ; jamais `TargetBuildID`, celle qu'une mise à jour visera) — facultative.
    pub buildid: Option<String>,
    /// Le nom du jeu (`name`), quand le manifeste le donne.
    pub nom: Option<String>,
}

/// Lit les clés simples d'un manifeste Steam (format texte de Valve : `"clé"  "valeur"`, une par ligne).
pub fn lire_manifeste_steam(texte: &str) -> Option<ManifesteSteam> {
    let valeur = |cle: &str| {
        texte.lines().find_map(|l| {
            let m: Vec<&str> = l.split('"').collect();
            (m.len() >= 4 && m[1].eq_ignore_ascii_case(cle)).then(|| m[3].to_string())
        })
    };
    Some(ManifesteSteam {
        appid: valeur("appid").filter(|a| !a.is_empty() && a.chars().all(|c| c.is_ascii_digit()))?,
        installdir: valeur("installdir")?,
        etat: valeur("StateFlags")?.parse().ok()?,
        buildid: valeur("buildid").filter(|b| !b.is_empty() && b.chars().all(|c| c.is_ascii_digit())),
        nom: valeur("name").map(|n| n.trim().to_string()).filter(|n| !n.is_empty()),
    })
}

/// Un jeu Steam prêt à jouer : entièrement installé (bit 4 de `StateFlags`) et rien en cours (mise à jour requise ou
/// en cours, fichiers manquants ou abîmés, désinstallation, vérification, téléchargement…). Valeurs de `EAppState`
/// (SteamKit) ; « Encrypted » (8, un pré-chargement pas encore jouable) empêche aussi ; « AppRunning » (64) et
/// « Locked » (16) n'empêchent pas.
pub fn steam_pret(etat: u64) -> bool {
    const EMPECHE: u64 = 2 | 8 | 32 | 128 | 256 | 512 | 1024 | 2048 | 4096 | 65536 | 131072 | 262144 | 524288 | 1048576 | 2097152 | 4194304 | 8388608;
    etat & 4 != 0 && etat & EMPECHE == 0
}

/// Un jeu Steam INSTALLÉ (bit 4) : il est sur le disque, même si une mise à jour l'attend (« mettre à jour au
/// lancement » : 6) ; s'il n'est pas `steam_pret`, Steam le mettra à jour au lancement.
pub fn steam_installe(etat: u64) -> bool {
    etat & 4 != 0
}

/// Un jeu Steam installé sur ce PC : son dossier et s'il est prêt à lancer tout de suite.
#[derive(Debug, Clone, PartialEq)]
pub struct InstalleSteam {
    pub dossier: PathBuf,
    pub pret: bool,
    /// Sa version installée (`buildid`), si l'appmanifest la donne.
    pub buildid: Option<String>,
    /// Steam le dit en cours (« AppRunning », 64) : rien n'y est posé ni retiré.
    pub lance: bool,
    /// Le nom du jeu, d'après son manifeste.
    pub nom: Option<String>,
}

/// Les jeux Steam installés sur ce PC, avec leur dossier : `<bibliothèque>\steamapps\common\<installdir>`, où
/// `installdir` doit être UN nom sûr (jamais vide, jamais « .. ») et le dossier exister.
pub fn installes_steam_dossiers(dossier_steam: &Path) -> std::collections::BTreeMap<String, InstalleSteam> {
    let mut s = std::collections::BTreeMap::new();
    for b in bibliotheques_steam(dossier_steam) {
        let Ok(entrees) = std::fs::read_dir(b.join("steamapps")) else { continue };
        for e in entrees.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if !(n.starts_with("appmanifest_") && n.ends_with(".acf")) || !e.metadata().is_ok_and(|m| m.len() <= 1024 * 1024) {
                continue;
            }
            let Some(m) = std::fs::read(e.path()).ok().and_then(|o| lire_manifeste_steam(&String::from_utf8_lossy(&o))) else { continue };
            if !steam_installe(m.etat) || crate::jeux_pc::nom_de_fichier_sur(&m.installdir).is_err() {
                continue;
            }
            let dossier = b.join("steamapps").join("common").join(&m.installdir);
            if dossier.is_dir() {
                s.insert(m.appid, InstalleSteam { dossier, pret: steam_pret(m.etat), lance: m.etat & 64 != 0, buildid: m.buildid, nom: m.nom });
            }
        }
    }
    s
}


/// Le dossier de Steam sur ce PC (registre `HKCU\Software\Valve\Steam`, valeur `SteamPath`, LU par l'API de Windows :
/// un chemin accentué reste juste). Jamais un dossier supposé (lot A, 18/11 : rien ne se suppose sur le PC) : sans
/// cette valeur, Steam est absent.
pub fn dossier_steam() -> Option<PathBuf> {
    let lu = crate::registre::lire_texte_hkcu(r"Software\Valve\Steam", "SteamPath").map(|v| PathBuf::from(v.trim().replace('/', "\\")));
    lu.filter(|p| p.is_dir())
}

fn fichier(dossier_profil: &Path) -> PathBuf {
    dossier_profil.join("boutiques.json")
}

/// Le compte Steam enregistré d'un profil (sans secret).
pub fn lire(dossier_profil: &Path) -> Option<CompteBoutique> {
    lire_source(dossier_profil, "steam")
}

pub fn ecrire(dossier_profil: &Path, c: Option<&CompteBoutique>) -> Resultat<()> {
    ecrire_source(dossier_profil, "steam", c)
}

/// Ce qui est gardé pour une source (« steam », « galaxy ») d'un profil.
pub fn lire_source(dossier_profil: &Path, source: &str) -> Option<CompteBoutique> {
    let v: Value = serde_json::from_slice(&std::fs::read(fichier(dossier_profil)).ok()?).ok()?;
    serde_json::from_value(v[source].clone()).ok()
}

pub fn ecrire_source(dossier_profil: &Path, source: &str, c: Option<&CompteBoutique>) -> Resultat<()> {
    modifier_source(dossier_profil, source, |_| Ok(c.cloned())).map(|_| ())
}

/// Un seul changement de `boutiques.json` à la fois (tous profils confondus : un verrou commun, plus simple et plus
/// sûr qu'un verrou par profil ; les écritures sont rares et courtes). La connexion à Steam (rappel de sa fenêtre) et
/// un import (qui attend le réseau) ne se défont donc plus l'un l'autre : chacun relit sous le verrou juste avant
/// d'écrire (revue de l'expert, 19/11).
static VERROU_BOUTIQUES: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Le contenu entier de `boutiques.json` : `{}` s'il n'existe pas encore ; une ERREUR s'il existe mais ne se lit pas
/// (il n'est alors jamais remplacé : les autres sources — Galaxy, RetroAchievements, Nexus — y vivent aussi).
fn lire_tout(f: &Path) -> Resultat<Value> {
    match std::fs::read(f) {
        Ok(o) => match serde_json::from_slice::<Value>(&o) {
            Ok(v) if v.is_object() => Ok(v),
            _ => Err(Erreur::Disque("Le fichier des comptes des boutiques de ce profil est illisible : Frogtend ne l'écrase pas.".into())),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(serde_json::json!({})),
        Err(e) => Err(e.into()),
    }
}

/// Change UNE source de `boutiques.json` sous le verrou : relit le fichier, passe l'état actuel de la source à `f`, et
/// écrit ce qu'elle rend (`None` = retirer la source) par un fichier provisoire renommé ensuite (une coupure en pleine
/// écriture ne laisse jamais un fichier à moitié écrit). Rend ce qui a été écrit.
pub fn modifier_source(
    dossier_profil: &Path,
    source: &str,
    f: impl FnOnce(Option<CompteBoutique>) -> Resultat<Option<CompteBoutique>>,
) -> Resultat<Option<CompteBoutique>> {
    let _verrou = VERROU_BOUTIQUES.lock().unwrap_or_else(|e| e.into_inner());
    let chemin = fichier(dossier_profil);
    let mut v = lire_tout(&chemin)?;
    // Une source présente mais illisible (écrite par une version plus récente, par exemple) n'est JAMAIS remplacée par
    // une source vide : ses comptes et ses fiches seraient perdus (revue du lot C4b).
    let avant = match &v[source] {
        Value::Null => None,
        brut => Some(serde_json::from_value(brut.clone()).map_err(|_| Erreur::Disque("Les comptes de cette boutique sont écrits sous une forme que cette version de Frogtend ne lit pas : elle ne les écrase pas (mets Frogtend à jour).".into()))?),
    };
    let apres = f(avant)?;
    match &apres {
        Some(c) => v[source] = serde_json::to_value(c).map_err(|_| Erreur::Disque("Compte de boutique impossible à écrire.".into()))?,
        None => {
            if let Some(o) = v.as_object_mut() {
                o.remove(source);
            }
        }
    }
    std::fs::create_dir_all(dossier_profil)?;
    let provisoire = dossier_profil.join("boutiques.json.provisoire");
    let octets = serde_json::to_vec_pretty(&v).map_err(|_| Erreur::Disque("Compte de boutique impossible à écrire.".into()))?;
    std::fs::write(&provisoire, octets)?;
    if let Err(e) = std::fs::rename(&provisoire, &chemin) {
        let _ = std::fs::remove_file(&provisoire);
        return Err(e.into());
    }
    Ok(apres)
}

pub const API_STEAM: &str = API;

/// **GOG Galaxy 2.0** (accord de Seb, 02/10 : lecture seule des lanceurs) : Galaxy regroupe les jeux de GOG ET des
/// boutiques que la personne y a reliées (Steam, Epic, Xbox, Ubisoft, EA…). Relevé dans sa base (02/10) :
/// `LibraryReleases(releaseKey « gog_123 », « epic_xxx »…)`, `ReleaseProperties(isDlc, isVisibleInLibrary)`,
/// `GamePieces(gamePieceTypeId → GamePieceTypes.type : originalTitle, title, originalImages{verticalCover…})`,
/// `GameTimes(minutesInGame)`, `LastPlayedDates(lastPlayedDate)`, `InstalledBaseProducts(productId)` (GOG),
/// `InstalledExternalProducts(productId)` (les autres). La base est TOUJOURS lue sur une copie.
pub fn dossier_galaxy() -> Option<PathBuf> {
    // Toujours sous les données communes de Windows (pas un dossier écrit en dur : `%ProgramData%`).
    let p = PathBuf::from(std::env::var_os("ProgramData")?).join("GOG.com").join("Galaxy").join("storage");
    p.join("galaxy-2.0.db").is_file().then_some(p)
}

/// Le libellé d'une boutique d'après le préfixe d'une clé Galaxy.
pub fn nom_plateforme(prefixe: &str) -> &'static str {
    match prefixe {
        "gog" => "GOG",
        "steam" => "Steam",
        "epic" => "Epic Games",
        "xboxone" | "xbox" => "Xbox",
        "uplay" => "Ubisoft Connect",
        "origin" | "ea" => "EA",
        "amazon" => "Amazon",
        "battlenet" => "Battle.net",
        _ => "Autre",
    }
}

/// Lit les jeux de GOG Galaxy (copie de la base dans `travail`).
pub fn lire_galaxy(storage: &Path, travail: &Path) -> Resultat<Vec<JeuBoutique>> {
    lire_galaxy_et_compte(storage, travail).map(|(l, _)| l)
}

/// Les comptes de la base de Galaxy qui ont des jeux (`LibraryReleases.userId`) : UN seul → c'est le compte ouvert dans
/// GOG Galaxy (lot C4b) ; plusieurs ou aucun → `None` (on n'affirme rien). Pur sur une connexion ouverte, testé.
fn compte_unique_galaxy(c: &rusqlite::Connection) -> rusqlite::Result<Option<String>> {
    let mut q = c.prepare("SELECT userId, COUNT(*) FROM LibraryReleases GROUP BY userId")?;
    let lignes: Vec<(Option<i64>, i64)> = q.query_map([], |r| Ok((r.get::<_, Option<i64>>(0)?, r.get::<_, i64>(1)?)))?.flatten().filter(|(_, n)| *n > 0).collect();
    // Des jeux sans compte (userId vide ou 0) : on ne sait pas à qui ils sont, rien n'est affirmé (et rien n'est filtré).
    if lignes.iter().any(|(u, _)| u.is_none_or(|u| u <= 0)) {
        return Ok(None);
    }
    let comptes: Vec<String> = lignes.iter().filter_map(|(u, _)| u.map(|u| u.to_string())).collect();
    Ok(if comptes.len() == 1 { comptes.into_iter().next() } else { None })
}

/// Lit les jeux de GOG Galaxy (copie de la base dans `travail`) et le compte qui y est ouvert, s'il est sûr (lot C4b :
/// la base peut garder les jeux de plusieurs comptes ; un seul compte → seuls SES jeux sont lus).
pub fn lire_galaxy_et_compte(storage: &Path, travail: &Path) -> Resultat<(Vec<JeuBoutique>, Option<String>)> {
    std::fs::create_dir_all(travail)?;
    let fichiers = ["galaxy-2.0.db", "galaxy-2.0.db-wal", "galaxy-2.0.db-shm"];
    // La place : la base et son journal, plus une marge (jamais un disque rempli par une copie de lecture).
    let besoin: u64 = fichiers.iter().filter_map(|f| std::fs::metadata(storage.join(f)).ok()).map(|m| m.len()).sum::<u64>() + (64 << 20);
    if crate::jeux_pc::place_libre(travail).is_some_and(|l| l < besoin) {
        return Err(Erreur::Disque("Pas assez de place sur le disque des données de Frogtend pour lire GOG Galaxy.".into()));
    }
    for f in fichiers {
        let src = storage.join(f);
        let dst = travail.join(f);
        let _ = std::fs::remove_file(&dst);
        if src.is_file() {
            std::fs::copy(&src, &dst)?;
        }
    }
    let lire = || -> rusqlite::Result<(Vec<JeuBoutique>, Option<String>)> {
        let c = rusqlite::Connection::open(travail.join("galaxy-2.0.db"))?;
        // Jamais un échec de la lecture des jeux à cause de cette question (comportement d'avant : tout est lu).
        let compte = compte_unique_galaxy(&c).ok().flatten();
        let piece = |t: &str| format!("(SELECT g.value FROM GamePieces g JOIN GamePieceTypes y ON y.id = g.gamePieceTypeId WHERE g.releaseKey = l.releaseKey AND y.type = '{t}' LIMIT 1)");
        // Un seul compte connu : ses jeux seulement (le numéro est un entier relu de la base, jamais un texte venu d'ailleurs).
        // Les temps de jeu et dernières parties aussi : ceux d'un ancien compte de Galaxy ne comptent pas.
        let u = compte.as_deref().and_then(|u| u.parse::<i64>().ok());
        // (Seulement si la table a bien la colonne : jamais une lecture qui casse sur une base d'une autre version.)
        let a_colonne = |table: &str| -> bool {
            c.query_row(&format!("SELECT COUNT(*) FROM pragma_table_info('{table}') WHERE name = 'userId'"), [], |r| r.get::<_, i64>(0)).is_ok_and(|n| n > 0)
        };
        let (filtre, filtre_t, filtre_d) = match u {
            Some(u) => (
                format!(" AND l.userId = {u}"),
                if a_colonne("GameTimes") { format!(" AND t.userId = {u}") } else { String::new() },
                if a_colonne("LastPlayedDates") { format!(" AND d.userId = {u}") } else { String::new() },
            ),
            None => Default::default(),
        };
        let sql = format!(
            "SELECT DISTINCT l.releaseKey, {}, {}, {},
                    (SELECT MAX(t.minutesInGame) FROM GameTimes t WHERE t.releaseKey = l.releaseKey{filtre_t}),
                    (SELECT MAX(d.lastPlayedDate) FROM LastPlayedDates d WHERE d.gameReleaseKey = l.releaseKey{filtre_d}),
                    {}
             FROM LibraryReleases l LEFT JOIN ReleaseProperties p ON p.releaseKey = l.releaseKey
             WHERE COALESCE(p.isDlc, 0) = 0 AND COALESCE(p.isVisibleInLibrary, 1) = 1{filtre}",
            piece("originalTitle"),
            piece("title"),
            piece("originalImages"),
            piece("allGameReleases")
        );
        // Les jeux GOG installés et leur dossier (`installationPath`, relevé le 20/10 : « D:\LaunchBox\Games\GOG
        // Galaxy\Games\Horizon Zero Dawn ») ; les jeux des autres boutiques n'ont AUCUN chemin dans Galaxy.
        let dossiers_gog: std::collections::HashMap<String, Option<String>> = c
            .prepare("SELECT productId, installationPath FROM InstalledBaseProducts")?
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?)))?
            .flatten()
            .map(|(id, p)| (format!("gog_{id}"), p.filter(|p| Path::new(p).is_dir())))
            .collect();
        let installes_autres: BTreeSet<String> =
            c.prepare("SELECT productId FROM InstalledExternalProducts")?.query_map([], |r| r.get::<_, String>(0))?.flatten().collect();
        // Les succès (table présente seulement si Galaxy en a synchronisé).
        let a_des_succes: bool = c
            .query_row("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'UserAchievements'", [], |r| r.get::<_, i64>(0))
            .map(|n| n > 0)?;
        let mut succes: std::collections::HashMap<String, (u32, u32)> = std::collections::HashMap::new();
        if a_des_succes {
            let mut q = c.prepare("SELECT gameReleaseKey, SUM(COALESCE(isUnlocked, 0)), COUNT(*) FROM UserAchievements GROUP BY gameReleaseKey")?;
            for r in q.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?)))?.flatten() {
                succes.insert(r.0, (r.1.max(0) as u32, r.2.max(0) as u32));
            }
        }
        let mut s = c.prepare(&sql)?;
        let l = s
            .query_map([], |r| {
                let cle: String = r.get(0)?;
                let titre = |v: Option<String>| v.and_then(|t| serde_json::from_str::<Value>(&t).ok()).and_then(|v| v["title"].as_str().map(String::from));
                let nom = titre(r.get(1)?).or(titre(r.get(2)?)).unwrap_or_else(|| cle.clone());
                let image = r.get::<_, Option<String>>(3)?.and_then(|t| serde_json::from_str::<Value>(&t).ok()).and_then(|v| v["verticalCover"].as_str().map(String::from));
                let minutes = r.get::<_, Option<i64>>(4)?.unwrap_or(0).max(0) as u64;
                let derniere = r.get::<_, Option<String>>(5)?.and_then(|d| date_en_secondes(&d)).unwrap_or(0);
                let (prefixe, id) = cle.split_once('_').unwrap_or(("", &cle));
                let installe = dossiers_gog.contains_key(&cle) || (prefixe != "gog" && installes_autres.contains(id));
                let dossier = dossiers_gog.get(&cle).cloned().flatten();
                let succes = succes.get(&cle).copied().filter(|(_, total)| *total > 0);
                // Les copies du même jeu (relevé du 21/10 : `{"releases":["psn_…","epic_HZDPP","gog_1209025141",…]}`), clés
                // sûres seulement.
                let releases: Vec<String> = r
                    .get::<_, Option<String>>(6)?
                    .and_then(|t| serde_json::from_str::<Value>(&t).ok())
                    .and_then(|v| v["releases"].as_array().cloned())
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|x| x.as_str())
                    .filter(|x| x.len() <= 128 && x.contains('_') && x.chars().all(|c| c.is_ascii_graphic()))
                    .map(String::from)
                    .collect();
                Ok(JeuBoutique { id: cle.clone(), nom, minutes, derniere, installe, plateforme: prefixe.to_string(), image, dossier, pret: None, succes, releases, titre_douteux: false })
            })?
            .flatten()
            .collect::<Vec<_>>();
        Ok((l, compte))
    };
    let (mut l, compte) = lire().map_err(|e| Erreur::Disque(format!("La base de GOG Galaxy est illisible ({e}).")))?;
    l.sort_by(|a, b| a.nom.to_lowercase().cmp(&b.nom.to_lowercase()));
    Ok((l, compte))
}

/// Les succès en détail d'UN jeu de GOG Galaxy (`cle` : sa clé Galaxy, « gog_… », « steam_620 »…), en LECTURE SEULE
/// sur une copie de sa base (effacée ensuite : elle contient les données du compte). `None` : Galaxy n'en a pas.
/// Relevé réel (21/10) : textes en français notés « non traduits » (`isLocalized` 0, ils sont en anglais) → le
/// français seulement s'il est vraiment traduit, sinon l'anglais (dit) ; aucun succès caché noté (`isVisible`
/// toujours 1) ; rareté en % ; icônes sur images.gog.com ; date « AAAA-MM-JJ hh:mm:ss ».
pub fn succes_galaxy(storage: &Path, travail: &Path, cle: &str) -> Resultat<Option<Vec<crate::succes::SuccesDetail>>> {
    if cle.is_empty() || cle.len() > 128 || !cle.chars().all(|c| c.is_ascii_alphanumeric() || "_-.".contains(c)) {
        return Ok(None);
    }
    // La copie part dans TOUS les cas (même une copie à moitié faite : elle contient les données du compte).
    struct Efface<'a>(&'a Path);
    impl Drop for Efface<'_> {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(self.0);
        }
    }
    let _efface = Efface(travail);
    let fichiers = ["galaxy-2.0.db", "galaxy-2.0.db-wal", "galaxy-2.0.db-shm"];
    // La place : la base et son journal, plus une marge (jamais un disque rempli par une copie de lecture).
    let besoin: u64 = fichiers.iter().filter_map(|f| std::fs::metadata(storage.join(f)).ok()).map(|m| m.len()).sum::<u64>() + (64 << 20);
    std::fs::create_dir_all(travail)?;
    if crate::jeux_pc::place_libre(travail).is_some_and(|l| l < besoin) {
        return Err(Erreur::Disque("Pas assez de place sur le disque des données de Frogtend pour lire les succès de GOG Galaxy.".into()));
    }
    for f in fichiers {
        let (src, dst) = (storage.join(f), travail.join(f));
        if src.is_file() {
            std::fs::copy(&src, &dst)?;
        }
    }
    let lire = || -> rusqlite::Result<Option<Vec<crate::succes::SuccesDetail>>> {
        let c = rusqlite::Connection::open(travail.join("galaxy-2.0.db"))?;
        let tables: i64 = c.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('Achievements', 'UserAchievements', 'LocalizedAchievements', 'Languages')",
            [],
            |r| r.get(0),
        )?;
        if tables < 4 {
            return Ok(None);
        }
        // Le texte : français vraiment traduit, sinon anglais (en-US), sinon n'importe lequel.
        let sql = "SELECT a.apikey, a.imageUnlockedUrl, a.imageLockedUrl, a.isVisible, a.rarity,
                    COALESCE(u.isUnlocked, 0), u.unlockTime,
                    (SELECT l.name || char(31) || COALESCE(l.description, '') || char(31) ||
                            (CASE WHEN g.code LIKE 'fr%' AND l.isLocalized = 1 THEN 'fr' ELSE 'autre' END)
                       FROM LocalizedAchievements l LEFT JOIN Languages g ON g.id = l.languageId
                      WHERE l.gameReleaseKey = a.gameReleaseKey AND l.apikey = a.apikey
                      ORDER BY (CASE WHEN g.code LIKE 'fr%' AND l.isLocalized = 1 THEN 0 WHEN g.code = 'en-US' THEN 1 ELSE 2 END)
                      LIMIT 1)
             FROM Achievements a
             LEFT JOIN UserAchievements u ON u.gameReleaseKey = a.gameReleaseKey AND u.apikey = a.apikey
            WHERE a.gameReleaseKey = ?1";
        let mut q = c.prepare(sql)?;
        let l: Vec<crate::succes::SuccesDetail> = q
            .query_map([cle], |r| {
                let texte: Option<String> = r.get(7)?;
                let mut t = texte.as_deref().unwrap_or("").split('\u{1f}');
                let (nom, description, langue) = (t.next().unwrap_or("").to_string(), t.next().unwrap_or("").to_string(), t.next().unwrap_or("autre"));
                let image = |v: Option<String>| v.filter(|u| image_autorisee(u));
                Ok(crate::succes::SuccesDetail {
                    id: r.get(0)?,
                    nom,
                    description,
                    en_anglais: langue != "fr",
                    obtenu: r.get::<_, Option<i64>>(5)?.unwrap_or(0) == 1,
                    quand: r.get::<_, Option<String>>(6)?.and_then(|d| date_en_secondes(&d)),
                    rarete: r.get::<_, Option<f64>>(4)?.filter(|x| x.is_finite() && *x >= 0.0 && *x <= 100.0),
                    icone: image(r.get(1)?),
                    icone_grise: image(r.get(2)?),
                    secret: r.get::<_, Option<i64>>(3)?.unwrap_or(1) == 0,
                })
            })?
            .flatten()
            .filter(|s| !s.nom.is_empty())
            .collect();
        Ok((!l.is_empty()).then_some(l))
    };
    // La connexion est fermée à la fin de `lire` ; `_efface` retire ensuite la copie.
    lire().map_err(|e| Erreur::Disque(format!("La base de GOG Galaxy est illisible ({e}).")))
}

/// La fiche d'un jeu de boutique (lot D, Seb 21/10 : prise DANS les boutiques) : la boutique Steam (en français) ou
/// GOG Galaxy (sa base sur ce PC, en anglais).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FicheBoutique {
    /// `steam` ou `galaxy`.
    pub source: String,
    pub resume: Option<String>,
    /// Le résumé n'existe qu'en anglais.
    pub en_anglais: bool,
    pub genres: Vec<String>,
    pub developpeurs: Vec<String>,
    pub editeurs: Vec<String>,
    pub annee: Option<i32>,
    /// La date de sortie telle que la boutique l'écrit (« 4 févr. 2016 »).
    pub sortie: Option<String>,
    pub fond: Option<String>,
    /// Les captures (miniature, grande), adresses publiques autorisées.
    pub captures: Vec<(String, String)>,
}

/// Quelques entités HTML d'un texte de boutique, rendues en clair (jamais de balise interprétée).
fn texte_clair(t: &str) -> String {
    t.replace("&quot;", "\"").replace("&#39;", "'").replace("&apos;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&nbsp;", " ").replace("&amp;", "&").trim().to_string()
}

/// La fiche d'un jeu d'après la réponse de la boutique Steam (`appdetails`, relevé réel XCOM 2 du 21/10).
pub fn lire_fiche_steam(v: &Value, appid: &str) -> Option<FicheBoutique> {
    let d = &v[appid];
    if d["success"].as_bool() != Some(true) {
        return None;
    }
    let x = &d["data"];
    let liste = |cle: &str| x[cle].as_array().map(|l| l.iter().filter_map(|s| s.as_str().map(String::from)).collect()).unwrap_or_default();
    let image = |u: &str| Some(u.to_string()).filter(|u| image_autorisee(u));
    let sortie = x["release_date"]["date"].as_str().filter(|s| !s.is_empty()).map(String::from);
    Some(FicheBoutique {
        source: "steam".into(),
        resume: x["short_description"].as_str().map(texte_clair).filter(|t| !t.is_empty()),
        // Un jeu sans français : Steam rend son texte anglais (la liste de ses langues, écrite en français, le dit).
        en_anglais: !x["supported_languages"].as_str().is_some_and(|l| l.contains("Français")),
        genres: x["genres"].as_array().map(|l| l.iter().filter_map(|g| g["description"].as_str().map(String::from)).collect()).unwrap_or_default(),
        developpeurs: liste("developers"),
        editeurs: liste("publishers"),
        annee: sortie.as_deref().and_then(|s| s.split(|c: char| !c.is_ascii_digit()).filter(|n| n.len() == 4).last()).and_then(|a| a.parse().ok()),
        sortie,
        fond: x["background_raw"].as_str().and_then(image),
        captures: x["screenshots"]
            .as_array()
            .map(|l| l.iter().filter_map(|s| Some((image(s["path_thumbnail"].as_str()?)?, image(s["path_full"].as_str()?)?))).take(20).collect())
            .unwrap_or_default(),
    })
}

/// La fiche FRANÇAISE d'un jeu Steam, par l'adresse publique de la boutique (sans clé), gardée une semaine dans
/// `cache` (données publiques, communes au PC). Hors ligne : l'ancienne fiche ; `None` : la boutique n'en a pas ;
/// `Err(Reseau)` : Steam n'a pas répondu (ou refuse un moment) et aucune fiche n'est gardée.
pub async fn fiche_steam(cache: &Path, appid: &str) -> Resultat<Option<FicheBoutique>> {
    if appid.is_empty() || appid.len() > 12 || !appid.chars().all(|c| c.is_ascii_digit()) {
        return Ok(None);
    }
    const SEMAINE: u64 = 7 * 24 * 3600;
    let f = cache.join(format!("steam-fiche-{appid}.json"));
    let recente = std::fs::metadata(&f).ok().and_then(|m| m.modified().ok()).and_then(|t| t.elapsed().ok()).is_some_and(|d| d.as_secs() < SEMAINE);
    let lire_cache = || std::fs::read(&f).ok().and_then(|o| serde_json::from_slice::<Value>(&o).ok());
    if recente {
        if let Some(v) = lire_cache() {
            return Ok(lire_fiche_steam(&v, appid));
        }
    }
    let r = async {
        reqwest::Client::builder()
            .user_agent("Frogtend")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .ok()?
            .get("https://store.steampowered.com/api/appdetails")
            .query(&[("appids", appid), ("l", "french"), ("cc", "fr")])
            .send()
            .await
            .ok()?
            .json::<Value>()
            .await
            .ok()
    }
    .await;
    match r {
        Some(v) if v[appid].is_object() => {
            std::fs::create_dir_all(cache)?;
            let _ = std::fs::write(&f, serde_json::to_vec(&v).unwrap_or_default());
            Ok(lire_fiche_steam(&v, appid))
        }
        // Hors ligne (ou la boutique refuse un moment) : l'ancienne fiche vaut mieux que rien ; sinon, dit.
        _ => match lire_cache() {
            Some(v) => Ok(lire_fiche_steam(&v, appid)),
            None => Err(Erreur::Reseau("La boutique Steam ne répond pas pour l'instant.".into())),
        },
    }
}

/// La fiche d'un jeu d'après GOG Galaxy (lecture seule, sur une copie de sa base effacée ensuite) : `summary`,
/// `meta` (genres, développeurs, éditeurs, date de sortie en secondes). Relevé du 21/10 : textes en anglais. Ses
/// images (`media`) ne sont pas reprises : leurs adresses à gabarit ne répondent pas aujourd'hui (à revoir).
pub fn fiche_galaxy(storage: &Path, travail: &Path, cle: &str) -> Resultat<Option<FicheBoutique>> {
    if cle.is_empty() || cle.len() > 128 || !cle.chars().all(|c| c.is_ascii_alphanumeric() || "_-.".contains(c)) {
        return Ok(None);
    }
    struct Efface<'a>(&'a Path);
    impl Drop for Efface<'_> {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(self.0);
        }
    }
    let _efface = Efface(travail);
    let fichiers = ["galaxy-2.0.db", "galaxy-2.0.db-wal", "galaxy-2.0.db-shm"];
    let besoin: u64 = fichiers.iter().filter_map(|f| std::fs::metadata(storage.join(f)).ok()).map(|m| m.len()).sum::<u64>() + (64 << 20);
    std::fs::create_dir_all(travail)?;
    if crate::jeux_pc::place_libre(travail).is_some_and(|l| l < besoin) {
        return Err(Erreur::Disque("Pas assez de place sur le disque des données de Frogtend pour lire GOG Galaxy.".into()));
    }
    for f in fichiers {
        if storage.join(f).is_file() {
            std::fs::copy(storage.join(f), travail.join(f))?;
        }
    }
    let lire = || -> rusqlite::Result<Option<FicheBoutique>> {
        let c = rusqlite::Connection::open(travail.join("galaxy-2.0.db"))?;
        let piece = |t: &str| -> rusqlite::Result<Option<Value>> {
            Ok(c.query_row(
                "SELECT g.value FROM GamePieces g JOIN GamePieceTypes y ON y.id = g.gamePieceTypeId WHERE g.releaseKey = ?1 AND y.type = ?2 LIMIT 1",
                rusqlite::params![cle, t],
                |r| r.get::<_, String>(0),
            )
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok()))
        };
        let (resume, meta) = (piece("summary")?, piece("meta")?);
        if resume.is_none() && meta.is_none() {
            return Ok(None);
        }
        let meta = meta.unwrap_or(Value::Null);
        let liste = |cle: &str| meta[cle].as_array().map(|l| l.iter().filter_map(|s| s.as_str().map(String::from)).collect()).unwrap_or_default();
        let annee = meta["releaseDate"].as_i64().or_else(|| meta["releaseDate"].as_str().and_then(|s| s.parse().ok())).filter(|s| *s > 0).map(|s| 1970 + (s / 31_556_952) as i32);
        Ok(Some(FicheBoutique {
            source: "galaxy".into(),
            resume: resume.and_then(|r| r["summary"].as_str().map(texte_clair)).filter(|t| !t.is_empty()),
            en_anglais: true,
            genres: liste("genres"),
            developpeurs: liste("developers"),
            editeurs: liste("publishers"),
            annee,
            sortie: None,
            fond: None,
            captures: vec![],
        }))
    };
    lire().map_err(|e| Erreur::Disque(format!("La base de GOG Galaxy est illisible ({e}).")))
}

/// Le dossier des données de l'Epic Games Launcher, tel qu'Epic l'écrit dans le registre (`HKLM\SOFTWARE\Epic
/// Games\EpicGamesLauncher`, valeur `AppDataPath` ; relevé du 18/11 : `C:\ProgramData\Epic\EpicGamesLauncher\Data\`).
/// Jamais un chemin supposé.
pub fn dossier_donnees_epic() -> Option<PathBuf> {
    let v = crate::registre::valeurs(crate::registre::Racine::Machine, r"SOFTWARE\Epic Games\EpicGamesLauncher")?;
    let p = PathBuf::from(v.get("AppDataPath")?.trim().trim_end_matches(['\\', '/']));
    (!p.as_os_str().is_empty() && p.is_dir()).then_some(p)
}

/// Le dossier des manifestes de l'Epic Games Launcher (`<AppDataPath>\Manifests`).
pub fn dossier_manifestes_epic() -> Option<PathBuf> {
    let d = dossier_donnees_epic()?.join("Manifests");
    d.is_dir().then_some(d)
}

/// `LauncherInstalled.dat`, la liste des installations que tient le lanceur, à côté de ses données
/// (`<…>\Epic\EpicGamesLauncher\Data` → `<…>\Epic\UnrealEngineLauncher\LauncherInstalled.dat`, relevé du 18/11).
pub fn liste_installations_epic(donnees: &Path) -> Option<PathBuf> {
    let f = donnees.parent()?.parent()?.join("UnrealEngineLauncher").join("LauncherInstalled.dat");
    f.is_file().then_some(f)
}

/// Les dossiers que `LauncherInstalled.dat` donne (AppName → InstallLocation). Le manifeste d'un jeu peut garder un
/// ancien dossier (jeu déplacé) : cette liste sert alors de repli (méthode de Playnite, relue le 18/11).
pub fn lire_installations_epic(texte: &str) -> std::collections::BTreeMap<String, String> {
    let v: Value = serde_json::from_str(texte).unwrap_or(Value::Null);
    v["InstallationList"]
        .as_array()
        .map(|l| {
            l.iter()
                .filter_map(|a| Some((a["AppName"].as_str().filter(|x| id_epic_sur(x))?.to_string(), a["InstallLocation"].as_str().filter(|x| !x.trim().is_empty())?.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

/// Le repli de `LauncherInstalled.dat` à côté des données `donnees` (vide s'il manque ou s'il est illisible).
pub fn repli_epic(donnees: &Path) -> std::collections::BTreeMap<String, String> {
    liste_installations_epic(donnees)
        .filter(|f| std::fs::metadata(f).is_ok_and(|m| m.len() <= 4 * 1024 * 1024))
        .and_then(|f| std::fs::read(f).ok())
        .map(|o| lire_installations_epic(&String::from_utf8_lossy(&o)))
        .unwrap_or_default()
}

/// Les jeux Epic installés sur CE PC, d'après le registre (AppDataPath), ses manifestes et `LauncherInstalled.dat` ;
/// `None` : le lanceur n'est pas là, ou ses manifestes ne se lisent pas (jamais « zéro jeu » dans ce cas).
pub fn installes_epic_du_pc() -> Option<Vec<JeuEpic>> {
    let donnees = dossier_donnees_epic()?;
    let manifestes = donnees.join("Manifests");
    std::fs::read_dir(&manifestes).ok()?;
    Some(installes_epic(&manifestes, &repli_epic(&donnees)))
}

/// Un jeu installé par l'Epic Games Launcher (son manifeste `.item`, lu en LECTURE SEULE) : (AppName, namespace,
/// item, nom, dossier). Relevé réel du 21/10 (TerraScape) ; seulement un JEU (`AppCategories` « games »), installé en
/// entier, avec un programme, qui n'est pas un DLC, et dont le dossier existe — jamais Unreal Engine ni un plugin.
#[derive(Debug, Clone, PartialEq)]
pub struct JeuEpic {
    pub app: String,
    pub namespace: String,
    pub item: String,
    pub nom: String,
    pub dossier: PathBuf,
}

/// Un identifiant Epic sûr (lettres, chiffres, quelques signes ; jamais un séparateur de chemin ou d'adresse).
pub(crate) fn id_epic_sur(s: &str) -> bool {
    !s.is_empty() && s.len() <= 128 && s.chars().all(|c| c.is_ascii_alphanumeric() || "_-.".contains(c))
}

pub fn lire_manifeste_epic(texte: &str) -> Option<JeuEpic> {
    lire_manifeste_epic_avec(texte, &Default::default())
}

/// `lire_manifeste_epic`, avec le repli de `LauncherInstalled.dat` quand le dossier du manifeste n'existe plus.
pub fn lire_manifeste_epic_avec(texte: &str, repli: &std::collections::BTreeMap<String, String>) -> Option<JeuEpic> {
    let v: Value = serde_json::from_str(texte).ok()?;
    let app = v["AppName"].as_str().filter(|a| id_epic_sur(a))?;
    let jeu = v["AppCategories"].as_array().is_some_and(|l| l.iter().any(|c| c.as_str() == Some("games")));
    let dlc = v["MainGameAppName"].as_str().is_some_and(|m| !m.is_empty() && m != app);
    if !jeu || dlc || v["bIsIncompleteInstall"].as_bool() != Some(false) || v["LaunchExecutable"].as_str().unwrap_or("").is_empty() {
        return None;
    }
    let manifeste = v["InstallLocation"].as_str().filter(|d| !d.is_empty()).map(PathBuf::from);
    let dossier = match manifeste {
        Some(d) if d.is_dir() => d,
        autre => repli.get(app).map(PathBuf::from).filter(|d| d.is_dir()).or(autre)?,
    };
    Some(JeuEpic {
        app: app.to_string(),
        namespace: v["CatalogNamespace"].as_str().filter(|a| id_epic_sur(a))?.to_string(),
        item: v["CatalogItemId"].as_str().filter(|a| id_epic_sur(a))?.to_string(),
        nom: v["DisplayName"].as_str().filter(|n| !n.trim().is_empty()).unwrap_or(app).to_string(),
        dossier,
    })
}

/// Les jeux Epic installés sur CE PC (dossier présent). `repli` : `lire_installations_epic`.
pub fn installes_epic(manifestes: &Path, repli: &std::collections::BTreeMap<String, String>) -> Vec<JeuEpic> {
    let mut l: Vec<JeuEpic> = std::fs::read_dir(manifestes)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("item")) && e.metadata().is_ok_and(|m| m.is_file() && m.len() <= 4 * 1024 * 1024))
        .filter_map(|e| std::fs::read(e.path()).ok().and_then(|o| lire_manifeste_epic_avec(&String::from_utf8_lossy(&o), repli)))
        .filter(|j| j.dossier.is_dir())
        .collect();
    l.sort_by(|a, b| a.nom.to_lowercase().cmp(&b.nom.to_lowercase()));
    l.dedup_by(|a, b| a.app == b.app);
    l
}

/// L'adresse qui demande à l'Epic Games Launcher de lancer ce jeu, telle qu'Epic l'écrit dans ses propres raccourcis
/// (relevé du 21/10 : `com.epicgames.launcher://apps/<namespace>%3A<item>%3A<app>?action=launch&silent=true`).
pub fn adresse_epic(j: &JeuEpic) -> String {
    format!("com.epicgames.launcher://apps/{}%3A{}%3A{}?action=launch&silent=true", j.namespace, j.item, j.app)
}

/// « 2022-04-29 20:23:22 » → secondes depuis 1970 (UTC, sans les secondes intercalaires).
pub(crate) fn date_en_secondes(d: &str) -> Option<u64> {
    let (date, heure) = d.split_once(' ').unwrap_or((d, "00:00:00"));
    let n: Vec<i64> = date.split('-').filter_map(|x| x.parse().ok()).collect();
    let h: Vec<i64> = heure.split(':').filter_map(|x| x.parse().ok()).collect();
    if n.len() != 3 {
        return None;
    }
    let (mut a, m, j) = (n[0], n[1], n[2]);
    // Jours depuis 1970 (algorithme civil de H. Hinnant).
    if m <= 2 {
        a -= 1;
    }
    let ere = a.div_euclid(400);
    let ya = a - ere * 400;
    let mp = (m + 9) % 12;
    let jour_an = (153 * mp + 2) / 5 + j - 1;
    let jour_ere = ya * 365 + ya / 4 - ya / 100 + jour_an;
    let jours = ere * 146_097 + jour_ere - 719_468;
    let s = jours * 86_400 + h.first().unwrap_or(&0) * 3600 + h.get(1).unwrap_or(&0) * 60 + h.get(2).unwrap_or(&0);
    (s >= 0).then_some(s as u64)
}

/// Les hôtes d'images de boutiques autorisés (images publiques).
pub fn image_autorisee(url: &str) -> bool {
    url.starts_with("https://images.gog.com/")
        || url.starts_with("https://cdn.akamai.steamstatic.com/")
        || url.starts_with("https://cdn1.epicgames.com/")
        // Les jaquettes des jeux Ubisoft possédés (lot B de l'import automatique, CDN public du lanceur).
        || url.starts_with("https://ubistatic3-a.akamaihd.net/orbit/uplay_launcher_3_0/assets/")
        // Les images de la fiche d'un jeu Steam (lot D : fond, captures ; relevé du 21/10).
        || url.starts_with("https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/")
        // Les badges des succès de RetroAchievements (lot C3).
        || url.starts_with("https://media.retroachievements.org/Badge/")
        // Les jaquettes des jeux du compte GOG (lot C4a), seulement sous la forme rendue par `image_gog`.
        || image_gog_permise(url)
}

/// Les hôtes d'images de la liste des jeux du compte GOG (« images-1.gog-statics.com »… ; lot C4a, forme supposée, pas
/// vérifiée sur un vrai compte) : un hôte EXACT de cette liste, jamais « gog-statics.com » au milieu d'un autre nom.
const HOTES_IMAGES_GOG: [&str; 5] = ["images.gog-statics.com", "images-1.gog-statics.com", "images-2.gog-statics.com", "images-3.gog-statics.com", "images-4.gog-statics.com"];

/// Le suffixe ajouté à l'empreinte d'une image GOG sans extension : la vignette de 392 pixels de large que la page
/// « Mes jeux » de gog.com utilise (forme supposée, pas vérifiée ; une image absente laisse le titre sur la carte).
pub const SUFFIXE_IMAGE_GOG: &str = "_392.jpg";

/// Une adresse d'image GOG telle que `image_gog` la rend : https, hôte exact, une empreinte (lettres et chiffres) puis
/// au plus un suffixe sûr (« _392.jpg », « .png »…), rien d'autre (ni dossier, ni paramètre).
fn image_gog_permise(url: &str) -> bool {
    let Some(reste) = url.strip_prefix("https://") else { return false };
    let Some((hote, chemin)) = reste.split_once('/') else { return false };
    HOTES_IMAGES_GOG.contains(&hote) && fichier_image_gog(chemin)
}

/// `<empreinte>[_<forme>].<jpg|png|webp>` : l'empreinte de 16 à 128 lettres et chiffres.
fn fichier_image_gog(f: &str) -> bool {
    let Some((nom, ext)) = f.rsplit_once('.') else { return false };
    let (empreinte, forme) = nom.split_once('_').unwrap_or((nom, ""));
    ["jpg", "png", "webp"].contains(&ext)
        && (16..=128).contains(&empreinte.len())
        && empreinte.bytes().all(|c| c.is_ascii_alphanumeric())
        && forme.len() <= 40
        && forme.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
}

/// 0.219.1 : les cartes GOG à retirer (id de carte) — les numéros dont la fiche en cache est classée PAS un jeu
/// (`raison_ecart`) et qui ne sont pas dans la liste gardée `gardes` (pur, testé). Une fiche introuvable ou en échec
/// n'est jamais visée.
pub fn cartes_gog_ecartees(c: &CompteBoutique, gardes: &[JeuBoutique]) -> BTreeSet<i64> {
    let gardes: BTreeSet<&str> = gardes.iter().filter_map(|j| j.id.strip_prefix("gog_")).collect();
    c.fiches_gog
        .iter()
        .filter(|(id, f)| !gardes.contains(id.as_str()) && crate::gog_galaxy::raison_ecart(f).is_some())
        .map(|(id, _)| crate::ludotheque::id_boutique("gog", id))
        .collect()
}

/// Normalise l'image d'un jeu du compte GOG (lot C4a) : « //images-1.gog-statics.com/<empreinte> » (forme supposée)
/// devient « https://images-1.gog-statics.com/<empreinte>_392.jpg » ; une adresse qui a déjà son extension est gardée
/// si elle est sûre. Une forme inconnue : `None` (une carte sans image plutôt qu'une adresse non autorisée).
pub fn image_gog(brut: &str) -> Option<String> {
    let b = brut.trim();
    let reste = b.strip_prefix("https://").or_else(|| b.strip_prefix("//"))?;
    let (hote, chemin) = reste.split_once('/')?;
    let hote = hote.to_ascii_lowercase();
    if !HOTES_IMAGES_GOG.contains(&hote.as_str()) {
        return None;
    }
    let url = if chemin.contains('.') { format!("https://{hote}/{chemin}") } else { format!("https://{hote}/{chemin}{SUFFIXE_IMAGE_GOG}") };
    image_gog_permise(&url).then_some(url)
}

/// Les jeux du compte GOG remis par la page (lot C4a) mis en forme pour la ludothèque : `gog_<id>` (le même id de carte
/// que GOG Galaxy et le registre : jamais deux cartes pour un jeu), titre sans ™, image normalisée, « pas installé »
/// (l'état installé vient du PC, `boutiques_pc::fusion_gog`).
pub fn jeux_du_compte_gog(lus: &[crate::gratuits::JeuGogLu]) -> Vec<JeuBoutique> {
    let mut vus = BTreeSet::new();
    lus.iter()
        .filter(|j| crate::gratuits::numero_gog(&j.id) && vus.insert(j.id.clone()))
        .map(|j| {
            let nom = crate::boutiques_pc::sans_marques(&j.titre);
            JeuBoutique {
                id: format!("gog_{}", j.id),
                nom: if nom.is_empty() { j.id.clone() } else { nom },
                minutes: 0,
                derniere: 0,
                installe: false,
                plateforme: "gog".into(),
                image: image_gog(&j.image),
                dossier: None,
                pret: None,
                succes: None,
                releases: vec![],
                titre_douteux: false,
            }
        })
        .collect()
}

/// Le refus d'ajouter un compte GOG déjà dans Frogtend (lot C4b).
pub const MOTIF_GOG_DEJA_LA: &str = "Ce compte GOG est déjà dans Frogtend.";

/// Ce qu'une lecture a donné pour UN compte GOG (lot C4b).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LectureCompteGog {
    pub id_site: String,
    pub id_galaxy: String,
    /// Le pseudo lu (`None` : pas lu cette fois, l'ancien reste).
    pub nom: Option<String>,
    pub jeux: Vec<JeuBoutique>,
    /// Les numéros de la bibliothèque de Galaxy lue (`None` : pas lue en entier, les anciens restent).
    pub ids_lus: Option<Vec<String>>,
    /// Changement de compte du navigateur du profil : le numéro Galaxy de l'ANCIEN compte `Profil`, retrouvé par son
    /// jeton (un compte repris d'un fichier 0.218 ne l'avait pas) ; il lui est donné s'il n'en avait pas.
    pub ancien_id_galaxy: Option<String>,
}

/// Ce que la lecture du compte `Profil` a changé dans la liste des comptes (lot C4b).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChangementGog {
    /// La session gog.com du navigateur du profil montre un AUTRE compte que le compte `Profil` rangé : l'ancien
    /// (numéro Galaxy, possiblement vide) est devenu « ajouté » (jamais retiré sans demande).
    pub ancien_profil: Option<String>,
    /// Un compte « ajouté » était ce compte-là : il est fondu dans l'entrée `Profil` ; son secret (`#<numéro>`) est à
    /// oublier APRÈS l'écriture (un seul secret par compte).
    pub fondu: Option<String>,
}

fn appliquer_lecture(x: &mut CompteGog, lu: &LectureCompteGog, quand: &str) {
    if !lu.id_galaxy.is_empty() {
        x.id_galaxy = lu.id_galaxy.clone();
    }
    if let Some(n) = &lu.nom {
        x.nom = n.clone();
    }
    if let Some(i) = &lu.ids_lus {
        x.ids_lus = i.clone();
    }
    x.jeux = lu.jeux.clone();
    x.maj_le = quand.to_string();
    x.connecte_le = quand.to_string();
}

/// Range la lecture du compte `Profil` (celui de la session gog.com du navigateur du profil) dans la source « gog »
/// (pur, appelé sous le verrou par `modifier_source`, testé). `avant` : le numéro du site du compte `Profil` rangé AVANT
/// la lecture ; s'il a changé entre-temps, refus (rien n'est mélangé). Seuls les jeux DE CE COMPTE changent ; les autres
/// comptes ne bougent pas. Un autre compte que le compte `Profil` rangé : l'ancien devient « ajouté » (gardé avec ses
/// jeux) ; un compte « ajouté » qui est ce compte-là est fondu dans l'entrée `Profil`.
pub fn appliquer_import_gog_profil(c: Option<CompteBoutique>, avant: &str, lu: &LectureCompteGog, quand: &str) -> Resultat<(CompteBoutique, ChangementGog)> {
    if !crate::gratuits::numero_gog(&lu.id_site) || !(lu.id_galaxy.is_empty() || crate::gratuits::numero_gog(&lu.id_galaxy)) {
        return Err(Erreur::Refus("Compte GOG illisible.".into()));
    }
    let mut c = c.unwrap_or_default();
    let mut comptes = comptes_gog_lus(&c);
    let ip = comptes.iter().position(|x| x.navigateur == NavigateurGog::Profil);
    let range = ip.map(|i| comptes[i].id_site.clone()).unwrap_or_default();
    if range != avant {
        return Err(Erreur::Refus("Le compte GOG a changé pendant la lecture : relance-la.".into()));
    }
    let mut ch = ChangementGog::default();
    // Un compte « ajouté » qui est ce même compte : même numéro Galaxy, ou (ancien compte `Profil` d'un fichier 0.218,
    // au numéro Galaxy inconnu) même numéro du site : fondu.
    let doublon = comptes.iter().position(|x| {
        x.navigateur == NavigateurGog::Ajoute && ((!lu.id_galaxy.is_empty() && x.id_galaxy == lu.id_galaxy) || (x.id_galaxy.is_empty() && !x.id_site.is_empty() && x.id_site == lu.id_site))
    });
    let mut fondu: Option<CompteGog> = None;
    if let Some(d) = doublon {
        let x = comptes.remove(d);
        ch.fondu = Some(x.id_galaxy.clone()).filter(|i| !i.is_empty());
        fondu = Some(x);
    }
    c.navigateur_gog_oublie = false;
    let ip = comptes.iter().position(|x| x.navigateur == NavigateurGog::Profil);
    match ip {
        Some(i) if comptes[i].id_site == lu.id_site => appliquer_lecture(&mut comptes[i], lu, quand),
        _ => {
            if let Some(i) = ip {
                comptes[i].navigateur = NavigateurGog::Ajoute;
                if comptes[i].id_galaxy.is_empty() {
                    if let Some(a) = lu.ancien_id_galaxy.as_ref().filter(|a| crate::gratuits::numero_gog(a) && **a != lu.id_galaxy) {
                        comptes[i].id_galaxy = a.clone();
                    }
                }
                ch.ancien_profil = Some(comptes[i].id_galaxy.clone());
            }
            let mut x = fondu.unwrap_or_default();
            x.navigateur = NavigateurGog::Profil;
            x.id_site = lu.id_site.clone();
            appliquer_lecture(&mut x, lu, quand);
            comptes.insert(0, x);
        }
    }
    poser_comptes_gog(&mut c, comptes);
    Ok((c, ch))
}

/// Range la lecture d'un compte « ajouté » (pur, testé) : seuls SES jeux changent. Un compte oublié pendant la lecture :
/// refus (rien n'est écrit).
pub fn appliquer_import_gog_ajoute(c: Option<CompteBoutique>, lu: &LectureCompteGog, quand: &str) -> Resultat<CompteBoutique> {
    let mut c = c.unwrap_or_default();
    let mut comptes = comptes_gog_lus(&c);
    let x = comptes
        .iter_mut()
        .find(|x| x.navigateur == NavigateurGog::Ajoute && !x.id_galaxy.is_empty() && x.id_galaxy == lu.id_galaxy)
        .ok_or_else(|| Erreur::Refus("Ce compte GOG a été oublié pendant la lecture : rien n'est changé.".into()))?;
    appliquer_lecture(x, lu, quand);
    poser_comptes_gog(&mut c, comptes);
    Ok(c)
}

/// Ajoute un compte GOG (pur, testé). Déjà là : refus (« Ce compte GOG est déjà dans Frogtend »), sauf un compte
/// « ajouté » dont la connexion était perdue (`reconnexion` : aucun secret au coffre), qui est alors relu.
pub fn ajouter_compte_gog(c: Option<CompteBoutique>, lu: &LectureCompteGog, reconnexion: bool, quand: &str) -> Resultat<CompteBoutique> {
    if !crate::gratuits::numero_gog(&lu.id_galaxy) {
        return Err(Erreur::Refus("Compte GOG illisible.".into()));
    }
    let mut c = c.unwrap_or_default();
    let mut comptes = comptes_gog_lus(&c);
    match comptes.iter_mut().find(|x| x.id_galaxy == lu.id_galaxy) {
        Some(x) if x.navigateur == NavigateurGog::Ajoute && reconnexion => appliquer_lecture(x, lu, quand),
        Some(_) => return Err(Erreur::Refus(MOTIF_GOG_DEJA_LA.into())),
        None => {
            let mut x = CompteGog { navigateur: NavigateurGog::Ajoute, ..Default::default() };
            appliquer_lecture(&mut x, lu, quand);
            comptes.push(x);
        }
    }
    poser_comptes_gog(&mut c, comptes);
    Ok(c)
}

/// Retire un compte GOG de la liste (pur, testé) : rend la source écrite et le compte retiré. Aucune carte de la
/// ludothèque n'est touchée (comme « Oublier » de Steam) ; le cache public des fiches reste.
pub fn oublier_compte_gog(c: Option<CompteBoutique>, cle: &str) -> Resultat<(CompteBoutique, CompteGog)> {
    let mut c = c.unwrap_or_default();
    let mut comptes = comptes_gog_lus(&c);
    let i = comptes.iter().position(|x| x.cle() == cle).ok_or_else(|| Erreur::Introuvable("Ce compte GOG n'est plus dans Frogtend.".into()))?;
    let x = comptes.remove(i);
    if x.navigateur == NavigateurGog::Profil {
        // Sa session gog.com n'est plus relue tant que la personne ne se reconnecte pas elle-même.
        c.navigateur_gog_oublie = true;
    }
    poser_comptes_gog(&mut c, comptes);
    let gardees = fiches_gardees(&c.comptes_gog);
    c.fiches_gog.retain(|k, _| gardees.contains(k));
    Ok((c, x))
}

/// Les numéros de jeux dont le cache commun des fiches publiques garde la fiche : ceux de la dernière bibliothèque lue
/// de CHAQUE compte, et ceux de ses jeux (jamais la lecture d'un compte qui vide les fiches d'un autre). Pur, testé.
pub fn fiches_gardees(comptes: &[CompteGog]) -> BTreeSet<String> {
    let mut s = BTreeSet::new();
    for c in comptes {
        s.extend(c.ids_lus.iter().cloned());
        s.extend(c.jeux.iter().filter_map(|j| j.id.strip_prefix("gog_")).map(String::from));
    }
    s
}

/// Ajoute au cache commun les fiches lues à l'instant, puis ne garde que celles de tous les comptes (`fiches_gardees`).
pub fn fusionner_fiches_gog(c: &mut CompteBoutique, nouvelles: std::collections::BTreeMap<String, crate::gog_galaxy::FicheGog>) {
    c.fiches_gog.extend(nouvelles);
    let gardees = fiches_gardees(&c.comptes_gog);
    c.fiches_gog.retain(|k, _| gardees.contains(k));
}

/// Une image publique de boutique (hôte autorisé), gardée en cache sous son empreinte.
pub async fn image_par_adresse(cache: &Path, url: &str) -> Option<Vec<u8>> {
    if !image_autorisee(url) {
        return None;
    }
    use sha2::{Digest, Sha256};
    let f = cache.join("images").join(format!("{:x}", Sha256::digest(url.as_bytes())));
    if let Ok(o) = std::fs::read(&f) {
        return Some(o);
    }
    let o = telecharger_plafonne(url, PLAFOND_IMAGE).await?;
    let _ = std::fs::create_dir_all(f.parent()?);
    let _ = std::fs::write(&f, &o);
    Some(o)
}

/// Le plafond d'une image de boutique téléchargée (une boîte Epic fait quelques centaines de Ko).
pub const PLAFOND_IMAGE: usize = 20 * 1024 * 1024;

/// Une image publique de boutique RÉDUITE à `largeur` pixels avant d'être gardée (`<cache>\images\<empreinte>-<largeur>`) :
/// l'original n'est jamais gardé (500 boîtes Epic de 1 200 × 1 600). Une image déjà assez petite est gardée telle quelle ;
/// une image illisible est rendue sans être gardée. Un original gardé avant ce lot sert de source (rien à retélécharger).
pub async fn image_reduite(cache: &Path, url: &str, largeur: u32) -> Option<Vec<u8>> {
    if !image_autorisee(url) {
        return None;
    }
    use sha2::{Digest, Sha256};
    let empreinte = format!("{:x}", Sha256::digest(url.as_bytes()));
    let dossier = cache.join("images");
    let f = dossier.join(format!("{empreinte}-{largeur}"));
    if let Ok(o) = std::fs::read(&f) {
        return Some(o);
    }
    let original = match std::fs::read(dossier.join(&empreinte)) {
        Ok(o) => o,
        Err(_) => {
            telecharger_plafonne(url, PLAFOND_IMAGE).await?
        }
    };
    let source = original.clone();
    let reduite = tokio::task::spawn_blocking(move || reduire_image(&source, largeur)).await.ok()?;
    let a_garder = match reduite {
        Some(Reduction::Reduite(o)) => o,
        Some(Reduction::DejaPetite) => original,
        None => return Some(original), // illisible pour Frogtend : montrée, jamais gardée
    };
    let _ = std::fs::create_dir_all(&dossier);
    let _ = std::fs::write(&f, &a_garder);
    Some(a_garder)
}

/// Télécharge une adresse, morceau par morceau, et ABANDONNE dès que `plafond` octets sont dépassés (annoncés par
/// l'en-tête de longueur, ou reçus, même sans en-tête) : jamais une réponse démesurée en mémoire. `None` : échec,
/// statut d'erreur ou plafond dépassé.
pub async fn telecharger_plafonne(url: &str, plafond: usize) -> Option<Vec<u8>> {
    let mut r = client().ok()?.get(url).send().await.ok()?;
    if !r.status().is_success() || r.content_length().is_some_and(|l| l > plafond as u64) {
        return None;
    }
    let mut o = Vec::new();
    while let Some(morceau) = r.chunk().await.ok()? {
        if o.len() + morceau.len() > plafond {
            return None;
        }
        o.extend_from_slice(&morceau);
    }
    Some(o)
}

/// Ce que donne la réduction d'une image.
#[derive(Debug, PartialEq)]
pub enum Reduction {
    /// L'image réduite, en JPEG.
    Reduite(Vec<u8>),
    /// L'image n'est pas plus large que demandé : elle reste telle quelle.
    DejaPetite,
}

/// Réduit une image (JPEG, PNG, WebP) à `largeur` pixels, proportions gardées, en JPEG. `None` : image illisible ou
/// démesurée (plafonds de taille et de mémoire : jamais une image piégée qui remplit la mémoire).
pub fn reduire_image(o: &[u8], largeur: u32) -> Option<Reduction> {
    let mut lecteur = image::ImageReader::new(std::io::Cursor::new(o)).with_guessed_format().ok()?;
    let mut limites = image::Limits::default();
    limites.max_image_width = Some(10_000);
    limites.max_image_height = Some(10_000);
    limites.max_alloc = Some(256 * 1024 * 1024);
    lecteur.limits(limites);
    let img = lecteur.decode().ok()?;
    let largeur = largeur.max(1);
    if img.width() <= largeur {
        return Some(Reduction::DejaPetite);
    }
    let hauteur = ((img.height() as u64 * largeur as u64) / img.width() as u64).max(1) as u32;
    let r = img.resize_exact(largeur, hauteur, image::imageops::FilterType::Triangle).to_rgb8();
    let mut sortie = vec![];
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut sortie, 85).encode_image(&r).ok()?;
    Some(Reduction::Reduite(sortie))
}

/// Les images officielles d'un jeu Steam (CDN public de Steam, sans clé) : la jaquette en hauteur, sinon la bannière.
pub fn adresses_image_steam(appid: &str) -> [String; 2] {
    [
        format!("https://cdn.akamai.steamstatic.com/steam/apps/{appid}/library_600x900.jpg"),
        format!("https://cdn.akamai.steamstatic.com/steam/apps/{appid}/header.jpg"),
    ]
}

/// La jaquette d'un jeu Steam, gardée en cache sur ce PC (`<cache>\steam\<appid>.jpg`) ; `None` si Steam n'en a pas.
/// La jaquette Steam d'un appid RÉDUITE à `largeur` (cartes « à installer » de Taodbox, lot C2 : des centaines de
/// jeux possédés), gardée réduite à côté de l'originale ; jamais agrandie. Une image illisible est montrée telle quelle.
pub async fn image_steam_reduite(cache: &Path, appid: &str, largeur: u32) -> Option<Vec<u8>> {
    if appid.is_empty() || !appid.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let f = cache.join("steam").join(format!("{appid}-{largeur}.jpg"));
    if let Ok(o) = std::fs::read(&f) {
        return Some(o);
    }
    let original = image_steam(cache, appid).await?;
    let source = original.clone();
    match tokio::task::spawn_blocking(move || reduire_image(&source, largeur)).await.ok()? {
        Some(Reduction::Reduite(o)) => {
            let _ = std::fs::write(&f, &o);
            Some(o)
        }
        _ => Some(original),
    }
}

pub async fn image_steam(cache: &Path, appid: &str) -> Option<Vec<u8>> {
    if appid.is_empty() || !appid.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let f = cache.join("steam").join(format!("{appid}.jpg"));
    if let Ok(o) = std::fs::read(&f) {
        return Some(o);
    }
    let c = client().ok()?;
    for u in adresses_image_steam(appid) {
        let Ok(r) = c.get(&u).send().await else { continue };
        if r.status().is_success() {
            let o = r.bytes().await.ok()?.to_vec();
            if o.len() > 1000 {
                let _ = std::fs::create_dir_all(f.parent()?);
                let _ = std::fs::write(&f, &o);
                return Some(o);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 0.219.1 : seules les fiches classées PAS un jeu, hors de la liste gardée, font retirer une carte.
    #[test]
    fn les_cartes_gog_ecartees_viennent_des_fiches_classees() {
        use crate::gog_galaxy::FicheGog;
        let mut c = CompteBoutique::default();
        let fiche = |genre: &str| FicheGog { titre: "T".into(), genre: genre.into(), genre_du_jeu: genre.into(), visible: Some(true), lu_le: 1, ..Default::default() };
        c.fiches_gog.insert("1".into(), fiche("dlc"));
        c.fiches_gog.insert("2".into(), fiche("inconnu"));
        c.fiches_gog.insert("3".into(), fiche("game"));
        c.fiches_gog.insert("4".into(), FicheGog { introuvable: true, ..Default::default() });
        c.fiches_gog.insert("5".into(), fiche("spam"));
        let garde = JeuBoutique { id: "gog_5".into(), nom: "Montré par le site".into(), minutes: 0, derniere: 0, installe: false, plateforme: "gog".into(), image: None, dossier: None, pret: None, succes: None, releases: vec![], titre_douteux: false };
        let e = cartes_gog_ecartees(&c, &[garde]);
        let id = |n: &str| crate::ludotheque::id_boutique("gog", n);
        assert_eq!(e, [id("1"), id("2")].into_iter().collect());
    }
    use httpmock::prelude::*;
    use serde_json::json;

    #[test]
    fn le_compte_steam_se_lit_de_toutes_les_facons() {
        assert_eq!(lire_compte_steam("NimaiTakahashi"), Some((true, "NimaiTakahashi".into())));
        assert_eq!(lire_compte_steam("https://steamcommunity.com/id/NimaiTakahashi/"), Some((true, "NimaiTakahashi".into())));
        assert_eq!(lire_compte_steam("https://steamcommunity.com/profiles/76561197960287930"), Some((false, "76561197960287930".into())));
        assert_eq!(lire_compte_steam(""), None);
        assert_eq!(lire_compte_steam("nom avec espace"), None);
    }

    #[tokio::test]
    async fn steam_resout_le_compte_puis_liste_les_jeux_et_ne_montre_jamais_la_cle() {
        let m = MockServer::start();
        let r = m.mock(|w, t| {
            w.method(GET).path("/ISteamUser/ResolveVanityURL/v1/").query_param("key", "CLE").query_param("vanityurl", "Seb");
            t.status(200).json_body(json!({"response": {"success": 1, "steamid": "76561197960287930"}}));
        });
        let j = m.mock(|w, t| {
            w.method(GET).path("/IPlayerService/GetOwnedGames/v1/").query_param("steamid", "76561197960287930").query_param("include_appinfo", "1");
            t.status(200).json_body(json!({"response": {"game_count": 2, "games": [
                {"appid": 620, "name": "Portal 2", "playtime_forever": 300, "rtime_last_played": 1700000000},
                {"appid": 400, "name": "Portal", "playtime_forever": 0}]}}));
        });
        let id = steamid(&m.base_url(), "CLE", "https://steamcommunity.com/id/Seb").await.unwrap();
        let l = jeux_possedes(&m.base_url(), "CLE", &id).await.unwrap();
        assert_eq!(l.iter().map(|x| x.nom.as_str()).collect::<Vec<_>>(), ["Portal", "Portal 2"]);
        assert_eq!(l[1].minutes, 300);
        r.assert();
        j.assert();

        // Clé refusée : message clair, sans l'adresse (qui contient la clé).
        let m2 = MockServer::start();
        m2.mock(|w, t| {
            w.method(GET).path("/IPlayerService/GetOwnedGames/v1/");
            t.status(403);
        });
        let e = format!("{:?}", jeux_possedes(&m2.base_url(), "SECRETE", "1").await.unwrap_err());
        assert!(e.contains("clé") && !e.contains("SECRETE"));
        // Profil privé : pas de « games ».
        let m3 = MockServer::start();
        m3.mock(|w, t| {
            w.method(GET).path("/IPlayerService/GetOwnedGames/v1/");
            t.status(200).json_body(json!({"response": {}}));
        });
        assert!(format!("{:?}", jeux_possedes(&m3.base_url(), "C", "1").await.unwrap_err()).contains("Confidentialité"));
    }

    #[test]
    fn les_jeux_installes_se_lisent_dans_les_bibliotheques() {
        let d = tempfile::tempdir().unwrap();
        let steam = d.path().join("Steam");
        let autre = d.path().join("SteamLibrary");
        std::fs::create_dir_all(steam.join("steamapps")).unwrap();
        std::fs::create_dir_all(autre.join("steamapps")).unwrap();
        std::fs::write(
            steam.join("steamapps/libraryfolders.vdf"),
            format!("\"libraryfolders\"\n{{\n\t\"0\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n\t\"1\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n}}\n",
                steam.display().to_string().replace('\\', "\\\\"), autre.display().to_string().replace('\\', "\\\\")),
        ).unwrap();
        let manifeste = |appid: &str, dir: &str, etat: u64| format!("\"AppState\"\n{{\n\t\"appid\"\t\t\"{appid}\"\n\t\"StateFlags\"\t\t\"{etat}\"\n\t\"installdir\"\t\t\"{dir}\"\n}}\n");
        std::fs::write(steam.join("steamapps/appmanifest_620.acf"), manifeste("620", "Portal 2", 4)).unwrap();
        std::fs::write(autre.join("steamapps/appmanifest_400.acf"), manifeste("400", "Portal", 4 | 64)).unwrap();
        // En cours de téléchargement (1048576) : pas prêt. Un dossier « .. » : refusé. Dossier absent : rien.
        std::fs::write(autre.join("steamapps/appmanifest_500.acf"), manifeste("500", "Left 4 Dead", 4 | 1_048_576)).unwrap();
        std::fs::write(autre.join("steamapps/appmanifest_501.acf"), manifeste("501", "..", 4)).unwrap();
        std::fs::write(autre.join("steamapps/appmanifest_502.acf"), manifeste("502", "Absent", 4)).unwrap();
        for (b, dir) in [(&steam, "Portal 2"), (&autre, "Portal"), (&autre, "Left 4 Dead")] {
            std::fs::create_dir_all(b.join("steamapps/common").join(dir)).unwrap();
        }
        let s = installes_steam_dossiers(&steam);
        assert_eq!(s.keys().collect::<Vec<_>>(), ["400", "500", "620"], "en mise à jour : installé quand même");
        assert_eq!(s["400"].dossier, autre.join("steamapps/common/Portal"), "jamais « Portal 2 » pour « Portal »");
        assert!(s["400"].pret && s["620"].pret && !s["500"].pret);
        assert!(lire_manifeste_steam("\"appid\" \"12a\"\n\"installdir\" \"x\"\n\"StateFlags\" \"4\"").is_none());
        assert!(s["400"].lance && !s["620"].lance, "AppRunning (64)");
        // `buildid` (relevé réel du 21/10 : présent dans les 11 appmanifest de Venkman), jamais `TargetBuildID`.
        let m = lire_manifeste_steam("\"appid\" \"268500\"\n\"StateFlags\" \"4\"\n\"installdir\" \"XCOM 2\"\n\"TargetBuildID\" \"9\"\n\"buildid\" \"8270065\"").unwrap();
        assert_eq!(m.buildid.as_deref(), Some("8270065"));
        assert_eq!(lire_manifeste_steam("\"appid\" \"1\"\n\"StateFlags\" \"4\"\n\"installdir\" \"x\"\n\"TargetBuildID\" \"9\"").unwrap().buildid, None);
        assert!(steam_pret(4) && steam_pret(4 | 64) && !steam_pret(6) && !steam_pret(1) && !steam_pret(4 | 1024) && !steam_pret(4 | 8));
        assert!(steam_installe(6) && !steam_installe(1) && !steam_installe(1_048_576), "mise à jour au lancement : installé ; 1er téléchargement : non");
    }

    #[test]
    fn une_bibliotheque_steam_ecrite_deux_fois_n_est_lue_qu_une_fois() {
        let d = tempfile::tempdir().unwrap();
        let steam = d.path().join("Steam");
        std::fs::create_dir_all(steam.join("steamapps")).unwrap();
        // Le registre donne « d:/…/steam », le vdf « D:\…\Steam » : le même dossier.
        let autre_forme = steam.to_string_lossy().to_uppercase().replace('\\', "\\\\");
        std::fs::write(steam.join("steamapps/libraryfolders.vdf"), format!("\"0\"\n{{\n\t\"path\"\t\t\"{autre_forme}\\\\\"\n}}\n")).unwrap();
        assert_eq!(bibliotheques_steam(&steam).len(), 1);
    }

    #[test]
    #[ignore]
    fn essai_lire_galaxy() {
        let d = tempfile::tempdir().unwrap();
        let l = lire_galaxy(&dossier_galaxy().expect("pas de Galaxy"), d.path()).unwrap();
        println!("{} jeux", l.len());
        let mut par: std::collections::BTreeMap<String, usize> = Default::default();
        for j in &l {
            *par.entry(nom_plateforme(&j.plateforme).to_string()).or_default() += 1;
        }
        println!("par boutique : {par:?}");
        // A6c : les jeux possédés sur plusieurs boutiques que Galaxy relie.
        let cles: std::collections::BTreeSet<&str> = l.iter().map(|j| j.id.as_str()).collect();
        let relies: Vec<(&str, Vec<&str>)> =
            l.iter().map(|j| (j.nom.as_str(), j.releases.iter().map(String::as_str).filter(|r| *r != j.id && cles.contains(r)).collect::<Vec<_>>())).filter(|(_, r)| !r.is_empty()).collect();
        println!("{} jeux avec releases ; {} reliés à une autre copie possédée : {:?}", l.iter().filter(|j| !j.releases.is_empty()).count(), relies.len(), relies.iter().take(8).collect::<Vec<_>>());
        for j in l.iter().filter(|j| j.installe) {
            println!("installé : {} [{}] {} min, image={}, dossier={:?}", j.nom, j.plateforme, j.minutes, j.image.is_some(), j.dossier);
        }
        // Steam, en lecture seule aussi : le dossier par le registre (API), les jeux prêts et leur dossier.
        if let Some(s) = dossier_steam() {
            println!("Steam : {} ; bibliothèques {:?}", s.display(), bibliotheques_steam(&s));
            for (id, i) in installes_steam_dossiers(&s) {
                println!("Steam installé : {id} → {} (prêt : {})", i.dossier.display(), i.pret);
            }
        }
        println!("avec jaquette : {}", l.iter().filter(|j| j.image.is_some()).count());
    }

    /// LECTURE SEULE, sur une COPIE (accord de Seb, 02/10) : la structure de la base de GOG Galaxy.
    #[test]
    #[ignore]
    fn essai_structure_gog_galaxy() {
        let d = tempfile::tempdir().unwrap();
        let source = Path::new(r"C:\ProgramData\GOG.com\Galaxy\storage");
        for f in ["galaxy-2.0.db", "galaxy-2.0.db-wal", "galaxy-2.0.db-shm"] {
            let _ = std::fs::copy(source.join(f), d.path().join(f));
        }
        let c = rusqlite::Connection::open(d.path().join("galaxy-2.0.db")).unwrap();
        let tables: Vec<String> = c.prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name").unwrap()
            .query_map([], |r| r.get(0)).unwrap().flatten().collect();
        println!("tables : {}", tables.join(", "));
        for t in ["LibraryReleases", "GamePieces", "GamePieceTypes", "InstalledBaseProducts", "InstalledExternalProducts", "ReleaseProperties"] {
            if let Ok(s) = c.prepare(&format!("SELECT * FROM {t} LIMIT 3")) {
                let cols: Vec<String> = s.column_names().iter().map(|x| x.to_string()).collect();
                let n: i64 = c.query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| r.get(0)).unwrap_or(-1);
                println!("{t} ({n} lignes) : {}", cols.join(", "));
            }
        }
        if let Ok(mut s) = c.prepare("SELECT id, type FROM GamePieceTypes") {
            let l: Vec<String> = s.query_map([], |r| Ok(format!("{}={}", r.get::<_, i64>(0)?, r.get::<_, String>(1)?))).unwrap().flatten().collect();
            println!("types : {}", l.join(" "));
        };
        let mut s = c.prepare("SELECT substr(releaseKey, 1, instr(releaseKey, '_') - 1) AS p, COUNT(*) FROM LibraryReleases GROUP BY p ORDER BY 2 DESC").unwrap();
        let l: Vec<String> = s.query_map([], |r| Ok(format!("{}={}", r.get::<_, String>(0)?, r.get::<_, i64>(1)?))).unwrap().flatten().collect();
        println!("par boutique : {}", l.join(" "));
        drop(s);
        for (t, nom) in [(460, "originalTitle"), (457, "originalImages"), (6, "storeProductState")] {
            let v: Option<(String, String)> = c
                .query_row(&format!("SELECT releaseKey, value FROM GamePieces WHERE gamePieceTypeId = {t} AND releaseKey LIKE 'gog_%' LIMIT 1"), [], |r| Ok((r.get(0)?, r.get(1)?)))
                .ok();
            println!("{nom} : {}", v.map(|(k, v)| format!("{k} → {}", v.chars().take(300).collect::<String>())).unwrap_or_default());
        }
        let n: i64 = c.query_row("SELECT COUNT(*) FROM LibraryReleases l JOIN ReleaseProperties p ON p.releaseKey = l.releaseKey WHERE p.isDlc = 0 AND p.isVisibleInLibrary = 1", [], |r| r.get(0)).unwrap_or(-1);
        println!("jeux visibles (hors DLC) : {n}");
        let ins: Vec<String> = c.prepare("SELECT productId, installationPath FROM InstalledBaseProducts").unwrap().query_map([], |r| Ok(format!("{} → {}", r.get::<_, i64>(0)?, r.get::<_, String>(1)?))).unwrap().flatten().collect();
        println!("installés GOG : {}", ins.join(" | "));
        for t in ["GameTimes", "LastPlayedDates", "InstalledExternalProducts", "Platforms", "PlayTasks", "PlayTaskLaunchParameters"] {
            let mut s = c.prepare(&format!("SELECT * FROM {t} LIMIT 2")).unwrap();
            let cols: Vec<String> = s.column_names().iter().map(|x| x.to_string()).collect();
            let n = cols.len();
            let lignes: Vec<String> = s.query_map([], |r| Ok((0..n).map(|i| r.get::<_, rusqlite::types::Value>(i).map(|v| format!("{v:?}").chars().take(80).collect::<String>()).unwrap_or_default()).collect::<Vec<_>>().join(" ; "))).unwrap().flatten().collect();
            println!("{t} [{}] : {}", cols.join(", "), lignes.join(" || "));
        }
        let v: String = c.query_row("SELECT value FROM GamePieces WHERE gamePieceTypeId = 457 AND releaseKey LIKE 'epic_%' LIMIT 1", [], |r| r.get(0)).unwrap_or_default();
        println!("images epic : {}", v.chars().take(500).collect::<String>());
    }

    /// Une image PNG inventée de `l` × `h` pixels.
    fn png(l: u32, h: u32) -> Vec<u8> {
        let img = image::RgbImage::from_pixel(l, h, image::Rgb([200, 30, 30]));
        let mut o = std::io::Cursor::new(vec![]);
        img.write_to(&mut o, image::ImageFormat::Png).unwrap();
        o.into_inner()
    }

    #[tokio::test]
    async fn la_jaquette_steam_reduite_vient_du_cache_et_reste_reduite() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("steam")).unwrap();
        std::fs::write(d.path().join("steam").join("620.jpg"), png(600, 900)).unwrap();
        let o = image_steam_reduite(d.path(), "620", 400).await.unwrap();
        let r = image::load_from_memory(&o).unwrap();
        assert_eq!((r.width(), r.height()), (400, 600));
        assert_eq!(std::fs::read(d.path().join("steam").join("620-400.jpg")).unwrap(), o, "gardée réduite");
        assert!(image_steam_reduite(d.path(), "../620", 400).await.is_none());
    }

    #[test]
    fn une_jaquette_de_boutique_est_reduite_a_la_largeur_demandee() {
        let Some(Reduction::Reduite(o)) = reduire_image(&png(1200, 1600), 300) else { panic!("réduite") };
        let r = image::load_from_memory(&o).unwrap();
        assert_eq!((r.width(), r.height()), (300, 400), "proportions gardées");
        assert_eq!(image::guess_format(&o).unwrap(), image::ImageFormat::Jpeg);
        assert_eq!(reduire_image(&png(200, 300), 300), Some(Reduction::DejaPetite), "jamais agrandie");
        assert_eq!(reduire_image(b"pas une image", 300), None);
        // Une image annoncée démesurée (en-tête PNG de 100 000 × 100 000) : refusée sans tout décoder.
        let mut piege = png(1, 1);
        piege[16..20].copy_from_slice(&100_000u32.to_be_bytes());
        piege[20..24].copy_from_slice(&100_000u32.to_be_bytes());
        assert_eq!(reduire_image(&piege, 300), None);
    }

    /// Un petit serveur qui répond en morceaux (`Transfer-Encoding: chunked`), SANS en-tête de longueur.
    fn serveur_en_morceaux(morceaux: usize, taille: usize) -> String {
        use std::io::{Read, Write};
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let adresse = format!("http://{}/image", l.local_addr().unwrap());
        std::thread::spawn(move || {
            if let Ok((mut c, _)) = l.accept() {
                let mut tampon = [0u8; 4096];
                let _ = c.read(&mut tampon);
                let _ = c.write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n");
                for _ in 0..morceaux {
                    if c.write_all(format!("{taille:x}\r\n").as_bytes()).is_err() || c.write_all(&vec![b'x'; taille]).is_err() || c.write_all(b"\r\n").is_err() {
                        return; // le client a abandonné : c'est attendu
                    }
                }
                let _ = c.write_all(b"0\r\n\r\n");
            }
        });
        adresse
    }

    #[tokio::test]
    async fn un_telechargement_d_image_s_arrete_des_le_plafond_meme_sans_longueur_annoncee() {
        // 10 morceaux de 1 000 octets, sans en-tête de longueur.
        assert_eq!(telecharger_plafonne(&serveur_en_morceaux(10, 1000), 20_000).await.map(|o| o.len()), Some(10_000));
        assert_eq!(telecharger_plafonne(&serveur_en_morceaux(10, 1000), 9_999).await, None, "plafond dépassé en route");
        // Avec un en-tête de longueur trop grand : refusé avant de lire.
        let s = MockServer::start();
        s.mock(|w, t| {
            w.method(GET).path("/grande");
            t.status(200).body(vec![b'y'; 5000]);
        });
        assert_eq!(telecharger_plafonne(&s.url("/grande"), 4999).await, None);
        assert_eq!(telecharger_plafonne(&s.url("/grande"), 5000).await.map(|o| o.len()), Some(5000));
    }

    /// `panic = "abort"` en version livrée : une image piégée ne doit JAMAIS faire paniquer le décodage (Frogtend se
    /// fermerait). Des images JPEG, PNG et WebP abîmées octet par octet et coupées partout : `None` ou une réduction,
    /// jamais une panique (un test qui panique échoue).
    #[test]
    fn une_image_abimee_ou_coupee_ne_fait_jamais_paniquer_le_decodage() {
        let source = image::RgbImage::from_fn(64, 48, |x, y| image::Rgb([(x * 4) as u8, (y * 5) as u8, ((x + y) * 2) as u8]));
        let mut essais = 0;
        for format in [image::ImageFormat::Jpeg, image::ImageFormat::Png, image::ImageFormat::WebP] {
            let mut o = std::io::Cursor::new(vec![]);
            source.write_to(&mut o, format).unwrap();
            let o = o.into_inner();
            assert!(matches!(reduire_image(&o, 32), Some(Reduction::Reduite(_))), "{format:?} intacte");
            // Coupée à chaque longueur.
            for n in 0..o.len() {
                let _ = reduire_image(&o[..n], 32);
                essais += 1;
            }
            // Chaque octet changé (plusieurs valeurs), un à la fois.
            let mut graine: u32 = 0x1234_5678;
            for i in 0..o.len() {
                for _ in 0..4 {
                    graine = graine.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    let mut a = o.clone();
                    a[i] = (graine >> 24) as u8;
                    let _ = reduire_image(&a, 32);
                    essais += 1;
                }
            }
            // Des dimensions annoncées démesurées dans l'en-tête.
            let mut a = o.clone();
            for k in (0..a.len().min(64)).step_by(2) {
                a[k] = 0xff;
                let _ = reduire_image(&a, 32);
                essais += 1;
            }
        }
        assert!(essais > 1000, "{essais} images piégées essayées");
    }

    #[tokio::test]
    async fn la_jaquette_reduite_est_gardee_et_jamais_l_original_telecharge() {
        use sha2::{Digest, Sha256};
        let d = tempfile::tempdir().unwrap();
        let url = "https://cdn1.epicgames.com/item/ns/boite";
        let empreinte = format!("{:x}", Sha256::digest(url.as_bytes()));
        // Un original gardé avant ce lot : il sert de source, sans réseau.
        std::fs::create_dir_all(d.path().join("images")).unwrap();
        std::fs::write(d.path().join("images").join(&empreinte), png(1200, 1600)).unwrap();
        let o = image_reduite(d.path(), url, 400).await.unwrap();
        assert_eq!(image::load_from_memory(&o).unwrap().width(), 400);
        let gardee = d.path().join("images").join(format!("{empreinte}-400"));
        assert_eq!(std::fs::read(&gardee).unwrap(), o, "la réduite est gardée");
        // La suivante vient du cache.
        std::fs::remove_file(d.path().join("images").join(&empreinte)).unwrap();
        assert_eq!(image_reduite(d.path(), url, 400).await.unwrap(), o);
        assert!(image_reduite(d.path(), "https://exemple.com/x.png", 400).await.is_none(), "hôte non autorisé");
    }

    #[test]
    fn galaxy_dates_plateformes_et_images_autorisees() {
        assert_eq!(date_en_secondes("1970-01-01 00:00:00"), Some(0));
        assert_eq!(date_en_secondes("2022-04-29 20:23:22"), Some(1_651_263_802));
        assert_eq!(date_en_secondes("n'importe quoi"), None);
        assert_eq!(nom_plateforme("epic"), "Epic Games");
        assert_eq!(nom_plateforme("origin"), "EA");
        assert!(image_autorisee("https://images.gog.com/x_glx_vertical_cover.webp?namespace=gamesdb"));
        assert!(!image_autorisee("https://exemple.com/x.png"));
        assert!(!image_autorisee("http://images.gog.com/x.png"), "https seulement");
        assert!(image_autorisee("https://ubistatic3-a.akamaihd.net/orbit/uplay_launcher_3_0/assets/da1733d50f6fbcf5b344d23ebb1ed52c.jpg"));
        assert!(!image_autorisee("https://ubistatic3-a.akamaihd.net/autre/x.jpg"), "seulement les images du lanceur");
    }

    #[test]
    fn les_images_du_compte_gog_sont_normalisees_vers_une_adresse_autorisee() {
        let e = "1e8a5a1fb0e6d4d4bfa4f6fbd1b8e8d2c3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8";
        // La forme supposée de la liste : sans protocole ni extension → https et la vignette de 392 pixels.
        let u = image_gog(&format!("//images-1.gog-statics.com/{e}")).unwrap();
        assert_eq!(u, format!("https://images-1.gog-statics.com/{e}_392.jpg"));
        assert!(image_autorisee(&u), "l'adresse rendue passe la porte des images");
        assert_eq!(image_gog(&format!("https://IMAGES-4.gog-statics.com/{e}")).as_deref(), Some(format!("https://images-4.gog-statics.com/{e}_392.jpg").as_str()));
        // Une extension déjà là : gardée si elle est sûre.
        assert_eq!(image_gog(&format!("//images.gog-statics.com/{e}_product_tile_256.webp")).as_deref(), Some(format!("https://images.gog-statics.com/{e}_product_tile_256.webp").as_str()));
        // Formes inconnues ou pièges : pas d'image plutôt qu'une adresse non autorisée.
        for piege in [
            String::new(),
            format!("http://images-1.gog-statics.com/{e}"),
            format!("//images-1.gog-statics.com.evil.example/{e}"),
            format!("//evil.example/images-1.gog-statics.com/{e}"),
            format!("//images-9.gog-statics.com/{e}"),
            format!("//images-1.gog-statics.com/dossier/{e}"),
            format!("//images-1.gog-statics.com/{e}.jpg?x=1"),
            format!("//images-1.gog-statics.com/{e}.svg"),
            "//images-1.gog-statics.com/court".into(),
            format!("//images-1.gog-statics.com:8443/{e}"),
            format!("javascript://images-1.gog-statics.com/{e}"),
        ] {
            assert_eq!(image_gog(&piege), None, "{piege}");
        }
        assert!(!image_autorisee(&format!("https://images-1.gog-statics.com/{e}_392.jpg/../x")));
        assert!(!image_autorisee(&format!("https://images-1.gog-statics.com.evil.example/{e}_392.jpg")));
    }

    #[test]
    fn les_jeux_du_compte_gog_se_rangent_sans_doublon_ni_marque() {
        let lu = |id: &str, t: &str, i: &str| crate::gratuits::JeuGogLu { id: id.into(), titre: t.into(), image: i.into() };
        let l = jeux_du_compte_gog(&[lu("1207658924", "Unreal Tournament™ GOTY", "//images-1.gog-statics.com/1e8a5a1fb0e6d4d4bfa4"), lu("1207658924", "Doublon", ""), lu("5", "™", "x")]);
        assert_eq!(l.len(), 2);
        assert_eq!((l[0].id.as_str(), l[0].nom.as_str(), l[0].installe, l[0].plateforme.as_str()), ("gog_1207658924", "Unreal Tournament GOTY", false, "gog"));
        assert_eq!(l[0].image.as_deref(), Some("https://images-1.gog-statics.com/1e8a5a1fb0e6d4d4bfa4_392.jpg"));
        assert_eq!((l[1].nom.as_str(), l[1].image.as_ref()), ("5", None), "titre vidé par les marques : le numéro ; image inconnue : aucune");
    }

    #[test]
    fn la_liste_du_compte_gog_s_ecrit_sans_melanger_deux_comptes() {
        let j = |id: &str| jg(id, id);
        // Première lecture.
        let (c, ch) = appliquer_import_gog_profil(None, "", &lu("42", "4200", &["gog_1"]), "t1").unwrap();
        assert_eq!(ch, ChangementGog::default());
        assert_eq!((c.id_compte.as_str(), c.jeux.len(), c.maj_le.as_str(), c.id_compte_precedent.as_str()), ("42", 1, "t1", ""));
        assert_eq!((c.comptes_gog.len(), c.comptes_gog[0].id_galaxy.as_str(), c.comptes_gog[0].navigateur), (1, "4200", NavigateurGog::Profil));
        // Le même compte : SA liste est remplacée.
        let (c, ch) = appliquer_import_gog_profil(Some(c), "42", &lu("42", "4200", &["gog_1", "gog_2"]), "t2").unwrap();
        assert!(ch == ChangementGog::default() && c.jeux.len() == 2);
        // Un autre compte dans le navigateur du profil : l'ancien devient « ajouté » (gardé, avec ses jeux).
        let (c, ch) = appliquer_import_gog_profil(Some(c), "42", &lu("77", "7700", &["gog_9"]), "t3").unwrap();
        assert_eq!(ch.ancien_profil.as_deref(), Some("4200"));
        assert_eq!((c.id_compte.as_str(), c.jeux.len()), ("77", 1));
        assert_eq!(c.comptes_gog.iter().map(|x| (x.id_galaxy.as_str(), x.navigateur, x.jeux.len())).collect::<Vec<_>>(), [("7700", NavigateurGog::Profil, 1), ("4200", NavigateurGog::Ajoute, 2)]);
        // Le compte rangé a changé PENDANT la lecture : refus, rien n'est mélangé.
        assert!(appliquer_import_gog_profil(Some(c.clone()), "42", &lu("42", "4200", &[]), "t4").is_err());
        assert!(appliquer_import_gog_profil(Some(c), "77", &lu("pas-un-numero", "", &[]), "t4").is_err());
        // Écrit dans boutiques.json sous « gog », à côté de Steam (jamais mélangé avec lui).
        let d = tempfile::tempdir().unwrap();
        ecrire(d.path(), Some(&CompteBoutique { steamid: "76561197960287930".into(), ..Default::default() })).unwrap();
        modifier_source(d.path(), "gog", |c| appliquer_import_gog_profil(c, "", &LectureCompteGog { jeux: vec![j("gog_1")], ..lu("42", "", &[]) }, "t").map(|(c, _)| Some(c))).unwrap();
        assert_eq!(lire_source(d.path(), "gog").unwrap().id_compte, "42");
        assert_eq!(lire(d.path()).unwrap().steamid, "76561197960287930");
        assert!(lire(d.path()).unwrap().id_compte.is_empty());
        // Un fichier d'avant le lot C4a se lit toujours (champs absents).
        std::fs::write(d.path().join("boutiques.json"), r#"{"steam":{"compte":"x","steamid":"1","maj_le":"","jeux":[]}}"#).unwrap();
        assert_eq!(lire(d.path()).unwrap().id_compte, "");
    }

    fn jg(id: &str, nom: &str) -> JeuBoutique {
        JeuBoutique { id: id.into(), nom: nom.into(), minutes: 0, derniere: 0, installe: false, plateforme: "gog".into(), image: None, dossier: None, pret: None, succes: None, releases: vec![], titre_douteux: false }
    }

    fn lu(site: &str, galaxy: &str, jeux: &[&str]) -> LectureCompteGog {
        LectureCompteGog { id_site: site.into(), id_galaxy: galaxy.into(), nom: None, jeux: jeux.iter().map(|i| jg(i, i)).collect(), ids_lus: None, ancien_id_galaxy: None }
    }

    fn jeux_de(n: std::ops::Range<u32>, prefixe: u32) -> Vec<JeuBoutique> {
        n.map(|i| jg(&format!("gog_{}", prefixe + i), &format!("Jeu {i}"))).collect()
    }

    /// Lot C4b : un `boutiques.json` écrit par la 0.218 (un seul compte : `id_compte` et `jeux`) se lit comme un compte
    /// `Profil`, et la lecture suivante du même compte remplit son numéro Galaxy sans rien perdre.
    #[test]
    fn un_fichier_de_la_0_218_devient_un_compte_profil() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            d.path().join("boutiques.json"),
            r#"{"steam":{"compte":"x","steamid":"1","maj_le":"","jeux":[]},
                "gog":{"compte":"GOG","steamid":"","maj_le":"t0","jeux":[{"id":"gog_1","nom":"Un","minutes":0,"derniere":0}],"connecte_le":"t0","id_compte":"42",
                       "fiches_gog":{"1":{"titre":"Un","genre":"game","lu_le":5}}}}"#,
        )
        .unwrap();
        let c = lire_source(d.path(), "gog").unwrap();
        assert!(c.comptes_gog.is_empty(), "rien n'est réécrit à la seule lecture");
        let comptes = comptes_gog_lus(&c);
        assert_eq!(comptes.len(), 1);
        assert_eq!((comptes[0].id_site.as_str(), comptes[0].id_galaxy.as_str(), comptes[0].navigateur, comptes[0].jeux.len(), comptes[0].maj_le.as_str()), ("42", "", NavigateurGog::Profil, 1, "t0"));
        assert_eq!(comptes[0].cle(), "site-42");
        // La lecture suivante du même compte : numéro Galaxy rempli, anciens champs gardés en accord (retour arrière).
        modifier_source(d.path(), "gog", |c| appliquer_import_gog_profil(c, "42", &lu("42", "4200", &["gog_1", "gog_2"]), "t1").map(|(c, _)| Some(c))).unwrap();
        let c = lire_source(d.path(), "gog").unwrap();
        assert_eq!((c.comptes_gog.len(), c.comptes_gog[0].id_galaxy.as_str(), c.id_compte.as_str(), c.jeux.len()), (1, "4200", "42", 2));
        assert_eq!(c.fiches_gog.len(), 1, "le cache des fiches reste");
        assert_eq!(lire(d.path()).unwrap().steamid, "1", "Steam n'est pas touché");
        // Un fichier sans compte GOG : aucun compte inventé.
        assert!(comptes_gog_lus(&CompteBoutique::default()).is_empty());
    }

    /// Lot C4b : 88 jeux d'un compte + 197 d'un autre, sans jeu commun puis avec un jeu commun (une seule carte).
    #[test]
    fn les_jeux_de_deux_comptes_se_reunissent_une_carte_par_jeu() {
        let a = CompteGog { id_galaxy: "1".into(), jeux: jeux_de(0..88, 1000), navigateur: NavigateurGog::Profil, ..Default::default() };
        let mut b = CompteGog { id_galaxy: "2".into(), jeux: jeux_de(0..197, 5000), navigateur: NavigateurGog::Ajoute, ..Default::default() };
        assert_eq!(jeux_gog_reunis(&[a.clone(), b.clone()]).len(), 285);
        // Un jeu en commun (Cyberpunk chez les deux) : une carte, minutes et dernière partie au plus grand, l'image du
        // premier qui en a une, le compte Profil d'abord même rangé en second.
        let mut commun_a = jg("gog_1423049311", "Cyberpunk 2077");
        commun_a.minutes = 10;
        let mut commun_b = jg("gog_1423049311", "Cyberpunk 2077");
        commun_b.minutes = 92;
        commun_b.derniere = 7;
        commun_b.image = Some("https://images.gog.com/a.webp".into());
        commun_b.releases = vec!["steam_1091500".into()];
        let mut a2 = a.clone();
        a2.jeux.push(commun_a);
        b.jeux.push(commun_b);
        let l = jeux_gog_reunis(&[b, a2]);
        assert_eq!(l.len(), 286);
        assert_eq!(l[0].id, "gog_1000", "le compte Profil d'abord");
        let c: Vec<&JeuBoutique> = l.iter().filter(|j| j.id == "gog_1423049311").collect();
        assert_eq!(c.len(), 1);
        assert_eq!((c[0].minutes, c[0].derniere, c[0].image.as_deref(), c[0].releases.len()), (92, 7, Some("https://images.gog.com/a.webp"), 1));
    }

    /// Lot C4b : la lecture d'un compte ne touche jamais les jeux d'un autre ; une lecture partielle ne change rien.
    #[test]
    fn la_lecture_d_un_compte_ne_touche_pas_les_autres() {
        let (c, _) = appliquer_import_gog_profil(None, "", &lu("42", "4200", &["gog_1"]), "t1").unwrap();
        let c = ajouter_compte_gog(Some(c), &LectureCompteGog { nom: Some("Seb".into()), ..lu("", "7700", &["gog_9", "gog_8"]) }, false, "t2").unwrap();
        // Le compte ajouté relu : seuls ses jeux changent.
        let c = appliquer_import_gog_ajoute(Some(c), &lu("", "7700", &["gog_9"]), "t3").unwrap();
        assert_eq!(c.comptes_gog.iter().map(|x| x.jeux.len()).collect::<Vec<_>>(), [1, 1]);
        assert_eq!(c.comptes_gog[1].nom, "Seb", "le pseudo reste quand il n'est pas relu");
        // Le compte Profil relu : le compte ajouté garde les siens.
        let (c, _) = appliquer_import_gog_profil(Some(c), "42", &lu("42", "4200", &["gog_1", "gog_2", "gog_3"]), "t4").unwrap();
        assert_eq!(c.comptes_gog.iter().map(|x| x.jeux.len()).collect::<Vec<_>>(), [3, 1]);
        assert_eq!(c.jeux.len(), 3, "les anciens champs suivent le compte Profil seulement");
        // Un compte oublié pendant sa lecture : refus.
        assert!(appliquer_import_gog_ajoute(Some(c), &lu("", "9999", &[]), "t5").is_err());
    }

    /// Lot C4b : un compte déjà là ne s'ajoute pas deux fois ; un compte ajouté sans connexion se reconnecte.
    #[test]
    fn un_compte_deja_connu_ne_s_ajoute_pas() {
        let (c, _) = appliquer_import_gog_profil(None, "", &lu("42", "4200", &["gog_1"]), "t1").unwrap();
        let e = ajouter_compte_gog(Some(c.clone()), &lu("", "4200", &[]), true, "t2").unwrap_err();
        assert!(matches!(e, Erreur::Refus(ref m) if m == MOTIF_GOG_DEJA_LA), "le compte Profil");
        let c = ajouter_compte_gog(Some(c), &lu("", "7700", &["gog_9"]), false, "t2").unwrap();
        assert!(matches!(ajouter_compte_gog(Some(c.clone()), &lu("", "7700", &[]), false, "t3"), Err(Erreur::Refus(_))));
        // Reconnexion d'un compte ajouté dont la connexion était perdue : relu, jamais en double.
        let c = ajouter_compte_gog(Some(c), &lu("", "7700", &["gog_9", "gog_8"]), true, "t3").unwrap();
        assert_eq!((c.comptes_gog.len(), c.comptes_gog[1].jeux.len()), (2, 2));
        assert!(ajouter_compte_gog(None, &lu("", "pas-un-numero", &[]), false, "t").is_err());
    }

    /// Lot C4b : la session du navigateur du profil passe sur un compte déjà « ajouté » : une seule entrée Profil, et
    /// l'ancien compte Profil devient « ajouté ».
    #[test]
    fn le_navigateur_du_profil_passe_sur_un_compte_ajoute() {
        let (c, _) = appliquer_import_gog_profil(None, "", &lu("42", "4200", &["gog_1"]), "t1").unwrap();
        let c = ajouter_compte_gog(Some(c), &LectureCompteGog { nom: Some("Seb".into()), ..lu("", "7700", &["gog_9"]) }, false, "t2").unwrap();
        let (c, ch) = appliquer_import_gog_profil(Some(c), "42", &lu("77", "7700", &["gog_9", "gog_8"]), "t3").unwrap();
        assert_eq!(ch, ChangementGog { ancien_profil: Some("4200".into()), fondu: Some("7700".into()) });
        assert_eq!(
            c.comptes_gog.iter().map(|x| (x.id_galaxy.as_str(), x.id_site.as_str(), x.navigateur, x.nom.as_str())).collect::<Vec<_>>(),
            [("7700", "77", NavigateurGog::Profil, "Seb"), ("4200", "42", NavigateurGog::Ajoute, "")]
        );
        assert_eq!(c.comptes_gog.iter().filter(|x| x.navigateur == NavigateurGog::Profil).count(), 1);
        // Un compte ajouté AVANT que le numéro Galaxy du compte Profil soit connu : fondu à la lecture suivante.
        let (c, _) = appliquer_import_gog_profil(None, "", &lu("42", "", &["gog_1"]), "t1").unwrap();
        let c = ajouter_compte_gog(Some(c), &lu("", "4200", &["gog_1"]), false, "t2").unwrap();
        let (c, ch) = appliquer_import_gog_profil(Some(c), "42", &lu("42", "4200", &["gog_1"]), "t3").unwrap();
        assert_eq!((ch.ancien_profil, ch.fondu.as_deref(), c.comptes_gog.len()), (None, Some("4200"), 1));
    }

    /// Lot C4b : « Oublier B » retire le compte, garde les autres et les fiches de leurs jeux ; un jeu de B installé reste
    /// une carte (il vient du PC, `boutiques_pc::fusion_gog`).
    #[test]
    fn oublier_un_compte_garde_les_autres_et_les_jeux_installes() {
        let (c, _) = appliquer_import_gog_profil(None, "", &lu("42", "4200", &["gog_1"]), "t1").unwrap();
        let mut c = ajouter_compte_gog(Some(c), &LectureCompteGog { ids_lus: Some(vec!["9".into(), "10".into()]), ..lu("", "7700", &["gog_9"]) }, false, "t2").unwrap();
        fusionner_fiches_gog(&mut c, [("1", "Un"), ("9", "Neuf"), ("10", "Licence"), ("55", "Vieux")].into_iter().map(|(k, t)| (k.to_string(), crate::gog_galaxy::FicheGog { titre: t.into(), ..Default::default() })).collect());
        assert_eq!(c.fiches_gog.keys().map(String::as_str).collect::<Vec<_>>(), ["1", "10", "9"], "les fiches des deux comptes, pas celles d'aucun");
        let (c, oublie) = oublier_compte_gog(Some(c), "7700").unwrap();
        assert_eq!((oublie.id_galaxy.as_str(), oublie.navigateur), ("7700", NavigateurGog::Ajoute));
        assert_eq!(c.comptes_gog.len(), 1);
        assert_eq!(c.fiches_gog.keys().map(String::as_str).collect::<Vec<_>>(), ["1"]);
        assert!(oublier_compte_gog(Some(c.clone()), "7700").is_err(), "déjà oublié");
        // Le jeu de B installé sur ce PC reste une carte après l'oubli.
        let mut installe = jg("gog_9", "Neuf");
        installe.installe = true;
        let l = crate::boutiques_pc::fusion_gog(jeux_gog_reunis(&c.comptes_gog), &[installe], &[]);
        assert!(l.iter().any(|j| j.id == "gog_9" && j.installe));
        // Oublier le compte Profil : les anciens champs se vident (le numéro va dans `id_compte_precedent`).
        let (c, _) = oublier_compte_gog(Some(c), "4200").unwrap();
        assert_eq!((c.id_compte.as_str(), c.id_compte_precedent.as_str(), c.jeux.len(), c.comptes_gog.len()), ("", "42", 0, 0));
        assert!(comptes_gog_lus(&c).is_empty(), "jamais recréé depuis les anciens champs");
    }

    /// Lot C4b : les noms des secrets — le compte Profil garde l'ancien, un compte ajouté a le sien.
    #[test]
    fn chaque_compte_gog_a_son_secret() {
        assert_eq!(secret_compte_gog("p1", NavigateurGog::Profil, "4200"), "p1#gog-renouvellement");
        assert_eq!(secret_compte_gog("p1", NavigateurGog::Ajoute, "7700"), "p1#gog-renouvellement#7700");
        assert_eq!(nom_compte_gog("", 2), "Compte GOG 2", "son numéro rangé, pas sa place");
        assert_eq!(nom_compte_gog("", 0), "Compte GOG 1");
        assert_eq!(nom_compte_gog("Seb", 1), "Seb");
    }

    /// Lot C4b : le compte de GOG Galaxy ne s'affirme que s'il est sûr.
    #[test]
    fn galaxy_sur_un_autre_compte_seulement_si_on_le_sait() {
        let a = CompteGog { id_galaxy: "4200".into(), jeux: vec![jg("gog_1", "Un")], navigateur: NavigateurGog::Profil, numero: 1, ..Default::default() };
        let b = CompteGog { id_galaxy: "7700".into(), nom: "Seb".into(), jeux: vec![jg("gog_9", "Neuf")], navigateur: NavigateurGog::Ajoute, numero: 2, ..Default::default() };
        let sans_numero = CompteGog { id_site: "42".into(), jeux: vec![jg("gog_5", "Cinq")], navigateur: NavigateurGog::Ajoute, ..Default::default() };
        assert!(!galaxy_sur_un_autre_compte(&[a.clone()], ""), "compte de Galaxy inconnu");
        assert!(!galaxy_sur_un_autre_compte(&[], "7700"), "aucun compte chez Frogtend");
        assert!(!galaxy_sur_un_autre_compte(&[a.clone(), sans_numero.clone()], "7700"), "un compte au numéro Galaxy inconnu : dans le doute, rien");
        assert!(galaxy_sur_un_autre_compte(&[a.clone()], "7700"));
        assert!(!galaxy_sur_un_autre_compte(&[a.clone(), b.clone()], "7700"));
        let g = |compte: &str, cles: &[&str]| CompteGalaxyPc { compte: compte.into(), cles: cles.iter().map(|c| c.to_string()).collect() };
        // Un jeu du compte Profil quand Galaxy est sur le compte ajouté : on sait qu'il n'est pas chez Galaxy.
        assert_eq!(compte_hors_galaxy(&[a.clone(), b.clone()], &g("7700", &["gog_9"]), "1"), Some((String::new(), 1)));
        assert_eq!(compte_hors_galaxy(&[a.clone(), b.clone()], &g("7700", &[]), "9"), None, "un compte de Galaxy le possède");
        // La bibliothèque COMPLÈTE de Galaxy (jeux cachés compris) a le jeu : jamais « hors Galaxy ».
        assert_eq!(compte_hors_galaxy(&[a.clone()], &g("7700", &["gog_1"]), "1"), None);
        assert_eq!(compte_hors_galaxy(&[a.clone(), b.clone()], &g("4200", &["gog_1"]), "9"), Some(("Seb".into(), 2)));
        assert_eq!(compte_hors_galaxy(&[a.clone(), b.clone()], &g("", &[]), "9"), None, "compte de Galaxy inconnu");
        assert_eq!(compte_hors_galaxy(&[a.clone(), b.clone()], &g("7700", &[]), "12345"), None, "jeu d'aucun compte");
        assert_eq!(compte_hors_galaxy(&[a, b, sans_numero], &g("4200", &[]), "9"), None, "un compte au numéro Galaxy inconnu : rien d'affirmé");
    }

    /// Lot C4b : la base de Galaxy garde les jeux de DEUX comptes → rien d'affirmé, tout est lu ; UN compte → ses jeux.
    #[test]
    fn la_base_de_galaxy_se_filtre_sur_son_compte_s_il_est_seul() {
        let base = |lignes: &str| {
            let d = tempfile::tempdir().unwrap();
            let stock = d.path().join("storage");
            std::fs::create_dir_all(&stock).unwrap();
            let c = rusqlite::Connection::open(stock.join("galaxy-2.0.db")).unwrap();
            c.execute_batch(&format!(
                "CREATE TABLE LibraryReleases(id INTEGER, userId INTEGER, releaseKey TEXT);
                 CREATE TABLE ReleaseProperties(releaseKey TEXT, isDlc INTEGER, isVisibleInLibrary INTEGER, gameId TEXT);
                 CREATE TABLE GamePieceTypes(id INTEGER, type TEXT);
                 CREATE TABLE GamePieces(releaseKey TEXT, gamePieceTypeId INTEGER, userId INTEGER, value TEXT, languageId INTEGER);
                 CREATE TABLE GameTimes(userId INTEGER, releaseKey TEXT, minutesInGame INTEGER);
                 CREATE TABLE LastPlayedDates(userId INTEGER, gameReleaseKey TEXT, lastPlayedDate TEXT);
                 CREATE TABLE InstalledBaseProducts(productId INTEGER, installationPath TEXT);
                 CREATE TABLE InstalledExternalProducts(id INTEGER, platformId INTEGER, productId TEXT);
                 {lignes}"
            ))
            .unwrap();
            drop(c);
            let r = lire_galaxy_et_compte(&stock, &d.path().join("copie")).unwrap();
            (d, r)
        };
        let (_d, (l, compte)) = base("INSERT INTO LibraryReleases VALUES (1, 48628349957132247, 'gog_10'), (2, 48628349957132247, 'gog_11');");
        assert_eq!((l.len(), compte.as_deref()), (2, Some("48628349957132247")));
        let (_d, (l, compte)) = base("INSERT INTO LibraryReleases VALUES (1, 5, 'gog_10'), (2, 6, 'gog_11'), (3, 6, 'gog_12');");
        assert_eq!((l.len(), compte), (3, None), "deux comptes : rien d'affirmé, comportement d'avant");
        let (_d, (l, compte)) = base("");
        assert_eq!((l.len(), compte), (0, None));
        // Des jeux sans compte (userId vide ou 0) à côté d'un compte : rien d'affirmé, rien de filtré.
        let (_d, (l, compte)) = base("INSERT INTO LibraryReleases VALUES (1, 5, 'gog_10'), (2, NULL, 'gog_11'), (3, 0, 'gog_12');");
        assert_eq!((l.len(), compte), (3, None));
        // La relecture du compte de Galaxy (installer, lancer) : TOUTES ses clés, jeux cachés et DLC compris ; une vieille
        // copie de son journal (-wal) n'est jamais mêlée.
        let d = tempfile::tempdir().unwrap();
        let stock = d.path().join("storage");
        std::fs::create_dir_all(&stock).unwrap();
        let c = rusqlite::Connection::open(stock.join("galaxy-2.0.db")).unwrap();
        c.execute_batch(
            "CREATE TABLE LibraryReleases(id INTEGER, userId INTEGER, releaseKey TEXT);
             CREATE TABLE ReleaseProperties(releaseKey TEXT, isDlc INTEGER, isVisibleInLibrary INTEGER, gameId TEXT);
             INSERT INTO LibraryReleases VALUES (1, 7700, 'gog_10'), (2, 7700, 'gog_11');
             INSERT INTO ReleaseProperties VALUES ('gog_11', 0, 0, '');",
        )
        .unwrap();
        drop(c);
        let travail = d.path().join("copie");
        std::fs::create_dir_all(&travail).unwrap();
        std::fs::write(travail.join("galaxy-2.0.db-wal"), b"vieux journal").unwrap();
        let g = compte_galaxy_du_pc(&stock, &travail).unwrap();
        assert_eq!(g.compte, "7700");
        assert_eq!(g.cles.iter().map(String::as_str).collect::<Vec<_>>(), ["gog_10", "gog_11"], "le jeu caché compte aussi");
        assert!(!travail.exists(), "la copie est effacée");
        // Une base sans la table : rien d'affirmé, et la lecture des jeux ne casse pas pour autant.
        let stock2 = d.path().join("vide");
        std::fs::create_dir_all(&stock2).unwrap();
        rusqlite::Connection::open(stock2.join("galaxy-2.0.db")).unwrap().execute_batch("CREATE TABLE Autre(x INTEGER);").unwrap();
        assert_eq!(compte_galaxy_du_pc(&stock2, &d.path().join("copie2")), None);
    }

    /// Revue du lot C4b : une source « gog » écrite sous une forme que cette version ne lit pas n'est jamais écrasée.
    #[test]
    fn une_source_illisible_n_est_jamais_remplacee() {
        let d = tempfile::tempdir().unwrap();
        let brut = r#"{"gog":{"compte":"GOG","steamid":"","maj_le":"","jeux":[],"comptes_gog":[{"id_galaxy":"1","navigateur":"futur"}]},"steam":{"compte":"x","steamid":"1","maj_le":"","jeux":[]}}"#;
        std::fs::write(d.path().join("boutiques.json"), brut).unwrap();
        assert!(modifier_source(d.path(), "gog", |_| Ok(Some(CompteBoutique::default()))).is_err());
        assert_eq!(std::fs::read_to_string(d.path().join("boutiques.json")).unwrap(), brut, "rien n'a bougé");
        // Les autres sources s'écrivent toujours.
        modifier_source(d.path(), "steam", |c| Ok(c)).unwrap();
        assert!(std::fs::read_to_string(d.path().join("boutiques.json")).unwrap().contains("\"futur\""));
    }

    /// Revue du lot C4b : un fichier 0.218 (compte Profil sans numéro Galaxy) puis un changement de compte : l'ancien
    /// reçoit le numéro retrouvé par son jeton ; revenu dans le navigateur, il est fondu (jamais un doublon éternel) ;
    /// les numéros « Compte GOG n » restent stables.
    #[test]
    fn un_compte_de_la_0_218_change_de_compte_sans_doublon() {
        let c = CompteBoutique { compte: "GOG".into(), id_compte: "42".into(), jeux: vec![jg("gog_1", "Un")], ..Default::default() };
        let (c, ch) = appliquer_import_gog_profil(Some(c), "42", &LectureCompteGog { ancien_id_galaxy: Some("4200".into()), ..lu("77", "7700", &["gog_9"]) }, "t1").unwrap();
        assert_eq!(ch.ancien_profil.as_deref(), Some("4200"));
        assert_eq!(c.comptes_gog.iter().map(|x| (x.id_galaxy.as_str(), x.navigateur, x.numero)).collect::<Vec<_>>(), [("7700", NavigateurGog::Profil, 2), ("4200", NavigateurGog::Ajoute, 1)]);
        // Sans numéro retrouvé (GOG injoignable n'arrive pas ici ; jeton absent) : l'ancien reste « site-42 »…
        let c0 = CompteBoutique { compte: "GOG".into(), id_compte: "42".into(), jeux: vec![jg("gog_1", "Un")], ..Default::default() };
        let (c0, _) = appliquer_import_gog_profil(Some(c0), "42", &lu("77", "7700", &["gog_9"]), "t1").unwrap();
        assert_eq!(c0.comptes_gog[1].cle(), "site-42");
        // … et revenu dans le navigateur, il est fondu par le numéro du site.
        let (c0, ch) = appliquer_import_gog_profil(Some(c0), "77", &lu("42", "4200", &["gog_1"]), "t2").unwrap();
        assert_eq!(ch.fondu, None, "aucun jeton à oublier pour un compte sans numéro Galaxy");
        assert_eq!(c0.comptes_gog.iter().map(|x| (x.id_galaxy.as_str(), x.navigateur, x.numero)).collect::<Vec<_>>(), [("4200", NavigateurGog::Profil, 1), ("7700", NavigateurGog::Ajoute, 2)]);
        // Le numéro « Compte GOG n » d'un compte oublié n'est pas repris par les autres.
        let (c0, _) = oublier_compte_gog(Some(c0), "4200").unwrap();
        assert_eq!(c0.comptes_gog[0].numero, 2);
        assert!(c0.navigateur_gog_oublie, "le compte du navigateur oublié est noté");
        let (c0, _) = appliquer_import_gog_profil(Some(c0), "", &lu("42", "4200", &[]), "t3").unwrap();
        assert!(!c0.navigateur_gog_oublie, "une lecture du navigateur (après reconnexion) le retire");
    }

    /// Une petite base « façon Galaxy » : les DLC et jeux cachés sont écartés, les installés repérés.
    #[test]
    fn galaxy_se_lit_sur_une_copie_et_ecarte_les_dlc() {
        let d = tempfile::tempdir().unwrap();
        let stock = d.path().join("storage");
        std::fs::create_dir_all(&stock).unwrap();
        let c = rusqlite::Connection::open(stock.join("galaxy-2.0.db")).unwrap();
        c.execute_batch(
            "CREATE TABLE LibraryReleases(id INTEGER, userId INTEGER, releaseKey TEXT);
             CREATE TABLE ReleaseProperties(releaseKey TEXT, isDlc INTEGER, isVisibleInLibrary INTEGER, gameId TEXT);
             CREATE TABLE GamePieceTypes(id INTEGER, type TEXT);
             CREATE TABLE GamePieces(releaseKey TEXT, gamePieceTypeId INTEGER, userId INTEGER, value TEXT, languageId INTEGER);
             CREATE TABLE GameTimes(userId INTEGER, releaseKey TEXT, minutesInGame INTEGER);
             CREATE TABLE LastPlayedDates(userId INTEGER, gameReleaseKey TEXT, lastPlayedDate TEXT);
             CREATE TABLE InstalledBaseProducts(productId INTEGER, installationPath TEXT);
             CREATE TABLE InstalledExternalProducts(id INTEGER, platformId INTEGER, productId TEXT);
             INSERT INTO GamePieceTypes VALUES (460, 'originalTitle'), (457, 'originalImages');
             INSERT INTO LibraryReleases VALUES (1, 1, 'gog_10'), (2, 1, 'epic_Fortnite'), (3, 1, 'gog_11'), (4, 1, 'steam_620');
             INSERT INTO ReleaseProperties VALUES ('gog_10', 0, 1, ''), ('epic_Fortnite', 0, 1, ''), ('gog_11', 1, 1, ''), ('steam_620', 0, 0, '');
             INSERT INTO GamePieces VALUES ('gog_10', 460, 1, '{\"title\":\"Cyberpunk 2077\"}', 0), ('gog_10', 457, 1, '{\"verticalCover\":\"https://images.gog.com/a.webp\"}', 0),
                                           ('epic_Fortnite', 460, 1, '{\"title\":\"Fortnite\"}', 0);
             INSERT INTO GameTimes VALUES (1, 'gog_10', 92);
             INSERT INTO LastPlayedDates VALUES (1, 'gog_10', '2022-04-29 20:23:22');
             INSERT INTO InstalledBaseProducts VALUES (10, 'D:\\Jeux\\Cyberpunk');
             INSERT INTO InstalledExternalProducts VALUES (5, 3, 'Fortnite');
             CREATE TABLE UserAchievements(gameReleaseKey TEXT, userId INTEGER, apikey TEXT, unlockTime INTEGER, isUnlocked INTEGER);
             INSERT INTO UserAchievements VALUES ('gog_10', 1, 'a', 1, 1), ('gog_10', 1, 'b', 0, 0), ('gog_10', 1, 'c', 0, 0);",
        )
        .unwrap();
        drop(c);
        let l = lire_galaxy(&stock, &d.path().join("copie")).unwrap();
        assert_eq!(l.iter().map(|j| j.nom.as_str()).collect::<Vec<_>>(), ["Cyberpunk 2077", "Fortnite"], "DLC et caché écartés");
        assert!(l[0].installe && l[0].minutes == 92 && l[0].derniere == 1_651_263_802);
        assert_eq!(l[0].dossier, None, "un dossier noté par Galaxy mais absent du disque n'est pas gardé");
        assert_eq!(l[1].dossier, None, "Galaxy ne note aucun chemin pour les autres boutiques");
        assert_eq!(l[0].image.as_deref(), Some("https://images.gog.com/a.webp"));
        assert!(l[1].installe && l[1].plateforme == "epic");
        assert_eq!(l[0].succes, Some((1, 3)), "succès lus dans Galaxy");
        assert_eq!(l[1].succes, None, "Epic : Galaxy ne les a pas");
        assert!(stock.join("galaxy-2.0.db").is_file(), "l'original n'est pas touché");
    }

    #[test]
    fn les_succes_galaxy_en_detail_francais_s_il_est_vraiment_traduit_sinon_anglais() {
        // La forme réelle des tables (relevé du 21/10 sur la base de Venkman, lue sur une copie).
        let d = tempfile::tempdir().unwrap();
        let stock = d.path().join("storage");
        std::fs::create_dir_all(&stock).unwrap();
        let c = rusqlite::Connection::open(stock.join("galaxy-2.0.db")).unwrap();
        c.execute_batch(
            "CREATE TABLE Achievements(gameReleaseKey TEXT, apikey TEXT, backendId TEXT, imageUnlockedUrl TEXT, imageLockedUrl TEXT, isVisible INTEGER, rarity REAL, raritySlug TEXT);
             CREATE TABLE UserAchievements(gameReleaseKey TEXT, userId INTEGER, apikey TEXT, unlockTime TEXT, isUnlocked INTEGER);
             CREATE TABLE LocalizedAchievements(gameReleaseKey TEXT, apikey TEXT, name TEXT, description TEXT, languageId INTEGER, isLocalized INTEGER);
             CREATE TABLE Languages(id INTEGER, name TEXT, code TEXT);
             INSERT INTO Languages VALUES (16, 'English', 'en-US'), (24, 'French', 'fr-FR'), (12, 'German', 'de-DE');
             INSERT INTO Achievements VALUES ('gog_1', 'a', '1', 'https://images.gog.com/a.png', 'https://images.gog.com/a-gris.png', 1, 3.81, 'rare'),
                                             ('gog_1', 'b', '2', 'https://ailleurs.example/b.png', NULL, 0, 80.5, 'common'),
                                             ('gog_2', 'z', '3', NULL, NULL, 1, NULL, NULL);
             INSERT INTO UserAchievements VALUES ('gog_1', 7, 'a', '2022-04-29 20:23:22', 1), ('gog_1', 7, 'b', NULL, 0);
             INSERT INTO LocalizedAchievements VALUES ('gog_1', 'a', 'Legendary Fossil', 'Caught a Starlight Nautilus', 24, 0),
                                                      ('gog_1', 'a', 'Legendary Fossil', 'Caught a Starlight Nautilus', 16, 0),
                                                      ('gog_1', 'a', 'Legendärer', 'Gefangen', 12, 1),
                                                      ('gog_1', 'b', 'Fossile', 'Un vrai texte français', 24, 1),
                                                      ('gog_1', 'b', 'Fossil', 'English', 16, 0);",
        )
        .unwrap();
        drop(c);
        let travail = d.path().join("copie");
        let l = succes_galaxy(&stock, &travail, "gog_1").unwrap().unwrap();
        let a = l.iter().find(|s| s.id == "a").unwrap();
        assert_eq!((a.nom.as_str(), a.en_anglais, a.obtenu, a.quand, a.rarete), ("Legendary Fossil", true, true, Some(1_651_263_802), Some(3.81)), "« français » non traduit : l'anglais, dit");
        assert_eq!(a.icone_grise.as_deref(), Some("https://images.gog.com/a-gris.png"));
        let b = l.iter().find(|s| s.id == "b").unwrap();
        assert_eq!((b.description.as_str(), b.en_anglais, b.obtenu, b.secret), ("Un vrai texte français", false, false, true));
        assert_eq!(b.icone, None, "un hôte d'image inconnu n'est jamais gardé");
        assert!(!travail.exists(), "la copie de la base ne reste pas");
        assert_eq!(succes_galaxy(&stock, &travail, "gog_2").unwrap(), None, "sans texte : rien");
        assert_eq!(succes_galaxy(&stock, &travail, "x' OR 1=1 --").unwrap(), None, "clé douteuse : rien lu");
        assert!(stock.join("galaxy-2.0.db").is_file());
    }

    #[test]
    fn la_fiche_steam_se_lit_en_francais() {
        // Relevé réel (XCOM 2, 21/10), réduit.
        let v: Value = serde_json::from_str(include_str!("../fixtures/simule/steam-fiche-268500.json")).unwrap();
        let f = lire_fiche_steam(&v, "268500").unwrap();
        assert!(f.resume.as_deref().unwrap().starts_with("XCOM"));
        assert!(!f.en_anglais);
        assert_eq!(f.genres, ["Stratégie"]);
        assert_eq!(f.developpeurs[0], "Firaxis Games");
        assert_eq!(f.annee, Some(2016));
        assert_eq!(f.captures.len(), 2);
        assert!(f.captures[0].0.contains("600x338") && image_autorisee(&f.captures[0].1));
        assert!(f.fond.as_deref().is_some_and(image_autorisee));
        assert_eq!(lire_fiche_steam(&serde_json::json!({"1": {"success": false}}), "1"), None);
        assert_eq!(texte_clair("Rock &amp; Roll &quot;x&quot;"), "Rock & Roll \"x\"");
    }

    #[test]
    fn un_manifeste_epic_donne_un_jeu_jamais_unreal_ni_un_dlc() {
        // La forme réelle (TerraScape, relevé du 21/10), réduite à ce que Frogtend lit.
        let terra = r#"{"FormatVersion":0,"bIsIncompleteInstall":false,"AppName":"6ef17d9f63c143ea88a4a4bcbcf1e91c","DisplayName":"TerraScape",
            "InstallLocation":"D:\\LaunchBox\\Games\\Epic\\Epic Games\\Games\\TerraScape","LaunchExecutable":"EOSBootstrapper.exe","LaunchCommand":"",
            "CatalogNamespace":"bfb80547289b477193ce3b14d83cd292","CatalogItemId":"e6ebabecf20740628e42d3894ed872c5","AppCategories":["games","applications"],
            "MainGameAppName":"","bCanRunOffline":false}"#;
        let j = lire_manifeste_epic(terra).unwrap();
        assert_eq!((j.app.as_str(), j.nom.as_str()), ("6ef17d9f63c143ea88a4a4bcbcf1e91c", "TerraScape"));
        // EXACTEMENT l'adresse du raccourci qu'Epic a posé lui-même sur le Bureau.
        assert_eq!(
            adresse_epic(&j),
            "com.epicgames.launcher://apps/bfb80547289b477193ce3b14d83cd292%3Ae6ebabecf20740628e42d3894ed872c5%3A6ef17d9f63c143ea88a4a4bcbcf1e91c?action=launch&silent=true"
        );
        let avec = |a: &str, b: &str| terra.replace(a, b);
        assert!(lire_manifeste_epic(&avec(r#"["games","applications"]"#, r#"["engines"]"#)).is_none(), "Unreal Engine");
        assert!(lire_manifeste_epic(&avec(r#""MainGameAppName":"""#, r#""MainGameAppName":"Autre""#)).is_none(), "un DLC");
        assert!(lire_manifeste_epic(&avec(r#""bIsIncompleteInstall":false"#, r#""bIsIncompleteInstall":true"#)).is_none(), "installation pas finie");
        assert!(lire_manifeste_epic(&avec(r#""LaunchExecutable":"EOSBootstrapper.exe""#, r#""LaunchExecutable":"""#)).is_none());
        assert!(lire_manifeste_epic(&avec("6ef17d9f63c143ea88a4a4bcbcf1e91c", "x/../y")).is_none(), "identifiant douteux");
    }

    #[test]
    fn epic_reprend_le_dossier_de_launcher_installed_quand_le_manifeste_a_un_ancien_dossier() {
        let d = tempfile::tempdir().unwrap();
        // Les données comme Epic les range (relevé du 18/11) : <…>\Epic\EpicGamesLauncher\Data et, à côté,
        // <…>\Epic\UnrealEngineLauncher\LauncherInstalled.dat.
        let donnees = d.path().join("Epic").join("EpicGamesLauncher").join("Data");
        std::fs::create_dir_all(donnees.join("Manifests")).unwrap();
        std::fs::create_dir_all(d.path().join("Epic").join("UnrealEngineLauncher")).unwrap();
        let nouveau = d.path().join("Jeux").join("Terra");
        std::fs::create_dir_all(&nouveau).unwrap();
        let dat = serde_json::json!({"InstallationList": [
            {"InstallLocation": nouveau.to_string_lossy(), "NamespaceId": "ns", "ItemId": "it", "AppName": "Terra"},
            {"InstallLocation": "D:\\unreal\\UE_5.7", "AppName": "UE_5.7"},
            {"InstallLocation": "x", "AppName": "a/../b"}]});
        std::fs::write(d.path().join("Epic/UnrealEngineLauncher/LauncherInstalled.dat"), dat.to_string()).unwrap();
        assert_eq!(liste_installations_epic(&donnees), Some(d.path().join("Epic/UnrealEngineLauncher/LauncherInstalled.dat")));
        let repli = repli_epic(&donnees);
        assert_eq!(repli.len(), 2, "un AppName douteux est écarté");
        let manifeste = |dossier: &str| {
            serde_json::json!({"bIsIncompleteInstall": false, "AppName": "Terra", "DisplayName": "TerraScape", "InstallLocation": dossier,
                "LaunchExecutable": "x.exe", "CatalogNamespace": "ns", "CatalogItemId": "it", "AppCategories": ["games"], "MainGameAppName": ""})
            .to_string()
        };
        std::fs::write(donnees.join("Manifests/A.item"), manifeste(r"Z:\Ancien\Terra")).unwrap();
        let l = installes_epic(&donnees.join("Manifests"), &repli);
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].dossier, nouveau, "le dossier de LauncherInstalled.dat remplace un dossier disparu");
        assert!(installes_epic(&donnees.join("Manifests"), &Default::default()).is_empty(), "sans repli : dossier absent, rien");
        assert!(lire_installations_epic("pas du json").is_empty());
    }

    #[test]
    fn un_chemin_se_compare_sous_toutes_ses_formes() {
        assert_eq!(cle_chemin(Path::new("D:/Jeux/Far Cry/")), r"d:\jeux\far cry");
        assert_eq!(cle_chemin(Path::new(r"\\?\D:\Jeux")), r"d:\jeux");
        assert_eq!(cle_chemin(Path::new(r"\\?\UNC\Egon\Jeux\X")), r"\\egon\jeux\x", "un chemin réseau long");
        assert_eq!(cle_chemin(Path::new("//Egon/Jeux/X")), r"\\egon\jeux\x");
        let m = lire_manifeste_steam("\"appid\" \"620\"\n\"name\" \"Portal 2\"\n\"StateFlags\" \"4\"\n\"installdir\" \"Portal 2\"").unwrap();
        assert_eq!(m.nom.as_deref(), Some("Portal 2"));
    }

    /// RÉEL, lecture seule (lot D) : la fiche d'un jeu d'après la vraie base de Galaxy (copie effacée ensuite).
    /// `FROGTEND_GALAXY_JEU=gog_1209025141 cargo test essai_fiche_galaxy -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn essai_fiche_galaxy() {
        let cle = std::env::var("FROGTEND_GALAXY_JEU").unwrap();
        let d = tempfile::tempdir().unwrap();
        let f = fiche_galaxy(&dossier_galaxy().expect("Galaxy"), &d.path().join("copie"), &cle).unwrap();
        println!("{f:?}");
        assert!(!d.path().join("copie").exists());
    }

    /// RÉEL, lecture seule (A6d) : les jeux Epic installés sur ce PC et leur adresse de lancement (rien n'est lancé).
    /// `cargo test essai_jeux_epic -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn essai_jeux_epic() {
        for j in installes_epic_du_pc().expect("Epic") {
            println!("{} → {} ({})", j.nom, adresse_epic(&j), j.dossier.display());
        }
    }

    /// RÉEL, lecture seule : les succès d'un jeu de la vraie base de Galaxy (sur une copie effacée ensuite).
    /// `FROGTEND_GALAXY_JEU=gog_1084933262 cargo test essai_succes_galaxy -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn essai_succes_galaxy() {
        let cle = std::env::var("FROGTEND_GALAXY_JEU").unwrap();
        let d = tempfile::tempdir().unwrap();
        let l = succes_galaxy(&dossier_galaxy().expect("Galaxy"), &d.path().join("copie"), &cle).unwrap().unwrap_or_default();
        let obtenus = l.iter().filter(|s| s.obtenu).count();
        println!("{cle} : {} succès, {obtenus} obtenus, {} en anglais, {} secrets, {} avec rareté, {} avec icône", l.len(), l.iter().filter(|s| s.en_anglais).count(), l.iter().filter(|s| s.secret).count(), l.iter().filter(|s| s.rarete.is_some()).count(), l.iter().filter(|s| s.icone.is_some()).count());
        if let Some(s) = l.iter().find(|s| s.obtenu) {
            println!("exemple obtenu : « {} » — {} ({:?} %, quand {:?})", s.nom, s.description, s.rarete, s.quand);
        }
    }

    #[tokio::test]
    async fn la_jaquette_steam_vient_du_cache_et_un_identifiant_douteux_est_refuse() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("steam")).unwrap();
        std::fs::write(d.path().join("steam").join("620.jpg"), b"image-en-cache").unwrap();
        assert_eq!(image_steam(d.path(), "620").await.unwrap(), b"image-en-cache");
        assert!(image_steam(d.path(), "../../secret").await.is_none());
        assert!(adresses_image_steam("620")[0].ends_with("/620/library_600x900.jpg"));
    }

    #[test]
    fn le_compte_s_enregistre_sans_secret_et_s_oublie() {
        let d = tempfile::tempdir().unwrap();
        let c = CompteBoutique { compte: "Seb".into(), steamid: "1".into(), maj_le: "0".into(), jeux: vec![], ..Default::default() };
        ecrire(d.path(), Some(&c)).unwrap();
        assert_eq!(lire(d.path()), Some(c));
        assert!(!std::fs::read_to_string(d.path().join("boutiques.json")).unwrap().to_lowercase().contains("key"));
        ecrire(d.path(), None).unwrap();
        assert_eq!(lire(d.path()), None);
    }

    #[test]
    fn la_cle_d_api_et_le_compte_connecte_ne_se_melangent_jamais() {
        let (a, b) = ("76561197960287930", "76561198000000000");
        // Fichier d'avant C1 réglé avec la clé : la clé est celle du steamid.
        let ancien = CompteBoutique { steamid: a.into(), ..Default::default() };
        assert_eq!(steamid_de_la_cle(&ancien), a);
        assert!(!cle_d_un_autre_compte(&ancien));
        // Connecté par la page, sans clé connue : rien à refuser.
        assert!(!cle_d_un_autre_compte(&CompteBoutique { steamid: b.into(), connecte_le: "t".into(), ..Default::default() }));
        // Clé de A, connecté à B : refusé ; connecté à A : permis.
        assert!(cle_d_un_autre_compte(&CompteBoutique { steamid: b.into(), connecte_le: "t".into(), steamid_cle: a.into(), ..Default::default() }));
        assert!(!cle_d_un_autre_compte(&CompteBoutique { steamid: a.into(), connecte_le: "t".into(), steamid_cle: a.into(), ..Default::default() }));
    }

    #[test]
    fn un_fichier_des_comptes_illisible_n_est_jamais_ecrase() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("boutiques.json");
        for abime in [&b"{pas du json"[..], b"[1, 2]", b"\"texte\""] {
            std::fs::write(&f, abime).unwrap();
            assert!(noter_connexion_steam(d.path(), "76561197960287930", "t").is_err());
            assert!(ecrire_source(d.path(), "galaxy", None).is_err());
            assert_eq!(std::fs::read(&f).unwrap(), abime, "le fichier abîmé est resté tel quel");
        }
    }

    #[test]
    fn une_ecriture_garde_les_autres_sources_et_ne_laisse_pas_de_fichier_provisoire() {
        let d = tempfile::tempdir().unwrap();
        ecrire_source(d.path(), "galaxy", Some(&CompteBoutique { compte: "GOG Galaxy".into(), ..Default::default() })).unwrap();
        ecrire_source(d.path(), "retroachievements", Some(&CompteBoutique { compte: "seb".into(), ..Default::default() })).unwrap();
        noter_connexion_steam(d.path(), "76561197960287930", "t").unwrap();
        assert_eq!(lire_source(d.path(), "galaxy").unwrap().compte, "GOG Galaxy");
        assert_eq!(lire_source(d.path(), "retroachievements").unwrap().compte, "seb");
        assert!(!d.path().join("boutiques.json.provisoire").exists());
        // Une modification refusée n'écrit rien.
        let avant = std::fs::read(d.path().join("boutiques.json")).unwrap();
        assert!(modifier_source(d.path(), "steam", |_| Err(Erreur::Refus("non".into()))).is_err());
        assert_eq!(std::fs::read(d.path().join("boutiques.json")).unwrap(), avant);
    }

    #[test]
    fn deux_modifications_en_meme_temps_ne_se_defont_pas() {
        let d = tempfile::tempdir().unwrap();
        let chemin = d.path().to_path_buf();
        // Chaque fil relit, attend un peu (comme un import qui attend le réseau), puis ajoute sa source.
        let fils: Vec<_> = (0..8)
            .map(|i| {
                let chemin = chemin.clone();
                std::thread::spawn(move || {
                    modifier_source(&chemin, &format!("source{i}"), |_| {
                        std::thread::sleep(std::time::Duration::from_millis(20));
                        Ok(Some(CompteBoutique { compte: format!("c{i}"), ..Default::default() }))
                    })
                    .unwrap();
                })
            })
            .collect();
        for f in fils {
            f.join().unwrap();
        }
        for i in 0..8 {
            assert_eq!(lire_source(&chemin, &format!("source{i}")).unwrap().compte, format!("c{i}"));
        }
    }

    #[test]
    fn la_connexion_steam_ne_garde_que_le_steamid() {
        let d = tempfile::tempdir().unwrap();
        let id = "76561197960287930";
        // Rien avant : le compte est créé avec le seul steamid.
        assert!(!noter_connexion_steam(d.path(), id, "2026-11-19T12:00:00").unwrap(), "première connexion : pas un changement de compte");
        let c = lire(d.path()).unwrap();
        assert_eq!((c.steamid.as_str(), c.compte.as_str(), c.connecte_le.as_str()), (id, id, "2026-11-19T12:00:00"));
        assert!(connecte_par_la_page(&c));
        // Un compte réglé à la main (clé d'API) n'est pas « connecté par la page ».
        assert!(!connecte_par_la_page(&CompteBoutique { steamid: id.into(), ..Default::default() }));
        assert!(!connecte_par_la_page(&CompteBoutique { steamid: "1".into(), connecte_le: "t".into(), ..Default::default() }));
        let brut = std::fs::read_to_string(d.path().join("boutiques.json")).unwrap();
        assert!(!brut.contains("token") && !brut.contains("cookie"), "{brut}");
        // Le même compte, déjà importé avec la clé : la liste et le nom donné restent.
        let jeu: JeuBoutique = serde_json::from_value(serde_json::json!({"id": "620", "nom": "Portal 2", "minutes": 0, "derniere": 0})).unwrap();
        ecrire(d.path(), Some(&CompteBoutique { compte: "NimaiTakahashi".into(), steamid: id.into(), maj_le: "x".into(), jeux: vec![jeu.clone()], ..Default::default() })).unwrap();
        assert!(!noter_connexion_steam(d.path(), id, "t2").unwrap(), "le même compte");
        let c = lire(d.path()).unwrap();
        assert_eq!((c.compte.as_str(), c.jeux.len(), c.connecte_le.as_str()), ("NimaiTakahashi", 1, "t2"));
        // Un AUTRE compte : l'ancienne liste ne vaut plus.
        assert!(noter_connexion_steam(d.path(), "76561198000000000", "t3").unwrap(), "un autre compte : changement");
        let c = lire(d.path()).unwrap();
        assert_eq!((c.steamid.as_str(), c.compte.as_str(), c.jeux.len()), ("76561198000000000", "76561198000000000", 0));
        // Un steamid invalide n'écrit rien.
        assert!(noter_connexion_steam(d.path(), "123", "t4").is_err());
        assert_eq!(lire(d.path()).unwrap().connecte_le, "t3");
        // Le compte de la clé est gardé à part : la clé de 7656119796… n'ira jamais avec le compte 7656119800….
        assert_eq!(c.steamid_cle, id);
        assert!(cle_d_un_autre_compte(&c));
        // Un ancien fichier (sans `connecte_le`) se lit toujours.
        std::fs::write(d.path().join("boutiques.json"), r#"{"steam":{"compte":"a","steamid":"1","maj_le":"","jeux":[]}}"#).unwrap();
        assert_eq!(lire(d.path()).unwrap().connecte_le, "");
    }

    /// Encode en base64url sans remplissage (pour fabriquer des JWT de test).
    fn en_base64url(o: &[u8]) -> String {
        const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let mut s = String::new();
        for b in o.chunks(3) {
            let n = (b[0] as u32) << 16 | (*b.get(1).unwrap_or(&0) as u32) << 8 | *b.get(2).unwrap_or(&0) as u32;
            for i in 0..=b.len() {
                s.push(A[((n >> (18 - 6 * i)) & 63) as usize] as char);
            }
        }
        s
    }

    #[test]
    fn le_resume_des_cookies_ne_montre_aucune_valeur() {
        let l = vec![
            ("steamLoginSecure".to_string(), "SECRET123".to_string(), Some("store.steampowered.com".to_string())),
            ("browserid".to_string(), "AUTRE456".to_string(), Some(".store.steampowered.com".to_string())),
            ("steamLoginSecure".to_string(), "SECRET789".to_string(), Some("steamcommunity.com".to_string())),
        ];
        let r = resume_cookies_steam(&l, true);
        assert_eq!(r, "boutique chargée, 3 cookie(s) lu(s) dont 2 de la boutique, cookie de connexion présent");
        assert!(!r.contains("SECRET") && !r.contains("AUTRE") && !r.contains("browserid"));
        assert!(resume_cookies_steam(&l[1..], false).ends_with("cookie de connexion ABSENT"));
    }

    fn ck(nom: &str, domaine: &str) -> CookieSansValeur {
        CookieSansValeur { nom: nom.into(), domaine: domaine.into(), chemin: "/".into() }
    }

    #[test]
    fn seuls_les_domaines_de_steam_sont_reconnus() {
        for d in [".steampowered.com", "steampowered.com", "store.steampowered.com", "login.steampowered.com", ".steamcommunity.com", "STEAMCOMMUNITY.COM"] {
            assert!(domaine_steam(d), "{d}");
        }
        for d in ["", ".", "epicgames.com", ".epicgames.com", "gog.com", "evilsteampowered.com", "steampowered.com.evil.fr", "playstation.com"] {
            assert!(!domaine_steam(d), "{d}");
        }
    }

    /// Lot C4b : seuls les cookies de gog.com s'effacent (Epic, Amazon, PlayStation partagent le navigateur du profil).
    #[test]
    fn seuls_les_domaines_de_gog_sont_reconnus() {
        for d in [".gog.com", "gog.com", "www.gog.com", "login.gog.com", ".auth.gog.com", "WWW.GOG.COM"] {
            assert!(domaine_gog(d), "{d}");
        }
        for d in ["", ".", "evilgog.com", "gog.com.evil.fr", "epicgames.com", "gog-statics.com", "amazon.com", "playstation.com"] {
            assert!(!domaine_gog(d), "{d}");
        }
    }

    #[test]
    fn la_deconnexion_steam_se_prouve_sans_aucun_cookie_de_connexion() {
        assert!(cookie_de_connexion_steam("steamLoginSecure") && cookie_de_connexion_steam("steamRefresh_steam"));
        assert!(!cookie_de_connexion_steam("sessionid") && !cookie_de_connexion_steam("browserid"));
        // Il reste des cookies Steam ordinaires et la connexion Epic : la déconnexion Steam est prouvée.
        let propre = [ck("sessionid", "store.steampowered.com"), ck("EPIC_SSO", ".epicgames.com"), ck("steamLoginSecure", "evilsteampowered.com")];
        assert!(connexion_steam_effacee(&propre));
        assert!(connexion_steam_effacee(&[]));
        // Un seul cookie de connexion qui reste, sur n'importe quel domaine Steam : pas prouvé.
        assert!(!connexion_steam_effacee(&[ck("steamRefresh_steam", "login.steampowered.com")]));
        assert!(!connexion_steam_effacee(&[ck("steamLoginSecure", ".steamcommunity.com")]));
        let r = resume_cookies_steam_restants(&[ck("sessionid", "store.steampowered.com"), ck("steamLoginSecure", ".steampowered.com"), ck("EPIC_SSO", ".epicgames.com")]);
        assert_eq!(r, "2 cookie(s) Steam, dont 1 de connexion");
    }

    fn jwt(charge: Value) -> String {
        format!("{}.{}.{}", en_base64url(br#"{"typ":"JWT","alg":"EdDSA"}"#), en_base64url(charge.to_string().as_bytes()), en_base64url(b"signature"))
    }

    #[test]
    fn le_base64url_se_decode_avec_ou_sans_remplissage() {
        for t in [&b""[..], b"f", b"fo", b"foo", b"foob", b"fooba", b"foobar", &[0xfb, 0xff, 0xfe]] {
            assert_eq!(base64url(&en_base64url(t)).as_deref(), Some(t));
        }
        assert_eq!(base64url("Zm9vYg==").as_deref(), Some(&b"foob"[..]));
        assert_eq!(base64url("Zm9v!g"), None);
        assert_eq!(base64url("Z"), None);
    }

    #[test]
    fn le_jeton_steam_doit_etre_celui_du_compte_connecte() {
        let (a, b) = ("76561197960287930", "76561198000000000");
        let maintenant = 1_800_000_000;
        // Forme réelle de la charge d'un jeton de la boutique (valeurs inventées).
        let bon = jwt(json!({"iss": "r:0F1B", "sub": a, "aud": ["web:store"], "exp": maintenant + 3600, "nbf": 1, "iat": 1, "per": 0}));
        assert!(verifier_jeton_steam(&bon, a, maintenant).is_ok());
        // Un autre compte : refusé, avec un message clair, jamais le jeton.
        let e = format!("{:?}", verifier_jeton_steam(&bon, b, maintenant).unwrap_err());
        assert!(e.contains("n'est pas celui connecté") && !e.contains(&bon), "{e}");
        // `sub` en nombre : accepté s'il dit le même compte ; absent : refusé.
        assert!(verifier_jeton_steam(&jwt(json!({"sub": 76561197960287930u64})), a, maintenant).is_ok());
        assert!(verifier_jeton_steam(&jwt(json!({"aud": ["web:store"]})), a, maintenant).is_err());
        // Expiré : « reconnecte-toi ».
        let vieux = jwt(json!({"sub": a, "exp": maintenant - 1}));
        assert_eq!(verifier_jeton_steam(&vieux, a, maintenant), Err(Erreur::Refus(MOTIF_RECONNECTER.into())));
        // Steamid rangé douteux, jeton sans la forme d'un JWT, charge illisible : refusés.
        assert!(verifier_jeton_steam(&bon, "123", maintenant).is_err());
        assert!(verifier_jeton_steam("pas.un-jwt", a, maintenant).is_err());
        assert!(verifier_jeton_steam(&format!("aaa.{}.ccc", en_base64url(b"[1,2]")), a, maintenant).is_err());
        assert!(verifier_jeton_steam("aaa.!!!.ccc", a, maintenant).is_err());
    }

    #[tokio::test]
    async fn les_jeux_possedes_se_lisent_avec_le_jeton_sans_jamais_le_montrer() {
        let (id, jeton) = ("76561197960287930", "aaa.JETON-SECRET.ccc");
        let m = MockServer::start();
        let j = m.mock(|w, t| {
            w.method(GET)
                .path("/IPlayerService/GetOwnedGames/v1/")
                .query_param("access_token", jeton)
                .query_param("steamid", id)
                .query_param("include_appinfo", "1")
                .query_param("include_played_free_games", "1");
            t.status(200).json_body(json!({"response": {"game_count": 2, "games": [
                {"appid": 620, "name": "Portal 2", "playtime_forever": 300},
                {"appid": 400, "name": "Portal"}]}}));
        });
        let l = jeux_possedes_par_jeton(&m.base_url(), jeton, id).await.unwrap();
        assert_eq!(l.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(), ["400", "620"]);
        assert!(l.iter().all(|x| !x.installe));
        j.assert();

        // Jeton refusé (401, 403) : « reconnecte-toi », sans le jeton.
        for code in [401, 403] {
            let m = MockServer::start();
            m.mock(|w, t| {
                w.method(GET).path("/IPlayerService/GetOwnedGames/v1/");
                t.status(code);
            });
            let e = jeux_possedes_par_jeton(&m.base_url(), jeton, id).await.unwrap_err();
            assert_eq!(e, Erreur::Refus(MOTIF_RECONNECTER.into()));
        }
        // Pas de liste : un message clair ; un compte sans jeu : une liste vide.
        let m = MockServer::start();
        m.mock(|w, t| {
            w.method(GET).path("/IPlayerService/GetOwnedGames/v1/");
            t.status(200).json_body(json!({"response": {}}));
        });
        let e = format!("{:?}", jeux_possedes_par_jeton(&m.base_url(), jeton, id).await.unwrap_err());
        assert!(e.contains("pas rendu la liste") && !e.contains("JETON"), "{e}");
        let m = MockServer::start();
        m.mock(|w, t| {
            w.method(GET).path("/IPlayerService/GetOwnedGames/v1/");
            t.status(200).json_body(json!({"response": {"game_count": 0}}));
        });
        assert!(jeux_possedes_par_jeton(&m.base_url(), jeton, id).await.unwrap().is_empty());
        // Réseau coupé : l'erreur ne contient jamais l'adresse (elle porte le jeton).
        let e = format!("{:?}", jeux_possedes_par_jeton("http://127.0.0.1:9", jeton, id).await.unwrap_err());
        assert!(!e.contains("JETON") && !e.contains("access_token"), "{e}");
    }

    fn cookie(n: &str, v: &str, d: Option<&str>) -> (String, String, Option<String>) {
        (n.into(), v.into(), d.map(String::from))
    }

    #[test]
    fn seuls_les_cookies_de_la_boutique_steam_partent_et_jamais_dans_un_journal() {
        let l = [
            cookie("steamLoginSecure", "76561197960287930%7C%7CeyJ.SECRET", Some("store.steampowered.com")),
            cookie("sessionid", "abc123", Some(".store.steampowered.com")),
            cookie("steamCountry", "FR%7Cx", Some("steampowered.com")),
            cookie("steamLoginSecure", "AUTRE", Some("steamcommunity.com")),
            cookie("browserid", "1", None),
            cookie("mauvais", "a;b", Some("store.steampowered.com")),
            cookie("espace", "a b", Some("store.steampowered.com")),
        ];
        let c = cookies_de_la_boutique_steam(&l).unwrap();
        assert_eq!(c.0, "steamLoginSecure=76561197960287930%7C%7CeyJ.SECRET; sessionid=abc123");
        assert_eq!(format!("{c:?}"), "CookiesSteam(<cachés>)");
        // Sans cookie de connexion de la boutique : pas connecté.
        assert!(cookies_de_la_boutique_steam(&l[1..]).is_none());
        assert!(cookies_de_la_boutique_steam(&[]).is_none());
        assert!(cookies_de_la_boutique_steam(&[cookie("steamLoginSecure", "", Some("store.steampowered.com"))]).is_none());
    }

    #[tokio::test]
    async fn le_jeton_se_demande_avec_les_cookies_et_ne_se_montre_jamais() {
        let jwt = "eyJhbGciOiJFZERTQSJ9.eyJzdWIiOiI3NjU2MTE5Nzk2MDI4NzkzMCJ9.c2lnbmF0dXJl";
        let cookies = CookiesSteam("steamLoginSecure=SECRET-COOKIE; sessionid=s".into());
        let m = MockServer::start();
        let j = m.mock(|w, t| {
            w.method(GET).path("/pointssummary/ajaxgetasyncconfig").header("cookie", "steamLoginSecure=SECRET-COOKIE; sessionid=s");
            t.status(200).json_body(json!({"success": 1, "data": {"webapi_token": jwt}}));
        });
        assert_eq!(jeton_par_cookies(&m.base_url(), &cookies).await.unwrap(), jwt);
        j.assert();
        // Les réponses qui disent « pas connecté » : reconnecter.
        let reconnecter = Erreur::Refus(MOTIF_RECONNECTER.into());
        for (code, corps) in [(200, json!({"success": 1, "data": {}})), (200, json!({"success": 1, "data": {"webapi_token": ""}})), (200, json!({"success": 21})), (401, json!({})), (403, json!({}))] {
            let m = MockServer::start();
            m.mock(|w, t| {
                w.method(GET).path("/pointssummary/ajaxgetasyncconfig");
                t.status(code).json_body(corps.clone());
            });
            assert_eq!(jeton_par_cookies(&m.base_url(), &cookies).await.unwrap_err(), reconnecter, "{code} {corps}");
        }
        // Une redirection (vers la connexion) n'est JAMAIS suivie : les cookies ne partent pas ailleurs.
        let m = MockServer::start();
        let ailleurs = m.mock(|w, t| {
            w.method(GET).path("/login");
            t.status(200).json_body(json!({"success": 1, "data": {"webapi_token": jwt}}));
        });
        m.mock(|w, t| {
            w.method(GET).path("/pointssummary/ajaxgetasyncconfig");
            t.status(302).header("Location", "/login");
        });
        assert_eq!(jeton_par_cookies(&m.base_url(), &cookies).await.unwrap_err(), reconnecter);
        ailleurs.assert_hits(0);
        // Page changée : illisible, sans cookie ni jeton dans le message.
        for corps in ["<html>page</html>", r#"{"success":1,"data":{"webapi_token":"pas un jwt"}}"#, "[1,2]"] {
            let m = MockServer::start();
            m.mock(|w, t| {
                w.method(GET).path("/pointssummary/ajaxgetasyncconfig");
                t.status(200).body(corps);
            });
            let e = format!("{:?}", jeton_par_cookies(&m.base_url(), &cookies).await.unwrap_err());
            assert!(e.contains("Steam a changé sa page") && !e.contains("SECRET"), "{e}");
        }
        // Réseau coupé : ni l'adresse, ni les cookies.
        let e = format!("{:?}", jeton_par_cookies("http://127.0.0.1:9", &cookies).await.unwrap_err());
        assert!(!e.contains("SECRET") && !e.contains("ajaxgetasyncconfig"), "{e}");
    }

    #[test]
    fn oublier_garde_le_compte_d_avant_pour_voir_un_changement_de_compte() {
        let d = tempfile::tempdir().unwrap();
        let (a, b) = ("76561197960287930", "76561198000000000");
        noter_connexion_steam(d.path(), a, "t1").unwrap();
        // « Oublier » : seul le compte d'avant reste, sans clé, connexion ni liste.
        modifier_source(d.path(), "steam", |c| Ok(compte_oublie(c))).unwrap();
        let c = lire(d.path()).unwrap();
        assert_eq!(c, CompteBoutique { steamid_precedent: a.into(), ..Default::default() });
        assert!(!connecte_par_la_page(&c));
        // Oublier deux fois de suite garde le même compte d'avant ; rien à oublier : rien d'écrit.
        modifier_source(d.path(), "steam", |c| Ok(compte_oublie(c))).unwrap();
        assert_eq!(lire(d.path()).unwrap().steamid_precedent, a);
        assert_eq!(compte_oublie(None), None);
        assert_eq!(compte_oublie(Some(CompteBoutique::default())), None);
        // Le même compte se reconnecte : pas un changement.
        assert!(!noter_connexion_steam(d.path(), a, "t2").unwrap());
        let c = lire(d.path()).unwrap();
        assert!(c.steamid_precedent.is_empty() && !c.menage_ancien_compte);
        // Oublier puis un AUTRE compte : changement, ménage noté en attente.
        modifier_source(d.path(), "steam", |c| Ok(compte_oublie(c))).unwrap();
        assert!(noter_connexion_steam(d.path(), b, "t3").unwrap());
        let c = lire(d.path()).unwrap();
        assert_eq!((c.steamid.as_str(), c.steamid_precedent.as_str(), c.menage_ancien_compte), (b, "", true));
    }

    #[tokio::test]
    async fn trop_de_demandes_une_seule_nouvelle_tentative() {
        let (id, jeton) = ("76561197960287930", "aaa.bbb.ccc");
        // Toujours 429 : deux appels en tout, puis « attendre ».
        let m = MockServer::start();
        let toujours = m.mock(|w, t| {
            w.method(GET).path("/IPlayerService/GetOwnedGames/v1/");
            t.status(429);
        });
        let e = jeux_possedes_par_jeton_apres(&m.base_url(), jeton, id, std::time::Duration::from_millis(10)).await.unwrap_err();
        assert_eq!(e, Erreur::Refus(MOTIF_ATTENDRE.into()));
        toujours.assert_hits(2);
    }
}
