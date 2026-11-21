// Les messages communs aux imports des comptes de boutique (Steam lot C2, GOG lot C4a) : les textes de Steam restent
// exactement ceux d'avant la mise en commun.
import { describe, expect, it } from 'vitest';
import { annonceImport, direUneFoisParJour, messageExpire } from './import-compte';
import { annonceImportSteam, direExpireAujourdhui, MESSAGE_STEAM_EXPIRE, messageSteamExpire } from './steam.svelte';

describe('import-compte', () => {
  it('le message d’expiration, selon l’écran et la boutique', () => {
    expect(messageExpire('GOG', '/')).toBe('🔑 Ta connexion à GOG a expiré : dans 🛒 Boutiques, reconnecte-toi.');
    expect(messageExpire('GOG', '/boutiques')).toBe('🔑 Ta connexion à GOG a expiré : reconnecte-toi (bouton « 🔑 Me reconnecter à GOG »).');
    expect(messageExpire('GOG', '/taodbox/jeu')).toBe('🔑 Ta connexion à GOG a expiré : au bureau, dans 🛒 Boutiques, reconnecte-toi.');
  });

  it('Steam garde mot pour mot ses textes d’avant (lot C2)', () => {
    expect(MESSAGE_STEAM_EXPIRE).toBe('🔑 Ta connexion à Steam a expiré : dans 🛒 Boutiques, reconnecte-toi.');
    expect(messageSteamExpire('/boutiques')).toBe('🔑 Ta connexion à Steam a expiré : reconnecte-toi (bouton « 🔑 Me reconnecter à Steam »).');
    expect(messageSteamExpire('/taodbox')).toBe('🔑 Ta connexion à Steam a expiré : au bureau, dans 🛒 Boutiques, reconnecte-toi.');
    const r = { reconnecter: false, total: 1, nouveaux: 0, nouveaux_installes: 0 };
    expect(annonceImportSteam(r, true)).toEqual({ texte: 'Rien de nouveau : tes jeux Steam sont déjà dans ta ludothèque.' });
    expect(annonceImportSteam(r, true)).toEqual(annonceImport('steam', 'Steam', r, true));
  });

  it('une fois par jour : la clé de Steam ne change pas', () => {
    const m = new Map<string, string>();
    const s = { getItem: (k: string) => m.get(k) ?? null, setItem: (k: string, v: string) => void m.set(k, v) };
    direExpireAujourdhui('seb', '2026-11-21', s);
    expect([...m.keys()]).toEqual(['frogtend.steamExpire.seb']);
    expect(direUneFoisParJour('autre', '2026-11-21', s)).toBe(true);
    expect(direUneFoisParJour('autre', '2026-11-21', s)).toBe(false);
    expect(direUneFoisParJour('x', '2026-11-21', null)).toBe(true);
  });
});
