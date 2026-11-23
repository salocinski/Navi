// Génère THIRD_PARTY_NOTICES.md (carte N-061) :
// partie rédigée (tools/notices-manuel.md) + textes de licence des crates Rust embarquées
// (graphe normal de la cible Windows) et des paquets npm de production.
// Usage : node tools/gen-notices.mjs   (cargo et npm requis, dépendances déjà téléchargées)
import { execFileSync, execSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';

const ROOT = join(dirname(new URL(import.meta.url).pathname.replace(/^\/(\w:)/, '$1')), '..');
const opts = { cwd: ROOT, encoding: 'utf8', maxBuffer: 1 << 28 };
const run = (cmd, args) => execFileSync(cmd, args, opts);

const LICENSE_FILE = /^(licen[cs]e|copying|notice|unlicense)/i;
const licenseTexts = (dir) =>
  readdirSync(dir)
    .filter((f) => LICENSE_FILE.test(f))
    .sort()
    .map((f) => readFileSync(join(dir, f), 'utf8').replace(/\r\n/g, '\n').trim())
    .filter(Boolean);

// --- crates Rust : graphe normal depuis la racine, cible Windows ---
const meta = JSON.parse(
  run('cargo', [
    'metadata',
    '--format-version',
    '1',
    '--locked',
    '--filter-platform',
    'x86_64-pc-windows-msvc',
    '--manifest-path',
    'src-tauri/Cargo.toml',
  ]),
);
const byId = new Map(meta.packages.map((p) => [p.id, p]));
const nodes = new Map(meta.resolve.nodes.map((n) => [n.id, n]));
const seen = new Set();
const queue = [meta.resolve.root];
while (queue.length) {
  const id = queue.pop();
  if (seen.has(id)) continue;
  seen.add(id);
  for (const d of nodes.get(id).deps)
    if (d.dep_kinds.some((k) => k.kind === null)) queue.push(d.pkg);
}
const items = [...seen]
  .filter((id) => id !== meta.resolve.root)
  .map((id) => byId.get(id))
  .map((p) => ({
    label: p.name + ' ' + p.version,
    license: p.license || 'non déclarée',
    dir: dirname(p.manifest_path),
  }));

// --- paquets npm de production ---
const npmDirs = execSync('npm ls --omit=dev --all --parseable', opts)
  .split('\n')
  .map((s) => s.trim())
  .filter((d) => d && d !== ROOT && existsSync(join(d, 'package.json')));
for (const dir of npmDirs) {
  const pkg = JSON.parse(readFileSync(join(dir, 'package.json'), 'utf8'));
  items.push({
    label: 'npm:' + pkg.name + ' ' + pkg.version,
    license: String(pkg.license || 'non déclarée'),
    dir,
  });
}

// Regroupe les crates qui partagent exactement les mêmes textes de licence.
const groups = new Map();
const missing = [];
for (const it of items.sort((a, b) => a.label.localeCompare(b.label))) {
  const texts = licenseTexts(it.dir);
  if (!texts.length) {
    missing.push(it);
    continue;
  }
  const body = texts.join('\n\n---\n\n');
  const key = createHash('sha256').update(body).digest('hex');
  if (!groups.has(key)) groups.set(key, { body, members: [] });
  groups.get(key).members.push(it);
}

let out = readFileSync(join(ROOT, 'tools', 'notices-manuel.md'), 'utf8').trim() + '\n\n';
out += '## Dépendances Rust et JavaScript embarquées\n\n';
out +=
  'Liste générée par `tools/gen-notices.mjs` (' +
  items.length +
  " paquets). Les textes de licence sont regroupés lorsqu'ils sont identiques.\n\n";
let n = 0;
for (const g of [...groups.values()].sort((a, b) => b.members.length - a.members.length)) {
  n += 1;
  out +=
    '### Texte de licence ' +
    n +
    '\n\nPaquets : ' +
    g.members.map((m) => m.label + ' (' + m.license + ')').join(', ') +
    '\n\n```text\n' +
    g.body +
    '\n```\n\n';
}
if (missing.length) {
  out +=
    '### Paquets sans fichier de licence dans leur archive\n\nLicence déclarée dans leurs métadonnées :\n\n';
  out += missing.map((m) => '- ' + m.label + ' : ' + m.license).join('\n') + '\n\n';
  out +=
    'Textes de ces licences standard : MPL-2.0 https://www.mozilla.org/MPL/2.0/ ; MIT et BSD-3-Clause : modèles ci-dessus (copyright des auteurs dans le dépôt de chaque paquet, https://crates.io/crates/<nom>). Le code source des paquets MPL-2.0 est disponible sur crates.io.\n';
}
writeFileSync(join(ROOT, 'THIRD_PARTY_NOTICES.md'), out);
console.log(
  items.length +
    ' paquets, ' +
    groups.size +
    ' textes distincts, ' +
    missing.length +
    ' sans fichier de licence',
);
