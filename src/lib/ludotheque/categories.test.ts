import { describe, expect, it } from 'vitest';
import { categorieDe, grouperParCategorie } from './categories';

describe('categorieDe', () => {
  it('classe les plateformes LaunchBox courantes', () => {
    expect(categorieDe('Arcade')).toBe('Arcade');
    expect(categorieDe('Nintendo Entertainment System')).toBe('Consoles');
    expect(categorieDe('Super Nintendo')).toBe('Consoles');
    expect(categorieDe('Sony Playstation')).toBe('Consoles');
    expect(categorieDe('Nintendo Game Boy Advance')).toBe('Consoles portables');
    expect(categorieDe('Sony PSP')).toBe('Consoles portables');
    expect(categorieDe('MS-DOS')).toBe('Ordinateurs');
    expect(categorieDe('Windows')).toBe('Ordinateurs');
    expect(categorieDe('Atari ST')).toBe('Ordinateurs');
  });

  it('range un nom inconnu dans « Autres » plutôt que de le perdre', () => {
    expect(categorieDe('Machine inventée')).toBe('Autres');
  });
});

describe('grouperParCategorie', () => {
  it('garde l’ordre Arcade, Consoles, Portables, Ordinateurs, Autres et trie chaque groupe', () => {
    const g = grouperParCategorie([
      { nom: 'Windows' },
      { nom: 'Super Nintendo' },
      { nom: 'Arcade' },
      { nom: 'MS-DOS' },
      { nom: 'Nintendo 64' },
    ]);
    expect(g.map((x) => x.categorie)).toEqual(['Arcade', 'Consoles', 'Ordinateurs']);
    expect(g[1].plateformes.map((p) => p.nom)).toEqual(['Nintendo 64', 'Super Nintendo']);
    expect(g[2].plateformes.map((p) => p.nom)).toEqual(['MS-DOS', 'Windows']);
  });
});
