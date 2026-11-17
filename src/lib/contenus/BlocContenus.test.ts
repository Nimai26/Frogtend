// Le panneau 📦 d'un jeu Switch (✅ Seb 17/11) : un interrupteur par contenu, allumé d'office ; éteindre demande une
// confirmation maison ; rien ne se change pendant une partie ou un émulateur ouvert seul.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';

// La forme réelle d'`apercu_eden` (commandes.rs) : une mise à jour, une vieille, deux DLC dont un éteint.
const EDEN = {
  etat: 'ok',
  systeme: 'eden',
  titre_id: '01007EF00011E000',
  ignores: [],
  remarque: null,
  contenus: [
    { id: 'E:\\Zelda maj [v786432].nsp', nom: 'Zelda maj', genre: 'maj', taille: 1, licence: false, installe: true, raison: null, cle: 'maj', actif: true },
    { id: 'E:\\Zelda vieille.nsp', nom: 'Zelda vieille', genre: 'maj', taille: 1, licence: false, installe: false, raison: 'pas utilisée : une plus récente est là', cle: null, actif: false },
    { id: 'E:\\DLC_Pack_1.nsp', nom: 'DLC Pack 1', genre: 'dlc', taille: 1, licence: false, installe: true, raison: null, cle: 'dlc:01007EF00011F001', actif: true },
    { id: 'E:\\DLC_Pack_2.nsp', nom: 'DLC Pack 2', genre: 'dlc', taille: 1, licence: false, installe: true, raison: null, cle: 'dlc:01007EF00011F002', actif: false },
  ],
};
// La forme réelle d'`apercu_ryubing` : un DLC allumé que Ryubing ne prend pas (cause technique), un autre éteint.
const RYUBING = {
  ...EDEN,
  systeme: 'ryubing',
  contenus: [
    EDEN.contenus[0],
    { ...EDEN.contenus[2], installe: false, raison: 'Ryubing ne le prend pas pour l’instant' },
    { ...EDEN.contenus[3], installe: false },
  ],
};
// PS3 (`contenus_du_jeu`, branche RPCS3) : des cases, jamais d'interrupteur.
const PS3 = {
  etat: 'ok',
  systeme: 'ps3',
  titre_id: 'BLES01234',
  ignores: [],
  remarque: null,
  contenus: [
    { id: 'EP0001-BLES01234_00-DLC1', nom: 'Asura - Episode 1', genre: 'dlc', taille: 1048576, licence: true, installe: false, version: null, raison: null, cle: null, actif: true },
    { id: 'EP0001-BLES01234_00-DLC2', nom: 'Asura - Episode 2', genre: 'dlc', taille: 1048576, licence: false, installe: true, version: null, raison: null, cle: null, actif: true },
  ],
};
let reponse: object = EDEN;
const choisir = vi.fn(async (..._a: unknown[]) => true);
const installer = vi.fn(async (..._a: unknown[]) => ({ installes: 1, refuses: [] }));
const contenusDuJeu = vi.fn(async () => structuredClone(reponse));
vi.mock('$lib/api', () => ({
  taille: (o: number) => `${o} o`,
  api: {
    contenusDuJeu: () => contenusDuJeu(),
    contenusSwitchChoisir: (...a: unknown[]) => choisir(...a),
    contenusInstaller: (...a: unknown[]) => installer(...a),
  },
}));
const confirmer = vi.fn(async (..._a: unknown[]) => true);
const toast = vi.fn();
vi.mock('$lib/dialogues/fenetres.svelte', () => ({ confirmer: (...a: unknown[]) => confirmer(...a), toast: (...a: unknown[]) => toast(...a) }));
// Une partie ou un émulateur ouvert seul : la VRAIE règle de `lancementImpossible` (seul.ts), sur un état simulé.
const partie = { enJeu: null as number | null, lancement: null as number | null };
const seul = { ouvert: null as string | null, ouverture: null as number | null };
vi.mock('$lib/ludotheque/jeu.svelte', async () => {
  const s = await import('$lib/emulateurs/seul');
  return { lancementImpossible: () => s.lancementImpossibleSelon(partie, seul), attenteEmulateurSeul: () => s.raisonDAttente(seul) };
});

import BlocContenus from './BlocContenus.svelte';

