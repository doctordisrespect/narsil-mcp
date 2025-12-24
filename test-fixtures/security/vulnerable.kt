package com.example.security

// INTENTIONAL VULNERABILITIES - DO NOT USE IN PRODUCTION
// Test fixture for security scanner validation

import android.app.PendingIntent
import android.content.Intent
import android.webkit.WebView
import java.security.SecureRandom
import java.sql.PreparedStatement
import java.util.Random

class VulnerableKotlin {

    // KOTLIN-001: SQL Injection
    fun searchUser(userId: String) {
        val query = "SELECT * FROM users WHERE id = $userId" // BAD: String template
        val query2 = "SELECT * FROM users WHERE id = " + userId // BAD: Concat
        db.rawQuery("SELECT * FROM users WHERE name = $name", null) // BAD
    }

    // KOTLIN-002: WebView JavaScript Enabled
    fun setupWebView(webView: WebView) {
        webView.settings.setJavaScriptEnabled(true) // BAD: JS enabled
        webView.addJavascriptInterface(this, "Android") // BAD: JS interface
    }

    // KOTLIN-003: Insecure Intent Handling
    fun handleIntent(intent: Intent) {
        val userId = intent.getStringExtra("user_id") // BAD: Unvalidated extra
        val uri = intent.data?.toString() // BAD: Unvalidated data

        // BAD: FLAG_MUTABLE for PendingIntent
        val pendingIntent = PendingIntent.getActivity(
            context, 0, intent, PendingIntent.FLAG_MUTABLE
        )
    }

    // KOTLIN-004: Hardcoded Secrets
    val apiKey = "sk_live_1234567890abcdefghij" // BAD: Hardcoded key
    val secretToken = "mysecrettoken123456789" // BAD: Hardcoded secret
    val dbPassword = "password123" // BAD: Hardcoded password

    // KOTLIN-005: Insecure Random
    fun generateToken(): String {
        val random = Random() // BAD: Insecure Random
        return random.nextInt().toString()
    }

    // SAFE PATTERNS
    fun safeQuery(userId: String) {
        val pstmt = connection.prepareStatement("SELECT * FROM users WHERE id = ?")
        pstmt.setString(1, userId) // GOOD: Parameterized
    }

    fun safeRandom(): String {
        val random = SecureRandom() // GOOD: SecureRandom
        return random.nextInt().toString()
    }

    fun safePendingIntent(): PendingIntent {
        return PendingIntent.getActivity(
            context, 0, intent, PendingIntent.FLAG_IMMUTABLE // GOOD: Immutable
        )
    }
}
