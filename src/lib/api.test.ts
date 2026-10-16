import { describe, expect, it } from 'vitest';
import { adresseFond, adresseFondDuJeu, adresseJaquette, adressePageLivret, libelleCompte, taille } from './api';

describe('adressePageLivret', () => {
  it('demande une page à une largeur arrondie aux 200 px supérieurs, entre 200 et 2 400', () => {
    expect(adressePageLivret(111, 6142, 0, 700)).toBe('http://document.localhost/111/6142/0?largeur=800');
    expect(adressePageLivret(111, 6142, 22, 50)).toBe('http://document.localhost/111/6142/22?largeur=200');
    expect(adressePageLivret(111, 6142, 3, 9000)).toBe('http://document.localhost/111/6142/3?largeur=2400');
  });
});

describe('adresseJaquette', () => {
  it('demande une miniature arrondie à la centaine supérieure, entre 100 et 1000', () => {
    expect(adresseJaquette(110)).toBe('http://jaquette.localhost/110');
    expect(adresseJaquette(110, 170)).toBe('http://jaquette.localhost/110?largeur=200');
    expect(adresseJaquette(110, 340)).toBe('http://jaquette.localhost/110?largeur=400');
    expect(adresseJaquette(110, 40)).toBe('http://jaquette.localhost/110?largeur=100');
    expect(adresseJaquette(110, 2400)).toBe('http://jaquette.localhost/110?largeur=1000');
  });
});

describe('adresseFondDuJeu', () => {
  it('demande le fond, sinon une capture, sinon l’écran titre, en 1 000 px au plus', () => {
    expect(adresseFondDuJeu(116)).toBe('http://jaquette.localhost/116?visuel=fond,capture,ecran_titre&largeur=1000');
  });
});

describe('divers', () => {
  it('nomme la vidéo de fond d’un skin', () => {
    expect(adresseFond('firehouse')).toBe('http://fond.localhost/firehouse');
  });
  it('présente un compte Firehouse', () => {
    expect(libelleCompte({ nom: 'Seb', grade: 'admin' })).toBe('Seb (admin)');
    expect(libelleCompte({ username: 'lea' })).toBe('lea');
    expect(libelleCompte(null)).toBe('');
  });
  it('écrit les tailles en français', () => {
    expect(taille(237887038)).toBe('226,9 Mo');
    expect(taille(324)).toBe('324 o');
  });
});
