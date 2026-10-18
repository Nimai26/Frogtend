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
