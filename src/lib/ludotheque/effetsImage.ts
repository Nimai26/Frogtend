// Les effets d'image de RetroArch (lot 4a) et de BigPEmu (préfixe `bigpemu:`)
// ce que la personne lit. Les identifiants sont ceux du cœur
// (`EFFETS_IMAGE`, src-tauri/src/emulateurs.rs) ; le cœur dit lesquels ce RetroArch a.

export const EFFETS_IMAGE: Record<string, { libelle: string; detail: string }> = {
  lisse: { libelle: 'Image adoucie', detail: 'les pixels sont fondus, l’image est plus douce' },
  net: { libelle: 'Pixels nets', detail: 'de gros pixels bien carrés, sans flou' },
  crt: { libelle: 'Télé d’époque (simple)', detail: 'de fines lignes horizontales, comme une vieille télé' },
  'crt-riche': { libelle: 'Télé d’époque (détaillée)', detail: 'lignes, lueur et grain d’une vieille télé ; demande plus à la carte graphique' },
  portable: { libelle: 'Écran de console portable', detail: 'la grille de points d’un écran de Game Boy' },
  // BigPEmu (Atari Jaguar, 11/11) : ses effets livrés (`bigpemu::EFFETS`), libellés tirés de leur nom — à confirmer à
  // l'œil avec Seb.
  'bigpemu:cheesy_scanlines': { libelle: 'Lignes de télé (léger)', detail: 'de fines lignes, comme une vieille télé' },
  'bigpemu:crt-hyllian': { libelle: 'Télé d’époque', detail: 'lignes et couleurs d’un vieux téléviseur ; demande plus à la carte graphique' },
  'bigpemu:gamma': { libelle: 'Image plus claire', detail: 'éclaircit les zones sombres' },
  'bigpemu:liz': { libelle: 'Télé d’époque (douce)', detail: 'un rendu de vieille télé, plus doux' },
  'bigpemu:oh_my_gauss': { libelle: 'Image adoucie', detail: 'les pixels sont fondus, sans escaliers' },
  'bigpemu:warping': { libelle: 'Écran bombé', detail: 'l’image courbée comme un écran à tube' },
  'bigpemu:xbr-lv2': { libelle: 'Contours lissés', detail: 'les bords des dessins sont arrondis' },
  'bigpemu:xbr-lv3-multipass': { libelle: 'Contours lissés (fort)', detail: 'plus lisse ; demande plus à la carte graphique' },
};

/** Le nom d'un réglage d'effet (`undefined` ou `aucun` : aucun effet). */
export function libelleEffet(v: string | undefined): string {
  return v === undefined || v === 'aucun' ? 'aucun effet' : (EFFETS_IMAGE[v]?.libelle ?? v);
}
