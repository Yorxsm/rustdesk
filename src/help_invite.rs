use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use hbb_common::log;

/// Name prefix on a login that only asks the host to connect back.
/// It is not a control session.
pub const INVITE_NAME_PREFIX: &str = "screen-invite:";
const INVITE_TTL: Duration = Duration::from_secs(15 * 60);

struct PendingInvite {
    token: String,
    expires: Instant,
}

static PENDING: Mutex<Option<PendingInvite>> = Mutex::new(None);

pub fn invite_display_name(token: &str) -> String {
    format!("{INVITE_NAME_PREFIX}{token}")
}

pub fn valid_token(token: &str) -> bool {
    uuid::Uuid::parse_str(token).is_ok()
}

pub fn valid_peer_id(id: &str) -> bool {
    let len = id.chars().count();
    (1..=64).contains(&len)
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '@' | '.' | '-' | ':'))
}

pub fn store_invite(token: &str) -> bool {
    if !valid_token(token) {
        return false;
    }
    *PENDING.lock().unwrap() = Some(PendingInvite {
        token: token.to_owned(),
        expires: Instant::now() + INVITE_TTL,
    });
    true
}

fn take_invite(token: &str) -> bool {
    let mut pending = PENDING.lock().unwrap();
    match pending.as_ref() {
        Some(invite) if invite.token == token && invite.expires > Instant::now() => {
            *pending = None;
            true
        }
        _ => false,
    }
}

#[derive(Debug)]
pub enum Incoming {
    NotInvite,
    /// Staff id to connect back to. The invite login itself is not authorized.
    Callback(String),
    Rejected,
}

pub fn incoming(my_name: &str, my_id: &str) -> Incoming {
    let Some(token) = my_name.strip_prefix(INVITE_NAME_PREFIX) else {
        return Incoming::NotInvite;
    };
    if !valid_token(token) || !valid_peer_id(my_id) || !take_invite(token) {
        return Incoming::Rejected;
    }
    Incoming::Callback(my_id.to_owned())
}

pub fn notify_host(staff_id: String) {
    #[cfg(all(feature = "flutter", any(target_os = "android", target_os = "ios")))]
    {
        push_staff_ready(&staff_id);
        return;
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        hbb_common::tokio::spawn(async move {
            match crate::ipc::connect(1000, "_cm").await {
                Ok(mut conn) => {
                    hbb_common::allow_err!(
                        conn.send(&crate::ipc::Data::HelpInviteReady(staff_id)).await
                    );
                }
                Err(err) => {
                    log::warn!("Could not tell the host window about the invite: {err}");
                }
            }
        });
    }
}

#[cfg(feature = "flutter")]
pub(crate) fn push_staff_ready(staff_id: &str) {
    let event = serde_json::json!({
        "name": "on_staff_ready",
        "id": staff_id,
    });
    let _ = crate::flutter::push_global_event(
        crate::flutter::APP_TYPE_MAIN,
        event.to_string(),
    );
}

pub fn push_result(ok: bool, message: &str) {
    #[cfg(feature = "flutter")]
    {
        let event = serde_json::json!({
            "name": "on_help_invite_result",
            "ok": if ok { "true" } else { "false" },
            "message": message,
        });
        let _ = crate::flutter::push_global_event(
            crate::flutter::APP_TYPE_MAIN,
            event.to_string(),
        );
    }
    #[cfg(not(feature = "flutter"))]
    {
        let _ = (ok, message);
    }
}

pub fn issue_token_blocking() -> String {
    let token = uuid::Uuid::new_v4().to_string();
    if send_issue_to_server(&token) {
        token
    } else {
        String::new()
    }
}

#[hbb_common::tokio::main(flavor = "current_thread")]
async fn send_issue_to_server(token: &str) -> bool {
    if crate::is_server() {
        return store_invite(token);
    }
    for _ in 0..8 {
        if let Ok(mut conn) = crate::ipc::connect(300, "").await {
            if conn
                .send(&crate::ipc::Data::HelpInviteIssue(token.to_owned()))
                .await
                .is_ok()
            {
                return true;
            }
        }
        hbb_common::sleep(0.2).await;
    }
    false
}

pub fn spawn_send(host_id: String, token: String) -> bool {
    if !valid_peer_id(&host_id) || !valid_token(&token) {
        return false;
    }
    std::thread::spawn(move || {
        if let Err(err) = send_invite_blocking(host_id, token) {
            log::warn!("Help invite was not delivered: {err}");
            push_result(false, &err.to_string());
        }
    });
    true
}

