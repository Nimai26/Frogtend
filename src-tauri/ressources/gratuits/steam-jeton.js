// Frogtend — lire le jeton d'accès Steam du compte connecté (lot C2, 0.215.2), dans la fenêtre CACHÉE ouverte sur la
// boutique (navigateur propre au profil). Seb 20/11 : demandé par le cœur avec les cookies, Steam répondait 302 ; la
// page, elle, a le jeton dans sa configuration (`#application_config` `data-store_user_config` : `webapi_token`).
// Le jeton est remis au cœur par une navigation vers https://frogtend.invalid/… que le cœur INTERCEPTE et ANNULE : la
// page n'est jamais chargée, rien n'entre dans l'historique, rien ne passe par le titre.
//   https://frogtend.invalid/steam-jeton#<jeton>   un jeton lu ;
//   https://frogtend.invalid/steam-absent#<étape>  pas de jeton (étape en quelques mots, pour le journal).
// Ce script ne clique sur rien, ne lit aucun cookie, n'envoie rien ailleurs qu'au cœur.
(function () {
  try {
    if (window.__frogtendSteamJeton) return;
    window.__frogtendSteamJeton = true;
    if (location.protocol !== 'https:' || location.hostname !== 'store.steampowered.com') return;
  } catch (e) {
    return;
  }
  const FORME = /^[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$/;
  const remettre = (chemin, fragment) => {
    try {
      location.replace('https://frogtend.invalid/' + chemin + '#' + fragment);
    } catch (e) {
      // Rien : le cœur attendra puis dira « pas de réponse ».
    }
  };
  // 1. Le jeton déjà dans la page.
  const depuisLaPage = () => {
    try {
      const el = document.getElementById('application_config');
      const brut = el && el.getAttribute('data-store_user_config');
      if (!brut) return null;
      const v = JSON.parse(brut);
      const t = v && v.webapi_token;
      return typeof t === 'string' && FORME.test(t) ? t : null;
    } catch (e) {
      return null;
    }
  };
  // 0.216.0 (lot C3) : ce que la page dit du compte, pour distinguer « reconnecte-toi » d'une page qui a changé.
  //   'connecte' / 'deconnecte' : la page le dit ; 'sans-configuration' : la page n'a plus sa configuration (page
  //   changée, maintenance, vérification…).
  const etatDuCompte = () => {
    try {
      const el = document.getElementById('application_config');
      if (!el) return 'sans-configuration';
      const brut = el.getAttribute('data-userinfo');
      if (!brut) return 'sans-configuration';
      const u = JSON.parse(brut);
      return u && u.logged_in === true ? 'connecte' : 'deconnecte';
    } catch (e) {
      return 'sans-configuration';
    }
  };
  const compte = etatDuCompte();
  const t = depuisLaPage();
  if (t) return remettre('steam-jeton', t);
  // 2. Sinon, demandé à la boutique PAR LA PAGE (mêmes cookies, même navigateur que Steam lui-même).
  fetch('/pointssummary/ajaxgetasyncconfig', { credentials: 'same-origin', cache: 'no-store' })
    .then(async (r) => {
      let j = null;
      try {
        j = await r.json();
      } catch (e) {
        j = null;
      }
      const t2 = j && j.data && j.data.webapi_token;
      if (typeof t2 === 'string' && FORME.test(t2)) return remettre('steam-jeton', t2);
      const succes = j ? '-success-' + String(j.success).replace(/[^0-9a-z]/gi, '') : '-pas-json';
      remettre('steam-absent', compte + '-statut-' + r.status + (r.redirected ? '-redirige' : '') + succes);
    })
    .catch(() => remettre('steam-absent', compte + '-erreur-reseau'));
})();
