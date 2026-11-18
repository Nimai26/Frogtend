// Taodbox : le retour d'un écran ouvert depuis « ⚙ Gérer le jeu » (Réglages du jeu, Succès, Boutons) remet la grille
// comme on l'a laissée — la même console, le focus sur la carte du jeu — au lieu de repartir en haut, sur « Tout ».
// Gardé dans le module (la page de la grille est recréée au retour), consommé une seule fois.

export interface PlaceDansLaGrille {
  /** La carte du jeu (`data-id`). */
  id: number;
  /** Le filtre de console choisi (`null` : « Tout »). */
  console: string | null;
}

let place: PlaceDansLaGrille | null = null;

/** Retient la place avant d'ouvrir « Gérer le jeu » (il peut mener à un autre écran). */
export function retenirPlace(p: PlaceDansLaGrille) {
  place = { ...p };
}

/** Oublie la place (« Gérer le jeu » s'est refermé sans quitter la grille). */
export function oublierPlace() {
  place = null;
}

/** Rend la place retenue, une seule fois (`null` s'il n'y en a pas). */
export function reprendrePlace(): PlaceDansLaGrille | null {
  const p = place;
  place = null;
  return p;
}

/** La console à remettre : seulement si elle a encore des jeux (sinon « Tout »). */
export function consoleValide(p: PlaceDansLaGrille | null, consoles: string[]): string | null {
  return p?.console && consoles.includes(p.console) ? p.console : null;
}

/** Le sélecteur de la carte à viser : celle du jeu, sinon la première. */
export function selecteurCarte(p: PlaceDansLaGrille | null): string {
  return p ? `.taodbox .carte[data-id="${p.id}"]` : '.taodbox .carte';
}
