import { describe, expect, it } from 'vitest';
import { EFFETS_IMAGE, libelleEffet } from './effetsImage';

describe('les effets d’image', () => {
  it('les mêmes identifiants que le cœur, chacun avec un nom et une phrase', () => {
    // RetroArch (`EFFETS_IMAGE` du cœur) et BigPEmu (`bigpemu::EFFETS`, préfixés : jamais confondus).
    const bigpemu = ['cheesy_scanlines', 'crt-hyllian', 'gamma', 'liz', 'oh_my_gauss', 'warping', 'xbr-lv2', 'xbr-lv3-multipass'].map((e) => `bigpemu:${e}`);
    expect(Object.keys(EFFETS_IMAGE).sort()).toEqual([...bigpemu, 'crt', 'crt-riche', 'lisse', 'net', 'portable'].sort());
    for (const e of Object.values(EFFETS_IMAGE)) expect(e.libelle && e.detail).toBeTruthy();
  });
  it('aucun réglage ou « aucun » : aucun effet', () => {
    expect(libelleEffet(undefined)).toBe('aucun effet');
    expect(libelleEffet('aucun')).toBe('aucun effet');
    expect(libelleEffet('crt')).toBe('Télé d’époque (simple)');
    expect(libelleEffet('bigpemu:crt-hyllian')).toBe('Télé d’époque');
  });
});
