import { describe, expect, it } from 'vitest';
import { connexionApres, messageObtention, messagePsPlus, messageRecolte, sessionEpicPerdue } from './gratuits.svelte';

describe('sessionEpicPerdue (Seb 20/11 : prévenir quand la session Epic expire)', () => {
  it('seulement si le compte était connecté et que la vérification le trouve déconnecté', () => {
    expect(sessionEpicPerdue(true, false)).toBe(true);
    expect(sessionEpicPerdue(true, true)).toBe(false);
    expect(sessionEpicPerdue(true, null)).toBe(false);
    expect(sessionEpicPerdue(false, false)).toBe(false);
  });
});

describe('messageObtention', () => {
  it('dit clairement ce qui s’est passé', () => {
    expect(messageObtention('Jeu', { etat: 'obtenu' }).ton).toBe('ok');
    expect(messageObtention('Jeu', { etat: 'deja' }).texte).toContain('déjà');
    expect(messageObtention('Jeu', { etat: 'connexion' }).texte).toContain('Connecte-toi');
    expect(messageObtention('Jeu', { etat: 'captcha' }).texte).toContain('vérification');
    expect(messageObtention('Jeu', { etat: 'erreur', motif: 'délai dépassé' }).texte).toContain('délai dépassé');
  });

  it('montre l’étape de l’erreur, pas la liste des boutons (elle va au journal)', () => {
    const t = messageObtention('Jeu', { etat: 'erreur', motif: "pas de bouton d'achat | wishlist ; share" }).texte;
    expect(t).toContain("(pas de bouton d'achat)");
    expect(t).not.toContain('wishlist');
    expect(messageObtention('Jeu', { etat: 'erreur' }).texte).toContain('raison inconnue');
  });

  it('déduit du résultat si le compte de la boutique est connecté', () => {
    for (const e of ['obtenu', 'deja', 'connecte', 'faite', 'aucun', 'pas_abonne']) expect(connexionApres(e)).toBe(true);
    expect(connexionApres('connexion')).toBe(false);
    // Une erreur ou un captcha ne disent rien du compte : l'état connu est gardé.
    expect(connexionApres('erreur')).toBeNull();
    expect(connexionApres('captcha')).toBeNull();
  });

  it('résume un passage sur GOG ou Prime Gaming', () => {
    const base = { obtenus: [], deja: 0, a_finir: 0 };
    expect(messageRecolte('gog', { ...base, etat: 'faite', obtenus: ['Beyond Good & Evil'] }).texte).toContain('Beyond Good & Evil');
    expect(messageRecolte('gog', { ...base, etat: 'aucun' }).texte).toContain('pas de jeu offert');
    expect(messageRecolte('prime', { ...base, etat: 'faite', a_finir: 2 }).ton).toBe('alerte');
    expect(messageRecolte('prime', { ...base, etat: 'pas_abonne' }).texte).toContain('pas abonné');
    expect(messageRecolte('gog', { ...base, etat: 'connexion' }).texte).toContain('Connecte-toi à GOG');
  });

  it('résume un passage sur PS Plus', () => {
    expect(messagePsPlus({ etat: 'faite', ajoutes: 2, deja: 1, vus: 5 }).texte).toContain('2 jeu(x) ajouté(s)');
    expect(messagePsPlus({ etat: 'faite', ajoutes: 0, deja: 3, vus: 5 }).texte).toContain('déjà dans ta bibliothèque');
    expect(messagePsPlus({ etat: 'faite', ajoutes: 0, deja: 0, vus: 5 }).ton).toBe('alerte');
    expect(messagePsPlus({ etat: 'connexion' }).texte).toContain('Connecte-toi');
    expect(messagePsPlus({ etat: 'erreur', motif: 'délai dépassé' }).texte).toContain('délai dépassé');
  });
});