async function ouvrir() {
  render(BlocContenus, { id: 7, titre: 'Zelda' });
  const entete = await screen.findByRole('button', { name: /Mises à jour et DLC/ });
  await fireEvent.click(entete);
  await tick();
  return entete;
}

beforeEach(() => {
  reponse = EDEN;
  installer.mockClear();
  contenusDuJeu.mockImplementation(async () => structuredClone(reponse));
  choisir.mockReset();
  choisir.mockImplementation(async () => true);
  confirmer.mockClear();
  confirmer.mockImplementation(async () => true);
  toast.mockClear();
  contenusDuJeu.mockClear();
  partie.enJeu = null;
  seul.ouvert = null;
});
afterEach(() => cleanup());

describe('le panneau des mises à jour et DLC Switch', () => {
  it('montre un interrupteur par contenu, le résumé et la remarque d’Eden', async () => {
    const entete = await ouvrir();
    // Eden garde le DLC éteint tant qu'un autre est allumé : le panneau le dit (Seb 17/11).
    expect(entete.textContent).toContain('3 utilisés (dont 1 éteint qu’Eden garde)');
    const inter = screen.getAllByRole('switch');
    expect(inter).toHaveLength(3);
    expect(inter.map((b) => b.getAttribute('aria-checked'))).toEqual(['true', 'true', 'false']);
    expect(inter[2].textContent).toContain('⚠ Éteint, mais Eden l’utilise');
    expect(inter[0].textContent).toContain('✅ Utilisé');
    expect(screen.getByText(/pour toute la maison/)).toBeTruthy();
    expect(screen.getByText(/Eden ne sait couper que tous les DLC ensemble/)).toBeTruthy();
    expect(screen.queryByText(/prochain lancement/)).toBeNull();
    // La vieille mise à jour : pas d'interrupteur, sa raison technique dite.
    expect(screen.getByText(/une plus récente est là/)).toBeTruthy();
  });

  it('éteindre demande une confirmation maison, puis enregistre et relit', async () => {
    await ouvrir();
    await fireEvent.click(screen.getAllByRole('switch')[1]);
    await vi.waitFor(() => expect(choisir).toHaveBeenCalled());
    expect(confirmer).toHaveBeenCalledTimes(1);
    expect(String((confirmer.mock.calls[0][1] as { message: string }).message)).toContain('à l’abri');
    expect(choisir).toHaveBeenCalledWith(7, 'dlc:01007EF00011F001', false);
    await vi.waitFor(() => expect(contenusDuJeu).toHaveBeenCalledTimes(2));
    expect(toast).not.toHaveBeenCalled();
  });

  it('refuser la confirmation ne change rien ; rallumer ne demande rien', async () => {
    confirmer.mockImplementation(async () => false);
    await ouvrir();
    await fireEvent.click(screen.getAllByRole('switch')[0]);
    await tick();
    expect(confirmer).toHaveBeenCalledTimes(1);
    expect(choisir).not.toHaveBeenCalled();
    await fireEvent.click(screen.getAllByRole('switch')[2]);
    await vi.waitFor(() => expect(choisir).toHaveBeenCalledWith(7, 'dlc:01007EF00011F002', true));
    expect(confirmer).toHaveBeenCalledTimes(1);
  });

  it('un échec est dit par un toast, avec le motif du cœur', async () => {
    choisir.mockImplementationOnce(async () => {
      throw { sorte: 'refus', motif: 'Une partie est en cours : tu pourras changer ces choix quand elle sera finie.' };
    });
    await ouvrir();
    await fireEvent.click(screen.getAllByRole('switch')[2]);
    await vi.waitFor(() => expect(toast).toHaveBeenCalled());
    expect(String(toast.mock.calls[0][0])).toContain('Impossible d');
    expect(toast.mock.calls[0][1]).toBe('erreur');
  });

  it('pendant une partie ou un émulateur ouvert seul : interrupteurs désactivés, motif visible', async () => {
    partie.enJeu = 12;
    await ouvrir();
    for (const b of screen.getAllByRole('switch')) expect((b as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText(/Une partie est en cours/)).toBeTruthy();
    cleanup();
    partie.enJeu = null;
    seul.ouvert = 'Ryubing';
    await ouvrir();
    expect(screen.getAllByRole('switch').every((b) => (b as HTMLButtonElement).disabled)).toBe(true);
    expect(screen.getByText(/Ryubing est ouvert/)).toBeTruthy();
    await fireEvent.click(screen.getAllByRole('switch')[0]);
    expect(confirmer).not.toHaveBeenCalled();
  });

  it('un double appui n’ouvre qu’une confirmation et n’enregistre qu’une fois', async () => {
    let repondre: (v: boolean) => void = () => {};
    confirmer.mockImplementation(() => new Promise<boolean>((r) => (repondre = r)));
    await ouvrir();
    const b = screen.getAllByRole('switch')[0];
    await fireEvent.click(b);
    await fireEvent.click(b);
    await tick();
    expect(confirmer).toHaveBeenCalledTimes(1);
    // Pendant ce temps : les AUTRES interrupteurs attendent, celui-ci garde le focus (aria-disabled).
    const tous = screen.getAllByRole('switch') as HTMLButtonElement[];
    expect([tous[0].disabled, tous[1].disabled, tous[2].disabled]).toEqual([false, true, true]);
    expect(tous[0].getAttribute('aria-disabled')).toBe('true');
    repondre(true);
    await vi.waitFor(() => expect(choisir).toHaveBeenCalledTimes(1));
    await fireEvent.click(b);
    await vi.waitFor(() => expect(contenusDuJeu).toHaveBeenCalledTimes(2));
    expect(choisir).toHaveBeenCalledTimes(1);
  });

  it('un échec d’enregistrement relit quand même la fiche ; une relecture ratée garde l’ancienne liste', async () => {
    choisir.mockImplementationOnce(async () => {
      throw { sorte: 'refus', motif: 'Ryubing est ouvert : ferme-le d’abord, puis change ces choix.' };
    });
    await ouvrir();
    await fireEvent.click(screen.getAllByRole('switch')[2]);
    await vi.waitFor(() => expect(contenusDuJeu).toHaveBeenCalledTimes(2));
    expect(toast).toHaveBeenCalledTimes(1);
    // La relecture échoue : la liste reste, un toast le dit.
    contenusDuJeu.mockImplementation(async () => {
      throw { sorte: 'disque', motif: 'Lecture impossible.' };
    });
    await fireEvent.click(screen.getAllByRole('switch')[2]);
    await vi.waitFor(() => expect(toast).toHaveBeenCalledTimes(2));
    expect(String(toast.mock.calls[1][0])).toContain('relire');
    expect(screen.getAllByRole('switch')).toHaveLength(3);
  });

  it('Ryubing : sans la remarque d’Eden ; un contenu allumé mais pas pris est dit tel quel', async () => {
    reponse = RYUBING;
    const entete = await ouvrir();
    expect(screen.queryByText(/Eden ne sait couper/)).toBeNull();
    const inter = screen.getAllByRole('switch');
    expect(inter[1].textContent).toContain('⚠ Allumé, pas utilisé');
    expect(inter[1].getAttribute('aria-checked')).toBe('true');
    expect(inter[1].getAttribute('aria-label')).toBe('DLC Pack 1');
    expect(screen.getByText(/Ryubing ne le prend pas pour l’instant/)).toBeTruthy();
    expect(entete.textContent).toContain('1 utilisé, 1 éteint, 1 allumé mais pas utilisé');
  });

  it('PS3 : des cases, jamais d’interrupteur ; rien ne s’installe sans le oui', async () => {
    reponse = PS3;
    render(BlocContenus, { id: 7, titre: 'Asura' });
    await fireEvent.click(await screen.findByRole('button', { name: /Contenus additionnels/ }));
    await tick();
    expect(screen.queryAllByRole('switch')).toHaveLength(0);
    expect(screen.getAllByRole('checkbox')).toHaveLength(1);
    expect(screen.getByRole('button', { name: 'Tout cocher' })).toBeTruthy();
    const installerBouton = screen.getByRole('button', { name: /📦 Installer/ }) as HTMLButtonElement;
    expect(installerBouton.disabled).toBe(true);
    await fireEvent.click(screen.getByRole('checkbox'));
    await tick();
    expect(installerBouton.disabled).toBe(false);
    confirmer.mockImplementationOnce(async () => false);
    await fireEvent.click(installerBouton);
    await tick();
    expect(installer).not.toHaveBeenCalled();
    await fireEvent.click(installerBouton);
    await vi.waitFor(() => expect(installer).toHaveBeenCalledWith(7, ['EP0001-BLES01234_00-DLC1']));
    expect(confirmer).toHaveBeenCalledTimes(2);
  });
});
