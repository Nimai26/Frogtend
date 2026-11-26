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

Précisions de Firehouse (19/10, besoin 23) : `fusions` est filtrée par grade, fail-closed (les deux fiches visibles à
ce jeton, jamais un ancien numéro repris par une fiche vivante). Un `telechargement_id` négatif est une vraie version :
`-<n° de ROM du stock>` (-1 existe), `-(10¹⁵ + …)` pour une version MAME ; `0` = recensement.

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

### 1.16 (19/10, Firehouse 2.78.0, en-tête `X-Api-Jeux-Version: 1.16`) — lignes de commande des autonomes, plateformes
Reçu par message de la session Firehouse (sur demande de Seb), en service le 19/10 à 0 h 11. Ajout compatible.
- `conseil.ligne_de_commande` d'un émulateur AUTONOME est maintenant remplie (avant : toujours vide) : les ARGUMENTS
  après le `programme` de la source, vérifiés dans les sources ou la doc officielle de chaque émulateur :
  Dolphin `-b -e "%rom%"` ; PCSX2 `-batch -fullscreen -- "%rom%"` ; RPCS3 `--no-gui --fullscreen "%rom%"` ; Cemu et
  Eden `-f -g "%rom%"` ; xemu `-full-screen -dvd_path "%rom%"` (non vérifié : XISO peut-être exigé) ; Xenia Canary
  `--fullscreen "%rom%"` ; EKA2L1 `-f --mount "%rom%"` ; BigPEmu `"%rom%" -localdata` (l'image EN PREMIER) ; MAME
  (Arcade) `%rom_nom% -rompath "%rom_dossier%"` ; ScummVM `-f -p "%rom%" --auto-detect` (ici `%rom%` = le DOSSIER du
  jeu) ; machines de salon par MAME `studio2 -cart "%rom%"` (crvision, apfm1000, vsmile, pv1000 : même forme ; gamecom
  `-cart1 "%rom%"`). Vide = non fournie (Azahar, Vita3K, shadPS4, Apple I) : programme + fichier, comme avant.
- Règles : remplacer `%rom%` (le fichier), `%rom_nom%` (son nom sans extension), `%rom_dossier%` (son dossier, SANS
  barre finale) ; une variable inconnue = ne pas lancer ; jamais de shell ; MAME : dossier de travail = dossier de MAME
  (une machine de salon cherche ses ROM système dans `roms/`).
- 21 plateformes de plus au conseil : Casio PV-1000, RCA Studio II, VTech CreatiVision, Tiger Game.com, APF
  Imagination Machine, VTech V.Smile, Apple I, Fairchild Channel F, Watara Supervision, Mega Duck, NEC PC-9801, Sharp
  X1, Sega Dreamcast VMU, Bandai Sufami Turbo, J2ME ; d'avance : Nintendo 3DS, Sony Playstation Vita, Sony Playstation
  4, SNK Neo Geo CD, Atari 800, Sharp X68000.
- BIOS des machines MAME de salon : un zip système dans `roms/` de MAME (`racine: dossier_emulateur`, `chemin:
  roms/studio2.zip`…), servis : studio2, crvision, gamecom, apfm1000, vsmile (apple1 non servi, incomplet).
- Valeurs changées : Emerson Arcadia 2001 → cœur `amiarcadia_libretro` ; clés Wii U `chemin: portable/keys.txt`
  (toujours `coffre: true`) ; 10 cœurs de plus servis (freechaf, potator, sameduck, np2kai, x1, vemulator, squirreljme,
  neocd, atari800, px68k).
- Exemple réel (19/10) : `GET /emulateurs?plateforme=Nintendo GameCube` → `conseil.emulateur: "dolphin"`,
  `conseil.ligne_de_commande: "-b -e \"%rom%\""`.
- ⚠️ Côté Frogtend (relevé du 19/10, avant adaptation) : `ligneDe` (src/lib/emulateurs/pret.ts) colle les options de
  Frogtend devant `%rom%` de la ligne du conseil → options EN DOUBLE pour un autonome (Dolphin `-b -e … -b -e`) ;
  `%rom_nom%`/`%rom_dossier%` ne sont pas remplacés ; MAME construit sa propre commande. ✅ Adapté en 0.69.0.
- Réponses de Firehouse (19/10) : `%rom_dossier%` à la racine d'un disque — correction de Seb : PAS d'exception au
  contrat (les jeux sont dans les dossiers de jeux choisis, la racine n'est pas un cas prévu ; si quelqu'un la choisit
  quand même, c'est accepté sans règle). Frogtend y garde `D:\` (« D: » seul voudrait dire le dossier courant de D:). xemu : XISO EXIGÉ (doc officielle « Disc Images » : un ISO Redump n'est pas
  compatible ; conversion `xdvdfs pack jeu.iso`, sans le correctif média des .xbe) ; la `note` du conseil Xbox le dit
  (Firehouse 2.78.2) ; aucun jeu Xbox au stock au 19/10. Le `username` d'un compte ne change jamais (seul le grade).

### 1.18 (19/10, Firehouse 2.81.2, en service le 19/10 à 16 h 37) — patchs RPCS3 (besoin 24)
Reçu par message de la session Firehouse, sur décision de Seb (un essai, puis une vérification par jour). Ajout
compatible.
- Firehouse garde la liste officielle `patch.yml`, prise comme RPCS3 la prend (rpcs3.net, moteur 1.2) et contrôlée
  comme lui (moteur, sha256, YAML lu, taille) ; vérifiée chaque jour, l'ancienne copie reste servie si une nouvelle est
  refusée. Au 19/10 : 1,2 Mo, 4 204 patchs pour 1 088 numéros de série.
- **État de la liste** : `GET /api/jeux/v1/emulateurs/rpcs3/patchs[?v=1.2]` → `{version_moteur, sha256, taille,
  maj_le, vu_le, fichier}` ; 404 si ce moteur n'a pas de copie ; `v` invalide → 404.
- **Le fichier** : `GET …/rpcs3/patchs/fichier?v=1.2` → `patch.yml` brut ; reprise `Range`, `HEAD`, `ETag` = sha256 ;
  **503** pendant un remplacement (réessayer plus tard). Vérifier le sha256 AVANT de remplacer
  `<config RPCS3>/patches/patch.yml`. Ne JAMAIS toucher `imported_patch.yml` (à l'utilisateur) ; `patch_config.yml` seulement pour activer un patch,
  à la demande de la personne, après une copie de sauvegarde (lecture côté Frogtend, 19/10 : les deux phrases du
  message de Firehouse se lisent ainsi).
- **Par jeu** : `GET …/rpcs3/patchs?serial=BLES00932[&version_jeu=01.02]` → `{patchs: [{hash, description, titre,
  versions_du_jeu, auteur, notes, patch_version, groupe}]}`. `serial` : 9 lettres ou chiffres (TITLE_ID du PARAM.SFO),
  `version_jeu` = APP_VER ; serial invalide → 400. `versions_du_jeu` : une liste, ou `"All"`. `hash` = clé de RPCS3
  (`PPU-…`, `SPU-…`, `PRX-…`, `OVL-…`). Les patchs génériques (titre et série « All ») ne se cherchent pas par série :
  ils sont dans le fichier et RPCS3 les applique lui-même.
- **Activer un patch** : écrire SON `patch_config.yml` (après une copie de sauvegarde).
- Exemple réel : BLES00932 (Demon's Souls) → 5 patchs (Aspect Ratio, Disable Motion Blur, Enable FreeCam, Skip Intro
  Videos…) ; relevés complets à la fin de EXEMPLES-API-JEUX-V1.md (côté Firehouse).
- **Besoin 25 (mods), décisions de Seb** : Nexus SANS clé d'API (payante, et interdite pour une application partagée ;
  ⚠ décision du 19/10 remplacée : clé PERSONNELLE de chaque utilisateur, gratuite, voie `nxm://`, livrée en 0.82.0)
  — Frogtend montre le LIEN de la page Nexus, chacun télécharge avec son compte ; un mod servi en FICHIER à tous :
  seulement sous licence libre explicite (GitHub d'abord), empreinte vérifiée, validé un par un par un admin, jamais
  pour un jeu en ligne, jamais mis à jour sans nouvelle validation. Rien de livré pour le 25 au 19/10 (annoncé en 1.19).

### 1.19 (19/10, Firehouse 2.82.0, en service le 19/10 à 17 h 50) — codes et mods (besoin 25)
Reçu par message de la session Firehouse ; détail dans son BRIEF § 1.5 et § 3 (« Codes et mods (1.19) »), relevés
réels à la fin de son EXEMPLES-API-JEUX-V1.md. Décisions de Seb (19/10) :
1. **Frogtend télécharge les MODS lui-même** (exception au « jamais sur Internet », limitée aux mods ; Firehouse n'en
   garde aucune copie). GitHub : `fichiers[].url`, sha256 vérifié quand il est donné (différent : rien installé, liste
   redemandée — relue chaque jour). Nexus : la voie OFFICIELLE `nxm://` (s'enregistrer pour les liens « Mod Manager
   Download », puis demander le lien à l'API Nexus avec la clé d'API PERSONNELLE de l'utilisateur — gratuite, dans les
   réglages de son compte Nexus ; chacun son compte et sa clé, rangés chez lui). JAMAIS d'automate sur les pages de
   Nexus (conditions d'usage art. 11 : robots interdits, compte suspendu).
2. Aucune validation par un admin ni filtre adulte sur les mods. Seul refus de principe : `refus: "anti_triche"` (jeu
   à anti-triche : le lien reste, aucun fichier). `licence`, `licence_libre`, `pre_version`, `depot_deplace` : AFFICHÉS
   avant le oui, jamais bloquants. Autres refus = Firehouse ne sait pas QUEL fichier donner (`sans_release_precise`,
   `liens_contradictoires`, `github_indisponible`…).
3. `a_confirmer: false` seulement quand la consigne PCGamingWiki dit d'extraire à la racine du jeu, pour une release
   précise et (si elle en nomme un) pour CE fichier ; sinon montrer `consignes_url` et demander où poser. Frogtend
   choisit ajouter ou remplacer en comparant l'archive au dossier du jeu (sauvegarde avant).
4. Nouveau `references` : liens seulement CITÉS par la page, jamais à télécharger. `liens` garde son sens (1.18).
5. Jeu sans fiche (25a) : `GET /api/jeux/v1/triche/recherche?titre=&plateforme=&rom=&serial=&steam_appid=` (au moins
   un) → même forme que `/jeu/{id}/triche` + `patchs_rpcs3` (null hors numéro PS3, [] sans patch ; tiret accepté).
   Sans plateforme : seul le numéro rapproche, aucun mod (mods seulement pour une plateforme PC ou un `steam_appid`).
   Même réponse quel que soit le grade, jeu connu de Firehouse ou non. Fichier d'un code :
   `GET /api/jeux/v1/triche/{cle}/fichier[?rom=]`, en reprenant le `fichier` de la liste tel quel.
6. ⚠️ Les clés de code changent de forme PARTOUT (y compris `/jeu/{id}/triche`) : `source-<16 hexa>`, stables d'une
   semaine à l'autre ; une ancienne clé numérique → 404 : toujours reprendre la clé dans la liste.
7. Un identifiant PPSSPP partagé par deux jeux (19 cas, dont ULES-00301) → UN seul .ini qui contient les deux blocs.
- Côté Frogtend (relevé du 19/10) : les clés de code sont lues dans la liste à chaque fois (`codes[].cle`, texte) et
  jamais gardées : la nouvelle forme passe telle quelle. ✅ Mods GitHub adaptés en 0.81.0 : un fichier n'est installé
  d'un clic que si `methode: extraire`, `cible.base: jeu`, archive `.zip`, `taille` donnée, `refus` nul ; `a_confirmer`
  respecté (consignes, choix racine / dossier du programme, jamais au canapé) ; `licence`, `licence_libre`,
  `pre_version`, `depot_deplace`, empreinte absente AFFICHÉS avant le oui ; `references` montrées comme pages citées,
  jamais téléchargées ; `sources_url` et les autres méthodes ignorés (la page reste). Recherche d'un jeu sans fiche :
  ✅ 0.81.1 (jeu importé : `titre`, `plateforme` toujours, `rom` = nom du fichier seul, `serial` PS3 ; le `fichier` d'un
  code repris tel quel, seule une route `/triche/<clé>/fichier` admise ; relevé réel : Sonic → 8 codes RetroArch,
  Super Mario World avec `rom` → 1 code, `fichier` avec `?rom=`). Nexus (`nxm://`, enregistré par l'installeur, ✅ Seb) : 0.82.0.

### 1.20 (19/10, Firehouse 2.84.0, en service le 19/10 à 21 h 37) — patchs RPCS3 en français (besoin 26)
Reçu par message de la session Firehouse ; accord chiffré de Seb après un échantillon de 3. Ajout compatible.
- Chaque patch de `GET …/rpcs3/patchs?serial=` (et de `patchs_rpcs3` dans `/triche/recherche`) porte en plus
  `description_fr` et `notes_fr`, traduits UNE fois par Firehouse (touches, chiffres, versions, sigles laissés tels
  quels). `null` = pas encore traduit (montrer l'anglais, marqué « en anglais ») ; `""` = texte d'origine vide.
- `description` et `notes` restent l'anglais d'ORIGINE : c'est `description` que RPCS3 attend dans
  `patch_config.yml` — n'y écrire JAMAIS la traduction.
- Relevé réel (BLES01227) : « Boot to title screen » → « Démarrage jusqu'à l'écran titre » ; « Developer Menu » →
  « Menu développeur » (note : « Appuyez sur la croix directionnelle gauche à tout moment pour accéder au menu
  développeur. ») ; « Disable Ambient Occlusion » → « Désactiver l'occlusion ambiante ».

### 1.21 (20/10, Firehouse 2.89.0, en service le 20/10 à 15 h 22, en-tête `X-Api-Jeux-Version: 1.21`) — réconciliation (besoin 28)
Reçu par message de la session Firehouse (référence : `docs/frogtend/BRIEF-AGENT-FROGTEND.md` § 3 « Réconciliation
(1.21) » du dépôt Firehouse). La route ne fait que LIRE.
- `POST /api/jeux/v1/reconcilier` `{"fichiers": [{cle, groupe?, plateforme, nom, taille, sha1?, crc32?, entrees?:
  [{nom, crc32, taille}]}]}`.
  - Lots ≤ 500 fichiers (413 au-delà, la limite est dans le message), ≤ 4 096 entrées par fichier, ≤ 50 000 entrées
    par appel (réglables par l'admin : `jeux.reconcilier.lot_max` / `entrees_max` / `entrees_lot_max`).
  - `nom` = nom SEUL, jamais un chemin (400) ; `cle` = la nôtre, unique dans l'appel, rendue telle quelle ; `groupe`
    relie les fichiers d'un même jeu (.cue + .bin, .m3u + disques).
  - `plateforme` (nom LaunchBox) INDISPENSABLE au rapprochement par contenu et par nom (sans elle : seuls sha1 et
    signature MAME répondent).
- Sens des empreintes : `sha1` = le fichier TEL QU'IL EST sur le disque (un .7z entier) ; `entrees` = nom SANS
  dossier + crc32 + taille lus dans le répertoire du zip/7z (sans décompresser) ; fichier en clair (.nes, .sfc,
  .lnx) : `crc32` + `taille` du fichier ENTIER, en-tête compris.
- MAME : nom court (.zip) + l'ENSEMBLE des entrées ; « exact » seulement si l'ensemble (crc, taille) = celui du
  romset 0.289 de Firehouse (zips autonomes, BIOS et parent inclus) ; split ou autre version = « probable » (motif
  « autre version du romset MAME ») ; zip MAME SANS entrées = « probable » par le nom court.
- Réponse, une entrée par fichier : `{cle, statut: exact|probable|inconnu, correspondance:
  sha1|contenu|mame_ensemble|nom_taille|nom|null, confiance (1,0 exact ; 0,6 probable 1 candidat ; 0,5 plusieurs ; 0
  inconnu), media_id, telechargement_id, motif, candidats: [{media_id, telechargement_id}] (≤ 10 ; plusieurs = jamais
  exact), groupe?}` + `groupes: [{groupe, statut, media_id, telechargement_id, manquants}]` (exact seulement si TOUS
  les fichiers du groupe le sont). « exact » par contenu = MÊME ensemble d'entrées (partiel ou augmenté = probable).
  Plages de `telechargement_id` : stock = -<n° de ROM>, MAME ≤ -10^15.
- Règles : Frogtend ne rattache SEUL que les « exact » ; le reste est MONTRÉ à la personne. Seules les fiches que CE
  jeton voit sont cherchées (un jeu réservé ou caché répond comme un inconnu). 1re version (décision de Seb) :
  cartouches (stock No-Intro) + MAME ; disques / PS3 / PC = au mieux « probable ».
- État de l'index au 20/10 ~16 h 30 : sha1 des cartouches prêt ; le CONTENU des 16 512 archives du stock en lecture
  (fin vers 20 h 05), puis les signatures MAME (dans la soirée). Premier vrai essai APRÈS ~20 h 30 : Firehouse
  confirmera la fin des deux passages, avec un relevé réel.
- Aussi (2.90.0, 20/10) : une fiche de la bibliothèque RÉSERVÉE (admin seul) a `launchbox_id: null` au détail (clé
  LaunchBox interne) ; le sens du champ ne change pas — ne pas en déduire « jeu inconnu ».

### 1.22 (20/10, Firehouse 2.92.0, en service le 20/10 vers 18 h 25, en-tête `X-Api-Jeux-Version: 1.22`) — MAME par machine et CHD (besoin 29)
Reçu par message de la session Firehouse (référence : BRIEF § « 1.22 » et `docs/frogtend/EXEMPLES-API-JEUX-V1.md`
§ 1.22 du dépôt Firehouse). Index : 8 326 machines servies, 124 726 ROM requises, 298 avec un disque.
- Pour une requête 1.21 (sans `chd_sha1`), le HAUT de chaque réponse est INCHANGÉ. S'ajoute, pour un fichier
  Arcade : `machines: [{nom_court, statut: exact|probable, correspondance: mame_requis|chd_sha1, media_id,
  telechargement_id, manquantes, dependances: [{nom_court, type: bios|device}], disques: [{nom, chd_sha1, present}],
  complet_chez_nous}]`.
- Ce que MAME peut lancer avec CE zip : zip du PARENT (fusionné ou non) → le parent ET ses clones ; zip d'un CLONE →
  ce clone SEUL.
- exact = TOUTES les ROM requises de la machine (listxml 0.289, hors BIOS et ROM optionnelles) dans `entrees` ;
  probable = au moins la MOITIÉ ET, pour un clone, au moins une de ses ROM PROPRES ; sinon pas listée. Envoyer TOUTES
  les entrées (noms et sous-dossiers ignorés, doublons permis).
- `dependances` (précisé par Firehouse le 20/10 : SEULEMENT ce qui a des ROM à soi — le BIOS en remontant `romof`,
  puis les devices avec au moins une ROM, parcours récursif, sans doublon : tout ce qui est listé et absent est un
  vrai manque) : BIOS / devices à avoir À CÔTÉ (ex. neogeo) — la machine est exacte quand même ; à Frogtend de
  vérifier qu'ils sont chez la personne.
- CHD : champ `chd_sha1` (40 hexa) = SHA1 GLOBAL de l'en-tête (v5 offset 84, v4 48, v3 80) ; JAMAIS dans `sha1`.
  Rend toutes les machines qui demandent ce disque ; au fichier : exact si une seule, sinon probable + candidats —
  SAUF si le zip du MÊME `groupe` désigne l'une d'elles : alors exact sur elle. Nouvelle `correspondance` au
  fichier : `chd_sha1`. `disques[].present` : vrai/faux si le CHD est dans le même `groupe` que le zip, sinon null ;
  `complet_chez_nous` : faux si la version de Firehouse n'a pas tous ses disques.
- ⚠️ Le verdict de GROUPE reste calculé sur le haut de réponse (sémantique 1.21) : pour l'arcade, LIRE `machines`
  (exact au niveau machine), pas `groupes`.
- Limite connue : un set SPLIT n'est jamais exact pour un clone seul (chaque zip est jugé seul).
- Relevé réel (identifiants) : 2020bb.zip fusionné → machines 2020bb, 2020bba, 2020bbh exact ×3, dépendance neogeo
  (bios), haut probable / nom ; 2020bba.zip → 2020bba seul exact ; CHD seul 015cb0e6… → raycrisj exact (bios
  coh3002t) ; groupe fbaitmca.zip + CHD 0b567bf2… → CHD tranché sur fbaitmca, present = true, groupe probable.
  media_id : 2020bb 12984 (telechargement_id parent -1234836732104409, 2020bba -1047494276407722, 2020bbh
  -1195061964255313) ; raycrisj 12484 / -1281373763000399 ; fbaitmca 15103 / -1218799250261530. JSON exact reçu le
  20/10 : `src-tauri/fixtures/simule/reconcilier-122.json`.

### 1.23 (20/10, Firehouse 2.94.0, en service le 20/10 à 22 h 29, en-tête `X-Api-Jeux-Version: 1.23`) — rôles des fichiers, forme d'installation, multi-disques
Reçu par message de la session Firehouse (BRIEF § 1.23 du dépôt Firehouse). Contexte : Seb fait rapatrier les jeux de
Venkman dans Firehouse (4 392 jeux d'ordinateurs/portables rangés le 20/10 ; puis CPC, consoles à disques, Wii/Wii
U/PS3/Switch, PC) ; les consoles récentes vont dans une 2ᵉ bibliothèque « Jeux vidéo — récents » (Stay-Puft) : même
API. Ajouts COMPATIBLES au détail `/jeu/{id}` :
- `versions[].fichiers[].role` : `jeu` (défaut), `liste_disques` (le .m3u d'un multi-disques), `maj`, `dlc`,
  `licence` (.rap PS3, servi avec son DLC), `archive_a_extraire` (ZIP64 « stocké », sans compression).
- `versions[].installation = {forme, extraire_dans, lancer, arguments}` — forme : `fichier` (défaut : tout se pose
  dans le dossier de jeux choisi), `archive` (extraire dans `extraire_dans`, relatif au dossier choisi),
  `installeur` (lancer l'installeur d'origine), `contenus_emulateur` (poser le jeu de base puis installer
  maj/dlc/licence DANS l'émulateur : Eden pour la Switch, « Install Packages/Raps » de RPCS3) ; `lancer` = le
  fichier à ouvrir.
- MULTI-DISQUES : un jeu en plusieurs disques est UNE version. ⚠️ Son fichier n=0 est le .m3u : ne JAMAIS supposer
  que n=0 est le disque 1 — lancer `installation.lancer` (le .m3u ; tous les disques dans le même dossier, le .m3u
  ne contient que leurs noms nus). Une version réunie porte `disques: [{disque, telechargement_id}]` et N'A PAS
  d'empreinte de version (`sha1`/`crc32` vides).
- `/reconcilier` : un disque d'une version réunie rend le `telechargement_id` de SA VERSION + `disque: N` (un groupe
  disque 1 + disque 2 est « exact ») ; `/fichier/{id}/{version}/{n}` accepte aussi l'identifiant d'un disque ;
  3ᵉ exception à « les telechargement_id des versions sont gardés » (le disque 1 garde le sien et devient celui de
  la version).
- Relevé réel n° 1 (20/10, Firehouse 2.94.1, Xak PC-8801) : `.m3u` (role `liste_disques`) + 4 `.zip` TOSEC (role
  `archive_a_extraire`, un .d88 dans chacun) ; le .m3u liste les noms INTERNES (.d88) — corrigé à la demande de
  Frogtend (doc libretro : QUASI88 ne lit ni .zip ni un .zip nommé par un .m3u). Forme `fichier` tant qu'il y a un
  `liste_disques` ; `archive` seulement pour un `archive_a_extraire` SANS liste. Rangé :
  `src-tauri/fixtures/simule/version-multidisques-123.json`.
- Côté Frogtend (0.85.1) : `installation.lancer` suivi s'il est l'un des fichiers reçus (forme vide ou `fichier`),
  sinon le choix d'avant (.m3u d'abord) ; chaque `archive_a_extraire` décompressée À CÔTÉ d'elle (sans sous-dossier,
  doublons entre archives refusés, jamais par-dessus un fichier existant, place vérifiée, taille annoncée jamais
  dépassée) ; les zips sont GARDÉS (réinstallation, fichiers reçus complets) ; les fichiers posés retenus
  (`Installation.extraits`), relevés au manifeste (une partie écrite dans un disque part à l'abri) et effacés au
  retrait ; tout .m3u lancé est vérifié avant (chaque disque présent, chemin sûr, ≤ 64 Ko). Rôles `maj`/`dlc`/
  `licence` et formes `archive`/`installeur`/`contenus_emulateur` : pas encore lus (pas servis).
- Aujourd'hui SEUL `liste_disques` est posé ; `maj`/`dlc`/`licence`/`archive_a_extraire` et les formes
  archive/installeur/contenus_emulateur arriveront avec les lots Switch, PS3 et PC (contrat fixé d'avance).

### Note 2.58.0 (contrat inchangé)
Un jeu peut SORTIR du catalogue (absent de `ids_visibles`) parce que sa fiche a été RÉUNIE à celle du même jeu
(« Probotector 2 » → « Super C ») : ses versions sont sur l'autre fiche ; les données locales liées à l'ancien id ne
sont pas redirigées (une indication « fusionné dans N » serait un ajout 1.14 — à demander à Seb si gênant).

### 1.55 (25/11, en service ; reçu complet le 26/11) — 🎵 musiques de Mario Kart 8 Deluxe : `amorces` et BGM.bars fabriqué
Reçu par message de la session Firehouse (26/11), texte du BRIEF § 1.55 + exemple réel mesuré le 26/11.
⚠️ À ne PAS proposer à la famille avant l'essai réel avec Seb sur Venkman.

- Chaque mod de `/switch/{tid}/mods` porte **`amorces`** : la liste (chaînes) des entrées du `BGM.bars` qu'il remplace ;
  `[]` = mod sans musique (TOUS les mods portent le champ ; 45 sur 363 pour MK8D). Un mod relève du mécanisme quand
  `amorces` est NON VIDE. Les amorces sont gardées par Firehouse, jamais dans l'archive : un BGM.bars complet livré par
  un mod (fait pour une autre version du jeu) est converti en amorces puis retiré de l'archive (sa `revision` monte).
- **`GET|HEAD /switch/{tid}/bgm-bars?mods=<cle>,<cle>`** rend le BGM.bars de la VERSION SERVIE avec ces amorces,
  fabriqué et relu par Firehouse. Il se pose dans **`romfs/Audio/Static/BGM.bars`**. UN SEUL BGM.bars pour TOUS les
  mods de musique actifs du jeu. Réponse réelle (26/11, `mods=gamebanana-ee3c0683c3f6`, DK Summit) : 200,
  `application/octet-stream`, `Content-Disposition: attachment; filename="BGM.bars"`, 41 018 496 octets, `ETag` = sha256
  entre guillemets, `X-Sha256` (à vérifier après réception), `X-Version-Jeu: 1179648`, `X-Amorces:
  pBGM_GC_SHERBET_LAND_F,pBGM_GC_SHERBET_LAND_N`, `Cache-Control: private, no-cache`, `Range` accepté (206,
  `Accept-Ranges: bytes`). `HEAD` : mêmes en-têtes sans corps (comparer `X-Sha256` avec le fichier posé). `mods=` vide →
  l'ORIGINAL de la version servie : inutile à poser. Accès : même garde que `/switch/{tid}/mods`.
- Erreurs (corps JSON) : 409 `{ok: false, code: "conflit", detail, entrees: {"pBGM_…": [cle1, cle2]}}` (deux mods actifs
  veulent la même amorce : la personne choisit lequel garder, rien n'est posé) ; 409 `{code: "original_absent"}`
  (original de la version servie pas encore extrait : réessayer plus tard, ne rien poser, GARDER l'existant) ; 404
  `{code: "introuvable"}` (clé inconnue, retirée ou réservée).
- Un mod de musique qui s'ÉTEINT : refabriquer avec les AUTRES mods de musique encore actifs ; s'il n'en reste AUCUN,
  RETIRER le BGM.bars posé (le jeu reprend son original).
- Quand refabriquer : l'ensemble des mods de musique actifs change ; la `revision` d'un d'eux monte ;
  `version_servie.version` change (`X-Version-Jeu` du fichier posé ≠ version servie). Le plus simple : un `HEAD` avec la
  liste active, et retélécharger si `X-Sha256` diffère du fichier posé.
- Exemple réel de `version_servie` (MK8D, 26/11) : `{version: 1179648, version_affichee: "3.0.1", build_id:
  "9EF5CAA2D5B933C772358C5AA6FABA15"}` ; DK Summit : `{cle: "gamebanana-ee3c0683c3f6", nom: "DK Summit", revision: 2,
  amorces: ["pBGM_GC_SHERBET_LAND_F", "pBGM_GC_SHERBET_LAND_N"], …}`.
- Où le poser côté Frogtend : à proposer à Firehouse (un dossier de mod à part, ex. « Firehouse — musiques », ne
  contenant que `romfs/Audio/Static/BGM.bars`, à côté des dossiers des mods).

### 1.54 (25/11, Firehouse 3.338.0, EN SERVICE, complet) — le relais NEXUS MODS par Firehouse (besoin 61)
**Lot N3 EN SERVICE (Firehouse 3.338.0, 25/11), en-tête X-Api-Jeux-Version: 1.54** : compatibilite, renseigne. Essai réel de Firehouse sur Cyberpunk 2.50 (sortie 22/10/2026, 2.500.100, date tenue avec sa source) : CET 107 → compatible true, certitude haute, « le fichier cite la version 2.5 du jeu » (fichier 168615), derniere_compatible {168615, 1.38.1} ; ArchiveXL 4198 → compatible null (seule la page cite la 2.5 ; prérequis RED4ext 2380 pas jugé compatible), requis [{RED4ext, 2380, version_min null}]. version_jeu_reconnue « 2.5 » : « 2.50 » et « 2.5 » = même version. Aucun LLM ; renseigne relit Nexus sans cache (fait / impossible).
**Lot N1 EN SERVICE (Firehouse 3.336.0, 25/11)** : `etat`, `jeux`, `mod` (`?prerequis=1`), `fichiers`, `md5`, `maj` ; PAS ENCORE : `obtenir` + `archive` (N2), `compatibilite` + `renseigne` (N3) ; en-tête encore `X-Api-Jeux-Version: 1.53` jusqu'à la fin de N3. Essais réels de Firehouse : Premium reconnu ; quota 20 000/jour et 2 000/heure ; `cyberpunk2077` = game_id 3333 ; CET (107) v1.38.1 : 130 fichiers, `sizeInBytes` pour tous ; prérequis GraphQL OK (CET : aucun) ; refus de clé pas encore vu.
**Lot N2 EN SERVICE (Firehouse 3.337.0, 25/11)** : `obtenir` + `archive`. Essai réel de Firehouse (17:18, CET 107, fichier 168615 « CET 1.38.1 - Game version 2.5 … .zip ») : 34 868 963 octets = sizeInBytes, téléchargé chez Firehouse en 11 s ; réponse {etat: pret, progression: 1.0, recus = total = 34868963, fichier: {url: /api/jeux/v1/nexus/fichier/cyberpunk2077-107-168615/archive, taille_octets, sha256 2e52be4e…, md5 1f1eb9b2…}, remplace_par: null} ; archive avec Range, ETag = sha256, X-Sha256, X-Complet: 1, Content-Disposition au nom d origine. Reste N3 (compatibilite, renseigne), en-tête 1.54 à sa livraison.
Reçu par message de la session Firehouse (25/11, deux messages « texte complet » + six précisions tranchées). Rien
n'est codé chez Firehouse au 25/11 ; exemples réels à venir (Cyberpunk 2.50 : CET 107, RED4ext, ArchiveXL 4198).

**Décisions de Seb (25/11)** : une seule clé Nexus Premium (celle de Seb), au coffre de Firehouse, JAMAIS servie à un
PC (Seb assume le risque vis-à-vis des règles de Nexus) ; API officielle seule ; ouvert à TOUS les grades ; un mod
`contains_adult_content` réservé aux admins ; **Firehouse télécharge, GARDE et SERT les archives** (le `download_link`
signé ne sort jamais de Firehouse) ; lectures de nuit permises.

**Règles communes** : préfixe `/api/jeux/v1/nexus/`, `X-Api-Jeux-Version: 1.54`, jeton d'appareil ; `{jeu}` = domaine
Nexus (`cyberpunk2077`, `[a-z0-9]+`) ; jamais un chemin du PC envoyé ; tailles en OCTETS (`sizeInBytes`, `null` si
inconnue) ; dates ISO 8601, heure de Paris, TOUJOURS avec le décalage (`2026-11-25T16:00:00+01:00`).
Erreurs, corps `{"ok": false, "code", "detail"}` (+ `reprise_heure`, `reprise_jour` pour les 429 ; + `plafond:
"lot"|"taille"`, `valeur` pour le 413) :
- 400 `jeu_invalide` / `corps_invalide` ; 404 `introuvable` (aussi un mod adulte pour un non-admin) ;
- 413 `trop` (`valeur` : 300 mods, 500 md5, ou la taille maximale en octets) ;
- 429 `nexus_quota` (+ `Retry-After`) et 429 `plafond_maison` : jamais rejoués avant `Retry-After` ;
- 502 `nexus_cle_refusee`, 502 `premium_requis`, 503 `nexus_coupe` : JAMAIS rejoués en boucle (nouveau geste de la
  personne, ou `GET /nexus/etat` qui redit `actif: true`) ;
- 504 `nexus_indisponible` : rejoué au plus une fois toutes les 5 min ;
- 409 `archive_changee` : relire `obtenir`.
Mod adulte demandé par un non-admin = mod inconnu partout : 404 sur `mod`, `fichiers`, `obtenir`, `archive` ; `[]`
dans `md5` ; absent de `maj` ; dans `compatibilite`, TOUJOURS dans `resultats` avec `connu: false`, `compatible: null`
(et son `mod_id` dans `inconnus`), exactement comme un mod absent de Nexus.

**Routes** :
1. `GET /nexus/etat` → `{ok, actif, premium, quota: {jour_restant, heure_restant, reprise_jour, reprise_heure},
   plafonds: {lot_mods: 300, md5: 500, obtenir_par_heure: 60}}` (n'appelle jamais Nexus).
1 bis. `GET /nexus/jeux?titre=|media_id=|gog_id=|steam_appid=` → `{jeux: [{domaine, nom, game_id, certitude:
   haute|moyenne}]}` ; plusieurs résultats possibles : Frogtend ne choisit jamais seul sans certitude haute.
2. `GET /nexus/{jeu}/mod/{mod_id}` → `{mod_id, nom, version, maj_le, disponible, statut, adulte, url, prerequis:
   {nexus: [{mod_id, nom, url, notes}], dlc: [{nom, notes}], externes: [{nom, url, notes}]}}` (cache 1 h / 24 h).
3. `GET /nexus/{jeu}/mod/{mod_id}/fichiers` → `{fichiers: [{file_id, nom, nom_fichier, version, categorie:
   main|update|optional|old|misc|archived, taille_octets, depose_le, principal, garde}], remplacements: [{ancien,
   nouveau, le}]}` (`garde` : Firehouse a déjà le fichier).
4. `POST /nexus/{jeu}/md5` `{md5: [...]}` (≤ 500) → `{resultats: {"<md5>": [{mod_id, file_id, nom_fichier,
   taille_octets}], "<inconnu>": []}}` ; plusieurs résultats : départager par la taille.
5. `POST /nexus/{jeu}/mod/{mod_id}/fichier/{file_id}/obtenir` (corps facultatif `{version_jeu: {affichee}}`), APRÈS
   le oui de la personne, taille annoncée d'abord → 200 `{etat: "pret"|"en_cours", progression, recus, total,
   fichier: {url, taille_octets, sha256, md5}, remplace_par: {file_id, version, raison} | null}` ; un refus est
   TOUJOURS une erreur HTTP, jamais un 200 ; idempotent (un seul téléchargement chez Firehouse, un 2e PC voit la même
   progression) ; `en_cours` porte `Retry-After` (10 s) : rappeler la même route ; le fichier servi est TOUJOURS
   celui demandé (`remplace_par` est une proposition). `fichier.url` est TOUJOURS un chemin RELATIF sous
   `/api/jeux/v1/nexus/fichier/` : toute autre forme est REFUSÉE par Frogtend.
   `GET|HEAD /nexus/fichier/{cle}/archive` : 200/206 (`Range`), `Content-Length` exact, `ETag` = sha256, `X-Sha256`,
   `X-Complet: 1`, `Content-Disposition` (attachment, `filename*` UTF-8 = nom d'origine Nexus) ; 409
   `archive_changee` ; archive servie telle que Nexus l'a donnée ; aucune limite pour servir, plafond de
   téléchargement réglable chez Firehouse (50 Go par défaut, au-delà 413 `trop` `plafond: "taille"`) ; gardée sans
   limite de durée pour l'instant.
6. `GET /nexus/{jeu}/maj?periode=1d|1w|1m` → `{mods: [{mod_id, fichier_maj_le, activite_le}]}`.
7. `POST /nexus/{jeu}/compatibilite` `{version_jeu: {affichee?, exe?}, mods: [{mod_id, file_id?, md5?}]}` (≤ 300 ;
   sans file_id ni md5 : le fichier principal le plus récent) → `{version_jeu_reconnue: "2.50" | null, resultats:
   [{mod_id, file_id, connu, compatible: true|false|null, raison, certitude: haute|moyenne|basse, versions_jeu: [...] |
   "toutes" | null, requis: [{nom, mod_id, version_min: null}], remplace_par: {file_id, version} | null,
   derniere_compatible: {file_id, version} | null, sources: [{type: nexus_fichier|nexus_changelog|nexus_description|
   admin|llm, url, extrait, lu_le}]}], inconnus: [mod_id]}` ; chaque mod demandé a TOUJOURS sa ligne ;
   `compatible: null` = INCONNU, jamais « compatible ». Règles (de la plus sûre) : verdict admin verrouillé ;
   `file_updates` → `remplace_par` ; true/haute = mention explicite de la version + déposé après sa sortie ;
   false/haute = vise explicitement une version plus ancienne et un fichier plus récent vise celle-ci ; false/moyenne
   = fondation accrochée à l'exe (CET, RED4ext, ArchiveXL, TweakXL, Codeware — liste réglable) déposée avant la sortie
   sans mention ; jamais plus compatible que ses prérequis ; le reste `null` (une date seule ne prouve jamais true).
   `version_min` vaut `null` aujourd'hui.
8. `POST /nexus/{jeu}/renseigne` `{mod_id, file_id, version_jeu: {affichee}, question}` → `{demande_id, etat:
   "en_attente"}` ; `GET /nexus/renseigne/{id}` → `{demande_id, etat: en_attente|en_cours|fait|impossible, mod_id,
   file_id, version_jeu, cree_le, fini_le | null, motif | null}` ; `fait` = relire la route 7 ; `impossible` =
   `motif`, la route 7 reste à null ; visible du seul demandeur et des admins (404 sinon).

**Version du jeu** : `affichee` (« 2.50 ») et/ou `exe` (version de fichier), au moins un (400 sinon) ; `exe` seule
traduite par une table tenue par un admin, sinon `version_jeu_reconnue: null` et seules les règles indépendantes de la
version s'appliquent. Pas de buildId GOG.

**Côté Frogtend (lu par Firehouse)** : false + haute → grisé avec la raison ; false + moyenne → avertissement
contournable ; null → « compatibilité inconnue » ; un successeur est PROPOSÉ, jamais choisi seul ; taille annoncée
avant `obtenir` ; `en_cours` relu au rythme du `Retry-After` ; sha256 vérifié avant l'installation.

**À vérifier par Firehouse avec la clé** : forme d'un refus de clé, limites réelles du Premium, `sizeInBytes` réel,
`game_id` de Cyberpunk, mods Nexus de RED4ext et ArchiveXL, date de sortie de la 2.50.

### 1.50 (22/11, Firehouse 3.306.0, en service depuis 16:56) — repères en texte libre dans les chemins de parties
Reçu par message de la session Firehouse (22/11, après le cas Age of Empires IV de Seb) ; détail : son BRIEF § journal « 1.50 ».
1. Ramenés aux jetons fermés : « (UserID) », « [user id] », « <user-id> », « [User] », « (uid) », « (ProfileID) »… →
   `<identifiant utilisateur>` (→ `<identifiant Steam>` pour un compte Steam / un jeu relevé sur Steam) ; « (SteamID) »,
   « [SteamID64] »… → `<identifiant Steam>` ; « [username] » → `%USERNAME%`. Age of Empires IV (1466860) :
   `%USERPROFILE%\Documents\My Games\Age of Empires IV\Users\<identifiant Steam>\Savegames` (le dossier réel chez Seb porte
   le SteamID64, 17 chiffres — pas l'identifiant court de `userdata`).
2. Un chemin qui garde un morceau ILLISIBLE (entièrement entre () [] {}, ou un `<…>` / `%MOT%` hors liste fermée) est RETIRÉ
   de la réponse. Un vrai nom qui CONTIENT des parenthèses ou un « % » reste (« Program Files (x86) », « Dead Space (2023) »,
   « 200% Mixed Juice! »). Coquille « <winLocalAppData>Low » corrigée en `%USERPROFILE%\AppData\LocalLow`.
3. Mesure sur l'index : 235 jeux avaient des repères ; après 1.50, 10 chemins ramenés, 37 retirés (29 jeux), plus aucun
   jeton hors liste fermée. ⚠ Un jeton peut être À L'INTÉRIEUR d'un nom (« Steam_<identifiant Steam> »,
   « <identifiant Steam>.sav ») : le remplacer dans le nom.

### 1.49 (22/11, Firehouse 3.305.0, en service depuis 15:48) — chemins justes par système, dossiers génériques écartés
Reçu par message de la session Firehouse (22/11) ; détail : son BRIEF-AGENT-FROGTEND.md § journal « 1.49 ».
1. Le SYSTÈME d'abord : un chemin Linux ou Mac (y compris sans `os` mais visiblement Linux/Mac : `Library/Application
   Support`, `.config/`, `.local/share`, `$XDG_…`) est servi en « Linux » / « Mac » avec `~/…`, jamais sous « Steam ».
2. « Steam », « Microsoft Store »… SEULEMENT pour un chemin propre à la boutique (`<Steam>`, `<racine de la boutique>`,
   `<identifiant Steam>` / `<identifiant utilisateur>`, `\Packages\<PFN>\`) ; tout autre chemin Windows → `os: "Windows"`
   (valable pour toute boutique) avec `releve_sur` (la boutique du relevé : information seulement, ne pas filtrer dessus) ;
   `boutique` non nul seulement pour un chemin propre à une boutique. Vérifié par Firehouse : TerraScape → 3 `Windows`
   (`%USERPROFILE%\AppData\LocalLow\Bitfall Studios\TerraScape\…`, `releve_sur: "steam"`, `boutique: null`), 3 `Linux`, 3 `Mac`.
3. `dossier` : liste FERMÉE `dossiers_ignores` et `dossier_longueur_min` (4) dans `/sauvegardes/references` ; un dossier dont
   le jeu reconnu ne partage aucun mot significatif avec le `titre` envoyé est écarté ; écarté → on passe au titre,
   `reconnu_par` le dit, `emplacements_source.dossier_ecarte` donne la raison (« nom de dossier générique », « ne
   correspond pas au titre »). Toujours envoyer le `titre` avec le `dossier` ; jamais `dossier` pour Xbox.
4. `<racine de la boutique>` = le `<root>` de Ludusavi : le dossier QUI CONTIENT le dossier du jeu, pour les boutiques autres
   que Steam ; pour Steam, jamais écrit (`<Steam>` = dossier d'installation de Steam, `<dossier du jeu>` =
   `…\steamapps\common\<jeu>`).

### 1.48 (22/11, Firehouse 3.302.0, en service depuis 14:48) — les EMPLACEMENTS DE PARTIES de tous les jeux PC (besoin 56)
Reçu par message de la session Firehouse (22/11) ; forme complète : son BRIEF-AGENT-FROGTEND.md § journal « 1.48 », réponses
réelles : son EXEMPLES-API-JEUX-V1.md § « 1.48 » (pas encore recopiées ici : à relever par un essai en lecture seule).
1. `GET /triche/recherche` accepte en plus de `steam_appid` / `titre` : `gog_id`, `xbox_pfn`, `dossier` (le NOM du dossier
   d'installation, sans chemin), `moteur`. `epic_appname`, `uplay_id`, `amazon_id` sont acceptés mais AUCUNE source ne les
   indexe → envoyer TOUJOURS `dossier` (c'est lui qui reconnaît un jeu Epic / Ubisoft / Amazon / EA). Ordre de
   reconnaissance : steam_appid → gog_id → xbox_pfn → dossier → titre exact UNIQUE ; un dossier ou un titre partagé par
   plusieurs jeux ne reconnaît rien. Nouveau bloc `emplacements_source` `{source: ludusavi | pcgamingwiki | null,
   reconnu_par, nom, source_url, lu_le, indisponible}` ; **`indisponible: true` = INCONNU, jamais « rien à sauvegarder »**.
   Source : un index LOCAL du manifeste Ludusavi (53 567 jeux, relu chaque jour à 05:45, accord de Seb) — les recherches
   de sauvegardes ne partent plus chez PCGamingWiki (les mods y vont encore, freinés côté Firehouse : 1 requête / 3 s pour
   toute la maison, 15 min de pause après un refus).
2. `GET /api/jeux/v1/sauvegardes/references` sert les listes FERMÉES : `os` (11 valeurs, et à quel jeu chacune s'applique :
   prendre `Windows` + celles de SA boutique), `jetons` (avec leur sens), `natures`, `moteurs`, l'ordre de `reconnaissance`.
   Les libellés PCGamingWiki sont ramenés à la liste (`os_inconnu: true` sinon : à ignorer). Chaque entrée de
   `sauvegardes` gagne `boutique` et `etiquettes`.
3. `moteur` + `emplacements_moteur` `{moteur, moteur_nom, sauvegardes, nom_a_lire, source_url, certitude: officielle |
   secondaire, note}` ; 13 moteurs avec les fichiers qui les reconnaissent (règles SteamDB). `/jeu/{id}/triche` donne le
   moteur du préfixe F95 quand il est connu (« RPGM » → `null` : à Frogtend de distinguer MV / MZ / VX Ace par les
   fichiers).
4. Grade : la recherche et les références sont identiques pour tous ; `/jeu/{id}/triche` suit la fiche (404).
5. Registre : `nature: "registre"` (nouvelle valeur), jamais un fichier — à IGNORER pour l'instant (plus tard : `reg export`).
6. Documents de sauvegarde : liste fermée + motif `manifeste-<clé du jeu>.json` (minuscules, chiffres, `. _ -`, 80 car.
   au plus, 5 Mo chacun) ; `/moi ▸ sauvegarde` gagne `documents_motif` et `document_max_octets` ; `GET /sauvegarde` donne
   `manifestes_jeux` par profil / PC.
Corrigé au passage côté Firehouse : la forme longue `{{Path|…}}` de PCGamingWiki effaçait la racine du chemin.

### 1.47 (18/11, Firehouse 3.242.0, en service) — les NOMS FRANÇAIS des DLC et MAJ Switch (besoin 54)
Reçu par message de la session Firehouse ; détail : son BRIEF § « 1.47 » et EXEMPLES (réponse réelle Fire Emblem).
`GET /switch/{tid_base}/contenus` gagne :
- `connus` : TOUS les DLC et MAJ connus du jeu, servis ou non (titledb, eShop français) : `{tid, type: dlc|maj, nom_fr
  (officiel ; null s'il n'existe pas), nom (langue d'origine), version (MAJ : brute ; DLC : 0), version_affichee (MAJ lue
  par Firehouse), publiee_le (MAJ)}`. Une MAJ : `nom_fr` « Mise à jour 1.2.0 » (version affichée connue) sinon « Mise à
  jour (version N) ». `connus` VIDE si une fiche du jeu est cachée à ce jeton.
- `nom_fr` et `titre` (nom d'origine) sur chaque contenu SERVI de `majs[]` et `dlc[]`.
Réel, Fire Emblem Three Houses (010055d009f78000) : 9001 « Histoire additionnelle - Ombres embrasées » (Side Story:
Cindered Shadows) ; 9002 « Nouvelles tenues » ; 9003 « Nouvelles escarmouches » ; 9004 « Quêtes additionnelles » ; 9005
« Nouveaux objets » ; 9006 « Uniforme de l'Académie des officiers (Byleth) » ; MAJ servie 010055d009f78800 v327680 :
« Mise à jour 1.2.0 ». Couverture : 31 018 titres, 29 200 avec un nom français ; titledb relu tous les 30 jours.

### 1.46 (18/11, Firehouse 3.239.0, en service) — la fiche de chaque mod Switch, écrite par l'IA
Reçu par message de la session Firehouse (18/11) ; forme complète : son BRIEF-AGENT-FROGTEND.md § « 1.46 ». Par mod :
`description` (français ; priorité admin > IA > source ; sans texte, l'IA le dit), `type` technique | contenu |
cosmetique | triche | autre, `interet` 1 à 5 (5 = technique utile à tous ; 1-2 = un skin parmi des centaines),
`public` tous | ado | adulte — un mod « adulte » n'est servi qu'à un jeton ADMIN (absent de la liste, archive 404 pour
les autres). Tous null tant que la fiche n'est pas écrite (jusqu'à 3 000 par nuit, ~14 000 mods). Décision de Seb : un
mod TECHNIQUE noté 4 ou 5 est validé d'office la nuit ; retiré aussitôt s'il s'avère incompatible.
⚠ Côté Frogtend : les mods rangés sont communs à la maison (dossier du jeu) → un mod « adulte » téléchargé par un admin
ne doit JAMAIS être montré ni posé pour un autre profil (brief § 2). Règle retenue : « réservé » couvre tout public
autre que « tous » ou « ado » EXPLICITES — `null` (fiche pas encore écrite, mod rangé avant la 1.46), valeur inconnue,
« adulte », et tout mod déjà vu « adulte » une fois (même reclassé depuis). Un mod rangé réservé n'est montré et posé
que si la dernière liste de CE profil (fraîche, ou son cache hors ligne) le contient — et, pour un mod « adulte » ou
déjà vu « adulte », seulement si elle le contient AVEC `public: "adulte"` (une entrée `null` ou « tous » d'un cache
plus ancien ne suffit pas). Sinon il est écarté (jamais effacé), jamais nommé au journal ni dans ce qui est commun ;
dans Eden (commun à la maison) il porte un nom de dossier neutre « Mod <clé> » et il est COUPÉ pour les autres profils ;
s'il ne peut pas l'être, la partie ne se lance pas.

### 1.45 (18/11, Firehouse 3.238.0, en service) — chaque mod Switch porte son PLAN D'INSTALLATION (à EXÉCUTER)
Reçu par message de la session Firehouse (18/11) ; forme complète dans son BRIEF-AGENT-FROGTEND.md § « 1.45 ». Consigne
de Seb citée par Firehouse (18/11, à confirmer avec Seb côté Frogtend) : « permettre à ceux qui n'y connaissent rien de
bénéficier des mods, des triches, des améliorations comme ReShade, sans avoir à rien configurer. Frogtend fait tout tout
seul et comprend les demandes en langage simple […] On ne veut pas que l'IA fournisse des instructions simples (sauf en
dernier recours) mais que l'IA FASSE l'installation de tout. »
Ajouts à chaque mod de `GET /switch/{tid}/mods` :
- `instructions {texte, source}` : texte d'origine (page GameBanana, README) — pour l'IA si un cas sort du plan ;
- `requis [{nom, lien, cle}]` : prérequis ; `cle` non nulle = Firehouse sert ce prérequis (installer SON archive d'abord) ;
- `variante {nom, version, description}` : la variante GameBanana servie ;
- `plan` (null tant que l'IA de Firehouse ne l'a pas écrit) : `actions[]`, dans l'ordre, liste FERMÉE :
  `installer_prerequis {nom, lien, type: mod|outil|firmware|emulateur, cle}` ; `poser_mod` (toujours une fois) ;
  `regler_emulateur {emulateur: ryubing|eden|tous, reglage, valeur, raison}` (texte libre : à traduire en clé de
  configuration) ; `desactiver_mod {nom, cle, raison}` ; `a_la_main {texte, raison}` (dernier recours, la SEULE chose à
  montrer) ; plus `resume_fr`, `mots_cles`, `ignores [{nom, raison}]` (ex. Atmosphère, inutile en émulation),
  `emulateurs_testes`, `touche_sauvegardes`, `certitude haute|moyenne|basse`, `a_la_main` (nombre), `modele`, `le`.
- Nouvelle source GameBanana (`gamebanana-…`, licence propre à chaque mod) ; .bnp (BCML) et mods NSFW jamais servis.
Exemple RÉEL BotW, mod « 1080p » (wollnashorn-9027f4987dc8) : compatible true ; fichier 366 o, sha256 206f648f… ;
`requis []` ; `variante null` ; `plan = {"resume_fr": "Le jeu s'affiche en 1080p au lieu de 900p quand la console est
dans son dock, l'image est donc plus nette.", "mots_cles": ["1080p", "plus net", "meilleure résolution", "image plus
claire", "docked"], "actions": [{"action": "poser_mod"}], "ignores": [{"nom": "Anisotropic Filtering Fix", "raison":
"mod séparé, non inclus ici"}, {"nom": "Disable FXAA", "raison": "mod séparé, seulement recommandé avec le 1080p, non
inclus ici"}], "emulateurs_testes": [], "touche_sauvegardes": null, "certitude": "haute", "a_la_main": 0, "modele":
"deepseek-direct/deepseek-flash", "le": "2026-11-18T12:00:53"}`. Servis le 18/11 pour BotW : 1080p, Anisotropic Fix,
Disable FXAA, ZeldaHighRes-1.6.0.
⚠ Côté Frogtend (0.208.1) : rien de 1.45 n'est encore lu ; la forme 1.44 continue de marcher (champs ignorés). Un
`regler_emulateur` en texte libre ne sera JAMAIS appliqué à l'aveugle : traduction vers une liste fermée de réglages
connus de Frogtend, après l'accord de la personne, configuration sauvegardée avant (brief § 4).

### 1.44 (18/11, Firehouse 3.234.0, en service) — les MODS SWITCH servis par Firehouse (besoin 53)
Reçu par message de la session Firehouse (18/11) ; forme complète dans son `docs/frogtend/BRIEF-AGENT-FROGTEND.md`
§ « 1.44 ». Exemple RÉEL BotW à venir, quand Seb aura validé les mods à l'écran (la validation vaut accord de
téléchargement chez Firehouse). Décisions de Seb : un admin valide chaque mod UNE fois ; ensuite servi à TOUS (mineurs
compris), mises à jour suivies sans nouvelle validation.
1. `GET /api/jeux/v1/switch/{tid_base}/mods` → `{ok, tid_base, version_servie: {version, version_affichee, build_id} |
   null, mods: [...]}`. Seuls les mods VALIDÉS, pour tout membre. 400 tid mauvais ; 404 si une fiche de ce jeu est
   cachée pour ce jeton. Par mod :
   - identité : `cle` (stable, « <source>-<12 hex> »), `nom`, `revision` (monte quand le contenu change),
     `description`, `auteur` ;
   - origine : `source` (page épinglée au commit), `licence` (« aucune » possible), `valide_le` ;
   - `versions` : `[{build_id, version_brute, version_affichee}]` OU la chaîne `"toutes"` (romfs seul) ;
     `version_brute`/`version_affichee` remplies quand la signature est celle de la MAJ servie ; `versions_annoncees` =
     ce que dit la source ;
   - `compatible` : `true | false | null` + `raison` — false = GRISÉ ; null = inconnu, ne PAS griser ;
   - `emulateurs_essayes []`, `reglage_requis ""`, `conflits [clés]`, `touche_sauvegardes true|false|null`,
     `contenu {exefs, romfs, cheats}` ;
   - `fichier {url, taille, sha256}`.
2. `GET|HEAD /api/jeux/v1/switch/mods/{cle}/archive` : ZIP NON compressé, racine = UN dossier de mod (`exefs/`,
   `romfs/`, `cheats/`), `Range` → 206, `ETag` = sha256.
3. Compatibilité jugée par Firehouse sur les signatures RÉELLES lues par NSTool dans la MAJ servie (`ModuleId` de main,
   rtld, sdk, subsdk*) ; `build_id` rendus en 32 hex quand la fin n'est que des zéros.
Relevé réel annoncé, BotW : `version_servie = {version: 786432, version_affichee: "1.6.0", build_id:
"8E9978D50BDD20B4C8395A106C27FFDE"}`. Mods trouvés, EN ATTENTE de validation : Wollnashorn (1080p, Anisotropic Fix,
Disable FXAA), masagrator (ZeldaHighRes-1.6.0, ZeldaBOTW_TripleBuffer), yuzu-mod-archive (Clean UI). PAS de 60 i/s
pour BotW 1.6.0 (liens theboy181 morts ; jeu accéléré à 60 i/s sans correctif d'après ses issues).
`emulateurs_essayes` vide tant que personne n'a essayé : Firehouse attend nos retours d'essai (Ryubing, Eden).
⚠ Côté Frogtend : la compatibilité est jugée sur la MAJ SERVIE ; la MAJ réellement utilisée sur le PC peut différer
(interrupteur éteint, MAJ locale plus récente) → Frogtend juge aussi d'après la MAJ qu'il pose.

### 1.43 (17/11, Firehouse 3.226.0, en service) — un fichier REFUSÉ n'est jamais renvoyé (besoin 52)
Reçu par message de la session Firehouse (17/11), recopié tel quel :
🚫 1.43 (17/11) — besoin 52 : un fichier REFUSÉ n'est jamais renvoyé (ajout compatible ; feu vert de Seb) — `POST /switch/envois` peut désormais répondre `{"ok": true, "deja_la": false, "refuse": true, "motif": "…"}` (HTTP 200) : CE fichier (même sha256, quel que soit l'envoyeur) a déjà été refusé pour une raison qui tient au fichier lui-même — ticket personnel, contenu ou jeu différent de l'annonce, plusieurs méta-contenus, conteneur illisible. Rien n'est créé, rien n'est à envoyer : dis le `motif` à l'utilisateur. Une raison passagère (abandon après 7 jours, empreinte d'un transfert raté, problème de clés chez Firehouse) ne compte pas : le POST crée alors un nouvel envoi, comme avant. Ordre des réponses du POST : `deja_la: true` d'abord, puis `refuse: true`, puis l'envoi.
Réponse RÉELLE (base d'essai) : `{"ok": true, "deja_la": false, "refuse": true, "motif": "ticket personnel (lié à une console ou un compte) : jamais servi à la famille"}`. `version` de l'API = "1.43".

### 1.42 (16/11, Firehouse 3.217.0, en service) — mises à jour et DLC Switch : ce que Firehouse a, et l'ENVOI (besoin 51)
Reçu par message de la session Firehouse (détail : son `docs/frogtend/BRIEF-AGENT-FROGTEND.md` § « 1.42 », formes dans
son EXEMPLES-API-JEUX-V1.md). Ajout compatible. Ouvert à tous les membres (décision de Seb). Jamais de clés, de
sauvegardes ni de jeux de base par ici.
1. `GET /api/jeux/v1/switch/{tid_base}/contenus` (title id du JEU DE BASE, 16 hex) → `{tid_base, jeu (media_id|null),
   majs[], dlc[]}` ; chaque contenu : `tid, version` (la VRAIE, lue dans le méta-contenu avec les clés), `taille, sha256,
   sha1, format (nsp|nsz), nom, fichier` (chemin à télécharger, `Range`). Une seule MAJ (la plus récente). Ticket
   personnel jamais servi ; fiche cachée pour le jeton → rien. Marche aussi sans fiche. Au 16/11 : 11 MAJ, 10 DLC.
2. Avant d'envoyer : TOUJOURS consulter la route 1 ; même MAJ (ou plus récente) ou même DLC déjà là → ne rien envoyer.
3. `POST /api/jeux/v1/switch/envois {tid, role (maj|dlc), version, nom (.nsp|.nsz), taille, sha256 (fichier entier),
   media_id?}` → `{deja_la: true, motif}` ou `{envoi, recu, taille, etat: "en_cours"}`. `media_id` facultatif si la route 1
   donne `jeu` (sinon 404 « media_id nécessaire »). Même POST (même sha256) = reprise. 400 annonce fausse / jeu de base /
   fiche pas Switch ; 404 fiche inconnue ou cachée ; 507 plafonds (32 Go par fichier, 100 Go d'envois en cours par personne).
4. `PUT /api/jeux/v1/switch/envois/{id}` : morceaux de 8 Mo AU PLUS, `Content-Range: bytes début-fin/total` ; 409 `{recu}`
   = reprendre à `recu` ; `HEAD` → `X-Recu`, `X-Etat` ; le dernier morceau rend `etat: "verification"`.
5. `GET /api/jeux/v1/switch/envois/{id}` → `{etat: en_cours | verification | range | refuse | deja_la, motif, recu,
   taille}` ; interroger toutes les ~10 s pendant « verification » (sha256, conteneur lu avec les clés : title id et rôle
   conformes, ticket commun, version supérieure, jeu de base de la fiche). « range » = servi à tous aussitôt (route 1 et
   `fichiers[]` de la fiche, rôle maj/dlc). Envoi sans nouvelles 7 jours = abandonné.
Aucun essai réel d'envoi fait par Firehouse : le premier sera celui de Frogtend (à lui rapporter).
