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

/**
 * Sélectionne un élément : le focus, et la marque `data-choisi` (visible même quand la fenêtre n'a pas le premier
 * plan, où le navigateur n'affiche pas `:focus` — le menu en jeu reçoit alors la manette du cœur quand même).
 */
export function selectionner(e: HTMLElement | null | undefined) {
  if (!e) return;
  document.querySelectorAll('[data-choisi]').forEach((x) => x.removeAttribute('data-choisi'));
  e.setAttribute('data-choisi', '');
  // `focusVisible` : le contour s'affiche même si la dernière action était un clic (Chromium ne montre
  // `:focus-visible` d'un focus posé par le code qu'après une touche du clavier ; essai de Seb, 29/10 : à la manette,
  // rien ne semblait bouger avant d'avoir tapé une flèche). Un filet CSS (`[data-choisi]`) le double.
  e.focus({ preventScroll: true, focusVisible: true } as FocusOptions);
  e.scrollIntoView({ block: 'nearest', inline: 'nearest' });
}

/** Le point de départ de la manette : l'élément qui a le focus ; sinon (focus perdu sur la page après un clic, ou au
 * retour d'une autre fenêtre) l'élément choisi en dernier s'il est encore là. */
function courant(l: HTMLElement[]): HTMLElement | null {
  const a = document.activeElement as HTMLElement | null;
  if (a && a !== document.body && a !== document.documentElement) return a;
  const choisi = l.find((e) => e.hasAttribute('data-choisi'));
  return choisi ?? null;
}

/** Déplace le focus dans une direction (ou sur le premier élément s'il n'y en a pas). */
export function deplacer(d: Direction) {
  const l = visibles();
  if (!l.length) return;
  const c = courant(l);
  if (c && c !== document.activeElement) {
    // Le focus était perdu : on le rend d'abord à l'élément choisi (aucun saut vers le haut de la page).
    selectionner(c);
    return;
  }
  const i = c ? l.indexOf(c) : -1;
  if (i < 0) {
    selectionner(l[0]);
    return;
  }
  const boites = l.map((e) => {
    const r = e.getBoundingClientRect();
    return { x: r.left, y: r.top, l: r.width, h: r.height };
  });
  const n = voisin(boites, i, d);
  if (n !== null) selectionner(l[n]);
  // Le menu en jeu boucle (✅ Seb 30/10) : en haut, « haut » va au dernier élément ; en bas, « bas » au premier.
  else if (enBoucle && (d === 'haut' || d === 'bas')) selectionner(boucler(l, boites, d));
}

/** Le menu en jeu se parcourt en boucle verticale (réglé par `demarrerManette`). */
let enBoucle = false;

/** L'élément où l'on arrive en sortant par le haut (le plus bas) ou par le bas (le plus haut) de la liste. */
function boucler(l: HTMLElement[], boites: { x: number; y: number; l: number; h: number }[], d: 'haut' | 'bas'): HTMLElement {
  let k = 0;
  for (let j = 1; j < l.length; j++) {
    const plusBas = boites[j].y > boites[k].y;
    if (d === 'haut' ? plusBas : boites[j].y < boites[k].y) k = j;
  }
  return l[k];
}

function touche(cle: string) {
  const cible = (document.activeElement as HTMLElement) ?? document.body;
  const e = new KeyboardEvent('keydown', { key: cle, bubbles: true, cancelable: true });
  fabriquees.add(e);
  cible.dispatchEvent(e);
}

// --- Les doublons. Un autre programme peut transformer la MÊME manette en touches du clavier (Steam ouvert : sa
// « configuration bureau » fait croix → flèches, A → Entrée, B → Échap ; essai de Seb, 28/10 : la croix sautait un
// élément sur deux, « Options du jeu » s'ouvrait et validait aussitôt sa 1re option). Un appui arrivé par les DEUX
// voies à moins de `DOUBLON` ms ne compte qu'une fois ; chaque paire est consommée (deux appuis rapides restent deux).
const DOUBLON = 250;
/** Nos propres touches (l'Échap du B…) : jamais prises pour un doublon. */
const fabriquees = new WeakSet<Event>();
type Recu = { c: Commande; t: number };
let parLaManette: Recu[] = [];
let parLeClavier: Recu[] = [];
/** Les touches dont l'appui a été écarté : leur relâchement (et leur répétition) l'est aussi (Espace clique au relâché). */
const ecartees = new Set<string>();

