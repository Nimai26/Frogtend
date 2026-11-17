// « ▶ Ouvrir l'émulateur seul » avec un faux PC : refus au canapé, un émulateur ouvert directement, le choix avec ⭐,
// le refus poli, l'état relu après un rechargement, et la sauvegarde automatique à la fermeture.
import { beforeEach, describe, expect, it, vi } from 'vitest';

const ouvrir = vi.fn(async (_p: string, cle: string) => (cle === '2' ? 'DuckStation' : 'Eden'));
const etatCoeur = vi.fn(async (): Promise<string | null> => null);
vi.mock('$lib/api', () => ({ api: { emulateurOuvrirSeul: (p: string, c: string) => ouvrir(p, c), emulateurSeulEtat: () => etatCoeur() } }));

const choisir = vi.fn(async (..._a: unknown[]): Promise<string | null> => '2');
const toast = vi.fn((..._a: unknown[]) => {});
vi.mock('$lib/dialogues/fenetres.svelte', () => ({ choisir: (...a: unknown[]) => choisir(...a), toast: (...a: unknown[]) => toast(...a) }));
vi.mock('$lib/dialogues/messages', () => ({ motifDuRefus: (e: unknown) => String((e as { motif?: string }).motif ?? e) }));

const etat = vi.hoisted(() => ({ pc: { emulateurs: {} as Record<string, unknown>, emulateursJeux: {} as Record<string, string> } }));
vi.mock('$lib/etat.svelte', () => ({ etat }));

const apresUnePartie = vi.fn(async () => {});
vi.mock('$lib/sauvegarde.svelte', () => ({ apresUnePartie: () => apresUnePartie() }));

const auCanape = vi.hoisted(() => ({ oui: false }));
const seulementAuBureau = vi.fn(async (_q: string) => auCanape.oui);
vi.mock('$lib/taodbox/canape', () => ({ seulementAuBureau: (q: string) => seulementAuBureau(q) }));

import { emulateurSeul, ouvrirSeul, PAS_ENCORE, relireEtat, surEvenement } from './seul.svelte';

const EDEN = { cle: '1', nom: 'Eden', programme: 'E:\\Eden\\eden.exe', ligne: '-f -g "%rom%"' };
const RYU = { cle: '3', nom: 'Ryubing', programme: 'E:\\Ryubing\\Ryujinx.exe', ligne: '-f "%rom%"' };
const RA = { cle: '4', nom: 'RetroArch', programme: 'E:\\RA\\retroarch.exe', ligne: '-L x' };

beforeEach(() => {
  vi.clearAllMocks();
  auCanape.oui = false;
  emulateurSeul.ouvert = null;
  emulateurSeul.ouverture = null;
  etat.pc.emulateurs = {};
  etat.pc.emulateursJeux = {};
});

describe('ouvrir un émulateur seul', () => {
  it('est refusé au canapé (Taodbox), sans rien ouvrir', async () => {
    auCanape.oui = true;
    etat.pc.emulateurs = { 'Nintendo Switch': { liste: [EDEN], defaut: '1' } };
    expect(await ouvrirSeul(7, 'Nintendo Switch')).toBe(false);
    expect(seulementAuBureau).toHaveBeenCalledWith('Ouvrir un émulateur seul');
    expect(ouvrir).not.toHaveBeenCalled();
  });

  it('ouvre directement le seul émulateur ouvrable (et ignore ceux qui ne s’ouvrent pas seuls)', async () => {
    etat.pc.emulateurs = { 'Nintendo Switch': { liste: [RA, EDEN], defaut: '4' } };
    expect(await ouvrirSeul(7, 'Nintendo Switch')).toBe(true);
    expect(choisir).not.toHaveBeenCalled();
    expect(ouvrir).toHaveBeenCalledWith('Nintendo Switch', '1');
    expect(toast).toHaveBeenCalledWith('▶ Eden s’ouvre…');
    expect(emulateurSeul.ouverture).toBeNull();
  });

  it('propose le choix avec ⭐ sur celui du jeu quand il y en a plusieurs', async () => {
    etat.pc.emulateurs = { 'Nintendo Switch': { liste: [EDEN, RYU], defaut: '1' } };
    etat.pc.emulateursJeux = { '7': '3' };
    choisir.mockResolvedValueOnce('3');
    expect(await ouvrirSeul(7, 'Nintendo Switch')).toBe(true);
    const options = choisir.mock.calls[0][1] as { valeur: string; libelle: string; detail?: string }[];
    expect(options).toEqual([
      { valeur: '1', libelle: 'Eden', detail: undefined },
      { valeur: '3', libelle: '⭐ Ryubing', detail: 'celui d’habitude pour ce jeu' },
    ]);
    expect(ouvrir).toHaveBeenCalledWith('Nintendo Switch', '3');
    // Renoncer : rien ne s'ouvre.
    ouvrir.mockClear();
    choisir.mockResolvedValueOnce(null);
    expect(await ouvrirSeul(7, 'Nintendo Switch')).toBe(false);
    expect(ouvrir).not.toHaveBeenCalled();
  });

  it('dit poliment qu’un émulateur ne s’ouvre pas encore seul, et dit le refus du cœur', async () => {
    etat.pc.emulateurs = { 'Super Nintendo': { liste: [RA], defaut: '4' } };
    expect(await ouvrirSeul(5, 'Super Nintendo')).toBe(false);
    expect(toast).toHaveBeenCalledWith(PAS_ENCORE, 'alerte');
    expect(ouvrir).not.toHaveBeenCalled();
    etat.pc.emulateurs = { 'Nintendo Switch': { liste: [EDEN], defaut: '1' } };
    ouvrir.mockRejectedValueOnce({ sorte: 'refus', motif: 'Eden est déjà ouvert : ferme-le d’abord, puis réessaie.' });
    expect(await ouvrirSeul(7, 'Nintendo Switch')).toBe(false);
    expect(toast).toHaveBeenLastCalledWith('Eden est déjà ouvert : ferme-le d’abord, puis réessaie.', 'erreur');
  });

  it('bloque une 2e ouverture pendant qu’il est ouvert, et sauvegarde les parties à sa fermeture', async () => {
    etat.pc.emulateurs = { 'Nintendo Switch': { liste: [EDEN], defaut: '1' } };
    await surEvenement({ sorte: 'debut', nom: 'Eden' });
    expect(emulateurSeul.ouvert).toBe('Eden');
    expect(toast, 'pas d’annonce de session pour un émulateur seul').not.toHaveBeenCalled();
    expect(await ouvrirSeul(7, 'Nintendo Switch')).toBe(false);
    expect(ouvrir).not.toHaveBeenCalled();
    await surEvenement({ sorte: 'fin', nom: 'Eden' });
    expect(emulateurSeul.ouvert).toBeNull();
    expect(apresUnePartie).toHaveBeenCalledTimes(1);
    expect(String(toast.mock.calls.at(-1)?.[0])).not.toContain('Partie');
  });

  it('relit chez le cœur l’émulateur ouvert (après un rechargement de la page)', async () => {
    etatCoeur.mockResolvedValueOnce('Ryubing');
    await relireEtat();
    expect(emulateurSeul.ouvert).toBe('Ryubing');
    etatCoeur.mockRejectedValueOnce(new Error('hors de Frogtend'));
    await relireEtat();
    expect(emulateurSeul.ouvert).toBeNull();
  });
});
