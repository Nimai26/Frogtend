# Contrat de l'API jeux v1 (Firehouse 2.16.1, `X-Api-Jeux-Version` 1.3)

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
