// Frogtend — ajouter les jeux PS Plus du mois à la bibliothèque PlayStation de la personne (demande de Seb, 02/10 :
// optionnel par profil ; ces jeux ne vont PAS dans la ludothèque). Page OFFICIELLE du PlayStation Store, connexion
// propre au profil. Règle de sûreté : le script ne clique QUE sur « Ajouter à la bibliothèque » (jamais sur un prix,
// un panier ou un abonnement) : il ne peut rien acheter.
// Déroulé : la page de la catégorie PS Plus donne la liste des jeux ; le script les visite un par un (la file vit
// dans sessionStorage, le script est réinjecté à chaque page). Résultat par le TITRE de la page :
// « FROGTEND:CONNEXION », « FROGTEND:PSPLUS:<ajoutés>:<déjà>:<vus> », « FROGTEND:ERREUR:<motif> ».
(function () {
  if (window.__frogtend || location.hostname !== 'store.playstation.com') return;
  window.__frogtend = true;
  const CLE = 'frogtend-psplus';
  const MAX_JEUX = 30;
  const dire = (m) => {
    document.title = 'FROGTEND:' + m;
  };
  const texte = (e) => ((e && (e.innerText || e.textContent)) || '').trim().toLowerCase();
  const lireEtat = () => {
    try {
      return JSON.parse(sessionStorage.getItem(CLE) || 'null');
    } catch (e) {
      return null;
    }
  };
  const ecrireEtat = (e) => sessionStorage.setItem(CLE, JSON.stringify(e));
  const attendre = (f, ms) =>
    new Promise((ok, ko) => {
      const debut = Date.now();
      const minuterie = setInterval(() => {
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
      }, 400);
    });
  const pause = (ms) => new Promise((ok) => setTimeout(ok, ms));
  const connecte = () => !document.querySelector('[data-qa="web-toolbar#signin-button"]');
  // Les boutons d'action de la page d'un jeu (zone d'achat officielle « mfeCtaMain »).
  const actions = () => [...document.querySelectorAll('[data-qa^="mfeCtaMain"] button, [data-qa^="mfeCtaMain"] a')];
  const estAjout = (b) => /ajouter à la bibliothèque|add to library/.test(texte(b));
  const estDeja = (b) => /dans la bibliothèque|in library|télécharger|download|^jouer$|^play$|acheté|purchased/.test(texte(b));
  const suivant = (e) => {
    e.i++;
    ecrireEtat(e);
    if (e.i < e.file.length) location.href = e.file[e.i];
    else {
      sessionStorage.removeItem(CLE);
      dire(`PSPLUS:${e.ajoutes}:${e.deja}:${e.file.length}`);
    }
  };

  (async () => {
    try {
      let e = lireEtat();
      if (!e) {
        // 1) La liste : les jeux de la catégorie PS Plus.
        await pause(3000);
        if (!connecte()) return dire('CONNEXION');
        const liens = await attendre(() => {
          const l = [...document.querySelectorAll('main a[href*="/product/"], main a[href*="/concept/"]')];
          return l.length ? l : null;
        }, 30000);
        const file = [...new Set(liens.map((a) => a.href.split('?')[0]))].slice(0, MAX_JEUX);
        e = { file, i: -1, ajoutes: 0, deja: 0 };
        return suivant(e);
      }
      // 2) Un jeu de la file.
      await pause(2500);
      if (!connecte()) {
        sessionStorage.removeItem(CLE);
        return dire('CONNEXION');
      }
      const bouton = await attendre(() => {
        const l = actions();
        return l.find(estAjout) || l.find(estDeja) || null;
      }, 12000).catch(() => null);
      if (bouton && estAjout(bouton)) {
        bouton.click();
        await attendre(() => !actions().some(estAjout) || actions().some(estDeja), 15000).catch(() => {});
        if (!actions().some(estAjout)) e.ajoutes++;
      } else if (bouton) {
        e.deja++;
      }
      suivant(e);
    } catch (err) {
      sessionStorage.removeItem(CLE);
      dire('ERREUR:' + ((err && err.message) || err));
    }
  })();
})();
