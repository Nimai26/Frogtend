<script lang="ts">
  // La jaquette d'un jeu, servie par le cœur pour le profil ouvert. Sans image : une carte de remplacement
  // lisible (titre et plateforme), peinte avec les jetons du skin.
  import { adresseJaquette } from '$lib/api';

  let { id, titre, plateforme = '' }: { id: number; titre: string; plateforme?: string } = $props();
  let absente = $state(false);
  let chargee = $state(false);

  $effect(() => {
    // Un autre jeu : on retente l'image.
    void id;
    absente = false;
    chargee = false;
  });
</script>

<div class="jaquette" class:chargee>
  {#if !absente}
    <img
      src={adresseJaquette(id)}
      alt=""
      loading="lazy"
      decoding="async"
      onload={() => (chargee = true)}
      onerror={() => (absente = true)}
    />
  {/if}
  {#if absente || !chargee}
    <div class="remplacement" aria-hidden="true">
      <span class="titre">{titre}</span>
      {#if plateforme}<span class="plateforme">{plateforme}</span>{/if}
    </div>
  {/if}
</div>

<style>
  .jaquette {
    position: relative;
    aspect-ratio: 3 / 4;
    width: 100%;
    border-radius: calc(10 * var(--u));
    overflow: hidden;
  }
  img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .remplacement {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    gap: calc(4 * var(--u));
    padding: calc(12 * var(--u));
    background:
      linear-gradient(160deg, color-mix(in srgb, var(--accent2) 45%, var(--panel)), color-mix(in srgb, var(--accent) 35%, var(--panel)));
    border: 1px solid var(--line);
    border-radius: inherit;
  }
  .chargee .remplacement {
    display: none;
  }
  .titre {
    font-weight: 700;
    font-size: calc(15 * var(--u));
    color: var(--ink);
    line-height: 1.2;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 4;
    line-clamp: 4;
    -webkit-box-orient: vertical;
  }
  .plateforme {
    font-size: calc(11 * var(--u));
    color: var(--dim);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
</style>
