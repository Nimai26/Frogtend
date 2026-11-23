// Lot S3 (Seb, 23/11) : l'abri AUTOMATIQUE après une partie se fait dans le cœur (calme d'au moins 60 s, 10 minutes au
// plus) ; l'interface le DIT, discrètement (modèle : la sauvegarde après une partie) :
// - réussi : un toast court, sans chiffres ni chemin ; « rien de neuf » : aucun message (la ligne d'un échec part) ;
// - pendant la copie : un message de travail seulement si elle dure plus de 2 s (15 minutes au plus) ;
// - un problème : une ligne qui RESTE sur la fiche du jeu (BlocParties, gardée aussi par le cœur : elle survit au
//   redémarrage) ; un toast d'alerte seulement quand la personne peut y faire quelque chose (dans le nuage, trop gros,
//   échec) — jamais pour une attente dépassée ni un emplacement inconnu (« abandonner sans bruit inutile ») ; « dans ton
//   nuage OneDrive » une seule fois par jeu et par profil pendant une ouverture de Frogtend.
// Un événement d'un autre profil que celui ouvert est ignoré (jamais montré à quelqu'un d'autre).
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { api, type BilanAbriAuto } from '$lib/api';
import { fermerToast, toast, toastProgres } from '$lib/dialogues/fenetres.svelte';
import { estProblemeAbriAuto, texteToastAbriAuto } from './parties';

/** Le dernier abri automatique de chaque jeu pour le profil ouvert (la ligne de la fiche) ; `version` change à chaque
 * abri (fait, rien de neuf, problème) : BlocParties relit alors la liste. */
export const abrisAuto = $state<{ derniers: Record<number, BilanAbriAuto>; version: number }>({ derniers: {}, version: 0 });

/** Au-delà, la copie montre un message de travail. */
export const DELAI_ATTENTE_MS = 2000;
/** Le message de travail ne reste jamais plus longtemps (un compte rendu perdu). */
export const ATTENTE_MAX_MS = 15 * 60 * 1000;

let ecoute: (() => void) | null = null;
/** « profil:jeu » déjà prévenus que des parties sont seulement dans le nuage (pendant cette ouverture). */
const nuageDit = new Set<string>();
const attentes = new Map<number, { minuteur: ReturnType<typeof setTimeout>; fin: ReturnType<typeof setTimeout>; toast: number | null }>();

function finirAttente(jeu: number) {
  const a = attentes.get(jeu);
  if (!a) return;
  clearTimeout(a.minuteur);
  clearTimeout(a.fin);
  if (a.toast !== null) fermerToast(a.toast);
  attentes.delete(jeu);
}

/** Un abri de ce jeu vient d'être fait (après une partie, ou avant une opération depuis cette fenêtre) : la ligne d'un
 * ancien problème part et la liste des abris se relit. */
export function signalerAbri(jeu: number) {
  delete abrisAuto.derniers[jeu];
  abrisAuto.version++;
}

/** Ce que fait l'interface d'un compte rendu (séparé de l'écoute, pour les tests). `profil` : celui ouvert ;
 * `titre` : le titre du jeu, lu dans la ludothèque de CE profil. */
export function recevoirAbriAuto(b: BilanAbriAuto, profil: string | null | undefined, titre: (jeu: number) => string) {
  if (!profil || b.profil !== profil) return;
  if (b.etat === 'en_cours') {
    if (attentes.has(b.jeu)) return;
    const jeu = b.jeu;
    const a = {
      minuteur: setTimeout(() => (a.toast = toastProgres(`🛟 Copie des parties de « ${titre(jeu)} »…`)), DELAI_ATTENTE_MS),
      fin: setTimeout(() => finirAttente(jeu), ATTENTE_MAX_MS),
      toast: null as number | null,
    };
    attentes.set(jeu, a);
    return;
  }
  finirAttente(b.jeu);
  if (b.etat === 'aucun') return;
  if (b.etat === 'rien_de_neuf') {
    signalerAbri(b.jeu);
    return;
  }
  if (estProblemeAbriAuto(b)) abrisAuto.derniers[b.jeu] = b;
  else delete abrisAuto.derniers[b.jeu];
  abrisAuto.version++;
  // Un abri d'avant une opération : la fenêtre de l'opération le dit elle-même.
  if (b.declencheur !== 'apres_partie') return;
  if (b.etat === 'en_ligne') {
    const cle = `${b.profil}:${b.jeu}`;
    if (nuageDit.has(cle)) return;
    nuageDit.add(cle);
  }
  const t = texteToastAbriAuto(b, titre(b.jeu));
  if (t) toast(t, estProblemeAbriAuto(b) ? 'alerte' : 'ok');
}

/** M6 : la ligne gardée par le cœur pour ce jeu (après un redémarrage), si l'interface n'en sait pas plus récent. */
export async function chargerLigneGardee(jeu: number) {
  if (abrisAuto.derniers[jeu]) return;
  let b: BilanAbriAuto | null = null;
  try {
    b = await api.partiesAbriAutoEtat(jeu);
  } catch {
    // Illisible (hors de l'application…) : la ligne n'est qu'un plus.
  }
  if (b && !abrisAuto.derniers[jeu]) abrisAuto.derniers[jeu] = b;
}

/** Un nouveau profil ouvert : ce qui avait été reçu pour l'autre n'est plus montré. */
export function oublierAbrisAuto() {
  for (const j of [...attentes.keys()]) finirAttente(j);
  abrisAuto.derniers = {};
  abrisAuto.version++;
}

/** Pour les tests : « dans le nuage » redevient à dire. */
export function remettreAZeroAbrisAuto() {
  oublierAbrisAuto();
  nuageDit.clear();
}

/** Écoute les abris automatiques (une seule fois, dès le montage de l'application). */
export async function ecouterAbrisAuto(profil: () => string | null | undefined, titre: (jeu: number) => string) {
  if (ecoute || !isTauri()) return;
  ecoute = await listen<BilanAbriAuto>('parties-abri-auto', (e) => recevoirAbriAuto(e.payload, profil(), titre));
}
