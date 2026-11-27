// Lot 7 (✅ Seb 17/11) : envoyer à Firehouse les mises à jour et DLC Switch qu'il n'a pas, pour qu'il les serve ensuite à
// toute la famille. Détecté à l'ouverture du panneau 📦 ; ACCORD chiffré de la personne qui utilise Frogtend (ou « tout
// seul » si elle l'a choisi) ; envoi en arrière-plan, un fichier à la fois, en pause pendant une partie.

import { environDuree, taille } from '$lib/api';

export interface AEnvoyer {
  nom: string;
  role: 'maj' | 'dlc' | string;
  taille: number;
}

export interface EnvoiVu {
  nom: string;
  role: 'maj' | 'dlc' | string;
  taille: number;
  recu: number;
  /** `attente`, `envoi`, `verification` (Firehouse vérifie), `range`, `deja_la`, `refuse`, `arrete`. */
  etat: string;
  motif?: string | null;
}

export interface ProgresEnvoi {
  jeu: number;
  nom: string;
  /** `empreinte` (lecture du fichier), `envoi`, `pause` (une partie en cours), `verification`. */
  etape: string;
  faits: number;
  total: number;
  /** Octets par seconde, s'il est mesuré. */
  debit?: number | null;
  /** Secondes restantes estimées. */
  restant?: number | null;
}

export interface SuiviEnvois {
  envois: EnvoiVu[];
  en_cours: ProgresEnvoi | null;
  /** Pourquoi les envois se sont arrêtés (jeton refusé, Firehouse injoignable…), à dire. */
  arret: string | null;
}

export interface ExamenEnvois {
  /** `rien`, `simule`, `interdit` (compte sans droit d'envoi : rien n'est proposé), `injoignable`, `ok`. */
  etat: 'rien' | 'simule' | 'interdit' | 'injoignable' | 'ok' | string;
  motif: string | null;
  a_envoyer: AEnvoyer[];
  total: number;
  /** Secondes estimées au débit montant mesuré ; `null` : pas encore mesuré. */
  duree: number | null;
  plafonne: boolean;
  ecartes: { nom: string; raison: string }[];
  deja_la: number;
  suivi: SuiviEnvois;
}

const FINIS = new Set(['range', 'deja_la', 'refuse', 'arrete']);

/** Un envoi est-il fini (rangé, déjà là, refusé, arrêté) ? */
export function envoiFini(e: EnvoiVu): boolean {
  return FINIS.has(e.etat);
}

/** Reste-t-il quelque chose en route (à relire de temps en temps) ? Non quand le travail s'est arrêté (`arret`, sans
 * envoi en cours) : rien ne bougera avant la prochaine ouverture. */
export function envoisEnRoute(s: SuiviEnvois | null | undefined): boolean {
  if (!s || (!s.en_cours && s.arret)) return false;
  return !!s.en_cours || s.envois.some((e) => !envoiFini(e));
}

/** Y a-t-il un envoi de ce jeu à arrêter (pas fini) ? */
export function envoiArretable(s: SuiviEnvois | null | undefined): boolean {
  return !!s && (!!s.en_cours || s.envois.some((e) => !envoiFini(e)));
}

/** Un contenu trouvé qui ne part pas : « « nom » pas envoyé : raison » (la raison est déjà dite simplement). */
export function ligneEcarte(e: { nom: string; raison: string }): string {
  return `« ${e.nom} » pas envoyé : ${e.raison}`;
}

/** Ce qu'annonce le lecteur d'écran : l'ÉTAPE seulement (jamais le pourcentage qui change toutes les 2 s). */
export function etapeAnnoncee(p: ProgresEnvoi | null | undefined): string {
  if (!p) return '';
  switch (p.etape) {
    case 'pause':
      return 'Envoi à Firehouse en pause pendant la partie.';
    case 'empreinte':
      return `Préparation de « ${p.nom} » pour Firehouse.`;
    case 'verification':
      return `Firehouse vérifie « ${p.nom} ».`;
    default:
      return `Envoi de « ${p.nom} » à Firehouse en cours.`;
  }
}

const pluriel = (n: number) => (n > 1 ? 's' : '');

/** « environ 3 h 20 », ou ce qu'on sait quand le débit n'est pas encore mesuré. */
export function dureeDite(secondes: number | null | undefined): string {
  return secondes == null ? 'durée inconnue : elle sera mesurée au premier envoi' : environDuree(secondes);
}

