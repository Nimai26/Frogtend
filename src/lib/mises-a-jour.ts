// Les mises à jour de Frogtend : vérifiées au démarrage, installées SEULEMENT après accord.

import { isTauri } from '@tauri-apps/api/core';
import { confirmer, toast } from './dialogues/fenetres.svelte';
import { motifDuRefus } from './dialogues/messages';

export type ResultatVerification = 'a-jour' | 'installee' | 'refusee' | 'indisponible' | 'erreur';

/**
 * Cherche une nouvelle version. S'il y en a une, la propose ; ne télécharge et n'installe qu'après un oui.
 * `silencieux` : au démarrage, on ne dit rien quand tout est à jour ou que la vérification échoue.
 */
export async function verifierMiseAJour(silencieux = false): Promise<ResultatVerification> {
  if (!isTauri()) return 'indisponible';
  try {
    const { check } = await import('@tauri-apps/plugin-updater');
    const maj = await check();
    if (!maj) {
      if (!silencieux) toast('✅ Frogtend est à jour.');
      return 'a-jour';
    }

    const oui = await confirmer(`⬆ Frogtend ${maj.version} est disponible`, {
      message:
        `Tu as la version ${maj.currentVersion}.` +
        (maj.body ? `\n\n${maj.body}` : '') +
        '\n\nFrogtend va télécharger la mise à jour, l’installer, puis redémarrer.',
      libelleValider: '⬇ Mettre à jour',
    });
    if (!oui) return 'refusee';

    let recu = 0;
    let total = 0;
    await maj.downloadAndInstall((e) => {
      if (e.event === 'Started') total = e.data.contentLength ?? 0;
      else if (e.event === 'Progress') recu += e.data.chunkLength;
    });
    // Vérifier le résultat, pas le retour : on annonce les octets reçus.
    toast(`✅ Mise à jour installée (${(recu / 1e6).toFixed(1)} Mo${total ? ` sur ${(total / 1e6).toFixed(1)}` : ''}). Redémarrage…`);
    const { relaunch } = await import('@tauri-apps/plugin-process');
    await relaunch();
    return 'installee';
  } catch (e) {
    if (!silencieux) toast(`Impossible de vérifier les mises à jour : ${motifDuRefus(e)}`, 'erreur');
    return 'erreur';
  }
}
