// Le script de la fenêtre cachée qui lit les jeux du compte GOG (src-tauri/ressources/gratuits/gog-jeux.js, lot C4a),
// essayé sur une boutique SIMULÉE : ce qu'il remet au cœur (le compte, puis les morceaux demandés un à un, ou l'étape
// d'un échec). La remise (`location.replace`) est remplacée par une capture ; `fetch` répond comme GOG (forme supposée).
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { afterEach, describe, expect, it, vi } from 'vitest';

const SCRIPT = readFileSync(resolve(process.cwd(), 'src-tauri/ressources/gratuits/gog-jeux.js'), 'utf8');
const dom = (globalThis as unknown as { jsdom: { reconfigure(o: { url: string }): void } }).jsdom;
const fenetre = window as unknown as Record<string, unknown>;

type Reponse = { status: number; json?: unknown } | 'erreur';
/** Un produit de la liste de GOG. */
const produit = (id: number | string, title: string, autre: Record<string, unknown> = {}) => ({ id, title, image: `//images-1.gog-statics.com/img${id}`, isGame: true, isMovie: false, ...autre });
/** Une page de la liste de GOG. */
const page = (totalPages: number, products: unknown[]): Reponse => ({ status: 200, json: { page: 1, totalPages, totalProducts: 0, products } });
const connecte = (userId: unknown = '48628349957132247'): Reponse => ({ status: 200, json: { isLoggedIn: true, userId, username: 'Seb', email: 'seb@exemple.fr' } });

/** Décode un morceau remis (`…/gog-jeux#k/n/<base64url>`). */
function morceau(u: string): { k: number; n: number; jeux: [string, string, string][] } {
  const [k, n, b] = u.slice('https://frogtend.invalid/gog-jeux#'.length).split('/');
  const binaire = atob(b.replace(/-/g, '+').replace(/_/g, '/'));
  const octets = Uint8Array.from(binaire, (c) => c.charCodeAt(0));
  return { k: Number(k), n: Number(n), jeux: JSON.parse(new TextDecoder().decode(octets)) };
}

