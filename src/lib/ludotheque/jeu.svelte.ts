// Les actions sur un jeu du PC : l'installer, choisir ce qui le lance, y jouer, régler l'émulateur de son système,
// mettre ses parties à l'abri, le retirer du PC. Chaque action qui touche au disque est expliquée et acceptée d'abord.

import { listen } from '@tauri-apps/api/event';
import { goto } from '$app/navigation';
import { isTauri } from '@tauri-apps/api/core';
import { api, duree, estErreurCoeur, taille, type ApercuNuage, type Candidat, type JeuPc, type ModInstalle, type PatchsPs3, type VerificationFiger } from '$lib/api';
import { attendreTouche, choisir, confirmer, fermerToast, informer, toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { completerRetroArch, installerFirmwareSwitch, poserBios, reglerEmulateur } from '$lib/emulateurs/assistant.svelte';
import { avecConseil, biosADireOuPoser, biosRetroArch, changementDeConseil, estConseille, firmwareSwitchServi, poidsBios, questionFirmwareSwitch } from '$lib/emulateurs/pret';
import { preparationEnCours, preparerPourJouer } from '$lib/emulateurs/pret.svelte';
import { libelleValeur, REGLAGES_IMAGE, reglagesMontres } from '$lib/emulateurs/imagePs3';
import { detailPatch, nomDuPatch, remarqueDuPatch, titreDuJeu } from '$lib/emulateurs/patchsPs3';
import { adresseWeb, avertissements, fichierInstallable, pageNexus, raisonSansClic, type ModJeu } from '$lib/mods/mods';
import { ouvrirSurNexus } from '$lib/mods/nexus.svelte';
import { avecProgres } from '$lib/mods/progres.svelte';
import { accordModSteam } from '$lib/mods/cheatengine';
import { groupeDe, libelleCopie } from '$lib/boutiques/copies';
import { nomPlateforme } from '$lib/boutiques/galaxy.svelte';
import { ouvrirPageWeb } from '$lib/web';
import { estRetroArch, normaliser, pourLeJeu } from '$lib/emulateurs/choix';
import { etat, reglerPc, reglerProfil } from '$lib/etat.svelte';
import type { CommandesJeu } from '$lib/reglages/reglages';
import { GENRES_MANETTE, libelleCommandes } from './commandes';
import { apresUnePartie } from '$lib/sauvegarde.svelte';
import { ludo, rechargerListe, rechargerPlateformes } from './ludotheque.svelte';
import { debitConnu, mettreDansLaLudotheque, rechargerJeuxDuPc, tele } from './telechargements.svelte';
import { aCartes, choixDeCartes } from './cartes';
import { estCourante, libelleVersion, nomDuFichier } from './versions';
import { libellePleinEcran, TOUCHES_PLEIN_ECRAN } from './plein-ecran';
import { BOUTONS, detailTouche, libelleClavier, memes, nettoyer, PREREGLAGES, toucheDepuisEvenement, TOUCHES, type Correspondance } from './clavier';
import { auCanape, seulementAuBureau } from '$lib/taodbox/canape';
import { nomAffiche } from './ludotheque.svelte';
import { proposerCopieAvantMod } from '$lib/mods/copieAvantMod';

export const partie = $state<{ enJeu: number | null; installation: number | null; lancement: number | null }>({ enJeu: null, installation: null, lancement: null });

let ecoute: (() => void) | null = null;

/** Écoute les débuts et fins de partie (une seule fois). */
export async function suivreParties() {
  if (ecoute || !isTauri()) return;
  // Un jeu en administrateur ne reçoit pas les touches de la manette : le dire (une fois par partie).
  await listen<{ message: string }>('clavier', (e) => toast(`🕹 ${e.payload.message}.`, 'alerte'));
  // Le plein écran au lancement n'a pas pu être appliqué : le dire, avec la piste (l'autre façon).
  await listen<{ jeu: number; message: string }>('plein-ecran', (e) => {
    const titre = tele.jeux[e.payload.jeu]?.titre ?? 'le jeu';
    toast(`🖥 Frogtend n’a pas pu mettre « ${titre} » en plein écran (${e.payload.message}). Essaie l’autre façon dans ⚙ Gérer le jeu ▸ Plein écran.`, 'alerte');
  });
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

/** Choisir un fichier ou un dossier avec l'explorateur de Windows, ouvert par le cœur (qui retient le choix). */
async function parcourir(options: { dossier?: boolean; titre: string; extensions?: string[] }): Promise<string | null> {
  if (!isTauri()) return null;
  // L'explorateur de Windows se pilote à la souris : jamais ouvert en Taodbox.
  if (await seulementAuBureau(`Choisir « ${options.titre} »`)) return null;
  return api.parcourir(options);
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
        // L'installeur de Windows se pilote à la souris : pas en Taodbox.
        ...(auCanape() ? [] : [{ valeur: 'guidee', libelle: '🧭 Installation guidée', detail: 'l’installeur s’ouvre, tu choisis toi-même' }]),
      ],
      `L’installeur (${p.methode.format === 'inno' ? 'Inno Setup' : 'NSIS'}) peut demander l’accord de Windows (droits administrateur).${notes}`,
    );
    if (!c) return false;
    automatique = c === 'auto';
  } else if (p.methode.sorte === 'installeur_guide') {
    if (await seulementAuBureau(`L’installeur de « ${p.titre} »`)) return false;
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
  // Le programme est-il déjà choisi (jeu PC en archive : Firehouse dit lequel lancer, contrat 1.23) ?
  let lanceurDonne = false;
  try {
    if (p.methode.sorte === 'archive') {
      // Un jeu en archive : la décompression montre son avancement (un gros jeu prend des minutes).
      lanceurDonne = !!(await avecProgres(`📦 Installation de « ${p.titre} »`, false, (suivi) => api.installer(id, automatique, suivi)))?.lanceur;
    } else {
      if (p.methode.sorte !== 'aucune') toast(`📦 Installation de « ${p.titre} »… (ça peut prendre quelques minutes)`);
      lanceurDonne = !!(await api.installer(id, automatique))?.lanceur;
    }
  } catch (e) {
    partie.installation = null;
    if (estErreurCoeur(e) && e.motif.includes("rien n'est dans")) {
      // Installé ailleurs : la personne indique où (avec l'explorateur de Windows : pas en Taodbox).
      if (await seulementAuBureau(`Montrer où « ${p.titre} » a été installé`)) return false;
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
    const plateforme = tele.jeux[id]?.plateforme;
    // Un jeu Switch servi avec ses mises à jour et DLC (contrat 1.23) : Eden ne les lit qu'après un oui, dans le bloc
    // « 📦 » de sa fiche (rien n'est écrit chez Eden sans lui).
    const avecContenus = plateforme === 'Nintendo Switch' && (tele.jeux[id]?.fichiers.length ?? 0) > 1;
    toast(
      `✅ « ${p.titre} » est prêt : il se lance par l’émulateur de son système.${avecContenus ? ' Ses mises à jour et DLC sont dans le bloc « 📦 » de sa fiche : un clic pour qu’Eden les lise.' : ''}`,
    );
    // Sa console n'a pas encore d'émulateur : le proposer tout de suite (un seul accord).
    if (plateforme) await preparerPourJouer(plateforme, 1);
    return true;
  }
  if (lanceurDonne) {
    toast(`✅ « ${p.titre} » est installé et prêt à jouer.`);
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
  // En Taodbox, « un autre programme » ouvrirait l'explorateur de Windows : seulement les candidats trouvés.
  if (auCanape() && !candidats.length) {
    await seulementAuBureau(`Choisir ce qui lance « ${titre} »`);
    return false;
  }
  const c = await choisir<Candidat | 'autre'>(
    `🎯 Qu’est-ce qui lance « ${titre} » ?`,
    [
      ...candidats.slice(0, 8).map((c, i) => ({
        valeur: c as Candidat | 'autre',
        libelle: `${i === 0 ? '⭐ ' : ''}${c.relatif}`,
        detail: i === 0 ? 'le plus probable' : undefined,
      })),
      ...(auCanape() ? [] : [{ valeur: 'autre' as const, libelle: '📂 Un autre programme…', detail: 'le choisir avec l’explorateur' }]),
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

/** Les consoles dont le conseil a déjà été regardé pendant cette séance (une seule question par console ; un échec
 * hors ligne n'est retenté qu'à la séance suivante). */
const conseilsVus = new Set<string>();

/** Une réponse attendue au plus `ms` : sinon `null` (le jeu ne doit jamais attendre le réseau pour démarrer). */
function auPlus<T>(ms: number, p: Promise<T>): Promise<T | null> {
  return Promise.race([p.catch(() => null), new Promise<null>((r) => setTimeout(() => r(null), ms))]);
}

/**
 * Le conseil de la maison pour cette console a-t-il changé depuis le réglage de son émulateur (contrat 1.8, `maj_le`) ?
 * Même émulateur : PROPOSER le nouveau réglage (un autre cœur, installé alors) ; autre émulateur : proposer de le
 * préparer (l'actuel reste). Une réponse, même « non », retient la date : la question ne revient pas pour ce conseil.
 * Hors ligne ou réglé à la main : rien.
 */
async function suivreConseil(id: number, plateforme: string | undefined, emulateur?: string) {
  if (!plateforme || conseilsVus.has(plateforme)) return;
  const s = normaliser(etat.pc.emulateurs[plateforme]);
  const e = (emulateur && s.liste.find((x) => x.cle === emulateur)) || pourLeJeu(etat.pc.emulateurs, etat.pc.emulateursJeux, plateforme, id);
  if (!e?.conseil) {
    conseilsVus.add(plateforme);
    return;
  }
  // Une réponse qui n'arrive pas à temps (hors ligne, Firehouse lent) : rien n'est décidé, réessayé au lancement suivant
  // (une ligne à réparer ne reste pas en place pour toute la séance).
  const r = await auPlus(3000, api.emulateursRecommandes(plateforme));
  if (!r) return;
  const c = r.conseil ?? null;
  // Le conseillé comme le trouve « Prêt à jouer » (même absent de la liste, ou fork) : sa fiche Frogtend (lue dans le
  // catalogue de Frogtend, sans réseau ; `null` = émulateur que Frogtend ne connaît pas) donne les options de Frogtend
  // (plein écran…), sans lesquelles la ligne paraîtrait toujours changée.
  const rec = c ? avecConseil(r.emulateurs ?? [], c).find((x) => estConseille(x, c)) : undefined;
  const fiche = rec ? await api.emulateurFiche(rec.nom).catch(() => null) : null;
  conseilsVus.add(plateforme);
  if (!c) return;
  const ch = changementDeConseil(e, c, fiche?.ligne ?? '');
  if (!ch) return;
  // La date est retenue sur l'émulateur réglé (même si la maison en conseille un autre) : la question ne revient pas.
  const retenir = async (patch: { ligne?: string; nom?: string }) => {
    const sys = normaliser(etat.pc.emulateurs[plateforme]);
    const liste = sys.liste.map((x) => (x.cle === e.cle ? { ...x, ...patch, conseil: c.maj_le } : x));
    await reglerPc('emulateurs', { ...etat.pc.emulateurs, [plateforme]: { ...sys, liste } });
  };
  if (ch.sorte === 'date') return retenir({});
  // Une ligne doublée par Frogtend lui-même (contrat 1.16, avant l'adaptation) : remise d'aplomb sans question.
  if (ch.sorte === 'reparer') return retenir({ ligne: ch.ligne });
  if (ch.sorte === 'ligne') {
    const poids = poidsBios(c.bios);
    const oui = await confirmer(`🕹 Nouveau réglage conseillé pour ${nomAffiche(plateforme)}`, {
      message: `La maison a mis à jour son conseil pour ${e.nom}${c.coeur ? ` (cœur « ${c.coeur} », quelques Mo, vérifié)` : ''}${biosADireOuPoser(c.bios) && poids ? `, avec ses fichiers système (${taille(poids)})` : ''}.${c.note ? `\n\n${c.note}` : ''}\n\nL’appliquer maintenant ? Sinon, ${e.nom} garde son réglage actuel.`,
      libelleValider: '✅ Appliquer',
    });
    let nom: string | undefined;
    if (oui) {
      // Les fichiers système du conseil (vérifiés), puis le nouveau cœur, AVANT de changer le réglage : sinon rien ne change.
      if (biosADireOuPoser(c.bios)) await poserBios(plateforme, e.programme);
      if (estRetroArch(e.programme)) {
        const n = await completerRetroArch(e.programme, ch.ligne, rec ? biosRetroArch(rec, c) : [], plateforme, e.nom, true);
        if (!n) return;
        nom = n;
      }
    }
    return retenir(oui ? { ligne: ch.ligne, ...(nom ? { nom } : {}) } : {});
  }
  const nom = rec?.nom ?? c.emulateur;
  const oui = await confirmer(`🕹 La maison conseille maintenant ${nom} pour ${nomAffiche(plateforme)}`, {
    message: `Ton émulateur actuel (${e.nom}) reste disponible, et ce jeu se lance avec lui maintenant.${c.note ? `\n\n${c.note}` : ''}\n\nPréparer ${nom} pour plus tard ?`,
    libelleValider: `🕹 Préparer ${nom}`,
  });
  await retenir({});
  if (oui) await reglerEmulateur(plateforme);
}

/**
 * RetroArch ouvre son port de commandes à tout le réseau (RetroArch 1.22.2 : aucun réglage « sur la machine
 * seulement ») : Windows peut demander d'autoriser l'accès réseau. Décision de Seb (16/10) : garder le comportement
 * de RetroArch et PRÉVENIR, une fois par PC, avant le premier jeu RetroArch. Frogtend ne répond jamais à la place.
 */
async function prevenirPareFeu(id: number, plateforme: string | undefined, emulateur?: string) {
  if (etat.pc.avertiPareFeuRetroArch || !plateforme) return;
  const s = normaliser(etat.pc.emulateurs[plateforme]);
  const e = (emulateur && s.liste.find((x) => x.cle === emulateur)) || pourLeJeu(etat.pc.emulateurs, etat.pc.emulateursJeux, plateforme, id);
  if (!e || !estRetroArch(e.programme)) return;
  await informer(
    '🛡 Une question de Windows va peut-être s’afficher',
    'Au premier jeu, Windows peut demander : « Voulez-vous autoriser les réseaux publics et privés à accéder à cette application ? » pour RetroArch.\n\nRéponds « Annuler » : Frogtend n’en a pas besoin, tout fonctionne quand même (le menu en jeu compris), et rien n’est ouvert aux autres appareils du réseau.\n\nSi tu joues sur la télé à la manette et ne peux pas y répondre, ce n’est pas grave : le jeu marche quand même, la question attendra sur l’écran du PC.',
  );
  await reglerPc('avertiPareFeuRetroArch', true);
}

/** À l'ouverture de Frogtend ou de Taodbox : le bouton Xbox ouvre la Game Bar de Windows (et ses jeux récents : un
 * appui sur A y a lancé RPCS3 pendant une partie SNES, essai de Seb du 28/10). Demandé UNE fois par PC (✅ Seb 28/10
 * soir « Demander à l'ouverture », même sans manette détectée : la page ne « voit » une manette qu'après un appui sur
 * un de ses boutons) : un réglage de Windows, changé seulement après le oui, réversible. */
let gameBarDemandee = false;

export async function proposerCouperGameBar() {
  // Jamais deux fois (l'effet du profil ouvert peut repasser avant la réponse), jamais pendant une partie (la fenêtre
  // serait derrière le jeu) : la question reviendra à la prochaine ouverture.
  if (etat.pc.questionGameBar || gameBarDemandee || partie.enJeu !== null || partie.lancement !== null) return;
  gameBarDemandee = true;
  const ouvre = await api.gameBarBouton().catch(() => null);
  if (ouvre !== true) return;
  const oui = await confirmer('🎮 Le bouton Xbox ouvre la Game Bar de Windows', {
    message:
      'Pendant une partie, un appui sur le bouton Xbox (le logo au milieu de la manette) ouvre la Game Bar de Windows par-dessus le jeu, et un appui sur A peut y lancer un autre programme.\n\nFrogtend peut couper ce bouton (un réglage de Windows : « Ouvrir la Game Bar avec ce bouton d’une manette »). La Game Bar reste disponible au clavier (touche Windows + G), et tu pourras remettre le bouton dans les paramètres de Windows (Jeux ▸ Game Bar).\n\nSi Steam est installé, ce bouton peut aussi ouvrir Steam : cela se règle dans les paramètres de Steam (rubrique Manette), Frogtend n’y touche pas.',
    libelleValider: '🎮 Couper le bouton',
    // Le focus sur « Annuler » : un A répété au canapé ne change jamais un réglage de Windows sans lecture.
    danger: true,
  });
  await reglerPc('questionGameBar', true);
  if (!oui) return;
  try {
    await api.gameBarBoutonRegler(false);
    toast('✅ Le bouton Xbox n’ouvre plus la Game Bar.');
  } catch (e) {
    toast(`Le réglage de Windows n’a pas pu être changé : ${motifDuRefus(e)}. Tu peux le faire dans les paramètres de Windows (Jeux ▸ Game Bar).`, 'alerte');
  }
}

/** Jouer. `emulateur` : la clé d'un émulateur du système, pour ce lancement seulement. Si un réglage manque
 * (émulateur, lanceur), on le demande puis on relance. */
/** La copie d'une CARTE à utiliser (A6c) : un jeu possédé sur plusieurs boutiques et installé de plusieurs côtés →
 * Frogtend DEMANDE laquelle, à chaque fois (Seb, 21/10) ; sinon la copie installée (ou le jeu lui-même). `null` :
 * annulé. */
export async function choisirCopie(id: number, quoi: 'jouer' | 'gerer'): Promise<number | null> {
  const groupes = await api.ludothequeGroupes().catch(() => [] as number[][]);
  const installees = groupeDe(id, groupes)
    .map((x) => tele.jeux[x])
    .filter((j): j is JeuPc => !!j?.installation && j.etat === 'telecharge');
  if (installees.length < 2) return installees[0]?.id ?? id;
  return choisir<number>(
    quoi === 'jouer' ? `▶ Jouer à « ${installees[0].titre} » avec…` : `⚙ Gérer « ${installees[0].titre} » : quelle copie ?`,
    installees.map((j) => ({ valeur: j.id, ...libelleCopie({ id: j.id, boutique: j.installation?.boutique?.source ?? '', temps_jeu: j.temps_jeu }, nomPlateforme, duree) })),
    'Ce jeu est installé par plusieurs boutiques : chacune a ses parties et ses mises à jour.',
  );
}

/** Jouer à une carte (A6c) : la copie choisie (voir `choisirCopie`). Rend l'id lancé. */
export async function jouerLaCarte(id: number): Promise<number | null> {
  const cible = await choisirCopie(id, 'jouer');
  if (cible === null) return null;
  await jouer(cible);
  return cible;
}

/** « ⚙ Gérer le jeu » d'une carte réunie : la copie d'abord, s'il y en a plusieurs installées. */
export async function gererLaCarte(id: number) {
  const cible = await choisirCopie(id, 'gerer');
  if (cible !== null) await gererJeu(cible);
}

/** Les firmwares Switch déjà proposés pendant cette séance (un « non » ne revient pas à chaque partie). */
const firmwaresProposes = new Set<string>();

/** Avant une partie Switch dans Eden : clés posées mais pas de firmware, et Firehouse le fournit → proposé (taille dite).
 * Jamais bloquant : un souci ici n'empêche pas de lancer. */
async function firmwareSwitchAvant(id: number, plateforme: string | undefined, emulateur?: string) {
  if (plateforme !== 'Nintendo Switch') return;
  const s = normaliser(etat.pc.emulateurs[plateforme]);
  const e = (emulateur && s.liste.find((x) => x.cle === emulateur)) || pourLeJeu(etat.pc.emulateurs, etat.pc.emulateursJeux, plateforme, id);
  if (!e || !/(^|[\\/])eden[^\\/]*\.exe$/i.test(e.programme) || firmwaresProposes.has(e.programme)) return;
  const f = await api.edenFirmware(e.programme).catch(() => null);
  if (!f?.portable || !f.cles || f.firmware) return;
  const r = await auPlus(3000, api.emulateursRecommandes(plateforme));
  const servi = r?.conseil ? firmwareSwitchServi(r.conseil.bios) : undefined;
  if (!servi) return;
  firmwaresProposes.add(e.programme);
  const oui = await confirmer('💿 Il manque le logiciel système de la Switch', {
    message: questionFirmwareSwitch(servi, debitConnu(), duree),
    libelleValider: '💿 Installer puis jouer',
  });
  if (oui) await installerFirmwareSwitch(e.programme);
}

/** `carte` : la carte e-Reader à lire (sa version) ; sans elle, Frogtend demande laquelle s'il y en a plusieurs. */
export async function jouer(id: number, emulateur?: string, version?: string, carte?: number) {
  // Un seul lancement à la fois (un jeu de boutique peut mettre plusieurs minutes à démarrer).
  if (partie.lancement !== null) return;
  partie.lancement = id;
  // Un jeu de boutique : sa boutique le prépare (mise à jour…), on le dit tout de suite.
  const b = tele.jeux[id]?.installation?.boutique;
  const nomBoutique = b?.source === 'gog' ? 'GOG Galaxy' : b?.source === 'epic' ? 'Epic Games' : 'Steam';
  const attente = b
    ? toast(`▶ ${nomBoutique} lance « ${tele.jeux[id]?.titre ?? 'le jeu'} »… S’il fait une mise à jour, ça peut prendre quelques minutes.`, 'neutre', { jusquaFermeture: true })
    : null;
  try {
    await jouerSansAttente(id, emulateur, version, carte);
  } finally {
    if (attente !== null) fermerToast(attente);
    partie.lancement = null;
  }
}

async function jouerSansAttente(id: number, emulateur?: string, version?: string, carte?: number) {
  const j = tele.jeux[id];
  // Sa console se prépare encore (installation de l'émulateur après le oui) : on l'attend, puis on lance — jamais une
  // 2e préparation en même temps (essai de Seb, 28/10).
  const enPreparation = preparationEnCours(j?.plateforme);
  if (enPreparation && !(await enPreparation)) return;
  // Un jeu e-Reader : quelle carte lire, si plusieurs sont sur le PC (la liste se filtre au-delà de 20).
  if (carte === undefined && aCartes(j?.plateforme)) {
    const cartes = choixDeCartes(j);
    if (cartes.length > 1) {
      const c = await choisir<number>(`▶ « ${j?.titre ?? 'Le jeu'} » : quelle carte lire ?`, cartes, 'La carte est lue dans la cartouche e-Reader, comme sur la vraie console.');
      if (c === null) return;
      carte = c;
    }
  }
  for (let essai = 0; essai < 2; essai++) {
    try {
      // Un souci ici n'empêche JAMAIS le jeu de se lancer.
      await suivreConseil(id, j?.plateforme, emulateur).catch(() => {});
      await firmwareSwitchAvant(id, j?.plateforme, emulateur).catch(() => {});
      await prevenirPareFeu(id, j?.plateforme, emulateur);
      const pe = etat.profil.pleinEcran[String(id)];
      await api.jouer(
        id,
        etat.profil.commandes[String(id)],
        emulateur,
        version,
        etat.profil.figer.includes(String(id)),
        typeof pe === 'string' ? pe : undefined,
        nettoyer(etat.profil.clavier[String(id)]),
        !etat.profil.sansRepetition.includes(String(id)),
        carte,
      );
      toast(`▶ « ${j?.titre ?? 'Le jeu'} » se lance…`);
      return;
    } catch (e) {
      if (estErreurCoeur(e) && e.sorte === 'reglage' && j) {
        // Pas d'émulateur pour cette console : « Prêt à jouer » le prépare (un seul accord), puis on relance.
        if (!(await preparerPourJouer(j.plateforme, 1))) return;
      } else if (estErreurCoeur(e) && e.motif.startsWith('Choisis d')) {
        if (!(await choisirLanceur(id))) return;
      } else if (estErreurCoeur(e) && e.sorte === 'cartouche_absente' && e.cartouche) {
        // La cartouche e-Reader manque : la mettre sur le PC (récapitulatif chiffré et accord, comme tout jeu), puis
        // la personne relance la carte quand elle est arrivée.
        const oui = await confirmer('🎴 Il manque la cartouche e-Reader', {
          message: `${e.motif}\n\nUne carte se lit dans la cartouche, comme sur la vraie console. Frogtend peut la mettre sur ce PC : tu verras sa taille avant de dire oui.`,
          libelleValider: 'Voir la cartouche',
        });
        if (oui) await mettreDansLaLudotheque(e.cartouche);
        return;
      } else if (estErreurCoeur(e) && e.sorte === 'prevenir') {
        // Refusé avant de lancer, avec ce qu'il faut faire : expliqué dans une fenêtre, pas un message qui passe.
        await informer(`▶ « ${j?.titre ?? 'Le jeu'} » ne peut pas encore se lancer`, e.motif);
        return;
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

/** Les parties d'un jeu Steam (lot B2) : Frogtend montre d'abord ce qu'il a trouvé, ne refait pas un abri identique,
 * et demande avant une très grosse copie. Rien n'est écrit chez Steam. */
export async function mettreALAbriSteam(id: number) {
  const attente = toast('💾 Frogtend cherche les parties de ce jeu…', 'neutre', { jusquaFermeture: true });
  let a: ApercuNuage;
  try {
    a = await api.nuageSteamApercu(id);
  } catch (e) {
    fermerToast(attente);
    await informer('💾 Rien à mettre à l’abri', motifDuRefus(e));
    return;
  }
  fermerToast(attente);
  const reste = [
    a.introuvables ? `${a.introuvables} fichier(s) notés par Steam n’ont pas été retrouvés tels quels sur ce PC : pas copiés.` : '',
    a.en_ligne_seulement ? `${a.en_ligne_seulement} fichier(s) sont seulement dans ton nuage (OneDrive…), pas sur ce PC : pas copiés.` : '',
    a.en_attente ? `Steam n’a pas fini de synchroniser ${a.en_attente} fichier(s) : lance Steam un moment, puis recommence.` : '',
    a.modifies ? `${a.modifies} partie(s) plus récente(s) que la dernière synchronisation de Steam : copiées telles quelles.` : '',
    a.hors_nuage ? `${a.hors_nuage} fichier(s) pris hors du nuage de Steam, là où le jeu range ses parties (d’après PCGamingWiki).` : '',
    a.emplacements_illisibles ? `${a.emplacements_illisibles} emplacement(s) de parties indiqué(s) pour ce jeu ne se lisent pas : pas copiés.` : '',
  ].filter(Boolean);
  if (a.fichiers === 0) {
    await informer('💾 Aucune partie sur ce PC', ['Steam garde peut-être les parties de ce jeu seulement dans son nuage.', ...reste].join('\n'));
    return;
  }
  if (a.deja) {
    if (reste.length) await informer('✅ Déjà à l’abri', ['Rien n’a changé depuis le dernier abri.', ...reste].join('\n'));
    else toast('✅ Déjà à l’abri : rien n’a changé depuis le dernier abri.');
    return;
  }
  if (a.gros && !(await confirmer('💾 Grosse copie', { message: `${a.fichiers} fichier(s), ${taille(a.octets)} : c’est beaucoup pour des parties. Les copier quand même ?`, libelleValider: 'Copier' }))) return;
  try {
    const r = await api.nuageSteamAbriter(id, a.gros);
    toast(`✅ ${r.fichiers} fichier(s) de parties (${taille(r.octets)}) mis à l’abri.${reste.length ? ' ' + reste.join(' ') : ''}`);
  } catch (e) {
    toast(`Échec de la mise à l’abri : ${motifDuRefus(e)}`, 'erreur');
  }
}

/** Un jeu importé de ton disque (menu Importer) : Frogtend n'a rien installé, il n'efface rien. Reconnu à son NUMÉRO,
 * toujours négatif (un jeu de Firehouse est positif) — jamais à sa version : Firehouse sert de vraies versions
 * négatives, -1 compris (besoin 23, 19/10). */
export function estImporte(j: { id: number }): boolean {
  return j.id < 0;
}

/** Retirer un jeu du PC, après avoir dit exactement ce qui part et ce qui reste. */
export async function retirer(j: JeuPc) {
  if (estImporte(j)) {
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
    // Installé ailleurs que dans son dossier Frogtend : il reste installé. Son désinstalleur n'est lancé qu'après un oui
    // qui montre le dossier (ce pourrait être celui d'un autre logiciel).
    if (a.reste_installe) {
      const oui = await confirmer(`🗑 « ${j.titre} » est toujours installé`, {
        message: `Il était installé ailleurs que dans son dossier Frogtend :\n${a.reste_installe}\n\nFrogtend n’y a rien effacé. Lancer son désinstalleur (unins000.exe) ? Sa fenêtre s’ouvrira : vérifie que c’est bien ce jeu.`,
        libelleValider: '🗑 Lancer le désinstalleur',
      });
      if (oui) {
        try {
          await api.desinstallerReste(a.reste_installe);
          toast('✅ Désinstalleur terminé.');
        } catch (e) {
          toast(`Refusé : ${motifDuRefus(e)}`, 'erreur');
        }
      }
    }
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
      // L'ajouter demande l'explorateur de Windows et une saisie : pas en Taodbox.
      ...(auCanape() ? [] : [{ valeur: '+', libelle: '➕ Un autre émulateur…', detail: `l’ajouter à ${nomAffiche(j.plateforme)}` }]),
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
  const v = await choisirUneVersion(j, `📀 Version de « ${j.titre} »`, 'Celle que ▶ Jouer lance (« Jouer avec… » en lance une autre pour une partie).');
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
      // L'ajouter demande l'explorateur de Windows et une saisie : pas en Taodbox.
      ...(auCanape() ? [] : [{ valeur: '+', libelle: '➕ Un autre émulateur…', detail: `l’ajouter à ${nomAffiche(j.plateforme)}` }]),
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

/** Le nom montré de la combinaison de la manette qui ouvre le menu (réglage du PC ; « Select + R1 » = View + RB). */
function nomCombinaison(): string {
  const c = etat.pc.menuJeu?.manette || 'Select + R1';
  return c.replace('Select', 'View').replace('R1', 'RB').replace('L1', 'LB');
}

/**
 * « 🕹 Jouer à la manette » (jeu PC lancé sans émulateur, fait pour le clavier) : Frogtend traduit les boutons en
 * touches pendant la partie. Un préréglage courant, ou bouton par bouton ; désactivé d'office.
 */
/** La touche d'un bouton : au bureau, en l'APPUYANT (IDEES 16/10), la liste en secours ; en Taodbox (pas de
 * clavier), la liste. Rend le nom de la touche, `''` (aucune), ou `null` (rien de changé). */
async function toucheDuBouton(nomBouton: string, bouton: string, actuelle: string | undefined): Promise<string | null> {
  const liste = () =>
    choisir<string>(`⌨ Quelle touche pour ${nomBouton} ?`, [
      { valeur: '', libelle: '⬜ Rien', detail: 'ce bouton n’envoie aucune touche' },
      ...TOUCHES.map((k) => ({ valeur: k, libelle: k, detail: detailTouche(k) })),
    ]);
  if (auCanape()) return liste();
  const lue = await attendreTouche(`⌨ Appuie sur la touche pour ${nomBouton}`, {
    message: `Actuellement : ${actuelle ?? 'rien'}.\nAppuie sur la touche voulue, sur ton clavier (Échap et Tab comptent aussi). Pour ne rien changer, clique sur « Annuler ».`,
    verifier: (t) => {
      const r = toucheDepuisEvenement(t);
      return 'refus' in r ? r.refus : null;
    },
  });
  if (lue === null) return null;
  if (lue === 'rien') return '';
  if (lue === 'liste') return liste();
  const r = toucheDepuisEvenement(lue);
  if (!('touche' in r)) return null;
  // Une lettre de déplacement (Z Q S D en français) : la LETTRE, ou la PLACE, qui marche avec tous les claviers. Pour
  // une direction de la manette, la place d'abord.
  if (r.position) {
    const direction = ['haut', 'bas', 'gauche', 'droite'].includes(bouton);
    const options = [
      { valeur: r.position, libelle: `🧭 ${r.position}`, detail: 'la touche à cette PLACE : marche avec tous les claviers (ZQSD ou WASD)' },
      { valeur: r.touche, libelle: `🔤 La lettre ${r.touche}`, detail: 'toujours cette lettre, où qu’elle soit sur le clavier' },
    ];
    return choisir<string>(`⌨ ${nomBouton} → ${r.touche}`, direction ? options : options.reverse(), 'Sur ton clavier, c’est la même touche ; ça change seulement d’un clavier français à un clavier anglais.');
  }
  const attention = detailTouche(r.touche);
  if (attention && !(await confirmer(`⌨ ${nomBouton} → ${r.touche} ?`, { message: `${r.touche} : ${attention}.`, libelleValider: 'Garder' }))) return null;
  return r.touche;
}

export async function choisirClavier(id: number) {
  const j = tele.jeux[id];
  if (!j) return;
  const actuel = nettoyer(etat.profil.clavier[String(id)]);
  const repete = !etat.profil.sansRepetition.includes(String(id));
  type Choix = 'non' | 'perso' | 'repetition' | `pre:${string}`;
  const c = await choisir<Choix>(
    `🕹 Jouer à « ${j.titre} » à la manette`,
    [
      { valeur: 'non', libelle: '⬜ Non', detail: 'le jeu voit ta manette lui-même (ou tu joues au clavier)' },
      ...PREREGLAGES.map((p) => ({ valeur: `pre:${p.id}` as Choix, libelle: p.nom, detail: p.detail })),
      {
        valeur: 'perso',
        libelle: '✏ Bouton par bouton…',
        detail: Object.keys(actuel).length ? 'en partant du réglage actuel' : 'en partant de « Flèches + Z X C »',
      },
      ...(Object.keys(actuel).length
        ? [
            {
              valeur: 'repetition' as Choix,
              libelle: `🔁 Un bouton tenu répète sa touche : ${repete ? 'oui' : 'non'}`,
              detail: repete
                ? 'comme une touche tenue sur un clavier (les menus défilent) ; « non » si le jeu fait l’action en boucle'
                : 'chaque appui du bouton = une seule touche',
            },
          ]
        : []),
    ],
    `Seulement pour un jeu fait pour le clavier, qui ne voit pas ta manette : Frogtend appuie sur les touches pour toi. Actuellement : ${libelleClavier(actuel)}. Le menu de Frogtend (${nomCombinaison()}) reste à sa place.`,
  );
  if (!c) return;
  if (c === 'repetition') {
    const l = etat.profil.sansRepetition.filter((x) => x !== String(id));
    await reglerProfil('sansRepetition', repete ? [...l, String(id)] : l);
    toast(`🔁 « ${j.titre} » : ${repete ? 'un bouton tenu n’envoie sa touche qu’une fois' : 'un bouton tenu répète sa touche, comme un clavier'}.`);
    return;
  }
  let nouvelle: Correspondance = {};
  if (c.startsWith('pre:')) nouvelle = { ...(PREREGLAGES.find((p) => `pre:${p.id}` === c)?.correspondance ?? {}) };
  if (c === 'perso') {
    const depart: Correspondance = { ...(Object.keys(actuel).length ? actuel : PREREGLAGES[0].correspondance) };
    nouvelle = { ...depart };
    for (;;) {
      const b = await choisir<string>(
        '✏ Quel bouton régler ?',
        [
          { valeur: 'fin', libelle: '✅ Terminé', detail: 'garder ce réglage' },
          ...BOUTONS.map((x) => ({ valeur: x.id, libelle: `🎮 ${x.nom}`, detail: nouvelle[x.id] ? `→ ${nouvelle[x.id]}` : '→ rien' })),
        ],
        'Choisis un bouton, puis sa touche. « Terminé » quand tout te va.',
      );
      if (b === 'fin') break;
      if (!b) {
        // Annulé : rien n'est changé — mais le dire, et ne jamais perdre des changements sans demander.
        if (!memes(nouvelle, depart) && !(await confirmer('Abandonner tes changements ?', { message: 'Les touches choisies ne seront pas gardées.', libelleValider: 'Abandonner', danger: true }))) continue;
        toast('Rien n’a changé.');
        return;
      }
      const nomBouton = BOUTONS.find((x) => x.id === b)?.nom ?? b;
      const t = await toucheDuBouton(nomBouton, b, nouvelle[b]);
      if (t === null || t === undefined) continue;
      if (t === '') delete nouvelle[b];
      else nouvelle[b] = t;
    }
  }
  const tous = { ...etat.profil.clavier };
  const desactive = c === 'non' || !Object.keys(nouvelle).length;
  if (desactive) delete tous[String(id)];
  else tous[String(id)] = nouvelle;
  await reglerProfil('clavier', tous);
  toast(`🕹 « ${j.titre} » : ${desactive ? 'la manette n’envoie plus de touches' : `manette — ${libelleClavier(nouvelle)}`}.`);
}

let recherchePleinEcran = false;

/** « 🖥 Plein écran au lancement » (jeu PC lancé sans émulateur) : rien, la touche du jeu (suggérée par ses textes),
 * ou sa fenêtre agrandie sans bordure. Généraliste (Seb, 16/10) : aucun jeu n'est traité à part. */
export async function choisirPleinEcran(id: number) {
  const j = tele.jeux[id];
  if (!j || recherchePleinEcran) return;
  // La lecture des textes du jeu peut prendre quelques secondes (un gros jeu) : on le dit, et un second clic n'ouvre
  // pas une seconde fenêtre.
  recherchePleinEcran = true;
  const attente = toast(`🖥 Frogtend lit le mode d’emploi de « ${j.titre} »…`, 'neutre');
  let suggeree: string | null;
  try {
    suggeree = await api.pleinEcranSuggestion(id).catch(() => null);
  } finally {
    fermerToast(attente);
    recherchePleinEcran = false;
  }
  const actuel = etat.profil.pleinEcran[String(id)];
  type Choix = 'aucun' | 'fenetre' | 'touche' | `touche:${string}`;
  const options: { valeur: Choix; libelle: string; detail?: string }[] = [
    { valeur: 'aucun', libelle: '⬜ Ne rien faire', detail: 'le jeu s’ouvre comme il veut' },
  ];
  if (suggeree) {
    options.push({
      valeur: `touche:${suggeree}`,
      libelle: `⌨ Frogtend appuie sur ${suggeree} pour toi`,
      detail: 'la touche plein écran que le jeu indique dans son mode d’emploi (Lisez-moi)',
    });
  }
  options.push(
    { valeur: 'touche', libelle: '⌨ Frogtend appuie sur une autre touche…', detail: 'choisis la touche qui met ce jeu en plein écran (F11, Alt+Entrée…)' },
    { valeur: 'fenetre', libelle: '🖥 Agrandir sa fenêtre à tout l’écran', detail: 'sans cadre ni barre de titre ; selon le jeu, l’image peut rester petite au milieu' },
  );
  let c = await choisir<Choix>(
    `🖥 Plein écran au lancement de « ${j.titre} »`,
    options,
    `Actuellement : ${libellePleinEcran(actuel)}. Si le jeu s’ouvre déjà en plein écran, choisis « Ne rien faire ».`,
  );
  if (!c) return;
  if (c === 'touche') {
    const t = await choisir<string>(
      '⌨ Quelle touche met ce jeu en plein écran ?',
      TOUCHES_PLEIN_ECRAN.map((k) => ({ valeur: k, libelle: k })),
      'Souvent écrite dans le fichier « Lisez-moi » du jeu ou dans ses options.',
    );
    if (!t) return;
    c = `touche:${t}`;
  }
  const tous = { ...etat.profil.pleinEcran };
  if (c === 'aucun') delete tous[String(id)];
  else tous[String(id)] = c;
  await reglerProfil('pleinEcran', tous);
  toast(`🖥 « ${j.titre} » : plein écran ${c === 'aucun' ? 'désactivé' : `— ${libellePleinEcran(c)}`}.`);
}

let verificationFiger = false;

/** « ❄ Figer le jeu pendant le menu » (jeu PC lancé sans émulateur) : désactivé d'office ; refusé si un anti-triche
 * est trouvé ; les risques sont dits avant d'activer (Seb, 05/10). */

export async function choisirFiger(id: number) {
  const j = tele.jeux[id];
  if (!j || verificationFiger) return;
  const cle = String(id);
  const actif = etat.profil.figer.includes(cle);
  if (actif) {
    await reglerProfil('figer', etat.profil.figer.filter((x) => x !== cle));
    toast(`❄ « ${j.titre} » : le menu de Frogtend ne fige plus le jeu.`);
    return;
  }
  // La recherche d'un anti-triche peut prendre quelques secondes (un gros dossier) : on le dit, et un second clic
  // n'ouvre pas une seconde fenêtre.
  verificationFiger = true;
  const attente = toast(`❄ Frogtend vérifie « ${j.titre} »…`, 'neutre');
  let v: VerificationFiger;
  try {
    v = await api.jeuAntiTriche(id);
  } catch (e) {
    toast(`❄ ${motifDuRefus(e)}`, 'erreur');
    return;
  } finally {
    fermerToast(attente);
    verificationFiger = false;
  }
  if (v.sorte !== 'aucun') {
    await informer(
      `❄ « ${j.titre} » ne peut pas être figé`,
      v.sorte === 'anti_triche'
        ? `Ce jeu est protégé par un anti-triche (${v.nom}). Le figer pourrait te faire déconnecter, voire signaler ton compte : Frogtend ne le figera donc pas. Le menu s’ouvre quand même par-dessus le jeu.`
        : 'Ce jeu a trop de fichiers pour que Frogtend vérifie qu’il n’a pas d’anti-triche : par prudence, il ne le figera pas. Le menu s’ouvre quand même par-dessus le jeu.',
    );
    return;
  }
  const ok = await confirmer(`❄ Figer « ${j.titre} » pendant le menu de Frogtend ?`, {
    message:
      'Quand tu ouvres le menu de Frogtend, le jeu se fige, comme une pause, et ne reçoit plus la manette ; il repart là où il était quand tu reprends. À utiliser pour un jeu solo qui ne se met pas en pause tout seul. Selon le jeu, le son peut grésiller pendant le menu ; un jeu en ligne perdrait sa connexion. Si Windows dit que le jeu « ne répond pas », attends : il repart quand tu reprends. (Frogtend n’a pas trouvé d’anti-triche dans ce jeu.)',
    libelleValider: 'Figer pendant le menu',
  });
  if (!ok) return;
  if (!etat.profil.figer.includes(cle)) await reglerProfil('figer', [...etat.profil.figer, cle]);
  toast(`❄ « ${j.titre} » sera figé pendant le menu de Frogtend.`, 'ok');
}

/**
 * Image et fluidité d'un jeu PS3 (RPCS3) : un sous-menu qui revient sur lui-même, une ligne par réglage avec sa valeur
 * actuelle ; un choix ne change QUE ce réglage pour CE jeu (le reste suit RPCS3). Prend effet au prochain lancement.
 */
async function choisirImagePs3(id: number) {
  const j = tele.jeux[id];
  if (!j) return;
  for (;;) {
    let r: Awaited<ReturnType<typeof api.rpcs3Image>>;
    try {
      r = await api.rpcs3Image(id);
    } catch (e) {
      toast(`Impossible : ${motifDuRefus(e)}`, 'erreur');
      return;
    }
    if (!r) {
      toast(`« ${j.titre} » ne se lance plus avec RPCS3 sur ce PC : rien à régler ici.`, 'erreur');
      return;
    }
    if (!r.numero) {
      await informer(
        '🎨 Image et fluidité',
        `Frogtend ne lit pas le numéro du disque de « ${j.titre} ».\n\nS’il est encore compressé (.zip), décompresse-le d’abord avec le bouton « 🗜 Décompresser pour jouer » ${auCanape() ? 'de son panneau, sur l’écran du PC (hors Taodbox)' : 'de son panneau, dans la ludothèque'} : RPCS3 en a besoin pour jouer, et Frogtend pour le régler.`,
      );
      return;
    }
    const valeurs = new Map(r.valeurs);
    const parJeu = new Set(r.par_jeu);
    type Ligne = string | 'defaut' | 'fini';
    const lignes: { valeur: Ligne; libelle: string; detail?: string }[] = reglagesMontres(valeurs).map((g) => ({
      valeur: g.cle,
      libelle: g.titre,
      detail: `${libelleValeur(g.cle, valeurs.get(g.cle) ?? '')}${parJeu.has(g.cle) ? ' — pour ce jeu' : ' — comme RPCS3'}`,
    }));
    if (parJeu.size)
      lignes.push({ valeur: 'defaut', libelle: '↩ Valeurs de RPCS3', detail: 'ce jeu reprend les réglages de RPCS3 (y compris ceux d’image faits pour lui dans RPCS3)' });
    lignes.push({ valeur: 'fini', libelle: '✅ Terminé' });
    const c = await choisir<Ligne>(
      `🎨 Image et fluidité — « ${j.titre} »`,
      lignes,
      `Ces réglages ne valent que pour ce jeu, pour tout le monde sur ce PC ; les autres jeux PS3 gardent ceux de RPCS3. Ils prennent effet au prochain lancement.` +
        (r.cache_coupe
          ? `\n\n⚠ RPCS3 est réglé pour oublier les effets déjà préparés (son cache de shaders est coupé) : le jeu doit les préparer de nouveau à chaque partie, et le début de chaque partie peut être plus lent. Pour le rétablir : dans les réglages de RPCS3 (ceux du jeu s’il a les siens), décoche « Disable On-Disk Shader Cache ».`
          : ''),
    );
    if (!c || c === 'fini') return;
    try {
      if (c === 'defaut') {
        await api.rpcs3ReglerImage(id, [], true);
        toast(`✅ « ${j.titre} » reprend les réglages de RPCS3 (au prochain lancement).`);
        continue;
      }
      const g = REGLAGES_IMAGE.find((x) => x.cle === c);
      if (!g) return;
      const actuelle = valeurs.get(c);
      const v = await choisir<string>(
        `${g.titre} — « ${j.titre} »`,
        g.valeurs.map((x) => ({ valeur: x.valeur, libelle: `${x.valeur === actuelle ? '⭐ ' : ''}${x.libelle}`, detail: x.detail })),
        'Pour ce jeu seulement. Prend effet au prochain lancement.',
      );
      // La valeur actuelle héritée de RPCS3, choisie exprès : elle est figée pour ce jeu.
      if (!v || (v === actuelle && parJeu.has(c))) continue;
      await api.rpcs3ReglerImage(id, [[c, v]]);
      toast(`✅ ${g.titre.replace(/^\S+\s/, '')} : ${libelleValeur(c, v)}, pour « ${j.titre} » (au prochain lancement).`);
    } catch (e) {
      toast(`Refusé : ${motifDuRefus(e)}`, 'erreur');
      return;
    }
  }
}

/**
 * Les retouches (patchs) d'un jeu PS3 (contrat 1.18 ; ✅ Seb 19/10 : « par Firehouse ») : celles de Firehouse pour
 * CE jeu et sa version, ✅/⬜ ; un choix active ou désactive (au prochain lancement). Seules celles du jeu lui-même
 * sont montrées (une démo a le même numéro, mais un autre programme : ses retouches ne s'appliquent pas). La remarque
 * de l'auteur est montrée AVANT d'activer. Au premier patch activé, la liste officielle des patchs de RPCS3 est
 * installée ou mise à jour — après un oui, taille annoncée. Hors ligne : seules les retouches actives, à désactiver.
 */
async function choisirPatchsPs3(id: number) {
  const j = tele.jeux[id];
  if (!j) return;
  const TITRE = '🩹 Retouches du jeu (patchs)';
  for (;;) {
    let r: PatchsPs3 | null;
    try {
      r = await api.rpcs3Patchs(id);
    } catch (e) {
      toast(`Impossible : ${motifDuRefus(e)}`, 'erreur');
      return;
    }
    if (!r) {
      toast(`« ${j.titre} » ne se lance plus avec RPCS3 sur ce PC : rien à régler ici.`, 'erreur');
      return;
    }
    if (!r.numero) {
      await informer(
        TITRE,
        `Frogtend ne lit pas le numéro du disque de « ${j.titre} ».\n\nS’il est encore compressé (.zip), décompresse-le d’abord avec le bouton « 🗜 Décompresser pour jouer » ${auCanape() ? 'de son panneau, sur l’écran du PC (hors Taodbox)' : 'de son panneau, dans la ludothèque'}.`,
      );
      return;
    }
    const titre = titreDuJeu(r.patchs, j.titre);
    const duJeu = r.patchs.filter((p) => p.titre === titre);
    const autres = r.patchs.length - duJeu.length;
    if (!duJeu.length) {
      await informer(TITRE, r.hors_ligne ? `Firehouse est injoignable, et aucune retouche n’est active pour « ${j.titre} ».` : `Aucune retouche connue pour « ${j.titre} »${r.version ? ` (version ${r.version})` : ''}.`);
      return;
    }
    const lignes: { valeur: number | 'fini'; libelle: string; detail?: string }[] = duJeu.map((p, i) => {
      const d = nomDuPatch(p);
      return { valeur: i, libelle: `${p.actif ? '✅' : '⬜'} ${d.texte}${d.anglais ? ' (en anglais)' : ''}`, detail: detailPatch(p) };
    });
    lignes.push({ valeur: 'fini', libelle: '✅ Terminé' });
    const message = [
      'Une retouche change le jeu pendant qu’il tourne (moins de flou, plus d’images par seconde…) ; tes fichiers ne sont pas modifiés. Elle vaut pour ce jeu, pour tout le monde sur ce PC, et prend effet au prochain lancement.',
      r.hors_ligne ? 'Firehouse est injoignable : seules les retouches actives sont montrées, pour les désactiver.' : '',
      !r.hors_ligne && !r.liste_a_jour ? `À la première retouche activée, Frogtend ${r.liste_presente ? 'met à jour' : 'installe'} la liste officielle des patchs de RPCS3 (${taille(r.taille_liste)}, depuis Firehouse).` : '',
      autres ? `${autres} autre(s) retouche(s) concernent une autre édition (démo…) : pas montrées, elles ne s’appliquent pas à ce jeu.` : '',
    ]
      .filter(Boolean)
      .join('\n\n');
    const c = await choisir<number | 'fini'>(`${TITRE} — « ${j.titre} »`, lignes, message);
    if (c === null || c === 'fini') return;
    const p = duJeu[c];
    const nom = nomDuPatch(p).texte;
    let liste = false;
    if (!p.actif) {
      // La remarque de l'auteur avant d'activer (souvent une mise en garde) : traduite par Firehouse (contrat 1.20),
      // sinon l'anglais marqué.
      const remarque = remarqueDuPatch(p);
      if (remarque && !(await confirmer(`🩹 « ${nom} »`, { message: `Remarque de l’auteur${remarque.anglais ? ' (en anglais)' : ''} :\n${remarque.texte}`, libelleValider: 'Continuer' }))) continue;
      if (!r.liste_a_jour) {
        liste = await confirmer(`🩹 Activer « ${nom} » ?`, {
          message: `Frogtend ${r.liste_presente ? 'met d’abord à jour' : 'installe d’abord'} la liste officielle des patchs de RPCS3 : ${taille(r.taille_liste)}, depuis Firehouse, vérifiée avant d’être posée.${r.liste_presente ? ' L’ancienne liste est gardée de côté.' : ''}`,
          libelleValider: '⬇ Installer et activer',
        });
        if (!liste) continue;
      }
    }
    try {
      await api.rpcs3PatchRegler(id, p, !p.actif, liste);
      toast(`🩹 « ${nom} » ${p.actif ? 'désactivée' : 'activée'} pour « ${j.titre} » (au prochain lancement).`);
    } catch (e) {
      toast(`Refusé : ${motifDuRefus(e)}`, 'erreur');
      return;
    }
  }
}

/**
 * Les mods d'un jeu PC (contrat 1.19 ; ✅ Seb 19/10) : ceux qui sont installés (✅, à retirer), ceux qui s'installent
 * d'un clic (⬇) et ceux qui se font à la main (🔗, la page). Avant le oui : ce qu'il fait, sa taille, son partage, une
 * version d'essai, une page déplacée, une empreinte absente. Frogtend télécharge depuis la page GitHub de l'auteur,
 * met à l'abri tout ce qui est remplacé (et les parties), et peut tout retirer.
 */
export async function choisirMods(id: number) {
  const j = tele.jeux[id];
  if (!j) return;
  const TITRE = '🧩 Mods';
  // Un jeu Steam (lot B3) : posé dans son dossier chez Steam, mais tout ce que Frogtend garde (originaux, manifestes)
  // reste dans ses données ; à la racine du jeu seulement ; risque d'un jeu en ligne demandé avant.
  const appidSteam = j.installation?.boutique?.source === 'steam' ? j.installation.boutique.cle : null;
  for (;;) {
    // Les mods installés se lisent sur le disque : hors ligne, on peut toujours les retirer.
    let installes: ModInstalle[];
    try {
      installes = await api.modsInstalles(id);
    } catch (e) {
      toast(`Impossible : ${motifDuRefus(e)}`, 'erreur');
      return;
    }
    const liste: ModJeu[] | null = await api
      .jeuTriches(id)
      .then((t) => t.mods ?? [])
      .catch(() => null);
    type Choix = { sorte: 'installe'; m: ModInstalle } | { sorte: 'mod'; m: ModJeu } | 'fini';
    const lignes: { valeur: Choix; libelle: string; detail?: string }[] = [
      ...installes.map((m) => ({
        valeur: { sorte: 'installe' as const, m },
        libelle: `${m.etat === 'installe' && !m.steam_mis_a_jour ? '✅' : '⚠'} ${m.mod_nom}`,
        detail:
          m.etat !== 'installe'
            ? 'installation interrompue — choisis-le pour remettre le jeu comme avant'
            : m.steam_mis_a_jour
              ? 'Steam a mis le jeu à jour depuis : le mod a pu être défait — choisis-le pour le retirer, puis réinstalle-le'
              : 'installé — choisis-le pour le retirer',
      })),
      ...(liste ?? [])
        .filter((m) => !installes.some((i) => i.mod_nom === m.nom))
        .map((m) => {
          const f = fichierInstallable(m);
          const surNexus = !f && !m.refus && !!(pageNexus(m.url) ?? pageNexus(m.page));
          return {
            valeur: { sorte: 'mod' as const, m },
            libelle: `${f ? '⬇' : surNexus ? '🌐' : '🔗'} ${m.nom}`,
            detail: f
              ? `s’installe d’un clic (${taille(f.taille)})${m.description ? ` — ${court(m.description)} (en anglais)` : ''}`
              : surNexus
                ? 'sur Nexus Mods : s’installe après « Mod Manager Download » sur sa page'
                : raisonSansClic(m),
          };
        }),
      { valeur: 'fini', libelle: '✅ Terminé' },
    ];
    if (lignes.length === 1) {
      await informer(TITRE, liste === null ? 'Firehouse est injoignable, et aucun mod n’est installé dans ce jeu.' : `Firehouse ne connaît pas de mod pour « ${j.titre} ».`);
      return;
    }
    const c = await choisir<Choix>(
      `${TITRE} — « ${j.titre} »`,
      lignes,
      'Un mod est un ajout fait par des joueurs (image plus large, plus fluide, corrections…). Frogtend garde de côté tout ce qu’il remplace et peut tout retirer.' +
        (liste === null ? '\n\nFirehouse est injoignable : tu peux retirer un mod, pas en ajouter.' : ''),
    );
    if (c === null || c === 'fini') return;
    if (c.sorte === 'installe') {
      const message = c.m.steam_mis_a_jour
        ? 'Steam a mis le jeu à jour depuis ce mod : les fichiers que Steam a déjà changés restent tels quels ; les autres fichiers d’origine sont remis, et Steam pourra ensuite tout revérifier.'
        : 'Les fichiers d’origine du jeu sont remis en place. Un fichier du mod que tu as modifié est gardé (et copié de côté).';
      if (!(await confirmer(`🧩 Retirer « ${c.m.mod_nom} » ?`, { message, libelleValider: 'Retirer' }))) continue;
      try {
        const r = await api.modDesinstaller(id, c.m.cle);
        toast(
          `🧩 « ${c.m.mod_nom} » retiré${r.gardes.length ? ` ; gardé(s) car modifié(s) : ${r.gardes.join(', ')}` : ''}${r.laisses_a_la_boutique.length ? ` ; laissé(s) à Steam (sa mise à jour) : ${r.laisses_a_la_boutique.join(', ')}` : ''}.`,
        );
        // Après une mise à jour de Steam, les fichiers d'origine remis peuvent être d'une version d'avant : Steam les
        // répare (« Vérifier l'intégrité »), avec l'accord de la personne — au bureau (la fenêtre de Steam se pilote à
        // la souris).
        if (appidSteam && (c.m.steam_mis_a_jour || r.laisses_a_la_boutique.length)) {
          const texte = 'Steam a mis le jeu à jour depuis ce mod : certains fichiers remis peuvent dater d’avant. Steam peut les vérifier et remettre les siens (« Vérifier l’intégrité des fichiers »).';
          if (auCanape()) await informer('🛠 À faire sur l’écran du PC', `${texte} Dans Steam : clic droit sur le jeu ▸ Propriétés ▸ Fichiers installés ▸ Vérifier l’intégrité.`);
          else if (await confirmer('🛠 Faire vérifier le jeu par Steam ?', { message: texte, libelleValider: 'Ouvrir la vérification dans Steam' }))
            await api.steamOuvrir(appidSteam, 'verifier').catch((e) => toast(`Steam ne s’ouvre pas : ${motifDuRefus(e)}`, 'erreur'));
        }
      } catch (e) {
        toast(`Refusé : ${motifDuRefus(e)}`, 'erreur');
      }
      continue;
    }
    const m = c.m;
    const f = fichierInstallable(m);
    const page = adresseWeb(m.consignes_url) ?? adresseWeb(m.page) ?? adresseWeb(m.url);
    if (!f) {
      // À la main : la page, au bureau (au canapé, le navigateur passerait derrière la télé).
      const nexus = pageNexus(m.url) ?? pageNexus(m.page);
      if (!page) {
        await informer(`🔗 ${m.nom}`, raisonSansClic(m));
      } else if (await seulementAuBureau(`Ouvrir la page de « ${m.nom} »`)) {
        // Dit par la fenêtre.
      } else if (nexus) {
        // Nexus (lot 0.82.0) : « Mod Manager Download » sur le site renvoie le fichier à Frogtend, pour CE jeu.
        if (
          await confirmer(`🔗 ${m.nom}`, {
            message: [
              m.description ? `Ce qu’il fait (en anglais) : ${m.description}` : '',
              'Ce mod est sur Nexus Mods : sur sa page, clique « Mod Manager Download » ; Frogtend le reçoit et te demande ton accord avant de l’installer dans ce jeu.',
            ]
              .filter(Boolean)
              .join('\n\n'),
            libelleValider: '🌐 Ouvrir sur Nexus',
          })
        )
          await ouvrirSurNexus(id, m.nom);
      } else if (
        await confirmer(`🔗 ${m.nom}`, {
          message: [m.description ? `Ce qu’il fait (en anglais) : ${m.description}` : '', raisonSansClic(m)].filter(Boolean).join('\n\n'),
          libelleValider: '🌐 Ouvrir la page',
        })
      ) {
        await ouvrirPageWeb(page);
      }
      continue;
    }
    // Un mod « à confirmer » : la page dit où le poser ; au canapé, à faire depuis l'écran du PC. Lire les consignes
    // ramène à la même question (jamais à la liste de tous les mods).
    // Un jeu Steam : le risque d'un jeu en ligne (anti-triche connu : refus) D'ABORD — un refus n'a pas à faire choisir
    // l'emplacement pour rien.
    let risque = false;
    if (appidSteam) {
      const a = await accordModSteam(id);
      if (a === null) continue;
      risque = a;
    }
    let programme = false;
    let annule = false;
    while (f.a_confirmer) {
      if (await seulementAuBureau(`Installer « ${m.nom} » (la page dit où le poser)`)) {
        annule = true;
        break;
      }
      const o = await choisir<'racine' | 'programme' | 'page'>(
        `🧩 Où poser « ${m.nom} » ?`,
        [
          { valeur: 'page', libelle: '📖 Lire les consignes de la page d’abord' },
          { valeur: 'racine', libelle: '📁 Dans le dossier du jeu' },
          // Un jeu Steam : Frogtend ne connaît pas son programme (Steam le lance) — la racine seulement.
          ...(appidSteam ? [] : [{ valeur: 'programme' as const, libelle: '🎮 À côté du programme du jeu', detail: 'si la page dit « à côté de l’exe »' }]),
        ],
        'La page du mod dit où le poser. En cas de doute, lis-la d’abord.',
      );
      if (!o) {
        annule = true;
        break;
      }
      if (o === 'page') {
        await ouvrirPageWeb(f.consignes_url ?? page);
        continue;
      }
      programme = o === 'programme';
      break;
    }
    if (annule) continue;
    const message = [
      m.description ? `Ce qu’il fait (en anglais) : ${m.description}` : '',
      `Taille : ${taille(f.taille)}, téléchargé depuis la page de l’auteur (GitHub).`,
      ...avertissements(f),
      appidSteam
        ? 'Les parties que Steam garde pour son nuage sont mises à l’abri, et les fichiers du jeu que le mod remplace sont gardés de côté dans les données de Frogtend. Après une mise à jour de Steam, le mod peut être à refaire.'
        : 'Tout fichier du jeu remplacé est gardé de côté, et tes parties sont mises à l’abri quand Frogtend sait où elles sont (jeu installé par Frogtend).',
    ]
      .filter(Boolean)
      .join('\n\n');
    if (!(await confirmer(`🧩 Installer « ${m.nom} » ?`, { message, libelleValider: '⬇ Télécharger et installer' }))) continue;
    if (!(await proposerCopieAvantMod(id))) continue;
    try {
      const b = await avecProgres(`Installation de « ${m.nom} » (ne ferme pas Frogtend)`, false, (suivi) => api.modInstaller(id, m.nom, f.nom, programme, suivi, risque));
      toast(`🧩 « ${m.nom} » installé : ${b.ajoutes} fichier(s) ajouté(s), ${b.remplaces} remplacé(s) (gardés de côté).`);
    } catch (e) {
      toast(`Refusé : ${motifDuRefus(e)}`, 'erreur');
    }
  }
}

/** Un texte court pour une ligne de liste (le texte entier est montré avant le oui). */
function court(t: string, n = 90): string {
  return t.length > n ? `${t.slice(0, n - 1).trimEnd()}…` : t;
}

/** Un jeu PC lancé sans émulateur (ses mods, son plein écran, sa manette → clavier). */
export function jeuPcSansEmulateur(id: number): boolean {
  const j = tele.jeux[id];
  return !!j?.installation && !j.installation.fichier_du_jeu && !pourLeJeu(etat.pc.emulateurs, etat.pc.emulateursJeux, j.plateforme, id);
}

/** Le menu « ⚙ Gérer le jeu » : toutes les actions possibles sur un jeu du PC, au même endroit, bien visibles. */
export async function gererJeu(id: number) {
  const j = tele.jeux[id];
  // Le jeu importé par une AUTRE personne de ce PC : elle seule le gère (✅ Seb 20/10).
  if (!j || j.d_un_autre) return;
  type Action = 'jouerAvec' | 'autreVersion' | 'installer' | 'lanceur' | 'version' | 'emulateur' | 'image' | 'patchs' | 'mods' | 'triches' | 'succes' | 'commandes' | 'figer' | 'pleinEcran' | 'clavier' | 'abri' | 'delier' | 'retirer';
  const options: { valeur: Action; libelle: string; detail?: string }[] = [];
  if (j.etat === 'telecharge' && !j.installation) options.push({ valeur: 'installer', libelle: '📦 Installer le jeu' });
  // Un jeu d'une boutique (Steam) : c'est elle qui le lance, l'installe et le désinstalle ; Frogtend n'écrit rien dans
  // son dossier (ni mods, ni abri pour l'instant).
  const deBoutique = !!j.installation?.boutique;
  if (j.installation && !j.installation.fichier_du_jeu && !deBoutique)
    options.push({ valeur: 'lanceur', libelle: '🎯 Changer ce qui lance le jeu' });
  const versions = j.installation?.versions ?? [];
  // Au canapé, pas de bouton « Jouer avec… » : une autre version (ou un autre émulateur) pour CETTE partie, par Y.
  if (auCanape() && j.installation && !deBoutique && (versions.length > 1 || emulateursDuJeu(j.plateforme).liste.length > 1))
    options.push({ valeur: 'jouerAvec', libelle: '▶ Jouer avec…', detail: 'une autre version ou un autre émulateur, pour cette partie' });
  // Un jeu de Firehouse (ROM, image disque) : ses autres versions (régions) s'ajoutent depuis sa fiche.
  if (!auCanape() && id > 0 && !deBoutique && !aCartes(j.plateforme) && j.etat === 'telecharge' && !!j.installation?.fichier_du_jeu)
    options.push({ valeur: 'autreVersion', libelle: '➕ Ajouter une autre version', detail: 'sur la fiche du jeu, version par version' });
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
    // Un jeu PS3 lancé par RPCS3 : son image et sa fluidité (✅ Seb 19/10 : « shaders = les deux »).
    if (j.installation && /(^|[\\/])rpcs3\.exe$/i.test(e?.programme ?? '')) {
      options.push({ valeur: 'image', libelle: '🎨 Image et fluidité', detail: 'résolution, saccades, netteté — pour ce jeu' });
      // Ses patchs (contrat 1.18 ; ✅ Seb 19/10).
      options.push({ valeur: 'patchs', libelle: '🩹 Retouches du jeu (patchs)', detail: 'moins de flou, plus d’images par seconde… — pour ce jeu' });
    }
  }
  // Un jeu lancé sans émulateur (jeu PC) : jouer à la manette, son plein écran au lancement, le figer pendant le menu.
  // « Commandes » ne concerne que les émulateurs : pas pour lui (deux entrées « manette » se confondraient).
  const pcSansEmulateur = jeuPcSansEmulateur(id);
  if (!pcSansEmulateur) options.push({ valeur: 'commandes', libelle: '🎮 Commandes', detail: libelleCommandes(etat.profil.commandes[String(id)]) });
  if (pcSansEmulateur) {
    // Ses mods (contrat 1.19 ; ✅ Seb 19/10) — d'une boutique, Steam seulement (lot B3, ✅ Seb 21/10).
    if (!deBoutique || j.installation?.boutique?.source === 'steam') options.push({ valeur: 'mods', libelle: '🧩 Mods', detail: 'ajouter ou retirer des mods — pour ce jeu' });
    // Triches et Cheat Engine (lot B, 21/10 : aussi pour un jeu de boutique) — au bureau : la page s'utilise à la
    // souris ; au canapé, Cheat Engine reste dans le menu en jeu.
    if (!auCanape())
      options.push({ valeur: 'triches', libelle: '🧰 Triches et Cheat Engine', detail: 'Cheat Engine, mods conseillés — pour ce jeu' });
    options.push({
      valeur: 'clavier',
      libelle: '🕹 Jouer à la manette (jeu fait pour le clavier)',
      detail: libelleClavier(nettoyer(etat.profil.clavier[String(id)])),
    });
    options.push({ valeur: 'pleinEcran', libelle: '🖥 Plein écran au lancement', detail: libellePleinEcran(etat.profil.pleinEcran[String(id)]) });
    options.push({
      valeur: 'figer',
      libelle: '❄ Figer le jeu pendant le menu de Frogtend',
      detail: etat.profil.figer.includes(String(id)) ? 'oui (le jeu s’arrête pendant le menu)' : 'non (le jeu continue derrière le menu)',
    });
  }
  if (j.installation && !deBoutique)
    options.push({ valeur: 'abri', libelle: '💾 Mettre mes parties à l’abri', detail: 'une copie de ce qui a changé depuis l’installation' });
  // Ses succès en détail (lot C ; au bureau ET au canapé, ✅ Seb 21/10) : un jeu Steam, ou un jeu d'émulateur
  // (RetroAchievements, lot C3).
  const ra = !!j.installation && !pcSansEmulateur ? await api.raEtat().catch(() => null) : null;
  const raRelie = !!ra?.cle_enregistree && !!ra?.compte;
  if (j.installation?.boutique?.source === 'steam' || j.installation?.boutique?.source === 'gog' || raRelie)
    options.push({ valeur: 'succes', libelle: '🏆 Succès', detail: 'la liste, obtenus et à obtenir' });
  // Un jeu Steam : ses parties que Steam garde pour son nuage (lot B2, lecture seule chez Steam).
  if (j.installation?.boutique?.source === 'steam')
    options.push({ valeur: 'abri', libelle: '💾 Mettre mes parties à l’abri', detail: 'une copie de ses parties (nuage de Steam et dossiers du jeu)' });
  // Un jeu importé relié à une fiche de Firehouse (contrat 1.21) : le lien se défait d'ici aussi.
  if (estImporte(j) && j.fiche)
    options.push({ valeur: 'delier', libelle: '🔗 Ce n’est pas ce jeu (ne plus relier)', detail: 'ton fichier ne bouge pas : il perd seulement la fiche de Firehouse' });
  // Retirer (et désinstaller) reste au bureau (✅ Seb 19/10) : il efface, et le désinstalleur se pilote à la souris.
  if (j.etat === 'telecharge' && !auCanape() && !deBoutique)
    options.push(
      estImporte(j)
        ? { valeur: 'retirer', libelle: '🗑 Retirer de la ludothèque', detail: 'ses fichiers restent sur ton disque' }
        : { valeur: 'retirer', libelle: '🗑 Retirer du PC (désinstaller)', detail: 'tes parties sont copiées à l’abri d’abord' },
    );
  // Un chemin de dossier ne dit rien à 3 m de la télé.
  const c = await choisir<Action>(`⚙ Gérer « ${j.titre} »`, options, auCanape() ? undefined : `Rangé dans ${j.installation?.dossier ?? j.dossier}`);
  if (c === 'jouerAvec') await jouerAvec(id);
  else if (c === 'autreVersion') await goto(`/jeu/${id}`);
  else if (c === 'installer') await installer(id);
  else if (c === 'lanceur') await choisirLanceur(id);
  else if (c === 'version') await choisirVersionParDefaut(id);
  else if (c === 'emulateur') await choisirEmulateurDuJeu(id);
  else if (c === 'image') await choisirImagePs3(id);
  else if (c === 'patchs') await choisirPatchsPs3(id);
  else if (c === 'mods') await choisirMods(id);
  else if (c === 'triches') await goto(`/triches?jeu=${id}`);
  else if (c === 'succes')
    // Au canapé, sous /taodbox : le mode canapé (manette, plein écran) reste actif. Le jeu DU PC : son jeu Steam, ou
    // RetroAchievements pour un jeu d'émulateur (le cœur choisit).
    await goto(`${auCanape() ? '/taodbox' : ''}/succes?jeu=${id}&titre=${encodeURIComponent(j.titre)}`);
  else if (c === 'commandes') await choisirCommandes(id);
  else if (c === 'figer') await choisirFiger(id);
  else if (c === 'pleinEcran') await choisirPleinEcran(id);
  else if (c === 'clavier') await choisirClavier(id);
  else if (c === 'abri') await (deBoutique ? mettreALAbriSteam(id) : mettreALAbri(id));
  else if (c === 'delier') await delier(j);
  else if (c === 'retirer') await retirer(j);
}

/** Défait le lien d'un jeu importé avec sa fiche de Firehouse (le jeu et ses fichiers ne bougent pas). */
async function delier(j: JeuPc) {
  if (!(await confirmer(`Ne plus relier « ${j.titre} » à sa fiche ?`, { message: 'Ton jeu ne bouge pas : il perd seulement la fiche et les images de Firehouse.', libelleValider: 'Ne plus relier' }))) return;
  try {
    await api.rapprochementDefaire(j.id);
    await rechargerJeuxDuPc();
    await rechargerListe();
    await rechargerPlateformes();
    // La fiche n'est plus montrée : la sélection revient sur la ligne du jeu importé.
    ludo.selection = ludo.jeux.find((x) => x.id === j.id) ?? ludo.selection;
    toast(`« ${j.titre} » n’est plus relié à Firehouse.`);
  } catch (e) {
    toast(`Refusé : ${motifDuRefus(e)}`, 'erreur');
  }
}
