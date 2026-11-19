// La connexion à Steam par sa page officielle (lot C1) : ce que l'interface en montre.
import { describe, expect, it } from 'vitest';
import type { EtatSteam } from '$lib/api';
import { annoncePourMoi, connecteASteam, libelleCompteSecours } from './steam.svelte';

const base: EtatSteam = { compte: null, cle_enregistree: false, nb_jeux: 0, maj_le: null, steam_installe: true, connecte: false, cle_autre_compte: false };

describe('libelleCompteSecours', () => {
  it('le bloc Secours voit la connexion par la page, et la clé d’un autre compte', () => {
    expect(libelleCompteSecours(null)).toBe('compte pas encore réglé');
    expect(libelleCompteSecours({ ...base, compte: 'NimaiTakahashi', cle_enregistree: true })).toBe('compte NimaiTakahashi');
    expect(libelleCompteSecours({ ...base, compte: '76561197960287930', connecte: true })).toBe('connecté à Steam · sans clé d’API');
    expect(libelleCompteSecours({ ...base, connecte: true, cle_enregistree: true })).toBe('connecté à Steam · clé d’API enregistrée');
    const autre = libelleCompteSecours({ ...base, connecte: true, cle_enregistree: true, cle_autre_compte: true });
    expect(autre).toContain('autre compte');
    expect(autre).toContain('pas effacée');
  });
});

describe('connecteASteam', () => {
  it('« ✅ Connecté » seulement quand le cœur le dit (steamid rangé par la page officielle)', () => {
    expect(connecteASteam({ ...base, connecte: true, compte: '76561197960287930' })).toBe(true);
    expect(connecteASteam(base)).toBe(false);
    // Un compte réglé avec la clé d'API (secours) n'est pas une connexion par la page.
    expect(connecteASteam({ ...base, compte: 'NimaiTakahashi', cle_enregistree: true })).toBe(false);
    expect(connecteASteam(null)).toBe(false);
    expect(connecteASteam(undefined)).toBe(false);
  });
});

describe('annoncePourMoi', () => {
  it('n’écoute que l’annonce du profil qui a ouvert la fenêtre, s’il est toujours ouvert', () => {
    expect(annoncePourMoi({ profil: 'seb', ok: true }, 'seb', 'seb')).toBe(true);
    expect(annoncePourMoi({ profil: 'seb', ok: false }, 'seb', 'seb')).toBe(true);
    // Un autre profil ouvert entre-temps, ou l'annonce d'un autre profil : rien.
    expect(annoncePourMoi({ profil: 'seb', ok: true }, 'seb', 'lea')).toBe(false);
    expect(annoncePourMoi({ profil: 'lea', ok: true }, 'seb', 'seb')).toBe(false);
    expect(annoncePourMoi({ ok: true }, 'seb', 'seb')).toBe(false);
    expect(annoncePourMoi(null, 'seb', 'seb')).toBe(false);
    expect(annoncePourMoi({ profil: 'seb' }, null, null)).toBe(false);
  });
});
