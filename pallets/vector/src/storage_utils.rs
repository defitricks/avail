use codec::{Decode, Encode, MaxEncodedLen};
use patricia_merkle_trie::{keccak256, EIP1186Layout, StorageProof};
use primitive_types::{H160, H256};
use rlp::Rlp;
use scale_info::TypeInfo;
use sp_io::hashing::keccak_256 as keccak256;
use sp_std::vec::Vec;
use trie_db::{Trie, TrieDBBuilder};

#[derive(Clone, Copy, Default, Encode, Decode, Debug, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
pub enum MessageStatusEnum {
	#[default]
	NotExecuted,
	ExecutionSucceeded,
}

#[derive(Debug, PartialEq)]
pub enum StorageError {
	StorageValueError,
	AccountNotFound,
	CannotDecodeItems,
}

/// get_storage_value returns a storage value based on the proof that is provided.
pub fn get_storage_value(
	slot_hash: H256,
	storage_root: H256,
	proof: Vec<Vec<u8>>,
) -> Result<H256, StorageError> {
	let key = keccak256(slot_hash.as_bytes());
	let db = StorageProof::new(proof).into_memory_db::<keccak256::KeccakHasher>();
	let trie =
		TrieDBBuilder::<EIP1186Layout<keccak256::KeccakHasher>>::new(&db, &storage_root).build();

	let Ok(Some(trie_value)) = trie.get(&key) else {
		return Err(StorageError::StorageValueError);
	};

	let Ok(rlp_storage_value) = Rlp::new(trie_value.as_slice()).data() else {
		return Err(StorageError::CannotDecodeItems);
	};

	if rlp_storage_value.is_empty() {
		return Err(StorageError::CannotDecodeItems);
	}

	let storage_value = rlp_to_h256(rlp_storage_value)?;

	Ok(storage_value)
}

/// get_storage_root returns storage root based on the provided proof.
pub fn get_storage_root(
	proof: Vec<Vec<u8>>,
	address: H160,
	state_root: H256,
) -> Result<H256, StorageError> {
	let key = keccak256(address.as_bytes());
	let db = StorageProof::new(proof).into_memory_db::<keccak256::KeccakHasher>();
	let trie =
		TrieDBBuilder::<EIP1186Layout<keccak256::KeccakHasher>>::new(&db, &state_root).build();

	let Ok(Some(trie_value)) = trie.get(key.as_slice()) else {
		return Err(StorageError::StorageValueError);
	};

	let r = Rlp::new(trie_value.as_slice());

	let Ok(item_count) = r.item_count() else {
		return Err(StorageError::StorageValueError);
	};

	if item_count != 4 {
		return Err(StorageError::AccountNotFound);
	}

	let Ok(item) = r.at(2).and_then(|e| e.data()) else {
		return Err(StorageError::StorageValueError);
	};

	let storage_root = rlp_to_h256(item)?;

	Ok(storage_root)
}

fn rlp_to_h256(value: &[u8]) -> Result<H256, StorageError> {
	const H256_LENGTH: usize = 32;

	if value.len() > H256_LENGTH {
		return Err(StorageError::CannotDecodeItems);
	}

	// 0s are prepended if value.len() is less than 32.
	let mut slot_value = [0u8; H256_LENGTH];
	let offset = H256_LENGTH - value.len();
	for (i, v) in value.iter().enumerate() {
		slot_value[i + offset] = *v;
	}

	Ok(H256::from(slot_value))
}

#[cfg(test)]
mod test {
	use super::*;
	use ark_std::vec;
	use avail_core::data_proof::{AddressedMessage, Message};
	use frame_support::assert_err;

	use hex_literal::hex;
	use primitive_types::{H160, H256};
	use sp_io::hashing::keccak_256;

	#[test]
	fn rlp_to_h256_fails_with_len_over_32() {
		let faulty = [0u8; 33];
		assert_err!(rlp_to_h256(&faulty), StorageError::CannotDecodeItems);
	}

