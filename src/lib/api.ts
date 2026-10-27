// Les appels au cœur de Frogtend (Rust). Le jeton ne passe jamais par ici : seul le cœur le connaît.

import { invoke, isTauri } from '@tauri-apps/api/core';
import type { ActionProposee } from '$lib/assistant/actions';
import type { CommandesJeu } from '$lib/reglages/reglages';
import type { InfosJeu } from '$lib/ludotheque/infos';
import type { ModJeu } from '$lib/mods/mods';

export interface Profil {
  id: string;
  nom: string;
  protege: boolean;
  a_un_jeton?: boolean;
}

/** Une image d'un système (contrat 1.28) et ses CRÉDITS (`licence` null : rien à créditer, LaunchBox). */
export interface ImageSysteme {
  id: number;
  empreinte: string;
  largeur?: number;
  hauteur?: number;
  source?: string;
  licence?: string | null;
  auteur?: string | null;
  url_source?: string | null;
}

/** La fiche d'un système (contrat 1.28), telle que Firehouse l'a donnée au profil. */
export interface FicheSysteme {
  nom: string;
  nom_affiche?: string;
  fabricant?: string | null;
  date_sortie?: string | null;
  description_fr?: string | null;
  description_source?: string | null;
  description_licence?: string | null;
  images?: Partial<Record<'logo' | 'banniere' | 'fond' | 'appareil' | 'icone', ImageSysteme | null>>;
}

/** L'adresse d'une image d'un système (servie par le cœur pour le profil ouvert) ; l'empreinte change l'adresse quand
 * l'image change. */
export function adresseImageSysteme(nom: string, sorte: string, largeur: number, empreinte: string): string {
  const l = Math.min(1000, Math.max(100, Math.ceil(largeur / 100) * 100));
  return `http://systeme.localhost/${sorte}?plateforme=${encodeURIComponent(nom)}&largeur=${l}&e=${encodeURIComponent(empreinte.slice(0, 16))}`;
}

