// Lot S7 (Seb, 24/11) : à l'ouverture du profil, le cœur RATTRAPE les parties des jeux PC jouées hors de Frogtend
// (Steam, Epic, un raccourci…) et envoie `parties-rattrapage` (toujours, même interrupteur éteint). L'interface le DIT :
// - un toast court « 🛟 N jeu(x) rattrapé(s) » ; rien si rien de neuf ;
// - plusieurs profils Frogtend sur ce compte Windows : « 🛟 C'est toi qui as joué à « … » ? » (fenêtre maison, focus
//   d'office sur « Décider plus tard »), groupé au-delà de 3 jeux (même forme que la reprise du lot S5). « Oui » : l'abri
//   part chez Firehouse avec la sauvegarde ; « Non » : il reste sur ce PC, jamais envoyé ; « Décider plus tard » :
//   reposé à l'ouverture suivante (dit par un toast).
// - JAMAIS pendant une partie : la question attend la fin de la partie (le toast, lui, passe).
// - Les titres viennent des jeux du PC (une fois chargés) ou de la fiche ; jamais « ce jeu » : un jeu sans titre connu
//   attend la prochaine fois.
// Un événement d'un autre profil que celui ouvert est ignoré (jamais montré à quelqu'un d'autre).
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { api, type BilanRattrapage } from '$lib/api';
import { choisir, demanderSiCestToi, toast, type ReponseCestToi } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { partie } from '$lib/ludotheque/jeu.svelte';
import { tele } from '$lib/ludotheque/telechargements.svelte';
import { signalerAbri } from './abriAuto.svelte';
import { pl, titresCites } from './reprise';

/** Au-delà, une question groupée d'abord (comme les conflits du lot S5). */
export const SEUIL_GROUPE_RATTRAPAGE = 3;
/** Le toast d'un « Décider plus tard ». */
export const TEXTE_PLUS_TARD = '🛟 Je te le redemanderai à la prochaine ouverture de Frogtend.';
/** L'attente des jeux du PC (pour leurs titres) : bornée. */
export const ATTENTE_TITRES_MS = 30_000;

/** Le toast d'un tour de rattrapage (`null` : rien de neuf, rien à dire). */
export function texteToastRattrapage(n: number): string | null {
  return n > 0 ? `🛟 ${n} ${pl(n, 'jeu rattrapé', 'jeux rattrapés')}` : null;
}

const MESSAGE =
  'Frogtend a trouvé de nouvelles parties, jouées en dehors de Frogtend. Plusieurs profils utilisent ce PC : si c’est toi qui as joué, elles partiront chez Firehouse avec ta sauvegarde. Sinon, elles restent à l’abri sur ce PC, sans jamais être envoyées.';

/** Le profil qui attend sa question (une partie était en cours), ou `null`. */
let enAttente: string | null = null;
let enCours = false;

const pause = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** Les jeux du PC sont chargés (borné : `ATTENTE_TITRES_MS`). */
async function attendreTitres(): Promise<void> {
  const debut = Date.now();
  while (!tele.charge && Date.now() - debut < ATTENTE_TITRES_MS) await pause(200);
}

/** Le titre d'un jeu : celui des jeux du PC, sinon celui de sa fiche ; `null` : inconnu. */
export async function titreDuJeu(jeu: number): Promise<string | null> {
  const t = tele.jeux[jeu]?.titre;
  if (t) return t;
  try {
    return (await api.fiche(jeu)).fiche.titre || null;
  } catch {
    return null;
  }
}

/** Pose la question pour ces jeux (un par un, ou groupée au-delà de 3). « Décider plus tard » arrête la série et le dit.
 * `profil` : celui à qui la question est posée (le cœur refuse une réponse pour un autre). Rend le nombre répondu. */
