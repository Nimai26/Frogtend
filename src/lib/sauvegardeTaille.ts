// La taille d'une sauvegarde, annoncée AVANT de l'envoyer (une première sauvegarde PS3 ou Xbox peut peser des
// centaines de Mo, et Frogtend vit aussi chez des connexions modestes).

/** Ce qu'une sauvegarde enverrait (`sauvegarde_estimer`). Lot S4 : `a_envoyer` = ce qui partira VRAIMENT (contre le
 * dernier manifeste de Firehouse) ; `utilise` / `quota` = la place chez Firehouse. `null` : pas lus (hors ligne…). */
export interface EstimationSauvegarde {
  total: number;
  precedent: number | null;
  a_envoyer?: number | null;
  fichiers_a_envoyer?: number | null;
  utilise?: number | null;
  quota?: number | null;
  /** Ce qui manquerait de place chez Firehouse pour cet envoi, calculé par le cœur (le même calcul que l'envoi). */
  manque?: number | null;
}

/** Les octets NOUVEAUX à annoncer (au-delà du seuil réglé), ou `null` : rien à demander. Lot S4 : ce qui partira
 * vraiment quand Firehouse l'a dit ; sinon, comme avant, la différence avec la dernière sauvegarde (sans elle, tout
 * est nouveau ; une sauvegarde qui a maigri n'annonce rien). */
export function aAnnoncer(estimation: EstimationSauvegarde, seuilMo: number): number | null {
  const nouveaux = estimation.a_envoyer ?? Math.max(0, estimation.total - (estimation.precedent ?? 0));
  return nouveaux > seuilMo * 1024 * 1024 ? nouveaux : null;
}

/** La place qui manquerait chez Firehouse (0 : assez, ou inconnue) : celle que le cœur a calculée — jamais recalculée
 * ici (une seule règle, celle de l'envoi). */
export function placeManquante(estimation: EstimationSauvegarde): number {
  return Math.max(0, estimation.manque ?? 0);
}

/** Le libellé du bouton de sauvegarde (lot S4 : l'envoi se met en pause pendant une partie, et le dit). */
export function libelleSauvegarde(enCours: boolean, partieEnCours: boolean): string {
  if (!enCours) return '💾 Sauvegarder maintenant';
  return partieEnCours ? '⏸ Sauvegarde en pause pendant ta partie…' : 'Sauvegarde en cours…';
}

/** Le dossier commun des copies restaurées en attente (`…\parties\emulateurs`), sinon la copie elle-même. */
export function dossierDesCopies(copie: string): string {
  const m = /^(.*?[\\/]parties[\\/]emulateurs)(?:[\\/]|$)/i.exec(copie);
  return m ? m[1] : copie;
}

/** Le dossier d'abri commun d'un remplacement (`…\remplacees\<date>`), sinon l'abri lui-même. */
export function dossierDesAbris(abri: string): string {
  const m = /^(.*?[\\/]remplacees[\\/][^\\/]+)(?:[\\/]|$)/i.exec(abri);
  return m ? m[1] : abri;
}
