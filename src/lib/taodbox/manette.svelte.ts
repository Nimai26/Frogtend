// Taodbox (lot 5) et menu en jeu (lot OSD) : la manette pilote les écrans de Frogtend (choix du profil, fenêtres
// maison, ludothèque, menu en jeu…). La croix et le stick gauche déplacent le focus selon la géométrie, A valide,
// B fait Échap (charte § 7). Une seule façon d'agir pour toutes les sources : `appliquer(commande)` — la manette lue
// par l'API Gamepad (fenêtre au premier plan), les flèches du clavier, et le cœur de Frogtend (manette lue pendant
// une partie, étape 3 de l'OSD).
import { directionDuStick, nouveauxAppuis, voisin, type Commande, type Direction } from './navigation';

export const taodbox = $state({ actif: false, manettes: 0 });

const FOCALISABLES = 'button:not([disabled]), a[href], input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])';

/** Les fenêtres maison (`confirmer` a le rôle `alertdialog`, les autres `dialog`). */
const FENETRES_MAISON = '[role="dialog"], [role="alertdialog"]';

/** Une fenêtre maison est-elle ouverte ? (Elle a alors la main : Échap/B la ferment, la croix reste dedans.) */
export function fenetreOuverte(): boolean {
  return !!document.querySelector(FENETRES_MAISON);
}

/** La zone où le focus peut aller : la fenêtre maison du dessus s'il y en a une, sinon la page. */
function zone(): ParentNode {
  const d = document.querySelectorAll(FENETRES_MAISON);
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

/** Une commande, quelle que soit sa source (manette, clavier, cœur de Frogtend). */
export function appliquer(c: Commande) {
  if (c === 'valider') valider();
  else if (c === 'retour') touche('Escape');
  else deplacer(c);
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

function manettes(): Gamepad[] {
  return navigator.getGamepads ? [...navigator.getGamepads()].filter((p): p is Gamepad => !!p) : [];
}

function appuyes(pads: Gamepad[]): Set<number> {
  const l = new Set<number>();
  for (const p of pads) p.buttons.forEach((b, i) => b.pressed && l.add(i));
  return l;
}

function lire() {
  const pads = manettes();
  taodbox.manettes = pads.length;
  const maintenant = appuyes(pads);
  // Une fenêtre qui n'a pas le premier plan ignore la manette (le menu en jeu caché, la fenêtre principale pendant
  // que le menu est ouvert) : un même A n'agit jamais dans deux fenêtres.
  if (!document.hasFocus()) {
    avant = maintenant;
    direction = null;
    boucle = requestAnimationFrame(lire);
    return;
  }
  let d: Direction | null = null;
  for (const p of pads) {
    for (const [i, dir] of CROIX) if (p.buttons[i]?.pressed) d = dir;
    d ??= directionDuStick(p.axes[0] ?? 0, p.axes[1] ?? 0);
  }
  const nouveaux = nouveauxAppuis(avant, maintenant);
  if (nouveaux.has(A)) appliquer('valider');
  if (nouveaux.has(B)) appliquer('retour');
  // Direction : un premier pas, puis la répétition après une pause (comme une touche tenue).
  const t = performance.now();
  if (d && d !== direction) {
    appliquer(d);
    prochaine = t + 380;
  } else if (d && t >= prochaine) {
    appliquer(d);
    prochaine = t + 130;
  }
  direction = d;
  avant = maintenant;
  boucle = requestAnimationFrame(lire);
}

/** Flèches du clavier : la même navigation géométrique (sauf dans un champ de saisie). */
function fleches(e: KeyboardEvent) {
  const dirs: Record<string, Direction> = { ArrowUp: 'haut', ArrowDown: 'bas', ArrowLeft: 'gauche', ArrowRight: 'droite' };
  const d = dirs[e.key];
  const cible = e.target as HTMLElement;
  if (!d || cible.tagName === 'INPUT' || cible.tagName === 'TEXTAREA' || cible.tagName === 'SELECT') return;
  e.preventDefault();
  appliquer(d);
}

export function demarrerManette() {
  if (taodbox.actif) return;
  taodbox.actif = true;
  // Les boutons déjà tenus au démarrage (la combinaison qui vient d'ouvrir le menu…) ne comptent pas.
  avant = appuyes(manettes());
  direction = null;
  window.addEventListener('keydown', fleches);
  boucle = requestAnimationFrame(lire);
}

export function arreterManette() {
  taodbox.actif = false;
  window.removeEventListener('keydown', fleches);
  if (boucle !== null) cancelAnimationFrame(boucle);
  boucle = null;
}
