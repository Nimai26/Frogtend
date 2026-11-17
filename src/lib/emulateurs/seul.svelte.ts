// « ▶ Ouvrir l'émulateur seul » (✅ Seb 16-17/11) : un émulateur du système d'un jeu, ouvert SANS jeu, avec le profil
// de la personne (ses parties, sa manette, sa liste de jeux), pour régler ses DLC, sa manette, ses réglages. Jamais en
// Taodbox (son interface se pilote à la souris). Un seul ouvrable : ouvert tout de suite ; plusieurs : le choix (⭐
// celui du jeu). Pendant qu'il est ouvert, aucune partie ne se lance ; à sa fermeture, la sauvegarde automatique des
// parties a lieu comme après une partie (aucun temps n'est compté à un jeu).

import { listen } from '@tauri-apps/api/event';
import { isTauri } from '@tauri-apps/api/core';
import { api } from '$lib/api';
import { choisir, toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { etat } from '$lib/etat.svelte';
import { apresUnePartie } from '$lib/sauvegarde.svelte';
import { seulementAuBureau } from '$lib/taodbox/canape';
import { normaliser, pourLeJeu } from './choix';
import { ouvrablesSeuls, type EtatSeul } from './seul';

/** `ouvert` : le nom de l'émulateur ouvert seul (aucune partie pendant ce temps) ; `ouverture` : le jeu dont le bouton
 * a demandé l'ouverture (« Lancement… » sur ce bouton). */
export const emulateurSeul = $state<EtatSeul>({ ouvert: null, ouverture: null });

/** Le message quand aucun émulateur du système ne s'ouvre seul. */
export const PAS_ENCORE = 'Frogtend ne sait pas encore ouvrir cet émulateur seul, sans jeu. Lance-le par un jeu, avec ▶ Jouer.';

/** Les émulateurs du système que Frogtend sait ouvrir seuls. */
export function emulateursOuvrables(plateforme: string) {
  return ouvrablesSeuls(normaliser(etat.pc.emulateurs[plateforme]).liste);
}

/** Ouvre seul un émulateur du système `plateforme` (depuis la fiche du jeu `id`). Rend vrai s'il s'ouvre. */
export async function ouvrirSeul(id: number, plateforme: string): Promise<boolean> {
  if (await seulementAuBureau('Ouvrir un émulateur seul')) return false;
  if (emulateurSeul.ouvert !== null || emulateurSeul.ouverture !== null) return false;
  const liste = emulateursOuvrables(plateforme);
  if (!liste.length) {
    toast(PAS_ENCORE, 'alerte');
    return false;
  }
  let cle: string | null = liste[0].cle;
  if (liste.length > 1) {
    const actuel = pourLeJeu(etat.pc.emulateurs, etat.pc.emulateursJeux, plateforme, id);
    cle = await choisir<string>(
      '▶ Ouvrir quel émulateur, sans jeu ?',
      liste.map((e) => ({
        valeur: e.cle,
        libelle: `${e.cle === actuel?.cle ? '⭐ ' : ''}${e.nom || e.programme}`,
        detail: e.cle === actuel?.cle ? 'celui d’habitude pour ce jeu' : undefined,
      })),
      'Il s’ouvre avec ton profil, sans jeu : pour régler ses DLC, sa manette, ses réglages. Ferme-le pour rejouer.',
    );
    if (!cle) return false;
  }
  emulateurSeul.ouverture = id;
  try {
    const nom = await api.emulateurOuvrirSeul(plateforme, cle);
    toast(`▶ ${nom} s’ouvre…`);
    return true;
  } catch (e) {
    toast(motifDuRefus(e), 'erreur');
    return false;
  } finally {
    emulateurSeul.ouverture = null;
  }
}

let ecoute = false;

/** Écoute l'ouverture et la fermeture de l'émulateur ouvert seul (une seule fois), après avoir relu l'état du cœur :
 * après un rechargement de la page, ▶ Jouer reste bloqué tant qu'il est ouvert. */
export async function suivreEmulateurSeul() {
  if (ecoute || !isTauri()) return;
  ecoute = true;
  await listen<{ sorte: 'debut' | 'fin'; nom: string; secondes?: number }>('emulateur-seul', async (e) => {
    await surEvenement(e.payload);
  });
  await relireEtat();
}

/** L'émulateur ouvert seul, relu chez le cœur (celui qui tient le verrou). */
export async function relireEtat() {
  const nom = await api.emulateurSeulEtat().catch(() => null);
  emulateurSeul.ouvert = nom ?? null;
}

/** L'ouverture ou la fermeture de l'émulateur ouvert seul (à part des parties : pas d'écran de lancement, pas de
 * « Partie terminée », pas d'annonce de session). */
export async function surEvenement(e: { sorte: 'debut' | 'fin'; nom: string }) {
  if (e.sorte === 'debut') {
    emulateurSeul.ouvert = e.nom;
    return;
  }
  emulateurSeul.ouvert = null;
  toast(`🕹 ${e.nom} est fermé : tu peux rejouer.`);
  // Comme après une partie : les parties (et réglages) de la personne à l'abri chez Firehouse, si elle l'a choisi.
  await apresUnePartie();
}
