// Les contenus additionnels d'un jeu (DLC, avatars, thèmes, mises à jour) — règle de Seb (03/10) : détectés, gérés,
// proposés, et installés seulement si la personne qui utilise Frogtend dit oui, sur SON PC.

export interface ContenuVu {
  id: string;
  nom: string;
  genre: 'dlc' | 'avatar' | 'theme' | 'maj' | 'autre' | string;
  taille: number;
  licence: boolean;
  installe: boolean;
  /** Switch : le dossier qu'Eden lira (tout son contenu devient visible). */
  dossier?: string | null;
  /** Une mise à jour PS3 : sa version (« 01.05 », contrat 1.31), dans l'ordre d'installation. */
  version?: string | null;
}

/** Les mises à jour s'installent DANS L'ORDRE (la chaîne de Sony) : cocher une mise à jour coche aussi les précédentes
 * pas encore installées ; décocher une mise à jour décoche les suivantes. `id` : celle que la personne vient de changer. */
export function cochesCoherentes(contenus: ContenuVu[], coches: string[], id: string): string[] {
  const majs = contenus.filter((c) => c.version);
  const i = majs.findIndex((c) => c.id === id);
  if (i < 0) return coches;
  if (coches.includes(id)) {
    const avant = majs.slice(0, i).filter((c) => !c.installe && !coches.includes(c.id)).map((c) => c.id);
    return [...coches, ...avant];
  }
  const apres = new Set(majs.slice(i + 1).map((c) => c.id));
  return coches.filter((c) => !apres.has(c));
}

export interface ContenusDuJeu {
  etat: 'aucun' | 'emulateur' | 'ok';
  systeme?: 'ps3' | 'switch' | 'wiiu';
  titre_id: string | null;
  contenus: ContenuVu[];
  /** Les fichiers servis comme contenus mais illisibles (en-tête abîmé) : dits, jamais écartés en silence. */
  ignores?: string[];
}

const GENRES: Record<string, string> = { dlc: 'DLC', avatar: 'Avatar', theme: 'Thème', maj: 'Mise à jour', licence: 'Licence', autre: 'Contenu' };

export function libelleGenre(g: string): string {
  return GENRES[g] ?? 'Contenu';
}

/** Le nom affiché : sans le titre du jeu répété ni les étiquettes de région. */
export function nomCourt(nom: string, titreDuJeu: string): string {
  const sansEtiquettes = nom.replace(/\s*[([][^)\]]*[)\]]/g, '').trim();
  const parties = sansEtiquettes.split(' - ');
  const simple = (t: string) => t.toLowerCase().replace(/[^a-z0-9]/g, '');
  if (parties.length > 1 && simple(titreDuJeu).startsWith(simple(parties[0]))) return parties.slice(1).join(' - ');
  return sansEtiquettes;
}

/** Ce qu'on dit en résumé : « 7 disponibles, 2 installés ». */
export function resume(l: ContenuVu[]): string {
  const faits = l.filter((c) => c.installe).length;
  return faits ? `${l.length} disponible(s), ${faits} installé(s)` : `${l.length} disponible(s)`;
}