export interface Plateforme {
  /** Le nom LaunchBox : la CLÉ partout (filtres, réglages par système, chemins). */
  nom: string;
  jeux: number;
  /** Le nom court choisi par Seb (contrat 1.27 : « Mega Drive ») — AFFICHAGE seulement. */
  nom_affiche?: string;
  /** L'empreinte de l'icône monochrome du système (contrat 1.28), s'il en a une. */
  icone?: string;
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
  /** Les visuels du jeu par type, avec leurs dimensions d'origine (contrat 1.6) ; absent = pas encore relu. */
  images?: Record<string, { largeur?: number | null; hauteur?: number | null }> | null;
  // Jeux des boutiques de la personne (lot 9 : id négatif, plateforme « Windows »).
  source?: 'steam' | 'galaxy' | 'local';
  /** Succès (obtenus, total) d'un jeu de boutique, quand la source les donne (GOG Galaxy). */
  succes?: [number, number] | null;
  boutique?: string;
  cle_boutique?: string;
  installe?: boolean;
  image?: string | null;
  minutes?: number;
  /** A6c : un jeu possédé sur plusieurs boutiques, réuni en une carte (la maîtresse d'abord). */
  copies?: { id: number; boutique: string; cle: string; installe: boolean; minutes: number }[] | null;
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
  /** `version` : la version (carte e-Reader) d'où vient ce fichier, si ce n'est pas celle de la ligne. */
  fichiers: { n: number; nom: string; taille: number; version?: number; sha1?: string }[];
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
  /** La fiche de Firehouse que montre ce jeu pour ce profil (un jeu importé relié : contrat 1.21). */
  fiche?: number | null;
  /** Importé par une autre personne de ce PC : on peut seulement y jouer (✅ Seb 20/10). */
  d_un_autre?: boolean;
  /** Le titre de la fiche reliée (à montrer à la place de celui du fichier). */
  titre_fiche?: string | null;
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
  /** Les versions d'un jeu importé (fr, eu, us/en, autres), absentes s'il n'y en a qu'une. */
  versions?: import('$lib/ludotheque/versions').VersionLocale[];
  /** Un jeu d'une boutique (Steam…), lancé par elle ; Frogtend n'écrit jamais dans son dossier. */
  boutique?: { source: string; cle: string } | null;
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
  /** Un jeu installé AILLEURS qui reste installé : son dossier (son désinstalleur, seulement après un oui). */
  reste_installe?: string | null;
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

/** Un patch d'un jeu PS3 (contrat 1.18), et s'il est activé dans RPCS3. */
export interface PatchPs3 {
  hash: string;
  description: string;
  titre: string;
  auteur: string;
  notes: string;
  /** Contrat 1.20 : la traduction de Firehouse (`null` : pas encore traduite ; `''` : texte d'origine vide). */
  description_fr?: string | null;
  notes_fr?: string | null;
  actif: boolean;
}

/** Les patchs d'un jeu PS3 et la liste des patchs de RPCS3 (présente, à jour, sa taille). */
export interface PatchsPs3 {
  numero: string | null;
  version: string | null;
  patchs: PatchPs3[];
  liste_presente: boolean;
  liste_a_jour: boolean;
  taille_liste: number;
  /** Firehouse injoignable : seules les retouches actives (pour les désactiver). */
  hors_ligne: boolean;
}

/** Un mod installé dans un jeu (son manifeste). */
export interface ModInstalle {
  cle: string;
  mod_nom: string;
  fichier: string;
  pose_le: string;
  etat: string;
  fichiers: { chemin: string; sorte: string }[];
  /** Un jeu Steam : Steam a mis le jeu à jour depuis la pose (le mod a pu être défait). */
  steam_mis_a_jour?: boolean;
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

/** Un BIOS ou firmware du conseil (contrat 1.8–1.11, relevés réels du 16/10). Seul `conseil.bios` fait foi (jamais la
 * chaîne `bios` d'une ligne LaunchBox). */
export interface BiosConseil {
  fichier: string;
  type: 'fichier' | 'dossier' | 'motif' | 'installation' | 'archive';
  racine: 'system_retroarch' | 'dossier_emulateur' | 'installation_par_l_emulateur' | string;
  chemin?: string;
  facultatif?: boolean;
  /** Une clé (prod.keys, keys.txt) : jamais servie, Frogtend dit qu'elle manque. */
  coffre?: boolean;
  note?: string;
  disponible?: boolean;
  sha1?: string;
  taille?: number;
  url?: string;
  /** Type `motif` : les fichiers servis (`null` est lu comme une liste vide). */
  fichiers?: { nom: string; sha1: string; taille: number; url: string }[] | null;
}

/** Un cœur RetroArch servi par Firehouse (contrat 1.12). */
export interface PaquetCoeur {
  url: string;
  sha256: string;
  taille: number;
  version: string;
  source: string;
}

/** Le conseil de la maison pour un système (contrat 1.8) : il PRIME sur la liste LaunchBox ; `null` = aucun conseil
 * (jamais deviné). */
export interface Conseil {
  plateforme: string;
  /** L'id de l'émulateur dans les sources de Firehouse (« retroarch », « pcsx2 », « dosbox_staging »…). */
  emulateur: string;
  coeur: string | null;
  /** SA ligne de commande (« -L "cores\snes9x_libretro.dll" "%rom%" ») ; depuis le contrat 1.16, COMPLÈTE aussi pour un
   * émulateur autonome (« -b -e "%rom%" »), avec `%rom%`, `%rom_nom%`, `%rom_dossier%` ; vide = non fournie. */
  ligne_de_commande: string;
  alternatives: { nom: string; emulateur: string | null; coeur: string | null; quand: string; site: string; paquet?: PaquetCoeur | null }[];
  bios: BiosConseil[];
  paquet?: PaquetCoeur | null;
  version_attendue: string | null;
  note: string;
  origine: string;
  maj_le: string;
}

/** Ce que Frogtend a fait des fichiers système (BIOS) d'une console (`bios_poser`). */
export interface BilanBios {
  poses: string[];
  deja: string[];
  a_poser: string[];
  cles: string[];
  firmwares: string[];
  echecs: string[];
}

/** Ce que devient l'installation d'un émulateur décrit par Firehouse. */
export type InstallationFirehouse =
  | { sorte: 'installe'; emulateur: EmulateurInstalle }
  | { sorte: 'a_choisir'; dossier: string; version: string; candidats: string[] };

/** Les triches et mods d'un jeu (contrat 13). */
export interface TrichesJeu {
  codes: { cle: string; emulateur: string; format: string; titre: string; nb_codes?: number; source?: string; correspondance?: string; confiance?: number; nom_fichier?: string }[];
  cheat_engine: { titre: string; version_jeu?: string; source?: string; page?: string; fichier?: string | null }[];
  /** Contrat 1.19 : fichiers, refus, liens cités (`src/lib/mods/mods.ts`). */
  mods: (ModJeu & { auteur?: string; version?: string; source?: string; maj_le?: string })[];
  maj_le: string | null;
  /** Ce que Firehouse ne sait pas encore fournir (par exemple les tables Cheat Engine). */
  note?: string | null;
  /** La page des mods du jeu (PCGamingWiki…). */
  page_mods?: string | null;
  /** Jeu en ligne et anti-triche d'après PCGamingWiki (relevé réel du 21/10, pas encore au contrat) : `null` = inconnu. */
  en_ligne?: { online: string | null; anticheat: string | null; source?: string; lu_le?: string } | null;
}

/** La fiche d'un jeu de boutique (lot D : boutique Steam en français, sinon GOG Galaxy). */
export interface FicheBoutique {
  source: 'steam' | 'galaxy';
  resume: string | null;
  en_anglais: boolean;
  genres: string[];
  developpeurs: string[];
  editeurs: string[];
  annee: number | null;
  sortie: string | null;
  fond: string | null;
  captures: [string, string][];
}

/** Ce que Frogtend mettrait à l'abri des parties d'un jeu Steam (`nuage_steam_apercu`). */
export interface ApercuNuage {
  fichiers: number;
  octets: number;
  introuvables: number;
  en_ligne_seulement: number;
  en_attente: number;
  modifies: number;
  hors_nuage: number;
  emplacements_illisibles: number;
  deja: boolean;
  gros: boolean;
}

/** Le risque de brancher Cheat Engine sur un jeu (`cheatengine_risque`). */
export interface RisqueTriche {
  anti_triche: string | null;
  en_ligne: string | null;
  inconnu: boolean;
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
  /** Une boutique (`steam`, `gog`, `epic`…, ou `firehouse`). */
  boutique?: string | null;
  /** Installé sur ce PC (vrai) ou pas (faux). */
  installe?: boolean | null;
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

/** Un document d'un jeu (contrat 1.7) : livret (PDF), musique (MP3), vidéos (MP4). */
export interface DocumentJeu {
  id: number;
  type: 'livret' | 'musique' | 'video' | 'video_theme';
  langue: string;
  format: string;
  taille: number;
  pages: number | null;
  duree_s: number | null;
  /** Déjà sur le PC : rien à télécharger au clic. */
  garde?: boolean;
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
  /** Contrat 1.27 : le nom court de la plateforme — affichage seulement (`plateforme` reste la clé). */
  plateforme_affichee?: string;
  developpeur?: string | null;
  editeur?: string | null;
  joueurs?: string | null;
  cooperatif?: boolean;
  launchbox_id?: number;
  versions?: Version[];
  annexes?: Annexe[];
  /** Contrat 1.15 : date de sortie, type, classement d'âge, note… (au détail seulement). */
  infos?: InfosJeu | null;
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
  /** Jeux du PC rattachés à leur nouvelle fiche pendant cette synchronisation (contrat 1.14) : [ancien, nouveau]. */
  rattaches?: [number, number][];
  /** La lecture s'est arrêtée au plafond de pages (catalogue peut-être incomplet : aucun ménage). */
  tronque?: boolean;
  /** Le ménage des caches d'images (lot Médias M4). */
  menage?: { elements: number; octets: number };
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
    | 'plage'
    | 'serveur'
    | 'reseau'
    | 'coffre'
    | 'pin'
    | 'profil'
    | 'reglage'
    | 'version'
    | 'disque'
    | 'fusionne'
    /** Une opération d'une session remplacée (profil ou jeton changé pendant qu'elle tournait) : rien à montrer. */
    | 'perimee'
    /** Refusé avant d'agir : le motif s'explique dans une fenêtre (image Xbox que xemu ne lit pas…). */
    | 'prevenir'
    /** Une carte e-Reader dont la cartouche (`cartouche`) n'est pas sur ce PC : proposer de l'y mettre. */
    | 'cartouche_absente';
  motif: string;
  /** Pour `fusionne` (contrat 1.14) : la fiche où celle-ci a été réunie. */
  nouveau?: number;
  /** Pour `cartouche_absente` : la fiche de la cartouche e-Reader qui lit la carte. */
  cartouche?: number;
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
  /** La boutique d'origine pour les jeux venus de GOG Galaxy (« gog », « epic »…). */
  plateforme?: string;
  /** La jaquette (adresse publique), quand la source la donne. */
  image?: string | null;
  /** Le dossier d'installation sur ce PC (le jeu est alors un jeu du PC : lancé et suivi par Frogtend). */
  dossier?: string | null;
}

/** Le bilan d'un passage sur GOG ou Prime Gaming. */
export interface Recolte {
  etat: 'faite' | 'connexion' | 'aucun' | 'pas_abonne' | 'erreur';
  obtenus: string[];
  deja: number;
  a_finir: number;
  motif?: string | null;
}

/** Le bilan d'un passage sur les jeux PS Plus du mois. */
export type RecoltePsPlus =
  | { etat: 'faite'; ajoutes: number; deja: number; vus: number }
  | { etat: 'connexion' }
  | { etat: 'erreur'; motif: string };

/** Un jeu offert en ce moment (lot 9). */
export interface JeuOffert {
  boutique: string;
  titre: string;
  slug: string;
  fin: string;
  image: string | null;
  obtenu: boolean;
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
  /** Ouvert en Taodbox (télé) : le menu est à l'échelle ×2. */
  taodbox: boolean;
}

/** Un réglage de manette de référence (livré avec Frogtend, ou repris sur ce PC). */
export interface ReferenceManette {
  emulateur: string;
  /** `Wiimote`, `GCPad` (Dolphin) ou `Pad` (DuckStation, PCSX2). */
  genre: string;
  nom: string;
  livree: boolean;
}

/** Peut-on figer un jeu pendant le menu ? (`aucun` : oui ; un anti-triche, ou `inconnu` — trop de fichiers pour le savoir — : non.) */
export type VerificationFiger = { sorte: 'aucun' } | { sorte: 'anti_triche'; nom: string } | { sorte: 'inconnu' };

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
  /** Les jeux du PC rattachés à une fiche réunie (contrat 1.14) : [ancien, nouveau]. */
  reunions: () => appeler<[number, number][]>('jeux_reunions'),
  synchroniseeLe: () => appeler<string | null>('ludotheque_synchronisee_le'),
  plateformes: (locale: boolean) => appeler<Plateforme[]>('ludotheque_plateformes', { locale }),
  /** La fiche d'un système (contrat 1.28) ; `null` : pas encore reçue. */
  plateformeFiche: (nom: string) => appeler<FicheSysteme | null>('plateforme_fiche', { nom }),
  genres: (plateforme: string | null, locale: boolean) => appeler<string[]>('ludotheque_genres', { plateforme, locale }),
  lister: (filtre: Filtre) => appeler<Liste>('ludotheque_lister', { filtre }),
  auHasard: (plateforme: string | null, locale: boolean) =>
    appeler<JeuResume | null>('ludotheque_au_hasard', { plateforme, locale }),
  /** `reunie_dans` : la fiche demandée a été réunie à celle-ci chez Firehouse (contrat 1.14). */
  fiche: (id: number) => appeler<{ fiche: Fiche; hors_ligne: boolean; locale: boolean; reunie_dans?: number }>('ludotheque_fiche', { id }),
  /** Les infos d'un jeu du PC relues en direct (contrat 1.15) ; `null` : caché, inconnu, ou rien de gardé hors ligne. */
  infosJeu: (id: number) => appeler<InfosJeu | null>('ludotheque_infos', { id }),
  annexeTexte: (id: number, i: number, cle: string) =>
    appeler<{ ok?: boolean; titre?: string; texte?: string }>('ludotheque_annexe_texte', { id, i, cle }),

