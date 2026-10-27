// Contrat 1.32 (besoin 16) : chercher dans la base de Firehouse les jeux importés de ton disque qui n'ont pas de fiche.
// Seul un « exact » est relié tout seul ; un « probable » t'est toujours proposé (jamais le premier d'office).

/** Un jeu de la base LaunchBox proposé par Firehouse. */
export interface CandidatBase {
  launchbox_id: number;
  titre: string;
  plateforme: string;
  annee: number | null;
  /** Ce jeu a une vraie fiche dans ta ludothèque : choisi, ton jeu y est relié. */
  fiche_id: number | null;
}

/** Un jeu importé à faire choisir. */
export interface AChoisir {
  local: number;
  titre: string;
  plateforme: string;
  candidats: CandidatBase[];
}

/** La fiche « de base » d'un jeu (contrat 1.32). */
export interface FicheBase {
  titre: string;
  titre_fr?: string | null;
  resume?: string | null;
  /** `fr`, `en`, ou vide. */
  resume_langue?: string | null;
  genres?: string[];
  developpeur?: string | null;
  editeur?: string | null;
  annee?: number | null;
}

/** Le libellé d'un candidat : « Titre (année) — console ». `court` : le nom court de la console. */
export function libelleCandidat(c: CandidatBase, court: (plateforme: string) => string): string {
  return `${c.titre}${c.annee ? ` (${c.annee})` : ''} — ${court(c.plateforme)}`;
}

/** Ce qu'on annonce avant d'envoyer : seuls des NOMS partent, rien n'est lu ni modifié. */
export function texteAnnonceBase(nouveaux: number, cherches: number): string {
  if (nouveaux === 0) {
    return cherches ? `Tes ${cherches} jeu(x) importé(s) sans fiche ont déjà été cherchés.` : 'Aucun jeu importé sans fiche à chercher.';
  }
  return `${nouveaux} jeu(x) importé(s) n’ont pas de fiche. Frogtend peut les chercher dans la base de Firehouse : seuls leurs noms (et celui de leur dossier) partent, aucun fichier n’est lu ni modifié. Les jeux trouvés à coup sûr reçoivent leur jaquette et leur résumé ; pour les autres, tu choisis.`;
}
