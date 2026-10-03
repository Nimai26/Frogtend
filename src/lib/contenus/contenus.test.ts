import { describe, expect, it } from 'vitest';
import { libelleGenre, nomCourt, resume } from './contenus';

describe('les contenus additionnels', () => {
  it('se présentent simplement', () => {
    expect(nomCourt('Asuras Wrath - Episode 11.5 (Europe) (EnJaFrDeEsIt) (DLC)', "Asura's Wrath")).toBe('Episode 11.5');
    expect(nomCourt('God of War - Ascension - Achilles (Europe) (Avatar)', 'God of War - Ascension')).toBe('Ascension - Achilles');
    expect(nomCourt('Thème libre', 'Autre jeu')).toBe('Thème libre');
    expect(libelleGenre('avatar')).toBe('Avatar');
    expect(libelleGenre('inconnu')).toBe('Contenu');
    const c = { id: 'a', nom: 'x', genre: 'dlc', taille: 1, licence: false, installe: false };
    expect(resume([c, { ...c, id: 'b', installe: true }])).toBe('2 disponible(s), 1 installé(s)');
    expect(resume([c])).toBe('1 disponible(s)');
  });
});
