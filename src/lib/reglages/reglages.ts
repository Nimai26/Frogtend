// Les réglages de Frogtend. Rien en dur : chaque valeur a un défaut ici et se change dans l'interface,
// avec un retour possible au réglage d'origine.
//
// Deux portées :
// - le PC (adresse de Firehouse, emplacements de jeux…) : les disques et le réseau appartiennent à la machine ;
// - le profil (apparence, ludothèque, commandes…) : chaque personne a les siens.

import type { EmulateursSysteme } from '$lib/emulateurs/choix';

export const ADRESSE_FIREHOUSE_PAR_DEFAUT = 'https://jeux.hikari-no-sekai.fr';

export interface ReglagesPc {
  /** Où ranger les jeux sur CE PC : un ou plusieurs dossiers par système, dans l'ordre de préférence. */
  emplacements: {
    /** Pour les systèmes qui n'ont pas les leurs. */
    defaut: string[];
    /** Par système (nom LaunchBox). */
    systemes: Record<string, string[]>;
  };
  /** Les émulateurs de chaque système (nom LaunchBox) : plusieurs possibles, un par défaut. */
  emulateurs: Record<string, EmulateursSysteme>;
  /** L'émulateur par défaut d'un jeu (identifiant du jeu → clé), s'il diffère de celui du système. */
  emulateursJeux: Record<string, string>;
  /** Où Frogtend installe les émulateurs (vide : demandé à la première installation). */
  dossierEmulateurs: string;
  /** La personne a été prévenue de la question du pare-feu Windows pour RetroArch (une fois par PC, décision de Seb
   * du 16/10). */
  avertiPareFeuRetroArch: boolean;
  /** Où Frogtend installe Cheat Engine et les autres outils (règle « pas de pieuvre ») ; vide : demandé la 1re fois. */
  dossierOutils: string;
  /** Où Frogtend copie les parties avant de retirer un jeu (abris) ; vide : demandé quand il le faut. */
  dossierAbris: string;
  /** La place au plus des images gardées pour chaque profil (jaquettes, visuels, consoles), en Go : au-delà, les moins
   * vues partent après une synchronisation (jamais celles des jeux du PC ni un document). */
  plafondImagesGo: number;
  /** Au-delà de cette taille de parties NOUVELLES (en Mo), une sauvegarde demande avant d'envoyer, et une sauvegarde
   * automatique attend que la personne la lance elle-même. */
  seuilSauvegardeMo: number;
  /** La liste MAME (le MAME.xml des métadonnées de LaunchBox) pour l'import « MAME Arcade Full Set » ; vide : demandée. */
  listeMame: string;
  /** Le menu universel en jeu. */
  menuJeu: {
    /** La touche du clavier qui l'ouvre pendant une partie (nom compris par Tauri : « Pause », « ScrollLock »…). */
    touche: string;
    /** La combinaison de la manette qui l'ouvre, tenue 1 s (Seb, 04/10 : « Select + R1 », bien moins courante en jeu
     * que Select + Start). Lue par le cœur au début de chaque partie. */
    manette: string;
    /** Après « Quitter le jeu » : les secondes laissées à l'émulateur pour se fermer seul avant que Frogtend l'arrête
     * (seulement s'il n'a plus de fenêtre). */
    delaiQuitter: number;
  };
  firehouse: {
    adresse: string;
    /** Mode simulé : des exemples, sans connexion à Firehouse (essais, démonstration, travail hors ligne). */
    simule: boolean;
  };
  /** Les réunions de fiches (contrat 1.14, « ancien>nouveau ») dont les réglages ont déjà été copiés : jamais deux fois. */
  reunionsSuivies: string[];
}

/** Comment on joue à un jeu : réglage d'office, clavier et souris, ou un réglage de manette de référence. */
export interface CommandesJeu {
  mode: 'auto' | 'clavier' | 'reference';
  genre?: string;
  reference?: string;
}

