# frozen_string_literal: true

player_dir = File.expand_path("..", __dir__)
repo_dir = File.expand_path("../..", player_dir)
source_dir = File.join(player_dir, "app/src/main/kotlin/tech/zseven/openpencil")
activity = File.read(File.join(source_dir, "MainActivity.kt"))
surface = File.read(File.join(source_dir, "OpSurfaceView.kt"))
surface_bridge = File.read(File.join(source_dir, "OpSurfaceViewShellBridge.kt"))
native = File.read(File.join(source_dir, "OpNative.kt"))
picker = File.read(File.join(source_dir, "ChatAttachmentPicker.kt"))
header = File.read(File.join(repo_dir, "crates/op-engine-ffi/include/op_engine.h"))
jni = File.read(File.join(repo_dir, "crates/op-engine-jni/src/bindings_chat_attachment.rs"))

raise "C header must declare action 13" unless header.include?("OpShellAction_PickChatAttachment = 13,")
raise "Android chat-attachment action missing" unless native.include?("SHELL_ACTION_PICK_CHAT_ATTACHMENT = 13")
raise "Android chat-attachment JNI declaration missing" unless native.include?("external fun nativeEditorAttachChatImage(")
raise "JNI export missing" unless jni.include?("Java_tech_zseven_openpencil_OpNative_nativeEditorAttachChatImage")

action = surface_bridge[/fun pollShellAction\(\).*?(?=\n    \/\*\*)/m]
raise "shell-action drain missing" unless action
branch = action.index("SHELL_ACTION_PICK_CHAT_ATTACHMENT")
post = action.index("post {", branch || 0)
invoke = action.index("pickChatAttachmentHandler?.invoke()", branch || 0)
unless branch && post && invoke && branch < post && post < invoke
  raise "photo picker must be presented after the editor JNI stack unwinds"
end

raise "surface must defer the photo picker to the Activity" unless surface.include?("setPickChatAttachmentHandler")
raise "Activity must register the photo picker handler" unless activity.include?("setPickChatAttachmentHandler(chatAttachmentPicker::launch)")
raise "picker must use the system Photo Picker" unless picker.include?("ActivityResultContracts.PickVisualMedia()")
raise "picker must request images only" unless picker.include?("PickVisualMedia.ImageOnly")
raise "picker must suppress concurrent launches" unless picker.include?("if (inProgress ||")
raise "picker cancellation must retire ownership" unless picker.match?(/registerForActivityResult.*?uri == null.*?inProgress = false/m)
raise "photo bytes must be read off the main thread" unless picker.include?('"OpenPencilChatAttachment"')
raise "HEIC / oversized photos must be re-encoded" unless picker.include?("ChatAttachmentImage.needsReencode(bytes)") &&
  picker.include?("Bitmap.CompressFormat.JPEG")
raise "raw picker input must be bounded" unless picker.include?("ChatAttachmentImage.MAX_INPUT_BYTES")

return_bytes = surface_bridge[/fun attachChatImage\(.*?(?=\n    \/\*\*)/m]
raise "attachment ABI return missing" unless return_bytes&.include?("nativeEditorAttachChatImage")
raise "attachment ABI return must repaint the strip" unless return_bytes.include?("requestFrame()")
raise "teardown must drop the photo picker handler" unless surface_bridge.match?(/fun releaseHandlers\(\).*?pickChatAttachmentHandler = null/m)

puts "Android chat attachment contract validates"
