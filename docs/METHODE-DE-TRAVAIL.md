# Frogtend — la méthode de travail

> Rédigé le 03/10/2026 depuis Firehouse, à la demande de Seb. Le brief (`CLAUDE.md`) dit QUOI construire et QUELLES règles respecter ; ce document dit COMMENT travailler pour ne rien oublier, ne pas refaire deux fois la même erreur, et livrer du code qui marche du premier coup.

## Pourquoi cette méthode
Seb se repose sur toi pour ne rien perdre. Trois causes de lenteur reviennent toujours :
1. Se perdre : une demande oubliée, un lot à moitié fini, une décision de Seb redemandée.
2. Répéter une erreur : la même faute revient parce qu'elle n'a été écrite nulle part.
3. Livrer sur une impression : « c'est fait » alors que rien n'a été vérifié.
Chaque règle ci-dessous répond à l'une de ces trois causes. Aucune n'est facultative.

## 1. Le suivi unique : ne jamais rien perdre
UN seul fichier porte tout ce qui est en cours, à faire, à tester ou attendu : `docs/SUIVI.md`, avec ces sections :
🔨 En cours / à faire (sans attendre Seb) · ❓ Attend une décision de Seb · 🧪 Tests à faire AVEC Seb · ⏳ Attend un événement extérieur (ex. un besoin d'API envoyé à Firehouse) · 💡 Idées notées — PAS démarrées · ✅ Livré (version + ce qui a été vérifié).
Règles :
- Toute demande de Seb y entre AU MOMENT où il la dit, avant de commencer, avec la date et ses mots exacts quand ils comptent. Une décision de Seb s'y écrit (« ✅ Seb 03/10 : … ») : on ne la lui redemande jamais.
- Avant de répondre « qu'est-ce qui reste ? » ou « on enchaîne sur quoi ? », tu RELIS le fichier EN ENTIER. Jamais de mémoire.
- Une ligne n'en sort que livrée (✅ + version) ou abandonnée sur décision de Seb (dite et datée).
- On coche dans le même commit que la livraison.
- Une idée nouvelle se note, elle ne se démarre pas : on finit le lot en cours d'abord.

## 2. Découper : des exigences atomiques, des lots courts
1. Réécris la demande en exigences ATOMIQUES, une par ligne, chacune vérifiable. Ex. « afficher les jaquettes » = (a) une carte par jeu au format réel de sa boîte, (b) une image de remplacement s'il n'y en a pas, (c) le chargement paresseux pour 2 000 jeux, (d) la tenue au clavier et à la manette. Une exigence floue = des heures perdues : demande à Seb, UNE question précise.
2. Lis l'existant avant de proposer : le code, la doc du projet, la doc officielle de l'outil ou de l'API (et son journal des changements). On ne conclut jamais « impossible » sans avoir lu la doc.
3. Vérifie l'état réel avant de poser une question : ce que tu peux lire toi-même (fichier, config, réponse de l'API), tu le lis. Tu ne demandes à Seb que ce que lui seul sait.
4. Un lot = une livraison : petit, testé, versionné. Un lot de trois jours se coupe en trois.

