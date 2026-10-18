import { describe, expect, it } from 'vitest';
import { visuelDuJeu } from './visuels';

// Le relevé réel du 18/10 (3x3 Eyes, jeu 116) : 11 types, dont boîte 3D 779×1350 et capture 256×224.
const images = {
  jaquette: { largeur: 1281, hauteur: 2311 },
  boite_3d: { largeur: 779, hauteur: 1350 },
  capture: { largeur: 256, hauteur: 224 },
};

describe('le visuel d’un jeu dans la grille', () => {
  it('prend le visuel choisi, au ratio réel, dans le cadre de son type', () => {
    const v = visuelDuJeu('boite_3d', images, 170);
    expect(v.sorte).toBe('boite_3d');
    expect(v.cadre).toBe('3 / 4');
    expect(v.ratio).toBeCloseTo(779 / 1350);
    expect(visuelDuJeu('capture', images, 170).cadre).toBe('4 / 3');
  });

  it('retombe sur la jaquette quand on SAIT que le jeu n’a pas ce visuel, dans le cadre du visuel choisi', () => {
    const v = visuelDuJeu('cartouche', images, 170);
    expect(v.sorte).toBe('jaquette');
    expect(v.cadre).toBe('4 / 3');
    expect(v.ratio).toBeCloseTo(1281 / 2311);
  });

  it('essaie quand on ne sait pas encore (catalogue pas relu, jeu du PC, Taodbox)', () => {
    const v = visuelDuJeu('cartouche', undefined, 170);
    expect(v.sorte).toBe('cartouche');
    expect(v.ratio).toBeNull();
    expect(visuelDuJeu(undefined, undefined, 170).sorte).toBe('jaquette');
    expect(visuelDuJeu('nimporte', images, 170).sorte).toBe('jaquette');
  });

  it('garde les gros pixels d’une petite image d’époque agrandie', () => {
    expect(visuelDuJeu('capture', images, 300).pixels).toBe(true);
    expect(visuelDuJeu('capture', images, 200).pixels).toBe(false);
    expect(visuelDuJeu('boite_3d', images, 2000).pixels).toBe(false);
  });

  it('un jeu sans aucun visuel (MS-DOS au 16/10) : la jaquette, sans ratio', () => {
    const v = visuelDuJeu('boite_3d', {}, 170);
    expect(v.sorte).toBe('jaquette');
    expect(v.ratio).toBeNull();
  });
});
