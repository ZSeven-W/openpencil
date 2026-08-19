import { common, Want } from '@kit.AbilityKit';
import { hilog } from '@kit.PerformanceAnalysisKit';

const DOMAIN = 0x0001;
const TAG = 'OpenPencilPlayer';

/**
 * Hands an https URL to whatever browser the system has.
 *
 * `UIAbilityContext.openLink` is API 23 and `restartApp` is API 22, while this
 * module targets API 12 — so the portable route is the classic implicit want
 * (`ohos.want.action.viewData` + `entity.system.browsable`), which every
 * HarmonyOS build resolves to the default browser.
 *
 * DESKTOP-CLASS (2in1) SIGN-IN RUNS ENTIRELY HERE: the shell never paints a
 * native login page on a PC, it opens the engine's verification URL in the
 * browser and lets the engine's background poll finish the flow.
 */
export class BrowserLauncher {
  /** Resolves false when no browser could be started. */
  static async open(context: common.UIAbilityContext, url: string): Promise<boolean> {
    if (!url.startsWith('https://')) {
      hilog.warn(DOMAIN, TAG, 'refusing to open a non-https link');
      return false;
    }
    const want: Want = {
      action: 'ohos.want.action.viewData',
      entities: ['entity.system.browsable'],
      uri: url,
      type: 'text/html',
    };
    try {
      await context.startAbility(want);
      // The URL itself carries a pairing id; log only that one opened.
      hilog.info(DOMAIN, TAG, 'opened the sign-in URL in the system browser');
      return true;
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'no browser could be started: %{public}s', JSON.stringify(error));
      return false;
    }
  }
}
