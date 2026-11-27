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
// Lot 7 (envois à Firehouse) : d'office, rien à proposer.
const RIEN = { etat: 'rien', motif: null, a_envoyer: [], total: 0, duree: null, plafonne: true, ecartes: [], deja_la: 0, suivi: { envois: [], en_cours: null, arret: null } };
let examen: object = RIEN;
let suiviReponse: object = RIEN.suivi;
const examiner = vi.fn(async (..._a: unknown[]) => structuredClone(examen));
const accepter = vi.fn(async (..._a: unknown[]) => 2);
const suivre = vi.fn(async (..._a: unknown[]) => structuredClone(suiviReponse));
const annuler = vi.fn(async (..._a: unknown[]) => 1);
// Contrat 1.47 : les noms français (d'office : rien de connu).
let nomsReponse: object = { par_cle: {}, par_id: {} };
const noms = vi.fn(async (..._a: unknown[]) => structuredClone(nomsReponse));
vi.mock('$lib/api', () => ({
  taille: (o: number) => `${o} o`,
  duree: (s: number) => `${s} s`,
  environDuree: (s: number) => `environ ${s} s`,
  estErreurCoeur: (e: unknown) => typeof e === 'object' && e !== null && 'sorte' in e,
  api: {
    contenusDuJeu: () => contenusDuJeu(),
    contenusSwitchChoisir: (...a: unknown[]) => choisir(...a),
    contenusInstaller: (...a: unknown[]) => installer(...a),
    envoisSwitchExaminer: (...a: unknown[]) => examiner(...a),
    envoisSwitchAccepter: (...a: unknown[]) => accepter(...a),
    envoisSwitchSuivi: (...a: unknown[]) => suivre(...a),
    envoisSwitchAnnuler: (...a: unknown[]) => annuler(...a),
    contenusSwitchNoms: (...a: unknown[]) => noms(...a),
    // Les mods Switch (BlocMods, inclus pour un jeu Switch) : un mod posé.
    modsSwitchLister: async () => ({
      etat: 'ok', vue_le: null, tid_base: '01007EF00011E000', version_servie: null, version_connue: true, remarque: null,
      mods: [{ cle: 'a-000000000001', nom: 'Clean UI', description: '', auteur: 'X', licence: 'MIT', source: '', revision: 1, revision_rangee: 1,
        etat: 'telecharge', compat: 'compatible', raison: null, allume: true, taille: 1, conflits: [], reglage_requis: '', touche_sauvegardes: false,
        contenu: { exefs: false, romfs: true, cheats: false }, emulateurs_essayes: ['Eden'], servi: true }],
    }),
    modsSwitchSuivi: async () => ({ actif: false, en_cours: null, recus: 0, total: 0, faits: 0, a_faire: 0, erreurs: [], arret: null }),
  },
}));
const confirmer = vi.fn(async (..._a: unknown[]) => true);
const toast = vi.fn();
const choisirDialogue = vi.fn(async (..._a: unknown[]): Promise<unknown> => null);
vi.mock('$lib/dialogues/fenetres.svelte', () => ({
  confirmer: (...a: unknown[]) => confirmer(...a),
  toast: (...a: unknown[]) => toast(...a),
  choisir: (...a: unknown[]) => choisirDialogue(...a),
}));
// Les réglages du profil (« envoyer tout seul »).
const profil = { envoisFirehouse: { auto: false } };
const reglerProfil = vi.fn(async (..._a: unknown[]) => {});
vi.mock('$lib/etat.svelte', () => ({
  etat: {
    get profil() {
      return profil;
    },
  },
  reglerProfil: (...a: unknown[]) => reglerProfil(...a),
}));
// Une partie ou un émulateur ouvert seul : la VRAIE règle de `lancementImpossible` (seul.ts), sur un état simulé.
const partie = { enJeu: null as number | null, lancement: null as number | null };
const seul = { ouvert: null as string | null, ouverture: null as number | null };
vi.mock('$lib/ludotheque/jeu.svelte', async () => {
  const s = await import('$lib/emulateurs/seul');
  return { lancementImpossible: () => s.lancementImpossibleSelon(partie, seul), attenteEmulateurSeul: () => s.raisonDAttente(seul) };
});

