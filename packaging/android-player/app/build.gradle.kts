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
    compileSdk = 35

    defaultConfig {
        applicationId = "tech.zseven.openpencil"
        minSdk = 26
        targetSdk = 34
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
        abortOnError = false
    }
}

dependencies {
    implementation("androidx.activity:activity-ktx:1.7.0")
    implementation("androidx.core:core-ktx:1.13.1")
    implementation("androidx.appcompat:appcompat:1.7.0")
    testImplementation("junit:junit:4.13.2")
}

tasks.register("printOpenPencilVersion") {
    group = "verification"
    description = "Print the canonical Android version metadata"
    doLast {
        println("versionName=$canonicalVersionName")
        println("versionCode=$canonicalVersionCode")
    }
}
