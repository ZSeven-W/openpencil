# frozen_string_literal: true

root = File.expand_path("../app/src/main/kotlin/tech/zseven/openpencil", __dir__)
store = File.read(File.join(root, "AndroidCollaborationCredentialStore.kt"))
envelope = File.read(File.join(root, "CollaborationCredentialEnvelope.kt"))
callbacks = File.read(File.join(root, "OpCallbacks.kt"))
implementation = File.read(File.join(root, "OpCallbacksImpl.kt"))
exclusive_file = File.read(File.join(root, "ExclusiveCredentialFile.kt"))
native = File.read(File.join(root, "OpNative.kt"))
surface = File.read(File.join(root, "OpSurfaceView.kt"))
manifest = File.read(File.expand_path("../app/src/main/AndroidManifest.xml", __dir__))

raise "collaboration credential must live in noBackupFilesDir" unless store.include?(
  'context.applicationContext.noBackupFilesDir',
)
raise "application backup must remain disabled" unless manifest.include?(
  'android:allowBackup="false"',
)
raise "nativeCreate must carry the sandbox root before editor mode" unless native.match?(
  /nativeCreate\(.*?receiver: OpCallbacks,.*?storageRoot: String,.*?mode: Int/m,
)
raise "engine config must live in a dedicated no-backup directory" unless surface.include?(
  'File(context.applicationContext.noBackupFilesDir, "config").absolutePath',
)
create_call = surface[/engine = OpNative\.nativeCreate\(.*?\n\s*\)/m]
raise "surface must pass private storage before engine construction" unless create_call&.match?(
  /callbacks,\s*privateStorageRoot,\s*if \(editorMode\)/m,
)
raise "wrapping key alias must use the canonical app namespace" unless store.include?(
  'tech.zseven.openpencil.collaboration.device-key-wrapping-v1',
)
raise "wrapping key must be owned by AndroidKeyStore" unless store.include?(
  'private const val ANDROID_KEY_STORE = "AndroidKeyStore"',
)
raise "wrapping key must be AES-GCM and non-exportable" unless store.match?(
  /AES\/GCM\/NoPadding.*?key\.encoded != null/m,
)
raise "wrapping key must require randomized encryption" unless store.include?(
  '.setRandomizedEncryptionRequired(true)',
)
raise "missing ciphertext may initialize only when its wrapping alias is also absent" unless store.match?(
  /if \(!credentialFile\.exists\(\)\) \{\s*requireWrappingKeyAbsent\(\)\s*return@withStoreLock null/m,
)
raise "store must recheck the wrapping alias before first creation" unless store.match?(
  /private fun createWrappingKey\(\).*?requireWrappingKeyAbsent\(\)/m,
)
raise "credential install must use exclusive atomic creation" unless exclusive_file.include?(
  'StandardOpenOption.CREATE_NEW',
)
raise "credential install must reject symlinks" unless exclusive_file.include?(
  'LinkOption.NOFOLLOW_LINKS',
)
raise "credential install must durably flush its winner" unless exclusive_file.include?(
  'channel.force(true)',
)
raise "credential install must never use replacement move semantics" if [store, exclusive_file].any? do |source|
  source.include?('StandardCopyOption') || source.include?('Files.move(')
end
raise "credential crash residue must remain fail-closed" unless exclusive_file.include?(
  'short file; it is deliberately retained',
)
raise "credential file and lock must reject symlinks" unless store.scan(
  'LinkOption.NOFOLLOW_LINKS',
).length >= 1
raise "credential locking must serialize threads before taking the process file lock" unless store.match?(
  /synchronized\(PROCESS_LOCK\).*?channel\.lock\(\)/m,
)
raise "GCM authentication failures must fail closed" unless store.include?(
  'catch (error: AEADBadTagException)',
)
raise "envelope version, nonce, tag, and private-key sizes must be pinned" unless [
  'private const val VERSION: Byte = 1',
  'const val NONCE_BYTES = 12',
  'const val TAG_BYTES = 16',
  'const val PRIVATE_KEY_BYTES = 32',
].all? { |needle| envelope.include?(needle) }
raise "credential callbacks must expose load and create-if-absent" unless [
  'fun onCredentialLoad(): ByteArray?',
  'fun onCredentialStoreIfAbsent(value: ByteArray): Boolean',
].all? { |needle| callbacks.include?(needle) }
raise "store callback must wipe its plaintext input" unless implementation.match?(
  /onCredentialStoreIfAbsent.*?finally.*?value\.fill\(0\)/m,
)
raise "secure credential code must not log secrets" if [store, envelope, exclusive_file].any? do |source|
  source.include?("android.util.Log") || source.include?("println(")
end

puts "Android collaboration credential storage contract validates"
