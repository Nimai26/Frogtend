// La connexion à Steam par sa page officielle (lot C1) : ce que l'interface en montre.
import { describe, expect, it } from 'vitest';
import type { EtatSteam } from '$lib/api';
import { annonceImportSteam, annoncePourMoi, connecteASteam, direExpireAujourdhui, libelleCompteSecours, MESSAGE_STEAM_EXPIRE, messageSteamExpire } from './steam.svelte';

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

describe('annonceImportSteam (lot C2)', () => {
  const r = { reconnecter: false, total: 120, nouveaux: 0, nouveaux_installes: 0 };
  it('à l’ouverture de Frogtend : rien sans nouveauté, les nouveaux jeux sinon', () => {
    expect(annonceImportSteam(r, false)).toBeNull();
    expect(annonceImportSteam({ ...r, nouveaux: 118, nouveaux_installes: 2 }, false)?.texte).toBe(
      '🎮 118 jeux de ton compte Steam ajoutés à ta ludothèque (2 déjà installés). Les autres s’installent avec ⬇ Installer.',
    );
  });
  it('à la main : toujours une réponse, « Rien de nouveau » compris', () => {
    expect(annonceImportSteam(r, true)?.texte).toContain('Rien de nouveau');
    expect(annonceImportSteam({ ...r, nouveaux: 1, nouveaux_installes: 0 }, true)?.texte).toBe('🎮 1 jeu de ton compte Steam ajouté à ta ludothèque. Il s’installe avec ⬇ Installer.');
  });
  it('connexion expirée : dite dans les deux cas, en alerte', () => {
    for (const main of [true, false]) expect(annonceImportSteam({ ...r, reconnecter: true }, main)).toEqual({ texte: MESSAGE_STEAM_EXPIRE, ton: 'alerte' });
    expect(MESSAGE_STEAM_EXPIRE).toBe('🔑 Ta connexion à Steam a expiré : dans 🛒 Boutiques, reconnecte-toi.');
  });
});

describe('connexion Steam expirée (lot C2)', () => {
  it('« dans 🛒 Boutiques » seulement hors de cette page ; « au bureau » depuis Taodbox', () => {
    expect(messageSteamExpire('/')).toBe(MESSAGE_STEAM_EXPIRE);
    expect(messageSteamExpire('/boutiques')).not.toContain('dans 🛒 Boutiques');
    expect(messageSteamExpire('/taodbox')).toBe('🔑 Ta connexion à Steam a expiré : au bureau, dans 🛒 Boutiques, reconnecte-toi.');
    expect(annonceImportSteam({ reconnecter: true, total: 0, nouveaux: 0, nouveaux_installes: 0 }, false, '/taodbox')?.texte).toContain('au bureau');
  });
  it('dite une fois par jour et par profil', () => {
    const m = new Map<string, string>();
    const s = { getItem: (k: string) => m.get(k) ?? null, setItem: (k: string, v: string) => void m.set(k, v) };
    expect(direExpireAujourdhui('seb', '2026-11-20', s)).toBe(true);
    expect(direExpireAujourdhui('seb', '2026-11-20', s)).toBe(false);
    expect(direExpireAujourdhui('lea', '2026-11-20', s)).toBe(true);
    expect(direExpireAujourdhui('seb', '2026-11-21', s)).toBe(true);
    // Sans stockage, ou stockage refusé : dite.
    expect(direExpireAujourdhui('seb', '2026-11-21', null)).toBe(true);
    const refuse = { getItem: () => { throw new Error('non'); }, setItem: () => {} };
    expect(direExpireAujourdhui('seb', '2026-11-21', refuse)).toBe(true);
  });
});
