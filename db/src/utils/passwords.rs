use argon2rs::verifier::Encoded;
use rand;
use rand::distributions::Alphanumeric;
use rand::Rng;
use std::str;
use utils::errors::{DatabaseError, ErrorCode};

pub struct PasswordHash {
    hash: Encoded,
}

impl PasswordHash {
    pub fn generate(password: &str, salt: Option<&str>) -> PasswordHash {
        let salty = match salt {
            Some(s) => s.to_string(),
            None => PasswordHash::generate_salt(),
        };
        let hash = Encoded::default2i(password.as_bytes(), &salty.as_bytes(), b"", b"");
        PasswordHash { hash }
    }

    // Generate a hash from a serialised string.
    pub fn from_str(hash_str: &str) -> Result<PasswordHash, DatabaseError> {
        match Encoded::from_u8(hash_str.as_bytes()) {
            Ok(hash) => Ok(PasswordHash { hash }),
            Err(e) => Err(DatabaseError::new(
                ErrorCode::InvalidInput,
                Some(e.to_string()),
            )),
        }
    }

    pub fn to_string(&self) -> String {
        String::from_utf8(self.hash.to_u8()).unwrap()
    }

    pub fn verify(&self, pw: &str) -> bool {
        self.hash.verify(pw.as_bytes())
    }

    pub fn generate_salt() -> String {
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .collect::<String>()
    }
}

#[test]
fn hashes_with_salt() {
    let sh = PasswordHash::generate("test", Some("somesalt"));
    assert_eq!(
        sh.to_string(),
        "$argon2i$m=4096,t=3,p=1$c29tZXNhbHQ$rVJmrKufM5nm57O4lxgQoBmRXtL42QhxbKzskhnBaMg"
    )
}

#[test]
fn verify_password() {
    let sh = PasswordHash::from_str("$argon2i$m=4096,t=3,p=1$c29tZXNhbHQ$rVJmrKufM5nm57O4lxgQoBmRXtL42QhxbKzskhnBaMg")
        .unwrap();
    assert!(sh.verify("test"));
    assert!(!sh.verify("wrong_password"));
}

#[test]
fn verify_library_hash() {
    let sh = PasswordHash::from_str(
        "$argon2i$m=4096,t=3,p=1$dG9kbzogZnV6eiB0ZXN0cw$Eh1lW3mjkhlMLRQdE7vXZnvwDXSGLBfXa6BGK4a1J3s",
    )
    .unwrap();
    assert!(sh.verify("argon2i!"));
}

#[test]
fn invalid_hash() {
    let sh = PasswordHash::from_str("$argon2i$p=invalid");
    assert!(sh.is_err());
}

#[test]
fn hashes_without_salt() {
    let sh = PasswordHash::generate("test", None);
    assert_eq!(&sh.to_string()[..8], "$argon2i");
}

#[test]
fn random_salt() {
    let salt = PasswordHash::generate_salt();
    assert_eq!(salt.len(), 32);
}
