<script lang="ts">
  // ⚡ Raccourcis en jeu (✅ Seb 11/11 : « une rubrique dans les options pour définir les raccourcis manette ET clavier,
  // et qu'ils restent à jour dans le menu des manettes » ; 15/11 : touches clavier d'office). Des listes fermées : un
  // choix déjà pris est grisé, avec ce qui le prend. Tout vaut à la prochaine partie (lu au lancement).
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { confirmer, toast } from '$lib/dialogues/fenetres.svelte';
  import { etat, reglerPc } from '$lib/etat.svelte';
  import { libererBoutonXbox } from '$lib/ludotheque/jeu.svelte';
  import { ETATS_CLAVIER, ETATS_MANETTE, MENU_CLAVIER, MENU_MANETTE, ORIGINE, etatClavier, etatManette, nomClavier, nomManette, prise } from './raccourcis';

  const r = $derived(etat.pc.menuJeu);
  let gameBarPrend = $state<boolean | null>(null);
  onMount(async () => {
    gameBarPrend = await api.gameBarBouton().catch(() => null);
  });
  async function liberer() {
    if (await libererBoutonXbox()) gameBarPrend = await api.gameBarBouton().catch(() => null);
  }

  /** Le libellé d'une option, et « (déjà : …) » si elle est prise. */
  function libelle(pour: Parameters<typeof prise>[1], v: string, nom: (x: string) => string): { texte: string; prise: boolean; deja: string } {
    const p = prise(r, pour, v);
    return { texte: nom(v), prise: !!p, deja: p ? ` (déjà : ${p})` : '' };
  }

  async function regler(cle: string, v: string) {
    await reglerPc(`menuJeu.${cle}`, v);
  }

  async function origine() {
    const oui = await confirmer('↺ Revenir aux raccourcis d’origine ?', {
      message: 'Menu : Vue (⧉) + RB (tenus 1 s) et Pause. Sauver : Vue (⧉) + LT et Ctrl + Maj + F5. Charger : Vue (⧉) + RT et Ctrl + Maj + F9. Le bouton Xbox ouvre le menu.',
      libelleValider: '↺ Revenir à l’origine',
    });
    if (!oui) return;
    for (const [cle, v] of Object.entries(ORIGINE)) await reglerPc(`menuJeu.${cle}`, v);
    await reglerPc('menuJeu.boutonXbox', true);
    await reglerPc('menuJeu.raccourcisEtats', true);
    toast('✅ Raccourcis d’origine remis (dès ta prochaine partie).', 'ok');
  }
</script>