export interface ReglagesProfil {
  apparence: {
    /** `null` = le skin choisi dans Firehouse. */
    skin: string | null;
    /** Échelle globale du texte et des tailles (1 = 100 %). */
    echelle: number;
    densite: 'compacte' | 'aeree';
    animations: 'normales' | 'reduites';
    /** Le fond vidéo du skin `firehouse`. */
    fondVideo: boolean;
  };
  /** La sauvegarde du profil chez Firehouse. */
  sauvegarde: {
    auto: 'apres_partie' | 'quotidienne' | 'manuelle';
  };
  ludotheque: {
    /** Largeur d'une jaquette dans la grille, en pixels (à l'échelle 1). */
    tailleJaquette: number;
    /** L'image des jeux dans la grille et dans Taodbox (lot Médias) : jaquette, boîte en relief, cartouche… */
    visuel: 'jaquette' | 'boite_3d' | 'cartouche' | 'ecran_titre' | 'capture';
    /** La ligne sous le titre d'une carte. */
    sousTitre: 'developpeur' | 'editeur' | 'annee' | 'plateforme' | 'rien';
    tri: 'titre' | 'annee' | 'annee_desc';
    panneauPlateformes: boolean;
    panneauDetails: boolean;
    /** Plateformes masquées dans la liste de gauche. */
    plateformesMasquees: string[];
    /** Taodbox : le fond du jeu choisi derrière la grille et son logo au-dessus (lot Médias, M3). */
    fondsTaodbox: boolean;
  };
  /** Les jeux offerts des boutiques (lot 9). */
  gratuits: {
    /** Les obtenir tout seul (vérification une fois par jour, à l'ouverture du profil). */
    auto: boolean;
    /** Dernière vérification automatique (secondes depuis 1970). */
    derniere: number;
    /** PS Plus (optionnel, demande de Seb) : ajouter tout seul les jeux du mois à mon compte PlayStation. */
    psplus: boolean;
    /** Dernier passage automatique sur PS Plus (secondes depuis 1970). */
    psplusDernier: number;
    /** GOG (jeu offert du moment) : le récupérer tout seul, une fois par jour. */
    gog: boolean;
    gogDernier: number;
    /** Prime Gaming : récupérer tout seul les jeux offerts aux membres Prime, une fois par semaine. */
    prime: boolean;
    primeDernier: number;
  };
  /** Les commandes choisies jeu par jeu (identifiant du jeu → choix). Absent : automatique. */
  commandes: Record<string, CommandesJeu>;
  /** Les jeux PC figés pendant le menu en jeu (identifiants ; Seb, 05/10 : option par jeu, désactivée d'office). */
  figer: string[];
  /** Le plein écran au lancement, jeu par jeu (identifiant → `fenetre` ou `touche:F4`) ; absent : rien. */
  pleinEcran: Record<string, string>;
  /** Manette → clavier, jeu par jeu (identifiant → bouton → touche) ; absent : le jeu lit la manette lui-même. */
  clavier: Record<string, Record<string, string>>;
  /** Manette → clavier : les jeux où une touche tenue ne se répète PAS (identifiants ; répétition d'office, 19/10). */
  sansRepetition: string[];
  /** Les réunions de fiches (contrat 1.14, « ancien>nouveau ») dont les réglages ont déjà été copiés : jamais deux fois. */
  reunionsSuivies: string[];
}

export const DEFAUTS_PC: ReglagesPc = {
  emplacements: { defaut: [], systemes: {} },
  emulateurs: {},
  emulateursJeux: {},
  dossierEmulateurs: '',
  avertiPareFeuRetroArch: false,
  dossierOutils: '',
  dossierAbris: '',
  plafondImagesGo: 2,
  seuilSauvegardeMo: 200,
  listeMame: '',
  menuJeu: { touche: 'Pause', manette: 'Select + R1', delaiQuitter: 10 },
  firehouse: { adresse: ADRESSE_FIREHOUSE_PAR_DEFAUT, simule: false },
  reunionsSuivies: [],
};

export const DEFAUTS_PROFIL: ReglagesProfil = {
  apparence: { skin: null, echelle: 1, densite: 'aeree', animations: 'normales', fondVideo: true },
  sauvegarde: { auto: 'apres_partie' },
  ludotheque: {
    tailleJaquette: 170,
    visuel: 'jaquette',
    sousTitre: 'developpeur',
    tri: 'titre',
    panneauPlateformes: true,
    panneauDetails: true,
    plateformesMasquees: [],
    fondsTaodbox: true,
  },
  gratuits: { auto: false, derniere: 0, psplus: false, psplusDernier: 0, gog: false, gogDernier: 0, prime: false, primeDernier: 0 },
  commandes: {},
  figer: [],
  pleinEcran: {},
  clavier: {},
  sansRepetition: [],
  reunionsSuivies: [],
};

