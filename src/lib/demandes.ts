// Demander un jeu absent (lot 6) : ce qu'on montre pour un résultat de la recherche.
import type { ResultatRecherche } from '$lib/api';

/** Le jeu peut-il être demandé, et sinon pourquoi. `demandes` : les jeux déjà demandés pendant cette visite. */
export function etatDemande(r: ResultatRecherche, demandes: number[]): { texte: string; demandable: boolean } {
  if (demandes.includes(r.launchbox_id)) return { texte: '📨 Demandé', demandable: false };
  if (!r.deja) return { texte: '', demandable: true };
  if (r.deja.statut === 'possede') return { texte: '✅ Déjà dans le Catalogue Firehouse', demandable: false };
  return { texte: '⏳ Déjà demandé : Firehouse le cherche', demandable: false };
}
