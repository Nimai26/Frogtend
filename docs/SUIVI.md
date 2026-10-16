# Frogtend — le suivi unique

> Méthode : [METHODE-DE-TRAVAIL.md](METHODE-DE-TRAVAIL.md) § 1. Ce fichier porte TOUT ce qui est en cours, attendu ou
> noté. Il se relit EN ENTIER avant de répondre « qu'est-ce qui reste ? ». Une décision de Seb s'écrit ici et ne se
> redemande jamais. Créé le 04/10/2026 en reprenant `PLAN.md`, `A-TESTER.md`, `BESOINS-API.md` et `IDEES.md` (qui
> gardent le détail ; ce fichier est la liste qui fait foi).

## 🔨 En cours / à faire (sans attendre Seb)

- **Lot OSD — le menu en jeu universel** (Seb, 30/09 puis 04/10). Mots de Seb (04/10) : « servira pour Frogtend et
  Taodbox » ; « manette ET clavier souris » ; « il doit être le MÊME pour tous, seul le fond peut être différent en
  fonction des jeux ou systèmes, et certaines options bien sûr ». Exigences atomiques :
  - [ ] a. Le menu s'ouvre à la manette par **Select + R1 tenus 1 s** (Back + RB sur une manette Xbox), pendant toute
        partie, quel que soit l'émulateur ; réglable dans les Options. ✅ Seb 04/10 : « Select + Start, certains jeux
        peuvent avoir cette combinaison ; il en faut une bien moins courante, par exemple Select + R1 » (remplace la
        décision du 30/09) ; le maintien d'1 s est gardé contre les ouvertures par erreur.
  - [ ] b. Il s'ouvre toujours au clavier (Pause/Attn, réglable) — ✅ existe depuis 0.11.0.
  - [ ] c. Lecture de la manette pendant la partie : XInput (manettes Xbox, manette virtuelle de Sunshine) ; SDL3
        compilé dans le programme pour les manettes PlayStation/Switch/8BitDo (✅ décidé par Seb le 30/09 : « SDL3
        embarqué, aucune DLL »). Lisibilité en arrière-plan à MESURER (sonde, voir 🧪).
  - [x] d. Un seul menu : retirer les combinaisons propres aux émulateurs posées en 0.8.0 (Select + Start de
        DuckStation/PCSX2, L3 + R3 de RetroArch). ✅ Seb 05/10 : « je ne sais pas d'où ils viennent mais le but est
        d'uniformiser : il faut pouvoir avoir le menu Frogtend PARTOUT avec View + RB » → TOUTE liaison manette du
        menu d'un émulateur part, même faite à la main (le clavier reste). ✅ 0.46.3.
  - [ ] d bis. Le menu MET LE JEU EN PAUSE et le jeu ne reçoit plus la manette tant qu'il est ouvert. Seb 05/10 :
        « le menu ne met pas en pause le jeu et les boutons sont encore saisis dans le jeu » (RPCS3). ✅ RPCS3 en
        0.46.3 (ses propres réglages : pause à la perte du premier plan, manette ignorée en arrière-plan). Reste : les
        autres émulateurs à l'essai, et les jeux PC (aucune pause possible sans eux : à étudier). Réglages RPCS3 par
        jeu (`custom_configs`) traités aussi.
  - [ ] d quater. **Jeux PC : « ❄ Figer le jeu pendant le menu »** — ✅ Seb 05/10 (« oui » à la proposition) : option
        PAR JEU, désactivée d'office ; refusée pour un jeu où Frogtend trouve un anti-triche (fichiers dans le dossier
        du jeu) ; risques dits avant d'activer (son qui grésille, jeu en ligne déconnecté). Par défaut, le menu s'ouvre
        par-dessus le jeu PC sans le figer. ✅ 0.47.0 (à essayer par Seb).
  - [x] d ter. ✅ Seb 05/10 : « d'accord » → accepté tel quel, on ne touche pas aux profils. RetroArch : le bouton Xbox (Guide) ouvre encore SON menu — imposé par ses profils de manette officiels
        (`input_menu_toggle_btn = "10"`), impossible à couper proprement par la configuration (input_driver.c : la
        liaison du profil passe quand celle du fichier est vide). Expert, 05/10 : accepter (pas de conflit avec
        View + RB) plutôt que bricoler les profils. À observer à l'essai (la Game Bar de Windows
        le capte peut-être avant).
  - [x] e. Navigation dans le menu à la manette (croix/stick, A valider, B retour), au clavier (flèches, Entrée,
        Échap) et à la souris (survol, clic). ✅ 0.45.0 (à essayer par Seb).
  - [x] f. Le même menu en Taodbox (échelle ×2, plein écran, contour de sélection fort). ✅ 0.45.0 (à essayer).
  - [x] g. Le fond : visuel du jeu (médias du contrat 1.6 : fond, capture, logo), sinon du système, sinon du skin.
        ✅ 0.48.0 : fond, sinon capture, sinon écran titre du jeu, sinon le skin. Le niveau « système » attend
        Firehouse (besoin 20). Question à garder pour l'essai : la fenêtre bureau (560×640) ne montre le visuel que
        sur une bande de 16 px ; l'agrandir (~760×640) si Seb veut le voir au bureau.
  - [ ] h. Adaptateurs manquants : RPCS3, Cemu, Eden, Xenia (documentation officielle de chacun d'abord).
  - [ ] i. Options propres à certains émulateurs (affichage des performances…), dans le même menu.
  - Relecture par les experts interface et lancement (04/10), qui ont relu le code (fichier:ligne) :
    - ✅ LEVÉ le 04/10 (sonde de Seb) : la manette est lue pendant qu'un jeu a le premier plan (XInput et SDL).
      Reste : le menu ouvert depuis la manette doit obtenir le premier plan (sans lui, pas de pause et la manette
      pilote encore le jeu) → vérifié par le code (GetForegroundWindow) et essayé avec Seb à l'étape 3.
    - Le cœur sera la seule source de la manette (XInput, puis SDL3) : il repère Select + R1 et envoie au menu
      haut/bas/valider/retour ; la Gamepad API du WebView2 n'est pas fiable dans une fenêtre sans focus.
    - Retirer Select + Start de DuckStation/PCSX2 demande une MIGRATION (valeur exacte écrite par Frogtend, le reste
      gardé, fichier mis à l'abri) ; RetroArch : retirer la ligne du fichier régénéré à chaque partie.
    - Pièges : boutons encore tenus à l'ouverture ; un seul gestionnaire des flèches ; menu à l'échelle de la télé en
      Taodbox (aujourd'hui 560×640 fixe) ; jeux PC sans pause (la manette agit aussi sur le jeu) ; plein écran
      exclusif (DOSBox, jeux PC) cache le menu ; deux parties en même temps.
  - Découpe : (1) sonde avec Seb ; (2) navigation commune clavier/souris/manette + échelle Taodbox ; (3) le cœur lit
    XInput, Select + R1 tenus 1 s, premier plan vérifié, migration des anciennes combinaisons ; (4) SDL3 ; (5) le
    fond ; (6) options, puis adaptateurs un par un (RPCS3, Cemu, Eden, Xenia, xemu, Azahar, Vita3K, PPSSPP).
- [ ] **Lot « Jeux PC : plein écran et manette → clavier »** (ordre du 16/10). ✅ Seb 16/10 : « pour le plein écran
      Mega Man, attention, ce n'est qu'un jeu parmi d'autres, il faut que notre création soit le plus généraliste
      possible ». Exigences : (a) plein écran au lancement, PAR JEU, désactivé d'office, trois modes : rien / touche
      du jeu (F4, Alt+Entrée, F11… ; suggestion trouvée dans le ReadMe du jeu) / fenêtre agrandie sans bordure sur
      l'écran du jeu ; (b) manette → clavier PAR JEU : préréglages courants + bouton par bouton ; touches envoyées
      seulement quand le jeu a le premier plan, toutes relâchées au menu et à la fin ; la combinaison du menu reste
      au menu. Livraisons : (a) puis (b).
      ✅ (a) en 0.53.0. Expert lancement : 1re version BLOQUÉE (fenêtres hors du jeu visées ; touche qui rebascule un
      jeu déjà en plein écran) — refaite ; 2e relecture : 2 importants corrigés (état relu juste avant d'agir ; le
      choix de la personne jamais défait). Reste (b) manette → clavier.
- [ ] **Lot « Contrat 1.13 »** (analyse d'écart de l'expert contrat, 16/10 ; texte et exemples réels reçus de Firehouse
      le 16/10, dans CONTRAT-API-JEUX-V1.md et EXEMPLES-API-JEUX-V1.md). Frogtend n'utilise rien de la 1.7 à la 1.13.
      Écarts : (1.8) l'émulateur est choisi par `recommande` LaunchBox et sa LIGNE DE COMMANDE LaunchBox est reprise
      (pret.ts:25, pret.svelte.ts:42, assistant.svelte.ts:283) — la 1.8 l'interdit, le `conseil` doit primer ; choix
      forcés en dur DOSBox/Eden (assistant.svelte.ts:98-105) ; lignes figées dans pc.json sans suivre `maj_le`.
      (1.12) cœurs RetroArch pris chez libretro en nightly, sans empreinte (emulateurs.rs:587) au lieu des cœurs
      stables vérifiés de Firehouse. (1.13) Arcade : CHD posée à plat et lancée comme fichier du jeu (partie.rs:56),
      MAME lancé avec le chemin du zip au lieu de `mame <nom> -rompath` — CASSE un jeu Arcade à CHD (aucun chez Seb
      aujourd'hui). (2.58.0) un jeu installé dont la fiche a été réunie à une autre ne se lance plus (« pas sur ce
      PC ») ; besoin 1.14 « fusionné dans N » proposé à Firehouse. (1.7, 1.9–1.11) documents (livret PDF, musique,
      vidéo) et BIOS/firmwares servis : rien n'est cassé, mais c'est ce qui rend « Prêt à jouer » sans étape manuelle.
      Découpe : (1) conseil en lecture ; (2) conseil : ligne et cœur, origine retenue ; (3) cœurs servis + sha256 ;
      (4) BIOS servis (fichier/motif/archive, sha1, un seul accord) puis PS2 ; (5) firmwares PS3 et Switch (prod.keys
      au coffre : le dire) ; (6) MAME (dossier, nom court, -rompath, « incomplet », paquet 0.289) ; (7) documents ;
      (8) fiches réunies. `fichiers: null` d'un motif = `[]`.
      ✅ (1)+(2) en 0.49.0 : le conseil prime (même absent de la liste : MS-DOS → RetroArch, liste = DOSBox Staging
      seul), ligne = options de Frogtend juste avant `%rom%`, `%rom%` remplacé au lancement, `maj_le` retenu dans
      pc.json (reste : PROPOSER la mise à jour quand `maj_le` change). Installer un jeu DOS exige DOSBox (refus clair
      sinon) — si MS-DOS a été préparé avec RetroArch, Frogtend devra proposer d'ajouter DOSBox Staging (à faire).
      ✅ (3) en 0.50.0 : cœurs RetroArch pris chez Firehouse (`/emulateurs/retroarch/coeurs`), reprise, empreintes du
      zip puis de la dll, pose seulement après ; un cœur non servi est refusé (rien chez libretro). Reste chez libretro
      (expert, 16/10) : RetroArch LUI-MÊME (buildbot stable, emulateurs.rs) → passer par `/emulateurs/retroarch/paquet`
      de Firehouse ; les profils de manette `autoconfig.zip` (buildbot, sans empreinte) → demander à Firehouse de les
      servir (besoin à écrire).
      ✅ (4) en 0.51.0 : BIOS servis posés (fichier/archive obligatoires, SHA-1 vérifié, jamais un fichier remplacé,
      copie sûre entre disques), clés et « à poser soi-même » dits, PCSX2.ini réglé sans défaire un choix. Reste :
      les motifs servis (`fichiers[]`, aucun au 16/10) ; une archive déjà en place est retéléchargée (mineur) ; PCSX2
      non portable (ini dans Documents) : écart commun à tous les modules PCSX2.
      ✅ (5) PS3 en 0.52.0 : micrologiciel servi (PUP 4.93) téléchargé, vérifié, installé sans fenêtre ; RPCS3 doit
      être fermé ; place vérifiée ; réussite = version annoncée dans la note relue après (RPCS3 rend 0 et plante en se
      fermant, ticket 16421). Reste : la Switch — firmware servi (1.10) NON branché : Eden refuse sans prod.keys ;
      à faire quand les clés seront posables (lire d'abord la doc d'Eden sur l'installation hors menu). Firehouse :
      demander un champ `version` dans le BIOS du conseil (aujourd'hui lue dans la `note`). Règles Firehouse (16/10) : seul `conseil.bios[]` fait foi ;
      ignorer `actif`/`statut_emu_france` pour l'émulateur du conseil ; `conseil.paquet: null` → « cœur non servi par
      Firehouse », rien chez libretro (question posée à Seb par Firehouse : laisser RetroArch l'installer ?).
- [ ] **Lot « Médias »** (contrat 1.6, Firehouse 2.41.0) : ratio réel de la boîte réservé, choix du visuel de la
      ludothèque (jaquette, boîte 3D, cartouche…), logos et fonds pour Taodbox, galerie dans la fiche, copie hors ligne
      des visuels choisis, retrait du cache d'une image que Firehouse ne sert plus (édition par un admin).
- [ ] **RPCS3 : mods, shaders, triches** (Seb, 03/10 : « il manque encore la gestion des mods, shaders, cheats, l'OSD »)
      — l'OSD passe d'abord (Seb, 04/10).
- [ ] **Formats des fichiers d'émulateurs** (expert, 04/10, après le bug YAML de RPCS3) : xemu `xemu.toml`
      (emulateurs_profils.rs:402) — une apostrophe dans le chemin (« Jeux d'émulation », profil « D'Aquin ») rend le
      TOML illisible → chaîne "…" échappée + test qui relit en TOML ; Qt/QSettings (Eden `external_content_dirs`
      contenus.rs:346, Azahar emulateurs_profils.rs:425) — une virgule ou un point-virgule dans un chemin en fait une
      liste → `valeur_qt` (guillemets) + relecture qui retire les guillemets + test « Zelda, TOTK ». Chez Seb
      aujourd'hui : aucun des deux cas (pas de xemu ; dossier DLC sans virgule).
- [ ] **Changer le jeton d'un profil** (expert contrat, 15/10, déjà vrai avant) : `changer_jeton` garde la
      ludothèque, les jaquettes et les visuels du profil ; si le nouveau jeton appartient à un AUTRE compte Firehouse
      (ex. un admin remplacé par un mineur), ce qui a été reçu pour l'ancien reste visible jusqu'à la synchronisation.
      → vider ces caches (ou forcer une synchronisation complète) quand le compte change. Règle du brief § 2.
- [ ] **Durcissement** (relevé à l'audit du 03/10) : le cœur ne recompare pas aux réglages les dossiers que
      l'interface lui donne (emplacements, émulateurs, outils).
- [ ] DLC Xbox 360 (Xenia) et PS Vita (Vita3K) — aucun jeu chez Seb pour l'instant, priorité basse.
- [ ] Succès Epic (pas d'API publique, pas synchronisés par Galaxy) — à étudier.

> **Ordre des lots — ✅ Seb 16/10 : « je te propose de choisir toi-même l'ordre, je te suis ».** Ordre retenu :
> (1) Contrat 1.13 : conseil → cœurs servis → BIOS servis → firmwares PS3/Switch ; (2) Jeux PC : manette → clavier,
> plein écran au lancement ; (3) MAME, documents, fin du lot OSD (SDL3, options, adaptateurs).

## ❓ Attend une décision de Seb

- [ ] **Jeux PC sans manette ni plein écran** (Seb, 05/10, Mega Man Rock Force : « il s'ouvre en tout petit fenêtré et
      ne reconnaît pas la manette »). Relevé : jeu Game Maker 8, clavier seulement (ReadMe : Z saut, X tir, C glisse,
      Entrée pause, A/S armes ; F4 plein écran) ; la manette est vue par Windows (joystick 1, XInput 0). Proposé :
      (1) « manette → clavier » par jeu (le cœur lit déjà XInput : traduction en touches, correspondance d'office
      modifiable) ; (2) « plein écran au lancement » par jeu (touche du jeu, ou fenêtre agrandie sans bordure).
      ✅ Ordre décidé le 16/10 (voir en tête de cette section) : ce lot vient après le contrat 1.13. Le gel (0.47.0) : ✅ « cela
      semble fonctionner » (Seb, 05/10).

- [ ] Questions encore ouvertes du 02–04/10 : aucune bloquante. (La combinaison du menu à la manette était DÉJÀ
      décidée le 30/09 : ne pas la redemander — voir LECONS.md, 04/10.)
- [x] Méthode de travail : ✅ Seb 04/10 « tu as mon accord » → ligne en tête de `CLAUDE.md` et trois experts dans
      `.claude\agents\` (contrat, ui, lancement).
- [ ] Option : désinscrire de la lettre d'information de GOG après un jeu offert (Frogtend ne touche pas aux réglages
      de compte sans décision).

## 🧪 Tests à faire AVEC Seb

Le détail (quoi faire, ce qu'on doit voir) est dans [A-TESTER.md](A-TESTER.md) (128 cases à cocher). Les plus utiles
d'abord :
- [ ] **Prêt à jouer (0.44.0)** : une console sans émulateur (ex. Sega CD) se prépare après UN accord.
- [ ] **💿 Micrologiciel** : affiche « Installé : version 4.91 » ; un PUP 4.93 s'installe sans fenêtre.
- [x] ✅ **Sonde manette faite par Seb le 04/10** (Asura's Wrath au premier plan dans RPCS3) : la manette Xbox Series X
      est lue EN ARRIÈRE-PLAN par XInput ET par SDL — tous les boutons, gâchettes, croix, Select, R1, L3/R3. Le
      blocage de l'OSD à la manette est levé. Reste à mesurer : le premier plan pris par le menu ouvert depuis la
      manette (vérifié par le code avec GetForegroundWindow), une manette PlayStation.
- [x] ✅ **Seb 05/10 : « ça a fonctionné » (0.44.4)** — **RPCS3 + manette** : Seb 04/10, deux fois « la manette ne fonctionne pas » : (1) réglage jamais appelé
      au lancement (0.44.2) ; (2) manette sans fil en veille au clic → Frogtend donnait le clavier. Corrigé : manette
      branchée, sinon la dernière vue, sinon Xbox ; RPCS3 se reconnecte seul. (3) 0.44.3 : « même le clavier ne
      fonctionnait pas » → « #1 » lu comme un commentaire YAML (journal RPCS3 : device='XInput Pad', NullPad) ;
      corrigé en 0.44.4 (guillemets). À revérifier : manette ÉTEINTE au clic, allumée ensuite → elle marche.
- [ ] Menu en jeu au clavier (0.11.0) dans un vrai jeu : Pause/Attn, reprendre, sauvegarde rapide, quitter.
- [ ] Succès RetroAchievements (compte, puis « Connecter mes émulateurs ») ; un jeu NES par RetroArch.
- [ ] Comptes des boutiques : régénérer la clé Steam (l'ancienne a été vue en clair dans une capture), GOG Galaxy,
      jeux offerts Epic / GOG / Prime / PS Plus.
- [ ] DLC : PS3 dans RPCS3, Switch dans Eden (dossiers maintenant écrits en « / »).
- [ ] Taodbox, Cheat Engine, MAME, installation DOS, assistant, consoles récentes.

## ⏳ Attend un événement extérieur

- [ ] **Firehouse — contrat 1.7 → 1.13** (16/10, message de la session Firehouse « FIREHOUSE 2 ») : Frogtend ne lit que
      la 1.3 (+ notes 1.6). ✅ Texte 1.7 → 1.13 reçu le 16/10 et intégré à CONTRAT-API-JEUX-V1.md ; exemples réels
      annoncés. → relecture par l'expert contrat : ce que Frogtend doit changer (documents 1.7, conseil 1.8, BIOS 1.9,
      cœurs 1.12, MAME 1.13, fiches réunies 2.58.0). Nouvelles transmises par
      Firehouse : 2.67.5 (BIOS du stock de départ reconnus, 7 BIOS ajoutés dont SCV `upd7801g.s01`, 6 « à décider ») ;
      2.67.8 (paquet RPCS3 servi de nouveau, RPCS3 a renommé ses fichiers `_win64_msvc.7z`) ; firmware Switch servi
      18.1.0, Nintendo publie 23.0.0 (rien téléchargé, avis à Seb) ; Ryubing (git.ryujinx.app) injoignable depuis la
      maison : source de son paquet peut-être périmée.
- [ ] **Firehouse — besoins 16 et 20 : proposition de Firehouse** (16/10, `docs/media/28-FROGTEND-BESOINS-16-20.md`
      côté Firehouse, soumise à Seb, RIEN codé) : 16 = `POST /identifier` (empreinte du stock → nom exact → titre
      unique, sinon candidats ; jamais deviné) + `GET /base/{launchbox_id}` ; PS3 : Frogtend extrairait
      `PS3_GAME/PARAM.SFO` de l'ISO et enverrait TITLE_ID + TITLE (forme proposée, PAS retenue). 20 = deux types par
      plateforme : logo/bannière transparent ≤ 500 px + fond 16:9 ≤ 1000 px. Images copiées depuis D:\LaunchBox\Images
      de Venkman d'abord. Ne rien coder avant la décision de Seb, transmise par Firehouse.
      Écart signalé par Firehouse : `/recherche` → `jaquette` est une URL Internet (images.launchbox-app.com) ;
      Frogtend ne l'affiche pas (vérifié 16/10) : ne jamais l'afficher sans décision de Seb.
- [ ] **Firehouse — images de Dune et de tout MS-DOS** (cause donnée par Firehouse le 16/10) : depuis la 1.6, seules les
      images RAPATRIÉES sont servies ; le rapatriement MS-DOS n'a jamais été lancé (25 images de Dune « à
      télécharger »). Firehouse le présente à Seb, chiffré (PC, MS-DOS, PS3…). Rien à changer dans Frogtend.
- [ ] **Firehouse — besoin 15** : la liste MAME (MAME.xml de LaunchBox) servie par l'API.
- [ ] **Firehouse — besoin 16** : fiche et jaquette d'un jeu importé, par `launchbox_id` (les 7 jeux PS3 importés
      n'ont pas d'informations).
- [ ] **Firehouse — besoin 17** : MAME et ses packs (non-merged + CHD, même version — ✅ décidé par Seb le 03/10).
- [ ] **Firehouse — besoin 18** : la liste « contenus » (DLC, mises à jour) par version — accepté le 03/10.
- [ ] **Firehouse — besoin 19** : BIOS et micrologiciels — ✅ FIGÉ côté Firehouse (contrat 1.9, précisé 1.10–1.11 ;
      texte dans CONTRAT-API-JEUX-V1.md, reçu le 16/10). Exemples réels frais annoncés par Firehouse ; coder contre
      eux seulement quand ils sont là.
- [ ] **Firehouse — stock de départ** (~1,9 To de Venkman) : réconciliation des jeux déjà présents (nom ou empreinte,
      ne rien retélécharger), MUGEN, lot PS3 en dernier (simulation montrée à Seb d'abord).

## 💡 Idées notées — PAS démarrées

- Profils de manette par jeu ; Dolphin et les manettes PlayStation ; extensions Cheat Engine du forum (voir
  [IDEES.md](IDEES.md)).
- MUGEN : un « build » = un jeu Windows prêt à jouer, personnages et décors = ses contenus (réponse à Firehouse, 03/10).
- Jellyfin → Taodbox : projet côté Firehouse, pas le travail de Frogtend.

## ✅ Livré (version — ce qui a été vérifié)

- **0.53.0** (16/10) — Plein écran au lancement des jeux PC (par jeu : touche du jeu suggérée par son mode d'emploi,
  ou fenêtre agrandie sans cadre). Vérifié : suggestion RÉELLE sur les 10 jeux Windows de Seb (Rock Force → F4,
  Minecraft → F11, 8 autres → rien, jamais deviné) ; essai RÉEL sur des fenêtres de test (640×480 agrandie à
  1920×1080 ; fenêtre déjà plein écran + touche F11 → « rien ») — il a attrapé un défaut (fenêtre maximisée
  restaurée avant la vérification), corrigé ; 3 relectures d'experts (lancement ×2, interface). Tests : 300 cœur,
  134 interface. PAS vérifié : un vrai jeu (Rock Force F4, un jeu Unity), écran à 125/150 %.

- **0.52.0** (16/10) — Micrologiciel PS3 servi par Firehouse. Vérifié : essai RÉEL complet sur une COPIE de RPCS3
  (hors dossier temporaire : RPCS3 refuse de tourner depuis Temp) — PUP servi 206 197 916 o téléchargé et vérifié,
  4.91 → 4.93 en 2,8 s, réinstallation par-dessus reconnue, rien ne traîne ; tests (firmware servi, empreinte fausse,
  version dans la note, pourcentage) ; relecture de l'expert lancement (aucun bloquant ; échec silencieux, RPCS3
  ouvert, attente sans fin corrigés). Tests : 295 cœur, 131 interface.

- **0.51.0** (16/10) — BIOS servis par Firehouse posés tout seuls (contrat 1.9–1.11). Vérifié : essai RÉEL (BIOS PS2
  conseillé 4 194 304 o, SHA-1 conforme, posé dans un PCSX2 de brouillon, PCSX2.ini réglé en une écriture,
  facultatifs ignorés) ; tests (réponse PS2 réelle, élément illisible, 404 = à poser soi-même, empreinte fausse,
  archive sans rien remplacer, chemins piégés, PCSX2 sans défaire un choix) ; relectures des experts lancement et
  contrat (aucun bloquant ; 8 points corrigés). Tests : 293 cœur, 129 interface.

- **0.50.0** (16/10) — Cœurs RetroArch servis par Firehouse, vérifiés (contrat 1.12). Vérifié : essai RÉEL (55 cœurs
  servis ; Snes9x posé dans un RetroArch de brouillon, 2 948 608 octets, en-tête MZ, aucun reste) ; tests (zip ou dll
  abîmés refusés sans rien poser, cœur présent jamais remplacé, cœur non servi refusé, nom piégé refusé, reprise 416
  sur un `.part` d'une autre version) ; relecture de l'expert lancement (aucun bloquant, 2 importants corrigés).
  Tests : 285 cœur, 129 interface.

- **0.49.0** (16/10) — « Prêt à jouer » suit le conseil de la maison (contrat 1.8) ; `%rom%` placé au lancement.
  Vérifié : conseils RÉELS relevés chez Firehouse pour 7 plateformes (EXEMPLES) et repris dans les tests (SNES, PS1,
  MS-DOS absent de la liste, Switch, PS2) ; relectures des experts contrat et lancement (1 bloquant : le conseil perdu
  quand son émulateur manque à la liste — corrigé) ; tests (282 cœur, 129 interface). PAS vérifié : une vraie
  préparation de console de bout en bout (à essayer par Seb).

- **0.48.0** (15/10) — Fond du menu en jeu : visuel du jeu (fond, capture, écran titre), sinon le skin ; même carte
  pour tous. Vérifié : forme réelle des visuels relevée chez Firehouse (jeu 116 : 11 types ; fond 118 776 octets en
  WebP 1 000 px) ; tests du cœur (premier visuel trouvé, cache par profil, 503 qui passe au suivant, hors ligne,
  autre profil, chemin piégé jamais demandé) ; relectures des experts contrat et interface (aucun bloquant, points
  corrigés). PAS vérifié : le rendu à l'écran (aucun jeu de Seb n'a de visuel aujourd'hui).

- **0.47.0** (05/10) — « ❄ Figer le jeu pendant le menu » (jeux PC, par jeu, désactivé d'office, refusé avec un
  anti-triche ou un dossier trop grand pour le savoir). Vérifié : gel réel d'un vrai programme (64 et 32 bits),
  programme déjà suspendu par son créateur qui le reste, relance après un « arrêt brutal » simulé par la note ;
  recherche d'anti-triche sur les 10 jeux Windows de Seb (aucun, 3 s au plus pour Minecraft) ; relectures des
  experts lancement (2) et interface ; tests (279 cœur, 121 interface). PAS vérifié : un vrai jeu figé (son, fenêtre
  « ne répond pas »), à l'essai avec Seb.

- **0.46.3** (05/10) — Le menu met RPCS3 en pause et lui coupe la manette (ses réglages « Pause emulation on RPCS3
  focus loss » et « Background input enabled », aussi dans ses réglages par jeu) ; un seul menu : les menus manette de
  DuckStation/PCSX2/RetroArch retirés ; un INI illisible n'est plus jamais remplacé. Vérifié : clés et comportement
  relevés dans le code de RPCS3 (relu par l'expert lancement, pas de bloquant) ; essai sur une COPIE du vrai
  config.yml de Seb (2 lignes changées sur 282, même taille) ; tests (274 cœur, 120 interface). PAS vérifié : l'essai
  en jeu (Seb) ; DuckStation/PCSX2/RetroArch pas installés chez Seb.

- **0.46.2** (05/10) — RPCS3 sans boîtes invisibles (accueil, « Exit Game? ») ; menu : seul le bouton choisi est
  coloré. Retours de Seb (05/10) : « le surlignage des boutons reste derrière les boutons et Reprendre reste en orange »
  ; erreur fatale de RPCS3 au 2e Quitter. Vérifié : cause relevée dans le code de RPCS3 et dans le réglage de Seb
  (`infoBoxEnabledWelcome=true`) ; essai sur une COPIE du vrai fichier (1 ligne changée sur 55) ; tests (270 cœur,
  120 interface). PAS vérifié : l'essai en jeu (Seb).

- **0.46.1** (05/10) — « Quitter » : un émulateur resté en vie sans fenêtre (RPCS3) est arrêté après 10 s, seulement
  dans le dossier de l'émulateur de la partie, jamais un jeu PC ni un émulateur qui a encore sa fenêtre. Vérifié :
  RPCS3 bloqué chez Seb (PID 64208, « ne répond pas », titre « Welcome to RPCS3 ») arrêté à la main par l'agent
  (Seb avait choisi Quitter) ; 2 tests avec de vrais programmes ; deux relectures de l'expert lancement (la 1re a
  bloqué une version dangereuse). PAS vérifié : l'essai en jeu (Seb).

- **0.46.0** (05/10) — OSD étape 3 : le cœur lit la manette (XInput) pendant la partie ; Select + R1 tenus 1 s ouvrent
  le menu (réglable) ; croix/stick, A, B pilotent le menu (A/B au relâchement) ; « Annuler » d'abord pour une action
  dangereuse ; sélection rendue après un retour et visible sans premier plan. Vérifié : tests (267 cœur, 120
  interface), lecture réelle de la manette de Seb, relecture des experts lancement et interface (aucun bloquant).
  PAS vérifié : l'essai en jeu (Seb), la prise du premier plan depuis la manette (notée au journal si elle échoue),
  les manettes non XInput (étape 4, SDL).
- **0.45.0** (05/10) — OSD étape 2 : ✅ clavier et souris dans le menu (Seb 05/10) ; manette dans le menu : NON (Gamepad
  API muette) → 0.46.0.

- **0.45.0** (05/10) — OSD étape 2 : menu en jeu à la manette, au clavier et à la souris ; Taodbox en plein écran ×2.
  Vérifié : relecture de l'expert interface (1 bloquant corrigé : B dans « Quitter ? » reprenait le jeu — rôle
  `alertdialog` ignoré, aussi en Taodbox ; 3 importants corrigés) ; test qui reconnaît la confirmation ; 261 + 119
  tests. PAS vérifié : l'essai réel du menu (Seb) ; la manette n'agit que dans la fenêtre au premier plan (à essayer).

- **0.44.4** (04/10) — RPCS3 : valeurs du fichier de manette entre guillemets (« #1 » était lu comme un commentaire).
  Vérifié : le journal de RPCS3 chez Seb montrait la cause exacte ; un test relit le fichier avec les règles YAML (il
  aurait attrapé le bug) ; le code de RPCS3 confirme la reconnexion d'une manette éteinte au lancement. PAS vérifié :
  l'essai en jeu (Seb).
- **0.44.3** — ⚠ CASSAIT la manette ET le clavier dans RPCS3 (voir 0.44.4).

- **0.44.3** (04/10) — RPCS3 : n'importe quelle manette (Xbox/compatibles, PS4, PS5), même réveillée après le
  lancement ; clavier seulement sur choix. Principe rappelé par Seb (04/10) : « n'importe quelle manette doit être
  automatiquement configurée et reconnue par Frogtend et ses émulateurs ». Vérifié : noms et reconnexion relevés dans le
  code de RPCS3 ; tests ; détection réelle chez Seb (XInput n° 0, 31 périphériques HID lus). PAS vérifié : le jeu avec
  la manette réveillée après le lancement (essai de Seb) ; une manette PlayStation (aucune branchée).

- **0.44.2** (04/10) — manette RPCS3 branchée sur chaque partie (bug de la 0.44.0) ; traductions de fan.
  Vérifié : relecture par l'expert lancement (chemin `jeu_jouer` → `preparer` → `regler_rpcs3`, fichier de la personne
  jamais défait) ; tests (259 cœur, 117 interface) ; manette de Seb détectée en vrai (XInput n° 0). PAS vérifié : que
  RPCS3 accepte le fichier écrit (à essayer par Seb) ; aucun fichier « [T-Fr …] » chez Seb pour un essai réel.

- **0.44.1** (03/10) — contrat 1.6 : jamais la jaquette d'origine (1 à 13 Mo), au plus 1 000 px ; essai RÉEL sur
  Firehouse (jeu 115 : WebP 56 Ko au lieu de 605 Ko, 19 médias listés).
- **0.44.0** (03/10) — Prêt à jouer ; RPCS3 manette XInput sinon clavier ; micrologiciel `--headless --installfw`.
  Vérifié : installation silencieuse RÉELLE sur une copie neuve de RPCS3 (3,4 s, aucune fenêtre) ; version 4.91 lue
  sur le RPCS3 de Seb ; détection de manette (aucune branchée). PAS vérifié : le cas manette branchée, l'enchaînement
  Prêt à jouer en vrai (simulé par tests).
- **0.43.4** — toutes les plateformes proposées pour régler un émulateur ; vérifié par Seb (RPCS3 installé).
- **0.43.3** — import sans extension à taper, « Ajouter » ne lit plus 196 Go ; répétition RÉELLE sur le dossier PS3
  (1,2 s, 7 jeux, 70 contenus écartés) ; vérifié par Seb (7 jeux importés).
- **0.43.2** — motifs d'erreur visibles, barre du haut, copie de côté à la désinstallation ; vérifié par Seb.
- **0.43.1** — 26 corrections de l'audit (sécurité et relecture).
- **0.43.0** — décompresser pour jouer ; ✅ ESSAI DÉCISIF RÉUSSI par Seb le 03/10 : Asura's Wrath démarre dans
  RPCS3 depuis l'.iso, 60 i/s.
- Avant : voir [NOTES-DE-VERSION.md](NOTES-DE-VERSION.md).
