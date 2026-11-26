//! The RP cipher: how a console and this client agree on a key, and how the
//! pairing traffic is then encrypted.
//!
//! Two 16-byte values do the work. The **ambassador** is a throwaway the client
//! makes up; the **bright** is a key the console can derive without being told
//! it, because both sides can work it out from data they already share: the
//! PIN, the tables, and a byte or two of padding.
//!
//! What travels over the wire is the ambassador turned into an **aeropause**, a
//! scrambled form both sides can read. The bright behind it is never sent. With
//! the two of them, each side builds an AES-128 key and an HMAC-SHA256 IV, and
//! the traffic between them is AES-128-CFB128 with no padding.
//!
//! Ported from chiaki-ng (AGPL-3.0), `lib/src/rpcrypt.c`. The tables it uses are
//! in [`crate::rpcrypt_tables`].

use crate::rpcrypt_tables::*;

use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockEncrypt, KeyInit};
use hmac::{Hmac, Mac};
use sha2::Sha256;

/// How long every key, IV and block is.
pub const KEY_SIZE: usize = 0x10;
/// How many candidate keys each table row holds, and how many bytes one is long.
const ROW: usize = 0x20;
const ROW_BYTES: usize = 0x70;

/// Which console a key is derived for (chiaki's `ChiakiTarget`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// A PS4 on firmware 8 or 9.
    Ps4_8,
    /// A PS4 on firmware 9.
    Ps4_9,
    /// A PS4 on firmware 10 or later.
    Ps4_10,
    /// Any PS5 we have not identified further.
    Ps5Unknown,
    /// A PS5 answering RP-Version 1.0.
    Ps5_1,
}

impl Target {
    /// True when this is a PlayStation 5.
    pub fn is_ps5(self) -> bool {
        matches!(self, Target::Ps5Unknown | Target::Ps5_1)
    }

    /// The table of first-stage keys for this console.
    fn keys_0(self) -> &'static [u8] {
        if self.is_ps5() {
            &ps5_keys_0
        } else {
            &ps4_keys_0
        }
    }

    /// The table of aeropause keys for this console.
    fn keys_1(self) -> &'static [u8] {
        if self.is_ps5() {
            &ps5_keys_1
        } else {
            &ps4_keys_1
        }
    }

    /// The two tables the ambassador and bright come from.
    fn key_tables(self) -> (&'static [u8], &'static [u8]) {
        if self.is_ps5() {
            (&keys_a_ps5, &keys_b_ps5)
        } else {
            (&keys_a_ps4, &keys_b_ps4)
        }
    }

    /// The key the IV is built with.
    fn hmac_key(self) -> &'static [u8] {
        match self {
            Target::Ps5_1 => &hmac_key_ps5,
            Target::Ps4_8 | Target::Ps4_9 => &hmac_key_ps4_pre10,
            _ => &hmac_key_ps4,
        }
    }
}

/// What went wrong deriving keys or encrypting.
#[derive(Debug, thiserror::Error)]
pub enum RPCryptError {
    /// The console kind cannot use this step.
    #[error("this console kind cannot derive a key this way")]
    UnsupportedTarget,
    /// A table index would run off the end.
    #[error("the key offset {0} is out of range")]
    BadOffset(usize),
    /// The cipher refused the data.
    #[error("the cipher failed: {0}")]
    Cipher(String),
}

/// The two keys a cipher needs: the ambassador that travels, and the bright
/// that never does.
#[derive(Clone, Copy, Debug)]
pub struct RPCrypt {
    /// Which console this cipher is for.
    pub target: Target,
    /// The half the peer can see.
    pub bright: [u8; KEY_SIZE],
    /// The half that never moves.
    pub ambassador: [u8; KEY_SIZE],
}

