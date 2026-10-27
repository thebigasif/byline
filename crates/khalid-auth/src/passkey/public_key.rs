//! A passkey's public key: read at registration from the CBOR key the authenticator sends, stored in a
//! short form of our own, and used to check signatures with `ring`.

use ciborium::Value;
use ring::signature::{ECDSA_P256_SHA256_ASN1, ED25519, UnparsedPublicKey};

use super::PasskeyError;

/// Algorithm number for ES256: ECDSA on P-256 with SHA-256 (RFC 9053).
pub(crate) const ES256: i64 = -7;
/// Algorithm number for `EdDSA`, here always Ed25519 (RFC 9053).
pub(crate) const EDDSA: i64 = -8;

/// A public key Khalid accepts.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PublicKey {
    /// A P-256 point, uncompressed: `0x04`, then x, then y.
    Es256([u8; 65]),
    /// An Ed25519 key.
    Ed25519([u8; 32]),
}

impl PublicKey {
    /// Reads the credential key, in CBOR, at the start of `bytes` (RFC 9052, section 7).
    /// Bytes after it, such as extensions, are ignored.
    ///
    /// # Errors
    ///
    /// Returns [`PasskeyError::UnsupportedKey`] for a key that is not ES256
    /// or Ed25519, and [`PasskeyError::Malformed`] for one that cannot be read.
    pub fn from_credential_key(bytes: &[u8]) -> Result<Self, PasskeyError> {
        let value: Value = ciborium::from_reader(bytes).map_err(|_| PasskeyError::Malformed)?;
        let map = value.as_map().ok_or(PasskeyError::Malformed)?;
        let field = |label: i64| {
            map.iter()
                .find(|(key, _)| {
                    key.as_integer().and_then(|key| i64::try_from(key).ok()) == Some(label)
                })
                .map(|(_, value)| value)
        };
        let int = |label| {
            field(label)
                .and_then(Value::as_integer)
                .and_then(|value| i64::try_from(value).ok())
        };
        let bytes32 = |label| -> Result<[u8; 32], PasskeyError> {
            let value = field(label)
                .and_then(Value::as_bytes)
                .ok_or(PasskeyError::Malformed)?;
            value
                .as_slice()
                .try_into()
                .map_err(|_| PasskeyError::Malformed)
        };
        // kty 2 is EC2 and crv 1 P-256; kty 1 is OKP and crv 6 Ed25519.
        match (int(1), int(3), int(-1)) {
            (Some(2), Some(ES256), Some(1)) => {
                let (x, y) = (bytes32(-2)?, bytes32(-3)?);
                let mut point = [0x04; 65];
                point
                    .get_mut(1..33)
                    .ok_or(PasskeyError::Malformed)?
                    .copy_from_slice(&x);
                point
                    .get_mut(33..)
                    .ok_or(PasskeyError::Malformed)?
                    .copy_from_slice(&y);
                Ok(Self::Es256(point))
            }
            (Some(1), Some(EDDSA), Some(6)) => Ok(Self::Ed25519(bytes32(-2)?)),
            _ => Err(PasskeyError::UnsupportedKey),
        }
    }

    /// The form a store keeps: one byte naming the algorithm, then the key.
    #[must_use]
    pub fn to_stored(&self) -> Vec<u8> {
        match self {
            Self::Es256(point) => [&[1], point.as_slice()].concat(),
            Self::Ed25519(key) => [&[2], key.as_slice()].concat(),
        }
    }

    /// A key read back from the store.
    ///
    /// # Errors
    ///
    /// Returns [`PasskeyError::Malformed`] for bytes [`Self::to_stored`]
    /// did not write.
    pub fn from_stored(bytes: &[u8]) -> Result<Self, PasskeyError> {
        match bytes.split_first() {
            Some((1, point)) => point.try_into().map(Self::Es256),
            Some((2, key)) => key.try_into().map(Self::Ed25519),
            _ => return Err(PasskeyError::Malformed),
        }
        .map_err(|_| PasskeyError::Malformed)
    }

