// Les versions d'un jeu importé (régions, révisions) : une fiche par jeu et par système, ses ROM en versions, dans
// l'ordre de préférence de Seb (02/10) : fr, eu, us/en, puis les autres. Le cœur les range déjà dans cet ordre.
import type { Installation } from '$lib/api';

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

/** « EU — Europe, Rev 1 · 2 disques ». */
export function libelleVersion(v: VersionLocale): string {
  const disques = v.disques?.length ? ` · ${v.disques.length} disques` : '';
  return `${RANGS[v.rang] ?? 'autre'} — ${v.libelle || 'sans étiquette'}${disques}`;
}

/** Le nom du fichier d'une version (affiché en détail). */
export function nomDuFichier(v: VersionLocale): string {
  return v.chemin.split(/[\\/]/).pop() ?? v.chemin;
}
