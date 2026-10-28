// La manette pilote la page ; un lecteur (le livret) peut PRENDRE LA MAIN, sans jamais passer devant une fenêtre maison.
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { Commande } from './navigation';
import { appliquer, appliquerManette, appuiY, arreterManette, demarrerManette, parcourir, prendreLaMain, taodbox } from './manette.svelte';
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

describe('la même manette par deux voies (Steam ouvert : croix → flèches, A → Entrée)', () => {
  afterEach(() => {
    arreterManette();
    vi.useRealTimers();
  });

  /** Trois boutons l'un sous l'autre (jsdom ne calcule pas la géométrie : on la donne). */
  function trois(): HTMLButtonElement[] {
    document.body.innerHTML = '<button id="b1">1</button><button id="b2">2</button><button id="b3">3</button>';
    const l = [...document.querySelectorAll('button')];
    l.forEach((b) => (b.scrollIntoView = () => {}));
    l.forEach((b, i) => (b.getBoundingClientRect =() => ({ left: 0, top: i * 50, width: 100, height: 40, right: 100, bottom: i * 50 + 40, x: 0, y: i * 50, toJSON() {} }) as DOMRect));
    l[0].focus();
    return l;
  }
  const touche = (type: 'keydown' | 'keyup', key: string, code = key) =>
    document.activeElement!.dispatchEvent(new KeyboardEvent(type, { key, code, bubbles: true, cancelable: true }));

  it('la croix par la manette PUIS par une flèche fabriquée : un seul pas ; dans l’autre ordre aussi', () => {
    vi.useFakeTimers();
    const l = trois();
    demarrerManette({ manette: false });
    appliquerManette('bas');
    touche('keydown', 'ArrowDown');
    expect(document.activeElement).toBe(l[1]);
    vi.advanceTimersByTime(400);
    touche('keydown', 'ArrowDown');
    appliquerManette('bas');
    expect(document.activeElement).toBe(l[2]);
  });

  it('A puis Entrée : un seul clic (le clic natif d’Entrée est empêché) ; une vraie flèche seule passe', () => {
    vi.useFakeTimers();
    const l = trois();
    let clics = 0;
    l[0].addEventListener('click', () => clics++);
    demarrerManette({ manette: false });
    appliquerManette('valider');
    const entree = new KeyboardEvent('keydown', { key: 'Enter', code: 'Enter', bubbles: true, cancelable: true });
    l[0].dispatchEvent(entree);
    expect(clics).toBe(1);
    expect(entree.defaultPrevented).toBe(true);
    touche('keyup', 'Enter');
    vi.advanceTimersByTime(400);
    touche('keydown', 'ArrowDown');
    expect(document.activeElement).toBe(l[1]);
  });

  it('LB / RB : rien hors d’un lecteur ; dans un lecteur, d’un bouton à l’autre en boucle, sans le fermer', () => {
    const l = trois();
    let echap = 0;
    window.addEventListener('keydown', (e) => e.key === 'Escape' && echap++);
    appliquer('suivant');
    appliquer('precedent');
    expect(document.activeElement).toBe(l[0]);
    expect(echap).toBe(0);
    const recues: string[] = [];
    const rendre = prendreLaMain((c) => {
      recues.push(c);
      if (c === 'suivant' || c === 'precedent') parcourir(document.body, c === 'suivant' ? 1 : -1);
      else return false;
      return true;
    });
    appliquer('suivant');
    appliquer('suivant');
    expect(document.activeElement).toBe(l[2]);
    appliquer('suivant');
    expect(document.activeElement, 'en boucle').toBe(l[0]);
    appliquer('precedent');
    expect(document.activeElement).toBe(l[2]);
    expect(recues).toEqual(['suivant', 'suivant', 'suivant', 'precedent']);
    rendre();
  });

  it('B tenu (Échap répété par Steam) : un seul retour ; une flèche tenue garde sa répétition', () => {
    trois();
    demarrerManette({ manette: false });
    let echap = 0;
    window.addEventListener('keydown', (e) => e.key === 'Escape' && echap++);
    const t = (key: string, repeat: boolean) => document.activeElement!.dispatchEvent(new KeyboardEvent('keydown', { key, code: key, repeat, bubbles: true, cancelable: true }));
    t('Escape', false);
    t('Escape', true);
    t('Escape', true);
    expect(echap).toBe(1);
    t('ArrowDown', false);
    t('ArrowDown', true);
    expect((document.activeElement as HTMLElement).id).toBe('b3');
  });

  it('deux appuis rapides restent deux pas ; l’Échap fabriqué par B n’est jamais écarté', () => {
    vi.useFakeTimers();
    const l = trois();
    demarrerManette({ manette: false });
    appliquerManette('bas');
    touche('keydown', 'ArrowDown');
    vi.advanceTimersByTime(120);
    appliquerManette('bas');
    touche('keydown', 'ArrowDown');
    expect(document.activeElement).toBe(l[2]);
    let echap = 0;
    window.addEventListener('keydown', (e) => e.key === 'Escape' && echap++);
    vi.advanceTimersByTime(400);
    appliquerManette('retour');
    expect(echap).toBe(1);
  });
});
