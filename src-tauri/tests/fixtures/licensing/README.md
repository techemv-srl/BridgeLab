# Cross-language signing contract fixtures

`server_signed_license.json` was signed with a THROW-AWAY test Ed25519 key
(deterministic seed `0x42 * 32`); its public key is `test_public_key.hex`.
The unit test `test_server_signing_contract` verifies the signature over the
compact serde JSON of `LicensePayload` (struct field order). If the license
server's signer changes its serialization, regenerate this fixture from the
backend repo's `scripts/make_test_vector.py` — a failure here means the two
sides have drifted.

These keys are test-only; the real private key never leaves TECHEMV.
