// Lot S5 : « 📥 Reprendre mes parties de Firehouse » et le règlement des conflits, sur les formes RÉELLES des commandes
// (`parties_reprise_apercu`, `parties_reprendre`, `parties_conflits`, `parties_conflit_regler`, commandes.rs).
import { beforeEach, describe, expect, it, vi } from 'vitest';

const BILAN = { jeux: 1, posees: 1, fichiers: 2, deja_la: 0, non_installes: 0, non_resolues: 0, identifiants: 0, en_cours: 0, conflits: 0, partielles: 0 };
const CONFLIT = { jeu: 7, cle: '0a1b2c3d', titre: 'Cyberpunk', ici: { date: 2000, octets: 1, fichiers: 1 }, firehouse: { date: 1000, octets: 2, fichiers: 1 }, partiel: false };
let liste: object[] = [];
let bilan: object = BILAN;
let conflits: object[] = [];
const appels: string[] = [];
const regler = vi.fn(async (..._a: unknown[]) => true);
const reprendreApi = vi.fn(async (..._a: unknown[]) => structuredClone(bilan));
const apercu = vi.fn(async (..._a: unknown[]) => ({ fichiers: 3, octets: 2048, jeux: 1 }));
vi.mock('$lib/api', () => ({
  taille: (o: number) => `${o} o`,
  api: {
    restaurationListe: async () => structuredClone(liste),
    partiesRepriseApercu: (...a: unknown[]) => apercu(...a),
    partiesReprendre: (...a: unknown[]) => reprendreApi(...a),
    partiesConflits: async () => structuredClone(conflits),
    partiesConflitRegler: (...a: unknown[]) => regler(...a),
  },
}));
let choixConflit = 'ici';
let reponseChoisir: unknown = null;
let reponseConfirmer = true;
const choisir = vi.fn(async (..._a: unknown[]) => reponseChoisir);
const confirmer = vi.fn(async (..._a: unknown[]) => reponseConfirmer);
const informer = vi.fn(async (..._a: unknown[]) => {});
const toast = vi.fn();
const deciderConflit = vi.fn(async (..._a: unknown[]) => {
  appels.push('conflit');
  return choixConflit;
});
vi.mock('$lib/dialogues/fenetres.svelte', () => ({
  choisir: (...a: unknown[]) => choisir(...a),
  confirmer: (...a: unknown[]) => confirmer(...a),
  informer: (...a: unknown[]) => informer(...a),
  toast: (...a: unknown[]) => toast(...a),
  toastProgres: () => 1,
  majProgres: () => {},
  fermerToast: () => {},
  deciderConflit: (...a: unknown[]) => deciderConflit(...a),
}));
vi.mock('@tauri-apps/api/event', () => ({ listen: async () => () => {} }));

import { reglerConflits, reprendreMesParties, reprise } from './reprise.svelte';

const s = (pc: string, date: string, a_moi = true) => ({ profil: 'Seb', pc, a_moi, octets: 0, derniere: { date } });

beforeEach(() => {
  liste = [s('VENKMAN', '5000')];
  bilan = BILAN;
  conflits = [];
  appels.length = 0;
  choixConflit = 'ici';
  reponseChoisir = null;
  reponseConfirmer = true;
  for (const f of [regler, reprendreApi, apercu, choisir, confirmer, informer, toast, deciderConflit]) f.mockClear();
  reprise.enCours = null;
  reprise.abriEnCours = null;
});

describe('📥 Reprendre mes parties de Firehouse', () => {
  it('un seul PC : aucun choix ; annonce fichiers et taille ; rien sans le oui', async () => {
    reponseConfirmer = false;
    expect(await reprendreMesParties(7)).toBe(false);
    expect(choisir).not.toHaveBeenCalled();
    expect(apercu).toHaveBeenCalledWith(7, 'Seb', 'VENKMAN');
    expect(String((confirmer.mock.calls[0][1] as { message: string }).message)).toContain('3 fichiers, 2048 o');
    expect(reprendreApi).not.toHaveBeenCalled();
  });

  it('plusieurs PC : le plus récent en premier, la personne choisit ; puis seulement ce jeu est repris', async () => {
    liste = [s('ANCIEN', '1000'), s('AUTRE', '9000', false), s('VENKMAN', '5000')];
    reponseChoisir = liste[2];
    expect(await reprendreMesParties(7)).toBe(true);
    const options = choisir.mock.calls[0][1] as { libelle: string }[];
    expect(options.map((o) => o.libelle)).toEqual(['⭐ VENKMAN', 'ANCIEN']);
    expect(reprendreApi).toHaveBeenCalledWith(7, 'Seb', 'VENKMAN');
    expect(toast.mock.calls.at(-1)?.[0]).toContain('remise');
    expect(reprise.enCours).toBeNull();
  });

  it('aucune sauvegarde de ce profil : dit, rien de téléchargé', async () => {
    liste = [s('AUTRE', '9000', false)];
    expect(await reprendreMesParties(7)).toBe(false);
    expect(informer.mock.calls[0][0]).toContain('Aucune sauvegarde');
    expect(apercu).not.toHaveBeenCalled();
  });

  it('un conflit après la reprise : la fenêtre de conflit, puis le choix envoyé au cœur', async () => {
    bilan = { ...BILAN, posees: 0, conflits: 1 };
    conflits = [CONFLIT];
    choixConflit = 'firehouse';
    await reprendreMesParties(7);
    expect(deciderConflit).toHaveBeenCalledTimes(1);
    expect(regler).toHaveBeenCalledWith(7, '0a1b2c3d', 'reprendre');
  });
});

