# Frogtend — le suivi unique

> Méthode : [METHODE-DE-TRAVAIL.md](METHODE-DE-TRAVAIL.md) § 1. Ce fichier porte TOUT ce qui est en cours, attendu ou
> noté. Il se relit EN ENTIER avant de répondre « qu'est-ce qui reste ? ». Une décision de Seb s'écrit ici et ne se
> redemande jamais. Créé le 04/10/2026 en reprenant `PLAN.md`, `A-TESTER.md`, `BESOINS-API.md` et `IDEES.md` (qui
> gardent le détail ; ce fichier est la liste qui fait foi).

## 🔨 En cours / à faire (sans attendre Seb)

- **Contrat 1.23 EN SERVICE** (20/10 22 h 29, Firehouse 2.94.0 ; détail dans CONTRAT § 1.23) : rôles des fichiers,
  `installation {forme, extraire_dans, lancer, arguments}`, MULTI-DISQUES = une version dont le fichier n=0 est le
  .m3u (lancer `installation.lancer`, jamais supposer n=0 = disque 1). ✅ Vérifié le 20/10 : Frogtend choisit déjà
  le .m3u d'abord (`partie.rs` `fichier_principal` : .m3u, .cue, .gdi, .chd, .iso), jamais « le fichier n=0 » ; les
  multi-disques servis aujourd'hui se lancent donc bien. À coder avec un relevé réel de chaque forme (demandé à
  Firehouse) : lire `installation.lancer`/`forme`, les rôles `maj`/`dlc`/`licence` (Switch : Eden ; PS3 : RPCS3).
  Relevé réel n° 1 (20/10, Xak PC-8801) rangé : `src-tauri/fixtures/simule/version-multidisques-123.json` — un .m3u
  qui liste des .ZIP. ⚠️ La doc libretro de QUASI88 n'accepte que .d88/.u88/.m3u (pas .zip), et un nom écrit dans un
  .m3u est ouvert par le cœur, pas décompressé par RetroArch : ce jeu ne se lancerait pas. Répondu à Firehouse
  (20/10) : garder les zips TOSEC avec le rôle `archive_a_extraire` (= décompresser sur place), servir le .m3u
  « après extraction » (noms .d88 lus dans les zips, jamais réécrit par Frogtend), Frogtend vérifie chaque ligne du
  .m3u avant de lancer ; zips dans un .m3u seulement si le cœur conseillé déclare .zip. ✅ Forme corrigée reçue
  (Firehouse 2.94.1) et lue en 0.85.1. Firehouse (20/10) a contrôlé ses 387 versions CPC et PC-8801 : aucun
  sous-dossier, aucun nom en double entre archives, tous les .m3u en UTF-8 ; il garde ses zips, et à sa prochaine
  livraison ne servira plus en `archive_a_extraire` une forme que Frogtend refuserait. Relevé réel Dreamcast
  (Shenmue II, .m3u de 4 .chd, rôle `jeu`) : déjà lu par 0.85.1. ✅ Firehouse garantit des .m3u en LF, UTF-8, sans
  BOM (410 contrôlés). À faire : Dolphin « Change Discs Automatically » (`AutoDiscChange` dans `Dolphin.ini`
  [Core]) posé par Frogtend pour les 4 multi-disques GameCube. À venir de Firehouse : forme `archive` (jeux de fans,
  MUGEN, DataXL : ZIP64 stocké, `extraire_dans`, `lancer` vide si ambigu) ; Switch `maj`/`dlc` en
  `contenus_emulateur` (Eden ≥ 0.2.0 : « External Content » suffit). ✅ Firehouse 2.95.0 EN SERVICE (21/10, contrat
  1.23 inchangé) : jeu PC en archive (UN ZIP64 stocké `archive_a_extraire`, forme `archive`, `extraire_dans` =
  dossier d'origine, `lancer` = exe relatif en « / », sous-dossiers PERMIS et dossiers vides présents, `lancer`
  vide = deux exe plausibles → extraire puis faire choisir) ; Switch `maj`/`dlc` (.nsp d'origine, seule la maj la
  plus récente) en `contenus_emulateur` ; Sega CD .chd par disque, multi-disques en .m3u LF ; PS3 en attente d'une
  décision de Seb (ISO Redump chiffrés). ⚠️ Frogtend 0.85.1 REFUSE un sous-dossier dans `archive_a_extraire` (prévu
  pour les multi-disques) : la forme `archive` demande son propre chemin d'installation (dans `Jeu/` ou
  `extraire_dans`) — à coder avec le relevé réel d'un jeu de fans. Reste : rôles `maj`/`dlc`/`licence`, formes archive/installeur/
  contenus_emulateur (prochain relevé : Dreamcast multi-disques).

- **Contrat 1.25** (21/10, Firehouse 2.98.0, besoins 30-32 SERVIS ; détail au BRIEF de Firehouse § 1.25) : (30)
  `en_ligne {online, anticheat, anticheats[], source, lu_le}` dans `/jeu/{id}/triche` ET `/triche/recherche` —
  `online` = texte de PCGamingWiki en minuscules (« true », « false », « limited », « hackable »… ; inconnu = en
  ligne/inconnu), `null` = INCONNU ; Frogtend 0.86.0 le lit déjà ainsi (tout sauf « false » = en ligne) ; à faire :
  lire `anticheats` ; (31) `GET /jeu/steam/{appid}` → `{media_id, launchbox_id}` (404 sinon, rapide) — pour le lot D
  (fiches des jeux Steam) ; (32) `sauvegardes [{os, chemin, steam_userdata, nuage}]` et `configuration [{os,
  chemin}]` (chemins NON résolus : %USERPROFILE%… <dossier du jeu>, <Steam>, <identifiant Steam>, HKEY_… ; clé
  `null` = pas encore lu, `[]` = rien) — pour compléter les abris des jeux Steam (lot B2 : parties hors du nuage).

- **Contrat 1.24** (21/10, Firehouse 2.96.0) : `fichiers[].sha1` sur toutes les versions (27 045 empreintes).
  Relevés réels rangés : `version-segacd-124.json` (Dracula Unleashed, .m3u de 2 .chd : déjà lu par 0.85.1) et
  `version-archive-pc-124.json` (Megaman's Christmas Carol 3, forme `archive`, UN ZIP64 stocké, `extraire_dans`,
  `lancer` = exe). ⚠ À CODER après le lot C en cours : sa `qualite` vaut « Jeu — ROM » (cosmétique selon
  Firehouse) — Frogtend 0.89.0 la prendrait pour une ROM (méthode « aucune ») et ne l'extrairait pas : la forme
  `archive` doit FAIRE FOI (extraire dans `Jeu/<extraire_dans>`, sous-dossiers permis, lanceur = `lancer`, vide = la
  personne choisit) ; puis vérifier chaque fichier téléchargé par son `sha1` (contrat 1.24). ✅ (0.91.0) forme
  archive. PS3 reçu (21/10, God of War III : .iso DÉCHIFFRÉ role jeu, 12 .pkg role dlc au nom aléatoire, 6
  `<content id>.rap` role licence, `contenus_emulateur`) — relevé COMPLET rangé `version-ps3-contenus-124.json`
  (19 fichiers). Réponses de Firehouse (21/10) : les .pkg ONT leur extension (le message tronquait) ; un .rap arrive
  aujourd'hui toujours avec son .pkg (24/24) mais le contrat ne le promet pas → poser chaque .rap dans `exdata` sous
  son nom, indépendamment ; apparier .rap ↔ .pkg par le content ID de l'en-tête du .pkg (octet 0x30), jamais par le
  nom du .pkg (aléatoire).
  ✅ Fait en 0.98.0 : les rôles `dlc`/`maj`/`licence` d'une version Firehouse PS3 installés par RPCS3 après le oui.
  Firehouse (21/10) : aucun .pkg PS3 de mise à jour dans sa collection (tout est `dlc` + `licence`) ; s'il en vient,
  ils seront en `maj` et Firehouse dira comment ordonner les versions (ne jamais réinstaller une vieille mise à jour
  par-dessus une récente) — À FAIRE alors côté Frogtend. `steam_userdata` = même chose que la position du jeton.

