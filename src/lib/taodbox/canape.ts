// Le mode canapé (Taodbox) : tout se fait à la manette. Ce qui demande la souris ou le clavier (sélecteur de fichiers
// de Windows, saisie de texte, installeur ou désinstalleur de Windows) n'est pas proposé ; s'il est atteint quand même,
// la personne est renvoyée à l'écran du PC (✅ Seb 19/10, lot 0.76.0).
import { informer } from '$lib/dialogues/fenetres.svelte';

/** Frogtend est-il en Taodbox ? L'attribut `data-taodbox` n'est posé QUE par la vraie Taodbox (la fenêtre du menu en
 * jeu se pilote aussi à la manette, mais au bureau elle n'est pas « au canapé »). */
export function auCanape(): boolean {
  return typeof document !== 'undefined' && document.documentElement.hasAttribute('data-taodbox');
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
