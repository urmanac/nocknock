/**
 * Phone install helpers (#89). Pure functions: the install card and the manifest test
 * share one definition of "the Lite view" and of what each phone can do.
 *
 * NockNock ships a web app manifest and no service worker. Nothing is cached by the app,
 * so an installed copy always reads the worker, and when it cannot it says so.
 */

/** The Lite cockpit is `/?view=lite` (App.tsx bootstrap). `search` is location.search. */
export function isLiteView(search: string): boolean {
  return new URLSearchParams(search).get('view') === 'lite';
}

export type Platform = 'ios' | 'android' | 'other';

/** iPadOS reports itself as a Mac; it is told apart by having a touch screen. */
export function detectPlatform(userAgent: string, maxTouchPoints = 0): Platform {
  if (/iPhone|iPad|iPod/.test(userAgent)) return 'ios';
  if (/Macintosh/.test(userAgent) && maxTouchPoints > 1) return 'ios';
  if (/Android/.test(userAgent)) return 'android';
  return 'other';
}

/** True when the page runs from the home screen rather than in a browser tab. */
export function isStandalone(displayModeStandalone: boolean, navigatorStandalone?: boolean): boolean {
  return displayModeStandalone || navigatorStandalone === true;
}

export interface InstallHint {
  state: 'installed' | 'ios' | 'android' | 'android_needs_https' | 'other';
  title: string;
  steps: string[];
  note?: string;
}

/**
 * What to tell the person holding this device. `secure` is window.isSecureContext:
 * Android Chrome installs a web app only over HTTPS, and over plain HTTP its
 * "Add to Home screen" makes a bookmark that opens in a browser tab.
 */
export function installHint(input: { platform: Platform; standalone: boolean; secure: boolean }): InstallHint {
  if (input.standalone) {
    return {
      state: 'installed',
      title: 'Running from your home screen',
      steps: [],
      note: 'Nothing is stored on the phone: every number is read from the worker when you open it.',
    };
  }
  if (input.platform === 'ios') {
    return {
      state: 'ios',
      title: 'Add NockNock to your home screen',
      steps: ['Open this page in Safari.', 'Tap Share.', 'Tap "Add to Home Screen", then Add.'],
      note: input.secure
        ? undefined
        : 'This works over plain HTTP. WalletConnect pairing from the phone wants HTTPS: see the opt-in TLS proxy in DEPLOYMENT.md section 7.',
    };
  }
  if (input.platform === 'android') {
    return input.secure
      ? {
          state: 'android',
          title: 'Install NockNock',
          steps: ['Open this page in Chrome.', 'Tap the ⋮ menu.', 'Tap "Add to Home screen", then Install.'],
        }
      : {
          state: 'android_needs_https',
          title: 'Add a shortcut (a full install needs HTTPS)',
          steps: ['Open this page in Chrome.', 'Tap the ⋮ menu.', 'Tap "Add to Home screen".'],
          note: 'Over plain HTTP Chrome adds a bookmark that opens in a browser tab. To install it as an app, turn on the opt-in TLS proxy (DEPLOYMENT.md section 7) and open the https:// address.',
        };
  }
  return {
    state: 'other',
    title: 'Open this page on your phone',
    steps: ['On the phone, open this address while on the same network.', 'Use the browser\'s "Add to Home Screen".'],
  };
}