/// Turn the ambassador into the aeropause the console sees.
///
/// This is what goes into the pairing request, and it is enough for a peer to
/// work the ambassador back out again.
pub fn aeropause(
    target: Target,
    key_1_off: usize,
    aeropause: &mut [u8; KEY_SIZE],
    ambassador: &[u8; KEY_SIZE],
) -> Result<(), RPCryptError> {
    if target == Target::Ps4_8 || target == Target::Ps4_9 {
        return Err(RPCryptError::UnsupportedTarget);
    }
    if key_1_off >= ROW {
        return Err(RPCryptError::BadOffset(key_1_off));
    }
    let keys = target.keys_1();
    let wurzelbert = if target.is_ps5() { 0xd3u8 } else { 0x29 };
    for i in 0..KEY_SIZE {
        let k = keys[i * ROW + key_1_off];
        aeropause[i] = (ambassador[i] ^ k)
            .wrapping_add(wurzelbert)
            .wrapping_add(i as u8);
    }
    Ok(())
}

/// The newer form used when a console reports a "PSN regist" (hole punch).
pub fn aeropause_psn(
    target: Target,
    key_1_off: usize,
    aeropause: &mut [u8; KEY_SIZE],
    ambassador: &[u8; KEY_SIZE],
) -> Result<(), RPCryptError> {
    if target == Target::Ps4_8 || target == Target::Ps4_9 {
        return Err(RPCryptError::UnsupportedTarget);
    }
    if key_1_off >= ROW {
        return Err(RPCryptError::BadOffset(key_1_off));
    }
    let keys = target.keys_1();
    let wurzelbert = if target.is_ps5() { 0x2b } else { 0xd7 };
    for i in 0..KEY_SIZE {
        let k = keys[i * ROW + key_1_off];
        aeropause[i] = (ambassador[i].wrapping_sub(i as u8).wrapping_add(wurzelbert)) ^ k;
    }
    Ok(())
}

/// Read the ambassador back out of a **regist** aeropause.
///
/// The client never needs this: it made the ambassador, so it still has it. A
/// console has it too, because the aeropause is what it was sent. A test fake
/// console uses this to answer like the real thing.
pub fn ambassador_from_regist_aeropause(
    target: Target,
    key_1_off: usize,
    aeropause: &[u8; KEY_SIZE],
    ambassador: &mut [u8; KEY_SIZE],
) -> Result<(), RPCryptError> {
    if target == Target::Ps4_8 || target == Target::Ps4_9 {
        return Err(RPCryptError::UnsupportedTarget);
    }
    if key_1_off >= ROW {
        return Err(RPCryptError::BadOffset(key_1_off));
    }
    let keys = target.keys_1();
    let wurzelbert = if target.is_ps5() { 0xd3 } else { 0x29 };
    for i in 0..KEY_SIZE {
        let k = keys[i * ROW + key_1_off];
        ambassador[i] = aeropause[i].wrapping_sub(wurzelbert).wrapping_sub(i as u8) ^ k;
    }
    Ok(())
}

/// Read the ambassador back out of a **PSN** aeropause (the hole-punch path).
pub fn ambassador_from_aeropause(
    target: Target,
    key_1_off: usize,
    aeropause: &[u8; KEY_SIZE],
    ambassador: &mut [u8; KEY_SIZE],
) -> Result<(), RPCryptError> {
    if target == Target::Ps4_8 || target == Target::Ps4_9 {
        return Err(RPCryptError::UnsupportedTarget);
    }
    if key_1_off >= ROW {
        return Err(RPCryptError::BadOffset(key_1_off));
    }
    let keys = target.keys_1();
    let wurzelbert = if target.is_ps5() { 0x2b } else { 0xd7 };
    for i in 0..KEY_SIZE {
        let k = keys[i * ROW + key_1_off];
        ambassador[i] = (aeropause[i] ^ k)
            .wrapping_add(i as u8)
            .wrapping_sub(wurzelbert);
    }
    Ok(())
}

/// The old PS4 form, where the PIN is the only secret.
pub fn aeropause_ps4_pre10(aeropause: &mut [u8; KEY_SIZE], ambassador: &[u8; KEY_SIZE]) {
    for i in 0..KEY_SIZE {
        aeropause[i] = ambassador[i].wrapping_sub(i as u8).wrapping_sub(0x29) ^ echo_b[i];
    }
}

