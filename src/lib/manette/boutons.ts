// Les boutons de la manette du joueur et ceux de la manette d'origine qu'ils remplacent (lot 4d, ✅ Seb 28/10 : « une
// image de la manette avec la correspondance vers les boutons de la manette d'origine »). Des tables FIXES, relevées dans
// le code source de chaque cœur libretro (28/10, réglages par défaut, sans remappage) et dans les profils que Frogtend
// écrit lui-même (DuckStation, PCSX2, Dolphin : `src-tauri/src/manettes.rs`). Jamais une correspondance devinée :
// inconnue, elle n'est pas dessinée.

/** La manette du joueur : Xbox (et la manette virtuelle de Sunshine) ou PlayStation. */
export type Famille = 'xbox' | 'playstation';

/** Les boutons, par POSITION (les mêmes noms que la correspondance manette → clavier, `ludotheque/clavier.ts`). */
export type IdBouton = 'a' | 'b' | 'x' | 'y' | 'lb' | 'rb' | 'lt' | 'rt' | 'croix' | 'stickg' | 'stickd' | 'select' | 'start' | 'l3' | 'r3';

export const ORDRE: IdBouton[] = ['a', 'b', 'x', 'y', 'lb', 'rb', 'lt', 'rt', 'croix', 'stickg', 'stickd', 'select', 'start', 'l3', 'r3'];

const COMMUNS: Record<'croix' | 'stickg' | 'stickd', string> = { croix: 'Croix directionnelle', stickg: 'Stick gauche', stickd: 'Stick droit' };

/** Le nom de chaque bouton sur la manette du joueur. */
export const NOMS: Record<Famille, Record<IdBouton, string>> = {
  xbox: { a: 'A', b: 'B', x: 'X', y: 'Y', lb: 'LB', rb: 'RB', lt: 'LT', rt: 'RT', select: 'Vue (⧉)', start: 'Menu (≡)', l3: 'L3 (clic du stick gauche)', r3: 'R3 (clic du stick droit)', ...COMMUNS },
  playstation: { a: 'Croix', b: 'Rond', x: 'Carré', y: 'Triangle', lb: 'L1', rb: 'R1', lt: 'L2', rt: 'R2', select: 'Créer', start: 'Options', l3: 'L3', r3: 'R3', ...COMMUNS },
};

/** RetroArch : la manette du joueur devient la « RetroPad » (profil XInput livré, `input_autodetect_builtin.c` :
 * A → B, B → A, X → Y, Y → X — la RetroPad a la disposition de la Super Nintendo). */
const VERS_RETROPAD: Record<IdBouton, string> = {
  a: 'B', b: 'A', x: 'Y', y: 'X', lb: 'L', rb: 'R', lt: 'L2', rt: 'R2', select: 'Select', start: 'Start', l3: 'L3', r3: 'R3', croix: 'Croix', stickg: 'StickG', stickd: 'StickD',
};

/** Une disposition : la console nommée, le bouton (RetroPad pour RetroArch, sinon le bouton du joueur) → celui
 * d'origine, et une remarque éventuelle. */
interface Disposition {
  console: string;
  boutons: Record<string, string>;
  remarque?: string;
}

/** Le rôle de la croix directionnelle (jamais « Croix » seul : c'est aussi le nom d'un bouton PlayStation). */
const CROIX = 'Croix directionnelle';

const NES = (turbo: string): Disposition => ({
  console: 'NES',
  boutons: { B: 'B', A: 'A', Y: `Turbo B${turbo}`, X: `Turbo A${turbo}`, Select: 'Select', Start: 'Start', Croix: CROIX },
});
const PS1: Disposition = {
  console: 'PlayStation',
  boutons: { B: 'Croix', A: 'Rond', Y: 'Carré', X: 'Triangle', L: 'L1', R: 'R1', L2: 'L2', R2: 'R2', Select: 'Select', Start: 'Start', Croix: CROIX },
};
const N64 = (boutonsC: boolean): Disposition => ({
  console: 'Nintendo 64',
  boutons: {
    B: 'A', Y: 'B', L: 'L', R: 'R', L2: 'Z', R2: 'Boutons C (tenu)', Start: 'Start', Croix: CROIX, StickG: 'Stick', StickD: 'Boutons C',
    ...(boutonsC ? { A: 'C-bas (avec RT tenu ; plus simple : le stick droit)', X: 'C-haut (avec RT tenu ; plus simple : le stick droit)' } : {}),
  },
});
const DS = (l2: string, r2: string, stickd?: string): Disposition => ({
  console: 'Nintendo DS',
  boutons: { B: 'B', A: 'A', Y: 'Y', X: 'X', L: 'L', R: 'R', L2: l2, R2: r2, Select: 'Select', Start: 'Start', Croix: CROIX, ...(stickd ? { StickD: stickd } : {}) },
});
const ARCADE = (l: string, r: string): Disposition => ({
  console: 'l’arcade',
  boutons: { B: 'Bouton 1', A: 'Bouton 2', Y: 'Bouton 3', X: 'Bouton 4', L: l, R: r, Select: 'Pièce', Start: 'Start', Croix: 'Joystick' },
  remarque: 'Certains jeux (combat, Neo Geo) ont leur propre disposition.',
});

