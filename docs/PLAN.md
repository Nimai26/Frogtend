# Frogtend — plan en lots

> Proposé le 29/09/2026, **à valider par Seb**. Un lot se termine (tests verts, version livrée, notes de version)
> avant que le suivant commence. Les idées en cours de route vont dans `docs/IDEES.md`.

## Décisions déjà prises (Seb, 29/09/2026)

> ⚠ **Règle « pas de pieuvre » (Seb, 02/10, non négociable).** Frogtend ne s'éparpille JAMAIS sur le disque :
> - **les jeux** vont dans les dossiers de jeux définis dans les réglages (⚙ Options ▸ Emplacements), et nulle part
>   ailleurs — sauf les jeux PC des boutiques officielles, qui restent dans les dossiers de Steam, Epic, GOG… ;
> - **les émulateurs** vont dans le dossier des émulateurs défini dans les réglages ;
> - **tout ce qui sert à UN jeu installé reste avec lui** : ses médias (jaquette, fiche, manuel, documents) **copiés
>   en local** pour l'affichage hors ligne, ses dossiers mods, triches (cheats), etc. — dans son dossier
>   d'installation ;
> - **les outils et ajouts** (Cheat Engine et tout autre) s'installent dans un dossier défini dans les réglages, jamais
>   ailleurs ;
> - « C: » dans DOSBox = le lecteur VIRTUEL de DOSBox, monté sur le dossier du jeu ; jamais le C:\ de Windows.
> Avant d'écrire où que ce soit : est-ce dans le dossier du jeu, ou dans un dossier défini dans les réglages ? Sinon,
> on ne l'écrit pas.
>
> **Décisions de Seb (02/10) pour l'appliquer :**
> 1. Jeu Firehouse installé : tout ce que Frogtend ajoute va dans **`<dossier du jeu>\Frogtend\`** (`medias\`,
>    `mods\`, `triches\`), séparé des fichiers du jeu. Les médias de jeux déjà installés sont migrés (copie
>    vérifiée, puis l'ancienne copie de `%APPDATA%` retirée).
> 2. Import de ROM : **la personne choisit** — les **laisser où elles sont** (le dossier est alors **ajouté
>    automatiquement aux sources (emplacements) du système** dans les réglages) ou les **copier** dans l'emplacement
>    du système (taille et place annoncées ; jamais déplacées : rien ne s'efface). Leurs médias vont dans
>    `<source>\Frogtend\<titre>\`.
> 3. Les **abris de parties** (copie avant retrait d'un jeu) vont dans un **nouveau réglage « dossier des abris »**
>    (⚙ Options ▸ Emplacements) ; ceux déjà dans `%APPDATA%\Frogtend\sauvegardes` sont migrés (copie vérifiée).
> 4. **Cheat Engine et les outils** (trainers, gestionnaires de mods…) vont dans un **nouveau réglage « dossier des
>    outils »**, séparé du dossier des émulateurs.
> **✅ 0.30.0** : médias des jeux du PC dans `<jeu>\Frogtend\medias` (importés : `<dossier>\Frogtend\<titre>\medias`),
> migrés au démarrage (copie vérifiée, puis l'ancienne retirée ; un disque absent attend) ; abris dans le réglage
> « dossier des abris » (sans lui, retirer un jeu qui a des parties est REFUSÉ ; anciens abris rangés à son réglage,
> lecture des deux endroits en attendant, la plus récente l'emporte) ; Cheat Engine et outils dans « dossier des
> outils » ; copie avant mod dans `<jeu>\Frogtend\copies\` ; le dossier `Frogtend` d'un jeu n'est jamais pris pour
> des parties ; import : « laisser » (source ajoutée aux emplacements du système) ou « copier » (pistes .cue/.m3u/.gdi
> et CHD MAME compris, place vérifiée, jamais par-dessus, originaux gardés). Les triches des émulateurs restent dans le
> dossier de l'émulateur (dossier des émulateurs : c'est là qu'ils les lisent).
> ⚠ Trouvé chez Seb (lecture seule, 02/10) : `%APPDATA%\com.frogtend.app` (8 Mo) et `com.frogtend.appmedia` (**3,1 Go**
> de packs de médias NES EmuMovies), datés de **mars 2026** : un ancien essai, pas le Frogtend actuel. **Supprimés à la
> demande de Seb (02/10)**, après vérification (aucun programme ouvert dessus ; 15 fichiers, 3,3 Go).
> Restent dans `%APPDATA%\Frogtend` : les données de Frogtend LUI-MÊME (profils, réglages, caches du catalogue et
> des skins, connexions des boutiques par profil, références de manettes) — rien qui appartienne à un jeu.

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

> ⚠ **Règle « réglé à l'installation » (Seb, 08/11).** Tout émulateur que Frogtend installe ou lance est réglé SEUL
> pour la personne : **la manette** (XInput standard, joueur 1) et **la langue en français** (d'après Windows), sauf
> indication contraire — « le même principe que RPCS3 ». Jamais par-dessus un choix fait par la personne.
> État au 08/11 : manette réglée pour RetroArch, DuckStation, PCSX2, Dolphin, RPCS3 ; langue pour RPCS3 seulement.
> Eden/Eglantine : en cours (0.164.0). Restent à passer en revue : DOSBox Staging, PPSSPP, Xenia, xemu, Cemu, Azahar,
> Vita3K, mGBA (et la langue de DuckStation, PCSX2, Dolphin, RetroArch).
> **Rappel de Seb (08/11, après Lichii, réglé seulement à la 2e partie) :** « frogtend DOIT configurer TOUS les
> émulateurs avec les langues, touches, etc. à chaque nouvelle installation. C'est très important pour être users
> friendly. Également il doit avoir son image des touches dans le menu. » → pour CHAQUE émulateur : manette, langue,
> bouton du menu de Frogtend, dès la PREMIÈRE partie d'une installation neuve (fichier de config pas encore créé),
> et son schéma « 🎮 Boutons » dans le menu en jeu. Testé sur une installation neuve à chaque fois.
>
> ⚠ **Tout ce que l'émulateur propose, dans le menu en jeu (Seb, 11/11, après l'essai de BigPEmu 0.184.0 : « pas de
> quick save et load, pas de touche non plus en raccourci, pas d'option pour shaders, pas de code triches […] quand un
> émulateur propose ce genre de chose et que c'est possible, il faut à tout prix que ce soit systématiquement inclus
> dans notre menu »).** Pour CHAQUE émulateur : sauvegarde et chargement rapides (avec leur raccourci), effets d'écran
> (shaders), codes de triche, et le reste de ce qu'il offre, dans le menu de Frogtend dès qu'il le permet ; relevé
> AVANT de livrer une fiche ; ce qui est impossible est dit à Seb avec sa raison. Audit de tous les émulateurs à faire
> (colonnes « États rapides », « Effets d'écran », « Triches » dans AUDIT-REGLAGES-EMULATEURS.md).
> Les raccourcis manette de sauvegarde rapide de Frogtend (Vue + LT sauver, Vue + RT charger, déjà décidés et montrés
> sous l'image de la manette) valent pour tout émulateur dont le menu sait sauver/charger.
> **Menus normalisés (Seb, 11/11, après BigPEmu 0.185.0 sans les emplacements) :** « quand cela est possible nos menus
> et actions doivent être normalisés ; ils sont différents uniquement quand des fonctions sont indisponibles sur
> certains émulateurs ». Référence : le menu de RetroArch (SNES) — sauvegarde/chargement rapides ET choix de
> l'emplacement (0 à 9), effets, triches, capture.
>
> ✅ **Décision de Seb (15/11)** : au clavier, des touches par défaut pour sauver et charger : Ctrl + Maj + F5 et Ctrl + Maj + F9 (modifiables) ; rubrique « ⚡ Raccourcis en jeu » après Émulateurs.
> ⚠ **Raccourcis réglables (Seb, 11/11) :** « ce sera cool que ce soit les réglages par défaut, mais qu'il y ait une
> rubrique dans les options pour définir les raccourcis manette ET clavier, et qu'ils restent tout de même à jour dans
> le menu des manettes ». → une rubrique des Réglages : les raccourcis de Frogtend (menu, sauvegarde et chargement
> rapides…) à la manette ET au clavier, ceux d'aujourd'hui par défaut ; l'image « 🎮 Les touches » du menu en jeu
> montre toujours les raccourcis réellement réglés. À faire après les états/effets/triches de BigPEmu.
>
> ⚠ **Relancer pour appliquer, et Reset partout (Seb, 12/11, après l'effet BigPEmu validé) :** « pour tout émulateur
> le supportant, un truc qui dit quand un restart est nécessaire, demander de confirmer et on restart auto pour la
> prise en compte des modifications ; en plus de quitter, un bouton reset éventuellement ; comme toujours normalisé
> avec tous les systèmes compatibles ». → (1) un réglage qui ne vaut qu'au prochain lancement (effet, triches de
> BigPEmu, compteur d'images de RPCS3…) propose « 🔄 Relancer maintenant », avec confirmation (la partie non
> sauvegardée serait perdue : proposer une sauvegarde rapide d'abord quand l'émulateur en a) ; Frogtend quitte
> proprement et relance le même jeu. (2) « Recommencer » (Reset) dans le menu pour TOUS les émulateurs qui le
> permettent (déjà : RetroArch, DuckStation, Dolphin, PCSX2, MAME, DOSBox Staging ; BigPEmu : son action « Reload
> Image » à relever ; les autres à l'audit).
>
> ⚠ **Mises à jour des émulateurs (Seb, 08/11, après l'essai de la 0.167.0 : « les mises à jour ont l'air bonnes et
> faites »).** (1) **L'avancement se voit** pendant une mise à jour lancée depuis ⚙ Réglages ▸ Émulateurs (« pas de
> suivi de l'avancement ») ; une seconde demande pendant qu'une autre tourne est dite clairement (essai : RPCS3 et
> Eglantine à la fois → « une installation est en cours », bien géré). (2) **Au démarrage**, si des émulateurs installés
> par Frogtend ont une nouvelle version, un message le dit et propose la mise à jour (toujours avec l'accord de la
> personne qui utilise le PC ; jamais en Taodbox ; hors ligne : rien).
>
> ⚠ **Afficher les images par seconde depuis le menu en jeu (Seb, 08/11, après Zelda OoT encore lent avec Eglantine
> 0.1.0-rc3 : « une option pour afficher ou non les FPS »).** Une entrée du menu de Frogtend (bouton Xbox) qui montre ou
> cache le compteur d'images par seconde DE L'ÉMULATEUR (son propre affichage), pour chaque émulateur qui le permet ;
> le choix est retenu. Les émulateurs qui ne le permettent pas : l'entrée n'apparaît pas.
> **Décision de Seb (08/11) : le choix vaut PAR PERSONNE, pour tous ses jeux** (ni par jeu, ni par PC).
> 0.171.0 : RetroArch, MAME (bascule), Eden/Eglantine/Lichii (clé au lancement + Ctrl+V), RPCS3 (au prochain lancement).
> RPCS3 par jeu : son réglage propre au jeu (custom_configs) est montré, jamais écrasé.
> **Seb, 18/11 (à l'essai des mods Switch sous Ryubing) : TOUS les émulateurs, quand c'est possible.** Codé le 18/11
> (pas encore livré ; relevés et sources dans `images_par_seconde.rs`) — tous « à ta prochaine partie » (aucun n'a de
> touche d'origine pour basculer en jouant), seulement en mode portable, émulateur fermé, fichier copié à l'abri,
> jamais créé : **DuckStation** (`[Display] ShowFPS`), **PCSX2** (`[EmuCore/GS] OsdShowFPS`), **Dolphin**
> (`-C Graphics.Settings.ShowFPS=…` sur la ligne de commande : rien d'écrit, vaut dès la 1re partie ; un `-C …ShowFPS`
> déjà dans sa ligne de lancement l'emporterait, il décide alors et le nôtre n'est pas ajouté), **PPSSPP**
> (`iShowStatusFlags`, celui du profil), **Cemu** (`<Overlay>` : `FPS`), **Vita3K** (`performance-overlay`, posé avant
> la fin de document `...`). PCSX2 et Cemu : une position d'affichage « aucune » passe à 3 seulement si aucune autre
> ligne des performances n'est cochée (sinon pas d'entrée, raison au journal), et Frogtend la remet à 0 en cachant
> (marque `Profils\.compteur-position-frogtend`).
> **Impossibles (pas d'entrée, raison à dire à Seb)** : **Ryubing** (les FPS ne sont que dans sa barre d'état, cachée
> en plein écran ; F4 la remontre avec la barre de menus et rétrécit l'image ; aucune clé — lu dans le miroir du code
> au 30/07/2026, plus récent non lisible), **Azahar** (barre d'état seulement, cachée en plein écran), **xemu** (une
> fenêtre de débogage à ouvrir à la main dans son menu, sans clé ni raccourci), **Xenia Canary** (aucun compteur : son
> profileur est retiré des versions publiées), **mGBA** (titre de la fenêtre seulement), **DOSBox Staging** (aucun
> compteur), **BigPEmu** (rien dans sa documentation officielle).
> RESTE : MAME au lancement (témoin) ; un réglage PROPRE à un jeu dans PPSSPP, PCSX2, DuckStation ou Dolphin (rare)
> passe devant et n'est pas lu (le menu pourrait alors se tromper).

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

**Ryubing, la fiche complète (Seb, 15/11 : « le b »)** — plan de l'expert lancement (code de Ryubing 1.3.3 lu) :
lot 0 relevé réel SANS jeu (Config.json v70 modèle, Profiles.json, arborescence, identifiant SDL2 de la manette) ;
lot 1 fiche + clés de l'ALTERNATIVE (`portable\system\`, jamais celles d'Eden) + firmware ; lot 2 réglé dès la 1re
partie (French/fr_FR, show_confirm_exit false, PauseEmulation, update Off, Discord off ; Config.json écrit Ryubing
FERMÉ, à l'abri avant, relu après) ; lot 3 une personne = ses données (voie A `-p <nom>` ou B `-r <dossier>`) ; lot 4
sauvegarde Firehouse (⚠ parties indexées par LibHac : jamais partie par partie) ; lot 5 menu (capture F8, muet F2,
turbo F13 posé, effets au prochain lancement ; triches REPORTÉES : pas encore de source de triches Switch ; pas de sauvegardes rapides ni de Recommencer natif) ; lot 6
mises à jour et DLC (`updates.json`, `dlc.json` dans `games\<id base x16>\` de la personne : Ryubing les lit à chaque lancement par `-f`, sans fenêtre ; `autoload_dirs` ne sert PAS dans ce cas et ouvre une fenêtre) ; ✅ Seb 16/11 : mises à jour et DLC en AUTOMATIQUE partout (plus d'accord « Rendre disponible ») ; un contenu que Firehouse n'a pas lui est ENVOYÉ pour qu'il le serve ensuite à tous (besoin 51) — ✅ contrat 1.42 servi le 16/11 : lot 7 de Ryubing = l'envoi (après l'essai de la 0.203.0) ; ✅ Seb 17/11 (avis des experts contrat et lancement) : lot 7 = détection à l'ouverture du panneau 📦 (route 1 « ce que Firehouse a », jamais dans le lancement), ACCORD CHIFFRÉ demandé à la personne à chaque fois (combien, Go, durée) avec une case « À l'avenir, envoyer tout seul » ; envoi en arrière-plan, un fichier à la fois, reprenable, en PAUSE pendant une partie ou un émulateur ouvert seul ; DÉBIT plafonné à la moitié du débit mesuré, réglage « sans limite » ; jamais le jeu, les clés, le firmware, les parties, ni chemin ni nom de profil ; ✅ Seb 17/11 : le format .nsz AJOUTÉ (lu, utilisé si l'émulateur le sait, envoyé) ; précisions demandées à Firehouse (besoin 52) avant le PREMIER envoi réel ; ✅ Seb 16/11 : AVANT l'envoi, un petit lot « ▶ Ouvrir l'émulateur seul » : sous « Lancer avec » (fiche d'un jeu), un bouton qui ouvre chaque émulateur du système SANS jeu, AVEC le profil de la personne (Ryubing `-r Profils\<nom>`, Eden et les autres : leur dossier de personne), pour gérer DLC, manettes, réglages là où ils servent vraiment (Ryubing lancé seul ouvre sa config commune, pas celle de la personne) ; suivi comme une partie (pas de réglage réécrit pendant qu'il est ouvert) ; et la LISTE DE JEUX de l'émulateur remplie (Seb 16/11 : « il n'y a pas de liste de jeux » — Ryubing `game_dirs` vide, idem Eden `Paths\gamedirs`) avec les emplacements de la plateforme, dans le profil de la personne ; ✅ Seb 17/11 (avis des experts lancement et interface) : bouton « ▶ Ouvrir l'émulateur seul » à côté de « ▶ Jouer avec… » (fiche et panneau de la ludothèque), jamais en Taodbox (interface à la souris) ; un émulateur réglé → ouvert tout de suite, plusieurs → choix avec ⭐ sur celui du jeu, aucun → « Prêt à jouer » ; sans plein écran ; pas de menu en jeu ; session Firehouse sans media_id (début, vivant, fin) ; ✅ Seb 17/11 : le temps passé n'est compté à AUCUN jeu ; ✅ Seb 17/11 : à la fermeture, sauvegarde automatique des parties vers Firehouse comme après une partie ; ✅ Seb 17/11 (après l'essai « Manage DLC » de Ryubing) : FROGTEND EST LA SOURCE DE VÉRITÉ des mises à jour et DLC Switch — un interrupteur par contenu dans le panneau 📦 de la fiche, allumé d'office (automatique), valable pour CE JEU quel que soit l'émulateur (Eden, Ryubing) et pour TOUTE LA MAISON (pas par personne) ; à chaque lancement Frogtend remet l'émulateur d'accord avec ce choix (un retrait fait dans l'émulateur ne compte plus : fin de la règle « retiré par la personne dans l'émulateur reste retiré » pour les contenus Switch) ; la mise à jour reste « la plus récente » (désactivable, pas de choix de version) ; ✅ Seb 17/11 : Eden ne sait couper que TOUS les DLC ensemble (`[DisabledAddOns]` : `DLC`, `Update` — source officielle) → dans Eden, tous les DLC restent actifs tant qu'au moins un est allumé (remarque visible sur la fiche), coupés quand tous sont éteints ; ✅ Seb 17/11 : les retraits déjà faits dans les émulateurs (dlc_refuses Ryubing, is_enabled:false, selected vide, refus Eden) sont REPRIS UNE FOIS comme « éteints » (migration), les anciens fichiers jamais effacés ; prudence : avertissement en éteignant un contenu et mise à l'abri des parties de ce jeu avant la partie suivante ; choix rangé dans `contenus-switch.json` du dossier de Frogtend, clé = title id de base ; listes de jeux FUSIONNÉES (jamais un dossier de la personne retiré, jamais remis un dossier qu'elle a retiré) — corrige aussi DuckStation, PCSX2, Dolphin qui REMPLAÇAIENT la liste à chaque partie ; ordre : Ryubing (`game_dirs`), Eden (`Paths\gamedirs`, commun à la maison, relire qt_config d'Eden avant), DuckStation/PCSX2/Dolphin ; plus tard RPCS3, Azahar, PPSSPP, RetroArch ; exclus DOSBox, xemu, MAME, BigPEmu, Cemu ; ✅ Seb 16/11 : ENSUITE, les MODS SWITCH avec Zelda BotW comme banc d'essai (jeu choisi exprès : mods, shaders, 4K, 60 i/s), « gérés de façon simple par Firehouse » — à concevoir avec Seb et Firehouse (le contrat 1.19 ne sert de mods qu'aux jeux PC) ; ✅ Seb 18/11 (recherche des sources officielles de Ryubing et Eden + avis de l'expert lancement) : les mods Switch sont SERVIS PAR FIREHOUSE (il les cherche, un admin les valide un par un, il les range et les sert avec reprise ; Frogtend ne cherche ni ne télécharge jamais un mod Switch ailleurs — différent des mods PC du contrat 1.19) ; choix 1a : Firehouse sert AUSSI les mods sans licence (usage familial, validés par un admin) ; choix 2a : un mod vaut pour TOUTE LA MAISON (comme les DLC), Frogtend source de vérité (un interrupteur par mod dans le panneau du jeu ; une case cochée dans l'émulateur ne compte pas) ; choix 3a : un mod qui ne va pas avec la version de mise à jour utilisée reste VISIBLE, grisé avec sa raison (jamais posé : l'émulateur l'ignorerait en silence) ; les mods sont gardés À CÔTÉ DU JEU (`<dossier Frogtend du jeu>\mods-switch\<TID>\<cle>\` — pas `mods\`, déjà celui des mods PC ; avis de l'expert lancement 18/11) ; COPIÉS (jamais de jonction) dans la racine Ryubing de la personne qui lance et mis en place à chaque partie dans l'émulateur choisi (Ryubing : `<racine de la personne>\mods\contents\<tid>\<nom>\` + `games\<tid>\mods.json` ; Eden : `user\load\<TID>\<nom>\` + `[DisabledAddOns]` par nom de dossier, noms « Update », « DLC », « SDMC » refusés) ; parties mises à l'abri avant la première partie qui suit un changement de mod ; demande à Firehouse = besoin 53 (✅ servi en contrat 1.44, 18/11) ; ✅ Seb 18/11 : Ryubing — la fenêtre de la CONSOLE cachée par défaut (`show_console: false`, une fois par personne ; « c'est moche et ça peut faire peur à ceux qui n'y connaissent rien ») ; ✅ Seb 18/11 (après l'avis des trois experts) : choix 1a — une NOUVELLE RÉVISION d'un mod posé est PROPOSÉE (accord chiffré, taille) avec une case « À l'avenir, mettre à jour tout seul » (comme les envois), jamais pendant un lancement, l'ancienne gardée tant que la nouvelle n'est pas vérifiée ; choix 2b — les mods se règlent AUSSI DANS TAODBOX, à la manette (« Il faut pouvoir faire des réglages depuis le canapé voyons ») : principe général, les interrupteurs des mises à jour et DLC Switch y vont avec (même panneau) ; règles des experts : conflits = allumer l'un éteint l'autre (après confirmation), `reglage_requis` jamais posé seul, abri des parties à TOUT changement de mod (allumé, éteint, révision, retrait), version inconnue = pas grisé mais « version pas vérifiée », Ryubing non portable ou émulateur ouvert = rien posé (raison sur la fiche), noms Update/Update@…/DLC/SDMC refusés sans casse, un nom déjà pris par la personne reçoit un suffixe ; piste vue par Seb (16/11) : le réglage « Autoload DLC/Updates Directories » de Ryubing (`autoload_dirs` du Config.json) — sans lui, Zelda BotW démarre en 1.0.0 sous Ryubing alors qu'Eden trouve seul la mise à jour 1.5.0. ✅ Décisions de Seb (15/11) : relevé réel SANS jeu accordé (dossier d'essai D:\temp-tests-frogtend) ; firmware POSÉ PAR FROGTEND (Ryubing fermé, vérifié, sans clic) ; UN DOSSIER PAR PERSONNE (`-r`, voie B). ✅ Seb 15/11 : `games\` = CHACUN LE SIEN (pas de jonction ; caches recompilés pour chaque personne ; un `games\<id>\Config.json` de la personne n'est jamais écrit par Frogtend, la pause y est lue). Plus tard (lots 2-3 relus) : le recours quand une restauration Ryubing reste en attente (la personne a déjà des parties ici) ; les mods et amiibo par personne ; relever la manette de Sunshine et le Bluetooth (seule la manette Xbox One 045e:02ff en USB est posée).

**PS4 et PS5 (Seb, 15/11 : « l'émulation PS4 et PS5 commence à bien fonctionner, nous allons donc devoir
l'intégrer ») :** à faire après les raccourcis réglables. D'abord une recherche dans les sources OFFICIELLES (quels
émulateurs, leur état, leur ligne de commande, leurs dossiers, manettes, langue, sauvegardes, firmware et clés
demandés), puis, comme pour toute console : fiche servie par Firehouse (besoin d'API si elle manque), installation sur
accord, réglage dès la première partie, menu en jeu complet et normalisé, sauvegardes chez Firehouse.

✅ FAIT (0.200.0) : **Recharger après « Sauver puis relancer » (✅ Seb 15/11 : « oui quand cela est possible ») :** après une relance
avec sauvegarde, Frogtend charge seul l'emplacement sauvé dès que le jeu est prêt (RetroArch, Jaguar, MAME) ; sinon,
menu ▸ 📂 Charger. À faire après la 0.199.0 (avis de l'expert lancement : quand le jeu est-il « prêt » ? Jaguar :
la fenêtre des triches au démarrage).

**Quitter Eglantine et Lichii en sécurité (Seb, 15/11) :** « souvent le logiciel plante après la fermeture du jeu,
donc peut-être attendre 5 secondes puis le tuer ». Aujourd'hui (`lancement::finir_la_partie`) : 10 s, puis arrêt
seulement si l'émulateur n'a PLUS de fenêtre visible (une fenêtre figée peut écrire une sauvegarde : jamais touchée).
À faire : pour la famille d'Eden, arrêt aussi d'une fenêtre restée figée après Quitter (sortie déjà acceptée par
Eden : `confirmStop=1` refuse tant que le jeu écrit) ; avis de l'expert lancement d'abord (parties sacrées).

**Les captures d'écran dans la sauvegarde (Seb, 12/11 : « il faut que les screens, comme les saves, soient
sauvegardés par Firehouse et proposés à la restauration lors d'une nouvelle installation d'un user »).** À faire :
vérifié le 12/11, elles n'y sont PAS (ni les images de 📷 Mes captures, ni l'index `captures.json` qui dit à quel
profil et à quel jeu chacune appartient). Prévu : les captures DU PROFIL (jamais celles d'un autre) partent avec ses
parties (`captures/…` dans la sauvegarde, seul ce qui a changé est renvoyé, taille comptée dans l'annonce) avec leur
fiche (jeu, titre, console, date) ; à la restauration, remises dans le dossier des captures choisi sur ce PC
(`<Système>\<jeu>\`), notées pour le nouveau profil, jamais par-dessus une image existante. Une capture
supprimée dans Frogtend sort de la version courante (l'historique de Firehouse la garde).

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

**Décisions de Seb (30/09)** : combinaison ~~Select + Start~~ → **Select + R1 tenus 1 s** (changée par Seb le 04/10 : Select + Start sert dans certains jeux) à la manette, touche **Pause/Attn** au
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
- ~~**Pas d'API Nexus Mods** (trop chère).~~ Remplacé le 19/10 : la clé d'API PERSONNELLE de chaque
  utilisateur est gratuite et « ne demande aucune approbation » (annonce officielle de Nexus) ; voie `nxm://`
  enregistrée par l'installeur, livrée en 0.82.0.
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
les logiciels en plus, en fait un zip et le confie à Firehouse**, qui le **distribue** à tous (contrat 14, `type:
direct`, `sha256`). Firehouse **prévient Seb** quand une nouvelle version sort ; Seb met à jour le zip sur Firehouse,
qui le transmet à tous les Frogtend. (Ce qui suit est la réflexion qui a mené là.)

