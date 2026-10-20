import { describe, expect, it } from 'vitest';
import vraie from '../../../src-tauri/fixtures/simule/triche-ultimate-spider-man.json';
import { adresseWeb, avertissements, chiffresMod, fichierInstallable, pageNexus, pourcentMod, raisonSansClic, type ModJeu, type ProgresMod } from './mods';

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

describe('pages Nexus (lot 0.82.0)', () => {
  it('reconnaît la page d’un mod de Nexus de la vraie réponse, et rien d’autre', () => {
    const urls = (vraie.mods as ModJeu[]).map((m) => m.url).filter((u) => pageNexus(u));
    expect(urls.length).toBeGreaterThan(0);
    expect(pageNexus('https://www.nexusmods.com/ultimatespiderman/mods/11')).toEqual({ domaine: 'ultimatespiderman', mod: 11 });
    expect(pageNexus('https://nexusmods.com/skyrim/mods/5?tab=files')).toEqual({ domaine: 'skyrim', mod: 5 });
    expect(pageNexus('http://www.nexusmods.com/skyrim/mods/5')).toBeNull();
    expect(pageNexus('https://www.nexusmods.com.evil.example/skyrim/mods/5')).toBeNull();
    expect(pageNexus('https://www.nexusmods.com/skyrim/mods/5x')).toBeNull();
    expect(pageNexus('https://github.com/a/b')).toBeNull();
    expect(pageNexus(null)).toBeNull();
  });
});

describe('mods en 7z (lot 0.82.2)', () => {
  it('un fichier .7z à extraire s’installe d’un clic, un .rar non', () => {
    const f = fichierInstallable(mod('Fusion Fix'))!;
    const avec = (nom: string): ModJeu => ({ nom: 'M', fichiers: [{ ...f, nom }] });
    expect(fichierInstallable(avec('Mod.7z'))?.nom).toBe('Mod.7z');
    expect(fichierInstallable(avec('Mod.7Z'))?.nom).toBe('Mod.7Z');
    expect(fichierInstallable(avec('Mod.rar'))).toBeNull();
    expect(fichierInstallable(avec('Mod.7z.exe'))).toBeNull();
  });
});

describe('progression d’un mod (lot 0.82.3)', () => {
  const p = (x: Partial<ProgresMod>): ProgresMod => ({ suivi: 1, etape: 'telechargement', faits: 0, total: 0, debit: 0, ...x });
  it('dit les octets, le débit et le temps restant, sans jamais NaN ni plus de 100 %', () => {
    const t = chiffresMod(p({ faits: 120 * 1024 ** 2, total: 450 * 1024 ** 2, debit: 5 * 1024 ** 2 }), true);
    expect(t).toBe('120 Mo sur environ 450 Mo · 5 Mo/s · encore 1 min');
    expect(chiffresMod(p({ faits: 10, total: 0 }))).toBe('10 o');
    expect(chiffresMod(p({ faits: 600, total: 500, debit: 100 }), true)).toBe('500 o sur environ 500 o · 100 o/s');
    expect(chiffresMod(p({ etape: 'pose', faits: 1, total: 30 }))).toBe('1 fichier sur 30');
    expect(chiffresMod(p({ etape: 'pose', faits: 12, total: 30, debit: 99 }))).toBe('12 fichiers sur 30');
    expect(pourcentMod(p({ faits: 600, total: 500 }))).toBe(100);
    expect(pourcentMod(p({ faits: 5, total: 0 }))).toBe(0);
    for (const x of [p({}), p({ debit: 0, total: 10 }), p({ etape: 'decompression', faits: 3, total: 9, debit: 1 })])
      expect(chiffresMod(x)).not.toMatch(/NaN|Infinity/);
  });
});