	#[test]
	fn rlp_to_h256_works_with_len_32() {
		let rlp = [1u8; 32];
		let expected = H256::from(rlp);

		let actual = rlp_to_h256(&rlp).unwrap();
		assert_eq!(actual, expected);
	}

	#[test]
	fn rlp_to_h256_works_with_len_under_32() {
		let rlp: [u8; 31] = [1u8; 31];

		let mut expected = [1u8; 32];
		expected[0] = 0;
		let expected = H256::from(expected);

		let actual = rlp_to_h256(&rlp).unwrap();
		assert_eq!(actual, expected);
	}

	#[test]
	fn test_account_proof() {
		let key = H160::from_slice(hex!("426bde66abd85741be832b824ea65a3aad70113e").as_slice());
		let proof = vec![
            hex!("f90211a00089429375db917315fb4b8d67055bdf76e13d11292801af4a4a151f5760ff7aa02ebce9bb13a075ff89c5aae6b67f4d457525c53dfcc016ce72ea17e0e15a3718a04201c7d41a78f6906183b252fecbb231305d4e22c7e5b729b95a5a6ac53f4d46a06b61a1f5e208c3babf5fc1c9c4180af47769ec421c2c3125f313b5394014fa8aa0b2f35b0e2a84ce9e685b3e9558a0495552c80baec0bd687092220314850f543ba0244dca6d79c72abe8e3a12d49f2cf1976ee7bef58c5c6eb9ff6708fa138abfcca005631aa85658a9962bfee9a4827df5ca6f5461c4bc533591c897a66421f9abbfa0478ef142f553c91d672d865bed8d5175ebbbfc72be010d23b8d81cdcb41247e0a0365a9b70e7c6d82d3246b130bc27453ba77f0bcb4301d43c719eae676a7e0d17a001768b342f6cbc790d57276817d0853c94a682e295930951059bd1c24352b46ea0e3d9b775f71b4c1b2a0c35b1e492b0f2c6ce66c94cf2c8320276fe5cd5e427c8a03bd4160a5626c0d56a4435cb13b6cd3adb5f93793b71148cafa16e07f554fa41a052ab349de3157030b412abdd7353ee1d6476c09c153ddb1dba487294f11a5c7ca0ab71e81c1fc9e656fa8f0df6ee16efa5f105acce3c43ef172a04534f00e5d25ea05306a9ed38acb653787765466a764d4c8748c29b4e7a9ad4a75c61c0840b4a17a0699307b9c473f45858fec9fecd034fa0b3427c0efdd02d407c03201dcdaca02380").to_vec(),
            hex!("f90211a0f7c14d7714348be36359dd28afd64e2fb72679a7ae09a27027fc95e335bcde1ca0824329840722c728e0f19ae424caad4581ac42015a4ab8e9d3ea550c857da804a040d48c9df564c00b11d304e2a2597a35b17b25429c3850c4e3fe4e9a278bec88a0a497297590785cfaa8491579745c077b1095348912d4e3288d8f00857ed9db5da0b0ea3abfcdab8c6cf03152cc7a57f602f85d86f4bdb3d1ca2242a5e737561bbda06bbe0e0416b59f1c4cba36afdee766ea4689f1c1ac8e2245f45c2631e2478119a0222dec72b36685a0ca89e49ce87262957f7f891e695ea8ec52e25fbc3a328589a00b3cac878feb2bcd5fc3d49fe5f607eabf75f014df74a268d4aaa1d25654d030a000deffa5e2879748ef9a634a3573484b4dd259c0d4c10453a7e1e3504b56322ea05c356b24b3b36089583f650cb954f884b05275b09b7715a2eb3cf6fa9175738ea093abf2b2cb15649c192d0d79f62627ce634843f84ec98eee99267c1354b5135aa059e9c60388154b3b810ffd41f81ed9128c8091a12e0c53062d9e7430fedf5939a06855c9a5622a40b5bce572522e4774986c7061557d2f1b8f7070d8d397888b4ea04d220a5fb22e38d64cdf4b46a42898b9f1ce9f316f1d332eebebd32c0cc59000a09004930139d4ae94070b29245230d5b28b25ac59c11339928a2eb547f0828341a00f37af44fb487a5ed675e12f0566a54e59cc025466e91cf56dcf348ff4049ed980").to_vec(),
            hex!("f90211a0e9fa1abfa1f1d84a27da9448b42e3c0f5c60c54a1e8cb90c9e28b60824157380a05e977e1d37e502ac74fd54a2debf7e9b7b6e64c261e45e9b0610bcc201ddbe93a02f8a351ea5204d62c85fe6b564eab729fd556b1941a4f83f6f4b6e40e4102869a0a4b62da8ab84fcd0cf425fba4fd03ad7f1350217679e105e57ee146f64b07e07a061049f894647148c39ec3d8c4563d22670ee697f2e4a003513595f5074fe0166a0de1551dd310c9206da56ff9288dc518cccf7cdfa259cc3ff0318a6f3f7539988a00e600d8cb072056fbf1f5bf7d18aec2eb2ba57e93b5e6bb3f0d36042ec8fbe9ba0fa02eb32060ca2e3fd46e39a8456f02156b8efb457c74ccab5789bce1d142613a0919bb37876273e3283660eb2c575ddcfa99239ab79cf7edaf64d5591689c7777a052a8ee269c13ef214ba56ff0ef6b3cb11da6b12ddadbf1883831e91c6768bf60a0028fdfd852916e9cfa13eee9bf6c540bdc7f6d9b18eee15e11da66a8cdfc933ba09d581d74aa42d7974e122d3a3ec6febaa74ca9f714ddf5c52a5bfa9ee41471e5a0c5608d4aef23664aaaa38aa2425cf959b62d30cf22a0d14147a3cab3d4178fc3a0beb1d967ae4415f30d7730c1bfd43446e24c5f0210cb3a0f5a9bc67e9f63228ea03117ae91a22815aac4b1c9210ba7a6682697a73cd68d7741d693c1cbd1925063a032cf653822d7a618300ef2113f0ff0be132dda944106f25350b5f37451c740a280").to_vec(),
            hex!("f90211a0f284a2e627542f07910ea0cb276b0b7813f3b9c056aafe496b3e7f93d1b3aa67a0d45d246efac9fb2e0c8052354aa0eebd68a28e9606efbbd4a5c2f9e990dc4d3ea0fd5d8349c16fda7a90a9c778cc74126188887aeacec8761349e1863d4008602fa022796160a8b1259fca46b22aa863131e970b077a449a5be4c486c9384335826da0b28076746e56b0bc37fb7586e2c4f23b624523d8e2f7abdffa73859cd531c12da08af556fb72bb802fde89a5562659959ef83a7846f0ced10ed6e139b44016bae9a0f948d4f88be556c183e053c131cd62aa278bcc83845437bfc03721828a3e2082a038c90f875a89a76b5b42d7c843ee790b759e482570a0bcb4d291496a40815093a031b88038ca3cd315ba56e783d4423c7c306cd9567f5a9eca972ac631c4c58e83a0858cbce5374ea0469281ee65c5a1aa5cfa19e7f7df02635821be244a5d39a38ea00cefc972ac8009f230bd9c8015753e98072b5f71d3a09093309ac6f09002f420a0e5fb8ae4800ad431a827003be4d719efcc29424f3ad2fbe483a42ab724a8610ea01a584c371a17ffc56a7713b2c6bb65bbcbf63c9d6382e0423dd577031c63842da0104f13e37d23eed61ebe6b78ee93ee9c30c3a92dab0ccbc57715051e9744eb58a0b211502efd34235ac7f948856c809f8aaf5e299df97ff24d4fb0d53caa3d1e83a043d845df46ad73ae3a9f2bfa319c19e7f760922f1268d8b96f0a54cb8ae88ab880").to_vec(),
            hex!("f90211a071241195c881f3437ebd19a9eccd009595c10537df66917a8fab0eb664f834dda0122c775309b9cff05db80ba77a60604d0fcb8a836a5e79999943f0d150297e19a0c32190d1506259a9ffa2ec1fbff6b23bd35d4e6bcb063b19a22ec10b914981f4a022a77ca63522f76d016d04e680d4c27c3ceee14bc4548f9e08c2cc10f9e1b789a0c646ec46e8f8d5fb7de785fe967200994afec4c48b2bcb001b5aed20db936326a0e20c61d63a3ac612051c43ed1acce68e185a08154e5f44e8eceebac0f454202da05b17a5f4ba7ed711f694536b96a69549fe097ba32dee1f9c71eb19a0533d46baa04da0bc8c8f03ad8f1efdf0da738f24c9ec4549acc71d43ae6607f22601ac4f38a08ea8a34e48a70ccac672eaa2c3a4538d61d38cb5a143a4596d571904b6e3181ea0148252504cc36b4f6b1ef7183df2ce176963bacfc97ad3949fcb6da7d4095821a03d63131beaa2c1137d599528084b0aeb4bea87ee8da16f424dd93c3b90087a75a059f94b55179b81bb657f5021b161ab30fffc8620706a858de7103a0da99a262ea0bb62efd30271c9e2bfc8a4938ebcf4d90623d1d55ffb97399f6456c597599464a024a60032c223c88b91e1fc98db296e58468ebf38eed7bdab0e114cdd754bdc80a0271ec93cc3efaacce706f26a3aa42d6f7c9d8fd6944329149ad63b43c78aae34a07caa42499d46895c9b948f37479c6572573db5b644a0862168e25e4e3bfdb57e80").to_vec(),
            hex!("f9015180a09089f0d1272f06751d391dfbc7b6d49b39731b8a14b5e5e97d45e34d89df0f3fa0820bbc641b62cf0f6a4c3836017cdef0bf7f43c1ee8cbc76ce7b5dcd80f58b9480a0fbe1f0ac8158473558c7b9964cc295027449f6e960f5f6407d9ca1c9ef15f7bca0a2fb890c487021019f73371bf6798e8db8b612ca3c7b30fc3495441a1f9518c4a02cd1ca2531caa6e63ac5f16e5ea76018826683f10442ab5c2b1f9963f23b011ca0429bcf37f564e67dd5764f96fa79532113668cbb32059affdfdc82cfdfd5d1e18080a09be000de088393ee33eac568ba00e318f0ed370eded1cdf38aa75ad55e63945380a0a9138320438845382842e94a5b4ea6756af0c82a0f6b4f17eaf049d617aba98ea0229898dbbae35aa9ef23f2a46c26d419257c35ba11aff1b02ca2024a057f8acaa0cc4c22a6806f250facbdecc1d8874d430ccc277d68ca91b5fb10b4d9f7c681578080").to_vec(),
            hex!("f891808080a076082e119bb693f858172779676f80da4deb1fd75b39db89ec6c96e36125cf6a8080a02b87e60a23ebea051ea7f029c26c5fad0ba86fb8d6d5d4bb563f48ddbf7fa6aca0d9693138b984cccc06a7461c7f39cc28947c9dd95d94bdea1047ddd420b81360808080808080a0ae23c016152c96bfa600e365cd62d6ce721f0b0d310e3c7c18b8a293b722a4ab8080").to_vec(),
            hex!("f8669d3e80870bed23e92a482b9f577efea539b7865c0383284e1bf8cb8ae0e3b846f8440280a06801798586ca88b0ef3b4fb3f83162a9f13e5e242b4c8024c490006054e43933a0f99c7a628a59cf1d27d3a906618656d06e3cdcbcd5f91503c002ea2f2420bc01").to_vec(),
        ];

		// execution state root
		let root = H256(hex!(
			"d6b8a2fb20ade94a56d9d87a07ca11e46cc169ed43dc0d2527a0d3ca2309ba9c"
		));

		let expected_storage_root = H256(hex!(
			"6801798586ca88b0ef3b4fb3f83162a9f13e5e242b4c8024c490006054e43933"
		));

		let storage_root_result = get_storage_root(proof, key, root);

		assert_eq!(expected_storage_root, storage_root_result.unwrap());
	}

