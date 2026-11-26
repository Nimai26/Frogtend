// 🧩 Le bloc des mods Switch (contrat 1.44, ✅ Seb 18/11) : accord chiffré avant tout téléchargement, un interrupteur
// par mod, conflits confirmés, mods grisés jamais allumables, rien ne change pendant une partie.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';

type Mod = Record<string, unknown>;
function mod(x: Mod = {}): Mod {
  return {
    cle: 'wollnashorn-000000000001',
    nom: '1080p',
    description: 'Rend le jeu en 1080p.',
    auteur: 'Wollnashorn',
    licence: 'MIT',
    source: 'https://example.invalid/commit',
    revision: 1,
    revision_rangee: null,
    etat: 'pas_telecharge',
    compat: 'compatible',
    raison: null,
    allume: false,
    taille: 1000,
    sha256: 'ab'.repeat(32),
    conflits: [],
    reglage_requis: '',
    touche_sauvegardes: false,
    contenu: { exefs: true, romfs: false, cheats: false },
    emulateurs_essayes: ['Ryubing'],
    servi: true,
    type: null,
    interet: null,
    ...x,
  };
}
const liste = (mods: Mod[], x: Mod = {}) => ({
  etat: 'ok',
  vue_le: null,
  tid_base: '01007EF00011E000',
  version_servie: null,
  version_connue: true,
  mods,
  remarque: null,
  derniere_pose: null,
  ...x,
});
const SHA = 'ab'.repeat(32);
const INACTIF = { actif: false, en_cours: null, recus: 0, total: 0, faits: 0, a_faire: 0, erreurs: [], arret: null };

let reponse: object = liste([]);
let suiviReponse: object = INACTIF;
const lister = vi.fn(async (..._a: unknown[]) => structuredClone(reponse));
const suivre = vi.fn(async (..._a: unknown[]) => structuredClone(suiviReponse));
const telecharger = vi.fn(async (..._a: unknown[]) => 1);
const choisirMod = vi.fn(async (..._a: unknown[]): Promise<boolean> => true);
const retirer = vi.fn(async (..._a: unknown[]) => {});
const tailleMusiques = vi.fn(async (..._a: unknown[]): Promise<number | null> => 41018496);
const preparerMusiques = vi.fn(async (..._a: unknown[]) => {});
vi.mock('$lib/api', () => ({
  taille: (o: number) => `${o} o`,
  duree: (s: number) => `${Math.round(s)} s`,
  estErreurCoeur: (e: unknown) => typeof e === 'object' && e !== null && 'sorte' in e,
  api: {
    modsSwitchLister: (...a: unknown[]) => lister(...a),
    modsSwitchSuivi: (...a: unknown[]) => suivre(...a),
    modsSwitchTelecharger: (...a: unknown[]) => telecharger(...a),
    modsSwitchChoisir: (...a: unknown[]) => choisirMod(...a),
    modsSwitchRetirer: (...a: unknown[]) => retirer(...a),
    modsSwitchMusiquesTaille: (...a: unknown[]) => tailleMusiques(...a),
    modsSwitchMusiquesPreparer: (...a: unknown[]) => preparerMusiques(...a),
  },
}));
const confirmer = vi.fn(async (..._a: unknown[]) => true);
const cocher = vi.fn(async (..._a: unknown[]): Promise<unknown> => null);
const choisirDialogue = vi.fn(async (..._a: unknown[]): Promise<unknown> => null);
const toast = vi.fn();
vi.mock('$lib/dialogues/fenetres.svelte', () => ({
  confirmer: (...a: unknown[]) => confirmer(...a),
  cocher: (...a: unknown[]) => cocher(...a),
  choisir: (...a: unknown[]) => choisirDialogue(...a),
  toast: (...a: unknown[]) => toast(...a),
}));
const profil = { modsSwitch: { auto: false } };
const reglerProfil = vi.fn(async (..._a: unknown[]) => {});
vi.mock('$lib/etat.svelte', () => ({
  etat: {
    get profil() {
      return profil;
    },
  },
  reglerProfil: (...a: unknown[]) => reglerProfil(...a),
}));
const partie = { enJeu: null as number | null, lancement: null as number | null };
const seul = { ouvert: null as string | null, ouverture: null as number | null };
vi.mock('$lib/ludotheque/jeu.svelte', async () => {
  const s = await import('$lib/emulateurs/seul');
  return { lancementImpossible: () => s.lancementImpossibleSelon(partie, seul), attenteEmulateurSeul: () => s.raisonDAttente(seul) };
});
let debit: number | null = null;
vi.mock('$lib/ludotheque/telechargements.svelte', () => ({ debitConnu: () => debit }));

import BlocMods from './BlocMods.svelte';
import { appliquer } from '$lib/taodbox/manette.svelte';

async function attendre() {
  for (let i = 0; i < 6; i++) {
    await tick();
    await Promise.resolve();
  }
}

