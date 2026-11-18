<script lang="ts">
  // ⚙ Options ▸ Comptes ▸ Mes jeux installés (lot A, ✅ Seb 18/11) : les jeux INSTALLÉS des boutiques de ce PC entrent
  // seuls dans la ludothèque de CE profil, relus à chaque ouverture (coché d'office, comme `gratuits.auto`).
  import { onMount } from 'svelte';
  import { etat, reglerProfil } from '$lib/etat.svelte';
  import { boutiquesPc, lireEtatsPc } from '$lib/boutiques/boutiques-pc.svelte';
  import { ligneEtat } from '$lib/boutiques/jeu-boutique';

  onMount(lireEtatsPc);
</script>

<div class="installes">
  <label class="auto">
    <input type="checkbox" checked={etat.profil.boutiquesPc.auto} onchange={(e) => reglerProfil('boutiquesPc.auto', e.currentTarget.checked)} />
    Ajouter tout seul les jeux installés de mes boutiques
  </label>
  <p class="muted">
    À chaque ouverture de ton profil, Frogtend regarde les jeux installés sur ce PC par Steam, Epic, GOG, Ubisoft Connect, Amazon Games,
    Xbox et Battle.net, et les range dans ta ludothèque. Sans compte à relier ni clé à donner. Jouer et Installer passent par le lanceur de la boutique.
  </p>
  <ul>
    {#each boutiquesPc.etats as e (e.boutique)}<li>{ligneEtat(e)}</li>{/each}
  </ul>
</div>

<style>
  .installes {
    display: grid;
    gap: calc(12 * var(--u));
  }
  .auto {
    display: flex;
    gap: calc(8 * var(--u));
    align-items: center;
  }
  .muted,
  ul {
    margin: 0;
  }
</style>