    /// Whether `signature` signs `message` with this key.
    #[must_use]
    pub fn verifies(&self, message: &[u8], signature: &[u8]) -> bool {
        let result = match self {
            Self::Es256(point) => {
                UnparsedPublicKey::new(&ECDSA_P256_SHA256_ASN1, point).verify(message, signature)
            }
            Self::Ed25519(key) => UnparsedPublicKey::new(&ED25519, key).verify(message, signature),
        };
        result.is_ok()
    }
}

#[cfg(test)]
mod tests {
    use ring::rand::SystemRandom;
    use ring::signature::{ECDSA_P256_SHA256_ASN1_SIGNING, EcdsaKeyPair, KeyPair};

    use super::*;

    fn cbor(pairs: Vec<(i64, Value)>) -> Vec<u8> {
        let map = pairs
            .into_iter()
            .map(|(label, value)| (Value::Integer(label.into()), value))
            .collect();
        let mut bytes = Vec::new();
        ciborium::into_writer(&Value::Map(map), &mut bytes).unwrap();
        bytes
    }

    fn int(n: i64) -> Value {
        Value::Integer(n.into())
    }

    fn es256(x: &[u8], y: &[u8]) -> Vec<u8> {
        let (x, y) = (Value::Bytes(x.to_vec()), Value::Bytes(y.to_vec()));
        cbor(vec![
            (1, int(2)),
            (3, int(ES256)),
            (-1, int(1)),
            (-2, x),
            (-3, y),
        ])
    }

    #[test]
    fn both_kinds_of_key_read_and_store_back() {
        let key = PublicKey::from_credential_key(&es256(&[1; 32], &[2; 32])).unwrap();
        let mut point = [2; 65];
        point[0] = 4;
        point[1..33].fill(1);
        assert_eq!(key, PublicKey::Es256(point));
        let x = Value::Bytes(vec![5; 32]);
        let ed = cbor(vec![(1, int(1)), (3, int(EDDSA)), (-1, int(6)), (-2, x)]);
        let ed = PublicKey::from_credential_key(&ed).unwrap();
        assert_eq!(ed, PublicKey::Ed25519([5; 32]));
        for key in [key, ed] {
            assert_eq!(PublicKey::from_stored(&key.to_stored()), Ok(key));
        }
    }

    #[test]
    fn other_keys_and_broken_ones_are_refused() {
        let rsa = cbor(vec![(1, int(3)), (3, int(-257))]);
        let refused = |bytes: &[u8]| PublicKey::from_credential_key(bytes).err();
        assert_eq!(refused(&rsa), Some(PasskeyError::UnsupportedKey));
        assert_eq!(
            refused(&es256(&[1; 31], &[2; 32])),
            Some(PasskeyError::Malformed)
        );
        assert_eq!(refused(b"\xff"), Some(PasskeyError::Malformed));
        for stored in [&[][..], &[1, 2, 3], &[3; 33], &[2; 34]] {
            let read = PublicKey::from_stored(stored);
            assert_eq!(read, Err(PasskeyError::Malformed), "{stored:?}");
        }
    }

    #[test]
    fn a_key_checks_its_own_signatures_only() {
        let rng = SystemRandom::new();
        let alg = &ECDSA_P256_SHA256_ASN1_SIGNING;
        let pkcs8 = EcdsaKeyPair::generate_pkcs8(alg, &rng).unwrap();
        let pair = EcdsaKeyPair::from_pkcs8(alg, pkcs8.as_ref(), &rng).unwrap();
        let point: [u8; 65] = pair.public_key().as_ref().try_into().unwrap();
        let key = PublicKey::Es256(point);
        let signature = pair.sign(&rng, b"signed").unwrap();
        assert!(key.verifies(b"signed", signature.as_ref()));
        assert!(!key.verifies(b"other", signature.as_ref()));
        assert!(!key.verifies(b"signed", b"not a signature"));
    }
}
