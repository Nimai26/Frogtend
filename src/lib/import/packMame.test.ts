import { describe, expect, it } from 'vitest';
import { avisVersionMame, placeSuffisante, texteAnnoncePack, type AnnoncePack } from './packMame';

const GO = 1024 ** 3;
const a = (x: Partial<AnnoncePack>): AnnoncePack => ({ version: '0.289', a_recevoir: 8994, octets: 40 * GO, deja_la: 0, anciens: 0, en_cours: false, libre: 500 * GO, chd_fichiers: 300, chd_octets: 240 * GO, ...x });

describe('le pack MAME', () => {
  it('annonce combien, combien de Go, combien de temps et la place', () => {
    const t = texteAnnoncePack(a({}), 10 * 1024 * 1024, (s) => `${Math.round(s / 3600)} h`);
    // Les milliers séparés par l'espace fine insécable du français (toLocaleString).
    expect(t.replace(/\s/g, ' ')).toContain('8 994 fichier(s) à recevoir, 40 Go (MAME 0.289)');
    expect(t).toContain('Durée estimée');
    expect(t.replace(/\s/g, ' ')).toContain('Place libre sur ce disque : 500 Go.');
    expect(texteAnnoncePack(a({}), null, () => '')).toContain('Durée : inconnue');
    expect(texteAnnoncePack(a({ a_recevoir: 0, deja_la: 8994 }), null, () => '')).toContain('complet');
  });

  it('refuse quand le disque est trop petit', () => {
    expect(placeSuffisante(a({ libre: 30 * GO }))).toBe(false);
    expect(placeSuffisante(a({ libre: 41.5 * GO }))).toBe(true);
    expect(placeSuffisante(a({ libre: null }))).toBe(true);
    expect(texteAnnoncePack(a({ libre: 30 * GO }), null, () => '')).toContain('pas assez');
  });

  it('dit quand MAME n’est pas à la version du pack', () => {
    expect(avisVersionMame({ version: '0.289', par_frogtend: true }, '0.289')).toBeNull();
    expect(avisVersionMame({ version: '0.288', par_frogtend: true }, '0.289')).toContain('mets MAME à jour');
    expect(avisVersionMame({ version: '0.288', par_frogtend: false }, '0.289')).toContain('ne démarreront pas');
    expect(avisVersionMame({ version: null, par_frogtend: false }, '0.289')).toContain('à la main');
    expect(avisVersionMame(undefined, '0.289')).toContain('pas encore installé');
  });
});
