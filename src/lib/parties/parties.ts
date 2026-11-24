// 📁 Où sont mes parties (lot S1, 22/11) : les textes de l'écran, d'après la réponse du cœur (`parties_emplacements`).
// Lot S2 : les textes de « 🛟 Mettre mes parties à l'abri » et de la liste des abris (`parties_abri_apercu`,
// `parties_abriter`, `parties_abris`).
import {
  taille,
  type AbriPartiesFait,
  type AbriVu,
  type ApercuAbriParties,
  type BilanAbriAuto,
  type EchecAbriVu,
  type EmplacementVu,
  type ErreurCoeur,
  type PartiesVues,
} from '$lib/api';

/** Un jeu PC (LaunchBox : « Windows ») : seuls ceux-là ont un « 📁 Où sont mes parties » (un jeu d'émulateur garde ses
 * parties chez son émulateur). */
export function aDesEmplacements(plateforme: string | null | undefined): boolean {
  return plateforme === 'Windows';
}

/** Le jeu de `?jeu=` : un entier non nul, sinon `null` (absent, vide, illisible : rien n'est demandé). */
export function jeuDeLAdresse(v: string | null | undefined): number | null {
  const t = (v ?? '').trim();
  const n = t ? Number(t) : NaN;
  return Number.isInteger(n) && n !== 0 ? n : null;
}

/** D'où vient un emplacement, en mots (la source précise de Firehouse : dans `titreOrigine`, au survol seulement).
 * Lot S6 : un emplacement reconnu d'après le moteur du jeu dit le nom du moteur (il vient des références de Firehouse). */
export function libelleOrigine(origine: string, moteur?: string | null): string {
  switch (origine) {
    case 'firehouse':
      return 'd’après Firehouse';
    case 'moteur':
      return moteur?.trim() ? `📁 d’après le moteur ${moteur.trim()}` : '📁 d’après les fichiers du jeu';
    case 'manifeste':
      return 'd’après le jeu lui-même';
    case 'personne':
      return 'ajouté par toi';
    default:
      return 'origine inconnue';
  }
}

/** La source précise d'un emplacement venu de Firehouse (contrat 1.48), pour un `title`. */
export function titreOrigine(origine: string, source: string | null | undefined): string | undefined {
  if (origine !== 'firehouse') return undefined;
  if (source === 'ludusavi') return 'Source : Ludusavi';
  if (source === 'pcgamingwiki' || source == null) return 'Source : PCGamingWiki';
  return undefined;
}

/** Les emplacements montrés en carte : ceux qui ont un vrai dossier sur ce PC. */
export function montres(v: PartiesVues): EmplacementVu[] {
  return v.emplacements.filter((e) => e.lisible && !!e.dossier);
}

/** Les autres endroits connus (illisibles ici, ou sans dossier sur ce PC), dits en une phrase sous la liste. */
export function texteAutresEndroits(v: PartiesVues): string | null {
  const n = v.emplacements.length - montres(v).length;
  if (n <= 0) return null;
  return n === 1 ? '1 autre endroit connu ne concerne pas ce PC.' : `${n} autres endroits connus ne concernent pas ce PC.`;
}

/** « Steam », « Mac ou Linux » → « Steam, Mac ou Linux » (chaque nom une fois). */
function versions(l: string[]): string {
  const noms = [...new Set(l.flatMap((x) => x.split(' ou ')))];
  return noms.length > 1 ? `${noms.slice(0, -1).join(', ')} ou ${noms[noms.length - 1]}` : (noms[0] ?? '');
}

