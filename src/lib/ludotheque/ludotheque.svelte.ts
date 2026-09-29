// L'état de l'écran Ludothèque : plateforme choisie, recherche, filtres, liste affichée, jeu sélectionné.

import { listen } from '@tauri-apps/api/event';
import { isTauri } from '@tauri-apps/api/core';
import { api, estErreurCoeur, type JeuResume, type Plateforme } from '$lib/api';
import { informer, toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { etat } from '$lib/etat.svelte';

/** Nombre de jeux chargés à la fois dans la grille. */
export const TAILLE_LOT = 240;
/** Au-delà, la ludothèque est resynchronisée à l'ouverture du profil. */
export const SYNCHRO_PERIMEE_S = 12 * 3600;

export const ludo = $state<{
  plateformes: Plateforme[];
  genres: string[];
  plateforme: string | null;
  texte: string;
  genre: string | null;
  jeux: JeuResume[];
  total: number;
  totalLudotheque: number;
  chargement: boolean;
  selection: JeuResume | null;
  synchro: { enCours: boolean; page: number; jeux: number };
  synchroniseeLe: number | null;
}>({
  plateformes: [],
  genres: [],
  plateforme: null,
  texte: '',
  genre: null,
  jeux: [],
  total: 0,
  totalLudotheque: 0,
  chargement: false,
  selection: null,
  synchro: { enCours: false, page: 0, jeux: 0 },
  synchroniseeLe: null,
});

/** Remet l'écran à zéro (changement de profil : rien du précédent ne doit rester affiché). */
export function viderLudotheque() {
  Object.assign(ludo, {
    plateformes: [],
    genres: [],
    plateforme: null,
    texte: '',
    genre: null,
    jeux: [],
    total: 0,
    totalLudotheque: 0,
    selection: null,
    synchroniseeLe: null,
  });
}

let numeroRequete = 0;

/** Recharge la liste selon les filtres (depuis le cache local : rapide, et marche hors ligne). */
export async function rechargerListe() {
  const n = ++numeroRequete;
  ludo.chargement = true;
  try {
    const l = await api.lister({
      plateforme: ludo.plateforme,
      texte: ludo.texte || null,
      genre: ludo.genre,
      tri: etat.profil.ludotheque.tri,
      limite: TAILLE_LOT,
      decalage: 0,
    });
    if (n !== numeroRequete) return; // une frappe plus récente a déjà relancé la recherche
    ludo.jeux = l.jeux;
    ludo.total = l.total;
    ludo.totalLudotheque = l.total_ludotheque;
    if (ludo.selection && !l.jeux.some((j) => j.id === ludo.selection!.id)) ludo.selection = null;
  } catch (e) {
    toast(`Impossible de lire la ludothèque : ${motifDuRefus(e)}`, 'erreur');
  } finally {
    if (n === numeroRequete) ludo.chargement = false;
  }
}

/** Charge le lot suivant (défilement vers le bas). */
export async function chargerSuite() {
  if (ludo.chargement || ludo.jeux.length >= ludo.total) return;
  ludo.chargement = true;
  try {
    const l = await api.lister({
      plateforme: ludo.plateforme,
      texte: ludo.texte || null,
      genre: ludo.genre,
      tri: etat.profil.ludotheque.tri,
      limite: TAILLE_LOT,
      decalage: ludo.jeux.length,
    });
    ludo.jeux = [...ludo.jeux, ...l.jeux];
  } finally {
    ludo.chargement = false;
  }
}

export async function rechargerPlateformes() {
  ludo.plateformes = await api.plateformes();
  ludo.genres = await api.genres(ludo.plateforme);
  const le = await api.synchroniseeLe();
  ludo.synchroniseeLe = le ? Number(le) : null;
}

export async function choisirPlateforme(nom: string | null) {
  ludo.plateforme = nom;
  ludo.genre = null;
  ludo.selection = null;
  ludo.genres = await api.genres(nom).catch(() => []);
  await rechargerListe();
}

/**
 * Synchronise la ludothèque avec Firehouse. Un jeton refusé est dit clairement, une seule fois, sans nouvel essai.
 * `discrete` : pas de message si tout va bien (synchronisation automatique à l'ouverture).
 */
export async function synchroniser(discrete = false) {
  if (ludo.synchro.enCours) return;
  ludo.synchro = { enCours: true, page: 0, jeux: 0 };
  const arreter = isTauri()
    ? await listen<{ page: number; jeux: number }>('synchro', (e) => {
        ludo.synchro.page = e.payload.page;
        ludo.synchro.jeux = e.payload.jeux;
      })
    : () => {};
  try {
    const b = await api.synchroniser();
    await rechargerPlateformes();
    await rechargerListe();
    if (!discrete) {
      toast(`✅ Ludothèque à jour : ${b.jeux.toLocaleString('fr-FR')} jeux sur ${b.plateformes} plateformes.`);
    }
  } catch (e) {
    if (estErreurCoeur(e) && e.sorte === 'jeton_refuse') {
      await informer('⛔ Jeton refusé', `${e.motif}\n\nTa ludothèque déjà téléchargée reste consultable.`);
    } else if (estErreurCoeur(e) && e.sorte === 'reseau') {
      toast(`⚠ Hors ligne : ${e.motif} La ludothèque affichée est celle de la dernière synchronisation.`, 'alerte');
    } else {
      toast(`Échec de la synchronisation : ${motifDuRefus(e)}`, 'erreur');
    }
  } finally {
    arreter();
    ludo.synchro.enCours = false;
  }
}

/** À l'ouverture de l'écran : le cache tout de suite, puis une synchronisation s'il est vide ou ancien. */
export async function ouvrirLudotheque() {
  try {
    await rechargerPlateformes();
    await rechargerListe();
  } catch (e) {
    toast(`Impossible de lire la ludothèque : ${motifDuRefus(e)}`, 'erreur');
  }
  const age = ludo.synchroniseeLe ? Date.now() / 1000 - ludo.synchroniseeLe : Infinity;
  if (ludo.totalLudotheque === 0 || age > SYNCHRO_PERIMEE_S) await synchroniser(ludo.totalLudotheque > 0);
}

export async function jeuAuHasard() {
  const j = await api.auHasard(ludo.plateforme);
  if (!j) {
    toast('Aucun jeu ici pour l’instant.', 'alerte');
    return null;
  }
  ludo.selection = j;
  return j;
}

/** « il y a 3 h », « il y a 2 jours »… */
export function depuis(secondes: number | null): string {
  if (!secondes) return 'jamais';
  const d = Math.max(0, Date.now() / 1000 - secondes);
  if (d < 60) return 'à l’instant';
  if (d < 3600) return `il y a ${Math.floor(d / 60)} min`;
  if (d < 86400) return `il y a ${Math.floor(d / 3600)} h`;
  const j = Math.floor(d / 86400);
  return `il y a ${j} jour${j > 1 ? 's' : ''}`;
}
