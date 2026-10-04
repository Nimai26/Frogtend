# À tester par Seb à son retour

> Tenu à jour au fil du travail. Chaque ligne dit **quoi faire** et **ce qu'on doit voir**. Coche (`[x]`) ce qui
> marche ; pour ce qui ne marche pas, un mot ou une capture suffit.

## Par où commencer (parcours conseillé, 03/10)
Dans cet ordre, chaque étape prépare la suivante ; le détail de chacune est plus bas, dans sa section.
0. ✅ **Fait le 03/10** : Asura's Wrath décompressé et lancé dans RPCS3 — « ça fonctionne nickel » (Seb).
1. **Installer la dernière version** (⚙ Options ▸ À propos ▸ Chercher une mise à jour).
2. **⚙ Options ▸ Emplacements ▸ Les autres dossiers de Frogtend** : choisir le dossier des **abris** (ton abri de
   Dune y est rangé) et celui des **outils** — section « Pas de pieuvre ».
3. **📥 Importer ▸ Fichiers ROM** sur ton dossier NES (laisser les fichiers où ils sont) — sections « Importer » et
   « Une fiche par jeu ».
4. **⚙ Options ▸ Comptes ▸ 🏆 RetroAchievements**, puis **🎮 Connecter mes émulateurs** ; regarder le panneau d'un jeu
   NES, Sega CD, Dreamcast, 3DO, PC Engine, GameCube ou Wii — sections « Les succès ».
5. **Jouer** à un jeu NES par RetroArch : il doit te saluer par ton nom (succès) ; essayer **Pause/Attn** (menu en
   jeu).
6. **Comptes des boutiques** : régénérer la clé Steam puis l'importer ; lire GOG Galaxy ; se connecter à Epic, GOG,
   Prime Gaming, PlayStation et récupérer les jeux offerts.
7. Le reste à ton rythme : Taodbox et manettes (sonde), MAME, DOS, Cheat Engine, assistant, consoles récentes.

## Décompresser pour jouer (0.43.0) — ✅ RÉUSSI le 03/10
> **Pourquoi c'est bloquant (décision de Seb, 03/10) :** tout le travail sur la PS3 (jeux, DLC déjà livrés, succès)
> repose sur une hypothèse : **RPCS3 démarre directement un .iso PS3 déchiffré**. La presse le dit, mais personne ne
> l'a vu marcher chez toi. On n'avance sur rien d'autre (DLC Xbox 360 et Vita, profils de manette par jeu, manettes
> PlayStation dans Dolphin) avant ce résultat.
>
> **Ce que la réponse implique :**
> - ✅ **Le jeu démarre** : l'approche est validée. Les 6 autres jeux PS3 peuvent être décompressés (≈ 208 Go en
>   tout : vérifier la place sur E: avant), et on reprend les lots en attente.
> - ⚠️ **RPCS3 refuse l'.iso** : d'abord, mettre RPCS3 à jour (⚙ Options ▸ Émulateurs) — la lecture des .iso est un
>   ajout récent. S'il refuse encore, Frogtend changera d'approche : extraire le CONTENU de l'.iso (le dossier
>   `PS3_GAME`, format que RPCS3 lit depuis toujours) au lieu de l'.iso seul. Frogtend sait déjà lire l'intérieur d'un
>   .iso. Ce changement prendra une livraison, et le même essai sera à refaire.
> - ❌ **Le jeu démarre puis plante ou affiche un écran noir** : c'est un souci de RPCS3 ou de ce jeu, pas de
>   Frogtend. Note ce que dit RPCS3 (ou envoie une capture) ; on regardera sa liste de compatibilité.
>
> Dans tous les cas, l'archive .zip reste intacte : rien n'est perdu, et le dossier décompressé peut être supprimé
> par toi si l'essai échoue.

- [x] Importe tes jeux PS3 (`E:\Games\Playstation 3`, « zip ») et sélectionne **Asura's Wrath** (le plus petit :
      6,7 Go). **Attendu :** « 🗜 Ce jeu est rangé dans une archive » et le bouton **🗜 Décompresser pour jouer**.
- [ ] Clique : la fenêtre annonce la taille, le dossier (`E:\Games\Playstation 3\Asuras Wrath (Europe)
      (EnJaFrDeEsIt)`), la place libre et la durée. Dis oui : la barre avance. **Attendu à la fin :** le message
      « est prêt », le zip toujours là, l'.iso dans le nouveau dossier.