/** La phrase d'état (`null` : rien à dire de plus que la liste). */
export function texteEtat(v: PartiesVues): string | null {
  const rien = !montres(v).length;
  if (v.jeton_refuse && rien)
    return 'Firehouse a refusé le jeton de ce profil : Frogtend ne peut pas lui demander où ce jeu range ses parties. Demande un nouveau jeton à la personne qui gère Firehouse.';
  if ((v.hors_ligne || v.motif) && rien && v.firehouse === 'inconnu')
    return 'Frogtend n’a pas encore pu demander à Firehouse où ce jeu range ses parties. Réessaie une fois connecté.';
  if (v.firehouse === 'inconnu' && rien && v.autres_systemes?.length)
    return `Firehouse connaît l’endroit des parties de ce jeu pour la version ${versions(v.autres_systemes)}, pas encore pour la tienne.`;
  if (v.firehouse === 'inconnu' && rien) return 'Firehouse ne connaît pas encore l’endroit des parties de ce jeu pour ce PC.';
  if (v.firehouse === 'vide' && rien) return 'Firehouse ne connaît aucun endroit de parties de ce jeu pour ce PC.';
  return null;
}

/** Pourquoi la liste vient de ce qui était gardé (hors ligne, Firehouse en panne, jeton refusé), ou `null`. */
export function texteSource(v: PartiesVues): string | null {
  if (!montres(v).length) return null;
  if (v.jeton_refuse) return 'Firehouse a refusé le jeton de ce profil : voici ce que Frogtend savait déjà.';
  if (v.hors_ligne) return 'Hors ligne : voici ce que Frogtend savait déjà.';
  if (v.motif) {
    const m = v.motif.trim();
    return `Firehouse n’a pas pu répondre : ${m}${/[.!?…]$/.test(m) ? '' : '.'} Voici ce que Frogtend savait déjà.`;
  }
  return null;
}

/** Ce que contient un emplacement, en mots. */
export function contenu(e: EmplacementVu): string {
  if (!e.lisible) return 'Frogtend ne sait pas lire cet emplacement sur ce PC.';
  if (!e.dossier) return 'Aucun dossier de parties pour l’instant sur ce PC.';
  if (!e.existe) return 'Pas encore de parties ici (le dossier n’existe pas).';
  if (e.illisible) return 'Le dossier est là, mais Frogtend ne peut pas le lire.';
  if (e.trop) return 'Trop de fichiers pour un dossier de parties : pas comptés.';
  if (!e.fichiers) return 'Le dossier est là, mais vide.';
  const base = `${e.fichiers.toLocaleString('fr-FR')} fichier${e.fichiers > 1 ? 's' : ''} · ${taille(e.octets)}`;
  // Seb 22/11 (Age of Empires IV) : dire tout de suite ce qui n'est pas sur ce PC.
  if ((e.en_ligne ?? 0) >= e.fichiers) return `${base} — dans ton nuage OneDrive, pas sur ce PC`;
  if (e.en_ligne) return `${base} — dont ${e.en_ligne.toLocaleString('fr-FR')} dans ton nuage OneDrive`;
  return base;
}

/** Les fichiers visés, quand ce n'est pas un dossier entier. */
export function texteFiltre(e: EmplacementVu): string | null {
  if (!e.filtre) return null;
  return e.nature === 'motif' ? `seulement les fichiers « ${e.filtre} »` : `seulement le fichier « ${e.filtre} »`;
}

/** La boutique garde aussi ces parties dans son nuage. */
export function texteNuage(e: EmplacementVu): string | null {
  return e.nuage_boutique ? 'aussi gardé dans le nuage de la boutique' : null;
}

/** « Ouvrir le dossier » : seulement un dossier qui existe sur ce PC. */
export function ouvrable(e: EmplacementVu): boolean {
  return e.existe && !!e.dossier;
}

// --- 🛟 Mettre mes parties à l'abri (lot S2) ---

/** Le bouton « 🛟 Mettre mes parties à l'abri » : seulement s'il y a au moins un emplacement lisible ici, avec des
 * fichiers comptés. */
export function abritable(v: PartiesVues): boolean {
  return montres(v).some((e) => e.existe && e.fichiers > 0 && !e.trop && !e.illisible);
}

function fichiers(n: number): string {
  return `${n.toLocaleString('fr-FR')} fichier${n > 1 ? 's' : ''}`;
}