/** Les cœurs de RetroArch (nom du cœur — `corename` de son fichier info, relevé dans le RetroArch 1.22.2 livré —, en
 * minuscules sans espace ni ponctuation) → leur disposition par défaut. Un nom absent n'est JAMAIS rapproché d'un
 * autre (« Mesen-S » n'est pas Mesen, « Snes9x 2010 » n'est pas Snes9x). */
const COEURS: Record<string, (plateforme: string) => Disposition> = {
  snes9x: () => ({ console: 'Super Nintendo', boutons: { B: 'B', A: 'A', Y: 'Y', X: 'X', L: 'L', R: 'R', Select: 'Select', Start: 'Start', Croix: CROIX } }),
  mesen: () => NES(' (si le turbo est activé)'),
  fceumm: () => NES(' (si le turbo est activé)'),
  nestopia: () => NES(''),
  gambatte: () => ({ console: 'Game Boy', boutons: { B: 'B', A: 'A', Y: 'Turbo B', X: 'Turbo A', Select: 'Select', Start: 'Start', Croix: CROIX } }),
  mgba: () => ({
    console: 'Game Boy Advance',
    boutons: { B: 'B', A: 'A', Y: 'Turbo B', X: 'Turbo A', L: 'L', R: 'R', L2: 'Turbo L', R2: 'Turbo R', Select: 'Select', Start: 'Start', Croix: CROIX },
  }),
  genesisplusgxwide: (plateforme) => COEURS.genesisplusgx(plateforme),
  genesisplusgx: (plateforme): Disposition =>
    /master system|game gear|sg-1000/i.test(plateforme)
      ? { console: /game gear/i.test(plateforme) ? 'Game Gear' : 'Master System', boutons: { B: 'Bouton 1', A: 'Bouton 2', Start: 'Pause / Start', Croix: CROIX } }
      : {
          console: 'Mega Drive',
          boutons: { B: 'B', A: 'C', Y: 'A', X: 'Y', L: 'X', R: 'Z', Select: 'Mode', Start: 'Start', Croix: CROIX },
          remarque: 'Manette à 6 boutons ; un jeu à 3 boutons n’utilise que A, B et C.',
        },
  mupen64plusnext: () => N64(true),
  paralleln64: () => N64(false),
  beetlepsxhw: () => PS1,
  beetlepsx: () => PS1,
  swanstation: () => PS1,
  beetlepce: () => pce(),
  beetlepcefast: () => pce(),
  beetlesaturn: () => ({ console: 'Saturn', boutons: { B: 'A', A: 'B', Y: 'X', X: 'Y', L: 'Z', R: 'C', L2: 'L', R2: 'R', Start: 'Start', Croix: CROIX } }),
  flycastgles2: () => COEURS.flycast(''),
  flycast: () => ({ console: 'Dreamcast', boutons: { B: 'A', A: 'B', Y: 'X', X: 'Y', L2: 'Gâchette L', R2: 'Gâchette R', Start: 'Start', Croix: CROIX, StickG: 'Stick' } }),
  ppsspp: () => ({ console: 'PSP', boutons: { B: 'Croix', A: 'Rond', Y: 'Carré', X: 'Triangle', L: 'L', R: 'R', Select: 'Select', Start: 'Start', Croix: CROIX, StickG: 'Stick' } }),
  melonds: () => DS('Bruit dans le micro', 'Échanger les écrans', 'Curseur de l’écran tactile'),
  melondsds: () => DS('Vitesse du pointeur', 'Toucher l’écran (curseur)', 'Pointeur'),
  desmume: () => DS('Fermer le couvercle', 'Toucher au stylet'),
  stella: () => ({
    console: 'Atari 2600',
    boutons: { B: 'Feu', Select: 'Select', Start: 'Reset', L: 'Difficulté gauche A', R: 'Difficulté droite A', L2: 'Difficulté gauche B', R2: 'Difficulté droite B', Croix: 'Joystick' },
  }),
  finalburnneo: () => ARCADE('Bouton 6', 'Bouton 5'),
  fbneo: () => ARCADE('Bouton 6', 'Bouton 5'),
  mame: () => ARCADE('Bouton 5', 'Bouton 6'),
};

