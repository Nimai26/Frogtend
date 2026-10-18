# Idées notées (à ne pas démarrer avant la fin du lot en cours)

- **Profils de manette par jeu** (lot 5, Taodbox) : RetroArch a ses « remaps » par jeu, DuckStation et PCSX2 leurs
  réglages par jeu (`gamesettings\`), Dolphin ses profils (`Config\Profiles\GCPad\`).
- **Dolphin et les manettes PlayStation** : Frogtend règle Dolphin sur XInput ; une manette PS4/PS5 n'y marche que
  par Steam ou DS4Windows. Piste : la source SDL de Dolphin (le nom de l'appareil y dépend du modèle).
- **« ⚙ Gérer le jeu » à la manette en Taodbox** (expert interface, 05/10) : Commandes, Émulateur, Figer pendant le
  menu, Plein écran au lancement, Jouer à la manette (jeu fait pour le clavier)… ne se règlent aujourd'hui qu'au bureau (le réglage s'applique ensuite en Taodbox).
- **Manette → clavier** (expert interface, 16/10) : choisir une touche en l'APPUYANT (« appuie sur la touche
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

- (16/10, expert lancement) MAME : sauver / charger / recommencer depuis le menu en jeu par le même script Lua (un
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
