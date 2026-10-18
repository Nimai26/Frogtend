# Contrat de l'API jeux v1 (Firehouse 2.16.1, `X-Api-Jeux-Version` 1.3 ; ajouts jusqu'à la 1.13 en fin de fichier)

> Reçu le 29/09/2026 de la session Firehouse (Shyrka), copié de son `docs/frogtend/BRIEF-AGENT-FROGTEND.md` § 3–4.
> C'est la référence de Frogtend pour l'API. Les réponses réelles relevées sont dans `EXEMPLES-API-JEUX-V1.md`.
> Les tests de Frogtend ne parlent jamais au vrai serveur (fixtures).

**Base** : `https://jeux.hikari-no-sekai.fr/api/jeux/v1/`

## Authentification

Jeton d'appareil (un par PC, créé dans le cockpit : ⬇️ DL ▸ 🧩 Extension navigateur ▸ « + Nouveau jeton »), en-tête
`Authorization: Bearer <jeton>`, rangé dans le coffre de Windows, jamais en clair ni au journal. **401** = révoqué ou
expiré : le dire, ne pas boucler. `/moi` le vérifie.

Chaque réponse porte **`X-Api-Jeux-Version`** (majeure = rupture, mineure = ajout compatible) : si la majeure diffère,
dire « serveur trop ancien » ou « trop récent ».

## Règles générales

- Un jeu caché à ce grade répond **404**, exactement comme un jeu inexistant (jamais 403).
- Un corps de requête qui n'est pas un objet JSON : **400**.
- **Sans jeton** : seulement `GET /themes` et `GET /skin/{nom}/video`. Tout le reste exige le jeton.

## Routes

