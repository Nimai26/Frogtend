// Manette → clavier pour les jeux PC qui ne connaissent que le clavier (Seb, 16/10 : « le plus généraliste
// possible »). Ce que l'interface propose ; le cœur traduit pendant la partie (`src-tauri/src/clavier.rs`). Les noms
// des boutons et des touches sont LES MÊMES que dans le cœur (un test du cœur le vérifie).

/** Une correspondance : nom de bouton → nom de touche. */
export type Correspondance = Record<string, string>;

/** Les boutons de la manette (nom stable, nom montré). Les directions : la croix ou le stick gauche. */
export const BOUTONS: { id: string; nom: string }[] = [
  { id: 'haut', nom: 'Haut' },
  { id: 'bas', nom: 'Bas' },
  { id: 'gauche', nom: 'Gauche' },
  { id: 'droite', nom: 'Droite' },
  { id: 'a', nom: 'A' },
  { id: 'b', nom: 'B' },
  { id: 'x', nom: 'X' },
  { id: 'y', nom: 'Y' },
  { id: 'lb', nom: 'LB' },
  { id: 'rb', nom: 'RB' },
  { id: 'lt', nom: 'LT' },
  { id: 'rt', nom: 'RT' },
  { id: 'start', nom: 'Start' },
  { id: 'select', nom: 'View (Back)' },
  { id: 'l3', nom: 'L3 (clic du stick gauche)' },
  { id: 'r3', nom: 'R3 (clic du stick droit)' },
];

/** Les touches qu'on peut envoyer (les mêmes que le cœur), rangées par usage : les quatre « Déplacement » partent par
 * leur POSITION (ZQSD sur un clavier français, WASD sur un clavier anglais), les autres par leur nom. */
export const TOUCHES: string[] = [
  'Déplacement haut', 'Déplacement gauche', 'Déplacement bas', 'Déplacement droite',
  'Haut', 'Bas', 'Gauche', 'Droite', 'Entrée', 'Espace', 'Échap', 'Maj', 'Ctrl', 'Alt', 'Tab', 'Retour arrière',
  'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X',
  'Y', 'Z',
  '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
  'F1', 'F2', 'F3', 'F4', 'F5', 'F6', 'F7', 'F8', 'F9', 'F10', 'F11', 'F12',
  'Inser', 'Suppr', 'Origine', 'Fin', 'Page préc.', 'Page suiv.',
];

/** Ce qu'il faut savoir d'une touche (dans la liste du « bouton par bouton »). */
export function detailTouche(t: string): string | undefined {
  if (t.startsWith('Déplacement')) return 'la touche à cette PLACE : Z Q S D sur un clavier français, W A S D sur un clavier anglais';
  if (t === 'Échap') return 'attention : quitte certains jeux sans demander';
  if (t === 'Alt') return 'attention : seule, elle peut ouvrir le menu de la fenêtre du jeu';
  return undefined;
}

const fleches: Correspondance = { haut: 'Haut', bas: 'Bas', gauche: 'Gauche', droite: 'Droite' };

/** Les préréglages courants : de quoi jouer tout de suite à la plupart des jeux au clavier. View (Back) n'est jamais
 * associé d'office ; Échap seulement sur Start (le bouton « pause » attendu). */
export const PREREGLAGES: { id: string; nom: string; detail: string; correspondance: Correspondance }[] = [
  {
    id: 'retro',
    nom: '🕹 Flèches + Z X C (jeux rétro, jeux de fans)',
    detail: 'croix → flèches, A → Z, X → X, B → C, Y → V, LB → A, RB → S, Start → Entrée',
    correspondance: { ...fleches, a: 'Z', x: 'X', b: 'C', y: 'V', lb: 'A', rb: 'S', start: 'Entrée' },
  },
  {
    id: 'moderne',
    nom: '🎯 Touches de déplacement + Espace (jeux PC)',
    detail: 'croix → les 4 touches de déplacement (ZQSD ou WASD selon ton clavier), A → Espace, B → Ctrl, X → E, Y → R, LT → Maj, RT → Entrée, Start → Échap',
    correspondance: {
      haut: 'Déplacement haut', gauche: 'Déplacement gauche', bas: 'Déplacement bas', droite: 'Déplacement droite',
      a: 'Espace', b: 'Ctrl', x: 'E', y: 'R', lt: 'Maj', rt: 'Entrée', start: 'Échap',
    },
  },
  {
    id: 'simple',
    nom: '⏎ Flèches + Entrée (jeux simples, aventures)',
    detail: 'croix → flèches, A → Entrée, B → Retour arrière, X → Espace, Y → Tab, Start → Échap',
    correspondance: { ...fleches, a: 'Entrée', b: 'Retour arrière', x: 'Espace', y: 'Tab', start: 'Échap' },
  },
];

/** Le libellé d'une correspondance (dans « Gérer le jeu »). */
export function libelleClavier(c: Correspondance | undefined): string {
  if (!c || !Object.keys(c).length) return 'non';
  const p = PREREGLAGES.find((x) => JSON.stringify(trier(x.correspondance)) === JSON.stringify(trier(c)));
  const n = Object.keys(c).length;
  return p ? p.nom.replace(/^\S+\s/, '') : `réglé à la main (${n} bouton${n > 1 ? 's' : ''})`;
}

/** Une correspondance propre : seulement des boutons et des touches connus. */
export function nettoyer(c: unknown): Correspondance {
  const r: Correspondance = {};
  if (!c || typeof c !== 'object') return r;
  for (const [b, t] of Object.entries(c as Record<string, unknown>)) {
    if (BOUTONS.some((x) => x.id === b) && typeof t === 'string' && TOUCHES.includes(t)) r[b] = t;
  }
  return r;
}

/** Deux correspondances identiques ? */
export function memes(a: Correspondance, b: Correspondance): boolean {
  return JSON.stringify(trier(a)) === JSON.stringify(trier(b));
}

function trier(c: Correspondance): [string, string][] {
  return Object.entries(c).sort(([a], [b]) => a.localeCompare(b));
}
