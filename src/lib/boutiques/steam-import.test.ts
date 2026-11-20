// Lot C2 (Seb 20/11) : l'import des jeux Steam POSSÉDÉS du compte connecté par la page — à l'ouverture de Frogtend
// (discret), à la main (message d'attente qui reste), jamais pendant une partie, silence si le profil a changé.
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { EtatSteam, ImportSteam } from '$lib/api';

const etatFaux = { profilOuvert: { id: 'seb' } as { id: string } | null };
const partieFausse = { enJeu: null as number | null };
const base: EtatSteam = { compte: '76561197960287930', cle_enregistree: false, nb_jeux: 0, maj_le: null, steam_installe: true, connecte: true, cle_autre_compte: false };
let etatSteam: EtatSteam = { ...base };
const importer = vi.fn(async (): Promise<ImportSteam> => ({ reconnecter: false, total: 3, nouveaux: 0, nouveaux_installes: 0 }));
const importerCle = vi.fn(async () => []);
const toast = vi.fn((..._a: unknown[]) => 7);
const fermerToast = vi.fn();
const demander = vi.fn();
const rafraichir = vi.fn(async () => {});

vi.mock('$lib/api', () => ({
  api: {
    steamEtat: async () => etatSteam,
    steamJeux: async () => [],
    steamImporterConnecte: () => importer(),
    steamImporter: () => importerCle(),
  },
}));
vi.mock('$lib/dialogues/fenetres.svelte', () => ({
  toast: (...a: unknown[]) => toast(...a),
  fermerToast: (id: number) => fermerToast(id),
  demander: (...a: unknown[]) => demander(...a),
  informer: async () => {},
  confirmer: async () => false,
}));
vi.mock('$lib/dialogues/messages', () => ({ motifDuRefus: () => 'Steam ne répond pas' }));
vi.mock('$lib/etat.svelte', () => ({ etat: etatFaux }));
vi.mock('$lib/ludotheque/jeu.svelte', () => ({ partie: partieFausse }));
vi.mock('./boutiques-pc.svelte', () => ({ rafraichirLudotheque: () => rafraichir() }));

const { connexionSteamExpiree, importerSteam, importSteamEnCours, mettreAJourSteam, steam, steamAuDemarrage, MESSAGE_STEAM_EXPIRE } = await import('./steam.svelte');

