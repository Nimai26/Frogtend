// Les appels au cœur de Frogtend (Rust). Le jeton ne passe jamais par ici : seul le cœur le connaît.

import { invoke, isTauri } from '@tauri-apps/api/core';
import type { ActionProposee } from '$lib/assistant/actions';
import type { CommandesJeu } from '$lib/reglages/reglages';
import type { InfosJeu } from '$lib/ludotheque/infos';
import type { ModJeu } from '$lib/mods/mods';
import type { InfosBoutons } from '$lib/manette/boutons';

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
  // Lot A (18/11) : aussi les boutiques lues sur ce PC (`epic`, `gog`, `uplay`, `amazon`, `xboxone`, `battlenet`).
  source?: 'steam' | 'galaxy' | 'local' | 'epic' | 'gog' | 'uplay' | 'amazon' | 'xboxone' | 'battlenet';
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

/** Lot M1 : un mod posé par Vortex — `source` (son dossier dans le dépôt de Vortex) est la clé, jamais montrée. */
export interface ModVortex {
  source: string;
  nom: string;
  fichiers: number;
}

/** Lot M1 : ce que les journaux du jeu et de ses outils de mods disent (aucun chemin). */
export interface DiagnosticVortex {
  version: string | null;
  version_precedente: string | null;
  refus: { outil: string; version_attendue: string | null }[];
  plante: { chargeur: string; greffon: string } | null;
  erreurs_scripts: number;
  outil_scripts: string | null;
}

/** Lot M1 : l'état des mods de Vortex d'un jeu (`mods_vortex_etat`). */
export interface EtatVortex {
  present: boolean;
  refus: string | null;
  mods: ModVortex[];
  /** Les fichiers PROUVÉS (ceux qu'un retrait rangerait) et leur poids (`octets`). */
  fichiers: number;
  octets: number;
  retrait: { en_cours: boolean; faits: number; total: number; laisses: number; laisses_rompus: number; sauvegardes_vortex: number } | null;
  retires: ModVortex[];
  /** 0.227.1 : les mods REMIS dans le jeu par Frogtend (partie « 🧩 Dans le jeu »), avec leur nombre de fichiers remis. */
  remis: ModVortex[];
  diagnostic: DiagnosticVortex;
  /** Les `source` des mods que le diagnostic désigne. */
  en_cause: string[];
  /** `partie` (le jeu tourne ou est occupé), `galaxy` (GOG Galaxy ouvert) ou `vortex` (Vortex ouvert) : rien ne peut
   * se faire maintenant. */
  verrou: 'partie' | 'galaxy' | 'vortex' | null;
}

export interface BilanRetraitVortex {
  deplaces: number;
  laisses: number;
  laisses_rompus: number;
  mods: number;
  octets: number;
  dossiers_retires: number;
  sauvegardes_vortex: number;
  deploiement_range: boolean;
}

export interface BilanRemiseVortex {
  remis: number;
  places_prises: number;
  changes: number;
  /** Windows a refusé le renommage pour une autre raison (0.227.1). */
  erreurs?: number;
  deploiement_remis: boolean;
}