vi.mock('$lib/ludotheque/telechargements.svelte', () => ({ debitConnu: () => null }));

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
  nomsReponse = { par_cle: {}, par_id: {} };
  noms.mockReset();
  noms.mockImplementation(async () => structuredClone(nomsReponse));
  examen = RIEN;
  suiviReponse = RIEN.suivi;
  profil.envoisFirehouse.auto = false;
  examiner.mockClear();
  accepter.mockClear();
  suivre.mockClear();
  annuler.mockClear();
  accepter.mockImplementation(async () => 2);
  choisirDialogue.mockReset();
  choisirDialogue.mockImplementation(async () => null);
  reglerProfil.mockClear();
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
  it('panneau étroit : le nom AVANT l’interrupteur, le détail dessous sur toute la largeur', async () => {
    await ouvrir();
    const li = screen.getAllByRole('switch')[0].closest('li')!;
    expect(li.closest('ul')?.classList.contains('grille')).toBe(true);
    expect(li.firstElementChild?.classList.contains('nom')).toBe(true);
    expect(li.lastElementChild?.classList.contains('detail')).toBe(true);
  });

  it('un jeu Switch montre aussi ses mods ; un autre jeu jamais', async () => {
    render(BlocContenus, { id: 7, titre: 'Zelda', plateforme: 'Nintendo Switch' });
    // La bande ne montre qu'un résumé et « Ouvrir les mods (N) » (✅ Seb 26/11) : la liste est sur la page des mods.
    expect(await screen.findByRole('button', { name: '🧩 Ouvrir les mods (1)' })).toBeTruthy();
    expect(screen.getByText('1 allumé sur 1')).toBeTruthy();
    cleanup();
    render(BlocContenus, { id: 7, titre: 'Zelda', plateforme: 'Sony Playstation 3' });
    await screen.findByRole('button', { name: /Mises à jour et DLC/ });
    expect(screen.queryByRole('button', { name: /Ouvrir les mods/ })).toBeNull();
  });

  it('« vide » (Réglages du jeu, « Rien à régler ») : seulement une fois lu, et seulement sans rien à montrer', async () => {
    const vus: boolean[] = [];
    const props = (p: object) => ({
      id: 7,
      titre: 'Zelda',
      ...p,
      get vide() {
        return false;
      },
      set vide(v: boolean) {
        vus.push(v);
      },
    });
    reponse = { etat: 'aucun', titre_id: null, contenus: [] };
    render(BlocContenus, props({}));
    await vi.waitFor(() => expect(vus.at(-1)).toBe(true));
    cleanup();
    vus.length = 0;
    reponse = EDEN;
    render(BlocContenus, props({}));
    await screen.findByRole('button', { name: /Mises à jour et DLC/ });
    expect(vus.includes(true)).toBe(false);
  });

  it('déplié d’office (Taodbox) : les interrupteurs sans rien appuyer', async () => {
    render(BlocContenus, { id: 7, titre: 'Zelda', deplie: true });
    await screen.findByRole('button', { name: /Mises à jour et DLC/ });
    await tick();
    expect(screen.getAllByRole('switch')).toHaveLength(3);
  });

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

// Lot 7 (Seb 17/11) : ce que Firehouse n'a pas, l'accord chiffré, puis où en est l'envoi.
const PROPOSITION = {
  etat: 'ok',
  motif: null,
  a_envoyer: [
    { nom: 'Zelda maj [v786432].nsp', role: 'maj', taille: 3000 },
    { nom: 'DLC 2.nsz', role: 'dlc', taille: 1000 },
  ],
  total: 4000,
  duree: 120,
  plafonne: true,
  ecartes: [],
  deja_la: 1,
  suivi: { envois: [], en_cours: null, arret: null },
};

