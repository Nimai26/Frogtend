import { describe, expect, it } from 'vitest';
import { anneeDe, creditImage, creditTexte, imageEntete, lienSur, sousTitre } from './systemes';
import type { FicheSysteme } from '$lib/api';

// Le relevé RÉEL du 24/10 (contrat 1.28), 3DO.
const troisDo: FicheSysteme = {
  nom: '3DO Interactive Multiplayer',
  nom_affiche: '3DO',
  date_sortie: '1993-10-04',
  fabricant: 'Panasonic Corporation, Sanyo Electric Co., Ltd., GoldStar',
  description_fr: 'La 3DO Interactive Multiplayer…',
  description_licence: 'CC BY-SA 4.0 (Wikipédia) / CC0 (Wikidata)',
  description_source: 'https://fr.wikipedia.org/wiki/3DO_Interactive_Multiplayer',
  images: {
    banniere: { id: 1, empreinte: 'f3c6', licence: null, auteur: null, url_source: null, source: 'launchbox_venkman' },
    fond: { id: 201, empreinte: 'b6a4', licence: 'CC BY-SA 3.0', auteur: 'Evan-Amos', url_source: 'https://commons.wikimedia.org/wiki/File:3DO-FZ1-Console-Set.png' },
    logo: { id: 2, empreinte: 'e47e', licence: null },
    appareil: null,
  },
};

describe('fiches des systèmes (contrat 1.28)', () => {
  it('l’en-tête prend la bannière, sinon le logo, sinon rien', () => {
    expect(imageEntete(troisDo)?.sorte).toBe('banniere');
    expect(imageEntete({ ...troisDo, images: { logo: troisDo.images!.logo } })?.sorte).toBe('logo');
    expect(imageEntete({ nom: 'x', images: { icone: { id: 3, empreinte: 'a' } } })).toBeNull();
    expect(imageEntete(null)).toBeNull();
    // La bannière n'a pas pu s'afficher : le logo.
    expect(imageEntete(troisDo, ['banniere'])?.sorte).toBe('logo');
  });

  it('fabricant et année', () => {
    expect(sousTitre(troisDo)).toBe('Panasonic Corporation, Sanyo Electric Co., Ltd., GoldStar · 1993');
    expect(anneeDe('1993-10')).toBe('1993');
    expect(anneeDe(null)).toBeNull();
    expect(sousTitre({ nom: 'x' })).toBe('');
  });

  it('les crédits : auteur et licence quand il y en a une, rien pour LaunchBox', () => {
    expect(creditImage(troisDo.images!.fond)).toBe('Image : Evan-Amos · CC BY-SA 3.0');
    expect(creditImage(troisDo.images!.banniere)).toBeNull();
    expect(creditImage({ id: 1, empreinte: 'e', licence: 'CC BY 4.0', auteur: null })).toBe('Image : CC BY 4.0');
    expect(creditTexte(troisDo)).toBe('Texte : CC BY-SA 4.0 (Wikipédia) / CC0 (Wikidata)');
    expect(creditTexte({ nom: 'x', description_fr: 'y' })).toBeNull();
  });

  it('seul un lien web s’ouvre', () => {
    expect(lienSur(troisDo.description_source)).toBe(troisDo.description_source);
    expect(lienSur('file:///C:/x')).toBeNull();
    expect(lienSur(null)).toBeNull();
  });
});
