# Exemples RÉELS de l'API jeux v1 (relevés le 29/09/2026 sur Firehouse 2.11.2)

Base publique : `https://jeux.hikari-no-sekai.fr/api/jeux/v1` — en-tête `Authorization: Bearer <jeton>`.
Le jeton se crée dans le cockpit : ⬇️ DL ▸ 🧩 Extension navigateur ▸ « + Nouveau jeton » (Seb te le donne ;
il n'est JAMAIS écrit dans le dépôt). Relevé avec un jeton temporaire, révoqué aussitôt.

## `GET /plateformes` → 200

```json
{
  "ok": true,
  "plateformes": [
    {
      "nom": "MS-DOS",
      "jeux": 1
    },
    {
      "nom": "Nintendo 64",
      "jeux": 1
    }
  ]
}
```

## `GET /catalogue?page=1&par_page=2` → 200

```json
{
  "ok": true,
  "total": 2,
  "page": 1,
  "par_page": 2,
  "suivante": null,
  "jeux": [
    {
      "id": 110,
      "titre": "Dune",
      "annee": 1992,
      "plateforme": "MS-DOS",
      "genres": [
        "Aventure",
        "Stratégie"
      ],
      "statut": "possede",
      "maj_le": "2026-09-29T01:44:40",
      "jaquette": true,
      "versions": 1
    },
    {
      "id": 111,
      "titre": "The Legend of Zelda: Ocarina of Time",
      "annee": 1998,
      "plateforme": "Nintendo 64",
      "genres": [
        "Action",
        "Aventure"
      ],
      "statut": "recherche",
      "maj_le": "2026-09-29T03:06:55",
      "jaquette": true,
      "versions": 0
    }
  ]
}
```

## `GET /jeu/110` → 200

```json
{
  "ok": true,
  "jeu": {
    "id": 110,
    "titre": "Dune",
    "titre_en": "Dune",
    "annee": 1992,
    "statut": "possede",
    "resume": "En l'an de grâce 10191, la planète Arrakis, plus connue sous le nom de Dune et source unique de l'Epice, est au centre de tous les conflits. Les nobles Atréides et les belliqueux Harkonnen s'affrontent depuis des générations pour obtenir le contrôle de cette planète aride et hostile.\n\nVous jouez le rôle de Paul Atréides. L'histoire commence lorsque votre père, le Duc Leto, se voit confier le contrôle de Dune par l'Empereur Padishah. Vous devrez, dans un premier temps, prospecter l'Epice pour le compte de l'Empire. Mais, très vite, les évènements vont s'accélérer. Les Harkonnen ne sont pas loin et complotent contre votre famille.\n\nPour leur faire face, il vous faudra gagner la confiance des mystérieux Fremen, le peuple du Désert, et percer leur secret. Un secret qui pourrait changer la face d'Arrakis...\n\nSorti en 1992, Dune fut la toute première adaptation de l'oeuvre de Frank Herbert en jeu vidéo. Plus inspirée par le film de David Lynch que par le livre lui-même, l'équipe de Cryo a su restituer l'ambiance unique de la planète Arrakis. Les graphismes en 256 couleurs, grandioses pour l'époque, gardent un charme onirique qui ne laisse pas indifférent. Dune vous propose une galerie de personnages inoubliables, de conquérir Arrakis à la manière d'un jeu de stratégie, de traverser la planète en ornithoptère, ou sur le dos d'un ver des sables. Un excellent jeu d'aventure, mais dont les nombreuses qualités sont quelque peu ternies par une interactivité limitée et une aventure linéaire.",
    "genres": [
      "Aventure",
      "Stratégie"
    ],
    "affiche": "326547a74b2c0c437a8d6fc5.jpg",
    "plateforme": "MS-DOS",
    "developpeur": "Cryo Interactive Entertainment",
    "editeur": "Virgin Games",
    "joueurs": "1",
    "cooperatif": false,
    "video": "",
    "launchbox_id": 14133,
    "versions": [
      {
        "telechargement_id": 200,
        "nom": "Version automatique - Jeu CD-ROM avec voix françaises [Abandonware France]",
        "qualite": "Jeu — prêt à jouer",
        "type_source": "ddl",
        "range_le": "2026-09-29T00:40:00",
        "notes": "Le jeu en version CD-ROM, préinstallé et préconfiguré\ndans Dosbox pour être utilisé sous un Windows moderne.\nCette version a été modifiée pour inclure le doublage\nen français de la version console Mega-CD de Sega.\n*Les sous-titres demeurent multilingues.\nMaj 24/08/2019 : Inclus le doublage VF des extraits du film.\nMaj 08/09/2020 : Doublage d'une qualité sonore supérieure.\nMaj 12/10/2025 : Musiques AdLib Gold + Module Surround.",
        "notes_origine": "",
        "notes_a_traduire": false,
        "fichiers": [
          {
            "nom": "Dune (1992) [MS-DOS].exe",
            "taille": 237887038
          }
        ]
      }
    ],
    "annexes": [
      {
        "i": 0,
        "cle": "abandonware_france:manuel:Manuel de Dune",
        "type": "manuel",
        "titre": "Manuel de Dune",
        "taille": 10200711,
        "source": "abandonware_france",
        "texte": false,
        "a_traduire": false
      },
      {
        "i": 1,
        "cle": "abandonware_france:lancement:Lancement sous DOSBox",
        "type": "lancement",
        "titre": "Lancement sous DOSBox",
        "taille": 324,
        "source": "abandonware_france",
        "texte": true,
        "a_traduire": false
      },
      {
        "i": 2,
        "cle": "abandonware_france:solution:Solution",
        "type": "solution",
        "titre": "Solution",
        "taille": 8897,
        "source": "abandonware_france",
        "texte": true,
        "a_traduire": false
      },
      {
        "i": 3,
        "cle": "abandonware_france:astuces:Astuces",
        "type": "astuces",
        "titre": "Astuces",
        "taille": 6517,
        "source": "abandonware_france",
        "texte": true,
        "a_traduire": false
      },
      {
        "i": 4,
        "cle": "abandonware_france:presse:Salons, récompenses et presse",
        "type": "presse",
        "titre": "Salons, récompenses et presse",
        "taille": 6552,
        "source": "abandonware_france",
        "texte": true,
        "a_traduire": false
      }
    ],
    "annexes_lues_le": "2026-09-29T01:09:30"
  }
}
```

## `GET /themes` → 200

```json
{
  "ok": true,
  "version": "58f6aa1e85098f91",
  "credit": "Palettes issues de Theme.Park (GilbN, GPL-3.0) — remappées sur les variables Firehouse. Valeurs de couleurs = données ; usage perso.",
  "defaut": "firehouse",
  "communs": {
    "--bg": "#0a0e14",
    "--bg2": "#111722",
    "--panel": "#151c28",
    "--line": "#232c3b",
    "--ink": "#e6ecf5",
    "--dim": "#8a97ab",
    "--accent": "#ff7a3c",
    "--accent2": "#3ca6ff",
    "--ok": "#46c98b",
    "--warn": "#e6b23c",
    "--err": "#e05a5a",
    "--encre-sombre": "#1a0e08",
    "--encre-claire": "#ffffff",
    "--on-accent": "var(--encre-sombre)",
    "--on-accent2": "var(--encre-sombre)",
    "--on-ok": "var(--encre-sombre)",
    "--on-err": "var(--encre-sombre)",
    "--on-warn": "var(--encre-sombre)",
    "--ok-texte": "var(--ok)",
    "--warn-texte": "var(--warn)",
    "--err-texte": "var(--err)",
    "--accent-texte": "var(--accent)",
    "--accent2-texte": "var(--accent2)",
    "--border": "var(--line)",
    "--bord": "var(--line)",
    "--bd": "var(--line)",
    "--surface": "var(--panel)",
    "--bg3": "var(--bg2)",
    "--muted": "var(--dim)",
    "--ac": "var(--accent)",
    "--bad": "var(--err)",
    "--fond-media": "#111111",
    "--fond-page": "#ffffff",
    "--encre-voile": "#ffffff",
    "--encre-voile-dim": "#bbbbbb",
    "--encre-voile-note": "#ffd479",
    "--encre-voile-ok": "#8fd19e",
    "--voile-video": "rgba(6, 9, 14, .5)",
    "--voile-leger": "rgba(0, 0, 0, .4)",
    "--voile": "rgba(0, 0, 0, .62)",
    "--voile-fort": "rgba(0, 0, 0, .86)",
    "--voile-opaque": "rgba(0, 0, 0, .94)",
    "--ombre-noire": "rgba(0, 0, 0, .5)",
    "--damier-clair": "#bbbbbb",
    "--damier-fonce": "#888888",
    "--effet-metal": "#9aa4b2",
    "--effet-arcenciel": "#e056fd",
    "--effet-phospho": "#7bed9f",
    "--radius": "12px",
    "--mono": "ui-monospace, \"SF Mono\", Menlo, Consolas, monospace"
  },
  "resolus_ok": true,
  "erreur": "",
  "themes": {
    "firehouse": {
      "base": {
        "--bg": "radial-gradient(1200px 800px at 70% -10%, #14202f 0%, #0a0e14 55%)",
        "--bg2": "#111722",
        "--panel": "#151c28",
        "--line": "#232c3b",
        "--ink": "#e6ecf5",
        "--dim": "#8a97ab",
        "--accent": "#ff7a3c",
        "--accent2": "#3ca6ff",
        "--ok": "#46c98b",
        "--warn": "#e6b23c",
        "--err": "#e05a5a"
      },
      "resolus": {
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": "/static/img/firehouse-bg.webm"
    },
    "aquamarine": {
      "base": {
        "--bg": "radial-gradient(ellipse at center, #47918a 0%, #0b3161 100%) center center/cover no-repeat fixed",
        "--bg2": "#265c74",
        "--panel": "rgba(20, 25, 35, 0.86)",
        "--line": "#47918a",
        "--ink": "#ddd",
        "--dim": "#8dcfc9",
        "--accent": "#009688",
        "--accent2": "#0ed2bf",
        "--ok": "#009688",
        "--warn": "#f39c12",
        "--err": "#e74c3c"
      },
      "resolus": {
        "--ok-texte": "rgb(188, 210, 208)",
        "--warn-texte": "rgb(228, 202, 160)",
        "--err-texte": "rgb(223, 199, 197)",
        "--accent-texte": "rgb(188, 210, 208)",
        "--accent2-texte": "rgb(138, 217, 209)",
        "--dim": "rgb(169, 212, 208)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "blackberry-abyss": {
      "base": {
        "--bg": "linear-gradient(to bottom, rgba(0, 0, 0, 0.45), rgba(0, 0, 0, 0), rgba(0, 0, 0, 0.35)) center center/cover no-repeat fixed, radial-gradient(at bottom center, rgba(0, 0, 0, 0.4) 0%, hsla(211, 18%, 45%, 0.55), hsla(211, 18%, 5%, 0)) center center/cover no-repeat fixed, linear-gradient(to right, hsla(211, 18%, 5%, 1), hsla(211, 18%, 45%, 1), hsla(211, 18%, 5%, 1)) center center/cover no-repeat fixed",
        "--bg2": "hsla(211, 18%, 20%, 0.95)",
        "--panel": "hsla(211, 18%, 15%, 0.95)",
        "--line": "rgba(130, 157, 185, 0.3)",
        "--ink": "#eee",
        "--dim": "#999",
        "--accent": "rgb(130, 157, 185)",
        "--accent2": "rgb(130, 157, 185)",
        "--ok": "#46c98b",
        "--warn": "#f39c12",
        "--err": "#e74c3c"
      },
      "resolus": {
        "--err-texte": "rgb(233, 133, 122)",
        "--accent-texte": "rgb(135, 161, 188)",
        "--accent2-texte": "rgb(135, 161, 188)",
        "--dim": "rgb(157, 157, 157)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "blackberry-amethyst": {
      "base": {
        "--bg": "radial-gradient(at top center, rgba(0, 0, 0, 0.25), hsla(0, 14%, 18%, 0.55), hsla(0, 18%, 5%, 0.9)) center center/cover no-repeat fixed, linear-gradient(to bottom, #df89de 0%, hsl(276, 100%, 3%) 100%) center center/cover no-repeat fixed",
        "--bg2": "rgba(60, 30, 60, 0.95)",
        "--panel": "rgba(50, 20, 50, 0.95)",
        "--line": "rgba(217, 164, 217, 0.3)",
        "--ink": "#eee",
        "--dim": "#999",
        "--accent": "rgb(199, 118, 197)",
        "--accent2": "rgb(199, 118, 197)",
        "--ok": "#46c98b",
        "--warn": "#f39c12",
        "--err": "#e74c3c"
      },
      "resolus": {
        "--err-texte": "rgb(233, 117, 105)",
        "--accent-texte": "rgb(201, 124, 199)",
        "--accent2-texte": "rgb(201, 124, 199)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "blackberry-carol": {
      "base": {
        "--bg": "linear-gradient(45deg, #0b5019 0%, #a70606 135%) center center/cover no-repeat fixed",
        "--bg2": "rgba(51, 0, 0, 0.95)",
        "--panel": "rgba(30, 10, 10, 0.95)",
        "--line": "rgba(170, 170, 170, 0.3)",
        "--ink": "#eee",
        "--dim": "#999",
        "--accent": "#a70606",
        "--accent2": "#aaa",
        "--ok": "#46c98b",
        "--warn": "#f39c12",
        "--err": "#e74c3c"
      },
      "resolus": {
        "--accent-texte": "rgb(195, 99, 99)",
        "--on-accent": "var(--encre-claire)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "blackberry-dreamscape": {
      "base": {
        "--bg": "linear-gradient(to top, #e2c9cc 1%, #e7627d 46%, #b8235a 59%, #801357 71%, #3d1635 84%, #1c1a27 100%) center center/cover no-repeat fixed",
        "--bg2": "rgba(80, 30, 60, 0.95)",
        "--panel": "rgba(61, 22, 53, 0.95)",
        "--line": "rgba(231, 98, 125, 0.3)",
        "--ink": "#eee",
        "--dim": "#999",
        "--accent": "#e7627d",
        "--accent2": "rgb(230, 125, 146)",
        "--ok": "#46c98b",
        "--warn": "#f39c12",
        "--err": "#e74c3c"
      },
      "resolus": {
        "--err-texte": "rgb(233, 133, 122)",
        "--accent-texte": "rgb(233, 133, 153)",
        "--accent2-texte": "rgb(230, 131, 151)",
        "--dim": "rgb(162, 162, 162)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "blackberry-flamingo": {
      "base": {
        "--bg": "radial-gradient(at bottom center, rgba(0, 0, 0, 0.15), hsla(0, 14%, 18%, 0.65), hsla(0, 18%, 5%, 0.95)) center center/cover no-repeat fixed, linear-gradient(45deg, #ff9a9e 0%, #fad0c4 99%, #fad0c4 100%) center center/cover no-repeat fixed",
        "--bg2": "rgba(60, 30, 40, 0.95)",
        "--panel": "rgba(40, 20, 25, 0.95)",
        "--line": "rgba(250, 208, 196, 0.3)",
        "--ink": "#eee",
        "--dim": "#999",
        "--accent": "#ff9a9e",
        "--accent2": "#fad0c4",
        "--ok": "#46c98b",
        "--warn": "#f39c12",
        "--err": "#e74c3c"
      },
      "resolus": {
        "--err-texte": "rgb(233, 117, 105)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "blackberry-hearth": {
      "base": {
        "--bg": "radial-gradient(at bottom center, rgba(0, 0, 0, 0.4) 0%, hsla(0, 14%, 18%, 0.55), hsla(0, 18%, 5%, 0)) center center/cover no-repeat fixed, linear-gradient(to bottom, hsl(0, 18%, 5%), hsl(0, 65%, 23%), hsl(0, 100%, 9%)) center center/cover no-repeat fixed",
        "--bg2": "rgba(70, 20, 20, 0.95)",
        "--panel": "rgba(50, 15, 15, 0.95)",
        "--line": "rgba(236, 106, 106, 0.3)",
        "--ink": "#eee",
        "--dim": "#999",
        "--accent": "rgb(236, 106, 106)",
        "--accent2": "#fad0c4",
        "--ok": "#46c98b",
        "--warn": "#f39c12",
        "--err": "#e74c3c"
      },
      "resolus": {
        "--err-texte": "rgb(232, 100, 87)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "blackberry-martian": {
      "base": {
        "--bg": "linear-gradient(to right, rgba(0, 0, 0, 0.15), rgba(0, 0, 0, 0), rgba(0, 0, 0, 0.45)) center center/cover no-repeat fixed, radial-gradient(at right center, rgba(0, 0, 0, 0.3) 10%, hsla(0, 14%, 18%, 0.6), hsla(0, 18%, 5%, 1) 95%) center center/cover no-repeat fixed, linear-gradient(to right, #43e97b 0%, #043815 100%) center center/cover no-repeat fixed",
        "--bg2": "rgba(20, 50, 30, 0.95)",
        "--panel": "rgba(10, 40, 20, 0.95)",
        "--line": "rgba(67, 233, 123, 0.3)",
        "--ink": "#eee",
        "--dim": "#999",
        "--accent": "#43e97b",
        "--accent2": "#43e97b",
        "--ok": "#46c98b",
        "--warn": "#f39c12",
        "--err": "#e74c3c"
      },
      "resolus": {
        "--err-texte": "rgb(233, 125, 113)",
        "--dim": "rgb(157, 157, 157)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "blackberry-pumpkin": {
      "base": {
        "--bg": "radial-gradient(at center center, #ff8a00 50%, #37033a 100%) center center/cover no-repeat fixed",
        "--bg2": "rgba(60, 25, 40, 0.95)",
        "--panel": "rgba(40, 15, 30, 0.95)",
        "--line": "rgba(255, 138, 0, 0.3)",
        "--ink": "#eee",
        "--dim": "#999",
        "--accent": "#ff8a00",
        "--accent2": "#ff8a00",
        "--ok": "#46c98b",
        "--warn": "#f39c12",
        "--err": "#e74c3c"
      },
      "resolus": {
        "--err-texte": "rgb(232, 108, 96)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "blackberry-royal": {
      "base": {
        "--bg": "linear-gradient(to bottom, rgba(255, 255, 255, 0.2) 0%, rgba(0, 0, 0, 0.7) 100%) center center/cover no-repeat fixed, radial-gradient(at top center, rgba(255, 255, 255, 0.3) 0%, rgba(0, 0, 0, 0.40) 120%) #000 center center/cover no-repeat fixed",
        "--bg2": "rgba(50, 40, 35, 0.95)",
        "--panel": "rgba(30, 25, 20, 0.95)",
        "--line": "rgba(185, 170, 159, 0.3)",
        "--ink": "#eee",
        "--dim": "#999",
        "--accent": "rgb(185, 170, 159)",
        "--accent2": "rgb(185, 170, 159)",
        "--ok": "#46c98b",
        "--warn": "#f39c12",
        "--err": "#e74c3c"
      },
      "resolus": {
        "--err-texte": "rgb(233, 133, 122)",
        "--dim": "rgb(157, 157, 157)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "blackberry-shadow": {
      "base": {
        "--bg": "linear-gradient(135deg, #252b2f, #090c0e) center center/cover no-repeat fixed",
        "--bg2": "rgba(35, 40, 45, 0.95)",
        "--panel": "rgba(25, 30, 35, 0.95)",
        "--line": "rgba(81, 101, 114, 0.3)",
        "--ink": "#eee",
        "--dim": "#999",
        "--accent": "rgb(96, 128, 150)",
        "--accent2": "rgb(96, 128, 150)",
        "--ok": "#46c98b",
        "--warn": "#f39c12",
        "--err": "#e74c3c"
      },
      "resolus": {
        "--err-texte": "rgb(232, 100, 87)",
        "--accent-texte": "rgb(124, 150, 168)",
        "--accent2-texte": "rgb(124, 150, 168)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "blackberry-solar": {
      "base": {
        "--bg": "radial-gradient(at top left, rgba(0, 0, 0, 0.31), hsla(0, 14%, 18%, 0.9), hsla(0, 18%, 5%, 1)) center center/cover no-repeat fixed, linear-gradient(120deg, #f6d365 0%, #a25a25 100%) center center/cover no-repeat fixed",
        "--bg2": "hsla(0, 18%, 10%, 0.95)",
        "--panel": "rgba(20, 25, 35, 0.86)",
        "--line": "rgba(246, 211, 101, 0.3)",
        "--ink": "#eee",
        "--dim": "#999",
        "--accent": "#f6d365",
        "--accent2": "rgb(246, 211, 101)",
        "--ok": "#6b5",
        "--warn": "#f6d365",
        "--err": "#e74c3c"
      },
      "resolus": {
        "--err-texte": "rgb(233, 125, 113)",
        "--dim": "rgb(157, 157, 157)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "blackberry-vanta": {
      "base": {
        "--bg": "#000",
        "--bg2": "#2d2d2d",
        "--panel": "#181818",
        "--line": "rgba(170, 170, 170, 0.3)",
        "--ink": "#eee",
        "--dim": "#999",
        "--accent": "#7a7a7a",
        "--accent2": "#7a7a7a",
        "--ok": "#46c98b",
        "--warn": "#f39c12",
        "--err": "#e74c3c"
      },
      "resolus": {
        "--err-texte": "rgb(233, 117, 105)",
        "--accent-texte": "rgb(151, 151, 151)",
        "--accent2-texte": "rgb(151, 151, 151)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "catppuccin-frappe": {
      "base": {
        "--bg": "#303446",
        "--bg2": "#414559",
        "--panel": "#292c3c",
        "--line": "rgba(140, 170, 238, 0.3)",
        "--ink": "#c6d0f5",
        "--dim": "#a5adce",
        "--accent": "#8caaee",
        "--accent2": "#8caaee",
        "--ok": "#a6d189",
        "--warn": "#ef9f76",
        "--err": "#e78284"
      },
      "resolus": {
        "--warn-texte": "rgb(235, 164, 131)",
        "--err-texte": "rgb(215, 169, 189)",
        "--accent-texte": "rgb(155, 180, 240)",
        "--accent2-texte": "rgb(155, 180, 240)",
        "--dim": "rgb(170, 178, 212)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "catppuccin-latte": {
      "base": {
        "--bg": "#eff1f5",
        "--bg2": "#ccd0da",
        "--panel": "#e6e9ef",
        "--line": "rgba(76, 79, 105, 0.2)",
        "--ink": "#4c4f69",
        "--dim": "#6c6f85",
        "--accent": "#1e66f5",
        "--accent2": "#1e66f5",
        "--ok": "#40a02b",
        "--warn": "#df8e1d",
        "--err": "#d20f39"
      },
      "resolus": {
        "--ok-texte": "rgb(74, 91, 96)",
        "--warn-texte": "rgb(91, 85, 97)",
        "--err-texte": "rgb(163, 37, 74)",
        "--accent-texte": "rgb(60, 87, 154)",
        "--accent2-texte": "rgb(60, 87, 154)",
        "--dim": "rgb(84, 87, 112)",
        "--on-accent": "var(--encre-claire)",
        "--on-accent2": "var(--encre-claire)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-claire)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "catppuccin-macchiato": {
      "base": {
        "--bg": "#24273a",
        "--bg2": "#363a4f",
        "--panel": "#1e2030",
        "--line": "rgba(138, 173, 244, 0.3)",
        "--ink": "#cad3f5",
        "--dim": "#a5adcb",
        "--accent": "#8aadf4",
        "--accent2": "#8aadf4",
        "--ok": "#a6da95",
        "--warn": "#f5a97f",
        "--err": "#ed8796"
      },
      "resolus": {
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "catppuccin-mocha": {
      "base": {
        "--bg": "#1e1e2e",
        "--bg2": "#313244",
        "--panel": "#181825",
        "--line": "#45475a",
        "--ink": "#cdd6f4",
        "--dim": "#a6adc8",
        "--accent": "#89b4fa",
        "--accent2": "#cdd6f4",
        "--ok": "#a6e3a1",
        "--warn": "#fab387",
        "--err": "#f38ba8"
      },
      "resolus": {
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "dark": {
      "base": {
        "--bg": "radial-gradient(circle, #3a3a3a, #2d2d2d, #202020, #141414, #000000) center center/cover no-repeat fixed",
        "--bg2": "#2d2d2d",
        "--panel": "rgba(20, 25, 35, 0.86)",
        "--line": "#444",
        "--ink": "#ddd",
        "--dim": "#999",
        "--accent": "#4a90d9",
        "--accent2": "#7a7a7a",
        "--ok": "#6b5",
        "--warn": "#f39c12",
        "--err": "#e74c3c"
      },
      "resolus": {
        "--err-texte": "rgb(228, 120, 108)",
        "--accent-texte": "rgb(89, 152, 217)",
        "--accent2-texte": "rgb(152, 152, 152)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "dracula": {
      "base": {
        "--bg": "#282a36",
        "--bg2": "#1e2029",
        "--panel": "#1e2029",
        "--line": "#44475a",
        "--ink": "#f8f8f2",
        "--dim": "#6272a4",
        "--accent": "#bd93f9",
        "--accent2": "#ff79c6",
        "--ok": "#50fa7b",
        "--warn": "#ffb86c",
        "--err": "#ff5555"
      },
      "resolus": {
        "--dim": "rgb(121, 134, 176)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "hotline": {
      "base": {
        "--bg": "linear-gradient(0deg, rgba(247, 101, 184, 1) 0%, rgb(21, 95, 165) 100%) center center/cover no-repeat fixed",
        "--bg2": "#5e61ab",
        "--panel": "rgba(20, 25, 35, 0.86)",
        "--line": "rgba(255, 255, 255, 0.3)",
        "--ink": "#ddd",
        "--dim": "#bbb",
        "--accent": "#f98dc9",
        "--accent2": "rgb(255, 179, 222)",
        "--ok": "#00ff9d",
        "--warn": "#ffb86c",
        "--err": "#ff4c4c"
      },
      "resolus": {
        "--ok-texte": "rgb(128, 255, 206)",
        "--warn-texte": "rgb(255, 227, 196)",
        "--err-texte": "rgb(255, 228, 228)",
        "--accent-texte": "rgb(254, 227, 242)",
        "--accent2-texte": "rgb(255, 225, 242)",
        "--dim": "rgb(235, 235, 235)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "hotpink": {
      "base": {
        "--bg": "linear-gradient(45deg, #fb3f62 0%, #204c80 37%, #004249 97%) center center/cover no-repeat fixed",
        "--bg2": "#204c80",
        "--panel": "rgba(20, 25, 35, 0.86)",
        "--line": "rgba(251, 63, 98, 0.5)",
        "--ink": "#eee",
        "--dim": "#999",
        "--accent": "#fb3f62",
        "--accent2": "rgb(0, 255, 157)",
        "--ok": "#00ff9d",
        "--warn": "#ffb86c",
        "--err": "#ff3333"
      },
      "resolus": {
        "--err-texte": "rgb(244, 173, 173)",
        "--accent-texte": "rgb(243, 168, 182)",
        "--dim": "rgb(187, 187, 187)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "ibracorp": {
      "base": {
        "--bg": "#262a2b",
        "--bg2": "#1b1b1b",
        "--panel": "#333",
        "--line": "#444",
        "--ink": "#d16057",
        "--dim": "#999",
        "--accent": "#ef7a70",
        "--accent2": "#ef7a70",
        "--ok": "#ef7a70",
        "--warn": "#f39c12",
        "--err": "#d16057"
      },
      "resolus": {
        "--err-texte": "rgb(221, 136, 129)",
        "--dim": "rgb(158, 158, 158)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "infinity-mind": {
      "base": {
        "--bg": "radial-gradient(ellipse at center bottom, rgba(255, 242, 0, 0.7) 0%, #0d0400 80%, rgba(0, 0, 0, 1) 100%) center center/cover no-repeat fixed",
        "--bg2": "rgba(51, 49, 0, 0.95)",
        "--panel": "rgba(20, 25, 35, 0.86)",
        "--line": "rgba(228, 216, 0, 0.3)",
        "--ink": "#ddd",
        "--dim": "#999",
        "--accent": "#fff200",
        "--accent2": "#fff200",
        "--ok": "#46c98b",
        "--warn": "#e1d500",
        "--err": "#e74c3c"
      },
      "resolus": {
        "--err-texte": "rgb(227, 141, 132)",
        "--dim": "rgb(163, 163, 163)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "infinity-power": {
      "base": {
        "--bg": "radial-gradient(ellipse at center bottom, rgba(166, 40, 140, 0.7) 0%, rgba(11, 8, 51, 1) 80%, rgba(0, 0, 0, 1) 100%) center center/cover no-repeat fixed",
        "--bg2": "rgba(35, 0, 57, 0.95)",
        "--panel": "rgba(20, 25, 35, 0.86)",
        "--line": "rgba(166, 40, 140, 0.3)",
        "--ink": "#ddd",
        "--dim": "#999",
        "--accent": "#d816ae",
        "--accent2": "rgb(223, 21, 179)",
        "--ok": "#46c98b",
        "--warn": "#f39c12",
        "--err": "#ff0055"
      },
      "resolus": {
        "--err-texte": "rgb(245, 66, 126)",
        "--accent-texte": "rgb(218, 82, 188)",
        "--accent2-texte": "rgb(222, 81, 192)",
        "--on-accent": "var(--encre-claire)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "infinity-reality": {
      "base": {
        "--bg": "radial-gradient(ellipse at center bottom, rgba(232, 11, 11, 0.7) 0%, #08000d 80%, rgba(0, 0, 0, 1) 100%) center center/cover no-repeat fixed",
        "--bg2": "rgba(102, 5, 5, 0.95)",
        "--panel": "rgba(20, 25, 35, 0.86)",
        "--line": "rgba(232, 12, 11, 0.3)",
        "--ink": "#ddd",
        "--dim": "#999",
        "--accent": "#e80c0b",
        "--accent2": "rgb(232, 12, 11)",
        "--ok": "#46c98b",
        "--warn": "#f39c12",
        "--err": "#e80c0b"
      },
      "resolus": {
        "--err-texte": "rgb(226, 127, 127)",
        "--accent-texte": "rgb(226, 127, 127)",
        "--accent2-texte": "rgb(226, 127, 127)",
        "--dim": "rgb(156, 156, 156)",
        "--on-accent": "var(--encre-claire)",
        "--on-accent2": "var(--encre-claire)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-claire)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "infinity-soul": {
      "base": {
        "--bg": "radial-gradient(ellipse at center bottom, rgba(255, 153, 0, 0.7) 0%, #3c0015 80%, rgba(0, 0, 0, 1) 100%) center center/cover no-repeat fixed",
        "--bg2": "rgba(140, 64, 2, 0.95)",
        "--panel": "rgba(20, 25, 35, 0.86)",
        "--line": "rgba(255, 153, 0, 0.3)",
        "--ink": "#ddd",
        "--dim": "#999",
        "--accent": "#ff9900",
        "--accent2": "rgb(255, 153, 0)",
        "--ok": "#46c98b",
        "--warn": "#ff9900",
        "--err": "#880030"
      },
      "resolus": {
        "--ok-texte": "rgb(191, 217, 205)",
        "--warn-texte": "rgb(228, 207, 177)",
        "--err-texte": "rgb(217, 210, 212)",
        "--accent-texte": "rgb(228, 207, 177)",
        "--accent2-texte": "rgb(228, 207, 177)",
        "--dim": "rgb(211, 211, 211)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-claire)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "infinity-space": {
      "base": {
        "--bg": "radial-gradient(ellipse at center bottom, rgba(0, 98, 255, 0.7) 0%, #020013 80%, rgb(0, 0, 0) 100%) center center/cover no-repeat fixed",
        "--bg2": "rgba(0, 57, 148, 0.95)",
        "--panel": "rgba(20, 25, 35, 0.86)",
        "--line": "rgba(0, 98, 255, 0.3)",
        "--ink": "#ddd",
        "--dim": "#999",
        "--accent": "#0062ff",
        "--accent2": "rgb(61, 126, 255)",
        "--ok": "#46c98b",
        "--warn": "#f39c12",
        "--err": "#e74c3c"
      },
      "resolus": {
        "--err-texte": "rgb(226, 156, 149)",
        "--accent-texte": "rgb(144, 178, 233)",
        "--accent2-texte": "rgb(141, 174, 238)",
        "--dim": "rgb(177, 177, 177)",
        "--on-accent": "var(--encre-claire)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "infinity-time": {
      "base": {
        "--bg": "radial-gradient(ellipse at center bottom, rgba(109, 247, 81, 0.7) 0%, #00130c 80%, rgb(0, 0, 0) 100%) center center/cover no-repeat fixed",
        "--bg2": "rgba(2, 77, 0, 0.95)",
        "--panel": "rgba(20, 25, 35, 0.86)",
        "--line": "rgba(109, 247, 81, 0.3)",
        "--ink": "#ddd",
        "--dim": "#999",
        "--accent": "#6df751",
        "--accent2": "rgb(109, 247, 81)",
        "--ok": "#46c98b",
        "--warn": "#f39c12",
        "--err": "#e74c3c"
      },
      "resolus": {
        "--ok-texte": "rgb(93, 204, 151)",
        "--warn-texte": "rgb(238, 172, 69)",
        "--err-texte": "rgb(224, 178, 173)",
        "--dim": "rgb(184, 184, 184)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "maroon": {
      "base": {
        "--bg": "radial-gradient(circle farthest-corner at 48.4% 47.5%, rgba(76, 21, 51, 1) 0%, rgba(34, 10, 37, 1) 90%) center center/cover no-repeat fixed",
        "--bg2": "#220a25",
        "--panel": "rgba(20, 25, 35, 0.86)",
        "--line": "rgba(162, 28, 101, 0.5)",
        "--ink": "#dadada",
        "--dim": "#999",
        "--accent": "#7b154d",
        "--accent2": "rgb(162, 28, 101)",
        "--ok": "#6b5",
        "--warn": "#f39c12",
        "--err": "#e74c3c"
      },
      "resolus": {
        "--accent-texte": "rgb(171, 120, 148)",
        "--accent2-texte": "rgb(184, 104, 148)",
        "--on-accent": "var(--encre-claire)",
        "--on-accent2": "var(--encre-claire)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "nord": {
      "base": {
        "--bg": "#2E3440",
        "--bg2": "#333947",
        "--panel": "#3B4252",
        "--line": "#4C566A",
        "--ink": "#D8DEE9",
        "--dim": "#81A1C1",
        "--accent": "#5E81AC",
        "--accent2": "#81A1C1",
        "--ok": "#A3BE8C",
        "--warn": "#D08770",
        "--err": "#BF616A"
      },
      "resolus": {
        "--warn-texte": "rgb(211, 165, 154)",
        "--err-texte": "rgb(205, 166, 176)",
        "--accent-texte": "rgb(155, 176, 203)",
        "--accent2-texte": "rgb(155, 179, 205)",
        "--dim": "rgb(155, 179, 205)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "onedark": {
      "base": {
        "--bg": "#282c34",
        "--bg2": "#1e222a",
        "--panel": "#1e222a",
        "--line": "#3e4451",
        "--ink": "#abb2bf",
        "--dim": "#565c64",
        "--accent": "#61afef",
        "--accent2": "#61afef",
        "--ok": "#98c379",
        "--warn": "#e5c07b",
        "--err": "#e06c75"
      },
      "resolus": {
        "--dim": "rgb(133, 139, 150)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "organizr": {
      "base": {
        "--bg": "#1f1f1f",
        "--bg2": "#1b1b1b",
        "--panel": "#333",
        "--line": "#444",
        "--ink": "#96a2b4",
        "--dim": "#999",
        "--accent": "#2cabe3",
        "--accent2": "#2cabe3",
        "--ok": "#2cabe3",
        "--warn": "#f39c12",
        "--err": "#e74c3c"
      },
      "resolus": {
        "--err-texte": "rgb(158, 153, 168)",
        "--dim": "rgb(152, 155, 158)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "overseerr": {
      "base": {
        "--bg": "linear-gradient(360deg, hsl(221, 39%, 11%) 65%, hsl(215, 28%, 17%) 100%)",
        "--bg2": "#374151",
        "--panel": "#1f2937",
        "--line": "#374151",
        "--ink": "#d1d5db",
        "--dim": "#9ca3af",
        "--accent": "#4f46e5",
        "--accent2": "#6366f1",
        "--ok": "#10b981",
        "--warn": "#f59e0b",
        "--err": "#ef4444"
      },
      "resolus": {
        "--ok-texte": "rgb(64, 192, 152)",
        "--err-texte": "rgb(221, 155, 159)",
        "--accent-texte": "rgb(170, 170, 222)",
        "--accent2-texte": "rgb(165, 169, 228)",
        "--dim": "rgb(167, 173, 184)",
        "--on-accent": "var(--encre-claire)",
        "--on-accent2": "var(--encre-claire)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "pine-shadow": {
      "base": {
        "--bg": "linear-gradient(135deg, #252b2f, #090c0e) center center/cover no-repeat fixed",
        "--bg2": "#1a1d20",
        "--panel": "rgba(20, 25, 35, 0.86)",
        "--line": "#3a3f44",
        "--ink": "#bbb",
        "--dim": "#999",
        "--accent": "#e5a00d",
        "--accent2": "#fff",
        "--ok": "#27c24c",
        "--warn": "#e5a00d",
        "--err": "#e74c3c"
      },
      "resolus": {
        "--err-texte": "rgb(229, 82, 66)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "plex": {
      "base": {
        "--bg": "radial-gradient(circle farthest-side at 0% 100%, rgb(47, 47, 47) 0%, rgba(47, 47, 47, 0) 100%), radial-gradient(circle farthest-side at 100% 100%, rgb(63, 63, 63) 0%, rgba(63, 63, 63, 0) 100%), radial-gradient(circle farthest-side at 100% 0%, rgb(76, 76, 76) 0%, rgba(76, 76, 76, 0) 100%), radial-gradient(circle farthest-side at 0% 0%, rgb(58, 58, 58) 0%, rgba(58, 58, 58, 0) 100%), black center center/cover no-repeat fixed",
        "--bg2": "#191a1c",
        "--panel": "#282828",
        "--line": "#444",
        "--ink": "#ddd",
        "--dim": "#999",
        "--accent": "#e5a00d",
        "--accent2": "#e5a00d",
        "--ok": "#27c24c",
        "--warn": "#e5a00d",
        "--err": "#e74c3c"
      },
      "resolus": {
        "--err-texte": "rgb(229, 105, 92)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "rosepine": {
      "base": {
        "--bg": "#1f1d2e",
        "--bg2": "#21202e",
        "--panel": "#26233a",
        "--line": "rgba(235, 111, 146, 0.3)",
        "--ink": "#e0def4",
        "--dim": "#6e6a86",
        "--accent": "#eb6f92",
        "--accent2": "#31748f",
        "--ok": "#31748f",
        "--warn": "#f6c177",
        "--err": "#eb6f92"
      },
      "resolus": {
        "--ok-texte": "rgb(102, 148, 173)",
        "--accent2-texte": "rgb(102, 148, 173)",
        "--dim": "rgb(144, 141, 167)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-claire)",
        "--on-ok": "var(--encre-claire)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "rosepine-dawn": {
      "base": {
        "--bg": "#fffaf3",
        "--bg2": "#f4ede8",
        "--panel": "#f2e9de",
        "--line": "rgba(87, 82, 121, 0.2)",
        "--ink": "#575279",
        "--dim": "#9893a5",
        "--accent": "#b4637a",
        "--accent2": "#286983",
        "--ok": "#286983",
        "--warn": "#ea9d34",
        "--err": "#b4637a"
      },
      "resolus": {
        "--warn-texte": "rgb(116, 97, 107)",
        "--err-texte": "rgb(138, 91, 122)",
        "--accent-texte": "rgb(138, 91, 122)",
        "--dim": "rgb(107, 102, 134)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-claire)",
        "--on-ok": "var(--encre-claire)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "rosepine-moon": {
      "base": {
        "--bg": "#2a273f",
        "--bg2": "#2a283e",
        "--panel": "#393552",
        "--line": "rgba(235, 111, 146, 0.3)",
        "--ink": "#e0def4",
        "--dim": "#6e6a86",
        "--accent": "#eb6f92",
        "--accent2": "#3e8fb0",
        "--ok": "#3e8fb0",
        "--warn": "#f6c177",
        "--err": "#eb6f92"
      },
      "resolus": {
        "--ok-texte": "rgb(119, 171, 200)",
        "--err-texte": "rgb(233, 133, 166)",
        "--accent-texte": "rgb(233, 133, 166)",
        "--accent2-texte": "rgb(119, 171, 200)",
        "--dim": "rgb(167, 164, 189)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "snowshelf-christmas": {
      "base": {
        "--bg": "linear-gradient(135deg, #1a472a 0%, #0d2818 25%, #1a0a0a 50%, #2d0f0f 75%, #1a472a 100%) center center/cover no-repeat fixed",
        "--bg2": "#0d1810",
        "--panel": "rgba(20, 25, 35, 0.86)",
        "--line": "rgba(255, 215, 0, 0.3)",
        "--ink": "#f0f0f0",
        "--dim": "#98d998",
        "--accent": "#228b22",
        "--accent2": "#ff6b6b",
        "--ok": "#32cd32",
        "--warn": "#ffd700",
        "--err": "#b22222"
      },
      "resolus": {
        "--err-texte": "rgb(200, 106, 106)",
        "--accent-texte": "rgb(65, 154, 65)",
        "--on-accent": "var(--encre-claire)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-claire)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "snowshelf-halloween": {
      "base": {
        "--bg": "linear-gradient(135deg, #1a0a1a 0%, #0d0d1a 25%, #1a1a0a 50%, #0a0a0a 75%, #1a0a1a 100%) center center/cover no-repeat fixed",
        "--bg2": "#100810",
        "--panel": "rgba(20, 25, 35, 0.86)",
        "--line": "rgba(255, 140, 0, 0.3)",
        "--ink": "#e8e8e8",
        "--dim": "#9370db",
        "--accent": "#ff6600",
        "--accent2": "#9932cc",
        "--ok": "#32cd32",
        "--warn": "#ff6600",
        "--err": "#8b0000"
      },
      "resolus": {
        "--err-texte": "rgb(186, 116, 116)",
        "--accent2-texte": "rgb(173, 96, 211)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-claire)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-claire)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "snowshelf-santa": {
      "base": {
        "--bg": "linear-gradient(135deg, #8b0000 0%, #660000 25%, #4a0000 50%, #2d0000 75%, #1a0000 100%) center center/cover no-repeat fixed",
        "--bg2": "#2d0000",
        "--panel": "rgba(20, 25, 35, 0.86)",
        "--line": "rgba(255, 255, 255, 0.25)",
        "--ink": "#fff",
        "--dim": "#ffcccc",
        "--accent": "#cc0000",
        "--accent2": "#ffcccc",
        "--ok": "#32cd32",
        "--warn": "#ffd700",
        "--err": "#8b0000"
      },
      "resolus": {
        "--err-texte": "rgb(191, 115, 115)",
        "--accent-texte": "rgb(222, 89, 89)",
        "--on-accent": "var(--encre-claire)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-claire)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "space-gray": {
      "base": {
        "--bg": "radial-gradient(ellipse at center, rgba(87, 108, 117, 1) 0%, rgba(37, 50, 55, 1) 100%) center center/cover no-repeat fixed",
        "--bg2": "#576c75",
        "--panel": "rgba(20, 25, 35, 0.86)",
        "--line": "#576c75",
        "--ink": "#bbb",
        "--dim": "#999",
        "--accent": "#607D8B",
        "--accent2": "#81a6b7",
        "--ok": "#81a6b7",
        "--warn": "#f39c12",
        "--err": "#e74c3c"
      },
      "resolus": {
        "--ok-texte": "rgb(230, 237, 241)",
        "--warn-texte": "rgb(252, 230, 196)",
        "--err-texte": "rgb(251, 228, 226)",
        "--accent-texte": "rgb(231, 236, 238)",
        "--accent2-texte": "rgb(230, 237, 241)",
        "--dim": "rgb(235, 235, 235)",
        "--on-accent": "var(--encre-claire)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    },
    "trueblack": {
      "base": {
        "--bg": "#000",
        "--bg2": "#000",
        "--panel": "#000",
        "--line": "#333",
        "--ink": "#ddd",
        "--dim": "#5a5a5a",
        "--accent": "#7a7a7a",
        "--accent2": "#7a7a7a",
        "--ok": "#ddd",
        "--warn": "#ffaa00",
        "--err": "#ff4444"
      },
      "resolus": {
        "--dim": "rgb(123, 123, 123)",
        "--on-accent": "var(--encre-sombre)",
        "--on-accent2": "var(--encre-sombre)",
        "--on-ok": "var(--encre-sombre)",
        "--on-err": "var(--encre-sombre)",
        "--on-warn": "var(--encre-sombre)"
      },
      "video": null
    }
  }
}
```

## Sans jeton valide → 401

```json
{"detail":"connexion requise"}
```
