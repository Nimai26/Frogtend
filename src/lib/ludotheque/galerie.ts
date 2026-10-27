// La galerie d'un jeu dans sa fiche (lot Médias M2) : l'ordre, les libellés pour tout le monde (aucun jargon, pas de
// dimensions) et la largeur à demander.

export interface MediaJeu {
  id: number;
  type: string;
  principal: boolean;
  region?: string | null;
  largeur?: number | null;
  hauteur?: number | null;
  /** D'où vient l'image (`launchbox`, `libretro`, `igdb`…), et ses crédits s'ils sont donnés. */
  source?: string | null;
  licence?: string | null;
  auteur?: string | null;
  /** La page d'origine, montrée en texte quand une licence la demande. */
  url_source?: string | null;
}

/** Les noms à créditer, par source (les images de LaunchBox ne demandent rien). */
const SOURCES: Record<string, string> = { libretro: 'libretro-thumbnails', igdb: 'IGDB.com' };

/** Le crédit d'une image (« Image : IGDB.com », « Image : Dupont · CC BY-SA 4.0 ») ; vide pour LaunchBox ou sans
 * source. */
export function creditMedia(m: MediaJeu): string {
  const s = (m.source ?? '').trim();
  if (!s || s.toLowerCase() === 'launchbox') return '';
  const qui = [m.auteur?.trim(), SOURCES[s.toLowerCase()] ?? s].filter(Boolean).join(', ');
  return `Image : ${qui}${m.licence?.trim() ? ` · ${m.licence.trim()}` : ''}`;
}

/** L'ordre des types (relu avec l'expert interface, 18/10) : la jaquette principale est déjà en haut de la fiche. */
const ORDRE = ['boite_3d', 'dos', 'tranche', 'cartouche', 'cartouche_3d', 'ecran_titre', 'capture', 'logo', 'fond', 'autre_image', 'jaquette'];

const LIBELLES: Record<string, string> = {
  boite_3d: 'Boîte en relief',
  dos: 'Dos de la boîte',
  tranche: 'Tranche',
  cartouche: 'Cartouche',
  cartouche_3d: 'Cartouche en relief',
  ecran_titre: 'Écran titre',
  capture: 'Image du jeu',
  logo: 'Logo',
  fond: 'Fond',
  autre_image: 'Autre image',
  jaquette: 'Jaquette',
};

const REGIONS: Record<string, string> = {
  Japan: 'Japon',
  Europe: 'Europe',
  'North America': 'Amérique du Nord',
  'United States': 'États-Unis',
  USA: 'États-Unis',
  France: 'France',
  World: 'Monde',
  Germany: 'Allemagne',
  'United Kingdom': 'Royaume-Uni',
  Australia: 'Australie',
  Korea: 'Corée',
  Brazil: 'Brésil',
  Spain: 'Espagne',
  Italy: 'Italie',
};

export function libelleMedia(m: MediaJeu): string {
  const type = LIBELLES[m.type] ?? 'Autre image';
  const region = m.region ? (REGIONS[m.region] ?? m.region) : '';
  return region ? `${type} · ${region}` : type;
}

/** Les visuels dans l'ordre de la galerie : par type, la principale d'abord. */
export function ordonner(l: MediaJeu[]): MediaJeu[] {
  // Un type inconnu se range avec les « autres images » (c'est ainsi qu'il est nommé).
  const rang = (t: string) => {
    const i = ORDRE.indexOf(t);
    return i < 0 ? ORDRE.indexOf('autre_image') : i;
  };
  return [...l].sort((a, b) => rang(a.type) - rang(b.type) || Number(b.principal) - Number(a.principal) || a.id - b.id);
}

/** La largeur à demander : celle de l'affichage, jamais plus que l'original (une capture de 256 reste en 256). */
export function largeurUtile(m: MediaJeu, affichee: number): number {
  return m.largeur ? Math.min(m.largeur, affichee) : affichee;
}

/** Une petite image d'époque agrandie au moins deux fois (une capture de 256 px) : garder ses gros pixels. Une
 * image à peine agrandie reste lissée. */
export function grosPixels(m: MediaJeu, affichee: number): boolean {
  return !!m.largeur && m.largeur * 2 <= affichee;
}
