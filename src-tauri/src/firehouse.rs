//! Le client de l'API de Firehouse (`/api/jeux/v1/`).
//!
//! Le jeton part dans l'en-tête `Authorization` et nulle part ailleurs. Les erreurs sont traduites en motifs
//! lisibles, sans chemin d'API ni jeton.

use crate::erreurs::{Erreur, Resultat};
use serde::de::DeserializeOwned;
use std::net::Ipv4Addr;
use std::time::Duration;

pub const PREFIXE: &str = "/api/jeux/v1";

/// Vrai pour une machine du réseau de la maison (10/8, 172.16/12, 192.168/16, 127/8, localhost, `.local`).
pub fn est_adresse_locale(hote: &str) -> bool {
    let h = hote.trim_matches(|c| c == '[' || c == ']').to_ascii_lowercase();
    if h == "localhost" || h == "::1" || h.ends_with(".local") {
        return true;
    }
    match h.parse::<Ipv4Addr>() {
        Ok(ip) => ip.is_private() || ip.is_loopback(),
        Err(_) => false,
    }
}

/// Vérifie et nettoie l'adresse de Firehouse. HTTPS obligatoire hors du réseau de la maison.
pub fn verifier_adresse(adresse: &str) -> Resultat<String> {
    let a = adresse.trim().trim_end_matches('/');
    let url = reqwest::Url::parse(a)
        .map_err(|_| Erreur::Reglage("L'adresse de Firehouse n'est pas une adresse web valable.".into()))?;
    let hote = url.host_str().unwrap_or_default();
    match url.scheme() {
        "https" => Ok(a.into()),
        "http" if est_adresse_locale(hote) => Ok(a.into()),
        "http" => Err(Erreur::Reglage(
            "Hors du réseau de la maison, l'adresse de Firehouse doit être en https://.".into(),
        )),
        _ => Err(Erreur::Reglage("L'adresse de Firehouse doit commencer par https://.".into())),
    }
}

/// Une réponse brute : utile quand on veut l'`ETag` ou les octets (images).
#[derive(Debug)]
pub enum Reponse {
    Corps { octets: Vec<u8>, etag: Option<String>, type_contenu: Option<String> },
    /// 304 : rien n'a changé depuis l'`ETag` donné.
    NonModifie,
}

/// La version MAJEURE du contrat que cette version de Frogtend sait lire (`X-Api-Jeux-Version`).
pub const VERSION_API_MAJEURE: u32 = 1;

/// Compare la version annoncée par Firehouse à celle que Frogtend sait lire. Sans en-tête : on accepte.
pub fn verifier_version(entete: Option<&str>) -> Resultat<()> {
    let Some(v) = entete else { return Ok(()) };
    let Some(majeure) = v.trim().split('.').next().and_then(|m| m.parse::<u32>().ok()) else { return Ok(()) };
    match majeure.cmp(&VERSION_API_MAJEURE) {
        std::cmp::Ordering::Equal => Ok(()),
        std::cmp::Ordering::Greater => Err(Erreur::Version(format!(
            "Firehouse est plus récent que ce Frogtend (contrat {v}). Mets Frogtend à jour depuis « À propos »."
        ))),
        std::cmp::Ordering::Less => Err(Erreur::Version(format!(
            "Firehouse est trop ancien pour ce Frogtend (contrat {v}). Il doit être mis à jour par un admin."
        ))),
    }
}

/// L'origine d'une adresse (`https://hote[:port]`) : c'est à elle que se joignent les chemins `video_api`.
pub fn origine(adresse: &str) -> Resultat<String> {
    let url = reqwest::Url::parse(adresse)
        .map_err(|_| Erreur::Reglage("L'adresse de Firehouse n'est pas une adresse web valable.".into()))?;
    Ok(url.origin().ascii_serialization())
}

pub struct Client {
    base: String,
    /// `None` : client sans jeton, pour les seules routes publiques (`/themes`, vidéo des skins).
    jeton: Option<String>,
    http: reqwest::Client,
    /// Pour les gros fichiers : pas de délai TOTAL (150 Go prennent des heures), seulement un délai de lecture.
    http_flux: reqwest::Client,
}

/// Une réponse brute.
#[derive(Debug)]
pub struct Brute {
    pub statut: u16,
    /// L'en-tête `X-Recu` (octets déjà reçus d'un envoi par morceaux), s'il y est.
    pub recu: Option<u64>,
    /// Contrat 1.51 (`HEAD …/fichier` d'un fichier déjà COMPLET) : `X-Complet: 1` et son `X-Sha256`.
    pub complet: bool,
    pub sha256: Option<String>,
    /// Contrat 1.52 (lot S5) : `X-Modifie` (ISO 8601 UTC) d'un fichier COMPLET de la sauvegarde — le moment où il a été
    /// rangé chez Firehouse. Absent avant la 1.52, ou pour un fichier pas complet.
    pub modifie: Option<String>,
    pub octets: Vec<u8>,
}

/// Un fichier à envoyer par morceaux (`PUT` + `Content-Range: bytes début-fin/total`), partagé par la sauvegarde
/// (contrat 1.5) et les envois Switch (contrat 1.42).
pub struct EnvoiParMorceaux<'a> {
    pub route: &'a str,
    pub source: &'a std::path::Path,
    /// La taille annoncée (le `total` de chaque `Content-Range`).
    pub total: u64,
    /// Où reprendre (ce que Firehouse a déjà reçu).
    pub depart: u64,
    /// Les en-têtes en plus (`X-Contenu-Sha256` pour la sauvegarde ; aucun pour la Switch).
    pub entetes: Vec<(&'static str, String)>,
    /// Le nom à dire dans un message (jamais un chemin).
    pub nom: &'a str,
}

