// La manette pilote la page ; un lecteur (le livret) peut PRENDRE LA MAIN, sans jamais passer devant une fenêtre maison.
import { afterEach, describe, expect, it } from 'vitest';
import type { Commande } from './navigation';
import { appliquer, prendreLaMain } from './manette.svelte';

afterEach(() => {
  document.body.innerHTML = '';
});

describe('prendre la main', () => {
  it('le lecteur reçoit les commandes, puis les rend en se fermant', () => {
    document.body.innerHTML = '<button id="a">A</button>';
    let clics = 0;
    document.getElementById('a')!.addEventListener('click', () => clics++);
    document.getElementById('a')!.focus();
    const recues: Commande[] = [];
    const rendre = prendreLaMain((c) => {
      recues.push(c);
      return true;
    });
    appliquer('droite');
    appliquer('valider');
    expect(recues).toEqual(['droite', 'valider']);
    expect(clics).toBe(0);
    rendre();
    appliquer('valider');
    expect(recues).toHaveLength(2);
    expect(clics).toBe(1);
  });

  it('une fenêtre maison ouverte par-dessus garde la main', () => {
    document.body.innerHTML = '<div role="dialog"><button id="ok">OK</button></div>';
    let clics = 0;
    document.getElementById('ok')!.addEventListener('click', () => clics++);
    document.getElementById('ok')!.focus();
    const recues: Commande[] = [];
    const rendre = prendreLaMain((c) => (recues.push(c), true));
    appliquer('valider');
    expect(recues).toEqual([]);
    expect(clics).toBe(1);
    rendre();
  });

  it('un preneur qui ne se sert pas d’une commande la laisse à la navigation', () => {
    document.body.innerHTML = '<button id="b">B</button>';
    let clics = 0;
    document.getElementById('b')!.addEventListener('click', () => clics++);
    document.getElementById('b')!.focus();
    const rendre = prendreLaMain(() => false);
    appliquer('valider');
    expect(clics).toBe(1);
    rendre();
  });
});
