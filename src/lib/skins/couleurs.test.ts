import { describe, expect, it } from 'vitest';
import { composer, contraste, lireCouleur, melanger, teintesDe } from './couleurs';

describe('lireCouleur', () => {
  it('lit les formes hexadécimales', () => {
    expect(lireCouleur('#fff')).toEqual([255, 255, 255, 1]);
    expect(lireCouleur('#0A0E14')).toEqual([10, 14, 20, 1]);
    expect(lireCouleur('#00000080')).toEqual([0, 0, 0, 128 / 255]);
  });

  it('refuse #rgba à 4 chiffres, les noms et les dégradés', () => {
    expect(lireCouleur('#0008')).toBeNull();
    expect(lireCouleur('red')).toBeNull();
    expect(lireCouleur('linear-gradient(#000, #fff)')).toBeNull();
  });

  it('lit rgb() et rgba() avec virgules ou espaces, alpha en nombre ou en pourcentage', () => {
    expect(lireCouleur('rgba(20, 25, 35, 0.86)')).toEqual([20, 25, 35, 0.86]);
    expect(lireCouleur('rgb(20 25 35 / 50%)')).toEqual([20, 25, 35, 0.5]);
    expect(lireCouleur('rgb(1,2,3)')).toEqual([1, 2, 3, 1]);
  });

  it('convertit hsl() en RGB arrondi', () => {
    expect(lireCouleur('hsl(0, 100%, 50%)')).toEqual([255, 0, 0, 1]);
    expect(lireCouleur('hsla(211, 18%, 20%, 0.95)')).toEqual([42, 51, 60, 0.95]);
  });
});

describe('teintesDe', () => {
  it('relève toutes les couleurs d’un dégradé à plusieurs couches', () => {
    const bg = 'linear-gradient(to bottom, rgba(0, 0, 0, 0.45), #123456), radial-gradient(hsla(211, 18%, 5%, 0), #fff)';
    expect(teintesDe(bg)).toHaveLength(4);
  });
});

describe('contraste', () => {
  it('vaut 21 entre le noir et le blanc, 1 entre deux couleurs égales', () => {
    expect(contraste([0, 0, 0], [255, 255, 255])).toBeCloseTo(21, 5);
    expect(contraste([80, 80, 80], [80, 80, 80])).toBe(1);
  });
});

describe('composer et mélanger', () => {
  it('pose une couleur translucide sur une teinte', () => {
    expect(composer([255, 255, 255, 0.5], [0, 0, 0])).toEqual([128, 128, 128]);
  });
  it('rapproche une couleur de sa cible par pas', () => {
    expect(melanger([0, 0, 0], [200, 100, 50], 50)).toEqual([100, 50, 25]);
    expect(melanger([0, 0, 0], [200, 100, 50], 100)).toEqual([200, 100, 50]);
  });
});
