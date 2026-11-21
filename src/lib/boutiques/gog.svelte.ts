// Lot C4a (décisions de Seb du 21/11) : les jeux POSSÉDÉS du compte GOG connecté dans le navigateur du profil entrent
// dans la ludothèque, « pas installés », sans GOG Galaxy — sur le modèle de Steam (lot C2) : juste après la connexion,
// à chaque ouverture de Frogtend (discret : un message seulement s'il y a du nouveau ; « connexion expirée » une fois
// par jour, jamais pendant une partie) et à la main (« 🔄 Mettre à jour mes jeux GOG »). La connexion GOG déjà faite pour
// le jeu offert VAUT ACCORD : c'est le même état de connexion (`gratuits.connectes.gog`) pour les deux usages.
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { api, type EtatGog, type ImportGog } from '$lib/api';
import { fermerToast, toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { etat } from '$lib/etat.svelte';
import { partie } from '$lib/ludotheque/jeu.svelte';
import { rafraichirLudotheque } from './boutiques-pc.svelte';
import { gratuits, noterConnexion } from './gratuits.svelte';
import { annonceImport, aujourdhui, cheminActuel, direUneFoisParJour, messageExpire, stockageLocal } from './import-compte';
import { toujoursLeMeme } from './jeu-boutique';

export const gog = $state<{
  etat: EtatGog | null;
  enCours: boolean;
  connexionEnCours: boolean;
  erreur: string | null;
  expire: boolean;
  erreurProfil: string | null;
  profilImport: string | null;
  imports: number;
}>({
  etat: null,
  // Une lecture tourne (pour `profilImport` : voir `importGogEnCours`).
  enCours: false,
  // La fenêtre de connexion est en train de s'ouvrir (garde contre le double clic).
  connexionEnCours: false,
  // La dernière erreur de lecture (ligne d'état du bloc « Mon compte GOG ») et la connexion expirée, pour le profil
  // `erreurProfil` seulement ; `null` / faux après une lecture réussie.
  erreur: null,
  expire: false,
  erreurProfil: null,
  profilImport: null,
  // Le nombre de lectures GOG terminées (Taodbox relit alors ses cartes « à installer »).
  imports: 0,
});

/** Une lecture GOG tourne-t-elle pour le profil OUVERT ? (Celle d'un autre profil ne bloque jamais ses boutons.) */
export function importGogEnCours(): boolean {
  return gog.enCours && gog.profilImport === (etat.profilOuvert?.id ?? null);
}

/** Les boutons GOG (connexion, jeu offert) attendent : une lecture des jeux, la récupération du jeu offert ou
 * l'ouverture de la fenêtre de connexion est en cours. */
export function gogOccupe(): boolean {
  return importGogEnCours() || gratuits.boutique === 'gog' || gog.connexionEnCours;
}

/** La connexion du profil ouvert à GOG a-t-elle expiré (dernière lecture) ? */
export function connexionGogExpiree(): boolean {
  return gog.expire && gog.erreurProfil === (etat.profilOuvert?.id ?? null);
}

/** Le profil ouvert est-il connecté à GOG (dernier état connu, jeu offert ou lecture de ses jeux) ? */
export function connecteAGog(): boolean {
  return etat.profil.gratuits.connectes.gog === true;
}

/** La connexion du profil à GOG a expiré : dit sans alarmer, les jeux déjà importés restent. */
export const MESSAGE_GOG_EXPIRE = messageExpire('GOG', '');

/** Le message d'une connexion GOG expirée selon l'écran (« au bureau » depuis Taodbox). */
export function messageGogExpire(chemin: string): string {
  return messageExpire('GOG', chemin);
}

/** Juste après la fenêtre de connexion fermée, GOG ne montre pas le compte connecté (fenêtre fermée trop tôt). Le
 * bouton nommé est celui que le bloc affiche alors : « Me reconnecter » pour un compte déjà connu, sinon « Se
 * connecter » (pur, testé). */
export function messageGogPasConnecte(connu: boolean): string {
  return `🔑 GOG ne te montre pas connecté : clique « 🔑 ${connu ? 'Me reconnecter à GOG' : 'Se connecter à GOG'} », connecte-toi dans la fenêtre, puis ferme-la.`;
}

/** Sans GOG Galaxy sur ce PC, « ⬇ Installer » ne peut rien pour l'instant (lot C4c) : la fin du message des jeux
 * nouveaux le dit (pur, testé). */
export function sansGalaxy(texte: string): string {
  return texte.replace(/Il s’installe avec ⬇ Installer\.$/, 'Installe GOG Galaxy pour l’installer.').replace(/(Ils|Les autres) s’installent avec ⬇ Installer\.$/, 'Installe GOG Galaxy pour les installer.');
}

/** La manière de lancer une lecture : à la main (bouton), juste après la connexion, ou à l'ouverture de Frogtend. */
export type ModeImportGog = 'main' | 'connexion' | 'discret';

/** Ce qu'une lecture GOG fait dire (pur, testé) : comme Steam ; juste après la connexion, « ✅ Connecté à GOG » d'abord,
 * et un compte vu déconnecté n'est pas une connexion « expirée » ; `connu` : le compte était déjà connu (le bouton
 * affiché est alors « Me reconnecter »). Sans GOG Galaxy, rien ne promet « ⬇ Installer ». */
export function annonceImportGog(r: ImportGog, mode: ModeImportGog, chemin = '', connu = false): { texte: string; ton?: 'alerte' } | null {
  const fin = (a: { texte: string; ton?: 'alerte' } | null) => (a && !r.reconnecter && !r.galaxy ? { ...a, texte: sansGalaxy(a.texte) } : a);
  if (mode === 'connexion') {
    if (r.reconnecter) return { texte: messageGogPasConnecte(connu), ton: 'alerte' };
    const a = fin(annonceImport('gog', 'GOG', r, true, chemin));
    return a && { texte: `✅ Connecté à GOG. ${a.texte}` };
  }
  return fin(annonceImport('gog', 'GOG', r, mode === 'main', chemin));
}

/** La connexion GOG expirée est-elle à dire AUJOURD'HUI à ce profil, à l'ouverture de Frogtend ? Une fois par jour. */
export function direGogExpireAujourdhui(profil: string, jour: string, stockage: Pick<Storage, 'getItem' | 'setItem'> | null): boolean {
  return direUneFoisParJour(`frogtend.gogExpire.${profil}`, jour, stockage);
}

/** Attend la fin de la récupération du jeu offert de GOG (au plus `plafondMs`) ; faux si le profil a changé ou si elle
 * dure trop. */
async function attendreFinRecolte(profil: string | null, plafondMs = 6 * 60_000): Promise<boolean> {
  const fin = Date.now() + plafondMs;
  while (gratuits.boutique === 'gog') {
    if (Date.now() > fin || !toujoursLeMeme(profil, etat.profilOuvert?.id)) return false;
    await new Promise((ok) => setTimeout(ok, 500));
  }
  return toujoursLeMeme(profil, etat.profilOuvert?.id);
}

/** La lecture en cours (une seule promesse à la fois) et le profil pour lequel elle tourne. */
let courant: { profil: string | null; promesse: Promise<boolean> } | null = null;

/** Met à jour les jeux du compte GOG (lot C4a) : à la main ou juste après la connexion, avec un message d'attente qui
 * reste jusqu'à la fin ; discrètement à l'ouverture de Frogtend. Une lecture déjà en cours : la discrète du même profil
 * la rejoint ; les autres attendent sa fin PUIS relancent. Le profil changé pendant l'attente : silence. Rend vrai si la
 * liste a été mise à jour. */
export async function mettreAJourGog(mode: ModeImportGog = 'main'): Promise<boolean> {
  const profil = etat.profilOuvert?.id ?? null;
  if (courant) {
    if (mode === 'discret' && courant.profil === profil) return courant.promesse;
    await courant.promesse.catch(() => false);
    return mettreAJourGog(mode);
  }
  const promesse = importerConnecte(mode, profil);
  courant = { profil, promesse };
  try {
    return await promesse;
  } finally {
    if (courant?.promesse === promesse) courant = null;
  }
}

/** Ce qui suit une lecture (ou une récupération du jeu offert qui a vu le compte déconnecté) : l'état, le message. */
async function conclure(r: ImportGog, mode: ModeImportGog, profil: string | null, connuAvant: boolean) {
  gog.erreurProfil = profil;
  // « Expirée » seulement pour un compte qui était connecté (une première connexion pas finie n'a rien d'expiré).
  gog.expire = r.reconnecter && (mode !== 'connexion' || connuAvant);
  gog.erreur = gog.expire ? messageGogExpire('/boutiques') : null;
  const a = annonceImportGog(r, mode, cheminActuel(), connuAvant);
  // Jamais un message par-dessus une partie (à l'ouverture de Frogtend) ; la connexion expirée, une fois par jour.
  const dire = a && (mode !== 'discret' || (partie.enJeu === null && (!r.reconnecter || direGogExpireAujourdhui(profil ?? '', aujourdhui(), stockageLocal()))));
  if (dire) toast(a.texte, a.ton);
}

async function importerConnecte(mode: ModeImportGog, profil: string | null): Promise<boolean> {
  // À l'ouverture : revérifié juste avant d'ouvrir la fenêtre cachée (une partie a pu commencer entre-temps).
  if (mode === 'discret' && partie.enJeu !== null) return false;
  const aLaMain = mode !== 'discret';
  const connuAvant = connecteAGog() || gog.etat?.connu === true;
  // Jamais en même temps que la récupération du jeu offert de GOG (même compte, même fenêtre de GOG). Juste après une
  // connexion, on ATTEND sa fin (une première connexion doit importer) ; à la main, on le dit ; au démarrage, rien.
  if (gratuits.boutique === 'gog') {
    if (mode === 'connexion') {
      if (!(await attendreFinRecolte(profil))) return false;
    } else {
      if (aLaMain) toast('⏳ Frogtend récupère le jeu offert de GOG : réessaie dans un moment.', 'alerte');
      return false;
    }
  }
  // Au démarrage, la récupération du jeu offert vient de trouver le compte déconnecté (sa fenêtre est ouverte pour se
  // connecter) : c'est une connexion expirée, sans rien redemander à GOG.
  if (mode === 'discret' && gratuits.gogDeconnecte) {
    await conclure({ reconnecter: true, total: 0, nouveaux: 0, nouveaux_installes: 0, galaxy: true }, mode, profil, connuAvant);
    return false;
  }
  gog.enCours = true;
  gog.profilImport = profil;
  gratuits.lectureGog = true;
  const attente = aLaMain ? toast(mode === 'connexion' ? '⏳ Lecture de tes jeux GOG…' : '⏳ Lecture de tes jeux GOG… (jusqu’à une minute)', undefined, { jusquaFermeture: true }) : null;
  try {
    const r = await api.gogImporterConnecte();
    if (!toujoursLeMeme(profil, etat.profilOuvert?.id)) return false;
    // Un seul état de connexion GOG (jeu offert et jeux du compte) : ce que la lecture a vu le met à jour.
    await noterConnexion('gog', r.reconnecter ? 'connexion' : 'connecte', profil);
    if (!r.reconnecter) gratuits.gogDeconnecte = false;
    if (attente !== null) fermerToast(attente);
    await conclure(r, mode, profil, connuAvant);
    if (r.reconnecter) return false;
    gog.imports++;
    await rafraichirLudotheque();
    await lireGog();
    return true;
  } catch (err) {
    if (!toujoursLeMeme(profil, etat.profilOuvert?.id)) return false;
    gog.erreurProfil = profil;
    gog.expire = false;
    gog.erreur = `Mise à jour de tes jeux GOG impossible : ${motifDuRefus(err)}`;
    if (aLaMain) toast(gog.erreur, 'erreur');
    return false;
  } finally {
    if (attente !== null) fermerToast(attente);
    gratuits.lectureGog = false;
    if (gog.profilImport === profil) {
      gog.enCours = false;
      gog.profilImport = null;
    }
  }
}

/** Oublie l'état de lecture d'un AUTRE profil que celui ouvert (`toujours` : même le sien, nouvelle ouverture). */
function oublierEtatImport(toujours: boolean) {
  if (toujours || gog.erreurProfil !== (etat.profilOuvert?.id ?? null)) {
    gog.erreur = null;
    gog.expire = false;
    gog.erreurProfil = null;
  }
}

export async function lireGog() {
  oublierEtatImport(false);
  gog.etat = await api.gogEtat().catch(() => null);
}

/** À chaque ouverture de Frogtend, en tâche de fond : si le profil est connecté à GOG (ou que ses jeux ont déjà été
 * lus), ses jeux possédés sont relus. Jamais pendant une partie ; appelée APRÈS la récupération du jeu offert de GOG
 * (`+layout.svelte`). `profil` : celui pour lequel elle a été demandée ; un autre profil ouvert entre-temps (elle
 * arrive en retard) : rien, pas même l'état du nouveau profil effacé. Une erreur reste dans la ligne d'état du bloc
 * GOG, sans message (seule une connexion expirée est dite, une fois par jour). */
export async function gogAuDemarrage(profil: string | null | undefined = etat.profilOuvert?.id) {
  if (!toujoursLeMeme(profil, etat.profilOuvert?.id)) return;
  oublierEtatImport(true);
  void ecouterFermetures();
  if (partie.enJeu !== null) return;
  const e = await api.gogEtat().catch(() => null);
  if (!toujoursLeMeme(profil, etat.profilOuvert?.id)) return;
  gog.etat = e;
  if (!connecteAGog() && e?.connu !== true) return;
  await mettreAJourGog('discret');
}

/** L'annonce du cœur « fenêtre de GOG fermée par la personne » (pure, testée) : vraie seulement pour le profil qui l'a
 * ouverte, s'il est toujours ouvert. */
export function fermeturePourMoi(p: { boutique?: unknown; profil?: unknown } | null | undefined, ouvert: string | null | undefined): boolean {
  return !!p && p.boutique === 'gog' && typeof p.profil === 'string' && toujoursLeMeme(p.profil, ouvert);
}

/** L'écoute des fenêtres de GOG fermées par la personne (connexion, ou jeu offert qui demandait de se connecter) : une
 * seule, pour toute la vie de Frogtend ; à chaque fermeture, les jeux du compte sont lus (se connecter vaut accord). */
let arreterEcoute: (() => void) | null = null;
let ecouteEnPreparation: Promise<boolean> | null = null;

/** Met l'écoute en place si elle ne l'est pas ; rend vrai si elle vient d'être créée. */
async function ecouterFermetures(): Promise<boolean> {
  if (!isTauri()) return false;
  if (arreterEcoute) return false;
  if (ecouteEnPreparation) return ecouteEnPreparation.then(() => false);
  ecouteEnPreparation = listen<{ boutique: string; profil: string }>('gratuits-ferme', (e) => {
    if (fermeturePourMoi(e.payload, etat.profilOuvert?.id)) void mettreAJourGog('connexion');
  })
    .then((f) => {
      arreterEcoute = f;
      return true;
    })
    .catch(() => false)
    .finally(() => (ecouteEnPreparation = null));
  return ecouteEnPreparation;
}

function oublierEcoute() {
  arreterEcoute?.();
  arreterEcoute = null;
}

/** Se connecter à GOG (ou se reconnecter) : sa page officielle s'ouvre ; à la fermeture de la fenêtre, Frogtend lit
 * les jeux du compte (se connecter vaut accord, Seb 21/11). Frogtend ne voit jamais le mot de passe. */
export async function connexionGog() {
  if (gogOccupe()) return;
  gog.connexionEnCours = true;
  let nouvelle = false;
  try {
    nouvelle = await ecouterFermetures();
    await api.gratuitsConnexion('gog');
    toast('🔑 Connecte-toi à GOG dans la fenêtre qui s’ouvre, puis ferme-la : tes jeux GOG entreront dans ta ludothèque. Frogtend ne voit jamais ton mot de passe.');
  } catch (err) {
    // Seule l'écoute créée par CE clic est retirée.
    if (nouvelle) oublierEcoute();
    toast(`Impossible d’ouvrir GOG : ${motifDuRefus(err)}`, 'erreur');
  } finally {
    gog.connexionEnCours = false;
  }
}
