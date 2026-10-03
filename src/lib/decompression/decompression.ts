// « Décompresser pour jouer » : un jeu importé rangé dans une archive (.zip, .7z) qui contient une image disque.
// Les émulateurs de consoles à disques ne la lisent pas telle quelle : Frogtend propose de la décompresser une fois,
// à côté de l'archive, seulement si la personne dit oui (taille, dossier et durée annoncés). L'archive est gardée.

import { taille } from '$lib/api';

export interface ArchiveDeJeu {
  archive: string;
  fichiers: number;
  taille: number;
  principal: string;
  destination: string;
  deja: boolean;
}

/** Débit prudent d'une décompression sur un disque dur (octets par seconde) : sert à annoncer une durée, pas plus. */
const DEBIT = 100 * 1024 * 1024;

/** « environ 8 minutes », « moins d'une minute ». */
export function dureeEstimee(octets: number): string {
  const minutes = Math.round(octets / DEBIT / 60);
  if (minutes < 1) return 'moins d’une minute';
  if (minutes < 60) return `environ ${minutes} minute${minutes > 1 ? 's' : ''}`;
  const h = Math.floor(minutes / 60);
  const m = Math.round((minutes % 60) / 5) * 5;
  return `environ ${h} h${m ? ` ${String(m).padStart(2, '0')}` : ''}`;
}

/** Le message de la confirmation. */
export function messageConfirmation(a: ArchiveDeJeu, libre: number | null): string {
  return [
    `${taille(a.taille)} à écrire (${a.fichiers} fichier${a.fichiers > 1 ? 's' : ''}), ${dureeEstimee(a.taille)}.`,
    `Dossier : ${a.destination}`,
    libre != null ? `Place libre sur ce disque : ${taille(libre)}.` : '',
    'L’archive est gardée telle quelle : tu pourras la supprimer toi-même une fois le jeu essayé.',
    'Les fichiers gardent leur nom ; rien n’est écrit par-dessus un fichier existant.',
  ]
    .filter(Boolean)
    .join('\n');
}
