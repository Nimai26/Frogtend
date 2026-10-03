// 📥 Importer des jeux déjà sur le disque (ROM, MS-DOS, Windows, ajout manuel). Rien n'est copié, déplacé ni
// renommé : Frogtend note où est le jeu. Retirer un jeu importé ne l'efface jamais du disque.

export interface RomTrouvee {
  chemin: string;
  titre: string;
  taille: number;
}

export interface JeuDosTrouve {
  dossier: string;
  titre: string;
  programmes: string[];
}

export interface JeuAImporter {
  titre: string;
  plateforme: string;
  dossier: string;
  fichier?: string | null;
  programme?: string | null;
  /** Les arguments du programme (un jeu DOS installé : les commandes de DOSBox). */
  arguments?: string[];
  annee?: number | null;
  editeur?: string | null;
  genres?: string[];
}

/** Ce qu'on GARDE d'un dossier MAME (mêmes cases que LaunchBox). */
export interface OptionsMame {
  clones: boolean;
  non_jouables: boolean;
  imparfaits: boolean;
  contrefacons: boolean;
  prototypes: boolean;
  hacks: boolean;
  adultes: boolean;
  jeux_d_argent: boolean;
  mecaniques: boolean;
  quiz: boolean;
  hors_arcade: boolean;
}

export const OPTIONS_MAME_DEFAUT: OptionsMame = {
  clones: false,
  non_jouables: false,
  imparfaits: true,
  contrefacons: false,
  prototypes: false,
  hacks: false,
  adultes: false,
  jeux_d_argent: false,
  mecaniques: false,
  quiz: true,
  hors_arcade: false,
};

export const LIBELLES_MAME: [keyof OptionsMame, string][] = [
  ['imparfaits', 'les jeux imparfaits (son ou image pas tout à fait justes)'],
  ['non_jouables', 'les jeux qui ne marchent pas encore dans MAME'],
  ['clones', 'les clones (autres versions d’un même jeu)'],
  ['contrefacons', 'les contrefaçons (bootlegs)'],
  ['prototypes', 'les prototypes'],
  ['hacks', 'les hacks'],
  ['quiz', 'les jeux de quiz'],
  ['jeux_d_argent', 'casino, machines à sous et mahjong'],
  ['mecaniques', 'les machines mécaniques'],
  ['hors_arcade', 'ce qui n’est pas une borne d’arcade (ordinateurs, consoles)'],
  ['adultes', 'les jeux pour adultes'],
];

export interface JeuMameRetenu extends RomTrouvee {
  annee: number | null;
  editeur: string | null;
  genre: string | null;
}

export interface TriMame {
  retenus: JeuMameRetenu[];
  ecartes: Record<string, number>;
}

/** Les jeux MAME retenus, prêts à ajouter (plateforme « Arcade », le zip garde son nom). */
export function depuisMame(jeux: JeuMameRetenu[]): JeuAImporter[] {
  return jeux.map((j) => {
    const { dossier, nom } = couper(j.chemin);
    return { titre: j.titre, plateforme: 'Arcade', dossier, fichier: nom, annee: j.annee, editeur: j.editeur, genres: j.genre ? [j.genre] : [] };
  });
}

export interface BilanImport {
  ajoutes: number;
  /** Versions (autres régions) ajoutées à des jeux déjà dans la ludothèque. */
  versions?: number;
  deja: number;
  refuses: [string, string][];
}

/** Le dossier et le nom d'un chemin Windows. */
export function couper(chemin: string): { dossier: string; nom: string } {
  const i = Math.max(chemin.lastIndexOf('\\'), chemin.lastIndexOf('/'));
  return i < 0 ? { dossier: '', nom: chemin } : { dossier: chemin.slice(0, i), nom: chemin.slice(i + 1) };
}

/** Les extensions que les émulateurs recommandés lisent (sans point, en minuscules, sans doublon). */
export function extensionsDe(emulateurs: { extensions?: string[] }[]): string[] {
  const l = emulateurs.flatMap((e) => e.extensions ?? []).map((x) => x.trim().replace(/^\./, '').toLowerCase()).filter(Boolean);
  return [...new Set(l)];
}

