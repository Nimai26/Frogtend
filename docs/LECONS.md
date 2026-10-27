# Frogtend — les leçons

> Méthode : [METHODE-DE-TRAVAIL.md](METHODE-DE-TRAVAIL.md) § 5. Chaque erreur qui a coûté du temps entre ici le jour
> même. À relire au début de chaque session et avant de toucher au domaine d'une leçon.

## 19/10 — Le même test rouge pendant la livraison : un 2e défaut du suivi des parties
Ce qui s'est passé : en livrant la 0.80.0, `un_jeu_lance_par_un_intermediaire…` a de nouveau échoué. Cause réelle :
sous charge, le lanceur (.bat) finit AVANT le premier relevé ; son heure de démarrage était notée « 0 », et tout vieux
processus orphelin dont le parent mort portait ce numéro passait pour un enfant du jeu (partie jamais finie). ·
Comment l'appliquer : une valeur « inconnue » ne doit jamais valoir « depuis toujours » (0) dans une comparaison de
dates ; prendre la borne sûre (l'instant du lancement). La leçon du 19/10 a payé : on a cherché, pas relancé.

## 19/10 — La lettre M refusée sur un clavier français
Ce qui s'est passé : « appuie sur la touche » n'acceptait une lettre que si son `code` commençait par `Key` ; en AZERTY,
M a le code `Semicolon` (la place du « ; » anglais). Les tests, écrits en pensant QWERTY, figeaient même ce refus. Vu
par l'expert interface avant livraison. · Comment l'appliquer : pour le clavier, tester AUSSI la disposition de Seb
(AZERTY) — `code` = la place sur un clavier anglais, `key` = ce qui est écrit sur la touche ; jamais l'un pour l'autre.

## 19/10 — Un test rouge seulement pendant la livraison était un vrai défaut
Ce qui s'est passé : le test du suivi d'un jeu lancé par un .bat passait seul, et a échoué dans la suite complète de
la livraison (machine plus chargée). Ce n'était pas « un test capricieux » : le suivi reconnaissait les processus à
leur seul numéro, que Windows réutilise. · Comment l'appliquer : un test qui échoue « de temps en temps » s'analyse
comme un vrai défaut (ici : numéro + heure de démarrage) ; la livraison qui s'arrête sur un test rouge a bien fait.

## 18/10 — Eden ne lisait pas les dossiers que Frogtend lui donnait
Ce qui s'est passé : depuis le lot des contenus Switch, Frogtend écrivait la liste des dossiers de mises à jour et
DLC d'Eden dans une section `[Paths]` ; Eden la lit dans `[UI]` sous `Paths\…` (groupe imbriqué dans `qt_config.cpp`).
Les tests vérifiaient que Frogtend relisait ce qu'il avait écrit — un contrôle circulaire —, jamais la forme qu'Eden
attend. · Comment l'appliquer : pour un fichier de réglages d'émulateur, tester contre un VRAI fichier de l'émulateur
(ou de sa famille) et sa source, pas contre ce que Frogtend écrit lui-même ; un « vérifié en relisant » qui relit sa
propre écriture ne prouve rien.

## 18/10 — Des octets nuls bruts glissés dans le code par un script d'écriture
Ce qui s'est passé : en écrivant des tests Rust par un script Python, l'échappement `\0` d'une chaîne d'octets est
devenu un vrai octet nul dans `noyau.rs` (livré ainsi en 0.67.0). Rust l'accepte, mais les outils de recherche
voient alors le fichier comme binaire (le même piège a ensuite glissé un octet nul et un saut de ligne dans cette
leçon même, écrite par script). · Comment l'appliquer : pour un texte qui contient des barres obliques inverses
(`\0`, `\n` voulus en toutes lettres), écrire avec l'outil d'écriture ou d'édition, jamais par un script ; après une
écriture scriptée, vérifier qu'aucun octet nul n'est apparu (`git diff` puis un compte des octets nuls).
19/10 : revenu deux fois (un `.join('\n\n')` devenu un vrai retour à la ligne dans `jeu.svelte.ts`, vu par
svelte-check ; un remplacement introuvable). Le code écrit par script passe désormais par un FICHIER de script dans le
scratchpad avec des chaînes brutes, ou par l'outil d'édition — jamais par un script tapé dans le shell.
19/10 (3e fois, lot 0.80.0, une chaîne de test cassée) : la règle devient stricte — tout texte de CODE qui contient
une barre oblique inverse s'écrit avec l'outil d'édition, sans exception (les scripts gardent les seuls documents).
20/10 (4e fois, lot 0.82.0) : un `sed` sur `crochets.nsh` (code NSIS : des chemins du registre) a mangé les
barres obliques inverses (`Classes` + retour à la ligne + `xm`). Vu aussitôt en relisant, fichier réécrit en entier
par l'outil d'écriture et vérifié octet par octet. Un fichier d'INSTALLEUR est du code : même règle, aucun `sed`.
21/10 (5e fois, lot B2) : un script python tapé dans le shell (heredoc) a encore changé un `'\n'` de `jeu.svelte.ts`
en vrai retour à la ligne ; vu en relisant la ligne (`cat -A`), réparé par l'outil d'édition. La règle tient telle
quelle : du code avec une barre oblique inverse = l'outil d'édition, même pour « une petite retouche ».
20/10 (lot 0.84.1) : `sed -i` dans le Git Bash de Windows RETIRE les fins de ligne Windows (CRLF → LF) de TOUT le
fichier : `GrilleJeux.svelte` montrait 452 lignes changées pour 4. Vu par la relecture de l'expert, réparé octet par
octet. Comment l'appliquer : jamais `sed -i` sur un fichier du projet ; un script de remplacement garde les fins de
ligne (lire, remplacer, réécrire à l'identique) ; avant chaque livraison, comparer les fins de ligne à HEAD (script
`eol.py` du brouillon).

## 18/10 — Un test vert parce que son serveur simulé était plus simple que la vraie vie
Ce qui s'est passé : le rattachement depuis la file de téléchargement (fiche réunie découverte par un 410) passait son
test, mais ne pouvait jamais marcher en vrai. Le catalogue simulé ne servait plus l'ancien numéro, alors qu'un vrai
profil le garde jusqu'à la synchronisation suivante. L'expert contrat l'a vu en relisant les corrections. · Pourquoi
c'est grave : un test vert faisait croire à un correctif. · Comment l'appliquer : un serveur simulé reproduit l'ÉTAT
RÉEL au moment visé (ce que le cache contient déjà, pas seulement la réponse du jour) ; toujours faire relire les
corrections d'un lot, pas seulement le lot.

## 18/10 — Une release publiée alors que le commit avait échoué
Ce qui s'est passé : `git add` contenait `CLAUDE.md`, ignoré par git : la commande a échoué, le commit (enchaîné par
`&&`) n'a pas eu lieu, mais la publication, sur une AUTRE ligne, est partie quand même — étiquette v0.63.0 posée sur
le commit de la 0.62.0. Rattrapé aussitôt (commit, push, étiquette recalée). · Comment l'appliquer : n'ajouter que
des fichiers suivis par git (`CLAUDE.md` est volontairement ignoré) ; enchaîner commit, push et release en UNE seule
chaîne `&&`, pour qu'un échec arrête tout ; vérifier que l'étiquette pointe sur HEAD (`git ls-remote`).

