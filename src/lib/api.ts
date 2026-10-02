// Les appels au cœur de Frogtend (Rust). Le jeton ne passe jamais par ici : seul le cœur le connaît.

import { invoke, isTauri } from '@tauri-apps/api/core';
import type { ActionProposee } from '$lib/assistant/actions';
import type { CommandesJeu } from '$lib/reglages/reglages';

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
  /** Change quand la jaquette change. */
  jaquette_empreinte?: string | null;
  /** Nombre de versions rangées dans Firehouse. */
  versions?: number | null;
}

export interface Liste {
  jeux: JeuResume[];
  total: number;
  total_ludotheque: number;
}

/** Où en est un jeu du PC. */
export type EtatJeuPc = 'attente' | 'en_cours' | 'pause' | 'telecharge' | 'erreur';

export interface JeuPc {
  id: number;
  version: number;
  titre: string;
  plateforme: string;
  dossier: string;
  etat: EtatJeuPc;
  total: number;
  fichiers: { n: number; nom: string; taille: number }[];
  message: string | null;
  ajoute_le: string;
  ajoute_par: string;
  /** `null` tant que le jeu n'est pas installé. */
  installation: Installation | null;
  /** Temps de jeu cumulé, en secondes. */
  temps_jeu: number;
  derniere_partie: string | null;
  /** Octets déjà sur le disque. */
  recus: number;
}

export interface Lanceur {
  programme: string;
  arguments: string[];
  dossier: string;
}

export type Methode =
  | { sorte: 'installeur'; fichier: string; format: string }
  | { sorte: 'installeur_guide'; fichier: string }
  | { sorte: 'archive'; fichier: string; format: string }
  | { sorte: 'aucune' };

export type NatureVersion = 'pret_a_jouer' | 'rom' | 'image_disque' | 'repack' | 'installeur_origine' | 'non_dite';

export interface Installation {
  dossier: string;
  methode: Methode;
  lanceur: Lanceur | null;
  fichier_du_jeu: string | null;
  installe_le: string;
}

export interface Preparation {
  nature: NatureVersion;
  methode: Methode;
  destination: string;
  notes: string | null;
  titre: string;
}

export interface Candidat {
  lanceur: Lanceur;
  relatif: string;
  note: number;
}

export interface Abri {
  dossier: string;
  fichiers: number;
  octets: number;
}

export interface EmulateurInstalle {
  id: string;
  nom: string;
  version: string | null;
  dossier: string;
  programme: string;
  par_frogtend: boolean;
  installe_le: string;
}

export interface Paquet {
  version: string;
  url: string;
  taille: number | null;
}

export interface EtatRetroArch {
  coeur: string | null;
  coeur_present: boolean;
  dossier_bios: string;
  bios_manquants: string[];
}

export interface EmulateurRecommande {
  nom: string;
  site: string;
  recommande: boolean;
  ligne_de_commande: string;
  extensions: string[];
  bios: string;
  // Contrat 14 (Firehouse, 02/10) : absents tant que Firehouse ne les sert pas.
  id?: string;
  site_officiel?: string;
  telechargement?: { type: 'github' | 'forgejo' | 'gitlab' | 'direct' | 'page'; url?: string | null; format?: string };
  programme?: string;
  forks?: (EmulateurRecommande & { actif?: boolean; note?: string })[];
}

/** Ce que devient l'installation d'un émulateur décrit par Firehouse. */
export type InstallationFirehouse =
  | { sorte: 'installe'; emulateur: EmulateurInstalle }
  | { sorte: 'a_choisir'; dossier: string; version: string; candidats: string[] };

/** Les triches et mods d'un jeu (contrat 13). */
export interface TrichesJeu {
  codes: { cle: string; emulateur: string; format: string; titre: string; nb_codes?: number; source?: string; correspondance?: string; confiance?: number; nom_fichier?: string }[];
  cheat_engine: { titre: string; version_jeu?: string; source?: string; page?: string; fichier?: string | null }[];
  mods: { nom: string; description?: string; auteur?: string; version?: string; source?: string; page?: string; maj_le?: string }[];
  maj_le: string | null;
  /** Ce que Firehouse ne sait pas encore fournir (par exemple les tables Cheat Engine). */
  note?: string | null;
  /** La page des mods du jeu (PCGamingWiki…). */
  page_mods?: string | null;
}

export interface EmplacementPropose {
  chemin: string;
  /** `null` : dossier introuvable. */
  libre: number | null;
  assez: boolean;
}

/** Ce que la file de téléchargements envoie à l'interface. */
export type EvenementTelechargement =
  | { sorte: 'progres'; jeu: number; recus: number; total: number; debit: number; fichier: string }
  | { sorte: 'etat'; jeu: number; etat: EtatJeuPc; message: string | null };

