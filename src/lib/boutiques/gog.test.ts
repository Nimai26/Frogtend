// Lot C4a (Seb 21/11) : la lecture des jeux du compte GOG connecté — à l'ouverture de Frogtend (discret), à la main
// (message d'attente qui reste), juste après la connexion, jamais pendant une partie ni pendant la récupération du jeu
// offert, silence si le profil a changé ; un seul état de connexion GOG pour le jeu offert et les jeux du compte.
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { AjoutGog, CompteGogEtat, EtatGog, ImportGog, OubliGog } from '$lib/api';

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
const reconnexions: boolean[] = [];
const toast = vi.fn((..._a: unknown[]) => 7);
const fermerToast = vi.fn();
const rafraichir = vi.fn(async () => {});
const noter = vi.fn(async (_b: string, e: string) => {
  etatFaux.profil.gratuits.connectes.gog = e !== 'connexion';
});

const ajouter = vi.fn(async (): Promise<AjoutGog> => ({ fait: true, cle: '7700', nom: 'Seb', numero: 2, premier: false, reconnexion: false, jeux: 197, partiel: false, nouveaux: 197, nouveaux_installes: 2, galaxy: true }));
const oublier = vi.fn(async (_cle: string): Promise<OubliGog> => ({ profil: false, secret_oublie: true, navigateur_deconnecte: false }));
const confirmer = vi.fn(async (..._a: unknown[]) => true);
vi.mock('$lib/api', () => ({
  api: {
    gogEtat: async () => etatGog,
    gogImporterConnecte: (montrer?: boolean, reconnexion?: boolean) => {
      montres.push(!!montrer);
      reconnexions.push(!!reconnexion);
      return importer();
    },
    gogAjouterCompte: () => ajouter(),
    gogOublierCompte: (cle: string) => oublier(cle),
  },
}));
vi.mock('$lib/dialogues/fenetres.svelte', () => ({ toast: (...a: unknown[]) => toast(...a), fermerToast: (id: number) => fermerToast(id), confirmer: (...a: unknown[]) => confirmer(...a) }));
vi.mock('$lib/dialogues/messages', () => ({ motifDuRefus: () => 'GOG ne répond pas' }));
vi.mock('$lib/etat.svelte', () => ({ etat: etatFaux }));
vi.mock('$lib/ludotheque/jeu.svelte', () => ({ partie: partieFausse }));
vi.mock('./boutiques-pc.svelte', () => ({ rafraichirLudotheque: () => rafraichir() }));
vi.mock('./gratuits.svelte', () => ({ gratuits: gratuitsFaux, noterConnexion: (b: string, e: string) => noter(b, e) }));