function lignesEchecs(echecs: EchecAbriVu[]): string[] {
  return echecs.map((e) => `⚠ ${e.dossier} : ${e.raison}.`);
}

function ligneEnLigne(n: number): string | null {
  if (!n) return null;
  return n === 1
    ? '1 fichier est seulement dans ton nuage (OneDrive…), pas sur ce PC : pas copié.'
    : `${n} fichiers sont seulement dans ton nuage (OneDrive…), pas sur ce PC : pas copiés.`;
}

/** Le message de la confirmation, AVANT la copie : combien, quelle taille, à qui l'abri appartiendra, ce qui ne sera
 * pas copié. */
export function messageAnnonce(a: ApercuAbriParties): string {
  const endroits = a.emplacements > 1 ? ` (${a.emplacements} endroits)` : '';
  return [
    `${fichiers(a.fichiers)} · ${taille(a.octets)}${endroits}, copiés dans le dossier des abris de ce PC.`,
    `Ces parties seront rangées dans ton profil « ${a.profil} ».`,
    a.gros ? '⚠ C’est beaucoup pour des parties (plus de 2 Go) : la copie peut être longue.' : null,
    ligneEnLigne(a.en_ligne_seulement),
    ...lignesEchecs(a.echecs),
    'Ferme le jeu avant : ses parties ne doivent pas changer pendant la copie.',
  ]
    .filter(Boolean)
    .join('\n');
}

/** Le choix proposé quand des fichiers sont seulement dans OneDrive (Seb, 22/11) : les télécharger puis tout mettre à
 * l'abri, ou (s'il y a aussi des fichiers sur ce PC) mettre à l'abri sans eux. « Annuler » est toujours là. */
export function optionsAbri(a: ApercuAbriParties): { valeur: 'telecharger' | 'local'; libelle: string; detail?: string }[] {
  const l: { valeur: 'telecharger' | 'local'; libelle: string; detail?: string }[] = [
    {
      valeur: 'telecharger',
      libelle: '☁ Télécharger depuis OneDrive puis mettre à l’abri',
      detail: `${fichiers(a.en_ligne_seulement)} · ${taille(a.octets_en_ligne)} à télécharger${a.gros_avec_nuage ? ' (plus de 2 Go en tout : la copie peut être longue)' : ''}`,
    },
  ];
  if (a.fichiers > 0) l.push({ valeur: 'local', libelle: '🛟 Mettre à l’abri sans eux', detail: `${fichiers(a.fichiers)} · ${taille(a.octets)} déjà sur ce PC` });
  return l;
}

/** Le message de ce choix. */
export function messageChoix(a: ApercuAbriParties): string {
  return [
    `${fichiers(a.en_ligne_seulement)} (${taille(a.octets_en_ligne)}) ${a.en_ligne_seulement > 1 ? 'sont' : 'est'} seulement dans ton nuage OneDrive, pas sur ce PC.`,
    'Pour les mettre à l’abri, Frogtend doit d’abord les télécharger ; ils resteront ensuite sur ce PC, comme quand tu les ouvres.',
    `Ces parties seront rangées dans ton profil « ${a.profil} ».`,
    ...lignesEchecs(a.echecs),
    'Ferme le jeu avant : ses parties ne doivent pas changer pendant la copie.',
  ].join('\n');
}

/** Rien à copier (tous les endroits en échec, ou vides) : pourquoi. */
export function messageRien(a: ApercuAbriParties): string {
  return ['Frogtend n’a trouvé aucune partie à copier pour ce jeu sur ce PC.', ligneEnLigne(a.en_ligne_seulement), ...lignesEchecs(a.echecs)].filter(Boolean).join('\n');
}

/** « Déjà à l'abri », avec ce qui n'a pas pu être lu cette fois (`null` : rien à ajouter) : un endroit en échec ou
 * des fichiers « en ligne seulement » ne sont jamais cachés derrière « déjà à l'abri ». */
