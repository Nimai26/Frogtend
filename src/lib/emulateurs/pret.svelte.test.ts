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
    dossierJeuxPropose: vi.fn(async () => ({ chemin: 'F:\\Frogtend\\Jeux', disque: 'F:\\', libre: 1 })),
    restaurationReposerEmulateurs: vi.fn(async () => {
      appels.push('reposer les parties restaurées');
      return { reposes: 0, deja_la: 0, conflits: [] };
    }),
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
const completerRetroArch = vi.fn(async (..._a: unknown[]) => 'RetroArch — snes9x');
vi.mock('./assistant.svelte', () => ({
  installationEmulateur: { id: 'rpcs3', recus: 19457118, total: 38914236 },
  installerEmulateur: (...a: unknown[]) => installer(...(a as [string, string, boolean, { sansQuestion?: boolean }])),
  installerParFirehouse: vi.fn(),
  installerMicrologicielPs3: vi.fn(async (programme: string) => {
    appels.push(`micrologiciel ${programme}`);
    return '4.93';
  }),
  poserBios: vi.fn(async (plateforme: string) => {
    appels.push(`bios ${plateforme}`);
  }),
  completerRetroArch: (...a: unknown[]) => completerRetroArch(...a),
  recommandationsParDefaut: () => [],
  reglerEmulateur: () => assistant(),
}));

const { preparerPourJouer, preparationEnCours, libellePreparation } = await import('./pret.svelte');

describe('une préparation par console à la fois (essai de Seb, 28/10)', () => {
  beforeEach(() => {
    appels.length = 0;
    confirmer.mockClear();
    informer.mockClear();
    etat.pc.emulateurs = {};
    etat.pc.dossierEmulateurs = '';
  });

  it('« Jouer » pendant la préparation l’attend : une seule question, une seule installation, la même clé', async () => {
    // L'installation dure (le téléchargement de RetroArch) : elle finit quand le test le décide.
    let finir = () => {};
    const lente = new Promise<void>((r) => (finir = r));
    installer.mockImplementationOnce(async (id: string) => {
      appels.push(`installer ${id} (lente)`);
      await lente;
      return { id, nom: 'RPCS3', programme: 'E:\\Emulateurs\\RPCS3\\rpcs3.exe', dossier: 'E:\\Emulateurs\\RPCS3' };
    });
    const premiere = preparerPourJouer('Sony Playstation 3', 1);
    const seconde = preparerPourJouer('Sony Playstation 3', 1);
    expect(seconde).toBe(premiere);
    expect(preparationEnCours('Sony Playstation 3')).toBe(premiere);
    await vi.waitFor(() => expect(libellePreparation('Sony Playstation 3')).toBe('⏳ Préparation de RPCS3… 50 %'));
    finir();
    expect(await seconde).toBe('1');
    expect(confirmer).toHaveBeenCalledTimes(1);
    expect(appels.filter((a) => a.startsWith('installer'))).toHaveLength(1);
    // Fini : plus rien en cours, le bouton redevient « Jouer ».
    expect(preparationEnCours('Sony Playstation 3')).toBeUndefined();
    expect(libellePreparation('Sony Playstation 3')).toBeNull();
  });

  it('un « non » rend null aux deux appelants, et rien n’est affiché comme en préparation', async () => {
    confirmer.mockImplementationOnce(async () => false);
    const a = preparerPourJouer('Sony Playstation 3', 1);
    const b = preparerPourJouer('Sony Playstation 3', 1);
    expect(await a).toBeNull();
    expect(await b).toBeNull();
    expect(libellePreparation('Sony Playstation 3')).toBeNull();
    expect(appels.filter((x) => x.startsWith('installer'))).toHaveLength(0);
  });
});

