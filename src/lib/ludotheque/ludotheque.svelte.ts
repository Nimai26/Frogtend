// L'état des deux espaces de jeux : « Ma ludothèque » (les jeux du PC) et « Catalogue Firehouse » (tout ce que
// Firehouse montre à ce profil, d'où l'on ajoute). Plateforme choisie, recherche, filtres, liste, jeu sélectionné.

import { listen } from '@tauri-apps/api/event';
import { isTauri } from '@tauri-apps/api/core';
import { api, estErreurCoeur, type JeuResume, type Plateforme } from '$lib/api';
import { informer, toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { jeuSurPc, rechargerJeuxDuPc } from './telechargements.svelte';
import { etat, suivreReunions } from '$lib/etat.svelte';
import { retenirNomsCourts } from './nomsCourts.svelte';

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
    // La ligne choisie a disparu : un jeu importé qu'on vient de relier se montre désormais sous sa fiche (contrat 1.21) —
    // la sélection la suit, pour ne jamais perdre sa place au clavier ou à la manette.
    // La carte choisie est toujours relue (une carte réunie, A6c, change de copies et de temps) ; une copie désormais
    // réunie sous une autre carte la suit.
    if (ludo.selection) {
      const ancien = ludo.selection.id;
      ludo.selection =
        l.jeux.find((j) => j.id === ancien) ?? l.jeux.find((j) => jeuSurPc(j.id)?.id === ancien || !!j.copies?.some((c) => c.id === ancien)) ?? null;
    }
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

// Les noms courts des plateformes : module à part (oubliés à la sortie du profil, voir nomsCourts.svelte.ts).
export { nomsCourts, nomAffiche } from './nomsCourts.svelte';

/** Les noms courts : TOUJOURS les mêmes listes réunies (ma ludothèque et le catalogue), d'où qu'on les charge
 * (ludothèque, Taodbox) — une console porte le même nom partout. Une liste illisible ne vide jamais la table. `lue` :
 * une des deux déjà lue (`locale` dit laquelle), pas redemandée. */
export async function chargerNomsCourts(lue?: { locale: boolean; liste: Plateforme[] }) {
  const lire = (l: boolean) => (lue && lue.locale === l ? Promise.resolve(lue.liste) : api.plateformes(l).catch(() => null));
  const [locales, catalogue] = await Promise.all([lire(true), lire(false)]);
  if (locales && catalogue) retenirNomsCourts(locales, catalogue);
}

export async function rechargerPlateformes() {
  ludo.plateformes = await api.plateformes(locale());
  // Les noms courts de tout le catalogue quand même (un jeu importé d'un autre système).
  await chargerNomsCourts({ locale: locale(), liste: ludo.plateformes });
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
    // Des jeux du PC rattachés à leur nouvelle fiche (contrat 1.14) : leurs réglages suivent, la liste est relue.
    if (b.rattaches?.length) {
      await suivreReunions();
      await rechargerJeuxDuPc();
      toast(`🔗 ${b.rattaches.length} jeu(x) de ton PC rattaché(s) à leur nouvelle fiche chez Firehouse (rien n’a été retéléchargé).`);
    }
    await rechargerPlateformes();
    await rechargerListe();
    // Fonds et logos de Taodbox gardés à l'avance pour le hors ligne (lot Médias M4), en arrière-plan : jamais une
    // raison d'échouer la synchronisation.
    // Les fiches des jeux importés reliés, pour les voir hors ligne (par petits passages, en arrière-plan) ; puis les
    // fonds et logos de Taodbox (les imports reliés déjà joués y sont).
    if (!b.simule) {
      void api
        .prelireFichesReliees()
        .catch(() => 0)
        .then(() => (etat.profil.ludotheque.fondsTaodbox ? api.prechargerTaodbox() : 0))
        .catch(() => {});
    }
    if (!discrete) {
      const detail =
        b.mode === 'increment'
          ? `${b.recus} changement(s)${b.retires ? `, ${b.retires} retiré(s)` : ''}`
          : `relu en entier`;
      toast(`✅ Catalogue Firehouse à jour (${detail}) : ${b.jeux.toLocaleString('fr-FR')} jeux.`);
    }
  } catch (e) {
    if (estErreurCoeur(e) && e.sorte === 'perimee') {
      // Lancée avec l'ancien jeton ou un autre profil : arrêtée exprès, rien n'a été gardé ; pas un échec.
    } else if (estErreurCoeur(e) && e.sorte === 'version') {
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
