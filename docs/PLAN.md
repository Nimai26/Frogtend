# Frogtend — plan en lots

> Proposé le 29/09/2026, **à valider par Seb**. Un lot se termine (tests verts, version livrée, notes de version)
> avant que le suivant commence. Les idées en cours de route vont dans `docs/IDEES.md`.

## Décisions déjà prises (Seb, 29/09/2026)

- **Pile : Tauri 2.** Cœur en Rust (processus, fichiers, téléchargements, coffre Windows) et interface en TypeScript.
  Pour l'interface, je propose **Svelte 5 + Vite** : c'est léger, et pour Taodbox une navigation à la manette ne
  demande rien de spécial.
- **Frogtend n'est pas lié à Venkman.** Il s'installe sur n'importe quel PC Windows 10/11 par un **installateur**
  (NSIS, par utilisateur, sans droits administrateur ; WebView2 installé s'il manque). Il **se met à jour** lui-même
  par des mises à jour **signées**. Rien ne suppose une machine, un disque ou un réseau précis.
- **Adresse de Firehouse** réglable dans les options. Par défaut : `https://core.hikari-no-sekai.fr`.
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

### Lot 0 — Le socle
- Squelette Tauri 2 + Svelte 5 + TypeScript. Tests : Vitest (interface) et `cargo test` (cœur), lancés ensemble par
  une seule commande.
- `VERSION`, `docs/NOTES-DE-VERSION.md`, script de livraison (vérifie les tests, monte la version, construit
  l'installateur).
- **Installateur Windows et mises à jour dès le départ** : l'installateur NSIS et le module de mise à jour de Tauri,
  avec une clé de signature des mises à jour rangée hors du dépôt. Il est plus simple de le faire au début que d'y
  revenir.
- La base de l'interface : jetons de thème (clair/sombre, aucune couleur en dur), fenêtres de dialogue propres à
  l'application, déplacement du focus au clavier.

### Lot 1 — Voir la ludothèque (sur le contrat simulé)
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

### Lot 2 — Télécharger
Une file de téléchargements avec reprise (`Range`) et un contrôle des octets reçus. Pour chaque version, la place
nécessaire est annoncée avant de commencer. Le dossier de destination, les plafonds et le débit maximal se règlent.
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

## Questions ouvertes pour Seb

1. **Où publier les mises à jour ?** Il faut une adresse qui serve l'installateur et un petit fichier `latest.json`.
   Firehouse (`https://core.hikari-no-sekai.fr/...`), un dépôt GitHub privé avec ses « releases », autre chose ?
2. **Signature de code Windows.** Sans certificat, Windows SmartScreen affiche « éditeur inconnu » à la première
   installation. On l'accepte pour l'instant (usage familial) ou on achète un certificat ?
3. **Protéger les profils.** Un code PIN par profil ? Sans code, un enfant peut ouvrir le profil admin sur un PC
   partagé.
4. **Identifiant de l'application** (il ne pourra plus changer) : `fr.hikari-no-sekai.frogtend` ?
5. **Svelte 5** pour l'interface : d'accord ?
