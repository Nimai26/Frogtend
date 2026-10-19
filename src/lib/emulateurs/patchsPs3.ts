// Les retouches (patchs) d'un jeu PS3 (contrat 1.18) dites en français : la liste officielle de RPCS3 est en anglais,
// et Seb ne lit pas l'anglais. Les plus courantes sont traduites ici, sans rien promettre de plus que le patch (avis
// de l'expert interface, 19/10) ; les autres restent telles quelles, marquées « en anglais » (besoin 26 : des
// descriptions françaises servies par Firehouse).

/** Les descriptions les plus courantes de la liste officielle de RPCS3, sans jargon. */
const TRADUCTIONS: Record<string, string> = {
  'unlock fps': 'Débloquer les images par seconde',
  'disable motion blur': 'Sans flou de mouvement',
  'disable bloom': 'Sans halo lumineux',
  'disable depth of field': 'Sans flou d’arrière-plan',
  'disable ambient occlusion': 'Sans ombres de contact (plus léger)',
  'disable fxaa': 'Sans lissage FXAA (image plus nette)',
  'disable film grain': 'Sans grain de film',
  'disable vignette': 'Sans assombrissement des bords',
  'disable chromatic aberration': 'Sans franges de couleur',
  'disable dynamic resolution': 'Sans résolution qui varie',
  'skip intro videos': 'Passer les vidéos du début',
  'skip intro': 'Passer l’introduction',
  'boot to title screen': 'Démarrer directement à l’écran titre',
  'developer menu': 'Menu des développeurs (pour curieux)',
  'debug menu': 'Menu de débogage (pour curieux)',
  'aspect ratio': 'Format d’image élargi',
  'enable freecam': 'Caméra libre',
  'freecam': 'Caméra libre',
};

/** La description d'un patch en français si elle est connue, sinon telle quelle (et `anglais` vrai). */
export function descriptionFr(description: string): { texte: string; anglais: boolean } {
  const t = TRADUCTIONS[description.trim().toLowerCase()];
  return t ? { texte: t, anglais: false } : { texte: description, anglais: true };
}

/** Le détail court d'un patch dans la liste : jamais la note entière (elle est montrée avant d'activer). */
export function detailPatch(p: { notes: string }): string | undefined {
  return p.notes.trim() ? '⚠ une remarque de l’auteur, montrée avant d’activer' : undefined;
}

/** Un titre réduit à ses lettres et chiffres (« Asura's Wrath » = « Asuras Wrath »). */
function reduit(t: string): string {
  return t
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]/g, '');
}

/** Le titre de la liste qui est CE jeu : le même que celui de la fiche s'il y en a un, sinon le plus fréquent. Les
 * autres titres sont d'autres exécutables (démo, autre édition) : leurs patchs ne s'appliquent pas à ce jeu. */
export function titreDuJeu(patchs: { titre: string }[], titreFiche: string): string | null {
  const titres = [...new Set(patchs.map((p) => p.titre))];
  const meme = titres.find((t) => reduit(t) === reduit(titreFiche));
  if (meme) return meme;
  const n = new Map<string, number>();
  for (const p of patchs) n.set(p.titre, (n.get(p.titre) ?? 0) + 1);
  // À égalité : le titre le plus court (une démo ou une édition ajoute des mots : « … Preview »).
  return [...n.entries()].sort((a, b) => b[1] - a[1] || a[0].length - b[0].length)[0]?.[0] ?? null;
}
