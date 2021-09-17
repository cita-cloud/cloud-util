//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::common::{ADDR_BYTES_LEN, HASH_BYTES_LEN};
use cita_cloud_proto::blockchain::BlockHeader;
use cita_cloud_proto::kms::kms_service_client::KmsServiceClient;
use cita_cloud_proto::kms::{HashDataRequest, RecoverSignatureRequest, SignMessageRequest};
use log::warn;
use prost::Message;
use status_code::StatusCode;
use tonic::transport::Channel;

pub async fn hash_data(
    mut client: KmsServiceClient<Channel>,
    data: &[u8],
) -> Result<Vec<u8>, StatusCode> {
    let data = data.to_vec();
    match client.hash_data(HashDataRequest { data }).await {
        Ok(res) => {
            let hash_respond = res.into_inner();
            let status_code =
                StatusCode::from(hash_respond.status.ok_or(StatusCode::NoneStatusCode)?.code);

            if status_code != StatusCode::Success {
                Err(status_code)
            } else {
                Ok(hash_respond.hash.ok_or(StatusCode::NoneHashResult)?.hash)
            }
        }
        Err(status) => {
            warn!("hash_data error: {}", status.to_string());
            Err(StatusCode::KmsServerNotReady)
        }
    }
}

pub async fn get_block_hash(
    client: KmsServiceClient<Channel>,
    header: Option<&BlockHeader>,
) -> Result<Vec<u8>, StatusCode> {
    match header {
        Some(header) => {
            let mut block_header_bytes = Vec::with_capacity(header.encoded_len());
            header.encode(&mut block_header_bytes).map_err(|_| {
                warn!("get_block_hash: encode block header failed");
                StatusCode::EncodeError
            })?;
            let block_hash = hash_data(client, &block_header_bytes).await?;
            Ok(block_hash)
        }
        None => Err(StatusCode::NoneBlockHeader),
    }
}

pub async fn pk2address(
    client: KmsServiceClient<Channel>,
    pk: &[u8],
) -> Result<Vec<u8>, StatusCode> {
    Ok(hash_data(client, pk).await?[HASH_BYTES_LEN - ADDR_BYTES_LEN..].to_vec())
}

pub async fn sign_message(
    mut client: KmsServiceClient<Channel>,
    key_id: u64,
    msg: &[u8],
) -> Result<Vec<u8>, StatusCode> {
    let respond = client
        .sign_message(SignMessageRequest {
            key_id,
            msg: msg.to_vec(),
        })
        .await
        .map_err(|e| {
            warn!("recover_signature failed: {}", e.to_string());
            StatusCode::KmsServerNotReady
        })?;

    let rsr = respond.into_inner();
    let status = StatusCode::from(rsr.status.ok_or(StatusCode::NoneStatusCode)?);
    if status != StatusCode::Success {
        Err(status)
    } else {
        Ok(rsr.signature)
    }
}

pub async fn recover_signature(
    mut client: KmsServiceClient<Channel>,
    signature: &[u8],
    msg: &[u8],
) -> Result<Vec<u8>, StatusCode> {
    let respond = client
        .recover_signature(RecoverSignatureRequest {
            msg: msg.to_vec(),
            signature: signature.to_vec(),
        })
        .await
        .map_err(|e| {
            warn!("recover_signature failed: {}", e.to_string());
            StatusCode::KmsServerNotReady
        })?;

    let rsr = respond.into_inner();
    let status = StatusCode::from(rsr.status.ok_or(StatusCode::NoneStatusCode)?);
    if status != StatusCode::Success {
        Err(status)
    } else {
        Ok(rsr.address)
    }
}
