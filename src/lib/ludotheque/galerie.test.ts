import { describe, expect, it } from 'vitest';
import { creditMedia, grosPixels, largeurUtile, libelleMedia, ordonner, type MediaJeu } from './galerie';

const m = (o: Partial<MediaJeu>): MediaJeu => ({ id: 1, type: 'capture', principal: false, ...o });

describe('la galerie d’un jeu', () => {
  it('range par type, la principale d’abord, la jaquette en dernier', () => {
    const l = ordonner([
      m({ id: 9, type: 'jaquette', principal: true }),
      m({ id: 3, type: 'capture' }),
      m({ id: 2, type: 'capture', principal: true }),
      m({ id: 5, type: 'boite_3d', principal: true }),
      m({ id: 7, type: 'type_inconnu' }),
    ]);
    expect(l.map((x) => x.id)).toEqual([5, 2, 3, 7, 9]);
  });

  it('dit les choses simplement, sans dimensions', () => {
    expect(libelleMedia(m({ type: 'dos', region: 'Japan' }))).toBe('Dos de la boîte · Japon');
    expect(libelleMedia(m({ type: 'capture' }))).toBe('Image du jeu');
    expect(libelleMedia(m({ type: 'tranche', region: 'Narnia' }))).toBe('Tranche · Narnia');
  });

  it('ne demande jamais plus grand que l’original, et garde les gros pixels', () => {
    expect(largeurUtile(m({ largeur: 256 }), 320)).toBe(256);
    expect(largeurUtile(m({ largeur: 4500 }), 320)).toBe(320);
    expect(largeurUtile(m({}), 320)).toBe(320);
    expect(grosPixels(m({ largeur: 256 }), 1000)).toBe(true);
    expect(grosPixels(m({ largeur: 800 }), 1000)).toBe(false);
    expect(grosPixels(m({ largeur: 1920 }), 1000)).toBe(false);
  });
});

describe('le crédit d’une image de jeu', () => {
  it('cite la source sauf LaunchBox, avec l’auteur et la licence quand ils sont donnés', () => {
    const m = { id: 1, type: 'capture', principal: false };
    expect(creditMedia({ ...m, source: 'launchbox' })).toBe('');
    expect(creditMedia(m)).toBe('');
    expect(creditMedia({ ...m, source: 'igdb' })).toBe('Image : IGDB.com');
    expect(creditMedia({ ...m, source: 'libretro' })).toBe('Image : libretro-thumbnails');
    expect(creditMedia({ ...m, source: 'wikimedia', auteur: 'Dupont', licence: 'CC BY-SA 4.0' })).toBe('Image : Dupont, wikimedia · CC BY-SA 4.0');
  });
});
