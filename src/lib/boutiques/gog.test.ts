// Lot C4a (Seb 21/11) : la lecture des jeux du compte GOG connecté — à l'ouverture de Frogtend (discret), à la main
// (message d'attente qui reste), juste après la connexion, jamais pendant une partie ni pendant la récupération du jeu
// offert, silence si le profil a changé ; un seul état de connexion GOG pour le jeu offert et les jeux du compte.
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { EtatGog, ImportGog } from '$lib/api';

const etatFaux = {
  profilOuvert: { id: 'seb' } as { id: string } | null,
  profil: { gratuits: { connectes: { epic: false, gog: true, prime: false, psplus: false } } },
};
const partieFausse = { enJeu: null as number | null };
const gratuitsFaux = { boutique: null as string | null, lectureGog: false, gogDeconnecte: false };
let etatGog: EtatGog = { connu: true, nb_jeux: 3, maj_le: null };
const importer = vi.fn(async (): Promise<ImportGog> => ({ reconnecter: false, total: 3, nouveaux: 0, nouveaux_installes: 0, galaxy: true }));
const lectureVue: boolean[] = [];
const montres: boolean[] = [];
const toast = vi.fn((..._a: unknown[]) => 7);
const fermerToast = vi.fn();
const rafraichir = vi.fn(async () => {});
const noter = vi.fn(async (_b: string, e: string) => {
  etatFaux.profil.gratuits.connectes.gog = e !== 'connexion';
});

vi.mock('$lib/api', () => ({ api: { gogEtat: async () => etatGog, gogImporterConnecte: (montrer?: boolean) => { montres.push(!!montrer); return importer(); } } }));
vi.mock('$lib/dialogues/fenetres.svelte', () => ({ toast: (...a: unknown[]) => toast(...a), fermerToast: (id: number) => fermerToast(id) }));
vi.mock('$lib/dialogues/messages', () => ({ motifDuRefus: () => 'GOG ne répond pas' }));
vi.mock('$lib/etat.svelte', () => ({ etat: etatFaux }));
vi.mock('$lib/ludotheque/jeu.svelte', () => ({ partie: partieFausse }));
vi.mock('./boutiques-pc.svelte', () => ({ rafraichirLudotheque: () => rafraichir() }));
vi.mock('./gratuits.svelte', () => ({ gratuits: gratuitsFaux, noterConnexion: (b: string, e: string) => noter(b, e) }));

const {
  annonceImportGog,
  connexionGogExpiree,
  direGogExpireAujourdhui,
  fermeturePourMoi,
  gog,
  gogAuDemarrage,
  gogOccupe,
  importGogEnCours,
  MESSAGE_GOG_ATTENTE,
  MESSAGE_GOG_EXPIRE,
  listeGogPartielle,
  messageGogPasConnecte,
  mettreAJourGog,
  sansGalaxy,
} = await import('./gog.svelte');

const rien: ImportGog = { reconnecter: false, total: 3, nouveaux: 0, nouveaux_installes: 0, galaxy: true, bibliotheque_galaxy: true };
const expire: ImportGog = { reconnecter: true, total: 0, nouveaux: 0, nouveaux_installes: 0, galaxy: true };