#[hbb_common::tokio::main(flavor = "current_thread")]
async fn send_invite_blocking(host_id: String, token: String) -> hbb_common::ResultType<()> {
    use crate::client::{handle_login_error, Client, Interface};
    use base::message_proto::*;
    use hbb_common::{
        bail,
        config::{LocalConfig, READ_TIMEOUT},
        futures::StreamExt,
        protobuf::Message as _,
        rendezvous_proto::ConnType,
        timeout,
    };

    let lc = std::sync::Arc::new(std::sync::RwLock::new(crate::client::LoginConfigHandler::default()));
    lc.write().unwrap().initialize(
        host_id.clone(),
        ConnType::DEFAULT_CONN,
        None,
        false,
        None,
        None,
        None,
        &format!("help-invite:{token}"),
    );
    let interface = InviteClient { lc: lc.clone() };
    let key = crate::get_key(false).await;
    let access_token = LocalConfig::get_option("access_token");
    let ((mut stream, direct, pk, _kcp, _stream_type), (feedback, rendezvous_server)) =
        Client::start(&host_id, &key, &access_token, ConnType::DEFAULT_CONN, interface.clone())
            .await?;
    let mut trust = crate::trusted_devices::client::Session::new(
        &lc.read().unwrap(),
        stream.is_secured(),
        pk.as_deref(),
    )?;
    interface.update_direct(Some(direct));
    if !stream.is_secured() && !crate::common::is_direct_ip_access(&host_id) {
        bail!("The invite could not reach the host over a secured connection");
    }
    let _keep = crate::client::hc_connection(feedback, rendezvous_server, &access_token).await;
    loop {
        let bytes = match timeout(READ_TIMEOUT, stream.next()).await {
            Err(_) => bail!("Timeout"),
            Ok(Some(Ok(bytes))) => bytes,
            Ok(Some(Err(err))) => bail!("Connection closed: {err}"),
            Ok(None) => bail!("The host closed the invite"),
        };
        let msg = Message::parse_from_bytes(&bytes)?;
        match msg.union {
            Some(message::Union::Hash(hash)) => {
                if !interface
                    .handle_hash("", hash, &mut trust, &mut stream)
                    .await
                {
                    bail!("The host did not accept the invite request");
                }
            }
            Some(message::Union::LoginResponse(response)) => {
                let response = trust.respond(response, &interface).await;
                match response.union {
                    Some(login_response::Union::Error(err))
                        if err == crate::client::LOGIN_MSG_HELP_INVITE_DELIVERED =>
                    {
                        let _ = handle_login_error(lc.clone(), &err, &interface);
                        push_result(true, "");
                        return Ok(());
                    }
                    Some(login_response::Union::Error(err)) => {
                        let _ = handle_login_error(lc.clone(), &err, &interface);
                        bail!(err);
                    }
                    Some(login_response::Union::PeerInfo(_)) => {
                        bail!("Invite was treated as a control session");
                    }
                    _ => {}
                }
            }
            Some(message::Union::TestDelay(delay)) => {
                interface.handle_test_delay(delay, &mut stream).await;
            }
            _ => {}
        }
    }
}

#[derive(Clone)]
struct InviteClient {
    lc: std::sync::Arc<std::sync::RwLock<crate::client::LoginConfigHandler>>,
}

#[async_trait]
impl crate::client::Interface for InviteClient {
    fn send(&self, _data: crate::client::Data) {}

    fn msgbox(&self, msgtype: &str, title: &str, text: &str, _link: &str) {
        log::debug!("help invite notice {msgtype} {title} {text}");
    }

    fn handle_login_error(&self, err: &str) -> bool {
        crate::client::handle_login_error(self.lc.clone(), err, self)
    }

    fn handle_peer_info(&self, _pi: base::message_proto::PeerInfo) {}

    fn set_multiple_windows_session(&self, _sessions: Vec<base::message_proto::WindowsSession>) {}

    async fn handle_hash(
        &self,
        pass: &str,
        hash: base::message_proto::Hash,
        trust: &mut crate::trusted_devices::client::Session,
        peer: &mut hbb_common::Stream,
    ) -> bool {
        crate::client::handle_hash(self.lc.clone(), pass, hash, self, trust, peer).await
    }

    async fn handle_login_from_ui(
        &self,
        _os_username: String,
        _os_password: String,
        _password: String,
        _remember: bool,
        _trust: &mut crate::trusted_devices::client::Session,
        _peer: &mut hbb_common::Stream,
    ) {
    }

    async fn handle_test_delay(
        &self,
        delay: base::message_proto::TestDelay,
        peer: &mut hbb_common::Stream,
    ) {
        crate::client::handle_test_delay(delay, peer).await;
    }

    fn get_lch(&self) -> std::sync::Arc<std::sync::RwLock<crate::client::LoginConfigHandler>> {
        self.lc.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invite_token_works_once_and_rejects_a_second_use() {
        let token = uuid::Uuid::new_v4().to_string();
        let staff_id = "123456789";
        let name = invite_display_name(&token);
        assert!(matches!(incoming(&name, staff_id), Incoming::Rejected));
        assert!(store_invite(&token));
        match incoming(&name, staff_id) {
            Incoming::Callback(id) => assert_eq!(id, staff_id),
            other => panic!("expected callback, got a different invite result: {other:?}"),
        }
        assert!(matches!(incoming(&name, staff_id), Incoming::Rejected));
        assert!(matches!(incoming("Ada", staff_id), Incoming::NotInvite));
        assert!(matches!(
            incoming(&invite_display_name("not-a-uuid"), staff_id),
            Incoming::Rejected
        ));
    }
}
