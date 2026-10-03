// Règles de texte des fenêtres maison (charte § 4) : type d'un toast, motif d'un refus.

export type TypeToast = 'ok' | 'alerte' | 'erreur' | 'neutre';

export const DUREE_TOAST_MS = 3200;
export const DUREE_TOAST_REFUS_MS = 6000;

const sansAccents = (t: string) => t.normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase();

/**
 * Déduit le type d'un toast de son message, quand il n'est pas donné.
 * Un ✅ l'emporte sur tout ; puis les emoji, puis les expressions d'échec, puis le premier mot.
 */
export function deduireTypeToast(message: string): TypeToast {
  if (message.includes('✅')) return 'ok';
  if (/[👍🎉]/u.test(message)) return 'ok';
  if (/[⛔❌🚫]/u.test(message)) return 'erreur';
  if (message.includes('⚠')) return 'alerte';

  const t = sansAccents(message);
  if (/\brefuse|a echoue|impossible de/.test(t)) return 'erreur';

  const premier = t.replace(/^[^a-z]+/, '');
  if (/^(refus|echec|erreur|introuvable)/.test(premier)) return 'erreur';
  if (/^(attention|deja|aucun resultat)/.test(premier)) return 'alerte';
  if (/^(enregistre|ajoute|cree)/.test(premier)) return 'ok';
  return 'neutre';
}

export const dureeToast = (type: TypeToast) => (type === 'erreur' ? DUREE_TOAST_REFUS_MS : DUREE_TOAST_MS);

/** Retire tout chemin d'API d'un message : on ne le montre jamais. */
const sansCheminApi = (t: string) => t.replace(/\s*\/api\/\S*/g, '').replace(/\s{2,}/g, ' ').trim();

/**
 * Extrait le motif lisible d'une erreur : texte, `message`, `erreur`/`error`, `detail`, ou liste de validation.
 * Jamais « [object Object] ».
 */
export function motifDuRefus(erreur: unknown): string {
  const motif = extraire(erreur, 0);
  return motif ? sansCheminApi(motif) : 'motif inconnu';
}

function extraire(e: unknown, profondeur: number): string {
  if (profondeur > 4 || e === null || e === undefined) return '';
  if (typeof e === 'string') return e.trim();
  if (typeof e === 'number' || typeof e === 'boolean') return String(e);
  if (Array.isArray(e)) return e.map((x) => extraire(x, profondeur + 1)).filter(Boolean).join(' ; ');
  if (typeof e === 'object') {
    const o = e as Record<string, unknown>;
    // `motif` : la forme des erreurs du cœur de Frogtend ({sorte, motif}).
    for (const cle of ['motif', 'message', 'erreur', 'error', 'detail', 'msg']) {
      const m = extraire(o[cle], profondeur + 1);
      if (m) return m;
    }
  }
  return '';
}

/** Le texte d'un refus : « ⛔ Refusé : motif ». */
export const texteRefus = (erreur: unknown) => `⛔ Refusé : ${motifDuRefus(erreur)}`;