describe('import Steam par la connexion', () => {
  beforeEach(() => {
    etatFaux.profilOuvert = { id: 'seb' };
    partieFausse.enJeu = null;
    etatSteam = { ...base };
    steam.erreur = null;
    steam.enCours = false;
    steam.profilImport = null;
    steam.expire = false;
    steam.erreurProfil = null;
    try {
      localStorage.clear();
    } catch {
      // Rien.
    }
    for (const f of [importer, importerCle, toast, fermerToast, demander, rafraichir]) f.mockClear();
  });

  it('à l’ouverture : discret sans nouveauté, la grille relue quand même', async () => {
    await steamAuDemarrage();
    expect(importer).toHaveBeenCalledTimes(1);
    expect(toast).not.toHaveBeenCalled();
    expect(rafraichir).toHaveBeenCalledTimes(1);
    expect(steam.erreur).toBeNull();
  });

  it('à l’ouverture : seulement si le profil est connecté à Steam, et jamais pendant une partie', async () => {
    etatSteam = { ...base, connecte: false };
    await steamAuDemarrage();
    partieFausse.enJeu = 110;
    etatSteam = { ...base };
    await steamAuDemarrage();
    expect(importer).not.toHaveBeenCalled();
  });

  it('à l’ouverture : les nouveaux jeux sont dits ; une erreur reste dans la ligne d’état, sans message', async () => {
    importer.mockResolvedValueOnce({ reconnecter: false, total: 3, nouveaux: 3, nouveaux_installes: 0 });
    await steamAuDemarrage();
    expect(String(toast.mock.calls[0][0])).toContain('3 jeux de ton compte Steam');
    toast.mockClear();
    importer.mockRejectedValueOnce({ sorte: 'reseau', motif: 'x' });
    await steamAuDemarrage();
    expect(toast).not.toHaveBeenCalled();
    expect(steam.erreur).toBe('Mise à jour de tes jeux Steam impossible : Steam ne répond pas');
  });

  it('connexion expirée : dite en alerte et gardée dans la ligne d’état ; rien n’est relu', async () => {
    importer.mockResolvedValueOnce({ reconnecter: true, total: 0, nouveaux: 0, nouveaux_installes: 0 });
    expect(await mettreAJourSteam('discret')).toBe(false);
    expect(toast).toHaveBeenCalledWith(MESSAGE_STEAM_EXPIRE, 'alerte');
    expect(steam.erreur).toContain('expiré');
    expect(connexionSteamExpiree()).toBe(true);
    expect(rafraichir).not.toHaveBeenCalled();
    // Le même jour, à la prochaine ouverture : la ligne d'état seulement, pas de nouveau message.
    toast.mockClear();
    importer.mockResolvedValueOnce({ reconnecter: true, total: 0, nouveaux: 0, nouveaux_installes: 0 });
    await steamAuDemarrage();
    expect(toast).not.toHaveBeenCalled();
    expect(connexionSteamExpiree()).toBe(true);
    // À la main : toujours dit.
    importer.mockResolvedValueOnce({ reconnecter: true, total: 0, nouveaux: 0, nouveaux_installes: 0 });
    await mettreAJourSteam('main');
    expect(toast).toHaveBeenCalledWith(MESSAGE_STEAM_EXPIRE, 'alerte');
    // Un autre profil ouvert : l'expiration de Seb ne le concerne pas.
    etatFaux.profilOuvert = { id: 'lea' };
    expect(connexionSteamExpiree()).toBe(false);
  });

  it('un import demandé pendant un autre : le discret le rejoint, celui à la main attend puis relance', async () => {
    let finir: (r: ImportSteam) => void = () => {};
    importer.mockImplementationOnce(() => new Promise<ImportSteam>((r) => (finir = r)));
    const premier = mettreAJourSteam('discret');
    await Promise.resolve();
    expect(importSteamEnCours()).toBe(true);
    const rejoint = mettreAJourSteam('discret');
    const aLaMain = mettreAJourSteam('main');
    finir({ reconnecter: false, total: 1, nouveaux: 0, nouveaux_installes: 0 });
    expect(await premier).toBe(true);
    expect(await rejoint).toBe(true);
    expect(await aLaMain).toBe(true);
    expect(importer).toHaveBeenCalledTimes(2);
    expect(steam.imports).toBeGreaterThanOrEqual(2);
  });

  it('l’import du profil A ne bloque pas les boutons du profil B', async () => {
    let finir: (r: ImportSteam) => void = () => {};
    importer.mockImplementationOnce(() => new Promise<ImportSteam>((r) => (finir = r)));
    const a = mettreAJourSteam('discret');
    await Promise.resolve();
    etatFaux.profilOuvert = { id: 'lea' };
    expect(importSteamEnCours()).toBe(false);
    finir({ reconnecter: false, total: 1, nouveaux: 0, nouveaux_installes: 0 });
    expect(await a).toBe(false);
    expect(steam.enCours).toBe(false);
  });

  it('juste après la connexion : un seul message « ✅ Connecté à Steam. ⏳ Lecture… »', async () => {
    await mettreAJourSteam('connexion');
    expect(toast.mock.calls[0][0]).toBe('✅ Connecté à Steam. ⏳ Lecture de tes jeux…');
  });

  it('une partie commencée avant l’ouverture de la fenêtre cachée : pas d’import discret', async () => {
    partieFausse.enJeu = 110;
    expect(await mettreAJourSteam('discret')).toBe(false);
    expect(importer).not.toHaveBeenCalled();
  });

  it('profil changé pendant l’attente : silence total', async () => {
    importer.mockImplementationOnce(async () => {
      etatFaux.profilOuvert = { id: 'lea' };
      return { reconnecter: true, total: 0, nouveaux: 5, nouveaux_installes: 0 };
    });
    await mettreAJourSteam('main');
    // Seul le message d'attente a été montré (et refermé).
    expect(toast).toHaveBeenCalledTimes(1);
    expect(fermerToast).toHaveBeenCalledWith(7);
    expect(steam.erreur).toBeNull();
    expect(rafraichir).not.toHaveBeenCalled();
  });

  it('à la main : un message d’attente qui reste, refermé, puis « Rien de nouveau »', async () => {
    await mettreAJourSteam('main');
    expect(importer).toHaveBeenCalledTimes(1);
    expect(toast.mock.calls[0][2]).toEqual({ jusquaFermeture: true });
    expect(fermerToast).toHaveBeenCalledWith(7);
    expect(String(toast.mock.calls[1][0])).toContain('Rien de nouveau');
    expect(steam.enCours).toBe(false);
  });

  it('le bouton du Secours, connecté : l’import sans clé, la clé jamais demandée', async () => {
    await importerSteam();
    expect(importer).toHaveBeenCalledTimes(1);
    expect(importerCle).not.toHaveBeenCalled();
    expect(demander).not.toHaveBeenCalled();
  });
});