function pce(): Disposition {
  return {
    console: 'PC Engine',
    boutons: { B: 'II', A: 'I', Y: 'III', X: 'IV', L: 'V', R: 'VI', L2: 'Manette 2 ou 6 boutons', Select: 'Select', Start: 'Run', Croix: CROIX },
    remarque: 'III à VI : seulement sur une manette à 6 boutons (LT pour la choisir).',
  };
}

const PS_FROGTEND: Record<string, string> = {
  a: 'Croix', b: 'Rond', x: 'Carré', y: 'Triangle', lb: 'L1', rb: 'R1', lt: 'L2', rt: 'R2', select: 'Select', start: 'Start', l3: 'L3', r3: 'R3', croix: CROIX, stickg: 'Stick gauche', stickd: 'Stick droit',
};

/** Les émulateurs à part, avec le profil que Frogtend écrit pour le joueur 1 (bouton du joueur → bouton d'origine). */
const EMULATEURS: Record<string, (plateforme: string) => { console: string; boutons: Record<string, string> } | null> = {
  duckstation: () => ({ console: 'PlayStation', boutons: PS_FROGTEND }),
  pcsx2: () => ({ console: 'PlayStation 2', boutons: PS_FROGTEND }),
  // Dolphin : seulement la GameCube (la Wii a ses propres références de Wiimote).
  dolphin: (plateforme) =>
    /gamecube/i.test(plateforme)
      ? { console: 'GameCube', boutons: { a: 'A', x: 'B', b: 'X', y: 'Y', rb: 'Z', lt: 'L', rt: 'R', start: 'Start', croix: CROIX, stickg: 'Stick', stickd: 'Stick C' } }
      : null,
};

/** Ce que le cœur dit du jeu (`boutons_du_jeu`). */
export interface InfosBoutons {
  plateforme: string;
  emulateur: string | null;
  coeur: string | null;
  famille: Famille;
  /** La disposition a été changée à la main (remappage RetroArch, profil de la personne) : rien n'est dessiné. */
  personnalise: boolean;
  /** Jeu PC : sa correspondance manette → clavier (bouton → touche). */
  clavier: Record<string, string> | null;
  /** Les raccourcis de Frogtend pendant une partie (réglages de ce PC). */
  raccourcis: { combinaison: string; bouton_xbox: boolean; touche: string; etats: boolean };
}

/** Une combinaison du réglage (« Select + R1 ») avec les noms de la manette du joueur (« Vue (⧉) + RB »). */
export function nomCombinaison(c: string, famille: Famille): string {
  const n = NOMS[famille] ?? NOMS.xbox;
  const parBouton: Record<string, string> = { Select: n.select, Start: n.start, R1: n.rb, L1: n.lb, L3: 'L3', R3: 'R3' };
  return c
    .split('+')
    .map((x) => parBouton[x.trim()] ?? x.trim())
    .join(' + ');
}

/** Le nom d'une touche du réglage, pour la personne (« Pause » → « Pause/Attn »). */
function nomTouche(t: string): string {
  return ({ Pause: 'Pause/Attn', ScrollLock: 'Arrêt défil', 'Ctrl+Shift+M': 'Ctrl + Maj + M' } as Record<string, string>)[t] ?? t;
}

