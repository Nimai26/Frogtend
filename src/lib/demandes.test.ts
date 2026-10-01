import { describe, expect, it } from 'vitest';
import type { ResultatRecherche } from '$lib/api';
import { etatDemande } from './demandes';

const jeu = (deja: ResultatRecherche['deja']): ResultatRecherche => ({
  launchbox_id: 161,
  titre: 'The Legend of Zelda: Ocarina of Time',
  titre_fr: '',
  plateforme: 'Nintendo 64',
  annee: 1998,
  genres: [],
  developpeur: '',
  jaquette: '',
  deja,
});

describe('etatDemande', () => {
  it('un jeu inconnu de Firehouse se demande', () => {
    expect(etatDemande(jeu(null), [])).toEqual({ texte: '', demandable: true });
  });
  it('un jeu possédé ou déjà cherché ne se redemande pas', () => {
    expect(etatDemande(jeu({ id: 110, statut: 'possede' }), []).demandable).toBe(false);
    expect(etatDemande(jeu({ id: 111, statut: 'recherche' }), []).texte).toContain('Firehouse le cherche');
  });
  it('une demande faite pendant la visite est retenue', () => {
    expect(etatDemande(jeu(null), [161])).toEqual({ texte: '📨 Demandé', demandable: false });
  });
});
