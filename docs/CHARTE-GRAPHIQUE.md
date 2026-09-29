# Charte graphique de Frogtend (dérivée de Firehouse)

> Document destiné à l'agent de code qui construit Frogtend (bureau Windows) et Taodbox (son mode canapé, à la manette).
> Relevé sur le code de Firehouse 2.9.1 le 29/09/2026 par l'expert UI de Firehouse. Quand ce document et le code de
> Firehouse ne disent pas la même chose, c'est le code qui fait foi. Aucune couleur n'est proposée ici : elles viennent
> toutes des skins de Firehouse.

## 1. Principe : une seule source

- Les skins viennent de Firehouse et nulle part ailleurs. Ce sont 44 thèmes, avec exactement 11 variables chacun. **On ne
  les recopie jamais à la main** dans Frogtend : on les télécharge, on les garde en cache, on les applique.
- Chaque personne a **son** thème, enregistré côté serveur dans son compte Firehouse. Frogtend applique ce thème par
  défaut.
- Frogtend peut proposer un autre thème, **localement**, seulement parmi les 44. Si la personne veut l'enregistrer,
  Frogtend l'écrit dans Firehouse par la route prévue (§ 8) : il n'invente pas de réglage à part.
- Un nom de thème inconnu ne s'applique pas : on reste sur le thème en cours, ou sur `firehouse` à défaut. Firehouse fait
  la même chose.
- Les palettes sont reprises de **Theme.Park (GilbN, GPL-3.0)**. La clé `_credit` de `themes.json` le dit : il faut la
  reprendre dans l'écran « À propos ».

## 2. Les jetons

### 2.1 Les 11 variables de base, définies par chaque thème

| Jeton | Rôle dans Firehouse |
|---|---|
| `--bg` | Fond de page, y compris l'écran de connexion. Peut être une couleur **ou un dégradé CSS complet**, parfois à plusieurs couches (30 thèmes sur 44). Posé en `background` avec `background-attachment: fixed`. On n'y écrit jamais de texte. |
| `--bg2` | Fond des **contrôles et des cartes** : boutons, champs, `<option>`, cartes, blocs, étiquettes, pastilles. |
| `--panel` | Surface des **fenêtres** : panneaux, barre du haut, toasts, modales. **Translucide dans 27 thèmes**, avec alors un flou d'arrière-plan. |
| `--line` | Bordures, séparateurs, poignée de la barre de défilement. |
| `--ink` | Texte principal. C'est aussi l'encre vers laquelle on rapproche les couleurs d'état trop pâles (§ 3). |
| `--dim` | Texte secondaire : libellés, texte discret, clés d'une liste clé/valeur. **Corrigé au contraste** (§ 3). |
| `--accent` | L'action : fond du bouton principal, bordure au survol ou à la sélection, onglet actif. L'en-tête de fenêtre en prend 12 % (`color-mix`). |
| `--accent2` | L'accent secondaire : liens, et départ du dégradé des barres de progression (`--accent2` vers `--accent`). |
| `--ok` / `--warn` / `--err` | Les **états**. En liseré, en bordure, en point de couleur ou en fond très dilué. Jamais tels quels comme couleur de texte. |

### 2.2 Les jetons dérivés, calculés à l'application du thème

| Jeton | Rôle | Origine |
|---|---|---|
| `--on-accent`, `--on-accent2`, `--on-ok`, `--on-warn`, `--on-err` | Encre posée **sur** la couleur correspondante (texte d'un bouton principal, d'un badge plein) | Calculé (§ 3). Un thème peut l'imposer. |
| `--ok-texte`, `--warn-texte`, `--err-texte`, `--accent-texte`, `--accent2-texte` | La couleur d'état **employée comme texte**, garantie lisible (≥ 4,5:1) | Calculé. Vaut `var(--ton)` quand la couleur d'origine se lit déjà. |
| `--dim` (corrigé) | Réécrit à sa place quand il ne se lit pas | Calculé |

### 2.3 Les jetons communs, identiques pour tous les thèmes

