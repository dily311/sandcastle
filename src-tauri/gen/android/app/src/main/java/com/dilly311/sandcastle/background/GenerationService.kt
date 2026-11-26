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
 * AI 작업(응답 생성 등)과 백그라운드 동기화·드라이브 정리가 도는 동안 켜 두는 포그라운드 서비스 (CLAUDE.md 1-C-2, 1-J-2).
 *
 * - 화면이 꺼지거나 앱이 뒤로 가도 스트리밍·동기화 전송이 멈추지 않게 프로세스를 유지한다.
 * - 동기화·데이터 삭제 중에는 상단 알림에 현재 단계와 진행률 막대(current/total · %)를 실시간으로 표시한다.
 * - AI 작업과 동기화가 모두 끝나면 즉시 서비스를 내린다.
 */
class GenerationService : Service() {
  override fun onBind(intent: Intent?): IBinder? = null

  override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
    val notification = buildNotification()
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
      startForeground(NOTIFICATION_ID, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC)
    } else {
      startForeground(NOTIFICATION_ID, notification)
    }
    return START_NOT_STICKY
  }

  private fun buildNotification(): Notification {
    val nm = getSystemService(NotificationManager::class.java)
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O && nm.getNotificationChannel(CHANNEL_ID) == null) {
      nm.createNotificationChannel(NotificationChannel(CHANNEL_ID, "백그라운드 작업", NotificationManager.IMPORTANCE_LOW))
    }
    val open = PendingIntent.getActivity(
      this, 0,
      Intent(this, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP),
      PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
    )
    val builder = NotificationCompat.Builder(this, CHANNEL_ID)
      .setSmallIcon(android.R.drawable.stat_notify_sync)
      .setOngoing(true)
      .setSilent(true)
      .setOnlyAlertOnce(true)
      .setContentIntent(open)

    if (syncOn) {
      builder.setContentTitle(syncTitle)
      if (syncText.isNotEmpty()) {
        builder.setContentText(syncText)
      }
      if (syncTotal > 0 && syncCurrent >= 0) {
        val clamped = syncCurrent.coerceIn(0, syncTotal)
        val pct = (clamped * 100) / syncTotal
        builder.setSubText("$pct%")
        builder.setProgress(syncTotal, clamped, false)
      } else if (syncIndeterminate) {
        builder.setProgress(0, 0, true)
      }
    } else {
      builder.setContentTitle(keepAliveLabel)
    }
    return builder.build()
  }

  companion object {
    private const val CHANNEL_ID = "sandcastle-tasks"
    private const val NOTIFICATION_ID = 4101

    @Volatile private var keepAliveOn: Boolean = false
    @Volatile private var keepAliveLabel: String = "응답을 만드는 중"

    @Volatile private var syncOn: Boolean = false
    @Volatile private var syncTitle: String = "동기화 중"
    @Volatile private var syncText: String = ""
    @Volatile private var syncCurrent: Int = -1
    @Volatile private var syncTotal: Int = 0
    @Volatile private var syncIndeterminate: Boolean = true

    fun start(context: Context, label: String) {
      keepAliveLabel = label.ifBlank { "응답을 만드는 중" }
      keepAliveOn = true
      ContextCompat.startForegroundService(context, Intent(context, GenerationService::class.java))
    }

    fun stop(context: Context) {
      keepAliveOn = false
      if (syncOn) {
        ContextCompat.startForegroundService(context, Intent(context, GenerationService::class.java))
      } else {
        context.stopService(Intent(context, GenerationService::class.java))
      }
    }

    fun updateSync(
      context: Context,
      title: String,
      text: String,
      current: Int,
      total: Int,
      indeterminate: Boolean,
    ) {
      syncTitle = title.ifBlank { "동기화 중" }
      syncText = text
      syncCurrent = current
      syncTotal = total
      syncIndeterminate = indeterminate
      syncOn = true
      ContextCompat.startForegroundService(context, Intent(context, GenerationService::class.java))
    }

    fun stopSync(context: Context) {
      syncOn = false
      syncText = ""
      syncCurrent = -1
      syncTotal = 0
      syncIndeterminate = false
      if (keepAliveOn) {
        ContextCompat.startForegroundService(context, Intent(context, GenerationService::class.java))
      } else {
        context.stopService(Intent(context, GenerationService::class.java))
      }
    }

    fun stopAll(context: Context) {
      keepAliveOn = false
      syncOn = false
      context.stopService(Intent(context, GenerationService::class.java))
    }
  }
}