<div class="raccourcis">
  <p class="muted">
    Pendant une partie, ces raccourcis ouvrent le menu de Frogtend ou font une sauvegarde rapide, sans quitter le jeu. Un choix
    déjà pris par un autre raccourci est grisé. Tout change à ta prochaine partie. L’image « 🎮 Les touches » du menu en jeu
    montre toujours ceux réglés ici.
  </p>

  <h3>🎮 À la manette</h3>
  <dl class="cx-kv">
    <dt>Bouton du milieu</dt>
    <dd>
      <select aria-label="Bouton du milieu de la manette" value={r.boutonXbox === false ? 'non' : 'oui'} onchange={(e) => reglerPc('menuJeu.boutonXbox', e.currentTarget.value === 'oui')}>
        <option value="oui">ouvre et ferme le menu (appui court)</option>
        <option value="non">n’ouvre pas le menu</option>
      </select>
      <p class="muted">
        Le bouton Xbox, PS ou Home. Il ne marche pas pour les jeux de console Xbox, PS3, Switch, Switch 2, Wii U, 3DS, PSP et
        Vita : leur émulateur s’en sert. Si Steam l’utilise aussi, cela se règle dans Steam (rubrique Manette).
      </p>
      {#if gameBarPrend === true && r.boutonXbox !== false}
        <p class="muted">
          ⚠ La Game Bar de Windows utilise encore ce bouton : il n’ouvrira pas le menu.
          <button class="btn" onclick={liberer}>🎮 Libérer le bouton Xbox</button>
        </p>
      {/if}
    </dd>
    <dt>Ouvrir le menu</dt>
    <dd>
      <select aria-label="Ouvrir le menu à la manette" value={(MENU_MANETTE as readonly string[]).includes(r.manette) ? r.manette : ORIGINE.manette} onchange={(e) => regler('manette', e.currentTarget.value)}>
        {#each MENU_MANETTE as v (v)}
          {@const l = libelle('manette', v, nomManette)}
          <option value={v} disabled={l.prise}>{l.texte}, tenus 1 s{l.deja}</option>
        {/each}
      </select>
    </dd>
    <dt>Sauvegarde rapide</dt>
    <dd>
      <select aria-label="Sauvegarde rapide à la manette" value={etatManette(r, 'sauver')} onchange={(e) => regler('sauverManette', e.currentTarget.value)}>
        {#each ETATS_MANETTE as v (v)}
          {@const l = libelle('sauverManette', v, nomManette)}
          <option value={v} disabled={l.prise}>{v === 'aucun' ? 'aucun raccourci' : `${l.texte}, tenus ½ s${l.deja}`}</option>
        {/each}
      </select>
    </dd>
    <dt>Chargement rapide</dt>
    <dd>
      <select aria-label="Chargement rapide à la manette" value={etatManette(r, 'charger')} onchange={(e) => regler('chargerManette', e.currentTarget.value)}>
        {#each ETATS_MANETTE as v (v)}
          {@const l = libelle('chargerManette', v, nomManette)}
          <option value={v} disabled={l.prise}>{v === 'aucun' ? 'aucun raccourci' : `${l.texte}, tenus 1 s${l.deja}`}</option>
        {/each}
      </select>
      <p class="muted">
        Dans l’emplacement choisi au menu en jeu (l’ancienne sauvegarde est gardée à côté), pour les émulateurs qui savent le faire.
        Si un de tes jeux utilise déjà ces boutons ensemble, choisis-en d’autres, ou « aucun ».
      </p>
    </dd>
  </dl>

  <h3>⌨ Au clavier</h3>
  <dl class="cx-kv">
    <dt>Ouvrir le menu</dt>
    <dd>
      <select aria-label="Ouvrir le menu au clavier" value={(MENU_CLAVIER as readonly string[]).includes(r.touche) ? r.touche : ORIGINE.touche} onchange={(e) => regler('touche', e.currentTarget.value)}>
        {#each MENU_CLAVIER as v (v)}
          {@const l = libelle('touche', v, nomClavier)}
          <option value={v} disabled={l.prise}>{l.texte}{l.deja}</option>
        {/each}
      </select>
    </dd>
    <dt>Sauvegarde rapide</dt>
    <dd>
      <select aria-label="Sauvegarde rapide au clavier" value={etatClavier(r, 'sauver')} onchange={(e) => regler('sauverClavier', e.currentTarget.value)}>
        {#each ETATS_CLAVIER as v (v)}
          {@const l = libelle('sauverClavier', v, nomClavier)}
          <option value={v} disabled={l.prise}>{v === 'aucune' ? 'aucune touche' : `${l.texte}${l.deja}`}</option>
        {/each}
      </select>
    </dd>
    <dt>Chargement rapide</dt>
    <dd>
      <select aria-label="Chargement rapide au clavier" value={etatClavier(r, 'charger')} onchange={(e) => regler('chargerClavier', e.currentTarget.value)}>
        {#each ETATS_CLAVIER as v (v)}
          {@const l = libelle('chargerClavier', v, nomClavier)}
          <option value={v} disabled={l.prise}>{v === 'aucune' ? 'aucune touche' : `${l.texte}${l.deja}`}</option>
        {/each}
      </select>
      <p class="muted">
        Pendant la partie, le jeu ne reçoit plus ces touches (Frogtend les prend). Elles ne sont actives que si le menu en jeu
        sait sauver et charger pour ce jeu.
      </p>
    </dd>
  </dl>

  <p><button class="btn" onclick={origine}>↺ Revenir aux raccourcis d’origine</button></p>
</div>

<style>
  .raccourcis {
    display: grid;
    gap: calc(12 * var(--u));
  }
  h3 {
    margin: 0;
  }
</style>
