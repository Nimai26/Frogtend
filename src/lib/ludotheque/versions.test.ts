import { describe, expect, it } from 'vitest';
import { cheminCourant, estCourante, libelleVersion, nomDuFichier, nomEnDouble, ordreDesCases, plusieursPossibles, rangCourt, type AvisVersion } from './versions';

describe('plusieurs versions de Firehouse à la fois', () => {
  const a = (telechargement_id: number, rang: number, fichiers: string[], cochable = true): AvisVersion => ({ telechargement_id, cochable, rang, qualite: 0, fichiers });
  const avis = [a(31, 2, ['Mario (USA).sfc']), a(32, 3, ['Mario (Japan).sfc']), a(33, 1, ['Mario (Europe).sfc']), a(35, 2, ['mario (usa).SFC'])];
  it('cochables selon le cœur, pas un jeu à cartes', () => {
    expect(plusieursPossibles(avis, 'Super Nintendo Entertainment System')).toBe(true);
    expect(plusieursPossibles([avis[0]], 'Super Nintendo Entertainment System')).toBe(false);
    expect(plusieursPossibles([avis[0], a(34, 3, ['x.exe'], false)], 'Windows')).toBe(false);
    expect(plusieursPossibles(avis, 'Nintendo e-Reader')).toBe(false);
  });
  it('l’Europe avant les USA, puis le Japon ; l’ordre reçu départage', () => {
    const v = [31, 32, 33, 35].map((telechargement_id) => ({ telechargement_id }));
    expect(ordreDesCases(v, avis).map((x) => x.telechargement_id)).toEqual([33, 31, 35, 32]);
  });
  it('deux versions au même nom de fichier ne se cochent pas ensemble', () => {
    expect(nomEnDouble([31, 32, 33], avis)).toBeNull();
    expect(nomEnDouble([31, 35], avis)).toBe('mario (usa).SFC');
  });
});

describe('les versions d’un jeu importé', () => {
  const i = { dossier: 'E:\\NES\\Europe', fichier_du_jeu: 'Mario (E).nes' };
  it('reconnaît la version lancée par défaut', () => {
    expect(cheminCourant(i)).toBe('E:\\NES\\Europe\\Mario (E).nes');
    expect(estCourante({ chemin: 'e:/nes/europe/mario (e).nes', libelle: 'E', rang: 1, qualite: 0 }, i)).toBe(true);
    expect(estCourante({ chemin: 'E:\\NES\\USA\\Mario (U).nes', libelle: 'U', rang: 2, qualite: 0 }, i)).toBe(false);
  });
  it('se lit facilement', () => {
    expect(libelleVersion({ chemin: 'x', libelle: 'France', rang: 0, qualite: 0 })).toBe('FR — France');
    expect(libelleVersion({ chemin: 'x', libelle: '', rang: 3, qualite: 0, disques: ['a', 'b'] })).toBe('autre — sans étiquette · 2 disques');
    // Une traduction de fan est dite comme telle (Seb, 04/10).
    const trad = { chemin: 'x', libelle: 'USA, Traduction française par Génération IX', rang: 0, qualite: 1 };
    expect(libelleVersion(trad)).toBe('FR (traduction de fan) — USA, Traduction française par Génération IX');
    expect(rangCourt(trad)).toBe('FR (trad.)');
    expect(rangCourt({ libelle: 'France', rang: 0 })).toBe('FR');
    expect(nomDuFichier({ chemin: 'E:\\NES\\Mario (F).nes', libelle: '', rang: 0, qualite: 0 })).toBe('Mario (F).nes');
  });
});
