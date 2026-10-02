// Le menu « Importer » (Seb, 02/10) : la MÊME liste que LaunchBox, dans le même ordre. Chaque entrée dit par quelle
// voie Frogtend la sert : l'API Steam, GOG Galaxy (qui regroupe les boutiques que la personne y a reliées), un
// import local (jeux déjà sur le disque), ou « bientôt » (pas encore livré : on le dit au lieu de faire semblant).

export type Voie = 'steam' | 'galaxy' | 'local' | 'bientot';

export interface SourceImport {
  id: string;
  libelle: string;
  icone: string;
  voie: Voie;
  /** Les préfixes de plateforme de GOG Galaxy qui correspondent à cette boutique (voie « galaxy »). */
  galaxy?: string[];
  /** Ce que fait l'import, en une phrase. */
  resume: string;
}

export const SOURCES: SourceImport[] = [
  {
    id: 'rom',
    libelle: 'Fichiers ROM',
    icone: '💾',
    voie: 'local',
    resume: 'Ajouter des ROM et des images disque d’un dossier, par système, sans jamais les renommer.',
  },
  { id: 'dos', libelle: 'Jeux MS-DOS', icone: '🖥', voie: 'local', resume: 'Ajouter des jeux MS-DOS déjà présents sur le disque, lancés par DOSBox.' },
  { id: 'mame', libelle: 'MAME Arcade Full Set', icone: '🕹', voie: 'local', resume: 'Ajouter un ensemble complet de ROM MAME, en écartant les clones et les jeux non jouables.' },
  { id: 'amazon', libelle: 'Amazon Games', icone: '📦', voie: 'galaxy', galaxy: ['amazon'], resume: 'Les jeux de ton compte Amazon (Prime Gaming), par GOG Galaxy.' },
  { id: 'ea', libelle: 'EA', icone: '🅴', voie: 'galaxy', galaxy: ['origin', 'ea'], resume: 'Les jeux de ton compte EA (EA app, ex-Origin), par GOG Galaxy.' },
  { id: 'epic', libelle: 'Jeux Epic Games', icone: '⚫', voie: 'galaxy', galaxy: ['epic'], resume: 'Les jeux de ton compte Epic Games, par GOG Galaxy.' },
  { id: 'gog', libelle: 'Jeux GOG', icone: '🟣', voie: 'galaxy', galaxy: ['gog'], resume: 'Les jeux de ton compte GOG, par GOG Galaxy.' },
  { id: 'steam', libelle: 'Jeux Steam', icone: '🎮', voie: 'steam', galaxy: ['steam'], resume: 'Les jeux de ton compte Steam, par l’API officielle de Steam (ou par GOG Galaxy).' },
  { id: 'ubisoft', libelle: 'Uplay / Ubisoft Connect', icone: '🌀', voie: 'galaxy', galaxy: ['uplay'], resume: 'Les jeux de ton compte Ubisoft Connect, par GOG Galaxy.' },
  { id: 'windows', libelle: 'Jeux Windows', icone: '🪟', voie: 'local', resume: 'Ajouter un jeu Windows installé à la main, en choisissant son programme (.exe).' },
  { id: 'xbox', libelle: 'Jeux Xbox / Microsoft Store', icone: '🟢', voie: 'galaxy', galaxy: ['xboxone', 'xbox'], resume: 'Les jeux de ton compte Xbox / Microsoft Store (et du Game Pass PC), par GOG Galaxy.' },
  { id: 'manuel', libelle: 'Ajouter un jeu manuellement', icone: '✏', voie: 'local', resume: 'Créer une fiche à la main : titre, plateforme, programme ou fichier à lancer.' },
  { id: 'installer-dos', libelle: 'Installer un jeu DOS', icone: '📀', voie: 'bientot', resume: 'Installer un jeu DOS depuis son disque ou son archive, dans DOSBox, puis l’ajouter.' },
];

export function source(id: string | null | undefined): SourceImport | undefined {
  return SOURCES.find((s) => s.id === id);
}

/** Combien de jeux de cette boutique GOG Galaxy a déjà rendus (d'après leur préfixe de plateforme). */
export function compterParGalaxy(s: SourceImport, jeux: { plateforme?: string | null }[]): number {
  if (!s.galaxy) return 0;
  return jeux.filter((j) => s.galaxy!.includes(j.plateforme ?? '')).length;
}

/**
 * L'explication du regroupement par GOG Galaxy, la même partout (aide, fenêtre d'import, Options ▸ Comptes) — demande
 * de Seb. `boutique` : le nom de la boutique dont on parle, ou rien pour l'explication générale.
 */
export function explicationGalaxy(boutique?: string): string[] {
  const qui = boutique ?? 'Epic, Amazon, EA, Ubisoft, Xbox, Steam…';
  return [
    `GOG Galaxy (le lanceur gratuit de GOG) sait REGROUPER tes comptes : une fois ${boutique ? `ton compte ${boutique}` : 'tes comptes'} relié${boutique ? '' : 's'} dans Galaxy, ses jeux y apparaissent avec ceux de GOG. Frogtend lit alors tout d’un coup, sans te demander aucun mot de passe : ${qui}.`,
    'Pour relier un compte : ouvre GOG Galaxy ▸ l’engrenage en haut à gauche ▸ Paramètres ▸ Intégrations ▸ « Connecter » à côté de la boutique, et connecte-toi sur sa page officielle. Galaxy synchronise ensuite tes jeux tout seul.',
    'Frogtend lit la base de Galaxy en LECTURE SEULE (sur une copie) : il ne change rien dans Galaxy ni dans tes comptes. Pour jouer ou installer un de ces jeux, Frogtend demande à Galaxy de l’ouvrir.',
  ];
}
