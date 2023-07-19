use codec::{Decode, Encode, Input};
use frame_support::PalletError;
use sp_runtime::{traits::Header as HeaderT, RuntimeAppPublic, RuntimeDebug};
use sp_std::{vec, vec::Vec};

use crate::{AuthoritySet, AuthoritySignature};
use scale_info::TypeInfo;

#[derive(TypeInfo, PartialEq, Eq, Clone, Debug, Decode, Encode)]
pub struct Signature(AuthoritySignature);

// This could be pulled from aleph_bft_cryto, but we need to implement TypeInfo for it.
// TODO: add TypeInfo to aleph_bft_crypto::NodeMap and reuse it here.
#[derive(TypeInfo, Clone, Eq, PartialEq, Debug, Default, Decode, Encode)]
pub struct SignatureSet(Vec<Option<Signature>>);

impl SignatureSet {
	/// Constructs a new node map with a given length.
	pub fn with_size(len: usize) -> Self {
		let v = vec![None; len];
		SignatureSet(v)
	}

	pub fn size(&self) -> usize {
		self.0.len()
	}

	pub fn iter(&self) -> impl Iterator<Item = (usize, &Signature)> {
		self.0
			.iter()
			.enumerate()
			.filter_map(|(idx, maybe_value)| Some((idx, maybe_value.as_ref()?)))
	}

	pub fn get(&self, node_id: usize) -> Option<&Signature> {
		self.0[node_id].as_ref()
	}

	pub fn insert(&mut self, node_id: usize, value: Signature) {
		self.0[node_id] = Some(value)
	}
}

/// A proof of block finality, currently in the form of a sufficiently long list of signatures or a
/// sudo signature of a block for emergency finalization.
#[derive(TypeInfo, Clone, Encode, Decode, Debug, PartialEq, Eq)]
pub enum AlephJustification {
	CommitteeMultisignature(SignatureSet),
	EmergencySignature(AuthoritySignature),
}

#[derive(Eq, Encode, Decode, PartialEq, Debug, Copy, Clone)]
pub struct Version(pub u16);

/// Actual on-chain justification format.
#[derive(Clone, Encode, Debug, PartialEq, Eq)]
pub struct VersionedAlephJustification {
	version: Version,
	num_bytes: u16,
	justification: AlephJustification,
}

impl Decode for VersionedAlephJustification {
	fn decode<I: Input>(input: &mut I) -> Result<Self, codec::Error> {
		let version = Version::decode(input)?;
		let num_bytes = u16::decode(input)?;
		let justification = match version {
			Version(3) => AlephJustification::decode(input)?,
			_ => return Err(codec::Error::from("Unsupported justification version")),
		};
		Ok(VersionedAlephJustification { version, num_bytes, justification })
	}
}

impl From<VersionedAlephJustification> for AlephJustification {
	fn from(justification: VersionedAlephJustification) -> Self {
		justification.justification
	}
}

#[derive(RuntimeDebug, Clone, Encode, PartialEq, Eq)]
pub struct AlephFullJustification<Header: HeaderT> {
	header_number: Header::Number,
	justification: AlephJustification,
}

impl<Header: HeaderT> Decode for AlephFullJustification<Header> {
	fn decode<I: Input>(input: &mut I) -> Result<Self, codec::Error> {
		let versioned_justification = VersionedAlephJustification::decode(input)?;
		Ok(AlephFullJustification {
			header_number: 0u32.into(),
			justification: versioned_justification.justification,
		})
	}
}

impl<Header: HeaderT> AlephFullJustification<Header> {
	pub fn justification(&self) -> &AlephJustification {
		&self.justification
	}
}

impl<Header: HeaderT> From<AlephFullJustification<Header>> for AlephJustification {
	fn from(value: AlephFullJustification<Header>) -> Self {
		value.justification
	}
}

impl<Header: HeaderT> bp_header_chain::FinalityProof<Header::Number>
	for AlephFullJustification<Header>
{
	fn target_header_number(&self) -> Header::Number {
		self.header_number
	}
}

#[derive(Eq, RuntimeDebug, PartialEq, Encode, Decode, TypeInfo, PalletError)]
pub enum Error {
	JustificationNotDecodable,
	NotEnoughCorrectSignatures,
	InvalidIndex,
	EmergencyFinalizerUsed,
}

