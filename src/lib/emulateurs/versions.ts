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
