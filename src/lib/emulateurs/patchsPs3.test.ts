import { describe, expect, it } from 'vitest';
import { descriptionFr, detailPatch, nomDuPatch, remarqueDuPatch, titreDuJeu } from './patchsPs3';
import vraie from '../../../src-tauri/fixtures/simule/patchs-BLES01227.json';

describe('retouches traduites par Firehouse (contrat 1.20, vraie réponse d’Asura’s Wrath)', () => {
  const patchs = (vraie as { patchs: { description: string; description_fr: string | null; notes: string; notes_fr: string | null }[] }).patchs;
  it('la traduction de Firehouse d’abord ; sans elle, la nôtre ou l’anglais marqué', () => {
    const flou = patchs.find((p) => p.description === 'Disable Motion Blur')!;
    expect(nomDuPatch(flou)).toEqual({ texte: 'Désactiver le flou de mouvement', anglais: false });
    expect(nomDuPatch({ ...flou, description_fr: null })).toEqual({ texte: 'Sans flou de mouvement', anglais: false });
    expect(nomDuPatch({ description: 'Fix Shadows', description_fr: null })).toEqual({ texte: 'Fix Shadows', anglais: true });
  });
  it('la remarque traduite, sinon l’anglais marqué ; vide : rien', () => {
    const menu = patchs.find((p) => p.description === 'Developer Menu')!;
    expect(remarqueDuPatch(menu)?.anglais).toBe(false);
    expect(remarqueDuPatch(menu)?.texte).toContain('croix directionnelle');
    expect(remarqueDuPatch({ ...menu, notes_fr: null })).toEqual({ texte: menu.notes, anglais: true });
    expect(remarqueDuPatch({ notes: '', notes_fr: '' })).toBeNull();
  });
});

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