pub fn verify_justification<Header: HeaderT>(
	authority_set: &AuthoritySet,
	header: &Header,
	justification: &AlephJustification,
) -> Result<(), Error> {
	match justification {
		AlephJustification::CommitteeMultisignature(signature_set) => {
			let mut signature_count = 0;

			for (index, signature) in signature_set.iter() {
				let authority = authority_set.get(index).ok_or(Error::InvalidIndex)?;
				if authority.verify(&header.hash().encode(), &signature.0) {
					signature_count += 1;
				}
			}

			if signature_count < 2 * authority_set.len() / 3 + 1 {
				return Err(Error::NotEnoughCorrectSignatures)
			}

			Ok(())
		},
		AlephJustification::EmergencySignature(_) => Err(Error::EmergencyFinalizerUsed),
	}
}

#[cfg(feature = "std")]
pub mod test_utils {
	use super::*;
	use crate::AuthorityId;
	use hex::FromHex;
	use sp_application_crypto::Pair;
	use sp_runtime::{testing::Header, ConsensusEngineId};

	pub type Seed = [u8; 32];
	pub type Seeds = Vec<Seed>;

	pub fn generate_seeds(size: usize) -> Seeds {
		let mut seed = [0u8; 32];
		let mut seeds = Vec::new();
		for i in 0..size {
			seed[i] = 1;
			seeds.push(seed);
		}
		seeds
	}

	pub fn authority_id_from_seed(s: &Seed) -> AuthorityId {
		pair_from_seed(s).public()
	}

	pub fn pair_from_seed(s: &Seed) -> crate::app::Pair {
		crate::app::Pair::from_seed(s)
	}

	pub fn generate_authority_set(seeds: Seeds) -> AuthoritySet {
		let mut authority_set = AuthoritySet::new();
		for authority in seeds.iter() {
			authority_set.push(authority_id_from_seed(authority));
		}
		authority_set
	}

	pub fn generate_signature_set(header: &Header, seeds: &Seeds) -> SignatureSet {
		let mut signatures = Vec::new();
		for authority in seeds.iter() {
			let aleph_authority = pair_from_seed(authority);
			let signature = aleph_authority.sign(&header.hash().encode());
			signatures.push(Some(Signature(signature)));
		}

		SignatureSet(signatures)
	}

	pub fn generate_justification(header: &Header, seeds: &Seeds) -> AlephJustification {
		let signature_set = generate_signature_set(header, seeds);
		AlephJustification::CommitteeMultisignature(signature_set)
	}

	pub fn decode_from_hex<D: Decode>(data: &str) -> D {
		Decode::decode(&mut &Vec::from_hex(data).unwrap()[..]).unwrap()
	}

	pub const AURA_ENGINE_ID: ConsensusEngineId = [b'a', b'u', b'r', b'a'];

	pub fn raw_authorities_into_authority_set(raw_authorities: &[&str]) -> AuthoritySet {
		let mut authorities = Vec::new();
		println!("raw_authorities: {:?}", raw_authorities);
		for raw_authority in raw_authorities {
			authorities.push(decode_from_hex(raw_authority));
		}
		authorities
	}

	pub fn aleph_justification_from_hex(hex: &str) -> AlephJustification {
		let encoded_justification: Vec<u8> = FromHex::from_hex(hex).unwrap();
		let versioned_justification =
			VersionedAlephJustification::decode(&mut encoded_justification.as_slice()).unwrap();
		versioned_justification.into()
	}
}

#[cfg(test)]
pub mod tests {
	use super::{test_utils::*, *};
	use bp_test_utils::test_header;
	use hex::FromHex;
	use sp_runtime::{testing::Header, Digest, DigestItem};

	#[test]
	fn accepts_valid_justification() {
		let header = test_header(5);
		let justification = generate_justification(&header, &generate_seeds(4));
		let authority_set = generate_authority_set(generate_seeds(4));

		assert!(verify_justification(&authority_set, &header, &justification).is_ok());
	}