describe('revue du lot S5', () => {
  it('des conflits restent : jamais « déjà à leur place »', async () => {
    bilan = { ...BILAN, posees: 0, fichiers: 0, conflits: 1 };
    conflits = [CONFLIT];
    choixConflit = 'plus_tard';
    await reprendreMesParties(7);
    expect(toast.mock.calls.some((c) => String(c[0]).includes('déjà à leur place'))).toBe(false);
  });

  it('rien chez Firehouse pour ce jeu : « Firehouse n’a aucune partie de ce jeu »', async () => {
    apercu.mockResolvedValueOnce({ fichiers: 0, octets: 0, jeux: 0 });
    expect(await reprendreMesParties(7)).toBe(false);
    expect(String(informer.mock.calls[0][1])).toContain('Firehouse n’a aucune partie de ce jeu');
  });

  it('pendant une mise à l’abri, la reprise ne part pas', async () => {
    reprise.abriEnCours = 7;
    expect(await reprendreMesParties(7)).toBe(false);
    expect(apercu).not.toHaveBeenCalled();
  });

  it('une par une : le premier « plus tard » arrête la série (les suivantes attendent, c’est dit)', async () => {
    choixConflit = 'plus_tard';
    const l = [1, 2, 3].map((i) => ({ ...CONFLIT, cle: `c${i}` }));
    expect(await reglerConflits(l as never)).toBe(0);
    expect(deciderConflit).toHaveBeenCalledTimes(1);
    expect(regler).not.toHaveBeenCalled();
  });

  it('le choix groupé cite les titres (les 5 premiers, puis « et N autres »)', async () => {
    const l = ['A', 'B', 'C', 'D', 'E', 'F', 'G'].map((t, i) => ({ ...CONFLIT, cle: `c${i}`, titre: t }));
    reponseChoisir = null;
    await reglerConflits(l as never);
    const reprendre = (choisir.mock.calls[0][1] as { valeur: string; detail?: string }[]).find((o) => o.valeur === 'reprendre');
    expect(reprendre?.detail).toContain('A, B, C, D, E et 2 autres');
  });
});

describe('les conflits', () => {
  it('« Décider plus tard » : rien n’est envoyé', async () => {
    choixConflit = 'plus_tard';
    expect(await reglerConflits([CONFLIT as never])).toBe(0);
    expect(regler).not.toHaveBeenCalled();
  });

  it('« Garder celle de ce PC » : envoyé comme « garder »', async () => {
    expect(await reglerConflits([CONFLIT as never])).toBe(1);
    expect(regler).toHaveBeenCalledWith(7, '0a1b2c3d', 'garder');
  });

  it('au-delà de 3 : un choix groupé d’abord (garder toutes : aucune fenêtre une par une)', async () => {
    const l = [1, 2, 3, 4].map((i) => ({ ...CONFLIT, cle: `c${i}` }));
    reponseChoisir = 'garder';
    expect(await reglerConflits(l as never)).toBe(4);
    expect((choisir.mock.calls[0][1] as { valeur: string }[]).map((o) => o.valeur)).toEqual(['garder', 'une', 'reprendre', 'plus_tard']);
    expect(deciderConflit).not.toHaveBeenCalled();
    expect(regler.mock.calls.every((c) => c[2] === 'garder')).toBe(true);
    // Fermé sans répondre : rien.
    regler.mockClear();
    reponseChoisir = null;
    expect(await reglerConflits(l as never)).toBe(0);
    expect(regler).not.toHaveBeenCalled();
  });
});
