// Les actions sur un jeu du PC : l'installer, choisir ce qui le lance, y jouer, régler l'émulateur de son système,
// mettre ses parties à l'abri, le retirer du PC. Chaque action qui touche au disque est expliquée et acceptée d'abord.

import { listen } from '@tauri-apps/api/event';
import { isTauri } from '@tauri-apps/api/core';
import { api, duree, estErreurCoeur, taille, type Candidat, type JeuPc, type VerificationFiger } from '$lib/api';
import { choisir, confirmer, fermerToast, informer, toast } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { completerRetroArch, poserBios, reglerEmulateur } from '$lib/emulateurs/assistant.svelte';
import { avecConseil, biosADireOuPoser, biosRetroArch, changementDeConseil, estConseille, poidsBios } from '$lib/emulateurs/pret';
import { preparerPourJouer } from '$lib/emulateurs/pret.svelte';
import { estRetroArch, normaliser, pourLeJeu } from '$lib/emulateurs/choix';
import { etat, reglerPc, reglerProfil } from '$lib/etat.svelte';
import type { CommandesJeu } from '$lib/reglages/reglages';
import { GENRES_MANETTE, libelleCommandes } from './commandes';
import { apresUnePartie } from '$lib/sauvegarde.svelte';
import { ludo, rechargerListe, rechargerPlateformes } from './ludotheque.svelte';
import { rechargerJeuxDuPc, tele } from './telechargements.svelte';
import { estCourante, libelleVersion, nomDuFichier } from './versions';
import { libellePleinEcran, TOUCHES_PLEIN_ECRAN } from './plein-ecran';
import { BOUTONS, detailTouche, libelleClavier, memes, nettoyer, PREREGLAGES, TOUCHES, type Correspondance } from './clavier';

export const partie = $state<{ enJeu: number | null; installation: number | null }>({ enJeu: null, installation: null });

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
    const oui = await confirmer(`🕹 Nouveau réglage conseillé pour ${plateforme}`, {
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
  const oui = await confirmer(`🕹 La maison conseille maintenant ${nom} pour ${plateforme}`, {
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

/** Jouer. `emulateur` : la clé d'un émulateur du système, pour ce lancement seulement. Si un réglage manque
 * (émulateur, lanceur), on le demande puis on relance. */
export async function jouer(id: number, emulateur?: string, version?: string) {
  const j = tele.jeux[id];
  for (let essai = 0; essai < 2; essai++) {
    try {
      // Un souci ici n'empêche JAMAIS le jeu de se lancer.
      await suivreConseil(id, j?.plateforme, emulateur).catch(() => {});
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
      );
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

/** Le nom montré de la combinaison de la manette qui ouvre le menu (réglage du PC ; « Select + R1 » = View + RB). */
function nomCombinaison(): string {
  const c = etat.pc.menuJeu?.manette || 'Select + R1';
  return c.replace('Select', 'View').replace('R1', 'RB').replace('L1', 'LB');
}

/**
 * « 🕹 Jouer à la manette » (jeu PC lancé sans émulateur, fait pour le clavier) : Frogtend traduit les boutons en
 * touches pendant la partie. Un préréglage courant, ou bouton par bouton ; désactivé d'office.
 */
export async function choisirClavier(id: number) {
  const j = tele.jeux[id];
  if (!j) return;
  const actuel = nettoyer(etat.profil.clavier[String(id)]);
  type Choix = 'non' | 'perso' | `pre:${string}`;
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
    ],
    `Seulement pour un jeu fait pour le clavier, qui ne voit pas ta manette : Frogtend appuie sur les touches pour toi. Actuellement : ${libelleClavier(actuel)}. Le menu de Frogtend (${nomCombinaison()}) reste à sa place.`,
  );
  if (!c) return;
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
      const t = await choisir<string>(`⌨ Quelle touche pour ${nomBouton} ?`, [
        { valeur: '', libelle: '⬜ Rien', detail: 'ce bouton n’envoie aucune touche' },
        ...TOUCHES.map((k) => ({ valeur: k, libelle: k, detail: detailTouche(k) })),
      ]);
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

/** Le menu « ⚙ Gérer le jeu » : toutes les actions possibles sur un jeu du PC, au même endroit, bien visibles. */
export async function gererJeu(id: number) {
  const j = tele.jeux[id];
  if (!j) return;
  type Action = 'installer' | 'lanceur' | 'version' | 'emulateur' | 'commandes' | 'figer' | 'pleinEcran' | 'clavier' | 'abri' | 'retirer';
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
  // Un jeu lancé sans émulateur (jeu PC) : jouer à la manette, son plein écran au lancement, le figer pendant le menu.
  // « Commandes » ne concerne que les émulateurs : pas pour lui (deux entrées « manette » se confondraient).
  const pcSansEmulateur = !!j.installation && !j.installation.fichier_du_jeu && !pourLeJeu(etat.pc.emulateurs, etat.pc.emulateursJeux, j.plateforme, id);
  if (!pcSansEmulateur) options.push({ valeur: 'commandes', libelle: '🎮 Commandes', detail: libelleCommandes(etat.profil.commandes[String(id)]) });
  if (pcSansEmulateur) {
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
  if (j.installation)
    options.push({ valeur: 'abri', libelle: '💾 Mettre mes parties à l’abri', detail: 'une copie de ce qui a changé depuis l’installation' });
  if (j.etat === 'telecharge')
    options.push(
      estImporte(j)
        ? { valeur: 'retirer', libelle: '🗑 Retirer de la ludothèque', detail: 'ses fichiers restent sur ton disque' }
        : { valeur: 'retirer', libelle: '🗑 Retirer du PC (désinstaller)', detail: 'tes parties sont copiées à l’abri d’abord' },
    );
  const c = await choisir<Action>(`⚙ Gérer « ${j.titre} »`, options, `Rangé dans ${j.installation?.dossier ?? j.dossier}`);
  if (c === 'installer') await installer(id);
  else if (c === 'lanceur') await choisirLanceur(id);
  else if (c === 'version') await choisirVersionParDefaut(id);
  else if (c === 'emulateur') await choisirEmulateurDuJeu(id);
  else if (c === 'commandes') await choisirCommandes(id);
  else if (c === 'figer') await choisirFiger(id);
  else if (c === 'pleinEcran') await choisirPleinEcran(id);
  else if (c === 'clavier') await choisirClavier(id);
  else if (c === 'abri') await mettreALAbri(id);
  else if (c === 'retirer') await retirer(j);
}
