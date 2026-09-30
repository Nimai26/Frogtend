import { describe, expect, it } from 'vitest';
import { libelleCommandes } from './commandes';

describe('libelleCommandes', () => {
  it('dit le choix en clair', () => {
    expect(libelleCommandes(undefined)).toBe('Automatique');
    expect(libelleCommandes({ mode: 'auto' })).toBe('Automatique');
    expect(libelleCommandes({ mode: 'clavier' })).toBe('Clavier et souris');
    expect(libelleCommandes({ mode: 'reference', genre: 'Wiimote', reference: 'Wiimote horizontale' })).toBe(
      'Wiimote : Wiimote horizontale',
    );
    expect(libelleCommandes({ mode: 'reference', genre: 'Autre', reference: 'X' })).toBe('Autre : X');
  });
});
