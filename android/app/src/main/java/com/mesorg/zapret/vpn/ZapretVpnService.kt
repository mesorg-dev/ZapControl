package com.mesorg.zapret.vpn

import android.content.Intent
import android.net.VpnService
import android.os.ParcelFileDescriptor

/**
 * Stub VpnService: establishes a TUN and reads/discards packets.
 * Real DPI-bypass engine lands in M2/M3 (userspace TCP splice).
 */
class ZapretVpnService : VpnService() {

    private var tun: ParcelFileDescriptor? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_STOP -> {
                stopVpn()
                stopSelf()
                return START_NOT_STICKY
            }
            else -> startVpn()
        }
        return START_STICKY
    }

    private fun startVpn() {
        if (tun != null) return

        val builder = Builder()
            .setSession("zapret")
            .setMtu(1500)
            .addAddress("10.0.0.2", 32)
            .addDnsServer("10.0.0.1")
            // Route only what we need to inspect later; for now route all
            // so the UI toggle visibly engages the tunnel.
            .addRoute("0.0.0.0", 0)
            .addRoute("::", 0)

        // Protect our own sockets (DNS fetch etc.) from the tunnel
        // protectSocket() is wired in M2 once we have a real engine.

        tun = builder.establish()
        // TODO(M2): spawn packet reader on tun fd
    }

    private fun stopVpn() {
        try {
            tun?.close()
        } catch (_: Exception) {
        }
        tun = null
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
        const val ACTION_STOP = "com.mesorg.zapret.vpn.STOP"
    }
}