/// The ambassador and bright for a session, from the nonce and morning the
/// console sends us.
pub fn bright_ambassador(
    target: Target,
    bright: &mut [u8; KEY_SIZE],
    ambassador: &mut [u8; KEY_SIZE],
    nonce: &[u8; KEY_SIZE],
    morning: &[u8; KEY_SIZE],
) -> Result<(), RPCryptError> {
    match target {
        Target::Ps4_8 | Target::Ps4_9 => {
            for i in 0..KEY_SIZE {
                ambassador[i] = nonce[i].wrapping_sub(i as u8).wrapping_sub(0x27) ^ echo_a[i];
                bright[i] =
                    morning[i].wrapping_sub(i as u8).wrapping_add(0x34) ^ echo_b[i] ^ nonce[i];
            }
            Ok(())
        }
        Target::Ps4_10 | Target::Ps5Unknown | Target::Ps5_1 => {
            let (keys_a, keys_b) = target.key_tables();
            let key = &keys_a[(usize::from(nonce[0] >> 3)) * ROW_BYTES..];
            for i in 0..KEY_SIZE {
                let mut v = nonce[i];
                if target.is_ps5() {
                    v = v.wrapping_sub(0x2d).wrapping_sub(i as u8);
                } else {
                    v = v.wrapping_add(0x36).wrapping_add(i as u8);
                }
                ambassador[i] = v ^ key[i];
            }
            let key = &keys_b[(usize::from(nonce[7] >> 3)) * ROW_BYTES..];
            for i in 0..KEY_SIZE {
                bright[i] = if target.is_ps5() {
                    morning[i].wrapping_add(0x18).wrapping_add(i as u8) ^ nonce[i] ^ key[i]
                } else {
                    (key[i] ^ morning[i])
                        .wrapping_add(0x21)
                        .wrapping_add(i as u8)
                        ^ nonce[i]
                };
            }
            Ok(())
        }
    }
}

impl RPCrypt {
    /// The cipher a session uses, once its nonce and morning arrive.
    pub fn init_auth(
        target: Target,
        nonce: &[u8; KEY_SIZE],
        morning: &[u8; KEY_SIZE],
    ) -> Result<RPCrypt, RPCryptError> {
        let mut cipher = RPCrypt {
            target,
            bright: [0; KEY_SIZE],
            ambassador: [0; KEY_SIZE],
        };
        bright_ambassador(
            target,
            &mut cipher.bright,
            &mut cipher.ambassador,
            nonce,
            morning,
        )?;
        Ok(cipher)
    }

    /// The cipher a PS4 pairing uses, where the PIN is the whole secret.
    pub fn init_regist_ps4_pre10(ambassador: &[u8; KEY_SIZE], pin: u32) -> RPCrypt {
        let mut cipher = RPCrypt {
            target: Target::Ps4_9,
            bright: regist_aes_key,
            ambassador: *ambassador,
        };
        cipher.bright[0] ^= (pin >> 0x18) as u8;
        cipher.bright[1] ^= (pin >> 0x10) as u8;
        cipher.bright[2] ^= (pin >> 0x08) as u8;
        cipher.bright[3] ^= pin as u8;
        cipher
    }

    /// The cipher a PS5 (or PS4 10+) pairing uses: a key from the table, mixed
    /// with the PIN so the wrong PIN decrypts to noise.
    pub fn init_regist(
        target: Target,
        ambassador: &[u8; KEY_SIZE],
        key_0_off: usize,
        pin: u32,
    ) -> Result<RPCrypt, RPCryptError> {
        if target == Target::Ps4_8 || target == Target::Ps4_9 {
            return Err(RPCryptError::UnsupportedTarget);
        }
        if key_0_off >= ROW {
            return Err(RPCryptError::BadOffset(key_0_off));
        }
        let keys = target.keys_0();
        let mut cipher = RPCrypt {
            target,
            bright: [0; KEY_SIZE],
            ambassador: *ambassador,
        };
        for i in 0..KEY_SIZE {
            cipher.bright[i] = keys[i * ROW + key_0_off];
        }
        cipher.bright[0x0c] ^= (pin >> 0x18) as u8;
        cipher.bright[0x0d] ^= (pin >> 0x10) as u8;
        cipher.bright[0x0e] ^= (pin >> 0x08) as u8;
        cipher.bright[0x0f] ^= pin as u8;
        Ok(cipher)
    }

