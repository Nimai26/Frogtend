// 0.229.1 (MK8 Deluxe, Seb 26/11) : une installation qui n'est qu'une NOTE (méthode « aucune », sans lanceur ni fichier
// donné à un émulateur) se défait sans rien effacer — après « Annuler » au choix du lanceur, et pour la refaire quand
// le jeu a été pris pour un jeu PC alors qu'il se joue par un émulateur. Le cœur reste juge (`installation_annuler`
// refuse toute autre installation) ; ces règles décident seulement de ce qui est proposé.
import type { Installation } from '$lib/api';

/** L'installation n'est-elle qu'une note (la défaire ne touche aucun fichier) ? Même règle que `note_seule` du cœur. */
export function noteSeule(i: Installation | null | undefined): boolean {
  return !!i && i.methode.sorte === 'aucune' && !i.lanceur && !i.fichier_du_jeu && !i.versions?.length && !i.boutique && !i.extraits?.length;
}

/**
 * Après une installation tout juste notée, le choix de ce qui lance le jeu : s'il n'aboutit pas (« Annuler », Taodbox
 * qui renvoie au bureau, refus) et que l'installation n'est toujours qu'une note, elle est DÉFAITE — le jeu redevient
 * « téléchargé, pas installé », « 📦 Installer » revient, et rien n'est effacé sur le disque.
 */
export async function choisirOuDefaire(
  id: number,
  deps: {
    choisirLanceur: (id: number) => Promise<boolean>;
    installationDe: (id: number) => Installation | null | undefined;
    annuler: (id: number) => Promise<void>;
  },
): Promise<boolean> {
  if (await deps.choisirLanceur(id)) return true;
  if (noteSeule(deps.installationDe(id))) {
    try {
      await deps.annuler(id);
    } catch {
      // Refusé par le cœur (un lanceur choisi entre-temps…) : rien n'est changé.
    }
  }
  return false;
}

/** L'entrée « 📦 Refaire l'installation » de « ⚙ Gérer le jeu », quand le cœur la dit à refaire. */
export const OPTION_REFAIRE = {
  libelle: '📦 Refaire l’installation (rien n’est retéléchargé)',
  detail: 'ce jeu se joue par un émulateur : Frogtend le prépare de nouveau, sans rien effacer',
} as const;

/**
 * Faut-il demander au cœur si l'installation est à refaire ? Seulement pour un jeu de Firehouse téléchargé dont
 * l'installation n'est qu'une note : le cœur relit alors sa fiche (ROM ou image disque ?).
 */
export function refaireAVerifier(id: number, j: { etat: string; installation?: Installation | null } | undefined): boolean {
  return !!j && id > 0 && j.etat === 'telecharge' && noteSeule(j.installation);
}

/** « Refaire l'installation » : la note défaite, puis l'installation normale (pour une image disque : son fichier noté). */
export async function refaire(
  id: number,
  deps: { annuler: (id: number) => Promise<void>; recharger: () => Promise<void>; installer: (id: number) => Promise<boolean> },
): Promise<boolean> {
  await deps.annuler(id);
  await deps.recharger();
  return deps.installer(id);
}
