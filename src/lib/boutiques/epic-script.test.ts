// Le script injecté dans la page d'Epic (src-tauri/ressources/gratuits/epic.js), essayé sur des pages SIMULÉES : la
// vraie page refuse les lectures hors navigateur et change sans prévenir (« The New Epic Games Store », 18/11).
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const SCRIPT = readFileSync(resolve(process.cwd(), 'src-tauri/ressources/gratuits/epic.js'), 'utf8');
const dom = (globalThis as unknown as { jsdom: { reconfigure(o: { url: string }): void } }).jsdom;
const fenetre = window as unknown as Record<string, unknown>;

/** Monte une page de la boutique, lance le script, et laisse passer `ms` millisecondes. */
async function essayer(html: string, ms = 0, options: { connexion?: boolean; verification?: boolean; url?: string } = {}) {
  dom.reconfigure({ url: options.url ?? 'https://store.epicgames.com/en-US/p/knightica-3be45c' });
  document.title = 'Knightica';
  document.body.innerHTML = html;
  delete fenetre.__frogtend;
  if (options.verification) fenetre.__frogtendMode = 'verification';
  else if (options.connexion) fenetre.__frogtendMode = 'connexion';
  else delete fenetre.__frogtendMode;
  new Function(SCRIPT)();
  await vi.advanceTimersByTimeAsync(ms);
  return document.title;
}

/** Note les clics sur un élément. */
function compterClics(selecteur: string) {
  const e = document.querySelector(selecteur)!;
  const clics = { n: 0 };
  e.addEventListener('click', () => clics.n++);
  return clics;
}

const AVATAR = '<header><button data-testid="avatar" aria-label="Account menu">N</button></header>';

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

describe('epic.js en mode vérification (page cachée de 🛒 Boutiques, 20/11)', () => {
  const ACCUEIL = { verification: true, url: 'https://store.epicgames.com/en-US/' };

  it('compte connecté : CONNECTE, sans rien cliquer', async () => {
    await essayer(`${AVATAR}<button id="g">Get</button>`, 0, ACCUEIL);
    const get = compterClics('#g');
    await vi.advanceTimersByTimeAsync(1000);
    expect(document.title).toBe('FROGTEND:CONNECTE');
    expect(get.n).toBe(0);
  });

  it('Epic dit « pas connecté » (isloggedin="false") pendant 8 s : CONNEXION, pas avant', async () => {
    await essayer('<egs-navigation isloggedin="false"></egs-navigation>', 5000, ACCUEIL);
    expect(document.title).toBe('Knightica');
    await vi.advanceTimersByTimeAsync(4000);
    expect(document.title).toBe('FROGTEND:CONNEXION');
  });

  it('le seul texte « Sign In » ne décide rien (ligne lente) : ne dit rien', async () => {
    const titre = await essayer('<a href="https://www.epicgames.com/id/login">Sign In</a>', 25000, ACCUEIL);
    expect(titre).toBe('Knightica');
  });

  it('« Sign In » passager puis l’avatar (page qui se remplit) : CONNECTE', async () => {
    await essayer('<a id="s" href="https://www.epicgames.com/id/login">Sign In</a>', 1500, ACCUEIL);
    document.body.innerHTML = AVATAR;
    await vi.advanceTimersByTimeAsync(1000);
    expect(document.title).toBe('FROGTEND:CONNECTE');
  });

  it('rien de sûr au bout de 20 s : ne dit rien', async () => {
    const titre = await essayer('<main>chargement</main>', 25000, ACCUEIL);
    expect(titre).toBe('Knightica');
  });

  it('renvoyé vers la page de connexion d’Epic : CONNEXION ; un autre site : rien', async () => {
    expect(await essayer('<form></form>', 0, { verification: true, url: 'https://www.epicgames.com/id/login?redirectUrl=x' })).toBe('FROGTEND:CONNEXION');
    expect(await essayer('<form></form>', 25000, { verification: true, url: 'https://exemple.fr/' })).toBe('Knightica');
    expect(await essayer('<form></form>', 25000, { verification: true, url: 'https://www.epicgames.com/id/autre' })).toBe('Knightica');
  });

  it('un captcha : ne dit rien (ni CAPTCHA ni ERREUR)', async () => {
    const titre = await essayer('<iframe title="hCaptcha"></iframe>', 1000, ACCUEIL);
    expect(titre).toBe('Knightica');
  });
});

