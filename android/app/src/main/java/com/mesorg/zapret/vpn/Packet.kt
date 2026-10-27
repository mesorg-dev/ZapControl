package com.mesorg.zapret.vpn

/**
 * Minimal IPv4/UDP parsing + packet building for the DNS-only TUN path.
 */
object Packet {

    data class UdpDns(
        val srcAddr: ByteArray,
        val dstAddr: ByteArray,
        val srcPort: Int,
        val dstPort: Int,
        val payload: ByteArray,
    ) {
        override fun equals(other: Any?): Boolean = this === other
        override fun hashCode(): Int = System.identityHashCode(this)
    }

    /**
     * Parse an IPv4+UDP datagram. Returns null if not IPv4/UDP or truncated.
     * Only strict, non-fragmented first fragments are accepted (DNS path).
     */
    fun parseUdp(packet: ByteArray): UdpDns? {
        if (packet.size < 28) return null
        val verIhl = packet[0].toInt() and 0xFF
        if (verIhl shr 4 != 4) return null
        val ihl = (verIhl and 0x0F) * 4
        if (ihl < 20 || packet.size < ihl + 8) return null
        // protocol must be UDP(17)
        if (packet[9].toInt() and 0xFF != 17) return null
        // fragmentation: MF or non-zero offset => bail (we don't reassemble)
        val frag = ((packet[6].toInt() and 0xFF) shl 8) or (packet[7].toInt() and 0xFF)
        if (frag and 0x3FFF != 0) return null

        val srcAddr = packet.copyOfRange(12, 16)
        val dstAddr = packet.copyOfRange(16, 20)
        val srcPort = ((packet[ihl].toInt() and 0xFF) shl 8) or (packet[ihl + 1].toInt() and 0xFF)
        val dstPort = ((packet[ihl + 2].toInt() and 0xFF) shl 8) or (packet[ihl + 3].toInt() and 0xFF)
        val udpLen = ((packet[ihl + 4].toInt() and 0xFF) shl 8) or (packet[ihl + 5].toInt() and 0xFF)
        if (udpLen < 8 || ihl + udpLen > packet.size) return null
        val payload = packet.copyOfRange(ihl + 8, ihl + udpLen)
        return UdpDns(srcAddr, dstAddr, srcPort, dstPort, payload)
    }

    /**
     * Build an IPv4/UDP reply: src<->dst swapped vs [req], payload = [response].
     * Correct IP + UDP checksums (IPv4).
     */
    fun buildUdpReply(req: UdpDns, srcAddr: ByteArray, payload: ByteArray): ByteArray {
        val udpLen = 8 + payload.size
        val total = 20 + udpLen
        val p = ByteArray(total)

        p[0] = 0x45
        p[1] = 0x00
        p[2] = (total shr 8).toByte()
        p[3] = (total and 0xFF).toByte()
        p[4] = 0x00 // id
        p[5] = 0x00
        p[6] = 0x40 // DF
        p[7] = 0x00
        p[8] = 64 // TTL
        p[9] = 17 // UDP
        // checksum @10..11 later
        srcAddr.copyInto(p, 12)
        req.dstAddr.copyInto(p, 16)

        val ipCk = checksum(p, 0, 20)
        p[10] = (ipCk shr 8).toByte()
        p[11] = (ipCk and 0xFF).toByte()

        // UDP header
        p[20] = (req.dstPort shr 8).toByte()
        p[21] = (req.dstPort and 0xFF).toByte()
        p[22] = (req.srcPort shr 8).toByte()
        p[23] = (req.srcPort and 0xFF).toByte()
        p[24] = (udpLen shr 8).toByte()
        p[25] = (udpLen and 0xFF).toByte()
        // udp checksum @26..27 later
        payload.copyInto(p, 28)

        // UDP checksum with IPv4 pseudo-header
        var sum = 0
        // pseudo: src(4) dst(4) zero(1) proto(1) udpLen(2)
        sum += wordsSum(p, 12, 8)
        sum += 17
        sum += udpLen
        sum += wordsSum(p, 20, udpLen)
        var ck = fold(sum)
        if (ck == 0) ck = 0xFFFF
        p[26] = (ck shr 8).toByte()
        p[27] = (ck and 0xFF).toByte()
        return p
    }

    private fun wordsSum(buf: ByteArray, off: Int, len: Int): Int {
        var sum = 0
        var i = off
        val end = off + len
        while (i + 1 < end) {
            val w = ((buf[i].toInt() and 0xFF) shl 8) or (buf[i + 1].toInt() and 0xFF)
            sum += w
            i += 2
        }
        if (i < end) {
            sum += (buf[i].toInt() and 0xFF) shl 8
        }
        return sum
    }

    private fun fold(sum: Int): Int {
        var s = sum
        while (s ushr 16 != 0) {
            s = (s and 0xFFFF) + (s ushr 16)
        }
        return s.inv() and 0xFFFF
    }

    fun checksum(buf: ByteArray, off: Int, len: Int): Int = fold(wordsSum(buf, off, len))
}
