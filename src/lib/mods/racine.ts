// Un mod rangé dans UN seul dossier (« MonMod/… ») : le cœur s'arrête après le téléchargement et demande où le poser
// (✅ Seb 25/10 : « Demander »). Rien n'est posé avant la réponse ; fermer la fenêtre annule.

import { listen } from '@tauri-apps/api/event';
import { api } from '$lib/api';
import { choisir } from '$lib/dialogues/fenetres.svelte';

export interface QuestionRacine {
  mod_nom: string;
  dossier: string;
}

/** Le choix à rendre au cœur : `true` son contenu directement, `false` le dossier tel quel, `null` annuler. */
export function reponse(c: 'contenu' | 'dossier' | null): boolean | null {
  return c === 'contenu' ? true : c === 'dossier' ? false : null;
}

/** Pose la question chaque fois que le cœur la demande (fenêtre principale seulement). Rend de quoi arrêter d'écouter. */
export function ecouterQuestionsRacine(): Promise<() => void> {
  return listen<QuestionRacine>('mod-racine', async (e) => {
    const { mod_nom, dossier } = e.payload;
    const c = await choisir(
      `🧩 Où poser « ${mod_nom} » ?`,
      [
        { valeur: 'contenu' as const, libelle: '📂 Son contenu, directement dans le jeu', detail: `Sans le dossier « ${dossier} ».` },
        { valeur: 'dossier' as const, libelle: `📁 Le dossier « ${dossier} » tel quel`, detail: 'Comme il est dans l’archive.' },
      ],
      `Ce mod est rangé dans un seul dossier, « ${dossier} ». La page du mod dit en général lequel il faut ; dans le doute, lis-la d’abord (fermer cette fenêtre annule, rien n’est posé).`,
    );
    await api.modRacineRepondre(reponse(c));
  });
}
