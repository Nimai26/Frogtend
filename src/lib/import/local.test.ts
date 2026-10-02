import { describe, expect, it } from 'vitest';
import { elementDe, emplacementDuSysteme, dansUnEmplacement, avecSource, apresCopie, nomDeDossier, depuisInstallationDos, couper, depuisDos, depuisMame, depuisManuel, depuisProgramme, depuisRoms, extensionsDe, lireExtensions, messageBilan } from './local';

describe('import local', () => {
  it('coupe un chemin Windows', () => {
    expect(couper('E:\\ROM\\SNES\\Mario (USA).sfc')).toEqual({ dossier: 'E:\\ROM\\SNES', nom: 'Mario (USA).sfc' });
    expect(couper('jeu.sfc')).toEqual({ dossier: '', nom: 'jeu.sfc' });
  });

  it('lit les extensions des émulateurs et celles tapées', () => {
    expect(extensionsDe([{ extensions: ['.SFC', 'smc'] }, { extensions: ['sfc', 'zip'] }, {}])).toEqual(['sfc', 'smc', 'zip']);
    expect(lireExtensions('sfc, .smc  *.zip;sfc')).toEqual(['sfc', 'smc', 'zip']);
  });

  it('garde le nom de la ROM tel quel', () => {
    const l = depuisRoms([{ chemin: 'E:\\ROM\\Mario (USA).sfc', titre: 'Mario', taille: 1 }], 'Super Nintendo Entertainment System');
    expect(l[0]).toEqual({ titre: 'Mario', plateforme: 'Super Nintendo Entertainment System', dossier: 'E:\\ROM', fichier: 'Mario (USA).sfc' });
  });

  it('prépare les jeux DOS, Windows et manuels', () => {
    expect(depuisDos([{ dossier: 'D:\\DOS\\Dune', titre: 'Dune', programmes: ['DUNE.EXE'], programme: 'DUNE.EXE' }])[0].fichier).toBe('DUNE.EXE');
    expect(depuisProgramme('D:\\Jeux\\Doom\\doom.exe')).toEqual({ titre: 'Doom', plateforme: 'Windows', dossier: 'D:\\Jeux\\Doom', programme: 'D:\\Jeux\\Doom\\doom.exe' });
    expect(depuisProgramme('D:\\Jeux\\Doom\\doom.exe', ' Mon Doom ').titre).toBe('Mon Doom');
    expect(depuisManuel('Zelda', 'Nintendo 64', 'E:\\N64\\zelda.z64')).toEqual({ titre: 'Zelda', plateforme: 'Nintendo 64', dossier: 'E:\\N64', fichier: 'zelda.z64' });
    expect(depuisManuel('Doom', 'Windows', 'D:\\Doom\\doom.exe').programme).toBe('D:\\Doom\\doom.exe');
  });

  it('prépare les jeux MAME retenus', () => {
    const l = depuisMame([{ chemin: 'E:\\Games\\MAME\\pacman.zip', titre: 'Pac-Man (Midway)', taille: 1, annee: 1980, editeur: 'Namco', genre: 'Maze' }]);
    expect(l[0]).toEqual({ titre: 'Pac-Man (Midway)', plateforme: 'Arcade', dossier: 'E:\\Games\\MAME', fichier: 'pacman.zip', annee: 1980, editeur: 'Namco', genres: ['Maze'] });
  });

  it('prépare un jeu DOS installé', () => {
    expect(nomDeDossier("King's Quest V: Absence?  ")).toBe("King's Quest V - Absence");
    expect(nomDeDossier('A/B\\C.')).toBe('ABC');
    const j = depuisInstallationDos(' KQ5 ', 'D:\\DOS\\KQ5', 'C:\\DOSBox\\dosbox.exe', ['-c', 'exit']);
    expect(j).toEqual({ titre: 'KQ5', plateforme: 'MS-DOS', dossier: 'D:\\DOS\\KQ5', programme: 'C:\\DOSBox\\dosbox.exe', arguments: ['-c', 'exit'] });
  });

  it('sait où garder les jeux importés', () => {
    const e = { defaut: ['D:\\Jeux'], systemes: { 'MS-DOS': ['E:\\DOS'] } };
    const rom = { titre: 'Mario', plateforme: 'Nintendo Entertainment System', dossier: 'E:\\Games\\NES\\USA', fichier: 'Mario (U).nes' };
    expect(elementDe(rom)).toBe('E:\\Games\\NES\\USA\\Mario (U).nes');
    expect(elementDe({ ...rom, dossier: 'D:\\DOS\\Dune', fichier: 'BIN/DUNE.EXE' })).toBe('D:\\DOS\\Dune');
    expect(elementDe({ titre: 'Doom', plateforme: 'Windows', dossier: 'F:\\Doom', programme: 'F:\\Doom\\doom.exe' })).toBe('F:\\Doom');
    expect(emplacementDuSysteme(e, 'MS-DOS', (t) => t)).toBe('E:\\DOS');
    expect(emplacementDuSysteme(e, 'Nintendo 64', (t) => t)).toBe('D:\\Jeux\\Nintendo 64');
    expect(emplacementDuSysteme({ defaut: [], systemes: {} }, 'X', (t) => t)).toBeNull();
    expect(dansUnEmplacement('d:/jeux/NES/a.nes', e, 'NES')).toBe(true);
    expect(dansUnEmplacement('D:\\Jeux2\\a.nes', e, 'NES')).toBe(false);
    expect(avecSource(e, 'Nintendo Entertainment System', 'E:\\Games\\NES')).toEqual({ 'MS-DOS': ['E:\\DOS'], 'Nintendo Entertainment System': ['E:\\Games\\NES'] });
    expect(avecSource(e, 'MS-DOS', 'E:\\DOS\\Sous')).toBeNull();
  });

  it('fait pointer chaque jeu vers sa copie', () => {
    const rom = { titre: 'Mario', plateforme: 'NES', dossier: 'E:\\Games\\NES\\USA', fichier: 'Mario (U).nes' };
    const doom = { titre: 'Doom', plateforme: 'Windows', dossier: 'F:\\Doom', programme: 'F:\\Doom\\bin\\doom.exe' };
    const [a, b] = apresCopie([rom, doom], ['D:\\Jeux\\NES\\Mario (U).nes', 'D:\\Jeux\\Windows\\Doom']);
    expect(a).toEqual({ ...rom, dossier: 'D:\\Jeux\\NES' });
    expect(b).toEqual({ ...doom, dossier: 'D:\\Jeux\\Windows\\Doom', programme: 'D:\\Jeux\\Windows\\Doom\\bin\\doom.exe' });
  });

  it('résume un import', () => {
    expect(messageBilan({ ajoutes: 2, versions: 3, deja: 0, refuses: [] })).toBe('✅ 2 jeu(x) ajouté(s) à ta ludothèque · 3 version(s) ajoutée(s) à des jeux déjà là');
    expect(messageBilan({ ajoutes: 3, deja: 1, refuses: [['X', 'introuvable']] })).toBe('✅ 3 jeu(x) ajouté(s) à ta ludothèque · 1 déjà dedans · 1 refusé(s) : X (introuvable)');
  });
});
