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
- **Souplesse des émulateurs** (Seb, 30/09) : jamais un seul émulateur ou cœur imposé. Chaque système peut en avoir
  **plusieurs** (dont plusieurs cœurs RetroArch), avec **un par défaut pour le système**, **un par défaut par jeu**
  (différent de celui de la console), et un **choix au lancement** (« ▶ Jouer avec… »). Réglable facilement depuis
  l'interface du système (⚙ Options ▸ Émulateurs) et du jeu (⚙ Gérer le jeu ▸ 🕹 Émulateur). ✅ fait en 0.10.0.
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

**✅ 0.13.0 (01/10)** : Frogtend sait installer **Xenia Canary, xemu, RPCS3, Cemu, Azahar et Vita3K** (relevé dans
leurs sources : paquet officiel GitHub, mode portable, ligne de commande plein écran). Parties **par profil** :
Xenia (`--content_root`), xemu (un fichier de réglages et un disque dur virtuel par profil, `-config_path`), RPCS3
(un compte RPCS3 par profil, `--user-id`), Azahar (NAND et carte SD du profil). **Limites** : Cemu et Vita3K gardent
des parties communes au PC (leurs comptes demandent d'écrire des fichiers internes : à faire après un essai réel) ;
pas encore de pause automatique ni de raccourcis pour le menu en jeu sur ces six ; manettes non réglées par
Frogtend (leurs défauts). Bouton **💿 Micrologiciel PS3…** (RPCS3 `--installfw`, fichier choisi par la personne).
**Seb (02/10)** : les sites de **Ryubing** et **Citron** fonctionnent → à proposer aussi (plusieurs émulateurs par
console). **Référence émulateurs** : [emu-france.com](https://www.emu-france.com/emulateurs/) (toutes les consoles,
souvent à jour) ; demandé à l'agent Firehouse de fournir, par plateforme, les sites officiels et sources de
téléchargement des émulateurs (besoin n° 14).

**Switch ✅ 0.15.0 : Eden.** Relevé le 01/10 : seul des trois candidats dont le dépôt officiel répond d'ici
(git.eden-emu.dev, Forgejo, v0.2.1 du 01/06/2026, paquet `Eden-Windows-…-amd64-msvc-standard.zip` hébergé sur
stable.eden-emu.dev) ; mode portable par un dossier `user` à côté du programme ; `-f -g <jeu>` ; **`-u <nom>`**
choisit l'utilisateur Switch par son nom → un utilisateur Eden par profil Frogtend (à créer une fois dans Eden, du
même nom), donc des parties par profil. Ryubing/Kenji-NX (git.ryujinx.app) et Citron (git.citron-emu.org) : leurs
serveurs n'ont pas répondu le 01/10 ; à réexaminer si Eden déçoit. Firehouse ne recommandant rien pour la Switch,
Frogtend propose Eden d'office.
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

