import { describe, expect, it } from 'vitest';
import { libelleValeur, REGLAGES_IMAGE, reglagesMontres } from './imagePs3';

describe('image et fluidité d’un jeu PS3', () => {
  it('les valeurs sont celles que le cœur accepte (liste fermée de RPCS3)', () => {
    // Le cœur (emulateurs_profils::RPCS3_IMAGE) n'accepte que ces valeurs : les deux listes doivent rester égales.
    const attendu: Record<string, string[]> = {
      'Shader Mode': ['Async Recompiler with Shader Interpreter', 'Async Recompiler (multi-threaded)', 'Shader Interpreter only'],
      'Resolution Scale': ['100', '150', '200', '300'],
      'Output Scaling Mode': ['Bilinear', 'FidelityFX Super Resolution'],
      'FidelityFX CAS Sharpening Intensity': ['0', '25', '50', '75', '100'],
      'Anisotropic Filter Override': ['0', '2', '4', '8', '16'],
      MSAA: ['Auto', 'Disabled'],
    };
    for (const r of REGLAGES_IMAGE) expect(r.valeurs.map((v) => v.valeur)).toEqual(attendu[r.cle]);
  });

  it('des libellés sans jargon ; une valeur faite à la main dans RPCS3 reste lisible', () => {
    expect(libelleValeur('Shader Mode', 'Async Recompiler with Shader Interpreter')).toBe('Sans saccades (recommandé)');
    expect(libelleValeur('Output Scaling Mode', 'FidelityFX Super Resolution')).toBe('Plus net');
    expect(libelleValeur('Resolution Scale', '125')).toBe('125');
    for (const r of REGLAGES_IMAGE) expect(r.titre).not.toMatch(/FSR|MSAA|anisotrope/i);
  });

  it('la netteté de l’agrandissement seulement avec l’agrandissement « Plus net »', () => {
    expect(reglagesMontres(new Map([['Output Scaling Mode', 'Bilinear']])).map((r) => r.cle)).not.toContain('FidelityFX CAS Sharpening Intensity');
    expect(reglagesMontres(new Map([['Output Scaling Mode', 'FidelityFX Super Resolution']])).map((r) => r.cle)).toContain('FidelityFX CAS Sharpening Intensity');
  });
});
