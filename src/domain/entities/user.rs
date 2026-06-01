use chrono::{DateTime, Utc};
use uuid::{Uuid};

use crate::domain::entities::user_credentials::UserCredentials;
use crate::domain::entities::user_profile::UserProfile;

#[derive(Debug, Clone)]
pub struct User {
  id: Uuid,
  tenant_id: Uuid,

  email: String,
  is_email_verified: bool,

  profile: Option<UserProfile>,
  credentials: Option<UserCredentials>,

  created_at: DateTime<Utc>,
  updated_at: Option<DateTime<Utc>>,
}

impl User {
  pub fn new(tenant_id: Uuid, email: String, profile: Option<UserProfile>, credentials: Option<UserCredentials>) -> Self {
    User {
      id: Uuid::now_v7(),
      tenant_id,
      email,
      is_email_verified: false,
      profile,
      credentials,
      created_at: Utc::now(),
      updated_at: None,
    }
  }

  pub fn reconstruct(
    id: Uuid,
    email: String,
    tenant_id: Uuid,
    is_email_verified: bool,
    profile: Option<UserProfile>,
    credentials: Option<UserCredentials>,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
  ) -> Self {
    User {
      id,
      tenant_id,
      email,
      is_email_verified,
      profile,
      credentials,
      created_at,
      updated_at,
    }
  }

  pub fn set_profile(&mut self, profile: UserProfile) {
    self.profile = Some(profile);
  }

  pub fn set_credentials(&mut self, credentials: UserCredentials) {
    self.credentials = Some(credentials);
  }

  pub fn update_profile(&mut self, profile: UserProfile) {
    self.profile = Some(profile);
    self.updated_at = Some(Utc::now());
  }

  pub fn update_credentials(&mut self, credentials: UserCredentials) {
    self.credentials = Some(credentials);
    self.updated_at = Some(Utc::now());
  }

  pub fn verify_email(&mut self) {
    self.is_email_verified = true;
    self.updated_at = Some(Utc::now());
  }

  pub fn tenant_id(&self) -> Uuid { self.tenant_id }
  pub fn id(&self) -> Uuid { self.id }
  pub fn email(&self) -> &str { &self.email }
  pub fn is_email_verified(&self) -> bool { self.is_email_verified }
  pub fn profile(&self) -> Option<&UserProfile> { self.profile.as_ref() }
  pub fn credentials(&self) -> Option<&UserCredentials> { self.credentials.as_ref() }
  pub fn created_at(&self) -> DateTime<Utc> { self.created_at }
  pub fn updated_at(&self) -> Option<DateTime<Utc>> { self.updated_at }
}