// L'assistant jeux (lot 7) : ce que Frogtend accepte de faire parmi les actions qu'il propose (brief § 4 : Frogtend
// reste juge ; il montre chaque action, ne l'exécute qu'après un oui, refuse ce qu'il ne sait pas faire sans risque).

/** Une action proposée par l'assistant (contrat : liste fermée de types). */
export interface ActionProposee {
  type: 'installer' | 'lancer' | 'configurer' | 'cheat' | 'mod' | 'telecharger' | 'ouvrir_url' | 'demander_jeu' | string;
  titre: string;
  details?: Record<string, unknown>;
  risque?: 'aucun' | 'faible' | 'modifie la configuration' | 'modifie le jeu' | string;
}

/** Un message de la conversation (rôles du contrat). */
export interface Message {
  role: 'user' | 'assistant';
  content: string;
}

/** Où en est le jeu de la conversation sur ce PC. */
export interface ContexteJeu {
  id: number | null;
  /** Dans la ludothèque de ce PC (téléchargé) ? */
  surPc: boolean;
  installe: boolean;
}

export type Verdict = { faisable: true; bouton: string } | { faisable: false; raison: string };

/** Ce que Frogtend fera d'une action. */
export function juger(a: ActionProposee, j: ContexteJeu): Verdict {
  switch (a.type) {
    case 'lancer':
      if (j.id === null) return { faisable: false, raison: 'Ouvre d’abord la conversation depuis la fiche du jeu.' };
      if (!j.surPc) return { faisable: false, raison: 'Ce jeu n’est pas encore dans ta ludothèque.' };
      if (!j.installe) return { faisable: true, bouton: '📦 Installer puis jouer' };
      return { faisable: true, bouton: '▶ Lancer' };
    case 'installer':
      if (j.id === null || !j.surPc) return { faisable: false, raison: 'Mets d’abord le jeu dans ta ludothèque (Catalogue).' };
      if (j.installe) return { faisable: false, raison: 'Le jeu est déjà installé.' };
      return { faisable: true, bouton: '📦 Installer' };
    case 'telecharger':
      if (j.id === null) return { faisable: false, raison: 'Aucun jeu précis.' };
      if (j.surPc) return { faisable: false, raison: 'Le jeu est déjà dans ta ludothèque.' };
      return { faisable: true, bouton: '📚 Voir la fiche pour l’ajouter' };
    case 'demander_jeu':
      return { faisable: true, bouton: '🔎 Demander un jeu' };
    case 'ouvrir_url': {
      const u = adresse(a);
      return u ? { faisable: true, bouton: '🌐 Ouvrir la page' } : { faisable: false, raison: 'Adresse absente ou non sûre.' };
    }
    case 'configurer':
    case 'cheat':
    case 'mod':
      return { faisable: false, raison: 'Frogtend ne sait pas encore le faire sans risque (prévu avec les mods et triches).' };
    default:
      return { faisable: false, raison: 'Action inconnue de Frogtend.' };
  }
}

/** L'adresse d'une action « ouvrir_url », seulement en http(s). */
export function adresse(a: ActionProposee): string | null {
  const u = a.details?.url;
  if (typeof u !== 'string') return null;
  try {
    const p = new URL(u);
    return p.protocol === 'https:' || p.protocol === 'http:' ? p.href : null;
  } catch {
    return null;
  }
}

/** Les détails d'une action, lisibles (« émulateur : DOSBox · fichier : dune.exe »). */
export function resumeDetails(a: ActionProposee): string {
  return Object.entries(a.details ?? {})
    .filter(([, v]) => v !== null && v !== undefined && v !== '')
    .map(([k, v]) => `${k.replace(/_/g, ' ')} : ${typeof v === 'object' ? JSON.stringify(v) : String(v)}`)
    .join(' · ');
}

/** L'historique envoyé : les 40 derniers messages (Firehouse lit les 20 derniers échanges), 4 000 caractères chacun. */
export function historiquePourEnvoi(h: Message[]): Message[] {
  return h.slice(-40).map((m) => ({ role: m.role, content: m.content.slice(0, 4000) }));
}
