// Lot C4b (revue) : l'avertissement avant d'installer un jeu GOG d'un autre compte que celui de GOG Galaxy — jamais un
// refus : la personne peut ouvrir Galaxy quand même.
import { beforeEach, describe, expect, it, vi } from 'vitest';

let reponse: string | null | Error = null;
const confirmer = vi.fn(async (..._a: unknown[]) => true);
vi.mock('$lib/api', () => ({
  api: { gogAvertissementInstallation: async () => (reponse instanceof Error ? Promise.reject(reponse) : reponse) },
}));
vi.mock('$lib/dialogues/fenetres.svelte', () => ({ confirmer: (...a: unknown[]) => confirmer(...a) }));

const { BOUTON_AVERTISSEMENT_GOG, confirmerInstallationGog, TITRE_AVERTISSEMENT_GOG } = await import('./installation-gog');

describe('avertissement avant d’installer un jeu GOG', () => {
  beforeEach(() => {
    confirmer.mockClear();
    reponse = null;
  });
  it('rien à dire (ou le cœur ne répond pas) : on continue, sans fenêtre', async () => {
    expect(await confirmerInstallationGog('gog_9')).toBe(true);
    reponse = new Error('panne');
    expect(await confirmerInstallationGog('gog_9')).toBe(true);
    expect(confirmer).not.toHaveBeenCalled();
  });
  it('un autre compte : une fenêtre maison avec « Ouvrir quand même dans GOG Galaxy » ; annuler = rien', async () => {
    reponse = 'Ce jeu est sur ton autre compte GOG « Seb ».';
    expect(await confirmerInstallationGog('gog_9')).toBe(true);
    expect(confirmer).toHaveBeenCalledWith(TITRE_AVERTISSEMENT_GOG, { message: reponse, libelleValider: BOUTON_AVERTISSEMENT_GOG });
    confirmer.mockResolvedValueOnce(false);
    expect(await confirmerInstallationGog('gog_9')).toBe(false);
  });
});
