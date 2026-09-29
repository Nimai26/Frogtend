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

Questions posées à Seb : voir la conversation du 29/09.

### Lot 2 — Télécharger
Une file de téléchargements avec reprise (`Range`) et un contrôle des octets reçus. Pour chaque version, la place
nécessaire est annoncée avant de commencer. La destination suit les emplacements du système (voir « Les
emplacements de jeux »). Les plafonds et le débit maximal se règlent.
Rien ne démarre sans accord.

### Lot 3 — Installer et lancer
L'installation suit la **qualité** de la version (prêt à jouer, repack, installeur d'origine, ROM, image disque),
sans jamais renommer une ROM ni une image. Le lancement se fait en natif ou par émulateur. Les sauvegardes de
parties sont localisées et sauvegardées avant toute réinstallation. Le début et la fin de chaque session sont
annoncés à Firehouse (`/session`).

### Lot 4 — Les émulateurs
Détection, chemins, BIOS, profils de manettes (XInput d'abord), profils par jeu, et une copie de sauvegarde de toute
configuration modifiée. Il restera à décider si Frogtend installe les émulateurs lui-même (question ouverte).

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