/** Lance le script avec ces réponses (par adresse demandée) et rend les remises et les adresses demandées. */
async function essayer(repondre: (adresse: string) => Reponse, attendues = 1) {
  dom.reconfigure({ url: 'https://www.gog.com/en' });
  delete fenetre.__frogtendGogJeux;
  delete fenetre.__frogtendGog;
  const remis: string[] = [];
  const demandees: string[] = [];
  fenetre.__remettre = (u: string) => remis.push(u);
  vi.useFakeTimers({ toFake: ['setTimeout'] });
  vi.stubGlobal(
    'fetch',
    vi.fn(async (adresse: string, options: RequestInit) => {
      demandees.push(adresse);
      expect(options.credentials).toBe('same-origin');
      const r = repondre(adresse);
      if (r === 'erreur') throw new Error('réseau');
      return {
        status: r.status,
        ok: r.status >= 200 && r.status < 300,
        json: async () => {
          if (r.json === undefined) throw new Error('pas du JSON');
          return r.json;
        },
      };
    }),
  );
  new Function(SCRIPT.replace(/location\.replace\(/g, 'window.__remettre('))();
  await vi.waitFor(async () => {
    await vi.advanceTimersByTimeAsync(300);
    expect(remis.length).toBe(attendues);
  });
  vi.useRealTimers();
  return { remis, demandees };
}

/** Demande au script le morceau k (comme le cœur) et rend la remise. */
function demander(remis: string[], k: number): string {
  (fenetre.__frogtendGog as (k: number) => void)(k);
  return remis[remis.length - 1];
}

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe('gog-jeux.js', () => {
  it('connecté : le compte d’abord (chiffres seulement), puis les morceaux DEMANDÉS un à un, jeux cachés compris', async () => {
    const visibles = Array.from({ length: 60 }, (_, i) => produit(1000 + i, `Jeu ${i}`));
    const { remis, demandees } = await essayer((a) => {
      if (a === '/userData.json') return connecte();
      if (a.includes('hiddenFlag=0') && a.endsWith('page=1')) return page(2, visibles.slice(0, 40));
      if (a.includes('hiddenFlag=0') && a.endsWith('page=2')) return page(2, visibles.slice(40));
      // Un jeu caché, et un jeu déjà vu (gardé une fois).
      if (a.includes('hiddenFlag=1')) return page(1, [produit(5, 'Caché™'), produit(1000, 'Jeu 0')]);
      return { status: 404 };
    });
    expect(remis[0]).toBe('https://frogtend.invalid/gog-compte#48628349957132247');
    // Toutes les pages, visibles puis cachées.
    expect(demandees).toEqual([
      '/userData.json',
      '/account/getFilteredProducts?mediaType=1&sortBy=title&hiddenFlag=0&page=1',
      '/account/getFilteredProducts?mediaType=1&sortBy=title&hiddenFlag=0&page=2',
      '/account/getFilteredProducts?mediaType=1&sortBy=title&hiddenFlag=1&page=1',
    ]);
    // Rien n'est remis sans demande du cœur ; 61 jeux = 2 morceaux.
    expect(remis.length).toBe(1);
    const m0 = morceau(demander(remis, 0));
    expect([m0.k, m0.n, m0.jeux.length]).toEqual([0, 2, 50]);
    expect(m0.jeux[0]).toEqual(['1000', 'Jeu 0', '//images-1.gog-statics.com/img1000']);
    const m1 = morceau(demander(remis, 1));
    expect([m1.k, m1.n, m1.jeux.length]).toEqual([1, 2, 11]);
    expect(m1.jeux[m1.jeux.length - 1]).toEqual(['5', 'Caché™', '//images-1.gog-statics.com/img5']);
    // Rien du compte (nom, e-mail) n'est jamais remis.
    expect(remis.join(' ')).not.toMatch(/Seb|exemple/);
    // Une demande hors limites ne remet rien.
    (fenetre.__frogtendGog as (k: number) => void)(2);
    expect(remis.length).toBe(3);
  });

  it('seuls les jeux : un film ou un DLC signalé n’est pas gardé ; un compte sans jeu rend un morceau vide', async () => {
    const { remis } = await essayer((a) => {
      if (a === '/userData.json') return connecte(123);
      if (a.includes('hiddenFlag=0')) return page(1, [produit(1, 'Jeu'), produit(2, 'Film', { isMovie: true }), produit(3, 'DLC', { isGame: false })]);
      return page(0, []);
    });
    expect(remis[0]).toBe('https://frogtend.invalid/gog-compte#123');
    expect(morceau(demander(remis, 0)).jeux).toEqual([['1', 'Jeu', '//images-1.gog-statics.com/img1']]);
    const vide = await essayer((a) => (a === '/userData.json' ? connecte() : page(0, [])));
    expect(morceau(demander(vide.remis, 0))).toEqual({ k: 0, n: 1, jeux: [] });
  });

  it('pas connecté : « deconnecte », et la liste n’est jamais demandée', async () => {
    const { remis, demandees } = await essayer(() => ({ status: 200, json: { isLoggedIn: false } }));
    expect(remis).toEqual(['https://frogtend.invalid/gog-absent#deconnecte']);
    expect(demandees).toEqual(['/userData.json']);
  });

  it('une page en échec : échec complet, jamais une liste partielle', async () => {
    const { remis } = await essayer((a) => {
      if (a === '/userData.json') return connecte();
      if (a.endsWith('page=1') && a.includes('hiddenFlag=0')) return page(3, [produit(1, 'A')]);
      if (a.endsWith('page=2')) return { status: 500 };
      return page(3, [produit(9, 'Z')]);
    });
    expect(remis).toEqual(['https://frogtend.invalid/gog-absent#statut-500']);
    expect(fenetre.__frogtendGog).toBeUndefined();
  });

  it('trop de demandes (429), erreur réseau, page changée : l’étape le dit', async () => {
    const r429 = await essayer((a) => (a === '/userData.json' ? connecte() : { status: 429 }));
    expect(r429.remis).toEqual(['https://frogtend.invalid/gog-absent#trop-de-demandes']);
    const reseau = await essayer(() => 'erreur');
    expect(reseau.remis).toEqual(['https://frogtend.invalid/gog-absent#erreur-reseau']);
    // Des lignes, mais aucune lisible (les champs ont changé de nom) : jamais « zéro jeu ».
    const changee = await essayer((a) => (a === '/userData.json' ? connecte() : page(1, [{ productId: 1, name: 'X' }])));
    expect(changee.remis).toEqual(['https://frogtend.invalid/gog-absent#page-changee']);
    const sansListe = await essayer((a) => (a === '/userData.json' ? connecte() : { status: 200, json: { message: 'non' } }));
    expect(sansListe.remis).toEqual(['https://frogtend.invalid/gog-absent#page-changee']);
    const compte = await essayer(() => connecte(''));
    expect(compte.remis).toEqual(['https://frogtend.invalid/gog-absent#compte-illisible']);
  });

  it('beaucoup de pages : la page dit qu’elle avance toutes les 5 pages (le délai du cœur repart), puis le compte', async () => {
    const { remis, demandees } = await essayer(
      (a) => {
        if (a === '/userData.json') return connecte();
        if (a.includes('hiddenFlag=0')) return page(11, [produit(Number(a.split('page=')[1]), 'Jeu')]);
        return page(0, []);
      },
      3,
    );
    expect(demandees.length).toBe(1 + 11 + 1);
    expect(remis).toEqual(['https://frogtend.invalid/gog-progres#5', 'https://frogtend.invalid/gog-progres#10', 'https://frogtend.invalid/gog-compte#48628349957132247']);
    expect(morceau(demander(remis, 0)).jeux.length).toBe(11);
  });

  it('n’agit que sur https://www.gog.com', async () => {
    dom.reconfigure({ url: 'https://www.gog.com.evil.example/' });
    delete fenetre.__frogtendGogJeux;
    const f = vi.fn();
    vi.stubGlobal('fetch', f);
    new Function(SCRIPT.replace(/location\.replace\(/g, 'window.__remettre('))();
    await Promise.resolve();
    expect(f).not.toHaveBeenCalled();
  });
});
