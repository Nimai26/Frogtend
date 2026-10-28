import { describe, expect, it } from 'vitest';
import { EFFETS_IMAGE, libelleEffet } from './effetsImage';

describe('les effets d’image', () => {
  it('les mêmes identifiants que le cœur, chacun avec un nom et une phrase', () => {
    expect(Object.keys(EFFETS_IMAGE).sort()).toEqual(['crt', 'crt-riche', 'lisse', 'net', 'portable']);
    for (const e of Object.values(EFFETS_IMAGE)) expect(e.libelle && e.detail).toBeTruthy();
  });
  it('aucun réglage ou « aucun » : aucun effet', () => {
    expect(libelleEffet(undefined)).toBe('aucun effet');
    expect(libelleEffet('aucun')).toBe('aucun effet');
    expect(libelleEffet('crt')).toBe('Télé d’époque (simple)');
  });
});
