// Nexus Mods (lot 0.82.0 ; ✅ Seb 19-20/10) : la clé d'API PERSONNELLE de chaque profil (dans le coffre de Windows), et
// les liens « Mod Manager Download » reçus du site (nxm://), installés dans le jeu d'où la page a été ouverte, après
// le oui de la personne. Jamais pendant une partie ni au canapé : les liens attendent.
import { api, taille } from '$lib/api';
import { choisir, confirmer, demander, fermerToast, informer, toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { ouvrirPageWeb } from '$lib/web';

/** La page de Nexus où se trouve la clé d'API personnelle (à vérifier à l'essai de Seb). */
export const PAGE_CLE_NEXUS = 'https://next.nexusmods.com/settings/api-keys';

export interface EtatNexus {
  compte: string | null;
  cle_enregistree: boolean;
}

export interface LienNexus {
  n: number;
  jeu_id: number | null;
  mod_nom: string | null;
  domaine: string;
  mod_id: number;
  fichier_id: number;
}

export interface InfosNexus {
  jeu_id: number;
  titre: string;
  mod_nom: string;
  fichier: string;
  taille_ko: number;
  zip: boolean;
  expire: boolean;
}

export const nexus = $state<{ etat: EtatNexus | null }>({ etat: null });

export async function lireNexus() {
  nexus.etat = await api.nexusEtat().catch(() => null);
}

/** Régler (ou remplacer) la clé d'API Nexus du profil : vérifiée par Nexus avant d'aller au coffre. */
export async function reglerNexus(): Promise<boolean> {
  const o = await choisir<'page' | 'coller'>(
    '🔑 Ta clé Nexus Mods',
    [
      { valeur: 'page', libelle: '🌐 Ouvrir la page de ma clé', detail: 'connecte-toi sur Nexus Mods si besoin' },
      { valeur: 'coller', libelle: '📋 Je l’ai copiée : la coller' },
    ],
    'Ta clé personnelle est gratuite : elle est sur une page de ton compte Nexus Mods (en anglais, « Personal API Key »). Copie-la, puis reviens ici.',
  );
  if (!o) return false;
  if (o === 'page') await ouvrirPageCleNexus();
  const cle = await demander('🔑 Colle ta clé Nexus Mods', {
    message: [
      'La longue suite de lettres et de chiffres copiée sur la page de ton compte Nexus.',
      'Elle ira dans le coffre de Windows : Frogtend ne l’affichera plus jamais, et ne l’envoie qu’à Nexus.',
    ].join('\n'),
    masque: true,
    libelleValider: 'Vérifier et enregistrer',
  });
  if (!cle) return false;
  try {
    nexus.etat = await api.nexusRegler(cle);
    toast(`✅ Clé Nexus vérifiée et enregistrée (compte « ${nexus.etat.compte ?? '?'} »).`);
    return true;
  } catch (err) {
    toast(`Clé non enregistrée : ${motifDuRefus(err)}`, 'erreur');
    return false;
  }
}

export async function oublierNexus() {
  if (!(await confirmer('Oublier ta clé Nexus ?', { message: 'Elle quitte le coffre de Windows. Les mods déjà installés restent.', libelleValider: 'Oublier' }))) return;
  try {
    await api.nexusOublier();
    await lireNexus();
    toast('Clé Nexus oubliée (elle a quitté le coffre de Windows).');
  } catch (err) {
    toast(`Impossible : ${motifDuRefus(err)}`, 'erreur');
  }
}

export async function ouvrirPageCleNexus() {
  await ouvrirPageWeb(PAGE_CLE_NEXUS);
}

/** Ouvre la page Nexus d'un mod depuis un jeu : le lien qui reviendra ira à ce jeu. */
export async function ouvrirSurNexus(id: number, modNom: string): Promise<void> {
  const e = await api.nexusEtat().catch(() => null);
  if (!e?.cle_enregistree) {
    const o = await confirmer('🔑 Ta clé Nexus d’abord', {
      message:
        'Pour que Frogtend installe un mod de Nexus, il faut ta clé d’API personnelle (gratuite, dans ton compte Nexus). Sans elle, la page s’ouvre quand même, mais tu installeras le mod à la main.',
      libelleValider: '🔑 Régler ma clé',
    });
    if (o) await reglerNexus();
  }
  const avecCle = !!(await api.nexusEtat().catch(() => null))?.cle_enregistree;
  try {
    await api.nexusOuvrir(id, modNom);
    await informer(
      '🌐 La page du mod s’ouvre dans ton navigateur',
      avecCle
        ? 'Onglet « Files » : clique « Mod Manager Download » sur le fichier voulu (Nexus demande parfois d’attendre quelques secondes avec un compte gratuit). Frogtend le reçoit et te demande ton accord avant d’installer quoi que ce soit.'
        : 'Sans ta clé Nexus, Frogtend ne peut pas l’installer pour toi : suis les explications de la page pour l’installer à la main.',
    );
  } catch (err) {
    toast(`Impossible : ${motifDuRefus(err)}`, 'erreur');
  }
}

let enCours = false;
/** Les liens déjà proposés (jamais reproposés, même si les oublier a échoué). */
const proposes = new Set<number>();

/** Les liens reçus du site : un par un, chacun après un oui (appelé quand un profil est ouvert, hors partie et hors
 * Taodbox). La boîte est relue après chaque lien : un clic fait pendant qu'une fenêtre était ouverte n'attend pas. */
export async function traiterLiensNexus(): Promise<void> {
  if (enCours) return;
  enCours = true;
  try {
    for (;;) {
      const l = (await api.nexusLiens().catch(() => [] as LienNexus[])).find((x) => !proposes.has(x.n));
      if (!l) break;
      proposes.add(l.n);
      await traiterUnLien(l);
    }
  } finally {
    enCours = false;
  }
}

async function traiterUnLien(l: LienNexus): Promise<void> {
  const oublier = () => api.nexusOublierLien(l.n).catch(() => {});
  if (l.jeu_id === null) {
    await informer(
      '🧩 Un mod de Nexus est arrivé',
      'Frogtend ne sait pas à quel jeu il va. Ouvre la page du mod depuis le jeu (⚙ Gérer le jeu ▸ 🧩 Mods ▸ « Ouvrir sur Nexus »), puis reclique « Mod Manager Download ».',
    );
    await oublier();
    return;
  }
  let infos: InfosNexus;
  try {
    infos = await api.nexusLienInfos(l.n);
  } catch (err) {
    const sansCle = !(await api.nexusEtat().catch(() => null))?.cle_enregistree;
    if (!sansCle) {
      await informer('🧩 Mod de Nexus', `Impossible de le préparer : ${motifDuRefus(err)}`);
    } else if (
      (await confirmer('🔑 Ta clé Nexus d’abord', { message: 'Pour installer ce mod, Frogtend a besoin de ta clé Nexus (gratuite, dans ton compte Nexus).', libelleValider: '🔑 Régler ma clé' })) &&
      (await reglerNexus())
    ) {
      return traiterUnLien(l);
    }
    await oublier();
    return;
  }
  if (infos.expire) {
    await informer('🧩 Lien expiré', 'Ce lien de Nexus a expiré : reclique « Mod Manager Download » sur la page du mod.');
    await oublier();
    return;
  }
  if (!infos.zip) {
    await informer(
      `🧩 « ${infos.fichier} »`,
      'Ce fichier est en 7z ou rar : Frogtend ne sait poser que les .zip pour l’instant. Choisis un fichier .zip sur la page du mod s’il y en a un, ou installe-le à la main.',
    );
    await oublier();
    return;
  }
  // Nexus ne dit pas où poser le mod : la page le dit (en cas de doute, la lire d'abord).
  const page = `https://www.nexusmods.com/${l.domaine}/mods/${l.mod_id}?tab=description`;
  let programme = false;
  for (;;) {
    const o = await choisir<'racine' | 'programme' | 'page'>(
      `🧩 Où poser « ${infos.mod_nom} » dans « ${infos.titre} » ?`,
      [
        { valeur: 'racine', libelle: '📁 Dans le dossier du jeu', detail: 'le plus courant' },
        { valeur: 'programme', libelle: '🎮 À côté du programme du jeu', detail: 'si la page dit « à côté de l’exe »' },
        { valeur: 'page', libelle: '📖 Lire les consignes de la page d’abord' },
      ],
      'La page du mod dit où le poser. En cas de doute, lis-la d’abord.',
    );
    if (!o) {
      await oublier();
      toast(`« ${infos.mod_nom} » laissé de côté : reclique « Mod Manager Download » sur Nexus pour le reprendre.`, 'neutre');
      return;
    }
    if (o === 'page') {
      await ouvrirPageWeb(page);
      continue;
    }
    programme = o === 'programme';
    break;
  }
  const message = [
    `Fichier : ${infos.fichier}, environ ${taille(infos.taille_ko * 1024)}, téléchargé depuis Nexus Mods avec ton compte.`,
    'Frogtend vérifie auprès de Nexus que c’est bien le bon fichier avant de l’installer.',
    'Tout fichier du jeu remplacé est gardé de côté, et tes parties sont mises à l’abri quand Frogtend sait où elles sont (jeu installé par Frogtend).',
  ].join('\n\n');
  if (!(await confirmer(`🧩 Installer « ${infos.mod_nom} » dans « ${infos.titre} » ?`, { message, libelleValider: '⬇ Télécharger et installer' }))) {
    await oublier();
    return;
  }
  const t = toast(`🧩 Installation de « ${infos.mod_nom} »… (téléchargement, puis pose ; ne ferme pas Frogtend)`, 'neutre', { jusquaFermeture: true });
  try {
    const b = await api.nexusInstaller(l.n, programme);
    fermerToast(t);
    toast(`🧩 « ${infos.mod_nom} » installé : ${b.ajoutes} fichier(s) ajouté(s), ${b.remplaces} remplacé(s) (gardés de côté).`);
  } catch (err) {
    fermerToast(t);
    // Pour réessayer : recliquer sur le site (la clé du lien ne sert qu'une fois).
    await oublier();
    toast(`🧩 « ${infos.mod_nom} » non installé : ${motifDuRefus(err)}`, 'erreur');
  }
}
