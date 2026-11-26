// @ts-nocheck — Node builtins; Vite config is not part of the app tsc project.
import { defineConfig } from 'vite';
import { sveltekit } from '@sveltejs/kit/vite';
import fs from 'fs';
import path from 'path';

const host = process.env.TAURI_DEV_HOST;
const home = process.env.HOME || process.env.USERPROFILE || '';

const THUMB_PREFIX = '/__lixa_thumb__';

/**
 * Serve on-disk WebP thumbs same-origin under Vite.
 * SvelteKit swallows Vite `/@fs` for paths outside the project (404), so we
 * use an explicit middleware with an allowlist instead.
 */
function lixaThumbPlugin(allowedRoots) {
  const roots = allowedRoots
    .filter(Boolean)
    .map((root) => path.resolve(root));

  function allowed(filePath) {
    const resolved = path.resolve(filePath);
    if (!resolved.toLowerCase().endsWith('.webp')) return false;
    return roots.some(
      (root) => resolved === root || resolved.startsWith(root + path.sep),
    );
  }

  return {
    name: 'lixa-thumb-dev',
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        try {
          if (!req.url || !req.url.startsWith(THUMB_PREFIX)) {
            next();
            return;
          }
          const url = new URL(req.url, 'http://localhost');
          const filePath = url.searchParams.get('p') ?? '';
          if (!filePath || !allowed(filePath)) {
            res.statusCode = 403;
            res.end('forbidden');
            return;
          }
          if (!fs.existsSync(filePath)) {
            res.statusCode = 404;
            res.end('missing');
            return;
          }
          res.setHeader('Content-Type', 'image/webp');
          res.setHeader('Cache-Control', 'public, max-age=3600');
          fs.createReadStream(filePath).pipe(res);
        } catch (err) {
          res.statusCode = 500;
          res.end(String(err));
        }
      });
    },
  };
}

const shareRoot = path.join(home, '.local/share');
const picturesRoot = path.join(home, 'Pictures');

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  plugins: [lixaThumbPlugin([shareRoot, picturesRoot]), sveltekit()],

  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
    fs: {
      allow: [path.resolve('.'), shareRoot, picturesRoot],
    },
  },
  resolve: {
    alias: {
      $lib: path.resolve('./src/lib'),
    },
  },
}));
