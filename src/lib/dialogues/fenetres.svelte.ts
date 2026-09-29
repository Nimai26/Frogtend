// Les cinq outils maison (charte § 4) : Confirmer, Demander, Choisir, Informer, Toast.
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
  | (Base & { sorte: 'informer'; fermer: () => void });

export interface Toast {
  id: number;
  message: string;
  type: TypeToast;
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

/** Un message qu'il faut avoir lu : un seul bouton « OK ». */
export function informer(titre: string, message?: string): Promise<void> {
  return ouvrir<void>((id, fermer) => ({ id, sorte: 'informer', titre, message, fermer: () => fermer() }));
}

/** Un compte rendu bref, qui ne bloque rien. Le type se déduit du message s'il n'est pas donné. */
export function toast(message: string, type?: TypeToast): number {
  const id = ++compteur;
  const t = type ?? deduireTypeToast(message);
  pile.toasts.push({ id, message, type: t });
  setTimeout(() => fermerToast(id), dureeToast(t));
  return id;
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
  else d.fermer(null);
  return true;
}
