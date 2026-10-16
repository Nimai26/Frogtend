import { describe, expect, it } from 'vitest';
import type { Conseil } from '$lib/api';
import { avecConseil, biosADire, biosRetroArch, choisirSeul, dossierParDefaut, estConseille, ligneDe, messagePret, poidsBios, type Possible } from './pret';

const rec = (nom: string, recommande: boolean, bios = '') => ({ nom, site: '', recommande, ligne_de_commande: '', extensions: [], bios });

// Relevés RÉELS du 16/10 (Firehouse 2.67.x, contrat 1.13), réduits à ce qui sert.
const conseilSnes: Conseil = {
  plateforme: 'Super Nintendo Entertainment System',
  emulateur: 'retroarch',
  coeur: 'snes9x_libretro',
  ligne_de_commande: '-L "cores\\snes9x_libretro.dll" "%rom%"',
  alternatives: [{ coeur: null, emulateur: 'mesen', nom: 'MesenCE', quand: '', site: '' }],
  bios: [],
  version_attendue: null,
  note: '',
  origine: 'amorce',
  maj_le: '2026-10-04T13:27:38',
};
const conseilPsx: Conseil = {
  ...conseilSnes,
  plateforme: 'Sony Playstation',
  coeur: 'mednafen_psx_hw_libretro',
  ligne_de_commande: '-L "cores\\mednafen_psx_hw_libretro.dll" "%rom%"',
  bios: [
    { chemin: 'scph5500.bin', coffre: false, disponible: true, facultatif: false, fichier: 'scph5500.bin', racine: 'system_retroarch', type: 'fichier', sha1: 'b05def971d8ec59f346f2d9ac21fb742e3eb6917', url: '/api/jeux/v1/bios/b05def971d8ec59f346f2d9ac21fb742e3eb6917?nom=scph5500.bin' },
    { chemin: 'scph5501.bin', coffre: false, disponible: true, facultatif: false, fichier: 'scph5501.bin', racine: 'system_retroarch', type: 'fichier', sha1: '0555c6fae8906f3f09baf5988f00e55f88e9f30b', url: '/api/jeux/v1/bios/0555c6fae8906f3f09baf5988f00e55f88e9f30b?nom=scph5501.bin' },
  ],
};
const conseilSwitch: Conseil = {
  ...conseilSnes,
  plateforme: 'Nintendo Switch',
  emulateur: 'eden',
  coeur: null,
  ligne_de_commande: '',
  bios: [
    { chemin: 'prod.keys', coffre: true, disponible: false, facultatif: false, fichier: 'prod.keys', racine: 'dossier_emulateur', type: 'fichier' },
    { chemin: '', coffre: false, disponible: true, facultatif: false, fichier: 'firmware-switch.zip', racine: 'installation_par_l_emulateur', type: 'installation' },
  ],
};
const ligneLb = (nom: string, id: string, recommande: boolean, ligne = '', bios = '') => ({
  nom,
  site: '',
  recommande,
  ligne_de_commande: ligne,
  extensions: [],
  bios,
  id,
});

