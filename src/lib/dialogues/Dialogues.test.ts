import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';
import Dialogues from './Dialogues.svelte';
import { choisir, confirmer, demander, informer, pile, toast } from './fenetres.svelte';

afterEach(() => {
  cleanup();
  pile.dialogues = [];
  pile.toasts = [];
  document.body.innerHTML = '';
});

/** Un bouton hors fenêtre, qui a le focus avant l'ouverture. */
function boutonOrigine() {
  const b = document.createElement('button');
  b.textContent = 'origine';
  document.body.appendChild(b);
  b.focus();
  return b;
}

describe('Confirmer', () => {
  it('met le focus sur le bouton de validation, puis le rend à l’origine en fermant par Échap', async () => {
    const origine = boutonOrigine();
    render(Dialogues);
    const reponse = confirmer('🗑 Retirer ce jeu ?');
    await tick();
    expect(document.activeElement?.textContent?.trim()).toBe('Confirmer');

    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(await reponse).toBe(false);
    await tick();
    expect(document.activeElement).toBe(origine);
  });

  it('rend vrai quand on confirme, avec le bouton « Supprimer » en liseré si danger', async () => {
    render(Dialogues);
    const reponse = confirmer('Supprimer la sauvegarde ?', { danger: true });
    await tick();
    const bouton = screen.getByRole('button', { name: 'Supprimer' });
    expect(bouton.classList.contains('danger')).toBe(true);
    expect(bouton.classList.contains('primary')).toBe(false);
    await fireEvent.click(bouton);
    expect(await reponse).toBe(true);
  });

  it('annule par un clic sur le voile', async () => {
    render(Dialogues);
    const reponse = confirmer('Continuer ?');
    await tick();
    await fireEvent.click(document.querySelector('.voile')!);
    expect(await reponse).toBe(false);
  });
});

describe('Demander', () => {
  it('présélectionne le texte proposé et le rend par Entrée', async () => {
    render(Dialogues);
    const reponse = demander('Nom du profil', { valeur: 'Seb' });
    await tick();
    const champ = screen.getByRole('textbox') as HTMLInputElement;
    expect(document.activeElement).toBe(champ);
    expect(champ.selectionStart).toBe(0);
    expect(champ.selectionEnd).toBe(3);
    await fireEvent.input(champ, { target: { value: 'Sébastien' } });
    await fireEvent.keyDown(champ, { key: 'Enter' });
    expect(await reponse).toBe('Sébastien');
  });

  it('rend null si on annule', async () => {
    render(Dialogues);
    const reponse = demander('Nom');
    await tick();
    await fireEvent.click(screen.getByRole('button', { name: 'Annuler' }));
    expect(await reponse).toBeNull();
  });
});

describe('Choisir, Informer, Toast', () => {
  it('rend la valeur choisie', async () => {
    render(Dialogues);
    const reponse = choisir('Emplacement', [
      { valeur: 'D', libelle: 'D:\\Jeux' },
      { valeur: 'E', libelle: 'E:\\Jeux' },
    ]);
    await tick();
    await fireEvent.click(screen.getByRole('button', { name: 'E:\\Jeux' }));
    expect(await reponse).toBe('E');
  });

  it('Informer n’a qu’un bouton OK', async () => {
    render(Dialogues);
    const reponse = informer('Jeton révoqué', 'Demande un nouveau jeton à un admin.');
    await tick();
    expect(screen.getAllByRole('button').map((b) => b.textContent?.trim())).toEqual(['OK']);
    await fireEvent.click(screen.getByRole('button', { name: 'OK' }));
    await expect(reponse).resolves.toBeUndefined();
  });

  it('seule la fenêtre du dessus réagit au clavier', async () => {
    render(Dialogues);
    const dessous = confirmer('Dessous');
    await tick();
    const dessus = confirmer('Dessus');
    await tick();
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(await dessus).toBe(false);
    expect(pile.dialogues.map((d) => d.titre)).toEqual(['Dessous']);
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(await dessous).toBe(false);
  });

  it('un toast déduit son type et se ferme d’un clic', async () => {
    render(Dialogues);
    toast('⛔ Refusé : jeton expiré');
    await tick();
    const t = screen.getByRole('button', { name: /Refusé/ });
    expect(t.classList.contains('erreur')).toBe(true);
    await fireEvent.click(t);
    expect(pile.toasts).toHaveLength(0);
  });
});

describe('Une fenêtre maison garde la main (menu en jeu, Taodbox)', () => {
  it('« confirmer » (rôle alertdialog) est reconnue : Échap/B la ferme sans reprendre le jeu', async () => {
    const { fenetreOuverte } = await import('$lib/taodbox/manette.svelte');
    render(Dialogues);
    expect(fenetreOuverte()).toBe(false);
    void confirmer('⏹ Quitter le jeu ?');
    await tick();
    expect(fenetreOuverte()).toBe(true);
  });
});

describe('Une action dangereuse sélectionne « Annuler » d’abord', () => {
  it('« Quitter le jeu ? » : deux appuis sur A ne quittent pas par erreur', async () => {
    render(Dialogues);
    void confirmer('⏹ Quitter le jeu ?', { libelleValider: 'Quitter', danger: true });
    await tick();
    expect(document.activeElement?.textContent?.trim()).toBe('Annuler');
    void confirmer('🗑 Autre chose ?');
    await tick();
    expect(document.activeElement?.textContent?.trim()).toBe('Confirmer'); // sans danger : la validation, comme avant
  });
});
