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
// Lot S2 : les formes RÉELLES de `parties_abri_apercu`, `parties_abriter`, `parties_abris` (commandes.rs).
const APERCU = { fichiers: 2, octets: 7, emplacements: 1, echecs: [], en_ligne_seulement: 0, deja: false, gros: false, profil: 'Léa' };
let apercu: object = APERCU;
const abriApercu = vi.fn(async (..._a: unknown[]) => structuredClone(apercu));
const abriter = vi.fn(async (..._a: unknown[]) => ({ date: '1763800000', fichiers: 2, octets: 7, emplacements: 1, echecs: [], en_ligne_seulement: 0, deja: false }));
const ABRI = { date: '1763800000', fichiers: 2, octets: 7, sorte: 'emplacements', a_moi: true, echecs: 0, en_ligne_seulement: 0, sans_origine: 0 };
let listeAbris: object[] = [];
const abrisLus = vi.fn(async (..._a: unknown[]) => structuredClone(listeAbris));
vi.mock('$lib/api', () => ({
  taille: (o: number) => `${o} o`,
  api: {
    partiesEmplacements: (...a: unknown[]) => emplacements(...a),
    partiesOuvrirDossier: (...a: unknown[]) => ouvrirDossier(...a),
    partiesAbriApercu: (...a: unknown[]) => abriApercu(...a),
    partiesAbriter: (...a: unknown[]) => abriter(...a),
    partiesAbris: (...a: unknown[]) => abrisLus(...a),
  },
}));
const toast = vi.fn();
let reponseConfirmer = true;
const confirmer = vi.fn(async (..._a: unknown[]) => reponseConfirmer);
const informer = vi.fn(async (..._a: unknown[]) => {});
const toastProgres = vi.fn((..._a: unknown[]) => 42);
const fermerToast = vi.fn();
vi.mock('$lib/dialogues/fenetres.svelte', () => ({
  toast: (...a: unknown[]) => toast(...a),
  confirmer: (...a: unknown[]) => confirmer(...a),
  informer: (...a: unknown[]) => informer(...a),
  toastProgres: (...a: unknown[]) => toastProgres(...a),
  fermerToast: (...a: unknown[]) => fermerToast(...a),
}));

import BlocParties from './BlocParties.svelte';