function commandeDeTouche(e: KeyboardEvent): Commande | null {
  const dirs: Record<string, Commande> = { ArrowUp: 'haut', ArrowDown: 'bas', ArrowLeft: 'gauche', ArrowRight: 'droite', Enter: 'valider', ' ': 'valider', Escape: 'retour' };
  return dirs[e.key] ?? null;
}

/** Retire de `l` un appui de la même commande reçu il y a moins de `DOUBLON` ms : `true` s'il y en avait un. */
function consommer(l: Recu[], c: Commande, t: number): boolean {
  const i = l.findIndex((r) => r.c === c && t - r.t <= DOUBLON);
  if (i >= 0) l.splice(i, 1);
  return i >= 0;
}

const recents = (l: Recu[], t: number) => l.filter((r) => t - r.t <= DOUBLON);

/** La fenêtre perd le focus : les relâchements attendus n'arriveront pas ici. */
function oublierEcartees() {
  ecartees.clear();
}

/** Une commande venue de la MANETTE (lue par Frogtend ou par son cœur) : ignorée si la même vient d'arriver par le
 * clavier (fabriquée par un autre programme depuis le même appui). */
export function appliquerManette(c: Commande) {
  const t = performance.now();
  parLeClavier = recents(parLeClavier, t);
  if (consommer(parLeClavier, c, t)) return;
  parLaManette = [...recents(parLaManette, t), { c, t }];
  appliquer(c);
}

/** Avant tout autre écouteur (capture sur la fenêtre) : une touche qui double un appui de la manette est écartée
 * (ni navigation, ni clic natif d'un bouton). Un vrai clavier seul n'est jamais touché. */
function filtrerDoublons(e: KeyboardEvent) {
  if (fabriquees.has(e)) return;
  const id = e.code || e.key;
  // Échap, Entrée ou Espace TENUS (Steam répète la touche quand on tient B ou A) : une seule action, jamais deux
  // retours ou deux validations (essai de Seb, 28/10 : B tenu fermait les Options puis le menu). Les flèches gardent
  // leur répétition.
  if (e.type === 'keydown' && e.repeat && (e.key === 'Escape' || e.key === 'Enter' || e.key === ' ')) {
    e.preventDefault();
    e.stopImmediatePropagation();
    return;
  }
  if (e.type === 'keyup' || e.repeat) {
    if (ecartees.has(id)) {
      if (e.type === 'keyup') ecartees.delete(id);
      e.preventDefault();
      e.stopImmediatePropagation();
    }
    return;
  }
  // Un nouvel appui de cette touche : un relâchement jamais reçu (la fenêtre a perdu le focus) ne la garde pas écartée.
  ecartees.delete(id);
  const c = commandeDeTouche(e);
  if (!c) return;
  const t = performance.now();
  parLaManette = recents(parLaManette, t);
  if (consommer(parLaManette, c, t)) {
    ecartees.add(id);
    e.preventDefault();
    e.stopImmediatePropagation();
    return;
  }
  parLeClavier = [...recents(parLeClavier, t), { c, t }];
}

/** A : valider l'élément qui a le focus. */
function valider() {
  const l = visibles();
  const e = courant(l);
  if (!e) {
    selectionner(l[0]);
    return;
  }
  if (e !== document.activeElement) selectionner(e);
  if (e.tagName === 'INPUT' || e.tagName === 'SELECT') touche('Enter');
  else e.click();
}

/** Ce qui a pris la main (le lecteur du livret…) : il reçoit les commandes avant la navigation, et rend `true`
 * s'il s'en est servi. Le dernier arrivé passe en premier ; une fenêtre maison ouverte par-dessus reste prioritaire. */
type Preneur = (c: Commande) => boolean;
const preneurs: Preneur[] = [];

