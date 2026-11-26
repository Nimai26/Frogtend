// 0.229.1 (MK8 Deluxe, Seb 26/11) : « Annuler » au choix du lanceur défait l'installation tout juste notée, et
// « 📦 Refaire l'installation » n'est proposé que pour une installation qui n'est qu'une note.
import { describe, expect, it, vi } from 'vitest';
import type { Installation } from '$lib/api';
import { choisirOuDefaire, noteSeule, OPTION_REFAIRE, refaire, refaireAVerifier } from './refaireInstallation';

/** L'état EXACT de Seb le 26/11 (jeux.db, table jeux_pc). */
const deSeb = (): Installation => ({
  dossier: 'D:\\Jeux\\Switch\\Mario Kart 8 Deluxe (2017)',
  methode: { sorte: 'aucune' },
  lanceur: null,
  fichier_du_jeu: null,
  installe_le: '1795699748',
});

describe('🧹 Annuler le choix du lanceur', () => {
  it('« Annuler » (faux) sur une installation tout juste notée : l’annulation est appelée', async () => {
    const annuler = vi.fn(async (_id: number) => {});
    const r = await choisirOuDefaire(24188, { choisirLanceur: async () => false, installationDe: () => deSeb(), annuler });
    expect(r).toBe(false);
    expect(annuler).toHaveBeenCalledWith(24188);
  });

  it('un lanceur choisi : rien n’est défait', async () => {
    const annuler = vi.fn(async (_id: number) => {});
    const r = await choisirOuDefaire(1, { choisirLanceur: async () => true, installationDe: () => deSeb(), annuler });
    expect(r).toBe(true);
    expect(annuler).not.toHaveBeenCalled();
  });

  it('une installation qui a mis le jeu en place (installeur, archive, lanceur, image disque) : jamais défaite', async () => {
    const annuler = vi.fn(async (_id: number) => {});
    const autres: Installation[] = [
      { ...deSeb(), methode: { sorte: 'archive', fichier: 'j.zip', format: 'zip' } },
      { ...deSeb(), lanceur: { programme: 'C:\\j.exe', arguments: [], dossier: 'C:\\' } },
      { ...deSeb(), fichier_du_jeu: 'MK8.xci' },
      { ...deSeb(), boutique: { source: 'steam', cle: '1' } },
      { ...deSeb(), extraits: ['Disque 1.bin'] },
    ];
    for (const i of autres) await choisirOuDefaire(1, { choisirLanceur: async () => false, installationDe: () => i, annuler });
    await choisirOuDefaire(1, { choisirLanceur: async () => false, installationDe: () => null, annuler });
    expect(annuler).not.toHaveBeenCalled();
  });

  it('un refus du cœur ne casse rien', async () => {
    const r = await choisirOuDefaire(1, {
      choisirLanceur: async () => false,
      installationDe: () => deSeb(),
      annuler: async () => {
        throw new Error('refus');
      },
    });
    expect(r).toBe(false);
  });
});

describe('📦 Refaire l’installation', () => {
  it('demandé au cœur seulement pour un jeu de Firehouse téléchargé dont l’installation n’est qu’une note', () => {
    expect(refaireAVerifier(24188, { etat: 'telecharge', installation: deSeb() })).toBe(true);
    expect(refaireAVerifier(24188, { etat: 'telecharge', installation: null })).toBe(false);
    expect(refaireAVerifier(24188, { etat: 'telecharge', installation: { ...deSeb(), fichier_du_jeu: 'MK8.xci' } })).toBe(false);
    expect(refaireAVerifier(24188, { etat: 'telecharge', installation: { ...deSeb(), methode: { sorte: 'installeur_guide', fichier: 'setup.exe' } } })).toBe(false);
    expect(refaireAVerifier(24188, { etat: 'en_cours', installation: deSeb() })).toBe(false);
    expect(refaireAVerifier(-3, { etat: 'telecharge', installation: deSeb() })).toBe(false);
    expect(refaireAVerifier(24188, undefined)).toBe(false);
    expect(noteSeule(deSeb())).toBe(true);
  });

  it('le libellé dit que rien n’est retéléchargé', () => {
    expect(OPTION_REFAIRE.libelle).toBe('📦 Refaire l’installation (rien n’est retéléchargé)');
  });

  it('défait la note, relit, puis relance l’installation normale, dans cet ordre', async () => {
    const ordre: string[] = [];
    const r = await refaire(24188, {
      annuler: async (id) => void ordre.push(`annuler ${id}`),
      recharger: async () => void ordre.push('relire'),
      installer: async (id) => (ordre.push(`installer ${id}`), true),
    });
    expect(r).toBe(true);
    expect(ordre).toEqual(['annuler 24188', 'relire', 'installer 24188']);
  });

  it('un refus de l’annulation n’installe rien', async () => {
    const installer = vi.fn(async () => true);
    await expect(refaire(1, { annuler: async () => Promise.reject(new Error('refus')), recharger: async () => {}, installer })).rejects.toThrow();
    expect(installer).not.toHaveBeenCalled();
  });
});
