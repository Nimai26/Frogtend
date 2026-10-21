import { describe, expect, it } from 'vitest';
import { groupeDe, libelleCopie, nomOrigine, originesCarte, originesJeuPc, unParGroupe } from './copies';

describe('Un jeu sur plusieurs boutiques = une carte (A6c)', () => {
  const groupes = [[-1, -2], [-5, -6, -7]];
  it('retrouve le groupe d’une copie', () => {
    expect(groupeDe(-2, groupes)).toEqual([-1, -2]);
    expect(groupeDe(-9, groupes)).toEqual([-9]);
  });
  it('ne garde qu’une carte par groupe en Taodbox (la première du groupe présente)', () => {
    const jeux = [{ id: -2 }, { id: 110 }, { id: -1 }, { id: -7 }];
    expect(unParGroupe(jeux, groupes).map((j) => j.id)).toEqual([-1, 110, -7]);
  });
  it('dit d’où vient chaque carte, pour tous les jeux', () => {
    expect(originesCarte({ id: -1, boutique: 'steam', copies: [{ boutique: 'steam' }, { boutique: 'gog' }] })).toEqual(['steam', 'gog']);
    expect(originesCarte({ id: -3, boutique: 'epic' })).toEqual(['epic']);
    expect(originesCarte({ id: -4, source: 'local' })).toEqual(['disque']);
    expect(originesCarte({ id: 110 })).toEqual(['firehouse']);
    expect(originesJeuPc({ id: -1, installation: { boutique: { source: 'steam' } } }, [-1, -2], (x) => (x === -1 ? 'steam' : 'gog'))).toEqual(['steam', 'gog']);
    expect(originesJeuPc({ id: 110, installation: {} }, [110], () => undefined)).toEqual(['firehouse']);
    expect([nomOrigine('gog'), nomOrigine('disque'), nomOrigine('zzz')]).toEqual(['GOG', 'Disque', 'Autre']);
  });
  it('présente une copie pour le choix avant de lancer', () => {
    const l = libelleCopie({ id: -2, boutique: 'gog', temps_jeu: 0 }, (b) => b.toUpperCase(), (s) => `${s} s`);
    expect(l).toEqual({ libelle: '▶ GOG', detail: 'jamais lancé par Frogtend' });
  });
});
