use quickcheck::quickcheck;

use libdata::keypair;

quickcheck! {
    fn keypair_bip39_generate_recover() -> bool {
        let (original, phrase) = keypair::generate_bip39();
        let recovered = keypair::recover_bip39(&phrase).unwrap();

        recovered.as_slice() == original.as_slice()
    }
}
