import { describe, expect, it } from 'vitest';
import { adresse, historiquePourEnvoi, juger, resumeDetails, type ActionProposee } from './actions';

const a = (type: string, details?: Record<string, unknown>): ActionProposee => ({ type, titre: 'x', details });
const installe = { id: 110, surPc: true, installe: true };
const telecharge = { id: 110, surPc: true, installe: false };
const absent = { id: 110, surPc: false, installe: false };

describe('juger les actions de l’assistant', () => {
  it('lancer et installer suivent l’état du jeu', () => {
    expect(juger(a('lancer'), installe)).toEqual({ faisable: true, bouton: '▶ Lancer' });
    expect(juger(a('lancer'), telecharge)).toMatchObject({ faisable: true });
    expect(juger(a('lancer'), absent).faisable).toBe(false);
    expect(juger(a('installer'), installe).faisable).toBe(false);
    expect(juger(a('installer'), telecharge).faisable).toBe(true);
    expect(juger(a('telecharger'), absent).faisable).toBe(true);
    expect(juger(a('lancer'), { id: null, surPc: false, installe: false }).faisable).toBe(false);
  });

  it('ce qui touche à la configuration ou au jeu est refusé pour l’instant', () => {
    for (const t of ['configurer', 'cheat', 'mod', 'formater']) expect(juger(a(t), installe).faisable).toBe(false);
  });

  it('une adresse doit être http(s)', () => {
    expect(adresse(a('ouvrir_url', { url: 'https://www.pcgamingwiki.com/wiki/Dune' }))).toBe('https://www.pcgamingwiki.com/wiki/Dune');
    expect(adresse(a('ouvrir_url', { url: 'file:///C:/Windows/system32' }))).toBeNull();
    expect(adresse(a('ouvrir_url', { url: 'javascript:alert(1)' }))).toBeNull();
    expect(juger(a('ouvrir_url', {}), installe).faisable).toBe(false);
  });

  it('les détails se lisent, l’historique est borné', () => {
    expect(resumeDetails(a('lancer', { emulateur: 'DOSBox', fichier: 'dune.exe', vide: '' }))).toBe('emulateur : DOSBox · fichier : dune.exe');
    const h = Array.from({ length: 50 }, (_, i) => ({ role: 'user' as const, content: 'x'.repeat(i === 49 ? 5000 : 3) }));
    const e = historiquePourEnvoi(h);
    expect(e).toHaveLength(40);
    expect(e[39].content).toHaveLength(4000);
  });
});