  skins: () => appeler<unknown | null>('skins_obtenir'),
  skinPersonnel: () => appeler<{ theme?: string; remplace?: boolean } | null>('skin_personnel'),
  enregistrerSkin: (nom: string) => appeler<void>('skin_enregistrer', { nom }),

  jeuxDuPc: () => appeler<JeuPc[]>('jeux_du_pc'),
  proposerEmplacements: (plateforme: string, taille: number) =>
    appeler<EmplacementPropose[]>('emplacements_proposer', { plateforme, taille }),
  /** Retire UNE carte d'un jeu e-Reader (jamais la dernière, jamais les parties). */
  retirerCarte: (id: number, version: number) => appeler<JeuPc>('jeu_retirer_carte', { id, version }),
  ajouterJeu: (id: number, version: number, emplacement: string) =>
    appeler<Omit<JeuPc, 'recus'>>('jeu_ajouter', { id, version, emplacement }),
  pause: (id: number) => appeler<void>('telechargement_pause', { id }),
  reprendre: (id: number) => appeler<void>('telechargement_reprendre', { id }),
  annuler: (id: number) => appeler<void>('telechargement_annuler', { id }),
  ouvrirAnnexe: (id: number, i: number) => appeler<void>('annexe_ouvrir', { id, i }),
  documents: (id: number) => appeler<DocumentJeu[]>('documents_jeu', { id }),
  /** Tous les visuels d'un jeu (galerie de la fiche, contrat 1.6). */
  medias: (id: number) => appeler<import('./ludotheque/galerie').MediaJeu[]>('medias_jeu', { id }),
  /** Télécharge le livret s'il n'est pas sur le PC (le clic vaut accord, taille montrée), rend son nombre de pages. */
  ouvrirLivret: (id: number, doc: number) => appeler<number>('livret_ouvrir', { id, doc }),
  /** Télécharge une musique ou une vidéo si elle n'est pas sur le PC (le clic vaut accord, taille montrée). */
  preparerDocument: (id: number, doc: number) => appeler<DocumentJeu>('document_preparer', { id, doc }),
  /** Contrat 1.32 : identifier les jeux importés sans fiche. `annonce` : [à envoyer, déjà cherchés, lots]. */
  identificationAnnonce: () => appeler<[number, number, number]>('identification_annonce'),
  identificationLancer: (tout = false) =>
    appeler<{ envoyes: number; exacts: number; probables: number; inconnus: number; motif?: string | null }>('identification_lancer', { tout }),
  identificationAChoisir: () => appeler<import('$lib/import/identification').AChoisir[]>('identification_a_choisir'),
  identificationChoisir: (jeu: number, launchboxId: number) => appeler<void>('identification_choisir', { jeu, launchboxId }),
  identificationEcarter: (jeu: number) => appeler<void>('identification_ecarter', { jeu }),
  ficheBase: (jeu: number) => appeler<import('$lib/import/identification').FicheBase | null>('jeu_fiche_base', { jeu }),
  /** Ce qu'une sauvegarde enverrait : taille de toutes les parties, et celle de la dernière sauvegarde. */
  sauvegardeEstimer: () => appeler<{ total: number; precedent: number | null }>('sauvegarde_estimer'),
  /** `ecarter` : la personne a choisi de REMPLACER une sauvegarde existante qu'elle n'a pas restaurée. */
  sauvegarder: (ecarter = false) => appeler<import('./sauvegarde.svelte').BilanSauvegarde>('sauvegarde_lancer', { ecarter }),
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
  /** Le dossier des jeux proposé quand rien n'est réglé : `<disque fixe le plus libre>\Frogtend\Jeux` (rien n'est créé). */
  dossierJeuxPropose: () => appeler<{ chemin: string; disque: string; libre: number; disques: string[]; externe: boolean } | null>('dossier_jeux_propose'),
  /** Crée le dossier PROPOSÉ (seul celui-là est admis par le cœur), après le oui de la personne. */
  dossierJeuxCreer: (chemin: string) => appeler<string>('dossier_jeux_creer', { chemin }),
  /** Repose les parties d'émulateurs restaurées qui attendaient un émulateur absent (jamais par-dessus une autre). */
  restaurationReposerEmulateurs: () => appeler<{ reposes: number; deja_la: number; conflits: string[] }>('restauration_reposer_emulateurs'),

