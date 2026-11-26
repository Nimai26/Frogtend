// Contrat 1.55 : la taille des musiques que la personne LIT dans l'accord, avec le vrai `taille()` de l'interface
// (les autres tests le simulent) — DK Summit réel : 41 018 496 octets.
import { describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), isTauri: () => false }));

import { libellePreparer, plusMusiques, titreAllumageMusique } from './mods';
import type { ModSwitchVu } from '$lib/api';

describe('la taille des musiques, telle qu’elle s’affiche', () => {
  it('41 018 496 octets se disent « 39,1 Mo »', () => {
    expect(plusMusiques(41018496)).toBe('+ 39,1 Mo de musiques');
    expect(plusMusiques(41018496, true)).toBe('+ environ 39,1 Mo de musiques');
    expect(libellePreparer(41018496)).toBe('🎵 Préparer les musiques (≈ 39,1 Mo)');
    expect(titreAllumageMusique({ nom: 'DK Summit' } as ModSwitchVu, 41018496)).toBe('🎵 Allumer « DK Summit » (+ 39,1 Mo de musiques) ?');
  });
});
