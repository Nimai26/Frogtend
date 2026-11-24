// Lot S7 (Seb, 24/11) : ce que l'interface dit d'un tour de RATTRAPAGE (événement `parties-rattrapage`, forme RÉELLE de
// `BilanRattrapage` dans commandes.rs), et la question « C'est toi qui as joué ? ».
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { BilanRattrapage } from '$lib/api';
import type { ReponseCestToi } from '$lib/dialogues/fenetres.svelte';

const toast = vi.fn();
let reponses: ReponseCestToi[] = [];
const demanderSiCestToi = vi.fn(async (..._a: unknown[]) => reponses.shift() ?? 'plus_tard');
let choixGroupe: string | null = 'plus_tard';
const choisir = vi.fn(async (..._a: unknown[]) => choixGroupe);
vi.mock('$lib/dialogues/fenetres.svelte', () => ({
  toast: (...a: unknown[]) => toast(...a),
  toastProgres: vi.fn(),
  fermerToast: vi.fn(),
  demanderSiCestToi: (...a: unknown[]) => demanderSiCestToi(...a),
  choisir: (...a: unknown[]) => choisir(...a),
}));
const partiesConfirmer = vi.fn(async (..._a: unknown[]) => 1);
let aConfirmer: number[] = [];
const titresFiche: Record<number, string> = { 5: 'Hollow Knight' };
vi.mock('$lib/api', () => ({
  api: {
    partiesConfirmer: (...a: unknown[]) => partiesConfirmer(...a),
    partiesAConfirmer: async () => aConfirmer,
    partiesAbriAutoEtat: async () => null,
    fiche: async (id: number) => {
      if (!titresFiche[id]) throw new Error('inconnu');
      return { fiche: { id, titre: titresFiche[id] }, hors_ligne: false, locale: true };
    },
  },
}));
const { partie, tele } = vi.hoisted(() => ({
  partie: { enJeu: null as number | null },
  tele: { charge: true, jeux: { 1: { titre: 'Hades' }, 2: { titre: 'Celeste' }, 3: { titre: 'Tunic' }, 4: { titre: 'Cyberpunk 2077' } } as Record<number, { titre: string }> },
}));
vi.mock('$lib/ludotheque/jeu.svelte', () => ({ partie }));
vi.mock('$lib/ludotheque/telechargements.svelte', () => ({ tele }));

import { abrisAuto } from './abriAuto.svelte';
import { apresPartie, proposerConfirmations, recevoirRattrapage, remettreAZeroRattrapage, TEXTE_PLUS_TARD, texteToastRattrapage, titreDuJeu } from './rattrapage.svelte';

const bilan = (p: Partial<BilanRattrapage> = {}): BilanRattrapage => ({ profil: 'p1', rattrapes: [], a_confirmer: [], ...p });

beforeEach(() => {
  toast.mockClear();
  demanderSiCestToi.mockClear();
  choisir.mockClear();
  partiesConfirmer.mockClear();
  reponses = [];
  aConfirmer = [];
  choixGroupe = 'plus_tard';
  partie.enJeu = null;
  tele.charge = true;
  remettreAZeroRattrapage();
});

describe('🛟 Le toast du rattrapage', () => {
  it('« 🛟 N jeu(x) rattrapé(s) » au singulier et au pluriel ; rien si rien de neuf', () => {
    expect(texteToastRattrapage(0)).toBeNull();
    expect(texteToastRattrapage(1)).toBe('🛟 1 jeu rattrapé');
    expect(texteToastRattrapage(3)).toBe('🛟 3 jeux rattrapés');
  });

  it('un tour du profil ouvert : un toast court et la liste des abris relue ; rien de neuf : aucun message', async () => {
    const v = abrisAuto.version;
    await recevoirRattrapage(bilan({ rattrapes: [1, 2] }), 'p1');
    expect(toast).toHaveBeenCalledWith('🛟 2 jeux rattrapés', 'ok');
    expect(abrisAuto.version).toBeGreaterThan(v);
    toast.mockClear();
    await recevoirRattrapage(bilan(), 'p1');
    expect(toast).not.toHaveBeenCalled();
    expect(demanderSiCestToi).not.toHaveBeenCalled();
  });

  it('jamais montré à un autre profil que celui ouvert', async () => {
    aConfirmer = [1];
    await recevoirRattrapage(bilan({ rattrapes: [1], a_confirmer: [1] }), 'p2');
    await recevoirRattrapage(bilan({ rattrapes: [1], a_confirmer: [1] }), null);
    expect(toast).not.toHaveBeenCalled();
    expect(demanderSiCestToi).not.toHaveBeenCalled();
  });
});

