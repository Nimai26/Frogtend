// Les jeux du PC et leurs téléchargements : état, progression, débit. Mis à jour par les nouvelles du cœur.
// Mettre un jeu dans sa ludothèque : version, emplacement, récapitulatif chiffré, accord, puis file.

import { listen } from '@tauri-apps/api/event';
import { isTauri } from '@tauri-apps/api/core';
import {
  api,
  duree,
  estErreurCoeur,
  taille,
  type EtatJeuPc,
  type EvenementTelechargement,
  type Fiche,
  type JeuPc,
  type Version,
} from '$lib/api';
import { choisir, cocher, confirmer, informer, toast } from '$lib/dialogues/fenetres.svelte';
import { nomEnDouble, ordreDesCases, plusieursPossibles } from './versions';
import { motifDuRefus } from '$lib/dialogues/messages';
import { ludo, rechargerListe, rechargerPlateformes } from './ludotheque.svelte';
import { proposerDossierJeux } from './dossierJeux';
import { nomAffiche } from './ludotheque.svelte';

/** Une version AJOUTÉE à un jeu du PC qui arrive (son état et son avancement à elle). */
export interface VersionEnRoute {
  version: number;
  etat: EtatJeuPc;
  message: string | null;
  recus: number;
  total: number;
  debit: number;
}

export const tele = $state<{
  /** Les jeux du PC visibles pour ce profil, par id. */
  jeux: Record<number, JeuPc & { debit?: number }>;
  /** Les jeux importés reliés à une fiche, par numéro de fiche (contrat 1.21). */
  parFiche: Record<number, JeuPc & { debit?: number }>;
  /** Les versions ajoutées en route, par jeu puis par version (événements de la file). */
  versions: Record<number, Record<number, VersionEnRoute>>;
}>({ jeux: {}, parFiche: {}, versions: {} });

/** Le jeu du PC d'une ligne de la ludothèque : par son numéro, sinon le jeu importé relié à cette fiche (✅ Seb
 * 20/10 : celui de ce profil d'abord, sinon le plus ancien). */
export function jeuSurPc(id: number): (JeuPc & { debit?: number }) | undefined {
  return tele.jeux[id] ?? tele.parFiche[id];
}

/** Les jeux importés reliés, rangés par fiche (celui de ce profil d'abord, sinon le plus ancien). */
export function rangerParFiche(liste: JeuPc[]): Record<number, JeuPc> {
  const r: Record<number, JeuPc> = {};
  const relies = liste.filter((j) => j.id < 0 && j.fiche != null && j.fiche > 0);
  relies.sort((a, b) => Number(!!a.d_un_autre) - Number(!!b.d_un_autre) || a.ajoute_le.localeCompare(b.ajoute_le));
  for (const j of relies) if (!(j.fiche! in r)) r[j.fiche!] = j;
  return r;
}

/** Le nombre de téléchargements qui ne sont pas finis (pour la pastille de la barre du haut). */
export function nombreEnCours(): number {
  const versions = Object.values(tele.versions).flatMap((l) => Object.values(l)).filter((v) => v.etat === 'attente' || v.etat === 'en_cours').length;
  return Object.values(tele.jeux).filter((j) => j.etat !== 'telecharge').length + versions;
}

export async function rechargerJeuxDuPc() {
  try {
    const liste = await api.jeuxDuPc();
    tele.jeux = Object.fromEntries(liste.map((j) => [j.id, j]));
    tele.parFiche = rangerParFiche(liste);
  } catch {
    tele.jeux = {};
    tele.parFiche = {};
  }
}

let ecoute: (() => void) | null = null;
let ecouteArrives: (() => void) | null = null;

/** Le démarrage de l'écoute en cours (deux appels rapprochés n'écoutent jamais deux fois) et la « génération » du
 * suivi (un `arreterSuivi` passé pendant ce démarrage : les écoutes reçues sont aussitôt rendues). */
let demarrage: Promise<void> | null = null;
let generation = 0;

