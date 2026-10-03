import { describe, expect, it } from 'vitest';
import { deduireTypeToast, dureeToast, motifDuRefus, texteRefus } from './messages';

describe('deduireTypeToast', () => {
  it('reconnaît les emoji', () => {
    expect(deduireTypeToast('🎉 Bravo')).toBe('ok');
    expect(deduireTypeToast('⛔ Non')).toBe('erreur');
    expect(deduireTypeToast('❌ raté')).toBe('erreur');
    expect(deduireTypeToast('⚠ Disque presque plein')).toBe('alerte');
  });

  it('fait l’emporter ✅ sur tout le reste', () => {
    expect(deduireTypeToast('✅ Erreur corrigée ⛔')).toBe('ok');
  });

  it('reconnaît les expressions d’échec n’importe où', () => {
    expect(deduireTypeToast('Le téléchargement a échoué')).toBe('erreur');
    expect(deduireTypeToast('Accès refusé par Firehouse')).toBe('erreur');
    expect(deduireTypeToast('Impossible de lire le dossier')).toBe('erreur');
  });

  it('regarde ensuite le premier mot', () => {
    expect(deduireTypeToast('Échec de la synchronisation')).toBe('erreur');
    expect(deduireTypeToast('Introuvable : Dune')).toBe('erreur');
    expect(deduireTypeToast('Attention, disque réseau lent')).toBe('alerte');
    expect(deduireTypeToast('Déjà installé')).toBe('alerte');
    expect(deduireTypeToast('Aucun résultat')).toBe('alerte');
    expect(deduireTypeToast('Enregistré.')).toBe('ok');
    expect(deduireTypeToast('Créé')).toBe('ok');
    expect(deduireTypeToast('Synchronisation en cours')).toBe('neutre');
  });

  it('laisse un refus affiché plus longtemps', () => {
    expect(dureeToast('erreur')).toBe(6000);
    expect(dureeToast('ok')).toBe(3200);
  });
});

describe('motifDuRefus', () => {
  it('extrait le motif des formes courantes', () => {
    expect(motifDuRefus('jeton expiré')).toBe('jeton expiré');
    expect(motifDuRefus(new Error('disque plein'))).toBe('disque plein');
    expect(motifDuRefus({ erreur: 'grade insuffisant' })).toBe('grade insuffisant');
    // Les erreurs du cœur de Frogtend : {sorte, motif}.
    expect(motifDuRefus({ sorte: 'jeton_refuse', motif: 'Firehouse refuse le jeton.' })).toBe('Firehouse refuse le jeton.');
    expect(motifDuRefus({ detail: [{ msg: 'champ requis' }, { msg: 'trop long' }] })).toBe('champ requis ; trop long');
  });

  it('ne rend jamais « [object Object] »', () => {
    expect(motifDuRefus({ code: 12 })).toBe('motif inconnu');
    expect(motifDuRefus(null)).toBe('motif inconnu');
  });

  it('ne montre jamais un chemin d’API', () => {
    expect(motifDuRefus('introuvable : /api/jeux/v1/jeu/110')).toBe('introuvable :');
  });

  it('formule le refus', () => {
    expect(texteRefus('jeton expiré')).toBe('⛔ Refusé : jeton expiré');
  });
});
