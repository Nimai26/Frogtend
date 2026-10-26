// Prépare une livraison : suite complète, installateur signé, fichier latest.json des mises à jour.
// Le dépôt des fichiers dans la « release » GitHub reste une étape à part, faite à la main (rien n'est publié ici).
//
// La clé privée de signature n'est JAMAIS dans le dépôt. Elle est lue dans le fichier désigné par
// FROGTEND_CLE_SIGNATURE, sinon dans %USERPROFILE%\.tauri\frogtend.key. Son mot de passe éventuel :
// TAURI_SIGNING_PRIVATE_KEY_PASSWORD.
import { execSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, statSync, statfsSync, writeFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';

const DEPOT = 'https://github.com/Nimai26/Frogtend';

const version = readFileSync('VERSION', 'utf8').trim();
const cheminCle = process.env.FROGTEND_CLE_SIGNATURE ?? join(homedir(), '.tauri', 'frogtend.key');
if (!existsSync(cheminCle)) {
  console.error(`Refusé : clé de signature introuvable (${cheminCle}). Sans elle, les mises à jour ne sont pas signées.`);
  process.exit(1);
}

// La place du disque AVANT de compiler (leçon du 20/10, 3e disque plein : Venkman calcule aussi pour Firehouse). Le
// cache de compilation (`src-tauri/target/debug`) se reconstruit : on le dit, on ne l'efface pas tout seul.
// Le dossier de compilation est celui que donne cargo (réglé dans src-tauri/.cargo/config.toml : E: sur Venkman,
// ✅ Seb 25/10) : la place se mesure LÀ.
const cible = JSON.parse(
  execSync('cargo metadata --format-version 1 --no-deps', { cwd: 'src-tauri', encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }),
).target_directory;
mkdirSync(cible, { recursive: true });
const PLACE_MIN = 20 * 1024 ** 3;
const libre = statfsSync(cible).bavail * statfsSync(cible).bsize;
if (libre < PLACE_MIN) {
  console.error(
    `Refusé : il ne reste que ${(libre / 1024 ** 3).toFixed(1)} Go sur le disque de compilation (${cible} ; 20 Go au moins pour compiler).\n` +
      'Le cache de compilation se vide par « cargo clean --profile dev » dans src-tauri (il se reconstruit).',
  );
  process.exit(1);
}

// SDL3 se compile avec CMake : s'il n'est pas dans le PATH, celui des outils de Visual Studio.
if (!process.env.CMAKE) {
  let trouve = true;
  try {
    execSync('cmake --version', { stdio: 'ignore' });
  } catch {
    trouve = false;
  }
  const vs = 'C:/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools/Common7/IDE/CommonExtensions/Microsoft/CMake/CMake/bin/cmake.exe';
  if (!trouve && existsSync(vs)) process.env.CMAKE = vs;
}

execSync('node scripts/tests.mjs', { stdio: 'inherit' });

console.log(`\n=== Construction de Frogtend ${version}`);
execSync('pnpm tauri build', {
  stdio: 'inherit',
  env: {
    ...process.env,
    TAURI_SIGNING_PRIVATE_KEY: readFileSync(cheminCle, 'utf8'),
    TAURI_SIGNING_PRIVATE_KEY_PASSWORD: process.env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD ?? '',
  },
});

const dossier = join(cible, 'release', 'bundle', 'nsis');
const installateur = `Frogtend_${version}_x64-setup.exe`;
const signature = `${installateur}.sig`;
for (const f of [installateur, signature]) {
  if (!existsSync(join(dossier, f))) {
    console.error(`Échec : ${f} n'a pas été produit dans ${dossier}.`);
    process.exit(1);
  }
}

/** Les notes de cette version, tirées de docs/NOTES-DE-VERSION.md. */
function notes() {
  const texte = readFileSync('docs/NOTES-DE-VERSION.md', 'utf8');
  const debut = texte.indexOf(`## ${version}`);
  if (debut < 0) return `Frogtend ${version}`;
  const suite = texte.indexOf('\n## ', debut + 1);
  return texte.slice(debut, suite < 0 ? undefined : suite).trim();
}

const latest = {
  version,
  notes: notes(),
  pub_date: new Date().toISOString(),
  platforms: {
    'windows-x86_64': {
      signature: readFileSync(join(dossier, signature), 'utf8').trim(),
      url: `${DEPOT}/releases/download/v${version}/${installateur}`,
    },
  },
};
writeFileSync(join(dossier, 'latest.json'), `${JSON.stringify(latest, null, 2)}\n`);

const taille = (f) => `${(statSync(join(dossier, f)).size / 1e6).toFixed(1)} Mo`;
console.log(`
✅ Livraison ${version} prête dans ${dossier} :
   - ${installateur} (${taille(installateur)})
   - latest.json

À faire pour publier : créer la release « v${version} » sur ${DEPOT}/releases/new
et y joindre ces DEUX fichiers (les noms doivent rester exacts).`);
