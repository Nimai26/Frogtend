import { describe, expect, it } from 'vitest';
import { filtrerOptions } from './filtre';

describe('chercher dans une longue liste de choix', () => {
  const cartes = [
    { libelle: 'Pokémon-e TCG - 01-A001 (USA)' },
    { libelle: 'Pokemon-e TCG - 01-A002 (USA)', detail: 'Expedition' },
    { libelle: 'Air Hockey-e - 00-B000 (USA) (Promo)' },
  ];
  it('tous les mots, sans majuscules ni accents', () => {
    expect(filtrerOptions(cartes, 'pokemon a002').map((o) => o.libelle)).toEqual(['Pokemon-e TCG - 01-A002 (USA)']);
    expect(filtrerOptions(cartes, 'POKÉMON').length).toBe(2);
    expect(filtrerOptions(cartes, 'expedition').length).toBe(1);
    expect(filtrerOptions(cartes, '  ').length).toBe(3);
    expect(filtrerOptions(cartes, 'zelda')).toEqual([]);
  });
});
