/** Eden et ses dérivés (mêmes dossiers `user\`, mêmes utilisateurs, clés et firmware) : `emulateurs.rs` ▸ `famille_eden`. */
export const FAMILLE_EDEN = ['eden', 'eglantine', 'lichii'];

/** Les émulateurs du catalogue que Firehouse résout (origine `Firehouse` côté cœur) : Eglantine (contrat 1.38), Lichii
 * (contrat 1.40). */
export const PAR_FIREHOUSE = ['eglantine', 'lichii'];

/** Une mise à jour à proposer ? Pour un émulateur résolu par Firehouse, toute version servie DIFFÉRENTE de l'installée
 * (Firehouse est juge : Eglantine passe de « daily-20261107 » à « v0.1.0-rc3 », que la comparaison des nombres donne
 * pour plus ancienne) ; sinon, une version plus récente. */
export function miseAJourDisponible(id: string, servie: string | undefined, installee: string | null | undefined): boolean {
  if (!servie || !installee) return false;
  return PAR_FIREHOUSE.includes(id) ? servie.trim() !== installee.trim() : plusRecente(servie, installee);
}

/** « 1.22.10 » est plus récent que « 1.22.2 » : on compare les nombres, dans l'ordre (« v2.3 », « 2026-09-28 »…). */
export function plusRecente(a: string, b: string): boolean {
  const n = (v: string) => (v.match(/\d+/g) ?? []).map(Number);
  const x = n(a);
  const y = n(b);
  for (let i = 0; i < Math.max(x.length, y.length); i++) {
    if ((x[i] ?? 0) !== (y[i] ?? 0)) return (x[i] ?? 0) > (y[i] ?? 0);
  }
  return false;
}
