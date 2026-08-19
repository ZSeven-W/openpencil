/**
 * The handful of lucide glyphs the native SSO screens use, as raw SVG path
 * data on a 24x24 viewport — the exact `pathData` strings from the Android
 * shell's `res/drawable/ic_*.xml`, so both platforms draw the same marks.
 *
 * ArkUI has no vector-drawable resource type, but `Path` consumes SVG command
 * strings directly, so the icons stay stroke-scalable and theme-tinted instead
 * of being flattened into PNGs.
 */
export class LucideIcon {
  static readonly MAIL: string[] = [
    'M4 4h16a2 2 0 0 1 2 2v12a2 2 0 0 1 -2 2H4a2 2 0 0 1 -2 -2V6a2 2 0 0 1 2 -2Z',
    'm22 7 -8.97 5.7a1.94 1.94 0 0 1 -2.06 0L2 7',
  ];
  static readonly LOCK: string[] = [
    'M5 11h14a2 2 0 0 1 2 2v7a2 2 0 0 1 -2 2H5a2 2 0 0 1 -2 -2v-7a2 2 0 0 1 2 -2Z',
    'M7 11V7a5 5 0 0 1 10 0v4',
  ];
  static readonly SHIELD: string[] = [
    'M20 13c0 5 -3.5 7.5 -7.66 8.95a1 1 0 0 1 -0.67 -0.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1 ' +
      '-1c2 0 4.5 -1.2 6.24 -2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z',
  ];
  static readonly EYE: string[] = [
    'M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z',
    'M12 9a3 3 0 1 0 0 6 3 3 0 1 0 0-6Z',
  ];
  static readonly EYE_OFF: string[] = [
    'M9.88 9.88a3 3 0 1 0 4.24 4.24',
    'M10.73 5.08A10.43 10.43 0 0 1 12 5c7 0 10 7 10 7a13.16 13.16 0 0 1-1.67 2.68',
    'M6.61 6.61A13.526 13.526 0 0 0 2 12s3 7 10 7a9.74 9.74 0 0 0 5.39-1.61',
    'M2 2l20 20',
  ];
  static readonly X: string[] = [
    'M18 6 6 18',
    'm6 6 12 12',
  ];
  static readonly ARROW_LEFT: string[] = [
    'M19 12H5',
    'm12 19 -7 -7 7 -7',
  ];
}

/** One stroked lucide glyph, tinted by `color` and scaled to `size` vp. */
@Component
export struct LucideGlyph {
  @Prop paths: string[];
  @Prop color: string = '#FF000000';
  @Prop glyphSize: number = 18;
  @Prop strokeWidth: number = 2;

  build() {
    Shape() {
      ForEach(this.paths, (command: string) => {
        Path()
          .commands(command)
          .fill(Color.Transparent)
          .stroke(this.color)
          .strokeWidth(this.strokeWidth)
          .strokeLineCap(LineCapStyle.Round)
          .strokeLineJoin(LineJoinStyle.Round)
      }, (command: string) => command)
    }
    .viewPort({ x: 0, y: 0, width: 24, height: 24 })
    .width(this.glyphSize)
    .height(this.glyphSize)
  }
}
