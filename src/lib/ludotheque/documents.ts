// Les documents d'un jeu (contrat 1.7) : ce qui se montre sur la fiche et dans le menu en jeu, et comment on le dit.
import { api, taille, type DocumentJeu } from '$lib/api';

/** Un document ouvert : le livret (ses pages), ou une musique ou une vidéo. */
export type DocumentOuvert = { doc: number; livret: true; pages: number } | { doc: number; livret: false; video: boolean };

/** Ouvre un document (téléchargé d'abord s'il n'est pas sur le PC : sa taille est sur le bouton, le clic vaut accord). */
export async function ouvrirDocument(jeu: number, d: DocumentJeu): Promise<DocumentOuvert> {
  if (d.type === 'livret') return { doc: d.id, livret: true, pages: await api.ouvrirLivret(jeu, d.id) };
  await api.preparerDocument(jeu, d.id);
  return { doc: d.id, livret: false, video: d.type !== 'musique' };
}

/** Une durée lisible par tout le monde : « 2 min 32 », « 45 s ». */
export function duree(secondes: number): string {
  const s = Math.round(secondes);
  if (s < 60) return `${s} s`;
  const m = Math.floor(s / 60);
  const r = s % 60;
  return r ? `${m} min ${String(r).padStart(2, '0')}` : `${m} min`;
}

/** Les documents montrés : livret, musique, vidéo, dans cet ordre (la vidéo de thème est pour Taodbox, plus tard). */
export function documentsMontres(l: DocumentJeu[]): DocumentJeu[] {
  const ordre = ['livret', 'musique', 'video'];
  return l.filter((d) => ordre.includes(d.type)).sort((a, b) => ordre.indexOf(a.type) - ordre.indexOf(b.type));
}

/** Les documents du menu en jeu (décision de Seb, 16/10 : « mieux de couper musique et vidéo pendant le jeu ») : la
 * musique et la vidéo seulement si le jeu est vraiment en pause ; le livret toujours. */
export function documentsDuMenu(l: DocumentJeu[], enPause: boolean): DocumentJeu[] {
  return enPause ? l : l.filter((d) => d.type === 'livret');
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
