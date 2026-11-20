// Régénère les frames PNG 32x32 des anneaux d'état Navi (carte N-050, DA maquette C).
// Prérequis (outil ponctuel, hors build) : npm install --no-save sharp
// Usage : node tools/gen_tray_icons.js
// Entrée : src-tauri/icons/icon.png (sphère néon ronde, coins transparents)
// Sortie : src-tauri/icons/tray/*.png + aperçus 16/20/24 px dans scratch/tray-preview/
const path = require('path');
const fs = require('fs');
const sharp = require('sharp');

const REPO = path.join(__dirname, '..');
const BASE = path.join(REPO, 'src-tauri', 'icons', 'icon.png');
const OUT = path.join(REPO, 'src-tauri', 'icons', 'tray');
const PREVIEW = path.join(REPO, 'scratch', 'tray-preview');

const SIZE = 32;
const SPHERE = 26; // sphère 26px sur canvas 32 — marge pour l'anneau
const OFF = (SIZE - SPHERE) / 2;
const R = 14.2; // rayon de l'anneau
const CIRC = 2 * Math.PI * R;

const COLOR = {
  gen: '#38bdf8', // bleu — génération
  play: '#22d3ee', // cyan — lecture
  pause: '#fbbf24', // ambre — pause
  error: '#ef4444', // rouge — erreur
};

function svgRing({ stroke, width, opacity = 1, dash = null, dashOffset = 0, track = false }) {
  const parts = [
    `<svg width="${SIZE}" height="${SIZE}" viewBox="0 0 ${SIZE} ${SIZE}" xmlns="http://www.w3.org/2000/svg">`,
  ];
  if (track) {
    parts.push(
      `<circle cx="16" cy="16" r="${R}" fill="none" stroke="#ffffff" stroke-opacity="0.22" stroke-width="${width}"/>`,
    );
  }
  const dashAttr = dash
    ? ` stroke-dasharray="${dash} ${CIRC - dash}" stroke-dashoffset="${dashOffset}"`
    : '';
  parts.push(
    `<circle cx="16" cy="16" r="${R}" fill="none" stroke="${stroke}" stroke-opacity="${opacity * 0.35}" stroke-width="${width * 2.2}" stroke-linecap="round"${dashAttr}/>`,
    `<circle cx="16" cy="16" r="${R}" fill="none" stroke="${stroke}" stroke-opacity="${opacity}" stroke-width="${width}" stroke-linecap="round"${dashAttr}/>`,
  );
  parts.push('</svg>');
  return Buffer.from(parts.join(''));
}

function svgErrorBadge() {
  // Pastille rouge en bas à droite + « ! » dessiné (aucune police requise).
  return Buffer.from(
    `<svg width="${SIZE}" height="${SIZE}" viewBox="0 0 ${SIZE} ${SIZE}" xmlns="http://www.w3.org/2000/svg">` +
      `<circle cx="24.5" cy="24.5" r="7" fill="${COLOR.error}" stroke="#111827" stroke-width="1.6" stroke-opacity="0.9"/>` +
      `<rect x="23.4" y="19.6" width="2.2" height="5.4" rx="1.1" fill="#ffffff"/>` +
      `<circle cx="24.5" cy="28.2" r="1.25" fill="#ffffff"/>` +
      `</svg>`,
  );
}

async function baseSphere() {
  return sharp(BASE).resize(SPHERE, SPHERE, { fit: 'fill' }).png().toBuffer();
}

async function frame(overlays) {
  const sphere = await baseSphere();
  let img = sharp({
    create: {
      width: SIZE,
      height: SIZE,
      channels: 4,
      background: { r: 0, g: 0, b: 0, alpha: 0 },
    },
  }).composite([{ input: sphere, left: OFF, top: OFF }]);
  for (const ov of overlays) {
    img = img.composite([{ input: ov, left: 0, top: 0 }]);
  }
  return img.png().toBuffer();
}

async function main() {
  fs.mkdirSync(OUT, { recursive: true });
  fs.mkdirSync(PREVIEW, { recursive: true });
  const jobs = {};

  // prêt : aucun anneau
  jobs['ready.png'] = frame([]);

  // génération : arc bleu en rotation (8 frames) — piste faible pour la lisibilité
  for (let i = 0; i < 8; i++) {
    const offset = -((i / 8) * CIRC);
    jobs[`gen_${i}.png`] = frame([
      svgRing({ stroke: COLOR.gen, width: 2.4, track: true, dash: 26, dashOffset: offset }),
    ]);
  }

  // lecture : anneau cyan complet qui pulse (3 frames)
  [0.45, 0.72, 1.0].forEach((op, i) => {
    jobs[`play_${i}.png`] = frame([
      svgRing({ stroke: COLOR.play, width: 2.4, opacity: op, track: true }),
    ]);
  });

  // pause : anneau ambre fixe
  jobs['pause.png'] = frame([svgRing({ stroke: COLOR.pause, width: 2.4, track: true })]);

  // erreur : pastille rouge
  jobs['error.png'] = frame([svgErrorBadge()]);

  for (const [name, promise] of Object.entries(jobs)) {
    const buf = await promise;
    await sharp(buf).toFile(path.join(OUT, name));
    for (const s of [16, 20, 24]) {
      await sharp(buf)
        .resize(s, s, { kernel: 'lanczos3' })
        .toFile(path.join(PREVIEW, name.replace('.png', `_${s}.png`)));
    }
  }
  console.log(`écrit : ${OUT} — frames : ${Object.keys(jobs).length}`);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
