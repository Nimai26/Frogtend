import { beforeEach, describe, expect, it } from 'vitest';
import { nomAffiche, nomsCourts, oublierNomsCourts, retenirNomsCourts } from './nomsCourts.svelte';

describe('noms courts des plateformes', () => {
  beforeEach(() => oublierNomsCourts());

  it('montre le nom court, sinon le nom LaunchBox', () => {
    retenirNomsCourts([{ nom: 'Sega Genesis', nom_affiche: 'Mega Drive' }, { nom: 'Sony Playstation' }]);
    expect(nomAffiche('Sega Genesis')).toBe('Mega Drive');
    expect(nomAffiche('Sony Playstation')).toBe('Sony Playstation');
    expect(nomAffiche(null)).toBe('');
  });

  it('deux plateformes au même nom court gardent leur nom LaunchBox ; une troisième garde le sien', () => {
    retenirNomsCourts([
      { nom: 'Sega Genesis', nom_affiche: 'Mega Drive' },
      { nom: 'Sega Mega Drive', nom_affiche: 'Mega Drive' },
      { nom: 'Nintendo 64', nom_affiche: 'N64' },
    ]);
    expect(nomAffiche('Sega Genesis')).toBe('Sega Genesis');
    expect(nomAffiche('Sega Mega Drive')).toBe('Sega Mega Drive');
    expect(nomAffiche('Nintendo 64')).toBe('N64');
  });

  it("un nom court égal au nom LaunchBox d'une autre plateforme, même dans l'autre liste, est écarté", () => {
    retenirNomsCourts([{ nom: 'Nintendo Switch', nom_affiche: 'Switch' }], [{ nom: 'Switch' }]);
    expect(nomAffiche('Nintendo Switch')).toBe('Nintendo Switch');
    expect(nomAffiche('Switch')).toBe('Switch');
  });

  it('un nom court retiré par Firehouse disparaît au rechargement', () => {
    retenirNomsCourts([{ nom: 'Sega Genesis', nom_affiche: 'Mega Drive' }]);
    retenirNomsCourts([{ nom: 'Sega Genesis' }]);
    expect(nomAffiche('Sega Genesis')).toBe('Sega Genesis');
    expect(nomsCourts).toEqual({});
  });

  it('oubliés à la sortie du profil', () => {
    retenirNomsCourts([{ nom: 'Sega Genesis', nom_affiche: 'Mega Drive' }]);
    oublierNomsCourts();
    expect(nomAffiche('Sega Genesis')).toBe('Sega Genesis');
  });
});
