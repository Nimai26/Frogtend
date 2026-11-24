// Lot S5 (24/11) : « 📥 Reprendre mes parties de Firehouse » sur UN jeu (bureau et Taodbox), et la fenêtre de conflit
// (une partie de ce PC différente de celle de Firehouse : TOUJOURS une question, jamais un choix par les dates).
import { listen } from '@tauri-apps/api/event';
import { api, taille, type ConflitEmplacement } from '$lib/api';
import { choisir, confirmer, deciderConflit, fermerToast, informer, majProgres, toast, toastProgres, type ChoixConflit } from '$lib/dialogues/fenetres.svelte';
import { motifDuRefus } from '$lib/dialogues/messages';
import { dateConflit, colonnesConflit, lignesBilan, messageConflit, pcDuProfil, pl, TEXTE_SEL_ECART, titresCites } from './reprise';

/** Au-delà, un choix groupé d'abord (Seb, 24/11). */
export const SEUIL_GROUPE = 3;

/** Une reprise à la fois (gardien) ; `abriEnCours` : une mise à l'abri à la main (les deux boutons se grisent l'un
 * pendant l'autre). */
export const reprise = $state<{ enCours: number | null; abriEnCours: number | null }>({ enCours: null, abriEnCours: null });

type Groupe = 'garder' | 'reprendre' | 'une' | 'plus_tard';

/** Règle les conflits un par un (fenêtre à deux colonnes), ou d'un coup au-delà de 3. Un « Décider plus tard » dans la
 * série une par une arrête la série (les suivantes restent en attente, et c'est dit). Rend le nombre réglé. */
export async function reglerConflits(conflits: ConflitEmplacement[]): Promise<number> {
  if (!conflits.length) return 0;
  let groupe: Groupe = 'une';
  if (conflits.length > SEUIL_GROUPE) {
    const c = await choisir<Groupe>(
      `❓ ${conflits.length} parties différentes de celles de ce PC`,
      [
        { valeur: 'garder', libelle: '💻 Garder toutes celles de ce PC', detail: 'rien n’est remplacé' },
        { valeur: 'une', libelle: '🔎 Choisir une par une' },
        { valeur: 'reprendre', libelle: '🔥 Reprendre toutes celles de Firehouse', detail: `${titresCites(conflits.map((x) => x.titre))} — celles de ce PC mises à l’abri d’abord` },
        { valeur: 'plus_tard', libelle: '⏳ Décider plus tard' },
      ],
      `Les jeux : ${titresCites(conflits.map((x) => x.titre))}.\nRien n’a été remplacé. Tu peux aussi décider plus tard : « 📥 Reprendre mes parties de Firehouse » te reposera la question.`,
    );
    if (!c || c === 'plus_tard') return 0;
    groupe = c;
  }
  let gardees = 0;
  let reprises = 0;
  let sansAbriApres = 0;
  let restent = 0;
  const echecs: string[] = [];
  for (const [i, c] of conflits.entries()) {
    let choix: ChoixConflit;
    if (groupe === 'une') {
      const { ici, firehouse } = colonnesConflit(c, taille);
      const suite = conflits.length - i - 1;
      const message = messageConflit(c) + (suite ? `\n(« Décider plus tard » laisse aussi ${suite} autre${suite > 1 ? 's' : ''} en attente.)` : '');
      choix = await deciderConflit(c.titre ? `❓ ${c.titre} : quelle partie garder ?` : '❓ Quelle partie garder ?', { message, ici, firehouse });
    } else {
      choix = groupe === 'garder' ? 'ici' : 'firehouse';
    }
    if (choix === 'plus_tard') {
      restent = conflits.length - i;
      break;
    }
    try {
      const fait = await api.partiesConflitRegler(c.jeu, c.cle, choix === 'ici' ? 'garder' : 'reprendre');
      if (choix === 'ici') gardees++;
      else {
        reprises++;
        if (!fait) sansAbriApres++;
      }
    } catch (e) {
      echecs.push(`${c.titre || 'Un jeu'} : ${motifDuRefus(e)}`);
    }
  }
  const lignes = [
    reprises
      ? `🔥 ${reprises} ${pl(reprises, 'partie reprise', 'parties reprises')} de Firehouse ; ${pl(reprises, 'celle de ce PC est', 'celles de ce PC sont')} à l’abri (🛟 Mes abris).`
      : '',
    gardees ? `💻 ${gardees} ${pl(gardees, 'partie de ce PC gardée', 'parties de ce PC gardées')}.` : '',
    restent ? `⏳ ${restent} ${pl(restent, 'partie attend', 'parties attendent')} ta décision : « 📥 Reprendre mes parties de Firehouse » te reposera la question.` : '',
    sansAbriApres
      ? `⚠ ${sansAbriApres} ${pl(sansAbriApres, 'partie reprise n’a', 'parties reprises n’ont')} pas pu être mise${sansAbriApres > 1 ? 's' : ''} à l’abri à son tour : Frogtend ne l’enverra pas à Firehouse tant qu’elle n’est pas à l’abri (🛟 Mettre mes parties à l’abri).`
      : '',
    echecs.length ? `⚠ ${pl(echecs.length, 'Pas réglée', 'Pas réglées')} (rien n’a changé) :\n${echecs.slice(0, 8).join('\n')}` : '',
  ].filter(Boolean);
  if (lignes.length && (reprises || gardees || echecs.length)) await informer('💾 Tes parties', lignes.join('\n\n'));
  return gardees + reprises;
}

