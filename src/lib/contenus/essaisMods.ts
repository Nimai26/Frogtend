// Les simulations communes des tests des mods Switch (la classe `ModsSwitchJeu`, la bande `BlocMods`, la page
// `PageMods`) : le cœur de Frogtend, les fenêtres maison, le profil, la partie en cours. Aucun test ne parle au vrai
// cœur ni à Firehouse. Chaque fichier de test branche ces simulations par `vi.mock(…, () => import('./essaisMods')…)`.
import { vi } from 'vitest';
import { tick } from 'svelte';

export type Mod = Record<string, unknown>;

export function mod(x: Mod = {}): Mod {
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

export const liste = (mods: Mod[], x: Mod = {}) => ({
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

export const SHA = 'ab'.repeat(32);
export const INACTIF = { actif: false, en_cours: null, recus: 0, total: 0, faits: 0, a_faire: 0, erreurs: [], arret: null };
/** La clé de « DK Summit » (contrat 1.55) et d'un remix qui change la même musique. */
export const DK = 'gamebanana-ee3c0683c3f6';
export const REMIX = 'gamebanana-0123456789ab';
export const dk = (x: Mod = {}) => mod({ cle: DK, nom: 'DK Summit', amorces: ['pBGM_GC_SHERBET_LAND_F', 'pBGM_GC_SHERBET_LAND_N'], ...x });

/** Ce que rendent le cœur et l'état simulés (remis à zéro par `remettre`). */
export const simu = {
  reponse: liste([]) as object,
  suiviReponse: INACTIF as object,
  debit: null as number | null,
  profil: { modsSwitch: { auto: false } },
  profilOuvert: { id: 'seb' } as { id: string } | null,
  partie: { enJeu: null as number | null, lancement: null as number | null },
  seul: { ouvert: null as string | null, ouverture: null as number | null },
};

export const lister = vi.fn(async (..._a: unknown[]): Promise<unknown> => structuredClone(simu.reponse));
export const suivre = vi.fn(async (..._a: unknown[]): Promise<unknown> => structuredClone(simu.suiviReponse));
export const telecharger = vi.fn(async (..._a: unknown[]) => 1);
export const choisirMod = vi.fn(async (..._a: unknown[]): Promise<boolean> => true);
export const retirer = vi.fn(async (..._a: unknown[]) => {});
export const tailleMusiques = vi.fn(async (..._a: unknown[]): Promise<number | null> => 41018496);
export const preparerMusiques = vi.fn(async (..._a: unknown[]) => {});
export const confirmer = vi.fn(async (..._a: unknown[]) => true);
export const cocher = vi.fn(async (..._a: unknown[]): Promise<unknown> => null);
export const choisirDialogue = vi.fn(async (..._a: unknown[]): Promise<unknown> => null);
export const toast = vi.fn();
export const reglerProfil = vi.fn(async (..._a: unknown[]) => {});
export const goto = vi.fn(async (..._a: unknown[]) => {});

export function apiSimulee() {
  return {
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
  };
}

export function fenetresSimulees() {
  return {
    confirmer: (...a: unknown[]) => confirmer(...a),
    cocher: (...a: unknown[]) => cocher(...a),
    choisir: (...a: unknown[]) => choisirDialogue(...a),
    toast: (...a: unknown[]) => toast(...a),
    informer: vi.fn(async () => {}),
  };
}

export function etatSimule() {
  return {
    etat: {
      get profil() {
        return simu.profil;
      },
      get profilOuvert() {
        return simu.profilOuvert;
      },
    },
    reglerProfil: (...a: unknown[]) => reglerProfil(...a),
  };
}

export async function jeuSimule() {
  const s = await import('$lib/emulateurs/seul');
  return { lancementImpossible: () => s.lancementImpossibleSelon(simu.partie, simu.seul), attenteEmulateurSeul: () => s.raisonDAttente(simu.seul) };
}

export function telechargementsSimules() {
  return { debitConnu: () => simu.debit };
}

/** Avant chaque test : le cœur répond comme d'habitude, rien n'a été appelé. */
export function remettre() {
  simu.reponse = liste([]);
  simu.suiviReponse = INACTIF;
  simu.profil.modsSwitch.auto = false;
  simu.profilOuvert = { id: 'seb' };
  simu.debit = null;
  simu.partie.enJeu = null;
  simu.seul.ouvert = null;
  for (const f of [lister, suivre, telecharger, choisirMod, retirer, confirmer, cocher, choisirDialogue, toast, reglerProfil, tailleMusiques, preparerMusiques, goto]) f.mockClear();
  tailleMusiques.mockImplementation(async () => 41018496);
  lister.mockImplementation(async () => structuredClone(simu.reponse));
  suivre.mockImplementation(async () => structuredClone(simu.suiviReponse));
  telecharger.mockImplementation(async () => 1);
  choisirMod.mockImplementation(async () => true);
  confirmer.mockImplementation(async () => true);
  cocher.mockImplementation(async () => null);
  choisirDialogue.mockImplementation(async () => null);
  preparerMusiques.mockImplementation(async () => {});
  retirer.mockImplementation(async () => {});
}

/** Laisse passer les promesses en cours (lecture de la liste, du suivi…). */
export async function attendre() {
  for (let i = 0; i < 8; i++) {
    await tick();
    await Promise.resolve();
  }
}
