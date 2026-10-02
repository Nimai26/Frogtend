// L'état des deux espaces de jeux : « Ma ludothèque » (les jeux du PC) et « Catalogue Firehouse » (tout ce que
// Firehouse montre à ce profil, d'où l'on ajoute). Plateforme choisie, recherche, filtres, liste, jeu sélectionné.

import { listen } from '@tauri-apps/api/event';
import { isTauri } from '@tauri-apps/api/core';
import { api, estErreurCoeur, type JeuResume, type Plateforme } from '$lib/api';
import { informer, toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { etat } from '$lib/etat.svelte';

/** Nombre de jeux chargés à la fois dans la grille. */
export const TAILLE_LOT = 240;
/** Au-delà, le catalogue est resynchronisé à l'ouverture. */
export const SYNCHRO_PERIMEE_S = 12 * 3600;

export type Espace = 'ludotheque' | 'catalogue';

export const ludo = $state<{
  espace: Espace;
  plateformes: Plateforme[];
  genres: string[];
  plateforme: string | null;
  texte: string;
  genre: string | null;
  /** Filtre de la ludothèque : une boutique (lot 9), installé ou non. */
  boutique: string | null;
  installe: boolean | null;
  jeux: JeuResume[];
  total: number;
  /** Nombre de jeux de l'espace, sans filtre. */
  totalLudotheque: number;
  chargement: boolean;
  selection: JeuResume | null;
  synchro: { enCours: boolean; page: number; jeux: number };
  synchroniseeLe: number | null;
}>({
  espace: 'ludotheque',
  plateformes: [],
  genres: [],
  plateforme: null,
  texte: '',
  genre: null,
  boutique: null,
  installe: null,
  jeux: [],
  total: 0,
  totalLudotheque: 0,
  chargement: false,
  selection: null,
  synchro: { enCours: false, page: 0, jeux: 0 },
  synchroniseeLe: null,
});

const locale = () => ludo.espace === 'ludotheque';

/** Remet l'écran à zéro (changement de profil : rien du précédent ne doit rester affiché). */
export function viderLudotheque() {
  Object.assign(ludo, {
    plateformes: [],
    genres: [],
    plateforme: null,
    texte: '',
    genre: null,
    boutique: null,
    installe: null,
    jeux: [],
    total: 0,
    totalLudotheque: 0,
    selection: null,
    synchroniseeLe: null,
  });
}

let numeroRequete = 0;

function filtre(decalage: number) {
  return {
    plateforme: ludo.plateforme,
    texte: ludo.texte || null,
    genre: ludo.genre,
    tri: etat.profil.ludotheque.tri,
    limite: TAILLE_LOT,
    decalage,
    ludotheque: locale(),
    boutique: locale() ? ludo.boutique : null,
    installe: locale() ? ludo.installe : null,
  };
}

/** Recharge la liste selon les filtres (depuis le disque : rapide, et marche hors ligne). */
export async function rechargerListe() {
  const n = ++numeroRequete;
  ludo.chargement = true;
  try {
    const l = await api.lister(filtre(0));
    if (n !== numeroRequete) return; // une frappe plus récente a déjà relancé la recherche
    ludo.jeux = l.jeux;
    ludo.total = l.total;
    ludo.totalLudotheque = l.total_ludotheque;
    if (ludo.selection && !l.jeux.some((j) => j.id === ludo.selection!.id)) ludo.selection = null;
  } catch (e) {
    toast(`Impossible de lire la liste des jeux : ${motifDuRefus(e)}`, 'erreur');
  } finally {
    if (n === numeroRequete) ludo.chargement = false;
  }
}

/** Charge le lot suivant (défilement vers le bas). */
export async function chargerSuite() {
  if (ludo.chargement || ludo.jeux.length >= ludo.total) return;
  ludo.chargement = true;
  try {
    const l = await api.lister(filtre(ludo.jeux.length));
    ludo.jeux = [...ludo.jeux, ...l.jeux];
  } finally {
    ludo.chargement = false;
  }
}

export async function rechargerPlateformes() {
  ludo.plateformes = await api.plateformes(locale());
  ludo.genres = await api.genres(ludo.plateforme, locale());
  const le = await api.synchroniseeLe();
  ludo.synchroniseeLe = le ? Number(le) : null;
}

export async function choisirPlateforme(nom: string | null) {
  ludo.plateforme = nom;
  ludo.genre = null;
  ludo.selection = null;
  ludo.genres = await api.genres(nom, locale()).catch(() => []);
  await rechargerListe();
}

/**
 * Synchronise le catalogue du profil avec Firehouse. Un jeton refusé est dit clairement, une seule fois, sans
 * nouvel essai. `discrete` : pas de message si tout va bien (synchronisation automatique à l'ouverture).
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
      const detail =
        b.mode === 'increment'
          ? `${b.recus} changement(s)${b.retires ? `, ${b.retires} retiré(s)` : ''}`
          : `relu en entier`;
      toast(`✅ Catalogue Firehouse à jour (${detail}) : ${b.jeux.toLocaleString('fr-FR')} jeux.`);
    }
  } catch (e) {
    if (estErreurCoeur(e) && e.sorte === 'version') {
      await informer('⚠ Versions incompatibles', e.motif);
    } else if (estErreurCoeur(e) && e.sorte === 'jeton_refuse') {
      await informer('⛔ Jeton refusé', `${e.motif}\n\nTa ludothèque reste utilisable.`);
    } else if (estErreurCoeur(e) && e.sorte === 'reseau') {
      if (!discrete) toast(`⚠ Hors ligne : ${e.motif} Ta ludothèque reste utilisable.`, 'alerte');
    } else {
      toast(`Échec de la synchronisation : ${motifDuRefus(e)}`, 'erreur');
    }
  } finally {
    arreter();
    ludo.synchro.enCours = false;
  }
}

/**
 * Ouvre un espace : ce qui est sur le disque tout de suite, puis une synchronisation du catalogue s'il n'a jamais
 * été lu ou s'il est ancien (sans rien dire si Internet manque : la ludothèque marche sans).
 */
export async function ouvrirEspace(espace: Espace) {
  if (ludo.espace !== espace) {
    ludo.espace = espace;
    Object.assign(ludo, { plateforme: null, genre: null, texte: '', selection: null, jeux: [], total: 0 });
  }
  try {
    await rechargerPlateformes();
    await rechargerListe();
  } catch (e) {
    toast(`Impossible de lire la liste des jeux : ${motifDuRefus(e)}`, 'erreur');
  }
  const age = ludo.synchroniseeLe ? Date.now() / 1000 - ludo.synchroniseeLe : Infinity;
  if (age > SYNCHRO_PERIMEE_S) await synchroniser(ludo.synchroniseeLe !== null);
}

export async function jeuAuHasard() {
  const j = await api.auHasard(ludo.plateforme, locale());
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
