// L'enchaînement « Prêt à jouer » avec un faux Firehouse et un faux PC : une seule question, puis tout se fait.
import { beforeEach, describe, expect, it, vi } from 'vitest';

const appels: string[] = [];
const reglages: Record<string, unknown> = {};

vi.mock('$lib/api', () => ({
  taille: (o: number) => `${(o / 1024 / 1024).toFixed(1)} Mo`,
  api: {
    emulateursRecommandes: vi.fn(async () => ({
      emulateurs: [{ nom: 'RPCS3', site: '', recommande: true, ligne_de_commande: '--no-gui', extensions: [], bios: '' }],
    })),
    emulateursInstalles: vi.fn(async () => []),
    emulateurFiche: vi.fn(async () => ({ id: 'rpcs3', nom: 'RPCS3', ligne: '', installable: true })),
    emulateurDerniereVersion: vi.fn(async () => ({ url: 'https://github.com/x', version: '1', taille: 38914236 })),
    rpcs3Micrologiciel: vi.fn(async () => null),
    emulateurReglerManette: vi.fn(async (id: string) => {
      appels.push(`manette ${id}`);
      return { reglee: true, profils_ajoutes: 0 };
    }),
  },
}));

const confirmer = vi.fn(async (..._a: unknown[]) => true);
const informer = vi.fn(async (..._a: unknown[]) => {});
vi.mock('$lib/dialogues/fenetres.svelte', () => ({ confirmer: (...a: unknown[]) => confirmer(...a), informer: (...a: unknown[]) => informer(...a), toast: vi.fn() }));
vi.mock('$lib/dialogues/messages', () => ({ motifDuRefus: () => '' }));

const etat = { pc: { emulateurs: {} as Record<string, unknown>, dossierEmulateurs: '', emplacements: { defaut: ['E:\\Jeux'], systemes: {} } } };
vi.mock('$lib/etat.svelte', () => ({
  etat,
  reglerPc: vi.fn(async (cle: string, v: unknown) => {
    reglages[cle] = v;
    if (cle === 'emulateurs') etat.pc.emulateurs = v as Record<string, unknown>;
    if (cle === 'dossierEmulateurs') etat.pc.dossierEmulateurs = v as string;
  }),
}));

const installer = vi.fn(async (id: string, _nom: string, _maj: boolean, o: { sansQuestion?: boolean }) => {
  appels.push(`installer ${id} sansQuestion=${o.sansQuestion}`);
  return { id, nom: 'RPCS3', programme: 'E:\\Emulateurs\\RPCS3\\rpcs3.exe', dossier: 'E:\\Emulateurs\\RPCS3' };
});
const assistant = vi.fn(async () => 'assistant');
vi.mock('./assistant.svelte', () => ({
  installerEmulateur: (...a: unknown[]) => installer(...(a as [string, string, boolean, { sansQuestion?: boolean }])),
  installerParFirehouse: vi.fn(),
  completerRetroArch: vi.fn(),
  recommandationsParDefaut: () => [],
  reglerEmulateur: () => assistant(),
}));

const { preparerPourJouer } = await import('./pret.svelte');

describe('prêt à jouer, de bout en bout', () => {
  beforeEach(() => {
    appels.length = 0;
    confirmer.mockClear();
    etat.pc.emulateurs = {};
    etat.pc.dossierEmulateurs = '';
  });

  it('une seule question, puis l’émulateur est installé, retenu et sa manette réglée', async () => {
    const cle = await preparerPourJouer('Sony Playstation 3', 7);
    expect(cle).toBe('1');
    expect(confirmer).toHaveBeenCalledTimes(1);
    const [titre, o] = confirmer.mock.calls[0] as [string, { message: string }];
    expect(titre).toContain('Sony Playstation 3');
    expect(o.message).toContain('tes 7 jeux');
    expect(o.message).toContain('37.1 Mo');
    expect(o.message).toContain('E:\\Emulateurs\\RPCS3');
    expect(appels).toEqual(['installer rpcs3 sansQuestion=true', 'manette rpcs3']);
    expect(reglages.dossierEmulateurs).toBe('E:\\Emulateurs');
    expect(JSON.stringify(etat.pc.emulateurs)).toContain('rpcs3.exe');
    // Pas de micrologiciel PS3 : c'est dit tout de suite.
    expect(informer).toHaveBeenCalledTimes(1);
    expect(String(informer.mock.calls[0][0])).toContain('micrologiciel');
  });

  it('ne demande rien si la console a déjà son émulateur', async () => {
    etat.pc.emulateurs = { 'Sony Playstation 3': { defaut: '1', liste: [{ cle: '1', nom: 'RPCS3', programme: 'x', ligne: '' }] } };
    expect(await preparerPourJouer('Sony Playstation 3', 1)).toBe('1');
    expect(confirmer).not.toHaveBeenCalled();
  });

  it('n’installe rien si la personne dit non', async () => {
    confirmer.mockResolvedValueOnce(false);
    expect(await preparerPourJouer('Sony Playstation 3', 1)).toBeNull();
    expect(appels).toEqual([]);
    expect(etat.pc.dossierEmulateurs).toBe('');
  });

  it('les jeux Windows n’ont pas besoin d’émulateur', async () => {
    expect(await preparerPourJouer('Windows', 3)).toBeNull();
    expect(confirmer).not.toHaveBeenCalled();
  });
});