- [x] ✅ **▶ Jouer** : RPCS3 démarre le jeu depuis l'.iso (60 i/s, Seb 03/10). **Dis-moi lequel des trois cas ci-dessus tu as obtenu**
      (démarre / refuse l'.iso / plante), avec la version de RPCS3 (affichée dans sa fenêtre).
- [ ] Le bloc 🏆 RetroAchievements ne parle plus de « version zippée ».
- [ ] Un jeu NES zippé ne propose PAS de décompresser (les émulateurs lisent les cartouches zippées).

## La manette dans RPCS3 (0.44.4 — la 0.44.3 cassait tout : nom mal lu par RPCS3)
- [ ] Manette Xbox **éteinte**, clique ▶ Jouer sur Asura's Wrath, PUIS allume la manette : en 2 secondes elle marche
      dans le jeu (✕ = A, ◯ = B, Start = Start), sans relancer.
- [ ] Une manette PS4 ou PS5 (si tu en as une, branchée en USB ou Bluetooth) : elle marche aussi.
- [ ] ⚙ Gérer le jeu ▸ 🎮 Commandes ▸ Clavier : à la partie suivante, le clavier marche (✕ = X, Start = Entrée).

## Prêt à jouer (0.44.0)
- [ ] **Une console sans émulateur** (par exemple la **Sega CD** : importe `E:\Games\Sega CD`) : après l'import,
      UNE fenêtre « 🎮 Préparer Sega CD pour jouer ? » dit l'émulateur, sa taille, le dossier, la manette. **Tout
      préparer** : il s'installe sans autre question ; un jeu se lance ensuite par ▶ Jouer.
- [ ] Dis **non** une fois : rien n'est installé ; ▶ Jouer sur un jeu de cette console repose la question.
- [ ] **RPCS3 et une manette Xbox** : branche-la AVANT de lancer Asura's Wrath : elle marche (✕ = A, ◯ = B). Débranche-la
      et relance : le clavier marche (✕ = X). *(Le cas « manette branchée » n'a pas pu être essayé ici : aucune
      manette n'était branchée.)*
- [ ] **💿 Micrologiciel** : sur la ligne de RPCS3, la fenêtre dit « Installé : version 4.91 ». Avec un PS3UPDAT.PUP
      plus récent (4.93), l'installation se fait en quelques secondes, sans aucune fenêtre de RPCS3, puis « ✅
      Micrologiciel PS3 4.93 installé ».

## Régler un émulateur pour n'importe quel système (0.43.4)
- [ ] **⚙ Options ▸ 🕹 Émulateurs ▸ Régler un autre système…** : en tête, **Sony Playstation 3 — 7 jeu(x)** ; plus
      bas, toutes les autres consoles (même sans jeux).

## Import sans rien taper (0.43.3)
- [x] **📥 Importer ▸ Fichiers ROM** : aucune fenêtre noire ne s'ouvre.
- [ ] **📂 Choisir le dossier** `E:\Games\Playstation 3`. **Attendu :** plateforme « Sony Playstation 3 » déjà
      choisie ; « Types de fichiers trouvés » : .zip (77) coché, .mkv (1) pas coché.
- [ ] **🔍 Chercher** : 7 jeux, et « 70 contenus additionnels » écartés. **➕ Ajouter 7 jeux** : la question
      « Où garder ces 7 jeux ? » arrive en moins d'une seconde ; **📌 Les laisser où ils sont** ; les 7 fiches
      apparaissent dans la ludothèque, rubrique Sony Playstation 3.
- [ ] Un autre dossier (ex. `E:\Games\Sega CD`) : la plateforme est devinée, .cue coché, .bin pas coché.

## Corrections de la 0.43.2
- [ ] Un jeton refusé (ou une autre erreur) affiche son vrai motif, plus jamais « motif inconnu ».
- [ ] Fenêtre étroite : pas de barre de défilement autour de la fenêtre ; les onglets du haut défilent à la molette ;
      Réduire / Agrandir / Fermer toujours visibles.

## Corrections de la 0.43.1 (à voir au passage, pas d'essai spécial)
- [ ] **⚙ Options ▸ Comptes ▸ 🏆 RetroAchievements ▸ Oublier** (sur un profil d'essai) : ensuite, dans le dossier de
      RetroArch, `Profils\<profil>\frogtend.cfg` ne contient plus de ligne `cheevos_token`. Puis reconnecte-toi.
- [ ] Un jeu **MAME** zippé ne propose PAS « 🗜 Décompresser pour jouer ».
- [ ] La fenêtre de « 🗜 Décompresser pour jouer » affiche bien la **place libre** du disque.
- [ ] Switch : après « Rendre disponible », Eden voit les contenus (les dossiers sont maintenant écrits avec des « / »
      dans ses réglages). Le panneau les montre ✅ ensuite.

## L'aide (0.43.0)
- [ ] **❓ Aide** : le sommaire en haut mène aux rubriques (versions, succès, DLC, jeux offerts, en jeu, Taodbox,
      rangement). Dis-moi ce qui n'est pas clair pour quelqu'un qui découvre Frogtend.

## Machine
- [x] Disque D: plein (670 Mo libres le 02/10) : **Seb a fait de la place** (02/10). Le cache de construction de
      Frogtend est maintenant allégé (il avait atteint 61 Go).

## Tout de suite : installer la dernière version
- [ ] Frogtend propose la mise à jour au démarrage (ou ⚙ Options ▸ À propos ▸ Chercher une mise à jour).
      **Attendu :** la version la plus récente de [NOTES-DE-VERSION.md](NOTES-DE-VERSION.md).

## Triches, mods et émulateurs via Firehouse (0.17.1) — routes EN SERVICE (Firehouse 2.26.0)
- [ ] Fiche de **Dune** ▸ **🎯 Triches et mods**. **Attendu :** une note (pas encore de tables Cheat Engine), et
      **🌐 Page des mods et correctifs** (PCGamingWiki). Sur Dune installé, Frogtend propose d'abord une **copie du
      jeu** (taille annoncée) ; si tu refuses, un avertissement.
- [ ] Un jeu d'émulateur avec des codes (PS1, PS2, SNES…) : « 🎯 Ajouter » pose le fichier dans le dossier de ton
      profil. Si la correspondance n'est pas sûre, Frogtend le dit (charger le fichier soi-même dans l'émulateur).
- [ ] Switch ▸ ➕ Ajouter un émulateur : **Eden**, **Citron Neo** et **Ryujinx (Ryubing)** apparaissent.
      **Citron Neo** s'installe tout seul (vérifié par son empreinte). **Ryubing** : sa source ne répond pas, Frogtend
      propose d'ouvrir sa page officielle.

## Les succès (0.32.0)
- [ ] ⚙ Options ▸ Comptes ▸ **🏆 RetroAchievements** : ton nom et ta clé d'API Web (site ▸ Settings ▸ Keys).
      **Attendu :** « compte vérifié et enregistré » ; la clé n'apparaît plus jamais.
- [ ] Sélectionne un jeu NES importé (par exemple Super Mario Bros) : le panneau montre **🏆 RetroAchievements**,
      « ✅ Ta version est compatible » et tes succès. Sur un jeu dont ta version n'est pas reconnue : Frogtend dit
      laquelle l'est (ou qu'elle manque).
- [ ] Un jeu Steam : **🏆 Succès Steam x / y**.
- [ ] (0.33.0) ⚙ Options ▸ Comptes ▸ 🏆 RetroAchievements ▸ **🎮 Connecter mes émulateurs** (ton mot de passe, une
      fois). **Attendu :** « Émulateurs : ✅ ». Lance un jeu NES par RetroArch : la notification RetroAchievements de
      RetroArch te salue par ton nom, et un succès débloqué apparaît sur le site.
- [ ] Un jeu PlayStation par DuckStation : il demande de te connecter la 1re fois ; connecte-toi. Ensuite, joue avec un
      autre profil (sans compte), puis reviens au tien : **tu es toujours connecté**, l'autre profil ne l'était pas.

## Wii U : ce que contient un .wua (0.42.0)
- [ ] Importe tes jeux Wii U (`E:\Games\Nintendo Wii U`, « wua ») et sélectionne **Adventure Time Explore the
      Dungeon** : **📦 Contenus additionnels** montre « Mise à jour v16 » ✅, « inclus dans le fichier .wua ». Un jeu avec
      DLC en montre aussi (12 de tes jeux en ont).

## Mises à jour et DLC Switch dans Eden (0.41.0)
- [ ] Il faut Eden réglé pour la Switch. **Ferme Eden** avant d'activer des contenus (il réécrit ses réglages en se
      fermant).