export interface Filtre {
  plateforme?: string | null;
  texte?: string | null;
  genre?: string | null;
  tri?: 'titre' | 'annee' | 'annee_desc';
  /** Vrai : seulement les jeux du PC (« Ma ludothèque ») ; faux : tout le catalogue Firehouse. */
  ludotheque?: boolean;
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
  /** `complete` (tout relu) ou `increment` (seulement ce qui a changé). */
  mode: 'complete' | 'increment';
  recus: number;
  retires: number;
}

/** Qui porte un jeton, d'après Firehouse (`/moi`). */
export interface Compte {
  ok?: boolean;
  username?: string;
  nom?: string;
  grade?: string;
  via?: string;
  api?: { version?: string; firehouse?: string };
}

/** « Seb (admin) ». */
export const libelleCompte = (c: Compte | null | undefined) =>
  c ? `${c.nom || c.username || '?'}${c.grade ? ` (${c.grade})` : ''}` : '';

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
    | 'version'
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

/** Un jeu de la base LaunchBox de Firehouse (`/recherche`, relevé le 30/09). */
export interface ResultatRecherche {
  launchbox_id: number;
  titre: string;
  titre_fr: string;
  plateforme: string;
  annee: number | null;
  genres: string[];
  developpeur: string;
  jaquette: string;
  /** Déjà connu de Firehouse : possédé, ou en cours de recherche. */
  deja: { id: number; statut: string } | null;
}

/** Un jeu possédé dans une boutique (lot 9). */
export interface JeuBoutique {
  id: string;
  nom: string;
  minutes: number;
  derniere: number;
  installe: boolean;
}

/** Le compte Steam du profil (jamais la clé : seulement si elle est enregistrée). */
export interface EtatSteam {
  compte: string | null;
  cle_enregistree: boolean;
  nb_jeux: number;
  maj_le: string | null;
  steam_installe: boolean;
}

/** Ce que montre le menu en jeu. */
export interface EtatMenuJeu {
  jeu: number;
  titre: string;
  plateforme: string;
  emulateur: string | null;
  /** `reset`, `disque`, `sauver`, `charger` : ce que l'émulateur sait faire depuis le menu. */
  actions: string[];
  /** Le jeu se met-il en pause quand le menu s'ouvre ? */
  en_pause: boolean;
}

/** Un réglage de manette de référence (livré avec Frogtend, ou repris sur ce PC). */
export interface ReferenceManette {
  emulateur: string;
  /** `Wiimote`, `GCPad` (Dolphin) ou `Pad` (DuckStation, PCSX2). */
  genre: string;
  nom: string;
  livree: boolean;
}