async function ouvrir(props: { deplie?: boolean } = {}) {
  render(BlocMods, { id: 7, titre: 'Zelda', ...props });
  const entete = await screen.findByRole('button', { name: /Mods :/ });
  if (!props.deplie) await fireEvent.click(entete);
  await attendre();
  return entete;
}

beforeEach(() => {
  reponse = liste([]);
  suiviReponse = INACTIF;
  profil.modsSwitch.auto = false;
  debit = null;
  partie.enJeu = null;
  seul.ouvert = null;
  for (const f of [lister, suivre, telecharger, choisirMod, retirer, confirmer, cocher, choisirDialogue, toast, reglerProfil, tailleMusiques, preparerMusiques]) f.mockClear();
  tailleMusiques.mockImplementation(async () => 41018496);
  lister.mockImplementation(async () => structuredClone(reponse));
  suivre.mockImplementation(async () => structuredClone(suiviReponse));
  telecharger.mockImplementation(async () => 1);
  choisirMod.mockImplementation(async () => true);
  confirmer.mockImplementation(async () => true);
  cocher.mockImplementation(async () => null);
  choisirDialogue.mockImplementation(async () => null);
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});
// jsdom ne fait pas défiler : `selectionner` (focus rendu après une action) appelle scrollIntoView.
Element.prototype.scrollIntoView = () => {};