/// Les règles propres à une route d'envoi par morceaux.
pub struct ReglesMorceaux {
    /// La réponse d'un morceau dit-elle que l'envoi est fini (`complet` pour la sauvegarde, `etat` hors « en_cours »
    /// pour la Switch) ?
    pub fini: fn(&serde_json::Value) -> bool,
    /// Un 409 SANS `recu` : repartir de zéro une seule fois (sauvegarde : empreinte refusée au dernier morceau) ; sinon,
    /// l'erreur est dite tout de suite (Switch : jamais de boucle).
    pub zero_sans_recu: bool,
}

/// Le rythme d'un envoi par morceaux : la taille de chaque morceau, une attente avant (pause, débit plafonné), une
/// vérification du fichier ; et ce qui est fait après chaque morceau reçu (progression, mesure du débit).
#[allow(async_fn_in_trait)]
pub trait Cadence {
    /// Avant le morceau qui commence à `position` : rend sa taille (au plus 8 Mo). Une erreur arrête l'envoi.
    async fn avant(&mut self, position: u64) -> Resultat<usize>;
    /// Après un morceau accepté : `octets` envoyés en `duree`, Firehouse en est à `recu`.
    fn apres(&mut self, _recu: u64, _octets: u64, _duree: Duration) {}
}

/// Une cadence fixe (la sauvegarde) : toujours la même taille de morceau, aucune attente.
pub struct MorceauxFixes(pub usize);

impl Cadence for MorceauxFixes {
    async fn avant(&mut self, _position: u64) -> Resultat<usize> {
        Ok(self.0)
    }
}

/// Le début d'un téléchargement en flux.
pub struct Flux {
    pub reponse: reqwest::Response,
    /// Vrai si le serveur reprend là où on s'était arrêté (206) ; faux s'il renvoie tout le fichier (200).
    pub reprise: bool,
}

/// Le motif envoyé par Firehouse (`{"detail": "..."}`), s'il y en a un lisible.
fn motif_du_serveur(corps: &[u8]) -> Option<String> {
    let v: serde_json::Value = serde_json::from_slice(corps).ok()?;
    let d = v.get("detail").or_else(|| v.get("message")).or_else(|| v.get("erreur"))?;
    let texte = match d {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(liste) => liste
            .iter()
            .filter_map(|x| x.get("msg").and_then(|m| m.as_str()).map(String::from))
            .collect::<Vec<_>>()
            .join(" ; "),
        _ => return None,
    };
    // Jamais de chemin d'API dans un message.
    let propre: String = texte.split_whitespace().filter(|m| !m.starts_with("/api/")).collect::<Vec<_>>().join(" ");
    (!propre.is_empty()).then_some(propre)
}

/// Traduit un code HTTP d'erreur en erreur lisible.
pub fn erreur_du_statut(code: u16, corps: &[u8]) -> Erreur {
    let motif = motif_du_serveur(corps);
    match code {
        401 => Erreur::JetonRefuse(
            "Firehouse refuse le jeton de ce profil : il a été révoqué ou il a expiré. Crée-en un nouveau dans Firehouse \
             (👤 Mon compte ▸ 🔌 Mes jetons, usage « 🎮 Frogtend »), puis remplace-le dans ⚙ Options ▸ Mon profil."
                .into(),
        ),
        413 => Erreur::Refus("Firehouse refuse un envoi trop gros d'un coup.".into()),
        429 => Erreur::Refus("Firehouse traite déjà une question pour toi : attends sa réponse, puis réessaie.".into()),
        503 => Erreur::Serveur("Firehouse est occupé (maintenance en cours). Réessaie dans une minute.".into()),
        // 502 : une source extérieure ne répond pas ; Firehouse dit laquelle et, si possible, quelle page ouvrir.
        502 => Erreur::Serveur(motif.unwrap_or_else(|| "Une source extérieure ne répond pas. Réessaie plus tard.".into())),
        // Lot S4 : une sorte à part (jamais reconnue par son texte). Contrat 1.50 : `{detail}` sans chiffres ; 1.51 :
        // `{detail, utilise_octets, quota_octets, manque_octets}` — les chiffres lus s'ils sont là.
        507 => {
            let manque = serde_json::from_slice::<serde_json::Value>(corps).ok().and_then(|v| v["manque_octets"].as_u64()).filter(|n| *n > 0);
            Erreur::Plein {
                motif: match manque {
                    Some(n) => format!("Il manque {} chez Firehouse : ton espace est plein. Libère de la place ou demande plus d'espace à Seb.", crate::erreurs::taille_lisible(n)),
                    None => "Ton espace chez Firehouse est plein. Libère de la place ou demande plus d'espace à Seb.".into(),
                },
                manque_octets: manque,
            }
        }
        404 => Erreur::Introuvable("Firehouse ne connaît pas ce jeu (ou il ne t'est pas visible).".into()),
        // 410 (contrat 1.14) : la fiche a été RÉUNIE à une autre. En attendant le rattachement, comme un 404 (le contrat
        // le permet), avec le texte de Firehouse — et surtout JAMAIS un `.part` effacé.
        410 => {
            let texte = motif.unwrap_or_else(|| "Ce jeu a été réuni à une autre fiche dans Firehouse.".into());
            // Le nouveau numéro, s'il est lisible ; sinon comme un 404 (le contrat le permet).
            match serde_json::from_slice::<serde_json::Value>(corps).ok().and_then(|v| v["fusionne_dans"].as_i64()).filter(|n| *n > 0) {
                Some(nouveau) => Erreur::Fusionne { motif: texte, nouveau },
                None => Erreur::Introuvable(texte),
            }
        }
        416 => Erreur::Plage("La reprise du téléchargement n'est plus possible : il recommence depuis le début.".into()),
        409 => Erreur::Conflit(motif.unwrap_or_else(|| "Firehouse signale un conflit : réessaie plus tard.".into())),
        400..=499 => Erreur::Refus(motif.unwrap_or_else(|| format!("Firehouse a refusé la demande (code {code})."))),
        _ => Erreur::Serveur(format!("Firehouse a rencontré un problème (code {code}). Réessaie plus tard.")),
    }
}

