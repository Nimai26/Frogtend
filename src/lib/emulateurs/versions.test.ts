import { describe, expect, it } from 'vitest';
import { plusRecente } from './versions';

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
