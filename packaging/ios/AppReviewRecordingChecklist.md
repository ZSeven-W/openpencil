# iOS Guideline 2.1 补充材料与真机录屏

对应英文回复及 Notes 草稿：[AppReviewNotes.txt](AppReviewNotes.txt)。必须按实际提交的构建补全，不能把源码检查或模拟器运行写成真机验收。当前这份文件是执行清单，不是已经完成的验收记录。

## 当前阻塞项

| 项目 | 当前证据与所需动作 |
| --- | --- |
| 真机 QA 与录屏 | 本次仅检测到模拟器。需要实际 iPhone 和 iPad 测试记录；录屏设备运行 Apple 要求的最新正式系统。 |
| 账户删除 | 已按“仅注销 OpenPencil，保留共享 ZSeven 身份及其他产品”实现源代码；仍需部署两区服务、数据库迁移及新签名认证库，再用新构建完成端到端真机验收。详见 [ACCOUNT_DELETION.md](ACCOUNT_DELETION.md)。 |
| 审核登录 | 尚无已验证的审核账号。准备可持续使用且有足够权限/额度的账号，凭证只填 App Store Connect 的审核字段，不提交到 Git。 |
| Hosted AI / 付费 | 源码有 Hosted 账号权益访问，iOS 未实现 StoreKit 购买流程。必须确认实际收费与权益来源、适用的购买方式以及审核账号访问，不得写成“没有付费功能”。 |
| 协作与用户内容 | 通过邀请/地址加入，所有者接受或拒绝编辑者/查看者，访客确认所有者；未发现公开信息流。目前没有举报、屏蔽或移除已加入参与者的 UI。需要核对适用要求并补齐，不能直接填写“不适用”。 |
| 第三方材料 | 已有项目和字体许可证文件，但尚未完成随包字体、图标、模板、SDK 及其他材料的完整授权核对。可选图片搜索使用 Openverse / Wikimedia；搜索来源不能代替每张图片的许可及署名核对。 |

## 录制前

- 安装将提交审核的同一版本和 build，记录 TestFlight 构建号、设备型号、系统版本和测试日期。代码修复必须进入新构建；旧构建不会随源码更新。
- iPhone、iPad 都在当前支持范围内。分别测试启动、编辑、键盘、文件选择/保存/导出、登录、旋转及后台返回；保留结果和失败复现步骤。
- 先按删除功能发布依赖完成部署，再用一次性测试账号验证。保持审核账号可用，勿用审核账号演示删除。用户将用 iPhone 录制；iPad 仍需独立真机测试。
- 将 [Resources/sample.op](Resources/sample.op) 作为审核附件或提供可直接下载的地址，并在真机“文件”中准备好。该示例不会在正常启动时自动显示；通过 More > Open File 打开。
- 审核账号在录屏地区可登录，并实际完成一次 Hosted AI 请求；仅看到模型列表不足以证明可以调用。
- 本次源码验证 `bash packaging/ios/Tests/validate_sources.sh` 已通过；未生成新的 app/archive，也未做真机 QA。两地区隐私政策网页已返回 HTTP 200，内容一致；仍要在新构建中实测打开。
- 不在录像里暴露密码、验证码、恢复码或私人设计。输入敏感信息时使用适当遮挡，但保留完整操作顺序和成功/失败结果。

## 一次典型流程的录屏顺序

