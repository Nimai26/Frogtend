# Besoins de Frogtend envers l'API Firehouse `/api/jeux/v1/`

> Les points que le contrat (brief § 3) ne couvre pas encore. Seb les transmet à Firehouse ; Frogtend n'invente
> jamais une route de son côté.

| # | Date | Besoin | Pourquoi |
|---|---|---|---|
| 1 | 29/09/2026 | Une route « qui suis-je » (ex. `GET /api/jeux/v1/moi`) qui renvoie le nom affiché et le grade du porteur du jeton | Afficher le profil connecté et vérifier un jeton au moment où on le colle, sans charger tout le catalogue |
| 2 | 29/09/2026 | Une version d'API annoncée (en-tête ou route) | Frogtend est installé sur plusieurs PC et mis à jour à des rythmes différents : il doit détecter un serveur trop récent ou trop ancien et le dire |
| 3 | 29/09/2026 | Le moyen pour un PC autre que Venkman d'obtenir un jeton d'appareil (création dans le cockpit, puis copier-coller ? appairage par code ?) | Frogtend n'est pas lié à Venkman |
| 4 | 29/09/2026 | `POST /session` : Frogtend l'envoie depuis n'importe quel PC. C'est Firehouse qui décide s'il doit céder une carte graphique (seulement pour Venkman) | Frogtend ne sait pas, et n'a pas à savoir, quelle machine calcule pour Firehouse |
| 5 | 29/09/2026 | Pour les images de `/media`, un `ETag` ou une date de modification, et, dans le catalogue, une indication qu'une jaquette a changé | Garder les jaquettes en cache sans les retélécharger à chaque synchronisation |
| 6 | 29/09/2026 | Dans la synchronisation incrémentale (`depuis`), la liste des jeux **retirés** ou devenus invisibles pour ce jeton | Sinon le cache local garde des jeux que le profil ne doit plus voir |
| 7 | 29/09/2026 | Une route pour **enregistrer** le skin choisi (ex. `PUT /api/jeux/v1/theme {"theme": "<nom>"}`). La charte (§ 1) prévoit que Frogtend l'écrive dans Firehouse, mais le contrat ne contient que la lecture (`GET /theme`) | Proposer « enregistrer ce skin dans mon compte » sans passer par les routes de session du cockpit |
| 8 | 29/09/2026 | ✅ **Réglé** : exemples réels dans `docs/EXEMPLES-API-JEUX-V1.md` (Firehouse 2.11.2). Frogtend les lit, testé contre eux et contre le vrai serveur le 29/09. | — |
| 9 | 29/09/2026 | La vidéo de fond du skin `firehouse` : `jeux.hikari-no-sekai.fr` ne publie que `/api/jeux/v1/`, donc `/static/img/firehouse-bg.webm` (confirmé dans la réponse de `/theme`) n'y est pas joignable. Servir la vidéo sous `/api/jeux/v1/` (par exemple `/media/skin/firehouse/video`) ? | Proposer le fond vidéo hors de la maison |
| 10 | 29/09/2026 | Les skins avant l'ouverture d'un profil : `/themes` exige un jeton, alors l'écran « Qui joue ? » affiche l'instantané fourni avec Frogtend. Accepté tel quel, ou `/themes` public (les skins ne sont pas secrets) ? | Afficher les vrais skins dès l'écran d'accueil |
| 11 | 29/09/2026 | Le **développeur** (et l'éditeur) dans chaque jeu de `/catalogue` | LaunchBox affiche le développeur sous chaque jaquette ; aujourd'hui il n'est que dans la fiche complète, alors Frogtend affiche l'année |
| 12 | 29/09/2026 | Une **miniature** de jaquette (par exemple `/media/{id}/jaquette?largeur=400`) | La jaquette de Dune pèse 3 Mo : sur une grille de centaines de jeux, et hors de la maison, c'est très lourd. Frogtend garde l'image en cache, mais le premier affichage coûte cher |