	#[test]
	fn test_storage_value() {
		let abi_encoded = hex!("00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004").as_slice();
		let key = keccak_256(abi_encoded);

		let proof = vec![
            hex!("f8d18080a0fc8644862938b67a6de59daee2ca86a4a43c8c4fe6d7ca5f71ea19a3e85565c080a002116e22ba81d7274dc866a4612e9b4e3f10345d5164d4c6e02fd6b672446f4da0b23f6176235c786974b40b6a64b3428c26e7ecc9530b122dd26ebe148d12c33380a04ee52d46ac712e1be0869a689dd6116bed17180e70d9d327d0e335e4098c0397808080a072b7b4fabd398c9b5c05e5f329038a9a9bda658b15a56a3d6a298755511538b18080a079866ac4ff54c3062d8fbd4fa347961e9a905b4114a2ed9785e22a5c03f4ffb88080").to_vec(),
            hex!("e219a0053d037613f1c22bb588aaa70237b3798774d2b20413c686e2263daef21ec226").to_vec(),
            hex!("f851a0c45dca792d516550b57f7f31e33c67f0e6debfe0bdb3076fe0078c65c5afbf8280808080a022e43fa2c06d3d498253aadec7a7db94183eec2aabbdf2afc67a45107d19932b8080808080808080808080").to_vec(),
            hex!("f8429f3841a49a1089f4b560f91cfbb0133326654dcbb1041861fc5dde96c724a22fa1a0efac9989593dfa1e64bac26dd75fd613470d99766ad2c954af658253a09d1ad8").to_vec(),
        ];

		let storage_root = H256(hex!(
			"6801798586ca88b0ef3b4fb3f83162a9f13e5e242b4c8024c490006054e43933"
		));

		let value = get_storage_value(H256(key), storage_root, proof);
		let expected_value =
			hex!("efac9989593dfa1e64bac26dd75fd613470d99766ad2c954af658253a09d1ad8");

		assert_eq!(H256(expected_value), value.unwrap())
	}