/** Les conflits d'un jeu (ou de tous), réglés avec la personne ; une lecture impossible est dite, jamais tue. */
export async function reglerConflitsDe(id: number | null): Promise<number> {
  try {
    return await reglerConflits(await api.partiesConflits(id));
  } catch (e) {
    toast(`Impossible de lire les parties à départager : ${motifDuRefus(e)}`, 'erreur');
    return 0;
  }
}

/** Le PC d'une sauvegarde de ce profil : le plus récent (`derniere.date`) en premier, un choix seulement s'il y en a
 * plusieurs. `null` : aucune, ou la personne a fermé. */
export async function choisirPcDuProfil<T extends { pc: string; a_moi: boolean; derniere: { date?: string } | null }>(liste: T[], titre: string): Promise<T | null> {
  const miennes = pcDuProfil(liste);
  if (miennes.length <= 1) return miennes[0] ?? null;
  return choisir(
    titre,
    miennes.map((x, i) => ({
      valeur: x,
      libelle: `${i === 0 ? '⭐ ' : ''}${x.pc}`,
      detail: x.derniere?.date ? `sauvegardé le ${dateConflit(Number(x.derniere.date))}` : 'date inconnue',
    })),
    'Ta sauvegarde la plus récente est la première.',
  );
}

const mo = (o: number) => `${(o / (1024 * 1024)).toLocaleString('fr-FR', { maximumFractionDigits: 1 })} Mo`;

/** « 📥 Reprendre mes parties de Firehouse » sur un jeu : le PC (le plus récent par défaut, un choix seulement s'il y en
 * a plusieurs), l'annonce (fichiers, Mo), puis SEULEMENT ses parties téléchargées et posées sans rien écraser, puis les
 * conflits. Rend vrai si quelque chose a été téléchargé. */
export async function reprendreMesParties(id: number): Promise<boolean> {
  if (reprise.enCours !== null || reprise.abriEnCours !== null) return false;
  reprise.enCours = id;
  try {
    const liste = await api.restaurationListe().catch((e) => {
      toast(`Impossible de lire tes sauvegardes : ${motifDuRefus(e)}`, 'erreur');
      return null;
    });
    if (!liste) return false;
    if (!pcDuProfil(liste).length) {
      await informer('📭 Aucune sauvegarde de ce profil', 'Firehouse n’a encore aucune sauvegarde de ce profil : rien à reprendre.');
      return false;
    }
    const s = await choisirPcDuProfil(liste, '📥 Depuis quel PC ?');
    if (!s) return false;
    const a = await api.partiesRepriseApercu(id, s.profil, s.pc);
    if (!a.fichiers) {
      await informer('📭 Rien à reprendre', `Firehouse n’a aucune partie de ce jeu dans la sauvegarde de ${s.pc}.`);
      return false;
    }
    const oui = await confirmer('📥 Reprendre mes parties de Firehouse ?', {
      message: `${a.fichiers} ${pl(a.fichiers, 'fichier', 'fichiers')}, ${taille(a.octets)} à télécharger depuis la sauvegarde de ${s.pc}.\nCe qui manque sur ce PC est remis à sa place ; une partie DIFFÉRENTE de celle de ce PC n’est jamais remplacée sans te demander.`,
      libelleValider: '📥 Reprendre',
    });
    if (!oui) return false;
    const attente = toastProgres('📥 Téléchargement de tes parties…');
    let arreter: (() => void) | null = null;
    try {
      arreter = await listen<{ jeu: number; faits: number; total: number; octets_faits: number; octets_total: number }>('parties-reprise-progres', (e) => {
        const { jeu, faits, total, octets_faits, octets_total } = e.payload;
        if (jeu !== id) return;
        majProgres(attente, {
          etape: `Téléchargement de tes parties… ${mo(octets_faits)} sur ${mo(octets_total)}`,
          chiffres: `${faits}/${total} ${pl(total, 'fichier', 'fichiers')}`,
          pourcent: octets_total ? Math.round((100 * octets_faits) / octets_total) : 0,
        });
      }).catch(() => null);
      const b = await api.partiesReprendre(id, s.profil, s.pc);
      fermerToast(attente);
      if (b.sel_ecart) toast(TEXTE_SEL_ECART, 'alerte');
      const lignes = lignesBilan(b);
      const attend = b.non_installes + b.non_resolues + b.identifiants + b.en_cours + b.partielles + (b.incompletes ?? 0);
      if (attend) await informer('📥 Tes parties de Firehouse', lignes.join('\n'));
      else if (!b.conflits) toast(lignes.join(' ') || '✅ Tes parties sont déjà à leur place.', 'ok');
      else {
        // Il reste des conflits : jamais « déjà à leur place » ; ce qui est posé est dit, la question suit.
        const poses = lignes.filter((l) => !l.startsWith('❓'));
        if (poses.length) toast(poses.join(' '), 'ok');
      }
      if (b.conflits) await reglerConflitsDe(id);
      return true;
    } finally {
      arreter?.();
      fermerToast(attente);
    }
  } catch (e) {
    toast(`Échec de la reprise de tes parties : ${motifDuRefus(e)}`, 'erreur');
    return false;
  } finally {
    reprise.enCours = null;
  }
}
