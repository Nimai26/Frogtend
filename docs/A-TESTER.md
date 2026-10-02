# À tester par Seb à son retour

> Tenu à jour au fil du travail. Chaque ligne dit **quoi faire** et **ce qu'on doit voir**. Coche (`[x]`) ce qui
> marche ; pour ce qui ne marche pas, un mot ou une capture suffit.

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

## MAME Arcade Full Set (0.28.0)
- [ ] 📥 Importer ▸ **MAME Arcade Full Set** : dossier `E:\Games\MAME`, liste `D:\LaunchBox\Metadata\MAME.xml`,
      🔍 Trier. **Attendu :** ~4 526 jeux retenus, et le détail des écartés (mécaniques, hors arcade, non jouables…).
- [ ] Ajoute-en quelques-uns, puis ▶ Jouer : à vérifier avec ton émulateur Arcade (MAME ou le cœur MAME de RetroArch)
      — Frogtend lui donne le zip avec son chemin complet. Dis-moi si ton MAME préfère « -rompath » + nom court.

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
- [ ] Barre du haut ▸ **📥 Importer ▾** : la même liste que LaunchBox. Les jeux locaux sont marqués « prochain lot ».
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
      `HKCUSoftwareCheat Engine` n'est plus dans le registre ; tes réglages sont dans
      `<dossier Cheat Engine>Profils<toi>cheatengine.reg` ; la clé d'avant Frogtend est dans
      `.frogtend-sauvegardesegistre-avant-frogtend.reg`.
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
