// Les outils maison (charte § 4) : Confirmer, Demander, Choisir, Informer, Toast — et « Appuie sur une touche ».
// Aucune fenêtre native : ces fonctions ouvrent des fenêtres propres à l'application et rendent une promesse.

import { deduireTypeToast, dureeToast, type TypeToast } from './messages';

export interface OptionChoix<T = string> {
  valeur: T;
  libelle: string;
  detail?: string;
}

interface Base {
  id: number;
  titre: string;
  message?: string;
}

export type Dialogue =
  | (Base & { sorte: 'confirmer'; libelleValider: string; danger: boolean; fermer: (r: boolean) => void })
  | (Base & {
      sorte: 'demander';
      valeur: string;
      multiligne: boolean;
      /** Saisie masquée (code PIN, jeton) : rien ne s'affiche en clair. */
      masque: boolean;
      libelleValider: string;
      fermer: (r: string | null) => void;
    })
  | (Base & { sorte: 'choisir'; options: OptionChoix<unknown>[]; fermer: (r: unknown | null) => void })
  | (Base & {
      sorte: 'cocher';
      options: OptionChoix<unknown>[];
      /** Cochées à l'ouverture (même ordre que `options`). */
      coches: boolean[];
      /** Une ligne qui résume ce qui est coché (le poids total…), recalculée à chaque case. */
      resume?: (valeurs: unknown[]) => string;
      /** Pourquoi ce qui est coché ne peut pas être validé (dit dans la fenêtre, validation impossible), ou `null`. */
      refus?: (valeurs: unknown[]) => string | null;
      libelleValider: string;
      /** « Tout cocher » et « Tout décocher » au-dessus de la liste (une longue liste : les mods d'un onglet…). */
      toutCocher?: boolean;
      fermer: (r: unknown[] | null) => void;
    })
  | (Base & { sorte: 'informer'; fermer: () => void })
  | (Base & {
      sorte: 'conflit';
      /** « Sur ce PC », puis « Chez Firehouse » : des lignes déjà mises en mots (jamais un chemin). */
      colonnes: [ColonneConflit, ColonneConflit];
      fermer: (r: ChoixConflit) => void;
    })
  | (Base & {
      sorte: 'cestToi';
      fermer: (r: ReponseCestToi) => void;
    })
  | (Base & {
      sorte: 'touche';
      /** Pourquoi une touche n'est pas possible (`null` : acceptée). */
      verifier: (t: ToucheLue) => string | null;
      fermer: (r: ToucheLue | 'liste' | 'rien' | null) => void;
    });

/** Une colonne de la fenêtre de conflit (lot S5). `recente` : le badge « ⭐ la plus récente », seulement quand c'est sûr. */
export interface ColonneConflit {
  titre: string;
  lignes: string[];
  recente: boolean;
}

/** Le choix d'un conflit : garder la partie de ce PC, reprendre celle de Firehouse, ou décider plus tard. */
export type ChoixConflit = 'ici' | 'firehouse' | 'plus_tard';

/** Lot S7 : la réponse à « C'est toi qui as joué ? ». */
export type ReponseCestToi = 'oui' | 'non' | 'plus_tard';

/** Une touche appuyée : sa place (`code`) et ce qu'elle affiche (`key`). */
export interface ToucheLue {
  code: string;
  key: string;
}

export interface Toast {
  id: number;
  message: string;
  type: TypeToast;
  /** Un travail en cours (lot 0.82.3) : l'étape (seule annoncée), les chiffres et le pourcentage ; jamais fermé au
   * clic. */
  progres?: { etape: string; chiffres: string; pourcent: number };
}

let compteur = 0;

/** La pile des fenêtres ouvertes ; seule celle du dessus (la dernière) écoute le clavier. */
export const pile = $state<{ dialogues: Dialogue[]; toasts: Toast[] }>({ dialogues: [], toasts: [] });

function ouvrir<R>(fabrique: (id: number, fermer: (r: R) => void) => Dialogue): Promise<R> {
  return new Promise<R>((resoudre) => {
    const id = ++compteur;
    const fermer = (r: R) => {
      pile.dialogues = pile.dialogues.filter((d) => d.id !== id);
      resoudre(r);
    };
    pile.dialogues.push(fabrique(id, fermer));
  });
}

/** Avant une action irréversible. Rend `true` si la personne confirme. */
export function confirmer(
  titre: string,
  options: { message?: string; libelleValider?: string; danger?: boolean } = {},
): Promise<boolean> {
  const danger = options.danger ?? false;
  return ouvrir<boolean>((id, fermer) => ({
    id,
    sorte: 'confirmer',
    titre,
    message: options.message,
    danger,
    libelleValider: options.libelleValider ?? (danger ? 'Supprimer' : 'Confirmer'),
    fermer,
  }));
}

/** Saisir un texte. Rend le texte, ou `null` si la personne annule. */
export function demander(
  titre: string,
  options: { message?: string; valeur?: string; multiligne?: boolean; masque?: boolean; libelleValider?: string } = {},
): Promise<string | null> {
  return ouvrir<string | null>((id, fermer) => ({
    id,
    sorte: 'demander',
    titre,
    message: options.message,
    valeur: options.valeur ?? '',
    multiligne: options.multiligne ?? false,
    masque: options.masque ?? false,
    libelleValider: options.libelleValider ?? 'Valider',
    fermer,
  }));
}

