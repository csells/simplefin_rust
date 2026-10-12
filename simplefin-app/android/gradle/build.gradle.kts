// Root Gradle build for the SimpleFIN Android app.
//
// Packages the Rust library libsimplefin_app.so (built separately; see
// simplefin-app/README.md) into an APK hosted by gpui-mobile's GpuiActivity,
// a NativeActivity. The library must already be in
// app/src/main/jniLibs/arm64-v8a/ when Gradle runs.

buildscript {
    repositories {
        google()
        mavenCentral()
    }
    dependencies {
        classpath("com.android.tools.build:gradle:9.1.0")
        classpath("org.jetbrains.kotlin:kotlin-gradle-plugin:1.9.22")
    }
}

tasks.register("clean", Delete::class) {
    delete(rootProject.layout.buildDirectory)
}

// ── Convenience task: build Rust + APK in one go ────────────────────────────
