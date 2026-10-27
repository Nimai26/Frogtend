// Le pack MAME (contrat 1.33 ; ✅ Seb 27/10) : tout le romset de Firehouse dans un dossier à lui, annoncé et chiffré
// AVANT tout accord (combien de fichiers, combien de Go, combien de temps, la place libre).

export interface AnnoncePack {
  version: string;
  a_recevoir: number;
  octets: number;
  deja_la: number;
  /** Fichiers d'une version plus ancienne dans le pack : la mise à jour viendra (rien n'est remplacé). */
  anciens: number;
  /** Une réception du pack est déjà en cours sur ce PC. */
  en_cours: boolean;
  libre: number | null;
  chd_fichiers: number;
  chd_octets: number;
}

const GO = 1024 ** 3;
const go = (o: number) => `${(o / GO).toLocaleString('fr-FR', { maximumFractionDigits: 1 })} Go`;

/** Assez de place pour tout (avec 1 Go de marge) ? Place inconnue : on ne bloque pas. */
export function placeSuffisante(a: AnnoncePack): boolean {
  return a.libre === null || a.libre >= a.octets + GO;
}

/** L'annonce lisible : combien, combien de Go, combien de temps (au débit mesuré sur ce PC), la place. */
export function texteAnnoncePack(a: AnnoncePack, debit: number | null, duree: (s: number) => string): string {
  if (a.a_recevoir === 0) return `Le pack MAME ${a.version} est complet dans ce dossier (${a.deja_la} fichier(s)).`;
  const lignes = [
    `${a.a_recevoir.toLocaleString('fr-FR')} fichier(s) à recevoir, ${go(a.octets)} (MAME ${a.version})${a.deja_la ? ` ; ${a.deja_la.toLocaleString('fr-FR')} déjà là` : ''}.`,
    debit ? `Durée estimée à ta vitesse habituelle : ${duree(Math.round(a.octets / debit))}.` : 'Durée : inconnue (Frogtend n’a pas encore mesuré ta connexion).',
    a.libre === null ? '' : `Place libre sur ce disque : ${go(a.libre)}${placeSuffisante(a) ? '' : ' — pas assez !'}.`,
    a.anciens ? `${a.anciens} fichier(s) de ton pack sont d’une version plus ancienne : la mise à jour du pack viendra (rien n’est remplacé).` : '',
  ];
  return lignes.filter(Boolean).join('\n');
}

/** MAME et son romset doivent être de la même version (un romset 0.289 dans un MAME 0.288 : des jeux qui ne démarrent
 * pas). Rend l'avis à montrer, ou rien quand tout va. `mame` : le MAME installé (s'il y en a un). */
export function avisVersionMame(mame: { version: string | null; par_frogtend: boolean } | undefined, attendue: string): string | null {
  if (!attendue) return null;
  if (!mame) return `MAME n’est pas encore installé : Frogtend l’installera (version ${attendue}) au premier jeu, après ton accord.`;
  if (!mame.version) return `Ton MAME a été installé à la main (version inconnue) : le pack est fait pour MAME ${attendue}.`;
  if (mame.version === attendue) return null;
  return mame.par_frogtend
    ? `Ton MAME est en version ${mame.version}, le pack en ${attendue} : mets MAME à jour (⚙ Options ▸ Émulateurs ▸ MAME), sinon certains jeux ne démarreront pas.`
    : `Ton MAME est en version ${mame.version}, le pack en ${attendue} : certains jeux ne démarreront pas tant que MAME n’est pas en ${attendue}.`;
}
