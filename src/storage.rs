// Copyright Rivtower Technologies LLC.
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

use cita_cloud_proto::storage::storage_service_client::StorageServiceClient;
use cita_cloud_proto::storage::{Content, ExtKey};
use status_code::StatusCode;
use tonic::transport::Channel;
use tonic::Request;

pub async fn store_data(
    mut client: StorageServiceClient<Channel>,
    region: u32,
    key: Vec<u8>,
    value: Vec<u8>,
) -> StatusCode {
    let content = Content { region, key, value };
    let request = Request::new(content.clone());
    match client.store(request).await {
        Ok(response) => StatusCode::from(response.into_inner()),
        Err(e) => {
            log::warn!("store_data({:?}) failed: {}", content, e.to_string());
            StatusCode::StorageServerNotReady
        }
    }
}

pub async fn load_data(
    mut client: StorageServiceClient<Channel>,
    region: u32,
    key: Vec<u8>,
) -> Result<Vec<u8>, StatusCode> {
    let ext_key = ExtKey { region, key };
    let request = Request::new(ext_key.clone());
    let response = client.load(request).await.map_err(|e| {
        log::warn!("load_data({:?}) failed: {}", ext_key, e.to_string());
        StatusCode::StorageServerNotReady
    })?;

    let value = response.into_inner();
    StatusCode::from(value.status.ok_or(StatusCode::NoneStatusCode)?).is_success()?;
    Ok(value.value)
}
