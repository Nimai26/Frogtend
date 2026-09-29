// Les actions sur un jeu du PC : l'installer, choisir ce qui le lance, y jouer, régler l'émulateur de son système,
// mettre ses parties à l'abri, le retirer du PC. Chaque action qui touche au disque est expliquée et acceptée d'abord.

import { listen } from '@tauri-apps/api/event';
import { isTauri } from '@tauri-apps/api/core';
import { api, duree, estErreurCoeur, taille, type Candidat, type EmulateurRecommande, type JeuPc } from '$lib/api';
import { choisir, confirmer, demander, informer, toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { etat, reglerPc } from '$lib/etat.svelte';
import { ludo, rechargerListe, rechargerPlateformes } from './ludotheque.svelte';
import { rechargerJeuxDuPc, tele } from './telechargements.svelte';

export const partie = $state<{ enJeu: number | null; installation: number | null }>({ enJeu: null, installation: null });

let ecoute: (() => void) | null = null;

/** Écoute les débuts et fins de partie (une seule fois). */
export async function suivreParties() {
  if (ecoute || !isTauri()) return;
  ecoute = await listen<{ sorte: 'debut' | 'fin'; jeu: number; secondes?: number }>('partie', async (e) => {
    if (e.payload.sorte === 'debut') {
      partie.enJeu = e.payload.jeu;
    } else {
      partie.enJeu = null;
      const titre = tele.jeux[e.payload.jeu]?.titre ?? 'le jeu';
      toast(`🏁 Partie de « ${titre} » terminée (${duree(e.payload.secondes ?? 0)}).`);
      await rechargerJeuxDuPc();
    }
  });
}

/** Choisir un fichier ou un dossier avec l'explorateur de Windows. */
async function parcourir(options: { dossier?: boolean; titre: string; extensions?: string[] }): Promise<string | null> {
  if (!isTauri()) return null;
  const { open } = await import('@tauri-apps/plugin-dialog');
  const r = await open({
    directory: options.dossier ?? false,
    multiple: false,
    title: options.titre,
    filters: options.extensions ? [{ name: 'Programmes', extensions: options.extensions }] : undefined,
  });
  return typeof r === 'string' ? r : null;
}

async function apresChangement() {
  await rechargerJeuxDuPc();
  if (ludo.espace === 'ludotheque') {
    await rechargerPlateformes();
    await rechargerListe();
  }
}

/** Installer un jeu téléchargé : on explique ce qui va se passer (et ses notes), puis on attend l'accord. */
export async function installer(id: number): Promise<boolean> {
  let p;
  try {
    p = await api.preparerInstallation(id);
  } catch (e) {
    toast(`Refusé : ${motifDuRefus(e)}`, 'erreur');
    return false;
  }
  const notes = p.notes ? `\n\n📝 Notes de cette version :\n${p.notes}` : '';
  let automatique = true;

  if (p.methode.sorte === 'installeur') {
    const c = await choisir(
      `📦 Installer « ${p.titre} »`,
      [
        { valeur: 'auto', libelle: '⚙ Installation automatique (conseillée)', detail: `dans ${p.destination}` },
        { valeur: 'guidee', libelle: '🧭 Installation guidée', detail: 'l’installeur s’ouvre, tu choisis toi-même' },
      ],
      `L’installeur (${p.methode.format === 'inno' ? 'Inno Setup' : 'NSIS'}) peut demander l’accord de Windows (droits administrateur).${notes}`,
    );
    if (!c) return false;
    automatique = c === 'auto';
  } else if (p.methode.sorte === 'installeur_guide') {
    const oui = await confirmer(`📦 Installer « ${p.titre} »`, {
      message: `L’installeur du jeu va s’ouvrir. Quand il demande où installer, choisis ce dossier :\n${p.destination}\n\nIl peut demander l’accord de Windows (droits administrateur).${notes}`,
      libelleValider: '📦 Ouvrir l’installeur',
    });
    if (!oui) return false;
  } else if (p.methode.sorte === 'archive') {
    const oui = await confirmer(`📦 Installer « ${p.titre} »`, {
      message: `L’archive va être décompressée dans :\n${p.destination}${notes}`,
      libelleValider: '📦 Installer',
    });
    if (!oui) return false;
  }
  // « aucune » : rien n'est écrit (ROM, image disque, jeu déjà prêt) ; on retient seulement comment le lancer.

  partie.installation = id;
  try {
    if (p.methode.sorte !== 'aucune') toast(`📦 Installation de « ${p.titre} »… (ça peut prendre quelques minutes)`);
    await api.installer(id, automatique);
  } catch (e) {
    partie.installation = null;
    if (estErreurCoeur(e) && e.motif.includes("rien n'est dans")) {
      // Installé ailleurs : la personne indique où.
      const d = await confirmer('📂 Où le jeu a-t-il été installé ?', {
        message: `${e.motif}\n\nChoisis le dossier où l’installeur a mis le jeu.`,
        libelleValider: '📂 Choisir le dossier',
      });
      if (!d) return false;
      const dossier = await parcourir({ dossier: true, titre: 'Dossier du jeu installé' });
      if (!dossier) return false;
      try {
        await api.installeAilleurs(id, dossier);
      } catch (e2) {
        toast(`Refusé : ${motifDuRefus(e2)}`, 'erreur');
        return false;
      }
    } else {
      toast(`Échec de l’installation : ${motifDuRefus(e)}`, 'erreur');
      return false;
    }
  }
  partie.installation = null;
  await apresChangement();
  if (p.nature === 'rom' || p.nature === 'image_disque') {
    toast(`✅ « ${p.titre} » est prêt : il se lance par l’émulateur de son système.`);
    return true;
  }
  toast(`✅ « ${p.titre} » est installé. Reste à choisir ce qui le lance.`);
  return choisirLanceur(id);
}

/** Choisir ce qui lance le jeu : les candidats trouvés dans son dossier, ou un autre programme. */
export async function choisirLanceur(id: number): Promise<boolean> {
  let candidats: Candidat[] = [];
  try {
    candidats = await api.candidatsLancement(id);
  } catch (e) {
    toast(`Refusé : ${motifDuRefus(e)}`, 'erreur');
    return false;
  }
  const titre = tele.jeux[id]?.titre ?? 'ce jeu';
  const c = await choisir<Candidat | 'autre'>(
    `🎯 Qu’est-ce qui lance « ${titre} » ?`,
    [
      ...candidats.slice(0, 8).map((c, i) => ({
        valeur: c as Candidat | 'autre',
        libelle: `${i === 0 ? '⭐ ' : ''}${c.relatif}`,
        detail: i === 0 ? 'le plus probable' : undefined,
      })),
      { valeur: 'autre', libelle: '📂 Un autre programme…', detail: 'le choisir avec l’explorateur' },
    ],
    'Frogtend retiendra ce choix. Les notes de la version disent souvent quoi lancer.',
  );
  if (!c) return false;
  let lanceur = c === 'autre' ? null : c.lanceur;
  if (c === 'autre') {
    const programme = await parcourir({ titre: 'Programme qui lance le jeu', extensions: ['exe', 'bat', 'cmd', 'lnk'] });
    if (!programme) return false;
    const dossier = programme.replace(/[\\/][^\\/]+$/, '');
    const bas = programme.toLowerCase();
    lanceur = bas.endsWith('.bat') || bas.endsWith('.cmd')
      ? { programme: 'cmd.exe', arguments: ['/C', programme], dossier }
      : bas.endsWith('.lnk')
        ? { programme: 'explorer.exe', arguments: [programme], dossier }
        : { programme, arguments: [], dossier };
  }
  try {
    await api.choisirLanceur(id, lanceur!);
    await rechargerJeuxDuPc();
    toast('Enregistré.');
    return true;
  } catch (e) {
    toast(`Refusé : ${motifDuRefus(e)}`, 'erreur');
    return false;
  }
}

/** Régler l'émulateur d'un système, d'après les recommandations de Firehouse. */
export async function reglerEmulateur(plateforme: string): Promise<boolean> {
  const recs = (await api.emulateursRecommandes(plateforme).catch(() => ({ emulateurs: [] }))).emulateurs ?? [];
  const reponse = await choisir<EmulateurRecommande | 'autre'>(
    `🕹 Quel émulateur pour ${plateforme} ?`,
    [
      ...recs.map((r) => ({
        valeur: r as EmulateurRecommande | 'autre',
        libelle: `${r.recommande ? '⭐ ' : ''}${r.nom}${r.recommande ? ' (recommandé)' : ''}`,
        detail: r.ligne_de_commande || undefined,
      })),
      { valeur: 'autre', libelle: '📂 Un autre émulateur…', detail: 'ligne de commande à écrire' },
    ],
    recs.length ? 'Recommandations de Firehouse. L’émulateur doit déjà être installé sur ce PC.' : 'Firehouse n’a pas de recommandation pour ce système.',
  );
  if (reponse === null) return false;
  const choix = reponse === 'autre' ? null : reponse;
  const nom = choix?.nom ?? 'l’émulateur';
  const ok = await confirmer(`📂 Où est ${nom} ?`, {
    message: `Choisis le programme de ${nom} sur ce PC${nom === 'Retroarch' ? ' (retroarch.exe)' : ''}.${
      choix?.bios ? `\n\n⚠ Ce système a besoin du BIOS « ${choix.bios} », à placer dans le dossier « system » de l’émulateur.` : ''
    }${choix?.site ? `\n\nPas encore installé ? Site officiel : ${choix.site}` : ''}`,
    libelleValider: '📂 Choisir le programme',
  });
  if (!ok) return false;
  const programme = await parcourir({ titre: `Programme de ${nom}`, extensions: ['exe'] });
  if (!programme) return false;
  const ligne = await demander(`⌨ Ligne de commande de ${nom}`, {
    message: 'Le fichier du jeu est ajouté à la fin. Laisse vide si l’émulateur n’a besoin de rien d’autre.',
    valeur: choix?.ligne_de_commande ?? '',
  });
  if (ligne === null) return false;
  await reglerPc('emulateurs', { ...etat.pc.emulateurs, [plateforme]: { programme, ligne, nom } });
  toast(`✅ ${nom} réglé pour ${plateforme}.`);
  return true;
}

/** Jouer. Si un réglage manque (émulateur, lanceur), on le demande puis on relance. */
export async function jouer(id: number) {
  const j = tele.jeux[id];
  for (let essai = 0; essai < 2; essai++) {
    try {
      await api.jouer(id);
      toast(`▶ « ${j?.titre ?? 'Le jeu'} » se lance…`);
      return;
    } catch (e) {
      if (estErreurCoeur(e) && e.sorte === 'reglage' && j) {
        if (!(await reglerEmulateur(j.plateforme))) return;
      } else if (estErreurCoeur(e) && e.motif.startsWith('Choisis d')) {
        if (!(await choisirLanceur(id))) return;
      } else {
        toast(`Impossible de lancer le jeu : ${motifDuRefus(e)}`, 'erreur');
        return;
      }
    }
  }
}

export async function mettreALAbri(id: number) {
  try {
    const a = await api.mettreALAbri(id);
    if (a.fichiers === 0) toast('Rien de nouveau à mettre à l’abri : aucune partie n’a changé depuis l’installation.');
    else toast(`✅ ${a.fichiers} fichier(s) de parties (${taille(a.octets)}) mis à l’abri dans ${a.dossier}.`);
  } catch (e) {
    toast(`Échec de la mise à l’abri : ${motifDuRefus(e)}`, 'erreur');
  }
}

/** Retirer un jeu du PC, après avoir dit exactement ce qui part et ce qui reste. */
export async function retirer(j: JeuPc) {
  const oui = await confirmer(`🗑 Retirer « ${j.titre} » de ce PC ?`, {
    message: [
      `Tes parties seront d’abord mises à l’abri (copie gardée par Frogtend). Si la copie échoue, rien n’est retiré.`,
      ``,
      `Partira : l’installation${j.installation ? ` (${j.installation.dossier})` : ''}, les fichiers téléchargés (${taille(j.total)}), la fiche et les documents gardés.`,
      `Restera : les copies de tes parties, et tout fichier que tu aurais mis toi-même dans le dossier du jeu.`,
      ``,
      `Tu pourras le remettre dans ta ludothèque depuis le catalogue.`,
    ].join('\n'),
    danger: true,
    libelleValider: 'Retirer du PC',
  });
  if (!oui) return;
  try {
    const a = await api.retirer(j.id);
    toast(
      a.fichiers
        ? `✅ « ${j.titre} » retiré. ${a.fichiers} fichier(s) de parties à l’abri dans ${a.dossier}.`
        : `✅ « ${j.titre} » retiré du PC.`,
    );
    if (ludo.selection?.id === j.id) ludo.selection = null;
    await apresChangement();
  } catch (e) {
    toast(`Refusé : ${motifDuRefus(e)}`, 'erreur');
  }
}
