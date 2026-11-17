package com.dilly311.sandcastle

import android.os.Bundle
import android.view.View
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.webkit.WebViewCompat
import androidx.webkit.WebViewFeature
import com.dilly311.sandcastle.background.GenerationService

/** 화면이 읽는 상태 표시줄 높이 (CSS px). 숫자만 돌려준다 */
class SystemInsets {
  @Volatile var topCss: Float = 0f

  @JavascriptInterface
  fun top(): Float = topCss
}

class MainActivity : TauriActivity() {
  private val systemInsets = SystemInsets()
  private var webView: WebView? = null
  // 구글 드라이브 권한 (2-D). 동의 화면 결과를 받으려면 생성될 때 등록해야 해서 필드로
  private val driveAuth = DriveAuth(this)

  override fun onWebViewCreate(webView: WebView) {
    this.webView = webView
    webView.addJavascriptInterface(systemInsets, "SandcastleInsets")
    // 참거짓 하나만 돌려준다 (데이터 요금이 드는 연결인가)
    webView.addJavascriptInterface(NetworkState(applicationContext), "SandcastleNet")

    // 백그라운드 서비스 켜고 끄기 (src/app/keepAlive.ts). 앱 자기 주소에서만 받는다 —
    // AI가 쓴 HTML(htmlview iframe)은 다른 출처라 이 객체가 아예 없다 (Least Privilege)
    if (WebViewFeature.isFeatureSupported(WebViewFeature.WEB_MESSAGE_LISTENER)) {
      WebViewCompat.addWebMessageListener(webView, "SandcastleNative", setOf("http://tauri.localhost")) { _, message, _, isMainFrame, reply ->
        if (!isMainFrame) return@addWebMessageListener
        val data = message.data ?: return@addWebMessageListener
        when {
          data == "keepalive:start" -> GenerationService.start(applicationContext, "응답을 만드는 중")
          data == "keepalive:stop" -> GenerationService.stop(applicationContext)
          // 구글 드라이브 권한: drive:authorize:<id>:<1이면 동의 화면을 띄워도 됨> (src/app/androidDrive.ts)
          data.startsWith("drive:authorize:") -> {
            val parts = data.split(":")
            if (parts.size == 4) driveAuth.authorize(reply, parts[2], parts[3] == "1")
          }
        }
      }
    }
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)

    // Android 15+ edge-to-edge에서는 창이 키보드에 맞춰 줄지 않아 입력칸이 키보드에 가려진다 (CLAUDE.md 1-C-2).
    // - 아래: 키보드(열려 있으면) 또는 하단 탐색 막대만큼 비운다
    // - 위: WebView가 화면 전체를 덮지 않으면 env(safe-area-inset-top)이 0이 되므로, 상태 표시줄 높이를 화면에 알려 준다
    //   (화면은 max(env(safe-area-inset-top), var(--native-inset-top))을 쓴다 — src/app/systemInsets.ts)
    val content = findViewById<View>(android.R.id.content)
    ViewCompat.setOnApplyWindowInsetsListener(content) { v, insets ->
      val ime = insets.getInsets(WindowInsetsCompat.Type.ime()).bottom
      val nav = insets.getInsets(WindowInsetsCompat.Type.navigationBars()).bottom
      v.setPadding(0, 0, 0, maxOf(ime, nav))
      val top = insets.getInsets(WindowInsetsCompat.Type.statusBars() or WindowInsetsCompat.Type.displayCutout()).top
      systemInsets.topCss = top / resources.displayMetrics.density
      webView?.evaluateJavascript("window.dispatchEvent(new Event('sc-insets'))", null)
      insets
    }
  }

  override fun onDestroy() {
    // 앱이 닫히면 서비스도 끈다 (작업은 프로세스와 함께 끝남)
    GenerationService.stop(applicationContext)
    super.onDestroy()
  }
}
