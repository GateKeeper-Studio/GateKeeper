use chrono::{DateTime, Utc};
use uuid::Uuid;

// How the SMTP connection is secured. Mirrors lettre's three connection modes
// and is persisted as a lowercase discriminator in tenant_emails.encryption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmtpEncryption {
    // Upgrade a plaintext connection to TLS via STARTTLS (typically port 587).
    StartTls,
    // Implicit TLS for the whole connection (typically port 465).
    Tls,
    // No transport security — for local/dev relays only.
    None,
}

impl SmtpEncryption {
    pub fn as_str(&self) -> &'static str {
        match self {
            SmtpEncryption::StartTls => "starttls",
            SmtpEncryption::Tls => "tls",
            SmtpEncryption::None => "none",
        }
    }

    pub fn from_db(value: &str) -> Self {
        match value {
            "tls" => SmtpEncryption::Tls,
            "none" => SmtpEncryption::None,
            _ => SmtpEncryption::StartTls,
        }
    }
}

// A tenant-owned SMTP email provider (e.g. "SMTP Gmail"). An OAuth client points
// at one of these to choose how its users' emails are delivered.
//
// SECURITY: the SMTP password is never stored in the clear — `password_encrypted`
// holds an AES-256-GCM ciphertext (see SecretCipher) and is decrypted only at the
// moment of sending. It is never exposed through any API projection.
#[derive(Debug, Clone)]
pub struct EmailProvider {
    id: Uuid,
    tenant_id: Uuid,

    name: String,
    host: String,
    port: i32,
    username: String,
    password_encrypted: String,

    from_email: String,
    from_name: Option<String>,
    encryption: SmtpEncryption,

    // At most one default per tenant (enforced by a partial unique index). The
    // default is used when an OAuth client has not picked a specific provider.
    is_default: bool,
    is_active: bool,

    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl EmailProvider {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: Uuid,
        name: String,
        host: String,
        port: i32,
        username: String,
        password_encrypted: String,
        from_email: String,
        from_name: Option<String>,
        encryption: SmtpEncryption,
        is_default: bool,
    ) -> Self {
        EmailProvider {
            id: Uuid::now_v7(),
            tenant_id,
            name,
            host,
            port,
            username,
            password_encrypted,
            from_email,
            from_name,
            encryption,
            is_default,
            is_active: true,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        tenant_id: Uuid,
        name: String,
        host: String,
        port: i32,
        username: String,
        password_encrypted: String,
        from_email: String,
        from_name: Option<String>,
        encryption: SmtpEncryption,
        is_default: bool,
        is_active: bool,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    ) -> Self {
        EmailProvider {
            id,
            tenant_id,
            name,
            host,
            port,
            username,
            password_encrypted,
            from_email,
            from_name,
            encryption,
            is_default,
            is_active,
            created_at,
            updated_at,
        }
    }

    // Mutates the editable details (admin CRUD). `password_encrypted` is updated
    // only when the admin supplies a new password (handled in the use case); the
    // id/tenant_id are never changed here.
    #[allow(clippy::too_many_arguments)]
    pub fn update_details(
        &mut self,
        name: String,
        host: String,
        port: i32,
        username: String,
        password_encrypted: String,
        from_email: String,
        from_name: Option<String>,
        encryption: SmtpEncryption,
        is_default: bool,
        is_active: bool,
    ) {
        self.name = name;
        self.host = host;
        self.port = port;
        self.username = username;
        self.password_encrypted = password_encrypted;
        self.from_email = from_email;
        self.from_name = from_name;
        self.encryption = encryption;
        self.is_default = is_default;
        self.is_active = is_active;
        self.updated_at = Some(Utc::now());
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn host(&self) -> &str {
        &self.host
    }
    pub fn port(&self) -> i32 {
        self.port
    }
    pub fn username(&self) -> &str {
        &self.username
    }
    pub fn password_encrypted(&self) -> &str {
        &self.password_encrypted
    }
    pub fn from_email(&self) -> &str {
        &self.from_email
    }
    pub fn from_name(&self) -> Option<&str> {
        self.from_name.as_deref()
    }
    pub fn encryption(&self) -> &SmtpEncryption {
        &self.encryption
    }
    pub fn is_default(&self) -> bool {
        self.is_default
    }
    pub fn is_active(&self) -> bool {
        self.is_active
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn updated_at(&self) -> Option<DateTime<Utc>> {
        self.updated_at
    }
}
