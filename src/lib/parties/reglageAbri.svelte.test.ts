// Lot S3 (Seb, 23/11) : « 🛟 Mettre mes parties à l'abri sur ce PC après chaque partie » — allumé d'office, réglable
// dans Options ▸ Sauvegarde ET dans les réglages du jeu de Taodbox (à la manette : un interrupteur, A le bascule).
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';
import { DEFAUTS_PROFIL } from '$lib/reglages/reglages';

const profil = $state(structuredClone(DEFAUTS_PROFIL));
const reglerProfil = vi.fn(async (chemin: string, valeur: unknown) => {
  if (chemin === 'parties.abriApresPartie') profil.parties.abriApresPartie = valeur as boolean;
  if (chemin === 'parties.rattrapage') profil.parties.rattrapage = valeur as boolean;
});
vi.mock('$lib/etat.svelte', () => ({
  etat: {
    get profil() {
      return profil;
    },
    pc: { firehouse: { simule: true }, envoisFirehouse: { debit: 'moitie' } },
  },
  reglerProfil: (c: string, v: unknown) => reglerProfil(c, v),
  reglerPc: vi.fn(),
}));
vi.mock('$lib/api', () => ({ api: { restaurationConflits: async () => [] }, taille: (o: number) => `${o} o` }));
vi.mock('$lib/ludotheque/ludotheque.svelte', () => ({ depuis: () => 'hier' }));
vi.mock('$lib/sauvegarde.svelte', () => ({
  sauvegarde: { derniere: null, enCours: false, restauration: null },
  lireDerniereSauvegarde: vi.fn(),
  proposerRemplacements: vi.fn(),
  restaurer: vi.fn(),
  sauvegarder: vi.fn(),
}));
const adresse = { url: new URL('http://localhost/taodbox/reglages?jeu=900&titre=Hades') };
vi.mock('$app/state', () => ({
  get page() {
    return adresse;
  },
}));
let plateforme = 'Windows';
vi.mock('$lib/ludotheque/telechargements.svelte', () => ({
  tele: {
    get jeux() {
      return { 900: { titre: 'Hades', plateforme } };
    },
  },
}));
vi.mock('$lib/taodbox/manette.svelte', () => ({ fenetreOuverte: () => false, selectionner: vi.fn() }));
vi.mock('$lib/contenus/BlocContenus.svelte', async () => ({ default: (await import('./FauxBlocContenus.test.svelte')).default }));

import Sauvegarde from '$lib/options/Sauvegarde.svelte';
import ReglagesTaodbox from '../../routes/taodbox/reglages/+page.svelte';

beforeEach(() => {
  profil.parties.abriApresPartie = true;
  profil.parties.rattrapage = true;
  plateforme = 'Windows';
  reglerProfil.mockClear();
});
afterEach(() => cleanup());

describe('🛟 Le réglage de l’abri après une partie', () => {
  it('allumé d’office', () => {
    expect(DEFAUTS_PROFIL.parties.abriApresPartie).toBe(true);
  });

  it('Options ▸ Sauvegarde : une case, cochée d’office, qui règle le profil', async () => {
    render(Sauvegarde);
    const c = screen.getByRole('checkbox', { name: '🛟 Mettre mes parties à l’abri sur ce PC après chaque partie (pour tous tes jeux PC)' }) as HTMLInputElement;
    expect(c.checked).toBe(true);
    await fireEvent.click(c);
    expect(reglerProfil).toHaveBeenCalledWith('parties.abriApresPartie', false);
  });

  it('Taodbox : un interrupteur pour un jeu PC, et plus de « Rien à régler » faux', async () => {
    render(ReglagesTaodbox);
    await tick();
    const s = screen.getByRole('switch', { name: /Mettre mes parties à l’abri sur ce PC après chaque partie/ });
    expect(s.getAttribute('aria-checked')).toBe('true');
    await fireEvent.click(s);
    await tick();
    expect(reglerProfil).toHaveBeenCalledWith('parties.abriApresPartie', false);
    expect(s.getAttribute('aria-checked')).toBe('false');
    expect(screen.queryByText(/Rien à régler pour ce jeu/)).toBeNull();
    expect(screen.queryByText(/ni mise à jour, ni DLC, ni mod/)).toBeNull();
  });

  it('Taodbox : pas d’interrupteur pour un jeu d’émulateur', async () => {
    plateforme = 'Super Nintendo';
    render(ReglagesTaodbox);
    await tick();
    expect(screen.queryByRole('switch', { name: /après chaque partie/ })).toBeNull();
    expect(screen.getByText('Rien à régler pour ce jeu : il n’a ni mise à jour, ni DLC, ni mod.')).toBeTruthy();
  });
});

// Lot S7 (Seb 24/11) : « 🛟 Rattraper les parties jouées hors de Frogtend » — allumé d'office, SÉPARÉ de l'abri après une
// partie, réglable dans Options ▸ Sauvegarde ET dans les réglages de Taodbox (à la manette).
describe('🛟 Le réglage du rattrapage', () => {
  it('allumé d’office', () => {
    expect(DEFAUTS_PROFIL.parties.rattrapage).toBe(true);
  });

  it('Options ▸ Sauvegarde : sa propre case, qui ne touche pas à l’abri après une partie', async () => {
    render(Sauvegarde);
    const c = screen.getByRole('checkbox', { name: '🛟 Rattraper les parties jouées hors de Frogtend' }) as HTMLInputElement;
    expect(c.checked).toBe(true);
    await fireEvent.click(c);
    expect(reglerProfil).toHaveBeenCalledWith('parties.rattrapage', false);
    expect(reglerProfil).not.toHaveBeenCalledWith('parties.abriApresPartie', expect.anything());
  });

  it('Taodbox : un interrupteur à la manette pour un jeu PC', async () => {
    render(ReglagesTaodbox);
    await tick();
    const s = screen.getByRole('switch', { name: '🛟 Rattraper les parties jouées hors de Frogtend (pour tous tes jeux PC)' });
    expect(s.getAttribute('aria-checked')).toBe('true');
    await fireEvent.click(s);
    await tick();
    expect(reglerProfil).toHaveBeenCalledWith('parties.rattrapage', false);
    expect(s.getAttribute('aria-checked')).toBe('false');
    expect(screen.getByRole('switch', { name: /après chaque partie/ }).getAttribute('aria-checked')).toBe('true');
  });
});
