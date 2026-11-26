// 🧩 La bande des mods Switch (✅ Seb 26/11) : TOUJOURS un résumé et « 🧩 Ouvrir les mods (N) », même pour 2 mods ;
// ce que la dernière partie a dit, les musiques et « Préparer », la progression ; la liste est sur la page des mods.
// (La logique — accords, conflits, musiques, mises à jour — est testée dans modsSwitch.test.ts ; la page dans
// PageMods.test.ts.)
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { attendre, confirmer, dk, goto, liste, lister, mod, preparerMusiques, remettre, simu, suivre, tailleMusiques, telecharger, toast, INACTIF } from './essaisMods';

vi.mock('$lib/api', async () => (await import('./essaisMods')).apiSimulee());
vi.mock('$lib/dialogues/fenetres.svelte', async () => (await import('./essaisMods')).fenetresSimulees());
vi.mock('$lib/etat.svelte', async () => (await import('./essaisMods')).etatSimule());
vi.mock('$lib/ludotheque/jeu.svelte', async () => (await import('./essaisMods')).jeuSimule());
vi.mock('$lib/ludotheque/telechargements.svelte', async () => (await import('./essaisMods')).telechargementsSimules());
vi.mock('$app/navigation', async () => {
  const e = await import('./essaisMods');
  return { goto: (...a: unknown[]) => e.goto(...a) };
});

import BlocMods from './BlocMods.svelte';
import { reprendreRetourMods, retenirRetourMods } from './pageMods';

async function ouvrir() {
  render(BlocMods, { id: 7, titre: 'Zelda' });
  await attendre();
}

beforeEach(() => {
  remettre();
  reprendreRetourMods(-1);
  document.documentElement.removeAttribute('data-taodbox');
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  document.documentElement.removeAttribute('data-taodbox');
});
Element.prototype.scrollIntoView = () => {};