export const TAILLE_JAQUETTE_MIN = 110;
export const TAILLE_JAQUETTE_MAX = 320;

export const ECHELLE_MIN = 0.75;
export const ECHELLE_MAX = 2.5;

type Objet = Record<string, unknown>;
const estObjet = (x: unknown): x is Objet => typeof x === 'object' && x !== null && !Array.isArray(x);

/**
 * Complète des réglages lus sur le disque avec les défauts : une clé absente, ou d'un autre type que le défaut,
 * prend la valeur par défaut. Les clés inconnues sont ignorées. (Un fichier d'une ancienne version reste lisible.)
 */
export function completer<T>(defauts: T, lus: unknown): T {
  if (Array.isArray(defauts)) return (Array.isArray(lus) ? lus : defauts) as T;
  // Un objet VIDE par défaut est un dictionnaire libre (ex. emplacements par système) : on garde ce qui est lu.
  if (estObjet(defauts) && Object.keys(defauts).length === 0) return (estObjet(lus) ? lus : defauts) as T;
  if (!estObjet(defauts)) {
    // Un défaut `null` est un texte facultatif (le skin, par exemple).
    if (defauts === null) return (typeof lus === 'string' ? lus : null) as T;
    return (typeof lus === typeof defauts ? lus : defauts) as T;
  }
  const source = estObjet(lus) ? lus : {};
  const resultat: Objet = {};
  for (const [cle, defaut] of Object.entries(defauts)) {
    resultat[cle] = completer(defaut, source[cle]);
  }
  return resultat as T;
}

/** Lit une valeur par son chemin : `lire(r, 'apparence.skin')`. */
export function lire(reglages: unknown, chemin: string): unknown {
  return chemin.split('.').reduce<unknown>((o, k) => (estObjet(o) ? o[k] : undefined), reglages);
}

/** Rend une copie où la valeur du chemin est remplacée. */
export function ecrire<T>(reglages: T, chemin: string, valeur: unknown): T {
  const [tete, ...reste] = chemin.split('.');
  const o = (estObjet(reglages) ? reglages : {}) as Objet;
  return { ...o, [tete]: reste.length === 0 ? valeur : ecrire(o[tete], reste.join('.'), valeur) } as T;
}

/** Remet un chemin (ou tout, sans chemin) à sa valeur d'origine. */
export function reinitialiser<T>(reglages: T, defauts: T, chemin?: string): T {
  if (!chemin) return structuredClone(defauts);
  return ecrire(reglages, chemin, structuredClone(lire(defauts, chemin)));
}

/**
 * Vrai pour une machine du réseau local : 10/8, 172.16/12, 192.168/16, 127/8, `localhost`, un nom en `.local`.
 * Frogtend est aussi installé hors de la maison : tout le reste passe par Internet.
 */
export function estAdresseLocale(hote: string): boolean {
  const h = hote.toLowerCase().replace(/^\[|\]$/g, '');
  if (h === 'localhost' || h === '::1' || h.endsWith('.local')) return true;
  const ip = /^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})$/.exec(h);
  if (!ip) return false;
  const [a, b] = [Number(ip[1]), Number(ip[2])];
  return a === 10 || a === 127 || (a === 172 && b >= 16 && b <= 31) || (a === 192 && b === 168);
}

/**
 * Une adresse de Firehouse acceptable. HTTPS obligatoire hors du réseau local : le HTTP clair n'est permis que
 * vers une machine de la maison (le jeton circulerait sinon en clair sur Internet).
 * Rend l'adresse nettoyée, ou un motif de refus.
 */
export function verifierAdresse(texte: string): { adresse: string } | { refus: string } {
  const t = texte.trim().replace(/\/+$/, '');
  let url: URL;
  try {
    url = new URL(t);
  } catch {
    return { refus: `ce n’est pas une adresse web (exemple : ${ADRESSE_FIREHOUSE_PAR_DEFAUT})` };
  }
  if (url.protocol !== 'https:' && url.protocol !== 'http:') {
    return { refus: 'l’adresse doit commencer par https:// (ou http:// pour une machine de la maison)' };
  }
  if (url.protocol === 'http:' && !estAdresseLocale(url.hostname)) {
    return { refus: 'hors du réseau de la maison, l’adresse doit être en https:// (sinon ton jeton passerait en clair)' };
  }
  return { adresse: t };
}
