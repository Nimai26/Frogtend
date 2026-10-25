// Une longue liste de choix (une carte e-Reader parmi ~660 versions) : un champ « 🔎 Chercher » au-delà de SEUIL_FILTRE
// options, qui garde celles dont le libellé ou le détail contient tous les mots tapés (sans majuscules ni accents).

/** Au-delà de ce nombre d'options, la fenêtre de choix propose de chercher. */
export const SEUIL_FILTRE = 20;

const simple = (t: string) =>
  t
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase();

/** Les options qui répondent au texte tapé (vide : toutes). */
export function filtrerOptions<T extends { libelle: string; detail?: string }>(options: T[], texte: string): T[] {
  const mots = simple(texte).split(/\s+/).filter(Boolean);
  if (!mots.length) return options;
  return options.filter((o) => {
    const t = simple(`${o.libelle} ${o.detail ?? ''}`);
    return mots.every((m) => t.includes(m));
  });
}