describe('la bande des mods Switch', () => {
  it('ne se montre pas sans mod', async () => {
    await ouvrir();
    expect(lister).toHaveBeenCalledWith(7);
    expect(screen.queryByText(/Mods :/)).toBeNull();
    expect(screen.queryByRole('button', { name: /Ouvrir les mods/ })).toBeNull();
  });

  it('même pour 2 mods : le résumé et « Ouvrir les mods (2) », jamais la liste ni un interrupteur', async () => {
    simu.reponse = liste([mod({ cle: 'a-000000000001', nom: 'Anisotropic Fix', etat: 'telecharge', revision_rangee: 1, allume: true }), mod({ cle: 'b-000000000001', nom: 'Clean UI' })]);
    await ouvrir();
    expect(screen.getByText('1 allumé sur 1, 1 à télécharger')).toBeTruthy();
    expect(screen.getByRole('button', { name: '🧩 Ouvrir les mods (2)' })).toBeTruthy();
    expect(screen.queryAllByRole('switch')).toHaveLength(0);
    expect(screen.queryByText('Anisotropic Fix')).toBeNull();
    expect(screen.queryByRole('button', { name: /Télécharger/ })).toBeNull();
    // Rien n'est téléchargé en ouvrant la fiche.
    expect(telecharger).not.toHaveBeenCalled();
  });

  it('« Ouvrir les mods » : la page du bureau (ou du canapé), et le retour est retenu pour ce jeu', async () => {
    simu.reponse = liste([mod()]);
    await ouvrir();
    await fireEvent.click(screen.getByRole('button', { name: /Ouvrir les mods/ }));
    expect(goto).toHaveBeenCalledWith('/mods?jeu=7&titre=Zelda');
    expect(reprendreRetourMods(7)).toBe(true);
    cleanup();
    document.documentElement.setAttribute('data-taodbox', '');
    await ouvrir();
    await fireEvent.click(screen.getByRole('button', { name: /Ouvrir les mods/ }));
    expect(goto).toHaveBeenLastCalledWith('/taodbox/mods?jeu=7&titre=Zelda');
  });

  it('au retour de la page : le focus revient sur « Ouvrir les mods » (une seule fois)', async () => {
    simu.reponse = liste([mod()]);
    retenirRetourMods(7);
    await ouvrir();
    expect(document.activeElement).toBe(screen.getByRole('button', { name: /Ouvrir les mods/ }));
    cleanup();
    await ouvrir();
    expect(document.activeElement).not.toBe(screen.getByRole('button', { name: /Ouvrir les mods/ }));
  });

  it('au retour : le focus n’est jamais volé à ce que la personne a choisi entre-temps', async () => {
    simu.reponse = liste([mod()]);
    retenirRetourMods(7);
    const ailleurs = document.createElement('button');
    ailleurs.textContent = 'Autre chose';
    document.body.append(ailleurs);
    ailleurs.focus();
    await ouvrir();
    expect(document.activeElement).toBe(ailleurs);
    ailleurs.remove();
  });

  it('au retour : « ← Retour » posé d’office (Réglages du jeu) laisse la place à « Ouvrir les mods »', async () => {
    simu.reponse = liste([mod()]);
    retenirRetourMods(7);
    const retour = document.createElement('button');
    retour.textContent = '← Retour';
    document.body.append(retour);
    retour.focus();
    await ouvrir();
    expect(document.activeElement).toBe(screen.getByRole('button', { name: /Ouvrir les mods/ }));
    retour.remove();
  });

  it('la dernière partie : ce qui n’a pas été posé est dit, et le nombre de « ⚠ Allumé, pas utilisé »', async () => {
    const s = new Date(2026, 10, 18, 14, 5).getTime() / 1000;
    simu.reponse = liste([mod({ etat: 'telecharge', revision_rangee: 1, allume: true })], {
      derniere_pose: { quand: String(s), emulateur: 'Eden', raisons: ['« 1080p » pas posé : réglage d’émulateur requis : résolution ×2'] },
      remarque: 'Ryubing n’a pas été installé par Frogtend.',
    });
    await ouvrir();
    expect(screen.getByText('⚠ À la dernière partie (Eden, le 18/11 à 14:05) :')).toBeTruthy();
    expect(screen.getByText('• « 1080p » pas posé : réglage d’émulateur requis : résolution ×2')).toBeTruthy();
    expect(screen.getByText('Ryubing n’a pas été installé par Frogtend.')).toBeTruthy();
    expect(screen.getByText(/⚠ Allumé, pas utilisé : 1 mod/)).toBeTruthy();
  });

  it('Firehouse injoignable et rien de lu avant : la bande le dit (jamais muette)', async () => {
    simu.reponse = liste([], { etat: 'injoignable' });
    await ouvrir();
    expect(screen.getByText('aucun')).toBeTruthy();
    expect(screen.getByText(/la liste des mods n’a pas pu être lue/)).toBeTruthy();
  });

  it('hors ligne : la liste vue la dernière fois est dite', async () => {
    const s = new Date(2026, 10, 18, 14, 5).getTime() / 1000;
    simu.reponse = liste([mod()], { etat: 'hors_ligne', vue_le: String(s) });
    await ouvrir();
    expect(screen.getByText(/vue le 18\/11 à 14:05/)).toBeTruthy();
  });

  it('pendant un téléchargement : la ligne visible, seule l’étape annoncée', async () => {
    simu.reponse = liste([mod()]);
    simu.suiviReponse = { ...INACTIF, actif: true, en_cours: '1080p', recus: 450, total: 1000, a_faire: 1 };
    await ouvrir();
    expect(screen.getByText('⬇ « 1080p » : 45 %')).toBeTruthy();
    const annonces = [...document.querySelectorAll('[aria-live="polite"]')].map((e) => e.textContent);
    expect(annonces).toContain('Téléchargement du mod « 1080p ».');
    expect(annonces.join(' ')).not.toContain('%');
  });

  it('la fin d’un téléchargement est dite : un toast et « Mods téléchargés » ; une seule minuterie', async () => {
    vi.useFakeTimers();
    simu.reponse = liste([mod()]);
    simu.suiviReponse = { ...INACTIF, actif: true, en_cours: '1080p', recus: 10, total: 1000, a_faire: 1 };
    render(BlocMods, { id: 7, titre: 'Zelda' });
    await vi.advanceTimersByTimeAsync(10);
    expect(screen.getByText('⬇ « 1080p » : 1 %')).toBeTruthy();
    suivre.mockClear();
    // Deux relectures pendant le téléchargement : une seule minuterie (jamais recréée à chaque relecture).
    simu.suiviReponse = { ...INACTIF, actif: true, en_cours: '1080p', recus: 500, total: 1000, a_faire: 1 };
    await vi.advanceTimersByTimeAsync(4000);
    expect(suivre).toHaveBeenCalledTimes(2);
    simu.suiviReponse = { ...INACTIF, faits: 1, a_faire: 1 };
    await vi.advanceTimersByTimeAsync(2000);
    expect(toast).toHaveBeenCalledWith('🧩 1 mod prêt : il s’allume d’un simple bouton.', 'ok');
    const annonces = [...document.querySelectorAll('[aria-live="polite"]')].map((e) => e.textContent);
    expect(annonces).toContain('Mods téléchargés.');
    // Fini : plus aucune relecture.
    suivre.mockClear();
    await vi.advanceTimersByTimeAsync(6000);
    expect(suivre).not.toHaveBeenCalled();
  });

  it('bande vide ou visible : dit au parent (« Rien à régler »)', async () => {
    const etats: string[] = [];
    render(BlocMods, {
      id: 7,
      titre: 'Zelda',
      get etatBloc() {
        return 'attente' as const;
      },
      set etatBloc(v: 'attente' | 'vide' | 'visible') {
        etats.push(v);
      },
    });
    await attendre();
    expect(etats.at(-1)).toBe('vide');
  });

  it('un autre jeu : la liste du jeu d’avant n’est jamais montrée', async () => {
    simu.reponse = liste([mod(), mod({ cle: 'b-000000000001', nom: 'B' })]);
    const r = render(BlocMods, { id: 7, titre: 'Zelda' });
    await attendre();
    expect(screen.getByRole('button', { name: '🧩 Ouvrir les mods (2)' })).toBeTruthy();
    simu.reponse = liste([mod()]);
    await r.rerender({ id: 8, titre: 'Mario' });
    await attendre();
    expect(lister).toHaveBeenLastCalledWith(8);
    expect(screen.getByRole('button', { name: '🧩 Ouvrir les mods (1)' })).toBeTruthy();
  });
});

