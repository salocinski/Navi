// Copie les DLL CUDA redistribuables (cublas, cublasLt ; cudart est lié en statique) vers src-tauri/cuda-redist/,
// d'où l'installeur les embarque (src-tauri/tauri.bundle.conf.json). À lancer avant `tauri build`.
// Usage : node tools/copy-cuda-dlls.mjs   (CUDA_PATH_V13_5 ou CUDA_PATH requis)
import { copyFileSync, existsSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';

const root = process.env.CUDA_PATH_V13_5 ?? process.env.CUDA_PATH;
if (!root) throw new Error('CUDA_PATH_V13_5 (ou CUDA_PATH) non défini');
const out = join(import.meta.dirname, '..', 'src-tauri', 'cuda-redist');
mkdirSync(out, { recursive: true });
for (const dll of ['cublas64_13.dll', 'cublasLt64_13.dll']) {
  const src = join(root, 'bin', 'x64', dll);
  if (!existsSync(src)) throw new Error(`DLL introuvable : ${src}`);
  copyFileSync(src, join(out, dll));
  console.log(`copié : ${dll}`);
}
