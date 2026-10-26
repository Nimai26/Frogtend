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