1. 从设备主屏幕开始录制，点击 OpenPencil 图标，等待真实启动完成。记录语言和登录地区；不要从已打开的画布或模拟器开始。
2. 打开 More > New File，创建文本和形状，修改文字、颜色或位置，演示撤销/重做与缩放。若实际启动进入其他首页，先录出进入编辑器的真实路径，并同步修改英文步骤。
3. More > Save / Save As，通过系统“文件”保存；More > Open File 再次打开，确认内容一致。再打开提供给审核员的 `sample.op`。
4. More > Export，选择 PNG、JPEG、SVG 或 PDF，展示系统保存界面和导出结果。iOS 不提供 WebP；iPad 额外测试其 Code 面板，别把桌面专属功能写进 iPhone 流程。
5. More > Sign In，展示隐私政策入口、创建账户、验证邮箱及成功登录。用一次性邮箱完成注册；另演示已有审核账号的登录路径和 More > Account。若第三方登录在该地区提供，测试其实际返回流程。
6. More > AI，选取该账号实际可用的 Hosted 模型，输入简短设计请求，等待真实结果，并展示结果可编辑。录出收费/权益入口及访问付费功能的实际步骤；若没有购买入口，说明其真实业务模式，不虚构 IAP 或绕过要求。
7. 如果本构建允许用户向他人分享内容或协作，展示完整加入/共享流程和对应权限；根据适用情况录制举报内容、屏蔽用户及处理结果。相关能力欠缺时先修复，勿用文字代替未实现的机制。
8. 用一次性测试账号，从 More > Account > Delete OpenPencil Account 进入删除流程，展示范围提示、身份确认、最终确认、“处理中”及完成结果。旧协作凭据需要 16 分钟失效等待，两区清理失败时可能更久；可以提供有时间标记的补充录像。确认 OpenPencil 专用账户及数据已清理，同时共享 ZSeven 身份、其他产品和钱包仍可用。不要声称通用共享 LLM 权限也已撤销。
9. 重新启动应用，确认删除后的本地功能仍正常。展示任何需要联网/账号的功能如何提示用户。

可以按设备或区域提供补充录像，但每段都标明对应 build、设备和系统。最终视频必须是实际设备操作记录；链接应能直接播放，不要求审核员注册额外账号。

## 验收记录（完成后填写）

| 设备 / 系统 | 版本 / build | 地区 | 启动编辑与文件 | 注册登录删除 | Hosted / 付费访问 | 协作 | 证据 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| iPhone：待填 | 待填 | 待填 | 未测 | 未测 | 未测 | 未测 | 视频/日志待填 |
| iPad：待填 | 待填 | 待填 | 未测 | 未测 | 未测 | 未测 | 视频/日志待填 |

如某项不适用，写出提交构建中的依据及复核人，不把“未测”改写成“不适用”。模型、地区和权益由服务端决定时记录测试日期及所用账号类型，不将敏感凭证写入表格。

## 填写与提交

1. 补全英文草稿六项，删除所有占位符与草稿警告；每个“已完成”都要能对应上面的证据。
2. 将相同内容填入 App Store Connect 的审核回复和 App Review Information > Notes。登录凭证填专用审核字段；多个账号类型需要分别提供访问说明。
3. 上传/链接真机录屏、示例文件，以及适用的资质或授权材料。核对链接在未登录浏览器可访问。
4. 选择包含修复的构建，检查实际界面截图和购买项目配置，再进行提审。上传成功、提交成功、等待审核和审核通过是不同状态，应分别记录。

## 证据入口

- [Apple App Review Guidelines](https://developer.apple.com/app-store/review/guidelines/)：2.1 完整性/审核访问、2.3.3 截图、3.1.1 购买及 5.1.1 隐私。
- [Apple 账户删除说明](https://developer.apple.com/help/app-review/guideline-reference/5-1-1-account-deletion)：支持创建账户时需提供应用内发起删除的路径；若网站完成删除，应直达删除页面。
- [iOS README](README.md)：构建、认证依赖、文件操作与验证边界。
- [设备支持配置](project.yml)：`TARGETED_DEVICE_FAMILY: "1,2"`。
- [More 菜单](../../crates/op-editor-ui/src/widgets/mobile_more_panel.rs) 与 [触摸入口处理](../../crates/op-host-native/src/widget_host/press_chrome_tiers.rs)：文件、AI、登录/账户及协作入口。
- [地区配置](Sources/SsoRegion.swift)、[SSO 客户端](Sources/SsoAuthClient.swift)、[账户页](Sources/AccountCenterViewController.swift)、[注册页](Sources/RegisterViewController.swift)：地区、登录注册及账户操作的源码依据。
- [Hosted 运行时](../../crates/op-hosted-models/src/lib.rs) 与 [Hosted 认证桥](../../crates/op-auth-bridge/src/real_hosted_access.rs)：登录账号提供访问与模型服务，不能从公开源码推断某个付费账号已获授权。
- [图片搜索](../../crates/op-engine-ffi/src/editor_image_search.rs)：可选 Openverse / Wikimedia 图片来源。
- [项目许可证](../../LICENSE)、[字体声明](../shared/fonts/NOTICE.txt)、[私有认证桥声明](../../crates/op-auth-bridge/NOTICE)：授权核对起点，不代表全部第三方材料已审计完成。
