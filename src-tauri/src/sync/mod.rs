//! One-way sync from the PC to the phone over the local network.
//!
//! The PC (desktop app) serves; the phone pairs once by scanning the PC's code, then pulls a
//! snapshot of the PC's database and the media files it references whenever the PC is
//! reachable. Nothing flows back: the phone is a read-only mirror.

pub mod node;
pub mod proto;
pub mod snapshot;

#[cfg(test)]
mod tests;

pub use node::{ClientHooks, Invite, Node, NodeConfig, Role, Status, DEFAULT_PORT};

/// The name other devices show for this one.
#[cfg(target_os = "android")]
pub fn device_name() -> String {
    fn prop(name: &str) -> Option<String> {
        let name = std::ffi::CString::new(name).ok()?;
        let mut buf = [0 as libc::c_char; 92]; // PROP_VALUE_MAX
        // SAFETY: `buf` is PROP_VALUE_MAX bytes, the documented maximum for this call.
        let len = unsafe { libc::__system_property_get(name.as_ptr(), buf.as_mut_ptr()) };
        if len <= 0 {
            return None;
        }
        let bytes: Vec<u8> = buf[..len as usize].iter().map(|&c| c as u8).collect();
        String::from_utf8(bytes).ok().filter(|s| !s.trim().is_empty())
    }
    prop("ro.product.marketname")
        .or_else(|| prop("ro.product.model"))
        .unwrap_or_else(|| "Android phone".into())
}

#[cfg(not(target_os = "android"))]
pub fn device_name() -> String {
    if let Ok(name) = std::env::var("KB_DEVICE_NAME") {
        return name;
    }
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .or_else(|| std::env::var("COMPUTERNAME").ok())
        .map(|h| h.trim().to_owned())
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "PC".into())
}
