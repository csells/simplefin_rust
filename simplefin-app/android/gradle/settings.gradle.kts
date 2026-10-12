// Gradle settings for the SimpleFIN Android app.
//
// Packages the Rust library libsimplefin_app.so (built separately; see
// simplefin-app/README.md) into an APK hosted by gpui-mobile's GpuiActivity,
// a NativeActivity. The library must already be in
// app/src/main/jniLibs/arm64-v8a/ when Gradle runs.

pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "SimpleFIN"
include(":app")