impl Client {
    pub fn nouveau(adresse: &str, jeton: &str) -> Resultat<Self> {
        Self::construire(adresse, Some(jeton.into()))
    }

    /// Un client sans jeton, pour les routes publiques.
    pub fn public(adresse: &str) -> Resultat<Self> {
        Self::construire(adresse, None)
    }

    fn construire(adresse: &str, jeton: Option<String>) -> Resultat<Self> {
        let base = verifier_adresse(adresse)?;
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(60))
            .user_agent(concat!("Frogtend/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| Erreur::Reseau("Impossible de préparer la connexion.".into()))?;
        // Les réponses JSON viennent compressées (contrat 1.30, `Accept-Encoding: gzip`, décompressées par reqwest) ;
        // le flux des FICHIERS jamais (`no_gzip`) : sa taille, sa reprise `Range` et son empreinte restent exactes.
        let http_flux = reqwest::Client::builder()
            .no_gzip()
            .connect_timeout(Duration::from_secs(15))
            .read_timeout(Duration::from_secs(90))
            .user_agent(concat!("Frogtend/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| Erreur::Reseau("Impossible de préparer la connexion.".into()))?;
        Ok(Client { base, jeton, http, http_flux })
    }

    fn url(&self, route: &str) -> String {
        format!("{}{PREFIXE}{route}", self.base)
    }

    fn erreur_reseau(e: reqwest::Error) -> Erreur {
        if e.is_timeout() {
            Erreur::Reseau("Firehouse ne répond pas (délai dépassé). Vérifie ta connexion.".into())
        } else {
            Erreur::Reseau("Impossible de joindre Firehouse. Vérifie ta connexion et l'adresse dans les réglages.".into())
        }
    }

    fn avec_jeton(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.jeton {
            Some(j) => req.bearer_auth(j),
            None => req,
        }
    }

    /// Envoie une requête et contrôle la version du contrat annoncée par Firehouse.
    async fn envoyer(&self, req: reqwest::RequestBuilder) -> Resultat<reqwest::Response> {
        let rep = self.avec_jeton(req).send().await.map_err(Self::erreur_reseau)?;
        verifier_version(rep.headers().get("x-api-jeux-version").and_then(|v| v.to_str().ok()))?;
        Ok(rep)
    }

    /// `GET` d'une route (`/plateformes`…), avec un `ETag` éventuel.
    pub async fn obtenir(&self, route: &str, etag: Option<&str>) -> Resultat<Reponse> {
        let mut req = self.http.get(self.url(route));
        if let Some(e) = etag {
            req = req.header(reqwest::header::IF_NONE_MATCH, e);
        }
        let rep = self.envoyer(req).await?;
        let statut = rep.status().as_u16();
        if statut == 304 {
            return Ok(Reponse::NonModifie);
        }
        let entete = |nom: reqwest::header::HeaderName| {
            rep.headers().get(nom).and_then(|v| v.to_str().ok()).map(String::from)
        };
        let etag = entete(reqwest::header::ETAG);
        let type_contenu = entete(reqwest::header::CONTENT_TYPE);
        let octets = rep.bytes().await.map_err(Self::erreur_reseau)?.to_vec();
        if !(200..300).contains(&statut) {
            return Err(erreur_du_statut(statut, &octets));
        }
        Ok(Reponse::Corps { octets, etag, type_contenu })
    }

    /// `GET` d'une GROSSE réponse JSON (la liste MAME, contrat 1.33 : 16 Mo) : comme [`Client::obtenir`] (ETag, gzip),
    /// mais jusqu'à 10 minutes pour cette seule requête (une connexion modeste), lue par morceaux et jamais au-delà de
    /// `plafond` octets.
    pub async fn obtenir_gros(&self, route: &str, etag: Option<&str>, plafond: usize) -> Resultat<Reponse> {
        let mut req = self.http.get(self.url(route)).timeout(Duration::from_secs(600));
        if let Some(e) = etag {
            req = req.header(reqwest::header::IF_NONE_MATCH, e);
        }
        let mut rep = self.envoyer(req).await?;
        let statut = rep.status().as_u16();
        if statut == 304 {
            return Ok(Reponse::NonModifie);
        }
        let entete = |nom: reqwest::header::HeaderName| rep.headers().get(nom).and_then(|v| v.to_str().ok()).map(String::from);
        let (etag, type_contenu) = (entete(reqwest::header::ETAG), entete(reqwest::header::CONTENT_TYPE));
        let mut octets = Vec::new();
        while let Some(m) = rep.chunk().await.map_err(Self::erreur_reseau)? {
            if octets.len() + m.len() > plafond {
                return Err(Erreur::Serveur("Firehouse a envoyé une réponse bien trop grosse : elle n'est pas lue.".into()));
            }
            octets.extend_from_slice(&m);
        }
        if !(200..300).contains(&statut) {
            return Err(erreur_du_statut(statut, &octets));
        }
        Ok(Reponse::Corps { octets, etag, type_contenu })
    }

    /// `GET` d'une adresse complète hors de la base de l'API (la vidéo d'un skin : `origine + video_api`).
    pub async fn obtenir_absolu(&self, url: &str) -> Resultat<Vec<u8>> {
        let rep = self.envoyer(self.http.get(url)).await?;
        let statut = rep.status().as_u16();
        let octets = rep.bytes().await.map_err(Self::erreur_reseau)?.to_vec();
        if !(200..300).contains(&statut) {
            return Err(erreur_du_statut(statut, &octets));
        }
        Ok(octets)
    }

    /// `GET` d'une route en FLUX, à partir de l'octet `debut` (reprise `Range`). 416 : rien à reprendre.
    pub async fn flux(&self, route: &str, debut: u64) -> Resultat<Flux> {
        // Un fichier ne voyage JAMAIS compressé : sa taille, sa reprise `Range` et son empreinte doivent rester exactes
        // (relevé du 24/11 : Firehouse compresse parfois sans qu'on le demande, besoin 58).
        let mut req = self.http_flux.get(self.url(route)).header(reqwest::header::ACCEPT_ENCODING, "identity");
        if debut > 0 {
            req = req.header(reqwest::header::RANGE, format!("bytes={debut}-"));
        }
        let rep = self.envoyer(req).await?;
        let encodage = rep.headers().get(reqwest::header::CONTENT_ENCODING).and_then(|v| v.to_str().ok()).map(|v| v.trim().to_ascii_lowercase());
        if (200..300).contains(&rep.status().as_u16()) && encodage.as_deref().is_some_and(|e| !e.is_empty() && e != "identity") {
            return Err(Erreur::Serveur("Firehouse a envoyé ce fichier compressé : le téléchargement s'arrête pour ne pas l'abîmer.".into()));
        }
        match rep.status().as_u16() {
            206 => Ok(Flux { reponse: rep, reprise: true }),
            200..=299 => Ok(Flux { reponse: rep, reprise: false }),
            code => {
                let octets = rep.bytes().await.unwrap_or_default();
                Err(erreur_du_statut(code, &octets))
            }
        }
    }

    /// Lit le morceau suivant d'un flux (`None` à la fin).
    pub async fn morceau(flux: &mut Flux) -> Resultat<Option<bytes::Bytes>> {
        flux.reponse.chunk().await.map_err(|e| {
            if e.is_timeout() {
                Erreur::Reseau("Le téléchargement ne reçoit plus rien depuis 90 s. Il reprendra où il en était.".into())
            } else {
                Erreur::Reseau("La connexion a été coupée pendant le téléchargement. Il reprendra où il en était.".into())
            }
        })
    }

/// Une réponse brute (code, en-têtes utiles, corps), pour les échanges qui ont besoin du code (reprise d'un envoi).
    pub async fn brute(
        &self,
        methode: reqwest::Method,
        route: &str,
        corps: Option<Vec<u8>>,
        entetes: &[(&str, String)],
    ) -> Resultat<Brute> {
        let mut req = self.http_flux.request(methode, self.url(route));
        for (n, v) in entetes {
            req = req.header(*n, v);
        }
        if let Some(c) = corps {
            req = req.body(c);
        }
        let rep = self.envoyer(req).await?;
        let statut = rep.status().as_u16();
        let recu = rep.headers().get("x-recu").and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok());
        let complet = rep.headers().get("x-complet").and_then(|v| v.to_str().ok()).is_some_and(|v| v.trim() == "1");
        let sha256 = rep.headers().get("x-sha256").and_then(|v| v.to_str().ok()).map(|v| v.trim().to_ascii_lowercase());
        let modifie = rep.headers().get("x-modifie").and_then(|v| v.to_str().ok()).map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
        let gzip = rep.headers().get(reqwest::header::CONTENT_ENCODING).and_then(|v| v.to_str().ok()).is_some_and(|v| v.trim().eq_ignore_ascii_case("gzip"));
        let octets = rep.bytes().await.map_err(Self::erreur_reseau)?.to_vec();
        // Relevé réel du 24/11 : Firehouse compresse ses grosses réponses JSON (manifeste, liste des sauvegardes) même
        // sans `Accept-Encoding` ; ce canal ne les décompressait pas (le manifeste passait pour vide). Une réponse
        // annoncée gzip est donc décompressée ici ; illisible : une erreur, jamais des octets compressés pris pour du JSON.
        // Seulement un vrai corps gzip (`1f 8b`) : un HEAD ou une erreur au corps vide garde son code et ses en-têtes.
        // Hors 2xx, un corps indéchiffrable reste brut (le vrai code de Firehouse prime).
        let octets = if gzip && octets.starts_with(&[0x1f, 0x8b]) {
            match degonfler(&octets) {
                Ok(o) => o,
                Err(e) if (200..300).contains(&statut) => return Err(e),
                Err(_) => octets,
            }
        } else {
            octets
        };
        Ok(Brute { statut, recu, complet, sha256, modifie, octets })
    }

    /// Envoie `e.source` par morceaux à partir de `e.depart` ; rend les octets envoyés et la dernière réponse (celle qui
    /// dit « fini »). Un 409 `{recu}` fait reprendre là où Firehouse en est (3 fois au plus) ; un 409 sans `recu` suit
    /// `regles.zero_sans_recu`. Toute autre réponse hors 2xx est traduite (`erreur_du_statut`).
    pub async fn envoyer_par_morceaux(
        &self,
        e: &EnvoiParMorceaux<'_>,
        regles: &ReglesMorceaux,
        cadence: &mut impl Cadence,
    ) -> Resultat<(u64, serde_json::Value)> {
        use std::io::{Read, Seek, SeekFrom};
        let total = e.total;
        let mut position = e.depart.min(total);
        // Ouvert APRÈS la première attente de la cadence (lot S4 : une pause pendant une partie ne tient jamais le
        // fichier ouvert).
        let mut fichier: Option<std::fs::File> = None;
        let mut envoyes = 0u64;
        let mut conflits = 0;
        // Lot S4 : le dernier morceau renvoyé une fois pour un `.part` déjà entier, et la reprise à zéro faite une fois.
        let (mut dernier_renvoye, mut zero_fait) = (false, false);
        loop {
            let morceau = cadence.avant(position).await?.max(1) as u64;
            // Jamais un morceau vide (`Content-Range` sans octet) : Firehouse a déjà tout (`.part` entier) → le DERNIER
            // morceau est renvoyé (contrat 1.51 : il finalise si l'empreinte colle).
            if total > 0 && position >= total {
                position = (total - 1) / morceau * morceau;
            }
            let fin = (position + morceau).min(total);
            let mut octets = vec![0u8; (fin - position) as usize];
            if fichier.is_none() {
                fichier = Some(std::fs::File::open(e.source)?);
            }
            let f = fichier.as_mut().expect("ouvert juste avant");
            f.seek(SeekFrom::Start(position))?;
            f.read_exact(&mut octets)?;
            let mut entetes: Vec<(&str, String)> = vec![("Content-Range", format!("bytes {position}-{}/{total}", fin.max(1) - 1))];
            entetes.extend(e.entetes.iter().cloned());
            entetes.push(("Content-Type", "application/octet-stream".into()));
            let debut = std::time::Instant::now();
            let b = self.brute(reqwest::Method::PUT, e.route, Some(octets), &entetes).await?;
            let duree = debut.elapsed();
            let v: serde_json::Value = serde_json::from_slice(&b.octets).unwrap_or(serde_json::Value::Null);
            match b.statut {
                200..=299 => {
                    envoyes += fin - position;
                    if (regles.fini)(&v) || fin == total {
                        if !(regles.fini)(&v) {
                            // Lot S4 : propre à CE fichier (sauté et compté), jamais l'arrêt de toute une sauvegarde.
                            return Err(Erreur::Conflit(format!("Firehouse n'a pas confirmé « {} ».", e.nom)));
                        }
                        cadence.apres(fin, fin - position, duree);
                        return Ok((envoyes, v));
                    }
                    let recu = v["recu"].as_u64().unwrap_or(fin).min(total);
                    cadence.apres(recu, fin - position, duree);
                    position = recu;
                }
                409 => {
                    conflits += 1;
                    match v["recu"].as_u64() {
                        // Lot S4 : un `.part` déjà ENTIER (`recu` = total) : le dernier morceau, renvoyé UNE fois (la 1.51 le
                        // finalise) ; encore 409 (la 1.50 ne sait pas finir) : ce fichier est sauté, jamais une boucle.
                        Some(r) if total > 0 && r >= total => {
                            if dernier_renvoye {
                                return Err(Erreur::Conflit(format!("Firehouse ne finit pas l'envoi de « {} » : il sera réessayé plus tard.", e.nom)));
                            }
                            dernier_renvoye = true;
                            position = total;
                        }
                        // 1.51 : empreinte refusée sur un `.part` entier (effacé) : de zéro, une seule fois.
                        Some(0) if position > 0 => {
                            if zero_fait {
                                return Err(Erreur::Conflit(format!("« {} » arrive abîmé chez Firehouse.", e.nom)));
                            }
                            zero_fait = true;
                            position = 0;
                        }
                        // Le morceau ne commençait pas là où Firehouse en est : on reprend là.
                        Some(r) if conflits <= 3 => position = r.min(total),
                        // Lot S4 : propre à CE fichier (sauté et compté), jamais l'arrêt de toute une sauvegarde.
                        Some(_) => return Err(Erreur::Conflit(format!("L'envoi de « {} » ne se cale pas.", e.nom))),
                        // Sauvegarde : empreinte différente au dernier morceau (le .part est effacé) : le fichier a
                        // changé pendant l'envoi, ou il est arrivé abîmé. Un seul nouvel essai complet.
                        None if regles.zero_sans_recu && conflits <= 1 => position = 0,
                        None if regles.zero_sans_recu => return Err(Erreur::Conflit(format!("« {} » arrive abîmé chez Firehouse.", e.nom))),
                        None => return Err(erreur_du_statut(409, &b.octets)),
                    }
                }
                _ => return Err(erreur_du_statut(b.statut, &b.octets)),
            }
        }
    }

    /// `GET` d'une route JSON.
    pub async fn obtenir_json<T: DeserializeOwned>(&self, route: &str) -> Resultat<T> {
        match self.obtenir(route, None).await? {
            Reponse::Corps { octets, .. } => serde_json::from_slice(&octets)
                .map_err(|_| Erreur::Serveur("Firehouse a envoyé une réponse illisible.".into())),
            Reponse::NonModifie => Err(Erreur::Serveur("Réponse inattendue de Firehouse.".into())),
        }
    }

    async fn ecrire_json<T: DeserializeOwned>(
        &self,
        req: reqwest::RequestBuilder,
        corps: &serde_json::Value,
    ) -> Resultat<T> {
        let rep = self.envoyer(req.json(corps)).await?;
        let statut = rep.status().as_u16();
        let octets = rep.bytes().await.map_err(Self::erreur_reseau)?;
        if !(200..300).contains(&statut) {
            return Err(erreur_du_statut(statut, &octets));
        }
        serde_json::from_slice(&octets).map_err(|_| Erreur::Serveur("Firehouse a envoyé une réponse illisible.".into()))
    }

    /// `POST` d'un objet JSON.
    pub async fn envoyer_json<T: DeserializeOwned>(&self, route: &str, corps: &serde_json::Value) -> Resultat<T> {
        self.ecrire_json(self.http.post(self.url(route)), corps).await
    }

    /// `PUT` d'un objet JSON.
    pub async fn remplacer_json<T: DeserializeOwned>(&self, route: &str, corps: &serde_json::Value) -> Resultat<T> {
        self.ecrire_json(self.http.put(self.url(route)), corps).await
    }
}

/// Le plafond d'une réponse brute décompressée (jamais une bombe de décompression).
const PLAFOND_DEGONFLE: u64 = 256 * 1024 * 1024;

/// Décompresse une réponse gzip ; au-delà du plafond, une ERREUR (jamais une réponse coupée rendue comme entière).
fn degonfler(octets: &[u8]) -> Resultat<Vec<u8>> {
    use std::io::Read;
    let mut sortie = Vec::new();
    flate2::read::GzDecoder::new(octets)
        .take(PLAFOND_DEGONFLE + 1)
        .read_to_end(&mut sortie)
        .map_err(|_| Erreur::Serveur("Firehouse a envoyé une réponse compressée illisible.".into()))?;
    if sortie.len() as u64 > PLAFOND_DEGONFLE {
        return Err(Erreur::Serveur("Firehouse a envoyé une réponse compressée trop grosse.".into()));
    }
    Ok(sortie)
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    /// Relevé réel du 24/11 : une réponse brute annoncée gzip (sans qu'on l'ait demandée) est décompressée ; une
    /// réponse gzip abîmée est une erreur.
    #[tokio::test]
    async fn une_reponse_brute_compressee_est_decompressee() {
        use std::io::Write;
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gz.write_all(br#"{"utilise_octets": 5, "quota_octets": 10}"#).unwrap();
        let compresse = gz.finish().unwrap();
        let s = MockServer::start();
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/sauvegarde");
            t.status(200).header("content-encoding", "gzip").header("content-type", "application/json").body(&compresse);
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/abime");
            t.status(200).header("content-encoding", "gzip").body([0x1f, 0x8b, 0, 1, 2]);
        });
        let c = Client::nouveau(&s.base_url(), "j").unwrap();
        let b = c.brute(reqwest::Method::GET, "/sauvegarde", None, &[]).await.unwrap();
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&b.octets).unwrap()["quota_octets"], 10);
        assert!(matches!(crate::sauvegarde::lire_place(&c).await, Ok(Some(_))));
        assert!(matches!(c.brute(reqwest::Method::GET, "/abime", None, &[]).await, Err(Erreur::Serveur(_))));
    }

    /// Contrat 1.52 (lot S5) : `HEAD …/fichier` d'un fichier complet porte `X-Modifie` ; absent (avant la 1.52, fichier
    /// pas complet) : `None`, jamais une date inventée.
    #[tokio::test]
    async fn l_en_tete_x_modifie_est_lu_quand_il_est_la() {
        let s = MockServer::start();
        s.mock(|w, t| {
            w.method(httpmock::Method::HEAD).path("/api/jeux/v1/present");
            t.status(200).header("X-Complet", "1").header("X-Modifie", "2026-11-24T10:53:09Z");
        });
        s.mock(|w, t| {
            w.method(httpmock::Method::HEAD).path("/api/jeux/v1/absent");
            t.status(200).header("X-Complet", "1");
        });
        let c = Client::nouveau(&s.base_url(), "j").unwrap();
        let b = c.brute(reqwest::Method::HEAD, "/present", None, &[]).await.unwrap();
        assert_eq!((b.complet, b.modifie.as_deref()), (true, Some("2026-11-24T10:53:09Z")));
        let b = c.brute(reqwest::Method::HEAD, "/absent", None, &[]).await.unwrap();
        assert_eq!((b.complet, b.modifie), (true, None));
    }

    /// Besoin 58 (24/11) : Firehouse ne compresse que si la requête contient « gzip » : quels en-têtes le canal brut
    /// envoie-t-il vraiment ?
    #[tokio::test]
    async fn entetes_du_canal_brut() {
        use std::io::{Read, Write};
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let adresse = format!("http://{}", l.local_addr().unwrap());
        let fil = std::thread::spawn(move || {
            let (mut f, _) = l.accept().unwrap();
            let mut tampon = [0u8; 4096];
            let n = f.read(&mut tampon).unwrap();
            f.write_all(b"HTTP/1.1 404 Not Found
content-length: 2
connection: close

{}").unwrap();
            String::from_utf8_lossy(&tampon[..n]).to_string()
        });
        let c = Client::nouveau(&adresse, "j").unwrap();
        let _ = c.brute(reqwest::Method::GET, "/x", None, &[]).await;
        let requete = fil.join().unwrap();
        let entetes: Vec<&str> = requete.lines().filter(|l| !l.to_ascii_lowercase().starts_with("authorization")).collect();
        println!("== {entetes:?}");
        assert!(!requete.to_ascii_lowercase().contains("gzip"), "le canal brut demande gzip : {entetes:?}");
    }

    /// Revue du 24/11 : un corps vide annoncé gzip (HEAD, erreur) garde son code ; une erreur au corps abîmé aussi ; un
    /// fichier en flux arrivé compressé est refusé (jamais des octets gzip dans un `.part`) ; trop gros : une erreur.
    #[tokio::test]
    async fn gzip_vide_erreur_flux_et_plafond() {
        use std::io::Write;
        let s = MockServer::start();
        s.mock(|w, t| {
            w.method(httpmock::Method::HEAD).path("/api/jeux/v1/tete");
            t.status(200).header("content-encoding", "gzip").header("X-Recu", "4");
        });
        s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/refus");
            t.status(409).header("content-encoding", "gzip").body([0x1f, 0x8b, 0, 1]);
        });
        let fichier = s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/fichier/1/0/0").header("accept-encoding", "identity");
            t.status(200).header("content-encoding", "gzip").body("x");
        });
        let c = Client::nouveau(&s.base_url(), "j").unwrap();
        let b = c.brute(reqwest::Method::HEAD, "/tete", None, &[]).await.unwrap();
        assert_eq!((b.statut, b.recu), (200, Some(4)));
        assert_eq!(c.brute(reqwest::Method::GET, "/refus", None, &[]).await.unwrap().statut, 409);
        assert!(matches!(c.flux("/fichier/1/0/0", 0).await, Err(Erreur::Serveur(_))));
        fichier.assert();
        // Plus gros que le plafond une fois décompressé : une erreur, jamais une réponse coupée.
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
        let zeros = vec![0u8; 1024 * 1024];
        for _ in 0..(PLAFOND_DEGONFLE / zeros.len() as u64 + 1) {
            gz.write_all(&zeros).unwrap();
        }
        assert!(matches!(degonfler(&gz.finish().unwrap()), Err(Erreur::Serveur(_))));
    }

    #[tokio::test]
    async fn les_listes_arrivent_compressees_jamais_les_fichiers() {
        use std::io::Write;
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gz.write_all(br#"{"ok": true, "plateformes": [{"nom": "SNES", "jeux": 3}]}"#).unwrap();
        let compresse = gz.finish().unwrap();
        let s = MockServer::start();
        // Une liste : demandée compressée, décompressée par le client.
        let liste = s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/plateformes").header_exists("accept-encoding");
            t.status(200).header("content-encoding", "gzip").header("content-type", "application/json").body(&compresse);
        });
        // Un fichier : JAMAIS demandé compressé (taille, reprise et empreinte exactes).
        let fichier = s.mock(|w, t| {
            w.method(GET).path("/api/jeux/v1/fichier/1/2/0").matches(|r| {
                !r.headers.as_ref().is_some_and(|h| h.iter().any(|(k, v)| k.eq_ignore_ascii_case("accept-encoding") && v.contains("gzip")))
            });
            t.status(200).body(b"octets");
        });
        let c = Client::nouveau(&s.base_url(), "j").unwrap();
        let v: serde_json::Value = c.obtenir_json("/plateformes").await.unwrap();
        assert_eq!(v["plateformes"][0]["nom"], "SNES");
        liste.assert();
        let mut f = c.flux("/fichier/1/2/0", 0).await.unwrap();
        let mut recu = Vec::new();
        while let Some(m) = Client::morceau(&mut f).await.unwrap() {
            recu.extend_from_slice(&m);
        }
        assert_eq!(recu, b"octets");
        fichier.assert();
    }

    #[test]
    fn https_partout_http_seulement_a_la_maison() {
        assert_eq!(verifier_adresse("https://jeux.hikari-no-sekai.fr/").unwrap(), "https://jeux.hikari-no-sekai.fr");
        assert!(verifier_adresse("http://10.10.0.2:8100").is_ok());
        assert!(verifier_adresse("http://127.0.0.1:9").is_ok());
        assert!(matches!(verifier_adresse("http://jeux.hikari-no-sekai.fr"), Err(Erreur::Reglage(_))));
        assert!(matches!(verifier_adresse("http://8.8.8.8"), Err(Erreur::Reglage(_))));
        assert!(matches!(verifier_adresse("pas une adresse"), Err(Erreur::Reglage(_))));
    }

    #[test]
    fn les_codes_d_erreur_deviennent_des_motifs_lisibles() {
        assert!(matches!(erreur_du_statut(401, b""), Erreur::JetonRefuse(_)));
        assert!(matches!(erreur_du_statut(404, b""), Erreur::Introuvable(_)));
        // Contrat 1.14 (relevé réel du 18/10) : une fiche réunie, comme un 404 avec le texte de Firehouse ; 416 : la
        // seule erreur qui autorise à effacer un `.part`.
        let e = erreur_du_statut(410, br#"{"ok":false,"fusionne_dans":324,"le":"2026-10-05T14:02:37","detail":"ce jeu a ete reuni a la fiche 324"}"#);
        assert!(matches!(&e, Erreur::Fusionne { motif, nouveau: 324 } if motif.contains("324")), "{e:?}");
        assert!(matches!(erreur_du_statut(410, b"pas du json"), Erreur::Introuvable(_)));
        assert!(matches!(erreur_du_statut(416, b""), Erreur::Plage(_)));
        assert_eq!(
            erreur_du_statut(409, br#"{"detail":"fichier abime"}"#),
            Erreur::Conflit("fichier abime".into())
        );
        assert_eq!(
            erreur_du_statut(403, br#"{"detail":"grade insuffisant"}"#),
            Erreur::Refus("grade insuffisant".into())
        );
        assert!(matches!(erreur_du_statut(502, b"<html>"), Erreur::Serveur(_)));
        assert!(matches!(
            erreur_du_statut(502, r#"{"detail":"la forge ne répond pas — installation manuelle : ouvrir https://ryujinx.app/download"}"#.as_bytes()),
            Erreur::Serveur(m) if m.contains("https://ryujinx.app/download")
        ));
        // Lot S4 : une sorte à part, jamais reconnue par son texte.
        assert!(matches!(erreur_du_statut(507, b""), Erreur::Plein { motif, manque_octets: None } if motif.contains("plein")));
        assert!(matches!(erreur_du_statut(503, b""), Erreur::Serveur(m) if m.contains("minute")));
        assert!(matches!(erreur_du_statut(413, b""), Erreur::Refus(_)));
    }

    #[test]
    fn un_motif_ne_montre_jamais_de_chemin_d_api() {
        let e = erreur_du_statut(400, br#"{"detail":"route /api/jeux/v1/jeu/12 invalide"}"#);
        assert_eq!(e, Erreur::Refus("route invalide".into()));
    }

    #[tokio::test]
    async fn le_jeton_part_dans_l_en_tete_et_le_json_est_lu() {
        let serveur = MockServer::start();
        let m = serveur.mock(|when, then| {
            when.method(GET).path("/api/jeux/v1/plateformes").header("authorization", "Bearer jeton-test");
            then.status(200).json_body(serde_json::json!([{"nom": "MS-DOS", "jeux": 3}]));
        });
        let c = Client::nouveau(&serveur.base_url(), "jeton-test").unwrap();
        let v: serde_json::Value = c.obtenir_json("/plateformes").await.unwrap();
        assert_eq!(v[0]["nom"], "MS-DOS");
        m.assert();
    }

    #[tokio::test]
    async fn un_401_devient_jeton_refuse_sans_nouvel_essai() {
        let serveur = MockServer::start();
        let m = serveur.mock(|when, then| {
            when.method(GET).path("/api/jeux/v1/catalogue");
            then.status(401).json_body(serde_json::json!({"detail": "connexion requise"}));
        });
        let c = Client::nouveau(&serveur.base_url(), "vieux").unwrap();
        let e = c.obtenir_json::<serde_json::Value>("/catalogue").await.unwrap_err();
        assert!(matches!(e, Erreur::JetonRefuse(_)));
        m.assert_hits(1);
    }

    #[tokio::test]
    async fn l_etag_donne_un_304_non_modifie() {
        let serveur = MockServer::start();
        serveur.mock(|when, then| {
            when.method(GET).path("/api/jeux/v1/themes").header("if-none-match", "\"v1\"");
            then.status(304);
        });
        serveur.mock(|when, then| {
            when.method(GET).path("/api/jeux/v1/themes");
            then.status(200).header("etag", "\"v1\"").body("{}");
        });
        let c = Client::nouveau(&serveur.base_url(), "j").unwrap();
        match c.obtenir("/themes", None).await.unwrap() {
            Reponse::Corps { etag, .. } => assert_eq!(etag.as_deref(), Some("\"v1\"")),
            r => panic!("{r:?}"),
        }
        assert!(matches!(c.obtenir("/themes", Some("\"v1\"")).await.unwrap(), Reponse::NonModifie));
    }

    #[test]
    fn la_version_majeure_decide_de_la_compatibilite() {
        assert!(verifier_version(Some("1.3")).is_ok());
        assert!(verifier_version(Some("1.0")).is_ok());
        assert!(verifier_version(None).is_ok());
        assert!(matches!(verifier_version(Some("2.0")), Err(Erreur::Version(m)) if m.contains("plus récent")));
        assert!(matches!(verifier_version(Some("0.9")), Err(Erreur::Version(m)) if m.contains("trop ancien")));
    }

    #[test]
    fn l_origine_ignore_le_chemin() {
        assert_eq!(origine("https://jeux.hikari-no-sekai.fr").unwrap(), "https://jeux.hikari-no-sekai.fr");
        assert_eq!(origine("http://10.10.0.2:8100/sous/chemin").unwrap(), "http://10.10.0.2:8100");
    }

    #[tokio::test]
    async fn un_serveur_d_une_autre_majeure_est_refuse() {
        let serveur = MockServer::start();
        serveur.mock(|when, then| {
            when.method(GET).path("/api/jeux/v1/plateformes");
            then.status(200).header("X-Api-Jeux-Version", "2.0").json_body(serde_json::json!({"ok": true}));
        });
        let c = Client::nouveau(&serveur.base_url(), "j").unwrap();
        let e = c.obtenir_json::<serde_json::Value>("/plateformes").await.unwrap_err();
        assert!(matches!(e, Erreur::Version(_)), "{e:?}");
    }

    #[tokio::test]
    async fn le_client_public_n_envoie_aucun_jeton() {
        let serveur = MockServer::start();
        let avec = serveur.mock(|when, then| {
            when.method(GET).path("/api/jeux/v1/themes").header_exists("authorization");
            then.status(500);
        });
        let sans = serveur.mock(|when, then| {
            when.method(GET).path("/api/jeux/v1/themes");
            then.status(200).body("{}");
        });
        Client::public(&serveur.base_url()).unwrap().obtenir("/themes", None).await.unwrap();
        avec.assert_hits(0);
        sans.assert_hits(1);
    }

    #[tokio::test]
    async fn put_envoie_le_corps_json() {
        let serveur = MockServer::start();
        let m = serveur.mock(|when, then| {
            when.method(PUT).path("/api/jeux/v1/theme").json_body(serde_json::json!({"theme": "dracula"}));
            then.status(200).json_body(serde_json::json!({"ok": true}));
        });
        let c = Client::nouveau(&serveur.base_url(), "j").unwrap();
        let _: serde_json::Value = c.remplacer_json("/theme", &serde_json::json!({"theme": "dracula"})).await.unwrap();
        m.assert();
    }

    #[tokio::test]
    async fn un_serveur_injoignable_est_une_erreur_reseau() {
        // Port 9 (discard) de la machine elle-même : personne n'écoute.
        let c = Client::nouveau("http://127.0.0.1:9", "j").unwrap();
        let e = c.obtenir_json::<serde_json::Value>("/plateformes").await.unwrap_err();
        assert!(matches!(e, Erreur::Reseau(_)), "{e:?}");
    }

    /// Lot S4 (M1) : le fichier n'est ouvert qu'APRÈS la première attente de la cadence (une pause pendant une partie
    /// ne le tient jamais ouvert). Preuve : il n'existe qu'une fois l'attente finie.
    #[tokio::test]
    async fn le_fichier_n_est_ouvert_qu_apres_la_pause() {
        struct Cree(std::path::PathBuf);
        impl Cadence for Cree {
            async fn avant(&mut self, _position: u64) -> Resultat<usize> {
                if !self.0.exists() {
                    std::fs::write(&self.0, b"abcd").unwrap();
                }
                Ok(4)
            }
        }
        let s = MockServer::start();
        let m = s.mock(|w, t| {
            w.method(PUT).path("/api/jeux/v1/x").body("abcd");
            t.status(200).json_body(serde_json::json!({"complet": true}));
        });
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("pas-encore-la.sav");
        let c = Client::nouveau(&s.base_url(), "j").unwrap();
        let e = EnvoiParMorceaux { route: "/x", source: &f, total: 4, depart: 0, entetes: vec![], nom: "un fichier" };
        let r = ReglesMorceaux { fini: |v| v["complet"] == true, zero_sans_recu: true };
        c.envoyer_par_morceaux(&e, &r, &mut Cree(f.clone())).await.unwrap();
        m.assert();
    }

    /// Lot S4 (I3) : « ne se cale pas » et « n'a pas confirmé » sont propres à UN fichier (Conflit), jamais un arrêt.
    #[tokio::test]
    async fn un_envoi_qui_ne_se_cale_pas_ou_non_confirme_est_un_conflit() {
        let s = MockServer::start();
        s.mock(|w, t| {
            w.method(PUT).path("/api/jeux/v1/nc");
            t.status(200).json_body(serde_json::json!({"ok": true}));
        });
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("x");
        std::fs::write(&f, b"abcd").unwrap();
        let c = Client::nouveau(&s.base_url(), "j").unwrap();
        let e = EnvoiParMorceaux { route: "/nc", source: &f, total: 4, depart: 0, entetes: vec![], nom: "un fichier" };
        let r = ReglesMorceaux { fini: |v| v["complet"] == true, zero_sans_recu: true };
        let x = c.envoyer_par_morceaux(&e, &r, &mut MorceauxFixes(4)).await;
        assert!(matches!(&x, Err(Erreur::Conflit(m)) if m.contains("n'a pas confirmé")), "{x:?}");
    }
}
