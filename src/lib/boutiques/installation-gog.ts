// Lot C4b (revue des experts) : avant d'installer un jeu GOG par GOG Galaxy, Frogtend relit à l'instant le compte
// ouvert dans Galaxy. Si le jeu n'est que sur un AUTRE compte GOG du profil, il le DIT dans une fenêtre maison, sans
// interdire : « Ouvrir quand même dans GOG Galaxy » ou « Annuler ». Au moindre doute, rien n'est dit.
import { api, taille, type BilanInstallationGog, type PreparationGog } from '$lib/api';
import { confirmer } from '$lib/dialogues/fenetres.svelte';
import { dureeEstimee } from '$lib/contenus/mods';

/** Le titre et le bouton de l'avertissement (purs). */
export const TITRE_AVERTISSEMENT_GOG = 'Installer avec GOG Galaxy ?';
export const BOUTON_AVERTISSEMENT_GOG = 'Ouvrir quand même dans GOG Galaxy';

/** Rend vrai pour continuer : rien à dire (ou le cœur ne répond pas), ou la personne veut ouvrir Galaxy quand même. */
export async function confirmerInstallationGog(cle: string): Promise<boolean> {
  const m = await api.gogAvertissementInstallation(cle).catch(() => null);
  if (!m) return true;
  return confirmer(TITRE_AVERTISSEMENT_GOG, { message: m, libelleValider: BOUTON_AVERTISSEMENT_GOG });
}

// ───── Lot C4c-2 : installer un jeu GOG SANS GOG Galaxy (installeur hors ligne officiel), fonctions pures ─────

/** La marge des tailles de GOG (arrondies), comme le cœur (`gog_installeur::MARGE_ARRONDI_POURCENT`). */
export const MARGE_ARRONDI_POURCENT = 2;

/** La place à demander (comme le cœur, `place_necessaire`) : l'installeur ET le jeu installé, l'arrondi de GOG et la
 * marge de Windows, moins ce qui est déjà reçu d'une installation arrêtée. */
export function placeNecessaire(annonce: number, marge: number, dejaRecus = 0): number {
  const avecArrondi = annonce + Math.floor(annonce / 100) * MARGE_ARRONDI_POURCENT;
  return Math.max(0, avecArrondi * 2 + marge - dejaRecus);
}

/** Les octets annoncés pour ce qui est coché (le jeu et ses DLC). */
export function octetsCoches(p: PreparationGog, ids: string[]): number {
  return [p.jeu, ...p.dlcs].filter((e) => !!e && ids.includes(e.id)).reduce((s, e) => s + (e?.taille ?? 0), 0);
}

/** Le texte de la fenêtre d'accord : ce qui va se passer, la langue, le dossier, la question de Windows. `auCanape` :
 * Taodbox (décision de Seb, 27/11 : il faudra un clavier ou la souris pour répondre Oui à Windows). */
export function messageAccordGog(p: PreparationGog, auCanape: boolean): string {
  const l = [
    'Frogtend télécharge l’installeur officiel de GOG, puis installe le jeu tout seul.',
    // L'installeur français dit « en français » ; un installeur anglais d'un jeu qui CONTIENT le français (Cyberpunk,
    // relevé du 27/11) ne le prouve pas pour la langue au démarrage (relecture de l'expert) : « contient le français ».
    p.langue_installeur === 'fr' ? 'Le jeu est en français.' : p.en_francais ? 'Le jeu contient le français.' : 'Le jeu est en anglais (GOG ne le propose pas en français).',
    `Dossier : ${p.dossier}`,
    'Windows va te demander une autorisation (droits administrateur) : réponds Oui.',
  ];
  if (auCanape) l.push('Il faudra un clavier ou la souris pour répondre Oui à Windows.');
  l.push('Une fois le jeu installé et vérifié, l’installeur est effacé.');
  l.push('Tu peux l’arrêter pendant le téléchargement ; une fois l’installation lancée, elle va jusqu’au bout.');
  if (p.deja_recus > 0) l.push(`Déjà téléchargé : ${taille(p.deja_recus)} (le téléchargement reprend où il en était).`);
  return l.join('\n');
}

/** La ligne de total de l'accord : « 4,2 Go à télécharger, environ 12 min » ; sans débit connu, pas de durée ; rien
 * de coché : pas de durée non plus. */
export function resumeAccordGog(p: PreparationGog, ids: string[], debit: number | null | undefined): string {
  const octets = octetsCoches(p, ids);
  if (!octets) return 'Rien de coché.';
  const reste = Math.max(0, octets - p.deja_recus);
  const d = reste > 0 ? dureeEstimee(reste, debit) : null;
  return `Environ ${taille(reste)} à télécharger${d ? `, ${d}` : ''}.`;
}

/** Pourquoi on ne peut pas valider l'accord : le jeu décoché, ou la place qui manque (`null` : on peut). */
export function refusAccordGog(p: PreparationGog, ids: string[]): string | null {
  if (!p.jeu || !ids.includes(p.jeu.id)) return 'Le jeu lui-même est nécessaire : garde-le coché.';
  const besoin = placeNecessaire(octetsCoches(p, ids), p.marge, p.deja_recus);
  if (p.libre !== null && p.libre < besoin) {
    return `Pas assez de place sur le disque : il faut ${taille(besoin)} libres (l’installeur, puis le jeu installé), il en reste ${taille(p.libre)}.`;
  }
  return null;
}

/** Le message de fin : « ✅ « X » est installé : ▶ Jouer. », chaque DLC qui n'a pas pu s'installer (le jeu reste),
 * la place gardée par leurs installeurs, et un dossier où Frogtend ne peut pas écrire. */
export function messageFinGog(b: BilanInstallationGog): string {
  const l = [b.rattrape ? `✅ « ${b.titre} » avait été installé pendant que Frogtend était fermé : il est vérifié, ▶ Jouer.` : `✅ « ${b.titre} » est installé : ▶ Jouer.`];
  for (const [t, r] of b.dlcs_echoues) l.push(`⚠ Le DLC « ${t} » ne s’est pas installé (${r}) ; le jeu, lui, est bien installé.`);
  if (b.installeurs_gardes > 0) l.push(`Ses installeurs sont gardés (${taille(b.installeurs_gardes)}) dans le dossier « Installeurs GOG (Frogtend) » de tes jeux.`);
  if (b.dossier_protege) l.push('⚠ Frogtend ne peut pas écrire dans le dossier du jeu : ses parties et ses mods pourraient ne pas s’y poser.');
  return l.join('\n');
}

/** La fin mérite-t-elle une fenêtre (à lire, même au retour d'une longue absence) plutôt qu'un message qui passe ? */
export function finALire(b: BilanInstallationGog): boolean {
  return b.dlcs_echoues.length > 0 || b.installeurs_gardes > 0 || b.dossier_protege;
}

/** Le numéro GOG d'une clé de carte (« gog_123 » ou « 123 »). */
export function numeroGog(cle: string | null | undefined): string {
  return (cle ?? '').replace(/^gog_/, '');
}
