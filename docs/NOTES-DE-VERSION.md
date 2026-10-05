# Notes de version de Frogtend

## 0.45.0 — le menu en jeu à la manette, au clavier et à la souris (OSD, étape 2)

- Le menu en jeu (touche Pause/Attn pendant une partie) se pilote maintenant **à la manette** (croix ou stick pour
  choisir, A pour valider, B pour revenir ou reprendre le jeu), **au clavier** (flèches, Entrée, Échap) et **à la
  souris** (le survol déplace la sélection). La sélection se voit toujours.
- **En Taodbox**, le menu s'ouvre en plein écran, à la taille de la télé.
- Corrigé au passage (relecture de l'expert) : dans une question comme « Quitter le jeu ? », B ou Échap ferment la
  question SANS reprendre le jeu, et la croix reste dans la question (le même défaut existait en Taodbox) ; une
  fenêtre qui n'est pas au premier plan ignore la manette (un appui ne compte jamais dans deux fenêtres).
- Une seule façon de naviguer pour Taodbox et le menu en jeu. Prochaine étape : ouvrir le menu à la manette
  (Select + R1 tenus 1 seconde) et retirer les anciens menus des émulateurs.

## 0.44.4 — RPCS3 : la manette (et le clavier) remarchent

- **Le problème de la 0.44.3** : le nom de la manette écrit pour RPCS3 (« XInput Pad #1 ») était mal lu par RPCS3, qui
  ne voyait que « XInput Pad » (dans ce format de fichier, « #1 » après une espace est un commentaire). Ne trouvant
  rien, RPCS3 mettait le joueur 1 sur « aucune entrée » : ni manette, ni clavier. Le nom est maintenant écrit entre
  guillemets.
- Vérifié dans le code de RPCS3 : avec ce nom, il accepte la manette même éteinte au lancement et la prend dès qu'elle
  s'allume.

## 0.44.3 — RPCS3 : n'importe quelle manette, même réveillée après le lancement

- **Le problème** : au clic sur ▶ Jouer, ta manette sans fil était en veille ; Frogtend ne la voyait pas et donnait le
  CLAVIER à RPCS3. Ta manette ne pouvait donc pas marcher.
- **Maintenant** : Frogtend donne à RPCS3 la manette branchée (Xbox et compatibles, PS4, PS5) ; si aucune ne répond, la
  dernière vue sur ce PC (sinon une manette Xbox). RPCS3 la prend dès qu'elle se réveille, sans relancer le jeu. Le
  clavier seulement si tu le choisis pour un jeu (⚙ Gérer le jeu ▸ 🎮 Commandes).
- Les autres modèles (manette Switch Pro, 8BitDo hors mode Xbox…) viendront avec le module SDL (lot OSD).

## 0.44.2 — la manette dans RPCS3, et les traductions de fan

- **RPCS3 : ta manette marche enfin.** La 0.44.0 annonçait la manette réglée à chaque partie, mais ce réglage n'était
  en fait jamais appelé au lancement (mon oubli). Maintenant, à chaque partie : ta manette Xbox si elle est branchée,
  sinon le clavier. Un réglage que tu fais toi-même dans RPCS3 n'est jamais défait ; et si le réglage ne peut pas être
  écrit, le jeu se lance quand même.
- **Traductions de fan** : un fichier « [T-Fr by groupe] » compte comme une version française (juste après la
  française officielle, avant l'Europe), « [T-En by groupe] » comme anglaise ; c'est dit en clair : « FR (trad.) »,
  « Traduction française par … ». (Un jeu déjà importé avant cette version garde son ancien classement : retire-le
  et réimporte-le si besoin — aucun chez toi pour l'instant.)

## 0.44.1 — les nouvelles images de Firehouse, légères

- Firehouse sert maintenant de nombreuses images par jeu (boîtes, cartouches, logos, captures…), et la jaquette
  d'origine peut peser jusqu'à 13 Mo. Frogtend ne demande plus jamais l'original : au plus une version de
  1 000 pixels (environ 56 Ko au lieu de 605 Ko pour un jeu Super Nintendo essayé), y compris pour la copie gardée
  avec un jeu installé (hors ligne).
- Ta prochaine synchronisation relira une fois les jeux Super Nintendo (Firehouse vient de mettre à jour leurs fiches).

## 0.44.0 — Prêt à jouer

- **Une console sans émulateur se prépare toute seule, après un seul « oui »** : quand tu importes des jeux,
  télécharges un jeu de console, ou cliques ▶ Jouer, Frogtend propose l'émulateur recommandé par Firehouse et dit
  tout dans une seule fenêtre (taille, dossier, manette, première partie plus longue). Puis il installe, règle la
  manette et retient l'émulateur. Le dossier des émulateurs est proposé d'office, à côté de tes jeux.
- **RPCS3 (PS3) : la manette est réglée à chaque partie** : ta manette Xbox si elle est branchée, sinon le clavier
  (✕ = X, Start = Entrée). Un réglage que tu fais toi-même dans RPCS3 n'est jamais défait.
- **Micrologiciel PS3 installé en quelques secondes, sans aucune fenêtre ni question** ; Frogtend vérifie la version
  vraiment installée et te la dit (le bouton 💿 montre aussi la version actuelle).
- **« Sur ce PC »** (⚙ Options ▸ Émulateurs) montre l'émulateur dès qu'il vient d'être installé.

## 0.43.4 — régler un émulateur sans attendre d'avoir les jeux

- **⚙ Options ▸ Émulateurs ▸ Régler un autre système** propose maintenant TOUTES les plateformes connues : d'abord
  celles qui ont des jeux (sur ce PC, importés compris, ou dans le catalogue), avec leur nombre, puis les autres. Avant,
  seules celles du catalogue de Firehouse apparaissaient : impossible de régler RPCS3 pour tes jeux PS3 importés.

## 0.43.3 — l'import qui marche, sans rien taper

- **Le bouton « Ajouter » de l'import ne bloque plus** : il lisait en entier chaque fichier du dossier (196 Go pour
  tes jeux PS3) avant de proposer où les garder. Il ne lit plus que leur taille : moins d'une seconde.
- **Plus d'extension à taper** : choisis le dossier, Frogtend propose la plateforme d'après son nom
  (« Playstation 3 » → « Sony Playstation 3 ») et montre les types de fichiers trouvés, avec leur nombre ; ceux des
  jeux sont déjà cochés (les vidéos, images, textes et pistes .bin d'un .cue ne le sont pas). La plateforme se choisit
  dans une liste.
- **Plus de fenêtres noires** qui s'ouvrent et se ferment (en ouvrant « Importer », par exemple).

## 0.43.2 — les vrais messages d'erreur, une barre du haut qui tient dans la fenêtre

- **Les erreurs disent enfin leur motif** : depuis le début, toute erreur venue du cœur de Frogtend s'affichait
  « motif inconnu » (jeton refusé, création de compte, installation…). Elles disent maintenant ce qui se passe.
- **Plus de barres de défilement autour de la fenêtre** : la barre du haut ne dépasse plus ; s'il y a trop
  d'onglets pour la largeur, ils défilent dans la barre, et les boutons Réduire / Agrandir / Fermer restent visibles.
- **Désinstallation** : si la case « Supprimer les données de l'application » est cochée, une copie de tes données
  est d'abord mise de côté (`%APPDATA%\fr.hikari-no-sekai.frogtend.avant-desinstallation`).

## 0.43.1 — contrôle de sécurité et corrections

Une relecture complète du code (sécurité, puis les lots 0.36 à 0.43) a trouvé des défauts avant tes essais. Corrigés :

- **Sécurité**
  - Une archive .7z piégée ne peut plus écrire hors du dossier du jeu (elle est refusée entière).
  - Ta connexion RetroAchievements ne part plus dans la sauvegarde chez Firehouse ; « Oublier ce compte » et la
    suppression d'un profil la retirent aussi des émulateurs (RetroArch, PCSX2, DuckStation).
  - Une connexion faite dans PCSX2 avant Frogtend est copiée à l'abri avant d'être remplacée.
  - Une fiche de triche ne peut plus faire écrire ailleurs que dans le dossier de l'émulateur.
  - Un profil ne peut plus lire les contenus ou succès d'un jeu qu'il ne voit pas.
- **Décompresser pour jouer**
  - Plus jamais proposé pour l'arcade (MAME) ni pour une cartouche zippée en .bin.
  - La place libre s'affiche bien avant de décompresser ; les fichiers vides d'un .7z sont recréés ; les autres
    disques d'un jeu multi-disques sont gardés.
- **Import**
  - Ajouter une autre région à un jeu importé seul ne fait plus disparaître sa version d'origine.
  - Une ROM différente de même taille n'est plus prise pour la même ; une copie interrompue ne laisse plus de faux
    « déjà là ».
  - La copie d'un jeu multi-disques (.m3u) emporte bien toutes les pistes.
  - Les feuilles .cue en ANSI (accents) ou avec marque UTF-8 sont lues.
- **Stabilité** : un fichier .cso ou .nsp abîmé, ou une image disque bizarre, ne peut plus fermer Frogtend ni le
  faire chercher des heures ; un jeu DS zippé est dit « pas vérifiable » au lieu de « pas compatible ».
- **DLC** : les dossiers ajoutés à Eden sont écrits avec des « / » (format des réglages Qt) ; les licences PS3 vont
  dans le même compte RPCS3 que celui du jeu, et regarder la liste ne crée plus de compte.
- **Confort** : régler le dossier des abris et le démarrage de Frogtend ne figent plus la fenêtre pendant les copies.

## 0.43.0 — décompresser pour jouer, et une vraie page d'aide

- Un jeu à disque rangé dans un **.zip** ou un **.7z** (tes jeux PS3, par exemple) : le panneau du jeu propose
  **🗜 Décompresser pour jouer**. Taille, dossier et durée annoncés ; rien ne se fait sans ton oui ; l'archive est
  gardée ; ▶ Jouer lance ensuite le jeu décompressé.
- **❓ Aide** explique maintenant tout Frogtend : versions d'un jeu, succès, mises à jour et DLC, jeux offerts, le
  menu en jeu, Taodbox, et où Frogtend range les choses.

## 0.42.0 — Wii U : les mises à jour et DLC d'un .wua

- Le panneau d'un jeu Wii U montre les **mises à jour et DLC** contenus dans son fichier .wua (Cemu les charge tout
  seul : rien à installer). Chez toi : 35 jeux avec mise à jour, 12 avec DLC.

## 0.41.0 — mises à jour et DLC Switch dans Eden

- Frogtend trouve les **mises à jour et DLC Switch** (.nsp) et les rattache à leur jeu.
- Dans le panneau du jeu, **📦 Contenus additionnels** ▸ **Rendre disponible** : Eden lit directement le dossier
  (rien n'est copié), seulement si tu dis oui.
- À l'import, une mise à jour ou un DLC n'est plus pris pour un jeu.

## 0.40.0 — les DLC et contenus additionnels (PS3)

- Frogtend **trouve** les DLC, avatars et thèmes PS3 (.pkg et licences .rap, même zippés) et les **rattache** à leur
  jeu.
- Dans le panneau du jeu, **📦 Contenus additionnels** : ce qui est disponible et ce qui est installé pour toi ; tu
  coches, Frogtend annonce la taille, et **installe seulement si tu dis oui** (par RPCS3, licence dans TON compte).
- À l'import, un zip de DLC n'est plus pris pour un jeu : il est annoncé comme contenu additionnel.

## 0.39.0 — succès : PS3, Jaguar CD et PSP compressé

- RetroAchievements sait maintenant vérifier les jeux **PS3** (.iso, .chd, ou jeu en dossier), **Jaguar CD** et les
  **PSP compressés (.cso)**.
- Un jeu zippé trop gros pour être lu tel quel (tes .iso PS3) : Frogtend te dit de le décompresser d'abord.
- Avec ça, toutes les consoles de RetroAchievements que tu peux avoir sont couvertes.

## 0.38.0 — succès : PS2, PSP, DS, PC Engine CD, PC-FX, Neo Geo CD

- RetroAchievements sait maintenant vérifier les jeux **PlayStation 2**, **PSP**, **Nintendo DS**, **PC Engine CD**,
  **PC-FX** et **Neo Geo CD**.
- Les images de CD à plusieurs pistes (.cue) sont lues piste par piste.
- Pas de jeux de ces consoles sur ton disque : vérifié par les tests seulement, à confirmer avec de vrais jeux.

## 0.37.0 — succès : 3DO, PC Engine, GameCube et Wii

- RetroAchievements reconnaît maintenant tes jeux **3DO**, **PC Engine** (même en .7z), **GameCube** et **Wii** (même
  en .rvz). Chez toi : 3DO 200/200, PC Engine 455/455, GameCube 81/81, Wii 99/99.
- Une version vérifiée l'est une fois pour toutes : Frogtend s'en souvient même après un redémarrage (un jeu Wii
  demande ~30 s la première fois).
- La liste des tests (docs/A-TESTER.md) commence par un parcours conseillé.

## 0.36.0 — GOG, Prime Gaming et les succès par GOG Galaxy

- **🛒 Boutiques ▸ 🎁 Jeux offerts** : **GOG** (le jeu offert du moment) et **Prime Gaming** (les jeux offerts aux
  membres Prime) se récupèrent tout seuls, sur leurs pages officielles, avec ta connexion à toi. Ce qui demande un code
  ou un compte à relier s'ouvre pour que tu finisses.
- Cases « les récupérer tout seul » : GOG une fois par jour, Prime une fois par semaine.
- Les jeux GOG, Xbox et EA importés par GOG Galaxy montrent leurs **succès** (Galaxy ne connaît pas ceux d'Epic).

## 0.35.0 — les .chd et la Dreamcast

- Frogtend lit les images **.chd** : tes **135 jeux Dreamcast** sont reconnus par RetroAchievements (chez toi : 135
  sur 135), et les CD Sega CD, Saturn et PlayStation en .chd aussi.
- Une empreinte calculée n'est pas recalculée tant que le fichier ne change pas.

## 0.34.0 — les succès des jeux sur CD

- RetroAchievements reconnaît maintenant tes jeux **Sega CD, Saturn et PlayStation** (images .cue/.bin, .ccd/.img,
  .iso, listes .m3u) : Frogtend lit le disque comme RetroAchievements le fait.
- Chez toi : les 56 jeux Sega CD sont lus.
- Pas encore : les .chd (tes jeux Dreamcast) ; Frogtend le dit au lieu de prétendre que ta version n'est pas reconnue.

## 0.33.0 — gagner les succès en jouant

- ⚙ Options ▸ Comptes ▸ 🏆 RetroAchievements ▸ **🎮 Connecter mes émulateurs** : ton mot de passe une seule fois ;
  Frogtend garde le jeton de connexion dans le coffre de Windows et oublie le mot de passe.
- À chaque partie, **RetroArch et PCSX2 jouent avec le compte du profil qui joue**. DuckStation demande la connexion
  une fois ; Frogtend la garde ensuite pour ton profil.
- Un profil sans compte a les succès coupés : personne ne joue avec le compte d'un autre.

## 0.32.0 — les succès

- **⚙ Options ▸ Comptes ▸ 🏆 RetroAchievements** : ton compte et ta clé (vérifiés, puis rangés dans le coffre de
  Windows ; chaque profil a le sien).
- Dans le panneau d'un jeu émulé : **ta version est-elle compatible RetroAchievements ?** Sinon, Frogtend te dit
  quelle version l'est — une de tes autres versions, ou celle qui manque (à demander à Firehouse). Et tes succès
  obtenus.
- Les jeux Steam montrent leurs **succès Steam** (obtenus / total).
- Pas encore : les jeux sur CD (PlayStation…), et gagner les succès en jouant (le compte à régler dans RetroArch).

## 0.31.0 — une fiche par jeu, toutes ses versions

- À l'import, les ROM d'un même jeu (France, Europe, USA, Japon, révisions…) sont réunies sous **une seule fiche**.
- Ses versions sont rangées dans ton ordre : **français, européen, américain/anglais, puis les autres** ; à région
  égale, la meilleure copie d'abord ([!]). Les disques d'un même jeu restent ensemble.
- **▶ Jouer** lance la version préférée ; **▶ Jouer avec…** en propose une autre pour la partie ; ⚙ Gérer le jeu ▸
  **📀 Version** change celle d'habitude.
- Une nouvelle région importée plus tard rejoint la fiche existante. Chez toi : 1 086 ROM NES → 828 fiches.

## 0.30.0 — pas de pieuvre : rien d'éparpillé sur le disque

- Ce que Frogtend ajoute à un jeu (jaquette, fiche, documents, copie avant un mod) va **dans le dossier du jeu**, sous
  « Frogtend ». Les jeux déjà sur le PC sont rangés tout seuls au démarrage (copie vérifiée).
- ⚙ Options ▸ Emplacements ▸ **Les autres dossiers de Frogtend** : émulateurs, **outils** (Cheat Engine…) et **abris
  de parties**. Sans dossier des abris, Frogtend refuse de retirer un jeu qui a des parties : elles ne se perdent pas.
- 📥 Importer : **laisser** les jeux où ils sont (le dossier devient une source du système) ou **les copier** dans
  l'emplacement du système (pistes de CD et CHD compris ; les originaux restent).

## 0.29.0 — installer un jeu DOS

- **📥 Importer ▸ Installer un jeu DOS** : depuis le dossier d'un CD, une image (.iso, .cue) ou une disquette (.img).
  DOSBox s'ouvre avec le disque en D: et le dossier du jeu en C: ; tu installes, tu tapes EXIT, et Frogtend te propose
  le programme qui lance le jeu.
- Le jeu se relance ensuite directement, avec ses chemins d'installation.
- Le menu « Importer » contient maintenant toutes les entrées de LaunchBox.

## 0.28.0 — MAME Arcade Full Set

- **📥 Importer ▸ MAME Arcade Full Set** : Frogtend trie ton dossier MAME avec la liste MAME de ton LaunchBox
  (choisie une fois) et garde les vrais jeux d'arcade jouables, comme LaunchBox : sans clones, BIOS, machines à sous,
  mahjong, machines mécaniques, jeux qui ne marchent pas… Chaque catégorie se coche si tu la veux quand même.
- L'année, l'éditeur et le genre de chaque jeu viennent avec.
- Chez toi : 14 191 zips → 4 526 jeux, en 3 secondes.

## 0.27.0 — importer les jeux déjà sur ton disque

- **📥 Importer ▸ Fichiers ROM** : choisis un dossier et une plateforme ; Frogtend propose les extensions de ses
  émulateurs, liste les jeux trouvés (un jeu par disque, pas par piste), tu coches, il les ajoute.
- **Jeux MS-DOS** (un sous-dossier par jeu, avec le programme qui le lance), **Jeux Windows** (le .exe) et
  **Ajouter un jeu manuellement**.
- Rien n'est copié, déplacé ni renommé. Retirer un jeu importé l'enlève de Frogtend, **jamais de ton disque**.
- Filtre « Boutique ▸ Importés de mon disque » dans la ludothèque.
- Encore à venir : MAME Arcade Full Set, Installer un jeu DOS, et les jaquettes des jeux importés.

## 0.26.0 — les jeux PS Plus du mois, sans les oublier

- **🛒 Boutiques ▸ 🎁 Jeux offerts ▸ 🎮 PlayStation Plus** : connecte-toi une fois au PlayStation Store (chaque
  profil a sa connexion ; Frogtend ne voit jamais ton mot de passe), puis « Ajouter les jeux du mois ».
- Frogtend parcourt les jeux PS Plus sans fenêtre et ne clique que sur « Ajouter à la bibliothèque » : il ne peut rien
  acheter. Les jeux vont dans ta bibliothèque PlayStation, pas dans la ludothèque de Frogtend.
- Case **« J'ai PS Plus »** (désactivée par défaut) : une fois par semaine, à l'ouverture de ton profil.
- Première version : à régler avec toi au premier essai.

## 0.25.0 — le menu « Importer », comme LaunchBox

- **📥 Importer ▾** dans la barre du haut : la même liste que LaunchBox (Fichiers ROM, MS-DOS, MAME, Amazon, EA,
  Epic, GOG, Steam, Ubisoft, Windows, Xbox, ajout manuel, installation DOS).
- Chaque boutique a sa fenêtre : Steam par ton compte (ou GOG Galaxy) ; Amazon, EA, Epic, GOG, Ubisoft et Xbox par
  **GOG Galaxy**, qui regroupe les comptes que tu y as reliés. Frogtend dit combien de jeux il a trouvés pour chacune.
- **❓ Aide** et **⚙ Options ▸ Comptes ▸ GOG Galaxy** expliquent comment relier tes comptes dans Galaxy.
- Les imports de jeux locaux (ROM, MS-DOS, MAME, Windows…) arrivent au prochain lot : ils sont déjà dans le menu,
  marqués « prochain lot ».

## 0.24.0 — les jeux offerts d'Epic, obtenus tout seuls

- **🛒 Boutiques ▸ 🎁 Jeux offerts** : les jeux gratuits d'Epic de la semaine, avec leur date de fin.
- **🎁 Obtenir** : Frogtend ouvre la page officielle d'Epic sans la montrer et clique pour toi (« Get », puis la
  commande à 0 €). Si Epic demande une connexion ou une vérification, sa page s'affiche et tu finis toi-même.
- **🔑 Se connecter à Epic** une fois : chaque profil a sa propre connexion ; Frogtend ne voit jamais ton mot de passe.
- Case **« Les obtenir tout seul »** : une fois par jour, à l'ouverture de ton profil.

## 0.23.0 — tes jeux de boutiques dans la ludothèque

- Les jeux de tes boutiques (Steam, et par GOG Galaxy : GOG, Epic, Xbox, Ubisoft, EA), **installés ou non**, sont
  maintenant **dans « Ma ludothèque », avec les autres**, sous **Windows**.
- Deux nouveaux filtres : **Boutique** et **Installés / Pas installés**.
- Un jeu de boutique se lance (▶ Jouer) ou s'installe (⬇ Installer) par Steam ou GOG Galaxy, depuis le panneau de
  droite ou d'un double-clic.
- Le Catalogue Firehouse reste celui de Firehouse : les jeux de tes boutiques n'y apparaissent pas.

## 0.22.0 — GOG Galaxy : GOG, Epic, Xbox, Ubisoft et EA

- **🛒 Boutiques ▸ GOG Galaxy** : Frogtend lit (sans rien y changer) la bibliothèque de GOG Galaxy, qui regroupe tes
  jeux GOG et ceux des boutiques que tu y as reliées : **Epic, Xbox, Ubisoft, EA**… Sur ton PC : 324 jeux, avec leurs
  jaquettes. Aucune connexion à donner : Galaxy est déjà connecté.
- Filtre par boutique, recherche, « installés sur ce PC » ; un clic ouvre le jeu dans GOG Galaxy pour y jouer ou
  l'installer.
- **Taodbox** montre aussi tes jeux installés via GOG Galaxy.

## 0.21.0 — tes jeux Steam en jaquettes, et dans Taodbox

- **🛒 Boutiques** montre tes jeux Steam en **grandes jaquettes** (les images officielles de Steam, gardées sur ce PC).
  Un clic : ▶ Jouer s'il est installé, sinon ⬇ Installer (par Steam).
- **Taodbox** : tes jeux Steam installés apparaissent dans une section « Steam », jouables à la manette.

## 0.20.0 — tes jeux Steam (lot 9, début)

- **⚙ Options ▸ Comptes ▸ Steam** : l'URL de ton profil Steam et ta propre clé d'API Steam (comme dans LaunchBox).
  Frogtend vérifie le compte auprès de Steam avant de l'enregistrer ; la clé va dans le coffre de Windows et ne
  s'affiche plus jamais. Chaque profil a son compte.
- **🛒 Boutiques** (barre du haut) : importe la liste de tes jeux Steam (si ton compte n'est pas encore réglé,
  Frogtend te le demande d'abord), cherche, filtre les jeux installés sur ce PC, **▶ Jouer** ou **⬇ Installer** :
  Steam fait le reste.
- Ces jeux restent dans ton Frogtend, jamais dans Firehouse.

## 0.19.1 — Cheat Engine s'installe depuis Firehouse

- **Cheat Engine 7.7** (la version propre préparée par Seb, avec ses extensions) est maintenant servi par Firehouse :
  **⬇ Installer Cheat Engine** le télécharge depuis Firehouse (avec reprise si la connexion coupe), vérifie qu'il
  est intact, puis l'installe.

## 0.19.0 — Cheat Engine se branche tout seul sur le jeu

- Pendant une partie, **🧰 Cheat Engine** dans le menu en jeu (touche Pause/Attn), ou **🧰 Brancher Cheat Engine sur
  le jeu** dans 🎯 Triches et mods : Cheat Engine s'ouvre déjà branché sur le jeu, sans chercher le jeu dans une
  liste. Pratique pour tous, enfants compris.
- Firehouse ajoutera à Cheat Engine des extensions choisies (mémoire des émulateurs, jeux MS-DOS, texte plus grand).

## 0.18.0 — Cheat Engine, chacun ses réglages

- **🧰 Cheat Engine** (🎯 Triches et mods, sur la fiche d'un jeu) : Frogtend l'installe depuis Firehouse (la version
  propre préparée par Seb, sans logiciels en plus) et le lance.
- **Chacun ses réglages** : Cheat Engine range les siens dans le registre de Windows. Frogtend remet ceux de TON profil
  avant de le lancer et les range dans ton profil quand il se ferme : le registre reste propre, et rien n'est jamais
  retiré sans copie (la clé d'avant Frogtend est gardée à part).

## 0.17.1 — avec les vraies données de Firehouse

- Les **triches, mods** et **émulateurs décrits par Firehouse** fonctionnent avec ses routes, en service depuis
  Firehouse 2.26.0 : 36 000 codes (RetroArch, DuckStation, PPSSPP, PCSX2, Dolphin), 23 émulateurs, dont pour la
  Switch **Eden**, **Citron Neo** et **Ryujinx (Ryubing)**.
- Le programme d'un émulateur est trouvé même dans un sous-dossier de son paquet (Cemu).
- RetroArch : les codes vont dans le dossier du cœur choisi pour le jeu.
- Quand une source ne répond pas (Ryubing aujourd'hui), Frogtend propose d'ouvrir la page officielle pour l'installer
  soi-même.
- Si des codes ont été trouvés par le titre (autre région possible), Frogtend le dit et garde le nom de la base.

## 0.17.0 — triches et mods, émulateurs décrits par Firehouse (lot 8, 1re partie)

- **🎯 Triches et mods**, sur la fiche d'un jeu : les codes de triche de l'émulateur du jeu, les tables Cheat Engine
  et les mods connus, **fournis par Firehouse** jeu par jeu (ils arriveront quand Firehouse les servira). Un code
  s'ajoute dans le dossier de TON profil, sur ta demande.
- Avant d'aller chercher un mod, Frogtend **propose de copier le jeu** (taille annoncée) ; si tu refuses, il te
  prévient.
- Frogtend peut installer **n'importe quel émulateur que Firehouse décrit**, forks compris (Switch : Ryubing,
  Citron…), depuis sa source officielle, vérifié avant installation.

## 0.16.0 — Taodbox, le mode canapé (lot 5)

- **🛋 Taodbox** (barre du haut) : plein écran, tout en grand pour être lu de loin, tes jeux en grandes jaquettes
  (les derniers joués d'abord), filtre par console.
- **Tout se fait à la manette** : la croix ou le stick pour choisir, A pour valider, B pour revenir. Ça marche aussi
  pour choisir son profil (pavé du PIN à l'écran) et pour toutes les fenêtres de confirmation. Les flèches du
  clavier font pareil.
- A sur un jeu le lance (et l'installe d'abord s'il le faut) ; à la fin de la partie, Taodbox revient tout seul.
- Pour Sunshine/Apollo : `Frogtend.exe --taodbox` ouvre directement Taodbox, et « ⏻ Quitter » ferme tout.

## 0.15.0 — la Switch (Eden)

- **Nintendo Switch** : Frogtend sait installer **Eden** (depuis son site officiel, en mode portable) et lancer les
  jeux en plein écran. Firehouse ne recommandant aucun émulateur Switch, Frogtend propose Eden.
- **Chacun ses parties** : Frogtend démarre Eden avec l'utilisateur Switch qui porte le nom de ton profil. Crée-le
  une fois dans Eden (même nom que ton profil Frogtend).
- Frogtend ne fournit jamais les clés ni le micrologiciel de la console : ils viennent de ta Switch.

## 0.14.0 — l'assistant jeux (lot 7)

- **💬 Demander à l'assistant**, sur la fiche d'un jeu, ou **Assistant** dans la barre du haut : pose une question
  (lancement, réglages, astuces, solution…), l'assistant de Firehouse répond en quelques secondes, sources à l'appui.
- Quand il **propose une action** (lancer le jeu, l'installer, ouvrir une page…), Frogtend te la montre avec son
  risque et ne la fait qu'après ton **oui**. Ce qu'il ne sait pas encore faire sans risque (régler un émulateur,
  triches, mods) est montré mais refusé, avec la raison.
- La conversation reste en mémoire le temps de la session, n'est jamais écrite sur le disque, et s'efface quand on
  change de profil.

## 0.13.0 — les consoles récentes (lot 4d)

- Frogtend sait maintenant **installer et lancer** (sur ton accord, depuis leur source officielle, en mode portable) :
  **Xenia Canary** (Xbox 360), **xemu** (Xbox), **RPCS3** (PS3), **Cemu** (Wii U), **Azahar** (3DS), **Vita3K**
  (PS Vita). Comme pour les autres, il propose celui que Firehouse recommande, au premier jeu de la console.
- **Chacun ses parties** sur Xbox 360, Xbox (un disque dur virtuel par profil), PS3 (un compte RPCS3 par profil) et
  3DS. Sur Wii U et PS Vita, les parties sont encore communes à tous les profils du PC.
- **💿 Micrologiciel PS3…** (⚙ Options ▸ Émulateurs, sur RPCS3) : montre le fichier PS3UPDAT.PUP téléchargé sur le
  site de PlayStation, RPCS3 l'installe. Frogtend ne télécharge jamais les fichiers de console (BIOS, clés,
  micrologiciels).
- La Switch arrive plus tard (choix de l'émulateur à finir).

## 0.12.0 — demander un jeu (lot 6)

- **Demander un jeu**, dans la barre du haut : cherche un jeu dans toute la base LaunchBox de Firehouse (titre,
  console, année, studio). S'il n'est pas encore dans le Catalogue Firehouse, **📨 Demander** l'envoie à Firehouse,
  qui le fait valider, le cherche et le télécharge ; il apparaîtra ensuite dans le catalogue.
- Un jeu déjà dans le catalogue, ou déjà demandé, est signalé et ne se redemande pas.

## 0.11.0 — le menu en jeu, au clavier

- **Pendant une partie, la touche Pause/Attn ouvre le menu de Frogtend par-dessus le jeu** (réglable dans ⚙ Options
  ▸ Émulateurs : Arrêt défil, ou Ctrl+Maj+M). Le jeu se met en pause pendant que le menu est ouvert (émulateurs), et
  le menu propose, selon ce que l'émulateur sait faire :
  - ▶ **Reprendre** (ou Échap) ;
  - 💾 **Sauvegarde rapide** et 📂 **Charger la sauvegarde rapide** ;
  - 💿 **Disque suivant** (jeux sur plusieurs CD) ;
  - 📖 **Manuel et documents** du jeu (lus sans quitter le jeu pour les textes) ;
  - 🔄 **Recommencer** et ⏹ **Quitter le jeu** (toujours après une confirmation).
- Les flèches ↑ ↓ et Entrée suffisent : pas besoin de souris. La touche n'est prise qu'**pendant une partie** : le
  reste du temps, elle reste aux autres programmes.
- Pour un jeu PC, le menu propose Reprendre, les documents et Quitter (le jeu, lui, continue : il n'existe pas de
  pause sûre pour tous les jeux PC).
- La manette (Select + Start tenus) viendra après l'essai de la sonde.

## 0.10.0 — plusieurs émulateurs par console, au choix

- **Autant d'émulateurs que tu veux par console** (par exemple plusieurs cœurs RetroArch pour la Super Nintendo) :
  **⚙ Options ▸ Émulateurs** les liste par console, ⭐ celui par défaut (un clic sur l'étoile pour changer),
  ➕ pour en ajouter, ✕ pour en retirer (rien n'est désinstallé).
- **Un émulateur propre à un jeu** : **⚙ Gérer le jeu ▸ 🕹 Émulateur** (« Comme la console », ou un autre).
- **▶ Jouer avec…** à côté de ▶ Jouer (quand la console en a plusieurs) : un autre émulateur pour cette partie
  seulement.
- Tes réglages d'avant sont repris tels quels (un émulateur par console devient une liste d'un seul).
- **Préparation du menu en jeu** (à venir) : les émulateurs lancés par Frogtend se mettent maintenant **en pause
  quand tu passes à une autre fenêtre** (Alt+Tab), et Dolphin se met en plein écran sans bordure. Frogtend leur
  règle aussi des touches F13 à F16, que ton clavier n'a pas, pour les piloter bientôt depuis son menu. Tes propres
  raccourcis sont gardés.

## 0.9.1 — les parties à l'abri partent aussi dans la sauvegarde

- Correction importante : les parties **mises à l'abri** (avant de retirer ou réinstaller un jeu) n'entraient pas dans
  la sauvegarde du profil. Un jeu retiré du PC, ou téléchargé mais pas encore réinstallé (comme Dune), pouvait donc
  perdre ses parties après un reformatage. Maintenant, la copie la plus récente de chaque jeu part avec la
  sauvegarde, et elle est reposée quand le jeu est réinstallé.

## 0.9.0 — tes réglages de manette, jeu par jeu (lot 4c)

- **🎮 Commandes, jeu par jeu** (⚙ Gérer le jeu ▸ 🎮 Commandes) : **Automatique**, **Clavier et souris** (Frogtend ne
  règle alors aucune manette pour ce jeu), ou un **réglage de manette de référence** (par exemple « Wiimote
  horizontale » pour un jeu Wii qui se joue la Wiimote couchée).
- **📌 Réglages de référence** (Options ▸ Émulateurs, pour Dolphin, DuckStation et PCSX2) : règle une manette dans
  l'émulateur, enregistre-la comme profil avec son bouton « Enregistrer », puis fais-en une référence. Elle est
  proposée pour chaque jeu, et on la retrouve dans les profils de l'émulateur de chacun. Garde le nom d'une
  référence existante pour la remplacer.
- **Wii** : deux réglages de départ sur manette Xbox, **« Wiimote + Nunchuk »** (utilisé d'office : stick gauche =
  Nunchuk, stick droit = pointeur, RB = secouer) et **« Wiimote horizontale »** (croix ou stick gauche, A = 2,
  X = 1, gâchettes = pencher). Ce sont des départs : tu les remplaceras par les tiens. Une Wiimote que tu as retouchée
  à la main dans Dolphin n'est jamais défaite.
- **DS** (par RetroArch et son cœur melonDS DS) : l'écran tactile suit la souris au bureau, ou un stick à la
  manette, tout seul.
- Messages plus clairs quand Firehouse est plein (sauvegarde), en maintenance, ou quand le jeton est refusé (il se
  crée maintenant dans 👤 Mon compte ▸ 🔌 Mes jetons).

## 0.8.0 — la manette marche d'emblée (fin du lot 4)

- **Branche ta manette, joue** : au premier lancement d'un jeu, Frogtend règle la manette du joueur 1 dans
  l'émulateur si elle ne l'est pas encore. DuckStation et PCSX2 : manettes Xbox, PlayStation, Switch Pro, 8BitDo et
  génériques ; **Select + Start** ouvre leur menu de pause. Dolphin : manette GameCube sur une manette Xbox.
  RetroArch : ses profils de manette officiels (ajoutés s'ils manquent) ; **L3 + R3** ouvre son menu. PPSSPP et
  DOSBox Staging reconnaissent déjà les manettes tout seuls.
- **Rien n'est défait** : une manette que tu as réglée toi-même dans un émulateur n'est jamais touchée, et les
  touches du clavier sont gardées. **Options ▸ Émulateurs ▸ 🎮 Manette par défaut** remet le réglage de Frogtend, sur
  ton accord ; la configuration d'avant est copiée à part.
- Correction : « Chercher des mises à jour » des émulateurs ne voyait jamais de nouvelle version.
- **Sauvegarde et restauration du profil** (Options ▸ Sauvegarde) : prêtes, elles fonctionneront dès que Firehouse
  ouvrira ses routes de sauvegarde. D'ici là, la sauvegarde automatique se tait et le bouton explique que Firehouse
  ne la propose pas encore.

## 0.7.1 — une seule barre, des Options rangées

- **Une seule barre fine en haut**, comme LaunchBox : Frogtend dessine sa propre barre de titre (logo, menus en
  petites capitales, compte des jeux, profil, Options) avec les boutons réduire / agrandir / fermer. On la saisit
  pour déplacer la fenêtre ; un double-clic l'agrandit. Plus de double barre.
- **⚙ Options** remplace « Réglages » et « À propos » : une arborescence à gauche (Général ▸ Apparence, Mon profil ;
  Ludothèque ▸ Affichage ; Jeux ▸ Emplacements, Émulateurs ; Connexion ▸ Firehouse ; Frogtend ▸ À propos et mises
  à jour), une seule rubrique à droite. Prêt pour tout ce qui va s'ajouter.

## 0.7.0 — chacun ses parties, rien qui déborde (lot 4, suite)

- **Chaque profil a ses propres parties** dans les émulateurs : parties, états, codes de triche et cartes mémoire
  sont rangés dans `<émulateur>\Profils\<ton profil>\`, à côté de l'émulateur. Personne n'écrase la partie d'un
  autre (RetroArch, DuckStation, PCSX2, Dolphin, PPSSPP). Pour PPSSPP, un dossier `memstick` déjà rempli est mis
  de côté sous `memstick (avant Frogtend)` : rien n'est effacé.
- **Les dossiers de jeux des émulateurs** pointent vers tes emplacements Frogtend.
- **Pas d'« effet pieuvre »** : DOSBox Staging s'installe maintenant vraiment en mode portable, PPSSPP ne part plus
  dans Documents. **Réglages ▸ Émulateurs** signale les dossiers qu'un émulateur aurait laissés dans ton dossier
  utilisateur Windows (sans les toucher).
- Toute configuration d'émulateur modifiée par Frogtend est d'abord copiée dans `.frogtend-sauvegardes`.
- Les jeux PC et DOS natifs (comme Dune) gardent leurs parties dans leur propre dossier, commun au PC : la
  sauvegarde du profil (lot 3 bis) les couvrira.

## 0.6.0 — les émulateurs (lot 4, 1re partie)

- **Frogtend installe les émulateurs pour toi**, sur accord : au premier lancement d'un jeu qui en a besoin, il
  propose l'émulateur recommandé par Firehouse, annonce sa taille et sa source officielle, puis l'installe dans ton
  dossier « Émulateurs » (choisi la première fois). Émulateurs connus : **RetroArch**, **DOSBox Staging**,
  **DuckStation**, **PCSX2**, **Dolphin**, **PPSSPP**. Chacun est installé en **mode portable** : sa configuration
  reste dans son dossier.
- **Déjà installé à la main ?** Frogtend le retrouve dans ses dossiers habituels (sans fouiller tout le disque) et
  s'en sert.
- **RetroArch** : le cœur qui manque pour une console s'installe sur accord ; un **BIOS** manquant est signalé avec
  le dossier où le mettre (Frogtend ne télécharge pas les BIOS).
- **Réglages ▸ Émulateurs** : le dossier des émulateurs, ceux installés et leur version, **« Chercher des mises à
  jour »** puis « ⬆ » pour mettre à jour. Une mise à jour garde ta configuration ; un fichier de réglages remplacé
  est d'abord copié à part.
- Lignes de commande (plein écran, fermeture avec le jeu) relevées dans les sources officielles de chaque émulateur.
- Pas encore : les profils de manette (2e partie du lot 4).

## 0.5.1 — plus facile à trouver

- **« ⚙ Gérer le jeu »**, bien visible sous « ▶ Jouer » (et sur la fiche du jeu) : installer, changer ce qui lance
  le jeu, mettre tes parties à l'abri, **retirer du PC (désinstaller)**. Ces actions étaient cachées dans une ligne
  repliée « Autres actions ».
- **Fiche d'un jeu de ta ludothèque** : « 📦 Installer » ou « ▶ Jouer » directement en haut.
- **Emplacement propre à un système** (ex. `E:JeuxMS-DOS`) : les jeux y vont directement, sans sous-dossier
  « MS-DOS » en double.
- Au lancement d'un jeu sur Venkman : « 🎮 Session de jeu annoncée à Firehouse ».

## 0.5.0 — installer et jouer (lot 3)

- **📦 Installer** un jeu téléchargé, selon sa nature :
  - installeur Inno Setup ou NSIS (comme les jeux « prêts à jouer » d'Abandonware France) : **installation
    automatique** dans le dossier du jeu, ou guidée si tu préfères ;
  - archive zip ou 7z : décompressée ;
  - ROM ou image disque : rien à installer, le fichier n'est jamais renommé ;
  - repack ou installeur d'origine : l'installeur s'ouvre, Frogtend te dit quel dossier choisir.
  Les notes de LA version sont montrées avant, et Frogtend prévient si Windows va demander son accord.
- **🎯 Ce qui lance le jeu** : Frogtend propose ce qu'il a trouvé (le plus probable en premier), tu confirmes une fois.
- **▶ Jouer** : en natif, ou par l'émulateur du système. Le premier lancement d'une ROM propose l'émulateur
  recommandé par Firehouse (RetroArch et sa ligne de commande), il suffit de montrer où il est installé. Le temps de
  jeu est compté.
- **Firehouse est prévenu** du début et de la fin de chaque partie (et toutes les 10 minutes pendant) : sur Venkman,
  il libère la carte graphique pendant que tu joues.
- **💾 Tes parties sont protégées** : à l'installation, Frogtend relève les fichiers du jeu ; avant de retirer un jeu
  (ou à la demande), tout ce qui a changé depuis — tes parties — est copié à l'abri. Les abris ne s'effacent jamais.
- **🗑 Retirer du PC** : parties à l'abri d'abord (sinon rien n'est retiré), puis le désinstalleur du jeu, puis
  seulement ce que Frogtend a mis sur le disque.
- **Réglages ▸ Émulateurs** : l'émulateur de chaque système.

## 0.4.0 — ta ludothèque, sur ton PC

- **« 🎮 Ma ludothèque » ne montre que les jeux de ce PC.** Les autres se parcourent dans le **« 🛒 Catalogue
  Firehouse »**, d'où l'on choisit ce qu'on met dans sa ludothèque.
- **Mettre un jeu dans sa ludothèque** : choisis la version, l'emplacement (seuls ceux qui ont la place sont
  proposés), lis le récapitulatif (taille, place restante, durée estimée) et accepte : le téléchargement commence.
  Il reprend où il en était s'il est coupé, et chaque fichier est vérifié à l'octet près.
- **Page « ⬇ Téléchargements »** : progression, débit, temps restant, pause, reprise, annulation (qui n'efface que
  les fichiers du jeu).
- **Sans Internet**, ta ludothèque reste utilisable : la fiche, la jaquette et les documents (solution, astuces,
  manuel…) de chaque jeu sont gardés sur le PC. Sans connexion, on perd seulement l'ajout de nouveaux jeux.
- **Réglages ▸ Emplacements des jeux** : un ou plusieurs dossiers par défaut, et des dossiers propres à un système
  (SNES, MS-DOS…), dans l'ordre de préférence. **À régler avant le premier ajout.**
- Chaque profil ne voit que les jeux du PC que Firehouse lui montre.
- Jaquettes plus fiables (un échec est retenté ; plus d'icône d'image cassée) ; journal des échecs pour comprendre
  un problème.
- Pas encore : installer et lancer un jeu (lot 3).

## 0.3.0 — plus léger, plus rapide, à jour avec Firehouse

- **Jaquettes allégées** : Frogtend demande des miniatures à la taille affichée (par exemple 200 pixels de large au
  lieu de l'image complète de 3 Mo), et ne retélécharge une jaquette que si elle a changé dans Firehouse.
- **Synchronisation rapide** : après la première, seuls les jeux modifiés sont relus ; un jeu qui n'est plus
  visible pour ton profil disparaît de ta ludothèque.
- **Le développeur sous chaque jaquette** (réglable : éditeur, année, plateforme ou rien), et dans les détails.
- **Tes skins dès l'accueil** : l'écran « Qui joue ? » prend les vrais skins de Firehouse, avec le **fond vidéo**
  du skin Firehouse (désactivable dans « Réglages »).
- **Enregistrer ton skin** dans ton compte Firehouse, depuis « Réglages » : il te suit aussi dans le cockpit.
- **Le jeton est vérifié** auprès de Firehouse avant d'être rangé (« Jeton vérifié : Seb (admin) ») ; « Mon profil »
  affiche le compte Firehouse du profil.
- **Versions incompatibles** : si Firehouse et Frogtend ne parlent plus la même version, Frogtend le dit clairement
  (« mets Frogtend à jour » ou « Firehouse doit être mis à jour »).
- Les cases à cocher et curseurs prennent la couleur du skin.

## 0.2.0 — la ludothèque (lot 1)

Première version utilisable : Frogtend affiche la ludothèque de Firehouse.

- **Qui joue ?** Au lancement, chacun choisit son profil. Un profil peut être protégé par un **code PIN**
  (après 5 erreurs, il est bloqué une minute). Chaque profil a son **jeton Firehouse**, rangé dans le coffre de
  Windows, jamais affiché.
- **La ludothèque, façon LaunchBox** : les plateformes à gauche (classées par catégorie), les jaquettes au centre,
  les détails du jeu choisi à droite, avec « 🎲 Jeu au hasard ». La recherche ignore les accents ; un filtre par
  genre et trois tris (titre, année). En haut : « Affichage de X jeux sur Y ».
- **La fiche d'un jeu** : le résumé, les informations, les documents à lire (lancement, solution, astuces…), puis
  chaque version avec ses fichiers, leur taille et ses notes.
- **Hors ligne** : la ludothèque déjà synchronisée et les fiches déjà lues restent consultables.
- **Les skins de Firehouse** : Frogtend applique celui que tu as choisi dans Firehouse ; tu peux en prendre un
  autre dans « Réglages ».
- **Réglages de la ludothèque** : taille des jaquettes, ligne sous le titre, tri, colonnes affichées, plateformes
  masquées. **Mon profil** : nom, code PIN, jeton, suppression.
- **Adresse de Firehouse** par défaut : `https://jeux.hikari-no-sekai.fr` (HTTPS obligatoire hors de la maison).
  Un mode simulé (des exemples, sans connexion) reste disponible.
- Pas encore : télécharger, installer et lancer un jeu (lot 2 et lot 3).

## 0.1.0 — le socle (lot 0)

Premier Frogtend installable. Il n'affiche pas encore de jeux : c'est le lot 1.

- **Installation** : un installateur Windows en français, sans droits administrateur. Au premier lancement, Windows
  peut afficher « éditeur inconnu » (pas de certificat, c'est voulu) : « Informations complémentaires » puis
  « Exécuter quand même ».
- **Mises à jour** : au démarrage, Frogtend regarde s'il existe une nouvelle version. S'il y en a une, il la
  propose et ne l'installe qu'après ton accord. La recherche se lance aussi à la main depuis « À propos ».
- **Skins** : les 44 skins de Firehouse. Par défaut, Frogtend suivra celui choisi dans ton compte Firehouse (quand
  l'API sera prête) ; tu peux en choisir un autre dans « Réglages ».
- **Réglages** : taille du texte, animations, adresse de Firehouse (par défaut `https://jeux.hikari-no-sekai.fr`).
  Chaque réglage peut revenir à sa valeur d'origine.
- **Mode simulé** : tant que l'API de Firehouse pour Frogtend est en construction, Frogtend ne se connecte à rien.
