// Les réglages de Frogtend. Rien en dur : chaque valeur a un défaut ici et se change dans l'interface,
// avec un retour possible au réglage d'origine.
//
// Deux portées :
// - le PC (adresse de Firehouse, emplacements de jeux…) : les disques et le réseau appartiennent à la machine ;
// - le profil (apparence, ludothèque, commandes…) : chaque personne a les siens.

export const ADRESSE_FIREHOUSE_PAR_DEFAUT = 'https://core.hikari-no-sekai.fr';

export interface ReglagesPc {
  firehouse: {
    adresse: string;
    /** Tant que l'API `/api/jeux/v1/` n'existe pas, Frogtend travaille sur des réponses simulées. */
    simule: boolean;
  };
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
}

export const DEFAUTS_PC: ReglagesPc = {
  firehouse: { adresse: ADRESSE_FIREHOUSE_PAR_DEFAUT, simule: true },
};

export const DEFAUTS_PROFIL: ReglagesProfil = {
  apparence: { skin: null, echelle: 1, densite: 'aeree', animations: 'normales', fondVideo: true },
};

export const ECHELLE_MIN = 0.75;
export const ECHELLE_MAX = 2.5;

type Objet = Record<string, unknown>;
const estObjet = (x: unknown): x is Objet => typeof x === 'object' && x !== null && !Array.isArray(x);

/**
 * Complète des réglages lus sur le disque avec les défauts : une clé absente, ou d'un autre type que le défaut,
 * prend la valeur par défaut. Les clés inconnues sont ignorées. (Un fichier d'une ancienne version reste lisible.)
 */
export function completer<T>(defauts: T, lus: unknown): T {
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

/** Une adresse de Firehouse acceptable : http(s), sans chemin superflu. Rend l'adresse nettoyée, ou un motif de refus. */
export function verifierAdresse(texte: string): { adresse: string } | { refus: string } {
  const t = texte.trim().replace(/\/+$/, '');
  let url: URL;
  try {
    url = new URL(t);
  } catch {
    return { refus: 'ce n’est pas une adresse web (exemple : https://core.hikari-no-sekai.fr)' };
  }
  if (url.protocol !== 'https:' && url.protocol !== 'http:') {
    return { refus: 'l’adresse doit commencer par https:// ou http://' };
  }
  return { adresse: t };
}
