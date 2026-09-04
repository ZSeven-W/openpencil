import { EngineLanguage, EngineLocale } from './EngineLanguage';

/**
 * Shell action 7: the engine's 15-language picker.
 *
 * The list is `EngineLanguage.all` verbatim — same codes, same order, same
 * display names as the Android and iOS shells and as `op_i18n::Locale::ALL`.
 * The engine applies the choice immediately; the shell persists it so the
 * next launch re-applies it right after engine create.
 */
@Component
export struct LanguagePickerDialog {
  @Link visible: boolean;
  @Prop currentCode: string;
  onSelect: (code: string) => void = () => {};

  build() {
    if (this.visible) {
      Column() {
        Column() {
          Text($r('app.string.language_picker_title'))
            .fontSize(18)
            .fontWeight(FontWeight.Medium)
            .fontColor('#FF1A1A1A')
            .margin({ top: 20, bottom: 12 })
          List() {
            ForEach(EngineLanguage.all, (locale: EngineLocale) => {
              ListItem() {
                Row() {
                  Text(locale.name)
                    .fontSize(16)
                    .fontColor('#FF1A1A1A')
                    .layoutWeight(1)
                    .textAlign(TextAlign.Start)
                  if (locale.code === this.currentCode) {
                    Text('✓').fontSize(16).fontColor('#FF0A66FF')
                  }
                }
                .width('100%')
                .height(48)
                .padding({ left: 24, right: 24 })
              }
              .onClick(() => {
                this.visible = false;
                this.onSelect(locale.code);
              })
            }, (locale: EngineLocale) => locale.code)
          }
          .width('100%')
          .layoutWeight(1)
          .scrollBar(BarState.Auto)

          Divider().color('#1F000000')
          Button($r('app.string.dialog_cancel'))
            .type(ButtonType.Normal)
            .backgroundColor(Color.Transparent)
            .fontColor('#FF0A66FF')
            .fontSize(16)
            .width('100%')
            .height(48)
            .onClick(() => {
              this.visible = false;
            })
        }
        .width(320)
        .height('70%')
        .backgroundColor('#FFFFFFFF')
        .borderRadius(16)
        .clip(true)
      }
      .width('100%')
      .height('100%')
      .backgroundColor('#66000000')
      .justifyContent(FlexAlign.Center)
      .alignItems(HorizontalAlign.Center)
    }
  }
}
