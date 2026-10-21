import { describe, expect, it } from 'vitest';
import gta from '../../../src-tauri/fixtures/simule/triche-steam-gta5.json';
import xcom from '../../../src-tauri/fixtures/simule/triche-steam-xcom2.json';
import { decisionCheatEngine, decisionModSteam } from './cheatengine';

describe('Un mod dans un jeu Steam (lot B3, même règle)', () => {
  it('refuse un anti-triche, avertit un jeu en ligne ou inconnu, laisse un jeu sans risque', () => {
    expect(decisionModSteam({ anti_triche: gta.en_ligne.anticheat, en_ligne: 'true', inconnu: false }).sorte).toBe('refus');
    expect(decisionModSteam({ anti_triche: null, en_ligne: xcom.en_ligne.online, inconnu: false }).sorte).toBe('avertir');
    expect(decisionModSteam({ anti_triche: null, en_ligne: null, inconnu: true }).sorte).toBe('avertir');
    expect(decisionModSteam({ anti_triche: null, en_ligne: null, inconnu: false }).sorte).toBe('libre');
  });
});

describe('Cheat Engine sur un jeu (Seb, 21/10)', () => {
  it('refuse un jeu avec un anti-triche, sans moyen de passer outre', () => {
    // Le cœur reprend `en_ligne.anticheat` des relevés réels (GTA V : BattlEye).
    const d = decisionCheatEngine({ anti_triche: gta.en_ligne.anticheat, en_ligne: gta.en_ligne.online, inconnu: false });
    expect(d.sorte).toBe('refus');
    expect(d.sorte === 'refus' && d.message).toContain('BattlEye');
  });
  it('avertit pour un jeu en ligne ou inconnu', () => {
    const d = decisionCheatEngine({ anti_triche: null, en_ligne: xcom.en_ligne.online, inconnu: false });
    expect(d.sorte).toBe('avertir');
    expect(d.sorte === 'avertir' && d.message).toContain('se joue aussi en ligne');
    const i = decisionCheatEngine({ anti_triche: null, en_ligne: null, inconnu: true });
    expect(i.sorte === 'avertir' && i.message).toContain('ne sait pas');
  });
  it('laisse faire un jeu sans risque connu', () => {
    expect(decisionCheatEngine({ anti_triche: null, en_ligne: null, inconnu: false }).sorte).toBe('libre');
  });
});
