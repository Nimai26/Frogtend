import { describe, expect, it } from 'vitest';
import { choisirSeul, dossierParDefaut, messagePret, type Possible } from './pret';

const rec = (nom: string, recommande: boolean, bios = '') => ({ nom, site: '', recommande, ligne_de_commande: '', extensions: [], bios });

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
