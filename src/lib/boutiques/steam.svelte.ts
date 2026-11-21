// Steam (lot 9) : le compte se règle dans ⚙ Options ▸ Comptes ▸ Steam, OU au moment d'un import si rien n'est encore
// réglé (règle de Seb, comme LaunchBox). La clé d'API ne s'affiche jamais : on dit seulement si elle est enregistrée.
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { api, type EtatSteam, type ImportSteam, type JeuBoutique, type OubliSteam } from '$lib/api';
import { confirmer, demander, fermerToast, informer, toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { etat } from '$lib/etat.svelte';
import { partie } from '$lib/ludotheque/jeu.svelte';
import { rafraichirLudotheque } from './boutiques-pc.svelte';
import { messageNouveaux, toujoursLeMeme } from './jeu-boutique';

export const PAGE_CLE_STEAM = 'https://steamcommunity.com/dev/apikey';

export const steam = $state<{
  etat: EtatSteam | null;
  jeux: JeuBoutique[];
  enCours: boolean;
  connexionEnCours: boolean;
  oubliEnCours: boolean;
  erreur: string | null;
  expire: boolean;
  erreurProfil: string | null;
  profilImport: string | null;
  imports: number;
}>({
  etat: null,
  jeux: [],
  // Un import par la connexion tourne (pour `profilImport` : voir `importSteamEnCours`).
  enCours: false,
  connexionEnCours: false,
  // « Oublier ma connexion Steam » en cours (lot C3 : la déconnexion du navigateur prend quelques secondes).
  oubliEnCours: false,
  // La dernière erreur d'import (ligne d'état du bloc « Mon compte Steam ») et la connexion expirée, pour le profil
  // `erreurProfil` seulement ; `null` / faux après un import réussi.
  erreur: null,
  expire: false,
  erreurProfil: null,
  profilImport: null,
  // Le nombre d'imports Steam terminés (Taodbox relit alors ses cartes « à installer »).
  imports: 0,
});

/** Un import Steam tourne-t-il pour le profil OUVERT ? (Celui d'un autre profil ne bloque jamais ses boutons.) */
export function importSteamEnCours(): boolean {
  return steam.enCours && steam.profilImport === (etat.profilOuvert?.id ?? null);
}

/** La connexion du profil ouvert à Steam a-t-elle expiré (dernier import) ? */
export function connexionSteamExpiree(): boolean {
  return steam.expire && steam.erreurProfil === (etat.profilOuvert?.id ?? null);
}

/** La connexion du profil à Steam a expiré (lot C2) : dit sans alarmer, les jeux déjà importés restent. */
export const MESSAGE_STEAM_EXPIRE = '🔑 Ta connexion à Steam a expiré : dans 🛒 Boutiques, reconnecte-toi.';

/** Le message d'une connexion expirée selon l'écran (pur, testé) : « dans 🛒 Boutiques » seulement hors de cette page,
 * et « au bureau » depuis Taodbox (la connexion demande la souris). */
export function messageSteamExpire(chemin: string): string {
  if (chemin.startsWith('/taodbox')) return '🔑 Ta connexion à Steam a expiré : au bureau, dans 🛒 Boutiques, reconnecte-toi.';
  if (chemin.startsWith('/boutiques')) return '🔑 Ta connexion à Steam a expiré : reconnecte-toi (bouton « 🔑 Me reconnecter à Steam »).';
  return MESSAGE_STEAM_EXPIRE;
}

/** L'écran où l'on est (pour les messages). */
function cheminActuel(): string {
  try {
    return typeof location === 'undefined' ? '' : location.pathname;
  } catch {
    return '';
  }
}

/** Ce qu'un import Steam par la connexion fait dire (pur, testé). À la main : toujours un message (« Rien de nouveau »
 * compris) ; à l'ouverture de Frogtend (discret) : seulement des jeux nouveaux ou une connexion expirée. */
export function annonceImportSteam(r: ImportSteam, aLaMain: boolean, chemin = ''): { texte: string; ton?: 'alerte' } | null {
  if (r.reconnecter) return { texte: messageSteamExpire(chemin), ton: 'alerte' };
  const m = messageNouveaux([['steam', r.nouveaux, r.nouveaux_installes]]);
  if (m) return { texte: m };
  return aLaMain ? { texte: 'Rien de nouveau : tes jeux Steam sont déjà dans ta ludothèque.' } : null;
}

/** La connexion expirée est-elle à dire AUJOURD'HUI à ce profil, à l'ouverture de Frogtend ? Une fois par jour
 * (la ligne d'état reste visible) ; retenu dans ce navigateur seulement, sans rien de secret. Sans stockage : dit. */
export function direExpireAujourdhui(profil: string, jour: string, stockage: Pick<Storage, 'getItem' | 'setItem'> | null): boolean {
  const cle = `frogtend.steamExpire.${profil}`;
  try {
    if (stockage?.getItem(cle) === jour) return false;
    stockage?.setItem(cle, jour);
  } catch {
    // Stockage refusé : dit, comme sans stockage.
  }
  return true;
}

function stockageLocal(): Storage | null {
  try {
    return typeof localStorage === 'undefined' ? null : localStorage;
  } catch {
    return null;
  }
}

/** La manière de lancer un import : à la main (bouton), juste après la connexion, ou à l'ouverture de Frogtend. */
export type ModeImportSteam = 'main' | 'connexion' | 'discret';

/** L'import en cours (une seule promesse à la fois) et le profil pour lequel il tourne. */
let courant: { profil: string | null; promesse: Promise<boolean> } | null = null;

/** Met à jour les jeux Steam POSSÉDÉS du compte connecté par la page (lot C2), sans clé d'API : à la main (bouton,
 * juste après la connexion) avec un message d'attente qui reste jusqu'à la fin, ou discrètement à l'ouverture de
 * Frogtend. Un import déjà en cours : le discret du même profil le rejoint ; les autres attendent sa fin PUIS relancent.
 * Le profil changé pendant l'attente : silence. La grille est relue après chaque import. Rend vrai si la liste a été
 * mise à jour. */
export async function mettreAJourSteam(mode: ModeImportSteam = 'main'): Promise<boolean> {
  const profil = etat.profilOuvert?.id ?? null;
  if (courant) {
    if (mode === 'discret' && courant.profil === profil) return courant.promesse;
    await courant.promesse.catch(() => false);
    return mettreAJourSteam(mode);
  }
  const promesse = importerConnecte(mode, profil);
  courant = { profil, promesse };
  try {
    return await promesse;
  } finally {
    if (courant?.promesse === promesse) courant = null;
  }
}

async function importerConnecte(mode: ModeImportSteam, profil: string | null): Promise<boolean> {
  // À l'ouverture : revérifié juste avant d'ouvrir la fenêtre cachée (une partie a pu commencer entre-temps).
  if (mode === 'discret' && partie.enJeu !== null) return false;
  const aLaMain = mode !== 'discret';
  steam.enCours = true;
  steam.profilImport = profil;
  const attente = aLaMain
    ? toast(mode === 'connexion' ? '✅ Connecté à Steam. ⏳ Lecture de tes jeux…' : '⏳ Lecture de tes jeux Steam… (jusqu’à une minute)', undefined, { jusquaFermeture: true })
    : null;
  try {
    const r = await api.steamImporterConnecte();
    if (!toujoursLeMeme(profil, etat.profilOuvert?.id)) return false;
    steam.erreurProfil = profil;
    steam.expire = r.reconnecter;
    steam.erreur = r.reconnecter ? messageSteamExpire('/boutiques') : null;
    if (attente !== null) fermerToast(attente);
    const a = annonceImportSteam(r, aLaMain, cheminActuel());
    // Jamais un message par-dessus une partie (à l'ouverture de Frogtend) ; la connexion expirée, une fois par jour.
    const dire = a && (aLaMain || (partie.enJeu === null && (!r.reconnecter || direExpireAujourdhui(profil ?? '', new Date().toISOString().slice(0, 10), stockageLocal()))));
    if (dire) toast(a.texte, a.ton);
    if (r.reconnecter) return false;
    steam.imports++;
    await rafraichirLudotheque();
    await lireSteam();
    return true;
  } catch (err) {
    if (!toujoursLeMeme(profil, etat.profilOuvert?.id)) return false;
    steam.erreurProfil = profil;
    steam.expire = false;
    steam.erreur = `Mise à jour de tes jeux Steam impossible : ${motifDuRefus(err)}`;
    if (aLaMain) toast(steam.erreur, 'erreur');
    return false;
  } finally {
    if (attente !== null) fermerToast(attente);
    if (steam.profilImport === profil) {
      steam.enCours = false;
      steam.profilImport = null;
    }
  }
}

/** Oublie l'état d'import d'un AUTRE profil que celui ouvert (`toujours` : même le sien, nouvelle ouverture). */
function oublierEtatImport(toujours: boolean) {
  if (toujours || steam.erreurProfil !== (etat.profilOuvert?.id ?? null)) {
    steam.erreur = null;
    steam.expire = false;
    steam.erreurProfil = null;
  }
}

/** À chaque ouverture de Frogtend (décision de Seb du 20/11), en tâche de fond : si le profil est connecté à Steam par
 * la page, ses jeux possédés sont relus. Jamais pendant une partie. Une erreur reste dans la ligne d'état du bloc
 * Steam, sans toast (seule une connexion expirée est dite, une fois par jour). */
export async function steamAuDemarrage() {
  oublierEtatImport(true);
  if (partie.enJeu !== null) return;
  const profil = etat.profilOuvert?.id;
  const e = await api.steamEtat().catch(() => null);
  if (!connecteASteam(e) || !toujoursLeMeme(profil, etat.profilOuvert?.id)) return;
  steam.etat = e;
  await mettreAJourSteam('discret');
}

export async function lireSteam() {
  oublierEtatImport(false);
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
    if (connecteASteam(steam.etat)) {
      // Lot C2 : les jeux du compte entrent tout de suite (se connecter vaut accord, Seb 19/11) ; un seul message
      // « ✅ Connecté à Steam. ⏳ Lecture de tes jeux… ».
      await mettreAJourSteam('connexion');
    } else toast('⚠ Steam a dit « connecté », mais Frogtend ne le voit pas : réessaie.', 'alerte');
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

/** Importer la liste des jeux Steam. Connecté par la page (lot C2) : sans clé d'API, jamais demandée. Sinon (secours)
 * par la clé : si le compte n'est pas réglé, on le règle d'abord. */
export async function importerSteam(): Promise<boolean> {
  let e = await api.steamEtat().catch(() => null);
  if (connecteASteam(e)) {
    steam.etat = e;
    return mettreAJourSteam('main');
  }
  if (!e?.compte || !e.cle_enregistree) {
    await informer('🎮 Ton compte Steam d’abord', 'Pour importer tes jeux Steam, Frogtend a besoin de ton compte et de ta clé d’API Steam. On les règle maintenant (tu les retrouveras dans ⚙ Options ▸ Comptes ▸ Steam).');
    if (!(await reglerSteam())) return false;
    e = steam.etat;
  }
  if (importSteamEnCours()) return false;
  const profil = etat.profilOuvert?.id ?? null;
  steam.enCours = true;
  steam.profilImport = profil;
  try {
    steam.jeux = await api.steamImporter();
    steam.etat = await api.steamEtat().catch(() => steam.etat);
    steam.erreur = null;
    steam.erreurProfil = profil;
    toast(`✅ ${steam.jeux.length} jeu(x) Steam dans ta liste.`);
    steam.imports++;
    // La grille et la barre des systèmes suivent (les cartes viennent d'être rangées par le cœur).
    await rafraichirLudotheque();
    return true;
  } catch (err) {
    steam.erreur = `Import Steam impossible : ${motifDuRefus(err)}`;
    steam.erreurProfil = profil;
    toast(steam.erreur, 'erreur');
    return false;
  } finally {
    if (steam.profilImport === profil) {
      steam.enCours = false;
      steam.profilImport = null;
    }
  }
}

/** Le message de fin de « Oublier » (lot C3, pur, testé) : le compte est TOUJOURS oublié ; la déconnexion de Steam
 * dans Frogtend n'est dite sûre que prouvée par le cœur. */
export function messageOubliSteam(r: OubliSteam): { texte: string; ton?: 'alerte' } {
  if (r.navigateur_deconnecte) return { texte: '✅ Compte Steam oublié. Tu peux te connecter avec un autre compte dans 🛒 Boutiques.' };
  return {
    texte: 'Compte Steam oublié. La déconnexion de Steam dans Frogtend n’est pas sûre : à ta prochaine connexion, si Steam affiche encore l’ancien compte, clique sur « Se déconnecter » en haut à droite de sa page.',
    ton: 'alerte',
  };
}

export async function oublierSteam() {
  if (steam.oubliEnCours) return;
  const oui = await confirmer('Oublier ton compte Steam ?', {
    message:
      'Frogtend oublie ton compte Steam (et ta clé d’API, si tu en avais une) et te déconnecte de Steam dans Frogtend (pas dans l’application Steam). Tes jeux restent dans ta ludothèque, et tes jeux installés ne bougent pas. Ensuite, tu pourras te connecter avec un autre compte.',
    libelleValider: 'Oublier',
    danger: true,
  });
  if (!oui) return;
  const profil = etat.profilOuvert?.id ?? '';
  steam.oubliEnCours = true;
  try {
    const r = await api.steamOublier();
    // Plus de « connexion expirée » ni d'erreur d'import qui survivrait à l'oubli, ni de rappel du jour retenu.
    oublierEtatImport(true);
    try {
      stockageLocal()?.removeItem(`frogtend.steamExpire.${profil}`);
    } catch {
      // Stockage refusé : rien à retirer.
    }
    await lireSteam();
    // Taodbox relit ses cartes (les jeux restent ; leur état peut changer).
    steam.imports++;
    const m = messageOubliSteam(r);
    toast(m.texte, m.ton);
  } catch (err) {
    toast(`Impossible : ${motifDuRefus(err)}`, 'erreur');
  } finally {
    steam.oubliEnCours = false;
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
