import { describe, expect, it } from 'vitest';
import type { JeuResume } from '$lib/api';
import { estJeuDeBoutique, moyenDOuverture } from './jeu-boutique';

const jeu = (o: Partial<JeuResume>): JeuResume =>
  ({ id: -5, titre: 'X', annee: null, plateforme: 'Windows', genres: [], developpeur: null, editeur: null, statut: 'boutique', ...o }) as JeuResume;

describe('jeux de boutique', () => {
  it('se reconnaissent à leur id négatif', () => {
    expect(estJeuDeBoutique({ id: -3 })).toBe(true);
    expect(estJeuDeBoutique({ id: 110 })).toBe(false);
  });

  it('s’ouvrent par Steam ou par GOG Galaxy selon leur source', () => {
    expect(moyenDOuverture(jeu({ source: 'steam', cle_boutique: '620' }))).toEqual({ par: 'steam', appid: '620' });
    expect(moyenDOuverture(jeu({ source: 'galaxy', cle_boutique: 'epic_Fortnite' }))).toEqual({ par: 'galaxy', cle: 'epic_Fortnite' });
    expect(moyenDOuverture(jeu({ source: 'galaxy' }))).toBeNull();
  });
});
