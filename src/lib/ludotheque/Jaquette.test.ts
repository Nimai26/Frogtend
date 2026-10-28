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
