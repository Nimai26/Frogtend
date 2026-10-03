import { describe, expect, it } from 'vitest';
import { resumeRetro, type SuccesRetro } from './succes.svelte';

const base: SuccesRetro = { etat: 'ok', versions: [], jeu: null, compatibles: [] };

describe('le résumé RetroAchievements d’un jeu', () => {
  it('dit ce qu’il faut faire', () => {
    expect(resumeRetro({ ...base, etat: 'aucun' })).toBeNull();
    expect(resumeRetro({ ...base, etat: 'compte' })?.texte).toContain('Règle ton compte');
    expect(resumeRetro({ ...base, etat: 'pas_verifiable' })?.texte).toContain('pas encore');
    expect(resumeRetro({ ...base, etat: 'zip' })?.texte).toContain('décompresse');
    expect(resumeRetro({ ...base, versions: [{ chemin: 'E:\\Mario (E).nes', compatible: true, courante: true }] })?.ton).toBe('ok');
    expect(resumeRetro({ ...base, versions: [{ chemin: 'E:\\Jeu.chd', compatible: false, courante: true, verifiable: false }] })?.texte).toContain('format');
  });

  it('propose une autre version compatible, ou dit laquelle manque', () => {
    const r = resumeRetro({
      ...base,
      versions: [
        { chemin: 'E:\\Mario (F).nes', compatible: false, courante: true },
        { chemin: 'E:\\USA\\Mario (U).nes', compatible: true, courante: false },
      ],
    });
    expect(r?.texte).toContain('« Mario (U).nes »');
    const m = resumeRetro({
      ...base,
      versions: [{ chemin: 'E:\\Mario (F).nes', compatible: false, courante: true }],
      compatibles: [{ md5: 'a', nom: 'Super Mario Bros. (World)', etiquettes: ['nointro'] }],
    });
    expect(m?.texte).toContain('Version compatible manquante : Super Mario Bros. (World)');
    expect(m?.texte).toContain('Firehouse');
  });
});