**Relevé dans les sources de Cheat Engine (02/10, `Cheat Engine/ceregistry.pas`)** : ses réglages vont dans le
**registre** (`HKCU\Software\Cheat Engine`), sans option portable. La copie zippée marche, mais laisse des réglages
dans le registre (« effet pieuvre »). Piste, **à annoncer à Seb avant tout (registre)** : Frogtend exporterait cette
clé par profil après usage et la remettrait avant (`reg export` / `reg import`), ce qui donnerait aussi des réglages
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
décrit (contrat 14 : `type: direct`, `programme`, `sha256`), sans rien de plus à coder. Avis de l'agent : voir la
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
**Décision de Seb (20/11)** : l'obtention automatique est ouverte à TOUS les profils, mineurs compris, tant que le jeu
est gratuit (« peu importe ») — même si « Add to library » d'Epic vaut déclaration d'avoir plus de 18 ans.
**✅ 0.24.0 (Epic)** : liste publique (`freeGamesPromotions`, vérifiée le 02/10 : System Shock 2 25th Anniversary
Remaster et BURIED STARS jusqu'au 08/10) ; « 🔑 Se connecter à Epic » (page officielle, navigateur PROPRE AU PROFIL dans
`profils\<id>\navigateur`) ; « 🎁 Obtenir » ouvre la page `/en-US/p/<slug>` CACHÉE, le script `ressources/gratuits/
epic.js` clique Get → licence → Place Order → I Agree et rend le résultat par le titre ; connexion requise, captcha ou
échec → la fenêtre s'affiche (repli A) ; case « les obtenir tout seul » (par profil, 1 fois par jour, désactivée par
défaut). **Pas encore essayé en vrai** (compte Epic de Seb). ~~Reste : Prime Gaming, GOG~~ **✅ 0.36.0** : GOG (bannière
du jeu offert, puis l'adresse officielle `/giveaway/claim` : « {} » = ajouté, « Already claimed » = déjà) et Prime
Gaming (offres « Claim » prises sur Amazon ; offres d'autres boutiques — code GOG/Legacy, compte Epic/EA à relier —
COMPTÉES et laissées à finir sur la page affichée) ; méthode relevée dans vogler/free-games-claimer (rien n'est copié) ;
connexion propre au profil ; cases « tout seul » (GOG chaque jour, Prime chaque semaine). GOG inscrit à sa lettre
d'information quand on prend un jeu offert : Frogtend NE change PAS les réglages du compte (à décider avec Seb). Pas de
jeu offert GOG le 02/10 : la bannière n'a pas pu être vue en vrai (script tolérant aux noms de classes).

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
**✅ 0.27.0 — jeux locaux** (`src-tauri/src/import_local.rs`, `src/lib/import/`) : Fichiers ROM (dossier + plateforme +
extensions proposées d'après les émulateurs de Firehouse, sous-dossiers ; les pistes désignées par un .cue/.m3u/.gdi ne
comptent pas à part ; titres sans étiquettes No-Intro/Redump, FICHIER jamais renommé ; liste à cocher, titres
modifiables, confirmation chiffrée avant l'ajout), Jeux MS-DOS (un sous-dossier = un jeu, programme probable
proposé, lancé par l'émulateur MS-DOS), Jeux Windows (le .exe), Ajouter un jeu manuellement (titre, plateforme,
fichier). Un jeu importé : id négatif « local », dans la ludothèque du profil (filtre Boutique ▸ « Importés de mon
disque ») ET dans le registre du PC (`VERSION_IMPORTEE` = -1) pour « Jouer ». **Le retirer ne touche à AUCUN fichier**
(ni désinstalleur, ni suppression : vérifié par un test). Essayé en lecture seule sur le disque de Seb : E:\Games\NES
→ 1 086 jeux en 0,16 s ; E:\Games\3DO → 200 .chd. **✅ 0.28.0 — MAME Arcade Full Set** (`src-tauri/src/mame.rs`) : tri
d'après la liste MAME de LaunchBox (`Metadata\MAME.xml`, lue en lecture seule, choisie une fois, réglage PC
`listeMame`) ; mêmes cases que LaunchBox (imparfaits et quiz gardés par défaut ; clones, non jouables, contrefaçons,
prototypes, hacks, casino/mahjong, mécaniques, hors arcade, adultes écartés) ; un zip absent de la liste (BIOS,
appareil) est écarté ; année, éditeur et genre repris. Essai réel en lecture seule : E:\Games\MAME, 14 191 zips →
**4 526 jeux** en 3 s. **✅ 0.29.0 — Installer un jeu DOS** (comme LaunchBox) : source = dossier du CD, image .iso/.cue
(lecteur D:) ou disquette .img/.ima (A:) ; destination = un sous-dossier NEUF au nom du jeu dans l'emplacement MS-DOS
(jamais par-dessus un dossier non vide) monté en C: ; DOSBox (celui réglé pour MS-DOS) s'ouvre, la personne installe,
puis Frogtend propose le programme du jeu ; le jeu se relance par DOSBox avec C: au même endroit et `cd` dans son
dossier (il retrouve ses chemins). **Le menu « Importer » est complet** (13 entrées de LaunchBox). **Reste** : jaquettes et fiches des jeux importés (chercher
le jeu dans la base LaunchBox de Firehouse, `/recherche`).

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
  (`steamcommunity.com/id/<nom>`) et une **clé d'API personnelle** (Steam révoque celles de LaunchBox : chacun crée
  la sienne sur `steamcommunity.com/dev/apikey`).
- **Règle de Seb** : les comptes se règlent **dans les Options**, OU **au moment d'un import** si rien n'est encore
  réglé (Frogtend ouvre alors le réglage avant d'importer).
- **Ordre retenu** : Steam d'abord (API officielle : ResolveVanityURL, GetOwnedGames), puis les autres après étude.
- ⚠ La clé Steam de Seb est apparue en clair dans une capture (02/10) : à régénérer ; jamais recopiée par l'agent.
- **Epic dans LaunchBox (captures de Seb, 02/10)** : « Assistant d'importation de jeux Epic » ; s'il n'est pas connecté,
  il ouvre une **fenêtre avec la page de connexion OFFICIELLE d'Epic** (e-mail, ou Google, Steam, PlayStation, Xbox,
  Nintendo…), puis analyse la bibliothèque. **Seb : « ça ne fonctionne pas très bien »**. En revanche LaunchBox
  **propose d'installer les jeux Steam (et normalement Epic) depuis son interface** → à reproduire (Steam : fait en
  0.20.0 par `steam://install`).