## 16/10 — Un réglage « sur la machine seulement » que RetroArch n'a jamais eu
Ce qui s'est passé : depuis la 0.8, Frogtend écrivait `network_cmd_bind_address = "127.0.0.1"` et le code affirmait
que les commandes de RetroArch restaient sur la machine. Le réglage n'existe pas : RetroArch 1.22.2 ouvre son port sur
tout le réseau, d'où la question du pare-feu que Seb avait déjà vue. Je l'ai découvert seulement grâce à un essai
réel (la fenêtre de Windows) puis en lisant `command.c`. · Comment l'appliquer : toute clé de réglage écrite dans un
émulateur se vérifie dans SON code (le nom exact, et qu'elle est bien lue) avant d'être écrite ; un commentaire qui
promet une garantie de sécurité cite la ligne de code qui la prouve.

## 16/10 — Le disque D: de Venkman était plein, par MA faute
Ce qui s'est passé : une compilation a échoué faute de place (96 Mo libres). Le cache de compilation de Frogtend
(`src-tauri\target\debug`) pesait 65,8 Go : il regrossit à chaque session, malgré `debug = "line-tables-only"` (même
cas le 02/10). Venkman sert aussi à Firehouse : un disque plein peut gêner ses travaux. Vidé par `cargo clean
--profile dev` (notre cache seulement, reconstructible) → 59,7 Go libres. Autre leçon du même essai : un abonnement Lua
de MAME (`emu.add_machine_*_notifier`) non gardé dans une variable est retiré par le ramasse-miettes. · Comment
l'appliquer : regarder la place libre de D: au début de chaque session et avant une grosse compilation ; vider
`target\debug` dès qu'il dépasse 30 Go ; dans un script Lua de MAME, garder chaque abonnement (ou utiliser
`register_periodic`, permanent).
20/10 (3e fois, livraison 0.82.1) : D: à 0 octet libre, `target\debug` à 65,7 Go ; je n'avais PAS regardé la place
au début de la session malgré la règle. Une règle qui dépend de ma mémoire ne tient pas : `scripts/livrer.mjs`
mesure désormais la place de D: et la taille de `target\debug` avant de compiler, et s'arrête en le disant.

