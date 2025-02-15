package com.z3phyrl

import java.net.URI
import java.io.BufferedReader
import java.io.InputStreamReader
import org.java_websocket.client.WebSocketClient
import org.java_websocket.handshake.ServerHandshake

class Client(uri: URI) : WebSocketClient(uri) {
    override fun onOpen(hsd: ServerHandshake) {
        var getprop = Runtime.getRuntime().exec("getprop ro.serialno")
        getprop.waitFor()
        var serial = BufferedReader(InputStreamReader(getprop.getInputStream())).readLine()
        send(serial)
    }
    override fun onMessage(message: String) {
        if (message == "?") {
            send("?")
        }
        println(message);
    }
    override fun onClose(code: Int, reason: String, remote: Boolean) {
        println(code);
        println(reason);
        Runtime.getRuntime().exec("pm uninstall com.z3phyrl.Deskr").waitFor()
    }
    override fun onError(ex: Exception) {
    }
}

fun main() {
    var client = Client(URI("ws://localhost:6969"))
    client.connect()
    Runtime.getRuntime().exec("rm /data/local/tmp/cleaner.jar")
}
