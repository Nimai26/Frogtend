import { describe, expect, it } from 'vitest';
import { messageObtention, messagePsPlus } from './gratuits.svelte';

describe('messageObtention', () => {
  it('dit clairement ce qui s’est passé', () => {
    expect(messageObtention('Jeu', { etat: 'obtenu' }).ton).toBe('ok');
    expect(messageObtention('Jeu', { etat: 'deja' }).texte).toContain('déjà');
    expect(messageObtention('Jeu', { etat: 'connexion' }).texte).toContain('Connecte-toi');
    expect(messageObtention('Jeu', { etat: 'captcha' }).texte).toContain('vérification');
    expect(messageObtention('Jeu', { etat: 'erreur', motif: 'délai dépassé' }).texte).toContain('délai dépassé');
  });

  it('résume un passage sur PS Plus', () => {
    expect(messagePsPlus({ etat: 'faite', ajoutes: 2, deja: 1, vus: 5 }).texte).toContain('2 jeu(x) ajouté(s)');
    expect(messagePsPlus({ etat: 'faite', ajoutes: 0, deja: 3, vus: 5 }).texte).toContain('déjà dans ta bibliothèque');
    expect(messagePsPlus({ etat: 'faite', ajoutes: 0, deja: 0, vus: 5 }).ton).toBe('alerte');
    expect(messagePsPlus({ etat: 'connexion' }).texte).toContain('Connecte-toi');
    expect(messagePsPlus({ etat: 'erreur', motif: 'délai dépassé' }).texte).toContain('délai dépassé');
  });
});
