// Le plein écran au lancement d'un jeu PC (Seb, 16/10 : « le plus généraliste possible ») : ce que l'interface
// montre et propose. Le cœur fait le reste (`src-tauri/src/plein_ecran.rs`).

/** Les touches plein écran proposées (les mêmes que le cœur, `plein_ecran::TOUCHES`). */
export const TOUCHES_PLEIN_ECRAN = ['F1', 'F2', 'F3', 'F4', 'F5', 'F6', 'F7', 'F8', 'F9', 'F10', 'F11', 'F12', 'Alt+Entrée'];

/** Le libellé d'un réglage de plein écran (`fenetre`, `touche:F4`, rien). */
export function libellePleinEcran(r: string | undefined): string {
  if (r === 'fenetre') return 'fenêtre agrandie à tout l’écran';
  const t = r?.startsWith('touche:') ? r.slice('touche:'.length) : null;
  if (t && TOUCHES_PLEIN_ECRAN.includes(t)) return `Frogtend appuie sur ${t}`;
  return 'non (le jeu s’ouvre comme il veut)';
}
