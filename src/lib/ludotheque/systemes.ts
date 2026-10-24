// Les fiches des systèmes (contrat 1.28) : ce qu'on en montre, et leurs CRÉDITS. Des icônes libretro (CC BY 4.0), des
// photos Wikimedia Commons (CC BY / BY-SA) et des descriptions Wikipédia (CC BY-SA) exigent d'afficher l'auteur, la
// licence et la source LÀ où l'image ou le texte est montré. Une image sans licence (LaunchBox) n'a rien à créditer.

import type { FicheSysteme, ImageSysteme } from '$lib/api';

/** L'image d'en-tête d'un système : la bannière, sinon le logo (`sauf` : celles qui n'ont pas pu s'afficher). */
export function imageEntete(f: FicheSysteme | null | undefined, sauf: string[] = []): { sorte: 'banniere' | 'logo'; image: ImageSysteme } | null {
  for (const sorte of ['banniere', 'logo'] as const) {
    if (sauf.includes(sorte)) continue;
    const image = f?.images?.[sorte];
    if (image?.empreinte) return { sorte, image };
  }
  return null;
}

/** L'année d'une date de sortie (« 1993-10-04 », « 1993-10 », « 1993 ») ; `null` si illisible. */
export function anneeDe(date: string | null | undefined): string | null {
  return /^(\d{4})/.exec(date ?? '')?.[1] ?? null;
}

/** « Fabricant · Année », ce qui en est connu. */
export function sousTitre(f: FicheSysteme | null | undefined): string {
  return [f?.fabricant?.trim(), anneeDe(f?.date_sortie)].filter(Boolean).join(' · ');
}

/** Une adresse web qui s'ouvre (http ou https), sinon `null`. */
export function lienSur(url: string | null | undefined): string | null {
  return /^https?:\/\//i.test(url ?? '') ? url! : null;
}

/** Le crédit d'une image (« Image : Evan-Amos · CC BY-SA 3.0 »), ou `null` s'il n'y a rien à créditer. */
export function creditImage(i: ImageSysteme | null | undefined): string | null {
  if (!i?.licence) return null;
  return `Image : ${[i.auteur?.trim(), i.licence.trim()].filter(Boolean).join(' · ')}`;
}

/** Le crédit de la description (« Texte : CC BY-SA 4.0 (Wikipédia) »), ou `null`. */
export function creditTexte(f: FicheSysteme | null | undefined): string | null {
  if (!f?.description_fr || !f.description_licence) return null;
  return `Texte : ${f.description_licence.trim()}`;
}
