use crypto::digest::Digest;
use crypto::ripemd160::*;
use crypto::sha2::Sha256;
use hex;
use jsonrpc_core::*;
use rand;
use rand::{OsRng, Rng};
use secp256k1::{constants, Message, PublicKey, Secp256k1, SecretKey, Signature};
use std::cmp::min;
use std::result::Result;
use tari_error::*;
use tari_messages::*;

pub fn random_hash() -> String {
    let hash_length = 32;
    let hash_bytes: Vec<u8> = (0..hash_length).map(|_| rand::thread_rng().gen_range(0, 255)).collect();
    convert_bytes_to_hexstring(&hash_bytes)
}

pub fn convert_hexstring_to_bytes(input_hexstring: &String) -> Vec<u8> {
    let decode_result = hex::decode(input_hexstring);
    match decode_result {
        Ok(v) => (v),
        Err(_e) => (vec![0 as u8]),
    }
}

pub fn convert_bytes_to_hexstring(input_bytes: &Vec<u8>) -> String {
    hex::encode(input_bytes)
}

fn force_byte_array_size(input_bytes: Vec<u8>, desired_byte_count: usize) -> Vec<u8> {
    let mut output_bytes = vec![0; desired_byte_count];
    for i in 0..min(input_bytes.len(), desired_byte_count) {
        output_bytes[i] = input_bytes[i];
    }
    output_bytes
}

pub fn cryptographic_keypair() -> (Vec<u8>, Vec<u8>) {
    let secp = Secp256k1::new();
    let mut rng = OsRng::new().expect("OsRng");
    let (secp_secret_key, secp_public_key) = secp.generate_keypair(&mut rng);
    let secret_key_bytes = convert_hexstring_to_bytes(&secp_secret_key.to_string());
    let public_key_bytes = convert_hexstring_to_bytes(&secp_public_key.to_string());
    (secret_key_bytes, public_key_bytes)
}

pub fn cryptographic_hash(input_msg: &String) -> Vec<u8> {
    //H=RIPEMD160(SHA256(message))
    //Hash Message using Sha256
    let mut sha = Sha256::new();
    sha.input_str(&input_msg);
    let hash_hexstring = sha.result_str();
    let hash_bytes = convert_hexstring_to_bytes(&hash_hexstring);
    //Hash Message using RIPEMD160
    let mut ripemd = Ripemd160::new();
    ripemd.input(&hash_bytes);
    let hash_hexstring = ripemd.result_str();
    convert_hexstring_to_bytes(&hash_hexstring)
}

pub fn cryptographic_signature(input_msg: &String, secret_key: &Vec<u8>) -> Result<Vec<u8>, TariError> {
    //Note: ECDSA of secp256k1 requires exactly 32 bytes as input, pad with zeros if less and discard entries if more
    let msg_hash_bytes = force_byte_array_size(cryptographic_hash(input_msg), constants::MESSAGE_SIZE);
    //S=ECDSA(HASH(message),private_key)
    let secp = Secp256k1::new();
    let secp_message = Message::from_slice(&msg_hash_bytes)?;
    let secp_secret_key = SecretKey::from_slice(&secp, &secret_key)?;
    let secp_data_signature = secp.sign(&secp_message, &secp_secret_key);
    Ok(secp_data_signature.serialize_der(&secp))
}

pub fn cryptographic_verify(data_signature: &Vec<u8>, input_msg: &String, public_key: &Vec<u8>) -> bool {
    //Note: ECDSA of secp256k1 requires exactly 32 bytes as input, pad with zeros if less and discard entries if more
    let msg_hash_bytes = force_byte_array_size(cryptographic_hash(input_msg), constants::MESSAGE_SIZE);
    let secp = Secp256k1::new();
    let secp_message_result = Message::from_slice(&msg_hash_bytes);
    match secp_message_result {
        Ok(secp_message) => {
            let secp_signature_result = Signature::from_der(&secp, &data_signature);
            match secp_signature_result {
                Ok(secp_signature) => {
                    let secp_public_key_result = PublicKey::from_slice(&secp, &public_key);
                    match secp_public_key_result {
                        Ok(secp_public_key) => (secp.verify(&secp_message, &secp_signature, &secp_public_key).is_ok()),
                        Err(_e) => (false),
                    }
                }
                Err(_e) => (false),
            }
        }
        Err(_e) => (false),
    }
}

pub fn message_data_signature(
    msg_header: &MessageHeader,
    msg_payload: &String,
    secret_key: &Vec<u8>,
) -> Result<String, TariError> {
    let msg_header_string = to_value(&msg_header)?.to_string();
    let complete_msg_string = msg_header_string + &msg_payload;
    Ok(convert_bytes_to_hexstring(&cryptographic_signature(
        &complete_msg_string,
        &secret_key,
    )?))
}

pub fn message_verification(
    msg_header: &MessageHeader,
    msg_payload: &String,
    msg_signature: &MessageSignature,
) -> Result<bool, TariError> {
    let msg_header_string = to_value(&msg_header)?.to_string();
    let complete_msg_string = msg_header_string + &msg_payload;
    let data_signature = convert_hexstring_to_bytes(&msg_signature.data_signature);
    let msg_public_key = convert_hexstring_to_bytes(&msg_signature.public_key);
    /*
    //temp_start - Code to retrieve the actual message signature during debugging
    //user_secret_key= 7b19dfe6596f47e1b63b3d940f3550eff647a055c2cc9771820adf5674b9bea8
    //user_public_key= 038bfbd19b918eb24b894e10430d100aae74ec44d6c69c0c4f961999ae25f1ed54
    let secret_key = convert_hexstring_to_bytes(&String::from(
        "7b19dfe6596f47e1b63b3d940f3550eff647a055c2cc9771820adf5674b9bea8",
    ));
    println!(
        "  + actual_data_signature= {}",
        convert_bytes_to_hexstring(&cryptographic_signature(&complete_msg_string, &secret_key)?)
    );
    //temp_stop
    */
    Ok(cryptographic_verify(
        &data_signature,
        &complete_msg_string,
        &msg_public_key,
    ))
}
