//! The user's identity details. Document numbers are deliberately absent (plan section 5).

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::core::register::{IdentityField, is_email};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub full_name: String,
    #[serde(default)]
    pub alias_names: Vec<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    /// `YYYY-MM-DD`.
    pub date_of_birth: Option<String>,
    pub address: Option<String>,
    #[serde(default)]
    pub previous_addresses: Vec<String>,
    pub postcode: Option<String>,
}

impl Profile {
    /// The value a letter shows for `field`, or `None` when the profile has none. Lists are
    /// joined with semicolons.
    pub fn value(&self, field: IdentityField) -> Option<String> {
        let some = |s: &Option<String>| s.clone().filter(|s| !s.is_empty());
        let list = |v: &[String]| Some(v.join("; ")).filter(|s| !s.is_empty());
        match field {
            IdentityField::FullName => Some(self.full_name.clone()).filter(|s| !s.is_empty()),
            IdentityField::AliasNames => list(&self.alias_names),
            IdentityField::Email => some(&self.email),
            IdentityField::Phone => some(&self.phone),
            IdentityField::DateOfBirth => some(&self.date_of_birth),
            IdentityField::Address => some(&self.address),
            IdentityField::PreviousAddresses => list(&self.previous_addresses),
            IdentityField::Postcode => some(&self.postcode),
        }
    }

    /// Problems that make the profile unusable, one message each.
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.full_name.trim().is_empty() {
            problems.push("full name must not be empty".to_owned());
        }
        if let Some(email) = &self.email
            && !is_email(email)
        {
            problems.push(format!("`{email}` is not an email address"));
        }
        if let Some(date) = &self.date_of_birth
            && NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err()
        {
            problems.push(format!(
                "date of birth `{date}` is not a date such as 1980-01-31"
            ));
        }
        if let Some(postcode) = &self.postcode
            && !(postcode.len() == 4 && postcode.bytes().all(|b| b.is_ascii_digit()))
        {
            problems.push(format!("postcode `{postcode}` is not 4 digits"));
        }
        let all = [&self.full_name]
            .into_iter()
            .chain(&self.alias_names)
            .chain(&self.email)
            .chain(&self.phone)
            .chain(&self.address)
            .chain(&self.previous_addresses);
        if all.into_iter().any(|s| s.contains(['\r', '\n'])) {
            problems.push("details must not contain line breaks".to_owned());
        }
        problems
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> Profile {
        Profile {
            full_name: "James McGill".to_owned(),
            alias_names: vec!["Saul Goodman".to_owned(), "Gene Takavic".to_owned()],
            email: Some("jimmy@example.com".to_owned()),
            date_of_birth: Some("1960-11-12".to_owned()),
            postcode: Some("2000".to_owned()),
            ..Profile::default()
        }
    }

    #[test]
    fn values_join_lists_and_skip_empty_fields() {
        let p = profile();
        assert_eq!(
            p.value(IdentityField::AliasNames).as_deref(),
            Some("Saul Goodman; Gene Takavic")
        );
        assert_eq!(p.value(IdentityField::Phone), None);
        assert_eq!(p.value(IdentityField::PreviousAddresses), None);
        assert!(p.problems().is_empty());
    }

    #[test]
    fn rejects_bad_details() {
        let p = Profile {
            full_name: " ".to_owned(),
            email: Some("jimmy".to_owned()),
            date_of_birth: Some("12/11/1960".to_owned()),
            postcode: Some("200".to_owned()),
            address: Some("1 Main St\nTo: someone else".to_owned()),
            ..profile()
        };
        let problems = p.problems().join("\n");
        for needle in [
            "full name",
            "email",
            "date of birth",
            "postcode",
            "line breaks",
        ] {
            assert!(
                problems.contains(needle),
                "{needle} missing from {problems}"
            );
        }
    }
}
