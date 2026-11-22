// Lot C4a (décisions de Seb du 21/11) : les jeux POSSÉDÉS du compte GOG connecté dans le navigateur du profil entrent
// dans la ludothèque, « pas installés », sans GOG Galaxy — sur le modèle de Steam (lot C2) : juste après la connexion,
// à chaque ouverture de Frogtend (discret : un message seulement s'il y a du nouveau ; « connexion expirée » une fois
// par jour, jamais pendant une partie) et à la main (« 🔄 Mettre à jour mes jeux GOG »). La connexion GOG déjà faite pour
// le jeu offert VAUT ACCORD : c'est le même état de connexion (`gratuits.connectes.gog`) pour les deux usages.
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { api, type AjoutGog, type CompteGogEtat, type EtatGog, type ImportGog, type OubliGog } from '$lib/api';
import { confirmer, fermerToast, toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { etat } from '$lib/etat.svelte';
import { partie } from '$lib/ludotheque/jeu.svelte';
import { rafraichirLudotheque } from './boutiques-pc.svelte';
import { gratuits, noterConnexion } from './gratuits.svelte';
import { annonceImport, aujourdhui, cheminActuel, direUneFoisParJour, messageExpire, stockageLocal } from './import-compte';
import { messageNouveaux, toujoursLeMeme } from './jeu-boutique';

export const gog = $state<{
  etat: EtatGog | null;
  etatProfil: string | null;
  enCours: boolean;
  connexionEnCours: boolean;
  erreur: string | null;
  expire: boolean;
  erreurProfil: string | null;
  profilImport: string | null;
  imports: number;
  partiel: boolean;
  comptes: Record<string, { expire: boolean; partiel: boolean }>;
  ajoutEnCours: boolean;
  oubliEnCours: string | null;
}>({
  etat: null,
  // Le profil pour lequel `etat` a été lu (jamais l'état d'un autre profil affiché).
  etatProfil: null,
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
  // Lot C5 : la dernière lecture (du profil `erreurProfil`) n'a pas pu lire la bibliothèque de GOG Galaxy en entier.
  partiel: false,
  // Lot C4b : l'état de la dernière lecture de chaque compte AJOUTÉ (par sa clé), pour le profil `erreurProfil`.
  comptes: {},
  // Lot C4b : « ➕ Ajouter un compte GOG » en cours ; « Oublier » en cours (la clé du compte).
  ajoutEnCours: false,
  oubliEnCours: null,
});

/** Une lecture GOG tourne-t-elle pour le profil OUVERT ? (Celle d'un autre profil ne bloque jamais ses boutons.) */
export function importGogEnCours(): boolean {
  return gog.enCours && gog.profilImport === (etat.profilOuvert?.id ?? null);
}

/** Les boutons GOG (connexion, jeu offert) attendent : une lecture des jeux, la récupération du jeu offert ou
 * l'ouverture de la fenêtre de connexion est en cours. */
export function gogOccupe(): boolean {
  return importGogEnCours() || gratuits.boutique === 'gog' || gog.connexionEnCours || gog.ajoutEnCours || gog.oubliEnCours !== null;
}

/** Le nom d'un compte GOG à l'écran (lot C4b, pur, testé) : son pseudo, sinon « Compte GOG n » — jamais son numéro. */
export function nomCompteGog(c: { nom: string; numero?: number }, place: number): string {
  // Le numéro rangé à l'arrivée du compte (stable même quand un autre compte est oublié) ; sa place à défaut.
  return c.nom.trim() ? c.nom.trim() : `Compte GOG ${c.numero && c.numero > 0 ? c.numero : place + 1}`;
}

/** « 1 jeu », « 197 jeux » (pur). */
export function nombreDeJeux(n: number): string {
  return `${n} jeu${n > 1 ? 'x' : ''}`;
}

/** La ligne d'un compte dans le bloc GOG (pur, testé) : « Seb — 197 jeux, mis à jour le 22 novembre » (le 🟣 est déjà
 * dans le titre du bloc). */
export function ligneCompteGog(c: CompteGogEtat, place: number): string {
  const quand = c.maj_le ? new Date(c.maj_le) : null;
  const date = quand && !Number.isNaN(quand.getTime()) ? `, mis à jour le ${quand.toLocaleDateString('fr-FR', { day: 'numeric', month: 'long' })}` : '';
  return `${nomCompteGog(c, place)} — ${nombreDeJeux(c.nb_jeux)}${date}`;
}

/** La connexion d'un ou plusieurs comptes GOG AJOUTÉS est perdue (lot C4b, pur, testé) : UNE phrase pour tous ;
 * « au bureau » depuis Taodbox. */
export function messageGogComptesExpires(noms: string[], chemin: string): string {
  const liste = noms.map((n) => `« ${n} »`).join(', ');
  const sujet = noms.length > 1 ? `de tes comptes GOG ${liste}` : `de ton compte GOG ${liste}`;
  const le = noms.length > 1 ? 'les' : 'le';
  if (chemin.startsWith('/taodbox')) return `🔑 La connexion ${sujet} a expiré : au bureau, dans 🛒 Boutiques, reconnecte-${le} (« ➕ Ajouter un compte GOG »).`;
  if (chemin.startsWith('/boutiques')) return `🔑 La connexion ${sujet} a expiré : reconnecte-${le} avec « ➕ Ajouter un compte GOG ».`;
  return `🔑 La connexion ${sujet} a expiré : dans 🛒 Boutiques, reconnecte-${le} (« ➕ Ajouter un compte GOG »).`;
}

/** Un seul compte ajouté expiré (pur, testé). */
export function messageGogCompteExpire(nom: string, chemin: string): string {
  return messageGogComptesExpires([nom], chemin);
}

/** Google bloque souvent la connexion dans une fenêtre intégrée (lot C4b) : la marche à suivre. */
export const MESSAGE_GOG_GOOGLE =
  'Google refuse la connexion dans cette fenêtre : définis un mot de passe pour ton compte GOG sur gog.com, puis connecte-toi avec ton adresse et ce mot de passe.';

/** La confirmation de « Oublier ce compte » (lot C4b, pur, testé). */
export function confirmationOubliGog(nom: string, profil: boolean): { titre: string; message: string } {
  const base = 'Frogtend ne lira plus les jeux de ce compte et oublie sa connexion. Tes jeux installés ne bougent pas. Tu pourras le rajouter plus tard.';
  const navigateur = ' Frogtend te déconnecte aussi de GOG dans Frogtend (pas dans GOG Galaxy) : le jeu offert ne sera plus récupéré tant que tu ne te reconnectes pas.';
  return { titre: `Oublier le compte GOG « ${nom} » ?`, message: profil ? base + navigateur : base };
}

/** Le message de fin de « Oublier ce compte » (lot C4b, pur, testé) : jamais dit sûr sans preuve du cœur. Le compte du
 * navigateur n'est plus relu par Frogtend (même si GOG garde la session) tant que la personne ne se reconnecte pas. */
export function messageOubliGog(r: OubliGog, nom: string): { texte: string; ton?: 'alerte' } {
  if (!r.secret_oublie)
    return { texte: `Compte GOG « ${nom} » oublié, mais sa connexion est restée dans le coffre de Windows (Frogtend ne s’en sert plus). Ses jeux déjà dans ta ludothèque y restent.`, ton: 'alerte' };
  if (r.profil && !r.navigateur_deconnecte)
    return {
      texte: `Compte GOG « ${nom} » oublié : Frogtend ne relit plus ses jeux. Mais GOG garde peut-être ta session dans Frogtend (le jeu offert pourrait encore arriver sur ce compte) : pour en être sûr, clique « 🔑 Se connecter à GOG », puis « Se déconnecter » sur sa page.`,
      ton: 'alerte',
    };
  return { texte: `✅ Compte GOG « ${nom} » oublié. Ses jeux déjà dans ta ludothèque y restent.` };
}

/** Le message de fin de « ➕ Ajouter un compte GOG » (lot C4b, pur, testé). */
export function messageAjoutGog(r: AjoutGog): { texte: string; ton?: 'alerte' } {
  const nom = nomCompteGog(r, Math.max(0, (r.numero ?? 1) - 1));
  const tete = `✅ Compte GOG « ${nom} » ${r.reconnexion ? 'reconnecté' : 'ajouté'}.`;
  const nouveaux = messageNouveaux([['gog', r.nouveaux, r.nouveaux_installes]]);
  const corps = nouveaux ? (r.galaxy ? nouveaux : sansGalaxy(nouveaux)) : 'Rien de nouveau : ses jeux sont déjà dans ta ludothèque.';
  const fin = r.partiel ? ' Sa liste complète arrivera à la prochaine mise à jour (GOG ne l’a pas donnée en entier).' : '';
  return { texte: `${tete} ${corps}${fin}`, ton: r.partiel ? 'alerte' : undefined };
}

/** Le message d'attente de « ➕ Ajouter un compte GOG » (pur, testé) : « ton autre compte » seulement s'il y en a déjà un. */
export function messageAttenteAjoutGog(dejaUnCompte: boolean): string {
  return `🔑 Connecte-toi à ${dejaUnCompte ? 'ton autre compte GOG' : 'ton compte GOG'} dans la fenêtre qui s’ouvre, puis attends : ses jeux arriveront ensuite. Frogtend ne voit jamais ton mot de passe.`;
}

/** La ligne quand GOG Galaxy est ouvert sur un autre compte que ceux de Frogtend (lot C4b). */
export const MESSAGE_GALAXY_AUTRE_COMPTE =
  'GOG Galaxy est ouvert sur un autre compte que ceux de Frogtend : pour installer ses jeux, change de compte dans GOG Galaxy, ou ajoute ce compte ici.';

/** Les comptes GOG à montrer : ceux du profil OUVERT seulement (jamais l'état lu pour un autre profil). */
export function comptesGogAffiches(): CompteGogEtat[] {
  return gog.etatProfil === (etat.profilOuvert?.id ?? null) ? (gog.etat?.comptes ?? []) : [];
}

/** L'état de la dernière lecture d'un compte ajouté, pour le profil ouvert seulement. */
export function etatLectureCompteGog(cle: string): { expire: boolean; partiel: boolean } | null {
  return gog.erreurProfil === (etat.profilOuvert?.id ?? null) ? (gog.comptes[cle] ?? null) : null;
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

/** Le message d'attente d'une lecture (lot C5 : la première fois, Frogtend lit la fiche de chaque jeu). */
export const MESSAGE_GOG_ATTENTE = '⏳ Lecture de tes jeux GOG… (jusqu’à quelques minutes la première fois)';

/** La ligne d'état quand la bibliothèque de GOG Galaxy n'a pas pu être lue en entier (pas une erreur : rien n'est
 * perdu). */
export const MESSAGE_GOG_PARTIEL = 'Liste partielle de GOG pour l’instant : clique « 🔄 Mettre à jour mes jeux GOG » (GOG peut te demander de confirmer ta connexion).';

/** La ligne d'état « liste partielle » vaut-elle pour le profil ouvert ? */
export function listeGogPartielle(): boolean {
  return gog.partiel && gog.erreurProfil === (etat.profilOuvert?.id ?? null);
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
/** Lot C4b : `cle` = la clé d'un compte AJOUTÉ (sa propre clé « une fois par jour » : `frogtend.gogExpire.<profil>.<clé>`) ;
 * sans elle, le compte du navigateur du profil (clé d'avant, `frogtend.gogExpire.<profil>`, inchangée). */
export function direGogExpireAujourdhui(profil: string, jour: string, stockage: Pick<Storage, 'getItem' | 'setItem'> | null, cle?: string): boolean {
  return direUneFoisParJour(cleExpireGog(profil, cle), jour, stockage);
}

function cleExpireGog(profil: string, cle?: string): string {
  return cle ? `frogtend.gogExpire.${profil}.${cle}` : `frogtend.gogExpire.${profil}`;
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
  // Lot C4b : un profil qui n'a QUE des comptes ajoutés (jamais connecté par son navigateur) : « pas connecté » n'est
  // pas une connexion expirée (sauf juste après « Se connecter », qui le dit autrement).
  const rp: ImportGog = mode !== 'connexion' && !connuAvant ? { ...r, reconnecter: false } : r;
  // « Expirée » seulement pour un compte qui était connecté (une première connexion pas finie n'a rien d'expiré).
  gog.expire = rp.reconnecter && (mode !== 'connexion' || connuAvant);
  gog.erreur = gog.expire ? messageGogExpire('/boutiques') : r.erreur ? `Mise à jour de ton compte GOG impossible : ${r.erreur}` : null;
  gog.comptes = {};
  for (const c of r.comptes ?? []) gog.comptes[c.cle] = { expire: c.reconnecter, partiel: c.partiel };
  const m = annonceConclusionGog(rp, mode, connuAvant, {
    chemin: cheminActuel(),
    enPartie: partie.enJeu !== null,
    places: new Map((gog.etat?.comptes ?? []).map((c, i) => [c.cle, i])),
    direAujourdhui: (cle?: string) => direGogExpireAujourdhui(profil ?? '', aujourdhui(), stockageLocal(), cle),
  });
  if (m) toast(m.texte, m.ton);
}

/** Le message UNIQUE d'une lecture de tous les comptes GOG (lot C4b, revue ; pur, testé) : le compte du navigateur
 * (expiré, pas connecté, erreur), les jeux nouveaux de TOUS les comptes (même quand le compte du navigateur a échoué),
 * puis les comptes ajoutés dont la connexion est perdue, réunis en une phrase. Rien n'a été lu (`lus` 0) : pas de
 * « Rien de nouveau ». À l'ouverture (discret) : jamais pendant une partie, une connexion perdue une fois par jour
 * (`direAujourdhui`, par compte), une erreur jamais. */
export function annonceConclusionGog(
  r: ImportGog,
  mode: ModeImportGog,
  connuAvant: boolean,
  o: { chemin: string; enPartie: boolean; places: Map<string, number>; direAujourdhui: (cle?: string) => boolean },
): { texte: string; ton?: 'alerte' } | null {
  const discret = mode === 'discret';
  if (discret && o.enPartie) return null;
  const morceaux: string[] = [];
  let alerte = false;
  const lu = r.lus === undefined ? !r.reconnecter && !r.erreur : r.lus > 0;
  if (r.erreur) {
    if (!discret) {
      morceaux.push(`Mise à jour de ton compte GOG impossible : ${r.erreur}`);
      alerte = true;
    }
  } else if (r.reconnecter) {
    const a = annonceImportGog(r, mode, o.chemin, connuAvant);
    if (a && (!discret || o.direAujourdhui())) {
      morceaux.push(a.texte);
      alerte = true;
    }
  }
  if (!r.erreur && !r.reconnecter) {
    const a = lu ? annonceImportGog(r, mode, o.chemin, connuAvant) : null;
    if (a) morceaux.push(a.texte);
  } else if (r.nouveaux > 0) {
    // Le compte du navigateur a échoué, mais les comptes ajoutés ont apporté des jeux : le dire aussi.
    const n = messageNouveaux([['gog', r.nouveaux, r.nouveaux_installes]]);
    if (n) morceaux.push(r.galaxy ? n : sansGalaxy(n));
  }
  const noms = (r.comptes ?? []).filter((c) => c.reconnecter && (!discret || o.direAujourdhui(c.cle))).map((c) => nomCompteGog(c, o.places.get(c.cle) ?? 0));
  if (noms.length) {
    morceaux.push(messageGogComptesExpires(noms, o.chemin));
    alerte = true;
  }
  return morceaux.length ? { texte: morceaux.join(' '), ton: alerte ? 'alerte' : undefined } : null;
}

/** Un compte ajouté a-t-il été lu (lot C4b) ? */
function unAjouteLu(r: ImportGog): boolean {
  return (r.comptes ?? []).some((c) => !c.reconnecter && !c.partiel);
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
  const attente = aLaMain ? toast(MESSAGE_GOG_ATTENTE, undefined, { jusquaFermeture: true }) : null;
  try {
    // La page de connexion façon Galaxy peut s'afficher quand la personne est là (« Se connecter » ou « 🔄 Mettre à jour
    // mes jeux GOG » — Seb 21/11 : sinon la lecture à la main abandonnait en silence), jamais en lecture discrète ni
    // pendant une partie.
    // Lot C4b : juste après que la personne s'est connectée elle-même, un compte du navigateur oublié est relu.
    const r = await api.gogImporterConnecte(mode !== 'discret' && partie.enJeu === null, mode === 'connexion');
    if (!toujoursLeMeme(profil, etat.profilOuvert?.id)) return false;
    // Un seul état de connexion GOG (jeu offert et jeux du compte) : ce que la lecture a vu le met à jour (une lecture
    // en ÉCHEC du compte du navigateur ne dit rien de sa connexion : rien n'est noté).
    if (!r.erreur) await noterConnexion('gog', r.reconnecter ? 'connexion' : 'connecte', profil);
    if (!r.reconnecter && !r.erreur) gratuits.gogDeconnecte = false;
    if (attente !== null) fermerToast(attente);
    await conclure(r, mode, profil, connuAvant);
    gog.partiel = !r.reconnecter && !r.erreur && r.bibliotheque_galaxy === false;
    if ((r.reconnecter || r.erreur) && !unAjouteLu(r)) {
      // Lot C4b : l'état des comptes ajoutés (connexion perdue…) se relit quand même.
      if (r.comptes?.length) await lireGog();
      return false;
    }
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
    gog.partiel = false;
    gog.comptes = {};
  }
}

export async function lireGog() {
  oublierEtatImport(false);
  const profil = etat.profilOuvert?.id ?? null;
  const e = await api.gogEtat().catch(() => null);
  // Un profil changé pendant la lecture : l'état lu n'est pas le sien.
  if (!toujoursLeMeme(profil, etat.profilOuvert?.id)) return;
  gog.etat = e;
  gog.etatProfil = profil;
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
  gog.etatProfil = profil ?? null;
  // Lot C4b : aussi un profil qui n'a que des comptes ajoutés.
  if (!connecteAGog() && e?.connu !== true && !e?.comptes?.length) return;
  await mettreAJourGog('discret');
}

/** L'annonce du cœur « fenêtre de GOG fermée par la personne » (pure, testée) : vraie seulement pour le profil qui l'a
 * ouverte, s'il est toujours ouvert. */
export function fermeturePourMoi(p: { boutique?: unknown; profil?: unknown } | null | undefined, ouvert: string | null | undefined): boolean {
  return !!p && p.boutique === 'gog' && typeof p.profil === 'string' && toujoursLeMeme(p.profil, ouvert);
}

/** Le message quand la fenêtre de connexion façon Galaxy s'affiche (GOG demande de se connecter) ; lot C4b : il nomme le
 * compte attendu quand Frogtend connaît son pseudo (pur, testé). */
export function messageGogFenetre(attendu?: unknown): string {
  const nom = typeof attendu === 'string' ? attendu.trim().slice(0, 40) : '';
  return `🔑 Connecte-toi à GOG${nom ? ` avec ton compte « ${nom} »` : ''} dans sa fenêtre (elle vient de s’ouvrir) : tes jeux arriveront ensuite.`;
}
export const MESSAGE_GOG_FENETRE = messageGogFenetre();

/** L'écoute des fenêtres de GOG fermées par la personne (connexion, ou jeu offert qui demandait de se connecter) : une
 * seule, pour toute la vie de Frogtend ; à chaque fermeture, les jeux du compte sont lus (se connecter vaut accord). */
let arreterEcoute: (() => void) | null = null;
let ecouteEnPreparation: Promise<boolean> | null = null;

/** Met l'écoute en place si elle ne l'est pas ; rend vrai si elle vient d'être créée. */
async function ecouterFermetures(): Promise<boolean> {
  if (!isTauri()) return false;
  if (arreterEcoute) return false;
  if (ecouteEnPreparation) return ecouteEnPreparation.then(() => false);
  ecouteEnPreparation = Promise.all([
    listen<{ boutique: string; profil: string }>('gratuits-ferme', (e) => {
      if (fermeturePourMoi(e.payload, etat.profilOuvert?.id)) void mettreAJourGog('connexion');
    }),
    // Lot C5 : GOG demande de se connecter dans la fenêtre de la connexion façon Galaxy, qui vient de s'afficher.
    listen<{ profil: string; compte?: string }>('gog-connexion-montree', (e) => {
      if (fermeturePourMoi({ boutique: 'gog', profil: e.payload?.profil }, etat.profilOuvert?.id)) toast(messageGogFenetre(e.payload?.compte), 'alerte');
    }),
    // Lot C4b : Google refuse la connexion dans la fenêtre (navigateur intégré) : la marche à suivre.
    listen<{ profil: string }>('gog-google-refuse', (e) => {
      if (fermeturePourMoi({ boutique: 'gog', profil: e.payload?.profil }, etat.profilOuvert?.id)) toast(MESSAGE_GOG_GOOGLE, 'alerte');
    }),
  ])
    .then(([a, b, c]) => {
      arreterEcoute = () => {
        a();
        b();
        c();
      };
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

/** Lot C4b (Seb 22/11) : « ➕ Ajouter un compte GOG ». La page de connexion de GOG s'ouvre (fenêtre à part, rien n'y
 * reste) ; ses jeux entrent dans la ludothèque (se connecter vaut accord). Un compte déjà là est refusé par le cœur.
 * Rend vrai si un compte a été ajouté ou reconnecté. */
export async function ajouterCompteGog(): Promise<boolean> {
  if (gogOccupe()) return false;
  // Jamais une fenêtre de connexion par-dessus une partie.
  if (partie.enJeu !== null) {
    toast('⏳ Une partie est en cours : ajoute ton compte GOG quand elle sera finie.', 'alerte');
    return false;
  }
  const profil = etat.profilOuvert?.id ?? null;
  gog.ajoutEnCours = true;
  const attente = toast(messageAttenteAjoutGog(comptesGogAffiches().length > 0), undefined, { jusquaFermeture: true });
  try {
    // L'écoute des annonces du cœur (Google qui refuse la fenêtre…).
    await ecouterFermetures();
    const r = await api.gogAjouterCompte();
    if (!toujoursLeMeme(profil, etat.profilOuvert?.id)) return false;
    fermerToast(attente);
    if (!r.fait) {
      toast('Fenêtre fermée : aucun compte GOG ajouté.');
      return false;
    }
    delete gog.comptes[r.cle];
    try {
      stockageLocal()?.removeItem(cleExpireGog(profil ?? '', r.cle));
    } catch {
      // Stockage refusé : rien à retirer.
    }
    const m = messageAjoutGog(r);
    toast(m.texte, m.ton);
    // Taodbox relit ses cartes « à installer ».
    gog.imports++;
    await rafraichirLudotheque();
    await lireGog();
    return true;
  } catch (err) {
    if (toujoursLeMeme(profil, etat.profilOuvert?.id)) toast(`Impossible d’ajouter ce compte GOG : ${motifDuRefus(err)}`, 'erreur');
    return false;
  } finally {
    fermerToast(attente);
    gog.ajoutEnCours = false;
  }
}

/** Lot C4b : « Oublier ce compte » GOG, après confirmation. La liste est réécrite, puis sa connexion oubliée ; le compte
 * du navigateur : Frogtend se déconnecte aussi de GOG (cookies gog.com seulement). Ses jeux restent dans la ludothèque.
 * Rend vrai si le compte a été oublié (l'appelant déplace alors le focus : sa ligne disparaît). */
export async function oublierCompteGog(c: CompteGogEtat, place: number): Promise<boolean> {
  if (gogOccupe()) return false;
  const nom = nomCompteGog(c, place);
  const q = confirmationOubliGog(nom, c.profil);
  const oui = await confirmer(q.titre, { message: q.message, libelleValider: 'Oublier', danger: true });
  if (!oui || gogOccupe()) return false;
  const profil = etat.profilOuvert?.id ?? null;
  gog.oubliEnCours = c.cle;
  try {
    const r = await api.gogOublierCompte(c.cle);
    if (!toujoursLeMeme(profil, etat.profilOuvert?.id)) return true;
    delete gog.comptes[c.cle];
    if (c.profil) {
      // Frogtend n'est plus connecté à GOG par son navigateur (jeu offert compris) : plus de « connexion expirée ».
      await noterConnexion('gog', 'connexion', profil);
      oublierEtatImport(true);
    }
    try {
      stockageLocal()?.removeItem(cleExpireGog(profil ?? '', c.profil ? undefined : c.cle));
    } catch {
      // Stockage refusé : rien à retirer.
    }
    await lireGog();
    // Taodbox relit ses cartes (les jeux restent ; leur état peut changer).
    gog.imports++;
    const m = messageOubliGog(r, nom);
    toast(m.texte, m.ton);
    return true;
  } catch (err) {
    if (toujoursLeMeme(profil, etat.profilOuvert?.id)) toast(`Impossible d’oublier ce compte GOG : ${motifDuRefus(err)}`, 'erreur');
    return false;
  } finally {
    gog.oubliEnCours = null;
  }
}
