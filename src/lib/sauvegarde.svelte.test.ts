// Lot S4 : la sauvegarde d'après une partie attend la fin de l'abri automatique des parties (événement
// `parties-abris-finis` du même profil et du même jeu, 15 minutes au plus) ; un profil changé entre-temps : rien
// n'est envoyé. Les fichiers sautés, la place manquante et « déjà en cours » sont dits sans chemin.
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => () => {}) }));
vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => false }));

const sauvegarderCoeur = vi.fn(async (_ecarter?: boolean, _plafond?: number | null): Promise<Record<string, unknown>> => ({ envoyes: 1, inchanges: 0, retires: 0, octets: 5, fichiers: 1, taille: 5, date: '1', pc: 'PC', dossier: 'Firehouse' }));
const estimer = vi.fn(async (): Promise<Record<string, unknown> | null> => ({ total: 5, precedent: 5, a_envoyer: 5, fichiers_a_envoyer: 1, utilise: 0, quota: 1000 }));
vi.mock('$lib/api', () => ({
  api: { sauvegarder: (e: boolean, p: number | null) => sauvegarderCoeur(e, p), sauvegardeEstimer: () => estimer() },
  estErreurCoeur: (e: unknown) => typeof e === 'object' && e !== null && 'sorte' in e,
  taille: (o: number) => `${o} o`,
}));

const toast = vi.fn((..._a: unknown[]) => 0);
vi.mock('$lib/dialogues/fenetres.svelte', () => ({
  choisir: vi.fn(),
  confirmer: vi.fn(async () => true),
  informer: vi.fn(),
  toast: (...a: unknown[]) => toast(...a),
}));
vi.mock('$lib/dialogues/messages', () => ({ motifDuRefus: (e: unknown) => String((e as { motif?: string }).motif ?? e) }));

const etat = vi.hoisted(() => ({
  profilOuvert: { id: 'seb' } as { id: string } | null,
  profil: { sauvegarde: { auto: 'apres_partie' } },
  pc: { firehouse: { simule: false }, seuilSauvegardeMo: 200 },
}));
vi.mock('$lib/etat.svelte', () => ({ etat, reglerPc: vi.fn(), remplacerReglagesProfil: vi.fn() }));
vi.mock('$lib/emulateurs/choix', () => ({ ajouter: vi.fn(), normaliserTous: vi.fn(() => ({})) }));
vi.mock('$lib/ludotheque/ludotheque.svelte', () => ({ depuis: () => '' }));
vi.mock('$lib/ludotheque/telechargements.svelte', () => ({ rechargerJeuxDuPc: vi.fn(), tele: { jeux: {} } }));
vi.mock('$lib/ludotheque/dossierJeux', () => ({ proposerDossierJeux: vi.fn() }));
vi.mock('$lib/ludotheque/cartes', () => ({ aCartes: () => false }));

import { apresUnePartie, attendreAbrisFinis, recevoirAbrisFinis, recevoirSauvegardeFinie, sauvegarde, sauvegarder, DEJA_EN_COURS, GROSSE_AUTO, PLAFOND_RELANCE_MS } from './sauvegarde.svelte';

/** Laisse passer les promesses en attente. */
const laisser = () => new Promise((r) => setTimeout(r, 0));

beforeEach(() => {
  vi.clearAllMocks();
  etat.profilOuvert = { id: 'seb' };
  etat.profil.sauvegarde.auto = 'apres_partie';
  sauvegarde.enCours = false;
  sauvegarde.aRefaire = false;
});

describe('la sauvegarde d’après une partie', () => {
  it('attend la fin de l’abri automatique du même profil et du même jeu, puis envoie', async () => {
    const attente = attendreAbrisFinis('seb', 110);
    const fini = apresUnePartie({ jeu: 110, abriApres: true, attente });
    await laisser();
    expect(sauvegarderCoeur).not.toHaveBeenCalled();
    // Un autre jeu, puis un autre profil : on attend encore.
    recevoirAbrisFinis({ profil: 'seb', jeu: 7, faits: 1 });
    recevoirAbrisFinis({ profil: 'lea', jeu: 110, faits: 1 });
    await laisser();
    expect(sauvegarderCoeur).not.toHaveBeenCalled();
    recevoirAbrisFinis({ profil: 'seb', jeu: 110, faits: 1 });
    await fini;
    expect(sauvegarderCoeur).toHaveBeenCalledTimes(1);
  });

  it('un événement arrivé avant que la sauvegarde ne demande n’est jamais perdu', async () => {
    const attente = attendreAbrisFinis('seb', 110);
    recevoirAbrisFinis({ profil: 'seb', jeu: 110, faits: 0 });
    await apresUnePartie({ jeu: 110, abriApres: true, attente });
    expect(sauvegarderCoeur).toHaveBeenCalledTimes(1);
  });

  it('n’envoie rien si le profil a changé pendant l’attente', async () => {
    const attente = attendreAbrisFinis('seb', 110);
    const fini = apresUnePartie({ jeu: 110, abriApres: true, attente });
    etat.profilOuvert = { id: 'lea' };
    recevoirAbrisFinis({ profil: 'seb', jeu: 110, faits: 1 });
    await fini;
    expect(sauvegarderCoeur).not.toHaveBeenCalled();
  });

  it('attend 15 minutes au plus, puis envoie quand même', async () => {
    vi.useFakeTimers();
    try {
      const fini = apresUnePartie({ jeu: 110, abriApres: true, attente: attendreAbrisFinis('seb', 110) });
      await vi.advanceTimersByTimeAsync(15 * 60 * 1000 - 1);
      expect(sauvegarderCoeur).not.toHaveBeenCalled();
      await vi.advanceTimersByTimeAsync(1);
      await fini;
      expect(sauvegarderCoeur).toHaveBeenCalledTimes(1);
    } finally {
      vi.useRealTimers();
    }
  });

  it('sans abri après la partie (émulateur, relance) : envoie tout de suite ; sauvegarde manuelle : rien', async () => {
    await apresUnePartie({ jeu: 5, abriApres: false });
    expect(sauvegarderCoeur).toHaveBeenCalledTimes(1);
    etat.profil.sauvegarde.auto = 'manuelle';
    await apresUnePartie({ jeu: 5, abriApres: true });
    expect(sauvegarderCoeur).toHaveBeenCalledTimes(1);
  });
});