- **Jeux PC des boutiques — vrais jeux du PC** (Seb, 20/10 : « gérer l'installation, le lancement, les succès, les
  mods… »). État au 20/10 : Frogtend importe la liste (Steam par l'API, Galaxy pour GOG + Epic/Xbox/Ubisoft/EA/Amazon
  reliés) et OUVRE le jeu dans sa boutique, rien de plus : pas de dossier d'installation lu, pas de suivi de partie
  (ni session Firehouse, ni menu en jeu, ni manette→clavier), pas de mods/triches/abris, pas dans la grille Taodbox,
  pas de fiche. ✅ Seb 20/10 : **Galaxy reste la porte d'entrée** (Frogtend guide la liaison des comptes ; connexions
  directes plus tard si besoin) ; ordre **A** (dossier d'installation, fin d'installation, lancement par la boutique
  puis partie SUIVIE : session, temps, menu en jeu, manette→clavier, grille Taodbox) → **B** (mods, triches, Cheat
  Engine, abris ; `steam_appid` pour la recherche ; jamais avec un anti-triche) → **C** (succès en détail) → **D**
  (fiche par la base LaunchBox de Firehouse).
  Avis de l'expert lancement (20/10, relevés réels en lecture seule) : inscrire ces jeux au REGISTRE du PC (sinon il
  faut tout recopier) avec : `JeuPc.dossier` = `<données>outiques\<id>` (médias : pas de pieuvre), `Installation.
  dossier` = le dossier de la boutique, JAMAIS écrit ni effacé ; marqueur `boutique` en `#[serde(default)]` (pas une
  nouvelle `Methode`) et `est_de_boutique` (revoir chaque `est_importe`) ; lancement TOUJOURS par la boutique (Steam
  `steam://rungameid`, GOG `GalaxyClient.exe /command=runGame /gameId= /path=` — doc non officielle, à vérifier ;
  jamais l'exe direct : sauvegardes nuage) ; suivi PAR DOSSIER sans racine (attente « la boutique prépare le jeu »,
  relances absorbées, fin après K relevés vides ; session et temps depuis l'APPARITION du jeu) ; quitter / rendre la
  main / Cheat Engine par les processus du DOSSIER ; `dans_un_dossier` à corriger (séparateur, racine) ; figer non par
  défaut ; jeu lancé en administrateur : le dire. Découpage : **A1** lire (Steam `installdir` + `StateFlags`,
  bibliothèques dédoublonnées, registre par l'API ; Galaxy `installationPath`) → **A2** registre → **A3** suivi par
  dossier → **A4** lancer → **A5** interface + Taodbox → **A6** Epic par ses manifestes `.item` (Xbox, Ubisoft, EA,
  Amazon : plus tard). Relevés : `InstalledExternalProducts` n'a AUCUN chemin ; une PlayTask ne prouve pas
  l'installation ; Steam : registre `d:/launchbox/games/steam` et vdf `D:\LaunchBox\Games\Steam` (double lecture).
  ✅ Seb 21/10 : « ok lot b » — après A (Steam, 0.85.0), on passe au **lot B** (mods, triches, Cheat Engine, abris
  des jeux de boutique) ; GOG/Epic (A6) attendront. Décisions de Seb (21/10, ne plus les redemander) :
  (a) mods dans un dossier Steam : OUI avec garde-fous (originaux remplacés gardés dans les données de Frogtend,
  `<données>\boutiques\<id>`, rien d'autre chez Steam ; retrait propre ; après une mise à jour Steam, prévenir que le
  mod a pu être défait) — corriger la promesse de la note 0.85.0 ; (b) Cheat Engine : REFUSÉ si anti-triche connu
  (Firehouse `en_ligne.anticheat`, ou trouvé sur le disque) ; jeu en ligne sans anti-triche connu, ou inconnu :
  avertir « risque de bannissement » puis oui/non ; (c) parties : nuage Steam d'abord (`userdata\<compte>\<appid>`,
  lecture seule) + besoin Firehouse « emplacement des parties » ; (d) PAS de copie complète du jeu avant un mod.
  Relevé réel (21/10, lecture seule) : `/triche/recherche?steam_appid=` rend un bloc `en_ligne {online, anticheat,
  source: pcgamingwiki, lu_le}` NON documenté au contrat (GTA V : BattlEye, online "true" ; XCOM 2 : "limited" ;
  Skyrim SE : null) ; codes et tables CE vides pour ces jeux ; mods = conseils PCGamingWiki sans fichier.
  Avis des experts (21/10) : `steam_appid` jamais envoyé (`criteres_jeu_importe`) ; `jeu_copie_avant_mod` sans
  garde boutique ; `refuser_anti_triche` laisse passer l'inconnu ; Cheat Engine vise le 1er pid (le jeu relancé
  échappe) ; manifestes de mods DANS le dossier du jeu ; Nexus exige un lanceur. Découpage : **B0** garde-fous →
  **B1** triches + Cheat Engine (aucune écriture chez Steam) → **B2** abris nuage Steam → **B3** mods GitHub (données
  à part, `buildid` surveillé) → **B4** Nexus pour Steam.
  ✅ Seb 21/10 (après 0.97.0) : « PS3 » — installer les contenus additionnels (.pkg) et licences (.rap) des jeux
  PS3 servis par Firehouse (contrat 1.24, relevé God of War III `version-ps3-contenus-124.json`).
  ✅ Seb 21/10 (après 0.96.0) : « Parties Steam hors du nuage » — mettre aussi à l'abri les parties qu'un jeu
  Steam range HORS du nuage, d'après les emplacements que Firehouse donne (contrat 1.25 `sauvegardes`).
  ✅ Seb 21/10 (après 0.95.0) : « D » — les fiches des jeux de boutique (résumé, images) par Firehouse (contrat 1.25
  `/jeu/steam/{appid}`). Relevé réel (21/10, lecture seule, jeton de Seb) : les 11 jeux Steam installés de Venkman
  répondent TOUS 404 (pas dans le stock de Firehouse, probablement). Questions envoyées à Firehouse : la route ne
  sert-elle que le stock ? peut-elle servir la fiche de la BASE LaunchBox (hors stock : résumé, genres, jaquette,
  visuels, sans versions) — esprit du besoin 16 ? et pour GOG / Epic ? ✅ Seb 21/10 : plutôt que d'attendre
  Firehouse, prendre les fiches des jeux de boutique DANS LES BOUTIQUES : GOG Galaxy (base locale, lecture seule :
  `summary`, `meta` genres/développeurs/éditeurs/date, `media` captures, `originalImages` fond — en anglais) pour
  tous ses jeux, et pour un jeu Steam la fiche FRANÇAISE de la boutique Steam (`store.steampowered.com/api/
  appdetails?appids=<id>&l=french`, adresse publique, sans clé ; relevé réel XCOM 2 : description FR, genres,
  développeurs, date, 16 captures, fond `page_bg_raw`, images sur `shared.akamai.steamstatic.com`), gardée en cache.
  (LaunchBox, lui, prend ses fiches dans sa propre base LaunchBox Games DB.)
  ✅ Seb 21/10 (après 0.94.0) : pour Epic, il installe un PETIT jeu, TerraScape, dans
  `D:\LaunchBox\Games\Epic\Epic Games\Games` (Fortnite laissé tel quel) ; et « tout sera reset en fin de
  développement de Frogtend pour que tout soit bien géré par lui » : les installations actuelles (Steam, GOG,
  Epic, LaunchBox) sont des données d'ESSAI, Frogtend reprendra tout en main à la fin — ne rien optimiser pour leur
  état actuel, mais toujours en lecture seule tant que ce n'est pas lui qui les gère.
  ✅ Seb 21/10 (après 0.92.0) : « A6 » — GOG et Epic comme Steam (inscrits au registre, lancés par leur
  boutique, partie suivie) ; PS3 contenus, D et parties Steam hors nuage attendront. Décisions (21/10, ne plus les
  redemander) : jeux GOG lancés PAR GOG GALAXY (nuage, succès, temps de Galaxy ; commande `GalaxyClient.exe
  /command=runGame /gameId=<id> /path=<dossier>` NON documentée par GOG — forums GOG/LaunchBox/Lutris — à vérifier
  au 1er essai avec Seb) ; un jeu possédé sur Steam ET sur GOG = UNE SEULE ligne, et s'il est installé des deux
  côtés, Frogtend DEMANDE avant de lancer (« comme pour les autres jeux ») ; Epic : Seb signale Fortnite dans
  `D:\LaunchBox\Games\Epic\Epic Games\Games\Fortnite` (81 Go) — relevé (lecture seule, 21/10) : dossier présent
  (`.egstore` avec `Pending`) mais AUCUN manifeste `.item` de l'Epic Launcher pour lui (les 6 sont Unreal Engine et
  plugins) : Epic ne le considère pas comme installé ; à faire reconnaître dans Epic (Bibliothèque ▸ Fortnite ▸
  installer → choisir ce dossier, Epic le vérifie) avant le relevé et le code d'Epic. Relevés de l'expert : 2 jeux
  GOG installés (HZD, Cyberpunk), `goggame-<id>.info` (playTasks), registre `HKLM\…\GOG.com\Games\<id>` ; Galaxy
  regroupe les copies d'un même jeu dans `GamePieces` type `allGameReleases` (`{"releases":["steam_…","gog_…",
  "epic_…"…]}`) : la clé fiable pour « même jeu, une seule ligne » ; manifestes Epic à filtrer (`AppCategories`
  contient `games`, installation complète, `LaunchExecutable`, pas un DLC). Découpage : A6a GOG au registre +
  `synchroniser_boutique(source)` (retrait limité à SA source) → A6b lancement par Galaxy + suivi → A6c une ligne
  par jeu (allGameReleases) + choix de la copie avant de lancer + succès GOG au menu en jeu → A6d Epic.
  ✅ Seb 21/10 : « mettre des petites icônes sur les jeux représentant leur(s) magasin(s) d'origine » — sur chaque
  jeu (grille, panneau, Taodbox) : une petite icône par boutique où il est possédé (Steam, GOG, Epic…), plusieurs si
  la ligne réunit plusieurs copies ; à faire dans A6c (avec la ligne unique), icônes aux couleurs du thème (aucune
  couleur en dur), lisibles à la télé. Décisions A6c (Seb, 21/10, ne plus les redemander) : copies réunies
  SEULEMENT si Galaxy les relie (`allGameReleases`), jamais par le titre ; icônes d'origine pour TOUS les jeux
  (boutiques, Firehouse, « Disque » pour un jeu importé) ; temps de jeu d'un jeu réuni = la somme, avec le détail par
  boutique ; installé de plusieurs côtés : demander À CHAQUE FOIS (rien retenu). Avis de l'expert interface : table
  `copies(groupe, id)` remplie au rangement (toutes sources), carte maîtresse choisie à la lecture (steam, gog,
  epic, puis autres), autres masquées ; filtres sur toutes les copies ; registre : une ligne par copie (inchangé) ;
  `jouerLaCarte` commun (panneau, double-clic, Taodbox jeu/gérer) ; icônes en pastilles texte (pas de logos de
  marque) ; doublon possible en Taodbox : `galaxyInstalles` ne filtre pas `dossier`. Découpage : A6c-1 cœur → A6c-2
  jouerLaCarte → A6c-3 Taodbox → A6c-4 icônes.
  ✅ Seb 21/10 (après 0.89.0) : « lot C » — les succès en détail (B fini ; D et A6 attendront). Décisions (21/10,
  ne plus les redemander) : succès SECRETS cachés (« 🔒 Succès secret — la description apparaîtra quand tu l'auras
  obtenu ») avec « Montrer » à la demande ; un jeu Steam vu aussi par Galaxy : les succès de STEAM font foi (Galaxy
  pour les jeux GOG et les autres boutiques reliées) ; la liste au bureau (panneau) ET dans le menu en jeu ET en
  Taodbox ; l'alerte « succès obtenu » pendant la partie : plus tard (IDEES). Avis de l'expert interface (21/10) :
  aujourd'hui seulement « x / y » dans le panneau ; Steam déjà appelé avec `l=french` mais réduit à un compte ;
  Galaxy lisible en détail (tables `Achievements`, `UserAchievements`, `LocalizedAchievements` ; défauts : pas de
  filtre `userId`, total compté sur `UserAchievements`) ; découpage C1 cœur Steam (détail, schéma, rareté, cache
  PAR PROFIL pour le hors ligne) → C2 Galaxy → C3 RetroAchievements → C4 fenêtre du panneau → C5 menu en jeu et
  Taodbox. ✅ Seb 21/10 (après 0.91.0) : « ok pour C3 » — RetroAchievements en détail ; C1 (Steam) attend son compte.
  ✅ Seb 21/10 : les abris des parties Steam partent AUSSI chez Firehouse avec la sauvegarde du profil (comme les
  autres jeux) ; les remettre en place : plus tard (Steam Cloud peut écraser ce qu'on remet à la main).
  Relevé réel B2 (21/10, lecture seule) : un seul compte dans `userdata` ; `remotecache.vdf` liste chaque fichier
  (`root`, `size`, `sha` = SHA1 vérifié sur 50 fichiers, `syncstate`) ; 13 jeux sur 21 ont un `remote\` (root 0),
  d'autres n'ont QUE l'Auto-Cloud (XCOM 2 : root 2 = Documents, REDIRIGÉ vers OneDrive chez Seb — les 42 fichiers
  retrouvés par SHA1 dans `OneDrive\Documents`, aucun dans `%USERPROFILE%\Documents` ; root 3 = AppData\Local) ;
  root 1 = configurations de manette Steam (pas des parties). Les numéros de racine ne sont PAS documentés par Valve
  (seuls les noms `WinMyDocuments`… le sont) : Frogtend retrouve chaque fichier par taille + SHA1 parmi les
  dossiers de Windows (lus dans le registre « User Shell Folders »), jamais en devinant ; un fichier OneDrive « en
  ligne seulement » n'est jamais lu (il serait téléchargé).

- **Suites du lot Nexus (0.82.0 livré)** — à faire, dans l'ordre :
  - ✅ Seb 20/10 : « pas besoin de question » — les liens nxm:// restent pris d'office (installeur ou mise à jour,
    une seule fois), même si Vortex / Mod Organizer les avait. Ne plus le redemander.
  - Les 7z en PPMd (méthode [3,4,1], ex. sauvegardes de LaunchBox) : non pris en charge par sevenz-rust ; refus propre.
  - Refuser une mise à jour de Frogtend pendant une décompression (déjà vrai avant 0.82.1 ; l'enfant meurt avec
    Frogtend, la décompression repart de zéro).
  - Mods posés « tels quels » : un mod Nexus dont l'archive a un seul dossier racine (`NomDuMod/`, `Data/`) peut rester
    inerte ; montrer l'arborescence avant le oui, ou déduire l'endroit — à proposer à Seb.
  - Jeu PC sans `manifeste.json` (rattaché du disque) : `mettre_a_l_abri` ne copie rien. Le texte Nexus le dit ; le
    texte GitHub promet encore « parties à l'abri » → à corriger (ou localiser les sauvegardes connues).
  - Copie de sauvegarde du jeu avant un mod « proposée » (PLAN, lot 8, 02/10) : pas encore proposée (GitHub ni Nexus).
  - À relever à l'essai de Seb : vraie réponse de l'API Nexus (`size_kb`, `file_name`, `URI`, `md5_search`), vrai
    lien nxm (caractères de `key`), adresse de la page de la clé, statut rendu pour un lien d'un autre compte.

- **Lot mods (contrat 1.19)** — ✅ Seb 19/10 : « oui prends-le en compte, et il faudra bien sûr que cela soit fait à
  l'installation de Frogtend sur PC » : l'enregistrement de `nxm://` (registre de Windows, annoncé à Seb avant) se
  fait par l'INSTALLEUR de Frogtend, pas au premier usage. Frogtend télécharge les mods lui-même (GitHub : sha256
  vérifié ; Nexus : voie officielle nxm:// + clé d'API PERSONNELLE de chaque utilisateur, au coffre ; jamais
  d'automate sur les pages de Nexus).
  Découpage (19/10) : **0.81.0 = mods GitHub** (montrer licence / pré-version / dépôt déplacé avant le oui ; télécharger
  avec sha256 ; installer dans le dossier du jeu, ce qui est remplacé copié à l'abri ; désinstaller) + recherche des
  codes et mods d'un jeu IMPORTÉ (`/triche/recherche`) ; **0.82.0 = Nexus** (`nxm://` enregistré par l'installeur,
  clé d'API de chaque profil au coffre). Relevé réel (lecture seule) : Ultimate Spider-Man → « Fusion Fix », 1 fichier
  GitHub (7,1 Mo, sha256, MIT, `methode: extraire`, `cible: {base: jeu, dossier: ""}`, `a_confirmer: false`),
  `references` cite la release d'un autre jeu ; Skyrim : mods sans fichier (consignes seulement) ; copié dans
  `fixtures/simule/triche-ultimate-spider-man.json`.
  ✅ Seb 19/10 : un mod SANS empreinte fournie = permis, avec avertissement avant le oui et taille exacte exigée ;
  essai réel accepté (Fusion Fix, 7,1 Mo, dans un FAUX dossier de jeu, effacé après). Avis des 3 experts reçus
  (téléchargeur dédié sans jeton, HTTPS + domaines GitHub, plafond = taille annoncée ; installation : liste refusée au
  1er chemin dangereux, plafond de décompression, comparer au jeu, remplacés à l'abri dans `<jeu>\Frogtend\mods\`,
  pose par renommage, manifeste par mod, désinstallation seulement si le fichier n'a pas changé ; écran « 🧩 Mods » dans
  ⚙ Gérer le jeu). Découpage : 0.81.0 = mods GitHub ; 0.81.1 = recherche pour un jeu importé.
  Fait (0.81.0) : `mods.rs` (FichierMod depuis la réponse : seulement `extraire` + `jeu` + .zip ; adresse admise
  github.com/<dépôt>/releases/download/… ; client SANS jeton, https_only, redirections ≤5 vers *.githubusercontent.com ;
  taille annoncée = plafond exact ; sha256 si donné ; reprise Range ; erreurs `without_url` ; `lister` refuse l'archive
  au 1er chemin dangereux (.., absolu, « : », nom réservé, casse, lien) ; `installer` : décompression plafonnée à côté,
  comparaison, remplacés copiés et vérifiés dans `<jeu>\Frogtend\mods\<clé>\remplaces`, manifeste « en_cours » avant
  la pose, pose par renommage, défaite si interrompue, conflit avec un autre mod refusé ; `desinstaller` : seulement
  les fichiers inchangés, les modifiés gardés + copiés). Commandes `mods_installes`, `mod_installer` (mod et fichier
  REPRIS de la liste, anti-triche local, jeu non utilisé, parties à l'abri d'abord), `mod_desinstaller`. Interface :
  `src/lib/mods/mods.ts` (+ tests sur la vraie réponse), `src/lib/web.ts`, « 🧩 Mods » dans ⚙ Gérer le jeu (jeu PC),
  la page Triches renvoie vers lui. ✅ ESSAI RÉEL (accord de Seb) : vrai Fusion Fix téléchargé par le code de Frogtend
  (7 115 342 octets, sha256 conforme, redirection vers release-assets.githubusercontent.com), installé dans un FAUX
  jeu (2 ajoutés, dsound.dll remplacé et gardé), désinstallé (original revenu, scripts\ retiré). Essai effacé.
  Relectures (contrat, interface : aucun bloquant ; lancement : 2 BLOQUANTS corrigés) : (1) un original remplacé n'est
  plus jamais effacé tant qu'il n'a pas été remis (fichier modifié ou supprimé par la personne : sa copie reste) ;
  (2) une archive ne dépose plus rien sous `Frogtend\`, et un manifeste n'est cru que s'il est sain (clé = nom,
  chemins sûrs) — un faux manifeste ne fait rien effacer. Aussi : liens/jonctions sous la cible refusés ; empreinte de
  l'original au manifeste (désinstallation reprise) ; manifeste « en cours » AVANT de toucher au jeu, nettoyage auto
  avant, défaite après ; dossiers créés retirés ; reprise : nom = clé+taille+empreinte, jamais sans empreinte,
  `Content-Range` vérifié ; adresse = chemin EXACT de la release ; règle de redirection testée ; messages réseau en
  français ; jeu occupé dès la vérification ; refus pour un jeu émulé ; interface : attente qui reste affichée, retrait
  possible hors ligne, page Triches = même tri, descriptions coupées, « installation interrompue » montrée. Essai réel
  rejoué après corrections : OK. Tests 404 cœur, 206 interface.
- **0.81.1** (19/10) — Recherche des codes et mods d'un jeu IMPORTÉ (`/triche/recherche`, 25a) : `criteres_jeu_importe`
  (lu AVANT d'ouvrir la session), `triches_avec` dans `jeu_triches`, `triche_installer` (fichier par
  `fichier_triche_route`, `fichier` repris tel quel) et `mod_installer`. Relevé réel (lecture seule) : Sonic (8 codes),
  Super Mario World + `rom` (1 code, `?rom=`) ; copié dans `fixtures/simule/triche-recherche-sonic.json`. Steam : pas
  encore (les jeux Steam ne sont pas des jeux du PC de Frogtend ; à voir).
  Relecture de l'expert contrat (aucun bloquant) appliquée : 404 du fichier d'un code = « ce code n'est plus proposé »
  (aussi pour les jeux à fiche) ; route du fichier EXACTE `/triche/<clé [a-z0-9-]>/fichier[?rom=…]` (un nom No-Intro
  « ...Iru! » passe, `%2e%2e` non) ; relevé réel de Super Mario World (`?rom=`) testé et consigné (EXEMPLES) ; pas de
  `rom` quand le numéro PS3 suffit ou pour un EBOOT.BIN. Note : `est_importe` (id < 0) couvrirait aussi un jeu de
  boutique rangé dans le registre (aujourd'hui : recherche par titre et plateforme, sans fuite).
- **0.81.2** (19/10) — Contrat 1.20 (besoin 26 servi) : `description_fr` / `notes_fr` passés du cœur (`InfosPatch`) à
  l'écran (`nomDuPatch`, `remarqueDuPatch` : Firehouse d'abord, puis nos ~20 traductions, sinon l'anglais marqué) ;
  `description` anglaise d'origine seule écrite dans `patch_config.yml` (inchangé). Réponse simulée remplacée par le
  vrai relevé traduit du 19/10.

- **Lot 0.80.0 — MAME : les sauvegardes des jeux d'arcade par profil** (Seb 19/10 : « ok enchaîne » après la 0.79.0 ;
  relevé par l'expert lancement pendant le lot 0.79.0 : `nvram\` (meilleurs scores, réglages de la borne) et les
  autres données de MAME ne sont ni par profil ni dans la sauvegarde — règle « les sauvegardes sont sacrées »).
  Sources de MAME 0.289 lues (`emuopts.cpp/.h`, `plugins/hiscore/init.lua` + `plugin.json`) : `cfg_directory`,
  `nvram_directory`, `state_directory`, `diff_directory`, `homepath` (hiscore → `<homepath>/hiscore`, greffon
  désactivé d'office). Fait : `preparer_mame_profil` (options sur la LIGNE DE COMMANDE vers `Profils\<profil>\…`,
  rien d'écrit dans mame.ini ; option réglée par la personne = respectée et journalisée ; reprise COPIÉE la 1re fois,
  sans écraser, liens ignorés, plafond 256 Mo) branché dans `preparer_emulateur` (arcade et machines de salon). La
  sauvegarde/restauration du profil couvre déjà `Profils\<profil>\`. ✅ ESSAI RÉEL (MAME 0.289 officiel, profil
  « Zoé l'été ») : Pong lancé et quitté proprement → `cfg\pong.cfg` + `default.cfg` DANS le profil, aucun dossier
  commun créé. NON vérifié : l'écriture de la nvram en vrai (gridlee ne s'est pas fermé seul : avertissement de MAME
  sur un romset de 2022 ?) — à essayer par Seb.
  Relecture de l'expert lancement (aucun bloquant) appliquée : reprise marquée PAR OPTION une fois réussie
  (`.frogtend-reprise-<dossier>`) — un `sta` créé par « Sauver » (0.79.0) n'empêche plus de reprendre les états F7 ;
  trop gros ou copie interrompue = MAME garde SON dossier commun pour cette option (jamais un dossier vide), nouvel
  essai au lancement suivant ; `homepath` reprend aussi autofire, inputmacro, timer (sources 0.289 vérifiées) ;
  `ini\mame.ini` lu si la racine n'en a pas ; un nom de profil avec « % », « ; », « " » ou « autoboot » = dossiers
  communs, noté. Tests 394 cœur.
  Livraison arrêtée par un test rouge (`un_jeu_lance_par_un_intermediaire…`) : VRAI défaut trouvé et corrigé
  (lanceur fini avant le 1er relevé → heure « 0 » → vieux orphelins adoptés ; désormais l'instant du lancement −5 s),
  test ajouté, leçon écrite.

- **Lot 0.78.0 — Patchs PS3 par Firehouse** (contrat 1.18 reçu le 19/10 16 h 37 ; ✅ Seb 19/10 : « Patchs PS3
  d'abord », MAME juste après). Fiche d'un jeu PS3 : ses patchs (par numéro de série + version du jeu), activés d'un
  clic dans `patch_config.yml` (copie de sauvegarde avant) ; `patch.yml` tenu à jour depuis Firehouse (sha256 vérifié
  avant de remplacer) ; jamais `imported_patch.yml`.
  Avis des experts (contrat : `telecharger_controle`, `.part` nommé d'après le sha256, messages 404/503 propres à la
  route ; lancement : sources de RPCS3 lues — `config\patch_config.yml`, 5 niveaux, clé de version = APP_VER si citée
  sinon « All », version = mise à jour seulement si EBOOT.BIN + même jeu + GD ; plafond du PARAM.SFO). ✅ ESSAIS RÉELS
  (lecture seule) : formes des réponses relevées (EXEMPLES-API-JEUX-V1.md) ; Asura's Wrath = BLES01227 v01.00 → 8
  patchs ; sur une COPIE de RPCS3 : liste posée (1 197 547 octets, sha256 = annoncé), « Disable Motion Blur » écrit
  et relu. Nouvelle dépendance : yaml-rust2 0.13 (lecture du YAML de RPCS3, pur Rust). Besoin 26 (patchs en français).
  Relectures (lancement, contrat, interface ; aucun bloquant technique) appliquées : téléchargement PLAFONNÉ à la
  taille annoncée (`telecharger_plafonne`), `installer_liste` déplacée dans patchs_ps3.rs et testée avec l'état RÉEL ;
  restes nettoyés par nom exact (`patch-<16 hex>.yml.frogtend|.part`) ; désactiver sans réseau ou un patch retiré
  (depuis `actives`) ; écran hors ligne ; clés entier/booléen et imbrication excessive = refus ; `rpcs3_ferme`
  revérifié avant d'écrire ; refus journalisé ; 404/503 propres (`erreur_patchs`). Interface : « Retouches du jeu
  (patchs) », seules celles du jeu (`titreDuJeu` : titre de la fiche, sinon le plus fréquent, à égalité le plus
  court), remarque montrée AVANT d'activer, « Débloquer les images par seconde » sans promesse. Besoin 27 (même liste
  pour tous les jetons ?). contenus.rs gardé en CRLF. NON vérifié : un jeu lancé avec le patch (Seb) ; la version du
  moteur attendue par un RPCS3 ancien.
- **Lot 0.79.0 — MAME : sauver / charger / recommencer depuis le menu en jeu** (idée IDEES de l'expert lancement 16/10 ;
  pour les 592 jeux d'arcade). Avis de l'expert lancement reçu le 19/10 : script Lua + fichier « commande » écrit puis
  renommé ; `save`/`load`/`soft_reset` relancent MAME (commande AVANT de cacher le menu) ; état dans
  `<MAME>\Profils\<profil>\sta\<nom court>\frogtend.sta` (chemin absolu, par profil, couvert par la sauvegarde) écrit
  sous un autre nom puis échangé (MAME efface après une écriture ratée) ; témoin « vivant » plus tard (0.79.1).
  ✅ Seb 19/10 : essai réel avec MAME 0.289 officiel (GitHub mamedev, 87,6 Mo, dans `D:\Frogtend\.essai-mame`, effacé
  après) ; un jeu qui gère mal les sauvegardes d'état (`supports_save` faux) : Sauver/Charger CACHÉS, Recommencer reste.
  Fait : script Lua (commande numérotée lue puis effacée, `pcall`, `apres_commande` = jamais de nouvelle pause tant
  que le témoin du menu qui se ferme existe ; `frogtend\jeu` = nom court + supports_save) ; `commande_mame` (écrite
  puis renommée, verbes fermés, jamais deux lignes) ; `garder_etat_mame` (taille stable, sinon la bonne intacte) ;
  `action_mame` : commande AVANT de cacher le menu ; menu filtré par `jeu_mame`. ✅ ESSAI RÉEL (MAME 0.289 officiel,
  sha256 = celui publié par mamedev, dans `.essai-mame`) : Pong « sauve » ; Sauver ×2 → `Profils\Sébastien\sta\pong  frogtend.sta` 10 Ko (accents OK), précédente gardée ; Charger et Recommencer lus, MAME vivant ; Sauver après
  Recommencer OK (le script survit). NON vérifié : que Charger ramène visiblement au moment sauvé (à voir par Seb) ;
  le témoin « vivant » (0.79.1).
  Relecture de l'expert lancement (aucun bloquant) appliquée : un échec de sauvegarde ROUVRE le menu (jeu en pause,
  message visible) au lieu d'un message dans une fenêtre cachée ; la bonne sauvegarde reprend sa place si la nouvelle
  ne se pose pas ; la sauvegarde du profil saute `frogtend-nouveau.sta` et un fichier disparu (au lieu de s'arrêter) ;
  fichiers de la commande hors du fil asynchrone ; effacement du témoin réessayé ; `.essai-*/` ignoré par git ;
  commentaire sur la pause de la personne (MAME repart après une commande). ✅ 2e ESSAI RÉEL (chemin du menu) : menu
  ouvert → « pause » ; Sauver puis menu fermé → « reprise », jamais une nouvelle pause. MAME d'essai effacé.

- **Lot 0.77.0 — Manette → clavier, plus simple** (Seb 19/10 : « ok enchaîne » après la 0.76.0 ; lot pris dans IDEES,
  expert interface 16/10, déjà proposé à Seb dans le choix du lot précédent) : choisir une touche en l'APPUYANT au
  lieu d'une liste de 70 ; répétition automatique d'une touche tenue (les vieux menus ne défilent que par répétition).
  Avis des experts avant de coder (lancement : répétition dans `Moteur::pas`, `Frappe.repetition` qui ne compte pas,
  délai/cadence de Windows `SPI_GETKEYBOARDDELAY/SPEED`, seule la dernière touche, sans Maj/Ctrl/Alt, interrupteur
  par jeu ; interface : fenêtre maison « touche » en capture, lettres par `key`, chiffres et le reste par `code`,
  liste au canapé). Choix sans question à Seb, sur avis : répétition ACTIVE d'office, coupable jeu par jeu
  (`sansRepetition`). ✅ ESSAI RÉEL : réglages Windows de ce PC 500 ms / 33 ms ; Z tenu 1,5 s → 31 « z » (attendu 31).
  ✅ LIVRÉ en 0.77.0. Relectures : lancement (aucun bloquant ; commentaires précisés, test `sansRepetition`, note sur
  Échap/Entrée tenus) ; interface (BLOQUANT corrigé : M refusé en AZERTY — leçon ; focus sur la zone d'attente, jamais
  « Annuler » ; refus effacé, touche oubliée si la fenêtre perd la main ; textes). Tests 378 cœur, 199 interface.
  NON vérifié : F5, F12, Alt, F10 réels dans WebView2 pendant la capture (à essayer par Seb) ; la répétition dans un
  vrai jeu.

- **Lot 0.76.0 — « ⚙ Gérer le jeu » à la manette en Taodbox** (✅ Seb 19/10, choisi après le constat : AUCUN jeu
  ScummVM ni Xbox au catalogue, 12 717 jeux surtout des cartouches rétro — SNES 1 892, GBA 1 774, GBC 1 225, GB 1 175,
  Mega Drive 1 130… ; ScummVM et conversion XISO repoussés). Idée d'origine : IDEES (expert interface, 05/10).
  ✅ Seb 19/10 : bouton **Y** (clavier : touche Menu ≣) ; **Retirer** (et le désinstalleur) **reste au bureau**, caché
  en Taodbox. Plan de l'expert interface : mode canapé dans `gererJeu` (ni sélecteur de fichiers Windows, ni saisie,
  ni installeur guidé : « à faire depuis le bureau ») ; Y → `ContextMenu` hors de `Commande` (les lecteurs de
  livret/médias/images se ferment sur une commande inconnue) ; focus rendu à la carte après le menu.
  ✅ LIVRÉ en 0.76.0 : `src/lib/taodbox/canape.ts` (`auCanape` = attribut `data-taodbox`, posé seulement par la vraie
  Taodbox — pas `taodbox.actif`, vrai aussi dans le menu en jeu au bureau ; `seulementAuBureau`) ; filet dans les deux
  `parcourir` ; renvoi AVANT les questions (installé ailleurs, dossier des outils) ; dossier des émulateurs proposé
  d'office au canapé (comme « Prêt à jouer ») ; options cachées (guidée, Un autre programme/émulateur, à installer
  soi-même, Retirer, « Rangé dans ») ; `appuiY` ; page : `gerer` + verrou, A bloqué pendant, menu natif bloqué
  (`svelte:window oncontextmenu`), focus rendu. Relecture de l'expert interface appliquée. Vérifié : 189 tests
  interface (Y, preneur, fenêtre, canapé), svelte-check. NON vérifié : à la vraie manette (Seb), la touche ≣ réelle
  dans WebView2, Y par la manette virtuelle de Sunshine.

- **Lot OSD — le menu en jeu universel** (Seb, 30/09 puis 04/10). Mots de Seb (04/10) : « servira pour Frogtend et
  Taodbox » ; « manette ET clavier souris » ; « il doit être le MÊME pour tous, seul le fond peut être différent en
  fonction des jeux ou systèmes, et certaines options bien sûr ». Exigences atomiques :
  - [ ] a. Le menu s'ouvre à la manette par **Select + R1 tenus 1 s** (Back + RB sur une manette Xbox), pendant toute
        partie, quel que soit l'émulateur ; réglable dans les Options. ✅ Seb 04/10 : « Select + Start, certains jeux
        peuvent avoir cette combinaison ; il en faut une bien moins courante, par exemple Select + R1 » (remplace la
        décision du 30/09) ; le maintien d'1 s est gardé contre les ouvertures par erreur.
  - [ ] b. Il s'ouvre toujours au clavier (Pause/Attn, réglable) — ✅ existe depuis 0.11.0.
  - [x] c. ✅ 0.60.0 (à essayer avec une manette PlayStation/Switch). Lecture de la manette pendant la partie : XInput (manettes Xbox, manette virtuelle de Sunshine) ; SDL3
        compilé dans le programme pour les manettes PlayStation/Switch/8BitDo (✅ décidé par Seb le 30/09 : « SDL3
        embarqué, aucune DLL »). Lisibilité en arrière-plan à MESURER (sonde, voir 🧪).
  - [x] d. Un seul menu : retirer les combinaisons propres aux émulateurs posées en 0.8.0 (Select + Start de
        DuckStation/PCSX2, L3 + R3 de RetroArch). ✅ Seb 05/10 : « je ne sais pas d'où ils viennent mais le but est
        d'uniformiser : il faut pouvoir avoir le menu Frogtend PARTOUT avec View + RB » → TOUTE liaison manette du
        menu d'un émulateur part, même faite à la main (le clavier reste). ✅ 0.46.3.
  - [ ] d bis. Le menu MET LE JEU EN PAUSE et le jeu ne reçoit plus la manette tant qu'il est ouvert. Seb 05/10 :
        « le menu ne met pas en pause le jeu et les boutons sont encore saisis dans le jeu » (RPCS3). ✅ RPCS3 en
        0.46.3 (ses propres réglages : pause à la perte du premier plan, manette ignorée en arrière-plan). Reste : les
        autres émulateurs à l'essai, et les jeux PC (aucune pause possible sans eux : à étudier). Réglages RPCS3 par
        jeu (`custom_configs`) traités aussi.
  - [ ] d quater. **Jeux PC : « ❄ Figer le jeu pendant le menu »** — ✅ Seb 05/10 (« oui » à la proposition) : option
        PAR JEU, désactivée d'office ; refusée pour un jeu où Frogtend trouve un anti-triche (fichiers dans le dossier
        du jeu) ; risques dits avant d'activer (son qui grésille, jeu en ligne déconnecté). Par défaut, le menu s'ouvre
        par-dessus le jeu PC sans le figer. ✅ 0.47.0 (à essayer par Seb).
  - [x] d ter. ✅ Seb 05/10 : « d'accord » → accepté tel quel, on ne touche pas aux profils. RetroArch : le bouton Xbox (Guide) ouvre encore SON menu — imposé par ses profils de manette officiels
        (`input_menu_toggle_btn = "10"`), impossible à couper proprement par la configuration (input_driver.c : la
        liaison du profil passe quand celle du fichier est vide). Expert, 05/10 : accepter (pas de conflit avec
        View + RB) plutôt que bricoler les profils. À observer à l'essai (la Game Bar de Windows
        le capte peut-être avant).
  - [x] e. Navigation dans le menu à la manette (croix/stick, A valider, B retour), au clavier (flèches, Entrée,
        Échap) et à la souris (survol, clic). ✅ 0.45.0 (à essayer par Seb).
  - [x] f. Le même menu en Taodbox (échelle ×2, plein écran, contour de sélection fort). ✅ 0.45.0 (à essayer).
  - [x] g. Le fond : visuel du jeu (médias du contrat 1.6 : fond, capture, logo), sinon du système, sinon du skin.
        ✅ 0.48.0 : fond, sinon capture, sinon écran titre du jeu, sinon le skin. Le niveau « système » attend
        Firehouse (besoin 20). Question à garder pour l'essai : la fenêtre bureau (560×640) ne montre le visuel que
        sur une bande de 16 px ; l'agrandir (~760×640) si Seb veut le voir au bureau.
  - [x] h. Adaptateurs : ✅ RPCS3 (0.46.3), ✅ MAME (0.58.0), ✅ Eden, Azahar, Cemu, Xenia, xemu, Vita3K (0.59.0) — à essayer
        (documentation officielle de chacun d'abord : recherche lancée le 16/10).
        Recherche faite (code source de chacun, 16/10) : Eden et Azahar ont `pauseWhenInBackground` ([UI], deux lignes
        `\default=false` + valeur) ; Cemu, Xenia, xemu, Vita3K n'ont AUCUNE pause réglable. ✅ DÉCIDÉ par Seb le 16/10 :
        ces quatre-là sont FIGÉS d'office pendant le menu (« Oui, toujours » : vraie pause, manette sans effet ; ni
        anti-triche ni jeu en ligne ; risque dit : son qui grésille à la reprise).
  - [x] i. Options propres à certains émulateurs (affichage des performances…), dans le même menu. ✅ 0.61.0 pour
        RetroArch (FPS_TOGGLE, FAST_FORWARD, SCREENSHOT, MUTE — command.h de la 1.22.2). Reste : les autres émulateurs
        (raccourcis F13–F16 déjà pris ; d'autres touches à étudier dans leur documentation).
  - Relecture par les experts interface et lancement (04/10), qui ont relu le code (fichier:ligne) :
    - ✅ LEVÉ le 04/10 (sonde de Seb) : la manette est lue pendant qu'un jeu a le premier plan (XInput et SDL).
      Reste : le menu ouvert depuis la manette doit obtenir le premier plan (sans lui, pas de pause et la manette
      pilote encore le jeu) → vérifié par le code (GetForegroundWindow) et essayé avec Seb à l'étape 3.
    - Le cœur sera la seule source de la manette (XInput, puis SDL3) : il repère Select + R1 et envoie au menu
      haut/bas/valider/retour ; la Gamepad API du WebView2 n'est pas fiable dans une fenêtre sans focus.
    - Retirer Select + Start de DuckStation/PCSX2 demande une MIGRATION (valeur exacte écrite par Frogtend, le reste
      gardé, fichier mis à l'abri) ; RetroArch : retirer la ligne du fichier régénéré à chaque partie.
    - Pièges : boutons encore tenus à l'ouverture ; un seul gestionnaire des flèches ; menu à l'échelle de la télé en
      Taodbox (aujourd'hui 560×640 fixe) ; jeux PC sans pause (la manette agit aussi sur le jeu) ; plein écran
      exclusif (DOSBox, jeux PC) cache le menu ; deux parties en même temps.
  - Découpe : (1) sonde avec Seb ; (2) navigation commune clavier/souris/manette + échelle Taodbox ; (3) le cœur lit
    XInput, Select + R1 tenus 1 s, premier plan vérifié, migration des anciennes combinaisons ; (4) SDL3 ; (5) le
    fond ; (6) options, puis adaptateurs un par un (RPCS3, Cemu, Eden, Xenia, xemu, Azahar, Vita3K, PPSSPP).
- [ ] **Lot « Documents » (contrat 1.7)** — ✅ (D1) livret en 0.56.0 ; ✅ (D2) musique et vidéo en 0.57.0 (question
      DÉCIDÉ par Seb le 16/10 : « mieux de couper musique et vidéo pendant le jeu » → dans le menu en jeu, musique et
      vidéo seulement si le jeu est en pause (le livret toujours) ; arrêtées si le jeu repart ; lecture automatique à la manette seule dans le menu en jeu : non vérifiée). Reste : (D3) ; la liste des
      documents est gardée PAR PROFIL, mais le PDF d'un jeu du PC est dans son dossier (commun) : à revoir si Firehouse
      répond au besoin 21 que les documents dépendent du grade ; pas d'avancement ni d'annulation pendant le
      téléchargement du livret (assez pour 8 Mo, pas pour un gros scan chez la sœur) ; pas de cache des pages rendues
      (0,3 s par page, pas de préchargement) ; ouvrir d'emblée agrandi une double page dans la petite fenêtre du menu
      en jeu ? (à voir avec Seb au premier essai). Relu AVANT de coder par les experts contrat et interface (16/10). Découpe :
      (D1) le LIVRET : `/jeu/{id}/documents` lu à l'ouverture de la fiche (en ligne), liste gardée avec le jeu du PC ;
      PDF téléchargé au clic (la taille est sur le bouton : le clic vaut accord pour CE document), sha256 et type
      vérifiés avant de le garder (dossier du jeu s'il est sur le PC, sinon cache DU PROFIL) ; pages rendues en images
      par Windows (Windows.Data.Pdf, intégré) et montrées par un lecteur maison (←/→ page, ↑/↓ défiler, A zoom, B
      retour) sur la fiche et dans le menu en jeu — ni iframe PDF ni lecteur de Windows (pas pilotables à la manette,
      lecteur caché derrière le menu) ; protocole gardé par le profil et la liste du jeu. (D2) musique et vidéo :
      lecteur maison (⏯, ±10 s, barre, heure), protocole avec `Range` (206), verrou par document. (D3) catalogue :
      `documents` et `documents_empreinte` conservés (JeuResume) + relecture complète unique ; besoin 21 (est-ce que
      `maj_le` bouge avec les documents ?). `video_theme` : pour Taodbox, plus tard. Purge et plafond du cache du
      profil (visuels aussi) : à faire. Mots distincts à l'écran : « annexes » (Abandonware France) ≠ « livret ».
- [x] **Lot MAME (contrat 1.13)** ✅ 0.55.0 : fichiers dans leur sous-dossier (CHD chez le parent), version MAME =
      ROM (jamais décompressée), zip lancé (jamais la CHD), `mame <nom court> -rompath <dossier>`, « incomplet » dit,
      paquet officiel auto-extractible ouvert sans être exécuté. Reste : plusieurs versions (clones) d'un même jeu sur
      le PC (Frogtend n'en garde qu'une par fiche) ; ✅ MAME se met en pause au menu en jeu (0.58.0).
- [ ] **Lot « Jeux PC : plein écran et manette → clavier »** (ordre du 16/10). ✅ Seb 16/10 : « pour le plein écran
      Mega Man, attention, ce n'est qu'un jeu parmi d'autres, il faut que notre création soit le plus généraliste
      possible ». Exigences : (a) plein écran au lancement, PAR JEU, désactivé d'office, trois modes : rien / touche
      du jeu (F4, Alt+Entrée, F11… ; suggestion trouvée dans le ReadMe du jeu) / fenêtre agrandie sans bordure sur
      l'écran du jeu ; (b) manette → clavier PAR JEU : préréglages courants + bouton par bouton ; touches envoyées
      seulement quand le jeu a le premier plan, toutes relâchées au menu et à la fin ; la combinaison du menu reste
      au menu. Livraisons : (a) puis (b).
      ✅ (a) en 0.53.0. Expert lancement : 1re version BLOQUÉE (fenêtres hors du jeu visées ; touche qui rebascule un
      jeu déjà en plein écran) — refaite ; 2e relecture : 2 importants corrigés (état relu juste avant d'agir ; le
      choix de la personne jamais défait). ✅ (b) en 0.54.0 : « Jouer à la manette (jeu fait pour le clavier) ».
      Expert lancement : 1 BLOQUANT (premier plan vérifié toutes les 250 ms seulement → frappes possibles vers une
      autre fenêtre) corrigé (vérifié à chaque envoi) + 4 importants corrigés ; expert interface : ZQSD faux en QWERTY
      → touches « Déplacement » par POSITION. Idées notées : choisir une touche en l'appuyant, répétition auto.
- [ ] **Lot « Contrat 1.13 »** (analyse d'écart de l'expert contrat, 16/10 ; texte et exemples réels reçus de Firehouse
      le 16/10, dans CONTRAT-API-JEUX-V1.md et EXEMPLES-API-JEUX-V1.md). Frogtend n'utilise rien de la 1.7 à la 1.13.
      Écarts : (1.8) l'émulateur est choisi par `recommande` LaunchBox et sa LIGNE DE COMMANDE LaunchBox est reprise
      (pret.ts:25, pret.svelte.ts:42, assistant.svelte.ts:283) — la 1.8 l'interdit, le `conseil` doit primer ; choix
      forcés en dur DOSBox/Eden (assistant.svelte.ts:98-105) ; lignes figées dans pc.json sans suivre `maj_le`.
      (1.12) cœurs RetroArch pris chez libretro en nightly, sans empreinte (emulateurs.rs:587) au lieu des cœurs
      stables vérifiés de Firehouse. (1.13) Arcade : CHD posée à plat et lancée comme fichier du jeu (partie.rs:56),
      MAME lancé avec le chemin du zip au lieu de `mame <nom> -rompath` — CASSE un jeu Arcade à CHD (aucun chez Seb
      aujourd'hui). (2.58.0) un jeu installé dont la fiche a été réunie à une autre ne se lance plus (« pas sur ce
      PC ») ; besoin 1.14 « fusionné dans N » proposé à Firehouse. (1.7, 1.9–1.11) documents (livret PDF, musique,
      vidéo) et BIOS/firmwares servis : rien n'est cassé, mais c'est ce qui rend « Prêt à jouer » sans étape manuelle.
      Découpe : (1) conseil en lecture ; (2) conseil : ligne et cœur, origine retenue ; (3) cœurs servis + sha256 ;
      (4) BIOS servis (fichier/motif/archive, sha1, un seul accord) puis PS2 ; (5) firmwares PS3 et Switch (prod.keys
      au coffre : le dire) ; (6) MAME (dossier, nom court, -rompath, « incomplet », paquet 0.289) ; (7) documents ;
      (8) fiches réunies. `fichiers: null` d'un motif = `[]`.
      (8) ✅ Seb 18/10 : demander à Firehouse l'ajout 1.14 « fusionné dans N » (besoin 22, envoyé à la session
      Firehouse le 18/10) ; Firehouse (18/10) : reçu, traité après sa 2.71.0 ; un jeu caché reste en 404 ;
      probablement redirection sur `/jeu/{ancien}` ET liste `fusions` dans `/catalogue` ; `telechargement_id` à
      vérifier. Côté Frogtend : rattacher le jeu du PC au nouvel id quand ce sera servi.
      ✅ SERVI le 18/10 (contrat 1.14) et vérifié en réel (`/jeu/1927` → 410 `fusionne_dans: 324` ; `fusions` = 154 sur la
      page 1 du catalogue). Lot « 1.14 » commencé le 18/10 : inventaire de ce qui est rangé sous le numéro d'un jeu
      demandé aux experts avant de coder. ✅ LIVRÉ en 0.65.0 (rattachement automatique).
      ✅ (1)+(2) en 0.49.0 : le conseil prime (même absent de la liste : MS-DOS → RetroArch, liste = DOSBox Staging
      seul), ligne = options de Frogtend juste avant `%rom%`, `%rom%` remplacé au lancement, `maj_le` retenu dans
      pc.json ; ✅ 0.62.0 : la mise à jour est PROPOSÉE quand `maj_le` change (au lancement, une fois par console et
      par séance, 3 s au plus) ; « Préparer DOSBox » quand MS-DOS a été préparé avec RetroArch.
      ✅ (3) en 0.50.0 : cœurs RetroArch pris chez Firehouse (`/emulateurs/retroarch/coeurs`), reprise, empreintes du
      zip puis de la dll, pose seulement après ; un cœur non servi est refusé (rien chez libretro). Reste chez libretro
      (expert, 16/10) : RetroArch LUI-MÊME (buildbot stable, emulateurs.rs) → passer par `/emulateurs/retroarch/paquet`
      de Firehouse ; les profils de manette `autoconfig.zip` (buildbot, sans empreinte) → demander à Firehouse de les
      servir (besoin à écrire).
      ✅ (4) en 0.51.0 : BIOS servis posés (fichier/archive obligatoires, SHA-1 vérifié, jamais un fichier remplacé,
      copie sûre entre disques), clés et « à poser soi-même » dits, PCSX2.ini réglé sans défaire un choix. Reste :
      les motifs servis (`fichiers[]`, aucun au 16/10) ; une archive déjà en place est retéléchargée (mineur) ; PCSX2
      non portable (ini dans Documents) : écart commun à tous les modules PCSX2.
      ✅ (5) PS3 en 0.52.0 : micrologiciel servi (PUP 4.93) téléchargé, vérifié, installé sans fenêtre ; RPCS3 doit
      être fermé ; place vérifiée ; réussite = version annoncée dans la note relue après (RPCS3 rend 0 et plante en se
      fermant, ticket 16421). Reste : la Switch — firmware servi (1.10) NON branché : Eden refuse sans prod.keys ;
      à faire quand les clés seront posables (lire d'abord la doc d'Eden sur l'installation hors menu). Firehouse :
      demander un champ `version` dans le BIOS du conseil (aujourd'hui lue dans la `note`). Règles Firehouse (16/10) : seul `conseil.bios[]` fait foi ;
      ignorer `actif`/`statut_emu_france` pour l'émulateur du conseil ; `conseil.paquet: null` → « cœur non servi par
      Firehouse », rien chez libretro (question posée à Seb par Firehouse : laisser RetroArch l'installer ?).
- ✅ **Seb 18/10** : « tu peux modifier ce que tu veux » → CLAUDE.md § 1.7 précisé : Frogtend ne télécharge jamais un
  JEU sur Internet (les jeux ne viennent que de Firehouse) ; émulateurs, cœurs et outils : Firehouse s'il les sert,
  sinon leur source officielle, après accord et taille annoncée. Puis « on enchaîne » → lot Médias (choisi par moi).
- [ ] **Lot « Médias »** (en cours depuis le 18/10 ; découpe : M1 choix du visuel de la ludothèque au bon ratio +
      `images`/`medias_empreinte` et `documents`/`documents_empreinte` gardés dans le catalogue (fin du D3 des
      documents), relecture complète unique ; ✅ M2 galerie dans la fiche (0.64.0) ; ✅ M3 logos et fonds Taodbox (0.66.0) ; ✅ M4 (0.67.0) hors ligne et
      retrait du cache d'une image plus servie) (contrat 1.6, Firehouse 2.41.0) : ratio réel de la boîte réservé, choix du visuel de la
      ludothèque (jaquette, boîte 3D, cartouche…), logos et fonds pour Taodbox, galerie dans la fiche, copie hors ligne
      des visuels choisis, retrait du cache d'une image que Firehouse ne sert plus (édition par un admin).
- [ ] **RPCS3 : mods, shaders, triches** (Seb, 03/10 : « il manque encore la gestion des mods, shaders, cheats, l'OSD »)
      — l'OSD passe d'abord (Seb, 04/10). ✅ Seb 19/10 (lot commencé) : PATCHS (triches, 60 i/s, résolution) « par
      Firehouse » → besoin 24 envoyé (Firehouse n'a aucun jeu PS3 au stock ; les jeux PS3 de Seb sont importés :
      recherche par numéro de série) ; SHADERS : « les deux » (moins de saccades ET filtres d'image) ; MODS : « les
      installer » → besoin 25 envoyé. Ordre : shaders d'abord (ne dépend que de RPCS3), patchs et mods quand Firehouse
      les sert.
- [x] ✅ 0.68.2 **Formats des fichiers d'émulateurs** (expert, 04/10, après le bug YAML de RPCS3) : xemu `xemu.toml`
      (emulateurs_profils.rs:402) — une apostrophe dans le chemin (« Jeux d'émulation », profil « D'Aquin ») rend le
      TOML illisible → chaîne "…" échappée + test qui relit en TOML ; Qt/QSettings (Eden `external_content_dirs`
      contenus.rs:346, Azahar emulateurs_profils.rs:425) — une virgule ou un point-virgule dans un chemin en fait une
      liste → `valeur_qt` (guillemets) + relecture qui retire les guillemets + test « Zelda, TOTK ». Chez Seb
      aujourd'hui : aucun des deux cas (pas de xemu ; dossier DLC sans virgule).
- [x] ✅ 0.68.1 **Sauvegarde : abris des seuls jeux visibles** (expert contrat, 18/10, déjà vrai avant) : `contenu_sauvegarde`
      envoie les abris de TOUS les jeux du PC (`sauvegarde.rs` ~403-429), sans filtre de visibilité, alors que la liste
      `bibliotheque` est filtrée. Après un passage admin → mineur, des numéros et des parties de jeux cachés partiraient
      dans l'espace du mineur chez Firehouse. → ne garder que les abris des jeux `visible_ou_reuni` du profil.
- [x] ✅ 0.68.0 **Changer le jeton d'un profil** (expert contrat, 15/10, déjà vrai avant) : `changer_jeton` garde la
      ludothèque, les jaquettes et les visuels du profil ; si le nouveau jeton appartient à un AUTRE compte Firehouse
      (ex. un admin remplacé par un mineur), ce qui a été reçu pour l'ancien reste visible jusqu'à la synchronisation.
      → vider ces caches (ou forcer une synchronisation complète) quand le compte change. Règle du brief § 2.
- [x] ✅ 0.67.1 **Mode simulé et vrai profil** (expert contrat, 18/10, déjà vrai avant) : un profil passé en mode simulé
      (Réglages ▸ Firehouse) range les ~30 jeux d'exemple dans le MÊME `ludotheque.db` que le vrai catalogue (une
      synchronisation simulée le remplace) ; repassé au vrai Firehouse, une relecture le corrige, mais d'ici là la
      ludothèque est fausse. → cache à part pour le mode simulé. Le ménage des images (0.67.0) ne tourne jamais en simulé.
- [x] ✅ Durcissement TERMINÉ en 0.73.0 (Seb : « Poursuis » : étapes 4 à 7 ; restes mineurs dans IDEES). Historique :
      ✅ Seb 19/10 : « Les 3 premières étapes » (sur 7 proposées par l'expert lancement : module de garde, programmes
      lancés, effacements) → 0.71.0. Restent (à reproposer) : 4 choix de fichier ouverts par le cœur (`parcourir`,
      choix à usage unique, retrait de `dialog:allow-open`), `import_ajouter` et `emulateur_adopter` ; 5 écritures
      dans les dossiers d'émulateurs (`emulateur_installer(_firehouse)`, `bios_poser`, `retroarch_installer_coeur`,
      `emulateur_regler_manette`, `triche_installer` par clé) ; 6 écritures dans les emplacements (`import_copier`,
      `import_installer_dos`, `abris_regler`) ; 7 droits de la fenêtre `menu-jeu`, scan réseau annoncé.
      ✅ Étape 4 livrée en 0.72.0 (les deux brèches `emulateur_adopter` et `import_ajouter` sont fermées). Restent les
      étapes 5, 6, 7.
- [ ] **Durcissement** (relevé à l'audit du 03/10) : le cœur ne recompare pas aux réglages les dossiers que
      l'interface lui donne (emplacements, émulateurs, outils).
- [ ] DLC Xbox 360 (Xenia) et PS Vita (Vita3K) — aucun jeu chez Seb pour l'instant, priorité basse.
- [ ] Succès Epic (pas d'API publique, pas synchronisés par Galaxy) — à étudier.

> **Ordre des lots — ✅ Seb 16/10 : « je te propose de choisir toi-même l'ordre, je te suis ».** Ordre retenu :
> (1) Contrat 1.13 : conseil → cœurs servis → BIOS servis → firmwares PS3/Switch ; (2) Jeux PC : manette → clavier,
> plein écran au lancement ; (3) MAME, documents, fin du lot OSD (SDL3, options, adaptateurs).

## ❓ Attend une décision de Seb

- [ ] **Jeux PC sans manette ni plein écran** (Seb, 05/10, Mega Man Rock Force : « il s'ouvre en tout petit fenêtré et
      ne reconnaît pas la manette »). Relevé : jeu Game Maker 8, clavier seulement (ReadMe : Z saut, X tir, C glisse,
      Entrée pause, A/S armes ; F4 plein écran) ; la manette est vue par Windows (joystick 1, XInput 0). Proposé :
      (1) « manette → clavier » par jeu (le cœur lit déjà XInput : traduction en touches, correspondance d'office
      modifiable) ; (2) « plein écran au lancement » par jeu (touche du jeu, ou fenêtre agrandie sans bordure).
      ✅ Ordre décidé le 16/10 (voir en tête de cette section) : ce lot vient après le contrat 1.13. Le gel (0.47.0) : ✅ « cela
      semble fonctionner » (Seb, 05/10).

- [ ] Questions encore ouvertes du 02–04/10 : aucune bloquante. (La combinaison du menu à la manette était DÉJÀ
      décidée le 30/09 : ne pas la redemander — voir LECONS.md, 04/10.)
- [x] Méthode de travail : ✅ Seb 04/10 « tu as mon accord » → ligne en tête de `CLAUDE.md` et trois experts dans
      `.claude\agents\` (contrat, ui, lancement).
- [ ] Option : désinscrire de la lettre d'information de GOG après un jeu offert (Frogtend ne touche pas aux réglages
      de compte sans décision).

## 🧪 Tests à faire AVEC Seb

Le détail (quoi faire, ce qu'on doit voir) est dans [A-TESTER.md](A-TESTER.md) (128 cases à cocher). Les plus utiles
d'abord :
- [ ] **Prêt à jouer (0.44.0)** : une console sans émulateur (ex. Sega CD) se prépare après UN accord.
- [ ] **💿 Micrologiciel** : affiche « Installé : version 4.91 » ; un PUP 4.93 s'installe sans fenêtre.
- [x] ✅ **Sonde manette faite par Seb le 04/10** (Asura's Wrath au premier plan dans RPCS3) : la manette Xbox Series X
      est lue EN ARRIÈRE-PLAN par XInput ET par SDL — tous les boutons, gâchettes, croix, Select, R1, L3/R3. Le
      blocage de l'OSD à la manette est levé. Reste à mesurer : le premier plan pris par le menu ouvert depuis la
      manette (vérifié par le code avec GetForegroundWindow), une manette PlayStation.
- [x] ✅ **Seb 05/10 : « ça a fonctionné » (0.44.4)** — **RPCS3 + manette** : Seb 04/10, deux fois « la manette ne fonctionne pas » : (1) réglage jamais appelé
      au lancement (0.44.2) ; (2) manette sans fil en veille au clic → Frogtend donnait le clavier. Corrigé : manette
      branchée, sinon la dernière vue, sinon Xbox ; RPCS3 se reconnecte seul. (3) 0.44.3 : « même le clavier ne
      fonctionnait pas » → « #1 » lu comme un commentaire YAML (journal RPCS3 : device='XInput Pad', NullPad) ;
      corrigé en 0.44.4 (guillemets). À revérifier : manette ÉTEINTE au clic, allumée ensuite → elle marche.
- [ ] Menu en jeu au clavier (0.11.0) dans un vrai jeu : Pause/Attn, reprendre, sauvegarde rapide, quitter.
- [ ] Succès RetroAchievements (compte, puis « Connecter mes émulateurs ») ; un jeu NES par RetroArch.
- [ ] Comptes des boutiques : régénérer la clé Steam (l'ancienne a été vue en clair dans une capture), GOG Galaxy,
      jeux offerts Epic / GOG / Prime / PS Plus.
- [ ] DLC : PS3 dans RPCS3, Switch dans Eden (dossiers maintenant écrits en « / »).
- [ ] Taodbox, Cheat Engine, MAME, installation DOS, assistant, consoles récentes.

## ⏳ Attend un événement extérieur

- [ ] **Firehouse — contrat 1.7 → 1.13** (16/10, message de la session Firehouse « FIREHOUSE 2 ») : Frogtend ne lit que
      la 1.3 (+ notes 1.6). ✅ Texte 1.7 → 1.13 reçu le 16/10 et intégré à CONTRAT-API-JEUX-V1.md ; exemples réels
      annoncés. → relecture par l'expert contrat : ce que Frogtend doit changer (documents 1.7, conseil 1.8, BIOS 1.9,
      cœurs 1.12, MAME 1.13, fiches réunies 2.58.0). Nouvelles transmises par
      Firehouse : 2.67.5 (BIOS du stock de départ reconnus, 7 BIOS ajoutés dont SCV `upd7801g.s01`, 6 « à décider ») ;
      2.67.8 (paquet RPCS3 servi de nouveau, RPCS3 a renommé ses fichiers `_win64_msvc.7z`) ; firmware Switch servi
      18.1.0, Nintendo publie 23.0.0 (rien téléchargé, avis à Seb) ; Ryubing (git.ryujinx.app) injoignable depuis la
      maison : source de son paquet peut-être périmée.
- [ ] **Firehouse — besoins 16 et 20 : proposition de Firehouse** (16/10, `docs/media/28-FROGTEND-BESOINS-16-20.md`
      côté Firehouse, soumise à Seb, RIEN codé) : 16 = `POST /identifier` (empreinte du stock → nom exact → titre
      unique, sinon candidats ; jamais deviné) + `GET /base/{launchbox_id}` ; PS3 : Frogtend extrairait
      `PS3_GAME/PARAM.SFO` de l'ISO et enverrait TITLE_ID + TITLE (forme proposée, PAS retenue). 20 = deux types par
      plateforme : logo/bannière transparent ≤ 500 px + fond 16:9 ≤ 1000 px. Images copiées depuis D:\LaunchBox\Images
      de Venkman d'abord. Ne rien coder avant la décision de Seb, transmise par Firehouse.
      Écart signalé par Firehouse : `/recherche` → `jaquette` est une URL Internet (images.launchbox-app.com) ;
      Frogtend ne l'affiche pas (vérifié 16/10) : ne jamais l'afficher sans décision de Seb.
- [ ] **Firehouse — images de Dune et de tout MS-DOS** (cause donnée par Firehouse le 16/10) : depuis la 1.6, seules les
      images RAPATRIÉES sont servies ; le rapatriement MS-DOS n'a jamais été lancé (25 images de Dune « à
      télécharger »). Firehouse le présente à Seb, chiffré (PC, MS-DOS, PS3…). Rien à changer dans Frogtend.
      16/10 ~15 h : rapatriement des images des jeux POSSÉDÉS en cours (accord de Seb, plafond 5 Go) ; les jaquettes
      de Dune, etc. reviennent dans les médias 1.6. À revérifier dans Frogtend ensuite. (Même jour : stockages
      renommés côté Firehouse, Stay-Puft / Proton / PKE ; rien ne change pour Frogtend.)
      Demandes en attente du choix de Seb (Firehouse les a notées) : `version` d'une « installation » de BIOS,
      `autoconfig.zip` servi, `/emulateurs/retroarch/paquet`.
- [ ] **Firehouse — besoin 15** : la liste MAME (MAME.xml de LaunchBox) servie par l'API.
- [ ] **Firehouse — besoin 16** : fiche et jaquette d'un jeu importé, par `launchbox_id` (les 7 jeux PS3 importés
      n'ont pas d'informations).
- [ ] **Firehouse — besoin 17** : MAME et ses packs (non-merged + CHD, même version — ✅ décidé par Seb le 03/10).
- [ ] **Firehouse — besoin 18** : la liste « contenus » (DLC, mises à jour) par version — accepté le 03/10.
- [ ] **Firehouse — besoin 19** : BIOS et micrologiciels — ✅ FIGÉ côté Firehouse (contrat 1.9, précisé 1.10–1.11 ;
      texte dans CONTRAT-API-JEUX-V1.md, reçu le 16/10). Exemples réels frais annoncés par Firehouse ; coder contre
      eux seulement quand ils sont là.
- [ ] **Firehouse — stock de départ** (~1,9 To de Venkman) : réconciliation des jeux déjà présents (nom ou empreinte,
      ne rien retélécharger), MUGEN, lot PS3 en dernier (simulation montrée à Seb d'abord).
- [ ] **Firehouse — contrat 1.21 (besoin 28, RÉCONCILIATION) EN SERVICE** (20/10 15 h 22, Firehouse 2.89.0 ; détail
      dans CONTRAT-API-JEUX-V1.md § 1.21). Seb a dit (20/10) qu'il le rappelait à Firehouse. Index : sha1 des
      cartouches prêt ; contenu des archives vers 20 h 05 ; signatures MAME dans la soirée. Coder APRÈS le relevé
      réel promis par Firehouse (règle de Seb : forme exacte + vrai échantillon) ; premier essai après ~20 h 30.
      ✅ RELEVÉ RÉEL reçu (20/10 soir, Firehouse 2.90.1 ; index complets : 16 511/16 512 archives, 8 322 signatures
      MAME) → `fixtures/simule/reconcilier-releve.json` : exact sha1 / contenu / mame_ensemble, MAME autre version =
      probable « nom » 0,6 avec media_id, inconnu, groupe probable (manquants 1, media_id null). N'existent pas dans les
      vraies données : « plusieurs candidats » (0,5) et groupe .cue+.bin (aucun disque dans le stock).
      ✅ Seb 20/10 : (1) rattacher par une **correspondance** (le jeu importé reste tel quel, numéro, fichiers, temps de
      jeu ; on note « c'est la fiche N », réversible ; jamais de migration d'id — avis de l'expert contrat : la
      protection « un import ne s'efface jamais » repose sur l'id négatif) ; (2) un import reconnu est visible de
      **TOUS les profils qui voient ce jeu** chez Firehouse (sans retéléchargement) ; (3) versions qui ne donnent pas
      le même jeu : **montrées à la personne**, rien de rattaché seul. Conception (expert contrat) : table
      `correspondances` dans jeux.db (par fichier lancé, `telechargement_id` NULL possible), propositions
      probable/inconnu dans le cache du PROFIL, un fichier d'un groupe suit le statut du GROUPE, clé opaque (jamais un
      chemin), lots qui ne coupent jamais un groupe, 413 → lot coupé en deux, 404 → « Firehouse pas encore 1.21 »,
      `ajouter` refuse un jeu déjà là par correspondance, fusions 1.14 réécrivent `media_id`. Lots : 1 client → 2
      empreintes (annoncées, chiffrées) → 3 correspondances → 4 fiche sur l'import.
      ⏳ RESTE (lot 4) : la fiche, les images et les triches de Firehouse sur un jeu importé relié, visible de tous les
      profils qui voient la fiche (✅ Seb) ; les fusions 1.14 doivent réécrire `media_id` des correspondances.
      Conception (experts contrat et interface, 20/10) : le numéro LOCAL reste l'identité sur le PC (jouer, gérer,
      temps, abris, succès, réglages) ; la FICHE sert à l'affichage (fiche, jaquette, visuels, documents, annexes,
      triches, assistant, session) ; une seule fonction de visibilité `fiche_visible` ; la grille montre la FICHE
      (« installé sur ce PC ») et masque la ligne importée pour un profil qui voit la fiche ; `tele.jeux` aussi par
      fiche ; session sans `media_id` négatif ; fusions suivies à la lecture (`reuni_vers`). Lots 4a cœur → 4b session,
      triches, assistant → 4c interface → 4d hors-ligne et Taodbox.
      ✅ Seb 20/10 : (1) un autre profil qui voit la fiche peut **JOUER seulement** (ses sauvegardes, ses réglages) ;
      retirer, changer le programme, décompresser restent à celui qui a importé ; (2) **le lien est GARDÉ** au retrait
      du jeu importé (réimporter le même fichier le retrouve) — mais tant que le jeu est absent du PC, il ne compte pas
      (le téléchargement depuis Firehouse est permis : corrige le défaut de la 0.84.0) ; (3) deux imports reliés à la
      même fiche : « Jouer » prend celui du profil ouvert, sinon le plus ancien ; « Jouer avec… » propose les autres ;
      (4) filtre Boutique : un import relié est dans « Firehouse » ET « Importés de mon disque ».
      Firehouse (20/10 soir) : chaque inconnu de l'essai vérifié, aucun bug (versions écartées par le tri du stock ;
      MAME : machines absentes du romset servi). ✅ Seb 20/10 (via Firehouse) : **contrat 1.22 à venir** (besoin 29) —
      parent + CHAQUE clone d'un zip fusionné ; CHD par `chd_sha1` (SHA1 GLOBAL de l'en-tête v5 offset 84, v4 48,
      v3 80) ; machine exact si toutes ses ROM requises (hors BIOS/devices) sont dans le zip, avec une liste
      `dependances` à vérifier chez nous ; probable si au moins la moitié ; CHD présent chez nous mais absent chez
      Firehouse = exact quand même ; CHD de console hors périmètre. Coder APRÈS l'annonce + relevé réel. ✅ **1.22 EN SERVICE** (20/10 ~18 h 25, Firehouse 2.92.0 ;
      détail dans CONTRAT § 1.22). Relevé réel exact REÇU → `fixtures/simule/reconcilier-122.json` ; à faire : lire `machines` pour l'arcade (un zip fusionné → parent + clones), `chd_sha1` des CHD,
      vérifier les `dependances` (BIOS/devices) chez la personne.
      Avis de l'expert lancement (côté fichiers, mesures réelles en lecture seule sur E:) : archive = `entrees`
      toujours + `sha1` du fichier entier sous un plafond réglable (512 Mo) ; zip MAME = nom court + entrées, JAMAIS de
      sha1 ; fichier en clair = crc32 + taille (une passe crc32+sha1, jamais `fs::read`) ; images disque = nom + taille
      (rien lu) ; **CHD hors v1** (champ non défini au contrat) ; une entrée non vide sans CRC → pas d'`entrees` ; CRC
      des 7z à ajouter à `EntreeSeptZ` ; lecteur isolé par LISTE (pas un enfant par archive : 18 141 .7z) et répertoires
      zip AUSSI par l'enfant (28 784 zips étrangers, `panic = "abort"`) ; une version locale à plusieurs fichiers = un
      groupe, jamais de fichier annexe ; disque réseau détecté (`GetDriveTypeW`) : répertoires seulement sans accord ;
      annonce chiffrée (fichiers, Go lus en entier, réseau, durée mesurée) ; cache d'empreintes SQLite PC (chemin +
      taille + date + version du calcul) ; réponses jamais partagées entre profils. Mesures : NES 1 086 (230 Mo) en
      11,5 s ; 14 191 zips MAME : répertoires en 136 s à froid ; 10 679 7z en 96 s ; 5 643 zips MAME FUSIONNÉS (clones
      en sous-dossier, 798 noms en double) → question à Firehouse (besoin 29).

## 💡 Idées notées — PAS démarrées

- Profils de manette par jeu ; Dolphin et les manettes PlayStation ; extensions Cheat Engine du forum (voir
  [IDEES.md](IDEES.md)).
- MUGEN : un « build » = un jeu Windows prêt à jouer, personnages et décors = ses contenus (réponse à Firehouse, 03/10).
- Jellyfin → Taodbox : projet côté Firehouse, pas le travail de Frogtend.

- [ ] **Pare-feu Windows et RetroArch** (vu le 16/10 à l'essai) : au premier lancement d'un RetroArch avec les
      commandes réseau de Frogtend (`network_cmd_enable`, port 55355, lié à 127.0.0.1), Windows demande « autoriser
      les réseaux publics et privés ? ». Les commandes marchent SANS l'autoriser (elles restent sur la machine). Chez
      un novice (la sœur de Seb), cette question fait peur : chercher pourquoi RetroArch déclenche la question malgré
      127.0.0.1 (autre fonction réseau ?) et comment l'éviter, ou au moins le dire avant. Ne jamais répondre à la place
      de la personne (décision de sécurité).
      Enquête (16/10, code de RetroArch 1.22.2) : `network_cmd_bind_address` N'EXISTE PAS ; le port des commandes est
      ouvert sur TOUT le réseau (`command_network_new` → `socket_init(..., NULL, AI_PASSIVE)`). Commandes par l'entrée
      standard (`stdin_cmd_enable`) : présentes dans le code mais ABSENTES du RetroArch Windows officiel (essai réel :
      rien). Touches rares acceptées : F13, F14, F15, Aide seulement (4 pour 8 actions).
      ✅ DÉCIDÉ par Seb le 16/10 : « laisser le comportement par défaut de RetroArch et juste prévenir pour la
      fenêtre » → Frogtend prévient UNE fois par PC, avant le premier jeu RetroArch, de répondre « Annuler ».
      Seb (18/10) : l'essai « Annuler cliqué pour de vrai » est inutile (refusé) ; il laisse son installation telle
      quelle (règles « Autoriser, privé » posées sur le RetroArch d'essai supprimé : sans effet, gardées).

- 19/10 (Seb : « n'oublie pas de répondre à Firehouse ») : réponse envoyée à la session Firehouse (accusé des
  contrats 1.15 et 1.16, ce que Frogtend en fait ; questions : `D:\` pour `%rom_dossier%` à la racine, XISO pour xemu,
  ScummVM non branché, besoin 23 a/b, compte retenu `username#grade`). Remise acceptée par le serveur, lecture NON
  confirmée : à retransmettre par Seb si pas de réponse.

## ✅ Livré (version — ce qui a été vérifié)

- **0.98.0** (21/10) — Contenus PS3 servis par Firehouse (contrat 1.24 ; avis de l'expert lancement avant et après,
  ses IMPORTANTS corrigés) : `ps3_de_la_version` (fichiers aux rôles `dlc`/`maj`/`licence` de la version, SANS tri par
  TITLE_ID — un disque BCES00510 sert des licences BCES00799 ; .rap apparié par l'en-tête du .pkg, jamais le nom ;
  .rap seul = licence seule si son nom est un content ID) ; `installer_ps3` : .pkg vérifié par un relevé avant/après
  de `dev_hdd0\game\<jeu>` (sinon un fichier récent dans `game`), « déjà installé hors de Frogtend ? » dit tel quel ;
  licence posée même si le .pkg n'a rien changé ; `poser_licence` : 16 octets, jamais par-dessus (create_new,
  relecture) ; `dev_hdd0` déplacé par `vfs.yml` → refus ; RPCS3 ouvert → refus (avant le compte, et avant chaque
  contenu) ; `exiger_emulateur_pour_ecrire`. Parties GTA V : hors de `userdata`, `<identifiant Steam>`/`<identifiant
  utilisateur>` = un sous-dossier par utilisateur, énumérés (32 au plus, sans lien), jamais le compte Steam.
  Vérifié : tests (fixture God of War III, .iso construit pour `titre_id_ps3`) ; essai RÉEL en lecture seule
  (contrat 1.26 en service) : GTA V → `OneDrive\Documents\Rockstar Games\GTA V\Profiles\706F1DE1`, 0 illisible ;
  XCOM 2 inchangé. NON vérifié : une vraie installation par RPCS3 (heure des fichiers écrits, réinstallation
  d'un .pkg déjà là, code de retour en échec) ; un .pkg illisible n'est pas encore signalé dans la liste.

- **0.97.0** (21/10) — Parties Steam HORS du nuage (contrats 1.25/1.26 `sauvegardes` ; relecture de l'expert
  lancement : 2 bloquants corrigés) : `nuage_steam::resoudre` (jetons du contrat seulement ; Mes documents par le
  registre — OneDrive ; refus de la racine perdue « \… », `~`, registre, jeton inconnu, `..`) ; `emplacements`
  (os « Windows »/« Steam », liste ouverte ; `nature` 1.26 dossier/fichier/motif, déduite sinon ; une RACINE —
  dossier du jeu, de Steam, de Windows, de l'utilisateur, d'un disque — jamais un emplacement ; un dossier dans un
  autre une seule fois) ; `ajouter_emplacements` (sans lien, déjà couvert par le nuage écarté — séparateurs
  normalisés —, taille comptée AVANT toute lecture : plus de 2 Go = illisible, un emplacement en échec ne gêne
  jamais les autres, clé stable par chemin) ; `plan_nuage` lit Firehouse (hors ligne : le nuage seul) ; abri refusé
  pendant une partie lancée par Steam (programme du dossier) ; avant un mod, au-delà de `GROS`, le nuage seulement.
  Vérifié : tests (cœur 462) ; essai RÉEL en lecture seule : XCOM 2 → 25 fichiers dans `OneDrive\Documents\My
  Games\XCOM2\XComGame\SaveData` (que le nuage ne couvrait pas), GTA V → 2 illisibles (racine perdue chez
  Firehouse, corrigée dans sa 1.26 — en service le 21/10, relue en 0.98.0). NON vérifié : l'abri complet dans le vrai dossier des
  abris (A-TESTER) ; le délai d'attente du client Firehouse avant un mod (à vérifier).

- **0.96.0** (21/10) — Lot D, fiches des jeux de boutique prises DANS les boutiques (décision de Seb ; relecture de
  l'expert interface) : `boutiques::fiche_steam` (`store.steampowered.com/api/appdetails`, `l=french`, cache une
  semaine, hors ligne = ancienne fiche, refus = pause de 5 minutes sans rien écrire ; « en anglais » d'après la
  liste des langues) et `fiche_galaxy` (base locale, `summary` + `meta`, copie effacée, cache une semaine) ;
  commande `boutique_fiche` (Steam français d'abord, aussi pour une clé Galaxy « steam_… », sinon Galaxy) ; hôte
  d'images de la boutique Steam autorisé ; `FicheBoutique.svelte` dans le panneau (attente de 300 ms avant de
  demander : le défilement ne lance rien). Vérifié : tests (cœur 461 : la fiche réelle de XCOM 2 lue) ; essais
  RÉELS en lecture seule (Steam XCOM 2 en français ; Galaxy HZD et Shadow Tactics en anglais). NON vérifié : le
  rendu (A-TESTER) ; les images de Galaxy (adresses à gabarit ; les serveurs d'images de GOG répondaient 500 et
  gog.com 503 le 21/10 — à revoir) ; une fiche Epic hors Galaxy (aucune : TerraScape n'a que son nom).

- **0.95.0** (21/10) — A6d, jeux Epic du PC (relevé réel TerraScape installé par Seb ; relecture de l'expert
  lancement) : `boutiques::installes_epic` lit les manifestes `.item` (`%ProgramData%\Epic\…\Manifests`) en
  lecture seule — seulement un JEU (`AppCategories` « games »), complet, avec un programme, pas un DLC, ids sûrs,
  dossier présent (Unreal Engine et plugins écartés ; Fortnite sans manifeste : rien) ; `adresse_epic` =
  `com.epicgames.launcher://apps/<namespace>%3A<item>%3A<app>?action=launch&silent=true`, IDENTIQUE au raccourci
  qu'Epic a posé sur le Bureau de Seb ; manifeste relu à chaque lancement ; `synchroniser_epic` (registre via
  `synchroniser_boutique("epic")` + carte `epic` si Galaxy n'en a pas déjà une au même id — Galaxy range les jeux
  Epic sous `epic_<AppName>` — et les cartes Galaxy d'un jeu Epic suivent l'état « installé » lu chez Epic), appelé
  à la lecture des jeux Steam et Galaxy ; message clair si Epic ne voit plus le jeu. Vérifié : tests (cœur 460) ;
  essai RÉEL en lecture seule : seul TerraScape vu, adresse exacte (rien lancé). NON vérifié : le vrai lancement
  et le suivi (EOSBootstrapper puis TerraScape.exe, même dossier) — avec Seb. Lot A6 FINI (Steam, GOG, Epic, une
  carte par jeu, pastilles). Reste aux boutiques : D (fiches des jeux Steam, contrat 1.25 `/jeu/steam/{appid}`),
  parties Steam hors nuage (contrat 1.25 `sauvegardes`), PS3 contenus Firehouse, C1 (compte Steam de Seb).

- **0.94.0** (21/10) — A6c, un jeu = une carte (avis avant code et relecture de l'expert interface ; décisions de
  Seb du 21/10) : `lire_galaxy` lit `allGameReleases` (`JeuBoutique.releases`) ; `ludotheque::regrouper` (union des
  copies que Galaxy relie, jamais par le titre ; maîtresse Steam > GOG > Epic > autres ; `installe` = une copie au
  moins ; `minutes` = la somme ; valeurs propres gardées et rendues quand la réunion se défait) appliqué à chaque
  rangement de boutique ; `restreindre` cache les copies non maîtresses (liste, plateformes, genres, hasard) ; le
  filtre par boutique trouve la carte par ses copies ; `ludotheque_groupes` ; interface : `choisirCopie` (demande À
  CHAQUE FOIS si plusieurs copies installées) → `jouerLaCarte` et `gererLaCarte` (panneau, double-clic, Taodbox),
  partie en cours reconnue sur toute copie, sélection relue, Taodbox une carte par groupe (Galaxy sans doublon GOG),
  pastilles d'origine `IconesBoutiques` (texte, aux couleurs du thème, pour TOUS les jeux). Vérifié : tests (cœur
  458, interface 225) ; essai RÉEL en lecture seule : 18 jeux de Seb reliés à une autre copie possédée (GTA V
  Steam + Epic, AoE IV Steam + Xbox…). NON vérifié : le rendu (A-TESTER) ; les cartes ne se réunissent qu'après une
  relecture de GOG Galaxy (les lignes d'avant n'ont pas `releases`). Reste A6 : A6d Epic (Fortnite à faire
  reconnaître par Epic d'abord).

- **0.93.0** (21/10) — A6a + A6b, jeux GOG du PC (avis avant code et relecture de l'expert lancement) :
  `Noyau::synchroniser_boutique(source…)` (Steam délègue ; retrait limité à SA source, testé) ; `synchroniser_gog`
  à l'import Galaxy et à l'ouverture de sa liste (clés « gog_<id> », dossier de Galaxy présent) ; lancement PAR
  GALAXY (`commande_galaxy` : client lu dans le registre HKLM, seul `GalaxyClient.exe` accepté, identifiant en
  chiffres, `/command=runGame /gameId=<id>` puis `/path="<dossier>"` écrit tel quel comme sur les forums) puis
  attente d'un programme dans le dossier et suivi comme Steam ; messages au nom de la boutique ; succès GOG au menu
  en jeu et dans Gérer. Vérifié : tests (cœur 457) ; essai RÉEL en lecture seule (rien lancé) : client trouvé
  (`D:\LaunchBox\Games\GOG Galaxy\GalaxyClient.exe`), commande construite pour HZD. NON vérifié : que Galaxy accepte
  la commande (non documentée) — 1er essai AVEC Seb ; Cyberpunk (prélanceur dans le dossier, REDlauncher hors du
  dossier : partie finie trop tôt, ou temps gonflé par REDlauncher ouvert) ; abris des parties GOG (dans le nuage
  de Galaxy, rien hors de lui). Reste A6 : A6c (une ligne par jeu possédé sur plusieurs boutiques, choix de la copie
  avant de lancer, icônes des boutiques), A6d Epic (Fortnite à faire reconnaître par Epic d'abord).

- **0.92.0** (21/10) — Lot C3, RetroAchievements en détail (relecture de l'expert interface) :
  `Progression.liste` (non envoyée avec le résumé) lue dans la réponse d'`API_GetGameInfoAndUserProgress` d'après
  l'exemple de la documentation OFFICIELLE (pas de compte RA relié chez Seb pour un relevé réel) : badges
  `media.retroachievements.org/Badge/<BadgeName>.png` / `_lock.png` (hôte ajouté à `image_autorisee`, nom filtré),
  date la plus ancienne (hardcore ou non), rareté `NumAwarded / NumDistinctPlayers`, points dans la description,
  ordre du jeu puis numéro ; `retro_du_jeu` (cœur de `succes_retro`) + `hors_ligne` ; `succes_jeu` : Steam, sinon
  RetroAchievements (erreur claire hors ligne) ; interface : bouton du bloc RA, `?jeu=<id>` (page commune), menu en
  jeu (jeux d'émulateur et jeux Windows de boutique), Gérer ▸ 🏆 Succès si le compte RA est relié (nom ET clé).
  Vérifié : tests (cœur 456, interface 221). NON vérifié : une vraie réponse de RetroAchievements (compte de Seb à
  relier), dates en UTC, calcul d'empreinte d'un CD jamais vu demandé PENDANT une partie (le bouton dit
  « chargement… »). Lot C : reste C1 (Steam en détail, attend le compte Steam de Seb).

- **0.91.0** (21/10) — Contrat 1.23/1.24, forme `archive` (avis avant code et relecture de l'expert lancement ;
  Firehouse a confirmé : zip SANS dossier racine, `lancer` relatif au dossier `extraire_dans`, introuvable = la
  personne choisit) : la FORME fait foi (qualité « Jeu — ROM » ignorée) → prêt à jouer, Methode::Archive d'UNE
  `archive_a_extraire` reçue ; `decompression::extraire_dans` (refus entier d'un chemin dangereux — fichier ou
  dossier —, jamais par-dessus un dossier non vide, place vérifiée, taille plafonnée, dossiers vides créés,
  provisoire puis renommé, résultat relu) dans `Jeu/<extraire_dans>` ; programme de Firehouse retenu par
  `choisir_lanceur` (ses contrôles), sinon choix de la personne ; réinstallation refusée avec un message clair
  (retirer d'abord) ; retrait sans `Jeu/` vide ; l'interface ne redemande pas le lanceur s'il est donné. Vérifié :
  tests (cœur 456 : parcours complet sur la forme du relevé réel, pièges, place). NON vérifié : le vrai jeu (60 Mo,
  A-TESTER) ; progression pendant une très grosse extraction (pas affichée : à faire si MUGEN/DataXL sont gros).

- **0.90.0** (21/10) — Lot C des jeux de boutique, sous-lots C2 + C4 + C5 (avis avant code et relectures des
  experts interface et lancement) : `succes::SuccesDetail` / `ListeSucces` (même forme pour Steam, Galaxy,
  RetroAchievements) ; `boutiques::succes_galaxy` (copie de la base, effacée dans TOUS les cas par un garde, place
  vérifiée, requête paramétrée ; français seulement s'il est vraiment traduit, sinon anglais dit) ; commandes
  `succes_liste` (profil ouvert exigé, une lecture à la fois, reste d'une copie interrompue effacé) et `succes_jeu`
  (jeu du PC visible ; Steam → `steam_<appid>` de Galaxy en attendant C1) ; interface `ListeSucces.svelte` (chaque
  succès est un bouton : clavier et manette ; secret caché, montré en le choisissant ; ligne choisie lisible),
  `PageSucces` servie par `/succes` ET `/taodbox/succes` (le mode canapé reste actif), ← Retour et Échap/B,
  panneau (jeu Galaxy), Gérer ▸ 🏆 Succès (jeu Steam), menu en jeu (jeu Windows de boutique). Vérifié : tests
  (cœur 454, interface 221) ; essai RÉEL en lecture seule sur la base Galaxy de Venkman (FF VIII Steam : 34 succès,
  29 obtenus, rareté, icônes ; un jeu GOG : 59). NON vérifié : rendu réel (A-TESTER) ; images dans la fenêtre du
  menu en jeu. Reste du lot C : **C1** Steam en détail (API `GetPlayerAchievements` + `GetSchemaForGame` en
  français, secrets, rareté `GetGlobalAchievementPercentagesForApp` — relevé réel de la rareté fait : `percent` est
  un TEXTE « 80.3 ») — ATTEND que Seb relie son compte Steam (nouvelle clé) pour un relevé réel ; **C3**
  RetroAchievements en détail.

- **0.89.0** (21/10) — Lots B3 + B4 des jeux de boutique, mods GitHub et Nexus dans les jeux Steam (avis avant
  code et relectures des experts lancement et interface) : mods.rs `installes_dans` / `installer_dans` (paramètre
  `garde` + `steam_buildid` ; place vérifiée sur les deux disques) / `desinstaller_dans` (`apres_maj` : un fichier
  REMPLACÉ que Steam a changé est laissé à Steam, sans copie ; un fichier ajouté par le mod reste à la personne) ;
  jeux Firehouse inchangés (les anciennes fonctions délèguent avec `dossier_mods(jeu)`). commandes.rs : `PourMod`
  (Steam seulement ; garde = `<données>\boutiques\<id>\mods`, jamais chez Steam ni par un lien), état relu chez
  Steam à la vérification ET juste avant poser/retirer (`steam_pret_a_ecrire` : prêt, pas lancé — bit 64 OU un
  programme du dossier — même `buildid`), `risque_mod_steam` (anti-triche connu : refus ; en ligne / inconnu :
  accord), parties du nuage à l'abri avant (`abriter_avant_mod`), racine seulement, `mods_installes` →
  `steam_mis_a_jour`, `steam://validate/<appid>` ; boutiques.rs `buildid` (relevé réel : présent dans les 11
  appmanifest de Venkman, jamais `TargetBuildID`) et `lance`. Interface : Gérer ▸ Mods pour Steam, risque demandé
  D'ABORD, ligne ⚠ après une mise à jour, « Vérifier l'intégrité » proposé (au canapé : dit, à faire au PC),
  `nexus_lien_infos.steam` dit par le cœur. Vérifié : tests (cœur 453, interface 219 ; un aller-retour complet
  dans un faux dossier Steam avec mise à jour simulée). NON vérifié : un vrai mod dans un vrai jeu Steam (avec
  Seb, A-TESTER) ; que Steam écrive le bit 64 pendant une partie (doublé par la recherche d'un programme du
  dossier) ; `steam://validate` sur Venkman. Mineurs notés : les originaux d'un fichier laissé à Steam restent dans
  `remplaces` (place qui s'accumule, rien n'est effacé — à proposer à Seb) ; `Frogtend\mods` vide créé avant un
  refus faute de place (jeu Firehouse) ; « même disque » comparé sur le premier élément du chemin ; un
  `.frogtend-tmp` pourrait rester après une coupure pile pendant la pose. Lot B fini (B0 à B4) ; suite proposée :
  C (succès en détail), D (fiches des jeux Steam), A6 (GOG/Epic).

- **0.88.0** (21/10) — Contrat 1.23, Switch `contenus_emulateur` (avis avant code et relecture de l'expert
  lancement ; relevé réel Fire Emblem Three Houses rangé : `version-switch-contenus-123.json`) : le fichier lancé
  = `installation.lancer` s'il est reçu, sinon L'UNIQUE fichier au rôle `jeu` (jamais un .nsp ; sinon refus) ;
  nature non dite → image disque ; contenus : Eden seulement (un autre émulateur Switch = réglage à changer, dit
  dans le bloc 📦), `exiger_emulateur_pour_ecrire`, Eden portable exigé (dossier `user`) ; après l'installation,
  le message renvoie au bloc 📦 (aucune écriture chez Eden sans le oui). Vérifié : tests (cœur 452, interface
  218). NON vérifié : un vrai jeu Switch avec contenus (Eden n'est pas réglé dans le Frogtend de Seb) ; Eden ≥ 0.2.0
  pour un Eden installé à la main (version jamais lue). Plus tard : retirer du `qt-config.ini` l'entrée posée par
  Frogtend au retrait du jeu ; empreinte par fichier de contenu (Firehouse propose une mineure : utile, à brancher
  dans un lot à part) ; repli « Install Files to NAND » pour un vieil Eden.

- **0.87.0** (21/10) — Lot B2 des jeux de boutique, abris du nuage Steam (avis avant code et relectures des experts
  lancement et interface) : `nuage_steam.rs` (lecture tolérante de `remotecache.vdf` ; racine 0 = `remote\`, les
  autres retrouvées par taille + SHA1 dans les dossiers de Windows du registre, jamais devinées ; racine 1 =
  manettes, sautée ; partie plus récente que Steam, à un seul endroit : copiée et comptée ; OneDrive « en ligne
  seulement » jamais lu ; `remote\` parcouru sans suivre lien ni jonction, 20 000 fichiers au plus ; abri par
  dossier provisoire `.frogtend-en-cours` puis renommé, chaque copie vérifiée par SHA1, un fichier qui change est
  recopié une fois sinon refus ; `nuage-steam/<marque du compte>/…` + `abri.json`, « déjà à l'abri » par compte) ;
  commandes `nuage_steam_apercu` / `nuage_steam_abriter` (compte du profil, sinon le seul compte du PC ; refus
  pendant une partie ; > 2 Go : confirmer) ; sauvegarde du profil : jamais les parties d'un autre compte Steam.
  Vérifié : tests (cœur 451, interface 218) ; essai RÉEL en lecture seule chez Steam, abri dans un dossier
  temporaire : XCOM 2 42 fichiers 98 Mo (OneDrive Documents), 1466860 8 fichiers, 241100 5 fichiers, « déjà à
  l'abri » ensuite. NON vérifié : un vrai fichier OneDrive « en ligne seulement » ; l'abri dans le vrai dossier des
  abris de Seb (A-TESTER) ; deux comptes Steam sur un PC (l'envoi ne prend que l'abri le plus récent du jeu) ;
  remettre en place (plus tard, avec Seb). Suite : B3 (mods Steam), B4 (Nexus).

- **0.86.0** (21/10) — Lot B0 + B1 des jeux de boutique (avis avant code et relectures des experts lancement,
  contrat et interface) : `steam_appid` envoyé à `/triche/recherche` ; `cheatengine_risque` (anti-triche du disque
  ou de Firehouse `en_ligne.anticheat` = refus ; en ligne, ou jeu de boutique inconnu / hors ligne = avertissement
  et `accepte`) revérifié par `cheatengine_lancer` sur UN relevé de la partie (refus si elle a changé) ;
  `processus_du_jeu` cherche aussi dans le dossier du jeu ; mods : « trop de fichiers pour savoir » refusé pour un
  jeu de boutique seulement ; `jeu_copie_avant_mod` refusé pour un jeu de boutique ; interface : Gérer ▸ Triches et
  Cheat Engine (hors canapé), accord avant brancher (page et menu en jeu), bloc anti-triche / en ligne.
  Vérifié : tests (cœur 446, interface 218), relevé réel lecture seule (GTA V, XCOM 2, Skyrim SE ; fixtures
  `triche-steam-*.json`). NON vérifié : un vrai branchement de Cheat Engine sur un jeu Steam (A-TESTER, avec Seb) ;
  `en_ligne` dans `/jeu/{id}/triche` (jeux de Firehouse) ; le pid écrit pour Cheat Engine qui changerait avant
  qu'il le lise (mineur noté). Suite : B2 (parties du nuage Steam), B3 (mods), B4 (Nexus).

- **0.85.1** (20/10) — Contrat 1.23, multi-disques zippés (avis et relecture de l'expert lancement) :
  `installation.lancer` suivi (s'il est un fichier reçu) ; `archive_a_extraire` décompressés à côté
  (`decompression::extraire_a_cote` : tout examiné avant d'écrire, lien dur pour ne jamais écraser, `take(taille)`
  contre une archive piégée) ; zips gardés ; `Installation.extraits` relevés au manifeste, mis à l'abri avant
  réinstallation ou retrait, effacés au retrait ; `verifier_m3u` avant d'installer (chaque disque présent).
  Vérifié : tests (cœur 444, dont le relevé réel lu et un parcours complet installer → partie écrite → retrait avec
  abri). NON vérifié : un vrai Xak téléchargé, installé et lancé par QUASI88 (à faire par Seb, A-TESTER) ; un .d88
  qui change de TAILLE quand le jeu y écrit (la réinstallation serait alors refusée, message clair).

- **0.85.0** (20/10) — Jeux de boutique, lots A2-A4 pour STEAM (avis et relectures des experts lancement et
  interface) : `Installation.boutique` ; `synchroniser_steam` (installés d'après le Steam de CE PC, jamais la liste
  d'un profil ; une ligne n'est retirée que si le jeu est désinstallé, son dossier disparu, sans temps de jeu et hors
  partie ; `JeuPc.dossier` dans les données de Frogtend) ; `jouer` : `steam://rungameid/<appid>` puis attente (180 s)
  d'un programme DANS le dossier, qui devient la racine de la partie (suivi, session APRÈS l'apparition, menu,
  clavier) ; un second lancement refusé (`utilise`) ; « Quitter » ajoute les programmes du dossier ; `dans_un_dossier`
  avec séparateur (testé) ; garde-fous : lanceur, mods, retrait, émulateur refusés pour un jeu de boutique.
  Interface : Jouer suivi si le jeu est au registre (panneau, double-clic), « Lancement… » et message tout de suite,
  Gérer réduit, Taodbox sans doublon, « installé » de la ludothèque suit le disque. Vérifié : tests (cœur 438,
  interface 215). NON vérifié : un VRAI lancement (à faire AVEC Seb : Steam ouvert / fermé, menu en jeu, Quitter,
  manette → clavier) ; jeu lancé en administrateur ; jeu qui démarre après 180 s (non suivi). GOG (ligne de commande
  de Galaxy à vérifier avec Seb) et Epic : lots suivants. Deux temps de jeu (Steam / Frogtend) : à trancher avec Seb.

- **0.84.2** (20/10) — Contrat 1.22 (besoin 29 ; relevé réel exact rangé : `fixtures/simule/reconcilier-122.json`).
  Arcade : verdict de LA machine du zip (le parent d'un zip fusionné), pas le groupe 1.21 ; BIOS / périphériques
  cherchés à côté du zip, disques (CHD) `present`, `complet_chez_nous` : dits dans le motif sans empêcher « exact » ;
  CHD d'arcade (`<dossier>\<nom court>\`) envoyés avec leur zip, SHA1 GLOBAL lu dans l'en-tête (v3/v4/v5), jamais
  dans `sha1` ; CHD hors arcade : toujours écartés. Vérifié : tests (cœur 436 : relevé relu à l'identique, décision
  sur le relevé, CHD v3/v4/v5) ; essai réel : 30 premiers zips MAME de Seb → 16 exact (4 avant), 14 inconnus
  (machines absentes du romset de Firehouse). Relecture de l'expert contrat : disques manquants ajoutés au motif.
  NON vérifié : un vrai CHD d'arcade (Seb n'en a aucun) ; un BIOS rangé ailleurs (autre `rompath`) serait signalé à
  tort.

- **0.84.1** (20/10) — Lot 4 de la réconciliation (avis des experts contrat et interface ; choix de Seb du 20/10).
  Cœur : `reliees`/`fiche_de` (fiche suivie par `reuni_vers`, vue par CE profil ; jamais pour une fiche aussi
  téléchargée : l'import reste alors un jeu à part) ; « Ma ludothèque » montre la FICHE et masque la ligne importée ;
  « installé » sous sa fiche ; `jeu_jouable` (B joue l'import relié de A, jamais `jeu_visible` : gérer reste à A) ;
  `jeux_du_pc` + `fiche`, `titre_fiche`, `d_un_autre` ; session jamais avec un numéro négatif ; triches par la fiche ;
  liens dormants (`correspondances()` = imports encore au registre) ; la sauvegarde de B n'emporte pas l'import de A.
  Interface : `jeuSurPc` (celui du profil, sinon le plus ancien), actions sous le numéro local, « Déjà sur ce PC »,
  Gérer caché pour le jeu d'un autre (bureau, fiche, Taodbox), Taodbox sans doublon avec la jaquette, le fond, le logo
  et le titre de la fiche, ⚙ Gérer ▸ « Ce n'est pas ce jeu », sélection gardée. Vérifié : tests (cœur 432,
  interface 215), svelte-check ; test du scénario Seb/Léa. Deux interblocages de verrou trouvés par les tests et
  corrigés ; fins de ligne d'un fichier réparées (leçon). NON vérifié : dans le programme installé.
  ⏳ Lot 4d : prélire la fiche des jeux reliés (hors ligne) ; préparer les images de Taodbox par la fiche
  (`medias.rs`) ; un réimport par un AUTRE profil réveille le lien confirmé par le premier (à trancher avec Seb).

- **0.84.0** (20/10) — Reconnaître les jeux importés (contrat 1.21, lots 1 à 3 + écran ; avis des experts contrat,
  lancement et interface). Client `/reconcilier` (lots qui ne coupent jamais un groupe, 413 coupé en deux, 404 « pas
  encore 1.21 ») ; empreintes en lecture seule (`empreintes.rs` : crc32+sha1 en une passe ; répertoires zip ET 7z par
  le lecteur isolé, par LISTE, CRC compris ; disque réseau détecté ; cache SQLite du PC) ; inventaire des imports du
  PROFIL (clés opaques, noms seuls, MS-DOS/Windows écartés, CHD et disques : nom + taille) ; décision par jeu ;
  table `correspondances` + `ecartes` (jamais de migration d'id) ; « exact » concordants rattachés seuls ; écran
  📥 Importer ▸ 🔗. `ajouter` refuse un jeu déjà là par correspondance. Vérifié : tests (cœur 431, interface 214) ;
  essai réel (90 vrais fichiers, vrai Firehouse) : NES 25/30 exact, SNES 26/30, MAME 4 exact / 11 probable / 15
  inconnu. Relectures (contrat, interface) — corrigés : UNE session du début à la fin (jamais le jeton d'un autre
  profil si on change de profil pendant la lecture ; rien d'écrit alors) ; un 413 sur un seul groupe ne boucle plus ;
  réponses filtrées sur les clés du lot ; un rattachement n'est montré qu'à un profil qui VOIT la fiche ; `ajouter`
  lit la fiche AVANT de dire « déjà sur ce PC » (aucun indice d'une fiche cachée) et suit les fiches réunies ; CHD
  plus envoyés (besoin 29) ; chemins `\\?\UNC\` vus comme réseau ; une seule reconnaissance à la fois ; écran : focus
  gardé après chaque action, « Ce n'est pas lui » confirmé et réversible, durée estimée (débits mesurés chez Seb),
  « pas vérifiés » distingués, fiche sans titre jamais proposée à juger, boutons inactifs pendant l'appel.
  NON vérifié : la commande entière dans le programme installé sur les vrais imports de Seb ; affichage de la fiche
  Firehouse sur un jeu relié (lot 4, à faire) ; fusions 1.14 sur les correspondances (seul `ajouter` les suit).

- **0.83.0** (20/10) — Jeux de boutique, lot A1 « lire sans lancer » (avis de l'expert lancement) : registre de
  Windows lu par son API (`registre.rs`, jamais `reg.exe`, jamais d'écriture) ; bibliothèques Steam dédoublonnées ;
  manifestes Steam lus (`installdir` sûr, `StateFlags` : « installé » = bit 4, « prêt » = rien en attente — un jeu en
  « mise à jour au lancement » reste installé, le 1er téléchargement non) ; dossier des jeux GOG (`installationPath`,
  gardé s'il existe) ; Galaxy cherché sous %ProgramData%. Vérifié : tests (419) ; essai réel en lecture seule : 1
  bibliothèque Steam (au lieu de 2), 11 jeux avec dossier, 2 jeux GOG avec dossier. Rien inscrit ni lancé (A2+).

- **0.82.3** (20/10) — Progression d'une installation de mod (avis de l'expert interface) : le cœur émet `mod-progres`
  {suivi, étape, faits, total, débit} au plus toutes les 400 ms (débit calculé par le cœur ; décompression zip et 7z,
  pose en fichiers) ; message de progression non cliquable avec barre `role="progressbar"`, chiffres hors annonce ;
  « environ » pour Nexus ; un numéro de suivi par installation. Vérifié : tests (cœur 417, interface 211), svelte-check ;
  essai réel (vrai Fusion Fix 7,1 Mo de GitHub, faux jeu effacé) : 438 signes pendant le téléchargement, puis
  décompression et pose 3/3. Relecture de l'expert : message créé APRÈS l'écoute (jamais bloqué à l'écran), pas de
  rafale si le total est nul. NON vérifié : le rendu réel chez Seb, le lecteur d'écran.

- **0.82.2** (20/10) — Les mods en 7z (Nexus et Firehouse/GitHub) par le lecteur isolé de 0.82.1 (avis de l'expert
  lancement) : format pris sur les octets ; liste par l'enfant puis les règles de mods.rs (liens refusés par leurs
  attributs, noms vides sans données et entrées « à supprimer » ignorés — sevenz-rust 0.6.1 marque à tort ainsi les
  dossiers qu'il écrit) ; extraction en UNE passe dans `brut`, chaque fichier vérifié (vrai fichier, pas de lien,
  taille exacte) puis renommé ; rar refusé. Vérifié : tests du cœur, 2 tests par le VRAI lecteur (chemin accentué, jeu
  identique après retrait) ; essai réel : vraie archive de Seb (3 244 fichiers) posée dans un faux jeu puis retirée,
  identique, archive intacte, essai effacé. Relecture de l'expert : défaut ANTÉRIEUR corrigé — réinstaller un mod
  retiré effaçait les originaux gardés (`remplaces`) et les fichiers modifiés par la personne (`gardes`) : le dossier
  est désormais mis de côté (`<clé>-ancien-…`), testé ; restes d'une pose arrêtée net (`brut`, `contenu`) rangés ;
  conflit avec un autre mod vu AVANT de décompresser ; place = 2 × mod + originaux. NON vérifié : un vrai mod .7z de
  Nexus chez Seb ; la progression pendant l'extraction d'un mod (toast fixe, comme avant).

- **0.82.1** (20/10) — Toute lecture de 7z par un Frogtend ENFANT (`sept_z.rs`, avis de l'expert lancement : voie B) :
  pré-lecture des 32 octets dans le parent ; job Windows (2 Gio, tué avec Frogtend) posé avant « go » ; lignes JSON,
  réussite = ligne de fin ET code 0. Migrés : decompression.rs, installation.rs (2), succes.rs, commandes.rs.
  Vérifié : tests du cœur (413), test avec le VRAI programme (chemin accentué, archive abîmée, en-tête codé gonflé à
  3 Gio + dictionnaire 3,75 Gio → l'enfant meurt, Frogtend répond « piégée ») ; essai réel en lecture seule sur 3
  archives de Seb (37 / 3 244 / 502 fichiers, 251 Mo, tailles exactes, archives intactes, essai effacé).
  Après la relecture de l'expert : progression envoyée à chaque ligne (elle restait figée : test qui le prouve, rouge
  sans le correctif) ; lignes et entrées bornées côté parent ; empreinte RetroAchievements calculée DANS l'enfant (plus
  de fichier dans %TEMP%) ; pas de fenêtre d'erreur Windows quand l'enfant meurt ; contrôle des 32 octets partout.
  Rejoué contre le programme LIVRÉ (release : sans console, `abort`) : 5 tests dont l'essai réel, aucun rapport
  d'erreur Windows ni processus restant. Disque D: plein pendant la livraison (cache de compilation 65,7 Go, vidé) :
  `livrer.mjs` vérifie désormais la place (20 Go) et trouve le CMake de Visual Studio.
  NON vérifié : dans le programme INSTALLÉ chez Seb (antivirus, Sunshine) ; la mémoire réelle sur ses gros 7z ; un
  chien de garde (enfant qui bouclerait sans rien écrire : comme avant, il bloquerait la décompression).

- **0.82.0** (20/10) — Nexus Mods. Clé d'API personnelle par profil (vérifiée par `/users/validate`, au coffre,
  effacée avec le profil) ; `nxm://` pris par l'installeur (HKCU, une fois, rendu à la désinstallation) ; un 2e
  Frogtend dépose le lien dans `nxm-recus\` et s'arrête (mutex nommé ; jamais avec `--taodbox`, même un lien
  illisible ne fait jamais deux Frogtend) ; « Ouvrir sur Nexus » (mod et page repris de la liste de Firehouse) relie le
  lien au jeu (30 min, par profil) ; client API sans redirection, clé par requête ; téléchargement sans clé, taille de
  Nexus = plafond, `md5_search` exige le mod et le fichier demandés ; zip seulement (7z depuis 0.82.2) ; archive toujours ramassée.
  Vérifié : tests du cœur (410) et de l'interface (209), svelte-check 0 erreur, 3 experts (corrigés : un lien
  illisible lançait un 2e Frogtend, archive laissée dans le jeu sur erreur, 401/403 Nexus pris pour le jeton
  Firehouse, occupation trop tardive, clés NSIS avant BUNDLEID, lien reçu pendant une fenêtre oublié, textes).
  NON vérifié : aucun essai réel avec Nexus (il faut la clé de Seb) ; installeur non installé ici (registre de Seb).

- **0.74.0** (19/10) — RPCS3 : image et fluidité par jeu (✅ Seb 19/10 : shaders = « les deux »). Clés et valeurs
  relevées dans les SOURCES de RPCS3 (`Emu/system_config.h`, `system_config_types.cpp`, `Emu/system_utils.cpp` :
  `config\custom_configs\config_<numéro>.yml`, appliqué par-dessus le global) et vérifiées dans le vrai `config.yml`
  de Seb (lu sans écrire). Cœur : liste fermée de 6 réglages, numéro de disque vérifié (`[A-Z]{4}[0-9]{5}`), lecture
  clé par clé (jeu, global, défaut), écriture des SEULES clés changées (`regler_cles_yaml` + `remplacer_config`,
  copie à l'abri), « Valeurs de RPCS3 » retire nos clés (section vidée retirée), RPCS3 fermé, programme de confiance.
  Interface : sous-menu « 🎨 Image et fluidité » dans ⚙ Gérer le jeu (libellés sans jargon, « pour ce jeu, pour tout
  le monde sur ce PC, au prochain lancement »). Avis des experts lancement et interface avant de coder et relecture
  avant livraison. Vérifié : tests (forme réelle du fichier, liste fermée, réglage fait à la main gardé, retour) ;
  lecture seule sur le registre de Seb (Asura's Wrath : BLES01227 ; 6 jeux encore en .zip : illisibles, l'écran le
  dit) ; répétition sur une COPIE de son `config.yml` (fichier du jeu = 2 lignes, global intact) ; 374 tests cœur,
  184 interface. NON vérifié : RPCS3 lancé avec ces réglages (essai demandé à Seb).
  Relecture avant livraison appliquée : numéro lu en OCTETS (un PARAM.SFO non ASCII faisait planter Frogtend,
  `panic = abort`) ; doublons et « vide + valeurs » refusés ; fichier du jeu jamais laissé vide (les 2 lignes de la
  pause au menu à la place) ; global illisible = erreur ; texte « disque illisible » renvoie au vrai bouton
  « 🗜 Décompresser pour jouer » ; valeur héritée choisie exprès = figée pour le jeu. FSR existe aussi pour OpenGL
  dans RPCS3 (`Emu/RSX/GL/upscalers/fsr_pass.h`) : rien à cacher selon le moteur. Avertissement « cache de shaders
  désactivé » noté dans IDEES.
- **0.74.1** (19/10) — ✅ Seb 19/10 (lot suivant choisi : « PS3 : cache de shaders ») : 🎨 Image et fluidité avertit
  si `Video ▸ Disable On-Disk Shader Cache: true` (fichier du jeu, sinon global ; clé vérifiée ligne 102 du vrai
  `config.yml` de Seb, où elle vaut `false`). Lu seulement, jamais écrit ; une erreur de lecture ne ferme pas l'écran
  (`unwrap_or(false)`). Relecture de l'expert lancement : texte corrigé (début de partie plus lent, pas des saccades)
  et « comment le rétablir » ajouté. NON vérifié : l'onglet exact de RPCS3 où se trouve la case (le texte ne le nomme
  pas).
- **0.75.0** (19/10) — ✅ Seb 19/10 (lot choisi : « Xbox : prévenir avant »). Doc officielle xemu
  (xemu.app/docs/disc-images) : XISO exigée, Redump incompatible, conversion `xdvdfs pack`. Code xdvdfs
  (`xdvdfs-core/src/layout/volume.rs`) : descripteur 0x800 octets au secteur 32, magie `MICROSOFT*XBOX*MEDIA` au début
  ET à 0x7ec ; partition du jeu d'une Redump à 387 Mio. Cœur : `disque::image_xbox` (2 lectures de 0x800 octets au
  plus) ; `jeu_jouer` refuse AVANT `preparer_emulateur` et `marquer_occupe` (rien écrit, rien copié) quand l'émulateur
  est xemu et l'image Redump ou une archive ; inconnue/illisible = lancée. Le fichier contrôlé est celui de CETTE
  partie (`installation_de_la_partie`, commun avec `jouer`). Nouvelle sorte d'erreur `prevenir` → fenêtre maison
  `informer`. Choix sans question à Seb (avis de l'expert) : une archive pour xemu est refusée aussi, avec renvoi vers
  « 🗜 Décompresser pour jouer ». Conversion en XISO : plus tard (IDEES), avec accord. NON vérifié : aucun jeu Xbox
  chez Seb ni au stock (tests sur fichiers creux seulement). Relectures (lancement + interface) appliquées : le
  bouton n'est cité que s'il apparaîtra (`decompression::examiner` ; pas pour un .rar ni pour une autre version que
  l'habituelle), avec le chemin (panneau de la ludothèque, hors Taodbox) ; « xemu » → « l'émulateur de la Xbox » ;
  « écran noir » (non vérifié) → « ne se lancerait pas » ; « tes fichiers restent intacts ».

- **0.73.0** (19/10) — Durcissement, étapes 5 à 7 (fin du lot). Écritures dans les dossiers d'émulateurs :
  installation seulement dans `dossierEmulateurs`/`dossierOutils` réglé (`garde::dossier_de_base_admis`, créé s'il
  manque) ; BIOS, cœur RetroArch, réglages de manette, triches seulement pour un émulateur de confiance ou qui vient
  d'être choisi (`a_ete_choisi`, sans consommer : l'assistant pose BIOS et cœur avant d'enregistrer). Emplacements :
  `import_copier` sous un emplacement réglé ; `import_installer_dos` dans un dossier choisi ou un emplacement réglé,
  jamais un dossier système ; `abris_regler` = le `dossierAbris` réglé. Recherche d'import dans un dossier `\\réseau`
  annoncée avant. Relecture de l'expert lancement : ni bloquant ni important (parcours « Prêt à jouer », assistant,
  import DOS, copie, abris suivis un par un ; le cœur lit le même magasin `pc.json` que l'interface, à jour). Vérifié :
  tests (`dossier_de_base_admis`) ; 373 tests cœur, 181 interface.

- **0.72.0** (19/10) — Durcissement, étape 4 (Seb : « Poursuis »). Les fenêtres de choix sont ouvertes PAR LE CŒUR
  (commande `parcourir` : `tauri-plugin-dialog` 2.8.0, API lue dans ses sources, `set_parent`, `blocking_pick_*` dans
  `spawn_blocking`) qui retient le chemin choisi (fichier ou dossier, 10 min, une fois : `retenir_choix` /
  `consommer_choix`) ; `dialog:allow-open` retiré à l'interface. Consommateurs : `choisir_lanceur` (programme hors du
  dossier du jeu, .bat/.lnk d'ailleurs), `installation_ailleurs` (dossier), `emulateur_adopter` (détecté, sous le
  dossier des émulateurs/outils réglé, ou choisi), `import_ajouter` (`programme_admis` : jamais un programme de
  Windows ; DOSBox de confiance avec EXACTEMENT `arguments_jeu_dos` ; sinon sans arguments, sous le dossier du jeu non
  système, ou choisi). Correctif hors lot : un .lnk importé passe par explorer, un .bat par cmd. Correctif trouvé
  par un test ROUGE pendant la livraison (suite complète chargée, 21 s) : `Suivi` reconnaissait les processus de la
  partie à leur seul NUMÉRO, que Windows réutilise → retient aussi leur heure de démarrage (`est_le_meme`, `descend`,
  testés) ; suite complète rejouée 3 fois, verte. Relecture de l'expert
  lancement : aucun parcours normal cassé, ni bloquant ni important. Vérifié : tests (choix à usage unique et du bon
  genre ; règle d'import ; raccourci importé) ; 371 tests cœur, 181 interface. NON vérifié : l'ouverture RÉELLE de la
  fenêtre de Windows par le cœur (un clic humain) → premier essai demandé à Seb (A-TESTER).

- **0.71.0** (19/10) — Durcissement, étapes 1 à 3 (✅ Seb 19/10). Module `garde.rs` (chemins comparables : absolus,
  sans `..`, sans `\\?\`, sans casse, composant par composant ; jonctions vues par l'attribut de point de réparation ;
  dossiers système ; réglages relus dans `pc.json` ; programme de confiance = réglé, au registre, ou détecté). Lancés :
  Cheat Engine, RPCS3 (micrologiciel), DOSBox (installation DOS) seulement s'ils sont de confiance ; lanceur `cmd.exe`
  = `/C <.bat|.cmd du jeu>`, `explorer.exe` = `<.lnk du jeu, du Bureau ou du menu Démarrer>`, tout programme sous
  `%SystemRoot%` traité comme système (le chemin complet de cmd ne contourne plus rien), un autre programme SANS
  arguments ; `cmd`/`explorer` pris dans `%SystemRoot%`. Effacements : « installé ailleurs » refuse dossiers système
  (racine de disque ou de partage), réglés, contenant un dossier réglé ou d'autres jeux ; `unins000.exe` seulement dans
  un dossier installé par Frogtend, sinon `reste_installe` dit à la personne et `desinstaller_reste` (une fois, 10 min,
  après un oui, fenêtre visible) ; `remove_dir_all` sans détour ni jonction (test : jonction DANS le dossier installé
  jamais suivie). Seules les NOUVELLES entrées sont vérifiées. Limite dite à Seb :
  l'interface écrit `pc.json` (protection contre un bogue, pas une fenêtre compromise). Vérifié : tests (chemins,
  dossiers système, réglages, programmes de confiance, vraie jonction créée et reconnue, lanceurs refusés, jonction
  jamais traversée au retrait, vrai `.bat` lancé par le `cmd.exe` de Windows) ; lecture seule chez Seb : RPCS3 réglé
  et au registre (accepté), ni Cheat Engine ni DOSBox ; relecture de l'expert lancement (3 importants corrigés avant livraison : chemin complet de cmd, raccourcis du Bureau, désinstalleur proposé) ; 369 tests cœur, 181 interface.

- **0.70.1** (19/10) — Correctif (réponse de Firehouse au besoin 23 : `-1` est une VRAIE version, la ROM n° 1 de son
  stock) : un jeu importé est reconnu à son NUMÉRO négatif (`est_importe`, `estImporte` : `id < 0`), jamais à
  `version == -1`. Vérifié : test (jeu de Firehouse en version -1 ≠ importé ; ses médias dans le dossier du jeu) ;
  répétition en lecture seule sur le registre de Seb (9 jeux, verdict identique) ; 360 tests cœur, 181 interface.

- **0.70.0** (19/10) — Contrat 1.15 (✅ Seb 19/10, après le 1.16) : les infos du jeu dans sa fiche — date de sortie en
  français (découpée à la main, jamais `new Date`), type de sortie signalé en étiquette s'il n'est pas « Released »,
  âge conseillé (« N ans et plus (PEGI/ESRB) », rien pour « Not Rated »/« RP »/inconnu), « Avis des joueurs ★ x,y / 5 »,
  « (massivement multijoueur) ». Jeu du PC (fiche lue sur le disque) : infos relues EN DIRECT (`infos_jeu` : profil,
  `visible_ou_reuni`, repli cache puis fiche.json) comme Firehouse le recommande. Avis de l'expert interface avant de
  coder et relecture avant livraison. Vérifié : tests (formatage, garde-fous du cœur) ; essai réel en lecture seule
  (Dune, Contra III, 3x3 Eyes, Spark World : dates, notes, ESRB « Not Rated ») ; 359 tests cœur, 181 interface.

- **0.69.0** (19/10) — Contrat 1.16 (✅ Seb 19/10 : « Adapter le contrat 1.16 » d'abord, puis les infos 1.15) :
  `ligneDe` prend SEULE la ligne complète du conseil d'un autonome (RetroArch : comme avant) ; `changementDeConseil`
  compare sans le `"%rom%"` final et RÉPARE, sans question, une ligne doublée par Frogtend lui-même (préparée entre
  0 h 11 et cette version) ; `lanceur_emulateur`/`lanceur_mame` remplacent `%rom%`, `%rom_nom%`, `%rom_dossier%` (sans
  casse) et refusent une variable inconnue (cherchée sur la ligne seule) ; MAME : l'arcade garde la commande de
  Frogtend (variables du conseil retirées), une machine de salon (`studio2 -cart "%rom%"`) passe par le lanceur
  d'émulateur même zippée (décidé d'après la LIGNE). Avis de l'expert lancement avant de coder (3 trous évités) et
  relecture avant livraison. Vérifié : tests (tableau 1.16 figé : rien des options de Frogtend n'est perdu ; variables ;
  `D:\` ; arcade/salon) ; essai réel en lecture seule des conseils de Firehouse (GameCube, PS2, PS3, Arcade, Studio II,
  Jaguar, Xbox : commandes exactes) ; chez Seb, seul RPCS3 réglé à la main (jamais touché) ; 358 tests cœur, 177
  interface. NON vérifié : un vrai lancement de ces émulateurs (aucun chez Seb, hors RPCS3). ScummVM : noté (IDEES).

- **0.68.2** (18/10) — Formats des fichiers d'émulateurs (relevé de l'expert du 04/10) : xemu `hdd_path` en chaîne TOML
  basique (`chaine_toml`, au lieu de `'…'` qu'une apostrophe fermait) ; `valeur_toml` relit littérale et basique ;
  valeurs Qt (Eden `external_content_dirs`, Azahar `nand_directory`/`sdmc_directory`) toujours entre guillemets
  (`valeur_qt`), relues sans eux (`lire_valeur_qt`). Vérifié : tests (dossier « Jeux d'émulation, essai », profil
  « D'Aquin » : xemu.toml relu par un vrai lecteur TOML, chemin exact ; Azahar ; Eden « Zelda, TOTK » = un seul
  dossier, sans guillemets doublés) ; 356 tests cœur. NON vérifié en réel : xemu, Eden et Azahar lancés sur ces
  chemins (aucun chez Seb aujourd'hui).
  Relecture de l'expert lancement : (1) chemins d'Eden tous en « / » ; (2) DÉCOUVERTE — Eden lit
  `external_content_dirs` dans `[UI]` sous `Paths\external_content_dirs\…` (sources `qt_config.cpp` relues le 18/10 :
  groupe Paths DANS le groupe UI ; même forme que `Paths\gamedirs\…` du qt-config.ini de suyu sur Venkman) : depuis le
  lot contenus Switch, Frogtend écrivait dans une section `[Paths]` jamais lue — ÉCRIT mais pas BRANCHÉ. Corrigé, les
  dossiers déjà donnés sont repris UNE fois (leurs anciennes lignes retirées ensuite : un dossier retiré dans Eden par
  la personne ne revient jamais) ; (3) MAME (vérifié AVANT de poser le script dans son dossier) : dossier de jeu avec « ; » refusé (`verifier_rompath`) ; Azahar :
  `\default=false` pour nand/sdmc ; DEL échappé en TOML.

- **0.68.1** (18/10) — Confidentialité : `contenu_sauvegarde` n'envoie plus que les abris des jeux `visible_ou_reuni`
  du profil (les abris sont ceux du PC) ; un abri qui ne part plus reste sur le PC, et ce qui en avait été envoyé à CE
  compte reste dans la version courante (`Contenu.gardes` : jamais retiré — relevé BLOQUANT de l'expert lancement :
  sinon la copie de Firehouse disparaissait après 8 semaines d'instantanés) ; même règle pour les parties d'un jeu
  installé devenu invisible. Bilan `non_envoyes` (nombre de jeux, jamais nommés). `sauvegarder` refuse une ludothèque
  jamais synchronisée (`synchronise_le` absent) ; la sauvegarde automatique se tait alors. Vérifié : tests (abri d'un
  jeu caché jamais envoyé et gardé sur le PC ; parties d'un jeu devenu invisible gardées dans la version courante,
  `retires` = 0 ; refus sans synchronisation, aucune requête) ; répétition en lecture seule chez Seb (1 abri, visible ;
  date de synchronisation présente) ; 351 tests cœur, 174 interface.
- ✅ **Seb 18/10 : « Laisser comme ça »** (un profil ne change pas de personne chez lui) — les parties d'ÉMULATEURS d'un profil (`<émulateur>\Profils\
  <profil>\`) partent sans filtre de visibilité : si le jeton d'un même profil passe d'un admin à un mineur, les
  fichiers de parties de jeux cachés (au nom parlant) partiraient dans l'espace du mineur. Prévenir ? Exclure jusqu'à
  un choix explicite ?

- **0.68.0** (18/10) — Confidentialité (brief § 2) : le cache d'un profil ET le profil retiennent son compte Firehouse
  (`username#grade` de `/moi` : un grade changé compte comme un autre compte). Un jeton d'un AUTRE compte (changé dans
  « Mon profil », en mode simulé — compte inconnu, dans le doute —, ou ailleurs : vu par `/moi` au début de chaque
  synchronisation, ou à l'ouverture même hors ligne) fait oublier tout ce qui avait été reçu AVANT d'être montré :
  catalogue, fiches, fusions, jaquettes, visuels, listes de documents, résumé de la dernière sauvegarde (les fichiers
  de documents téléchargés restent, jamais montrés). Une session remplacée est « périmée » : plus aucune écriture
  (synchronisation ou image en route avec l'ancien jeton : arrêtée, rien gardé). Le jeton n'est rangé qu'une fois le
  cache vidé ; un échec ne change rien. Les jeux des boutiques restent. Un jeton renouvelé du même compte ne change
  rien ; un profil existant retient son compte au premier passage sans rien vider. Relecture de l'expert contrat (1
  bloquant — synchronisation en cours qui réécrivait l'ancien catalogue — et 4 importants, corrigés). Vérifié : test
  complet (même compte, autre compte, session périmée, grade changé, ouverture hors ligne, jeton changé en simulé) ;
  essai réel `/moi` → `username: "sebastien"`, grade admin (jeton lu dans le coffre, jamais affiché) ; 348 tests
  cœur, 174 interface.

- **0.67.1** (18/10) — Correctif : le mode simulé a son propre dossier de cache (`profils/<id>/simule` : catalogue,
  jaquettes, visuels, documents) ; il ne remplace plus le vrai catalogue du profil ni ses images (les numéros se
  recouvrent). Comptes du profil (Steam, RetroAchievements) inchangés. Les caches déjà mélangés sont assainis :
  `SCHEMA_CATALOGUE` « 3 » force une relecture complète unique, et une relecture d'un ancien schéma vide aussi les
  fiches gardées (relevé de l'expert contrat). Vérifié : test aller-retour vrai → simulé → vrai (catalogue et images
  intacts sans resynchroniser), test d'un ancien cache mêlé (Dune d'exemple remplacé, sa fiche partie) ; 347 tests
  cœur. Au passage : 12 octets nuls bruts de deux tests
  remplacés par `\0` (le fichier passait pour binaire aux outils de recherche).

- **0.67.0** (18/10) — Lot Médias M4 (FIN du lot Médias) : ménage des caches d'images d'un profil après une
  synchronisation lue en entier, face au vrai Firehouse seulement (`jaquettes/`, `visuels/` des ids ni au catalogue
  ni au registre ; la LISTE des documents seulement, jamais leurs fichiers ; jamais un lien) ; repli hors ligne sur la
  même empreinte à une autre largeur (jaquette, visuels ; un jeton refusé se dit toujours) ; fonds et logos de Taodbox
  gardés à l'avance après la synchronisation (jeux du PC prêts et visibles ; plafond de 50 jeux TÉLÉCHARGÉS par passage,
  le reste au passage suivant ; un 404 fait passer au jeu suivant ; arrêt hors ligne ; un seul passage à la fois —
  deux défauts relevés à la relecture et corrigés avant livraison).
  `BilanSynchro.tronque` (plafond de pages : pas de ménage). Avis de l'expert contrat avant de coder (bloquants évités :
  mode simulé qui partage le cache du profil, documents parfois seule copie) et relecture avant livraison. Vérifié :
  tests (346 cœur, 174 interface), répétition en lecture seule sur une copie des bases de Seb (catalogue 12 717 jeux,
  9 jeux du PC, 66 jaquettes : 0 retirée). NON vérifié en réel : un vrai hors ligne (câble débranché).

- **0.66.0** (18/10) — Lot Médias M3 : dans Taodbox, le fond du jeu focalisé (fond, sinon capture, sinon écran titre ;
  flou léger, `--voile-fort` et encres de voile, d'après la charte § 7) et son logo dans un bandeau à hauteur fixe
  (sinon son titre), changés après 200 ms de repos du focus ; Steam, GOG et boutons : fond du skin ; réglage du profil
  `ludotheque.fondsTaodbox` (oui d'office). Fondu du menu en jeu soumis aux animations réduites. Avis de l'expert
  interface avant de coder et relecture avant livraison (éclair sans voile corrigé : `{#key}` + voile qui suit
  l'image). Vérifié en réel (`essai_taodbox`, profil temporaire) : logo de 116 en WebP (43 ko, 456 ms puis 5 ms en
  cache), fond 118 ko ; Dune (110) a désormais logo et fond chez Firehouse. Tests : 174 interface (règle du repos
  testée avec de fausses minuteries). NON vérifié : le rendu à l'écran et sur la télé (à voir par Seb).

- **0.65.0** (18/10) — Contrat 1.14 : un jeu du PC dont la fiche est réunie chez Firehouse est RATTACHÉ à la nouvelle,
  seulement si tout concorde (même version, mêmes fichiers nom + taille, nouvelle fiche visible du profil et absente
  du PC, jeu ni joué, ni installé, ni téléchargé, médias plus à l'ancien emplacement). Installation, temps de jeu,
  abris (réunis fichier par fichier, le plus récent prime), parties à restaurer (aussi sur un autre PC), réglages
  (copiés une fois par réunion, le plus récent prime) suivent ; `fiche-avant-reunion-N.json` gardée. Un jeu non
  rattaché garde sa fiche et son « Jouer ». Une fiche réunie découverte par un 410 (fiche, file de téléchargement) est
  retenue et l'ancien numéro sort du catalogue gardé ; la file reprend sous le nouveau numéro. Une liste complète de
  fusions remplace l'ancienne ; un ancien numéro rendu à une fiche vivante reste le sien. Deux revues (contrat,
  lancement) puis une relecture des corrections (contrat) : 2 bloquants et 9 importants corrigés. Vérifié en réel
  (profil temporaire, jeton de Seb lu dans le coffre) : 15 507 jeux en 201 s, 1927 rattaché à 324 (version -602),
  154 fusions gardées, un jeu douteux (1932 → 1360) garde sa fiche locale. Tests : 340 cœur, 169 interface. NON
  vérifié en réel : la file face à un vrai 410 (aucun téléchargement lancé). Restes notés dans IDEES ; besoin 23.

- **0.64.1** (18/10) — Correctif URGENT (relevé par l'expert contrat avant le lot 1.14) : un refus en pleine reprise
  (404, 409, 410, 503…) effaçait le `.part` déjà reçu ; seul un 416 (`Erreur::Plage`) l'autorise désormais
  (téléchargements des jeux, fichiers servis, restauration). 410 traité comme un 404 avec le texte de Firehouse en
  attendant le rattachement. Vérifié : test de non-régression (410/404/503 : `.part` intact, une requête) ; en lecture
  seule, aucun des 9 jeux installés de Seb n'a sa fiche réunie (ses sauvegardes ne sont pas touchées par le défaut
  « jeu réuni = parties plus sauvegardées »). Tests : 328 cœur, 165 interface ; relecture de l'expert contrat.

- **0.64.0** (18/10) — Lot Médias M2 : galerie de la fiche (vignettes en cellules fixes, visionneuse à la manette et au
  clavier, libellés et régions en français, damier, gros pixels des petites images agrandies ×2). Cœur : `medias.rs`
  (liste gardée PAR PROFIL relue quand `medias_empreinte` change ; image précise par son id, gardée sous son empreinte,
  écrite sous un nom provisoire unique puis renommée ; images retirées purgées ; 404 = tout ce qui est gardé du jeu
  pour ce profil part ; réponse mal formée sans effet). Vérifié en RÉEL : `?id=` sert l'image précise (id inconnu ou
  sous un autre type → 404, consigné dans EXEMPLES) ; profil temporaire : 21 visuels listés, 2e capture 21 696 o
  gardée. Tests : 327 cœur, 165 interface ; avis AVANT et relectures des experts (importants corrigés). NON vérifié :
  la galerie à l'écran.

- **0.63.0** (18/10) — Lot Médias M1 : choix de l'image des jeux (grille + Taodbox, réglage du profil
  `ludotheque.visuel`), cadre du visuel choisi (grille régulière) et boîte au ratio réel (place réservée), repli sur
  la jaquette dans le cœur, gros pixels gardés. Cœur : catalogue schéma 2 (`images`, `medias_empreinte`, `documents`,
  `documents_empreinte` lus de façon TOLÉRANTE, relecture complète unique, marqueur posé seulement par une relecture
  réussie, page coupée retentée une fois) ; visuels rangés par jeu sous l'empreinte, écrits puis renommés, toutes
  largeurs gardées, anciennes versions et visuels retirés effacés, sans durée (sinon 7 jours). Vérifié en RÉEL (profil
  d'essai temporaire, catalogue de Seb non touché) : 15 507 jeux relus en 190–199 s ; 3x3 Eyes : 11 types avec
  dimensions, boîte 3D 64 Ko et cartouche 14 Ko gardées. Tests : 325 cœur, 162 interface ; avis AVANT des experts
  contrat et interface, puis relectures (1 bloquant : une largeur effaçait l'autre ; importants corrigés). NON
  vérifié : la grille à l'écran.

- **0.62.0** (18/10) — Fin du lot Contrat 1.13 (sauf (8), attendu de Firehouse) : conseil changé → même émulateur :
  nouveau réglage proposé (BIOS posés puis cœur installé AVANT d'écrire la ligne, options de Frogtend gardées) ;
  autre émulateur : proposé, l'actuel reste ; toute réponse retient la date ; jamais plus de 3 s d'attente, jamais un
  lancement empêché. « 📀 Préparer DOSBox » dans l'import DOS (repris, adopté ou installé ; défaut inchangé s'il y en a
  un). Besoin 22 « fusionné dans N » envoyé à Firehouse (reçu). Vérifié : pc.json de Seb en lecture seule (RPCS3 réglé
  à la main : rien proposé). Tests : 157 interface ; relecture de l'expert lancement (4 importants corrigés). NON
  vérifié : une vraie proposition (aucun conseil appliqué chez Seb), le bouton DOSBox à l'écran.

- **0.61.1** (16/10) — Décision de Seb (garder RetroArch tel quel, prévenir) : avertissement UNE fois par PC avant le
  premier jeu RetroArch (fenêtre maison, réglage `avertiPareFeuRetroArch`), y compris pour la télé (Taodbox) ; le
  réglage inexistant `network_cmd_bind_address` retiré et le commentaire du moteur corrigé (leçon). Vérifié : code de
  RetroArch 1.22.2 (command.c:236-261) ; essais réels du 16/10 (réseau : question de Windows et commandes qui marchent
  sans y répondre ; entrée standard : absente du RetroArch Windows). Tests : 322 cœur, 152 interface ; relecture de
  l'expert interface. NON vérifié : un PC neuf, et « Annuler » cliqué pour de vrai.

- **0.61.0** (16/10) — Lot OSD (i) : « ⚙ Options du jeu » pour RetroArch (compteur de fluidité, avance rapide rappelée
  tant qu'elle a été demandée ici, photo de l'écran, son) ; fenêtre du menu 560×700 ; le retour sélectionne l'entrée
  d'où l'on vient. Vérifié en RÉEL : RetroArch 1.22.2 officiel (décompressé par Frogtend, dossier d'essai supprimé)
  affiche le compteur (« FPS: 143.97 ») puis l'icône muet après les commandes, photos de sa fenêtre regardées ; le
  pare-feu Windows a demandé une autorisation (non donnée), noté ci-dessus. Tests : 322 cœur, 151 interface ;
  relecture de l'expert interface (2 importants + mineurs corrigés). NON vérifié : avance rapide et photo en vrai.

- **0.60.0** (16/10) — Lot OSD étape 4 : SDL3 3.4.18 compilé dans Frogtend (sdl3-sys, aucune DLL ; CMake des outils
  Visual Studio indiqué dans `src-tauri/.cargo/config.toml`, rien installé). Un fil SDL unique, endormi hors partie
  (manettes alors refermées) ; évènements coupés et file vidée ; manettes `XInput#` (Xbox, Sunshine, Steam) laissées à
  XInput ; boutons par position ; voyants PS5/Switch éteints plutôt que numérotés ; DirectInput COUPÉ (SDL y prend la
  manette en exclusif : une manette purement DirectInput n'ouvre pas le menu — dit à Seb). Vérifié : SDL démarre sans
  erreur sur Venkman (aucune manette allumée à l'essai). Tests : 322 cœur ; avis AVANT de l'expert lancement, puis
  relecture (1 bloquant : file d'évènements qui se remplissait ; 3 importants ; corrigés). NON vérifié : une vraie
  manette PlayStation/Switch, dans l'application lancée, avec un émulateur qui lit aussi la manette.

- **0.59.0** (16/10) — Adaptateurs du menu en jeu (h) : Eden et Azahar en pause par leur `pauseWhenInBackground`
  (seulement portables ; un « non » de la personne est gardé) ; Cemu, Xenia, xemu, Vita3K FIGÉS d'office (décision de
  Seb), ainsi qu'Eden/Azahar dont la pause n'est pas réglée ; le gel ne vise que les programmes de LA partie dans le
  dossier de l'émulateur ; « en pause » dit seulement une fois vraiment figé. Vérifié : code source des six émulateurs
  (recherche du 16/10) ; en RÉEL, Azahar 2126.2 et Eden v0.2.1 portables relisent et gardent le réglage posé par
  Frogtend. Tests : 320 cœur ; relecture de l'expert lancement (2 importants + mineurs corrigés). NON vérifié : le gel
  réel de Cemu/Xenia/xemu/Vita3K (aucun installé chez Seb), le son et l'affichage du menu par-dessus leur plein
  écran. Azahar non portable : plus de NAND/carte SD par profil (il n'est plus basculé en portable sans le dire).

- **0.58.0** (16/10) — MAME en pause au menu en jeu (fin du lot OSD : MAME). MAME 0.289 n'a pas de pause à la perte du
  premier plan (mametesters 7339) : script Lua officiel (`-autoboot_script frogtend\menu-en-jeu.lua`, dans le dossier
  de MAME) qui se met en pause tant qu'un témoin existe (créé à l'ouverture du menu, retiré à la fermeture et à la
  sortie de Frogtend) ; ne défait jamais une pause de la personne ; jamais à la place de son propre script ; sinon le
  menu ne promet aucune pause. Vérifié dans les SOURCES de MAME 0.289 (machine.cpp:414-420, video.cpp:253,
  luaengine.cpp:917-928) et en RÉEL : Pong dans MAME 0.289 extrait par Frogtend → « pause » puis « reprise », sortie 0.
  Tests : 318 cœur ; relecture de l'expert lancement (3 importants corrigés). NON vérifié : MAME en plein écran avec
  le menu par-dessus. Incident : disque D: plein (cache de compilation 65,8 Go) → vidé, 59,7 Go libres (leçon).

- **0.57.1** (16/10) — Décision de Seb : musique et vidéo coupées pendant le jeu (menu en jeu : seulement jeu en
  pause ; livret toujours). Tests : 151 interface ; relecture de l'expert interface (message « Aucun document » faux
  corrigé). NON vérifié : dans la vraie fenêtre.

- **0.57.0** (16/10) — La musique et les vidéos du jeu (contrat 1.7, D2). Vérifié en RÉEL : musique 6482 (MP3 ID3,
  6 110 240 o) et vidéo 6842 (MP4 H.264 + AAC, index au début, 5 873 996 o) de Zelda téléchargées par le code de
  Frogtend, sha256 et types vérifiés, lues par morceaux jusqu'au dernier octet. Réponses du protocole testées (206 +
  Content-Length + Content-Range, 200, 416, 404). Tests : 317 cœur, 150 interface ; relectures des experts (bloquant
  corrigé : la lecture continuait quand le cœur cachait le menu en jeu). NON vérifié : le son et l'image dans la vraie
  fenêtre (WebView2), la lecture automatique, la manette.

- **0.56.0** (16/10) — Le livret du jeu (contrat 1.7, D1). Vérifié en RÉEL : `/jeu/111/documents` → 4 documents ;
  livret de Zelda téléchargé par le code de Frogtend (8 583 117 o en 1,9 s, application/pdf et sha256 vérifiés),
  23 pages ; pages rendues par Windows.Data.Pdf en 0,3 s, image regardée (nette ; doubles pages à l'italienne →
  lecteur adapté). Tests : 314 cœur, 149 interface ; relectures des experts contrat et interface (points importants
  corrigés). NON vérifié : le lecteur dans la vraie fenêtre, à la manette, et dans le menu en jeu.

- **0.55.0** (16/10) — L'arcade MAME depuis Firehouse. Vérifié en RÉEL : paquet officiel MAME 0.289 (sha256 conforme à
  Firehouse) décompressé sans exécution (2 332 fichiers, `mame -version` → 0.289) ; fichiers de Street Fighter III
  (sfiii.zip + sfiii/cap-sf3-3.chd) téléchargés de Firehouse par le code de Frogtend ; `mame -verifyroms sfiii` →
  « romset sfiii is good » ; `-rompath "<dossier avec espace>;roms"` relu par `mame -showconfig`. Tests : 310 cœur,
  142 interface ; relecture de l'expert lancement (3 points importants corrigés : rompath de mame.ini gardé, entrée
  7z sans nom acceptée seulement vide, version attendue dite). Non vérifié : lancer un clone (sfiiia) en vrai, et le
  menu en jeu par-dessus MAME.

- **0.54.0** (16/10) — Manette → clavier pour les jeux PC (préréglages + bouton par bouton). Vérifié : essai RÉEL sur
  une fenêtre de test avec l'AZERTY de Seb — lettres Z Q S D et positions « Déplacement » → « zqsdzqsd » ; tests du
  moteur (appui/relâche, deux boutons une touche, gâchettes, combinaison du menu en deux temps et relâchée dans le
  désordre, chaque touche a un code) ; relectures des experts lancement et interface (1 bloquant + 7 importants
  corrigés). Tests : 305 cœur, 140 interface. PAS vérifié : un vrai jeu (Rock Force), un jeu DirectInput/SDL.

- **0.53.0** (16/10) — Plein écran au lancement des jeux PC (par jeu : touche du jeu suggérée par son mode d'emploi,
  ou fenêtre agrandie sans cadre). Vérifié : suggestion RÉELLE sur les 10 jeux Windows de Seb (Rock Force → F4,
  Minecraft → F11, 8 autres → rien, jamais deviné) ; essai RÉEL sur des fenêtres de test (640×480 agrandie à
  1920×1080 ; fenêtre déjà plein écran + touche F11 → « rien ») — il a attrapé un défaut (fenêtre maximisée
  restaurée avant la vérification), corrigé ; 3 relectures d'experts (lancement ×2, interface). Tests : 300 cœur,
  134 interface. PAS vérifié : un vrai jeu (Rock Force F4, un jeu Unity), écran à 125/150 %.

- **0.52.0** (16/10) — Micrologiciel PS3 servi par Firehouse. Vérifié : essai RÉEL complet sur une COPIE de RPCS3
  (hors dossier temporaire : RPCS3 refuse de tourner depuis Temp) — PUP servi 206 197 916 o téléchargé et vérifié,
  4.91 → 4.93 en 2,8 s, réinstallation par-dessus reconnue, rien ne traîne ; tests (firmware servi, empreinte fausse,
  version dans la note, pourcentage) ; relecture de l'expert lancement (aucun bloquant ; échec silencieux, RPCS3
  ouvert, attente sans fin corrigés). Tests : 295 cœur, 131 interface.

- **0.51.0** (16/10) — BIOS servis par Firehouse posés tout seuls (contrat 1.9–1.11). Vérifié : essai RÉEL (BIOS PS2
  conseillé 4 194 304 o, SHA-1 conforme, posé dans un PCSX2 de brouillon, PCSX2.ini réglé en une écriture,
  facultatifs ignorés) ; tests (réponse PS2 réelle, élément illisible, 404 = à poser soi-même, empreinte fausse,
  archive sans rien remplacer, chemins piégés, PCSX2 sans défaire un choix) ; relectures des experts lancement et
  contrat (aucun bloquant ; 8 points corrigés). Tests : 293 cœur, 129 interface.

- **0.50.0** (16/10) — Cœurs RetroArch servis par Firehouse, vérifiés (contrat 1.12). Vérifié : essai RÉEL (55 cœurs
  servis ; Snes9x posé dans un RetroArch de brouillon, 2 948 608 octets, en-tête MZ, aucun reste) ; tests (zip ou dll
  abîmés refusés sans rien poser, cœur présent jamais remplacé, cœur non servi refusé, nom piégé refusé, reprise 416
  sur un `.part` d'une autre version) ; relecture de l'expert lancement (aucun bloquant, 2 importants corrigés).
  Tests : 285 cœur, 129 interface.

- **0.49.0** (16/10) — « Prêt à jouer » suit le conseil de la maison (contrat 1.8) ; `%rom%` placé au lancement.
  Vérifié : conseils RÉELS relevés chez Firehouse pour 7 plateformes (EXEMPLES) et repris dans les tests (SNES, PS1,
  MS-DOS absent de la liste, Switch, PS2) ; relectures des experts contrat et lancement (1 bloquant : le conseil perdu
  quand son émulateur manque à la liste — corrigé) ; tests (282 cœur, 129 interface). PAS vérifié : une vraie
  préparation de console de bout en bout (à essayer par Seb).

- **0.48.0** (15/10) — Fond du menu en jeu : visuel du jeu (fond, capture, écran titre), sinon le skin ; même carte
  pour tous. Vérifié : forme réelle des visuels relevée chez Firehouse (jeu 116 : 11 types ; fond 118 776 octets en
  WebP 1 000 px) ; tests du cœur (premier visuel trouvé, cache par profil, 503 qui passe au suivant, hors ligne,
  autre profil, chemin piégé jamais demandé) ; relectures des experts contrat et interface (aucun bloquant, points
  corrigés). PAS vérifié : le rendu à l'écran (aucun jeu de Seb n'a de visuel aujourd'hui).

- **0.47.0** (05/10) — « ❄ Figer le jeu pendant le menu » (jeux PC, par jeu, désactivé d'office, refusé avec un
  anti-triche ou un dossier trop grand pour le savoir). Vérifié : gel réel d'un vrai programme (64 et 32 bits),
  programme déjà suspendu par son créateur qui le reste, relance après un « arrêt brutal » simulé par la note ;
  recherche d'anti-triche sur les 10 jeux Windows de Seb (aucun, 3 s au plus pour Minecraft) ; relectures des
  experts lancement (2) et interface ; tests (279 cœur, 121 interface). PAS vérifié : un vrai jeu figé (son, fenêtre
  « ne répond pas »), à l'essai avec Seb.

- **0.46.3** (05/10) — Le menu met RPCS3 en pause et lui coupe la manette (ses réglages « Pause emulation on RPCS3
  focus loss » et « Background input enabled », aussi dans ses réglages par jeu) ; un seul menu : les menus manette de
  DuckStation/PCSX2/RetroArch retirés ; un INI illisible n'est plus jamais remplacé. Vérifié : clés et comportement
  relevés dans le code de RPCS3 (relu par l'expert lancement, pas de bloquant) ; essai sur une COPIE du vrai
  config.yml de Seb (2 lignes changées sur 282, même taille) ; tests (274 cœur, 120 interface). PAS vérifié : l'essai
  en jeu (Seb) ; DuckStation/PCSX2/RetroArch pas installés chez Seb.

- **0.46.2** (05/10) — RPCS3 sans boîtes invisibles (accueil, « Exit Game? ») ; menu : seul le bouton choisi est
  coloré. Retours de Seb (05/10) : « le surlignage des boutons reste derrière les boutons et Reprendre reste en orange »
  ; erreur fatale de RPCS3 au 2e Quitter. Vérifié : cause relevée dans le code de RPCS3 et dans le réglage de Seb
  (`infoBoxEnabledWelcome=true`) ; essai sur une COPIE du vrai fichier (1 ligne changée sur 55) ; tests (270 cœur,
  120 interface). PAS vérifié : l'essai en jeu (Seb).

- **0.46.1** (05/10) — « Quitter » : un émulateur resté en vie sans fenêtre (RPCS3) est arrêté après 10 s, seulement
  dans le dossier de l'émulateur de la partie, jamais un jeu PC ni un émulateur qui a encore sa fenêtre. Vérifié :
  RPCS3 bloqué chez Seb (PID 64208, « ne répond pas », titre « Welcome to RPCS3 ») arrêté à la main par l'agent
  (Seb avait choisi Quitter) ; 2 tests avec de vrais programmes ; deux relectures de l'expert lancement (la 1re a
  bloqué une version dangereuse). PAS vérifié : l'essai en jeu (Seb).

- **0.46.0** (05/10) — OSD étape 3 : le cœur lit la manette (XInput) pendant la partie ; Select + R1 tenus 1 s ouvrent
  le menu (réglable) ; croix/stick, A, B pilotent le menu (A/B au relâchement) ; « Annuler » d'abord pour une action
  dangereuse ; sélection rendue après un retour et visible sans premier plan. Vérifié : tests (267 cœur, 120
  interface), lecture réelle de la manette de Seb, relecture des experts lancement et interface (aucun bloquant).
  PAS vérifié : l'essai en jeu (Seb), la prise du premier plan depuis la manette (notée au journal si elle échoue),
  les manettes non XInput (étape 4, SDL).
- **0.45.0** (05/10) — OSD étape 2 : ✅ clavier et souris dans le menu (Seb 05/10) ; manette dans le menu : NON (Gamepad
  API muette) → 0.46.0.

- **0.45.0** (05/10) — OSD étape 2 : menu en jeu à la manette, au clavier et à la souris ; Taodbox en plein écran ×2.
  Vérifié : relecture de l'expert interface (1 bloquant corrigé : B dans « Quitter ? » reprenait le jeu — rôle
  `alertdialog` ignoré, aussi en Taodbox ; 3 importants corrigés) ; test qui reconnaît la confirmation ; 261 + 119
  tests. PAS vérifié : l'essai réel du menu (Seb) ; la manette n'agit que dans la fenêtre au premier plan (à essayer).

- **0.44.4** (04/10) — RPCS3 : valeurs du fichier de manette entre guillemets (« #1 » était lu comme un commentaire).
  Vérifié : le journal de RPCS3 chez Seb montrait la cause exacte ; un test relit le fichier avec les règles YAML (il
  aurait attrapé le bug) ; le code de RPCS3 confirme la reconnexion d'une manette éteinte au lancement. PAS vérifié :
  l'essai en jeu (Seb).
- **0.44.3** — ⚠ CASSAIT la manette ET le clavier dans RPCS3 (voir 0.44.4).

- **0.44.3** (04/10) — RPCS3 : n'importe quelle manette (Xbox/compatibles, PS4, PS5), même réveillée après le
  lancement ; clavier seulement sur choix. Principe rappelé par Seb (04/10) : « n'importe quelle manette doit être
  automatiquement configurée et reconnue par Frogtend et ses émulateurs ». Vérifié : noms et reconnexion relevés dans le
  code de RPCS3 ; tests ; détection réelle chez Seb (XInput n° 0, 31 périphériques HID lus). PAS vérifié : le jeu avec
  la manette réveillée après le lancement (essai de Seb) ; une manette PlayStation (aucune branchée).

- **0.44.2** (04/10) — manette RPCS3 branchée sur chaque partie (bug de la 0.44.0) ; traductions de fan.
  Vérifié : relecture par l'expert lancement (chemin `jeu_jouer` → `preparer` → `regler_rpcs3`, fichier de la personne
  jamais défait) ; tests (259 cœur, 117 interface) ; manette de Seb détectée en vrai (XInput n° 0). PAS vérifié : que
  RPCS3 accepte le fichier écrit (à essayer par Seb) ; aucun fichier « [T-Fr …] » chez Seb pour un essai réel.

- **0.44.1** (03/10) — contrat 1.6 : jamais la jaquette d'origine (1 à 13 Mo), au plus 1 000 px ; essai RÉEL sur
  Firehouse (jeu 115 : WebP 56 Ko au lieu de 605 Ko, 19 médias listés).
- **0.44.0** (03/10) — Prêt à jouer ; RPCS3 manette XInput sinon clavier ; micrologiciel `--headless --installfw`.
  Vérifié : installation silencieuse RÉELLE sur une copie neuve de RPCS3 (3,4 s, aucune fenêtre) ; version 4.91 lue
  sur le RPCS3 de Seb ; détection de manette (aucune branchée). PAS vérifié : le cas manette branchée, l'enchaînement
  Prêt à jouer en vrai (simulé par tests).
- **0.43.4** — toutes les plateformes proposées pour régler un émulateur ; vérifié par Seb (RPCS3 installé).
- **0.43.3** — import sans extension à taper, « Ajouter » ne lit plus 196 Go ; répétition RÉELLE sur le dossier PS3
  (1,2 s, 7 jeux, 70 contenus écartés) ; vérifié par Seb (7 jeux importés).
- **0.43.2** — motifs d'erreur visibles, barre du haut, copie de côté à la désinstallation ; vérifié par Seb.
- **0.43.1** — 26 corrections de l'audit (sécurité et relecture).
- **0.43.0** — décompresser pour jouer ; ✅ ESSAI DÉCISIF RÉUSSI par Seb le 03/10 : Asura's Wrath démarre dans
  RPCS3 depuis l'.iso, 60 i/s.
- Avant : voir [NOTES-DE-VERSION.md](NOTES-DE-VERSION.md).
