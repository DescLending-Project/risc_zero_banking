# TLSN (TLSNotary) Verifier

TLSNotary Verifier is a secure service running in a Trusted Execution Environment (TEE) that validates TLSNotary proofs and provides attestation reports to verify its own integrity.

## Deployed on

Phala Cloud : [https://5bc711d2610565cbc9609527aba1a7846c6f2237-8080.dstack-prod7.phala.network](https://5bc711d2610565cbc9609527aba1a7846c6f2237-8080.dstack-prod7.phala.network)

via : `../github/workflows/deploy.yml`

To test the deployment CI/CD locally also see: `../.secrets.template` and create your own `../.secrets` and then run :
```shell
act workflow_dispatch --secret-file .secrets --container-architecture linux/amd64
```
in the parent directory `../`.

- `Dockerfile`: Used to generate Docker image to be deployed and run on Phala Cloud, defined in docker-compose.phala.yml.

- `docker-compose.phala.yml`: Docker Compose configuration for Phala Cloud. Pulls the image from `Dockerfile`, mounts `/var/run/tappd.sock`, and sets environment variables.

- `docker-compose.yml`: Used to compose up locally using `.env` for testing.


## Overview
This service verifies the authenticity of TLSNotary proofs, which are cryptographic proofs that demonstrate a TLS connection occurred with specific data exchanged. The verifier runs inside a TEE, ensuring that verification cannot be tampered with. TLSN proof verification takes approximately ±14 ms in the TEE.

## Key Derivation and Generation Process
The verifier running inside a Trusted Execution Environment (TEE) generates a fresh **EC key pair** using a deterministic derivation process from `:

1. The verifier sends a `DeriveKey` request to the TEE's secure API:
    ```http
    POST /prpc/Tappd.DeriveKey?json
    Content-Type: application/json

    {
        // empty
    }
    ```
2. Internally, the TEE:
    - Derives a key pair (`SigningKey` / `VerifyingKey`) from **root key**.
    - Serializes the **private key** and **certificates** in **PEM** format and returns as follows.
    ```json
    {
        "key": "-----BEGIN PRIVATE KEY-----\nMIGHAgEAMBMG...kljULmdS\n-----END PRIVATE KEY-----\n",
        "certificate_chain": 
            [
                "-----BEGIN CERTIFICATE-----\nMIIBQjCB6qADAgECAhRR...CaEDDTjk=\n-----END CERTIFICATE-----\n",
                "-----BEGIN CERTIFICATE-----\nMIIBqjCCAU+gAwIBAgIU...szIg==\n-----END CERTIFICATE-----\n"
            ]
    }

    ```
3. Inside the application, the response is used to reconstruct the key:
```rust
let signing_key = SigningKey::from_pkcs8_pem(&response.key)?;
let verifying_key = signing_key.verifying_key();
```
This process ensures the key is deterministically tied to the session identifier but generated within the enclave. It is used to sign the attestation quote.

## Features

- **Proof Verification**: Validates TLSNotary proofs with cryptographic certainty
- **TEE Attestation**: Provides attestation reports to prove the verifier is running in a genuine TEE
- **Greedy API Key Authentication**: Secures access to verification endpoints with preset API key.


##  API Endpoints

- **GET /health**
    
    Returns the health status 

    **Example Request**
     **Headers**
    ```json
    x-api-key: <api-key> //ask @rbbozkurt
    ```

    **Example Response**
     ```json
    Ok
    ```

- **GET /attestation**
    
    Returns the attestation quote from Phala Cloud

    **Example Request**
     **Headers**
    ```json
    x-api-key: <api-key> //ask @rbbozkurt
    ```

    **Example Response**
     ```json
    {
        "quote": "0400...000",
        "signature_hex_encoded": "5d9...f2c",
        "verifying_key_hex_encoded": "044...422",
        "verifying_key_certificate_chain": 
            ["-----BEGIN PUBLIC KEY-----\nMFYwEAYHKo...\n-----END PUBLIC KEY-----",
            "-----BEGIN PUBLIC KEY-----\nPFYklOYHsV...\n-----END PUBLIC KEY-----"]
    }
    ```

    **Fields Explanation**

    - `quote`: The attestation report from the TEE, which includes report_data tied to the verifier's key.

    - `signature_hex_encoded`: Signature (ECDSA) of the quote, generated using the verifier's private key.

    - `verifying_key_hex_encoded`: The public key used to verify the quote signature (uncompressed SEC1 format, hex).
  
    - `verifying_key_certificate_chain` : A certificate chain (PEM format) proving that the enclave key pair was generated and certified by a valid DCAP authority. Includes the verifier’s leaf certificate and root CA certificate.

 - **POST /verify-proof**

    Verifies a TLSNotary proof and returns both the verification result and an attestation report.

    **Example Request:**

    **Body**
    ```json
    // The body can be directly retrieved by TLSNotary, copy the proof from TLSNotary and paste below.
    {
        "version": "0.1.0-alpha.10",
        "data": "0140...ffda",
        "meta": {
            "notaryUrl": "https://notary.pse.dev",
            "websocketProxyUrl": "ws://localhost:8080"
        }
    }
    ```

    **Headers**
    ```json
    x-api-key: <api-key> //ask @rbbozkurt
    ```

    **Example Response**
    ```json
    {
        "verification": {
            "Ok": {
                "is_valid": true,
                "server_name": "openbanking-api-826260723607.europe-west3.run.app",
                "score": "59",
                "verifying_key": "037b48f19c139b6888fb5e383a4d72c2335186fd5858e7ae743ab4bf8e071b06e7",
                "sent_hex_encoded": "4745..d0a",
                "sent_readable": "GET https://openbanking-api-826260723607.europe-west3.run.app/users/aaa/credit-score HTTP/1.1\r\nhost: openbanking-api-826260723607.europe-west3.run.app\r\nconnection: close\r\ncontent-length: 0\r\n\r\n",
                "recv_hex_encoded": "485...858",
                "recv_readable": "HTTP/1.1 200 OK\r\nXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXserver: Google Frontend\r\nAlt-Svc: h3=\":443\"; ma=2592000,h3-29=\":443\"; ma=2592000\r\nConnection: close\r\nTransfer-Encoding: chunked\r\nXXXXXXX\"path\":\"/users/aaa/credit-score\"X\"message\":\"Credit score retrieved successfully\"XXXXXXXXX\"userId\":\"aaa\"XXXXXXXXXX\"value\":59XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
                "time": "2025-06-20T19:56:17+00:00"
            }
        },
        "attestation": {
            "Ok": {
                "quote": "0400...000",
                "signature_hex_encoded": "5d9...f2c",
                "verifying_key_hex_encoded": "044...422",
                "verifying_key_certificate_chain": 
                    ["-----BEGIN PUBLIC KEY-----\nMFYwEAYHKo...\n-----END PUBLIC KEY-----",
                    "-----BEGIN PUBLIC KEY-----\nPFYklOYHsV...\n-----END PUBLIC KEY-----"]
            }
        }
    }
    ```


## More abouts fields on attestion report.

In the deployed TEE we generated random private key (`SigningKey`), public key (`VerifyingKey`) pair, which we will then used in creation of quote and verification.

### quote
Attestation quote from TEE environment's attestation report. Similar to a Zero-Knowledge Proof, an attestation report in a TEE allows anyone to verify that the TEE is genuine. Moreover, it guarantees that data in the quote (the measurement of the TEE) can be considered trustworthy.

Below is a typical Intel DCAP attestation report produced by Phala Cloud.


```json
{
  "tee_tcb_svn": "06010300000000000000000000000000",
  "mr_seam": "5b38e33a6487958b72c3c12a938eaa5e3fd4510c51aeeab58c7d5ecee41d7c436489d6c8e4f92f160b7cad34207b00c1",
  "mr_signer_seam": "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
  "seam_attributes": "0000000000000000",
  "td_attributes": "0000001000000000",
  "xfam": "e702060000000000",
  "mr_td": "c68518a0ebb42136c12b2275164f8c72f25fa9a34392228687ed6e9caeb9c0f1dbd895e9cf475121c029dc47e70e91fd",
  "mr_config_id": "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
  "mr_owner": "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
  "mr_owner_config": "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
  "rt_mr0": "85e0855a6384fa1c8a6ab36d0dcbfaa11a5753e5a070c08218ae5fe872fcb86967fd2449c29e22e59dc9fec998cb6547",
  "rt_mr1": "9b43f9f34a64bc7191352585be0da1774a1499e698ba77cbf6184547d53d1770d6524c1cfa00b86352f273fc272a8cfe",
  "rt_mr2": "7cc2dadd5849bad220ab122c4fbf25a74dc91cc12702447d3b5cac0f49b2b139994f5cd936b293e5f0f14dea4262d668",
  "rt_mr3": "2c482b5b34f6902293bc203696f407241bfa319d2410a04c604d1021888d6028bf4bd280ff859ee270a0429aac5f0d82",
  "report_data": "afab9790acb13c4c651c1933a22b5f0663ef22927120dd08cc8291d7e0912d8b1c36eb75cf661a64735042f8e81bbe42cb9ab310ca95bf8d36c44cb8835c901f"
}
```

Notice the `report_data`. While generating the attestion quote from attestation report above, in the `report_data`, we embed and hash the `VerifyingKey` from pre-generated private (signing), public(verifying) key pair as in following : 
```rust
let verifying_key_bytes = signing_key.verifying_key()
            .to_encoded_point(false)
            .as_bytes()
            .to_vec();

let verifying_key_hash = Sha512::digest(&verifying_key_bytes);

let report_data = format!("0x{}", hex::encode(verifying_key_hash));
```

Also see [this guide](https://phala.network/posts/how-to-generate-attestation-repport-and-prove-your-application-runs-in-tee) to understand more about `report_data`.

### signature_hex_encoded

The encoded version of signature, private key (`SigningKey`) signing the encoded version of `quote` above, which is then obtained as following:

```rust 
let encoded_msg = hex::encode(quote); //encoded quote
let signature = signing_key.sign(encoded_msg.as_bytes()); //sign
let signature_hex_encoded = hex::encode(signature.to_bytes()); //encoded signature
```
### verifying_key_hex_encoded
Encoded version of public key (`VerifyingKey`) obtained as followed: 

```rust
let verifying_key_bytes = signing_key.verifying_key()
            .to_encoded_point(false)
            .as_bytes()
            .to_vec();

let verifying_key_hex_encoded = hex::encode(verifying_key_bytes)
```

### verifying_key_certificate_chain

A PEM-encoded **X.509 certificate** chain that proves the authenticity and attestation status of the TEE:

- The **leaf certificate** is bound to the public key (`VerifyingKey`) generated inside the enclave.

- The certificate includes a DCAP-signed quote (via SGX PCK certificate extensions) verifying that the enclave is running trusted code and holds the corresponding private key.

- The chain ends with the **root certificate** issued by Intel’s DCAP Certificate Authority (or equivalent).

This chain allows external verifiers to **validate the public key origin and integrity** through DCAP. It ensures the key pair was securely generated inside a TEE and not tampered with.

## Verification

First, suggested is the reobtain the `VerifyingKey` from `verifying_key_hex_encoded`, so we could be able to verify and the integrity of TEE. 

```rust
let bytes = hex::decode(verifying_key_hex_encoded).ok().expect("Failed to decode verifying_key hex");

let point = EncodedPoint::from_bytes(&bytes).ok().expect("Failed to decode point from bytes");

let verifying_key = VerifyingKey::from_encoded_point(&point).ok().expect("Failed to create verifying key from encoded point");
```

Now we have the `VerifyingKey` obtained, then we could pregenerate our `report_data`:
```rust
let verifying_key_bytes = verifying_key
            .to_encoded_point(false)
            .as_bytes()
            .to_vec();

let verifying_key_hash = Sha512::digest(&verifying_key_bytes);

let report_data = format!("0x{}", hex::encode(verifying_key_hash));
```
We now also have the `report_data`, after obtaining the attestation `quote`, use any DCAP-compatible verifier to validate it. For example, [https://proof.t16z.com/](https://proof.t16z.com/) relies on [dcap-qvl](https://github.com/Phala-Network/dcap-qvl). Other verifiers exist, including on-chain solutions like Automata’s Solidity implementation.

To verify the quote you generated, copy the quote’s hexadecimal data from your response body and paste it into [https://proof.t16z.com/](https://proof.t16z.com/). After verification, you can scroll through the report contents to confirm that `report_data` matches the value you precalcuated above.

To able to also verify the signature, given `quote`, `signature_hex_encoded` (from response body) and reobtained `VerifyingKey` do the following: 

```rust
let quote_hex_encoded = hex::encode(quote); //first encode the given quote

let signature_bytes = hex::decode(signature_hex_encoded).expect("Failed to decode hex signature");
    let signature = Signature::from_bytes(
        signature_bytes
            .as_slice()
            .try_into()
            .expect("Signature bytes have incorrect length"),
    ).expect("Failed to create signature from bytes"); //
    let quote_hex_encoded_bytes = quote_hex_encoded.as_bytes(); //obtain signature given signature_hex_encoded
    verifying_key.verify(quote_hex_encoded_bytes, &signature).is_ok() //verify the signature using VerifyingKey
```
