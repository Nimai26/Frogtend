import { describe, expect, it } from 'vitest';
import {
  completer,
  DEFAUTS_PC,
  DEFAUTS_PROFIL,
  ecrire,
  lire,
  reinitialiser,
  verifierAdresse,
} from './reglages';

describe('les défauts', () => {
  it('pointent Firehouse sur core.hikari-no-sekai.fr, en mode simulé tant que l’API n’existe pas', () => {
    expect(DEFAUTS_PC.firehouse.adresse).toBe('https://core.hikari-no-sekai.fr');
    expect(DEFAUTS_PC.firehouse.simule).toBe(true);
  });
  it('suivent le skin choisi dans Firehouse', () => {
    expect(DEFAUTS_PROFIL.apparence.skin).toBeNull();
  });
});

describe('completer', () => {
  it('rend les défauts pour un fichier vide ou illisible', () => {
    expect(completer(DEFAUTS_PROFIL, undefined)).toEqual(DEFAUTS_PROFIL);
    expect(completer(DEFAUTS_PROFIL, 'n’importe quoi')).toEqual(DEFAUTS_PROFIL);
  });
  it('garde les valeurs lues et complète les clés absentes', () => {
    const r = completer(DEFAUTS_PROFIL, { apparence: { skin: 'aquamarine', echelle: 1.5 } });
    expect(r.apparence.skin).toBe('aquamarine');
    expect(r.apparence.echelle).toBe(1.5);
    expect(r.apparence.densite).toBe('aeree');
  });
  it('remplace une valeur du mauvais type et ignore les clés inconnues', () => {
    const r = completer(DEFAUTS_PROFIL, { apparence: { echelle: 'grand', inconnue: 1 } });
    expect(r.apparence.echelle).toBe(1);
    expect(r.apparence).not.toHaveProperty('inconnue');
  });
});

describe('lire, écrire, réinitialiser', () => {
  it('lit et écrit par chemin sans modifier l’original', () => {
    const r = ecrire(DEFAUTS_PROFIL, 'apparence.skin', 'dracula');
    expect(lire(r, 'apparence.skin')).toBe('dracula');
    expect(DEFAUTS_PROFIL.apparence.skin).toBeNull();
  });
  it('remet un seul réglage, ou tout, à l’origine', () => {
    let r = ecrire(DEFAUTS_PROFIL, 'apparence.skin', 'dracula');
    r = ecrire(r, 'apparence.echelle', 2);
    const un = reinitialiser(r, DEFAUTS_PROFIL, 'apparence.echelle');
    expect(un.apparence.echelle).toBe(1);
    expect(un.apparence.skin).toBe('dracula');
    expect(reinitialiser(r, DEFAUTS_PROFIL)).toEqual(DEFAUTS_PROFIL);
  });
});

describe('verifierAdresse', () => {
  it('accepte une adresse http(s) et retire la barre finale', () => {
    expect(verifierAdresse(' https://core.hikari-no-sekai.fr/ ')).toEqual({ adresse: 'https://core.hikari-no-sekai.fr' });
    expect(verifierAdresse('http://10.10.0.2:8100')).toEqual({ adresse: 'http://10.10.0.2:8100' });
  });
  it('refuse le reste, avec un motif', () => {
    expect(verifierAdresse('core.hikari')).toHaveProperty('refus');
    expect(verifierAdresse('ftp://x.fr')).toHaveProperty('refus');
  });
});
