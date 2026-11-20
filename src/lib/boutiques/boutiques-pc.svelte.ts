// L'import automatique des jeux PC (lot A, ✅ Seb 18/11) : à l'ouverture d'un profil, les jeux INSTALLÉS de chaque
// boutique de ce PC (Steam, Epic, GOG, Ubisoft, Amazon, Xbox, Battle.net) entrent seuls dans la ludothèque, en tâche de
// fond, sans jamais ralentir l'ouverture. Un seul toast, regroupé, et seulement pour des jeux NOUVEAUX. Lot B : les jeux
// POSSÉDÉS d'Epic et d'Ubisoft (lus sans connexion) entrent en même temps, « pas installé », avec ⬇ Installer — mais
// seulement si le profil l'a ACCEPTÉ (✅ Seb 18/11 : ils viennent du compte ouvert sur ce PC ; demandé une fois).
import { api, type EtatBoutiquePc } from '$lib/api';
import { choisir, toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { etat, reglerProfil } from '$lib/etat.svelte';
import { partie } from '$lib/ludotheque/jeu.svelte';
import { rechargerListe, rechargerPlateformes } from '$lib/ludotheque/ludotheque.svelte';
import { suivreTelechargements } from '$lib/ludotheque/telechargements.svelte';
import { messageNouveaux, moyenDOuverture, nomBoutique, ouvrirJeuDeBoutique, questionPossedes, toujoursLeMeme, estBattlenet, MESSAGE_BATTLENET } from './jeu-boutique';
import type { JeuResume } from '$lib/api';

/** L'état des boutiques de ce PC, connu après la relecture (ou lu par la page 🛒 Boutiques). */
export const boutiquesPc = $state<{ etats: EtatBoutiquePc[]; enCours: boolean; erreur: string | null }>({ etats: [], enCours: false, erreur: null });




/** Le lanceur de cette boutique est-il absent de ce PC ? (`false` tant qu'on ne sait pas : on essaie.) */
export function lanceurAbsent(boutique: string | null | undefined): boolean {
  return !!boutique && boutiquesPc.etats.some((e) => e.boutique === boutique && !e.lanceur);
}

/** Le message à montrer à la place d'un ▶ Jouer qui échouerait : `null` si le lanceur est là (ou par GOG Galaxy). */
export function messageLanceur(j: JeuResume): string | null {
  // Battle.net D'ABORD : même lanceur installé, Jouer échouerait toujours.
  if (estBattlenet(j)) return MESSAGE_BATTLENET;
  const m = moyenDOuverture(j);
  if (!m || m.par !== 'boutique' || !lanceurAbsent(m.boutique)) return null;
  return `Ce jeu a besoin du lanceur ${nomBoutique(m.boutique)}, absent de ce PC.`;
}

/** À l'ouverture du profil : relit les boutiques si la personne l'a choisi (coché d'office), sinon Steam seul comme
 * avant. Jamais attendu par l'ouverture ; une erreur est notée, jamais montrée (rien n'est cassé). */
export async function boutiquesAuDemarrage() {
  if (!etat.profil.boutiquesPc.auto) {
    await api.steamJeux().catch(() => {});
    suivreTelechargements();
    return;
  }
  // Le profil d'avant l'attente : s'il a changé entre-temps, rien n'est montré ni rechargé (le cœur n'a rien écrit).
  const profil = etat.profilOuvert?.id;
  boutiquesPc.enCours = true;
  try {
    const r = await api.boutiquesRelirePc(possedesAcceptes());
    if (!toujoursLeMeme(profil, etat.profilOuvert?.id)) return;
    boutiquesPc.etats = r.boutiques;
    suivreTelechargements();
    const m = messageNouveaux(r.nouveaux);
    if (m) {
      // Le message D'ABORD (Seb 18/11 : 15 jeux ajoutés, aucun message : une relecture de la grille en échec
      // l'empêchait), puis la grille ET la barre des systèmes (son compteur « Windows » restait à 2).
      if (partie.enJeu === null) toast(m);
      await rafraichirLudotheque();
    }
    boutiquesPc.enCours = false;
    await demanderPossedes(profil);
  } catch {
    // Profil fermé entre-temps, lecture impossible : la ludothèque reste comme elle était.
  } finally {
    boutiquesPc.enCours = false;
  }
}

/** Le profil a-t-il accepté les jeux possédés des comptes ouverts sur ce PC ? */
export function possedesAcceptes(): boolean {
  return etat.profil.boutiquesPc.possedes === 'oui';
}

/** La question des jeux possédés, posée UNE fois par profil (✅ Seb 18/11), quand la relecture en a trouvé : jamais
 * pendant une partie (elle attend la prochaine ouverture du profil). Échap : redemandée la prochaine fois ; « Non » :
 * plus jamais (changeable dans ⚙ Options). */
export async function demanderPossedes(profil: string | null | undefined) {
  if (etat.profil.boutiquesPc.possedes !== '' || partie.enJeu !== null) return;
  const q = questionPossedes(boutiquesPc.etats);
  if (!q) return;
  const c = await choisir<'oui' | 'non'>(
    q.titre,
    [
      { valeur: 'oui', libelle: '✅ Oui, les ajouter' },
      { valeur: 'non', libelle: 'Non, seulement mes jeux installés' },
    ],
    q.message,
  );
  if (c === null || !toujoursLeMeme(profil, etat.profilOuvert?.id)) return;
  await choisirPossedes(c === 'oui');
}

/** Oui : les jeux possédés entrent (relecture aussitôt). Non : ils n'entrent plus, et ceux déjà rangés qui ne sont ni
 * installés ni joués sont retirés (le cœur garde tout jeu installé, joué ou au registre du PC), et c'est dit. */
export async function choisirPossedes(oui: boolean) {
  const profil = etat.profilOuvert?.id;
  await reglerProfil('boutiquesPc.possedes', oui ? 'oui' : 'non');
  if (oui) {
    await relireMaintenant();
    return;
  }
  try {
    const n = await api.boutiquesRetirerPossedes();
    if (!toujoursLeMeme(profil, etat.profilOuvert?.id)) return;
    if (n > 0) {
      toast(n === 1 ? '1 jeu de tes comptes retiré de ta ludothèque (jamais installé ni joué).' : `${n} jeux de tes comptes retirés de ta ludothèque (jamais installés ni joués).`);
      await rafraichirLudotheque();
    }
  } catch (e) {
    toast(`Retrait impossible : ${motifDuRefus(e)}`, 'erreur');
  }
}

/** La page 🛒 Boutiques : l'état de chaque boutique de ce PC (lecture seule). */
export async function lireEtatsPc() {
  try {
    boutiquesPc.etats = await api.boutiquesEtatPc();
    boutiquesPc.erreur = null;
  } catch (e) {
    boutiquesPc.erreur = `Lecture des boutiques impossible : ${motifDuRefus(e)}`;
  }
}

/** La grille et la barre des systèmes relues après un import (un échec ne cache jamais le message déjà montré). */
export async function rafraichirLudotheque() {
  await rechargerListe().catch(() => {});
  await rechargerPlateformes().catch(() => {});
}

/** « 🔄 Relire maintenant » (page 🛒 Boutiques) : comme à l'ouverture du profil, avec un toast même sans nouveauté. */
export async function relireMaintenant() {
  const profil = etat.profilOuvert?.id;
  boutiquesPc.enCours = true;
  try {
    const r = await api.boutiquesRelirePc(possedesAcceptes());
    if (!toujoursLeMeme(profil, etat.profilOuvert?.id)) return;
    boutiquesPc.etats = r.boutiques;
    boutiquesPc.erreur = null;
    suivreTelechargements();
    const m = messageNouveaux(r.nouveaux);
    toast(m ?? 'Rien de nouveau : tes jeux sont déjà dans ta ludothèque.');
    if (m) await rafraichirLudotheque();
  } catch (e) {
    boutiquesPc.erreur = `Lecture des boutiques impossible : ${motifDuRefus(e)}`;
    toast(boutiquesPc.erreur, 'erreur');
  } finally {
    boutiquesPc.enCours = false;
  }
}

/** Jouer ou installer par la boutique ; une erreur (lanceur absent…) est DITE, jamais un échec en silence. */
export async function ouvrirParLaBoutique(j: JeuResume) {
  try {
    await ouvrirJeuDeBoutique(j);
  } catch (e) {
    toast(motifDuRefus(e), 'erreur');
  }
}
