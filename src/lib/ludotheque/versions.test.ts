import { describe, expect, it } from 'vitest';
import { cheminCourant, estCourante, libelleVersion, nomDuFichier } from './versions';

describe('les versions d’un jeu importé', () => {
  const i = { dossier: 'E:\\NES\\Europe', fichier_du_jeu: 'Mario (E).nes' };
  it('reconnaît la version lancée par défaut', () => {
    expect(cheminCourant(i)).toBe('E:\\NES\\Europe\\Mario (E).nes');
    expect(estCourante({ chemin: 'e:/nes/europe/mario (e).nes', libelle: 'E', rang: 1, qualite: 0 }, i)).toBe(true);
    expect(estCourante({ chemin: 'E:\\NES\\USA\\Mario (U).nes', libelle: 'U', rang: 2, qualite: 0 }, i)).toBe(false);
  });
  it('se lit facilement', () => {
    expect(libelleVersion({ chemin: 'x', libelle: 'France', rang: 0, qualite: 0 })).toBe('FR — France');
    expect(libelleVersion({ chemin: 'x', libelle: '', rang: 3, qualite: 0, disques: ['a', 'b'] })).toBe('autre — sans étiquette · 2 disques');
    expect(nomDuFichier({ chemin: 'E:\\NES\\Mario (F).nes', libelle: '', rang: 0, qualite: 0 })).toBe('Mario (F).nes');
  });
});
