import { describe, expect, it } from 'vitest';
import { messageObtention } from './gratuits.svelte';

describe('messageObtention', () => {
  it('dit clairement ce qui s’est passé', () => {
    expect(messageObtention('Jeu', { etat: 'obtenu' }).ton).toBe('ok');
    expect(messageObtention('Jeu', { etat: 'deja' }).texte).toContain('déjà');
    expect(messageObtention('Jeu', { etat: 'connexion' }).texte).toContain('Connecte-toi');
    expect(messageObtention('Jeu', { etat: 'captcha' }).texte).toContain('vérification');
    expect(messageObtention('Jeu', { etat: 'erreur', motif: 'délai dépassé' }).texte).toContain('délai dépassé');
  });
});