describe('🛟 C’est toi qui as joué ?', () => {
  it('un par un jusqu’à 3 jeux : « Oui » et « Non » enregistrés pour CE profil, « Décider plus tard » arrête et le dit', async () => {
    reponses = ['oui', 'non', 'plus_tard'];
    expect(await proposerConfirmations([1, 2, 3], 'p1')).toBe(2);
    expect(demanderSiCestToi.mock.calls.map((c) => c[0])).toEqual([
      '🛟 C’est toi qui as joué à « Hades » ?',
      '🛟 C’est toi qui as joué à « Celeste » ?',
      '🛟 C’est toi qui as joué à « Tunic » ?',
    ]);
    expect(partiesConfirmer.mock.calls).toEqual([
      [1, true, 'p1'],
      [2, false, 'p1'],
    ]);
    expect(toast).toHaveBeenCalledWith(TEXTE_PLUS_TARD, 'neutre');
    expect(TEXTE_PLUS_TARD).toContain('Je te le redemanderai à la prochaine ouverture de Frogtend');
  });

  it('au-delà de 3 jeux : un choix groupé, comme la reprise du lot S5 (« Décider plus tard » en premier)', async () => {
    choixGroupe = 'oui';
    expect(await proposerConfirmations([1, 2, 3, 4, 5], 'p1')).toBe(5);
    expect(choisir).toHaveBeenCalledTimes(1);
    expect(choisir.mock.calls[0][0]).toBe('🛟 C’est toi qui as joué à ces 5 jeux ?');
    const options = choisir.mock.calls[0][1] as { valeur: string; libelle: string }[];
    expect(options[0]).toMatchObject({ valeur: 'plus_tard', libelle: '⏳ Décider plus tard' });
    expect(demanderSiCestToi).not.toHaveBeenCalled();
    expect(partiesConfirmer.mock.calls.every((c) => c[1] === true)).toBe(true);
    // « Décider plus tard » groupé (ou fenêtre fermée) : rien d'enregistré, et c'est dit.
    partiesConfirmer.mockClear();
    choixGroupe = null;
    expect(await proposerConfirmations([1, 2, 3, 4], 'p1')).toBe(0);
    expect(partiesConfirmer).not.toHaveBeenCalled();
    expect(toast).toHaveBeenCalledWith(TEXTE_PLUS_TARD, 'neutre');
    // « Un par un » : la question pour chaque jeu.
    choixGroupe = 'une';
    reponses = ['non', 'plus_tard'];
    expect(await proposerConfirmations([1, 2, 3, 4], 'p1')).toBe(1);
    expect(partiesConfirmer.mock.calls).toEqual([[1, false, 'p1']]);
  });

  it('les titres : jeux du PC chargés d’abord, sinon la fiche ; jamais « ce jeu » (sans titre : la question attend)', async () => {
    expect(await titreDuJeu(5)).toBe('Hollow Knight');
    expect(await titreDuJeu(77)).toBeNull();
    reponses = ['oui'];
    expect(await proposerConfirmations([77], 'p1')).toBe(0);
    expect(demanderSiCestToi).not.toHaveBeenCalled();
    // Les jeux du PC pas encore chargés : attendus avant la question.
    tele.charge = false;
    setTimeout(() => (tele.charge = true), 300);
    reponses = ['oui'];
    expect(await proposerConfirmations([1], 'p1')).toBe(1);
    expect(demanderSiCestToi.mock.calls.every((c) => !String(c[0]).includes('ce jeu'))).toBe(true);
  });

  it('une réponse impossible à enregistrer est dite, jamais tue', async () => {
    partiesConfirmer.mockRejectedValueOnce('Un autre profil est ouvert maintenant : rien n’a changé.');
    reponses = ['oui'];
    expect(await proposerConfirmations([1], 'p1')).toBe(0);
    expect(toast).toHaveBeenCalledWith(expect.stringContaining('Impossible d’enregistrer ta réponse pour « Hades »'), 'erreur');
  });

  it('les abris à confirmer sont RELUS chez le cœur (ceux d’avant compris), après le toast', async () => {
    aConfirmer = [2];
    reponses = ['oui'];
    await recevoirRattrapage(bilan({ rattrapes: [1], a_confirmer: [] }), 'p1');
    expect(toast.mock.invocationCallOrder[0]).toBeLessThan(demanderSiCestToi.mock.invocationCallOrder[0]);
    expect(partiesConfirmer).toHaveBeenCalledWith(2, true, 'p1');
  });

  it('JAMAIS pendant une partie : le toast passe, la question attend la fin de la partie', async () => {
    partie.enJeu = 900;
    aConfirmer = [1];
    reponses = ['oui'];
    await recevoirRattrapage(bilan({ rattrapes: [1], a_confirmer: [1] }), 'p1');
    expect(toast).toHaveBeenCalledWith('🛟 1 jeu rattrapé', 'ok');
    expect(demanderSiCestToi).not.toHaveBeenCalled();
    expect(await apresPartie('p1')).toBe(0);
    partie.enJeu = null;
    expect(await apresPartie('p1')).toBe(1);
    expect(partiesConfirmer).toHaveBeenCalledWith(1, true, 'p1');
    // Une seule fois ; et jamais pour un autre profil ouvert entre-temps.
    expect(await apresPartie('p1')).toBe(0);
    partie.enJeu = 900;
    await recevoirRattrapage(bilan({ a_confirmer: [1] }), 'p1');
    partie.enJeu = null;
    expect(await apresPartie('p2')).toBe(0);
  });
});
