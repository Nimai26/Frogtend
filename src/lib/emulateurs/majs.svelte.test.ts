// Les mises à jour d'émulateurs au démarrage (Seb, 08/11) : une fois par jour et par PC, une seule question, jamais
// en Taodbox ni pendant une partie, silencieux hors ligne ; puis l'une après l'autre.
import { beforeEach, describe, expect, it, vi } from 'vitest';

const installes = [
  { id: 'rpcs3', nom: 'RPCS3', version: '0.0.38', par_frogtend: true, dossier: '', programme: '', installe_le: '' },
  { id: 'eglantine', nom: 'Eglantine', version: 'daily-20261107', par_frogtend: true, dossier: '', programme: '', installe_le: '' },
  { id: 'dolphin', nom: 'Dolphin', version: '2509', par_frogtend: false, dossier: '', programme: '', installe_le: '' },
];
const servies: Record<string, { version: string; taille: number }> = {
  rpcs3: { version: '0.0.39', taille: 40 * 1048576 },
  eglantine: { version: 'v0.1.0-rc3', taille: 70 * 1048576 },
};
let horsLigne = false;
vi.mock('$lib/api', () => ({
  taille: (o: number) => `${Math.round(o / 1048576)} Mo`,
  api: {
    emulateursInstalles: vi.fn(async () => installes),
    emulateurDerniereVersion: vi.fn(async (id: string) => {
      if (horsLigne) throw new Error('hors ligne');
      return { url: 'https://x', ...servies[id] };
    }),
  },
}));
const confirmer = vi.fn(async (..._a: unknown[]) => true);
const pile = { dialogues: [] as unknown[] };
vi.mock('$lib/dialogues/fenetres.svelte', () => ({ confirmer: (...a: unknown[]) => confirmer(...a), toast: vi.fn(), pile }));
const etat = { pc: { majEmulateursDerniere: 0, firehouse: { simule: false } } };
vi.mock('$lib/etat.svelte', () => ({
  etat,
  reglerPc: vi.fn(async (cle: string, v: number) => {
    if (cle === 'majEmulateursDerniere') etat.pc.majEmulateursDerniere = v;
  }),
}));
let canape = false;
vi.mock('$lib/taodbox/canape', () => ({ auCanape: () => canape }));
const partie = { enJeu: null as number | null, lancement: null as number | null };
vi.mock('$lib/ludotheque/jeu.svelte', () => ({ partie }));
const installer = vi.fn(async (id: string, _n: string, maj: boolean, o: { sansQuestion?: boolean }) => ({ id, maj, o }));
vi.mock('./assistant.svelte', () => ({
  installationEmulateur: { id: null },
  installerEmulateur: (...a: [string, string, boolean, { sansQuestion?: boolean }]) => installer(...a),
}));

beforeEach(() => {
  vi.resetModules();
  confirmer.mockClear();
  installer.mockClear();
  etat.pc.majEmulateursDerniere = 0;
  horsLigne = false;
  canape = false;
  partie.enJeu = null;
  pile.dialogues = [];
});

describe('les mises à jour d’émulateurs au démarrage', () => {
  it('une seule question pour celles trouvées (installées par Frogtend), puis l’une après l’autre sans redemander', async () => {
    const m = await import('./majs.svelte');
    await m.emulateursAuDemarrage();
    expect(confirmer).toHaveBeenCalledTimes(1);
    const message = String((confirmer.mock.calls[0][1] as { message: string }).message);
    expect(message).toContain('RPCS3 0.0.39 (40 Mo)');
    expect(message).toContain('Eglantine v0.1.0-rc3');
    expect(message).not.toContain('Dolphin');
    expect(installer.mock.calls.map((c) => [c[0], c[2], c[3].sansQuestion])).toEqual([
      ['rpcs3', true, true],
      ['eglantine', true, true],
    ]);
    expect(etat.pc.majEmulateursDerniere).toBeGreaterThan(0);
  });

  it('une fois par jour : le lendemain seulement', async () => {
    etat.pc.majEmulateursDerniere = Math.floor(Date.now() / 1000) - 3600;
    const m = await import('./majs.svelte');
    await m.emulateursAuDemarrage();
    expect(confirmer).not.toHaveBeenCalled();
  });

  it('rien en Taodbox, rien hors ligne (et la vérification revient au prochain démarrage)', async () => {
    canape = true;
    await (await import('./majs.svelte')).emulateursAuDemarrage();
    expect(confirmer).not.toHaveBeenCalled();
    vi.resetModules();
    canape = false;
    horsLigne = true;
    await (await import('./majs.svelte')).emulateursAuDemarrage();
    expect(confirmer).not.toHaveBeenCalled();
    expect(etat.pc.majEmulateursDerniere).toBe(0);
  });

  it('pendant une partie : pas de question', async () => {
    partie.enJeu = 12;
    await (await import('./majs.svelte')).emulateursAuDemarrage();
    expect(confirmer).not.toHaveBeenCalled();
  });

  it('une autre question du démarrage est ouverte : elle passe d’abord, jamais deux fenêtres empilées', async () => {
    vi.useFakeTimers();
    pile.dialogues = [{ titre: 'Game Bar' }];
    const fin = (await import('./majs.svelte')).emulateursAuDemarrage();
    await vi.advanceTimersByTimeAsync(2000);
    expect(confirmer).not.toHaveBeenCalled();
    pile.dialogues = [];
    await vi.advanceTimersByTimeAsync(1000);
    vi.useRealTimers();
    await fin;
    expect(confirmer).toHaveBeenCalledTimes(1);
  });

  it('« Plus tard » : rien n’est installé', async () => {
    confirmer.mockResolvedValueOnce(false);
    await (await import('./majs.svelte')).emulateursAuDemarrage();
    expect(installer).not.toHaveBeenCalled();
  });
});
