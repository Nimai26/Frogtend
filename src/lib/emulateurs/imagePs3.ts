// Image et fluidité d'un jeu PS3 dans RPCS3 (✅ Seb 19/10 : « shaders = les deux »). Les clés et valeurs exactes de
// RPCS3 (section `Video:` de sa configuration, relevées dans ses sources) dites sans jargon pour des gens qui veulent
// juste jouer (avis de l'expert interface du 19/10). Le cœur n'accepte que ces valeurs-là.

export interface ValeurImage {
  valeur: string;
  libelle: string;
  detail?: string;
}

export interface ReglageImage {
  cle: string;
  titre: string;
  valeurs: ValeurImage[];
}

export const REGLAGES_IMAGE: ReglageImage[] = [
  {
    cle: 'Shader Mode',
    titre: '⚡ Fluidité',
    valeurs: [
      { valeur: 'Async Recompiler with Shader Interpreter', libelle: 'Sans saccades (recommandé)', detail: 'l’image ne se fige plus quand un nouvel effet apparaît' },
      { valeur: 'Async Recompiler (multi-threaded)', libelle: 'Rapide', detail: 'le plus fluide, de rares effets peuvent manquer un instant' },
      { valeur: 'Shader Interpreter only', libelle: 'Le plus sûr', detail: 'pour un jeu qui affiche des erreurs ; plus lent' },
    ],
  },
  {
    cle: 'Resolution Scale',
    titre: '🖼 Résolution',
    valeurs: [
      { valeur: '100', libelle: '100 %', detail: 'l’image d’origine du jeu' },
      { valeur: '150', libelle: '150 %', detail: 'plus fin, un peu plus lourd' },
      { valeur: '200', libelle: '200 %', detail: 'environ 1440p : demande une bonne carte graphique' },
      { valeur: '300', libelle: '300 %', detail: '4K : très lourd, le jeu peut ralentir' },
    ],
  },
  {
    cle: 'Output Scaling Mode',
    titre: '🔍 Agrandissement de l’image',
    valeurs: [
      { valeur: 'Bilinear', libelle: 'Doux', detail: 'l’image agrandie simplement' },
      { valeur: 'FidelityFX Super Resolution', libelle: 'Plus net', detail: 'contours plus nets quand l’image est agrandie (FSR)' },
    ],
  },
  {
    cle: 'FidelityFX CAS Sharpening Intensity',
    titre: '✨ Netteté de l’agrandissement',
    valeurs: [
      { valeur: '0', libelle: 'Aucune' },
      { valeur: '25', libelle: 'Légère' },
      { valeur: '50', libelle: 'Moyenne' },
      { valeur: '75', libelle: 'Forte' },
      { valeur: '100', libelle: 'Maximale' },
    ],
  },
  {
    cle: 'Anisotropic Filter Override',
    titre: '🏞 Netteté des sols et des murs au loin',
    valeurs: [
      { valeur: '0', libelle: 'Automatique', detail: 'comme le jeu le prévoit' },
      { valeur: '2', libelle: '2×' },
      { valeur: '4', libelle: '4×' },
      { valeur: '8', libelle: '8×' },
      { valeur: '16', libelle: '16×', detail: 'le plus net (filtrage anisotrope)' },
    ],
  },
  {
    cle: 'MSAA',
    titre: '📐 Bords lissés',
    valeurs: [
      { valeur: 'Auto', libelle: 'Automatique' },
      { valeur: 'Disabled', libelle: 'Non (plus léger)', detail: 'bords en escalier, mais moins de travail pour le PC' },
    ],
  },
];

/** Le libellé d'une valeur de RPCS3 (une valeur inconnue — réglée à la main dans RPCS3 — est montrée telle quelle). */
export function libelleValeur(cle: string, valeur: string): string {
  return REGLAGES_IMAGE.find((r) => r.cle === cle)?.valeurs.find((v) => v.valeur === valeur)?.libelle ?? valeur;
}

/** Les réglages à montrer : la netteté de l'agrandissement seulement avec l'agrandissement « Plus net » (FSR). */
export function reglagesMontres(valeurs: Map<string, string>): ReglageImage[] {
  return REGLAGES_IMAGE.filter((r) => r.cle !== 'FidelityFX CAS Sharpening Intensity' || valeurs.get('Output Scaling Mode') === 'FidelityFX Super Resolution');
}
