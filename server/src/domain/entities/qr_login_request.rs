use chrono::{DateTime, Utc};
use uuid::Uuid;

// QR code sign-in ("scan to sign in", like a game launcher's mobile app).
//
// A browser that wants to sign in shows a QR code. Another device scans it: either a
// phone that opens the hosted approval page and signs in there, or an app the user is
// already signed in to (it calls `/me/qr-login/*` with its token). That device sees
// which browser is asking (app, device, approximate location) and approves or denies.
// The browser, polling with its own login-session code, then completes the sign-in.
//
// Two secrets, both stored only as SHA-256 hashes:
//   * the QR token, inside the code itself. Scanning it is not enough to approve: it
//     only lets the FIRST device that presents it claim the request;
//   * the claim code, returned to that first device only. Approving or denying needs
//     it, so a second device that photographs the same code (a shoulder surfer) can
//     neither claim nor approve.
//
// The browser's half is its login session (`auth_session_id`): only the browser that
// asked holds that session's code, so an approval can only ever sign in that browser.
//
// The status only moves forward, each step one conditional statement in the
// repository (two devices racing to scan, or a deny racing an approve, cannot both
// win):
//   pending -> scanned -> approved -> completed
//                      -> denied
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QrLoginStatus {
    // Shown, not scanned yet.
    Pending,
    // A device claimed it; waiting for that device's decision.
    Scanned,
    // The device approved; the browser has not collected the sign-in yet.
    Approved,
    Denied,
    // The browser collected the approval: its session is signed in.
    Completed,
}

impl QrLoginStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            QrLoginStatus::Pending => "pending",
            QrLoginStatus::Scanned => "scanned",
            QrLoginStatus::Approved => "approved",
            QrLoginStatus::Denied => "denied",
            QrLoginStatus::Completed => "completed",
        }
    }

    // Fails closed: an unknown value reads as `Denied`, which nothing can move on.
    pub fn from_db(value: &str) -> Self {
        match value {
            "pending" => QrLoginStatus::Pending,
            "scanned" => QrLoginStatus::Scanned,
            "approved" => QrLoginStatus::Approved,
            "completed" => QrLoginStatus::Completed,
            _ => QrLoginStatus::Denied,
        }
    }
}

// Which kind of device approved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QrApprover {
    // The hosted approval page, after the user signed in on that device.
    HostedPage,
    // An app the user was already signed in to (an access token with `account`),
    // confirmed with the account's password.
    SignedInApp,
    // A signed-in app whose registered device key signed the approval.
    TrustedDevice,
}

impl QrApprover {
    pub fn as_str(&self) -> &'static str {
        match self {
            QrApprover::HostedPage => "hosted_page",
            QrApprover::SignedInApp => "signed_in_app",
            QrApprover::TrustedDevice => "trusted_device",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "hosted_page" => Some(QrApprover::HostedPage),
            "signed_in_app" => Some(QrApprover::SignedInApp),
            "trusted_device" => Some(QrApprover::TrustedDevice),
            _ => None,
        }
    }
}

// What the approving device is shown about the browser that asked.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct QrRequester {
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    // "City, Region, CC" when GeoIP is configured.
    pub location: Option<String>,
}

