// Lot C4c-2 : « ⬇ Installer » un jeu GOG SANS GOG Galaxy — la fenêtre d'accord (le jeu en 1re case, ses DLC cochés
// d'office, la langue, le dossier, la question de Windows, la place), le suivi, la fin, les refus, le rattrapage d'une
// installation finie Frogtend fermé, et Galaxy en dernier secours seulement quand il peut aider.
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { BilanInstallationGog, PreparationGog } from '$lib/api';

let preparation: PreparationGog | Error;
let bilan: BilanInstallationGog | { sorte: string; motif: string };
let attendre = false;
let enAttente: ((b: BilanInstallationGog) => void) | null = null;
const api = {
  gogInstallationPreparer: vi.fn(async (_cle: string) => {
    if (preparation instanceof Error || 'motif' in preparation) throw preparation;
    return preparation;
  }),
  gogInstaller: vi.fn(async (_cle: string, _dlcs: string[], _suivi: number): Promise<BilanInstallationGog> => {
    if ('motif' in bilan) throw bilan;
    if (attendre) return new Promise((r) => (enAttente = r));
    return bilan;
  }),
  gogInstallationAnnuler: vi.fn(async (_cle: string) => true),
  gogAvertissementInstallation: vi.fn(async () => null),
  boutiqueOuvrir: vi.fn(async () => {}),
};
vi.mock('$lib/api', async (original) => ({ ...(await original<typeof import('$lib/api')>()), api }));

const fenetres = {
  cocher: vi.fn(async (..._a: unknown[]): Promise<string[] | null> => null),
  confirmer: vi.fn(async (..._a: unknown[]) => true),
  informer: vi.fn(async (..._a: unknown[]) => {}),
  toast: vi.fn((..._a: unknown[]) => 0),
};
vi.mock('$lib/dialogues/fenetres.svelte', () => fenetres);
let progresVus: ((p: { etape: string }) => void) | undefined;
vi.mock('$lib/mods/progres.svelte', () => ({
  avecProgres: async (_m: string, _a: boolean, travail: (s: number) => Promise<unknown>, _d: unknown, sur?: (p: { etape: string }) => void) => {
    progresVus = sur;
    return travail(7);
  },
}));
let debit: number | null = null;
const rechargerJeuxDuPc = vi.fn(async () => {});
vi.mock('$lib/ludotheque/telechargements.svelte', () => ({ debitConnu: () => debit, rechargerJeuxDuPc: () => rechargerJeuxDuPc() }));
const rechargerListe = vi.fn(async () => {});
vi.mock('$lib/ludotheque/ludotheque.svelte', () => ({ rechargerListe: () => rechargerListe(), rechargerPlateformes: async () => {} }));

const { installerGog, arreterInstallationGog, avertissementSortieGog, enInstallationGog, installGog, accordGog } = await import('./installer-gog.svelte');
const { finALire, messageAccordGog, messageFinGog, placeNecessaire, refusAccordGog, resumeAccordGog } = await import('./installation-gog');

const GO = 1024 ** 3;
function prep(o: Partial<PreparationGog> = {}): PreparationGog {
  return {
    titre: 'Cyberpunk 2077',
    en_francais: true,
    langue_installeur: 'en',
    dossier: 'E:\\Frogtend\\Jeux\\Cyberpunk 2077',
    jeu: { id: '1423049311', titre: 'Cyberpunk 2077', taille: 100 * GO },
    dlcs: [{ id: '1597316373', titre: 'REDmod', taille: 2 * GO }],
    libre: 500 * GO,
    deja_recus: 0,
    marge: GO,
    refus: null,
    galaxy: false,
    galaxy_peut_aider: false,
    rattrapage: false,
    ...o,
  };
}
const fin = (o: Partial<BilanInstallationGog> = {}): BilanInstallationGog => ({ pause: false, titre: 'Cyberpunk 2077', dlcs_installes: [], dlcs_echoues: [], installeurs_gardes: 0, dossier_protege: false, rattrape: false, ...o });

