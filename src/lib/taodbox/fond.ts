// Taodbox (lot Médias, M3) : quel jeu montrer en fond et en logo. Le jeu de la carte qui a le focus (`data-id`),
// choisi seulement après un court REPOS du focus : la croix tenue répète toutes les 130 ms, le fond ne change donc
// jamais pendant un défilement. Hors d'une carte de jeu Firehouse (boutons, Steam, GOG) : aucun (le fond du skin).

export const REPOS_MS = 200;

/** Le numéro du jeu de la carte qui contient `cible`, ou `null`. */
export function idDuFocus(cible: EventTarget | null): number | null {
  const carte = cible instanceof Element ? cible.closest<HTMLElement>('[data-id]') : null;
  const n = carte ? Number(carte.dataset.id) : NaN;
  return Number.isFinite(n) ? n : null;
}

/** Appelle `rappel` avec la DERNIÈRE valeur poussée, une fois `ms` écoulées sans nouvelle poussée. */
export function apresRepos<T>(rappel: (v: T) => void, ms = REPOS_MS) {
  let minuterie: ReturnType<typeof setTimeout> | undefined;
  return {
    pousser(v: T) {
      clearTimeout(minuterie);
      minuterie = setTimeout(() => rappel(v), ms);
    },
    annuler() {
      clearTimeout(minuterie);
    },
  };
}

/** Une image assez grande pour couvrir l'écran en fond ? Le fond d'un jeu se replie sur une capture ou un écran titre
 * quand il n'a pas de vrai fond : une capture de Super Nintendo fait 256 × 224, et étirée sur une télé elle n'est plus
 * qu'un énorme zoom flou (essai de Seb, 30/10). En dessous de 640 × 360 : le fond du thème à la place. */
export function assezGrandPourUnFond(largeur: number, hauteur: number): boolean {
  return largeur >= 640 && hauteur >= 360;
}
