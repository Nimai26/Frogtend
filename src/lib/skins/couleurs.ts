// Lecture des couleurs et calcul du contraste (WCAG 2), selon la charte § 3.
// Aucune couleur n'est définie ici : on ne fait que lire celles des skins.

/** [rouge, vert, bleu, alpha] ; canaux de 0 à 255, alpha de 0 à 1. */
export type Rgba = [number, number, number, number];
export type Rgb = [number, number, number];

const borner = (x: number, min: number, max: number) => Math.min(max, Math.max(min, x));

/** Lit un nombre CSS, éventuellement en pourcentage (`pourCent` = valeur de 100 %). */
function nombre(texte: string, pourCent: number): number | null {
  const t = texte.trim();
  if (t === '') return null;
  const enPourcent = t.endsWith('%');
  const n = Number(enPourcent ? t.slice(0, -1) : t);
  if (!Number.isFinite(n)) return null;
  return enPourcent ? (n / 100) * pourCent : n;
}

/** Sépare les arguments d'une fonction CSS : virgules, ou espaces avec `/` avant l'alpha. */
function arguments_(interieur: string): string[] {
  if (interieur.includes(',')) return interieur.split(',').map((s) => s.trim());
  const [avant, alpha] = interieur.split('/');
  const parts = avant.trim().split(/\s+/);
  if (alpha !== undefined) parts.push(alpha.trim());
  return parts;
}

function hslVersRgb(h: number, s: number, l: number): Rgb {
  // h en degrés, s et l de 0 à 1.
  const k = (n: number) => (n + h / 30) % 12;
  const a = s * Math.min(l, 1 - l);
  const f = (n: number) => l - a * Math.max(-1, Math.min(k(n) - 3, Math.min(9 - k(n), 1)));
  return [Math.round(f(0) * 255), Math.round(f(8) * 255), Math.round(f(4) * 255)];
}

/**
 * Lit une couleur : `#rgb`, `#rrggbb`, `#rrggbbaa`, `rgb()`/`rgba()`, `hsl()`/`hsla()`.
 * Tout le reste (dégradé, nom de couleur, `#rgba` à 4 chiffres) est illisible : `null`.
 */
export function lireCouleur(texte: string): Rgba | null {
  const t = texte.trim().toLowerCase();

  const hex = /^#([0-9a-f]+)$/.exec(t);
  if (hex) {
    const h = hex[1];
    if (h.length === 3) return [0, 1, 2].map((i) => parseInt(h[i] + h[i], 16)).concat(1) as Rgba;
    if (h.length === 6 || h.length === 8) {
      const c = [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16));
      const a = h.length === 8 ? parseInt(h.slice(6, 8), 16) / 255 : 1;
      return [c[0], c[1], c[2], a];
    }
    return null;
  }

  const fn = /^(rgba?|hsla?)\((.*)\)$/.exec(t);
  if (!fn) return null;
  const args = arguments_(fn[2]);
  if (args.length !== 3 && args.length !== 4) return null;
  const alpha = args.length === 4 ? nombre(args[3], 1) : 1;
  if (alpha === null) return null;

  if (fn[1].startsWith('rgb')) {
    const c = args.slice(0, 3).map((x) => nombre(x, 255));
    if (c.some((x) => x === null)) return null;
    const [r, g, b] = (c as number[]).map((x) => Math.round(borner(x, 0, 255)));
    return [r, g, b, borner(alpha, 0, 1)];
  }

  const h = nombre(args[0].replace(/deg$/, ''), 360);
  const s = nombre(args[1], 1);
  const l = nombre(args[2], 1);
  if (h === null || s === null || l === null) return null;
  // Sans « % », s et l sont donnés sur 100 (syntaxe moderne) : on ramène sur 1.
  const sur1 = (x: number, brut: string) => (brut.trim().endsWith('%') ? x : x / 100);
  const [r, g, b] = hslVersRgb(
    ((h % 360) + 360) % 360,
    borner(sur1(s, args[1]), 0, 1),
    borner(sur1(l, args[2]), 0, 1),
  );
  return [r, g, b, borner(alpha, 0, 1)];
}

/** Toutes les couleurs lisibles contenues dans une valeur (un dégradé à plusieurs couches, par exemple). */
export function teintesDe(valeur: string): Rgba[] {
  const motifs = valeur.match(/#[0-9a-fA-F]+\b|(?:rgba?|hsla?)\([^)]*\)/g) ?? [];
  return motifs.map(lireCouleur).filter((c): c is Rgba => c !== null);
}

/** Luminance relative (WCAG 2) ; l'alpha est ignoré. */
export function luminance([r, g, b]: Rgb | Rgba): number {
  const lin = (x: number) => {
    const c = x / 255;
    return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
  };
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}

export function contraste(a: Rgb | Rgba, b: Rgb | Rgba): number {
  const la = luminance(a);
  const lb = luminance(b);
  return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05);
}

/** Le pire contraste d'une couleur sur une liste de fonds. */
export function pireContraste(couleur: Rgb | Rgba, fonds: Rgb[]): number {
  if (fonds.length === 0) return Infinity;
  return Math.min(...fonds.map((f) => contraste(couleur, f)));
}

/** Pose une couleur translucide sur une teinte : `canal × a + teinte × (1 − a)`, arrondi. */
export function composer([r, g, b, a]: Rgba, teinte: Rgb | Rgba): Rgb {
  return [
    Math.round(r * a + teinte[0] * (1 - a)),
    Math.round(g * a + teinte[1] * (1 - a)),
    Math.round(b * a + teinte[2] * (1 - a)),
  ];
}

/** Rapproche `x` de `cible` de `pas` % : `m = x + (cible − x) × pas/100`, arrondi. */
export function melanger(x: Rgb | Rgba, cible: Rgb | Rgba, pas: number): Rgb {
  return [0, 1, 2].map((i) => Math.round(x[i] + (cible[i] - x[i]) * (pas / 100))) as Rgb;
}

export const ecrireRgb = ([r, g, b]: Rgb | Rgba) => `rgb(${r}, ${g}, ${b})`;