describe('l’accord d’une installation GOG (fonctions pures)', () => {
  it('dit la langue, le dossier, Windows, et au canapé le clavier ou la souris', () => {
    const m = messageAccordGog(prep(), false);
    expect(m).toContain('installeur officiel de GOG');
    expect(m).toContain('Le jeu contient le français.');
    expect(m).toContain('Dossier : E:\\Frogtend\\Jeux\\Cyberpunk 2077');
    expect(m).toContain('Windows va te demander une autorisation (droits administrateur) : réponds Oui.');
    expect(m).toContain('l’installeur est effacé');
    expect(m).toContain('Tu peux l’arrêter pendant le téléchargement');
    expect(m).not.toContain('clavier ou la souris');
    expect(messageAccordGog(prep(), true)).toContain('Il faudra un clavier ou la souris pour répondre Oui à Windows.');
    expect(messageAccordGog(prep({ langue_installeur: 'fr' }), false)).toContain('Le jeu est en français.');
    expect(messageAccordGog(prep({ en_francais: false }), false)).toContain('Le jeu est en anglais (GOG ne le propose pas en français).');
    expect(messageAccordGog(prep({ deja_recus: GO }), false)).toContain('Déjà téléchargé : 1');
  });

  it('la ligne de total : jamais « environ moins d’une minute », pas de durée sans débit ni sans case cochée', () => {
    expect(resumeAccordGog(prep(), ['1423049311'], null)).toBe('Environ 100 Go à télécharger.');
    expect(resumeAccordGog(prep(), ['1423049311', '1597316373'], 102 * GO)).toBe('Environ 102 Go à télécharger, moins d’une minute.');
    expect(resumeAccordGog(prep(), ['1423049311'], GO / 60)).toBe('Environ 100 Go à télécharger, environ 1 h 40.');
    expect(resumeAccordGog(prep(), [], 1000)).toBe('Rien de coché.');
  });

  it('refuse le jeu décoché et la place qui manque (2 × la taille + l’arrondi + la marge, moins le déjà reçu)', () => {
    expect(refusAccordGog(prep(), ['1597316373'])).toBe('Le jeu lui-même est nécessaire : garde-le coché.');
    expect(refusAccordGog(prep(), ['1423049311', '1597316373'])).toBeNull();
    expect(placeNecessaire(100, 0)).toBe(204);
    expect(placeNecessaire(100_000, 10, 4)).toBe(2 * 102_000 + 10 - 4);
    const r = refusAccordGog(prep({ libre: 150 * GO }), ['1423049311']);
    expect(r).toContain('Pas assez de place sur le disque');
    expect(r).toContain('il en reste 150 Go');
    expect(refusAccordGog(prep({ libre: null }), ['1423049311'])).toBeNull();
  });

  it('la fin : ✅ ▶ Jouer ; un DLC raté, la place gardée et un dossier protégé sont dits, dans une fenêtre', () => {
    expect(messageFinGog(fin({ titre: 'Dune' }))).toBe('✅ « Dune » est installé : ▶ Jouer.');
    expect(finALire(fin())).toBe(false);
    const b = fin({ titre: 'Dune', dlcs_echoues: [['Bonus', 'tu as répondu Non à Windows']], installeurs_gardes: 2 * GO, dossier_protege: true });
    const m = messageFinGog(b);
    expect(m).toContain('⚠ Le DLC « Bonus » ne s’est pas installé (tu as répondu Non à Windows) ; le jeu, lui, est bien installé.');
    expect(m).toContain('Ses installeurs sont gardés (2 Go)');
    expect(m).toContain('ne peut pas écrire dans le dossier du jeu');
    expect(finALire(b)).toBe(true);
    expect(messageFinGog(fin({ titre: 'Dune', rattrape: true }))).toContain('pendant que Frogtend était fermé');
  });
});

