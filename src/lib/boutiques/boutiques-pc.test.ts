// Les jeux POSSÉDÉS des comptes ouverts sur ce PC (lot B, ✅ Seb 18/11) : ils n'entrent dans un profil qu'après SON
// accord, demandé une fois ; jamais pendant une partie ; « Non » retire ceux déjà rangés et ne redemande plus.
import { beforeEach, describe, expect, it, vi } from 'vitest';

const profil = { boutiquesPc: { auto: true, possedes: '' as '' | 'oui' | 'non' } };
const etatFaux = { profil, profilOuvert: { id: 'seb' } as { id: string } | null };
const partieFausse = { enJeu: null as number | null };
const relire = vi.fn(async (_possedes?: boolean) => ({
  boutiques: [
    { boutique: 'steam', nom: 'Steam', lanceur: true, lu: true, installes: 2, possedes: null },
    { boutique: 'epic', nom: 'Epic Games', lanceur: true, lu: true, installes: 1, possedes: 500 },
  ],
  nouveaux: [] as [string, number, number][],
}));
const retirer = vi.fn(async () => 3);
const choisir = vi.fn(async (..._a: unknown[]): Promise<unknown> => 'oui');
const toast = vi.fn();

vi.mock('$lib/api', () => ({ api: { boutiquesRelirePc: (p?: boolean) => relire(p), boutiquesRetirerPossedes: () => retirer(), steamJeux: async () => [] } }));
vi.mock('$lib/dialogues/fenetres.svelte', () => ({ choisir: (...a: unknown[]) => choisir(...a), toast: (...a: unknown[]) => toast(...a) }));
vi.mock('$lib/dialogues/messages', () => ({ motifDuRefus: () => 'refus' }));
vi.mock('$lib/etat.svelte', () => ({
  etat: etatFaux,
  reglerProfil: async (chemin: string, valeur: unknown) => {
    if (chemin === 'boutiquesPc.possedes') profil.boutiquesPc.possedes = valeur as 'oui' | 'non';
  },
}));
vi.mock('$lib/ludotheque/jeu.svelte', () => ({ partie: partieFausse }));
vi.mock('$lib/ludotheque/ludotheque.svelte', () => ({ rechargerListe: async () => {}, rechargerPlateformes: async () => {} }));
vi.mock('$lib/ludotheque/telechargements.svelte', () => ({ suivreTelechargements: () => {} }));

const { boutiquesAuDemarrage } = await import('./boutiques-pc.svelte');

describe('les jeux possédés des comptes', () => {
  beforeEach(() => {
    profil.boutiquesPc.possedes = '';
    partieFausse.enJeu = null;
    for (const f of [relire, retirer, choisir, toast]) f.mockClear();
  });

  it('ne sont rangés qu’après le oui du profil, demandé une seule fois', async () => {
    choisir.mockResolvedValueOnce('oui');
    await boutiquesAuDemarrage();
    expect(relire.mock.calls[0][0]).toBe(false);
    expect(choisir).toHaveBeenCalledTimes(1);
    expect(String(choisir.mock.calls[0][0])).toContain('Ajouter les 500 jeux du compte Epic Games connecté sur ce PC');
    expect(profil.boutiquesPc.possedes).toBe('oui');
    expect(relire.mock.calls[1][0]).toBe(true);
    // La fois suivante : rangés d'office, sans question.
    await boutiquesAuDemarrage();
    expect(relire.mock.calls[2][0]).toBe(true);
    expect(choisir).toHaveBeenCalledTimes(1);
  });

  it('« Non » : plus jamais demandé, et les cartes déjà rangées sont retirées (et c’est dit)', async () => {
    choisir.mockResolvedValueOnce('non');
    await boutiquesAuDemarrage();
    expect(profil.boutiquesPc.possedes).toBe('non');
    expect(retirer).toHaveBeenCalledTimes(1);
    expect(toast).toHaveBeenCalledWith(expect.stringContaining('3 jeux de tes comptes retirés'));
    await boutiquesAuDemarrage();
    expect(relire.mock.calls.every((c) => c[0] === false)).toBe(true);
    expect(choisir).toHaveBeenCalledTimes(1);
  });

  it('Échap : rien n’est décidé, la question revient à la prochaine ouverture', async () => {
    choisir.mockResolvedValueOnce(null);
    await boutiquesAuDemarrage();
    expect(profil.boutiquesPc.possedes).toBe('');
    expect(retirer).not.toHaveBeenCalled();
  });

  it('jamais pendant une partie : la question attend', async () => {
    partieFausse.enJeu = 110;
    await boutiquesAuDemarrage();
    expect(choisir).not.toHaveBeenCalled();
    expect(profil.boutiquesPc.possedes).toBe('');
  });
});
