// Frogtend — obtenir un jeu offert sur la page OFFICIELLE d'Epic (lot 9, décision de Seb : automatique d'abord,
// sinon la personne finit elle-même). Méthode inspirée de vogler/free-games-claimer (rien n'est copié) : la page est
// en anglais (/en-US/) pour des libellés stables. Le résultat est rendu par le TITRE de la page :
// « FROGTEND:CONNEXION » (pas connecté), « FROGTEND:DEJA » (déjà dans la bibliothèque), « FROGTEND:OBTENU »,
// « FROGTEND:CAPTCHA » (vérification humaine), « FROGTEND:ERREUR:<motif> ».
(function () {
  if (window.__frogtend) return;
  window.__frogtend = true;
  const dire = (m) => {
    document.title = 'FROGTEND:' + m;
  };
  const texte = (e) => ((e && (e.innerText || e.textContent)) || '').trim().toLowerCase();
  const boutons = (racine) => [...(racine || document).querySelectorAll('button')];
  const cadreAchat = () => document.querySelector('#webPurchaseContainer iframe');
  const docAchat = () => {
    try {
      return cadreAchat() && cadreAchat().contentDocument;
    } catch (e) {
      return null;
    }
  };
  const captcha = () =>
    !!document.querySelector('iframe[src*="hcaptcha"], iframe[title*="hCaptcha"], .h_captcha_challenge') ||
    !!(docAchat() && docAchat().querySelector('iframe[src*="hcaptcha"], .h_captcha_challenge'));
  // Attend qu'une condition rende quelque chose ; s'arrête net sur un captcha.
  const attendre = (f, ms) =>
    new Promise((ok, ko) => {
      const debut = Date.now();
      const minuterie = setInterval(() => {
        if (captcha()) {
          clearInterval(minuterie);
          return ko('captcha');
        }
        let r = null;
        try {
          r = f();
        } catch (e) {
          r = null;
        }
        if (r) {
          clearInterval(minuterie);
          ok(r);
        } else if (Date.now() - debut > ms) {
          clearInterval(minuterie);
          ko('délai dépassé');
        }
      }, 300);
    });

  (async () => {
    try {
      const nav = await attendre(() => document.querySelector('egs-navigation'), 30000);
      await attendre(() => nav.getAttribute('isloggedin') !== null, 15000).catch(() => {});
      if (nav.getAttribute('isloggedin') !== 'true') return dire('CONNEXION');

      const cta = () => {
        const b = document.querySelector('button[data-testid="purchase-cta-button"]');
        return b && /[a-z]/.test(texte(b)) ? b : null;
      };
      const bouton = await attendre(cta, 30000);
      const t = texte(bouton);
      if (t.includes('in library') || t.includes('owned')) return dire('DEJA');
      if (!t.includes('get')) return dire('ERREUR:bouton « ' + t + ' »');

      // Une page d'âge (« Continue ») peut précéder.
      const continuer = boutons().find((b) => texte(b) === 'continue');
      if (continuer) continuer.click();
      bouton.click();

      // Contrat de licence d'Epic, quand il est demandé.
      attendre(() => document.querySelector('input#agree'), 6000)
        .then((c) => {
          c.click();
          const accepter = boutons().find((b) => texte(b).includes('accept'));
          if (accepter) accepter.click();
        })
        .catch(() => {});

      // La commande (à 0 €) se passe dans un cadre de la même origine.
      const commander = await attendre(() => {
        const d = docAchat();
        if (!d) return null;
        return boutons(d).find((b) => texte(b).includes('place order') && !b.querySelector('.payment-loading--loading'));
      }, 45000);
      commander.click();
      // Comptes européens : « I Agree » après la commande.
      attendre(() => {
        const d = docAchat();
        return d && boutons(d).find((b) => texte(b).includes('i agree'));
      }, 10000)
        .then((b) => b.click())
        .catch(() => {});

      await attendre(() => {
        const b = cta();
        return !cadreAchat() && b && texte(b).includes('in library') ? 'ok' : null;
      }, 60000);
      dire('OBTENU');
    } catch (e) {
      dire(e === 'captcha' ? 'CAPTCHA' : 'ERREUR:' + ((e && e.message) || e));
    }
  })();
})();
