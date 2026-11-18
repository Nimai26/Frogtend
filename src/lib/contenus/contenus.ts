// Les contenus additionnels d'un jeu (DLC, avatars, thèmes, mises à jour) — règle de Seb (03/10) : détectés, gérés,
// proposés, et installés seulement si la personne qui utilise Frogtend dit oui, sur SON PC.

export interface ContenuVu {
  id: string;
  nom: string;
  genre: 'dlc' | 'avatar' | 'theme' | 'maj' | 'autre' | string;
  taille: number;
  licence: boolean;
  installe: boolean;
  /** Une mise à jour PS3 : sa version (« 01.05 », contrat 1.31), dans l'ordre d'installation. */
  version?: string | null;
  /** Ryubing et Eden : pourquoi ce contenu trouvé n'est pas utilisé, quand la cause est TECHNIQUE (dite par le cœur). */
  raison?: string | null;
  /** Switch (Seb 17/11) : la clé de son interrupteur (« maj » ou l'identifiant du DLC) ; absente : pas d'interrupteur
   * (une vieille mise à jour, un contenu d'une autre console). */
  cle?: string | null;
  /** Switch : le choix de la maison (allumé d'office). */
  actif?: boolean;
}

/** Les mises à jour s'installent DANS L'ORDRE (la chaîne de Sony) : cocher une mise à jour coche aussi les précédentes
 * pas encore installées ; décocher une mise à jour décoche les suivantes. `id` : celle que la personne vient de changer. */
export function cochesCoherentes(contenus: ContenuVu[], coches: string[], id: string): string[] {
  const majs = contenus.filter((c) => c.version);
  const i = majs.findIndex((c) => c.id === id);
  if (i < 0) return coches;
  if (coches.includes(id)) {
    const avant = majs.slice(0, i).filter((c) => !c.installe && !coches.includes(c.id)).map((c) => c.id);
    return [...coches, ...avant];
  }
  const apres = new Set(majs.slice(i + 1).map((c) => c.id));
  return coches.filter((c) => !apres.has(c));
}

export interface ContenusDuJeu {
  etat: 'aucun' | 'emulateur' | 'ok';
  /** `ryubing` et `eden` (Seb 16/11 : « automatique » partout) : donnés TOUT SEULS à chaque lancement — montrés,
   * jamais d'accord demandé. `switch` : aucun émulateur Switch utilisable réglé. */
  systeme?: 'ps3' | 'switch' | 'wiiu' | 'ryubing' | 'eden';
  titre_id: string | null;
  contenus: ContenuVu[];
  /** Les fichiers servis comme contenus mais illisibles (en-tête abîmé) : dits, jamais écartés en silence. */
  ignores?: string[];
  /** Ryubing et Eden : une phrase à dire (clés absentes, choix fait dans l'émulateur gardé…). */
  remarque?: string | null;
}

/** L'émulateur à qui Frogtend donne tout seul les mises à jour et DLC (Ryubing, Eden) ; `null` : choisis par la
 * personne (PS3) ou inclus dans le jeu (Wii U). */
export function emulateurAutomatique(systeme: ContenusDuJeu['systeme']): string | null {
  return systeme === 'ryubing' ? 'Ryubing' : systeme === 'eden' ? 'Eden' : null;
}

/** Switch (Seb 17/11) : les interrupteurs, résumés — « 3 sur 3 utilisés », « 2 utilisés, 1 éteint » ou, avec Eden,
 * « 3 utilisés (dont 1 éteint qu’Eden garde) ». Seuls les contenus qui ont un interrupteur comptent (pas une vieille
 * mise à jour). */
