import { describe, expect, it } from 'vitest';
import { BOUTONS, detailTouche, libelleClavier, memes, nettoyer, PREREGLAGES, toucheDepuisEvenement, TOUCHES } from './clavier';

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

describe('choisir une touche en l’appuyant', () => {
  const t = (code: string, key: string) => toucheDepuisEvenement({ code, key });
  it('les lettres d’après ce qui est écrit sur la touche (clavier français)', () => {
    expect(t('KeyQ', 'a')).toEqual({ touche: 'A' });
    expect(t('KeyA', 'q')).toEqual({ touche: 'Q', position: 'Déplacement gauche' });
    expect(t('KeyW', 'z')).toEqual({ touche: 'Z', position: 'Déplacement haut' });
    expect(t('KeyS', 's')).toEqual({ touche: 'S', position: 'Déplacement bas' });
    // Le M d'un clavier français est à la place du « ; » anglais (qui, lui, reste refusé).
    expect(t('Semicolon', 'm')).toEqual({ touche: 'M' });
  });
  it('les chiffres d’après leur place (& é " sur la rangée du haut)', () => {
    expect(t('Digit1', '&')).toEqual({ touche: '1' });
    expect(t('Digit2', 'é')).toEqual({ touche: '2' });
  });
  it('les touches spéciales, Échap et Tab compris', () => {
    expect(t('Escape', 'Escape')).toEqual({ touche: 'Échap' });
    expect(t('Tab', 'Tab')).toEqual({ touche: 'Tab' });
    expect(t('ShiftRight', 'Shift')).toEqual({ touche: 'Maj' });
    expect(t('NumpadEnter', 'Enter')).toEqual({ touche: 'Entrée' });
    expect(t('ArrowLeft', 'ArrowLeft')).toEqual({ touche: 'Gauche' });
  });
  it('refuse ce que le cœur ne sait pas envoyer', () => {
    for (const [code, key] of [['Numpad1', '1'], ['BracketLeft', 'Dead'], ['MetaLeft', 'Meta'], ['CapsLock', 'CapsLock'], ['Semicolon', ';'], ['AltRight', 'AltGraph'], ['F13', 'F13']])
      expect('refus' in t(code, key), code).toBe(true);
  });
  it('tout ce qui est accepté est une touche connue du cœur, et chaque touche est atteignable', () => {
    const codes = [
      ...'ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split('').map((l) => [`Key${l}`, l.toLowerCase()]),
      ...'0123456789'.split('').map((c) => [`Digit${c}`, c]),
      ...['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'Enter', 'Space', 'Escape', 'ShiftLeft', 'ControlLeft', 'AltLeft', 'Tab', 'Backspace', 'Insert', 'Delete', 'Home', 'End', 'PageUp', 'PageDown', ...Array.from({ length: 12 }, (_, i) => `F${i + 1}`)].map((c) => [c, c]),
    ];
    const vues = new Set<string>();
    for (const [code, key] of codes) {
      const r = t(code, key);
      expect('touche' in r, code).toBe(true);
      if ('touche' in r) {
        expect(TOUCHES).toContain(r.touche);
        vues.add(r.touche);
        if (r.position) vues.add(r.position);
      }
    }
    expect(TOUCHES.filter((x) => !vues.has(x))).toEqual([]);
  });
});
