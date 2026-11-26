// 🧩 La logique partagée des mods Switch (`ModsSwitchJeu`, contrat 1.44, ✅ Seb 18/11) : accord chiffré avant tout
// téléchargement, conflits confirmés, rien ne change pendant une partie, mise à jour automatique d'un seul endroit,
// remise à zéro au changement de jeu ou de profil ; les musiques de MK8D (contrat 1.55) jamais sans un oui chiffré.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  DK,
  REMIX,
  SHA,
  attendre,
  choisirDialogue,
  choisirMod,
  cocher,
  confirmer,
  dk,
  liste,
  lister,
  mod,
  preparerMusiques,
  reglerProfil,
  remettre,
  retirer,
  simu,
  suivre,
  tailleMusiques,
  telecharger,
  toast,
  INACTIF,
} from './essaisMods';

vi.mock('$lib/api', async () => (await import('./essaisMods')).apiSimulee());
vi.mock('$lib/dialogues/fenetres.svelte', async () => (await import('./essaisMods')).fenetresSimulees());
vi.mock('$lib/etat.svelte', async () => (await import('./essaisMods')).etatSimule());
vi.mock('$lib/ludotheque/jeu.svelte', async () => (await import('./essaisMods')).jeuSimule());
vi.mock('$lib/ludotheque/telechargements.svelte', async () => (await import('./essaisMods')).telechargementsSimules());

import type { ModSwitchVu } from '$lib/api';
import { ModsSwitchJeu } from './modsSwitch.svelte';
import { nombreDeModsSwitch, nombreDeModsVu, optionModsSwitch } from './pageMods';

/** Une instance ouverte sur le jeu 7 (« Zelda ») pour le profil de Seb, la liste lue. */
async function ouvrir(): Promise<ModsSwitchJeu> {
  const ms = new ModsSwitchJeu();
  ms.ouvrir(7, 'Zelda', simu.profilOuvert?.id ?? null);
  await attendre();
  return ms;
}
const leMod = (ms: ModsSwitchJeu, cle = 'wollnashorn-000000000001') => ms.mods.find((m) => m.cle === cle) as ModSwitchVu;

beforeEach(() => remettre());
afterEach(() => vi.useRealTimers());
// jsdom ne fait pas défiler : `selectionner` (focus rendu après une action) appelle scrollIntoView.
Element.prototype.scrollIntoView = () => {};