/** 0.227.1 : ce que le retrait d'UN mod remis a fait. */
export interface BilanRetraitModVortex {
  ranges: number;
  /** Plus ceux que Frogtend avait remis (changés, remplacés, lien sur le chemin). */
  changes: number;
  /** Effacés depuis la remise (ils ne bloquent plus le mod). */
  partis: number;
  places_prises: number;
  /** Windows a refusé le renommage pour une autre raison. */
  erreurs: number;
  /** Partis à part sans être reconnus : proposés à la remise. */
  coinces: number;
  /** Les mods dont au moins un fichier a été rangé. */
  mods: number;
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
  /** Une clé gardée au coffre (keys.txt de la Wii U) : jamais servie, Frogtend dit qu'elle manque. Les clés Switch
   * sont servies depuis le contrat 1.38 (`disponible`, sous `user/keys/`). */
  coffre?: boolean;
  note?: string;
  disponible?: boolean;
  sha1?: string;
  taille?: number;
  url?: string;
  /** Type `motif` : les fichiers servis (`null` est lu comme une liste vide). */
  fichiers?: { nom: string; sha1: string; taille: number; url: string }[] | null;
  /** La version (un firmware, contrat 1.38), quand Firehouse la connaît. */
  version?: string;
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
  /** Contrat 1.41 : une alternative décrit SES fichiers système (`bios`) et SA ligne (`commande`) ; `null` (ou
   * absent) = non décrits : jamais ceux du conseil (Ryubing : ses clés vont dans `portable/system/`, pas dans le
   * `user/keys/` d'Eden ; sa ligne `-f "%rom%"`, jamais `-g`). */
  alternatives: { nom: string; emulateur: string | null; coeur: string | null; quand: string; site: string; paquet?: PaquetCoeur | null; bios?: BiosConseil[] | null; commande?: string | null }[];
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
  en_ligne?: { online: string | null; anticheat: string | null; anticheats?: string[]; source?: string; lu_le?: string } | null;
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

/** Un emplacement de parties d'un jeu PC tel qu'il est sur CE PC (`parties_emplacements`, lot S1). */
export interface EmplacementVu {
  /** Son rang dans le cache du profil. */
  n: number;
  /** Le dossier résolu sur ce PC (à l'écran seulement) ; `null` : rien à montrer. */
  dossier: string | null;
  /** Seulement les fichiers de ce nom ou de ce motif. */
  filtre: string | null;
  nature: 'dossier' | 'fichier' | 'motif' | string;
  /** `firehouse` (PCGamingWiki), `moteur`, `manifeste`, `personne`. */
  origine: string;
  os: string;
  /** La source de Firehouse (contrat 1.48 : `ludusavi`, `pcgamingwiki`) ; `null` avant la 1.48. */
  source: string | null;
  /** Lot S6, origine `moteur` : le nom du moteur d'après les références de Firehouse (« Unity »). */
  moteur?: string | null;
  nuage_boutique: boolean;
  /** Frogtend sait lire ce chemin sur ce PC. */
  lisible: boolean;
  existe: boolean;
  fichiers: number;
  octets: number;
  /** Parmi `fichiers`, ceux qui sont seulement dans le nuage (OneDrive « en ligne seulement »), pas sur ce PC. */
  en_ligne: number;
  /** Trop de fichiers, de dossiers, ou trop profond pour un dossier de parties : pas comptés. */
  trop: boolean;
  /** Le dossier existe mais ne se lit pas : pas compté. */
  illisible: boolean;
}

/** « 📁 Où sont mes parties » d'un jeu PC (`parties_emplacements`, lot S1). */
export interface PartiesVues {
  firehouse: 'inconnu' | 'vide' | 'lu';
  lu_le: string | null;
  hors_ligne: boolean;
  motif: string | null;
  /** Le jeton de ce profil a été refusé : plus de demande pendant cette session. */
  jeton_refuse: boolean;
  moteur: { nom: string; preuve: string } | null;
  /** D'où Firehouse le tient et par quoi il a reconnu le jeu (contrat 1.48). */
  source: string | null;
  reconnu_par: string | null;
  /** Les systèmes dont Firehouse connaît des emplacements qui ne valent pas pour ce jeu (« Steam » pour un jeu Epic). */
  autres_systemes: string[];
  emplacements: EmplacementVu[];
}

/** Un emplacement de parties qui ne sera pas (ou n'a pas été) copié dans l'abri (lot S2) : son dossier sur ce PC, la
 * raison. */
export interface EchecAbriVu {
  dossier: string;
  raison: string;
}

/** Ce que « 🛟 Mettre mes parties à l'abri » copierait (`parties_abri_apercu`, lot S2), annoncé avant. */
export interface ApercuAbriParties {
  fichiers: number;
  octets: number;
  /** Les emplacements qui seront copiés. */
  emplacements: number;
  echecs: EchecAbriVu[];
  /** Fichiers OneDrive « en ligne seulement » : jamais lus pour l'annonce ; copiés seulement si la personne choisit
   * de les télécharger. */
  en_ligne_seulement: number;
  /** Leur taille : ce qu'il faudrait télécharger. */
  octets_en_ligne: number;
  /** Le dernier abri de ce profil contient déjà exactement ces fichiers (jamais quand il y en a en ligne). */
  deja: boolean;
  /** Plus de 2 Go sans les fichiers en ligne : la personne confirme. */
  gros: boolean;
  /** Plus de 2 Go AVEC les fichiers en ligne. */
  gros_avec_nuage: boolean;
  /** Le nom du profil qui sera propriétaire de l'abri (celui ouvert). */
  profil: string;
}

/** Ce que « 🛟 Mettre mes parties à l'abri » a fait (`parties_abriter`, lot S2). */
export interface AbriPartiesFait {
  /** La date de l'abri (secondes) ; `null` : rien de neuf, pas de copie. */
  date: string | null;
  fichiers: number;
  octets: number;
  emplacements: number;
  echecs: EchecAbriVu[];
  en_ligne_seulement: number;
  /** Fichiers téléchargés depuis OneDrive puis mis à l'abri. */
  telecharges: number;
  deja: boolean;
}

/** La progression d'un téléchargement depuis OneDrive pendant un abri (`parties-abri-progres`). */
export interface ProgresAbri {
  jeu: number;
  faits: number;
  total: number;
}

/** Un abri des parties d'un jeu (`parties_abris`, lot S2) : seulement ceux du profil ouvert, et les anciens sans
 * propriétaire connu. */
export interface AbriVu {
  /** Secondes depuis 1970. */
  date: string;
  fichiers: number;
  octets: number;
  /** `emplacements` (fait à la main), `nuage-steam` (parties d'un jeu Steam), `retrait` (avant un retrait). */
  sorte: string;
  /** Fait par ce profil ; `false` : un ancien abri, sans propriétaire connu. */
  a_moi: boolean;
  echecs: number;
  en_ligne_seulement: number;
  /** Fichiers d'un ancien abri Steam pris hors du nuage, dont le chemin d'origine n'a pas été gardé. */
  sans_origine: number;
  /** Lot S3 : ce qui a fait faire un abri de sorte `emplacements` (`main`, `apres_partie`, `avant_retrait`,
   * `avant_reinstallation`, `avant_mise_a_jour`, `avant_mod`, lot S7 : `rattrapage`) ; vide pour les autres sortes (et
   * absent avant le S3). */
  declencheur?: string;
  /** Lot S7 : un abri rattrapé qui attend « C'est toi qui as joué ? » (jamais envoyé avant). */
  a_confirmer?: boolean;
  /** Lot S7 : la personne a dit « Non » : gardé sur ce PC, jamais envoyé. */
  pas_a_moi?: boolean;
}

/** Lot S7 : un tour de RATTRAPAGE des parties jouées hors de Frogtend, à l'ouverture du profil (événement
 * `parties-rattrapage`) : des numéros de jeux, jamais un titre ni un chemin. */
export interface BilanRattrapage {
  profil: string;
  /** Les jeux dont un abri a été écrit pendant ce tour. */
  rattrapes: number[];
  /** Tous les jeux de ce profil dont un abri rattrapé attend « C'est toi qui as joué ? ». */
  a_confirmer: number[];
}

/** Ce qu'a fait un abri AUTOMATIQUE des parties d'un jeu PC (lot S3, événement `parties-abri-auto`) : des numéros et
 * des nombres, jamais un titre ni un chemin. */
export interface BilanAbriAuto {
  jeu: number;
  /** L'identifiant du profil propriétaire (celui qui jouait) : l'interface l'ignore si ce n'est plus le sien. */
  profil: string;
  declencheur: string;
  etat: 'en_cours' | 'fait' | 'rien_de_neuf' | 'aucun' | 'inconnu' | 'en_ligne' | 'trop_gros' | 'echec' | 'attente_depassee';
  fichiers: number;
  octets: number;
  en_ligne: number;
  octets_en_ligne: number;
  echecs: number;
  /** Pourquoi, pour un échec (sans chemin ni titre). */
  motif: string | null;
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
  | { sorte: 'etat'; jeu: number; etat: EtatJeuPc; message: string | null }
  /** Une version AJOUTÉE à un jeu du PC (la ligne du jeu, elle, ne change pas : il reste jouable). */
  | { sorte: 'version'; jeu: number; version: number; etat: EtatJeuPc; message: string | null }
  | { sorte: 'progres_version'; jeu: number; version: number; recus: number; total: number; debit: number };

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
    | 'cartouche_absente'
    /** Lot S3 : les parties n'ont pas pu être mises à l'abri AVANT l'opération (qui ne s'est pas faite). */
    | 'abri_impossible'
    /** Lot S4 : l'espace de la personne chez Firehouse est plein (507, ou place vérifiée avant d'envoyer). */
    | 'plein';
  motif: string;
  /** Pour `plein` : ce qui manque (octets), quand c'est connu. */
  manque_octets?: number;
  /** Pour `fusionne` (contrat 1.14) : la fiche où celle-ci a été réunie. */
  nouveau?: number;
  /** Pour `cartouche_absente` : la fiche de la cartouche e-Reader qui lit la carte. */
  cartouche?: number;
  /** Pour `abri_impossible` : `onedrive`, `trop_gros`, `inconnu` (Frogtend ne sait pas encore où sont les parties) ou
   * `echec`. */
  cause?: 'onedrive' | 'trop_gros' | 'inconnu' | 'echec';
  /** Pour `abri_impossible` : l'opération arrêtée (`avant_retrait`, `avant_reinstallation`, `avant_mise_a_jour`,
   * `avant_mod`). */
  operation?: string;
  fichiers_en_ligne?: number;
  octets_en_ligne?: number;
  octets?: number;
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
  /** Connecté par la page officielle de Steam (lot C1) : son steamid est connu. */
  connecte: boolean;
  /** La clé d'API rangée est celle d'un AUTRE compte que celui connecté (l'import par la clé est refusé). */
  cle_autre_compte: boolean;
}

/** Ce que « Oublier ma connexion Steam » a fait (lot C3) : le compte est toujours oublié ; la déconnexion de Steam
 * dans le navigateur du profil n'est vraie que prouvée par le cœur. */
export interface OubliSteam {
  navigateur_deconnecte: boolean;
}

/** Ce qu'un import Steam par la connexion a donné (lot C2) ; le jeton d'accès ne quitte jamais le cœur. */
export interface ImportSteam {
  /** La connexion à Steam ne vaut plus : « reconnecte-toi » (les jeux déjà importés restent). */
  reconnecter: boolean;
  total: number;
  /** Les cartes nouvelles dans la ludothèque, et combien sont installées. */
  nouveaux: number;
  nouveaux_installes: number;
}

/** Ce qu'une lecture des jeux du compte GOG a donné (lot C4a) : la même forme que Steam ; rien du compte ne quitte le
 * cœur (ni numéro, ni nom, ni cookie). */
export interface ImportGog extends ImportSteam {
  /** GOG Galaxy est installé sur ce PC (sans lui, « ⬇ Installer » ne peut rien pour l'instant). */
  galaxy: boolean;
  /** Lot C5 : la bibliothèque de GOG Galaxy a été lue en entier (sinon : liste partielle, nouvel essai plus tard). */
  bibliotheque_galaxy?: boolean;
  /** Lot C4b : ce que la lecture a donné pour chaque compte AJOUTÉ (le compte du navigateur : les champs ci-dessus). */
  comptes?: ResultatCompteGog[];
  /** Lot C4b : la lecture du compte du navigateur a échoué (message), alors que des comptes ajoutés ont été lus. */
  erreur?: string | null;
  /** Lot C4b : combien de comptes ont été lus (0 : rien n'a été lu). */
  lus?: number;
}

/** Ce que la lecture d'un compte GOG ajouté a donné (lot C4b). */
export interface ResultatCompteGog {
  cle: string;
  /** Le pseudo GOG (vide : « Compte GOG n »). */
  nom: string;
  /** Le n de « Compte GOG n », rangé. */
  numero?: number;
  /** Sa connexion est perdue : « ➕ Ajouter un compte GOG » le reconnecte. */
  reconnecter: boolean;
  /** Sa bibliothèque n'a pas pu être lue pour l'instant (ses jeux restent). */
  partiel: boolean;
}

/** Un compte GOG du profil (lot C4b) : jamais son numéro à l'écran (le pseudo, sinon « Compte GOG n »). */
export interface CompteGogEtat {
  cle: string;
  nom: string;
  /** Le compte de la session gog.com de Frogtend (celui du jeu offert). */
  profil: boolean;
  /** Le n de « Compte GOG n », rangé. */
  numero?: number;
  nb_jeux: number;
  maj_le: string | null;
  /** Un compte ajouté dont la connexion est perdue. */
  reconnecter: boolean;
}

/** Ce que Frogtend sait des comptes GOG du profil (lots C4a, C4b). */
export interface EtatGog {
  /** Le compte du navigateur du profil a déjà été lu. */
  connu: boolean;
  /** Les jeux de tous les comptes réunis (une carte par jeu). */
  nb_jeux: number;
  maj_le: string | null;
  /** Lot C4b : tous les comptes, celui du navigateur d'abord. */
  comptes?: CompteGogEtat[];
  /** Lot C4b : GOG Galaxy est ouvert sur un autre compte que ceux de Frogtend (seulement si c'est sûr). */
  galaxy_autre_compte?: boolean;
  /** Lot C4b : le compte du navigateur a été oublié (sa session n'est plus relue avant « 🔑 Se connecter à GOG »). */
  navigateur_oublie?: boolean;
}

/** Ce que « ➕ Ajouter un compte GOG » a donné (lot C4b) ; `fait` faux : fenêtre fermée sans connexion. */
export interface AjoutGog {
  fait: boolean;
  cle: string;
  nom: string;
  numero: number;
  /** C'est le seul compte GOG du profil. */
  premier: boolean;
  reconnexion: boolean;
  jeux: number;
  partiel: boolean;
  nouveaux: number;
  nouveaux_installes: number;
  galaxy: boolean;
}

/** Ce que « Oublier ce compte » GOG a fait (lot C4b). */
export interface OubliGog {
  /** C'était le compte du navigateur du profil. */
  profil: boolean;
  /** Sa connexion a bien quitté le coffre de Windows. */
  secret_oublie: boolean;
  /** Compte du navigateur : déconnexion de GOG dans Frogtend PROUVÉE. */
  navigateur_deconnecte: boolean;
}

/** Une boutique telle que ce PC la voit (import automatique, lot A). */
export interface EtatBoutiquePc {
  boutique: string;
  nom: string;
  /** Le lanceur est là (Jouer, Installer). */
  lanceur: boolean;
  /** Ses jeux installés ont pu être lus. */
  lu: boolean;
  installes: number;
  /** Ses jeux POSSÉDÉS, installés compris (lot B : Epic, Ubisoft) ; `null` : pas lus pour cette boutique. */
  possedes?: number | null;
}

/** La relecture des boutiques de ce PC à l'ouverture d'un profil : (boutique, nombre de jeux NOUVEAUX, dont
 * installés). */
export interface ReleveBoutiques {
  boutiques: EtatBoutiquePc[];
  nouveaux: [string, number, number][];
}

/** Ce que montre le menu en jeu. */
export interface EtatMenuJeu {
  /** Les réglages changés en jeu qui ne valent qu'au prochain lancement (vide : rien à relancer). */
  a_relancer: string[];
  /** « Relancer » possible (un émulateur, jamais un jeu PC). */
  relancer_possible: boolean;
  /** « Sauver puis relancer » : seulement si la sauvegarde rapide est vérifiée avant de quitter. */
  sauver_avant: boolean;
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
  /** La fiche Firehouse du jeu pour ce profil (un import relié : celle de Firehouse), pour son fond. */
  fiche: number | null;
  /** L'emplacement de sauvegarde rapide choisi (RetroArch, MAME) ; `null` : celui de l'émulateur. */
  emplacement: number | null;
  /** Les triches du jeu activables depuis le menu (RetroArch hors mode hardcore) ; `[]` : aucune posée ; `null` :
   * pas de triches depuis le menu. */
  triches: { titre: string; active: boolean }[] | null;
  /** Les triches changent tout de suite (RetroArch) ; sinon (BigPEmu) le choix vaut au prochain lancement. */
  triches_en_direct: boolean;
  /** La partie du jeu est déjà à l'abri pour cette partie (la mise en garde a été lue). */
  abri_triches: boolean;
  /** RPCS3 a refusé une sauvegarde rapide faute de son « mode compatible ». */
  refus_etat: boolean;
  /** L'échec d'un raccourci de sauvegarde rapide (menu fermé), dit une fois à l'ouverture. */
  message: string | null;
  /** Les effets d'image (`null` : pas d'écran) : RetroArch les change tout de suite (`en_direct`) ; BigPEmu au
   * prochain lancement. */
  effets: { presents: string[]; actuel: string | null; de_la_personne: boolean; en_direct: boolean } | null;
  /** Les images par seconde : affichées (`null` : inconnu) ; la bascule se voit-elle tout de suite (sinon : à la
   * prochaine partie). `null` : pas d'entrée. */
  fps: { affiche: boolean | null; en_direct: boolean } | null;
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

/** Une partie restaurée en double (dite UNE fois) : sa copie, et la partie en place, gardée. */
export interface ConflitRestaure {
  cle: string;
  etiquette: string;
  copie: string;
  cible: string;
}

/** Lot S5 : ce qu'une restauration (ou la reprise d'un jeu) téléchargerait, annoncé avant le oui. */
export interface ApercuRestauration {
  fichiers: number;
  octets: number;
  /** Les jeux (visibles de ce profil) dont des parties sont dans la sauvegarde. */
  jeux: number;
}

/** Lot S5 : ce que la pose des parties PC restaurées a fait (des nombres d'emplacements, sauf `fichiers`). */
export interface BilanReprise {
  jeux: number;
  posees: number;
  fichiers: number;
  deja_la: number;
  non_installes: number;
  non_resolues: number;
  identifiants: number;
  en_cours: number;
  conflits: number;
  partielles: number;
  /** Copies téléchargées pas entières (reprise coupée) : à reprendre. */
  incompletes?: number;
  /** La reprise d'un jeu : le sel de ce PC diffère de celui de Firehouse. */
  sel_ecart?: boolean;
}

/** Un côté d'un conflit : date (secondes, `null` : inconnue), taille, nombre de fichiers — jamais un chemin. */
export interface CoteConflit {
  date: number | null;
  octets: number;
  fichiers: number;
}

/** Lot S5 : une partie PC de Firehouse différente de celle de ce PC (jamais écrasée sans le choix de la personne). */
export interface ConflitEmplacement {
  jeu: number;
  cle: string;
  titre: string;
  ici: CoteConflit;
  firehouse: CoteConflit;
  partiel: boolean;
}

/** Ce qu'une repose de parties restaurées a fait. */
export interface Reposes {
  reposes: number;
  deja_la: number;
  conflits: string[];
  a_remplacer: ConflitRestaure[];
}

/** Mods Switch (contrat 1.44, ✅ Seb 18/11) : un mod vu sur la fiche (`mods_switch_lister`). */
export interface ModSwitchVu {
  cle: string;
  nom: string;
  description: string;
  auteur: string;
  licence: string;
  source: string;
  revision: number;
  revision_rangee: number | null;
  etat: 'pas_telecharge' | 'telecharge' | 'revision_dispo' | 'incompatible';
  /** `pas_verifie` : jamais grisé ; `incompatible` : grisé, `raison` à montrer, jamais posé. */
  compat: 'compatible' | 'pas_verifie' | 'incompatible';
  raison: string | null;
  allume: boolean;
  /** Taille de l'archive servie, en octets (à montrer avant l'accord). */
  taille: number;
  /** L'empreinte de l'archive : rendue telle quelle dans l'accord (le cœur refuse si le contenu a changé). */
  sha256: string;
  conflits: string[];
  /** Non vide : jamais posé seul (« réglage d'émulateur requis »). */
  reglage_requis: string;
  touche_sauvegardes: boolean | null;
  contenu: { exefs: boolean; romfs: boolean; cheats: boolean };
  emulateurs_essayes: string[];
  /** Faux : rangé sur ce PC, Firehouse ne le sert plus. */
  servi: boolean;
  /** Contrat 1.46 (fiche écrite par l'IA de Firehouse ; `null` tant qu'elle n'est pas écrite) : la sorte du mod. */
  type: 'technique' | 'contenu' | 'cosmetique' | 'triche' | 'autre' | null;
  /** 1 à 5 (5 : utile à tous) ; la liste arrive triée du plus intéressant au moins intéressant. */
  interet: number | null;
}

/** La liste des mods Switch d'un jeu. `etat` : ok | hors_ligne (`vue_le` : date de la dernière liste lue) | injoignable | rien. */
export interface ModsSwitch {
  etat: 'ok' | 'hors_ligne' | 'injoignable' | 'rien';
  vue_le: string | null;
  tid_base: string | null;
  version_servie: { version: number | null; version_affichee: string | null; build_id: string | null } | null;
  version_connue: boolean;
  mods: ModSwitchVu[];
  remarque: string | null;
  /** Ce que la dernière pose dans un émulateur a dit (`quand` : secondes depuis 1970 ; une raison par mod pas posé ou
   * par souci), ou `null`. */
  derniere_pose: { quand: string; emulateur: string; raisons: string[] } | null;
}

/** Où en sont les téléchargements de mods Switch d'un jeu. */
export interface SuiviModsSwitch {
  actif: boolean;
  en_cours: string | null;
  recus: number;
  total: number;
  faits: number;
  a_faire: number;
  erreurs: string[];
  arret: string | null;
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
  /** Retire (ou annule) UNE version ajoutée d'un jeu (jamais la version d'origine, jamais les parties). */
  retirerVersion: (id: number, version: number) => appeler<JeuPc>('jeu_retirer_version', { id, version }),
  /** Les versions ajoutées d'un jeu du PC et où elles en sont. */
  versionsAjoutees: (id: number) =>
    appeler<{ version: number; etat: 'attente' | 'en_cours' | 'pause' | 'telecharge' | 'erreur'; message: string | null }[]>('jeu_versions_ajoutees', { id }),
  /** Retire UNE carte d'un jeu e-Reader (jamais la dernière, jamais les parties). */
  retirerCarte: (id: number, version: number) => appeler<JeuPc>('jeu_retirer_carte', { id, version }),
  /** `autres` : d'autres versions (régions) du même jeu, sur la même ligne — ROM ou images disque seulement. */
  ajouterJeu: (id: number, version: number, emplacement: string, autres: number[] = []) =>
    appeler<Omit<JeuPc, 'recus'>>('jeu_ajouter', { id, version, emplacement, autres }),
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
  /** Arrête le téléchargement d'un document (ce qui est reçu est gardé pour la reprise). */
  arreterDocument: (id: number, doc: number) => appeler<void>('document_arreter', { id, doc }),
  /** Hors ligne : les fiches des jeux importés reliés lues à l'avance (150 au plus par passage). */
  prelireFichesReliees: () => appeler<number>('fiches_reliees_prelire'),
  /** Contrat 1.33 : le pack MAME (dossier proposé ou choisi, annonce chiffrée, réception, arrêt). */
  packMameAnnonce: (dossier: string | null, avecChd: boolean) =>
    appeler<[string, import('$lib/import/packMame').AnnoncePack]>('pack_mame_annonce', { dossier, avecChd }),
  /** `maj` : la version ANNONCÉE, quand la personne a accepté la mise à jour (refusé si Firehouse a changé depuis). */
  packMameLancer: (dossier: string, avecChd: boolean, maj: string | null = null) =>
    appeler<{ recus: number; octets: number; laisses: [string, string][]; arret: string | null }>('pack_mame_lancer', { dossier, avecChd, maj }),
  packMameArreter: () => appeler<void>('pack_mame_arreter'),
  packMameLudotheque: (dossier: string) =>
    appeler<{ jeux: number; versions: number; deja: number; sans_fiche: number }>('pack_mame_ludotheque', { dossier }),
  /** Contrat 1.32 : identifier les jeux importés sans fiche. `annonce` : [à envoyer, déjà cherchés, lots]. */
  identificationAnnonce: () => appeler<[number, number, number]>('identification_annonce'),
  identificationLancer: (tout = false) =>
    appeler<{ envoyes: number; exacts: number; probables: number; inconnus: number; motif?: string | null }>('identification_lancer', { tout }),
  identificationAChoisir: () => appeler<import('$lib/import/identification').AChoisir[]>('identification_a_choisir'),
  identificationChoisir: (jeu: number, launchboxId: number) => appeler<void>('identification_choisir', { jeu, launchboxId }),
  identificationEcarter: (jeu: number) => appeler<void>('identification_ecarter', { jeu }),
  ficheBase: (jeu: number) => appeler<import('$lib/import/identification').FicheBase | null>('jeu_fiche_base', { jeu }),
  /** Ce qu'une sauvegarde enverrait : taille de toutes les parties, celle de la dernière sauvegarde ; lot S4 : ce qui
   * partira vraiment, la place utilisée et le plafond chez Firehouse (`null` : pas lus). Rien n'est écrit. */
  sauvegardeEstimer: () => appeler<import('./sauvegardeTaille').EstimationSauvegarde>('sauvegarde_estimer'),
  /** Lot S4 : la place utilisée chez Firehouse (tous profils et PC de la personne) et le plafond ; `null` : inconnus. */
  sauvegardePlace: () => appeler<{ utilise: number; quota: number } | null>('sauvegarde_place'),
  /** `ecarter` : la personne a choisi de REMPLACER une sauvegarde existante qu'elle n'a pas restaurée. */
  /** Lot S4 : `plafondAuto` (une sauvegarde automatique) : au-delà de ces octets à envoyer vraiment, le cœur n'envoie
   * rien (refus « remise à plus tard ») — il vérifie aussi lui-même la place chez Firehouse. */
  sauvegarder: (ecarter = false, plafondAuto: number | null = null) =>
    appeler<import('./sauvegarde.svelte').BilanSauvegarde>('sauvegarde_lancer', { ecarter, plafondAuto }),
  derniereSauvegarde: () => appeler<import('./sauvegarde.svelte').BilanSauvegarde | null>('sauvegarde_derniere'),
  restaurationListe: () =>
    appeler<{ profil: string; pc: string; derniere: { date?: string; fichiers?: number; taille?: number } | null; octets: number; a_moi: boolean }[]>(
      'restauration_liste',
    ),
  restaurationPreparer: (profil: string, pc: string) =>
    appeler<[{ configuration: Record<string, any>; bibliotheque: { jeux?: any[] }; fichiers: number; taille: number; sel?: string }, Reposes]>(
      'restauration_preparer',
      { profil, pc },
    ),
  espaceLibre: (chemin: string) => appeler<number | null>('espace_libre', { chemin }),
  /** Le dossier des jeux proposé quand rien n'est réglé : `<disque fixe le plus libre>\Frogtend\Jeux` (rien n'est créé). */
  dossierJeuxPropose: () => appeler<{ chemin: string; disque: string; libre: number; disques: string[]; externe: boolean } | null>('dossier_jeux_propose'),
  /** Crée le dossier PROPOSÉ (seul celui-là est admis par le cœur), après le oui de la personne. */
  dossierJeuxCreer: (chemin: string) => appeler<string>('dossier_jeux_creer', { chemin }),
  /** Ce que le cœur dit de chaque version d'une fiche avant qu'on coche (cochable, région, fichiers). */
  versionsAvis: (fiche: unknown) => appeler<import('$lib/ludotheque/versions').AvisVersion[]>('versions_avis', { fiche }),
  /** Le bouton Xbox ouvre-t-il la Game Bar de Windows ? (`null` : hors de Windows.) */
  gameBarBouton: () => appeler<boolean | null>('game_bar_bouton'),
  /** Change CE seul réglage de Windows, après le oui de la personne. */
  gameBarBoutonRegler: (actif: boolean) => appeler<void>('game_bar_bouton_regler', { actif }),
  /** Pourquoi ce dossier ne convient pas pour ranger des jeux (racine d'un disque, Windows, Documents…), ou `null`. */
  dossierJeuxRefus: (chemin: string, sorte?: 'emulateurs' | 'outils' | 'abris' | 'captures') =>
    appeler<string | null>('dossier_jeux_refus', { chemin, sorte: sorte ?? null }),
  /** Repose les parties d'émulateurs restaurées qui attendaient un émulateur absent (jamais par-dessus une autre). */
  restaurationReposerEmulateurs: () => appeler<Reposes>('restauration_reposer_emulateurs'),
  /** Les captures d'écran restaurées, remises dans le dossier des captures réglé (sinon elles attendent). */
  restaurationReposerCaptures: () => appeler<{ reposees: number; en_attente: number }>('restauration_reposer_captures'),
  /** Remplace la partie en place par sa copie restaurée (sa partie actuelle mise à l'abri d'abord) ; rend l'abri. */
  restaurationRemplacer: (cle: string) => appeler<string>('restauration_remplacer', { cle }),
  /** Les parties restaurées encore en double (une proposition fermée sans répondre n'est jamais perdue). */
  restaurationConflits: () => appeler<ConflitRestaure[]>('restauration_conflits'),
  /** Lot S5 : ce qu'une restauration complète téléchargerait (fichiers, taille, jeux), annoncé avant le oui. */
  restaurationApercu: (profil: string, pc: string) => appeler<ApercuRestauration>('restauration_apercu', { profil, pc }),
  /** Lot S5 : après une restauration complète, pose d'office ce qui manque des parties PC (jamais par-dessus une autre). */
  restaurationPoserPartiesPc: () => appeler<BilanReprise>('restauration_poser_parties_pc'),
  /** Lot S5 : ce que « 📥 Reprendre mes parties de Firehouse » téléchargerait pour ce jeu. */
  partiesRepriseApercu: (id: number, profil: string, pc: string) => appeler<ApercuRestauration>('parties_reprise_apercu', { id, profil, pc }),
  /** Lot S5 : télécharge SEULEMENT les parties PC de ce jeu, puis pose ce qui manque (`parties-reprise-progres`). */
  partiesReprendre: (id: number, profil: string, pc: string) => appeler<BilanReprise>('parties_reprendre', { id, profil, pc }),
  /** Lot S5 : les parties PC de Firehouse en conflit avec celles de ce PC (`id` : seulement ce jeu). */
  partiesConflits: (id: number | null = null) => appeler<ConflitEmplacement[]>('parties_conflits', { id }),
  /** Lot S5 : `garder` (celle de ce PC) ou `reprendre` (celle de Firehouse, la sienne à l'abri d'abord). */
  partiesConflitRegler: (jeu: number, cle: string, choix: 'garder' | 'reprendre') => appeler<boolean>('parties_conflit_regler', { jeu, cle, choix }),