| Jeton | Rôle |
|---|---|
| `--encre-sombre` / `--encre-claire` | Les deux seules encres de base (valeurs servies par l'API) |
| `--fond-media` | Fond fixe derrière une image ou une vidéo (ne suit pas le thème, exprès) |
| `--fond-page` | Page blanche d'un document intégré |
| `--encre-voile`, `--encre-voile-dim`, `--encre-voile-note`, `--encre-voile-ok` | Encres posées sur un voile noir plein écran (visionneuse) |
| `--voile-video` | Voile de lisibilité posé sur le fond vidéo |
| `--voile-leger`, `--voile`, `--voile-fort`, `--voile-opaque` | Voiles noirs des modales et des visionneuses, de 40 à 94 % |
| `--ombre-noire` | Ombre portée des modales |
| `--damier-clair` / `--damier-fonce` | Damier derrière une image transparente |
| `--radius` | 12 px, le rayon des fenêtres |
| `--mono` | `ui-monospace, "SF Mono", Menlo, Consolas, monospace` |

Firehouse garde aussi des alias hérités d'anciens écrans (`--border`, `--bord`, `--bd` → `--line` ; `--surface` →
`--panel` ; `--bg3` → `--bg2` ; `--muted` → `--dim` ; `--ac` → `--accent` ; `--bad` → `--err`). **Frogtend ne s'en sert
pas** : il emploie les noms de base.

## 3. L'algorithme des encres, à reproduire à l'identique

Le plus sûr est de **prendre les valeurs déjà résolues par Firehouse** (§ 8). S'il faut les recalculer :