	#[test]
	fn rejects_justification_with_too_little_signatures() {
		let header = test_header(5);
		let justification = generate_justification(&header, &generate_seeds(4));
		let authority_set = generate_authority_set(generate_seeds(6));

		assert!(verify_justification(&authority_set, &header, &justification).is_err());
	}

	const DEVNET_JUSTIFICATION: &str = "0300c60000100182e110ef61d591076351794f8d0927ddf0ee3aa4fcf0de6d3b4a07c9c0ce836a7d2efb1e7aef1ff645a1337a2b3eebccf80c78feba49e1f11997c636a6999f0d0001ecb430ade18767790b85280a134ae73dce133ee614f167c3f88f9fb3cf6a3c583f8e604410f3750bb86292167e86f06fd687fc1eb840c47cc27379a8e752d30e017b7e8504257b9e55dfb5f06a9e0a22d1b94aa8da0f202fc685ef6089d8a9286d011e687c8db69e7162b4b07ed55b35f837f22f93aacdb4e54dafe2d2dbde8509";
	const DEVNET_AUTHORITIES: [&str; 4] = [
		"bc3faef89f7b46c69088f4b3be1545a2dbb132a22bbfa9778625a924b0c38320",
		"7e2dc59e0fcf3ece8a37a8672952fbccb4b73403069901fb5804ac2e36a90d5a",
		"c2e071ddd5c993e3498ccd5e38a5a77b4d94848d6b066bc9391ac4e3be84ce5c",
		"7e0a8df85cf67e1e83eacb7690245872a571ce5d37bd5cb761ec98da4f3238d4",
	];

	// Block 51730103
	const MAINNET_JUSTIFICATION: &str = "03009002003801752af5d2d57c25657cf083014c65eb39d18a4eeba5ca4a1ffeee378df14298ef449f9b076a3f5918de1e774410a52fb38f0fd5e70ba44dfea1453647388b94060112cb082077986f0e79bb12a69d9fa483f99f8a552e5124623a4e8a14ff324f9ba33d9b0cc11d9c773c8ee6d830a334c012f3c9b1036f44ba08f0fe5e7b1e540f01c5906221eec48d57d992573cebc1e6861f1e7e572d1d0d47328dd417361cb53d71864407246a6c0f575e67471e6692a3066d1a5787817645b9f1e5dc57fb2a0d000158b8d87262541a2f73d6c311657f74234203a3650ac442acf99bf4e1600486bb1e5849c6ac204c58c1120189c8e3c76918c564ec2a0468365751f15591580f0d0114e28c22ff3cee483567ae2fc0e4aab6386e5cba24eb3c2866b0e2245ad1a5bb70a0714e807e0b903ef58463792595a6d6a4e5fe58c3c0ad118eb031a32c7b060001c10be8f8665c804552dd3af729724422841fb47f8dfbb5487935ca86bdee3489bc03ea4c47c022462c19bb668c913392192e27f0bcc8538edf61d076c772b80600019ebf0579cd074ce2385326ad8dfadb720e14365c9cb588d56776ce9fbf73d3a4e30671fec68c6a1cee052d19ef5e018e53a44222063379f754e8aa80ea54cd05012f4b3f8a0845134fa599aea859a9c09c020712f31bf24a6e98587bf44d89f3d854df2a96c6af8ddc81885dd7c0d26f64a69f9b0da418025a6b42d955d97dcd0301e4094b019782509277dbe7969b1cb9ae22f65b7c33c32542b22ffe0d179a7661ffa3743747cf19870a1491406adb8438d11c70e5732655a340344a1c0686d20701f42cb05c7793f8055bb8c0d8e872426f2293a2cc54898548a0d71b2906caa2c4c5d5af502331c9b7c5d7e5f587414a6b5a00f92cff843d44eebb81d84a592e0400";
	const MAINNET_AUTHORITIES: [&str; 14] = [
		"3e1f808aba722e8c2b605186d525330fc0602448aa1a0e4095db3e691e82aa53",
		"4e9800a7078b680ba72b0f077d6803c3d35ccdcba15b618542801a5cdf5c949b",
		"55f42ea5496bd242dc59f3a299ebc4f3ad6ea1b47b745abdda2b3a6129cf0afe",
		"8f725daf2944ef12357e298058c317a409d313d5379fee88abddd2967b0c02da",
		"37371d98a2d7d88096414e55e861e06a07ba7645c49d446cc5e414bb3c00d456",
		"749e7ca3ff95b3c92608500525f33071cd2ce4e592ba9535a6eb7bac1b667faa",
		"c86669783d7c1c1516dd7bd70dcc893232c9a73b4e1095557ef3a929d324904a",
		"bc16685f99c1ebfbaeb0bb1b1f6b81f078977cfb5418c4e125b8a989817d9fcf",
		"8e6efcceba526685383b74cc53eb09f8fe2e6bcb9988e2bc618ead55251f7e97",
		"def643f576925be43e4e6f139c6d6787b3da8f637a3d29fef01ccaf24d82259e",
		"fb35011223beff7e82a7932bfb9f5ad0a774c9812189b0bb8aec3389b515eafc",
		"10ae993223432b051b8ecf175c2ab0ef72186b540d8abd402b0568329a5d3b03",
		"0c5b46f7d638edf0e76e85d451096f6bf361a476b43d61f177d8cf0253588cb1",
		"f4ad895cce6857c6f8b955d55907969ee6a8f177187921e771cee52bd7b2b800",
	];

