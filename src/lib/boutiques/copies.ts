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

/** « Vient de Steam et d’Epic » : l'élision devant une voyelle (Epic, Ubisoft, Amazon, EA…). */
export function venantDe(noms: string[]): string {
  const de = (n: string) => (/^[aeiouyéèêàâîïôûAEIOUYÉÈÊÀÂÎÏÔÛ]/.test(n) ? `d’${n}` : `de ${n}`);
  return `Vient ${noms.map(de).join(' et ')}`;
}

/** Ce qu'on montre pour choisir une copie : « ▶ Steam », « 12 h jouées ». */
export function libelleCopie(c: CopieInstallee, nom: (b: string) => string, duree: (s: number) => string): { libelle: string; detail: string } {
  return { libelle: `▶ ${nom(c.boutique)}`, detail: c.temps_jeu > 0 ? `${duree(c.temps_jeu)} jouées par Frogtend` : 'jamais lancé par Frogtend' };
}

/** Un jeu installé qu'une boutique montre sans être un jeu du PC (Taodbox : dans la grille principale, avec sa
 * pastille ; lot A, Seb 18/11). */
export interface CarteBoutiqueTaodbox {
  /** `steam_620`, `epic_Fortnite`… (unique). */
  cle: string;
  nom: string;
  /** L'origine pour la pastille (`steam`, `epic`, `uplay`…). */
  boutique: string;
  /** Par quoi l'ouvrir : Steam (appid) ou GOG Galaxy (clé Galaxy). */
  par: 'steam' | 'galaxy';
  image: string | null;
}

/** Les jeux installés de Steam et de GOG Galaxy qui ne sont PAS déjà des jeux du PC (même boutique et même clé) : une
 * carte chacun, jamais deux fois le même jeu. Steam d'abord. */
export function cartesBoutiqueTaodbox(
  steamJeux: { id: string; nom: string; installe: boolean }[],
  galaxyJeux: { id: string; nom: string; installe: boolean; plateforme?: string | null; image?: string | null }[],
  jeuxPc: { installation?: { boutique?: { source: string; cle: string } | null } | null }[],
): CarteBoutiqueTaodbox[] {
  const duPc = new Set(jeuxPc.map((j) => j.installation?.boutique).filter((b) => !!b).map((b) => `${b!.source}_${b!.cle}`));
  const r: CarteBoutiqueTaodbox[] = [];
  const vus = new Set<string>();
  for (const j of steamJeux.filter((x) => x.installe)) {
    const cle = `steam_${j.id}`;
    if (duPc.has(cle) || vus.has(cle)) continue;
    vus.add(cle);
    r.push({ cle, nom: j.nom, boutique: 'steam', par: 'steam', image: null });
  }
  for (const j of galaxyJeux.filter((x) => x.installe)) {
    if (duPc.has(j.id) || vus.has(j.id)) continue;
    vus.add(j.id);
    r.push({ cle: j.id, nom: j.nom, boutique: j.plateforme || j.id.split('_')[0], par: 'galaxy', image: j.image ?? null });
  }
  return r;
}

/** Une carte « à installer » au canapé (lot B, ✅ Seb 18/11) : un jeu de boutique de la ludothèque PAS installé — possédé
 * dans un compte, ou désinstallé hors de Frogtend. A ouvre l'installation par le lanceur de sa boutique ; jamais une
 * partie (pas de ▶ Jouer). */
export interface CarteAInstallerTaodbox {
  id: number;
  /** La clé de la carte (`epic_<AppName>`, `uplay_54`…), passée au lanceur. */
  cle: string;
  nom: string;
  /** La boutique qui installe (`epic`, `uplay`…), aussi pour la pastille. */
  boutique: string;
  image: string | null;
}

/** Les boutiques qui savent installer un jeu par leur lanceur depuis Frogtend (`boutique_ouvrir`). Steam (lot C2 : ses
 * jeux possédés pas installés, `steam://install/<appid>`) ; GOG Galaxy a ses propres cartes ; Battle.net n'a aucune
 * adresse prouvée. */
export const INSTALLABLES_AU_CANAPE = ['steam', 'epic', 'uplay', 'gog', 'amazon', 'xboxone'];

/** La clé de la carte « à installer », comme les autres boutiques la portent (`epic_<AppName>`) : une carte Steam garde
 * l'appid seul dans la ludothèque, elle devient `steam_<appid>` (le cœur retire ce préfixe pour l'installer). */
function cleAInstaller(c: { boutique?: string | null; cle_boutique?: string | null }): string {
  return c.boutique === 'steam' ? `steam_${c.cle_boutique}` : c.cle_boutique!;
}

/** Les cartes « à installer » de Taodbox : les cartes de la ludothèque d'une boutique installable, pas installées, et
 * qui ne sont pas déjà des jeux du PC (même boutique et même clé). Triées par nom. */
export function cartesAInstallerTaodbox(
  cartes: { id: number; titre: string; source?: string | null; boutique?: string | null; cle_boutique?: string | null; installe?: boolean | null; image?: string | null }[],
  jeuxPc: { installation?: { boutique?: { source: string; cle: string } | null } | null }[],
): CarteAInstallerTaodbox[] {
  const duPc = new Set(jeuxPc.map((j) => j.installation?.boutique).filter((b) => !!b).map((b) => `${b!.source}_${b!.cle}`));
  const vus = new Set<string>();
  return cartes
    .filter((c) => c.id < 0 && c.installe === false && !!c.cle_boutique && !!c.boutique && c.source === c.boutique && INSTALLABLES_AU_CANAPE.includes(c.boutique))
    .map((c) => ({ id: c.id, cle: cleAInstaller(c), nom: c.titre, boutique: c.boutique!, image: c.image ?? null }))
    .filter((c) => !duPc.has(c.cle) && !vus.has(c.cle) && !!vus.add(c.cle))
    .sort((a, b) => a.nom.localeCompare(b.nom, 'fr'));
}