describe('les messages de GOG (purs)', () => {
  it('à l’ouverture : rien sans nouveauté, les nouveaux sinon ; à la main : « Rien de nouveau »', () => {
    expect(annonceImportGog(rien, 'discret')).toBeNull();
    expect(annonceImportGog({ ...rien, nouveaux: 12, nouveaux_installes: 1 }, 'discret')?.texte).toBe(
      '🎮 12 jeux de ton compte GOG ajoutés à ta ludothèque (1 déjà installé). Les autres s’installent avec ⬇ Installer.',
    );
    expect(annonceImportGog(rien, 'main')?.texte).toBe('Rien de nouveau : tes jeux GOG sont déjà dans ta ludothèque.');
  });
  it('juste après la connexion : « ✅ Connecté à GOG » d’abord ; pas connecté : nommer le bouton affiché', () => {
    expect(annonceImportGog({ ...rien, nouveaux: 1 }, 'connexion')?.texte).toBe('✅ Connecté à GOG. 🎮 1 jeu de ton compte GOG ajouté à ta ludothèque. Il s’installe avec ⬇ Installer.');
    expect(annonceImportGog(rien, 'connexion')?.texte).toBe('✅ Connecté à GOG. Rien de nouveau : tes jeux GOG sont déjà dans ta ludothèque.');
    expect(annonceImportGog(expire, 'connexion')).toEqual({ texte: messageGogPasConnecte(false), ton: 'alerte' });
    // « Se connecter » pour un compte jamais connu, « Me reconnecter » pour un compte déjà connu (le bouton du bloc).
    expect(annonceImportGog(expire, 'connexion', '', false)?.texte).toContain('« 🔑 Se connecter à GOG »');
    expect(annonceImportGog(expire, 'connexion', '', true)?.texte).toContain('« 🔑 Me reconnecter à GOG »');
  });
  it('sans GOG Galaxy sur ce PC : jamais « ⬇ Installer » promis', () => {
    const sans = { ...rien, galaxy: false };
    expect(annonceImportGog({ ...sans, nouveaux: 12 }, 'discret')?.texte).toBe('🎮 12 jeux de ton compte GOG ajoutés à ta ludothèque. Installe GOG Galaxy pour les installer.');
    expect(annonceImportGog({ ...sans, nouveaux: 1 }, 'connexion')?.texte).toBe(
      '✅ Connecté à GOG. 🎮 1 jeu de ton compte GOG ajouté à ta ludothèque. Installe GOG Galaxy pour l’installer.',
    );
    expect(annonceImportGog({ ...sans, nouveaux: 3, nouveaux_installes: 1 }, 'main')?.texte).toBe(
      '🎮 3 jeux de ton compte GOG ajoutés à ta ludothèque (1 déjà installé). Installe GOG Galaxy pour les installer.',
    );
    expect(sansGalaxy('Rien de nouveau : tes jeux GOG sont déjà dans ta ludothèque.')).toBe('Rien de nouveau : tes jeux GOG sont déjà dans ta ludothèque.');
  });
  it('connexion expirée : dite en alerte ; « au bureau » depuis Taodbox', () => {
    expect(MESSAGE_GOG_EXPIRE).toBe('🔑 Ta connexion à GOG a expiré : dans 🛒 Boutiques, reconnecte-toi.');
    for (const m of ['main', 'discret'] as const) expect(annonceImportGog(expire, m)).toEqual({ texte: MESSAGE_GOG_EXPIRE, ton: 'alerte' });
    expect(annonceImportGog(expire, 'discret', '/taodbox')?.texte).toBe('🔑 Ta connexion à GOG a expiré : au bureau, dans 🛒 Boutiques, reconnecte-toi.');
  });
  it('« expirée » une fois par jour et par profil, sous une clé à GOG (jamais celle de Steam)', () => {
    const m = new Map<string, string>();
    const s = { getItem: (k: string) => m.get(k) ?? null, setItem: (k: string, v: string) => void m.set(k, v) };
    expect(direGogExpireAujourdhui('seb', '2026-11-21', s)).toBe(true);
    expect(direGogExpireAujourdhui('seb', '2026-11-21', s)).toBe(false);
    expect([...m.keys()]).toEqual(['frogtend.gogExpire.seb']);
    expect(direGogExpireAujourdhui('lea', '2026-11-21', s)).toBe(true);
    expect(direGogExpireAujourdhui('seb', '2026-11-22', s)).toBe(true);
  });
  it('une fenêtre de GOG fermée ne vaut que pour le profil qui l’a ouverte, s’il est toujours ouvert', () => {
    expect(fermeturePourMoi({ boutique: 'gog', profil: 'seb' }, 'seb')).toBe(true);
    expect(fermeturePourMoi({ boutique: 'prime', profil: 'seb' }, 'seb')).toBe(false);
    expect(fermeturePourMoi({ boutique: 'gog', profil: 'seb' }, 'lea')).toBe(false);
    expect(fermeturePourMoi({ boutique: 'gog', profil: 'seb' }, null)).toBe(false);
    expect(fermeturePourMoi(null, 'seb')).toBe(false);
  });
});