	#[test]
	fn devnet_justification_decodes() {
		let encoded_justification: Vec<u8> = FromHex::from_hex(DEVNET_JUSTIFICATION).unwrap();
		assert!(VersionedAlephJustification::decode(&mut encoded_justification.as_slice()).is_ok());
	}

	#[test]
	fn mainnet_justification_decodes() {
		let encoded_justification: Vec<u8> = FromHex::from_hex(MAINNET_JUSTIFICATION).unwrap();
		assert!(VersionedAlephJustification::decode(&mut encoded_justification.as_slice()).is_ok());
	}

	#[test]
	fn devnet_justification_is_valid() {
		let authority_set = raw_authorities_into_authority_set(&DEVNET_AUTHORITIES);
		let justification = aleph_justification_from_hex(DEVNET_JUSTIFICATION);
		let header = Header::new(
			49,
			decode_from_hex("bf45a153b83c7981aede86e12cd072a1fde518dda899b7f9a5b222eaa432b9a0"),
			decode_from_hex("96ea6b88d102208f8072513b6015ff32a4070de19c5988e040d7710e5919ce64"),
			decode_from_hex("5bc9cb5341a1ae9a282208f19d68333f18c29b7096eecdf4983b78de642266d7"), 
			Digest {
				logs: vec![
				DigestItem::PreRuntime(AURA_ENGINE_ID, FromHex::from_hex("beeda36400000000").unwrap()), 
				DigestItem::Seal(AURA_ENGINE_ID, FromHex::from_hex("746d6432f2d81c9710b70bcefda2c6e584ab67f09d59c17e69fce3d07c74d80589032511258587f1107eb3ac90b6166b1ecac7d558739a07a70e864aef1c8488").unwrap())
			]}
		);

		assert!(verify_justification(&authority_set, &header, &justification).is_ok());
	}

	#[test]
	fn mainnet_justification_is_valid() {
		let authority_set = raw_authorities_into_authority_set(&MAINNET_AUTHORITIES);
		let justification = aleph_justification_from_hex(MAINNET_JUSTIFICATION);
		let header = Header::new(
			51730103,
			decode_from_hex("78d422f744c6207f40ceb5504021803a7551c03e74b3e2c847df2a052b565942"),
			decode_from_hex("ffa33bb51a943ebd44426569cbfc901ca1e37a8f06652cf5cff20473cd980690"),
			decode_from_hex("de7ce2d09b3d3b6decfdf9a1e8b0582d413642d8edd4d326a5677f6222027028"), 
			Digest {
				logs: vec![
				DigestItem::PreRuntime(AURA_ENGINE_ID, FromHex::from_hex("b1b8a26400000000").unwrap()), 
				DigestItem::Seal(AURA_ENGINE_ID, FromHex::from_hex("220b93bbbb5af5352c2da639536b2c0ac33e6cfe4d765846f33838344e24fb54373092c0b16671414d431828ddb771552e17a89a67928e88e5ac343a0021548e").unwrap())
			]}
		);

		assert!(verify_justification(&authority_set, &header, &justification).is_ok());
	}
}
