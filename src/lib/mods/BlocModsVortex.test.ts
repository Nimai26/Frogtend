// 🧩 Le bloc des mods de Vortex (lot M1, ✅ Seb 25/11) : invisible sans rien à dire, fermé d'office au bureau, le
// diagnostic puis les mods en cause, un retrait confirmé (danger) précédé de l'abri des parties, la liste par 25, la
// remise mod par mod, la reprise d'un retrait interrompu, et verrouillé quand Galaxy est ouvert.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';
import type { EtatVortex, ModVortex } from '$lib/api';

const m = (nom: string, fichiers = 7, source = `${nom}-1-1-1700000000`): ModVortex => ({ source, nom, fichiers });
const MODS = [m('ArchiveXL', 12, 'ArchiveXL-4198-1-21-1-1737797101'), m('CET 1.35.0 - Patch 2.21', 9, 'CET-107'), ...Array.from({ length: 282 }, (_, i) => m(`Mod ${String(i).padStart(3, '0')}`))];
function etatVortex(x: Partial<EtatVortex> = {}): EtatVortex {
  return {
    present: true,
    refus: null,
    mods: MODS,
    fichiers: 2015,
    octets: 4.1 * 1024 ** 3,
    retrait: null,
    retires: [],
    remis: [],
    diagnostic: {
      version: '2.50',
      version_precedente: '2.21',
      refus: [{ outil: 'Cyber Engine Tweaks', version_attendue: '2.21' }],
      plante: { chargeur: 'RED4ext', greffon: 'ArchiveXL' },
      erreurs_scripts: 0,
      outil_scripts: null,
    },
    en_cause: ['ArchiveXL-4198-1-21-1-1737797101', 'CET-107'],
    verrou: null,
    ...x,
  };
}

let reponse: EtatVortex | null = etatVortex();
const lire = vi.fn(async (..._a: unknown[]) => structuredClone(reponse));
const retirer = vi.fn(async (..._a: unknown[]) => ({ deplaces: 2014, laisses: 1, laisses_rompus: 1, mods: 284, octets: 1, dossiers_retires: 449, sauvegardes_vortex: 1, deploiement_range: true }));
const reprendre = vi.fn(async (..._a: unknown[]) => ({ deplaces: 2014, laisses: 1, laisses_rompus: 1, mods: 284, octets: 1, dossiers_retires: 449, sauvegardes_vortex: 1, deploiement_range: true }));
const remettre = vi.fn(async (..._a: unknown[]) => ({ remis: 12, places_prises: 0, changes: 0, deploiement_remis: false }));
const toutRemettre = vi.fn(async (..._a: unknown[]) => ({ remis: 2014, places_prises: 0, changes: 0, deploiement_remis: true }));
const retirerMod = vi.fn(async (..._a: unknown[]) => ({ ranges: 3, changes: 0, partis: 0, places_prises: 0, erreurs: 0, coinces: 0, mods: 1 }));
const toutRanger = vi.fn(async (..._a: unknown[]) => ({ ranges: 5, changes: 0, partis: 0, places_prises: 0, erreurs: 0, coinces: 0, mods: 2 }));
vi.mock('$lib/api', () => ({
  taille: (o: number) => `${(o / 1024 ** 3).toLocaleString('fr-FR', { maximumFractionDigits: 1 })} Go`,
  estErreurCoeur: (e: unknown) => typeof e === 'object' && e !== null && 'sorte' in e,
  api: {
    modsVortexEtat: (...a: unknown[]) => lire(...a),
    modsVortexRetirer: (...a: unknown[]) => retirer(...a),
    modsVortexReprendre: (...a: unknown[]) => reprendre(...a),
    modsVortexRemettre: (...a: unknown[]) => remettre(...a),
    modsVortexToutRemettre: (...a: unknown[]) => toutRemettre(...a),
    modsVortexRetirerMod: (...a: unknown[]) => retirerMod(...a),
    modsVortexToutRanger: (...a: unknown[]) => toutRanger(...a),
  },
}));
const confirmer = vi.fn(async (..._a: unknown[]) => true);
const toast = vi.fn();
vi.mock('$lib/dialogues/fenetres.svelte', () => ({
  confirmer: (...a: unknown[]) => confirmer(...a),
  toast: (...a: unknown[]) => toast(...a),
}));
/** L'ordre des étapes : l'abri d'abord, puis la progression, puis l'opération. */
const etapes: string[] = [];
const abri = vi.fn(async (_id: number, op: string, faire: () => Promise<unknown>, o: { jeuPc?: boolean }) => {
  etapes.push(`abri ${op} ${o.jeuPc}`);
  return faire();
});
vi.mock('$lib/parties/abriAvant', () => ({ avecAbriAvant: (...a: unknown[]) => abri(...(a as Parameters<typeof abri>)) }));
/** Le départ de la progression donné à `avecProgres` (0.227.1 : montré avant le premier événement du cœur). */
const departs: unknown[] = [];
vi.mock('./progres.svelte', () => ({
  avecProgres: async (message: string, _approx: boolean, travail: (s: number) => Promise<unknown>, depart?: unknown) => {
    etapes.push(`progres ${message}`);
    departs.push(depart);
    return travail(42);
  },
}));
let enPartie = false;
vi.mock('$lib/ludotheque/jeu.svelte', () => ({ lancementImpossible: () => enPartie }));