/** Écoute les nouvelles de la file (une seule fois), PUIS relit les jeux du PC : un avis envoyé par le cœur pendant
 * ce temps (le rattrapage des jeux prêts à l'ouverture du profil) n'est jamais perdu. */
export function suivreTelechargements(): Promise<void> {
  if (!isTauri()) return rechargerJeuxDuPc();
  if (ecoute) return rechargerJeuxDuPc();
  if (demarrage) return demarrage.then(() => rechargerJeuxDuPc());
  const g = generation;
  demarrage = (async () => {
    // Le rattrapage à l'ouverture du profil a rendu prêts des jeux déjà arrivés : la ludothèque les montre jouables.
    const a = await listen<number>('jeux-pc-changes', () => void rechargerJeuxDuPc());
    const b = await listen<EvenementTelechargement>('telechargement', surEvenement);
    if (g !== generation) {
      a();
      b();
      return;
    }
    ecouteArrives = a;
    ecoute = b;
    await rechargerJeuxDuPc();
  })().finally(() => (demarrage = null));
  return demarrage;
}

async function surEvenement(e: { payload: EvenementTelechargement }) {
  {
    const ev = e.payload;
    const j = tele.jeux[ev.jeu];
    // Une version ajoutée : SON état et SON avancement (le jeu, lui, reste « téléchargé » et jouable).
    if (ev.sorte === 'progres_version' || ev.sorte === 'version') {
      const l = (tele.versions[ev.jeu] ??= {});
      const v = (l[ev.version] ??= { version: ev.version, etat: 'en_cours', message: null, recus: 0, total: 0, debit: 0 });
      if (ev.sorte === 'progres_version') {
        Object.assign(v, { recus: ev.recus, total: ev.total, debit: ev.debit });
        retenirDebit(ev.debit);
        return;
      }
      Object.assign(v, { etat: ev.etat, message: ev.message });
      const nom = `une autre version de « ${j?.titre ?? 'ce jeu'} »`;
      if (ev.etat === 'telecharge') {
        toast(`✅ ${nom.charAt(0).toUpperCase()}${nom.slice(1)} est arrivée : choisis-la avec « Jouer avec… ».`);
        await rechargerJeuxDuPc();
      } else if (ev.etat === 'erreur') {
        toast(`⚠ ${nom.charAt(0).toUpperCase()}${nom.slice(1)} n’est pas arrivée : ${ev.message ?? 'motif inconnu'}. Le jeu reste jouable.`, 'erreur');
      }
      return;
    }
    if (ev.sorte === 'progres') {
      if (j) Object.assign(j, { recus: ev.recus, total: ev.total, debit: ev.debit });
      retenirDebit(ev.debit);
      return;
    }
    if (j) Object.assign(j, { etat: ev.etat, message: ev.message });
    if (ev.etat === 'telecharge') {
      await rechargerJeuxDuPc();
      // Une ROM ou une image disque est déjà prête (le cœur l'a installée : rien n'était à faire) — jamais la question
      // de l'émulateur ici : le premier « ▶ Jouer » la pose.
      const pret = !!tele.jeux[ev.jeu]?.installation;
      toast(pret ? `✅ « ${j?.titre ?? 'Le jeu'} » est arrivé : ▶ Jouer.` : `✅ « ${j?.titre ?? 'Le jeu'} » est téléchargé.`);
    } else if (ev.etat === 'erreur') {
      toast(`Échec du téléchargement de « ${j?.titre ?? 'ce jeu'} » : ${ev.message ?? 'motif inconnu'}`, 'erreur');
    }
  }
}

