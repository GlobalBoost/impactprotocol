use parity_scale_codec::{Decode, Encode};
use sc_consensus_pow::{Error, PowAlgorithm};
use impact_primitives::{AlgorithmApi, Difficulty};
use sp_api::ProvideRuntimeApi;
use sp_consensus_pow::{DifficultyApi, Seal as RawSeal};
use sp_core::{H256, U256};
use sp_runtime::generic::BlockId;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;

use rust_yescrypt::yescrypt;

pub mod app {
	use sp_application_crypto::{app_crypto, sr25519};
	use sp_core::crypto::KeyTypeId;

	pub const ID: KeyTypeId = KeyTypeId(*b"klp2");
	app_crypto!(sr25519, ID);
}

/// Determine whether the given hash satisfies the given difficulty.
/// The test is done by multiplying the two together. If the product
/// overflows the bounds of U256, then the product (and thus the hash)
/// was too high.
pub fn hash_meets_difficulty(hash: &H256, difficulty: U256) -> bool {
	let num_hash = U256::from(&hash[..]);
	let (_, overflowed) = num_hash.overflowing_mul(difficulty);

	!overflowed
}

/// A Seal struct that will be encoded to a Vec<u8> as used as the
/// `RawSeal` type.
#[derive(Clone, PartialEq, Eq, Encode, Decode, Debug)]
pub struct Seal {
	pub difficulty: U256,
	pub work: H256,
	pub nonce: U256,
}

/// A not-yet-computed attempt to solve the proof of work. Calling the
/// compute method will compute the hash and return the seal.
#[derive(Clone, PartialEq, Eq, Encode, Decode, Debug)]
pub struct Compute {
	pub difficulty: U256,
	pub pre_hash: H256,
	pub nonce: U256,
}

impl Compute {
	pub fn compute(self) -> Seal {
		
		let mut buf = [0u8; 32];
		yescrypt(&self.encode()[..], &mut buf);
		let work = H256::from_slice(&buf);
		Seal {
			nonce: self.nonce,
			difficulty: self.difficulty,
			work,
		}
	}
}

/// A minimal PoW algorithm that uses Yescrypt hashing.
/// Difficulty is fixed at 1_000_000
#[derive(Clone)]
pub struct MinimalYescryptAlgorithm;

// Here we implement the general PowAlgorithm trait for our concrete Yescrypt Algorithm
impl<B: BlockT<Hash = H256>> PowAlgorithm<B> for MinimalYescryptAlgorithm {
	type Difficulty = U256;

	fn difficulty(&self, _parent: B::Hash) -> Result<Self::Difficulty, Error<B>> {
		// Fixed difficulty hardcoded here
		Ok(U256::from(1_000))
	}

	fn verify(
		&self,
		_parent: &BlockId<B>,
		pre_hash: &H256,
		_pre_digest: Option<&[u8]>,
		seal: &RawSeal,
		difficulty: Self::Difficulty,
	) -> Result<bool, Error<B>> {
		// Try to construct a seal object by decoding the raw seal given
		let seal = match Seal::decode(&mut &seal[..]) {
			Ok(seal) => seal,
			Err(_) => return Ok(false),
		};

		// See whether the hash meets the difficulty requirement. If not, fail fast.
		if !hash_meets_difficulty(&seal.work, difficulty) {
			return Ok(false);
		}

		// Make sure the provided work actually comes from the correct pre_hash
		let compute = Compute {
			difficulty,
			pre_hash: *pre_hash,
			nonce: seal.nonce,
		};

		if compute.compute() != seal {
			return Ok(false);
		}

		Ok(true)
	}
}

/// A complete PoW Algorithm that uses Yescrypt hashing.
/// Needs a reference to the client so it can grab the difficulty from the runtime.
pub struct YescryptAlgorithm<C> {
	client: Arc<C>,
}

impl<C> YescryptAlgorithm<C> {
	pub fn new(client: Arc<C>) -> Self {
		Self { client }
	}
}

// Manually implement clone. Deriving doesn't work because
// it'll derive impl<C: Clone> Clone for YescryptAlgorithm<C>. But C in practice isn't Clone.
impl<C> Clone for YescryptAlgorithm<C> {
	fn clone(&self) -> Self {
		Self{			
			client: self.client.clone(),
		}
	}
}

// Here we implement the general PowAlgorithm trait for our concrete YescryptAlgorithm
impl<B: BlockT<Hash = H256>, C> PowAlgorithm<B> for YescryptAlgorithm<C>
where
	C: ProvideRuntimeApi<B>,
	C::Api: DifficultyApi<B, Difficulty> + AlgorithmApi<B>,
{
	type Difficulty = U256;

	fn difficulty(&self, parent: B::Hash) -> Result<Self::Difficulty, Error<B>> {
		self.client
			.runtime_api()
			.difficulty(parent)
			.map_err(|err| {
				sc_consensus_pow::Error::Environment(format!(
					"Fetching difficulty from runtime failed: {:?}",
					err
				))
			})
	}

	fn verify(
		&self,
		_parent: &BlockId<B>,
		pre_hash: &H256,
		_pre_digest: Option<&[u8]>,
		seal: &RawSeal,
		difficulty: Self::Difficulty,
	) -> Result<bool, Error<B>> {
		// Try to construct a seal object by decoding the raw seal given
		let seal = match Seal::decode(&mut &seal[..]) {
			Ok(seal) => seal,
			Err(_) => return Ok(false),
		};

		// See whether the hash meets the difficulty requirement. If not, fail fast.
		if !hash_meets_difficulty(&seal.work, difficulty) {
			return Ok(false);
		}

		// Make sure the provided work actually comes from the correct pre_hash
		let compute = Compute {
			difficulty,
			pre_hash: *pre_hash,
			nonce: seal.nonce,
		};

		if compute.compute() != seal {
			return Ok(false);
		}

		Ok(true)
	}
}