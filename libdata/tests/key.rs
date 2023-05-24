use insta;
use quickcheck::{quickcheck, TestResult};

use libdata::{keypair, KeyPair};

#[test]
fn key_can_generate() {
    let _keypair = KeyPair::generate();
}

#[test]
fn key_can_derive() {
    let keypair = KeyPair::generate();
    let _derived = keypair::derive(&keypair.sk, "hello");
}

quickcheck! {
    fn key_same_key_different_names(a: String, b: String) -> TestResult {
        if a == b {
            return TestResult::discard()
        }

        let main = KeyPair::generate();
        let a = keypair::derive(&main.sk, &a);
        let b = keypair::derive(&main.sk, &b);

        TestResult::from_bool(a.as_slice() != b.as_slice())
    }

    fn key_different_key_same_name(name: String) -> bool {
        let a = KeyPair::generate();
        let b = KeyPair::generate();
        let a = keypair::derive(&a.sk, &name);
        let b = keypair::derive(&b.sk, &name);

        a.as_slice() != b.as_slice()
    }

    fn key_same_key_same_name(name: String) -> bool {
        let main = KeyPair::generate();
        let a = keypair::derive(&main.sk, &name);
        let b = keypair::derive(&main.sk, &name);

        a.as_slice() == b.as_slice()
    }
}

const SEED_BYTES: [u8; 32] = [
    157, 097, 177, 157, 239, 253, 090, 096, 186, 132, 074, 244, 146, 236, 044, 196, 068, 073, 197,
    105, 123, 050, 105, 025, 112, 059, 172, 003, 028, 174, 127, 096,
];

#[test]
fn key_secret_key_bytes_have_not_changed() {
    let main = KeyPair::from_seed(keypair::Seed::from(SEED_BYTES)).sk;
    insta::assert_debug_snapshot!(main.as_slice());
}

#[test]
fn key_snapshot_1() {
    let main = KeyPair::from_seed(keypair::Seed::from(SEED_BYTES)).sk;
    let keypair = keypair::derive(&main, "hello");
    insta::assert_debug_snapshot!(keypair.as_slice());
}

#[test]
fn key_snapshot_2() {
    let main = KeyPair::from_seed(keypair::Seed::from(SEED_BYTES)).sk;
    let keypair = keypair::derive(&main, "hello2");
    insta::assert_debug_snapshot!(keypair.as_slice());
}

#[test]
fn key_snapshot_3() {
    let main = KeyPair::from_seed(keypair::Seed::from(SEED_BYTES)).sk;
    let keypair = keypair::derive(
        &main,
        "a very long string as a key name should not break the key derive, \
        it should just work without any issues, this is just testing it, \
        to be sure",
    );
    insta::assert_debug_snapshot!(keypair.as_slice());
}
