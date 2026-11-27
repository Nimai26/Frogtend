// Lot C4c-2 (décisions de Seb des 26 et 27/11) : « ⬇ Installer » d'un jeu GOG l'installe SANS GOG Galaxy, par
// l'installeur hors ligne officiel de GOG, avec ses DLC possédés cochés d'office. Le même chemin au bureau (panneau,
// double-clic) et au canapé (Taodbox). GOG Galaxy n'est plus qu'un dernier secours, proposé seulement quand GOG n'a pas
// d'installeur hors ligne (et si Galaxy est sur ce PC).
import { api, taille, type ElementGogVu, type PreparationGog } from '$lib/api';
import { cocher, confirmer, informer, toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { avecProgres } from '$lib/mods/progres.svelte';
import { debitConnu, rechargerJeuxDuPc } from '$lib/ludotheque/telechargements.svelte';
import { rechargerListe, rechargerPlateformes } from '$lib/ludotheque/ludotheque.svelte';
import { confirmerInstallationGog, finALire, messageAccordGog, messageFinGog, numeroGog, octetsCoches, refusAccordGog, resumeAccordGog } from './installation-gog';

/** Où en est une installation GOG. */
export type PhaseGog = 'preparation' | 'telechargement' | 'installation';

/** L'installation GOG en cours (une seule à la fois : jamais deux par un double appui). `phase` : `preparation` (fiche
 * de GOG, accord), `telechargement` (seule phase où l'on peut l'arrêter), `installation` (l'installeur de GOG travaille :
 * il va jusqu'au bout). `finies` : compteur des installations réussies (Taodbox relit alors ses cartes, d'où que
 * l'installation soit partie). */
export const installGog = $state<{ cle: string | null; titre: string; phase: PhaseGog | null; finies: number }>({ cle: null, titre: '', phase: null, finies: 0 });

/** Ce jeu (sa clé de carte) est-il en train de s'installer ? */
export function enInstallationGog(cle: string | null | undefined): boolean {
  return installGog.cle !== null && !!cle && numeroGog(installGog.cle) === numeroGog(cle);
}

/** Avant de quitter Frogtend : le message si un installeur de GOG travaille (le jeu ne serait ni vérifié ni prêt à
 * jouer avant le prochain « ⬇ Installer ») ; `null` sinon. Pur, testé. */
export function avertissementSortieGog(): string | null {
  if (installGog.cle === null || installGog.phase === 'preparation') return null;
  if (installGog.phase === 'telechargement') return `« ${installGog.titre} » est en train de se télécharger : ce qui est reçu est gardé, ⬇ Installer reprendra où il en était.`;
  return `« ${installGog.titre} » est en train de s’installer : si tu quittes maintenant, il ne sera ni vérifié ni prêt à jouer avant ton prochain ⬇ Installer.`;
}

/** Le libellé d'une case de l'accord. */
function option(e: ElementGogVu, jeu: boolean) {
  return { valeur: e.id, libelle: jeu ? `${e.titre} (le jeu)` : `${e.titre} (DLC)`, detail: taille(e.taille) };
}

/** La fenêtre d'accord : les cases (le jeu en premier, ses DLC cochés d'office), ou une simple confirmation sans DLC.
 * Rend les numéros des DLC gardés, ou `null` (la personne a annulé, ou il n'y a pas la place). */
export async function accordGog(p: PreparationGog, auCanape: boolean): Promise<string[] | null> {
  if (!p.jeu) return null;
  const titre = `⬇ Installer « ${p.titre} » ?`;
  const message = messageAccordGog(p, auCanape);
  const debit = debitConnu();
  if (!p.dlcs.length) {
    const ids = [p.jeu.id];
    const refus = refusAccordGog(p, ids);
    if (refus) {
      await informer(titre, `${message}\n\n${refus}`);
      return null;
    }
    const oui = await confirmer(titre, { message: `${message}\n\n${resumeAccordGog(p, ids, debit)}`, libelleValider: '⬇ Installer' });
    return oui ? [] : null;
  }
  const jeu = p.jeu.id;
  const choisis = await cocher(titre, [option(p.jeu, true), ...p.dlcs.map((d) => option(d, false))], {
    message,
    coches: [true, ...p.dlcs.map(() => true)],
    resume: (ids) => resumeAccordGog(p, ids, debit),
    refus: (ids) => refusAccordGog(p, ids),
    libelleValider: '⬇ Installer',
  });
  if (!choisis || !choisis.includes(jeu)) return null;
  return choisis.filter((id) => id !== jeu);
}

/** Quand Frogtend ne peut pas l'installer lui-même : le dire ; GOG Galaxy en dernier secours SEULEMENT s'il peut aider
 * (pas d'installeur hors ligne) et s'il est sur ce PC — au canapé, en prévenant qu'il peut demander la souris. */
async function secoursGalaxy(cle: string, titre: string, p: PreparationGog, auCanape: boolean) {
  const raison = p.refus ?? 'Frogtend ne peut pas installer ce jeu lui-même.';
  if (!p.galaxy || !p.galaxy_peut_aider) {
    await informer(`Installer « ${titre} »`, raison);
    return;
  }
  // (Le même conseil que `jeu-boutique::messageInstallationCanape`, écrit ici : jeu-boutique importe ce module.)
  const canape = auCanape ? '\n\nGOG Galaxy peut demander la souris ; ⊞/Guide pour revenir à Frogtend.' : '';
  const oui = await confirmer(`Installer « ${titre} » avec GOG Galaxy ?`, { message: `${raison}\n\nTu peux l’installer avec GOG Galaxy, qui est sur ce PC.${canape}`, libelleValider: 'Ouvrir GOG Galaxy' });
  if (!oui || !(await confirmerInstallationGog(cle))) return;
  try {
    await api.boutiqueOuvrir('gog', cle, 'installer');
  } catch (e) {
    toast(motifDuRefus(e), 'erreur');
  }
}

/** Après une installation réussie : les jeux du PC et la ludothèque relus (▶ Jouer aussitôt), et Taodbox prévenu. */
async function relireApres() {
  await rechargerJeuxDuPc().catch(() => {});
  await rechargerListe().catch(() => {});
  await rechargerPlateformes().catch(() => {});
  installGog.finies++;
}

/** « ⬇ Installer » un jeu GOG (`cle` : sa clé de carte) : préparation, accord, téléchargement suivi (arrêtable),
 * installeur, vérification, puis les listes relues. Rend vrai si le jeu est installé. `auCanape` : Taodbox. */
export async function installerGog(j: { cle: string; titre: string }, o: { auCanape?: boolean } = {}): Promise<boolean> {
  if (installGog.cle !== null) {
    toast(enInstallationGog(j.cle) ? 'Ce jeu est déjà en train de s’installer.' : 'Une installation GOG est déjà en cours : attends qu’elle finisse.', 'alerte');
    return false;
  }
  const cle = numeroGog(j.cle);
  // Pris tout de suite : un double appui pendant la préparation ne lance jamais une 2e installation.
  installGog.cle = cle;
  installGog.titre = j.titre;
  installGog.phase = 'preparation';
  let titreFin = j.titre;
  try {
    let p: PreparationGog;
    try {
      p = await api.gogInstallationPreparer(cle);
    } catch (e) {
      toast(motifDuRefus(e), 'erreur');
      return false;
    }
    titreFin = p.titre || j.titre;
    let dlcs: string[];
    if (p.rattrapage) {
      // Installé pendant que Frogtend était fermé : rien à télécharger, seulement vérifier et rendre jouable.
      const oui = await confirmer(`« ${titreFin} » est déjà installé`, {
        message: 'Il a été installé pendant que Frogtend était fermé. Frogtend le vérifie et le rend jouable, sans rien télécharger.',
        libelleValider: '✅ Terminer',
      });
      if (!oui) return false;
      dlcs = [];
    } else if (p.refus || !p.jeu) {
      await secoursGalaxy(cle, titreFin, p, !!o.auCanape);
      return false;
    } else {
      const accord = await accordGog(p, !!o.auCanape);
      if (accord === null) return false;
      dlcs = accord;
    }
    installGog.titre = titreFin;
    installGog.phase = p.rattrapage ? 'installation' : 'telechargement';
    const total = p.jeu ? octetsCoches(p, [p.jeu.id, ...dlcs]) : 0;
    const b = await avecProgres(
      `⬇ Installation de « ${titreFin} »`,
      true,
      (suivi) => api.gogInstaller(cle, dlcs, suivi),
      p.rattrapage ? undefined : { etape: 'telechargement', total, faits: p.deja_recus },
      (e) => {
        if (e.etape === 'installation') installGog.phase = 'installation';
      },
    );
    if (b.pause) {
      toast(`⏸ Installation de « ${titreFin} » arrêtée : ce qui est téléchargé est gardé. Relance l’installation de ce jeu pour reprendre où elle en était.`, 'neutre');
      return false;
    }
    await relireApres();
    // Une fin à lire (DLC ratés, place gardée, dossier protégé) : une fenêtre, jamais un message qui passe.
    if (finALire(b)) await informer(`Installation de « ${titreFin} »`, messageFinGog(b));
    else toast(messageFinGog(b), 'ok');
    return true;
  } catch (e) {
    // Après l'accord, souvent sans personne devant l'écran : une fenêtre à lire (refus de Windows, fichier abîmé…).
    const m = motifDuRefus(e);
    if (installGog.phase === 'preparation') toast(m, m.includes('⚠') ? 'alerte' : 'erreur');
    else await informer(`Installation de « ${titreFin} »`, m);
    return false;
  } finally {
    installGog.cle = null;
    installGog.phase = null;
  }
}

/** « ⏹ Arrêter l'installation » : seulement pendant le téléchargement, après une confirmation (ce qui est reçu est
 * gardé). Une fois l'installeur de GOG lancé, il n'y a plus rien à arrêter : on le dit. */
export async function arreterInstallationGog() {
  const cle = installGog.cle;
  if (!cle) return;
  if (installGog.phase === 'preparation') {
    toast('Frogtend prépare l’installation : rien n’est encore téléchargé.', 'neutre');
    return;
  }
  if (installGog.phase !== 'telechargement') {
    toast('L’installeur de GOG travaille déjà : il ne peut plus être arrêté. Attends la fin.', 'alerte');
    return;
  }
  const oui = await confirmer('Arrêter l’installation ?', {
    message: `Le téléchargement de « ${installGog.titre} » s’arrête ; ce qui est déjà reçu est gardé pour reprendre plus tard. Rien n’est installé.`,
    libelleValider: '⏹ Arrêter',
    danger: true,
  });
  if (!oui || installGog.cle !== cle) return;
  try {
    await api.gogInstallationAnnuler(cle);
  } catch (e) {
    toast(motifDuRefus(e), 'alerte');
  }
}
