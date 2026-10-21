import { describe, expect, it } from 'vitest';
import type { Conseil } from '$lib/api';
import { avecConseil, changementDeConseil, memeEmulateur, biosADire, biosRetroArch, choisirSeul, dossierParDefaut, estConseille, ligneDe, messagePret, micrologicielServi, palier, poidsBios, versionAutre, versionDansNote, type Possible } from './pret';

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
    // Contrat 1.16 : la ligne COMPLÈTE d'un autonome, seule — jamais doublée des options de Frogtend.
    const pcsx2 = ligneLb('PCSX2', 'pcsx2', true);
    expect(ligneDe(pcsx2, '-batch -fullscreen --', { ...conseilSnes, emulateur: 'pcsx2', ligne_de_commande: '-batch -fullscreen -- "%rom%"' })).toBe('-batch -fullscreen -- "%rom%"');
    expect(ligneDe(ligneLb('Dolphin', 'dolphin', true), '-b -e', { ...conseilSnes, emulateur: 'dolphin', ligne_de_commande: '-b -e "%rom%"' })).toBe('-b -e "%rom%"');
    // Un autonome (ligne du conseil vide) : seulement les options de Frogtend (RPCS3 : sans interface, plein écran).
    const rpcs3 = ligneLb('RPCS3', 'rpcs3', true, '--lb');
    expect(ligneDe(rpcs3, '--no-gui --fullscreen', { ...conseilSnes, emulateur: 'rpcs3', ligne_de_commande: '' })).toBe('--no-gui --fullscreen');
    // Sans conseil, ou un autre émulateur choisi : comme avant (LaunchBox, sinon Frogtend).
    expect(ligneDe(ra, '-f', null)).toBe('-L "cores\\bsnes_libretro.dll"');
    expect(ligneDe(ligneLb('Mesen', 'mesen', false), '-x', conseilSnes)).toBe('-x');
  });

  it('contrat 1.16 : la ligne du conseil de chaque autonome porte déjà toutes les options de Frogtend', () => {
    // Relevé du 19/10 (section 1.16 du contrat) face au catalogue de Frogtend (src-tauri/src/emulateurs.rs, champ
    // `ligne` de CATALOGUE — à recopier ici s'il change). Si un jour une ligne du conseil oubliait une option de
    // Frogtend, ce test le dirait (elle serait perdue sans bruit).
    const tableau: [string, string, string][] = [
      ['dolphin', '-b -e', '-b -e "%rom%"'],
      ['pcsx2', '-batch -fullscreen --', '-batch -fullscreen -- "%rom%"'],
      ['rpcs3', '--no-gui --fullscreen', '--no-gui --fullscreen "%rom%"'],
      ['cemu', '-f -g', '-f -g "%rom%"'],
      ['eden', '-f -g', '-f -g "%rom%"'],
      ['xenia_canary', '--fullscreen', '--fullscreen "%rom%"'],
      ['xemu', '-dvd_path', '-full-screen -dvd_path "%rom%"'],
    ];
    for (const [id, frogtend, ligne] of tableau) {
      const conseil = { ...conseilSnes, emulateur: id, coeur: null, ligne_de_commande: ligne };
      expect(ligneDe(ligneLb(id, id, true), frogtend, conseil)).toBe(ligne);
      for (const o of frogtend.split(' ')) expect(ligne.split(' ')).toContain(o);
    }
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

  it('l’avancement se montre par pas de 10 %', () => {
    expect(palier(0, 206197916)).toBe(0);
    expect(palier(103098958, 206197916)).toBe(50);
    expect(palier(206197916, 206197916)).toBe(100);
    expect(palier(5, null)).toBeNull();
  });

  it('l’accord dit le micrologiciel PS3 fourni (et seulement pour RPCS3)', () => {
    const conseilPs3: Conseil = {
      ...conseilSnes,
      plateforme: 'Sony Playstation 3',
      emulateur: 'rpcs3',
      coeur: null,
      ligne_de_commande: '',
      // Relevé réel du 16/10.
      bios: [{ chemin: '', coffre: false, disponible: true, facultatif: false, fichier: 'PS3UPDAT.PUP', note: 'firmware 4.93', racine: 'installation_par_l_emulateur', sha1: 'ae2a4f49ec6c09d124fed20f84231ce27d10c7b7', taille: 206197916, type: 'installation', url: '/api/jeux/v1/bios/ae2a4f49ec6c09d124fed20f84231ce27d10c7b7?nom=PS3UPDAT.PUP' }],
    };
    expect(micrologicielServi(conseilPs3.bios)?.fichier).toBe('PS3UPDAT.PUP');
    expect(versionDansNote('firmware 4.93 récupéré chez Sony (04/10) — installé par le menu de RPCS3')).toBe('4.93');
    expect(versionDansNote('clés : dans le coffre')).toBeNull();
    expect(micrologicielServi(conseilSwitch.bios)).toBeUndefined();
    const choix = { rec: ligneLb('RPCS3', 'rpcs3', true), id: 'rpcs3', nom: 'RPCS3', comment: 'frogtend' as const };
    const m = messagePret({ plateforme: 'Sony Playstation 3', jeux: 7, choix, taille: null, dossier: 'E:\\Emulateurs', dossierNouveau: false, conseil: conseilPs3 });
    expect(m).toContain('le micrologiciel PS3 (197 Mo), fourni par Firehouse');
    expect(m).not.toContain('fichier système');
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

describe('la version attendue par le conseil (contrat 1.13)', () => {
  const c = { version_attendue: '0.289' } as Conseil;
  it('ne dit rien quand c’est la bonne, ou quand on ne sait pas', () => {
    expect(versionAutre('0.289', c)).toBeNull();
    expect(versionAutre('0.289 (mame0289)', c)).toBeNull();
    expect(versionAutre(null, c)).toBeNull();
    expect(versionAutre('0.250', { version_attendue: null } as Conseil)).toBeNull();
  });
  it('dit la version attendue quand c’en est une autre', () => {
    expect(versionAutre('0.250', c)).toBe('0.289');
    expect(versionAutre('0.2891', c)).toBe('0.289');
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
    expect(dossierParDefaut(['E:\\Jeux'])).toBe('E:\\Frogtend\\Emulateurs');
    expect(dossierParDefaut(['d:\\Ludo\\Jeux'], 'C:\\')).toBe('D:\\Frogtend\\Emulateurs');
    // Aucun dossier de jeux : le disque proposé par le cœur (il existe sur CE PC) ; aucun : rien d'inventé.
    expect(dossierParDefaut([], 'F:\\')).toBe('F:\\Frogtend\\Emulateurs');
    expect(dossierParDefaut([])).toBe('');
    // Un dossier de jeux restauré d'un autre PC (E: n'existe pas ici) : le disque proposé, jamais E:.
    expect(dossierParDefaut(['E:\\Jeux'], 'C:\\', ['C:\\'])).toBe('C:\\Frogtend\\Emulateurs');
    expect(dossierParDefaut(['e:\\Jeux'], 'C:\\', ['C:\\', 'E:\\'])).toBe('E:\\Frogtend\\Emulateurs');
    expect(dossierParDefaut([], null)).toBe('');
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

describe('le conseil de la maison a changé (contrat 1.8, maj_le)', () => {
  const c = (o: Partial<Conseil>) =>
    ({ emulateur: 'retroarch', coeur: 'snes9x_libretro', ligne_de_commande: '-L "cores\\snes9x_libretro.dll" "%rom%"', maj_le: '2026-10-18T10:00:00', bios: [], alternatives: [], ...o }) as Conseil;
  const ra = { programme: String.raw`E:\Emulateurs\RetroArch\retroarch.exe`, ligne: '-L "cores\\snes9x_libretro.dll" "%rom%"', conseil: '2026-10-04T17:23:15' };
  it('reconnaît le même émulateur sous ses deux noms', () => {
    expect(memeEmulateur(String.raw`E:\PCSX2\pcsx2-qt.exe`, 'pcsx2')).toBe(true);
    expect(memeEmulateur(String.raw`E:\DOSBox Staging\dosbox.exe`, 'dosbox_staging')).toBe(true);
    expect(memeEmulateur(ra.programme, 'retroarch')).toBe(true);
    expect(memeEmulateur(ra.programme, 'duckstation')).toBe(false);
  });
  it('ne dit rien pour un réglage fait à la main, ni pour un conseil inchangé', () => {
    expect(changementDeConseil({ ...ra, conseil: undefined }, c({}), '')).toBeNull();
    expect(changementDeConseil({ ...ra, conseil: '2026-10-18T10:00:00' }, c({}), '')).toBeNull();
    expect(changementDeConseil(ra, null, '')).toBeNull();
  });
  it('nouvelle date, même réglage : seulement retenir la date', () => {
    expect(changementDeConseil(ra, c({}), '')).toEqual({ sorte: 'date' });
  });
  it('un autre cœur : la nouvelle ligne est proposée', () => {
    const r = changementDeConseil(ra, c({ coeur: 'bsnes_libretro', ligne_de_commande: '-L "cores\\bsnes_libretro.dll" "%rom%"' }), '');
    expect(r).toEqual({ sorte: 'ligne', ligne: '-L "cores\\bsnes_libretro.dll" "%rom%"' });
  });
  it('un autre émulateur conseillé : à proposer', () => {
    expect(changementDeConseil(ra, c({ emulateur: 'mesen' }), '')).toEqual({ sorte: 'autre' });
  });
  it('contrat 1.16 : un autonome réglé avant (options de Frogtend seules) garde son réglage, rien à demander', () => {
    const dolphin = { programme: String.raw`E:\Dolphin\Dolphin.exe`, ligne: '-b -e', conseil: '2026-10-04T17:23:15' };
    const c16 = c({ emulateur: 'dolphin', coeur: null, ligne_de_commande: '-b -e "%rom%"', maj_le: '2026-10-19T00:11:00' });
    expect(changementDeConseil(dolphin, c16, '-b -e')).toEqual({ sorte: 'date' });
    // xemu : le conseil AJOUTE le plein écran → proposé.
    const xemu = { programme: String.raw`E:\xemu\xemu.exe`, ligne: '-dvd_path', conseil: '2026-10-04T17:23:15' };
    const cx = c({ emulateur: 'xemu', coeur: null, ligne_de_commande: '-full-screen -dvd_path "%rom%"', maj_le: '2026-10-19T00:11:00' });
    expect(changementDeConseil(xemu, cx, '-dvd_path')).toEqual({ sorte: 'ligne', ligne: '-full-screen -dvd_path "%rom%"' });
    // MAME arcade réglé avant (ligne vide) : la commande reste la même (Frogtend construit nom court et rompath).
    const mame = { programme: String.raw`E:\MAME\mame.exe`, ligne: '', conseil: '2026-10-05T10:00:00' };
    const cm = c({ emulateur: 'mame', coeur: null, ligne_de_commande: '%rom_nom% -rompath "%rom_dossier%"', maj_le: '2026-10-19T00:11:00' });
    expect(changementDeConseil(mame, cm, '')).toEqual({ sorte: 'date' });
  });
  it('contrat 1.16 : une ligne doublée par Frogtend lui-même (préparée le 19/10 avant l’adaptation) est réparée', () => {
    const c16 = c({ emulateur: 'dolphin', coeur: null, ligne_de_commande: '-b -e "%rom%"', maj_le: '2026-10-19T00:11:00' });
    // Même date (préparé APRÈS le changement) : réparé quand même, sans question.
    const doublee = { programme: String.raw`E:\Dolphin\Dolphin.exe`, ligne: '-b -e -b -e "%rom%"', conseil: '2026-10-19T00:11:00' };
    expect(changementDeConseil(doublee, c16, '-b -e')).toEqual({ sorte: 'reparer', ligne: '-b -e "%rom%"' });
    // Une ligne choisie par la personne n'est jamais touchée.
    const perso = { ...doublee, ligne: '-b -e --debugger "%rom%"' };
    expect(changementDeConseil(perso, c16, '-b -e')).toBeNull();
  });
});
