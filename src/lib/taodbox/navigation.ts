// Taodbox (lot 5) : la navigation à la croix suit la géométrie de l'écran (charte § 7) et n'est jamais piégée.

export type Direction = 'haut' | 'bas' | 'gauche' | 'droite';

export interface Boite {
  x: number;
  y: number;
  l: number;
  h: number;
}

const centre = (b: Boite) => ({ x: b.x + b.l / 2, y: b.y + b.h / 2 });

/** L'élément le plus proche dans une direction (indice dans `boites`), ou `null` s'il n'y en a pas. On privilégie
 * ce qui est bien aligné : la distance perpendiculaire compte trois fois plus. */
export function voisin(boites: Boite[], courant: number, d: Direction): number | null {
  const c = centre(boites[courant]);
  let meilleur: number | null = null;
  let score = Infinity;
  boites.forEach((b, i) => {
    if (i === courant) return;
    const p = centre(b);
    const dx = p.x - c.x;
    const dy = p.y - c.y;
    const avant = d === 'droite' ? dx : d === 'gauche' ? -dx : d === 'bas' ? dy : -dy;
    if (avant <= 1) return; // pas dans cette direction
    const cote = d === 'droite' || d === 'gauche' ? Math.abs(dy) : Math.abs(dx);
    const s = avant + 3 * cote;
    if (s < score) {
      score = s;
      meilleur = i;
    }
  });
  return meilleur;
}

/** La direction d'un stick (axes −1…1) ou `null` dans la zone morte. */
export function directionDuStick(x: number, y: number, zoneMorte = 0.5): Direction | null {
  if (Math.abs(x) < zoneMorte && Math.abs(y) < zoneMorte) return null;
  if (Math.abs(x) > Math.abs(y)) return x > 0 ? 'droite' : 'gauche';
  return y > 0 ? 'bas' : 'haut';
}

/** Ce qu'une manette (ou le cœur, qui lit la manette pendant une partie) demande à l'interface. */
export type Commande = Direction | 'valider' | 'retour';

/**
 * Les boutons qui viennent d'être enfoncés : pas tenus au tour précédent. Un bouton déjà tenu quand la lecture
 * commence (par exemple la combinaison qui vient d'ouvrir le menu) ne compte qu'une fois relâché puis ré-enfoncé.
 */
export function nouveauxAppuis(avant: ReadonlySet<number>, maintenant: ReadonlySet<number>): Set<number> {
  return new Set([...maintenant].filter((b) => !avant.has(b)));
}
