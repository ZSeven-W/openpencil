import { AuthTheme } from './AuthTheme';
import { LucideGlyph, LucideIcon } from './LucideIcon';

/**
 * One labeled, boxed input of the ZSeven sign-in design: a bold field label
 * above a rounded hairline-bordered row carrying a lucide glyph, the input,
 * and — for passwords — an eye toggle. Mirrors `AuthUi.boxedField` +
 * `AuthUi.fieldLabel` in the Android shell.
 */
@Component
export struct AuthField {
  @Prop label: ResourceStr;
  @Prop placeholder: ResourceStr;
  @Prop icon: string[] = LucideIcon.MAIL;
  @Prop night: boolean = false;
  @Prop password: boolean = false;
  @Prop numeric: boolean = false;
  @Prop active: boolean = true;
  @Link text: string;
  @State private revealed: boolean = false;
  /** Optional trailing control (the register form's send-code link). */
  @BuilderParam trailing: () => void = this.noTrailing;

  @Builder
  noTrailing() {
  }

  private inputType(): InputType {
    if (this.password) {
      return this.revealed ? InputType.Normal : InputType.Password;
    }
    if (this.numeric) {
      return InputType.Number;
    }
    return InputType.Email;
  }

  build() {
    Column() {
      Text(this.label)
        .fontSize(14)
        .fontWeight(FontWeight.Bold)
        .fontColor(AuthTheme.text(this.night))
        .width('100%')
        .textAlign(TextAlign.Start)
        .padding({ top: 10, bottom: 6 })

      Row() {
        LucideGlyph({
          paths: this.icon,
          color: AuthTheme.secondary(this.night),
          glyphSize: 18,
        })
          .margin({ right: 10 })

        TextInput({ text: this.text, placeholder: this.placeholder })
          .type(this.inputType())
          .enabled(this.active)
          .backgroundColor(Color.Transparent)
          .borderRadius(0)
          .padding(0)
          .fontSize(16)
          .fontColor(AuthTheme.text(this.night))
          .placeholderColor(AuthTheme.secondary(this.night))
          .layoutWeight(1)
          .onChange((value: string) => {
            this.text = value;
          })

        if (this.password) {
          Button({ type: ButtonType.Normal }) {
            LucideGlyph({
              paths: this.revealed ? LucideIcon.EYE : LucideIcon.EYE_OFF,
              color: AuthTheme.secondary(this.night),
              glyphSize: 20,
            })
          }
          .backgroundColor(Color.Transparent)
          .padding(8)
          .onClick(() => {
            this.revealed = !this.revealed;
          })
        }

        this.trailing()
      }
      .width('100%')
      .constraintSize({ minHeight: 52 })
      .padding({ left: 14, right: 6 })
      .borderRadius(12)
      .borderWidth(1)
      .borderColor(AuthTheme.border(this.night))
      .backgroundColor(AuthTheme.fieldFill(this.night))
      .alignItems(VerticalAlign.Center)
    }
    .width('100%')
  }
}
