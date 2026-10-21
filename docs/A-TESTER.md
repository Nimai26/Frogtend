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

## Jeux GOG lancés par Galaxy (0.93.0) — À ESSAYER AVEC MOI
- [ ] Ludothèque ▸ Horizon Zero Dawn (GOG) : ▶ Jouer (pas « Jouer (GOG Galaxy) ») ; le message « GOG Galaxy
      lance… » ; Galaxy s'ouvre et le jeu démarre (la commande de Galaxy n'est pas documentée : c'est LE test) ;
      menu en jeu, « Quitter », temps de jeu.
- [ ] Cyberpunk 2077 : son prélanceur, puis le jeu — la partie reste-t-elle suivie jusqu'au jeu, et se finit-elle
      quand on quitte ?
- [ ] En Taodbox : les deux jeux GOG sont dans la grille.

## Succès RetroAchievements en détail (0.92.0)
- [ ] Il faut ton compte RetroAchievements relié (⚙ Options ▸ Succès).
- [ ] Un jeu d'émulateur reconnu (panneau : « x / y succès ») ▸ 🏆 Voir les succès : badges, points, dates, rareté.
- [ ] Pendant une partie : menu en jeu ▸ 🏆 Succès, à la manette.
- [ ] Sans Internet : « RetroAchievements ne répond pas ».

## Jeux PC de fans en archive (0.91.0)
- [ ] Ludothèque ▸ Windows ▸ « Megaman's Christmas Carol 3 » (60 Mo) : télécharger, installer ; aucun choix de
      programme demandé ; ▶ Jouer lance le jeu.
- [ ] Retirer le jeu : son dossier disparaît entièrement.

## Succès en détail (0.90.0)
- [ ] Ludothèque ▸ un jeu GOG avec succès ▸ panneau ▸ 🏆 Voir les succès : la liste, obtenus d'abord, icônes,
      dates, rareté ; ← Retour (ou Échap) revient.
- [ ] Un jeu Steam installé (Final Fantasy VIII…) ▸ ⚙ Gérer le jeu ▸ 🏆 Succès : 34 succès, 29 obtenus.
- [ ] En Taodbox, même chemin à la manette : on reste en plein écran, la croix parcourt la liste, B revient.
- [ ] Pendant une partie d'un jeu Steam : menu en jeu ▸ 🏆 Succès, à la manette ; la ligne choisie reste lisible.

## Mods dans les jeux Steam (0.89.0) — À ESSAYER AVEC MOI
- [ ] GTA V ▸ ⚙ Gérer le jeu ▸ 🧩 Mods ▸ un mod : REFUSÉ d'abord (BattlEye), sans choisir où le poser.
- [ ] Un jeu solo (Skyrim SE, XCOM 2) ▸ 🧩 Mods : l'avertissement « Risque de bannissement » (Annuler d'abord) ;
      un mod installable d'un clic ou par Nexus (« Mod Manager Download ») : il s'installe ; dans le dossier du jeu
      chez Steam, AUCUN dossier « Frogtend » n'apparaît.