import BlocModsVortex from './BlocModsVortex.svelte';

async function attendre() {
  for (let i = 0; i < 8; i++) {
    await tick();
    await Promise.resolve();
  }
}

async function ouvrir(props: { deplie?: boolean } = {}) {
  render(BlocModsVortex, { id: 7, titre: 'Cyberpunk 2077', ...props });
  const entete = await screen.findByRole('button', { name: /Mods :/ });
  if (!props.deplie) await fireEvent.click(entete);
  await attendre();
  return entete;
}

const retire = (n: number) => Array.from({ length: n }, (_, i) => m(`Retiré ${String(i).padStart(3, '0')}`, 3));

beforeEach(() => {
  reponse = etatVortex();
  enPartie = false;
  etapes.length = 0;
  departs.length = 0;
  for (const f of [lire, retirer, reprendre, remettre, toutRemettre, retirerMod, toutRanger, confirmer, toast, abri]) f.mockClear();
  lire.mockImplementation(async () => structuredClone(reponse));
  confirmer.mockImplementation(async () => true);
  document.documentElement.removeAttribute('data-taodbox');
});
afterEach(() => cleanup());
// jsdom ne fait pas défiler : `selectionner` appelle scrollIntoView.
Element.prototype.scrollIntoView = () => {};

describe('le bloc des mods de Vortex', () => {
  it('ne se montre pas sans Vortex', async () => {
    reponse = etatVortex({ present: false });
    render(BlocModsVortex, { id: 7, titre: 'X' });
    await attendre();
    expect(lire).toHaveBeenCalledWith(7);
    expect(screen.queryByRole('button', { name: /Mods :/ })).toBeNull();
    reponse = null;
    cleanup();
    render(BlocModsVortex, { id: 8, titre: 'X' });
    await attendre();
    expect(screen.queryByRole('button', { name: /Mods :/ })).toBeNull();
  });

  it('fermé d’office au bureau, l’en-tête dit tout ; à l’ouverture : le diagnostic, puis les mods en cause', async () => {
    render(BlocModsVortex, { id: 7, titre: 'Cyberpunk 2077' });
    const entete = await screen.findByRole('button', { name: /Mods :/ });
    expect(entete.tagName).toBe('BUTTON');
    expect(entete.getAttribute('aria-expanded')).toBe('false');
    expect(entete.textContent).toContain('🧩 Mods : 284 mods posés par Vortex · ⚠ le jeu ne démarre plus');
    expect(screen.queryByText(/Retirer les mods/)).toBeNull();
    expect(document.querySelector('details')).toBeNull();
    await fireEvent.click(entete);
    await attendre();
    expect(entete.getAttribute('aria-expanded')).toBe('true');
    const texte = document.body.textContent ?? '';
    expect(texte).toContain('⚠ Cyberpunk 2077 a été mis à jour (version 2.21 → 2.50) et certains mods ne suivent plus : le jeu ne démarre plus.');
    expect(texte).toContain('« Cyber Engine Tweaks » ne marche pas avec cette version du jeu.');
    expect(texte).toContain('« RED4ext » s’arrête en chargeant « ArchiveXL ».');
    const lignes = [...document.querySelectorAll<HTMLElement>('li[data-cle]')];
    expect(lignes.map((l) => l.querySelector('.nom')?.textContent)).toEqual(['ArchiveXL', 'CET 1.35.0 - Patch 2.21']);
    expect(lignes.every((l) => l.getAttribute('tabindex') === '-1')).toBe(true);
    expect(screen.getByRole('button', { name: '🧹 Retirer les mods de Vortex (284)' })).toBeTruthy();
    expect(texte).not.toMatch(/lien dur|manifeste|deployment|D:\\/i);
  });

  it('les autres mods par 25, la recherche au bureau avec son compteur', async () => {
    await ouvrir();
    const voir = screen.getByRole('button', { name: 'Voir les 282 autres mods' });
    expect(voir.getAttribute('aria-expanded')).toBe('false');
    await fireEvent.click(voir);
    await attendre();
    expect(document.querySelectorAll('li[data-cle]')).toHaveLength(2 + 25);
    await fireEvent.click(screen.getByRole('button', { name: 'Voir 25 de plus' }));
    await attendre();
    expect(document.querySelectorAll('li[data-cle]')).toHaveLength(2 + 50);
    const champ = screen.getByRole('searchbox', { name: 'Chercher un mod' }) as HTMLInputElement;
    expect(champ.placeholder).toBe('🔎 Chercher…');
    // La recherche porte sur les autres mods : un mod en cause n'y revient jamais en double.
    await fireEvent.input(champ, { target: { value: 'archive' } });
    await attendre();
    expect(screen.getByText(/Aucun mod ne correspond/)).toBeTruthy();
    expect(document.querySelectorAll('li[data-cle="ArchiveXL-4198-1-21-1-1737797101"]')).toHaveLength(1);
    await fireEvent.input(champ, { target: { value: 'mod 281' } });
    await attendre();
    expect(screen.getByText('1 sur 282')).toBeTruthy();
  });

  it('au canapé : les lignes des mods se visent à la manette (tabindex 0)', async () => {
    document.documentElement.setAttribute('data-taodbox', '');
    await ouvrir({ deplie: true });
    const lignes = [...document.querySelectorAll<HTMLElement>('li[data-cle]')];
    expect(lignes.length).toBeGreaterThan(0);
    expect(lignes.every((l) => l.getAttribute('tabindex') === '0')).toBe(true);
  });

  it('rien n’est verrouillé avant le oui ; après « Annuler », le focus revient', async () => {
    let repondre: (v: boolean) => void = () => {};
    confirmer.mockImplementation(() => new Promise<boolean>((r) => (repondre = r)));
    await ouvrir();
    const b = screen.getByRole('button', { name: /Retirer les mods de Vortex/ }) as HTMLButtonElement;
    b.focus();
    await fireEvent.click(b);
    await attendre();
    expect(b.disabled).toBe(false);
    expect(document.body.textContent).not.toContain('Une opération est en cours');
    repondre(false);
    await attendre();
    expect(abri).not.toHaveBeenCalled();
    expect(document.activeElement?.isConnected).toBe(true);
    expect(document.activeElement).not.toBe(document.body);
  });

  it('au canapé : déplié, sans recherche', async () => {
    document.documentElement.setAttribute('data-taodbox', '');
    const entete = await ouvrir({ deplie: true });
    expect(entete.getAttribute('aria-expanded')).toBe('true');
    await fireEvent.click(screen.getByRole('button', { name: /autres mods/ }));
    await attendre();
    expect(screen.queryByRole('searchbox')).toBeNull();
  });

  it('retirer : confirmation « danger », abri des parties AVANT, progression, puis l’état relu', async () => {
    await ouvrir();
    reponse = etatVortex({ present: false, mods: [], retires: retire(284), retrait: { en_cours: false, faits: 2015, total: 2015, laisses: 1, laisses_rompus: 1, sauvegardes_vortex: 1 } });
    await fireEvent.click(screen.getByRole('button', { name: '🧹 Retirer les mods de Vortex (284)' }));
    await attendre();
    const [titre, o] = confirmer.mock.calls[0] as [string, { message: string; danger: boolean; libelleValider: string }];
    expect(titre).toBe('🧹 Retirer les 284 mods de Vortex ?');
    expect(o.danger).toBe(true);
    expect(o.libelleValider).toBe('🧹 Retirer les mods');
    expect(o.message).toContain(`les ${(2015).toLocaleString('fr-FR')} fichiers (4,1 Go)`);
    expect(o.message).toContain('Aucun fichier de mod n’est effacé ni copié');
    expect(o.message).toContain('désinstalle Vortex');
    expect(etapes).toEqual(['abri avant_mod true', 'progres 🧹 Retrait des mods de Vortex (ne ferme pas Frogtend)']);
    expect(retirer).toHaveBeenCalledWith(7, 42);
    // 0.227.1 : la barre dit tout de suite l'étape et le total (les fichiers des mods listés : 12 + 9 + 282 × 7).
    expect(departs).toEqual([{ etape: 'verification', total: 12 + 9 + 282 * 7 }]);
    expect(toast.mock.calls[0][0]).toBe(
      '✅ 284 mods retirés. 1 fichier laissé tel quel : il ne menait plus nulle part. Désinstalle maintenant Vortex (Paramètres de Windows ▸ Applications ▸ Vortex ▸ Désinstaller) : s’il est rouvert, il remet ses mods.',
    );
    expect(lire).toHaveBeenCalledTimes(2);
    const texte = document.body.textContent ?? '';
    // B1 (relecture UI) : plus aucune alerte ni ligne de diagnostic d'avant le retrait.
    expect(screen.getByRole('button', { name: /Mods :/ }).textContent).not.toContain('ne démarre plus');
    expect(texte).not.toContain('ne marche pas avec cette version');
    expect(texte).not.toContain('s’arrête en chargeant');
    expect(texte).toContain('Désinstalle maintenant Vortex');
    expect(texte).toContain('Vortex avait gardé à côté l’original d’un fichier du jeu');
    expect(texte).toContain('✅ 284 mods retirés.');
    expect(texte).toContain('Les réglages que tes mods avaient écrits restent dans le jeu');
    expect(texte).toContain('⚠ Tes parties faites avec des mods peuvent en avoir besoin');
    expect(texte).toContain('🧩 Mods retirés (284)');
    expect(screen.getAllByRole('button', { name: /^Remettre / })).toHaveLength(25);
  });

  it('un non à la confirmation : rien ne part', async () => {
    confirmer.mockImplementation(async () => false);
    await ouvrir();
    await fireEvent.click(screen.getByRole('button', { name: /Retirer les mods de Vortex/ }));
    await attendre();
    expect(abri).not.toHaveBeenCalled();
    expect(retirer).not.toHaveBeenCalled();
  });

  it('remettre un mod : confirmation sans danger, avec la version du jeu', async () => {
    reponse = etatVortex({ present: false, mods: [], retires: [m('ArchiveXL', 12, 'ArchiveXL-4198-1-21-1-1737797101')], retrait: { en_cours: false, faits: 2015, total: 2015, laisses: 1, laisses_rompus: 1, sauvegardes_vortex: 0 } });
    await ouvrir();
    await fireEvent.click(screen.getByRole('button', { name: 'Remettre ArchiveXL' }));
    await attendre();
    const [titre, o] = confirmer.mock.calls[0] as [string, { message: string; danger?: boolean }];
    expect(titre).toBe('↩ Remettre « ArchiveXL » ?');
    expect(o.danger).toBeFalsy();
    expect(o.message).toBe('« ArchiveXL » revient dans le jeu (12 fichiers). Il ne marche peut-être pas avec la version 2.50.');
    expect(remettre).toHaveBeenCalledWith(7, 'ArchiveXL-4198-1-21-1-1737797101', 42);
    expect(departs).toEqual([{ etape: 'remise', total: 12 }]);
    expect(etapes[0]).toBe('abri avant_mod true');
    expect(lire).toHaveBeenCalledTimes(2);
  });

  it('un retrait interrompu : reprendre (focus d’office) ou tout remettre', async () => {
    reponse = etatVortex({ retrait: { en_cours: true, faits: 812, total: 2015, laisses: 0, laisses_rompus: 0, sauvegardes_vortex: 0 } });
    document.documentElement.setAttribute('data-taodbox', '');
    await ouvrir({ deplie: true });
    expect(document.body.textContent).toContain(`⚠ Le retrait des mods a été interrompu (812 fichiers sur ${(2015).toLocaleString('fr-FR')})`);
    const r = screen.getByRole('button', { name: '▶ Reprendre le retrait' });
    expect(document.activeElement).toBe(r);
    expect(screen.queryByRole('button', { name: /Retirer les mods de Vortex/ })).toBeNull();
    await fireEvent.click(r);
    await attendre();
    expect(reprendre).toHaveBeenCalledWith(7, 42);
    cleanup();
    reponse = etatVortex({ retrait: { en_cours: true, faits: 812, total: 2015, laisses: 0, laisses_rompus: 0, sauvegardes_vortex: 0 } });
    await ouvrir({ deplie: true });
    await fireEvent.click(screen.getByRole('button', { name: '↩ Tout remettre comme avant' }));
    await attendre();
    expect(toutRemettre).toHaveBeenCalledWith(7, 42);
  });

  it('verrouillé quand GOG Galaxy est ouvert, ou pendant une partie', async () => {
    reponse = etatVortex({ verrou: 'galaxy' });
    await ouvrir();
    expect(document.body.textContent).toContain('⛔ Ferme GOG Galaxy d’abord');
    expect((screen.getByRole('button', { name: /Retirer les mods de Vortex/ }) as HTMLButtonElement).disabled).toBe(true);
    // « C'est fait, revérifier » encore trop tôt : il le dit, le focus reste sur le bouton.
    await fireEvent.click(screen.getByRole('button', { name: /revérifier/ }));
    await attendre();
    expect(toast).toHaveBeenCalledWith(expect.stringMatching(/^GOG Galaxy est encore ouvert/), 'alerte');
    expect(document.activeElement).toBe(screen.getByRole('button', { name: /revérifier/ }));
    // Puis fermé : l'état relu, le bloc se libère et le focus va au bouton de retrait.
    reponse = etatVortex();
    await fireEvent.click(screen.getByRole('button', { name: /revérifier/ }));
    await attendre();
    const retrait = screen.getByRole('button', { name: /Retirer les mods de Vortex/ }) as HTMLButtonElement;
    expect(retrait.disabled).toBe(false);
    expect(document.activeElement).toBe(retrait);
    cleanup();
    // Vortex ouvert (I1), et le jeu lancé hors de Frogtend (verrou du cœur) : le même bouton.
    for (const [v, texte] of [
      ['vortex', '⛔ Ferme Vortex d’abord'],
      ['partie', '⛔ Le jeu est en cours, ou Frogtend y travaille : attends la fin.'],
    ] as const) {
      reponse = etatVortex({ verrou: v });
      await ouvrir();
      expect(document.body.textContent).toContain(texte);
      expect(screen.getByRole('button', { name: /revérifier/ })).toBeTruthy();
      cleanup();
    }
    // Une partie lancée par Frogtend : verrouillé aussi (sans bouton : la fin de partie relit l'état).
    enPartie = true;
    reponse = etatVortex();
    await ouvrir();
    expect(document.body.textContent).toContain('attends la fin');
    expect((screen.getByRole('button', { name: /Retirer les mods de Vortex/ }) as HTMLButtonElement).disabled).toBe(true);
  });

  // --- 0.227.1 (retours de Seb du 25/11) ---

  const APRES = { en_cours: false, faits: 2015, total: 2015, laisses: 1, laisses_rompus: 1, sauvegardes_vortex: 0 };

  it('un mod REMIS apparaît dans « 🧩 Dans le jeu » et se retire de nouveau, par le même enchaînement', async () => {
    const bh = m('(4k) Immersive Bullet Holes', 3, 'BH-15309');
    reponse = etatVortex({ present: false, mods: [], retires: retire(283), remis: [bh], retrait: APRES });
    const entete = await ouvrir();
    expect(entete.textContent).toContain('283 mods retirés de Vortex · 1 mod remis dans le jeu');
    expect(document.body.textContent).toContain('🧩 Dans le jeu (1)');
    const ligne = document.querySelector<HTMLElement>('li[data-cle="jeu:BH-15309"]')!;
    expect(ligne.querySelector('.nom')?.textContent).toBe('(4k) Immersive Bullet Holes');
    expect(ligne.textContent).toContain('3 fichiers');
    const b = screen.getByRole('button', { name: 'Retirer (4k) Immersive Bullet Holes' });
    reponse = etatVortex({ present: false, mods: [], retires: [...retire(283), bh], remis: [], retrait: APRES });
    b.focus();
    await fireEvent.click(b);
    await attendre();
    const [titre, o] = confirmer.mock.calls[0] as [string, { message: string; danger: boolean; libelleValider: string }];
    expect(titre).toBe('🧹 Retirer « (4k) Immersive Bullet Holes » ?');
    expect(o.danger).toBe(true);
    expect(o.message).toContain('Rien n’est effacé');
    expect(etapes).toEqual(['abri avant_mod true', 'progres 🧹 Retrait de « (4k) Immersive Bullet Holes » (ne ferme pas Frogtend)']);
    expect(departs).toEqual([{ etape: 'retrait', total: 3 }]);
    expect(retirerMod).toHaveBeenCalledWith(7, 'BH-15309', 42);
    expect(toast).toHaveBeenCalledWith('✅ « (4k) Immersive Bullet Holes » retiré du jeu (3 fichiers rangés).', 'ok');
    expect(lire).toHaveBeenCalledTimes(2);
    expect(document.body.textContent).not.toContain('🧩 Dans le jeu');
    // Le focus n'est jamais perdu en haut de la page.
    expect(document.activeElement?.isConnected).toBe(true);
    expect(document.activeElement).not.toBe(document.body);
  });

  it('un « non » au retrait d’un mod remis : rien ne part', async () => {
    confirmer.mockImplementation(async () => false);
    reponse = etatVortex({ present: false, mods: [], retires: [], remis: [m('A', 2, 'A-1')], retrait: APRES });
    await ouvrir();
    await fireEvent.click(screen.getByRole('button', { name: 'Retirer A' }));
    await attendre();
    expect(abri).not.toHaveBeenCalled();
    expect(retirerMod).not.toHaveBeenCalled();
  });

  it('Galaxy ouvert après le retrait : le motif et « revérifier » UNE fois, collés aux listes, et chaque bouton grisé le désigne', async () => {
    reponse = etatVortex({ present: false, mods: [], retires: retire(30), remis: [m('A', 2, 'A-1')], retrait: APRES, verrou: 'galaxy' });
    await ouvrir();
    const motifs = [...document.querySelectorAll<HTMLElement>('p')].filter((p) => p.textContent?.startsWith('⛔ Ferme GOG Galaxy'));
    expect(motifs).toHaveLength(1);
    expect(screen.getAllByRole('button', { name: /revérifier/ })).toHaveLength(1);
    // Juste au-dessus des listes : après le compte rendu du retrait, avant « Dans le jeu ».
    const motif = motifs[0];
    const titre = [...document.querySelectorAll('p.sous-titre')].find((p) => p.textContent?.startsWith('🧩 Dans le jeu'))!;
    expect(motif.compareDocumentPosition(titre) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    const compteRendu = [...document.querySelectorAll('p')].find((p) => p.textContent?.startsWith('✅ 30 mods retirés'))!;
    expect(compteRendu.compareDocumentPosition(motif) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    const grises = [...document.querySelectorAll<HTMLButtonElement>('button')].filter((b) => b.disabled);
    // Retirer (Dans le jeu), Tout ranger de nouveau, 25 Remettre, Tout remettre.
    expect(grises.length).toBe(1 + 1 + 25 + 1);
    expect(motif.id).not.toBe('');
    expect(grises.every((b) => b.getAttribute('aria-describedby') === motif.id)).toBe(true);
    // Déverrouillé : plus de motif, plus de description ; le focus va au premier bouton actif des listes.
    reponse = etatVortex({ present: false, mods: [], retires: retire(30), remis: [m('A', 2, 'A-1')], retrait: APRES });
    await fireEvent.click(screen.getByRole('button', { name: /revérifier/ }));
    await attendre();
    expect(document.body.textContent).not.toContain('⛔');
    expect(document.querySelectorAll('[aria-describedby]')).toHaveLength(0);
    expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Retirer A' }));
  });

  it('« Dans le jeu » par paquets de 25, avec la recherche au bureau ; au canapé, chaque ligne se vise même tout grisé', async () => {
    const beaucoup = Array.from({ length: 30 }, (_, i) => m(`Remis ${String(i).padStart(2, '0')}`, 2));
    reponse = etatVortex({ present: false, mods: [], retires: retire(3), remis: beaucoup, retrait: APRES });
    await ouvrir();
    expect(document.querySelectorAll('li[data-cle^="jeu:"]')).toHaveLength(25);
    await fireEvent.click(screen.getByRole('button', { name: 'Voir 5 de plus' }));
    await attendre();
    expect(document.querySelectorAll('li[data-cle^="jeu:"]')).toHaveLength(30);
    const champ = screen.getByRole('searchbox', { name: 'Chercher un mod remis' });
    await fireEvent.input(champ, { target: { value: 'remis 07' } });
    await attendre();
    expect(document.querySelectorAll('li[data-cle^="jeu:"]')).toHaveLength(1);
    expect(screen.getByText('1 sur 30')).toBeTruthy();
    expect([...document.querySelectorAll('li[data-cle^="jeu:"], li[data-cle^="retire:"]')].every((l) => l.getAttribute('tabindex') === '-1')).toBe(true);
    cleanup();
    document.documentElement.setAttribute('data-taodbox', '');
    reponse = etatVortex({ present: false, mods: [], retires: retire(3), remis: [m('A', 2, 'A-1')], retrait: APRES, verrou: 'galaxy' });
    await ouvrir({ deplie: true });
    expect(screen.queryByRole('searchbox')).toBeNull();
    const lignes = [...document.querySelectorAll('li[data-cle^="jeu:"], li[data-cle^="retire:"]')];
    expect(lignes).toHaveLength(4);
    expect(lignes.every((l) => l.getAttribute('tabindex') === '0')).toBe(true);
  });

  it('sans manifeste de Vortex : « 🧹 Tout ranger de nouveau » range tous les mods remis, par le même enchaînement', async () => {
    reponse = etatVortex({ present: false, mods: [], retires: [], remis: [m('A', 2, 'A-1'), m('B', 3, 'B-1')], retrait: APRES });
    await ouvrir();
    const b = screen.getByRole('button', { name: '🧹 Tout ranger de nouveau' });
    await fireEvent.click(b);
    await attendre();
    const [titre, o] = confirmer.mock.calls[0] as [string, { message: string; danger: boolean }];
    expect(titre).toBe('🧹 Ranger de nouveau les 2 mods remis ?');
    expect(o.danger).toBe(true);
    expect(etapes).toEqual(['abri avant_mod true', 'progres 🧹 Rangement des mods remis (ne ferme pas Frogtend)']);
    expect(departs).toEqual([{ etape: 'retrait', total: 5 }]);
    expect(toutRanger).toHaveBeenCalledWith(7, 42);
    expect(toast).toHaveBeenCalledWith('✅ 2 mods rangés de nouveau (5 fichiers).', 'ok');
    cleanup();
    // Vortex encore là (son manifeste) : pas ce bouton, le retrait global suffit.
    reponse = etatVortex({ remis: [m('A', 2, 'A-1')] });
    await ouvrir();
    expect(screen.queryByRole('button', { name: '🧹 Tout ranger de nouveau' })).toBeNull();
  });

  it('un retrait partiel d’un mod remis se dit « en partie », en alerte', async () => {
    retirerMod.mockImplementationOnce(async () => ({ ranges: 1, changes: 1, partis: 0, places_prises: 0, erreurs: 0, coinces: 0, mods: 1 }));
    reponse = etatVortex({ present: false, mods: [], retires: [], remis: [m('A', 2, 'A-1')], retrait: APRES });
    await ouvrir();
    await fireEvent.click(screen.getByRole('button', { name: 'Retirer A' }));
    await attendre();
    expect(toast).toHaveBeenCalledWith(expect.stringMatching(/^« A » est en partie retiré \(1 fichier rangé\)/), 'alerte');
  });

  it('la reprise d’un retrait interrompu démarre la barre au vrai compte', async () => {
    reponse = etatVortex({ retrait: { en_cours: true, faits: 812, total: 2015, laisses: 1, laisses_rompus: 1, sauvegardes_vortex: 0 } });
    await ouvrir();
    await fireEvent.click(screen.getByRole('button', { name: '▶ Reprendre le retrait' }));
    await attendre();
    expect(departs).toEqual([{ etape: 'retrait', total: 2014, faits: 811 }]);
  });

  it('Galaxy ouvert avant le retrait : le motif reste en tête, près du bouton de retrait, une seule fois', async () => {
    reponse = etatVortex({ verrou: 'galaxy' });
    await ouvrir();
    const motifs = [...document.querySelectorAll<HTMLElement>('p')].filter((p) => p.textContent?.startsWith('⛔'));
    expect(motifs).toHaveLength(1);
    const b = screen.getByRole('button', { name: /Retirer les mods de Vortex/ });
    expect(b.getAttribute('aria-describedby')).toBe(motifs[0].id);
  });

  it('une méthode de Vortex refusée : le motif, jamais le bouton', async () => {
    reponse = etatVortex({ refus: 'Vortex a posé ces mods par des raccourcis vers son dossier de mods : Frogtend ne sait retirer avec certitude que ceux posés de la façon habituelle de Vortex. Rien n’est touché.' });
    await ouvrir();
    expect(document.body.textContent).toContain('par des raccourcis');
    expect(screen.queryByRole('button', { name: /Retirer les mods de Vortex/ })).toBeNull();
  });
});
