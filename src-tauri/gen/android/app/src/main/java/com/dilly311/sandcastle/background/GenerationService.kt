package com.dilly311.sandcastle.background

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import com.dilly311.sandcastle.MainActivity

/**
 * AI 작업(응답 생성 등)이 도는 동안만 켜 두는 포그라운드 서비스 (CLAUDE.md 1-C-2).
 *
 * 화면이 꺼지거나 앱이 뒤로 가면 Android가 프로세스를 얼려 스트리밍이 끊긴다. 이 서비스가 켜져 있으면
 * 프로세스가 살아 있다. 아무 일도 하지 않고, 알림 줄에 "응답을 만드는 중"을 보여 준다(Observability).
 * 켜고 끄기는 화면(src/app/keepAlive.ts)이 작업 이벤트를 보고 정한다 — 작업이 없으면 바로 끈다.
 */
class GenerationService : Service() {
  override fun onBind(intent: Intent?): IBinder? = null

  override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
    val label = intent?.getStringExtra(EXTRA_LABEL) ?: "응답을 만드는 중"
    val notification = buildNotification(label)
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
      startForeground(NOTIFICATION_ID, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC)
    } else {
      startForeground(NOTIFICATION_ID, notification)
    }
    // 시스템이 서비스를 죽이면 다시 살리지 않는다 (작업은 다음 시작 때 복구 목록으로)
    return START_NOT_STICKY
  }

  private fun buildNotification(label: String): Notification {
    val nm = getSystemService(NotificationManager::class.java)
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O && nm.getNotificationChannel(CHANNEL_ID) == null) {
      nm.createNotificationChannel(NotificationChannel(CHANNEL_ID, "작업 진행", NotificationManager.IMPORTANCE_LOW))
    }
    val open = PendingIntent.getActivity(
      this, 0,
      Intent(this, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP),
      PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
    )
    return NotificationCompat.Builder(this, CHANNEL_ID)
      .setContentTitle(label)
      .setSmallIcon(android.R.drawable.stat_notify_sync)
      .setOngoing(true)
      .setSilent(true)
      .setContentIntent(open)
      .build()
  }

  companion object {
    private const val CHANNEL_ID = "sandcastle-tasks"
    private const val NOTIFICATION_ID = 4101
    private const val EXTRA_LABEL = "label"

    fun start(context: Context, label: String) {
      val intent = Intent(context, GenerationService::class.java).putExtra(EXTRA_LABEL, label)
      ContextCompat.startForegroundService(context, intent)
    }

    fun stop(context: Context) {
      context.stopService(Intent(context, GenerationService::class.java))
    }
  }
}