describe('le conseil de la maison (contrat 1.8)', () => {
  it('l’émulateur du conseil passe devant l’avis de LaunchBox ; sans conseil, rien ne change', () => {
    const snes9x: Possible = { rec: ligneLb('Snes9x', 'snes9x', true), id: null, nom: 'Snes9x', parFirehouse: true };
    const retroarch: Possible = { rec: ligneLb('RetroArch', 'retroarch', false), id: 'retroarch', nom: 'RetroArch' };
    expect(choisirSeul([snes9x, retroarch], conseilSnes)?.nom).toBe('RetroArch');
    expect(choisirSeul([snes9x, retroarch], null)?.nom).toBe('Snes9x');
    // Le conseillé n'est pas préparable par Frogtend : le suivant de la liste.
    const inconnu: Possible = { rec: ligneLb('RetroArch', 'retroarch', false), id: null, nom: 'RetroArch' };
    expect(choisirSeul([inconnu, snes9x], conseilSnes)?.nom).toBe('Snes9x');
    expect(estConseille(ligneLb('X', 'retroarch', false), conseilSnes)).toBe(true);
    expect(estConseille({ ...ligneLb('X', 'retroarch', false), id: undefined }, conseilSnes)).toBe(false);
  });

  it('la ligne : les options de Frogtend puis celle du conseil, jamais celle de LaunchBox', () => {
    const ra = ligneLb('RetroArch', 'retroarch', true, '-L "cores\\bsnes_libretro.dll"');
    expect(ligneDe(ra, '-f', conseilSnes)).toBe('-L "cores\\snes9x_libretro.dll" -f "%rom%"');
    // Une option de Frogtend qui attend le fichier du jeu (« -- », « -e ») reste collée au jeu.
    const pcsx2 = ligneLb('PCSX2', 'pcsx2', true);
    expect(ligneDe(pcsx2, '-batch -fullscreen --', { ...conseilSnes, emulateur: 'pcsx2', ligne_de_commande: '-option "%rom%"' })).toBe('-option -batch -fullscreen -- "%rom%"');
    expect(ligneDe(ligneLb('Dolphin', 'dolphin', true), '-b -e', { ...conseilSnes, emulateur: 'dolphin', ligne_de_commande: '-x' })).toBe('-x -b -e');
    // Un autonome (ligne du conseil vide) : seulement les options de Frogtend (RPCS3 : sans interface, plein écran).
    const rpcs3 = ligneLb('RPCS3', 'rpcs3', true, '--lb');
    expect(ligneDe(rpcs3, '--no-gui --fullscreen', { ...conseilSnes, emulateur: 'rpcs3', ligne_de_commande: '' })).toBe('--no-gui --fullscreen');
    // Sans conseil, ou un autre émulateur choisi : comme avant (LaunchBox, sinon Frogtend).
    expect(ligneDe(ra, '-f', null)).toBe('-L "cores\\bsnes_libretro.dll"');
    expect(ligneDe(ligneLb('Mesen', 'mesen', false), '-x', conseilSnes)).toBe('-x');
  });

  it('les fichiers système : ceux du conseil, jamais la chaîne LaunchBox', () => {
    const ra = ligneLb('RetroArch', 'retroarch', true, '', 'chaîne LaunchBox');
    expect(biosRetroArch(ra, conseilPsx)).toEqual(['scph5500.bin', 'scph5501.bin']);
    expect(biosRetroArch(ra, null)).toEqual(['chaîne LaunchBox']);
    expect(biosRetroArch(ra, { ...conseilPsx, bios: [{ ...conseilPsx.bios[0], facultatif: true }] })).toEqual([]);
    expect(biosADire(conseilSwitch.bios)).toEqual({ fournis: [], aPoser: [], cles: ['prod.keys'] });
    expect(biosADire(conseilPsx.bios).fournis).toEqual(['scph5500.bin', 'scph5501.bin']);
    // Le poids annoncé : les obligatoires servis seulement (ni clé, ni firmware, ni facultatif).
    expect(poidsBios(conseilPsx.bios.map((b) => ({ ...b, taille: 524288 })))).toBe(1048576);
    expect(poidsBios(conseilSwitch.bios.map((b) => ({ ...b, taille: 336375526 })))).toBe(0);
    // Ce que Firehouse ne sert pas (disponible: false) n'est jamais promis ; un motif se dit par sa note.
    const xbox = [{ fichier: '*.bin', type: 'motif' as const, racine: 'dossier_emulateur', chemin: '*.bin', disponible: false, fichiers: null, note: 'BIOS flash (Complex 4627)' }];
    expect(biosADire(xbox)).toEqual({ fournis: [], aPoser: ['BIOS flash (Complex 4627)'], cles: [] });
    // Un motif servi n'est pas encore posé par Frogtend : jamais promis comme « fourni ».
    expect(biosADire([{ ...xbox[0], disponible: true }]).fournis).toEqual([]);
    // Avec un conseil, la chaîne LaunchBox n'a plus cours, même pour un autre émulateur.
    expect(biosRetroArch(ligneLb('Autre', 'autre', false, '', 'x.bin'), conseilPsx)).toEqual([]);
  });

  it('l’accord unique parle des fichiers du conseil et des clés manquantes', () => {
    const choix = { rec: ligneLb('Eden', 'eden', true, '', 'ignoré'), id: 'eden', nom: 'Eden', comment: 'frogtend' as const };
    const m = messagePret({ plateforme: 'Nintendo Switch', jeux: 1, choix, taille: null, dossier: 'E:\\Emulateurs', dossierNouveau: false, conseil: conseilSwitch });
    expect(m).toContain('tes clés (prod.keys)');
    expect(m).not.toContain('ignoré');
    const psx = { ...choix, rec: ligneLb('RetroArch', 'retroarch', true), id: 'retroarch', nom: 'RetroArch' };
    expect(messagePret({ plateforme: 'Sony Playstation', jeux: 2, choix: psx, taille: null, dossier: 'E:\\Emulateurs', dossierNouveau: false, conseil: conseilPsx })).toContain(
      'des fichiers système (scph5500.bin, scph5501.bin) : Firehouse les fournit, Frogtend les pose pour toi (vérifié)',
    );
  });
});

