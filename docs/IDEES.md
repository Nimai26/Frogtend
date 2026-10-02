# Idées notées (à ne pas démarrer avant la fin du lot en cours)

- **Profils de manette par jeu** (lot 5, Taodbox) : RetroArch a ses « remaps » par jeu, DuckStation et PCSX2 leurs
  réglages par jeu (`gamesettings\`), Dolphin ses profils (`Config\Profiles\GCPad\`).
- **Dolphin et les manettes PlayStation** : Frogtend règle Dolphin sur XInput ; une manette PS4/PS5 n'y marche que
  par Steam ou DS4Windows. Piste : la source SDL de Dolphin (le nom de l'appareil y dépend du modèle).
- ~~Wii dans Dolphin~~ → passé dans le plan (lot 4c, profils Wiimote réglés par Seb).

## Cheat Engine : ce que propose le forum « Extensions » (analysé le 02/10, demande de Seb)
Source : https://forum.cheatengine.org/viewforum.php?f=130 (≈ 230 sujets, scripts Lua de la communauté, dont
beaucoup écrits avec l'aide d'une IA : qualité variable). Mécanisme commun : un script `.lua` posé dans le dossier
`autorun\` de Cheat Engine s'exécute à son démarrage. **Règle** : rien n'est téléchargé du forum par Frogtend ; les
extensions retenues par Seb iraient dans SON zip (Firehouse les distribue), après relecture.
- **« Lancer avec Cheat Engine » en un clic** (inspiré de *automatic game launcher* et *Auto-Load Corresponding Cheat
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
