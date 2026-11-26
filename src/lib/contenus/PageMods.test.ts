// 🧩 La page des mods d'un jeu Switch (✅ Seb 26/11 : « une fenêtre dédiée, avec une barre de recherche et un
// classement par type ») : onglets et nombres, filtres d'état, recherche (bureau), paquets, détail, conflit dit
// d'avance, « Tout télécharger » limité à l'onglet (⭐ cochés d'office), focus rendu, clavier ; au canapé : pas de
// recherche, LB / RB, A déplie, B replie puis revient.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, within } from '@testing-library/svelte';
import { DK, REMIX, SHA, attendre, choisirMod, cocher, confirmer, dk, liste, mod, remettre, simu, tailleMusiques, telecharger, INACTIF, type Mod } from './essaisMods';

vi.mock('$lib/api', async () => (await import('./essaisMods')).apiSimulee());
vi.mock('$lib/dialogues/fenetres.svelte', async () => (await import('./essaisMods')).fenetresSimulees());
vi.mock('$lib/etat.svelte', async () => (await import('./essaisMods')).etatSimule());
vi.mock('$lib/ludotheque/jeu.svelte', async () => (await import('./essaisMods')).jeuSimule());
vi.mock('$lib/ludotheque/telechargements.svelte', async () => (await import('./essaisMods')).telechargementsSimules());
const adresse = { url: new URL('http://localhost/mods?jeu=7&titre=Zelda') };
vi.mock('$app/state', () => ({
  get page() {
    return adresse;
  },
}));

import PageMods from './PageMods.svelte';
import { appliquer } from '$lib/taodbox/manette.svelte';

const retour = vi.spyOn(history, 'back').mockImplementation(() => {});

async function ouvrir(canape = false) {
  adresse.url = new URL(`http://localhost${canape ? '/taodbox' : ''}/mods?jeu=7&titre=Zelda`);
  if (canape) document.documentElement.setAttribute('data-taodbox', '');
  render(PageMods);
  await attendre();
}
const ligneDe = (cle: string) => document.querySelector<HTMLElement>(`li[data-cle="${cle}"]`)!;
const nomsAffiches = () => [...document.querySelectorAll<HTMLElement>('li[data-cle]')].map((l) => l.dataset.cle);
const detail = () => screen.getByRole('complementary', { name: 'Fiche du mod' });
/** Des mods numérotés (clés « p-000000000001 »…), pour les paquets. */
const nombreux = (n: number, x: Mod = {}) => Array.from({ length: n }, (_, i) => mod({ cle: `p-${String(i + 1).padStart(12, '0')}`, nom: `Mod ${i + 1}`, ...x }));

beforeEach(() => {
  remettre();
  retour.mockClear();
  document.documentElement.removeAttribute('data-taodbox');
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  document.documentElement.removeAttribute('data-taodbox');
});
Element.prototype.scrollIntoView = () => {};

