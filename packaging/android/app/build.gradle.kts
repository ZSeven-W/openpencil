plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

val repositoryRoot = rootProject.layout.projectDirectory.dir("../..")
val androidVersionOutput = providers.exec {
    workingDir(repositoryRoot.asFile)
    commandLine(
        repositoryRoot.file("scripts/android-version.sh").asFile.absolutePath,
        repositoryRoot.file("Cargo.toml").asFile.absolutePath,
    )
}.standardOutput.asText.get().trim()
val androidVersionLines = androidVersionOutput.lines()
check(androidVersionLines.size == 2) {
    "scripts/android-version.sh returned invalid version metadata"
}
val canonicalVersionName = Regex("""versionName=([0-9]+\.[0-9]+\.[0-9]+)""")
    .matchEntire(androidVersionLines[0])?.groupValues?.get(1)
    ?: error("scripts/android-version.sh returned an invalid versionName")
val canonicalVersionCode = Regex("""versionCode=([1-9][0-9]*)""")
    .matchEntire(androidVersionLines[1])?.groupValues?.get(1)?.toIntOrNull()
    ?: error("scripts/android-version.sh returned a non-numeric versionCode")

android {
    namespace = "tech.zseven.openpencil"
    compileSdk = 36
    buildToolsVersion = "36.0.0"
    ndkVersion = "28.2.13676358"

    defaultConfig {
        applicationId = "tech.zseven.openpencil"
        minSdk = 26
        targetSdk = 36
        versionCode = canonicalVersionCode
        versionName = canonicalVersionName
        ndk {
            // The op-engine-jni cdylib is built by cargo-ndk into jniLibs;
            // ship only the ABIs it produces.
            abiFilters += listOf("arm64-v8a", "x86_64")
        }
    }

    buildTypes {
        debug {
            isDebuggable = true
        }
        release {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }

    buildFeatures {
        buildConfig = true
    }

    // Native libraries are populated out-of-band by cargo-ndk. Keep Debug and
    // Release in distinct source sets so an unsigned local auth build can
    // never be consumed by a shipping variant.
    sourceSets["main"].jniLibs.setSrcDirs(emptyList<String>())
    sourceSets["debug"].jniLibs.srcDirs("src/debug/jniLibs")
    sourceSets["release"].jniLibs.srcDirs("src/release/jniLibs")

    lint {
        abortOnError = true
        checkReleaseBuilds = true
    }
}

dependencies {
    implementation("androidx.activity:activity-ktx:1.7.0")
    implementation("androidx.core:core-ktx:1.13.1")
    implementation("androidx.appcompat:appcompat:1.7.0")
    // Native third-party sign-in SDKs: Douyin OpenSDK (auth code via the
    // Douyin app) and the Alipay SDK (in-app authorization). The auth codes
    // they mint are exchanged server-side by the SSO backend.
    // Pinned to 0.2.0.10: 0.2.0.11 depends on com.bytedance.security:polaris,
    // which ByteDance does not publish on any reachable repository.
    implementation("com.bytedance.ies.ugc.aweme:opensdk-china-external:0.2.0.10")
    implementation("com.bytedance.ies.ugc.aweme:opensdk-common:0.2.0.10")
    implementation("com.alipay.sdk:alipaysdk-android:15.8.42")
    implementation("com.tencent.mm.opensdk:wechat-sdk-android:6.8.40")
    testImplementation("junit:junit:4.13.2")
    // Real org.json for JVM unit tests (the android.jar copy is a stub).
    testImplementation("org.json:json:20240303")
}

tasks.register("printOpenPencilVersion") {
    group = "verification"
    description = "Print the canonical Android version metadata"
    doLast {
        println("versionName=$canonicalVersionName")
        println("versionCode=$canonicalVersionCode")
    }
}
