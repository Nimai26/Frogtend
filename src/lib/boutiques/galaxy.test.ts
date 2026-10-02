import { describe, expect, it } from 'vitest';
import { nomPlateforme } from './galaxy.svelte';

describe('nomPlateforme', () => {
  it('nomme les boutiques reliées à GOG Galaxy', () => {
    expect(nomPlateforme('gog')).toBe('GOG');
    expect(nomPlateforme('epic')).toBe('Epic Games');
    expect(nomPlateforme('xboxone')).toBe('Xbox');
    expect(nomPlateforme('origin')).toBe('EA');
    expect(nomPlateforme('inconnue')).toBe('Autre');
  });
});
