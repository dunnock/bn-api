use super::tari_messages::*;
use tari_error::TariError;

use cryptographic::*;
use log::Level;
use reqwest::blocking::Client;
use serde_json;

pub trait TariClient {
    fn create_asset(
        &self,
        secret_key: &String,
        public_key: &String,
        asset: MessagePayloadCreateAsset,
    ) -> Result<String, TariError>;

    fn modify_asset_increase_supply(
        &self,
        secret_key: &String,
        public_key: &String,
        asset_id: &String,
        new_supply: u64,
    ) -> Result<(), TariError>;

    fn modify_asset_nullify_tokens(
        &self,
        secret_key: &String,
        public_key: &String,
        asset_id: &String,
        token_ids: Vec<u64>,
    ) -> Result<(), TariError>;

    fn modify_asset_redeem_token(
        &self,
        secret_key: &String,
        public_key: &String,
        asset_id: &String,
        token_ids: Vec<u64>,
    ) -> Result<(), TariError>;

    fn transfer_tokens(
        &self,
        secret_key: &String,
        public_key: &String,
        asset_id: &String,
        token_ids: Vec<u64>,
        new_owner: String,
    ) -> Result<(), TariError>;

    fn get_asset_info(
        &self,
        secret_key: &String,
        public_key: &String,
        asset_id: &String,
    ) -> Result<ResponsePayloadReadAsset, TariError>;

    fn box_clone(&self) -> Box<dyn TariClient + Send + Sync>;
}

impl Clone for Box<dyn TariClient + Send + Sync> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

#[derive(Clone, Debug)]
pub struct HttpTariClient {
    tari_url: String,
}

impl HttpTariClient {
    pub fn new(tari_url: String) -> HttpTariClient {
        HttpTariClient { tari_url }
    }
}

impl TariClient for HttpTariClient {
    fn create_asset(
        &self,
        secret_key: &String,
        public_key: &String,
        asset: MessagePayloadCreateAsset,
    ) -> Result<String, TariError> {
        let header_command = String::from("create_asset");
        let msg_payload = serde_json::to_value(asset)?;
        let secret_key = convert_hexstring_to_bytes(&secret_key);
        let public_key = convert_hexstring_to_bytes(&public_key);
        let jsonrpc_request = construct_jsonrpc_request(header_command, msg_payload, &secret_key, &public_key)?;

        let client = Client::new();
        let resp = client.post(&self.tari_url).json(&jsonrpc_request).send()?;
        let raw: String = resp.text()?;
        jlog!(Level::Info, &format!("Response from create_asset:{}", raw));
        let response_message: RPCResponse = serde_json::from_str(&raw)?;
        let response_message_result: ResponsePayloadSuccessId = serde_json::from_value(response_message.result)?;

        if response_message_result.success {
            Ok(response_message_result.id)
        } else {
            Err(TariError {
                description: "Failed to create Asset on Tari".to_string(),
                cause: None,
            })
        }
    }

    fn modify_asset_increase_supply(
        &self,
        secret_key: &String,
        public_key: &String,
        asset_id: &String,
        new_supply: u64,
    ) -> Result<(), TariError> {
        let header_command = String::from("modify_asset");
        let msg_payload = serde_json::to_value(MessagePayloadModifyAsset {
            request_type: 2,
            asset_id: asset_id.clone(),
            authorised_signer: None,
            token_ids: None,
            token_metadata: None,
            new_supply: Some(new_supply),
        })?;
        let secret_key = convert_hexstring_to_bytes(&secret_key);
        let public_key = convert_hexstring_to_bytes(&public_key);
        let jsonrpc_request = construct_jsonrpc_request(header_command, msg_payload, &secret_key, &public_key)?;

        let client = Client::new();
        let resp = client.post(&self.tari_url).json(&jsonrpc_request).send()?;
        let raw: String = resp.text()?;
        jlog!(Level::Info, &format!("Response from modify_asset: {}", raw));
        let response_message: RPCResponse = serde_json::from_str(&raw)?;
        let response_message_result: ResponsePayloadSuccess = serde_json::from_value(response_message.result)?;

        if response_message_result.success {
            Ok(())
        } else {
            Err(TariError {
                description: "Failed to increase token supply on Tari".to_string(),
                cause: None,
            })
        }
    }