export const api = {
  profils: () => appeler<Profil[]>('profils_lister'),
  creerProfil: (nom: string, pin: string | null, jeton: string | null) =>
    appeler<Profil & { compte: Compte | null }>('profil_creer', { nom, pin, jeton }),
  compte: () => appeler<Compte>('profil_compte'),
  ouvrirProfil: (id: string, pin: string | null) => appeler<Profil>('profil_ouvrir', { id, pin }),
  fermerProfil: () => appeler<void>('profil_fermer'),
  profilActif: () => appeler<Profil | null>('profil_actif'),
  changerJeton: (jeton: string) => appeler<Compte | null>('profil_changer_jeton', { jeton }),
  reconnecter: () => appeler<void>('profil_reconnecter'),
  changerPin: (ancien: string | null, nouveau: string | null) => appeler<void>('profil_changer_pin', { ancien, nouveau }),
  renommerProfil: (nom: string) => appeler<Profil>('profil_renommer', { nom }),
  supprimerProfil: (id: string, pin: string | null) => appeler<void>('profil_supprimer', { id, pin }),

  synchroniser: () => appeler<BilanSynchro>('ludotheque_synchroniser'),
  synchroniseeLe: () => appeler<string | null>('ludotheque_synchronisee_le'),
  plateformes: (locale: boolean) => appeler<Plateforme[]>('ludotheque_plateformes', { locale }),
  genres: (plateforme: string | null, locale: boolean) => appeler<string[]>('ludotheque_genres', { plateforme, locale }),
  lister: (filtre: Filtre) => appeler<Liste>('ludotheque_lister', { filtre }),
  auHasard: (plateforme: string | null, locale: boolean) =>
    appeler<JeuResume | null>('ludotheque_au_hasard', { plateforme, locale }),
  fiche: (id: number) => appeler<{ fiche: Fiche; hors_ligne: boolean; locale: boolean }>('ludotheque_fiche', { id }),
  annexeTexte: (id: number, i: number, cle: string) =>
    appeler<{ ok?: boolean; titre?: string; texte?: string }>('ludotheque_annexe_texte', { id, i, cle }),

  skins: () => appeler<unknown | null>('skins_obtenir'),
  skinPersonnel: () => appeler<{ theme?: string; remplace?: boolean } | null>('skin_personnel'),
  enregistrerSkin: (nom: string) => appeler<void>('skin_enregistrer', { nom }),

  jeuxDuPc: () => appeler<JeuPc[]>('jeux_du_pc'),
  proposerEmplacements: (plateforme: string, taille: number) =>
    appeler<EmplacementPropose[]>('emplacements_proposer', { plateforme, taille }),
  ajouterJeu: (id: number, version: number, emplacement: string) =>
    appeler<Omit<JeuPc, 'recus'>>('jeu_ajouter', { id, version, emplacement }),
  pause: (id: number) => appeler<void>('telechargement_pause', { id }),
  reprendre: (id: number) => appeler<void>('telechargement_reprendre', { id }),
  annuler: (id: number) => appeler<void>('telechargement_annuler', { id }),
  ouvrirAnnexe: (id: number, i: number) => appeler<void>('annexe_ouvrir', { id, i }),
  sauvegarder: () => appeler<import('./sauvegarde.svelte').BilanSauvegarde>('sauvegarde_lancer'),
  derniereSauvegarde: () => appeler<import('./sauvegarde.svelte').BilanSauvegarde | null>('sauvegarde_derniere'),
  restaurationListe: () =>
    appeler<{ profil: string; pc: string; derniere: { date?: string; fichiers?: number; taille?: number } | null; octets: number }[]>(
      'restauration_liste',
    ),
  restaurationPreparer: (profil: string, pc: string) =>
    appeler<[{ configuration: Record<string, any>; bibliotheque: { jeux?: any[] }; fichiers: number; taille: number }, { reposes: number; deja_la: number; conflits: string[] }]>(
      'restauration_preparer',
      { profil, pc },
    ),
  espaceLibre: (chemin: string) => appeler<number | null>('espace_libre', { chemin }),

  preparerInstallation: (id: number) => appeler<Preparation>('installation_preparer', { id }),
  installer: (id: number, automatique: boolean) => appeler<Installation>('installation_lancer', { id, automatique }),
  installeAilleurs: (id: number, dossier: string) => appeler<Installation>('installation_ailleurs', { id, dossier }),
  candidatsLancement: (id: number) => appeler<Candidat[]>('lancement_candidats', { id }),
  choisirLanceur: (id: number, lanceur: Lanceur) => appeler<void>('lanceur_choisir', { id, lanceur }),
  rpcs3InstallerMicrologiciel: (programme: string, pup: string) => appeler<void>('rpcs3_installer_micrologiciel', { programme, pup }),
  taodboxLance: () => appeler<boolean>('taodbox_lance'),
  assistant: (question: string, mediaId: number | null, historique: { role: string; content: string }[]) =>
    appeler<{ ok?: boolean; texte?: string; actions_proposees?: ActionProposee[] }>('assistant_demander', { question, mediaId, historique }),
  assistantJournal: (action: string, decision: string) => appeler<void>('assistant_journal', { action, decision }),
  rechercherJeu: (texte: string) => appeler<{ ok?: boolean; resultats?: ResultatRecherche[] }>('jeu_rechercher', { texte }),
  demanderJeu: (launchboxId: number) => appeler<void>('jeu_demander', { launchboxId }),
  menuJeuEtat: () => appeler<EtatMenuJeu | null>('menu_jeu_etat'),
  menuJeuReprendre: () => appeler<void>('menu_jeu_reprendre'),
  menuJeuAction: (action: string) => appeler<void>('menu_jeu_action', { action }),
  menuJeuQuitter: () => appeler<number>('menu_jeu_quitter'),
  jouer: (id: number, commandes?: CommandesJeu, emulateur?: string) =>
    appeler<void>('jeu_jouer', { id, commandes: commandes ?? null, emulateur: emulateur ?? null }),
  referencesManette: (id: string) => appeler<ReferenceManette[]>('references_manette', { id }),
  profilsManetteEmulateur: (id: string, programme: string) =>
    appeler<{ genre: string; nom: string; chemin: string }[]>('profils_manette_emulateur', { id, programme }),
  referenceReprendre: (id: string, programme: string, genre: string, chemin: string, nom: string) =>
    appeler<ReferenceManette>('reference_reprendre', { id, programme, genre, chemin, nom }),
  mettreALAbri: (id: number) => appeler<Abri>('parties_abri', { id }),
  retirer: (id: number) => appeler<Abri>('jeu_retirer', { id }),
  emulateursInstalles: () => appeler<EmulateurInstalle[]>('emulateurs_installes'),
  emulateurFiche: (nom: string) =>
    appeler<{ id: string; nom: string; ligne: string; installable: boolean } | null>('emulateur_fiche', { nom }),
  emulateurDerniereVersion: (id: string) => appeler<Paquet>('emulateur_derniere_version', { id }),
  emulateurInstaller: (id: string, dossier: string) => appeler<EmulateurInstalle>('emulateur_installer', { id, dossier }),
  emulateurAdopter: (id: string, programme: string, nom?: string, version?: string) =>
    appeler<EmulateurInstalle>('emulateur_adopter', { id, programme, nom: nom ?? null, version: version ?? null }),
  emulateurInstallerFirehouse: (id: string, nom: string, dossier: string) =>
    appeler<InstallationFirehouse>('emulateur_installer_firehouse', { id, nom, dossier }),
  steamEtat: () => appeler<EtatSteam>('boutique_steam_etat'),
  steamRegler: (compte: string, cle?: string) => appeler<EtatSteam>('boutique_steam_regler', { compte, cle: cle ?? null }),
  steamOublier: () => appeler<void>('boutique_steam_oublier'),
  steamJeux: () => appeler<JeuBoutique[]>('boutique_steam_jeux'),
  steamImporter: () => appeler<JeuBoutique[]>('boutique_steam_importer'),
  steamOuvrir: (appid: string, action: 'jouer' | 'installer') => appeler<void>('boutique_steam_ouvrir', { appid, action }),
  cheatengineLancer: (programme: string, table?: string, brancher = false) =>
    appeler<void>('cheatengine_lancer', { programme, table: table ?? null, brancher }),
  jeuTriches: (id: number) => appeler<TrichesJeu>('jeu_triches', { id }),
  tricheInstaller: (id: number, cle: string, programme: string, ligne: string) =>
    appeler<string>('triche_installer', { id, cle, programme, ligne }),
  jeuTailleInstallation: (id: number) => appeler<number>('jeu_taille_installation', { id }),
  jeuCopieAvantMod: (id: number) => appeler<string>('jeu_copie_avant_mod', { id }),
  retroarchEtat: (programme: string, ligne: string, bios: string[]) =>
    appeler<EtatRetroArch>('retroarch_etat', { programme, ligne, bios }),
  emulateurReglerManette: (id: string, programme: string) =>
    appeler<{ reglee: boolean; profils_ajoutes: number }>('emulateur_regler_manette', { id, programme }),
  retroarchInstallerCoeur: (programme: string, coeur: string) =>
    appeler<string>('retroarch_installer_coeur', { programme, coeur }),
  emulateursTraces: () => appeler<{ id: string; nom: string; dossiers: string[] }[]>('emulateurs_traces'),
  emulateursRecommandes: (plateforme: string) =>
    appeler<{ emulateurs?: EmulateurRecommande[] }>('emulateurs_recommandes', { plateforme }),
};

