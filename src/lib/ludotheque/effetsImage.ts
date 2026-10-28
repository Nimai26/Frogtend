// Les effets d'image de RetroArch (lot 4a) : ce que la personne lit. Les identifiants sont ceux du cœur
// (`EFFETS_IMAGE`, src-tauri/src/emulateurs.rs) ; le cœur dit lesquels ce RetroArch a.

export const EFFETS_IMAGE: Record<string, { libelle: string; detail: string }> = {
  lisse: { libelle: 'Image adoucie', detail: 'les pixels sont fondus, l’image est plus douce' },
  net: { libelle: 'Pixels nets', detail: 'de gros pixels bien carrés, sans flou' },
  crt: { libelle: 'Télé d’époque (simple)', detail: 'de fines lignes horizontales, comme une vieille télé' },
  'crt-riche': { libelle: 'Télé d’époque (détaillée)', detail: 'lignes, lueur et grain d’une vieille télé ; demande plus à la carte graphique' },
  portable: { libelle: 'Écran de console portable', detail: 'la grille de points d’un écran de Game Boy' },
};

/** Le nom d'un réglage d'effet (`undefined` ou `aucun` : aucun effet). */
export function libelleEffet(v: string | undefined): string {
  return v === undefined || v === 'aucun' ? 'aucun effet' : (EFFETS_IMAGE[v]?.libelle ?? v);
}
