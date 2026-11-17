// « ▶ Ouvrir l'émulateur seul » (✅ Seb 17/11) : les émulateurs que Frogtend sait ouvrir SANS jeu, avec le profil de
// la personne. La même table que le cœur (src-tauri/src/emulateur_seul.rs, `PRIS_EN_CHARGE`) : reconnus au nom de leur
// programme, comme au lancement d'un jeu (le début du nom de fichier). Et les règles du bouton, sans état (testées).

import type { EmulateurRegle } from './choix';

/** Les émulateurs pris en charge : le début de leur nom de programme (catalogue du cœur) et leur nom affiché. */
export const PROGRAMMES: { debut: string; nom: string }[] = [
  { debut: 'ryujinx.exe', nom: 'Ryubing' },
  { debut: 'eden.exe', nom: 'Eden' },
  { debut: 'eglantine.exe', nom: 'Eglantine' },
  { debut: 'lichii.exe', nom: 'Lichii' },
  { debut: 'duckstation-qt', nom: 'DuckStation' },
  { debut: 'pcsx2-qt', nom: 'PCSX2' },
  { debut: 'dolphin.exe', nom: 'Dolphin' },
];

/** Les noms des émulateurs pris en charge, pour l'aide (« Ryubing, Eden, … et Dolphin »). */
export function nomsPrisEnCharge(): string {
  const n = PROGRAMMES.map((p) => p.nom);
  return `${n.slice(0, -1).join(', ')} et ${n[n.length - 1]}`;
}

/** Cet émulateur (son programme) s'ouvre-t-il seul ? */
export function ouvrableSeul(programme: string | undefined | null): boolean {
  const nom = (programme ?? '').split(/[\\/]/).pop()?.toLowerCase() ?? '';
  return nom !== '' && PROGRAMMES.some((p) => nom.startsWith(p.debut));
}

/** Parmi les émulateurs réglés d'un système, ceux qui s'ouvrent seuls. */
export function ouvrablesSeuls(liste: EmulateurRegle[]): EmulateurRegle[] {
  return liste.filter((e) => ouvrableSeul(e.programme));
}

/** L'état de l'émulateur ouvert seul : son nom (`ouvert`), le jeu dont le bouton l'ouvre (`ouverture`). */
export interface EtatSeul {
  ouvert: string | null;
  ouverture: number | null;
}

/** Une partie, un lancement, ou un émulateur ouvert seul (ou qui s'ouvre) : aucun autre lancement. */
export function lancementImpossibleSelon(partie: { enJeu: number | null; lancement: number | null }, seul: EtatSeul): boolean {
  return partie.enJeu !== null || partie.lancement !== null || seul.ouvert !== null || seul.ouverture !== null;
}

/** Le bouton est-il montré ? Un émulateur ouvrable au moins, et jamais pour un jeu PC lancé sans émulateur. */
export function boutonSeulMontre(jeuPcSansEmulateur: boolean, ouvrables: number): boolean {
  return !jeuPcSansEmulateur && ouvrables >= 1;
}

/** Le libellé du bouton du jeu `id` (« … » quand il y a un choix). */
export function libelleSeul(id: number, seul: EtatSeul, ouvrables: number): string {
  if (seul.ouverture === id) return 'Lancement…';
  if (seul.ouvert !== null) return `🕹 ${seul.ouvert} ouvert…`;
  return `▶ Ouvrir l’émulateur seul${ouvrables > 1 ? '…' : ''}`;
}

/** Pourquoi ▶ Jouer attend (affiché sous les boutons et en info-bulle), ou `null`. */
export function raisonDAttente(seul: EtatSeul): string | null {
  return seul.ouvert !== null ? `🕹 ${seul.ouvert} est ouvert : ferme-le pour jouer.` : null;
}

/** Le refus de ▶ Jouer pendant qu'un émulateur est ouvert seul (ou s'ouvre), ou `null`. */
export function refusDeJouer(seul: EtatSeul): string | null {
  if (seul.ouvert === null && seul.ouverture === null) return null;
  return `🕹 ${seul.ouvert ?? 'Un émulateur'} est ouvert : ferme-le d’abord, puis lance le jeu.`;
}
