// La galerie des captures d'écran (✅ Seb 06/11) : celles que Frogtend a rangées pendant les parties du profil OUVERT
// (chaque profil ne voit que les siennes). Les images sont servies par le cœur (`capture.localhost/<id>`), jamais
// celles d'un autre profil.

/** Une capture de la galerie (`captures_liste`). */
export interface Capture {
  id: string;
  jeu: number;
  titre: string;
  plateforme: string;
  /** Secondes depuis 1970. */
  quand: number;
  /** Le jeu a une fiche Firehouse : la capture peut en devenir le fond. */
  fond_possible: boolean;
  /** C'est le fond choisi de son jeu. */
  est_le_fond: boolean;
}

/** L'image d'une capture (servie par le cœur pour le profil ouvert). */
export function adresseCapture(id: string): string {
  return `http://capture.localhost/${encodeURIComponent(id)}`;
}

/** « le 4 novembre 2026 à 16:13 ». */
export function dateCapture(quand: number): string {
  const d = new Date(quand * 1000);
  return `le ${d.toLocaleDateString('fr-FR', { day: 'numeric', month: 'long', year: 'numeric' })} à ${d.toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })}`;
}

/** Les captures regroupées par console puis par jeu, dans l'ordre de la liste (plus récentes d'abord). */
export function parConsole(l: Capture[]): { plateforme: string; jeux: { jeu: number; titre: string; captures: Capture[] }[] }[] {
  const consoles = new Map<string, Map<number, { jeu: number; titre: string; captures: Capture[] }>>();
  for (const c of l) {
    let jeux = consoles.get(c.plateforme);
    if (!jeux) consoles.set(c.plateforme, (jeux = new Map()));
    let j = jeux.get(c.jeu);
    if (!j) jeux.set(c.jeu, (j = { jeu: c.jeu, titre: c.titre, captures: [] }));
    j.captures.push(c);
  }
  return [...consoles].map(([plateforme, jeux]) => ({ plateforme, jeux: [...jeux.values()] }));
}

/** Le délai du diaporama (millisecondes). */
export const DIAPORAMA_MS = 5000;