beforeEach(() => {
  reponse = LU;
  apercu = APERCU;
  listeAbris = [];
  reponseConfirmer = true;
  emplacements.mockClear();
  ouvrirDossier.mockClear();
  toast.mockClear();
  abriApercu.mockClear();
  abriter.mockClear();
  abrisLus.mockClear();
  confirmer.mockClear();
  informer.mockClear();
  toastProgres.mockClear();
  fermerToast.mockClear();
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
    // Un seul « Ouvrir le dossier » : le dossier absent n'en a pas.
    const b = screen.getByRole('button', { name: /Ouvrir le dossier/ });
    expect(screen.getAllByRole('button', { name: /Ouvrir le dossier/ }).length).toBe(1);
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

  it('au canapé : lisible, seule action « 🛟 Mettre mes parties à l’abri », chaque ligne se sélectionne à la croix', async () => {
    render(BlocParties, { id: -7, canape: true });
    expect(await screen.findByText(DOSSIER)).toBeTruthy();
    expect(screen.getAllByRole('button').map((b) => b.textContent?.trim())).toEqual(['🛟 Mettre mes parties à l’abri']);
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

describe('🛟 Mettre mes parties à l’abri (lot S2)', () => {
  it('annonce le nombre de fichiers, la taille et le profil, puis met à l’abri après le oui et liste l’abri', async () => {
    render(BlocParties, { id: -7 });
    const b = await screen.findByRole('button', { name: /Mettre mes parties à l’abri/ });
    expect(abrisLus).toHaveBeenCalledWith(-7);
    expect(screen.queryByText('🛟 Mes abris')).toBeNull();
    listeAbris = [ABRI];
    await fireEvent.click(b);
    await new Promise((r) => setTimeout(r, 0));
    await tick();
    expect(abriApercu).toHaveBeenCalledWith(-7);
    expect(confirmer).toHaveBeenCalledTimes(1);
    const [titre, options] = confirmer.mock.calls[0] as [string, { message: string; libelleValider: string }];
    expect(titre).toBe('🛟 Mettre mes parties à l’abri ?');
    expect(options.message).toContain('2 fichiers · 7 o');
    expect(options.message).toContain('Ces parties seront rangées dans ton profil « Léa ».');
    expect(options.libelleValider).toBe('Mettre à l’abri');
    expect(abriter).toHaveBeenCalledWith(-7, false);
    // La copie se voit jusqu'à la fin (même si on quitte la page), puis se ferme.
    expect(toastProgres).toHaveBeenCalledTimes(1);
    expect(fermerToast).toHaveBeenCalledWith(42);
    expect(toast.mock.calls[0][0]).toContain('2 fichiers de parties (7 o) mis à l’abri dans ton profil');
    expect(await screen.findByText('🛟 Mes abris')).toBeTruthy();
    expect(screen.getByText('mis à l’abri à la main · dans ton profil')).toBeTruthy();
  });

  it('un non ne copie rien ; « déjà à l’abri » ne demande rien', async () => {
    reponseConfirmer = false;
    render(BlocParties, { id: -7 });
    await fireEvent.click(await screen.findByRole('button', { name: /Mettre mes parties à l’abri/ }));
    await new Promise((r) => setTimeout(r, 0));
    expect(confirmer).toHaveBeenCalledTimes(1);
    expect(abriter).not.toHaveBeenCalled();
    apercu = { ...APERCU, deja: true };
    await fireEvent.click(screen.getByRole('button', { name: /Mettre mes parties à l’abri/ }));
    await new Promise((r) => setTimeout(r, 0));
    expect(confirmer).toHaveBeenCalledTimes(1);
    expect(abriter).not.toHaveBeenCalled();
    expect(toast).toHaveBeenCalledWith('✅ Déjà à l’abri : rien n’a changé depuis ton dernier abri.', 'ok');
  });

  it('pendant la mise à l’abri, le bouton attend et un 2e appui ne fait rien', async () => {
    let fin: (v: object) => void = () => {};
    abriApercu.mockImplementationOnce(() => new Promise<object>((r) => (fin = r)));
    render(BlocParties, { id: -7 });
    const b = await screen.findByRole('button', { name: /Mettre mes parties à l’abri/ });
    await fireEvent.click(b);
    await tick();
    expect(b.getAttribute('aria-disabled')).toBe('true');
    // Pendant l'annonce : Frogtend compte, rien n'est copié avant le oui.
    expect(b.textContent?.trim()).toBe('⏳ Frogtend compte tes parties…');
    expect(toastProgres).not.toHaveBeenCalled();
    await fireEvent.click(b);
    expect(abriApercu).toHaveBeenCalledTimes(1);
    fin(structuredClone(APERCU));
    await new Promise((r) => setTimeout(r, 0));
    await tick();
    expect(b.getAttribute('aria-disabled')).toBe('false');
  });

  it('pas de bouton sans emplacement lisible avec des fichiers', async () => {
    reponse = { ...LU, emplacements: LU.emplacements.map((e) => ({ ...e, fichiers: 0, octets: 0 })) };
    render(BlocParties, { id: -7 });
    await screen.findByText(DOSSIER);
    expect(screen.queryByRole('button', { name: /Mettre mes parties à l’abri/ })).toBeNull();
  });

  it('un ancien abri Steam dit que le chemin d’origine de ses parties hors du nuage est inconnu', async () => {
    listeAbris = [{ ...ABRI, sorte: 'nuage-steam', a_moi: false, sans_origine: 3 }];
    render(BlocParties, { id: -7 });
    expect(await screen.findByText('parties d’un jeu Steam · ancien abri, sans propriétaire connu')).toBeTruthy();
    expect(screen.getByText('Frogtend ne sait plus d’où venaient 3 de ces fichiers : ils sont gardés, mais il faudra les remettre à la main.')).toBeTruthy();
  });

  it('un refus du cœur (jeu en cours) est dit tel quel', async () => {
    abriter.mockImplementationOnce(async () => Promise.reject({ sorte: 'refus', motif: 'Le jeu est en cours : quitte-le d’abord.' }));
    render(BlocParties, { id: -7 });
    await fireEvent.click(await screen.findByRole('button', { name: /Mettre mes parties à l’abri/ }));
    await new Promise((r) => setTimeout(r, 0));
    expect(toast).toHaveBeenCalledWith(expect.stringContaining('Le jeu est en cours'), 'erreur');
  });
});

describe('🛟 corrections de la revue (lot S2)', () => {
  it('« déjà à l’abri » ne cache jamais un endroit en échec ni des fichiers en ligne seulement', async () => {
    apercu = { ...APERCU, deja: true, en_ligne_seulement: 2, echecs: [{ dossier: 'C:\\X', raison: 'un fichier ne se lit pas' }] };
    render(BlocParties, { id: -7 });
    await fireEvent.click(await screen.findByRole('button', { name: /Mettre mes parties à l’abri/ }));
    await new Promise((r) => setTimeout(r, 0));
    expect(informer).toHaveBeenCalledTimes(1);
    const [titre, message] = informer.mock.calls[0] as [string, string];
    expect(titre).toBe('✅ Déjà à l’abri');
    expect(message).toContain('2 fichiers sont seulement dans ton nuage');
    expect(message).toContain('un fichier ne se lit pas');
    expect(abriter).not.toHaveBeenCalled();
  });

  it('devenue très grosse entre l’annonce et la copie : annoncée de nouveau, jamais copiée sans le dire', async () => {
    abriter.mockImplementationOnce(async () => Promise.reject({ sorte: 'refus', motif: "Ces parties sont très grosses : confirme d'abord." }));
    render(BlocParties, { id: -7 });
    await fireEvent.click(await screen.findByRole('button', { name: /Mettre mes parties à l’abri/ }));
    for (let i = 0; i < 5; i++) await new Promise((r) => setTimeout(r, 0));
    expect(abriApercu).toHaveBeenCalledTimes(2);
    expect(confirmer).toHaveBeenCalledTimes(2);
    expect(abriter).toHaveBeenCalledTimes(2);
    expect(fermerToast).toHaveBeenCalledTimes(2);
    expect(toast.mock.calls.some((c) => String(c[1]) === 'erreur')).toBe(false);
  });

  it('deux abris de la même date et de la même sorte : tous les deux montrés', async () => {
    listeAbris = [ABRI, { ...ABRI }];
    render(BlocParties, { id: -7 });
    expect((await screen.findAllByText('mis à l’abri à la main · dans ton profil')).length).toBe(2);
  });
});
