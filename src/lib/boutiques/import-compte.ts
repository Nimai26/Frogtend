// Les messages communs aux imports des jeux d'un COMPTE de boutique connecté par sa page officielle (Steam, lot C2 ;
// GOG, lot C4a) : connexion expirée, jeux nouveaux, « une fois par jour ». Purs et testés ; chaque boutique garde ses
// propres textes et sa propre clé de stockage (celle de Steam ne change pas : `frogtend.steamExpire.<profil>`).
import { messageNouveaux } from './jeu-boutique';

/** Ce qu'un import par la connexion rend (la même forme pour Steam et GOG). */
export interface ResultatImportCompte {
  reconnecter: boolean;
  total: number;
  nouveaux: number;
  nouveaux_installes: number;
}

/** Le message d'une connexion expirée selon l'écran : « dans 🛒 Boutiques » seulement hors de cette page, et « au
 * bureau » depuis Taodbox (la connexion demande la souris). `nom` : « Steam », « GOG ». */
export function messageExpire(nom: string, chemin: string): string {
  if (chemin.startsWith('/taodbox')) return `🔑 Ta connexion à ${nom} a expiré : au bureau, dans 🛒 Boutiques, reconnecte-toi.`;
  if (chemin.startsWith('/boutiques')) return `🔑 Ta connexion à ${nom} a expiré : reconnecte-toi (bouton « 🔑 Me reconnecter à ${nom} »).`;
  return `🔑 Ta connexion à ${nom} a expiré : dans 🛒 Boutiques, reconnecte-toi.`;
}

/** Ce qu'un import par la connexion fait dire. À la main : toujours un message (« Rien de nouveau » compris) ; à
 * l'ouverture de Frogtend (discret) : seulement des jeux nouveaux ou une connexion expirée. `boutique` : sa clé
 * (« steam », « gog ») pour le message des jeux nouveaux. */
export function annonceImport(boutique: string, nom: string, r: ResultatImportCompte, aLaMain: boolean, chemin = ''): { texte: string; ton?: 'alerte' } | null {
  if (r.reconnecter) return { texte: messageExpire(nom, chemin), ton: 'alerte' };
  const m = messageNouveaux([[boutique, r.nouveaux, r.nouveaux_installes]]);
  if (m) return { texte: m };
  return aLaMain ? { texte: `Rien de nouveau : tes jeux ${nom} sont déjà dans ta ludothèque.` } : null;
}

/** Une chose (une connexion expirée) est-elle à dire AUJOURD'HUI, sous cette clé ? Une fois par jour ; retenu dans ce
 * navigateur seulement, sans rien de secret. Sans stockage, ou stockage refusé : dite. */
export function direUneFoisParJour(cle: string, jour: string, stockage: Pick<Storage, 'getItem' | 'setItem'> | null): boolean {
  try {
    if (stockage?.getItem(cle) === jour) return false;
    stockage?.setItem(cle, jour);
  } catch {
    // Stockage refusé : dit, comme sans stockage.
  }
  return true;
}

/** Le stockage de ce navigateur, s'il est permis. */
export function stockageLocal(): Storage | null {
  try {
    return typeof localStorage === 'undefined' ? null : localStorage;
  } catch {
    return null;
  }
}

/** L'écran où l'on est (pour les messages). */
export function cheminActuel(): string {
  try {
    return typeof location === 'undefined' ? '' : location.pathname;
  } catch {
    return '';
  }
}

/** Le jour d'aujourd'hui (AAAA-MM-JJ), pour « une fois par jour ». */
export function aujourdhui(): string {
  return new Date().toISOString().slice(0, 10);
}
