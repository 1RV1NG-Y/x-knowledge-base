package dev.local.xknowledgebase

import android.app.Activity
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin

@InvokeArg
class KeepAwakeArgs {
  var on: Boolean = false
}

/** Called from Rust (src-tauri/src/lib.rs) while a sync from the PC is downloading. */
@TauriPlugin
class PhonePlugin(private val activity: Activity) : Plugin(activity) {
  @Command
  fun keepAwake(invoke: Invoke) {
    val args = invoke.parseArgs(KeepAwakeArgs::class.java)
    activity.runOnUiThread {
      (activity as? MainActivity)?.keepScreenOn(args.on)
      invoke.resolve()
    }
  }
}
