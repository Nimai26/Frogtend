# Réglages des émulateurs dès la première partie — audit du 08/11/2026

> Règle de Seb (08/11, voir PLAN.md) : « frogtend DOIT configurer TOUS les émulateurs avec les langues, touches, etc. à
> chaque nouvelle installation […] Également il doit avoir son image des touches dans le menu. »
> Relevé fait par l'expert lancement, en lecture seule, au moment de la 0.169.0. Les ✅ ont été vus dans le code, et
> non essayés sur une installation neuve, sauf mention contraire.

Légende : ✅ fait · ⚠ fait avec un risque · ❌ absent · n/a sans objet.

| Émulateur | Manette J1 | Langue | Bouton Xbox → menu | Installation neuve | Schéma 🎮 |
|---|---|---|---|---|---|
| RetroArch | ✅ profils officiels + autodétection | ❌ | ✅ (99, non essayé en vrai) | ✅ frogtend.cfg à chaque partie | ✅ par cœur |
| DuckStation | ✅ SDL-0 | ❌ (la PS1 n'a pas de langue ; l'interface : à vérifier) | ✅ | ⚠ settings.ini créé sans `SettingsVersion` | ✅ |
| PCSX2 | ✅ SDL-0 | ❌ (langue dans la NVM du BIOS : jamais touchée) | ✅ | ⚠ PCSX2.ini créé sans `[UI] SettingsVersion = 1` (contrat § 1.11) | ✅ |
| Dolphin | ✅ GameCube ; Wii par référence | ❌ | ✅ | ✅ | ⚠ GameCube seulement |
| RPCS3 | ✅ | ✅ | ✅ | ⚠ langue réglée à la 1re partie grâce à l'ordre des appels | ✅ |
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