export function arreterSuivi() {
  generation++;
  ecoute?.();
  ecoute = null;
  ecouteArrives?.();
  ecouteArrives = null;
  tele.jeux = {};
  tele.versions = {};
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
export function debitConnu(): number | null {
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
export async function mettreDansLaLudotheque(id: number, versionVoulue?: number): Promise<boolean> {
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

  // 1. La version (la plus récente en premier) ; une carte e-Reader précise : celle-là. Des ROM ou images disque :
  // plusieurs à la fois, cochées (✅ Seb 28/10 : choisir sa version au lancement ; la plus récente cochée d'office,
  // elle est lancée par défaut).
  const options = versions.map((v, i) => ({
    valeur: v,
    libelle: `${nature(v.qualite).libelle} — ${v.nom}${i === 0 ? ' (la plus récente)' : ''}`,
    detail: taille(tailleVersion(v)),
  }));
  let choisies: Version[] | null;
  let avis: import('./versions').AvisVersion[] = [];
  if (versionVoulue !== undefined) {
    const v = versions.find((x) => x.telechargement_id === versionVoulue);
    choisies = v ? [v] : null;
  } else if (versions.length === 1) {
    choisies = [versions[0]];
  } else if (plusieursPossibles((avis = await api.versionsAvis(fiche).catch(() => [])), fiche.plateforme)) {
    // La région préférée d'abord (FR, EU, US/EN), cochée d'office : la plus haute cochée est ⭐ celle d'habitude.
    const ordre = ordreDesCases(versions, avis);
    const a = avis;
    choisies = await cocher(
      `📦 Quelle(s) version(s) de « ${fiche.titre} » ?`,
      ordre.map((v) => ({ valeur: v, libelle: `${nature(v.qualite).libelle} — ${v.nom}`, detail: taille(tailleVersion(v)) })),
      {
        message: 'Coche celles que tu veux garder : tu choisiras laquelle lancer au moment de jouer. ⭐ Celle d’habitude : la plus haute cochée.',
        coches: ordre.map((_, i) => i === 0),
        resume: (l) => (l.length ? `${l.length} version(s) — ${taille(l.reduce((s, v) => s + tailleVersion(v), 0))}` : 'Coche au moins une version.'),
        refus: (l) => {
          const n = nomEnDouble(l.map((v) => v.telechargement_id), a);
          return n ? `Deux versions cochées ont un fichier du même nom (« ${n} ») : n’en garde qu’une.` : null;
        },
        libelleValider: 'Continuer',
      },
    );
  } else {
    const v = await choisir(`📦 Quelle version de « ${fiche.titre} » ?`, options, 'Chaque version a ses propres fichiers et ses propres notes.');
    choisies = v ? [v] : null;
  }
  if (!choisies?.length) return false;
  const version = choisies[0];
  const autres = choisies.slice(1);
  const total = choisies.reduce((s, v) => s + tailleVersion(v), 0);
  const plateforme = fiche.plateforme ?? 'Autres';

  // 2. L'emplacement.
  let proposes = await api.proposerEmplacements(plateforme, total).catch(() => []);
  // Rien de réglé (PC neuf), ou aucun dossier réglé n'existe sur ce PC (formaté, disque changé) : un dossier proposé
  // ici même, jamais « va dans les Options ».
  if (proposes.length === 0 || proposes.every((p) => p.libre === null)) {
    const raison = proposes.length
      ? `Le dossier de tes jeux (${proposes[0].chemin}) n’existe pas sur ce PC.`
      : `C’est ton premier jeu sur ce PC : Frogtend ne sait pas encore où ranger les jeux ${nomAffiche(plateforme)}.`;
    if (!(await proposerDossierJeux(raison, total))) return false;
    proposes = await api.proposerEmplacements(plateforme, total).catch(() => []);
    if (proposes.length === 0) {
      toast('Le dossier choisi n’est pas utilisable pour ce jeu : réessaie, ou choisis-en un autre.', 'alerte');
      return false;
    }
  }
  const possibles = proposes.filter((p) => p.assez);
  if (possibles.length === 0) {
    const details = proposes
      .map((p) => `• ${p.chemin} : ${p.libre === null ? 'introuvable (disque débranché ?)' : `${taille(p.libre)} libres`}`)
      .join('\n');
    await informer(
      '💾 Pas assez de place',
      `« ${fiche.titre} » demande ${taille(total)} (plus 1 Go de marge).\n\n${details}\n\nAjoute un autre emplacement dans « ⚙ Options », ou libère de la place.`,
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
      autres.length
        ? `Versions : ${choisies.map((v) => v.nom).join(' ; ')}\n⭐ Lancée d’habitude : ${version.nom}`
        : `Version : ${nature(version.qualite).libelle} — ${version.nom}`,
      `Taille : ${taille(total)} (${choisies.reduce((s, v) => s + v.fichiers.length, 0)} fichier(s)), plus la fiche, la jaquette et les documents`,
      `Emplacement : ${emplacement.chemin}`,
      `Place restante ensuite : ${taille(Math.max(0, (emplacement.libre ?? 0) - total))}`,
      debit ? `Durée estimée : ${duree(total / debit)} au débit mesuré sur ce PC` : 'Durée : estimée dès le début du téléchargement',
    ].join('\n'),
    libelleValider: '⬇ Télécharger',
  });
  if (!oui) return false;

  try {
    await api.ajouterJeu(id, version.telechargement_id, emplacement.chemin, autres.map((v) => v.telechargement_id));
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

/** Ajoute UNE autre version (région) à un jeu déjà sur le PC (✅ Seb 28/10) : dans SON dossier, après un
 * récapitulatif chiffré ; le jeu reste jouable pendant qu'elle arrive. */
export async function ajouterUneVersion(id: number, v: Version, titre: string): Promise<boolean> {
  const j = tele.jeux[id];
  if (!j) return false;
  const t = tailleVersion(v);
  const debit = debitConnu();
  const oui = await confirmer(`➕ Ajouter la version « ${v.nom} » ?`, {
    message: [
      `Jeu : ${titre} (déjà sur ce PC, il reste jouable pendant l’arrivée)`,
      `Taille : ${taille(t)} (${v.fichiers.length} fichier(s)), dans le dossier du jeu`,
      debit ? `Durée estimée : ${duree(t / debit)} au débit mesuré sur ce PC` : 'Durée : estimée dès le début du téléchargement',
      'Tu la choisiras au moment de jouer (« Jouer avec… »).',
    ].join('\n'),
    libelleValider: '⬇ Télécharger',
  });
  if (!oui) return false;
  try {
    // Le dossier du jeu : le cœur range la version à côté des autres (l'emplacement n'y change rien).
    await api.ajouterJeu(id, v.telechargement_id, j.dossier);
  } catch (e) {
    toast(`Refusé : ${motifDuRefus(e)}`, 'erreur');
    return false;
  }
  toast(`✅ La version « ${v.nom} » arrive : le jeu reste jouable en attendant.`);
  await rechargerJeuxDuPc();
  return true;
}

/** Retire (ou annule) UNE version ajoutée : ses fichiers seulement ; jamais la version d'origine ni les parties. */
export async function retirerUneVersion(id: number, version: number, nom: string, enRoute: boolean): Promise<boolean> {
  const oui = await confirmer(enRoute ? `✖ Annuler l’arrivée de « ${nom} » ?` : `🗑 Retirer la version « ${nom} » ?`, {
    message: enRoute
      ? 'Ce qui est déjà reçu de cette version est effacé ; le jeu et ses autres versions restent.'
      : 'Ses fichiers sont retirés de ce PC ; le jeu, ses autres versions et tes parties restent. Tu pourras la remettre plus tard.',
    libelleValider: enRoute ? '✖ Annuler l’arrivée' : '🗑 Retirer',
    danger: true,
  });
  if (!oui) return false;
  try {
    await api.retirerVersion(id, version);
    await rechargerJeuxDuPc();
    toast(enRoute ? '✖ Arrivée annulée.' : '🗑 Version retirée.');
    return true;
  } catch (e) {
    toast(`Impossible : ${motifDuRefus(e)}`, 'erreur');
    return false;
  }
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