**✅ Steam en 0.20.0** : ⚙ Options ▸ **Comptes ▸ Steam** (compte = URL personnalisée ou identifiant à 17 chiffres ;
clé d'API personnelle, **vérifiée auprès de Steam avant d'être enregistrée**, rangée dans le coffre de Windows sous
`jeton:<profil>#steam`, jamais réaffichée ; erreurs rendues sans l'adresse qui contient la clé). **🛒 Boutiques** :
import (réglage demandé d'abord s'il manque), liste filtrable, installés sur CE PC (`libraryfolders.vdf` +
`appmanifest_<id>.acf`, registre lu seulement), ▶ Jouer / ⬇ Installer par `steam://`. Profil Steam privé : Frogtend dit
quoi changer. Supprimer un profil retire aussi sa clé Steam. **0.21.0** : jaquettes officielles de Steam (protocole `boutique://`, cache sur ce PC), grille, et section Steam dans
Taodbox. **Reste** : mêler ces jeux à la ludothèque principale, puis Epic, GOG, Amazon, EA, Ubisoft, Xbox (étude de LaunchBox d'abord), et les jeux gratuits.

**✅ Seb 18/11 — IMPORT AUTOMATIQUE des jeux PC des boutiques** (sans clé d'API ni GOG Galaxy à installer/relier ;
étude en lecture seule du 18/11 sur Venkman + méthode de Playnite, sans recopier son code) : découpage ACCEPTÉ
(choix 1) — **Lot A** : jeux INSTALLÉS de toutes les boutiques (Steam `libraryfolders.vdf`/`appmanifest` ; Epic
`Manifests\*.item` + `LauncherInstalled.dat` ; GOG registre `GOG.com\Games` ; Ubisoft `Launcher\Installs` ; Amazon
`GameInstallInfo.sqlite` ; Xbox paquets AppX à `MicrosoftGame.config` ; Battle.net `product.db`), relus À CHAQUE
DÉMARRAGE, Jouer/Installer par les protocoles officiels ; **Lot B** : jeux POSSÉDÉS sans connexion — Epic par
`Data\Catalog\catcache.bin` (510 jeux chez Seb contre 43 par Galaxy ; ✅ Seb 18/11 : « ça me semble probable »,
format non documenté → tolérant) et Ubisoft par son cache `configurations` ; **Lot C** : connexion par la PAGE OFFICIELLE
(fenêtre, jeton au coffre, jamais de mot de passe dans Frogtend) pour Steam (remplace la clé d'API) puis GOG ; **Lot D** :
même connexion pour Amazon, Xbox, Battle.net — ✅ Seb 18/11 (choix 2 : « oui bien sûr ») : Frogtend PEUT se présenter
comme le lanceur officiel (identifiant client du lanceur, API non publiques, comme Playnite) ; **Lot E** : EA plus tard
(fichier `IS` illisible ; GOG Galaxy reste le secours). L'import par Galaxy et la clé Steam restent possibles. ✅ Seb 18/11 (après la relecture du lot B) : (1) les jeux POSSÉDÉS lus sur ce PC (compte Epic/Ubisoft connecté sur
Windows) n'entrent dans un profil qu'après SON ACCORD, demandé UNE fois (« Ajouter les N jeux du compte Epic connecté sur ce
PC à ta ludothèque ? » ; non = jamais pour lui, changeable dans les Options) — les jeux INSTALLÉS restent pour tous ; (2) un
jeu possédé qui disparaît du catalogue (changement de compte…) est TOUJOURS GARDÉ, « pas installé » ; (3) Taodbox MONTRE
aussi les jeux possédés pas installés, et l'installation se LANCE depuis le canapé (par le lanceur de la boutique ; sa
fenêtre de confirmation peut demander la souris : le dire au moment du lancement).

**✅ Seb 19/11 — LOT C, découpage et règle** (étude en lecture seule du 19/11, méthode de Playnite relue sans recopier) :
**C1** connexion Steam par sa page officielle (fenêtre avec le navigateur PROPRE AU PROFIL, Steam Guard sur la vraie
page, « ✅ Connecté » quand `#application_config` dit `logged_in`, seul le `steamid` gardé) ; **C2** import des jeux
Steam POSSÉDÉS (fenêtre cachée, `webapi_token` relu à chaque fois et JAMAIS écrit ni journalisé, `IPlayerService/
GetOwnedGames`) ; la clé d'API reste en secours ; **C3** « Oublier ma connexion Steam » et messages clairs (page
changée, trop de demandes, profil privé) ; **C4** connexion et import GOG par les cookies du profil (sans Galaxy) ;
C5 (OAuth de Galaxy) seulement si besoin. **Règle** : SE CONNECTER VAUT ACCORD — les jeux possédés du compte connecté
entrent dans la ludothèque du profil, « pas installés », et RESTENT même après une déconnexion. Une livraison par étape.
**Seb 20/11 — C2** : les jeux Steam possédés se récupèrent À L'OUVERTURE DE FROGTEND (en arrière-plan) et À LA MAIN
depuis l'interface (« 🔄 Mettre à jour mes jeux Steam »), plus une fois juste après la connexion. Le `webapi_token` dure
environ un jour : il n'est JAMAIS gardé, il est relu à chaque import grâce à la connexion du navigateur du profil ; si
cette connexion expire, Frogtend le dit (« reconnecte-toi ») et les jeux déjà importés restent. Le jeton doit être
celui du compte connecté (son `sub` = le `steamid` rangé), sinon refus. Taodbox montre aussi ces jeux « à installer ».
**Seb 20/11 — changement de compte Steam sur un profil** : les jeux de l'ANCIEN compte jamais installés ni joués
quittent la ludothèque ; ceux installés, joués ou réunis à une autre copie restent. « Oublier ma connexion » n'efface
AUCUNE carte (revue de l'expert du lancement : `boutique_steam_oublier` vidait toutes les cartes Steam).
**21/11 — C2 validé par Seb (142 jeux, 0.215.3). C3 (« oui » de Seb)** : « Oublier ce compte » (aussi dans 🛒 Boutiques)
déconnecte AUSSI Steam du navigateur du profil — seuls les cookies des domaines Steam, jamais le dossier ni les autres
boutiques (Epic, PlayStation y vivent) ; la déconnexion n'est dite que PROUVÉE par relecture ; « reconnecte-toi »
seulement si la page dit le compte déconnecté, sinon « page changée » / « réponse inhabituelle ».
**Seb 21/11 (interface)** : le bloc « Mon compte Steam » va AVEC les autres comptes (Epic, PS Plus, GOG, Prime Gaming)
dans 🛒 Boutiques, pas isolé sous « Mes jeux installés ».
**✅ 21/11 — C3 validé par Seb (0.216.1). C4 GOG, décisions de Seb (21/11)** : (1) la connexion GOG déjà faite pour
les jeux offerts VAUT ACCORD (même navigateur du profil) : les jeux du compte entrent à l'ouverture suivante, avec le
message habituel ; (2) les jeux CACHÉS sur gog.com entrent AUSSI (tous les jeux possédés) ; (3) sans GOG Galaxy,
« ⬇ Installer » PROPOSE d'installer Galaxy depuis sa source officielle (taille et durée dites, rien sans un oui), puis
Galaxy installe le jeu — Frogtend ne télécharge jamais le jeu lui-même. Découpage (avis des experts) : **C4a** lecture
par la page (fenêtre cachée sur www.gog.com, étiquette à part, remise par morceaux numérotés demandés par le cœur),
`userId` gardé, `fusion_gog` dans la relecture du PC (même id `gog_<id>` que Galaxy, le compte passe avant Galaxy
comme pour Steam), bloc « 🟣 Mon compte GOG » unique (jeu offert + jeux du compte), import après connexion / à
l'ouverture / à la main, Taodbox relit ; **C4b** changement de compte et « Oublier » (cookies gog.com seulement) ;
**C4c** installer Galaxy après accord.
**21/11 — C4a essayé par Seb : 88 jeux lus, GOG Galaxy en montre 197.** Sondes (0.217.1-3) : la liste du site
(`getFilteredProducts`) annonce 88 ; `/user/data/games` donne 160 numéros (72 en plus, surtout des licences « Amazon
Prime / Luna », type `spam` dans gamesdb) ; la bibliothèque de Galaxy (`galaxy-library.gog.com/users/<id>/releases`)
REFUSE le jeton du site (403). **Décision de Seb (21/11) : C5 « connexion façon Galaxy »** — OAuth de GOG avec
l'identifiant public de GOG Galaxy (méthode de Heroic, Playnite, Lutris), dans la fenêtre officielle et le navigateur
du profil (silencieuse si la session GOG est ouverte) ; jeton de renouvellement dans le coffre de Windows, jamais écrit
ni journalisé ; lecture de la bibliothèque de Galaxy + base publique gamesdb pour titres, types et jaquettes. Marche
sur tous les PC, Galaxy installé ou non. D'abord une sonde (types réels des 197), puis le rangement.
**22/11 — 0.218.2 essayée : la connexion façon Galaxy MARCHE (160 licences lues, 88 jeux + 72 licences Amazon
« spam » invisibles, écartées à raison).** La cause des 197 : une COPIE de la base de GOG Galaxy
(`C:\ProgramData\GOG.com\Galaxy\storage\galaxy-2.0.db`) montre que Galaxy est connecté à un AUTRE compte GOG
(197 jeux possédés, visibles, hors DLC, dont Cyberpunk et Horizon ; aucun jeu en commun avec les 160). Seb ne le
savait pas : un compte lié à Google, l'autre à une adresse Hotmail (celui de Frogtend reçoit les jeux Prime).
**Décision de Seb (22/11) : « les deux comptes »** — Frogtend accepte PLUSIEURS comptes GOG par profil, chacun
avec sa connexion et son jeton de renouvellement (coffre de Windows), et réunit leurs jeux dans la ludothèque
(un jeu possédé deux fois = une seule carte) ; « ➕ Ajouter un compte GOG » et « Oublier ce compte » par compte.
Intégré au lot **C4b** (qui portait déjà le changement de compte et « Oublier »). Idée à vérifier sur la
documentation : Frogtend pourrait DIRE qu'un autre compte est ouvert dans Galaxy (lire sa base en lecture seule).
**22/11 — C4b livré (0.219.0-0.219.2) et essayé par Seb** : 2 comptes, 88 + 197 = 285 jeux, cartes « non-jeux »
retirées, garde du compte en double et avertissement d'installation validés. **Décision de Seb (22/11) : EXCEPTION
GOG à la règle « les jeux ne viennent que de Firehouse »** — pour un jeu GOG d'un compte connecté, Frogtend peut
TÉLÉCHARGER L'INSTALLEUR OFFICIEL depuis GOG (jeux sans DRM, connexion façon Galaxy du compte qui le possède) et
l'installer lui-même, toujours après l'accord de la personne (taille et durée annoncées, reprise des
téléchargements). Contrepartie dite à l'écran : pas de sauvegardes dans le nuage ni de succès de GOG Galaxy pour un jeu
installé ainsi. Ce lot **remplace le C4c** (« proposer d'installer Galaxy »). L'exception vaut pour GOG seulement.
**Décision de Seb (22/11, essai de Cyberpunk) : LANCER LES JEUX DIRECTEMENT, COMME LAUNCHBOX, sans passer par les
lanceurs.** Chez Seb, « ▶ Jouer » sur Cyberpunk passait par GOG Galaxy (`runGame`) puis le lanceur de CD Projekt
(REDprelauncher, tâche « principale » du `goggame-<id>.info`), écran noir avant le jeu. Pour GOG (sans DRM) : lancer la
tâche de catégorie « game » du `goggame-<id>.info` (Cyberpunk : `bin\x64\Cyberpunk2077.exe`), avec ses arguments et
son dossier de travail ; Galaxy seulement en secours si aucune tâche « game » n'est lisible. Contrepartie : pas de
synchronisation des sauvegardes dans le nuage de GOG ni des succès de Galaxy. ⚠ Vérifié le 22/11 (expert lancement) :
la sauvegarde vers Firehouse n'emporte AUJOURD'HUI PAS les parties d'un jeu GOG rangées hors du dossier du jeu
(`Saved Games`, Documents, AppData) — seul le nuage Steam y passe (`plan_nuage`, `nuage_steam::emplacements` filtré sur
« Steam »). **Le C4c doit donc d'abord faire partir les parties des jeux GOG chez Firehouse** : emplacements servis par
Firehouse (`sauvegardes`, d'après PCGamingWiki) avec la plateforme « Windows » sans exiger Steam, gardés en cache pour
le hors ligne, mise à l'abri dans `abris/<id>/` après chaque partie et avant toute réinstallation, mise à jour ou
retrait, restauration vers le bon dossier sur un nouveau PC. Intégré au **nouveau C4c** (installer et lancer les jeux
GOG sans Galaxy).
Steam et les jeux Epic protégés exigent leur lanceur : la règle ne s'y applique pas telle quelle (IDEES.md).
**Décision de Seb (22/11) : c'est un DÉFAUT — les parties de TOUS les jeux PC, de TOUTES les boutiques (Steam hors
nuage compris, Epic, GOG, Ubisoft, Amazon, Xbox, jeux importés du disque), partent chez Firehouse, « y compris les jeux
pénibles qui cachent leurs parties dans Mes documents, AppData, etc. »** Lot **S** (sauvegardes de tous les jeux PC),
à faire AVANT le C4c (les parties sont sacrées) : emplacements servis par Firehouse (`sauvegardes`, PCGamingWiki) pour
chaque jeu PC de la ludothèque quelle que soit sa boutique (plateforme « Windows »), cache hors ligne, mise à l'abri
après chaque partie et avant réinstallation / mise à jour / retrait, envoi par la sauvegarde existante, restauration
au bon endroit sur un autre PC. Un jeu dont Firehouse ne connaît pas les emplacements : le dire (besoin d'API si
nécessaire), jamais deviner en copiant tout un dossier Documents.
**Précisions de Seb (22/11)** : (1) les JEUX POUR ADULTES servis par Firehouse (F95zone, admins seulement) sont
concernés aussi — souvent des moteurs que PCGamingWiki ne connaît pas (Ren'Py, RPG Maker, Unity…) : il faudra savoir
où ils rangent leurs parties (règles par moteur, documentation officielle de chaque moteur) ; leurs parties ne vont
qu'au profil admin qui y joue (jamais montrées, mises en cache ni journalisées pour un autre profil). (2) Savoir où sont
les parties de chaque jeu est la base de **l'éditeur de sauvegardes prévu dans Frogtend** : le lot S doit ranger ces
emplacements de façon réutilisable (par jeu : dossier, fichiers, moteur), pas seulement pour la copie.
**Lot S — plan des experts lancement et contrat (22/11, avant codage).** Constats : Firehouse ne reconnaît un jeu que
par fiche ou par `titre/plateforme/rom/serial/steam_appid` (aucun identifiant GOG/Epic/Ubisoft/Amazon/Xbox) → besoin
d'API 56 ; ⚠ DÉFAUT DÉJÀ PRÉSENT : les parties Steam « hors nuage » mises à l'abri ne sont PAS restaurables (chemin
d'origine non gardé, `nuage_steam.rs:238-239`, `:578-581` ; remise sautée, `partie.rs:74-75`) ; les abris sont ceux du
PC, pas du profil (seul Steam filtre par compte) ; une erreur sur un seul fichier arrête toute la sauvegarde
(`sauvegarde.rs:278`) ; le quota Firehouse (100 Go par personne) n'est jamais lu. Règles : emplacements rangés PAR
PROFIL (`profils/<id>/emplacements-parties.json` : chemin NON résolu avec jetons, nature, origine
firehouse/moteur/manifeste, preuve, moteur, `lu_le`) — jamais dans un dossier commun (un chemin révèle le titre d'un
jeu réservé aux admins) ; chaque abri a sa fiche (profil propriétaire, chemins non résolus, SHA1) sous un préfixe
propre (`jeux/<id>/emplacements/<n°>/…`) ; envoi DEPUIS l'abri ; restauration avec `reposer_sauf` (jamais écraser une
partie différente : conflit proposé), JAMAIS `reposer_parties_du_jeu` (écrase) ; registre refusé dans S ; journal :
l'id et le code seulement, jamais un chemin ni un titre ; un fichier en échec est sauté et compté (sauf 401/507).
Sous-lots, chacun livré et essayé seul : **S0** relevé réel en lecture seule (titres GOG/Epic de Seb demandés à
`/triche/recherche`, combien se résolvent ; fixture) ; **S1** modèle + cache par profil + « 📁 Où sont mes parties »
sur la fiche (aussi dans Taodbox) ; **S2** abri à la main pour tout jeu PC + correction du défaut Steam hors nuage ;
**S3** abri automatique après une partie (temps de calme, jamais pendant que le jeu écrit) et avant réinstallation /
mise à jour / retrait / mod ; **S4** envoi chez Firehouse filtré par propriétaire, quota lu et annoncé ; **S5**
restauration au bon endroit avec conflits, aussi à la manette ; **S6** règles par moteur (Ren'Py, RPG Maker, Unity,
Unreal, Godot) après lecture des documentations officielles, et test « une partie de jeu admin n'apparaît jamais pour
un mineur » ; **S7** rattrapage des parties jouées hors de Frogtend. Piège : plusieurs profils Frogtend sur un même
compte Windows partagent `%APPDATA%` → un abri appartient au profil qui a lancé la partie (fichiers modifiés après son
départ), un abri sans propriétaire n'est jamais envoyé sans demander.
**S0 fait le 22/11 (accord de Seb, lecture seule, essai `essai_sauvegardes_s0`)** : 40 jeux GOG et 40 jeux Epic de Seb
demandés à `/triche/recherche` par le TITRE et la plateforme Windows (400 ms entre deux). GOG : 33 avec emplacements,
1 `[]`, 6 `null` (pas encore lus) ; Epic : 16 avec emplacements, 1 `[]`, 23 `null`. Valeurs de `os` vues : Windows
48, Linux 10, OS X 10, Microsoft Store 8, DOS 3, Epic Games Store 1, Epic Games Launcher 1, GOG.com 1, Steam 1.
Cyberpunk : `%USERPROFILE%\Saved Games\CD Projekt Red\Cyberpunk 2077` (dossier), le bon (réponse rangée en fixture
`src-tauri/fixtures/triche-cyberpunk-sauvegardes.json`). Compte transmis à Firehouse. Besoin 56 REÇU par Firehouse. ⚠ Firehouse (22/11) : ce rythme (~2 demandes/s) a fait
refuser PCGamingWiki (Cloudflare, 429) depuis 13:56 → les `null` NE VEULENT RIEN DIRE (« PCGamingWiki muet »).
Relevé à refaire après 30 min au moins, à UNE demande toutes les 5 s au plus (essai corrigé) ; Firehouse ajoute un
frein sur ses appels à PCGamingWiki dans la livraison du besoin 56.
**S1 livré en 0.220.0 (22/11)** : contrats 1.48 et 1.49 servis par Firehouse le jour même (index Ludusavi, gog_id,
xbox_pfn, dossier vérifié par le titre, références fermées) ; relevé réel : GOG 39/41, Epic 33/40 ; relu par les trois
experts (18 corrections, dont les dossiers OneDrive jamais parcourus — essai réel : 166 fichiers vus dans
Documents\My Games\Age of Empires IV sous OneDrive —, une course entre profils, des racines interdites).
**S2 livré en 0.221.0 (22/11)** : abri À LA MAIN de tout jeu PC (module abri_parties.rs : clé du chemin NON résolu,
fiche à côté de l'abri avec profil propriétaire et SHA1, tout ou rien, refus si le jeu tourne ou a écrit depuis moins
de 60 s) ; défaut Steam hors nuage corrigé pour les nouveaux abris ; relu par les trois experts (15 corrections, dont
une régression de la sauvegarde entre profils : chaque profil envoie SA dernière copie, fusion par emplacement, et la
fiche part chez Firehouse en jeux/<id>/emplacements/fiche.json). À régler au S5 : le sel des empreintes d'identifiant
vit dans les données du profil (sur un autre PC, ré-énumérer) ; un parent qui échoue à la COPIE fait perdre cette
fois-là les emplacements qu'il contient (seul l'échec au plan est géré).
**Décision de Seb (22/11, essai d'Age of Empires IV : 9 fichiers, 180 Mo seulement dans OneDrive) : PROPOSER DE LES
RAPATRIER.** Quand des parties ne sont que dans le nuage (OneDrive « en ligne seulement »), la fenêtre d'abri propose
« Télécharger depuis OneDrive puis mettre à l'abri » avec la taille annoncée ; rien sans le oui de la personne ;
jamais automatiquement. « 📁 Où sont mes parties » dit aussi tout de suite que ces fichiers sont dans le nuage.
**Lot S3 — décisions de Seb (23/11, après l'avis des experts lancement et interface)** : (1) si l'abri d'avant
retrait / réinstallation / mise à jour / mod ÉCHOUE, la fenêtre maison propose PARTOUT « 🔁 Réessayer », « Continuer
sans abri » (seconde confirmation, focus sur Annuler) et « Annuler » ; l'option OneDrive (télécharger puis réessayer)
quand c'est la cause ; (2) après une partie, attendre le CALME (au moins 60 s sans écriture, aucun programme du dossier
du jeu) jusqu'à 10 MINUTES au plus, puis abandonner sans bruit inutile et réessayer à la partie suivante (repère « abri
à faire » par profil, id et date seulement) ; (3) INTERDIRE de changer de profil Frogtend pendant qu'un jeu tourne (comme
pour l'émulateur ouvert seul) ; (4) l'abri APRÈS UNE PARTIE se coupe par un interrupteur du profil, ALLUMÉ d'office, dans
Options ▸ Sauvegarde ET dans les réglages de Taodbox ; l'abri AVANT un retrait / une réinstallation / un mod ne se coupe
jamais. Règles des experts retenues : fiche de sorte `emplacements` gardée, avec un champ `declencheur` (main,
apres_partie, avant_retrait, avant_reinstallation, avant_mise_a_jour, avant_mod) — jamais une nouvelle sorte (la remise
d'après réinstallation poserait `emplacements/` dans le jeu) ; en automatique jamais de téléchargement OneDrive ni de
copie au-delà de 2 Go sans question (signalé, ligne persistante sur la fiche) ; « rien de neuf » = aucun abri et aucun
message ; un abri réussi = un toast court, et « 🛟 Dernier abri : … » sur la fiche ; abri avant le désinstalleur au
retrait, avant l'installeur à la réinstallation, et pour les mods de jeux non-Steam et Steam (cache S1, hors ligne) ;
aucun abri effacé (rétention = idée à part) ; le relevé du calme par dates et tailles seulement, jamais un SHA1 en
boucle. Les abris automatiques ne partent chez Firehouse qu'au S4.
**S3 livré en 0.222.0 (23/11)** : abri automatique après une partie (attente du calme, repère « abri à faire » et
dernier état par profil), avant retrait / réinstallation / mod (abri à part, fenêtre Réessayer / Continuer sans abri /
Annuler, accord sans abri par profil et pour une minute), changement de profil interdit pendant une partie,
interrupteur bureau + Taodbox ; relu par les experts lancement et interface (1 bloquant corrigé : sans cache S1 l'abri
d'avant passait en silence → Frogtend demande d'abord à Firehouse, sinon cause « inconnu » ; plus 9 importants et les
mineurs). Reste : `avant_mise_a_jour` défini mais non branché (aucun parcours de mise à jour distinct de la
réinstallation pour un jeu PC) ; après un redémarrage, la ligne d'échec revient sans sa raison.
**Lot S4 — décisions de Seb (23/11, après l'avis des experts lancement et contrat)** : (1) TOUS les abris de sorte
`emplacements` partent chez Firehouse, ceux faits à la main compris (fusion par emplacement : la copie la plus récente
du profil propriétaire, chemin `jeux/<id>/emplacements/<clé>/…` — la clé, pas un numéro) ; (2) le SEL des empreintes
d'identifiant (`profils/<id>/abris-sel.txt`) est ENVOYÉ dans l'espace du profil (restauration sur un autre PC sans
question ; contrepartie dite à Seb : Firehouse pourrait retrouver l'identifiant du compte Steam) ; (3) un emplacement dont
des fichiers ne sont que dans OneDrive GARDE leurs anciennes copies chez Firehouse (réunion avec la copie précédente,
`partiel` marqué ; une partie effacée par le jeu peut revenir à la restauration) ; (4) relevé réel accordé et fait
(23/11, lecture seule, essai `essai_quota_s4`) : `/moi ▸ sauvegarde` = `quota_octets` 107374182400, `morceau_max`
8388608, `document_max_octets` 5242880, documents `configuration.json`, `bibliotheque.json`, `manifeste.json`,
`derniere-sauvegarde.json` et le motif `manifeste-<clé du jeu>.json` ; `GET /sauvegarde` = `utilise_octets` et
`quota_octets` à la racine, `profils[{profil, pc, derniere, octets, manifestes_jeux}]` (Seb : 30,7 Mo utilisés). Le
quota est donc lisible ; précisions demandées (besoin 57). Règles des experts retenues : la sauvegarde d'après partie
attend la fin de l'abri automatique (événement `parties-abris-finis`, toujours émis) ; un verrou côté cœur et un
« à refaire » au lieu d'un abandon muet ; un fichier en échec est SAUTÉ ET COMPTÉ en gardant sa ligne de l'ancien
manifeste (jamais supprimé chez Firehouse) — arrêt sur 401, 507, réseau, 503 ; la fiche part AVANT ses fichiers ;
manifestes par jeu (`manifeste-<clé>.json`) si le manifeste approche 5 Mo ; sha256 des abris mis en cache (un abri ne
change jamais) ; place libre vérifiée AVANT l'envoi (quota − utilisé) et annoncée ; pause de l'envoi pendant une partie
(`Cadence`) ; le journal commun ne reçoit plus aucun chemin relatif (id, nombres et classe d'erreur seulement) ; le
compteur « N jeux non envoyés » ne compte que les jeux de CE profil ; les abris de sorte `jeu` filtrés par propriétaire.
**S4 codé le 23/11, NON LIVRÉ (décision de Seb : attendre que Firehouse annonce la 1.51 en service)** : relu par les
experts contrat et lancement (1 bloquant corrigé : sel envoyé sous le mauvais nom → `abris-sel.json` ; un important :
un abri `jeu`/`nuage-steam` d'un autre profil faisait reculer ou retirer la copie de ce profil → protégé par `gardes`).
Comptage local (essai `essai_abris_s4`, lecture seule) : 3 jeux, 203 fichiers, 434,8 Mo au premier envoi. À la mise en
service de la 1.51 : retirer `MARGE_REMPLACES`, construire et livrer, premier envoi avec l'accord de Seb.
**S4 livré en 0.223.0 (23/11)** : Firehouse a annoncé la 1.51 en service (3.316.0) ; formes comparées au code, aucun
écart ; la marge du double compte (`MARGE_REMPLACES`) est retirée (calcul de place prudent : la place rendue par un
fichier remplacé n'est pas comptée, le 507 chiffré tranche). Le premier envoi réel (environ 435 Mo, 203 fichiers,
3 jeux) se fait à la main, avec l'accord de Seb (son seuil de 200 Mo remet toute sauvegarde automatique plus grosse à
plus tard).
**0.223.1 (24/11)** : premier envoi réel fait par Seb (415 fichiers, 463,9 Mo ; place 464,2 Mo), mais la 2e
sauvegarde disait « 415 envoyés, 0 à jour » (0 octet parti). Essai réel en lecture seule (accord de Seb,
`essai_manifeste_s4`) : Firehouse compresse le manifeste même sans `Accept-Encoding`, et le canal « fichiers » de
Frogtend (`no_gzip`) le lisait illisible → manifeste vide. Corrigé : toute réponse brute annoncée gzip est décompressée
(`firehouse::degonfler`, plafond 256 Mo), les documents de la sauvegarde se lisent par le canal JSON, un document
illisible est une erreur (jamais un manifeste vide en silence). La place libre (`lire_place`, petite réponse non compressée) était, elle, bien lue ;
elle profite aussi de la correction si la liste grossit. Besoin 58 (compression non demandée) transmis à Firehouse.
**S4 validé par Seb (24/11, 0.223.1)** : les 5 étapes de A-TESTER cochées ; journal : 2e sauvegarde 0 envoyé / 415
inchangés ; après une partie de Cyberpunk, 5 fichiers envoyés (1,6 Mo), 413 inchangés, aucun sauté. Prochain : S5 (restauration).
**Lot S5 — décisions de Seb (24/11, après l'avis des trois experts)** : (1) un CONFLIT (partie de ce PC différente de
celle de Firehouse) est TOUJOURS une question, jamais un choix par les dates : fenêtre maison à deux colonnes (« Sur ce
PC » / « Chez Firehouse » : date, taille, ⭐ la plus récente ; aucun chemin), boutons « Garder celle de ce PC » (focus
d'office), « Reprendre celle de Firehouse » (celle du PC mise à l'abri d'abord, abri relu) et « Décider plus tard » ;
groupé au-delà de 3 ; (2) sur un PC neuf / après un reformatage, ce qui n'existe pas encore sur ce PC est reposé
D'OFFICE après un oui global annoncé (jeux, Mo), avec un bilan (posé / en attente : jeu non installé, identifiant non
retrouvé) ; (3) le nuage Steam (`nuage-steam/`) n'est PAS restauré : Steam s'en charge (gardé chez Firehouse en
secours) ; (4) Taodbox : « 📥 Reprendre mes parties de Firehouse » sur un jeu (BlocParties, mode canapé) et la fenêtre
de conflit à la manette ; la restauration complète reste au bureau. Règles des experts retenues : le SEL se relit chez
Firehouse (`_profil/abris-sel.json`) AVANT tout abri sur ce PC (un 404 = sel inconnu, jamais « on en crée un » qui
écraserait l'ancien chez Firehouse) ; rangé dans `profils/<id>/abris-sel.txt` seulement s'il manque (un sel local
différent n'est jamais écrasé : écart dit) ; jamais journalisé ni dans une erreur ; un sel neuf n'est créé QUE sur un 404 `document absent` (hors ligne, 401 ou
réponse illisible : aucun sel créé, les emplacements à identifiant attendent — règle des experts, relecture S5) ; pose
par `poser_absents` (mêmes règles que `reposer_sauf` sans ses restes d'anciennes restaurations, copie relue ; SHA1 de
chaque fichier vérifié, un emplacement incomplet reste en attente), jamais `reposer_parties_du_jeu` ; identifiant (`<identifiant …>`) retrouvé par `empreinte_identifiant(nom, sel)` parmi
les dossiers qui existent, sinon en attente (ou choix À LA MAIN), jamais deviné ; jeu non installé : en attente, posé
à la fin de son installation (à côté de `partie.rs:924`) ; jeu en cours : refusé ; emplacement `partiel` dit ; jeux
invisibles du profil filtrés, journal = id et nombres ; restaurer UN jeu = filtre du manifeste `jeux/<id>/` (manifeste
par jeu `manifeste-jeu-<id>.json`), jamais tout télécharger ; plusieurs PC du profil : la personne choisit le PC
(`derniere.date` pour aider) ; défaut de reprise corrigé (`.part-frogtend` AJOUTÉ au nom, plus `with_extension`).
**S5 codé le 24/11 (0.224.0)** : module `reprise_parties.rs` (pose par `poser_absents`, SHA1 de chaque fichier,
fiche téléchargée en DERNIER, remplacement défait s'il échoue au milieu, abri `avant_restauration` puis `restauree`,
la sauvegarde n'envoie jamais un `avant_restauration` plus récent qu'un remplacement), sel relu chez Firehouse avant
tout abri (créé seulement sur 404 `document absent`), fenêtre de conflit maison (`X-Modifie` du contrat 1.52, sinon
`derniere.date`), bouton au bureau et au canapé, pose en fin d'installation, choix du PC au démarrage ; relu par les
trois experts (1 bloquant — reprise coupée posée incomplète — et 8 importants corrigés). Non fait : choix À LA MAIN de
l'identifiant ; parties rangées dans le dossier du jeu hors `emplacements/` (reprise d'un jeu) ; restauration complète
non essayée sur un vrai PC neuf.
**S5 validé par Seb (24/11, 0.224.0)** : reprise d'un jeu (Cyberpunk, 196 fichiers, 255,9 Mo) et fenêtre de conflit
essayées ; l'étape Taodbox à la manette est reportée à la prochaine session canapé. Prochain : S6 (règles par moteur).
**Lot S6 — décisions de Seb (24/11, après l'avis de l'expert lancement et la recherche `docs/MOTEURS-PARTIES.md`)** :
(1) quand Firehouse ne connaît pas l'emplacement et que Frogtend reconnaît le moteur avec DEUX preuves (un fichier du
moteur au premier niveau du dossier du jeu + le nom lu dans UN fichier du jeu, lecture plafonnée, jamais d'exécution
ni de « dé-pickle »), la règle s'applique D'OFFICE et la fiche l'affiche (« 📁 d'après le moteur Unity ») ; sans les
deux preuves, rien ; (2) Firehouse dit « aucune partie » (`Vide`) mais le moteur est prouvé et des parties existent :
le MOTEUR l'emporte, le désaccord est noté et transmis ; (3) les parties rangées DANS le dossier du jeu (RPG Maker
XP/VX/Ace/MV/MZ, Ren'Py `game/saves`) : ABRI ET REPRISE dès S6 (la reprise S5 ne posait que `emplacements/` hors du
dossier du jeu) ; (4) le moteur reconnu est REMONTÉ à Firehouse : numéro du jeu, moteur et nom lu seulement, jamais un
chemin (besoin 60). Règles retenues : les RÈGLES viennent de Firehouse (contrat 1.48 : `moteur`, `emplacements_moteur`
{moteur_nom, sauvegardes, nom_a_lire, certitude}, `/sauvegardes/references ▸ moteurs` et leurs fichiers de
reconnaissance) — Frogtend ne recopie aucune règle en dur, il reconnaît et LIT le nom ; ordre : `personne` > Firehouse
`Lu` > moteur prouvé ; doublons retirés ; jamais le registre (PlayerPrefs Unity), jamais un disque parcouru, jamais un
dossier commun seul (`%APPDATA%\RenPy`, `LocalLow\<société>`, `%APPDATA%\RenPy	okens` exclus) ; valeur lue refusée si
vide, avec `\`, `..`, caractère interdit ou racine ; journal = id du jeu, jamais un nom lu ; corriger `retirer_si`
(emplacements_parties.rs ~485) qui garde les emplacements non-Firehouse d'un jeu devenu invisible ; test « une partie
de jeu admin n'apparaît jamais pour un mineur » (cache par profil, `retirer_si`, abri auto, Mes abris, envoi, reprise,
Taodbox).
Précision (24/11, relecture S6) : pour un gabarit SANS nom à lire, rangé DANS le dossier du jeu (RPG Maker, Wolf RPG,
Tyrano, `game\saves` de Ren'Py), la seconde preuve est le chemin lui-même (aucun nom à lire, aucun risque de toucher un
autre jeu) : le marqueur suffit. Essai réel du 24/11 (lecture seule, jeu Ren'Py de Seb installé depuis une archive) :
reconnaissance juste et `options.rpy` lu sur le dossier de l'EXÉCUTABLE, mais « aucun » sur `installation.dossier`
→ `<dossier du jeu>` = dossier du lanceur quand il est dans le dossier d'installation (corrigé dans S6, vaut aussi pour
S1). Refus ajoutés : Unity « DefaultCompany / My project », tout chemin moteur déjà pris par un autre jeu du profil,
journaux Unity (`*.log`) non comptés comme parties.
**S6 codé le 24/11 (0.225.0)** : module `moteurs_parties.rs` (règles des références de Firehouse, reconnaissance au
premier niveau avec budget par règle, lecteurs Ren'Py `options.rpy`/`.rpyc` sans dé-pickle avec budget global 64 Mo,
Unity `app.info`, Unreal dossier voisin d'`Engine`), ordre personne > Firehouse > moteur, désaccords notés (`Vide`
renversé seulement par de vraies parties, journaux exclus ; `moteur_contredit`) pour la remontée de la 1.53 (besoin 60),
`retirer_si` corrigé (jeu invisible : tout part sauf `personne`), test « admin jamais vu par un mineur » sur tous les
parcours ; `<dossier du jeu>` = dossier d'installation, descendu à travers les ENVELOPPES d'archive (aucun fichier, un
seul sous-dossier, 3 niveaux, contrôlé par le lanceur) — relevé réel : 1 jeu sur 35 change (le jeu Ren'Py), aucun jeu
de boutique ; relu par les experts lancement et contrat (aucun bloquant, 6 importants corrigés). Non fait : Godot et
GameMaker reconnus mais non appliqués (nom illisible sans risque), archives `.rpa` de Ren'Py, stockage NW.js de RPG
Maker MV/MZ, remontée à Firehouse (1.53).
**S6 validé par Seb (24/11, 0.225.0)** : What a Legend! (Ren'Py, archive enveloppée) — deux endroits d'après le
moteur, abri automatique après une partie (8 fichiers), envoi chez Firehouse par la sauvegarde d'après partie,
Cyberpunk inchangé. **0.225.1** : remontée du moteur (contrat 1.53, `POST /jeu/{id}/moteur`, une fois par (jeu, moteur,
nom lu), `nom_lu` omis sans nom, `desaccord` seulement sur `Vide`, jamais un nom de forme chemin, 400/404 jamais
rejoués, hors ligne/5xx réessayés à l'ouverture suivante). Prochain : S7 (rattrapage des parties jouées hors de Frogtend).
**Lot S7 — décisions de Seb (24/11, après l'avis de l'expert lancement)** : (1) le rattrapage des parties jouées HORS
de Frogtend (jeu lancé depuis Steam, Epic, un raccourci…) se fait SEULEMENT AU DÉMARRAGE de Frogtend (ouverture du
profil) ; (2) propriétaire : un seul profil Frogtend sur ce compte Windows → l'abri rattrapé part d'office ; plusieurs →
mis à l'abri tout de suite au nom du profil ouvert mais marqué « à confirmer », envoyé chez Firehouse seulement après
« C'est toi qui as joué à … ? » (fenêtre maison) ; (3) un court message « 🛟 N jeux rattrapés » et, sur la fiche,
« Dernier abri : …, rattrapé » ; rien si rien de neuf ; (4) un interrupteur à part « 🛟 Rattraper les parties jouées
hors de Frogtend », ALLUMÉ d'office, dans Options ▸ Sauvegarde et les réglages de Taodbox. Règles de l'expert retenues :
« du neuf » = relevé dates/tailles (jamais un SHA en boucle) comparé au dernier abri du profil, SHA seulement pour un jeu
qui a changé ; jeux lus depuis le cache des emplacements du profil, jamais de question à Firehouse pendant le
rattrapage ; un seul relevé des processus par tour ; plafond de jeux par tour ; jeu qui tourne ou écrit → « abri à
faire », jamais de copie ; calme et refus d'`abriter_auto` réutilisés ; déclencheur `rattrapage` ; OneDrive jamais
téléchargé, plus de 2 Go jamais copié sans question ; test « jamais vu par un mineur » étendu ; journal = id et nombres.
**S7 codé le 24/11 (0.226.0)** : tour de rattrapage à l'ouverture du profil (jeux visibles du cache des emplacements,
aucune question à Firehouse, un seul relevé des processus, dossier de boutique contrôlé pour un jeu non installé par
Frogtend), « du neuf » = relevé dates/tailles comparé au dernier abri de CHAQUE profil (date gardée par la copie,
vérifiée), SHA seulement pour un jeu qui a bougé, signature du relevé notée, plafond 50 avec départ tournant,
déclencheur `rattrapage`, `a_confirmer` / `pas_a_moi` jamais envoyés, question « C'est toi ? » jamais pendant une
partie, interrupteur `parties.rattrapage` au bureau et dans Taodbox ; relu par les experts lancement et interface
(1 bloquant — question en pleine partie — et 8 importants corrigés). Limite acceptée et testée : même taille, date à 2 s
près, autre contenu = non vu.
**Demande de Seb (22/11, après « Ajouter un compte GOG ») : une FENÊTRE D'ATTENTE pendant la lecture d'un compte.**
Après la connexion, la lecture prend du temps et le seul signe d'activité est le bouton « Connexion… » ; le message
« Connecte-toi… dans la fenêtre qui s'ouvre » laisse croire que rien ne se passe ou que c'est raté. Voulu : une petite
fenêtre maison BLOQUANTE (« ⏳ Lecture de tes jeux GOG… »), avec un bouton « Annuler » qui arrête vraiment l'opération,
pour l'ajout d'un compte et la mise à jour à la main. Intégré au nouveau C4c.

**✅ Décision de Seb (25/11) : S7 MIS DE CÔTÉ, on attaque le MODDING DES JEUX PC, avec Cyberpunk 2077 comme cobaye.** S7 : étape 1 validée (interrupteur allumé), étapes 2–3 à reprendre plus tard (A-TESTER). Déclencheur : la mise à jour de Cyberpunk (GOG, 2.50, exe 3.0.88) l'empêche de démarrer à cause de mods périmés (relevé en lecture seule le 25/11 : CET 1.35 « Only 2.21 is supported », RED4ext 1.27 plante en chargeant ArchiveXL). 284 mods installés par VORTEX (`vortex.deployment.json`, liens durs, dépôt `D:\LaunchBox\Modding\Vortex Mods\Cyberpunk 2077`, noms de dossier avec l'id Nexus et la version). Existant dans Frogtend : liens `nxm://` + API Nexus par clé personnelle (`nexus.rs`), pose avec manifeste et désinstallation avec preuve (`mods.rs`).

**✅ Décisions de Seb (25/11) sur le modding PC** :
- **Repartir de zéro** : Frogtend ne gère que les mods qu'il installe lui-même ; les mods posés par Vortex, **c'est Frogtend qui les retire** (avec copie et preuve), à la place de Seb ; puis Frogtend réinstalle, à jour, ceux que la personne choisit.
- **Premier lot** : repérer les mods périmés, proposer de les couper pour jouer tout de suite, puis les mettre à jour depuis Nexus.
- **Nexus Premium** : Seb a un compte Premium (téléchargement direct par l'API, sans clic sur le site, pour son profil ; les comptes gratuits gardent la voie `nxm://`).
- **Le savoir** (quel mod va avec quelle version du jeu, dépendances) : **Nexus d'abord** ; en cas de doute, **Firehouse se renseigne et l'enregistre** pour les fois suivantes (besoin d'API à écrire).
- **La clé Nexus vient de Firehouse** (Seb, 25/11) : c'est Firehouse qui la fournit à chaque Frogtend de la famille (interface dans Firehouse pour la saisir) ; **la clé Premium de Seb sert à TOUT LE FOYER**, en connaissance de cause (Nexus : « NEVER share your personal API keys with any other user », règles d'usage de l'API : risque de blocage ; Seb : « on est le même foyer », deux personnes au plus s'en serviront). **Forme B retenue** (confirmée par Seb à Firehouse, 25/11) : la clé se saisit dans Firehouse et n'en sort JAMAIS ; Firehouse relaie l'API de Nexus (fichiers, mises à jour, md5_search, prérequis GraphQL v2) et rend le `download_link` signé, que Frogtend télécharge sans clé (besoin 61). La saisie par profil dans Frogtend reste un repli tant que la route n'existe pas.
- **Vortex** (Seb, 25/11) : après le retrait de ses mods par Frogtend, **Seb désinstalle Vortex lui-même** ; son dossier `D:\LaunchBox\Modding\Vortex Mods` n'est jamais touché par Frogtend. Raison (relevé de l'expert lancement dans le code de Vortex 1.13.7) : sinon il reposerait les 284 mods, ou sa fenêtre « External Changes » proposerait « Save », qui effacerait son dépôt.
- **Rangement des fichiers retirés DANS le jeu** (Seb, 25/11) : `<jeu>\Frogtend\vortex\…`, par renommage (même disque : instantané, 0 octet copié, lien dur gardé = preuve), réversible.

**Lot M — modding des jeux PC (✅ découpage validé par Seb le 25/11, M1 lancé)** :
- **M1 (sans Nexus)** : (a) les jeux GOG acceptés pour les mods (aujourd'hui Steam seul : `jeu_pour_un_mod`, `garde_mods`), jeu ET GOG Galaxy au repos vérifiés avant d'écrire ; (b) **retrait des mods de Vortex** : preuve par fichier (listé dans `vortex.deployment.json`, chemins vérifiés, même volume + même index de fichier que le dépôt, ≥ 2 liens, pas de point de réanalyse), DÉPLACÉ sous `<jeu>\Frogtend\vortex\<date>\` (jamais effacé), manifeste du retrait écrit avant, reprise après coupure ; lien rompu (`options.json`) jamais touché, `.vortex_backup` jamais remis ; marqueur retiré seulement dans un dossier vide ; données des mods hors manifeste (configs CET, AMM, sqlite) laissées en place ; `vortex.deployment.json` déplacé en dernier ; opération annoncée et chiffrée (2015 fichiers, 4,1 Go par renommage, 284 mods) ; (c) **recoller** par mod (`source`), réversible ; (d) **abri des parties avant** (déclencheur `avant_mod`, aussi pour un jeu GOG) et avertissement « parties faites avec des mods » ; (e) **repérer les mods périmés** : version du jeu (journal RED4ext « Product version », exe), refus de CET, greffon RED4ext qui n'a jamais fini de charger, changement de version entre deux journaux ; (f) un mod Frogtend ne s'installe jamais par-dessus un fichier encore lié à Vortex ; (g) couper/recoller aussi dans Taodbox ; (h) journal : ids et nombres seulement (corriger aussi `nexus_installer`).
- **M1 codé le 25/11 (0.227.0)** : `mods_vortex.rs` (preuve par fichier, rangement par renommage sous
  `<jeu>\Frogtend\vortex\<secondes>\`, `retrait.json` + copie dans la garde du jeu, reprise, remise par mod ou totale
  sur tous les retraits, marqueurs vides seulement, `.vortex_backup` jamais remis), jeux GOG acceptés pour les mods
  (`gog_pret_a_ecrire`, verrous Galaxy et Vortex ouverts), diagnostic REDengine (3 derniers journaux, ignorés s'ils
  précèdent le retrait), `BlocModsVortex.svelte` au bureau et dans Taodbox. Revues des experts lancement et UI : 2
  bloquants + 10 importants corrigés. Essai réel en LECTURE SEULE (accord de Seb) : 2 014 fichiers prouvés, 1 laissé
  (`options.json`, lien rompu), 284 mods, 4 110 194 748 octets, bon diagnostic. Aucun retrait réel avant l'essai de Seb.
  **Essai réel de Seb (25/11)** : retrait fait (2 014 fichiers rangés, liens gardés, chargeurs partis, 368 dossiers vides retirés, 81 marqueurs gardés avec les réglages des mods, dépôt Vortex intact, vérifié sur le disque) ; Cyberpunk 2.50 démarre sans mods. **À corriger** : pendant le retrait, la barre de progression est restée vide, sans « N fichiers sur 2 014 ».
  Étape 4 (remettre « (4k) Immersive Bullet Holes ») : remis, vérifié sur le disque. **Retours de Seb à corriger** : (1) un mod remis DISPARAÎT de la liste et n'apparaît nulle part : il faut une partie « 🧩 Dans le jeu » (mods remis, puis ceux installés par Frogtend en M2) ; (2) les boutons grisés quand Galaxy est ouvert ne disent pas pourquoi à hauteur de la liste ; (3) Frogtend lance les jeux GOG PAR Galaxy (C4c pas fait), donc Galaxy reste ouvert après une partie et bloque le bloc Mods : avancer C4c, ou proposer « Fermer GOG Galaxy » avec accord ; (4) « je ne me souviens pas avoir installé 284 mods » : beaucoup sont des pièces (prérequis comme ArchiveXL/Codeware/TweakXL, patchs de compatibilité, variantes, traductions) ; Vortex ne garde pas qui a été choisi à la main.
  **✅ Décisions de Seb (25/11)** : (a) une **0.227.1 d'abord** (partie « 🧩 Dans le jeu », compteur du retrait, motif du verrou près des boutons) ; (b) pour Galaxy laissé ouvert : **avancer C4c** (lancement direct des jeux GOG, sans Galaxy) ; ordre : 0.227.1, puis C4c, puis M2.
  **0.227.1 codée le 25/11** : partie « 🧩 Dans le jeu » (re-ranger un mod remis, preuve par l'identité notée, sans
  dépôt ni 2 liens ; « 🧹 Tout ranger de nouveau »), progression dès la vérification, motif du verrou près des listes,
  messages vrais. Revues lancement + UI : 0 bloquant ; corrigés : garde-fou `installer_dans` contre un fichier remis,
  `retirer_du_pc` refusé si des mods sont rangés dans le jeu, recalage écrit après coupure, état `Parti`, total global
  de la progression. Cause de la barre vide : rien n'était émis pendant l'abri et la preuve ; l'absence d'affichage
  pendant les 5 s de rangement reste NON expliquée (à regarder au prochain essai). Essai réel en lecture seule après
  le retrait de Seb : état lu sans rien écrire (1 mod remis, 283 retirés, 0 posé, plus d'alerte).
  **✅ M1 VALIDÉ par Seb le 25/11** (0.227.0 étapes 1–4, 0.227.1 étapes 1–3 ; étape 6, Taodbox, à la prochaine séance canapé).
  **25/11, Seb** : Vortex désinstallé ET son dépôt « Vortex Mods » supprimé (« au moins on pourra vérifier que tout vient bien de Firehouse »). Constat : les 2 013 fichiers rangés ont encore 2 liens (dépôt sans doute dans la Corbeille) ; le dossier du retrait dans le jeu devient la SEULE copie des mods de Vortex (3,98 Go) : jamais effacé. « VortexDownload » (les archives) existe toujours : utile en M2 pour reconnaître les mods par md5. Remise et re-retrait ne doivent jamais exiger le dépôt (0.227.1). **Vision de Seb** : à la fin, un grand ménage pour ne plus dépendre de l'ancien système (LaunchBox / Vortex) : tout vient de Firehouse et passe par Frogtend.
  **✅ Précisé par Seb (25/11) — le « grand ménage » (lot à venir, après M2, à découper avec lui)** : plus de LaunchBox comme système, plus de Vortex installé ; les mods ET les outils (Cheat Engine, Special K, ReShade, etc.) sont **gérés par Firehouse** et installés **proprement dans des dossiers de Frogtend** (règle « pas de pieuvre » : dossiers réglés, jamais éparpillés). Point de départ : ce qui existe déjà sous `D:\LaunchBox\Modding` (Cheat Engine 7.5, Special K, ReShade 6.3/6.4, trainers, KHPCPatchManager, RE2Mods…) — inventaire chiffré et accord avant tout retrait. Cheat Engine : déjà servi par Firehouse (lot 8, 02/10).
- **✅ Seb (25/11), après l'essai de la 0.227.1** : « tout ce texte qui parle de retiré / remettre, c'est étrange » : le bloc Vortex est un DÉMÉNAGEMENT unique (mods posés hors de Frogtend, ce qui ne doit plus arriver). À partir de M2, le bloc « 🧩 Mods » montre les mods GÉRÉS PAR FROGTEND (Firehouse/Nexus) ; la partie Vortex **disparaît de la fiche une fois le déménagement fini** (mods reconnus et réinstallés depuis Firehouse) ; la copie rangée reste sur le disque tant que Seb ne demande pas de l'effacer. Le long bilan du retrait ne doit pas rester affiché en permanence.
- **M2 (relais Nexus de Firehouse, besoin 61 A)** : identifier les archives de Vortex (`md5_search`), proposer les mises à jour (prérequis d'abord), télécharger en Premium sans clic, réinstaller ; .rar (29 archives) et archives > 2 Go à traiter.
- **M3 (savoir de Firehouse, besoin 61 B)** : compatibilité par version du jeu affichée (« compatibilité inconnue » jamais « compatible »).


### Prochains lots (décisions de Seb, 02/10)

**✅ Ordre (Seb, 25/11)** : M1 validé → **C4c-1** (lancer les jeux GOG SANS Galaxy : tâche « game » du goggame-<id>.info ; à défaut, la tâche principale FileTask du dossier, comme Heroic et Playnite, pour les jeux Unreal à petit programme de démarrage — décision de Seb 25/11 ; Galaxy en tout dernier secours) → **C4c-2** (installer les jeux GOG sans Galaxy) → **C4c-3** (fenêtre d'attente pendant la lecture d'un compte GOG) → **M2**. Avis de l'expert lancement (25/11) : suivi de partie, abri, session Firehouse marchent déjà en direct ; à vérifier en vrai : le service GalaxyCommunication réveillé par le jeu (rouvrirait le verrou des mods) ; REDmod/-modded renvoyé à M2.

**C4c-1 codé le 25/11 (0.228.0)** : `boutiques_pc::tache_jeu_gog` (goggame-<id>.info plafonné à 1 Mo ; tâche « game »
64 bits puis 32 bits, langue du jeu, principale préférée ; à défaut la principale FileTask, langue d'abord ; chemins
sûrs, jonctions refusées, arguments en chaîne seulement), `partie::plan_gog` (direct → Galaxy → registre → refus, jamais
Galaxy après un échec direct), pid suivi directement, `Suivi::nouveau` prend l'heure notée d'une racine fermée, 740 →
message vers Galaxy (aucune élévation), secours Galaxy dit AVANT l'attente (bureau / canapé), contrepartie honnête sur
la fiche (`gog_lancement_direct`). Revue de l'expert lancement : 0 bloquant, 5 importants corrigés. Incident : un test
du codeur a ouvert 2 fenêtres d'erreur Windows chez Seb (`cmd /C start` mal formé) — test réécrit sans processus ;
leçon : jamais de `start`, ShellExecute ni explorer dans un test.
**✅ C4c-1 VALIDÉ par Seb le 26/11** (étapes 1–4 : Cyberpunk lancé par sa tâche game, 175 s, 203 fichiers à l'abri et envoyés ; HZD lancé directement, 86 s, 5 fichiers à l'abri ; aucun processus ni service Galaxy ; étape 5 Taodbox à la prochaine séance canapé). Remarque de Seb : le menu de ReShade s'affiche au lancement de HZD (ReShade posé à la main dans le dossier du jeu) — à recenser dans l'inventaire du « grand ménage » (outils gérés par Firehouse).
**✅ Lot « Serveurs Firehouse » (Seb, 26/11, relayé par Firehouse ; ordre choisi par Seb : APRÈS MK8 (1.55) et C4c-2, avant M2 ; et « avant le transmedia »)** : « nos serveurs directement intégrés dans Frogtend, une partie Serveurs Firehouse, un peu la même interface que le cockpit, par user les mêmes choses (arrêt, démarrage, demande de démarrage, et pour un user simple juste installer / lancer le jeu) ». Serveurs : Minecraft (Hikari, Fabric) et Palworld. Droits (Seb, 17/11, ceux du cockpit, jamais plus) : TOUS voient serveurs, état, NOMBRE de joueurs (noms = admin) et DEMANDENT un démarrage ; avancé/admin valident ou refusent (et démarrent aussitôt s'ils demandent) ; seuls les admins démarrent/arrêtent/redémarrent et règlent. Partage proposé par Firehouse : Firehouse = contrat 1.56 sous /api/jeux/v1/serveurs (liste + droits du jeton, demander/valider/refuser/annuler, piloter, « recette » client par serveur) ; session Minecraft = contenu du pack client ; Frogtend = la partie Serveurs (cartes, boutons selon droits, suivi des demandes, Installer/Lancer selon la recette).
**Précisions de Firehouse (26/11, avant le contrat 1.56)** : recette = `actions` en liste FERMÉE (installer_jeu, poser_fichiers à destination relative, lanceur, lancer, executer_installeur en dernier recours), avec `revision` (empreinte du contenu servi) et `adresses` {maison, exterieur} (aujourd'hui les mêmes noms, résolus vers Louis par le DNS local). Minecraft, 1er pas = [executer_installeur, lancer] : Hikari-Setup.exe (sha256 cd9d346d…2428, 30 334 034 o, version 20261124-1737, admin: false, /VERYSILENT /SUPPRESSMSGBOXES /NORESTART à confirmer à l'essai, signature: null) pose Prism Launcher 11.1.1 portable + instance Hikari (MC 26.3, Fabric 0.20.1) dans %LOCALAPPDATA%\HikariNoSekai ; le pack se met à jour par Prism à chaque lancement (adresse du pack jamais servie) ; lancer = prismlauncher.exe --launch Hikari --server <adresse> ; 1er lancement : connexion Microsoft et Java par Prism. Palworld = [installer_jeu (boutique), lancer + adresse:port]. Mineurs : mêmes cartes, sans noms des joueurs, sans piloter. **Question à poser à Seb au début du lot** : Prism s'installe dans %LOCALAPPDATA%\HikariNoSekai, hors des dossiers de Frogtend (règle « pas de pieuvre ») — accepté tel quel, ou dossier réglé par Frogtend (si l'installeur accepte /DIR=) ? **✅ Réponse de Seb (26/11)** : « si possible on garde tout dans Frogtend, sinon on garde LOCALAPPDATA » → demander à Firehouse/Minecraft si Hikari-Setup.exe accepte /DIR= (et si Prism portable reste juste ailleurs) ; sinon %LOCALAPPDATA%\HikariNoSekai.
**✅ Seb (26/11) — installations Hikari DÉJÀ PRÉSENTES (chez ses nièces, par exemple)** : Frogtend doit les DÉTECTER (dossier enregistré par l'installeur Inno, prismlauncher.exe, instance « Hikari ») et, si possible, les DÉPLACER dans le dossier de Frogtend, sans rien casser (monde, captures, réglages, connexion Microsoft de Prism, Java) et de façon TRANSPARENTE pour les utilisateurs : mettre à jour aussi le RACCOURCI DU BUREAU (et du menu Démarrer) et l'enregistrement de l'installeur (désinstallation Windows). Aucun déplacement sans l'accord de la personne, avec taille et preuve ; sinon, laisser en place et lancer depuis là. À étudier au début du lot Serveurs (Prism portable déplaçable ? chemins absolus dans prismlauncher.cfg / instance ?).

**✅ Priorité (Seb, 26/11)** : l'essai des MUSIQUES de Mario Kart 8 Deluxe (contrat 1.55 de Firehouse : amorces + BGM.bars fabriqué par Firehouse, posé à romfs/Audio/Static/BGM.bars dans UN dossier de mod commun « !Firehouse - musiques » (le premier dossier dans l'ordre des noms gagne chez Ryubing et Eden : code ouvert relu le 26/11) ; 1er mod DK Summit gamebanana-ee3c0683c3f6 ; version servie 3.0.1 = 1179648) passe AVANT la suite de C4c-2 ; le relevé GOG en lecture seule (Another World, accord de Seb) se finit d'abord. Texte complet du 1.55 reçu et recopié (CONTRAT § 1.55). **✅ Décisions de Seb (26/11)** : les 41 Mo du BGM.bars sont ANNONCÉS dans la fenêtre d'accord (une refabrication due à une nouvelle version servie suit le réglage « mettre à jour les mods tout seul ») ; si la version utilisée n'est pas EXACTEMENT celle du fichier (X-Version-Jeu), PAS de musique de mod, la fiche dit pourquoi. Avis de l'expert lancement (26/11) : refabriquer au lancement (HEAD 5 s, X-Sha256), précharger à l'allumage dans un cache ; hors ligne, garder si l'ensemble n'a pas changé, sinon retirer ; refuser un BGM.bars venu du dossier d'un mod ; accepter une archive vide quand amorces n'est pas vide ; profils et mods réservés jamais mêlés ; abri des parties quand le BGM change.
**1.55 codé le 26/11 (0.229.0)** : amorces lues ; dossier commun « !Firehouse - musiques » (Ryubing par profil,
Eden commun, note .frogtend-musiques.json, cache <stock>\.musiques\BGM-<sha12>.bars, 3 gardés) ; règle de version
stricte ; HEAD 5 s + X-Sha256 ; X-Code ; ACCORD noté à part (.musiques\accord.json : oui à l'allumage, au
téléchargement, ou bouton « 🎵 Préparer les musiques ») — jamais de téléchargement sans accord ni « tout seul » ;
budget de 30 s au lancement ; BGM.bars d'un mod de musique jamais posé ; pose atomique avec sha attendu noté ; abri
des parties quand le BGM change ; journal en nombres. Revues lancement + UI : 3 bloquants (téléchargements non
annoncés ×2, BGM.bars de mod) + 9 importants corrigés.
**✅ 1.55 VALIDÉ par Seb le 26/11 dans Eden** (après 0.229.1 et 0.229.2) : DK Summit à la place de GCN Royaume Sorbet avec SA musique, dernier tour, menus et résultats sans plantage ; fichier commun posé (empreinte 34f63d9a… juste, version 3.0.1 confirmée par le build id dans le journal d'Eden), puis RETIRÉ quand le mod est éteint (dossier vide chez Eden, musique d'origine revenue). Reste : la fiche « 🎵 Musiques de 1 mod » (pas regardée), Ryubing et le canapé (plus tard).
**✅ Seb (26/11)** : les mods de musique de MK8D sont ouverts à toute la famille (« tout est ouvert à la famille sauf le NSFW »). **Suite choisie par Seb** : la FENÊTRE DÉDIÉE aux mods d'un jeu (recherche, classement par type ; IDEES « fenêtre dédiée »), puis C4c-2.
**✅ Décisions de Seb (26/11) pour la page des mods** : la bande de droite ne montre TOUJOURS qu'un résumé et le bouton « 🧩 Ouvrir les mods (N) », même pour un jeu à 2 mods ; « Tout télécharger » ne vaut que pour l'ONGLET ou le FILTRE affiché, seuls les ⭐ recommandés cochés d'office, taille totale dite. Avis de l'expert UI : une vraie page commune PageMods.svelte servie par /mods?jeu= (bureau) et /taodbox/mods?jeu= (canapé), modèle PageSucces ; logique de BlocMods sortie dans une classe partagée modsSwitch.svelte.ts ; onglets par type (technique, contenu, apparence, triche, autre, 🎵 musiques, ⭐ recommandés, pas encore décrit) avec LB/RB au canapé ; filtres d'état ; recherche au bureau seulement ; paquets de 50 (bureau) / 25 (canapé), pas de virtualisation ; détail en colonne (bureau) ou ligne dépliée (canapé) ; flèches haut/bas au bureau ; focus rendu sur la liste filtrée ; jamais sha, clé, révision ni amorces à l'écran.
**Page des mods codée le 26/11 (0.230.0)** : classe partagée modsSwitch.svelte.ts (logique de l'ancien BlocMods, remise
à zéro par profil, une seule mise à jour auto, rien n'est écrit si le jeu ou le profil a changé pendant une fenêtre),
PageMods.svelte (/mods et /taodbox/mods), bande réduite au résumé, « 🧩 Mods (N) » dans « Gérer le jeu » sans attente,
« Tout cocher / Tout décocher » dans la fenêtre maison cocher. Revue UI : 0 bloquant, 4 importants + mineurs corrigés.

**0.229.2 (26/11)** : version d'une mise à jour EMBARQUÉE dans un XCI (SuperXCI MK8D 3.0.1) lue dans le `.cnmt` du
NCA Meta (clés de zone de prod.keys, AES-CTR, lectures bornées), règles d'Eden v0.2.1 (PACKED / fournisseur manuel,
NAND et SD, coupures « Update », « Update (NAND) », « Update (SDMC) », « Update@v » : un seul chemin coupé → inconnue),
cache par fichier, Ryubing lu dans les méta-contenus quand `selected` vise le .xci, `version_du_nom` refuse « v3.01 ».
Essai réel en lecture seule : Eden, version 1179648. Revue de l'expert lancement : 0 bloquant, 6 importants corrigés.

**✅ Décisions de Seb pour C4c-2 (26/11)** : voie (a) = l'INSTALLEUR HORS LIGNE OFFICIEL (setup_*.exe Inno + .bin), « comme GOG » : droits administrateur ANNONCÉS dans la fenêtre d'accord, écritures normales de l'installeur acceptées (registre GOG.com\Games, menu Démarrer, désinstallation par Windows) ; installeur EFFACÉ après une installation vérifiée (retéléchargé pour réinstaller) ; langue : FRANÇAIS sinon anglais, dit dans l'accord ; DLC possédés PROPOSÉS et cochés d'office dans la même fenêtre (décochables). Voie (b) content system : notée pour plus tard (mises à jour différentielles). **Relevé RÉEL du 26/11 (accord de Seb, lecture seule, Another World 20th, compte « Profil », essai_gog_installeur)** : jeton renouvelé et rangé au coffre, expires_in 3600 ; products?expand=downloads SANS jeton → 200, 3 installeurs (Windows : seulement « en » pour ce jeu → le repli anglais sert), clés de l'installeur {id, language, language_full, name, os, total_size, version, files[{id, size, downlink}]}, bonus_content 17, patches 0, dlcs [] ; downlink (jeton Bearer, api.gog.com, sans redirection) → 200 {downlink, checksum}, tous deux sur gog-cdn-fastly.gog.com avec un JETON DANS LE CHEMIN (secret) ; XML de contrôle <file name md5 chunks timestamp total_size available notavailablemsg> + 12 <chunk id from to method=md5> de 10 Mo ; ⚠ total_size du XML = 123 571 824 ≠ size/total_size de products = 122 683 392 (arrondi) : la taille EXACTE vient du XML ou du Content-Range, jamais de products ; CDN : Range 0-0 → 206, Accept-Ranges bytes, Content-Range bytes 0-0/123571824, ETag = md5 du fichier, pas de Content-Disposition (nom pris dans le XML) ; validité du lien « nva » ≈ 24 h. Avant de coder : relevé RÉEL en lecture seule sur un petit jeu (products?expand=downloads, downlink, XML de contrôle, en-têtes du CDN, sans télécharger), avec l'accord de Seb. Avis des experts (26/11) : réutiliser jeton_acces_gog (renouvellement sous le verrou COMPTES_GOG), un téléchargeur GOG sur le modèle de mods::telecharger_depuis (MD5 calculé pendant la réception, lien redemandé à chaque reprise, .part lié à l'installeur/version), jeton seulement vers api.gog.com, client CDN sans jeton, lien signé jamais journalisé, signature Authenticode « GOG » vérifiée avant d'exécuter, marque « installé par Frogtend », abri avant mise à jour, reposer_parties_restaurees après installation.

**Ordre en cours (Seb, 22/11)** : lot **S** (S1 livré en 0.220.0-0.220.1 ; puis S2 à S7) → lot **R** (recompilations,
ci-dessous) → **C4c** (installer et lancer les jeux GOG sans Galaxy, fenêtre d'attente GOG).

**Dépôt GitHub bientôt PRIVÉ (Seb, 22/11).** ⚠ AVANT de le passer en privé : la mise à jour en un clic de Frogtend lit
`https://github.com/Nimai26/Frogtend/releases/latest/download/latest.json` (src-tauri/tauri.conf.json, `endpoints`) et
télécharge l'installeur sur les releases du même dépôt. Un dépôt privé rend ces adresses inaccessibles sans compte : plus
aucune mise à jour chez la famille (règle « mise à jour simple »). Choisir d'abord un autre point de distribution (par
exemple un dépôt PUBLIC qui ne contient que les releases, ou Firehouse qui sert les mises à jour — besoin d'API),
livrer une version qui le lit, attendre qu'elle soit installée partout, puis seulement passer le dépôt en privé.
**Décision de Seb (22/11) : c'est FIREHOUSE qui livrera les mises à jour ET le téléchargement du logiciel complet**
(premier installeur). Plus tard : écrire le besoin d'API (latest.json signé, installeur, reprise), adapter
`endpoints`, livrer, attendre que la famille soit à jour, puis dépôt privé.

**R. Les RECOMPILATIONS (« recomps ») dans Frogtend (Seb, 22/11 : « il faut à tout prix »)** — vu sur
recomplauncher.com (RecompLauncher, GPL v3, Tauri, github.com/elripalda/RecompLauncher ; détail dans IDEES.md). Une recomp
est un jeu de console reconstruit en programme PC natif (Zelda 64: Recompiled, Ship of Harkinian, Starship,
SpaghettiKart, Banjo: Recompiled…), publié par son auteur sur GitHub, qui exige la ROM ou le disque ORIGINAL. Dans
Frogtend : un « émulateur » de plus pour un jeu dont Firehouse sert la ROM (le jeu vient toujours de Firehouse ; le
programme vient du GitHub officiel de son auteur, après accord et taille annoncée) ; « ▶ Jouer en version PC
(recompilée) » sur la fiche ; vérification de la ROM (`dumps` du format `recomplauncher.json`) ; parties par le lot S
(champ `saves`) ; Taodbox par `RECOMPLAUNCHER_MODE=bigpicture` ; manette réglée dès la 1re partie (`config`) ; liste des
recomps demandée à Firehouse (besoin d'API à écrire au début du lot). **Décision de Seb (22/11) sur le code de
RecompLauncher** : usage privé et familial — on peut examiner son code et en reprendre des parties si cela fait gagner du
temps, en l'adaptant à Frogtend. ⚠ À garder en tête (dit à Seb le 22/11) : la GPL v3 n'impose rien pour un usage privé,
mais Frogtend est distribué (dépôt GitHub PUBLIC Nimai26/Frogtend, installé chez la famille) : du code GPL repris
obligerait Frogtend entier à suivre la GPL (sources publiées sous GPL, mention des auteurs). Chaque reprise est notée
(fichier d'origine, commit, auteur) pour pouvoir s'y conformer.

**D. Les DLC et contenus additionnels (Seb, 03/10)** : « la gestion de ce genre de chose, DLC et autres, doit pouvoir
être **détectée, gérée, proposée et installée si l'utilisateur est d'accord** ». Règle générale, pour toutes les
consoles (DLC, mises à jour, avatars, thèmes…), jamais d'installation sans un oui. ⚠ Précision de Seb : l'accord est
celui de **la personne qui utilise Frogtend sur SON PC** (Frogtend sera installé par d'autres, ailleurs) — jamais un
réglage décidé pour tous par Seb. Vérifié le 03/10 : les 70 .pkg de Seb ont un en-tête valide (0x7F « PKG ») et leur
content ID se lit dans les zips (ex. EP0102-**BLES01227**_00-DLCS12… = Asura's Wrath) → rattachement sûr au jeu :
- **détecter** : à l'import et dans les dossiers des systèmes, repérer les contenus additionnels et les RATTACHER à
  leur jeu (par l'identifiant du jeu quand le fichier le porte — PS3 : « content ID » EP9000-BCES01741_00-… dans
  l'en-tête du .pkg et le nom du .rap ; sinon par le titre) ;
- **gérer** : dans le panneau du jeu, la liste de ses contenus (disponibles / installés) ;
- **proposer / installer** : cases à cocher, taille annoncée, confirmation, puis installation par l'émulateur lui-même ;
- PS3 (relevé dans les sources de RPCS3, 03/10) : `rpcs3 --installpkg <fichier>` installe un .pkg (contenu commun à
  tous) ; une licence .rap se copie dans `dev_hdd0\home\<compte>\exdata\<content id>.rap` = **par compte
  RPCS3**, donc dans le compte DU PROFIL qui installe. Chez Seb : 70 zips de DLC/avatars (.pkg, 50 avec .rap).
- **✅ 0.40.0 (PS3)** (`src-tauri/src/contenus.rs`, `src/lib/contenus/`) : détection des .pkg/.rap isolés ou dans des
  zips (en-tête et sommaire seulement : 70 contenus chez Seb en 0,3 s, les 50 licences rattachées) dans le dossier du
  jeu, son installation et les emplacements de la PS3 ; rattachement par TITLE_ID (PARAM.SFO de l'.iso/du dossier) ou
  par le titre ; panneau du jeu « 📦 Contenus additionnels » (disponibles/installés pour CE profil, cases, taille,
  confirmation) ; installation : extraction temporaire dans `<données de Frogtend>\travail\contenus` (retirée
  ensuite), `rpcs3 --headless --installpkg` une seule fois par PC (registre `<rpcs3>\.frogtend-contenus.json`), licence
  copiée dans le compte RPCS3 du profil ; à l'import, un zip qui ne contient que des .pkg/.rap n'est PAS un jeu
  (dossier PS3 de Seb : 7 jeux, 70 contenus écartés et annoncés).
- **✅ 0.41.0 (Switch / Eden)** : mises à jour et DLC .nsp détectés (identifiant dans le nom « [0100…] », sinon dans
  le ticket « <rights id>.tik » de l'en-tête PFS0, sans rien déchiffrer) et classés (…800 = mise à jour ; DLC = jeu +
  0x1000 + n) ; « installer » = AJOUTER LE DOSSIER à `[Paths] external_content_dirs` de `<eden>\user\config\qt-config.ini`
  (relevé dans les sources d'Eden ; config copiée à l'abri avant ; rien n'est copié ; résultat relu) ; un .nsp de mise
  à jour ou de DLC n'est pas pris pour un jeu à l'import. Chez Seb (lecture seule) : 24 contenus sur 25 en 0,2 s
  (Luigi's Mansion 3 et les DLC de Zelda BOTW reconnus par leur ticket). Limite : les jeux .xci ne disent pas leur
  identifiant sans les clés de la console → rattachement par le titre pour eux.
- **✅ 0.42.0 (Wii U / Cemu)** : un .wua (ZArchive, relevé dans les sources officielles : fin de fichier de 144 octets,
  arbre d'entrées de 16 octets, table des noms non compressée) contient ses mises à jour et DLC dans des dossiers
  « <id>_v<version> » (00050000 jeu, 0005000E mise à jour, 0005000C DLC) : le panneau les MONTRE, rien à installer
  (Cemu lit le .wua entier). Chez Seb (lecture seule) : 58 .wua lus, 35 avec mise à jour, 12 avec DLC. **Reste** :
  Xbox 360 (Xenia), PS Vita (Vita3K) — pas de jeux chez Seb pour l'instant.
- **✅ 0.43.0 (décompresser pour jouer)** : un jeu importé rangé dans un .zip/.7z qui contient une image disque (ou un
  jeu PS3 en dossier) : le panneau propose « 🗜 Décompresser pour jouer » (taille, dossier, place libre et durée
  annoncés ; oui de la personne). Décompression À CÔTÉ de l'archive (dossier de la console, choisi par la personne),
  dans un dossier provisoire, chaque fichier vérifié à sa taille, puis renommé ; jamais par-dessus un fichier
  existant ; l'archive est GARDÉE (rien ne s'efface sans preuve). Le jeu se lance ensuite par l'image (nom gardé).
  RPCS3 lit les .iso PS3 DÉCHIFFRÉS (ajout récent de RPCS3, d'après la presse : à vérifier au premier essai) ; ceux de
  Seb le sont. Chez Seb (lecture seule) : 7 jeux PS3 reconnus (6,7 à 47 Go), aucun DLC ni avatar pris pour un jeu.
- **✅ 0.43.1 (contrôle de sécurité et relecture, 03/10)** : 9 constats de sécurité et 17 de relecture, vérifiés
  dans le code puis corrigés avec leurs tests (7z sans contrôle de chemin, jetons RetroAchievements dans la
  sauvegarde et après « Oublier », chemin de triche enraciné, MAME pris pour un disque, versions perdues, .cso/.nsp
  abîmés qui fermaient Frogtend — `panic = "abort"` en version livrée —, Eden en « / », compte RPCS3…). **Pas
  corrigé, noté** : le cœur ne recompare pas aux réglages les dossiers que l'interface lui donne (emplacements,
  émulateurs, outils) — défense en plus, à faire dans un lot « durcissement » ; la recherche ISO 9660 n'exige pas
  « CD001 » (plafonnée à la place, pour ne pas risquer les empreintes déjà vérifiées chez Seb).
- **Stock de départ de Firehouse (cadrage du 03/10, demandé par Firehouse)** : ~1,9 To de Venkman (E:\Games) va
  entrer dans le stock de Firehouse. Réponses de Frogtend transmises : cartouches zippées OK, disques JAMAIS zippés
  (CHD, RVZ, .wua, PS3 selon l'essai RPCS3), noms No-Intro/Redump gardés tels quels (titre clair dans l'API), MAME
  non-merged + CHD dans le dossier du jeu + mises à jour par différence, DLC séparés annoncés par version (besoin 18
  à écrire après accord), MUGEN = nouveau lot, réconciliation des fichiers déjà sur Venkman (ne pas retélécharger).
  **À faire côté Frogtend une fois l'étiquette décidée** : une fantrad « [T-Fr…] » doit compter comme une version
  FRANÇAISE (rang 0, choix de Seb « fantrad FR sinon EN ») ; aujourd'hui toute étiquette « [T…] » est pénalisée
  comme un hack.
  **Décisions de Seb du 03/10, transmises par Firehouse (à confirmer par Seb avant de coder)** : noms No-Intro /
  Redump EXACTS, jamais renommés ; une fantrad reçoit seulement « [T-Fr by <groupe>] » (ou « [T-En by <groupe>] »),
  à compter comme version française (resp. anglaise) et plus comme un hack ; le titre clair vient de l'API. MAME :
  romset NON-MERGED + CHD dans le dossier du jeu, même version que le paquet MAME servi. Le stock va sur Egon
  (« Jeux vidéo »). La liste « contenus » devient le besoin API n° 18 (écrit). Restent à cadrer : réconciliation des
  jeux déjà sur Venkman (nom ou empreinte), format PS3 après l'essai RPCS3, MUGEN.
- **Incident du 03/10 (mise à jour 0.9.1 → 0.43.1)** : les données de Seb (profil « Sebastien », ludothèque) ont été
  effacées par la désinstallation de l'ancienne version (case « Supprimer les données de l'application » de
  l'installeur Tauri). Restaurées depuis la sauvegarde faite juste avant (`E:\Frogtend-sauvegarde-avant-0.43.1`,
  empreintes vérifiées). 0.43.2 : copie de côté avant tout effacement par la désinstallation (crochet NSIS
  `src-tauri/windows/crochets.nsh`). **Règle** : avant toute installation chez Seb, sauvegarder ses données.
- **Essais du 03/10 avec Seb (0.43.3)** : le bouton « Ajouter » de l'import lisait chaque fichier en entier
  (`pistes_designees` lisait avant de regarder l'extension : 196 Go pour le dossier PS3) → corrigé, test avec un
  fichier creux de 4 Go, et **répétition du parcours complet sur le vrai dossier** (1,2 s : 7 jeux, 70 contenus
  écartés, 7 fiches, décompression proposée). Extensions à taper → types trouvés cochés d'office + plateforme devinée
  (53 des 60 dossiers de Seb reconnus). Fenêtres de console (reg.exe) masquées. **Méthode désormais** : répéter
  chaque parcours sur les vrais fichiers avant de livrer.
- **DÉCISION DE SEB (03/10) — Frogtend pour des novices** : « destiné à des gens qui veulent juste jouer et ne
  connaissent vraiment pas l'émulation ». Toute étape technique se fait seule, après UN accord simple. Lot à venir
  (« Prêt à jouer »), juste après l'essai RPCS3 :
  1. **À l'ajout de jeux d'une plateforme sans émulateur** (import, téléchargement depuis Firehouse) : proposer
     d'installer automatiquement l'émulateur recommandé ET ses fichiers système, en une seule confirmation (taille,
     durée annoncées).
  2. **BIOS et micrologiciels fournis par Firehouse, toujours à jour** (ex. PS3UPDAT.PUP pour RPCS3, BIOS PS1/PS2,
     Saturn…) : installés automatiquement dans l'émulateur avec son installation (besoin API n° 19). Plus jamais de
     fichier à chercher soi-même ; le bouton 💿 actuel reste en secours. Firehouse (03/10) : besoin reçu, À CADRER
     (source et mise à jour de chaque fichier, stockage sur Egon, accord chiffré de Seb pour chaque rapatriement) ;
     contrat publié plus tard dans EXEMPLES-API-JEUX-V1.md → d'ici là, ne PAS coder contre la route : la SIMULER.
     Annoncé aussi : un contrat 1.6 pour les médias des jeux (visuels par type, empreintes, synchronisation).
  3. Revoir chaque écran avec la question « un novice saurait-il quoi faire ? » (déjà fait : extensions → types
     cochés d'office, plateforme devinée, toutes les plateformes proposées pour un émulateur).
- **✅ ESSAI RPCS3 RÉUSSI (03/10, Seb)** : Asura's Wrath décompressé par Frogtend (0.43.4) démarre dans RPCS3
  0.0.43-20192 directement depuis l'.iso déchiffré, à 60 i/s (écran d'installation des données du jeu). L'approche
  PS3 est VALIDÉE : .iso déchiffré non zippé. → Format PS3 du stock de Firehouse : .iso déchiffré. La pause est levée.
  Vu pendant l'essai, à mettre dans le lot « Prêt à jouer » : RPCS3 n'a AUCUNE manette réglée par Frogtend (pas de
  `config/input_configs/…`) → régler une manette XInput standard à l'installation ; l'installation du micrologiciel
  pose des questions (à rendre silencieuse si RPCS3 le permet : vérifier sa documentation) ; le PUP de Seb est ancien
  (Firehouse le fournira à jour, besoin 19) ; la 1re compilation PPU est longue (une seule fois : le dire avant).
  Corrigé localement, à livrer : « Sur ce PC » relu après l'installation d'un émulateur.
  Seb (03/10, après l'essai) : « ça fonctionne nickel ; il manque encore la gestion des mods, shaders, cheats, l'OSD »
  → pour RPCS3 (et les émulateurs récents) : cache de shaders, codes de triche (patches RPCS3), mods, affichage à
  l'écran (OSD : performances, menu) — à intégrer aux lots « Prêt à jouer » et « Triches et mods ».
- **✅ 0.44.0 « Prêt à jouer » (03/10, en autonomie)** : `preparerPourJouer` (src/lib/emulateurs/pret*.ts) appelé
  après un import, après l'installation d'un jeu de console et quand ▶ Jouer manque d'émulateur : choix seul de
  l'émulateur (déjà là > installable par Frogtend > par Firehouse ; recommandé d'abord ; sinon l'assistant), UN accord
  qui dit tout, installation sans autre question, cœur RetroArch sans question, manette réglée, retenue pour la
  console. RPCS3 (relevé dans son code) : `config\input_configs\global\Default.yml`, joueur 1 = manette XInput
  branchée (XInputGetState) sinon clavier, à chaque partie, seulement si le fichier porte la marque de Frogtend ;
  micrologiciel par `--headless --installfw` (essayé en vrai sur une copie neuve : 3,4 s, aucune fenêtre, version
  relue dans `dev_flash\vsh\etc\version.txt` + `liblv2.sprx`). Reste (besoin 19) : fichiers système fournis par
  Firehouse et installés d'office ; première compilation PPU annoncée mais pas précompilable (aucune option de RPCS3).
- **Contrat 1.6 de Firehouse (2.41.0, 03/10) : les médias des jeux.** `/catalogue` porte `images` (mises en avant
  par type : empreinte, largeur, hauteur, région), `medias_nb`, `medias_empreinte` ; `/jeu/{id}/medias` liste tout ;
  `/media/{id}/{type}?id=&largeur=` (100 à 1000, WebP). Sans largeur = ORIGINAL (1 à 13 Mo). **0.44.1** : Frogtend ne
  demande jamais l'original (affichage et copie hors ligne en 1 000 px ; essai réel : 56 Ko au lieu de 605 Ko).
  **Lot à venir « Médias »** : réserver la place au vrai ratio de la boîte (plus de saut à l'affichage), choisir le
  visuel (jaquette, boîte 3D, cartouche…) dans l'affichage de la ludothèque, logos et fonds pour Taodbox, galerie
  (captures, dos, disque) dans la fiche, garder hors ligne les visuels choisis. Firehouse (03/10) : un admin pourra
  mettre en avant, retirer ou ajouter des images ; chaque geste change `medias_empreinte` et `maj_le` (la synchro par
  `depuis` le voit) ; une image retirée disparaît de `/jeu/{id}/medias` → le lot « Médias » doit aussi RETIRER du
  cache (et de la copie hors ligne) une image qui n'est plus servie.
- **Traductions de fan (Seb, 04/10 : « oui, mais il faudra le préciser »)** : « [T-Fr by <groupe>] » = version
  française (juste après le français officiel, avant l'Europe), « [T-En by …] » = anglaise ; affiché « FR (trad.) »
  et « FR (traduction de fan) — …, Traduction française par <groupe> ». Fait (0.44.2).
- **LOT OSD — le menu en jeu universel (Seb, 04/10)** : « servira pour Frogtend et Taodbox » ; « manette ET clavier
  souris » ; « il doit être le MÊME pour tous [les émulateurs], seul le fond peut être différent en fonction des jeux ou
  systèmes, et certaines options bien sûr ». Principe (déjà celui du menu en jeu, lot 4 ter) : UNE fenêtre de
  Frogtend par-dessus le jeu, identique pour tous ; derrière, un « adaptateur » par émulateur traduit l'ordre choisi
  (pause, sauvegarde rapide, disque…). Une action qu'un émulateur ne sait pas faire n'apparaît pas.
  Étapes :
  1. Ouvrir le menu à la manette (combinaison réglable) en plus du clavier ; lire la manette pendant la partie
     (XInput, déjà utilisé pour RPCS3) ; retirer les menus propres aux émulateurs (combinaisons Select + Start de
     DuckStation/PCSX2, L3 + R3 de RetroArch) pour qu'il n'y ait qu'UN menu.
  2. Se déplacer dans le menu à la manette (croix/stick, A valider, B retour), au clavier et à la souris ; même menu
     en Taodbox (échelle ×2).
  3. Le fond : visuel du jeu (médias du contrat 1.6 : fond, capture, logo), sinon du système, sinon du skin.
  4. Adaptateurs manquants : RPCS3, Cemu, Eden, Xenia… (documentation de chacun d'abord).
  5. Options propres à certains émulateurs (affichage des performances, résolution…), dans le même menu.
- **(levée le 03/10) PAUSE jusqu'à l'essai de Seb (décision du 03/10)** : décompresser Asura's Wrath et le lancer dans RPCS3
  (docs/A-TESTER.md, « Décompresser pour jouer »). S'il démarre : approche validée, on reprend. Si RPCS3 refuse l'.iso
  (même à jour) : extraire le contenu de l'.iso (dossier PS3_GAME) au lieu de l'.iso, une livraison de plus. En
  attente derrière cet essai : DLC Xbox 360 (Xenia) et PS Vita (Vita3K), profils de manette par jeu, manettes
  PlayStation dans Dolphin.

**MAME vient de Firehouse (Seb, 02/10)** : ni l'émulateur MAME ni les ROM ne sont « ceux de Seb ». **Firehouse
fournira MAME à jour (avec ses mises à jour) et les packs de ROM**, au choix **avec ou sans CHD** (les images de
disques durs / CD de certains jeux, rangées dans un sous-dossier au nom du jeu). Conséquences :
- la version du romset doit correspondre à la version de MAME : Frogtend installe les deux ENSEMBLE depuis Firehouse
  et les met à jour ensemble (annoncer la taille avant ; un pack MAME fait des dizaines, avec CHD des centaines de Go) ;
- la liste MAME (clones, BIOS, statut) doit venir de Firehouse, pour la MÊME version (besoin API n° 15) ; la lecture du
  `MAME.xml` d'un LaunchBox (0.28.0) n'est qu'un dépannage en attendant ;
- le lancement passe par le MAME fourni, avec son `rompath` réglé sur le dossier du pack (et des CHD).

**A. Les fichiers du disque rejoindront Firehouse.** Les jeux trouvés par les imports locaux (ROM, MAME, DOS…) vont
probablement entrer dans le stock TÉLÉCHARGEABLE de Firehouse, avec même des **packs téléchargeables** (par exemple
une saga entière). Conséquences pour Frogtend :
- un jeu importé du disque et le même jeu chez Firehouse doivent pouvoir se **rapprocher** (même jeu, même version :
  plateforme + nom de fichier officiel / empreinte), sans doublon dans la ludothèque ;
- prévoir l'affichage et le téléchargement d'un **pack** (plusieurs jeux d'un coup : annoncer le nombre, les Go et la
  durée avant, comme toute opération de masse) — contrat à demander à Firehouse quand il sera prêt.

**B. Les succès, partout où c'est possible** : Steam, Epic, et **RetroAchievements** pour l'émulation.
- Dans la fiche d'un jeu : dire si la **version choisie** (la ROM, son empreinte) est **compatible RetroAchievements** ;
  sinon, quelle version l'est et **la demander** à Firehouse si elle y est disponible, ou **indiquer la version
  compatible comme manquante** si Firehouse ne l'a pas.
- À étudier d'abord (documentation officielle) : l'API de RetroAchievements (compte et clé de la personne, dans le
  coffre ; liste des empreintes reconnues par jeu), les succès Steam (API Web, clé déjà réglée par profil), Epic.
- **✅ 0.32.0** (`src-tauri/src/succes.rs`, `src/lib/succes/`) : ⚙ Options ▸ Comptes ▸ 🏆 RetroAchievements (nom +
  clé d'API Web, VÉRIFIÉS par `API_GetUserProfile` avant d'être gardés ; clé dans le coffre, jamais dans un message ni
  le cache ; partie avec le profil). Empreintes selon « Game Identification » / rcheevos (en-têtes NES/FDS/Lynx/7800/
  SNES/PC Engine, ordre des octets N64, arcade = nom du fichier, ROM dans un .zip) ; numéros de console de
  `rc_consoles.h` ; liste d'une console (`API_GetGameList`, `h=1`) en cache une semaine (`cache\retroachievements`
  dans les données de Frogtend) ; panneau du jeu : « ✅ compatible », « ⚠ pas reconnue, mais <autre version> l'est »,
  ou « version compatible manquante : <nom officiel> → Demander à Firehouse » (`API_GetGameHashes`) ; progression
  (`API_GetGameInfoAndUserProgress`). Steam : x / y succès (`GetPlayerAchievements`). Essai réel (lecture seule) :
  Super Mario Bros (JU) (PRG 0) → `8e3630186e35d477231bf8fd50e54cdd`, l'empreinte sans en-tête connue de
  « Super Mario Bros. (Japan, USA) ». ~~jeux sur CD~~ **✅ 0.34.0** (`src-tauri/src/disque.rs`, logique de
  rcheevos `cdreader.c`/`hash_disc.c`) : lecture de la 1re piste (.cue/.bin, .ccd/.img, .iso, liste .m3u → 1er
  disque), taille de secteur devinée (synchro + « CD001 »), fichiers ISO 9660 ; Sega CD et Saturn (512 octets du
  secteur 0), PlayStation (nom du programme de SYSTEM.CNF + son contenu). Essai réel (lecture seule) : E:\Games\Sega CD
  → **56 empreintes sur 56**. Une version illisible (.chd) est dite « pas vérifiable », jamais « pas reconnue ».
  **✅ 0.35.0** : .chd lus (bibliothèque `chd` = chd-rs, Rust pur, BSD-3 : pistes des métadonnées CHT2/CHGD, 2448
  octets par secteur, pistes alignées sur 4, adresse absolue lue dans l'en-tête du secteur) ; Dreamcast (IP.BIN de la
  piste 3 + programme de démarrage, MIL-CD par la 1re piste de données) ; les CD Sega CD/Saturn/PlayStation en .chd
  passent aussi ; empreintes gardées en mémoire (fichier, taille, date). Essai réel (lecture seule) :
  E:\Games\Sega Dreamcast → **135 empreintes sur 135**, 0 erreur, ~0,7 s par jeu. **✅ 0.37.0** : 3DO (volume Opera +
  LaunchMe), ROM dans un .7z (un seul fichier), PC Engine SuperGrafx, GameCube et Wii par la bibliothèque `nod` 1.4
  (MIT/Apache : .iso, .rvz, .wia, .gcz, .wbfs, .ciso, lus comme un .iso brut, données Wii RECHIFFRÉES comme le disque
  d'origine) ; empreintes gardées sur le disque (`cache\retroachievements\empreintes.json`, par fichier/taille/date).
  Essais réels (lecture seule) : 3DO 200/200 (31 s), PC Engine .7z 455/455 (26 s), GameCube .rvz 81/81 (6 s), Wii .rvz
  99/99 (~30 s par jeu, une seule fois grâce au cache). **✅ 0.38.0** : PS2 (BOOT2/cdrom0:), PSP (.pbp entier, ou
  PARAM.SFO + EBOOT.BIN), Nintendo DS (en-tête, arm9, arm7, icône ; en-tête SuperCard ignoré), PC Engine CD (1re piste de
  données : en-tête du secteur 1 + programme, ou BOOT.BIN), PC-FX (plus grande piste de données ou piste 2), Neo Geo CD
  (les .PRG d'IPL.TXT) ; feuilles .cue à plusieurs pistes (toutes lues, piste choisie : n°, 1re de données, plus grande
  de données, dernière). **Aucun jeu de ces consoles sur le disque de Seb** : vérifié seulement par les tests (images
  fabriquées) — à confirmer avec de vrais jeux (Firehouse). **✅ 0.39.0** : PS3 (.iso/.chd : PS3_GAME\PARAM.SFO +
  USRDIR\EBOOT.BIN ; jeu en dossier : son EBOOT.BIN + le PARAM.SFO de son dossier), Jaguar CD (.cue à deux sessions,
  « REM SESSION 02 » : en-tête Atari dans la 1re piste de la 2e session, octets échangés ou non, cas « homebrew » par
  la piste 2), .cso (PSP compressé, lu bloc par bloc, `flate2`). Les .iso PS3 de Seb sont ZIPPÉS (78 zips de 1,5 Go
  et plus, un .iso chacun) : pas lisibles sans tout décompresser → le panneau dit « décompresse-la » (RPCS3 en a
  besoin aussi). Vérifié le 03/10 (lecture seule, dans les zips) : les **7 .iso PS3 sont DÉCHIFFRÉS** (EBOOT.BIN
  « SCE » et PARAM.SFO « PSF » lisibles dans la zone chiffrée d'origine ; README « decrypted ISO ») ; « Animaniacs »
  est un jeu **PS2** mal rangé ; 70 zips de DLC/avatars (.pkg + .rap) — idée : les installer dans RPCS3 (à décider avec
  Seb). **Toutes les consoles de RetroAchievements que Frogtend peut rencontrer sont couvertes** (sauf
  Wii U : pas de succès RetroAchievements) ;
  ~~brancher le compte dans les émulateurs~~ **✅ 0.33.0** : « 🎮 Connecter mes émulateurs » (mot de passe UNE fois →
  jeton `login2` comme rcheevos, au coffre ; mot de passe jamais gardé) ; à chaque partie, le compte DU PROFIL QUI JOUE :
  RetroArch (`cheevos_*` dans le fichier `--appendconfig` du profil), PCSX2 (`[Achievements]` + jeton dans
  `inis\secrets.ini`, jamais copié dans les sauvegardes de config), DuckStation (jeton chiffré par lui → connexion
  faite une fois DANS DuckStation, gardée par profil dans `<émulateur>\Profils\<profil>\cheevos-duckstation.json` et
  remise à chaque partie) ; un profil sans compte : succès coupés (jamais le compte d'un autre). Mode hardcore : laissé
  au réglage de l'émulateur ;
  **Epic** : pas d'API publique de succès, et **GOG Galaxy ne les synchronise pas** (vérifié le 02/10 sur une copie de
  sa base : succès GOG 557 jeux, Steam 47, Xbox 5, EA 1, Epic 0) → **✅ 0.36.0** : succès des jeux importés par Galaxy
  (GOG, Xbox, EA, Steam) lus dans `UserAchievements`, affichés « x / y (GOG Galaxy) » ; Epic reste sans succès ; demander à
  Firehouse une version manquante par son nom/empreinte (besoin API à écrire quand le cas se présente).

**C. Une fiche par jeu et par système, plusieurs versions de ROM.** Sur un même système, un jeu = UNE fiche ; ses
ROM (régions, révisions) en sont les **versions**, présentées dans cet **ordre de préférence : fr, puis eu, puis us
ou en, puis les autres**. Conséquence : l'import local (0.27.0) crée aujourd'hui une ligne par fichier ; il faudra
**regrouper** les versions d'un même jeu (titre nettoyé + système) sous une seule fiche, la version préférée étant
lancée par défaut, les autres au choix (« Jouer avec… »).
**✅ 0.31.0** : les étiquettes (No-Intro, Redump, TOSEC, GoodTools) donnent la région (fr : France/Fr/F ; eu : Europe/E/
langues européennes/pays d'Europe ; us/en : USA/U/World/En ; autres) et la qualité ([!] d'abord, [a] [o] [h] [t] [b],
bêta/proto en dernier) ; les disques d'une même version restent ensemble ; import regroupé par (système, titre
nettoyé), une version d'une autre région s'ajoute à la fiche existante sans changer celle lancée ; fichiers déjà
importés jamais en double (y compris ceux importés un par un en 0.27–0.30). ▶ Jouer lance la version préférée, « Jouer
avec… » propose les versions (pour cette partie), ⚙ Gérer le jeu ▸ 📀 Version la change d'habitude. Essai réel
(lecture seule) : E:\Games\NES, 1 086 fichiers → 828 fiches.

**Icônes des boutiques sur les pastilles** (✅ Seb 19/11, « les deux ») : à la place du texte « Steam », « Epic »… sur
les pastilles des jeux, une petite icône : (1) celle du lanceur INSTALLÉ sur ce PC, lue par Windows dans son programme
(rien n'est embarqué dans Frogtend, jamais un logo de marque copié) ; (2) en secours, celle servie par Firehouse
(BESOINS-API n° 55) ; (3) le texte en dernier recours. ✅ Fait en 0.213.0 pour (1) et (3) (module
`icones_lanceurs.rs` : Steam, Epic, GOG, Ubisoft, EA relevés sur Venkman ; Amazon et Battle.net non essayés) ; (2) attend
Firehouse (point d'entrée `iconeFirehouse` dans `src/lib/boutiques/icones.svelte.ts`). ✅ Seb 19/11 : le DISQUE a une icône dessinée par
Frogtend (un disque dur, couleur du thème) ; FIREHOUSE a son logo (celui que Seb utilise, une marque d'un tiers) :
EXCEPTION décidée par Seb (« si Sony veut nous faire un procès on avisera, c'est ma décision ») — livré dans
l'application ET gardé dans le dépôt (`static/icones/firehouse.png`, 64 px ; l'original `docs/logo/FireHouse.png`) —
Seb 19/11 : « je me fiche qu'il soit dans le dépôt », l'application n'est pas destinée au public, ne sert à rien sans
Firehouse, et le dépôt repassera en privé à la fin du développement ; absent d'une fabrication, la pastille écrit
« Firehouse ». Cette exception
ne vaut que pour ce logo : la règle « jamais un logo de marque copié » reste pour tout le reste.

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
