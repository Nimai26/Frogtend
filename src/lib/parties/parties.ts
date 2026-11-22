// 📁 Où sont mes parties (lot S1, 22/11) : les textes de l'écran, d'après la réponse du cœur (`parties_emplacements`).
// Lecture seule : rien n'est copié ici (les abris, c'est le lot S2).
import { taille, type EmplacementVu, type PartiesVues } from '$lib/api';

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
