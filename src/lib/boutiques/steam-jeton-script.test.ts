// Le script de la fenêtre cachée qui lit le jeton Steam (src-tauri/ressources/gratuits/steam-jeton.js, lots C2 et C3),
// essayé sur des pages SIMULÉES : ce qu'il remet au cœur (le jeton, ou l'étape qui choisit le message — « reconnecte-toi »,
// page changée, réponse inhabituelle). La remise (`location.replace`) est remplacée par une capture.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { afterEach, describe, expect, it, vi } from 'vitest';

const SCRIPT = readFileSync(resolve(process.cwd(), 'src-tauri/ressources/gratuits/steam-jeton.js'), 'utf8');
const dom = (globalThis as unknown as { jsdom: { reconfigure(o: { url: string }): void } }).jsdom;
const fenetre = window as unknown as Record<string, unknown>;

function config(userinfo: unknown, magasin: unknown = {}) {
  const attr = (v: string) => v.replace(/&/g, '&amp;').replace(/"/g, '&quot;');
  return `<div id="application_config" data-userinfo="${attr(JSON.stringify(userinfo))}" data-store_user_config="${attr(JSON.stringify(magasin))}"></div>`;
}

/** Lance le script sur cette page, avec cette réponse de la boutique (ou une erreur réseau), et rend l'adresse remise. */
async function essayer(html: string, reponse: { status: number; redirected?: boolean; json?: unknown } | 'erreur') {
  dom.reconfigure({ url: 'https://store.steampowered.com/' });
  document.body.innerHTML = html;
  delete fenetre.__frogtendSteamJeton;
  const remis: string[] = [];
  fenetre.__remettre = (u: string) => remis.push(u);
  vi.stubGlobal(
    'fetch',
    vi.fn(async () => {
      if (reponse === 'erreur') throw new Error('réseau');
      return {
        status: reponse.status,
        redirected: !!reponse.redirected,
        json: async () => {
          if (reponse.json === undefined) throw new Error('pas du JSON');
          return reponse.json;
        },
      };
    }),
  );
  new Function(SCRIPT.replace(/location\.replace\(/g, 'window.__remettre('))();
  await vi.waitFor(() => expect(remis.length).toBe(1));
  return remis[0];
}

afterEach(() => vi.unstubAllGlobals());

describe('steam-jeton.js', () => {
  it('le jeton dans la page : remis tel quel, sans rien demander à la boutique', async () => {
    const u = await essayer(config({ logged_in: true }, { webapi_token: 'aaa.bbb.ccc' }), { status: 500 });
    expect(u).toBe('https://frogtend.invalid/steam-jeton#aaa.bbb.ccc');
    expect(fetch).not.toHaveBeenCalled();
  });

  it('connecté selon la page, la boutique répond sans jeton : étape « connecte-statut-200 » (page changée)', async () => {
    const u = await essayer(config({ logged_in: true }), { status: 200, json: { success: 1, data: {} } });
    expect(u).toBe('https://frogtend.invalid/steam-absent#connecte-statut-200-success-1');
  });

  it('déconnecté selon la page : étape « deconnecte » (reconnecte-toi)', async () => {
    const u = await essayer(config({ logged_in: false }), { status: 200, redirected: true });
    expect(u).toBe('https://frogtend.invalid/steam-absent#deconnecte-statut-200-redirige-pas-json');
  });

  it('plus de configuration du tout (maintenance, page changée) : étape « sans-configuration »', async () => {
    expect(await essayer('<main>Maintenance</main>', { status: 503 })).toBe('https://frogtend.invalid/steam-absent#sans-configuration-statut-503-pas-json');
    expect(await essayer('<main></main>', 'erreur')).toBe('https://frogtend.invalid/steam-absent#sans-configuration-erreur-reseau');
  });

  it('la boutique donne le jeton : remis', async () => {
    const u = await essayer(config({ logged_in: true }), { status: 200, json: { success: 1, data: { webapi_token: 'xxx.yyy.zzz' } } });
    expect(u).toBe('https://frogtend.invalid/steam-jeton#xxx.yyy.zzz');
  });
});
