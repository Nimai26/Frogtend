// L'état partagé de l'application : réglages (PC et profil), catalogue des skins, skin appliqué.

import { isTauri } from '@tauri-apps/api/core';
import instantane from '../../docs/charte/themes.instantane.json';
import fixtureCommuns from './fixtures/communs.simule.json';
import {
  completer,
  DEFAUTS_PC,
  DEFAUTS_PROFIL,
  ecrire,
  reinitialiser,
  type ReglagesPc,
  type ReglagesProfil,
} from './reglages/reglages';
import {
  appliquerSkin,
  catalogueDepuisInstantane,
  type CatalogueSkins,
  type Instantane,
} from './skins/skins';

/** Le profil tant que la gestion des profils (lot 1) n'existe pas. */
const PROFIL_PROVISOIRE = 'local';

/** Un magasin clé/valeur sur le disque (Tauri), ou en mémoire hors de Tauri (navigateur, tests). */
interface Magasin {
  lire(): Promise<unknown>;
  ecrire(valeur: unknown): Promise<void>;
}

async function magasin(fichier: string): Promise<Magasin> {
  if (!isTauri()) {
    let memoire: unknown;
    return { lire: async () => memoire, ecrire: async (v) => void (memoire = v) };
  }
  const { load } = await import('@tauri-apps/plugin-store');
  const s = await load(fichier, { autoSave: false, defaults: {} });
  return {
    lire: () => s.get('reglages'),
    async ecrire(v) {
      await s.set('reglages', v);
      await s.save();
    },
  };
}

export const etat = $state<{
  pret: boolean;
  pc: ReglagesPc;
  profil: ReglagesProfil;
  catalogue: CatalogueSkins | null;
  skinApplique: string | null;
}>({
  pret: false,
  pc: structuredClone(DEFAUTS_PC),
  profil: structuredClone(DEFAUTS_PROFIL),
  catalogue: null,
  skinApplique: null,
});

let magasinPc: Magasin | null = null;
let magasinProfil: Magasin | null = null;

/** Le catalogue des skins. En mode simulé : l'instantané de Firehouse (une fixture, jamais la source). */
function chargerCatalogue(): CatalogueSkins {
  // Le mode réel (GET /api/jeux/v1/themes, mis en cache) arrive avec le client Firehouse, au lot 1.
  return catalogueDepuisInstantane(instantane as Instantane, fixtureCommuns.communs);
}

/** Applique à la page l'apparence du profil : skin, échelle, densité, animations. */
export function appliquerApparence() {
  if (!etat.catalogue) return;
  const a = etat.profil.apparence;
  // Sans choix local, le skin de la personne dans Firehouse (lot 1 : GET /api/jeux/v1/theme) ; d'ici là, le repli.
  etat.skinApplique = appliquerSkin(etat.catalogue, a.skin ?? '');
  const racine = document.documentElement;
  racine.style.setProperty('--echelle', String(a.echelle));
  racine.dataset.densite = a.densite;
  racine.dataset.animations = a.animations;
}

export async function demarrer() {
  magasinPc = await magasin('pc.json');
  magasinProfil = await magasin(`profils/${PROFIL_PROVISOIRE}.json`);
  etat.pc = completer(DEFAUTS_PC, await magasinPc.lire());
  etat.profil = completer(DEFAUTS_PROFIL, await magasinProfil.lire());
  etat.catalogue = chargerCatalogue();
  appliquerApparence();
  etat.pret = true;
}

/** Change un réglage du profil (`'apparence.skin'`…) et l'enregistre. */
export async function reglerProfil(chemin: string, valeur: unknown) {
  etat.profil = ecrire($state.snapshot(etat.profil), chemin, valeur);
  appliquerApparence();
  await magasinProfil?.ecrire($state.snapshot(etat.profil));
}

/** Change un réglage du PC et l'enregistre. */
export async function reglerPc(chemin: string, valeur: unknown) {
  etat.pc = ecrire($state.snapshot(etat.pc), chemin, valeur);
  await magasinPc?.ecrire($state.snapshot(etat.pc));
}

/** Remet un réglage (ou toute une portée) à sa valeur d'origine. */
export async function reinitialiserProfil(chemin?: string) {
  etat.profil = reinitialiser($state.snapshot(etat.profil), DEFAUTS_PROFIL, chemin);
  appliquerApparence();
  await magasinProfil?.ecrire($state.snapshot(etat.profil));
}

export async function reinitialiserPc(chemin?: string) {
  etat.pc = reinitialiser($state.snapshot(etat.pc), DEFAUTS_PC, chemin);
  await magasinPc?.ecrire($state.snapshot(etat.pc));
}
