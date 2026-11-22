// Un jeu de boutique dans la ludothèque (lot 9, id négatif) : jouer ou installer passe par SA boutique — Steam, Epic,
// GOG, Ubisoft Connect, Amazon Games, Xbox (import automatique, lot A du 18/11), ou GOG Galaxy pour les jeux qu'il relie.
import { adresseJaquette, api, type JeuResume } from '$lib/api';
import { confirmerInstallationGog } from './installation-gog';

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

/** Les succès d'une carte que GOG Galaxy connaît (pur, testé) : une carte Galaxy, ou une carte GOG du compte ou du PC
 * (lot C4a : elle passe avant Galaxy et reprend ses succès) ; le détail se lit toujours chez Galaxy, sous la clé de la
 * carte (« gog_123 »). `null` : pas de succès Galaxy à montrer. */
export function succesGalaxy(j: Pick<JeuResume, 'source' | 'succes' | 'cle_boutique'>): { obtenus: number; total: number; cle: string | null } | null {
  if ((j.source !== 'galaxy' && j.source !== 'gog') || !j.succes || !j.succes[1]) return null;
  return { obtenus: j.succes[0], total: j.succes[1], cle: j.cle_boutique || null };
}

/** Jouer (installé) ou installer, par la boutique. */
export async function ouvrirJeuDeBoutique(j: JeuResume) {
  const m = moyenDOuverture(j);
  if (!m) return;
  const action = j.installe ? 'jouer' : 'installer';
  if (m.par === 'steam') await api.steamOuvrir(m.appid, action);
  else if (m.par === 'galaxy') await api.galaxyOuvrir(m.cle);
  else {
    // Lot C4b : un jeu d'un autre compte GOG que celui ouvert dans Galaxy : averti avant (jamais interdit).
    if (m.boutique === 'gog' && action === 'installer' && !(await confirmerInstallationGog(m.cle))) return;
    await api.boutiqueOuvrir(m.boutique, m.cle, action);
  }
}

/** Le nom court d'une boutique dans un message (« Epic », « Ubisoft »…). */
function nomCourt(b: string): string {
  return ({ epic: 'Epic', uplay: 'Ubisoft', amazon: 'Amazon', xboxone: 'Xbox' } as Record<string, string>)[b] ?? nomBoutique(b);
}

/** « Epic », « Epic et Ubisoft », « Epic, GOG et Ubisoft ». */
function enumerer(noms: string[]): string {
  return noms.length <= 1 ? (noms[0] ?? '') : `${noms.slice(0, -1).join(', ')} et ${noms[noms.length - 1]}`;
}

/** Le toast regroupé des jeux NOUVEAUX dans la ludothèque ; `null` s'il n'y en a aucun. Chaque entrée : (boutique,
 * nouveaux, dont installés). Seulement des jeux installés (lot A) : « 🎮 3 nouveaux jeux trouvés sur ce PC (Epic 2,
 * Ubisoft 1) ». Avec des jeux des comptes (lot B), des mots pour un novice : « 🎮 498 jeux de tes comptes Epic et
 * Ubisoft ajoutés à ta ludothèque (1 déjà installé). Les autres s'installent avec ⬇ Installer. » */
export function messageNouveaux(nouveaux: [string, number, number?][]): string | null {
  const l = nouveaux.filter(([, n]) => n > 0).map(([b, n, i]) => ({ b, n, i: Math.min(n, Math.max(0, i ?? n)) }));
  const total = l.reduce((s, x) => s + x.n, 0);
  if (!total) return null;
  const installes = l.reduce((s, x) => s + x.i, 0);
  if (installes === total) {
    const detail = l.map((x) => `${nomCourt(x.b)} ${x.n}`).join(', ');
    return total === 1 ? `🎮 1 nouveau jeu trouvé sur ce PC (${detail})` : `🎮 ${total} nouveaux jeux trouvés sur ce PC (${detail})`;
  }
  const comptes = enumerer(l.filter((x) => x.i < x.n).map((x) => nomCourt(x.b)));
  // « de ton compte Steam » quand une seule boutique est concernée, « de tes comptes Epic et Ubisoft » sinon.
  const nbComptes = l.filter((x) => x.i < x.n).length;
  const de = nbComptes === 1 ? `de ton compte ${comptes}` : `de tes comptes ${comptes}`;
  const debut = total === 1 ? `🎮 1 jeu ${de} ajouté à ta ludothèque` : `🎮 ${total} jeux ${de} ajoutés à ta ludothèque`;
  if (installes === 0) return `${debut}. ${total === 1 ? 'Il s’installe' : 'Ils s’installent'} avec ⬇ Installer.`;
  return `${debut} (${installes} déjà installé${installes > 1 ? 's' : ''}). Les autres s’installent avec ⬇ Installer.`;
}

