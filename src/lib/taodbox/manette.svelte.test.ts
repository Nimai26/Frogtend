// La manette pilote la page ; un lecteur (le livret) peut PRENDRE LA MAIN, sans jamais passer devant une fenêtre maison.
import { afterEach, describe, expect, it } from 'vitest';
import type { Commande } from './navigation';
import { appliquer, appuiY, prendreLaMain, taodbox } from './manette.svelte';
import { auCanape, seulementAuBureau } from './canape';
import { pile } from '$lib/dialogues/fenetres.svelte';

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

describe('Y : gérer le jeu', () => {
  it('envoie la touche Menu à la carte choisie', () => {
    document.body.innerHTML = '<button id="carte">Jeu</button>';
    const touches: string[] = [];
    const carte = document.getElementById('carte')!;
    carte.addEventListener('keydown', (e) => touches.push(e.key));
    carte.focus();
    expect(appuiY()).toBe(true);
    expect(touches).toEqual(['ContextMenu']);
  });

  it('rien par-dessus une fenêtre maison ni quand un lecteur a pris la main', () => {
    document.body.innerHTML = '<button id="carte">Jeu</button>';
    const touches: string[] = [];
    document.getElementById('carte')!.addEventListener('keydown', (e) => touches.push(e.key));
    document.getElementById('carte')!.focus();
    const recues: Commande[] = [];
    const rendre = prendreLaMain((c) => (recues.push(c), true));
    expect(appuiY()).toBe(false);
    rendre();
    document.body.insertAdjacentHTML('beforeend', '<div role="dialog"><button>OK</button></div>');
    expect(appuiY()).toBe(false);
    // Le lecteur n'a rien reçu : il ne se ferme pas.
    expect(recues).toEqual([]);
    expect(touches).toEqual([]);
  });
});

describe('mode canapé', () => {
  afterEach(() => document.documentElement.removeAttribute('data-taodbox'));

  it('au bureau, rien n’est renvoyé à l’écran du PC', async () => {
    expect(auCanape()).toBe(false);
    expect(await seulementAuBureau('Choisir un fichier')).toBe(false);
    expect(pile.dialogues).toHaveLength(0);
  });

  it('la manette du menu en jeu n’est pas « au canapé » : seul l’attribut de la vraie Taodbox compte', () => {
    taodbox.actif = true;
    expect(auCanape()).toBe(false);
    taodbox.actif = false;
  });

  it('au canapé : une fenêtre l’explique, et l’appelant s’arrête', async () => {
    document.documentElement.setAttribute('data-taodbox', '');
    expect(auCanape()).toBe(true);
    const r = seulementAuBureau('Choisir le dossier des outils');
    expect(pile.dialogues).toHaveLength(1);
    const d = pile.dialogues[0];
    expect(d.sorte).toBe('informer');
    expect(d.message).toContain('Choisir le dossier des outils');
    if (d.sorte === 'informer') d.fermer();
    expect(await r).toBe(true);
  });
});