describe('ce que la sauvegarde dit', () => {
  it('dit les fichiers sautés, même en automatique, sans les nommer', async () => {
    sauvegarderCoeur.mockResolvedValueOnce({ envoyes: 2, inchanges: 0, retires: 0, octets: 5, fichiers: 3, taille: 5, date: '1', pc: 'PC', dossier: 'F', sautes: 1 });
    expect(await sauvegarder(true)).toBe(true);
    const [message, sorte] = toast.mock.calls[0] as [string, string];
    expect(message).toContain('1 fichier(s) n’ont pas pu partir');
    expect(message).toContain('Frogtend réessaiera');
    expect(sorte).toBe('alerte');
  });

  it('dit clairement la place qui manque chez Firehouse, sans rien envoyer', async () => {
    // La place qui manque est celle calculée par le cœur (`manque`), jamais recalculée ici.
    estimer.mockResolvedValueOnce({ total: 900, precedent: 0, a_envoyer: 900, fichiers_a_envoyer: 3, utilise: 500, quota: 1000, manque: 400 });
    expect(await sauvegarder(false)).toBe(false);
    expect(sauvegarderCoeur).not.toHaveBeenCalled();
    expect(String(toast.mock.calls[0][0])).toContain('il manque 400 o chez Firehouse (500 o sur 1000 o)');
    // Le refus du cœur (507) est reconnu par sa sorte.
    sauvegarderCoeur.mockRejectedValueOnce({ sorte: 'plein', motif: 'Ton espace chez Firehouse est plein.' });
    expect(await sauvegarder(false)).toBe(false);
    expect(toast.mock.calls[1]).toEqual(['💾 Sauvegarde non faite : Ton espace chez Firehouse est plein.', 'erreur']);
  });

  it('une sauvegarde demandée pendant une autre est refaite juste après, jamais abandonnée', async () => {
    let finir: (v: Record<string, unknown>) => void = () => {};
    sauvegarderCoeur.mockImplementationOnce(() => new Promise((r) => (finir = r)));
    const premiere = sauvegarder(true);
    await laisser();
    expect(await sauvegarder(true)).toBe(false);
    expect(sauvegarde.aRefaire).toBe(true);
    finir({ envoyes: 0, inchanges: 1, retires: 0, octets: 0, fichiers: 1, taille: 1, date: '2', pc: 'PC', dossier: 'F' });
    await premiere;
    await laisser();
    await laisser();
    expect(sauvegarderCoeur).toHaveBeenCalledTimes(2);
    expect(toast).not.toHaveBeenCalled();
  });

  it('« déjà en cours » côté cœur : jamais de relance immédiate, une seule attente, repartie à sauvegarde-finie', async () => {
    vi.useFakeTimers();
    try {
      const refus = { sorte: 'refus', motif: `${DEJA_EN_COURS} : elle sera refaite juste après.` };
      sauvegarderCoeur.mockRejectedValueOnce(refus).mockRejectedValueOnce(refus);
      await sauvegarder(true);
      await sauvegarder(true);
      await vi.advanceTimersByTimeAsync(10);
      expect(sauvegarderCoeur).toHaveBeenCalledTimes(2);
      // Le cœur a rendu son verrou : UNE relance, aussitôt.
      recevoirSauvegardeFinie();
      await vi.advanceTimersByTimeAsync(10);
      expect(sauvegarderCoeur).toHaveBeenCalledTimes(3);
      // Plus rien n'attend : ni l'événement, ni le plafond ne relancent encore.
      recevoirSauvegardeFinie();
      await vi.advanceTimersByTimeAsync(PLAFOND_RELANCE_MS + 10);
      expect(sauvegarderCoeur).toHaveBeenCalledTimes(3);
      // Sans l'événement : au bout du plafond (60 s).
      sauvegarderCoeur.mockRejectedValueOnce(refus);
      await sauvegarder(true);
      await vi.advanceTimersByTimeAsync(PLAFOND_RELANCE_MS - 100);
      expect(sauvegarderCoeur).toHaveBeenCalledTimes(4);
      await vi.advanceTimersByTimeAsync(200);
      expect(sauvegarderCoeur).toHaveBeenCalledTimes(5);
      expect(toast).not.toHaveBeenCalled();
    } finally {
      vi.useRealTimers();
    }
  });

  it('une sauvegarde automatique ne fait pas d’estimation séparée : le cœur juge la taille, dite une seule fois', async () => {
    sauvegarderCoeur.mockRejectedValueOnce({ sorte: 'refus', motif: `${GROSSE_AUTO} : 300,0 Mo de parties à envoyer.` });
    expect(await sauvegarder(true)).toBe(false);
    expect(estimer).not.toHaveBeenCalled();
    expect(sauvegarderCoeur).toHaveBeenCalledWith(false, 200 * 1024 * 1024);
    expect(String(toast.mock.calls[0][0])).toContain('remise à plus tard');
    sauvegarderCoeur.mockRejectedValueOnce({ sorte: 'refus', motif: `${GROSSE_AUTO} : 300,0 Mo de parties à envoyer.` });
    await sauvegarder(true);
    expect(toast).toHaveBeenCalledTimes(1);
  });
});
