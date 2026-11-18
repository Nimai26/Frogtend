import { describe, expect, it } from 'vitest';
import { cartesBoutiqueTaodbox, venantDe, groupeDe, libelleCopie, nomOrigine, originesCarte, originesJeuPc, unParGroupe } from './copies';

describe('Un jeu sur plusieurs boutiques = une carte (A6c)', () => {
  const groupes = [[-1, -2], [-5, -6, -7]];
  it('retrouve le groupe d’une copie', () => {
    expect(groupeDe(-2, groupes)).toEqual([-1, -2]);
    expect(groupeDe(-9, groupes)).toEqual([-9]);
  });
  it('ne garde qu’une carte par groupe en Taodbox (la première du groupe présente)', () => {
    const jeux = [{ id: -2 }, { id: 110 }, { id: -1 }, { id: -7 }];
    expect(unParGroupe(jeux, groupes).map((j) => j.id)).toEqual([-1, 110, -7]);
  });
  it('dit d’où vient chaque carte, pour tous les jeux', () => {
    expect(originesCarte({ id: -1, boutique: 'steam', copies: [{ boutique: 'steam' }, { boutique: 'gog' }] })).toEqual(['steam', 'gog']);
    expect(originesCarte({ id: -3, boutique: 'epic' })).toEqual(['epic']);
    expect(originesCarte({ id: -4, source: 'local' })).toEqual(['disque']);
    expect(originesCarte({ id: 110 })).toEqual(['firehouse']);
    expect(originesJeuPc({ id: -1, installation: { boutique: { source: 'steam' } } }, [-1, -2], (x) => (x === -1 ? 'steam' : 'gog'))).toEqual(['steam', 'gog']);
    expect(originesJeuPc({ id: 110, installation: {} }, [110], () => undefined)).toEqual(['firehouse']);
    expect([nomOrigine('gog'), nomOrigine('disque'), nomOrigine('zzz')]).toEqual(['GOG', 'Disque', 'Autre']);
  });
  it('présente une copie pour le choix avant de lancer', () => {
    const l = libelleCopie({ id: -2, boutique: 'gog', temps_jeu: 0 }, (b) => b.toUpperCase(), (s) => `${s} s`);
    expect(l).toEqual({ libelle: '▶ GOG', detail: 'jamais lancé par Frogtend' });
  });
});

describe('Taodbox : les jeux installés des boutiques dans la grille principale', () => {
  it('une carte par jeu, jamais un jeu déjà sur le PC, avec sa boutique pour la pastille', () => {
    const steam = [{ id: '620', nom: 'Portal 2', installe: true }, { id: '400', nom: 'Portal', installe: true }, { id: '1', nom: 'Pas là', installe: false }];
    const galaxy = [
      { id: 'steam_620', nom: 'Portal 2', installe: true, plateforme: 'steam' },
      { id: 'epic_Fortnite', nom: 'Fortnite', installe: true, plateforme: 'epic', image: 'https://images.gog.com/f.webp' },
      { id: 'gog_1', nom: 'Cyberpunk', installe: true, plateforme: 'gog' },
    ];
    const pc = [{ installation: { boutique: { source: 'steam', cle: '400' } } }, { installation: { boutique: { source: 'gog', cle: '1' } } }, { installation: null }];
    const l = cartesBoutiqueTaodbox(steam, galaxy, pc);
    expect(l.map((c) => [c.cle, c.boutique, c.par])).toEqual([
      ['steam_620', 'steam', 'steam'],
      ['epic_Fortnite', 'epic', 'galaxy'],
    ]);
    expect(l[1].image).toBe('https://images.gog.com/f.webp');
    expect(cartesBoutiqueTaodbox([], [], [])).toEqual([]);
  });
});

describe('la pastille d’origine', () => {
  it('élide devant une voyelle', () => {
    expect(venantDe(['Ubisoft'])).toBe('Vient d’Ubisoft');
    expect(venantDe(['Steam', 'Epic'])).toBe('Vient de Steam et d’Epic');
    expect(venantDe(['Amazon', 'GOG'])).toBe('Vient d’Amazon et de GOG');
  });
});
