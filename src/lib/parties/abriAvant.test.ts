// Lot S3 (Seb, 23/11) : l'abri des parties d'AVANT une opération — fait à part d'abord pour un jeu PC, puis la fenêtre
// maison quand il échoue : l'ordre des choix, la seconde confirmation (danger : le focus va sur Annuler), la
// progression du téléchargement OneDrive, et l'opération relancée seulement après le choix.
import { beforeEach, describe, expect, it, vi } from 'vitest';

const sansAbri = vi.fn(async (..._a: unknown[]) => {});
const abriter = vi.fn(async (..._a: unknown[]) => ({}));
let echecsAvant = 0;
const abriAvant = vi.fn(async (..._a: unknown[]) => {
  if (echecsAvant > 0) {
    echecsAvant--;
    throw structuredClone(IMPOSSIBLE);
  }
});
vi.mock('$lib/api', () => ({
  taille: (o: number) => `${o} o`,
  estErreurCoeur: (e: unknown) => typeof e === 'object' && e !== null && 'sorte' in e && 'motif' in e,
  api: {
    partiesContinuerSansAbri: (...a: unknown[]) => sansAbri(...a),
    partiesAbriter: (...a: unknown[]) => abriter(...a),
    partiesAbriAvant: (...a: unknown[]) => abriAvant(...a),
    partiesAbriAutoEtat: async () => null,
  },
}));
let choix: (string | null)[] = [];
const choisir = vi.fn(async (..._a: unknown[]) => choix.shift() ?? null);
let oui = true;
const confirmer = vi.fn(async (..._a: unknown[]) => oui);
const toast = vi.fn();
const toastProgres = vi.fn((..._a: unknown[]) => 7);
const fermerToast = vi.fn();
const majProgres = vi.fn();
vi.mock('$lib/dialogues/fenetres.svelte', () => ({
  choisir: (...a: unknown[]) => choisir(...a),
  confirmer: (...a: unknown[]) => confirmer(...a),
  toast: (...a: unknown[]) => toast(...a),
  toastProgres: (...a: unknown[]) => toastProgres(...a),
  fermerToast: (...a: unknown[]) => fermerToast(...a),
  majProgres: (...a: unknown[]) => majProgres(...a),
}));
let surProgres: ((e: { payload: { jeu: number; faits: number; total: number } }) => void) | null = null;
const ecoute = vi.fn(async (_n: string, f: (e: { payload: { jeu: number; faits: number; total: number } }) => void) => {
  surProgres = f;
  return () => {};
});
vi.mock('@tauri-apps/api/event', () => ({ listen: (n: string, f: (e: { payload: { jeu: number; faits: number; total: number } }) => void) => ecoute(n, f) }));

import { avecAbriAvant } from './abriAvant';
import { abrisAuto, remettreAZeroAbrisAuto } from '$lib/parties/abriAuto.svelte';

const IMPOSSIBLE = { sorte: 'abri_impossible', motif: '9 fichiers de tes parties sont seulement dans ton nuage OneDrive (180 Mo).', cause: 'onedrive', operation: 'avant_retrait', fichiers_en_ligne: 9, octets_en_ligne: 180, octets: 0 };

/** Une opération qui échoue (abri impossible, dans l'opération elle-même) `n` fois, puis réussit. */
function operation(n: number) {
  let essais = 0;
  return vi.fn(async () => {
    essais++;
    if (essais <= n) throw structuredClone(IMPOSSIBLE);
    return 'fait';
  });
}

beforeEach(() => {
  choix = [];
  oui = true;
  echecsAvant = 0;
  surProgres = null;
  remettreAZeroAbrisAuto();
  for (const f of [sansAbri, abriter, abriAvant, choisir, confirmer, toast, toastProgres, fermerToast, majProgres, ecoute]) f.mockClear();
});

