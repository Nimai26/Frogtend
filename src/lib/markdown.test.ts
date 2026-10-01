import { describe, expect, it } from 'vitest';
import { markdownSimple } from './markdown';

describe('markdownSimple', () => {
  it('met en forme titres, gras, listes', () => {
    expect(markdownSimple('# Lancer Dune\nTexte **important**.\n- un\n- deux\n1. a')).toBe(
      '<h2>Lancer Dune</h2><p>Texte <strong>important</strong>.</p><ul><li>un</li><li>deux</li></ul><ol><li>a</li></ol>',
    );
  });

  it('n’exécute jamais de HTML ni de lien douteux', () => {
    const h = markdownSimple('<img src=x onerror=alert(1)> [clic](javascript:alert(1)) [ok](https://pcgamingwiki.com)');
    expect(h).not.toContain('<img');
    expect(h).toContain('&lt;img');
    expect(h).not.toContain('href');
    expect(h).toContain('ok <span class="lien">(https://pcgamingwiki.com)</span>');
  });
});
