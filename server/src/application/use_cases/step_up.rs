// Step-up: fresh proof that the person holding an `account` token is the account's
// owner, asked for before a sensitive self-service action (deleting the account,
// removing a sign-in method, adding a passkey to an account without a password).
//
// An account with a password proves it with `current_password`. One without a
// password cannot, so `start` opens a ceremony instead: a passkey assertion when the
// account has a passkey, otherwise a 6-digit code emailed to its verified address.
// `confirm` checks whichever proof applies. See docs/FLOWS-AUTHENTICATION.md §2.
pub mod confirm;
pub mod start;

#[cfg(test)]
mod flow_tests;