describe('lecture des jeux du compte GOG', () => {
  beforeEach(() => {
    etatFaux.profilOuvert = { id: 'seb' };
    etatFaux.profil.gratuits.connectes.gog = true;
    partieFausse.enJeu = null;
    gratuitsFaux.boutique = null;
    gratuitsFaux.lectureGog = false;
    gratuitsFaux.gogDeconnecte = false;
    lectureVue.length = 0;
    etatGog = { connu: true, nb_jeux: 3, maj_le: null };
    gog.etat = null;
    gog.erreur = null;
    gog.enCours = false;
    gog.profilImport = null;
    gog.expire = false;
    gog.erreurProfil = null;
    try {
      localStorage.clear();
    } catch {
      // Rien.
    }
    for (const f of [importer, toast, fermerToast, rafraichir, noter]) f.mockClear();
  });

  it('à l’ouverture : discret sans nouveauté, la grille relue, Taodbox prévenu', async () => {
    const avant = gog.imports;
    await gogAuDemarrage('seb');
    expect(importer).toHaveBeenCalledTimes(1);
    expect(toast).not.toHaveBeenCalled();
    expect(rafraichir).toHaveBeenCalledTimes(1);
    expect(gog.imports).toBe(avant + 1);
    expect(noter).toHaveBeenCalledWith('gog', 'connecte');
  });

  it('à l’ouverture : seulement si connecté à GOG (ou déjà lu), jamais pendant une partie', async () => {
    etatFaux.profil.gratuits.connectes.gog = false;
    etatGog = { connu: false, nb_jeux: 0, maj_le: null };
    await gogAuDemarrage('seb');
    expect(importer).not.toHaveBeenCalled();
    // Connecté pour le jeu offert seulement (jamais lu) : la connexion vaut accord, la lecture se fait.
    etatFaux.profil.gratuits.connectes.gog = true;
    await gogAuDemarrage('seb');
    expect(importer).toHaveBeenCalledTimes(1);
    partieFausse.enJeu = 110;
    await gogAuDemarrage('seb');
    expect(importer).toHaveBeenCalledTimes(1);
  });

  it('à l’ouverture : les nouveaux jeux sont dits ; une erreur reste dans la ligne d’état, sans message', async () => {
    importer.mockResolvedValueOnce({ ...rien, nouveaux: 3 });
    await gogAuDemarrage('seb');
    expect(String(toast.mock.calls[0][0])).toContain('3 jeux de ton compte GOG');
    toast.mockClear();
    importer.mockRejectedValueOnce({ sorte: 'reseau', motif: 'x' });
    await gogAuDemarrage('seb');
    expect(toast).not.toHaveBeenCalled();
    expect(gog.erreur).toBe('Mise à jour de tes jeux GOG impossible : GOG ne répond pas');
  });

  it('connexion expirée : dite une fois par jour à l’ouverture, toujours à la main ; l’état de connexion passe à faux', async () => {
    importer.mockResolvedValue(expire);
    await gogAuDemarrage('seb');
    expect(toast).toHaveBeenCalledWith(MESSAGE_GOG_EXPIRE, 'alerte');
    expect(connexionGogExpiree()).toBe(true);
    expect(etatFaux.profil.gratuits.connectes.gog).toBe(false);
    expect(rafraichir).not.toHaveBeenCalled();
    // Le même jour : la ligne d'état seulement (la lecture se refait : le compte avait déjà été lu).
    toast.mockClear();
    await gogAuDemarrage('seb');
    expect(importer).toHaveBeenCalledTimes(2);
    expect(toast).not.toHaveBeenCalled();
    expect(connexionGogExpiree()).toBe(true);
    await mettreAJourGog('main');
    expect(toast).toHaveBeenCalledWith(MESSAGE_GOG_EXPIRE, 'alerte');
    // Un autre profil : l'expiration de Seb ne le concerne pas.
    etatFaux.profilOuvert = { id: 'lea' };
    expect(connexionGogExpiree()).toBe(false);
    importer.mockReset();
    importer.mockResolvedValue(rien);
  });

  it('juste après la connexion : attente, puis « ✅ Connecté à GOG » ; fenêtre fermée sans connexion : pas « expirée »', async () => {
    etatFaux.profil.gratuits.connectes.gog = false;
    etatGog = { connu: false, nb_jeux: 0, maj_le: null };
    gog.etat = etatGog;
    importer.mockResolvedValueOnce(expire);
    expect(await mettreAJourGog('connexion')).toBe(false);
    expect(toast.mock.calls[0][0]).toBe(MESSAGE_GOG_ATTENTE);
    expect(toast).toHaveBeenLastCalledWith(messageGogPasConnecte(false), 'alerte');
    expect(connexionGogExpiree()).toBe(false);
    toast.mockClear();
    expect(await mettreAJourGog('connexion')).toBe(true);
    expect(String(toast.mock.calls[1][0])).toMatch(/^✅ Connecté à GOG\./);
    expect(etatFaux.profil.gratuits.connectes.gog).toBe(true);
  });

  it('un compte connu, vu déconnecté juste après la connexion : « expirée », et le message nomme « Me reconnecter »', async () => {
    importer.mockResolvedValueOnce(expire);
    await mettreAJourGog('connexion');
    expect(toast).toHaveBeenLastCalledWith(messageGogPasConnecte(true), 'alerte');
    expect(connexionGogExpiree()).toBe(true);
  });

  it('jamais pendant la récupération du jeu offert de GOG ; juste après une connexion, attend sa fin', async () => {
    gratuitsFaux.boutique = 'gog';
    expect(gogOccupe()).toBe(true);
    expect(await mettreAJourGog('discret')).toBe(false);
    expect(await mettreAJourGog('main')).toBe(false);
    expect(importer).not.toHaveBeenCalled();
    expect(String(toast.mock.calls[0][0])).toContain('jeu offert');
    // Mode connexion : la lecture attend la fin de la récupération, puis se fait (une première connexion importe).
    vi.useFakeTimers();
    try {
      const p = mettreAJourGog('connexion');
      await vi.advanceTimersByTimeAsync(1200);
      expect(importer).not.toHaveBeenCalled();
      gratuitsFaux.boutique = null;
      await vi.advanceTimersByTimeAsync(600);
      expect(await p).toBe(true);
    } finally {
      vi.useRealTimers();
    }
    expect(importer).toHaveBeenCalledTimes(1);
  });

  it('la récupération du jeu offert sait qu’une lecture tourne (drapeau partagé)', async () => {
    importer.mockImplementationOnce(async () => {
      lectureVue.push(gratuitsFaux.lectureGog);
      return rien;
    });
    await mettreAJourGog('main');
    expect(lectureVue).toEqual([true]);
    expect(gratuitsFaux.lectureGog).toBe(false);
  });

  it('au démarrage, la récupération du jeu offert a vu le compte déconnecté : « expirée », sans rien redemander à GOG', async () => {
    gratuitsFaux.gogDeconnecte = true;
    etatFaux.profil.gratuits.connectes.gog = false;
    await gogAuDemarrage('seb');
    expect(importer).not.toHaveBeenCalled();
    expect(toast).toHaveBeenCalledWith(MESSAGE_GOG_EXPIRE, 'alerte');
    expect(connexionGogExpiree()).toBe(true);
  });

  it('un démarrage en retard (autre profil ouvert) ne touche à rien du nouveau profil', async () => {
    gog.erreurProfil = 'lea';
    gog.erreur = 'erreur de Léa';
    etatFaux.profilOuvert = { id: 'lea' };
    await gogAuDemarrage('seb');
    expect(importer).not.toHaveBeenCalled();
    expect(gog.erreur).toBe('erreur de Léa');
  });

  it('une lecture demandée pendant une autre : la discrète la rejoint, celle à la main attend puis relance', async () => {
    let finir: (r: ImportGog) => void = () => {};
    importer.mockImplementationOnce(() => new Promise<ImportGog>((r) => (finir = r)));
    const premier = mettreAJourGog('discret');
    await Promise.resolve();
    expect(importGogEnCours()).toBe(true);
    const rejoint = mettreAJourGog('discret');
    const aLaMain = mettreAJourGog('main');
    finir(rien);
    expect(await premier).toBe(true);
    expect(await rejoint).toBe(true);
    expect(await aLaMain).toBe(true);
    expect(importer).toHaveBeenCalledTimes(2);
  });

  it('profil changé pendant l’attente : silence total, rien de noté', async () => {
    importer.mockImplementationOnce(async () => {
      etatFaux.profilOuvert = { id: 'lea' };
      return { ...expire, nouveaux: 5 };
    });
    await mettreAJourGog('main');
    expect(toast).toHaveBeenCalledTimes(1);
    expect(fermerToast).toHaveBeenCalledWith(7);
    expect(noter).not.toHaveBeenCalled();
    expect(gog.erreur).toBeNull();
    expect(rafraichir).not.toHaveBeenCalled();
  });

  it('lot C5 : la page de connexion façon Galaxy ne peut s’afficher que juste après « Se connecter à GOG »', async () => {
    montres.length = 0;
    await mettreAJourGog('discret');
    await mettreAJourGog('main');
    await mettreAJourGog('connexion');
    expect(montres).toEqual([false, false, true]);
  });

  it('lot C5 : jamais la page de connexion pendant une partie ; « liste partielle » dite sans alarmer', async () => {
    montres.length = 0;
    partieFausse.enJeu = 110;
    await mettreAJourGog('connexion');
    expect(montres).toEqual([false]);
    partieFausse.enJeu = null;
    importer.mockResolvedValueOnce({ ...rien, bibliotheque_galaxy: false });
    await mettreAJourGog('discret');
    expect(listeGogPartielle()).toBe(true);
    expect(toast).not.toHaveBeenCalledWith(expect.stringContaining('partielle'), expect.anything());
    await mettreAJourGog('discret');
    expect(listeGogPartielle()).toBe(false);
  });

  it('à la main : un message d’attente qui reste, refermé, puis « Rien de nouveau »', async () => {
    await mettreAJourGog('main');
    expect(toast.mock.calls[0][2]).toEqual({ jusquaFermeture: true });
    expect(fermerToast).toHaveBeenCalledWith(7);
    expect(String(toast.mock.calls[1][0])).toContain('Rien de nouveau');
    expect(gog.enCours).toBe(false);
  });
});