describe('prêt à jouer, de bout en bout', () => {
  beforeEach(async () => {
    const { api } = await import('$lib/api');
    vi.mocked(api.emulateurFiche).mockImplementation(async () => ({ id: 'rpcs3', nom: 'RPCS3', ligne: '', installable: true }));
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
    expect(o.message).toContain('E:\\Frogtend\\Emulateurs\\RPCS3');
    expect(appels).toEqual(['installer rpcs3 sansQuestion=true', 'manette rpcs3', 'reposer les parties restaurées']);
    expect(reglages.dossierEmulateurs).toBe('E:\\Frogtend\\Emulateurs');
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

  it('sans dossier de jeux : le disque proposé par le cœur ; une installation ratée ne garde pas le dossier', async () => {
    etat.pc.emplacements.defaut = [];
    installer.mockResolvedValueOnce(null as never);
    expect(await preparerPourJouer('Sony Playstation 3', 1)).toBeNull();
    const [, o] = confirmer.mock.calls[0] as [string, { message: string }];
    expect(o.message).toContain('F:\\Frogtend\\Emulateurs');
    // Proposé de nouveau au prochain essai.
    expect(etat.pc.dossierEmulateurs).toBe('');
    etat.pc.emplacements.defaut = ['E:\\Jeux'];
  });

  it('n’installe rien si la personne dit non', async () => {
    confirmer.mockResolvedValueOnce(false);
    expect(await preparerPourJouer('Sony Playstation 3', 1)).toBeNull();
    expect(appels).toEqual([]);
    expect(etat.pc.dossierEmulateurs).toBe('');
  });

  it('suit le conseil de la maison : son émulateur, sa ligne (avec les options de Frogtend), jamais celle de LaunchBox', async () => {
    const { api } = await import('$lib/api');
    // Relevé réel du 16/10 (SNES) : la liste LaunchBox recommande autre chose ; le conseil dit RetroArch + snes9x.
    vi.mocked(api.emulateursRecommandes).mockResolvedValueOnce({
      conseil: {
        plateforme: 'Super Nintendo Entertainment System',
        emulateur: 'retroarch',
        coeur: 'snes9x_libretro',
        ligne_de_commande: '-L "cores\\snes9x_libretro.dll" "%rom%"',
        alternatives: [],
        bios: [],
        version_attendue: null,
        note: '',
        origine: 'amorce',
        maj_le: '2026-10-04T13:27:38',
      },
      emulateurs: [
        { nom: 'Snes9x', site: '', recommande: true, ligne_de_commande: '', extensions: [], bios: '', id: 'snes9x' },
        { nom: 'RetroArch', site: '', recommande: false, ligne_de_commande: '-L "cores\\bsnes_libretro.dll"', extensions: [], bios: '', id: 'retroarch' },
      ],
    } as never);
    vi.mocked(api.emulateurFiche).mockImplementation(async (nom: string) =>
      nom === 'RetroArch' ? { id: 'retroarch', nom: 'RetroArch', ligne: '-f', installable: true } : null,
    );
    installer.mockResolvedValueOnce({ id: 'retroarch', nom: 'RetroArch', programme: 'E:\\Emulateurs\\RetroArch\\retroarch.exe', dossier: 'E:\\Emulateurs\\RetroArch' });
    const cle = await preparerPourJouer('Super Nintendo Entertainment System', 2);
    expect(cle).toBe('1');
    expect(installer.mock.calls.at(-1)?.[0]).toBe('retroarch');
    expect(appels).toContain('manette retroarch');
    // La Super Nintendo n'a aucun fichier système : rien n'est demandé au cœur.
    expect(appels).not.toContain('bios Super Nintendo Entertainment System');
    expect(completerRetroArch.mock.calls[0][1]).toBe('-L "cores\\snes9x_libretro.dll" -f "%rom%"');
    const sys = etat.pc.emulateurs['Super Nintendo Entertainment System'] as { liste: { ligne: string; conseil?: string }[] };
    expect(sys.liste[0].ligne).toBe('-L "cores\\snes9x_libretro.dll" -f "%rom%"');
    expect(sys.liste[0].conseil).toBe('2026-10-04T13:27:38');
  });

  it('les jeux Windows n’ont pas besoin d’émulateur', async () => {
    expect(await preparerPourJouer('Windows', 3)).toBeNull();
    expect(confirmer).not.toHaveBeenCalled();
  });
});