describe('la page des mods, au bureau', () => {
  it('chaque ligne : le nom, sa sorte, UNE commande ; la fiche dit l’auteur, la licence, les mentions ; jamais de jargon', async () => {
    simu.reponse = liste([
      mod({ cle: 'a-000000000001', nom: 'Anisotropic Fix', etat: 'telecharge', revision_rangee: 1, allume: true, licence: '', type: 'technique', interet: 5 }),
      mod({ cle: 'b-000000000001', nom: 'Clean UI', compat: 'pas_verifie', emulateurs_essayes: [] }),
    ]);
    await ouvrir();
    expect(screen.getByRole('heading', { name: '🧩 Mods — Zelda' })).toBeTruthy();
    const inter = screen.getAllByRole('switch');
    expect(inter).toHaveLength(1);
    expect(inter[0].getAttribute('aria-checked')).toBe('true');
    expect(inter[0].getAttribute('aria-label')).toBe('Anisotropic Fix');
    expect(document.getElementById(inter[0].getAttribute('aria-describedby')!)?.textContent).toBe('✅ Allumé');
    expect(within(ligneDe('a-000000000001')).getByText('⭐ Recommandé · amélioration technique')).toBeTruthy();
    expect(within(ligneDe('b-000000000001')).getByRole('button', { name: '⬇ Télécharger (1000 o)' })).toBeTruthy();
    // La fiche du premier mod, à droite : « licence non précisée » (jamais « aucune »).
    expect(within(detail()).getByText('par Wollnashorn · licence non précisée')).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Clean UI' }));
    expect(within(detail()).getByText(/Version du jeu pas vérifiée/)).toBeTruthy();
    expect(within(detail()).getByText('pas encore essayé dans un émulateur')).toBeTruthy();
    expect(within(detail()).getByText('par Wollnashorn · licence : MIT')).toBeTruthy();
    // Jamais d'empreinte, de révision, de clé, de contenu technique.
    expect(document.body.textContent).not.toMatch(/exefs|romfs|build id|sha|révision|wollnashorn-0000|abab/i);
  });

  it('un mod grisé : jamais d’interrupteur, un bouton focalisable relié à sa raison visible', async () => {
    simu.reponse = liste([mod({ etat: 'incompatible', compat: 'incompatible', raison: 'fait pour la version 1.5.0 du jeu (build id 8E99).' })]);
    await ouvrir();
    expect(screen.queryAllByRole('switch')).toHaveLength(0);
    const b = screen.getByRole('button', { name: /Ne va pas avec ta version/ });
    expect(b.getAttribute('aria-disabled')).toBe('true');
    expect(b.hasAttribute('disabled')).toBe(false);
    expect(b.hasAttribute('title')).toBe(false);
    expect(document.getElementById(b.getAttribute('aria-describedby')!)?.textContent).toBe('Fait pour la version 1.5.0 du jeu (repère de version 8E99).');
    expect(screen.queryByRole('button', { name: /Télécharger/ })).toBeNull();
  });

  it('« Télécharger » : l’accord chiffré, puis la liste VUE donnée au cœur', async () => {
    simu.reponse = liste([mod()]);
    await ouvrir();
    await fireEvent.click(screen.getByRole('button', { name: '⬇ Télécharger (1000 o)' }));
    await attendre();
    expect(confirmer.mock.calls[0][0]).toBe('🧩 Télécharger « 1080p » (1000 o) ?');
    expect(telecharger).toHaveBeenCalledWith(7, [{ cle: 'wollnashorn-000000000001', revision: 1, taille: 1000, sha256: SHA }], 1000, false);
  });

  it('les onglets par sorte, avec leur nombre ; un onglet vide ne se montre pas', async () => {
    simu.reponse = liste([
      mod({ cle: 'a-000000000001', nom: 'A', type: 'technique', interet: 5 }),
      mod({ cle: 'b-000000000001', nom: 'B', type: 'cosmetique', interet: 2 }),
      mod({ cle: 'c-000000000001', nom: 'C', type: 'cosmetique', interet: 4 }),
      mod({ cle: 'd-000000000001', nom: 'D', type: 'sorte-inconnue' }),
      mod({ cle: 'e-000000000001', nom: 'E', type: null }),
      dk({ type: 'contenu' }),
    ]);
    await ouvrir();
    const onglets = screen.getAllByRole('tab').map((t) => t.textContent?.trim());
    expect(onglets).toEqual(['Tout (6)', '⭐ Recommandés (2)', 'Technique (1)', 'Apparence (2)', 'Contenu (1)', 'Autre (1)', '🎵 Musiques (1)', '📝 Pas encore décrit (1)']);
    expect(screen.getByRole('tab', { name: 'Tout (6)' }).getAttribute('aria-selected')).toBe('true');
    await fireEvent.click(screen.getByRole('tab', { name: 'Apparence (2)' }));
    expect(nomsAffiches()).toEqual(['b-000000000001', 'c-000000000001']);
    expect(screen.getByRole('tab', { name: 'Apparence (2)' }).getAttribute('aria-selected')).toBe('true');
    await fireEvent.click(screen.getByRole('tab', { name: '⭐ Recommandés (2)' }));
    expect(nomsAffiches()).toEqual(['a-000000000001', 'c-000000000001']);
    await fireEvent.click(screen.getByRole('tab', { name: '🎵 Musiques (1)' }));
    expect(nomsAffiches()).toEqual([DK]);
    await fireEvent.click(screen.getByRole('tab', { name: 'Autre (1)' }));
    expect(nomsAffiches()).toEqual(['d-000000000001']);
    await fireEvent.click(screen.getByRole('tab', { name: '📝 Pas encore décrit (1)' }));
    expect(nomsAffiches()).toEqual(['e-000000000001']);
    // Un mod de musique : 🎵 à côté de son nom.
    await fireEvent.click(screen.getByRole('tab', { name: 'Tout (6)' }));
    expect(ligneDe(DK).querySelector('[data-sorte="nom"]')?.textContent).toContain('🎵');
  });

  it('les filtres d’état : un à la fois (aria-pressed), avec leur nombre dans l’onglet ; le même bouton l’enlève', async () => {
    simu.reponse = liste([
      mod({ cle: 'a-000000000001', nom: 'A', etat: 'telecharge', revision_rangee: 1, allume: true, type: 'technique' }),
      mod({ cle: 'b-000000000001', nom: 'B', etat: 'telecharge', revision_rangee: 1 }),
      mod({ cle: 'c-000000000001', nom: 'C', type: 'technique' }),
      mod({ cle: 'd-000000000001', nom: 'D', etat: 'revision_dispo', revision: 2, revision_rangee: 1 }),
      mod({ cle: 'e-000000000001', nom: 'E', etat: 'incompatible', compat: 'incompatible' }),
      mod({ cle: 'f-000000000001', nom: 'F', etat: 'telecharge', revision_rangee: 1, servi: false }),
    ]);
    await ouvrir();
    const filtres = within(screen.getByRole('group', { name: 'Filtrer par état' }));
    expect(filtres.getAllByRole('button').map((b) => b.textContent?.trim())).toEqual([
      'Allumés (1)',
      'Téléchargés (4)',
      'À télécharger (1)',
      'Mises à jour (1)',
      'Ne vont pas avec ta version (1)',
      'Plus proposés par Firehouse (1)',
    ]);
    const allumes = filtres.getByRole('button', { name: 'Allumés (1)' });
    expect(allumes.getAttribute('aria-pressed')).toBe('false');
    await fireEvent.click(allumes);
    expect(allumes.getAttribute('aria-pressed')).toBe('true');
    expect(nomsAffiches()).toEqual(['a-000000000001']);
    await fireEvent.click(filtres.getByRole('button', { name: 'Téléchargés (4)' }));
    expect(nomsAffiches()).toEqual(['a-000000000001', 'b-000000000001', 'd-000000000001', 'f-000000000001']);
    await fireEvent.click(filtres.getByRole('button', { name: 'Plus proposés par Firehouse (1)' }));
    expect(nomsAffiches()).toEqual(['f-000000000001']);
    await fireEvent.click(filtres.getByRole('button', { name: 'Plus proposés par Firehouse (1)' }));
    expect(nomsAffiches()).toHaveLength(6);
    // Dans un onglet, les nombres sont ceux de l'onglet.
    await fireEvent.click(screen.getByRole('tab', { name: 'Technique (2)' }));
    expect(within(screen.getByRole('group', { name: 'Filtrer par état' })).getAllByRole('button').map((b) => b.textContent?.trim())).toEqual(['Allumés (1)', 'Téléchargés (1)', 'À télécharger (1)']);
  });

  it('la recherche : nom, description et auteur, sans majuscules ni accents ; le compte ; une liste vide dit pourquoi', async () => {
    simu.reponse = liste([
      mod({ cle: 'a-000000000001', nom: 'Écran large', description: 'Le jeu en 21:9.', auteur: 'Alice' }),
      mod({ cle: 'b-000000000001', nom: 'Couleurs', description: 'Des couleurs plus vives à l’écran.', auteur: 'Bob' }),
      mod({ cle: 'c-000000000001', nom: 'Musique', description: 'Rien à voir.', auteur: 'Écrandor' }),
    ]);
    await ouvrir();
    const champ = screen.getByRole('searchbox', { name: 'Chercher un mod' });
    await fireEvent.input(champ, { target: { value: 'ecran' } });
    expect(nomsAffiches()).toEqual(['a-000000000001', 'b-000000000001', 'c-000000000001']);
    await fireEvent.input(champ, { target: { value: 'ECRAN vives' } });
    expect(nomsAffiches()).toEqual(['b-000000000001']);
    expect(screen.getByText('1 sur 3')).toBeTruthy();
    await fireEvent.input(champ, { target: { value: 'bob' } });
    expect(nomsAffiches()).toEqual(['b-000000000001']);
    await fireEvent.input(champ, { target: { value: 'zzz' } });
    expect(nomsAffiches()).toEqual([]);
    expect(screen.getByText('Aucun mod ne correspond à « zzz » : essaie un autre mot.')).toBeTruthy();
    // Un seul message : pas de « Aucun mod ne correspond : essaie un autre mot. » en double.
    expect(screen.queryByText(/^Aucun mod ne correspond : essaie/)).toBeNull();
    // Échap dans le champ : il se vide (la page reste ouverte).
    await fireEvent.keyDown(champ, { key: 'Escape' });
    expect(retour).not.toHaveBeenCalled();
    expect(nomsAffiches()).toHaveLength(3);
  });

  it('un filtre vide dit pourquoi (jamais une liste muette)', async () => {
    simu.reponse = liste([mod({ cle: 'a-000000000001', nom: 'A', etat: 'telecharge', revision_rangee: 1, allume: true }), mod({ cle: 'b-000000000001', nom: 'B', type: 'triche' })]);
    await ouvrir();
    await fireEvent.click(screen.getByRole('button', { name: 'Allumés (1)' }));
    await fireEvent.click(screen.getByRole('tab', { name: 'Triche (1)' }));
    // Le filtre gardé ne se montre plus (0 dans l'onglet)… sauf s'il est allumé : il reste pour pouvoir l'enlever.
    expect(screen.getByRole('button', { name: 'Allumés (0)' }).getAttribute('aria-pressed')).toBe('true');
    expect(screen.getByText('Aucun mod « Allumés » dans « Triche » : enlève le filtre pour tout voir.')).toBeTruthy();
  });

  it('par paquets de 50 : « Voir N de plus », et retour au premier paquet quand l’onglet ou le filtre change', async () => {
    simu.reponse = liste([...nombreux(110), mod({ cle: 'z-000000000001', nom: 'Z', type: 'triche' })]);
    await ouvrir();
    // (Des sélections directes : `getByRole` est très lent sur une centaine de lignes.)
    const voirPlus = () => [...document.querySelectorAll<HTMLButtonElement>('.colonne > button')].find((b) => /de plus/.test(b.textContent ?? ''));
    const onglet = (cle: string) => document.querySelector<HTMLButtonElement>(`[role="tab"][data-onglet="${cle}"]`)!;
    expect(nomsAffiches()).toHaveLength(50);
    expect(voirPlus()?.textContent?.trim()).toBe('Voir 50 de plus');
    await fireEvent.click(voirPlus()!);
    await attendre();
    expect(nomsAffiches()).toHaveLength(100);
    // Le focus va au premier mod ajouté.
    expect(document.activeElement?.closest('li')?.dataset.cle).toBe('p-000000000051');
    expect(voirPlus()?.textContent?.trim()).toBe('Voir 11 de plus');
    await fireEvent.click(voirPlus()!);
    expect(nomsAffiches()).toHaveLength(111);
    expect(voirPlus()).toBeUndefined();
    await fireEvent.click(onglet('triche'));
    await fireEvent.click(onglet('tout'));
    expect(onglet('tout').textContent?.trim()).toBe('Tout (111)');
    expect(nomsAffiches()).toHaveLength(50);
  });

  it('le détail : la colonne de droite suit le mod choisi (clic ou focus) ; la description entière', async () => {
    const longue = `${'x'.repeat(200)} fin.`;
    simu.reponse = liste([mod({ cle: 'a-000000000001', nom: 'A', description: longue }), mod({ cle: 'b-000000000001', nom: 'B', description: 'Courte.', auteur: 'Bob' })]);
    await ouvrir();
    expect(within(detail()).getByRole('heading', { name: 'A' })).toBeTruthy();
    // Dans la liste, la description tient sur deux lignes ; dans la fiche, elle est entière.
    expect(within(ligneDe('a-000000000001')).getByText(longue).classList.contains('replie')).toBe(true);
    expect(within(detail()).getByText(longue).classList.contains('replie')).toBe(false);
    screen.getByRole('button', { name: 'B' }).focus();
    await attendre();
    expect(within(detail()).getByRole('heading', { name: 'B' })).toBeTruthy();
    expect(within(detail()).getByText('par Bob · licence : MIT')).toBeTruthy();
  });

  it('le conflit est dit AVANT l’appui, sur la ligne et dans la fiche, et relié à l’interrupteur', async () => {
    simu.reponse = liste([
      mod({ cle: 'a-000000000001', nom: 'A', etat: 'telecharge', revision_rangee: 1, conflits: ['b-000000000001'] }),
      mod({ cle: 'b-000000000001', nom: 'B', etat: 'telecharge', revision_rangee: 1, allume: true }),
      mod({ cle: 'c-000000000001', nom: 'C', conflits: ['b-000000000001'] }),
    ]);
    await ouvrir();
    const a = screen.getByRole('switch', { name: 'A' });
    const ligne = within(ligneDe('a-000000000001')).getByText('⚠ Ne va pas avec « B », allumé : l’allumer éteint l’autre.');
    expect(a.getAttribute('aria-describedby')?.split(' ')).toContain(ligne.id);
    expect(within(detail()).getByText('⚠ Ne va pas avec « B », allumé : l’allumer éteint l’autre.')).toBeTruthy();
    // Un mod à télécharger arrivera éteint ; le mod allumé lui-même ne dit rien.
    expect(within(ligneDe('c-000000000001')).getByText('⚠ Ne va pas avec « B », allumé : il arrivera éteint.')).toBeTruthy();
    expect(within(ligneDe('b-000000000001')).queryByText(/Ne va pas avec/)).toBeNull();
    await fireEvent.click(a);
    await attendre();
    expect(choisirMod).toHaveBeenCalledWith(7, 'a-000000000001', true, 'b-000000000001');
  });

  it('« Tout télécharger » ne vaut que pour l’onglet affiché : sa taille dite, les ⭐ recommandés cochés d’office', async () => {
    simu.reponse = liste([
      mod({ cle: 'a-000000000001', nom: 'A', taille: 10, type: 'technique', interet: 5 }),
      mod({ cle: 'b-000000000001', nom: 'B', taille: 5, type: 'technique', interet: 1 }),
      mod({ cle: 'c-000000000001', nom: 'C', taille: 7, type: 'cosmetique' }),
    ]);
    await ouvrir();
    expect(screen.getByRole('button', { name: '⬇ Tout télécharger (3, 22 o)' })).toBeTruthy();
    await fireEvent.click(screen.getByRole('tab', { name: 'Technique (2)' }));
    await fireEvent.click(screen.getByRole('button', { name: '⬇ Tout télécharger ici (2, 15 o)' }));
    await attendre();
    const [, options, o] = cocher.mock.calls[0] as [string, { valeur: string }[], { coches: boolean[] }];
    expect(options.map((x) => x.valeur)).toEqual(['a-000000000001', 'b-000000000001']);
    expect(o.coches).toEqual([true, false]);
    // Un seul mod à télécharger dans l'onglet : son propre bouton suffit.
    await fireEvent.click(screen.getByRole('tab', { name: 'Apparence (1)' }));
    expect(screen.queryByRole('button', { name: /Tout télécharger/ })).toBeNull();
  });

  it('après « Télécharger », le focus va à la commande de la ligne ; le mod sorti du filtre reste affiché jusqu’au prochain changement', async () => {
    simu.reponse = liste([mod({ cle: 'a-000000000001', nom: 'A' }), mod({ cle: 'b-000000000001', nom: 'B' }), mod({ cle: 'c-000000000001', nom: 'C', etat: 'telecharge', revision_rangee: 1 })]);
    await ouvrir();
    await fireEvent.click(screen.getByRole('button', { name: 'À télécharger (2)' }));
    const bouton = within(ligneDe('a-000000000001')).getByRole('button', { name: /Télécharger/ });
    bouton.focus();
    // Le mod arrive tout de suite (très léger) : sa ligne montre alors son interrupteur, le bouton a disparu.
    telecharger.mockImplementation(async () => {
      simu.reponse = liste([mod({ cle: 'a-000000000001', nom: 'A', etat: 'telecharge', revision_rangee: 1, allume: true }), mod({ cle: 'b-000000000001', nom: 'B' }), mod({ cle: 'c-000000000001', nom: 'C', etat: 'telecharge', revision_rangee: 1 })]);
      simu.suiviReponse = { ...INACTIF, faits: 1, a_faire: 1 };
      return 1;
    });
    await fireEvent.click(bouton);
    await attendre();
    const ici = document.activeElement as HTMLElement;
    expect(ici.getAttribute('role')).toBe('switch');
    expect(ici.closest('li')?.dataset.cle).toBe('a-000000000001');
    expect(ici.getAttribute('tabindex')).toBe('0');
    // Il n'est plus « à télécharger », mais reste là avec son nouvel état.
    expect(nomsAffiches()).toEqual(['a-000000000001', 'b-000000000001']);
    await fireEvent.click(screen.getByRole('button', { name: 'À télécharger (1)' }));
    await fireEvent.click(screen.getByRole('button', { name: 'À télécharger (1)' }));
    expect(nomsAffiches()).toEqual(['b-000000000001']);
  });

  it('au clavier : haut / bas d’un mod à l’autre, gauche / droite vers la commande ; une seule commande reçoit Tab', async () => {
    simu.reponse = liste([mod({ cle: 'a-000000000001', nom: 'A' }), mod({ cle: 'b-000000000001', nom: 'B', etat: 'telecharge', revision_rangee: 1 }), mod({ cle: 'c-000000000001', nom: 'C' })]);
    await ouvrir();
    const tabulables = () => [...document.querySelectorAll<HTMLElement>('li[data-cle] button')].filter((b) => b.tabIndex === 0);
    expect(tabulables().map((b) => b.textContent?.trim())).toEqual(['A']);
    const a = screen.getByRole('button', { name: 'A' });
    a.focus();
    await fireEvent.keyDown(a, { key: 'ArrowDown' });
    expect(document.activeElement).toBe(screen.getByRole('button', { name: 'B' }));
    await fireEvent.keyDown(document.activeElement!, { key: 'ArrowRight' });
    expect(document.activeElement).toBe(screen.getByRole('switch', { name: 'B' }));
    await attendre();
    expect(tabulables()).toEqual([screen.getByRole('switch', { name: 'B' })]);
    // La même colonne en descendant : la commande de C.
    await fireEvent.keyDown(document.activeElement!, { key: 'ArrowDown' });
    expect(document.activeElement?.textContent).toContain('⬇ Télécharger');
    await fireEvent.keyDown(document.activeElement!, { key: 'ArrowLeft' });
    expect(document.activeElement).toBe(screen.getByRole('button', { name: 'C' }));
    await fireEvent.keyDown(document.activeElement!, { key: 'ArrowUp' });
    await fireEvent.keyDown(document.activeElement!, { key: 'ArrowUp' });
    expect(document.activeElement).toBe(a);
    // En haut : rien ne bouge.
    await fireEvent.keyDown(a, { key: 'ArrowUp' });
    expect(document.activeElement).toBe(a);
  });

  it('au clavier : sous le dernier mod, « Voir N de plus » ; Début et Fin', async () => {
    simu.reponse = liste(nombreux(55));
    await ouvrir();
    const nom = (i: number) => ligneDe(`p-${String(i).padStart(12, '0')}`).querySelector<HTMLElement>('[data-sorte="nom"]')!;
    nom(1).focus();
    await fireEvent.keyDown(nom(1), { key: 'End' });
    expect(document.activeElement).toBe(nom(50));
    await fireEvent.keyDown(nom(50), { key: 'ArrowDown' });
    expect(document.activeElement?.textContent?.trim()).toBe('Voir 5 de plus');
    nom(50).focus();
    await fireEvent.keyDown(nom(50), { key: 'Home' });
    expect(document.activeElement).toBe(nom(1));
  });

  it('la barre d’onglets se parcourt aux flèches (modèle ARIA) : l’onglet atteint est choisi, lui seul reçoit Tab', async () => {
    simu.reponse = liste([mod({ cle: 'a-000000000001', nom: 'A', type: 'technique' }), mod({ cle: 'b-000000000001', nom: 'B', type: 'triche' })]);
    await ouvrir();
    const tout = screen.getByRole('tab', { name: 'Tout (2)' });
    expect(screen.getAllByRole('tab').filter((t) => t.tabIndex === 0)).toEqual([tout]);
    tout.focus();
    await fireEvent.keyDown(tout, { key: 'ArrowRight' });
    await attendre();
    const tech = screen.getByRole('tab', { name: 'Technique (1)' });
    expect(document.activeElement).toBe(tech);
    expect(tech.getAttribute('aria-selected')).toBe('true');
    expect(nomsAffiches()).toEqual(['a-000000000001']);
    await fireEvent.keyDown(tech, { key: 'End' });
    await attendre();
    expect(document.activeElement).toBe(screen.getByRole('tab', { name: 'Triche (1)' }));
    await fireEvent.keyDown(document.activeElement!, { key: 'ArrowRight' });
    await attendre();
    expect(document.activeElement).toBe(screen.getByRole('tab', { name: 'Tout (2)' }));
  });

  it('un mod de musique : 🎵 caché au lecteur d’écran, qui entend « (mod de musique) »', async () => {
    simu.reponse = liste([dk()]);
    await ouvrir();
    const nom = ligneDe(DK).querySelector<HTMLElement>('[data-sorte="nom"]')!;
    expect(nom.querySelector('[aria-hidden="true"]')?.textContent).toBe(' 🎵');
    expect(nom.querySelector('.annonce')?.textContent).toBe(' (mod de musique)');
    expect(nom.querySelector('[aria-label]')).toBeNull();
  });

  it('la fiche suit la liste AFFICHÉE : un mod choisi puis filtré laisse la place au premier affiché', async () => {
    simu.reponse = liste([mod({ cle: 'a-000000000001', nom: 'A', type: 'technique' }), mod({ cle: 'b-000000000001', nom: 'B', type: 'triche' })]);
    await ouvrir();
    await fireEvent.click(screen.getByRole('button', { name: 'B' }));
    expect(within(detail()).getByRole('heading', { name: 'B' })).toBeTruthy();
    await fireEvent.click(screen.getByRole('tab', { name: 'Technique (1)' }));
    expect(within(detail()).getByRole('heading', { name: 'A' })).toBeTruthy();
    // Chaque aria-describedby vise un élément présent.
    for (const e of document.querySelectorAll('[aria-describedby]'))
      for (const id of e.getAttribute('aria-describedby')!.split(' ')) expect(document.getElementById(id)).toBeTruthy();
  });

  it('une nouvelle révision : « Mettre à jour » sur la ligne (et son état en texte), l’interrupteur dans la fiche', async () => {
    simu.reponse = liste([mod({ etat: 'revision_dispo', revision: 2, revision_rangee: 1, allume: true })]);
    await ouvrir();
    expect(within(ligneDe('wollnashorn-000000000001')).getByRole('button', { name: '⬇ Mettre à jour (1000 o)' })).toBeTruthy();
    const etatTexte = within(ligneDe('wollnashorn-000000000001')).getByText('✅ Allumé');
    expect(etatTexte.closest('button')).toBeNull();
    expect(within(ligneDe('wollnashorn-000000000001')).queryByRole('switch')).toBeNull();
    expect(within(detail()).getByRole('switch', { name: '1080p' }).getAttribute('aria-checked')).toBe('true');
  });

  it('pendant une partie : rien ne change, et c’est dit', async () => {
    simu.partie.enJeu = 3;
    simu.reponse = liste([mod({ etat: 'telecharge', revision_rangee: 1 }), mod({ cle: 'b-000000000001', nom: 'B' })]);
    await ouvrir();
    const inter = screen.getByRole('switch');
    expect(inter.getAttribute('aria-disabled')).toBe('true');
    await fireEvent.click(inter);
    await attendre();
    expect(choisirMod).not.toHaveBeenCalled();
    expect(screen.getByText(/Une partie est en cours/)).toBeTruthy();
    expect((screen.getByRole('button', { name: /Télécharger/ }) as HTMLButtonElement).disabled).toBe(true);
  });

  it('pendant un téléchargement : le pourcentage sur la ligne, seule l’étape annoncée', async () => {
    simu.reponse = liste([mod(), mod({ cle: 'b-000000000001', nom: 'B' })]);
    simu.suiviReponse = { ...INACTIF, actif: true, en_cours: '1080p', recus: 450, total: 1000, a_faire: 1 };
    await ouvrir();
    expect(screen.getByText('⬇ « 1080p » : 45 %')).toBeTruthy();
    expect(within(ligneDe('wollnashorn-000000000001')).getByText('⬇ 45 %')).toBeTruthy();
    const annonces = [...document.querySelectorAll('[aria-live="polite"]')].map((e) => e.textContent);
    expect(annonces).toContain('Téléchargement du mod « 1080p ».');
    expect(annonces.join(' ')).not.toContain('%');
  });

  it('un mod que Firehouse ne sert plus : dit, et « Retirer » (dans sa fiche) après confirmation', async () => {
    simu.reponse = liste([mod({ etat: 'telecharge', revision_rangee: 1, servi: false, description: '', auteur: '', licence: '' })]);
    await ouvrir();
    expect(within(detail()).getByText(/Firehouse ne le propose plus/)).toBeTruthy();
    await fireEvent.click(within(detail()).getByRole('button', { name: /Retirer/ }));
    await attendre();
    expect((confirmer.mock.calls[0][1] as { danger: boolean }).danger).toBe(true);
  });

  it('hors ligne : la liste vue la dernière fois, rien à télécharger', async () => {
    const s = new Date(2026, 10, 18, 14, 5).getTime() / 1000;
    simu.reponse = liste([mod(), mod({ cle: 'b-000000000001', nom: 'B' })], { etat: 'hors_ligne', vue_le: String(s) });
    await ouvrir();
    expect(screen.getByText(/vue le 18\/11 à 14:05/)).toBeTruthy();
    expect(screen.getAllByRole('button', { name: /⬇ Télécharger/ }).every((b) => (b as HTMLButtonElement).disabled)).toBe(true);
    expect(screen.queryByRole('button', { name: /Tout télécharger/ })).toBeNull();
  });

  it('la fiche de Firehouse se lit : « ⭐ Recommandé », la sorte en mots simples (jamais « cosmetique »)', async () => {
    simu.reponse = liste([
      mod({ cle: 'a-000000000001', nom: 'A', description: 'Le jeu s’affiche en 1080p.', interet: 5, type: 'technique' }),
      mod({ cle: 'b-000000000001', nom: 'B', description: 'Un nouveau costume.', interet: 2, type: 'cosmetique' }),
    ]);
    await ouvrir();
    expect(within(ligneDe('a-000000000001')).getByText('Le jeu s’affiche en 1080p.')).toBeTruthy();
    expect(within(ligneDe('b-000000000001')).getByText('apparence')).toBeTruthy();
    expect(within(ligneDe('b-000000000001')).queryByText(/Recommandé/)).toBeNull();
    expect(document.body.textContent).not.toContain('cosmetique');
  });

  it('la dernière partie : l’interrupteur dit « ⚠ Allumé, pas utilisé »', async () => {
    simu.reponse = liste([mod({ etat: 'telecharge', revision_rangee: 1, allume: true })], {
      derniere_pose: { quand: '1', emulateur: 'Eden', raisons: ['« 1080p » pas posé : réglage d’émulateur requis : résolution ×2'] },
    });
    await ouvrir();
    expect(screen.getByRole('switch').textContent).toContain('⚠ Allumé, pas utilisé');
  });

  it('Échap revient d’où l’on vient ; jamais par-dessus une fenêtre maison', async () => {
    simu.reponse = liste([mod()]);
    await ouvrir();
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(retour).toHaveBeenCalledTimes(1);
    const d = document.createElement('div');
    d.setAttribute('role', 'dialog');
    document.body.append(d);
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(retour).toHaveBeenCalledTimes(1);
    d.remove();
  });

  it('allumer un mod de musique : « ⏳ Je regarde la taille des musiques… » pendant la lecture', async () => {
    let finir: (n: number) => void = () => {};
    tailleMusiques.mockImplementation(() => new Promise<number>((r) => (finir = r)));
    simu.reponse = liste([dk({ etat: 'telecharge', revision_rangee: 1 }), mod({ cle: REMIX, nom: 'Remix' })]);
    await ouvrir();
    await fireEvent.click(screen.getByRole('switch', { name: 'DK Summit' }));
    await attendre();
    expect(screen.getByText('⏳ Je regarde la taille des musiques…')).toBeTruthy();
    finir(41018496);
    await attendre();
    expect(screen.queryByText('⏳ Je regarde la taille des musiques…')).toBeNull();
    expect(confirmer.mock.calls[0][0]).toBe('🎵 Allumer « DK Summit » (+ 41018496 o de musiques) ?');
  });
});

