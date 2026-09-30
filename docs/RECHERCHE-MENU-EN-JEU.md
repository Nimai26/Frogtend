# Le menu universel en jeu (lot 4 ter) — ce que disent les sources

> Relevé le 30/09/2026 dans le code source officiel de chaque émulateur (branche principale) et dans la
> documentation Microsoft. Les numéros de ligne correspondent à ce jour-là. **[À MESURER]** = non vérifiable dans
> une doc : à essayer sur Venkman avant d'en dépendre.

## 1. Ce que Frogtend peut faire dans chaque émulateur, de l'extérieur

| | Pause | Reset | Quitter | Disque suivant | Triches en jeu | Sauvegarde rapide |
|---|---|---|---|---|---|---|
| **RetroArch** | UDP `PAUSE_TOGGLE` | UDP `RESET` | UDP `QUIT` | UDP `DISK_EJECT_TOGGLE` + `DISK_NEXT` | UDP `CHEAT_TOGGLE`, `CHEAT_INDEX_PLUS/MINUS` | UDP `SAVE_STATE` / `LOAD_STATE` |
| **DuckStation** | raccourci `TogglePause` | `Reset` | `PowerOff` | `SwitchToNextDisc` | son propre menu (`OpenCheatsMenu`) | `SaveSelectedSaveState` / `LoadSelectedSaveState` |
| **PCSX2** | raccourci `TogglePause` | `ResetVM` | `ShutdownVM` | ❌ impossible | ❌ (avant le lancement seulement) | PINE `MsgSaveState` / `MsgLoadState` |
| **Dolphin** | raccourci `General/Toggle Pause` | `General/Reset` | `General/Stop` | `General/Change Disc` | ❌ (avant le lancement seulement) | `Save State/…`, `Load State/…` |
| **PPSSPP** | raccourci `Pause` | WebSocket `game.reset` | fermer la fenêtre | (un seul UMD) | ❌ | raccourcis `Save State` / `Load State` |
| **DOSBox Staging** | Alt+Pause | Ctrl+Alt+Début | Ctrl+F9 | Ctrl+F4 (`swapimg`) | ❌ | ❌ (n'existe pas) |
| **Jeu PC natif** | ❌ (voir § 3) | ❌ | fermer proprement | — | — | — |

Détails et preuves :
- **RetroArch** : `network_cmd_enable = true`, `network_cmd_port = 55355`, et **`network_cmd_bind_address =
  "127.0.0.1"`** (sinon il écoute sur tout le réseau : command.c:280-284). Datagramme texte, une commande par ligne ;
  `GET_STATUS` répond `PAUSED|PLAYING …` (command.c:1420-1454). Pas de commande « disque n° N » ni « triche n° N »
  (on avance l'index). Réglages passés par notre `--appendconfig` (déjà en place).
- **DuckStation** : aucun canal de contrôle. Raccourcis `[Hotkeys] Nom = Keyboard/Touche` ; les touches **F13 à
  F24** sont reconnues (usb_key_code_data.inl:112-123) : des touches qu'aucun clavier n'a, donc sans conflit. Les
  touches n'arrivent que si sa fenêtre a le focus. `[Main] DisableBackgroundInput` (manette ignorée hors focus),
  `PauseOnFocusLoss` (défaut false). ⚠ Les noms des raccourcis d'emplacements numérotés sont inversés dans le code
  (hotkeys.cpp:804) : à essayer avant de s'y fier.
- **PCSX2** : PINE (`[EmuCore] EnablePINE`, TCP 127.0.0.1:28011) ne fait que mémoire, état et sauvegarde rapide.
  Raccourcis `TogglePause`, `ResetVM`, `ShutdownVM`. `[UI] PauseOnFocusLoss` (défaut false). F13–F24 : non vérifié.
- **Dolphin** : pas de contrôle externe sous Windows (les « pipes » sont Unix seulement). Raccourcis
  `Config\Hotkeys.ini [Hotkeys]`, périphérique `DInput/0/Keyboard Mouse`. `[Interface] PauseOnFocusLost` (défaut
  false). ⚠ **Plein écran EXCLUSIF par défaut** (`GFX.ini [Settings] BorderlessFullscreen = False`) : à passer à
  `True`, sinon aucune fenêtre ne peut s'afficher par-dessus.
- **PPSSPP** : WebSocket de débogage (`RemoteDebuggerOnStartup`, port `RemoteISOPort`, chemin `/debugger`) : `game.reset`,
  `game.status`, entrées simulées ; pas de sauvegarde rapide ni de quitter. Raccourcis `controls.ini [ControlMapping]`
  (`Pause`, `Reset`, `Save State`, `Load State`). `PauseOnLostFocus` (défaut false). Plein écran sans bordure.
- **DOSBox Staging** : raccourcis par défaut (sdl_gui.cpp:1770-1797, bios_disk.cpp:713). `[sdl] pause_when_inactive`
  (défaut false). `fullscreen_mode = standard` peut devenir exclusif selon le pilote : `forced-borderless` existe.

## 2. Le côté Windows

- **Lire la manette quand Frogtend n'a pas le focus** :
  - XInput (`xinput1_4.dll`, livré avec Windows 10+) : manettes Xbox et manette virtuelle de Sunshine. Mais Microsoft
    dit que l'entrée est « activée/désactivée par le système selon le focus » (doc de `XInputEnable`). **[À MESURER]**
  - Windows.Gaming.Input : au premier plan seulement (doc de gilrs) → écarté.
  - SDL3 avec `SDL_HINT_JOYSTICK_ALLOW_BACKGROUND_EVENTS` : le seul chemin qui couvre PS4/PS5/Switch Pro/8BitDo
    (HID brut). Coût : une DLL de 2 à 3 Mo, ou une compilation (CMake). **[À MESURER]** en arrière-plan.
  - GameInput (`GameInputEnableBackgroundInput`) : troisième piste.
  - Le bouton Guide n'est pas exposé par XInput (et Steam / la Game Bar le prennent) → pas de Guide.
- **Raccourci clavier global** : `tauri-plugin-global-shortcut` (RegisterHotKey). Appuyer dessus donne le droit de
  passer au premier plan.
- **Afficher le menu par-dessus** : une fenêtre « toujours au-dessus » passe au-dessus d'un jeu en plein écran SANS
  BORDURE (le compositeur de Windows reprend la main, comme la Game Bar). En plein écran EXCLUSIF, non : il faut
  sortir le jeu du plein écran exclusif (réglage de l'émulateur). Depuis la manette, prendre le premier plan exige un
  contournement (touche Alt simulée) **[À MESURER]**.

## 3. La pause

- **Voie retenue** : régler chaque émulateur pour qu'il se mette **en pause quand il perd le focus** (RetroArch le fait
  déjà ; `PauseOnFocusLoss` pour DuckStation et PCSX2, `PauseOnFocusLost` pour Dolphin, `PauseOnLostFocus` pour PPSSPP,
  `pause_when_inactive` pour DOSBox). Ouvrir le menu = lui prendre le focus = la pause. Le rendre = la reprise. Et la
  manette ne pilote plus le jeu pendant qu'on est dans le menu.
- Les autres actions (reset, disque, sauvegarde rapide) : on rend le focus à l'émulateur et on lui envoie son
  raccourci (F13–F24 réglées par Frogtend) ou sa commande (UDP RetroArch, PINE PCSX2).
- **Jeux PC natifs** : pas de pause universelle sûre. Suspendre le processus existe (gel par Job), mais son qui boucle,
  anti-triche, blocages : jamais d'office, jamais sur un jeu en ligne. Leur menu propose : reprendre, lire le manuel,
  quitter.
