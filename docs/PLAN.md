# Frogtend — plan en lots

> Proposé le 29/09/2026, **à valider par Seb**. Un lot se termine (tests verts, version livrée, notes de version)
> avant que le suivant commence. Les idées en cours de route vont dans `docs/IDEES.md`.

## Décisions déjà prises (Seb, 29/09/2026)

- **Pile : Tauri 2.** Cœur en Rust (processus, fichiers, téléchargements, coffre Windows) et interface en
  **Svelte 5 + Vite + TypeScript**.
- **Identifiant de l'application** : `fr.hikari-no-sekai.frogtend` (définitif).
- **Dépôt** : https://github.com/Nimai26/Frogtend (public, licence Apache 2.0). Les **mises à jour** sont publiées
  dans les « releases » GitHub de ce dépôt : un installateur et un `latest.json` signés.
- **Pas de certificat de signature de code** : à la première installation, Windows affichera « éditeur inconnu ».
  C'est accepté. Les mises à jour, elles, restent signées par la clé propre à Tauri.
- **Profils protégés par un code PIN** (stocké sous forme d'empreinte, jamais en clair).
- **Charte graphique de Firehouse et ses skins** (`docs/CHARTE-GRAPHIQUE.md`) : tous les skins de Firehouse sont
  proposés (44 aujourd'hui). Ils sont téléchargés depuis Firehouse et gardés en cache, jamais recopiés. Le skin par
  défaut est celui que la personne a choisi dans Firehouse. Les cinq outils maison (Confirmer, Demander, Choisir,
  Informer, Toast), les composants et le ton suivent la charte.
- **Beaucoup d'éléments modifiables dans l'interface** (exigence de Seb). Voir « Personnalisation » plus bas.
- **Deux façons de piloter** (Seb, 29/09) : **Frogtend** (le mode bureau) se pilote **au clavier et à la souris**,
  comme une application normale ; **Taodbox** (le mode canapé) se pilote **à la manette** de préférence. Le bureau
  n'a pas à être pensé pour la manette, seulement à rester utilisable au clavier (focus visible, Tab, Entrée, Échap).
- **Frogtend n'est pas lié à Venkman.** Il s'installe sur n'importe quel PC Windows 10/11 par un **installateur**
  (NSIS, par utilisateur, sans droits administrateur ; WebView2 installé s'il manque). Il **se met à jour** lui-même
  par des mises à jour **signées**. Rien ne suppose une machine, un disque ou un réseau précis.
- **Adresse de Firehouse** réglable dans les options. Par défaut : `https://jeux.hikari-no-sekai.fr` (publique, HTTPS, ne publie que l'API jeux ; décision de Seb le 29/09, qui remplace `core.`). HTTPS obligatoire hors du réseau local.
- **Plusieurs profils** sur un même PC : chacun a son jeton (dans le coffre Windows), son cache, ses sauvegardes et
  ses réglages. Rien n'est partagé entre profils de ce qui vient de Firehouse.
- **On part de zéro** : pas d'import de LaunchBox.
- **Manettes** de la famille : Xbox (360 et suivantes), PS4, PS5, Switch Pro, 8BitDo, génériques. Les profils des
  émulateurs visent la manette XInput standard (§ 4 bis du brief), avec un profil « physique » possible à côté.
- Les essais en streaming (Sunshine + Moonlight) auront lieu plus tard, quand on pourra lancer des jeux.
- **« On complexifie le code pour simplifier l'utilisation »** (Seb, 30/09). Le but : que tout le monde, **même un
  enfant**, profite de tout sans jamais passer par les menus compliqués des émulateurs. Frogtend fait le travail
  (réglages, manettes, disques, triches…), la personne n'a qu'un menu simple, le même partout (voir le lot 4 ter).
- **Les réglages de référence de Seb** (30/09) : au fil du temps, Seb peaufinera lui-même des réglages dans les
  émulateurs (profils de manette, options…) pour qu'ils deviennent ceux **par défaut** de Frogtend. Frogtend doit
  donc savoir **reprendre un réglage fait par Seb** et le livrer à tous, sans que personne ait à le refaire.

## Outils présents sur Venkman (vérifié le 29/09)

git 2.51, Node 24.11, pnpm 10.32, Rust 1.94 (stable, MSVC), Visual Studio Build Tools 2022, WebView2 153.
**Rien à installer en global.** Les dépendances du projet restent dans le projet (`node_modules`, `target`).

## Les lots

### Lot 0 — Le socle (✅ construit en 0.1.0, jamais publié : remplacé par la 0.2.0)
- Squelette Tauri 2 + Svelte 5 + TypeScript. Tests : Vitest (interface) et `cargo test` (cœur), lancés ensemble par
  une seule commande.
- `VERSION`, `docs/NOTES-DE-VERSION.md`, script de livraison (vérifie les tests, monte la version, construit
  l'installateur).
- **Installateur Windows et mises à jour dès le départ** : l'installateur NSIS et le module de mise à jour de Tauri,
  avec une clé de signature des mises à jour rangée hors du dépôt. Il est plus simple de le faire au début que d'y
  revenir.
- Le moteur de skins : il applique les 11 jetons de base, les jetons communs et les encres résolues. Pour coder hors
  ligne, il lit l'instantané `docs/charte/themes.instantane.json`, qui n'est qu'une fixture. Un nom inconnu retombe
  sur `firehouse`. Le fond vidéo est réservé au skin `firehouse`. Le crédit Theme.Park figure dans « À propos ».
- Les cinq outils maison (Confirmer, Demander, Choisir, Informer, Toast) : ils retiennent le focus à l'intérieur de la
  fenêtre et le rendent à l'élément d'origine en fermant.
- Les composants de base de la charte (§ 5) et un focus visible partout.
- Le magasin de réglages, par profil, qui sert à la personnalisation.

### Lot 1 — Voir la ludothèque (✅ livré en 0.2.0 le 29/09/2026, vérifié sur le vrai Firehouse)
- Client de l'API `/api/jeux/v1/` avec un **mode simulé** (fixtures tirées du brief) et un mode réel, qu'on change
  dans les options.
- Premier lancement : création d'un profil, adresse de Firehouse, collage du jeton (qui va dans le coffre, n'est
  jamais affiché ni journalisé). Un 401 est dit clairement et on ne réessaie pas en boucle.
- Synchronisation du catalogue (paginée et incrémentale avec `depuis`), avec un cache local **par profil** (SQLite).
- Jaquettes en cache, par profil.
- Écrans : plateformes, puis une grille de jeux, une recherche et des filtres (genre, année, statut). Une fiche jeu
  à **deux niveaux** : le jeu d'un côté, ses versions avec leurs fichiers et leurs notes de l'autre. Les annexes se
  lisent sur la fiche.
- Changer de profil vide l'affichage et n'utilise jamais les données d'un autre profil.

**L'interface de la ludothèque (Seb, 29/09).** On s'inspire de **LaunchBox** (le mode bureau, pas Big Box) sans le
copier : elle doit être **plaisante et facile**, car des personnes qui ne sont pas expertes doivent s'y retrouver.
- **Trois colonnes** :
  - **à gauche**, la recherche (avec un bouton de filtres), puis l'arbre des plateformes regroupées par catégorie
    (Tout, Arcade, Consoles, Ordinateurs, Portables…), chacune avec son icône et son nombre de jeux ;
  - **au centre**, la grille des jaquettes, avec sous chacune le titre et le développeur (ou un autre champ, au
    choix) ;
  - **à droite**, un panneau de détails. Quand rien n'est sélectionné, il présente la plateforme (logo, image,
    année, fabricant) et propose un bouton « 🎲 Jeu au hasard ». Quand un jeu est sélectionné, il montre sa jaquette,
    ses informations et ses boutons « ▶ Jouer » / « ⬇ Installer ».
- **Une barre du haut** discrète : les menus et « Affichage de 794 jeux sur 20 682 », pour savoir où l'on en est
  d'un coup d'œil.
- Les jaquettes occupent l'essentiel de l'écran, et le reste de l'interface reste sobre.
- **Pour les non-experts** : des mots simples plutôt que des termes techniques, une action principale évidente par
  écran, un état vide qui explique quoi faire (« Aucun jeu ici : choisis une autre plateforme »). Aucune fonction
  ne se cache derrière un clic droit ou un survol : il y a toujours un bouton visible.
- Les colonnes peuvent être repliées ou élargies, et la taille des jaquettes se règle (voir « Personnalisation »).
- Le tout reste **dans la charte** : les couleurs viennent des skins. Pilotage au clavier et à la souris (la manette,
  c'est pour Taodbox).
- La barre de navigation actuelle du lot 0 (Ludothèque / Réglages / À propos) sera remplacée par cette mise en page,
  avec les réglages dans le menu du haut.

### Version 0.3.0 — le contrat 1.3 de Firehouse (✅ livrée le 29/09/2026, décision de Seb : avant le lot 2)
`/moi`, `X-Api-Jeux-Version`, `PUT /theme`, skins et vidéo sans jeton, miniatures, empreintes de jaquette,
synchronisation incrémentale (`depuis` + `ids_visibles`), développeur et éditeur.

### Changement de fonctionnement demandé par Seb (29/09, 23 h) — à valider avant le lot 2

1. **La ludothèque ne montre QUE les jeux présents sur ce PC** (ceux que la personne a choisi d'y mettre). Le
   catalogue de Firehouse se parcourt dans un espace À PART (« 🛒 Catalogue Firehouse »), d'où l'on choisit ce qu'on
   met dans sa ludothèque.
2. **Frogtend est utilisable sans Internet.** Tout ce qui sert à un jeu de la ludothèque est sur le PC : fiche,
   jaquette (en grand), documents à lire, et plus tard les autres médias. Sans connexion, on perd seulement l'ajout
   de nouveaux jeux et l'assistant.
3. Conséquence : le lot 2 (télécharger) devient « **mettre un jeu dans sa ludothèque** » (choisir la version et
   l'emplacement, télécharger le jeu ET tous ses médias).

**Réponses de Seb (30/09)** :
- **Ajouter = télécharger tout de suite** : choix de la version et de l'emplacement, taille annoncée, puis
  téléchargement du jeu ET de ses médias. Pendant le téléchargement, le jeu apparaît dans la ludothèque avec sa
  progression ; jamais avant d'avoir été choisi.
- **Le catalogue est un écran dans la fenêtre** (« 🛒 Catalogue Firehouse » dans la barre du haut), même présentation
  en trois colonnes, « ← Ma ludothèque » pour revenir.
- **Tous les documents sont gardés** avec le jeu (textes et fichiers comme les manuels) : tout est consultable hors
  ligne.
- **La version se choisit à chaque fois** (nature, taille, notes ; la plus récente présélectionnée).

**Décidé par l'agent, d'après les règles** : un jeu téléchargé est rangé UNE fois sur le PC ; chaque profil ne voit
dans sa ludothèque que les jeux du PC que Firehouse lui montre (son propre catalogue). Un jeu réservé aux admins,
téléchargé par un admin, reste invisible aux autres profils, même hors ligne.

### Lot 2 — Mettre un jeu dans sa ludothèque (✅ livré en 0.4.0 le 30/09/2026 ; vérifié : Dune, 237 887 038 octets, à l’octet près, puis hors ligne)
- **Deux espaces** : « Ma ludothèque » (les jeux du PC, visibles pour ce profil) et « 🛒 Catalogue Firehouse »
  (tout ce que Firehouse montre à ce profil, d'où l'on ajoute).
- **Emplacements par système** (réglages du PC, voir plus haut).
- **Ajouter** : choix de la version, de l'emplacement (le premier qui a la place), récapitulatif chiffré (taille,
  place libre, durée estimée au débit mesuré) et accord.
- **File de téléchargements** dans le cœur : reprise `Range`, fichiers `.part`, taille vérifiée, pause / reprise /
  annulation, progression et débit affichés, 409 dit clairement sans boucler. Aucun nom de fichier modifié (ROM,
  images disque).
- **Médias gardés sur le PC** pour chaque jeu : fiche complète, jaquette en grand, documents (textes et fichiers).
  La ludothèque, les fiches et les documents marchent hors ligne.
- L'installation (décompresser, lancer un `setup.exe`…) et le lancement : lot 3.

### Lot 3 — Installer et lancer (✅ livré en 0.5.0 le 30/09/2026 ; à essayer par Seb sur ses jeux)
L'installation suit la **qualité** de la version (prêt à jouer, repack, installeur d'origine, ROM, image disque),
sans jamais renommer une ROM ni une image. Le lancement se fait en natif ou par émulateur. Les sauvegardes de
parties sont localisées et sauvegardées avant toute réinstallation. Le début et la fin de chaque session sont
annoncés à Firehouse (`/session`).

### Lot 3 bis — Sauvegarder « tout ce qui ne se retélécharge pas » (Seb, 30/09)

**Décision de Seb (30/09, après l'essai réel du partage SMB, réussi mais joignable seulement à la maison) : la
sauvegarde passe PAR L'API FIREHOUSE, PARTOUT** (HTTPS, jeton du profil), pour que les autres foyers (la sœur de Seb…)
puissent aussi sauvegarder. Firehouse l'écrit dans le même dossier sur Shyrka (`<username>/<profil>/<PC>/`, historique
par instantanés ZFS). Frogtend n'utilise plus le partage SMB. Routes demandées à Firehouse le 30/09 (liste,
documents, fichiers par morceaux avec reprise et sha256, suppression). Codé sur réponses simulées (contrat 1.5, 30/09) : contenu, envoi incrémental par morceaux avec reprise, sauvegarde
automatique (après chaque partie / chaque jour / jamais), restauration (réglages, parties d'émulateurs, parties des jeux
reposées à leur réinstallation, jeux remis sur accord chiffré). Publié dans 0.8.0/0.9.0. **✅ Essai réel réussi le
30/09/2026** sur Firehouse 2.21.0, avec l'accord de Seb, dans un dossier d'essai (« Essai-Frogtend / Venkman-essai »,
fichiers fabriqués) : 3 fichiers envoyés (20 979 726 octets, dont 20 Mo en 3 morceaux), 2e envoi sans rien renvoyer,
reprise après coupure (`HEAD` → `X-Recu` 8 388 608, seule la fin renvoyée), nom à parenthèses et chemin accentué
relus identiques (sha256), puis tout retiré (`DELETE` du couple → 200, absent de la liste). **Reste : la première
vraie sauvegarde, faite par Seb** (Options ▸ Sauvegarde ▸ 💾 Sauvegarder maintenant).
**But : pouvoir reformater son PC, réinstaller Frogtend, et TOUT retrouver.** Par profil, une sauvegarde
**automatique ou manuelle** (au choix de la personne) de :
- la **configuration de Frogtend** (réglages du profil et du PC, emplacements, émulateurs réglés) ;
- la **bibliothèque** : la LISTE des jeux, avec la version choisie et l'emplacement. PAS les jeux ni les médias, ni
  les émulateurs : tout cela se retélécharge ;
- les **parties sauvegardées et les codes de triche de TOUS les jeux**, quel que soit l'émulateur : jeux PC natifs
  (dossier du jeu, Documents, AppData…), DOSBox, RetroArch (`saves`, `states`, `cheats`), etc.

À la restauration, Frogtend remet la configuration et les parties, puis propose de retélécharger les jeux de la
liste (annoncé et chiffré d'abord : combien de jeux, combien de Go).

Le lot 3 en pose la base : relevé des fichiers à l'installation, mise à l'abri de ce qui a changé (les parties)
avant tout retrait ou réinstallation. **Questions pour Seb** (avant de coder ce lot) : où ranger la sauvegarde
(dossier au choix, disque externe, NAS, ou Firehouse lui-même ?) ; fréquence de l'automatique ; combien de
versions garder.

**Décision de Seb (30/09)** : un **dossier réseau créé spécialement pour Frogtend**, avec un **sous-dossier par
personne**, mis en place avec l'agent Firehouse (transmis le 30/09). En attente : chemin du partage et moyen pour
Frogtend de le connaître (proposé : champ `sauvegarde` dans `/moi`), droits d'accès, conduite hors de la maison
(sauvegarde locale en attente, envoyée au retour), nombre de versions gardées.

**Proposition de Firehouse (30/09, soumise à Seb)** : espace ZFS dédié `Egon/frogtend`, partagé en SMB
`\10.10.0.1Frogtend<username>` ; chemin donné par `/moi` (`sauvegarde: {partage, disponible}`, contrat 1.4) ;
un compte SMB PAR PERSONNE (`frogtend-<username>`, droits 700), mot de passe demandé une fois par Frogtend et rangé
dans le coffre Windows ; hors de la maison, file locale envoyée au retour ; versions gardées côté serveur par
instantanés ZFS nocturnes (30 derniers) : Frogtend écrit seulement la version courante sous
`<personne>/<nom du PC>/` (configuration.json, bibliotheque.json, parties/ + manifeste sha256).

*(Proposition écartée par Seb : Nextcloud via Firehouse.)* ranger la sauvegarde chez la personne, dans son
**Nextcloud** (stockage maître de la maison, déjà couvert par les sauvegardes 3-2-1 chiffrées hors site), en passant
par Firehouse avec le jeton du profil : `PUT /api/jeux/v1/sauvegarde` (archive + manifeste : PC, date, jeux, taille,
empreinte) et `GET /api/jeux/v1/sauvegarde` (liste, téléchargement). Déposée dans « Frogtend/<nom du PC>/ » chez le
propriétaire du jeton, invisible des autres ; les N dernières gardées. Avantages : un seul chemin authentifié, aucun
identifiant Nextcloud dans Frogtend, restauration après reformatage avec le seul jeton. Taille attendue : de
quelques Mo (jeux rétro) à quelques centaines de Mo par profil (parties de jeux PC récents, états d'émulateurs),
parfois plus de 1 Go : envoi par morceaux avec reprise.

### Lot 4 — Les émulateurs
**Décisions de Seb (30/09)** :
- **Frogtend installe et met à jour les émulateurs, sur accord** : quand un système n'en a pas, il propose le
  recommandé (téléchargé depuis sa source OFFICIELLE, taille annoncée), l'installe dans un dossier « Émulateurs »
  réglable, et propose ses mises à jour. Les cœurs RetroArch manquants aussi.
- **Manettes : profils XInput standard** (manette Xbox, ce que Moonlight recrée en streaming) ; un profil par jeu
  possible ensuite. Pas de profil par modèle physique pour l'instant.
- **Priorités : RetroArch** (consoles rétro), **DOSBox** (MS-DOS), **consoles récentes** (Dolphin, PCSX2,
  DuckStation, PPSSPP… selon les recommandations de Firehouse).

**Exigence de Seb (30/09) : pas d'« effet pieuvre ».** (✅ en 0.7.0, avec les parties séparées par profil — décision de Seb) Pour simplifier l'entretien :
- TOUS les émulateurs (et outils du même genre) sont installés dans le **dossier « Émulateurs »** réglable dans
  l'interface (fait en 0.6.0) ;
- chacun est en **mode portable** : rien dans le dossier utilisateur de Windows (AppData, Documents). Relevé dans les
  sources officielles : `portable.txt` (DuckStation, Dolphin), `portable.ini` (PCSX2), `dosbox-staging.conf` à côté
  du programme (DOSBox Staging), pas d'`installed.txt` (PPSSPP), paquet .7z (RetroArch). Après un premier
  lancement, Frogtend VÉRIFIE que rien n'est parti dans les dossiers utilisateur habituels, et le signale sinon ;
- leurs **dossiers de jeux** (ROM, images) sont réglés sur les **emplacements de Frogtend** ;
- **parties, états, codes de triche, mods… restent à côté de l'émulateur**, et entrent dans la **sauvegarde du
  profil** (lot 3 bis).

Découpage : **4a** (✅ livré en 0.6.0 le 30/09/2026 ; installer, détecter, mettre à jour les émulateurs et les cœurs ; vérifier les BIOS ; régler
chaque système tout seul d'après Firehouse) puis **4b** (profils de manette XInput, profils par jeu, copie de
sauvegarde de toute configuration modifiée).

**4b, profils standard : ✅ livré en 0.8.0.** Relevé dans les sources officielles : DuckStation et PCSX2 lisent les
manettes par SDL (Xbox, PlayStation, Switch Pro, 8BitDo, génériques) → Frogtend écrit le joueur 1 en `SDL-0/…` en
gardant le clavier, plus Select + Start pour le menu de pause ; Dolphin → manette GameCube sur `XInput/0/Gamepad`
(dans le dossier utilisateur du profil) ; RetroArch → ses profils officiels `autoconfig` (ajoutés s'ils manquent)
et L3 + R3 pour son menu ; PPSSPP et DOSBox Staging reconnaissent déjà les manettes. Règle : une manette déjà
réglée par la personne n'est jamais touchée, sauf « 🎮 Manette par défaut » (Options ▸ Émulateurs), sur accord.
Les **profils par jeu** viendront avec Taodbox (lot 5), quand on jouera vraiment à la manette.

**4c — reste à faire (Seb, 30/09)** :
- **Wiimote (Dolphin, Wii)** : Seb réglera lui-même un ou deux profils à mettre par défaut : Wiimote **à la
  verticale** ou **à l'horizontale**, **avec ou sans Nunchuk**. Frogtend doit pouvoir les reprendre (réglages de
  référence, voir « Décisions ») et choisir le bon selon le jeu.
- **Consoles tactiles** (DS, 3DS…) : il faut penser l'écran tactile. Au bureau, la souris joue le stylet. À la
  manette et à la télé, il faut une solution (un stick qui déplace un pointeur, la disposition des deux écrans…).
  Pistes à relever dans la documentation officielle des émulateurs concernés avant de choisir.
- **Clavier et souris en jeu, au choix par jeu** (Seb, 30/09) : la manette n'est pas imposée. Pour certains jeux, on
  doit pouvoir choisir de jouer au **clavier et à la souris** (jeux PC et DOS, jeux à pointer, stylet de la DS à la
  souris, pointeur de la Wiimote…). Réglage « Commandes » par jeu : automatique, manette, ou clavier et souris. Les
  touches du clavier déjà réglées dans les émulateurs sont toujours gardées (fait en 0.8.0).
- **Les réglages de référence** : un moyen simple pour Seb de dire « ce réglage devient celui par défaut » ;
  Frogtend le range (avec le numéro de version de l'émulateur) et l'applique ensuite partout, profil par profil,
  sans écraser ce que chacun a réglé lui-même.

**4c : ✅ livré en 0.9.0** (sauf les profils Wiimote définitifs, que Seb réglera) :
- **Réglages de référence** : on s'appuie sur les profils propres aux émulateurs (relevé dans les sources : Dolphin
  `Config\Profiles\<Wiimote|GCPad>\*.ini` section `[Profile]` ; DuckStation et PCSX2 `inputprofiles\*.ini`). Seb
  règle et enregistre un profil dans l'émulateur, puis **Options ▸ Émulateurs ▸ 📌 Réglages de référence** le reprend
  (rangé dans `<données>\references\`). Les références sont déposées dans les profils de chacun et proposées jeu par
  jeu. Pour qu'une référence reprise sur Venkman parte avec Frogtend sur les autres PC, l'agent l'intègre au dépôt
  (`src-tauri/references/`) dans la version suivante. *Le numéro de version de l'émulateur n'est pas encore retenu.*
- **Wii** : deux références de départ, sur manette Xbox, à remplacer par celles de Seb (même nom) : « Wiimote +
  Nunchuk » (d'office pour la Wii) et « Wiimote horizontale ». La référence d'office ne défait jamais une Wiimote
  retouchée à la main (Frogtend retient une empreinte de ce qu'il a écrit).
- **Commandes par jeu** (⚙ Gérer le jeu ▸ 🎮 Commandes) : Automatique, Clavier et souris (aucune manette réglée par
  Frogtend), ou une référence. Rangé dans les réglages du profil (`commandes`), donc dans sa sauvegarde.
- **DS** : relevé dans les sources du cœur RetroArch **melonDS DS** : `melonds_touch_mode` vaut `auto` par défaut
  (le stylet suit la souris OU un stick, selon ce qu'on a touché en dernier). Rien à régler : au bureau la souris, à
  la télé le stick. **3DS** : à étudier plus tard (émulateur à choisir avec Seb).

**4d — les consoles récentes (Seb, 30/09 : « tu ne me parles jamais de Xbox, Xbox 360, Switch, PS3… »)**. Frogtend
sait aujourd'hui INSTALLER six émulateurs (RetroArch, DOSBox Staging, DuckStation, PCSX2, Dolphin, PPSSPP) ; pour les
autres, il demande déjà où est le programme (installé à la main). Relevé le 30/09 sur le vrai Firehouse
(`/emulateurs`, base LaunchBox), l'émulateur recommandé :

| Console | Recommandé par Firehouse | Ce qu'il faut en plus (jamais téléchargé par Frogtend) |
|---|---|---|
| Xbox | **xemu** (autre : Cxbx-Reloaded) | fichiers de la console (BIOS, EEPROM, disque dur) — tirés de sa propre Xbox |
| Xbox 360 | **Xenia** | rien |
| PS3 | **RPCS3** | le micrologiciel PS3, téléchargeable sur le site officiel de Sony |
| Wii U | **Cemu** | les clés de la console (tirées de sa propre Wii U) |
| 3DS | **Azahar** | selon les jeux, fichiers système de sa propre console |
| PS Vita | **Vita3K** | le micrologiciel, site officiel de Sony |
| Switch | **aucun** (Firehouse n'en propose pas) | les clés de sa propre Switch ; les émulateurs principaux ont été arrêtés après des actions de Nintendo — **décision de Seb** avant tout travail |

À faire (après le menu en jeu, ordre à confirmer par Seb) : les ajouter au catalogue d'installation (source
officielle, mode portable, parties par profil, manette standard), en lisant la documentation de chacun d'abord.
Collection actuelle de Seb (30/09) : MS-DOS et Nintendo 64 seulement ; elle guidera les priorités.

**Switch — Seb est d'accord (30/09)** et donne les candidats actuels, à départager en lisant leurs sources officielles
(dépôts, dernières versions, mode portable, clés) avant de choisir :
- **Eden** : issu de l'écosystème de yuzu, réputé le plus stable et le plus compatible (PC et Android) ;
- **Ryubing / Kenji-NX** : les forks qui reprennent Ryujinx (réputé pour la précision de son émulation) ;
- **Citron** : autre fork de yuzu, avec gestionnaire de mods et de sauvegardes ; ses versions changent vite.

Frogtend ne fournit jamais les clés ni le micrologiciel : ils viennent de la Switch de la personne.

### Lot 4 ter — Le menu universel en jeu (OSD) (demandé par Seb le 30/09)
**Une combinaison de touches universelle** (à la manette, et au clavier), la même dans **tous les jeux et tous les
émulateurs**, en **Frogtend comme en Taodbox**, ouvre un menu simple de Frogtend par-dessus le jeu. On n'a plus
besoin des menus des émulateurs. Actions voulues :
- **reprendre** / **réinitialiser** (reset) / **arrêter** le jeu (retour à la ludothèque, parties mises à l'abri) ;
- **lire les livrets et informations** du jeu : manuel, solution, astuces, fiche — déjà gardés en local (lot 2) ;
- **activer ou couper des codes de triche** (lien avec le lot 8) ;
- **changer de CD / de disque** (jeux sur plusieurs disques) ;
- plus tard, selon les besoins : sauvegarde et chargement rapides, choix du profil de manette (Wiimote verticale ou
  horizontale…), etc.

Pistes techniques, **à vérifier dans la documentation officielle avant de coder** :
- Frogtend lit lui-même la manette en fond pendant la partie (pour reconnaître la combinaison quel que soit
  l'émulateur) et affiche une fenêtre par-dessus le jeu. Il faudra sans doute un plein écran « fenêtré sans
  bordure » dans les émulateurs, pour qu'une fenêtre puisse passer au-dessus.
- Pour agir dans l'émulateur : RetroArch accepte des commandes par le réseau local (réinitialiser, quitter,
  changer de disque, triches…) ; pour les autres, les raccourcis que Frogtend leur aura réglés ; sinon, fermer
  proprement le processus.
- La combinaison doit éviter les conflits avec celles déjà posées en 0.8.0 (Select + Start, L3 + R3) : elle les
  remplacera sans doute.

Ordre proposé : **avant Taodbox** (lot 5), qui s'en sert. À valider avec Seb.

**Recherche faite le 30/09** : voir [RECHERCHE-MENU-EN-JEU.md](RECHERCHE-MENU-EN-JEU.md) (possible / impossible,
émulateur par émulateur, avec les preuves). Point bloquant à mesurer en premier : **Frogtend lit-il la manette quand
le jeu a le focus ?** (XInput, et SDL pour les manettes PlayStation/Switch).

**Décisions de Seb (30/09)** : combinaison **Select + Start tenus 1 s** à la manette, touche **Pause/Attn** au
clavier (réglables dans les Options) ; SDL3 embarqué (compilé dans le programme, aucune DLL).

**⏳ En attente de Seb : l'essai de la sonde avec les vraies manettes** (il ne peut pas tout de suite).
`outils/sonde-manette` (construite le 30/09) : la lancer, appuyer sur chaque manette, puis dans un jeu au premier
plan, et laisser finir les 3 minutes ; le journal `sonde-manette.txt` dit quelles manettes restent lisibles en
arrière-plan. En attendant, on avance sur tout ce qui n'en dépend pas : déclenchement au CLAVIER, fenêtre du menu
par-dessus le jeu, réglages de pilotage des émulateurs (pause à la perte du focus, plein écran sans bordure,
raccourcis F13–F24, commandes réseau de RetroArch).

### Lot 5 — Taodbox
Démarrage avec `--taodbox`, plein écran, entièrement à la manette. Déclaration comme application Sunshine/Apollo.
La sortie est propre : le jeu est fermé, le bureau rendu et la fin de session signalée. Premier essai mesuré avec
Seb.

### Lot 6 — Demander un jeu absent
Recherche dans la base LaunchBox (`/recherche`) et envoi de la demande (`/demandes`). La réponse est neutre si la
demande est refusée.

### Lot 7 — L'assistant jeux
Une conversation avec l'assistant de Firehouse. Il **propose** des actions locales, que l'utilisateur voit et
accepte avant qu'elles s'exécutent ; chaque action est journalisée et, si possible, réversible.

### Lot 8 — Mods et outils
Cheat Engine et les tables FearLess, Nexus Mods (par son API), les trainers, les autres sources du brief § 1.5.

### Lot 9 — Jeux gratuits des boutiques
Epic, Amazon Prime Gaming, Xbox Game Pass, PlayStation Plus. Ils restent dans le Frogtend de l'utilisateur et ne
remontent pas dans Firehouse.

## Personnalisation (« beaucoup d'éléments modifiables »)

Les couleurs appartiennent aux skins : on ne crée jamais de couleur hors de ceux de Firehouse. Tout le reste se règle
dans l'interface, par profil, avec un bouton « Revenir au réglage d'origine » :

- **Apparence** : le skin (parmi ceux de Firehouse, avec l'option de l'enregistrer dans Firehouse), la taille du
  texte (échelle globale), la densité (compacte ou aérée), les animations (activées ou réduites), le fond vidéo du
  skin `firehouse` (oui ou non).
- **Ludothèque** : l'affichage (grille de jaquettes ou liste), la taille des jaquettes, les informations montrées sur
  une carte (titre, année, plateforme, genres…), le tri par défaut, les filtres mémorisés, l'ordre et la visibilité
  des plateformes dans la barre latérale.
- **Fiche jeu** : les blocs affichés et leur ordre (résumé, versions, annexes, informations…).
- **Commandes** : les raccourcis clavier et les boutons de la manette, réaffectables.
- **Taodbox** : l'échelle (×2 proposé en 1080p), la marge de sécurité de la télé, l'écran d'accueil.
- **Chemins et réseau** : l'adresse de Firehouse, les dossiers de jeux (voir ci-dessous), les plafonds de
  téléchargement.

### Les emplacements de jeux, par système (Seb, 29/09)

Une collection peut être énorme et répartie sur plusieurs disques :
- chaque système (SNES, PC, MS-DOS…) a **un ou plusieurs emplacements**, dans l'ordre de préférence ;
- un emplacement **par défaut** sert aux systèmes qui n'en ont pas ;
- à l'installation, Frogtend **propose** le premier emplacement du système qui a assez de place. La taille nécessaire
  et la place libre sont affichées, on peut en choisir un autre, et rien ne s'écrit sans accord ;
- chaque jeu installé retient son emplacement. Un disque débranché ou un partage réseau absent est **signalé** (« Disque
  introuvable : E:\Jeux »), jamais interprété comme une désinstallation ;
- ajouter un emplacement ne lance pas de parcours du disque. Un parcours à la recherche de jeux déjà présents serait
  une opération de masse : il est annoncé et chiffré avant d'être lancé.

Les emplacements sont **propres au PC**, pas au profil, puisque les disques appartiennent à la machine.

## Questions ouvertes pour Seb

- (aucune pour l'instant ; la liste « Personnalisation » est validée le 29/09)
