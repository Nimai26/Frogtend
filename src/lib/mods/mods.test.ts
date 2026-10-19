import { describe, expect, it } from 'vitest';
import vraie from '../../../src-tauri/fixtures/simule/triche-ultimate-spider-man.json';
import { adresseWeb, avertissements, fichierInstallable, raisonSansClic, type ModJeu } from './mods';

const mods = (vraie as { mods: ModJeu[] }).mods;
const mod = (nom: string) => mods.find((m) => m.nom === nom)!;

describe('mods (contrat 1.19, vraie réponse d’Ultimate Spider-Man)', () => {
  it('Fusion Fix s’installe d’un clic ; « Skip intro videos » est un réglage à faire', () => {
    expect(fichierInstallable(mod('Fusion Fix'))?.nom).toBe('UltimateSpiderMan.WidescreenFix.zip');
    expect(fichierInstallable(mod('Skip intro videos'))).toBeNull();
    expect(raisonSansClic(mod('Skip intro videos'))).toContain('réglage');
  });

  it('un refus de Firehouse : jamais d’installation, la raison dite simplement', () => {
    const anti: ModJeu = { ...mod('Fusion Fix'), refus: 'anti_triche' };
    expect(fichierInstallable(anti)).toBeNull();
    expect(raisonSansClic(anti)).toContain('bannir');
    expect(raisonSansClic({ nom: 'x', refus: 'sans_release_precise' })).toContain('quel fichier');
    expect(raisonSansClic({ nom: 'x', refus: 'nouveau_motif', refus_detail: 'Motif de Firehouse' })).toBe('Motif de Firehouse');
  });

  it('avant le oui : partage, version d’essai, page déplacée, empreinte absente', () => {
    const f = fichierInstallable(mod('Fusion Fix'))!;
    expect(avertissements(f)).toEqual(['Partage : libre (licence MIT).']);
    expect(avertissements({ ...f, licence: null, licence_libre: null, pre_version: true, depot_deplace: true, sha256: null })).toEqual([
      'Partage : non précisé par l’auteur.',
      '⚠ Version d’essai : l’auteur ne la dit pas terminée.',
      '⚠ L’auteur a déplacé sa page : vérifie que c’est bien le bon mod.',
      '⚠ Frogtend ne peut pas vérifier que le fichier est intact, seulement sa taille.',
    ]);
  });

  it('les références citées ne sont jamais un fichier à installer ; seules les adresses web s’ouvrent', () => {
    const f = mod('Fusion Fix');
    expect(f.references?.length).toBeGreaterThan(0);
    expect((f.fichiers ?? []).some((x) => f.references!.includes(x.url))).toBe(false);
    expect(adresseWeb('https://www.pcgamingwiki.com/wiki/X')).toBe('https://www.pcgamingwiki.com/wiki/X');
    expect(adresseWeb('file:///C:/Windows')).toBeNull();
    expect(adresseWeb(null)).toBeNull();
  });
});
