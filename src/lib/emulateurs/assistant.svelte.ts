// L'assistant des émulateurs : pour un système, proposer l'émulateur recommandé par Firehouse, l'installer depuis sa
// source officielle (sur accord, taille annoncée) ou reprendre celui déjà présent, installer le cœur RetroArch qui
// manque, vérifier les BIOS, puis retenir le réglage du système.

import { listen } from '@tauri-apps/api/event';
import { ajouter } from './choix';
import { avecConseil, biosADireOuPoser, biosATelecharger, biosRetroArch, dossierParDefaut, estConseille, ligneDe, palier, poidsBios, versionAutre } from './pret';
import { isTauri } from '@tauri-apps/api/core';
import { api, taille, type BilanBios, type EmulateurInstalle, type EmulateurRecommande } from '$lib/api';
import { choisir, confirmer, demander, fermerToast, informer, toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { etat, reglerPc } from '$lib/etat.svelte';
import { auCanape, seulementAuBureau } from '$lib/taodbox/canape';

export const installationEmulateur = $state<{ id: string | null; recus: number; total: number | null }>({
  id: null,
  recus: 0,
  total: null,
});

/** Choisir un fichier ou un dossier avec l'explorateur de Windows, ouvert par le cœur (qui retient le choix : seuls
 * les chemins vraiment choisis sont admis ensuite par les commandes sensibles). */
export async function parcourir(options: { dossier?: boolean; titre: string; extensions?: string[] }): Promise<string | null> {
  if (!isTauri()) return null;
  // L'explorateur de Windows se pilote à la souris : jamais ouvert en Taodbox.
  if (await seulementAuBureau(`Choisir « ${options.titre} »`)) return null;
  return api.parcourir(options);
}

/** Le dossier où Frogtend installe les émulateurs ; demandé la première fois. */
export async function dossierEmulateurs(changer = false): Promise<string | null> {
  if (etat.pc.dossierEmulateurs && !changer) return etat.pc.dossierEmulateurs;
  // Proposé sur un disque qui existe sur CE PC (celui des jeux, sinon le plus libre) — jamais une lettre supposée.
  const p = changer ? null : await api.dossierJeuxPropose().catch(() => null);
  const d = changer ? '' : dossierParDefaut(etat.pc.emplacements.defaut, p?.disque, p?.disques);
  // En Taodbox (pas d'explorateur) : le dossier proposé d'office, comme « Prêt à jouer », annoncé avant l'accord.
  if (auCanape() && !changer) {
    if (!d) {
      await informer('📁 Aucun disque trouvé', 'Frogtend ne trouve aucun disque où installer les émulateurs : règle un dossier à l’écran du PC (⚙ Options ▸ 📁 Emplacements).');
      return null;
    }
    const oui = await confirmer('📁 Où installer les émulateurs ?', {
      message: `Ils iront dans ${d}, chacun dans son sous-dossier. Tu pourras changer ce dossier plus tard, à l’écran du PC (⚙ Options ▸ 📁 Emplacements).`,
      libelleValider: '✅ D’accord',
    });
    if (!oui) return null;
    await reglerPc('dossierEmulateurs', d);
    return d;
  }
  if (!changer && d) {
    const c = await choisir(
      '📁 Où installer les émulateurs ?',
      [
        { valeur: 'propose', libelle: `📁 ${d}`, detail: 'Chaque émulateur y aura son sous-dossier, avec sa configuration.' },
        { valeur: 'autre', libelle: '📂 Choisir un autre dossier…', detail: 'Avec l’explorateur de Windows.' },
      ],
      'Tu pourras changer ce dossier plus tard (⚙ Options ▸ 📁 Emplacements).',
    );
    if (!c) return null;
    if (c === 'propose') {
      await reglerPc('dossierEmulateurs', d);
      return d;
    }
  }
  const ok = await confirmer('📁 Où installer les émulateurs ?', {
    message: `Choisis (ou crée) un dossier pour les émulateurs${d ? `, par exemple ${d}` : ''}.\nChaque émulateur y aura son sous-dossier, avec sa configuration.`,
    libelleValider: '📂 Choisir le dossier',
  });
  if (!ok) return null;
  const choisi = await parcourir({ dossier: true, titre: 'Dossier des émulateurs' });
  if (!choisi) return null;
  await reglerPc('dossierEmulateurs', choisi);
  return choisi;
}

/** Installe (ou met à jour) un émulateur depuis sa source officielle, après avoir annoncé la taille. */
export async function installerEmulateur(
  id: string,
  nom: string,
  miseAJour = false,
  options: { sansQuestion?: boolean } = {},
): Promise<EmulateurInstalle | null> {
  const dossier = await dossierEmulateurs();
  if (!dossier) return null;
  let paquet;
  try {
    paquet = await api.emulateurDerniereVersion(id);
  } catch (e) {
    toast(`Impossible de trouver ${nom} sur son site : ${motifDuRefus(e)}`, 'erreur');
    return null;
  }
  const site = new URL(paquet.url).host;
  // « Prêt à jouer » : la personne a déjà dit oui à une question qui disait tout.
  const oui = options.sansQuestion || await confirmer(`⬇ ${miseAJour ? 'Mettre à jour' : 'Installer'} ${nom} ${paquet.version} ?`, {
    message: [
      `Téléchargement depuis son site officiel (${site}) : ${paquet.taille ? taille(paquet.taille) : 'taille non annoncée'}.`,
      `Dossier : ${dossier}\\${nom}`,
      miseAJour
        ? 'Ta configuration est gardée ; un fichier de réglages remplacé par la nouvelle version est d’abord copié à part.'
        : 'Il sera installé en mode portable : sa configuration reste dans son dossier.',
      ...(id === 'retroarch' ? ['Ses profils de manette officiels (libretro, moins de 1 Mo) seront ajoutés s’ils manquent.'] : []),
    ].join('\n'),
    libelleValider: miseAJour ? '⬆ Mettre à jour' : '⬇ Installer',
  });
  if (!oui) return null;

  Object.assign(installationEmulateur, { id, recus: 0, total: paquet.taille });
  const arreter = isTauri()
    ? await listen<{ id: string; recus: number; total: number | null }>('emulateur', (e) => {
        if (e.payload.id === id) Object.assign(installationEmulateur, { recus: e.payload.recus, total: e.payload.total });
      })
    : () => {};
  toast(`⬇ ${nom} : téléchargement et installation…`);
  try {
    const e = await api.emulateurInstaller(id, dossier);
    toast(`✅ ${nom} ${e.version ?? ''} est installé.`);
    return e;
  } catch (e) {
    toast(`Échec de l’installation de ${nom} : ${motifDuRefus(e)}`, 'erreur');
    return null;
  } finally {
    arreter();
    installationEmulateur.id = null;
  }
}

/**
 * Le micrologiciel PS3 SERVI PAR FIREHOUSE : téléchargé (avancement montré tous les 10 %), vérifié, installé sans
 * fenêtre par RPCS3. L'accord (avec sa taille) est demandé AVANT par l'appelant. Rend la version installée, ou `null`.
 */
export async function installerMicrologicielPs3(programme: string): Promise<string | null> {
  let message = toast('💿 Micrologiciel PS3 : téléchargement…', 'neutre');
  let dernier: number | null = -1;
  const arreter = isTauri()
    ? await listen<{ id: string; recus: number; total: number | null }>('emulateur', (e) => {
        if (e.payload.id !== 'rpcs3-micrologiciel') return;
        const p = palier(e.payload.recus, e.payload.total);
        if (p === dernier || p === null) return;
        dernier = p;
        fermerToast(message);
        message = toast(
          p >= 100
            ? '💿 Micrologiciel PS3 : vérifié, installation dans RPCS3 (sans fenêtre)…'
            : `💿 Micrologiciel PS3 : ${p} % (${taille(e.payload.recus)}${e.payload.total ? ` sur ${taille(e.payload.total)}` : ''})…`,
          'neutre',
        );
      })
    : () => {};
  try {
    const v = await api.rpcs3MicrologicielServi(programme);
    toast(`✅ Micrologiciel PS3 ${v} installé. La première partie de chaque jeu prépare RPCS3 quelques minutes, une seule fois.`, 'ok');
    return v;
  } catch (e) {
    toast(`Micrologiciel PS3 non installé : ${motifDuRefus(e)}. En cas de coupure, il reprendra où il en était.`, 'erreur');
    return null;
  } finally {
    arreter();
    fermerToast(message);
  }
}

/** Quand Firehouse ne recommande rien pour MS-DOS : DOSBox Staging (les jeux « prêts à jouer » ont souvent le leur). */
export function recommandationsParDefaut(plateforme: string): EmulateurRecommande[] {
  if (plateforme === 'MS-DOS') {
    return [{ nom: 'DOSBox Staging', site: 'https://www.dosbox-staging.org/', recommande: true, ligne_de_commande: '', extensions: [], bios: '' }];
  }
  // Firehouse ne recommande rien pour la Switch : Eden, choisi avec Seb (01/10) parmi les émulateurs actuels.
  if (plateforme === 'Nintendo Switch') {
    return [{ nom: 'Eden', site: 'https://eden-emu.dev/', recommande: true, ligne_de_commande: '', extensions: [], bios: '' }];
  }
  return [];
}

/** Le dossier où Frogtend installe Cheat Engine et les autres outils (pas avec les émulateurs) ; demandé la 1re fois. */
export async function dossierOutils(changer = false): Promise<string | null> {
  if (etat.pc.dossierOutils && !changer) return etat.pc.dossierOutils;
  if (await seulementAuBureau('Choisir le dossier des outils')) return null;
  const disque = etat.pc.dossierEmulateurs ? null : (await api.dossierJeuxPropose().catch(() => null))?.disque;
  const suggestion = etat.pc.dossierEmulateurs
    ? `${etat.pc.dossierEmulateurs.replace(/[\\/][^\\/]*$/, '')}\\Outils`
    : disque
      ? `${disque.replace(/\\$/, '')}\\Frogtend\\Outils`
      : 'un dossier de ton choix';
  const ok = await confirmer('📁 Où installer les outils ?', {
    message: `Choisis (ou crée) un dossier pour Cheat Engine et les autres outils, par exemple ${suggestion}.\nChaque outil y aura son sous-dossier ; rien n’est installé ailleurs.`,
    libelleValider: '📂 Choisir le dossier',
  });
  if (!ok) return null;
  const d = await parcourir({ dossier: true, titre: 'Dossier des outils' });
  if (!d) return null;
  await reglerPc('dossierOutils', d);
  return d;
}

/** Installer un émulateur (ou un OUTIL, comme Cheat Engine : il va alors dans le dossier des outils) que Frogtend ne
 * connaît pas en dur, d'après Firehouse (contrat 14), sur accord. */
export async function installerParFirehouse(
  id: string,
  nom: string,
  outil = false,
  options: { sansQuestion?: boolean } = {},
): Promise<EmulateurInstalle | null> {
  const dossier = outil ? await dossierOutils() : await dossierEmulateurs();
  if (!dossier) return null;
  const oui = options.sansQuestion || await confirmer(`⬇ Installer ${nom} ?`, {
    message: [
      `Firehouse donne sa source officielle et sa dernière version ; Frogtend la télécharge et vérifie qu’elle est intacte.`,
      `Dossier : ${dossier}\\${nom}`,
      'Il sera installé en mode portable quand il en a un : sa configuration reste dans son dossier.',
    ].join('\n'),
    libelleValider: '⬇ Installer',
  });
  if (!oui) return null;
  Object.assign(installationEmulateur, { id, recus: 0, total: null });
  const arreter = isTauri()
    ? await listen<{ id: string; recus: number; total: number | null }>('emulateur', (e) => {
        if (e.payload.id === id) Object.assign(installationEmulateur, { recus: e.payload.recus, total: e.payload.total });
      })
    : () => {};
  try {
    const r = await api.emulateurInstallerFirehouse(id, nom, dossier);
    if (r.sorte === 'installe') {
      toast(`✅ ${nom} installé.`);
      return r.emulateur;
    }
    const choix = await choisir(
      `🎯 Quel est le programme de ${nom} ?`,
      r.candidats.map((c) => ({ valeur: c, libelle: c })),
      'Le paquet en contient plusieurs.',
    );
    if (!choix) return null;
    const e = await api.emulateurAdopter(id, `${r.dossier}\\${choix.replace(/\//g, '\\')}`, nom, r.version);
    toast(`✅ ${nom} installé.`);
    return e;
  } catch (e) {
    const motif = motifDuRefus(e);
    // Firehouse dit parfois quelle page ouvrir (source muette, installation à la main) : on la propose.
    const page = /https?:\/\/[^\s)»"]+/.exec(motif)?.[0];
    if (page) {
      const ouvrir = await confirmer(`⚠ ${nom} ne s’installe pas tout seul`, {
        message: `${motif}\n\nOuvrir sa page officielle pour l’installer toi-même ? Ensuite, ➕ Ajouter un émulateur ▸ « Un autre émulateur… » pour montrer son programme.`,
        libelleValider: '🌐 Ouvrir la page',
      });
      if (ouvrir) {
        const { openUrl } = await import('@tauri-apps/plugin-opener');
        await openUrl(page).catch(() => {});
      }
    } else {
      toast(`Impossible d’installer ${nom} : ${motif}`, 'erreur');
    }
    return null;
  } finally {
    arreter();
    installationEmulateur.id = null;
  }
}

/**
 * RetroArch : installe le cœur qui manque (sur accord, sauf `sansQuestion`), signale les fichiers système manquants, et
 * rend son nom complet (« RetroArch — snes9x » : un cœur = un émulateur), ou `null` si le cœur n'a pas pu être installé.
 */
export async function completerRetroArch(
  programme: string,
  ligne: string,
  bios: string[],
  plateforme: string,
  nom: string,
  sansQuestion = false,
): Promise<string | null> {
  const e = await api.retroarchEtat(programme, ligne, bios).catch(() => null);
  if (e?.coeur && !e.coeur_present) {
    const oui =
      sansQuestion ||
      (await confirmer(`🧩 Installer le cœur « ${e.coeur} » ?`, {
        message: `RetroArch a besoin de ce cœur pour ${plateforme}. Firehouse le fournit, vérifié (quelques Mo).`,
        libelleValider: '⬇ Installer le cœur',
      }));
    if (!oui) return null;
    try {
      await api.retroarchInstallerCoeur(programme, e.coeur);
      toast(`✅ Cœur ${e.coeur} installé.`);
    } catch (err) {
      toast(`Échec de l’installation du cœur : ${motifDuRefus(err)}`, 'erreur');
      return null;
    }
  }
  if (e && e.bios_manquants.length) {
    await informer(
      '⚠ Fichier système manquant',
      `${plateforme} a besoin de : ${e.bios_manquants.join(', ')}.\n\nPlace ce fichier (tel quel, sans le renommer) dans :\n${e.dossier_bios}\n\nFirehouse ne le fournit pas (encore) pour cet émulateur.`,
    );
  }
  const coeur = e?.coeur ?? (await api.retroarchEtat(programme, ligne, []).catch(() => null))?.coeur;
  return coeur ? `${nom} — ${coeur.replace(/_libretro.dll$/i, '')}` : nom;
}

/** Pose les fichiers système d'une console (cœur : vérifiés, jamais un fichier remplacé) et dit ce qui reste à faire. */
export async function poserBios(plateforme: string, programme: string): Promise<void> {
  let b: BilanBios;
  try {
    b = await api.biosPoser(plateforme, programme);
  } catch (e) {
    toast(`Fichiers système de ${plateforme} non posés : ${motifDuRefus(e)}`, 'alerte');
    return;
  }
  if (b.poses.length) toast(`📀 ${b.poses.length > 1 ? `${b.poses.length} fichiers système posés` : 'Fichier système posé'} (${b.poses.join(', ')}).`, 'ok');
  if (b.echecs.length) toast(`Fichiers système non posés : ${b.echecs.join(' ; ')}`, 'alerte');
  if (b.a_poser.length) {
    await informer(
      `📀 ${plateforme} : ${b.a_poser.length > 1 ? 'des fichiers système manquent' : 'un fichier système manque'}`,
      `Firehouse ne les fournit pas (encore) : ${b.a_poser.join(', ')}.\n\nSans eux, certains jeux ne démarrent pas. Frogtend te le redira au besoin.`,
    );
  }
  if (b.cles.length) {
    await informer(
      `🔑 Il manque tes clés pour ${plateforme}`,
      `${b.cles.join(', ')} : ce sont TES clés, tirées de ta propre console. Personne ne peut les fournir, ni Firehouse ni Frogtend. Sans elles, les jeux ne démarrent pas.`,
    );
  }
}

/** Ajouter un émulateur (ou un cœur RetroArch) à un système. Le premier devient celui par défaut. Rend la clé de
 * l'émulateur retenu, ou `null`. */
export async function reglerEmulateur(plateforme: string): Promise<string | null> {
  const reponse = await api.emulateursRecommandes(plateforme).catch(() => ({ emulateurs: [], conseil: null }));
  const conseil = reponse.conseil ?? null;
  // L'émulateur du conseil de la maison en tête (contrat 1.8 : il prime sur l'avis LaunchBox), même absent de la liste.
  let recs = avecConseil(reponse.emulateurs ?? [], conseil);
  if (recs.length === 0) recs = recommandationsParDefaut(plateforme);
  const installes = await api.emulateursInstalles().catch(() => []);

  type Option = { rec: EmulateurRecommande; id: string | null; nom: string; installe?: EmulateurInstalle } | 'autre';
  const options: { valeur: Option; libelle: string; detail?: string }[] = [];
  // Les forks actifs décrits par Firehouse (contrat 14) s'ajoutent après leur émulateur d'origine.
  const avecForks: EmulateurRecommande[] = recs.flatMap((r) => [
    r,
    ...(r.forks ?? []).filter((f) => f.actif !== false && !estConseille(f, conseil)).map((f) => ({ ...f, recommande: false })),
  ]);
  for (const rec of avecForks) {
    const f = await api.emulateurFiche(rec.nom).catch(() => null);
    const installe = f ? installes.find((i) => i.id === f.id) : rec.id ? installes.find((i) => i.id === rec.id) : undefined;
    const parFirehouse = !f && !!rec.id && ((!!rec.telechargement && rec.telechargement.type !== 'page') || estConseille(rec, conseil));
    // « À installer toi-même » finit par l'explorateur de Windows : pas proposé en Taodbox.
    if (auCanape() && !installe && !f && !parFirehouse) continue;
    options.push({
      valeur: { rec, id: f?.id ?? null, nom: f?.nom ?? rec.nom, installe },
      libelle: conseil
        ? `${estConseille(rec, conseil) ? '⭐ ' : ''}${f?.nom ?? rec.nom}${estConseille(rec, conseil) ? ' (conseillé par la maison)' : ''}`
        : `${rec.recommande ? '⭐ ' : ''}${f?.nom ?? rec.nom}${rec.recommande ? ' (recommandé)' : ''}`,
      detail: installe
        ? `✅ déjà sur ce PC${installe.version ? ` (${installe.version})` : ''}${estConseille(rec, conseil) && versionAutre(installe.version, conseil) ? ` ⚠️ les jeux de Firehouse attendent la ${versionAutre(installe.version, conseil)}` : ''}`
        : f || parFirehouse
          ? `⬇ Frogtend peut l’installer${parFirehouse ? ' (source donnée par Firehouse)' : ''}`
          : `à installer toi-même${rec.site_officiel || rec.site ? ` : ${rec.site_officiel || rec.site}` : ''}`,
    });
  }
  // Montrer un programme et écrire une ligne de commande : pas en Taodbox.
  if (!auCanape()) options.push({ valeur: 'autre', libelle: '📂 Un autre émulateur…', detail: 'montrer son programme et écrire sa ligne de commande' });

  const c = await choisir<Option>(
    `🕹 Quel émulateur pour ${plateforme} ?`,
    options,
    recs.length ? 'Recommandations de Firehouse.' : undefined,
  );
  if (!c) return null;

  let programme: string | null = null;
  let nom = 'l’émulateur';
  let ligne = '';
  let bios: string[] = [];
  let estRetroArch = false;
  let depuisConseil: string | undefined;

  if (c === 'autre') {
    programme = await parcourir({ titre: 'Programme de l’émulateur', extensions: ['exe'] });
    if (!programme) return null;
    const nomDonne = await demander('🏷 Nom de cet émulateur', { message: 'Le nom montré dans Frogtend (par exemple « mGBA »).' });
    if (!nomDonne) return null;
    nom = nomDonne;
    const l = await demander('⌨ Ligne de commande', {
      message: 'Le fichier du jeu est ajouté à la fin (ou à la place de %rom%). Laisse vide si l’émulateur n’a besoin de rien d’autre.',
    });
    if (l === null) return null;
    ligne = l;
  } else {
    nom = c.nom;
    bios = biosRetroArch(c.rec, conseil);
    estRetroArch = c.id === 'retroarch';
    const fiche = c.id ? await api.emulateurFiche(c.rec.nom).catch(() => null) : null;
    ligne = ligneDe(c.rec, fiche?.ligne ?? '', conseil);
    if (estConseille(c.rec, conseil)) depuisConseil = conseil!.maj_le;
    if (c.installe) {
      programme = c.installe.programme;
      if (!c.installe.installe_le && c.id) await api.emulateurAdopter(c.id, programme).catch(() => {});
    } else if (c.id) {
      const e = await installerEmulateur(c.id, c.nom);
      if (!e) return null;
      programme = e.programme;
    } else if (c.rec.id && ((c.rec.telechargement && c.rec.telechargement.type !== 'page') || estConseille(c.rec, conseil))) {
      const e = await installerParFirehouse(c.rec.id, c.nom);
      if (!e) return null;
      programme = e.programme;
    } else {
      const ok = await confirmer(`📂 Où est ${nom} ?`, {
        message: `Frogtend ne sait pas encore installer ${nom}.${c.rec.site ? ` Site officiel : ${c.rec.site}` : ''}\nMontre son programme une fois qu’il est installé.`,
        libelleValider: '📂 Choisir le programme',
      });
      if (!ok) return null;
      programme = await parcourir({ titre: `Programme de ${nom}`, extensions: ['exe'] });
      if (!programme) return null;
    }
  }

  // L'émulateur du conseil : ses fichiers système, posés par le cœur après UN accord chiffré (jamais sans le dire).
  if (programme && depuisConseil && conseil && biosADireOuPoser(conseil.bios)) {
    const servis = biosATelecharger(conseil.bios);
    const poids = poidsBios(conseil.bios);
    // Rien à télécharger (seulement des clés ou des fichiers à poser soi-même) : on le dit sans rien demander.
    const oui =
      !servis.length ||
      (await confirmer(`📀 Poser les fichiers système de ${plateforme} ?`, {
        message: `Firehouse les fournit (${servis.map((b) => b.fichier).join(', ')}${poids ? `, ${taille(poids)}` : ''}) ; Frogtend les vérifie et les pose là où ${nom} les cherche. Un fichier déjà là n’est jamais remplacé.`,
        libelleValider: '📀 Les poser',
      }));
    if (oui) await poserBios(plateforme, programme);
  }
  if (estRetroArch && programme) {
    const n = await completerRetroArch(programme, ligne, bios, plateforme, nom);
    if (!n) return null;
    nom = n;
  }
  if (!programme) return null;
  const { sys, cle } = ajouter(etat.pc.emulateurs[plateforme], { nom, programme, ligne, ...(depuisConseil ? { conseil: depuisConseil } : {}) });
  await reglerPc('emulateurs', { ...etat.pc.emulateurs, [plateforme]: sys });
  // Des parties restaurées (PC neuf, formaté) attendaient cet émulateur : reposées maintenant, jamais par-dessus une autre.
  const r = await api.restaurationReposerEmulateurs().catch(() => null);
  const parties = r?.reposes ? ` ${r.reposes} partie(s) restaurée(s) reposée(s).` : '';
  toast((sys.liste.length > 1 ? `✅ ${nom} ajouté pour ${plateforme}.` : `✅ ${nom} réglé pour ${plateforme}.`) + parties);
  return cle;
}
