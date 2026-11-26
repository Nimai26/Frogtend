// 🧩 La page des mods d'un jeu Switch (✅ Seb 26/11) : son adresse (bureau ou canapé), le retour du focus sur le bouton
// « Ouvrir les mods » (modèle `taodbox/retour.ts` : gardé dans le module, consommé une seule fois), et le nombre de mods
// vu en dernier pour « 🧩 Mods (N) » de « Gérer le jeu » — toujours PAR PROFIL (jamais les mods d'un autre profil).
import { api } from '$lib/api';
import { etat } from '$lib/etat.svelte';

/** `/mods?jeu=…&titre=…` au bureau, `/taodbox/mods?…` au canapé (le mode Taodbox reste actif). */
export function adressePageMods(id: number, titre: string, canape: boolean): string {
  return `${canape ? '/taodbox' : ''}/mods?jeu=${id}&titre=${encodeURIComponent(titre)}`;
}

let retour: number | null = null;

/** Retenu avant d'ouvrir la page : au retour, le bouton « Ouvrir les mods » de CE jeu reprend le focus. */
export function retenirRetourMods(id: number) {
  retour = id;
}

/** Rendu une seule fois : `true` si le retour vise ce jeu. */
export function reprendreRetourMods(id: number): boolean {
  const r = retour;
  retour = null;
  return r === id;
}

const nombres = new Map<string, number>();
const cleDe = (profil: string | null, jeu: number) => `${profil ?? ''}:${jeu}`;
const profilOuvert = () => etat.profilOuvert?.id ?? null;

/** Noté à chaque lecture de la liste (par la bande ou la page), pour le profil qui l'a lue. */
export function noterNombreDeMods(profil: string | null, jeu: number, n: number) {
  nombres.set(cleDe(profil, jeu), n);
}

/** Le nombre vu en dernier par CE profil, tout de suite (`undefined` : pas encore lu). */
export function nombreDeModsVu(jeu: number): number | undefined {
  return nombres.get(cleDe(profilOuvert(), jeu));
}

/** Le nombre de mods d'un jeu : celui vu en dernier par CE profil, sinon la liste lue (et notée pour la fois suivante).
 * `null` : pas connu (Firehouse injoignable, profil changé pendant la lecture) ; 0 : aucun mod. */
export async function nombreDeModsSwitch(jeu: number): Promise<number | null> {
  const profil = profilOuvert();
  const vu = nombres.get(cleDe(profil, jeu));
  if (vu !== undefined) return vu;
  const l = await Promise.resolve()
    .then(() => api.modsSwitchLister(jeu))
    .catch(() => null);
  // Le profil a changé pendant la lecture : rien n'est gardé ni dit.
  if (!l || profilOuvert() !== profil || l.etat === 'injoignable') return null;
  const n = l.etat === 'rien' ? 0 : l.mods.length;
  noterNombreDeMods(profil, jeu, n);
  return n;
}

/** L'entrée « 🧩 Mods (N) » de « Gérer le jeu » d'un jeu Switch : le menu n'attend JAMAIS. Le nombre déjà vu par ce
 * profil, sinon « 🧩 Mods » tout de suite et la liste lue en arrière-plan (pour la fois suivante). `null` : le jeu n'a
 * aucun mod (rien à proposer). Une valeur à part : `mods` sert aux jeux PC. */
export function optionModsSwitch(jeu: number): { valeur: 'modsSwitch'; libelle: string; detail: string } | null {
  const n = nombreDeModsVu(jeu);
  if (n === undefined) void nombreDeModsSwitch(jeu);
  if (n === 0) return null;
  return {
    valeur: 'modsSwitch',
    libelle: n === undefined ? '🧩 Mods' : `🧩 Mods (${n.toLocaleString('fr-FR')})`,
    detail: 'chercher, allumer, éteindre, télécharger — pour toute la maison',
  };
}