	#[test]
	fn test_storage_root_avail() {
		let expected_value =
			hex!("6801798586ca88b0ef3b4fb3f83162a9f13e5e242b4c8024c490006054e43933");

		let proof = vec![
            hex!("f90211a00089429375db917315fb4b8d67055bdf76e13d11292801af4a4a151f5760ff7aa02ebce9bb13a075ff89c5aae6b67f4d457525c53dfcc016ce72ea17e0e15a3718a04201c7d41a78f6906183b252fecbb231305d4e22c7e5b729b95a5a6ac53f4d46a06b61a1f5e208c3babf5fc1c9c4180af47769ec421c2c3125f313b5394014fa8aa0b2f35b0e2a84ce9e685b3e9558a0495552c80baec0bd687092220314850f543ba0244dca6d79c72abe8e3a12d49f2cf1976ee7bef58c5c6eb9ff6708fa138abfcca005631aa85658a9962bfee9a4827df5ca6f5461c4bc533591c897a66421f9abbfa0478ef142f553c91d672d865bed8d5175ebbbfc72be010d23b8d81cdcb41247e0a0365a9b70e7c6d82d3246b130bc27453ba77f0bcb4301d43c719eae676a7e0d17a001768b342f6cbc790d57276817d0853c94a682e295930951059bd1c24352b46ea0e3d9b775f71b4c1b2a0c35b1e492b0f2c6ce66c94cf2c8320276fe5cd5e427c8a03bd4160a5626c0d56a4435cb13b6cd3adb5f93793b71148cafa16e07f554fa41a052ab349de3157030b412abdd7353ee1d6476c09c153ddb1dba487294f11a5c7ca0ab71e81c1fc9e656fa8f0df6ee16efa5f105acce3c43ef172a04534f00e5d25ea05306a9ed38acb653787765466a764d4c8748c29b4e7a9ad4a75c61c0840b4a17a0699307b9c473f45858fec9fecd034fa0b3427c0efdd02d407c03201dcdaca02380").to_vec(),
            hex!("f90211a0f7c14d7714348be36359dd28afd64e2fb72679a7ae09a27027fc95e335bcde1ca0824329840722c728e0f19ae424caad4581ac42015a4ab8e9d3ea550c857da804a040d48c9df564c00b11d304e2a2597a35b17b25429c3850c4e3fe4e9a278bec88a0a497297590785cfaa8491579745c077b1095348912d4e3288d8f00857ed9db5da0b0ea3abfcdab8c6cf03152cc7a57f602f85d86f4bdb3d1ca2242a5e737561bbda06bbe0e0416b59f1c4cba36afdee766ea4689f1c1ac8e2245f45c2631e2478119a0222dec72b36685a0ca89e49ce87262957f7f891e695ea8ec52e25fbc3a328589a00b3cac878feb2bcd5fc3d49fe5f607eabf75f014df74a268d4aaa1d25654d030a000deffa5e2879748ef9a634a3573484b4dd259c0d4c10453a7e1e3504b56322ea05c356b24b3b36089583f650cb954f884b05275b09b7715a2eb3cf6fa9175738ea093abf2b2cb15649c192d0d79f62627ce634843f84ec98eee99267c1354b5135aa059e9c60388154b3b810ffd41f81ed9128c8091a12e0c53062d9e7430fedf5939a06855c9a5622a40b5bce572522e4774986c7061557d2f1b8f7070d8d397888b4ea04d220a5fb22e38d64cdf4b46a42898b9f1ce9f316f1d332eebebd32c0cc59000a09004930139d4ae94070b29245230d5b28b25ac59c11339928a2eb547f0828341a00f37af44fb487a5ed675e12f0566a54e59cc025466e91cf56dcf348ff4049ed980").to_vec(),
            hex!("f90211a0e9fa1abfa1f1d84a27da9448b42e3c0f5c60c54a1e8cb90c9e28b60824157380a05e977e1d37e502ac74fd54a2debf7e9b7b6e64c261e45e9b0610bcc201ddbe93a02f8a351ea5204d62c85fe6b564eab729fd556b1941a4f83f6f4b6e40e4102869a0a4b62da8ab84fcd0cf425fba4fd03ad7f1350217679e105e57ee146f64b07e07a061049f894647148c39ec3d8c4563d22670ee697f2e4a003513595f5074fe0166a0de1551dd310c9206da56ff9288dc518cccf7cdfa259cc3ff0318a6f3f7539988a00e600d8cb072056fbf1f5bf7d18aec2eb2ba57e93b5e6bb3f0d36042ec8fbe9ba0fa02eb32060ca2e3fd46e39a8456f02156b8efb457c74ccab5789bce1d142613a0919bb37876273e3283660eb2c575ddcfa99239ab79cf7edaf64d5591689c7777a052a8ee269c13ef214ba56ff0ef6b3cb11da6b12ddadbf1883831e91c6768bf60a0028fdfd852916e9cfa13eee9bf6c540bdc7f6d9b18eee15e11da66a8cdfc933ba09d581d74aa42d7974e122d3a3ec6febaa74ca9f714ddf5c52a5bfa9ee41471e5a0c5608d4aef23664aaaa38aa2425cf959b62d30cf22a0d14147a3cab3d4178fc3a0beb1d967ae4415f30d7730c1bfd43446e24c5f0210cb3a0f5a9bc67e9f63228ea03117ae91a22815aac4b1c9210ba7a6682697a73cd68d7741d693c1cbd1925063a032cf653822d7a618300ef2113f0ff0be132dda944106f25350b5f37451c740a280").to_vec(),
            hex!("f90211a0f284a2e627542f07910ea0cb276b0b7813f3b9c056aafe496b3e7f93d1b3aa67a0d45d246efac9fb2e0c8052354aa0eebd68a28e9606efbbd4a5c2f9e990dc4d3ea0fd5d8349c16fda7a90a9c778cc74126188887aeacec8761349e1863d4008602fa022796160a8b1259fca46b22aa863131e970b077a449a5be4c486c9384335826da0b28076746e56b0bc37fb7586e2c4f23b624523d8e2f7abdffa73859cd531c12da08af556fb72bb802fde89a5562659959ef83a7846f0ced10ed6e139b44016bae9a0f948d4f88be556c183e053c131cd62aa278bcc83845437bfc03721828a3e2082a038c90f875a89a76b5b42d7c843ee790b759e482570a0bcb4d291496a40815093a031b88038ca3cd315ba56e783d4423c7c306cd9567f5a9eca972ac631c4c58e83a0858cbce5374ea0469281ee65c5a1aa5cfa19e7f7df02635821be244a5d39a38ea00cefc972ac8009f230bd9c8015753e98072b5f71d3a09093309ac6f09002f420a0e5fb8ae4800ad431a827003be4d719efcc29424f3ad2fbe483a42ab724a8610ea01a584c371a17ffc56a7713b2c6bb65bbcbf63c9d6382e0423dd577031c63842da0104f13e37d23eed61ebe6b78ee93ee9c30c3a92dab0ccbc57715051e9744eb58a0b211502efd34235ac7f948856c809f8aaf5e299df97ff24d4fb0d53caa3d1e83a043d845df46ad73ae3a9f2bfa319c19e7f760922f1268d8b96f0a54cb8ae88ab880").to_vec(),
            hex!("f90211a071241195c881f3437ebd19a9eccd009595c10537df66917a8fab0eb664f834dda0122c775309b9cff05db80ba77a60604d0fcb8a836a5e79999943f0d150297e19a0c32190d1506259a9ffa2ec1fbff6b23bd35d4e6bcb063b19a22ec10b914981f4a022a77ca63522f76d016d04e680d4c27c3ceee14bc4548f9e08c2cc10f9e1b789a0c646ec46e8f8d5fb7de785fe967200994afec4c48b2bcb001b5aed20db936326a0e20c61d63a3ac612051c43ed1acce68e185a08154e5f44e8eceebac0f454202da05b17a5f4ba7ed711f694536b96a69549fe097ba32dee1f9c71eb19a0533d46baa04da0bc8c8f03ad8f1efdf0da738f24c9ec4549acc71d43ae6607f22601ac4f38a08ea8a34e48a70ccac672eaa2c3a4538d61d38cb5a143a4596d571904b6e3181ea0148252504cc36b4f6b1ef7183df2ce176963bacfc97ad3949fcb6da7d4095821a03d63131beaa2c1137d599528084b0aeb4bea87ee8da16f424dd93c3b90087a75a059f94b55179b81bb657f5021b161ab30fffc8620706a858de7103a0da99a262ea0bb62efd30271c9e2bfc8a4938ebcf4d90623d1d55ffb97399f6456c597599464a024a60032c223c88b91e1fc98db296e58468ebf38eed7bdab0e114cdd754bdc80a0271ec93cc3efaacce706f26a3aa42d6f7c9d8fd6944329149ad63b43c78aae34a07caa42499d46895c9b948f37479c6572573db5b644a0862168e25e4e3bfdb57e80").to_vec(),
            hex!("f9015180a09089f0d1272f06751d391dfbc7b6d49b39731b8a14b5e5e97d45e34d89df0f3fa0820bbc641b62cf0f6a4c3836017cdef0bf7f43c1ee8cbc76ce7b5dcd80f58b9480a0fbe1f0ac8158473558c7b9964cc295027449f6e960f5f6407d9ca1c9ef15f7bca0a2fb890c487021019f73371bf6798e8db8b612ca3c7b30fc3495441a1f9518c4a02cd1ca2531caa6e63ac5f16e5ea76018826683f10442ab5c2b1f9963f23b011ca0429bcf37f564e67dd5764f96fa79532113668cbb32059affdfdc82cfdfd5d1e18080a09be000de088393ee33eac568ba00e318f0ed370eded1cdf38aa75ad55e63945380a0a9138320438845382842e94a5b4ea6756af0c82a0f6b4f17eaf049d617aba98ea0229898dbbae35aa9ef23f2a46c26d419257c35ba11aff1b02ca2024a057f8acaa0cc4c22a6806f250facbdecc1d8874d430ccc277d68ca91b5fb10b4d9f7c681578080").to_vec(),
            hex!("f891808080a076082e119bb693f858172779676f80da4deb1fd75b39db89ec6c96e36125cf6a8080a02b87e60a23ebea051ea7f029c26c5fad0ba86fb8d6d5d4bb563f48ddbf7fa6aca0d9693138b984cccc06a7461c7f39cc28947c9dd95d94bdea1047ddd420b81360808080808080a0ae23c016152c96bfa600e365cd62d6ce721f0b0d310e3c7c18b8a293b722a4ab8080").to_vec(),
            hex!("f8669d3e80870bed23e92a482b9f577efea539b7865c0383284e1bf8cb8ae0e3b846f8440280a06801798586ca88b0ef3b4fb3f83162a9f13e5e242b4c8024c490006054e43933a0f99c7a628a59cf1d27d3a906618656d06e3cdcbcd5f91503c002ea2f2420bc01").to_vec(),
        ];

		let key = H160::from_slice(hex!("426BdE66aBd85741be832B824eA65A3AaD70113E").as_slice());

		let state_root = hex!("d6b8a2fb20ade94a56d9d87a07ca11e46cc169ed43dc0d2527a0d3ca2309ba9c");
		let value = get_storage_root(proof, key, H256(state_root));

		assert_eq!(H256(expected_value), value.unwrap())
	}