/** Les types de fichiers qui ne sont jamais des jeux (textes, images, vidéos, sauvegardes, empreintes…). */
export const PAS_DES_JEUX = new Set([
  'txt', 'nfo', 'diz', 'md', 'pdf', 'doc', 'docx', 'rtf', 'htm', 'html', 'xml', 'json', 'ini', 'cfg', 'log', 'dat',
  'jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp', 'ico', 'mkv', 'mp4', 'avi', 'mov', 'wmv', 'mp3', 'ogg', 'flac',
  'srm', 'sav', 'state', 'db', 'lnk', 'url', 'sfv', 'md5', 'sha1', 'par2', 'torrent', 'part', 'tmp', 'bak',
]);

/** Les pistes que décrit une feuille (.cue, .ccd, .gdi) : ce ne sont pas des jeux quand la feuille est là. */
const PISTES = ['bin', 'img', 'sub', 'raw', 'wav'];

/**
 * Les types de fichiers à cocher d'office parmi ceux trouvés dans le dossier : ceux que lisent les émulateurs de la
 * plateforme (plus .zip et .7z), sinon tout ce qui peut être un jeu. Jamais les textes, images, vidéos… ni les pistes
 * .bin d'un jeu sur CD décrit par sa feuille.
 */
export function typesACocher(trouves: [string, number][], recommandes: string[]): string[] {
  const exts = trouves.map(([e]) => e.toLowerCase());
  const feuille = exts.some((e) => ['cue', 'ccd', 'gdi'].includes(e));
  const possibles = exts.filter((e) => !PAS_DES_JEUX.has(e) && !(feuille && PISTES.includes(e)));
  const lus = new Set([...recommandes.map((e) => e.toLowerCase().replace(/^\./, '')), 'zip', '7z']);
  const pris = recommandes.length ? possibles.filter((e) => lus.has(e)) : [];
  return pris.length ? pris : possibles;
}

const mots = (t: string) =>
  t
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    .replace(/entertainement/g, 'entertainment')
    .replace(/[^a-z0-9]+/g, ' ')
    .trim()
    .split(' ')
    .filter(Boolean);

/**
 * La plateforme que le nom d'un dossier désigne (« Playstation 3 » → « Sony Playstation 3 »), parmi celles connues ;
 * `null` si rien ne ressemble assez. Une proposition : la personne peut toujours en choisir une autre.
 */
export function devinerPlateforme(dossier: string, plateformes: string[]): string | null {
  const nom = couper(dossier.replace(/[\\/]+$/, '')).nom;
  const m = new Set(mots(nom));
  if (!m.size) return null;
  let meilleure: string | null = null;
  let score = 0;
  for (const p of plateformes) {
    const mp = new Set(mots(p));
    const communs = [...m].filter((x) => mp.has(x)).length;
    // Tous les mots de l'un dans l'autre (« Playstation 3 » ⊂ « Sony Playstation 3 »), puis le plus proche.
    const inclus = communs === m.size || communs === mp.size;
    const s = communs / (m.size + mp.size - communs);
    if (inclus && communs > 0 && s > score) {
      score = s;
      meilleure = p;
    }
  }
  // 0,3 : « 3DO » trouve « 3DO Interactive Multiplayer » (1 mot sur 3).
  return score >= 0.3 ? meilleure : null;
}

/** « sfc, smc .zip » → ['sfc', 'smc', 'zip']. */
export function lireExtensions(texte: string): string[] {
  return [...new Set(texte.split(/[\s,;]+/).map((x) => x.replace(/^\*?\./, '').toLowerCase()).filter(Boolean))];
}

export function depuisRoms(roms: RomTrouvee[], plateforme: string): JeuAImporter[] {
  return roms.map((r) => {
    const { dossier, nom } = couper(r.chemin);
    return { titre: r.titre, plateforme, dossier, fichier: nom };
  });
}

export function depuisDos(jeux: (JeuDosTrouve & { programme: string })[]): JeuAImporter[] {
  return jeux.map((j) => ({ titre: j.titre, plateforme: 'MS-DOS', dossier: j.dossier, fichier: j.programme }));
}

/** Un jeu Windows d'après son programme : le titre proposé est le nom de son dossier. */
export function depuisProgramme(programme: string, titre?: string): JeuAImporter {
  const { dossier } = couper(programme);
  return { titre: titre?.trim() || couper(dossier).nom || couper(programme).nom.replace(/\.exe$/i, ''), plateforme: 'Windows', dossier, programme };
}

