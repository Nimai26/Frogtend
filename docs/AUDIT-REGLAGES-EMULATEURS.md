# Réglages des émulateurs dès la première partie — audit du 08/11/2026

> Règle de Seb (08/11, voir PLAN.md) : « frogtend DOIT configurer TOUS les émulateurs avec les langues, touches, etc. à
> chaque nouvelle installation […] Également il doit avoir son image des touches dans le menu. »
> Relevé fait par l'expert lancement, en lecture seule, au moment de la 0.169.0. Les ✅ ont été vus dans le code, et
> non essayés sur une installation neuve, sauf mention contraire.

Légende : ✅ fait · ⚠ fait avec un risque · ❌ absent · n/a sans objet.

| Émulateur | Manette J1 | Langue | Bouton Xbox → menu | Installation neuve | Schéma 🎮 |
|---|---|---|---|---|---|
| RetroArch | ✅ profils officiels + autodétection | ✅ par RetroArch lui-même (langue de Windows si `user_language` absent, configuration.c ; chez Seb : 2) | ✅ (99, non essayé en vrai) | ✅ frogtend.cfg à chaque partie | ✅ par cœur |
| DuckStation | ✅ SDL-0 | ❌ (la PS1 n'a pas de langue ; l'interface : à vérifier) | ✅ | ✅ 0.172.0 : base d'origine (assistant passé, clavier, raccourcis), essayé sur DuckStation 0.1-12640 neuf | ✅ |
| PCSX2 | ✅ SDL-0 | ❌ (langue dans la NVM du BIOS : jamais touchée) | ✅ | ✅ 0.172.0 : base d'origine (`SettingsVersion = 1`, clavier, raccourcis) ; pas encore ouvert en vrai | ✅ |
| Dolphin | ✅ GameCube ; Wii par référence | ✅ 0.173.0 GameCube (`SelectedLanguage`, une fois par profil) ; Wii et interface : langue de Windows d'office | ✅ | ✅ | ✅ GameCube ; Wii (0.182.0) : les références livrées |
| RPCS3 | ✅ | ✅ | ✅ | ✅ 0.173.0 : `rpcs3_avant_la_partie` crée le fichier et pose la langue, test d'installation neuve | ✅ |
| Eden / Eglantine / Lichii | ✅ manette XInput branchée | ✅ | ✅ | ✅ depuis la 0.169.0 | ✅ |
| Cemu | ✅ 0.174.0 : `controller0.xml` XInput, disposition d'origine de Cemu (une manette PlayStation seule : rien) | ✅ 0.174.0 : `console_language`, une fois (l'interface suit déjà Windows) | ✅ 0.174.0 avec la manette de Frogtend (Cemu ne lit pas le bouton Xbox) | ✅ settings.xml créé avec la langue : assistant sauté | ✅ |
| PPSSPP | ✅ par PPSSPP lui-même (XInput d'office, KeyMapDefaults.cpp) | ✅ par PPSSPP lui-même (`DefaultLangRegion`, jeux compris) | ⚠ à PPSSPP : son menu de pause (`NKCODE_HOME`), gardé | ppsspp.ini créé (une ligne, sans effet de bord : `FirstRun` ne sert à rien) | ✅ 0.174.0 manette Xbox ; PlayStation (DirectInput) : rien |
| Xenia | ✅ par Xenia lui-même (XInput ; SDL pour les autres) | ✅ 0.176.0 : `xconfig.settings` (binaire), une fois, à la 2e partie (Xenia le crée à la 1re) | ✅ 0.176.0 : `--guide_button=false` à chaque partie | n/a | ✅ 0.176.0 |
| xemu | ✅ par xemu lui-même (`auto_bind` : 1re manette SDL au port 1) | ✅ 0.179.0 : `eeprom.bin` (octet 0x90, somme refaite), une fois, à la 2e partie (xemu la crée à la 1re) | ⚠ à xemu : son menu, en dur (ui/xui/main.cc), gardé | xemu.toml créé | ✅ 0.179.0 |
| Azahar | ✅ 0.180.0 : profil courant de `qt-config.ini`, toute manette (`maptype:all`), disposition de l'auto-configuration d'Azahar (par position) ; jamais par-dessus celle de la personne | ✅ 0.180.0 : fichier `config` de la NAND du profil (bloc 0x000A0002), une fois par profil, à la 2e partie | ✅ 0.180.0 : Home laissé au clavier → menu de Frogtend ; à Azahar si la personne a mis Home sur sa manette | qt-config.ini créé avec le profil de manette | ✅ 0.180.0 |
| Vita3K | ✅ par Vita3K lui-même (SDL, par position, toute manette) | ✅ 0.175.0 : `sys-lang`, une fois (l'interface suit déjà Windows) ; accueil anglais retiré une fois, seulement firmware installé (système `fs\vs0\sys\external` et polices `fs\sa0`, 0.177.0) | ⚠ à Vita3K : pause, en dur (`on_ps_button`), gardé | config.yml créé ; firmware servi (besoin 48) installé par Frogtend 0.177.0 | ✅ 0.175.0 |
| mGBA | ❌ (voir plus bas : sa table XInput date d'un ancien SDL) | n/a | ✅ | n/a | ❌ |
| DOSBox Staging | joysticktype auto | n/a | ✅ | .conf créé | ✅ 0.181.0 manette Xbox seule (4 boutons, stick gauche) ; PlayStation, plusieurs manettes, lanceur du jeu : rien |
| BigPEmu | ✅ 0.184.0 par BigPEmu lui-même au premier démarrage (manette Xbox relevée) ; ensuite `Input.AutoAssign` posé par Frogtend tant que le joueur 1 est au clavier | n/a (anglais seulement, traduction non officielle) | ✅ 0.184.0 (DirectInput : le bouton Xbox lui est invisible) ; figé pendant le menu | ✅ 0.184.0 `UserData\` relié au profil, plein écran d'origine (`DisplayMode` 0 = fenêtre sans bord) | ✅ 0.184.0 manette Xbox ; PlayStation : rien |
| MAME | ✅ par MAME lui-même (XInput, winhybrid) | n/a | ✅ (0.181.0 : pas pris si MAME est réglé sur un autre pilote) | n/a | ✅ 0.181.0 manette Xbox ; PlayStation (DirectInput) : rien |

## À faire, dans l'ordre (les émulateurs de Seb d'abord)

1. **PCSX2 et DuckStation** : écrire `SettingsVersion` à la création du fichier (ajoutée si absente, jamais
   changée). Avant cela, relire dans leur code (qthost.cpp, QtHost.cpp) si un fichier sans cette clé est remis à zéro
   au premier démarrage, ce qui reproduirait le bug de Lichii.
2. **RPCS3** : écrire un test de lancement sur une installation neuve, pour que la langue ne dépende plus de l'ordre
   des appels.
3. **« Prêt à jouer »** : n'annoncer « manette réglée » que si le cœur répond `reglee` (`pret.svelte.ts`).
4. **Langue** de RetroArch (`user_language`, français = 2) et de Dolphin (`SelectedLanguage` ; SYSCONF pour la Wii),
   à vérifier dans leur code. Sans écraser un choix de la personne.
5. **Les autres émulateurs**, chacun après avoir relu son code : manette, langue, bouton Xbox et schéma.
   - ✅ Cemu (0.174.0). Reste : une manette PlayStation seule (profil SDL à relever). Le clavier choisi pour un jeu :
     Cemu n'a AUCUNE disposition clavier d'origine (VPADController.cpp `set_default_mapping` : SDL et XInput
     seulement) → rien d'inventé ; la manette de Frogtend reste. Si la personne valide les réglages de manette de
     Cemu, même sans rien changer, le profil devient le sien : le bouton Xbox revient à Cemu (prudent).
   - ✅ PPSSPP (0.174.0) : rien à écrire. Le bouton Xbox reste à son menu (le retirer = réécrire tout `controls.ini`).
   - ✅ Vita3K (0.175.0) : langue et accueil ; firmware de la Vita (0.177.0, besoin 48 servi par Firehouse 3.160.0).
   - ✅ Xenia (0.176.0). `user_language` n'existe plus : la langue est dans `xconfig.settings` (6 680 octets, octet
     2 322) ; Frogtend ne crée jamais ce fichier, d'où la langue à la 2e partie seulement.
   - ✅ xemu (0.179.0) : langue dans l'EEPROM (256 o, section utilisateur non chiffrée) ; seulement celle d'origine
     du dossier d'xemu portable, jamais une EEPROM choisie ailleurs.
   - ✅ Azahar (0.180.0) : touches d'origine au CLAVIER → profil de manette écrit (comme Eden, sans GUID) ; langue dans
     la sauvegarde système (un octet, pas de somme). Non vérifié : un jeu japonais lancé en premier peut faire
     réécrire le japonais par Azahar (`UpdatePreferredRegionCode`) ; alors le français n'est pas posé (« encore anglais »).
   - PPSSPP : `[SystemParam]`.
   - ✅ MAME et DOSBox Staging (0.181.0) : schéma, relevé dans leur code (MAME 0.289 input_xinput.cpp et
     inpttype.ipp ; DOSBox Staging 0.83.0 mapper.cpp). Rien de dessiné dès que la disposition peut être autre :
     touches changées (`<newseq`/`<mapdevice` des cfg, `ctrlr`, mappeur de DOSBox, `joysticktype`…), autre pilote
     de MAME, DOSBox non portable ou avec `dosbox.conf`, plusieurs manettes, jeu lancé par son propre lanceur.
     Limite connue : une manette DirectInput d'une autre marque (Switch Pro, 8BitDo en mode D) n'est pas reconnue ;
     seule, elle reçoit le dessin d'une manette Xbox, que MAME et DOSBox ne lui appliquent pas.
   - ⚠ mGBA : PAS de schéma. Sa disposition d'origine ne lie que la croix et le stick (sdl-events.c
     `mSDLInitBindingsGBA`) ; ses boutons viennent d'un profil intégré choisi par le NOM SDL de la manette
     (InputProfile.cpp : « XInput Controller #N », numéros de boutons de l'ancien SDL : A = 11, B = 10…). Avec un
     SDL récent, le nom et la numérotation ont changé : sans essai réel, on ne sait pas ce que font les boutons.
     À faire : un essai avec Seb (mGBA 0.10.5, manette Xbox), puis sans doute écrire la manette de mGBA comme pour
     Azahar.
6. **Points mineurs** :
   - ✅ (0.183.0, non essayé en vrai) Eden et sa famille avec une manette PlayStation : GUID et disposition lus dans
     le SDL3 de Frogtend (`manettes_sdl::releve_des_manettes`, sans ouvrir la manette) ; écrit seulement pour une
     manette Sony du pilote HIDAPI dont la disposition SDL est exactement la standard (`eden::DISPOSITION_SDL_HIDAPI`).
   - ⚠ Cemu avec une manette PlayStation seule : rien d'écrit. Cemu 2.6 lit les manettes par SDL2 et les nomme
     `<index>_<GUID SDL2>` avec la somme du nom (SDLController.cpp) ; SDL2 2.30 note le bus USB même en Bluetooth
     (SDL_hidapijoystick.c:929) : le GUID de SDL3 ne peut pas être recopié sans un relevé réel.
   - ✅ (0.182.0) Le schéma s'affichait même quand aucune manette n'était branchée : seule la famille Eden en dépend
     (manette liée par son GUID, donc seulement branchée au lancement) → état « au clavier » ; le clavier choisi pour
     un jeu → « clavier choisi » sur la fiche. Correctif : `\default=true` (clavier rendu) lu comme la valeur d'origine.
   - ✅ (0.182.0) Le schéma de la Wii : seulement une référence livrée en place (contenu comparé, `references::
     wiimote_en_place` / `wiimote_livree`) ; retouchée, vraie Wiimote : rien.
