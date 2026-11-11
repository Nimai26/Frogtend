import { describe, expect, it } from 'vitest';
import { NOMS, ORDRE, nomCombinaison, raccourcis, sansDessin, schema, type InfosBoutons } from './boutons';

const infos = (i: Partial<InfosBoutons>): InfosBoutons => ({
  plateforme: 'Super Nintendo Entertainment System',
  emulateur: 'retroarch',
  coeur: 'Snes9x',
  famille: 'xbox',
  personnalise: false,
  clavier: null,
  raccourcis: { combinaison: 'Select + R1', bouton_xbox: true, touche: 'Pause', etats: true },
  ...i,
});

const role = (s: ReturnType<typeof schema>, id: string) => s.lignes.find((l) => l.id === id)?.role;

describe('les boutons de la manette d’origine', () => {
  it('chaque position a un nom sur les deux manettes', () => {
    for (const f of ['xbox', 'playstation'] as const) for (const id of ORDRE) expect(NOMS[f][id]).toBeTruthy();
    expect(NOMS.playstation.b).toBe('Rond');
    expect(NOMS.xbox.select).toContain('Vue');
  });

  it('RetroArch : A de la manette Xbox = B de la Super Nintendo (la RetroPad), dans l’ordre', () => {
    const s = schema(infos({}));
    expect(s.etat).toBe('connu');
    expect(s.etat === 'connu' && s.console).toBe('Super Nintendo');
    expect(role(s, 'a')).toBe('B');
    expect(role(s, 'b')).toBe('A');
    expect(role(s, 'x')).toBe('Y');
    expect(role(s, 'y')).toBe('X');
    expect(role(s, 'lt')).toBeUndefined();
    expect(s.lignes.map((l) => l.id)).toEqual(ORDRE.filter((id) => s.lignes.some((l) => l.id === id)));
  });

  it('les autres consoles de RetroArch', () => {
    expect(role(schema(infos({ coeur: 'Beetle PSX HW', plateforme: 'Sony Playstation' })), 'a')).toBe('Croix');
    expect(role(schema(infos({ coeur: 'Beetle PSX HW', plateforme: 'Sony Playstation' })), 'croix')).toBe('Croix directionnelle');
    expect(role(schema(infos({ coeur: 'Genesis Plus GX', plateforme: 'Sega Genesis' })), 'x')).toBe('A');
    expect(role(schema(infos({ coeur: 'Genesis Plus GX', plateforme: 'Sega Genesis' })), 'b')).toBe('C');
    expect(role(schema(infos({ coeur: 'Genesis Plus GX', plateforme: 'Sega Master System' })), 'a')).toBe('Bouton 1');
    expect(role(schema(infos({ coeur: 'Mupen64Plus-Next', plateforme: 'Nintendo 64' })), 'lt')).toBe('Z');
    expect(role(schema(infos({ coeur: 'ParaLLEl N64', plateforme: 'Nintendo 64' })), 'b')).toBeUndefined();
    expect(role(schema(infos({ coeur: 'FinalBurn Neo', plateforme: 'Arcade' })), 'select')).toBe('Pièce');
  });

  it('un cœur inconnu, voisin d’un connu, ou un réglage personnalisé : rien de deviné', () => {
    expect(schema(infos({ coeur: 'Snes9x 2010' })).etat).toBe('inconnu');
    expect(schema(infos({ coeur: 'Mesen-S' })).etat).toBe('inconnu');
    expect(schema(infos({ coeur: 'Genesis Plus GX Wide', plateforme: 'Sega Genesis' })).etat).toBe('connu');
    expect(schema(infos({ coeur: 'Flycast GLES2' })).etat).toBe('connu');
    // Personnalisé, mais disposition inconnue : « inconnu » (la Wii n'est pas réglée par Frogtend).
    expect(schema(infos({ emulateur: 'dolphin', coeur: null, plateforme: 'Nintendo Wii', personnalise: true })).etat).toBe('inconnu');
    expect(schema(infos({ coeur: 'Un cœur nouveau' })).etat).toBe('inconnu');
    expect(schema(infos({ emulateur: 'dolphin', coeur: null, plateforme: 'Nintendo Wii' })).etat).toBe('inconnu');
    const p = schema(infos({ personnalise: true }));
    expect(p.etat).toBe('personnalise');
    expect(p.lignes).toEqual([]);
    expect(sansDessin(p)).toContain('changé toi-même');
    expect(sansDessin(schema(infos({ coeur: 'X' })))).toContain('ne connaît pas encore');
    expect(sansDessin(schema(infos({})))).toBeNull();
  });

  it('Cemu : le GamePad Wii U par position, comme la Switch', () => {
    const wiiu = schema(infos({ emulateur: 'cemu', coeur: null, plateforme: 'Nintendo Wii U' }));
    expect(wiiu.console).toBe('Wii U (GamePad)');
    expect(role(wiiu, 'b')).toBe('A');
    expect(role(wiiu, 'a')).toBe('B');
    expect(role(wiiu, 'lt')).toBe('ZL');
    expect(role(wiiu, 'start')).toBe('+');
    expect(schema(infos({ emulateur: 'cemu', coeur: null, personnalise: true })).etat).toBe('personnalise');
    expect(schema(infos({ emulateur: 'cemu', coeur: null, famille: 'playstation' })).etat).toBe('inconnu');
    expect(schema(infos({ emulateur: 'cemu', coeur: null, inconnue: true, personnalise: true })).etat).toBe('inconnu');
  });

  it('Vita3K : sa disposition d’origine par position, pour toute manette', () => {
    const vita = schema(infos({ emulateur: 'vita3k', coeur: null, plateforme: 'Sony PS Vita' }));
    expect(vita.console).toBe('PS Vita');
    expect(role(vita, 'a')).toBe('Croix');
    expect(role(vita, 'y')).toBe('Triangle');
    expect(role(vita, 'lt')).toBeUndefined();
    const ps = schema(infos({ emulateur: 'vita3k', coeur: null, famille: 'playstation' }));
    expect(ps.etat).toBe('connu');
    expect(ps.etat === 'connu' && ps.remarque).toContain('bouton PS');
    expect(schema(infos({ emulateur: 'vita3k', coeur: null, personnalise: true })).etat).toBe('personnalise');
  });

  it('Xenia : la manette de la Xbox 360 telle quelle', () => {
    const x360 = schema(infos({ emulateur: 'xenia', coeur: null, plateforme: 'Microsoft Xbox 360' }));
    expect(x360.console).toBe('Xbox 360');
    expect(role(x360, 'a')).toBe('A');
    expect(role(x360, 'select')).toBe('Back');
    expect(schema(infos({ emulateur: 'xenia', coeur: null, famille: 'playstation' })).lignes[0].nom).toBe('Croix');
  });

  it('Azahar : la 3DS par position, sans les clics des sticks', () => {
    const t = schema(infos({ emulateur: 'azahar', coeur: null, plateforme: 'Nintendo 3DS' }));
    expect(t.etat).toBe('connu');
    expect(t.console).toBe('3DS');
    expect(role(t, 'a')).toBe('B');
    expect(role(t, 'b')).toBe('A');
    expect(role(t, 'lt')).toBe('ZL');
    expect(role(t, 'select')).toBe('Select');
    expect(role(t, 'l3')).toBeUndefined();
    expect(role(t, 'stickg')).toBe('Pad circulaire');
  });

  it('xemu : la manette Xbox d’origine (Blanc et Noir), et son menu sur le bouton Xbox', () => {
    const xbox = schema(infos({ emulateur: 'xemu', coeur: null, plateforme: 'Microsoft Xbox' }));
    expect(xbox.etat).toBe('connu');
    expect(xbox.console).toBe('Xbox');
    expect(role(xbox, 'lb')).toBe('Blanc');
    expect(role(xbox, 'rb')).toBe('Noir');
    expect(xbox.etat === 'connu' && xbox.remarque).toContain('menu d’xemu');
    expect(schema(infos({ emulateur: 'xemu', coeur: null, personnalise: true })).etat).not.toBe('connu');
  });

  it('PPSSPP : sa disposition d’origine pour une manette Xbox, rien de deviné pour une PlayStation', () => {
    const psp = schema(infos({ emulateur: 'ppsspp', coeur: null, plateforme: 'Sony PSP' }));
    expect(psp.etat).toBe('connu');
    expect(psp.console).toBe('PSP');
    expect(role(psp, 'a')).toBe('Croix');
    expect(role(psp, 'b')).toBe('Rond');
    expect(role(psp, 'x')).toBe('Carré');
    expect(role(psp, 'y')).toBe('Triangle');
    expect(role(psp, 'lt')).toContain('Pause');
    expect(role(psp, 'stickd')).toBeUndefined();
    expect(psp.etat === 'connu' && psp.remarque).toContain('bouton Xbox');
    expect(schema(infos({ emulateur: 'ppsspp', coeur: null, famille: 'playstation' })).etat).toBe('inconnu');
    expect(schema(infos({ emulateur: 'ppsspp', coeur: null, personnalise: true })).etat).toBe('personnalise');
  });

  it('MAME autonome : sa disposition d’origine XInput, rien de deviné pour une PlayStation', () => {
    const m = schema(infos({ emulateur: 'mame', coeur: null, plateforme: 'Arcade' }));
    expect(m.etat).toBe('connu');
    expect(m.console).toBe('l’arcade');
    expect(role(m, 'a')).toBe('Bouton 1');
    expect(role(m, 'b')).toBe('Bouton 2');
    expect(role(m, 'lb')).toBe('Bouton 5');
    expect(role(m, 'r3')).toBe('Bouton 8');
    expect(role(m, 'select')).toBe('Pièce');
    expect(role(m, 'lt')).toBeUndefined();
    expect(m.etat === 'connu' && m.remarque).toContain('propre disposition');
    expect(schema(infos({ emulateur: 'mame', coeur: null, famille: 'playstation' })).etat).toBe('inconnu');
    expect(schema(infos({ emulateur: 'mame', coeur: null, personnalise: true })).etat).toBe('personnalise');
    // Le cœur MAME de RetroArch garde la sienne (LB = Bouton 5 par la RetroPad).
    expect(role(schema(infos({ coeur: 'MAME', plateforme: 'Arcade' })), 'lb')).toBe('Bouton 5');
  });

  it('DOSBox Staging : le joystick de PC à 4 boutons, rien de deviné pour une PlayStation', () => {
    const d = schema(infos({ emulateur: 'dosbox-staging', coeur: null, plateforme: 'MS-DOS' }));
    expect(d.etat).toBe('connu');
    expect(d.console).toBe('joystick du PC');
    expect(role(d, 'a')).toBe('Bouton 1');
    expect(role(d, 'y')).toBe('Bouton 4');
    expect(role(d, 'stickg')).toBe('Joystick');
    expect(role(d, 'lb')).toBeUndefined();
    expect(role(d, 'croix')).toBeUndefined();
    expect(role(d, 'stickd')).toBeUndefined();
    expect(d.etat === 'connu' && d.remarque).toContain('clavier');
    expect(schema(infos({ emulateur: 'dosbox-staging', coeur: null, famille: 'playstation' })).etat).toBe('inconnu');
    expect(schema(infos({ emulateur: 'dosbox-staging', coeur: null, inconnue: true })).etat).toBe('inconnu');
    // Pendant une partie, un émulateur sans nom (hors catalogue, MAME sans son script) n'est pas un jeu PC.
    expect(schema(infos({ emulateur: null, coeur: null, inconnue: true })).etat).toBe('inconnu');
  });

  it('les émulateurs à part : le profil écrit par Frogtend', () => {
    const gc = schema(infos({ emulateur: 'dolphin', coeur: null, plateforme: 'Nintendo GameCube' }));
    expect(role(gc, 'x')).toBe('B');
    expect(role(gc, 'b')).toBe('X');
    expect(role(gc, 'rb')).toBe('Z');
    expect(role(schema(infos({ emulateur: 'duckstation', coeur: null, famille: 'playstation' })), 'a')).toBe('Croix');
    expect(schema(infos({ emulateur: 'duckstation', coeur: null, famille: 'playstation' })).lignes[0].nom).toBe('Croix');
    const ps3 = schema(infos({ emulateur: 'rpcs3', coeur: null, plateforme: 'Sony Playstation 3' }));
    expect(ps3.console).toBe('PlayStation 3');
    expect(role(ps3, 'a')).toBe('Croix');
    expect(role(ps3, 'y')).toBe('Triangle');
    expect(role(ps3, 'select')).toBe('Select');
    // Eden, Eglantine : par position, comme sur une Switch (le A de la Switch est à droite : le B de la Xbox).
    const s2 = schema(infos({ emulateur: 'eglantine', coeur: null, plateforme: 'Nintendo Switch 2' }));
    expect(s2.console).toBe('Switch 2');
    expect(role(s2, 'b')).toBe('A');
    expect(role(s2, 'a')).toBe('B');
    expect(role(s2, 'lt')).toBe('ZL');
    expect(role(schema(infos({ emulateur: 'eden', coeur: null, plateforme: 'Nintendo Switch' })), 'start')).toBe('+');
    const lichii = schema(infos({ emulateur: 'lichii', coeur: null, plateforme: 'Nintendo Switch 2' }));
    expect(lichii.console).toBe('Switch 2');
    expect(role(lichii, 'b')).toBe('A');
  });

  it('un jeu PC : la correspondance manette → clavier', () => {
    const s = schema(infos({ emulateur: null, coeur: null, plateforme: 'Windows', clavier: { a: 'Espace', start: 'Échap', haut: '↑', bas: '↓', gauche: '←', droite: '→' } }));
    expect(role(s, 'a')).toBe('touche Espace');
    expect(role(s, 'start')).toBe('touche Échap');
    expect(role(s, 'croix')).toBe('touches ↑ ↓ ← →');
    const sans = schema(infos({ emulateur: null, coeur: null, clavier: null }));
    expect(sans.etat).toBe('jeu-pc');
    expect(sansDessin(sans)).toContain('Jouer à la manette');
    expect(schema(infos({ emulateur: null, coeur: null, clavier: { a: '' } })).etat).toBe('jeu-pc');
  });

  it('les raccourcis de Frogtend, d’après les réglages et la manette', () => {
    expect(nomCombinaison('Select + R1', 'xbox')).toBe('Vue (⧉) + RB');
    expect(nomCombinaison('L3 + R3', 'playstation')).toBe('L3 + R3');
    expect(nomCombinaison('Select + Start', 'playstation')).toBe('Créer + Options');
    const l = raccourcis(infos({}));
    expect(l.map((x) => x.effet)).toEqual(['ouvrir ou fermer le menu de Frogtend', 'ouvrir le menu de Frogtend', 'ouvrir le menu de Frogtend', 'sauvegarde rapide', 'chargement rapide (la partie revient à la sauvegarde)']);
    expect(l[2].geste).toContain('Pause/Attn');
    expect(l[3].geste).toBe('Vue (⧉) + LT, tenus ½ seconde');
    const sans = raccourcis(infos({ raccourcis: { combinaison: 'L3 + R3', bouton_xbox: false, touche: '', etats: false } }));
    expect(sans).toEqual([{ geste: 'L3 + R3, tenus 1 seconde', effet: 'ouvrir le menu de Frogtend' }]);
  });
});
