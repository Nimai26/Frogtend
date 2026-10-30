import { afterEach, describe, expect, it, vi } from 'vitest';
import { apresRepos, assezGrandPourUnFond, idDuFocus, REPOS_MS } from './fond';

describe('idDuFocus', () => {
  it('lit le numéro de la carte, même depuis un élément à l’intérieur', () => {
    document.body.innerHTML = '<button data-id="116"><span id="dedans">x</span></button><button id="bouton">Bureau</button>';
    expect(idDuFocus(document.querySelector('[data-id]'))).toBe(116);
    expect(idDuFocus(document.getElementById('dedans'))).toBe(116);
  });

  it('hors d’une carte de jeu : aucun', () => {
    document.body.innerHTML = '<button id="bouton">Bureau</button><button data-id="x">?</button>';
    expect(idDuFocus(document.getElementById('bouton'))).toBeNull();
    expect(idDuFocus(document.querySelector('[data-id]'))).toBeNull();
    expect(idDuFocus(null)).toBeNull();
  });
});

describe('apresRepos', () => {
  afterEach(() => vi.useRealTimers());

  it('ne choisit rien pendant un défilement à la croix (130 ms), puis la dernière carte au repos', () => {
    vi.useFakeTimers();
    const vus: (number | null)[] = [];
    const r = apresRepos<number | null>((v) => vus.push(v));
    for (const id of [1, 2, 3, 4]) {
      r.pousser(id);
      vi.advanceTimersByTime(130);
    }
    expect(vus).toEqual([]);
    vi.advanceTimersByTime(REPOS_MS);
    expect(vus).toEqual([4]);
  });

  it('annulé (changement de console, sortie de Taodbox) : rien', () => {
    vi.useFakeTimers();
    const vus: number[] = [];
    const r = apresRepos<number>((v) => vus.push(v));
    r.pousser(7);
    r.annuler();
    vi.advanceTimersByTime(REPOS_MS * 2);
    expect(vus).toEqual([]);
  });
});

describe('une image assez grande pour un fond', () => {
  it('une capture de Super Nintendo non ; un vrai fond oui', () => {
    expect(assezGrandPourUnFond(256, 224)).toBe(false);
    expect(assezGrandPourUnFond(1000, 562)).toBe(true);
    expect(assezGrandPourUnFond(1000, 300)).toBe(false);
  });
});