describe('la logique des mods Switch', () => {
  it('ouvrir : la liste puis le suivi sont lus, sans rien télécharger', async () => {
    simu.reponse = liste([mod()]);
    const ms = await ouvrir();
    expect(lister).toHaveBeenCalledWith(7);
    expect(suivre).toHaveBeenCalledWith(7);
    expect(ms.charge).toBe(true);
    expect(ms.mods).toHaveLength(1);
    expect(ms.nouveaux).toHaveLength(1);
    expect(telecharger).not.toHaveBeenCalled();
  });

  it('télécharger : un accord chiffré, puis la liste VUE donnée au cœur', async () => {
    simu.debit = 100;
    simu.reponse = liste([mod({ touche_sauvegardes: true })]);
    const ms = await ouvrir();
    await ms.proposer(leMod(ms));
    expect(confirmer).toHaveBeenCalledTimes(1);
    const [titre, o] = confirmer.mock.calls[0] as [string, { message: string }];
    expect(titre).toBe('🧩 Télécharger « 1080p » (1000 o, environ 10 s) ?');
    expect(o.message).toContain('toute la maison');
    expect(o.message).toContain('à l’abri');
    expect(telecharger).toHaveBeenCalledWith(7, [{ cle: 'wollnashorn-000000000001', revision: 1, taille: 1000, sha256: SHA }], 1000, false);
  });

  it('un « non » ne télécharge rien', async () => {
    simu.reponse = liste([mod()]);
    confirmer.mockImplementation(async () => false);
    const ms = await ouvrir();
    await ms.proposer(leMod(ms));
    expect(telecharger).not.toHaveBeenCalled();
    expect(ms.accordEnCours).toBe(false);
  });

  it('l’accord envoie au cœur l’empreinte de ce qui a été vu (sha256)', async () => {
    simu.reponse = liste([mod({ sha256: 'cd'.repeat(32) })]);
    const ms = await ouvrir();
    await ms.proposer(leMod(ms));
    const vus = telecharger.mock.calls[0][1] as { sha256: string }[];
    expect(vus[0].sha256).toBe('cd'.repeat(32));
  });

  it('tout télécharger : seuls les ⭐ recommandés cochés d’office, seuls les cochés partent, le total dit', async () => {
    simu.reponse = liste([mod({ cle: 'a-000000000001', nom: 'A', taille: 10, interet: 5 }), mod({ cle: 'b-000000000001', nom: 'B', taille: 5, interet: 2 })]);
    cocher.mockImplementation(async () => ['b-000000000001']);
    const ms = await ouvrir();
    await ms.proposerTout(ms.nouveaux);
    const [titre, options, o] = cocher.mock.calls[0] as [string, { detail: string }[], { coches: boolean[]; resume: (v: string[]) => string }];
    expect(titre).toBe('🧩 Télécharger des mods pour « Zelda » ?');
    expect(o.coches).toEqual([true, false]);
    expect(options.map((x) => x.detail)).toEqual(['⭐ 10 o', '5 o']);
    expect(o.resume(['a-000000000001', 'b-000000000001'])).toBe('2 mods, 15 o');
    expect(telecharger).toHaveBeenCalledWith(7, [{ cle: 'b-000000000001', revision: 1, taille: 5, sha256: SHA }], 5, false);
  });

  it('tout télécharger : « Tout cocher » proposé ; ni la liste ni les parties dans le message ; les parties dites selon les cases cochées', async () => {
    simu.reponse = liste([
      mod({ cle: 'a-000000000001', nom: 'A', taille: 10, touche_sauvegardes: true }),
      mod({ cle: 'b-000000000001', nom: 'B', taille: 5 }),
    ]);
    const ms = await ouvrir();
    await ms.proposerTout(ms.nouveaux);
    const o = cocher.mock.calls[0][2] as { message: string; toutCocher: boolean; resume: (v: string[]) => string };
    expect(o.toutCocher).toBe(true);
    expect(o.message).not.toContain('•');
    expect(o.message).not.toContain('parties');
    expect(o.message).toContain('Les ⭐ recommandés sont cochés d’office.');
    expect(o.resume(['b-000000000001'])).toBe('1 mod, 5 o');
    expect(o.resume(['a-000000000001', 'b-000000000001'])).toBe('2 mods, 15 o · 💾 1 mod coché touche à tes parties : Frogtend les met à l’abri avant la prochaine partie.');
  });

  it('le jeu ou le profil a changé pendant une fenêtre : rien n’est écrit ni téléchargé pour l’autre', async () => {
    simu.reponse = liste([mod({ cle: 'a-000000000001', nom: 'A', etat: 'telecharge', revision_rangee: 1, touche_sauvegardes: true, allume: true }), mod({ cle: 'b-000000000001', nom: 'B' })]);
    let finir: (v: boolean) => void = () => {};
    confirmer.mockImplementation(() => new Promise<boolean>((r) => (finir = r)));
    const ms = await ouvrir();
    const a = leMod(ms, 'a-000000000001');
    const bascule = ms.basculer(a);
    await attendre();
    ms.ouvrir(7, 'Zelda', 'niece-2');
    finir(true);
    await bascule;
    expect(choisirMod).not.toHaveBeenCalled();
    await attendre();
    const b = leMod(ms, 'b-000000000001');
    const accord = ms.proposer(b);
    await attendre();
    ms.ouvrir(8, 'Mario', 'niece-2');
    finir(true);
    await accord;
    expect(telecharger).not.toHaveBeenCalled();
  });

  it('mise à jour automatique refusée parce que des mods arrivent déjà : rien n’est dit', async () => {
    simu.profil.modsSwitch.auto = true;
    simu.reponse = liste([mod({ etat: 'revision_dispo', revision: 2, revision_rangee: 1 })]);
    telecharger.mockImplementation(async () => Promise.reject({ sorte: 'refus', motif: 'Des mods de ce jeu sont déjà en train d’arriver.' }));
    await ouvrir();
    expect(telecharger).toHaveBeenCalledTimes(1);
    expect(toast).not.toHaveBeenCalled();
    // Demandé par la personne : le refus est dit.
    simu.profil.modsSwitch.auto = false;
    simu.reponse = liste([mod()]);
    const ms = await ouvrir();
    await ms.proposer(leMod(ms));
    expect(toast).toHaveBeenCalledWith(expect.stringContaining('déjà en train'), 'erreur');
  });

  it('après « Préparer », le focus va à l’élément de secours (le bouton a disparu)', async () => {
    simu.reponse = liste([dk({ etat: 'telecharge', revision_rangee: 1, allume: true })], {
      musiques: { actives: 1, raison: null, taille_connue: 41018496, meme_ensemble: false, a_preparer: true },
    });
    const ms = await ouvrir();
    const secours = document.createElement('button');
    document.body.append(secours);
    ms.lieuDuFocus = () => ({ conteneur: null, secours });
    (document.activeElement as HTMLElement | null)?.blur();
    await ms.preparer();
    expect(document.activeElement).toBe(secours);
    secours.remove();
  });

  it('tout télécharger : seulement les mods donnés (l’onglet affiché), jamais les autres', async () => {
    simu.reponse = liste([mod({ cle: 'a-000000000001', nom: 'A' }), mod({ cle: 'b-000000000001', nom: 'B' }), mod({ cle: 'c-000000000001', nom: 'C' })]);
    cocher.mockImplementation(async (...a: unknown[]) => (a[1] as { valeur: string }[]).map((x) => x.valeur));
    const ms = await ouvrir();
    await ms.proposerTout(ms.mods.filter((m) => m.cle !== 'b-000000000001'));
    expect((cocher.mock.calls[0][1] as { valeur: string }[]).map((x) => x.valeur)).toEqual(['a-000000000001', 'c-000000000001']);
    expect((telecharger.mock.calls[0][1] as { cle: string }[]).map((x) => x.cle)).toEqual(['a-000000000001', 'c-000000000001']);
  });

  it('tout télécharger : rien de coché, la fenêtre le dit', async () => {
    simu.reponse = liste([mod({ cle: 'a-000000000001', nom: 'A' }), mod({ cle: 'b-000000000001', nom: 'B' })]);
    const ms = await ouvrir();
    await ms.proposerTout(ms.nouveaux);
    const o = cocher.mock.calls[0][2] as { refus: (v: string[]) => string | null };
    expect(o.refus([])).toBe('Coche au moins un mod.');
    expect(o.refus(['a-000000000001'])).toBeNull();
    expect(telecharger).not.toHaveBeenCalled();
  });

  it('la liste a changé : le refus du cœur est dit et la liste relue', async () => {
    simu.reponse = liste([mod()]);
    telecharger.mockImplementation(async () => Promise.reject({ sorte: 'conflit', motif: 'La liste des mods a changé.' }));
    const ms = await ouvrir();
    lister.mockClear();
    await ms.proposer(leMod(ms));
    expect(toast).toHaveBeenCalledWith(expect.stringContaining('La liste des mods a changé'), 'alerte');
    expect(lister).toHaveBeenCalled();
  });

  it('les refus du cœur sont dits sans jargon', async () => {
    simu.reponse = liste([mod()]);
    const ms = await ouvrir();
    telecharger.mockImplementation(async () => {
      throw { sorte: 'refus', motif: 'archive zip sans exefs ni cheats' };
    });
    await ms.proposer(leMod(ms));
    const t = String(toast.mock.calls.at(-1)?.[0]);
    expect(t).toContain('archive archive sans programme du jeu ni triches');
    expect(t).not.toMatch(/zip|exefs|cheats/);
  });

  it('allumer un mod en conflit : confirmé, puis l’autre est éteint d’un coup', async () => {
    simu.reponse = liste([
      mod({ cle: 'a-000000000001', nom: 'A', etat: 'telecharge', revision_rangee: 1, conflits: ['b-000000000001'] }),
      mod({ cle: 'b-000000000001', nom: 'B', etat: 'telecharge', revision_rangee: 1, allume: true }),
    ]);
    const ms = await ouvrir();
    await ms.basculer(leMod(ms, 'a-000000000001'));
    expect(confirmer.mock.calls[0][0]).toBe('Ne va pas avec « B » : l’allumer éteint l’autre');
    expect(choisirMod).toHaveBeenCalledWith(7, 'a-000000000001', true, 'b-000000000001');
  });

  it('un conflit refusé par le cœur que la liste ne montrait pas : relue, demandé, puis réessayé', async () => {
    simu.reponse = liste([
      mod({ cle: 'a-000000000001', nom: 'A', etat: 'telecharge', revision_rangee: 1 }),
      mod({ cle: 'b-000000000001', nom: 'B', etat: 'telecharge', revision_rangee: 1, allume: true }),
    ]);
    const ms = await ouvrir();
    // Entre-temps, Firehouse a déclaré le conflit.
    simu.reponse = liste([
      mod({ cle: 'a-000000000001', nom: 'A', etat: 'telecharge', revision_rangee: 1, conflits: ['b-000000000001'] }),
      mod({ cle: 'b-000000000001', nom: 'B', etat: 'telecharge', revision_rangee: 1, allume: true }),
    ]);
    choisirMod.mockImplementationOnce(async () => Promise.reject({ sorte: 'refus', motif: '« A » ne va pas avec « B »' }));
    await ms.basculer(leMod(ms, 'a-000000000001'));
    expect(choisirMod).toHaveBeenNthCalledWith(1, 7, 'a-000000000001', true, null);
    expect(confirmer).toHaveBeenCalledTimes(1);
    expect(choisirMod).toHaveBeenNthCalledWith(2, 7, 'a-000000000001', true, 'b-000000000001');
  });

  it('éteindre un mod qui touche aux parties demande confirmation ; un double appui n’enregistre qu’une fois', async () => {
    simu.reponse = liste([mod({ etat: 'telecharge', revision_rangee: 1, allume: true, touche_sauvegardes: true })]);
    let finir: (v: boolean) => void = () => {};
    confirmer.mockImplementation(() => new Promise<boolean>((r) => (finir = r)));
    const ms = await ouvrir();
    const premier = ms.basculer(leMod(ms));
    const second = ms.basculer(leMod(ms));
    await attendre();
    expect(confirmer).toHaveBeenCalledTimes(1);
    finir(true);
    await Promise.all([premier, second]);
    expect(choisirMod).toHaveBeenCalledTimes(1);
    expect(choisirMod).toHaveBeenCalledWith(7, 'wollnashorn-000000000001', false, null);
  });

  it('pendant une partie : rien ne change (ni interrupteur, ni téléchargement, ni retrait)', async () => {
    simu.partie.enJeu = 3;
    simu.reponse = liste([mod({ etat: 'telecharge', revision_rangee: 1, servi: false }), mod({ cle: 'b-000000000001', nom: 'B' })]);
    const ms = await ouvrir();
    expect(ms.bloque).toBe(true);
    await ms.basculer(leMod(ms));
    await ms.proposer(leMod(ms, 'b-000000000001'));
    await ms.retirer(leMod(ms));
    expect(choisirMod).not.toHaveBeenCalled();
    expect(confirmer).not.toHaveBeenCalled();
    expect(telecharger).not.toHaveBeenCalled();
  });

  it('nouvelle révision : « à l’avenir, mettre à jour tout seul » est retenu dans le profil', async () => {
    simu.reponse = liste([mod({ etat: 'revision_dispo', revision: 2, revision_rangee: 1, allume: true })]);
    choisirDialogue.mockImplementation(async () => 'auto');
    const ms = await ouvrir();
    await ms.proposerMiseAJour(leMod(ms));
    expect(choisirDialogue.mock.calls[0][0]).toBe('🧩 Mettre à jour « 1080p » (1000 o) ?');
    expect(reglerProfil).toHaveBeenCalledWith('modsSwitch.auto', true);
    expect(telecharger).toHaveBeenCalledWith(7, [{ cle: 'wollnashorn-000000000001', revision: 2, taille: 1000, sha256: SHA }], 1000, true);
  });

  it('mise à jour automatique choisie : les révisions partent seules à l’ouverture (dit par un toast)', async () => {
    simu.profil.modsSwitch.auto = true;
    simu.reponse = liste([mod({ etat: 'revision_dispo', revision: 2, revision_rangee: 1 }), mod({ cle: 'b-000000000001', nom: 'B' })]);
    await ouvrir();
    expect(telecharger).toHaveBeenCalledTimes(1);
    expect(telecharger).toHaveBeenCalledWith(7, [{ cle: 'wollnashorn-000000000001', revision: 2, taille: 1000, sha256: SHA }], 1000, true);
    expect(toast).toHaveBeenCalledWith(expect.stringContaining('1 mod de « Zelda » se met à jour tout seul'));
    expect(choisirDialogue).not.toHaveBeenCalled();
  });

  it('mise à jour automatique : jamais pendant une partie', async () => {
    simu.profil.modsSwitch.auto = true;
    simu.partie.enJeu = 3;
    simu.reponse = liste([mod({ etat: 'revision_dispo', revision: 2, revision_rangee: 1 })]);
    await ouvrir();
    expect(telecharger).not.toHaveBeenCalled();
  });

  it('mise à jour automatique : une seule fois, même si la bande et la page s’ouvrent ensemble', async () => {
    simu.profil.modsSwitch.auto = true;
    simu.reponse = liste([mod({ etat: 'revision_dispo', revision: 2, revision_rangee: 1 })]);
    let fin: (n: number) => void = () => {};
    telecharger.mockImplementation(() => new Promise<number>((r) => (fin = r)));
    const a = new ModsSwitchJeu();
    const b = new ModsSwitchJeu();
    a.ouvrir(7, 'Zelda', 'seb');
    b.ouvrir(7, 'Zelda', 'seb');
    await attendre();
    expect(telecharger).toHaveBeenCalledTimes(1);
    fin(1);
    await attendre();
  });

  it('un mod qui a fini avant la première lecture du suivi : la fin est dite quand même', async () => {
    simu.reponse = liste([mod()]);
    const ms = await ouvrir();
    telecharger.mockImplementation(async () => {
      simu.suiviReponse = { ...INACTIF, faits: 1, a_faire: 1 };
      return 1;
    });
    lister.mockClear();
    await ms.proposer(leMod(ms));
    expect(toast).toHaveBeenCalledWith(expect.stringContaining('1 mod prêt'), 'ok');
    expect(ms.annonceFin).toBe('Mods téléchargés.');
    expect(lister).toHaveBeenCalled();
  });

  it('le suivi relevé : la fin se dit (toast, annonce) et la liste est relue', async () => {
    simu.reponse = liste([mod()]);
    simu.suiviReponse = { ...INACTIF, actif: true, en_cours: '1080p', recus: 10, total: 1000, a_faire: 1 };
    const ms = await ouvrir();
    expect(ms.actif).toBe(true);
    expect(ms.enCours(leMod(ms))).toBe(true);
    const g = ms.debutSuivi();
    simu.suiviReponse = { ...INACTIF, faits: 1, a_faire: 1 };
    lister.mockClear();
    await ms.releverSuivi(g);
    expect(ms.actif).toBe(false);
    expect(toast).toHaveBeenCalledWith('🧩 1 mod prêt : il s’allume d’un simple bouton.', 'ok');
    expect(ms.annonceFin).toBe('Mods téléchargés.');
    expect(lister).toHaveBeenCalled();
  });

  it('un « Retirer » : confirmé (danger), puis le cœur et la liste relue', async () => {
    simu.reponse = liste([mod({ etat: 'telecharge', revision_rangee: 1, servi: false })]);
    const ms = await ouvrir();
    lister.mockClear();
    await ms.retirer(leMod(ms));
    expect((confirmer.mock.calls[0][1] as { danger: boolean }).danger).toBe(true);
    expect(retirer).toHaveBeenCalledWith(7, 'wollnashorn-000000000001');
    expect(lister).toHaveBeenCalled();
  });

  it('un autre jeu ou un autre profil : tout est remis à zéro, la lecture d’avant est jetée', async () => {
    let premiere: (v: unknown) => void = () => {};
    lister.mockImplementationOnce(() => new Promise((r) => (premiere = r)));
    const ms = new ModsSwitchJeu();
    ms.ouvrir(7, 'Zelda', 'seb');
    // Le profil change pendant la lecture : la liste de Seb n'est jamais montrée à l'autre profil.
    simu.reponse = liste([mod({ cle: 'b-000000000001', nom: 'B' })]);
    ms.ouvrir(7, 'Zelda', 'niece');
    await attendre();
    premiere(liste([mod({ nom: 'Réservé à Seb' })]));
    await attendre();
    expect(ms.profil).toBe('niece');
    expect(ms.mods.map((m) => m.nom)).toEqual(['B']);
    // Et à la réouverture, tout repart de zéro avant la nouvelle lecture.
    ms.ouvrir(8, 'Mario', 'niece');
    expect(ms.liste).toBeNull();
    expect(ms.charge).toBe(false);
    expect(ms.suivi).toBeNull();
  });

  it('le nombre de mods vu est noté par profil (« Gérer le jeu » ne montre jamais celui d’un autre)', async () => {
    simu.reponse = liste([mod(), mod({ cle: 'b-000000000001', nom: 'B' })]);
    await ouvrir();
    lister.mockClear();
    expect(await nombreDeModsSwitch(7)).toBe(2);
    expect(lister).not.toHaveBeenCalled();
    simu.profilOuvert = { id: 'cousine' };
    simu.reponse = liste([mod()]);
    expect(await nombreDeModsSwitch(7)).toBe(1);
    expect(lister).toHaveBeenCalledTimes(1);
    // Firehouse injoignable : pas de nombre inventé.
    simu.profilOuvert = { id: 'autre' };
    simu.reponse = liste([], { etat: 'injoignable' });
    expect(await nombreDeModsSwitch(7)).toBeNull();
  });

  it('« Gérer le jeu » n’attend jamais : « 🧩 Mods » tout de suite, la liste lue en arrière-plan, puis « 🧩 Mods (N) »', async () => {
    simu.profilOuvert = { id: 'menu' };
    let fin: (v: unknown) => void = () => {};
    lister.mockImplementationOnce(() => new Promise((r) => (fin = r)));
    const o = optionModsSwitch(42);
    // Rendu sans attendre la lecture (encore en cours).
    expect(o).toEqual({ valeur: 'modsSwitch', libelle: '🧩 Mods', detail: expect.stringContaining('pour toute la maison') });
    await Promise.resolve();
    expect(lister).toHaveBeenCalledWith(42);
    expect(nombreDeModsVu(42)).toBeUndefined();
    fin(liste([mod(), mod({ cle: 'b-000000000001', nom: 'B' }), mod({ cle: 'c-000000000001', nom: 'C' })]));
    await attendre();
    expect(nombreDeModsVu(42)).toBe(3);
    lister.mockClear();
    expect(optionModsSwitch(42)?.libelle).toBe('🧩 Mods (3)');
    expect(lister).not.toHaveBeenCalled();
    // Un jeu sans aucun mod : pas d'entrée.
    simu.reponse = liste([]);
    await nombreDeModsSwitch(43);
    expect(optionModsSwitch(43)).toBeNull();
    // Un autre profil ne voit jamais le nombre de celui-ci.
    simu.profilOuvert = { id: 'menu-2' };
    expect(nombreDeModsVu(42)).toBeUndefined();
  });

  it('rendreFocus : la commande de la ligne du mod, sinon la suivante dans la liste affichée, sinon le secours', async () => {
    simu.reponse = liste([mod({ cle: 'a-000000000001', nom: 'A' }), mod({ cle: 'b-000000000001', nom: 'B' })]);
    const ms = await ouvrir();
    const ul = document.createElement('ul');
    ul.innerHTML =
      '<li data-cle="b-000000000001" tabindex="-1"><button data-sorte="nom">B</button><button data-commande>⬇ B</button></li>';
    const secours = document.createElement('button');
    document.body.append(ul, secours);
    ms.ordreVisible = () => ['a-000000000001', 'b-000000000001'];
    confirmer.mockImplementation(async () => false);
    ms.lieuDuFocus = () => ({ conteneur: ul, secours });
    (document.activeElement as HTMLElement | null)?.blur();
    // La ligne de A a disparu (filtrée) : la suivante AFFICHÉE, sa commande et non son nom.
    await ms.proposer(leMod(ms, 'a-000000000001'));
    expect(document.activeElement?.textContent).toBe('⬇ B');
    (document.activeElement as HTMLElement).blur();
    ul.innerHTML = '';
    await ms.rendreFocus('a-000000000001', ul, secours);
    expect(document.activeElement).toBe(secours);
    ul.remove();
    secours.remove();
  });
});

