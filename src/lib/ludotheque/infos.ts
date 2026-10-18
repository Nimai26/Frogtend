// Les infos d'un jeu (contrat 1.15 : `infos` du détail), dites en français pour des gens qui veulent juste jouer :
// jamais l'anglais brut de LaunchBox, jamais une ligne vide. Sans `new Date()` : une date lue en UTC reculerait d'un
// jour selon le fuseau.

/** Les infos telles que Firehouse les sert (contrat 1.15). */
export interface InfosJeu {
  date_sortie?: string | null;
  type_sortie?: string | null;
  esrb?: string | null;
  pegi?: number | null;
  note?: number | null;
  joueurs_min?: number | null;
  joueurs_massif?: boolean | null;
}

const MOIS = ['janvier', 'février', 'mars', 'avril', 'mai', 'juin', 'juillet', 'août', 'septembre', 'octobre', 'novembre', 'décembre'];

/** « 1992 », « mars 1992 », « 12 mars 1992 » (`AAAA`, `AAAA-MM`, `AAAA-MM-JJ`) ; rien si vide ou mal formée. */
export function dateDeSortie(d: string | null | undefined): string | null {
  const m = /^(\d{4})(?:-(\d{2})(?:-(\d{2}))?)?$/.exec((d ?? '').trim());
  if (!m) return null;
  const [, a, mo, j] = m;
  if (!mo) return a;
  const mois = MOIS[Number(mo) - 1];
  if (!mois) return a;
  if (!j) return `${mois} ${a}`;
  const jour = Number(j);
  if (jour < 1 || jour > 31) return `${mois} ${a}`;
  return `${jour === 1 ? '1er' : jour} ${mois} ${a}`;
}

/** Une date plus précise que l'année seule (sinon la ligne « Sortie » répéterait l'année déjà montrée). */
export function datePrecise(d: string | null | undefined): boolean {
  return /^\d{4}-\d{2}/.test((d ?? '').trim());
}

const TYPES: Record<string, string> = {
  Unreleased: 'Jamais sorti',
  'Early Access': 'Accès anticipé',
  Homebrew: 'Homebrew (amateur)',
  'ROM Hack': 'ROM modifiée (hack)',
  Unlicensed: 'Sans licence',
  DLC: 'Contenu additionnel (DLC)',
};

/** Le type de sortie à SIGNALER (un jeu sorti normalement : rien) ; une valeur hors du contrat : rien. */
export function typeDeSortie(t: string | null | undefined): string | null {
  return TYPES[(t ?? '').trim()] ?? null;
}

/** L'âge conseillé par l'ESRB (contrat 1.15 : liste fermée des libellés LaunchBox). */
const AGES_ESRB: Record<string, number> = {
  'EC - Early Childhood': 3,
  'E - Everyone': 6,
  'E10+ - Everyone 10+': 10,
  'T - Teen': 13,
  'M - Mature': 17,
  'AO - Adults Only': 18,
};

/** L'âge conseillé, lisible tout de suite : PEGI s'il existe, sinon l'ESRB ; « en attente », « non classé » ou une
 * valeur inconnue : rien. */
export function ageConseille(i: InfosJeu | null | undefined): string | null {
  if (!i) return null;
  if (typeof i.pegi === 'number' && [3, 7, 12, 16, 18].includes(i.pegi)) return `${i.pegi} ans et plus (PEGI)`;
  const a = AGES_ESRB[(i.esrb ?? '').trim()];
  return a ? `${a} ans et plus (ESRB)` : null;
}

const UNE_DECIMALE = new Intl.NumberFormat('fr-FR', { minimumFractionDigits: 1, maximumFractionDigits: 1 });

/** « 3,9 / 5 » (0 est une vraie note) ; rien sans note. */
export function noteSurCinq(n: number | null | undefined): string | null {
  if (typeof n !== 'number' || !Number.isFinite(n) || n < 0 || n > 5) return null;
  return `${UNE_DECIMALE.format(n)} / 5`;
}
