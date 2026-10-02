# Notes de version de Frogtend

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