/** Les raccourcis de Frogtend à montrer : (ce qu'on fait, ce que ça fait). */
export function raccourcis(infos: InfosBoutons): { geste: string; effet: string }[] {
  const r = infos.raccourcis;
  const n = NOMS[infos.famille] ?? NOMS.xbox;
  const l: { geste: string; effet: string }[] = [];
  if (r.bouton_xbox) l.push({ geste: `Bouton du milieu (${infos.famille === 'playstation' ? 'PS' : 'Xbox'}), appui court`, effet: 'ouvrir ou fermer le menu de Frogtend' });
  l.push({ geste: `${nomCombinaison(r.combinaison, infos.famille)}, tenus 1 seconde`, effet: 'ouvrir le menu de Frogtend' });
  if (r.touche) l.push({ geste: `Touche ${nomTouche(r.touche)} du clavier`, effet: 'ouvrir le menu de Frogtend' });
  if (r.etats) {
    l.push({ geste: `${n.select} + ${n.lt}, tenus ½ seconde`, effet: 'sauvegarde rapide' });
    l.push({ geste: `${n.select} + ${n.rt}, tenus 1 seconde`, effet: 'chargement rapide (la partie revient à la sauvegarde)' });
  }
  return l;
}

/** Une ligne montrée : le bouton (sa position), son nom sur la manette du joueur, et ce qu'il fait. */
export interface Ligne {
  id: IdBouton;
  nom: string;
  role: string;
}

export type Schema =
  | { etat: 'connu'; console: string; lignes: Ligne[]; remarque?: string }
  | { etat: 'personnalise' | 'inconnu' | 'jeu-pc'; console: string | null; lignes: [] };

const cle = (s: string) => s.toLowerCase().replace(/[^a-z0-9]/g, '');

/** Le cœur RetroArch reconnu, par son nom exact seulement. */
function dispositionDuCoeur(coeur: string, plateforme: string): Disposition | null {
  const d = COEURS[cle(coeur)];
  return d ? d(plateforme) : null;
}

/** Le schéma à montrer pour un jeu. */
export function schema(infos: InfosBoutons): Schema {
  const noms = NOMS[infos.famille] ?? NOMS.xbox;
  const lignes = (role: (id: IdBouton) => string | undefined): Ligne[] =>
    ORDRE.flatMap((id) => {
      const r = role(id);
      return r ? [{ id, nom: noms[id], role: r }] : [];
    });
  // « Personnalisé » ne compte qu'une fois la disposition connue (sinon : « inconnu », la seule phrase juste).
  const connu = (console: string, role: (id: IdBouton) => string | undefined, remarque?: string): Schema =>
    infos.personnalise ? { etat: 'personnalise', console: null, lignes: [] } : { etat: 'connu', console, lignes: lignes(role), remarque };
  if (infos.emulateur === null) {
    const c = infos.clavier;
    if (!c || !Object.values(c).some(Boolean)) return { etat: 'jeu-pc', console: null, lignes: [] };
    // La croix : les quatre touches des directions (« ↑ ↓ ← → », ZQSD…).
    const directions = ['haut', 'bas', 'gauche', 'droite'].map((d) => c[d]).filter(Boolean);
    const touche = (id: IdBouton) =>
      id === 'croix' ? (directions.length ? `touches ${directions.join(' ')}` : undefined) : id === 'stickg' || id === 'stickd' ? undefined : c[id] ? `touche ${c[id]}` : undefined;
    return { etat: 'connu', console: 'le clavier', lignes: lignes(touche) };
  }
  if (infos.emulateur === 'retroarch' && infos.coeur) {
    const d = dispositionDuCoeur(infos.coeur, infos.plateforme);
    if (!d) return { etat: 'inconnu', console: null, lignes: [] };
    return connu(d.console, (id) => d.boutons[VERS_RETROPAD[id]], d.remarque);
  }
  const e = EMULATEURS[infos.emulateur]?.(infos.plateforme);
  if (e) return connu(e.console, (id) => e.boutons[id]);
  return { etat: 'inconnu', console: null, lignes: [] };
}

/** La phrase d'un schéma sans dessin. */
export function sansDessin(s: Schema): string | null {
  if (s.etat === 'personnalise') return 'Tu as changé toi-même les boutons de ce jeu : Frogtend ne peut pas te les montrer.';
  if (s.etat === 'inconnu') return 'Frogtend ne connaît pas encore les boutons de cet émulateur.';
  if (s.etat === 'jeu-pc')
    return 'Ce jeu lit ta manette lui-même : ses boutons sont dans ses options. Pour un jeu fait pour le clavier : ⚙ Gérer le jeu ▸ 🕹 Jouer à la manette.';
  return null;
}