## 16/10 — J'avais imaginé la forme du document au lieu de la regarder
Ce qui s'est passé : j'ai écrit le lecteur du livret pour des pages en hauteur (largeur = hauteur × 0,75). La
première page réelle rendue (Zelda) était une DOUBLE page à l'italienne : le lecteur l'aurait demandée trop petite,
et « Agrandir » n'aurait rien agrandi. Je ne l'ai vu qu'en ouvrant l'image. · Comment l'appliquer : avant de coder un
affichage, regarder au moins un vrai exemple du contenu (une page, une image, une vidéo), pas seulement sa
description dans le contrat.

## 16/10 — J'ai remplacé un réglage de l'émulateur au lieu de m'y ajouter
Ce qui s'est passé : pour lancer les jeux MAME, j'ai passé `-rompath <dossier du jeu>`, qui REMPLACE les dossiers
réglés dans `mame.ini` ; les jeux importés du disque, dont les CHD ou BIOS sont ailleurs, ne les auraient plus
trouvés. J'avais aussi toléré une entrée 7z sans nom sans lire son flux, au risque de décaler tout le bloc. L'expert
lancement l'a vu. · Comment l'appliquer : une option de ligne de commande qui a aussi un réglage dans l'émulateur
s'AJOUTE à ce réglage (relu dans son fichier), sauf raison écrite ; une entrée d'archive qu'on saute se lit quand même
jusqu'au bout, ou l'archive se refuse.

## 16/10 — Un essai réel dans un dossier temporaire a menti
Ce qui s'est passé : l'installation du micrologiciel sur une copie de RPCS3 plantait (0xC0000409). Ce n'était pas
Frogtend : RPCS3 refuse de tourner depuis un dossier temporaire, et mon brouillon était dans Temp. L'essai, refait
dans un dossier ordinaire, a montré deux vérités utiles : RPCS3 plante en se fermant APRÈS avoir installé (son code
de sortie ne vaut rien), et il garde la date d'origine des fichiers (une comparaison de dates aurait menti). ·
Comment l'appliquer : un essai réel d'émulateur se fait dans un dossier ordinaire (pas Temp), et sa sortie console
se lit avant de conclure ; on vérifie le RÉSULTAT par le contenu, pas par une date ni un code de sortie.

