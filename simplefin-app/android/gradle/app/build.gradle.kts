// App module build for the SimpleFIN Android app.
//
// Packages the Rust library libsimplefin_app.so (built separately; see
// simplefin-app/README.md) into an APK hosted by gpui-mobile's GpuiActivity,
// a NativeActivity. The library must already be in
// app/src/main/jniLibs/arm64-v8a/ when Gradle runs.

plugins {
    id("com.android.application")
}

android {
    namespace = "com.sellsbrothers.simplefin"
    compileSdk = 34

    defaultConfig {
        applicationId = "com.sellsbrothers.simplefin"
        minSdk = 26          // Vulkan 1.0 is mandatory from API 26+
        targetSdk = 34
        versionCode = 1
        versionName = "1.0.0"

        // Tell NativeActivity which .so to load.
        // Must match the cdylib name in simplefin-app/Cargo.toml.
        ndk {
            abiFilters += listOf("arm64-v8a")
        }

        // Forward the library name to the manifest via a placeholder.
        manifestPlaceholders["nativeLibraryName"] = "simplefin_app"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
        debug {
            isDebuggable = true
            isJniDebuggable = true
        }
    }

    // We do NOT use CMake / ndk-build — the native library is compiled
    // externally via cargo-ndk and placed directly into jniLibs.
    //
    // Disable the built-in native build system so Gradle doesn't look for
    // a CMakeLists.txt or Android.mk.
    externalNativeBuild {
        // Intentionally left empty.
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }

    // Tell Gradle where the pre-built .so files live.
    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
        }
    }

    packaging {
        // Prevent stripping of the Rust library — cargo already strips in
        // release mode and stripping again can break backtraces.
        jniLibs {
            keepDebugSymbols += listOf(
                "*/arm64-v8a/libsimplefin_app.so",
                "*/armeabi-v7a/libsimplefin_app.so",
                "*/x86_64/libsimplefin_app.so",
                "*/x86/libsimplefin_app.so"
            )
        }
    }

    // Lint configuration.
    lint {
        abortOnError = false
        checkReleaseBuilds = false
    }
}

dependencies {
    // AndroidX core for NotificationCompat (used by GpuiNotifications)
    implementation("androidx.core:core:1.12.0")
    // AndroidX SplashScreen compat (used by GpuiActivity to hold splash until native init)
    implementation("androidx.core:core-splashscreen:1.0.1")
    // AndroidX Biometric for BiometricPrompt (used by GpuiAuthActivity)
    implementation("androidx.biometric:biometric:1.1.0")
    // AndroidX Media for MediaSessionCompat (used by GpuiMediaSession for system controls)
    implementation("androidx.media:media:1.7.1")
}
