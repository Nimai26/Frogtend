// La progression d'une installation de mod (lot 0.82.3) : un message non cliquable, avec une barre, l'étape, les
// octets, le débit et le temps restant (calculés par le cœur, événement `mod-progres`).
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { fermerToast, majProgres, toastProgres } from '$lib/dialogues/fenetres.svelte';
import { chiffresMod, ETAPE_MOD, pourcentMod, type ProgresMod } from './mods';

let prochain = 0;

/** Lance `travail(suivi)` en montrant sa progression ; le message et l'écoute s'arrêtent quoi qu'il arrive.
 * `approx` : le total annoncé est arrondi (Nexus). */
export async function avecProgres<T>(message: string, approx: boolean, travail: (suivi: number) => Promise<T>): Promise<T> {
  const suivi = ++prochain;
  let t = 0;
  // L'écoute d'abord (un échec la remplace par rien) : le message, qui ne se ferme pas au clic, n'est créé qu'ensuite.
  const arreter = isTauri()
    ? await listen<ProgresMod>('mod-progres', (e) => {
        if (e.payload.suivi !== suivi) return;
        majProgres(t, { etape: ETAPE_MOD[e.payload.etape] ?? '', chiffres: chiffresMod(e.payload, approx && e.payload.etape === 'telechargement'), pourcent: pourcentMod(e.payload) });
      }).catch(() => () => {})
    : () => {};
  t = toastProgres(message);
  try {
    return await travail(suivi);
  } finally {
    arreter();
    fermerToast(t);
  }
}
