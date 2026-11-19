// Frogtend — voir si la page OFFICIELLE de la boutique Steam montre un compte connecté (lot C1, décision de Seb du
// 19/11). La personne se connecte elle-même sur la vraie page (Steam Guard compris) : ce script ne clique sur RIEN et
// ne lit RIEN d'autre que deux champs de `#application_config` (attribut `data-userinfo`) : `logged_in` et `steamid`.
// Il ne lit JAMAIS `data-store_user_config` (qui porte un jeton) ni aucun cookie.
// Le résultat est rendu par le TITRE de la page (comme les autres scripts des boutiques) :
// « FROGTEND:STEAM:{"connecte":true,"steamid":"7656119…"} » ou « FROGTEND:STEAM:{"connecte":false} ».
// Le cœur revérifie le `steamid` (17 chiffres) : le script n'est jamais cru sur parole.
// Toute erreur est attrapée (sous Windows, une exception d'un script injecté est avalée sans bruit) : en cas de doute,
// le script ne dit rien.
(function () {
  try {
    if (window.__frogtendSteam) return;
    window.__frogtendSteam = true;
    if (location.protocol !== 'https:' || location.hostname !== 'store.steampowered.com') return;
  } catch (e) {
    return;
  }
  const dire = (r) => {
    try {
      document.title = ('FROGTEND:STEAM:' + JSON.stringify(r)).slice(0, 200);
    } catch (e) {
      // Rien : le cœur ne recevra simplement pas de résultat.
    }
  };
  // Lit l'état du compte ; null = la page ne le dit pas (encore).
  const lire = () => {
    try {
      const c = document.getElementById('application_config');
      if (!c) return null;
      const brut = c.getAttribute('data-userinfo');
      if (!brut) return null;
      const u = JSON.parse(brut);
      if (!u || typeof u !== 'object') return null;
      if (u.logged_in === true) {
        const id = String(u.steamid || '');
        return /^[0-9]{17}$/.test(id) ? { connecte: true, steamid: id } : null;
      }
      return { connecte: false };
    } catch (e) {
      return null;
    }
  };
  let essais = 0;
  const regarder = () => {
    const r = lire();
    if (r) return dire(r);
    // La configuration arrive avec la page ; on laisse 10 s au plus à une page lente, puis on se tait.
    if (++essais < 20) setTimeout(regarder, 500);
  };
  regarder();
})();