export function messageDeja(a: ApercuAbriParties): string | null {
  const reste = [ligneEnLigne(a.en_ligne_seulement), ...lignesEchecs(a.echecs)].filter(Boolean);
  return reste.length ? ['Rien n’a changé depuis ton dernier abri, mais :', ...reste].join('\n') : null;
}

/** Le compte rendu, APRÈS (un toast qui s'efface seul : jamais un chemin complet). Commence par « ⚠ » s'il reste un
 * endroit en échec. */
export function texteFait(r: AbriPartiesFait): string {
  if (r.deja || !r.date) return '✅ Déjà à l’abri : rien n’a changé depuis ton dernier abri.';
  const n = r.echecs.length;
  const echecs = n ? `${n === 1 ? '1 endroit n’a' : `${n} endroits n’ont`} pas pu être copié${n > 1 ? 's' : ''} (ouvre « 🛟 Mettre mes parties à l’abri » à nouveau pour voir pourquoi).` : null;
  const reste = [echecs, ligneEnLigne(r.en_ligne_seulement)].filter(Boolean);
  const nuage = r.telecharges ? ` (dont ${r.telecharges.toLocaleString('fr-FR')} téléchargé${r.telecharges > 1 ? 's' : ''} depuis OneDrive)` : '';
  return [`${n ? '⚠' : '✅'} ${fichiers(r.fichiers)} de parties (${taille(r.octets)}) mis à l’abri dans ton profil${nuage}.`, ...reste].join(' ');
}

/** La date d'un abri (secondes depuis 1970), en français. */
export function texteDate(date: string): string {
  const n = Number(date);
  if (!Number.isFinite(n) || n <= 0) return 'date inconnue';
  return new Date(n * 1000).toLocaleString('fr-FR', { dateStyle: 'long', timeStyle: 'short' });
}

/** Ce qu'un abri contient, en mots. */
export function texteContenuAbri(a: AbriVu): string {
  return `${fichiers(a.fichiers)} · ${taille(a.octets)}`;
}

/** Ce qui a fait faire un abri des emplacements (lot S3) : un repère (🖐 à la main, 🏁 après une partie, 🛠 avant une
 * opération) et ses mots. Inconnu ou absent (abri d'avant le lot S3) : « à la main ». */
export function texteDeclencheur(d: string | null | undefined): { repere: string; texte: string } {
  switch (d) {
    case 'apres_partie':
      return { repere: '🏁', texte: 'après une partie' };
    case 'avant_reinstallation':
      return { repere: '🛠', texte: 'avant une réinstallation' };
    case 'avant_mise_a_jour':
      return { repere: '🛠', texte: 'avant une mise à jour' };
    case 'avant_retrait':
      return { repere: '🛠', texte: 'avant de retirer le jeu' };
    case 'avant_mod':
      return { repere: '🛠', texte: 'avant un changement de mod' };
    // Lot S5 : la partie de ce PC, avant d'être remplacée par celle de Firehouse ; puis la partie reprise.
    case 'avant_restauration':
      return { repere: '🛠', texte: 'avant de reprendre la partie de Firehouse' };
    case 'restauree':
      return { repere: '📥', texte: 'reprise de Firehouse' };
    // Lot S7 : une partie jouée hors de Frogtend, rattrapée à l'ouverture du profil.
    case 'rattrapage':
      return { repere: '🛟', texte: 'rattrapé' };
    // Absent (un abri d'avant le lot S3) ou « main » : fait à la main.
    case undefined:
    case null:
    case '':
    case 'main':
      return { repere: '🖐', texte: 'mis à l’abri à la main' };
    // Un déclencheur d'une version plus récente de Frogtend : jamais dit « à la main ».
    default:
      return { repere: '🛟', texte: 'mis à l’abri par Frogtend' };
  }
}

