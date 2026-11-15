# L'émulation PS4 et PS5 — recherche du 15/11/2026

> Demande de Seb (15/11) : « l'émulation PS4 et PS5 commence à bien fonctionner, nous allons donc devoir l'intégrer ».
> Relevé fait en lecture seule dans les sources officielles (README, code de la branche `main`, API GitHub, listes de
> compatibilité). Rien n'a été installé ni lancé. À revérifier sur les zips réels avant de coder.

## PS4 : shadPS4 (github.com/shadps4-emu/shadPS4) — la référence

- **Version** : v0.21.0 (30/10/2026), `shadps4-win64-sdl-0.21.0.zip` (~31,7 Mo), un zip, le CŒUR en ligne de commande
  (l'interface officielle, QtLauncher, est un autre dépôt : inutile à Frogtend). Autres projets PS4 : fpPS4 (dernière
  version 2022), Obliteration (noyau expérimental), Prosperity (Linux, v0.0.1), Kyty d'origine (abandonné).
- **Portable** : `<dossier COURANT>\user` s'il existe, sinon `%APPDATA%\shadPS4` (path_util.cpp:88-108) → Frogtend crée
  `user\` et lance avec le dossier courant = celui de l'exe.
- **Jeu** : un dossier extrait `CUSAxxxxx\` (`eboot.bin`, `sce_sys\param.sfo`) ou une archive `.zar`. **Aucune
  installation de `.pkg`**.
- **Ligne de commande** : `shadPS4.exe -g "<dossier>\eboot.bin" --fullscreen true` (main.cpp:80-110). Sans argument : une
  boîte de message.
- **Réglages** : `user\config.json` (JSON ; ancien `config.toml` migré). `General.console_language` (français France = 2),
  `General.install_dirs`, `GPU.full_screen`, `General.show_fps_counter`… ; par jeu : `user\custom_configs\<CUSA>.json`.
- **Parties** : par utilisateur PS4, `user\home\<user_id>\savedata\<CUSA>\…` (1000 à 1003 par défaut, `users.json`).
- **Demande** : 38 modules système `.sprx` dans `user\sys_modules`, « dumped from your legally owned PlayStation 4
  console » (README:148-167). Ni firmware complet ni clés.
- **Raccourcis** (`user\input_config\global.ini`) : F12 capture, F11 plein écran, F10 compteur, F9 pause,
  Ctrl+Maj+Fin quitter (avec confirmation). Ni avance rapide, ni recommencer, ni son coupé, ni sauvegarde rapide.
- **API de pilotage** (ipc.cpp) : `SHADPS4_ENABLE_IPC=true`, commandes par l'entrée standard (`RUN` puis `START`
  obligatoires, sinon arrêt en 5 s) : `PAUSE`, `RESUME`, `STOP`, `TOGGLE_FULLSCREEN`, `ADJUST_VOLUME`, `PATCH_MEMORY`…
  ⚠ Avec l'IPC, les patches automatiques sont coupés (au lanceur de les envoyer).
- **Triches / patches** : XML dans `user\patches\` ; triches appliquées par `PATCH_MEMORY` (dépôts ps4_cheats,
  GoldHEN_Cheat_Repository, PS4-PS5-Game-Patch).
- **Manette** : SDL3, Xbox et DualShock d'office. **Fermeture** : fenêtre fermée = sans confirmation.
- **Compatibilité** (shadps4-compatibility, 15/11, fiches et non jeux uniques) : jouable 18 %, en jeu 27 %, menus 15 %,
  démarre 15 %, rien 26 %.

## PS5 : KytyPS5 (github.com/KytyPS5/KytyPS5) — réel et actif, jeune

- Créé le 09/06/2026, 11 251 étoiles, versions QUOTIDIENNES sans numéro stable (`KytyPS5-2026-11-14-3e226bf-Windows-x64.zip` :
  `kyty_launcher.exe` + `kyty_emulator.exe`).
- **Demande** : rien (« External low-level emulation modules are neither required nor planned ») ; Vulkan 1.3.
- **Jeu** : dossier avec `eboot.bin` (`sce_sys\param.json`), `.zar` ou ELF.
- **Ligne de commande** : `kyty_emulator.exe --game "<dossier>" --fullscreen --console-language 2 --no-splash` ; tout par
  la ligne de commande (`--user-id`, `--user-name`, `--game-patch`, `--addcont-dir`, `--pssr-upscaler`…). Langue 2 =
  français : supposé (même table Sony), non vérifié.
- **Parties** : `_SaveData\<TITLE_ID>\…` RELATIF au dossier courant → lancer depuis le dossier de l'exe.
- **Raccourcis** : F11 plein écran, F2 compteur ; ni capture, ni pause, ni sauvegarde rapide, ni API.
- **Compatibilité** (compatibility.json, 470 jeux) : en jeu 59 %, menu 15 %, logo 14 %, ne démarre pas 11 % — pas de
  niveau « jouable ».
- Écartés : Kyty d'origine, KytyPlus (mainteneur qui se retire), Prosperity (Linux), et de nombreux faux « PS5 emulator ».

## Ce que Frogtend devra faire (à valider avec Seb)

1. Installer shadPS4 (PS4) et KytyPS5 (PS5) sur accord, depuis leur zip officiel (taille annoncée) ; mode portable,
   lancés depuis leur dossier.
2. Réglés dès la première partie : français (2), plein écran ; manette XInput d'office (SDL3).
3. Les jeux : seulement des dossiers extraits ou des `.zar` (jamais un `.pkg`) → à demander à Firehouse.
4. PS4 : les modules système `.sprx` d'une vraie PS4 → servis par Firehouse comme un BIOS ? (décision de Seb).
5. Menu en jeu : shadPS4 par son IPC (pause, arrêt propre, son par le volume, capture F12, triches) ; KytyPS5 réduit
   (plein écran, quitter).
6. Parties par personne : un utilisateur PS4 par profil (shadPS4), `--user-id` par profil (KytyPS5) ; sauvegardées chez
   Firehouse.
7. Dire franchement la compatibilité.
