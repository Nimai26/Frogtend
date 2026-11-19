// Entrer en Taodbox et en sortir : l'attribut et la hauteur de l'écran sur laquelle toutes les tailles sont calculées.
import { afterEach, describe, expect, it, vi } from 'vitest';
import { auCanape, poserModeCanape } from './canape';

afterEach(() => {
  poserModeCanape(false);
  vi.unstubAllGlobals();
});

describe('poserModeCanape', () => {
  it('pose data-taodbox et la hauteur de l’ÉCRAN (pas celle de la fenêtre du menu en jeu)', () => {
    vi.stubGlobal('screen', { height: 1080 });
    poserModeCanape(true);
    expect(auCanape()).toBe(true);
    expect(document.documentElement.style.getPropertyValue('--hauteur-ecran')).toBe('1080px');
  });
  it('en sortant, retire les deux : le bureau garde ses tailles', () => {
    vi.stubGlobal('screen', { height: 2160 });
    poserModeCanape(true);
    poserModeCanape(false);
    expect(auCanape()).toBe(false);
    expect(document.documentElement.style.getPropertyValue('--hauteur-ecran')).toBe('');
  });
});
