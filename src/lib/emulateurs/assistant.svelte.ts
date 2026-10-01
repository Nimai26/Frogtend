// L'assistant des émulateurs : pour un système, proposer l'émulateur recommandé par Firehouse, l'installer depuis sa
// source officielle (sur accord, taille annoncée) ou reprendre celui déjà présent, installer le cœur RetroArch qui
// manque, vérifier les BIOS, puis retenir le réglage du système.

import { listen } from '@tauri-apps/api/event';
import { ajouter } from './choix';
import { isTauri } from '@tauri-apps/api/core';
import { api, taille, type EmulateurInstalle, type EmulateurRecommande } from '$lib/api';
import { choisir, confirmer, demander, informer, toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { etat, reglerPc } from '$lib/etat.svelte';

export const installationEmulateur = $state<{ id: string | null; recus: number; total: number | null }>({
  id: null,
  recus: 0,
  total: null,
});

/** Choisir un fichier ou un dossier avec l'explorateur de Windows. */
export async function parcourir(options: { dossier?: boolean; titre: string; extensions?: string[] }): Promise<string | null> {
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

/** Le dossier où Frogtend installe les émulateurs ; demandé la première fois. */
export async function dossierEmulateurs(changer = false): Promise<string | null> {
  if (etat.pc.dossierEmulateurs && !changer) return etat.pc.dossierEmulateurs;
  const suggestion = etat.pc.emplacements.defaut[0] ? `${etat.pc.emplacements.defaut[0]}\\Émulateurs` : 'E:\\Émulateurs';
  const ok = await confirmer('📁 Où installer les émulateurs ?', {
    message: `Choisis (ou crée) un dossier pour les émulateurs, par exemple ${suggestion}.\nChaque émulateur y aura son sous-dossier, avec sa configuration (mode portable).`,
    libelleValider: '📂 Choisir le dossier',
  });
  if (!ok) return null;
  const d = await parcourir({ dossier: true, titre: 'Dossier des émulateurs' });
  if (!d) return null;
  await reglerPc('dossierEmulateurs', d);
  return d;
}

/** Installe (ou met à jour) un émulateur depuis sa source officielle, après avoir annoncé la taille. */
export async function installerEmulateur(id: string, nom: string, miseAJour = false): Promise<EmulateurInstalle | null> {
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
  const oui = await confirmer(`⬇ ${miseAJour ? 'Mettre à jour' : 'Installer'} ${nom} ${paquet.version} ?`, {
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

/** Quand Firehouse ne recommande rien pour MS-DOS : DOSBox Staging (les jeux « prêts à jouer » ont souvent le leur). */
function recommandationsParDefaut(plateforme: string): EmulateurRecommande[] {
  if (plateforme === 'MS-DOS') {
    return [{ nom: 'DOSBox Staging', site: 'https://www.dosbox-staging.org/', recommande: true, ligne_de_commande: '', extensions: [], bios: '' }];
  }
  // Firehouse ne recommande rien pour la Switch : Eden, choisi avec Seb (01/10) parmi les émulateurs actuels.
  if (plateforme === 'Nintendo Switch') {
    return [{ nom: 'Eden', site: 'https://eden-emu.dev/', recommande: true, ligne_de_commande: '', extensions: [], bios: '' }];
  }
  return [];
}

/** Installer un émulateur que Frogtend ne connaît pas en dur, d'après Firehouse (contrat 14), sur accord. */
export async function installerParFirehouse(id: string, nom: string): Promise<EmulateurInstalle | null> {
  const dossier = await dossierEmulateurs();
  if (!dossier) return null;
  const oui = await confirmer(`⬇ Installer ${nom} ?`, {
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
    toast(`Impossible d’installer ${nom} : ${motifDuRefus(e)}`, 'erreur');
    return null;
  } finally {
    arreter();
    installationEmulateur.id = null;
  }
}

/** Ajouter un émulateur (ou un cœur RetroArch) à un système. Le premier devient celui par défaut. Rend la clé de
 * l'émulateur retenu, ou `null`. */
export async function reglerEmulateur(plateforme: string): Promise<string | null> {
  let recs = (await api.emulateursRecommandes(plateforme).catch(() => ({ emulateurs: [] }))).emulateurs ?? [];
  if (recs.length === 0) recs = recommandationsParDefaut(plateforme);
  const installes = await api.emulateursInstalles().catch(() => []);

  type Option = { rec: EmulateurRecommande; id: string | null; nom: string; installe?: EmulateurInstalle } | 'autre';
  const options: { valeur: Option; libelle: string; detail?: string }[] = [];
  // Les forks actifs décrits par Firehouse (contrat 14) s'ajoutent après leur émulateur d'origine.
  const avecForks: EmulateurRecommande[] = recs.flatMap((r) => [r, ...(r.forks ?? []).filter((f) => f.actif !== false).map((f) => ({ ...f, recommande: false }))]);
  for (const rec of avecForks) {
    const f = await api.emulateurFiche(rec.nom).catch(() => null);
    const installe = f ? installes.find((i) => i.id === f.id) : rec.id ? installes.find((i) => i.id === rec.id) : undefined;
    const parFirehouse = !f && !!rec.id && !!rec.telechargement && rec.telechargement.type !== 'page';
    options.push({
      valeur: { rec, id: f?.id ?? null, nom: f?.nom ?? rec.nom, installe },
      libelle: `${rec.recommande ? '⭐ ' : ''}${f?.nom ?? rec.nom}${rec.recommande ? ' (recommandé)' : ''}`,
      detail: installe
        ? `✅ déjà sur ce PC${installe.version ? ` (${installe.version})` : ''}`
        : f || parFirehouse
          ? `⬇ Frogtend peut l’installer${parFirehouse ? ' (source donnée par Firehouse)' : ''}`
          : `à installer toi-même${rec.site_officiel || rec.site ? ` : ${rec.site_officiel || rec.site}` : ''}`,
    });
  }
  options.push({ valeur: 'autre', libelle: '📂 Un autre émulateur…', detail: 'montrer son programme et écrire sa ligne de commande' });

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

  if (c === 'autre') {
    programme = await parcourir({ titre: 'Programme de l’émulateur', extensions: ['exe'] });
    if (!programme) return null;
    const nomDonne = await demander('🏷 Nom de cet émulateur', { message: 'Le nom montré dans Frogtend (par exemple « mGBA »).' });
    if (!nomDonne) return null;
    nom = nomDonne;
    const l = await demander('⌨ Ligne de commande', {
      message: 'Le fichier du jeu est ajouté à la fin. Laisse vide si l’émulateur n’a besoin de rien d’autre.',
    });
    if (l === null) return null;
    ligne = l;
  } else {
    nom = c.nom;
    bios = c.rec.bios ? [c.rec.bios] : [];
    estRetroArch = c.id === 'retroarch';
    const fiche = c.id ? await api.emulateurFiche(c.rec.nom).catch(() => null) : null;
    ligne = c.rec.ligne_de_commande || fiche?.ligne || '';
    if (c.installe) {
      programme = c.installe.programme;
      if (!c.installe.installe_le && c.id) await api.emulateurAdopter(c.id, programme).catch(() => {});
    } else if (c.id) {
      const e = await installerEmulateur(c.id, c.nom);
      if (!e) return null;
      programme = e.programme;
    } else if (c.rec.id && c.rec.telechargement && c.rec.telechargement.type !== 'page') {
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

  // RetroArch : son cœur, ses BIOS.
  if (estRetroArch && programme) {
    const e = await api.retroarchEtat(programme, ligne, bios).catch(() => null);
    if (e?.coeur && !e.coeur_present) {
      const oui = await confirmer(`🧩 Installer le cœur « ${e.coeur} » ?`, {
        message: `RetroArch a besoin de ce cœur pour ${plateforme}. Il vient du site officiel de libretro (quelques Mo).`,
        libelleValider: '⬇ Installer le cœur',
      });
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
        '⚠ BIOS manquant',
        `${plateforme} a besoin de : ${e.bios_manquants.join(', ')}.\n\nPlace ce fichier (tel quel, sans le renommer) dans :\n${e.dossier_bios}\n\nFrogtend ne télécharge pas les BIOS : ils viennent de ta console.`,
      );
    }
  }

  // RetroArch : un cœur = un émulateur (« RetroArch — snes9x »).
  if (estRetroArch && programme) {
    const coeur = (await api.retroarchEtat(programme, ligne, []).catch(() => null))?.coeur;
    if (coeur) nom = `${nom} — ${coeur.replace(/_libretro.dll$/i, '')}`;
  }
  if (!programme) return null;
  const { sys, cle } = ajouter(etat.pc.emulateurs[plateforme], { nom, programme, ligne });
  await reglerPc('emulateurs', { ...etat.pc.emulateurs, [plateforme]: sys });
  toast(sys.liste.length > 1 ? `✅ ${nom} ajouté pour ${plateforme}.` : `✅ ${nom} réglé pour ${plateforme}.`);
  return cle;
}
