import { describe, expect, it } from 'vitest';
import { directionDuStick, voisin, type Boite } from './navigation';

// Une grille 3 × 2 de cartes, et un bouton « Quitter » seul en haut à droite.
const b = (x: number, y: number): Boite => ({ x, y, l: 100, h: 140 });
const grille: Boite[] = [b(0, 100), b(120, 100), b(240, 100), b(0, 260), b(120, 260), b(240, 260), { x: 400, y: 0, l: 80, h: 30 }];

describe('voisin', () => {
  it('suit la grille', () => {
    expect(voisin(grille, 0, 'droite')).toBe(1);
    expect(voisin(grille, 1, 'bas')).toBe(4);
    expect(voisin(grille, 4, 'gauche')).toBe(3);
    expect(voisin(grille, 3, 'haut')).toBe(0);
  });

  it('n’est jamais piégée : depuis le bord, on atteint ce qui reste (le bouton seul en haut)', () => {
    expect(voisin(grille, 2, 'haut')).toBe(6);
    expect(voisin(grille, 6, 'bas')).not.toBeNull();
  });

  it('rend null quand il n’y a rien dans la direction', () => {
    expect(voisin(grille, 0, 'gauche')).toBeNull();
    expect(voisin(grille, 3, 'bas')).toBeNull();
  });
});

describe('directionDuStick', () => {
  it('ignore la zone morte et prend l’axe dominant', () => {
    expect(directionDuStick(0.2, -0.3)).toBeNull();
    expect(directionDuStick(0.9, 0.2)).toBe('droite');
    expect(directionDuStick(-0.1, -0.8)).toBe('haut');
    expect(directionDuStick(0.1, 0.95)).toBe('bas');
  });
});
