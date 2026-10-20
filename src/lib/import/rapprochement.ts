// Reconnaître les jeux importés (contrat 1.21, besoin 28 ; ✅ Seb 20/10) : les formes renvoyées par le cœur, et les
// mots de Firehouse dits simplement pour des gens qui veulent juste jouer.
import { duree, taille } from '$lib/api';

export interface Annonce {
  fichiers: number;
  a_lire: number;
  octets: number;
  repertoires: number;
  reseau: number;
  octets_reseau: number;
  connus: number;
  absents: number;
}

export interface Candidat {
  media_id: number;
  telechargement_id: number | null;
  titre?: string;
  annee?: number | null;
}

export interface VersionRapprochee {
  version: string;
  statut: 'exact' | 'probable' | 'inconnu' | 'non_envoye' | 'ecarte';
  media_id: number | null;
  telechargement_id: number | null;
  correspondance: string | null;
  motif: string | null;
  candidats: Candidat[];
  titre_fiche?: string | null;
  annee_fiche?: number | null;
}

export interface JeuRapproche {
  jeu: number;
  titre: string;
  plateforme: string;
  versions: VersionRapprochee[];
  decision: 'rattache' | 'a_voir' | 'inconnu';
  media_id: number | null;
  rattache_par?: 'auto' | 'personne' | null;
}

export interface Rapprochement {
  le: string;
  jeux: JeuRapproche[];
  non_envoyes: number;
}

/** Le statut d'une version, dit simplement. */
export function texteStatut(s: VersionRapprochee['statut']): string {
  switch (s) {
    case 'exact':
      return 'reconnu à coup sûr';
    case 'probable':
      return 'sans doute ce jeu';
    case 'non_envoye':
      return 'pas vérifié : trop de fichiers dans ce jeu';
    case 'ecarte':
      return 'tu as dit que ce n’était pas lui';
    default:
      return 'Firehouse ne connaît pas ce fichier';
  }
}

/** Le motif donné par Firehouse, dit simplement ; un motif inconnu est montré tel quel (jamais vide). */
export function texteMotif(m: string | null | undefined): string {
  if (!m) return '';
  const connus: Record<string, string> = {
    'autre version du romset MAME': 'ta version MAME n’est pas exactement celle de Firehouse ; le jeu est sans doute le même',
    'aucune correspondance': 'Firehouse ne connaît pas ce fichier',
    'pas envoyé (trop gros pour un appel)': 'pas vérifié : trop de fichiers dans ce jeu',
    'mode simulé : aucune fiche à rapprocher': 'mode simulé : Firehouse n’est pas interrogé',
  };
  return connus[m] ?? m;
}

/** Les fiches que Firehouse propose pour ce jeu (sans doublon), la plus probable d'abord. */
export function propositions(j: JeuRapproche): Candidat[] {
  const l: Candidat[] = [];
  for (const v of j.versions) {
    if (v.media_id !== null && !l.some((c) => c.media_id === v.media_id))
      l.push({ media_id: v.media_id, telechargement_id: v.telechargement_id, titre: v.titre_fiche ?? undefined, annee: v.annee_fiche ?? null });
    for (const c of v.candidats) if (!l.some((x) => x.media_id === c.media_id)) l.push(c);
  }
  return l;
}

/** Le nom d'une fiche proposée (« Zelda (1987) ») ; une fiche pas encore synchronisée le dit. */
export function nomFiche(c: Candidat): string {
  return c.titre ? `${c.titre}${c.annee ? ` (${c.annee})` : ''}` : `un jeu que ta ludothèque n’a pas encore reçu (n° ${c.media_id})`;
}

/** La phrase de l'annonce, avant de lancer (règle de Seb : chiffrer avant une opération de masse). */
export function texteAnnonce(a: Annonce): string {
  const l = [`${a.fichiers} fichier(s) de tes jeux importés.`];
  if (a.connus) l.push(`${a.connus} déjà lu(s) avant : rien à relire.`);
  if (a.a_lire) l.push(`${a.a_lire} fichier(s) lu(s) en entier (${taille(a.octets)}), sans rien modifier.`);
  if (a.repertoires) l.push(`${a.repertoires} archive(s) dont seule la liste du contenu est lue (rien n’est décompressé).`);
  if (a.absents) l.push(`${a.absents} fichier(s) introuvable(s) sur le disque : ignoré(s).`);
  const s = dureeEstimee(a, false);
  if (s >= 1) l.push(`Durée : environ ${duree(s)} (moins si tes fichiers ont déjà été lus récemment).`);
  l.push('Seuls les NOMS de fichiers et une signature calculée de chacun partent vers Firehouse, jamais l’endroit où ils sont rangés.');
  return l.join('\n');
}

/** Débits PRUDENTS mesurés chez Seb le 20/10 (disque local froid : 230 Mo en 11,5 s ; 14 191 listes de zip en 136 s). */
const OCTETS_PAR_SECONDE = 20 * 1024 ** 2;
const OCTETS_PAR_SECONDE_RESEAU = 5 * 1024 ** 2;
const LISTES_PAR_SECONDE = 100;

/** Une durée estimée en secondes (avec, ou non, la lecture entière du disque réseau). */
export function dureeEstimee(a: Annonce, reseau: boolean): number {
  return a.octets / OCTETS_PAR_SECONDE + a.repertoires / LISTES_PAR_SECONDE + (reseau ? a.octets_reseau / OCTETS_PAR_SECONDE_RESEAU : 0);
}

/** Une fiche dont on connaît le titre (sinon la personne ne peut pas juger). */
export function ficheNommee(c: Candidat): boolean {
  return !!c.titre;
}
