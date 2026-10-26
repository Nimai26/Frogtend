import { describe, expect, it } from 'vitest';
import { aAnnoncer } from './sauvegardeTaille';

const MO = 1024 * 1024;

describe('la taille d’une sauvegarde', () => {
  it('est annoncée seulement au-delà du seuil, et seulement pour ce qui est nouveau', () => {
    // Première sauvegarde : tout est nouveau.
    expect(aAnnoncer({ total: 300 * MO, precedent: null }, 200)).toBe(300 * MO);
    expect(aAnnoncer({ total: 150 * MO, precedent: null }, 200)).toBeNull();
    // Une sauvegarde de 1 Go déjà faite, 50 Mo de plus : rien à demander.
    expect(aAnnoncer({ total: 1074 * MO, precedent: 1024 * MO }, 200)).toBeNull();
    // 400 Mo de plus (un jeu PS3 commencé) : demandé.
    expect(aAnnoncer({ total: 1424 * MO, precedent: 1024 * MO }, 200)).toBe(400 * MO);
    // Une sauvegarde qui a maigri : rien.
    expect(aAnnoncer({ total: 10 * MO, precedent: 900 * MO }, 200)).toBeNull();
  });
});
