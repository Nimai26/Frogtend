// Frogtend — les jeux offerts de Prime Gaming (gaming.amazon.com), page OFFICIELLE, connexion propre au profil.
// Méthode relevée dans vogler/free-games-claimer (rien n'est copié) : la liste « Games » ; les offres à prendre SUR
// Amazon ont un bouton « Claim » (data-a-target="FGWPOffer") ; celles d'autres boutiques (code GOG, Legacy Games,
// compte Epic/EA à relier) sont des liens : Frogtend les compte et laisse la personne finir sur la page.
// Résultat par le TITRE : « FROGTEND:{json} » avec etat = faite | connexion | pas_abonne | erreur.
(function () {
  if (window.__frogtend || location.hostname !== 'gaming.amazon.com') return;
  window.__frogtend = true;
  const dire = (o) => {
    document.title = 'FROGTEND:' + JSON.stringify(o);
  };
  const pause = (ms) => new Promise((ok) => setTimeout(ok, ms));
  const texte = (e) => ((e && (e.innerText || e.textContent)) || '').trim();
  const boutons = () => [...document.querySelectorAll('button')];
  (async () => {
    try {
      const connecte = () => document.querySelector('[data-a-target="user-dropdown-first-name-text"]');
      for (let i = 0; i < 40 && !connecte() && !boutons().some((b) => texte(b) === 'Sign in'); i++) await pause(500);
      if (!connecte()) return dire({ etat: 'connexion' });
      if (boutons().some((b) => /Try Prime/i.test(texte(b)))) return dire({ etat: 'pas_abonne' });
      const jeux = document.querySelector('button[data-type="Game"]');
      if (jeux) {
        jeux.click();
        await pause(2500);
      }
      const liste = () => document.querySelector('div[data-a-target="offer-list-FGWP_FULL"]') || document;
      const obtenus = [];
      // Les offres SUR Amazon : un clic sur « Claim », une par une (la liste se met à jour après chaque clic).
      for (let i = 0; i < 30; i++) {
        const b = [...liste().querySelectorAll('.item-card__action button[data-a-target="FGWPOffer"]')].find((x) => /Claim/i.test(texte(x)));
        if (!b) break;
        const carte = b.closest('.item-card') || b.closest('[class*="item-card"]');
        const titre = texte(carte && carte.querySelector('.item-card-details__body__primary')) || 'jeu';
        b.click();
        await pause(3000);
        obtenus.push(titre);
      }
      const externes = liste().querySelectorAll('.item-card__action a[data-a-target="FGWPOffer"]').length;
      dire({ etat: 'faite', obtenus, deja: 0, a_finir: externes });
    } catch (e) {
      dire({ etat: 'erreur', motif: String((e && e.message) || e).slice(0, 120) });
    }
  })();
})();
