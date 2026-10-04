# Frogtend — les leçons

> Méthode : [METHODE-DE-TRAVAIL.md](METHODE-DE-TRAVAIL.md) § 5. Chaque erreur qui a coûté du temps entre ici le jour
> même. À relire au début de chaque session et avant de toucher au domaine d'une leçon.

## 04/10 — Détecter une manette une seule fois, au lancement
Ce qui s'est passé : la 0.44.2 regardait la manette au clic sur ▶ Jouer ; la manette sans fil de Seb était en veille,
Frogtend a donné le clavier à RPCS3. · Pourquoi c'est grave : deuxième échec de suite chez Seb ; contraire au principe
« n'importe quelle manette doit être automatiquement configurée et reconnue ». · Comment l'appliquer : une manette peut
arriver ou se réveiller APRÈS le lancement : régler l'émulateur sur un nom générique qui se reconnecte seul (lu dans
sa documentation), retenir la dernière manette vue, ne retomber sur le clavier que si la personne le choisit ; penser
aussi aux manettes PlayStation, Switch, génériques.

## 04/10 — Écrit mais pas branché : la manette RPCS3 de la 0.44.0
Ce qui s'est passé : `manettes::regler_rpcs3` était écrit et testé, mais `emulateurs_profils::preparer` (le lancement
d'une partie) ne l'appelait pas pour RPCS3 ; seul « Prêt à jouer » l'appelait, à l'installation. Les notes de version
disaient « réglée à chaque partie ». Seb : manette Xbox sans effet dans Asura's Wrath. · Pourquoi c'est grave : une
promesse fausse dans les notes, et Seb bloqué. · Comment l'appliquer : pour toute nouvelle fonction, suivre le chemin
RÉEL depuis l'action de la personne (commande Tauri → … → la fonction) et le tester PAR CE CHEMIN (ici :
`preparer("rpcs3", …)` écrit `Default.yml`) ; faire relire « écrit / branché » par l'expert avant de livrer.

## 04/10 — Une décision de Seb redemandée
Ce qui s'est passé : pour l'OSD, l'agent a demandé à Seb quelle combinaison de la manette ouvre le menu ; Seb l'avait
DÉJÀ décidée le 30/09 (Select + Start tenus 1 s, Pause/Attn au clavier, SDL3 embarqué), écrit dans `PLAN.md` § lot 4 ter.
· Pourquoi c'est grave : Seb perd du temps et doute que ses décisions soient retenues.
· Comment l'appliquer : avant toute question à Seb, chercher la réponse dans `docs/SUIVI.md` puis `docs/PLAN.md`
(`grep` sur le sujet). Une décision de Seb s'écrit dans SUIVI.md (« ✅ décidé par Seb le … »).

## 03/10 — La mise à jour a effacé les données de Seb
Ce qui s'est passé : passage de 0.9.1 à 0.43.1 par l'installeur ; la désinstallation de l'ancienne version (case
« Supprimer les données de l'application ») a effacé le profil et la ludothèque. Restaurés depuis une copie faite par
l'agent juste avant ; jeton à refaire. · Pourquoi c'est grave : données perdues sans l'agent ; Seb : « une update ne
doit jamais être aussi complexe ». · Comment l'appliquer : une mise à jour = un clic dans Frogtend (mise à jour
automatique) ; copie de côté avant tout effacement (crochet NSIS `src-tauri/windows/crochets.nsh`) ; l'agent
sauvegarde les données de Seb (avec empreintes) avant toute installation chez lui.

## 03/10 — « Motif inconnu » sur toutes les erreurs depuis le lot 0
Ce qui s'est passé : le cœur envoie `{sorte, motif}`, l'interface ne lisait pas `motif` ; un jeton refusé, une
création de compte… s'affichaient « motif inconnu ». · Pourquoi c'est grave : Seb ne pouvait pas comprendre ni
corriger seul. · Comment l'appliquer : un test d'interface qui reçoit la VRAIE forme d'erreur du cœur
(`messages.test.ts`) ; à chaque nouvelle forme de réponse, un test avec un exemple réel.

## 03/10 — « Ajouter » lisait 196 Go : les tests ne couvraient que de petits fichiers
Ce qui s'est passé : `pistes_designees` lisait chaque fichier EN ENTIER avant de regarder son extension ; sur le dossier
PS3 de Seb, Frogtend montait à 9,7 Go de mémoire et rien ne se passait. Le bug a traversé trois livraisons.
· Pourquoi c'est grave : bouton mort en vrai, tests verts. · Comment l'appliquer : pour toute fonction qui lit un
fichier : « et sur 50 Go ? » → test avec un fichier creux (`set_len`) ; avant de livrer un parcours, le RÉPÉTER sur
les vrais fichiers de Seb en lecture seule (`repetition_reelle` dans import_local.rs).

## 03/10 — Les scripts en ligne abîment les « \ » et les fins de ligne
Ce qui s'est passé : des scripts passés par heredoc (bash) ont réduit les « \\ » à « \ » (regex cassée dans
BlocDecompression, chemins de test vides de sens, « \n » devenus de vrais retours à la ligne) et converti des fichiers
CRLF en LF. · Pourquoi c'est grave : bugs invisibles (un test passait à vide) et diffs illisibles.
· Comment l'appliquer : pour tout texte qui contient « \ », l'outil Edit, ou un script écrit avec l'outil Write ;
avant chaque commit, comparer `git diff --stat` et `git diff --ignore-cr-at-eol --stat`.

## 03/10 — Décompression proposée pour MAME : livrée sans relecture
Ce qui s'est passé : « décompresser pour jouer » considérait les `.bin` d'un zip d'arcade comme une image disque ; au
oui, le jeu MAME ne se lançait plus. Trouvé par la relecture d'un expert, après la livraison. · Pourquoi c'est grave :
un jeu cassé chez la personne. · Comment l'appliquer : relecture par un expert AVANT chaque livraison, même petite ;
penser aux autres systèmes (arcade, cartouches) quand on écrit une règle pour un seul.

## 03/10 — Un écran qui ne se relit pas après une action
Ce qui s'est passé : après l'installation de RPCS3, « Sur ce PC » restait vide (liste lue une seule fois à
l'ouverture) ; la liste des systèmes ne venait que du catalogue de Firehouse. · Pourquoi c'est grave : Seb bloqué en
plein essai. · Comment l'appliquer : après toute action qui change une liste affichée, la relire ; se demander
« d'où vient cette liste, et que manque-t-il ? » (jeux importés, systèmes sans jeux).

## 30/09 — Une suppression pendant que le Frogtend de Seb était ouvert
Ce qui s'est passé : un test a effacé puis restauré `%APPDATA%\fr.hikari-no-sekai.frogtend` alors que le Frogtend
installé de Seb était ouvert. · Pourquoi c'est grave : fichiers verrouillés, risque de perte. · Comment l'appliquer :
`tasklist` avant ; sauvegarde + empreintes ; restaurer en copiant, jamais `rm -rf` ; préférer des tests `#[ignore]`
qui travaillent dans un dossier temporaire.

## Avant — Corriger un commentaire a cassé un test (0.38)
Ce qui s'est passé : une retouche de commentaire a modifié une chaîne d'octets de test. · Comment l'appliquer : après
TOUTE modification, relancer la suite complète ; ne jamais modifier le code pendant une construction.