**Lecture d'une couleur.** On obtient `[r, g, b, alpha]` depuis `#rgb`, `#rrggbb`, `#rrggbbaa`, `rgb()`/`rgba()` (virgules
ou espaces, alpha en nombre ou en pourcentage), `hsl()`/`hsla()` (converti en RGB arrondi ; 7 thèmes l'emploient). Tout
le reste est « illisible » : dégradé, nom de couleur, et `#rgba` à 4 chiffres.

**Les fonds de référence.** On relève toutes les teintes présentes dans la chaîne `--bg`. Pour `--panel` et `--bg2` : si
le fond est opaque, ou si `--bg` n'a aucune teinte, on garde son RGB (alpha ignoré) ; sinon, on le compose sur **chacune**
des teintes de `--bg` : `canal × a + teinte × (1 − a)`, arrondi. Le contraste retenu est toujours le **pire** de la liste.

**Luminance et contraste (WCAG 2).** `c = x/255`, puis `c ≤ 0,03928 ? c/12,92 : ((c + 0,055)/1,055)^2,4` ;
`L = 0,2126 R + 0,7152 G + 0,0722 B` ; contraste = `(Lmax + 0,05) / (Lmin + 0,05)`.

**Textes d'état** (seuil 4,5), dans l'ordre `--ok-texte`, `--warn-texte`, `--err-texte`, `--accent-texte`,
`--accent2-texte`, puis `--dim` :
1. Si le thème impose déjà la variante `-texte`, on n'y touche pas.
2. Si le pire contraste de la couleur d'origine est ≥ 4,5, rien ne change.
3. Sinon, on mélange la couleur vers `--ink` **par pas de 5 %** (5 à 100 %) : `m = x + (cible − x) × pas/100`, arrondi.
   Le premier mélange qui atteint 4,5 est retenu, en `rgb(r, g, b)`.
4. Sinon, même chose vers l'encre **neutre** : l'encre claire ou sombre dont le pire contraste sur les fonds est le
   meilleur (égalité : l'encre claire).
5. Sinon, on garde le meilleur mélange trouvé s'il améliore l'original — jamais pire que l'original.

**Encres sur fond coloré (`--on-*`).** Si le thème l'impose, on la garde. Sinon `l` = luminance du fond (alpha ignoré ;
fond illisible → encre sombre). Avec `Ls` la luminance de l'encre sombre : `surSombre = (l + 0,05)/(Ls + 0,05)`,
`surClair = 1,05/(l + 0,05)` ; `surSombre ≥ surClair ? encre sombre : encre claire`. Même règle pour une couleur de
DONNÉE choisie par l'utilisateur (si illisible : `--ink`).

**Garanties testées dans Firehouse** : sur les 44 thèmes, chaque encre `--on-*` atteint au moins 4,3:1 sur son fond, et
les 44 × 6 textes atteignent au moins 4,5:1 sur le pire fond. Un recalcul doit retrouver ces nombres.

**Valeurs qui ne sont pas des couleurs.**
- `--bg` se peint tel quel en CSS. En rendu natif incapable de peindre le dégradé : repli sur `--bg2` en aplat.
- La **vidéo de fond** ne concerne que le thème `firehouse` (`/static/img/firehouse-bg.webm`, 2,4 Mo) : plein écran,
  `object-fit: cover`, recouverte du voile `--voile-video`, fond de page transparent.

## 4. Les règles non négociables de Seb

1. **Jamais de couleur en dur.** Une couleur n'apparaît que dans les jetons. Blanc translucide :
   `color-mix(in srgb, var(--ink) N%, transparent)`. Fond teinté : `color-mix(in srgb, var(--ton) 12 à 24%,
   var(--panel) ou transparent)`. Un rendu canvas ou natif lit la valeur calculée du jeton. On peint aussi les `<option>`
   (`--bg2` / `--ink`) : sinon le fond clair du système rend le texte invisible.
2. **Le danger se dit par un liseré.** Un état ne se signale jamais par un fond plein coloré. Le texte reste `--ink`, la
   couleur va au trait :
   - bouton dangereux : bordure `--err` plus un liseré intérieur gauche de 3 px `--err` ;
   - toast : bordure gauche de 3 px à la couleur de l'état ;
   - étiquette d'état : texte `--*-texte`, bordure `--ton`, fond à `ton 15 %`.

   Un texte d'état s'écrit **toujours** avec la variante `-texte`, jamais avec la couleur brute.
3. **Pas de fenêtre native** (`alert`, `confirm`, `prompt`, `MessageBox`) : les cinq outils maison ci-dessous.
4. **Chaque refus est dit, avec son motif.**
5. **Utilisable au doigt, au clavier et à la manette.** Jamais un geste (survol, clic droit, glisser-déposer,
   double-clic) sans un bouton qui fait la même chose. Au doigt, une cible fait **au moins 40 px** (44 px pour les
   en-têtes repliables et les barres d'action). Ce qui n'apparaît qu'au survol devient visible en permanence sans souris.
6. **Les écrans étroits** (si Frogtend a un jour une vue compacte) : sous 720 px, chaque fenêtre passe en plein écran, les
   grilles sur une colonne, les boutons d'action s'étirent, la sortie n'est jamais rognée (c'est le libellé qui passe en
   ellipse) ; sous 560 px, les grilles d'affiches se resserrent. Aucun élément ne doit élargir la fenêtre.

**Les cinq outils maison.** Tous s'ouvrent sur un voile `--voile` ; boîte `--panel`, bordure `ink 16 %`, rayon 14 px,
18 px de marge intérieure, 440 px de large par défaut (94 % de la largeur et 86 % de la hauteur au plus, contenu qui
défile), ombre `--ombre-noire`.

| Outil | Quand | Rend |
|---|---|---|
| Confirmer (`fhConfirmer`) | Avant une action irréversible | vrai / faux. « Annuler » puis « Confirmer » (« Supprimer » si danger), à droite. |
| Demander (`fhDemander`) | Saisir un texte | le texte, ou rien. Le texte proposé est présélectionné. |
| Choisir (`fhChoisir`) | Un choix dans une liste (jamais « tape le numéro ») | la valeur, ou rien |
| Informer (`fhInfo`) | Un message qu'il **faut** avoir lu | un seul bouton « OK » |
| Toast (`fhToast`) | Un compte rendu bref, qui ne bloque rien | — |

Comportement commun : Échap et le clic sur le voile **annulent** ; Entrée valide (sauf dans un texte multiligne) ; seule
la fenêtre du dessus écoute le clavier ; le focus va au premier champ, sinon au bouton de validation. **À faire mieux que
Firehouse** : retenir le focus à l'intérieur de la fenêtre, et le rendre à l'élément d'origine en fermant —
indispensable à la manette. (Et le bouton « Supprimer » d'une confirmation suit la règle du liseré, pas un fond rouge.)

**Les toasts.** 3,2 s par défaut (6 s pour un refus) ; pile en bas au centre, au-dessus des modales ; un clic ferme.
Types : `ok` (liseré `--ok`), `alerte` (`--warn`), `erreur` (`--err`), neutre (`--dim`). Sans type donné, il se déduit du
message : ✅ 👍 🎉 → ok ; ⛔ ❌ 🚫 → erreur ; ⚠ → alerte ; puis « refusé », « a échoué », « impossible de » → erreur ;
puis le premier mot (« Refus… », « Échec », « Erreur », « Introuvable » → erreur ; « Attention », « Déjà », « Aucun
résultat » → alerte ; « Enregistré », « Ajouté », « Créé » → ok). Un ✅ l'emporte sur tout.

**Refus toujours dits.** Le motif s'extrait du texte, de `message`, `erreur`/`error`, `detail` ou d'une liste de
validation — jamais « [object Object] ». Une écriture refusée que l'écran n'a pas affichée est annoncée quand même
(« ⛔ Refusé : motif »). Jamais de chemin d'API dans un message.

## 5. Les composants de base

Typographie : `system-ui, -apple-system, "Segoe UI", Roboto, sans-serif` ; corps de base **15 px** ; chasse fixe `--mono`.

| Composant (classe Firehouse de référence) | Valeurs |
|---|---|
| **Bouton** `.cx-btn` | Rayon 10, marges 9 × 15, graisse 600, 13 px, 7 px entre icône et texte, bordure 1 px `--line`, fond `--bg2`, texte `--ink`, transition 0,15 s. Survol : bordure `--accent`, remonté d'1 px. |
| … principal `.primary` | Fond et bordure `--accent`, texte `--on-accent`. Survol : luminosité × 1,08. |
| … danger `.danger` | Bordure `--err`, texte `--ink`, liseré intérieur gauche 3 px `--err`. |
| … désactivé | Opacité 0,45, aucun clic. |
| **Étiquette d'état** `.tag` (`.ok`, `.warn`, `.err`) | 11 px, marges 2 × 7, rayon 6. Neutre : fond `--bg2`, bordure `--line`, texte `--dim`. État : texte `--*-texte`, bordure `--ton`, fond `ton 15 %`. |
| **Pastille** `.pill` | 12 px, marges 4 × 10, rayon 999, fond `--bg2`, texte `--dim` ; point de 7 px en `--ok` / `--warn` / `--err`. |
| **Pastille de donnée** `.cx-pastille` | Fond `ton 24 %` mêlé à `--panel`, texte `--ink`, bordure `ton 60 %`, rayon 10. |
| **Carte** `.cx-card` | Rayon 14, fond `--bg2`, bordure `--line`. Survol : remontée 4 px, bordure `--accent`, ombre. Jaquette au format 3/4 en `contain`. Grille à colonnes de 158 px minimum, espacées de 16 px. |
| **Bloc** `.cx-block` | Rayon 14, marges 15 × 17, fond `--bg2`. Titre 11,5 px, capitales, lettres espacées 0,6 px, `--dim`, graisse 700. |
| **Fenêtre** `.panel` | Fond `--panel`, bordure `--line`, rayon `--radius`, flou 16 px. En-tête : `panel 88 %` + `accent 12 %`, marges 11 × 13 ; titre 14 px, graisse 600. |
| **Bloc repliable** `details > summary` | Hauteur minimale 44 px, marqueur natif masqué, ▸ fermé / ▾ ouvert en `--dim`. |
| **Liste clé/valeur** `.cx-kv` | Deux colonnes, 12 px d'écart, marges verticales 7 px, filet `--line` sous chaque ligne sauf la dernière, 13 px. Clé `--dim`, valeur à droite, coupée au besoin. |
| **Formulaire** `.cx-form-section` | Fond `--bg2`, rayon 14, marges 15 × 17. Libellé 12,5 px `--dim`, 5 px au-dessus du champ. Deux colonnes (12 px), une seule en vue étroite. |
| **Champ** `input`, `select`, `textarea` | Fond `--bg2`, texte `--ink`, bordure `--line`, rayon 8, marges 6 × 9, police héritée. `<option>` en `--bg2` / `--ink`. Au doigt : 40 px de haut au minimum. |
| **Texte secondaire** `.muted` | `--dim`, 12 px |
| **Barre de progression** `.bar` | 6 px, rayon 4, fond `--bg2`, remplissage en dégradé `--accent2` → `--accent` |

**Focus clavier** : `outline: 2px solid var(--accent)`, 3 px d'écart — **partout** dans Frogtend (Firehouse ne l'applique
pas encore partout).

## 6. Le ton des textes

- Du **français**, en **tutoyant** la personne : « choisis un dossier », « aucun jeu ne correspond ».
- **Dire ce qui se passe, et ce qui est vrai.** Pas de promesse sans date. Une liste coupée l'annonce (« 12 autre(s) —
  affine la recherche »). Un sélecteur vide dit pourquoi il est vide.
