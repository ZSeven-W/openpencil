plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "dev.openpencil.player"
    compileSdk = 35

    defaultConfig {
        applicationId = "dev.openpencil.player"
        minSdk = 26
        targetSdk = 34
        versionCode = 1
        versionName = "0.1.0"
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
