#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        // WebKitGTK's DMA-BUF renderer can terminate on NVIDIA Wayland sessions
        // with GDK protocol error 71 before the first window is drawn.
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    x_knowledge_base_lib::run();
}
