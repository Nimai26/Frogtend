// La liste des jeux offerts : le compte Epic du LANCEUR ne compte pour le profil que s'il a accepté « les jeux de mes
// comptes » (lot B) — sinon un autre membre de la famille verrait « obtenu » pour un jeu qu'il n'a pas (Seb 19/11).
import { beforeEach, describe, expect, it, vi } from 'vitest';

const demandes: (boolean | undefined)[] = [];
const profil = { boutiquesPc: { possedes: '' as '' | 'oui' | 'non' } };
vi.mock('$lib/api', () => ({ api: { gratuitsListe: async (p?: boolean) => (demandes.push(p), []) } }));
vi.mock('$lib/etat.svelte', () => ({ etat: { profil }, reglerProfil: async () => {} }));
vi.mock('$lib/dialogues/fenetres.svelte', () => ({ toast: () => {} }));

const { lireGratuits } = await import('./gratuits.svelte');

beforeEach(() => (demandes.length = 0));

describe('lireGratuits', () => {
  it('demande la reconnaissance par le compte du lanceur seulement après le « oui » du lot B', async () => {
    for (const p of ['', 'non', 'oui'] as const) {
      profil.boutiquesPc.possedes = p;
      await lireGratuits();
    }
    expect(demandes).toEqual([false, false, true]);
  });
});