  preparerInstallation: (id: number) => appeler<Preparation>('installation_preparer', { id }),
  installer: (id: number, automatique: boolean) => appeler<Installation>('installation_lancer', { id, automatique }),
  installeAilleurs: (id: number, dossier: string) => appeler<Installation>('installation_ailleurs', { id, dossier }),
  candidatsLancement: (id: number) => appeler<Candidat[]>('lancement_candidats', { id }),
  choisirLanceur: (id: number, lanceur: Lanceur) => appeler<void>('lanceur_choisir', { id, lanceur }),
  rpcs3InstallerMicrologiciel: (programme: string, pup: string) => appeler<string>('rpcs3_installer_micrologiciel', { programme, pup }),
  rpcs3Micrologiciel: (programme: string) => appeler<string | null>('rpcs3_micrologiciel', { programme }),
  taodboxLance: () => appeler<boolean>('taodbox_lance'),
  /** Garde à l'avance fonds et logos des jeux du PC pour Taodbox hors ligne : le nombre de jeux prêts. */
  prechargerTaodbox: () => appeler<number>('taodbox_precharger'),
  assistant: (question: string, mediaId: number | null, historique: { role: string; content: string }[]) =>
    appeler<{ ok?: boolean; texte?: string; actions_proposees?: ActionProposee[] }>('assistant_demander', { question, mediaId, historique }),
  assistantJournal: (action: string, decision: string) => appeler<void>('assistant_journal', { action, decision }),
  rechercherJeu: (texte: string) => appeler<{ ok?: boolean; resultats?: ResultatRecherche[] }>('jeu_rechercher', { texte }),
  demanderJeu: (launchboxId: number) => appeler<void>('jeu_demander', { launchboxId }),
  menuJeuEtat: () => appeler<EtatMenuJeu | null>('menu_jeu_etat'),
  menuJeuReprendre: () => appeler<void>('menu_jeu_reprendre'),
  menuJeuAction: (action: string) => appeler<void>('menu_jeu_action', { action }),
  menuJeuQuitter: () => appeler<number>('menu_jeu_quitter'),
  jouer: (
    id: number,
    commandes?: CommandesJeu,
    emulateur?: string,
    version?: string,
    figer?: boolean,
    pleinEcran?: string,
    clavier?: Record<string, string>,
    repeterTouches = true,
    carte?: number,
  ) =>
    appeler<void>('jeu_jouer', {
      id,
      commandes: commandes ?? null,
      emulateur: emulateur ?? null,
      version: version ?? null,
      figer: figer ?? false,
      pleinEcran: pleinEcran ?? null,
      clavier: clavier && Object.keys(clavier).length ? clavier : null,
      repeterTouches,
      carte: carte ?? null,
    }),
  pleinEcranSuggestion: (id: number) => appeler<string | null>('jeu_plein_ecran_suggestion', { id }),
  jeuAntiTriche: (id: number) => appeler<VerificationFiger>('jeu_anti_triche', { id }),
  biosPoser: (plateforme: string, programme: string) => appeler<BilanBios>('bios_poser', { plateforme, programme }),
  firmwareServi: (plateforme: string) => appeler<BiosConseil | null>('firmware_servi', { plateforme }),
  rpcs3MicrologicielServi: (programme: string) => appeler<string>('rpcs3_micrologiciel_servi', { programme }),
  choisirVersion: (id: number, chemin: string) => appeler<void>('jeu_choisir_version', { id, chemin }),
  referencesManette: (id: string) => appeler<ReferenceManette[]>('references_manette', { id }),
  profilsManetteEmulateur: (id: string, programme: string) =>
    appeler<{ genre: string; nom: string; chemin: string }[]>('profils_manette_emulateur', { id, programme }),
  referenceReprendre: (id: string, programme: string, genre: string, chemin: string, nom: string) =>
    appeler<ReferenceManette>('reference_reprendre', { id, programme, genre, chemin, nom }),
  mettreALAbri: (id: number) => appeler<Abri>('parties_abri', { id }),
  retirer: (id: number) => appeler<Abri>('jeu_retirer', { id }),
  /** Image et fluidité d'un jeu PS3 dans RPCS3 : `null` = pas un jeu PS3 lancé par RPCS3 ; `numero` absent = disque
   * illisible (archive à décompresser…). */
  rpcs3Image: (id: number) =>
    appeler<{ numero: string | null; valeurs: [string, string][]; par_jeu: string[]; cache_coupe: boolean } | null>('rpcs3_image', { id }),
  /** Règle pour CE jeu les réglages donnés ; `vide` : revenir aux valeurs de RPCS3. */
  rpcs3ReglerImage: (id: number, valeurs: [string, string][], vide = false) =>
    appeler<void>('rpcs3_regler_image', { id, valeurs, vide }),
  /** Les patchs d'un jeu PS3 (contrat 1.18) ; `null` : pas un jeu PS3 lancé par RPCS3. */
  rpcs3Patchs: (id: number) => appeler<PatchsPs3 | null>('rpcs3_patchs', { id }),
  /** Active ou désactive un patch ; `liste` : la personne a accepté d'installer la liste des patchs de RPCS3. */
  rpcs3PatchRegler: (id: number, p: PatchPs3, actif: boolean, liste = false) =>
    appeler<void>('rpcs3_patch_regler', { id, hash: p.hash, description: p.description, titre: p.titre, actif, liste }),
  /** La fenêtre de choix de Windows, ouverte PAR LE CŒUR (durcissement) : il retient le chemin choisi, seul admis
   * ensuite par les commandes sensibles. `null` : fenêtre fermée. */
  parcourir: (options: { dossier?: boolean; titre: string; extensions?: string[] }) =>
    appeler<string | null>('parcourir', { dossier: options.dossier ?? false, titre: options.titre, extensions: options.extensions ?? null }),
  /** Le désinstalleur d'un jeu retiré qui restait installé ailleurs (proposé par le retrait, une fois). */
  desinstallerReste: (dossier: string) => appeler<number>('jeu_desinstaller_reste', { dossier }),
  emulateursInstalles: () => appeler<EmulateurInstalle[]>('emulateurs_installes'),
  emulateurFiche: (nom: string) =>
    appeler<{ id: string; nom: string; ligne: string; installable: boolean } | null>('emulateur_fiche', { nom }),
  emulateurDerniereVersion: (id: string) => appeler<Paquet>('emulateur_derniere_version', { id }),
  emulateurInstaller: (id: string, dossier: string) => appeler<EmulateurInstalle>('emulateur_installer', { id, dossier }),
  emulateurAdopter: (id: string, programme: string, nom?: string, version?: string) =>
    appeler<EmulateurInstalle>('emulateur_adopter', { id, programme, nom: nom ?? null, version: version ?? null }),
  emulateurInstallerFirehouse: (id: string, nom: string, dossier: string) =>
    appeler<InstallationFirehouse>('emulateur_installer_firehouse', { id, nom, dossier }),
  gratuitsListe: () => appeler<JeuOffert[]>('gratuits_liste'),
  gratuitsConnexionEpic: () => appeler<void>('gratuits_connexion_epic'),
  gratuitsConnexion: (boutique: 'gog' | 'prime') => appeler<void>('gratuits_connexion', { boutique }),
  gratuitsRecuperer: (boutique: 'gog' | 'prime') => appeler<Recolte>('gratuits_recuperer', { boutique }),
  gratuitsConnexionPlaystation: () => appeler<void>('gratuits_connexion_playstation'),
  gratuitsPsPlus: () => appeler<RecoltePsPlus>('gratuits_psplus'),
  gratuitsObtenirEpic: (slug: string) => appeler<{ etat: string; motif?: string }>('gratuits_obtenir_epic', { slug }),
  galaxyEtat: () => appeler<{ galaxy_installe: boolean; nb_jeux: number; maj_le: string | null }>('boutique_galaxy_etat'),
  galaxyJeux: () => appeler<JeuBoutique[]>('boutique_galaxy_jeux'),
  galaxyImporter: () => appeler<JeuBoutique[]>('boutique_galaxy_importer'),
  galaxyOuvrir: (cle: string) => appeler<void>('boutique_galaxy_ouvrir', { cle }),
  steamEtat: () => appeler<EtatSteam>('boutique_steam_etat'),
  steamRegler: (compte: string, cle?: string) => appeler<EtatSteam>('boutique_steam_regler', { compte, cle: cle ?? null }),
  steamOublier: () => appeler<void>('boutique_steam_oublier'),
  steamJeux: () => appeler<JeuBoutique[]>('boutique_steam_jeux'),
  steamImporter: () => appeler<JeuBoutique[]>('boutique_steam_importer'),
  steamOuvrir: (appid: string, action: 'jouer' | 'installer' | 'verifier') => appeler<void>('boutique_steam_ouvrir', { appid, action }),
  /** `accepte` : la personne a dit oui à l'avertissement de `cheatengineRisque` (jeu en ligne ou inconnu). */
  cheatengineLancer: (programme: string, table?: string, brancher = false, accepte = false) =>
    appeler<void>('cheatengine_lancer', { programme, table: table ?? null, brancher, accepte }),
  /** Les parties d'un jeu Steam que Steam garde pour son nuage (lot B2) : ce qui serait mis à l'abri. */
  nuageSteamApercu: (id: number) => appeler<ApercuNuage>('nuage_steam_apercu', { id }),
  /** Les met à l'abri (`accepte` : la personne a confirmé une très grosse copie). */
  nuageSteamAbriter: (id: number, accepte = false) => appeler<{ dossier: string; fichiers: number; octets: number }>('nuage_steam_abriter', { id, accepte }),
  /** Le risque de brancher Cheat Engine sur ce jeu (Seb, 21/10 : anti-triche = refus ; en ligne ou inconnu = avertir). */
  cheatengineRisque: (id: number) => appeler<RisqueTriche>('cheatengine_risque', { id }),
  jeuTriches: (id: number) => appeler<TrichesJeu>('jeu_triches', { id }),
  /** Les mods installés dans un jeu du PC (contrat 1.19, lot 0.81.0). */
  modsInstalles: (id: number) => appeler<ModInstalle[]>('mods_installes', { id }),
  /** Télécharge et installe un mod (le cœur le reprend dans la liste de Firehouse) ; `programme` : dans le dossier du
   * programme du jeu plutôt qu'à sa racine. */
  modInstaller: (id: number, modNom: string, fichier: string, programme: boolean, suivi: number, risqueAccepte = false) =>
    appeler<{ ajoutes: number; remplaces: number; identiques: number }>('mod_installer', { id, modNom, fichier, programme, suivi, risqueAccepte }),
  /** Retire un mod ; rend les fichiers gardés (modifiés par la personne) et ceux laissés à Steam (sa mise à jour). */
  modDesinstaller: (id: number, cle: string) => appeler<{ gardes: string[]; laisses_a_la_boutique: string[] }>('mod_desinstaller', { id, cle }),
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
  importMesurer: (elements: string[], destination: string) => appeler<[number, number | null]>('import_mesurer', { elements, destination }),
  importCopier: (elements: string[], destination: string) => appeler<string[]>('import_copier', { elements, destination }),
  archiveDuJeu: (id: number) => appeler<import('$lib/decompression/decompression').ArchiveDeJeu | null>('archive_du_jeu', { id }),
  decompresserJeu: (id: number) => appeler<string>('decompresser_jeu', { id }),
  contenusDuJeu: (id: number) => appeler<import('$lib/contenus/contenus').ContenusDuJeu>('contenus_du_jeu', { id }),
  contenusInstaller: (id: number, choisis: string[]) =>
    appeler<{ installes: number; deja?: number; refuses: [string, string][] }>('contenus_installer', { id, choisis }),
  /** Nexus Mods (lot 0.82.0) : la clé du profil (au coffre), les liens nxm:// reçus, l'installation après le oui. */
  /** Reconnaître les jeux importés (contrat 1.21, besoin 28). */
  rapprocherAnnonce: () => appeler<import('$lib/import/rapprochement').Annonce>('rapprocher_annonce'),
  rapprocher: (reseau: boolean) => appeler<import('$lib/import/rapprochement').Rapprochement>('rapprocher', { reseau }),
  rapprochementDernier: () => appeler<import('$lib/import/rapprochement').Rapprochement | null>('rapprochement_dernier'),
  rapprochementRattacher: (jeu: number, mediaId: number) => appeler<void>('rapprochement_rattacher', { jeu, mediaId }),
  rapprochementDefaire: (jeu: number) => appeler<void>('rapprochement_defaire', { jeu }),
  rapprochementEcarter: (jeu: number, mediaId: number) => appeler<void>('rapprochement_ecarter', { jeu, mediaId }),
  rapprochementReprendre: (jeu: number) => appeler<void>('rapprochement_reprendre', { jeu }),
  nexusEtat: () => appeler<import('$lib/mods/nexus.svelte').EtatNexus>('nexus_etat'),
  nexusRegler: (cle: string) => appeler<import('$lib/mods/nexus.svelte').EtatNexus>('nexus_regler', { cle }),
  nexusOublier: () => appeler<void>('nexus_oublier'),
  nexusOuvrir: (id: number, modNom: string) => appeler<void>('nexus_ouvrir', { id, modNom }),
  nexusLiens: () => appeler<import('$lib/mods/nexus.svelte').LienNexus[]>('nexus_liens'),
  nexusOublierLien: (n: number) => appeler<void>('nexus_oublier_lien', { n }),
  nexusLienInfos: (n: number) => appeler<import('$lib/mods/nexus.svelte').InfosNexus>('nexus_lien_infos', { n }),
  /** La réponse à « où poser ce mod rangé dans un seul dossier ? » (`null` : annuler). */
  modRacineRepondre: (choix: boolean | null) => appeler<void>('mod_racine_repondre', { choix }),
  nexusInstaller: (n: number, programme: boolean, suivi: number, risqueAccepte = false) =>
    appeler<{ ajoutes: number; remplaces: number; identiques: number }>('nexus_installer', { n, programme, suivi, risqueAccepte }),
  raEtat: () => appeler<import('$lib/succes/succes.svelte').EtatRetro>('ra_etat'),
  raRegler: (compte: string, cle?: string) =>
    appeler<import('$lib/succes/succes.svelte').EtatRetro>('ra_regler', { compte, cle: cle ?? null }),
  raOublier: () => appeler<void>('ra_oublier'),
  raConnecterEmulateurs: (motDePasse: string) =>
    appeler<import('$lib/succes/succes.svelte').EtatRetro>('ra_connecter_emulateurs', { motDePasse }),
  succesRetro: (id: number) => appeler<import('$lib/succes/succes.svelte').SuccesRetro>('succes_retro', { id }),
  succesSteam: (appid: string) => appeler<[number, number] | null>('succes_steam', { appid }),
  /** La fiche d'un jeu de boutique (lot D) ; `null` : aucune. */
  boutiqueFiche: (source: 'steam' | 'galaxy', cle: string) => appeler<FicheBoutique | null>('boutique_fiche', { source, cle }),
  /** Les jeux possédés sur plusieurs boutiques, réunis en une carte (A6c) : les ids de leurs copies. */
  ludothequeGroupes: () => appeler<number[][]>('ludotheque_groupes'),
  /** Les succès en détail d'un jeu de boutique (lot C) ; `null` : la source n'en a pas. */
  succesListe: (source: 'steam' | 'galaxy', cle: string) => appeler<import('$lib/succes/liste').ListeSucces | null>('succes_liste', { source, cle }),
  /** Les succès en détail d'un jeu DU PC (menu en jeu, Taodbox). */
  succesJeu: (id: number) => appeler<import('$lib/succes/liste').ListeSucces | null>('succes_jeu', { id }),
  abrisRegler: (dossier: string) => appeler<number>('abris_regler', { dossier }),
  importTypesFichiers: (dossier: string, recursif: boolean) => appeler<[string, number][]>('import_types_fichiers', { dossier, recursif }),
  importChercherRoms: (dossier: string, extensions: string[], recursif: boolean) =>
    appeler<{ roms: import('$lib/import/local').RomTrouvee[]; contenus: number }>('import_chercher_roms', { dossier, extensions, recursif }),
  importChercherMame: (dossier: string, liste: string | null, options: import('$lib/import/local').OptionsMame) =>
    appeler<import('$lib/import/local').TriMame>('import_chercher_mame', { dossier, liste, options }),
  importInstallerDos: (dosbox: string, source: string, destination: string, titre: string) =>
    appeler<string[]>('import_installer_dos', { dosbox, source, destination, titre }),
  importArgumentsJeuDos: (destination: string, programme: string) =>
    appeler<string[]>('import_arguments_jeu_dos', { destination, programme }),
  importChercherDos: (dossier: string) => appeler<import('$lib/import/local').JeuDosTrouve[]>('import_chercher_dos', { dossier }),
  importAjouter: (jeux: import('$lib/import/local').JeuAImporter[]) =>
    appeler<import('$lib/import/local').BilanImport>('import_ajouter', { jeux }),
  emulateursRecommandes: (plateforme: string) =>
    appeler<{ emulateurs?: EmulateurRecommande[]; conseil?: Conseil | null }>('emulateurs_recommandes', { plateforme }),
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

/** UNE image précise de la galerie d'un jeu, en miniature (100 à 1 000 px), sans repli. */
export function adresseMedia(jeu: number, media: number, largeur: number): string {
  const l = Math.min(1000, Math.max(100, Math.ceil(largeur / 100) * 100));
  return `http://jaquette.localhost/${jeu}?media=${media}&largeur=${l}`;
}

/** Un visuel d'un jeu (boîte en relief, cartouche…) en miniature ; le cœur se replie sur la jaquette si le jeu ne
 * l'a pas. */
export function adresseVisuel(id: number, sorte: string, largeur?: number): string {
  const l = Math.min(1000, Math.max(100, Math.ceil((largeur ?? 1000) / 100) * 100));
  return `http://jaquette.localhost/${id}?visuel=${sorte},jaquette&largeur=${l}`;
}

/** Une page d'un livret déjà sur le PC (page 0 = la première), rendue en image par Windows à une largeur arrondie
 * aux 200 px supérieurs (entre 200 et 2 400) : quelques tailles seulement, l'image se redimensionne ensuite. */
export function adressePageLivret(jeu: number, doc: number, page: number, largeur: number): string {
  const l = Math.min(2400, Math.max(200, Math.ceil(largeur / 200) * 200));
  return `http://document.localhost/${jeu}/${doc}/${page}?largeur=${l}`;
}

/** Une musique ou une vidéo d'un jeu déjà sur le PC, servie par morceaux par le cœur. */
export function adresseDocument(jeu: number, doc: number): string {
  return `http://document.localhost/${jeu}/${doc}/fichier`;
}

/** Le fond du menu en jeu : le premier visuel que Firehouse a pour ce jeu (fond, sinon capture, sinon écran titre),
 * servi et gardé par le cœur pour le profil ouvert, en 1 000 px au plus. */
export function adresseFondDuJeu(id: number): string {
  return `http://jaquette.localhost/${id}?visuel=fond,capture,ecran_titre&largeur=1000`;
}

/** Le logo d'un jeu (Taodbox), SANS repli sur la jaquette : rien s'il n'en a pas. 600 px suffisent au bandeau. */
export function adresseLogoDuJeu(id: number): string {
  return `http://jaquette.localhost/${id}?visuel=logo&largeur=600`;
}

/** La jaquette officielle d'un jeu de boutique (servie et gardée en cache par le cœur). */
export const adresseImageBoutique = (boutique: string, id: string) => `http://boutique.localhost/${boutique}/${encodeURIComponent(id)}`;

/** Une image publique de boutique (GOG, Steam), servie et gardée en cache par le cœur. */
export const adresseImageParUrl = (url: string) => `http://boutique.localhost/image/${encodeURIComponent(url)}`;

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
