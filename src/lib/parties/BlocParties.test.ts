// 📁 Où sont mes parties (lot S1) : la section de la fiche (bureau) et de Taodbox (canapé, sans action), sur la forme
// RÉELLE de `parties_emplacements` (commandes.rs, `PartiesVues`).
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';

const DOSSIER = 'C:\\Users\\Léa\\Saved Games\\CD Projekt Red\\Cyberpunk 2077';
const LU = {
  firehouse: 'lu',
  lu_le: '1763800000',
  hors_ligne: false,
  motif: null,
  jeton_refuse: false,
  moteur: null,
  source: 'ludusavi',
  reconnu_par: 'gog_id',
  autres_systemes: ['Mac', 'Microsoft Store'],
  emplacements: [
    { n: 0, dossier: DOSSIER, filtre: null, nature: 'dossier', origine: 'firehouse', os: 'Windows', source: 'ludusavi', nuage_boutique: false, lisible: true, existe: true, fichiers: 2, octets: 7, trop: false, illisible: false },
    { n: 1, dossier: null, filtre: null, nature: 'dossier', origine: 'firehouse', os: 'Windows', source: null, nuage_boutique: false, lisible: false, existe: false, fichiers: 0, octets: 0, trop: false, illisible: false },
    { n: 2, dossier: 'C:\\Users\\Léa\\AppData\\Local\\Jeu', filtre: null, nature: 'dossier', origine: 'firehouse', os: 'Windows', source: 'ludusavi', nuage_boutique: false, lisible: true, existe: false, fichiers: 0, octets: 0, trop: false, illisible: false },
  ],
};
let reponse: object = LU;
const emplacements = vi.fn(async (..._a: unknown[]) => structuredClone(reponse));
let finOuverture: () => void = () => {};
const ouvrirDossier = vi.fn((..._a: unknown[]) => new Promise<void>((r) => (finOuverture = r)));
vi.mock('$lib/api', () => ({
  taille: (o: number) => `${o} o`,
  api: {
    partiesEmplacements: (...a: unknown[]) => emplacements(...a),
    partiesOuvrirDossier: (...a: unknown[]) => ouvrirDossier(...a),
  },
}));
const toast = vi.fn();
vi.mock('$lib/dialogues/fenetres.svelte', () => ({ toast: (...a: unknown[]) => toast(...a) }));

import BlocParties from './BlocParties.svelte';

beforeEach(() => {
  reponse = LU;
  emplacements.mockClear();
  ouvrirDossier.mockClear();
  toast.mockClear();
});
afterEach(() => cleanup());

