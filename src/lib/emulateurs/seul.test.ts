import { describe, expect, it } from 'vitest';
import { boutonSeulMontre, lancementImpossibleSelon, libelleSeul, nomsPrisEnCharge, ouvrableSeul, ouvrablesSeuls, raisonDAttente, refusDeJouer } from './seul';

const libre = { ouvert: null, ouverture: null };
const ouvert = { ouvert: 'Eden', ouverture: null };
const enOuverture = { ouvert: null, ouverture: 7 };

describe('ouvrir un émulateur seul', () => {
  it('reconnaît les émulateurs pris en charge à leur programme (la même table que le cœur)', () => {
    // emulateur_seul.rs, `les_emulateurs_pris_en_charge_sont_reconnus_a_leur_programme`.
    for (const p of [
      'E:\\Emulateurs\\Ryubing\\Ryujinx.exe',
      'E:\\Emulateurs\\Eden\\eden.exe',
      'E:\\Emulateurs\\Eglantine\\eglantine.exe',
      'E:\\Emulateurs\\Lichii\\Lichii.exe',
      'E:\\Emulateurs\\DuckStation\\duckstation-qt-x64-ReleaseLTCG.exe',
      'E:\\Emulateurs\\PCSX2\\pcsx2-qt.exe',
      'E:/Emulateurs/Dolphin/Dolphin.exe',
    ])
      expect(ouvrableSeul(p), p).toBe(true);
    for (const p of ['E:\\Emulateurs\\RetroArch\\retroarch.exe', 'E:\\Emulateurs\\RPCS3\\rpcs3.exe', 'E:\\mame\\mame.exe', 'E:\\Eden\\', '', null, undefined])
      expect(ouvrableSeul(p), String(p)).toBe(false);
  });

  it('ne garde que ceux-là parmi les émulateurs réglés, et les nomme pour l’aide', () => {
    const l = [
      { cle: '1', nom: 'RetroArch', programme: 'E:\\RA\\retroarch.exe', ligne: '-L x' },
      { cle: '2', nom: 'DuckStation', programme: 'E:\\DS\\duckstation-qt-x64-ReleaseLTCG.exe', ligne: '' },
    ];
    expect(ouvrablesSeuls(l).map((e) => e.cle)).toEqual(['2']);
    expect(nomsPrisEnCharge()).toBe('Ryubing, Eden, Eglantine, Lichii, DuckStation, PCSX2 et Dolphin');
  });

  it('bloque tout lancement pendant une partie, un lancement, ou un émulateur ouvert seul', () => {
    const rien = { enJeu: null, lancement: null };
    expect(lancementImpossibleSelon(rien, libre)).toBe(false);
    expect(lancementImpossibleSelon({ enJeu: 3, lancement: null }, libre)).toBe(true);
    expect(lancementImpossibleSelon({ enJeu: null, lancement: 3 }, libre)).toBe(true);
    expect(lancementImpossibleSelon(rien, ouvert)).toBe(true);
    expect(lancementImpossibleSelon(rien, enOuverture)).toBe(true);
  });

  it('montre le bouton seulement avec un émulateur ouvrable, jamais pour un jeu PC sans émulateur', () => {
    expect(boutonSeulMontre(false, 1)).toBe(true);
    expect(boutonSeulMontre(false, 0)).toBe(false);
    expect(boutonSeulMontre(true, 2)).toBe(false);
  });

  it('dit « Lancement… », l’émulateur ouvert, et « … » quand il y a un choix', () => {
    expect(libelleSeul(7, libre, 1)).toBe('▶ Ouvrir l’émulateur seul');
    expect(libelleSeul(7, libre, 2)).toBe('▶ Ouvrir l’émulateur seul…');
    expect(libelleSeul(7, enOuverture, 2)).toBe('Lancement…');
    expect(libelleSeul(8, enOuverture, 2), 'seulement sur le bouton de ce jeu').toBe('▶ Ouvrir l’émulateur seul…');
    expect(libelleSeul(7, ouvert, 1)).toBe('🕹 Eden ouvert…');
  });

  it('refuse ▶ Jouer pendant l’ouverture et explique pourquoi', () => {
    expect(refusDeJouer(libre)).toBeNull();
    expect(refusDeJouer(ouvert)).toBe('🕹 Eden est ouvert : ferme-le d’abord, puis lance le jeu.');
    expect(refusDeJouer(enOuverture)).toContain('Un émulateur est ouvert');
    expect(raisonDAttente(ouvert)).toBe('🕹 Eden est ouvert : ferme-le pour jouer.');
    expect(raisonDAttente(libre)).toBeNull();
  });
});
