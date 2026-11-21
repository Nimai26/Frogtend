# Idées notées (à ne pas démarrer avant la fin du lot en cours)

- **Taodbox : refaire toute l'interface pour la version définitive** (Seb, 19/11 : « c'est un prototype l'ui de
  Taodbox, il faudra complètement la revoir en version définitive pour la rendre plus belle »). En attendant (0.212.1),
  seule la taille est corrigée : jaquettes d'après l'écran, réglage « Taille de Taodbox ».
  **La direction voulue par Seb** (19/11, deux captures d'un thème de Big Box en exemple, à s'en INSPIRER sans rien
  recopier : ni images, ni logos, ni mise en page à l'identique) :
  - **choix du système** : une roue VERTICALE de logos de systèmes (à gauche), le système choisi agrandi au centre de la
    roue, flèches haut/bas ; derrière, un **fond propre au système** (la console, ses manettes, une vieille télé) et son
    année de sortie ;
  - **choix du jeu** : une roue verticale des LOGOS des jeux (à droite), la fiche courte du jeu choisi (genres, mode de
    jeu, état), sa jaquette, son résumé ; le fond change avec le système ;
  - **des vidéos qui jouent DANS les écrans du décor** : la vidéo du système ou du jeu est placée dans la télé dessinée
    sur le fond, pour qu'on ait l'impression que le jeu tourne dans cette télé. Il faut donc, pour chaque fond, la
    position de son « écran » (un rectangle, voire un coin incliné) ;
  - à prévoir : d'où viennent les fonds de systèmes, les vidéos de systèmes et de jeux et les logos (Firehouse :
    médias EmuMovies/LaunchBox, contrat 1.6 — vérifier ce qui est déjà servi, sinon une demande dans BESOINS-API) ;
    le poids des vidéos (cache, mode hors ligne) ; la fluidité à la manette avec des milliers de jeux ; la charte de
    Firehouse et ses skins (aucune couleur en dur) ; tout réglable au canapé.
- **Epic : ouvrir directement la fenêtre d'installation** (essai de Seb, 19/11) : `apps/<AppName>?action=install`
  ouvre la bibliothèque d'Epic filtrée sur le jeu (un clic de plus sur « Installer ») ; essayer la forme longue
  `apps/<namespace>%3A<item>%3A<app>?action=install` (celle du lancement), à vérifier avec Seb.
- **Arrêt d'une partie : reconnaître les programmes au dix-millionième de seconde** (expert lancement, 16/11) :
  `processus_de_la_partie` / `descendants` (lancement.rs), qui choisissent ce que « Quitter » et l'arrêt de sécurité
  d'Eden/Eglantine vont TERMINER, comparent encore l'heure de démarrage à la seconde (un numéro repris dans la même
  seconde serait pris pour le jeu). `Suivi` est passé aux heures précises en 0.203.0 : faire de même ici.
- **Micrologiciel PS3 : proposé comme une mise à jour** (Seb, 16/11) : le bouton « 💿 Micrologiciel PS3… » toujours
  affiché prête à confusion (on croit à une mise à jour en attente). Le remplacer par la même logique que les mises à
  jour d'émulateurs : au démarrage, si Firehouse sert une version plus récente que celle de RPCS3, le proposer (taille
  dite) ; sinon un simple état « ✅ à jour (version …) ». Même idée pour le firmware Switch d'Eden/Ryubing et Vita3K.