/** Prend la main ; rend la fonction qui la rend. */
export function prendreLaMain(p: Preneur): () => void {
  preneurs.push(p);
  return () => {
    const i = preneurs.lastIndexOf(p);
    if (i >= 0) preneurs.splice(i, 1);
  };
}

/** LB / RB dans un lecteur (livret, musique, vidéo, images) : le bouton précédent / suivant de sa barre, en boucle
 * (la croix y reste au contenu : pages, piste, volume — essai de Seb, 28/10 : les boutons étaient inaccessibles). */
export function parcourir(conteneur: ParentNode | null | undefined, sens: 1 | -1) {
  if (!conteneur) return;
  const l = [...conteneur.querySelectorAll<HTMLElement>(FOCALISABLES)];
  if (!l.length) return;
  const i = l.indexOf(document.activeElement as HTMLElement);
  selectionner(l[i < 0 ? (sens > 0 ? 0 : l.length - 1) : (i + sens + l.length) % l.length]);
}

/** Une commande, quelle que soit sa source (manette, clavier, cœur de Frogtend). */
export function appliquer(c: Commande) {
  const p = preneurs[preneurs.length - 1];
  if (p && !fenetreOuverte() && p(c)) return;
  // LB / RB : seulement pour un lecteur qui les prend ; ailleurs, rien (jamais un déplacement ni un retour).
  if (c === 'precedent' || c === 'suivant') return;
  if (c === 'valider') valider();
  else if (c === 'retour') touche('Escape');
  else deplacer(c);
}

/** Y : « ⚙ Gérer le jeu » (✅ Seb 19/10), comme la touche Menu (≣) du clavier. Hors des commandes de navigation : un
 * lecteur qui a pris la main se fermerait sur une commande inconnue ; rien non plus par-dessus une fenêtre. Rend
 * `true` si la touche a été envoyée. */
export function appuiY(): boolean {
  if (fenetreOuverte() || preneurs.length) return false;
  // Focus perdu (après un clic) : la touche irait à la page, pas à la carte choisie.
  const c = courant(visibles());
  if (c && c !== document.activeElement) selectionner(c);
  touche('ContextMenu');
  return true;
}

// Boutons de la disposition standard (W3C) : 0 = A, 1 = B, 3 = Y, 12–15 = croix.
const A = 0;
const B = 1;
const Y = 3;
const LB = 4;
const RB = 5;
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
  if (nouveaux.has(A)) appliquerManette('valider');
  if (nouveaux.has(B)) appliquerManette('retour');
  if (nouveaux.has(Y)) appuiY();
  if (nouveaux.has(LB)) appliquerManette('precedent');
  if (nouveaux.has(RB)) appliquerManette('suivant');
  // Direction : un premier pas, puis la répétition après une pause (comme une touche tenue).
  const t = performance.now();
  if (d && d !== direction) {
    appliquerManette(d);
    prochaine = t + 380;
  } else if (d && t >= prochaine) {
    appliquerManette(d);
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

/** `manette: false` : seulement les flèches du clavier (le menu en jeu reçoit la manette du cœur de Frogtend). */
export function demarrerManette(options: { manette?: boolean; boucle?: boolean } = {}) {
  if (taodbox.actif) return;
  taodbox.actif = true;
  enBoucle = options.boucle === true;
  // Les boutons déjà tenus au démarrage (la combinaison qui vient d'ouvrir le menu…) ne comptent pas.
  avant = appuyes(manettes());
  direction = null;
  parLaManette = [];
  parLeClavier = [];
  ecartees.clear();
  window.addEventListener('keydown', filtrerDoublons, true);
  window.addEventListener('keyup', filtrerDoublons, true);
  window.addEventListener('blur', oublierEcartees);
  window.addEventListener('keydown', fleches);
  if (options.manette !== false) boucle = requestAnimationFrame(lire);
}

export function arreterManette() {
  taodbox.actif = false;
  enBoucle = false;
  window.removeEventListener('keydown', filtrerDoublons, true);
  window.removeEventListener('keyup', filtrerDoublons, true);
  window.removeEventListener('blur', oublierEcartees);
  window.removeEventListener('keydown', fleches);
  if (boucle !== null) cancelAnimationFrame(boucle);
  boucle = null;
}