/**
 * L'adresse d'une jaquette du profil ouvert (servie par le cœur, jamais par Internet directement).
 * `largeur` (en pixels affichés) : une miniature suffit, arrondie à la centaine supérieure (100 à 1000).
 */
export function adresseJaquette(id: number, largeur?: number): string {
  // Sous Windows, les protocoles propres à l'application s'écrivent http://<nom>.localhost/.
  if (!largeur) return `http://jaquette.localhost/${id}`;
  const l = Math.min(1000, Math.max(100, Math.ceil(largeur / 100) * 100));
  return `http://jaquette.localhost/${id}?largeur=${l}`;
}

/** La jaquette officielle d'un jeu de boutique (servie et gardée en cache par le cœur). */
export const adresseImageBoutique = (boutique: string, id: string) => `http://boutique.localhost/${boutique}/${encodeURIComponent(id)}`;

/** L'adresse de la vidéo de fond d'un skin (servie par le cœur, gardée pour ce PC). */
export const adresseFond = (skin: string) => `http://fond.localhost/${encodeURIComponent(skin)}`;

/** Une durée lisible : « 3 min », « 1 h 20 ». */
export function duree(secondes: number): string {
  if (!Number.isFinite(secondes) || secondes < 0) return '—';
  if (secondes < 60) return 'moins d’une minute';
  const min = Math.round(secondes / 60);
  if (min < 60) return `${min} min`;
  const h = Math.floor(min / 60);
  return `${h} h ${String(min % 60).padStart(2, '0')}`;
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
