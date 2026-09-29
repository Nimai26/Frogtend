# Notes de version de Frogtend

## 0.2.0 — la ludothèque (lot 1)

Première version utilisable : Frogtend affiche la ludothèque de Firehouse.

- **Qui joue ?** Au lancement, chacun choisit son profil. Un profil peut être protégé par un **code PIN**
  (après 5 erreurs, il est bloqué une minute). Chaque profil a son **jeton Firehouse**, rangé dans le coffre de
  Windows, jamais affiché.
- **La ludothèque, façon LaunchBox** : les plateformes à gauche (classées par catégorie), les jaquettes au centre,
  les détails du jeu choisi à droite, avec « 🎲 Jeu au hasard ». La recherche ignore les accents ; un filtre par
  genre et trois tris (titre, année). En haut : « Affichage de X jeux sur Y ».
- **La fiche d'un jeu** : le résumé, les informations, les documents à lire (lancement, solution, astuces…), puis
  chaque version avec ses fichiers, leur taille et ses notes.
- **Hors ligne** : la ludothèque déjà synchronisée et les fiches déjà lues restent consultables.
- **Les skins de Firehouse** : Frogtend applique celui que tu as choisi dans Firehouse ; tu peux en prendre un
  autre dans « Réglages ».
- **Réglages de la ludothèque** : taille des jaquettes, ligne sous le titre, tri, colonnes affichées, plateformes
  masquées. **Mon profil** : nom, code PIN, jeton, suppression.
- **Adresse de Firehouse** par défaut : `https://jeux.hikari-no-sekai.fr` (HTTPS obligatoire hors de la maison).
  Un mode simulé (des exemples, sans connexion) reste disponible.
- Pas encore : télécharger, installer et lancer un jeu (lot 2 et lot 3).

## 0.1.0 — le socle (lot 0)

Premier Frogtend installable. Il n'affiche pas encore de jeux : c'est le lot 1.

- **Installation** : un installateur Windows en français, sans droits administrateur. Au premier lancement, Windows
  peut afficher « éditeur inconnu » (pas de certificat, c'est voulu) : « Informations complémentaires » puis
  « Exécuter quand même ».
- **Mises à jour** : au démarrage, Frogtend regarde s'il existe une nouvelle version. S'il y en a une, il la
  propose et ne l'installe qu'après ton accord. La recherche se lance aussi à la main depuis « À propos ».
- **Skins** : les 44 skins de Firehouse. Par défaut, Frogtend suivra celui choisi dans ton compte Firehouse (quand
  l'API sera prête) ; tu peux en choisir un autre dans « Réglages ».
- **Réglages** : taille du texte, animations, adresse de Firehouse (par défaut `https://jeux.hikari-no-sekai.fr`).
  Chaque réglage peut revenir à sa valeur d'origine.
- **Mode simulé** : tant que l'API de Firehouse pour Frogtend est en construction, Frogtend ne se connecte à rien.
