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
    expect(schema(infos({ emulateur: 'cemu', coeur: null })).etat).toBe('inconnu');
    expect(schema(infos({ emulateur: 'dolphin', coeur: null, plateforme: 'Nintendo Wii' })).etat).toBe('inconnu');
    const p = schema(infos({ personnalise: true }));
    expect(p.etat).toBe('personnalise');
    expect(p.lignes).toEqual([]);
    expect(sansDessin(p)).toContain('changé toi-même');
    expect(sansDessin(schema(infos({ coeur: 'X' })))).toContain('ne connaît pas encore');
    expect(sansDessin(schema(infos({})))).toBeNull();
  });

  it('les émulateurs à part : le profil écrit par Frogtend', () => {
    const gc = schema(infos({ emulateur: 'dolphin', coeur: null, plateforme: 'Nintendo GameCube' }));
    expect(role(gc, 'x')).toBe('B');
    expect(role(gc, 'b')).toBe('X');
    expect(role(gc, 'rb')).toBe('Z');
    expect(role(schema(infos({ emulateur: 'duckstation', coeur: null, famille: 'playstation' })), 'a')).toBe('Croix');
    expect(schema(infos({ emulateur: 'duckstation', coeur: null, famille: 'playstation' })).lignes[0].nom).toBe('Croix');
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
