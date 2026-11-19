// Les pastilles d'origine : l'icône dessinée du Disque, le texte quand il n'y a pas d'icône (hors de l'application,
// aucune icône de lanceur n'est demandée).
import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';
import IconesBoutiques from './IconesBoutiques.svelte';

afterEach(() => cleanup());

describe('IconesBoutiques', () => {
  it('le Disque : un disque dur dessiné (aucun texte), « Vient de Disque » gardé pour la lecture', () => {
    const { container } = render(IconesBoutiques, { origines: ['disque'] });
    expect(container.querySelector('svg')).not.toBeNull();
    expect(container.textContent?.trim()).toBe('');
    expect(screen.getByRole('img').getAttribute('aria-label')).toBe('Vient de Disque');
  });
  it('sans icône (navigateur, Xbox) : le nom écrit', () => {
    const { container } = render(IconesBoutiques, { origines: ['xboxone'] });
    expect(container.querySelector('svg')).toBeNull();
    expect(container.textContent).toContain('Xbox');
  });
  it('Firehouse : son logo livré avec l’application ; illisible (absent), le nom écrit revient', async () => {
    const { container } = render(IconesBoutiques, { origines: ['firehouse'] });
    const img = container.querySelector('img')!;
    expect(img.getAttribute('src')).toBe('/icones/firehouse.png');
    img.dispatchEvent(new Event('error'));
    await tick();
    expect(container.querySelector('img')).toBeNull();
    expect(container.textContent).toContain('Firehouse');
  });
});
