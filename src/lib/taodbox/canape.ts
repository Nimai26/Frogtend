// Le mode canapé (Taodbox) : tout se fait à la manette. Ce qui demande la souris ou le clavier (sélecteur de fichiers
// de Windows, saisie de texte, installeur ou désinstalleur de Windows) n'est pas proposé ; s'il est atteint quand même,
// la personne est renvoyée à l'écran du PC (✅ Seb 19/10, lot 0.76.0).
import { informer } from '$lib/dialogues/fenetres.svelte';

/** Frogtend est-il en Taodbox ? L'attribut `data-taodbox` n'est posé QUE par la vraie Taodbox (la fenêtre du menu en
 * jeu se pilote aussi à la manette, mais au bureau elle n'est pas « au canapé »). */
export function auCanape(): boolean {
  return typeof document !== 'undefined' && document.documentElement.hasAttribute('data-taodbox');
}

/** Entre en Taodbox (ou en sort) : pose `data-taodbox` et la hauteur de l'ÉCRAN, sur laquelle toutes les tailles de
 * Taodbox sont calculées (base.css, Seb 19/11). L'écran et non la fenêtre : le menu en jeu, plus petit que l'écran,
 * garde les mêmes tailles que la grille. */
export function poserModeCanape(actif: boolean): void {
  const racine = document.documentElement;
  racine.toggleAttribute('data-taodbox', actif);
  const hauteur = typeof screen !== 'undefined' && screen.height > 0 ? screen.height : window.innerHeight;
  if (actif && hauteur > 0) racine.style.setProperty('--hauteur-ecran', `${hauteur}px`);
  else racine.style.removeProperty('--hauteur-ecran');
}

/** En Taodbox : explique que `quoi` se fait à l'écran du PC et rend `true` (l'appelant s'arrête) ; sinon `false`. */
export async function seulementAuBureau(quoi: string): Promise<boolean> {
  if (!auCanape()) return false;
  await informer(
    '🖥 À faire sur l’écran du PC',
    `${quoi} : ça se fait à la souris. Ouvre Frogtend sur l’écran du PC, sans Taodbox, pour le faire.`,
  );
  return true;
}
