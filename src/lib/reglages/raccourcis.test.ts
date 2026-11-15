import { describe, expect, it } from 'vitest';
import { etatClavier, etatManette, nomClavier, nomManette, prise } from './raccourcis';

describe('les raccourcis en jeu', () => {
  it('d’origine, et l’ancien interrupteur coupé', () => {
    expect(etatManette({}, 'sauver')).toBe('Select + LT');
    expect(etatManette({}, 'charger')).toBe('Select + RT');
    expect(etatManette({ raccourcisEtats: false }, 'sauver')).toBe('aucun');
    expect(etatManette({ raccourcisEtats: false, sauverManette: 'Select + L1' }, 'sauver')).toBe('Select + L1');
    expect(etatClavier({}, 'sauver')).toBe('Ctrl+Shift+F5');
    expect(etatClavier({}, 'charger')).toBe('Ctrl+Shift+F9');
  });

  it('un choix déjà pris est dit', () => {
    expect(prise({}, 'chargerManette', 'Select + R1')).toBe('ouvrir le menu');
    expect(prise({}, 'chargerManette', 'Select + LT')).toBe('sauver');
    expect(prise({}, 'chargerManette', 'Select + L1')).toBeNull();
    expect(prise({ sauverManette: 'Select + L1' }, 'manette', 'Select + L1')).toBe('sauver');
    expect(prise({}, 'sauverClavier', 'Ctrl+Shift+F9')).toBe('charger');
    expect(prise({}, 'chargerClavier', 'aucune')).toBeNull();
  });

  it('les noms pour la personne', () => {
    expect(nomManette('Select + LT')).toBe('Vue (⧉) + LT (PlayStation : Créer + L2)');
    expect(nomManette('L3 + R3')).toBe('L3 + R3');
    expect(nomClavier('Ctrl+Shift+F5')).toBe('Ctrl + Maj + F5');
    expect(nomClavier('Pause')).toBe('Pause/Attn');
  });
});