/** D'où vient un abri, et à qui il est. */
export function texteSorteAbri(a: AbriVu): string {
  const d = texteDeclencheur(a.declencheur);
  const sorte =
    a.sorte === 'emplacements'
      ? `${d.repere} ${d.texte}`
      : a.sorte === 'nuage-steam'
        ? 'parties d’un jeu Steam'
        : a.sorte === 'jeu' || a.sorte === 'retrait'
          ? 'mis à l’abri par Frogtend (avant un mod, une réinstallation ou un retrait)'
          : 'abri';
  // Lot S7 : un abri rattrapé qui attend la réponse de la personne, ou qu'elle a dit ne pas être le sien.
  const confirmation = a.a_confirmer ? ' · à confirmer : c’est toi qui as joué ?' : a.pas_a_moi ? ' · pas à toi : gardé sur ce PC, jamais envoyé' : '';
  return a.a_moi ? `${sorte} · dans ton profil${confirmation}` : `${sorte} · ancien abri, sans propriétaire connu`;
}

/** Ce qu'il faut savoir d'un abri (`null` : rien). */
export function noteAbri(a: AbriVu): string | null {
  const l = [
    a.sans_origine
      ? `Frogtend ne sait plus d’où venai${a.sans_origine > 1 ? 'en' : ''}t ${a.sans_origine.toLocaleString('fr-FR')} de ces fichiers : ${a.sans_origine > 1 ? 'ils sont gardés' : 'il est gardé'}, mais il faudra ${a.sans_origine > 1 ? 'les' : 'le'} remettre à la main.`
      : null,
    a.echecs ? `${a.echecs === 1 ? '1 endroit n’avait' : `${a.echecs} endroits n’avaient`} pas pu être copié${a.echecs > 1 ? 's' : ''}.` : null,
    ligneEnLigne(a.en_ligne_seulement),
  ].filter(Boolean);
  return l.length ? l.join(' ') : null;
}

// --- Lot S3 (23/11) : l'abri AUTOMATIQUE (après une partie, avant une opération) ---

/** « aujourd’hui à 21:14 », « hier à 08:05 », sinon « le 21 novembre à 21:14 » (`maintenant` : pour les tests). */
export function texteQuand(date: string, maintenant: Date = new Date()): string {
  const n = Number(date);
  if (!Number.isFinite(n) || n <= 0) return 'à une date inconnue';
  const d = new Date(n * 1000);
  const heure = d.toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' });
  const jour = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const ecart = Math.round((jour(maintenant) - jour(d)) / 86_400_000);
  if (ecart === 0) return `aujourd’hui à ${heure}`;
  if (ecart === 1) return `hier à ${heure}`;
  return `le ${d.toLocaleDateString('fr-FR', { day: 'numeric', month: 'long', year: d.getFullYear() === maintenant.getFullYear() ? undefined : 'numeric' })} à ${heure}`;
}

/** « 🛟 Dernier abri : aujourd’hui à 21:14, après une partie » — le plus récent abri de CE profil (`null` : aucun). */
export function texteDernierAbri(abris: AbriVu[] | undefined, maintenant: Date = new Date()): string | null {
  const a = abris?.find((x) => x.a_moi);
  if (!a) return null;
  const d = texteDeclencheur(a.declencheur);
  const quoi =
    a.sorte === 'emplacements'
      ? d.repere === '🖐'
        ? 'fait à la main'
        : d.texte
      : a.sorte === 'nuage-steam'
        ? 'avec le nuage de Steam'
        : 'par Frogtend (avant un mod, une réinstallation ou un retrait)';
  return `🛟 Dernier abri : ${texteQuand(a.date, maintenant)}, ${quoi}`;
}

/** Combien d'abris sont montrés d'office. */
export const ABRIS_MONTRES = 5;

/** Une liste longue (lot S3 : un abri après chaque partie) : les 5 plus récents, plus le dernier de chaque sorte
 * « avant … » s'il est plus ancien (c'est lui qu'on chercherait après un retrait ou un mod) ; `tous` : toute la liste.
 * Rend aussi le nombre d'abris cachés. L'ordre (du plus récent au plus ancien) est gardé. */
