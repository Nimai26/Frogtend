import { describe, expect, it } from 'vitest';
import { aCartes, cartesSurPc } from './cartes';
import type { JeuPc } from '$lib/api';

describe('jeux à cartes (e-Reader)', () => {
  it('reconnaît la plateforme et les cartes sur le PC', () => {
    expect(aCartes('Nintendo e-Reader')).toBe(true);
    expect(aCartes('Game Boy Advance')).toBe(false);
    const j = { version: -41144, fichiers: [{ n: 0, nom: 'a.7z', taille: 1 }, { n: 0, nom: 'b.7z', taille: 1, version: -41145 }] } as unknown as JeuPc;
    expect(cartesSurPc(j)).toEqual([-41144, -41145]);
    expect(cartesSurPc(null)).toEqual([]);
  });
});
