use serde::Serialize;

use crate::domain::entities::admin_user::AdminUser;

// What the console shows about the signed-in admin. Carries no credential.
// camelCase because the dashboard's `AdminIdentity` type reads it as is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminIdentity {
    pub id: String,
    pub email: String,
    pub organization_id: String,
    // Friendly label: the account name, else derived from the email.
    pub name: String,
    // 1–2 letter avatar fallback derived from `name`.
    pub initials: String,
}

impl AdminIdentity {
    pub fn of(admin: &AdminUser) -> Self {
        let name = match admin.name().trim() {
            "" => name_from_email(admin.email()),
            name => name.to_string(),
        };
        AdminIdentity {
            id: admin.id().to_string(),
            email: admin.email().to_string(),
            organization_id: admin.organization_id().to_string(),
            initials: initials_from_name(&name),
            name,
        }
    }
}

// "jane.doe@x.com" -> "Jane Doe".
fn name_from_email(email: &str) -> String {
    let local = email.split('@').next().unwrap_or(email);
    let name = local
        .split(['.', '_', '-'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    if name.is_empty() {
        email.to_string()
    } else {
        name
    }
}

fn initials_from_name(name: &str) -> String {
    let parts: Vec<&str> = name.split_whitespace().collect();
    match parts.as_slice() {
        [] => "?".to_string(),
        [only] => only.chars().take(2).collect::<String>().to_uppercase(),
        [first, .., last] => first
            .chars()
            .take(1)
            .chain(last.chars().take(1))
            .collect::<String>()
            .to_uppercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_initials_match_the_former_dashboard_rules() {
        assert_eq!(name_from_email("jane.doe@x.com"), "Jane Doe");
        assert_eq!(name_from_email("ops_team-lead@x.com"), "Ops Team Lead");
        assert_eq!(initials_from_name("Jane Doe"), "JD");
        assert_eq!(initials_from_name("Ada Lovelace King"), "AK");
        assert_eq!(initials_from_name("ada"), "AD");
        assert_eq!(initials_from_name(""), "?");
    }
}
