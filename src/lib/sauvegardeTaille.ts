// La taille d'une sauvegarde, annoncée AVANT de l'envoyer (une première sauvegarde PS3 ou Xbox peut peser des
// centaines de Mo, et Frogtend vit aussi chez des connexions modestes).

/** Les octets NOUVEAUX à annoncer (au-delà du seuil réglé), ou `null` : rien à demander. Sans sauvegarde précédente,
 * tout est nouveau ; une sauvegarde qui a maigri n'annonce rien. */
export function aAnnoncer(estimation: { total: number; precedent: number | null }, seuilMo: number): number | null {
  const nouveaux = Math.max(0, estimation.total - (estimation.precedent ?? 0));
  return nouveaux > seuilMo * 1024 * 1024 ? nouveaux : null;
}
