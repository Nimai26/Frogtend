// Steam (lot 9) : le compte se règle dans ⚙ Options ▸ Comptes ▸ Steam, OU au moment d'un import si rien n'est encore
// réglé (règle de Seb, comme LaunchBox). La clé d'API ne s'affiche jamais : on dit seulement si elle est enregistrée.
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { api, type EtatSteam, type JeuBoutique } from '$lib/api';
import { confirmer, demander, informer, toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { etat } from '$lib/etat.svelte';
import { toujoursLeMeme } from './jeu-boutique';

export const PAGE_CLE_STEAM = 'https://steamcommunity.com/dev/apikey';

export const steam = $state<{ etat: EtatSteam | null; jeux: JeuBoutique[]; enCours: boolean; connexionEnCours: boolean }>({
  etat: null,
  jeux: [],
  enCours: false,
  connexionEnCours: false,
});

export async function lireSteam() {
  steam.etat = await api.steamEtat().catch(() => null);
  steam.jeux = await api.steamJeux().catch(() => []);
}

/** Connecté à Steam par sa page officielle (lot C1) ? Pur, testé : seul ce que dit le cœur compte. */
export function connecteASteam(e: EtatSteam | null | undefined): boolean {
  return e?.connecte === true;
}

/** L'annonce du cœur quand la fenêtre de Steam a vu le compte connecté (pure, testée) : vraie seulement pour le
 * profil qui a ouvert la fenêtre ET qui est toujours ouvert. */
export function annoncePourMoi(p: { profil?: unknown; ok?: unknown } | null | undefined, avant: string | null | undefined, maintenant: string | null | undefined): boolean {
  return !!p && typeof p.profil === 'string' && p.profil === avant && toujoursLeMeme(avant, maintenant);
}

/** Ce que le bloc « Secours » dit du compte Steam (pur, testé) : il tient compte de la connexion par la page, et dit
 * clairement quand la clé d'API rangée est celle d'un AUTRE compte (elle n'est pas effacée pour autant). */
export function libelleCompteSecours(e: EtatSteam | null | undefined): string {
  if (e?.cle_autre_compte)
    return '⚠ ta clé d’API est celle d’un autre compte que celui connecté : l’import par la clé est bloqué (règle ta clé pour ce compte dans ⚙ Options ▸ Comptes ▸ Steam ; elle n’est pas effacée)';
  if (connecteASteam(e)) return e?.cle_enregistree ? 'connecté à Steam · clé d’API enregistrée' : 'connecté à Steam · sans clé d’API';
  return e?.compte ? `compte ${e.compte}` : 'compte pas encore réglé';
}

/** L'écoute de la fenêtre de connexion à Steam (une seule à la fois) et le profil pour lequel elle écoute. */
let arreterEcoute: (() => void) | null = null;
let ecouteDe: string | null = null;

function oublierEcoute() {
  arreterEcoute?.();
  arreterEcoute = null;
  ecouteDe = null;
}

/** Écoute l'annonce du cœur pour `profil` ; une écoute déjà en place pour lui est GARDÉE (la fenêtre peut être
 * ouverte). Rend vrai si une écoute vient d'être créée. */
async function ecouter(profil: string): Promise<boolean> {
  if (arreterEcoute && ecouteDe === profil) return false;
  oublierEcoute();
  ecouteDe = profil;
  arreterEcoute = await listen<{ profil: string; ok: boolean }>('steam-connecte', async (e) => {
    if (!annoncePourMoi(e.payload, profil, etat.profilOuvert?.id)) return;
    oublierEcoute();
    if (!e.payload.ok) {
      toast('⚠ Connecté à Steam, mais Frogtend n’a pas pu le retenir : réessaie.', 'erreur');
      return;
    }
    // Relu : l'écran montre ce que le cœur a vraiment rangé.
    try {
      steam.etat = await api.steamEtat();
    } catch (err) {
      toast(`Connecté à Steam, mais l’état n’a pas pu être relu (${motifDuRefus(err)}) : rouvre cette page.`, 'alerte');
      return;
    }
    if (connecteASteam(steam.etat)) toast('✅ Connecté à Steam.');
    else toast('⚠ Steam a dit « connecté », mais Frogtend ne le voit pas : réessaie.', 'alerte');
  });
  return true;
}

/** Se connecter à Steam (ou changer de compte) : la page officielle s'ouvre dans une fenêtre de Frogtend ; la personne
 * s'y connecte elle-même (Steam Guard compris). Frogtend ne voit jamais le mot de passe : il garde seulement le numéro
 * du compte (steamid). La fenêtre se ferme toute seule une fois connecté. */
export async function connexionSteam() {
  if (steam.connexionEnCours) return;
  steam.connexionEnCours = true;
  let nouvelle = false;
  try {
    const profil = etat.profilOuvert?.id;
    if (isTauri() && profil) nouvelle = await ecouter(profil);
    await api.steamConnexion();
    toast('🔑 Connecte-toi à Steam dans la fenêtre qui s’ouvre (elle se fermera toute seule). Frogtend ne voit jamais ton mot de passe.');
  } catch (err) {
    // Seule l'écoute créée par CE clic est retirée : celle d'une fenêtre déjà ouverte reste.
    if (nouvelle) oublierEcoute();
    toast(`Impossible d’ouvrir Steam : ${motifDuRefus(err)}`, 'erreur');
  } finally {
    steam.connexionEnCours = false;
  }
}

/** Régler (ou corriger) le compte Steam du profil. Rend vrai si c'est réglé. */
export async function reglerSteam(): Promise<boolean> {
  const e = await api.steamEtat().catch(() => null);
  const compte = await demander('🎮 Ton compte Steam', {
    message:
      'L’URL de ton profil Steam, ou seulement son nom (par exemple « https://steamcommunity.com/id/MonNom »).\nPour la trouver : Steam ▸ ton nom en haut ▸ Profil ▸ Modifier le profil.',
    valeur: e?.compte ?? '',
    libelleValider: 'Suivant',
  });
  if (!compte) return false;
  const cle = await demander('🔑 Ta clé d’API Steam', {
    message: [
      `Steam ne donne la liste de tes jeux qu’avec TA clé d’API (gratuite). Crée-la sur ${PAGE_CLE_STEAM} (connecté à Steam ; nom de domaine : mets « localhost »).`,
      e?.cle_enregistree ? 'Une clé est déjà enregistrée : laisse vide pour la garder.' : '',
      'Elle ira dans le coffre de Windows : Frogtend ne l’affichera plus jamais.',
    ]
      .filter(Boolean)
      .join('\n'),
    masque: true,
    libelleValider: 'Vérifier et enregistrer',
  });
  if (cle === null) return false;
  try {
    steam.etat = await api.steamRegler(compte, cle || undefined);
    toast('✅ Compte Steam vérifié et enregistré.');
    return true;
  } catch (err) {
    toast(`Compte Steam non enregistré : ${motifDuRefus(err)}`, 'erreur');
    return false;
  }
}

/** Importer la liste des jeux Steam : si le compte n'est pas réglé, on le règle d'abord. */
export async function importerSteam(): Promise<boolean> {
  let e = await api.steamEtat().catch(() => null);
  if (!e?.compte || !e.cle_enregistree) {
    await informer('🎮 Ton compte Steam d’abord', 'Pour importer tes jeux Steam, Frogtend a besoin de ton compte et de ta clé d’API Steam. On les règle maintenant (tu les retrouveras dans ⚙ Options ▸ Comptes ▸ Steam).');
    if (!(await reglerSteam())) return false;
    e = steam.etat;
  }
  steam.enCours = true;
  try {
    steam.jeux = await api.steamImporter();
    steam.etat = await api.steamEtat().catch(() => steam.etat);
    toast(`✅ ${steam.jeux.length} jeu(x) Steam dans ta liste.`);
    return true;
  } catch (err) {
    toast(`Import Steam impossible : ${motifDuRefus(err)}`, 'erreur');
    return false;
  } finally {
    steam.enCours = false;
  }
}

export async function oublierSteam() {
  const oui = await confirmer('Oublier ton compte Steam ?', {
    message: 'Ta clé quitte le coffre de Windows et la liste de tes jeux Steam est retirée de Frogtend. Rien ne change chez Steam ni dans tes jeux installés.',
    libelleValider: 'Oublier',
    danger: true,
  });
  if (!oui) return;
  try {
    await api.steamOublier();
    await lireSteam();
    toast('Compte Steam oublié.');
  } catch (err) {
    toast(`Impossible : ${motifDuRefus(err)}`, 'erreur');
  }
}

/** Jouer ou installer : Steam fait le travail. */
export async function ouvrirDansSteam(j: JeuBoutique, action: 'jouer' | 'installer') {
  try {
    await api.steamOuvrir(j.id, action);
    toast(action === 'jouer' ? `▶ Steam lance « ${j.nom} »…` : `⬇ Steam propose d’installer « ${j.nom} ».`);
  } catch (err) {
    toast(`Impossible : ${motifDuRefus(err)}`, 'erreur');
  }
}

export async function ouvrirPageCle() {
  const { openUrl } = await import('@tauri-apps/plugin-opener');
  await openUrl(PAGE_CLE_STEAM).catch(() => {});
}
