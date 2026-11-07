// OIDC RP-Initiated Logout 1.0: an RP sends the user here to sign out. Ending the
// user's sessions fans out to the other RPs through back-channel logout.
pub mod command;
pub mod response;
pub mod tests;
pub mod use_case;
