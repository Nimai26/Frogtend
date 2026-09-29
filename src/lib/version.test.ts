import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const lireTexte = (chemin: string) => readFileSync(resolve(process.cwd(), chemin), 'utf8');

describe('le numéro de version', () => {
  it('est le même dans VERSION, package.json et Cargo.toml', () => {
    const version = lireTexte('VERSION').trim();
    expect(version).toMatch(/^\d+\.\d+\.\d+$/);
    expect(JSON.parse(lireTexte('package.json')).version).toBe(version);
    expect(/^version = "(.*)"$/m.exec(lireTexte('src-tauri/Cargo.toml'))?.[1]).toBe(version);
  });

  it('est lu par tauri.conf.json depuis package.json (une seule source)', () => {
    expect(JSON.parse(lireTexte('src-tauri/tauri.conf.json')).version).toBe('../package.json');
  });
});
