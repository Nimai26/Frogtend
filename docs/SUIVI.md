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
  - [ ] d. Un seul menu : retirer les combinaisons propres aux émulateurs posées en 0.8.0 (Select + Start de
        DuckStation/PCSX2, L3 + R3 de RetroArch).
  - [ ] e. Navigation dans le menu à la manette (croix/stick, A valider, B retour), au clavier (flèches, Entrée,
        Échap) et à la souris (survol, clic).
  - [ ] f. Le même menu en Taodbox (échelle ×2, focus fort).
  - [ ] g. Le fond : visuel du jeu (médias du contrat 1.6 : fond, capture, logo), sinon du système, sinon du skin.
  - [ ] h. Adaptateurs manquants : RPCS3, Cemu, Eden, Xenia (documentation officielle de chacun d'abord).
  - [ ] i. Options propres à certains émulateurs (affichage des performances…), dans le même menu.
- [ ] **Livrer la 0.44.2** : traductions de fan (« [T-Fr by …] » = version française, « [T-En by …] » = anglaise,
      affichées « FR (trad.) ») — codé et testé, commité (aed196d), PAS encore livré.
- [ ] **Lot « Médias »** (contrat 1.6, Firehouse 2.41.0) : ratio réel de la boîte réservé, choix du visuel de la
      ludothèque (jaquette, boîte 3D, cartouche…), logos et fonds pour Taodbox, galerie dans la fiche, copie hors ligne
      des visuels choisis, retrait du cache d'une image que Firehouse ne sert plus (édition par un admin).
- [ ] **RPCS3 : mods, shaders, triches** (Seb, 03/10 : « il manque encore la gestion des mods, shaders, cheats, l'OSD »)
      — l'OSD passe d'abord (Seb, 04/10).
- [ ] **Durcissement** (relevé à l'audit du 03/10) : le cœur ne recompare pas aux réglages les dossiers que
      l'interface lui donne (emplacements, émulateurs, outils).
- [ ] DLC Xbox 360 (Xenia) et PS Vita (Vita3K) — aucun jeu chez Seb pour l'instant, priorité basse.
- [ ] Succès Epic (pas d'API publique, pas synchronisés par Galaxy) — à étudier.

## ❓ Attend une décision de Seb

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
- [ ] **RPCS3 + manette Xbox** : branchée avant la partie → elle marche ; débranchée → le clavier.
- [ ] **💿 Micrologiciel** : affiche « Installé : version 4.91 » ; un PUP 4.93 s'installe sans fenêtre.
- [ ] **Sonde manette** (en attente depuis le 30/09) : lire les manettes quand un jeu a le premier plan — condition de
      l'OSD à la manette (exigence c).
- [ ] Menu en jeu au clavier (0.11.0) dans un vrai jeu : Pause/Attn, reprendre, sauvegarde rapide, quitter.
- [ ] Succès RetroAchievements (compte, puis « Connecter mes émulateurs ») ; un jeu NES par RetroArch.
- [ ] Comptes des boutiques : régénérer la clé Steam (l'ancienne a été vue en clair dans une capture), GOG Galaxy,
      jeux offerts Epic / GOG / Prime / PS Plus.
- [ ] DLC : PS3 dans RPCS3, Switch dans Eden (dossiers maintenant écrits en « / »).
- [ ] Taodbox, Cheat Engine, MAME, installation DOS, assistant, consoles récentes.

## ⏳ Attend un événement extérieur

- [ ] **Firehouse — besoin 15** : la liste MAME (MAME.xml de LaunchBox) servie par l'API.
- [ ] **Firehouse — besoin 16** : fiche et jaquette d'un jeu importé, par `launchbox_id` (les 7 jeux PS3 importés
      n'ont pas d'informations).
- [ ] **Firehouse — besoin 17** : MAME et ses packs (non-merged + CHD, même version — ✅ décidé par Seb le 03/10).
- [ ] **Firehouse — besoin 18** : la liste « contenus » (DLC, mises à jour) par version — accepté le 03/10.
- [ ] **Firehouse — besoin 19** : BIOS et micrologiciels fournis à jour — reçu, À CADRER chez Firehouse ; ne pas coder
      contre la route, la simuler.
- [ ] **Firehouse — stock de départ** (~1,9 To de Venkman) : réconciliation des jeux déjà présents (nom ou empreinte,
      ne rien retélécharger), MUGEN, lot PS3 en dernier (simulation montrée à Seb d'abord).

## 💡 Idées notées — PAS démarrées

- Profils de manette par jeu ; Dolphin et les manettes PlayStation ; extensions Cheat Engine du forum (voir
  [IDEES.md](IDEES.md)).
- MUGEN : un « build » = un jeu Windows prêt à jouer, personnages et décors = ses contenus (réponse à Firehouse, 03/10).
- Jellyfin → Taodbox : projet côté Firehouse, pas le travail de Frogtend.

## ✅ Livré (version — ce qui a été vérifié)

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
