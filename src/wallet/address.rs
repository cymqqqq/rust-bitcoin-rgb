// Rust Bitcoin Library
// Written in 2014 by
//   Andrew Poelstra <apoelstra@wpsoftware.net>
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the CC0 Public Domain Dedication
// along with this software.
// If not, see <http://creativecommons.org/publicdomain/zero/1.0/>.
//

//! # Addresses
//!
//! Support for ordinary base58 Bitcoin addresses
//!

use secp256k1::key::PublicKey;
use crypto::digest::Digest;
use crypto::sha2::Sha256;
use std::ops;

use blockdata::script::Script;
use blockdata::opcodes;
use network::constants::Network;
use util::hash::Ripemd160Hash;
use util::base58::{self, FromBase58, ToBase58};

#[derive(Clone, PartialEq, Eq)]
/// A Bitcoin address
pub struct Address {
  /// The network on which this address is usable
  pub network: Network,
  /// The pubkeyhash that this address encodes
  pub hash: Ripemd160Hash
}

impl Address {
  /// Creates an address from a public key
  #[inline]
  pub fn from_key(network: Network, pk: &PublicKey) -> Address {
    let mut sha = Sha256::new();
    let mut out = [0;32];
    sha.input(&pk[..]);
    sha.result(&mut out);
    Address {
      network: network,
      hash: Ripemd160Hash::from_data(&out)
    }
  }

  /// Generates a script pubkey spending to this address
  #[inline]
  pub fn script_pubkey(&self) -> Script {
    let mut script = Script::new();
    script.push_opcode(opcodes::All::OP_DUP);
    script.push_opcode(opcodes::All::OP_HASH160);
    script.push_slice(&self.hash[..]);
    script.push_opcode(opcodes::All::OP_EQUALVERIFY);
    script.push_opcode(opcodes::All::OP_CHECKSIG);
    script
  }
}

impl ops::Index<usize> for Address {
  type Output = u8;
  #[inline]
  fn index(&self, index: usize) -> &u8 {
    &self.hash[index]
  }
}

impl ops::Index<ops::Range<usize>> for Address {
  type Output = [u8];
  #[inline]
  fn index(&self, index: ops::Range<usize>) -> &[u8] {
    &self.hash[index]
  }
}

impl ops::Index<ops::RangeTo<usize>> for Address {
  type Output = [u8];
  #[inline]
  fn index(&self, index: ops::RangeTo<usize>) -> &[u8] {
    &self.hash[index]
  }
}

impl ops::Index<ops::RangeFrom<usize>> for Address {
  type Output = [u8];
  #[inline]
  fn index(&self, index: ops::RangeFrom<usize>) -> &[u8] {
    &self.hash[index]
  }
}

impl ops::Index<ops::RangeFull> for Address {
  type Output = [u8];
  #[inline]
  fn index(&self, _: ops::RangeFull) -> &[u8] {
    &self.hash[..]
  }
}

/// Conversion from other types into an address
pub trait ToAddress {
  /// Copies `self` into a new `Address`
  fn to_address(&self, network: Network) -> Address;
}

impl<'a> ToAddress for &'a [u8] {
  #[inline]
  fn to_address(&self, network: Network) -> Address {
    Address {
      network: network,
      hash: Ripemd160Hash::from_slice(*self)
    }
  }
}

impl ToBase58 for Address {
  fn base58_layout(&self) -> Vec<u8> {
    let mut ret = vec![
      match self.network {
        Network::Bitcoin => 0,
        Network::Testnet => 111
      }
    ];
    ret.push_all(&self.hash[..]);
    ret
  }
}

impl FromBase58 for Address {
  fn from_base58_layout(data: Vec<u8>) -> Result<Address, base58::Error> {
    if data.len() != 21 {
      return Err(base58::Error::InvalidLength(data.len()));
    }

    Ok(Address {
      network: match data[0] {
        0   => Network::Bitcoin,
        111 => Network::Testnet,
        x   => { return Err(base58::Error::InvalidVersion(vec![x])); }
      },
      hash: Ripemd160Hash::from_slice(&data[1..])
    })
  }
}

impl ::std::fmt::Debug for Address {
  fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
    write!(f, "{}", self.to_base58check())
  }
}

#[cfg(test)]
mod tests {
  use serialize::hex::FromHex;
  use test::{Bencher, black_box};

  use secp256k1::Secp256k1;

  use network::constants::Network::Bitcoin;
  use util::hash::Ripemd160Hash;
  use util::base58::{FromBase58, ToBase58};
  use super::Address;

  #[test]
  fn test_address_58() {
    let addr = Address {
      network: Bitcoin,
      hash: Ripemd160Hash::from_slice(&"162c5ea71c0b23f5b9022ef047c4a86470a5b070".from_hex().unwrap())
    };

    assert_eq!(&addr.to_base58check(), "132F25rTsvBdp9JzLLBHP5mvGY66i1xdiM");
    assert_eq!(FromBase58::from_base58check("132F25rTsvBdp9JzLLBHP5mvGY66i1xdiM"), Ok(addr));
  }

  #[bench]
  pub fn generate_address(bh: &mut Bencher) {
    let mut s = Secp256k1::new().unwrap();
    bh.iter( || {
      let (sk, pk) = s.generate_keypair(true);
      black_box(sk);
      black_box(pk);
      let addr = Address::from_key(Bitcoin, &pk);
      black_box(addr);
    });
  }

  #[bench]
  pub fn generate_uncompressed_address(bh: &mut Bencher) {
    let mut s = Secp256k1::new().unwrap();
    bh.iter( || {
      let (sk, pk) = s.generate_keypair(false);
      black_box(sk);
      black_box(pk);
      let addr = Address::from_key(Bitcoin, &pk);
      black_box(addr);
    });
  }

  #[bench]
  pub fn generate_sequential_address(bh: &mut Bencher) {
    let mut s = Secp256k1::new().unwrap();
    let (sk, _) = s.generate_keypair(true);
    let mut iter = sk.sequence(true);
    bh.iter( || {
      let (sk, pk) = iter.next().unwrap();
      black_box(sk);
      let addr = Address::from_key(Bitcoin, &pk);
      black_box(addr);
    });
  }
}

