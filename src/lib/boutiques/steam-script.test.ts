// Le script injecté dans la boutique Steam (src-tauri/ressources/gratuits/steam.js, lot C1), essayé sur des pages
// SIMULÉES : il ne doit dire que « connecté ou pas » et le steamid, jamais rien d'autre de la page.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const SCRIPT = readFileSync(resolve(process.cwd(), 'src-tauri/ressources/gratuits/steam.js'), 'utf8');
const dom = (globalThis as unknown as { jsdom: { reconfigure(o: { url: string }): void } }).jsdom;
const fenetre = window as unknown as Record<string, unknown>;
const ID = '76561197960287930';

/** La configuration que la vraie page met dans `#application_config` (forme relevée, valeurs inventées). */
function config(userinfo: unknown, magasin = '{"webapi_token":"JETON-SECRET-A-NE-JAMAIS-LIRE"}') {
  const attr = (v: string) => v.replace(/&/g, '&amp;').replace(/"/g, '&quot;');
  const u = typeof userinfo === 'string' ? userinfo : JSON.stringify(userinfo);
  return `<div id="application_config" data-config="{}" data-userinfo="${attr(u)}" data-store_user_config="${attr(magasin)}"></div>`;
}

async function essayer(html: string, ms = 0, url = 'https://store.steampowered.com/explore/') {
  dom.reconfigure({ url });
  document.title = 'Steam';
  document.body.innerHTML = html;
  delete fenetre.__frogtendSteam;
  new Function(SCRIPT)();
  await vi.advanceTimersByTimeAsync(ms);
  return document.title;
}

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

describe('steam.js', () => {
  it('connecté : rend seulement connecte et steamid (jamais le jeton ni le nom du compte)', async () => {
    const titre = await essayer(config({ logged_in: true, steamid: ID, accountid: 22202, account_name: 'seb_secret', country_code: 'FR' }));
    expect(titre).toBe(`FROGTEND:STEAM:{"connecte":true,"steamid":"${ID}"}`);
    expect(titre).not.toContain('JETON');
    expect(titre).not.toContain('seb_secret');
  });

  it('pas connecté (page de connexion) : connecte faux', async () => {
    const titre = await essayer(config({ logged_in: false }), 0, 'https://store.steampowered.com/login/?redir=explore%2F');
    expect(titre).toBe('FROGTEND:STEAM:{"connecte":false}');
  });

  it('un steamid qui n’a pas 17 chiffres : le script se tait', async () => {
    expect(await essayer(config({ logged_in: true, steamid: '123' }), 11000)).toBe('Steam');
    expect(await essayer(config({ logged_in: true, steamid: '7656119796028793x' }), 11000)).toBe('Steam');
  });

  it('configuration illisible ou absente : le script se tait, sans exception', async () => {
    expect(await essayer(config('{pas du json'), 11000)).toBe('Steam');
    expect(await essayer('<main>rien</main>', 11000)).toBe('Steam');
  });

  it('la configuration arrive un peu après : il la voit', async () => {
    await essayer('<main></main>', 0);
    document.body.insertAdjacentHTML('beforeend', config({ logged_in: true, steamid: ID }));
    await vi.advanceTimersByTimeAsync(600);
    expect(document.title).toBe(`FROGTEND:STEAM:{"connecte":true,"steamid":"${ID}"}`);
  });

  it('ne fait rien ailleurs que sur la boutique Steam en https', async () => {
    expect(await essayer(config({ logged_in: true, steamid: ID }), 0, 'https://steamcommunity.com/login/home/')).toBe('Steam');
    expect(await essayer(config({ logged_in: true, steamid: ID }), 0, 'http://store.steampowered.com/explore/')).toBe('Steam');
    expect(await essayer(config({ logged_in: true, steamid: ID }), 0, 'https://store.steampowered.com.evil.net/')).toBe('Steam');
  });

  it('ne lit jamais data-store_user_config', async () => {
    expect(SCRIPT).not.toMatch(/getAttribute\(\s*['"]data-store_user_config/);
    expect(SCRIPT).not.toMatch(/dataset/);
    expect(SCRIPT).not.toMatch(/document\.cookie/);
  });
});
