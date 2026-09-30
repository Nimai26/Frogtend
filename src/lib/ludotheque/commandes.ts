// Les commandes d'un jeu (automatique, clavier et souris, ou un réglage de manette de référence).
import type { CommandesJeu } from '$lib/reglages/reglages';

/** Les libellés des genres de manette. */
export const GENRES_MANETTE: Record<string, string> = { Wiimote: 'Wiimote', GCPad: 'Manette GameCube', Pad: 'Manette' };

/** Le libellé d'un choix de commandes. */
export function libelleCommandes(c: CommandesJeu | undefined): string {
  if (!c || c.mode === 'auto') return 'Automatique';
  if (c.mode === 'clavier') return 'Clavier et souris';
  return `${GENRES_MANETTE[c.genre ?? ''] ?? c.genre} : ${c.reference}`;
}