export async function proposerConfirmations(jeux: number[], profil: string): Promise<number> {
  if (!jeux.length || enCours) return 0;
  enCours = true;
  try {
    await attendreTitres();
    const titres = new Map<number, string>();
    for (const j of jeux) {
      const t = await titreDuJeu(j);
      if (t) titres.set(j, t);
    }
    // Sans titre connu, jamais « ce jeu » : la question attend la prochaine ouverture.
    const connus = jeux.filter((j) => titres.has(j));
    if (!connus.length) return 0;
    let groupe: Exclude<ReponseCestToi, 'plus_tard'> | 'une' = 'une';
    if (connus.length > SEUIL_GROUPE_RATTRAPAGE) {
      const liste = titresCites(connus.map((j) => titres.get(j)!));
      const c = await choisir<ReponseCestToi | 'une'>(
        `🛟 C’est toi qui as joué à ces ${connus.length} jeux ?`,
        [
          { valeur: 'plus_tard', libelle: '⏳ Décider plus tard', detail: 'rien ne part chez Firehouse' },
          { valeur: 'une', libelle: '🔎 Répondre un par un' },
          { valeur: 'oui', libelle: '✅ Oui, c’est moi pour tous', detail: 'ils partiront avec ta sauvegarde' },
          { valeur: 'non', libelle: '🚫 Non, aucun', detail: 'gardés sur ce PC, jamais envoyés' },
        ],
        `${MESSAGE}\n\nLes jeux : ${liste}.`,
      );
      if (!c || c === 'plus_tard') {
        toast(TEXTE_PLUS_TARD, 'neutre');
        return 0;
      }
      groupe = c;
    }
    let repondus = 0;
    for (const jeu of connus) {
      const r = groupe === 'une' ? await demanderSiCestToi(`🛟 C’est toi qui as joué à « ${titres.get(jeu)} » ?`, { message: MESSAGE }) : groupe;
      if (r === 'plus_tard') {
        toast(TEXTE_PLUS_TARD, 'neutre');
        break;
      }
      try {
        await api.partiesConfirmer(jeu, r === 'oui', profil);
        repondus++;
        signalerAbri(jeu);
      } catch (e) {
        toast(`Impossible d’enregistrer ta réponse pour « ${titres.get(jeu)} » : ${motifDuRefus(e)}`, 'erreur');
      }
    }
    return repondus;
  } finally {
    enCours = false;
  }
}

/** Les abris à confirmer de ce profil (relus chez le cœur), proposés tout de suite — ou à la fin de la partie en cours. */
export async function reproposer(profil: string, fallback: number[] = []): Promise<number> {
  if (partie.enJeu !== null) {
    enAttente = profil;
    return 0;
  }
  let jeux = fallback;
  try {
    jeux = await api.partiesAConfirmer();
  } catch {
    // Illisible : la liste reçue avec le tour.
  }
  return proposerConfirmations(jeux, profil);
}

/** La partie est finie : la question mise en attente est posée, si le même profil est toujours ouvert. */
export async function apresPartie(profilOuvert: string | null | undefined): Promise<number> {
  if (partie.enJeu !== null || !enAttente) return 0;
  const p = enAttente;
  enAttente = null;
  if (p !== profilOuvert) return 0;
  return reproposer(p);
}

/** Ce que fait l'interface d'un tour de rattrapage (séparé de l'écoute, pour les tests). `profil` : celui ouvert. */
export async function recevoirRattrapage(b: BilanRattrapage, profil: string | null | undefined): Promise<void> {
  if (!profil || b.profil !== profil) return;
  for (const jeu of b.rattrapes) signalerAbri(jeu);
  const t = texteToastRattrapage(b.rattrapes.length);
  if (t) toast(t, 'ok');
  await reproposer(profil, b.a_confirmer);
}

/** Pour les tests. */
export function remettreAZeroRattrapage() {
  enAttente = null;
  enCours = false;
}

let ecoute: (() => void) | null = null;

/** Écoute les tours de rattrapage (une seule fois, dès le montage de l'application) et la fin des parties. */
export async function ecouterRattrapage(profil: () => string | null | undefined) {
  if (ecoute || !isTauri()) return;
  ecoute = await listen<BilanRattrapage>('parties-rattrapage', (e) => void recevoirRattrapage(e.payload, profil()));
  $effect.root(() => {
    $effect(() => {
      if (partie.enJeu === null) void apresPartie(profil());
    });
  });
}
