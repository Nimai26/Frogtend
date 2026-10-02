// GOG Galaxy (lot 9) : Galaxy regroupe GOG et les boutiques que la personne y a reliées (Epic, Xbox, Ubisoft, EA,
// Steam…). Frogtend lit sa base en LECTURE SEULE, sur une copie (accord de Seb, 02/10) ; jouer et installer passent
// par Galaxy. Aucun secret : Galaxy est déjà connecté sur ce PC.
import { api, type JeuBoutique } from '$lib/api';
import { toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';

export const galaxy = $state<{ installe: boolean; jeux: JeuBoutique[]; enCours: boolean }>({ installe: false, jeux: [], enCours: false });

/** Le libellé d'une boutique d'après le préfixe Galaxy (même table que le cœur). */
export function nomPlateforme(p: string): string {
  return ({ gog: 'GOG', steam: 'Steam', epic: 'Epic Games', xboxone: 'Xbox', xbox: 'Xbox', uplay: 'Ubisoft Connect', origin: 'EA', ea: 'EA', amazon: 'Amazon', battlenet: 'Battle.net' } as Record<string, string>)[p] ?? 'Autre';
}

export async function lireGalaxy() {
  galaxy.installe = (await api.galaxyEtat().catch(() => null))?.galaxy_installe ?? false;
  galaxy.jeux = await api.galaxyJeux().catch(() => []);
}

export async function importerGalaxy() {
  galaxy.enCours = true;
  try {
    galaxy.jeux = await api.galaxyImporter();
    toast(`✅ ${galaxy.jeux.length} jeu(x) lus dans GOG Galaxy.`);
  } catch (e) {
    toast(`Import GOG Galaxy impossible : ${motifDuRefus(e)}`, 'erreur');
  } finally {
    galaxy.enCours = false;
  }
}

/** Ouvre le jeu dans GOG Galaxy (sa page : jouer, installer). */
export async function ouvrirDansGalaxy(j: JeuBoutique) {
  try {
    await api.galaxyOuvrir(j.id);
    toast(`GOG Galaxy ouvre « ${j.nom} ».`);
  } catch (e) {
    toast(`Impossible : ${motifDuRefus(e)}`, 'erreur');
  }
}
