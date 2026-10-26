import { describe, expect, it } from 'vitest';
import { cochesCoherentes, libelleGenre, nomCourt, resume, type ContenuVu } from './contenus';

describe('les contenus additionnels', () => {
  it('se présentent simplement', () => {
    expect(nomCourt('Asuras Wrath - Episode 11.5 (Europe) (EnJaFrDeEsIt) (DLC)', "Asura's Wrath")).toBe('Episode 11.5');
    expect(nomCourt('God of War - Ascension - Achilles (Europe) (Avatar)', 'God of War - Ascension')).toBe('Ascension - Achilles');
    expect(nomCourt('Thème libre', 'Autre jeu')).toBe('Thème libre');
    expect(libelleGenre('avatar')).toBe('Avatar');
    expect(libelleGenre('licence')).toBe('Licence');
    expect(libelleGenre('inconnu')).toBe('Contenu');
    const c = { id: 'a', nom: 'x', genre: 'dlc', taille: 1, licence: false, installe: false };
    expect(resume([c, { ...c, id: 'b', installe: true }])).toBe('2 disponible(s), 1 installé(s)');
    expect(resume([c])).toBe('1 disponible(s)');
  });

  it('cochent les mises à jour dans l’ordre de la chaîne', () => {
    const maj = (v: string, installe = false): ContenuVu => ({ id: `A${v}`, nom: `Mise à jour ${v}`, genre: 'maj', taille: 1, licence: false, installe, version: v });
    const l = [maj('01.05', true), maj('01.06'), maj('01.07'), maj('01.08'), { id: 'dlc', nom: 'DLC', genre: 'dlc', taille: 1, licence: false, installe: false }];
    // Cocher 01.08 coche aussi 01.06 et 01.07 (01.05 est installée).
    expect(cochesCoherentes(l, ['A01.08'], 'A01.08').sort()).toEqual(['A01.06', 'A01.07', 'A01.08']);
    // Décocher 01.06 décoche les suivantes, jamais le DLC.
    expect(cochesCoherentes(l, ['A01.07', 'A01.08', 'dlc'], 'A01.06')).toEqual(['dlc']);
    // Un DLC ne change rien aux autres.
    expect(cochesCoherentes(l, ['dlc'], 'dlc')).toEqual(['dlc']);
  });
});
