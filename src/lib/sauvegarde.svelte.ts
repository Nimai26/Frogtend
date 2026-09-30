// La sauvegarde du profil ouvert chez Firehouse : à la demande, après chaque partie, ou chaque jour (réglage du
// profil). Une sauvegarde automatique est discrète : elle ne dit rien si tout va bien, et signale un échec sans
// bloquer (elle sera refaite à la prochaine occasion).

import { listen } from '@tauri-apps/api/event';
import { isTauri } from '@tauri-apps/api/core';
import { api, taille } from '$lib/api';
import { choisir, confirmer, informer, toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { etat, reglerPc, remplacerReglagesProfil } from '$lib/etat.svelte';
import { depuis } from '$lib/ludotheque/ludotheque.svelte';
import { rechargerJeuxDuPc, tele } from '$lib/ludotheque/telechargements.svelte';

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

/** Sauvegarde maintenant. `discrete` : pas de message si tout va bien. */
export async function sauvegarder(discrete = false): Promise<boolean> {
  if (sauvegarde.enCours || etat.pc.firehouse.simule) return false;
  sauvegarde.enCours = true;
  try {
    const b = await api.sauvegarder();
    sauvegarde.derniere = b;
    if (!discrete) {
      toast(
        b.envoyes
          ? `✅ Sauvegardé : ${b.envoyes} fichier(s) envoyé(s), ${b.inchanges} déjà à jour (${b.fichiers} en tout).`
          : `✅ Sauvegarde à jour : ${b.fichiers} fichier(s) de parties, rien n’avait changé.`,
      );
    }
    return true;
  } catch (e) {
    const motif = motifDuRefus(e);
    // Firehouse sans la sauvegarde (avant le contrat 1.5) : une sauvegarde automatique se tait.
    if (discrete && motif.includes('ne propose pas encore')) return false;
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

/** Le nom tel que Firehouse le range (accents retirés, casse gardée), pour reconnaître la sauvegarde d'un profil. */
const nomRange = (s: string) => s.normalize('NFD').replace(/[\u0300-\u036f]/g, '').replace(/[^A-Za-z0-9 ._-]/g, '_').slice(0, 64);

/** À l'ouverture du profil : propose une restauration (profil jamais sauvegardé d'ici, sauvegarde à son nom), puis
 * sauvegarde quotidienne si le profil l'a choisi et que la dernière est ancienne. */
export async function auDemarrage() {
  await lireDerniereSauvegarde();
  if (!sauvegarde.derniere && !etat.pc.firehouse.simule && Object.keys(tele.jeux).length === 0) {
    const liste = await api.restaurationListe().catch(() => []);
    const a_moi = liste.filter((s) => s.profil === nomRange(etat.profilOuvert?.nom ?? ''));
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
  if (etat.pc.emplacements.defaut.length === 0 && Object.keys(etat.pc.emplacements.systemes).length === 0 && pcLu.emplacements) {
    const defaut = [];
    for (const d of pcLu.emplacements.defaut ?? []) if (await existe(d)) defaut.push(d);
    const systemes: Record<string, string[]> = {};
    for (const [s, l] of Object.entries(pcLu.emplacements.systemes ?? {})) {
      const ok = [];
      for (const d of l as string[]) if (await existe(d)) ok.push(d);
      if (ok.length) systemes[s] = ok;
    }
    await reglerPc('emplacements', { defaut, systemes });
  }
  if (!etat.pc.dossierEmulateurs && pcLu.dossierEmulateurs && (await existe(pcLu.dossierEmulateurs))) {
    await reglerPc('dossierEmulateurs', pcLu.dossierEmulateurs);
  }
  const emus = { ...etat.pc.emulateurs };
  for (const [s, e] of Object.entries(pcLu.emulateurs ?? {}) as [string, any][]) {
    const dossier = String(e.programme ?? '').replace(/[\\/][^\\/]+$/, '');
    if (!emus[s] && dossier && (await existe(dossier))) emus[s] = e;
  }
  await reglerPc('emulateurs', emus);

  toast(
    `✅ Restauré : réglages, ${prete.fichiers} fichier(s) de parties (${taille(prete.taille)}).` +
      (reposes.reposes ? ` ${reposes.reposes} partie(s) d’émulateur reposée(s).` : ''),
  );
  if (reposes.conflits.length) {
    await informer(
      '⚠ Parties gardées telles quelles',
      `Ces parties existaient déjà sur ce PC avec un contenu différent ; elles n’ont pas été remplacées :\n${reposes.conflits.slice(0, 12).join('\n')}${reposes.conflits.length > 12 ? `\n… et ${reposes.conflits.length - 12} autre(s)` : ''}`,
    );
  }

  // Les jeux : on propose de les remettre (annoncé et chiffré d'abord).
  await rechargerJeuxDuPc();
  const aRemettre = (prete.bibliotheque?.jeux ?? []).filter((j: any) => !tele.jeux[j.id]);
  if (!aRemettre.length) return;
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
      ok++;
    } catch (e) {
      echecs.push(`${j.titre} : ${motifDuRefus(e)}`);
    }
  }
  await rechargerJeuxDuPc();
  toast(`✅ ${ok} jeu(x) en cours de téléchargement.`);
  if (echecs.length) await informer('⚠ Jeux non remis', echecs.join('\n'));
}
