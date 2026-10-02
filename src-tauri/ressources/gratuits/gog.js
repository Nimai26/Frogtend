// Frogtend — le jeu offert de GOG (giveaway), sur la page OFFICIELLE, connexion propre au profil. Méthode relevée dans
// vogler/free-games-claimer (rien n'est copié) : la bannière #giveaway annonce le jeu ; l'adresse officielle
// /giveaway/claim l'ajoute à la bibliothèque (« {} ») ou répond « Already claimed ».
// Résultat par le TITRE : « FROGTEND:{json} » avec etat = faite | connexion | aucun | erreur.
(function () {
  if (window.__frogtend || location.hostname !== 'www.gog.com') return;
  window.__frogtend = true;
  const dire = (o) => {
    document.title = 'FROGTEND:' + JSON.stringify(o);
  };
  const pause = (ms) => new Promise((ok) => setTimeout(ok, ms));
  (async () => {
    try {
      // Laisser la page se construire (menu du compte ou bouton « Sign in »).
      // La bannière : #giveaway, ou un bloc « giveaway__… » (la page a changé de noms de classes avec le temps).
      const trouverBanniere = () => document.querySelector('#giveaway') || document.querySelector('[class*="giveaway__"]:not(style)');
      for (let i = 0; i < 30 && !document.querySelector('#menuUsername'); i++) await pause(500);
      if (!document.querySelector('#menuUsername')) return dire({ etat: 'connexion' });
      await pause(1500);
      const banniere = trouverBanniere();
      if (!banniere) return dire({ etat: 'aucun' });
      const zone = banniere.closest('#giveaway') || banniere.parentElement || banniere;
      const entete = (document.querySelector('.giveaway__content-header') || zone.querySelector('h1, h2, h3, [class*="header"]') || zone).innerText || '';
      const m = entete.match(/Claim (.*) and don't miss the|Success! (.*) was added to/);
      const titre = (m && (m[1] || m[2])) || entete.trim().slice(0, 80);
      const r = await fetch('/giveaway/claim', { credentials: 'same-origin' });
      const texte = (await r.text()).trim();
      if (texte === '{}') return dire({ etat: 'faite', obtenus: [titre], deja: 0 });
      let message = texte;
      try {
        message = JSON.parse(texte).message || texte;
      } catch (e) {}
      if (message === 'Already claimed') return dire({ etat: 'faite', obtenus: [], deja: 1, titres_deja: [titre] });
      dire({ etat: 'erreur', motif: String(message).slice(0, 120) });
    } catch (e) {
      dire({ etat: 'erreur', motif: String((e && e.message) || e).slice(0, 120) });
    }
  })();
})();
