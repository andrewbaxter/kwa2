use htwrap::htreq::Limits;

pub const KEY_VAPID_PRIVATE_B64: &str = "vapid_private_b64";
pub const KEY_VAPID_PUBLIC_B64: &str = "vapid_public_b64";
pub const PUSH_LIMITS: Limits = Limits {
    read_body_size: 64 * 1024,
    resolve_time: std::time::Duration::from_secs(10),
    connect_time: std::time::Duration::from_secs(10),
    read_header_time: std::time::Duration::from_secs(30),
    read_body_time: std::time::Duration::from_secs(30),
    read_idle_time: std::time::Duration::from_secs(30),
};

pub struct VapidKeypair {
    pub private_b64url: String,
    pub public_b64url: String,
}

pub enum PushSendError {
    Gone,
    Other(loga::Error),
}
