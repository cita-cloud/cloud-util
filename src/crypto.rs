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


use cita_cloud_proto::kms::kms_service_client::KmsServiceClient;
use cita_cloud_proto::kms::HashDataRequest;
use status_code::StatusCode;

pub async fn hash_data(mut client: KmsServiceClient<Channel>, data: Vec<u8>) -> Result<Vec<u8>, StatusCode> {
    match client.hash_data(HashDataRequest { data }).await {
        Ok(res) => {
            Ok(res.into_inner().hash)
        }
        Err(status) => {
            warn!("hash_data error: {}", status.to_string());
            StatusCode::KmsServerNotReady
        }
    }
}


