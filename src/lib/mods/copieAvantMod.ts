// Avant de modifier un jeu (un mod, de GitHub, de Nexus ou d'une page) : proposer d'abord une copie du jeu — pas
// obligatoire, avec un avertissement si la personne n'en veut pas. Un jeu de boutique n'est jamais copié en entier
// (Seb, 21/10) : sa boutique sait le réparer.

import { api, taille } from '$lib/api';
import { confirmer, fermerToast, toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { tele } from '$lib/ludotheque/telechargements.svelte';

/** Les jeux déjà copiés (ou dont la copie a été refusée en connaissance de cause) depuis l'ouverture de Frogtend : on
 * ne repropose pas une copie complète à chaque mod installé à la suite. */
const dejaDecide = new Set<number>();

/** Rend `true` pour continuer (copie faite, ou refusée en connaissance de cause), `false` si la personne renonce ou si
 * la copie demandée a échoué. */
export async function proposerCopieAvantMod(id: number): Promise<boolean> {
  const i = tele.jeux[id]?.installation;
  if (!i || i.boutique || dejaDecide.has(id)) return true;
  const octets = await api.jeuTailleInstallation(id).catch(() => null);
  const copier = await confirmer('💾 Copier le jeu avant de le modifier ?', {
    message: `Conseillé : un mod peut abîmer le jeu. La copie prend ${octets !== null ? taille(octets) : 'la taille du jeu'}, dans le dossier du jeu (sous « Frogtend\\copies »).`,
    libelleValider: '💾 Copier d’abord',
  });
  if (copier) {
    const attente = toast('💾 Copie du jeu en cours… (ne ferme pas Frogtend)', 'neutre', { jusquaFermeture: true });
    try {
      const c = await api.jeuCopieAvantMod(id);
      dejaDecide.add(id);
      toast(`✅ Copie faite : ${c}`);
      return true;
    } catch (e) {
      toast(`La copie a échoué : ${motifDuRefus(e)}`, 'erreur');
      return false;
    } finally {
      fermerToast(attente);
    }
  }
  const quandMeme = await confirmer('⚠ Sans copie', {
    message:
      'Si le mod abîme le jeu, il faudra le réinstaller. Tes parties restent sauvegardées quand Frogtend sait où elles sont (jeu installé par Frogtend).',
    libelleValider: 'Continuer sans copie',
    danger: true,
  });
  if (quandMeme) dejaDecide.add(id);
  return quandMeme;
}
