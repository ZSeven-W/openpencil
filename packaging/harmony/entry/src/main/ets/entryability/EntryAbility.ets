import { AbilityConstant, UIAbility, Want } from '@kit.AbilityKit';
import { window } from '@kit.ArkUI';
import { deviceInfo } from '@kit.BasicServicesKit';
import { hilog } from '@kit.PerformanceAnalysisKit';
import { WindowBridge } from '../common/WindowBridge';

const DOMAIN = 0x0001;
const TAG = 'OpenPencilPlayer';

/**
 * Hosts the single OpenPencil window.
 *
 * The window is edge-to-edge: the engine's surface spans the full window
 * including the status bar, navigation area, and any display cutout, while
 * the safe-area bands are forwarded to the engine SEPARATELY so only its
 * interactive chrome avoids system UI. Window resizes (rotation, folding,
 * multi-window, 2in1 free-form drag) publish a new viewport tuple through
 * `WindowBridge`, which the page turns into one atomic engine resize.
 *
 * The ability itself never touches the engine; `pages/Index` owns it.
 */
export default class EntryAbility extends UIAbility {
  onCreate(_want: Want, _launchParam: AbilityConstant.LaunchParam): void {
    hilog.info(DOMAIN, TAG, 'ability create');
  }

  onWindowStageCreate(windowStage: window.WindowStage): void {
    windowStage.loadContent('pages/Index', (error) => {
      if (error.code) {
        hilog.error(DOMAIN, TAG, 'loadContent failed: %{public}s', JSON.stringify(error));
        return;
      }
      hilog.info(DOMAIN, TAG, 'content loaded');
    });

    windowStage
      .getMainWindow()
      .then((mainWindow: window.Window) => {
        // Desktop-class free windows drop the system decor bar: the engine
        // paints its own top bar, so the window reads as borderless. Phones
        // and tablets have no decor and the call is skipped.
        if (deviceInfo.deviceType === '2in1') {
          try {
            mainWindow.setWindowDecorVisible(false);
          } catch (error) {
            hilog.warn(DOMAIN, TAG, 'decor hide unavailable: %{public}s', String(error));
          }
          // Hiding the decor bar leaves the platform's maximise / minimise /
          // close buttons FLOATING over the window's top-right corner, right
          // on top of the engine's own right-hand TopBar cluster. The engine
          // paints its own traffic lights (shell actions 8/9/10), so retire
          // the system trio instead of reserving space around it.
          hilog.info(
            DOMAIN,
            TAG,
            'title buttons hidden, residual reserve=%{public}d vp',
            WindowBridge.shared.hideTitleButtons(mainWindow)
          );
        }
        // Keep the surface behind the system bars, then start publishing the
        // four-sided safe area and keyboard occlusion to the page.
        return WindowBridge.shared.attach(mainWindow);
      })
      .catch((error: Object) => {
        hilog.error(DOMAIN, TAG, 'main window setup failed: %{public}s', JSON.stringify(error));
      });
  }

  onWindowStageDestroy(): void {
    WindowBridge.shared.detach();
  }

  onForeground(): void {
    // The window may have been resized or folded while backgrounded.
    WindowBridge.shared.refresh();
  }

  onDestroy(): void {
    hilog.info(DOMAIN, TAG, 'ability destroy');
  }
}
