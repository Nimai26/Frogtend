import { describe, expect, it } from 'vitest';
import { aCartes, cartesSurPc, choixDeCartes } from './cartes';
import type { JeuPc } from '$lib/api';

describe('jeux à cartes (e-Reader)', () => {
  it('reconnaît la plateforme et les cartes sur le PC', () => {
    expect(aCartes('Nintendo e-Reader')).toBe(true);
    expect(aCartes('Game Boy Advance')).toBe(false);
    const j = { version: -41144, fichiers: [{ n: 0, nom: 'a.7z', taille: 1 }, { n: 0, nom: 'b.7z', taille: 1, version: -41145 }] } as unknown as JeuPc;
    expect(cartesSurPc(j)).toEqual([-41144, -41145]);
    expect(cartesSurPc(null)).toEqual([]);
  });

  it('propose les cartes du PC par leur nom, dans l’ordre des numéros', () => {
    const j = {
      version: 1,
      fichiers: [
        { n: 0, nom: 'Pokemon-e TCG - 10-A010 (USA).7z', taille: 1 },
        { n: 0, nom: 'Pokemon-e TCG - 2-A002 (USA).7z', taille: 1, version: 2 },
      ],
    } as unknown as JeuPc;
    expect(choixDeCartes(j)).toEqual([
      { valeur: 2, libelle: 'Pokemon-e TCG - 2-A002 (USA)' },
      { valeur: 1, libelle: 'Pokemon-e TCG - 10-A010 (USA)' },
    ]);
    expect(choixDeCartes(undefined)).toEqual([]);
  });
});
