# 🐸 Frogtend

La ludothèque de la maison, pour Windows : afficher, installer et lancer les jeux servis par **Firehouse**, en natif
ou par émulateur. Taodbox, son mode canapé, se pilote entièrement à la manette.

> Projet familial en cours de construction. Le plan est dans [`docs/PLAN.md`](docs/PLAN.md), les nouveautés dans
> [`docs/NOTES-DE-VERSION.md`](docs/NOTES-DE-VERSION.md).

## Installer

Télécharger `Frogtend_<version>_x64-setup.exe` depuis les
[releases](https://github.com/Nimai26/Frogtend/releases). L'installateur n'a pas besoin des droits administrateur.
Windows peut afficher « éditeur inconnu » : « Informations complémentaires » puis « Exécuter quand même ». Frogtend se
met ensuite à jour tout seul, après ton accord.

## Développer

Il faut Node 20+, pnpm, Rust (stable, MSVC) et les Visual Studio Build Tools. Tout reste dans le projet.

| Commande | Rôle |
|---|---|
| `pnpm install` | installer les dépendances du projet |
| `pnpm tauri dev` | lancer Frogtend en développement |
| `pnpm test:tout` | la suite complète : types, tests de l'interface (Vitest), tests du cœur (cargo) |
| `pnpm version:appliquer 0.2.0` | changer de version (le fichier `VERSION` est la seule source) |
| `pnpm livrer` | tests, puis installateur signé et `latest.json` à joindre à la release GitHub |

La clé privée qui signe les mises à jour n'est **jamais** dans le dépôt (voir `scripts/livrer.mjs`).

## Crédits

Les skins viennent de Firehouse. Leurs palettes sont reprises de **Theme.Park** (GilbN, GPL-3.0).
Licence de Frogtend : Apache 2.0.
