import { describe, expect, it } from 'vitest';
import { migrerPc, migrerProfil } from './reunions';

// Le relevé réel du 18/10 : la fiche 1927 a été réunie à 324.
const R: [number, number][] = [[1927, 324]];

describe('les réglages d’un jeu suivent sa nouvelle fiche', () => {
  it('l’émulateur choisi pour le jeu est copié sous le nouveau numéro, l’ancien gardé', () => {
    const pc = { emulateursJeux: { '1927': '2', '10': '1' }, reunionsSuivies: [], autre: true };
    expect(migrerPc(pc, R)).toEqual({ emulateursJeux: { '1927': '2', '10': '1', '324': '2' }, reunionsSuivies: ['1927>324'], autre: true });
  });

  it('un réglage déjà fait sous le nouveau numéro prime', () => {
    expect(migrerPc({ emulateursJeux: { '1927': '2', '324': '3' }, reunionsSuivies: [] }, R)?.emulateursJeux).toEqual({ '1927': '2', '324': '3' });
  });

  it('une réunion n’est suivie qu’une fois : un réglage effacé exprès ne revient pas', () => {
    const pc = migrerPc({ emulateursJeux: { '1927': '2' }, reunionsSuivies: [] as string[] }, R)!;
    const efface = { ...pc, emulateursJeux: { '1927': '2' } };
    expect(migrerPc(efface, R)).toBeNull();
    expect(migrerPc({ emulateursJeux: {}, reunionsSuivies: [] }, R)?.reunionsSuivies).toEqual(['1927>324']);
  });

  it('commandes, plein écran, clavier et gel du profil suivent aussi', () => {
    const p = { commandes: { '1927': { a: 1 } }, pleinEcran: { '1927': 'F11' }, clavier: {}, figer: ['1927'], sansRepetition: ['1927'], reunionsSuivies: [], skin: 'x' };
    const m = migrerProfil(p, R)!;
    expect(m.commandes).toEqual({ '1927': { a: 1 }, '324': { a: 1 } });
    expect(m.pleinEcran).toEqual({ '1927': 'F11', '324': 'F11' });
    expect(m.clavier).toEqual({});
    expect(m.figer).toEqual(['1927', '324']);
    expect(m.sansRepetition).toEqual(['1927', '324']);
    expect(m.skin).toBe('x');
    expect(migrerProfil(m, R)).toBeNull();
  });
});
