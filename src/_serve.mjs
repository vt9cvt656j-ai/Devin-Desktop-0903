import { createServer } from 'http';
import { readFile } from 'fs/promises';
import { join, extname } from 'path';
import { fileURLToPath } from 'url';
const dir = fileURLToPath(new URL('.', import.meta.url));
const MIME = {'.html':'text/html','.css':'text/css','.js':'text/javascript','.svg':'image/svg+xml'};
const port = parseInt(process.env.PORT || '8899');
createServer(async (req, res) => {
  const p = join(dir, req.url === '/' ? 'wardrobe-test.html' : req.url);
  try { const d = await readFile(p); res.writeHead(200, {'Content-Type': MIME[extname(p)]||'text/html'}); res.end(d); }
  catch { res.writeHead(404); res.end('404'); }
}).listen(port, '127.0.0.1', () => console.log(`http://127.0.0.1:${port}`));
