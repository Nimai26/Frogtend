// Recopie le numéro du fichier VERSION (la seule source) dans package.json et src-tauri/Cargo.toml.
// tauri.conf.json lit déjà celui de package.json.
// Usage : node scripts/version.mjs            → applique VERSION
//         node scripts/version.mjs 0.2.0      → écrit 0.2.0 dans VERSION, puis l'applique
import { readFileSync, writeFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';

const racine = new URL('../', import.meta.url);
const chemin = (p) => new URL(p, racine);

const demande = process.argv[2];
if (demande) {
  if (!/^\d+\.\d+\.\d+$/.test(demande)) {
    console.error(`Refusé : « ${demande} » n'est pas une version de la forme 1.2.3.`);
    process.exit(1);
  }
  writeFileSync(chemin('VERSION'), `${demande}\n`);
}
const version = readFileSync(chemin('VERSION'), 'utf8').trim();

const pkg = JSON.parse(readFileSync(chemin('package.json'), 'utf8'));
pkg.version = version;
writeFileSync(chemin('package.json'), `${JSON.stringify(pkg, null, 2)}\n`);

const cargo = readFileSync(chemin('src-tauri/Cargo.toml'), 'utf8');
const nouveau = cargo.replace(/^version = ".*"$/m, `version = "${version}"`);
writeFileSync(chemin('src-tauri/Cargo.toml'), nouveau);

// Cargo.lock suit Cargo.toml.
execFileSync('cargo', ['update', '--workspace', '--offline', '--manifest-path', 'src-tauri/Cargo.toml'], {
  cwd: racine,
  stdio: 'inherit',
});

// Vérifier le résultat, pas le retour : on relit les fichiers.
const relu = [
  JSON.parse(readFileSync(chemin('package.json'), 'utf8')).version,
  /^version = "(.*)"$/m.exec(readFileSync(chemin('src-tauri/Cargo.toml'), 'utf8'))?.[1],
];
if (relu.some((v) => v !== version)) {
  console.error(`Échec : versions relues ${relu.join(', ')} au lieu de ${version}.`);
  process.exit(1);
}
console.log(`Version ${version} appliquée à package.json et Cargo.toml.`);
