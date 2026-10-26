// Les jeux à CARTES (e-Reader : une version = une carte ; Firehouse 2.99.1) : plusieurs versions d'une même fiche
// gardées ensemble sur le PC, chacune ajoutée ou retirée à part. La même liste que `jeux_pc::PLATEFORMES_A_CARTES`.

import type { JeuPc } from '$lib/api';

export const PLATEFORMES_A_CARTES = ['Nintendo e-Reader'];

export function aCartes(plateforme: string | null | undefined): boolean {
  return !!plateforme && PLATEFORMES_A_CARTES.some((p) => p.toLowerCase() === plateforme.toLowerCase());
}

/** Les versions (cartes) d'un jeu à cartes présentes sur le PC. */
export function cartesSurPc(j: JeuPc | null | undefined): number[] {
  if (!j) return [];
  return [...new Set(j.fichiers.map((f) => f.version ?? j.version))];
}

/** Les cartes du PC à proposer avant de jouer : leur nom officiel (sans « .7z »), dans l'ordre des noms. */
export function choixDeCartes(j: JeuPc | null | undefined): { valeur: number; libelle: string }[] {
  if (!j) return [];
  const noms = new Map<number, string>();
  for (const f of j.fichiers) {
    const v = f.version ?? j.version;
    if (!noms.has(v)) noms.set(v, f.nom.replace(/^.*[\/]/, '').replace(/\.(7z|zip|raw)$/i, ''));
  }
  return [...noms].map(([valeur, libelle]) => ({ valeur, libelle })).sort((a, b) => a.libelle.localeCompare(b.libelle, 'fr', { numeric: true }));
}
