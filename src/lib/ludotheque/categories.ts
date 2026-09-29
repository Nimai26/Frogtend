// Le classement des plateformes en catégories, pour l'arbre de gauche (comme LaunchBox).
// Les noms sont ceux de LaunchBox, que Firehouse emploie. Un nom inconnu va dans « Autres » : on n'en perd aucun.

export type Categorie = 'Arcade' | 'Consoles' | 'Consoles portables' | 'Ordinateurs' | 'Autres';

export const ORDRE_CATEGORIES: Categorie[] = ['Arcade', 'Consoles', 'Consoles portables', 'Ordinateurs', 'Autres'];

export const ICONES_CATEGORIES: Record<Categorie, string> = {
  Arcade: '🕹',
  Consoles: '🎮',
  'Consoles portables': '📱',
  Ordinateurs: '💻',
  Autres: '📦',
};

const PORTABLES = [
  'Nintendo Game Boy', 'Nintendo Game Boy Color', 'Nintendo Game Boy Advance', 'Nintendo DS', 'Nintendo 3DS',
  'Nintendo Virtual Boy', 'Nintendo Game & Watch', 'Sony PSP', 'Sony PlayStation Vita', 'Sega Game Gear',
  'Atari Lynx', 'SNK Neo Geo Pocket', 'SNK Neo Geo Pocket Color', 'Bandai WonderSwan', 'Bandai WonderSwan Color',
  'NEC TurboExpress', 'Nokia N-Gage', 'Tiger Game.com', 'Sega Nomad',
];

const ORDINATEURS = [
  'MS-DOS', 'Windows', 'Windows 3.X', 'Linux', 'Apple Mac OS', 'Apple II', 'Apple IIGS', 'Commodore 64',
  'Commodore Amiga', 'Commodore Amiga CD32', 'Commodore VIC-20', 'Commodore PET', 'Commodore 128',
  'Amstrad CPC', 'Amstrad GX4000', 'Atari ST', 'Atari 800', 'Sinclair ZX Spectrum', 'Sinclair ZX-81',
  'Microsoft MSX', 'Microsoft MSX2', 'MSX', 'MSX2', 'NEC PC-8801', 'NEC PC-9801', 'Sharp X68000', 'Sharp X1',
  'Fujitsu FM Towns Marty', 'Fujitsu FM-7', 'Tandy TRS-80', 'Thomson MO/TO', 'Oric Atmos', 'Acorn Electron',
  'BBC Microcomputer System', 'Texas Instruments TI 99/4A', 'Dragon 32/64', 'Web Browser', 'ScummVM', 'Pinball',
];

const ARCADE = ['Arcade', 'MAME', 'SNK Neo Geo AES', 'SNK Neo Geo MVS', 'Sega Naomi', 'Sega Model 2', 'Sega Model 3', 'Namco System 22', 'Daphne', 'American Laser Games'];

/** La catégorie d'une plateforme, d'après son nom LaunchBox. */
export function categorieDe(nom: string): Categorie {
  if (ARCADE.includes(nom)) return 'Arcade';
  if (PORTABLES.includes(nom)) return 'Consoles portables';
  if (ORDINATEURS.includes(nom)) return 'Ordinateurs';
  // Les consoles de salon : les fabricants connus, sauf ce qui a déjà été classé plus haut.
  if (
    /^(Nintendo|Sony|Sega|Microsoft Xbox|Atari|NEC|SNK|3DO|Panasonic|Philips|Mattel|Coleco|ColecoVision|Magnavox|Bally|Emerson|Epoch|Fairchild|GCE|Bandai|Casio|Entex|Hartung|Interton|Memorex|Nuon|Ouya|RCA|VTech|Watara|Super Nintendo|Sony Playstation)/i.test(
      nom,
    )
  ) {
    return 'Consoles';
  }
  return 'Autres';
}

/** Regroupe des plateformes par catégorie, dans l'ordre d'affichage ; chaque groupe trié par nom. */
export function grouperParCategorie<T extends { nom: string }>(plateformes: T[]): { categorie: Categorie; plateformes: T[] }[] {
  const groupes = new Map<Categorie, T[]>();
  for (const p of plateformes) {
    const c = categorieDe(p.nom);
    groupes.set(c, [...(groupes.get(c) ?? []), p]);
  }
  return ORDRE_CATEGORIES.filter((c) => groupes.has(c)).map((c) => ({
    categorie: c,
    plateformes: groupes.get(c)!.sort((a, b) => a.nom.localeCompare(b.nom, 'fr')),
  }));
}
