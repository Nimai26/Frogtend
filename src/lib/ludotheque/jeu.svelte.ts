// Les actions sur un jeu du PC : l'installer, choisir ce qui le lance, y jouer, régler l'émulateur de son système,
// mettre ses parties à l'abri, le retirer du PC. Chaque action qui touche au disque est expliquée et acceptée d'abord.

import { listen } from '@tauri-apps/api/event';
import { isTauri } from '@tauri-apps/api/core';
import { api, duree, estErreurCoeur, taille, type Candidat, type JeuPc } from '$lib/api';
import { choisir, confirmer, toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { reglerEmulateur } from '$lib/emulateurs/assistant.svelte';
import { preparerPourJouer } from '$lib/emulateurs/pret.svelte';
import { normaliser, pourLeJeu } from '$lib/emulateurs/choix';
import { etat, reglerPc, reglerProfil } from '$lib/etat.svelte';
import type { CommandesJeu } from '$lib/reglages/reglages';
import { GENRES_MANETTE, libelleCommandes } from './commandes';
import { apresUnePartie } from '$lib/sauvegarde.svelte';
import { ludo, rechargerListe, rechargerPlateformes } from './ludotheque.svelte';
import { rechargerJeuxDuPc, tele } from './telechargements.svelte';
import { estCourante, libelleVersion, nomDuFichier } from './versions';

export const partie = $state<{ enJeu: number | null; installation: number | null }>({ enJeu: null, installation: null });

let ecoute: (() => void) | null = null;

/** Écoute les débuts et fins de partie (une seule fois). */
export async function suivreParties() {
  if (ecoute || !isTauri()) return;
  ecoute = await listen<{ sorte: 'debut' | 'fin'; jeu: number; secondes?: number; carte_cedee?: boolean }>('partie', async (e) => {
    if (e.payload.sorte === 'debut') {
      partie.enJeu = e.payload.jeu;
      if (e.payload.carte_cedee) toast('🎮 Session de jeu annoncée à Firehouse.', 'ok');
    } else {
      partie.enJeu = null;
      const titre = tele.jeux[e.payload.jeu]?.titre ?? 'le jeu';
      toast(`🏁 Partie de « ${titre} » terminée (${duree(e.payload.secondes ?? 0)}).`);
      await rechargerJeuxDuPc();
      await apresUnePartie();
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
    // Sa console n'a pas encore d'émulateur : le proposer tout de suite (un seul accord).
    const plateforme = tele.jeux[id]?.plateforme;
    if (plateforme) await preparerPourJouer(plateforme, 1);
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

/** Jouer. `emulateur` : la clé d'un émulateur du système, pour ce lancement seulement. Si un réglage manque
 * (émulateur, lanceur), on le demande puis on relance. */
export async function jouer(id: number, emulateur?: string, version?: string) {
  const j = tele.jeux[id];
  for (let essai = 0; essai < 2; essai++) {
    try {
      await api.jouer(id, etat.profil.commandes[String(id)], emulateur, version);
      toast(`▶ « ${j?.titre ?? 'Le jeu'} » se lance…`);
      return;
    } catch (e) {
      if (estErreurCoeur(e) && e.sorte === 'reglage' && j) {
        // Pas d'émulateur pour cette console : « Prêt à jouer » le prépare (un seul accord), puis on relance.
        if (!(await preparerPourJouer(j.plateforme, 1))) return;
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
/** Un jeu importé de ton disque (menu Importer) : Frogtend n'a rien installé, il n'efface rien. */
export const VERSION_IMPORTEE = -1;

export async function retirer(j: JeuPc) {
  if (j.version === VERSION_IMPORTEE) {
    const oui = await confirmer(`Retirer « ${j.titre} » de ta ludothèque ?`, {
      message: `Ce jeu a été importé de ton disque : Frogtend l’oublie, mais ne touche à AUCUN fichier (${j.installation?.dossier ?? j.dossier}). Tu pourras le réimporter.`,
      libelleValider: 'Retirer de la ludothèque',
    });
    if (!oui) return;
    try {
      await api.retirer(j.id);
      toast(`✅ « ${j.titre} » retiré de ta ludothèque ; ses fichiers sont toujours sur le disque.`);
      if (ludo.selection?.id === j.id) ludo.selection = null;
      await apresChangement();
    } catch (e) {
      toast(`Refusé : ${motifDuRefus(e)}`, 'erreur');
    }
    return;
  }
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

/** Les émulateurs réglés pour le système d'un jeu. */
export function emulateursDuJeu(plateforme: string) {
  return normaliser(etat.pc.emulateurs[plateforme]);
}

/** « ▶ Jouer avec… » : choisir, pour ce lancement seulement, parmi les émulateurs du système (ou en ajouter un). */
/** Choisir une version (région) d'un jeu importé ; rend son chemin, `undefined` s'il n'y a pas le choix, `null` si
 * la personne renonce. */
async function choisirUneVersion(j: JeuPc, titre: string, aide: string): Promise<string | null | undefined> {
  const versions = j.installation?.versions ?? [];
  if (versions.length < 2 || !j.installation) return undefined;
  const i = j.installation;
  return choisir<string>(
    titre,
    versions.map((v) => ({
      valeur: v.chemin,
      libelle: `${estCourante(v, i) ? '⭐ ' : ''}${libelleVersion(v)}`,
      detail: nomDuFichier(v),
    })),
    aide,
  );
}

export async function jouerAvec(id: number) {
  const j = tele.jeux[id];
  if (!j) return;
  // D'abord la version (région), s'il y en a plusieurs.
  const version = await choisirUneVersion(j, `▶ Quelle version de « ${j.titre} » ?`, 'Pour cette partie seulement. ⭐ : celle d’habitude (⚙ Gérer le jeu ▸ 📀 Version).');
  if (version === null) return;
  const s = emulateursDuJeu(j.plateforme);
  if (version !== undefined && s.liste.length < 2) {
    await jouer(id, undefined, version);
    return;
  }
  const actuel = pourLeJeu(etat.pc.emulateurs, etat.pc.emulateursJeux, j.plateforme, id);
  const c = await choisir<string>(
    `▶ Jouer à « ${j.titre} » avec…`,
    [
      ...s.liste.map((e) => ({
        valeur: e.cle,
        libelle: `${e.cle === actuel?.cle ? '⭐ ' : ''}${e.nom || e.programme}`,
        detail: e.cle === actuel?.cle ? 'celui d’habitude pour ce jeu' : undefined,
      })),
      { valeur: '+', libelle: '➕ Un autre émulateur…', detail: `l’ajouter à ${j.plateforme}` },
    ],
    'Pour cette partie seulement. Le choix d’habitude se règle dans ⚙ Gérer le jeu ▸ 🕹 Émulateur.',
  );
  if (!c) return;
  const cle = c === '+' ? await reglerEmulateur(j.plateforme) : c;
  if (cle) await jouer(id, cle, version ?? undefined);
}

/** « 📀 Version » d'un jeu importé : celle qu'on lance d'habitude. */
export async function choisirVersionParDefaut(id: number) {
  const j = tele.jeux[id];
  if (!j) return;
  const v = await choisirUneVersion(j, `📀 Version de « ${j.titre} »`, 'Celle que ▶ Jouer lance. Ordre de préférence : FR, EU, US/EN, puis les autres.');
  if (!v) return;
  try {
    await api.choisirVersion(id, v);
    await rechargerJeuxDuPc();
    toast('✅ Version retenue pour ce jeu.');
  } catch (e) {
    toast(`Impossible : ${motifDuRefus(e)}`, 'erreur');
  }
}

/** « 🕹 Émulateur » d'un jeu : comme la console, ou un autre émulateur du système, par défaut pour ce jeu. */
export async function choisirEmulateurDuJeu(id: number) {
  const j = tele.jeux[id];
  if (!j) return;
  const s = emulateursDuJeu(j.plateforme);
  const duSysteme = s.liste.find((e) => e.cle === s.defaut);
  const propre = etat.pc.emulateursJeux[String(id)];
  const c = await choisir<string>(
    `🕹 Émulateur de « ${j.titre} »`,
    [
      { valeur: '', libelle: `${!propre ? '⭐ ' : ''}Comme la console`, detail: duSysteme ? duSysteme.nom || duSysteme.programme : 'aucun réglé' },
      ...s.liste.map((e) => ({ valeur: e.cle, libelle: `${propre === e.cle ? '⭐ ' : ''}${e.nom || e.programme}` })),
      { valeur: '+', libelle: '➕ Un autre émulateur…', detail: `l’ajouter à ${j.plateforme}` },
    ],
    `Le choix de la console se règle dans ⚙ Options ▸ Émulateurs.`,
  );
  if (c === null) return;
  const cle = c === '+' ? await reglerEmulateur(j.plateforme) : c;
  if (cle === null) return;
  const jeux = { ...etat.pc.emulateursJeux };
  if (cle) jeux[String(id)] = cle;
  else delete jeux[String(id)];
  await reglerPc('emulateursJeux', jeux);
  const e = emulateursDuJeu(j.plateforme).liste.find((x) => x.cle === cle);
  toast(cle ? `🕹 « ${j.titre} » se lancera avec ${e?.nom || 'cet émulateur'}.` : `🕹 « ${j.titre} » suit la console.`);
}

/** L'émulateur d'un jeu sur ce PC (son identifiant Frogtend), s'il est connu de Frogtend. */
async function emulateurDuSysteme(plateforme: string, jeu: number): Promise<{ id: string; nom: string } | null> {
  const programme = pourLeJeu(etat.pc.emulateurs, etat.pc.emulateursJeux, plateforme, jeu)?.programme;
  if (!programme) return null;
  const installes = await api.emulateursInstalles().catch(() => []);
  const e = installes.find((i) => i.programme.toLowerCase() === programme.toLowerCase());
  return e ? { id: e.id, nom: e.nom } : null;
}

/** « 🎮 Commandes » d'un jeu : automatique, clavier et souris, ou un réglage de manette de référence. */
export async function choisirCommandes(id: number) {
  const j = tele.jeux[id];
  if (!j) return;
  const emu = await emulateurDuSysteme(j.plateforme, id);
  const refs = emu ? await api.referencesManette(emu.id).catch(() => []) : [];
  const actuel = etat.profil.commandes[String(id)];
  const options: { valeur: CommandesJeu; libelle: string; detail?: string }[] = [
    { valeur: { mode: 'auto' }, libelle: '✨ Automatique', detail: 'la manette réglée par Frogtend (ou la tienne)' },
    { valeur: { mode: 'clavier' }, libelle: '⌨ Clavier et souris', detail: 'Frogtend ne règle pas de manette pour ce jeu' },
    ...refs.map((r) => ({
      valeur: { mode: 'reference' as const, genre: r.genre, reference: r.nom },
      libelle: `🎮 ${GENRES_MANETTE[r.genre] ?? r.genre} : ${r.nom}`,
      detail: r.livree ? 'réglage de départ de Frogtend' : 'réglage de référence de ce PC',
    })),
  ];
  const c = await choisir<CommandesJeu>(
    `🎮 Commandes de « ${j.titre} »`,
    options,
    `Actuellement : ${libelleCommandes(actuel)}.${emu ? '' : ' (Pas d’émulateur réglé pour ce système : seuls « Automatique » et « Clavier et souris » ont un sens.)'}`,
  );
  if (!c) return;
  const tous = { ...etat.profil.commandes };
  if (c.mode === 'auto') delete tous[String(id)];
  else tous[String(id)] = c;
  await reglerProfil('commandes', tous);
  toast(`🎮 « ${j.titre} » : ${libelleCommandes(c)}.`);
}

/** Le menu « ⚙ Gérer le jeu » : toutes les actions possibles sur un jeu du PC, au même endroit, bien visibles. */
export async function gererJeu(id: number) {
  const j = tele.jeux[id];
  if (!j) return;
  type Action = 'installer' | 'lanceur' | 'version' | 'emulateur' | 'commandes' | 'abri' | 'retirer';
  const options: { valeur: Action; libelle: string; detail?: string }[] = [];
  if (j.etat === 'telecharge' && !j.installation) options.push({ valeur: 'installer', libelle: '📦 Installer le jeu' });
  if (j.installation && !j.installation.fichier_du_jeu)
    options.push({ valeur: 'lanceur', libelle: '🎯 Changer ce qui lance le jeu' });
  const versions = j.installation?.versions ?? [];
  if (versions.length > 1 && j.installation) {
    const courante = versions.find((v) => estCourante(v, j.installation!));
    options.push({ valeur: 'version', libelle: '📀 Version', detail: `${courante ? libelleVersion(courante) : '?'} (${versions.length} versions)` });
  }
  const s = emulateursDuJeu(j.plateforme);
  if (s.liste.length) {
    const e = pourLeJeu(etat.pc.emulateurs, etat.pc.emulateursJeux, j.plateforme, id);
    options.push({
      valeur: 'emulateur',
      libelle: '🕹 Émulateur',
      detail: `${e?.nom || e?.programme || ''}${etat.pc.emulateursJeux[String(id)] ? ' (propre à ce jeu)' : ' (comme la console)'}`,
    });
  }
  options.push({ valeur: 'commandes', libelle: '🎮 Commandes', detail: libelleCommandes(etat.profil.commandes[String(id)]) });
  if (j.installation)
    options.push({ valeur: 'abri', libelle: '💾 Mettre mes parties à l’abri', detail: 'une copie de ce qui a changé depuis l’installation' });
  if (j.etat === 'telecharge')
    options.push(
      j.version === VERSION_IMPORTEE
        ? { valeur: 'retirer', libelle: '🗑 Retirer de la ludothèque', detail: 'ses fichiers restent sur ton disque' }
        : { valeur: 'retirer', libelle: '🗑 Retirer du PC (désinstaller)', detail: 'tes parties sont copiées à l’abri d’abord' },
    );
  const c = await choisir<Action>(`⚙ Gérer « ${j.titre} »`, options, `Rangé dans ${j.installation?.dossier ?? j.dossier}`);
  if (c === 'installer') await installer(id);
  else if (c === 'lanceur') await choisirLanceur(id);
  else if (c === 'version') await choisirVersionParDefaut(id);
  else if (c === 'emulateur') await choisirEmulateurDuJeu(id);
  else if (c === 'commandes') await choisirCommandes(id);
  else if (c === 'abri') await mettreALAbri(id);
  else if (c === 'retirer') await retirer(j);
}
