import { describe, expect, it } from 'vitest';
import { nomFiche, propositions, texteAnnonce, texteMotif, texteStatut, type JeuRapproche } from './rapprochement';

describe('reconnaître les jeux importés (contrat 1.21)', () => {
  it('dit les statuts et les motifs simplement, et un motif inconnu tel quel', () => {
    expect(texteStatut('exact')).toBe('reconnu à coup sûr');
    expect(texteStatut('probable')).toBe('sans doute ce jeu');
    expect(texteMotif('autre version du romset MAME')).toContain('ta version MAME');
    expect(texteMotif('aucune correspondance')).toBe('Firehouse ne connaît pas ce fichier');
    expect(texteMotif('nouveau motif')).toBe('nouveau motif');
    expect(texteMotif(null)).toBe('');
  });

  it('rassemble les fiches proposées sans doublon et les nomme', () => {
    const j: JeuRapproche = {
      jeu: -1,
      titre: 'x',
      plateforme: 'p',
      decision: 'a_voir',
      media_id: null,
      versions: [
        { version: 'a', statut: 'probable', media_id: 10, telechargement_id: null, correspondance: 'nom', motif: null, candidats: [{ media_id: 11, telechargement_id: null }], titre_fiche: 'Zelda', annee_fiche: 1987 },
        { version: 'b', statut: 'probable', media_id: 10, telechargement_id: null, correspondance: 'nom', motif: null, candidats: [] },
      ],
    };
    const p = propositions(j);
    expect(p.map((c) => c.media_id)).toEqual([10, 11]);
    expect(nomFiche(p[0])).toBe('Zelda (1987)');
    expect(nomFiche(p[1])).toBe('un jeu que ta ludothèque n’a pas encore reçu (n° 11)');
  });

  it('annonce ce qui sera lu, sans parler de chemins', () => {
    const t = texteAnnonce({ fichiers: 90, a_lire: 60, octets: 230 * 1024 ** 2, repertoires: 30, reseau: 0, octets_reseau: 0, connus: 0, absents: 0 });
    expect(t).toContain('90 fichier(s)');
    expect(t).toContain('230 Mo');
    expect(t).toContain('jamais l’endroit');
    expect(t).toContain('Durée : environ');
    expect(t).not.toMatch(/NaN|Infinity/);
  });
});
