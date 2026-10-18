// Le visuel des jeux dans la ludothèque (lot Médias, M1 ; contrat 1.6) : jaquette, boîte en relief, cartouche…
// Une grille RÉGULIÈRE (relu avec l'expert interface, 18/10) : un cadre au ratio type du visuel choisi, et dedans une
// boîte au ratio RÉEL de l'image (connu par le catalogue) qui porte la carte de remplacement puis l'image : la place
// est réservée d'emblée, plus rien ne bouge quand l'image arrive.

export type Visuel = 'jaquette' | 'boite_3d' | 'cartouche' | 'ecran_titre' | 'capture';

export const VISUELS: { valeur: Visuel; libelle: string; cadre: string }[] = [
  { valeur: 'jaquette', libelle: 'Jaquette', cadre: '3 / 4' },
  { valeur: 'boite_3d', libelle: 'Boîte en relief', cadre: '3 / 4' },
  { valeur: 'cartouche', libelle: 'Cartouche', cadre: '4 / 3' },
  { valeur: 'ecran_titre', libelle: 'Écran titre', cadre: '4 / 3' },
  { valeur: 'capture', libelle: 'Image du jeu', cadre: '4 / 3' },
];

export type Dimensions = Record<string, { largeur?: number | null; hauteur?: number | null }>;

/**
 * Ce que montre la carte d'un jeu pour le visuel choisi : le type effectif (la jaquette si le jeu n'a pas ce visuel,
 * quand on le SAIT ; sinon on essaie, le cœur se replie lui-même sur la jaquette), le ratio du cadre, le ratio réel
 * de l'image s'il est connu, et s'il faut garder les gros pixels (une petite image d'époque agrandie).
 */
export function visuelDuJeu(
  choisi: string | undefined,
  images: Dimensions | null | undefined,
  largeurAffichee: number,
): { sorte: Visuel; cadre: string; ratio: number | null; pixels: boolean } {
  const v = VISUELS.find((x) => x.valeur === choisi) ?? VISUELS[0];
  // Le jeu n'a pas ce visuel (catalogue relu) : sa jaquette.
  const sorte: Visuel = images && v.valeur !== 'jaquette' && !images[v.valeur] ? 'jaquette' : v.valeur;
  // Le cadre du visuel CHOISI pour toute la grille (des lignes régulières) ; un repli sur la jaquette s'y loge par sa
  // boîte au ratio réel.
  const cadre = v.cadre;
  const d = images?.[sorte];
  const ratio = d?.largeur && d?.hauteur ? d.largeur / d.hauteur : null;
  const pixels = (sorte === 'capture' || sorte === 'ecran_titre') && !!d?.largeur && d.largeur < largeurAffichee;
  return { sorte, cadre, ratio, pixels };
}
