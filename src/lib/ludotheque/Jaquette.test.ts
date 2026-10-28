// Une jaquette qui ne se charge pas : trois nouveaux essais espacés avant d'abandonner (essai de Seb, 28/10 : au
// retour vers la ludothèque, un échec passager vidait toutes les jaquettes jusqu'au redémarrage).
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { tick } from 'svelte';
import Jaquette from './Jaquette.svelte';

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe('jaquette déjà chargée quand la liste est relue', () => {
  it('des informations d’image neuves mais identiques, ou quelques pixels de plus : elle RESTE affichée', async () => {
    // L'essai réel de Seb (28/10) : au retour vers la ludothèque, l'image en mémoire s'affichait une fraction de
    // seconde, puis la liste relue donnait des `images` NEUVES (identiques) et l'image se cachait pour toujours.
    const images = { jaquette: { largeur: 600, hauteur: 800 } };
    const { container, rerender } = render(Jaquette, { id: 110, titre: 'Dune', largeur: 150, visuel: 'jaquette', images });
    await tick();
    const img = () => container.querySelector('img')!;
    const chargee = () => container.querySelector('.jaquette')!.classList.contains('chargee');
    const avant = img().getAttribute('src');
    await fireEvent.load(img());
    await tick();
    expect(chargee()).toBe(true);
    await rerender({ id: 110, titre: 'Dune', largeur: 150, visuel: 'jaquette', images: { jaquette: { ...images.jaquette } } });
    await tick();
    expect(img().getAttribute('src')).toBe(avant);
    expect(chargee(), 'la liste relue ne la cache pas').toBe(true);
    await rerender({ id: 110, titre: 'Dune', largeur: 160 });
    await tick();
    expect(img().getAttribute('src')).toBe(avant);
    expect(container.querySelector('.jaquette')!.classList.contains('chargee'), 'toujours visible').toBe(true);
    // Une taille qui change vraiment l'image demandée : elle repart de zéro (et se recharge).
    await rerender({ id: 110, titre: 'Dune', largeur: 600 });
    await tick();
    expect(img().getAttribute('src')).not.toBe(avant);
    expect(container.querySelector('.jaquette')!.classList.contains('chargee')).toBe(false);
  });
});

describe('jaquette qui ne vient pas tout de suite', () => {
  it('réessaie à 1,5 s, 4 s et 10 s, puis abandonne seulement après le 4e échec', async () => {
    vi.useFakeTimers();
    const { container } = render(Jaquette, { id: 110, titre: 'Dune', plateforme: 'MS-DOS', largeur: 150 });
    await tick();
    const img = () => container.querySelector('img');
    const vus: string[] = [];
    // Chaque affichage a sa propre adresse (jamais un échec gardé par le navigateur pour une autre).
    expect(img()!.getAttribute('src')).toMatch(/[?&]m=[a-z0-9]+$/);
    for (const delai of [1500, 4000, 10000]) {
      vus.push(img()!.getAttribute('src')!);
      await fireEvent.error(img()!);
      // Un léger décalage (± 10 %) : rien avant 90 % du délai, le nouvel essai au plus tard à 110 %.
      await vi.advanceTimersByTimeAsync(delai * 0.9 - 1);
      expect(img()!.getAttribute('src')).toBe(vus.at(-1));
      await vi.advanceTimersByTimeAsync(delai * 0.2 + 2);
      await tick();
      expect(img(), `nouvel essai après ${delai} ms`).not.toBeNull();
    }
    expect(new Set([...vus, img()!.getAttribute('src')]).size).toBe(4);
    await fireEvent.error(img()!);
    await tick();
    expect(img(), 'abandon après le 4e échec : la carte de remplacement reste').toBeNull();
    expect(container.textContent).toContain('Dune');
  });
});
