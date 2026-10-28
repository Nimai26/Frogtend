// Les noms courts des plateformes (contrat 1.27), par nom LaunchBox : AFFICHAGE seulement. Module à part (sans appel
// réseau) pour que l'état du profil puisse les oublier à la sortie sans import circulaire avec la ludothèque.

/** Les noms courts connus pour TOUT le catalogue, quel que soit le filtre (Taodbox nomme les consoles des jeux du PC). */
export const nomsCourts = $state<Record<string, string>>({});

/** Les noms MONTRÉS par plus d'une plateforme : celles-là gardent leur nom LaunchBox (sinon indiscernables). */
let doublons = $state<Set<string>>(new Set());

/** Le nom à MONTRER d'une plateforme : son nom court, sinon son nom LaunchBox — et son nom LaunchBox aussi quand une
 * autre plateforme montrerait le même nom. Jamais une clé (filtres, réglages, chemins gardent le nom LaunchBox). */
export function nomAffiche(nom: string | null | undefined): string {
  if (!nom) return nom || '';
  const court = nomsCourts[nom];
  return court && !doublons.has(court) ? court : nom;
}

/** Remplace la table par ces listes (réunies) : un nom court retiré par Firehouse disparaît, et les noms montrés deux
 * fois sont repérés sur tout ce qui est connu (une console porte le même nom partout). */
export function retenirNomsCourts(...listes: { nom: string; nom_affiche?: string }[][]) {
  const table: Record<string, string> = {};
  const noms = new Set<string>();
  for (const l of listes) for (const p of l) {
    noms.add(p.nom);
    if (p.nom_affiche) table[p.nom] = p.nom_affiche;
  }
  // Combien de plateformes montrent chaque nom (le nom court, sinon le nom LaunchBox).
  const vus = new Map<string, number>();
  for (const n of noms) {
    const montre = table[n] || n;
    vus.set(montre, (vus.get(montre) ?? 0) + 1);
  }
  for (const k of Object.keys(nomsCourts)) if (!(k in table)) delete nomsCourts[k];
  for (const [k, v] of Object.entries(table)) if (nomsCourts[k] !== v) nomsCourts[k] = v;
  doublons = new Set([...vus].filter(([, c]) => c > 1).map(([m]) => m));
}

/** À la sortie d'un profil : rien de ce qu'il a reçu ne reste (le cache des plateformes est propre à chaque profil). */
export function oublierNomsCourts() {
  for (const k of Object.keys(nomsCourts)) delete nomsCourts[k];
  doublons = new Set();
}
