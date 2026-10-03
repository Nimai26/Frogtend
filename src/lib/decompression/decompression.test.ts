import { describe, expect, it } from 'vitest';
import { dossierDe, dureeEstimee, messageConfirmation } from './decompression';

const Go = 1024 ** 3;

describe('décompresser pour jouer', () => {
  it('trouve le dossier d’une archive, chemin Windows compris', () => {
    expect(dossierDe('E:\\Games\\Playstation 3\\Jeu.zip')).toBe('E:\\Games\\Playstation 3');
    expect(dossierDe('E:/Games/Jeu.zip')).toBe('E:/Games');
  });

  it('annonce une durée lisible', () => {
    expect(dureeEstimee(10 * 1024 ** 2)).toBe('moins d’une minute');
    expect(dureeEstimee(6.7 * Go)).toBe('environ 1 minute');
    expect(dureeEstimee(47 * Go)).toBe('environ 8 minutes');
    expect(dureeEstimee(400 * Go)).toBe('environ 1 h 10');
    expect(dureeEstimee(360 * Go)).toBe('environ 1 h');
  });

  it('dit la taille, le dossier, la place et que l’archive est gardée', () => {
    const m = messageConfirmation(
      { archive: 'E:\\PS3\\Jeu.zip', fichiers: 2, taille: 47 * Go, principal: 'Jeu.iso', destination: 'E:\\PS3\\Jeu', deja: false },
      200 * Go,
    );
    expect(m).toContain('Dossier : E:\\PS3\\Jeu');
    expect(m).toContain('2 fichiers');
    expect(m).toContain('L’archive est gardée');
    expect(m).toContain('Place libre');
    expect(messageConfirmation({ archive: 'a.zip', fichiers: 1, taille: 10, principal: 'a.iso', destination: 'a', deja: false }, null)).not.toContain('Place libre');
  });
});
