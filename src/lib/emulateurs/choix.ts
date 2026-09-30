// Plusieurs émulateurs (ou cœurs RetroArch) par système, un par défaut pour le système, un par défaut possible par
// jeu, et un choix ponctuel au lancement (Seb, 30/09). Même règle que le cœur (src-tauri/src/choix_emulateur.rs).

/** Un émulateur réglé pour un système. Le fichier du jeu est ajouté à la fin de `ligne`. */
export interface EmulateurRegle {
  cle: string;
  nom: string;
  programme: string;
  ligne: string;
}

export interface EmulateursSysteme {
  liste: EmulateurRegle[];
  /** La clé de celui par défaut. */
  defaut: string;
}

/** Lit un réglage de système, y compris l'ancienne forme `{programme, ligne, nom}` (avant 0.10). */
export function normaliser(v: unknown): EmulateursSysteme {
  const o = (v ?? {}) as Record<string, unknown>;
  if (Array.isArray(o.liste)) {
    const liste = (o.liste as Partial<EmulateurRegle>[])
      .filter((e) => e && typeof e.programme === 'string' && e.programme !== '')
      .map((e, i) => ({ cle: e.cle || String(i + 1), nom: e.nom ?? '', programme: e.programme!, ligne: e.ligne ?? '' }));
    const defaut = liste.some((e) => e.cle === o.defaut) ? String(o.defaut) : (liste[0]?.cle ?? '');
    return { liste, defaut };
  }
  if (typeof o.programme === 'string' && o.programme !== '') {
    return { liste: [{ cle: '1', nom: String(o.nom ?? ''), programme: o.programme, ligne: String(o.ligne ?? '') }], defaut: '1' };
  }
  return { liste: [], defaut: '' };
}

/** Normalise tous les systèmes d'un réglage `emulateurs` (au chargement). */
export function normaliserTous(v: unknown): Record<string, EmulateursSysteme> {
  const r: Record<string, EmulateursSysteme> = {};
  for (const [s, e] of Object.entries((v ?? {}) as Record<string, unknown>)) {
    const n = normaliser(e);
    if (n.liste.length) r[s] = n;
  }
  return r;
}

/** Ajoute un émulateur à un système (s'il n'y est pas déjà : même programme et même ligne). Le premier devient le
 * défaut. Rend le système et la clé de l'émulateur. */
export function ajouter(sys: EmulateursSysteme | undefined, e: Omit<EmulateurRegle, 'cle'>): { sys: EmulateursSysteme; cle: string } {
  const s = normaliser(sys);
  const deja = s.liste.find((x) => x.programme.toLowerCase() === e.programme.toLowerCase() && x.ligne.trim() === e.ligne.trim());
  if (deja) return { sys: s, cle: deja.cle };
  let n = s.liste.length + 1;
  while (s.liste.some((x) => x.cle === String(n))) n++;
  const cle = String(n);
  const liste = [...s.liste, { ...e, cle }];
  return { sys: { liste, defaut: s.defaut || cle }, cle };
}

/** Retire un émulateur d'un système (rien n'est désinstallé). Le défaut passe au premier restant. */
export function retirer(sys: EmulateursSysteme, cle: string): EmulateursSysteme {
  const liste = sys.liste.filter((e) => e.cle !== cle);
  return { liste, defaut: liste.some((e) => e.cle === sys.defaut) ? sys.defaut : (liste[0]?.cle ?? '') };
}

/** L'émulateur d'un jeu : son défaut à lui, sinon celui du système. */
export function pourLeJeu(
  emulateurs: Record<string, unknown>,
  emulateursJeux: Record<string, string>,
  plateforme: string,
  jeu: number,
): EmulateurRegle | null {
  const s = normaliser(emulateurs[plateforme]);
  for (const cle of [emulateursJeux[String(jeu)], s.defaut]) {
    const e = s.liste.find((x) => x.cle === cle);
    if (e) return e;
  }
  return s.liste[0] ?? null;
}
