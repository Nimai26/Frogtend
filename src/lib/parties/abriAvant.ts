// Lot S3 (Seb, 23/11) : l'abri des parties AVANT un retrait, une réinstallation, une mise à jour ou un mod. Pour un jeu
// PC, il se fait d'abord À PART (« 🛟 Tes parties d'abord… »), puis l'opération, avec sa propre progression ; le cœur
// ne le refait pas dans l'opération. S'il échoue, l'opération ne se fait PAS (erreur « abri_impossible ») : la fenêtre
// maison propose PARTOUT « 🔁 Réessayer », « ☁ Télécharger depuis OneDrive puis réessayer » (quand c'est la cause, la
// taille annoncée, la progression montrée), « Continuer sans abri » (seconde confirmation, focus sur Annuler) et
// « Annuler ». Rien sans le choix de la personne ; jamais une boîte native.
import { listen } from '@tauri-apps/api/event';
import { api, estErreurCoeur, type ErreurCoeur, type ProgresAbri } from '$lib/api';
import { choisir, confirmer, fermerToast, majProgres, toast, toastProgres } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { signalerAbri } from '$lib/parties/abriAuto.svelte';
import { messageAbriImpossible, messageSansAbri, optionsAbriImpossible, type ChoixAbriImpossible } from './parties';

/** L'erreur du cœur « l'abri d'avant n'a pas pu se faire ». */
export function estAbriImpossible(e: unknown): e is ErreurCoeur & { sorte: 'abri_impossible' } {
  return estErreurCoeur(e) && e.sorte === 'abri_impossible';
}

/** Les opérations dont l'abri peut échouer (`abri_parties::DECLENCHEURS`). */
export type OperationAbri = 'avant_retrait' | 'avant_reinstallation' | 'avant_mise_a_jour' | 'avant_mod';

/** Lance `faire` (une opération qui commence par mettre les parties à l'abri). `operation` : son abri ; `jeuPc` : un jeu
 * PC (« Windows ») — son abri est d'abord fait à part, avec le message `attente`. Si l'abri échoue, la personne
 * choisit ; l'opération est relancée seulement après son choix. Rend son résultat, ou `undefined` si elle a annulé.
 * Toute autre erreur remonte telle quelle. */
export async function avecAbriAvant<T>(
  id: number,
  operation: OperationAbri,
  faire: () => Promise<T>,
  o: { jeuPc?: boolean; attente?: string } = {},
): Promise<T | undefined> {
  /** Après « Continuer sans abri » : plus de « Tes parties d'abord… » (rien n'est copié). */
  let sansAbri = false;
  for (;;) {
    let e: ErreurCoeur;
    try {
      if (o.jeuPc) {
        const t = sansAbri ? null : toastProgres(o.attente ?? '🛟 Tes parties d’abord…');
        try {
          await api.partiesAbriAvant(id, operation);
        } finally {
          if (t !== null) fermerToast(t);
        }
        if (!sansAbri) signalerAbri(id);
      }
      return await faire();
    } catch (x) {
      if (!estAbriImpossible(x)) throw x;
      e = { ...x, operation: x.operation || operation };
    }
    // Le choix ; « Continuer sans abri » refusé à la seconde confirmation : la même fenêtre revient (rien n'est relancé).
    let c: ChoixAbriImpossible | null;
    for (;;) {
      c = await choisir<ChoixAbriImpossible>('🛟 Tes parties d’abord', optionsAbriImpossible(e), messageAbriImpossible(e));
      if (c !== 'sans_abri') break;
      const oui = await confirmer('⚠ Continuer sans abri ?', { message: messageSansAbri(e), danger: true, libelleValider: 'Continuer sans abri' });
      if (oui) break;
    }
    if (c === null) return undefined;
    if (c === 'reessayer') continue;
    if (c === 'sans_abri') {
      await api.partiesContinuerSansAbri(id, operation);
      sansAbri = true;
      continue;
    }
    // ☁ Télécharger depuis OneDrive, ou copier quand même (plus de 2 Go) : un abri à la main, rangé sous l'opération ;
    // complet, le cœur le retient une minute et l'opération qui suit ne le refait pas.
    const attente = toastProgres(c === 'onedrive' ? '☁ Téléchargement depuis OneDrive…' : '🛟 Copie de tes parties en cours…');
    const arreter =
      c === 'onedrive'
        ? await listen<ProgresAbri>('parties-abri-progres', (ev) => {
            const { jeu, faits, total } = ev.payload;
            if (jeu !== id) return;
            majProgres(attente, { etape: `Téléchargement depuis OneDrive… ${faits}/${total}`, chiffres: '', pourcent: total ? Math.round((100 * faits) / total) : 0 });
          }).catch(() => () => {})
        : () => {};
    try {
      await api.partiesAbriter(id, true, c === 'onedrive', operation);
      signalerAbri(id);
    } catch (x) {
      toast(`Échec de la mise à l’abri : ${motifDuRefus(x)}`, 'erreur');
    } finally {
      arreter();
      fermerToast(attente);
    }
  }
}
