package com.mesorg.zapret.vpn

import uniffi.zapret_core_ffi.dnsRespond
import java.io.FileInputStream
import java.io.FileOutputStream
import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.InetAddress

/**
 * DNS-only TUN handler:
 *  - qname in hosts map  -> answer locally (Rust core, Roblox override)
 *  - otherwise           -> forward to upstream over a protected socket
 *  - non-DNS/other       -> drop (TUN only routes our DNS server /32 anyway)
 */
class DnsServer(
    private val tunIn: FileInputStream,
    private val tunOut: FileOutputStream,
    private val dnsAddr: InetAddress,
    private val protect: (DatagramSocket) -> Boolean,
    private val upstream: String = "8.8.8.8",
    private val log: (String) -> Unit,
) {
    @Volatile
    var hostsJson: String = "{}"

    @Volatile
    var running: Boolean = false

    fun loop() {
        val buf = ByteArray(65535)
        log("[dns] старт, upstream=$upstream, hosts=${hostsJson.length} б")
        while (running) {
            val n = try {
                tunIn.read(buf)
            } catch (_: Exception) {
                break
            }
            if (n <= 0) continue
            val packet = buf.copyOf(n)
            try {
                handle(packet)
            } catch (e: Exception) {
                log("[dns] ошибка пакета: ${e.message}")
            }
        }
        log("[dns] остановлен")
    }

    private fun handle(packet: ByteArray) {
        val req = Packet.parseUdp(packet) ?: return
        if (req.dstPort != 53) return

        val local = dnsRespond(req.payload, hostsJson)
        val payload = local ?: forward(req.payload) ?: run {
            log("[dns] upstream недоступен для ${qname(req.payload)}")
            return
        }
        val reply = Packet.buildUdpReply(req, dnsAddr.address, payload)
        synchronized(tunOut) {
            tunOut.write(reply)
            tunOut.flush()
        }
    }

    private fun forward(payload: ByteArray): ByteArray? {
        var socket: DatagramSocket? = null
        return try {
            socket = DatagramSocket()
            if (!protect(socket)) {
                log("[dns] protect() failed")
                return null
            }
            socket.soTimeout = 3000
            val addr = InetAddress.getByName(upstream)
            socket.send(DatagramPacket(payload, payload.size, addr, 53))
            val rbuf = ByteArray(4096)
            val rp = DatagramPacket(rbuf, rbuf.size)
            socket.receive(rp)
            rbuf.copyOf(rp.length)
        } catch (_: Exception) {
            null
        } finally {
            socket?.close()
        }
    }

    /** Best-effort qname for logging (first question of a raw DNS message). */
    private fun qname(msg: ByteArray): String {
        if (msg.size < 13) return "?"
        val sb = StringBuilder()
        var i = 12
        var hops = 0
        while (i < msg.size && hops < 32) {
            val len = msg[i].toInt() and 0xFF
            if (len == 0) break
            if (len and 0xC0 == 0xC0) break
            if (sb.isNotEmpty()) sb.append('.')
            if (i + 1 + len > msg.size) return "?"
            sb.append(String(msg, i + 1, len, Charsets.UTF_8))
            i += 1 + len
            hops++
        }
        return sb.toString().ifEmpty { "?" }
    }
}
