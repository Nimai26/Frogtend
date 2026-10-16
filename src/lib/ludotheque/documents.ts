// Les documents d'un jeu (contrat 1.7) : ce qui se montre sur la fiche et dans le menu en jeu, et comment on le dit.
import { taille, type DocumentJeu } from '$lib/api';

/** Une durée lisible par tout le monde : « 2 min 32 », « 45 s ». */
export function duree(secondes: number): string {
  const s = Math.round(secondes);
  if (s < 60) return `${s} s`;
  const m = Math.floor(s / 60);
  const r = s % 60;
  return r ? `${m} min ${String(r).padStart(2, '0')}` : `${m} min`;
}

/** Les documents montrés aujourd'hui : le livret (la musique et les vidéos viendront avec leur lecteur ; la vidéo
 * de thème est pour Taodbox). */
export function documentsMontres(l: DocumentJeu[]): DocumentJeu[] {
  return l.filter((d) => d.type === 'livret');
}

/** Le libellé d'un document : sa sorte, puis ce qui aide à choisir (pages, durée), puis son poids s'il faut encore
 * le télécharger (le clic vaut alors accord : la taille est dite AVANT). */
export function libelleDocument(d: DocumentJeu): { titre: string; detail: string } {
  const titre = { livret: '📖 Livret (scan)', musique: '🎵 Musique', video: '🎬 Vidéo', video_theme: '🎬 Vidéo de thème' }[d.type] ?? '📎 Document';
  const parties: string[] = [];
  if (d.pages) parties.push(`${d.pages} page${d.pages > 1 ? 's' : ''}`);
  if (d.duree_s) parties.push(duree(d.duree_s));
  if (d.langue) parties.push(d.langue.toUpperCase());
  if (!d.garde && d.taille) parties.push(taille(d.taille));
  return { titre, detail: parties.join(' · ') };
}