describe('« ⬇ Installer » un jeu GOG', () => {
  beforeEach(() => {
    [...Object.values(api), ...Object.values(fenetres), rechargerJeuxDuPc, rechargerListe].forEach((f) => f.mockClear());
    fenetres.cocher.mockResolvedValue(null);
    fenetres.confirmer.mockResolvedValue(true);
    preparation = prep();
    bilan = fin({ dlcs_installes: ['REDmod'] });
    attendre = false;
    enAttente = null;
    debit = null;
  });

  it('avec des DLC : le jeu en 1re case, tout coché d’office, puis l’installation, la fin et les listes relues', async () => {
    fenetres.cocher.mockResolvedValueOnce(['1423049311', '1597316373']);
    const finies = installGog.finies;
    expect(await installerGog({ cle: 'gog_1423049311', titre: 'Cyberpunk' })).toBe(true);
    const [titre, options, o] = fenetres.cocher.mock.calls[0] as [string, { valeur: string; libelle: string }[], { coches: boolean[]; refus: (v: string[]) => string | null; libelleValider: string }];
    expect(titre).toBe('⬇ Installer « Cyberpunk 2077 » ?');
    expect(options.map((x) => x.valeur)).toEqual(['1423049311', '1597316373']);
    expect(options[0].libelle).toBe('Cyberpunk 2077 (le jeu)');
    expect(o.coches).toEqual([true, true]);
    expect(o.refus(['1597316373'])).toBe('Le jeu lui-même est nécessaire : garde-le coché.');
    expect(o.libelleValider).toBe('⬇ Installer');
    expect(api.gogInstaller).toHaveBeenCalledWith('1423049311', ['1597316373'], 7);
    expect(fenetres.toast).toHaveBeenCalledWith('✅ « Cyberpunk 2077 » est installé : ▶ Jouer.', 'ok');
    expect(rechargerJeuxDuPc).toHaveBeenCalled();
    expect(rechargerListe).toHaveBeenCalled();
    expect(installGog.finies).toBe(finies + 1);
    expect(installGog.cle).toBeNull();
  });

  it('sans DLC : une simple confirmation ; annulée, rien n’est installé ni relu', async () => {
    preparation = prep({ dlcs: [] });
    fenetres.confirmer.mockResolvedValueOnce(false);
    expect(await installerGog({ cle: '1423049311', titre: 'Cyberpunk' })).toBe(false);
    expect(fenetres.cocher).not.toHaveBeenCalled();
    expect(api.gogInstaller).not.toHaveBeenCalled();
    expect(rechargerJeuxDuPc).not.toHaveBeenCalled();
    // Sans la place : la raison est dite, jamais de bouton pour continuer.
    expect(await accordGog(prep({ dlcs: [], libre: GO }), false)).toBeNull();
    expect(String(fenetres.informer.mock.calls[0][1])).toContain('Pas assez de place');
  });

  it('un refus : expliqué ; Galaxy n’est proposé que s’il peut aider (pas d’installeur hors ligne) et s’il est là', async () => {
    preparation = prep({ refus: 'Ce jeu GOG est déjà installé sur ce PC.', galaxy: true });
    expect(await installerGog({ cle: '1', titre: 'X' })).toBe(false);
    expect(fenetres.informer).toHaveBeenCalled();
    expect(fenetres.confirmer).not.toHaveBeenCalled();
    preparation = prep({ refus: 'GOG ne propose pas d’installeur hors ligne.', jeu: null, galaxy: false, galaxy_peut_aider: true });
    await installerGog({ cle: '1', titre: 'X' });
    expect(fenetres.confirmer).not.toHaveBeenCalled();
    preparation = prep({ refus: 'GOG ne propose pas d’installeur hors ligne.', jeu: null, galaxy: true, galaxy_peut_aider: true });
    await installerGog({ cle: '1', titre: 'X' }, { auCanape: true });
    const o = fenetres.confirmer.mock.calls[0][1] as { libelleValider: string; message: string };
    expect(o.libelleValider).toBe('Ouvrir GOG Galaxy');
    expect(o.message).toContain('peut demander la souris');
    expect(api.boutiqueOuvrir).toHaveBeenCalledWith('gog', '1', 'installer');
    expect(api.gogInstaller).not.toHaveBeenCalled();
  });

  it('installé pendant que Frogtend était fermé : « ✅ Terminer » finit le travail sans rien télécharger', async () => {
    preparation = prep({ rattrapage: true });
    bilan = fin({ rattrape: true });
    expect(await installerGog({ cle: '1423049311', titre: 'C' })).toBe(true);
    expect((fenetres.confirmer.mock.calls[0][1] as { libelleValider: string }).libelleValider).toBe('✅ Terminer');
    expect(fenetres.cocher).not.toHaveBeenCalled();
    expect(api.gogInstaller).toHaveBeenCalledWith('1423049311', [], 7);
  });

  it('après l’accord, un échec se lit dans une fenêtre (le refus de Windows, un fichier abîmé) ; avant, un message', async () => {
    fenetres.cocher.mockResolvedValue(['1423049311']);
    bilan = { sorte: 'refus', motif: '⚠ Installation annulée : tu as répondu Non à Windows. Rien n’a été installé.' };
    expect(await installerGog({ cle: '1423049311', titre: 'C' })).toBe(false);
    expect(fenetres.informer).toHaveBeenLastCalledWith('Installation de « Cyberpunk 2077 »', expect.stringContaining('tu as répondu Non à Windows'));
    preparation = { sorte: 'reseau', motif: 'GOG ne répond pas.' } as unknown as Error;
    await installerGog({ cle: '1423049311', titre: 'C' });
    expect(fenetres.toast).toHaveBeenLastCalledWith('GOG ne répond pas.', 'erreur');
  });

  it('jamais deux installations par un double appui ; Arrêter seulement pendant le téléchargement, et la sortie avertie', async () => {
    fenetres.cocher.mockResolvedValue(['1423049311']);
    attendre = true;
    expect(avertissementSortieGog()).toBeNull();
    const premiere = installerGog({ cle: 'gog_1423049311', titre: 'C' });
    await vi.waitFor(() => expect(enAttente).not.toBeNull());
    expect(enInstallationGog('1423049311')).toBe(true);
    expect(await installerGog({ cle: '1423049311', titre: 'C' })).toBe(false);
    expect(api.gogInstallationPreparer).toHaveBeenCalledTimes(1);
    // Pendant le téléchargement : confirmer (danger), puis l'arrêt demandé au cœur ; quitter n'abîme rien.
    expect(installGog.phase).toBe('telechargement');
    expect(avertissementSortieGog()).toContain('ce qui est reçu est gardé');
    await arreterInstallationGog();
    expect((fenetres.confirmer.mock.calls.at(-1)?.[1] as { danger: boolean }).danger).toBe(true);
    expect(api.gogInstallationAnnuler).toHaveBeenCalledWith('1423049311');
    // L'installeur lancé : plus d'arrêt, c'est dit ; quitter est averti.
    progresVus?.({ etape: 'installation' });
    expect(installGog.phase).toBe('installation');
    expect(avertissementSortieGog()).toContain('ni vérifié ni prêt à jouer');
    api.gogInstallationAnnuler.mockClear();
    await arreterInstallationGog();
    expect(api.gogInstallationAnnuler).not.toHaveBeenCalled();
    expect(fenetres.toast).toHaveBeenLastCalledWith(expect.stringContaining('ne peut plus être arrêté'), 'alerte');
    enAttente!(fin({ pause: true }));
    expect(await premiere).toBe(false);
    expect(fenetres.toast).toHaveBeenLastCalledWith(expect.stringContaining('ce qui est téléchargé est gardé'), 'neutre');
    expect(enInstallationGog('1423049311')).toBe(false);
    expect(avertissementSortieGog()).toBeNull();
  });
});