## 16/10 — J'avais le relevé réel qui montrait le cas, et je ne l'ai pas croisé avec le code
Ce qui s'est passé : pour le conseil de Firehouse, j'avais relevé en réel que MS-DOS conseille RetroArch alors que la
liste ne donne que DOSBox Staging. Mon code ne cherchait le conseillé QUE dans la liste : il aurait été ignoré sans un
mot. Les deux experts l'ont vu. · Comment l'appliquer : après un relevé réel, passer CHAQUE cas relevé dans le code (ou
mieux, en faire un test) avant de demander la relecture — un relevé qui n'est pas devenu un test ne protège de rien.

## 05/10 — Suspendre un programme : penser à TOUTES les sorties
Ce qui s'est passé : la première version du gel des jeux PC (0.47.0) ne dégelait qu'en passant par « Reprendre »,
« Quitter » ou la fin de partie. L'expert a trouvé les autres sorties : menu fermé par Alt+F4, Frogtend fermé pendant
le gel, Frogtend tué ; et un jeu lancé par raccourci n'était jamais figé alors que le menu disait « en pause ». ·
Comment l'appliquer : tout état « suspendu » (jeu figé, manette prise, fichier verrouillé) a un dégel sur CHAQUE
sortie (fermeture de fenêtre, sortie de l'application, plantage, arrêt brutal → note relue au démarrage) ; et
l'interface n'annonce un état que s'il est VÉRIFIÉ, pas seulement demandé.

## 05/10 — Le menu affichait « en pause » sans que rien ne le garantisse
Ce qui s'est passé : RPCS3 a été branché au menu (0.46.0) sans son adaptateur de pause (étape h encore à faire) ;
le jeu continuait et recevait la manette pendant que le menu était ouvert. Seb l'a vu. · Comment l'appliquer : avant
de brancher le menu sur un émulateur, vérifier dans son code les DEUX points de base — il se met en pause quand le
menu passe devant, et il ignore la manette en arrière-plan — et les régler par ses propres réglages.

## 05/10 — Chercher la cause dans le code de l'émulateur avant de forcer
Ce qui s'est passé : RPCS3 restait caché après « Quitter », puis plantait à l'arrêt. La 0.46.1 a ajouté un arrêt forcé
(un pansement). La lecture du code de RPCS3 a donné la VRAIE cause : son écran d'accueil, ouvert à chaque lancement,
invisible derrière le jeu en plein écran (`infoBoxEnabledWelcome`). · Comment l'appliquer : un émulateur qui se
comporte mal en plein écran sans interface → chercher dans son code les boîtes qu'il ouvre (accueil, confirmations,
mises à jour) et les désactiver par ses propres réglages ; l'arrêt forcé reste un dernier recours.

## 05/10 — « Quitter » ne vérifiait pas que le jeu était vraiment fermé
Ce qui s'est passé : « Quitter le jeu » envoyait la demande polie (comme la croix) et s'arrêtait là ; RPCS3 a fermé son
écran mais gardé son programme en vie, caché : Frogtend bloqué sur « partie en cours », RPCS3 impossible à relancer. Le
premier correctif (tout arrêter après 10 s) a été BLOQUÉ par l'expert : il pouvait tuer un programme étranger (numéro
de processus réutilisé, Steam relancé par un jeu) et couper une sauvegarde. · Comment l'appliquer : après toute
demande d'arrêt, VÉRIFIER que le processus est parti ; un arrêt forcé ne vise que ce qui est sûrement à nous (dossier de
l'émulateur) et seulement s'il a abandonné (plus de fenêtre, ou bloqué) ; faire relire tout ce qui termine des
processus.