- **Un échec donne son motif**, commence par le mot qui le signale (« Refusé : … », « Impossible de … »), sans jargon
  technique. Un succès dit ce qui a été fait (« Jeu installé. »).
- Des libellés d'action courts, à l'infinitif ou à l'impératif : « Annuler », « Valider », « Installer ». Pour sauter
  une étape facultative : « Passer », pas « Annuler ».
- **Les emoji servent de repères**, pas de décoration : en tête d'un titre de fenêtre ou de section ; dans les boutons,
  pour le verbe (« ⬇ Télécharger ») ; dans les messages, pour le ton : ✅ réussi, ⚠ attention, ⛔ refusé.

## 7. Taodbox : la télé, le canapé, la manette

Règles **dérivées** des jetons existants — aucune nouvelle couleur. C'est une proposition : aucun écran « télé »
n'existe encore dans Firehouse, tout est à valider sur le vrai téléviseur avec Seb.

- **Échelle.** Toutes les tailles sont multipliées par un seul facteur appliqué au corps de base : proposé ×2 en 1080p
  (15 px → 30 px ; le 12 px du texte secondaire → 24 px, minimum pour être lu à 3 m). Rayons, marges et épaisseurs
  suivent.
- **Focus fort et toujours visible** : contour 4 px `--accent`, écart 4 px, légère mise à l'échelle (1,05), bordure
  `--accent`, sans animation longue.
