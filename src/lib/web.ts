// Ouvrir une page web dans le navigateur de la personne (seulement http ou https ; un échec est dit).
import { isTauri } from '@tauri-apps/api/core';
import { toast } from '$lib/dialogues/fenetres.svelte';
import { adresseWeb } from '$lib/mods/mods';

export async function ouvrirPageWeb(url: string | null | undefined): Promise<void> {
  const a = adresseWeb(url);
  if (!a) {
    toast('Cette page ne peut pas s’ouvrir.', 'erreur');
    return;
  }
  if (!isTauri()) return;
  try {
    const { openUrl } = await import('@tauri-apps/plugin-opener');
    await openUrl(a);
  } catch {
    toast('Impossible d’ouvrir la page dans le navigateur.', 'erreur');
  }
}
