// Les contenus additionnels d'un jeu (DLC, avatars, thèmes, mises à jour) — règle de Seb (03/10) : détectés, gérés,
// proposés, et installés seulement si la personne qui utilise Frogtend dit oui, sur SON PC.

export interface ContenuVu {
  id: string;
  nom: string;
  genre: 'dlc' | 'avatar' | 'theme' | 'maj' | 'autre' | string;
  taille: number;
  licence: boolean;
  installe: boolean;
}

export interface ContenusDuJeu {
  etat: 'aucun' | 'emulateur' | 'ok';
  titre_id: string | null;
  contenus: ContenuVu[];
}

const GENRES: Record<string, string> = { dlc: 'DLC', avatar: 'Avatar', theme: 'Thème', maj: 'Mise à jour', autre: 'Contenu' };

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
