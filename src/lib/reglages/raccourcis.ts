// Les raccourcis de Frogtend en jeu (✅ Seb 11/11 et 15/11) : les choix proposés et leurs conflits. Le cœur revérifie
// tout (`osd_manette::raccourcis_regles`, `commandes::touches_etats`) : ceci évite seulement de proposer un choix qui ne
// marcherait pas.

/** Les combinaisons de la manette qui ouvrent le menu (tenues 1 s). */
export const MENU_MANETTE = ['Select + R1', 'Select + L1', 'L3 + R3', 'Select + Start'] as const;

/** Les raccourcis de sauvegarde et de chargement à la manette (Vue + un bouton ou une gâchette), ou aucun. */
export const ETATS_MANETTE = ['Select + LT', 'Select + RT', 'Select + L1', 'Select + R1', 'Select + Start', 'aucun'] as const;

/** Les touches du clavier qui ouvrent le menu (jamais Ctrl + Maj + M : c'est le son coupé de RPCS3, envoyé par Frogtend). */
export const MENU_CLAVIER = ['Pause', 'ScrollLock'] as const;

/** Les touches de sauvegarde et de chargement (jamais F1 à F16 : prises par les émulateurs ou envoyées par Frogtend). */
export const ETATS_CLAVIER = ['Ctrl+Shift+F5', 'Ctrl+Shift+F9', 'Ctrl+Shift+F6', 'Ctrl+Shift+F7', 'Ctrl+Shift+F8', 'Ctrl+Shift+S', 'Ctrl+Shift+L', 'aucune'] as const;

/** Les réglages d'origine (✅ Seb 15/11). */
export const ORIGINE = {
  manette: 'Select + R1',
  sauverManette: 'Select + LT',
  chargerManette: 'Select + RT',
  touche: 'Pause',
  sauverClavier: 'Ctrl+Shift+F5',
  chargerClavier: 'Ctrl+Shift+F9',
};

export interface Reglages {
  manette?: string;
  touche?: string;
  raccourcisEtats?: boolean;
  sauverManette?: string;
  chargerManette?: string;
  sauverClavier?: string;
  chargerClavier?: string;
}

/** Le raccourci de la manette en place : le réglé, sinon celui d'origine (ou aucun si l'ancien interrupteur était coupé). */
export function etatManette(r: Reglages, quoi: 'sauver' | 'charger'): string {
  const v = quoi === 'sauver' ? r.sauverManette : r.chargerManette;
  if (v) return v;
  if (r.raccourcisEtats === false) return 'aucun';
  return quoi === 'sauver' ? ORIGINE.sauverManette : ORIGINE.chargerManette;
}

/** La touche du clavier en place pour sauver / charger. */
export function etatClavier(r: Reglages, quoi: 'sauver' | 'charger'): string {
  const v = quoi === 'sauver' ? r.sauverClavier : r.chargerClavier;
  return v || (quoi === 'sauver' ? ORIGINE.sauverClavier : ORIGINE.chargerClavier);
}

/** Ce qui occupe déjà ce choix (`null` : libre). `pour` : le réglage qu'on change. */
export function prise(r: Reglages, pour: 'manette' | 'sauverManette' | 'chargerManette' | 'touche' | 'sauverClavier' | 'chargerClavier', valeur: string): string | null {
  if (valeur === 'aucun' || valeur === 'aucune') return null;
  const menu = r.manette || ORIGINE.manette;
  const [sm, cm] = [etatManette(r, 'sauver'), etatManette(r, 'charger')];
  const [sc, cc] = [etatClavier(r, 'sauver'), etatClavier(r, 'charger')];
  const touche = r.touche || ORIGINE.touche;
  switch (pour) {
    case 'manette':
      return valeur === sm ? 'sauver' : valeur === cm ? 'charger' : null;
    case 'sauverManette':
      return valeur === menu ? 'ouvrir le menu' : valeur === cm ? 'charger' : null;
    case 'chargerManette':
      return valeur === menu ? 'ouvrir le menu' : valeur === sm ? 'sauver' : null;
    case 'touche':
      return valeur === sc ? 'sauver' : valeur === cc ? 'charger' : null;
    case 'sauverClavier':
      return valeur === touche ? 'ouvrir le menu' : valeur === cc ? 'charger' : null;
    case 'chargerClavier':
      return valeur === touche ? 'ouvrir le menu' : valeur === sc ? 'sauver' : null;
  }
}

/** Le nom d'une combinaison de manette pour la personne, Xbox et PlayStation (dans les Options, la manette est inconnue). */
export function nomManette(c: string): string {
  if (c === 'aucun') return 'aucun';
  const x: Record<string, string> = { Select: 'Vue (⧉)', Start: 'Menu (≡)', R1: 'RB', L1: 'LB', LT: 'LT', RT: 'RT', L3: 'L3', R3: 'R3' };
  const p: Record<string, string> = { Select: 'Créer', Start: 'Options', R1: 'R1', L1: 'L1', LT: 'L2', RT: 'R2', L3: 'L3', R3: 'R3' };
  const n = (m: Record<string, string>) => c.split('+').map((b) => m[b.trim()] ?? b.trim()).join(' + ');
  const xb = n(x);
  const ps = n(p);
  return xb === ps ? xb : `${xb} (PlayStation : ${ps})`;
}

/** Le nom d'une touche pour la personne (« Ctrl+Shift+F5 » → « Ctrl + Maj + F5 »). */
export function nomClavier(t: string): string {
  if (t === 'aucune') return 'aucune';
  const noms: Record<string, string> = { Pause: 'Pause/Attn', ScrollLock: 'Arrêt défil', Shift: 'Maj' };
  return t.split('+').map((x) => noms[x.trim()] ?? x.trim()).join(' + ');
}
