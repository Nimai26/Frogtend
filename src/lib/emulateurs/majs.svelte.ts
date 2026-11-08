// Les mises à jour des émulateurs installés par Frogtend (Seb, 08/11) : cherchées depuis ⚙ Options ▸ Émulateurs, et
// une fois par jour au démarrage (un message qui les propose, jamais en Taodbox ni pendant une partie, rien hors
// ligne). Une seule à la fois : le cœur refuse deux installations dans le même dossier (`occuper_installation`).

import { api, taille, type EmulateurInstalle } from '$lib/api';
import { confirmer, pile, toast } from '$lib/dialogues/fenetres.svelte';
import { etat, reglerPc } from '$lib/etat.svelte';
import { auCanape } from '$lib/taodbox/canape';
import { partie } from '$lib/ludotheque/jeu.svelte';
import { installationEmulateur, installerEmulateur } from './assistant.svelte';
import { miseAJourDisponible } from './versions';

const JOUR_S = 24 * 3600;

/** La dernière version connue (et sa taille), par émulateur : partagée par ⚙ Réglages et le démarrage. */
export const dernieres = $state<Record<string, { version: string; taille?: number | null }>>({});

export interface MiseAJour {
  emulateur: EmulateurInstalle;
  version: string;
  taille?: number | null;
}

/** Les mises à jour disponibles des émulateurs installés par Frogtend (un émulateur injoignable : passé sous silence). */
export async function chercher(installes?: EmulateurInstalle[]): Promise<MiseAJour[]> {
  const l = installes ?? (await api.emulateursInstalles().catch(() => []));
  const trouvees: MiseAJour[] = [];
  for (const e of l.filter((x) => x.par_frogtend)) {
    try {
      const p = await api.emulateurDerniereVersion(e.id);
      dernieres[e.id] = { version: p.version, taille: p.taille };
      if (miseAJourDisponible(e.id, p.version, e.version)) trouvees.push({ emulateur: e, version: p.version, taille: p.taille });
    } catch {
      // hors ligne, ou sa source ne répond pas : on n'en sait pas plus
    }
  }
  return trouvees;
}

/** Une ligne lisible par un novice : le nom, la nouvelle version, la taille. */
export function ligne(m: MiseAJour): string {
  return `${m.emulateur.nom} ${m.version}${m.taille ? ` (${taille(m.taille)})` : ''}`;
}

/** Met à jour ces émulateurs l'un après l'autre (l'accord est déjà donné pour toute la liste). Rend le nombre réussi. */
export async function toutMettreAJour(l: MiseAJour[]): Promise<number> {
  let n = 0;
  for (const m of l) {
    // Une autre installation en cours (« Prêt à jouer ») : on attend qu'elle finisse (une heure au plus), jamais deux
    // à la fois.
    for (let i = 0; installationEmulateur.id !== null && i < 7200; i++) await new Promise((r) => setTimeout(r, 500));
    if (installationEmulateur.id !== null) break;
    if (await installerEmulateur(m.emulateur.id, m.emulateur.nom, true, { sansQuestion: true })) n++;
  }
  return n;
}

let demandee = false;

/** Au démarrage (profil ouvert) : une fois par jour et par PC, propose les mises à jour trouvées, en UNE question. */
export async function emulateursAuDemarrage() {
  if (demandee || auCanape() || etat.pc.firehouse.simule) return;
  const maintenant = Math.floor(Date.now() / 1000);
  if (maintenant - (etat.pc.majEmulateursDerniere ?? 0) < JOUR_S) return;
  demandee = true;
  const l = await chercher();
  // Hors ligne (rien n'a pu être lu, ou aucun émulateur installé par Frogtend) : silencieux, et la date n'est pas notée
  // (on réessaie au prochain démarrage ; sans émulateur, aucun appel réseau n'est fait).
  if (!Object.keys(dernieres).length) return;
  await reglerPc('majEmulateursDerniere', maintenant);
  if (!l.length) return;
  // Les autres questions du démarrage d'abord (sauvegardes, Game Bar, Frogtend) : jamais deux fenêtres empilées.
  for (let i = 0; pile.dialogues.length && i < 1200; i++) await new Promise((r) => setTimeout(r, 500));
  if (pile.dialogues.length || auCanape() || partie.enJeu !== null || partie.lancement !== null) return;
  const total = l.reduce((s, m) => s + (m.taille ?? 0), 0);
  const oui = await confirmer(`⬆ ${l.length === 1 ? 'Un émulateur a' : `${l.length} émulateurs ont`} une nouvelle version`, {
    message: [
      ...l.map((m) => `• ${ligne(m)}`),
      '',
      'Tes parties, tes clés et tes réglages sont gardés ; l’ancienne version est mise à l’abri dans son dossier.',
      'Plus tard : ⚙ Options ▸ Émulateurs ▸ « 🔎 Chercher des mises à jour ».',
    ].join('\n'),
    libelleValider: `⬆ Mettre à jour${total ? ` (${taille(total)})` : ''}`,
  });
  if (!oui) return;
  const n = await toutMettreAJour(l);
  if (l.length > 1) toast(n === l.length ? `✅ ${n} émulateurs mis à jour.` : `⬆ ${n} sur ${l.length} émulateurs mis à jour.`);
}