| Route | Réponse et règles |
|---|---|
| `GET /moi` | `{ok, username, nom, grade, via, api: {version, firehouse}}` |
| `GET /plateformes` | `{ok, plateformes: [{nom, jeux}]}` |
| `GET /catalogue?plateforme=&depuis=&page=&par_page=` | `par_page` ≤ 500 (défaut 200). `{ok, total, page, par_page, suivante, jeux: [{id, titre, annee, plateforme, genres, statut, maj_le, jaquette (bool), jaquette_empreinte, versions (nombre), developpeur, editeur}]}`. Avec `depuis` (date ISO) : seulement ce qui a changé ; la **page 1** ajoute `ids_visibles` = TOUS les ids encore visibles pour ce jeton (le reste sort du cache). `jaquette_empreinte` change quand la jaquette change. |
| `GET /jeu/{id}` | `{ok, jeu: {…}}` (fiche à deux niveaux, voir plus bas) |
| `GET /media/{id}/jaquette?largeur=N` | L'image. `largeur` de 100 à 1000 (arrondie à la centaine) = miniature WebP, jamais agrandie. `ETag` + `Last-Modified` ; `Cache-Control: private` 7 jours. |
| `GET /skin/{nom}/video` | La vidéo de fond (webm), **sans jeton**, reprise `Range` (206). ⚠️ `video_api` (dans `/themes` et `/theme`) est un chemin depuis la **racine du serveur** : `https://jeux.hikari-no-sekai.fr` + `video_api`, jamais derrière la base de l'API. |
| `GET /fichier/{id}/{version}/{n}` | Le n-ième fichier d'une version, en flux, reprise `Range` (206), jusqu'à 150 Go. **409** si le fichier n'a plus la taille notée (ne pas boucler). `version` = `telechargement_id` ; une version trouvée au recensement a `telechargement_id` **0** (`type_source` « recensement ») : c'est un vrai numéro, pas une absence. |
| `GET /annexe/{id}/{i}?cle=<cle>` | Texte : `{ok, titre, texte}` (Markdown simple) ; fichier : flux. **409** si les annexes ont changé depuis la lecture de la fiche (la relire). |
| `POST /demandes {launchbox_id}` | Demander un jeu absent (admin et avancé ; réponse neutre si refusé). |
| `GET /recherche?texte=` | La base LaunchBox en plein texte, pour trouver un jeu à demander. |
| `GET /themes` (sans jeton) | Les 44 skins, jetons communs, encres résolues, `video`/`video_api` ; `ETag` faible → `If-None-Match` = 304. |
| `GET /theme` | Le skin de la personne : `{theme, demande, remplace, version, resolus_ok, video, video_api, jetons}` ; `remplace: true` = son thème enregistré n'existe pas, on applique `firehouse`. |
| `PUT /theme {"theme": "<nom>"}` | L'enregistre dans son compte ; nom inconnu = 400. |
| `POST /session {etat: "debut"\|"vivant"\|"fin", media_id?, streaming?}` | « vivant » toutes les 10 min pendant le jeu (expire seule à 30 min), « fin » à la sortie. Depuis n'importe quel PC : Firehouse reconnaît la machine et ne cède la carte graphique que si elle calcule pour lui ; sinon `{ok, session: null, machine: "autre", note}`. Finir la session d'un autre : 403 sauf admin. |
| `GET /emulateurs?plateforme=` | `{ok, plateforme, emulateurs: [{nom, site, recommande, ligne_de_commande, extensions: [..], bios}]}` (le recommandé d'abord ; MS-DOS : « DOSBox intégré ») ; 400 sans plateforme. |
| `POST /assistant {question, media_id?, historique?}` | `{ok, texte, actions_proposees: [{type, titre, details, risque}], outils, confidentialite}`. `type` ∈ installer, lancer, configurer, cheat, mod, telecharger, ouvrir_url, demander_jeu (liste fermée). `risque` ∈ aucun, faible, « modifie la configuration », « modifie le jeu ». Frogtend reste juge : il montre chaque action, ne l'exécute qu'après un oui, sauvegarde toute configuration avant. `historique` gardé côté Frogtend (20 derniers échanges ; rôles user/assistant ; 4 000 car. par message). Réponse en 10–40 s. 400 = question vide ; 404 = jeu inconnu/caché ; 429 = une question à la fois par personne ; 503 = modèle indisponible, réessayer. |

## La fiche d'un jeu — deux niveaux (décision de Seb)

Le **JEU** (vrai quelle que soit la version) et les **VERSIONS** (chacune SES fichiers et SES notes). Forme :
voir `EXEMPLES-API-JEUX-V1.md`, `GET /jeu/110`.

Qualités (la nature, qui décide de l'installation) : prêt à jouer (préinstallé, DOSBox inclus), ROM (nom No-Intro),
image disque (ISO/CHD, nom Redump), repack (installeur FitGirl/DODI : `setup.exe`, suivre les notes), installeur
d'origine, nature non dite. ⚠️ **Ne jamais renommer une ROM ni une image disque.**

## À venir (annoncé par Firehouse, pas encore dans l'API)

La base de jeux de Firehouse (188 792 jeux, un par système, titres français Wikidata, noms de ROM LaunchBox, résumés
français en cours de traduction) nourrira `/recherche`, `/jeu` et une recherche par **nom de ROM** (utile au
recensement). `launchbox_id` reste le contrat ; un `jeu_id` s'ajoutera à côté, jamais à sa place.

## Précisions sur `/catalogue?depuis=` (Firehouse, 29/09/2026)

- **Valeur** : le plus grand `maj_le` déjà reçu, recopié TEL QUEL (`YYYY-MM-DDTHH:MM:SS`, heure locale du serveur,
  sans décalage). Ne jamais le recalculer depuis l'horloge du PC ni le convertir en UTC. Le serveur compare des chaînes.
- **Comparaison** : `maj_le >= depuis` à partir de Firehouse 2.16.2 (`>` avant). Le dernier jeu reçu revient donc à
  chaque synchronisation : la mise à jour par id doit être idempotente (elle l'est : `INSERT OR REPLACE`).
- **Ce qui modifie `maj_le`** : un changement de jaquette (et de `jaquette_empreinte`), l'ajout ou la modification de
  versions, les métadonnées de la fiche.
- **Ce qui ne le modifie PAS** : un jeu qui redevient visible pour ce jeton (changement de grade). D'où la comparaison
  d'`ids_visibles` (page 1) avec le cache : un id visible inconnu déclenche une synchronisation complète, un id absent
  sort du cache.
- Un changement d'heure (été/hiver) peut renvoyer une heure de jeux en double, jamais en faire perdre.

## Précisions sur `POST /session` (Firehouse, 30/09/2026)

- PC qui calcule pour Firehouse (Venkman), `debut` ou `vivant` → `{ok, session: {par, machine: "venkman",
  media_id, streaming, debut, vu_le, expire_le}}`. **`session` non nulle = la carte graphique est cédée** (les
  travaux GPU de Firehouse se retirent tant que la session vit). Frogtend l'affiche.
- Autre PC → `{ok, session: null, machine: "autre" (ou son nom), note: "…rien à céder"}`.
- `fin` → `{ok, session: null}`.
- 403 : grade non permis (`jeux.session_grades`) ou session de quelqu'un d'autre. Ne pas réessayer (Frogtend note
  l'échec dans son journal et ne bloque jamais le jeu).

## Sauvegarde des profils (Firehouse 2.19.0, contrat 1.4, 30/09/2026)

> ⚠ **Remplacé par décision de Seb (30/09)** : Frogtend ne passe plus par le partage SMB (joignable seulement à la
> maison) mais par l'API (routes `/sauvegarde…` demandées à Firehouse). Ce qui suit reste pour mémoire : l'essai réel
> du partage a réussi (SMB 3.1.1 chiffré AES-128-GCM, signé, dossier en 0700).

> **Firehouse 2.20.0 (30/09/2026)** : le partage `[Frogtend]` et les comptes `frogtend-*` sont SUPPRIMÉS ;
> `GET /sauvegarde/identifiants` répond 503. Frogtend ne s'en sert plus.

- **Partage** : `\10.10.0.2\Frogtend` (ZFS de Shyrka, décision de Seb). Ouvrir DIRECTEMENT
  `\10.10.0.2\Frogtend\<username>` : la racine est illisible par conception.
- `GET /moi` → `sauvegarde: {partage, compte: "frogtend-<username>", disponible}`.
- `GET /sauvegarde/identifiants` → `{ok, compte, mot_de_passe, partage}` : réservé au JETON d'appareil (403 depuis
  le cockpit), 503 si le serveur ne gère pas les comptes. Compte créé à la demande. Mot de passe généré par
  Firehouse, jamais affiché : rangé dans le coffre Windows ; en cas d'échec d'authentification, le redemander UNE fois.
- **SMB 3.1.1 minimum, chiffrement et signature obligatoires** ; réseau de la maison seulement (hors de la maison :
  file locale envoyée au retour).
- **Arborescence** (plusieurs profils par compte) : `<username>/<profil>/<nom du PC>/` → `configuration.json`,
  `bibliotheque.json`, `parties/` + manifeste sha256. Frogtend écrit la version courante ; l'historique est gardé par
  les instantanés ZFS (horaires 24 h, quotidiens 14 j, hebdomadaires 8 sem.), copie hors site chiffrée chaque nuit.
- Premier essai réel : lecture/écriture d'un petit fichier dans `sebastien/<profil>/<PC>/`, avec l'accord de Seb.

## Sauvegarde par l'API (Firehouse 2.21.0, contrat 1.5, 30/09/2026) — EN SERVICE

Remplace le partage SMB (supprimé en 2.20.0). Relu par l'expert infra de Firehouse.

- **Jeton** : se crée dans 👤 Mon compte ▸ 🔌 Mes jetons, usage « 🎮 Frogtend » (valable sur `/api/jeux/v1/*`
  seulement, 401 ailleurs). Les jetons créés avant le 30/09 gardent leurs deux usages.
- `GET /moi` → `sauvegarde: {api: "/api/jeux/v1/sauvegarde", quota_octets, morceau_max: 8388608, documents: [...]}`
  et `avatar` (adresse, ou `""`). `GET /avatar` → PNG carré 512 px, 404 sans image.
- `GET /sauvegarde` → `{profils: [{profil, pc, derniere, octets}]}`.
- `GET|PUT /sauvegarde/{profil}/{pc}/document/{nom}` : JSON, 5 Mo au plus.
- `PUT /sauvegarde/{profil}/{pc}/fichier?chemin=` : par morceaux de **8 Mo au plus** (`Content-Range`), avec
  `X-Contenu-Sha256` (empreinte du fichier entier). Le `total` et l'empreinte sont **figés au 1er morceau** : un
  morceau qui annonce autre chose reçoit **409 `{recu: 0}`** → reprendre à zéro. Morceau mal placé : 409 `{recu}`.
  Dernier morceau renvoyé après une coupure sur un fichier déjà complet et identique : `{complet: true}`.
  `HEAD` → `X-Recu` (ce qui est déjà reçu). `GET` avec `Range`. `DELETE` retire le fichier. `DELETE /sauvegarde/{profil}/{pc}` retire tout le couple, documents compris (pas de DELETE pour un document seul : un PUT le remplace).
- **Noms** : `profil` et `pc` en ASCII `[A-Za-z0-9 ._-]`, 64 au plus (« Zoé » est refusé : Frogtend translittère en
  « Zoe »). `chemin` : normalisé en NFC, 240 octets au plus par segment, sensible à la casse (éviter deux noms qui ne
  diffèrent que par la casse : Windows les confondrait).
- ⚠️ **Encoder ENTIÈREMENT `chemin`** (« ( » → `%28`, « / » → `%2F`, espace → `%20`) : le proxy public filtre la
  chaîne de requête brute, un nom comme « concat (1).sav » non encodé serait bloqué (403 du proxy). Frogtend le fait
  (`source::encoder`, testé).
- **Plafond** : 100 Go par personne par défaut, vérifié à chaque morceau, documents compris ; 1 To pour tout l'espace.
- **Codes** : 400 refus (nom, en-tête), 404 absent, 409 `{recu}`, 413 morceau > 8 Mo ou document > 5 Mo, 503 migration
  en cours (réessayer dans une minute), 507 plein.

## Sources des émulateurs, triches et mods (Firehouse, contrat FIGÉ le 02/10/2026, routes à venir)

Besoins n° 13 et 14 (BESOINS-API.md). Implémentation côté Firehouse après l'accord de Seb ; **un champ absent =
« pas encore »**.

**14 — émulateurs** :
- `GET /emulateurs?plateforme=` : chaque émulateur gagne `id` (clé stable), `site_officiel`, `telechargement:
  {type: github|forgejo|gitlab|direct|page, depot, hote, url, motif_paquet (glob du paquet Windows), format:
  zip|7z|exe|msi}`, `forks: [même forme + actif, note]`, `source_info: emu-france|manuel`, `verifie_le`.
- `GET /emulateurs/catalogue` → `{maj_le, plateformes: [{plateforme, emulateurs: [...]}]}` (à garder en cache ;
  `maj_le` change seulement quand la base change).
- `GET /emulateurs/{id}/paquet` → Firehouse résout la dernière version : `{id, version, nom_fichier, url, taille,
  sha256|null, format, publie_le, page_release}` (cache 6 h). **409** (raison dite) si `type = page` : installation
  à la main ; **502** si la forge ne répond pas. Frogtend ne parle alors à aucune forge.
- Base modifiable dans Firehouse, amorcée depuis emu-france.com ; les forks Switch (Eden, Ryubing, Citron) y sont.

**13 — triches et mods, par jeu** :
- `GET /jeu/{media_id}/triche` → `{codes: [{cle, emulateur, format: cht|pnach|ini|gecko…, titre, nb_codes, source,
  correspondance: serial|crc|titre, confiance: 0–1, fichier}], cheat_engine: [{titre, version_jeu, source, page,
  fichier|null}], mods: [{nom, description, auteur, version, source, page, maj_le}], maj_le}`. Jeu caché → 404.
- `GET /jeu/{media_id}/triche/{cle}/fichier` → le fichier brut, prêt à poser dans le dossier de l'émulateur.
- Sources chez Firehouse : libretro-database (cheats), bases PCSX2, DuckStation, Dolphin (GeckoCodes) ; Cheat Engine :
  liens (fichier seulement s'il est librement téléchargeable) ; mods : PCGamingWiki (API gratuite), liens ModDB/GitHub ;
  Nexus : liens seulement.

**Précisions acceptées par Firehouse (02/10)** :
- `codes[]` : + `nom_fichier` (nom exact attendu par l'émulateur, ex. `SLUS-20946_1A2B3C4D.pnach`,
  `Crash Bandicoot (USA).cht`), `dossier` (relatif, séparateur « / », ex. `cheats/Beetle PSX HW`), `base`
  (`donnees` = dossier de données/portable de l'émulateur, cas courant ; `programme` = à côté de l'exe). La route
  fichier envoie aussi `Content-Disposition: attachment; filename="…"`. RetroArch : cœur = nom de son dossier, jeu =
  nom du fichier de jeu sans extension.
- émulateurs et forks (`/emulateurs`, `/emulateurs/catalogue`, `/emulateurs/{id}/paquet`) : + `programme` (exe à
  lancer, relatif à la racine décompressée, « / ») et `portable: {type: fichier|dossier|aucun, nom}`. Repli de
  Frogtend : chercher l'exe dans l'archive, demander s'il y en a plusieurs.

## Contrat 1.7 → 1.13 (reçu le 16/10/2026 de la session Firehouse, texte de son brief § 3)

> Reçu par message de la session Firehouse « FIREHOUSE 2 » (nuit du 15 au 16/10). Les exemples réels suivront ;
> d'ici là, un champ absent de ce texte n'existe pas.

### 1.6 (03/10) — les visuels des jeux
Voir l'exemple réel consigné dans `EXEMPLES-API-JEUX-V1.md` (15/10). Depuis la 1.6, Firehouse ne sert plus que les
images RAPATRIÉES sur son disque (avant : à la volée depuis LaunchBox). Un jeu dont les images ne sont pas encore
rapatriées a `/jeu/{id}/medias` → `[]` et `/media/{id}/…` → 404 : c'est le cas de Dune (110) et de tout MS-DOS au
16/10 (rapatriement = téléchargement, soumis à l'accord chiffré de Seb).

### 1.7 (04/10) — les documents
- `/catalogue` : chaque jeu porte `documents` (comptes par type, ex. `{"livret": 1, "musique": 1, "video": 2}`) et
  `documents_empreinte` (change dès qu'un document est ajouté, retiré ou remplacé → relire `/jeu/{id}/documents` ;
  À PART de `medias_empreinte`, qui ne concerne que les images).
- `GET /jeu/{id}/documents` → `[{id, type, langue, format, taille, pages, duree_s, empreinte (sha256), source}]` +
  `documents_empreinte` — jamais un chemin. Types : `livret` (PDF, pages), `musique` (MP3, duree_s), `video` (MP4,
  duree_s), `video_theme` (MP4 : la vidéo plein écran de LaunchBox/Big Box). `langue` vide = inconnue. Un document
  retiré de la source reste listé et servi. (2.57.0, sans changement de contrat) `source` peut valoir `emumovies` à
  côté de `launchbox_local` ; jamais deux documents du même type pour un jeu : celui de LaunchBox prime.
- `GET /jeu/{id}/document/{doc}` → un document DE CE jeu (sinon 404), reprise `Range` ; `Content-Type` du format
  VÉRIFIÉ (`application/pdf`, `audio/mpeg`, `video/mp4`), `ETag` = empreinte, `Cache-Control: immutable` (un contenu
  nouveau = un nouvel id).

### 1.8 (04/10) — le conseil par système
- `GET /emulateurs?plateforme=` : `conseil` = le choix PAR DÉFAUT de la maison — `{plateforme, emulateur (id des
  sources), coeur ("xxx_libretro" si RetroArch, sinon null), ligne_de_commande (la SIENNE), alternatives [{nom,
  emulateur, coeur, quand, site}], bios [{fichier, racine, chemin, facultatif, coffre, note}], version_attendue, note,
  origine, maj_le}` ou `null` (plateforme sans conseil : jamais deviné). Le `conseil` PRIME ; ensuite la liste
  LaunchBox (`recommande` = avis LaunchBox, informatif). Ne JAMAIS reprendre la ligne de commande d'une ligne
  LaunchBox pour appliquer le conseil.
- `GET /emulateurs/catalogue` : chaque plateforme porte aussi son `conseil` ; `maj_le` change aussi quand un conseil
  est corrigé.
- `type` d'un BIOS (sens fixé) : `fichier` = nom EXACT ; `dossier` = dossier à poser tel quel (aucun servi ainsi) ;
  `motif` (`scph*.bin`) ; `installation` = firmware à installer par le MENU de l'émulateur (jamais déposé, chemin
  vide) ; `archive` (1.11) = ZIP à extraire dans racine/chemin (blueMSX.zip → `Machines/` et `Databases/` dans
  `system/`).
- `ligne_de_commande` vide (autonomes : Dolphin, PCSX2, RPCS3, Eden, BigPEmu, MAME…) : lancer le programme de la
  source avec le fichier du jeu en argument.
- `racine` : `system_retroarch` (`system/` de RetroArch, chemin relatif ex. `dc/naomi.zip`) | `dossier_emulateur`
  (PCSX2 : `bios/`) | `installation_par_l_emulateur` (PS3UPDAT.PUP dans RPCS3, firmware Switch dans Eden « Install
  Firmware »).
- `coffre: true` (Wii U `keys.txt`, Switch `prod.keys`) : un SIGNAL ; les clés ne sont JAMAIS servies ; Frogtend dit
  qu'elles manquent.
- `facultatif` = ce que demande la plateforme, pas ce que déclare le cœur.
- `version_attendue` (MAME = celle du romset, 0.289) : `/emulateurs/mame/paquet` sert CETTE version, jamais « la
  dernière ». `/emulateurs/{id}/paquet` : toujours la version STABLE.

### 1.9 (04/10) — les BIOS (besoin 19, FIGÉ ; précisé en 1.10–1.11)
- `GET /bios` → les BIOS SERVIS : `[{sha1, md5, taille, noms [{nom (chemin libretro, ex. "dc/naomi.zip"), systeme}],
  verification}]` — jamais un chemin.
- `GET`, `HEAD /bios/{sha1}?nom=` → UN BIOS servi, reprise `Range` (le firmware PS3 pèse 206 Mo) ; `nom` = l'un de SES
  noms (sinon son premier) en `Content-Disposition` ; `Cache-Control: private, immutable`. 404 = inconnu OU pas
  (encore) servi.
- Dans le conseil (`GET /emulateurs?plateforme=`), chaque BIOS dit s'il est `disponible` (calculé à chaque lecture) :
  type `fichier`, `installation` ou `archive` → disponible, et s'il l'est `sha1`, `taille`, `url`
  (`/api/jeux/v1/bios/{sha1}?nom=…`) à télécharger puis déposer sous racine/chemin (une archive : à EXTRAIRE dans
  racine/chemin, chemin vide = la racine même) ; type `motif` → disponible + `fichiers [{nom, sha1, taille, url}]` =
  tous les BIOS servis du système ; type `dossier` → `disponible: false` ; une CLÉ (`coffre: true`) → toujours
  `disponible: false`, sans url. Rapprochement par le CHEMIN puis par le nom DANS le système de la plateforme — jamais
  par un nom d'origine.
- `verification` (1.10) : `system_dat` | `source_officielle` (firmware PS3 Sony) | `nca_contentid` (firmware Switch) |
  `aucune` (gardé à la main).

### 1.10 (04/10) — le firmware Switch
`firmware-switch.zip` (type `installation`) = zip REPRODUCTIBLE des NCA, chacun vérifié par son ContentId ;
« Install Firmware » d'Eden ; version « 18.1.0 » d'après le nom du dossier (non vérifiable sans clés) ; ⚠️ Eden
REFUSE l'installation sans `prod.keys` : le dire avant de proposer. `verification: nca_contentid`.

### 1.11 (04/10) — archives et PS2
Type `archive` (blueMSX). PS2 = 3 BIOS (Europe par défaut, USA et Japon facultatifs) dans `bios/` de PCSX2 ; un seul
suffit (PCSX2 reconnaît le contenu) ; modifier `PCSX2.ini` clé par clé (`[Filenames] BIOS =
ps2-0250e-20100415.bin`, `[Folders] Bios = bios`, `[EmuCore] EnableFastBoot = true` ; à la création ajouter `[UI]
SettingsVersion = 1`) ; ne jamais toucher aux `.nvm`. BIOS « admin » (gardés à la main avec nom + système)
rapprochables.

### 1.12 (05/10) — les cœurs RetroArch servis
- `GET /emulateurs/retroarch/coeurs` → `{coeurs: [{coeur, source, version, version_retroarch, build_le, sha256_zip,
  sha256_dll, taille, recupere_le, url}]}`.
- `GET|HEAD …/retroarch/coeurs/{coeur}/fichier` = un zip avec `<coeur>.dll` à la racine : DÉZIPPER dans `cores\` de
  RetroArch, vérifier `sha256_zip` (et `sha256_dll` après extraction). Reprise `Range` ; `ETag` = `sha256_zip` ; PAS
  immuable (revérifier l'empreinte publiée).
- `source` `stable` = paquet stable RetroArch (`version` = `version_retroarch` = « 1.22.2 ») ; `source` `nightly` = pris
  une fois et FIGÉ (« nightly 2026-10-04 », `version_retroarch` null : applewin, emuscv, hatarib).
- Le conseil porte `paquet {url, sha256, taille, version, source}` sur son cœur principal ET sur chaque alternative qui
  a un cœur (null si non servi : RetroArch l'installe lui-même). Aucun `.info` servi.

### 1.13 (05/10) — le romset MAME
Plateforme « Arcade », MAME 0.289 (`version_attendue`). Chaque version d'une fiche Arcade porte `mame {nom (NOM
COURT, ex. « mslug »), version, parent, complet}` et ses fichiers : le zip AUTONOME du jeu (BIOS et devices DEDANS)
et, s'il en a, ses CHD. Chaque fichier porte `dossier` : vide = à plat dans le rompath ; non vide = dans
`<rompath>/<dossier>/` (une CHD vit dans le dossier de son PARENT). Lancer `mame <nom> -rompath "<rompath>"` (pas le
chemin du zip). `complet: false` = une CHD manque encore : dire « incomplet ». Les clones sont des VERSIONS de la
fiche du parent.

### 1.14 (18/10, Firehouse 2.72.1, en-tête `X-Api-Jeux-Version: 1.14`) — fiche réunie : « fusionné dans N »
Reçu par message de la session Firehouse (besoin 22 de Frogtend, demandé par Seb le 18/10).
- `GET /jeu/{ancien}` et TOUTES les routes d'un jeu (`/medias`, `/documents`, `/media`, `/fichier`, `/annexe`,
  `/triche`, `POST /assistant` avec un ancien `media_id`) → **410** `{ok: false, fusionne_dans: N, le, detail}` ; une
  chaîne A→B→C donne directement C.
- Page 1 de `/catalogue` : `fusions: [{ancien, nouveau, le}]`, TOUJOURS présente (aussi sans `depuis` ; filtrée par
  `le >= depuis` si donné ; jamais par `plateforme`).
- Fail-closed : jeu caché à ce jeton, ou ancienne fiche cachée/réservée sans droit → 404, identique à un numéro
  inconnu. Un numéro repris par une fiche vivante sert cette fiche (200).
- Les `telechargement_id` des versions sont GARDÉS tels quels sur la nouvelle fiche ; exceptions : 3 versions de stock
  déplacées plus tard vers une troisième fiche (repli : chercher par `sha1`/`crc32`), et deux versions de recensement
  `telechargement_id: 0`.
- Exemple réel (Firehouse) : `GET /jeu/1927` → 410 `{ok: false, fusionne_dans: 324, le: "2026-10-05T14:02:37"}` ;
  154 fusions au 18/10. Un client < 1.14 peut traiter le 410 comme un 404.

### 1.15 (18/10, Firehouse 2.76.0, en-tête `X-Api-Jeux-Version: 1.15`) — les infos du jeu
Reçu par message de la session Firehouse (sur demande de Seb), en service depuis le 18/10 à 22 h 37. Ajout compatible :
un client < 1.15 ignore le champ.
- `GET /jeu/{id}` porte `infos: {date_sortie, type_sortie, esrb, pegi, note, joueurs_min, joueurs_massif}` :
  - `date_sortie` : `AAAA`, `AAAA-MM` ou `AAAA-MM-JJ`, `""` si inconnue ;
  - `type_sortie` : `Released`, `Unreleased`, `Early Access`, `Homebrew`, `ROM Hack`, `Unlicensed`, `DLC` ou `""` ;
  - `esrb` : libellé LaunchBox (`E - Everyone`, `E10+ - Everyone 10+`, `T - Teen`, `M - Mature`, `AO - Adults Only`,
    `EC - Early Childhood`, `RP - Rating Pending`, `Not Rated`) ou `""` ;
  - `pegi` : 3, 7, 12, 16, 18 ou `null` (aucun jeu n'en a encore : seulement par une correction d'admin) ;
  - `note` : 0 à 5 (0 est une vraie note) ou `null`, NON arrondie (ex. 3.93827160493827) ;
  - `joueurs_min` : entier ou `null` ; `joueurs_massif` : booléen.
- `joueurs` et `cooperatif` restent à leur place. `/catalogue` ne change pas : `infos` n'est qu'au détail.
- Corrigeables par un admin (verrouillé contre la mise à jour LaunchBox de la semaine, appliqué aux fiches publique et
  réservée) ; une correction fait bouger `maj_le`.
- ⚠️ Lues EN DIRECT dans la base des jeux : la mise à jour LaunchBox de la semaine peut changer une info NON corrigée
  sans faire bouger `maj_le` → pour un écran qui s'en sert, relire le détail à l'ouverture de la fiche.
- Exemple réel (Dune, `launchbox_id` 14133) : `"infos": {"date_sortie": "1992", "type_sortie": "Released", "esrb":
  "Not Rated", "pegi": null, "note": 3.93827160493827, "joueurs_min": null, "joueurs_massif": false}`.
- Côté Frogtend : pas encore affiché (à proposer à Seb : fiche du jeu — date, classement d'âge, note).

### Note 2.58.0 (contrat inchangé)
Un jeu peut SORTIR du catalogue (absent de `ids_visibles`) parce que sa fiche a été RÉUNIE à celle du même jeu
(« Probotector 2 » → « Super C ») : ses versions sont sur l'autre fiche ; les données locales liées à l'ancien id ne
sont pas redirigées (une indication « fusionné dans N » serait un ajout 1.14 — à demander à Seb si gênant).
