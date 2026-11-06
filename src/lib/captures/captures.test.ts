import { describe, expect, it } from 'vitest';
import { adresseCapture, parConsole, type Capture } from './captures';

const c = (id: string, jeu: number, titre: string, plateforme: string, quand: number): Capture => ({ id, jeu, titre, plateforme, quand, fond_possible: true, est_le_fond: false });

describe('la galerie des captures', () => {
  it('regroupe par console puis par jeu, sans changer l’ordre', () => {
    const g = parConsole([c('a', 1, 'SMW', 'SNES', 30), c('b', 2, 'Asura', 'PS3', 20), c('d', 1, 'SMW', 'SNES', 10)]);
    expect(g.map((x) => x.plateforme)).toEqual(['SNES', 'PS3']);
    expect(g[0].jeux[0].captures.map((x) => x.id)).toEqual(['a', 'd']);
  });
  it('l’adresse d’une capture', () => {
    expect(adresseCapture('0a1b')).toBe('http://capture.localhost/0a1b');
  });
});
