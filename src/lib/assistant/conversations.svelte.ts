// Les conversations avec l'assistant, gardées en mémoire tant que Frogtend est ouvert (jamais écrites sur le disque :
// elles peuvent parler de tout). Une par jeu, et une générale.
import type { ActionProposee } from './actions';

export interface Echange {
  question: string;
  texte: string;
  actions: ActionProposee[];
  attente: boolean;
  erreur?: string;
}

export const conversations = $state<Record<string, Echange[]>>({});

/** À la fermeture d'un profil : rien d'une personne ne reste pour la suivante. */
export function oublierToutes() {
  for (const k of Object.keys(conversations)) delete conversations[k];
}
