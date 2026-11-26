package dev.local.xknowledgebase

import android.content.res.Configuration
import android.graphics.Color
import android.net.wifi.WifiManager
import android.os.Bundle
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
  private var multicast: WifiManager.MulticastLock? = null

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    // Keep the page clear of the status bar, navigation bar, camera cutout and keyboard.
    val content = findViewById<android.view.View>(android.R.id.content)
    ViewCompat.setOnApplyWindowInsetsListener(content) { view, insets ->
      val bars = insets.getInsets(
        WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout() or WindowInsetsCompat.Type.ime()
      )
      view.setPadding(bars.left, bars.top, bars.right, bars.bottom)
      WindowInsetsCompat.CONSUMED
    }
    applyBarColors(resources.configuration)
  }

  // Finding the PC on the Wi-Fi uses mDNS, which Android filters out unless an app holds a
  // multicast lock. Held only while the app is on screen.
  override fun onStart() {
    super.onStart()
    try {
      val wifi = applicationContext.getSystemService(WIFI_SERVICE) as WifiManager
      multicast = wifi.createMulticastLock("x-knowledge-base:mdns").apply {
        setReferenceCounted(false)
        acquire()
      }
    } catch (error: Exception) {
      android.util.Log.w("PhoneSync", "No multicast lock; the PC is found by its last address", error)
    }
  }

  override fun onStop() {
    multicast?.let { if (it.isHeld) it.release() }
    multicast = null
    super.onStop()
  }

  override fun onConfigurationChanged(newConfig: Configuration) {
    super.onConfigurationChanged(newConfig)
    applyBarColors(newConfig)
  }

  /** The strips behind the system bars match the page: black in dark mode, white in light. */
  private fun applyBarColors(config: Configuration) {
    val dark = (config.uiMode and Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES
    val content = findViewById<android.view.View>(android.R.id.content)
    content.setBackgroundColor(if (dark) Color.BLACK else Color.WHITE)
    WindowCompat.getInsetsController(window, content).apply {
      isAppearanceLightStatusBars = !dark
      isAppearanceLightNavigationBars = !dark
    }
  }

  /** Keeps the screen on while a sync downloads (see PhonePlugin). */
  fun keepScreenOn(on: Boolean) {
    if (on) window.addFlags(android.view.WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
    else window.clearFlags(android.view.WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
  }
}
