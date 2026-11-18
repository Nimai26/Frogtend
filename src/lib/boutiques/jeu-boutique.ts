// Un jeu de boutique dans la ludothèque (lot 9, id négatif) : jouer ou installer passe par SA boutique — Steam, Epic,
// GOG, Ubisoft Connect, Amazon Games, Xbox (import automatique, lot A du 18/11), ou GOG Galaxy pour les jeux qu'il relie.
import { api, type JeuResume } from '$lib/api';

/** Les boutiques proposées dans le filtre de la ludothèque. */
export const BOUTIQUES: { valeur: string; libelle: string }[] = [
  { valeur: 'firehouse', libelle: 'Firehouse' },
  { valeur: 'steam', libelle: 'Steam' },
  { valeur: 'gog', libelle: 'GOG' },
  { valeur: 'epic', libelle: 'Epic Games' },
  { valeur: 'xboxone', libelle: 'Xbox' },
  { valeur: 'uplay', libelle: 'Ubisoft Connect' },
  { valeur: 'origin', libelle: 'EA' },
  { valeur: 'amazon', libelle: 'Amazon Games' },
  { valeur: 'battlenet', libelle: 'Battle.net' },
  { valeur: 'local', libelle: 'Importés de mon disque' },
];

/** Le nom d'une boutique pour les personnes (« Ubisoft Connect », « Epic Games »…). */
export function nomBoutique(b: string | null | undefined): string {
  return BOUTIQUES.find((x) => x.valeur === b && b !== 'firehouse' && b !== 'local')?.libelle ?? (b === 'xbox' ? 'Xbox' : 'sa boutique');
}

/** Un jeu de boutique (Steam, Galaxy) ; un jeu IMPORTÉ du disque (id négatif aussi) se joue comme un jeu du PC. */
export const estJeuDeBoutique = (j: Pick<JeuResume, 'id' | 'source'>) => j.id < 0 && j.source !== 'local';

/** Comment ouvrir ce jeu : par Steam (appid), par GOG Galaxy (sa clé), ou par le lanceur de sa boutique (clé de la
 * carte, « uplay_46 »…). */
export type MoyenOuverture = { par: 'steam'; appid: string } | { par: 'galaxy'; cle: string } | { par: 'boutique'; boutique: string; cle: string };

export function moyenDOuverture(j: JeuResume): MoyenOuverture | null {
  const cle = j.cle_boutique ?? '';
  if (!cle) return null;
  if (j.source === 'steam') return { par: 'steam', appid: cle };
  if (j.source === 'galaxy') return { par: 'galaxy', cle };
  if (j.source && j.boutique) return { par: 'boutique', boutique: j.boutique, cle };
  return null;
}

/** Le libellé du lanceur qui ouvrira ce jeu, tiré de sa boutique (« Ubisoft Connect », « GOG Galaxy »…). */
export function libelleOuverture(j: JeuResume): string {
  const m = moyenDOuverture(j);
  if (!m) return nomBoutique(j.boutique);
  if (m.par === 'galaxy') return 'GOG Galaxy';
  if (m.par === 'steam') return 'Steam';
  return nomBoutique(m.boutique);
}

/** Jouer (installé) ou installer, par la boutique. */
export async function ouvrirJeuDeBoutique(j: JeuResume) {
  const m = moyenDOuverture(j);
  if (!m) return;
  const action = j.installe ? 'jouer' : 'installer';
  if (m.par === 'steam') await api.steamOuvrir(m.appid, action);
  else if (m.par === 'galaxy') await api.galaxyOuvrir(m.cle);
  else await api.boutiqueOuvrir(m.boutique, m.cle, action);
}

/** Le toast regroupé des jeux NOUVEAUX trouvés sur ce PC (« 🎮 3 nouveaux jeux trouvés sur ce PC (Epic 2, Ubisoft
 * 1) ») ; `null` s'il n'y en a aucun. */
export function messageNouveaux(nouveaux: [string, number][]): string | null {
  const l = nouveaux.filter(([, n]) => n > 0);
  const total = l.reduce((s, [, n]) => s + n, 0);
  if (!total) return null;
  const court = (b: string) => ({ epic: 'Epic', uplay: 'Ubisoft', amazon: 'Amazon', xboxone: 'Xbox' } as Record<string, string>)[b] ?? nomBoutique(b);
  const detail = l.map(([b, n]) => `${court(b)} ${n}`).join(', ');
  return total === 1 ? `🎮 1 nouveau jeu trouvé sur ce PC (${detail})` : `🎮 ${total} nouveaux jeux trouvés sur ce PC (${detail})`;
}

/** La ligne d'état d'une boutique sur ce PC (page 🛒 Boutiques). */
export function ligneEtat(e: { nom: string; lanceur: boolean; lu: boolean; installes: number }): string {
  if (!e.lu) return e.lanceur ? `${e.nom} : jeux illisibles pour l’instant` : `${e.nom} : lanceur absent`;
  const jeux = e.installes === 0 ? 'aucun jeu installé' : e.installes === 1 ? '1 jeu sur ce PC' : `${e.installes} jeux sur ce PC`;
  return `${e.nom} : ${jeux}, relus au démarrage${e.lanceur ? '' : ' (lanceur absent : pour jouer, réinstalle-le)'}`;
}

/** Le profil est-il toujours celui d'avant une attente ? (Sinon : rien n'est fait, ni écrit, ni montré.) */
export function toujoursLeMeme(avant: string | null | undefined, maintenant: string | null | undefined): boolean {
  return !!avant && avant === maintenant;
}

/** Un jeu Battle.net : Frogtend ne sait pas le lancer (aucune adresse prouvée), lanceur présent ou non. Carte de la
 * ludothèque (`boutique`) ou jeu du PC (`installation.boutique.source`). */
export function estBattlenet(j: { boutique?: string | null; source?: string | null; installation?: { boutique?: { source: string } | null } | null }): boolean {
  return (j.boutique === 'battlenet' && j.source !== 'galaxy') || j.installation?.boutique?.source === 'battlenet';
}

/** Le message pour un jeu Battle.net (au bureau et dans Taodbox). */
export const MESSAGE_BATTLENET = 'Frogtend ne sait pas encore lancer un jeu Battle.net : ouvre-le depuis Battle.net.';

/** Une ligne du PC d'un jeu de boutique désinstallé hors de Frogtend (gardée pour son temps de jeu, sans installation) :
 * jamais montrée jouable dans Taodbox. */
export function boutiqueDesinstallee(j: { id: number; installation?: unknown; ajoute_par?: string }): boolean {
  return j.id < 0 && !j.installation && ['steam', 'epic', 'gog', 'uplay', 'amazon', 'xboxone', 'battlenet'].includes(j.ajoute_par ?? '');
}

/** LE toast du lancement d'un jeu de boutique (un seul, lot A) : au canapé, il dit aussi comment revenir à Frogtend
 * si le lanceur prend la main. */
export function messageLancementBoutique(nom: string, titre: string, auCanape: boolean): string {
  const base = `▶ ${nom} lance « ${titre} »… S’il fait une mise à jour, ça peut prendre quelques minutes.`;
  return auCanape ? `${base} S’il demande la souris, appuie sur ⊞/Guide pour revenir à Frogtend.` : base;
}
