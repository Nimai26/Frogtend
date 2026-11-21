// Frogtend — lire les jeux POSSÉDÉS du compte GOG connecté (lot C4a), dans la fenêtre CACHÉE ouverte sur
// https://www.gog.com (navigateur propre au profil, le même que pour le jeu offert). Comme pour Steam (0.215.2), la
// page demande elle-même, avec SES cookies (`credentials: 'same-origin'`), ce que la boutique montre sur
// gog.com ▸ Mes jeux ; elle ne clique sur rien, ne lit aucun cookie et n'envoie rien ailleurs qu'au cœur.
//
// Remise au cœur par des navigations vers https://frogtend.invalid/… que le cœur INTERCEPTE et ANNULE (rien n'est
// chargé, rien n'entre dans l'historique, rien ne passe par le titre) :
//   …/gog-compte#<userId>            le compte connecté (chiffres seulement) ; le cœur demande alors le morceau 0 ;
//   …/gog-jeux#<k>/<n>/<base64url>   le morceau k sur n (50 jeux au plus : [[id, titre, image], …]), envoyé quand le
//                                    cœur le DEMANDE (`window.__frogtendGog(k)`) — jamais deux remises d'un coup ;
//   …/gog-progres#<pages>            toutes les 5 pages lues : la page avance (le cœur attend 60 s au plus sans
//                                    nouvelle, 10 min en tout : 2 × 100 pages au plus, 250 ms de pause entre deux) ;
//   …/gog-absent#<étape>             pas de liste (deconnecte, trop-de-demandes, erreur-reseau, page-changee…).
// Si la page se recharge avant le premier morceau, ce script, relancé, redonne le même compte : le cœur redemande le
// morceau 0 à la nouvelle page.
//
// Ce qui est supposé de GOG (relevé dans la documentation publique de la communauté, PAS vérifié sur un vrai compte) :
//   - /userData.json rend `isLoggedIn` (booléen) et `userId` (texte de chiffres) ; seuls ces deux champs sont lus,
//     jamais le reste de l'objet (il contient aussi le nom et d'autres données du compte) ;
//   - /account/getFilteredProducts?mediaType=1&hiddenFlag=0|1&page=N rend `{totalPages, products: [{id, title,
//     image, isGame, isMovie…}]}`, pages comptées à partir de 1 ; mediaType=1 = les jeux (pas les films) ;
//     hiddenFlag=1 = les jeux CACHÉS par la personne sur gog.com (Seb, 21/11 : ils entrent aussi) ;
//   - un DLC n'a pas sa propre ligne dans cette liste (il est rangé sous son jeu) ; si une ligne dit `isGame: false`
//     (ou `isMovie: true`), elle n'est pas gardée ;
//   - `image` est une adresse sans protocole ni extension (« //images-1.gog-statics.com/<empreinte> »), normalisée
//     par le cœur.
(function () {
  try {
    if (window.__frogtendGogJeux) return;
    window.__frogtendGogJeux = true;
    if (location.protocol !== 'https:' || location.hostname !== 'www.gog.com') return;
  } catch (e) {
    return;
  }
  const PLAFOND_PAGES = 100;
  const PAR_MORCEAU = 50;
  const PAGES_PAR_NOUVELLE = 5;
  const remettre = (chemin, fragment) => {
    try {
      location.replace('https://frogtend.invalid/' + chemin + '#' + fragment);
    } catch (e) {
      // Rien : le cœur attendra puis dira « pas de réponse ».
    }
  };
  const absent = (etape) => remettre('gog-absent', String(etape).replace(/[^A-Za-z0-9-]/g, '').slice(0, 60) || 'sans-detail');
  const pause = (ms) => new Promise((ok) => setTimeout(ok, ms));
  // Une étape d'échec (jamais une liste partielle : une page en échec arrête tout).
  function Arret(etape) {
    this.etape = etape;
  }
  const lireJson = async (adresse) => {
    let r;
    try {
      r = await fetch(adresse, { credentials: 'same-origin', cache: 'no-store', headers: { Accept: 'application/json' } });
    } catch (e) {
      throw new Arret('erreur-reseau');
    }
    if (r.status === 429) throw new Arret('trop-de-demandes');
    if (r.status === 401 || r.status === 403) throw new Arret('deconnecte');
    if (!r.ok) throw new Arret('statut-' + r.status);
    try {
      return await r.json();
    } catch (e) {
      throw new Arret('pas-json');
    }
  };
  // Le texte en base64url (UTF-8), sans remplissage.
  const encoder = (texte) => {
    const o = new TextEncoder().encode(texte);
    let b = '';
    for (let i = 0; i < o.length; i++) b += String.fromCharCode(o[i]);
    return btoa(b).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
  };
  // D'un produit, seulement son numéro, son titre et son image ; `null` : pas un jeu (film, DLC) ; `undefined` :
  // ligne illisible.
  const garder = (p) => {
    if (!p || typeof p !== 'object') return undefined;
    if (p.isGame === false || p.isMovie === true) return null;
    const id = typeof p.id === 'number' || typeof p.id === 'string' ? String(p.id) : '';
    if (!/^[0-9]{1,20}$/.test(id)) return undefined;
    const titre = typeof p.title === 'string' ? p.title.trim().slice(0, 300) : '';
    if (!titre) return undefined;
    const image = typeof p.image === 'string' && p.image.length <= 300 ? p.image : '';
    return [id, titre, image];
  };
  (async () => {
    try {
      // 1. Le compte connecté : seulement `isLoggedIn` et `userId` (réduit aux chiffres).
      const u = await lireJson('/userData.json');
      const connecte = !!u && typeof u === 'object' && u.isLoggedIn === true;
      const userId = connecte ? String(u.userId == null ? '' : u.userId).replace(/[^0-9]/g, '') : '';
      if (!connecte) return absent('deconnecte');
      if (!userId || userId.length > 20) return absent('compte-illisible');
      // 2. Toutes les pages, jeux visibles PUIS jeux cachés ; un jeu vu deux fois n'est gardé qu'une fois.
      const jeux = new Map();
      let demandes = 0;
      // 0.217.1 (Seb 21/11 : deux jeux de son compte manquaient) : le bilan de chaque passe, pour le journal — des
      // nombres et les numéros des lignes écartées, jamais le compte.
      const bilan = [];
      for (const cache of [0, 1]) {
        let total = 1;
        const b = { pages: 0, annonces: -1, lignes: 0, gardes: 0, nonJeux: [], illisibles: 0 };
        bilan.push(b);
        for (let page = 1; page <= total; page++) {
          if (demandes > 0) {
            // Toutes les 5 pages, la page dit au cœur qu'elle avance (son délai sans nouvelle repart) ; toujours AVANT une
            // pause, pour que deux remises ne se suivent jamais de trop près.
            if (demandes % PAGES_PAR_NOUVELLE === 0) remettre('gog-progres', String(demandes));
            await pause(250);
          }
          demandes++;
          const v = await lireJson('/account/getFilteredProducts?mediaType=1&sortBy=title&hiddenFlag=' + cache + '&page=' + page);
          if (!v || typeof v !== 'object' || !Array.isArray(v.products)) throw new Arret('page-changee');
          const t = Number(v.totalPages);
          if (!Number.isInteger(t) || t < 0) throw new Arret('page-changee');
          if (page === 1) {
            if (t > PLAFOND_PAGES) throw new Arret('trop-de-pages');
            total = t;
            const annonce = Number(v.totalProducts);
            if (Number.isInteger(annonce) && annonce >= 0) b.annonces = annonce;
          }
          b.pages++;
          let lisibles = 0;
          for (const p of v.products) {
            b.lignes++;
            const j = garder(p);
            if (j === undefined) {
              b.illisibles++;
              continue;
            }
            lisibles++;
            if (j === null) {
              const id = p && (typeof p.id === 'number' || typeof p.id === 'string') ? String(p.id).replace(/[^0-9]/g, '') : '';
              if (id && b.nonJeux.length < 15) b.nonJeux.push(id);
              continue;
            }
            b.gardes++;
            if (!jeux.has(j[0])) jeux.set(j[0], j);
          }
          // Des lignes, mais aucune lisible : la page a changé (jamais « zéro jeu » sur une page qu'on ne comprend plus).
          if (v.products.length > 0 && lisibles === 0) throw new Arret('page-changee');
        }
      }
      // Le bilan d'abord, puis une pause (deux remises ne se suivent jamais de trop près).
      remettre(
        'gog-bilan',
        bilan
          .map((b, i) => (i === 0 ? 'visibles' : 'caches') + '-pages-' + b.pages + '-annonces-' + b.annonces + '-lignes-' + b.lignes + '-gardes-' + b.gardes + '-illisibles-' + b.illisibles + '-nonjeux-' + (b.nonJeux.join('-') || 'aucun'))
          .join('--') + '--total-' + jeux.size,
      );
      await pause(400);
      // 3. La remise par morceaux, demandés un à un par le cœur.
      const liste = [...jeux.values()];
      const n = Math.max(1, Math.ceil(liste.length / PAR_MORCEAU));
      window.__frogtendGog = (k) => {
        if (!Number.isInteger(k) || k < 0 || k >= n) return;
        remettre('gog-jeux', k + '/' + n + '/' + encoder(JSON.stringify(liste.slice(k * PAR_MORCEAU, (k + 1) * PAR_MORCEAU))));
      };
      remettre('gog-compte', userId);
    } catch (e) {
      absent(e instanceof Arret ? e.etape : 'erreur-script');
    }
  })();
})();
