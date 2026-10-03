// « Prêt à jouer » (décision de Seb, 03/10) : Frogtend est pour des gens qui veulent jouer, pas pour des connaisseurs
// de l'émulation. Quand une console n'a pas d'émulateur, Frogtend choisit seul celui que Firehouse recommande, et
// demande UN accord qui dit tout (quoi, combien, où, la manette, la première partie plus longue). Ici : les décisions,
// sans effet de bord (testées) ; l'enchaînement est dans `pret.svelte.ts`.

import type { EmulateurRecommande } from '$lib/api';

/** Un émulateur possible pour une console, tel que Frogtend le voit. */
export interface Possible {
  rec: EmulateurRecommande;
  /** L'identifiant que Frogtend connaît (il sait l'installer), sinon `null`. */
  id: string | null;
  nom: string;
  /** Déjà sur ce PC : son programme. */
  programme?: string;
  /** Firehouse donne sa source (contrat 14) : Frogtend peut l'installer même sans le connaître. */
  parFirehouse?: boolean;
}

/** Ce que Frogtend fera seul. */
export type Choix = Possible & { comment: 'deja' | 'frogtend' | 'firehouse' };

/** Le premier émulateur que Frogtend peut préparer seul : déjà là, sinon installable ; le recommandé d'abord. */
export function choisirSeul(possibles: Possible[]): Choix | null {
  const ordre = [...possibles].sort((a, b) => Number(b.rec.recommande) - Number(a.rec.recommande));
  for (const p of ordre) {
    if (p.programme) return { ...p, comment: 'deja' };
    if (p.id) return { ...p, comment: 'frogtend' };
    if (p.parFirehouse) return { ...p, comment: 'firehouse' };
  }
  return null;
}

/** Le dossier des émulateurs proposé d'office : « Emulateurs » à la racine du disque des jeux (sinon E:). */
export function dossierParDefaut(emplacementsJeux: string[]): string {
  const lecteur = /^([a-z]):/i.exec(emplacementsJeux[0] ?? '')?.[1]?.toUpperCase() ?? 'E';
  return `${lecteur}:\\Emulateurs`;
}

/** Les émulateurs dont Frogtend règle la manette (une manette Xbox standard, ou toute manette reconnue). */
export const MANETTE_REGLEE = ['retroarch', 'duckstation', 'pcsx2', 'dolphin', 'rpcs3'];

/** Ceux qui préparent longuement la première partie de chaque jeu (compilation, une seule fois). */
const PREMIERE_PARTIE_LONGUE = ['rpcs3', 'cemu', 'eden', 'xenia', 'yuzu', 'suyu', 'ryujinx'];

/** Le texte de l'accord unique. */
export function messagePret(o: {
  plateforme: string;
  jeux: number;
  choix: Choix;
  taille: string | null;
  dossier: string;
  dossierNouveau: boolean;
}): string {
  const { choix } = o;
  const pour = o.jeux > 1 ? `tes ${o.jeux} jeux ${o.plateforme}` : `ce jeu ${o.plateforme}`;
  const lignes = [
    choix.comment === 'deja'
      ? `Pour jouer à ${pour}, Frogtend utilise ${choix.nom}, déjà installé sur ce PC.`
      : `Pour jouer à ${pour}, Frogtend installe ${choix.nom}, l’émulateur recommandé${o.taille ? ` (${o.taille})` : ''}, depuis son site officiel.`,
  ];
  if (choix.comment !== 'deja') lignes.push(`Dossier : ${o.dossier}\\${choix.nom}${o.dossierNouveau ? ' (le dossier des émulateurs, modifiable dans ⚙ Options ▸ Émulateurs)' : ''}`);
  if (choix.id && MANETTE_REGLEE.includes(choix.id)) lignes.push('Ta manette (Xbox ou autre) est réglée pour toi.');
  if (choix.rec.bios) lignes.push(`Il lui faut aussi un fichier système (${choix.rec.bios}) : Firehouse le fournira ; en attendant, Frogtend te dira où le mettre.`);
  if (choix.id && PREMIERE_PARTIE_LONGUE.includes(choix.id)) {
    lignes.push('La première partie de chaque jeu demande quelques minutes de préparation, une seule fois ; ensuite, il démarre vite.');
  }
  return lignes.join('\n');
}
