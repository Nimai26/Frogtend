// 🏆 Les succès d'un jeu en détail (lot C, Seb 21/10) : le même affichage pour Steam, GOG Galaxy et RetroAchievements.

/** Un succès, tel que le cœur le rend (`SuccesDetail`). */
export interface SuccesDetail {
  id: string;
  nom: string;
  description: string;
  /** Le texte n'existe qu'en anglais. */
  en_anglais: boolean;
  obtenu: boolean;
  /** Secondes depuis 1970. */
  quand: number | null;
  /** Part des joueurs qui l'ont, en %. */
  rarete: number | null;
  icone: string | null;
  icone_grise: string | null;
  /** Description cachée tant qu'il n'est pas obtenu (« Montrer » à la demande). */
  secret: boolean;
}

export interface ListeSucces {
  source: 'steam' | 'galaxy' | 'retroachievements';
  succes: SuccesDetail[];
}

/** Obtenus d'abord (les plus récents en tête), puis les autres du plus courant au plus rare (le prochain à viser). */
export function trier(l: SuccesDetail[]): SuccesDetail[] {
  return [...l].sort((a, b) => {
    if (a.obtenu !== b.obtenu) return a.obtenu ? -1 : 1;
    if (a.obtenu) return (b.quand ?? 0) - (a.quand ?? 0);
    return (b.rarete ?? -1) - (a.rarete ?? -1);
  });
}

/** La rareté en mots, pour un novice (« 3 % des joueurs : très rare »). */
export function libelleRarete(pct: number | null): string | null {
  if (pct === null || !Number.isFinite(pct)) return null;
  const p = pct < 1 ? pct.toLocaleString('fr-FR', { maximumFractionDigits: 1 }) : Math.round(pct).toString();
  const mot = pct < 5 ? 'très rare' : pct < 15 ? 'rare' : pct < 40 ? 'peu courant' : 'courant';
  return `${p} % des joueurs : ${mot}`;
}

/** La date d'obtention, en français. */
export function dateObtenu(s: number | null): string | null {
  if (!s) return null;
  return new Date(s * 1000).toLocaleDateString('fr-FR', { day: 'numeric', month: 'long', year: 'numeric' });
}

/** « 29 / 34 obtenus ». */
export function compte(l: SuccesDetail[]): [number, number] {
  return [l.filter((s) => s.obtenu).length, l.length];
}

/** D'où vient la liste, en mots. */
export function libelleSource(s: ListeSucces['source']): string {
  return s === 'steam' ? 'Steam' : s === 'galaxy' ? 'GOG Galaxy' : 'RetroAchievements';
}