const {
  ajouterCompteGog,
  annonceConclusionGog,
  comptesGogAffiches,
  messageAttenteAjoutGog,
  messageGogComptesExpires,
  confirmationOubliGog,
  ligneCompteGog,
  messageAjoutGog,
  messageGogCompteExpire,
  messageGogFenetre,
  messageOubliGog,
  nomCompteGog,
  oublierCompteGog,
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

  it('lot C5 : la page de connexion façon Galaxy peut s’afficher quand la personne est là (connexion, à la main), jamais en discret', async () => {
    montres.length = 0;
    await mettreAJourGog('discret');
    await mettreAJourGog('main');
    await mettreAJourGog('connexion');
    expect(montres).toEqual([false, true, true]);
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

// Lot C4b (Seb 22/11 : « les deux comptes ») : plusieurs comptes GOG par profil.
const profilGog: CompteGogEtat = { cle: '4200', nom: '', profil: true, nb_jeux: 88, maj_le: '2026-11-22T10:00:00', reconnecter: false };
const ajouteGog: CompteGogEtat = { cle: '7700', nom: 'Seb', profil: false, nb_jeux: 197, maj_le: null, reconnecter: false };

describe('plusieurs comptes GOG (purs)', () => {
  it('le nom d’un compte : son pseudo, sinon « Compte GOG n », jamais son numéro', () => {
    expect(nomCompteGog(ajouteGog, 1)).toBe('Seb');
    expect(nomCompteGog(profilGog, 0)).toBe('Compte GOG 1');
    expect(nomCompteGog({ nom: '   ' }, 2)).toBe('Compte GOG 3');
    expect(ligneCompteGog(profilGog, 0)).toBe('Compte GOG 1 — 88 jeux, mis à jour le 22 novembre');
    expect(ligneCompteGog({ ...ajouteGog, nb_jeux: 1 }, 1)).toBe('Seb — 1 jeu');
    expect(ligneCompteGog(profilGog, 0)).not.toContain('4200');
  });
  it('la confirmation de « Oublier » : le compte du navigateur dit aussi la déconnexion de GOG dans Frogtend', () => {
    expect(confirmationOubliGog('Seb', false)).toEqual({
      titre: 'Oublier le compte GOG « Seb » ?',
      message: 'Frogtend ne lira plus les jeux de ce compte et oublie sa connexion. Tes jeux installés ne bougent pas. Tu pourras le rajouter plus tard.',
    });
    expect(confirmationOubliGog('Compte GOG 1', true).message).toContain('Frogtend te déconnecte aussi de GOG dans Frogtend (pas dans GOG Galaxy)');
  });
  it('la fin de « Oublier » : jamais dite sûre sans preuve', () => {
    expect(messageOubliGog({ profil: false, secret_oublie: true, navigateur_deconnecte: false }, 'Seb')).toEqual({ texte: '✅ Compte GOG « Seb » oublié. Ses jeux déjà dans ta ludothèque y restent.' });
    expect(messageOubliGog({ profil: false, secret_oublie: false, navigateur_deconnecte: false }, 'Seb').ton).toBe('alerte');
    expect(messageOubliGog({ profil: true, secret_oublie: true, navigateur_deconnecte: false }, 'X').texte).toContain('Frogtend ne relit plus ses jeux');
    expect(messageOubliGog({ profil: true, secret_oublie: true, navigateur_deconnecte: true }, 'X').ton).toBeUndefined();
  });
  it('la fin d’un ajout : le nom du compte, ses jeux nouveaux (sans promettre ⬇ Installer sans Galaxy), une liste partielle dite', () => {
    const r: AjoutGog = { fait: true, cle: '7700', nom: 'Seb', numero: 2, premier: false, reconnexion: false, jeux: 197, partiel: false, nouveaux: 197, nouveaux_installes: 2, galaxy: true };
    expect(messageAjoutGog(r).texte).toBe('✅ Compte GOG « Seb » ajouté. 🎮 197 jeux de ton compte GOG ajoutés à ta ludothèque (2 déjà installés). Les autres s’installent avec ⬇ Installer.');
    expect(messageAjoutGog({ ...r, galaxy: false }).texte).toContain('Installe GOG Galaxy pour les installer.');
    expect(messageAjoutGog({ ...r, nom: '', nouveaux: 0, reconnexion: true }).texte).toBe('✅ Compte GOG « Compte GOG 2 » reconnecté. Rien de nouveau : ses jeux sont déjà dans ta ludothèque.');
    expect(messageAjoutGog({ ...r, partiel: true })).toMatchObject({ ton: 'alerte' });
  });
  it('un compte ajouté expiré : son nom ; « au bureau » depuis Taodbox ; la fenêtre de connexion nomme le compte attendu', () => {
    expect(messageGogCompteExpire('Seb', '/taodbox')).toBe('🔑 La connexion de ton compte GOG « Seb » a expiré : au bureau, dans 🛒 Boutiques, reconnecte-le (« ➕ Ajouter un compte GOG »).');
    expect(messageGogCompteExpire('Seb', '/boutiques')).toContain('reconnecte-le avec « ➕ Ajouter un compte GOG »');
    expect(messageGogFenetre('Seb')).toBe('🔑 Connecte-toi à GOG avec ton compte « Seb » dans sa fenêtre (elle vient de s’ouvrir) : tes jeux arriveront ensuite.');
    expect(messageGogFenetre('')).toBe('🔑 Connecte-toi à GOG dans sa fenêtre (elle vient de s’ouvrir) : tes jeux arriveront ensuite.');
    expect(messageGogFenetre(42)).not.toContain('42');
  });
  it('« une fois par jour » : une clé par compte ajouté, celle du compte du navigateur inchangée', () => {
    const m = new Map<string, string>();
    const s = { getItem: (k: string) => m.get(k) ?? null, setItem: (k: string, v: string) => void m.set(k, v) };
    expect(direGogExpireAujourdhui('seb', '2026-11-22', s, '7700')).toBe(true);
    expect(direGogExpireAujourdhui('seb', '2026-11-22', s, '7700')).toBe(false);
    expect(direGogExpireAujourdhui('seb', '2026-11-22', s)).toBe(true);
    expect([...m.keys()].sort()).toEqual(['frogtend.gogExpire.seb', 'frogtend.gogExpire.seb.7700']);
  });
});

describe('plusieurs comptes GOG (lecture, ajout, oubli)', () => {
  beforeEach(() => {
    etatFaux.profilOuvert = { id: 'seb' };
    etatFaux.profil.gratuits.connectes.gog = true;
    partieFausse.enJeu = null;
    gratuitsFaux.boutique = null;
    etatGog = { connu: true, nb_jeux: 285, maj_le: null, comptes: [profilGog, ajouteGog] };
    gog.etat = etatGog;
    gog.comptes = {};
    gog.erreurProfil = null;
    gog.ajoutEnCours = false;
    gog.oubliEnCours = null;
    try {
      localStorage.clear();
    } catch {
      // Rien.
    }
    for (const f of [importer, toast, fermerToast, rafraichir, noter, ajouter, oublier, confirmer]) f.mockClear();
  });

  it('une seule annonce pour le total ; la connexion perdue d’un compte ajouté est dite avec son nom', async () => {
    importer.mockResolvedValueOnce({ ...rien, nouveaux: 3, comptes: [{ cle: '7700', nom: 'Seb', reconnecter: true, partiel: false }] });
    await mettreAJourGog('main');
    const textes = toast.mock.calls.map((c) => String(c[0]));
    expect(textes.filter((t) => t.startsWith('🎮'))).toHaveLength(1);
    expect(textes.some((t) => t.includes('« Seb » a expiré'))).toBe(true);
    expect(gog.comptes['7700']).toEqual({ expire: true, partiel: false });
  });
  it('à l’ouverture : la connexion perdue d’un compte ajouté une fois par jour, jamais pendant une partie', async () => {
    const r = { ...rien, comptes: [{ cle: '7700', nom: 'Seb', reconnecter: true, partiel: false }] };
    importer.mockResolvedValue(r);
    partieFausse.enJeu = 110;
    await mettreAJourGog('discret');
    expect(toast).not.toHaveBeenCalled();
    partieFausse.enJeu = null;
    // (La lecture discrète pendant une partie n'est même pas lancée ; à la sortie, une fois dans la journée.)
    await mettreAJourGog('discret');
    await mettreAJourGog('discret');
    expect(toast.mock.calls.filter((c) => String(c[0]).includes('« Seb »'))).toHaveLength(1);
    importer.mockReset();
    importer.mockImplementation(async () => rien);
  });
  it('un profil qui n’a QUE des comptes ajoutés : relu à l’ouverture, jamais « ta connexion à GOG a expiré »', async () => {
    etatFaux.profil.gratuits.connectes.gog = false;
    etatGog = { connu: false, nb_jeux: 197, maj_le: null, comptes: [ajouteGog] };
    importer.mockResolvedValueOnce({ ...expire, comptes: [{ cle: '7700', nom: 'Seb', reconnecter: false, partiel: false }], nouveaux: 1 });
    await gogAuDemarrage('seb');
    expect(importer).toHaveBeenCalledTimes(1);
    expect(gog.expire).toBe(false);
    expect(toast.mock.calls.map((c) => String(c[0])).join(' ')).not.toContain('a expiré');
    expect(rafraichir).toHaveBeenCalledTimes(1);
  });
  it('la lecture du compte du navigateur en échec, un compte ajouté lu : l’erreur reste dans la ligne d’état, la ludothèque est relue', async () => {
    importer.mockResolvedValueOnce({ ...rien, erreur: 'GOG n’a pas répondu', comptes: [{ cle: '7700', nom: 'Seb', reconnecter: false, partiel: false }] });
    await mettreAJourGog('discret');
    expect(gog.erreur).toContain('GOG n’a pas répondu');
    expect(noter).not.toHaveBeenCalled();
    expect(rafraichir).toHaveBeenCalledTimes(1);
  });
  it('« ➕ Ajouter un compte GOG » : attente, message nommé, ludothèque et Taodbox relus ; fenêtre fermée : rien', async () => {
    const avant = gog.imports;
    expect(await ajouterCompteGog()).toBe(true);
    expect(toast.mock.calls[0][2]).toEqual({ jusquaFermeture: true });
    expect(toast.mock.calls.some((c) => String(c[0]).startsWith('✅ Compte GOG « Seb » ajouté.'))).toBe(true);
    expect(gog.imports).toBe(avant + 1);
    expect(rafraichir).toHaveBeenCalledTimes(1);
    expect(gog.ajoutEnCours).toBe(false);
    ajouter.mockResolvedValueOnce({ fait: false, cle: '', nom: '', numero: 0, premier: true, reconnexion: false, jeux: 0, partiel: false, nouveaux: 0, nouveaux_installes: 0, galaxy: true });
    expect(await ajouterCompteGog()).toBe(false);
    expect(String(toast.mock.calls.at(-1)?.[0])).toBe('Fenêtre fermée : aucun compte GOG ajouté.');
  });
  it('un ajout refusé (compte déjà là) : le motif est dit ; jamais deux opérations à la fois', async () => {
    ajouter.mockRejectedValueOnce(new Error('Ce compte GOG est déjà dans Frogtend.'));
    expect(await ajouterCompteGog()).toBe(false);
    expect(toast).toHaveBeenLastCalledWith('Impossible d’ajouter ce compte GOG : GOG ne répond pas', 'erreur');
    gog.oubliEnCours = '7700';
    expect(await ajouterCompteGog()).toBe(false);
    expect(ajouter).toHaveBeenCalledTimes(1);
  });
  it('« Oublier » : confirmation d’abord ; non = rien ; oui = oublié, Taodbox prévenu', async () => {
    confirmer.mockResolvedValueOnce(false);
    expect(await oublierCompteGog(ajouteGog, 1)).toBe(false);
    expect(oublier).not.toHaveBeenCalled();
    const avant = gog.imports;
    expect(await oublierCompteGog(ajouteGog, 1)).toBe(true);
    expect(confirmer.mock.calls.at(-1)?.[0]).toBe('Oublier le compte GOG « Seb » ?');
    expect(confirmer.mock.calls.at(-1)?.[1]).toMatchObject({ libelleValider: 'Oublier', danger: true });
    expect(oublier).toHaveBeenCalledWith('7700');
    expect(noter).not.toHaveBeenCalled();
    expect(gog.imports).toBe(avant + 1);
    expect(gog.oubliEnCours).toBeNull();
  });
  it('« Oublier » le compte du navigateur : Frogtend n’est plus connecté à GOG', async () => {
    oublier.mockResolvedValueOnce({ profil: true, secret_oublie: true, navigateur_deconnecte: true });
    expect(await oublierCompteGog(profilGog, 0)).toBe(true);
    expect(noter).toHaveBeenCalledWith('gog', 'connexion');
    expect(String(toast.mock.calls.at(-1)?.[0])).toBe('✅ Compte GOG « Compte GOG 1 » oublié. Ses jeux déjà dans ta ludothèque y restent.');
  });
  it('« Oublier » en échec : le motif dit, avec ce qui a échoué', async () => {
    oublier.mockRejectedValueOnce(new Error('x'));
    expect(await oublierCompteGog(ajouteGog, 1)).toBe(false);
    expect(toast).toHaveBeenLastCalledWith('Impossible d’oublier ce compte GOG : GOG ne répond pas', 'erreur');
  });
  it('« ➕ Ajouter » pendant une partie : refus poli, aucune fenêtre', async () => {
    partieFausse.enJeu = 110;
    expect(await ajouterCompteGog()).toBe(false);
    expect(ajouter).not.toHaveBeenCalled();
    expect(String(toast.mock.calls.at(-1)?.[0])).toContain('Une partie est en cours');
    partieFausse.enJeu = null;
  });
  it('juste après la connexion, la lecture dit « reconnexion » au cœur ; jamais à la main ni en discret', async () => {
    reconnexions.length = 0;
    await mettreAJourGog('connexion');
    await mettreAJourGog('main');
    await mettreAJourGog('discret');
    expect(reconnexions).toEqual([true, false, false]);
  });
  it('les comptes affichés sont ceux du profil ouvert seulement', () => {
    gog.etatProfil = 'seb';
    expect(comptesGogAffiches()).toHaveLength(2);
    gog.etatProfil = 'lea';
    expect(comptesGogAffiches()).toEqual([]);
  });
});

describe('le message unique d’une lecture GOG (revue du lot C4b)', () => {
  const o = (enPartie = false, dire = true) => ({ chemin: '/boutiques', enPartie, places: new Map([['7700', 1]]), direAujourdhui: () => dire });
  const seb = { cle: '7700', nom: 'Seb', numero: 2, reconnecter: true, partiel: false };
  const lea = { cle: '8800', nom: '', numero: 3, reconnecter: true, partiel: false };
  it('le compte du navigateur en échec : les jeux nouveaux des comptes ajoutés sont dits DANS le même message', () => {
    const m = annonceConclusionGog({ ...rien, erreur: 'GOG n’a pas répondu', nouveaux: 5, lus: 1 }, 'main', true, o());
    expect(m?.texte).toBe('Mise à jour de ton compte GOG impossible : GOG n’a pas répondu 🎮 5 jeux de ton compte GOG ajoutés à ta ludothèque. Ils s’installent avec ⬇ Installer.');
    expect(m?.ton).toBe('alerte');
    const e = annonceConclusionGog({ ...expire, nouveaux: 1, lus: 1, galaxy: false }, 'main', true, o());
    expect(e?.texte).toContain('a expiré');
    expect(e?.texte).toContain('Installe GOG Galaxy pour l’installer.');
  });
  it('rien n’a été lu : jamais « Rien de nouveau », seulement les alertes', () => {
    expect(annonceConclusionGog({ ...rien, lus: 0, comptes: [seb] }, 'main', false, o())?.texte).toBe(messageGogComptesExpires(['Seb'], '/boutiques'));
    expect(annonceConclusionGog({ ...rien, lus: 0 }, 'main', false, o())).toBeNull();
    expect(annonceConclusionGog({ ...rien, lus: 1 }, 'main', true, o())?.texte).toContain('Rien de nouveau');
  });
  it('les comptes ajoutés expirés : une seule phrase, dans le même message que le reste', () => {
    const m = annonceConclusionGog({ ...rien, nouveaux: 2, lus: 1, comptes: [seb, lea] }, 'main', true, o());
    expect(m?.texte).toBe(
      '🎮 2 jeux de ton compte GOG ajoutés à ta ludothèque. Ils s’installent avec ⬇ Installer. 🔑 La connexion de tes comptes GOG « Seb », « Compte GOG 3 » a expiré : reconnecte-les avec « ➕ Ajouter un compte GOG ».',
    );
    expect(m?.ton).toBe('alerte');
  });
  it('à l’ouverture : jamais pendant une partie, les connexions perdues une fois par jour, jamais l’erreur', () => {
    expect(annonceConclusionGog({ ...rien, nouveaux: 2, lus: 1, comptes: [seb] }, 'discret', true, o(true))).toBeNull();
    expect(annonceConclusionGog({ ...rien, lus: 1, comptes: [seb] }, 'discret', true, o(false, false))).toBeNull();
    expect(annonceConclusionGog({ ...rien, lus: 1, erreur: 'x' }, 'discret', true, o())).toBeNull();
  });
  it('les messages purs : pluriel, Taodbox, premier compte', () => {
    expect(messageGogComptesExpires(['A', 'B'], '/taodbox')).toBe('🔑 La connexion de tes comptes GOG « A », « B » a expiré : au bureau, dans 🛒 Boutiques, reconnecte-les (« ➕ Ajouter un compte GOG »).');
    expect(messageAttenteAjoutGog(false)).toContain('Connecte-toi à ton compte GOG dans');
    expect(messageAttenteAjoutGog(true)).toContain('ton autre compte GOG');
    expect(nomCompteGog({ nom: '', numero: 3 }, 0)).toBe('Compte GOG 3');
    expect(messageAjoutGog({ fait: true, cle: '1', nom: '', numero: 4, premier: false, reconnexion: false, jeux: 0, partiel: false, nouveaux: 0, nouveaux_installes: 0, galaxy: true }).texte).toContain('« Compte GOG 4 »');
  });
});
