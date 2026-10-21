// Brancher Cheat Engine sur un jeu (Seb, 21/10) : jamais avec un anti-triche connu ; un jeu en ligne, ou un jeu de
// boutique dont on ne sait rien, seulement après un avertissement et le oui de la personne. Le cœur revérifie.
import { api, type RisqueTriche } from '$lib/api';
import { confirmer, informer } from '$lib/dialogues/fenetres.svelte';

export type DecisionCheatEngine =
  | { sorte: 'libre' }
  | { sorte: 'refus'; titre: string; message: string }
  | { sorte: 'avertir'; titre: string; message: string };

/** Ce qu'on dit à la personne d'après le risque rendu par le cœur. */
export function decisionCheatEngine(r: RisqueTriche): DecisionCheatEngine {
  if (r.anti_triche)
    return {
      sorte: 'refus',
      titre: '🧰 Cheat Engine n’est pas permis pour ce jeu',
      message: `Ce jeu est protégé par un anti-triche (${r.anti_triche}). S’il voit Cheat Engine, ton compte peut être bloqué pour de bon. Frogtend ne le lancera donc pas sur ce jeu.`,
    };
  if (r.en_ligne || r.inconnu)
    return {
      sorte: 'avertir',
      titre: '⚠ Risque de bannissement',
      message: `${r.en_ligne ? 'Ce jeu se joue aussi en ligne.' : 'Frogtend ne sait pas si ce jeu se joue en ligne ou s’il a un anti-triche.'} Tricher dans un jeu en ligne peut faire bannir ton compte. Si tu continues, joue seul, hors ligne.`,
    };
  return { sorte: 'libre' };
}

/** Le même risque pour un MOD dans un jeu Steam (lot B3 : même règle que Cheat Engine). */
export function decisionModSteam(r: RisqueTriche): DecisionCheatEngine {
  if (r.anti_triche)
    return {
      sorte: 'refus',
      titre: '🧩 Pas de mod pour ce jeu',
      message: `Ce jeu est protégé par un anti-triche (${r.anti_triche}). Un mod peut faire bloquer ton compte pour de bon : Frogtend n’en installera pas.`,
    };
  if (r.en_ligne || r.inconnu)
    return {
      sorte: 'avertir',
      titre: '⚠ Risque de bannissement',
      message: `${r.en_ligne ? 'Ce jeu se joue aussi en ligne.' : 'Frogtend ne sait pas si ce jeu se joue en ligne ou s’il a un anti-triche.'} Un mod dans un jeu en ligne peut faire bannir ton compte. Si tu continues, joue seul, hors ligne, tant que le mod est là.`,
    };
  return { sorte: 'libre' };
}

/** Avant un mod dans un jeu Steam : `null` = on n'installe pas ; sinon l'accord à passer au cœur. */
export async function accordModSteam(id: number): Promise<boolean | null> {
  const r = await api.cheatengineRisque(id).catch(() => ({ anti_triche: null, en_ligne: null, inconnu: true }));
  const d = decisionModSteam(r);
  if (d.sorte === 'refus') {
    await informer(d.titre, d.message);
    return null;
  }
  if (d.sorte === 'avertir') return (await confirmer(d.titre, { message: d.message, libelleValider: 'Continuer quand même', danger: true })) ? true : null;
  return false;
}

/** Avant de brancher Cheat Engine sur le jeu `id` : `null` = on ne lance pas ; sinon l'accord à passer au cœur. Si le
 * risque n'a pas pu être lu, c'est « inconnu » : l'avertissement est posé (le cœur revérifie un anti-triche). */
export async function accordCheatEngine(id: number): Promise<boolean | null> {
  let r: RisqueTriche;
  try {
    r = await api.cheatengineRisque(id);
  } catch {
    r = { anti_triche: null, en_ligne: null, inconnu: true };
  }
  const d = decisionCheatEngine(r);
  if (d.sorte === 'refus') {
    await informer(d.titre, d.message);
    return null;
  }
  if (d.sorte === 'avertir') {
    return (await confirmer(d.titre, { message: d.message, libelleValider: 'Lancer quand même', danger: true })) ? true : null;
  }
  return false;
}
