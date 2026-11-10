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
| Dolphin | ✅ GameCube ; Wii par référence | ✅ 0.173.0 GameCube (`SelectedLanguage`, une fois par profil) ; Wii et interface : langue de Windows d'office | ✅ | ✅ | ⚠ GameCube seulement |
| RPCS3 | ✅ | ✅ | ✅ | ✅ 0.173.0 : `rpcs3_avant_la_partie` crée le fichier et pose la langue, test d'installation neuve | ✅ |
| Eden / Eglantine / Lichii | ✅ manette XInput branchée | ✅ | ✅ | ✅ depuis la 0.169.0 | ✅ |
| Cemu | ✅ 0.174.0 : `controller0.xml` XInput, disposition d'origine de Cemu (une manette PlayStation seule : rien) | ✅ 0.174.0 : `console_language`, une fois (l'interface suit déjà Windows) | ✅ 0.174.0 avec la manette de Frogtend (Cemu ne lit pas le bouton Xbox) | ✅ settings.xml créé avec la langue : assistant sauté | ✅ |
| PPSSPP | ✅ par PPSSPP lui-même (XInput d'office, KeyMapDefaults.cpp) | ✅ par PPSSPP lui-même (`DefaultLangRegion`, jeux compris) | ⚠ à PPSSPP : son menu de pause (`NKCODE_HOME`), gardé | ppsspp.ini créé (une ligne, sans effet de bord : `FirstRun` ne sert à rien) | ✅ 0.174.0 manette Xbox ; PlayStation (DirectInput) : rien |
| Xenia | ✅ par Xenia lui-même (XInput ; SDL pour les autres) | ✅ 0.176.0 : `xconfig.settings` (binaire), une fois, à la 2e partie (Xenia le crée à la 1re) | ✅ 0.176.0 : `--guide_button=false` à chaque partie | n/a | ✅ 0.176.0 |
| xemu | ❌ | ❌ | ❌ | xemu.toml créé | ❌ |
| Azahar | ❌ | ❌ | ⚠ donné au menu sans relevé (la 3DS a un bouton Home) | n/a | ❌ |
| Vita3K | ✅ par Vita3K lui-même (SDL, par position, toute manette) | ✅ 0.175.0 : `sys-lang`, une fois (l'interface suit déjà Windows) ; accueil anglais retiré une fois, seulement firmware installé (`fs\vs0\sys\external`) | ⚠ à Vita3K : pause, en dur (`on_ps_button`), gardé | config.yml créé ; ⚠ **firmware non installé** (besoin 48) | ✅ 0.175.0 |
| mGBA | ❌ | n/a | ✅ | n/a | ❌ |
| DOSBox Staging | joysticktype auto | n/a | ✅ | .conf créé | ❌ |
| MAME | ❌ | n/a | ✅ | n/a | ❌ |

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
   - ✅ Vita3K (0.175.0) : langue et accueil. Reste : le firmware de la Vita (besoin 48), installé par
     `Vita3K --firmware <PUP>` une fois servi par Firehouse.
   - ✅ Xenia (0.176.0). `user_language` n'existe plus : la langue est dans `xconfig.settings` (6 680 octets, octet
     2 322) ; Frogtend ne crée jamais ce fichier, d'où la langue à la 2e partie seulement.
   - Azahar : manette SDL ; la langue est dans la sauvegarde système.
   - PPSSPP : `[SystemParam]`.
   - mGBA, DOSBox et MAME : schéma uniquement.
6. **Points mineurs** :
   - Eden et sa famille avec une manette PlayStation (seul le GUID XInput est calculé).
   - Le schéma s'affiche même quand aucune manette n'était branchée.
   - Le schéma de la Wii.