- **Sélection et focus ne se confondent pas** : la sélection prend `color-mix(--accent 20 %, transparent)` avec bordure
  `--accent` ; le focus garde le contour.
- **Navigation à la croix** : elle suit la géométrie et **n'est jamais piégée** (chaque zone a une sortie). A valide,
  B fait Échap et referme la fenêtre du dessus ; au retour d'une fenêtre, le focus revient où il était.
- **Aucun survol requis** : tout est visible ou atteignable par un élément focalisable.
- **Lisibilité** : texte en `--ink` ou variantes `-texte` ; sur une affiche ou une vidéo, voile `--voile-fort` et encres
  `--encre-voile*` ; le fond vidéo du thème `firehouse` est permis, sous `--voile-video`.
- **Marges de sécurité de la télé** : environ 5 % de bord libre (surbalayage), rien de vital dans les coins.
- **Modales et toasts identiques au bureau**, à l'échelle, 100 % pilotables à la manette.

## 8. Récupérer les skins

**Aujourd'hui** (en attendant l'API de Frogtend) :
- `GET /static/themes.json` est **public, sans session**. Format : `{"_credit": "...", "themes": {"<nom>": {11 variables}}}`.
- Le thème d'une personne se lit dans `GET /api/me` (champ `theme`) et s'écrit par `POST /api/me/theme {"theme": "<nom>"}`
  — deux routes de session du cockpit, pas pour Frogtend.
- Les jetons communs et les encres résolues ne sont servis par aucune route aujourd'hui.
- Un instantané des 44 skins au 29/09 est fourni dans `docs/charte/themes.instantane.json` : **une fixture pour coder hors
  ligne, jamais la source**.

**L'API de Frogtend** (EN SERVICE depuis Firehouse 2.10.0 ; voir `CLAUDE.md` § 3) :
- `GET /api/jeux/v1/themes` →
  `{version, credit, communs: {jetons communs}, themes: {nom: {base: {11}, resolus: {--on-*, --*-texte, --dim}, video: url|null}}}`,
  avec une empreinte (ETag) : Frogtend la met en cache et ne la retélécharge que si elle change.
- `GET /api/jeux/v1/theme` → `{theme, jetons: {base + communs + résolus fusionnés}}` pour la personne du jeton ; un thème
  enregistré inconnu est remplacé par `firehouse`, et c'est signalé.
- Les encres résolues sont calculées par Firehouse avec le même algorithme que son cockpit (et un test garantit qu'elles
  sont identiques) : **Frogtend n'a pas à le réimplémenter**.