describe('la liste complétée par le conseil', () => {
  it('le conseillé en tête : de la liste, d’un fork, ou décrit d’après le conseil s’il manque', () => {
    // Relevé réel du 16/10 : MS-DOS conseille RetroArch, la liste ne donne que DOSBox Staging.
    const dosbox = ligneLb('DOSBox Staging', 'dosbox_staging', false);
    const l = avecConseil([dosbox], { ...conseilSnes, plateforme: 'MS-DOS', emulateur: 'retroarch' });
    expect(l.map((r) => r.id)).toEqual(['retroarch', 'dosbox_staging']);
    expect(l[0].ligne_de_commande).toBe('');
    // Switch : conseil Eden, liste vide.
    expect(avecConseil([], conseilSwitch).map((r) => r.id)).toEqual(['eden']);
    // Porté par un fork (même inactif : le conseil prime).
    const yuzu = { ...ligneLb('Yuzu', 'yuzu', true), forks: [{ ...ligneLb('Eden', 'eden', false), actif: false }] };
    expect(avecConseil([yuzu], conseilSwitch)[0].nom).toBe('Eden');
    // Le reste garde l'avis LaunchBox ; sans conseil, rien ne bouge.
    const a = ligneLb('A', 'a', false);
    const b = ligneLb('B', 'b', true);
    expect(avecConseil([a, b], conseilSnes).map((r) => r.id)).toEqual(['retroarch', 'b', 'a']);
    expect(avecConseil([a, b], null)).toEqual([a, b]);
  });
});

describe('prêt à jouer', () => {
  it('choisit seul l’émulateur : déjà là, sinon installable, le recommandé d’abord', () => {
    const rpcs3: Possible = { rec: rec('RPCS3', true), id: 'rpcs3', nom: 'RPCS3' };
    const autre: Possible = { rec: rec('Autre', false), id: null, nom: 'Autre' };
    expect(choisirSeul([autre, rpcs3])?.comment).toBe('frogtend');
    expect(choisirSeul([autre, rpcs3])?.nom).toBe('RPCS3');
    expect(choisirSeul([{ ...rpcs3, programme: 'E:\\Emulateurs\\RPCS3\\rpcs3.exe' }])?.comment).toBe('deja');
    expect(choisirSeul([{ ...autre, parFirehouse: true }])?.comment).toBe('firehouse');
    // Rien que Frogtend sache préparer seul : on repassera par l'assistant.
    expect(choisirSeul([autre])).toBeNull();
    expect(choisirSeul([])).toBeNull();
  });

  it('propose le dossier des émulateurs sur le disque des jeux', () => {
    expect(dossierParDefaut(['E:\\Jeux'])).toBe('E:\\Emulateurs');
    expect(dossierParDefaut(['d:\\Ludo\\Jeux'])).toBe('D:\\Emulateurs');
    expect(dossierParDefaut([])).toBe('E:\\Emulateurs');
  });

  it('dit tout dans un seul message', () => {
    const choix = { rec: rec('RPCS3', true), id: 'rpcs3', nom: 'RPCS3', comment: 'frogtend' as const };
    const m = messagePret({ plateforme: 'Sony Playstation 3', jeux: 7, choix, taille: '37,1 Mo', dossier: 'E:\\Emulateurs', dossierNouveau: true });
    expect(m).toContain('tes 7 jeux Sony Playstation 3');
    expect(m).toContain('RPCS3, l’émulateur recommandé (37,1 Mo)');
    expect(m).toContain('Dossier : E:\\Emulateurs\\RPCS3');
    expect(m).toContain('manette');
    expect(m).toContain('première partie');
    const deja = messagePret({ plateforme: 'Nintendo Wii', jeux: 1, choix: { ...choix, id: 'dolphin', nom: 'Dolphin', comment: 'deja' }, taille: null, dossier: 'E:\\Emulateurs', dossierNouveau: false });
    expect(deja).toContain('ce jeu Nintendo Wii');
    expect(deja).toContain('déjà installé');
    expect(deja).not.toContain('Dossier');
    expect(deja).not.toContain('première partie');
    const bios = messagePret({ plateforme: 'Sony Playstation', jeux: 2, choix: { ...choix, id: 'duckstation', rec: rec('DuckStation', true, 'scph5502.bin') }, taille: null, dossier: 'E:\\Emulateurs', dossierNouveau: false });
    expect(bios).toContain('fichier système (scph5502.bin)');
  });
});
