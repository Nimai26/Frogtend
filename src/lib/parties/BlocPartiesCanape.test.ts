// 🛟 Mettre mes parties à l'abri au canapé (lot S2, Taodbox) : la VRAIE fenêtre maison (`confirmer`) et la manette
// (`appliquer` : A = valider, B = retour), sur la forme réelle des réponses du cœur.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';

const DOSSIER = 'C:\\Users\\Léa\\Saved Games\\CD Projekt Red\\Cyberpunk 2077';
const VUES = {
  firehouse: 'lu',
  lu_le: '1763800000',
  hors_ligne: false,
  motif: null,
  jeton_refuse: false,
  moteur: null,
  source: 'ludusavi',
  reconnu_par: 'gog_id',
  autres_systemes: [],
  emplacements: [{ n: 0, dossier: DOSSIER, filtre: null, nature: 'dossier', origine: 'firehouse', os: 'Windows', source: 'ludusavi', nuage_boutique: false, lisible: true, existe: true, fichiers: 2, octets: 7, en_ligne: 0, trop: false, illisible: false }],
};
const abriter = vi.fn(async (..._a: unknown[]) => ({ date: '1763800000', fichiers: 2, octets: 7, emplacements: 1, echecs: [], en_ligne_seulement: 0, telecharges: 0, deja: false }));
let enLigne = 0;
vi.mock('@tauri-apps/api/event', () => ({ listen: async () => () => {} }));
vi.mock('$lib/api', () => ({
  taille: (o: number) => `${o} o`,
  api: {
    partiesEmplacements: async () => structuredClone(VUES),
    partiesOuvrirDossier: async () => {},
    partiesAbriApercu: async () => ({ fichiers: 2, octets: 7, emplacements: 1, echecs: [], en_ligne_seulement: enLigne, octets_en_ligne: enLigne * 100, deja: false, gros: false, gros_avec_nuage: false, profil: 'Léa' }),
    partiesAbriter: (...a: unknown[]) => abriter(...a),
    partiesAbris: async () => [],
  },
}));

import Dialogues from '$lib/dialogues/Dialogues.svelte';
import { pile } from '$lib/dialogues/fenetres.svelte';
import { appliquer, fenetreOuverte, selectionner } from '$lib/taodbox/manette.svelte';
import BlocParties from './BlocParties.svelte';

// jsdom ne fait pas défiler : `selectionner` appelle scrollIntoView.
Element.prototype.scrollIntoView = () => {};

async function attendre() {
  for (let i = 0; i < 5; i++) {
    await new Promise((r) => setTimeout(r, 0));
    await tick();
  }
}

beforeEach(() => {
  abriter.mockClear();
  enLigne = 0;
});
afterEach(() => {
  cleanup();
  pile.dialogues.length = 0;
  pile.toasts.length = 0;
});

describe('🛟 au canapé, tout à la manette', () => {
  it('A sur le bouton ouvre la fenêtre maison (jamais une boîte native), A encore met à l’abri', async () => {
    const natif = vi.spyOn(window, 'confirm');
    render(Dialogues);
    render(BlocParties, { id: -7, canape: true });
    const b = await screen.findByRole('button', { name: /Mettre mes parties à l’abri/ });
    selectionner(b);
    appliquer('valider');
    await attendre();
    expect(fenetreOuverte()).toBe(true);
    const fenetre = screen.getByRole('alertdialog');
    expect(fenetre.textContent).toContain('Ces parties seront rangées dans ton profil « Léa ».');
    expect(fenetre.textContent).toContain('2 fichiers · 7 o');
    // La validation a la main (pas une action dangereuse) : A la presse.
    expect(document.activeElement?.textContent?.trim()).toBe('Mettre à l’abri');
    appliquer('valider');
    await attendre();
    expect(abriter).toHaveBeenCalledWith(-7, false, false);
    expect(fenetreOuverte()).toBe(false);
    expect(natif).not.toHaveBeenCalled();
  });

  it('B ferme la fenêtre sans rien copier', async () => {
    render(Dialogues);
    render(BlocParties, { id: -7, canape: true });
    selectionner(await screen.findByRole('button', { name: /Mettre mes parties à l’abri/ }));
    appliquer('valider');
    await attendre();
    expect(fenetreOuverte()).toBe(true);
    appliquer('retour');
    await attendre();
    expect(fenetreOuverte()).toBe(false);
    expect(abriter).not.toHaveBeenCalled();
  });
});

describe('☁ au canapé : le choix OneDrive à la manette', () => {
  it('la croix va sur « Télécharger depuis OneDrive », A le choisit ; B annule sans rien', async () => {
    enLigne = 9;
    render(Dialogues);
    render(BlocParties, { id: -7, canape: true });
    selectionner(await screen.findByRole('button', { name: /Mettre mes parties à l’abri/ }));
    appliquer('valider');
    await attendre();
    expect(fenetreOuverte()).toBe(true);
    const fenetre = screen.getByRole('dialog');
    expect(fenetre.textContent).toContain('seulement dans ton nuage OneDrive');
    const choix = screen.getByRole('button', { name: /Télécharger depuis OneDrive puis mettre à l’abri/ });
    expect(screen.getByRole('button', { name: /Mettre à l’abri sans eux/ })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Annuler' })).toBeTruthy();
    selectionner(choix);
    appliquer('valider');
    await attendre();
    expect(abriter).toHaveBeenCalledWith(-7, false, true);
    expect(fenetreOuverte()).toBe(false);
    // B : rien n'est copié ni téléchargé.
    abriter.mockClear();
    selectionner(screen.getByRole('button', { name: /Mettre mes parties à l’abri/ }));
    appliquer('valider');
    await attendre();
    appliquer('retour');
    await attendre();
    expect(fenetreOuverte()).toBe(false);
    expect(abriter).not.toHaveBeenCalled();
  });
});
