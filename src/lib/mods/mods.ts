// Les mods d'un jeu (contrat 1.19 ; ✅ Seb 19/10) dits pour des gens qui veulent juste jouer : ce qui s'installe d'un
// clic, pourquoi un autre ne s'installe pas, et ce qu'il faut savoir AVANT de dire oui (taille, partage, version
// d'essai, page déplacée, empreinte absente). Le cœur reprend toujours le mod dans la liste de Firehouse.

/** Un fichier de mod servi par Firehouse (contrat 1.19). */
export interface FichierMod {
  nom: string;
  taille: number;
  sha256: string | null;
  url: string;
  licence: string | null;
  licence_libre: boolean | null;
  pre_version: boolean;
  depot: string | null;
  depot_deplace: boolean;
  methode: string;
  cible: { base: string; dossier: string } | null;
  a_confirmer: boolean;
  consignes_url: string | null;
}

/** Un mod d'un jeu (contrat 1.19). */
export interface ModJeu {
  nom: string;
  description?: string;
  liens?: string[];
  /** Liens seulement CITÉS par la page : jamais à télécharger. */
  references?: string[];
  page?: string;
  consignes_url?: string | null;
  url?: string;
  fichiers?: FichierMod[];
  refus?: string | null;
  refus_detail?: string | null;
}

/** Le fichier que Frogtend sait installer d'un clic (une archive zip à extraire dans le jeu), sinon `null`. */
export function fichierInstallable(m: ModJeu): FichierMod | null {
  if (m.refus) return null;
  return (
    (m.fichiers ?? []).find(
      (f) => f.methode === 'extraire' && f.cible?.base === 'jeu' && f.nom.toLowerCase().endsWith('.zip') && f.taille > 0 && f.url.startsWith('https://'),
    ) ?? null
  );
}

/** Pourquoi un mod ne s'installe pas d'un clic, dit simplement. */
export function raisonSansClic(m: ModJeu): string {
  switch (m.refus) {
    case 'anti_triche':
      return 'Pas possible : ce jeu est protégé contre la triche en ligne, un mod pourrait faire bannir ton compte.';
    case 'sans_release_precise':
    case 'liens_contradictoires':
      return 'Frogtend ne sait pas quel fichier prendre : la page du mod l’explique.';
    case 'github_indisponible':
      return 'La page du mod ne répond pas pour l’instant : réessaie plus tard.';
    case null:
    case undefined:
      return (m.fichiers ?? []).length ? 'Ce mod s’installe à la main : la page du mod explique comment.' : 'Pas de fichier à installer : c’est un réglage à faire, la page l’explique.';
    default:
      return m.refus_detail || 'Pas installable d’un clic : la page du mod l’explique.';
  }
}

/** Ce qu'il faut savoir avant de dire oui (jamais bloquant). */
export function avertissements(f: FichierMod): string[] {
  const l: string[] = [];
  if (f.licence_libre) l.push(`Partage : libre${f.licence ? ` (licence ${f.licence})` : ''}.`);
  else l.push(`Partage : ${f.licence ? `licence « ${f.licence} »` : 'non précisé par l’auteur'}.`);
  if (f.pre_version) l.push('⚠ Version d’essai : l’auteur ne la dit pas terminée.');
  if (f.depot_deplace) l.push('⚠ L’auteur a déplacé sa page : vérifie que c’est bien le bon mod.');
  if (!f.sha256) l.push('⚠ Frogtend ne peut pas vérifier que le fichier est intact, seulement sa taille.');
  return l;
}

/** Une adresse web qu'on peut ouvrir (http ou https seulement). */
export function adresseWeb(url: string | null | undefined): string | null {
  return url && /^https?:\/\//i.test(url) ? url : null;
}

/** Une page de mod sur Nexus (`https://www.nexusmods.com/<jeu>/mods/<n>`) : son jeu et son numéro, sinon `null`. */
export function pageNexus(url: string | null | undefined): { domaine: string; mod: number } | null {
  const m = /^https:\/\/(?:www\.)?nexusmods\.com\/([a-z0-9]{1,64})\/mods\/(\d+)(?:[/?#]|$)/.exec(url ?? '');
  return m ? { domaine: m[1], mod: Number(m[2]) } : null;
}
