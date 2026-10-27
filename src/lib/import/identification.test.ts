import { describe, expect, it } from 'vitest';
import { libelleCandidat, texteAnnonceBase } from './identification';

describe('chercher les jeux importés dans la base', () => {
  it('nomme un candidat par son titre, son année et sa console', () => {
    const court = (p: string) => (p === 'Nintendo Entertainment System' ? 'NES' : p);
    expect(libelleCandidat({ launchbox_id: 1501, titre: 'Abadox: The Deadly Inner War', plateforme: 'Nintendo Entertainment System', annee: 1989, fiche_id: 7742 }, court)).toBe(
      'Abadox: The Deadly Inner War (1989) — NES',
    );
    expect(libelleCandidat({ launchbox_id: 1, titre: 'Sans date', plateforme: 'Windows', annee: null, fiche_id: null }, court)).toBe('Sans date — Windows');
  });

  it('annonce ce qui part avant de l’envoyer', () => {
    expect(texteAnnonceBase(0, 0)).toContain('Aucun');
    expect(texteAnnonceBase(0, 4)).toContain('déjà été cherchés');
    expect(texteAnnonceBase(12, 0)).toContain('12 jeu(x)');
    expect(texteAnnonceBase(12, 0)).toContain('seuls leurs noms');
  });
});
