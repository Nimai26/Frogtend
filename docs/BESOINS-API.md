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
