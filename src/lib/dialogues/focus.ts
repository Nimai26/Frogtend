// Retenir le focus dans une fenêtre et le rendre à l'élément d'origine en la fermant (charte § 4 :
// « à faire mieux que Firehouse », indispensable à la manette).

const FOCALISABLES =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

export function focalisables(conteneur: HTMLElement): HTMLElement[] {
  return Array.from(conteneur.querySelectorAll<HTMLElement>(FOCALISABLES));
}

/** Le premier champ de saisie, sinon le bouton de validation, sinon le premier élément focalisable. */
export function premierFocus(conteneur: HTMLElement): HTMLElement | null {
  return (
    conteneur.querySelector<HTMLElement>('input:not([disabled]), textarea:not([disabled]), select:not([disabled])') ??
    conteneur.querySelector<HTMLElement>('[data-valider]') ??
    focalisables(conteneur)[0] ??
    null
  );
}

/** Action Svelte : `use:retenirFocus`. */
export function retenirFocus(noeud: HTMLElement) {
  const origine = document.activeElement as HTMLElement | null;
  const cible = premierFocus(noeud);
  if (cible) {
    cible.focus();
    if (cible instanceof HTMLInputElement || cible instanceof HTMLTextAreaElement) cible.select();
  }

  function surTouche(e: KeyboardEvent) {
    if (e.key !== 'Tab') return;
    const liste = focalisables(noeud);
    if (liste.length === 0) {
      e.preventDefault();
      return;
    }
    const premier = liste[0];
    const dernier = liste[liste.length - 1];
    if (e.shiftKey && document.activeElement === premier) {
      e.preventDefault();
      dernier.focus();
    } else if (!e.shiftKey && document.activeElement === dernier) {
      e.preventDefault();
      premier.focus();
    }
  }

  noeud.addEventListener('keydown', surTouche);
  return {
    destroy() {
      noeud.removeEventListener('keydown', surTouche);
      if (origine && origine.isConnected) origine.focus();
    },
  };
}
