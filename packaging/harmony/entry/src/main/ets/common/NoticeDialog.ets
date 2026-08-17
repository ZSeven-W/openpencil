/**
 * A minimal, engine-independent notice sheet.
 *
 * The engine paints the entire editor UI, so the shell owns exactly one
 * piece of chrome: a modal message the platform must show when a shell
 * action cannot be served (today: sign-in and the account center, because
 * the private `op-auth` runtime has no HarmonyOS build yet).
 *
 * It is state-driven rather than a `promptAction` toast so that shell action
 * 3 (CloseLoginWebView) can dismiss it programmatically.
 */
@Component
export struct NoticeDialog {
  @Link visible: boolean;
  @Prop message: ResourceStr;
  onDismiss: () => void = () => {};

  build() {
    if (this.visible) {
      Column() {
        Column() {
          Text(this.message)
            .fontSize(16)
            .fontColor('#FF1A1A1A')
            .textAlign(TextAlign.Center)
            .margin({ top: 24, bottom: 20, left: 24, right: 24 })
          Divider().color('#1F000000')
          Button($r('app.string.notice_dismiss'))
            .type(ButtonType.Normal)
            .backgroundColor(Color.Transparent)
            .fontColor('#FF0A66FF')
            .fontSize(16)
            .width('100%')
            .height(48)
            .onClick(() => {
              this.visible = false;
              this.onDismiss();
            })
        }
        .width(300)
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