**Avancement** : ✅ réglages de pilotage des émulateurs (0.10.0) ; ✅ **menu au clavier** (0.11.0) : touche
Pause/Attn (réglable : Arrêt défil, Ctrl+Maj+M), armée pendant la partie seulement ; fenêtre « menu-jeu » toujours
au-dessus : reprendre, sauvegarde et chargement rapides, disque suivant, manuel et documents, recommencer, quitter
(confirmés). À vérifier en vrai avec Seb (aucun jeu lancé par l'agent). Reste : la **manette** (après la sonde),
les triches (lot 8), et le choix de la manette (Wiimote…) depuis le menu.

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

**✅ Première version en 0.16.0** (à essayer sur la télé avec Seb) :
- **🛋 Taodbox** dans la barre du haut, ou **`Frogtend.exe --taodbox`** (pour Sunshine/Apollo) : plein écran,
  barre de titre cachée, **échelle ×2** (charte § 7), marge de 5 % pour la télé.
- **La manette pilote TOUS les écrans** (choix du joueur et pavé du PIN, fenêtres de confirmation, Taodbox) : croix
  ou stick gauche = déplacement selon la géométrie (jamais piégé), A = valider, B = Échap. Lu par l'API Gamepad de
  WebView2 (Taodbox est au premier plan) ; les flèches du clavier font pareil.
- Les jeux de CE PC en grandes jaquettes, les plus récemment joués d'abord, filtre par console ; A lance (installe
  d'abord si besoin) ; à la fin du jeu, Taodbox revient au premier plan tout seul.
- Lancé par `--taodbox` : bouton **⏻ Quitter** (ferme Frogtend, donc la session Sunshine se termine) ; sinon
  **🖥 Bureau**. **👥 Changer de joueur**.
- **Pas encore** : la combinaison manette du menu en jeu (attend la sonde), la déclaration automatique dans Sunshine
  (Frogtend n'écrit pas dans sa configuration : à faire par Seb, mode d'emploi dans A-TESTER), la mesure de latence.

### Lot 6 — Demander un jeu absent
Recherche dans la base LaunchBox (`/recherche`) et envoi de la demande (`/demandes`). La réponse est neutre si la
demande est refusée.

**✅ Livré en 0.12.0** : menu « Demander un jeu » ; recherche dans toute la base LaunchBox (forme de `/recherche` relevée
sur le vrai Firehouse le 30/09 : `{ok, resultats: [{launchbox_id, titre, titre_fr, plateforme, annee, genres,
developpeur, jaquette, deja}]}`) ; un jeu déjà possédé ou déjà cherché ne se redemande pas ; demande confirmée,
puis `POST /demandes {launchbox_id}`. Les jaquettes LaunchBox (site extérieur) ne s'affichent pas : la politique de
sécurité de l'application n'autorise que les images de Frogtend. **Pas encore essayé en vrai** (une vraie demande
crée une entrée chez Firehouse : à faire par Seb).

### Lot 7 — L'assistant jeux
Une conversation avec l'assistant de Firehouse. Il **propose** des actions locales, que l'utilisateur voit et
accepte avant qu'elles s'exécutent ; chaque action est journalisée et, si possible, réversible.

**✅ Livré en 0.14.0** : menu « Assistant » (conversation générale) et « 💬 Demander à l'assistant » sur la fiche d'un jeu
(conversation de ce jeu). Essai réel le 01/10 : « Comment je lance ce jeu ? » sur Dune → réponse en 5 s, Markdown,
action `lancer` avec `{emulateur, fichier, media_id}`. Conversations gardées en mémoire seulement (jamais sur le
disque), oubliées à la fermeture du profil ; 40 derniers messages envoyés, 4 000 caractères chacun. Actions
**exécutées après un oui** : lancer (installer d'abord si besoin), installer, voir la fiche pour l'ajouter, demander
un jeu, ouvrir une page (http/https seulement). **Refusées pour l'instant** : configurer, cheat, mod (lot 8). Chaque
décision (acceptée/refusée) est notée au journal. Réponse rendue par un Markdown simple et sûr (aucun HTML exécuté).

### Lot 8 — Mods et outils
Cheat Engine et les tables FearLess, Nexus Mods (par son API), les trainers, les autres sources du brief § 1.5.

**Décisions de Seb (02/10)** :
- **Pas d'API Nexus Mods** (trop chère).
- **Cheat Engine : Frogtend peut l'installer**, mais **une version propre** (sans les logiciels en plus que propose
  son installeur officiel) : trouver la source ou la méthode (installation silencieuse qui refuse les offres, paquet
  sans offres…) en lisant la doc et les sources officielles.
- **Frogtend ne télécharge PAS lui-même** les bases de triche (libretro, autres émulateurs) ni les listes de mods :
  ces informations seront **fournies par Firehouse, jeu par jeu** (besoin n° 13 de BESOINS-API.md).
- **Copie de sauvegarde du jeu avant un mod : proposée** (pas obligatoire), avec un **avertissement** si on la refuse.

**Cheat Engine, relevé le 02/10** : le dépôt GitHub officiel ne publie que les sources (aucun paquet) ; le site
officiel dit lui-même que **l'installeur sans logiciels en plus est réservé aux membres de son Patreon**. Pistes (à
choisir avec Seb) : (a) Seb récupère l'installeur propre (Patreon) et le confie à Firehouse, qui le sert à Frogtend ;
(b) installation silencieuse de l'installeur public en refusant les offres : impossible à garantir sans essai.

