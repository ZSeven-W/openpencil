/**
 * Two-button modal in the shape of `NoticeDialog`, for the two auth decisions
 * that need a confirmation: switching the sign-in region (which only takes
 * effect after a restart) and signing out.
 *
 * State-driven rather than a `promptAction` toast, so an engine-terminal shell
 * action can dismiss it programmatically.
 */
@Component
export struct ConfirmDialog {
  @Link visible: boolean;
  @Prop title: ResourceStr;
  @Prop message: ResourceStr;
  @Prop confirmLabel: ResourceStr;
  @Prop cancelLabel: ResourceStr;
  onConfirm: () => void = () => {};

  build() {
    if (this.visible) {
      Column() {
        Column() {
          if (this.title !== '') {
            Text(this.title)
              .fontSize(17)
              .fontWeight(FontWeight.Medium)
              .fontColor('#FF1A1A1A')
              .textAlign(TextAlign.Center)
              .margin({ top: 22, left: 24, right: 24 })
          }
          Text(this.message)
            .fontSize(15)
            .fontColor('#FF4A4A4A')
            .textAlign(TextAlign.Center)
            .margin({ top: 10, bottom: 18, left: 24, right: 24 })
          Divider().color('#1F000000')
          Row() {
            Button(this.cancelLabel)
              .type(ButtonType.Normal)
              .backgroundColor(Color.Transparent)
              .fontColor('#FF616161')
              .fontSize(16)
              .layoutWeight(1)
              .height(48)
              .onClick(() => {
                this.visible = false;
              })
            Divider().vertical(true).color('#1F000000').height(48)
            Button(this.confirmLabel)
              .type(ButtonType.Normal)
              .backgroundColor(Color.Transparent)
              .fontColor('#FF0A66FF')
              .fontSize(16)
              .layoutWeight(1)
              .height(48)
              .onClick(() => {
                this.visible = false;
                this.onConfirm();
              })
          }
          .width('100%')
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
