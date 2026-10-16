import { describe, expect, it } from 'vitest';
import { ajouter, estRetroArch, normaliser, normaliserTous, pourLeJeu, retirer, systemesProposes } from './choix';

const snes9x = { nom: 'RetroArch — snes9x', programme: 'E:\\RetroArch\\retroarch.exe', ligne: '-L cores\\snes9x_libretro.dll' };
const bsnes = { nom: 'RetroArch — bsnes', programme: 'E:\\RetroArch\\retroarch.exe', ligne: '-L cores\\bsnes_libretro.dll' };

describe('l’origine d’un réglage (conseil de Firehouse)', () => {
  it('survit à la relecture des réglages et à un ajout', () => {
    const sys = { defaut: '1', liste: [{ cle: '1', nom: 'RetroArch', programme: 'r.exe', ligne: '-f', conseil: '2026-10-04T13:27:38' }] };
    expect(normaliser(JSON.parse(JSON.stringify(sys))).liste[0].conseil).toBe('2026-10-04T13:27:38');
    expect(ajouter(sys, { nom: 'X', programme: 'x.exe', ligne: '' }).sys.liste[0].conseil).toBe('2026-10-04T13:27:38');
    expect('conseil' in normaliser({ liste: [{ programme: 'a.exe' }] }).liste[0]).toBe(false);
  });
});

describe('plusieurs émulateurs par système', () => {
  it('lit l’ancienne forme comme une liste d’un seul', () => {
    const n = normaliser({ programme: 'E:\\DOSBox\\dosbox.exe', ligne: '', nom: 'DOSBox Staging' });
    expect(n).toEqual({ liste: [{ cle: '1', nom: 'DOSBox Staging', programme: 'E:\\DOSBox\\dosbox.exe', ligne: '' }], defaut: '1' });
    expect(normaliserTous({ 'MS-DOS': { programme: 'x.exe', ligne: '' }, Vide: {} })).toHaveProperty('MS-DOS');
    expect(normaliserTous({ Vide: {} })).toEqual({});
  });

  it('ajoute sans doublon, le premier devient le défaut', () => {
    const a = ajouter(undefined, snes9x);
    expect(a.sys.defaut).toBe(a.cle);
    const b = ajouter(a.sys, bsnes);
    expect(b.sys.liste).toHaveLength(2);
    expect(b.sys.defaut).toBe(a.cle); // le défaut ne change pas
    const c = ajouter(b.sys, { ...snes9x, nom: 'autre nom' });
    expect(c.sys.liste).toHaveLength(2);
    expect(c.cle).toBe(a.cle);
  });

  it('retirer le défaut le passe au suivant', () => {
    const a = ajouter(undefined, snes9x);
    const b = ajouter(a.sys, bsnes);
    const r = retirer(b.sys, a.cle);
    expect(r.liste.map((e) => e.nom)).toEqual(['RetroArch — bsnes']);
    expect(r.defaut).toBe(b.cle);
    expect(retirer(r, b.cle)).toEqual({ liste: [], defaut: '' });
  });

  it('le défaut du jeu passe avant celui du système, et un défaut disparu retombe', () => {
    const a = ajouter(undefined, snes9x);
    const b = ajouter(a.sys, bsnes);
    const emus = { 'Super Nintendo': b.sys };
    expect(pourLeJeu(emus, {}, 'Super Nintendo', 1)?.nom).toBe('RetroArch — snes9x');
    expect(pourLeJeu(emus, { '42': b.cle }, 'Super Nintendo', 42)?.nom).toBe('RetroArch — bsnes');
    expect(pourLeJeu(emus, { '42': 'disparu' }, 'Super Nintendo', 42)?.nom).toBe('RetroArch — snes9x');
    expect(pourLeJeu(emus, {}, 'Nintendo 64', 1)).toBeNull();
  });

  it('propose tous les systèmes, ceux qui ont des jeux d’abord', () => {
    const l = systemesProposes(
      [{ nom: 'Super Nintendo Entertainment System', jeux: 1 }, { nom: 'Sony Playstation 3', jeux: 7 }, { nom: 'Sony Playstation 3', jeux: 2 }],
      ['Sega Saturn', 'Sony Playstation 3', 'Atari 2600'],
    );
    expect(l).toEqual([
      { nom: 'Sony Playstation 3', jeux: 7 },
      { nom: 'Super Nintendo Entertainment System', jeux: 1 },
      { nom: 'Atari 2600', jeux: 0 },
      { nom: 'Sega Saturn', jeux: 0 },
    ]);
    // Aucun jeu nulle part : toutes les plateformes connues restent proposées.
    expect(systemesProposes([], ['Sony Playstation 3']).map((p) => p.nom)).toEqual(['Sony Playstation 3']);
  });
});

describe('RetroArch reconnu par son programme', () => {
  it('pour prévenir de la question du pare-feu', () => {
    expect(estRetroArch(String.raw`E:\Emulateurs\RetroArch\retroarch.exe`)).toBe(true);
    expect(estRetroArch('D:/RA/RetroArch.EXE')).toBe(true);
    expect(estRetroArch(String.raw`E:\Emulateurs\DuckStation\duckstation-qt.exe`)).toBe(false);
    expect(estRetroArch(String.raw`E:\x\notretroarch.exe`)).toBe(false);
    expect(estRetroArch(undefined)).toBe(false);
  });
});
