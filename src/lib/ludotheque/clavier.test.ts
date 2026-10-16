import { describe, expect, it } from 'vitest';
import { BOUTONS, detailTouche, libelleClavier, memes, nettoyer, PREREGLAGES, TOUCHES } from './clavier';

describe('manette → clavier', () => {
  it('les préréglages n’utilisent que des boutons et des touches connus, jamais View (Back) d’office', () => {
    for (const p of PREREGLAGES) {
      expect(nettoyer(p.correspondance)).toEqual(p.correspondance);
      expect(p.correspondance.select).toBeUndefined();
    }
  });
  it('le préréglage rétro convient aux jeux de fans comme Mega Man Rock Force (Z saut, X tir, C glisse)', () => {
    const r = PREREGLAGES.find((p) => p.id === 'retro')!.correspondance;
    expect([r.a, r.x, r.b, r.start, r.lb, r.rb, r.haut]).toEqual(['Z', 'X', 'C', 'Entrée', 'A', 'S', 'Haut']);
  });
  it('le déplacement des jeux PC marche avec tous les claviers (par position, pas par lettre)', () => {
    const m = PREREGLAGES.find((p) => p.id === 'moderne')!.correspondance;
    expect([m.haut, m.gauche, m.bas, m.droite].every((t) => t.startsWith('Déplacement'))).toBe(true);
    expect(detailTouche('Déplacement haut')).toContain('W A S D');
  });
  it('dit la correspondance en mots simples', () => {
    expect(libelleClavier(undefined)).toBe('non');
    expect(libelleClavier({})).toBe('non');
    expect(libelleClavier({ ...PREREGLAGES[0].correspondance })).toBe('Flèches + Z X C (jeux rétro, jeux de fans)');
    expect(libelleClavier({ a: 'Z' })).toBe('réglé à la main (1 bouton)');
    expect(libelleClavier({ a: 'Z', b: 'X' })).toBe('réglé à la main (2 boutons)');
    expect(memes({ a: 'Z', b: 'X' }, { b: 'X', a: 'Z' })).toBe(true);
  });
  it('nettoie une correspondance abîmée', () => {
    expect(nettoyer({ a: 'Z', b: 'Inconnue', zz: 'X', x: 3 })).toEqual({ a: 'Z' });
    expect(nettoyer(null)).toEqual({});
    expect(TOUCHES).toContain('Échap');
    expect(new Set(TOUCHES).size).toBe(TOUCHES.length);
    expect(BOUTONS).toHaveLength(16);
  });
});
