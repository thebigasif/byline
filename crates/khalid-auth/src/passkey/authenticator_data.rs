//! The authenticator data a passkey signs (`WebAuthn` Level 2, section 6.1).

use super::{PasskeyError, PublicKey};

/// User present: someone touched the authenticator.
const UP: u8 = 0x01;
/// User verified: by fingerprint, face or PIN.
const UV: u8 = 0x04;
/// Attested credential data follows: only at registration.
const AT: u8 = 0x40;

/// What an authenticator says about itself in every answer: which site the
/// passkey is for, whether the person unlocked it, its counter, and at
/// registration the new credential.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatorData {
    rp_id_hash: [u8; 32],
    flags: u8,
    sign_count: u32,
    credential: Option<(Vec<u8>, PublicKey)>,
}

impl AuthenticatorData {
    /// Reads `bytes`: 32 bytes of relying-party hash, one of flags, four of
    /// counter, then the credential when the `AT` flag is set.
    ///
    /// # Errors
    ///
    /// Returns [`PasskeyError::Malformed`] for bytes that end early or a
    /// key that cannot be read, and [`PasskeyError::UnsupportedKey`] for a
    /// key that is not ES256 or Ed25519.
    pub fn parse(bytes: &[u8]) -> Result<Self, PasskeyError> {
        let (rp_id_hash, rest) = take::<32>(bytes)?;
        let ([flags], rest) = take::<1>(rest)?;
        let (count, rest) = take::<4>(rest)?;
        let credential = if flags & AT == 0 {
            None
        } else {
            let (_aaguid, rest) = take::<16>(rest)?;
            let (length, rest) = take::<2>(rest)?;
            let length = usize::from(u16::from_be_bytes(length));
            let id = rest.get(..length).ok_or(PasskeyError::Malformed)?;
            let key = rest.get(length..).ok_or(PasskeyError::Malformed)?;
            Some((id.to_vec(), PublicKey::from_credential_key(key)?))
        };
        Ok(Self {
            rp_id_hash,
            flags,
            sign_count: u32::from_be_bytes(count),
            credential,
        })
    }

    /// The SHA-256 of the host name the passkey belongs to.
    #[must_use]
    pub fn rp_id_hash(&self) -> &[u8; 32] {
        &self.rp_id_hash
    }

    /// Whether the person was present and unlocked the passkey with a
    /// fingerprint, a face or a PIN.
    #[must_use]
    pub fn user_verified(&self) -> bool {
        self.flags & UP != 0 && self.flags & UV != 0
    }

    /// How many times the passkey has signed, or 0 for one that does not
    /// count, as synced passkeys do not.
    #[must_use]
    pub fn sign_count(&self) -> u32 {
        self.sign_count
    }

    /// The new credential's identity and key, present at registration.
    #[must_use]
    pub fn into_credential(self) -> Option<(Vec<u8>, PublicKey)> {
        self.credential
    }
}

/// The first `N` bytes of `bytes` and the rest.
fn take<const N: usize>(bytes: &[u8]) -> Result<([u8; N], &[u8]), PasskeyError> {
    let head = bytes.get(..N).ok_or(PasskeyError::Malformed)?;
    let rest = bytes.get(N..).ok_or(PasskeyError::Malformed)?;
    let head: [u8; N] = head.try_into().map_err(|_| PasskeyError::Malformed)?;
    Ok((head, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(flags: u8, credential: Option<(&[u8], &[u8])>) -> Vec<u8> {
        let mut bytes = vec![9; 32];
        bytes.push(flags);
        bytes.extend(7_u32.to_be_bytes());
        if let Some((id, key)) = credential {
            bytes.extend([0; 16]);
            bytes.extend(u16::try_from(id.len()).unwrap().to_be_bytes());
            bytes.extend(id);
            bytes.extend(key);
        }
        bytes
    }

    #[test]
    fn flags_and_the_counter_read_back() {
        let parsed = AuthenticatorData::parse(&data(UP | UV, None)).unwrap();
        assert_eq!((parsed.rp_id_hash(), parsed.sign_count()), (&[9; 32], 7));
        assert!(parsed.user_verified());
        assert_eq!(parsed.into_credential(), None);
        for flags in [UP, UV] {
            let parsed = AuthenticatorData::parse(&data(flags, None)).unwrap();
            assert!(!parsed.user_verified(), "{flags:#x}");
        }
    }

    #[test]
    fn short_or_overlong_data_is_malformed() {
        assert_eq!(
            AuthenticatorData::parse(&[0; 36]),
            Err(PasskeyError::Malformed)
        );
        let mut lying = data(UP | UV | AT, Some((&[1; 4], &[])));
        lying.truncate(lying.len() - 2);
        assert_eq!(
            AuthenticatorData::parse(&lying),
            Err(PasskeyError::Malformed)
        );
    }
}