describe('l’envoi à Firehouse des contenus qu’il n’a pas', () => {
  it('dit ce que Firehouse n’a pas ; l’accord est chiffré (combien, poids, durée) avec « à l’avenir, tout seul »', async () => {
    examen = PROPOSITION;
    render(BlocContenus, { id: 7, titre: 'Zelda' });
    expect(await screen.findByText(/Firehouse n’a pas 2 contenus de ce jeu \(4000 o\)/)).toBeTruthy();
    expect(examiner).toHaveBeenCalledWith(7);
    choisirDialogue.mockImplementation(async () => 'non');
    await fireEvent.click(screen.getByRole('button', { name: 'Les envoyer…' }));
    await vi.waitFor(() => expect(choisirDialogue).toHaveBeenCalledTimes(1));
    const [titreAccord, options, message] = choisirDialogue.mock.calls[0] as [string, { valeur: string; libelle: string }[], string];
    expect(titreAccord).toBe('Firehouse n’a pas 2 contenus (4000 o, environ 120 s) : les lui envoyer ?');
    expect(options.map((o) => o.valeur)).toEqual(['oui', 'auto', 'non']);
    expect(options[1].libelle).toContain('à l’avenir envoyer tout seul');
    expect(message).toContain('Zelda maj [v786432].nsp');
    expect(message).toContain('la moitié de ta connexion');
    expect(message).toContain('ni le jeu, ni tes clés, ni tes parties');
    // « Pas maintenant » : rien ne part.
    expect(accepter).not.toHaveBeenCalled();
    // « Les envoyer » : la personne a dit oui, une fois.
    choisirDialogue.mockImplementation(async () => 'oui');
    await fireEvent.click(screen.getByRole('button', { name: 'Les envoyer…' }));
    await vi.waitFor(() => expect(accepter).toHaveBeenCalledWith(7, ['Zelda maj [v786432].nsp', 'DLC 2.nsz'], 4000, false));
    expect(reglerProfil).not.toHaveBeenCalled();
  });

  it('« à l’avenir, tout seul » est retenu pour le profil, puis plus rien n’est demandé', async () => {
    examen = PROPOSITION;
    render(BlocContenus, { id: 7, titre: 'Zelda' });
    choisirDialogue.mockImplementation(async () => 'auto');
    await fireEvent.click(await screen.findByRole('button', { name: 'Les envoyer…' }));
    await vi.waitFor(() => expect(accepter).toHaveBeenCalledWith(7, ['Zelda maj [v786432].nsp', 'DLC 2.nsz'], 4000, true));
    expect(reglerProfil).toHaveBeenCalledWith('envoisFirehouse.auto', true);
    cleanup();
    accepter.mockClear();
    choisirDialogue.mockClear();
    profil.envoisFirehouse.auto = true;
    render(BlocContenus, { id: 7, titre: 'Zelda' });
    await vi.waitFor(() => expect(accepter).toHaveBeenCalledWith(7, ['Zelda maj [v786432].nsp', 'DLC 2.nsz'], 4000, true));
    expect(choisirDialogue).not.toHaveBeenCalled();
  });

  it('pendant l’envoi : fichier, pourcentage, débit, temps restant ; en pause pendant une partie', async () => {
    const suiviEnvoi = {
      envois: [{ nom: 'DLC 2.nsz', role: 'dlc', taille: 1000, recu: 420, etat: 'envoi', motif: null }],
      en_cours: { jeu: 7, nom: 'DLC 2.nsz', etape: 'envoi', faits: 420, total: 1000, debit: 2048, restant: 30 },
      arret: null,
    };
    examen = { ...RIEN, etat: 'ok', suivi: suiviEnvoi };
    suiviReponse = suiviEnvoi;
    render(BlocContenus, { id: 7, titre: 'Zelda' });
    expect(await screen.findByText('📤 Envoi de « DLC 2.nsz » à Firehouse : 42 % · 2048 o/s · reste environ 30 s')).toBeTruthy();
    cleanup();
    const enPause = { ...suiviEnvoi, en_cours: { jeu: 7, nom: 'DLC 2.nsz', etape: 'pause', faits: 420, total: 1000 } };
    examen = { ...RIEN, etat: 'ok', suivi: enPause };
    suiviReponse = enPause;
    render(BlocContenus, { id: 7, titre: 'Zelda' });
    expect(await screen.findByText(/⏸ En pause pendant la partie/)).toBeTruthy();
  });

  it('après : « Firehouse l’a rangé », ou le refus avec le motif de Firehouse ; rien pour un compte sans droit', async () => {
    examen = {
      ...RIEN,
      etat: 'ok',
      suivi: {
        envois: [
          { nom: 'Zelda maj [v786432].nsp', role: 'maj', taille: 3000, recu: 3000, etat: 'range', motif: null },
          { nom: 'DLC 1.nsp', role: 'dlc', taille: 10, recu: 10, etat: 'refuse', motif: 'ticket personnel (lié à une console ou un compte) : jamais servi à la famille' },
        ],
        en_cours: null,
        arret: null,
      },
    };
    render(BlocContenus, { id: 7, titre: 'Zelda' });
    expect(await screen.findByText(/✅ Firehouse l’a rangé : « Zelda maj \[v786432\].nsp »/)).toBeTruthy();
    expect(screen.getByText('⚠ Refusé par Firehouse : « DLC 1.nsp » : ticket personnel (lié à une console ou un compte) : jamais servi à la famille')).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Les envoyer…' })).toBeNull();
    cleanup();
    examen = { ...PROPOSITION, etat: 'interdit', motif: 'Ton compte Firehouse ne permet pas d’envoyer.' };
    render(BlocContenus, { id: 7, titre: 'Zelda' });
    await vi.waitFor(() => expect(examiner).toHaveBeenCalled());
    await tick();
    expect(screen.queryByText(/Firehouse n’a pas/)).toBeNull();
    expect(screen.queryByRole('button', { name: 'Les envoyer…' })).toBeNull();
  });

  it('un double appui sur « Les envoyer… » n’ouvre qu’un accord', async () => {
    examen = PROPOSITION;
    let repondre: (v: unknown) => void = () => {};
    choisirDialogue.mockImplementation(() => new Promise((r) => (repondre = r)));
    render(BlocContenus, { id: 7, titre: 'Zelda' });
    const b = await screen.findByRole('button', { name: 'Les envoyer…' });
    await fireEvent.click(b);
    await fireEvent.click(b);
    await tick();
    expect(choisirDialogue).toHaveBeenCalledTimes(1);
    expect((b as HTMLButtonElement).disabled).toBe(true);
    repondre('oui');
    await vi.waitFor(() => expect(accepter).toHaveBeenCalledTimes(1));
  });

  it('un refus du cœur est dit et la proposition reste ; une liste changée est relue', async () => {
    examen = PROPOSITION;
    accepter.mockImplementation(async () => {
      throw { sorte: 'reseau', motif: 'Impossible de joindre Firehouse.' };
    });
    choisirDialogue.mockImplementation(async () => 'oui');
    render(BlocContenus, { id: 7, titre: 'Zelda' });
    await fireEvent.click(await screen.findByRole('button', { name: 'Les envoyer…' }));
    await vi.waitFor(() => expect(toast).toHaveBeenCalledWith(expect.stringContaining('Envoi impossible'), 'erreur'));
    expect(screen.getByRole('button', { name: 'Les envoyer…' })).toBeTruthy();
    // La liste a changé depuis qu'elle a été montrée : le cœur refuse (« conflit »), elle est relue.
    accepter.mockImplementation(async () => {
      throw { sorte: 'conflit', motif: 'La liste des contenus à envoyer a changé : regarde-la de nouveau avant de dire oui.' };
    });
    examiner.mockClear();
    await fireEvent.click(screen.getByRole('button', { name: 'Les envoyer…' }));
    await vi.waitFor(() => expect(examiner).toHaveBeenCalledTimes(1));
    expect(toast).toHaveBeenCalledWith(expect.stringContaining('a changé'), 'alerte');
  });

  it('les contenus écartés sont dits simplement', async () => {
    examen = { ...PROPOSITION, ecartes: [{ nom: 'perso.nsp', raison: 'ce fichier est lié à un compte ou une console' }] };
    render(BlocContenus, { id: 7, titre: 'Zelda' });
    expect(await screen.findByText('« perso.nsp » pas envoyé : ce fichier est lié à un compte ou une console')).toBeTruthy();
  });

  it('« ✋ Arrêter l’envoi » retire l’envoi en cours, puis la proposition revient', async () => {
    const enRoute = {
      envois: [{ nom: 'DLC 2.nsz', role: 'dlc', taille: 1000, recu: 420, etat: 'envoi', motif: null }],
      en_cours: { jeu: 7, nom: 'DLC 2.nsz', etape: 'envoi', faits: 420, total: 1000, debit: null, restant: null },
      arret: null,
    };
    examen = { ...RIEN, etat: 'ok', suivi: enRoute };
    suiviReponse = enRoute;
    render(BlocContenus, { id: 7, titre: 'Zelda' });
    const b = await screen.findByRole('button', { name: '✋ Arrêter l’envoi' });
    examen = PROPOSITION;
    await fireEvent.click(b);
    await vi.waitFor(() => expect(annuler).toHaveBeenCalledWith(7));
    expect(await screen.findByRole('button', { name: 'Les envoyer…' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: '✋ Arrêter l’envoi' })).toBeNull();
  });

  it('le lecteur d’écran n’entend que l’étape, pas le pourcentage', async () => {
    const enRoute = {
      envois: [],
      en_cours: { jeu: 7, nom: 'DLC 2.nsz', etape: 'envoi', faits: 420, total: 1000, debit: null, restant: null },
      arret: null,
    };
    examen = { ...RIEN, etat: 'ok', suivi: enRoute };
    suiviReponse = enRoute;
    const { container } = render(BlocContenus, { id: 7, titre: 'Zelda' });
    await screen.findByText(/42 %/);
    const vivants = [...container.querySelectorAll('[aria-live]')].map((n) => n.textContent ?? '');
    expect(vivants.some((x) => x.includes('%'))).toBe(false);
    expect(vivants).toContain('Envoi de « DLC 2.nsz » à Firehouse en cours.');
  });

  describe('la relecture toutes les 2 s', () => {
    afterEach(() => vi.useRealTimers());
    const avec = (suivi: object) => {
      examen = { ...RIEN, etat: 'ok', suivi };
      suiviReponse = suivi;
    };
    const enRoute = {
      envois: [{ nom: 'DLC 2.nsz', role: 'dlc', taille: 1000, recu: 420, etat: 'envoi', motif: null }],
      en_cours: { jeu: 7, nom: 'DLC 2.nsz', etape: 'envoi', faits: 420, total: 1000, debit: null, restant: null },
      arret: null,
    };

    it('tourne pendant un envoi et s’arrête au démontage', async () => {
      vi.useFakeTimers();
      avec(enRoute);
      const { unmount } = render(BlocContenus, { id: 7, titre: 'Zelda' });
      await vi.waitFor(() => expect(screen.getByText(/42 %/)).toBeTruthy());
      suivre.mockClear();
      await vi.advanceTimersByTimeAsync(2100);
      expect(suivre).toHaveBeenCalled();
      unmount();
      suivre.mockClear();
      await vi.advanceTimersByTimeAsync(10_000);
      expect(suivre).not.toHaveBeenCalled();
    });

    it('ne tourne pas quand tout est fini, ni quand le travail s’est arrêté', async () => {
      vi.useFakeTimers();
      avec({ envois: [{ nom: 'a.nsp', role: 'maj', taille: 1, recu: 1, etat: 'range', motif: null }], en_cours: null, arret: null });
      render(BlocContenus, { id: 7, titre: 'Zelda' });
      await vi.waitFor(() => expect(screen.getByText(/Firehouse l’a rangé/)).toBeTruthy());
      suivre.mockClear();
      await vi.advanceTimersByTimeAsync(10_000);
      expect(suivre).not.toHaveBeenCalled();
      cleanup();
      avec({ envois: [{ nom: 'b.nsp', role: 'dlc', taille: 1, recu: 0, etat: 'attente', motif: null }], en_cours: null, arret: 'Firehouse n’est pas joignable pour l’instant.' });
      render(BlocContenus, { id: 7, titre: 'Zelda' });
      await vi.waitFor(() => expect(screen.getByText(/reprendra à la prochaine ouverture/)).toBeTruthy());
      expect(screen.getByText(/⚠ Firehouse n’est pas joignable/)).toBeTruthy();
      suivre.mockClear();
      await vi.advanceTimersByTimeAsync(10_000);
      expect(suivre).not.toHaveBeenCalled();
    });
  });

  it('PS3 : rien n’est demandé à Firehouse pour l’envoi', async () => {
    reponse = PS3;
    render(BlocContenus, { id: 7, titre: 'Asura' });
    await screen.findByRole('button', { name: /Contenus additionnels/ });
    expect(examiner).not.toHaveBeenCalled();
  });
});

