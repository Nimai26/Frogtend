// Taodbox (lot 5) : la manette pilote TOUS les écrans de Frogtend (choix du profil, fenêtres maison, ludothèque…).
// La croix et le stick gauche déplacent le focus selon la géométrie, A valide, B fait Échap (charte § 7). Lu par
// l'API Gamepad du navigateur : Taodbox est au premier plan, la lecture y est permise (manette Xbox, PlayStation,
// Switch Pro… selon ce que Windows et WebView2 reconnaissent).
import { directionDuStick, voisin, type Direction } from './navigation';

export const taodbox = $state({ actif: false, manettes: 0 });

const FOCALISABLES = 'button:not([disabled]), a[href], input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])';

/** La zone où le focus peut aller : la fenêtre maison du dessus s'il y en a une, sinon la page. */
function zone(): ParentNode {
  const d = document.querySelectorAll('[role="dialog"]');
  return d.length ? d[d.length - 1] : document;
}

function visibles(): HTMLElement[] {
  return [...zone().querySelectorAll<HTMLElement>(FOCALISABLES)].filter((e) => {
    const r = e.getBoundingClientRect();
    return r.width > 0 && r.height > 0 && getComputedStyle(e).visibility !== 'hidden';
  });
}

/** Déplace le focus dans une direction (ou sur le premier élément s'il n'y en a pas). */
export function deplacer(d: Direction) {
  const l = visibles();
  if (!l.length) return;
  const i = l.indexOf(document.activeElement as HTMLElement);
  if (i < 0) {
    l[0].focus();
    return;
  }
  const boites = l.map((e) => {
    const r = e.getBoundingClientRect();
    return { x: r.left, y: r.top, l: r.width, h: r.height };
  });
  const n = voisin(boites, i, d);
  if (n !== null) {
    l[n].focus();
    l[n].scrollIntoView({ block: 'nearest', inline: 'nearest', behavior: 'smooth' });
  }
}

function touche(cle: string) {
  const cible = (document.activeElement as HTMLElement) ?? document.body;
  cible.dispatchEvent(new KeyboardEvent('keydown', { key: cle, bubbles: true, cancelable: true }));
}

/** A : valider l'élément qui a le focus. */
function valider() {
  const e = document.activeElement as HTMLElement | null;
  if (!e || e === document.body) {
    visibles()[0]?.focus();
    return;
  }
  if (e.tagName === 'INPUT' || e.tagName === 'SELECT') touche('Enter');
  else e.click();
}

// Boutons de la disposition standard (W3C) : 0 = A, 1 = B, 12–15 = croix.
const A = 0;
const B = 1;
const CROIX: [number, Direction][] = [
  [12, 'haut'],
  [13, 'bas'],
  [14, 'gauche'],
  [15, 'droite'],
];

let boucle: number | null = null;
let avant = new Set<number>();
let direction: Direction | null = null;
let prochaine = 0;

function lire() {
  const pads = navigator.getGamepads ? [...navigator.getGamepads()].filter((p): p is Gamepad => !!p) : [];
  taodbox.manettes = pads.length;
  const appuyes = new Set<number>();
  let d: Direction | null = null;
  for (const p of pads) {
    p.buttons.forEach((b, i) => b.pressed && appuyes.add(i));
    for (const [i, dir] of CROIX) if (p.buttons[i]?.pressed) d = dir;
    d ??= directionDuStick(p.axes[0] ?? 0, p.axes[1] ?? 0);
  }
  if (appuyes.has(A) && !avant.has(A)) valider();
  if (appuyes.has(B) && !avant.has(B)) touche('Escape');
  // Direction : un premier pas, puis la répétition après une pause (comme une touche tenue).
  const t = performance.now();
  if (d && d !== direction) {
    deplacer(d);
    prochaine = t + 380;
  } else if (d && t >= prochaine) {
    deplacer(d);
    prochaine = t + 130;
  }
  direction = d;
  avant = appuyes;
  boucle = requestAnimationFrame(lire);
}

/** Flèches du clavier : la même navigation géométrique (sauf dans un champ de saisie). */
function fleches(e: KeyboardEvent) {
  const dirs: Record<string, Direction> = { ArrowUp: 'haut', ArrowDown: 'bas', ArrowLeft: 'gauche', ArrowRight: 'droite' };
  const d = dirs[e.key];
  const cible = e.target as HTMLElement;
  if (!d || cible.tagName === 'INPUT' || cible.tagName === 'TEXTAREA' || cible.tagName === 'SELECT') return;
  e.preventDefault();
  deplacer(d);
}

export function demarrerManette() {
  if (taodbox.actif) return;
  taodbox.actif = true;
  window.addEventListener('keydown', fleches);
  boucle = requestAnimationFrame(lire);
}

export function arreterManette() {
  taodbox.actif = false;
  window.removeEventListener('keydown', fleches);
  if (boucle !== null) cancelAnimationFrame(boucle);
  boucle = null;
}