	#[test]
	fn test_abi_encoding() {
		let expected_encoded_message = hex!("00000000000000000000000000000000000000000000000000000000000000200200000000000000000000000000000000000000000000000000000000000000f39fd6e51aad88f6f4ce6ab8827279cfffb9226600000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000e00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000de0b6b3a7640000").to_vec();

		let asset_id = H256::zero();
		let amount = 1_000_000_000_000_000_000u128;
		let from = hex!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266000000000000000000000000");
		let to = hex!("0000000000000000000000000000000000000000000000000000000000000001");
		let m = AddressedMessage {
			message: Message::FungibleToken { asset_id, amount },
			from: from.into(),
			to: to.into(),
			origin_domain: 2,
			destination_domain: 1,
			id: 0,
		};

		let encoded = m.abi_encode();
		assert_eq!(expected_encoded_message, encoded);
	}

	#[test]
	fn test_storage_with_padded_value() {
		let expected_value = H256(hex!(
			"00eee07ead3b0877b420f4f13c67d4449fa051db6a6b877de1265def8f1f3f99"
		));

		let trimmed_value = hex!("eee07ead3b0877b420f4f13c67d4449fa051db6a6b877de1265def8f1f3f99");
		let padded_value_resutl = rlp_to_h256(trimmed_value.as_slice());
		assert_eq!(expected_value, padded_value_resutl.unwrap());

		let exact_value = hex!("00eee07ead3b0877b420f4f13c67d4449fa051db6a6b877de1265def8f1f3f99");
		let padded_exact_value = rlp_to_h256(exact_value.as_slice());
		assert_eq!(expected_value, padded_exact_value.unwrap());

		let empty: &[u8] = &[];
		let empty_padded = rlp_to_h256(empty);
		assert_eq!(H256::zero(), empty_padded.unwrap());

		let invalid_value =
			hex!("0000eee07ead3b0877b420f4f13c67d4449fa051db6a6b877de1265def8f1f3f99");
		let error = rlp_to_h256(invalid_value.as_slice());

		assert_err!(error, StorageError::CannotDecodeItems);
	}
}
