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
| Cemu | ❌ | ❌ | ❌ (laissé à l'émulateur) | n/a | ❌ |
| PPSSPP | « rien à régler » (relevé du 30/09) | ❌ | ❌ | ppsspp.ini créé | ❌ |
| Xenia | ❌ | ❌ | ❌ | n/a | ❌ |
| xemu | ❌ | ❌ | ❌ | xemu.toml créé | ❌ |
| Azahar | ❌ | ❌ | ⚠ donné au menu sans relevé (la 3DS a un bouton Home) | n/a | ❌ |
| Vita3K | ❌ | ❌ | ❌ | config.yml créé | ❌ |
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
   - Cemu : controller0.xml, `console_language`.
   - Vita3K : `sys-lang`.
   - Xenia : `user_language`.
   - Azahar : manette SDL ; la langue est dans la sauvegarde système.
   - PPSSPP : `[SystemParam]`.
   - mGBA, DOSBox et MAME : schéma uniquement.
6. **Points mineurs** :
   - Eden et sa famille avec une manette PlayStation (seul le GUID XInput est calculé).
   - Le schéma s'affiche même quand aucune manette n'était branchée.
   - Le schéma de la Wii.
