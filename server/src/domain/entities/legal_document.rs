use chrono::{DateTime, Utc};
use uuid::Uuid;

/// The kind of legal document. Persisted as a short string in `document_type`
/// and exposed in URLs via the friendly `terms` / `privacy` slugs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LegalDocumentType {
    #[default]
    TermsOfService,
    PrivacyPolicy,
}

impl LegalDocumentType {
    /// Stored value in the `document_type` column.
    pub fn as_str(&self) -> &'static str {
        match self {
            LegalDocumentType::TermsOfService => "terms_of_service",
            LegalDocumentType::PrivacyPolicy => "privacy_policy",
        }
    }

    /// Parse the stored column value back into the enum.
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "terms_of_service" => Some(LegalDocumentType::TermsOfService),
            "privacy_policy" => Some(LegalDocumentType::PrivacyPolicy),
            _ => None,
        }
    }

    /// Parse the URL slug (`terms` / `privacy`) used by public + admin routes.
    /// Also accepts the stored column values for convenience.
    pub fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "terms" | "terms_of_service" => Some(LegalDocumentType::TermsOfService),
            "privacy" | "privacy_policy" => Some(LegalDocumentType::PrivacyPolicy),
            _ => None,
        }
    }

    /// Built-in default markdown served when a tenant has not customized the
    /// document. Single source of truth for the default copy.
    pub fn default_markdown(&self) -> String {
        match self {
            LegalDocumentType::TermsOfService => DEFAULT_TERMS_OF_SERVICE.to_string(),
            LegalDocumentType::PrivacyPolicy => DEFAULT_PRIVACY_POLICY.to_string(),
        }
    }
}

/// A tenant's editable Terms of Service or Privacy Policy, stored as markdown.
#[derive(Debug, Clone)]
pub struct LegalDocument {
    tenant_id: Uuid,
    document_type: LegalDocumentType,
    markdown_content: String,
    updated_at: DateTime<Utc>,
}

impl LegalDocument {
    pub fn new(
        tenant_id: Uuid,
        document_type: LegalDocumentType,
        markdown_content: String,
    ) -> Self {
        LegalDocument {
            tenant_id,
            document_type,
            markdown_content,
            updated_at: Utc::now(),
        }
    }

    pub fn reconstruct(
        tenant_id: Uuid,
        document_type: LegalDocumentType,
        markdown_content: String,
        updated_at: DateTime<Utc>,
    ) -> Self {
        LegalDocument {
            tenant_id,
            document_type,
            markdown_content,
            updated_at,
        }
    }

    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }
    pub fn document_type(&self) -> LegalDocumentType {
        self.document_type
    }
    pub fn markdown_content(&self) -> &str {
        &self.markdown_content
    }
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

const DEFAULT_TERMS_OF_SERVICE: &str = r#"These Terms of Service (the "Terms") govern your access to and use of the Service. By creating an account or otherwise using the Service, you agree to be bound by these Terms. If you do not agree, do not use the Service.

## 1. Acceptance of Terms

By accessing or using the Service you confirm that you are at least the age of majority in your jurisdiction and that you have the legal capacity to enter into these Terms. If you are using the Service on behalf of an organization, you represent that you are authorized to bind that organization to these Terms.

## 2. Description of the Service

The Service provides identity and access management, including authentication, authorization, and related account-management features. We may add, change, or remove functionality at any time without prior notice.

## 3. Accounts and Registration

To use certain features you must register for an account. You agree to provide accurate and complete information and to keep it up to date. You are responsible for safeguarding your credentials and for all activity that occurs under your account. Notify us immediately of any unauthorized use.

## 4. Acceptable Use

You agree not to:

- Use the Service in violation of any applicable law or regulation;
- Attempt to gain unauthorized access to any system, account, or data;
- Interfere with or disrupt the integrity or performance of the Service;
- Reverse engineer or attempt to extract source code, except as permitted by law;
- Use the Service to transmit malware or other harmful code.

## 5. Intellectual Property

The Service and all related content, features, and functionality are owned by us or our licensors and are protected by intellectual-property laws. These Terms do not grant you any right to use our trademarks, logos, or other brand features.

## 6. Third-Party Services

The Service may integrate with third-party services, such as external identity providers. Your use of those services is governed by their own terms and privacy policies, and we are not responsible for them.

## 7. Termination

We may suspend or terminate your access to the Service at any time, with or without cause and with or without notice. Upon termination, your right to use the Service immediately ceases. Provisions that by their nature should survive termination will survive.

## 8. Disclaimer of Warranties

The Service is provided "as is" and "as available" without warranties of any kind, whether express or implied, including warranties of merchantability, fitness for a particular purpose, and non-infringement. We do not warrant that the Service will be uninterrupted, secure, or error-free.

## 9. Limitation of Liability

To the maximum extent permitted by law, we will not be liable for any indirect, incidental, special, consequential, or punitive damages, or any loss of profits or data, arising out of or related to your use of the Service.

## 10. Changes to These Terms

We may update these Terms from time to time. When we do, we will revise the "Last updated" date above. Your continued use of the Service after changes take effect constitutes acceptance of the revised Terms.

## 11. Contact Us

If you have questions about these Terms, contact us at [legal@example.com](mailto:legal@example.com).
"#;

const DEFAULT_PRIVACY_POLICY: &str = r#"This Privacy Policy explains how we ("we", "us", or "our") collect, use, and protect your personal information when you use our identity and access management service (the "Service").

## 1. Information We Collect

We collect the following categories of information:

- **Account information** — such as your email address, name, and password credentials used to authenticate you.
- **Authentication data** — such as multi-factor authentication settings, security keys, and session details.
- **Usage data** — such as log data, IP address, browser type, and interactions with the Service.

## 2. How We Use Your Information

We use the information we collect to:

- Provide, maintain, and secure the Service;
- Authenticate users and prevent fraud or unauthorized access;
- Communicate with you about your account and security events;
- Comply with legal obligations and enforce our terms.

## 3. Cookies and Similar Technologies

We use cookies and similar technologies to keep you signed in, remember your preferences, and understand how the Service is used. You can control cookies through your browser settings, though disabling them may affect functionality.

## 4. How We Share Information

We do not sell your personal information. We may share it with service providers who process data on our behalf, with third-party identity providers you choose to connect, and when required by law or to protect our rights and the safety of others.

## 5. Data Retention

We retain your personal information for as long as your account is active or as needed to provide the Service, comply with our legal obligations, resolve disputes, and enforce our agreements.

## 6. Data Security

We implement technical and organizational measures designed to protect your information against unauthorized access, alteration, disclosure, or destruction. However, no method of transmission or storage is completely secure, and we cannot guarantee absolute security.

## 7. Your Rights

Depending on your location, you may have the right to access, correct, delete, or export your personal information, and to object to or restrict certain processing. To exercise these rights, contact us using the details below.

## 8. International Data Transfers

Your information may be processed in countries other than your own. Where required, we put appropriate safeguards in place to protect your information when it is transferred internationally.

## 9. Children's Privacy

The Service is not directed to children under the age required by applicable law, and we do not knowingly collect personal information from them. If you believe a child has provided us with personal information, please contact us so we can remove it.

## 10. Changes to This Policy

We may update this Privacy Policy from time to time. When we do, we will revise the "Last updated" date above. We encourage you to review this page periodically.

## 11. Contact Us

If you have questions about this Privacy Policy or our data practices, contact us at [privacy@example.com](mailto:privacy@example.com).
"#;
