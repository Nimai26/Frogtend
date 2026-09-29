// Les appels au cœur de Frogtend (Rust). Le jeton ne passe jamais par ici : seul le cœur le connaît.

import { invoke, isTauri } from '@tauri-apps/api/core';

export interface Profil {
  id: string;
  nom: string;
  protege: boolean;
  a_un_jeton?: boolean;
}

export interface Plateforme {
  nom: string;
  jeux: number;
}

export interface JeuResume {
  id: number;
  titre: string;
  annee: number | null;
  plateforme: string;
  genres: string[];
  developpeur: string | null;
  editeur: string | null;
  statut: string | null;
  /** Firehouse a-t-il une jaquette pour ce jeu ? */
  jaquette?: boolean | null;
  /** Nombre de versions rangées dans Firehouse. */
  versions?: number | null;
}

export interface Liste {
  jeux: JeuResume[];
  total: number;
  total_ludotheque: number;
}

export interface Filtre {
  plateforme?: string | null;
  texte?: string | null;
  genre?: string | null;
  tri?: 'titre' | 'annee' | 'annee_desc';
  limite?: number;
  decalage?: number;
}

export interface FichierVersion {
  nom: string;
  taille: number;
}

export interface Version {
  telechargement_id: number;
  nom: string;
  qualite: string;
  type_source?: string;
  range_le?: string;
  fichiers: FichierVersion[];
  notes?: string;
  notes_origine?: string;
  notes_a_traduire?: boolean;
}

export interface Annexe {
  i: number;
  cle: string;
  type: string;
  titre: string;
  taille: number;
  texte: boolean;
}

export interface Fiche {
  id: number;
  titre: string;
  titre_en?: string;
  annee?: number | null;
  statut?: string;
  resume?: string;
  genres?: string[];
  plateforme?: string;
  developpeur?: string | null;
  editeur?: string | null;
  joueurs?: string | null;
  cooperatif?: boolean;
  launchbox_id?: number;
  versions?: Version[];
  annexes?: Annexe[];
}

export interface BilanSynchro {
  jeux: number;
  plateformes: number;
  pages: number;
  simule: boolean;
}

/** Une erreur renvoyée par le cœur : une sorte, et un motif déjà rédigé pour une personne non experte. */
export interface ErreurCoeur {
  sorte:
    | 'jeton_refuse'
    | 'introuvable'
    | 'conflit'
    | 'refus'
    | 'serveur'
    | 'reseau'
    | 'coffre'
    | 'pin'
    | 'profil'
    | 'reglage'
    | 'disque';
  motif: string;
}

export function estErreurCoeur(e: unknown): e is ErreurCoeur {
  return typeof e === 'object' && e !== null && 'sorte' in e && 'motif' in e;
}

function appeler<T>(commande: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) {
    return Promise.reject({ sorte: 'serveur', motif: 'Cette fonction n’existe que dans l’application Frogtend.' });
  }
  return invoke<T>(commande, args);
}

export const api = {
  profils: () => appeler<Profil[]>('profils_lister'),
  creerProfil: (nom: string, pin: string | null, jeton: string | null) =>
    appeler<Profil>('profil_creer', { nom, pin, jeton }),
  ouvrirProfil: (id: string, pin: string | null) => appeler<Profil>('profil_ouvrir', { id, pin }),
  fermerProfil: () => appeler<void>('profil_fermer'),
  profilActif: () => appeler<Profil | null>('profil_actif'),
  changerJeton: (jeton: string) => appeler<void>('profil_changer_jeton', { jeton }),
  reconnecter: () => appeler<void>('profil_reconnecter'),
  changerPin: (ancien: string | null, nouveau: string | null) => appeler<void>('profil_changer_pin', { ancien, nouveau }),
  renommerProfil: (nom: string) => appeler<Profil>('profil_renommer', { nom }),
  supprimerProfil: (id: string, pin: string | null) => appeler<void>('profil_supprimer', { id, pin }),

  synchroniser: () => appeler<BilanSynchro>('ludotheque_synchroniser'),
  synchroniseeLe: () => appeler<string | null>('ludotheque_synchronisee_le'),
  plateformes: () => appeler<Plateforme[]>('ludotheque_plateformes'),
  genres: (plateforme: string | null) => appeler<string[]>('ludotheque_genres', { plateforme }),
  lister: (filtre: Filtre) => appeler<Liste>('ludotheque_lister', { filtre }),
  auHasard: (plateforme: string | null) => appeler<JeuResume | null>('ludotheque_au_hasard', { plateforme }),
  fiche: (id: number) => appeler<{ fiche: Fiche; hors_ligne: boolean }>('ludotheque_fiche', { id }),
  annexeTexte: (id: number, i: number, cle: string) =>
    appeler<{ ok?: boolean; titre?: string; texte?: string }>('ludotheque_annexe_texte', { id, i, cle }),

  skins: () => appeler<unknown | null>('skins_obtenir'),
  skinPersonnel: () => appeler<{ theme?: string; remplace?: boolean } | null>('skin_personnel'),
};

/** L'adresse d'une jaquette du profil ouvert (servie par le cœur, jamais par Internet directement). */
export function adresseJaquette(id: number): string {
  // Sous Windows, les protocoles propres à l'application s'écrivent http://<nom>.localhost/.
  return `http://jaquette.localhost/${id}`;
}

/** « 237 887 038 » octets → « 226,9 Mo ». */
export function taille(octets: number): string {
  const unites = ['o', 'Ko', 'Mo', 'Go', 'To'];
  let v = octets;
  let u = 0;
  while (v >= 1024 && u < unites.length - 1) {
    v /= 1024;
    u++;
  }
  return `${v.toLocaleString('fr-FR', { maximumFractionDigits: u === 0 ? 0 : 1 })} ${unites[u]}`;
}
