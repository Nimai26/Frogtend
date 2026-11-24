# Où les moteurs de jeux PC rangent leurs parties (Windows)

> Recherche documentaire du 24/11/2026, pour la mise à l'abri des parties PC par Frogtend.
> Lecture web seulement : rien n'a été téléchargé ni installé, aucun jeu n'a été ouvert.
> Chaque affirmation porte son niveau de preuve :
> - **[doc]** documentation officielle du moteur ;
> - **[code]** code source officiel du moteur (dépôt de l'éditeur) ;
> - **[secondaire]** forum, outil tiers, guide : à confirmer sur un vrai jeu ;
> - **[à vérifier]** souvenir ou déduction non confirmés par une source lue aujourd'hui.
>
> Jetons utilisés : `<jeu>` = le dossier d'installation du jeu (celui du .exe) ; `%APPDATA%` = `…\AppData\Roaming` ;
> `%LOCALAPPDATA%` = `…\AppData\Local` ; `LocalLow` = `%USERPROFILE%\AppData\LocalLow` (il n'a pas de variable
> d'environnement ; Windows le donne par le dossier connu `FOLDERID_LocalAppDataLow`).

## Règles communes (valent pour tous les moteurs)

1. **Ne jamais exécuter de code du jeu pour lire un nom.** Ren'Py (`.rpyc`, `.rpa`) range ses données en *pickle*
   Python : les « dé-pickler » exécute du code arbitraire. On cherche des octets, on ne désérialise jamais.
2. **Une preuve = le marqueur du moteur ET le dossier de parties qui existe.** Le nom calculé depuis les fichiers du
   jeu donne un *candidat* ; on ne retient le dossier que s'il existe et contient des fichiers du bon motif
   (extension, nom). Sinon : « parties introuvables », jamais une supposition.
3. **Windows VirtualStore** [à vérifier, comportement Windows connu] : un vieux jeu 32 bits sans manifeste, installé
   sous `Program Files`, qui écrit dans son propre dossier est redirigé vers
   `%LOCALAPPDATA%\VirtualStore\Program Files (x86)\<jeu>\…`. Concerne surtout RPG Maker XP/VX/Ace et les vieux
   Ren'Py : chercher aussi là quand `<jeu>` est sous `Program Files`.
4. **Le jeu doit être fermé** pendant la copie (fichiers en cours d'écriture, bases LevelDB de NW.js verrouillées).

---

## 1. Ren'Py

### (a) Marqueurs sûrs
- un dossier `game\` à côté du .exe, et un dossier `renpy\` (le moteur en Python) ;
- un dossier `lib\` (Python embarqué : `lib\py3-windows-x86_64\` pour Ren'Py 8, `lib\py2-windows-x86_64\` ou
  `lib\windows-i686\` pour 7 et 6) [à vérifier : noms exacts selon la version] ;
- `<nom>.exe` **et** `<nom>.py` côte à côte (le .exe lance le .py ; le nom vient de `build.executable_name`, par
  défaut `build.name`) [doc build] ;
- dans `game\` : des `*.rpyc` (script compilé), souvent des `*.rpa` (archives), et `game\script_version.txt`
  (ajouté aux paquets depuis Ren'Py 6.99.5 ; contient un tuple comme `(8, 3, 7)`) [doc changelog 6 + secondaire].
- Deux marqueurs suffisent en pratique : `renpy\` + `game\` contenant au moins un `.rpyc` ou `.rpa`.

### (b) Emplacements des parties
Le moteur écrit **dans plusieurs emplacements à la fois** [code `renpy/savelocation.py`] :
1. `config.savedir`, qui vaut sur Windows **`%APPDATA%\RenPy\<save_directory>`** [doc `config.save_directory`,
   code `renpy.py:path_to_saves`] ;
2. **`<jeu>\game\saves\`** (sauf mobile et application macOS) [code `savelocation.py`] ;
3. les dossiers de `config.extra_savedirs`, s'il y en a [code].

`MultiLocation.save` écrit chaque partie **dans tous les emplacements actifs** (un emplacement est actif si le moteur
a pu y créer puis effacer un fichier d'essai) ; au chargement, il prend **le plus récent** (date de modification).
Le fichier `persistent` est lui aussi écrit partout [code].

Autres cas [code `renpy.py:path_to_saves`] :
- `config.save_directory = None` → seulement `<jeu>\game\saves\` ;
- un dossier nommé `Ren'Py Data` trouvé en remontant depuis le dossier du moteur → il remplace `%APPDATA%\RenPy`
  (installation portable) ;
- la variable d'environnement `RENPY_PATH_TO_SAVES` remplace `%APPDATA%\RenPy` ;
- sans `APPDATA` : `~\RenPy\<save_directory>`.

Fichiers voisins, hors du dossier du jeu [code] :
- **`%APPDATA%\RenPy\tokens\security_keys.txt`** et `upgraded.txt` (Ren'Py 8.1 et plus : signature des parties ;
  une partie venue d'un autre PC déclenche l'avertissement « partie d'origine inconnue ») — **dossier commun à
  tous les jeux Ren'Py** (`renpy/savetoken.py`) ;
- **`%APPDATA%\RenPy\persistent\<nom>`** : les données `MultiPersistent` partagées volontairement entre plusieurs
  jeux d'un même auteur (`renpy/persistent.py`, aussi `~\RenPy\Persistent`).

### (c) Trouver `save_directory` sans lancer le jeu
La doc dit qu'il « doit être réglé par `define` ou dans un bloc `python early` », et que c'est une chaîne littérale,
pas une expression [doc]. D'où l'ordre de recherche :

1. **Les `.rpy` en clair** (le plus souvent `game\options.rpy`) — fiabilité **haute**. Motifs :
   - Ren'Py 7 et 8, gabarit du lanceur : `define config.save_directory = "MonJeu-1234567890"` ;
   - Ren'Py 6 (anciens gabarits) : `config.save_directory = "…"` dans un `init -1 python hide:` [à vérifier] ;
   - accepter `'…'`, `u"…"`, `r"…"`, et la valeur `None` (→ seulement `game\saves`).
   Ignorer les lignes commentées (`#`). Si plusieurs fichiers en donnent une, c'est un cas douteux : le dire.
2. **`script_version.txt` ne contient PAS `save_directory`** : seulement la version `(majeur, mineur, correctif)`
   [secondaire, cohérent avec le code `script.py`]. Il sert à dater la version de Ren'Py, rien d'autre.
3. **Les `.rpyc` compressés** — fiabilité **moyenne** [secondaire unrpyc + code `script.py`] :
   - format 2 (Ren'Py ≥ 6.99.5) : en-tête `RENPY RPC2` (10 octets), puis une table d'entrées de 12 octets
     `(slot, début, longueur)` en entiers 32 bits petit-boutistes, terminée par `slot == 0` ; le bloc du slot 1
     (ou 2) est compressé zlib et contient l'arbre du script en pickle ;
   - format 1 (ancien) : le fichier entier est un bloc zlib ;
   - **méthode sûre** : décompresser avec zlib, puis chercher les octets `save_directory` et la chaîne littérale qui
     suit (la source de l'expression `"MonJeu-123…"` est rangée en texte dans le nœud `define`). **Ne jamais
     dé-pickler.**
   - piège : des jeux « obscurcissent » leurs `.rpyc` (en-tête modifié comme `RENPY RPC2 DDGAME`, couches base64/
     zlib supplémentaires) → zlib échoue : abandonner proprement, passer à l'étape 5.
4. **Les archives `.rpa`** — fiabilité **moyenne** [code `renpy/loader.py`] :
   - `RPA-3.0 ` : la 1re ligne donne en hexadécimal le décalage de l'index (`[8:24]`) et une clé (`[25:33]`) ;
     l'index, à ce décalage, est un pickle compressé zlib : `{chemin: [(décalage ^ clé, longueur ^ clé, préfixe)]}` ;
   - `RPA-2.0 ` : même chose sans clé ; `.rpi` (RPA-1) : index zlib entier ;
   - l'index est un pickle : le lire avec un **lecteur de pickle restreint** (dictionnaire, listes, tuples, entiers,
     chaînes, octets seulement ; refus de tout autre opcode), jamais avec `pickle.loads`. Puis extraire
     `options.rpyc` (ou tout `.rpyc`) et appliquer l'étape 3.
   - les fichiers en vrac sur le disque l'emportent sur ceux des archives [code] : un `options.rpy`/`.rpyc` en
     vrac est prioritaire.
5. **Repli par le disque** — fiabilité **faible à moyenne** : `<jeu>\game\saves\` s'il existe, puis les dossiers de
   `%APPDATA%\RenPy\` contenant un `persistent` et des `*.save`, dont le nom commence par `build.name` / le nom du
   .exe (le gabarit du lanceur produit `<NomDuProjet>-<horodatage>`). Proposer, ne jamais conclure seul.
6. `log.txt` à la racine du jeu peut citer le dossier de parties [à vérifier sur un jeu réel].

### Noms de fichiers
- parties : `*.save` (suffixe `renpy.savegame_suffix`) [secondaire, code `loadsave.py`] ; ce sont des **archives
  zip** (métadonnées JSON, capture d'écran, `renpy_version`) [doc save_load_rollback + code] ;
- noms de slots : `<page>-<case>` (`1-1.save`), `auto-…`, `quick-…`, `_tracesave-1…10` [doc] ; en pratique on voit
  aussi des suffixes comme `1-1-LT1.save` [secondaire] ;
- **`persistent`** (sans extension ; `persistent.new` pendant l'écriture) : progression transversale (galerie,
  fins débloquées, choix « déjà vus ») **mêlée aux préférences** ; c'est une donnée de progression, à garder ;
- fichiers temporaires `.<horodatage>.tmp` à ignorer [code].

### Différences selon les versions
- **6.x** : Python 2 ; avant 6.99.5, `script_version.rpy/.rpyc` au lieu de `.txt`, et `.rpyc` au format 1 ;
- **7.x** : Python 2, `.rpyc` format 2, gabarit `define config.save_directory` ;
- **8.x** : Python 3 ; **8.1+** ajoute les jetons de signature (`%APPDATA%\RenPy\tokens`) [code `savetoken.py`].
- L'écriture simultanée dans `game\saves` est décrite d'après le code actuel ; son ancienneté exacte n'est pas
  datée par une source lue [à vérifier sur de vieux jeux 6.x].

### (d) Pièges
- **Deux copies** (APPDATA et `game\saves`) : sauvegarder les deux ; à la restauration, poser la plus récente
  dans les deux, sinon le moteur reprendra la plus récente des deux… qui peut être l'ancienne.
- **`%APPDATA%\RenPy\tokens\`** est commun à tous les jeux : ne jamais l'attribuer à un jeu ni le restaurer par
  jeu. Il contient une clé de signature (à traiter comme un secret). Décision à prendre avec Seb : le sauvegarder
  une fois pour le PC, pour éviter l'avertissement à la reprise sur un autre poste.
- **`save_directory` partagé volontairement** entre épisodes d'une série (Ep1, Ep2…) ou générique (`renpy`,
  `game`…) : un même dossier pour plusieurs jeux. Le signaler, ne pas dupliquer, ne pas restaurer à l'aveugle.
- `MultiPersistent` (`%APPDATA%\RenPy\persistent\`) : partagé entre jeux d'un même auteur.

---

## 2. RPG Maker

### 2.1 XP, VX, VX Ace (RGSS 1, 2, 3)

**(a) Marqueurs sûrs** [secondaire : guides de détection et éditeurs de parties]
- `Game.exe` + **`Game.ini`** (section `[Game]` : `Library=…RGSS*.dll`, `Scripts=Data\Scripts.rxdata|rvdata|rvdata2`,
  `Title=…`) [à vérifier : clés exactes] ;
- la DLL du moteur : XP `RGSS10xE.dll` / `RGSS10xJ.dll`, VX `RGSS20xE.dll`, Ace `RGSS30x.dll` (souvent dans
  `System\`) ;
- l'archive chiffrée : XP `Game.rgssad`, VX `Game.rgss2a`, Ace `Game.rgss3a` ; ou bien un dossier `Data\` en clair
  (`.rxdata` / `.rvdata` / `.rvdata2`) ;
- la **version** se lit sur l'extension (rxdata = XP, rvdata = VX, rvdata2 = Ace) ou le nom de la DLL.

**(b) Emplacement** : **`<jeu>\`**, à côté de `Game.exe` (scripts d'origine : XP `Save1.rxdata`…`Save4.rxdata`,
VX `Save1.rvdata`…`Save4.rvdata`, Ace `Save01.rvdata2`…`Save16.rvdata2`) [secondaire concordant ; noms exacts
d'après les scripts d'origine : à vérifier]. Variantes rencontrées : un sous-dossier `Save\`, ou `Documents` /
`%APPDATA%` quand le jeu a modifié ses scripts [secondaire]. Et le VirtualStore (règle commune 3).

**(c) Nom variable** : aucun, le dossier est `<jeu>`. Fiabilité haute quand des `Save*.r*data*` existent.

**(d) Pièges**
- **Les `.rxdata/.rvdata/.rvdata2` de `Data\` ne sont PAS des parties** (cartes, base de données) : seuls les
  `Save*` à la racine (ou dans `Save\`) en sont. Filtrer par nom, pas par extension.
- Les scripts sont modifiables par l'auteur : nombre de cases, nom et dossier peuvent changer.
- Les portages **mkxp / mkxp-z** (fichier `mkxp.json`) peuvent déplacer les parties dans `%APPDATA%` [à vérifier].
- RPG Maker 2000/2003 (en bonus) : `RPG_RT.exe` + `RPG_RT.ini`, parties `Save01.lsd`… dans `<jeu>` [secondaire].

### 2.2 MV

**(a) Marqueurs sûrs**
- `Game.exe` (c'est NW.js renommé) + `nw.dll`, `nw_elf.dll`… + **`package.json`** à la racine ;
- dossier **`www\`** avec `www\index.html`, `www\js\rpg_core.js`, `rpg_managers.js`, `www\data\System.json`.

**(b) Emplacements** [code officiel `rpgtkoolmv/corescript`, `StorageManager.js`]
- mode local (`isLocalMode()` = « on tourne sous NW.js ») : **`<jeu>\www\save\`** ; et si `www\` n'est pas
  inscriptible (`canMakeWwwSaveDirectory()` échoue), **`<jeu>\save\`** (le dossier parent de `www`) ;
- fichiers : **`file1.rpgsave`, `file2.rpgsave`…** (parties), **`global.rpgsave`** (index des cases : titre, durée),
  **`config.rpgsave`** (réglages : volume, « toujours courir »…) ; copies de secours avec suffixe `bak` ;
- contenu : JSON compressé `LZString.compressToBase64` (texte lisible par Frogtend si besoin, sans exécuter quoi que
  ce soit).

### 2.3 MZ

**(a) Marqueurs sûrs** : `Game.exe` (NW.js) + `package.json` à la racine, **sans `www\`** ; `js\rmmz_core.js`,
`js\rmmz_managers.js`, `data\System.json` [secondaire].

**(b) Emplacement** : **`<jeu>\save\`** ; fichiers **`file1.rmmzsave`…, `global.rmmzsave`, `config.rmmzsave`**
(`filePath = fileDirectoryPath() + nom + ".rmmzsave"`) ; contenu compressé zlib par la bibliothèque **pako** (et non
LZString comme MV) [secondaire : extraits de `rmmz_managers.js` cités sur le forum officiel rpgmakerweb et la
référence de tonbijp ; le code de MZ n'est pas publié sur un dépôt officiel].

### 2.4 Quand MV/MZ rangent les parties dans le navigateur (NW.js)

- Le code d'origine n'utilise le stockage web que **hors NW.js** (`isLocalMode()` faux) : jeu lancé dans un
  navigateur, emballé dans un autre lanceur qui n'expose pas Node (Electron sans `nodeIntegration`, lecteurs
  tiers…), ou **greffon** qui force ce mode [code MV ; secondaire pour les greffons].
  - MV → `localStorage`, clés **`RPG File1`, `RPG Global`, `RPG Config`** (+ `bak`) [code] ;
  - MZ → **IndexedDB par localforage**, clés **`rmmzsave.<gameId>.<nom>`** (`rmmzsave.123456.file1`) [secondaire].
- Sous NW.js, le profil Chromium vit dans **`%LOCALAPPDATA%\<name>\User Data\Default\`** où `<name>` est le champ
  `name` de `package.json` [doc NW.js : `App.dataPath` = `%LOCALAPPDATA%/<name>` ; le sous-chemin
  `User Data\Default` est constaté depuis NW.js 0.13 par le ticket nwjs #4696, la doc n'est pas à jour]. On y
  trouve `Local Storage\leveldb\` (MV) et `IndexedDB\` (MZ).
- **Comment le reconnaître** : aucun `file*.rpgsave` / `file*.rmmzsave` dans `<jeu>\www\save`, `<jeu>\save` ;
  mais le dossier `%LOCALAPPDATA%\<name>\User Data\Default\Local Storage\leveldb\` (ou `IndexedDB\`) existe et ses
  fichiers `*.ldb` / `*.log` contiennent la chaîne `RPG File` (MV) ou `rmmzsave.` (MZ). Recherche d'octets
  seulement.
- **Sauvegarde** : copier le dossier `Local Storage` (ou `IndexedDB`) **en bloc**, jeu fermé ; ne pas tenter
  d'extraire une case de la base LevelDB.
- Greffons de sauvegarde (Cook Tool « Advanced Save Handler »…) : parties dans `%LOCALAPPDATA%\<nom tiré de
  package.json>`, ou `%APPDATA%\<jeu>\save\` [secondaire].

**(c) Nom variable** : `name` de `package.json` (lu tel quel, JSON). Fiabilité haute pour le profil NW.js ; inutile
pour les parties en fichiers (dossier `<jeu>`).

**(d) Pièges**
- **`name` par défaut partagé** : `KADOKAWA/RPGMV` → `%LOCALAPPDATA%\KADOKAWA\RPGMV\…` est **commun à tous les
  jeux MV** dont l'auteur n'a pas changé le nom (le forum officiel en avertit) ; pour MZ, la valeur par défaut serait
  `rmmz-game` [à vérifier]. Un dossier partagé ne s'attribue jamais à un seul jeu.
- Le dossier `%LOCALAPPDATA%\<name>\User Data` est créé par NW.js **même quand les parties sont en fichiers**
  (cache) : sa présence ne prouve rien.
- `config.rpgsave` / `config.rmmzsave` = réglages mêlés aux parties ; `global` = index nécessaire au menu de
  chargement : les sauvegarder tous ensemble.

---

## 3. Unity

### (a) Marqueurs sûrs
- un dossier **`<Jeu>_Data\`** à côté de **`<Jeu>.exe`** (même nom de base ; c'est `Application.dataPath`)
  [secondaire kimsama/Unity-Paths, conforme à la doc `Application.dataPath`] ;
- **`UnityPlayer.dll`** à côté du .exe (builds récents ; les très vieux builds intègrent le moteur dans le .exe)
  [à vérifier : version d'apparition, vers 2017] ;
- dans `<Jeu>_Data\` : `globalgamemanagers` (ou `mainData` dans les vieux builds), `Managed\Assembly-CSharp.dll`
  (Mono) **ou** `GameAssembly.dll` à la racine + `il2cpp_data\` (IL2CPP) ; dossier `MonoBleedingEdge\` ou `Mono\`.

### (b) Emplacements
- **`Application.persistentDataPath`** = **`%USERPROFILE%\AppData\LocalLow\<companyname>\<productname>`** pour le
  lecteur Windows (résolu par `SHGetKnownFolderPath(FOLDERID_LocalAppDataLow)`, sinon `CSIDL_LOCAL_APPDATA`)
  [doc]. C'est l'emplacement conseillé, mais **pas obligatoire** : le jeu écrit où il veut.
- **PlayerPrefs** : registre **`HKCU\Software\<companyname>\<productname>`** [doc]. Les noms de valeurs sont
  hachés (`Cle_h3232628825`). **Frogtend ne touche pas au registre** : à signaler seulement (« ce jeu garde peut-être
  une partie dans le registre, non sauvegardée »). Beaucoup de petits jeux y rangent leur progression entière.
- Autres emplacements fréquents choisis par l'auteur [secondaire] : `Documents\My Games\…`, `%APPDATA%\…`,
  `%LOCALAPPDATA%\…`, ou `<jeu>\` / `<Jeu>_Data\` lui-même.

### (c) Lire companyName et productName
1. **`<Jeu>_Data\app.info`** : petit fichier texte, **ligne 1 = companyName, ligne 2 = productName** (pas de fin de
   ligne finale). **Aucune doc officielle ne le décrit** ; c'est l'usage constaté de plusieurs outils libres
   (FolderForensics, hallow-vault, agvn-player) [secondaire]. Fiabilité haute quand il existe ; lire en UTF-8,
   garder les caractères exacts (accents, espaces).
2. Repli : `globalgamemanagers` contient les PlayerSettings (binaire sérialisé Unity) ; lisible seulement avec un
   lecteur de format Unity — fiabilité moyenne, coût élevé. Probablement inutile en premier lot.
3. Confirmation : le dossier `LocalLow\<company>\<product>\` doit exister ; il contient en général **`Player.log`**
   et `Player-prev.log` (journaux du lecteur Unity) [secondaire], ce qui prouve qu'il appartient bien à un jeu Unity
   lancé sur ce PC.

### (d) Pièges
- **`DefaultCompany`** (valeur par défaut de l'éditeur) : `LocalLow\DefaultCompany\` regroupe plusieurs jeux ;
  le niveau `<productname>` les sépare, mais un `productName` générique (`My project`, `Game`) peut collisionner.
- `persistentDataPath` mêle **parties, réglages, journaux (`Player.log`), caches** (`Unity\`, analytics,
  shaders) : exclure les journaux et caches de la sauvegarde, garder le reste.
- Le nom du .exe ≠ productName (le .exe se renomme librement).
- Le registre (PlayerPrefs) peut contenir toute la progression : ne pas promettre une sauvegarde complète.
- Le nuage Steam / Epic de certains jeux écrit ailleurs (sous-dossier au nom de l'identifiant du compte).

---

## 4. Unreal Engine 4 / 5

### (a) Marqueurs sûrs [secondaire, structure standard d'un jeu empaqueté]
- à la racine : `<Projet>.exe` (petit lanceur) + dossier **`Engine\`** (`Engine\Binaries\ThirdParty\…`) ;
- un dossier **`<Projet>\`** contenant **`Binaries\Win64\<Projet>-Win64-Shipping.exe`** et
  **`Content\Paks\*.pak`** (UE4 : `<Projet>-WindowsNoEditor.pak` ou `pakchunk0-WindowsNoEditor.pak` ; UE5 :
  `pakchunk0-Windows.pak`, plus `.utoc` / `.ucas` avec IoStore) [à vérifier : noms selon version].

### (b) Emplacements
- La doc : « sur les plateformes de développement, les parties portent l'extension **`.sav`** et se trouvent dans
  le dossier **`Saved\SaveGames`** du projet » [doc *Saving and Loading Your Game*]. `SaveGameToSlot(objet,
  NomDuSlot, IndexUtilisateur)` → fichier `<NomDuSlot>.sav` [doc API].
- Où est `Saved` : `FPaths::ShouldSaveToUserDir()` décide si `Saved` est enraciné dans le dossier utilisateur ou à
  côté du jeu [doc API FPaths]. Il est vrai si `FApp::IsInstalled()` (vrai en **Shipping**), ou `-SaveToUserDir`,
  ou `-UserDir=…` [forum officiel Epic, extraits du code moteur]. D'où :
  - **build Shipping (cas normal d'un jeu vendu)** : **`%LOCALAPPDATA%\<Projet>\Saved\SaveGames\`** ;
  - **build Development / Debug**, ou lancé avec `-NotInstalled` : **`<jeu>\<Projet>\Saved\SaveGames\`**
    [forum officiel Epic].
  - `UE_SAVED_DIR_OVERRIDE` (à la compilation) remplace `<Projet>` par un autre nom [forum officiel Epic].
- Beaucoup de jeux choisissent un autre dossier : `Documents\My Games\<X>\Saved\SaveGames\`, `%APPDATA%\…`, avec un
  sous-dossier par identifiant Steam/Epic [secondaire].

### (c) Trouver ProjectName
1. **Le dossier frère de `Engine\` qui contient `Binaries\Win64\` et `Content\Paks\`** — fiabilité **haute** (c'est
   `FApp::GetProjectName()` tel qu'empaqueté).
2. Le .exe `<Projet>-Win64-Shipping.exe` (ou `-Win64-Test.exe`, `-Win64-DebugGame.exe` ; le suffixe dit aussi le type
   de build, donc où chercher `Saved`) — haute, à croiser avec 1.
3. Le .exe racine `<Projet>.exe` — faible (renommable).
4. Confirmation : `%LOCALAPPDATA%\<Projet>\Saved\` existe et contient `SaveGames\` (ou `Config\`, `Logs\`).

### (d) Pièges
- `Saved\` mêle **`SaveGames\`** (parties), **`Config\Windows\`** ou **`Config\WindowsNoEditor\`** (réglages ;
  `GameUserSettings.ini`), **`Logs\`**, **`Crashes\`** (lourds, inutiles) : sauvegarder `SaveGames\` (et
  éventuellement `Config\`), jamais `Logs\` ni `Crashes\`.
- Nom de projet générique ou de code (`Game`, `Client`, `Prototype`, `Project`) partagé par plusieurs jeux :
  `%LOCALAPPDATA%\Game\` peut servir à deux jeux. Croiser avec le contenu.
- Les deux emplacements (Shipping et non-installé) peuvent coexister si le jeu a été lancé des deux façons.
- Un `.sav` peut exister sans `SaveGames\` (moteur maison au-dessus d'Unreal) : ne rien conclure d'un dossier vide.

---

## 5. Godot 3 / 4

### (a) Marqueurs sûrs
- un **`<Jeu>.pck`** à côté de `<Jeu>.exe` (même nom de base) [code `project_settings.cpp` : cherche
  `<dossier du .exe>\<nom du .exe>.pck`, puis dans le dossier courant] ;
- **ou un pck intégré au .exe** : les 4 derniers octets du .exe valent `GDPC`, un entier 64 bits situé 12 octets
  avant la fin donne la taille du pck, et l'en-tête `GDPC` se retrouve à `fin − taille − 8` [code
  `file_access_pack.cpp`] ;
- un pck commence par la signature **`GDPC`** suivie de la version du format et de la version du moteur
  (majeur.mineur.correctif) ; les jeux C# ont en plus un dossier `data_<Jeu>_windows_x86_64\` [à vérifier].

### (b) Emplacements [doc *Data paths*, Godot 3.6 et stable ; code `core/os/os.cpp`]
- par défaut : **`%APPDATA%\Godot\app_userdata\<nom du projet>\`** ;
- si `application/config/use_custom_user_dir` est vrai : **`%APPDATA%\<custom_user_dir_name>\`**, ou
  **`%APPDATA%\<nom du projet>\`** si `custom_user_dir_name` est vide ; ce nom **peut contenir des `/`**
  (`Studio/Jeu`) ;
- nom vide : **`%APPDATA%\Godot\app_userdata\[unnamed project]\`** [code].
- Nettoyage du nom [code `OS::get_safe_dir_name`] : espaces en bord retirés ; `: * ? " < > |` remplacés par `-` ;
  `/` et `\` remplacés par `-` pour le nom du projet (gardés pour `custom_user_dir_name`, où `..` devient `-`) ;
  points finaux retirés ; `.` → `dot`, `..` → `twodots`. **Frogtend doit appliquer la même règle.**
- Le mode « autonome » (`._sc_`) ne vaut que pour l'éditeur, pas pour un jeu exporté [doc].

### (c) Lire le nom du projet dans le pck, sans l'exécuter — oui, c'est faisable
1. Lire l'en-tête du pck [code `file_access_pack.cpp`] : `GDPC`, version du format, version du moteur, drapeaux
   (`PACK_DIR_ENCRYPTED`, `PACK_REL_FILEBASE`…), base des fichiers, puis (format 3 et 4) le décalage du répertoire ;
   le format 2 a 16 mots réservés à la place ; le code actuel accepte les formats 2 à 4 (Godot 3 écrit un format
   plus ancien, de même forme générale [à vérifier]).
2. Pour chaque entrée : longueur du chemin, chemin, décalage, taille, MD5 (16 octets), drapeaux. Trouver
   **`res://project.binary`** (le chemin peut être stocké sans `res://` selon la version [à vérifier]).
3. Lire `project.binary` [code `project_settings.cpp`] : `ECFG`, un entier 32 bits = nombre de réglages, puis pour
   chacun : longueur 32 bits + clé UTF-8, longueur 32 bits + valeur encodée en *Variant*. Chercher
   `application/config/name`, `application/config/use_custom_user_dir`, `application/config/custom_user_dir_name`.
   Une valeur chaîne = type `STRING` (4), longueur 32 bits, octets UTF-8 complétés à un multiple de 4
   [à vérifier : identifiant du type selon Godot 3/4]. Booléen : type `BOOL`.
4. Fiabilité **haute** si le pck n'est pas chiffré ; **impossible** si le répertoire ou le fichier est chiffré
   (drapeau `PACK_DIR_ENCRYPTED` ou drapeau de chiffrement de l'entrée) → repli par le disque : chercher dans
   `%APPDATA%\Godot\app_userdata\` un dossier au nom du jeu, et le proposer sans conclure.
5. Un `project.godot` en clair (jeu non empaqueté) se lit comme un INI : section `[application]`, clé
   `config/name="…"`.
- Attention : `application/config/name` peut être **localisé** (`config/name_localized`) dans Godot 4 ; le dossier
  utilise la valeur de base [à vérifier].

### (d) Pièges
- **Nom générique ou vide** → `[unnamed project]`, ou un nom de gabarit, partagé par plusieurs jeux.
- `custom_user_dir_name` avec `/` : arborescence `%APPDATA%\Studio\Jeu`, et `%APPDATA%\Studio\` peut contenir
  d'autres jeux du même studio — ne sauvegarder que le dernier niveau.
- `user://` mêle parties, réglages, **`logs\`**, **`shader_cache\`** / `vulkan\` (caches lourds) : les exclure.
- Le nom du .exe ≠ nom du projet.

---

## Tableau de synthèse

| Moteur | Marqueur décisif | Parties | Nom variable lu dans | Fiabilité |
|---|---|---|---|---|
| Ren'Py | `renpy\` + `game\*.rpyc/.rpa` | `%APPDATA%\RenPy\<save_directory>` **et** `<jeu>\game\saves` | `options.rpy`, sinon octets d'un `.rpyc` (zlib) ou `.rpa` | haute (rpy) / moyenne |
| RPG Maker XP/VX/Ace | `Game.ini` + `RGSS*.dll` / `Game.rgss*` | `<jeu>\Save*.rxdata/rvdata/rvdata2` | — | haute |
| RPG Maker MV | `package.json` + `www\js\rpg_managers.js` | `<jeu>\www\save\` ou `<jeu>\save\` (`*.rpgsave`) ; rarement `%LOCALAPPDATA%\<name>\User Data\Default\Local Storage` | `package.json` → `name` | haute |
| RPG Maker MZ | `package.json` + `js\rmmz_managers.js` | `<jeu>\save\*.rmmzsave` ; rarement `…\User Data\Default\IndexedDB` | `package.json` → `name` | haute |
| Unity | `<Jeu>_Data\` + `UnityPlayer.dll` | `LocalLow\<company>\<product>` (conseillé, non garanti) ; PlayerPrefs au registre | `<Jeu>_Data\app.info` (2 lignes) | haute si le dossier existe |
| Unreal 4/5 | `Engine\` + `<Projet>\Content\Paks\` | `%LOCALAPPDATA%\<Projet>\Saved\SaveGames` (Shipping) ou `<jeu>\<Projet>\Saved\SaveGames` | nom du dossier `<Projet>` / `<Projet>-Win64-Shipping.exe` | haute |
| Godot 3/4 | `<Jeu>.pck` ou `GDPC` en fin de .exe | `%APPDATA%\Godot\app_userdata\<nom>` ou `%APPDATA%\<dossier perso>` | `project.binary` dans le pck (non chiffré) | haute / nulle si chiffré |

Outil de confirmation conseillé (source secondaire) : le manifeste de **Ludusavi** recense les emplacements jeu
par jeu (jetons `<winAppData>`, `<winLocalAppData>`, `<winLocalAppDataLow>`, `<base>`…) ; il sert à vérifier nos
déductions, pas à les remplacer — et sa reprise dans Frogtend serait un autre lot (à noter dans `IDEES.md` si Seb
le souhaite).

---

## Sources

### Officielles (documentation)
- Ren'Py, `config.save_directory` : https://www.renpy.org/doc/html/config.html
- Ren'Py, parties et retour arrière : https://www.renpy.org/doc/html/save_load_rollback.html
- Ren'Py, données persistantes : https://www.renpy.org/doc/html/persistent.html
- Ren'Py, distribution (`build.executable_name`, archives) : https://www.renpy.org/doc/html/build.html
- Ren'Py, journal des versions 6.11–6.99 (`script_version.txt`, format `.rpyc`) : https://www.renpy.org/doc/html/changelog6.html
- Unity, `Application.persistentDataPath` : https://docs.unity.com/en-us/engine/current/script-reference/unityengine/application/persistentdatapath
  (ancienne adresse : https://docs.unity3d.com/ScriptReference/Application-persistentDataPath.html)
- Unity, `PlayerPrefs` : https://docs.unity.com/en-us/engine/current/script-reference/unityengine/playerprefs
- Unreal, *Saving and Loading Your Game* : https://dev.epicgames.com/documentation/en-us/unreal-engine/saving-and-loading-your-game-in-unreal-engine
- Unreal, `UGameplayStatics::SaveGameToSlot` : https://dev.epicgames.com/documentation/unreal-engine/API/Runtime/Engine/UGameplayStatics/SaveGameToSlot?lang=en-US
- Unreal, `FPaths` : https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Runtime/Core/FPaths
- Godot, *Data paths* (stable) : https://docs.godotengine.org/en/stable/tutorials/io/data_paths.html
- Godot 3.6, *Data paths* : https://docs.godotengine.org/en/3.6/tutorials/io/data_paths.html
- NW.js, `App.dataPath` : https://docs.nwjs.io/References/App/

### Officielles (code source)
- Ren'Py, emplacements des parties : https://github.com/renpy/renpy/blob/master/renpy/savelocation.py
- Ren'Py, `path_to_saves` : https://github.com/renpy/renpy/blob/master/renpy.py
- Ren'Py, format `.rpyc` : https://github.com/renpy/renpy/blob/master/renpy/script.py
- Ren'Py, archives `.rpa` : https://github.com/renpy/renpy/blob/master/renpy/loader.py
- Ren'Py, jetons de signature : https://github.com/renpy/renpy/blob/master/renpy/savetoken.py
- Ren'Py, `MultiPersistent` : https://github.com/renpy/renpy/blob/master/renpy/persistent.py
- Ren'Py, `savegame_suffix` : https://github.com/renpy/renpy/blob/master/renpy/loadsave.py
- RPG Maker MV (dépôt officiel KADOKAWA) : https://github.com/rpgtkoolmv/corescript/blob/master/js/rpg_managers/StorageManager.js
- Godot, `project.binary` et recherche du pck : https://github.com/godotengine/godot/blob/master/core/config/project_settings.cpp
- Godot, format pck : https://github.com/godotengine/godot/blob/master/core/io/file_access_pack.cpp
- Godot, `user://` et nettoyage du nom : https://github.com/godotengine/godot/blob/master/core/os/os.cpp
- Godot, `%APPDATA%` sous Windows : https://github.com/godotengine/godot/blob/master/platform/windows/os_windows.cpp

### Secondaires (pour confirmer seulement)
- Forum officiel Epic, `Saved` dans `%LOCALAPPDATA%` (Shipping, `-SaveToUserDir`, `-userdir`) : https://forums.unrealengine.com/t/how-to-point-saved-directory-on-pc-to-localappdata/2689924
- Forum officiel Epic, emplacement des `.sav` en Shipping : https://forums.unrealengine.com/t/where-do-sav-files-go-when-shipping-the-build-via-project-launcher/765654
- Forum officiel RPG Maker, `KADOKAWA/RPGMV` et `%LOCALAPPDATA%` : https://forums.rpgmakerweb.com/threads/save-files-saved-to-local-data-folders.56229/
- Forum officiel RPG Maker, MZ en TypeScript (extraits de `rmmz_managers.js`) : https://forums.rpgmakerweb.com/index.php?threads/rpg-maker-mz-typescript-edition.124983/
- Référence StorageManager MZ (tonbijp) : https://github.com/tonbijp/RPGMakerMZ/blob/master/Reference/StorageManager.md
- NW.js, `User Data\Default` depuis 0.13 : https://github.com/nwjs/nw.js/issues/4696
- unrpyc (format `.rpyc` 2) : https://github.com/CensoredUsername/unrpyc/issues/45 et https://github.com/CensoredUsername/unrpyc/issues/76
- `script_version.txt` (contenu) : https://steamcommunity.com/sharedfiles/filedetails/?id=3486572030
- Détection des jeux RPG Maker : https://guides.nbhzvn.com/en/detect-rpg
- Éditeur de parties RPG Maker / Ren'Py (emplacements) : https://github.com/truongthang2211/RPGSaveEditor
- Guide savegame.org (RPG Maker) : https://savegame.org/guides/how-to-edit-an-rpg-maker-save/
- Unity, chemins (kimsama) : https://github.com/kimsama/Unity-Paths
- Unity, `app.info` lu par des outils libres : https://github.com/MrUSBEN/FolderForensics , https://github.com/Dom428/hallow-vault , https://github.com/son242537-gif/agvn-player/pull/53

## Restant à vérifier sur de vrais jeux (avant de coder)
1. Ren'Py : un `.rpyc` décompressé contient-il bien `save_directory` suivi de la chaîne en clair (6.x, 7.x, 8.x) ?
   `log.txt` cite-t-il le dossier ? Ancienneté de l'écriture dans `game\saves`.
2. Unity : forme exacte d'`app.info` (fin de ligne, encodage), et son absence éventuelle dans de vieux builds.
3. Unreal : noms des `.pak` selon la version ; dossier `Config\Windows` contre `WindowsNoEditor`.
4. Godot : format pck de Godot 3, forme du chemin `project.binary` dans le répertoire, encodage des Variant
   chaîne/booléen en 3 et 4.
5. RPG Maker : noms des `Save*` des scripts d'origine XP/VX/Ace, valeur par défaut de `name` dans le `package.json`
   de MZ, emplacement mkxp-z.
