// Frogtend — obtenir un jeu offert sur la page OFFICIELLE d'Epic (lot 9, décision de Seb : automatique d'abord,
// sinon la personne finit elle-même). Méthode inspirée de vogler/free-games-claimer (rien n'est copié) : la page est
// en anglais (/en-US/) pour des libellés stables. Le résultat est rendu par le TITRE de la page (200 caractères au plus) :
// « FROGTEND:CONNEXION » (pas connecté), « FROGTEND:DEJA » (déjà dans la bibliothèque), « FROGTEND:OBTENU »,
// « FROGTEND:CAPTCHA » (vérification humaine), « FROGTEND:CONNECTE » (fenêtre de connexion : compte connecté),
// « FROGTEND:ERREUR:<étape> | <textes des boutons visibles> » (pour comprendre la page la prochaine fois).
// 18/11 : « The New Epic Games Store » a changé la page (`egs-navigation` absent, Knightica restait « délai dépassé »
// alors qu'elle était « In Library ») : l'état du compte et les boutons se repèrent aussi par leur TEXTE, les anciens
// repères restant essayés d'abord.
// GARDE : le script ne clique JAMAIS sur un bouton qui coûte de l'argent (prix non nul, « Buy »…) : tous ses clics
// passent par `cliquer`, qui le refuse.
// Mode connexion (`window.__frogtendMode = 'connexion'`, fenêtre « Se connecter ») : aucun clic ; il dit seulement
// « CONNECTE » quand la boutique montre le compte connecté.
// Mode vérification (`window.__frogtendMode = 'verification'`, page CACHÉE ouverte avec 🛒 Boutiques, Seb 20/11) :
// aucun clic ; il dit « CONNECTE » ou « CONNEXION » selon ce que la boutique montre, et rien s'il n'est pas sûr.
(function () {
  if (window.__frogtend) return;
  window.__frogtend = true;
  const modeConnexion = window.__frogtendMode === 'connexion';
  const modeVerification = window.__frogtendMode === 'verification';
  const dire = (m) => {
    document.title = ('FROGTEND:' + m).slice(0, 200);
  };
  const net = (s) => String(s || '').replace(/\s+/g, ' ').trim().toLowerCase();
  const texte = (e) => net(e && (e.innerText || e.textContent));

  // La page et ses cadres de la même origine (le cadre d'achat d'Epic).
  const documents = () => {
    const l = [document];
    for (const f of document.querySelectorAll('iframe')) {
      try {
        if (f.contentDocument) l.push(f.contentDocument);
      } catch (e) {
        // Cadre d'une autre origine : illisible, on l'ignore.
      }
    }
    return l;
  };
  const visible = (e) => {
    for (let n = e; n && n.nodeType === 1; n = n.parentElement) {
      if (n.hidden) return false;
      const s = (n.ownerDocument.defaultView || window).getComputedStyle(n);
      if (s.display === 'none' || s.visibility === 'hidden') return false;
    }
    return true;
  };
  const cliquables = () =>
    documents().flatMap((d) => [...d.querySelectorAll('button, [role="button"], a, [data-testid*="cta" i]')]);

  // Un mot d'achat payant, ou un prix non nul (« €19.99 », « 9,99 € », « $5 »).
  const payant = (t) => {
    if (/\b(buy|purchase|pre-?purchase|pre-?order|add to cart|acheter)\b/.test(t)) return true;
    const prix = t.match(/(?:[€$£¥]|\b(?:eur|usd|gbp)\b)\s*\d+(?:[.,]\d+)?|\d+(?:[.,]\d+)?\s*(?:[€$£¥]|\b(?:eur|usd|gbp)\b)/g) || [];
    return prix.some((p) => parseFloat(p.replace(/[^\d.,]/g, '').replace(',', '.')) > 0);
  };
  const cliquer = (e) => {
    if (payant(texte(e))) throw { etape: 'bouton payant refusé (« ' + texte(e).slice(0, 30) + ' »)' };
    e.click();
  };

  // La sorte d'un bouton d'achat : `payant` (jamais cliqué), `deja` (dans la bibliothèque), `prendre` (gratuit).
  const sorte = (e) => {
    const t = texte(e);
    if (!t || t.length > 60) return null;
    if (payant(t)) return 'payant';
    if (/\b(in (your )?library|owned)\b/.test(t) && !/\bnot\b/.test(t)) return 'deja';
    if (/^(get|free|get free|get it free|get for free)$/.test(t)) return 'prendre';
    return null;
  };
  // Un bouton posé sur la carte d'un AUTRE jeu (carrousel, « vous aimerez aussi ») : un lien vers une autre page /p/.
  const slugDe = (href) => (String(href || '').match(/\/p\/([^/?#]+)/) || [])[1] || null;
  const autreJeu = (e) => {
    const ici = slugDe(location.pathname);
    for (let n = e; n && n.nodeType === 1; n = n.parentElement) {
      if (n.tagName === 'A' && slugDe(n.getAttribute('href')) && slugDe(n.getAttribute('href')) !== ici) return true;
    }
    return false;
  };
  // Le bouton d'achat : l'ancien repère d'abord, sinon par le texte, dans la page du jeu affiché seulement (jamais sur la
  // carte d'un autre jeu). Plusieurs « Get » possibles : on ne choisit PAS au hasard (aucun clic).
  const achat = () => {
    const ancien = document.querySelector('button[data-testid="purchase-cta-button"]');
    if (ancien && sorte(ancien) && visible(ancien)) return { b: ancien, sorte: sorte(ancien) };
    const l = [];
    for (const e of cliquables()) {
      const s = sorte(e);
      if (s && visible(e) && !autreJeu(e)) l.push({ b: e, sorte: s });
    }
    const prendre = l.filter((x) => x.sorte === 'prendre');
    if (prendre.length > 1) throw { etape: "plusieurs boutons d'achat" };
    return prendre[0] || l.find((x) => x.sorte === 'deja') || l.find((x) => x.sorte === 'payant') || null;
  };

  // L'état du compte : `oui`, `non`, ou null (pas sûr).
  const SIGNES_COMPTE =
    '[data-testid*="avatar" i], [class*="avatar" i], [data-testid*="user-menu" i], [data-testid*="account" i], [aria-label*="account" i], a[href*="/account/personal"]';
  const compte = () => {
    const nav = document.querySelector('egs-navigation');
    const v = nav && nav.getAttribute('isloggedin');
    if (v === 'true') return 'oui';
    if (v === 'false') return 'non';
    const connexion = cliquables().some((e) => {
      const t = texte(e);
      return (/^(sign in|sign-in|log in|login|se connecter)$/.test(t) || (t.length < 30 && e.matches('a[href*="/id/login"]'))) && visible(e);
    });
    if (connexion) return 'non';
    if ([...document.querySelectorAll(SIGNES_COMPTE)].some(visible)) return 'oui';
    return null;
  };

  const captcha = () =>
    documents().some((d) => !!d.querySelector('iframe[src*="hcaptcha"], iframe[title*="hCaptcha"], .h_captcha_challenge'));
  // Attend qu'une condition rende quelque chose (null au bout de `ms`) ; s'arrête net sur un captcha.
  const attendre = (f, ms) =>
    new Promise((ok, ko) => {
      const debut = Date.now();
      const tour = () => {
        if (captcha()) return ko('captcha');
        let r = null;
        try {
          r = f();
        } catch (e) {
          if (e && e.etape) return ko(e);
          r = null;
        }
        if (r) return ok(r);
        if (Date.now() - debut > ms) return ok(null);
        setTimeout(tour, 300);
      };
      tour();
    });
  const pause = (ms) => new Promise((ok) => setTimeout(ok, ms));

  // Les boutons visibles, en quelques mots, pour le journal (jamais une adresse électronique).
  const resume = () => {
    const vus = [];
    for (const d of documents()) {
      for (const e of d.querySelectorAll('button, [role="button"]')) {
        const t = (texte(e) || net(e.getAttribute('aria-label'))).slice(0, 24);
        if (t && !t.includes('@') && !vus.includes(t) && visible(e)) vus.push(t);
        if (vus.length >= 12) break;
      }
    }
    return vus.length ? vus.join(' ; ') : 'aucun bouton visible';
  };
  // L'étape (en français, montrée à la personne) ; après « | », le détail et les boutons vus (pour le journal seulement).
  const echec = (etape, detail) => dire('ERREUR:' + etape + ' | ' + (detail ? detail + ' ; ' : '') + resume());

  const cadreAchat = () => document.querySelector('#webPurchaseContainer iframe, iframe[src*="/purchase"]');
  const merci = () => documents().some((d) => /thanks? (you )?for your order/.test(net(d.body && d.body.textContent)));
  // Le total de la commande (0 attendu) : dans le cadre d'achat, ou la fenêtre qui porte « Place Order ». `null` : pas
  // trouvé (et alors on ne commande PAS : jamais « pas vu = gratuit »).
  const totalCommande = (b) => {
    const zone = b.ownerDocument !== document ? b.ownerDocument.body : b.closest('[role="dialog"], form');
    // TOUS les « Total » (jamais « Subtotal » : la lettre avant est exclue) ; le plus grand compte. Un seul total
    // payant suffit à refuser (« Subtotal 0 € » puis « Total 4,99 € » n'est jamais pris pour gratuit).
    const montants = [...net(zone && zone.textContent).matchAll(/(?:^|[^a-z])total[^0-9€$£]{0,20}((?:[€$£]\s*)?\d+(?:[.,]\d+)?)/g)]
      .map((m) => parseFloat(m[1].replace(/[^\d.,]/g, '').replace(',', '.')));
    if (!montants.length || montants.some((v) => !Number.isFinite(v))) return null;
    return Math.max(...montants);
  };

  const surveillerConnexion = async () => {
    if (location.hostname !== 'store.epicgames.com') return;
    if (await attendre(() => (compte() === 'oui' ? 'oui' : null), 120000)) dire('CONNECTE');
  };

  // Le vrai état du compte, sans rien cliquer. « Non » n'est cru que si Epic le DIT (`egs-navigation
  // isloggedin="false"` pendant 8 s) ou renvoie vers sa page de connexion : le seul texte « Sign In » peut être
  // passager sur une ligne lente (revue du 20/11), il ne décide rien. Pas sûr au bout de 20 s : rien n'est dit.
  const verifierCompte = async () => {
    if (location.hostname !== 'store.epicgames.com') {
      if (/(^|\.)epicgames\.com$/.test(location.hostname) && /^\/id\/login/.test(location.pathname)) dire('CONNEXION');
      return;
    }
    const ditNon = () => {
      const nav = document.querySelector('egs-navigation');
      return !!nav && nav.getAttribute('isloggedin') === 'false';
    };
    let nonDepuis = 0;
    const r = await attendre(() => {
      if (compte() === 'oui') return 'oui';
      if (!ditNon()) return (nonDepuis = 0), null;
      nonDepuis = nonDepuis || Date.now();
      return Date.now() - nonDepuis >= 8000 ? 'non' : null;
    }, 20000);
    if (r === 'oui') dire('CONNECTE');
    else if (r === 'non') dire('CONNEXION');
  };

  const obtenir = async () => {
    if (location.hostname !== 'store.epicgames.com') {
      // Epic a renvoyé vers sa page de connexion : pas connecté.
      if (/(^|\.)epicgames\.com$/.test(location.hostname) && /\/id\//.test(location.pathname)) return dire('CONNEXION');
      return echec('page inattendue', location.hostname);
    }
    // 1. Le bouton d'achat, ou un signe clair que le compte n'est pas connecté (8 s sans bouton suffisent).
    let nonDepuis = 0;
    const trouver = () => {
      const a = achat();
      if (a) return a;
      if (compte() === 'non') {
        nonDepuis = nonDepuis || Date.now();
        if (Date.now() - nonDepuis > 8000) return { sorte: 'connexion' };
      } else nonDepuis = 0;
      return null;
    };
    let a = await attendre(trouver, 30000);
    // Un « Get » peut s'afficher avant que la page sache le jeu possédé : on le relit un instant plus tard.
    if (a && a.sorte === 'prendre') {
      await pause(1500);
      a = achat() || a;
    }
    if (a && a.sorte === 'deja') return dire('DEJA');
    if (!a) return compte() === 'non' ? dire('CONNEXION') : echec("pas de bouton d'achat");
    if (a.sorte === 'connexion' || compte() === 'non') return dire('CONNEXION');
    if (a.sorte === 'payant') return echec('pas gratuit');
    const bouton = a.b;

    // 2. Une page d'âge (« Continue ») peut précéder ; puis « Get ».
    const continuer = cliquables().find((b) => texte(b) === 'continue' && visible(b));
    if (continuer) cliquer(continuer);
    cliquer(bouton);

    // Contrat de licence d'Epic, quand il est demandé.
    attendre(() => documents().map((d) => d.querySelector('input#agree')).find(Boolean), 8000)
      .then((c) => {
        if (!c) return;
        c.click();
        const accepter = cliquables().find((b) => /\baccept\b/.test(texte(b)) && visible(b));
        if (accepter) cliquer(accepter);
      })
      .catch(() => {});

    // 3. La commande (à 0 €), dans un cadre de la même origine ; certains jeux passent sans commande.
    const fini = () => {
      if (cadreAchat()) return false;
      if (bouton.isConnected) return sorte(bouton) === 'deja';
      const x = achat();
      return !!x && x.sorte === 'deja';
    };
    const commande = await attendre(() => {
      if (fini()) return 'fini';
      for (const d of documents()) {
        const b = [...d.querySelectorAll('button')].find(
          (x) => texte(x).includes('place order') && !x.disabled && !x.querySelector('.payment-loading--loading') && visible(x),
        );
        if (b) return b;
      }
      return null;
    }, 45000);
    if (!commande) return echec('pas de commande');
    if (commande !== 'fini') {
      const total = totalCommande(commande);
      if (total === null) return echec('total introuvable');
      if (total !== 0) return echec('commande payante refusée');
      cliquer(commande);
      // Comptes européens : « I Agree » après la commande.
      attendre(() => cliquables().find((b) => texte(b).includes('i agree') && visible(b)), 10000)
        .then((b) => b && cliquer(b))
        .catch(() => {});
      // 4. La confirmation : le bouton passe à « In Library », ou Epic remercie pour la commande.
      if (!(await attendre(() => (fini() || merci() ? 'ok' : null), 60000))) return echec('pas de confirmation');
    }
    dire('OBTENU');
  };

  (async () => {
    try {
      await (modeVerification ? verifierCompte() : modeConnexion ? surveillerConnexion() : obtenir());
    } catch (e) {
      // En mode vérification, rien n'est sûr : rien à dire (l'état connu est gardé).
      if (modeVerification) return;
      // En mode connexion, la personne résout elle-même la vérification : rien à dire.
      if (e === 'captcha') return modeConnexion ? undefined : dire('CAPTCHA');
      if (e && e.etape) return echec(e.etape);
      echec('imprévu', net((e && e.message) || e).slice(0, 40));
    }
  })();
})();
