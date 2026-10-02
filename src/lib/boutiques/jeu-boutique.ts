// Un jeu de boutique dans la ludothèque (lot 9, id négatif) : jouer ou installer passe par sa boutique — Steam
// directement, les autres par GOG Galaxy (qui les relie).
import { api, type JeuResume } from '$lib/api';

/** Les boutiques proposées dans le filtre de la ludothèque. */
export const BOUTIQUES: { valeur: string; libelle: string }[] = [
  { valeur: 'firehouse', libelle: 'Firehouse' },
  { valeur: 'steam', libelle: 'Steam' },
  { valeur: 'gog', libelle: 'GOG' },
  { valeur: 'epic', libelle: 'Epic Games' },
  { valeur: 'xboxone', libelle: 'Xbox' },
  { valeur: 'uplay', libelle: 'Ubisoft Connect' },
  { valeur: 'origin', libelle: 'EA' },
];

export const estJeuDeBoutique = (j: Pick<JeuResume, 'id'>) => j.id < 0;

/** Comment ouvrir ce jeu : par Steam (appid) ou par GOG Galaxy (sa clé). */
export function moyenDOuverture(j: JeuResume): { par: 'steam'; appid: string } | { par: 'galaxy'; cle: string } | null {
  const cle = j.cle_boutique ?? '';
  if (!cle) return null;
  if (j.source === 'steam') return { par: 'steam', appid: cle };
  if (j.source === 'galaxy') return { par: 'galaxy', cle };
  return null;
}

/** Jouer (installé) ou installer, par la boutique. */
export async function ouvrirJeuDeBoutique(j: JeuResume) {
  const m = moyenDOuverture(j);
  if (!m) return;
  if (m.par === 'steam') await api.steamOuvrir(m.appid, j.installe ? 'jouer' : 'installer');
  else await api.galaxyOuvrir(m.cle);
}
