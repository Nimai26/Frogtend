import { describe, expect, it } from 'vitest';
import { couper, depuisDos, depuisMame, depuisManuel, depuisProgramme, depuisRoms, extensionsDe, lireExtensions, messageBilan } from './local';

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

  it('résume un import', () => {
    expect(messageBilan({ ajoutes: 3, deja: 1, refuses: [['X', 'introuvable']] })).toBe('✅ 3 jeu(x) ajouté(s) à ta ludothèque · 1 déjà dedans · 1 refusé(s) : X (introuvable)');
  });
});
