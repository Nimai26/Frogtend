import { describe, expect, it } from 'vitest';
import { ageConseille, datePrecise, dateDeSortie, noteSurCinq, typeDeSortie } from './infos';

describe('les infos du jeu (contrat 1.15), en français', () => {
  it('la date de sortie, sans dépendre du fuseau', () => {
    expect(dateDeSortie('1992')).toBe('1992');
    expect(dateDeSortie('1992-03')).toBe('mars 1992');
    expect(dateDeSortie('1992-03-12')).toBe('12 mars 1992');
    expect(dateDeSortie('1992-01-01')).toBe('1er janvier 1992');
    expect(dateDeSortie('')).toBeNull();
    expect(dateDeSortie(null)).toBeNull();
    expect(dateDeSortie('mars 92')).toBeNull();
    expect(dateDeSortie('1992-13')).toBe('1992');
    expect(datePrecise('1992')).toBe(false);
    expect(datePrecise('1992-03')).toBe(true);
  });

  it('le type de sortie : seulement ce qui mérite d’être signalé', () => {
    expect(typeDeSortie('Released')).toBeNull();
    expect(typeDeSortie('Early Access')).toBe('Accès anticipé');
    expect(typeDeSortie('Unreleased')).toBe('Jamais sorti');
    expect(typeDeSortie('ROM Hack')).toBe('ROM modifiée (hack)');
    expect(typeDeSortie('')).toBeNull();
    expect(typeDeSortie('Something New')).toBeNull();
  });

  it('l’âge conseillé : PEGI d’abord, sinon ESRB, jamais l’anglais brut', () => {
    expect(ageConseille({ pegi: 18, esrb: 'T - Teen' })).toBe('18 ans et plus (PEGI)');
    expect(ageConseille({ pegi: null, esrb: 'M - Mature' })).toBe('17 ans et plus (ESRB)');
    expect(ageConseille({ esrb: 'E - Everyone' })).toBe('6 ans et plus (ESRB)');
    // L'exemple réel de Dune : « Not Rated ».
    expect(ageConseille({ esrb: 'Not Rated', pegi: null })).toBeNull();
    expect(ageConseille({ esrb: 'RP - Rating Pending' })).toBeNull();
    expect(ageConseille({ esrb: 'Inconnu', pegi: 5 })).toBeNull();
    expect(ageConseille(null)).toBeNull();
  });

  it('la note sur 5, une décimale, 0 compris', () => {
    expect(noteSurCinq(3.93827160493827)).toBe('3,9 / 5');
    expect(noteSurCinq(0)).toBe('0,0 / 5');
    expect(noteSurCinq(5)).toBe('5,0 / 5');
    expect(noteSurCinq(null)).toBeNull();
    expect(noteSurCinq(7)).toBeNull();
  });
});