describe('les noms français des DLC et mises à jour Switch (contrat 1.47)', () => {
  it('le nom français remplace celui du fichier : ligne, interrupteur et fenêtre « Éteindre »', async () => {
    nomsReponse = {
      par_cle: { maj: 'Mise à jour 1.2.0', 'dlc:01007EF00011F001': 'Nouvelles tenues' },
      par_id: { 'E:\\Zelda vieille.nsp': 'Mise à jour (version 65536)' },
    };
    await ouvrir();
    await vi.waitFor(() => expect(screen.getByRole('switch', { name: 'Nouvelles tenues' })).toBeTruthy());
    expect(noms).toHaveBeenCalledWith(7);
    expect(screen.getByRole('switch', { name: 'Mise à jour 1.2.0' })).toBeTruthy();
    expect(screen.getByText('Mise à jour (version 65536)')).toBeTruthy();
    // Inconnu de Firehouse : le nom du fichier, comme avant.
    expect(screen.getByRole('switch', { name: 'DLC Pack 2' })).toBeTruthy();
    await fireEvent.click(screen.getByRole('switch', { name: 'Nouvelles tenues' }));
    await vi.waitFor(() => expect(confirmer).toHaveBeenCalled());
    expect(confirmer.mock.calls[0][0]).toBe('Éteindre « Nouvelles tenues » ?');
  });

  it('le panneau s’affiche d’abord avec les noms des fichiers ; sans réponse, ils restent', async () => {
    let finir: (v: object) => void = () => {};
    noms.mockImplementation(() => new Promise((r) => (finir = r)));
    await ouvrir();
    expect(screen.getByRole('switch', { name: 'DLC Pack 1' })).toBeTruthy();
    finir({ par_cle: { 'dlc:01007EF00011F001': 'Nouvelles tenues' }, par_id: {} });
    await vi.waitFor(() => expect(screen.getByRole('switch', { name: 'Nouvelles tenues' })).toBeTruthy());
    cleanup();
    noms.mockImplementation(async () => {
      throw { sorte: 'reseau', motif: 'injoignable' };
    });
    await ouvrir();
    await tick();
    expect(screen.getByRole('switch', { name: 'DLC Pack 1' })).toBeTruthy();
    expect(toast).not.toHaveBeenCalled();
  });

  it('PS3 : jamais demandés', async () => {
    reponse = PS3;
    render(BlocContenus, { id: 7, titre: 'Asura' });
    await screen.findByRole('button', { name: /Contenus additionnels/ });
    await tick();
    expect(noms).not.toHaveBeenCalled();
  });
});
