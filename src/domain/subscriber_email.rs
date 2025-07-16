use validator::ValidateEmail;

#[derive(Debug)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(s: String) -> Result<Self, String> {
        if s.validate_email() {
            Ok(Self(s))
        } else {
            Err(format!("{} is not a valid subscriber email.", s))
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberEmail;
    use claim::{assert_err, assert_ok};
    use fake::{Fake, faker::internet::en::SafeEmail};
    use quickcheck::Arbitrary;
    use rand::{SeedableRng, rngs::StdRng};
    #[test]
    fn empty_string_is_rejected() {
        let email = "".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "ursuladomain.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_subject_js_rejected() {
        let emial = "@domain.com".to_string();
        assert_err!(SubscriberEmail::parse(emial));
    }

    #[test]
    fn valid_email_are_parsed_successfully() {
        let email = SafeEmail().fake();
        assert_ok!(SubscriberEmail::parse(email));
    }

    #[derive(Debug, Clone)]
    struct ValidateEmailFixture(pub String);

    impl quickcheck::Arbitrary for ValidateEmailFixture {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            // 从 quickcheck::Gen 取出一个 u64 作为种子
            let seed: u64 = Arbitrary::arbitrary(g);
            let mut rng = StdRng::seed_from_u64(seed);
            let email = SafeEmail().fake_with_rng(&mut rng);
            Self(email)
        }
    }

    #[quickcheck_macros::quickcheck]
    fn valid_email_are_parsed_successfully1(valid_email: ValidateEmailFixture) -> bool {
        SubscriberEmail::parse(valid_email.0).is_ok()
    }
}
