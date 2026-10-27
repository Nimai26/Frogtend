import { beforeEach, describe, expect, it, vi } from 'vitest';

const choisir = vi.fn();
const crees: string[] = [];
let propose: { chemin: string; disque: string; libre: number } | null = null;
vi.mock('$lib/api', () => ({
  taille: (o: number) => `${o} o`,
  api: {
    dossierJeuxPropose: vi.fn(async () => propose),
    dossierJeuxCreer: vi.fn(async (c: string) => {
      crees.push(c);
      return c;
    }),
    // Le cœur refuse la racine d'un disque (garde::dossier_systeme).
    dossierJeuxRefus: vi.fn(async (c: string) => (/^[A-Z]:\\?$/i.test(c) ? `« ${c} » ne convient pas` : null)),
  },
}));
const informer = vi.fn();
vi.mock('$lib/dialogues/fenetres.svelte', () => ({ choisir: (...a: unknown[]) => choisir(...a), informer: (...a: unknown[]) => informer(...a), toast: vi.fn() }));
let canape = false;
vi.mock('$lib/taodbox/canape', () => ({ auCanape: () => canape }));
vi.mock('$lib/dialogues/messages', () => ({ motifDuRefus: () => '' }));
const etat = { pc: { emplacements: { defaut: [] as string[], systemes: {} } } };
vi.mock('$lib/etat.svelte', () => ({
  etat,
  reglerPc: vi.fn(async (cle: string, v: unknown) => {
    if (cle === 'emplacements') etat.pc.emplacements = v as typeof etat.pc.emplacements;
  }),
}));
const parcourir = vi.fn();
vi.mock('$lib/emulateurs/assistant.svelte', () => ({ parcourir: (...a: unknown[]) => parcourir(...a) }));

const { proposerDossierJeux } = await import('./dossierJeux');

describe('où ranger les jeux sur un PC neuf', () => {
  beforeEach(() => {
    etat.pc.emplacements = { defaut: [], systemes: {} };
    crees.length = 0;
    choisir.mockReset();
    parcourir.mockReset();
    informer.mockReset();
  });

  it('propose le dossier du disque le plus libre, le crée au oui, et le retient', async () => {
    propose = { chemin: 'F:\\Frogtend\\Jeux', disque: 'F:\\', libre: 900 };
    choisir.mockResolvedValueOnce('propose');
    expect(await proposerDossierJeux('Premier jeu.')).toBe('F:\\Frogtend\\Jeux');
    expect(crees).toEqual(['F:\\Frogtend\\Jeux']);
    expect(etat.pc.emplacements.defaut).toEqual(['F:\\Frogtend\\Jeux']);
    const [, options, message] = choisir.mock.calls[0] as [string, { libelle: string; detail: string }[], string];
    expect(options[0].detail).toContain('900');
    expect(message).toContain('Premier jeu.');
  });

  it('la racine d’un disque est refusée, dite, et la même question revient (rien de perdu)', async () => {
    propose = null;
    informer.mockReset();
    choisir.mockResolvedValueOnce('autre').mockResolvedValueOnce('autre');
    parcourir.mockResolvedValueOnce('D:\\').mockResolvedValueOnce('D:\\Jeux');
    expect(await proposerDossierJeux('x')).toBe('D:\\Jeux');
    expect(informer).toHaveBeenCalledTimes(1);
    expect(choisir).toHaveBeenCalledTimes(2);
    expect(etat.pc.emplacements.defaut).toEqual(['D:\\Jeux']);
  });

  it('un autre dossier choisi dans l’explorateur ; rien de créé ni retenu si la personne renonce', async () => {
    propose = null; // aucun disque connu : seulement « choisir »
    choisir.mockResolvedValueOnce('autre');
    parcourir.mockResolvedValueOnce('G:\\Mes jeux');
    expect(await proposerDossierJeux('x')).toBe('G:\\Mes jeux');
    expect((choisir.mock.calls[0][1] as unknown[]).length).toBe(1);
    expect(crees).toEqual([]);
    choisir.mockResolvedValueOnce(null);
    etat.pc.emplacements = { defaut: [], systemes: {} };
    expect(await proposerDossierJeux('x')).toBeNull();
    expect(etat.pc.emplacements.defaut).toEqual([]);
  });

  it('au canapé : jamais l’explorateur ; sans disque proposé, dit de le faire à l’écran du PC', async () => {
    canape = true;
    propose = { chemin: 'F:\\Frogtend\\Jeux', disque: 'F:\\', libre: 900 };
    choisir.mockResolvedValueOnce('propose');
    expect(await proposerDossierJeux('x', 5)).toBe('F:\\Frogtend\\Jeux');
    const [, options, message] = choisir.mock.calls[0] as [string, unknown[], string];
    expect(options.length).toBe(1);
    expect(message).toContain('Ce jeu demande 5 o');
    propose = null;
    choisir.mockClear();
    expect(await proposerDossierJeux('x')).toBeNull();
    expect(choisir).not.toHaveBeenCalled();
    expect(informer).toHaveBeenCalledTimes(1);
    canape = false;
  });
});