/** La ligne d'état d'une boutique sur ce PC (page 🛒 Boutiques), sans mot technique. */
export function ligneEtat(e: { nom: string; lanceur: boolean; lu: boolean; installes: number; possedes?: number | null }): string {
  if (!e.lu) return e.lanceur ? `${e.nom} : jeux illisibles pour l’instant` : `${e.nom} n’est pas installé sur ce PC`;
  const possedes = typeof e.possedes === 'number' ? e.possedes : null;
  const installes = e.installes === 0 ? 'aucun' : String(e.installes);
  const jeux =
    possedes !== null
      ? // Lot B : « Epic Games : 500 jeux dans ton compte, dont 1 installé ».
        `${possedes === 1 ? '1 jeu' : `${possedes} jeux`} dans ton compte, dont ${installes} installé${e.installes > 1 ? 's' : ''}`
      : e.installes === 0
        ? 'aucun jeu installé'
        : e.installes === 1
          ? '1 jeu sur ce PC'
          : `${e.installes} jeux sur ce PC`;
  return `${e.nom} : ${jeux}, mis à jour à chaque ouverture${e.lanceur ? '' : ` (${e.nom} n’est plus installé sur ce PC : pour jouer, réinstalle-le)`}`;
}

/** La question posée UNE fois par profil (✅ Seb 18/11) avant d'ajouter les jeux possédés des comptes ouverts sur ce
 * PC : « Ajouter les 500 jeux du compte Epic Games connecté sur ce PC (et 4 d'Ubisoft Connect) à ta ludothèque ? » ;
 * `null` s'il n'y a aucun jeu possédé. */
export function questionPossedes(etats: { nom: string; possedes?: number | null; installes: number }[]): { titre: string; message: string } | null {
  const l = etats.filter((e) => typeof e.possedes === 'number' && e.possedes > 0) as { nom: string; possedes: number }[];
  if (!l.length) return null;
  const jeux = (n: number) => (n === 1 ? 'le jeu' : `les ${n} jeux`);
  const de = (nom: string) => (/^[aeiouyéèêàâîïôûAEIOUYÉÈÊÀÂÎÏÔÛ]/.test(nom) ? `d’${nom}` : `de ${nom}`);
  const [premier, ...autres] = l;
  const suite = autres.length ? ` (et ${autres.map((e) => `${e.possedes} ${de(e.nom)}`).join(', ')})` : '';
  return {
    titre: `Ajouter ${jeux(premier.possedes)} du compte ${premier.nom} connecté sur ce PC${suite} à ta ludothèque ?`,
    message: 'Ils viennent du compte de la boutique ouvert sur ce PC. Ils seront marqués « à installer » ; tu pourras changer d’avis dans ⚙ Options ▸ Comptes ▸ 🛒 Mes jeux installés.',
  };
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

/** Une ligne du PC d'un jeu de boutique SANS installation (désinstallé hors de Frogtend, gardé pour son temps de jeu) :
 * jamais une carte jouable dans Taodbox. Sa carte « à installer » vient de la ludothèque, comme celle d'un jeu possédé
 * jamais installé (`cartesAInstallerTaodbox`, lot B). */
export function boutiqueSansInstallation(j: { id: number; installation?: unknown; ajoute_par?: string }): boolean {
  return j.id < 0 && !j.installation && ['steam', 'epic', 'gog', 'uplay', 'amazon', 'xboxone', 'battlenet'].includes(j.ajoute_par ?? '');
}

/** La jaquette d'une carte « à installer » de Taodbox : RÉDUITE par le cœur à 400 px (et gardée réduite), jamais
 * l'image d'origine de la boutique (500 boîtes Epic de 1 200 × 1 600) ; `null` sans image. Steam (lot C2) : sa carte n'a
 * pas d'adresse d'image, le cœur réduit la jaquette de son appid de la même façon. */
export function imageCarteAInstaller(c: { id: number; image: string | null; boutique?: string }): string | null {
  return c.image || c.boutique === 'steam' ? adresseJaquette(c.id, 400) : null;
}

/** Le message quand Taodbox ouvre l'installation d'un jeu par sa boutique (lot B, ✅ Seb 18/11). */
export function messageInstallationCanape(boutique: string): string {
  const nom = nomBoutique(boutique);
  const de = /^[aeiouyéèêàâîïôûAEIOUYÉÈÊÀÂÎÏÔÛ]/.test(nom) ? `d’${nom}` : `de ${nom}`;
  return `La fenêtre d’installation ${de} s’ouvre : valide-la (elle peut demander la souris ; ⊞/Guide pour revenir à Frogtend).`;
}

/** LE toast du lancement d'un jeu de boutique (un seul, lot A) : au canapé, il dit aussi comment revenir à Frogtend
 * si le lanceur prend la main. */
export function messageLancementBoutique(nom: string, titre: string, auCanape: boolean): string {
  const base = `▶ ${nom} lance « ${titre} »… S’il fait une mise à jour, ça peut prendre quelques minutes.`;
  return auCanape ? `${base} S’il demande la souris, appuie sur ⊞/Guide pour revenir à Frogtend.` : base;
}
