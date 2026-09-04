import { hilog } from '@kit.PerformanceAnalysisKit';
import { fileIo as fs } from '@kit.CoreFileKit';
import napi from './OpNative';

const DOMAIN = 0x0001;
const TAG = 'OpenPencilPlayer';

/**
 * Registers the platform's CJK / Thai / Devanagari system fonts with a
 * freshly created engine.
 *
 * Split out of `EngineHost` only because that file sits at the repo's
 * 800-line cap; it is otherwise unchanged, and `EngineHost.create` remains
 * its only caller.
 */
export class SystemFontRegistry {
  /**
   * skia has no OHOS system font manager, so without this the engine can
   * paint only its embedded Latin font and CJK chrome strings render as
   * tofu. HarmonyOS Sans SC carries the CJK repertoire; the engine's
   * imported-font fallback chain picks it up per codepoint.
   */
  static registerCjkFonts(engine: number): void {
    // Coverage for every non-Latin chrome locale: Simplified/Traditional
    // Chinese, Japanese+Korean (the CJK collection), Thai, and Devanagari.
    // HarmonyOS Sans itself covers Latin/Cyrillic/Greek.
    const paths: string[] = [
      '/system/fonts/HarmonyOS_Sans_SC.ttf',
      '/system/fonts/HarmonyOS_Sans_TC.ttf',
      '/system/fonts/NotoSansCJK-Regular.ttc',
      '/system/fonts/NotoSansThai[wdth,wght].ttf',
      '/system/fonts/NotoSansDevanagariUI-Regular.ttf',
    ];
    for (const path of paths) {
      try {
        const stat = fs.statSync(path);
        const file = fs.openSync(path, fs.OpenMode.READ_ONLY);
        const bytes = new ArrayBuffer(stat.size);
        fs.readSync(file.fd, bytes);
        fs.closeSync(file);
        const status: number = napi.registerFont(engine, bytes);
        hilog.info(
          DOMAIN,
          TAG,
          'system font registered %{public}s',
          `${path} (${stat.size} bytes) status=${status}`
        );
      } catch (error) {
        hilog.warn(DOMAIN, TAG, 'system font unavailable %{public}s', `${path}: ${error}`);
      }
    }
  }
}