describe('🛟 Tes parties d’abord', () => {
  it('un jeu PC : l’abri à part d’abord (« Tes parties d’abord… »), puis l’opération ; la fiche relit ses abris', async () => {
    const f = operation(0);
    const v = abrisAuto.version;
    expect(await avecAbriAvant(900, 'avant_retrait', f, { jeuPc: true })).toBe('fait');
    expect(abriAvant).toHaveBeenCalledWith(900, 'avant_retrait');
    expect(toastProgres).toHaveBeenCalledWith('🛟 Tes parties d’abord…');
    expect(fermerToast).toHaveBeenCalledWith(7);
    expect(abriAvant.mock.invocationCallOrder[0]).toBeLessThan(f.mock.invocationCallOrder[0]);
    expect(abrisAuto.version).toBe(v + 1);
    expect(choisir).not.toHaveBeenCalled();
  });

  it('pas un jeu PC : ni abri à part, ni message', async () => {
    const f = operation(0);
    expect(await avecAbriAvant(900, 'avant_retrait', f)).toBe('fait');
    expect(abriAvant).not.toHaveBeenCalled();
    expect(toastProgres).not.toHaveBeenCalled();
  });

  it('l’abri à part échoue : rien n’est lancé ; la cause d’abord (focus), puis Réessayer, puis Continuer sans abri', async () => {
    const f = operation(0);
    echecsAvant = 1;
    choix = ['reessayer'];
    expect(await avecAbriAvant(900, 'avant_retrait', f, { jeuPc: true })).toBe('fait');
    expect(f).toHaveBeenCalledTimes(1);
    const [titre, options, message] = choisir.mock.calls[0] as [string, { valeur: string; libelle: string; detail?: string }[], string];
    expect(titre).toBe('🛟 Tes parties d’abord');
    expect(options.map((o) => o.libelle)).toEqual(['☁ Télécharger depuis OneDrive puis réessayer', '🔁 Réessayer', 'Continuer sans abri']);
    expect(options[1].detail).toBeUndefined();
    expect(message).toContain('Rien n’a été fait : le retrait du jeu attend');
  });

  it('« Annuler » : rien n’est fait, rien n’est relancé', async () => {
    const f = operation(1);
    choix = [null];
    expect(await avecAbriAvant(900, 'avant_retrait', f)).toBeUndefined();
    expect(f).toHaveBeenCalledTimes(1);
    expect(sansAbri).not.toHaveBeenCalled();
  });

  it('« Continuer sans abri » : une seconde confirmation (danger), puis l’opération sans « Tes parties d’abord… »', async () => {
    const f = operation(0);
    echecsAvant = 1;
    choix = ['sans_abri'];
    expect(await avecAbriAvant(900, 'avant_retrait', f, { jeuPc: true })).toBe('fait');
    const [titre, o] = confirmer.mock.calls[0] as [string, { danger: boolean; libelleValider: string; message: string }];
    expect(titre).toBe('⚠ Continuer sans abri ?');
    expect(o.danger).toBe(true);
    expect(o.libelleValider).toBe('Continuer sans abri');
    expect(sansAbri).toHaveBeenCalledWith(900, 'avant_retrait');
    expect(toastProgres).toHaveBeenCalledTimes(1);
    expect(f).toHaveBeenCalledTimes(1);
  });

  it('la seconde confirmation refusée : la fenêtre revient, rien n’est accordé', async () => {
    const f = operation(1);
    choix = ['sans_abri', null];
    oui = false;
    expect(await avecAbriAvant(900, 'avant_retrait', f)).toBeUndefined();
    expect(sansAbri).not.toHaveBeenCalled();
    expect(choisir).toHaveBeenCalledTimes(2);
    expect(f).toHaveBeenCalledTimes(1);
  });

  it('« Télécharger depuis OneDrive » : la progression filtrée sur CE jeu, un abri rangé sous l’opération, puis l’opération', async () => {
    const f = operation(1);
    choix = ['onedrive'];
    abriter.mockImplementationOnce(async () => {
      surProgres?.({ payload: { jeu: 900, faits: 3, total: 9 } });
      surProgres?.({ payload: { jeu: 5, faits: 1, total: 1 } });
      return {};
    });
    expect(await avecAbriAvant(900, 'avant_retrait', f)).toBe('fait');
    expect(ecoute.mock.calls[0][0]).toBe('parties-abri-progres');
    expect(majProgres).toHaveBeenCalledTimes(1);
    expect(majProgres).toHaveBeenCalledWith(7, { etape: 'Téléchargement depuis OneDrive… 3/9', chiffres: '', pourcent: 33 });
    expect(abriter).toHaveBeenCalledWith(900, true, true, 'avant_retrait');
    expect(f).toHaveBeenCalledTimes(2);
  });

  it('une autre erreur remonte telle quelle', async () => {
    const f = vi.fn(async () => {
      throw { sorte: 'refus', motif: 'Installe d’abord le jeu.' };
    });
    await expect(avecAbriAvant(900, 'avant_mod', f)).rejects.toEqual({ sorte: 'refus', motif: 'Installe d’abord le jeu.' });
    expect(choisir).not.toHaveBeenCalled();
  });
});
