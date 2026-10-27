package com.mesorg.zapret.vpn

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Intent
import android.net.VpnService
import android.os.Build
import android.os.ParcelFileDescriptor
import com.mesorg.zapret.MainActivity
import com.mesorg.zapret.R
import uniffi.zapret_core_ffi.hostsBlockToJson
import java.io.FileInputStream
import java.io.FileOutputStream
import java.net.InetAddress

/**
 * DNS-only VPN engine:
 *  - routes ONLY the virtual DNS server (/32), everything else bypasses
 *  - answers overridden hosts (Roblox JSON) locally via zapret-core
 *  - forwards the rest upstream (protected socket)
 *
 * DPI/TCP bypass (frag-mode emulation) is M2 TODO: needs userspace TCP splice.
 */
class ZapretVpnService : VpnService() {

    private var tun: ParcelFileDescriptor? = null
    private var dnsThread: Thread? = null
    private var dnsServer: DnsServer? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_STOP -> {
                stopVpn()
                stopSelf()
                return START_NOT_STICKY
            }
            ACTION_RELOAD_HOSTS -> {
                loadHosts()
                return START_STICKY
            }
            else -> startVpn()
        }
        return START_STICKY
    }

    private fun startVpn() {
        if (tun != null) return

        createChannelAndNotification()

        val dnsIp = DNS_SERVER // 10.0.0.3
        val builder = Builder()
            .setSession("zapret")
            .setMtu(1500)
            .addAddress("10.0.0.2", 32)
            .addDnsServer(dnsIp)
            // DNS-only: route just our resolver; all other traffic unaffected
            .addRoute(dnsIp, 32)

        val fd = try {
            builder.establish()
        } catch (e: Exception) {
            VpnState.lastError = "TUN: ${e.message}"
            VpnState.running = false
            stopSelf()
            return
        }
        if (fd == null) {
            VpnState.lastError = "TUN не установлен (отказано?)"
            VpnState.running = false
            stopSelf()
            return
        }
        tun = fd

        val dns = DnsServer(
            tunIn = FileInputStream(fd.fileDescriptor),
            tunOut = FileOutputStream(fd.fileDescriptor),
            dnsAddr = InetAddress.getByName(dnsIp),
            protect = { s -> protect(s) },
            log = { line ->
                android.util.Log.i(TAG, line)
            },
        )
        val hosts = readHostsJson()
        dns.hostsJson = hosts
        VpnState.robloxApplied = hosts != "{}"
        dnsServer = dns
        dns.running = true
        dnsThread = Thread({ dns.loop() }, "zapret-dns").apply {
            isDaemon = true
            start()
        }

        VpnState.running = true
        VpnState.lastError = null
        android.util.Log.i(TAG, "VPN started (dns-only, hosts=${dns.hostsJson.length}b)")
    }

    private fun readHostsJson(): String {
        return try {
            val f = java.io.File(filesDir, "roblox-hosts.json")
            if (f.exists()) {
                val block = f.readText()
                hostsBlockToJson(block)
            } else {
                "{}"
            }
        } catch (_: Exception) {
            "{}"
        }
    }

    private fun loadHosts() {
        val json = readHostsJson()
        dnsServer?.hostsJson = json
        VpnState.robloxApplied = json != "{}"
    }

    private fun stopVpn() {
        dnsServer?.running = false
        dnsThread?.interrupt()
        dnsThread = null
        dnsServer = null
        try {
            tun?.close()
        } catch (_: Exception) {
        }
        tun = null
        VpnState.running = false
        android.util.Log.i(TAG, "VPN stopped")
    }

    private fun createChannelAndNotification() {
        val nm = getSystemService(NotificationManager::class.java)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val ch = NotificationChannel(
                CHANNEL_ID,
                getString(R.string.vpn_channel_name),
                NotificationManager.IMPORTANCE_LOW,
            )
            nm.createNotificationChannel(ch)
        }
        val open = PendingIntent.getActivity(
            this,
            0,
            Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_IMMUTABLE,
        )
        val stopIntent = PendingIntent.getService(
            this,
            1,
            Intent(this, ZapretVpnService::class.java).setAction(ACTION_STOP),
            PendingIntent.FLAG_IMMUTABLE,
        )
        val notif: Notification = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            Notification.Builder(this, CHANNEL_ID)
        } else {
            @Suppress("DEPRECATION")
            Notification.Builder(this)
        }
            .setContentTitle(getString(R.string.vpn_notification_title))
            .setContentText(getString(R.string.vpn_notification_text))
            .setSmallIcon(android.R.drawable.ic_lock_idle_lock)
            .setContentIntent(open)
            .addAction(
                Notification.Action.Builder(
                    android.R.drawable.ic_menu_close_clear_cancel,
                    "Стоп",
                    stopIntent,
                ).build(),
            )
            .setOngoing(true)
            .build()

        // FGS with specialUse type on API 34+ (manifest declares it)
        if (Build.VERSION.SDK_INT >= 34) {
            startForeground(
                NOTIF_ID,
                notif,
                android.content.pm.ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE,
            )
        } else {
            startForeground(NOTIF_ID, notif)
        }
    }

    override fun onDestroy() {
        stopVpn()
        super.onDestroy()
    }

    override fun onRevoke() {
        stopVpn()
        stopSelf()
    }

    companion object {
        private const val TAG = "ZapretVpn"
        private const val CHANNEL_ID = "vpn"
        private const val NOTIF_ID = 1
        const val DNS_SERVER = "10.0.0.3"
        const val ACTION_STOP = "com.mesorg.zapret.vpn.STOP"
        const val ACTION_RELOAD_HOSTS = "com.mesorg.zapret.vpn.RELOAD_HOSTS"

        fun start(context: android.content.Context) {
            val i = Intent(context, ZapretVpnService::class.java)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                context.startForegroundService(i)
            } else {
                context.startService(i)
            }
        }

        fun stop(context: android.content.Context) {
            val i = Intent(context, ZapretVpnService::class.java).setAction(ACTION_STOP)
            context.startService(i)
        }

        fun reloadHosts(context: android.content.Context) {
            val i = Intent(context, ZapretVpnService::class.java).setAction(ACTION_RELOAD_HOSTS)
            context.startService(i)
        }

        /** Persist fetched hosts block + refresh live server (if any). */
        fun persistHosts(context: android.content.Context, block: String) {
            java.io.File(context.filesDir, "roblox-hosts.json").writeText(block)
            VpnState.robloxApplied = true
            reloadHosts(context)
        }
    }
}
