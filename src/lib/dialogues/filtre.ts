// Une longue liste de choix (une carte e-Reader parmi ~660 versions) : un champ « 🔎 Chercher » au-delà de SEUIL_FILTRE
// options, qui garde celles dont le libellé ou le détail contient tous les mots tapés (sans majuscules ni accents).

/** Au-delà de ce nombre d'options, la fenêtre de choix propose de chercher. */
export const SEUIL_FILTRE = 20;

/** Le texte sans majuscules ni accents (à calculer une fois pour une longue liste : la page des mods). */
export const simplifier = (t: string) =>
  t
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase();
const simple = simplifier;

/** Les options qui répondent au texte tapé (vide : toutes). */
export function filtrerOptions<T extends { libelle: string; detail?: string }>(options: T[], texte: string): T[] {
  const mots = simple(texte).split(/\s+/).filter(Boolean);
  if (!mots.length) return options;
  return options.filter((o) => {
    const t = simple(`${o.libelle} ${o.detail ?? ''}`);
    return mots.every((m) => t.includes(m));
  });
}

/** « N sur 282 » (une recherche dans une longue liste de mods), ou le mot quand rien ne répond. */
export function compteRecherche(montres: number, total: number): string {
  return montres ? `${montres.toLocaleString('fr-FR')} sur ${total.toLocaleString('fr-FR')}` : 'Aucun mod ne correspond : essaie un autre mot.';
}
