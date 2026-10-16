<script lang="ts">
  // Le lecteur de la musique et des vidéos d'un jeu (contrat 1.7) : un lecteur maison aux couleurs du thème (pas les
  // commandes du navigateur, ni atteignables à la manette ni aux couleurs du skin). Le fichier est servi par le cœur,
  // par morceaux (on peut avancer dans la lecture). Une seule lecture à la fois ; elle s'arrête en fermant.
  // ←/→ : −10 s / +10 s ; A (ou ⏯) : lecture ou pause ; ↑/↓ : volume ; B / Échap : revenir.
  import { onMount } from 'svelte';
  import { adresseDocument, adresseJaquette } from '$lib/api';
  import { retenirFocus } from '$lib/dialogues/focus';
  import { fenetreOuverte, prendreLaMain, selectionner, taodbox } from '$lib/taodbox/manette.svelte';
  import { duree } from './documents';

  let {
    jeu,
    doc,
    video,
    titre,
    sousLaBarre = false,
    onfermer,
  }: { jeu: number; doc: number; video: boolean; titre: string; sousLaBarre?: boolean; onfermer: () => void } = $props();

  let media = $state<HTMLMediaElement>();
  let pause = $state(true);
  let temps = $state(0);
  let total = $state(0);
  let volume = $state(1);
  let echec = $state(false);
  let lecture = $state<HTMLButtonElement>();
  let sansJaquette = $state(false);
  // Windows a refusé de lancer la lecture tout seul : on le dit, ⏯ la lance (ce n'est pas un fichier abîmé).
  let refusee = $state(false);

  const adresse = $derived(adresseDocument(jeu, doc));
  const avance = $derived(total > 0 ? Math.min(100, (temps / total) * 100) : 0);

  function basculer() {
    if (!media) return;
    if (media.paused) media.play().then(() => (refusee = false)).catch(rejet);
    else media.pause();
  }

  /** Un refus de `play()` : seul un format illisible est une panne (les autres : lecture interrompue ou refusée). */
  function rejet(e: unknown) {
    const nom = (e as { name?: string } | null)?.name;
    if (nom === 'NotSupportedError') echec = true;
    else if (nom === 'NotAllowedError') refusee = true;
  }

  function sauter(secondes: number) {
    if (!media || !total) return;
    media.currentTime = Math.max(0, Math.min(total - 0.1, media.currentTime + secondes));
  }

  function regler(pas: number) {
    volume = Math.round(Math.max(0, Math.min(1, volume + pas)) * 10) / 10;
  }

  onMount(() => {
    selectionner(lecture);
    media?.play().catch(rejet); // si Windows refuse la lecture d'office, ⏯ la lance
    const rendre = prendreLaMain((c) => {
      if (c === 'gauche') sauter(-10);
      else if (c === 'droite') sauter(10);
      else if (c === 'haut') regler(0.1);
      else if (c === 'bas') regler(-0.1);
      else if (c === 'valider') basculer();
      else onfermer();
      return true;
    });
    const touche = (e: KeyboardEvent) => {
      if (fenetreOuverte()) return;
      const fleches = !taodbox.actif;
      if (e.key === 'Escape') onfermer();
      else if (fleches && e.key === 'ArrowLeft') sauter(-10);
      else if (fleches && e.key === 'ArrowRight') sauter(10);
      else if (fleches && e.key === 'ArrowUp') regler(0.1);
      else if (fleches && e.key === 'ArrowDown') regler(-0.1);
      else return;
      e.preventDefault();
      e.stopPropagation();
    };
    window.addEventListener('keydown', touche, { capture: true });
    return () => {
      rendre();
      window.removeEventListener('keydown', touche, { capture: true });
      media?.pause();
    };
  });
</script>

