// Les jeux du PC et leurs téléchargements : état, progression, débit. Mis à jour par les nouvelles du cœur.
// Mettre un jeu dans sa ludothèque : version, emplacement, récapitulatif chiffré, accord, puis file.

import { listen } from '@tauri-apps/api/event';
import { isTauri } from '@tauri-apps/api/core';
import {
  api,
  duree,
  estErreurCoeur,
  taille,
  type EvenementTelechargement,
  type Fiche,
  type JeuPc,
  type Version,
} from '$lib/api';
import { choisir, confirmer, informer, toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { ludo, rechargerListe, rechargerPlateformes } from './ludotheque.svelte';

export const tele = $state<{
  /** Les jeux du PC visibles pour ce profil, par id. */
  jeux: Record<number, JeuPc & { debit?: number }>;
}>({ jeux: {} });

/** Le nombre de téléchargements qui ne sont pas finis (pour la pastille de la barre du haut). */
export function nombreEnCours(): number {
  return Object.values(tele.jeux).filter((j) => j.etat !== 'telecharge').length;
}

export async function rechargerJeuxDuPc() {
  try {
    const liste = await api.jeuxDuPc();
    tele.jeux = Object.fromEntries(liste.map((j) => [j.id, j]));
  } catch {
    tele.jeux = {};
  }
}

let ecoute: (() => void) | null = null;

/** Écoute les nouvelles de la file (une seule fois). */
export async function suivreTelechargements() {
  await rechargerJeuxDuPc();
  if (ecoute || !isTauri()) return;
  ecoute = await listen<EvenementTelechargement>('telechargement', async (e) => {
    const ev = e.payload;
    const j = tele.jeux[ev.jeu];
    if (ev.sorte === 'progres') {
      if (j) Object.assign(j, { recus: ev.recus, total: ev.total, debit: ev.debit });
      retenirDebit(ev.debit);
      return;
    }
    if (j) Object.assign(j, { etat: ev.etat, message: ev.message });
    if (ev.etat === 'telecharge') {
      toast(`✅ « ${j?.titre ?? 'Le jeu'} » est téléchargé.`);
      await rechargerJeuxDuPc();
    } else if (ev.etat === 'erreur') {
      toast(`Échec du téléchargement de « ${j?.titre ?? 'ce jeu'} » : ${ev.message ?? 'motif inconnu'}`, 'erreur');
    }
  });
}

export function arreterSuivi() {
  ecoute?.();
  ecoute = null;
  tele.jeux = {};
}

/** Libellé lisible de la nature d'une version. */
export function nature(q: string): { libelle: string; ton: string } {
  const t = q.toLowerCase();
  if (t.includes('prêt à jouer')) return { libelle: '✅ Prêt à jouer', ton: 'ok' };
  if (t.includes('rom')) return { libelle: '💾 ROM', ton: '' };
  if (t.includes('image disque')) return { libelle: '💿 Image disque', ton: '' };
  if (t.includes('repack')) return { libelle: '📦 Repack (installeur)', ton: 'warn' };
  if (t.includes('installeur')) return { libelle: '🧰 Installeur d’origine', ton: 'warn' };
  return { libelle: `❔ ${q || 'Nature non dite'}`, ton: '' };
}

const tailleVersion = (v: Version) => v.fichiers.reduce((s, f) => s + f.taille, 0);

/** Débit moyen mesuré sur ce PC (pour estimer la durée d'un téléchargement), gardé dans le navigateur. */
function debitConnu(): number | null {
  try {
    const d = Number(localStorage.getItem('frogtend.debit'));
    return d > 0 ? d : null;
  } catch {
    return null;
  }
}
export function retenirDebit(debit: number) {
  if (debit <= 0) return;
  try {
    const ancien = debitConnu();
    localStorage.setItem('frogtend.debit', String(Math.round(ancien ? ancien * 0.7 + debit * 0.3 : debit)));
  } catch {
    // Sans stockage : pas d'estimation, rien de grave.
  }
}

/**
 * Mettre un jeu dans sa ludothèque : choisir la version, puis l'emplacement, voir le récapitulatif chiffré,
 * donner son accord. Rend vrai si le jeu a été ajouté.
 */
export async function mettreDansLaLudotheque(id: number): Promise<boolean> {
  let fiche: Fiche;
  try {
    fiche = (await api.fiche(id)).fiche;
  } catch (e) {
    toast(`Impossible de lire la fiche : ${motifDuRefus(e)}`, 'erreur');
    return false;
  }
  const versions = [...(fiche.versions ?? [])].sort((a, b) => (b.range_le ?? '').localeCompare(a.range_le ?? ''));
  if (versions.length === 0) {
    await informer(
      `📭 « ${fiche.titre} » n’a pas encore de version`,
      'Firehouse ne l’a pas encore téléchargé. Tu pourras le mettre dans ta ludothèque quand une version sera rangée.',
    );
    return false;
  }

  // 1. La version (la plus récente en premier).
  const version =
    versions.length === 1
      ? versions[0]
      : await choisir(
          `📦 Quelle version de « ${fiche.titre} » ?`,
          versions.map((v, i) => ({
            valeur: v,
            libelle: `${nature(v.qualite).libelle} — ${v.nom}${i === 0 ? ' (la plus récente)' : ''}`,
            detail: taille(tailleVersion(v)),
          })),
          'Chaque version a ses propres fichiers et ses propres notes.',
        );
  if (!version) return false;
  const total = tailleVersion(version);
  const plateforme = fiche.plateforme ?? 'Autres';

  // 2. L'emplacement.
  const proposes = await api.proposerEmplacements(plateforme, total).catch(() => []);
  if (proposes.length === 0) {
    await informer(
      '📁 Aucun emplacement réglé',
      `Frogtend ne sait pas encore où ranger les jeux ${plateforme}.\n\nRègle-le dans « Réglages » ▸ « Emplacements des jeux » (un dossier par défaut suffit).`,
    );
    return false;
  }
  const possibles = proposes.filter((p) => p.assez);
  if (possibles.length === 0) {
    const details = proposes
      .map((p) => `• ${p.chemin} : ${p.libre === null ? 'introuvable (disque débranché ?)' : `${taille(p.libre)} libres`}`)
      .join('\n');
    await informer(
      '💾 Pas assez de place',
      `« ${fiche.titre} » demande ${taille(total)} (plus 1 Go de marge).\n\n${details}\n\nAjoute un autre emplacement dans « Réglages », ou libère de la place.`,
    );
    return false;
  }
  const emplacement =
    possibles.length === 1
      ? possibles[0]
      : await choisir(
          '📁 Où ranger le jeu ?',
          possibles.map((p) => ({ valeur: p, libelle: p.chemin, detail: `${taille(p.libre ?? 0)} libres` })),
          proposes.length > possibles.length ? 'Les emplacements sans assez de place ne sont pas proposés.' : undefined,
        );
  if (!emplacement) return false;

  // 3. Le récapitulatif, et l'accord.
  const debit = debitConnu();
  const oui = await confirmer(`⬇ Mettre « ${fiche.titre} » dans ta ludothèque ?`, {
    message: [
      `Version : ${nature(version.qualite).libelle} — ${version.nom}`,
      `Taille : ${taille(total)} (${version.fichiers.length} fichier(s)), plus la fiche, la jaquette et les documents`,
      `Emplacement : ${emplacement.chemin}`,
      `Place restante ensuite : ${taille(Math.max(0, (emplacement.libre ?? 0) - total))}`,
      debit ? `Durée estimée : ${duree(total / debit)} au débit mesuré sur ce PC` : 'Durée : estimée dès le début du téléchargement',
    ].join('\n'),
    libelleValider: '⬇ Télécharger',
  });
  if (!oui) return false;

  try {
    await api.ajouterJeu(id, version.telechargement_id, emplacement.chemin);
  } catch (e) {
    if (estErreurCoeur(e) && e.sorte === 'reseau') {
      toast(`⚠ Impossible sans connexion : ${e.motif}`, 'alerte');
    } else {
      toast(`Refusé : ${motifDuRefus(e)}`, 'erreur');
    }
    return false;
  }
  toast(`✅ « ${fiche.titre} » arrive dans ta ludothèque : le téléchargement commence.`);
  await rechargerJeuxDuPc();
  if (ludo.espace === 'ludotheque') {
    await rechargerPlateformes();
    await rechargerListe();
  }
  return true;
}

export async function mettreEnPause(id: number) {
  try {
    await api.pause(id);
    await rechargerJeuxDuPc();
  } catch (e) {
    toast(`Refusé : ${motifDuRefus(e)}`, 'erreur');
  }
}

export async function reprendre(id: number) {
  try {
    await api.reprendre(id);
    await rechargerJeuxDuPc();
  } catch (e) {
    toast(`Refusé : ${motifDuRefus(e)}`, 'erreur');
  }
}

export async function annuler(id: number) {
  const j = tele.jeux[id];
  const oui = await confirmer(`🗑 Annuler le téléchargement de « ${j?.titre ?? 'ce jeu'} » ?`, {
    message: `Les fichiers déjà reçus (${taille(j?.recus ?? 0)}) seront effacés, et le jeu sortira de ta ludothèque. Rien d’autre n’est touché.`,
    danger: true,
    libelleValider: 'Annuler le téléchargement',
  });
  if (!oui) return;
  try {
    await api.annuler(id);
    toast('Téléchargement annulé.');
    await rechargerJeuxDuPc();
    if (ludo.espace === 'ludotheque') {
      await rechargerPlateformes();
      await rechargerListe();
    }
  } catch (e) {
    toast(`Refusé : ${motifDuRefus(e)}`, 'erreur');
  }
}