  preparerInstallation: (id: number) => appeler<Preparation>('installation_preparer', { id }),
  /** `suivi` : l'avancement de la décompression arrive par « mod-progres » sous ce numéro (voir `avecProgres`). */
  installer: (id: number, automatique: boolean, suivi: number | null = null) => appeler<Installation>('installation_lancer', { id, automatique, suivi }),
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
  /** Change l'emplacement de sauvegarde rapide choisi (−1 ou +1) ; rend le nouveau. */
  menuJeuEmplacement: (sens: number) => appeler<number | null>('menu_jeu_emplacement', { sens }),
  /** Les sauvegardes rapides du jeu en cours : [emplacement, date en secondes ou null si vide]. */
  menuJeuEmplacements: () => appeler<[number, number | null][]>('menu_jeu_emplacements'),
  menuJeuTriche: (index: number) => appeler<void>('menu_jeu_triche', { index }),
  /** BigPEmu : une triche est cochée pour ce jeu (rappel sous ▶ Jouer : sa fenêtre de triches attend A puis B). */
  bigpemuTrichesCochees: (id: number) => appeler<boolean>('bigpemu_triches_cochees', { id }),
  /** Change l'effet d'image en jeu (RetroArch) : `aucun` ou un identifiant d'effet. */
  menuJeuEffet: (effet: string) => appeler<void>('menu_jeu_effet', { effet }),
  /** Le compteur d'images par seconde pour les prochaines parties (émulateur qui ne bascule pas en jeu). */
  menuJeuFps: (afficher: boolean) => appeler<void>('menu_jeu_fps', { afficher }),
  /** Les boutons d'un jeu du PC, ou (`null`) de la partie en cours (lot 4d). */
  boutonsDuJeu: (id: number | null, wiimoteChoisie: string | null = null) => appeler<InfosBoutons | null>('boutons_du_jeu', { id, wiimoteChoisie }),
  /** La fenêtre du menu en jeu, agrandie pour l'écran des boutons (puis rendue à sa taille). */
  menuJeuAgrandir: (grand: boolean) => appeler<void>('menu_jeu_agrandir', { grand }),
  menuJeuQuitter: () => appeler<number>('menu_jeu_quitter'),
  /** Relancer le jeu (✅ Seb 15/11) : une sauvegarde rapide d'abord si `sauver`, puis Quitter ; la fenêtre principale
   * le relance à la fin de la partie. */
  menuJeuRelancer: (jeu: number, sauver: boolean, recharger: boolean) => appeler<number>('menu_jeu_relancer', { jeu, sauver, recharger }),
  /** Plus aucun rechargement en attente (relance ratée, ou lancement à la main). */
  menuJeuOublierRechargement: () => appeler<void>('menu_jeu_oublier_rechargement'),
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
    image?: string,
    fps?: boolean,
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
      image: image ?? null,
      fps: fps ?? null,
    }),
  /** « ▶ Ouvrir l'émulateur seul » (✅ Seb 17/11) : l'émulateur `cle` du système, ouvert sans jeu avec le profil de la
   * personne ; rend son nom. Sa fermeture arrive par l'événement « emulateur-seul ». */
  emulateurOuvrirSeul: (plateforme: string, cle: string) => appeler<string>('emulateur_ouvrir_seul', { plateforme, cle }),
  /** Le nom de l'émulateur ouvert seul en ce moment (relu après un rechargement), ou `null`. */
  emulateurSeulEtat: () => appeler<string | null>('emulateur_seul_etat'),
  /** Des parties à l'abri à remettre dans ce jeu réinstallé (lot 3). */
  /** Le mode compatible des sauvegardes rapides d'un jeu PS3 : [actif, réglé pour ce jeu] ; `null` : sans objet. */
  rpcs3EtatsCompatibles: (id: number) => appeler<[boolean, boolean] | null>('rpcs3_etats_compatibles', { id }),
  rpcs3ReglerEtatsCompatibles: (id: number, actif: boolean) => appeler<void>('rpcs3_regler_etats_compatibles', { id, actif }),
  partiesAbriAReposer: (id: number) => appeler<{ date: string; fichiers: number; octets: number; gardes: number } | null>('parties_abri_a_reposer', { id }),
  partiesAbriReposer: (id: number) => appeler<number>('parties_abri_reposer', { id }),
  partiesAbriIgnorer: (id: number, date: string) => appeler<void>('parties_abri_ignorer', { id, date }),
  /** Un jeu PS3 qui sera long à démarrer (RPCS3 compile) : la phrase à dire avant de jouer. */
  jeuCompilationPs3: (id: number, emulateur?: string, version?: string) =>
    appeler<string | null>('jeu_compilation_ps3', { id, emulateur: emulateur ?? null, version: version ?? null }),
  /** Les effets d'image d'un RetroArch (lot 4a/4b). */
  effetsImage: (programme: string, jeu?: number) =>
    appeler<{ impossible: string | null; effets: { id: string; present: boolean }[] }>('effets_image', { programme, jeu: jeu ?? null }),
  pleinEcranSuggestion: (id: number) => appeler<string | null>('jeu_plein_ecran_suggestion', { id }),
  jeuAntiTriche: (id: number) => appeler<VerificationFiger>('jeu_anti_triche', { id }),
  biosPoser: (plateforme: string, programme: string) => appeler<BilanBios>('bios_poser', { plateforme, programme }),
  firmwareServi: (plateforme: string) => appeler<BiosConseil | null>('firmware_servi', { plateforme }),
  rpcs3MicrologicielServi: (programme: string) => appeler<string>('rpcs3_micrologiciel_servi', { programme }),
  /** Vita3K : les fichiers du firmware de la Vita servis par Firehouse qui manquent encore (vide : rien à faire). */
  vita3kFirmwareManquant: (programme: string) => appeler<BiosConseil[]>('vita3k_firmware_manquant', { programme }),
  /** Vita3K : installe ces fichiers (accord déjà donné) ; rend le nombre installé. */
  vita3kFirmwareServi: (programme: string) => appeler<number>('vita3k_firmware_servi', { programme }),
  /** Eden (et sa famille) ou Ryubing (`portable\system\prod.keys`, `registered\`) : portable, clés posées (prod.keys),
   * firmware déjà là. */
  edenFirmware: (programme: string) =>
    appeler<{ portable: boolean; cles: boolean; firmware: boolean; version: string | null; de_frogtend: boolean }>('eden_firmware', { programme }),
  /** Le firmware Switch servi par Firehouse, posé dans un Eden (ou un Ryubing : celui de SON alternative, contrat 1.41)
   * fermé, portable, avec ses clés et SANS firmware (rend le nombre de fichiers posés ; progression : événement
   * « emulateur », id « eden-firmware »). */
  edenFirmwareServi: (programme: string, remplacer = false) => appeler<number>('eden_firmware_servi', { programme, remplacer }),
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
  /** `possedes` : le profil a accepté les jeux de ses comptes (lot B) ; un jeu offert déjà dans le compte Epic du
   * lanceur est alors « obtenu » sans clic. */
  gratuitsListe: (possedes = false) => appeler<JeuOffert[]>('gratuits_liste', { possedes }),
  gratuitsConnexionEpic: () => appeler<void>('gratuits_connexion_epic'),
  /** Le vrai état du compte Epic du profil (page cachée) : vrai, faux, ou null si pas sûr. */
  gratuitsVerifierEpic: () => appeler<boolean | null>('gratuits_verifier_epic'),
  gratuitsConnexion: (boutique: 'gog' | 'prime') => appeler<void>('gratuits_connexion', { boutique }),
  gratuitsRecuperer: (boutique: 'gog' | 'prime') => appeler<Recolte>('gratuits_recuperer', { boutique }),
  gratuitsConnexionPlaystation: () => appeler<void>('gratuits_connexion_playstation'),
  gratuitsPsPlus: () => appeler<RecoltePsPlus>('gratuits_psplus'),
  gratuitsObtenirEpic: (slug: string) => appeler<{ etat: string; motif?: string }>('gratuits_obtenir_epic', { slug }),
  /** L'icône du lanceur d'une boutique installé sur ce PC (`data:image/png;base64,…`), `null` : la pastille garde son texte. */
  boutiqueIcone: (boutique: string) => appeler<string | null>('boutique_icone', { boutique }),
  galaxyEtat: () => appeler<{ galaxy_installe: boolean; nb_jeux: number; maj_le: string | null }>('boutique_galaxy_etat'),
  galaxyJeux: () => appeler<JeuBoutique[]>('boutique_galaxy_jeux'),
  galaxyImporter: () => appeler<JeuBoutique[]>('boutique_galaxy_importer'),
  galaxyOuvrir: (cle: string) => appeler<void>('boutique_galaxy_ouvrir', { cle }),
  steamEtat: () => appeler<EtatSteam>('boutique_steam_etat'),
  steamRegler: (compte: string, cle?: string) => appeler<EtatSteam>('boutique_steam_regler', { compte, cle: cle ?? null }),
  steamOublier: () => appeler<OubliSteam>('boutique_steam_oublier'),
  steamConnexion: () => appeler<void>('boutique_steam_connexion'),
  steamJeux: () => appeler<JeuBoutique[]>('boutique_steam_jeux'),
  steamImporter: () => appeler<JeuBoutique[]>('boutique_steam_importer'),
  /** Lot C2 : les jeux possédés du compte connecté par la page, sans clé d'API. */
  steamImporterConnecte: () => appeler<ImportSteam>('boutique_steam_importer_connecte'),
  steamOuvrir: (appid: string, action: 'jouer' | 'installer' | 'verifier') => appeler<void>('boutique_steam_ouvrir', { appid, action }),
  /** Lot C4a : ce que Frogtend sait du compte GOG du profil (sans rien du compte). */
  gogEtat: () => appeler<EtatGog>('boutique_gog_etat'),
  /** Lot C4a : les jeux possédés du compte GOG connecté dans le navigateur du profil (jeux cachés compris). */
  /** Lot C5 : `montrer` (juste après « Se connecter à GOG ») : la page de connexion façon Galaxy peut s'afficher. */
  /** Lot C4b : `reconnexion` : la personne vient de se connecter elle-même (un compte du navigateur oublié est relu). */
  gogImporterConnecte: (montrer = false, reconnexion = false) => appeler<ImportGog>('boutique_gog_importer_connecte', { montrer, reconnexion }),
  /** Lot C4b : avant d'installer un jeu GOG, l'avertissement « GOG Galaxy est sur un autre compte » (ou `null`). */
  gogAvertissementInstallation: (cle: string) => appeler<string | null>('boutique_gog_avertissement_installation', { cle }),
  /** Lot C4b : « ➕ Ajouter un compte GOG » (fenêtre de connexion de GOG, montrée, navigation privée). */
  gogAjouterCompte: () => appeler<AjoutGog>('boutique_gog_ajouter_compte'),
  /** Lot C4b : « Oublier ce compte » GOG (ses jeux restent dans la ludothèque). */
  gogOublierCompte: (cle: string) => appeler<OubliGog>('boutique_gog_oublier_compte', { cle }),
  /** Import automatique (lot A) : relit les jeux installés de toutes les boutiques de ce PC et les range. */
  /** `possedes` : le profil a accepté ses jeux possédés (lot B) ; sinon seuls les jeux installés sont rangés. */
  boutiquesRelirePc: (possedes = false) => appeler<ReleveBoutiques>('boutiques_relire_pc', { possedes }),
  /** Le profil refuse ses jeux possédés : leurs cartes jamais installées ni jouées sont retirées (rend leur nombre). */
  boutiquesRetirerPossedes: () => appeler<number>('boutiques_retirer_possedes'),
  /** L'état des boutiques de ce PC (lecture seule). */
  boutiquesEtatPc: () => appeler<EtatBoutiquePc[]>('boutiques_etat_pc'),
  /** Jouer à un jeu de boutique ou l'installer, par SON lanceur (clé de la carte). */
  boutiqueOuvrir: (boutique: string, cle: string, action: 'jouer' | 'installer') => appeler<void>('boutique_ouvrir', { boutique, cle, action }),
  /** `accepte` : la personne a dit oui à l'avertissement de `cheatengineRisque` (jeu en ligne ou inconnu). */
  cheatengineLancer: (programme: string, table?: string, brancher = false, accepte = false) =>
    appeler<void>('cheatengine_lancer', { programme, table: table ?? null, brancher, accepte }),
  /** Les parties d'un jeu Steam que Steam garde pour son nuage (lot B2) : ce qui serait mis à l'abri. */
  nuageSteamApercu: (id: number) => appeler<ApercuNuage>('nuage_steam_apercu', { id }),
  /** Les met à l'abri (`accepte` : la personne a confirmé une très grosse copie). */
  nuageSteamAbriter: (id: number, accepte = false) => appeler<{ dossier: string; fichiers: number; octets: number }>('nuage_steam_abriter', { id, accepte }),
  /** 📁 Où sont mes parties (lot S1) : les emplacements de parties d'un jeu PC, résolus sur ce PC (lecture seule ;
   * Firehouse n'est redemandé qu'au bout de 30 jours). */
  partiesEmplacements: (id: number) => appeler<PartiesVues>('parties_emplacements', { id }),
  /** Ouvre dans l'Explorateur un dossier de parties EXISTANT de ce jeu (le cœur revérifie). */
  partiesOuvrirDossier: (id: number, dossier: string) => appeler<void>('parties_ouvrir_dossier', { id, dossier }),
  /** 🛟 Ce que « Mettre mes parties à l'abri » copierait (lot S2 ; lecture seule). */
  partiesAbriApercu: (id: number) => appeler<ApercuAbriParties>('parties_abri_apercu', { id }),
  /** 🛟 Met les parties d'un jeu PC à l'abri pour le profil ouvert (`accepte` : très grosse copie confirmée ;
   * `rapatrier` : la personne a choisi de télécharger d'abord les fichiers « en ligne seulement » de OneDrive). */
  partiesAbriter: (id: number, accepte = false, rapatrier = false, declencheur: string | null = null) =>
    appeler<AbriPartiesFait>('parties_abriter', { id, accepte, rapatrier, declencheur }),
  /** Lot S3 : « Continuer sans abri » (après la seconde confirmation) — la prochaine `operation` sur ce jeu, dans les
   * 10 minutes et une seule fois, se fait sans mettre ses parties à l'abri. */
  partiesContinuerSansAbri: (id: number, operation: string) => appeler<void>('parties_continuer_sans_abri', { id, operation }),
  /** Lot S3 : l'abri des parties d'AVANT `operation`, fait à part juste avant elle (l'opération ne le refait pas). */
  partiesAbriAvant: (id: number, operation: string) => appeler<void>('parties_abri_avant', { id, operation }),
  /** Lot S3 : le dernier problème gardé de l'abri après une partie de ce jeu (la ligne de la fiche), ou `null`. */
  partiesAbriAutoEtat: (id: number) => appeler<BilanAbriAuto | null>('parties_abri_auto_etat', { id }),
  /** Lot S7 : les jeux dont un abri rattrapé attend « C'est toi qui as joué ? ». */
  partiesAConfirmer: () => appeler<number[]>('parties_a_confirmer'),
  /** Lot S7 : « Oui, c'est moi » (`oui`) ou « Non » pour les abris rattrapés d'un jeu, par `profil` (celui à qui la
   * question a été posée : refusé si ce n'est plus le profil ouvert) ; rend leur nombre. */
  partiesConfirmer: (id: number, oui: boolean, profil: string) => appeler<number>('parties_confirmer', { id, oui, profil }),
  /** Les abris de parties d'un jeu PC, pour le profil ouvert. */
  partiesAbris: (id: number) => appeler<AbriVu[]>('parties_abris', { id }),
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
  /** Lot M1 : les mods posés par Vortex dans un jeu du PC (`null` : pas installé ici). */
  modsVortexEtat: (id: number) => appeler<EtatVortex | null>('mods_vortex_etat', { id }),
  /** Lot M1 : retire les mods de Vortex (rangés dans le jeu, jamais effacés ; parties à l'abri avant). */
  modsVortexRetirer: (id: number, suivi: number) => appeler<BilanRetraitVortex>('mods_vortex_retirer', { id, suivi }),
  modsVortexReprendre: (id: number, suivi: number) => appeler<BilanRetraitVortex>('mods_vortex_reprendre', { id, suivi }),
  /** Lot M1 : remet UN mod retiré (`source` tel que l'état l'a donné). */
  modsVortexRemettre: (id: number, source: string, suivi: number) => appeler<BilanRemiseVortex>('mods_vortex_remettre', { id, source, suivi }),
  modsVortexToutRemettre: (id: number, suivi: number) => appeler<BilanRemiseVortex>('mods_vortex_tout_remettre', { id, suivi }),
  /** 0.227.1 : range de nouveau UN mod remis dans le jeu (`source` tel que l'état l'a donné dans `remis`). */
  modsVortexRetirerMod: (id: number, source: string, suivi: number) => appeler<BilanRetraitModVortex>('mods_vortex_retirer_mod', { id, source, suivi }),
  /** 0.227.1 : range de nouveau TOUS les mods remis (sans le dépôt de Vortex, plus de retrait global possible). */
  modsVortexToutRanger: (id: number, suivi: number) => appeler<BilanRetraitModVortex>('mods_vortex_tout_ranger', { id, suivi }),
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
  /** Switch (✅ Seb 17/11) : allume ou éteint un contenu (`cle` : « maj » ou l'identifiant d'un DLC) pour toute la
   * maison. Rend vrai si un contenu utilisé vient d'être éteint (parties mises à l'abri avant la prochaine partie). */
  contenusSwitchChoisir: (idJeu: number, cle: string, actif: boolean) => appeler<boolean>('contenus_switch_choisir', { idJeu, cle, actif }),
  /** Contrat 1.47 : les noms français des DLC et MAJ Switch (en tâche de fond ; hors ligne, le cache du profil). */
  contenusSwitchNoms: (id: number) => appeler<import('$lib/contenus/contenus').NomsContenus>('contenus_switch_noms', { id }),
  /** Lot 7 (✅ Seb 17/11) : ce que Firehouse n'a pas parmi les mises à jour et DLC Switch de ce jeu (à l'ouverture du
   * panneau 📦, en tâche de fond) ; l'accord de la personne ; où en sont les envois (rien n'est demandé à Firehouse). */
  envoisSwitchExaminer: (id: number) => appeler<import('$lib/contenus/envois').ExamenEnvois>('envois_switch_examiner', { id }),
  /** `noms` et `total` : la liste que la personne a VUE (le cœur refuse, sorte « conflit », si elle a changé). */
  envoisSwitchAccepter: (id: number, noms: string[], total: number, auto: boolean) =>
    appeler<number>('envois_switch_accepter', { id, noms, total, auto }),
  /** « ✋ Arrêter l'envoi » : les envois pas finis de ce jeu sont retirés (reproposables) ; rend combien. */
  envoisSwitchAnnuler: (id: number) => appeler<number>('envois_switch_annuler', { id }),
  envoisSwitchSuivi: (id: number) => appeler<import('$lib/contenus/envois').SuiviEnvois>('envois_switch_suivi', { id }),
  /** Mods Switch (contrat 1.44, ✅ Seb 18/11) : la liste (servis + rangés, compatibilité avec la mise à jour choisie). */
  modsSwitchLister: (id: number) => appeler<ModsSwitch>('mods_switch_lister', { id }),
  /** Après l'accord chiffré (ou « mettre à jour tout seul » : `auto`, réglage du profil `modsSwitch.auto`) : la liste
   * VUE (`clesVues`, `total` en octets) ; le cœur refuse (sorte « conflit ») si elle a changé. Rend combien. */
  modsSwitchTelecharger: (id: number, clesVues: { cle: string; revision: number; taille: number; sha256: string }[], total: number, auto: boolean) =>
    appeler<number>('mods_switch_telecharger', { id, clesVues, total, auto }),
  modsSwitchSuivi: (id: number) => appeler<SuiviModsSwitch>('mods_switch_suivi', { id }),
  /** Allume / éteint un mod pour toute la maison ; `eteindre` : l'autre mod en conflit, éteint d'un coup (après la
   * confirmation de la personne). Refus lisible (sorte « refus ») si un conflit n'est pas réglé. Rend vrai si changé. */
  modsSwitchChoisir: (id: number, cle: string, allume: boolean, eteindre: string | null = null) =>
    appeler<boolean>('mods_switch_choisir', { id, cle, allume, eteindre }),
  modsSwitchRetirer: (id: number, cle: string) => appeler<void>('mods_switch_retirer', { id, cle }),
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
  capturesListe: (jeu?: number) => appeler<import('$lib/captures/captures').Capture[]>('captures_liste', { jeu: jeu ?? null }),
  captureSupprimer: (id: string) => appeler<void>('capture_supprimer', { id }),
  captureOuvrirDossier: (id: string) => appeler<void>('capture_ouvrir_dossier', { id }),
  captureFond: (id: string, actif: boolean) => appeler<void>('capture_fond', { id, actif }),
  emulateurDeplacer: (id: string, base: string) => appeler<{ ancien: string; nouveau: string; renomme: boolean }>('emulateur_deplacer', { id, base }),
  dossierRetirerSiVide: (chemin: string) => appeler<boolean>('dossier_retirer_si_vide', { chemin }),
  dossierContenu: (sorte: 'abris' | 'captures' | 'tout', chemin: string) => appeler<{ fichiers: number; octets: number }>('dossier_contenu', { sorte, chemin }),
  dossierDeplacerContenu: (sorte: 'abris' | 'captures', ancien: string, nouveau: string) =>
    appeler<{ deplaces: number; octets: number; laisses: string[] }>('dossier_deplacer_contenu', { sorte, ancien, nouveau }),
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
  // `choisi` d'abord : une capture que la personne a choisie comme fond (son profil, ✅ Seb 06/11).
  return `http://jaquette.localhost/${id}?visuel=choisi,fond,capture,ecran_titre&largeur=1000`;
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
