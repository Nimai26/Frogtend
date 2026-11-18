import { describe, expect, it } from 'vitest';
import { compterParGalaxy, explicationGalaxy, source, SOURCES } from './sources';

describe('le menu Importer', () => {
  it('reprend la liste de LaunchBox, dans le même ordre', () => {
    // Les 13 entrées de LaunchBox d'abord (règle de Seb), puis Battle.net (lot A) et celle propre à Frogtend (contrat 1.21).
    expect(SOURCES.at(-1)?.libelle).toBe('Reconnaître mes jeux importés');
    expect(SOURCES.at(-2)?.libelle).toBe('Jeux Battle.net');
    expect(SOURCES.slice(0, -2).map((s) => s.libelle)).toEqual([
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

  it('fait entrer seuls les jeux installés des boutiques (lot A), Galaxy et la clé Steam restant en secours', () => {
    const boutiques: Record<string, string> = { amazon: 'amazon', epic: 'epic', gog: 'gog', steam: 'steam', ubisoft: 'uplay', xbox: 'xboxone', battlenet: 'battlenet' };
    for (const [id, b] of Object.entries(boutiques)) {
      expect(source(id)?.voie).toBe('auto');
      expect(source(id)?.boutique).toBe(b);
      expect(source(id)?.resume).not.toContain('par GOG Galaxy');
    }
    expect(source('steam')?.secours).toBe('steam');
    expect(source('epic')?.secours).toBe('galaxy');
    // EA : pas encore lisible sur le PC (lot E) ; GOG Galaxy reste sa voie.
    expect(source('ea')?.voie).toBe('galaxy');
    for (const id of ['rom', 'dos', 'mame', 'windows', 'manuel', 'installer-dos']) expect(source(id)?.voie).toBe('local');
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