    fn modify_asset_nullify_tokens(
        &self,
        secret_key: &String,
        public_key: &String,
        asset_id: &String,
        token_ids: Vec<u64>,
    ) -> Result<(), TariError> {
        let header_command = String::from("modify_asset");
        let msg_payload = serde_json::to_value(MessagePayloadModifyAsset {
            request_type: 4,
            asset_id: asset_id.clone(),
            authorised_signer: None,
            token_ids: Some(token_ids),
            token_metadata: None,
            new_supply: None,
        })?;
        let secret_key = convert_hexstring_to_bytes(&secret_key);
        let public_key = convert_hexstring_to_bytes(&public_key);
        let jsonrpc_request = construct_jsonrpc_request(header_command, msg_payload, &secret_key, &public_key)?;

        let client = Client::new();
        let resp = client.post(&self.tari_url).json(&jsonrpc_request).send()?;
        let raw: String = resp.text()?;
        jlog!(Level::Info, &format!("Response from modify_asset: {}", raw));
        let response_message: RPCResponse = serde_json::from_str(&raw)?;
        let response_message_result: ResponsePayloadSuccess = serde_json::from_value(response_message.result)?;

        if response_message_result.success {
            Ok(())
        } else {
            Err(TariError {
                description: "Failed to redeem tokens on Tari".to_string(),
                cause: None,
            })
        }
    }

    fn modify_asset_redeem_token(
        &self,
        secret_key: &String,
        public_key: &String,
        asset_id: &String,
        token_ids: Vec<u64>,
    ) -> Result<(), TariError> {
        let header_command = String::from("modify_asset");
        let msg_payload = serde_json::to_value(MessagePayloadModifyAsset {
            request_type: 5,
            asset_id: asset_id.clone(),
            authorised_signer: None,
            token_ids: Some(token_ids),
            token_metadata: None,
            new_supply: None,
        })?;
        let secret_key = convert_hexstring_to_bytes(&secret_key);
        let public_key = convert_hexstring_to_bytes(&public_key);
        let jsonrpc_request = construct_jsonrpc_request(header_command, msg_payload, &secret_key, &public_key)?;

        let client = Client::new();
        let resp = client.post(&self.tari_url).json(&jsonrpc_request).send()?;
        let raw: String = resp.text()?;
        jlog!(Level::Info, &format!("Response from modify_asset: {}", raw));
        let response_message: RPCResponse = serde_json::from_str(&raw)?;
        let response_message_result: ResponsePayloadSuccess = serde_json::from_value(response_message.result)?;

        if response_message_result.success {
            Ok(())
        } else {
            Err(TariError {
                description: "Failed to redeem tokens on Tari".to_string(),
                cause: None,
            })
        }
    }

    fn transfer_tokens(
        &self,
        secret_key: &String,
        public_key: &String,
        asset_id: &String,
        token_ids: Vec<u64>,
        new_owner: String,
    ) -> Result<(), TariError> {
        let header_command = String::from("transfer_token");
        let msg_payload = serde_json::to_value(MessagePayloadTransferToken {
            asset_id: asset_id.clone(),
            token_ids,
            new_owner,
        })?;
        let secret_key = convert_hexstring_to_bytes(&secret_key);
        let public_key = convert_hexstring_to_bytes(&public_key);
        let jsonrpc_request = construct_jsonrpc_request(header_command, msg_payload, &secret_key, &public_key)?;

        let client = Client::new();
        let resp = client.post(&self.tari_url).json(&jsonrpc_request).send()?;
        let raw: String = resp.text()?;
        jlog!(Level::Info, &format!("Response from transfer_token: {}", raw));
        let response_message: RPCResponse = serde_json::from_str(&raw)?;
        let response_message_result: ResponsePayloadSuccess = serde_json::from_value(response_message.result)?;

        if response_message_result.success {
            Ok(())
        } else {
            Err(TariError {
                description: "Failed to transfer tokens on Tari".to_string(),
                cause: None,
            })
        }
    }

    fn get_asset_info(
        &self,
        secret_key: &String,
        public_key: &String,
        asset_id: &String,
    ) -> Result<ResponsePayloadReadAsset, TariError> {
        let header_command = String::from("read_asset");
        let msg_payload = serde_json::to_value(MessagePayloadReadAsset {
            request_type: 0,
            user: None,
            asset_id: asset_id.clone(),
            token_ids: None,
        })?;
        let secret_key = convert_hexstring_to_bytes(&secret_key);
        let public_key = convert_hexstring_to_bytes(&public_key);
        let jsonrpc_request = construct_jsonrpc_request(header_command, msg_payload, &secret_key, &public_key)?;

        let client = Client::new();
        let resp = client.post(&self.tari_url).json(&jsonrpc_request).send()?;
        let raw: String = resp.text()?;
        jlog!(Level::Info, &format!("Response from read_asset: {}", raw));
        let response_message: RPCResponse = serde_json::from_str(&raw)?;
        let response_message_result: ResponsePayloadReadAsset = serde_json::from_value(response_message.result)?;

        Ok(response_message_result)
    }

    fn box_clone(&self) -> Box<dyn TariClient + Send + Sync> {
        Box::new((*self).clone())
    }
}
