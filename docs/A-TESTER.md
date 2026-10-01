# À tester par Seb à son retour

> Tenu à jour au fil du travail. Chaque ligne dit **quoi faire** et **ce qu'on doit voir**. Coche (`[x]`) ce qui
> marche ; pour ce qui ne marche pas, un mot ou une capture suffit.

## Tout de suite : installer la dernière version
- [ ] Frogtend propose la mise à jour au démarrage (ou ⚙ Options ▸ À propos ▸ Chercher une mise à jour).
      **Attendu :** la version la plus récente de [NOTES-DE-VERSION.md](NOTES-DE-VERSION.md).

## Demander un jeu (0.12.0)
- [ ] Barre du haut ▸ **Demander un jeu** ▸ chercher « zelda ocarina ».
      **Attendu :** une liste (titre, console, année, studio) ; Ocarina of Time N64 marqué « ⏳ Déjà demandé ».
- [ ] Demander un jeu qui n'y est pas encore (confirmer). **Attendu :** « 📨 Demande envoyée », et la demande
      apparaît dans Firehouse (à vérifier côté cockpit).

## Menu en jeu (0.11.0)
- [ ] Installer Dune si besoin, le lancer, appuyer sur **Pause/Attn**.
      **Attendu :** le menu Frogtend apparaît PAR-DESSUS le jeu ; le jeu est figé.
      *Si le jeu continue :* Dune lance peut-être sa propre copie de DOSBox (pas réglée par Frogtend) : me le dire.
- [ ] ▶ Reprendre (ou Échap). **Attendu :** le menu disparaît, le jeu repart.
- [ ] 📖 Manuel et documents : ouvrir un texte. **Attendu :** lisible dans le menu ; Échap revient.
- [ ] ⏹ Quitter le jeu (confirmer). **Attendu :** le jeu se ferme proprement, Frogtend compte le temps de jeu.
- [ ] Hors partie, la touche Pause/Attn ne fait rien de spécial (elle reste aux autres programmes).
- [ ] ⚙ Options ▸ Émulateurs ▸ « Touche du menu en jeu » : essayer Arrêt défil à la partie suivante.

## Sonde manette (lot 4 ter)
- [ ] Lancer `D:\Frogtend\outils\sonde-manette\target\release\sonde-manette.exe`, appuyer sur chaque manette
      (Xbox, PS4, PS5, Switch Pro, 8BitDo…), puis lancer un jeu (au premier plan) et appuyer encore ; laisser finir
      les 3 minutes. **Attendu :** le bilan de `sonde-manette.txt` (à côté du programme). Me le donner.

## Plusieurs émulateurs par console (0.10.0)
- [ ] ⚙ Options ▸ Émulateurs : chaque console montre sa liste, ⭐ par défaut, ➕ Ajouter, ✕ Retirer.
- [ ] Un jeu : ⚙ Gérer le jeu ▸ 🕹 Émulateur ▸ en choisir un autre que celui de la console.
- [ ] Avec deux émulateurs sur une console : le bouton **▶ Jouer avec…** apparaît à côté de ▶ Jouer.

## Manettes (0.8.0 – 0.9.0)
- [ ] Un jeu PS1/PS2/GameCube/Wii (si tu en as) : la manette marche sans rien régler.
- [ ] ⚙ Options ▸ Émulateurs ▸ 🎮 Manette par défaut (sur un émulateur installé) : message de réussite.
- [ ] Wii : tes profils Wiimote. Dans Dolphin, régler la Wiimote, « Enregistrer » le profil, puis ⚙ Options ▸
      Émulateurs ▸ 📌 Réglages de référence ▸ le reprendre (garde le nom « Wiimote + Nunchuk » pour remplacer le
      mien). Me dire lesquels intégrer à Frogtend pour tous les PC.
- [ ] ⚙ Gérer le jeu ▸ 🎮 Commandes : Automatique / Clavier et souris / une référence.

## Émulateurs (0.6.0 – 0.7.0), jamais essayés en vrai
- [ ] Lancer un jeu d'une console sans émulateur : Frogtend propose le recommandé, annonce la taille, l'installe
      dans le dossier « Émulateurs », puis lance le jeu.
- [ ] ⚙ Options ▸ Émulateurs ▸ 🔎 Chercher des mises à jour.
- [ ] Deux profils, le même jeu d'émulateur : chacun retrouve SES parties.

## Sauvegarde (0.9.1)
- [x] Première vraie sauvegarde : faite et vérifiée le 30/09 (DUNE37S0.SAV chez Firehouse, identique).
