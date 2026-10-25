import { describe, expect, it } from 'vitest';
import { reponse } from './racine';

describe('mod rangé dans un seul dossier', () => {
  it('le choix rendu au cœur', () => {
    expect(reponse('contenu')).toBe(true);
    expect(reponse('dossier')).toBe(false);
    expect(reponse(null)).toBeNull();
  });
});
