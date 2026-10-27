// L'avancement du téléchargement d'un document du jeu (livret, musique, vidéo) : montré sur son bouton, avec
// « Arrêter » à côté (ce qui est reçu est gardé : le clic suivant reprend où il en était).
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { api } from '$lib/api';

export interface ProgresDocument {
  jeu: number;
  doc: number;
  recus: number;
  total: number | null;
}

/** Le téléchargement de document en cours (un seul à la fois par écran). */
export const suiviDocument = $state<{ courant: ProgresDocument | null }>({ courant: null });

/** Lance `travail` en suivant l'avancement de ce document ; l'écoute s'arrête quoi qu'il arrive. */
export async function avecSuiviDocument<T>(jeu: number, doc: number, travail: () => Promise<T>): Promise<T> {
  const arreter = isTauri()
    ? await listen<ProgresDocument>('document-progres', (e) => {
        if (e.payload.jeu === jeu && e.payload.doc === doc) suiviDocument.courant = e.payload;
      }).catch(() => () => {})
    : () => {};
  try {
    return await travail();
  } finally {
    arreter();
    suiviDocument.courant = null;
  }
}

/** « Arrêter » : le téléchargement s'interrompt tout de suite (repris au prochain clic). */
export function arreterDocument(jeu: number, doc: number) {
  api.arreterDocument(jeu, doc).catch(() => {});
}