describe('le bloc des mods Switch', () => {
  it('ne se montre pas sans mod', async () => {
    render(BlocMods, { id: 7, titre: 'Zelda' });
    await attendre();
    expect(lister).toHaveBeenCalledWith(7);
    expect(screen.queryByRole('button', { name: /Mods :/ })).toBeNull();
  });

  it('montre chaque mod : auteur, licence, interrupteur des mods posés, mentions', async () => {
    reponse = liste([
      mod({ cle: 'a-000000000001', nom: 'Anisotropic Fix', etat: 'telecharge', revision_rangee: 1, allume: true, licence: '' }),
      mod({ cle: 'b-000000000001', nom: 'Clean UI', compat: 'pas_verifie', emulateurs_essayes: [] }),
    ]);
    const entete = await ouvrir();
    expect(entete.getAttribute('aria-expanded')).toBe('true');
    expect(entete.textContent).toContain('1 allumé sur 1, 1 à télécharger');
    const inter = screen.getAllByRole('switch');
    expect(inter).toHaveLength(1);
    expect(inter[0].getAttribute('aria-checked')).toBe('true');
    expect(inter[0].getAttribute('aria-label')).toBe('Anisotropic Fix');
    expect(document.getElementById(inter[0].getAttribute('aria-describedby')!)?.textContent).toBe('✅ Allumé');
    expect(screen.getByText('par Wollnashorn · licence : aucune')).toBeTruthy();
    expect(screen.getByText(/Version du jeu pas vérifiée/)).toBeTruthy();
    expect(screen.getByText('pas encore essayé dans un émulateur')).toBeTruthy();
    expect(document.body.textContent).not.toMatch(/exefs|romfs|build id/i);
  });

  it('un mod grisé : jamais d’interrupteur, un bouton focalisable relié à sa raison visible', async () => {
    reponse = liste([mod({ etat: 'incompatible', compat: 'incompatible', raison: 'fait pour la version 1.5.0 du jeu (build id 8E99).' })]);
    await ouvrir();
    expect(screen.queryAllByRole('switch')).toHaveLength(0);
    const b = screen.getByRole('button', { name: /Ne va pas avec ta version/ });
    expect(b.getAttribute('aria-disabled')).toBe('true');
    expect(b.hasAttribute('disabled')).toBe(false);
    expect(b.hasAttribute('title')).toBe(false);
    expect(document.getElementById(b.getAttribute('aria-describedby')!)?.textContent).toBe('Fait pour la version 1.5.0 du jeu (repère de version 8E99).');
    expect(screen.queryByRole('button', { name: /Télécharger/ })).toBeNull();
  });

  it('télécharger : un accord chiffré, puis la liste VUE donnée au cœur', async () => {
    debit = 100;
    reponse = liste([mod({ touche_sauvegardes: true })]);
    await ouvrir();
    await fireEvent.click(screen.getByRole('button', { name: '⬇ Télécharger (1000 o)' }));
    await attendre();
    expect(confirmer).toHaveBeenCalledTimes(1);
    const [titre, o] = confirmer.mock.calls[0] as [string, { message: string }];
    expect(titre).toBe('🧩 Télécharger « 1080p » (1000 o, environ 10 s) ?');
    expect(o.message).toContain('toute la maison');
    expect(o.message).toContain('à l’abri');
    expect(telecharger).toHaveBeenCalledWith(7, [{ cle: 'wollnashorn-000000000001', revision: 1, taille: 1000, sha256: SHA }], 1000, false);
  });

  it('un « non » ne télécharge rien', async () => {
    reponse = liste([mod()]);
    confirmer.mockImplementation(async () => false);
    await ouvrir();
    await fireEvent.click(screen.getByRole('button', { name: /Télécharger/ }));
    await attendre();
    expect(telecharger).not.toHaveBeenCalled();
  });

  it('tout télécharger : tout coché d’office, seuls les cochés partent', async () => {
    reponse = liste([mod({ cle: 'a-000000000001', nom: 'A', taille: 10 }), mod({ cle: 'b-000000000001', nom: 'B', taille: 5 })]);
    cocher.mockImplementation(async () => ['b-000000000001']);
    await ouvrir();
    await fireEvent.click(screen.getByRole('button', { name: /Tout télécharger \(2, 15 o\)/ }));
    await attendre();
    const o = cocher.mock.calls[0][2] as { coches: boolean[]; resume: (v: string[]) => string };
    expect(o.coches).toEqual([true, true]);
    expect(o.resume(['a-000000000001', 'b-000000000001'])).toBe('2 mods, 15 o');
    expect(telecharger).toHaveBeenCalledWith(7, [{ cle: 'b-000000000001', revision: 1, taille: 5, sha256: SHA }], 5, false);
  });

  it('la liste a changé : le refus du cœur est dit et la liste relue', async () => {
    reponse = liste([mod()]);
    telecharger.mockImplementation(async () => Promise.reject({ sorte: 'conflit', motif: 'La liste des mods a changé.' }));
    await ouvrir();
    lister.mockClear();
    await fireEvent.click(screen.getByRole('button', { name: /Télécharger/ }));
    await attendre();
    expect(toast).toHaveBeenCalledWith(expect.stringContaining('La liste des mods a changé'), 'alerte');
    expect(lister).toHaveBeenCalled();
  });

  it('allumer un mod en conflit : confirmé, puis l’autre est éteint d’un coup', async () => {
    reponse = liste([
      mod({ cle: 'a-000000000001', nom: 'A', etat: 'telecharge', revision_rangee: 1, conflits: ['b-000000000001'] }),
      mod({ cle: 'b-000000000001', nom: 'B', etat: 'telecharge', revision_rangee: 1, allume: true }),
    ]);
    await ouvrir();
    await fireEvent.click(screen.getByRole('switch', { name: 'A' }));
    await attendre();
    expect(confirmer.mock.calls[0][0]).toBe('Ne va pas avec « B » : l’allumer éteint l’autre');
    expect(choisirMod).toHaveBeenCalledWith(7, 'a-000000000001', true, 'b-000000000001');
  });

  it('un conflit refusé par le cœur que la liste ne montrait pas : relue, demandé, puis réessayé', async () => {
    reponse = liste([
      mod({ cle: 'a-000000000001', nom: 'A', etat: 'telecharge', revision_rangee: 1 }),
      mod({ cle: 'b-000000000001', nom: 'B', etat: 'telecharge', revision_rangee: 1, allume: true }),
    ]);
    await ouvrir();
    // Entre-temps, Firehouse a déclaré le conflit.
    reponse = liste([
      mod({ cle: 'a-000000000001', nom: 'A', etat: 'telecharge', revision_rangee: 1, conflits: ['b-000000000001'] }),
      mod({ cle: 'b-000000000001', nom: 'B', etat: 'telecharge', revision_rangee: 1, allume: true }),
    ]);
    choisirMod.mockImplementationOnce(async () => Promise.reject({ sorte: 'refus', motif: '« A » ne va pas avec « B »' }));
    await fireEvent.click(screen.getByRole('switch', { name: 'A' }));
    await attendre();
    expect(choisirMod).toHaveBeenNthCalledWith(1, 7, 'a-000000000001', true, null);
    expect(confirmer).toHaveBeenCalledTimes(1);
    expect(choisirMod).toHaveBeenNthCalledWith(2, 7, 'a-000000000001', true, 'b-000000000001');
  });

  it('éteindre un mod qui touche aux parties demande confirmation ; un double appui n’enregistre qu’une fois', async () => {
    reponse = liste([mod({ etat: 'telecharge', revision_rangee: 1, allume: true, touche_sauvegardes: true })]);
    let finir: (v: boolean) => void = () => {};
    confirmer.mockImplementation(() => new Promise<boolean>((r) => (finir = r)));
    await ouvrir();
    const inter = screen.getByRole('switch');
    await fireEvent.click(inter);
    await fireEvent.click(inter);
    expect(confirmer).toHaveBeenCalledTimes(1);
    finir(true);
    await attendre();
    expect(choisirMod).toHaveBeenCalledTimes(1);
    expect(choisirMod).toHaveBeenCalledWith(7, 'wollnashorn-000000000001', false, null);
  });

  it('pendant une partie : rien ne change, et c’est dit', async () => {
    partie.enJeu = 3;
    reponse = liste([mod({ etat: 'telecharge', revision_rangee: 1 }), mod({ cle: 'b-000000000001', nom: 'B' })]);
    await ouvrir();
    const inter = screen.getByRole('switch');
    expect(inter.getAttribute('aria-disabled')).toBe('true');
    await fireEvent.click(inter);
    await attendre();
    expect(choisirMod).not.toHaveBeenCalled();
    expect(screen.getByText(/Une partie est en cours/)).toBeTruthy();
    expect((screen.getByRole('button', { name: /Télécharger/ }) as HTMLButtonElement).disabled).toBe(true);
  });

  it('nouvelle révision : « à l’avenir, mettre à jour tout seul » est retenu dans le profil', async () => {
    reponse = liste([mod({ etat: 'revision_dispo', revision: 2, revision_rangee: 1, allume: true })]);
    choisirDialogue.mockImplementation(async () => 'auto');
    await ouvrir();
    await fireEvent.click(screen.getByRole('button', { name: '⬇ Mettre à jour (1000 o)' }));
    await attendre();
    expect(choisirDialogue.mock.calls[0][0]).toBe('🧩 Mettre à jour « 1080p » (1000 o) ?');
    expect(reglerProfil).toHaveBeenCalledWith('modsSwitch.auto', true);
    expect(telecharger).toHaveBeenCalledWith(7, [{ cle: 'wollnashorn-000000000001', revision: 2, taille: 1000, sha256: SHA }], 1000, true);
  });

  it('mise à jour automatique choisie : les révisions partent seules à l’ouverture (dit par un toast)', async () => {
    profil.modsSwitch.auto = true;
    reponse = liste([mod({ etat: 'revision_dispo', revision: 2, revision_rangee: 1 }), mod({ cle: 'b-000000000001', nom: 'B' })]);
    render(BlocMods, { id: 7, titre: 'Zelda' });
    await attendre();
    expect(telecharger).toHaveBeenCalledTimes(1);
    expect(telecharger).toHaveBeenCalledWith(7, [{ cle: 'wollnashorn-000000000001', revision: 2, taille: 1000, sha256: SHA }], 1000, true);
    expect(toast).toHaveBeenCalledWith(expect.stringContaining('1 mod de « Zelda » se met à jour tout seul'));
    expect(choisirDialogue).not.toHaveBeenCalled();
  });

  it('mise à jour automatique : jamais pendant une partie', async () => {
    profil.modsSwitch.auto = true;
    partie.enJeu = 3;
    reponse = liste([mod({ etat: 'revision_dispo', revision: 2, revision_rangee: 1 })]);
    render(BlocMods, { id: 7, titre: 'Zelda' });
    await attendre();
    expect(telecharger).not.toHaveBeenCalled();
  });

  it('pendant un téléchargement : le pourcentage visible, seule l’étape annoncée', async () => {
    reponse = liste([mod()]);
    suiviReponse = { ...INACTIF, actif: true, en_cours: '1080p', recus: 450, total: 1000, a_faire: 1 };
    await ouvrir();
    expect(screen.getByText('⬇ « 1080p » : 45 %')).toBeTruthy();
    expect(screen.getByText('⬇ 45 %')).toBeTruthy();
    const annonces = [...document.querySelectorAll('[aria-live="polite"]')].map((e) => e.textContent);
    expect(annonces).toContain('Téléchargement du mod « 1080p ».');
    expect(annonces.join(' ')).not.toContain('%');
  });

  it('un mod que Firehouse ne sert plus : dit, et « Retirer » après confirmation', async () => {
    reponse = liste([mod({ etat: 'telecharge', revision_rangee: 1, servi: false, description: '', auteur: '', licence: '' })]);
    await ouvrir();
    expect(screen.getByText(/Firehouse ne le propose plus/)).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: /Retirer/ }));
    await attendre();
    expect((confirmer.mock.calls[0][1] as { danger: boolean }).danger).toBe(true);
    expect(retirer).toHaveBeenCalledWith(7, 'wollnashorn-000000000001');
  });

  it('hors ligne : la liste vue la dernière fois, rien à télécharger', async () => {
    const s = new Date(2026, 10, 18, 14, 5).getTime() / 1000;
    reponse = liste([mod()], { etat: 'hors_ligne', vue_le: String(s) });
    await ouvrir();
    expect(screen.getByText(/vue le 18\/11 à 14:05/)).toBeTruthy();
    expect((screen.getByRole('button', { name: /Télécharger/ }) as HTMLButtonElement).disabled).toBe(true);
  });

  it('la fiche de Firehouse se lit : description, « ⭐ Recommandé », la sorte en mots simples', async () => {
    reponse = liste([
      mod({ cle: 'a-000000000001', nom: 'A', description: 'Le jeu s’affiche en 1080p.', interet: 5, type: 'technique' }),
      mod({ cle: 'b-000000000001', nom: 'B', description: 'Un nouveau costume.', interet: 2, type: 'cosmetique' }),
      mod({ cle: 'c-000000000001', nom: 'C', description: '', interet: null, type: null }),
    ]);
    await ouvrir();
    expect(screen.getByText('Le jeu s’affiche en 1080p.')).toBeTruthy();
    expect(screen.getByText('⭐ Recommandé · amélioration technique')).toBeTruthy();
    expect(screen.getByText('apparence')).toBeTruthy();
    expect(screen.queryAllByText(/Recommandé/).length).toBe(1);
    expect(document.body.textContent).not.toContain('cosmetique');
  });

  it('une description longue se replie, « Lire plus » la déplie', async () => {
    reponse = liste([mod({ description: 'x'.repeat(200) })]);
    await ouvrir();
    const lire = screen.getByRole('button', { name: 'Lire plus' });
    expect(lire.getAttribute('aria-expanded')).toBe('false');
    expect(document.getElementById(lire.getAttribute('aria-controls')!)?.classList.contains('replie')).toBe(true);
    await fireEvent.click(lire);
    expect(screen.getByRole('button', { name: 'Lire moins' }).getAttribute('aria-expanded')).toBe('true');
  });

  it('à la manette : A bascule l’interrupteur choisi ; le bouton d’un mod grisé est atteignable mais ne fait rien', async () => {
    reponse = liste([
      mod({ cle: 'a-000000000001', nom: 'A', etat: 'telecharge', revision_rangee: 1 }),
      mod({ cle: 'b-000000000001', nom: 'B', etat: 'incompatible', compat: 'incompatible', raison: 'Autre version.' }),
    ]);
    await ouvrir({ deplie: true });
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

  it('l’accord envoie au cœur l’empreinte de ce qui a été vu (sha256)', async () => {
    reponse = liste([mod({ sha256: 'cd'.repeat(32) })]);
    await ouvrir();
    await fireEvent.click(screen.getByRole('button', { name: /Télécharger/ }));
    await attendre();
    const vus = telecharger.mock.calls[0][1] as { sha256: string }[];
    expect(vus[0].sha256).toBe('cd'.repeat(32));
  });

  it('tout télécharger : rien de coché, la fenêtre le dit', async () => {
    reponse = liste([mod({ cle: 'a-000000000001', nom: 'A' }), mod({ cle: 'b-000000000001', nom: 'B' })]);
    await ouvrir();
    await fireEvent.click(screen.getByRole('button', { name: /Tout télécharger/ }));
    await attendre();
    const o = cocher.mock.calls[0][2] as { refus: (v: string[]) => string | null };
    expect(o.refus([])).toBe('Coche au moins un mod.');
    expect(o.refus(['a-000000000001'])).toBeNull();
  });

  it('après « Télécharger », le focus va à une commande de la ligne que la manette sait quitter (jamais en haut)', async () => {
    reponse = liste([mod({ cle: 'a-000000000001', nom: 'A' }), mod({ cle: 'b-000000000001', nom: 'B' })]);
    const entete = await ouvrir();
    const bouton = screen.getAllByRole('button', { name: /Télécharger/ }).find((b) => b.closest('li')?.dataset.cle === 'a-000000000001')!;
    bouton.focus();
    // Le mod arrive tout de suite (très léger) : sa ligne montre alors son interrupteur, le bouton a disparu.
    telecharger.mockImplementation(async () => {
      reponse = liste([mod({ cle: 'a-000000000001', nom: 'A', etat: 'telecharge', revision_rangee: 1, allume: true }), mod({ cle: 'b-000000000001', nom: 'B' })]);
      suiviReponse = { ...INACTIF, faits: 1, a_faire: 1 };
      return 1;
    });
    await fireEvent.click(bouton);
    await attendre();
    const ici = document.activeElement as HTMLElement;
    expect(ici.getAttribute('role')).toBe('switch');
    expect(ici.closest('li')?.dataset.cle).toBe('a-000000000001');
    expect(ici.getAttribute('tabindex')).not.toBe('-1');
    // La VRAIE manette (géométrie : chaque commande sous la précédente) : « bas » descend depuis là, sans repartir
    // du haut de la page.
    const commandes = [...document.querySelectorAll<HTMLElement>('button:not([disabled]), [role="switch"]')];
    commandes.forEach(
      (b, i) => (b.getBoundingClientRect = () => ({ left: 0, top: i * 50, width: 100, height: 40, right: 100, bottom: i * 50 + 40, x: 0, y: i * 50, toJSON() {} }) as DOMRect),
    );
    appliquer('bas');
    const apres = document.activeElement as HTMLElement;
    expect(apres).not.toBe(entete);
    expect(commandes.indexOf(apres)).toBeGreaterThan(commandes.indexOf(ici));
  });

  it('un mod qui a fini avant la première lecture du suivi : la fin est dite quand même', async () => {
    reponse = liste([mod()]);
    await ouvrir();
    telecharger.mockImplementation(async () => {
      suiviReponse = { ...INACTIF, faits: 1, a_faire: 1 };
      return 1;
    });
    lister.mockClear();
    await fireEvent.click(screen.getByRole('button', { name: /Télécharger/ }));
    await attendre();
    expect(toast).toHaveBeenCalledWith(expect.stringContaining('1 mod prêt'), 'ok');
    expect(screen.getByText('Mods téléchargés.')).toBeTruthy();
    expect(lister).toHaveBeenCalled();
  });

  it('les refus du cœur sont dits sans jargon', async () => {
    reponse = liste([mod()]);
    await ouvrir();
    telecharger.mockImplementation(async () => {
      throw { sorte: 'refus', motif: 'archive zip sans exefs ni cheats' };
    });
    await fireEvent.click(screen.getByRole('button', { name: /Télécharger/ }));
    await attendre();
    const t = String(toast.mock.calls.at(-1)?.[0]);
    expect(t).toContain('archive archive sans programme du jeu ni triches');
    expect(t).not.toMatch(/zip|exefs|cheats/);
  });

  it('la dernière partie : ce qui n’a pas été posé est dit, et l’interrupteur du mod le montre', async () => {
    const s = new Date(2026, 10, 18, 14, 5).getTime() / 1000;
    reponse = liste([mod({ etat: 'telecharge', revision_rangee: 1, allume: true })], {
      derniere_pose: { quand: String(s), emulateur: 'Eden', raisons: ['« 1080p » pas posé : réglage d’émulateur requis : résolution ×2'] },
      remarque: 'Ryubing n’a pas été installé par Frogtend.',
    });
    await ouvrir();
    expect(screen.getByText('⚠ À la dernière partie (Eden, le 18/11 à 14:05) :')).toBeTruthy();
    expect(screen.getByText('• « 1080p » pas posé : réglage d’émulateur requis : résolution ×2')).toBeTruthy();
    expect(screen.getByText('Ryubing n’a pas été installé par Frogtend.')).toBeTruthy();
    expect(screen.getByRole('switch').textContent).toContain('⚠ Allumé, pas utilisé');
  });

  it('Firehouse injoignable et rien de lu avant : le bloc le dit (jamais muet)', async () => {
    reponse = liste([], { etat: 'injoignable' });
    render(BlocMods, { id: 7, titre: 'Zelda' });
    expect(await screen.findByRole('button', { name: /Mods : aucun/ })).toBeTruthy();
    expect(screen.getByText(/la liste des mods n’a pas pu être lue/)).toBeTruthy();
  });

  it('la fin d’un téléchargement est dite : un toast et « Mods téléchargés » pour le lecteur d’écran', async () => {
    vi.useFakeTimers();
    reponse = liste([mod()]);
    suiviReponse = { ...INACTIF, actif: true, en_cours: '1080p', recus: 10, total: 1000, a_faire: 1 };
    render(BlocMods, { id: 7, titre: 'Zelda' });
    await vi.advanceTimersByTimeAsync(10);
    expect(screen.getByText('⬇ « 1080p » : 1 %')).toBeTruthy();
    suivre.mockClear();
    // Deux relectures pendant le téléchargement : une seule minuterie (jamais recréée à chaque relecture).
    suiviReponse = { ...INACTIF, actif: true, en_cours: '1080p', recus: 500, total: 1000, a_faire: 1 };
    await vi.advanceTimersByTimeAsync(4000);
    expect(suivre).toHaveBeenCalledTimes(2);
    suiviReponse = { ...INACTIF, faits: 1, a_faire: 1 };
    await vi.advanceTimersByTimeAsync(2000);
    expect(toast).toHaveBeenCalledWith('🧩 1 mod prêt : il s’allume d’un simple bouton.', 'ok');
    const annonces = [...document.querySelectorAll('[aria-live="polite"]')].map((e) => e.textContent);
    expect(annonces).toContain('Mods téléchargés.');
    // Fini : plus aucune relecture.
    suivre.mockClear();
    await vi.advanceTimersByTimeAsync(6000);
    expect(suivre).not.toHaveBeenCalled();
  });

  it('bloc vide ou visible : dit au parent (« Rien à régler »)', async () => {
    const etats: string[] = [];
    reponse = liste([]);
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

  it('au canapé (déplié d’office) : la liste est ouverte sans rien appuyer', async () => {
    reponse = liste([mod({ etat: 'telecharge', revision_rangee: 1 })]);
    await ouvrir({ deplie: true });
    expect(screen.getByRole('switch')).toBeTruthy();
  });
});

describe('les musiques de Mario Kart 8 Deluxe (contrat 1.55)', () => {
  const DK = 'gamebanana-ee3c0683c3f6';
  const REMIX = 'gamebanana-0123456789ab';
  const dk = (x: Mod = {}) => mod({ cle: DK, nom: 'DK Summit', amorces: ['pBGM_GC_SHERBET_LAND_F', 'pBGM_GC_SHERBET_LAND_N'], ...x });

  it('télécharger un mod de musique : l’accord annonce aussi « + 41 Mo de musiques », la taille va au cœur', async () => {
    reponse = liste([dk()]);
    await ouvrir();
    await fireEvent.click(screen.getByRole('button', { name: '⬇ Télécharger (1000 o)' }));
    await attendre();
    expect(tailleMusiques).toHaveBeenCalledWith(7, [DK], null);
    const [titre, o] = confirmer.mock.calls[0] as [string, { message: string }];
    expect(titre).toBe('🧩 Télécharger « DK Summit » (1000 o + 41018496 o de musiques) ?');
    expect(o.message).toContain('🎵 + 41018496 o de musiques');
    expect(telecharger).toHaveBeenCalledWith(7, [{ cle: DK, revision: 1, taille: 1000, sha256: SHA }], 1000, false, 41018496);
  });

  it('un mod de musique qui arrivera éteint (conflit avec un mod allumé) n’annonce pas de musiques', async () => {
    reponse = liste([dk(), mod({ cle: REMIX, nom: 'Sherbet remix', etat: 'telecharge', revision_rangee: 1, allume: true, amorces: ['pBGM_GC_SHERBET_LAND_N'] })]);
    await ouvrir();
    await fireEvent.click(screen.getByRole('button', { name: '⬇ Télécharger (1000 o)' }));
    await attendre();
    expect(tailleMusiques).not.toHaveBeenCalled();
    expect(confirmer.mock.calls[0][0]).toBe('🧩 Télécharger « DK Summit » (1000 o) ?');
    expect(telecharger).toHaveBeenCalledWith(7, [{ cle: DK, revision: 1, taille: 1000, sha256: SHA }], 1000, false);
  });

  it('Firehouse muet : la taille du dernier fichier connu est annoncée ; hors ligne, sans rien demander', async () => {
    tailleMusiques.mockImplementation(async () => Promise.reject({ sorte: 'reseau', motif: 'x' }));
    reponse = liste([dk({ etat: 'telecharge', revision_rangee: 1 })], { musiques: { actives: 0, raison: null, taille_connue: 40000000 } });
    await ouvrir();
    await fireEvent.click(screen.getByRole('switch', { name: 'DK Summit' }));
    await attendre();
    expect(confirmer.mock.calls[0][0]).toContain('+ 40000000 o de musiques');
    cleanup();
    tailleMusiques.mockClear();
    confirmer.mockClear();
    reponse = liste([dk({ etat: 'telecharge', revision_rangee: 1 })], { etat: 'hors_ligne', musiques: { actives: 0, raison: null, taille_connue: 40000000 } });
    await ouvrir();
    await fireEvent.click(screen.getByRole('switch', { name: 'DK Summit' }));
    await attendre();
    expect(tailleMusiques).not.toHaveBeenCalled();
    expect(confirmer.mock.calls[0][0]).toContain('+ 40000000 o de musiques');
  });

  it('pendant que la taille est demandée : « ⏳ Je regarde la taille des musiques… » est dit', async () => {
    let finir: (n: number) => void = () => {};
    tailleMusiques.mockImplementation(() => new Promise<number>((r) => (finir = r)));
    reponse = liste([dk({ etat: 'telecharge', revision_rangee: 1 })]);
    await ouvrir();
    await fireEvent.click(screen.getByRole('switch', { name: 'DK Summit' }));
    await attendre();
    expect(screen.getByText('⏳ Je regarde la taille des musiques…')).toBeTruthy();
    finir(41018496);
    await attendre();
    expect(screen.queryByText('⏳ Je regarde la taille des musiques…')).toBeNull();
  });

  it('allumer un mod de musique : l’accord dit la taille ; un « non » ne change rien ; un oui donne la taille au cœur', async () => {
    reponse = liste([dk({ etat: 'telecharge', revision_rangee: 1 })]);
    await ouvrir();
    confirmer.mockImplementation(async () => false);
    await fireEvent.click(screen.getByRole('switch', { name: 'DK Summit' }));
    await attendre();
    expect(confirmer.mock.calls[0][0]).toBe('🎵 Allumer « DK Summit » (+ 41018496 o de musiques) ?');
    expect(choisirMod).not.toHaveBeenCalled();
    confirmer.mockImplementation(async () => true);
    await fireEvent.click(screen.getByRole('switch', { name: 'DK Summit' }));
    await attendre();
    expect(choisirMod).toHaveBeenCalledWith(7, DK, true, null, 41018496);
  });

  it('sans taille connue : allumé, mais rien n’est accordé ni préchargé (c’est dit)', async () => {
    tailleMusiques.mockImplementation(async () => null);
    reponse = liste([dk({ etat: 'telecharge', revision_rangee: 1 })]);
    await ouvrir();
    await fireEvent.click(screen.getByRole('switch', { name: 'DK Summit' }));
    await attendre();
    const [titre, o] = confirmer.mock.calls[0] as [string, { message: string }];
    expect(titre).toContain('+ des musiques, taille pas encore connue');
    expect(o.message).toContain('les musiques seront demandées à la prochaine partie');
    expect(choisirMod).toHaveBeenCalledWith(7, DK, true, null);
  });

  it('deux mods de musique qui changent la même musique : UNE fenêtre (éteindre l’autre + la taille)', async () => {
    reponse = liste([
      dk({ etat: 'telecharge', revision_rangee: 1 }),
      mod({ cle: REMIX, nom: 'Sherbet remix', etat: 'telecharge', revision_rangee: 1, allume: true, amorces: ['pBGM_GC_SHERBET_LAND_N'] }),
    ]);
    await ouvrir();
    await fireEvent.click(screen.getByRole('switch', { name: 'DK Summit' }));
    await attendre();
    expect(confirmer).toHaveBeenCalledTimes(1);
    const [titre, o] = confirmer.mock.calls[0] as [string, { message: string }];
    expect(titre).toBe('🎵 Allumer « DK Summit » (+ 41018496 o de musiques) ?');
    expect(o.message).toContain('« DK Summit » et « Sherbet remix » changent la même musique du jeu');
    // La taille est demandée SANS le mod qui s'éteint.
    expect(tailleMusiques).toHaveBeenCalledWith(7, [DK], REMIX);
    expect(choisirMod).toHaveBeenCalledWith(7, DK, true, REMIX, 41018496);
    // Renoncer : rien ne change.
    choisirMod.mockClear();
    confirmer.mockImplementation(async () => false);
    await fireEvent.click(screen.getByRole('switch', { name: 'DK Summit' }));
    await attendre();
    expect(choisirMod).not.toHaveBeenCalled();
  });

  it('la fiche dit simplement où en sont les musiques, au bureau comme au canapé', async () => {
    const pose = { quand: '1', emulateur: 'Ryubing', raisons: [], musiques: { actives: 1, en_place: true, raison: null } };
    reponse = liste([dk({ etat: 'telecharge', revision_rangee: 1, allume: true })], {
      musiques: { actives: 1, raison: null, taille_connue: 41018496, meme_ensemble: true, a_preparer: false },
      derniere_pose: pose,
    });
    await ouvrir();
    expect(screen.getByText('🎵 Les musiques de 1 mod sont en place (préparées par Firehouse).')).toBeTruthy();
    cleanup();
    reponse = liste([dk({ etat: 'telecharge', revision_rangee: 1, allume: true })], {
      musiques: { actives: 1, raison: 'musiques des mods faites pour la version 3.0.1 ; ce jeu utilise une autre mise à jour : le jeu garde ses musiques d’origine', taille_connue: null },
      derniere_pose: pose,
    });
    await ouvrir({ deplie: true });
    expect(screen.getByText('🎵 Musiques des mods faites pour la version 3.0.1 ; ce jeu utilise une autre mise à jour : le jeu garde ses musiques d’origine.')).toBeTruthy();
  });

  it('« 🎵 Préparer les musiques » : la taille, l’accord, puis le cœur ; un « non » ne prépare rien', async () => {
    reponse = liste([dk({ etat: 'telecharge', revision_rangee: 1, allume: true })], {
      musiques: { actives: 1, raison: null, taille_connue: 41018496, meme_ensemble: false, a_preparer: true },
    });
    await ouvrir({ deplie: true });
    expect(screen.getByText('🎵 Musiques de 1 mod à préparer : elles ne se téléchargent jamais sans ton accord.')).toBeTruthy();
    confirmer.mockImplementation(async () => false);
    await fireEvent.click(screen.getByRole('button', { name: '🎵 Préparer les musiques (≈ 41018496 o)' }));
    await attendre();
    expect(confirmer.mock.calls[0][0]).toBe('🎵 Préparer les musiques des mods (41018496 o) ?');
    expect(preparerMusiques).not.toHaveBeenCalled();
    confirmer.mockImplementation(async () => true);
    await fireEvent.click(screen.getByRole('button', { name: /Préparer les musiques/ }));
    await attendre();
    expect(preparerMusiques).toHaveBeenCalledWith(7, 41018496);
    // Taille inconnue (Firehouse muet, rien de connu) : rien n'est préparé, c'est dit.
    preparerMusiques.mockClear();
    confirmer.mockClear();
    tailleMusiques.mockImplementation(async () => null);
    reponse = liste([dk({ etat: 'telecharge', revision_rangee: 1, allume: true })], {
      musiques: { actives: 1, raison: null, taille_connue: null, meme_ensemble: false, a_preparer: true },
    });
    cleanup();
    await ouvrir({ deplie: true });
    await fireEvent.click(screen.getByRole('button', { name: '🎵 Préparer les musiques' }));
    await attendre();
    expect(confirmer).not.toHaveBeenCalled();
    expect(preparerMusiques).not.toHaveBeenCalled();
    expect(toast).toHaveBeenCalledWith(expect.stringContaining('pas encore connue'), 'alerte');
  });
});
