import { describe, expect, it } from 'vitest';
import { miseAJourDisponible, plusRecente } from './versions';

describe('miseAJourDisponible', () => {
  it('résolu par Firehouse : toute version servie différente (contrat 1.40 : daily → v0.1.0-rc3)', () => {
    expect(miseAJourDisponible('eglantine', 'v0.1.0-rc3', 'daily-20261107')).toBe(true);
    expect(miseAJourDisponible('lichii', 'build-117', 'build-117')).toBe(false);
    expect(miseAJourDisponible('eglantine', undefined, 'daily-20261107')).toBe(false);
    expect(miseAJourDisponible('eglantine', 'v0.1.0-rc3', null)).toBe(false);
  });
  it('les autres : seulement plus récente', () => {
    expect(miseAJourDisponible('pcsx2', 'v2.3.1', 'v2.3.0')).toBe(true);
    expect(miseAJourDisponible('pcsx2', 'v2.2.0', 'v2.3.0')).toBe(false);
  });
});

describe('plusRecente', () => {
  it('compare les nombres, pas le texte', () => {
    expect(plusRecente('1.22.10', '1.22.2')).toBe(true);
    expect(plusRecente('1.22.2', '1.22.10')).toBe(false);
    expect(plusRecente('v2.3', '2.2.9')).toBe(true);
  });

  it('une même version n’est pas plus récente', () => {
    expect(plusRecente('1.20.0', '1.20')).toBe(false);
  });

  it('marche avec les dates de DuckStation', () => {
    expect(plusRecente('2026-09-28', '2026-08-30')).toBe(true);
  });
});