export function resumeChoix(l: ContenuVu[], systeme?: ContenusDuJeu['systeme']): string {
  const avec = l.filter((c) => c.cle);
  if (!avec.length) return 'aucun à choisir';
  const s = (n: number) => (n > 1 ? 's' : '');
  // « Utilisé » = ce que l'émulateur prend VRAIMENT (Eden garde un DLC éteint tant qu'un autre est allumé).
  const garde = (c: ContenuVu) => eteintMaisUtilise(c, systeme, l);
  const eteintsUtilises = avec.filter(garde).length;
  const utilises = avec.filter((c) => (c.actif && c.installe) || garde(c)).length;
  const eteints = avec.filter((c) => !c.actif && !garde(c)).length;
  const enPanne = avec.filter((c) => allumePasUtilise(c)).length;
  if (!eteints && !enPanne && !eteintsUtilises) return `${utilises} sur ${avec.length} utilisé${s(avec.length)}`;
  const morceaux = [
    `${utilises} utilisé${s(utilises)}` + (eteintsUtilises ? ` (dont ${eteintsUtilises} éteint${s(eteintsUtilises)} qu’Eden garde)` : ''),
  ];
  if (eteints) morceaux.push(`${eteints} éteint${s(eteints)}`);
  if (enPanne) morceaux.push(`${enPanne} allumé${s(enPanne)} mais pas utilisé${s(enPanne)}`);
  return morceaux.join(', ');
}

/** Allumé, mais l'émulateur ne le prend pas (une cause technique, dite par le cœur). */
export function allumePasUtilise(c: ContenuVu): boolean {
  return !!c.cle && !!c.actif && !c.installe && !!c.raison;
}

/** Un DLC éteint qu'Eden utilise quand même : il ne sait couper que TOUS les DLC ensemble, donc il les garde tant
 * qu'un seul est allumé (Seb 17/11 : le panneau dit ce qui se passe vraiment). Même règle que `dlcEnPartieEden` :
 * tous éteints, Eden les coupe (jamais « Eden l'utilise » alors). Jamais pour Ryubing, qui coupe un DLC précis. */
export function eteintMaisUtilise(c: ContenuVu, systeme: ContenusDuJeu['systeme'], l: ContenuVu[]): boolean {
  return c.genre === 'dlc' && !!c.cle && !c.actif && dlcEnPartieEden(systeme, l);
}

/** Le texte d'un interrupteur (jamais la couleur seule). */
export function texteInterrupteur(c: ContenuVu, systeme?: ContenusDuJeu['systeme'], l: ContenuVu[] = []): string {
  if (allumePasUtilise(c)) return '⚠ Allumé, pas utilisé';
  if (eteintMaisUtilise(c, systeme, l)) return '⚠ Éteint, mais Eden l’utilise';
  return c.actif ? '✅ Utilisé' : '⬜ Pas utilisé';
}

/** Eden ne sait couper que TOUS les DLC ensemble : des DLC éteints, mais pas tous, restent actifs avec lui. */
export function dlcEnPartieEden(systeme: ContenusDuJeu['systeme'], l: ContenuVu[]): boolean {
  const dlc = l.filter((c) => c.genre === 'dlc' && c.cle);
  return systeme === 'eden' && dlc.some((c) => c.actif) && dlc.some((c) => !c.actif);
}

/** La phrase unique du panneau (Seb 17/11). */
export const PHRASE_CHOIX =
  'Ces choix valent pour ce jeu, avec Eden comme avec Ryubing, pour toute la maison. À chaque partie, Frogtend les remet en place : un changement fait dans l’émulateur ne compte pas.';

/** La remarque Eden quand des DLC sont éteints en partie. */
export const REMARQUE_EDEN = 'Eden ne sait couper que tous les DLC ensemble : avec Eden, ils restent tous actifs tant qu’un seul est allumé.';

/** Pourquoi les interrupteurs attendent : un émulateur ouvert seul (son motif), sinon une partie en cours. */
export function motifInterrupteursBloques(attenteSeul: string | null): string {
  return attenteSeul ?? '⏸ Une partie est en cours : tu pourras changer ces choix quand elle sera finie.';
}

/** Ryubing et Eden : pourquoi un contenu trouvé n'est pas utilisé — la cause dite par le cœur, jamais devinée ici. */
export function raisonNonPose(c: ContenuVu): string {
  return c.raison || 'pas utilisé';
}

/** Des noms à dire : au plus `n`, puis « et N autre(s) » (jamais une liste sans fin). */
export function quelquesNoms(l: string[], n = 5): string {
  const debut = l.slice(0, n).join(', ');
  return l.length > n ? `${debut} et ${l.length - n} autre(s)` : debut;
}