## 05/10 — Livrer en comptant sur ce qu'un expert a signalé « non vérifié »
Ce qui s'est passé : les deux experts avaient prévenu (04/10) que la Gamepad API d'une fenêtre qui apparaît par-dessus
un jeu n'est pas fiable et recommandé que le cœur soit la seule source de la manette ; la 0.45.0 a quand même été
livrée avec la manette lue par la page du menu. Seb : « la manette ne fonctionne pas pour naviguer dans le menu ».
· Pourquoi c'est grave : une livraison et un essai de Seb pour rien. · Comment l'appliquer : un point « non vérifié »
d'un expert sur le cœur de la fonction se TRANCHE avant de livrer (essai réel, ou la voie qu'il recommande) ; sinon la
livraison annonce clairement que ce point n'est pas couvert.

## 04/10 — Un fichier de configuration écrit « à la main » sans en respecter le format
Ce qui s'est passé : Frogtend écrivait `Device: XInput Pad #1` dans un fichier YAML de RPCS3 ; en YAML, « #1 » après une
espace est un COMMENTAIRE : RPCS3 a lu « XInput Pad », n'a rien trouvé et a mis le joueur 1 sur « aucune entrée »
(manette ET clavier morts). Troisième échec de suite sur la manette RPCS3. Les tests vérifiaient le TEXTE écrit
(`contains`), jamais ce que l'émulateur en LIT. · Pourquoi c'est grave : Seb a perdu trois essais ; une livraison a
rendu le jeu injouable. · Comment l'appliquer : toute valeur écrite dans un format (YAML, INI, JSON, TOML) passe par les
règles de ce format (guillemets, échappements) ; les tests relisent le fichier COMME l'émulateur (`valeur_yaml`) ;
après un échec chez Seb, LIRE LE JOURNAL de l'émulateur avant toute hypothèse (il donnait la cause exacte).

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

21/10 (0.88.0) : `lancement::tests::un_jeu_lance_par_un_intermediaire_est_suivi_jusqu_a_sa_fin` a échoué une fois
pendant une passe complète anormalement lente (21 s au lieu de 8 : PC chargé) ; analysé tout de suite : fichier non
touché par le lot, 3 passes seules et 2 passes complètes vertes. Test dépendant du temps (un `ping` de ~2 s) : s'il
rougit encore, le rendre robuste à la charge plutôt que de relancer jusqu'au vert.
21/10 (0.90.0) : rouge à nouveau pendant `livrer` (« la fin est bien vue » : 20 s sans fin) ; rendu robuste : le .bat
vit ~1 s de plus, le premier relevé connaît l'heure EXACTE du lanceur (plus de marge de 5 s qui laissait passer le
processus d'un autre test). 3 passes complètes vertes, puis la livraison.

21/10 (0.90.0-0.92.0) : `sed -i` utilisé DEUX fois de plus (menu-jeu, succes.rs) malgré la règle — sans dégât cette
fois (fichiers en LF, diff vérifié aussitôt), mais c'est un réflexe à casser : un remplacement d'une ligne de code
passe par l'outil d'édition, toujours.
21/10 (0.92.0) : `tests/sept_z.rs` (7z piégé) rouge pendant `livrer` : l'octet 0x0C était cherché dès le début d'un
en-tête qui contient des dates et des CRC (un 0x0C par hasard → mauvaise place modifiée). Corrigé : chercher après
le codeur. Un test qui « fabrique » un fichier binaire ne cherche jamais un octet seul sans ancrage.

## 24/10 — Des messages à Seb écrits en anglais

Après une longue suite de messages de Firehouse, plusieurs réponses à Seb (et deux messages à Firehouse) sont
parties en ANGLAIS. Seb ne répond qu'aux messages en français, pour être sûr de bien comprendre. Règle : relire
CHAQUE message avant de l'envoyer — à Seb comme à Firehouse — et vérifier qu'il est en français, sans exception,
même après un long travail ou un résumé de contexte.

