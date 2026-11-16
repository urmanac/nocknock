/**
 * Pins the shipped install files (#89): the manifest is valid and opens the Lite
 * cockpit, every icon it names exists at the size it declares, index.html carries the
 * iOS and Android tags, and nothing registers a service worker.
 */
import { describe, it, expect } from 'vitest';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { detectPlatform, installHint, isLiteView, isStandalone } from './pwa';

const at = (rel: string) => new URL(rel, import.meta.url);
const manifest = JSON.parse(readFileSync(at('../public/manifest.json'), 'utf8'));
const indexHtml = readFileSync(at('../index.html'), 'utf8');
const ORIGIN = 'http://chudmax:3000';

/** Width and height from a PNG's IHDR chunk. */
function pngSize(bytes: Uint8Array): { width: number; height: number } {
  const signature = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
  signature.forEach((b, i) => expect(bytes[i]).toBe(b));
  const u32 = (o: number) => ((bytes[o] << 24) | (bytes[o + 1] << 16) | (bytes[o + 2] << 8) | bytes[o + 3]) >>> 0;
  return { width: u32(16), height: u32(20) };
}

describe('web app manifest', () => {
  it('names the app and opens standalone', () => {
    expect(manifest.name).toBe('NockNock');
    expect(manifest.short_name.length).toBeLessThanOrEqual(12);
    expect(manifest.display).toBe('standalone');
    expect(manifest.theme_color).toMatch(/^#[0-9a-f]{6}$/);
    expect(manifest.background_color).toMatch(/^#[0-9a-f]{6}$/);
  });

  it('starts in the Lite cockpit, inside its own scope', () => {
    const start = new URL(manifest.start_url, ORIGIN);
    expect(start.origin).toBe(ORIGIN);
    expect(isLiteView(start.search)).toBe(true);
    // nginx serves index.html for "/" and the app reads ?view=lite itself.
    expect(start.pathname).toBe('/');
    const scope = new URL(manifest.scope, ORIGIN);
    expect(start.href.startsWith(scope.href)).toBe(true);
    // The full state room ("/") is in scope too, so its link stays inside the app.
    expect(new URL('/', ORIGIN).href.startsWith(scope.href)).toBe(true);
    expect(new URL(manifest.id, ORIGIN).origin).toBe(ORIGIN);
  });

  it('App.tsx still selects the Lite view the way the manifest assumes', () => {
    const app = readFileSync(at('./App.tsx'), 'utf8');
    expect(app).toContain(".get('view') === 'lite'");
  });

  it('every icon exists, is a PNG, and is the size it declares', () => {
    expect(manifest.icons.length).toBeGreaterThanOrEqual(4);
    for (const icon of manifest.icons) {
      expect(icon.type).toBe('image/png');
      expect(icon.src.startsWith('/icons/')).toBe(true);
      const file = at(`../public${icon.src}`);
      expect(existsSync(file), icon.src).toBe(true);
      const { width, height } = pngSize(readFileSync(file));
      expect(`${width}x${height}`, icon.src).toBe(icon.sizes);
    }
  });

  it('has what Android needs to install: 192 and 512, plus maskable variants', () => {
    const has = (sizes: string, purpose: string) =>
      manifest.icons.some((i: { sizes: string; purpose: string }) => i.sizes === sizes && i.purpose === purpose);
    expect(has('192x192', 'any')).toBe(true);
    expect(has('512x512', 'any')).toBe(true);
    expect(has('192x192', 'maskable')).toBe(true);
    expect(has('512x512', 'maskable')).toBe(true);
  });
});

describe('index.html', () => {
  it('links the manifest and the icons that exist', () => {
    expect(indexHtml).toContain('<link rel="manifest" href="/manifest.json" />');
    for (const [, href] of indexHtml.matchAll(/href="(\/icons\/[^"]+)"/g)) {
      expect(existsSync(at(`../public${href}`)), href).toBe(true);
    }
    const touch = /<link rel="apple-touch-icon" href="([^"]+)"/.exec(indexHtml);
    expect(touch).not.toBeNull();
    expect(pngSize(readFileSync(at(`../public${touch![1]}`)))).toEqual({ width: 180, height: 180 });
  });

  it('carries the iOS and Android home-screen tags', () => {
    expect(indexHtml).toContain('<meta name="apple-mobile-web-app-capable" content="yes" />');
    expect(indexHtml).toContain('<meta name="mobile-web-app-capable" content="yes" />');
    expect(indexHtml).toContain('<meta name="apple-mobile-web-app-title" content="NockNock" />');
    expect(indexHtml).toMatch(/<meta name="theme-color" content="#[0-9a-f]{6}" \/>/);
    expect(indexHtml).toContain('<meta name="viewport" content="width=device-width, initial-scale=1.0" />');
  });
});

