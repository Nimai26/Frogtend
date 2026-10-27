// « Prêt à jouer » : l'enchaînement (les décisions sont dans `pret.ts`). Appelé quand des jeux d'une console arrivent
// (import, téléchargement) et quand on veut jouer sans émulateur : UN accord, puis Frogtend installe l'émulateur
// recommandé, son cœur RetroArch s'il en faut un, règle la manette, et retient le tout pour la console.

import { api } from '$lib/api';
import { taille } from '$lib/api';
import { confirmer, informer, toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { etat, reglerPc } from '$lib/etat.svelte';
import { completerRetroArch, installerEmulateur, installerMicrologicielPs3, installerParFirehouse, poserBios, recommandationsParDefaut, reglerEmulateur } from './assistant.svelte';
import { ajouter } from './choix';
import { avecConseil, biosADireOuPoser, biosRetroArch, choisirSeul, dossierParDefaut, estConseille, ligneDe, MANETTE_REGLEE, messagePret, micrologicielServi, phraseVersionAutre, poidsBios, versionAutre, type Possible } from './pret';
import { nomAffiche } from '$lib/ludotheque/ludotheque.svelte';

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

  const reponse = await api.emulateursRecommandes(plateforme).catch(() => ({ emulateurs: [], conseil: null }));
  // Le conseil de la maison PRIME (contrat 1.8) ; la liste LaunchBox vient ensuite. Le choix d'office de Frogtend ne
  // sert que si Firehouse ne dit rien du tout (hors ligne, ou ni conseil ni liste).
  const conseil = reponse.conseil ?? null;
  let recs = avecConseil(reponse.emulateurs ?? [], conseil);
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
      // Firehouse sert le paquet de l'émulateur de SON conseil (contrat 1.8 : `/emulateurs/{id}/paquet`), même sans source dans la liste.
      parFirehouse: !f && !!rec.id && ((!!rec.telechargement && rec.telechargement.type !== 'page') || estConseille(rec, conseil)),
    };
    lignes.set(rec, ligneDe(rec, f?.ligne ?? '', conseil));
    possibles.push(p);
  }
  const choix = choisirSeul(possibles, conseil);
  // Rien que Frogtend sache préparer seul : l'assistant, qui montre les possibilités.
  if (!choix) return reglerEmulateur(plateforme);

  const dossierNouveau = !etat.pc.dossierEmulateurs;
  // Sur un disque qui existe sur CE PC (celui des jeux, sinon le plus libre) — jamais une lettre supposée.
  const propose = etat.pc.dossierEmulateurs ? null : await api.dossierJeuxPropose().catch(() => null);
  const dossier = etat.pc.dossierEmulateurs || dossierParDefaut(etat.pc.emplacements.defaut, propose?.disque, propose?.disques);
  // Aucun disque connu et un émulateur à installer : l'assistant, qui fait choisir le dossier.
  if (!dossier && choix.comment !== 'deja') return reglerEmulateur(plateforme);
  const paquet = choix.comment === 'frogtend' && choix.id ? await api.emulateurDerniereVersion(choix.id).catch(() => null) : null;
  // Celui du conseil, déjà sur le PC, mais pas dans la version attendue par les jeux de Firehouse (MAME) : on le dit.
  const installee = choix.programme ? installes.find((i) => i.programme === choix.programme)?.version : null;
  const autre = estConseille(choix.rec, conseil) ? versionAutre(installee, conseil) : null;
  const oui = await confirmer(`🎮 Préparer ${nomAffiche(plateforme)} pour jouer ?`, {
    message: (autre && installee ? `${phraseVersionAutre(choix.nom, installee, autre)}\n\n` : '') + messagePret({
      plateforme: nomAffiche(plateforme),
      jeux,
      choix,
      taille: paquet?.taille ? taille(paquet.taille) : null,
      deFirehouse: !!paquet && !/^https?:/i.test(paquet.url),
      dossier,
      dossierNouveau,
      conseil,
      tailleBios: conseil && estConseille(choix.rec, conseil) && poidsBios(conseil.bios) ? taille(poidsBios(conseil.bios)) : null,
    }),
    libelleValider: choix.comment === 'deja' ? '🎮 C’est parti' : '🎮 Tout préparer',
  });
  if (!oui) return null;
  if (dossierNouveau && dossier) await reglerPc('dossierEmulateurs', dossier);

  let programme = choix.programme ?? null;
  if (choix.comment === 'frogtend' && choix.id) programme = (await installerEmulateur(choix.id, choix.nom, false, { sansQuestion: true }))?.programme ?? null;
  if (choix.comment === 'firehouse' && choix.rec.id) programme = (await installerParFirehouse(choix.rec.id, choix.nom, false, { sansQuestion: true }))?.programme ?? null;
  if (!programme) {
    // L'installation a échoué : le dossier proposé n'est pas gardé (au prochain essai, il est proposé de nouveau).
    if (dossierNouveau && etat.pc.dossierEmulateurs === dossier) await reglerPc('dossierEmulateurs', '');
    return null;
  }

  const ligne = lignes.get(choix.rec) ?? '';
  let nom = choix.nom;
  // Les fichiers système du conseil, posés et vérifiés par le cœur (compris dans l'accord) AVANT que RetroArch ne
  // vérifie ce qui lui manque.
  if (conseil && estConseille(choix.rec, conseil) && biosADireOuPoser(conseil.bios)) await poserBios(plateforme, programme);
  if (choix.id === 'retroarch') {
    const n = await completerRetroArch(programme, ligne, biosRetroArch(choix.rec, conseil), plateforme, nom, true);
    if (!n) return null;
    nom = n;
  }
  const { sys, cle } = ajouter(etat.pc.emulateurs[plateforme], {
    nom,
    programme,
    ligne,
    ...(estConseille(choix.rec, conseil) ? { conseil: conseil!.maj_le } : {}),
  });
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
  // Des parties restaurées (PC neuf, formaté) attendaient cet émulateur : reposées maintenant, jamais par-dessus une autre.
  const r = await api.restaurationReposerEmulateurs().catch(() => null);
  const parties = r?.reposes ? `, ${r.reposes} partie(s) restaurée(s) reposée(s)` : '';
  toast(`✅ ${nomAffiche(plateforme)} est prêt : ${nom}${manette}${parties}.`);
  // RPCS3 sans micrologiciel ne lance aucun jeu du commerce : celui de Firehouse est installé (compris dans l'accord) ;
  // s'il n'est pas servi, on le dit tout de suite, simplement.
  if (choix.id === 'rpcs3' && !(await api.rpcs3Micrologiciel(programme).catch(() => null))) {
    const servi = conseil && estConseille(choix.rec, conseil) ? micrologicielServi(conseil.bios) : undefined;
    if (servi) {
      await installerMicrologicielPs3(programme);
    } else {
      await informer(
        '💿 Il manque encore le micrologiciel PS3',
        'Les jeux PS3 en ont besoin, et Firehouse ne le fournit pas en ce moment.\n\nTu peux l’installer toi-même : ⚙ Options ▸ 🕹 Émulateurs ▸ 💿 sur la ligne de RPCS3, avec le fichier PS3UPDAT.PUP du site officiel de PlayStation (quelques secondes, sans fenêtre).',
      );
    }
  }
  return cle;
}