export function abrisAMontrer(liste: AbriVu[], tous = false): { montres: AbriVu[]; caches: number } {
  if (tous || liste.length <= ABRIS_MONTRES) return { montres: liste, caches: 0 };
  const garder = new Set<number>(liste.slice(0, ABRIS_MONTRES).map((_, i) => i));
  const vus = new Set<string>();
  liste.forEach((a, i) => {
    const d = a.sorte === 'emplacements' ? (a.declencheur ?? '') : '';
    if (!d.startsWith('avant_') || vus.has(d)) return;
    vus.add(d);
    garder.add(i);
  });
  const montres = liste.filter((_, i) => garder.has(i));
  return { montres, caches: liste.length - montres.length };
}

/** « 23 abris · 1,2 Go » (`null` : aucun). */
export function texteTotalAbris(liste: AbriVu[] | undefined): string | null {
  if (!liste?.length) return null;
  const octets = liste.reduce((s, a) => s + a.octets, 0);
  return `${liste.length.toLocaleString('fr-FR')} abri${liste.length > 1 ? 's' : ''} · ${taille(octets)}`;
}

/** Les états d'un abri automatique qui méritent d'être dits (un toast, puis une ligne sur la fiche du jeu). */
export function estProblemeAbriAuto(b: BilanAbriAuto): boolean {
  return b.etat === 'en_ligne' || b.etat === 'trop_gros' || b.etat === 'echec' || b.etat === 'attente_depassee' || b.etat === 'inconnu';
}

/** Un motif qui donne une consigne (« réessaie dans une minute ») : après une partie, c'est Frogtend qui réessaie. */
function donneUneConsigne(m: string): boolean {
  return /réessaie|ferme|quitte|choisis|vérifie|attends/i.test(m);
}

/** Le toast d'un abri APRÈS UNE PARTIE (`null` : rien à dire — rien de neuf, aucun emplacement, en cours). Sans
 * chiffres ni chemin pour une réussite ; sans faire peur pour un problème (les parties sont toujours dans le jeu). */
export function texteToastAbriAuto(b: BilanAbriAuto, titre: string): string | null {
  const t = `« ${titre} »`;
  switch (b.etat) {
    case 'fait':
      return `🛟 Tes parties de ${t} sont à l’abri.`;
    case 'en_ligne':
      return `☁ Des parties de ${t} sont seulement dans ton nuage OneDrive : Frogtend ne les télécharge pas tout seul. Ouvre « 📁 Où sont mes parties » pour les mettre à l’abri.`;
    case 'trop_gros':
      return `🛟 Les parties de ${t} font plus de 2 Go : Frogtend ne les copie pas tout seul. Ouvre « 📁 Où sont mes parties » pour les mettre à l’abri.`;
    case 'echec':
      return `🛟 Frogtend n’a pas pu mettre les parties de ${t} à l’abri cette fois (elles sont toujours dans le jeu). Il réessaiera après ta prochaine partie.`;
    // Attente dépassée, emplacement inconnu : sans bruit (Seb, 23/11) — la ligne de la fiche suffit.
    default:
      return null;
  }
}

