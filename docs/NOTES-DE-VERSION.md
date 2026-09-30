# Notes de version de Frogtend

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
