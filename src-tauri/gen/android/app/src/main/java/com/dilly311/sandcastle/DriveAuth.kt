package com.dilly311.sandcastle

import android.app.Activity
import android.content.Context
import android.net.ConnectivityManager
import android.webkit.JavascriptInterface
import androidx.activity.ComponentActivity
import androidx.activity.result.ActivityResultLauncher
import androidx.activity.result.IntentSenderRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.webkit.JavaScriptReplyProxy
import com.google.android.gms.auth.GoogleAuthUtil
import com.google.android.gms.auth.api.identity.AuthorizationRequest
import com.google.android.gms.auth.api.identity.Identity
import com.google.android.gms.common.api.ApiException
import com.google.android.gms.common.api.Scope
import org.json.JSONObject

/**
 * 구글 드라이브 권한 (CLAUDE.md 2-D Android, ADR-011, 1-J-3).
 *
 * Play 서비스에 앱 전용 폴더(drive.appdata) 권한만 묻는다. 이미 허락했으면 화면 없이 access token을 돌려주고,
 * 처음이거나 권한이 취소되었으면(interactive일 때) 구글의 동의 화면을 띄운다.
 * 연결을 끊거나 서버에서 401이 돌아오면 clearToken으로 Play 서비스 로컬 캐시를 비워 옛 토큰이 재사용되지 않게 한다.
 */
class DriveAuth(private val activity: ComponentActivity) {
  private var pending: Pair<JavaScriptReplyProxy, String>? = null
  @Volatile private var lastToken: String? = null

  private val consent: ActivityResultLauncher<IntentSenderRequest> =
    activity.registerForActivityResult(ActivityResultContracts.StartIntentSenderForResult()) { result ->
      val (proxy, id) = pending ?: return@registerForActivityResult
      pending = null
      if (result.resultCode != Activity.RESULT_OK) {
        reply(proxy, id, error = "cancelled")
        return@registerForActivityResult
      }
      try {
        val r = Identity.getAuthorizationClient(activity).getAuthorizationResultFromIntent(result.data)
        lastToken = r.accessToken
        reply(proxy, id, token = r.accessToken)
      } catch (e: ApiException) {
        reply(proxy, id, error = "api:${e.statusCode}")
      }
    }

  fun authorize(proxy: JavaScriptReplyProxy, id: String, interactive: Boolean) {
    val request = AuthorizationRequest.builder()
      .setRequestedScopes(listOf(Scope(DRIVE_APPDATA)))
      .build()
    Identity.getAuthorizationClient(activity).authorize(request)
      .addOnSuccessListener { r ->
        if (r.hasResolution()) {
          val intent = r.pendingIntent
          if (!interactive || intent == null || pending != null) {
            reply(proxy, id, error = "needs_consent")
          } else {
            pending = proxy to id
            consent.launch(IntentSenderRequest.Builder(intent.intentSender).build())
          }
        } else {
          lastToken = r.accessToken
          reply(proxy, id, token = r.accessToken)
        }
      }
      .addOnFailureListener { e ->
        reply(proxy, id, error = if (e is ApiException) "api:${e.statusCode}" else (e.message ?: "failed"))
      }
  }

  /** Play 서비스가 캐시해 둔 취소·만료 access token을 비운다 */
  fun clearToken(proxy: JavaScriptReplyProxy, id: String, tokenArg: String?) {
    val tok = tokenArg?.takeIf { it.isNotBlank() } ?: lastToken
    lastToken = null
    Thread {
      if (tok != null) {
        try {
          GoogleAuthUtil.clearToken(activity.applicationContext, tok)
        } catch (_: Exception) {
        }
      }
      reply(proxy, id, token = "cleared")
    }.start()
  }

  private fun reply(proxy: JavaScriptReplyProxy, id: String, token: String? = null, error: String? = null) {
    val o = JSONObject().put("kind", "drive").put("id", id)
    if (token != null) o.put("token", token) else o.put("error", error ?: "no_token")
    activity.runOnUiThread { proxy.postMessage(o.toString()) }
  }

  companion object {
    const val DRIVE_APPDATA = "https://www.googleapis.com/auth/drive.appdata"
  }
}

/** 지금 연결이 데이터 요금이 들지 않는가 (Wi-Fi 등). 동기화의 "Wi-Fi에서만"이 읽는다. 숫자·참거짓만 돌려준다 */
class NetworkState(private val context: Context) {
  @JavascriptInterface
  fun unmetered(): Boolean {
    val cm = context.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
    return cm.activeNetwork != null && !cm.isActiveNetworkMetered
  }
}
