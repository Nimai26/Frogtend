// Lot C4c-2 : par le chemin RÉEL (bouton « ⬇ Installer » et double-clic → ouvrirParLaBoutique → ouvrirJeuDeBoutique),
// un jeu GOG pas installé s'installe par Frogtend (plus par GOG Galaxy), puis les jeux du PC et la ludothèque sont
// relus ; « ▶ Jouer » et les autres boutiques ne changent pas.
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { JeuResume } from '$lib/api';

const installerGog = vi.fn(async (..._a: unknown[]) => true);
vi.mock('./installer-gog.svelte', () => ({ installerGog: (...a: unknown[]) => installerGog(...a) }));
const api = { boutiqueOuvrir: vi.fn(async () => {}), gogAvertissementInstallation: vi.fn(async () => null), lister: vi.fn(async () => ({ jeux: [], total: 0 })) };
vi.mock('$lib/api', async (original) => ({ ...(await original<typeof import('$lib/api')>()), api }));
const rechargerJeuxDuPc = vi.fn(async () => {});
vi.mock('$lib/ludotheque/telechargements.svelte', () => ({ rechargerJeuxDuPc: () => rechargerJeuxDuPc(), suivreTelechargements: () => {} }));
const rechargerListe = vi.fn(async () => {});
vi.mock('$lib/ludotheque/ludotheque.svelte', () => ({ rechargerListe: () => rechargerListe(), rechargerPlateformes: async () => {} }));
vi.mock('$lib/ludotheque/jeu.svelte', () => ({ partie: { enJeu: null } }));
const toast = vi.fn();
vi.mock('$lib/dialogues/fenetres.svelte', () => ({ toast: (...a: unknown[]) => toast(...a), choisir: vi.fn(), confirmer: vi.fn(async () => true) }));

const { ouvrirParLaBoutique } = await import('./boutiques-pc.svelte');

const carte = (o: Partial<JeuResume>): JeuResume => ({ id: -5, titre: 'Another World', plateforme: 'Windows', source: 'gog', boutique: 'gog', cle_boutique: '1207658924', installe: false, ...o }) as JeuResume;

describe('« ⬇ Installer » un jeu GOG depuis la ludothèque', () => {
  beforeEach(() => {
    [installerGog, api.boutiqueOuvrir, rechargerJeuxDuPc, rechargerListe, toast].forEach((f) => f.mockClear());
  });

  it('passe par l’installation de Frogtend, jamais par GOG Galaxy (qui relit elle-même les listes)', async () => {
    await ouvrirParLaBoutique(carte({}));
    expect(installerGog).toHaveBeenCalledWith({ cle: '1207658924', titre: 'Another World' }, {});
    expect(api.boutiqueOuvrir).not.toHaveBeenCalled();
  });

  it('▶ Jouer et Epic gardent leur chemin', async () => {
    await ouvrirParLaBoutique(carte({ installe: true }));
    expect(api.boutiqueOuvrir).toHaveBeenCalledWith('gog', '1207658924', 'jouer');
    await ouvrirParLaBoutique(carte({ source: 'epic', boutique: 'epic', cle_boutique: 'Fortnite' }));
    expect(api.boutiqueOuvrir).toHaveBeenLastCalledWith('epic', 'Fortnite', 'installer');
    expect(installerGog).not.toHaveBeenCalled();
  });
});
