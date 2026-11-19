// 🏷 Les icônes des boutiques sur les pastilles d'origine (Seb, 19/11 ; PLAN « Icônes des boutiques sur les
// pastilles »). Frogtend n'embarque AUCUN logo de marque. Ordre de secours :
//   1. l'icône du LANCEUR installé sur ce PC, lue par Windows dans son programme (cœur : `boutique_icone`) ;
//   2. plus tard, l'icône servie par Firehouse (BESOINS-API n° 55, PAS encore servie : voir `iconeFirehouse`) ;
//   3. le texte (« Steam », « Epic »…), toujours là quand rien d'autre n'existe (Xbox, Firehouse, disque, navigateur).
// Chaque boutique n'est demandée qu'UNE fois pour toute l'application.
import { isTauri } from '@tauri-apps/api/core';
import { api } from '$lib/api';

/** L'icône connue de chaque boutique : une adresse `data:image/png`, `null` si aucune (le texte reste). */
export const icones = $state<Record<string, string | null>>({});
const demandees = new Set<string>();

/** La boutique dont on cherche l'icône pour une origine, ou `null` : pas de lanceur à lire (Xbox est une application
 *  du Store, Firehouse et le disque ne sont pas des boutiques). */
export function boutiqueDeLIcone(origine: string): string | null {
  const b = origine === 'ea' ? 'origin' : origine;
  return ['steam', 'epic', 'gog', 'uplay', 'amazon', 'battlenet', 'origin'].includes(b) ? b : null;
}

/** Une adresse d'icône acceptable : seulement une image PNG en `data:` (jamais une adresse qui partirait au dehors). */
export function estAdresseIcone(u: unknown): u is string {
  return typeof u === 'string' && /^data:image\/png;base64,[A-Za-z0-9+/]+=*$/.test(u);
}

/** L'icône à montrer pour une origine, ou `null` pour garder le texte. */
export function iconeDe(origine: string, connues: Record<string, string | null> = icones): string | null {
  const b = boutiqueDeLIcone(origine);
  return (b && connues[b]) || null;
}

/** Secours n° 2 (BESOINS-API n° 55) : l'icône d'une boutique servie par Firehouse. Aucune route n'existe encore :
 *  quand Firehouse la servira, c'est ICI qu'on la demandera (et seulement si le lanceur n'a rien donné). */
async function iconeFirehouse(_boutique: string): Promise<string | null> {
  return null;
}

/** Demande (une seule fois pour toute l'appli) l'icône de la boutique d'une origine. Hors de l'application
 *  (navigateur, tests) : rien, le texte reste. */
export function demanderIcone(origine: string): void {
  const b = boutiqueDeLIcone(origine);
  if (!b || demandees.has(b) || !isTauri()) return;
  demandees.add(b);
  api
    .boutiqueIcone(b)
    .then(async (u) => (estAdresseIcone(u) ? u : await iconeFirehouse(b)))
    .catch(() => null)
    .then((u) => {
      icones[b] = estAdresseIcone(u) ? u : null;
    });
}
