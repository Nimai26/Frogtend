// La sauvegarde du profil ouvert chez Firehouse : à la demande, après chaque partie, ou chaque jour (réglage du
// profil). Une sauvegarde automatique est discrète : elle ne dit rien si tout va bien, et signale un échec sans
// bloquer (elle sera refaite à la prochaine occasion).

import { api } from '$lib/api';
import { toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { etat } from '$lib/etat.svelte';

export interface BilanSauvegarde {
  envoyes: number;
  inchanges: number;
  retires: number;
  octets: number;
  fichiers: number;
  taille: number;
  date: string;
  pc: string;
  dossier: string;
}

/** Une sauvegarde quotidienne est refaite au-delà de ce délai. */
const JOUR_S = 24 * 3600;

export const sauvegarde = $state<{ enCours: boolean; derniere: BilanSauvegarde | null }>({
  enCours: false,
  derniere: null,
});

export async function lireDerniereSauvegarde() {
  sauvegarde.derniere = await api.derniereSauvegarde().catch(() => null);
}

/** Sauvegarde maintenant. `discrete` : pas de message si tout va bien. */
export async function sauvegarder(discrete = false): Promise<boolean> {
  if (sauvegarde.enCours || etat.pc.firehouse.simule) return false;
  sauvegarde.enCours = true;
  try {
    const b = await api.sauvegarder();
    sauvegarde.derniere = b;
    if (!discrete) {
      toast(
        b.envoyes
          ? `✅ Sauvegardé : ${b.envoyes} fichier(s) envoyé(s), ${b.inchanges} déjà à jour (${b.fichiers} en tout).`
          : `✅ Sauvegarde à jour : ${b.fichiers} fichier(s) de parties, rien n’avait changé.`,
      );
    }
    return true;
  } catch (e) {
    const motif = motifDuRefus(e);
    // Firehouse sans la sauvegarde (avant le contrat 1.5) : une sauvegarde automatique se tait.
    if (discrete && motif.includes('ne propose pas encore')) return false;
    toast(`${discrete ? '⚠ Sauvegarde automatique non faite' : 'Échec de la sauvegarde'} : ${motif}`, discrete ? 'alerte' : 'erreur');
    return false;
  } finally {
    sauvegarde.enCours = false;
  }
}

/** Après une partie : sauvegarde si le profil l'a choisi. */
export async function apresUnePartie() {
  if (etat.profil.sauvegarde.auto === 'apres_partie') await sauvegarder(true);
}

/** À l'ouverture du profil : sauvegarde quotidienne si le profil l'a choisi et que la dernière est ancienne. */
export async function auDemarrage() {
  await lireDerniereSauvegarde();
  if (etat.profil.sauvegarde.auto === 'manuelle') return;
  const age = sauvegarde.derniere ? Date.now() / 1000 - Number(sauvegarde.derniere.date) : Infinity;
  if (age > JOUR_S) await sauvegarder(true);
}