/** La ligne du panneau : « 📤 Firehouse n'a pas 3 contenus (1,2 Go) » ; `null` : rien à proposer. */
export function ligneProposition(e: ExamenEnvois | null | undefined): string | null {
  if (!e || e.etat !== 'ok' || !e.a_envoyer.length) return null;
  const n = e.a_envoyer.length;
  return `📤 Firehouse n’a pas ${n} contenu${pluriel(n)} de ce jeu (${taille(e.total)})`;
}

/** Le titre de l'accord, chiffré : combien, combien de Go, combien de temps. */
export function titreAccord(e: ExamenEnvois): string {
  const n = e.a_envoyer.length;
  return `Firehouse n’a pas ${n} contenu${pluriel(n)} (${taille(e.total)}, ${dureeDite(e.duree)}) : les lui envoyer ?`;
}

/** Le texte de l'accord (une ligne par paragraphe). */
export function messageAccord(e: ExamenEnvois): string {
  const liste = e.a_envoyer.slice(0, 8).map((c) => `• ${c.nom} (${c.role === 'maj' ? 'mise à jour' : 'DLC'}, ${taille(c.taille)})`);
  if (e.a_envoyer.length > 8) liste.push(`… et ${e.a_envoyer.length - 8} autre(s).`);
  return [
    ...liste,
    'Firehouse les vérifie, puis les donne à toute la famille : personne n’aura à les chercher.',
    'Ils partent en arrière-plan, un à la fois, et se mettent en pause pendant tes parties.',
    e.plafonne
      ? 'Ils n’utilisent que la moitié de ta connexion (réglable dans ⚙ Options ▸ 💾 Sauvegarde).'
      : 'Ils utilisent toute ta connexion (réglable dans ⚙ Options ▸ 💾 Sauvegarde).',
    'Rien d’autre ne part : ni le jeu, ni tes clés, ni tes parties.',
  ].join('\n');
}

/** La ligne de l'envoi en cours. */
export function ligneProgres(p: ProgresEnvoi): string {
  const pourcent = p.total > 0 ? Math.floor((p.faits * 100) / p.total) : 0;
  switch (p.etape) {
    case 'pause':
      return `⏸ En pause pendant la partie : l’envoi de « ${p.nom} » reprendra ensuite.`;
    case 'empreinte':
      return `📤 Préparation de « ${p.nom} » (lecture du fichier) : ${pourcent} %`;
    case 'verification':
      return `🔎 Firehouse vérifie « ${p.nom} »…`;
    default: {
      const morceaux = [`📤 Envoi de « ${p.nom} » à Firehouse : ${pourcent} %`];
      if (p.debit) morceaux.push(`${taille(p.debit)}/s`);
      if (p.restant != null) morceaux.push(`reste ${dureeDite(p.restant)}`);
      return morceaux.join(' · ');
    }
  }
}

/** La ligne d'un envoi de la file (après, ou en attente). `arrete` : le travail s'est arrêté (Firehouse injoignable…),
 * un envoi pas fini reprendra à la prochaine ouverture. */
export function ligneEnvoi(e: EnvoiVu, arrete = false): string {
  const motif = e.motif ? ` : ${e.motif}` : '';
  if (arrete && !envoiFini(e)) return `⏳ Envoi de « ${e.nom} » en attente : il reprendra à la prochaine ouverture.`;
  switch (e.etat) {
    case 'range':
      return `✅ Firehouse l’a rangé : « ${e.nom} » (toute la famille peut l’avoir)`;
    case 'deja_la':
      return `✅ Firehouse l’avait déjà : « ${e.nom} »${motif}`;
    case 'refuse':
      return `⚠ Refusé par Firehouse : « ${e.nom} »${motif}`;
    case 'arrete':
      return `⚠ Envoi arrêté : « ${e.nom} »${motif}`;
    case 'verification':
      return `🔎 Firehouse vérifie « ${e.nom} »…`;
    case 'envoi':
      return `📤 Envoi de « ${e.nom} » commencé (${e.taille ? Math.floor((e.recu * 100) / e.taille) : 0} %) : il reprendra où il en était.`;
    default:
      return `⏳ En attente d’envoi : « ${e.nom} »${motif}`;
  }
}
