// Le retour à la grille de Taodbox : même console, même carte, une seule fois.
import { beforeEach, describe, expect, it } from 'vitest';
import { consoleValide, oublierPlace, reprendrePlace, retenirPlace, selecteurCarte } from './retour';

beforeEach(() => oublierPlace());

describe('la place dans la grille', () => {
  it('se reprend une seule fois', () => {
    retenirPlace({ id: 12, console: 'Nintendo Switch' });
    expect(reprendrePlace()).toEqual({ id: 12, console: 'Nintendo Switch' });
    expect(reprendrePlace()).toBeNull();
  });

  it('s’oublie quand « Gérer » se referme sans quitter la grille', () => {
    retenirPlace({ id: 12, console: null });
    oublierPlace();
    expect(reprendrePlace()).toBeNull();
  });

  it('la console seulement si elle a encore des jeux', () => {
    expect(consoleValide({ id: 1, console: 'Nintendo Switch' }, ['Nintendo Switch', 'MS-DOS'])).toBe('Nintendo Switch');
    expect(consoleValide({ id: 1, console: 'Sega Saturn' }, ['MS-DOS'])).toBeNull();
    expect(consoleValide(null, ['MS-DOS'])).toBeNull();
  });

  it('vise la carte du jeu, sinon la première', () => {
    expect(selecteurCarte({ id: -3, console: null })).toBe('.taodbox .carte[data-id="-3"]');
    expect(selecteurCarte(null)).toBe('.taodbox .carte');
  });
});
