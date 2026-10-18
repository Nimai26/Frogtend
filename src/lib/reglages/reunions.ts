// Contrat 1.14 : un jeu du PC rattaché à sa nouvelle fiche (fiches réunies chez Firehouse) garde les réglages faits
// pour lui sous son ANCIEN numéro. Ils sont COPIÉS vers le nouveau numéro, seulement s'il n'y en a pas déjà (un
// réglage fait sous le nouveau numéro prime) ; l'ancien n'est jamais effacé (rien à défaire, rien de perdu).
// Chaque réunion n'est suivie qu'UNE fois (`reunionsSuivies`) : un réglage effacé exprès sous le nouveau numéro ne
// revient pas depuis l'ancien.

type Suivi = { reunionsSuivies: string[] };

/** Les réunions pas encore suivies, et la liste des suivies une fois celles-ci ajoutées. */
function nouvelles(r: Suivi, reunions: [number, number][]): { a_suivre: [number, number][]; suivies: string[] } {
  const cle = ([a, n]: [number, number]) => `${a}>${n}`;
  const a_suivre = reunions.filter((x) => !r.reunionsSuivies.includes(cle(x)));
  return { a_suivre, suivies: [...r.reunionsSuivies, ...a_suivre.map(cle)] };
}

/** Les réglages du PC propres à un jeu : son émulateur. */
export function migrerPc<T extends Suivi & { emulateursJeux: Record<string, string> }>(pc: T, reunions: [number, number][]): T | null {
  const { a_suivre, suivies } = nouvelles(pc, reunions);
  if (!a_suivre.length) return null;
  const e = copier(pc.emulateursJeux, a_suivre);
  return { ...pc, emulateursJeux: e ?? pc.emulateursJeux, reunionsSuivies: suivies };
}

/** Les réglages du profil propres à un jeu : commandes, plein écran, manette → clavier, gel pendant le menu. */
export function migrerProfil<
  T extends Suivi & {
    commandes: Record<string, unknown>;
    pleinEcran: Record<string, unknown>;
    clavier: Record<string, unknown>;
    figer: string[];
  },
>(p: T, reunions: [number, number][]): T | null {
  const { a_suivre, suivies } = nouvelles(p, reunions);
  if (!a_suivre.length) return null;
  const commandes = copier(p.commandes, a_suivre);
  const pleinEcran = copier(p.pleinEcran, a_suivre);
  const clavier = copier(p.clavier, a_suivre);
  const ajout = a_suivre.filter(([a, n]) => p.figer.includes(String(a)) && !p.figer.includes(String(n))).map(([, n]) => String(n));
  return {
    ...p,
    commandes: commandes ?? p.commandes,
    pleinEcran: pleinEcran ?? p.pleinEcran,
    clavier: clavier ?? p.clavier,
    figer: ajout.length ? [...p.figer, ...ajout] : p.figer,
    reunionsSuivies: suivies,
  };
}

/** Le dictionnaire avec les valeurs de l'ancien numéro copiées sous le nouveau (s'il n'y en a pas), ou `null`. */
function copier<V>(d: Record<string, V>, reunions: [number, number][]): Record<string, V> | null {
  let r: Record<string, V> | null = null;
  for (const [a, n] of reunions) {
    if (String(a) in d && !(String(n) in d)) {
      r ??= { ...d };
      r[String(n)] = d[String(a)];
    }
  }
  return r;
}
