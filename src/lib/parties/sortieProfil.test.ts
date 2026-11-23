// Lot S3 : à la sortie d'un profil, aucun message de lui (un abri, le titre d'un jeu) ne reste à l'écran, et ce que les
// abris automatiques avaient dit pour lui est oublié.
import { describe, expect, it, vi } from 'vitest';

vi.mock('$lib/api', async (original) => ({
  ...(await original<typeof import('$lib/api')>()),
  api: new Proxy({ fermerProfil: async () => {} } as Record<string, unknown>, { get: (o, k) => o[k as string] ?? (async () => null) }),
}));

import { pile, toast } from '$lib/dialogues/fenetres.svelte';
import { sortirDuProfil } from '$lib/etat.svelte';
import { abrisAuto } from '$lib/parties/abriAuto.svelte';

describe('🛟 Sortie du profil', () => {
  it('les messages et les lignes des abris automatiques partent', async () => {
    toast('🛟 Tes parties de « Jeu secret » sont à l’abri.', 'ok');
    abrisAuto.derniers[900] = { jeu: 900, profil: 'p1', declencheur: 'apres_partie', etat: 'echec', fichiers: 0, octets: 0, en_ligne: 0, octets_en_ligne: 0, echecs: 0, motif: null };
    expect(pile.toasts.length).toBe(1);
    expect(await sortirDuProfil()).toBe(true);
    expect(pile.toasts).toEqual([]);
    expect(abrisAuto.derniers).toEqual({});
  });
});