/** La ligne qui RESTE sur la fiche du jeu après un abri automatique qui n'a pas pu tout copier (`null` : rien). */
export function ligneAbriAuto(b: BilanAbriAuto | null | undefined): string | null {
  if (!b || !estProblemeAbriAuto(b)) return null;
  // Lot S7 : le rattrapage ne se refait qu'à l'ouverture de Frogtend (jamais après une partie).
  const quand = b.declencheur === 'rattrapage' ? 'à la prochaine ouverture de Frogtend' : 'après ta prochaine partie';
  switch (b.etat) {
    case 'en_ligne':
      return `☁ Dernier abri automatique : ${fichiers(b.en_ligne)} (${taille(b.octets_en_ligne)}) seulement dans ton nuage OneDrive, pas copié${b.en_ligne > 1 ? 's' : ''}. « 🛟 Mettre mes parties à l’abri » propose de les télécharger.`;
    case 'trop_gros':
      return `⚠ Dernier abri automatique : plus de 2 Go de parties, rien de copié sans ton accord. « 🛟 Mettre mes parties à l’abri » te le demandera.`;
    case 'attente_depassee':
      return `⏳ Dernier abri automatique : le jeu écrivait encore ses parties. Frogtend réessaiera ${quand}.`;
    case 'inconnu':
      return `❔ Dernier abri automatique : Frogtend ne sait pas encore où ce jeu range ses parties. Il redemandera à Firehouse ${quand}.`;
    default: {
      // Une consigne (« réessaie dans une minute ») n'est pas recopiée : c'est Frogtend qui réessaiera.
      const m = b.motif && !donneUneConsigne(b.motif) ? ` : ${b.motif.replace(/[.\s]+$/, '')}` : '';
      return `⚠ Dernier abri automatique impossible${m}. Frogtend réessaiera ${quand}.`;
    }
  }
}

/** L'opération qu'un abri impossible a arrêtée, en mots. */
export function texteOperation(op: string | null | undefined): string {
  switch (op) {
    case 'avant_retrait':
      return 'le retrait du jeu';
    case 'avant_reinstallation':
      return 'la réinstallation';
    case 'avant_mise_a_jour':
      return 'la mise à jour';
    case 'avant_mod':
      return 'le changement de mod';
    default:
      return 'l’opération';
  }
}

/** Ce qu'on peut faire quand l'abri d'AVANT une opération a échoué (Seb, 23/11), dans cet ordre : Réessayer ;
 * Télécharger depuis OneDrive (quand c'est la cause, taille annoncée) ou les copier quand même (plus de 2 Go) ;
 * Continuer sans abri. « Annuler » est toujours là (la fenêtre de choix l'ajoute). */
export type ChoixAbriImpossible = 'reessayer' | 'onedrive' | 'quand_meme' | 'sans_abri';

export function optionsAbriImpossible(e: ErreurCoeur): { valeur: ChoixAbriImpossible; libelle: string; detail?: string }[] {
  const l: { valeur: ChoixAbriImpossible; libelle: string; detail?: string }[] = [];
  // L'option qui règle la cause d'abord (elle reçoit le focus), puis « Réessayer ».
  if (e.cause === 'onedrive' && (e.fichiers_en_ligne ?? 0) > 0)
    l.push({ valeur: 'onedrive', libelle: '☁ Télécharger depuis OneDrive puis réessayer', detail: `${fichiers(e.fichiers_en_ligne ?? 0)} · ${taille(e.octets_en_ligne ?? 0)} à télécharger` });
  if (e.cause === 'trop_gros') l.push({ valeur: 'quand_meme', libelle: '🛟 Les mettre à l’abri quand même', detail: `${taille(e.octets ?? 0)} : la copie peut être longue` });
  const detail =
    e.cause === 'echec' ? 'si le jeu est ouvert, quitte-le d’abord' : e.cause === 'inconnu' ? 'Frogtend redemande à Firehouse où sont tes parties' : undefined;
  l.push({ valeur: 'reessayer', libelle: '🔁 Réessayer', detail });
  l.push({ valeur: 'sans_abri', libelle: 'Continuer sans abri', detail: `${texteOperation(e.operation)} sans copier tes parties d’abord` });
  return l;
}

/** Le message de cette fenêtre. */
export function messageAbriImpossible(e: ErreurCoeur): string {
  return [e.motif, `Rien n’a été fait : ${texteOperation(e.operation)} attend que tes parties soient à l’abri.`].join('\n\n');
}

/** La seconde confirmation de « Continuer sans abri ». */
export function messageSansAbri(e: ErreurCoeur): string {
  const op = texteOperation(e.operation);
  return `Tes parties de ce jeu ne seront pas copiées avant ${op}. Si ${op} les efface ou les abîme, Frogtend ne pourra pas te les rendre.`;
}
