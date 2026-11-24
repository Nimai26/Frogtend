// Lot S5 (24/11) : les mots de la restauration des parties des jeux PC — fenêtre de conflit (deux colonnes, jamais un
// chemin ni une empreinte), bilan, choix du PC. Fonctions pures, testées.
import type { ApercuRestauration, BilanReprise, ConflitEmplacement } from '$lib/api';
import type { ColonneConflit } from '$lib/dialogues/fenetres.svelte';

/** En deçà de cet écart (secondes), on ne dit jamais laquelle est la plus récente (horloges des PC, envoi en cours). */
export const ECART_SUR_S = 120;

/** La plus récente des deux, seulement si c'est SÛR : les deux dates connues et un écart net. */
export function plusRecente(ici: number | null | undefined, firehouse: number | null | undefined): 'ici' | 'firehouse' | null {
  if (ici == null || firehouse == null || !Number.isFinite(ici) || !Number.isFinite(firehouse)) return null;
  if (Math.abs(ici - firehouse) < ECART_SUR_S) return null;
  return ici > firehouse ? 'ici' : 'firehouse';
}

/** Une date (secondes) en mots ; inconnue : dit tel quel. */
export function dateConflit(s: number | null | undefined): string {
  if (s == null || !Number.isFinite(s) || s <= 0) return 'date inconnue';
  return new Date(s * 1000).toLocaleString('fr-FR', { dateStyle: 'long', timeStyle: 'short' });
}

function fichiers(n: number): string {
  return `${n.toLocaleString('fr-FR')} fichier${n > 1 ? 's' : ''}`;
}

/** Les deux colonnes de la fenêtre de conflit. `taille` : le formateur de l'application. */
export function colonnesConflit(c: ConflitEmplacement, taille: (o: number) => string): { ici: ColonneConflit; firehouse: ColonneConflit } {
  const r = plusRecente(c.ici.date, c.firehouse.date);
  return {
    ici: {
      titre: '💻 Sur ce PC',
      lignes: [`Modifiée le ${dateConflit(c.ici.date)}`, `${taille(c.ici.octets)} · ${fichiers(c.ici.fichiers)}`],
      recente: r === 'ici',
    },
    firehouse: {
      titre: '🔥 Chez Firehouse',
      lignes: [`Sauvegardée le ${dateConflit(c.firehouse.date)}`, `${taille(c.firehouse.octets)} · ${fichiers(c.firehouse.fichiers)}`],
      recente: r === 'firehouse',
    },
  };
}

/** Le message de la fenêtre de conflit (le jeu, ce qui se passe pour chaque choix). */
export function messageConflit(c: ConflitEmplacement): string {
  const jeu = c.titre ? `« ${c.titre} »` : 'ce jeu';
  return [
    `Ta partie de ${jeu} sur ce PC est différente de celle gardée chez Firehouse.`,
    'Si tu reprends celle de Firehouse, celle de ce PC est d’abord mise à l’abri.',
    c.partiel ? '⚠ La copie de Firehouse est incomplète : des fichiers étaient restés dans OneDrive sur l’autre PC.' : '',
  ]
    .filter(Boolean)
    .join('\n');
}

/** Le singulier ou le pluriel selon le nombre (jamais « (s) »). */
export const pl = (n: number, un: string, plusieurs: string) => (n > 1 ? plusieurs : un);

/** Lot S5 : dit (sans jamais la valeur) que la clé des abris de ce PC diffère de celle de Firehouse. */
export const TEXTE_SEL_ECART =
  '⚠ La clé des abris de ce PC n’est pas celle gardée chez Firehouse (un autre PC de ce profil) : celle de Firehouse est gardée. Certaines parties d’un autre PC pourraient ne pas retrouver leur place ici.';

/** Les titres cités d'un choix groupé : les 5 premiers, puis « et N autres ». */
export function titresCites(titres: string[], max = 5): string {
  const l = titres.map((t) => t || 'un jeu');
  const vus = [...new Set(l)];
  const reste = vus.length - max;
  return reste > 0 ? `${vus.slice(0, max).join(', ')} et ${reste} ${pl(reste, 'autre', 'autres')}` : vus.join(', ');
}

/** Le bilan d'une pose des parties PC restaurées, en lignes ; vide : rien à dire. */
export function lignesBilan(b: BilanReprise): string[] {
  const l: string[] = [];
  if (b.posees) l.push(`✅ ${b.posees} ${pl(b.posees, 'partie remise', 'parties remises')} à sa place (${fichiers(b.fichiers)}).`);
  if (b.deja_la) l.push(`👌 ${b.deja_la} ${pl(b.deja_la, 'partie était', 'parties étaient')} déjà là, à l’identique.`);
  if (b.non_installes) l.push(`⏳ ${b.non_installes} ${pl(b.non_installes, 'partie attend', 'parties attendent')} que son jeu soit installé : elle${b.non_installes > 1 ? 's seront posées' : ' sera posée'} à la fin de l’installation.`);
  const sans = b.non_resolues + b.identifiants;
  if (sans) l.push(`⏳ ${sans} ${pl(sans, 'partie n’a', 'parties n’ont')} pas encore trouvé sa place sur ce PC : lance le jeu une fois, puis reprends tes parties.`);
  if (b.en_cours) l.push(`⏸ ${b.en_cours} ${pl(b.en_cours, 'partie attend', 'parties attendent')} : le jeu est en cours. Quitte-le, puis reprends tes parties.`);
  if (b.incompletes) l.push(`⏳ ${b.incompletes} ${pl(b.incompletes, 'partie n’est', 'parties ne sont')} pas arrivée${b.incompletes > 1 ? 's' : ''} en entier (téléchargement coupé) : reprends tes parties pour finir.`);
  if (b.partielles) l.push(`⚠ ${b.partielles} ${pl(b.partielles, 'partie est incomplète', 'parties sont incomplètes')} : des fichiers étaient restés dans OneDrive sur l’autre PC.`);
  if (b.conflits) l.push(`❓ ${b.conflits} ${pl(b.conflits, 'partie est différente', 'parties sont différentes')} de celle de ce PC : rien n’est remplacé sans ton choix.`);
  return l;
}

/** L'annonce d'une restauration : combien de jeux, de fichiers, de Mo. */
export function texteApercu(a: ApercuRestauration | null, taille: (o: number) => string): string {
  if (!a) return '';
  const jeux = a.jeux ? `${a.jeux} jeu${a.jeux > 1 ? 'x' : ''}, ` : '';
  return `À télécharger : ${jeux}${fichiers(a.fichiers)}, ${taille(a.octets)}.`;
}

/** Les sauvegardes de CE profil, la plus récente (`derniere.date`) d'abord. */
export function pcDuProfil<T extends { a_moi: boolean; derniere: { date?: string } | null }>(liste: T[]): T[] {
  const date = (s: T) => Number(s.derniere?.date ?? 0) || 0;
  return liste.filter((s) => s.a_moi).sort((a, b) => date(b) - date(a));
}
