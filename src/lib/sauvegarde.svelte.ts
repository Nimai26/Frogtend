// La sauvegarde du profil ouvert chez Firehouse : à la demande, après chaque partie, ou chaque jour (réglage du
// profil). Une sauvegarde automatique est discrète : elle ne dit rien si tout va bien, et signale un échec sans
// bloquer (elle sera refaite à la prochaine occasion).

import { listen } from '@tauri-apps/api/event';
import { isTauri } from '@tauri-apps/api/core';
import { api, taille, type ConflitRestaure } from '$lib/api';
import { choisir, confirmer, informer, toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { etat, reglerPc, remplacerReglagesProfil } from '$lib/etat.svelte';
import { ajouter, normaliserTous } from '$lib/emulateurs/choix';
import { depuis } from '$lib/ludotheque/ludotheque.svelte';
import { rechargerJeuxDuPc, tele } from '$lib/ludotheque/telechargements.svelte';
import { proposerDossierJeux } from '$lib/ludotheque/dossierJeux';
import { aAnnoncer, dossierDesAbris, dossierDesCopies } from './sauvegardeTaille';

export interface BilanSauvegarde {
  envoyes: number;
  inchanges: number;
  retires: number;
  octets: number;
  fichiers: number;
  taille: number;
  date: string;
  pc: string;
  dossier: string;
  /** Jeux dont les parties restent seulement sur ce PC (ce profil ne les voit pas chez Firehouse) — jamais nommés. */
  non_envoyes?: number;
}

/** Une sauvegarde quotidienne est refaite au-delà de ce délai. */
const JOUR_S = 24 * 3600;

export const sauvegarde = $state<{
  enCours: boolean;
  derniere: BilanSauvegarde | null;
  restauration: { faits: number; total: number } | null;
}>({
  enCours: false,
  derniere: null,
  restauration: null,
});

export async function lireDerniereSauvegarde() {
  sauvegarde.derniere = await api.derniereSauvegarde().catch(() => null);
}

/** Le début du refus du cœur quand une sauvegarde existante n'a été ni restaurée ni écartée (`sauvegarde.rs`). */
export const A_REPRENDRE = 'Une sauvegarde de ce profil existe déjà chez Firehouse';

/** Déjà proposé pendant cette ouverture du profil (une sauvegarde automatique ne redemande pas à chaque partie). */
let repriseProposee = '';

/** Une sauvegarde de ce profil existe chez Firehouse et ce PC ne l'a jamais reprise : rien n'est envoyé tant que la
 * personne n'a pas choisi (l'envoyer la viderait de ce qui manque ici). */
export async function proposerReprise(motif: string, discrete: boolean) {
  const profil = etat.profilOuvert?.id ?? '';
  if (discrete && repriseProposee === profil) return;
  repriseProposee = profil;
  const c = await choisir(
    '📥 Une sauvegarde de ce profil existe déjà',
    [
      { valeur: 'restaurer', libelle: '📥 La restaurer sur ce PC', detail: 'Tes réglages, tes parties, puis tes jeux.' },
      { valeur: 'remplacer', libelle: '♻ La remplacer par ce PC', detail: 'Les parties qui ne sont pas sur ce PC seront retirées de la sauvegarde.' },
      { valeur: 'plus_tard', libelle: '⏳ Plus tard', detail: 'Rien n’est envoyé en attendant.' },
    ],
    motif,
  );
  if (c === 'restaurer') return restaurer();
  if (c !== 'remplacer') return;
  const oui = await confirmer('♻ Remplacer la sauvegarde de Firehouse ?', {
    message:
      'La sauvegarde de Firehouse ne contiendra plus que ce qui est sur CE PC : les parties qui n’y sont pas en sortiront (Firehouse en garde seulement un historique).',
    libelleValider: '♻ Remplacer',
  });
  if (oui) await sauvegarder(false, true);
}

/** Une grosse sauvegarde automatique déjà remise à plus tard (et dite) pendant cette session. */
let grosseRemise = false;

/** Sauvegarde maintenant. `discrete` : pas de message si tout va bien. `ecarter` : remplacer une sauvegarde existante
 * non restaurée (choix explicite de la personne). */
export async function sauvegarder(discrete = false, ecarter = false): Promise<boolean> {
  if (sauvegarde.enCours || etat.pc.firehouse.simule) return false;
  sauvegarde.enCours = true;
  try {
    // Une grosse sauvegarde (une première sauvegarde PS3 ou Xbox : des centaines de Mo) est ANNONCÉE avant d'envoyer :
    // demandée à la main, remise à plus tard en automatique. Une estimation impossible n'empêche rien.
    // Après « remplacer une sauvegarde » (`ecarter`), la taille a déjà été acceptée : jamais redemandée.
    const est = ecarter ? null : await api.sauvegardeEstimer().catch(() => null);
    const gros = est ? aAnnoncer(est, etat.pc.seuilSauvegardeMo ?? 200) : null;
    if (gros !== null && discrete) {
      // Dit une fois par ouverture de Frogtend, pas après chaque partie.
      if (!grosseRemise) {
        grosseRemise = true;
        toast(`💾 Sauvegarde automatique remise à plus tard : au moins ${taille(gros)} de parties à envoyer. Lance-la toi-même (💾 Sauvegarder) quand ta connexion le permet.`, 'alerte');
      }
      return false;
    }
    if (gros !== null) {
      const oui = await confirmer(`💾 Envoyer au moins ${taille(gros)} de parties à Firehouse ?`, {
        message: `Tes parties ont beaucoup grossi depuis la dernière sauvegarde${est?.precedent == null ? ' (c’est la première depuis ce PC)' : ''}. Sur une connexion modeste, l’envoi peut prendre du temps ; tu peux continuer à utiliser Frogtend pendant ce temps.`,
        libelleValider: 'Envoyer',
      });
      if (!oui) return false;
    }
    const b = await api.sauvegarder(ecarter);
    sauvegarde.derniere = b;
    if (!discrete) {
      const seulsIci = b.non_envoyes
        ? ` Les parties de ${b.non_envoyes} jeu(x) que ce profil ne voit pas restent seulement sur ce PC.`
        : '';
      toast(
        (b.envoyes
          ? `✅ Sauvegardé : ${b.envoyes} fichier(s) envoyé(s), ${b.inchanges} déjà à jour (${b.fichiers} en tout).`
          : `✅ Sauvegarde à jour : ${b.fichiers} fichier(s) de parties, rien n’avait changé.`) + seulsIci,
      );
    }
    return true;
  } catch (e) {
    const motif = motifDuRefus(e);
    if (motif.includes(A_REPRENDRE)) {
      void proposerReprise(motif, discrete);
      return false;
    }
    // Firehouse sans la sauvegarde (avant le contrat 1.5) : une sauvegarde automatique se tait.
    if (discrete && motif.includes('ne propose pas encore')) return false;
    // Ludothèque pas encore synchronisée (profil neuf, autre compte) : la sauvegarde automatique attend la suivante.
    if (discrete && motif.includes('Synchronise d')) return false;
    toast(`${discrete ? '⚠ Sauvegarde automatique non faite' : 'Échec de la sauvegarde'} : ${motif}`, discrete ? 'alerte' : 'erreur');
    return false;
  } finally {
    sauvegarde.enCours = false;
  }
}

/** Après une partie : sauvegarde si le profil l'a choisi. */
export async function apresUnePartie() {
  if (etat.profil.sauvegarde.auto === 'apres_partie') await sauvegarder(true);
}

/** À l'ouverture du profil : propose une restauration (profil jamais sauvegardé d'ici, sauvegarde à son nom), puis
 * sauvegarde quotidienne si le profil l'a choisi et que la dernière est ancienne. */
export async function auDemarrage() {
  await lireDerniereSauvegarde();
  if (!sauvegarde.derniere && !etat.pc.firehouse.simule && Object.keys(tele.jeux).length === 0) {
    // Liste illisible (hors ligne…) : on ne conclut JAMAIS « aucune sauvegarde » — pas de sauvegarde automatique cette fois.
    const liste = await api.restaurationListe().catch(() => null);
    if (!liste) return;
    // Le cœur marque celles rangées au nom du profil ouvert (la même règle qu'à l'envoi).
    const a_moi = liste.filter((s) => s.a_moi);
    if (a_moi.length) {
      const s = a_moi[0];
      const oui = await confirmer(`📥 Une sauvegarde de « ${s.profil} » existe`, {
        message: `Depuis ${s.pc}${s.derniere?.date ? `, ${depuis(Number(s.derniere.date))}` : ''}.\nLa restaurer sur ce PC (réglages, parties, puis tes jeux) ?`,
        libelleValider: '📥 Restaurer',
      });
      if (oui) {
        await restaurer(s.profil, s.pc);
        return;
      }
      // Déjà demandé : la sauvegarde automatique, refusée par le cœur, ne repose pas la question tout de suite.
      repriseProposee = etat.profilOuvert?.id ?? '';
    } else if (liste.length) {
      // PC neuf ou formaté, profil recréé sous un autre nom : les sauvegardes du COMPTE (même avec un nouveau jeton).
      const oui = await confirmer('📥 Des sauvegardes de ton compte existent', {
        message: `${liste.length} sauvegarde(s) faite(s) depuis un autre profil ou un autre PC. En restaurer une ici (réglages, parties, puis tes jeux) ?`,
        libelleValider: '📥 Voir les sauvegardes',
      });
      if (oui) {
        await restaurer();
        return;
      }
    }
  }
  if (etat.profil.sauvegarde.auto === 'manuelle') return;
  const age = sauvegarde.derniere ? Date.now() / 1000 - Number(sauvegarde.derniere.date) : Infinity;
  if (age > JOUR_S) await sauvegarder(true);
}

/** Restaurer une sauvegarde : on choisit laquelle (si besoin), on explique, puis réglages, parties, et les jeux. */
export async function restaurer(profilChoisi?: string, pcChoisi?: string) {
  let profil = profilChoisi;
  let pc = pcChoisi;
  if (!profil || !pc) {
    const liste = await api.restaurationListe().catch((e) => {
      toast(`Impossible de lire les sauvegardes : ${motifDuRefus(e)}`, 'erreur');
      return null;
    });
    if (!liste) return;
    if (liste.length === 0) {
      await informer('📭 Aucune sauvegarde', 'Firehouse n’a encore aucune sauvegarde pour ton compte.');
      return;
    }
    const c = await choisir(
      '📥 Quelle sauvegarde restaurer ?',
      liste.map((s) => ({
        valeur: s,
        libelle: `${s.profil} — ${s.pc}`,
        detail: [s.derniere?.date ? depuis(Number(s.derniere.date)) : null, s.derniere?.fichiers != null ? `${s.derniere.fichiers} fichier(s)` : null, taille(s.octets)]
          .filter(Boolean)
          .join(' · '),
      })),
    );
    if (!c) return;
    ({ profil, pc } = c);
    const oui = await confirmer(`📥 Restaurer « ${profil} — ${pc} » ?`, {
      message: [
        'Tes réglages de profil seront remplacés par ceux de la sauvegarde.',
        'Tes parties d’émulateurs seront reposées ; une partie DIFFÉRENTE déjà sur ce PC est gardée telle quelle.',
        'Les parties de tes jeux attendront que chaque jeu soit réinstallé.',
        'Ensuite, je te proposerai de remettre tes jeux dans ta ludothèque.',
      ].join('\n'),
      libelleValider: '📥 Restaurer',
    });
    if (!oui) return;
  }

  sauvegarde.restauration = { faits: 0, total: 0 };
  const arreter = isTauri()
    ? await listen<{ faits: number; total: number }>('restauration', (e) => (sauvegarde.restauration = e.payload))
    : () => {};
  let prete, reposes;
  try {
    [prete, reposes] = await api.restaurationPreparer(profil!, pc!);
  } catch (e) {
    toast(`Échec de la restauration : ${motifDuRefus(e)}`, 'erreur');
    return;
  } finally {
    arreter();
    sauvegarde.restauration = null;
  }

  // Les réglages : ceux du profil en entier ; ceux du PC seulement là où ce PC n'a encore rien, et s'ils existent ici.
  const conf = prete.configuration ?? {};
  if (conf.reglages_profil?.reglages) await remplacerReglagesProfil(conf.reglages_profil.reglages);
  const pcLu = conf.reglages_pc?.reglages ?? {};
  const existe = async (chemin: string) => (await api.espaceLibre(chemin).catch(() => null)) !== null;
  // Les anciens dossiers absents de ce PC (formaté, neuf, autres disques) : jamais abandonnés en silence.
  const absents: string[] = [];
  if (etat.pc.emplacements.defaut.length === 0 && Object.keys(etat.pc.emplacements.systemes).length === 0 && pcLu.emplacements) {
    const defaut = [];
    for (const d of pcLu.emplacements.defaut ?? []) if (await existe(d)) defaut.push(d);
    else absents.push(d);
    const systemes: Record<string, string[]> = {};
    for (const [s, l] of Object.entries(pcLu.emplacements.systemes ?? {})) {
      const ok = [];
      for (const d of l as string[]) if (await existe(d)) ok.push(d);
      else absents.push(d);
      if (ok.length) systemes[s] = ok;
    }
    await reglerPc('emplacements', { defaut, systemes });
  }
  if (!etat.pc.dossierEmulateurs && pcLu.dossierEmulateurs && (await existe(pcLu.dossierEmulateurs))) {
    await reglerPc('dossierEmulateurs', pcLu.dossierEmulateurs);
  }
  // Les émulateurs de chaque système : ceux de la sauvegarde dont le dossier existe sur ce PC, ajoutés à ceux déjà là.
  const emus = { ...etat.pc.emulateurs };
  for (const [s, sys] of Object.entries(normaliserTous(pcLu.emulateurs))) {
    const deja = !!emus[s];
    for (const e of sys.liste) {
      const dossier = e.programme.replace(/[\\/][^\\/]+$/, '');
      if (!dossier || !(await existe(dossier))) continue;
      const { sys: apres, cle } = ajouter(emus[s], e);
      emus[s] = !deja && e.cle === sys.defaut ? { ...apres, defaut: cle } : apres;
    }
  }
  await reglerPc('emulateurs', emus);

  toast(
    `✅ Restauré : réglages, ${prete.fichiers} fichier(s) de parties (${taille(prete.taille)}).` +
      (reposes.reposes ? ` ${reposes.reposes} partie(s) d’émulateur reposée(s).` : ''),
  );
  // Les parties communes à tous (déjà là et différentes) sont seulement dites ; celles de la personne, proposées.
  const autres = reposes.conflits.filter((c) => !(reposes.a_remplacer ?? []).some((a) => a.etiquette === c));
  if (autres.length) {
    await informer(
      '⚠ Parties gardées telles quelles',
      `Ces parties existaient déjà sur ce PC avec un contenu différent ; elles n’ont pas été remplacées :\n${autres.slice(0, 12).join('\n')}${autres.length > 12 ? `\n… et ${autres.length - 12} autre(s)` : ''}`,
    );
  }
  await proposerRemplacements(reposes.a_remplacer ?? []);

  // Les jeux : on propose de les remettre (annoncé et chiffré d'abord).
  await rechargerJeuxDuPc();
  const aRemettre = (prete.bibliotheque?.jeux ?? []).filter((j: any) => !tele.jeux[j.id]);
  if (!aRemettre.length) return;
  // Aucun dossier de jeux sur ce PC (les anciens n'existent plus) : on demande où les ranger, avant de rien chiffrer.
  if (etat.pc.emplacements.defaut.length === 0) {
    const raison = absents.length
      ? `Tes jeux étaient rangés dans ${[...new Set(absents)].slice(0, 3).join(', ')} : ${absents.length > 1 ? 'ces dossiers n’existent' : 'ce dossier n’existe'} pas sur ce PC.`
      : 'Frogtend ne sait pas encore où ranger tes jeux sur ce PC.';
    if (!(await proposerDossierJeux(raison))) {
      await informer('🎮 Tes jeux plus tard', 'Tes jeux pourront être remis quand un dossier sera choisi : ils restent dans ta ludothèque Firehouse, et leurs parties attendent sur ce PC.');
      return;
    }
  }
  const total = aRemettre.reduce((s: number, j: any) => s + (j.total ?? 0), 0);
  const oui = await confirmer(`🎮 Remettre ${aRemettre.length} jeu(x) dans ta ludothèque ?`, {
    message: `${taille(total)} à télécharger, dans tes emplacements. Les parties de chaque jeu seront reposées dès qu’il sera réinstallé.\n\n${aRemettre.slice(0, 10).map((j: any) => `• ${j.titre} (${taille(j.total ?? 0)})`).join('\n')}${aRemettre.length > 10 ? `\n… et ${aRemettre.length - 10} autre(s)` : ''}`,
    libelleValider: '⬇ Tout remettre',
  });
  if (!oui) return;
  let ok = 0;
  const echecs: string[] = [];
  for (const j of aRemettre) {
    try {
      const places = await api.proposerEmplacements(j.plateforme, j.total ?? 0);
      const place = places.find((p) => p.assez);
      if (!place) throw new Error('aucun emplacement avec assez de place');
      await api.ajouterJeu(j.id, j.version, place.chemin);
      // Un jeu à cartes (e-Reader) : ses autres cartes aussi, sur la même ligne.
      for (const v of (j.cartes ?? []) as number[]) {
        if (v !== j.version) await api.ajouterJeu(j.id, v, place.chemin).catch((e) => echecs.push(`${j.titre} (une carte) : ${motifDuRefus(e)}`));
      }
      ok++;
    } catch (e) {
      echecs.push(`${j.titre} : ${motifDuRefus(e)}`);
    }
  }
  await rechargerJeuxDuPc();
  toast(`✅ ${ok} jeu(x) en cours de téléchargement.`);
  if (echecs.length) await informer('⚠ Jeux non remis', echecs.join('\n'));
}

/** Une partie restaurée en double : dite UNE fois (le cœur ne la redit plus), avec l'endroit de la copie ; la personne
 * peut la mettre à la place de la sienne (✅ Seb 27/10 : sa partie actuelle d'abord mise à l'abri). Au-delà de 3, un
 * choix d'abord : toutes, une par une, ou garder les deux versions. */
export async function proposerRemplacements(conflits: ConflitRestaure[]): Promise<void> {
  // (Appelée aussi depuis ⚙ Options ▸ Sauvegarde : une proposition fermée sans répondre se retrouve là.)
  if (!conflits.length) return;
  let une = true;
  if (conflits.length > 3) {
    const c = await choisir<'toutes' | 'une' | 'garder'>(
      `💾 ${conflits.length} parties restaurées en double`,
      [
        { valeur: 'garder', libelle: '📂 Garder les deux versions', detail: 'tes parties restent ; les restaurées attendent à part' },
        { valeur: 'une', libelle: '🔎 Choisir une par une' },
        { valeur: 'toutes', libelle: '♻ Les remplacer toutes', detail: 'tes parties actuelles mises à l’abri d’abord' },
      ],
      'Ces parties existaient déjà sur ce PC avec un contenu différent. Rien n’a été remplacé.',
    );
    if (!c || c === 'garder') {
      await informer('📂 Les deux versions sont gardées', `Tes parties restent telles quelles. Les ${conflits.length} parties restaurées attendent à part, dans :\n${dossierDesCopies(conflits[0].copie)}`);
      return;
    }
    une = c === 'une';
  }
  const abris: string[] = [];
  const echecs: string[] = [];
  for (const c of conflits) {
    if (une) {
      const oui = await confirmer('💾 Une partie existe en deux versions', {
        message: `${c.etiquette}\n\nTa partie actuelle est gardée. La partie restaurée est rangée à part :\n${c.copie}\n\nRemplacer ta partie par la partie restaurée ? Ta partie actuelle sera d’abord mise à l’abri.`,
        libelleValider: '♻ Remplacer ma partie',
      });
      if (!oui) continue;
    }
    try {
      abris.push(await api.restaurationRemplacer(c.cle));
    } catch (e) {
      echecs.push(`${c.etiquette} : ${motifDuRefus(e)}`);
    }
  }
  // UN bilan qui reste affiché (un chemin ne disparaît jamais avant d'avoir été lu).
  if (!abris.length && !echecs.length) return;
  const lignes = [
    abris.length ? `✅ ${abris.length} partie(s) restaurée(s) remise(s) en place. Les anciennes sont à l’abri dans :\n${dossierDesAbris(abris[0])}` : '',
    echecs.length ? `⚠ Pas remplacée(s) (rien n’a changé pour elle·s) :\n${echecs.slice(0, 8).join('\n')}${echecs.length > 8 ? `\n… et ${echecs.length - 8} autre(s)` : ''}` : '',
  ];
  await informer('💾 Parties en double', lignes.filter(Boolean).join('\n\n'));
}
