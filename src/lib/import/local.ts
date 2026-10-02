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

export function messageBilan(b: BilanImport): string {
  const morceaux = [`✅ ${b.ajoutes} jeu(x) ajouté(s) à ta ludothèque`];
  if (b.deja) morceaux.push(`${b.deja} déjà dedans`);
  if (b.refuses.length) morceaux.push(`${b.refuses.length} refusé(s) : ${b.refuses.slice(0, 3).map(([t, m]) => `${t} (${m})`).join(', ')}`);
  return morceaux.join(' · ');
}
