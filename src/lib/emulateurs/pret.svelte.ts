// « Prêt à jouer » : l'enchaînement (les décisions sont dans `pret.ts`). Appelé quand des jeux d'une console arrivent
// (import, téléchargement) et quand on veut jouer sans émulateur : UN accord, puis Frogtend installe l'émulateur
// recommandé, son cœur RetroArch s'il en faut un, règle la manette, et retient le tout pour la console.

import { api } from '$lib/api';
import { taille } from '$lib/api';
import { confirmer, informer, toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { etat, reglerPc } from '$lib/etat.svelte';
import { completerRetroArch, installerEmulateur, installerParFirehouse, recommandationsParDefaut, reglerEmulateur } from './assistant.svelte';
import { ajouter } from './choix';
import { choisirSeul, dossierParDefaut, MANETTE_REGLEE, messagePret, type Possible } from './pret';

/** Les plateformes qui se jouent sans émulateur. */
const SANS_EMULATEUR = ['Windows'];

/**
 * Prépare une console pour jouer. Rend la clé de son émulateur (déjà réglé, ou préparé maintenant), ou `null` si la
 * personne n'a pas voulu ou si quelque chose a échoué (elle l'a lu dans un message).
 */
export async function preparerPourJouer(plateforme: string, jeux: number): Promise<string | null> {
  const deja = etat.pc.emulateurs[plateforme];
  if (deja?.liste?.length) return deja.defaut || deja.liste[0].cle;
  if (SANS_EMULATEUR.includes(plateforme)) return null;

  let recs = (await api.emulateursRecommandes(plateforme).catch(() => ({ emulateurs: [] }))).emulateurs ?? [];
  if (!recs.length) recs = recommandationsParDefaut(plateforme);
  const installes = await api.emulateursInstalles().catch(() => []);
  const lignes = new Map<unknown, string>();
  const possibles: Possible[] = [];
  for (const rec of recs) {
    const f = await api.emulateurFiche(rec.nom).catch(() => null);
    const id = f?.id ?? null;
    const installe = installes.find((i) => i.id === (id ?? rec.id));
    const p: Possible = {
      rec,
      id,
      nom: f?.nom ?? rec.nom,
      programme: installe?.programme,
      parFirehouse: !f && !!rec.id && !!rec.telechargement && rec.telechargement.type !== 'page',
    };
    lignes.set(rec, rec.ligne_de_commande || f?.ligne || '');
    possibles.push(p);
  }
  const choix = choisirSeul(possibles);
  // Rien que Frogtend sache préparer seul : l'assistant, qui montre les possibilités.
  if (!choix) return reglerEmulateur(plateforme);

  const dossierNouveau = !etat.pc.dossierEmulateurs;
  const dossier = etat.pc.dossierEmulateurs || dossierParDefaut(etat.pc.emplacements.defaut);
  const paquet = choix.comment === 'frogtend' && choix.id ? await api.emulateurDerniereVersion(choix.id).catch(() => null) : null;
  const oui = await confirmer(`🎮 Préparer ${plateforme} pour jouer ?`, {
    message: messagePret({ plateforme, jeux, choix, taille: paquet?.taille ? taille(paquet.taille) : null, dossier, dossierNouveau }),
    libelleValider: choix.comment === 'deja' ? '🎮 C’est parti' : '🎮 Tout préparer',
  });
  if (!oui) return null;
  if (dossierNouveau) await reglerPc('dossierEmulateurs', dossier);

  let programme = choix.programme ?? null;
  if (choix.comment === 'frogtend' && choix.id) programme = (await installerEmulateur(choix.id, choix.nom, false, { sansQuestion: true }))?.programme ?? null;
  if (choix.comment === 'firehouse' && choix.rec.id) programme = (await installerParFirehouse(choix.rec.id, choix.nom, false, { sansQuestion: true }))?.programme ?? null;
  if (!programme) return null;

  const ligne = lignes.get(choix.rec) ?? '';
  let nom = choix.nom;
  if (choix.id === 'retroarch') {
    const n = await completerRetroArch(programme, ligne, choix.rec.bios ? [choix.rec.bios] : [], plateforme, nom, true);
    if (!n) return null;
    nom = n;
  }
  const { sys, cle } = ajouter(etat.pc.emulateurs[plateforme], { nom, programme, ligne });
  await reglerPc('emulateurs', { ...etat.pc.emulateurs, [plateforme]: sys });

  let manette = '';
  if (choix.id && MANETTE_REGLEE.includes(choix.id)) {
    try {
      await api.emulateurReglerManette(choix.id, programme);
      manette = ', manette réglée';
    } catch (e) {
      toast(`${nom} est prêt, mais la manette n’a pas pu être réglée : ${motifDuRefus(e)}`, 'alerte');
    }
  }
  toast(`✅ ${plateforme} est prêt : ${nom}${manette}.`);
  // RPCS3 sans micrologiciel ne lance aucun jeu du commerce : le dire tout de suite, simplement.
  if (choix.id === 'rpcs3' && !(await api.rpcs3Micrologiciel(programme).catch(() => null))) {
    await informer(
      '💿 Il manque encore le micrologiciel PS3',
      'Les jeux PS3 en ont besoin. Bientôt, Firehouse le fournira et Frogtend l’installera tout seul.\n\nEn attendant : ⚙ Options ▸ 🕹 Émulateurs ▸ 💿 sur la ligne de RPCS3, avec le fichier PS3UPDAT.PUP du site officiel de PlayStation (quelques secondes, sans fenêtre).',
    );
  }
  return cle;
}
