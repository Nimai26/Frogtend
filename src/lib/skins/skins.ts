// Les skins de Firehouse : leur catalogue, le choix du skin et son application à la page (charte § 1 et § 8).

import { resoudreEncres, type Jetons } from './encres';

/** Le skin de repli, quand un nom est inconnu. Firehouse fait de même. */
export const SKIN_DE_REPLI = 'firehouse';

/** Les 11 variables que chaque skin définit. */
export const VARIABLES_DE_BASE = [
  '--bg', '--bg2', '--panel', '--line', '--ink', '--dim',
  '--accent', '--accent2', '--ok', '--warn', '--err',
] as const;

export interface Skin {
  base: Jetons;
  /** Encres déjà résolues : `--on-*`, `--*-texte`, `--dim` corrigé. */
  resolus: Jetons;
  /** Adresse du fond vidéo (seulement le skin `firehouse`), ou `null`. */
  video: string | null;
  /** Chemin de la vidéo depuis la RACINE du serveur (contrat 1.3), ou `null`. */
  video_api?: string | null;
}

/** La forme servie par `GET /api/jeux/v1/themes`. */
export interface CatalogueSkins {
  version: string;
  credit: string;
  communs: Jetons;
  themes: Record<string, Skin>;
}

/** L'instantané de Firehouse (`docs/charte/themes.instantane.json`) : 11 variables par skin, sans encres. */
export interface Instantane {
  _credit: string;
  themes: Record<string, Jetons>;
}

/**
 * Construit un catalogue à partir d'un instantané et de jetons communs, en calculant les encres.
 * Sert au mode simulé ; la vraie source, Firehouse, fournit les encres déjà résolues.
 */
export function catalogueDepuisInstantane(instantane: Instantane, communs: Jetons): CatalogueSkins {
  const themes: Record<string, Skin> = {};
  for (const [nom, base] of Object.entries(instantane.themes)) {
    themes[nom] = { base, resolus: resoudreEncres(base, communs), video: null };
  }
  return { version: 'instantane', credit: instantane._credit, communs, themes };
}

/** Le nom du skin à appliquer : celui demandé s'il existe, sinon celui en cours, sinon `firehouse`. */
export function choisirSkin(catalogue: CatalogueSkins, demande: string | null, enCours: string | null): string {
  if (demande && demande in catalogue.themes) return demande;
  if (enCours && enCours in catalogue.themes) return enCours;
  return SKIN_DE_REPLI;
}

/** Tous les jetons d'un skin, fusionnés dans l'ordre : communs, base, puis encres résolues. */
export function jetonsDuSkin(catalogue: CatalogueSkins, nom: string): Jetons {
  const skin = catalogue.themes[nom];
  if (!skin) return { ...catalogue.communs };
  return { ...catalogue.communs, ...skin.base, ...skin.resolus };
}

/** Les noms des skins, triés pour l'affichage (le skin de repli en premier). */
export function nomsDesSkins(catalogue: CatalogueSkins): string[] {
  return Object.keys(catalogue.themes).sort((a, b) =>
    a === SKIN_DE_REPLI ? -1 : b === SKIN_DE_REPLI ? 1 : a.localeCompare(b, 'fr'),
  );
}

/** Nom lisible d'un skin : « blackberry-abyss » → « Blackberry abyss ». */
export function libelleSkin(nom: string): string {
  const t = nom.replace(/[-_]+/g, ' ');
  return t.charAt(0).toUpperCase() + t.slice(1);
}

let jetonsPoses: string[] = [];

/**
 * Applique un skin à un élément (la racine du document par défaut).
 * Les jetons du skin précédent qui n'existent plus sont retirés.
 */
export function appliquerSkin(
  catalogue: CatalogueSkins,
  nom: string,
  cible: HTMLElement = document.documentElement,
): string {
  const retenu = choisirSkin(catalogue, nom, null);
  const jetons = jetonsDuSkin(catalogue, retenu);
  for (const ancien of jetonsPoses) if (!(ancien in jetons)) cible.style.removeProperty(ancien);
  for (const [k, v] of Object.entries(jetons)) cible.style.setProperty(k, v);
  jetonsPoses = Object.keys(jetons);
  cible.dataset.skin = retenu;
  return retenu;
}
