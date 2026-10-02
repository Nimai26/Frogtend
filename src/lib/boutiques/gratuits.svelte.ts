// Les jeux offerts (lot 9). Décision de Seb (02/10) : Frogtend essaie de les obtenir TOUT SEUL ; si ça bloque
// (connexion, captcha, page changée), la page de la boutique s'affiche et on finit soi-même. Epic pour commencer.
import { api, type JeuOffert, type RecoltePsPlus } from '$lib/api';
import { toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { etat, reglerProfil } from '$lib/etat.svelte';

export const gratuits = $state<{ liste: JeuOffert[]; enCours: string | null; psplusEnCours: boolean }>({
  liste: [],
  enCours: null,
  psplusEnCours: false,
});

/** Un jour, en secondes : la vérification automatique ne se refait pas plus souvent. */
const JOUR_S = 24 * 3600;
/** PS Plus : les jeux changent une fois par mois ; un passage par semaine suffit. */
const SEMAINE_S = 7 * JOUR_S;

export async function lireGratuits() {
  gratuits.liste = await api.gratuitsListe().catch(() => []);
}

export async function connexionEpic() {
  try {
    await api.gratuitsConnexionEpic();
    toast('🔑 Connecte-toi à Epic dans la fenêtre qui s’ouvre, puis ferme-la. Frogtend ne voit jamais ton mot de passe.');
  } catch (e) {
    toast(`Impossible d’ouvrir Epic : ${motifDuRefus(e)}`, 'erreur');
  }
}

/** Le message d'une tentative. */
export function messageObtention(titre: string, r: { etat: string; motif?: string }): { texte: string; ton: 'ok' | 'alerte' | 'erreur' } {
  switch (r.etat) {
    case 'obtenu':
      return { texte: `🎁 « ${titre} » est à toi !`, ton: 'ok' };
    case 'deja':
      return { texte: `✅ « ${titre} » était déjà dans ta bibliothèque Epic.`, ton: 'ok' };
    case 'connexion':
      return { texte: `🔑 Connecte-toi à Epic dans la fenêtre ouverte, puis réessaie pour « ${titre} ».`, ton: 'alerte' };
    case 'captcha':
      return { texte: `🧩 Epic demande une vérification pour « ${titre} » : fais-la dans la fenêtre, puis clique « Place Order ».`, ton: 'alerte' };
    default:
      return { texte: `⚠ « ${titre} » n’a pas pu être obtenu tout seul (${r.motif ?? 'raison inconnue'}) : finis dans la fenêtre ouverte.`, ton: 'alerte' };
  }
}

export async function connexionPlaystation() {
  try {
    await api.gratuitsConnexionPlaystation();
    toast('🔑 Clique « Se connecter » en haut du PlayStation Store, connecte-toi, puis ferme la fenêtre. Frogtend ne voit jamais ton mot de passe.');
  } catch (e) {
    toast(`Impossible d’ouvrir le PlayStation Store : ${motifDuRefus(e)}`, 'erreur');
  }
}

/** Le message d'un passage sur PS Plus. */
export function messagePsPlus(r: RecoltePsPlus): { texte: string; ton: 'ok' | 'alerte' | 'erreur' } {
  switch (r.etat) {
    case 'faite':
      if (r.ajoutes) return { texte: `🎮 PS Plus : ${r.ajoutes} jeu(x) ajouté(s) à ta bibliothèque PlayStation (${r.deja} déjà dedans).`, ton: 'ok' };
      if (r.deja) return { texte: `✅ PS Plus : rien de nouveau, les ${r.deja} jeu(x) du mois sont déjà dans ta bibliothèque.`, ton: 'ok' };
      return { texte: `⚠ PS Plus : ${r.vus} jeu(x) vus, aucun bouton « Ajouter à la bibliothèque » trouvé. Regarde le Store toi-même.`, ton: 'alerte' };
    case 'connexion':
      return { texte: '🔑 Connecte-toi au PlayStation Store dans la fenêtre ouverte, puis réessaie.', ton: 'alerte' };
    default:
      return { texte: `⚠ PS Plus n’a pas pu être fait tout seul (${r.motif}) : finis dans la fenêtre ouverte.`, ton: 'alerte' };
  }
}

/** Ajouter les jeux PS Plus du mois à la bibliothèque PlayStation (rien n'entre dans la ludothèque). */
export async function recolterPsPlus(discret = false) {
  if (gratuits.psplusEnCours) return;
  gratuits.psplusEnCours = true;
  try {
    const m = messagePsPlus(await api.gratuitsPsPlus());
    if (!discret || m.ton !== 'ok' || m.texte.includes('ajouté')) toast(m.texte, m.ton);
  } catch (e) {
    toast(`Impossible : ${motifDuRefus(e)}`, 'erreur');
  } finally {
    gratuits.psplusEnCours = false;
  }
}

/** Obtenir un jeu offert. Rend vrai si c'est fait (ou déjà fait). */
export async function obtenir(j: JeuOffert, discret = false): Promise<boolean> {
  gratuits.enCours = j.slug;
  try {
    const r = await api.gratuitsObtenirEpic(j.slug);
    const m = messageObtention(j.titre, r);
    if (!discret || m.ton !== 'ok') toast(m.texte, m.ton);
    if (r.etat === 'obtenu' || r.etat === 'deja') {
      j.obtenu = true;
      return true;
    }
    return false;
  } catch (e) {
    toast(`Impossible : ${motifDuRefus(e)}`, 'erreur');
    return false;
  } finally {
    gratuits.enCours = null;
  }
}

/** Obtenir tous les jeux offerts pas encore obtenus ; s'arrête dès qu'il faut l'intervention de la personne. */
export async function obtenirTout(discret = false) {
  await lireGratuits();
  let n = 0;
  for (const j of gratuits.liste.filter((x) => !x.obtenu)) {
    if (!(await obtenir(j, discret))) return;
    n++;
  }
  if (n && discret) toast(`🎁 ${n} jeu(x) offert(s) obtenu(s) sur Epic.`);
  else if (!n && !discret) toast('Rien de nouveau : tous les jeux offerts sont déjà à toi.');
}

/** À l'ouverture du profil : si la personne l'a choisi, une vérification par jour, discrète. */
export async function gratuitsAuDemarrage() {
  if (etat.pc.firehouse.simule) return;
  const g = etat.profil.gratuits;
  const maintenant = Math.floor(Date.now() / 1000);
  if (g.auto && maintenant - (g.derniere ?? 0) >= JOUR_S) {
    await reglerProfil('gratuits.derniere', maintenant);
    await obtenirTout(true);
  }
  if (g.psplus && maintenant - (g.psplusDernier ?? 0) >= SEMAINE_S) {
    await reglerProfil('gratuits.psplusDernier', maintenant);
    await recolterPsPlus(true);
  }
}