describe('📁 Où sont mes parties (section)', () => {
  it('montre le dossier de ce PC, ce qu’il contient, « d’après Firehouse », et ouvre le dossier au bureau', async () => {
    render(BlocParties, { id: -7 });
    expect(await screen.findByText(DOSSIER)).toBeTruthy();
    expect(emplacements).toHaveBeenCalledWith(-7);
    expect(screen.getByText('2 fichiers · 7 o')).toBeTruthy();
    // « d'après Firehouse » ; la source précise au survol seulement.
    const origines = screen.getAllByText('d’après Firehouse');
    expect(origines.length).toBe(2);
    expect(origines[0].getAttribute('title')).toBe('Source : Ludusavi');
    expect(screen.queryByText(/Ludusavi|PCGamingWiki/)).toBeNull();
    // L'endroit illisible ici : pas de carte « — », une phrase sous la liste.
    expect(screen.queryByText('—')).toBeNull();
    expect(screen.getByText('1 autre endroit connu ne concerne pas ce PC.')).toBeTruthy();
    expect(screen.getAllByRole('listitem').length).toBe(2);
    // Un seul bouton : le dossier absent n'en a pas.
    const b = screen.getByRole('button', { name: /Ouvrir le dossier/ });
    expect(screen.getAllByRole('button').length).toBe(1);
    await fireEvent.click(b);
    expect(ouvrirDossier).toHaveBeenCalledWith(-7, DOSSIER);
    await tick();
    // Pendant l'ouverture : le bouton attend (aria-disabled, il garde le focus) et un 2e clic ne fait rien.
    expect(b.getAttribute('aria-disabled')).toBe('true');
    await fireEvent.click(b);
    expect(ouvrirDossier).toHaveBeenCalledTimes(1);
    finOuverture();
    await new Promise((r) => setTimeout(r, 0));
    await tick();
    expect(b.getAttribute('aria-disabled')).toBe('false');
    // Seule la ligne d'état est annoncée (jamais toute la liste).
    expect(document.querySelectorAll('[aria-live]').length).toBe(1);
    expect(document.querySelector('[aria-live] ul')).toBeNull();
  });

  it('au canapé : lisible, sans aucune action, chaque ligne se sélectionne à la croix', async () => {
    render(BlocParties, { id: -7, canape: true });
    expect(await screen.findByText(DOSSIER)).toBeTruthy();
    expect(screen.queryAllByRole('button').length).toBe(0);
    const lignes = screen.getAllByRole('listitem');
    expect(lignes.length).toBe(2);
    expect(lignes.every((l) => l.getAttribute('tabindex') === '0')).toBe(true);
  });

  it('au bureau, les lignes ne sont pas des arrêts du clavier en plus', async () => {
    render(BlocParties, { id: -7 });
    await screen.findByText(DOSSIER);
    expect(screen.getAllByRole('listitem').every((l) => !l.hasAttribute('tabindex'))).toBe(true);
  });

  it('dit quand Firehouse ne connaît pas encore le jeu', async () => {
    reponse = { ...LU, firehouse: 'inconnu', autres_systemes: [], emplacements: [] };
    render(BlocParties, { id: 900 });
    expect(await screen.findByText('Firehouse ne connaît pas encore l’endroit des parties de ce jeu pour ce PC.')).toBeTruthy();
  });

  it('un jeu Epic connu seulement pour Steam (TerraScape, 1.48) : dit tel quel, aucun dossier montré', async () => {
    reponse = { ...LU, firehouse: 'inconnu', autres_systemes: ['Steam'], emplacements: [] };
    render(BlocParties, { id: -9 });
    expect(await screen.findByText('Firehouse connaît l’endroit des parties de ce jeu pour la version Steam, pas encore pour la tienne.')).toBeTruthy();
    expect(screen.queryAllByRole('button').length).toBe(0);
  });

  it('hors ligne avec ce qui était gardé : le dit, et montre la liste', async () => {
    reponse = { ...LU, hors_ligne: true };
    render(BlocParties, { id: 900 });
    expect(await screen.findByText('Hors ligne : voici ce que Frogtend savait déjà.')).toBeTruthy();
    expect(screen.getByText(DOSSIER)).toBeTruthy();
  });

  it('hors ligne sans rien de gardé : « pas encore pu demander », jamais « ne connaît pas »', async () => {
    reponse = { ...LU, hors_ligne: true, firehouse: 'inconnu', autres_systemes: [], emplacements: [] };
    render(BlocParties, { id: 900 });
    expect(await screen.findByText('Frogtend n’a pas encore pu demander à Firehouse où ce jeu range ses parties. Réessaie une fois connecté.')).toBeTruthy();
    expect(screen.queryByText(/ne connaît pas/)).toBeNull();
  });

  it('le refus du cœur est dit tel quel (sa vraie forme {sorte, motif})', async () => {
    emplacements.mockImplementationOnce(async () => Promise.reject({ sorte: 'refus', motif: 'Ce n’est pas un jeu PC : ses parties sont gardées par son émulateur.' }));
    render(BlocParties, { id: 110 });
    expect(await screen.findByText(/Ce n’est pas un jeu PC/)).toBeTruthy();
  });
});