- [ ] Importe tes jeux Switch (`D:\LaunchBox\Games\Nintendo Switch`, « xci, nsp ») : les .nsp de mises à jour et DLC
      ne sont pas pris pour des jeux.
- [ ] Ajoute `D:\LaunchBox\Games\Nintendo Switch Maj & DLC` comme source de la Switch (⚙ Options ▸ Emplacements),
      puis sélectionne **Fire Emblem Three Houses** : **📦 Contenus additionnels : 7 disponible(s)** (6 DLC + la mise à
      jour). Coche, **Rendre disponible** : le dossier est ajouté à Eden. **Attendu dans Eden :** la colonne
      « Add-Ons » du jeu montre la mise à jour et les DLC ; le jeu démarre en v1.2.0.
- [ ] Un jeu au nom bizarre (« v-luigi3 », c'est Luigi's Mansion 3) : ses contenus ne se rattachent que si le jeu
      porte son identifiant [0100…] dans son nom (un .xci ne le dit pas) — dis-moi si c'est gênant.

## DLC et contenus additionnels PS3 (0.40.0)
- [ ] Il faut RPCS3 réglé pour la PS3 (⚙ Options ▸ Émulateurs).
- [ ] 📥 Importer ▸ Fichiers ROM sur `E:\Games\Playstation 3`, plateforme « Sony Playstation 3 », extension « zip ».
      **Attendu :** 7 jeux, et le message « 📦 70 contenu(s) additionnel(s) trouvé(s)… ce ne sont pas des jeux ».
