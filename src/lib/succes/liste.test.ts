import { describe, expect, it } from 'vitest';
import { compte, libelleRarete, trier, type SuccesDetail } from './liste';

const s = (id: string, obtenu: boolean, quand: number | null, rarete: number | null): SuccesDetail => ({
  id,
  nom: id,
  description: '',
  en_anglais: true,
  obtenu,
  quand,
  rarete,
  icone: null,
  icone_grise: null,
  secret: false,
});

describe('Succès en détail (lot C)', () => {
  it('trie : obtenus d’abord, les plus récents en tête, puis du plus courant au plus rare', () => {
    const l = [s('rare', false, null, 2), s('vieux', true, 100, 50), s('courant', false, null, 70), s('recent', true, 200, 10), s('inconnu', false, null, null)];
    expect(trier(l).map((x) => x.id)).toEqual(['recent', 'vieux', 'courant', 'rare', 'inconnu']);
    expect(compte(l)).toEqual([2, 5]);
  });
  it('dit la rareté en mots', () => {
    // Relevé réel (Galaxy, 21/10) : 16,49 % ; 3,81 % ; 0,82 %.
    expect(libelleRarete(16.49)).toBe('16 % des joueurs : peu courant');
    expect(libelleRarete(3.81)).toBe('4 % des joueurs : très rare');
    expect(libelleRarete(0.82)).toBe('0,8 % des joueurs : très rare');
    expect(libelleRarete(80)).toBe('80 % des joueurs : courant');
    expect(libelleRarete(null)).toBeNull();
  });
});
