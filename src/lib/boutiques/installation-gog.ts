// Lot C4b (revue des experts) : avant d'installer un jeu GOG par GOG Galaxy, Frogtend relit à l'instant le compte
// ouvert dans Galaxy. Si le jeu n'est que sur un AUTRE compte GOG du profil, il le DIT dans une fenêtre maison, sans
// interdire : « Ouvrir quand même dans GOG Galaxy » ou « Annuler ». Au moindre doute, rien n'est dit.
import { api } from '$lib/api';
import { confirmer } from '$lib/dialogues/fenetres.svelte';

/** Le titre et le bouton de l'avertissement (purs). */
export const TITRE_AVERTISSEMENT_GOG = 'Installer avec GOG Galaxy ?';
export const BOUTON_AVERTISSEMENT_GOG = 'Ouvrir quand même dans GOG Galaxy';

/** Rend vrai pour continuer : rien à dire (ou le cœur ne répond pas), ou la personne veut ouvrir Galaxy quand même. */
export async function confirmerInstallationGog(cle: string): Promise<boolean> {
  const m = await api.gogAvertissementInstallation(cle).catch(() => null);
  if (!m) return true;
  return confirmer(TITRE_AVERTISSEMENT_GOG, { message: m, libelleValider: BOUTON_AVERTISSEMENT_GOG });
}