describe('epic.js', () => {
  it('voit « In Library » sur la nouvelle boutique (Knightica, 18/11) : DEJA, sans rien cliquer', async () => {
    const titre = await essayer(`${AVATAR}<main><button id="b"><span>In Library</span></button></main>`);
    expect(titre).toBe('FROGTEND:DEJA');
  });

  it('ignore un bouton caché', async () => {
    const titre = await essayer(`${AVATAR}<button style="display:none">In Library</button><button>Wishlist</button>`, 31000);
    expect(titre.startsWith("FROGTEND:ERREUR:pas de bouton d'achat | ")).toBe(true);
  });

  it('pas connecté (« Sign In » visible) : CONNEXION, et « Get » n’est pas cliqué', async () => {
    await essayer('<a href="https://www.epicgames.com/id/login">Sign In</a><button id="g">Get</button>');
    const get = compterClics('#g');
    await vi.advanceTimersByTimeAsync(3000);
    expect(document.title).toBe('FROGTEND:CONNEXION');
    expect(get.n).toBe(0);
  });

  it('pas connecté et pas de bouton : CONNEXION en quelques secondes, pas au bout de 30 s', async () => {
    const titre = await essayer('<button>Sign in</button>', 9000);
    expect(titre).toBe('FROGTEND:CONNEXION');
  });

  it('la page de connexion d’Epic dit CONNEXION', async () => {
    const titre = await essayer('<form></form>', 0, { url: 'https://www.epicgames.com/id/login?redirectUrl=x' });
    expect(titre).toBe('FROGTEND:CONNEXION');
  });

  it('ne clique jamais sur un bouton payant, et dit lequel', async () => {
    const t1 = await essayer(`${AVATAR}<button id="p">Buy Now</button>`, 6000);
    expect(t1.startsWith('FROGTEND:ERREUR:pas gratuit | vu « buy now »')).toBe(true);
    const t2 = await essayer(`${AVATAR}<button id="p" data-testid="purchase-cta-button">€19.99</button>`, 6000);
    expect(t2.startsWith('FROGTEND:ERREUR:pas gratuit')).toBe(true);
  });

  it('un prix vu AVANT « Get » (page qui se remplit, Mark of the Deep 20/11) : pas de conclusion, « Get » est pris', async () => {
    await essayer(`${AVATAR}<a href="#">-100% €19.50* Free</a>`, 2000);
    expect(document.title).toBe('Knightica');
    document.body.insertAdjacentHTML('beforeend', '<button id="g">Get</button>');
    const get = compterClics('#g');
    await vi.advanceTimersByTimeAsync(3000);
    expect(get.n).toBe(1);
  });

  it('un contenu additionnel payant (/bundles/) n’est pas le jeu : ignoré', async () => {
    const titre = await essayer(`${AVATAR}<a href="/en-US/bundles/mark-of-the-deep-deluxe"><button>Buy Now €24.99</button></a>`, 31000);
    expect(titre.startsWith("FROGTEND:ERREUR:pas de bouton d'achat")).toBe(true);
  });

  it('dit l’étape et les boutons visibles, en 200 caractères au plus', async () => {
    const beaucoup = Array.from({ length: 30 }, (_, i) => `<button>Bouton numéro ${i} assez long pour déborder</button>`).join('');
    const titre = await essayer(`${AVATAR}${beaucoup}`, 31000);
    expect(titre.startsWith("FROGTEND:ERREUR:pas de bouton d'achat | n ; bouton numéro 0")).toBe(true);
    expect(titre.length).toBeLessThanOrEqual(200);
  });

  it('obtient un jeu : « Get », puis « Place Order » à 0 € dans le cadre d’achat, puis « In Library »', async () => {
    await essayer(`${AVATAR}<button id="g">Get</button>`);
    const get = document.getElementById('g')!;
    let commandes = 0;
    // Au clic sur « Get » : le cadre d'achat (même origine) s'ouvre avec « Place Order ».
    get.addEventListener('click', () => {
      const conteneur = document.createElement('div');
      conteneur.id = 'webPurchaseContainer';
      const cadre = document.createElement('iframe');
      conteneur.appendChild(cadre);
      document.body.appendChild(conteneur);
      const d = cadre.contentDocument!;
      d.body.innerHTML = '<p>Total €0.00</p><button id="c">Place Order</button>';
      d.getElementById('c')!.addEventListener('click', () => {
        commandes++;
        conteneur.remove();
        get.textContent = 'In Library';
      });
    });
    await vi.advanceTimersByTimeAsync(5000);
    expect(commandes).toBe(1);
    expect(document.title).toBe('FROGTEND:OBTENU');
  });

  it('refuse une commande dont le total n’est pas nul', async () => {
    await essayer(`${AVATAR}<button id="g">Get</button>`);
    const get = document.getElementById('g')!;
    let commandes = 0;
    get.addEventListener('click', () => {
      const conteneur = document.createElement('div');
      conteneur.id = 'webPurchaseContainer';
      const cadre = document.createElement('iframe');
      conteneur.appendChild(cadre);
      document.body.appendChild(conteneur);
      cadre.contentDocument!.body.innerHTML = '<p>Total €4.99</p><button id="c">Place Order</button>';
      cadre.contentDocument!.getElementById('c')!.addEventListener('click', () => commandes++);
    });
    await vi.advanceTimersByTimeAsync(5000);
    expect(commandes).toBe(0);
    expect(document.title.startsWith('FROGTEND:ERREUR:commande payante refusée')).toBe(true);
  });

  /** « Get » ouvre un cadre d'achat dont le contenu est `contenu` ; rend le compteur des clics sur « Place Order ». */
  function cadreAuClic(contenu: string) {
    const get = document.getElementById('g')!;
    const clics = { n: 0 };
    get.addEventListener('click', () => {
      const conteneur = document.createElement('div');
      conteneur.id = 'webPurchaseContainer';
      const cadre = document.createElement('iframe');
      conteneur.appendChild(cadre);
      document.body.appendChild(conteneur);
      cadre.contentDocument!.body.innerHTML = contenu;
      cadre.contentDocument!.getElementById('c')!.addEventListener('click', () => clics.n++);
    });
    return clics;
  }

  it('ne commande JAMAIS quand le total est introuvable (revue des experts, 18/11)', async () => {
    await essayer(`${AVATAR}<button id="g">Get</button>`);
    const clics = cadreAuClic('<p>Order summary</p><button id="c">Place Order</button>');
    await vi.advanceTimersByTimeAsync(5000);
    expect(clics.n).toBe(0);
    expect(document.title.startsWith('FROGTEND:ERREUR:total introuvable | ')).toBe(true);
  });

  it('un sous-total à 0 avant un total payant n’est jamais pris pour gratuit (revue des experts, 18/11)', async () => {
    await essayer(`${AVATAR}<button id="g">Get</button>`);
    const clics = cadreAuClic('<p>Subtotal €0.00</p><p>Total €4.99</p><button id="c">Place Order</button>');
    await vi.advanceTimersByTimeAsync(5000);
    expect(clics.n).toBe(0);
    expect(document.title.startsWith('FROGTEND:ERREUR:commande payante refusée')).toBe(true);
  });

  it('un sous-total payant et un total à 0 : seul « Total » compte, la commande passe à 0', async () => {
    await essayer(`${AVATAR}<button id="g">Get</button>`);
    const clics = cadreAuClic('<p>Subtotal €4.99</p><p>Discount -€4.99</p><p>Total €0.00</p><button id="c">Place Order</button>');
    await vi.advanceTimersByTimeAsync(5000);
    expect(clics.n).toBe(1);
  });

  it('montre une étape en français seulement : le détail anglais va après « | » (pour le journal)', async () => {
    const t = await essayer('<p>x</p>', 0, { url: 'https://evil.example.com/p/x' });
    expect(t.startsWith('FROGTEND:ERREUR:page inattendue | evil.example.com')).toBe(true);
    expect(t.split(' | ')[0]).toBe('FROGTEND:ERREUR:page inattendue');
  });

  it('ignore le « Get » d’un autre jeu (carte d’un carrousel) et ne choisit jamais entre deux « Get »', async () => {
    // Le jeu affiché est déjà possédé ; un autre jeu offert, sur sa carte, a « Get » : rien n'est cliqué.
    const t1 = await essayer(`${AVATAR}<main><button>In Library</button></main><a href="/en-US/p/autre-jeu"><button id="x">Get</button></a>`, 2000);
    expect(t1).toBe('FROGTEND:DEJA');
    // Deux « Get » sans rien pour les départager : aucun clic.
    await essayer(`${AVATAR}<button id="g1">Get</button><button id="g2">Get</button>`);
    const c1 = compterClics('#g1');
    const c2 = compterClics('#g2');
    await vi.advanceTimersByTimeAsync(3000);
    expect(c1.n + c2.n).toBe(0);
    expect(document.title.startsWith("FROGTEND:ERREUR:plusieurs boutons d'achat")).toBe(true);
  });

  it('mode connexion : CONNECTE quand le compte apparaît, sans rien cliquer', async () => {
    const t1 = await essayer('<button>Sign in</button>', 2000, { connexion: true, url: 'https://store.epicgames.com/en-US/' });
    expect(t1).toBe('Knightica');
    const clics = compterClics('button');
    document.body.innerHTML = `${AVATAR}<button id="g">Get</button>`;
    const get = compterClics('#g');
    await vi.advanceTimersByTimeAsync(1000);
    expect(document.title).toBe('FROGTEND:CONNECTE');
    expect(clics.n + get.n).toBe(0);
  });
});
