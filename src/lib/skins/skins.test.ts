import { describe, expect, it } from 'vitest';
import instantane from '../../../docs/charte/themes.instantane.json';
import fixtureCommuns from '../fixtures/communs.simule.json';
import { contraste, lireCouleur, pireContraste } from './couleurs';
import { fondsDeReference, TONS } from './encres';
import {
  appliquerSkin,
  catalogueDepuisInstantane,
  choisirSkin,
  jetonsDuSkin,
  libelleSkin,
  nomsDesSkins,
  SKIN_DE_REPLI,
  VARIABLES_DE_BASE,
  type Instantane,
} from './skins';

const communs = fixtureCommuns.communs;
const catalogue = catalogueDepuisInstantane(instantane as Instantane, communs);
const noms = Object.keys(catalogue.themes);

/** Résout `var(--x)` dans un jeu de jetons jusqu'à une couleur lisible. */
function valeur(jetons: Record<string, string>, nom: string): string {
  let v = jetons[nom];
  for (let i = 0; i < 5 && v?.startsWith('var('); i++) v = jetons[v.slice(4, -1).trim()];
  return v;
}

describe('le catalogue tiré de l’instantané', () => {
  it('contient les 44 skins, chacun avec exactement les 11 variables de base', () => {
    expect(noms).toHaveLength(44);
    for (const nom of noms) {
      expect(Object.keys(catalogue.themes[nom].base).sort()).toEqual([...VARIABLES_DE_BASE].sort());
    }
  });

  it('garde le crédit Theme.Park', () => {
    expect(catalogue.credit).toContain('Theme.Park');
  });
});

describe('les garanties de Firehouse, sur les 44 skins', () => {
  it('chaque encre --on-* atteint au moins 4,3:1 sur son fond', () => {
    const echecs: string[] = [];
    for (const nom of noms) {
      const j = jetonsDuSkin(catalogue, nom);
      for (const ton of TONS) {
        const fond = lireCouleur(j[`--${ton}`]);
        const encre = lireCouleur(valeur(j, `--on-${ton}`));
        if (!fond || !encre) continue;
        const k = contraste(fond, encre);
        if (k < 4.3) echecs.push(`${nom} --on-${ton} : ${k.toFixed(2)}`);
      }
    }
    expect(echecs).toEqual([]);
  });

  it('les 44 × 6 textes (5 états + --dim) atteignent au moins 4,5:1 sur le pire fond', () => {
    const echecs: string[] = [];
    let comptes = 0;
    for (const nom of noms) {
      const j = jetonsDuSkin(catalogue, nom);
      const fonds = fondsDeReference(catalogue.themes[nom].base);
      for (const jeton of [...TONS.map((t) => `--${t}-texte`), '--dim']) {
        const c = lireCouleur(valeur(j, jeton));
        if (!c) continue;
        comptes++;
        const k = pireContraste(c, fonds);
        if (k < 4.5) echecs.push(`${nom} ${jeton} : ${k.toFixed(2)}`);
      }
    }
    expect(echecs).toEqual([]);
    expect(comptes).toBe(44 * 6);
  });
});

describe('le choix du skin', () => {
  it('prend le skin demandé s’il existe', () => {
    expect(choisirSkin(catalogue, 'aquamarine', 'firehouse')).toBe('aquamarine');
  });
  it('reste sur le skin en cours si le nom demandé est inconnu', () => {
    expect(choisirSkin(catalogue, 'inexistant', 'aquamarine')).toBe('aquamarine');
  });
  it('retombe sur firehouse à défaut', () => {
    expect(choisirSkin(catalogue, 'inexistant', null)).toBe(SKIN_DE_REPLI);
  });
  it('liste firehouse en premier, les autres dans l’ordre alphabétique', () => {
    const liste = nomsDesSkins(catalogue);
    expect(liste[0]).toBe('firehouse');
    expect(liste.slice(1)).toEqual([...liste.slice(1)].sort((a, b) => a.localeCompare(b, 'fr')));
  });
  it('donne un nom lisible', () => {
    expect(libelleSkin('blackberry-abyss')).toBe('Blackberry abyss');
  });
});

describe('l’application à la page', () => {
  it('pose les jetons et retire ceux du skin précédent', () => {
    const el = document.createElement('div');
    appliquerSkin(catalogue, 'firehouse', el);
    expect(el.style.getPropertyValue('--accent')).toBe(catalogue.themes.firehouse.base['--accent']);
    expect(el.style.getPropertyValue('--on-accent')).not.toBe('');
    expect(el.dataset.skin).toBe('firehouse');

    const retenu = appliquerSkin(catalogue, 'nom-inconnu', el);
    expect(retenu).toBe('firehouse');
  });
});