/** Un choix dans une liste. Rend la valeur choisie, ou `null`. */
export function choisir<T>(titre: string, options: OptionChoix<T>[], message?: string): Promise<T | null> {
  return ouvrir<T | null>((id, fermer) => ({
    id,
    sorte: 'choisir',
    titre,
    message,
    options: options as OptionChoix<unknown>[],
    fermer: fermer as (r: unknown | null) => void,
  }));
}

/** Plusieurs choix dans une liste (cases à cocher : A ou Entrée coche / décoche). Rend les valeurs cochées (au moins
 * une), ou `null`. */
export function cocher<T>(
  titre: string,
  options: OptionChoix<T>[],
  o: { message?: string; coches?: boolean[]; resume?: (valeurs: T[]) => string; refus?: (valeurs: T[]) => string | null; libelleValider?: string; toutCocher?: boolean } = {},
): Promise<T[] | null> {
  return ouvrir<T[] | null>((id, fermer) => ({
    id,
    sorte: 'cocher',
    titre,
    message: o.message,
    options: options as OptionChoix<unknown>[],
    coches: options.map((_, i) => o.coches?.[i] ?? false),
    resume: o.resume as ((valeurs: unknown[]) => string) | undefined,
    refus: o.refus as ((valeurs: unknown[]) => string | null) | undefined,
    libelleValider: o.libelleValider ?? 'Valider',
    toutCocher: o.toutCocher,
    fermer: fermer as (r: unknown[] | null) => void,
  }));
}

/** Un message qu'il faut avoir lu : un seul bouton « OK ». */
export function informer(titre: string, message?: string): Promise<void> {
  return ouvrir<void>((id, fermer) => ({ id, sorte: 'informer', titre, message, fermer: () => fermer() }));
}

/** Lot S5 : une partie de ce PC différente de celle de Firehouse. Deux colonnes, trois boutons ; « Garder celle de ce
 * PC » a le focus d'office (deux appuis sur A ne remplacent jamais une partie) ; Échap ou B : « Décider plus tard ». */
export function deciderConflit(titre: string, o: { message?: string; ici: ColonneConflit; firehouse: ColonneConflit }): Promise<ChoixConflit> {
  return ouvrir<ChoixConflit>((id, fermer) => ({ id, sorte: 'conflit', titre, message: o.message, colonnes: [o.ici, o.firehouse], fermer }));
}

/** Lot S7 : « 🛟 C'est toi qui as joué à … ? » — « Oui, c'est moi », « Non », « Décider plus tard ». « Décider plus
 * tard » a le focus d'office (deux appuis sur A ne décident jamais pour quelqu'un d'autre) ; Échap ou B : « Décider plus
 * tard ». */
export function demanderSiCestToi(titre: string, o: { message?: string } = {}): Promise<ReponseCestToi> {
  return ouvrir<ReponseCestToi>((id, fermer) => ({ id, sorte: 'cestToi', titre, message: o.message, fermer }));
}

/** « Appuie sur la touche voulue » : TOUTES les touches sont prises (Échap et Tab compris) ; la touche est rendue à son
 * relâchement (jamais une touche encore tenue qui validerait la fenêtre suivante). Rend la touche, `'liste'` (choisir
 * dans la liste), `'rien'` (aucune touche) ou `null` (annulé, à la souris). */
export function attendreTouche(titre: string, options: { message?: string; verifier: (t: ToucheLue) => string | null }): Promise<ToucheLue | 'liste' | 'rien' | null> {
  return ouvrir<ToucheLue | 'liste' | 'rien' | null>((id, fermer) => ({ id, sorte: 'touche', titre, message: options.message, verifier: options.verifier, fermer }));
}

/** Un compte rendu bref, qui ne bloque rien. Le type se déduit du message s'il n'est pas donné. */
export function toast(message: string, type?: TypeToast, options: { jusquaFermeture?: boolean } = {}): number {
  const id = ++compteur;
  const t = type ?? deduireTypeToast(message);
  pile.toasts.push({ id, message, type: t });
  // Une attente (« Installation… ») reste affichée jusqu'à `fermerToast` : jamais un écran vide pendant le travail.
  if (!options.jusquaFermeture) setTimeout(() => fermerToast(id), dureeToast(t));
  return id;
}

/** Un message de travail en cours, avec une barre : reste jusqu'à `fermerToast`, ne se ferme pas au clic. */
export function toastProgres(message: string): number {
  const id = ++compteur;
  pile.toasts.push({ id, message, type: 'neutre', progres: { etape: '', chiffres: '', pourcent: 0 } });
  return id;
}

/** Met à jour un message de progression (sans effet s'il est déjà fermé). */
export function majProgres(id: number, progres: { etape: string; chiffres: string; pourcent: number }) {
  const t = pile.toasts.find((x) => x.id === id);
  if (t) t.progres = progres;
}

export function fermerToast(id: number) {
  pile.toasts = pile.toasts.filter((t) => t.id !== id);
}

/** Annule la fenêtre du dessus (Échap, bouton B de la manette, clic sur le voile). */
export function annulerDessus() {
  const d = pile.dialogues.at(-1);
  if (!d) return false;
  if (d.sorte === 'confirmer') d.fermer(false);
  else if (d.sorte === 'informer') d.fermer();
  else if (d.sorte === 'conflit') d.fermer('plus_tard');
  else if (d.sorte === 'cestToi') d.fermer('plus_tard');
  else d.fermer(null);
  return true;
}
