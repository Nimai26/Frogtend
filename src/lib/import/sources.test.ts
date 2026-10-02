import { describe, expect, it } from 'vitest';
import { compterParGalaxy, explicationGalaxy, source, SOURCES } from './sources';

describe('le menu Importer', () => {
  it('reprend la liste de LaunchBox, dans le même ordre', () => {
    expect(SOURCES.map((s) => s.libelle)).toEqual([
      'Fichiers ROM',
      'Jeux MS-DOS',
      'MAME Arcade Full Set',
      'Amazon Games',
      'EA',
      'Jeux Epic Games',
      'Jeux GOG',
      'Jeux Steam',
      'Uplay / Ubisoft Connect',
      'Jeux Windows',
      'Jeux Xbox / Microsoft Store',
      'Ajouter un jeu manuellement',
      'Installer un jeu DOS',
    ]);
    expect(new Set(SOURCES.map((s) => s.id)).size).toBe(SOURCES.length);
  });

  it('passe par GOG Galaxy pour les boutiques sans API ouverte', () => {
    for (const id of ['amazon', 'ea', 'epic', 'gog', 'ubisoft', 'xbox']) expect(source(id)?.voie).toBe('galaxy');
    expect(source('steam')?.voie).toBe('steam');
    for (const id of ['rom', 'dos', 'windows', 'manuel']) expect(source(id)?.voie).toBe('local');
    for (const id of ['mame', 'installer-dos']) expect(source(id)?.voie).toBe('bientot');
    expect(source('inconnue')).toBeUndefined();
  });

  it('compte les jeux d’une boutique parmi ceux de Galaxy', () => {
    const jeux = [{ plateforme: 'epic' }, { plateforme: 'gog' }, { plateforme: 'origin' }, { plateforme: 'ea' }, { plateforme: null }];
    expect(compterParGalaxy(source('epic')!, jeux)).toBe(1);
    expect(compterParGalaxy(source('ea')!, jeux)).toBe(2);
    expect(compterParGalaxy(source('rom')!, jeux)).toBe(0);
  });

  it('explique où relier un compte dans Galaxy', () => {
    const t = explicationGalaxy('Epic Games').join(' ');
    expect(t).toContain('ton compte Epic Games');
    expect(t).toContain('Paramètres ▸ Intégrations');
    expect(t).toContain('LECTURE SEULE');
    expect(explicationGalaxy().join(' ')).toContain('tes comptes reliés');
  });
});
