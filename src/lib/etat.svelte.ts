// L'état partagé de l'application : réglages (PC et profil ouvert), catalogue des skins, skin appliqué.

import { isTauri } from '@tauri-apps/api/core';
import instantane from '../../docs/charte/themes.instantane.json';
import { api, type Profil } from './api';
import { toast } from './dialogues/fenetres.svelte';
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
  SKIN_DE_REPLI,
  type CatalogueSkins,
  type Instantane,
} from './skins/skins';

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
  /** Le profil ouvert, ou `null` : on est alors sur l'écran de choix du profil. */
  profilOuvert: Profil | null;
  profil: ReglagesProfil;
  catalogue: CatalogueSkins | null;
  /** Le skin choisi par la personne dans Firehouse, s'il est connu. */
  skinFirehouse: string | null;
  skinApplique: string | null;
  /** Le skin appliqué a-t-il un fond vidéo à montrer ? */
  fondVideo: string | null;
}>({
  pret: false,
  pc: structuredClone(DEFAUTS_PC),
  profilOuvert: null,
  profil: structuredClone(DEFAUTS_PROFIL),
  catalogue: null,
  skinFirehouse: null,
  skinApplique: null,
  fondVideo: null,
});

let magasinPc: Magasin | null = null;
let magasinProfil: Magasin | null = null;

/** L'instantané des skins : une fixture, utilisée seulement tant que Firehouse n'a pas donné les siens. */
const catalogueInstantane = () => catalogueDepuisInstantane(instantane as Instantane, fixtureCommuns.communs);

/** Applique à la page l'apparence du profil : skin, échelle, densité, animations. */
export function appliquerApparence() {
  if (!etat.catalogue) return;
  const a = etat.profil.apparence;
  etat.skinApplique = appliquerSkin(etat.catalogue, a.skin ?? etat.skinFirehouse ?? SKIN_DE_REPLI);
  const skin = etat.catalogue.themes[etat.skinApplique];
  etat.fondVideo = a.fondVideo && skin?.video_api ? etat.skinApplique : null;
  const racine = document.documentElement;
  racine.style.setProperty('--echelle', String(a.echelle));
  racine.dataset.densite = a.densite;
  racine.dataset.animations = a.animations;
}

/** Au lancement : réglages du PC et skin de repli (aucun profil n'est encore ouvert). */
export async function demarrer() {
  magasinPc = await magasin('pc.json');
  etat.pc = completer(DEFAUTS_PC, await magasinPc.lire());
  // Premier lancement : on écrit les réglages pour que le cœur les lise.
  await magasinPc.ecrire($state.snapshot(etat.pc));
  etat.catalogue = catalogueInstantane();
  appliquerApparence();
  etat.pret = true;
  // Les vrais skins de Firehouse, sans jeton : dès l'écran « Qui joue ? ».
  await chargerSkinsFirehouse();
}

/** Les skins servis par Firehouse (publics). Hors ligne ou en mode simulé : ceux déjà là. */
export async function chargerSkinsFirehouse() {
  try {
    const catalogue = (await api.skins()) as CatalogueSkins | null;
    if (catalogue?.themes && Object.keys(catalogue.themes).length > 0) {
      etat.catalogue = catalogue;
      appliquerApparence();
    }
  } catch {
    // Firehouse injoignable : l'instantané reste.
  }
}

/** Après l'ouverture d'un profil : ses réglages, puis les skins de Firehouse et le sien. */
export async function entrerDansProfil(profil: Profil) {
  magasinProfil = await magasin(`profils/${profil.id}.json`);
  etat.profil = completer(DEFAUTS_PROFIL, await magasinProfil.lire());
  etat.profilOuvert = profil;
  appliquerApparence();

  await chargerSkinsFirehouse();
  try {
    const perso = await api.skinPersonnel();
    if (perso?.theme) etat.skinFirehouse = perso.theme;
    if (perso?.remplace) toast('⚠ Ton skin enregistré dans Firehouse n’existe plus : le skin Firehouse est appliqué.');
  } catch {
    // Hors ligne ou mode simulé : on garde les skins déjà là.
  }
  appliquerApparence();
}

/** Ferme le profil : plus rien de lui n'est affiché ni gardé en mémoire ici. */
export async function sortirDuProfil() {
  await api.fermerProfil().catch(() => {});
  etat.profilOuvert = null;
  etat.skinFirehouse = null;
  magasinProfil = null;
  etat.profil = structuredClone(DEFAUTS_PROFIL);
  appliquerApparence();
}

/** Remplace TOUS les réglages du profil (restauration d'une sauvegarde), complétés par les défauts. */
export async function remplacerReglagesProfil(lus: unknown) {
  etat.profil = completer(DEFAUTS_PROFIL, lus);
  appliquerApparence();
  await magasinProfil?.ecrire($state.snapshot(etat.profil));
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
