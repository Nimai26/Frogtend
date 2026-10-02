// Steam (lot 9) : le compte se règle dans ⚙ Options ▸ Comptes ▸ Steam, OU au moment d'un import si rien n'est encore
// réglé (règle de Seb, comme LaunchBox). La clé d'API ne s'affiche jamais : on dit seulement si elle est enregistrée.
import { api, type EtatSteam, type JeuBoutique } from '$lib/api';
import { confirmer, demander, informer, toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';

export const PAGE_CLE_STEAM = 'https://steamcommunity.com/dev/apikey';

export const steam = $state<{ etat: EtatSteam | null; jeux: JeuBoutique[]; enCours: boolean }>({ etat: null, jeux: [], enCours: false });

export async function lireSteam() {
  steam.etat = await api.steamEtat().catch(() => null);
  steam.jeux = await api.steamJeux().catch(() => []);
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
