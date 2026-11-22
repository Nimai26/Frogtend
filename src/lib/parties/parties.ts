// 📁 Où sont mes parties (lot S1, 22/11) : les textes de l'écran, d'après la réponse du cœur (`parties_emplacements`).
// Lot S2 : les textes de « 🛟 Mettre mes parties à l'abri » et de la liste des abris (`parties_abri_apercu`,
// `parties_abriter`, `parties_abris`).
import { taille, type AbriPartiesFait, type AbriVu, type ApercuAbriParties, type EchecAbriVu, type EmplacementVu, type PartiesVues } from '$lib/api';

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

/** D'où vient un emplacement, en mots (la source précise de Firehouse : dans `titreOrigine`, au survol seulement). */
export function libelleOrigine(origine: string): string {
  switch (origine) {
    case 'firehouse':
      return 'd’après Firehouse';
    case 'moteur':
      return 'd’après les fichiers du jeu';
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
  return `${e.fichiers.toLocaleString('fr-FR')} fichier${e.fichiers > 1 ? 's' : ''} · ${taille(e.octets)}`;
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
  return [`${n ? '⚠' : '✅'} ${fichiers(r.fichiers)} de parties (${taille(r.octets)}) mis à l’abri dans ton profil.`, ...reste].join(' ');
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

/** D'où vient un abri, et à qui il est. */
export function texteSorteAbri(a: AbriVu): string {
  const sorte =
    a.sorte === 'emplacements'
      ? 'mis à l’abri à la main'
      : a.sorte === 'nuage-steam'
        ? 'parties d’un jeu Steam'
        : a.sorte === 'jeu' || a.sorte === 'retrait'
          ? 'mis à l’abri par Frogtend (avant un mod, une réinstallation ou un retrait)'
          : 'abri';
  return a.a_moi ? `${sorte} · dans ton profil` : `${sorte} · ancien abri, sans propriétaire connu`;
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
