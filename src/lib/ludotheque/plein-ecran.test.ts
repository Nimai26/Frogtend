import { describe, expect, it } from 'vitest';
import { libellePleinEcran, TOUCHES_PLEIN_ECRAN } from './plein-ecran';

describe('plein écran au lancement', () => {
  it('dit le réglage en mots simples, et rien pour une valeur inconnue', () => {
    expect(libellePleinEcran(undefined)).toBe('non (le jeu s’ouvre comme il veut)');
    expect(libellePleinEcran('fenetre')).toBe('fenêtre agrandie à tout l’écran');
    expect(libellePleinEcran('touche:F4')).toBe('Frogtend appuie sur F4');
    expect(libellePleinEcran('touche:Alt+Entrée')).toBe('Frogtend appuie sur Alt+Entrée');
    expect(libellePleinEcran('touche:Suppr')).toBe('non (le jeu s’ouvre comme il veut)');
  });
  it('propose les mêmes touches que le cœur', () => {
    expect(TOUCHES_PLEIN_ECRAN).toHaveLength(13);
    expect(TOUCHES_PLEIN_ECRAN).toContain('F11');
  });
});
