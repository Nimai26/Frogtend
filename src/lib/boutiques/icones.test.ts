// Les icônes des boutiques sur les pastilles (Seb, 19/11) : l'icône du lanceur du PC, sinon le texte ; une seule
// demande par boutique pour toute l'application ; hors de l'application, toujours le texte.
import { beforeEach, describe, expect, it, vi } from 'vitest';

let dansTauri = true;
const demande = vi.fn<(b: string) => Promise<string | null>>();
vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => dansTauri }));
vi.mock('$lib/api', () => ({ api: { boutiqueIcone: (b: string) => demande(b) } }));

const { boutiqueDeLIcone, demanderIcone, estAdresseIcone, iconeDe, icones } = await import('./icones.svelte');

const PNG = 'data:image/png;base64,iVBORw0KGgo=';
const attendre = () => new Promise((r) => setTimeout(r, 0));

describe('Icônes des boutiques sur les pastilles', () => {
  beforeEach(() => {
    demande.mockReset();
    dansTauri = true;
  });

  it('ne cherche un lanceur que pour une vraie boutique de PC', () => {
    expect(boutiqueDeLIcone('steam')).toBe('steam');
    expect(boutiqueDeLIcone('ea')).toBe('origin');
    for (const o of ['xboxone', 'xbox', 'firehouse', 'disque', 'autre', '']) expect(boutiqueDeLIcone(o)).toBeNull();
  });

  it('montre l’icône quand elle existe, sinon le texte', () => {
    const connues = { steam: PNG, epic: null };
    expect(iconeDe('steam', connues)).toBe(PNG);
    expect(iconeDe('epic', connues)).toBeNull();
    expect(iconeDe('gog', connues)).toBeNull();
    expect(iconeDe('xboxone', { xboxone: PNG })).toBeNull();
    expect(iconeDe('ea', { origin: PNG })).toBe(PNG);
  });

  it('n’accepte qu’une image PNG en data:', () => {
    expect(estAdresseIcone(PNG)).toBe(true);
    expect(estAdresseIcone('https://exemple.fr/steam.png')).toBe(false);
    expect(estAdresseIcone('data:image/svg+xml;base64,PHN2Zz4=')).toBe(false);
    expect(estAdresseIcone('data:image/png;base64,"><script>')).toBe(false);
    expect(estAdresseIcone(null)).toBe(false);
  });

  it('demande chaque boutique UNE fois pour toute l’appli', async () => {
    demande.mockResolvedValue(PNG);
    demanderIcone('steam');
    demanderIcone('steam');
    demanderIcone('firehouse');
    await attendre();
    expect(demande).toHaveBeenCalledTimes(1);
    expect(demande).toHaveBeenCalledWith('steam');
    expect(icones.steam).toBe(PNG);
    demanderIcone('steam');
    expect(demande).toHaveBeenCalledTimes(1);
  });

  it('garde le texte si le cœur ne rend rien, une erreur ou une adresse étrange', async () => {
    demande.mockResolvedValueOnce(null).mockRejectedValueOnce(new Error('x')).mockResolvedValueOnce('https://ailleurs/x.png');
    demanderIcone('epic');
    demanderIcone('gog');
    demanderIcone('uplay');
    await attendre();
    expect(iconeDe('epic')).toBeNull();
    expect(iconeDe('gog')).toBeNull();
    expect(iconeDe('uplay')).toBeNull();
  });

  it('hors de l’application (navigateur, tests) : rien n’est demandé, le texte reste', async () => {
    dansTauri = false;
    demanderIcone('amazon');
    await attendre();
    expect(demande).not.toHaveBeenCalled();
    expect(iconeDe('amazon')).toBeNull();
  });
});
