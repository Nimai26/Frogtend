import { describe, expect, it } from 'vitest';
import type { DocumentJeu } from '$lib/api';
import { documentsDuMenu, documentsMontres, duree, libelleDocument } from './documents';

// L'exemple réel du 16/10 (Zelda, /jeu/111/documents).
const livret: DocumentJeu = { id: 6142, type: 'livret', langue: '', format: 'pdf', taille: 8583117, pages: 23, duree_s: null };
const musique: DocumentJeu = { id: 6482, type: 'musique', langue: '', format: 'mp3', taille: 6110240, pages: null, duree_s: 152.6 };
const theme: DocumentJeu = { id: 6973, type: 'video_theme', langue: '', format: 'mp4', taille: 25198576, pages: null, duree_s: 37.3 };

describe('les documents d’un jeu', () => {
  it('dit une durée simplement', () => {
    expect(duree(152.6)).toBe('2 min 33');
    expect(duree(37.3)).toBe('37 s');
    expect(duree(120)).toBe('2 min');
    expect(duree(65)).toBe('1 min 05');
  });

  it('montre livret, musique et vidéo dans cet ordre, jamais la vidéo de thème', () => {
    const video: DocumentJeu = { ...musique, id: 6842, type: 'video', format: 'mp4', duree_s: 36.6 };
    expect(documentsMontres([video, musique, theme, livret])).toEqual([livret, musique, video]);
    expect(libelleDocument(video).detail).toMatch(/^37 s · 5[,.]8 Mo$/);
  });

  it('dans le menu en jeu : musique et vidéo seulement si le jeu est en pause, le livret toujours', () => {
    expect(documentsDuMenu([livret, musique, { ...musique, id: 6842, type: 'video' }], false)).toEqual([livret]);
    expect(documentsDuMenu([livret, musique], true)).toEqual([livret, musique]);
  });

  it('dit la taille tant que le livret n’est pas sur le PC (le clic vaut accord)', () => {
    const avant = libelleDocument(livret);
    expect(avant.titre).toBe('📖 Livret (scan)');
    expect(avant.detail).toMatch(/^23 pages · 8[,.]2 Mo$/);
    expect(libelleDocument({ ...livret, garde: true }).detail).toBe('23 pages');
    expect(libelleDocument({ ...livret, pages: 1, langue: 'fr', garde: true }).detail).toBe('1 page · FR');
  });
});
