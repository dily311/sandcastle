# Add project specific ProGuard rules here.
# You can control the set of applied configuration files using the
# proguardFiles setting in build.gradle.
#
# For more details, see
#   http://developer.android.com/guide/developing/tools/proguard.html

# If your project uses WebView with JS, uncomment the following
# and specify the fully qualified class name to the JavaScript interface
# class:
#-keepclassmembers class fqcn.of.javascript.interface.for.webview {
#   public *;
#}

# Uncomment this to preserve the line number information for
# debugging stack traces.
#-keepattributes SourceFile,LineNumberTable

# If you keep the line number information, uncomment this to
# hide the original source file name.
#-renamesourcefileattribute SourceFile
# 화면(JS)이 부르는 앱 함수 — 배포 빌드에서 이름이 바뀌면 상태 표시줄 높이를 못 읽는다 (MainActivity.kt, 1-C-2)
-keepclassmembers class com.dilly311.sandcastle.SystemInsets {
   @android.webkit.JavascriptInterface <methods>;
}
# 동기화의 "Wi-Fi에서만"이 부르는 함수 (DriveAuth.kt, 2-D)
-keepclassmembers class com.dilly311.sandcastle.NetworkState {
   @android.webkit.JavascriptInterface <methods>;
}
