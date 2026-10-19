import { describe, expect, it } from 'vitest';
import { descriptionFr, detailPatch, titreDuJeu } from './patchsPs3';

describe('retouches (patchs) PS3 en français', () => {
  it('les descriptions courantes sont traduites sans rien promettre, les autres marquées en anglais', () => {
    expect(descriptionFr('Unlock FPS')).toEqual({ texte: 'Débloquer les images par seconde', anglais: false });
    expect(descriptionFr('Unlock FPS').texte).not.toMatch(/\d/);
    expect(descriptionFr('Disable Motion Blur').anglais).toBe(false);
    expect(descriptionFr('Fix Shadows Flicker')).toEqual({ texte: 'Fix Shadows Flicker', anglais: true });
  });

  it('le titre qui est ce jeu, jamais sa démo (relevé réel : Asura’s Wrath et sa Preview)', () => {
    const l = [{ titre: "ASURA'S WRATH Preview" }, { titre: "Asura's Wrath" }, { titre: "ASURA'S WRATH Preview" }];
    // La fiche du PC s'appelle « Asuras Wrath » : c'est le même titre, même si la démo a plus de patchs.
    expect(titreDuJeu(l, 'Asuras Wrath')).toBe("Asura's Wrath");
    // Sans titre ressemblant : le plus fréquent ; à égalité, le plus court (la démo ajoute « Preview »).
    expect(titreDuJeu([{ titre: 'Jeu Preview' }, { titre: 'Jeu' }], 'Autre')).toBe('Jeu');
    expect(titreDuJeu([], 'X')).toBeNull();
  });

  it('une note, même longue, ne remplit jamais la ligne', () => {
    expect(detailPatch({ notes: 'x'.repeat(400) })).toBe('⚠ une remarque de l’auteur, montrée avant d’activer');
    expect(detailPatch({ notes: ' ' })).toBeUndefined();
  });
});
