#![forbid(unsafe_code)]

// CLI: create a platform-admin account.
//
//   cargo run --bin create_admin -- <email> <password> [organization_id]
//
// Hashes the password with Argon2id using the server-wide pepper (JWT_SECRET) — the
// SAME pepper the running API uses to verify admin logins — and inserts a row into
// `admin_users`. This is how the first admin is bootstrapped (and how more are added
// later). Requires DATABASE_URL + JWT_SECRET in the environment / .env.
//
// Every admin belongs to an organization. Pass an organization_id as the third
// argument to attach the admin to it; if omitted, a "Default Organization" is
// found or created and used.

use uuid::Uuid;
use web_api::{
    domain::{
        entities::{admin_user::AdminUser, organization::Organization},
        repositories::{
            admin_user_repository::AdminUserRepository,
            organization_repository::OrganizationRepository,
        },
    },
    infraestructure::{
        config::AppConfig,
        database::{
            database_pool::create_database_pool,
            repositories::{
                admin_user_repository::DbAdminUserRepository,
                organization_repository::DbOrganizationRepository,
            },
        },
    },
    utils::password_hasher::hash_password,
};

const DEFAULT_ORG_NAME: &str = "Default Organization";

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (Some(email), Some(password)) = (args.get(1), args.get(2)) else {
        eprintln!("usage: create_admin <email> <password> [organization_id]");
        std::process::exit(2);
    };

    // AppConfig loads .env (dotenvy) and validates DATABASE_URL / JWT_SECRET.
    let config = AppConfig::from_env();
    let pool = create_database_pool(&config.database_url).await;
    let org_repository = DbOrganizationRepository::new(pool.clone());
    let repository = DbAdminUserRepository::new(pool);

    // Resolve the organization: explicit id, or find-or-create the default org.
    let organization_id = match args.get(3) {
        Some(raw) => {
            let id = Uuid::parse_str(raw).unwrap_or_else(|_| {
                eprintln!("error: organization_id must be a valid UUID");
                std::process::exit(2);
            });
            match org_repository.find_by_id(id).await {
                Ok(Some(_)) => id,
                Ok(None) => {
                    eprintln!("error: organization {id} not found");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                }
            }
        }
        None => find_or_create_default_org(&org_repository).await,
    };

    let password_hash =
        hash_password(password, &config.jwt_secret).expect("failed to hash password");
    let admin = AdminUser::new(
        organization_id,
        email.to_string(),
        // Seed a display name from the email local part; the admin can change it later
        // from the account page.
        name_from_email(email),
        password_hash,
        "argon2id".to_string(),
    );
    let admin_id = admin.id();

    match repository.create(&admin).await {
        Ok(()) => {
            eprintln!("# created platform admin {email} in organization {organization_id}");
            println!("{admin_id}");
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}

// "jane.doe@acme.com" -> "Jane Doe": a friendly default display name from the email's
// local part. Splits on . _ - + and title-cases each word; falls back to the raw email.
fn name_from_email(email: &str) -> String {
    let local = email.split('@').next().unwrap_or(email);
    let words: Vec<String> = local
        .split(['.', '_', '-', '+'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect();
    if words.is_empty() {
        email.to_string()
    } else {
        words.join(" ")
    }
}

// Returns the id of the existing "Default Organization", creating it if absent.
async fn find_or_create_default_org(repo: &DbOrganizationRepository) -> Uuid {
    // The default org is the first page is small; scan for it by name.
    let existing = repo
        .find_all(1, 100)
        .await
        .unwrap_or_default()
        .into_iter()
        .find(|o| o.name() == DEFAULT_ORG_NAME);

    if let Some(org) = existing {
        return org.id();
    }

    let org = Organization::new(
        DEFAULT_ORG_NAME.to_string(),
        Some("Auto-created by create_admin".to_string()),
        true,
    );
    let id = org.id();
    if let Err(e) = repo.add(&org).await {
        eprintln!("error creating default organization: {e}");
        std::process::exit(1);
    }
    id
}
