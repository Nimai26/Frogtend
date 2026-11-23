// Lot S3 (Seb, 23/11) : ce que l'interface dit d'un abri AUTOMATIQUE (événement `parties-abri-auto`, forme RÉELLE de
// `BilanAbriAuto` dans commandes.rs).
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { BilanAbriAuto } from '$lib/api';

const toast = vi.fn();
const toastProgres = vi.fn((..._a: unknown[]) => 5);
const fermerToast = vi.fn();
vi.mock('$lib/dialogues/fenetres.svelte', () => ({
  toast: (...a: unknown[]) => toast(...a),
  toastProgres: (...a: unknown[]) => toastProgres(...a),
  fermerToast: (...a: unknown[]) => fermerToast(...a),
}));

let etatGarde: BilanAbriAuto | null = null;
vi.mock('$lib/api', () => ({ api: { partiesAbriAutoEtat: async () => etatGarde } }));

import { abrisAuto, ATTENTE_MAX_MS, chargerLigneGardee, DELAI_ATTENTE_MS, oublierAbrisAuto, recevoirAbriAuto, remettreAZeroAbrisAuto, signalerAbri } from '$lib/parties/abriAuto.svelte';

const bilan = (p: Partial<BilanAbriAuto> = {}): BilanAbriAuto => ({
  jeu: 900,
  profil: 'p1',
  declencheur: 'apres_partie',
  etat: 'fait',
  fichiers: 3,
  octets: 7,
  en_ligne: 0,
  octets_en_ligne: 0,
  echecs: 0,
  motif: null,
  ...p,
});
const titre = () => 'Hades';

beforeEach(() => {
  remettreAZeroAbrisAuto();
  toast.mockClear();
  toastProgres.mockClear();
  fermerToast.mockClear();
});
afterEach(() => vi.useRealTimers());

describe('🛟 Abri automatique : ce que l’interface dit', () => {
  it('réussi : un toast court « ok » ; rien de neuf : rien du tout', () => {
    recevoirAbriAuto(bilan(), 'p1', titre);
    expect(toast).toHaveBeenCalledWith('🛟 Tes parties de « Hades » sont à l’abri.', 'ok');
    toast.mockClear();
    recevoirAbriAuto(bilan({ etat: 'rien_de_neuf' }), 'p1', titre);
    recevoirAbriAuto(bilan({ etat: 'aucun' }), 'p1', titre);
    expect(toast).not.toHaveBeenCalled();
  });

  it('un autre profil que celui ouvert : ignoré (jamais montré à quelqu’un d’autre)', () => {
    const v = abrisAuto.version;
    recevoirAbriAuto(bilan({ profil: 'p2', etat: 'echec' }), 'p1', titre);
    recevoirAbriAuto(bilan(), null, titre);
    expect(toast).not.toHaveBeenCalled();
    expect(abrisAuto.version).toBe(v);
    expect(abrisAuto.derniers[900]).toBeUndefined();
  });

  it('pendant la copie : un message de travail seulement au-delà de 2 s, fermé à la fin', () => {
    vi.useFakeTimers();
    recevoirAbriAuto(bilan({ etat: 'en_cours' }), 'p1', titre);
    vi.advanceTimersByTime(DELAI_ATTENTE_MS - 100);
    recevoirAbriAuto(bilan(), 'p1', titre);
    expect(toastProgres).not.toHaveBeenCalled();
    recevoirAbriAuto(bilan({ etat: 'en_cours' }), 'p1', titre);
    vi.advanceTimersByTime(DELAI_ATTENTE_MS + 100);
    expect(toastProgres).toHaveBeenCalledWith('🛟 Copie des parties de « Hades »…');
    recevoirAbriAuto(bilan(), 'p1', titre);
    expect(fermerToast).toHaveBeenCalledWith(5);
  });

  it('un problème : un toast d’alerte et une ligne qui reste ; « dans le nuage » une fois par jeu et par profil', () => {
    recevoirAbriAuto(bilan({ etat: 'echec', motif: 'raison' }), 'p1', titre);
    expect(toast).toHaveBeenLastCalledWith(expect.stringContaining('« Hades »'), 'alerte');
    expect(abrisAuto.derniers[900].etat).toBe('echec');
    recevoirAbriAuto(bilan({ etat: 'en_ligne', en_ligne: 2 }), 'p1', titre);
    recevoirAbriAuto(bilan({ etat: 'en_ligne', en_ligne: 2 }), 'p1', titre);
    recevoirAbriAuto(bilan({ etat: 'en_ligne', en_ligne: 2, jeu: 12 }), 'p1', titre);
    expect(toast.mock.calls.filter((c) => String(c[0]).startsWith('☁')).length).toBe(2);
    expect(abrisAuto.derniers[12].etat).toBe('en_ligne');
    // Un nouveau profil : ce qui avait été reçu pour l'autre part.
    oublierAbrisAuto();
    expect(abrisAuto.derniers).toEqual({});
  });

  it('attente dépassée, emplacement inconnu : sans bruit, la ligne seule ; rien de neuf efface la ligne', () => {
    recevoirAbriAuto(bilan({ etat: 'attente_depassee' }), 'p1', titre);
    recevoirAbriAuto(bilan({ etat: 'inconnu', jeu: 12 }), 'p1', titre);
    expect(toast).not.toHaveBeenCalled();
    expect(abrisAuto.derniers[900].etat).toBe('attente_depassee');
    expect(abrisAuto.derniers[12].etat).toBe('inconnu');
    const v = abrisAuto.version;
    recevoirAbriAuto(bilan({ etat: 'rien_de_neuf' }), 'p1', titre);
    expect(abrisAuto.derniers[900]).toBeUndefined();
    expect(abrisAuto.version).toBe(v + 1);
    expect(toast).not.toHaveBeenCalled();
    signalerAbri(12);
    expect(abrisAuto.derniers[12]).toBeUndefined();
  });

  it('le message de travail ne reste jamais plus de 15 minutes', () => {
    vi.useFakeTimers();
    recevoirAbriAuto(bilan({ etat: 'en_cours' }), 'p1', titre);
    vi.advanceTimersByTime(DELAI_ATTENTE_MS + 10);
    expect(toastProgres).toHaveBeenCalledTimes(1);
    vi.advanceTimersByTime(ATTENTE_MAX_MS);
    expect(fermerToast).toHaveBeenCalledWith(5);
  });

  it('la ligne gardée par le cœur est relue (redémarrage), sans écraser une plus récente', async () => {
    etatGarde = bilan({ etat: 'trop_gros' });
    await chargerLigneGardee(900);
    expect(abrisAuto.derniers[900].etat).toBe('trop_gros');
    etatGarde = bilan({ etat: 'echec' });
    await chargerLigneGardee(900);
    expect(abrisAuto.derniers[900].etat).toBe('trop_gros');
  });

  it('un abri d’avant une opération : la liste est relue, mais pas de toast (la fenêtre de l’opération parle)', () => {
    const v = abrisAuto.version;
    recevoirAbriAuto(bilan({ declencheur: 'avant_retrait' }), 'p1', titre);
    expect(toast).not.toHaveBeenCalled();
    expect(abrisAuto.version).toBe(v + 1);
  });
});