/** Un ajout manuel : un jeu Windows lance son programme ; les autres donnent leur fichier à l'émulateur. */
export function depuisManuel(titre: string, plateforme: string, fichier: string): JeuAImporter {
  if (plateforme === 'Windows' || plateforme === 'Windows 3.X') return { ...depuisProgramme(fichier, titre), plateforme };
  const { dossier, nom } = couper(fichier);
  return { titre: titre.trim(), plateforme, dossier, fichier: nom };
}

/** Un nom de dossier sûr pour Windows, d'après un titre (« Kings Quest V: Absence » → « Kings Quest V - Absence »). */
export function nomDeDossier(titre: string): string {
  return titre.replace(/:/g, ' -').replace(/[<>"/\\|?*]/g, '').replace(/\s+/g, ' ').trim().replace(/[. ]+$/, '');
}

/** Un jeu DOS installé par Frogtend : DOSBox, lancé avec ses commandes (C: monté sur le dossier d'installation). */
export function depuisInstallationDos(titre: string, destination: string, dosbox: string, argumentsDosbox: string[]): JeuAImporter {
  return { titre: titre.trim(), plateforme: 'MS-DOS', dossier: destination, programme: dosbox, arguments: argumentsDosbox };
}

// --- Où garder les jeux importés (règle « pas de pieuvre », Seb 02/10) : les laisser où ils sont (leur dossier devient
// une source du système dans les réglages) ou les copier dans l'emplacement du système. Jamais déplacés. ---

export interface Emplacements {
  defaut: string[];
  systemes: Record<string, string[]>;
}

const bas = (c: string) => c.replace(/\//g, '\\').replace(/\\+$/, '').toLowerCase();

/** Ce qui représente le jeu sur le disque : son dossier (jeu Windows ou DOS), sinon son fichier. */
export function elementDe(j: JeuAImporter): string {
  if (j.programme || !j.fichier || /[\\/]/.test(j.fichier)) return j.dossier;
  return `${j.dossier.replace(/[\\/]+$/, '')}\\${j.fichier}`;
}

/** Le dossier où COPIER les jeux d'un système : son premier emplacement propre, sinon `<défaut>\<système>`. */
export function emplacementDuSysteme(e: Emplacements, plateforme: string, nomDossier: (t: string) => string): string | null {
  const propre = e.systemes[plateforme]?.[0];
  if (propre) return propre;
  return e.defaut[0] ? `${e.defaut[0].replace(/[\\/]+$/, '')}\\${nomDossier(plateforme)}` : null;
}

/** Vrai si ce chemin est déjà dans un emplacement réglé (du système, ou par défaut). */
export function dansUnEmplacement(chemin: string, e: Emplacements, plateforme: string): boolean {
  const c = bas(chemin);
  return [...(e.systemes[plateforme] ?? []), ...e.defaut].some((d) => c === bas(d) || c.startsWith(`${bas(d)}\\`));
}

/** Les emplacements avec ce dossier ajouté aux sources du système (`null` s'il y est déjà couvert). */
export function avecSource(e: Emplacements, plateforme: string, dossier: string): Record<string, string[]> | null {
  if (dansUnEmplacement(dossier, e, plateforme)) return null;
  return { ...e.systemes, [plateforme]: [...(e.systemes[plateforme] ?? []), dossier] };
}

/** Les jeux après copie : chacun pointe vers SA copie (même nom de fichier, autre dossier). */
export function apresCopie(jeux: JeuAImporter[], nouveaux: string[]): JeuAImporter[] {
  return jeux.map((j, i) => {
    const n = nouveaux[i];
    if (elementDe(j) === j.dossier) {
      const programme = j.programme ? `${n}${j.programme.slice(j.dossier.replace(/[\\/]+$/, '').length)}` : j.programme;
      return { ...j, dossier: n, programme };
    }
    return { ...j, dossier: couper(n).dossier };
  });
}

export function messageBilan(b: BilanImport): string {
  const morceaux = [`✅ ${b.ajoutes} jeu(x) ajouté(s) à ta ludothèque`];
  if (b.versions) morceaux.push(`${b.versions} version(s) ajoutée(s) à des jeux déjà là`);
  if (b.deja) morceaux.push(`${b.deja} déjà dedans`);
  if (b.refuses.length) morceaux.push(`${b.refuses.length} refusé(s) : ${b.refuses.slice(0, 3).map(([t, m]) => `${t} (${m})`).join(', ')}`);
  return morceaux.join(' · ');
}