**✅ Décision de Seb (02/10, finale)** : les mises à jour de Cheat Engine sont rares → **Seb l'installe lui-même sans
les logiciels en plus, en fait un zip et le confie à Firehouse**, qui le **distribue** à tous (contrat 14, \`type:
direct\`, \`sha256\`). Firehouse **prévient Seb** quand une nouvelle version sort ; Seb met à jour le zip sur Firehouse,
qui le transmet à tous les Frogtend. (Ce qui suit est la réflexion qui a mené là.)

**Relevé dans les sources de Cheat Engine (02/10, \`Cheat Engine/ceregistry.pas\`)** : ses réglages vont dans le
**registre** (\`HKCU\Software\Cheat Engine\`), sans option portable. La copie zippée marche, mais laisse des réglages
dans le registre (« effet pieuvre »). Piste, **à annoncer à Seb avant tout (registre)** : Frogtend exporterait cette
clé par profil après usage et la remettrait avant (\`reg export\` / \`reg import\`), ce qui donnerait aussi des réglages
Cheat Engine par profil. **✅ 0.18.0–0.19.0** : réglages par profil (registre), et **🧰 Brancher Cheat Engine sur le
jeu** (fiche du jeu et menu en jeu) par un script `autorun\frogtend.lua` posé par Frogtend (fonctions relevées dans
celua.txt : getCheatEngineDir, fileExists, openProcess, loadTable, createTimer). Extensions du forum
(*Set Memory Region for Emulator*, *Dosbox base finder*, taille du texte) : à ajouter au zip par l'agent Firehouse,
à la demande de Seb (02/10). **✅ Accord de Seb (02/10) pour le registre** : Frogtend sauvegarde d'abord la clé existante
(jamais rien effacé sans copie vérifiée), puis la remet par profil avant de lancer Cheat Engine et la range à sa
fermeture.

**Proposition de Seb (02/10)** : Firehouse fournit à Frogtend un Cheat Engine **portable et propre**, de deux façons
possibles : (1) chaque semaine, vérifier la version ; si nouvelle, l'**installer dans un bac à sable**, puis zipper
SEULEMENT son dossier d'installation ; (2) le **construire depuis le dépôt git** officiel. (Pas de version portable
officielle gratuite : copier le dossier installé en donne une.) Côté Frogtend : il le reçoit comme un émulateur
décrit (contrat 14 : \`type: direct\`, \`programme\`, \`sha256\`), sans rien de plus à coder. Avis de l'agent : voir la
réponse du 02/10 (bac à sable d'abord ; points à vérifier : réglages dans le registre, pilote noyau).

**✅ 0.17.0 (partie 1, contre le contrat 13/14, routes Firehouse à venir)** :
- écran **🎯 Triches et mods** sur la fiche d'un jeu : codes de l'émulateur du jeu (correspondance incertaine
  signalée), posés sur demande dans le dossier du **profil** (`nom_fichier`/`dossier`/`base` de Firehouse, chemin
  vérifié, fichier différent gardé à côté) ; tables Cheat Engine et mods ouverts dans le navigateur ; **copie du jeu
  proposée avant un mod** (taille annoncée), avertissement si refusée ;
- **émulateurs décrits par Firehouse** (forks compris : Ryubing, Citron…) installables : Firehouse résout le paquet,
  Frogtend vérifie l'empreinte, crée le marqueur portable, trouve le programme (ou le fait choisir).

### Lot 9 — Jeux gratuits des boutiques

**Décision de Seb (02/10)** : les jeux possédés des boutiques (installés ou non) vont **dans la ludothèque principale,
avec les autres** (probablement sous la plateforme PC), **filtrables** par boutique et par « installé ou non ». Pas
d'écran à part au final (« 🛒 Boutiques » sert à régler et importer).
**✅ 0.23.0** : les jeux de Steam et de GOG Galaxy entrent dans la ludothèque (cache local du profil) sous la plateforme
**Windows**, avec un **id négatif** stable (`boutique:clé`, le même jeu vu par Steam et par Galaxy n'apparaît qu'une
fois) ; le catalogue de Firehouse ne les montre jamais ; une synchronisation de Firehouse ne les retire jamais.
Filtres de « Ma ludothèque » : **Boutique** (Firehouse, Steam, GOG, Epic, Xbox, Ubisoft, EA) et **Installés / Pas
installés** (jeux de Firehouse : installés sur ce PC). Panneau : ▶ Jouer / ⬇ Installer par Steam ou GOG Galaxy ;
double-clic pareil ; jaquettes de boutique par le protocole des jaquettes.

**Récupération automatique des jeux offerts — recherche GitHub (02/10, demande de Seb)** : la référence est
**vogler/free-games-claimer** (4 238 ★, actif au 01/10/2026, AGPL-3.0, JavaScript + Playwright) : Epic, Prime Gaming,
GOG. Méthode : un navigateur automatisé connecté UNE fois à chaque compte (session gardée dans son profil), qui passe
sur les pages des jeux offerts et clique « Obtenir ». Limites qu'il dit lui-même : captcha d'Epic (problème #183
ouvert), double authentification à saisir. Variante Python : P-Adamiec/Free-Games-Claimer-Remaster (374 ★).
**Licence AGPL : s'inspirer de la méthode, ne pas copier son code** (Frogtend est en Apache 2.0). Risque : les conditions
d'utilisation des boutiques (automatisation) — à dire à Seb. Piste Frogtend : une fenêtre WebView2 où la personne se
connecte elle-même (pages officielles, Frogtend ne voit pas le mot de passe), puis des clics automatisés ; si un
captcha apparaît, la fenêtre se montre. La liste des jeux offerts d'Epic est publique (sans compte).
**Décision de Seb (02/10)** : on essaie **B (récupération automatique)** ; en cas d'échec, **A** (Frogtend montre les jeux
offerts de la semaine et ouvre la page officielle ; la personne clique « Obtenir »).
**✅ 0.24.0 (Epic)** : liste publique (`freeGamesPromotions`, vérifiée le 02/10 : System Shock 2 25th Anniversary
Remaster et BURIED STARS jusqu'au 08/10) ; « 🔑 Se connecter à Epic » (page officielle, navigateur PROPRE AU PROFIL dans
`profils\<id>\navigateur`) ; « 🎁 Obtenir » ouvre la page `/en-US/p/<slug>` CACHÉE, le script `ressources/gratuits/
epic.js` clique Get → licence → Place Order → I Agree et rend le résultat par le titre ; connexion requise, captcha ou
échec → la fenêtre s'affiche (repli A) ; case « les obtenir tout seul » (par profil, 1 fois par jour, désactivée par
défaut). **Pas encore essayé en vrai** (compte Epic de Seb). Reste : Prime Gaming, GOG (mêmes principes).

**PS Plus (Seb, 02/10)** : ajouter aux récupérations automatiques les **jeux mensuels réservés aux membres PS Plus**
(Seb est abonné Premium et oublie souvent de les ajouter). **Optionnel** pour chaque profil (case à cocher, avec le
compte PlayStation de la personne) ; ces jeux **ne vont PAS dans la ludothèque** de Frogtend (ce sont des jeux de
console) : on les ajoute seulement à la bibliothèque du compte PlayStation. À étudier (page officielle des jeux du
mois, PlayStation Store, connexion au compte PSN) avant de coder ; même principe que pour Epic (automatique, sinon la
page s'affiche).
**Recherche (02/10)** : un seul projet GitHub (bnowakow/ps-plus-claimer, essai inachevé, Selenium sur la page du
Store). Ce qui est vérifié sur les pages publiques : la page produit d'un jeu du mois a un identifiant dédié (ex.
`UP9000-PPSA30630_00-MLBTHESHOW26PLUS`), une offre `PS_PLUS_FREE` et un bouton d'action principal
`data-qa="mfeCtaMain#cta#action"` (hors connexion : « UPSELL_PS_PLUS_FREE », s'abonner). La LISTE des jeux du mois
n'est pas publique en clair (pages du Store construites dans le navigateur ; la page playstation.com/fr-fr/ps-plus/
whats-new/ ne donne qu'une partie des liens). Donc : connexion Sony par profil (page officielle, navigateur propre au
profil), fenêtre cachée sur le Store connecté qui lit les jeux du mois et clique « Ajouter à la bibliothèque » ;
repli : la page s'affiche. Les sélecteurs se règlent AVEC Seb connecté (un compte membre est indispensable).
**✅ 0.26.0 (à essayer avec Seb)** : 🛒 Boutiques ▸ 🎁 Jeux offerts ▸ 🎮 PlayStation Plus : « 🔑 Se connecter à
PlayStation » (Store officiel, navigateur du profil), « 🎮 Ajouter les jeux du mois » (fenêtre cachée sur la catégorie
PS Plus, 30 jeux au plus, script `ressources/gratuits/psplus.js` : UN SEUL clic possible, sur « Ajouter à la
bibliothèque », vérifié par un test), case « J'ai PS Plus » par profil (désactivée par défaut, une fois par semaine).
Rien dans la ludothèque. Incertain tant que pas essayé : que la catégorie relevée soit bien celle des jeux du mois (elle
peut contenir aussi des jeux du catalogue Extra/Premium : les ajouter est sans risque) et les libellés exacts des
boutons pour un membre connecté.

**Menu « Importer » comme LaunchBox (Seb, 02/10, capture)** : la MÊME liste : Fichiers ROM (sous-menu), Jeux MS-DOS,
MAME Arcade Full Set, Amazon Games, EA, Jeux Epic Games, Jeux GOG, Jeux Steam, Uplay/Ubisoft Connect, Jeux Windows,
Jeux Xbox/Microsoft Store, Ajouter un jeu manuellement, Installer un jeu DOS. **Le regroupement par GOG Galaxy** (une
boutique reliée dans Galaxy arrive par Galaxy) doit être **expliqué** : dans l'aide, dans la fenêtre d'import de chaque
boutique, et dans les Options (partie « lier les comptes »).
**✅ 0.25.0** : bouton « 📥 Importer ▾ » dans la barre (les 13 entrées de LaunchBox, même ordre) ; page `/importer`
(une fenêtre par source : Steam par son compte OU Galaxy ; Amazon, EA, Epic, GOG, Ubisoft, Xbox par Galaxy, avec le
nombre de jeux de CETTE boutique lus dans Galaxy et « compte sans doute pas relié » s'il n'y en a aucun) ; page
« ❓ Aide » ; ⚙ Options ▸ Comptes ▸ GOG Galaxy (lier les comptes : Galaxy ▸ Paramètres ▸ Intégrations). Une seule
explication (`src/lib/import/sources.ts`) reprise aux trois endroits. **Reste (prochain lot, « jeux locaux »)** :
Fichiers ROM, Jeux MS-DOS, MAME Arcade Full Set, Jeux Windows, Ajouter un jeu manuellement, Installer un jeu DOS —
marqués « bientôt » dans le menu.

**✅ Accord de Seb (02/10)** : Frogtend peut LIRE (lecture seule) les fichiers des lanceurs installés (GOG Galaxy, Epic,
EA app). Ses installations sont personnalisées (GOG Galaxy dans `D:\LaunchBox\Games\GOG Galaxy`) : chercher les
lanceurs par le registre, pas les dossiers par défaut.

**✅ GOG Galaxy en 0.22.0** : relevé dans sa base (02/10) — Galaxy 2.0 regroupe GOG ET les boutiques reliées : sur Venkman
324 jeux hors DLC (GOG 197, Steam 70, Epic 43, Xbox 6, Ubisoft 4, EA 4), 323 avec jaquette (`verticalCover` sur
images.gog.com). Lu sur une COPIE de `C:\ProgramData\GOG.com\Galaxy\storage\galaxy-2.0.db` (+ wal/shm), copie effacée
après lecture ; DLC et jeux cachés écartés ; installés : `InstalledBaseProducts` (GOG) et `InstalledExternalProducts`.
Jouer / installer : `goggalaxy://openGameView/<releaseKey>`. **Epic, Xbox, Ubisoft et EA viennent donc de Galaxy, sans
connexion dans Frogtend** (y compris les jeux non installés). La connexion directe à Epic (comme LaunchBox, peu fiable
selon Seb) n'est plus nécessaire tant que Galaxy est relié.
Epic, Amazon Prime Gaming, Xbox Game Pass, PlayStation Plus. Ils restent dans le Frogtend de l'utilisateur et ne
remontent pas dans Firehouse.

**Décision de Seb (02/10)** : les jeux gratuits vont **avec la détection des jeux déjà possédés** sur les comptes de
la personne : **Amazon, Epic, GOG, Steam, EA Play** (et les autres boutiques du brief). Chacun devra entrer les clés
d'API ou secrets de ses comptes (dans le coffre de Windows, jamais affichés ni journalisés). **S'inspirer des
méthodes de LaunchBox** pour l'import de ces boutiques (à étudier dans sa documentation avant de coder).

**Seb (02/10), captures de LaunchBox à l'appui** :
- LaunchBox : **Outils ▸ Importer ▸** Amazon Games, EA, Epic Games, GOG, Steam, Uplay/Ubisoft Connect, Jeux Windows,
  Xbox/Microsoft Store. **Options ▸ Intégrations ▸** GOG, Steam… Pour Steam : l'**URL personnalisée** du profil
  (\`steamcommunity.com/id/<nom>\`) et une **clé d'API personnelle** (Steam révoque celles de LaunchBox : chacun crée
  la sienne sur \`steamcommunity.com/dev/apikey\`).
- **Règle de Seb** : les comptes se règlent **dans les Options**, OU **au moment d'un import** si rien n'est encore
  réglé (Frogtend ouvre alors le réglage avant d'importer).
- **Ordre retenu** : Steam d'abord (API officielle : ResolveVanityURL, GetOwnedGames), puis les autres après étude.
- ⚠ La clé Steam de Seb est apparue en clair dans une capture (02/10) : à régénérer ; jamais recopiée par l'agent.
- **Epic dans LaunchBox (captures de Seb, 02/10)** : « Assistant d'importation de jeux Epic » ; s'il n'est pas connecté,
  il ouvre une **fenêtre avec la page de connexion OFFICIELLE d'Epic** (e-mail, ou Google, Steam, PlayStation, Xbox,
  Nintendo…), puis analyse la bibliothèque. **Seb : « ça ne fonctionne pas très bien »**. En revanche LaunchBox
  **propose d'installer les jeux Steam (et normalement Epic) depuis son interface** → à reproduire (Steam : fait en
  0.20.0 par \`steam://install\`).

**✅ Steam en 0.20.0** : ⚙ Options ▸ **Comptes ▸ Steam** (compte = URL personnalisée ou identifiant à 17 chiffres ;
clé d'API personnelle, **vérifiée auprès de Steam avant d'être enregistrée**, rangée dans le coffre de Windows sous
`jeton:<profil>#steam`, jamais réaffichée ; erreurs rendues sans l'adresse qui contient la clé). **🛒 Boutiques** :
import (réglage demandé d'abord s'il manque), liste filtrable, installés sur CE PC (`libraryfolders.vdf` +
`appmanifest_<id>.acf`, registre lu seulement), ▶ Jouer / ⬇ Installer par `steam://`. Profil Steam privé : Frogtend dit
quoi changer. Supprimer un profil retire aussi sa clé Steam. **0.21.0** : jaquettes officielles de Steam (protocole `boutique://`, cache sur ce PC), grille, et section Steam dans
Taodbox. **Reste** : mêler ces jeux à la ludothèque principale, puis Epic, GOG, Amazon, EA, Ubisoft, Xbox (étude de LaunchBox d'abord), et les jeux gratuits.

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

- (la liste « Personnalisation » est validée le 29/09)
- ✅ Répondu le 02/10 (voir les lots 8 et 9). *Questions d'origine :*
- **Lot 8 (mods, triches, outils), avant de commencer (01/10)** :
  1. Nexus Mods : as-tu une **clé d'API** Nexus (compte Nexus ▸ API) ? Elle irait dans le coffre de Windows.
  2. **Cheat Engine** : Frogtend peut-il l'installer (programme tiers, avec installeur) dans le dossier des
     émulateurs, sur ton accord à chaque fois ? (Attention : son installeur officiel propose des logiciels en plus.)
  3. **Triches des consoles** : télécharger la base officielle de libretro (codes pour RetroArch, sur accord, une
     fois) et proposer les codes dans le menu en jeu ?
  4. **Mods qui modifient le jeu** : toujours une copie de sauvegarde du jeu avant ? (proposé : oui, des fichiers
     touchés seulement).
- **Lot 9 (jeux gratuits des boutiques)** : il touche à tes **comptes** Epic, Amazon, Xbox, PlayStation (brief :
  à annoncer avant). Lesquels veux-tu en premier ? Accepte-tu que Frogtend t'ouvre la page de connexion officielle
  de chaque boutique (jamais de mot de passe saisi dans Frogtend) ?