#[derive(Debug, Clone)]
pub struct QrLoginRequest {
    id: Uuid,
    tenant_id: Uuid,
    // The internal oauth_clients.id the browser is signing in to.
    client_id: Uuid,
    auth_session_id: Uuid,
    token_hash: String,
    status: QrLoginStatus,
    claim_hash: Option<String>,
    scanned_by_user_id: Option<Uuid>,
    approved_user_id: Option<Uuid>,
    approver_mfa_verified: bool,
    approved_via: Option<QrApprover>,
    requester: QrRequester,
    scanned_at: Option<DateTime<Utc>>,
    decided_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

impl QrLoginRequest {
    pub fn new(
        tenant_id: Uuid,
        client_id: Uuid,
        auth_session_id: Uuid,
        token_hash: String,
        requester: QrRequester,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> Self {
        QrLoginRequest {
            id: Uuid::now_v7(),
            tenant_id,
            client_id,
            auth_session_id,
            token_hash,
            status: QrLoginStatus::Pending,
            claim_hash: None,
            scanned_by_user_id: None,
            approved_user_id: None,
            approver_mfa_verified: false,
            approved_via: None,
            requester,
            scanned_at: None,
            decided_at: None,
            completed_at: None,
            expires_at,
            created_at: now,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        tenant_id: Uuid,
        client_id: Uuid,
        auth_session_id: Uuid,
        token_hash: String,
        status: QrLoginStatus,
        claim_hash: Option<String>,
        scanned_by_user_id: Option<Uuid>,
        approved_user_id: Option<Uuid>,
        approver_mfa_verified: bool,
        approved_via: Option<QrApprover>,
        requester: QrRequester,
        scanned_at: Option<DateTime<Utc>>,
        decided_at: Option<DateTime<Utc>>,
        completed_at: Option<DateTime<Utc>>,
        expires_at: DateTime<Utc>,
        created_at: DateTime<Utc>,
    ) -> Self {
        QrLoginRequest {
            id,
            tenant_id,
            client_id,
            auth_session_id,
            token_hash,
            status,
            claim_hash,
            scanned_by_user_id,
            approved_user_id,
            approver_mfa_verified,
            approved_via,
            requester,
            scanned_at,
            decided_at,
            completed_at,
            expires_at,
            created_at,
        }
    }

    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }

    // A device may still claim it.
    pub fn is_claimable(&self, now: DateTime<Utc>) -> bool {
        self.status == QrLoginStatus::Pending && !self.is_expired(now)
    }

    // The claiming device may still decide. `user_id` is the deciding user, when
    // known: a code claimed by a signed-in app is decided by that app's user only.
    pub fn is_decidable_by(
        &self,
        claim_hash: &str,
        user_id: Option<Uuid>,
        now: DateTime<Utc>,
    ) -> bool {
        self.status == QrLoginStatus::Scanned
            && !self.is_expired(now)
            && self.claim_hash.as_deref() == Some(claim_hash)
            && match (self.scanned_by_user_id, user_id) {
                (Some(scanner), Some(user)) => scanner == user,
                (Some(_), None) => false,
                (None, _) => true,
            }
    }

    // In-memory mirrors of the repository's conditional statements, for the fakes.
    pub fn claim(
        &mut self,
        claim_hash: String,
        scanned_by_user_id: Option<Uuid>,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) {
        self.status = QrLoginStatus::Scanned;
        self.claim_hash = Some(claim_hash);
        self.scanned_by_user_id = scanned_by_user_id;
        self.scanned_at = Some(now);
        self.expires_at = expires_at;
    }

    pub fn approve(
        &mut self,
        user_id: Uuid,
        mfa_verified: bool,
        via: QrApprover,
        now: DateTime<Utc>,
    ) {
        self.status = QrLoginStatus::Approved;
        self.approved_user_id = Some(user_id);
        self.approver_mfa_verified = mfa_verified;
        self.approved_via = Some(via);
        self.decided_at = Some(now);
    }

    pub fn deny(&mut self, now: DateTime<Utc>) {
        self.status = QrLoginStatus::Denied;
        self.decided_at = Some(now);
    }

    pub fn complete(&mut self, now: DateTime<Utc>) {
        self.status = QrLoginStatus::Completed;
        self.completed_at = Some(now);
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }
    pub fn client_id(&self) -> Uuid {
        self.client_id
    }
    pub fn auth_session_id(&self) -> Uuid {
        self.auth_session_id
    }
    pub fn token_hash(&self) -> &str {
        &self.token_hash
    }
    pub fn status(&self) -> QrLoginStatus {
        self.status
    }
    pub fn claim_hash(&self) -> Option<&str> {
        self.claim_hash.as_deref()
    }
    pub fn scanned_by_user_id(&self) -> Option<Uuid> {
        self.scanned_by_user_id
    }
    pub fn approved_user_id(&self) -> Option<Uuid> {
        self.approved_user_id
    }
    pub fn approver_mfa_verified(&self) -> bool {
        self.approver_mfa_verified
    }
    pub fn approved_via(&self) -> Option<QrApprover> {
        self.approved_via
    }
    pub fn requester(&self) -> &QrRequester {
        &self.requester
    }
    pub fn scanned_at(&self) -> Option<DateTime<Utc>> {
        self.scanned_at
    }
    pub fn decided_at(&self) -> Option<DateTime<Utc>> {
        self.decided_at
    }
    pub fn completed_at(&self) -> Option<DateTime<Utc>> {
        self.completed_at
    }
    pub fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;

    fn request(now: DateTime<Utc>) -> QrLoginRequest {
        QrLoginRequest::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            "token".to_string(),
            QrRequester::default(),
            now,
            now + Duration::minutes(2),
        )
    }

    #[test]
    fn only_a_pending_unexpired_request_is_claimable() {
        let now = Utc::now();
        let mut r = request(now);
        assert!(r.is_claimable(now));
        assert!(!r.is_claimable(now + Duration::minutes(2)));

        r.claim("claim".into(), None, now, now + Duration::minutes(10));
        assert!(!r.is_claimable(now), "claimed once");
    }

    #[test]
    fn only_the_claiming_device_and_user_can_decide() {
        let now = Utc::now();
        let user = Uuid::now_v7();
        let mut r = request(now);
        r.claim("claim".into(), Some(user), now, now + Duration::minutes(10));

        assert!(r.is_decidable_by("claim", Some(user), now));
        assert!(
            !r.is_decidable_by("other", Some(user), now),
            "wrong claim code"
        );
        assert!(
            !r.is_decidable_by("claim", Some(Uuid::now_v7()), now),
            "another user"
        );
        assert!(
            !r.is_decidable_by("claim", None, now),
            "an app's claim needs its user"
        );
        assert!(!r.is_decidable_by("claim", Some(user), now + Duration::minutes(10)));

        // A hosted-page claim binds no user: whoever signs in on that device decides.
        let mut hosted = request(now);
        hosted.claim("claim".into(), None, now, now + Duration::minutes(10));
        assert!(hosted.is_decidable_by("claim", Some(user), now));

        r.approve(user, false, QrApprover::SignedInApp, now);
        assert!(!r.is_decidable_by("claim", Some(user), now), "decided once");
    }

    #[test]
    fn statuses_round_trip_and_fail_closed() {
        for status in [
            QrLoginStatus::Pending,
            QrLoginStatus::Scanned,
            QrLoginStatus::Approved,
            QrLoginStatus::Denied,
            QrLoginStatus::Completed,
        ] {
            assert_eq!(QrLoginStatus::from_db(status.as_str()), status);
        }
        assert_eq!(QrLoginStatus::from_db("garbage"), QrLoginStatus::Denied);
        assert_eq!(
            QrApprover::from_db("hosted_page"),
            Some(QrApprover::HostedPage)
        );
        assert_eq!(QrApprover::from_db("garbage"), None);
    }
}