- **Profils de manette par jeu** (lot 5, Taodbox) : RetroArch a ses « remaps » par jeu, DuckStation et PCSX2 leurs
  réglages par jeu (`gamesettings\`), Dolphin ses profils (`Config\Profiles\GCPad\`).
- **Dolphin et les manettes PlayStation** : Frogtend règle Dolphin sur XInput ; une manette PS4/PS5 n'y marche que
  par Steam ou DS4Windows. Piste : la source SDL de Dolphin (le nom de l'appareil y dépend du modèle).
- ✅ (fait en 0.76.0, bouton Y) **« ⚙ Gérer le jeu » à la manette en Taodbox** (expert interface, 05/10) : Commandes, Émulateur, Figer pendant le
  menu, Plein écran au lancement, Jouer à la manette (jeu fait pour le clavier)… ne se règlent aujourd'hui qu'au bureau (le réglage s'applique ensuite en Taodbox).
- ✅ (fait en 0.77.0) **Manette → clavier** (expert interface, 16/10) : choisir une touche en l'APPUYANT (« appuie sur la touche
  voulue ») plutôt que dans une liste de 70 ; répétition automatique d'une touche tenue (vieux menus qui ne défilent
  que par répétition).
- ~~Wii dans Dolphin~~ → passé dans le plan (lot 4c, profils Wiimote réglés par Seb).

## Cheat Engine : ce que propose le forum « Extensions » (analysé le 02/10, demande de Seb)
Source : https://forum.cheatengine.org/viewforum.php?f=130 (≈ 230 sujets, scripts Lua de la communauté, dont
beaucoup écrits avec l'aide d'une IA : qualité variable). Mécanisme commun : un script `.lua` posé dans le dossier
`autorun\` de Cheat Engine s'exécute à son démarrage. **Règle** : rien n'est téléchargé du forum par Frogtend ; les
extensions retenues par Seb iraient dans SON zip (Firehouse les distribue), après relecture.
- ✅ *Fait en 0.19.0.* **« Lancer avec Cheat Engine » en un clic** (inspiré de *automatic game launcher* et *Auto-Load Corresponding Cheat
  Table On Attach*) : Frogtend poserait SON propre petit script `autorun\frogtend.lua` qui lit un fichier écrit par
  Frogtend (processus du jeu, table à charger) et s'attache tout seul au jeu. Idéal pour les enfants : pas de menu
  « Ouvrir un processus ».
- **Set Memory Region for Emulator** (v1.5.1, 14/08/2026) : reconnaît PCSX2, Cemu, DOSBox, PPSSPP, DuckStation,
  melonDS, DeSmuME, Mupen64/RMG ; règle seul la zone de recherche sur la mémoire du jeu émulé. Rend Cheat Engine
  utilisable sur nos jeux d'émulateurs.
- **Dosbox base finder** : trouve la base de la mémoire émulée de DOSBox (toutes versions) → Cheat Engine sur les jeux
  MS-DOS (Dune !).
- **Serveurs MCP pour Cheat Engine** (*Native Cheat Engine MCP server*, *Simple MCP Server*) : un assistant IA peut
  piloter Cheat Engine. Piste pour l'assistant de Firehouse (actions « cheat » du lot 7), à étudier avec prudence
  (serveur local, droits).
- **Confort** : *Edit CE all forms font size* (texte plus grand : Taodbox/télé), *Compact Mode*, *Table Hotkeys
  Manager* (raccourcis des tables).
- Peu utile pour nous : outils de développeurs (dumpers Unreal/Godot/IL2CPP, assembleur, structures).

- ✅ (fait en 0.79.0) (16/10, expert lancement) MAME : sauver / charger / recommencer depuis le menu en jeu par le même script Lua (un
  fichier de commande lu par le script : `manager.machine:soft_reset()`, `:save(nom)`, `:load(nom)` — à vérifier dans
  luaengine.cpp), plus sûr que des touches (MAME demande un emplacement après Maj+F7 / F7).
- (16/10, expert lancement) MAME : témoin « vivant » (réécrit par Frogtend toutes les quelques secondes ; le script
  relance le jeu s'il cesse de changer) pour qu'un plantage de Frogtend menu ouvert ne laisse pas le jeu en pause.
- (16/10) Vider tout seul `target\debug` quand il dépasse 30 Go (outil de développement, pas Frogtend lui-même).
- (16/10, expert lancement) Manettes Nintendo : une « disposition Nintendo » (A à droite valide) par
  `SDL_GetGamepadButtonLabel`, si Seb la veut ; aujourd'hui le bouton du BAS valide partout (comme Taodbox).
- (18/10, experts, lot 1.14) Petits restes du rattachement : un exemple de fiche réunie dans le mode simulé ; retenir
  un rattachement refusé (cas douteux) pour ne pas relire la nouvelle fiche ni réécrire le journal à chaque
  synchronisation ; un jeu rattaché garde sa fiche locale d'avant (une annexe absente du PC, demandée avec l'ancienne
  `cle`, répond 409 : relire alors la fiche) ; les documents et visuels gardés sous l'ancien numéro restent orphelins
  (à purger avec le lot Médias M4).
- (18/10, expert contrat, lot M4) Petits restes : une jaquette notée « absente » à une taille reste invisible hors ligne
  7 jours même gardée à une autre ; le ménage lit le disque dans une tâche asynchrone (à passer en `spawn_blocking`
  si des milliers d'images) ; le préchargement des fonds de Taodbox (au plus 50 jeux, ~100 Ko chacun) se fait sans
  annonce, suivant le réglage « Fonds et logos » : à confirmer avec Seb pour une connexion limitée (chez sa sœur).
- (18/10, expert contrat, 0.67.1) En mode simulé, les jeux Steam/GOG importés du vrai profil n'apparaissent pas, et
  un import fait en simulé reste dans le cache simulé : le dire dans l'interface du mode simulé, ou refuser l'import.
- (19/10, expert lancement, contrat 1.16) ScummVM n'est pas branché : un jeu ScummVM se lance par son exécutable
  (prêt à jouer) ; s'il passait un jour par un émulateur, `%rom%` doit être le DOSSIER du jeu (`-p "%rom%"`), pas le
  fichier — à faire avec un test avant de promettre ScummVM.
- (0.75.0 : le « prévenir » est fait ; RESTE la conversion `xdvdfs pack`, avec accord et taille annoncée) (19/10, Firehouse) xemu exige une image XISO : prévenir la personne AVANT de lancer un .iso Redump d'un jeu Xbox (la
  `note` du conseil Xbox le dit) ; proposer plus tard une conversion (`xdvdfs pack`) avec son accord. Aucun jeu Xbox au
  stock au 19/10.
- (19/10, expert lancement) La dépendance JavaScript `@tauri-apps/plugin-dialog` n'est plus importée (les fenêtres de
  choix passent par le cœur depuis 0.72.0) : la retirer de package.json au prochain renouvellement des dépendances.
- (19/10, expert lancement, durcissement) Restes mineurs : filtrer les commandes par fenêtre (déclarer les ~130
  commandes dans `build.rs` avec `AppManifest::commands`, puis une capacité `menu-jeu` restreinte — gain faible, la
  fenêtre charge le même code) ; annoncer aussi un lecteur réseau MONTÉ (`Z:`), pas seulement un chemin `\…`, avant
  une recherche d'import (une petite commande « ce disque est-il sur le réseau ? »).
- ✅ (fait en 0.74.1) (19/10, expert lancement, 0.74.0) « 🎨 Image et fluidité » : lire `Video ▸ Disable On-Disk Shader Cache` et avertir
  s'il vaut `true` (« Sans saccades » ne tiendrait plus d'une partie à l'autre), sans y toucher.
- (21/10, Seb, lot C) Une alerte « 🏆 Succès obtenu ! » de Frogtend pendant la partie (Steam, Galaxy, RetroArch
  l'affichent déjà chacun à leur façon) : plus tard, dans un lot à part.
- (03/11, essai de Seb) « Retirer du PC » refusé tant que le dossier des abris de parties n'est pas réglé : pour un
  novice, proposer à ce moment-là un dossier tout prêt (« E:\Abris de parties » sur le disque des jeux) en un clic,
  au lieu de l'envoyer dans Réglages ▸ Emplacements.
- (04/11, Seb + avis lancement) Déplacer un émulateur ou un outil déjà installé vers le nouveau dossier des émulateurs,
  émulateur par émulateur, accord chiffré : renommage sur le même disque, sinon copie vérifiée (contenu) qui recrée les
  dossiers vides et ne suit pas les jonctions ; puis réécrire emulateurs.json, pc.json ▸ emulateurs, les notes
  (langues-posees, rpcs3-compile, eden-contenus), xemu.toml, Eden nand_directory, la jonction PPSSPP ; refus si ouvert,
  partie en cours, Cheat Engine suivi, installation en cours.
- (04/11, relecture lancement) Un abri déplacé à moitié (un fichier en échec) existe dans deux dossiers à la même date :
  la sauvegarde vers Firehouse (sauvegarde.rs:520-529) n'en garde qu'un ; réunir les dossiers de même date.
- (05/11, Seb : vidéo « Tuto Emulation Switch 2 » https://www.youtube.com/watch?v=UXpQFE8CvYo) Émulation Switch 2 : à ce
  jour (relevé 05/11) un fork EXPÉRIMENTAL d'Eden, « Eden-NX2 » (apparu le 29/10, Mario Kart World et DK Bananza
  démarrent mais saccadent ; d'autres forks : Eglantine, nxnative) ; disponibilité contradictoire (sorti / archivé
  après 3 jours), risque de retrait par Nintendo. Frogtend connaît déjà Eden (profils, firmware, contenus) : l'intégrer
  serait proche, MAIS seulement quand (1) une version stable existe à une source officielle ou servie par Firehouse,
  (2) Firehouse sert une plateforme Switch 2 et ses jeux (les jeux ne viennent que de Firehouse). À revoir avec Seb.
  Relevé 05/11 (lovegame.space, mis à jour le 03/11) : l'émulation Switch 2 n'utilise PAS de prod.keys ; eden-nx2 /
  Eglantine chargent un jeu DÉJÀ déchiffré sur une Switch 2 (firmware 23.0.1 : exefs/main décompressé + romfs/) ; pas de
  support dans Eden officiel (PR en brouillon au 01/11) ; 16 Go de RAM, GPU Vulkan 8 Go. Les clés de Seb
  (Bureau\switch 2, vérifiées sans lire les valeurs) ont la forme d'un dump Switch 1 par Lockpick_RCM (mariko/tsec/keyblob,
  master_key 00-16, 170 title.keys) : utiles pour Eden (Switch 1), pas pour la Switch 2.
- (08/11, Firehouse contrat 1.39, Firehouse 3.126.0) `/plateformes` gagne `marque` (Nintendo, Sega, Sony, PC, Arcade,
  MSX…) et `type_machine` (salon | portable | ordinateur | arcade | mobile) ; ordre conseillé : marque, type,
  `date_sortie` ; `fabricant` (souvent l'usine : Switch 2 et Wii sous « Foxconn ») ne doit plus servir à ranger ;
  corrigeables par un admin. Frogtend range aujourd'hui avec des listes en dur (`categories.ts` : Arcade / Consoles /
  Portables / Ordinateurs) et montre `fabricant` dans la fiche système (`systemes.ts:29`) → utiliser `type_machine`
  pour les catégories (listes en dur en repli) et `marque` + date pour l'ordre et la fiche. À proposer à Seb.

- **Suivi d'une partie plus sûr** (08/11, après un test rouge sous charge) : reconnaître un processus par son heure de
  création EXACTE (GetProcessTimes, au 100 ns) plutôt qu'à la seconde, et/ou rattacher la partie à un « job » Windows
  (ses enfants y entrent seuls) : plus jamais un programme étranger adopté par un numéro repris.

- **Émulateur figé après « Quitter »** (08/11, Eglantine daily-20261107 chez Seb ; avis de l'expert lancement) : si
  sa fenêtre « ne répond plus » (IsHungAppWindow, déjà utilisé dans `plein_ecran.rs`) bien après le délai de
  fermeture, une fenêtre maison pilotable à la manette : « Eglantine ne répond plus — Attendre / Forcer la fermeture »
  (Attendre par défaut ; dire que les sauvegardes déjà écrites restent). Avant de forcer : mettre à l'abri le dossier
  des parties de l'émulateur ; ne terminer que les programmes de son dossier. À décider après l'essai de la rc3
  (besoin 44), qui corrige la fermeture.

- **Montrer l'émulateur « alternatif » sur la fiche du jeu** (08/11, contrat 1.40 : le conseil Switch 2 a Lichii en
  alternative, « si un jeu ne passe pas avec Eglantine ») : aujourd'hui l'interface ne lit pas `conseil.alternatives` ;
  il faut passer par ⚙ Réglages ▸ Émulateurs puis choisir l'émulateur pour le jeu. Pour un novice : un bouton « Essayer
  avec Lichii » sur la fiche (installation sur accord, choix pour CE jeu).
- **Visual C++ pour Lichii sur un autre PC** (08/11) : le zip de Lichii porte `vc_redist.x64.exe` (Lichii est compilé
  avec MSVC) ; sans le runtime Visual C++ 2015-2022 x64, lichii.exe ne démarre pas. Venkman l'a (14.51). À faire :
  le détecter (registre, lecture) et proposer de l'installer (droits administrateur : à annoncer, accord de la personne).
- **Parties entre Eglantine et Lichii** (08/11, avis lancement) : deux dossiers `user\`, rien de partagé. Proposer une
  copie unique sur accord (jamais par-dessus), après avoir vérifié sur un vrai jeu que le format est le même.
- **Abris des anciennes versions d'émulateurs** (08/11) : chaque mise à jour d'Eden, Eglantine ou Lichii garde
  l'ancienne version (environ 100 à 200 Mo) dans `.frogtend-sauvegardes` ; rien ne fait le ménage. Montrer leur
  taille et proposer de les retirer (accord, jamais seul).
- **Mises à jour et DLC des jeux Switch 2** (08/11) : `contexte_switch` ne les gère que pour Eden (Switch).
- **Vita3K `--frontend sdl`** (10/11, vu dans config.cpp) : une simple fenêtre qui lance le jeu donné par `-r` et
  se ferme avec lui, sans l'interface Qt (ni accueil, ni liste de jeux). À essayer pour les parties lancées par
  Frogtend : plein écran, pause et manette à vérifier d'abord.
- **lichii-updater.exe** (08/11) : à vérifier dans les sources de Lichii s'il se met à jour seul au démarrage (il
  écrirait hors de Frogtend, et la version notée serait fausse).
- BigPEmu : relever l'empreinte des jeux Jaguar CD (cheat_codes.json les couvre, ReadMe l.28) pour proposer leurs triches dans le menu (noté 11/11, 0.187.0).

- **Relancer puis recharger tout seul (15/11) — ✅ Seb 15/11 : « oui quand cela est possible » → PLAN** : après « 💾 Sauver puis relancer », le jeu repart du
  début ; il faut aujourd'hui faire menu ▸ 📂 Charger. Frogtend pourrait charger seul l'emplacement sauvé dès que le jeu
  est prêt (RetroArch, Jaguar, MAME). À décider par Seb.

- **Zelda lent sous Eglantine (Seb, 15/11) — ⏸ Seb 15/11 : « c'est Eglantine qui bug, t'en occupe pas pour le moment, l'émulation Switch 2 n'est pas totalement au point »** : « le jeu est toujours lent avec Eglantine (Zelda) ». À comprendre : lent
  aussi lancé hors de Frogtend ? (sinon, un réglage posé par Frogtend : pause, turbo, compteur…) ; journal d'Eglantine
  (compilation des shaders, `Shader precache build` vu le 15/11), réglages graphiques. Ne rien changer chez Eglantine
  sans l'accord de Seb.

- **Triches Switch pour Ryubing (lot 5, 15/11) — reporté : pas encore de source de triches Switch.** Ryubing lit
  `<racine>\mods\contents\<jeu>\cheats\<BuildId>.txt` (format Atmosphère, entrées `[Nom]`) et `enabled.txt`
  (ModLoader.cs L740-768) ; il écrit le BuildId de l'exécutable dans son journal (L730) et ignore un fichier qui ne
  correspond pas. À faire quand Firehouse servira des triches Switch : poser au prochain lancement, parties à l'abri
  avant (`menu_jeu::mettre_a_l_abri_fichiers`).
- **Changer de profil pendant une partie** (relevé le 17/11, lot « émulateur seul ») : rien ne l'empêche aujourd'hui pendant une partie ordinaire (seul l'émulateur ouvert seul le bloque) ; la sauvegarde de fin de partie irait alors au mauvais profil. À trancher avec Seb.
- **Ouvrir seul les autres émulateurs** (17/11) : RPCS3 (sans `--no-gui`), Azahar, PPSSPP, RetroArch (pas de liste de dossiers) ; plus tard MAME (`-rompath`), BigPEmu, Cemu.
- **Eden ouvert seul** (17/11) : ne remet pas les choix de mises à jour et DLC en place (il faudrait relire les DLC de chaque jeu) ; seul le lancement d'un jeu le fait. Eglantine et Lichii (Switch 2) pas encore branchés sur les choix.
- **DLC liés entre eux** (Seb 17/11, Zelda BotW) : le Pack 2 (119 Ko, la « clé » du Pass d'extension) et le Pack 1 (1,3 Go, les données) ne marchent qu'ensemble : en éteindre un seul coupe tout dans le jeu. Piste : dans la fenêtre de prudence, dire qu'un jeu peut avoir besoin de tous ses DLC ; repérer un tout petit DLC (quelques Ko) comme « clé » probable.
- **ReShade, DLSS, OptiScaler, DXVK par jeu PC — inspiration : RHI** (Seb 17/11 : https://github.com/rankftw/rhi, « ReShade HDR Installer », C# .NET 8, très actif, 2 638 étoiles). ⚠ Licence **GPL-3.0** : ne JAMAIS recopier son code ni son `manifest.json` (Frogtend deviendrait GPL) ; s'en INSPIRER et réécrire. Idées relevées (étude du 17/11, lecture seule) :
  - **Lire l'exe d'un jeu** (sans risque, petit) : 32/64 bits par l'en-tête PE (`0x3C` → champ machine `0x14C`/`0x8664`) ; l'API graphique par les DLL importées (d3d8/9/10/11/12, vulkan-1, opengl32 ; `dxgi` seul → DX12) ; Unity : `*_Data/boot.config` `gfx-device-type`. Utile aussi pour Cheat Engine (bon nombre de bits), trainers, DXVK. Choix de l'exe : chercher aussi `Binaries/Win64` (Unreal), la personne peut corriger.
  - **Détecter les jeux des boutiques** (lot « jeux des boutiques ») : Steam (registre `SteamPath`, `libraryfolders.vdf`, `appmanifest_*.acf`), GOG (`HKLM\SOFTWARE\GOG.com\Games`), EA (`installerdata.xml`), Ubisoft (`Launcher\Installs`), Battle.net/Rockstar (`Uninstall` + éditeur) — lecture seule, jamais de scan de disque.
  - **ReShade par jeu, réversible** : nom de DLL selon l'API (DX10-12 `dxgi.dll`, DX9 `d3d9.dll`, DX8 `d3d8.dll`, OpenGL `opengl32.dll`) ; un fichier inconnu déjà là renommé `.original` (refus si un `.original` existe) ; registre des installations ; désinstallation qui remet tout ; `ReShadePreset.ini` jamais touché. Vulkan = couche GLOBALE en HKLM + droits admin → à écarter ou annoncer (Venkman calcule pour Firehouse).
  - **Anti-triche** : RHI n'a qu'une liste de 8 jeux et avertit seulement → Frogtend devrait REFUSER par défaut s'il voit EasyAntiCheat, BattlEye, vgk… dans le dossier du jeu.
  - **Source** : RHI lit le HTML de reshade.me sans vérifier ni signature ni empreinte → Frogtend : Firehouse le sert (besoin d'API), sinon reshade.me avec signature Authenticode vérifiée, après accord avec la taille.
  - Plus loin : versions DLSS/Streamline par jeu (original gardé), OptiScaler, DXVK, profils du pilote NVIDIA (admin), paquets de shaders et préréglages, données PCGamingWiki (à faire passer par Firehouse).
- **Eden et les .nsz** (17/11) : Eden v0.2.1 ne lit pas les NCZ (`registered_cache.cpp` `ScanDirectory` : nsp, xci) ; sa branche principale les lit déjà (sauf « solid NCZ ») → à la prochaine version publiée d'Eden, revérifier et lui donner les .nsz. Ryubing 1.3.3 ne les lit pas.
- **Exécuter les plans d'installation des mods (contrat 1.45, 18/11)** : prérequis servis, `regler_emulateur` traduit vers des réglages connus (jamais à l'aveugle), `desactiver_mod`, `a_la_main` seul montré ; et la vision citée par Firehouse (« Frogtend fait tout tout seul, comprend les demandes en langage simple », mods, triches, ReShade) — À CONFIRMER avec Seb et à découper en lots après la fin du lot mods Switch.
- **⚠ Partie restée « en cours » (18/11)** : vue chez Seb (Eden quitté par le menu, 0.208.0) et dans la suite de tests (0.209.0, `jouer_suit_la_partie_et_compte_le_temps` : 26 min, non reproduit seul ni à la relance). Piste : un programme sans rapport pris pour un descendant (numéro réutilisé par Windows, fermeture vue avec ~2 s de retard sans poignée). Repères au journal depuis 0.208.1/0.209.0 (« toujours en cours : <programmes> ») : à relire au prochain cas.
- **⚠ Mise à jour 0.210.0 (18/11)** : l'installeur n'a pas pu écrire frogtend.exe (« Erreur lors de l'ouverture du fichier en écriture ») : un Frogtend ouvert à 15 h 13 (fenêtre « Frogtend », répondait) ; « Recommencer » après sa fermeture a marché. Seb : lancée par ⚙ Options (le bouton), Frogtend s'est bien fermé à la validation. Relevé : le greffon de mise à jour (tauri-plugin-updater 2.13.0, updater.rs L938-979) lance l'installeur puis `exit(0)` — le `relaunch()` de mises-a-jour.ts n'est donc jamais atteint sous Windows ; un NOUVEAU frogtend.exe est né à 15:13:30, son parent déjà disparu ; le journal d'Application de Windows montre une session du Gestionnaire de redémarrage (Restart Manager) à 15:13:05. Cause non prouvée (redémarrage par Windows, ou ouverture par la personne). Pistes : un crochet NSIS (`NSIS_HOOK_PREINSTALL`) qui attend la fin de TOUT frogtend.exe du dossier d'installation avant de copier ; refuser un 2e Frogtend (instance unique) pendant une mise à jour. Si la mise à jour en un clic laisse Frogtend ouvert (ou le relance avant la copie), à corriger (règle « mise à jour simple »).
- **Import automatique, suites notées pendant le lot A (18/11)** :
  - **Filtre « Nouveaux »** dans la ludothèque : retrouver les jeux arrivés seuls au dernier démarrage (le toast dit combien, sans lien vers eux).
  - **Un jeu de Firehouse installé AUSSI par Steam** : Firehouse donne `steam_appid` pour certains jeux, mais rien ne relie aujourd'hui la carte Firehouse et la copie Steam du PC (deux cartes) ; même idée pour GOG et Epic.
  - **Installer un lanceur absent** (Ubisoft Connect, Amazon Games, Epic, Battle.net…) : aujourd'hui Frogtend dit « Ce jeu a besoin du lanceur X, absent de ce PC » et ne télécharge RIEN ; plus tard, le proposer (source officielle, taille annoncée, accord de la personne).
  - **Battle.net : bouton Jouer** : aucune adresse ni ligne de commande prouvée par une source lue (`--exec="launch <code>"`, `battlenet://<code>` circulent, non vérifiés) ; à prouver avant de l'ajouter.
  - **Xbox et GOG Galaxy** : Galaxy note les jeux Xbox par un numéro (`xboxone_1755599316`), Frogtend par le nom de famille du paquet (`Microsoft.MinecraftUWP_8wekyb3d8bbwe`) : un jeu Xbox vu des deux côtés a deux cartes. À relier (le `MicrosoftGame.config` donne `TitleId` et `StoreId`).
- **Connexion repérée dans la fenêtre « Se connecter » de GOG, Prime Gaming et PlayStation (18/11)** : pour Epic, la fenêtre de connexion fait tourner `epic.js` en mode connexion et l'interface passe à « ✅ Connecté » dès que la boutique montre le compte. Pour GOG, Prime et PS Plus, l'état « ✅ Connecté » ne vient pour l'instant que d'un passage réussi (`faite`, `aucun`, `pas_abonne`) et repasse à « 🔑 Se connecter » sur `connexion` : leurs fenêtres de connexion n'ont pas de script. À faire : un mode connexion pour `gog.js` (`#menuUsername`), `prime.js` (`user-dropdown-first-name-text`) et `psplus.js`, sur le modèle d'Epic (aucun clic, événement `gratuits-connecte`).
- **Relecture des boutiques et disque réseau injoignable (relecture lancement, 18/11)** : `synchroniser_registre` (commandes.rs) tourne hors `spawn_blocking` et appelle `is_dir()` par jeu (noyau.rs `synchroniser_boutique`) : un partage réseau qui ne répond pas peut faire attendre l'échéance de Windows pour chaque jeu, RELECTURE tenu, un fil de tokio bloqué. Aucune donnée perdue. À faire : vérifier la présence des dossiers dans la phase de lecture (déjà en `spawn_blocking`) ou passer un AppHandle pour synchroniser hors du fil.
- **Figer d'office les jeux PC sans anti-triche (Seb 18/11 : « laissons ainsi pour le moment mais notons-le si jamais le cas pose vraiment problème »)** : vu avec Minecraft (Xbox, 0.211.1) — la manette agit sur le jeu DERRIÈRE le menu de Frogtend (un jeu PC lit la manette lui-même). Aujourd'hui « ❄ Figer le jeu pendant le menu » est désactivé d'office, jeu par jeu (décision du 05/10 : jeux en ligne, anti-triche). Piste si ça gêne : figer d'office quand aucun anti-triche n'est trouvé, en gardant le choix jeu par jeu.
- **Vrais titres des jeux Epic (18/11)** : le catcache d'Epic écrit les titres en ASCII (« ? » à la place de ™, é…) ; 21/500 gardent un « ? » chez Seb. Piste : demander à Firehouse le titre par l'id Epic (ou relier la carte à sa fiche LaunchBox), ou relire le titre au manifeste quand le jeu s'installe (déjà fait).

- **Réunir / séparer des copies à la main** (Seb, 21/11, après C4a) : un même jeu possédé sur plusieurs boutiques
  (Steam + GOG, GOG + Epic, Amazon + Steam…) n'est réuni sur une carte que si GOG Galaxy relie les copies (jamais par le
  titre, décision du 21/10). Quand Galaxy ne le fait pas : bouton « 🔗 Réunir avec… » dans le panneau du jeu (choisir
  l'autre carte), et « Séparer » pour défaire une réunion fausse (Galaxy comprise). Retenu par profil, jamais effacé par
  une relecture ; aussi à la manette dans Taodbox (règle des réglages au canapé). À faire APRÈS le lot C4.