- [ ] Le retirer : le jeu redevient comme avant (Steam ▸ Vérifier l'intégrité : rien à réparer).
- [ ] Pendant que Steam met le jeu à jour, ou pendant une partie : refusé, avec la raison.
- [ ] Après une vraie mise à jour Steam d'un jeu moddé : le mod est marqué ⚠ ; le retirer propose « Vérifier
      l'intégrité » et ouvre bien la vérification dans Steam.

## Jeux Switch avec mises à jour et DLC (0.88.0)
- [ ] Il faut Eden installé par Frogtend (⚙ Options ▸ Émulateurs).
- [ ] Un jeu Switch servi avec contenus (Fire Emblem Three Houses : 16 Go + 1,7 Go) : télécharger, installer ;
      le message dit que les mises à jour et DLC sont dans le bloc « 📦 ».
- [ ] Bloc « 📦 » : 1 mise à jour et 6 DLC listés ; les cocher ▸ Eden ferme ▸ « Rendre disponible ».
- [ ] ▶ Jouer : Eden démarre le jeu en v1.2.0, et les DLC (Cindered Shadows…) sont dans le jeu.

## Parties des jeux Steam à l'abri (0.87.0)
- [ ] XCOM 2 ▸ ⚙ Gérer le jeu ▸ 💾 Mettre mes parties à l'abri : « 42 fichier(s) de parties (environ 94 Mo) mis à
      l'abri ». Dans ton dossier des abris : un dossier du jeu avec `nuage-steam\…\Documents\my games\…`.
- [ ] Recommencer tout de suite : « Déjà à l'abri ».
- [ ] Jouer une partie, sauvegarder, quitter, recommencer : un nouvel abri.
- [ ] Pendant une partie : refusé (« quitte-le d'abord »).
- [ ] Sauvegarde du profil chez Firehouse : les parties XCOM 2 y sont.

## Triches et Cheat Engine pour les jeux Steam (0.86.0)
- [ ] Ludothèque ▸ un jeu Steam installé ▸ ⚙ Gérer le jeu ▸ 🧰 Triches et Cheat Engine : la page s'ouvre, avec
      « Anti-triche » et « Jeu en ligne » (GTA V : BattlEye, oui ; XCOM 2 : en partie).
- [ ] Lancer GTA V par Frogtend, puis « Brancher Cheat Engine » : REFUSÉ, avec la raison (BattlEye). (Ne lance PAS
      Cheat Engine sur GTA V autrement.)
- [ ] Un jeu solo (XCOM 2 en solo, ou Skyrim SE) : « Brancher Cheat Engine » → l'avertissement « Risque de
      bannissement » ; « Annuler » ne lance rien ; « Lancer quand même » → Cheat Engine s'ouvre, déjà branché sur le
      jeu (son nom en haut de Cheat Engine).
- [ ] Même chose depuis le menu en jeu, à la manette : l'avertissement se pilote à la manette, « Annuler » choisi
      d'abord.

## Jeux sur plusieurs disquettes zippées (0.85.1)
- [ ] Ludothèque ▸ NEC PC-8801 ▸ « Xak » : télécharger (moins de 1 Mo), installer : à côté des 4 zips, 4 fichiers
      .d88 apparaissent dans le dossier du jeu.
- [ ] ▶ Jouer : le jeu démarre sur le disque 1 ; quand il demande le disque 2, le changer par le menu de RetroArch
      (Disc Control) marche.
- [ ] Retirer le jeu : tout le dossier disparaît ; si tu as sauvegardé une partie, elle est dans tes abris.

## Jeux Steam suivis par Frogtend (0.85.0) — À ESSAYER AVEC MOI
- [ ] Ma ludothèque : un jeu Steam installé (GTA V, XCOM 2…) a « ▶ Jouer » (pas « Jouer (Steam) ») ; « Sur ce PC »
      montre son dossier Steam.
- [ ] ▶ Jouer : le message « Steam lance… » s'affiche ; le jeu démarre ; la touche du menu en jeu marche ;
      « Quitter » ferme le jeu ; le temps de jeu augmente. Essayer aussi avec Steam FERMÉ au départ.
- [ ] Un second clic sur Jouer pendant le démarrage ne lance rien de plus.
- [ ] En Taodbox : le jeu est dans la grille, une seule fois.
- [ ] Ouvrir un profil sans compte Steam, puis revenir : le jeu et son temps de jeu sont toujours là.

## MAME reconnu un par un (0.84.2)
- [ ] 📥 Importer ▸ 🔗 Reconnaître mes jeux importés après avoir importé ton MAME : la plupart des jeux MAME passent
      en « Reconnus » ; un jeu Neo-Geo dit « il manque à côté du jeu : neogeo (BIOS) » si le BIOS n'est pas dans le
      même dossier.

## Un jeu importé relié (0.84.1)
- [ ] Après une reconnaissance : un jeu NES reconnu apparaît UNE fois dans Ma ludothèque, avec la jaquette de
      Firehouse ; « 📄 Voir la fiche » ouvre sa fiche ; ▶ Jouer lance ton fichier.
- [ ] Ouvrir le profil d'une autre personne qui voit ce jeu : il est « sur ce PC », ▶ Jouer marche, ⚙ Gérer n'est
      pas proposé.
- [ ] ⚙ Gérer le jeu ▸ « Ce n'est pas ce jeu » : il redevient ton jeu importé, sans fiche.

## Reconnaître mes jeux importés (0.84.0)
- [ ] 📥 Importer ▸ 🔗 Reconnaître mes jeux importés : l'annonce dit combien de fichiers seront lus et leur poids.
      Lancer : la barre avance, puis le compte rendu (reconnus, à vérifier, inconnus). Essai réel du 20/10 sur 90 de
      tes fichiers : ~85 % des NES/SNES reconnus à coup sûr ; MAME surtout « à vérifier » (ton romset est fusionné).
- [ ] « À vérifier » : « C'est ce jeu » sur un jeu, « Ce n'est pas lui » sur un autre ; relancer la reconnaissance :
      le second ne revient pas.
- [ ] « Ne plus relier » sur un jeu reconnu : il revient dans « À vérifier ».
- [ ] Un jeu relié : « Mettre dans ma ludothèque » depuis le catalogue de Firehouse refuse de le retélécharger.

## Jeux Steam et GOG (0.83.0)
- [ ] 🛒 Boutiques ▸ Steam : un jeu en cours de téléchargement dans Steam n'apparaît PAS comme installé ; il le
      devient quand Steam a fini.

## Progression d'un mod (0.82.3)
- [ ] Installer un mod un peu gros (quelques dizaines de Mo) : en bas, l'étape change (Téléchargement →
      Décompression → Pose dans le jeu), la barre avance, la vitesse et « encore … » s'affichent ; un clic ne ferme
      pas le message ; il disparaît à la fin, remplacé par « installé ».

## Mods en .7z (0.82.2)
- [ ] Sur Nexus, « Mod Manager Download » sur un fichier **.7z** (petit) depuis ⚙ Gérer le jeu ▸ 🧩 Mods ▸ 🌐 :
      Frogtend demande où le poser puis ton accord ; le mod devient ✅ ; Retirer remet le jeu comme avant.
- [ ] Un fichier .rar : Frogtend dit poliment qu'il ne sait pas encore le poser.

## Archives 7z lues à part (0.82.1)
- [ ] Décompresser un jeu en .7z (⚙ Gérer le jeu ▸ Décompresser) : la barre de progression avance, le jeu se lance.
- [ ] Un jeu PC Engine en .7z : son panneau RetroAchievements dit toujours si ta version est compatible.
- [ ] Pendant une décompression, regarde le Gestionnaire des tâches : un 2e « Frogtend » apparaît le temps de la
      lecture, sans fenêtre, puis disparaît. Rien d'autre ne doit s'ouvrir.

## Les mods de Nexus Mods (0.82.0)
> La mise à jour vers 0.82.0 prend les liens « Mod Manager Download » (une seule fois : si tu les rends ensuite à un
> autre programme, Frogtend ne les reprend jamais). Si Vortex ou Mod Organizer est installé chez toi, Frogtend prend
> leur place pour ces liens (et les leur rend si tu désinstalles Frogtend).
- [ ] ⚙ Options ▸ Comptes ▸ 🧩 Nexus Mods ▸ **🌐 Trouver ma clé** : la page de la clé s'ouvre (sinon, dis-moi
      l'adresse où tu la trouves). Colle ta « Personal API Key » : on doit voir ton nom de compte Nexus.
- [ ] Un jeu PC installé qui a des mods 🌐 (Skyrim ?) ▸ ⚙ Gérer le jeu ▸ 🧩 Mods ▸ un mod 🌐 ▸ **Ouvrir sur Nexus** :
      la page s'ouvre. Clique « Mod Manager Download » sur un fichier **.zip** (petit, pour l'essai) : Frogtend
      revient avec « Où poser … ? » puis l'accord ; après le oui, le mod est ✅ installé dans la liste.
