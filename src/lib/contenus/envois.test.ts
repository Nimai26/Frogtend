// Les textes de l'envoi à Firehouse (lot 7) : chiffrés, pour un public novice, jamais de chemin.
import { describe, expect, it, vi } from 'vitest';

vi.mock('$lib/api', () => ({ taille: (o: number) => `${o} o`, duree: (s: number) => `${s} s`, environDuree: (s: number) => `environ ${s} s` }));

import { dureeDite, envoisEnRoute, ligneEnvoi, ligneProgres, ligneProposition, titreAccord, type ExamenEnvois } from './envois';

const examen = (a: { nom: string; role: string; taille: number }[], duree: number | null = null): ExamenEnvois => ({
  etat: 'ok',
  motif: null,
  a_envoyer: a,
  total: a.reduce((n, c) => n + c.taille, 0),
  duree,
  plafonne: true,
  ecartes: [],
  deja_la: 0,
  suivi: { envois: [], en_cours: null, arret: null },
});

describe('les textes de l’envoi à Firehouse', () => {
  it('un débit pas encore mesuré se dit, sans inventer de durée', () => {
    expect(dureeDite(null)).toContain('durée inconnue');
    expect(titreAccord(examen([{ nom: 'a.nsp', role: 'maj', taille: 5 }]))).toBe(
      'Firehouse n’a pas 1 contenu (5 o, durée inconnue : elle sera mesurée au premier envoi) : les lui envoyer ?',
    );
  });

  it('rien à proposer : aucune ligne', () => {
    expect(ligneProposition(examen([]))).toBeNull();
    expect(ligneProposition({ ...examen([{ nom: 'a.nsp', role: 'maj', taille: 5 }]), etat: 'interdit' })).toBeNull();
    expect(ligneProposition(null)).toBeNull();
  });

  it('chaque état d’un envoi a sa phrase ; un envoi fini ne fait plus relire', () => {
    const e = { nom: 'a.nsp', role: 'dlc', taille: 10, recu: 5 };
    expect(ligneEnvoi({ ...e, etat: 'attente' })).toContain('En attente');
    expect(ligneEnvoi({ ...e, etat: 'envoi' })).toContain('50 %');
    expect(ligneEnvoi({ ...e, etat: 'arrete', motif: 'fichier trop gros : 32 Go au plus' })).toBe('⚠ Envoi arrêté : « a.nsp » : fichier trop gros : 32 Go au plus');
    expect(ligneEnvoi({ ...e, etat: 'deja_la', motif: 'ce fichier est déjà servi par Firehouse' })).toContain('déjà servi');
    expect(ligneProgres({ jeu: 1, nom: 'a.nsp', etape: 'verification', faits: 10, total: 10 })).toContain('Firehouse vérifie');
    expect(ligneProgres({ jeu: 1, nom: 'a.nsp', etape: 'empreinte', faits: 1, total: 4 })).toContain('25 %');
    expect(envoisEnRoute({ envois: [{ ...e, etat: 'range' }], en_cours: null, arret: null })).toBe(false);
    expect(envoisEnRoute({ envois: [{ ...e, etat: 'verification' }], en_cours: null, arret: null })).toBe(true);
    // Le travail s'est arrêté (Firehouse injoignable) : plus rien ne bouge avant la prochaine ouverture.
    expect(envoisEnRoute({ envois: [{ ...e, etat: 'attente' }], en_cours: null, arret: 'injoignable' })).toBe(false);
    expect(ligneEnvoi({ ...e, etat: 'attente' }, true)).toContain('reprendra à la prochaine ouverture');
    expect(ligneEnvoi({ ...e, etat: 'range' }, true)).toContain('rangé');
  });
});
