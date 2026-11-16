/**
 * Minimal typings for the Node file calls that pwa.test.ts uses to read the shipped
 * manifest, index.html and icons. The project has no @types/node; Vitest runs in Node.
 */
declare module 'node:fs' {
  export function readFileSync(path: URL): Uint8Array;
  export function readFileSync(path: URL, encoding: 'utf8'): string;
  export function existsSync(path: URL): boolean;
  export function readdirSync(path: URL): string[];
}
