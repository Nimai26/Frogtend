// Un jeu possédé sur plusieurs boutiques = UNE carte (A6c, Seb 21/10) : ses copies (Steam, GOG…) sont réunies par
// Frogtend d'après GOG Galaxy. S'il est installé de plusieurs côtés, Frogtend DEMANDE à chaque fois laquelle lancer.

/** Une copie installée sur ce PC (une ligne du registre du PC). */
export interface CopieInstallee {
  id: number;
  boutique: string;
  temps_jeu: number;
}

/** Les ids réunis avec `id` (lui compris), ou `[id]` seul. */
export function groupeDe(id: number, groupes: number[][]): number[] {
  return groupes.find((g) => g.includes(id)) ?? [id];
}

/** Une carte par jeu en Taodbox : parmi les jeux du PC, une seule copie par groupe (la première installée dans l'ordre
 * du groupe : Steam, GOG, Epic, puis les autres). */
export function unParGroupe<T extends { id: number }>(jeux: T[], groupes: number[][]): T[] {
  const vus = new Set<number>();
  const r: T[] = [];
  for (const j of jeux) {
    const g = groupeDe(j.id, groupes);
    if (g.length > 1) {
      if (g.some((x) => vus.has(x))) continue;
      // La copie montrée : la première du groupe présente parmi ces jeux.
      const montree = g.map((x) => jeux.find((y) => y.id === x)).find(Boolean) ?? j;
      g.forEach((x) => vus.add(x));
      r.push(montree);
    } else {
      r.push(j);
    }
  }
  return r;
}

/** D'où vient une carte de la ludothèque (Seb, 21/10 : une icône par origine, pour TOUS les jeux) : ses boutiques
 * (toutes ses copies), sinon « disque » (importé) ou « firehouse ». */
export function originesCarte(j: { id: number; source?: string; boutique?: string; copies?: { boutique: string }[] | null }): string[] {
  if (j.copies?.length) return [...new Set(j.copies.map((c) => c.boutique))];
  if (j.boutique) return [j.boutique];
  if (j.id < 0 && j.source === 'local') return ['disque'];
  return j.id < 0 ? ['disque'] : ['firehouse'];
}

/** D'où vient un jeu DU PC (Taodbox) : sa boutique (et celles des copies réunies avec lui), sinon disque/Firehouse. */
export function originesJeuPc(j: { id: number; installation?: { boutique?: { source: string } | null } | null }, groupe: number[], sourceDe: (id: number) => string | undefined): string[] {
  if (j.installation?.boutique) return [...new Set(groupe.map(sourceDe).filter((s): s is string => !!s))];
  return j.id < 0 ? ['disque'] : ['firehouse'];
}

/** Le nom court d'une origine, pour sa pastille. */
export function nomOrigine(o: string): string {
  return (
    ({ steam: 'Steam', gog: 'GOG', epic: 'Epic', xboxone: 'Xbox', xbox: 'Xbox', uplay: 'Ubisoft', origin: 'EA', ea: 'EA', amazon: 'Amazon', battlenet: 'Battle.net', firehouse: 'Firehouse', disque: 'Disque' } as Record<string, string>)[o] ??
    'Autre'
  );
}

/** Ce qu'on montre pour choisir une copie : « ▶ Steam », « 12 h jouées ». */
export function libelleCopie(c: CopieInstallee, nom: (b: string) => string, duree: (s: number) => string): { libelle: string; detail: string } {
  return { libelle: `▶ ${nom(c.boutique)}`, detail: c.temps_jeu > 0 ? `${duree(c.temps_jeu)} jouées par Frogtend` : 'jamais lancé par Frogtend' };
}