    /// The IV for a block of traffic: HMAC-SHA256 of the ambassador and the
    /// counter, cut to 16 bytes.
    pub fn generate_iv(&self, counter: u64) -> Result<[u8; KEY_SIZE], RPCryptError> {
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(self.target.hmac_key())
            .map_err(|error| RPCryptError::Cipher(error.to_string()))?;
        mac.update(&self.ambassador);
        mac.update(&counter.to_be_bytes());
        let digest = mac.finalize().into_bytes();
        let mut iv = [0u8; KEY_SIZE];
        iv.copy_from_slice(&digest[..KEY_SIZE]);
        Ok(iv)
    }

    /// Encrypt a stream. `input` and `out` must be the same length and may be
    /// the same buffer.
    pub fn encrypt(&self, counter: u64, input: &[u8], out: &mut [u8]) -> Result<(), RPCryptError> {
        self.crypt(counter, input, out, true)
    }

    /// Decrypt a stream.
    pub fn decrypt(&self, counter: u64, input: &[u8], out: &mut [u8]) -> Result<(), RPCryptError> {
        self.crypt(counter, input, out, false)
    }

    /// AES-128-CFB128, no padding: XOR the input with the cipher's output and
    /// feed the ciphertext back in as the next block's input.
    fn crypt(
        &self,
        counter: u64,
        input: &[u8],
        out: &mut [u8],
        encrypt: bool,
    ) -> Result<(), RPCryptError> {
        if input.len() != out.len() {
            return Err(RPCryptError::Cipher(format!(
                "the input is {} bytes but the output is {}",
                input.len(),
                out.len()
            )));
        }
        let cipher = aes::Aes128::new_from_slice(&self.bright)
            .map_err(|error| RPCryptError::Cipher(error.to_string()))?;
        let mut iv = self.generate_iv(counter)?;

        let mut taken = 0;
        while taken < input.len() {
            let block = (input.len() - taken).min(KEY_SIZE);
            let mut keystream = GenericArray::clone_from_slice(&iv);
            cipher.encrypt_block(&mut keystream);
            let keystream = keystream.as_slice();
            for i in 0..block {
                out[taken + i] = input[taken + i] ^ keystream[i];
            }
            // Next block's input is this block's output, shifted in. When the
            // block is short there is nothing after it, so the shift is a copy.
            let next = if encrypt {
                out[taken..taken + block].to_vec()
            } else {
                input[taken..taken + block].to_vec()
            };
            iv.copy_within(block.., 0);
            iv[KEY_SIZE - block..].copy_from_slice(&next);
            taken += block;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 16-byte key of zeroes, for the table size test.
    fn key_zero() -> [u8; KEY_SIZE] {
        [0u8; KEY_SIZE]
    }

    #[test]
    fn the_regist_aeropause_round_trips() {
        let target = Target::Ps5_1;
        let ambassador = [
            0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee,
            0xff, 0x00,
        ];
        let mut scrambled = [0u8; KEY_SIZE];
        aeropause(target, 8, &mut scrambled, &ambassador).unwrap();
        assert_ne!(scrambled, ambassador, "the aeropause hides the ambassador");

        let mut back = [0u8; KEY_SIZE];
        ambassador_from_regist_aeropause(target, 8, &scrambled, &mut back).unwrap();
        assert_eq!(back, ambassador, "a console reads the ambassador back out");

        // A different offset reads as different bytes, as a peer with the wrong
        // offset would.
        let mut wrong = [0u8; KEY_SIZE];
        ambassador_from_regist_aeropause(target, 9, &scrambled, &mut wrong).unwrap();
        assert_ne!(wrong, ambassador);
    }

    #[test]
    fn the_psn_aeropause_round_trips_too() {
        // chiaki's own pair, for the hole-punch path.
        let target = Target::Ps5_1;
        let ambassador = [0x9a; KEY_SIZE];
        let mut scrambled = [0u8; KEY_SIZE];
        aeropause_psn(target, 4, &mut scrambled, &ambassador).unwrap();
        let mut back = [0u8; KEY_SIZE];
        ambassador_from_aeropause(target, 4, &scrambled, &mut back).unwrap();
        assert_eq!(back, ambassador);
    }

    #[test]
    fn the_same_pin_cipher_matches_and_the_wrong_pin_does_not() {
        let ambassador = [7u8; KEY_SIZE];
        let sealed = b"the quick brown fox".to_vec();
        let opened = {
            let cipher = RPCrypt::init_regist(Target::Ps5_1, &ambassador, 1, 1234).unwrap();
            let mut out = sealed.clone();
            cipher.encrypt(0, &sealed, &mut out).unwrap();
            assert_ne!(out, sealed);
            out
        };
        let cipher = RPCrypt::init_regist(Target::Ps5_1, &ambassador, 1, 1234).unwrap();
        let mut back = vec![0u8; opened.len()];
        cipher.decrypt(0, &opened, &mut back).unwrap();
        assert_eq!(back, sealed);

        let wrong = RPCrypt::init_regist(Target::Ps5_1, &ambassador, 1, 9999).unwrap();
        let mut wrong_out = vec![0u8; opened.len()];
        wrong.decrypt(0, &opened, &mut wrong_out).unwrap();
        assert_ne!(wrong_out, sealed, "the wrong PIN is noise");
    }

    #[test]
    fn a_stream_is_encrypted_all_the_way_through() {
        let cipher = RPCrypt::init_regist(Target::Ps5_1, &[0x5a; KEY_SIZE], 1, 4321).unwrap();
        // A length that is not a whole number of blocks, as a regist payload is.
        let input: Vec<u8> = (0..0x1f0u16).map(|i| (i % 251) as u8).collect();
        let mut out = vec![0u8; input.len()];
        cipher.encrypt(0, &input, &mut out).unwrap();
        assert_ne!(out, input);

        let mut back = vec![0u8; input.len()];
        cipher.decrypt(0, &out, &mut back).unwrap();
        assert_eq!(back, input);
    }

    #[test]
    fn each_block_is_chained_like_cfb() {
        // In CFB, changing one byte changes the byte it sits in and every byte
        // after it, which is what the console's reply relies on.
        let cipher = RPCrypt::init_regist(Target::Ps5_1, &[0x33; KEY_SIZE], 1, 1).unwrap();
        let input = [0u8; 64];
        let mut first = vec![0u8; 64];
        cipher.encrypt(0, &input, &mut first).unwrap();
        let mut changed = input;
        changed[20] = 1;
        let mut second = vec![0u8; 64];
        cipher.encrypt(0, &changed, &mut second).unwrap();
        assert_eq!(first[..20], second[..20], "earlier blocks are untouched");
        assert_ne!(first[20..], second[20..], "later blocks follow the change");
    }

    #[test]
    fn a_bad_offset_is_refused_before_it_can_read_past_the_table() {
        assert!(RPCrypt::init_regist(Target::Ps5_1, &[0; KEY_SIZE], 0x20, 1).is_err());
        assert!(aeropause(Target::Ps5_1, 0x20, &mut [0; KEY_SIZE], &[0; KEY_SIZE]).is_err());
        // And a PS4 8 console, which uses the older path.
        assert!(RPCrypt::init_regist(Target::Ps4_8, &[0; KEY_SIZE], 1, 1).is_err());
    }

    #[test]
    fn the_tables_are_the_right_size() {
        assert_eq!(keys_a_ps5.len(), 0x70 * ROW);
        assert_eq!(keys_a_ps4.len(), 0x70 * ROW);
        assert_eq!(keys_b_ps5.len(), 0x70 * ROW);
        assert_eq!(keys_b_ps4.len(), 0x70 * ROW);
        assert_eq!(ps5_keys_1.len(), 0x10 * ROW);
        assert_eq!(ps4_keys_0.len(), 0x10 * ROW);
        assert_eq!(hmac_key_ps5.len(), KEY_SIZE);
        assert_eq!(key_zero(), [0u8; KEY_SIZE]);
    }
}
