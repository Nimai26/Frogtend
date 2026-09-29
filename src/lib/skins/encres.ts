// Calcul des encres dérivées d'un skin (charte § 3).
// Firehouse les servira déjà résolues (`/api/jeux/v1/themes`) ; ce calcul ne sert que quand la source ne les
// fournit pas (mode simulé, sur l'instantané). Il doit retrouver les garanties de Firehouse :
// chaque `--on-*` ≥ 4,3:1 sur son fond, chaque texte ≥ 4,5:1 sur le pire fond.

import {
  composer,
  ecrireRgb,
  lireCouleur,
  luminance,
  melanger,
  pireContraste,
  teintesDe,
  type Rgb,
} from './couleurs';

export type Jetons = Record<string, string>;

export const SEUIL_TEXTE = 4.5;
export const TONS = ['ok', 'warn', 'err', 'accent', 'accent2'] as const;

/** Les fonds de référence sur lesquels un texte peut se trouver : `--panel` et `--bg2`, posés sur `--bg`. */
export function fondsDeReference(base: Jetons): Rgb[] {
  const teintes = teintesDe(base['--bg'] ?? '');
  const fonds: Rgb[] = [];
  for (const nom of ['--panel', '--bg2']) {
    const c = lireCouleur(base[nom] ?? '');
    if (!c) continue;
    if (c[3] >= 1 || teintes.length === 0) fonds.push([c[0], c[1], c[2]]);
    else for (const t of teintes) fonds.push(composer(c, t));
  }
  return fonds;
}

/**
 * Rend une couleur de texte lisible (≥ 4,5 sur le pire fond), ou `null` si elle l'est déjà.
 * Mélange vers `--ink` par pas de 5 %, puis vers l'encre neutre ; sinon le meilleur mélange s'il améliore.
 */
export function corrigerTexte(
  origine: string,
  encre: string,
  fonds: Rgb[],
  encreClaire: string,
  encreSombre: string,
): string | null {
  const x = lireCouleur(origine);
  if (!x) return null;
  const depart = pireContraste(x, fonds);
  if (depart >= SEUIL_TEXTE) return null;

  let meilleur: { c: Rgb; contraste: number } | null = null;
  const essayer = (cible: Rgb | null): string | null => {
    if (!cible) return null;
    for (let pas = 5; pas <= 100; pas += 5) {
      const m = melanger(x, cible, pas);
      const k = pireContraste(m, fonds);
      if (k >= SEUIL_TEXTE) return ecrireRgb(m);
      if (!meilleur || k > meilleur.contraste) meilleur = { c: m, contraste: k };
    }
    return null;
  };

  const versEncre = essayer(lireCouleur(encre)?.slice(0, 3) as Rgb | null);
  if (versEncre) return versEncre;

  const claire = lireCouleur(encreClaire);
  const sombre = lireCouleur(encreSombre);
  if (claire && sombre) {
    const neutre = pireContraste(claire, fonds) >= pireContraste(sombre, fonds) ? claire : sombre;
    const versNeutre = essayer(neutre.slice(0, 3) as Rgb);
    if (versNeutre) return versNeutre;
  }

  const m = meilleur as { c: Rgb; contraste: number } | null;
  return m && m.contraste > depart ? ecrireRgb(m.c) : null;
}

/** L'encre à poser SUR une couleur de fond : la sombre ou la claire, celle qui contraste le plus. */
export function encreSur(fond: string, encreClaire: string, encreSombre: string): 'claire' | 'sombre' {
  const f = lireCouleur(fond);
  const s = lireCouleur(encreSombre);
  if (!f || !s) return 'sombre';
  const l = luminance(f);
  const ls = luminance(s);
  const surSombre = (l + 0.05) / (ls + 0.05);
  const surClair = 1.05 / (l + 0.05);
  return surSombre >= surClair ? 'sombre' : 'claire';
}

/**
 * Calcule les jetons dérivés d'un skin : `--on-*`, `--*-texte` et `--dim` corrigé.
 * Ce que le skin impose déjà est gardé tel quel.
 */
export function resoudreEncres(base: Jetons, communs: Jetons): Jetons {
  const claire = communs['--encre-claire'] ?? '';
  const sombre = communs['--encre-sombre'] ?? '';
  const fonds = fondsDeReference(base);
  const resolus: Jetons = {};

  for (const ton of TONS) {
    const on = `--on-${ton}`;
    if (!base[on]) {
      const choix = encreSur(base[`--${ton}`] ?? '', claire, sombre);
      resolus[on] = choix === 'claire' ? 'var(--encre-claire)' : 'var(--encre-sombre)';
    }
  }

  for (const ton of TONS) {
    const nom = `--${ton}-texte`;
    if (base[nom]) continue;
    const corrige = corrigerTexte(base[`--${ton}`] ?? '', base['--ink'] ?? '', fonds, claire, sombre);
    resolus[nom] = corrige ?? `var(--${ton})`;
  }

  const dim = corrigerTexte(base['--dim'] ?? '', base['--ink'] ?? '', fonds, claire, sombre);
  if (dim) resolus['--dim'] = dim;

  return resolus;
}