<!-- Pas de role="dialog" : ce serait une « fenêtre maison » pour la manette, et le lecteur ne recevrait plus rien. -->
<section class="lecteur" class:sous-la-barre={sousLaBarre} aria-label={`${video ? 'Vidéo' : 'Musique'} : ${titre}`} use:retenirFocus>
  <header>
    <strong>{video ? '🎬' : '🎵'} {titre}</strong>
    <span class="temps" aria-live="off">{duree(temps)} / {total ? duree(total) : '…'}</span>
  </header>
  <div class="scene">
    {#if echec}
      <p class="note">Ce fichier ne se lit pas. Ferme et rouvre-le ; s’il ne se lit toujours pas, il est peut-être abîmé.</p>
    {:else if video}
      <!-- svelte-ignore a11y_media_has_caption -->
      <video
        bind:this={media}
        src={adresse}
        bind:paused={pause}
        bind:currentTime={temps}
        bind:duration={total}
        bind:volume
        onerror={() => (echec = true)}
        onclick={basculer}
      ></video>
    {:else}
      {#if !sansJaquette}<img class="jaquette" src={adresseJaquette(jeu, 400)} alt="" onerror={() => (sansJaquette = true)} />{/if}
      <audio bind:this={media} src={adresse} bind:paused={pause} bind:currentTime={temps} bind:duration={total} bind:volume onerror={() => (echec = true)}
      ></audio>
    {/if}
  </div>
  <div class="bar" role="progressbar" aria-label="Avancement" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(avance)}>
    <span style:width={`${avance}%`}></span>
  </div>
  <footer>
    <button class="btn" onclick={() => sauter(-10)}>⏪ 10 s</button>
    <button class="btn" bind:this={lecture} onclick={basculer}>{pause ? '▶ Lecture' : '⏸ Pause'}</button>
    <button class="btn" onclick={() => sauter(10)}>10 s ⏩</button>
    <button class="btn" onclick={() => regler(-0.1)} aria-label="Moins fort">🔉 −</button>
    <span class="volume">{Math.round(volume * 100)} %</span>
    <button class="btn" onclick={() => regler(0.1)} aria-label="Plus fort">🔊 +</button>
    <button class="btn" onclick={onfermer}>✕ Fermer</button>
    {#if refusee}<span class="aide">Appuie sur ▶ Lecture pour lancer.</span>
    {:else if taodbox.actif}<span class="aide">◀ ▶ 10 s · A lecture/pause · ▲ ▼ volume · B revenir</span>
    {:else}<span class="aide">← → 10 s · ↑ ↓ volume · Échap fermer</span>{/if}
  </footer>
</section>

<style>
  .lecteur {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    grid-template-rows: auto 1fr auto auto;
    background: var(--voile-opaque);
    color: var(--encre-voile);
  }
  /* Sur la fiche : sous la barre de titre (BarreTitre.svelte : 34 u + son filet), pour garder réduire et déplacer. */
  .lecteur.sous-la-barre {
    top: calc(34 * var(--u) + 1px);
  }
  header,
  footer {
    display: flex;
    align-items: center;
    gap: calc(10 * var(--u));
    padding: calc(10 * var(--u)) calc(16 * var(--u));
  }
  header .temps,
  .aide,
  .volume {
    color: var(--encre-voile-dim);
  }
  header .temps,
  .aide {
    margin-left: auto;
  }
  footer {
    justify-content: center;
    flex-wrap: wrap;
  }
  .scene {
    min-height: 0;
    display: grid;
    place-items: center;
    background: var(--fond-media);
  }
  video,
  .jaquette {
    max-width: 100%;
    max-height: 100%;
  }
  .jaquette {
    max-height: 70%;
  }
  /* La barre est celle du thème (base.css, doublée en Taodbox) : seulement ses marges ici. */
  .bar {
    margin: 0 calc(16 * var(--u));
  }
  .note {
    color: var(--encre-voile-note);
    text-align: center;
    padding: 0 calc(16 * var(--u));
  }
</style>