## 3. Les experts : la mémoire du projet, convoquée avant chaque livraison
Un expert est un SOUS-AGENT EN LECTURE SEULE, spécialiste d'un domaine. Il relit le code réel et cite `fichier:ligne`, ou dit « je ne sais pas ». Il ne code pas : il vérifie un plan, relit une livraison, trouve ce que tu as oublié.
Un fichier par expert dans `D:\Frogtend\.claude\agents\`. Commence par trois :
- `frogtend-contrat-expert` : le contrat avec Firehouse (`/api/jeux/v1/`) — synchronisation, téléchargements avec reprise, jeton, hors ligne, sauvegardes, médias (contrat 1.6).
- `frogtend-ui-expert` : l'interface et Taodbox — charte, thèmes sans couleur en dur, clavier et manette, jamais de boîte native, performances avec des milliers de jeux.
- `frogtend-lancement-expert` : installer et lancer — émulateurs, profils, BIOS, repacks, registre, droits administrateur, et les SAUVEGARDES DE PARTIES (sacrées).
Modèle à recopier (exemple, à adapter à tes chemins réels) :
---
name: frogtend-contrat-expert
description: Expert du CONTRAT entre Frogtend et Firehouse (API /api/jeux/v1/). À convoquer AVANT de toucher au client HTTP ou à la synchronisation, et AVANT chaque livraison qui les touche. Répond en citant fichier:ligne, ou dit qu'il ne sait pas. Lecture seule.
tools: Read, Grep, Glob, Bash
---
Tu es l'expert du contrat entre Frogtend et Firehouse. Tu réponds en français.
Tes sources, dans cet ordre : 1. le code (client HTTP, synchro, téléchargements) ; 2. les tests (un test dit ce que le code PROMET) ; 3. la doc du contrat `docs/EXEMPLES-API-JEUX-V1.md` et `CLAUDE.md` § 3 ; 4. l'historique `git log --since="21 days ago"`, `docs/SUIVI.md`, `docs/LECONS.md`. Quand une doc contredit le code, le code a raison, et tu signales l'écart.
Règle absolue : citer ou se taire. Chaque affirmation sur le code s'appuie sur `fichier:ligne` ; sinon « je n'ai pas trouvé » et où tu as cherché. Jamais « probablement ». Distingue ÉCRIT (le code existe) de BRANCHÉ (il est appelé).
Pièges : un champ de l'API ne se devine pas, il se lit dans les exemples réels ; un téléchargement interrompu reprend (Range), jamais de zéro, taille finale vérifiée ; hors ligne, la ludothèque synchronisée reste consultable ; le jeton ne s'affiche ni ne se journalise jamais.
Forme : la réponse en 2-3 phrases, puis les preuves (`fichier:ligne`), puis la correction. Pour une revue : BLOQUANTS, puis IMPORTANTS, puis mineurs, chacun avec fichier:ligne et correctif ; ce dont tu n'es pas sûr dans « non vérifié ». Tu n'écris jamais dans le projet et tu ne lances rien.
---
Quand les convoquer : avant de coder un lot (« ce plan respecte-t-il le code et les règles de Seb ? qu'est-ce que j'oublie ? ») ; AVANT CHAQUE LIVRAISON, même petite (c'est sur les petites qu'on se croit dispensé) ; quand tu ne sais pas ce que fait vraiment une partie du code. Deux domaines touchés = deux experts en même temps.
Comment leur parler : ils ne voient pas ta conversation — donne la demande (mots de Seb), les décisions prises, les fichiers concernés, et ce que tu attends (« BLOQUANTS, IMPORTANTS, mineurs, avec fichier:ligne »).
Ce que tu fais de leur réponse : chaque bloquant corrigé avant de livrer ; chaque important corrigé, ou dit à Seb avec la raison ; vérifie ce qui te surprend (un expert peut se tromper) ; DIS-LE à Seb dans le compte rendu.

## 4. Livrer : la boucle complète
1. Le lot est inscrit dans `docs/SUIVI.md`, avec ses exigences atomiques.
2. Plan → expert(s) → Seb si une décision lui revient.
3. Code ET tests. Chaque fonction a son test ; un test ne touche jamais les vraies données de Seb.
4. Revue par le ou les experts. Corriger.
5. La suite complète passe. Un test rouge s'analyse tout de suite, même « déjà rouge avant ».
6. UN essai réel, petit (un jeu, pas toute la ludothèque). Puis le reste s'il est bon. Jamais d'essais répétés sur la vraie machine de Seb.
7. Vérifier le RÉSULTAT, pas le retour : ouvre le fichier écrit, regarde le processus, compte les octets, affiche l'écran.
8. Version, notes de version lisibles par Seb, commit ciblé (jamais « tout ajouter »), suivi coché dans le même commit.
9. Compte rendu à Seb en trois parties honnêtes : fait, vérifié (et comment), PAS vérifié. Jamais « ça devrait marcher ».

## 5. Les leçons : ne jamais payer deux fois la même erreur
`docs/LECONS.md` : chaque erreur qui a coûté du temps, chaque correction de Seb, y entre LE JOUR MÊME, au format :
## 03/10 — Une ROM renommée n'est plus reconnue
Ce qui s'est passé : … · Pourquoi c'est grave : … · Comment l'appliquer : …
Tu le relis au début de chaque session et avant de toucher au domaine d'une leçon ; tes experts le lisent aussi. Une leçon périmée se corrige ou se supprime.

## 6. Parler à Firehouse
Le contrat = `docs/EXEMPLES-API-JEUX-V1.md` (réponses RÉELLES) : un champ absent n'existe pas. Un besoin nouveau s'écrit « Besoin n° N : ce que je veux faire, ce qu'il me manque, un exemple de réponse souhaitée », passe par Seb ou par ce canal, et entre dans ton suivi (⏳). Relis le contrat avant de toucher à la synchronisation : il s'enrichit souvent.

## 7. Parler à Seb
En français, court, sans jargon ; Seb mène le rythme (ne lui propose pas d'arrêter). Annonce AVANT le délicat : registre, pilotes, droits administrateur, comptes des boutiques, suppressions, téléchargements, opérations de masse (chiffrées : combien, combien de Go, combien de temps). Une question à la fois, avec ta recommandation. Quand tu t'es trompé : dis-le simplement et corrige.

## La liste du début de session
1. Relire `CLAUDE.md` puis ce document. 2. Relire `docs/SUIVI.md` EN ENTIER et `docs/LECONS.md`. 3. `git log --oneline -15`. 4. Reprendre le lot en cours, ou demander à Seb.
