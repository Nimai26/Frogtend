// Où ranger les jeux quand rien n'est réglé (PC neuf, ancien dossier absent après un formatage) : Frogtend PROPOSE
// un dossier sur un disque qui existe vraiment (le plus libre), la personne dit oui ou en choisit un autre. Jamais une
// lettre de disque supposée, jamais une étape « va dans les Options ».

import { api, taille } from '$lib/api';
import { choisir, informer, toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { etat, reglerPc } from '$lib/etat.svelte';
import { parcourir } from '$lib/emulateurs/assistant.svelte';
import { auCanape } from '$lib/taodbox/canape';

/** Demande où ranger les jeux, l'enregistre en tête des dossiers par défaut et le rend (`null` : pas maintenant).
 * `raison` : pourquoi on le demande (dit à la personne) ; `besoin` : la place que demande le jeu, s'il y en a un. */
export async function proposerDossierJeux(raison: string, besoin?: number): Promise<string | null> {
  const p = await api.dossierJeuxPropose().catch(() => null);
  // Au canapé, pas d'explorateur de Windows : seulement le dossier proposé (sinon, à faire à l'écran du PC).
  const canape = auCanape();
  if (!p && canape) {
    await informer('🖥 À faire sur l’écran du PC', `${raison}\nChoisis le dossier de tes jeux à l’écran du PC (⚙ Options ▸ 📁 Emplacements).`);
    return null;
  }
  const options: { valeur: 'propose' | 'autre'; libelle: string; detail?: string }[] = [];
  if (p) options.push({ valeur: 'propose', libelle: `📁 ${p.chemin}`, detail: `${taille(p.libre)} libres sur ce disque${p.externe ? ' — un disque externe (USB) : garde-le branché pour jouer' : ''}` });
  if (!canape) options.push({ valeur: 'autre', libelle: '📂 Choisir un autre dossier…', detail: 'Avec l’explorateur de Windows.' });
  let chemin: string | null = null;
  // Un dossier refusé (racine d'un disque, Windows, Documents) : dit, puis la même question revient (jamais
  // l'installation en cours perdue).
  for (;;) {
    const c = await choisir(
      '📁 Où ranger tes jeux ?',
      options,
      `${raison}${besoin ? `\nCe jeu demande ${taille(besoin)}.` : ''}\nChaque jeu y aura son propre dossier. Tu pourras en ajouter d’autres plus tard (⚙ Options ▸ Emplacements).`,
    );
    chemin = null;
    if (c === 'propose' && p) {
      chemin = await api.dossierJeuxCreer(p.chemin).catch((e) => {
        toast(`Le dossier n’a pas pu être créé : ${motifDuRefus(e)}`, 'erreur');
        return null;
      });
    }
    if (c === 'autre') chemin = await parcourir({ dossier: true, titre: 'Dossier de tes jeux' });
    if (!chemin) return null;
    const refus = await api.dossierJeuxRefus(chemin).catch(() => null);
    if (!refus) break;
    await informer('📁 Choisis un autre dossier', refus);
  }
  const e = etat.pc.emplacements;
  await reglerPc('emplacements', { ...e, defaut: [chemin, ...e.defaut.filter((d) => d.toLowerCase() !== chemin!.toLowerCase())] });
  return chemin;
}