- [ ] Même chose avec Frogtend FERMÉ : le clic sur le site doit ouvrir Frogtend, puis la fenêtre arrive une fois ton
      profil ouvert.
- [ ] Un clic pendant que Frogtend est déjà ouvert : pas de 2e fenêtre de Frogtend.
- [ ] Un fichier .rar : Frogtend dit poliment qu'il ne sait pas encore le poser (les .7z passent depuis 0.82.2).
- [ ] Retirer le mod (🧩 Mods ▸ ✅ le mod ▸ Retirer) : le jeu redevient comme avant.
- [ ] Pendant une partie, cliquer « Mod Manager Download » : rien ne s'ouvre avant la fin de la partie.
- [ ] À vérifier par moi avec toi : l'adresse du serveur de téléchargement de Nexus, la forme exacte de la clé du lien.

## Retouches PS3 en français (0.81.2)
- [ ] Asura's Wrath ▸ 🩹 Retouches du jeu : les noms sont en français ; « Menu développeur » montre sa remarque en
      français avant d'activer.
- [ ] Une retouche activée marche toujours en jeu (RPCS3 reçoit le nom anglais d'origine).
- [ ] Hors ligne : les retouches actives restent affichées (nom de Frogtend, ou anglais marqué) et se désactivent.

## Codes et mods des jeux importés (0.81.1)
- [ ] Un jeu importé de ton disque (une ROM, ou Mega Man Rock Force) ▸ sa fiche ▸ 🎯 Triches et mods : des codes
      apparaissent (s'il y en a pour ce jeu).
- [ ] Pose un code dans l'émulateur : il marche en jeu.

## Les mods des jeux PC (0.81.0)
- [ ] Un jeu PC installé ▸ ⚙ Gérer ▸ 🧩 Mods : la liste (⬇ d'un clic, 🔗 à la main).
- [ ] ⬇ un mod : la fenêtre dit sa taille et son partage ; oui → « installé : N ajouté(s), M remplacé(s) ».
- [ ] Lance le jeu : le mod marche (ex. Ultimate Spider-Man + Fusion Fix : écran large).
- [ ] 🧩 Mods ▸ ✅ le mod ▸ Retirer : le jeu redevient comme avant.
- [ ] 🔗 un mod : « Ouvrir la page » ouvre le navigateur (au bureau) ; au canapé, Frogtend dit de le faire à l'écran
      du PC.

## Les scores d'arcade de chacun (0.80.0)
- [ ] Un jeu d'arcade qui garde ses meilleurs scores : fais un score, quitte, relance : le score est là.
- [ ] Change de profil, lance le même jeu : pas le score de l'autre (au tout premier lancement, le profil reçoit une
      copie de ce que MAME avait avant).
- [ ] Dans le dossier de MAME : `Profils\<ton profil>
vram` et `cfg` contiennent le jeu.

## Sauver dans les jeux d'arcade (0.79.0)
- [ ] Un jeu d'arcade (MAME) ▸ menu en jeu : 💾 Sauver, 📂 Charger, 🔄 Recommencer sont là.
- [ ] 💾 Sauver, joue un peu, 📂 Charger : tu reviens au moment sauvé.
- [ ] 🔄 Recommencer : le jeu redémarre ; le menu marche encore ensuite (pause, sauver).
- [ ] Un autre profil de la maison : son 📂 Charger dit « aucune sauvegarde rapide » (chacun la sienne).

## Les patchs PS3 (0.78.0)
- [ ] Asura's Wrath ▸ ⚙ Gérer ▸ 🩹 Retouches du jeu : 6 retouches ; le texte dit que 2 autres (la démo) ne sont pas
      montrées.
- [ ] « Sans flou de mouvement » : Frogtend demande d'installer la liste (1,2 Mo) ; oui → ✅ devant la retouche.
- [ ] « Débloquer les images par seconde » : la remarque de l'auteur (en anglais) s'affiche avant d'activer.
- [ ] Débranche Internet, rouvre l'écran : seules les retouches actives, et on peut les désactiver.
- [ ] Lance le jeu : le flou de mouvement a disparu. Dans RPCS3 ▸ Gestionnaire de patchs, le patch est coché.
- [ ] Re-choisis-le : ⬜ ; relance : le flou revient.
- [ ] « Débloquer les images par seconde » seul ne suffit peut-être pas : la remarque dit que le plafond dépend du
      « Vblank » de RPCS3 (non réglé par Frogtend) ; à essayer.

## Manette → clavier, plus simple (0.77.0)
- [ ] Un jeu PC fait pour le clavier ▸ ⚙ Gérer ▸ 🕹 Jouer à la manette ▸ ✏ Bouton par bouton ▸ A : la fenêtre
      « Appuie sur la touche » ; appuie sur Espace : A → Espace.
- [ ] Appuie sur Z : Frogtend demande « 🧭 Déplacement haut » ou « 🔤 La lettre Z ».
- [ ] Échap : il demande confirmation (« quitte certains jeux ») ; la touche Windows : « pas encore possible ».
- [ ] « 📋 Dans la liste… », « ⬜ Aucune touche », « Annuler » marchent à la souris.
- [ ] En jeu, un bouton TENU dans un menu (liste d'options, inventaire) : ça défile comme une touche tenue.
- [ ] « 🔁 Un bouton tenu répète sa touche : non » : chaque appui = une seule touche.

## Gérer un jeu depuis le canapé (0.76.0)
- [ ] Taodbox, une jaquette choisie, **Y** : « ⚙ Gérer » s'ouvre ; la croix et A choisissent, B revient.
- [ ] 🕹 Émulateur, 🎮 Commandes, 📀 Version (Dune ou un jeu à plusieurs versions), 🎨 Image et fluidité (Asura's
      Wrath) : chacun se règle à la manette, sans souris.
- [ ] Il n'y a ni « 🗑 Retirer », ni « ➕ Un autre émulateur… », ni « 📂 Un autre programme… ».
- [ ] Le menu fermé, la même jaquette reste choisie (A la lance).
- [ ] Au clavier en Taodbox : la touche Menu (≣) fait pareil ; à la souris, le clic droit.
- [ ] Au bureau : « ⚙ Gérer le jeu » est comme avant (Retirer, Un autre émulateur… sont là).
- [ ] Par Moonlight (manette de la box) : Y marche aussi (non vérifié avec la manette virtuelle de Sunshine).

## Jeux Xbox : prévenu avant de lancer (0.75.0)
- [ ] (Quand un jeu Xbox sera là : aucun au stock ni chez toi au 19/10.) Une image XISO se lance dans xemu comme avant.
- [ ] Une image « Redump » (≈ 7 Go) : ▶ Jouer ouvre une fenêtre qui explique pourquoi elle ne se lance pas ; xemu
      ne s'ouvre pas. Pareil depuis Taodbox : la fenêtre se ferme à la manette.
- [ ] Un jeu Xbox encore en .zip : la fenêtre renvoie au bouton « 🗜 Décompresser pour jouer ».

## Image et fluidité d'un jeu PS3 (0.74.0)
- [ ] Asura's Wrath ▸ ⚙ Gérer le jeu ▸ 🎨 Image et fluidité : les 5 lignes (la 6e, netteté, apparaît avec
      « Plus net »), chacune « — comme RPCS3 ».
- [ ] Résolution ▸ 200 % ; Agrandissement ▸ Plus net ; puis joue : l'image est plus fine (et le jeu un peu plus lourd).
      Dans RPCS3, clic droit sur le jeu ▸ « Change Custom Configuration » : ces deux réglages y sont.
- [ ] Un autre jeu PS3 : il garde ses réglages habituels.
- [ ] ↩ Valeurs de RPCS3 : le jeu redevient comme avant.
- [ ] Un jeu encore en .zip (God of War III…) : Frogtend dit de le décompresser d'abord.
- [ ] (0.74.1) Chez toi le cache de shaders est allumé : AUCUN avertissement « ⚠ RPCS3 est réglé pour oublier… » sous
      le titre du menu. (Si tu veux voir l'avertissement : coche « Disable On-Disk Shader Cache » dans RPCS3, rouvre
      le menu, puis décoche-le.)

## Où Frogtend écrit (0.73.0)
- [ ] (Si tu prépares un nouvel émulateur avec « Prêt à jouer » ou l'assistant) : il s'installe comme avant, ses
      fichiers système et son cœur sont posés.
- [ ] Réglages ▸ Emplacements ▸ Abris de parties ▸ un dossier : réglé comme avant.
- [ ] Importer ▸ chercher dans un dossier réseau (\\…) : Frogtend prévient d'abord que ce peut être long.

## Les choix de fichiers par le cœur (0.72.0) — ⚠️ À ESSAYER EN PREMIER
Toutes les fenêtres « Choisir un fichier / un dossier » de Frogtend passent désormais par le cœur : je n'ai pas pu
l'essayer moi-même (il faut un clic). Si l'une ne s'ouvre pas, dis-le-moi tout de suite.
- [ ] Réglages ▸ Emplacements ▸ Ajouter ▸ « 📂 Choisir un dossier » : la fenêtre de Windows s'ouvre, au-dessus de
      Frogtend ; le dossier choisi s'ajoute.
- [ ] Bon à savoir : un choix ne sert qu'une fois, pendant 10 minutes. Si un import ou « installé ailleurs » refuse
      avec « choisis-le avec l'explorateur », refais simplement le choix.
- [ ] Importer ▸ un jeu Windows ▸ un raccourci (.lnk) : le jeu se lance (avant, un raccourci importé ne démarrait
      pas).
- [ ] Importer ▸ un jeu Windows ▸ « Choisir le programme » : la fenêtre s'ouvre ; l'import réussit.
- [ ] (Si tu l'utilises) ⚙ Gérer ▸ « Un autre programme… » : la fenêtre s'ouvre ; le choix est enregistré.

## Ce que Frogtend lance et efface (0.71.0)
- [ ] Un jeu PS3 : il se lance comme avant (RPCS3 est réglé).
- [ ] RPCS3 ▸ installer le micrologiciel (si on te le propose) : accepté comme avant.
- [ ] Un jeu PC lancé par un .bat : il se lance comme avant.
- [ ] (Facultatif) « Le jeu est installé ailleurs » ▸ choisis C:\ ou ton dossier « Documents » : refusé avec une
      explication ; un vrai dossier de jeu : accepté.
- [ ] (Facultatif) ⚙ Gérer ▸ Changer ce qui lance le jeu ▸ « Un autre programme… » ▸ un raccourci du Bureau :
      accepté.
- [ ] (Si un jour tu retires un jeu installé par un installeur ailleurs) : Frogtend dit qu'il reste installé et
      propose son désinstalleur ; refuse, rien ne se passe ; accepte, la fenêtre du désinstalleur s'ouvre.

## Les infos dans la fiche du jeu (0.70.0)
- [ ] Ouvre la fiche de Contra III (ou d'un jeu SNES) : « Sortie : 28 février 1992 » et « Avis des joueurs :
      ★ 4,3 / 5 » dans les informations.
- [ ] La fiche d'un de tes jeux installés : les mêmes lignes apparaissent (relues en direct).
- [ ] Un jeu sans ces infos : aucune ligne vide, rien d'anglais.

## Les lignes de commande de la maison (0.69.0)
- [ ] Un jeu PS3 : il se lance comme avant (ton RPCS3 réglé à la main n'est pas touché, aucune question au lancement).
- [ ] (Si un jour tu prépares Dolphin, PCSX2 ou un autre émulateur avec « Prêt à jouer ») : le jeu démarre en plein
      écran, directement.
- [ ] Une machine de salon par MAME (RCA Studio II…) : le jeu démarre ; le menu en jeu ne promet PAS de pause pour
      elles (seulement pour l'arcade).

## Apostrophes et virgules dans les dossiers (0.68.2)
- [ ] Rien à faire chez toi aujourd'hui (aucun de tes dossiers n'en a). Si un jour tu ranges des émulateurs dans un
      dossier comme « Jeux d'émulation » : xemu, Eden et Azahar démarrent normalement depuis Frogtend.

## La sauvegarde et les jeux visibles (0.68.1)
- [ ] 👤 Mon profil ▸ Sauvegarder maintenant : la sauvegarde se fait comme avant (le nombre de fichiers envoyés ne baisse pas : ton seul abri concerne un jeu
      que tu vois).

## Un jeton d'un autre compte (0.68.0)
- [ ] Rien à faire au quotidien : ta ludothèque ne doit pas se vider toute seule après la mise à jour (ton compte
      est simplement retenu à la première synchronisation).
- [ ] Si un jour tu renouvelles ton jeton (même compte « sebastien ») : la ludothèque reste, rien n'est relu.
- [ ] (Facultatif) Sur un profil d'essai, colle le jeton d'un autre compte (par exemple un compte mineur) : la
      ludothèque se vide aussitôt, puis ne montre que ce que ce compte a le droit de voir.

## Le mode simulé à part (0.67.1)
- [ ] Réglages ▸ Firehouse ▸ coche le mode simulé, synchronise : une trentaine de jeux d'exemple.
- [ ] Décoche-le : ta vraie ludothèque revient tout de suite (même nombre de jeux qu'avant), sans attendre.

## Images hors ligne et ménage (0.67.0)
Chez toi le ménage ne retire rien (vérifié le 18/10 sur une copie de tes données : 66 jaquettes, toutes gardées).
- [ ] Synchronise, attends une minute, puis coupe Internet (débranche le câble ou coupe le Wi-Fi) et ouvre Taodbox :
      les fonds et les logos de tes jeux installés s'affichent toujours.
- [ ] Toujours sans Internet, la ludothèque : les jaquettes déjà vues s'affichent.
- [ ] Rebranche Internet : tout refonctionne comme avant.

## Fonds et logos dans Taodbox (0.66.0)
- [ ] Taodbox : arrête-toi sur un jeu (par exemple Contra III ou un jeu SNES) : son fond apparaît derrière, flouté et
      assombri, et son logo en haut. Le texte reste lisible.
- [ ] Fais défiler vite avec la croix : rien ne clignote ; le fond change quand tu t'arrêtes.
- [ ] Un jeu sans logo (un jeu importé du disque, par exemple) : son titre en haut à la place.
- [ ] Change de console en haut : l'ancien fond disparaît.
- [ ] Passe d'un jeu à l'autre : jamais d'éclair (l'image reste toujours assombrie sous le texte).
- [ ] Une carte Steam ou GOG : le fond habituel du skin.
- [ ] Réglages ▸ Apparence ▸ Animations « Réduites » : le fond et le logo apparaissent d'un coup, sans fondu.
- [ ] Réglages ▸ Ludothèque ▸ décoche « Fonds et logos des jeux dans Taodbox » : Taodbox redevient comme avant.
- [ ] En streaming sur la télé : le flou du fond te plaît-il ? Dis-le-moi.

## Les fiches réunies (0.65.0)
Aucun de tes 9 jeux installés n'a de fiche réunie (vérifié le 18/10) : rien ne doit changer chez toi.
- [ ] Synchronise la ludothèque : aucun message « rattaché », tes jeux installés sont tous là et se lancent.
- [ ] Le temps de jeu de tes jeux est le même qu'avant la mise à jour.
- [ ] Si un jour un message « 🔗 jeu(x) de ton PC rattaché(s) » apparaît : le jeu est toujours là, il se lance, son
      temps de jeu est gardé. Dis-le-moi.

## La galerie d'images (0.64.0)
- [ ] Fiche de 3x3 Eyes (ou un autre jeu SNES) ▸ 🖼 Images du jeu : des vignettes de même taille (boîte, dos,
      tranche, cartouche, images du jeu…), libellées en français.
- [ ] Clic (ou A) sur une vignette : la visionneuse ; ◀ ▶ (ou ← →) changent d'image ; B / Échap : retour sur la
      vignette de la dernière image vue.
- [ ] Une fiche MS-DOS (pas d'images chez Firehouse) : pas de section Images.

## L'image des jeux (0.63.0)
- [ ] Après la mise à jour : la première synchronisation dure ~3 minutes (une fois), Frogtend reste utilisable.
- [ ] Ludothèque ▸ 🎨 : Boîte en relief, Cartouche, Écran titre, Image du jeu : la grille reste régulière ; un jeu
      sans cette image montre sa jaquette ; rien ne « saute » pendant le chargement.
- [ ] Écran titre / Image du jeu sur un jeu SNES : gros pixels nets (pas flous).
- [ ] Taodbox : les cartes suivent le même choix.

## Conseils suivis et DOSBox d'un clic (0.62.0)
- [ ] 📥 Importer ▸ Installer un jeu DOS, avec MS-DOS préparé par « Prêt à jouer » (RetroArch) : le bouton
      « 📀 Préparer DOSBox » ; un accord, puis l'installation du jeu se fait. Les jeux DOS se lancent toujours comme avant.
- [ ] Quand Firehouse changera un conseil (par exemple un autre cœur pour une console), au lancement d'un jeu de cette
      console : la question arrive une fois ; « Appliquer » installe ce qu'il faut ; le jeu démarre ensuite.

## Les options du jeu (0.61.0)
- [ ] Un jeu RetroArch ▸ View+RB1 ▸ ⚙ Options du jeu ▸ 📊 : le compteur s'affiche en haut à droite ; refais-le : il
      disparaît.
- [ ] ⏩ Avance rapide : le jeu va plus vite ; rouvre le menu : il le rappelle ; « Couper l'avance rapide » le remet.
- [ ] 📷 Photo de l'écran : une image arrive dans le dossier « screenshots » de RetroArch.
- [ ] 🔇 Son : coupé, puis remis.
- [ ] (0.61.1) Sur un PC qui n'a jamais lancé de jeu RetroArch par Frogtend (chez ta sœur, par exemple) : avant le
      premier jeu, Frogtend prévient de la question du pare-feu ; « Annuler » à la question de Windows, puis le menu en
      jeu (Options du jeu) marche toujours ? Le message ne revient plus ensuite.

## Les manettes PlayStation et Switch (0.60.0)
- [ ] Une manette PlayStation (USB, puis Bluetooth) pendant un jeu (RetroArch, RPCS3, DuckStation…) : Share/Create + R1
      tenus 1 s ouvrent le menu ? La croix et Croix/Rond le pilotent ? Le jeu garde la manette (et sa vibration)
      après « ▶ Reprendre » ? La barre lumineuse change-t-elle de couleur ?
- [ ] Une manette Switch Pro (ou Joy-Con) avec Eden : − + R ouvrent le menu ? Eden garde la manette ? Les voyants ?
- [ ] Une 8BitDo : en mode X (Xbox), et en mode D si tu l'utilises (en mode D, elle n'ouvre PAS le menu : voulu, pour
      ne jamais la retirer à l'émulateur).
- [ ] Un jeu PC réglé « manette → clavier » avec une manette PlayStation.
- [ ] Après la partie, dans le journal de Frogtend : les lignes « manette : … lue par SDL / laissée à XInput ».

## Les consoles récentes en pause au menu (0.59.0)
- [ ] Un jeu Switch (Eden) ou 3DS (Azahar) ▸ View+RB1 : le jeu se met en pause ? « ▶ Reprendre » le relance ?
- [ ] Un jeu Wii U (Cemu), Xbox 360 (Xenia), Xbox (xemu) ou PS Vita (Vita3K), en plein écran ▸ View+RB1 : le menu
      s'affiche par-dessus ? Le jeu est figé (image arrêtée, la manette ne fait plus rien dans le jeu) ? Après
      « ▶ Reprendre » : le jeu repart, le son revient (un petit grésillement possible) ?
- [ ] Menu ouvert plus de 5 secondes sur un émulateur figé : Windows affiche-t-il « Ne répond pas » ?

## MAME en pause au menu (0.58.0)
- [ ] Un jeu d'arcade de Firehouse (Street Fighter III…) ▸ ▶ Jouer, en PLEIN ÉCRAN (le réglage normal de MAME) ▸
      View+RB1 : le menu s'affiche-t-il par-dessus ? Le jeu est-il en pause (le menu dit « ⏸ Le jeu est en pause ») ?
- [ ] ▶ Reprendre : le jeu repart, l'image revient en plein écran ?
- [ ] Mets le jeu en pause toi-même (P), ouvre puis ferme le menu : le jeu doit rester en pause.

## La musique et les vidéos du jeu (0.57.0)
- [ ] Fiche de Zelda : Ocarina of Time ▸ 🎵 Musique (2 min 33) : elle se télécharge puis joue toute seule ? (Sinon le
      lecteur dit « Appuie sur ▶ Lecture » : dis-le-moi.) La jaquette s'affiche au milieu.
- [ ] 🎬 Vidéo (37 s) : elle joue ; ◀ ▶ (ou ← →) avancent et reculent de 10 s, même vers la fin de la vidéo.
- [ ] À la manette SEULE, dans le menu en jeu (View+RB1 ▸ 📖 Manuel et documents ▸ 🎵) : A lance-t-il la musique ?
- [ ] Ferme le jeu pendant que la musique joue dans le menu : la musique doit s'arrêter.
- [ ] (0.57.1) Menu en jeu d'un jeu NON en pause : seul le livret est proposé ; jeu en pause (RetroArch, ou jeu PC
      figé) : musique et vidéo sont là.

## Le livret du jeu (0.56.0)
- [ ] Ouvre la fiche de Zelda : Ocarina of Time ▸ 📚 Documents du jeu ▸ « 📖 Livret (scan) · 23 pages · 8,2 Mo » :
      il se télécharge (quelques secondes), puis la première page s'affiche.
- [ ] À la manette (Taodbox ou menu en jeu) : ◀ ▶ changent de page, A agrandit (◀ ▶ défilent alors sur le côté),
      ▲ ▼ défilent, B referme. Au clavier sur la fiche : flèches, Page préc./suiv., Échap.
- [ ] En jeu : View+RB1 ▸ 📖 Manuel et documents ▸ le livret : lisible dans la petite fenêtre ? (sinon A pour agrandir)
- [ ] Rouvre le même livret : plus de taille sur le bouton, il s'ouvre aussitôt (déjà sur le PC).

## L'arcade MAME (0.55.0)
- [ ] Catalogue ▸ Arcade ▸ un jeu (par exemple Street Fighter III) ▸ télécharger, installer ; ▶ Jouer : l'accord unique
      propose MAME (environ 84 Mo) ; après, le jeu démarre dans MAME.
- [ ] Un jeu marqué incomplet : l'avertissement apparaît avant l'installation.
- [ ] Le menu Frogtend (View+RB1) par-dessus MAME en plein écran : il s'affiche ? (MAME ne se met PAS en pause.)
- [ ] Un jeu MAME importé du disque (MAME Arcade Full Set) se lance toujours.

## Jouer à la manette aux jeux clavier (0.54.0)
- [ ] Mega Man Rock Force : ⚙ Gérer le jeu ▸ 🕹 Jouer à la manette ▸ « Flèches + Z X C ». ▶ Jouer : la croix déplace,
      A saute, X tire, B glisse, Start met en pause, LB/RB changent d'arme.
- [ ] Pendant la partie, View + RB 1 s : le menu s'ouvre, Mega Man ne continue PAS à marcher tout seul ; « Reprendre » :
      la manette repilote le jeu.
- [ ] Alt+Tab vers une autre fenêtre en tenant la croix : rien ne s'écrit dans l'autre fenêtre.
- [ ] Note : si le jeu voit aussi ta manette lui-même, choisis « Non » (sinon il reçoit tout en double).

## Le plein écran au lancement (0.53.0)
- [ ] Mega Man Rock Force : ⚙ Gérer le jeu ▸ 🖥 Plein écran au lancement ▸ « Frogtend appuie sur F4 pour toi ». ▶ Jouer :
      quelques secondes après l'ouverture, le jeu passe en plein écran. Relance-le : toujours en plein écran.
- [ ] Un autre jeu Windows : « Agrandir sa fenêtre à tout l'écran » ; dis-moi si l'image suit ou reste petite.
- [ ] Pendant ces quelques secondes, ouvre le menu (View + RB) : le menu reste devant.

## Le micrologiciel PS3 de Firehouse (0.52.0)
- [ ] RPCS3 FERMÉ : ⚙ Options ▸ 🕹 Émulateurs ▸ 💿 Micrologiciel PS3 → « Installé : version 4.91 », proposition de la
      4.93 (≈ 197 Mo) → ⬇ Installer : l'avancement s'affiche, puis « ✅ Micrologiciel PS3 4.93 installé ». Recliquer
      sur 💿 : « déjà à jour ». Un jeu PS3 se lance (la première partie de chaque jeu prépare RPCS3 un moment).
- [ ] RPCS3 OUVERT (un jeu PS3 en cours) : 💿 → message « RPCS3 est ouvert : ferme-le ».

## Les fichiers système (0.51.0)
- [ ] Préparer une console qui en a besoin (PS2 avec PCSX2, ou PlayStation avec RetroArch) : l'accord annonce les
      fichiers système et leur poids ; après, « 📀 Fichier système posé » ; un jeu démarre sans demander de BIOS.

## Les cœurs RetroArch de Firehouse (0.50.0)
- [ ] En préparant une console jouée par RetroArch (Super Nintendo…), le cœur s'installe sans erreur ; le jeu se lance.

## Le conseil de la maison (0.49.0)
- [ ] Une console que tu n'as PAS encore réglée (par exemple Super Nintendo, avec un jeu importé) : ▶ Jouer → l'accord
      unique propose RetroArch (le conseil de Firehouse) ; après « Tout préparer », le jeu se lance en plein écran.
- [ ] Tes consoles déjà réglées (PS3) : rien n'a changé.

## Le fond du menu en jeu (0.48.0)
- [ ] Un jeu venu de Firehouse (pas un jeu importé) : ouvre le menu en jeu. Derrière la carte du menu, l'image du jeu,
      floutée et assombrie ; la carte est la même qu'avant, au centre. ⚠️ Tes jeux actuels sont des jeux importés (PS3,
      Mega Man) ou Dune, que Firehouse n'illustre plus : ils gardent le fond du skin — c'est normal.
- [ ] En Taodbox : le menu laisse un bord libre autour de lui (environ 5 % de l'écran).

## Figer un jeu PC pendant le menu (0.47.0)
- [ ] D'abord l'importer (il n'est pas encore dans Frogtend) : 📥 Importer ▸ 🪟 Jeux Windows ▸ « Choisir le programme
      (.exe) » ▸ `E:\Games\Windows\Mega Man Rock Force\Mega Man Rock Force.exe` ▸ Ajouter.
- [ ] Puis ce jeu : ⚙ Gérer le jeu ▸ ❄ Figer le jeu pendant le menu
      de Frogtend ▸ lis la fenêtre, puis « Figer pendant le menu ». Lance le jeu, ouvre le menu (View + RB 1 s) :
      l'image s'arrête, le menu dit « ⏸ Le jeu est en pause », et la manette ne bouge plus rien dans le jeu.
      « Reprendre » : le jeu repart là où il était. Dis-moi si le son grésille ou boucle pendant le menu.
- [ ] Même jeu, menu ouvert, puis ⏹ Quitter le jeu : il se ferme normalement.
- [ ] Sans l'option (d'office) : le menu s'ouvre par-dessus le jeu, qui continue derrière.

## Le menu met en pause (0.46.3)
- [ ] Dans Asura's Wrath, ouvre le menu (View + RB 1 s) : la croix et A ne font plus rien DANS le jeu (seulement
      dans le menu) ; l'image et le son s'arrêtent **au bout d'environ une seconde** (délai de RPCS3 lui-même).
      « Reprendre » : le jeu repart là où il était.
- [ ] Menu ▸ ⏹ Quitter le jeu pendant cette pause : RPCS3 se ferme quand même.
- [ ] Pendant une partie normale (menu fermé), la manette pilote bien le jeu (RPCS3 ignore maintenant la manette quand
      il n'est pas au premier plan : si le jeu ne répond plus, le dire).
- [ ] Plus tard, avec DuckStation / PCSX2 / RetroArch installés : Select + Start et L3 + R3 n'ouvrent plus leur menu,
      View + RB ouvre celui de Frogtend. ⚠️ Dans RetroArch, le bouton Xbox (Guide) ouvre encore SON menu (voir
      SUIVI) : regarder si Windows (Game Bar) le capte avant.

## RPCS3 et le menu (0.46.2)
- [ ] Lance Asura's Wrath, ouvre le menu (View + RB 1 s) : un seul bouton est coloré, celui qui est choisi ; la croix
      déplace la couleur.
- [ ] ⏹ Quitter le jeu ▸ Quitter, **deux fois de suite** (relance entre les deux) : RPCS3 se ferme, aucune boîte
      d'erreur, ▶ Jouer relance sans « une instance est en cours ».

## « Quitter le jeu » (0.46.1)
- [ ] Dans Asura's Wrath : menu ▸ **⏹ Quitter le jeu** ▸ Quitter. **Attendu :** au plus ~10 secondes après, Frogtend
      n'affiche plus « partie en cours » et ▶ Jouer relance le jeu sans message « une instance est en cours ».

## Le menu en jeu à la manette (0.46.0)
- [ ] Dans une partie (Asura's Wrath, par exemple), tiens **Select + R1** (View + RB) **1 seconde** : le menu s'ouvre
      par-dessus le jeu, et le jeu se met en pause.
- [ ] À la manette : la croix (ou le stick) change la sélection, **A** valide, **B** reprend le jeu.
- [ ] Un appui bref sur Select + R1, ou Select seul, ou Select + Start : le menu ne s'ouvre PAS.
- [ ] ⚙ Options ▸ Émulateurs ▸ **Combinaison de la manette** : choisis L3 + R3, relance une partie : c'est elle qui
      ouvre le menu.
- [ ] Dans « ⏹ Quitter le jeu ? », c'est **Annuler** qui est sélectionné d'abord ; B referme la question, le jeu continue.
- [ ] 📖 Manuel et documents, puis B : on revient au menu, et un bouton est sélectionné (A agit tout de suite).
- [ ] Si le menu s'ouvre mais que la manette pilote encore le jeu derrière, dis-le-moi (c'est noté dans le journal de
      Frogtend : « sans le premier plan »).

## Le menu en jeu à la manette, au clavier et à la souris (0.45.0) — ✅ clavier et souris (Seb 05/10) ; manette : 0.46.0
- [ ] Dans une partie, **Pause/Attn** ouvre le menu. À la **manette** : la croix (ou le stick) change la sélection, **A**
      valide, **B** reprend le jeu. Au **clavier** : flèches, Entrée, Échap. À la **souris** : le survol déplace la
      sélection, le clic valide. La sélection est toujours entourée.
- [ ] **📖 Manuel et documents** puis **B** : on revient au menu (pas au jeu). **⏹ Quitter le jeu** puis **B** dans la
      question : la question se ferme, le jeu continue.
- [ ] **En Taodbox** (manette, télé) : le menu s'ouvre en plein écran, en grand.

## La manette dans RPCS3 (0.44.4 — la 0.44.3 cassait tout : nom mal lu par RPCS3)
- [x] ✅ (Seb 05/10, 0.44.4) Manette Xbox **éteinte**, clique ▶ Jouer sur Asura's Wrath, PUIS allume la manette : en 2 secondes elle marche
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