## 26/10 — `sed -i` a retourné un fichier CRLF entier (contenus.rs)

Un `sed -i` d'une ligne sur `contenus.rs` (fins de ligne CRLF dans Git) a réécrit TOUT le fichier en LF : le diff
montrait 2 858 lignes changées au lieu de 170. Repéré par `git diff --stat`, réparé en remettant les CRLF. La règle
existait déjà (« jamais `sed -i` pour du code ») : un remplacement passe par l'outil d'édition, ou par un script qui
garde les fins de ligne d'origine ; et `git diff --stat` après chaque changement d'un fichier CRLF.
Même jour : le nom du programme d'un émulateur (« mGBA.exe » écrit avec des majuscules) cassait toute sa recherche,
alors que le test d'un seul morceau restait vert — un test doit passer par le même chemin que le vrai code (ici la
recherche du programme dans le catalogue), pas appeler la fonction du bout directement.
26/10 (0.111.0) : un `\r` d'un chemin Windows écrit dans un script Python en ligne est devenu un VRAI retour
chariot dans un test Rust (`RetroArch\retroarch.exe`) ; le compilateur l'a refusé. Les chemins Windows dans du code
passent par l'outil d'édition ; un script ne s'écrit qu'en fichier (outil d'écriture), jamais en ligne de commande.

## 26/10 (0.113.0) — Commit et envoi faits sur une construction ÉCHOUÉE

Le résultat de `npm run livrer` n'a pas été lu avant le commit et l'envoi : la construction avait échoué (un test
rouge), aucun installeur n'existait, et le commit « version 0.113.0 » est parti quand même (aucune release publiée,
donc rien de proposé aux PC). Règle : AVANT le commit d'une livraison, vérifier « ✅ Livraison X prête » dans le
journal ET la présence de `Frogtend_X_x64-setup.exe` ; sinon, rien ne part.
Le test rouge (`un_jeu_lance_par_un_intermediaire…`, attente de 60 s jusqu'au bout) : un programme d'un autre test,
enfant d'un numéro REPRIS (son nouveau propriétaire vite refermé), était adopté comme morceau de la partie. Corrigé
dans le suivi lui-même (un processus de la partie vu fermé n'a pu lancer que des enfants nés AVANT) — le même trou
pouvait faire suivre un programme étranger dans une vraie partie.

27/10 (6e et 7e fois, tests du pack MAME et d'Eden) : deux scripts python tapés dans le shell ont encore abîmé des
barres obliques inverses dans du code de test (un chemin de test vidé, un littéral Rust cassé qui ne compilait plus,
vu par l'expert). · Garde-fou ajouté : tout script de retouche vit dans un FICHIER du scratchpad (jamais un heredoc),
et dès qu'un nouveau texte contient une barre oblique inverse, la retouche passe par l'outil d'édition ; après chaque
script, compiler et lancer les tests du module AVANT d'appeler un expert.

27/10 (0.128.0) : une livraison arrêtée par 3 tests de l'interface rouges (le faux Firehouse d'un test ne
connaissait pas la nouvelle commande `dossierJeuxRefus`) : après avoir retouché un fichier de l'interface, je n'avais
relancé que les tests du dossier voisin. · Comment l'appliquer : après toute retouche de l'interface, `npx vitest run`
EN ENTIER avant d'appeler un expert ou de construire. (La chaîne de livraison a bien arrêté tout : rien n'est parti.)

27/10 (8e fois, 0.132.0) : un heredoc python tapé dans le shell a mis de VRAIS retours chariot dans une chaîne de test
Rust (pilotage.rs devenu « mixed ») ; la compilation et les tests passaient, seule la vérification des fins de ligne
l'a vu. · Garde-fou : la règle « jamais de heredoc python pour du code » vaut AUSSI pour les petites retouches de
tests ; la vérification des fins de ligne est faite AVANT la construction, et son résultat est lu avant de continuer.