/** Ryubing et Eden : la remarque du cœur, un paragraphe par ligne. */
export function lignesRemarque(r: string | null | undefined): string[] {
  return (r ?? '').split('\n').map((l) => l.trim()).filter(Boolean);
}

const GENRES: Record<string, string> = { dlc: 'DLC', avatar: 'Avatar', theme: 'Thème', maj: 'Mise à jour', licence: 'Licence', autre: 'Contenu' };

export function libelleGenre(g: string): string {
  return GENRES[g] ?? 'Contenu';
}

/** Contrat 1.47 : les noms FRANÇAIS des DLC et mises à jour Switch (Firehouse), par clé d'interrupteur (« maj »,
 * « dlc:… ») et par fichier (`id` du contenu). */
export interface NomsContenus {
  par_cle: Record<string, string>;
  par_id: Record<string, string>;
}

/** Le nom montré d'un contenu : le nom français de Firehouse (ou son nom d'origine) s'il est connu, sinon le nom tiré du
 * fichier (`nomCourt`). */
export function nomMontre(c: Pick<ContenuVu, 'id' | 'nom' | 'cle'>, titreDuJeu: string, noms: NomsContenus | null | undefined): string {
  const connu = noms?.par_id[c.id] ?? (c.cle ? noms?.par_cle[c.cle] : undefined);
  return connu?.trim() ? connu.trim() : nomCourt(c.nom, titreDuJeu);
}

/** Le nom affiché : sans le titre du jeu répété ni les étiquettes de région. */
export function nomCourt(nomComplet: string, titreDuJeu: string): string {
  // L'extension d'un fichier Switch ne dit rien à la personne.
  const nom = nomComplet.replace(/\.(?:nsp|nsz|xci|xcz)$/i, '');
  const sansEtiquettes = nom.replace(/\s*[([][^)\]]*[)\]]/g, '').trim();
  const parties = sansEtiquettes.split(' - ');
  const simple = (t: string) => t.toLowerCase().replace(/[^a-z0-9]/g, '');
  if (parties.length > 1 && simple(titreDuJeu).startsWith(simple(parties[0]))) return parties.slice(1).join(' - ');
  // Une Switch range ce qui distingue un DLC ENTRE CROCHETS (« Fire Emblem Three Houses [Additional Quests][0100…][US]
  // [v196608] ») : sans eux, tous les DLC portaient le titre du jeu (Seb 18/11). Le premier crochet parlant est gardé ;
  // jamais un numéro de titre, une région, une version brute ou une étiquette de site.
  if (!sansEtiquettes || simple(titreDuJeu).startsWith(simple(sansEtiquettes))) {
    // Seulement les étiquettes entre CROCHETS (le style Switch) : jamais une parenthèse (« Jeu (Europe) » d'un DLC PS3 ou
    // Wii U ne devient pas « Europe »).
    const etiquettes = [...nom.matchAll(/\[([^\]]*)\]/g)].map((m) => m[1].trim());
    const parlante = etiquettes.find((e) => /[a-z]/i.test(e) && !ETIQUETTE_TECHNIQUE.test(e));
    if (parlante) return parlante;
    const version = etiquettes.find((e) => /^v?\d+(\.\d+){1,3}$/i.test(e));
    if (version) return `Version ${version.replace(/^v/i, '')}`;
  }
  return sansEtiquettes;
}

/** Une étiquette entre crochets qui ne dit rien à la personne : numéro de titre, région, langues, version brute, site. */
const ETIQUETTES_TECHNIQUES = [/^[0-9a-f]{16}$/i, /^v\d+(?:\.\d+)*$/i, /^[A-Z]{2,3}$/, /^(?:[A-Z][a-z]){2,}$/, /^(?:nsw2u|update|upd|dlc|base|eshop)$/i, /\.(?:com|net|org)\b/i];
const ETIQUETTE_TECHNIQUE = { test: (e: string) => ETIQUETTES_TECHNIQUES.some((r) => r.test(e)) };

/** Ce qu'on dit en résumé : « 7 disponibles, 2 installés ». */
export function resume(l: ContenuVu[]): string {
  const faits = l.filter((c) => c.installe).length;
  return faits ? `${l.length} disponible(s), ${faits} installé(s)` : `${l.length} disponible(s)`;
}
