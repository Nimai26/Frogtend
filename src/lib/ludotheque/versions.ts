// Les versions d'un jeu importé (régions, révisions) : une fiche par jeu et par système, ses ROM en versions, dans
// l'ordre de préférence de Seb (02/10) : fr, eu, us/en, puis les autres. Le cœur les range déjà dans cet ordre.
import type { Installation } from '$lib/api';
import { aCartes } from './cartes';

/** Ce que le cœur dit d'une version avant qu'on coche (`versions_avis` : SA règle, jamais refaite ici). */
export interface AvisVersion {
  telechargement_id: number;
  cochable: boolean;
  /** 0 fr, 1 eu, 2 us/en, 3 autres. */
  rang: number;
  qualite: number;
  fichiers: string[];
}

/** Plusieurs versions de Firehouse cochables à la fois (✅ Seb 28/10) : toutes des ROM ou images disque simples selon
 * le cœur, et pas un jeu à cartes (e-Reader : une carte à la fois). */
export function plusieursPossibles(avis: AvisVersion[], plateforme: string | null | undefined): boolean {
  return avis.length > 1 && !aCartes(plateforme) && avis.every((a) => a.cochable);
}

/** L'ordre des cases : la région préférée d'abord (FR, EU, US/EN, autres — Seb, 02/10), puis la qualité, puis l'ordre
 * reçu (la plus récente d'abord). La première est cochée d'office et devient ⭐ celle d'habitude. */
export function ordreDesCases<T extends { telechargement_id: number }>(versions: T[], avis: AvisVersion[]): T[] {
  const de = (v: T) => avis.find((a) => a.telechargement_id === v.telechargement_id);
  return versions
    .map((v, i) => ({ v, i, a: de(v) }))
    .sort((x, y) => (x.a?.rang ?? 3) - (y.a?.rang ?? 3) || (x.a?.qualite ?? 0) - (y.a?.qualite ?? 0) || x.i - y.i)
    .map((x) => x.v);
}

/** Deux versions cochées qui ont un fichier du même nom (sans casse) : le nom, sinon `null`. */
export function nomEnDouble(ids: number[], avis: AvisVersion[]): string | null {
  const vus = new Map<string, number>();
  for (const id of ids) {
    for (const f of avis.find((a) => a.telechargement_id === id)?.fichiers ?? []) {
      const k = f.toLowerCase();
      if (vus.has(k) && vus.get(k) !== id) return f;
      vus.set(k, id);
    }
  }
  return null;
}

export interface VersionLocale {
  chemin: string;
  libelle: string;
  /** 0 fr, 1 eu, 2 us/en, 3 autres. */
  rang: number;
  qualite: number;
  disques?: string[];
}

const RANGS = ['FR', 'EU', 'US/EN', 'autre'];

/** Le chemin de la version lancée par défaut. */
export function cheminCourant(i: Pick<Installation, 'dossier' | 'fichier_du_jeu'>): string {
  return i.fichier_du_jeu ? `${i.dossier.replace(/[\\/]+$/, '')}\\${i.fichier_du_jeu}` : i.dossier;
}

export function estCourante(v: VersionLocale, i: Pick<Installation, 'dossier' | 'fichier_du_jeu'>): boolean {
  return v.chemin.replace(/\//g, '\\').toLowerCase() === cheminCourant(i).replace(/\//g, '\\').toLowerCase();
}

/** Une traduction de fan (« [T-Fr by …] ») : le cœur l'écrit en clair dans le libellé. */
export function estTraduction(v: Pick<VersionLocale, 'libelle'>): boolean {
  return /Traduction (française|anglaise)/.test(v.libelle);
}

/** « FR », ou « FR (trad.) » pour une traduction de fan (Seb, 04/10 : « il faudra le préciser »). */
export function rangCourt(v: Pick<VersionLocale, 'libelle' | 'rang'>): string {
  return `${RANGS[v.rang] ?? 'autre'}${estTraduction(v) ? ' (trad.)' : ''}`;
}

/** « EU — Europe, Rev 1 · 2 disques » ; « FR (traduction de fan) — USA, Traduction française par … ». */
export function libelleVersion(v: VersionLocale): string {
  const disques = v.disques?.length ? ` · ${v.disques.length} disques` : '';
  const rang = `${RANGS[v.rang] ?? 'autre'}${estTraduction(v) ? ' (traduction de fan)' : ''}`;
  return `${rang} — ${v.libelle || 'sans étiquette'}${disques}`;
}

/** Le nom du fichier d'une version (affiché en détail). */
export function nomDuFichier(v: VersionLocale): string {
  return v.chemin.split(/[\\/]/).pop() ?? v.chemin;
}