describe('les musiques de Mario Kart 8 Deluxe (contrat 1.55)', () => {
  it('télécharger un mod de musique : l’accord annonce aussi « + 41 Mo de musiques », la taille va au cœur', async () => {
    simu.reponse = liste([dk()]);
    const ms = await ouvrir();
    await ms.proposer(leMod(ms, DK));
    expect(tailleMusiques).toHaveBeenCalledWith(7, [DK], null);
    const [titre, o] = confirmer.mock.calls[0] as [string, { message: string }];
    expect(titre).toBe('🧩 Télécharger « DK Summit » (1000 o + 41018496 o de musiques) ?');
    expect(o.message).toContain('🎵 + 41018496 o de musiques');
    expect(telecharger).toHaveBeenCalledWith(7, [{ cle: DK, revision: 1, taille: 1000, sha256: SHA }], 1000, false, 41018496);
  });

  it('un mod de musique qui arrivera éteint (conflit avec un mod allumé) n’annonce pas de musiques', async () => {
    simu.reponse = liste([dk(), mod({ cle: REMIX, nom: 'Sherbet remix', etat: 'telecharge', revision_rangee: 1, allume: true, amorces: ['pBGM_GC_SHERBET_LAND_N'] })]);
    const ms = await ouvrir();
    await ms.proposer(leMod(ms, DK));
    expect(tailleMusiques).not.toHaveBeenCalled();
    expect(confirmer.mock.calls[0][0]).toBe('🧩 Télécharger « DK Summit » (1000 o) ?');
    expect(telecharger).toHaveBeenCalledWith(7, [{ cle: DK, revision: 1, taille: 1000, sha256: SHA }], 1000, false);
  });

  it('Firehouse muet : la taille du dernier fichier connu est annoncée ; hors ligne, sans rien demander', async () => {
    tailleMusiques.mockImplementation(async () => Promise.reject({ sorte: 'reseau', motif: 'x' }));
    simu.reponse = liste([dk({ etat: 'telecharge', revision_rangee: 1 })], { musiques: { actives: 0, raison: null, taille_connue: 40000000 } });
    let ms = await ouvrir();
    await ms.basculer(leMod(ms, DK));
    expect(confirmer.mock.calls[0][0]).toContain('+ 40000000 o de musiques');
    tailleMusiques.mockClear();
    confirmer.mockClear();
    simu.reponse = liste([dk({ etat: 'telecharge', revision_rangee: 1 })], { etat: 'hors_ligne', musiques: { actives: 0, raison: null, taille_connue: 40000000 } });
    ms = await ouvrir();
    await ms.basculer(leMod(ms, DK));
    expect(tailleMusiques).not.toHaveBeenCalled();
    expect(confirmer.mock.calls[0][0]).toContain('+ 40000000 o de musiques');
  });

  it('pendant que la taille est demandée : `regardeTaille` est vrai (« ⏳ Je regarde… » à l’écran)', async () => {
    let finir: (n: number) => void = () => {};
    tailleMusiques.mockImplementation(() => new Promise<number>((r) => (finir = r)));
    simu.reponse = liste([dk({ etat: 'telecharge', revision_rangee: 1 })]);
    const ms = await ouvrir();
    const b = ms.basculer(leMod(ms, DK));
    await attendre();
    expect(ms.regardeTaille).toBe(true);
    finir(41018496);
    await b;
    expect(ms.regardeTaille).toBe(false);
  });

  it('allumer un mod de musique : l’accord dit la taille ; un « non » ne change rien ; un oui donne la taille au cœur', async () => {
    simu.reponse = liste([dk({ etat: 'telecharge', revision_rangee: 1 })]);
    const ms = await ouvrir();
    confirmer.mockImplementation(async () => false);
    await ms.basculer(leMod(ms, DK));
    expect(confirmer.mock.calls[0][0]).toBe('🎵 Allumer « DK Summit » (+ 41018496 o de musiques) ?');
    expect(choisirMod).not.toHaveBeenCalled();
    confirmer.mockImplementation(async () => true);
    await ms.basculer(leMod(ms, DK));
    expect(choisirMod).toHaveBeenCalledWith(7, DK, true, null, 41018496);
  });

  it('sans taille connue : allumé, mais rien n’est accordé ni préchargé (c’est dit)', async () => {
    tailleMusiques.mockImplementation(async () => null);
    simu.reponse = liste([dk({ etat: 'telecharge', revision_rangee: 1 })]);
    const ms = await ouvrir();
    await ms.basculer(leMod(ms, DK));
    const [titre, o] = confirmer.mock.calls[0] as [string, { message: string }];
    expect(titre).toContain('+ des musiques, taille pas encore connue');
    expect(o.message).toContain('les musiques seront demandées à la prochaine partie');
    expect(choisirMod).toHaveBeenCalledWith(7, DK, true, null);
  });

  it('deux mods de musique qui changent la même musique : UNE fenêtre (éteindre l’autre + la taille)', async () => {
    simu.reponse = liste([
      dk({ etat: 'telecharge', revision_rangee: 1 }),
      mod({ cle: REMIX, nom: 'Sherbet remix', etat: 'telecharge', revision_rangee: 1, allume: true, amorces: ['pBGM_GC_SHERBET_LAND_N'] }),
    ]);
    const ms = await ouvrir();
    await ms.basculer(leMod(ms, DK));
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
    await ms.basculer(leMod(ms, DK));
    expect(choisirMod).not.toHaveBeenCalled();
  });

  it('« 🎵 Préparer les musiques » : la taille, l’accord, puis le cœur ; un « non » ne prépare rien ; sans taille, rien', async () => {
    simu.reponse = liste([dk({ etat: 'telecharge', revision_rangee: 1, allume: true })], {
      musiques: { actives: 1, raison: null, taille_connue: 41018496, meme_ensemble: false, a_preparer: true },
    });
    let ms = await ouvrir();
    confirmer.mockImplementation(async () => false);
    await ms.preparer();
    expect(confirmer.mock.calls[0][0]).toBe('🎵 Préparer les musiques des mods (41018496 o) ?');
    expect(preparerMusiques).not.toHaveBeenCalled();
    confirmer.mockImplementation(async () => true);
    await ms.preparer();
    expect(preparerMusiques).toHaveBeenCalledWith(7, 41018496);
    // Taille inconnue (Firehouse muet, rien de connu) : rien n'est préparé, c'est dit.
    preparerMusiques.mockClear();
    confirmer.mockClear();
    tailleMusiques.mockImplementation(async () => null);
    simu.reponse = liste([dk({ etat: 'telecharge', revision_rangee: 1, allume: true })], {
      musiques: { actives: 1, raison: null, taille_connue: null, meme_ensemble: false, a_preparer: true },
    });
    ms = await ouvrir();
    await ms.preparer();
    expect(confirmer).not.toHaveBeenCalled();
    expect(preparerMusiques).not.toHaveBeenCalled();
    expect(toast).toHaveBeenCalledWith(expect.stringContaining('pas encore connue'), 'alerte');
  });
});
