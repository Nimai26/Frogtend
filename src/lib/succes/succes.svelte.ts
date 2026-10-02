// Les succès (Seb, 02/10) : RetroAchievements pour l'émulation (compte et clé d'API Web de la personne, dans le
// coffre de Windows), Steam pour ses jeux (clé Steam déjà réglée). La clé ne s'affiche jamais.
import { api } from '$lib/api';
import { demander, informer, toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';

export const PAGE_CLE_RA = 'https://retroachievements.org/settings';

export interface EtatRetro {
  compte: string | null;
  cle_enregistree: boolean;
  /** Le jeton des émulateurs (RetroArch, PCSX2) est-il au coffre ? */
  emulateurs_connectes?: boolean;
}

export interface Progression {
  id: number;
  titre: string;
  succes: number;
  obtenus: number;
  obtenus_hardcore: number;
  points: number;
  points_obtenus: number;
  image: string | null;
}

export interface SuccesRetro {
  etat: 'aucun' | 'compte' | 'pas_verifiable' | 'ok';
  versions: { chemin: string; compatible: boolean; courante: boolean; verifiable?: boolean }[];
  jeu: Progression | null;
  compatibles: { md5: string; nom: string; etiquettes: string[] }[];
}

export const retro = $state<{ etat: EtatRetro | null }>({ etat: null });

export async function lireRetro() {
  retro.etat = await api.raEtat().catch(() => null);
}

/** Régler (ou corriger) le compte RetroAchievements du profil. */
export async function reglerRetro(): Promise<boolean> {
  const e = await api.raEtat().catch(() => null);
  const compte = await demander('🏆 Ton compte RetroAchievements', {
    message: 'Ton nom d’utilisateur sur retroachievements.org.',
    valeur: e?.compte ?? '',
    libelleValider: 'Suivant',
  });
  if (!compte) return false;
  const cle = await demander('🔑 Ta clé d’API Web RetroAchievements', {
    message: [
      `Connecte-toi sur ${PAGE_CLE_RA}, section « Keys » : copie ta « Web API Key ».`,
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
    retro.etat = await api.raRegler(compte, cle || undefined);
    toast('✅ Compte RetroAchievements vérifié et enregistré.');
    return true;
  } catch (err) {
    toast(`Compte non enregistré : ${motifDuRefus(err)}`, 'erreur');
    return false;
  }
}

/** Connecter les émulateurs : le mot de passe sert une fois à obtenir leur jeton (au coffre), puis il est oublié. */
export async function connecterEmulateurs(): Promise<boolean> {
  const e = await api.raEtat().catch(() => null);
  if (!e?.compte) {
    await informer('🏆 Ton compte d’abord', 'Règle d’abord ton compte RetroAchievements (nom et clé), juste au-dessus.');
    return false;
  }
  const mdp = await demander(`🎮 Connecter les émulateurs au compte « ${e.compte} »`, {
    message: [
      'Ton mot de passe RetroAchievements, UNE fois : Frogtend obtient le jeton de connexion des émulateurs (comme ils le font eux-mêmes), le range dans le coffre de Windows, et oublie le mot de passe.',
      'RetroArch et PCSX2 seront ensuite connectés avec TON compte à chaque partie de ton profil.',
      'DuckStation chiffre lui-même sa connexion : connecte-toi une fois dans DuckStation (il le demandera à ta première partie) ; Frogtend la garde ensuite pour ton profil.',
    ].join('\n'),
    masque: true,
    libelleValider: 'Connecter',
  });
  if (!mdp) return false;
  try {
    retro.etat = await api.raConnecterEmulateurs(mdp);
    toast('✅ Émulateurs connectés à RetroAchievements pour ton profil.');
    return true;
  } catch (err) {
    toast(`Connexion refusée : ${motifDuRefus(err)}`, 'erreur');
    return false;
  }
}

export async function oublierRetro() {
  try {
    await api.raOublier();
    await lireRetro();
    toast('Compte RetroAchievements oublié (sa clé a quitté le coffre de Windows).');
  } catch (err) {
    toast(`Impossible : ${motifDuRefus(err)}`, 'erreur');
  }
}

export async function ouvrirPageCleRetro() {
  const { openUrl } = await import('@tauri-apps/plugin-opener');
  await openUrl(PAGE_CLE_RA).catch(() => {});
}

const autre_compatible = (r: SuccesRetro) => r.versions.some((v) => v.compatible && !v.courante);

/** Ce qu'on dit d'un jeu, d'après ses versions (pur, testé). */
export function resumeRetro(r: SuccesRetro): { ton: 'ok' | 'alerte' | 'neutre'; texte: string } | null {
  switch (r.etat) {
    case 'aucun':
      return null;
    case 'compte':
      return { ton: 'neutre', texte: 'Règle ton compte RetroAchievements (⚙ Options ▸ Comptes) pour voir les succès de ce jeu.' };
    case 'pas_verifiable':
      return { ton: 'neutre', texte: 'Frogtend ne sait pas encore vérifier cette version pour RetroAchievements (format .chd, ou console pas encore prise en charge).' };
  }
  const courante = r.versions.find((v) => v.courante) ?? r.versions[0];
  const autre = r.versions.find((v) => v.compatible && !v.courante);
  if (courante?.compatible) return { ton: 'ok', texte: '✅ Ta version est compatible RetroAchievements.' };
  if (courante && courante.verifiable === false && !autre_compatible(r))
    return { ton: 'neutre', texte: 'Frogtend ne sait pas encore lire le format de ta version (.chd) pour la vérifier.' };
  if (autre) {
    const nom = autre.chemin.split(/[\\/]/).pop();
    return { ton: 'alerte', texte: `⚠ Ta version n’est pas reconnue, mais « ${nom} » l’est : choisis-la (⚙ Gérer le jeu ▸ 📀 Version).` };
  }
  if (r.compatibles.length) {
    const noms = r.compatibles.map((c) => c.nom).filter(Boolean).slice(0, 3).join(' ; ');
    return { ton: 'alerte', texte: `⚠ Ta version n’est pas reconnue. Version compatible manquante : ${noms}. Demande-la à Firehouse (📨 Demander un jeu).` };
  }
  if (r.jeu) return { ton: 'alerte', texte: '⚠ Ta version n’est pas reconnue par RetroAchievements.' };
  return { ton: 'neutre', texte: 'Aucun succès RetroAchievements connu pour ce jeu.' };
}
