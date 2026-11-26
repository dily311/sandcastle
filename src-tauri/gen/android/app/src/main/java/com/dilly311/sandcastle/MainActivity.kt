package com.dilly311.sandcastle

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.view.View
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.core.content.ContextCompat
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.webkit.WebViewCompat
import androidx.webkit.WebViewFeature
import com.dilly311.sandcastle.background.GenerationService
import org.json.JSONObject

/** 화면이 읽는 상태 표시줄·하단 탐색 막대 높이 (CSS px). 숫자만 돌려준다 */
class SystemInsets {
  @Volatile var topCss: Float = 0f
  @Volatile var bottomCss: Float = 0f

  @JavascriptInterface
  fun top(): Float = topCss

  /** 화면이 직접 비워야 할 아래 높이 (하단 탐색 막대). 키보드가 열려 있으면 앱이 이미 비웠으므로 0 */
  @JavascriptInterface
  fun bottom(): Float = bottomCss
}

class MainActivity : TauriActivity() {
  private val systemInsets = SystemInsets()
  private var webView: WebView? = null
  // 구글 드라이브 권한 (2-D). 동의 화면 결과를 받으려면 생성될 때 등록해야 해서 필드로
  private val driveAuth = DriveAuth(this)
  // 모바일 시작 시 알림 권한 요청 (1-J-5)
  private val notifyPermission = registerForActivityResult(ActivityResultContracts.RequestPermission()) { _ -> }

  override fun onWebViewCreate(webView: WebView) {
    this.webView = webView
    webView.addJavascriptInterface(systemInsets, "SandcastleInsets")
    // 참거짓 하나만 돌려준다 (데이터 요금이 드는 연결인가)
    webView.addJavascriptInterface(NetworkState(applicationContext), "SandcastleNet")

    // 백그라운드 서비스·동기화 진행률·드라이브 권한 (앱 자기 주소에서만 — htmlview iframe에는 노출되지 않음)
    if (WebViewFeature.isFeatureSupported(WebViewFeature.WEB_MESSAGE_LISTENER)) {
      WebViewCompat.addWebMessageListener(webView, "SandcastleNative", setOf("http://tauri.localhost")) { _, message, _, isMainFrame, reply ->
        if (!isMainFrame) return@addWebMessageListener
        val data = message.data ?: return@addWebMessageListener
        when {
          data == "keepalive:start" -> GenerationService.start(applicationContext, "응답을 만드는 중")
          data.startsWith("keepalive:start:") -> {
            GenerationService.start(applicationContext, data.removePrefix("keepalive:start:"))
          }
          data == "keepalive:stop" -> GenerationService.stop(applicationContext)
          data.startsWith("sync:progress:") -> {
            try {
              val o = JSONObject(data.removePrefix("sync:progress:"))
              GenerationService.updateSync(
                applicationContext,
                o.optString("title", "동기화 중"),
                o.optString("text", ""),
                o.optInt("current", -1),
                o.optInt("total", 0),
                o.optBoolean("indeterminate", false),
              )
            } catch (_: Exception) {
            }
          }
          data == "sync:stop" -> GenerationService.stopSync(applicationContext)
          // 구글 드라이브 권한: drive:authorize:<id>:<1이면 동의 화면을 띄워도 됨> (src/app/androidDrive.ts)
          data.startsWith("drive:authorize:") -> {
            val parts = data.split(":")
            if (parts.size == 4) driveAuth.authorize(reply, parts[2], parts[3] == "1")
          }
          // 취소·만료된 Play 서비스 캐시 토큰 비우기: drive:clear:<id>:<token?>
          data.startsWith("drive:clear:") -> {
            val parts = data.split(":", limit = 4)
            if (parts.size >= 3) driveAuth.clearToken(reply, parts[2], parts.getOrNull(3))
          }
        }
      }
    }
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)

    // Android 13+(API 33+) 시작 시 알림 권한을 먼저 받아 포그라운드 동기화 진행률과 완료 알림이 바로 보이게 한다 (1-J-5)
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
      if (ContextCompat.checkSelfPermission(this, Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
        notifyPermission.launch(Manifest.permission.POST_NOTIFICATIONS)
      }
    }

    // Android 15+ edge-to-edge에서는 창이 키보드에 맞춰 줄지 않아 입력칸이 키보드에 가려진다 (CLAUDE.md 1-C-2).
    val content = findViewById<View>(android.R.id.content)
    ViewCompat.setOnApplyWindowInsetsListener(content) { v, insets ->
      val ime = insets.getInsets(WindowInsetsCompat.Type.ime()).bottom
      val nav = insets.getInsets(WindowInsetsCompat.Type.navigationBars()).bottom
      val keyboard = ime > nav
      v.setPadding(0, 0, 0, if (keyboard) ime else 0)
      systemInsets.bottomCss = if (keyboard) 0f else nav / resources.displayMetrics.density
      val top = insets.getInsets(WindowInsetsCompat.Type.statusBars() or WindowInsetsCompat.Type.displayCutout()).top
      systemInsets.topCss = top / resources.displayMetrics.density
      webView?.evaluateJavascript("window.dispatchEvent(new Event('sc-insets'))", null)
      insets
    }
  }

  override fun onDestroy() {
    // 앱이 닫히면 서비스도 끈다 (작업은 프로세스와 함께 끝남)
    GenerationService.stopAll(applicationContext)
    super.onDestroy()
  }
}