describe('no service worker', () => {
  // A service worker could answer /v1 from a cache and show an old snapshot as live.
  // None ships. If one is ever added, /v1 and /health must stay network-only and this
  // test must be replaced by one that proves it.
  it('ships no worker script', () => {
    const shipped = readdirSync(at('../public/'));
    expect(shipped.filter((f) => /^(sw|service-?worker)/i.test(f) || f.endsWith('.js'))).toEqual([]);
  });

  it('registers none from index.html or the app source', () => {
    expect(indexHtml).not.toMatch(/serviceWorker/);
    const sources = ['./App.tsx', './pwa.ts', './walletconnect.ts', './api/client.ts', './hooks/useFreshness.tsx'];
    const components = readdirSync(at('./components/')).map((f) => `./components/${f}`);
    for (const rel of [...sources, ...components]) {
      expect(readFileSync(at(rel), 'utf8'), rel).not.toMatch(/serviceWorker\s*\.\s*register|navigator\.serviceWorker/);
    }
  });
});

describe('install hint', () => {
  const IPHONE = 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1 Mobile/15E148 Safari/604.1';
  const PIXEL = 'Mozilla/5.0 (Linux; Android 15; Pixel 9) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Mobile Safari/537.36';
  const MAC = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1 Safari/605.1.15';

  it('tells the platforms apart', () => {
    expect(detectPlatform(IPHONE, 5)).toBe('ios');
    expect(detectPlatform(PIXEL, 5)).toBe('android');
    expect(detectPlatform(MAC, 0)).toBe('other');
    // iPadOS sends a Mac user agent; the touch screen gives it away.
    expect(detectPlatform(MAC, 5)).toBe('ios');
  });

  it('knows the Lite view and the installed state', () => {
    expect(isLiteView('?view=lite')).toBe(true);
    expect(isLiteView('?foo=1&view=lite')).toBe(true);
    expect(isLiteView('')).toBe(false);
    expect(isLiteView('?view=full')).toBe(false);
    expect(isStandalone(true)).toBe(true);
    expect(isStandalone(false, true)).toBe(true);
    expect(isStandalone(false, false)).toBe(false);
    expect(isStandalone(false)).toBe(false);
  });

  it('gives iOS the Safari steps and says HTTP is enough to add it', () => {
    const h = installHint({ platform: 'ios', standalone: false, secure: false });
    expect(h.state).toBe('ios');
    expect(h.steps.join(' ')).toContain('Add to Home Screen');
    expect(h.note).toContain('plain HTTP');
    expect(installHint({ platform: 'ios', standalone: false, secure: true }).note).toBeUndefined();
  });

  it('is honest that Android needs HTTPS for a real install', () => {
    const http = installHint({ platform: 'android', standalone: false, secure: false });
    expect(http.state).toBe('android_needs_https');
    expect(http.title).toContain('needs HTTPS');
    expect(http.note).toContain('bookmark');
    const https = installHint({ platform: 'android', standalone: false, secure: true });
    expect(https.state).toBe('android');
    expect(https.steps.join(' ')).toContain('Install');
  });

  it('says so once installed, and never claims anything is stored', () => {
    const h = installHint({ platform: 'ios', standalone: true, secure: true });
    expect(h.state).toBe('installed');
    expect(h.steps).toEqual([]);
    expect(h.note).toContain('Nothing is stored on the phone');
    expect(installHint({ platform: 'other', standalone: false, secure: false }).state).toBe('other');
  });
});