describe('les musiques de Mario Kart 8 Deluxe dans la bande (contrat 1.55)', () => {
  it('la bande dit simplement où en sont les musiques, au bureau comme au canapé', async () => {
    const pose = { quand: '1', emulateur: 'Ryubing', raisons: [], musiques: { actives: 1, en_place: true, raison: null } };
    simu.reponse = liste([dk({ etat: 'telecharge', revision_rangee: 1, allume: true })], {
      musiques: { actives: 1, raison: null, taille_connue: 41018496, meme_ensemble: true, a_preparer: false },
      derniere_pose: pose,
    });
    await ouvrir();
    expect(screen.getByText('🎵 Les musiques de 1 mod sont en place (préparées par Firehouse).')).toBeTruthy();
    cleanup();
    document.documentElement.setAttribute('data-taodbox', '');
    simu.reponse = liste([dk({ etat: 'telecharge', revision_rangee: 1, allume: true })], {
      musiques: { actives: 1, raison: 'musiques des mods faites pour la version 3.0.1 ; ce jeu utilise une autre mise à jour : le jeu garde ses musiques d’origine', taille_connue: null },
      derniere_pose: pose,
    });
    await ouvrir();
    expect(screen.getByText('🎵 Musiques des mods faites pour la version 3.0.1 ; ce jeu utilise une autre mise à jour : le jeu garde ses musiques d’origine.')).toBeTruthy();
  });

  it('« 🎵 Préparer les musiques » : le bouton dit la taille ; « ⏳ Je regarde la taille… » pendant la lecture ; un oui va au cœur', async () => {
    simu.reponse = liste([dk({ etat: 'telecharge', revision_rangee: 1, allume: true })], {
      musiques: { actives: 1, raison: null, taille_connue: 41018496, meme_ensemble: false, a_preparer: true },
    });
    let finir: (n: number) => void = () => {};
    tailleMusiques.mockImplementation(() => new Promise<number>((r) => (finir = r)));
    await ouvrir();
    expect(screen.getByText('🎵 Musiques de 1 mod à préparer : elles ne se téléchargent jamais sans ton accord.')).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: '🎵 Préparer les musiques (≈ 41018496 o)' }));
    await attendre();
    expect(screen.getByText('⏳ Je regarde la taille des musiques…')).toBeTruthy();
    finir(41018496);
    await attendre();
    expect(screen.queryByText('⏳ Je regarde la taille des musiques…')).toBeNull();
    expect(confirmer.mock.calls[0][0]).toBe('🎵 Préparer les musiques des mods (41018496 o) ?');
    expect(preparerMusiques).toHaveBeenCalledWith(7, 41018496);
  });
});
