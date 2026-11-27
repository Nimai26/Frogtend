// La progression d'une installation de mod (lot 0.82.3) : un message non cliquable, avec une barre, l'étape, les
// octets, le débit et le temps restant (calculés par le cœur, événement `mod-progres`).
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { fermerToast, majProgres, toastProgres } from '$lib/dialogues/fenetres.svelte';
import { chiffresMod, ETAPE_MOD, pourcentMod, type ProgresMod } from './mods';

let prochain = 0;

/** Ce qu'on sait AVANT le premier événement du cœur (étape et total connus de l'écran). */
export type DepartProgres = Pick<ProgresMod, 'etape' | 'total'> & { faits?: number };

/** L'état montré pour un événement (ou pour le départ). */
export function etatProgres(p: Omit<ProgresMod, 'suivi'>, approx: boolean) {
  return { etape: ETAPE_MOD[p.etape] ?? '', chiffres: chiffresMod({ suivi: 0, ...p }, approx && p.etape === 'telechargement'), pourcent: pourcentMod({ suivi: 0, ...p }) };
}

/** Lance `travail(suivi)` en montrant sa progression ; le message et l'écoute s'arrêtent quoi qu'il arrive.
 * `approx` : le total annoncé est arrondi (Nexus). `depart` (0.227.1, défaut vu par Seb le 25/11) : l'étape et le total
 * connus d'avance, montrés tout de suite (« 0 fichier sur 2 014 ») — le cœur ne parle qu'une fois le jeu vérifié et
 * les parties à l'abri, jamais une barre muette d'ici là. */
export async function avecProgres<T>(
  message: string,
  approx: boolean,
  travail: (suivi: number) => Promise<T>,
  depart?: DepartProgres,
  surProgres?: (p: ProgresMod) => void,
): Promise<T> {
  const suivi = ++prochain;
  let t = 0;
  // L'écoute d'abord (un échec la remplace par rien) : le message, qui ne se ferme pas au clic, n'est créé qu'ensuite.
  // `surProgres` (lot C4c-2) : l'appelant suit aussi les étapes (l'installation GOG n'est plus annulable une fois son
  // installeur lancé).
  const arreter = isTauri()
    ? await listen<ProgresMod>('mod-progres', (e) => {
        if (e.payload.suivi !== suivi) return;
        majProgres(t, etatProgres(e.payload, approx));
        surProgres?.(e.payload);
      }).catch(() => () => {})
    : () => {};
  t = toastProgres(message);
  if (depart && depart.total > 0) majProgres(t, etatProgres({ etape: depart.etape, total: depart.total, faits: Math.min(depart.faits ?? 0, depart.total), debit: 0 }, approx));
  try {
    return await travail(suivi);
  } finally {
    arreter();
    fermerToast(t);
  }
}