- [ ] Sélectionne **God of War - Ascension** : le panneau montre **📦 Contenus additionnels : 13 disponible(s)**.
      Ouvre, coche-en 1 ou 2, **📦 Installer** : la confirmation annonce la taille ; dis oui. **Attendu :** « ✅ …
      installé(s) » ; ils passent en ✅. Dans RPCS3, le contenu est là (avatar/DLC visible dans le jeu).
- [ ] Change de profil, même jeu : les contenus avec **licence** ne sont pas ✅ pour lui ; s'il les installe, le paquet
      n'est pas réinstallé, seule SA licence est posée (rapide).
- [ ] Rien d'extrait ne reste : `<données de Frogtend>\travail\contenus` est vide ou absent après.

## Succès : PS3, Jaguar CD, PSP compressé (0.39.0)
- [ ] Un jeu PS3 de `E:\Games\Playstation 3` importé tel quel (.zip) : le panneau dit **« Ta version est zippée :
      décompresse-la »**. Décompresse-en un (l'.iso), importe l'.iso : **🏆 RetroAchievements** dit s'il est compatible.
      (Vérifié le 03/10, en lecture seule dans les zips : tes **7 jeux PS3 sont DÉCHIFFRÉS** — EBOOT.BIN lisible
      (« SCE ») là où un disque d'origine serait chiffré — donc lisibles par RPCS3 une fois décompressés.)
- [x] **« Animaniacs - The Great Edgar Hunt »** (jeu **PlayStation 2**, rangé par erreur dans `E:\Games\Playstation 3`) :
      **déplacé dans `E:\Games\Playstation 2`** (dossier créé) à ta demande le 03/10 ; empreinte SHA-256 identique
      avant/après ; aucune entrée LaunchBox ne pointait vers l'ancien emplacement.
- [ ] Les 70 autres zips sont des **DLC et avatars** (.pkg, avec leur licence .rap pour 50 d'entre eux) : ils
      s'installent dans RPCS3 (Fichier ▸ Installer des paquets/raps). Dis-moi si tu veux que Frogtend le fasse.
- [ ] Un PSP en .cso, ou une Jaguar CD en .cue (si tu en as) : le panneau ne montre jamais d'erreur.

## Succès : PS2, PSP, DS, PC Engine CD, PC-FX, Neo Geo CD (0.38.0)
- [ ] Tu n'as pas de jeux de ces consoles sur E: : je n'ai pu les vérifier qu'avec des images fabriquées. **Dès que tu
      en as un** (Firehouse, ou ailleurs : .iso/.chd pour PS2, .iso/.pbp pour PSP, .nds pour la DS, .cue/.chd pour
      PC Engine CD, PC-FX et Neo Geo CD), importe-le et regarde **🏆 RetroAchievements** dans son panneau.
      **Attendu :** « compatible », « pas reconnue, mais … l'est » ou « version manquante » — jamais une erreur.

## Succès : 3DO, PC Engine, GameCube et Wii (0.37.0)
- [ ] Importe (📥 Importer ▸ Fichiers ROM) et sélectionne un jeu de chaque : **3DO** (`E:\Games\3DO`, « chd »),
      **PC Engine** (`E:\Games\NEC PC Engine TurboGrafx 16`, plateforme « NEC TurboGrafx-16 », « 7z »),
      **GameCube** et **Wii** (« rvz »). **Attendu :** **🏆 RetroAchievements** dit si ta version est compatible.
- [ ] Un jeu Wii : la 1re fois, « ~30 s » s'affiche puis le résultat ; **ferme et rouvre Frogtend**, re-sélectionne le
      jeu : le résultat est immédiat (empreinte gardée).

## GOG, Prime Gaming et succès Galaxy (0.36.0)
- [ ] 🛒 Boutiques ▸ 🎁 ▸ **🟣 GOG ▸ 🔑 Se connecter**, puis **🎁 Récupérer**. **Attendu :** « pas de jeu offert en ce
      moment » ou « 1 jeu récupéré » (vérifie dans ta bibliothèque GOG). Dis-moi si GOG t'a inscrit à sa lettre
      d'information et si tu veux que Frogtend t'en désinscrive tout seul.
- [ ] **📦 Prime Gaming ▸ 🔑 Se connecter** (compte Amazon), puis **🎁 Récupérer** : les offres « sur Amazon » sont
      prises ; pour les autres (codes, comptes à relier), la page s'ouvre avec le nombre à finir.
- [ ] Un jeu GOG importé par Galaxy : le panneau montre **🏆 Succès x / y (GOG Galaxy)** (pas pour Epic : Galaxy ne les
      a pas).

## Succès des jeux sur CD (0.34.0)
- [ ] Importe tes jeux Sega CD (`E:\Games\Sega CD`, extension « cue », sous-dossiers cochés) ; sélectionne Sonic CD :
      **🏆 RetroAchievements** dit si ta version est compatible. (Les 56 empreintes se calculent déjà chez toi.)
- [ ] (0.35.0) Un jeu Dreamcast (.chd) : **🏆 RetroAchievements** dit si ta version est compatible (les 135 empreintes
      se calculent chez toi ; la 1re fois ~1 s, ensuite c'est immédiat).

## Une fiche par jeu, plusieurs versions (0.31.0)
- [ ] 📥 Importer ▸ Fichiers ROM sur `E:\Games\Nintendo Entertainement System` : **attendu ~828 fiches** au lieu de
      1 086 lignes (les versions d'un même jeu réunies).
- [ ] Une fiche avec plusieurs versions (par exemple « Batman ») : le panneau montre « Versions : 3 (EU, US/EN, US/EN) » ;
      ▶ Jouer lance l'européenne ; **▶ Jouer avec…** propose les autres ; ⚙ Gérer le jeu ▸ **📀 Version** la change.
- [ ] Un jeu sur plusieurs disques (PS1) : une seule version, pas une par disque.

## Pas de pieuvre (0.30.0)
- [ ] Au démarrage, les médias de Dune passent de `%APPDATA%` à `<dossier de Dune>\Frogtend\medias` (copie vérifiée).
      **Attendu :** la jaquette et la fiche de Dune s'affichent toujours, hors ligne aussi.
- [ ] ⚙ Options ▸ Emplacements ▸ **Les autres dossiers de Frogtend** : choisis le **dossier des abris** de parties.
      **Attendu :** « 1 abri(s) rangé(s) » (celui de Dune, 21 Ko) et plus rien dans `%APPDATA%\…\sauvegardes`.
- [ ] Choisis aussi le **dossier des outils** ; ⬇ Installer Cheat Engine doit y aller (plus dans les émulateurs).
- [ ] Un jeu installé ▸ 🎯 Triches et mods ▸ accepte la **copie avant mod** : elle va dans
      `<dossier du jeu>\Frogtend\copies\avant-mod-<date>\` (plus à côté du dossier du jeu).
- [ ] 📥 Importer ▸ Fichiers ROM, sur un petit dossier : **« Les laisser où ils sont »** → le dossier apparaît dans
      Emplacements, sous le système ; refais avec **« Les copier »** → copie dans l'emplacement du système, originaux
      intacts.
- [x] `%APPDATA%\com.frogtend.app` et `com.frogtend.appmedia` (ancien essai de mars 2026, 3,3 Go) : supprimés à ta
      demande le 02/10.

## Installer un jeu DOS (0.29.0)
- [ ] Il faut un DOSBox réglé pour MS-DOS (⚙ Options ▸ Émulateurs).
- [ ] 📥 Importer ▸ **Installer un jeu DOS** : choisis le dossier d'un CD (ou une image .iso/.cue, ou une disquette
      .img), un titre, l'endroit. **Attendu :** DOSBox s'ouvre sur D: (ou A:) ; tape INSTALL, installe sur C:, puis EXIT.
      Frogtend propose alors le programme du jeu ; ajoute-le et ▶ Jouer : le jeu démarre directement dans DOSBox.

## MAME Arcade Full Set (0.28.0)
- [ ] 📥 Importer ▸ **MAME Arcade Full Set** : dossier `E:\Games\MAME`, liste `D:\LaunchBox\Metadata\MAME.xml`,
      🔍 Trier. **Attendu :** ~4 526 jeux retenus, et le détail des écartés (mécaniques, hors arcade, non jouables…).
- [ ] (Plus tard) Le lancement des jeux MAME se réglera avec le MAME et les packs **fournis par Firehouse** (versions
      assorties, avec ou sans CHD) : rien à essayer de ce côté-là pour l'instant.

## Importer tes jeux du disque (0.27.0)
- [ ] 📥 Importer ▸ **Fichiers ROM** : dossier `E:\Games\Nintendo Entertainement System`, plateforme « Nintendo
      Entertainment System », extensions « nes ». **Attendu :** ~1 086 jeux listés (titres sans « (U) », « [!] »).
      Décoche-en quelques-uns, ajoute : ils apparaissent dans la ludothèque (filtre Boutique ▸ « Importés de mon
      disque »), et ▶ Jouer les lance par ton émulateur NES.
- [ ] Un système sur CD (`E:\Games\3DO`, extensions « chd, cue ») : un jeu par disque, pas un par piste.
- [ ] **Jeux MS-DOS** : un dossier avec un sous-dossier par jeu ; vérifie le programme proposé pour chacun.
- [ ] **Jeux Windows** : choisis un .exe ; le jeu se lance par ▶ Jouer.
- [ ] ⚙ Gérer le jeu ▸ **Retirer de la ludothèque** : le jeu disparaît de Frogtend, **ses fichiers restent** (vérifie
      dans l'explorateur).

## PlayStation Plus : les jeux du mois (0.26.0)
- [ ] 🛒 Boutiques ▸ 🎁 ▸ **🔑 Se connecter à PlayStation** : clique « Se connecter » en haut du Store, connecte-toi,
      ferme la fenêtre.
- [ ] **🎮 Ajouter les jeux du mois** (quelques minutes, sans fenêtre). **Attendu :** « N jeu(x) ajouté(s) » ou « déjà
      dans ta bibliothèque ». Vérifie sur ta PS5 / l'appli PlayStation que les jeux du mois y sont. Si la fenêtre du
      Store s'ouvre à la place, dis-moi ce qu'elle montre (c'est là qu'on règle le script ensemble).
- [ ] Coche « J'ai PS Plus » si tu veux que ça se fasse tout seul chaque semaine.

## Menu « Importer » (0.25.0)
- [ ] Barre du haut ▸ **📥 Importer ▾** : la même liste que LaunchBox (13 entrées), toutes utilisables depuis la 0.29.0.
- [ ] **Jeux Epic Games** (puis Amazon, EA, Ubisoft, Xbox) : l'explication GOG Galaxy, et le nombre de jeux lus pour
      CETTE boutique. **Attendu chez toi :** Epic 43, Xbox 6, Ubisoft 4, EA 4 ; Amazon 0 → le message « compte sans
      doute pas relié » (relie-le dans Galaxy ▸ Paramètres ▸ Intégrations si tu veux tes jeux Prime Gaming).
- [ ] **❓ Aide** (barre du haut) et **⚙ Options ▸ Comptes ▸ GOG Galaxy** : l'explication est claire pour la famille ?

## Jeux offerts Epic (0.24.0)
- [ ] 🛒 Boutiques ▸ **🎁 Jeux offerts** : la liste de la semaine (System Shock 2, BURIED STARS jusqu'au 8 octobre).
- [ ] **🔑 Se connecter à Epic** : connecte-toi dans la fenêtre (page officielle), puis ferme-la.
- [ ] **🎁 Obtenir** sur un jeu. **Attendu :** « 🎁 … est à toi ! » sans rien faire ; sinon la page Epic s'ouvre
      (captcha, connexion) et tu finis toi-même. Me dire ce qui s'est passé (c'est l'essai du « B »).
- [ ] Case « Les obtenir tout seul » : au prochain démarrage (après 24 h), Frogtend le fait discrètement.

## Tes boutiques dans la ludothèque (0.23.0)
- [ ] Après « Lire mes jeux GOG Galaxy » (et/ou l'import Steam), **Ma ludothèque ▸ Windows**. **Attendu :** tes jeux
      de boutiques, en jaquettes, avec les autres.
- [ ] Filtres en haut : **Boutique** (Epic Games par exemple) et **Installés / Pas installés**.
- [ ] Un jeu de boutique : le panneau de droite propose ▶ Jouer (installé) ou ⬇ Installer, par Steam ou GOG Galaxy ;
      un double-clic fait pareil.
- [ ] Le **Catalogue Firehouse** ne montre PAS ces jeux.

## GOG Galaxy (0.22.0) — Epic, Xbox, Ubisoft, EA compris
- [ ] 🛒 Boutiques ▸ **⬇ Lire mes jeux GOG Galaxy**. **Attendu :** environ 324 jeux en jaquettes ; le menu « Toutes
      les boutiques » filtre GOG, Epic Games, Xbox, Ubisoft Connect, EA ; Cyberpunk 2077 et Horizon Zero Dawn
      marqués ✅ installés.
- [ ] Un clic sur un jeu : **GOG Galaxy s'ouvre sur sa page** (jouer ou installer).
- [ ] Taodbox : une section « GOG Galaxy » avec tes jeux installés.

## Steam (0.20.0)
- [ ] **Régénère d'abord ta clé d'API Steam** (elle est apparue dans une capture) : steamcommunity.com/dev/apikey.
- [ ] ⚙ Options ▸ **Comptes ▸ Steam** ▸ ✏ Régler mon compte Steam : ton URL personnalisée (NimaiTakahashi), puis
      ta NOUVELLE clé. **Attendu :** « Compte Steam vérifié et enregistré » ; la clé n'apparaît plus jamais.
- [ ] Barre du haut ▸ **Boutiques** ▸ ⬇ Importer mes jeux Steam. **Attendu :** ta liste de jeux Steam (tous ?),
      les installés marqués ✅. **▶ Jouer** lance le jeu par Steam ; **⬇ Installer** ouvre l'installation Steam.
- [ ] (0.21.0) Les jeux s'affichent en **jaquettes** Steam ; dans **Taodbox**, une section « Steam » avec tes jeux
      installés, lancés par A.
- [ ] Un autre profil : Boutiques ▸ Importer **sans réglage** → Frogtend demande d'abord le compte et la clé.

## Cheat Engine (0.18.0 – 0.19.1) — ton zip est sur Firehouse (Cheat Engine 7.7, 41,8 Mo)
- [ ] Fiche d'un jeu PC ▸ 🎯 Triches et mods ▸ **⬇ Installer Cheat Engine**. **Attendu :** installé depuis Firehouse.
- [ ] **🧰 Lancer Cheat Engine**, change un réglage (thème, raccourci), ferme-le. **Attendu :** la clé
      `HKCU\Software\Cheat Engine` n'est plus dans le registre ; tes réglages sont dans
      `<dossier Cheat Engine>\Profils\<toi>\cheatengine.reg` ; la clé d'avant Frogtend est dans
      `<dossier Cheat Engine>\.frogtend-sauvegardes\registre-avant-frogtend.reg`.
- [ ] Relance-le : **tes réglages reviennent**. Un autre profil : les siens (vierges au début).
- [ ] **Jeu lancé** (Dune par exemple) ▸ Pause/Attn ▸ **🧰 Cheat Engine** (ou 🎯 Triches et mods ▸ **🧰 Brancher
      Cheat Engine sur le jeu**). **Attendu :** Cheat Engine s'ouvre DÉJÀ branché sur le jeu (son nom en haut), sans
      passer par « Ouvrir un processus ». Avec les extensions de Firehouse : la zone de mémoire de l'émulateur réglée seule.

## Taodbox, le mode canapé (0.16.0)
- [ ] Barre du haut ▸ **🛋 Taodbox**. **Attendu :** plein écran, tout en grand, tes jeux en grandes jaquettes.
- [ ] Une manette branchée : la **croix** (ou le stick) déplace le cadre de sélection, **A** lance le jeu, **B**
      revient. **Attendu :** jamais bloqué dans un coin ; les fenêtres de confirmation se pilotent aussi.
- [ ] **👥 Changer de joueur**, puis ton profil et ton PIN **avec la manette** (pavé à l'écran).
- [ ] Lancer un jeu puis le quitter. **Attendu :** Taodbox revient tout seul au premier plan.
- [ ] **Sunshine/Apollo** (quand tu voudras) : dans Sunshine ▸ Applications ▸ Ajouter : nom « Taodbox », commande
      `"%LOCALAPPDATA%\Frogtend\Frogtend.exe" --taodbox` (vérifie le chemin de Frogtend.exe sur ton PC).
      **Attendu :** depuis Moonlight, « Taodbox » ouvre directement le mode canapé ; ⏻ Quitter ferme la session.
- [ ] Me dire quelles manettes marchent dans Taodbox (Xbox, PS4/PS5, Switch Pro, 8BitDo).

## Assistant jeux (0.14.0)
- [ ] Fiche de Dune ▸ **💬 Demander à l'assistant** ▸ « Comment je lance ce jeu ? ».
      **Attendu :** « L'assistant réfléchit… », puis une réponse mise en forme (titres, listes), et un bouton
      **▶ Lancer — …** (ou « 📦 Installer puis jouer »). Il demande confirmation avant de lancer.
- [ ] Barre du haut ▸ **Assistant** : une question générale. **Attendu :** une réponse, sans jeu précis.
- [ ] Changer de profil puis revenir : les conversations ont été oubliées (rien ne reste d'une personne).

## Consoles récentes (0.13.0)
Aucun de ces émulateurs n'a été installé ni lancé par l'agent. Pour chacun (si tu as un jeu de la console) :
- [ ] Lancer un jeu : Frogtend propose l'émulateur recommandé, annonce la taille, l'installe, le lance en plein
      écran. **Attendu :** le jeu démarre ; rien de nouveau dans Documents ni AppData (Options ▸ Émulateurs le dit).
- [ ] **Xbox 360 (Xenia)** : rien d'autre à fournir. Parties dans `Xenia Canary\Profils\<toi>\content`.
- [ ] **Xbox (xemu)** : au 1er lancement, xemu demande ses fichiers (MCPX, BIOS, disque dur, EEPROM) : les régler
      dans xemu. *Point à vérifier :* les réglages vont dans `xemu\Profils\<toi>\xemu.toml` ; un 2e profil devra
      peut-être les refaire (dis-moi).
- [ ] **PS3 (RPCS3)** : ⚙ Options ▸ Émulateurs ▸ 💿 Micrologiciel PS3… avec le PS3UPDAT.PUP du site de Sony.
      **Attendu :** RPCS3 l'installe. Puis un jeu : chaque profil a son compte RPCS3 (00000002, 00000003…).
- [ ] **Wii U (Cemu)** : clés de ta Wii U à mettre dans Cemu (keys.txt). Parties communes aux profils pour l'instant.
- [ ] **3DS (Azahar)** : un jeu .3ds/.cci ; écran tactile à la souris. Parties dans `Azahar\Profils\<toi>`.
- [ ] **PS Vita (Vita3K)** : micrologiciel de Sony à installer dans Vita3K ; un .vpk s'installe puis se lance.
- [ ] **Switch (Eden, 0.15.0)** : clés (prod.keys) et micrologiciel de TA Switch à mettre dans Eden (son menu).
      Puis, dans Eden, **créer un utilisateur au nom exact de ton profil Frogtend** (« Sebastien ») : Frogtend lance
      Eden avec cet utilisateur, donc tes parties à toi. *Point à vérifier :* le programme s'appelle bien `eden.exe`.
- [ ] La manette marche-t-elle d'emblée dans chacun ? (sinon : lesquels)

## Demander un jeu (0.12.0)
- [ ] Barre du haut ▸ **Demander un jeu** ▸ chercher « zelda ocarina ».
      **Attendu :** une liste (titre, console, année, studio) ; Ocarina of Time N64 marqué « ⏳ Déjà demandé ».
- [ ] Demander un jeu qui n'y est pas encore (confirmer). **Attendu :** « 📨 Demande envoyée », et la demande
      apparaît dans Firehouse (à vérifier côté cockpit).

## Menu en jeu (0.11.0)
- [ ] Installer Dune si besoin, le lancer, appuyer sur **Pause/Attn**.
      **Attendu :** le menu Frogtend apparaît PAR-DESSUS le jeu ; le jeu est figé.
      *Si le jeu continue :* Dune lance peut-être sa propre copie de DOSBox (pas réglée par Frogtend) : me le dire.
- [ ] ▶ Reprendre (ou Échap). **Attendu :** le menu disparaît, le jeu repart.
- [ ] 📖 Manuel et documents : ouvrir un texte. **Attendu :** lisible dans le menu ; Échap revient.
- [ ] ⏹ Quitter le jeu (confirmer). **Attendu :** le jeu se ferme proprement, Frogtend compte le temps de jeu.
- [ ] Hors partie, la touche Pause/Attn ne fait rien de spécial (elle reste aux autres programmes).
- [ ] ⚙ Options ▸ Émulateurs ▸ « Touche du menu en jeu » : essayer Arrêt défil à la partie suivante.

## Sonde manette (lot 4 ter)
- [ ] Lancer `D:\Frogtend\outils\sonde-manette\target\release\sonde-manette.exe`, appuyer sur chaque manette
      (Xbox, PS4, PS5, Switch Pro, 8BitDo…), puis lancer un jeu (au premier plan) et appuyer encore ; laisser finir
      les 3 minutes. **Attendu :** le bilan de `sonde-manette.txt` (à côté du programme). Me le donner.

## Plusieurs émulateurs par console (0.10.0)
- [ ] ⚙ Options ▸ Émulateurs : chaque console montre sa liste, ⭐ par défaut, ➕ Ajouter, ✕ Retirer.
- [ ] Un jeu : ⚙ Gérer le jeu ▸ 🕹 Émulateur ▸ en choisir un autre que celui de la console.
- [ ] Avec deux émulateurs sur une console : le bouton **▶ Jouer avec…** apparaît à côté de ▶ Jouer.

## Manettes (0.8.0 – 0.9.0)
- [ ] Un jeu PS1/PS2/GameCube/Wii (si tu en as) : la manette marche sans rien régler.
- [ ] ⚙ Options ▸ Émulateurs ▸ 🎮 Manette par défaut (sur un émulateur installé) : message de réussite.
- [ ] Wii : tes profils Wiimote. Dans Dolphin, régler la Wiimote, « Enregistrer » le profil, puis ⚙ Options ▸
      Émulateurs ▸ 📌 Réglages de référence ▸ le reprendre (garde le nom « Wiimote + Nunchuk » pour remplacer le
      mien). Me dire lesquels intégrer à Frogtend pour tous les PC.
- [ ] ⚙ Gérer le jeu ▸ 🎮 Commandes : Automatique / Clavier et souris / une référence.

## Émulateurs (0.6.0 – 0.7.0), jamais essayés en vrai
- [ ] Lancer un jeu d'une console sans émulateur : Frogtend propose le recommandé, annonce la taille, l'installe
      dans le dossier « Émulateurs », puis lance le jeu.
- [ ] ⚙ Options ▸ Émulateurs ▸ 🔎 Chercher des mises à jour.
- [ ] Deux profils, le même jeu d'émulateur : chacun retrouve SES parties.

## Sauvegarde (0.9.1)
- [x] Première vraie sauvegarde : faite et vérifiée le 30/09 (DUNE37S0.SAV chez Firehouse, identique).