describe('la page des mods, au canapé', () => {
  it('pas de recherche, des paquets de 25, la sélection sur le premier mod', async () => {
    simu.reponse = liste(nombreux(30));
    await ouvrir(true);
    expect(screen.queryByRole('searchbox')).toBeNull();
    expect(nomsAffiches()).toHaveLength(25);
    expect(screen.getByRole('button', { name: 'Voir 5 de plus' })).toBeTruthy();
    expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Mod 1' }));
    // Pas de colonne de détail : la ligne se déplie.
    expect(screen.queryByRole('complementary')).toBeNull();
  });

  it('LB / RB changent d’onglet (en boucle), la sélection suit', async () => {
    simu.reponse = liste([mod({ cle: 'a-000000000001', nom: 'A', type: 'technique' }), mod({ cle: 'b-000000000001', nom: 'B', type: 'triche' })]);
    await ouvrir(true);
    appliquer('suivant');
    await attendre();
    expect(screen.getByRole('tab', { name: 'Technique (1)' }).getAttribute('aria-selected')).toBe('true');
    expect(document.activeElement).toBe(screen.getByRole('tab', { name: 'Technique (1)' }));
    expect(nomsAffiches()).toEqual(['a-000000000001']);
    appliquer('suivant');
    await attendre();
    expect(nomsAffiches()).toEqual(['b-000000000001']);
    appliquer('suivant');
    appliquer('precedent');
    appliquer('precedent');
    await attendre();
    expect(screen.getByRole('tab', { name: 'Technique (1)' }).getAttribute('aria-selected')).toBe('true');
  });

  it('A sur le nom déplie la fiche dans la ligne ; B la replie, puis B revient', async () => {
    simu.reponse = liste([mod({ description: 'Rend le jeu en 1080p.', licence: '' })]);
    await ouvrir(true);
    const nom = screen.getByRole('button', { name: '1080p' });
    expect(nom.getAttribute('aria-expanded')).toBe('false');
    nom.focus();
    appliquer('valider');
    await attendre();
    expect(nom.getAttribute('aria-expanded')).toBe('true');
    expect(document.getElementById(nom.getAttribute('aria-controls')!)?.textContent).toContain('licence non précisée');
    appliquer('retour');
    await attendre();
    expect(nom.getAttribute('aria-expanded')).toBe('false');
    expect(retour).not.toHaveBeenCalled();
    expect(document.activeElement).toBe(nom);
    appliquer('retour');
    expect(retour).toHaveBeenCalledTimes(1);
  });

  it('à la manette : A bascule l’interrupteur choisi ; le bouton d’un mod grisé est atteignable mais ne fait rien', async () => {
    simu.reponse = liste([
      mod({ cle: 'a-000000000001', nom: 'A', etat: 'telecharge', revision_rangee: 1 }),
      mod({ cle: 'b-000000000001', nom: 'B', etat: 'incompatible', compat: 'incompatible', raison: 'Autre version.' }),
    ]);
    await ouvrir(true);
    const grise = screen.getByRole('button', { name: /Ne va pas avec ta version/ });
    grise.focus();
    expect(document.activeElement).toBe(grise);
    appliquer('valider');
    await attendre();
    expect(choisirMod).not.toHaveBeenCalled();
    screen.getByRole('switch', { name: 'A' }).focus();
    appliquer('valider');
    await attendre();
    expect(choisirMod).toHaveBeenCalledWith(7, 'a-000000000001', true, null);
  });

  it('après « Télécharger », le focus va à une commande de la ligne que la manette sait quitter (jamais en haut)', async () => {
    simu.reponse = liste([mod({ cle: 'a-000000000001', nom: 'A' }), mod({ cle: 'b-000000000001', nom: 'B' })]);
    await ouvrir(true);
    const bouton = within(ligneDe('a-000000000001')).getByRole('button', { name: /Télécharger/ });
    bouton.focus();
    telecharger.mockImplementation(async () => {
      simu.reponse = liste([mod({ cle: 'a-000000000001', nom: 'A', etat: 'telecharge', revision_rangee: 1, allume: true }), mod({ cle: 'b-000000000001', nom: 'B' })]);
      simu.suiviReponse = { ...INACTIF, faits: 1, a_faire: 1 };
      return 1;
    });
    await fireEvent.click(bouton);
    await attendre();
    const ici = document.activeElement as HTMLElement;
    expect(ici.getAttribute('role')).toBe('switch');
    expect(ici.closest('li')?.dataset.cle).toBe('a-000000000001');
    // La VRAIE manette (géométrie : chaque commande sous la précédente) : « bas » descend depuis là, sans repartir
    // du haut de la page.
    const commandes = [...document.querySelectorAll<HTMLElement>('button:not([disabled]), [role="switch"]')];
    commandes.forEach(
      (b, i) => (b.getBoundingClientRect = () => ({ left: 0, top: i * 50, width: 100, height: 40, right: 100, bottom: i * 50 + 40, x: 0, y: i * 50, toJSON() {} }) as DOMRect),
    );
    appliquer('bas');
    const apres = document.activeElement as HTMLElement;
    expect(commandes.indexOf(apres)).toBeGreaterThan(commandes.indexOf(ici));
  });
});
